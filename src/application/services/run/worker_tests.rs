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
use paladin_core::platform::container::battlefield::{Battlefield, BattlefieldSchema, StateDelta};
use paladin_core::platform::container::directive::Directive;
use paladin_core::platform::container::execution_result::PaladinResult;
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::parley::ParleyResponse;
use paladin_core::platform::container::run::{AssistantRef, Run, RunId, RunStatus};
use paladin_core::platform::container::waypoint::{
    NodeId, ThreadId, Waypoint, WaypointId, WaypointStatus,
};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinStream};
use paladin_ports::output::run_queue_port::{QueuedRun, RunQueuePort};
use paladin_ports::output::run_repository_port::RunRepositoryPort;
use paladin_ports::output::waypoint_port::{
    ThreadSummary, WaypointError, WaypointPort, WaypointSummary,
};
use paladin_storage::run::in_memory::InMemoryRunRepository;
use paladin_storage::run_queue::in_memory::InMemoryRunQueue;
use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;

use super::resolver::{AssistantResolver, CodeWorkflowResolver};
use super::worker::{RunWorkerOptions, RunWorkerPool};

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

#[tokio::test(flavor = "multi_thread")]
async fn heartbeat_extends_at_lease_over_four() {
    tokio::time::timeout(Duration::from_secs(15), async {
        struct RecordingQueue {
            inner: InMemoryRunQueue,
            extend_calls: std::sync::Mutex<Vec<tokio::time::Instant>>,
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

            async fn depth(
                &self,
            ) -> Result<u64, paladin_ports::output::run_queue_port::QueueError> {
                self.inner.depth().await
            }
        }

        let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let recording = Arc::new(RecordingQueue {
            inner: InMemoryRunQueue::new(),
            extend_calls: std::sync::Mutex::new(Vec::new()),
        });
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
