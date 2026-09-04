//! Program acceptance scenario E2E-3 -- the **muster/defer/order half**,
//! `.project/v0.10.0/00-program-overview.md` §6: a planner node musters N
//! worker tasks that run concurrently in one superstep, a `defer: true`
//! aggregator downstream runs exactly once after every task resolves, and
//! the aggregated Battlefield field holds every worker's result in
//! deterministic `task_key` order rather than completion order
//! (`.project/v0.10.0/02-control-flow-routing-fanout-subgraphs.md` §3
//! acceptance criterion 2).
//!
//! ## Scope: this file covers ONLY the muster/defer/order half of E2E-3
//!
//! E2E-3's full program text also describes a recovering worker: one
//! mustered task that fails on its first attempts and succeeds later,
//! standing in for a per-task Aegis retry policy. §3 acceptance criterion 2
//! is explicit that the muster/defer/order half passes NOW, and that the
//! recovering-worker half is exercised here with a manually-succeeding
//! mock rather than a real retry mechanism -- typed per-task retry,
//! timeouts and error handlers are **FT-FR-06, owned by Phase 25**. See
//! `one_worker_recovers_by_manual_attempt_scripting` below for the exact,
//! clearly marked seam a real Aegis retry policy will replace.
//!
//! ## Why a `Function` planner rather than a Paladin planner
//!
//! `FaultyPaladinPort` (this file's Paladin mock, from `tests/helpers/`)
//! always returns a fixed `"FaultyPaladinPort: {name} processed {input}"`
//! string -- it cannot script a JSON `Directive` envelope, which
//! `DirectiveParser::StructuredDirective` would need in order to parse a
//! `NextStep::Muster(..)` out of a Paladin's own output (that extraction
//! logic is already covered end-to-end by `directive_parser.rs`'s own unit
//! tests). A deterministic `Function` planner -- mirroring
//! `e2e_crash_resume_test.rs`'s `LoopGateNode`, itself a `Function` node
//! driving control flow -- returns the `Muster` directive directly. The
//! mustered WORKERS themselves are real `Paladin` nodes dispatched through
//! `FaultyPaladinPort`, which is what this scenario is actually about: fan
//! out through the genuine Paladin-execution path, in one superstep, with
//! deterministic `task_key`-ordered aggregation.

use std::sync::Arc;

use paladin_battalion::engine::{
    EdgeSpec, EngineLimits, InputMapping, NodeContext, NodeError, NodeSpec, RunOutcome, StateNode,
    WarEngine, WarGraph,
};
use paladin_core::base::entity::node::Node;
use paladin_core::platform::container::battlefield::{
    Battlefield, BattlefieldSchema, DispatchRule, FieldName, FieldSpec, StateDelta,
};
use paladin_core::platform::container::directive::{Directive, MusterTask, NextStep};
use paladin_core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus};
use paladin_core::platform::container::waypoint::{NodeId, ThreadId, Waypoint};
use paladin_ports::output::paladin_port::PaladinPort;
use paladin_ports::output::waypoint_port::WaypointPort;
use paladin_storage::waypoint::sqlite::SqliteWaypointStore;

// `tests/helpers/` is shared across many integration test binaries; this
// standalone [[test]] target only needs `FaultyPaladinPort`, so the rest of
// the module tree is unused here -- allowed rather than pruned, matching
// `e2e_crash_resume_test.rs`'s own precedent for this exact situation.
#[allow(dead_code, unused_imports)]
#[path = "../helpers/mod.rs"]
mod helpers;
use helpers::FaultyPaladinPort;

/// The five mustered task keys, already in lexicographic (`String` byte)
/// order -- CF-FR-11's ordering guarantee is proven under real concurrency
/// at the unit level by `engine::superstep::tests::
/// worker_deltas_merge_in_task_key_order_not_completion_order`; this file's
/// job is to prove the SAME guarantee holds through the full engine +
/// real-Paladin-dispatch path, not to re-derive it.
const TASK_KEYS: [&str; 5] = ["a", "b", "c", "d", "e"];

fn field(name: &str) -> FieldName {
    FieldName::new(name).expect("valid field name")
}

fn make_paladin(name: &str) -> Paladin {
    let data = PaladinData {
        system_prompt: format!("{name} prompt"),
        name: name.to_string(),
        user_name: "TestUser".to_string(),
        model: "test-model".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    Node::new(data, Some(name.to_string()))
}

/// Deterministic planner: on its one (and only) execution, musters five
/// worker tasks against the `worker` template, keyed `"a"`..`"e"`, each
/// carrying its own key as a JSON string payload (`{muster.payload}`
/// resolves to the bare key string -- see `InputMapping::resolve_muster`).
struct PlannerNode;

#[async_trait::async_trait]
impl StateNode for PlannerNode {
    async fn run(&self, _state: &Battlefield, _ctx: &NodeContext) -> Result<Directive, NodeError> {
        let worker = NodeId::new("worker");
        let tasks = TASK_KEYS
            .iter()
            .map(|key| MusterTask {
                worker: worker.clone(),
                payload: serde_json::json!(*key),
                task_key: key.to_string(),
            })
            .collect();
        Ok(Directive {
            delta: StateDelta::new(),
            next: NextStep::Muster(tasks),
        })
    }
}

/// Deferred aggregator (`defer: true`): reads the worker template's
/// `Append`-dispatched `worker_out` field -- exactly 5 entries once every
/// mustered task has resolved -- and copies it, unchanged, into
/// `aggregated`, the list-dispatch Battlefield field D-17 names.
struct AggregatorNode {
    worker_out: FieldName,
    aggregated: FieldName,
}

#[async_trait::async_trait]
impl StateNode for AggregatorNode {
    async fn run(&self, state: &Battlefield, _ctx: &NodeContext) -> Result<Directive, NodeError> {
        let results = state
            .get::<Vec<String>>(&self.worker_out)
            .map_err(|e| NodeError(e.to_string()))?
            .unwrap_or_default();
        let mut delta = StateDelta::new();
        delta.set_raw(self.aggregated.clone(), serde_json::json!(results));
        Ok(delta.into())
    }
}

/// Build the E2E-3 muster/defer/order fixture: `planner` (Function, entry,
/// one-shot `Muster` of 5 tasks) `-> worker` (Paladin worker template, no
/// static incoming edge -- dispatched only when mustered, D-12) `->
/// aggregator` (Function, `defer: true`, runs once after all 5 resolve).
fn build_graph() -> WarGraph {
    let worker_out = field("worker_out");
    let aggregated = field("aggregated");
    let schema = BattlefieldSchema::new(vec![
        FieldSpec::new(worker_out.clone(), DispatchRule::Append, None, false),
        FieldSpec::new(aggregated.clone(), DispatchRule::LastWrite, None, false),
    ]);
    let mut graph = WarGraph::new(schema, EngineLimits::default());

    let planner = NodeId::new("planner");
    let worker = NodeId::new("worker");
    let aggregator = NodeId::new("aggregator");

    graph.add_node(planner.clone(), NodeSpec::Function(Arc::new(PlannerNode)));
    graph.add_worker_template(
        worker.clone(),
        NodeSpec::paladin(
            make_paladin("worker"),
            InputMapping::new("{muster.payload}"),
            worker_out.clone(),
        ),
    );
    graph.add_deferred_node(
        aggregator.clone(),
        NodeSpec::Function(Arc::new(AggregatorNode {
            worker_out: worker_out.clone(),
            aggregated: aggregated.clone(),
        })),
    );
    graph.add_edge(EdgeSpec {
        from: worker.clone(),
        to: aggregator.clone(),
        condition: None,
    });
    graph.add_entry(planner);

    graph
}

fn temp_db_url(label: &str) -> String {
    let path = std::env::temp_dir().join(format!(
        "e2e_muster_defer_order_{label}_{}.sqlite",
        uuid::Uuid::new_v4()
    ));
    format!("sqlite://{}", path.display())
}

async fn full_history(store: &SqliteWaypointStore, thread: &ThreadId) -> Vec<Waypoint> {
    let summaries = store
        .history(thread, None, None)
        .await
        .expect("history should succeed");
    let mut waypoints = Vec::with_capacity(summaries.len());
    for summary in summaries {
        let wp = store
            .get(thread, &summary.waypoint_id)
            .await
            .expect("get should succeed")
            .expect("summary's own waypoint must exist");
        waypoints.push(wp);
    }
    waypoints.sort_by_key(|w| w.superstep);
    waypoints
}

/// The exact 5 strings `FaultyPaladinPort` produces for the worker
/// template's dispatch, in `task_key` order -- `{muster.payload}` renders a
/// JSON string payload as its bare (unquoted) text, so `PaladinResult.output`
/// reads `"FaultyPaladinPort: worker processed <key>"`.
fn expected_worker_outputs() -> Vec<String> {
    TASK_KEYS
        .iter()
        .map(|key| format!("FaultyPaladinPort: worker processed {key}"))
        .collect()
}

#[tokio::test]
async fn planner_musters_five_workers_and_the_deferred_aggregator_runs_once() {
    let graph = build_graph();
    let store = Arc::new(
        SqliteWaypointStore::new(&temp_db_url("basic"))
            .await
            .expect("store should connect"),
    );
    let port = Arc::new(FaultyPaladinPort::new());
    let thread = ThreadId::new("e2e-3-basic").expect("valid thread id");
    let engine = WarEngine::new(port.clone(), store.clone());

    let outcome = engine
        .start(&graph, thread.clone(), StateDelta::new())
        .await
        .expect("run should succeed");
    assert!(
        matches!(outcome, RunOutcome::Completed { .. }),
        "expected the run to complete: {outcome:?}"
    );

    // Exactly 5 worker executions: the port's own execution log names
    // "worker" once per mustered task, never more, never fewer.
    let log = port.execution_log();
    let worker_calls = log
        .iter()
        .filter(|entry| entry.starts_with("worker:"))
        .count();
    assert_eq!(
        worker_calls, 5,
        "exactly 5 worker executions, no more, no fewer"
    );

    // Read back the persisted history and confirm: exactly 5 worker
    // completion records at ONE shared superstep, exactly 1 aggregator
    // completion record, at a superstep STRICTLY greater than the workers'.
    // Only the "superstep complete" Waypoints (muster_progress: None) are
    // consulted here -- a Muster's intra-superstep progress Waypoints
    // (muster_progress: Some) each carry a cumulative, still-growing
    // snapshot of `completed` as tasks finish one at a time, and counting
    // across ALL of them would multiply-count the same task completions.
    let history = full_history(&store, &thread).await;
    let superstep_complete: Vec<&Waypoint> = history
        .iter()
        .filter(|w| w.muster_progress.is_none())
        .collect();

    let worker_id = NodeId::new("worker");
    let aggregator_id = NodeId::new("aggregator");
    let worker_supersteps: Vec<u64> = superstep_complete
        .iter()
        .flat_map(|w| {
            w.completed
                .iter()
                .filter(|r| r.node_id == worker_id)
                .map(move |_| w.superstep)
        })
        .collect();
    let aggregator_supersteps: Vec<u64> = superstep_complete
        .iter()
        .flat_map(|w| {
            w.completed
                .iter()
                .filter(|r| r.node_id == aggregator_id)
                .map(move |_| w.superstep)
        })
        .collect();

    assert_eq!(
        worker_supersteps.len(),
        5,
        "all five worker tasks must be recorded as having run"
    );
    assert_eq!(
        worker_supersteps
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        1,
        "all five worker tasks must run in the SAME superstep (CF-03: same-superstep fan-out)"
    );
    assert_eq!(
        aggregator_supersteps.len(),
        1,
        "the deferred aggregator must run exactly once"
    );

    let max_worker_superstep = *worker_supersteps.iter().max().unwrap();
    assert!(
        aggregator_supersteps[0] > max_worker_superstep,
        "the aggregator's superstep ({}) must be strictly greater than the workers' ({})",
        aggregator_supersteps[0],
        max_worker_superstep
    );
}

#[tokio::test]
async fn aggregated_results_are_exactly_five_in_task_key_order() {
    let graph = build_graph();
    let store = Arc::new(
        SqliteWaypointStore::new(&temp_db_url("order"))
            .await
            .expect("store should connect"),
    );
    let port = Arc::new(FaultyPaladinPort::new());
    let thread = ThreadId::new("e2e-3-order").expect("valid thread id");
    let engine = WarEngine::new(port, store);

    let outcome = engine
        .start(&graph, thread, StateDelta::new())
        .await
        .expect("run should succeed");
    match outcome {
        RunOutcome::Completed { final_state, .. } => {
            let aggregated = final_state
                .get::<Vec<String>>(&field("aggregated"))
                .expect("aggregated field should deserialize as Vec<String>");
            assert_eq!(
                aggregated,
                Some(expected_worker_outputs()),
                "the aggregated field must hold exactly 5 results in task_key order, not \
                 completion order"
            );
        }
        other => panic!("expected Completed, got {other:?}"),
    }
}

#[tokio::test]
async fn one_worker_recovers_by_manual_attempt_scripting() {
    // ============================================================
    // PHASE 25 SEAM (FT-FR-06) -- REPLACE THE BLOCK BELOW, NOT AROUND IT
    // ============================================================
    // `FaultyPaladinPort::fail_until_attempt`'s counter is GLOBAL, shared
    // across every `execute` call made through ONE port instance -- never
    // scoped per Paladin (`tests/helpers/mock_paladin_port.rs`'s own doc
    // comment). There is no real per-task retry mechanism in this phase: a
    // mustered worker's `PaladinPort::execute` failure fails the whole run
    // (CF-03's Muster validation/dispatch has no retry-in-place). D-17
    // scripts the "this worker recovered by attempt N" scenario manually
    // instead: configure the port to fail while its counter is <= 2, then
    // manually drive two "warm-up" `execute` calls through the SAME port
    // instance BEFORE the real muster run below. Every REAL Paladin call
    // the engine itself makes during the muster then lands past the
    // threshold and succeeds -- standing in for a worker that failed twice
    // and succeeded on its third (real) attempt.
    //
    // When FT-FR-06 lands: replace this scripted-warm-up block with a real
    // `WarEngine`-level (or `PaladinPort`-level) per-task retry policy that
    // lets a GENUINELY failing mustered task retry in place, inside the
    // same run, without any test-side pre-scripting. The assertions at the
    // end of this test -- exactly 5 results, all present, in task_key order
    // -- are the contract the real retry policy must continue to satisfy.
    let port = Arc::new(FaultyPaladinPort::new().fail_until_attempt(2));
    let warmup_paladin = make_paladin("warmup");
    for attempt in 1..=2 {
        let result = port.execute(&warmup_paladin, "warmup").await;
        assert!(
            result.is_err(),
            "manual attempt scripting: warm-up attempt {attempt} must fail, standing in for \
             the pre-recovery attempts a real Aegis retry policy would absorb"
        );
    }
    assert_eq!(
        port.call_count(),
        2,
        "exactly 2 scripted warm-up attempts before the real muster run"
    );
    // ============================================================
    // END PHASE 25 SEAM
    // ============================================================

    let graph = build_graph();
    let store = Arc::new(
        SqliteWaypointStore::new(&temp_db_url("recover"))
            .await
            .expect("store should connect"),
    );
    let thread = ThreadId::new("e2e-3-recover").expect("valid thread id");
    let engine = WarEngine::new(port.clone(), store);

    let outcome = engine
        .start(&graph, thread, StateDelta::new())
        .await
        .expect(
            "the run must still succeed: the shared counter is already past the \
             fail_until_attempt threshold by the time the engine dispatches any real worker call",
        );
    match outcome {
        RunOutcome::Completed { final_state, .. } => {
            let aggregated = final_state
                .get::<Vec<String>>(&field("aggregated"))
                .expect("aggregated field should deserialize as Vec<String>");
            assert_eq!(
                aggregated,
                Some(expected_worker_outputs()),
                "the run still produces all 5 results despite the recovering worker's scripted \
                 pre-recovery failures"
            );
        }
        other => panic!("expected Completed, got {other:?}"),
    }
    // 2 scripted warm-up calls + 5 real muster dispatches.
    assert_eq!(
        port.call_count(),
        7,
        "2 scripted warm-up attempts plus exactly 5 real worker executions"
    );
}

#[tokio::test]
async fn run_completes_with_a_single_superstep_complete_waypoint_per_superstep() {
    // ENG-FR-11 (D-14's clarification): exactly one superstep-COMPLETE
    // Waypoint per superstep is unchanged; a Muster may additionally write
    // zero-or-more `Running`-status progress Waypoints inside its own
    // superstep, counted SEPARATELY from that one-per-superstep guarantee.
    let graph = build_graph();
    let store = Arc::new(
        SqliteWaypointStore::new(&temp_db_url("waypoint-count"))
            .await
            .expect("store should connect"),
    );
    let port = Arc::new(FaultyPaladinPort::new());
    let thread = ThreadId::new("e2e-3-waypoint-count").expect("valid thread id");
    let engine = WarEngine::new(port, store.clone());

    let outcome = engine
        .start(&graph, thread.clone(), StateDelta::new())
        .await
        .expect("run should succeed");
    assert!(matches!(outcome, RunOutcome::Completed { .. }));

    let history = full_history(&store, &thread).await;

    let mut superstep_complete_indices: Vec<u64> = history
        .iter()
        .filter(|w| w.muster_progress.is_none())
        .map(|w| w.superstep)
        .collect();
    superstep_complete_indices.sort_unstable();
    let mut deduped = superstep_complete_indices.clone();
    deduped.dedup();
    assert_eq!(
        superstep_complete_indices, deduped,
        "exactly one superstep-complete Waypoint (muster_progress: None) per superstep index"
    );

    // The muster superstep specifically wrote progress Waypoints ALONGSIDE
    // its one completion Waypoint -- proving the two are counted
    // separately, never that per-task progress Waypoints replaced the
    // one-per-superstep completion guarantee.
    let progress_count = history
        .iter()
        .filter(|w| w.muster_progress.is_some())
        .count();
    assert_eq!(
        progress_count, 5,
        "one Running progress Waypoint per completed muster task, counted separately from the \
         one superstep-complete Waypoint per superstep"
    );
}
