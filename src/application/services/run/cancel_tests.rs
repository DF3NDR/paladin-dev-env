//! Cross-instance cancellation proof (27-07 Task 2, D-14/D-15/D-16,
//! PLAT-FR-04): `cross_instance_cancel_probe` proves a flag written through
//! ONE instance's repository handle halts a run executing on ANOTHER
//! instance at its next superstep boundary; `local_cancel_signals_token`
//! proves the same-instance fast path is instant, bypassing the probe's
//! debounce window entirely.
//!
//! Tier 1 per D-51: `SqliteRunRepository`/`SqliteWaypointStore` over a real
//! on-disk temp file (never `:memory:` -- these tests need the SAME
//! database observed from two logically separate service/pool instances,
//! which is the point: the durable flag, not an in-process shortcut, is
//! what reaches the other instance), no Docker.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use paladin_battalion::engine::{
    EdgeSpec, EngineLimits, NodeContext, NodeSpec, StateNode, StateNodeError, WarEngine, WarGraph,
};
use paladin_core::platform::container::battlefield::{Battlefield, BattlefieldSchema, StateDelta};
use paladin_core::platform::container::directive::Directive;
use paladin_core::platform::container::execution_result::PaladinResult;
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::run::{AssistantRef, Run, RunId, RunStatus};
use paladin_core::platform::container::waypoint::{NodeId, ThreadId, WaypointStatus};
use paladin_ports::input::run_submission_port::RunSubmissionPort;
use paladin_ports::output::paladin_port::{PaladinPort, PaladinStream};
use paladin_ports::output::run_queue_port::{QueuedRun, RunQueuePort};
use paladin_ports::output::run_repository_port::RunRepositoryPort;
use paladin_ports::output::waypoint_port::WaypointPort;
use paladin_storage::run::sqlite::SqliteRunRepository;
use paladin_storage::run_queue::in_memory::InMemoryRunQueue;
use paladin_storage::waypoint::sqlite::SqliteWaypointStore;

use super::resolver::{AssistantResolver, CodeWorkflowResolver};
use super::submission::RunSubmissionService;
use super::worker::RunWorkerPool;

/// A [`PaladinPort`] that must never be called -- every graph in this module
/// is Function-only (mirrors `worker_tests.rs`'s `UnusedPaladinPort`
/// precedent).
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

/// A [`StateNode`] that sleeps `delay` then completes -- the slow node
/// instrument (mirrors `worker_tests.rs`'s `DelayedCountingNode`, minus the
/// counter this module's tests do not need).
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
/// node sleeping `delay` before completing -- one node dispatched per
/// superstep.
fn build_slow_chain_graph(count: usize, delay: Duration) -> Arc<WarGraph> {
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

/// A fresh on-disk SQLite URL under the system temp dir, cleaned up by the
/// caller (best-effort; `#[tokio::test]` leaves stray files on panic, but
/// never on a normal pass).
fn temp_sqlite_url(label: &str) -> (std::path::PathBuf, String) {
    let path = std::env::temp_dir().join(format!(
        "paladin_cancel_test_{label}_{}.sqlite",
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

/// Build a per-run engine factory over `waypoint_store` -- the base piece
/// `RunWorkerPool::with_engine_factory` needs. The pool itself attaches
/// `with_cancellation_probe` (when `with_cancellation_probing` was called)
/// on top of what this closure returns; the closure only needs to wire the
/// per-run `CancellationToken`.
fn engine_factory(
    waypoint_store: Arc<SqliteWaypointStore>,
) -> Arc<dyn Fn(CancellationToken) -> WarEngine<SqliteWaypointStore> + Send + Sync> {
    Arc::new(move |token| {
        WarEngine::new(Arc::new(UnusedPaladinPort), waypoint_store.clone())
            .with_cancellation_token(token)
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn cross_instance_cancel_probe() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (repo_path, repo_url) = temp_sqlite_url("repo");
        let (wp_path, wp_url) = temp_sqlite_url("waypoint");

        let repository: Arc<dyn RunRepositoryPort> =
            Arc::new(SqliteRunRepository::new(&repo_url).await.unwrap());
        let waypoint_store = Arc::new(SqliteWaypointStore::new(&wp_url).await.unwrap());
        let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());

        let graph = build_slow_chain_graph(6, Duration::from_millis(200));
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("slow-chain", graph));

        // D-15: a short debounce so A's own periodic boundary checks (one
        // per ~200ms superstep) always see a fresh read well past the
        // debounce window.
        let min_probe_interval = Duration::from_millis(50);

        let base_engine_a = Arc::new(WarEngine::new(
            Arc::new(UnusedPaladinPort),
            waypoint_store.clone(),
        ));
        let pool_a = Arc::new(
            RunWorkerPool::new(
                base_engine_a,
                waypoint_store.clone(),
                repository.clone(),
                queue.clone(),
                resolver.clone(),
                Duration::from_secs(30),
            )
            .with_engine_factory(engine_factory(waypoint_store.clone()))
            .with_cancellation_probing(min_probe_interval),
        );

        let base_engine_b = Arc::new(WarEngine::new(
            Arc::new(UnusedPaladinPort),
            waypoint_store.clone(),
        ));
        // B's pool is constructed but NEVER dequeues/dispatches anything --
        // its own `local_tokens()` stays empty for the whole test, which is
        // exactly what proves `was_local == false` below: B is a
        // logically separate instance that never locally touched this run.
        let pool_b = RunWorkerPool::new(
            base_engine_b,
            waypoint_store.clone(),
            repository.clone(),
            queue.clone(),
            resolver,
            Duration::from_secs(30),
        )
        .with_engine_factory(engine_factory(waypoint_store.clone()))
        .with_cancellation_probing(min_probe_interval);

        let service_b = RunSubmissionService::new(
            repository.clone(),
            queue.clone(),
            Arc::new(CodeWorkflowResolver::new()),
        )
        .with_local_tokens(pool_b.local_tokens());

        let (run_id, thread_id) = submit(&repository, &queue, "slow-chain").await;

        let pool_a_run = Arc::clone(&pool_a);
        let run_task = tokio::spawn(async move { pool_a_run.run_once().await });

        // Wait for the first Waypoint to durably exist (superstep 1
        // complete) before requesting cancellation.
        loop {
            let history = waypoint_store
                .history(&thread_id, None, None)
                .await
                .unwrap();
            if !history.is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        let cancel_outcome = service_b.cancel(&run_id, None).await.unwrap();
        assert!(
            !cancel_outcome.was_local,
            "instance B never dispatched this run locally -- was_local must be false"
        );
        assert_eq!(cancel_outcome.status, RunStatus::Running);

        let processed = run_task
            .await
            .expect("pool A's run_once task must not panic")
            .unwrap();
        assert!(
            processed,
            "pool A must process the run to a terminal outcome"
        );

        let run = repository.get(&run_id).await.unwrap().unwrap();
        assert_eq!(
            run.status,
            RunStatus::Cancelled,
            "the run must be recorded Cancelled -- the caller explicitly asked for this (D-16)"
        );

        let latest = waypoint_store.latest(&thread_id).await.unwrap().unwrap();
        assert_eq!(
            latest.status,
            WaypointStatus::Halted,
            "the LATEST waypoint must be Halted -- the waypoint halted, the run was cancelled"
        );

        assert_eq!(
            queue.depth().await.unwrap(),
            0,
            "the queue message must be acked, not left in flight"
        );

        let history = waypoint_store
            .history(&thread_id, None, None)
            .await
            .unwrap();
        assert!(
            (history.len() as u64) < 6,
            "the cancel must stop the run before all 6 supersteps run, got {} waypoints",
            history.len()
        );

        cleanup(&repo_path);
        cleanup(&wp_path);
    })
    .await
    .expect("cross_instance_cancel_probe must finish within 30s");
}

#[tokio::test(flavor = "multi_thread")]
async fn local_cancel_signals_token() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (repo_path, repo_url) = temp_sqlite_url("repo-local");
        let (wp_path, wp_url) = temp_sqlite_url("waypoint-local");

        let repository: Arc<dyn RunRepositoryPort> =
            Arc::new(SqliteRunRepository::new(&repo_url).await.unwrap());
        let waypoint_store = Arc::new(SqliteWaypointStore::new(&wp_url).await.unwrap());
        let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());

        let graph = build_slow_chain_graph(6, Duration::from_millis(200));
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("slow-chain", graph));

        // A long debounce -- if cancellation reached the run through the
        // probe's own repository read, the run would keep executing well
        // past this test's timeout. The token path must not wait for it.
        let base_engine = Arc::new(WarEngine::new(
            Arc::new(UnusedPaladinPort),
            waypoint_store.clone(),
        ));
        let pool = Arc::new(
            RunWorkerPool::new(
                base_engine,
                waypoint_store.clone(),
                repository.clone(),
                queue.clone(),
                resolver,
                Duration::from_secs(30),
            )
            .with_engine_factory(engine_factory(waypoint_store.clone()))
            .with_cancellation_probing(Duration::from_secs(60)),
        );

        let service = RunSubmissionService::new(
            repository.clone(),
            queue.clone(),
            Arc::new(CodeWorkflowResolver::new()),
        )
        .with_local_tokens(pool.local_tokens());

        let (run_id, thread_id) = submit(&repository, &queue, "slow-chain").await;

        let pool_run = Arc::clone(&pool);
        let run_task = tokio::spawn(async move { pool_run.run_once().await });

        // Wait for the run to actually be dispatched (registered in
        // `local_tokens`) before cancelling -- a fixed short sleep is
        // enough since dispatch registration happens before the first
        // (200ms) node even starts.
        tokio::time::sleep(Duration::from_millis(50)).await;

        let cancel_outcome = service.cancel(&run_id, None).await.unwrap();
        assert!(
            cancel_outcome.was_local,
            "this instance IS the one dispatching the run -- was_local must be true"
        );

        let processed = run_task
            .await
            .expect("pool's run_once task must not panic")
            .unwrap();
        assert!(processed);

        let run = repository.get(&run_id).await.unwrap().unwrap();
        assert_eq!(run.status, RunStatus::Cancelled);

        let history = waypoint_store
            .history(&thread_id, None, None)
            .await
            .unwrap();
        assert!(
            (history.len() as u64) < 6,
            "the local token halt must stop the run well before all 6 supersteps run, got {} \
             waypoints",
            history.len()
        );

        cleanup(&repo_path);
        cleanup(&wp_path);
    })
    .await
    .expect("local_cancel_signals_token must finish within 30s");
}

#[tokio::test]
async fn cancel_is_idempotent_on_a_non_terminal_run() {
    let repository: Arc<dyn RunRepositoryPort> =
        Arc::new(paladin_storage::run::in_memory::InMemoryRunRepository::new());
    let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
    let resolver: Arc<dyn AssistantResolver> = Arc::new(CodeWorkflowResolver::new());
    let service = RunSubmissionService::new(repository.clone(), queue.clone(), resolver);

    let (run_id, _thread_id) = submit(&repository, &queue, "unused").await;

    let first = service.cancel(&run_id, None).await.unwrap();
    assert!(!first.was_local, "no worker pool is wired -- always false");
    let second = service.cancel(&run_id, None).await.unwrap();
    assert_eq!(
        first.status, second.status,
        "calling cancel twice on a non-terminal run must be Ok both times with the same status"
    );
}
