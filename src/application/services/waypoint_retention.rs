//! Application-layer Waypoint retention service (ENG-FR-18).
//!
//! This module holds the **single definition of "protected"** used anywhere
//! Waypoint retention runs: a thread's latest Waypoint plus every Waypoint
//! whose status is [`WaypointStatus::AwaitingInput`]. `paladin_storage`'s
//! `prune` routine (`crates/paladin-storage/src/waypoint/retention.rs`) does
//! not know this rule -- it is handed the answer as a plain function, per
//! X-01: the decision of what may never be deleted crosses into the
//! storage adapter as an argument, and the adapter carries no copy of it.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use paladin_core::platform::container::waypoint::{ThreadId, WaypointId, WaypointStatus};
use paladin_ports::output::run_trace_port::RunTracePort;
use paladin_ports::output::waypoint_port::{WaypointError, WaypointPort, WaypointSummary};
use paladin_storage::run_trace::retention::prune as prune_run_traces;
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
/// bounds. Additively (D-17), also drives `paladin_storage::run_trace::
/// retention::prune` over the SAME threads and the SAME configured bounds
/// when a [`RunTracePort`] is wired via [`with_run_trace_port`](Self::with_run_trace_port)
/// -- one config, one routine, two ports.
pub struct WaypointRetentionService {
    port: Arc<dyn WaypointPort>,
    config: WaypointRetentionConfig,
    /// Optional second port (D-17): `None` until
    /// [`with_run_trace_port`](Self::with_run_trace_port) is called, so
    /// [`new`](Self::new)'s two-argument construction path behaves exactly
    /// as before this field existed (X-03).
    run_trace_port: Option<Arc<dyn RunTracePort>>,
    /// How many `run_traces` rows the MOST RECENT [`prune`](Self::prune)
    /// call removed via a wired [`RunTracePort`]. A purely additive
    /// accessor (X-03): `prune()`'s own return type,
    /// `Result<PruneReport, WaypointError>`, is left untouched by this
    /// field's existence. `PruneReport` itself is defined in
    /// `crates/paladin-storage/src/waypoint/retention.rs`, a file outside
    /// this plan's `files_modified` scope, and extending its shape would be
    /// this crate's own public-API-breaking change (`paladin-ai`'s `[lib]
    /// name = "paladin"` is itself semver-checked) requiring a
    /// `MIGRATION.md` §9.2 entry -- and `MIGRATION.md` is plan 28-17's to
    /// own, not this plan's to edit. Reporting the trace-row count via this
    /// separate, additive getter instead avoids that breaking change
    /// entirely while still making the count observable right after a
    /// `prune()` call.
    last_run_traces_removed: AtomicU64,
}

impl WaypointRetentionService {
    /// Construct a service over `port`, driven by `config`. No
    /// [`RunTracePort`] is wired -- see
    /// [`with_run_trace_port`](Self::with_run_trace_port).
    pub fn new(port: Arc<dyn WaypointPort>, config: WaypointRetentionConfig) -> Self {
        Self {
            port,
            config,
            run_trace_port: None,
            last_run_traces_removed: AtomicU64::new(0),
        }
    }

    /// Additively wire a [`RunTracePort`] into this service (D-17): once
    /// set, every [`prune`](Self::prune) call also prunes `run_traces` for
    /// every thread the Waypoint pass touched, deriving each thread's
    /// superstep boundary from the SAME configured bounds already applied
    /// to that thread's Waypoints. Leaves [`new`](Self::new)'s
    /// two-argument signature untouched: no existing call site breaks
    /// (X-03).
    pub fn with_run_trace_port(mut self, run_trace_port: Arc<dyn RunTracePort>) -> Self {
        self.run_trace_port = Some(run_trace_port);
        self
    }

    /// How many `run_traces` rows the most recent [`prune`](Self::prune)
    /// call removed. `0` before the first `prune()` call, when no
    /// [`RunTracePort`] is wired, when the pass found nothing to remove, or
    /// when the trace prune attempt itself failed (see `prune`'s own
    /// rustdoc for the failure-does-not-abort contract).
    pub fn last_run_traces_removed(&self) -> u64 {
        self.last_run_traces_removed.load(Ordering::SeqCst)
    }

    /// Run one retention pass.
    ///
    /// If `config.enabled` is `false`, this is a no-op that returns an empty
    /// [`PruneReport`] without reading `port` (or a wired `run_trace_port`)
    /// at all -- the same disabled-by-default contract (X-09)
    /// `WaypointRetentionConfig` itself documents.
    ///
    /// Otherwise the configured bounds are applied to the Waypoints first,
    /// exactly as before [`with_run_trace_port`](Self::with_run_trace_port)
    /// existed, and this call returns exactly that routine's report
    /// unchanged. When a [`RunTracePort`] is wired, this THEN prunes
    /// `run_traces` for every thread the Waypoint pass touched, with each
    /// thread's superstep boundary derived from the minimum `superstep`
    /// among that thread's SURVIVING Waypoints -- any `TraceRecord` older
    /// than every Waypoint the configured bounds kept is exactly what gets
    /// removed, so the trace prune runs under the SAME bounds without
    /// re-deriving the policy at the storage layer (X-01). A failing trace
    /// prune (or a failed boundary lookup) is logged and folded into a `0`
    /// contribution, observable via
    /// [`last_run_traces_removed`](Self::last_run_traces_removed); it never
    /// aborts or fails the Waypoint pruning that already completed.
    pub async fn prune(&self) -> Result<PruneReport, WaypointError> {
        if !self.config.enabled {
            self.last_run_traces_removed.store(0, Ordering::SeqCst);
            return Ok(PruneReport::default());
        }

        let report = prune(
            self.port.as_ref(),
            self.config.max_age_days,
            self.config.max_waypoints_per_thread,
            &protected_waypoints,
        )
        .await?;

        let removed = match &self.run_trace_port {
            Some(run_trace_port) => {
                self.prune_run_traces_for(&report, run_trace_port.as_ref())
                    .await
            }
            None => 0,
        };
        self.last_run_traces_removed
            .store(removed, Ordering::SeqCst);

        Ok(report)
    }

    /// Prune `run_traces` for every thread `waypoints` (the just-completed
    /// Waypoint prune's own report) touched. Never propagates a failure --
    /// see [`prune`](Self::prune)'s own rustdoc for why.
    async fn prune_run_traces_for(
        &self,
        waypoints: &PruneReport,
        run_trace_port: &dyn RunTracePort,
    ) -> u64 {
        let threads: Vec<ThreadId> = waypoints.iter().map(|(thread, _)| thread.clone()).collect();
        if threads.is_empty() {
            return 0;
        }

        let mut boundaries: HashMap<ThreadId, u64> = HashMap::with_capacity(threads.len());
        for thread in &threads {
            let boundary = match self.port.history(thread, None, None).await {
                Ok(history) => history.iter().map(|s| s.superstep).min().unwrap_or(0),
                Err(e) => {
                    log::error!(
                        target: "paladin::retention",
                        "could not compute a run_traces prune boundary for a thread, skipping it: {e}"
                    );
                    0
                }
            };
            boundaries.insert(thread.clone(), boundary);
        }

        match prune_run_traces(run_trace_port, &threads, |thread| {
            boundaries.get(thread).copied().unwrap_or(0)
        })
        .await
        {
            Ok(removed) => removed,
            Err(e) => {
                log::error!(
                    target: "paladin::retention",
                    "run_traces prune failed, waypoint pruning already completed: {e}"
                );
                0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use paladin_core::platform::container::waypoint::{
        NodeId, OnExpire, ParleyId, ParleyKind, ParleyRequest, Waypoint,
    };
    use paladin_ports::output::run_trace_port::{RunTraceError, TraceRecord};
    use paladin_storage::run_trace::contract_tests::sample_record as sample_trace_record;
    use paladin_storage::run_trace::in_memory::InMemoryRunTraceStore;
    use paladin_storage::waypoint::contract_tests::sample_waypoint_at;
    use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;

    /// A `RunTracePort` test double whose `prune_thread` always fails --
    /// for proving a trace-prune failure never aborts or fails the
    /// Waypoint pruning it is layered on top of.
    struct AlwaysFailingRunTraceStore;

    #[async_trait::async_trait]
    impl RunTracePort for AlwaysFailingRunTraceStore {
        async fn append(&self, _records: &[TraceRecord]) -> Result<(), RunTraceError> {
            Ok(())
        }

        async fn read(
            &self,
            _thread: &ThreadId,
            _after_seq: u64,
            _limit: u32,
        ) -> Result<Vec<TraceRecord>, RunTraceError> {
            Ok(vec![])
        }

        async fn prune_thread(
            &self,
            _thread: &ThreadId,
            _before_superstep: u64,
        ) -> Result<u64, RunTraceError> {
            Err(RunTraceError::Backend {
                source: "simulated failure".into(),
            })
        }
    }

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
            fork_of: wp.fork_of,
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
            parleys: vec![sample_parley_request()],
            responses: Vec::new(),
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
            parleys: vec![sample_parley_request()],
            responses: Vec::new(),
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

    // ── D-17: RunTracePort join (Phase 28 Plan 04) ───────────────────────

    #[tokio::test]
    async fn prune_without_run_trace_port_is_unchanged() {
        // No `.with_run_trace_port(..)` call: behaves exactly as before
        // this join existed (X-03).
        let store: Arc<dyn WaypointPort> = Arc::new(InMemoryWaypointStore::new());
        let t = thread("service-bounds-pass-through-no-run-trace");
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
        assert_eq!(service.last_run_traces_removed(), 0);
        assert_eq!(store.history(&t, None, None).await.unwrap().len(), 3);
    }

    #[tokio::test]
    async fn prune_run_traces_uses_the_waypoint_bounds() {
        let waypoint_store: Arc<dyn WaypointPort> = Arc::new(InMemoryWaypointStore::new());
        let run_trace_store: Arc<dyn RunTracePort> = Arc::new(InMemoryRunTraceStore::new());
        let t = thread("run-trace-retention-uses-waypoint-bounds");
        let base = Utc::now();

        // Ten Waypoints, superstep 0..10; max_waypoints_per_thread = 3
        // keeps only the three newest (superstep 7, 8, 9) -- a boundary of
        // 7 for the trace prune.
        for superstep in 0..10u64 {
            waypoint_store
                .save(&sample_waypoint_at(
                    &t,
                    superstep,
                    base + Duration::seconds(superstep as i64),
                ))
                .await
                .unwrap();
        }

        // Ten trace records, one per superstep over the same range.
        for superstep in 0..10u64 {
            run_trace_store
                .append(&[sample_trace_record(&t, superstep + 1, superstep)])
                .await
                .unwrap();
        }

        let service = WaypointRetentionService::new(
            waypoint_store.clone(),
            WaypointRetentionConfig {
                enabled: true,
                max_age_days: None,
                max_waypoints_per_thread: Some(3),
            },
        )
        .with_run_trace_port(run_trace_store.clone());

        let report = service.prune().await.unwrap();

        assert_eq!(report.removed_for(&t), 7);
        // Traces for superstep 0..6 (7 records) fall below the boundary
        // derived from the surviving Waypoints' minimum superstep (7).
        assert_eq!(service.last_run_traces_removed(), 7);

        let remaining_traces = run_trace_store.read(&t, 0, 100).await.unwrap();
        assert_eq!(remaining_traces.len(), 3);
    }

    #[tokio::test]
    async fn run_trace_prune_error_does_not_abort_waypoint_pruning() {
        let waypoint_store: Arc<dyn WaypointPort> = Arc::new(InMemoryWaypointStore::new());
        let t = thread("run-trace-retention-error-does-not-abort");
        let base = Utc::now();
        for superstep in 0..5u64 {
            waypoint_store
                .save(&sample_waypoint_at(
                    &t,
                    superstep,
                    base + Duration::seconds(superstep as i64),
                ))
                .await
                .unwrap();
        }

        let service = WaypointRetentionService::new(
            waypoint_store.clone(),
            WaypointRetentionConfig {
                enabled: true,
                max_age_days: None,
                max_waypoints_per_thread: Some(2),
            },
        )
        .with_run_trace_port(Arc::new(AlwaysFailingRunTraceStore));

        let report = service.prune().await.unwrap();

        // Waypoint pruning still completed despite the trace port always
        // failing.
        assert_eq!(report.removed_for(&t), 3);
        assert_eq!(service.last_run_traces_removed(), 0);
        assert_eq!(
            waypoint_store.history(&t, None, None).await.unwrap().len(),
            2
        );
    }
}
