//! Phase 25's remaining acceptance-criterion tests for Aegis retry and
//! timeouts (Doc 04 FT-FR-06, FT-FR-07, FT-FR-10; D-15, D-17, D-20, D-31;
//! plan 25-12), proven end to end through the engine's public surface with
//! the shared `FaultyPaladinPort` mock rather than any in-crate double:
//!
//! 1. **The X-05 multi-thread stress** (PRD 04 §3.8: Muster combined with
//!    per-task retry). A wide Muster runs on a `multi_thread` Tokio runtime;
//!    every task fails a SCRIPTED number of times and is retried in place by
//!    its own template's `RetryPolicy`. Per-task execution counts, per-task
//!    `attempt`/`AttemptRecord` histories, the total port-call count and
//!    the `task_key`-ordered aggregation are all asserted EXACTLY -- never as
//!    a range -- so any cross-task leakage of retry state (a shared counter,
//!    a shared backoff, a re-run sibling) shows up as a count mismatch
//!    rather than as flakiness (T-25-56). Every concurrent section is
//!    wrapped in `tokio::time::timeout`, following
//!    `src/application/services/orchestration/listener.rs`'s house pattern,
//!    so a deadlock fails the test loudly instead of hanging CI (T-25-60).
//!
//! 2. **Kill during backoff** (D-15, D-17, FT-FR-07). A run whose one node
//!    is sleeping inside a 60 s retry backoff is cancelled through the
//!    engine's `CancellationToken` -- the SIGTERM-equivalent path -- and
//!    returns at once rather than after the remaining backoff
//!    (`retry::wait_backoff`'s `select!`, RESEARCH.md Pitfall 7; T-25-57).
//!    The node is recorded `Skipped { reason: "shutdown" }` and re-listed on
//!    the Halted Waypoint's vanguard, and resuming from that Waypoint
//!    re-executes it from `attempt: 1` with no Waypoint ever written between
//!    its attempts (T-25-58). These two run under `tokio::time::pause` -- an
//!    in-memory Waypoint backend, no I/O -- so "returned within a bound far
//!    shorter than the backoff" is a virtual-clock measurement the runtime
//!    cannot skew.
//!
//! 3. **Nested run-timeout bounds, both directions** (D-20, FT-FR-10). With
//!    `EngineLimits.run_timeout` tighter than the node's own per-attempt
//!    `run_timeout` and `idle_timeout`, the cut attempt records
//!    `Timeout(EngineRun)` and the run ends `EngineError::RunTimeoutExceeded`
//!    -- never retried, even under a retry policy. The mirror case (node
//!    bound tighter than the engine budget) names `Timeout(Run)`, IS retried
//!    to exhaustion and ends `NodeFailed`, never `RunTimeoutExceeded`. Both
//!    drive a real on-disk `SqliteWaypointStore` so the Failed Waypoint is
//!    read back through the production persistence path, and both assert the
//!    `TimeoutKind` by value.
//!
//! Nothing here needs a live service or a container runtime. These tests live
//! under `tests/integration/`, so the repository's default `make test`
//! (`--lib --bins`) does not run them; run `cargo test --test aegis_retry_stress`.

use std::sync::Arc;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use paladin_battalion::engine::{
    EdgeSpec, EngineError, EngineLimits, InputMapping, NodeContext, NodeSpec, RunOutcome,
    StateNode, StateNodeError, WarEngine, WarGraph,
};
use paladin_core::base::entity::node::Node;
use paladin_core::platform::container::aegis::{Aegis, RetryPolicy, TimeoutPolicy};
use paladin_core::platform::container::battlefield::{
    Battlefield, BattlefieldSchema, DispatchRule, FieldName, FieldSpec, StateDelta,
};
use paladin_core::platform::container::directive::{Directive, MusterTask, NextStep};
use paladin_core::platform::container::node_error::{NodeErrorSource, TimeoutKind};
use paladin_core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus};
use paladin_core::platform::container::transience::Transience;
use paladin_core::platform::container::waypoint::{
    NodeExecutionRecord, NodeId, NodeOutcomeKind, ThreadId, Waypoint, WaypointStatus,
};
use paladin_ports::output::waypoint_port::WaypointPort;
use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;
use paladin_storage::waypoint::sqlite::SqliteWaypointStore;

// `tests/helpers/` is shared across many integration test binaries; this
// standalone [[test]] target only needs `FaultyPaladinPort`, so the rest of
// the module tree is unused here -- allowed rather than pruned, matching
// `e2e_crash_resume_test.rs`'s own precedent for this exact situation.
#[allow(dead_code, unused_imports)]
#[path = "../helpers/mod.rs"]
mod helpers;
use helpers::FaultyPaladinPort;

/// The overall guard every concurrent or wall-clock section runs under
/// (the `listener.rs` convention): generous, finite, asserted not to have
/// elapsed.
const GUARD: Duration = Duration::from_secs(30);

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

fn temp_db_url(label: &str) -> String {
    let path = std::env::temp_dir().join(format!(
        "aegis_retry_stress_{label}_{}.sqlite",
        uuid::Uuid::new_v4()
    ));
    format!("sqlite://{}", path.display())
}

/// Every Waypoint of `thread`, oldest first, through the port's own
/// `history` + `get` surface (never a backend-specific shortcut).
async fn full_history<S: WaypointPort>(store: &S, thread: &ThreadId) -> Vec<Waypoint> {
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

/// A fast, jitter-free retry policy for the wall-clock (multi-thread and
/// SQLite-backed) tests: three attempts, 1 ms then 2 ms between them. The
/// predicate is left at its `TransientOnly` default -- the mock's failure is
/// Transient by value, never by a widened predicate (D-31).
fn fast_retry(max_attempts: u32) -> RetryPolicy {
    RetryPolicy {
        max_attempts,
        initial_interval: Duration::from_millis(1),
        jitter: false,
        ..RetryPolicy::default()
    }
}

// ============================================================================
// 1. The X-05 multi-thread stress: Muster + per-task retry, exact counts
// ============================================================================

/// One task per worker template: `w01`..`w{N}`, each mustered against its
/// own key (the template id doubles as the `task_key`, so `task_key` order
/// is the template order).
fn worker_names(count: usize) -> Vec<String> {
    (1..=count).map(|i| format!("w{i:02}")).collect()
}

/// The stress planner: musters one task against each named template.
struct WidePlannerNode {
    workers: Vec<String>,
}

#[async_trait::async_trait]
impl StateNode for WidePlannerNode {
    async fn run(
        &self,
        _state: &Battlefield,
        _ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        let tasks = self
            .workers
            .iter()
            .map(|name| MusterTask {
                worker: NodeId::new(name),
                payload: serde_json::json!(name),
                task_key: name.clone(),
            })
            .collect();
        Ok(Directive {
            delta: StateDelta::new(),
            next: NextStep::Muster(tasks),
        })
    }
}

/// Deferred aggregator: copies the `Append`-dispatched `worker_out` list
/// (already in `task_key` order, CF-FR-11) into `aggregated`.
struct AggregatorNode;

#[async_trait::async_trait]
impl StateNode for AggregatorNode {
    async fn run(
        &self,
        state: &Battlefield,
        _ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        let results = state
            .get::<Vec<String>>(&field("worker_out"))
            .map_err(|e| StateNodeError(e.to_string()))?
            .unwrap_or_default();
        let mut delta = StateDelta::new();
        delta.set_raw(field("aggregated"), serde_json::json!(results));
        Ok(delta.into())
    }
}

/// `planner` (entry, musters one task per template) `-> w01..wN` (Paladin
/// worker templates, each carrying `retry`) `-> aggregator` (deferred).
fn wide_muster_graph(workers: &[String], retry: RetryPolicy) -> WarGraph {
    let schema = BattlefieldSchema::new(vec![
        FieldSpec::new(field("worker_out"), DispatchRule::Append, None, false),
        FieldSpec::new(field("aggregated"), DispatchRule::LastWrite, None, false),
    ]);
    let mut graph = WarGraph::new(schema, EngineLimits::default());
    let planner = NodeId::new("planner");
    let aggregator = NodeId::new("aggregator");

    graph.add_node(
        planner.clone(),
        NodeSpec::Function(Arc::new(WidePlannerNode {
            workers: workers.to_vec(),
        })),
    );
    graph.add_deferred_node(
        aggregator.clone(),
        NodeSpec::Function(Arc::new(AggregatorNode)),
    );
    for name in workers {
        let worker = NodeId::new(name);
        graph.add_worker_template(
            worker.clone(),
            NodeSpec::paladin(
                make_paladin(name),
                InputMapping::new("{muster.payload}"),
                field("worker_out"),
            ),
        );
        graph.set_aegis(
            worker.clone(),
            Aegis {
                retry: Some(retry.clone()),
                ..Aegis::default()
            },
        );
        graph.add_edge(EdgeSpec {
            from: worker,
            to: aggregator.clone(),
            condition: None,
        });
    }
    graph.add_entry(planner);
    graph
}

/// The muster superstep's one completion Waypoint (the one recording the
/// worker templates), and each named worker's single record on it.
fn muster_records<'a>(history: &'a [Waypoint], workers: &[String]) -> Vec<&'a NodeExecutionRecord> {
    let first = NodeId::new(&workers[0]);
    let muster_waypoint = history
        .iter()
        .filter(|w| w.muster_progress.is_none())
        .find(|w| w.completed.iter().any(|r| r.node_id == first))
        .expect("the muster superstep's completion Waypoint records the workers");
    workers
        .iter()
        .map(|name| {
            let id = NodeId::new(name);
            let records: Vec<_> = muster_waypoint
                .completed
                .iter()
                .filter(|r| r.node_id == id)
                .collect();
            assert_eq!(records.len(), 1, "{name} has exactly one record");
            records[0]
        })
        .collect()
}

/// Drive one stress round: `workers.len()` tasks in one Muster, task `i`
/// scripted to fail `plan[i]` times (each strictly below `max_attempts`),
/// and assert every count exactly.
async fn run_stress_round(label: &str, workers: &[String], plan: &[usize], max_attempts: u32) {
    assert_eq!(workers.len(), plan.len());
    assert!(
        plan.iter().all(|&fails| (fails as u32) < max_attempts),
        "every scripted failure count must leave room to succeed"
    );

    let mut port = FaultyPaladinPort::new();
    for (name, &fails) in workers.iter().zip(plan) {
        if fails > 0 {
            port = port.fail_paladin_until_attempt(name.clone(), fails);
        }
    }
    let port = Arc::new(port);
    let store = Arc::new(InMemoryWaypointStore::new());
    let graph = wide_muster_graph(workers, fast_retry(max_attempts));
    let thread = ThreadId::new(format!("x05-{label}")).expect("valid thread id");
    let engine = WarEngine::new(port.clone(), store.clone());

    let outcome = engine
        .start(&graph, thread.clone(), StateDelta::new())
        .await
        .expect("the run must succeed: every scripted failure is retried in place");
    let final_state = match outcome {
        RunOutcome::Completed { final_state, .. } => final_state,
        other => panic!("[{label}] expected Completed, got {other:?}"),
    };

    // --- Exact per-task and total port-call counts.
    let log = port.execution_log();
    let mut expected_total = 0;
    for (name, &fails) in workers.iter().zip(plan) {
        let calls = log
            .iter()
            .filter(|entry| entry.starts_with(&format!("{name}:")))
            .count();
        assert_eq!(
            calls,
            fails + 1,
            "[{label}] {name} scripted to fail {fails}x must be called exactly {} times",
            fails + 1
        );
        expected_total += fails + 1;
    }
    assert_eq!(
        port.call_count(),
        expected_total,
        "[{label}] the total is exactly the sum of the per-task counts"
    );

    // --- Exact per-task attempt histories on the persisted records.
    let history = full_history(store.as_ref(), &thread).await;
    let records = muster_records(&history, workers);
    for ((name, &fails), record) in workers.iter().zip(plan).zip(records) {
        assert_eq!(
            record.outcome,
            NodeOutcomeKind::Succeeded,
            "[{label}] {name}"
        );
        assert_eq!(
            record.attempt,
            fails as u32 + 1,
            "[{label}] {name} succeeded on attempt {}",
            fails + 1
        );
        assert_eq!(
            record.attempts.len(),
            fails,
            "[{label}] {name} carries exactly {fails} failed AttemptRecord(s)"
        );
        let numbers: Vec<u32> = record.attempts.iter().map(|a| a.attempt).collect();
        assert_eq!(
            numbers,
            (1..=fails as u32).collect::<Vec<_>>(),
            "[{label}] {name}'s failed attempts are numbered 1..={fails}"
        );
        for attempt in &record.attempts {
            assert_eq!(attempt.error.node_id, NodeId::new(name));
            assert_eq!(attempt.error.attempt, attempt.attempt);
            assert_eq!(attempt.error.transience, Transience::Transient);
        }
    }

    // --- Aggregation: every task's result, in task_key order.
    let expected: Vec<String> = workers
        .iter()
        .map(|name| format!("FaultyPaladinPort: {name} processed {name}"))
        .collect();
    let aggregated = final_state
        .get::<Vec<String>>(&field("aggregated"))
        .expect("aggregated field should deserialize as Vec<String>");
    assert_eq!(
        aggregated,
        Some(expected),
        "[{label}] the aggregation holds every task's result in task_key order"
    );

    // --- One progress Waypoint per COMPLETED task, none per failed attempt.
    let progress_count = history
        .iter()
        .filter(|w| w.muster_progress.is_some())
        .count();
    assert_eq!(
        progress_count,
        workers.len(),
        "[{label}] exactly one progress Waypoint per completed task"
    );
    assert!(
        history
            .iter()
            .all(|w| !matches!(w.status, WaypointStatus::Failed { .. })),
        "[{label}] no Failed Waypoint is ever persisted for a recovered task"
    );
}

/// X-05 (PRD 04 §3.8), T-25-56: 24 mustered tasks on a `multi_thread`
/// runtime, task `i` scripted to fail `(i + round) % 3` times under a
/// 3-attempt policy, for three rounds with the failure plan rotated -- every
/// per-task call count, attempt number, `AttemptRecord` history, the total
/// and the aggregation are exact.
#[tokio::test(flavor = "multi_thread")]
async fn muster_with_per_task_retry_under_concurrency_has_exact_counts() {
    const TASKS: usize = 24;
    const ROUNDS: usize = 3;
    let workers = worker_names(TASKS);

    tokio::time::timeout(GUARD, async {
        for round in 0..ROUNDS {
            let plan: Vec<usize> = (0..TASKS).map(|i| (i + round) % 3).collect();
            run_stress_round(&format!("round-{round}"), &workers, &plan, 3).await;
        }
    })
    .await
    .expect("the X-05 stress must complete within its timeout guard");
}

/// T-25-56: under the same concurrency, exactly two of sixteen tasks fail
/// twice and every other task never fails -- the never-failing tasks read
/// `attempt: 1` with an empty history and the two failing tasks each carry
/// their OWN attempts 1 and 2, so no task's counter is influenced by
/// another's; the total is exactly 16 + 4.
#[tokio::test(flavor = "multi_thread")]
async fn concurrent_tasks_do_not_share_retry_state() {
    const TASKS: usize = 16;
    let workers = worker_names(TASKS);
    let failing: [usize; 2] = [4, 10]; // w05 and w11, zero-based indices

    tokio::time::timeout(GUARD, async {
        let plan: Vec<usize> = (0..TASKS)
            .map(|i| if failing.contains(&i) { 2 } else { 0 })
            .collect();
        run_stress_round("no-shared-state", &workers, &plan, 3).await;
    })
    .await
    .expect("the no-shared-state stress must complete within its timeout guard");
}

// ============================================================================
// 2. Kill during backoff, and resume from attempt 1
// ============================================================================

const FLAKY: &str = "flaky";

/// A backoff far longer than any bound this test measures against: the
/// abort must return in virtual time far shorter than this.
const LONG_BACKOFF: Duration = Duration::from_secs(60);

/// A single-Paladin graph: `flaky` (entry) reads a literal input and
/// writes `out`, under a 3-attempt policy whose first backoff is 60 s.
fn flaky_graph() -> WarGraph {
    let schema = BattlefieldSchema::new(vec![FieldSpec::new(
        field("out"),
        DispatchRule::LastWrite,
        None,
        false,
    )]);
    let mut graph = WarGraph::new(schema, EngineLimits::default());
    let flaky = NodeId::new(FLAKY);
    graph.add_node(
        flaky.clone(),
        NodeSpec::paladin(
            make_paladin(FLAKY),
            InputMapping::new("quest"),
            field("out"),
        ),
    );
    graph.set_aegis(
        flaky.clone(),
        Aegis {
            retry: Some(RetryPolicy {
                max_attempts: 3,
                initial_interval: LONG_BACKOFF,
                jitter: false,
                ..RetryPolicy::default()
            }),
            ..Aegis::default()
        },
    );
    graph.add_entry(flaky);
    graph
}

/// Start a run whose `flaky` node fails its first attempt and enters a 60 s
/// backoff, cancel the run's token the moment that first attempt has been
/// made (the SIGTERM-equivalent), and return the halted outcome, the
/// virtual time the abort took, and the shared port/store/thread for the
/// caller's own assertions or resume. Runs under the caller's paused clock.
async fn kill_during_backoff(
    label: &str,
) -> (
    RunOutcome,
    Duration,
    Arc<FaultyPaladinPort>,
    Arc<InMemoryWaypointStore>,
    ThreadId,
) {
    let port = Arc::new(FaultyPaladinPort::new().fail_paladin_until_attempt(FLAKY, 1));
    let store = Arc::new(InMemoryWaypointStore::new());
    let thread = ThreadId::new(format!("kill-during-backoff-{label}")).expect("valid thread id");
    let token = CancellationToken::new();
    let engine = WarEngine::new(port.clone(), store.clone()).with_cancellation_token(token.clone());

    let run = {
        let thread = thread.clone();
        tokio::spawn(async move {
            let graph = flaky_graph();
            engine.start(&graph, thread, StateDelta::new()).await
        })
    };

    // Spin (yielding, never sleeping -- a sleep would let the paused clock
    // auto-advance straight through the backoff) until the first attempt has
    // been made and the node is on its way into the 60 s backoff.
    let mut yields = 0usize;
    while port.call_count() < 1 {
        yields += 1;
        assert!(
            yields < 100_000,
            "the first attempt must be made within a bounded number of yields"
        );
        tokio::task::yield_now().await;
    }
    assert_eq!(port.call_count(), 1, "exactly one attempt before the kill");

    let killed_at = tokio::time::Instant::now();
    token.cancel();
    let outcome = tokio::time::timeout(Duration::from_secs(5), run)
        .await
        .expect("the killed run must return well inside 5 s (virtual) -- not after the backoff")
        .expect("the run task must not panic")
        .expect("a cancelled run returns Ok(Halted), never Err");
    let abort_took = killed_at.elapsed();

    (outcome, abort_took, port, store, thread)
}

/// D-15, T-25-57 (RESEARCH.md Pitfall 7): a run killed while its node sleeps
/// in a 60 s backoff halts at once -- in virtual time far shorter than the
/// remaining backoff -- makes no further attempt, and records the node
/// `Skipped { reason: "shutdown" }` re-listed on the Halted vanguard.
#[tokio::test(start_paused = true)]
async fn a_run_killed_during_backoff_aborts_immediately() {
    let (outcome, abort_took, port, store, thread) = kill_during_backoff("abort").await;

    assert!(
        matches!(outcome, RunOutcome::Halted { .. }),
        "a cancelled run halts: {outcome:?}"
    );
    assert!(
        abort_took < Duration::from_secs(1),
        "the abort returned in {abort_took:?}, not after the {LONG_BACKOFF:?} backoff"
    );
    assert_eq!(
        port.call_count(),
        1,
        "the interrupted backoff never retried: attempt 1 was the only call"
    );

    let history = full_history(store.as_ref(), &thread).await;
    assert_eq!(
        history.len(),
        1,
        "exactly one Waypoint -- the Halted one -- and none between the attempts"
    );
    let halted = &history[0];
    assert_eq!(halted.status, WaypointStatus::Halted);
    let flaky = NodeId::new(FLAKY);
    assert!(
        halted.vanguard.contains(&flaky),
        "the interrupted node is re-listed on the Halted vanguard: {:?}",
        halted.vanguard
    );
    let record = halted
        .completed
        .iter()
        .find(|r| r.node_id == flaky)
        .expect("the interrupted node has a record on the Halted Waypoint");
    assert_eq!(
        record.outcome,
        NodeOutcomeKind::Skipped {
            reason: "shutdown".to_string()
        }
    );
    assert_eq!(record.attempt, 1, "the killed attempt was attempt 1");
    assert_eq!(
        record.attempts.len(),
        1,
        "the killed attempt's own transient failure is on the record"
    );
    assert_eq!(record.attempts[0].error.transience, Transience::Transient);
}

/// D-17, FT-FR-07, T-25-58: resuming the thread killed mid-backoff from its
/// persisted Waypoint re-executes the interrupted node from `attempt: 1`
/// with an empty attempt history -- never continuing at attempt 2 -- and the
/// whole thread holds exactly two Waypoints (Halted, then Completed): none
/// was ever written between the killed attempts.
#[tokio::test(start_paused = true)]
async fn resuming_after_a_kill_during_backoff_restarts_at_attempt_one() {
    let (outcome, _abort_took, port, store, thread) = kill_during_backoff("resume").await;
    assert!(matches!(outcome, RunOutcome::Halted { .. }));
    assert_eq!(port.call_count(), 1);

    // A fresh engine (no cancellation token) over the SAME store and the
    // SAME port: flaky's per-Paladin counter is past its threshold, so the
    // resumed run's first (and only) attempt succeeds.
    let resume_engine = WarEngine::new(port.clone(), store.clone());
    let graph = flaky_graph();
    let resumed = tokio::time::timeout(
        Duration::from_secs(5),
        resume_engine.resume(&graph, thread.clone()),
    )
    .await
    .expect("the resumed run must complete well inside 5 s (virtual)")
    .expect("resume should succeed");
    let final_state = match resumed {
        RunOutcome::Completed { final_state, .. } => final_state,
        other => panic!("expected the resumed run to complete, got {other:?}"),
    };
    assert_eq!(
        final_state
            .get::<String>(&field("out"))
            .expect("out deserializes"),
        Some(format!("FaultyPaladinPort: {FLAKY} processed quest"))
    );
    assert_eq!(
        port.call_count(),
        2,
        "one call in the killed run, one in the resumed run -- no third"
    );

    let history = full_history(store.as_ref(), &thread).await;
    assert_eq!(
        history.len(),
        2,
        "Halted then Completed: no Waypoint was written between the attempts"
    );
    assert_eq!(history[0].status, WaypointStatus::Halted);
    assert_eq!(history[1].status, WaypointStatus::Completed);
    assert!(
        history
            .iter()
            .all(|w| !matches!(w.status, WaypointStatus::Failed { .. })),
        "no Failed Waypoint anywhere on the thread"
    );

    let flaky = NodeId::new(FLAKY);
    let record = history[1]
        .completed
        .iter()
        .find(|r| r.node_id == flaky)
        .expect("the resumed run recorded the node");
    assert_eq!(record.outcome, NodeOutcomeKind::Succeeded);
    assert_eq!(
        record.attempt, 1,
        "a resume re-executes the interrupted node from attempt 1 (FT-FR-07)"
    );
    assert!(
        record.attempts.is_empty(),
        "the killed run's attempt is not carried into the resumed run's history"
    );
}

// ============================================================================
// 3. Nested run-timeout bounds, named by value in both directions
// ============================================================================

const SLOW: &str = "slow";

/// A single-Paladin graph whose `slow` node's port sleeps `SLOW_MS` before
/// answering, under `retry` (3 fast attempts) and `timeout`, with the
/// run-level budget `engine_run_timeout`.
fn slow_graph(engine_run_timeout: Duration, timeout: TimeoutPolicy) -> WarGraph {
    let schema = BattlefieldSchema::new(vec![FieldSpec::new(
        field("out"),
        DispatchRule::LastWrite,
        None,
        false,
    )]);
    let mut graph = WarGraph::new(
        schema,
        EngineLimits {
            run_timeout: Some(engine_run_timeout),
            ..EngineLimits::default()
        },
    );
    let slow = NodeId::new(SLOW);
    graph.add_node(
        slow.clone(),
        NodeSpec::paladin(make_paladin(SLOW), InputMapping::new("quest"), field("out")),
    );
    graph.set_aegis(
        slow.clone(),
        Aegis {
            retry: Some(fast_retry(3)),
            timeout: Some(timeout),
            ..Aegis::default()
        },
    );
    graph.add_entry(slow);
    graph
}

/// The port's sleep: far longer than every bound below, so whichever bound
/// is tightest is the one that fires, and the sleep itself never completes.
const SLOW_MS: u64 = 30_000;

/// The Failed Waypoint `id` names, read back from the real backend.
async fn failed_waypoint(
    store: &SqliteWaypointStore,
    thread: &ThreadId,
    id: &paladin_core::platform::container::waypoint::WaypointId,
) -> Waypoint {
    store
        .get(thread, id)
        .await
        .expect("get should succeed")
        .expect("the failure Waypoint was persisted")
}

/// D-20, FT-FR-10: with `EngineLimits.run_timeout = 200 ms` tighter than
/// the node's own `run_timeout = 10 s` and `idle_timeout = 10 s`, the cut
/// attempt records `Timeout(EngineRun)` by value, is NOT retried despite
/// the 3-attempt policy, and the run ends `EngineError::RunTimeoutExceeded`
/// -- proven against a real on-disk SQLite Waypoint backend.
#[tokio::test]
async fn an_engine_run_timeout_tighter_than_both_bounds_names_enginerun() {
    tokio::time::timeout(GUARD, async {
        let engine_budget = Duration::from_millis(200);
        let graph = slow_graph(
            engine_budget,
            TimeoutPolicy {
                run_timeout: Some(Duration::from_secs(10)),
                idle_timeout: Some(Duration::from_secs(10)),
            },
        );
        let store = Arc::new(
            SqliteWaypointStore::new(&temp_db_url("enginerun"))
                .await
                .expect("store should connect"),
        );
        let port = Arc::new(FaultyPaladinPort::new().with_delay_ms(SLOW_MS));
        let thread = ThreadId::new("timeout-enginerun").expect("valid thread id");
        let engine = WarEngine::new(port.clone(), store.clone());

        let outcome = engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .expect("the engine returns Ok(RunOutcome::Failed), never Err");
        let waypoint_id = match outcome {
            RunOutcome::Failed {
                error: EngineError::RunTimeoutExceeded { elapsed, limit },
                waypoint: Some(waypoint_id),
            } => {
                assert_eq!(limit, engine_budget);
                assert!(elapsed >= limit, "elapsed {elapsed:?} >= limit {limit:?}");
                waypoint_id
            }
            other => panic!("expected Failed(RunTimeoutExceeded), got {other:?}"),
        };
        assert_eq!(
            port.call_count(),
            1,
            "Timeout(EngineRun) is never retried: the budget is gone"
        );

        let failed = failed_waypoint(store.as_ref(), &thread, &waypoint_id).await;
        let slow = NodeId::new(SLOW);
        match &failed.status {
            WaypointStatus::Failed {
                failed_node,
                node_error: Some(node_error),
                ..
            } => {
                assert_eq!(failed_node, &slow);
                assert_eq!(node_error.node_id, slow);
                assert_eq!(node_error.attempt, 1);
                assert_eq!(node_error.transience, Transience::Transient);
                assert_eq!(
                    node_error.source,
                    NodeErrorSource::Timeout(TimeoutKind::EngineRun),
                    "the cut attempt names the ENGINE budget, by value"
                );
            }
            other => panic!("expected a Failed Waypoint with a NodeError, got {other:?}"),
        }
        let record = failed
            .completed
            .iter()
            .find(|r| r.node_id == slow)
            .expect("the cut node's record is on the Waypoint");
        assert_eq!(record.outcome, NodeOutcomeKind::Failed);
        assert_eq!(record.attempt, 1, "never retried");
        assert!(record.attempts.is_empty());
    })
    .await
    .expect("the EngineRun scenario must complete within its timeout guard");
}

/// D-20 mirror: with the node's own `run_timeout = 200 ms` tighter than a
/// 30 s engine budget, each cut attempt names `Timeout(Run)` by value, IS
/// retried like any other transient failure until the 3-attempt policy is
/// exhausted, and the run ends `EngineError::NodeFailed` -- never
/// `RunTimeoutExceeded`.
#[tokio::test]
async fn a_node_run_timeout_tighter_than_the_engine_budget_names_run() {
    tokio::time::timeout(GUARD, async {
        let graph = slow_graph(
            Duration::from_secs(30),
            TimeoutPolicy {
                run_timeout: Some(Duration::from_millis(200)),
                idle_timeout: Some(Duration::from_secs(10)),
            },
        );
        let store = Arc::new(
            SqliteWaypointStore::new(&temp_db_url("run"))
                .await
                .expect("store should connect"),
        );
        let port = Arc::new(FaultyPaladinPort::new().with_delay_ms(SLOW_MS));
        let thread = ThreadId::new("timeout-run").expect("valid thread id");
        let engine = WarEngine::new(port.clone(), store.clone());

        let outcome = engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .expect("the engine returns Ok(RunOutcome::Failed), never Err");
        let node_error = outcome
            .node_error()
            .cloned()
            .unwrap_or_else(|| panic!("expected Failed(NodeFailed), got {outcome:?}"));
        let waypoint_id = match &outcome {
            RunOutcome::Failed {
                error: EngineError::NodeFailed(_),
                waypoint: Some(waypoint_id),
            } => *waypoint_id,
            other => panic!("expected Failed(NodeFailed) with a Waypoint, got {other:?}"),
        };
        assert!(
            !matches!(
                outcome,
                RunOutcome::Failed {
                    error: EngineError::RunTimeoutExceeded { .. },
                    ..
                }
            ),
            "the node's own bound fired, not the engine budget"
        );
        let slow = NodeId::new(SLOW);
        assert_eq!(node_error.node_id, slow);
        assert_eq!(node_error.attempt, 3, "the exhausted attempt");
        assert_eq!(
            node_error.source,
            NodeErrorSource::Timeout(TimeoutKind::Run),
            "the cut attempt names the node's OWN run bound, by value"
        );
        assert_eq!(
            port.call_count(),
            3,
            "Timeout(Run) IS retried: three attempts under a 3-attempt policy"
        );

        let failed = failed_waypoint(store.as_ref(), &thread, &waypoint_id).await;
        match &failed.status {
            WaypointStatus::Failed {
                failed_node,
                node_error: Some(persisted),
                ..
            } => {
                assert_eq!(failed_node, &slow);
                assert_eq!(
                    persisted, &node_error,
                    "the Waypoint carries the same NodeError"
                );
            }
            other => panic!("expected a Failed Waypoint with a NodeError, got {other:?}"),
        }
        let record = failed
            .completed
            .iter()
            .find(|r| r.node_id == slow)
            .expect("the exhausted node's record is on the Waypoint");
        assert_eq!(record.outcome, NodeOutcomeKind::Failed);
        assert_eq!(record.attempt, 3);
        assert_eq!(
            record.attempts.len(),
            2,
            "attempts 1 and 2 were cut and retried"
        );
        for attempt in &record.attempts {
            assert_eq!(
                attempt.error.source,
                NodeErrorSource::Timeout(TimeoutKind::Run),
                "every earlier cut also named Run"
            );
        }
    })
    .await
    .expect("the Run scenario must complete within its timeout guard");
}
