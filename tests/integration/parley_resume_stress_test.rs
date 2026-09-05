//! X-05 concurrency stress (`.project/v0.10.0/00-program-overview.md` section 3, acceptance
//! criterion 7): ten suspended threads, over one shared on-disk `SqliteWaypointStore` file,
//! resumed concurrently on a `multi_thread` Tokio runtime must all reach their expected terminal
//! outcome with EXACT counts and no cross-thread response leakage (T-24-18), under an explicit
//! timeout guard (T-24-19) so a deadlocked resume fails the test fast rather than hanging CI.
//!
//! There is no existing stress-test file in this tree to copy structurally (`24-PATTERNS.md`
//! records this as a genuine no-analog) -- built directly from the X-05 pattern:
//! `#[tokio::test(flavor = "multi_thread")]`, `tokio::spawn` per concurrent `resume_with` call
//! (a real cross-worker-thread race against the shared backend, not merely interleaved polling
//! within one task), joined through `futures::future::join_all`.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;

use paladin_battalion::engine::{
    EngineError, EngineLimits, NodeContext, NodeError, NodeSpec, RunOutcome, StateNode, WarEngine,
    WarGraph,
};
use paladin_core::platform::container::battlefield::{
    Battlefield, BattlefieldSchema, DispatchRule, FieldName, FieldSpec, StateDelta,
};
use paladin_core::platform::container::directive::{Directive, NextStep};
use paladin_core::platform::container::parley::{
    OnExpire, ParleyId, ParleyKind, ParleyRequest, ParleyResponse,
};
use paladin_core::platform::container::waypoint::{NodeId, ThreadId};
use paladin_ports::output::waypoint_port::WaypointPort;
use paladin_storage::waypoint::sqlite::SqliteWaypointStore;

#[allow(dead_code, unused_imports)]
#[path = "../helpers/mod.rs"]
mod helpers;
use helpers::FaultyPaladinPort;

const THREAD_COUNT: usize = 10;

fn field(name: &str) -> FieldName {
    FieldName::new(name).expect("valid field name")
}

/// A `Function` node that raises a `FreeText` parley on its first visit and
/// writes the delivered value to `output_field` on the post-resume visit --
/// the same single-parley shape used across every suspended thread in this
/// stress scenario.
struct ParleyingFunctionNode {
    output_field: FieldName,
}

impl ParleyingFunctionNode {
    fn new(output_field: FieldName) -> Arc<Self> {
        Arc::new(Self { output_field })
    }
}

#[async_trait::async_trait]
impl StateNode for ParleyingFunctionNode {
    async fn run(&self, _state: &Battlefield, ctx: &NodeContext) -> Result<Directive, NodeError> {
        match ctx.parley_response() {
            None => {
                let request = ParleyRequest {
                    parley_id: ParleyId::new(),
                    node_id: ctx.node_id.clone(),
                    kind: ParleyKind::FreeText,
                    prompt: "what is the answer?".to_string(),
                    payload: serde_json::json!({}),
                    choices: None,
                    expires_at: None,
                    created_at: Utc::now(),
                    on_expire: OnExpire::FailRun,
                };
                Ok(Directive {
                    delta: StateDelta::new(),
                    next: NextStep::Parley(request),
                })
            }
            Some(response) => {
                let mut delta = StateDelta::new();
                delta.set_raw(self.output_field.clone(), response.value.clone());
                Ok(delta.into())
            }
        }
    }
}

/// Build a graph with a single, terminal, always-parleying entry node.
fn build_graph() -> WarGraph {
    let schema = BattlefieldSchema::new(vec![FieldSpec::new(
        field("answer"),
        DispatchRule::LastWrite,
        None,
        false,
    )]);
    let mut graph = WarGraph::new(schema, EngineLimits::default());
    graph.add_node(
        NodeId::new("waiter"),
        NodeSpec::Function(ParleyingFunctionNode::new(field("answer"))),
    );
    graph.add_entry(NodeId::new("waiter"));
    graph
}

fn temp_db_url(label: &str) -> String {
    let path = std::env::temp_dir().join(format!(
        "parley_resume_stress_{label}_{}.sqlite",
        uuid::Uuid::new_v4()
    ));
    format!("sqlite://{}", path.display())
}

fn make_engine(store: Arc<SqliteWaypointStore>) -> WarEngine<SqliteWaypointStore> {
    WarEngine::new(Arc::new(FaultyPaladinPort::new()), store)
}

fn thread_id_for(i: usize) -> ThreadId {
    ThreadId::new(format!("stress-{i}")).expect("valid thread id")
}

/// Drive `count` independent threads to suspension, sequentially, over the
/// same engine/store -- returns each thread's id and its single raised
/// parley id, in index order.
async fn suspend_n(
    engine: &WarEngine<SqliteWaypointStore>,
    graph: &WarGraph,
    count: usize,
) -> Vec<(ThreadId, ParleyId)> {
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let thread = thread_id_for(i);
        let outcome = engine
            .start(graph, thread.clone(), StateDelta::new())
            .await
            .expect("start should suspend");
        let parley_id = match outcome {
            RunOutcome::AwaitingInput { parleys, .. } => {
                assert_eq!(parleys.len(), 1, "each thread raises exactly one parley");
                parleys[0].parley_id
            }
            other => panic!("expected AwaitingInput, got {other:?}"),
        };
        out.push((thread, parley_id));
    }
    out
}

fn answer_for(i: usize) -> serde_json::Value {
    serde_json::json!(format!("answer-{i}"))
}

fn response_for(parley_id: ParleyId, value: serde_json::Value) -> ParleyResponse {
    ParleyResponse {
        parley_id,
        kind: ParleyKind::FreeText,
        prompt: String::new(),
        value,
        responded_by: Some("tester".to_string()),
        responded_at: Utc::now(),
        defaulted: false,
    }
}

/// The core scenario: `THREAD_COUNT` threads suspended, then resumed
/// CONCURRENTLY via `tokio::spawn` (real cross-worker-thread races against
/// the one shared `SqliteWaypointStore` file), joined through
/// `futures::future::join_all`. Returns each thread's index alongside its
/// `resume_with` result, in join (not necessarily index) order.
async fn run_ten_concurrent_resumes() -> Vec<(usize, Result<RunOutcome, EngineError>)> {
    let db_url = temp_db_url("ten-concurrent");
    let graph = Arc::new(build_graph());
    let store = Arc::new(
        SqliteWaypointStore::new(&db_url)
            .await
            .expect("store should connect"),
    );
    let engine = Arc::new(make_engine(Arc::clone(&store)));

    let suspended = suspend_n(&engine, &graph, THREAD_COUNT).await;

    let mut handles = Vec::with_capacity(THREAD_COUNT);
    for (i, (thread, parley_id)) in suspended.into_iter().enumerate() {
        let engine = Arc::clone(&engine);
        let graph = Arc::clone(&graph);
        handles.push(tokio::spawn(async move {
            let response = response_for(parley_id, answer_for(i));
            let outcome = engine.resume_with(&graph, thread, vec![response]).await;
            (i, outcome)
        }));
    }

    futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|joined| joined.expect("spawned resume task must not panic"))
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn ten_suspended_threads_resume_concurrently() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let results = run_ten_concurrent_resumes().await;
        assert_eq!(results.len(), THREAD_COUNT);

        let mut completed = 0usize;
        for (i, outcome) in results {
            match outcome.unwrap_or_else(|e| panic!("thread {i} resume_with failed: {e}")) {
                RunOutcome::Completed { final_state, .. } => {
                    completed += 1;
                    assert_eq!(
                        final_state
                            .get::<String>(&field("answer"))
                            .expect("answer field should read"),
                        Some(format!("answer-{i}")),
                        "thread {i} must carry exactly its own submitted value, no more, no less"
                    );
                }
                other => panic!("thread {i} expected Completed, got {other:?}"),
            }
        }
        assert_eq!(
            completed, THREAD_COUNT,
            "exactly {THREAD_COUNT} completions, zero failures, zero still-suspended"
        );
    })
    .await
    .expect("ten-thread stress scenario must complete within its 30s timeout guard");
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_resumes_do_not_leak_responses_across_threads() {
    tokio::time::timeout(Duration::from_secs(30), async {
        const N: usize = 3;
        let db_url = temp_db_url("cross-thread-isolation");
        let graph = Arc::new(build_graph());
        let store = Arc::new(
            SqliteWaypointStore::new(&db_url)
                .await
                .expect("store should connect"),
        );
        let engine = Arc::new(make_engine(Arc::clone(&store)));

        let suspended = suspend_n(&engine, &graph, N).await;
        let parley_id_of = |i: usize| suspended[i].1;
        let thread_of = |i: usize| suspended[i].0.clone();

        // Thread 0 and thread 2 resume correctly; thread 1's submission
        // deliberately names THREAD 0's parley id -- a cross-thread
        // spoofing attempt (T-24-18) that must fail closed, not silently
        // succeed against the wrong thread's suspension.
        let mut handles = Vec::with_capacity(N);
        {
            let engine = Arc::clone(&engine);
            let graph = Arc::clone(&graph);
            let thread = thread_of(0);
            let response = response_for(parley_id_of(0), answer_for(0));
            handles.push(tokio::spawn(async move {
                (
                    0usize,
                    engine.resume_with(&graph, thread, vec![response]).await,
                )
            }));
        }
        {
            let engine = Arc::clone(&engine);
            let graph = Arc::clone(&graph);
            let thread = thread_of(1);
            // Wrong parley id: thread 0's, submitted against thread 1.
            let response = response_for(parley_id_of(0), answer_for(1));
            handles.push(tokio::spawn(async move {
                (
                    1usize,
                    engine.resume_with(&graph, thread, vec![response]).await,
                )
            }));
        }
        {
            let engine = Arc::clone(&engine);
            let graph = Arc::clone(&graph);
            let thread = thread_of(2);
            let response = response_for(parley_id_of(2), answer_for(2));
            handles.push(tokio::spawn(async move {
                (
                    2usize,
                    engine.resume_with(&graph, thread, vec![response]).await,
                )
            }));
        }

        let mut results: Vec<(usize, Result<RunOutcome, EngineError>)> =
            futures::future::join_all(handles)
                .await
                .into_iter()
                .map(|joined| joined.expect("spawned resume task must not panic"))
                .collect();
        results.sort_by_key(|(i, _)| *i);

        match &results[0].1 {
            Ok(RunOutcome::Completed { final_state, .. }) => {
                assert_eq!(
                    final_state
                        .get::<String>(&field("answer"))
                        .expect("answer field should read"),
                    Some("answer-0".to_string())
                );
            }
            other => panic!("thread 0 expected Ok(Completed), got {other:?}"),
        }

        match &results[1].1 {
            Err(EngineError::UnknownParleyId { parley_id }) => {
                assert_eq!(
                    *parley_id,
                    parley_id_of(0),
                    "the rejected id must be the SPOOFED one (thread 0's), not thread 1's own"
                );
            }
            other => panic!(
                "thread 1's cross-thread-id submission must fail UnknownParleyId, got {other:?}"
            ),
        }

        match &results[2].1 {
            Ok(RunOutcome::Completed { final_state, .. }) => {
                assert_eq!(
                    final_state
                        .get::<String>(&field("answer"))
                        .expect("answer field should read"),
                    Some("answer-2".to_string())
                );
            }
            other => panic!("thread 2 expected Ok(Completed), got {other:?}"),
        }

        // Thread 1 must remain untouched by the rejected cross-thread
        // submission -- still suspended, on its OWN unanswered parley id.
        let latest_thread_1 = store
            .latest(&thread_of(1))
            .await
            .expect("latest should succeed")
            .expect("thread 1 must still have a Waypoint");
        assert!(
            matches!(
                latest_thread_1.status,
                paladin_core::platform::container::waypoint::WaypointStatus::AwaitingInput { .. }
            ),
            "thread 1 must remain suspended -- no cross-thread response may have been accepted"
        );

        // Resuming thread 1 correctly afterward must still work, and must
        // carry ONLY its own value -- never thread 0's or thread 2's.
        let response = response_for(parley_id_of(1), answer_for(1));
        let outcome = engine
            .resume_with(&graph, thread_of(1), vec![response])
            .await
            .expect("thread 1's own correct resume_with must succeed");
        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state
                        .get::<String>(&field("answer"))
                        .expect("answer field should read"),
                    Some("answer-1".to_string()),
                    "thread 1's final state must carry exactly its own value"
                );
            }
            other => panic!("thread 1 expected Completed, got {other:?}"),
        }
    })
    .await
    .expect("cross-thread-isolation scenario must complete within its 30s timeout guard");
}

#[tokio::test(flavor = "multi_thread")]
async fn stress_run_completes_within_the_timeout_guard() {
    // Wraps the SAME concurrent scenario `ten_suspended_threads_resume_concurrently`
    // exercises in an explicit `tokio::time::timeout`, proving a deadlocked resume
    // would fail this test fast rather than hanging CI (T-24-19).
    let results = tokio::time::timeout(Duration::from_secs(10), run_ten_concurrent_resumes())
        .await
        .expect("the whole ten-thread scenario must finish well inside the timeout guard");

    let completed = results
        .into_iter()
        .filter(|(_, outcome)| matches!(outcome, Ok(RunOutcome::Completed { .. })))
        .count();
    assert_eq!(completed, THREAD_COUNT);
}
