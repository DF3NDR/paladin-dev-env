//! Waypoint retention/cleanup routine (ENG-FR-18).
//!
//! Bounds `waypoints` table growth by age and/or per-thread count, with two
//! **hard exclusions enforced as invariants of this routine itself, not left
//! to the caller**: a thread's latest Waypoint is never a deletion
//! candidate, and any Waypoint whose status is `AwaitingInput` is never a
//! deletion candidate (T-22-21) -- a retention routine that eats the
//! checkpoint a human is waiting on is unrecoverable.
//!
//! # Mechanism
//!
//! `WaypointPort` carries no per-waypoint delete primitive by design (only
//! `delete_thread`, which removes an entire thread's history) -- adding one
//! would mean re-opening the port Plan 22-03 already fully specified. This
//! routine instead composes the port's *existing* surface: read a thread's
//! full `history`, decide which waypoints survive, `get` each survivor's
//! full `Waypoint`, `delete_thread` (wiping everything), then `save` each
//! survivor back. This is backend-agnostic by construction -- it needs
//! nothing from any backend beyond what `WaypointPort` already promises, so
//! it runs identically over `InMemoryWaypointStore`, `SqliteWaypointStore`,
//! and `PostgresWaypointStore`.

use std::collections::HashMap;

use chrono::Utc;

use paladin_core::platform::container::waypoint::{ThreadId, WaypointStatus};
use paladin_ports::output::waypoint_port::{WaypointError, WaypointPort};

/// The outcome of one [`prune`] call: how many Waypoints were removed, per
/// thread. A thread with nothing removed (including a thread never
/// considered because it held only one Waypoint) has no entry -- absence
/// means zero, not "unknown".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PruneReport {
    removed_per_thread: HashMap<ThreadId, u64>,
}

impl PruneReport {
    /// How many Waypoints were removed from `thread`. `0` for a thread that
    /// was untouched (including one this prune run never even considered).
    pub fn removed_for(&self, thread: &ThreadId) -> u64 {
        self.removed_per_thread.get(thread).copied().unwrap_or(0)
    }

    /// Total Waypoints removed across every thread this prune run touched.
    pub fn total_removed(&self) -> u64 {
        self.removed_per_thread.values().sum()
    }

    /// Iterate the per-thread removal counts (threads with zero removals are
    /// not present).
    pub fn iter(&self) -> impl Iterator<Item = (&ThreadId, &u64)> {
        self.removed_per_thread.iter()
    }
}

/// Prune old Waypoints across every thread `port` holds, bounded by
/// `max_age_days` and/or `max_waypoints_per_thread`.
///
/// Both bounds `None` is a no-op (returns an empty [`PruneReport`] without
/// reading `port` at all) -- an operator who has not configured retention
/// gets exactly today's behavior, per `WaypointRetentionConfig`'s
/// disabled-by-default contract (X-09).
///
/// For each thread, ordered newest-first (matching
/// [`WaypointPort::history`]'s documented order):
/// - index `0` (the latest Waypoint) is **never** a deletion candidate,
///   regardless of age or count, so `resume` always has something to resume
///   from;
/// - any Waypoint whose `status` is [`WaypointStatus::AwaitingInput`] is
///   **never** a deletion candidate, regardless of age or count, so a
///   pending human response is never silently discarded;
/// - among the remaining candidates, `max_waypoints_per_thread` keeps the
///   `N` newest (by position) and marks the rest for deletion;
/// - `max_age_days` marks any candidate whose `created_at` is older than
///   `now - max_age_days` for deletion.
///
/// A thread holding exactly one Waypoint is always left untouched (there is
/// nothing beyond index `0` to consider), whatever the bounds.
pub async fn prune(
    port: &dyn WaypointPort,
    max_age_days: Option<u32>,
    max_waypoints_per_thread: Option<u32>,
) -> Result<PruneReport, WaypointError> {
    let mut report = PruneReport::default();

    if max_age_days.is_none() && max_waypoints_per_thread.is_none() {
        return Ok(report);
    }

    let cutoff = max_age_days.map(|days| Utc::now() - chrono::Duration::days(i64::from(days)));

    let threads = port.list_threads(None, None).await?;
    for thread_summary in threads {
        let thread_id = thread_summary.thread_id;
        // Newest-first, per WaypointPort::history's documented order.
        let history = port.history(&thread_id, None, None).await?;
        if history.len() <= 1 {
            // Nothing beyond the latest Waypoint to even consider.
            continue;
        }

        let mut keep_ids = Vec::with_capacity(history.len());
        let mut delete_ids = Vec::new();

        for (position, summary) in history.iter().enumerate() {
            if position == 0 {
                // The latest Waypoint: never a deletion candidate.
                keep_ids.push(summary.waypoint_id);
                continue;
            }
            if matches!(summary.status, WaypointStatus::AwaitingInput { .. }) {
                // Awaiting a human response: never a deletion candidate.
                keep_ids.push(summary.waypoint_id);
                continue;
            }

            let too_old = cutoff.is_some_and(|cutoff| summary.created_at < cutoff);
            let beyond_kept_count =
                max_waypoints_per_thread.is_some_and(|max| position as u32 >= max);

            if too_old || beyond_kept_count {
                delete_ids.push(summary.waypoint_id);
            } else {
                keep_ids.push(summary.waypoint_id);
            }
        }

        if delete_ids.is_empty() {
            continue;
        }

        // Fetch survivors' full Waypoints BEFORE wiping the thread -- there
        // is no port primitive for deleting a single waypoint, only a whole
        // thread (see this module's doc comment).
        let mut survivors = Vec::with_capacity(keep_ids.len());
        for id in &keep_ids {
            if let Some(wp) = port.get(&thread_id, id).await? {
                survivors.push(wp);
            }
        }

        port.delete_thread(&thread_id).await?;
        for wp in &survivors {
            port.save(wp).await?;
        }

        report
            .removed_per_thread
            .insert(thread_id, delete_ids.len() as u64);
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::waypoint::contract_tests::sample_waypoint_at;
    use crate::waypoint::in_memory::InMemoryWaypointStore;
    use chrono::Duration;
    use paladin_core::platform::container::waypoint::ParleyRequest;

    fn thread(name: &str) -> ThreadId {
        ThreadId::new(name).unwrap()
    }

    #[tokio::test]
    async fn both_bounds_none_deletes_nothing() {
        let store = InMemoryWaypointStore::new();
        let t = thread("retention-no-bounds");
        for superstep in 0..5u64 {
            store
                .save(&sample_waypoint_at(&t, superstep, Utc::now()))
                .await
                .unwrap();
        }

        let report = prune(&store, None, None).await.unwrap();

        assert_eq!(report.total_removed(), 0);
        assert_eq!(store.history(&t, None, None).await.unwrap().len(), 5);
    }

    #[tokio::test]
    async fn single_waypoint_thread_is_untouched_by_every_bound_combination() {
        let store = InMemoryWaypointStore::new();
        let t = thread("retention-single-waypoint");
        store
            .save(&sample_waypoint_at(&t, 0, Utc::now() - Duration::days(999)))
            .await
            .unwrap();

        for (max_age, max_count) in [
            (None, None),
            (Some(0), None), // will be rejected by config validation elsewhere, but the routine itself must still never touch a single-waypoint thread
            (None, Some(1)),
            (Some(1), Some(1)),
        ] {
            let report = prune(&store, max_age, max_count).await.unwrap();
            assert_eq!(report.total_removed(), 0);
            assert_eq!(store.history(&t, None, None).await.unwrap().len(), 1);
        }
    }

    #[tokio::test]
    async fn max_waypoints_per_thread_leaves_the_newest_n_including_latest() {
        let store = InMemoryWaypointStore::new();
        let t = thread("retention-max-count");
        let base = Utc::now();
        for superstep in 0..10u64 {
            // Distinct created_at per waypoint (superstep order == time
            // order) so "the three newest" is unambiguous.
            store
                .save(&sample_waypoint_at(
                    &t,
                    superstep,
                    base + Duration::seconds(superstep as i64),
                ))
                .await
                .unwrap();
        }

        let report = prune(&store, None, Some(3)).await.unwrap();

        assert_eq!(report.total_removed(), 7);
        let remaining = store.history(&t, None, None).await.unwrap();
        let supersteps: Vec<u64> = remaining.iter().map(|s| s.superstep).collect();
        assert_eq!(
            supersteps,
            vec![9, 8, 7],
            "must keep exactly the 3 newest, including the latest"
        );
    }

    #[tokio::test]
    async fn max_age_never_deletes_the_newest_waypoint_even_when_older_than_the_bound() {
        let store = InMemoryWaypointStore::new();
        let t = thread("retention-max-age-protects-latest");
        // The ONLY waypoint is ancient -- still must survive, since it is
        // also the latest.
        store
            .save(&sample_waypoint_at(
                &t,
                0,
                Utc::now() - Duration::days(9999),
            ))
            .await
            .unwrap();

        let report = prune(&store, Some(1), None).await.unwrap();

        assert_eq!(report.total_removed(), 0);
        assert!(store.latest(&t).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn max_age_deletes_old_non_latest_waypoints() {
        let store = InMemoryWaypointStore::new();
        let t = thread("retention-max-age-deletes-old");
        let now = Utc::now();
        store
            .save(&sample_waypoint_at(&t, 0, now - Duration::days(100)))
            .await
            .unwrap();
        store
            .save(&sample_waypoint_at(&t, 1, now)) // the latest, recent
            .await
            .unwrap();

        let report = prune(&store, Some(30), None).await.unwrap();

        assert_eq!(report.total_removed(), 1);
        let remaining = store.history(&t, None, None).await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].superstep, 1);
    }

    #[tokio::test]
    async fn awaiting_input_waypoint_is_never_deleted_by_either_bound() {
        let store = InMemoryWaypointStore::new();
        let t = thread("retention-awaiting-input-survives");
        let now = Utc::now();

        // An old AwaitingInput waypoint that would otherwise be pruned by
        // both bounds.
        let mut awaiting = sample_waypoint_at(&t, 0, now - Duration::days(365));
        awaiting.status = WaypointStatus::AwaitingInput {
            parley: ParleyRequest {
                prompt: "confirm?".to_string(),
            },
        };
        store.save(&awaiting).await.unwrap();

        // Nine more recent waypoints, so a max_waypoints_per_thread bound
        // would also want to evict the old one by count.
        for superstep in 1..10u64 {
            store
                .save(&sample_waypoint_at(&t, superstep, now))
                .await
                .unwrap();
        }

        let report = prune(&store, Some(1), Some(3)).await.unwrap();

        let remaining = store.history(&t, None, None).await.unwrap();
        assert!(
            remaining
                .iter()
                .any(|s| s.waypoint_id == awaiting.waypoint_id),
            "AwaitingInput waypoint must survive both bounds"
        );
        // Sanity: something else in the old/over-count zone WAS removed.
        assert!(report.total_removed() > 0);
    }

    #[tokio::test]
    async fn running_the_same_prune_twice_removes_nothing_the_second_time() {
        let store = InMemoryWaypointStore::new();
        let t = thread("retention-idempotent");
        let base = Utc::now();
        for superstep in 0..10u64 {
            store
                .save(&sample_waypoint_at(
                    &t,
                    superstep,
                    base + Duration::seconds(superstep as i64),
                ))
                .await
                .unwrap();
        }

        let first = prune(&store, None, Some(3)).await.unwrap();
        assert_eq!(first.total_removed(), 7);

        let second = prune(&store, None, Some(3)).await.unwrap();
        assert_eq!(second.total_removed(), 0);
        assert_eq!(store.history(&t, None, None).await.unwrap().len(), 3);
    }

    #[tokio::test]
    async fn report_reflects_per_thread_removal_counts() {
        let store = InMemoryWaypointStore::new();
        let a = thread("retention-report-thread-a");
        let b = thread("retention-report-thread-b");
        let base = Utc::now();

        for superstep in 0..5u64 {
            store
                .save(&sample_waypoint_at(
                    &a,
                    superstep,
                    base + Duration::seconds(superstep as i64),
                ))
                .await
                .unwrap();
        }
        // Thread b has only one waypoint: never touched.
        store.save(&sample_waypoint_at(&b, 0, base)).await.unwrap();

        let report = prune(&store, None, Some(2)).await.unwrap();

        assert_eq!(report.removed_for(&a), 3);
        assert_eq!(report.removed_for(&b), 0);
        assert_eq!(report.total_removed(), 3);
    }
}
