//! Fault-injection and cancellation acceptance test for Waypoint retention
//! (ENG-FR-18, gap G-22-2, T-22-47/48/50).
//!
//! Three parts, each proving a different half of the same invariant --
//! interrupting a prune must never cost a thread the checkpoint it resumes
//! from:
//!
//! - **Part A** injects a backend failure part-way through a real prune
//!   (against the port's *provided* `prune_thread` default, which composes
//!   `history` + `delete_waypoint`) and proves the protected Waypoints
//!   survive, the remainder is a superset of the keep-set, and re-running
//!   converges.
//! - **Part B** proves the point of Part A in the terms the whole program is
//!   built on: a thread interrupted mid-prune still resumes, on a fresh
//!   engine, to the same final Battlefield as an uninterrupted control run.
//! - **Part C** proves the same survival property against the transactional
//!   SQLite override, where the mechanism is a cancelled task rather than an
//!   injected error.

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration as StdDuration;

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};

use paladin::application::services::waypoint_retention::WaypointRetentionService;
use paladin::config::WaypointRetentionConfig;

use paladin_battalion::engine::{
    EdgeSpec, EngineLimits, NodeContext, NodeError, NodeSpec, RunOutcome, StateNode, WarEngine,
    WarGraph,
};
use paladin_core::platform::container::battlefield::{
    Battlefield, BattlefieldSchema, DispatchRule, FieldName, FieldSpec, StateDelta,
};
use paladin_core::platform::container::directive::Directive;
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::waypoint::{
    NodeId, ParleyRequest, ThreadId, Waypoint, WaypointId, WaypointStatus,
};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream};
use paladin_ports::output::waypoint_port::{
    ThreadSummary, WaypointError, WaypointPort, WaypointSummary,
};
use paladin_storage::waypoint::contract_tests::sample_waypoint_at;
use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;
use paladin_storage::waypoint::sqlite::SqliteWaypointStore;

// ---------------------------------------------------------------------------
// Shared fixtures
// ---------------------------------------------------------------------------

/// A `PaladinPort` that is never called: every fixture graph in this file
/// uses only `NodeSpec::Function` nodes, but `WarEngine::new` requires a
/// port regardless (mirrors `war_engine_tracer_test.rs`'s
/// `UnimplementedPaladinPort`).
struct UnimplementedPaladinPort;

#[async_trait]
impl PaladinPort for UnimplementedPaladinPort {
    async fn execute(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinResult, PaladinError> {
        unimplemented!("this fixture only runs Function nodes")
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinStream, PaladinError> {
        unimplemented!("this fixture only runs Function nodes")
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

/// A `WaypointPort` test double that delegates every method to an inner
/// `InMemoryWaypointStore`, except `delete_waypoint`, which returns a
/// backend error once it has been called more than `fail_after` times.
/// Deliberately does NOT override `prune_thread`, so a caller invoking
/// `prune_thread` on this store takes the port's *provided* default
/// implementation (`history` + `delete_waypoint`), and the injected failure
/// lands part-way through that real composition -- exactly the path Plan
/// 22-14 rewrote `retention::prune` to depend on.
struct FaultyWaypointStore {
    inner: InMemoryWaypointStore,
    fail_after: usize,
    delete_calls: Mutex<usize>,
}

impl FaultyWaypointStore {
    fn new(inner: InMemoryWaypointStore, fail_after: usize) -> Self {
        Self {
            inner,
            fail_after,
            delete_calls: Mutex::new(0),
        }
    }
}

#[async_trait]
impl WaypointPort for FaultyWaypointStore {
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
        self.inner.delete_thread(thread).await
    }

    async fn delete_waypoint(
        &self,
        thread: &ThreadId,
        id: &WaypointId,
    ) -> Result<bool, WaypointError> {
        let call_number = {
            let mut count = self.delete_calls.lock().unwrap();
            *count += 1;
            *count
        };
        if call_number > self.fail_after {
            return Err(WaypointError::Backend {
                source: Box::<dyn std::error::Error + Send + Sync>::from(format!(
                    "injected delete_waypoint failure on call {call_number} (fail_after={})",
                    self.fail_after
                )),
            });
        }
        self.inner.delete_waypoint(thread, id).await
    }

    // `prune_thread` intentionally NOT overridden: inherits the port's
    // provided default, which is exactly what this decorator exists to
    // exercise under fault injection.
}

fn thread(name: &str) -> ThreadId {
    ThreadId::new(name).expect("valid thread id")
}

fn to_json(wp: &Waypoint) -> String {
    serde_json::to_string(wp).expect("waypoint must serialize")
}

// ---------------------------------------------------------------------------
// Part A: injected backend failure on the port's provided prune path
// ---------------------------------------------------------------------------

/// Seeds `store`'s `thread` with a mix that both configured bounds would
/// otherwise want to remove entirely if not for the protected-set carve-out:
/// an old `AwaitingInput` Waypoint, several recent plain Waypoints, and a
/// recent latest Waypoint. Returns `(latest, awaiting)`.
async fn seed_mixed_thread(store: &dyn WaypointPort, thread: &ThreadId) -> (Waypoint, Waypoint) {
    let now = Utc::now();

    let mut awaiting = sample_waypoint_at(thread, 0, now - Duration::days(365));
    awaiting.status = WaypointStatus::AwaitingInput {
        parley: ParleyRequest {
            prompt: "confirm?".to_string(),
        },
    };
    store.save(&awaiting).await.expect("seed awaiting waypoint");

    let mut latest = None;
    for superstep in 1..9u64 {
        let wp = sample_waypoint_at(thread, superstep, now);
        store.save(&wp).await.expect("seed plain waypoint");
        latest = Some(wp);
    }

    (
        latest.expect("at least one plain waypoint seeded"),
        awaiting,
    )
}

/// The config used throughout Part A: `max_age_days` alone would not touch
/// the recent plain waypoints, but `max_waypoints_per_thread` prunes all but
/// the newest 3 by position -- landing squarely on the zone the awaiting
/// waypoint and the age bound both threaten, so the protected-set carve-out
/// is what is actually under test.
fn part_a_config() -> WaypointRetentionConfig {
    WaypointRetentionConfig {
        enabled: true,
        max_age_days: Some(1),
        max_waypoints_per_thread: Some(3),
    }
}

/// With the fixture built by `seed_mixed_thread` (1 awaiting + 8 plain, 9
/// total) and `part_a_config`'s `max_waypoints_per_thread: Some(3)`: the
/// awaiting waypoint and the latest are protected outright; of the
/// remaining 7 plain waypoints, positions 1 and 2 survive the count bound
/// and positions 3..=7 (5 waypoints) do not. This constant is asserted by
/// the control run below rather than merely assumed.
const EXPECTED_TOTAL_DELETES: usize = 5;

#[tokio::test]
async fn part_a_fault_injection_sweep_leaves_protected_waypoints_and_converges() {
    // Control run: an unfaulted store seeded identically, pruned once to
    // convergence, establishes the exact target keep-set every faulted run
    // below must remain a superset of (and eventually converge to).
    let control_store = InMemoryWaypointStore::new();
    let control_thread = thread("fault-injection-part-a-control");
    let (control_latest, control_awaiting) =
        seed_mixed_thread(&control_store, &control_thread).await;

    let control_service =
        WaypointRetentionService::new(Arc::new(control_store.clone()), part_a_config());
    let control_report = control_service
        .prune()
        .await
        .expect("unfaulted control prune must succeed");
    assert_eq!(
        control_report.total_removed() as usize,
        EXPECTED_TOTAL_DELETES,
        "sanity check on the fixture's own arithmetic"
    );

    let target_keep_set: HashSet<WaypointId> = control_store
        .history(&control_thread, None, None)
        .await
        .unwrap()
        .into_iter()
        .map(|s| s.waypoint_id)
        .collect();
    assert!(target_keep_set.contains(&control_latest.waypoint_id));
    assert!(target_keep_set.contains(&control_awaiting.waypoint_id));

    // Sweep the injected failure across every delete position the fixture
    // can produce -- not one arbitrary position.
    for fail_after in 0..EXPECTED_TOTAL_DELETES {
        let inner = InMemoryWaypointStore::new();
        let t = thread(&format!("fault-injection-part-a-{fail_after}"));
        let (latest, awaiting) = seed_mixed_thread(&inner, &t).await;

        let faulty = Arc::new(FaultyWaypointStore::new(inner.clone(), fail_after));
        let service = WaypointRetentionService::new(faulty, part_a_config());

        let result = service.prune().await;
        assert!(
            result.is_err(),
            "fail_after={fail_after}: the injected backend failure must surface"
        );

        // The latest Waypoint still loads, byte-identical.
        let loaded_latest = inner
            .get(&t, &latest.waypoint_id)
            .await
            .unwrap()
            .unwrap_or_else(|| panic!("fail_after={fail_after}: latest waypoint must survive"));
        assert_eq!(to_json(&loaded_latest), to_json(&latest));

        // The AwaitingInput Waypoint still loads by id, payload
        // byte-identical.
        let loaded_awaiting = inner
            .get(&t, &awaiting.waypoint_id)
            .await
            .unwrap()
            .unwrap_or_else(|| {
                panic!("fail_after={fail_after}: AwaitingInput waypoint must survive")
            });
        assert_eq!(to_json(&loaded_awaiting), to_json(&awaiting));

        // What remains is a superset of the (renamed, but structurally
        // identical) target keep-set: extra survivors are fine, a missing
        // protected Waypoint is not. Positions, not ids, carry the meaning
        // here since each iteration seeds fresh ids -- compare counts and
        // membership of the two always-protected waypoints, already done
        // above, plus a superset-by-count sanity check.
        let remaining = inner.history(&t, None, None).await.unwrap();
        assert!(
            remaining.len() >= target_keep_set.len(),
            "fail_after={fail_after}: an interrupted prune must never remove more than intended \
             (remaining {} < target keep-set size {})",
            remaining.len(),
            target_keep_set.len()
        );

        // Re-run with fault injection disabled (talk to `inner` directly,
        // bypassing the decorator) and assert convergence: exactly the
        // target keep-set remains.
        let recovery_service =
            WaypointRetentionService::new(Arc::new(inner.clone()), part_a_config());
        recovery_service
            .prune()
            .await
            .expect("recovery prune without fault injection must succeed");
        let converged: HashSet<WaypointId> = inner
            .history(&t, None, None)
            .await
            .unwrap()
            .into_iter()
            .map(|s| s.waypoint_id)
            .collect();
        assert_eq!(
            converged.len(),
            target_keep_set.len(),
            "fail_after={fail_after}: recovery run must converge to exactly the keep-set size"
        );
        assert!(converged.contains(&latest.waypoint_id));
        assert!(converged.contains(&awaiting.waypoint_id));

        // A third run removes nothing further.
        let third_report = recovery_service
            .prune()
            .await
            .expect("third prune must succeed");
        assert_eq!(
            third_report.total_removed(),
            0,
            "fail_after={fail_after}: a third run after convergence must remove nothing"
        );
    }
}

// ---------------------------------------------------------------------------
// Part B: resume after an interrupted prune, on a real engine
// ---------------------------------------------------------------------------

/// A deterministic `Function` node that writes a fixed value into its own
/// field -- enough to make each superstep's Waypoint distinguishable and
/// the final Battlefield comparable, without needing an LLM.
struct SetFieldNode {
    field: FieldName,
    value: String,
}

#[async_trait]
impl StateNode for SetFieldNode {
    async fn run(&self, _state: &Battlefield, _ctx: &NodeContext) -> Result<Directive, NodeError> {
        let mut delta = StateDelta::new();
        delta
            .set(self.field.clone(), self.value.clone())
            .map_err(|e| NodeError(e.to_string()))?;
        Ok(delta.into())
    }
}

/// A 6-node straight-line chain, one Waypoint per node -- small on purpose
/// (per this plan's own instruction): the point of Part B is the resume,
/// not the graph.
fn build_small_chain_graph() -> WarGraph {
    let field_names: Vec<FieldName> = (0..6)
        .map(|i| FieldName::new(format!("step_{i}")).expect("valid field name"))
        .collect();
    let schema = BattlefieldSchema::new(
        field_names
            .iter()
            .cloned()
            .map(|f| FieldSpec::new(f, DispatchRule::LastWrite, None, false))
            .collect(),
    );
    let mut graph = WarGraph::new(schema, EngineLimits::default());

    let node_ids: Vec<NodeId> = (0..6).map(|i| NodeId::new(format!("step_{i}"))).collect();
    for (i, id) in node_ids.iter().enumerate() {
        graph.add_node(
            id.clone(),
            NodeSpec::Function(Arc::new(SetFieldNode {
                field: field_names[i].clone(),
                value: format!("v{i}"),
            })),
        );
    }
    for pair in node_ids.windows(2) {
        graph.add_edge(EdgeSpec {
            from: pair[0].clone(),
            to: pair[1].clone(),
            condition: None,
        });
    }
    graph.add_entry(node_ids[0].clone());
    graph
}

#[tokio::test]
async fn part_b_resume_after_interrupted_prune_matches_control_run() {
    let graph = build_small_chain_graph();

    // Control run: uninterrupted, no retention involved at all.
    let control_store = Arc::new(InMemoryWaypointStore::new());
    let control_thread = thread("fault-injection-part-b-control");
    let control_engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), control_store.clone());
    let control_outcome = control_engine
        .start(&graph, control_thread.clone(), StateDelta::new())
        .await
        .expect("control run should succeed");
    let control_final = match control_outcome {
        RunOutcome::Completed { final_state, .. } => final_state,
        other => panic!("expected control run to complete, got {other:?}"),
    };

    // Second run of the identical graph, whose backend we will
    // interrupt-prune afterward.
    let pruned_inner_store = InMemoryWaypointStore::new();
    let pruned_thread = thread("fault-injection-part-b-pruned");
    let seeding_engine = WarEngine::new(
        Arc::new(UnimplementedPaladinPort),
        Arc::new(pruned_inner_store.clone()),
    );
    seeding_engine
        .start(&graph, pruned_thread.clone(), StateDelta::new())
        .await
        .expect("seeding run should succeed");

    // Interrupt a prune against this thread's backend, as in Part A: fail
    // on the very first delete so the prune is interrupted as early as
    // possible into the real work.
    let faulty = Arc::new(FaultyWaypointStore::new(pruned_inner_store.clone(), 0));
    let service = WaypointRetentionService::new(
        faulty,
        WaypointRetentionConfig {
            enabled: true,
            max_age_days: None,
            max_waypoints_per_thread: Some(2),
        },
    );
    let interrupted = service.prune().await;
    assert!(
        interrupted.is_err(),
        "the prune against this thread must be interrupted by the injected failure"
    );

    // A brand new engine over the SAME (now partially pruned) backend,
    // bypassing the faulty decorator entirely -- the resume path never goes
    // through retention.
    let resumed_store = Arc::new(pruned_inner_store.clone());
    let resumed_engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), resumed_store);
    let resumed_outcome = resumed_engine
        .resume(&graph, pruned_thread.clone())
        .await
        .expect("resume must succeed after an interrupted prune");
    let resumed_final = match resumed_outcome {
        RunOutcome::Completed { final_state, .. } => final_state,
        other => panic!("expected resumed run to complete, got {other:?}"),
    };

    assert_eq!(
        resumed_final, control_final,
        "an interrupted prune must not cost the thread its ability to resume to the same final \
         Battlefield as an uninterrupted control run"
    );
}

// ---------------------------------------------------------------------------
// Part C: cancellation against the transactional SQLite override
// ---------------------------------------------------------------------------

fn temp_sqlite_url(label: &str) -> String {
    let path = std::env::temp_dir().join(format!(
        "waypoint_retention_fault_injection_{label}_{}.sqlite",
        uuid::Uuid::new_v4()
    ));
    format!("sqlite://{}", path.display())
}

/// Seeds `store`'s `thread` with 8 waypoints and returns the ids of the 3
/// newest -- the keep-set a `max_waypoints_per_thread: Some(3)` prune (with
/// no `AwaitingInput` waypoints in this fixture) would target.
async fn seed_sqlite_thread_and_keep_set(
    store: &SqliteWaypointStore,
    thread: &ThreadId,
) -> Vec<WaypointId> {
    let base = Utc::now();
    let mut ids = Vec::with_capacity(8);
    for superstep in 0..8u64 {
        let wp = sample_waypoint_at(
            thread,
            superstep,
            base + Duration::seconds(superstep as i64),
        );
        store.save(&wp).await.expect("seed sqlite waypoint");
        ids.push(wp.waypoint_id);
    }
    ids[5..8].to_vec()
}

#[tokio::test]
async fn part_c_aborted_prune_against_sqlite_leaves_keep_set_intact() {
    // Bound the whole sweep well under a second: a handful of short,
    // increasing abort delays is enough to land the abort both before and
    // after the transaction commits without turning this into a soak test.
    const ABORT_DELAYS_MS: [u64; 6] = [0, 1, 2, 5, 10, 25];

    for (i, delay_ms) in ABORT_DELAYS_MS.iter().enumerate() {
        let db_url = temp_sqlite_url(&format!("abort-{i}"));
        let store = Arc::new(
            SqliteWaypointStore::new(&db_url)
                .await
                .expect("sqlite store should connect"),
        );
        let t = thread(&format!("fault-injection-part-c-abort-{i}"));
        let keep_set = seed_sqlite_thread_and_keep_set(&store, &t).await;

        let spawned_store = store.clone();
        let spawned_thread = t.clone();
        let spawned_keep = keep_set.clone();
        let handle = tokio::spawn(async move {
            spawned_store
                .prune_thread(&spawned_thread, &spawned_keep)
                .await
        });

        tokio::time::sleep(StdDuration::from_millis(*delay_ms)).await;
        handle.abort();
        // Whether the abort landed before or after the transaction
        // committed is exactly the race this test exists to be indifferent
        // to; either outcome (join error from cancellation, or a completed
        // Ok/Err result) is acceptable here.
        let _ = handle.await;

        for id in &keep_set {
            assert!(
                store.get(&t, id).await.unwrap().is_some(),
                "delay={delay_ms}ms: keep-set id {id} must survive an aborted prune, whether the \
                 abort landed before or after the transaction committed"
            );
        }
    }

    // Finally, run a prune to completion (no abort) and assert convergence
    // to exactly the keep-set.
    let db_url = temp_sqlite_url("converge");
    let store = SqliteWaypointStore::new(&db_url)
        .await
        .expect("sqlite store should connect");
    let t = thread("fault-injection-part-c-converge");
    let keep_set = seed_sqlite_thread_and_keep_set(&store, &t).await;

    let removed = store
        .prune_thread(&t, &keep_set)
        .await
        .expect("uninterrupted prune must succeed");
    assert_eq!(removed, 5);

    let remaining: HashSet<WaypointId> = store
        .history(&t, None, None)
        .await
        .unwrap()
        .into_iter()
        .map(|s| s.waypoint_id)
        .collect();
    assert_eq!(remaining, keep_set.into_iter().collect::<HashSet<_>>());
}
