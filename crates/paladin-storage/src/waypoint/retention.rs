//! Waypoint retention/cleanup routine (ENG-FR-18).
//!
//! Bounds `waypoints` table growth by age and/or per-thread count. This
//! routine does not decide what "protected" means -- it is handed the
//! protected set for each thread by its caller and applies the configured
//! bounds only to whatever the caller has not already protected. The one
//! definition of protected -- a thread's latest Waypoint plus every
//! `AwaitingInput` Waypoint, with two more classes named as not-yet-existing
//! seams -- lives in the application layer, at
//! `src/application/services/waypoint_retention.rs` (X-01: policy crosses
//! into this adapter as an argument, it is never re-derived here).
//!
//! # Mechanism
//!
//! Every deletion this routine performs goes through
//! [`WaypointPort::prune_thread`], added in Plan 22-13: for each thread with
//! something to remove, exactly one keep-set call. There is no longer a
//! delete-then-resave sequence -- the enumeration of survivors, the
//! whole-thread delete, and the resave loop that Plan 22-13's context
//! describes as this module's previous shape are gone, because
//! `prune_thread` makes them unnecessary. A recording port double in this
//! module's own tests proves the whole-thread delete is never called during
//! a prune.
//!
//! # Invariant
//!
//! `prune` is monotone and idempotent. The keep-set it hands to
//! `prune_thread` is intact under any crash or backend failure mid-call --
//! there is no interval during which a protected Waypoint does not exist,
//! because nothing removes it in the first place -- and a re-run after any
//! partial run converges to exactly the keep-set and removes nothing
//! further. Retention is best-effort reclamation: leaving an extra
//! surviving Waypoint behind is acceptable, and losing one the caller named
//! in its protected set is not. This is proven by an executable
//! fault-injection acceptance test
//! (`tests/integration/waypoint_retention_fault_injection_test.rs`), not
//! only asserted here.

use std::collections::{HashMap, HashSet};

use chrono::Utc;

use paladin_core::platform::container::waypoint::{ThreadId, WaypointId};
use paladin_ports::output::waypoint_port::{WaypointError, WaypointPort, WaypointSummary};

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
/// `max_age_days` and/or `max_waypoints_per_thread`, on top of whatever
/// `protected` names as never-removable for that thread.
///
/// Both bounds `None` is a no-op (returns an empty [`PruneReport`] without
/// reading `port` at all) -- an operator who has not configured retention
/// gets exactly today's behavior, per `WaypointRetentionConfig`'s
/// disabled-by-default contract (X-09).
///
/// `protected` is called once per thread with that thread's full,
/// newest-first `history` and must return the set of ids this call may
/// never delete for that thread -- typically the thread's latest Waypoint
/// and every `AwaitingInput` Waypoint (see
/// `src/application/services/waypoint_retention.rs` for the one definition
/// this project uses). This routine has no opinion on what belongs in that
/// set; it only unions it with whatever survives the configured bounds and
/// hands the result to [`WaypointPort::prune_thread`] as the keep-set.
///
/// For each thread, ordered newest-first (matching
/// [`WaypointPort::history`]'s documented order):
/// - any id `protected` returns is kept unconditionally, regardless of age
///   or position;
/// - among the remaining candidates, `max_waypoints_per_thread` keeps the
///   `N` newest (by position) and marks the rest for deletion;
/// - `max_age_days` marks any candidate whose `created_at` is older than
///   `now - max_age_days` for deletion.
///
/// A thread holding exactly one Waypoint is always left untouched -- and
/// `port` receives no `prune_thread` call for it at all, since there is
/// nothing beyond that one Waypoint to consider, whatever the bounds.
pub async fn prune(
    port: &dyn WaypointPort,
    max_age_days: Option<u32>,
    max_waypoints_per_thread: Option<u32>,
    protected: &dyn Fn(&ThreadId, &[WaypointSummary]) -> HashSet<WaypointId>,
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
            // Nothing beyond a single Waypoint to even consider -- no call
            // to the port for this thread at all.
            continue;
        }

        let mut keep_ids = protected(&thread_id, &history);
        let mut delete_ids = Vec::new();

        for (position, summary) in history.iter().enumerate() {
            if keep_ids.contains(&summary.waypoint_id) {
                // Protected by the caller: never a deletion candidate,
                // whatever the configured bounds say.
                continue;
            }

            let too_old = cutoff.is_some_and(|cutoff| summary.created_at < cutoff);
            let beyond_kept_count =
                max_waypoints_per_thread.is_some_and(|max| position as u32 >= max);

            if too_old || beyond_kept_count {
                delete_ids.push(summary.waypoint_id);
            } else {
                keep_ids.insert(summary.waypoint_id);
            }
        }

        if delete_ids.is_empty() {
            // Nothing to remove for this thread: no port call at all.
            continue;
        }

        let keep_vec: Vec<WaypointId> = keep_ids.into_iter().collect();
        let removed = port.prune_thread(&thread_id, &keep_vec).await?;
        if removed > 0 {
            report.removed_per_thread.insert(thread_id, removed);
        }
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::waypoint::contract_tests::sample_waypoint_at;
    use crate::waypoint::in_memory::InMemoryWaypointStore;
    use async_trait::async_trait;
    use chrono::{DateTime, Duration};
    use paladin_core::platform::container::waypoint::{
        NodeId, OnExpire, ParleyId, ParleyKind, ParleyRequest, Waypoint, WaypointStatus,
    };
    use paladin_ports::output::waypoint_port::ThreadSummary;
    use std::sync::Mutex;

    fn thread(name: &str) -> ThreadId {
        ThreadId::new(name).unwrap()
    }

    /// A minimal, fully-populated `ParleyRequest` (D-01/D-02 shape) for
    /// tests that only care that an `AwaitingInput` Waypoint exists, not
    /// what it is asking about.
    fn sample_parley_request() -> ParleyRequest {
        ParleyRequest {
            parley_id: ParleyId::new(),
            node_id: NodeId::new("asker"),
            kind: ParleyKind::FreeText,
            prompt: "confirm?".to_string(),
            payload: serde_json::json!({}),
            choices: None,
            expires_at: None,
            created_at: Utc::now(),
            on_expire: OnExpire::FailRun,
        }
    }

    /// The same shape of protected-set definition the real
    /// `src/application/services/waypoint_retention.rs` service supplies:
    /// a thread's latest Waypoint plus every `AwaitingInput` Waypoint.
    /// Duplicated here (rather than imported) because `paladin-storage`
    /// does not and must not depend on the application layer -- this
    /// module exercises the mechanical composition, not the policy's
    /// source.
    fn latest_and_awaiting_protected(
        _thread: &ThreadId,
        history: &[WaypointSummary],
    ) -> HashSet<WaypointId> {
        let mut set = HashSet::new();
        if let Some(latest) = history.first() {
            set.insert(latest.waypoint_id);
        }
        for summary in history {
            if matches!(summary.status, WaypointStatus::AwaitingInput { .. }) {
                set.insert(summary.waypoint_id);
            }
        }
        set
    }

    #[derive(Default)]
    struct CallLog {
        prune_thread_calls: Vec<(ThreadId, Vec<WaypointId>)>,
        delete_thread_calls: Vec<ThreadId>,
    }

    /// A `WaypointPort` test double that delegates every method to an inner
    /// `InMemoryWaypointStore` while recording every call to `prune_thread`
    /// and `delete_thread` -- so a test can assert on the keep-set actually
    /// handed to the port, and that the whole-thread delete path is never
    /// reached, rather than only inferring these from after-the-fact state.
    #[derive(Default)]
    struct RecordingStore {
        inner: InMemoryWaypointStore,
        log: Mutex<CallLog>,
    }

    impl RecordingStore {
        fn prune_thread_calls(&self) -> Vec<(ThreadId, Vec<WaypointId>)> {
            self.log.lock().unwrap().prune_thread_calls.clone()
        }

        fn delete_thread_calls(&self) -> Vec<ThreadId> {
            self.log.lock().unwrap().delete_thread_calls.clone()
        }
    }

    #[async_trait]
    impl WaypointPort for RecordingStore {
        async fn save(&self, wp: &Waypoint) -> Result<(), WaypointError> {
            self.inner.save(wp).await
        }

        async fn latest(&self, thread: &ThreadId) -> Result<Option<Waypoint>, WaypointError> {
            self.inner.latest(thread).await
        }

        async fn get(
            &self,
            thread: &ThreadId,
            id: &WaypointId,
        ) -> Result<Option<Waypoint>, WaypointError> {
            self.inner.get(thread, id).await
        }

        async fn history(
            &self,
            thread: &ThreadId,
            limit: Option<u32>,
            before: Option<WaypointId>,
        ) -> Result<Vec<WaypointSummary>, WaypointError> {
            self.inner.history(thread, limit, before).await
        }

        async fn list_threads(
            &self,
            limit: Option<u32>,
            before: Option<DateTime<Utc>>,
        ) -> Result<Vec<ThreadSummary>, WaypointError> {
            self.inner.list_threads(limit, before).await
        }

        async fn delete_thread(&self, thread: &ThreadId) -> Result<u64, WaypointError> {
            self.log
                .lock()
                .unwrap()
                .delete_thread_calls
                .push(thread.clone());
            self.inner.delete_thread(thread).await
        }

        async fn delete_waypoint(
            &self,
            thread: &ThreadId,
            id: &WaypointId,
        ) -> Result<bool, WaypointError> {
            self.inner.delete_waypoint(thread, id).await
        }

        async fn prune_thread(
            &self,
            thread: &ThreadId,
            keep: &[WaypointId],
        ) -> Result<u64, WaypointError> {
            self.log
                .lock()
                .unwrap()
                .prune_thread_calls
                .push((thread.clone(), keep.to_vec()));
            self.inner.prune_thread(thread, keep).await
        }
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

        let report = prune(&store, None, None, &latest_and_awaiting_protected)
            .await
            .unwrap();

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
            let report = prune(&store, max_age, max_count, &latest_and_awaiting_protected)
                .await
                .unwrap();
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

        let report = prune(&store, None, Some(3), &latest_and_awaiting_protected)
            .await
            .unwrap();

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

        let report = prune(&store, Some(1), None, &latest_and_awaiting_protected)
            .await
            .unwrap();

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

        let report = prune(&store, Some(30), None, &latest_and_awaiting_protected)
            .await
            .unwrap();

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
            parleys: vec![sample_parley_request()],
            responses: Vec::new(),
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

        let report = prune(&store, Some(1), Some(3), &latest_and_awaiting_protected)
            .await
            .unwrap();

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

    /// Case 4 (Phase 24 Plan 06, D-14): the SAME protection holds for an
    /// `AwaitingInput` Waypoint that lives on a BRANCH (`fork_of: Some(..)`)
    /// -- the wildcard `AwaitingInput { .. }` match in
    /// `latest_and_awaiting_protected` above (and the real
    /// `src/application/services/waypoint_retention.rs` it mirrors) is
    /// unaffected by the payload reshape, so a branch-resident suspension
    /// is protected exactly like a mainline one.
    #[tokio::test]
    async fn retention_protects_awaiting_input_on_any_branch() {
        let store = InMemoryWaypointStore::new();
        let t = thread("retention-awaiting-input-on-branch-survives");
        let now = Utc::now();
        let branch_root = WaypointId::generate();

        // An old, BRANCH-RESIDENT AwaitingInput waypoint that would
        // otherwise be pruned by both bounds.
        let mut awaiting = sample_waypoint_at(&t, 0, now - Duration::days(365));
        awaiting.fork_of = Some(branch_root);
        awaiting.status = WaypointStatus::AwaitingInput {
            parleys: vec![sample_parley_request()],
            responses: Vec::new(),
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

        let report = prune(&store, Some(1), Some(3), &latest_and_awaiting_protected)
            .await
            .unwrap();

        let remaining = store.history(&t, None, None).await.unwrap();
        assert!(
            remaining
                .iter()
                .any(|s| s.waypoint_id == awaiting.waypoint_id),
            "a branch-resident AwaitingInput waypoint must survive both bounds"
        );
        assert!(report.total_removed() > 0);

        // Confirm the surviving summary still carries its branch marker --
        // retention did not silently strip it.
        let survivor = remaining
            .iter()
            .find(|s| s.waypoint_id == awaiting.waypoint_id)
            .unwrap();
        assert_eq!(survivor.fork_of, Some(branch_root));
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

        let first = prune(&store, None, Some(3), &latest_and_awaiting_protected)
            .await
            .unwrap();
        assert_eq!(first.total_removed(), 7);

        let second = prune(&store, None, Some(3), &latest_and_awaiting_protected)
            .await
            .unwrap();
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

        let report = prune(&store, None, Some(2), &latest_and_awaiting_protected)
            .await
            .unwrap();

        assert_eq!(report.removed_for(&a), 3);
        assert_eq!(report.removed_for(&b), 0);
        assert_eq!(report.total_removed(), 3);
    }

    #[tokio::test]
    async fn keep_set_handed_to_the_port_always_contains_latest_and_awaiting_input() {
        let store = RecordingStore::default();
        let t = thread("retention-keep-set-contents");
        let now = Utc::now();

        let mut awaiting = sample_waypoint_at(&t, 0, now - Duration::days(365));
        awaiting.status = WaypointStatus::AwaitingInput {
            parleys: vec![sample_parley_request()],
            responses: Vec::new(),
        };
        store.save(&awaiting).await.unwrap();

        let mut latest_id = awaiting.waypoint_id;
        for superstep in 1..10u64 {
            let wp = sample_waypoint_at(&t, superstep, now);
            latest_id = wp.waypoint_id;
            store.save(&wp).await.unwrap();
        }

        prune(&store, Some(1), Some(3), &latest_and_awaiting_protected)
            .await
            .unwrap();

        let calls = store.prune_thread_calls();
        assert_eq!(
            calls.len(),
            1,
            "exactly one prune_thread call for this thread"
        );
        let (_, keep) = &calls[0];
        assert!(
            keep.contains(&awaiting.waypoint_id),
            "keep-set must contain the AwaitingInput waypoint"
        );
        assert!(
            keep.contains(&latest_id),
            "keep-set must contain the thread's latest waypoint"
        );
    }

    #[tokio::test]
    async fn prune_issues_exactly_one_keep_set_call_per_thread_with_something_to_remove() {
        let store = RecordingStore::default();
        let pruned_thread = thread("retention-one-call-pruned");
        let untouched_thread = thread("retention-one-call-untouched");
        let base = Utc::now();

        for superstep in 0..10u64 {
            store
                .save(&sample_waypoint_at(
                    &pruned_thread,
                    superstep,
                    base + Duration::seconds(superstep as i64),
                ))
                .await
                .unwrap();
        }
        // Single-waypoint thread: nothing to remove, no call expected.
        store
            .save(&sample_waypoint_at(&untouched_thread, 0, base))
            .await
            .unwrap();

        prune(&store, None, Some(3), &latest_and_awaiting_protected)
            .await
            .unwrap();

        let calls = store.prune_thread_calls();
        assert_eq!(
            calls.len(),
            1,
            "only the thread with something to remove is called"
        );
        assert_eq!(calls[0].0, pruned_thread);
    }

    #[tokio::test]
    async fn prune_never_calls_the_whole_thread_delete() {
        let store = RecordingStore::default();
        let t = thread("retention-never-delete-thread");
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

        prune(&store, None, Some(3), &latest_and_awaiting_protected)
            .await
            .unwrap();
        // Run again to also exercise the idempotent (nothing-to-remove)
        // path through the same double.
        prune(&store, None, Some(3), &latest_and_awaiting_protected)
            .await
            .unwrap();

        assert!(
            store.delete_thread_calls().is_empty(),
            "prune must never call delete_thread"
        );
    }
}
