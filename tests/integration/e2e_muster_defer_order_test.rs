//! Program acceptance scenario E2E-3 -- the **muster/defer/order half**,
//! `.project/v0.10.0/00-program-overview.md` §6: a planner node musters N
//! worker tasks that run concurrently in one superstep, a `defer: true`
//! aggregator downstream runs exactly once after every task resolves, and
//! the aggregated Battlefield field holds every worker's result in
//! deterministic `task_key` order rather than completion order
//! (`.project/v0.10.0/02-control-flow-routing-fanout-subgraphs.md` §3
//! acceptance criterion 2).
//!
//! ## Scope: the muster/defer/order half AND the recovering worker
//!
//! E2E-3's full program text also describes a recovering worker: one
//! mustered task that fails on its first attempts and succeeds later under
//! a per-task Aegis retry policy. Phase 23 exercised that half with a
//! manually-scripted mock because no retry mechanism existed yet; Phase 25
//! (FT-FR-06, D-31) replaced that stand-in with the real thing --
//! `one_worker_recovers_by_real_per_task_retry` below drives a GENUINELY
//! failing mustered Paladin through the engine's own per-task retry loop,
//! under the DEFAULT `TransientOnly` predicate, with no test-side
//! pre-scripting of any kind. The exact port-call count (5 workers + 2
//! retries = 7) and the recovering task's two `AttemptRecord`s are what
//! prove the retry was real.
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
//!
//! ## Why the recovering-worker graph has one worker template per task
//!
//! The engine dispatches every mustered task with its worker TEMPLATE's own
//! `Paladin` (`superstep.rs`'s `execute_observed(&paladin, ..)` call), so a
//! single shared template presents the same Paladin name on every task and
//! neither `FaultyPaladinPort::fail_paladin_until_attempt` (keyed by that
//! name) nor `NodeExecutionRecord.node_id` (the template id) could single
//! out "the third task". The recovering-worker fixture therefore registers
//! five worker templates `w1`..`w5` -- each a real Paladin node carrying the
//! per-task `RetryPolicy` -- and musters one task against each, keyed
//! `"a"`..`"e"` exactly as the shared-template fixture does. Same-superstep
//! fan-out, `task_key`-ordered aggregation and the one-aggregator-run
//! contract are unchanged by that choice; what it buys is a `w3` that is
//! addressable by value in the port, in the records and in the assertions.
//!
//! These tests live under `tests/integration/`, so the repository's default
//! `make test` (`--lib --bins`) does not run them; run
//! `cargo test --test e2e_muster_defer_order`.

use std::sync::Arc;

use paladin_battalion::engine::{RunOutcome, WarEngine};
use paladin_core::platform::container::aegis::{RetryPolicy, RetryPredicate};
use paladin_core::platform::container::battlefield::StateDelta;
use paladin_core::platform::container::node_error::NodeErrorSource;
use paladin_core::platform::container::transience::Transience;
use paladin_core::platform::container::waypoint::{
    NodeId, NodeOutcomeKind, ThreadId, Waypoint, WaypointStatus,
};
use paladin_ports::output::waypoint_port::WaypointPort;
use paladin_storage::waypoint::sqlite::SqliteWaypointStore;

// `tests/helpers/` is shared across many integration test binaries; this
// standalone [[test]] target needs `FaultyPaladinPort` and the shared
// `e2e_fixtures` graph builders (plan 28-16, D-34) -- the rest of the module
// tree is unused here, allowed rather than pruned, matching
// `e2e_crash_resume_test.rs`'s own precedent for this exact situation.
#[allow(dead_code, unused_imports)]
#[path = "../helpers/mod.rs"]
mod helpers;
use helpers::FaultyPaladinPort;
use helpers::e2e_fixtures::{self, RECOVERING_WORKER, TASK_KEYS, WORKER_NAMES};

// The five mustered task keys (`TASK_KEYS`), the five worker templates
// (`WORKER_NAMES`), the recovering worker (`RECOVERING_WORKER`), the
// `PlannerNode`/`AggregatorNode` fixtures, and both graph builders
// (`build_muster_defer_order_graph` / `_with_a_template_per_task`) now live
// in `tests/helpers/e2e_fixtures.rs` -- shared verbatim with the eval
// harness (plan 28-16, D-34).

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

/// The recovering-worker fixture's counterpart of
/// [`expected_worker_outputs`]: `w1` ran `"a"`, .., `w5` ran `"e"`, so the
/// aggregated list reads `"FaultyPaladinPort: w<i> processed <key>"` in
/// `task_key` order.
fn expected_per_template_worker_outputs() -> Vec<String> {
    WORKER_NAMES
        .iter()
        .zip(TASK_KEYS.iter())
        .map(|(worker, key)| format!("FaultyPaladinPort: {worker} processed {key}"))
        .collect()
}

#[tokio::test]
async fn planner_musters_five_workers_and_the_deferred_aggregator_runs_once() {
    let graph = e2e_fixtures::build_muster_defer_order_graph();
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
    let graph = e2e_fixtures::build_muster_defer_order_graph();
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
                .get::<Vec<String>>(&e2e_fixtures::field("aggregated"))
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

/// E2E-3's recovering-worker half, through a REAL per-task Aegis retry
/// (FT-FR-06, D-31): `w3` fails its own first two calls with a
/// Transient-classified `LlmFailure { status: Some(503) }` and succeeds on
/// its third, retried in place by the engine under the DEFAULT
/// `TransientOnly` predicate -- no test-side pre-scripting, no widened
/// predicate, no relaxed counts. The port sees exactly 7 calls (5 workers +
/// 2 retries of `w3`), `w3`'s record reads `attempt: 3` with two
/// `AttemptRecord`s, every other worker ran once, the aggregator ran once,
/// the aggregated field holds all 5 results in `task_key` order, and the
/// muster superstep wrote ONE superstep-complete Waypoint plus Phase 23's
/// five progress Waypoints -- none between `w3`'s attempts (FT-FR-07).
#[tokio::test]
async fn one_worker_recovers_by_real_per_task_retry() {
    let port = Arc::new(FaultyPaladinPort::new().fail_paladin_until_attempt(RECOVERING_WORKER, 2));
    let graph = e2e_fixtures::build_muster_defer_order_graph_with_a_template_per_task(Some(
        e2e_fixtures::per_task_retry_policy(),
    ));
    let store = Arc::new(
        SqliteWaypointStore::new(&temp_db_url("recover"))
            .await
            .expect("store should connect"),
    );
    let thread = ThreadId::new("e2e-3-recover").expect("valid thread id");
    let engine = WarEngine::new(port.clone(), store.clone());

    let outcome = engine
        .start(&graph, thread.clone(), StateDelta::new())
        .await
        .expect("the run must succeed: w3's two transient failures are retried in place");
    match outcome {
        RunOutcome::Completed { final_state, .. } => {
            let aggregated = final_state
                .get::<Vec<String>>(&e2e_fixtures::field("aggregated"))
                .expect("aggregated field should deserialize as Vec<String>");
            assert_eq!(
                aggregated,
                Some(expected_per_template_worker_outputs()),
                "the run still produces all 5 results, in task_key order, despite the \
                 recovering worker's two real failures"
            );
        }
        other => panic!("expected Completed, got {other:?}"),
    }

    // --- Exactly 7 port calls: 5 workers + 2 retries of w3, nothing else.
    assert_eq!(
        port.call_count(),
        7,
        "exactly 5 worker executions plus exactly 2 retries of w3 -- no scripted calls"
    );
    let log = port.execution_log();
    for name in WORKER_NAMES {
        let calls = log
            .iter()
            .filter(|entry| entry.starts_with(&format!("{name}:")))
            .count();
        let expected = if name == RECOVERING_WORKER { 3 } else { 1 };
        assert_eq!(
            calls, expected,
            "{name} must be called exactly {expected} time(s)"
        );
    }

    // --- Records: w3 at attempt 3 with two AttemptRecords numbered 1 and
    // 2, each carrying the Transient 503 by value; every other worker at
    // attempt 1 with an empty history; all five Succeeded.
    let history = full_history(&store, &thread).await;
    let superstep_complete: Vec<&Waypoint> = history
        .iter()
        .filter(|w| w.muster_progress.is_none())
        .collect();
    let w3 = NodeId::new(RECOVERING_WORKER);
    let muster_waypoint = superstep_complete
        .iter()
        .find(|w| w.completed.iter().any(|r| r.node_id == w3))
        .expect("the muster superstep's completion Waypoint records w3");
    for name in WORKER_NAMES {
        let id = NodeId::new(name);
        let records: Vec<_> = muster_waypoint
            .completed
            .iter()
            .filter(|r| r.node_id == id)
            .collect();
        assert_eq!(records.len(), 1, "{name} has exactly one record");
        let record = records[0];
        assert_eq!(
            record.outcome,
            NodeOutcomeKind::Succeeded,
            "{name} succeeded"
        );
        if name == RECOVERING_WORKER {
            assert_eq!(record.attempt, 3, "w3 succeeded on its third attempt");
            assert_eq!(
                record.attempts.len(),
                2,
                "w3 carries exactly two failed AttemptRecords"
            );
            let numbers: Vec<u32> = record.attempts.iter().map(|a| a.attempt).collect();
            assert_eq!(
                numbers,
                vec![1, 2],
                "the failed attempts are numbered 1 and 2"
            );
            for attempt in &record.attempts {
                assert_eq!(attempt.error.node_id, w3);
                assert_eq!(attempt.error.attempt, attempt.attempt);
                assert_eq!(attempt.error.transience, Transience::Transient);
                assert!(
                    matches!(
                        &attempt.error.source,
                        NodeErrorSource::Llm {
                            status: Some(503),
                            ..
                        }
                    ),
                    "the attempt's error carries the 503 by value: {:?}",
                    attempt.error.source
                );
            }
        } else {
            assert_eq!(record.attempt, 1, "{name} succeeded on its first attempt");
            assert!(record.attempts.is_empty(), "{name} has no failed attempts");
        }
    }

    // --- The aggregator ran exactly once.
    let aggregator_id = NodeId::new("aggregator");
    let aggregator_runs = superstep_complete
        .iter()
        .flat_map(|w| w.completed.iter())
        .filter(|r| r.node_id == aggregator_id)
        .count();
    assert_eq!(
        aggregator_runs, 1,
        "the deferred aggregator must run exactly once"
    );

    // --- Waypoints: ONE superstep-complete Waypoint for the muster
    // superstep, plus Phase 23's five progress Waypoints (one per completed
    // task), and none between w3's attempts -- the whole thread is exactly
    // planner (1) + muster (1 + 5 progress) + aggregator (1) = 8, with no
    // Failed status anywhere.
    let muster_superstep = muster_waypoint.superstep;
    let muster_complete_count = superstep_complete
        .iter()
        .filter(|w| w.superstep == muster_superstep)
        .count();
    assert_eq!(
        muster_complete_count, 1,
        "exactly one superstep-complete Waypoint for the muster superstep"
    );
    let progress_count = history
        .iter()
        .filter(|w| w.muster_progress.is_some())
        .count();
    assert_eq!(
        progress_count, 5,
        "one progress Waypoint per COMPLETED task -- a failed attempt writes none"
    );
    assert_eq!(
        history.len(),
        8,
        "planner + (muster + 5 progress) + aggregator: no Waypoint between w3's attempts"
    );
    assert!(
        history
            .iter()
            .all(|w| !matches!(w.status, WaypointStatus::Failed { .. })),
        "a retried-and-recovered attempt never persists a Failed Waypoint"
    );
}

/// The negative control for T-25-59 ("a green E2E-3 that proves nothing"):
/// the SAME fixture and the SAME failing port with NO retry policy attached
/// does not complete -- w3's transient failure fails the run on its first
/// and only attempt, and the port sees exactly 5 calls. Whatever makes the
/// scenario above green is therefore the per-task retry, not the fixture.
#[tokio::test]
async fn without_a_retry_policy_the_same_transient_failure_fails_the_run() {
    let port = Arc::new(FaultyPaladinPort::new().fail_paladin_until_attempt(RECOVERING_WORKER, 2));
    let graph = e2e_fixtures::build_muster_defer_order_graph_with_a_template_per_task(None);
    let store = Arc::new(
        SqliteWaypointStore::new(&temp_db_url("no-retry"))
            .await
            .expect("store should connect"),
    );
    let thread = ThreadId::new("e2e-3-no-retry").expect("valid thread id");
    let engine = WarEngine::new(port.clone(), store);

    let outcome = engine
        .start(&graph, thread, StateDelta::new())
        .await
        .expect("the engine itself returns Ok(RunOutcome::Failed), never Err");
    assert!(
        matches!(outcome, RunOutcome::Failed { .. }),
        "with no retry policy the transient failure fails the run: {outcome:?}"
    );
    assert_eq!(
        port.call_count(),
        5,
        "one call per worker and no retry of w3 -- exactly 5"
    );
    let w3_calls = port
        .execution_log()
        .iter()
        .filter(|entry| entry.starts_with(&format!("{RECOVERING_WORKER}:")))
        .count();
    assert_eq!(w3_calls, 1, "w3 ran exactly once without a retry policy");
}

/// D-31: the recovering-worker fixture leaves `retry_on` at its default, so
/// the scenario above is proven under `TransientOnly` -- the same predicate
/// `RetryPolicy::default()` carries -- and never by widening it.
#[test]
fn the_default_predicate_is_used() {
    let policy = e2e_fixtures::per_task_retry_policy();
    assert_eq!(policy.max_attempts, 3);
    assert_eq!(policy.retry_on, RetryPredicate::TransientOnly);
    assert_eq!(policy.retry_on, RetryPolicy::default().retry_on);

    let graph = e2e_fixtures::build_muster_defer_order_graph_with_a_template_per_task(Some(
        e2e_fixtures::per_task_retry_policy(),
    ));
    for name in WORKER_NAMES {
        let aegis = graph
            .aegis_for(&NodeId::new(name))
            .expect("every worker template carries the per-task Aegis");
        let retry = aegis
            .retry
            .as_ref()
            .expect("the Aegis carries a retry policy");
        assert_eq!(retry.max_attempts, 3);
        assert_eq!(retry.retry_on, RetryPredicate::TransientOnly);
    }
}

#[tokio::test]
async fn run_completes_with_a_single_superstep_complete_waypoint_per_superstep() {
    // ENG-FR-11 (D-14's clarification): exactly one superstep-COMPLETE
    // Waypoint per superstep is unchanged; a Muster may additionally write
    // zero-or-more `Running`-status progress Waypoints inside its own
    // superstep, counted SEPARATELY from that one-per-superstep guarantee.
    let graph = e2e_fixtures::build_muster_defer_order_graph();
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
