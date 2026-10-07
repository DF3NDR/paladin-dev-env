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

use paladin_battalion::engine::shutdown::ShutdownCoordinator;
use paladin_battalion::engine::{
    EdgeSpec, EngineLimits, InputMapping, NodeContext, NodeSpec, StateNode, StateNodeError,
    WarEngine, WarGraph, graph::GateRequestTemplate,
};
use paladin_core::platform::container::allowance::HaltReason;
use paladin_core::platform::container::battlefield::{
    Battlefield, BattlefieldSchema, DispatchRule, FieldName, FieldSpec, StateDelta,
};
use paladin_core::platform::container::directive::Directive;
use paladin_core::platform::container::execution_result::PaladinResult;
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::parley::ParleyKind;
use paladin_core::platform::container::principal::{RunAttribution, TenantId};
use paladin_core::platform::container::run::{
    AssistantRef, Run, RunId, RunStatus, RunStreamEvent, RunStreamEventKind, RunStreamMode,
};
use paladin_core::platform::container::token_usage::TokenUsage;
use paladin_core::platform::container::treasury_ledger::{LedgerScope, SettlementKey};
use paladin_core::platform::container::waypoint::{NodeId, ThreadId};
use paladin_ports::input::run_event_stream_port::RunEventStreamPort;
use paladin_ports::input::run_submission_port::RunSubmissionPort;
use paladin_ports::output::paladin_port::{PaladinPort, PaladinStream};
use paladin_ports::output::run_queue_port::{QueuedRun, RunQueuePort};
use paladin_ports::output::run_repository_port::RunRepositoryPort;
use paladin_ports::output::run_trace_port::{RunTraceError, RunTracePort};
use paladin_ports::output::spend_guard::{SpendDecision, SpendGuard};
use paladin_ports::output::trace_sink_port::{CompositeSink, TraceEvent, TraceRecord, TraceSink};
use paladin_ports::output::treasury_ledger_port::TreasuryLedgerPort;
use paladin_ports::output::waypoint_port::WaypointPort;
use paladin_storage::run::in_memory::InMemoryRunRepository;
use paladin_storage::run::sqlite::SqliteRunRepository;
use paladin_storage::run_queue::in_memory::InMemoryRunQueue;
use paladin_storage::run_trace::in_memory::InMemoryRunTraceStore;
use paladin_storage::treasury::contract_tests::{settle_request, usd};
use paladin_storage::treasury::sqlite::SqliteTreasuryLedger;
use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;
use paladin_storage::waypoint::sqlite::SqliteWaypointStore;

use crate::application::services::treasurer::{Treasurer, window_for};
use crate::config::trace::TraceConfig;
use crate::config::treasurer::TreasurerConfig;
use crate::infrastructure::telemetry::PersistingTraceSink;

use super::events::{RunEventBus, RunEventBusSink, RunEventStreamService, map_trace_event};
use super::resolver::{AssistantResolver, CodeWorkflowResolver};
use super::submission::RunSubmissionService;
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

/// D-14's completion, exercised at this integration layer too (the plan's
/// own acceptance criteria pin this exact test name in THIS file, alongside
/// the pure-function coverage table in `events.rs`'s own test module):
/// `map_trace_event` is total over the twelve-variant `TraceEvent` enum,
/// producing exactly seven of the wire kinds (`RunFinished` alone produces
/// two, `done` and `error`, split on `status` -- thirteen rows over twelve
/// variants).
#[test]
fn map_trace_event_covers_exactly_seven_of_twelve() {
    use paladin_core::platform::container::parley::ParleyId;
    use paladin_core::platform::container::waypoint::NodeOutcomeKind;
    use paladin_ports::output::trace_sink_port::{
        FieldChange, MiddlewareAction, RunFinishStatus, TraceEvent,
    };

    fn wrap(
        thread_id: ThreadId,
        seq: u64,
        event: TraceEvent,
    ) -> paladin_ports::output::trace_sink_port::TraceRecord {
        paladin_ports::output::trace_sink_port::TraceRecord {
            thread_id,
            run_id: None,
            seq,
            at: chrono::Utc::now(),
            event,
        }
    }

    let thread_id = ThreadId::new("t1").unwrap();
    let cases: Vec<(&str, TraceEvent, bool)> = vec![
        (
            "RunStarted",
            TraceEvent::RunStarted {
                run_id: None,
                graph_fingerprint: "fp".to_string(),
            },
            false,
        ),
        (
            "SuperstepStarted",
            TraceEvent::SuperstepStarted {
                superstep: 1,
                vanguard: vec![NodeId::new("n1")],
            },
            true,
        ),
        (
            "NodeStarted",
            TraceEvent::NodeStarted {
                superstep: 1,
                node_id: NodeId::new("n1"),
                attempt: 1,
                muster_task_key: None,
            },
            true,
        ),
        (
            "NodeProgress",
            TraceEvent::NodeProgress {
                node_id: NodeId::new("n1"),
                progress: paladin_ports::output::trace_sink_port::NodeProgressKind::Heartbeat,
            },
            false,
        ),
        (
            "NodeFinished",
            TraceEvent::NodeFinished {
                superstep: 1,
                node_id: NodeId::new("n1"),
                attempt: 1,
                outcome: NodeOutcomeKind::Succeeded,
                duration_ms: 5,
                usage: TokenUsage::default(),
                cost: None,
                cache_hit: false,
            },
            true,
        ),
        (
            "EdgeEvaluated",
            TraceEvent::EdgeEvaluated {
                from: NodeId::new("a"),
                to: NodeId::new("b"),
                condition_kind: "always".to_string(),
                fired: true,
            },
            false,
        ),
        (
            "DeltaMerged",
            TraceEvent::DeltaMerged {
                superstep: 1,
                field_changes: vec![FieldChange {
                    field: FieldName::new("x").unwrap(),
                    dispatch: "last_write".to_string(),
                    writers: vec![NodeId::new("n1")],
                    value_bytes: 4,
                    value: None,
                }],
            },
            true,
        ),
        (
            "WaypointSaved",
            TraceEvent::WaypointSaved {
                waypoint_id: paladin_core::platform::container::waypoint::WaypointId::generate(),
                superstep: 1,
                status: "completed".to_string(),
            },
            false,
        ),
        (
            "ParleyRaised",
            TraceEvent::ParleyRaised {
                parley_id: ParleyId::new(),
                node_id: NodeId::new("n1"),
                parley_kind: ParleyKind::Approval,
            },
            true,
        ),
        (
            "RunFinished{Completed}",
            TraceEvent::RunFinished {
                status: RunFinishStatus::Completed,
                total_supersteps: 1,
                usage: TokenUsage::default(),
                cost: None,
                halt_reason: None,
                duration_ms: 5,
                trace_dropped_total: 0,
            },
            true,
        ),
        (
            "RunFinished{Failed}",
            TraceEvent::RunFinished {
                status: RunFinishStatus::Failed,
                total_supersteps: 1,
                usage: TokenUsage::default(),
                cost: None,
                halt_reason: None,
                duration_ms: 5,
                trace_dropped_total: 0,
            },
            true,
        ),
        (
            "FallbackHop",
            TraceEvent::FallbackHop {
                node_id: None,
                from_provider: "openai".to_string(),
                to_provider: "anthropic".to_string(),
            },
            false,
        ),
        (
            "MiddlewareEvent",
            TraceEvent::MiddlewareEvent {
                name: "limit".to_string(),
                action: MiddlewareAction::Finish,
            },
            false,
        ),
    ];
    assert_eq!(
        cases.len(),
        13,
        "must enumerate all twelve variants, with RunFinished split into its two status rows"
    );

    let mut mapped = 0;
    let mut dropped = 0;
    for (seq, (name, event, expect_some)) in cases.into_iter().enumerate() {
        let record = wrap(thread_id.clone(), seq as u64 + 1, event);
        match map_trace_event(record) {
            Some(_) if expect_some => mapped += 1,
            None if !expect_some => dropped += 1,
            other => panic!("unexpected mapping result for {name}: {other:?}"),
        }
    }
    assert_eq!(mapped, 7, "exactly seven rows must map to Some");
    assert_eq!(dropped, 6, "exactly six rows must map to None");
}

// --- Replay mode (D-16) --------------------------------------------------

/// Poll `store` until it holds a `RunFinished` record for `thread_id`, or
/// panic after `timeout` -- `paladin-battalion`'s `TraceDispatcher` is
/// deliberately fire-and-forget (ENG-FR-21), so a run's `run_once` return
/// does not guarantee its trailing records already landed in the store.
async fn wait_for_run_finished_persisted(
    store: &InMemoryRunTraceStore,
    thread_id: &ThreadId,
    timeout: Duration,
) {
    tokio::time::timeout(timeout, async {
        loop {
            let rows = store.read(thread_id, 0, 1024).await.unwrap();
            if rows
                .iter()
                .any(|r| matches!(r.event, TraceEvent::RunFinished { .. }))
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("RunFinished must be persisted within the timeout");
}

/// Behavior: a finished run with `run_traces` rows streams every mapped
/// record with `mode: replay`, and terminates with `done` (D-16).
#[tokio::test(flavor = "multi_thread")]
async fn terminal_run_with_rows_replays() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
        let store = Arc::new(InMemoryWaypointStore::new());
        let trace_store = Arc::new(InMemoryRunTraceStore::new());
        let graph = build_chain_graph(3, Duration::from_millis(5));
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("chain", graph));

        let run_bus = Arc::new(RunEventBus::new());
        let trace_sink: Arc<dyn TraceSink> = Arc::new(CompositeSink::new(vec![
            Arc::new(RunEventBusSink::new(run_bus.clone())),
            Arc::new(PersistingTraceSink::new(
                trace_store.clone() as Arc<dyn RunTracePort>
            )),
        ]));
        let engine = Arc::new(
            WarEngine::new(Arc::new(UnusedPaladinPort), store.clone()).with_trace_sink(trace_sink),
        );
        let pool = RunWorkerPool::new(
            engine,
            store.clone(),
            repository.clone(),
            queue.clone(),
            resolver,
            Duration::from_secs(30),
        )
        .with_event_bus(run_bus.clone());

        let (run_id, thread_id) = submit(&repository, &queue, "chain").await;
        run_bus.bind(thread_id.clone(), run_id.clone()).await;

        assert!(pool.run_once().await.unwrap());
        wait_for_run_finished_persisted(&trace_store, &thread_id, Duration::from_secs(5)).await;

        // The replay service's OWN bus is never bound for this run --
        // guarantees the live path is unreachable regardless of the
        // producing pool's own unbind timing.
        let service_bus = Arc::new(RunEventBus::new());
        let waypoints_dyn: Arc<dyn WaypointPort> = store;
        let service = RunEventStreamService::new(
            service_bus,
            repository.clone(),
            waypoints_dyn,
            Duration::from_millis(20),
        )
        .with_replay(trace_store.clone() as Arc<dyn RunTracePort>);

        let mut stream = service.stream(&run_id).await.unwrap();
        let mut saw_done = false;
        let mut saw_superstep = false;
        while let Some(event) = stream.next().await {
            assert_eq!(event.mode, RunStreamMode::Replay);
            match event.kind {
                RunStreamEventKind::Superstep => saw_superstep = true,
                RunStreamEventKind::Done => {
                    saw_done = true;
                    break;
                }
                _ => {}
            }
        }
        assert!(saw_superstep, "the replay must include superstep events");
        assert!(saw_done, "a terminal run's replay must end with done");
    })
    .await
    .expect("terminal_run_with_rows_replays must finish within 10s");
}

/// Behavior: for one run, the live stream and a subsequent replay of the
/// SAME run yield the same ordered sequence of wire kinds and payload
/// fields, apart from `mode` (D-16) -- both routes read the SAME
/// dispatcher's record stream (one live, through the bus; one persisted,
/// through `run_traces`), so `map_trace_event` produces byte-identical
/// payloads for the same underlying records.
#[tokio::test(flavor = "multi_thread")]
async fn replay_and_live_produce_the_same_wire_sequence() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
        let store = Arc::new(InMemoryWaypointStore::new());
        let trace_store = Arc::new(InMemoryRunTraceStore::new());
        let graph = build_chain_graph(3, Duration::from_millis(5));
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("chain", graph));

        let run_bus = Arc::new(RunEventBus::new());
        let trace_sink: Arc<dyn TraceSink> = Arc::new(CompositeSink::new(vec![
            Arc::new(RunEventBusSink::new(run_bus.clone())),
            Arc::new(PersistingTraceSink::new(
                trace_store.clone() as Arc<dyn RunTracePort>
            )),
        ]));
        let engine = Arc::new(
            WarEngine::new(Arc::new(UnusedPaladinPort), store.clone()).with_trace_sink(trace_sink),
        );
        let pool = RunWorkerPool::new(
            engine,
            store.clone(),
            repository.clone(),
            queue.clone(),
            resolver,
            Duration::from_secs(30),
        )
        .with_event_bus(run_bus.clone());

        let (run_id, thread_id) = submit(&repository, &queue, "chain").await;
        run_bus.bind(thread_id.clone(), run_id.clone()).await;
        let mut live_rx = run_bus.subscribe(&run_id).await.unwrap();

        let run_task = tokio::spawn(async move { pool.run_once().await });

        let mut live_sequence = Vec::new();
        loop {
            match live_rx.recv().await {
                Ok(event) => {
                    let is_done = event.kind == RunStreamEventKind::Done;
                    live_sequence.push((event.kind, event.payload));
                    if is_done {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
        assert!(run_task.await.unwrap().unwrap());

        wait_for_run_finished_persisted(&trace_store, &thread_id, Duration::from_secs(5)).await;

        let service_bus = Arc::new(RunEventBus::new());
        let waypoints_dyn: Arc<dyn WaypointPort> = store;
        let service = RunEventStreamService::new(
            service_bus,
            repository.clone(),
            waypoints_dyn,
            Duration::from_millis(20),
        )
        .with_replay(trace_store.clone() as Arc<dyn RunTracePort>);
        let mut stream = service.stream(&run_id).await.unwrap();

        let mut replay_sequence = Vec::new();
        while let Some(event) = stream.next().await {
            assert_eq!(event.mode, RunStreamMode::Replay);
            let is_done = event.kind == RunStreamEventKind::Done;
            replay_sequence.push((event.kind, event.payload));
            if is_done {
                break;
            }
        }

        assert_eq!(
            live_sequence, replay_sequence,
            "the live and replayed sequences (kind, payload) must be identical"
        );
    })
    .await
    .expect("replay_and_live_produce_the_same_wire_sequence must finish within 10s");
}

/// Behavior: with persistence off (no rows for the thread at all), a
/// terminal run's stream falls straight through to the existing degraded
/// Waypoint-polling path, unchanged -- and its synthesized events carry no
/// `trace_seq` (D-16).
#[tokio::test]
async fn terminal_run_without_rows_falls_back_to_degraded() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let waypoints: Arc<dyn WaypointPort> = Arc::new(InMemoryWaypointStore::new());
        let bus = Arc::new(RunEventBus::new());
        let empty_trace_store: Arc<dyn RunTracePort> = Arc::new(InMemoryRunTraceStore::new());

        let run = Run::new(
            RunId::new_v7(),
            ThreadId::new("t-terminal-no-rows").unwrap(),
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
        )
        .with_replay(empty_trace_store);
        let mut stream = service.stream(&run_id).await.unwrap();

        let mut saw_done = false;
        while let Some(event) = stream.next().await {
            assert_eq!(event.mode, RunStreamMode::Degraded);
            assert!(
                event.payload.get("trace_seq").is_none(),
                "a degraded-mode event must never carry trace_seq"
            );
            if event.kind == RunStreamEventKind::Done {
                saw_done = true;
                break;
            }
        }
        assert!(
            saw_done,
            "an already-terminal run with no persisted rows must still fall back to degraded \
             and end with done"
        );
    })
    .await
    .expect("terminal_run_without_rows_falls_back_to_degraded must finish within 5s");
}

/// A [`RunTracePort`] wrapping a real [`InMemoryRunTraceStore`] but capping
/// every `read`'s effective page size to `page_size`, regardless of the
/// caller's requested `limit` -- proves the replay loop actually issues
/// MULTIPLE `read` calls to walk a run's full record set, rather than
/// assuming one call returns everything.
struct PagingSpy {
    inner: Arc<InMemoryRunTraceStore>,
    page_size: usize,
    read_calls: Arc<std::sync::atomic::AtomicUsize>,
}

#[async_trait]
impl RunTracePort for PagingSpy {
    async fn append(&self, records: &[TraceRecord]) -> Result<(), RunTraceError> {
        self.inner.append(records).await
    }

    async fn read(
        &self,
        thread: &ThreadId,
        after_seq: u64,
        limit: u32,
    ) -> Result<Vec<TraceRecord>, RunTraceError> {
        self.read_calls
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let capped = (self.page_size as u32).min(limit);
        self.inner.read(thread, after_seq, capped).await
    }

    async fn prune_thread(
        &self,
        thread: &ThreadId,
        before_superstep: u64,
    ) -> Result<u64, RunTraceError> {
        self.inner.prune_thread(thread, before_superstep).await
    }
}

/// Behavior: a run with more records than one `read` page size is replayed
/// completely, in order, with no repeats -- proven against a backend
/// deliberately capped to a page size of 2 over 6 persisted records
/// (T-28-11-03).
#[tokio::test]
async fn replay_paginates_through_the_port() {
    let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
    let waypoints: Arc<dyn WaypointPort> = Arc::new(InMemoryWaypointStore::new());
    let inner = Arc::new(InMemoryRunTraceStore::new());
    let read_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let spy: Arc<dyn RunTracePort> = Arc::new(PagingSpy {
        inner: inner.clone(),
        page_size: 2,
        read_calls: read_calls.clone(),
    });

    let run = Run::new(
        RunId::new_v7(),
        ThreadId::new("t-paginate").unwrap(),
        AssistantRef {
            assistant_id: "a1".to_string(),
            version: 1,
        },
        serde_json::json!({}),
    )
    .with_status(RunStatus::Completed);
    let run_id = run.run_id.clone();
    let thread_id = run.thread_id.clone();
    repository.insert(&run).await.unwrap();

    let mut records: Vec<TraceRecord> = (1..=5u64)
        .map(|seq| TraceRecord {
            thread_id: thread_id.clone(),
            run_id: None,
            seq,
            at: chrono::Utc::now(),
            event: TraceEvent::SuperstepStarted {
                superstep: seq,
                vanguard: vec![],
            },
        })
        .collect();
    records.push(TraceRecord {
        thread_id: thread_id.clone(),
        run_id: None,
        seq: 6,
        at: chrono::Utc::now(),
        event: TraceEvent::RunFinished {
            status: paladin_ports::output::trace_sink_port::RunFinishStatus::Completed,
            total_supersteps: 5,
            usage: TokenUsage::default(),
            cost: None,
            halt_reason: None,
            duration_ms: 1,
            trace_dropped_total: 0,
        },
    });
    inner.append(&records).await.unwrap();

    let bus = Arc::new(RunEventBus::new());
    let service = RunEventStreamService::new(
        bus,
        repository.clone(),
        waypoints,
        Duration::from_millis(10),
    )
    .with_replay(spy);
    let mut stream = service.stream(&run_id).await.unwrap();

    let mut kinds = Vec::new();
    while let Some(event) = stream.next().await {
        assert_eq!(event.mode, RunStreamMode::Replay);
        let is_done = event.kind == RunStreamEventKind::Done;
        kinds.push(event.kind);
        if is_done {
            break;
        }
    }
    assert_eq!(
        kinds.len(),
        6,
        "all five superstep records plus the terminal done must be replayed: {kinds:?}"
    );
    assert!(
        read_calls.load(std::sync::atomic::Ordering::SeqCst) >= 3,
        "a page size of 2 over 6 records must take at least 3 read calls, got {}",
        read_calls.load(std::sync::atomic::Ordering::SeqCst)
    );
}

// --- The halted `done` on the live, degraded and replay paths (PLAT-09, D-14, D-16, G10) ----

/// A node standing in for a priced superstep: when `amount_nanos` is positive it settles that
/// many nano-units against `scope` through the real ledger, exactly what the engine's
/// settlement does after a metered model call (a test-local copy of `http_surface_tests`'s
/// `SpendingNode`, per this module's small-double precedent).
struct SpendStep {
    ledger: Arc<dyn TreasuryLedgerPort>,
    scope: LedgerScope,
    amount_nanos: i64,
}

#[async_trait]
impl StateNode for SpendStep {
    async fn run(
        &self,
        _state: &Battlefield,
        _ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        if self.amount_nanos > 0 {
            self.ledger
                .settle(settle_request(
                    self.scope.clone(),
                    SettlementKey::new(RunId::new_v7(), 0, 0),
                    self.amount_nanos,
                    usd(),
                    "gpt-4",
                ))
                .await
                .map_err(|e| StateNodeError(format!("test settlement failed: {e}")))?;
        }
        Ok(StateDelta::new().into())
    }
}

/// A [`SpendGuard`] that halts every run on its first boundary with a fixed reason.
struct HaltsWith(HaltReason);

#[async_trait]
impl SpendGuard for HaltsWith {
    async fn check(&self, _thread: &ThreadId) -> SpendDecision {
        SpendDecision::Halt(self.0.clone())
    }
}

/// How a test run comes to halt.
enum HaltScript {
    /// A real Treasurer over a real SQLite ledger: the run's first superstep spends the whole
    /// 1.00 USD allowance, so the guard halts it at the second boundary with
    /// `allowance_exhausted`.
    Spend,
    /// A stub guard that halts at the first boundary with the given reason (used for
    /// `ledger_unavailable`, which needs no ledger at all).
    Guard(HaltReason),
    /// A caller cancel through the instance that dispatches the run (the in-process route: only
    /// the run's own child token fires; no durable-flag probing is wired, D-14, G1b).
    CancelSameInstance,
    /// A caller cancel through another instance (the durable flag only, observed by the
    /// debounced probe, G1a).
    CancelCrossInstance,
}

impl HaltScript {
    /// Whether this script ends with a caller cancel rather than a halt.
    fn is_cancel(&self) -> bool {
        matches!(self, Self::CancelSameInstance | Self::CancelCrossInstance)
    }
}

/// One halted run's persisted artefacts plus the `done` its live subscriber saw.
struct HaltedRun {
    repository: Arc<dyn RunRepositoryPort>,
    waypoints: Arc<InMemoryWaypointStore>,
    traces: Arc<InMemoryRunTraceStore>,
    run_id: RunId,
    live_terminal: (RunStreamEventKind, serde_json::Value),
    cleanup_path: Option<std::path::PathBuf>,
}

impl HaltedRun {
    fn release(self) {
        if let Some(path) = &self.cleanup_path {
            cleanup(path);
        }
    }
}

/// Poll the run row until it is terminal (G14: the engine's `done` can precede the row's
/// status write), panicking after `timeout`.
async fn wait_for_terminal_row(
    repository: &Arc<dyn RunRepositoryPort>,
    run_id: &RunId,
    timeout: Duration,
) -> Run {
    tokio::time::timeout(timeout, async {
        loop {
            let run = repository.get(run_id).await.unwrap().unwrap();
            if run.status.is_terminal() {
                return run;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the run row must reach a terminal status")
}

/// Drive one run to a halt through a real worker pool (the live bus, the persisted trace
/// pipeline and the run row all populated by production code), capturing the live terminal
/// event. `None` when the allowance window rolled over mid-scenario (Pitfall 10) -- the caller
/// retries once.
async fn drive_halted_run(script: &HaltScript) -> Option<HaltedRun> {
    let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
    let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
    let waypoints = Arc::new(InMemoryWaypointStore::new());
    let traces = Arc::new(InMemoryRunTraceStore::new());
    let bus = Arc::new(RunEventBus::new());

    let mut cleanup_path = None;
    let mut treasurer = None;
    let mut ledger_for_window = None;
    let graph = match script {
        HaltScript::Guard(_) => build_chain_graph(3, Duration::from_millis(2)),
        // Slow enough that the cancel lands while the run is still going.
        HaltScript::CancelSameInstance | HaltScript::CancelCrossInstance => {
            build_chain_graph(6, Duration::from_millis(100))
        }
        HaltScript::Spend => {
            let (path, url) = temp_sqlite_url("halted_done");
            let ledger = Arc::new(SqliteTreasuryLedger::new(&url).await.unwrap());
            let ledger_port: Arc<dyn TreasuryLedgerPort> = ledger.clone();
            let config: TreasurerConfig = serde_json::from_value(serde_json::json!({
                "currency": "USD",
                "allowance": { "api_keys": { "svc-h": { "period": "1d", "amount": "1.00" } } }
            }))
            .unwrap();
            treasurer = Some(Arc::new(Treasurer::new(
                config.allowance_policy().unwrap(),
                Arc::clone(&ledger_port),
            )));
            let mut graph = WarGraph::new(BattlefieldSchema::new(vec![]), EngineLimits::default());
            let ids: Vec<NodeId> = (0..3).map(|i| NodeId::new(format!("n{i}"))).collect();
            for (i, id) in ids.iter().enumerate() {
                graph.add_node(
                    id.clone(),
                    NodeSpec::Function(Arc::new(SpendStep {
                        ledger: Arc::clone(&ledger_port),
                        scope: LedgerScope::new("acme", "svc-h"),
                        amount_nanos: if i == 0 { 1_000_000_000 } else { 0 },
                    })),
                );
            }
            for pair in ids.windows(2) {
                graph.add_edge(EdgeSpec {
                    from: pair[0].clone(),
                    to: pair[1].clone(),
                    condition: None,
                });
            }
            graph.add_entry(ids[0].clone());
            cleanup_path = Some(path);
            ledger_for_window = Some(ledger);
            Arc::new(graph)
        }
    };
    let window_before = match &ledger_for_window {
        Some(ledger) => window_for(ledger.store_now().await.unwrap(), 86_400),
        None => None,
    };
    let resolver: Arc<dyn AssistantResolver> =
        Arc::new(CodeWorkflowResolver::new().register("halt-wf", graph));

    let run_id = RunId::new_v7();
    let thread_id = ThreadId::new(format!("thread-{run_id}")).unwrap();
    let mut run = Run::new(
        run_id.clone(),
        thread_id.clone(),
        AssistantRef {
            assistant_id: "halt-wf".to_string(),
            version: 1,
        },
        serde_json::json!({}),
    );
    if matches!(script, HaltScript::Spend) {
        run = run.with_submitted_by(RunAttribution::new(TenantId::new("acme").unwrap(), "svc-h"));
    }
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

    let stub_reason = match script {
        HaltScript::Guard(reason) => Some(reason.clone()),
        HaltScript::Spend | HaltScript::CancelSameInstance | HaltScript::CancelCrossInstance => {
            None
        }
    };
    let factory_store = waypoints.clone();
    let engine_factory: Arc<
        dyn Fn(tokio_util::sync::CancellationToken) -> WarEngine<InMemoryWaypointStore>
            + Send
            + Sync,
    > = Arc::new(move |token| {
        let engine = WarEngine::new(Arc::new(UnusedPaladinPort), factory_store.clone())
            .with_cancellation_token(token);
        match &stub_reason {
            Some(reason) => engine.with_spend_guard(Arc::new(HaltsWith(reason.clone()))),
            None => engine,
        }
    });
    let mut pool = RunWorkerPool::new(
        Arc::new(WarEngine::new(
            Arc::new(UnusedPaladinPort),
            waypoints.clone(),
        )),
        waypoints.clone(),
        repository.clone(),
        queue.clone(),
        resolver,
        Duration::from_secs(30),
    )
    .with_engine_factory(engine_factory)
    .with_event_bus(bus.clone())
    .with_trace_config(TraceConfig {
        log_sink: false,
        persist: true,
        ..TraceConfig::default()
    })
    .with_run_trace_port(traces.clone() as Arc<dyn RunTracePort>);
    if let Some(treasurer) = treasurer {
        pool = pool.with_treasurer(treasurer);
    }
    // The cross-instance route needs the debounced durable-flag probe; the same-instance route
    // deliberately has none, so it proves the per-run probe alone reports a local cancel.
    if matches!(script, HaltScript::CancelCrossInstance) {
        pool = pool.with_cancellation_probing(Duration::from_millis(50));
    }
    // The submission service a cancelling caller talks to: wired to this pool's local-token
    // registry for the same-instance route, and to nothing (another instance) otherwise.
    let mut cancel_service = RunSubmissionService::new(
        repository.clone(),
        queue.clone(),
        Arc::new(CodeWorkflowResolver::new()),
    );
    if matches!(script, HaltScript::CancelSameInstance) {
        cancel_service = cancel_service.with_local_tokens(pool.local_tokens());
    }
    let mut cancel_pending = script.is_cancel();

    // Subscribe BEFORE dispatch: a broadcast never buffers for a late subscriber.
    bus.bind(thread_id.clone(), run_id.clone()).await;
    let mut rx = bus.subscribe(&run_id).await.expect("bus must be bound");
    let run_task = tokio::spawn(async move { pool.run_once().await });

    let mut live_terminal = None;
    loop {
        match rx.recv().await {
            Ok(event) => {
                // A cancel scenario cancels once, as soon as the run is visibly going.
                if cancel_pending && event.kind == RunStreamEventKind::Superstep {
                    cancel_pending = false;
                    cancel_service.cancel(&run_id, None).await.unwrap();
                }
                if matches!(
                    event.kind,
                    RunStreamEventKind::Done | RunStreamEventKind::Error
                ) {
                    live_terminal = Some((event.kind, event.payload));
                    break;
                }
            }
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }
    assert!(run_task.await.unwrap().unwrap());
    let live_terminal = live_terminal.expect("the live stream must end with a terminal event");

    wait_for_terminal_row(&repository, &run_id, Duration::from_secs(5)).await;
    wait_for_run_finished_persisted(&traces, &thread_id, Duration::from_secs(5)).await;

    if let Some(ledger) = &ledger_for_window {
        let window_after = window_for(ledger.store_now().await.unwrap(), 86_400);
        if window_before != window_after {
            if let Some(path) = &cleanup_path {
                cleanup(path);
            }
            return None;
        }
    }
    Some(HaltedRun {
        repository,
        waypoints,
        traces,
        run_id,
        live_terminal,
        cleanup_path,
    })
}

/// Drive a halted run, retrying once when an allowance window boundary was crossed.
async fn halted_run(script: HaltScript) -> HaltedRun {
    for _ in 0..2 {
        if let Some(rig) = drive_halted_run(&script).await {
            return rig;
        }
    }
    panic!("the allowance window boundary was crossed on both attempts");
}

/// The terminal event of the degraded polling path for `rig`'s run (no replay port wired).
async fn degraded_terminal(rig: &HaltedRun) -> (RunStreamEventKind, serde_json::Value) {
    let waypoints: Arc<dyn WaypointPort> = rig.waypoints.clone();
    let service = RunEventStreamService::new(
        Arc::new(RunEventBus::new()),
        rig.repository.clone(),
        waypoints,
        Duration::from_millis(20),
    );
    let mut stream = service.stream(&rig.run_id).await.unwrap();
    while let Some(event) = stream.next().await {
        assert_eq!(event.mode, RunStreamMode::Degraded);
        if matches!(
            event.kind,
            RunStreamEventKind::Done | RunStreamEventKind::Error
        ) {
            return (event.kind, event.payload);
        }
    }
    panic!("the degraded stream ended without a terminal event");
}

/// The terminal event of the replay path for `rig`'s run.
async fn replay_terminal(rig: &HaltedRun) -> (RunStreamEventKind, serde_json::Value) {
    let waypoints: Arc<dyn WaypointPort> = rig.waypoints.clone();
    let service = RunEventStreamService::new(
        Arc::new(RunEventBus::new()),
        rig.repository.clone(),
        waypoints,
        Duration::from_millis(20),
    )
    .with_replay(rig.traces.clone() as Arc<dyn RunTracePort>);
    let mut stream = service.stream(&rig.run_id).await.unwrap();
    while let Some(event) = stream.next().await {
        assert_eq!(event.mode, RunStreamMode::Replay);
        if matches!(
            event.kind,
            RunStreamEventKind::Done | RunStreamEventKind::Error
        ) {
            return (event.kind, event.payload);
        }
    }
    panic!("the replay stream ended without a terminal event");
}

/// The two fields every path must agree on byte for byte: `status` and `halt_reason`
/// (`waypoint_id`, `usage` and `trace_seq` follow each path's own existing rule).
fn status_and_reason(payload: &serde_json::Value) -> (String, Option<String>) {
    (
        serde_json::to_string(&payload["status"]).unwrap(),
        payload
            .get("halt_reason")
            .map(|reason| serde_json::to_string(reason).unwrap()),
    )
}

/// PLAT-09, D-14, D-16: one spend-halted run observed live, then degraded, then by replay
/// emits a `done` with the same `status` and the same `halt_reason` object on all three, and
/// that object is the run row's own `wire_json`.
#[tokio::test(flavor = "multi_thread")]
async fn halted_done_agrees_on_live_degraded_and_replay() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let rig = halted_run(HaltScript::Spend).await;

        let row = rig.repository.get(&rig.run_id).await.unwrap().unwrap();
        assert_eq!(row.status, RunStatus::Halted);
        let row_reason = row
            .halt_reason
            .as_ref()
            .expect("a spend-halted row carries its reason")
            .wire_json();

        let (live_kind, live) = rig.live_terminal.clone();
        let (degraded_kind, degraded) = degraded_terminal(&rig).await;
        let (replay_kind, replay) = replay_terminal(&rig).await;

        for kind in [live_kind, degraded_kind, replay_kind] {
            assert_eq!(
                kind,
                RunStreamEventKind::Done,
                "a halt is a done, never an error"
            );
        }
        assert_eq!(live["status"], "halted");
        assert_eq!(live["halt_reason"], row_reason, "live done: {live}");
        assert_eq!(live["halt_reason"]["reason"], "allowance_exhausted");
        assert_eq!(
            status_and_reason(&live),
            status_and_reason(&degraded),
            "live {live} vs degraded {degraded}"
        );
        assert_eq!(
            status_and_reason(&live),
            status_and_reason(&replay),
            "live {live} vs replay {replay}"
        );
        rig.release();
    })
    .await
    .expect("halted_done_agrees_on_live_degraded_and_replay timed out");
}

/// D-16: a ledger-unavailable halt is a `done` carrying `{"reason":"ledger_unavailable"}` on
/// the live and degraded (and replay) paths -- never an `error` event.
#[tokio::test(flavor = "multi_thread")]
async fn ledger_unavailable_done_is_done_not_error_on_live_and_degraded() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let rig = halted_run(HaltScript::Guard(HaltReason::LedgerUnavailable)).await;
        let expected = serde_json::json!({ "reason": "ledger_unavailable" });

        let (live_kind, live) = rig.live_terminal.clone();
        let (degraded_kind, degraded) = degraded_terminal(&rig).await;
        let (replay_kind, replay) = replay_terminal(&rig).await;

        for (path, kind, payload) in [
            ("live", live_kind, &live),
            ("degraded", degraded_kind, &degraded),
            ("replay", replay_kind, &replay),
        ] {
            assert_eq!(
                kind,
                RunStreamEventKind::Done,
                "{path} must emit done, not error"
            );
            assert_eq!(payload["status"], "halted", "{path}: {payload}");
            assert_eq!(payload["halt_reason"], expected, "{path}: {payload}");
        }
        rig.release();
    })
    .await
    .expect("ledger_unavailable_done_is_done_not_error_on_live_and_degraded timed out");
}

/// PLAT-09 idempotency: two successive degraded reads and two successive replays of the same
/// terminal halted run each emit a terminal `done` whose `status` and `halt_reason` are
/// byte-identical to the first.
#[tokio::test(flavor = "multi_thread")]
async fn repeated_degraded_and_replay_reads_of_a_halted_run_are_identical() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let rig = halted_run(HaltScript::Guard(HaltReason::LedgerUnavailable)).await;

        let (_, degraded_first) = degraded_terminal(&rig).await;
        let (_, degraded_second) = degraded_terminal(&rig).await;
        assert_eq!(
            status_and_reason(&degraded_first),
            status_and_reason(&degraded_second)
        );
        assert!(status_and_reason(&degraded_first).1.is_some());

        let (_, replay_first) = replay_terminal(&rig).await;
        let (_, replay_second) = replay_terminal(&rig).await;
        assert_eq!(
            status_and_reason(&replay_first),
            status_and_reason(&replay_second)
        );
        assert_eq!(
            status_and_reason(&degraded_first),
            status_and_reason(&replay_first)
        );
        rig.release();
    })
    .await
    .expect("repeated_degraded_and_replay_reads_of_a_halted_run_are_identical timed out");
}

/// A trace store holding a superstep record and a persisted `RunFinished { Halted }` for a
/// thread (no run id on the records, as a drained worker writes them).
async fn seed_halted_trace(thread_id: &ThreadId) -> Arc<InMemoryRunTraceStore> {
    let store = Arc::new(InMemoryRunTraceStore::new());
    let at = chrono::Utc::now();
    store
        .append(&[
            TraceRecord {
                thread_id: thread_id.clone(),
                run_id: None,
                seq: 1,
                at,
                event: TraceEvent::SuperstepStarted {
                    superstep: 1,
                    vanguard: vec![NodeId::new("n0")],
                },
            },
            TraceRecord {
                thread_id: thread_id.clone(),
                run_id: None,
                seq: 2,
                at,
                event: TraceEvent::RunFinished {
                    status: paladin_ports::output::trace_sink_port::RunFinishStatus::Halted,
                    total_supersteps: 1,
                    usage: TokenUsage::new(3, 4),
                    cost: None,
                    halt_reason: None,
                    duration_ms: 5,
                    trace_dropped_total: 0,
                },
            },
        ])
        .await
        .unwrap();
    store
}

/// G10, D-15, T-42-19: a replayed `RunFinished` for a run whose row is not terminal (a drained
/// worker's persisted halt) never ends the replay; once the row turns terminal the replay ends
/// on the row's own terminal event.
#[tokio::test]
async fn replay_skips_a_run_finished_while_the_row_is_not_terminal() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let waypoints: Arc<dyn WaypointPort> = Arc::new(InMemoryWaypointStore::new());
        let run = Run::new(
            RunId::new_v7(),
            ThreadId::new("t-replay-drained").unwrap(),
            AssistantRef {
                assistant_id: "a1".to_string(),
                version: 1,
            },
            serde_json::json!({}),
        )
        .with_status(RunStatus::Running);
        let run_id = run.run_id.clone();
        repository.insert(&run).await.unwrap();
        let traces = seed_halted_trace(&run.thread_id).await;

        let service = RunEventStreamService::new(
            Arc::new(RunEventBus::new()),
            repository.clone(),
            waypoints,
            Duration::from_millis(20),
        )
        .with_replay(traces as Arc<dyn RunTracePort>);
        let mut stream = service.stream(&run_id).await.unwrap();

        let first = stream.next().await.expect("the superstep record replays");
        assert_eq!(first.kind, RunStreamEventKind::Superstep);
        assert!(
            tokio::time::timeout(Duration::from_millis(300), stream.next())
                .await
                .is_err(),
            "a persisted RunFinished must not end the replay while the row is Running"
        );

        repository
            .update_status(
                &run_id,
                RunStatus::Running,
                RunStatus::Cancelled,
                chrono::Utc::now(),
            )
            .await
            .unwrap();
        let terminal = stream
            .next()
            .await
            .expect("the replay ends once the row is terminal");
        assert_eq!(terminal.kind, RunStreamEventKind::Done);
        assert_eq!(terminal.payload["status"], "cancelled");
        assert!(
            stream.next().await.is_none(),
            "one terminal event ends the stream"
        );
    })
    .await
    .expect("replay_skips_a_run_finished_while_the_row_is_not_terminal timed out");
}

/// G10: when a replayed `RunFinished` maps and the row is terminal, the emitted event takes
/// its `status` and `halt_reason` from the row (a pre-phase trace for a caller-cancelled run,
/// which stored `halted`, converges on `cancelled`; a row carrying a reason adds it) while the
/// record's own `usage` and `trace_seq` survive.
#[tokio::test]
async fn replay_trusts_the_row_over_the_recorded_status() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let waypoints: Arc<dyn WaypointPort> = Arc::new(InMemoryWaypointStore::new());
        for (row_status, row_reason, want_reason) in [
            (RunStatus::Cancelled, None, None),
            (
                RunStatus::Halted,
                Some(HaltReason::LedgerUnavailable),
                Some(serde_json::json!({ "reason": "ledger_unavailable" })),
            ),
            (RunStatus::Halted, None, None),
        ] {
            let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
            let mut run = Run::new(
                RunId::new_v7(),
                ThreadId::new(format!("t-replay-row-{}", RunId::new_v7())).unwrap(),
                AssistantRef {
                    assistant_id: "a1".to_string(),
                    version: 1,
                },
                serde_json::json!({}),
            )
            .with_status(row_status);
            run.halt_reason = row_reason;
            let run_id = run.run_id.clone();
            repository.insert(&run).await.unwrap();
            let traces = seed_halted_trace(&run.thread_id).await;

            let service = RunEventStreamService::new(
                Arc::new(RunEventBus::new()),
                repository,
                waypoints.clone(),
                Duration::from_millis(20),
            )
            .with_replay(traces as Arc<dyn RunTracePort>);
            let mut stream = service.stream(&run_id).await.unwrap();
            let mut terminal = None;
            while let Some(event) = stream.next().await {
                if event.kind == RunStreamEventKind::Done {
                    terminal = Some(event);
                    break;
                }
            }
            let terminal = terminal.expect("the replay ends with done");
            assert_eq!(
                terminal.payload["status"],
                serde_json::json!(row_status.to_string()),
                "status comes from the row"
            );
            assert_eq!(terminal.payload.get("halt_reason").cloned(), want_reason);
            assert_eq!(
                terminal.payload["trace_seq"], 2,
                "the record's trace_seq is kept"
            );
            assert_eq!(
                terminal.payload["usage"]["prompt_tokens"], 3,
                "the record's usage is kept"
            );
        }
    })
    .await
    .expect("replay_trusts_the_row_over_the_recorded_status timed out");
}

/// A run record carrying `run_id` for the shared-thread replay tests (WR-2).
fn run_stamped(thread_id: &ThreadId, run_id: &RunId, seq: u64, event: TraceEvent) -> TraceRecord {
    TraceRecord {
        thread_id: thread_id.clone(),
        run_id: Some(run_id.clone()),
        seq,
        at: chrono::Utc::now(),
        event,
    }
}

fn superstep_event(superstep: u64) -> TraceEvent {
    TraceEvent::SuperstepStarted {
        superstep,
        vanguard: vec![NodeId::new("n0")],
    }
}

fn finished_event(
    status: paladin_ports::output::trace_sink_port::RunFinishStatus,
    halt_reason: Option<HaltReason>,
) -> TraceEvent {
    TraceEvent::RunFinished {
        status,
        total_supersteps: 1,
        usage: TokenUsage::new(3, 4),
        cost: None,
        halt_reason,
        duration_ms: 5,
        trace_dropped_total: 0,
    }
}

/// Replay `run_id` over `traces` and collect every event until the stream ends.
async fn collect_replay(
    run: &Run,
    repository: Arc<dyn RunRepositoryPort>,
    traces: Arc<InMemoryRunTraceStore>,
) -> Vec<RunStreamEvent> {
    let waypoints: Arc<dyn WaypointPort> = Arc::new(InMemoryWaypointStore::new());
    let service = RunEventStreamService::new(
        Arc::new(RunEventBus::new()),
        repository,
        waypoints,
        Duration::from_millis(20),
    )
    .with_replay(traces as Arc<dyn RunTracePort>);
    let mut stream = service.stream(&run.run_id).await.unwrap();
    let mut events = Vec::new();
    while let Some(event) = stream.next().await {
        events.push(event);
    }
    events
}

/// WR-2 (42-REVIEW): replay reads the whole THREAD, and the documented recovery path is a NEW
/// run on the SAME thread (a fork). A prior run's `RunFinished` must never end the later run's
/// replay: records stamped with another run's id are skipped, so the replay shows only this
/// run's events and ends on this run's own terminal record.
#[tokio::test]
async fn replay_ignores_another_runs_records_on_a_shared_thread() {
    use paladin_ports::output::trace_sink_port::RunFinishStatus;
    tokio::time::timeout(Duration::from_secs(10), async {
        let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let thread = ThreadId::new("t-replay-shared-thread").unwrap();
        let assistant = AssistantRef {
            assistant_id: "a1".to_string(),
            version: 1,
        };
        let first = Run::new(
            RunId::new_v7(),
            thread.clone(),
            assistant.clone(),
            serde_json::json!({}),
        )
        .with_status(RunStatus::Halted);
        let second = Run::new(
            RunId::new_v7(),
            thread.clone(),
            assistant,
            serde_json::json!({}),
        )
        .with_status(RunStatus::Completed);
        repository.insert(&first).await.unwrap();
        repository.insert(&second).await.unwrap();

        let traces = Arc::new(InMemoryRunTraceStore::new());
        traces
            .append(&[
                run_stamped(&thread, &first.run_id, 1, superstep_event(1)),
                run_stamped(
                    &thread,
                    &first.run_id,
                    2,
                    finished_event(RunFinishStatus::Halted, Some(HaltReason::LedgerUnavailable)),
                ),
                run_stamped(&thread, &second.run_id, 3, superstep_event(2)),
                run_stamped(&thread, &second.run_id, 4, superstep_event(3)),
                run_stamped(
                    &thread,
                    &second.run_id,
                    5,
                    finished_event(RunFinishStatus::Completed, None),
                ),
            ])
            .await
            .unwrap();

        let events = collect_replay(&second, repository, traces).await;
        let kinds: Vec<_> = events.iter().map(|e| e.kind).collect();
        assert_eq!(
            kinds,
            vec![
                RunStreamEventKind::Superstep,
                RunStreamEventKind::Superstep,
                RunStreamEventKind::Done,
            ],
            "the second run replays only its own two supersteps and its own terminal event"
        );
        let done = events.last().expect("a terminal event");
        assert_eq!(done.payload["status"], "completed");
        assert_eq!(
            done.payload["trace_seq"], 5,
            "the terminal event is the second run's own record, not the first run's"
        );
        assert!(done.payload.get("halt_reason").is_none());
    })
    .await
    .expect("replay_ignores_another_runs_records_on_a_shared_thread timed out");
}

/// WR-2 (42-REVIEW): a worker that drained on shutdown persisted a reasonless
/// `RunFinished { Halted }` for a run that was then requeued and later completed. Once the row
/// is `Completed`, that stale halted record must not end the replay early.
#[tokio::test]
async fn replay_skips_a_drained_halted_record_when_the_run_later_completed() {
    use paladin_ports::output::trace_sink_port::RunFinishStatus;
    tokio::time::timeout(Duration::from_secs(10), async {
        let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let run = Run::new(
            RunId::new_v7(),
            ThreadId::new("t-replay-drained-then-done").unwrap(),
            AssistantRef {
                assistant_id: "a1".to_string(),
                version: 1,
            },
            serde_json::json!({}),
        )
        .with_status(RunStatus::Completed);
        repository.insert(&run).await.unwrap();

        let traces = Arc::new(InMemoryRunTraceStore::new());
        traces
            .append(&[
                run_stamped(&run.thread_id, &run.run_id, 1, superstep_event(1)),
                run_stamped(
                    &run.thread_id,
                    &run.run_id,
                    2,
                    finished_event(RunFinishStatus::Halted, None),
                ),
                run_stamped(&run.thread_id, &run.run_id, 3, superstep_event(2)),
                run_stamped(
                    &run.thread_id,
                    &run.run_id,
                    4,
                    finished_event(RunFinishStatus::Completed, None),
                ),
            ])
            .await
            .unwrap();

        let events = collect_replay(&run, repository, traces).await;
        let kinds: Vec<_> = events.iter().map(|e| e.kind).collect();
        assert_eq!(
            kinds,
            vec![
                RunStreamEventKind::Superstep,
                RunStreamEventKind::Superstep,
                RunStreamEventKind::Done,
            ],
            "the drained halt record is skipped and the replay runs to the real terminal event"
        );
        let done = events.last().expect("a terminal event");
        assert_eq!(done.payload["status"], "completed");
        assert_eq!(done.payload["trace_seq"], 4);
    })
    .await
    .expect("replay_skips_a_drained_halted_record_when_the_run_later_completed timed out");
}

/// Seed a thread with a stale drained `Halted` record at `stale_seq`, `later` further superstep
/// records behind it, and the run's own genuine terminal record last; replay the run (whose row
/// is `row_status`) and return every event.
async fn replay_after_a_stale_halted_record(
    thread: &str,
    row_status: RunStatus,
    row_halt_reason: Option<HaltReason>,
    prefix_supersteps: u64,
    own_terminal: TraceEvent,
) -> Vec<RunStreamEvent> {
    use paladin_ports::output::trace_sink_port::RunFinishStatus;
    let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
    let mut run = Run::new(
        RunId::new_v7(),
        ThreadId::new(thread).unwrap(),
        AssistantRef {
            assistant_id: "a1".to_string(),
            version: 1,
        },
        serde_json::json!({}),
    )
    .with_status(row_status);
    run.halt_reason = row_halt_reason;
    repository.insert(&run).await.unwrap();

    let mut records = Vec::new();
    let mut seq = 0;
    for superstep in 1..=prefix_supersteps {
        seq += 1;
        records.push(run_stamped(
            &run.thread_id,
            &run.run_id,
            seq,
            superstep_event(superstep),
        ));
    }
    seq += 1;
    records.push(run_stamped(
        &run.thread_id,
        &run.run_id,
        seq,
        finished_event(RunFinishStatus::Halted, None),
    ));
    seq += 1;
    records.push(run_stamped(
        &run.thread_id,
        &run.run_id,
        seq,
        superstep_event(prefix_supersteps + 1),
    ));
    seq += 1;
    records.push(run_stamped(&run.thread_id, &run.run_id, seq, own_terminal));

    let traces = Arc::new(InMemoryRunTraceStore::new());
    traces.append(&records).await.unwrap();
    collect_replay(&run, repository, traces).await
}

/// WR-5 (42-REVIEW): a drained dispatch's reasonless `Halted` record must not end the replay of a
/// run that was requeued and then ended `Halted` (a spend halt) -- the row status cannot tell the
/// stale record from the genuine one, so a later record of the same run decides.
#[tokio::test]
async fn replay_skips_a_drained_halted_record_when_the_run_later_halted_on_spend() {
    use paladin_ports::output::trace_sink_port::RunFinishStatus;
    tokio::time::timeout(Duration::from_secs(10), async {
        let events = replay_after_a_stale_halted_record(
            "t-replay-drained-then-halted",
            RunStatus::Halted,
            Some(HaltReason::LedgerUnavailable),
            1,
            finished_event(RunFinishStatus::Halted, Some(HaltReason::LedgerUnavailable)),
        )
        .await;
        let kinds: Vec<_> = events.iter().map(|e| e.kind).collect();
        assert_eq!(
            kinds,
            vec![
                RunStreamEventKind::Superstep,
                RunStreamEventKind::Superstep,
                RunStreamEventKind::Done,
            ],
            "the stale halt record is skipped and the replay reaches the final record"
        );
        let done = events.last().expect("a terminal event");
        assert_eq!(done.payload["status"], "halted");
        assert_eq!(
            done.payload["trace_seq"], 4,
            "the run's own final record ends it"
        );
        assert_eq!(done.payload["halt_reason"]["reason"], "ledger_unavailable");
    })
    .await
    .expect("replay_skips_a_drained_halted_record_when_the_run_later_halted_on_spend timed out");
}

/// WR-5 (42-REVIEW): the same stale drained record when the requeued run ends `Cancelled`.
#[tokio::test]
async fn replay_skips_a_drained_halted_record_when_the_run_later_cancelled() {
    use paladin_ports::output::trace_sink_port::RunFinishStatus;
    tokio::time::timeout(Duration::from_secs(10), async {
        let events = replay_after_a_stale_halted_record(
            "t-replay-drained-then-cancelled",
            RunStatus::Cancelled,
            None,
            1,
            finished_event(RunFinishStatus::Cancelled, None),
        )
        .await;
        let kinds: Vec<_> = events.iter().map(|e| e.kind).collect();
        assert_eq!(
            kinds,
            vec![
                RunStreamEventKind::Superstep,
                RunStreamEventKind::Superstep,
                RunStreamEventKind::Done,
            ],
        );
        let done = events.last().expect("a terminal event");
        assert_eq!(done.payload["status"], "cancelled");
        assert_eq!(done.payload["trace_seq"], 4);
    })
    .await
    .expect("replay_skips_a_drained_halted_record_when_the_run_later_cancelled timed out");
}

/// WR-5 (42-REVIEW): a genuine reasonless `Halted` record that really is the last record of the
/// run (a token halt with a persisted cancel flag, row `Cancelled`) still ends the replay, so the
/// later-record rule does not swallow a real end.
#[tokio::test]
async fn replay_keeps_a_last_reasonless_halted_record_beside_a_cancelled_row() {
    use paladin_ports::output::trace_sink_port::RunFinishStatus;
    tokio::time::timeout(Duration::from_secs(10), async {
        let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let run = Run::new(
            RunId::new_v7(),
            ThreadId::new("t-replay-genuine-halted-cancelled").unwrap(),
            AssistantRef {
                assistant_id: "a1".to_string(),
                version: 1,
            },
            serde_json::json!({}),
        )
        .with_status(RunStatus::Cancelled);
        repository.insert(&run).await.unwrap();
        let traces = Arc::new(InMemoryRunTraceStore::new());
        traces
            .append(&[
                run_stamped(&run.thread_id, &run.run_id, 1, superstep_event(1)),
                run_stamped(
                    &run.thread_id,
                    &run.run_id,
                    2,
                    finished_event(RunFinishStatus::Halted, None),
                ),
            ])
            .await
            .unwrap();
        let events = collect_replay(&run, repository, traces).await;
        let done = events.last().expect("a terminal event");
        assert_eq!(done.kind, RunStreamEventKind::Done);
        assert_eq!(done.payload["status"], "cancelled");
        assert_eq!(done.payload["trace_seq"], 2, "the last record is the end");
        assert_eq!(events.len(), 2);
    })
    .await
    .expect("replay_keeps_a_last_reasonless_halted_record_beside_a_cancelled_row timed out");
}

/// WR-5 (42-REVIEW): the stale record sits at the end of the first replay page, so the later
/// same-run records are only found by reading the next page.
#[tokio::test]
async fn replay_skips_a_stale_halted_record_found_at_a_page_boundary() {
    use paladin_ports::output::trace_sink_port::RunFinishStatus;
    tokio::time::timeout(Duration::from_secs(10), async {
        // 255 supersteps + the stale record fill the 256-record first page exactly.
        let events = replay_after_a_stale_halted_record(
            "t-replay-stale-at-page-end",
            RunStatus::Halted,
            Some(HaltReason::LedgerUnavailable),
            255,
            finished_event(RunFinishStatus::Halted, Some(HaltReason::LedgerUnavailable)),
        )
        .await;
        // 255 + 1 supersteps, then the run's own `done`; the stale record contributes nothing.
        assert_eq!(events.len(), 257);
        let done = events.last().expect("a terminal event");
        assert_eq!(done.kind, RunStreamEventKind::Done);
        assert_eq!(done.payload["trace_seq"], 258);
        assert_eq!(done.payload["halt_reason"]["reason"], "ledger_unavailable");
    })
    .await
    .expect("replay_skips_a_stale_halted_record_found_at_a_page_boundary timed out");
}

// --- Cancel and drain: the terminal event says what the row says (PLAT-09, D-05, D-14, D-15) ----

/// D-05 assumption-delta invariant: every halt cause -- an allowance-exhausted halt, a
/// ledger-unavailable halt, a caller cancel through the dispatching instance and a caller cancel
/// through another instance -- is driven through a real worker pool, and for each one the run
/// row's status, the live `done` status, the degraded `done` status and the replay `done` status
/// are the same string. A spend halt reads `halted`; a caller cancel reads `cancelled`.
#[tokio::test(flavor = "multi_thread")]
async fn every_halt_cause_maps_to_one_status_on_every_leg() {
    tokio::time::timeout(Duration::from_secs(90), async {
        let scripts = [
            ("allowance_exhausted", HaltScript::Spend, "halted"),
            (
                "ledger_unavailable",
                HaltScript::Guard(HaltReason::LedgerUnavailable),
                "halted",
            ),
            (
                "same_instance_cancel",
                HaltScript::CancelSameInstance,
                "cancelled",
            ),
            (
                "cross_instance_cancel",
                HaltScript::CancelCrossInstance,
                "cancelled",
            ),
        ];
        for (label, script, expected) in scripts {
            let rig = halted_run(script).await;
            let row = rig.repository.get(&rig.run_id).await.unwrap().unwrap();
            assert_eq!(row.status.as_str(), expected, "{label}: the row status");

            let (live_kind, live) = rig.live_terminal.clone();
            let (degraded_kind, degraded) = degraded_terminal(&rig).await;
            let (replay_kind, replay) = replay_terminal(&rig).await;
            for (leg, kind, payload) in [
                ("live", live_kind, &live),
                ("degraded", degraded_kind, &degraded),
                ("replay", replay_kind, &replay),
            ] {
                assert_eq!(
                    kind,
                    RunStreamEventKind::Done,
                    "{label}/{leg}: a halt or a cancel is a done, never an error"
                );
                assert_eq!(
                    payload["status"],
                    row.status.as_str(),
                    "{label}/{leg}: the done status must equal the row status: {payload}"
                );
            }
            if expected == "cancelled" {
                assert!(
                    live.get("halt_reason").is_none(),
                    "{label}: a caller cancel names no Treasurer reason: {live}"
                );
            }
            rig.release();
        }
    })
    .await
    .expect("every_halt_cause_maps_to_one_status_on_every_leg timed out");
}

/// D-15, G1c: a worker drain streams no terminal event. The shutdown halts the run at a
/// boundary; the row stays `Running`, the message is requeued, and no subscriber sees a `done`
/// (or `error`) for a run that is still running.
#[tokio::test(flavor = "multi_thread")]
async fn drain_streams_no_done_and_leaves_the_run_running() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
        let store = Arc::new(InMemoryWaypointStore::new());
        let bus = Arc::new(RunEventBus::new());
        let graph = build_chain_graph(6, Duration::from_millis(200));
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("drain-chain", graph));

        let coordinator = ShutdownCoordinator::new();
        let factory_store = store.clone();
        let engine_factory: Arc<
            dyn Fn(tokio_util::sync::CancellationToken) -> WarEngine<InMemoryWaypointStore>
                + Send
                + Sync,
        > = Arc::new(move |token| {
            WarEngine::new(Arc::new(UnusedPaladinPort), factory_store.clone())
                .with_cancellation_token(token)
        });
        let pool = RunWorkerPool::new(
            Arc::new(WarEngine::new(Arc::new(UnusedPaladinPort), store.clone())),
            store.clone(),
            repository.clone(),
            queue.clone(),
            resolver,
            Duration::from_secs(30),
        )
        .with_engine_factory(engine_factory)
        .with_event_bus(bus.clone())
        .with_shutdown_coordinator(coordinator.clone());

        let (run_id, thread_id) = submit(&repository, &queue, "drain-chain").await;
        bus.bind(thread_id, run_id.clone()).await;
        let mut rx = bus.subscribe(&run_id).await.expect("bus must be bound");
        let run_task = tokio::spawn(async move { pool.run_once().await });

        // Wait until the run is visibly going, then drain the worker mid-run.
        loop {
            match rx.recv().await {
                Ok(event) if event.kind == RunStreamEventKind::Superstep => break,
                Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => {
                    panic!("the run ended before the drain started")
                }
            }
        }
        let outcome = coordinator.cancel_and_wait(Duration::from_secs(10)).await;
        assert!(outcome.drained(), "the in-flight run must drain in grace");
        assert!(run_task.await.unwrap().unwrap());

        // Everything the run published is already on the channel (or the channel closed on
        // unbind); read it out under a bounded wait and assert nothing terminal is there.
        let mut terminal = Vec::new();
        loop {
            match tokio::time::timeout(Duration::from_millis(500), rx.recv()).await {
                Ok(Ok(event)) => {
                    if matches!(
                        event.kind,
                        RunStreamEventKind::Done | RunStreamEventKind::Error
                    ) {
                        terminal.push(event.payload);
                    }
                }
                Ok(Err(broadcast::error::RecvError::Lagged(_))) => continue,
                Ok(Err(broadcast::error::RecvError::Closed)) | Err(_) => break,
            }
        }
        assert!(
            terminal.is_empty(),
            "a drain is not a finish: no terminal event may reach a subscriber, got {terminal:?}"
        );

        let row = repository.get(&run_id).await.unwrap().unwrap();
        assert_eq!(
            row.status,
            RunStatus::Running,
            "a drained run stays Running -- it is a redelivery point, not a finish"
        );
        assert_eq!(queue.depth().await.unwrap(), 1, "the message is requeued");
    })
    .await
    .expect("drain_streams_no_done_and_leaves_the_run_running timed out");
}
