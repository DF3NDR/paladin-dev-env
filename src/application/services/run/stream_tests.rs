//! Live-path (real engine, real bus) and degraded-path (polling, real
//! on-disk SQLite for the cross-instance proof) tests for
//! `GET /v1/runs/{run_id}/stream` (27-10 Task 1, D-24..D-27, PLAT-FR-07).
//!
//! `UnusedPaladinPort`/`submit`/`temp_sqlite_url`/`cleanup` mirror
//! `worker_tests.rs`/`cancel_tests.rs`'s own precedent of small,
//! per-test-module doubles rather than reaching across modules for
//! `#[cfg(test)]`-private helpers.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use tokio::sync::broadcast;

use paladin_battalion::engine::{
    EdgeSpec, EngineLimits, InputMapping, NodeContext, NodeSpec, StateNode, StateNodeError,
    WarEngine, WarGraph, graph::GateRequestTemplate,
};
use paladin_core::platform::container::battlefield::{
    Battlefield, BattlefieldSchema, DispatchRule, FieldName, FieldSpec, StateDelta,
};
use paladin_core::platform::container::directive::Directive;
use paladin_core::platform::container::execution_result::PaladinResult;
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::parley::ParleyKind;
use paladin_core::platform::container::run::{
    AssistantRef, Run, RunId, RunStatus, RunStreamEventKind, RunStreamMode,
};
use paladin_core::platform::container::waypoint::{NodeId, ThreadId};
use paladin_ports::input::run_event_stream_port::RunEventStreamPort;
use paladin_ports::output::paladin_port::{PaladinPort, PaladinStream};
use paladin_ports::output::run_queue_port::{QueuedRun, RunQueuePort};
use paladin_ports::output::run_repository_port::RunRepositoryPort;
use paladin_ports::output::waypoint_port::WaypointPort;
use paladin_storage::run::in_memory::InMemoryRunRepository;
use paladin_storage::run::sqlite::SqliteRunRepository;
use paladin_storage::run_queue::in_memory::InMemoryRunQueue;
use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;
use paladin_storage::waypoint::sqlite::SqliteWaypointStore;

use super::events::{RunEventBus, RunEventBusSink, RunEventStreamService};
use super::resolver::{AssistantResolver, CodeWorkflowResolver};
use super::worker::RunWorkerPool;

/// A [`PaladinPort`] that must never be called -- every graph in this module
/// is Function/Gate-only (mirrors `worker_tests.rs`'s own precedent).
struct UnusedPaladinPort;

#[async_trait]
impl PaladinPort for UnusedPaladinPort {
    async fn execute(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinResult, PaladinError> {
        unreachable!("this test's WarGraph has no NodeSpec::Paladin nodes")
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinStream, PaladinError> {
        unreachable!("this test's WarGraph has no NodeSpec::Paladin nodes")
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

/// A [`StateNode`] that sleeps `delay` then completes -- one node dispatched
/// per superstep (mirrors `cancel_tests.rs`'s own `SlowNode`).
struct SlowNode {
    delay: Duration,
}

#[async_trait]
impl StateNode for SlowNode {
    async fn run(
        &self,
        _state: &Battlefield,
        _ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        tokio::time::sleep(self.delay).await;
        Ok(StateDelta::new().into())
    }
}

/// Build a linear `count`-node chain (`n0 -> n1 -> ... -> n{count-1}`), each
/// node sleeping `delay` before completing.
fn build_chain_graph(count: usize, delay: Duration) -> Arc<WarGraph> {
    let mut graph = WarGraph::new(BattlefieldSchema::new(vec![]), EngineLimits::default());
    let ids: Vec<NodeId> = (0..count).map(|i| NodeId::new(format!("n{i}"))).collect();
    for id in &ids {
        graph.add_node(id.clone(), NodeSpec::Function(Arc::new(SlowNode { delay })));
    }
    for pair in ids.windows(2) {
        graph.add_edge(EdgeSpec {
            from: pair[0].clone(),
            to: pair[1].clone(),
            condition: None,
        });
    }
    graph.add_entry(ids[0].clone());
    Arc::new(graph)
}

/// Build a single-node graph whose only node is a `Gate` approval request
/// (mirrors `worker_tests.rs`'s own `build_gate_graph`).
fn build_gate_graph() -> Arc<WarGraph> {
    let field = FieldName::new("approved").unwrap();
    let schema = BattlefieldSchema::new(vec![FieldSpec::new(
        field.clone(),
        DispatchRule::LastWrite,
        Some(serde_json::json!(false)),
        false,
    )]);
    let mut graph = WarGraph::new(schema, EngineLimits::default());
    let gate_id = NodeId::new("gate");
    graph.add_node(
        gate_id.clone(),
        NodeSpec::gate(
            GateRequestTemplate::new(ParleyKind::Approval, InputMapping::new("Proceed?")),
            Some(field),
        ),
    );
    graph.add_entry(gate_id);
    Arc::new(graph)
}

/// A [`StateNode`] that writes one field to a sentinel value -- the
/// instrument for `state_delta_carries_field_names_only`.
struct FieldWritingNode {
    field: FieldName,
}

#[async_trait]
impl StateNode for FieldWritingNode {
    async fn run(
        &self,
        _state: &Battlefield,
        _ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        let mut delta = StateDelta::new();
        delta
            .set(self.field.clone(), SENTINEL_VALUE)
            .expect("&str always serializes");
        Ok(delta.into())
    }
}

/// The value `state_delta_carries_field_names_only` asserts never appears
/// anywhere in a `state_delta` payload.
const SENTINEL_VALUE: &str = "AAAA_SENSITIVE_VALUE_AAAA";

/// Build a single-node graph whose only node writes `SENTINEL_VALUE` into a
/// declared field.
fn build_field_writing_graph() -> Arc<WarGraph> {
    let field = FieldName::new("secret_field").unwrap();
    let schema = BattlefieldSchema::new(vec![FieldSpec::new(
        field.clone(),
        DispatchRule::LastWrite,
        Some(serde_json::json!(null)),
        false,
    )]);
    let mut graph = WarGraph::new(schema, EngineLimits::default());
    let node_id = NodeId::new("writer");
    graph.add_node(
        node_id.clone(),
        NodeSpec::Function(Arc::new(FieldWritingNode { field })),
    );
    graph.add_entry(node_id);
    Arc::new(graph)
}

/// Insert a fresh `Queued` run against `assistant_id` on a fresh thread and
/// enqueue its pointer, returning the run id and thread id.
async fn submit(
    repository: &Arc<dyn RunRepositoryPort>,
    queue: &Arc<dyn RunQueuePort>,
    assistant_id: &str,
) -> (RunId, ThreadId) {
    let run_id = RunId::new_v7();
    let thread_id = ThreadId::new(format!("thread-{run_id}")).unwrap();
    let run = Run::new(
        run_id.clone(),
        thread_id.clone(),
        AssistantRef {
            assistant_id: assistant_id.to_string(),
            version: 1,
        },
        serde_json::json!({}),
    );
    repository.insert(&run).await.unwrap();
    queue
        .enqueue(QueuedRun {
            run_id: run_id.clone(),
            thread_id: thread_id.clone(),
            attempt: 1,
            enqueued_at: chrono::Utc::now(),
        })
        .await
        .unwrap();
    (run_id, thread_id)
}

/// A fresh on-disk SQLite URL under the system temp dir.
fn temp_sqlite_url(label: &str) -> (std::path::PathBuf, String) {
    let path = std::env::temp_dir().join(format!(
        "paladin_stream_test_{label}_{}.sqlite",
        uuid::Uuid::new_v4()
    ));
    let url = format!("sqlite://{}", path.display());
    (path, url)
}

fn cleanup(path: &std::path::PathBuf) {
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(format!("{}-wal", path.display()));
    let _ = std::fs::remove_file(format!("{}-shm", path.display()));
}

#[tokio::test(flavor = "multi_thread")]
async fn live_stream_yields_progress_then_done() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
        let store = Arc::new(InMemoryWaypointStore::new());
        let graph = build_chain_graph(3, Duration::from_millis(20));
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("chain", graph));

        let bus = Arc::new(RunEventBus::new());
        let engine = Arc::new(
            WarEngine::new(Arc::new(UnusedPaladinPort), store.clone())
                .with_trace_sink(Arc::new(RunEventBusSink::new(bus.clone()))),
        );
        let pool = RunWorkerPool::new(
            engine,
            store,
            repository.clone(),
            queue.clone(),
            resolver,
            Duration::from_secs(30),
        )
        .with_event_bus(bus.clone());

        let (run_id, thread_id) = submit(&repository, &queue, "chain").await;

        // Pre-bind and subscribe BEFORE `run_once` starts dispatching:
        // `broadcast` never buffers a publish for a subscriber that
        // connects later, so subscribing only after spawning the run below
        // would race the run's own (idempotent) `bind` and drop early
        // events -- exactly the ordinary "connect after it is already
        // live" case a real SSE handler tolerates, but not what this test
        // means to prove.
        bus.bind(thread_id, run_id.clone()).await;
        let mut rx = bus.subscribe(&run_id).await.expect("bus must be bound");

        let run_task = tokio::spawn(async move { pool.run_once().await });

        let mut superstep_count = 0;
        let mut saw_done = false;
        loop {
            match rx.recv().await {
                Ok(event) => match event.kind {
                    RunStreamEventKind::Superstep => superstep_count += 1,
                    RunStreamEventKind::Done => {
                        saw_done = true;
                        break;
                    }
                    _ => {}
                },
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }

        assert!(
            superstep_count >= 3,
            "expected at least three superstep events over a three-node chain, got {superstep_count}"
        );
        assert!(saw_done, "the live stream must end with a done event");

        assert!(run_task.await.unwrap().unwrap());
    })
    .await
    .expect("live_stream_yields_progress_then_done must finish within 10s");
}

#[tokio::test(flavor = "multi_thread")]
async fn live_stream_yields_parley_on_gate() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
        let store = Arc::new(InMemoryWaypointStore::new());
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("gate", build_gate_graph()));

        let bus = Arc::new(RunEventBus::new());
        let engine = Arc::new(
            WarEngine::new(Arc::new(UnusedPaladinPort), store.clone())
                .with_trace_sink(Arc::new(RunEventBusSink::new(bus.clone()))),
        );
        let pool = RunWorkerPool::new(
            engine,
            store,
            repository.clone(),
            queue.clone(),
            resolver,
            Duration::from_secs(30),
        )
        .with_event_bus(bus.clone());

        let (run_id, thread_id) = submit(&repository, &queue, "gate").await;

        // Pre-bind and subscribe before spawning `run_once` -- see the
        // matching comment in `live_stream_yields_progress_then_done`.
        bus.bind(thread_id, run_id.clone()).await;
        let mut rx = bus.subscribe(&run_id).await.expect("bus must be bound");

        let run_task = tokio::spawn(async move { pool.run_once().await });

        let mut saw_parley = false;
        loop {
            match rx.recv().await {
                Ok(event) => {
                    if event.kind == RunStreamEventKind::Parley {
                        saw_parley = true;
                        let parleys = event
                            .payload
                            .get("parleys")
                            .and_then(|v| v.as_array())
                            .cloned()
                            .unwrap_or_default();
                        assert!(
                            !parleys.is_empty(),
                            "the parley event must carry the gate's ParleyRequest"
                        );
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }

        assert!(
            saw_parley,
            "expected a parley event when the run suspends on the gate"
        );
        assert!(run_task.await.unwrap().unwrap());
    })
    .await
    .expect("live_stream_yields_parley_on_gate must finish within 10s");
}

#[tokio::test]
async fn degraded_stream_terminates_with_done_for_terminal_run() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let waypoints: Arc<dyn WaypointPort> = Arc::new(InMemoryWaypointStore::new());
        let bus = Arc::new(RunEventBus::new());

        let run = Run::new(
            RunId::new_v7(),
            ThreadId::new("t-terminal").unwrap(),
            AssistantRef {
                assistant_id: "a1".to_string(),
                version: 1,
            },
            serde_json::json!({}),
        )
        .with_status(RunStatus::Completed);
        let run_id = run.run_id.clone();
        repository.insert(&run).await.unwrap();

        let service = RunEventStreamService::new(
            bus,
            repository.clone(),
            waypoints,
            Duration::from_millis(20),
        );
        let mut stream = service.stream(&run_id).await.unwrap();

        let mut saw_done = false;
        while let Some(event) = stream.next().await {
            assert_eq!(event.mode, RunStreamMode::Degraded);
            if event.kind == RunStreamEventKind::Done {
                saw_done = true;
                break;
            }
        }
        assert!(
            saw_done,
            "an already-terminal run's degraded stream must end with done"
        );
    })
    .await
    .expect("degraded_stream_terminates_with_done_for_terminal_run must finish within 5s");
}

#[tokio::test(flavor = "multi_thread")]
async fn degraded_stream_follows_remote_progress() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (repo_path, repo_url) = temp_sqlite_url("repo");
        let (wp_path, wp_url) = temp_sqlite_url("waypoint");

        let repository: Arc<dyn RunRepositoryPort> =
            Arc::new(SqliteRunRepository::new(&repo_url).await.unwrap());
        let waypoint_store = Arc::new(SqliteWaypointStore::new(&wp_url).await.unwrap());
        let waypoints_dyn: Arc<dyn WaypointPort> = waypoint_store.clone();
        let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());

        let graph = build_chain_graph(4, Duration::from_millis(100));
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("slow-chain", graph));

        // Pool A executes the run over the shared, real on-disk store --
        // NEVER wired to the bus the service under test reads from below.
        let engine_a = Arc::new(WarEngine::new(
            Arc::new(UnusedPaladinPort),
            waypoint_store.clone(),
        ));
        let pool_a = RunWorkerPool::new(
            engine_a,
            waypoint_store.clone(),
            repository.clone(),
            queue.clone(),
            resolver,
            Duration::from_secs(30),
        );

        let (run_id, _thread_id) = submit(&repository, &queue, "slow-chain").await;
        let run_task = tokio::spawn(async move { pool_a.run_once().await });

        // The service under test has its OWN, separate, never-bound bus --
        // it must fall back to degraded polling and observe pool A's
        // progress only through the durable store.
        let service_bus = Arc::new(RunEventBus::new());
        let service = RunEventStreamService::new(
            service_bus,
            repository.clone(),
            waypoints_dyn,
            Duration::from_millis(50),
        );
        let mut stream = service.stream(&run_id).await.unwrap();

        let mut saw_superstep = false;
        let mut saw_done = false;
        while let Some(event) = stream.next().await {
            assert_eq!(event.mode, RunStreamMode::Degraded);
            match event.kind {
                RunStreamEventKind::Superstep => saw_superstep = true,
                RunStreamEventKind::Done => {
                    saw_done = true;
                    break;
                }
                RunStreamEventKind::Error => panic!("the run must not fail: {:?}", event.payload),
                _ => {}
            }
        }

        assert!(
            saw_superstep,
            "the degraded stream must observe at least one superstep via polling"
        );
        assert!(saw_done, "the degraded stream must end with done");

        assert!(run_task.await.unwrap().unwrap());

        cleanup(&repo_path);
        cleanup(&wp_path);
    })
    .await
    .expect("degraded_stream_follows_remote_progress must finish within 30s");
}

#[tokio::test(flavor = "multi_thread")]
async fn state_delta_carries_field_names_only() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
        let store = Arc::new(InMemoryWaypointStore::new());
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("writer", build_field_writing_graph()));

        let bus = Arc::new(RunEventBus::new());
        let engine = Arc::new(
            WarEngine::new(Arc::new(UnusedPaladinPort), store.clone())
                .with_trace_sink(Arc::new(RunEventBusSink::new(bus.clone()))),
        );
        let pool = RunWorkerPool::new(
            engine,
            store,
            repository.clone(),
            queue.clone(),
            resolver,
            Duration::from_secs(30),
        )
        .with_event_bus(bus.clone());

        let (run_id, thread_id) = submit(&repository, &queue, "writer").await;

        // Pre-bind and subscribe before spawning `run_once` -- see the
        // matching comment in `live_stream_yields_progress_then_done`.
        bus.bind(thread_id, run_id.clone()).await;
        let mut rx = bus.subscribe(&run_id).await.expect("bus must be bound");

        let run_task = tokio::spawn(async move { pool.run_once().await });

        let mut found = false;
        loop {
            match rx.recv().await {
                Ok(event) => {
                    if event.kind == RunStreamEventKind::StateDelta {
                        found = true;
                        let payload_str = event.payload.to_string();
                        assert!(
                            !payload_str.contains(SENTINEL_VALUE),
                            "state_delta must never carry the changed field's value: {payload_str}"
                        );
                        let fields = event
                            .payload
                            .get("fields")
                            .and_then(|v| v.as_array())
                            .expect("fields array");
                        assert!(
                            fields.iter().any(|f| f.as_str() == Some("secret_field")),
                            "fields must name the changed field: {fields:?}"
                        );
                        assert!(
                            event
                                .payload
                                .get("bytes")
                                .and_then(|v| v.as_u64())
                                .is_some(),
                            "bytes must be a byte-size count, not the value itself"
                        );
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }

        assert!(found, "expected a state_delta event");
        assert!(run_task.await.unwrap().unwrap());
    })
    .await
    .expect("state_delta_carries_field_names_only must finish within 10s");
}
