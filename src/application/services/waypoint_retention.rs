//! Application-layer Waypoint retention service (ENG-FR-18).
//!
//! This module holds the **single definition of "protected"** used anywhere
//! Waypoint retention runs: a thread's latest Waypoint plus every Waypoint
//! whose status is [`WaypointStatus::AwaitingInput`]. `paladin_storage`'s
//! `prune` routine (`crates/paladin-storage/src/waypoint/retention.rs`) does
//! not know this rule -- it is handed the answer as a plain function, per
//! X-01: the decision of what may never be deleted crosses into the
//! storage adapter as an argument, and the adapter carries no copy of it.

use std::collections::HashSet;
use std::sync::Arc;

use paladin_core::platform::container::waypoint::{ThreadId, WaypointId, WaypointStatus};
use paladin_ports::output::waypoint_port::{WaypointError, WaypointPort, WaypointSummary};
use paladin_storage::waypoint::retention::{PruneReport, prune};

use crate::config::WaypointRetentionConfig;

/// Every id in `history` that this project's retention policy says must
/// never be deleted for `thread`: the thread's latest Waypoint (`history`
/// is newest-first, so that is simply the first element) plus every
/// Waypoint whose status is [`WaypointStatus::AwaitingInput`].
///
/// # Future seams
///
/// Two more classes of protected Waypoint do not exist in this tree yet.
/// Both belong here, in this one function, when their owning phase lands --
/// not as a new field, flag, or stub type invented ahead of time, and not
/// re-derived at a different layer:
///
/// - **A Waypoint referenced by an unresolved Parley.** Phase 24
///   (Pause/Resume, History & Graceful Shutdown) introduces indefinite
///   Parley pauses: a node raising a `ParleyRequest` suspends the run and
///   persists an `AwaitingInput` Waypoint. Today `AwaitingInput` status
///   alone is sufficient protection (this function already covers it) --
///   but once a Parley can be answered from a Waypoint *other than* the one
///   that raised it (e.g. a superseding retry), the Waypoint the still-open
///   Parley refers to must be protected by that reference, independent of
///   its own status.
/// - **A Waypoint pinned by an active fork lineage.** Phase 24 also
///   introduces an inspectable, forkable Chronicle: `fork`-with-edit creates
///   a new chain with `fork_of` lineage while the original chain stays
///   byte-identical. Once forking exists, a Waypoint that an active fork's
///   lineage points back to must survive pruning of its original thread
///   even if it is neither the latest nor `AwaitingInput` there.
///
/// A lost protected Waypoint is a data-loss defect; the point of naming
/// these seams here is that the next author extends this one function
/// instead of discovering the omission from a data-loss report.
pub fn protected_waypoints(_thread: &ThreadId, history: &[WaypointSummary]) -> HashSet<WaypointId> {
    let mut protected = HashSet::new();
    if let Some(latest) = history.first() {
        protected.insert(latest.waypoint_id);
    }
    for summary in history {
        if matches!(summary.status, WaypointStatus::AwaitingInput { .. }) {
            protected.insert(summary.waypoint_id);
        }
    }
    protected
}

/// Drives `paladin_storage::waypoint::retention::prune` with this project's
/// one definition of protected ([`protected_waypoints`]) and the configured
/// bounds.
pub struct WaypointRetentionService {
    port: Arc<dyn WaypointPort>,
    config: WaypointRetentionConfig,
}

impl WaypointRetentionService {
    /// Construct a service over `port`, driven by `config`.
    pub fn new(port: Arc<dyn WaypointPort>, config: WaypointRetentionConfig) -> Self {
        Self { port, config }
    }

    /// Run one retention pass.
    ///
    /// If `config.enabled` is `false`, this is a no-op that returns an empty
    /// [`PruneReport`] without reading `port` at all -- the same
    /// disabled-by-default contract (X-09) `WaypointRetentionConfig` itself
    /// documents. Otherwise the configured bounds are passed through to the
    /// routine unchanged, and this call returns exactly the routine's
    /// report.
    pub async fn prune(&self) -> Result<PruneReport, WaypointError> {
        if !self.config.enabled {
            return Ok(PruneReport::default());
        }

        prune(
            self.port.as_ref(),
            self.config.max_age_days,
            self.config.max_waypoints_per_thread,
            &protected_waypoints,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use paladin_core::platform::container::waypoint::{ParleyRequest, Waypoint};
    use paladin_storage::waypoint::contract_tests::sample_waypoint_at;
    use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;

    fn thread(name: &str) -> ThreadId {
        ThreadId::new(name).unwrap()
    }

    /// Build a `WaypointSummary` directly from a `Waypoint`, without going
    /// through a store -- these tests exercise `protected_waypoints` as a
    /// pure function over a hand-built `history` slice, not the storage
    /// layer.
    fn to_summary(wp: &Waypoint) -> WaypointSummary {
        WaypointSummary {
            waypoint_id: wp.waypoint_id,
            parent_waypoint_id: wp.parent_waypoint_id,
            superstep: wp.superstep,
            status: wp.status.clone(),
            created_at: wp.created_at,
        }
    }

    #[tokio::test]
    async fn protected_set_is_exactly_latest_plus_awaiting_input_over_both_neither_and_each_alone()
    {
        // Fixture containing: an AwaitingInput waypoint that is NOT latest,
        // several plain Running waypoints, and the latest waypoint (also
        // plain). Covers "both", "neither" and "each alone" across the
        // three history entries checked below.
        let t = thread("protected-set-fixture");
        let now = Utc::now();

        let mut awaiting_not_latest = sample_waypoint_at(&t, 0, now - Duration::days(1));
        awaiting_not_latest.status = WaypointStatus::AwaitingInput {
            parley: ParleyRequest {
                prompt: "confirm?".to_string(),
            },
        };

        let plain_middle = sample_waypoint_at(&t, 1, now - Duration::minutes(30));
        let latest_plain = sample_waypoint_at(&t, 2, now);

        // history is newest-first, matching WaypointPort::history's order.
        let history = vec![
            to_summary(&latest_plain),
            to_summary(&plain_middle),
            to_summary(&awaiting_not_latest),
        ];

        let protected = protected_waypoints(&t, &history);

        assert!(
            protected.contains(&latest_plain.waypoint_id),
            "the latest waypoint must be protected (neither-awaiting-nor... alone case)"
        );
        assert!(
            protected.contains(&awaiting_not_latest.waypoint_id),
            "a non-latest AwaitingInput waypoint must be protected (awaiting-alone case)"
        );
        assert!(
            !protected.contains(&plain_middle.waypoint_id),
            "a plain, non-latest, non-awaiting waypoint must not be protected (neither case)"
        );
        assert_eq!(protected.len(), 2);
    }

    #[tokio::test]
    async fn protected_set_covers_a_single_waypoint_that_is_both_latest_and_awaiting_input() {
        let t = thread("protected-set-both-latest-and-awaiting");
        let mut wp = sample_waypoint_at(&t, 0, Utc::now());
        wp.status = WaypointStatus::AwaitingInput {
            parley: ParleyRequest {
                prompt: "confirm?".to_string(),
            },
        };
        let history = vec![to_summary(&wp)];

        let protected = protected_waypoints(&t, &history);

        assert_eq!(protected.len(), 1);
        assert!(protected.contains(&wp.waypoint_id));
    }

    #[tokio::test]
    async fn service_passes_configured_bounds_through_unchanged_and_returns_the_report() {
        let store: Arc<dyn WaypointPort> = Arc::new(InMemoryWaypointStore::new());
        let t = thread("service-bounds-pass-through");
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

        let service = WaypointRetentionService::new(
            store.clone(),
            WaypointRetentionConfig {
                enabled: true,
                max_age_days: None,
                max_waypoints_per_thread: Some(3),
            },
        );

        let report = service.prune().await.unwrap();

        assert_eq!(report.removed_for(&t), 7);
        assert_eq!(store.history(&t, None, None).await.unwrap().len(), 3);
    }

    #[tokio::test]
    async fn service_disabled_is_a_no_op_and_does_not_touch_the_port() {
        let store: Arc<dyn WaypointPort> = Arc::new(InMemoryWaypointStore::new());
        let t = thread("service-disabled-no-op");
        for superstep in 0..5u64 {
            store
                .save(&sample_waypoint_at(&t, superstep, Utc::now()))
                .await
                .unwrap();
        }

        let service = WaypointRetentionService::new(
            store.clone(),
            WaypointRetentionConfig {
                enabled: false,
                max_age_days: None,
                max_waypoints_per_thread: Some(1),
            },
        );

        let report = service.prune().await.unwrap();

        assert_eq!(report.total_removed(), 0);
        assert_eq!(store.history(&t, None, None).await.unwrap().len(), 5);
    }
}
