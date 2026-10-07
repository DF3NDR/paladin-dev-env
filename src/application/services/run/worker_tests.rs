//! Kill-mid-run redelivery, `AwaitingInput` ack, resume-with-pending
//! responses, heartbeat cadence and shutdown-drain -- the InMemory twin of
//! PRD 06 acceptance 2 (27-04 Task 2, D-51). Every assertion here is
//! Tier 1: InMemory queue, InMemory repository, `InMemoryWaypointStore`, no
//! Docker. The Redis twin is 27-03's CI job.
//!
//! `CountingFunctionNode` (`paladin_battalion::engine::test_support`) is
//! `pub(crate)` to `paladin-battalion` and unreachable from this facade
//! crate, so this module defines its own minimal `StateNode` doubles
//! (`DelayedCountingNode`), mirroring `tracer_e2e.rs`'s own
//! `AlwaysFailingNode` precedent of a small, local test double rather than
//! reaching for a crate-private one.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::Notify;

use paladin_battalion::engine::shutdown::ShutdownCoordinator;
use paladin_battalion::engine::{
    EdgeSpec, EngineLimits, NodeContext, NodeSpec, StateNode, StateNodeError, WarEngine, WarGraph,
};
use paladin_core::platform::container::allowance::HaltReason;
use paladin_core::platform::container::battlefield::{Battlefield, BattlefieldSchema, StateDelta};
use paladin_core::platform::container::directive::Directive;
use paladin_core::platform::container::execution_result::PaladinResult;
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::parley::ParleyResponse;
use paladin_core::platform::container::run::{
    AssistantRef, Run, RunEventKind, RunId, RunStatus, RunStreamEventKind, RunStreamMode,
    WebhookSpec,
};
use paladin_core::platform::container::token_usage::TokenUsage;
use paladin_core::platform::container::trace::{RunFinishStatus, TraceEvent};
use paladin_core::platform::container::waypoint::{
    NodeId, ThreadId, Waypoint, WaypointId, WaypointStatus,
};
use paladin_core::platform::container::webhook::{
    WebhookAttemptResult, WebhookDelivery, WebhookDeliveryId,
};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinStream};
use paladin_ports::output::run_queue_port::{QueuedRun, RunQueuePort};
use paladin_ports::output::run_repository_port::{
    RunOutcomeRecord, RunPage, RunQuery, RunRepositoryError, RunRepositoryPort,
};
use paladin_ports::output::run_trace_port::RunTracePort;
use paladin_ports::output::spend_guard::{SpendDecision, SpendGuard};
use paladin_ports::output::waypoint_port::{
    ThreadSummary, WaypointError, WaypointPort, WaypointSummary,
};
use paladin_ports::output::webhook_delivery_port::{
    WebhookDeliveryPage, WebhookDeliveryRepositoryError, WebhookDeliveryRepositoryPort,
};
use paladin_storage::run::in_memory::InMemoryRunRepository;
use paladin_storage::run_queue::in_memory::InMemoryRunQueue;
use paladin_storage::run_trace::in_memory::InMemoryRunTraceStore;
use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;
use paladin_storage::webhook::in_memory::InMemoryWebhookDeliveryRepository;

use super::resolver::{AssistantResolver, CodeWorkflowResolver};
use super::worker::{LeaseHeartbeat, RunWorkerOptions, RunWorkerPool};

/// A [`PaladinPort`] that must never be called -- every graph in this
/// module is Function/Gate-only, mirroring the `UnusedPaladinPort`
/// precedent (`src/config/engine.rs`, `src/bin/paladin-server.rs`).
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

/// A [`StateNode`] that counts its own executions and, if `delay` is
/// non-zero, sleeps that long before returning -- the instrument for both
/// "no node re-executes beyond the interrupted superstep" and a
/// deliberately slow superstep (heartbeat / shutdown-drain tests).
struct DelayedCountingNode {
    run_count: Arc<AtomicUsize>,
    delay: Duration,
}

impl DelayedCountingNode {
    fn new(delay: Duration) -> (Arc<Self>, Arc<AtomicUsize>) {
        let run_count = Arc::new(AtomicUsize::new(0));
        (
            Arc::new(Self {
                run_count: run_count.clone(),
                delay,
            }),
            run_count,
        )
    }
}

#[async_trait]
impl StateNode for DelayedCountingNode {
    async fn run(
        &self,
        _state: &Battlefield,
        _ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        if !self.delay.is_zero() {
            tokio::time::sleep(self.delay).await;
        }
        self.run_count.fetch_add(1, Ordering::SeqCst);
        Ok(StateDelta::new().into())
    }
}

/// A [`WaypointPort`] wrapper around an [`InMemoryWaypointStore`] that, the
/// first time it saves a Waypoint whose `superstep` equals `pause_after`,
/// performs the real save (so the state is durably persisted first), fires
/// a one-shot `Notify`, then parks forever.
///
/// This makes "kill worker A right after superstep N persists" exact rather
/// than racy: the wrapped `save` call is what the engine's own superstep
/// loop is awaiting when the pause fires, so NO subsequent code path
/// (including spawning the next superstep's node tasks) has run yet when
/// the test aborts the outer future -- there is no stray node task to
/// reason about.
struct PausingAfterSuperstep {
    inner: Arc<InMemoryWaypointStore>,
    pause_after: u64,
    paused: Arc<Notify>,
}

impl PausingAfterSuperstep {
    fn new(inner: Arc<InMemoryWaypointStore>, pause_after: u64) -> (Self, Arc<Notify>) {
        let paused = Arc::new(Notify::new());
        (
            Self {
                inner,
                pause_after,
                paused: paused.clone(),
            },
            paused,
        )
    }
}

#[async_trait]
impl WaypointPort for PausingAfterSuperstep {
    async fn save(&self, wp: &Waypoint) -> Result<(), WaypointError> {
        self.inner.save(wp).await?;
        if wp.superstep == self.pause_after {
            self.paused.notify_one();
            std::future::pending::<()>().await;
        }
        Ok(())
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
        before: Option<chrono::DateTime<chrono::Utc>>,
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
        self.inner.delete_waypoint(thread, id).await
    }
}

/// Build a linear `count` node chain (`n0 -> n1 -> ... -> n{count-1}`) of
/// [`DelayedCountingNode`]s, each with `delay`, returning the graph and each
/// node's own run-count handle in chain order.
fn build_chain_graph(count: usize, delay: Duration) -> (Arc<WarGraph>, Vec<Arc<AtomicUsize>>) {
    let mut graph = WarGraph::new(BattlefieldSchema::new(vec![]), EngineLimits::default());
    let mut counters = Vec::with_capacity(count);
    let mut ids = Vec::with_capacity(count);
    for i in 0..count {
        let id = NodeId::new(format!("n{i}"));
        let (node, counter) = DelayedCountingNode::new(delay);
        graph.add_node(id.clone(), NodeSpec::Function(node));
        ids.push(id);
        counters.push(counter);
    }
    for pair in ids.windows(2) {
        graph.add_edge(EdgeSpec {
            from: pair[0].clone(),
            to: pair[1].clone(),
            condition: None,
        });
    }
    graph.add_entry(ids[0].clone());
    (Arc::new(graph), counters)
}

/// Build a single-node graph whose only node is a
/// [`paladin_battalion::engine::graph::NodeSpec::Gate`] approval request --
/// the fixture for `awaiting_input_acks_queue` and
/// `resume_dispatch_uses_pending_responses`.
fn build_gate_graph() -> Arc<WarGraph> {
    use paladin_battalion::engine::InputMapping;
    use paladin_battalion::engine::graph::GateRequestTemplate;
    use paladin_core::platform::container::battlefield::{DispatchRule, FieldName, FieldSpec};
    use paladin_core::platform::container::parley::ParleyKind;

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

/// Insert a fresh `Queued` run against `graph_id` on a fresh thread, and
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

// --- worker_pool_lease_expiry_exactly_once ------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn worker_pool_lease_expiry_exactly_once() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
        let (graph, counters) = build_chain_graph(4, Duration::ZERO);
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("chain", graph));

        let store = Arc::new(InMemoryWaypointStore::new());
        let (pausing, paused) = PausingAfterSuperstep::new(store.clone(), 2);
        let pausing = Arc::new(pausing);

        let lease = Duration::from_millis(300);
        let engine_a = Arc::new(WarEngine::new(Arc::new(UnusedPaladinPort), pausing.clone()));
        let pool_a = Arc::new(RunWorkerPool::new(
            engine_a,
            pausing,
            repository.clone(),
            queue.clone(),
            resolver.clone(),
            lease,
        ));

        let engine_b = Arc::new(WarEngine::new(Arc::new(UnusedPaladinPort), store.clone()));
        let pool_b = Arc::new(RunWorkerPool::new(
            engine_b,
            store.clone(),
            repository.clone(),
            queue.clone(),
            resolver,
            lease,
        ));

        let (run_id, thread_id) = submit(&repository, &queue, "chain").await;

        let a_handle = tokio::spawn(async move { pool_a.run_once().await });
        paused.notified().await;
        // Superstep 2 is durably persisted at this point (observed via
        // `WaypointPort::history`) and worker A's task is parked inside its
        // own `save()` call -- no superstep 3 node has been spawned yet.
        let history_at_pause = store.history(&thread_id, None, None).await.unwrap();
        assert_eq!(
            history_at_pause.len(),
            2,
            "exactly two waypoints must be persisted before the kill"
        );
        a_handle.abort();
        let _ = a_handle.await;

        // Let the lease expire so worker B can dequeue the redelivered
        // message (real wall-clock wait -- the queue lease uses wall-clock
        // instants, D-51's own instruction against `tokio::time::pause`
        // here).
        tokio::time::sleep(lease + Duration::from_millis(100)).await;

        let processed = pool_b.run_once().await.unwrap();
        assert!(processed, "worker B must process the redelivered message");

        let run = repository.get(&run_id).await.unwrap().unwrap();
        assert_eq!(run.status, RunStatus::Completed);

        for (i, counter) in counters.iter().enumerate() {
            assert_eq!(
                counter.load(Ordering::SeqCst),
                1,
                "node n{i} must execute exactly once"
            );
        }

        let history = store.history(&thread_id, None, None).await.unwrap();
        assert_eq!(history.len(), 4, "exactly one waypoint per superstep");
        let mut supersteps: Vec<u64> = history.iter().map(|w| w.superstep).collect();
        supersteps.sort_unstable();
        assert_eq!(supersteps, vec![1, 2, 3, 4]);

        assert_eq!(queue.depth().await.unwrap(), 0);
    })
    .await
    .expect("worker_pool_lease_expiry_exactly_once must finish within 30s");
}

// --- halting transitions write the outcome before the status (G14) ------

/// A [`RunRepositoryPort`] double that delegates to an [`InMemoryRunRepository`] and logs the
/// order of the two writes a terminal transition makes: `record_outcome` and `update_status`
/// (with its target status). Every other method is a straight delegation.
struct OrderRecordingRepository {
    inner: InMemoryRunRepository,
    log: std::sync::Mutex<Vec<String>>,
}

impl OrderRecordingRepository {
    fn new() -> Self {
        Self {
            inner: InMemoryRunRepository::new(),
            log: std::sync::Mutex::new(Vec::new()),
        }
    }

    fn log(&self) -> Vec<String> {
        self.log.lock().unwrap().clone()
    }
}

#[async_trait]
impl RunRepositoryPort for OrderRecordingRepository {
    async fn insert(&self, run: &Run) -> Result<(), RunRepositoryError> {
        self.inner.insert(run).await
    }

    async fn get(&self, run_id: &RunId) -> Result<Option<Run>, RunRepositoryError> {
        self.inner.get(run_id).await
    }

    async fn update_status(
        &self,
        run_id: &RunId,
        from: RunStatus,
        to: RunStatus,
        at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), RunRepositoryError> {
        // Record the row's halt reason as seen at the instant the status flips, so the test can
        // prove a reader of the new status already finds the reason.
        let reason_present = self
            .inner
            .get(run_id)
            .await?
            .is_some_and(|run| run.halt_reason.is_some());
        self.log.lock().unwrap().push(format!(
            "update_status:{to}:reason_present={reason_present}"
        ));
        self.inner.update_status(run_id, from, to, at).await
    }

    async fn record_outcome(
        &self,
        run_id: &RunId,
        outcome: RunOutcomeRecord,
    ) -> Result<(), RunRepositoryError> {
        self.log.lock().unwrap().push("record_outcome".to_string());
        self.inner.record_outcome(run_id, outcome).await
    }

    async fn list(&self, query: RunQuery) -> Result<RunPage, RunRepositoryError> {
        self.inner.list(query).await
    }

    async fn active_run_for_thread(
        &self,
        thread_id: &ThreadId,
    ) -> Result<Option<Run>, RunRepositoryError> {
        self.inner.active_run_for_thread(thread_id).await
    }

    async fn request_cancel(&self, run_id: &RunId) -> Result<RunStatus, RunRepositoryError> {
        self.inner.request_cancel(run_id).await
    }

    async fn is_cancel_requested(&self, thread_id: &ThreadId) -> Result<bool, RunRepositoryError> {
        self.inner.is_cancel_requested(thread_id).await
    }

    async fn bump_attempt(&self, run_id: &RunId) -> Result<u32, RunRepositoryError> {
        self.inner.bump_attempt(run_id).await
    }

    async fn record_resume(
        &self,
        run_id: &RunId,
        responses: Vec<ParleyResponse>,
    ) -> Result<u32, RunRepositoryError> {
        self.inner.record_resume(run_id, responses).await
    }

    async fn clear_pending_responses(&self, run_id: &RunId) -> Result<(), RunRepositoryError> {
        self.inner.clear_pending_responses(run_id).await
    }
}

/// A [`SpendGuard`] that halts at its first consultation with a fixed reason.
struct AlwaysHalts(HaltReason);

#[async_trait]
impl SpendGuard for AlwaysHalts {
    async fn check(&self, _thread: &ThreadId) -> SpendDecision {
        SpendDecision::Halt(self.0.clone())
    }
}

/// G14 / PLAT-09 / T-42-13: a spend-halted run's worker write puts `record_outcome` (carrying
/// the reason) BEFORE the `Running -> Halted` flip, so the status flip already finds the reason
/// on the row; and a `Completed` transition keeps today's order (status first, outcome second).
#[tokio::test]
async fn halting_transition_records_the_outcome_before_the_status_flip() {
    let reason = HaltReason::LedgerUnavailable;

    // Halted: record_outcome first.
    let repository = Arc::new(OrderRecordingRepository::new());
    let repo_port: Arc<dyn RunRepositoryPort> = repository.clone();
    let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
    let store = Arc::new(InMemoryWaypointStore::new());
    let (graph, counters) = build_chain_graph(2, Duration::ZERO);
    let resolver: Arc<dyn AssistantResolver> =
        Arc::new(CodeWorkflowResolver::new().register("chain", graph));
    let engine = Arc::new(
        WarEngine::new(Arc::new(UnusedPaladinPort), store.clone())
            .with_spend_guard(Arc::new(AlwaysHalts(reason.clone()))),
    );
    let pool = RunWorkerPool::new(
        engine,
        store,
        repo_port.clone(),
        queue.clone(),
        resolver,
        Duration::from_secs(30),
    );
    let (run_id, _thread) = submit(&repo_port, &queue, "chain").await;

    assert!(pool.run_once().await.unwrap());

    let run = repo_port.get(&run_id).await.unwrap().unwrap();
    assert_eq!(run.status, RunStatus::Halted);
    assert_eq!(run.halt_reason, Some(reason));
    assert_eq!(run.error, None, "a halt is a resume point, not a failure");
    assert!(
        run.final_waypoint_id.is_some(),
        "the fork point is recorded"
    );
    assert_eq!(counters[0].load(Ordering::SeqCst), 0, "no node ran");
    assert_eq!(
        repository.log(),
        vec![
            "update_status:running:reason_present=false".to_string(),
            "record_outcome".to_string(),
            "update_status:halted:reason_present=true".to_string(),
        ],
        "the reason must be on the row before the status flips to halted"
    );

    // Completed: today's order is unchanged.
    let repository = Arc::new(OrderRecordingRepository::new());
    let repo_port: Arc<dyn RunRepositoryPort> = repository.clone();
    let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
    let store = Arc::new(InMemoryWaypointStore::new());
    let (graph, _counters) = build_chain_graph(1, Duration::ZERO);
    let resolver: Arc<dyn AssistantResolver> =
        Arc::new(CodeWorkflowResolver::new().register("chain", graph));
    let engine = Arc::new(WarEngine::new(Arc::new(UnusedPaladinPort), store.clone()));
    let pool = RunWorkerPool::new(
        engine,
        store,
        repo_port.clone(),
        queue.clone(),
        resolver,
        Duration::from_secs(30),
    );
    let (run_id, _thread) = submit(&repo_port, &queue, "chain").await;
    assert!(pool.run_once().await.unwrap());
    let run = repo_port.get(&run_id).await.unwrap().unwrap();
    assert_eq!(run.status, RunStatus::Completed);
    assert_eq!(run.halt_reason, None);
    let log = repository.log();
    assert_eq!(
        log,
        vec![
            "update_status:running:reason_present=false".to_string(),
            "update_status:completed:reason_present=false".to_string(),
            "record_outcome".to_string(),
        ],
        "completed keeps status-then-outcome order"
    );
}

// --- awaiting_input_acks_queue -------------------------------------------

#[tokio::test]
async fn awaiting_input_acks_queue() {
    let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
    let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
    let store = Arc::new(InMemoryWaypointStore::new());
    let resolver: Arc<dyn AssistantResolver> =
        Arc::new(CodeWorkflowResolver::new().register("gate", build_gate_graph()));
    let engine = Arc::new(WarEngine::new(Arc::new(UnusedPaladinPort), store.clone()));
    let worker = RunWorkerPool::new(
        engine,
        store,
        repository.clone(),
        queue.clone(),
        resolver,
        Duration::from_secs(30),
    );

    let (run_id, _thread_id) = submit(&repository, &queue, "gate").await;

    let processed = worker.run_once().await.unwrap();
    assert!(processed);

    let run = repository.get(&run_id).await.unwrap().unwrap();
    assert_eq!(run.status, RunStatus::AwaitingInput);
    assert_eq!(run.attempt, 1);
    assert_eq!(queue.depth().await.unwrap(), 0);
}

// --- resume_dispatch_uses_pending_responses ------------------------------

#[tokio::test]
async fn resume_dispatch_uses_pending_responses() {
    let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
    let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
    let store = Arc::new(InMemoryWaypointStore::new());
    let resolver: Arc<dyn AssistantResolver> =
        Arc::new(CodeWorkflowResolver::new().register("gate", build_gate_graph()));
    let engine = Arc::new(WarEngine::new(Arc::new(UnusedPaladinPort), store.clone()));
    let worker = RunWorkerPool::new(
        engine,
        store.clone(),
        repository.clone(),
        queue.clone(),
        resolver,
        Duration::from_secs(30),
    );

    let (run_id, thread_id) = submit(&repository, &queue, "gate").await;
    assert!(worker.run_once().await.unwrap());
    let suspended = repository.get(&run_id).await.unwrap().unwrap();
    assert_eq!(suspended.status, RunStatus::AwaitingInput);

    let latest = store.latest(&thread_id).await.unwrap().unwrap();
    let parleys = match latest.status {
        WaypointStatus::AwaitingInput { parleys, .. } => parleys,
        other => panic!("expected AwaitingInput, got {other:?}"),
    };
    let parley = parleys.first().expect("gate raised exactly one parley");

    let response = ParleyResponse {
        parley_id: parley.parley_id,
        kind: parley.kind.clone(),
        prompt: parley.prompt.clone(),
        value: serde_json::json!(true),
        responded_by: Some("tester".to_string()),
        responded_at: chrono::Utc::now(),
        defaulted: false,
    };
    let attempt = repository
        .record_resume(&run_id, vec![response])
        .await
        .unwrap();
    assert_eq!(attempt, 2);

    // The AwaitingInput ack already removed the original message (D-22);
    // a resume re-enqueues the SAME run_id (D-19).
    queue
        .enqueue(QueuedRun {
            run_id: run_id.clone(),
            thread_id: thread_id.clone(),
            attempt,
            enqueued_at: chrono::Utc::now(),
        })
        .await
        .unwrap();

    let processed = worker.run_once().await.unwrap();
    assert!(processed);

    let run = repository.get(&run_id).await.unwrap().unwrap();
    assert_eq!(run.status, RunStatus::Completed);
    assert!(run.pending_responses.is_empty());
    assert_eq!(run.attempt, 2);
}

// --- heartbeat_extends_at_lease_over_four --------------------------------

/// A [`RunQueuePort`] wrapping [`InMemoryRunQueue`] that records every
/// `extend_lease` call's timestamp -- shared by
/// `heartbeat_extends_at_lease_over_four` (a positive lease keeps
/// extending) and `lease_heartbeat_with_a_zero_lease_never_extends` (a
/// zero lease extends zero times, WR-04).
struct RecordingQueue {
    inner: InMemoryRunQueue,
    extend_calls: std::sync::Mutex<Vec<tokio::time::Instant>>,
}

impl RecordingQueue {
    fn new() -> Self {
        Self {
            inner: InMemoryRunQueue::new(),
            extend_calls: std::sync::Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl RunQueuePort for RecordingQueue {
    async fn enqueue(
        &self,
        run: QueuedRun,
    ) -> Result<(), paladin_ports::output::run_queue_port::QueueError> {
        self.inner.enqueue(run).await
    }

    async fn dequeue(
        &self,
        lease: Duration,
    ) -> Result<
        Option<paladin_ports::output::run_queue_port::LeasedRun>,
        paladin_ports::output::run_queue_port::QueueError,
    > {
        self.inner.dequeue(lease).await
    }

    async fn extend_lease(
        &self,
        token: &paladin_ports::output::run_queue_port::LeaseToken,
        lease: Duration,
    ) -> Result<(), paladin_ports::output::run_queue_port::QueueError> {
        self.extend_calls
            .lock()
            .unwrap()
            .push(tokio::time::Instant::now());
        self.inner.extend_lease(token, lease).await
    }

    async fn ack(
        &self,
        token: &paladin_ports::output::run_queue_port::LeaseToken,
    ) -> Result<(), paladin_ports::output::run_queue_port::QueueError> {
        self.inner.ack(token).await
    }

    async fn nack(
        &self,
        token: &paladin_ports::output::run_queue_port::LeaseToken,
        requeue_delay: Duration,
    ) -> Result<(), paladin_ports::output::run_queue_port::QueueError> {
        self.inner.nack(token, requeue_delay).await
    }

    async fn depth(&self) -> Result<u64, paladin_ports::output::run_queue_port::QueueError> {
        self.inner.depth().await
    }
}

/// (WR-04) Constructing a [`LeaseHeartbeat`] with a zero-duration lease
/// must start no background task: a zero interval would make
/// `tokio::time::sleep` resolve immediately, turning the extend-lease loop
/// into a CPU-bound spin. This is the tripwire for that guard -- letting
/// real time pass and asserting the recorded `extend_lease` count is
/// EXACTLY zero (not merely bounded), so any future regression to a
/// spinning heartbeat fails immediately.
#[tokio::test(flavor = "multi_thread")]
async fn lease_heartbeat_with_a_zero_lease_never_extends() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let recording = Arc::new(RecordingQueue::new());
        let queue: Arc<dyn RunQueuePort> = recording.clone();
        let token = paladin_ports::output::run_queue_port::LeaseToken::new("zero-lease-token");

        let heartbeat = LeaseHeartbeat::spawn(queue, token, Duration::ZERO);

        // Let enough real time pass that a spinning implementation would
        // have made many `extend_lease` calls by now.
        tokio::time::sleep(Duration::from_millis(200)).await;
        drop(heartbeat);

        let calls = recording.extend_calls.lock().unwrap();
        assert_eq!(
            calls.len(),
            0,
            "a zero-duration lease must start no heartbeat task at all, got {} extend_lease calls",
            calls.len()
        );
    })
    .await
    .expect("lease_heartbeat_with_a_zero_lease_never_extends must finish within 5s");
}

#[tokio::test(flavor = "multi_thread")]
async fn heartbeat_extends_at_lease_over_four() {
    tokio::time::timeout(Duration::from_secs(15), async {
        let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let recording = Arc::new(RecordingQueue::new());
        let queue: Arc<dyn RunQueuePort> = recording.clone();
        let store = Arc::new(InMemoryWaypointStore::new());
        let (graph, _counters) = build_chain_graph(1, Duration::from_secs(1));
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("slow", graph));
        let engine = Arc::new(WarEngine::new(Arc::new(UnusedPaladinPort), store.clone()));
        let worker = RunWorkerPool::new(
            engine,
            store,
            repository.clone(),
            queue.clone(),
            resolver,
            Duration::from_millis(400),
        );

        let (run_id, _thread_id) = submit(&repository, &queue, "slow").await;
        assert!(worker.run_once().await.unwrap());

        let run = repository.get(&run_id).await.unwrap().unwrap();
        assert_eq!(run.status, RunStatus::Completed);

        let calls = recording.extend_calls.lock().unwrap();
        assert!(
            calls.len() >= 8,
            "expected at least 8 lease extensions over a 1s run with a 400ms lease, got {}",
            calls.len()
        );
    })
    .await
    .expect("heartbeat_extends_at_lease_over_four must finish within 15s");
}

// --- shutdown_drains_in_flight_run ---------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn shutdown_drains_in_flight_run() {
    tokio::time::timeout(Duration::from_secs(15), async {
        let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
        let store = Arc::new(InMemoryWaypointStore::new());
        // Two supersteps: the first (500ms) is in flight when shutdown
        // fires; the engine observes cancellation at the boundary BEFORE
        // the second superstep starts, so the run ends `Halted` after
        // exactly one superstep, never running the second node.
        let (graph, counters) = build_chain_graph(2, Duration::from_millis(500));
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("slow-chain", graph));

        let coordinator = ShutdownCoordinator::new();
        let engine = Arc::new(
            WarEngine::new(Arc::new(UnusedPaladinPort), store.clone())
                .with_cancellation_token(coordinator.token()),
        );
        let pool = Arc::new(
            RunWorkerPool::new(
                engine,
                store,
                repository.clone(),
                queue.clone(),
                resolver,
                Duration::from_secs(30),
            )
            .with_shutdown_coordinator(coordinator.clone()),
        );

        let (run_id, _thread_id) = submit(&repository, &queue, "slow-chain").await;

        let handles = pool.spawn(RunWorkerOptions {
            concurrency: 1,
            lease: Duration::from_secs(30),
            min_probe_interval: Duration::from_millis(20),
        });

        // Let the worker dequeue and start the first (slow) superstep
        // before requesting shutdown.
        tokio::time::sleep(Duration::from_millis(100)).await;

        let outcome = coordinator.cancel_and_wait(Duration::from_secs(2)).await;
        assert!(
            outcome.drained(),
            "the in-flight run must drain within grace"
        );

        for handle in handles {
            // Every spawned task must have exited on its own by now.
            tokio::time::timeout(Duration::from_millis(500), handle)
                .await
                .expect("worker task must have exited after drain")
                .expect("worker task must not panic");
        }

        let run = repository.get(&run_id).await.unwrap().unwrap();
        assert_eq!(
            run.status,
            RunStatus::Running,
            "a shutdown-halted run stays Running -- it is a redelivery point, not a finish"
        );
        assert_eq!(
            counters[1].load(Ordering::SeqCst),
            0,
            "the second node must never run"
        );

        // The message was nacked with a zero delay: it must be visible
        // again immediately.
        assert_eq!(queue.depth().await.unwrap(), 1);
        let redelivered = queue
            .dequeue(Duration::from_secs(30))
            .await
            .unwrap()
            .expect("the message must be visible again after the shutdown nack");
        assert_eq!(redelivered.queued.run_id, run_id);
    })
    .await
    .expect("shutdown_drains_in_flight_run must finish within 15s");
}

// --- webhook delivery hook (27-13, D-40, PLAT-FR-14) --------------------

/// Insert a fresh `Queued` run carrying `webhook`, enqueue its pointer, and
/// return the run/thread ids -- mirrors `submit` but for the webhook-hook
/// tests.
async fn submit_with_webhook(
    repository: &Arc<dyn RunRepositoryPort>,
    queue: &Arc<dyn RunQueuePort>,
    assistant_id: &str,
    webhook: WebhookSpec,
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
    )
    .with_webhook(webhook);
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

/// A [`WebhookDeliveryRepositoryPort`] test double whose `enqueue` always
/// fails -- proves a delivery-repository error is logged and NEVER changes
/// the run's own status (prohibition P2).
struct AlwaysErrorWebhookDeliveries;

#[async_trait]
impl WebhookDeliveryRepositoryPort for AlwaysErrorWebhookDeliveries {
    async fn enqueue(
        &self,
        _delivery: WebhookDelivery,
    ) -> Result<(), WebhookDeliveryRepositoryError> {
        Err(WebhookDeliveryRepositoryError::Backend {
            source: "always fails".into(),
        })
    }

    async fn get(
        &self,
        delivery_id: &WebhookDeliveryId,
    ) -> Result<Option<WebhookDelivery>, WebhookDeliveryRepositoryError> {
        Err(WebhookDeliveryRepositoryError::NotFound {
            delivery_id: delivery_id.clone(),
        })
    }

    async fn claim_due(
        &self,
        _now: chrono::DateTime<chrono::Utc>,
        _limit: u32,
    ) -> Result<Vec<WebhookDelivery>, WebhookDeliveryRepositoryError> {
        Ok(vec![])
    }

    async fn record_attempt(
        &self,
        delivery_id: &WebhookDeliveryId,
        _result: WebhookAttemptResult,
    ) -> Result<(), WebhookDeliveryRepositoryError> {
        Err(WebhookDeliveryRepositoryError::NotFound {
            delivery_id: delivery_id.clone(),
        })
    }

    async fn list_for_run(
        &self,
        _run_id: &RunId,
        _limit: u32,
        _cursor: Option<WebhookDeliveryId>,
    ) -> Result<WebhookDeliveryPage, WebhookDeliveryRepositoryError> {
        Ok(WebhookDeliveryPage::default())
    }
}

#[tokio::test]
async fn webhook_delivery_enqueued_on_completed_event() {
    let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
    let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
    let store = Arc::new(InMemoryWaypointStore::new());
    let (graph, _counters) = build_chain_graph(1, Duration::ZERO);
    let resolver: Arc<dyn AssistantResolver> =
        Arc::new(CodeWorkflowResolver::new().register("chain-webhook", graph));
    let engine = Arc::new(WarEngine::new(Arc::new(UnusedPaladinPort), store.clone()));
    let deliveries: Arc<dyn WebhookDeliveryRepositoryPort> =
        Arc::new(InMemoryWebhookDeliveryRepository::new());
    let worker = RunWorkerPool::new(
        engine,
        store,
        repository.clone(),
        queue.clone(),
        resolver,
        Duration::from_secs(30),
    )
    .with_webhook_deliveries(Arc::clone(&deliveries));

    let webhook = WebhookSpec {
        url: "https://example.com/hook".to_string(),
        secret: None,
        events: vec![RunEventKind::Completed],
    };
    let (run_id, _thread_id) =
        submit_with_webhook(&repository, &queue, "chain-webhook", webhook).await;

    assert!(worker.run_once().await.unwrap());
    let run = repository.get(&run_id).await.unwrap().unwrap();
    assert_eq!(run.status, RunStatus::Completed);

    let page = deliveries.list_for_run(&run_id, 10, None).await.unwrap();
    assert_eq!(page.items.len(), 1, "exactly one delivery must be enqueued");
    assert!(matches!(page.items[0].event, RunEventKind::Completed));
    assert!(page.items[0].payload.contains(run_id.as_str()));
}

#[tokio::test]
async fn webhook_delivery_repository_error_never_affects_run_status() {
    let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
    let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
    let store = Arc::new(InMemoryWaypointStore::new());
    let (graph, _counters) = build_chain_graph(1, Duration::ZERO);
    let resolver: Arc<dyn AssistantResolver> =
        Arc::new(CodeWorkflowResolver::new().register("chain-webhook-err", graph));
    let engine = Arc::new(WarEngine::new(Arc::new(UnusedPaladinPort), store.clone()));
    let deliveries: Arc<dyn WebhookDeliveryRepositoryPort> = Arc::new(AlwaysErrorWebhookDeliveries);
    let worker = RunWorkerPool::new(
        engine,
        store,
        repository.clone(),
        queue.clone(),
        resolver,
        Duration::from_secs(30),
    )
    .with_webhook_deliveries(deliveries);

    let webhook = WebhookSpec {
        url: "https://example.com/hook".to_string(),
        secret: None,
        events: vec![RunEventKind::Completed],
    };
    let (run_id, _thread_id) =
        submit_with_webhook(&repository, &queue, "chain-webhook-err", webhook).await;

    assert!(worker.run_once().await.unwrap());
    let run = repository.get(&run_id).await.unwrap().unwrap();
    assert_eq!(
        run.status,
        RunStatus::Completed,
        "a webhook delivery repository error must never change the run's own status"
    );
}

// --- agent_kind_run_with_a_webhook_enqueues_a_delivery (PLAT-08) --------

/// An [`AssistantResolver`] resolving every id to a code-registered
/// `Runnable::Agent` -- the legacy dispatch path `run_once` routes to
/// [`super::worker::RunWorkerPool::run_agent`], which binds the event bus,
/// streams through the per-run trace dispatcher and enqueues its webhook
/// deliveries exactly as the graph path does.
struct AgentOnlyResolver;

#[async_trait]
impl AssistantResolver for AgentOnlyResolver {
    async fn resolve(
        &self,
        assistant_id: &str,
        version: Option<u32>,
    ) -> Result<super::resolver::ResolvedAssistant, super::resolver::ResolveError> {
        use paladin_core::base::entity::node::Node;
        use paladin_core::platform::container::paladin::PaladinData;

        Ok(super::resolver::ResolvedAssistant {
            reference: AssistantRef {
                assistant_id: assistant_id.to_string(),
                version: version.unwrap_or(1),
            },
            runnable: super::resolver::Runnable::Agent(Arc::new(Node::new(
                PaladinData::default(),
                Some(assistant_id.to_string()),
            ))),
            allowed_roles: vec![],
            source: paladin_core::platform::container::assistant::AssistantSource::Code,
        })
    }
}

/// A [`PaladinPort`] that always succeeds with a fixed output, standing in
/// for a real LLM call -- this test only cares about the webhook/event-bus
/// carve-out, not agent execution semantics.
struct AlwaysSucceedsPaladinPort;

#[async_trait]
impl PaladinPort for AlwaysSucceedsPaladinPort {
    async fn execute(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinResult, PaladinError> {
        Ok(PaladinResult {
            output: "agent completed".to_string(),
            usage: TokenUsage::new(11, 7),
            ..Default::default()
        })
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinStream, PaladinError> {
        unreachable!("this test never calls execute_stream")
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

/// PLAT-08, D-15: assistant-kind parity. An `Agent`-kind run carrying a
/// `webhook` spec subscribed to `completed` completes normally and enqueues
/// EXACTLY ONE `Pending` delivery through the same
/// `webhook_delivery_for_outcome` -> `enqueue` path the `Runnable::Workflow`
/// path uses (`webhook_delivery_enqueued_on_completed_event`, above),
/// strictly after the status write and ack. This is the WR-02 tripwire
/// inverted (ledger row 31): it used to assert zero deliveries.
#[tokio::test]
async fn agent_kind_run_with_a_webhook_enqueues_a_delivery() {
    let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
    let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
    let store = Arc::new(InMemoryWaypointStore::new());
    let resolver: Arc<dyn AssistantResolver> = Arc::new(AgentOnlyResolver);
    let engine = Arc::new(WarEngine::new(Arc::new(UnusedPaladinPort), store.clone()));
    let deliveries: Arc<dyn WebhookDeliveryRepositoryPort> =
        Arc::new(InMemoryWebhookDeliveryRepository::new());
    let worker = RunWorkerPool::new(
        engine,
        store,
        repository.clone(),
        queue.clone(),
        resolver,
        Duration::from_secs(30),
    )
    .with_paladin_port(Arc::new(AlwaysSucceedsPaladinPort))
    .with_webhook_deliveries(Arc::clone(&deliveries));

    let webhook = WebhookSpec {
        url: "https://example.com/hook".to_string(),
        secret: None,
        events: vec![RunEventKind::Completed],
    };
    let (run_id, _thread_id) =
        submit_with_webhook(&repository, &queue, "code-agent", webhook).await;

    assert!(worker.run_once().await.unwrap());

    let run = repository.get(&run_id).await.unwrap().unwrap();
    assert_eq!(run.status, RunStatus::Completed);

    let page = deliveries.list_for_run(&run_id, 10, None).await.unwrap();
    assert_eq!(
        page.items.len(),
        1,
        "an Agent-kind run must enqueue exactly one delivery"
    );
    let delivery = &page.items[0];
    assert!(matches!(
        delivery.status,
        paladin_core::platform::container::webhook::WebhookDeliveryStatus::Pending
    ));
    assert!(matches!(delivery.event, RunEventKind::Completed));

    let payload: serde_json::Value = serde_json::from_str(&delivery.payload).unwrap();
    assert_eq!(payload["run_id"], run_id.as_str());
    assert_eq!(payload["status"], "completed");
    assert_eq!(payload["event"], "completed");
    assert_eq!(payload["assistant"]["assistant_id"], "code-agent");
    assert_eq!(payload["assistant"]["version"], 1);
}

// --- 45-02 (PLAT-08): agent runs stream live through the one mapping ---

/// D-14, D-00e: a code-registered agent run streams `node_started`,
/// `node_finished` and exactly one `done` in `Live` mode, every one of them
/// produced by `RunEventBusSink` -> `map_trace_event` (never a hand-published
/// event), with the agent call's non-zero usage on both `node_finished` and
/// `done`.
///
/// `RunStarted` maps to no wire event (D-24, the `map_trace_event` table), so
/// D-16's "`RunStarted`-derived" evidence is asserted where it actually lands:
/// the first persisted `run_traces` row (seq 1) of a gapless 1..=4 sequence
/// ending in `RunFinished`; the first WIRE event is `node_started`.
#[tokio::test]
async fn agent_kind_run_streams_done_live() {
    use crate::application::services::run::events::RunEventBus;
    use crate::config::trace::TraceConfig;

    let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
    let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
    let store = Arc::new(InMemoryWaypointStore::new());
    let resolver: Arc<dyn AssistantResolver> = Arc::new(AgentOnlyResolver);
    let engine = Arc::new(WarEngine::new(Arc::new(UnusedPaladinPort), store.clone()));
    let bus = Arc::new(RunEventBus::new());
    let traces = Arc::new(InMemoryRunTraceStore::new());
    let worker = RunWorkerPool::new(
        engine,
        store,
        repository.clone(),
        queue.clone(),
        resolver,
        Duration::from_secs(30),
    )
    .with_paladin_port(Arc::new(AlwaysSucceedsPaladinPort))
    .with_event_bus(bus.clone())
    .with_trace_config(TraceConfig {
        log_sink: false,
        persist: true,
        ..TraceConfig::default()
    })
    .with_run_trace_port(traces.clone());

    let (run_id, thread_id) = submit(&repository, &queue, "code-agent").await;
    // Pre-bind so a subscriber can attach before `run_agent`'s own `bind`
    // re-affirms the SAME channel (`bind` is idempotent).
    bus.bind(thread_id.clone(), run_id.clone()).await;
    let mut rx = bus
        .subscribe(&run_id)
        .await
        .expect("the channel exists once bound");

    assert!(worker.run_once().await.unwrap());
    let run = repository.get(&run_id).await.unwrap().unwrap();
    assert_eq!(run.status, RunStatus::Completed);

    let mut events = Vec::new();
    while let Ok(event) = rx.try_recv() {
        events.push(event);
    }
    let kinds: Vec<RunStreamEventKind> = events.iter().map(|e| e.kind).collect();
    assert_eq!(
        kinds,
        vec![
            RunStreamEventKind::NodeStarted,
            RunStreamEventKind::NodeFinished,
            RunStreamEventKind::Done,
        ],
        "an agent run must stream node_started, node_finished, then exactly one done"
    );
    assert!(
        events.iter().all(|e| e.mode == RunStreamMode::Live),
        "every agent-run wire event must be Live"
    );

    let expected_usage = serde_json::to_value(TokenUsage::new(11, 7)).unwrap();
    assert_eq!(events[0].payload["node_id"], "code-agent");
    assert_eq!(events[1].payload["outcome"], "Succeeded");
    assert_eq!(events[1].payload["usage"], expected_usage);
    assert_eq!(events[2].payload["status"], "completed");
    assert_eq!(events[2].payload["usage"], expected_usage);
    assert!(
        events[2].payload["usage"]["total_tokens"].as_u64().unwrap() > 0,
        "the done event must carry the agent call's non-zero usage"
    );

    // The persisting sink drains on its own schedule: poll until the
    // terminal row lands (up to 2 s).
    let mut rows = Vec::new();
    for _ in 0..100 {
        rows = traces.read(&thread_id, 0, 100).await.unwrap();
        if rows
            .iter()
            .any(|r| matches!(r.event, TraceEvent::RunFinished { .. }))
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let seqs: Vec<u64> = rows.iter().map(|r| r.seq).collect();
    assert_eq!(
        seqs,
        vec![1, 2, 3, 4],
        "the persisted trace must be gapless"
    );
    match &rows[0].event {
        TraceEvent::RunStarted {
            run_id: traced_run,
            graph_fingerprint,
        } => {
            assert_eq!(traced_run.as_ref(), Some(&run_id));
            assert_eq!(graph_fingerprint, "agent");
        }
        other => panic!("seq 1 must be RunStarted, got {other:?}"),
    }
    assert!(matches!(rows[1].event, TraceEvent::NodeStarted { .. }));
    assert!(matches!(rows[2].event, TraceEvent::NodeFinished { .. }));
    assert!(matches!(
        rows[3].event,
        TraceEvent::RunFinished {
            status: RunFinishStatus::Completed,
            total_supersteps: 0,
            ..
        }
    ));
}

/// A [`PaladinPort`] whose every call fails, standing in for an LLM outage.
/// The error text is deliberately recognisable so a test can prove it never
/// reaches a wire event (T-45-08).
struct AlwaysFailsPaladinPort;

/// The failure text [`AlwaysFailsPaladinPort`] returns.
const AGENT_FAILURE_TEXT: &str = "llm exploded: sk-agent-secret-marker";

#[async_trait]
impl PaladinPort for AlwaysFailsPaladinPort {
    async fn execute(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinResult, PaladinError> {
        Err(PaladinError::ExecutionError(AGENT_FAILURE_TEXT.to_string()))
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinStream, PaladinError> {
        unreachable!("this test never calls execute_stream")
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

/// Build a pool over an [`AgentOnlyResolver`] with `port`, an event bus and
/// (optionally) a webhook delivery repository -- the shared harness of the
/// 45-02 agent-path failure tests.
#[allow(clippy::type_complexity)]
fn agent_pool(
    port: Arc<dyn PaladinPort>,
    deliveries: Option<Arc<dyn WebhookDeliveryRepositoryPort>>,
) -> (
    RunWorkerPool<InMemoryWaypointStore>,
    Arc<dyn RunRepositoryPort>,
    Arc<dyn RunQueuePort>,
    Arc<super::events::RunEventBus>,
) {
    let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
    let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
    let store = Arc::new(InMemoryWaypointStore::new());
    let resolver: Arc<dyn AssistantResolver> = Arc::new(AgentOnlyResolver);
    let engine = Arc::new(WarEngine::new(Arc::new(UnusedPaladinPort), store.clone()));
    let bus = Arc::new(super::events::RunEventBus::new());
    let mut pool = RunWorkerPool::new(
        engine,
        store,
        repository.clone(),
        queue.clone(),
        resolver,
        Duration::from_secs(30),
    )
    .with_paladin_port(port)
    .with_event_bus(bus.clone())
    .with_trace_config(crate::config::trace::TraceConfig {
        log_sink: false,
        ..crate::config::trace::TraceConfig::default()
    });
    if let Some(deliveries) = deliveries {
        pool = pool.with_webhook_deliveries(deliveries);
    }
    (pool, repository, queue, bus)
}

/// Drain every event currently buffered on `rx`.
fn drain_events(
    rx: &mut tokio::sync::broadcast::Receiver<
        paladin_core::platform::container::run::RunStreamEvent,
    >,
) -> Vec<paladin_core::platform::container::run::RunStreamEvent> {
    let mut events = Vec::new();
    while let Ok(event) = rx.try_recv() {
        events.push(event);
    }
    events
}

fn terminal_count(events: &[paladin_core::platform::container::run::RunStreamEvent]) -> usize {
    events
        .iter()
        .filter(|e| matches!(e.kind, RunStreamEventKind::Done | RunStreamEventKind::Error))
        .count()
}

/// D-15, T-45-08, T-45-10: every agent run ends with EXACTLY ONE terminal
/// wire event. A failure is one `error` (status `failed`, `message: null`,
/// exactly like a graph run's engine-emitted failure) -- never an `error`
/// plus a second publish, and never a `done`; a success is one `done`. The
/// failure text stays behind the tenant-scoped `GET /runs/{id}`.
#[tokio::test]
async fn agent_kind_run_emits_exactly_one_terminal_event() {
    // --- failure ---
    let (pool, repository, queue, bus) = agent_pool(Arc::new(AlwaysFailsPaladinPort), None);
    let (run_id, thread_id) = submit(&repository, &queue, "code-agent").await;
    bus.bind(thread_id.clone(), run_id.clone()).await;
    let mut rx = bus.subscribe(&run_id).await.expect("bound");

    assert!(pool.run_once().await.unwrap());
    let events = drain_events(&mut rx);
    assert_eq!(terminal_count(&events), 1, "got {events:?}");
    let terminal = events.last().expect("at least the terminal event");
    assert_eq!(terminal.kind, RunStreamEventKind::Error);
    assert_eq!(terminal.mode, RunStreamMode::Live);
    assert_eq!(terminal.payload["status"], "failed");
    assert!(
        terminal.payload["message"].is_null(),
        "the wire error carries no failure text (message: null)"
    );
    assert!(
        !events
            .iter()
            .any(|e| e.payload.to_string().contains("sk-agent-secret-marker")),
        "the failure text must never reach any wire event"
    );
    assert!(
        !events.iter().any(|e| e.kind == RunStreamEventKind::Done),
        "a failed agent run never emits done"
    );
    let failed = repository.get(&run_id).await.unwrap().unwrap();
    assert_eq!(failed.status, RunStatus::Failed);
    assert_eq!(
        failed.error.as_deref(),
        Some(format!("Execution error: {AGENT_FAILURE_TEXT}").as_str()),
        "the failure text stays readable through GET /runs/{{id}}"
    );

    // --- success ---
    let (pool, repository, queue, bus) = agent_pool(Arc::new(AlwaysSucceedsPaladinPort), None);
    let (run_id, thread_id) = submit(&repository, &queue, "code-agent").await;
    bus.bind(thread_id.clone(), run_id.clone()).await;
    let mut rx = bus.subscribe(&run_id).await.expect("bound");

    assert!(pool.run_once().await.unwrap());
    let events = drain_events(&mut rx);
    assert_eq!(terminal_count(&events), 1, "got {events:?}");
    assert_eq!(
        events.last().map(|e| e.kind),
        Some(RunStreamEventKind::Done),
        "a successful agent run ends with one done"
    );
}

/// PLAT-08, D-15: a failing agent run subscribed to `failed` enqueues
/// exactly one `Pending` `failed` delivery, after its status write and ack.
#[tokio::test]
async fn agent_kind_run_failure_enqueues_a_failed_delivery() {
    let deliveries: Arc<dyn WebhookDeliveryRepositoryPort> =
        Arc::new(InMemoryWebhookDeliveryRepository::new());
    let (pool, repository, queue, _bus) = agent_pool(
        Arc::new(AlwaysFailsPaladinPort),
        Some(Arc::clone(&deliveries)),
    );
    let webhook = WebhookSpec {
        url: "https://example.com/hook".to_string(),
        secret: None,
        events: vec![RunEventKind::Failed],
    };
    let (run_id, _thread_id) =
        submit_with_webhook(&repository, &queue, "code-agent", webhook).await;

    assert!(pool.run_once().await.unwrap());
    let run = repository.get(&run_id).await.unwrap().unwrap();
    assert_eq!(run.status, RunStatus::Failed);

    let page = deliveries.list_for_run(&run_id, 10, None).await.unwrap();
    assert_eq!(page.items.len(), 1, "exactly one delivery must be enqueued");
    let delivery = &page.items[0];
    assert!(matches!(delivery.event, RunEventKind::Failed));
    assert!(matches!(
        delivery.status,
        paladin_core::platform::container::webhook::WebhookDeliveryStatus::Pending
    ));
    let payload: serde_json::Value = serde_json::from_str(&delivery.payload).unwrap();
    assert_eq!(payload["status"], "failed");
    assert_eq!(payload["event"], "failed");
    assert!(
        !delivery.payload.contains("sk-agent-secret-marker"),
        "the failure text never rides in the webhook payload"
    );
}

/// P2 (T-45-11): a delivery-repository error on the FAILURE path is logged
/// and never changes the run's `Failed` status, nor leaves the bus bound.
#[tokio::test]
async fn failed_delivery_enqueue_error_never_changes_the_failed_status() {
    let deliveries: Arc<dyn WebhookDeliveryRepositoryPort> = Arc::new(AlwaysErrorWebhookDeliveries);
    let (pool, repository, queue, bus) =
        agent_pool(Arc::new(AlwaysFailsPaladinPort), Some(deliveries));
    let webhook = WebhookSpec {
        url: "https://example.com/hook".to_string(),
        secret: None,
        events: vec![RunEventKind::Failed],
    };
    let (run_id, _thread_id) =
        submit_with_webhook(&repository, &queue, "code-agent", webhook).await;

    assert!(pool.run_once().await.unwrap());
    let run = repository.get(&run_id).await.unwrap().unwrap();
    assert_eq!(
        run.status,
        RunStatus::Failed,
        "a webhook delivery repository error must never change the run's own status"
    );
    assert!(
        bus.subscribe(&run_id).await.is_none(),
        "the channel must be unbound once the run returns"
    );
}

// --- 28-06: per-run trace composition (Task 1) --------------------------

/// Build a `RunWorkerPool` over a fresh `InMemoryWaypointStore`/repository/
/// queue, wired with an `engine_factory` (required for per-run trace
/// composition, D-24's own documented limitation) and the given
/// `trace_config`/`event_bus`. Returns the pool plus the pieces a test
/// needs to submit a run and inspect its outcome.
#[allow(clippy::type_complexity)]
fn build_traced_pool(
    trace_config: crate::config::trace::TraceConfig,
    event_bus: Option<Arc<super::events::RunEventBus>>,
) -> (
    RunWorkerPool<InMemoryWaypointStore>,
    Arc<dyn RunRepositoryPort>,
    Arc<dyn RunQueuePort>,
    Arc<InMemoryWaypointStore>,
    Vec<Arc<AtomicUsize>>,
) {
    let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
    let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
    let store = Arc::new(InMemoryWaypointStore::new());
    let (graph, counters) = build_chain_graph(2, Duration::ZERO);
    let resolver: Arc<dyn AssistantResolver> =
        Arc::new(CodeWorkflowResolver::new().register("chain", graph));
    let base_engine = Arc::new(WarEngine::new(Arc::new(UnusedPaladinPort), store.clone()));

    let factory_store = store.clone();
    let engine_factory: Arc<
        dyn Fn(tokio_util::sync::CancellationToken) -> WarEngine<InMemoryWaypointStore>
            + Send
            + Sync,
    > = Arc::new(move |token| {
        WarEngine::new(Arc::new(UnusedPaladinPort), factory_store.clone())
            .with_cancellation_token(token)
    });

    let mut pool = RunWorkerPool::new(
        base_engine,
        store.clone(),
        repository.clone(),
        queue.clone(),
        resolver,
        Duration::from_secs(30),
    )
    .with_engine_factory(engine_factory)
    .with_trace_config(trace_config);
    if let Some(bus) = event_bus {
        pool = pool.with_event_bus(bus);
    }

    (pool, repository, queue, store, counters)
}

/// Behavior (Task 1): with `trace.log_sink` on and an event bus present,
/// the run's bus sink is live -- proving the worker attached a composite
/// (or, at minimum, a sink that still forwards to the bus alongside the
/// log sink `build_run_sink`'s own unit tests already prove is fanned into
/// the same composite when both are configured). With `trace.log_sink`
/// off, the run still completes and the bus sink alone still works. With
/// neither configured, the run completes using the engine's own untraced
/// path (no sink attached at all).
#[tokio::test]
async fn worker_builds_one_composite_per_run() {
    use crate::application::services::run::events::RunEventBus;
    use crate::config::trace::TraceConfig;

    // --- Both `log_sink` and the event bus configured: the bus sink must
    // still receive every record, proving it is part of whatever sink
    // `build_run_sink` assembled (its own unit tests prove that assembly
    // is a `CompositeSink` of both when both are configured).
    let bus = Arc::new(RunEventBus::new());
    let (pool, repository, queue, _store, _counters) = build_traced_pool(
        TraceConfig {
            log_sink: true,
            ..TraceConfig::default()
        },
        Some(bus.clone()),
    );
    let (run_id, thread_id) = submit(&repository, &queue, "chain").await;
    // Pre-bind so a subscriber can attach before `run_once`'s own `bind`
    // call re-affirms the SAME channel (`bind` is idempotent -- see
    // `RunEventBus::bind`'s own doc comment).
    bus.bind(thread_id.clone(), run_id.clone()).await;
    let mut rx = bus
        .subscribe(&run_id)
        .await
        .expect("the channel exists once bound");

    assert!(pool.run_once().await.unwrap());
    let run = repository.get(&run_id).await.unwrap().unwrap();
    assert_eq!(run.status, RunStatus::Completed);

    let mut saw_any_event = false;
    while let Ok(event) = rx.try_recv() {
        let _ = event;
        saw_any_event = true;
    }
    assert!(
        saw_any_event,
        "the bus sink must have received at least one record through whatever sink \
         build_run_sink assembled for this run"
    );

    // --- `trace.log_sink` off, bus still present: the bus-only path still
    // works (build_run_sink returns the bus sink alone, unwrapped).
    let bus2 = Arc::new(RunEventBus::new());
    let (pool2, repository2, queue2, _store2, _counters2) = build_traced_pool(
        TraceConfig {
            log_sink: false,
            ..TraceConfig::default()
        },
        Some(bus2.clone()),
    );
    let (run_id2, thread_id2) = submit(&repository2, &queue2, "chain").await;
    bus2.bind(thread_id2.clone(), run_id2.clone()).await;
    let mut rx2 = bus2
        .subscribe(&run_id2)
        .await
        .expect("the channel exists once bound");
    assert!(pool2.run_once().await.unwrap());
    let run2 = repository2.get(&run_id2).await.unwrap().unwrap();
    assert_eq!(run2.status, RunStatus::Completed);
    assert!(
        rx2.try_recv().is_ok(),
        "the bus sink alone (log_sink off) must still receive records"
    );

    // --- Neither configured: the run still completes -- the engine's own
    // untraced path (no `TraceSink` attached at all) never affects
    // correctness.
    let (pool3, repository3, queue3, _store3, _counters3) = build_traced_pool(
        TraceConfig {
            log_sink: false,
            ..TraceConfig::default()
        },
        None,
    );
    let (run_id3, _thread_id3) = submit(&repository3, &queue3, "chain").await;
    assert!(pool3.run_once().await.unwrap());
    let run3 = repository3.get(&run_id3).await.unwrap().unwrap();
    assert_eq!(run3.status, RunStatus::Completed);
}

/// Behavior (Task 1, prohibition): enabling any sink combination MUST NOT
/// change a run's outcome or its final executed node count -- the same
/// fixture graph, dispatched once with sinks fully off and once with both
/// `trace.log_sink` and an event bus on, must reach the SAME terminal
/// status and each of its two chain nodes must have executed EXACTLY once
/// either way.
#[tokio::test]
async fn worker_run_result_is_identical_with_and_without_sinks() {
    use crate::application::services::run::events::RunEventBus;
    use crate::config::trace::TraceConfig;

    // Sinks fully off.
    let (pool_off, repository_off, queue_off, _store_off, counters_off) = build_traced_pool(
        TraceConfig {
            log_sink: false,
            ..TraceConfig::default()
        },
        None,
    );
    let (run_id_off, _thread_id_off) = submit(&repository_off, &queue_off, "chain").await;
    assert!(pool_off.run_once().await.unwrap());
    let run_off = repository_off.get(&run_id_off).await.unwrap().unwrap();

    // Both sinks on.
    let bus = Arc::new(RunEventBus::new());
    let (pool_on, repository_on, queue_on, _store_on, counters_on) = build_traced_pool(
        TraceConfig {
            log_sink: true,
            ..TraceConfig::default()
        },
        Some(bus),
    );
    let (run_id_on, _thread_id_on) = submit(&repository_on, &queue_on, "chain").await;
    assert!(pool_on.run_once().await.unwrap());
    let run_on = repository_on.get(&run_id_on).await.unwrap().unwrap();

    assert_eq!(
        run_off.status, run_on.status,
        "the run's terminal status must be identical with and without sinks"
    );
    assert_eq!(run_off.status, RunStatus::Completed);

    assert_eq!(
        counters_off.len(),
        counters_on.len(),
        "the same fixture graph must have the same node count either way"
    );
    for (off, on) in counters_off.iter().zip(counters_on.iter()) {
        assert_eq!(
            off.load(Ordering::SeqCst),
            on.load(Ordering::SeqCst),
            "each node's own execution count must be identical with and without sinks"
        );
        assert_eq!(
            off.load(Ordering::SeqCst),
            1,
            "each of this chain's two nodes must execute exactly once"
        );
    }
}

// --- 41-07 (D-18, C6): allowance warnings on the run's own stream -------

mod allowance_warnings {
    use super::*;

    use paladin_core::platform::container::allowance::{
        AllowanceLimitKind, AllowanceScopeKind, AllowanceWarning, NoticeOutcome, NoticeRecord,
    };
    use paladin_core::platform::container::cost::{Cost, CurrencyCode};
    use paladin_core::platform::container::heartbeat::HeartbeatHandle;
    use paladin_core::platform::container::run_scope::RunScope;
    use paladin_core::platform::container::trace::TraceRecord;
    use paladin_ports::output::treasury_ledger_port::TreasuryLedgerError;
    use paladin_ports::output::treasury_notice_port::TreasuryNoticePort;
    use paladin_storage::treasury::in_memory::InMemoryTreasuryLedger;

    use crate::config::trace::TraceConfig;

    fn persisting_config() -> TraceConfig {
        TraceConfig {
            log_sink: false,
            persist: true,
            ..TraceConfig::default()
        }
    }

    fn notice_for(run_id: &RunId) -> NoticeRecord {
        use chrono::TimeZone;
        let usd = CurrencyCode::new("USD").unwrap();
        NoticeRecord {
            notice_id: format!("notice-{run_id}"),
            tenant_id: "acme".to_string(),
            api_key_id: Some("svc-a".to_string()),
            warning: AllowanceWarning {
                scope_kind: AllowanceScopeKind::ApiKey,
                limit_kind: AllowanceLimitKind::Window,
                balance: Cost::new(20_500_000_000, usd.clone()),
                ceiling: Cost::new(25_000_000_000, usd),
                window_start: chrono::Utc.with_ymd_and_hms(2026, 10, 2, 0, 0, 0).single(),
                window_end: chrono::Utc.with_ymd_and_hms(2026, 10, 3, 0, 0, 0).single(),
                warn_at: 80,
            },
            run_id: Some(run_id.clone()),
            recorded_at: chrono::Utc::now(),
        }
    }

    /// Seed `notices` with one row naming `run_id` as its admitting run.
    async fn seed(notices: &InMemoryTreasuryLedger, run_id: &RunId) {
        let outcome = notices.record(&notice_for(run_id)).await.unwrap();
        assert_eq!(outcome, NoticeOutcome::Recorded);
    }

    /// Poll the persisted trace until the terminal row lands (the persisting sink drains on
    /// its own schedule), then return every record in `seq` order.
    async fn read_traces(traces: &InMemoryRunTraceStore, thread_id: &ThreadId) -> Vec<TraceRecord> {
        let mut rows = Vec::new();
        for _ in 0..100 {
            rows = traces.read(thread_id, 0, 100).await.unwrap();
            if rows
                .iter()
                .any(|r| matches!(r.event, TraceEvent::RunFinished { .. }))
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        rows
    }

    fn allowance_rows(rows: &[TraceRecord]) -> Vec<&TraceRecord> {
        rows.iter()
            .filter(|r| matches!(r.event, TraceEvent::AllowanceWarning { .. }))
            .collect()
    }

    /// A graph pool with a persisted trace store and, when given, a notice store.
    #[allow(clippy::type_complexity)]
    fn graph_pool(
        notices: Option<Arc<dyn TreasuryNoticePort>>,
    ) -> (
        RunWorkerPool<InMemoryWaypointStore>,
        Arc<dyn RunRepositoryPort>,
        Arc<dyn RunQueuePort>,
        Arc<InMemoryRunTraceStore>,
    ) {
        let traces = Arc::new(InMemoryRunTraceStore::new());
        let (pool, repository, queue, _store, _counters) =
            build_traced_pool(persisting_config(), None);
        let mut pool = pool.with_run_trace_port(traces.clone());
        if let Some(notices) = notices {
            pool = pool.with_treasury_notices(notices);
        }
        (pool, repository, queue, traces)
    }

    #[tokio::test]
    async fn first_dispatch_emits_one_allowance_warning_before_run_started() {
        let notices = Arc::new(InMemoryTreasuryLedger::new());
        let (pool, repository, queue, traces) = graph_pool(Some(notices.clone()));
        let (run_id, thread_id) = submit(&repository, &queue, "chain").await;
        seed(&notices, &run_id).await;

        assert!(pool.run_once().await.unwrap());
        assert_eq!(
            repository.get(&run_id).await.unwrap().unwrap().status,
            RunStatus::Completed
        );

        let rows = read_traces(&traces, &thread_id).await;
        let warnings = allowance_rows(&rows);
        assert_eq!(warnings.len(), 1, "exactly one allowance_warning: {rows:?}");
        assert_eq!(warnings[0].run_id.as_ref(), Some(&run_id));
        let started = rows
            .iter()
            .find(|r| matches!(r.event, TraceEvent::RunStarted { .. }))
            .expect("the run has a RunStarted row");
        assert!(
            warnings[0].seq < started.seq,
            "the warning ({}) must precede RunStarted ({})",
            warnings[0].seq,
            started.seq
        );
        let seqs: Vec<u64> = rows.iter().map(|r| r.seq).collect();
        assert_eq!(
            seqs,
            (1..=rows.len() as u64).collect::<Vec<_>>(),
            "the stream stays gapless: no seq collision"
        );
        // The persisted row carries no tenant id or key name (D-00g).
        let json = serde_json::to_string(warnings[0]).unwrap();
        assert!(!json.contains("acme") && !json.contains("svc-a"), "{json}");
    }

    #[tokio::test]
    async fn running_redelivery_does_not_reemit_the_allowance_warning() {
        let notices = Arc::new(InMemoryTreasuryLedger::new());
        let (pool, repository, queue, traces) = graph_pool(Some(notices.clone()));
        let (run_id, thread_id) = submit(&repository, &queue, "chain").await;
        seed(&notices, &run_id).await;
        // The run was already dispatched once: a redelivery meets `Running`.
        repository
            .update_status(
                &run_id,
                RunStatus::Queued,
                RunStatus::Running,
                chrono::Utc::now(),
            )
            .await
            .unwrap();

        assert!(pool.run_once().await.unwrap());

        let rows = read_traces(&traces, &thread_id).await;
        assert!(!rows.is_empty(), "the redelivered run still traces");
        assert!(allowance_rows(&rows).is_empty(), "{rows:?}");
    }

    #[tokio::test]
    async fn awaiting_input_resume_does_not_reemit_the_allowance_warning() {
        let notices = Arc::new(InMemoryTreasuryLedger::new());
        let (pool, repository, queue, traces) = graph_pool(Some(notices.clone()));
        let (run_id, thread_id) = submit(&repository, &queue, "chain").await;
        seed(&notices, &run_id).await;
        for (from, to) in [
            (RunStatus::Queued, RunStatus::Running),
            (RunStatus::Running, RunStatus::AwaitingInput),
        ] {
            repository
                .update_status(&run_id, from, to, chrono::Utc::now())
                .await
                .unwrap();
        }

        assert!(pool.run_once().await.unwrap());

        let rows = read_traces(&traces, &thread_id).await;
        assert!(!rows.is_empty(), "the resumed run still traces");
        assert!(allowance_rows(&rows).is_empty(), "{rows:?}");
    }

    #[tokio::test]
    async fn pool_without_a_notice_store_emits_no_allowance_warning() {
        // A notice exists for the run, but this pool never had a store attached.
        let notices = Arc::new(InMemoryTreasuryLedger::new());
        let (pool, repository, queue, traces) = graph_pool(None);
        let (run_id, thread_id) = submit(&repository, &queue, "chain").await;
        seed(&notices, &run_id).await;

        assert!(pool.run_once().await.unwrap());
        assert_eq!(
            repository.get(&run_id).await.unwrap().unwrap().status,
            RunStatus::Completed
        );
        let rows = read_traces(&traces, &thread_id).await;
        assert!(!rows.is_empty());
        assert!(allowance_rows(&rows).is_empty(), "{rows:?}");
    }

    /// A notice store whose every read fails.
    struct FailingNotices;

    #[async_trait]
    impl TreasuryNoticePort for FailingNotices {
        async fn record(
            &self,
            _notice: &NoticeRecord,
        ) -> Result<NoticeOutcome, TreasuryLedgerError> {
            Err(TreasuryLedgerError::Backend {
                source: "always fails".into(),
            })
        }

        async fn notices_for_run(
            &self,
            _run_id: &RunId,
        ) -> Result<Vec<NoticeRecord>, TreasuryLedgerError> {
            Err(TreasuryLedgerError::Backend {
                source: "read failed".into(),
            })
        }

        async fn discard(&self, _notice_ids: &[String]) -> Result<(), TreasuryLedgerError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn notice_read_failure_never_fails_the_run() {
        let (pool, repository, queue, traces) = graph_pool(Some(Arc::new(FailingNotices)));
        let (run_id, thread_id) = submit(&repository, &queue, "chain").await;

        assert!(pool.run_once().await.unwrap());
        assert_eq!(
            repository.get(&run_id).await.unwrap().unwrap().status,
            RunStatus::Completed,
            "a notice read failure must never change the run's outcome"
        );
        let rows = read_traces(&traces, &thread_id).await;
        assert!(allowance_rows(&rows).is_empty(), "{rows:?}");
        assert!(
            rows.iter()
                .any(|r| matches!(r.event, TraceEvent::RunStarted { .. })),
            "the run still started"
        );
    }

    /// A [`PaladinPort`] that records whether the `RunScope` it was handed carried allowance
    /// warnings (the worker path must never pass them: the dispatcher emits them once).
    struct ScopeRecordingPort {
        scope_warning_counts: std::sync::Mutex<Vec<usize>>,
    }

    #[async_trait]
    impl PaladinPort for ScopeRecordingPort {
        async fn execute(
            &self,
            _paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinResult, PaladinError> {
            unreachable!("the worker calls execute_scoped")
        }

        async fn execute_scoped(
            &self,
            _paladin: &Paladin,
            _input: &str,
            _heartbeat: &HeartbeatHandle,
            scope: &RunScope,
        ) -> Result<PaladinResult, PaladinError> {
            self.scope_warning_counts
                .lock()
                .unwrap()
                .push(scope.allowance_warnings.len());
            Ok(PaladinResult {
                output: "agent completed".to_string(),
                usage: TokenUsage::new(11, 7),
                ..Default::default()
            })
        }

        async fn execute_stream(
            &self,
            _paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinStream, PaladinError> {
            unreachable!("this test never streams")
        }

        fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn agent_run_first_dispatch_emits_the_allowance_warning_once() {
        let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
        let store = Arc::new(InMemoryWaypointStore::new());
        let resolver: Arc<dyn AssistantResolver> = Arc::new(AgentOnlyResolver);
        let engine = Arc::new(WarEngine::new(Arc::new(UnusedPaladinPort), store.clone()));
        let traces = Arc::new(InMemoryRunTraceStore::new());
        let notices = Arc::new(InMemoryTreasuryLedger::new());
        let port = Arc::new(ScopeRecordingPort {
            scope_warning_counts: std::sync::Mutex::new(Vec::new()),
        });
        let pool = RunWorkerPool::new(
            engine,
            store,
            repository.clone(),
            queue.clone(),
            resolver,
            Duration::from_secs(30),
        )
        .with_paladin_port(port.clone())
        .with_trace_config(persisting_config())
        .with_run_trace_port(traces.clone())
        .with_treasury_notices(notices.clone());

        let (run_id, thread_id) = submit(&repository, &queue, "code-agent").await;
        seed(&notices, &run_id).await;

        assert!(pool.run_once().await.unwrap());
        assert_eq!(
            repository.get(&run_id).await.unwrap().unwrap().status,
            RunStatus::Completed
        );

        let rows = read_traces(&traces, &thread_id).await;
        let warnings = allowance_rows(&rows);
        assert_eq!(warnings.len(), 1, "emitted exactly once: {rows:?}");
        assert_eq!(
            warnings[0].seq, 1,
            "before RunStarted, the run's lowest seq"
        );
        assert!(matches!(rows[1].event, TraceEvent::RunStarted { .. }));
        assert_eq!(
            *port.scope_warning_counts.lock().unwrap(),
            vec![0],
            "the worker-path RunScope must not also carry the warning"
        );
    }
}

// --- ALLOW-03, Phase 42 D-00e, D-04: no guard for an unattributed run (42-04) --------------

/// A [`TreasuryLedgerPort`] over an in-memory ledger that counts every read the boundary guard
/// makes (`store_now` and `balance`), so a test can assert a run made none.
#[derive(Debug, Default)]
struct ReadCountingLedger {
    inner: paladin_storage::treasury::in_memory::InMemoryTreasuryLedger,
    reads: AtomicUsize,
    /// When set, every `store_now` and `balance` read fails with a backend error (42-09: an
    /// unreadable ledger at dispatch).
    fail_reads: std::sync::atomic::AtomicBool,
}

impl ReadCountingLedger {
    /// The backend error a failing read returns, once [`Self::fail_reads`] is set.
    fn read_failure(
        &self,
    ) -> Option<paladin_ports::output::treasury_ledger_port::TreasuryLedgerError> {
        self.fail_reads.load(Ordering::SeqCst).then(|| {
            paladin_ports::output::treasury_ledger_port::TreasuryLedgerError::Backend {
                source: "scripted ledger outage".into(),
            }
        })
    }
}

#[async_trait]
impl paladin_ports::output::treasury_ledger_port::TreasuryLedgerPort for ReadCountingLedger {
    async fn reserve(
        &self,
        request: paladin_core::platform::container::treasury_ledger::ReserveRequest,
    ) -> Result<
        paladin_core::platform::container::treasury_ledger::ReservationId,
        paladin_ports::output::treasury_ledger_port::TreasuryLedgerError,
    > {
        self.inner.reserve(request).await
    }

    async fn release(
        &self,
        reservation: paladin_core::platform::container::treasury_ledger::ReservationId,
    ) -> Result<(), paladin_ports::output::treasury_ledger_port::TreasuryLedgerError> {
        self.inner.release(reservation).await
    }

    async fn settle(
        &self,
        request: paladin_core::platform::container::treasury_ledger::SettleRequest,
    ) -> Result<
        paladin_core::platform::container::treasury_ledger::SettleOutcome,
        paladin_ports::output::treasury_ledger_port::TreasuryLedgerError,
    > {
        self.inner.settle(request).await
    }

    async fn spend(
        &self,
        query: paladin_core::platform::container::treasury_ledger::SpendQuery,
    ) -> Result<
        Vec<paladin_core::platform::container::treasury_ledger::SpendRow>,
        paladin_ports::output::treasury_ledger_port::TreasuryLedgerError,
    > {
        self.inner.spend(query).await
    }

    async fn store_now(
        &self,
    ) -> Result<
        chrono::DateTime<chrono::Utc>,
        paladin_ports::output::treasury_ledger_port::TreasuryLedgerError,
    > {
        self.reads.fetch_add(1, Ordering::SeqCst);
        if let Some(error) = self.read_failure() {
            return Err(error);
        }
        self.inner.store_now().await
    }

    async fn balance(
        &self,
        query: paladin_core::platform::container::treasury_ledger::BalanceQuery,
    ) -> Result<
        paladin_core::platform::container::cost::Cost,
        paladin_ports::output::treasury_ledger_port::TreasuryLedgerError,
    > {
        self.reads.fetch_add(1, Ordering::SeqCst);
        if let Some(error) = self.read_failure() {
            return Err(error);
        }
        self.inner.balance(query).await
    }
}

/// A run whose row records no submitter gets no guard, so it completes with zero ledger reads
/// even when a Treasurer with a ceiling for another principal is attached to the pool; the same
/// pool DOES guard an attributed run (the control that keeps the zero from being vacuous).
#[tokio::test]
async fn unattributed_run_gets_no_guard_and_reads_no_ledger() {
    use crate::application::services::treasurer::{AllowancePolicy, ScopeAllowance, Treasurer};
    use paladin_core::platform::container::cost::CurrencyCode;
    use paladin_core::platform::container::principal::{RunAttribution, TenantId};

    let ledger = Arc::new(ReadCountingLedger::default());
    let policy = AllowancePolicy::new(CurrencyCode::new("USD").unwrap(), 80)
        .with_api_key("svc-a", ScopeAllowance::new(3_600, 1_000_000_000));
    let treasurer = Arc::new(Treasurer::new(policy, ledger.clone()));

    let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
    let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
    let store = Arc::new(InMemoryWaypointStore::new());
    let (graph, _counters) = build_chain_graph(2, Duration::ZERO);
    let resolver: Arc<dyn AssistantResolver> =
        Arc::new(CodeWorkflowResolver::new().register("chain", graph));
    let base_engine = Arc::new(WarEngine::new(Arc::new(UnusedPaladinPort), store.clone()));
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
        base_engine,
        store.clone(),
        repository.clone(),
        queue.clone(),
        resolver,
        Duration::from_secs(30),
    )
    .with_engine_factory(engine_factory)
    .with_treasurer(treasurer);

    // An unattributed run: `submit` never records a submitter.
    let (unattributed, _) = submit(&repository, &queue, "chain").await;
    assert!(pool.run_once().await.unwrap());
    let run = repository.get(&unattributed).await.unwrap().unwrap();
    assert!(run.submitted_by.is_none());
    assert_eq!(run.status, RunStatus::Completed);
    assert_eq!(
        ledger.reads.load(Ordering::SeqCst),
        0,
        "an unattributed run has no allowance identity: no guard, no ledger read"
    );

    // Control: an attributed run on the same pool is guarded and reads the ledger.
    let attributed_id = RunId::new_v7();
    let attributed_thread = ThreadId::new(format!("thread-{attributed_id}")).unwrap();
    let attributed = Run::new(
        attributed_id.clone(),
        attributed_thread.clone(),
        AssistantRef {
            assistant_id: "chain".to_string(),
            version: 1,
        },
        serde_json::json!({}),
    )
    .with_submitted_by(RunAttribution::new(TenantId::new("acme").unwrap(), "svc-a"));
    repository.insert(&attributed).await.unwrap();
    queue
        .enqueue(QueuedRun {
            run_id: attributed_id.clone(),
            thread_id: attributed_thread,
            attempt: 1,
            enqueued_at: chrono::Utc::now(),
        })
        .await
        .unwrap();
    assert!(pool.run_once().await.unwrap());
    let run = repository.get(&attributed_id).await.unwrap().unwrap();
    assert_eq!(run.status, RunStatus::Completed);
    assert!(
        ledger.reads.load(Ordering::SeqCst) > 0,
        "the attributed run's guard reads the ledger at its boundaries"
    );
}

// --- ALLOW-03, ALLOW-05, Phase 42 D-12, D-13, G8, A8: the agent-kind halt (42-09) ----------

/// The shared run-engine service as the worker's `with_paladin_port` receives it in production:
/// the `TokenBudget` installed in Treasurer-only mode (G12), so only a scope carrying a derived
/// figure is ever cut. With the `web-server` feature this IS the production constructor
/// (`shared_engine_execution_port`); without it, which the facade module's feature gate forces,
/// a local adapter installs the same middleware over the same service, so the worker's behaviour
/// is proven on both builds.
pub(super) fn shared_agent_port(
    service: crate::application::services::paladin::paladin_execution_service::PaladinExecutionService,
) -> Arc<dyn PaladinPort> {
    #[cfg(feature = "web-server")]
    {
        crate::infrastructure::web::facade_provisioner::shared_engine_execution_port(
            service,
            &crate::config::agent_runtime::TokenBudgetConfig::default(),
        )
    }
    #[cfg(not(feature = "web-server"))]
    {
        use crate::application::services::paladin::middleware::limits::TokenBudget;
        use crate::config::agent_runtime::TokenBudgetConfig;
        use paladin_ports::output::streaming_executor_port::StreamingExecutorPort;

        struct ScopedServicePort(
            Arc<
                crate::application::services::paladin::paladin_execution_service::PaladinExecutionService,
            >,
        );

        #[async_trait]
        impl PaladinPort for ScopedServicePort {
            async fn execute(
                &self,
                paladin: &Paladin,
                input: &str,
            ) -> Result<PaladinResult, PaladinError> {
                self.0.execute(paladin, input).await
            }

            async fn execute_stream(
                &self,
                paladin: &Paladin,
                input: &str,
            ) -> Result<PaladinStream, PaladinError> {
                self.0.execute_stream(paladin, input).await
            }

            fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
                Ok(())
            }

            async fn execute_scoped(
                &self,
                paladin: &Paladin,
                input: &str,
                _heartbeat: &paladin_core::platform::container::heartbeat::HeartbeatHandle,
                scope: &paladin_core::platform::container::run_scope::RunScope,
            ) -> Result<PaladinResult, PaladinError> {
                self.0.execute_scoped(paladin, input, None, scope).await
            }
        }

        let service = service.with_middleware(Arc::new(TokenBudget::new(TokenBudgetConfig {
            enabled: false,
            ..TokenBudgetConfig::default()
        })));
        Arc::new(ScopedServicePort(Arc::new(service)))
    }
}

mod agent_budget {
    use super::*;

    use crate::application::services::paladin::paladin_execution_service::PaladinExecutionService;
    use crate::application::services::treasurer::{AllowancePolicy, ScopeAllowance, Treasurer};
    use crate::infrastructure::resilience::circuit_breaker::CircuitBreaker;
    use paladin_core::base::entity::node::Node;
    use paladin_core::platform::container::cost::{Cost, CurrencyCode, PriceRow, PriceTable};
    use paladin_core::platform::container::paladin::{MaxLoops, PaladinData};
    use paladin_core::platform::container::principal::{RunAttribution, TenantId};
    use paladin_core::platform::container::treasury_ledger::{
        LedgerScope, SettleRequest, SettlementKey,
    };
    use paladin_llm::mock::MockLlmAdapter;
    use paladin_ports::output::treasury_ledger_port::TreasuryLedgerPort;

    /// 10 USD per 1M tokens on the dearest axis: one token is 10_000 nanos, so a lifetime
    /// ceiling of 1_500_000 nanos derives exactly 150 tokens.
    const PRICE_PER_MILLION: i64 = 10_000_000_000;
    const CEILING_NANOS: i64 = 1_500_000;

    /// Resolves every assistant id to a looping `Runnable::Agent` on `model`: with a scripted
    /// model reporting 100 tokens per response, only a budget or `max_loops` ends it.
    struct LoopingAgentResolver {
        model: String,
    }

    #[async_trait]
    impl AssistantResolver for LoopingAgentResolver {
        async fn resolve(
            &self,
            assistant_id: &str,
            version: Option<u32>,
        ) -> Result<super::super::resolver::ResolvedAssistant, super::super::resolver::ResolveError>
        {
            Ok(super::super::resolver::ResolvedAssistant {
                reference: AssistantRef {
                    assistant_id: assistant_id.to_string(),
                    version: version.unwrap_or(1),
                },
                runnable: super::super::resolver::Runnable::Agent(Arc::new(Node::new(
                    PaladinData {
                        system_prompt: "system".to_string(),
                        model: self.model.clone(),
                        max_loops: MaxLoops::Fixed(10),
                        ..Default::default()
                    },
                    Some(assistant_id.to_string()),
                ))),
                allowed_roles: vec![],
                source: paladin_core::platform::container::assistant::AssistantSource::Code,
            })
        }
    }

    fn usd() -> CurrencyCode {
        CurrencyCode::new("USD").expect("USD is valid")
    }

    struct Harness {
        pool: RunWorkerPool<InMemoryWaypointStore>,
        repository: Arc<dyn RunRepositoryPort>,
        queue: Arc<dyn RunQueuePort>,
        ledger: Arc<ReadCountingLedger>,
        llm: Arc<MockLlmAdapter>,
        deliveries: Arc<dyn WebhookDeliveryRepositoryPort>,
        bus: Arc<super::super::events::RunEventBus>,
    }

    /// A worker over a looping agent on `model`, a real `PaladinExecutionService` over a mock
    /// reporting 100 tokens per response, the shared-service wrapper, and a Treasurer with a
    /// lifetime ceiling of `ceiling` nanos for `svc-a` (`None`: no ceiling for anyone) priced
    /// only for `priced_model`.
    fn harness(model: &str, priced_model: &str, ceiling: Option<i64>) -> Harness {
        let ledger = Arc::new(ReadCountingLedger::default());
        let mut policy = AllowancePolicy::new(usd(), 80);
        if let Some(cap) = ceiling {
            policy = policy.with_api_key(
                "svc-a",
                ScopeAllowance::new(86_400, i64::MAX).with_lifetime(cap),
            );
        }
        let prices = Arc::new(PriceTable::new(usd()).with_row(
            priced_model,
            PriceRow::new(PRICE_PER_MILLION, PRICE_PER_MILLION).unwrap(),
        ));
        let ledger_port: Arc<dyn TreasuryLedgerPort> = ledger.clone();
        let treasurer = Arc::new(Treasurer::new(policy, ledger_port).with_pricing(prices));

        let llm = Arc::new(
            MockLlmAdapter::new()
                .with_response("chunk")
                .with_token_usage(0, 100, 100),
        );
        let service = PaladinExecutionService::new(
            llm.clone(),
            Arc::new(CircuitBreaker::new(5, 2, Duration::from_secs(30))),
            None,
            None,
        );

        let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
        let store = Arc::new(InMemoryWaypointStore::new());
        let resolver: Arc<dyn AssistantResolver> = Arc::new(LoopingAgentResolver {
            model: model.to_string(),
        });
        let engine = Arc::new(WarEngine::new(Arc::new(UnusedPaladinPort), store.clone()));
        let deliveries: Arc<dyn WebhookDeliveryRepositoryPort> =
            Arc::new(InMemoryWebhookDeliveryRepository::new());
        let bus = Arc::new(super::super::events::RunEventBus::new());
        let pool = RunWorkerPool::new(
            engine,
            store,
            repository.clone(),
            queue.clone(),
            resolver,
            Duration::from_secs(30),
        )
        .with_paladin_port(shared_agent_port(service))
        .with_treasurer(treasurer)
        .with_event_bus(bus.clone())
        .with_webhook_deliveries(Arc::clone(&deliveries))
        .with_trace_config(crate::config::trace::TraceConfig {
            log_sink: false,
            ..crate::config::trace::TraceConfig::default()
        });
        Harness {
            pool,
            repository,
            queue,
            ledger,
            llm,
            deliveries,
            bus,
        }
    }

    /// Insert a queued agent-kind run submitted by `acme` / `svc-a` (or by nobody) subscribed
    /// to every terminal webhook event, and return its ids with a subscribed event receiver.
    async fn submit_agent_run(
        h: &Harness,
        submitter: bool,
    ) -> (
        RunId,
        tokio::sync::broadcast::Receiver<paladin_core::platform::container::run::RunStreamEvent>,
    ) {
        let run_id = RunId::new_v7();
        let thread_id = ThreadId::new(format!("thread-{run_id}")).unwrap();
        let mut run = Run::new(
            run_id.clone(),
            thread_id.clone(),
            AssistantRef {
                assistant_id: "code-agent".to_string(),
                version: 1,
            },
            serde_json::json!({ "input": "hi" }),
        )
        .with_webhook(WebhookSpec {
            url: "https://example.com/hook".to_string(),
            secret: None,
            events: vec![
                RunEventKind::Completed,
                RunEventKind::Halted,
                RunEventKind::Failed,
            ],
        });
        if submitter {
            run =
                run.with_submitted_by(RunAttribution::new(TenantId::new("acme").unwrap(), "svc-a"));
        }
        h.repository.insert(&run).await.unwrap();
        h.queue
            .enqueue(QueuedRun {
                run_id: run_id.clone(),
                thread_id: thread_id.clone(),
                attempt: 1,
                enqueued_at: chrono::Utc::now(),
            })
            .await
            .unwrap();
        h.bus.bind(thread_id, run_id.clone()).await;
        let rx = h.bus.subscribe(&run_id).await.expect("bound");
        (run_id, rx)
    }

    /// Settle `nanos` for `acme` / `svc-a` -- the allowance spent while a run sat in the queue.
    async fn spend(h: &Harness, nanos: i64) {
        h.ledger
            .settle(SettleRequest::unreserved(
                LedgerScope::new("acme", "svc-a"),
                SettlementKey::new(RunId::new_v7(), 1, 1),
                Cost::new(nanos, usd()),
                std::collections::BTreeMap::from([("gpt-4".to_string(), nanos)]),
            ))
            .await
            .expect("the settlement is accepted");
    }

    async fn only_delivery(h: &Harness, run_id: &RunId) -> (RunEventKind, serde_json::Value) {
        let page = h.deliveries.list_for_run(run_id, 10, None).await.unwrap();
        assert_eq!(
            page.items.len(),
            1,
            "exactly one delivery: {:?}",
            page.items
        );
        let delivery = &page.items[0];
        (
            delivery.event,
            serde_json::from_str(&delivery.payload).unwrap(),
        )
    }

    /// D-12, D-13, G8: a worker-dispatched agent-kind run whose derived budget is crossed is
    /// recorded `Halted` with the `allowance_exhausted` object, `error` null, the partial output
    /// kept, and its single terminal event and `halted` webhook both carry the reason.
    #[tokio::test]
    async fn agent_kind_run_crossing_its_derived_budget_is_halted_with_the_reason() {
        let h = harness("gpt-4", "gpt-4", Some(CEILING_NANOS));
        let (run_id, mut rx) = submit_agent_run(&h, true).await;

        assert!(h.pool.run_once().await.unwrap());

        // 150 tokens derived, 100 per response: the second response crosses it.
        assert_eq!(h.llm.call_count(), 2, "cut after the crossing response");
        let run = h.repository.get(&run_id).await.unwrap().unwrap();
        assert_eq!(run.status, RunStatus::Halted);
        assert!(run.error.is_none(), "a halt is not a failure");
        let reason = run.halt_reason.clone().expect("the reason is on the row");
        let HaltReason::AllowanceExhausted(refusal) = &reason else {
            panic!("expected allowance_exhausted, got {reason:?}");
        };
        assert_eq!(refusal.ceiling.nanos(), CEILING_NANOS);
        let output = run
            .output
            .as_ref()
            .and_then(|v| v.as_str())
            .expect("partial output kept");
        assert!(output.contains("chunk"), "partial output kept: {output}");
        assert!(
            output.contains("Token budget reached"),
            "truncation notice kept: {output}"
        );
        assert!(
            run.final_waypoint_id.is_none(),
            "an agent-kind run has no checkpoint (D-08)"
        );

        // One terminal wire event, `done` with status halted and the reason (D-12, D-14).
        let events = drain_events(&mut rx);
        assert_eq!(terminal_count(&events), 1, "got {events:?}");
        let terminal = events.last().unwrap();
        assert_eq!(terminal.kind, RunStreamEventKind::Done);
        assert_eq!(terminal.payload["status"], "halted");
        assert_eq!(terminal.payload["halt_reason"], reason.wire_json());

        // The halted webhook carries the reason (D-19).
        let (event, payload) = only_delivery(&h, &run_id).await;
        assert_eq!(event, RunEventKind::Halted);
        assert_eq!(payload["status"], "halted");
        assert_eq!(payload["halt_reason"], reason.wire_json());
    }

    /// G8: the allowance was spent while the run was queued. Dispatch re-derives, finds the
    /// ceiling exhausted and records the halt with the binding figures WITHOUT calling the LLM.
    /// A derived figure of zero tokens (headroom below one token's price) is the same.
    #[tokio::test]
    async fn agent_kind_run_with_zero_budget_at_dispatch_halts_without_calling_the_llm() {
        // Spent to the ceiling while queued.
        let h = harness("gpt-4", "gpt-4", Some(CEILING_NANOS));
        let (run_id, mut rx) = submit_agent_run(&h, true).await;
        spend(&h, CEILING_NANOS).await;

        assert!(h.pool.run_once().await.unwrap());

        assert_eq!(
            h.llm.call_count(),
            0,
            "no LLM call once the allowance is spent"
        );
        let run = h.repository.get(&run_id).await.unwrap().unwrap();
        assert_eq!(run.status, RunStatus::Halted);
        assert!(run.error.is_none());
        assert!(
            run.output.is_none(),
            "nothing ran, so there is no partial output"
        );
        let Some(HaltReason::AllowanceExhausted(refusal)) = run.halt_reason.clone() else {
            panic!("expected allowance_exhausted, got {:?}", run.halt_reason);
        };
        assert_eq!(refusal.ceiling.nanos(), CEILING_NANOS);
        assert_eq!(
            refusal.balance.nanos(),
            CEILING_NANOS,
            "the binding ceiling's real balance"
        );
        let events = drain_events(&mut rx);
        assert_eq!(terminal_count(&events), 1, "got {events:?}");
        assert_eq!(events.last().unwrap().payload["status"], "halted");
        assert_eq!(
            events.last().unwrap().payload["halt_reason"],
            run.halt_reason.as_ref().unwrap().wire_json()
        );
        let (event, payload) = only_delivery(&h, &run_id).await;
        assert_eq!(event, RunEventKind::Halted);
        assert_eq!(payload["halt_reason"], run.halt_reason.unwrap().wire_json());

        // Headroom of 5_000 nanos is half a token at 10_000 nanos per token: derived zero.
        let h = harness("gpt-4", "gpt-4", Some(CEILING_NANOS));
        let (run_id, _rx) = submit_agent_run(&h, true).await;
        spend(&h, CEILING_NANOS - 5_000).await;
        assert!(h.pool.run_once().await.unwrap());
        assert_eq!(
            h.llm.call_count(),
            0,
            "a zero derived figure is a refusal, not a run"
        );
        let run = h.repository.get(&run_id).await.unwrap().unwrap();
        assert_eq!(run.status, RunStatus::Halted);
        assert!(matches!(
            run.halt_reason,
            Some(HaltReason::AllowanceExhausted(_))
        ));
    }

    /// A8, D-03: an unreadable ledger at dispatch fails closed -- `Halted` with
    /// `ledger_unavailable`, no LLM call, no figures.
    #[tokio::test]
    async fn agent_kind_run_with_an_unreadable_ledger_at_dispatch_halts_ledger_unavailable_without_calling_the_llm()
     {
        let h = harness("gpt-4", "gpt-4", Some(CEILING_NANOS));
        let (run_id, mut rx) = submit_agent_run(&h, true).await;
        h.ledger.fail_reads.store(true, Ordering::SeqCst);

        assert!(h.pool.run_once().await.unwrap());

        assert_eq!(h.llm.call_count(), 0);
        let run = h.repository.get(&run_id).await.unwrap().unwrap();
        assert_eq!(run.status, RunStatus::Halted);
        assert!(run.error.is_none());
        assert_eq!(run.halt_reason, Some(HaltReason::LedgerUnavailable));
        let events = drain_events(&mut rx);
        assert_eq!(terminal_count(&events), 1, "got {events:?}");
        assert_eq!(
            events.last().unwrap().payload["halt_reason"],
            HaltReason::LedgerUnavailable.wire_json()
        );
        let (event, payload) = only_delivery(&h, &run_id).await;
        assert_eq!(event, RunEventKind::Halted);
        assert_eq!(
            payload["halt_reason"],
            HaltReason::LedgerUnavailable.wire_json()
        );
    }

    /// D-10: a model that lost its price row between admission and dispatch is `Failed` with a
    /// typed error naming the model -- not a halt, and never a run of unmetered spend.
    #[tokio::test]
    async fn agent_kind_run_whose_model_became_unpriced_fails_without_calling_the_llm() {
        let h = harness("gpt-4", "some-other-model", Some(CEILING_NANOS));
        let (run_id, mut rx) = submit_agent_run(&h, true).await;

        assert!(h.pool.run_once().await.unwrap());

        assert_eq!(h.llm.call_count(), 0);
        let run = h.repository.get(&run_id).await.unwrap().unwrap();
        assert_eq!(run.status, RunStatus::Failed);
        assert!(run.halt_reason.is_none());
        let error = run.error.as_deref().expect("the typed error is on the row");
        assert!(error.contains("gpt-4"), "names the model: {error}");
        assert!(
            error.contains("treasurer.pricing"),
            "names the missing row: {error}"
        );
        let events = drain_events(&mut rx);
        assert_eq!(terminal_count(&events), 1, "got {events:?}");
        assert_eq!(events.last().unwrap().kind, RunStreamEventKind::Error);
        let (event, _payload) = only_delivery(&h, &run_id).await;
        assert_eq!(event, RunEventKind::Failed);
    }

    /// A principal with no configured ceiling, and a run that records no submitter, complete as
    /// before: no budget, every loop runs, no allowance ledger read is made for the former.
    #[tokio::test]
    async fn agent_kind_run_without_a_ceiling_completes_as_before() {
        // No ceiling for anyone: `evaluate` short-circuits with no ledger read.
        let h = harness("gpt-4", "gpt-4", None);
        let (run_id, mut rx) = submit_agent_run(&h, true).await;
        assert!(h.pool.run_once().await.unwrap());
        assert_eq!(h.llm.call_count(), 10, "only max_loops ends the run");
        assert_eq!(
            h.ledger.reads.load(Ordering::SeqCst),
            0,
            "no ledger read without a ceiling"
        );
        let run = h.repository.get(&run_id).await.unwrap().unwrap();
        assert_eq!(run.status, RunStatus::Completed);
        assert!(run.halt_reason.is_none());
        let events = drain_events(&mut rx);
        assert_eq!(events.last().unwrap().kind, RunStreamEventKind::Done);
        assert_eq!(events.last().unwrap().payload["status"], "completed");
        assert!(events.last().unwrap().payload.get("halt_reason").is_none());
        let (event, payload) = only_delivery(&h, &run_id).await;
        assert_eq!(event, RunEventKind::Completed);
        assert!(payload.get("halt_reason").is_none());

        // A run whose row records no submitter is never derived, even under a ceiling.
        let h = harness("gpt-4", "gpt-4", Some(CEILING_NANOS));
        let (run_id, _rx) = submit_agent_run(&h, false).await;
        assert!(h.pool.run_once().await.unwrap());
        assert_eq!(h.llm.call_count(), 10);
        assert_eq!(h.ledger.reads.load(Ordering::SeqCst), 0);
        assert_eq!(
            h.repository.get(&run_id).await.unwrap().unwrap().status,
            RunStatus::Completed
        );
    }
}
