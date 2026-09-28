//! The router-level ten-concurrent-submits race (PRD acceptance 3, D-52)
//! and the fork-from-waypoint end-to-end proof (D-45), both driven through
//! the real facade services -- 27-15 Task 2.
//!
//! `ten_concurrent_submits_one_accepted` is Tier 1 per D-51:
//! `SqliteRunRepository` over a real on-disk temp file (never `:memory:` --
//! the one-active-run-per-thread invariant is a database uniqueness
//! constraint, D-17, and proving it holds under real concurrent writers is
//! the whole point), no Docker. `fork_run_completes_from_waypoint` uses
//! InMemory adapters, mirroring `worker_tests.rs`'s own Tier 1 convention
//! for behavior that does not need to cross a real process boundary.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use paladin_battalion::engine::{
    EdgeSpec, EngineLimits, NodeContext, NodeSpec, StateNode, StateNodeError, WarEngine, WarGraph,
};
use paladin_core::platform::container::battlefield::{BattlefieldSchema, StateDelta};
use paladin_core::platform::container::directive::Directive;
use paladin_core::platform::container::execution_result::PaladinResult;
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::principal::{RunAttribution, TenantId};
use paladin_core::platform::container::run::{RunId, RunStatus};
use paladin_core::platform::container::user::UserRole;
use paladin_core::platform::container::waypoint::NodeId;
use paladin_ports::input::run_submission_port::{ForkRun, RunSubmissionPort, SubmitRun};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinStream};
use paladin_ports::output::run_queue_port::RunQueuePort;
use paladin_ports::output::run_repository_port::RunRepositoryPort;
use paladin_ports::output::waypoint_port::WaypointPort;
use paladin_storage::run::in_memory::InMemoryRunRepository;
use paladin_storage::run::sqlite::SqliteRunRepository;
use paladin_storage::run_queue::in_memory::InMemoryRunQueue;
use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;
use paladin_web::agent_auth::{AgentAuthConfig, Principal};
use paladin_web::run_controller::{RunApiState, run_router};

use super::resolver::{AssistantResolver, CodeWorkflowResolver};
use super::submission::RunSubmissionService;
use super::worker::RunWorkerPool;

/// A [`PaladinPort`] that must never be called -- every graph in this
/// module is `Function`-only, mirroring `worker_tests.rs`'s/
/// `cancel_tests.rs`'s own `UnusedPaladinPort` precedent (a small, local
/// double per module rather than a shared `pub(crate)` one).
struct UnusedPaladinPort;

#[async_trait]
impl PaladinPort for UnusedPaladinPort {
    async fn execute(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinResult, PaladinError> {
        unreachable!("this module's WarGraphs have no NodeSpec::Paladin nodes")
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinStream, PaladinError> {
        unreachable!("this module's WarGraphs have no NodeSpec::Paladin nodes")
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

/// A trivial `StateNode` that advances with an empty delta -- one
/// superstep, no state, no delay. Mirrors `worker_tests.rs`'s
/// `DelayedCountingNode` with `delay: Duration::ZERO` and no counter.
struct NoopNode;

#[async_trait]
impl StateNode for NoopNode {
    async fn run(
        &self,
        _state: &paladin_core::platform::container::battlefield::Battlefield,
        _ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        Ok(StateDelta::new().into())
    }
}

/// Build a linear `count`-node chain (`n0 -> n1 -> ... -> n{count-1}`) of
/// [`NoopNode`]s, one superstep per node, with an EMPTY Battlefield schema
/// (D-45's `fork_run_completes_from_waypoint` forks with `edit: None` --
/// `fork_edit_to_state_delta`'s own merge logic is already unit-tested in
/// `worker.rs` in isolation, so this integration test does not also need a
/// declared schema field to exercise it).
fn build_chain_graph(count: usize) -> Arc<WarGraph> {
    let mut graph = WarGraph::new(BattlefieldSchema::new(vec![]), EngineLimits::default());
    let mut ids = Vec::with_capacity(count);
    for i in 0..count {
        let id = NodeId::new(format!("n{i}"));
        graph.add_node(id.clone(), NodeSpec::Function(Arc::new(NoopNode)));
        ids.push(id);
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

/// A fresh on-disk SQLite URL under the system temp dir, cleaned up by the
/// caller -- mirrors `cancel_tests.rs`'s own `temp_sqlite_url` helper
/// (module-local, not `pub`, so duplicated here rather than reached for
/// across a `#[cfg(test)]` module boundary).
fn temp_sqlite_url(label: &str) -> (std::path::PathBuf, String) {
    let path = std::env::temp_dir().join(format!(
        "paladin_http_surface_test_{label}_{}.sqlite",
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

/// PRD acceptance 3 / D-52: ten concurrent `POST /v1/runs` for ONE thread,
/// through the real `run_router` (oneshot, cloned router) over
/// `SqliteRunRepository` on a temp file -- exactly one `202` and nine `409
/// thread_busy`, proving D-17's partial-unique-index invariant holds under
/// real concurrent writers, not just a single-threaded check-then-insert.
#[tokio::test(flavor = "multi_thread")]
async fn ten_concurrent_submits_one_accepted() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (repo_path, repo_url) = temp_sqlite_url("race");
        let repository: Arc<dyn RunRepositoryPort> =
            Arc::new(SqliteRunRepository::new(&repo_url).await.unwrap());
        let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("race-wf", build_chain_graph(1)));
        let submission: Arc<dyn RunSubmissionPort> = Arc::new(RunSubmissionService::new(
            repository.clone(),
            queue.clone(),
            resolver.clone(),
        ));
        let state = RunApiState::new()
            .with_submission(submission)
            .with_repository(repository.clone());
        let app = run_router(state);

        let mut handles = Vec::with_capacity(10);
        for _ in 0..10 {
            let app = app.clone();
            handles.push(tokio::spawn(async move {
                let body = serde_json::to_vec(&serde_json::json!({
                    "assistant_id": "race-wf",
                    "thread_id": "race-thread",
                    "input": {}
                }))
                .unwrap();
                app.oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/v1/runs")
                        .header("content-type", "application/json")
                        .body(Body::from(body))
                        .unwrap(),
                )
                .await
                .unwrap()
                .status()
            }));
        }

        let mut accepted = 0;
        let mut busy = 0;
        for handle in handles {
            match handle.await.unwrap() {
                StatusCode::ACCEPTED => accepted += 1,
                StatusCode::CONFLICT => busy += 1,
                other => panic!("unexpected status from a concurrent submit: {other}"),
            }
        }

        assert_eq!(
            accepted, 1,
            "exactly one of ten concurrent submits must be accepted"
        );
        assert_eq!(busy, 9, "the other nine must be 409 thread_busy");

        cleanup(&repo_path);
    })
    .await
    .expect("ten_concurrent_submits_one_accepted did not hang");
}

/// D-45: a run to completion, forked from its second superstep's Waypoint
/// with an edit, completes and its history shows a Waypoint recording
/// `fork_of == Some(wp2)`.
#[tokio::test(flavor = "multi_thread")]
async fn fork_run_completes_from_waypoint() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
        let waypoint_store = Arc::new(InMemoryWaypointStore::new());
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("fork-wf", build_chain_graph(3)));

        let engine = Arc::new(WarEngine::new(
            Arc::new(UnusedPaladinPort),
            waypoint_store.clone(),
        ));
        let worker = RunWorkerPool::new(
            engine,
            waypoint_store.clone(),
            repository.clone(),
            queue.clone(),
            resolver.clone(),
            Duration::from_secs(30),
        );

        let waypoints_port: Arc<dyn WaypointPort> = waypoint_store.clone();
        let submission =
            RunSubmissionService::new(repository.clone(), queue.clone(), resolver.clone())
                .with_waypoints(waypoints_port);

        let accepted = submission
            .submit(SubmitRun {
                assistant_id: "fork-wf".to_string(),
                version: None,
                thread_id: None,
                input: serde_json::json!({}),
                webhook: None,
                requested_by: None,
            })
            .await
            .unwrap();
        let thread_id = accepted.thread_id.clone();

        // The whole 3-superstep chain runs synchronously inside one
        // dispatch (`WarEngine::start` drives the graph to `Completed`
        // internally; the worker never re-dequeues mid-graph).
        assert!(worker.run_once().await.unwrap());

        let original_run = repository.get(&accepted.run_id).await.unwrap().unwrap();
        assert_eq!(original_run.status, RunStatus::Completed);

        let history = waypoint_store
            .history(&thread_id, None, None)
            .await
            .unwrap();
        let wp2 = history
            .iter()
            .find(|s| s.superstep == 2)
            .expect("a superstep-2 waypoint exists")
            .waypoint_id;

        let forked = submission
            .fork(ForkRun {
                thread_id: thread_id.clone(),
                from_waypoint_id: wp2,
                edit: None,
                webhook: None,
                requested_by: None,
            })
            .await
            .unwrap();
        assert_eq!(forked.thread_id, thread_id);
        assert_ne!(forked.run_id, accepted.run_id);

        // Drive the fork dispatch -- `WorkerDispatch::decide` sees
        // `run.fork_from` on the freshly-enqueued forked run and the
        // latest Waypoint's `fork_of` not yet matching `wp2`, so it
        // selects `Fork` and calls `WarEngine::fork`.
        assert!(worker.run_once().await.unwrap());

        let forked_run = repository.get(&forked.run_id).await.unwrap().unwrap();
        assert_eq!(forked_run.status, RunStatus::Completed);

        let history_after_fork = waypoint_store
            .history(&thread_id, None, None)
            .await
            .unwrap();
        let fork_waypoint = history_after_fork
            .iter()
            .find(|s| s.fork_of == Some(wp2))
            .expect("a waypoint records fork_of == Some(wp2)");
        assert_eq!(fork_waypoint.fork_of, Some(wp2));
    })
    .await
    .expect("fork_run_completes_from_waypoint did not hang");
}

/// Phase 40's own tracer (TENANT-01, TENANT-02, PLAT-07, D-01, D-02, D-08, D-09, D-11,
/// D-12): an API key's configured tenant travels config -> `Principal.tenant_id`
/// (paladin-web) -> `PrincipalRef` on `SubmitRun` (paladin-ports) -> `Run.submitted_by`
/// (paladin-core) -> the `008` columns written by `SqliteRunRepository` (paladin-storage)
/// -> `GET /v1/runs/{id}` gated by `load_visible_run` (`RunReadScope::permits`) -- driven
/// through the real `run_router` over an on-disk SQLite store.
#[tokio::test(flavor = "multi_thread")]
async fn tenant_scoped_run_read_tracer() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (repo_path, repo_url) = temp_sqlite_url("tenant");
        let repository: Arc<dyn RunRepositoryPort> =
            Arc::new(SqliteRunRepository::new(&repo_url).await.unwrap());
        let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("tenant-wf", build_chain_graph(1)));
        let submission: Arc<dyn RunSubmissionPort> = Arc::new(RunSubmissionService::new(
            repository.clone(),
            queue.clone(),
            resolver.clone(),
        ));

        let mut api_keys = HashMap::new();
        api_keys.insert(
            "tracer-key-a".to_string(),
            Principal::new("svc-a", UserRole::User, TenantId::new("acme").unwrap()),
        );
        api_keys.insert(
            "tracer-key-b".to_string(),
            Principal::new("svc-b", UserRole::User, TenantId::new("globex").unwrap()),
        );
        api_keys.insert(
            "tracer-key-ops".to_string(),
            Principal::new("ops", UserRole::Admin, TenantId::new("ops-tenant").unwrap()),
        );
        let auth = AgentAuthConfig {
            enabled: true,
            api_keys,
            token_verifier: None,
            bearer_tenant: None,
        };

        let state = RunApiState::new()
            .with_submission(submission)
            .with_repository(repository.clone())
            .with_auth(auth);
        let app = run_router(state);

        // (a) POST /v1/runs with key tracer-key-a, plus a spoofed tenant on every
        // client-controlled surface (body field, query param, header) -- the tenant must
        // still come only from AgentAuthConfig (D-02).
        let submit_body = serde_json::to_vec(&serde_json::json!({
            "assistant_id": "tenant-wf",
            "thread_id": "tenant-thread-1",
            "input": {},
            "tenant_id": "globex",
            "tenant": "globex"
        }))
        .unwrap();
        let submit_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/runs?tenant_id=globex")
                    .header("content-type", "application/json")
                    .header("x-api-key", "tracer-key-a")
                    .header("x-tenant-id", "globex")
                    .body(Body::from(submit_body))
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(submit_response.status(), StatusCode::ACCEPTED);
        let submit_bytes = axum::body::to_bytes(submit_response.into_body(), usize::MAX)
            .await
            .expect("read submit body");
        let submit_json: serde_json::Value =
            serde_json::from_slice(&submit_bytes).expect("submit body is JSON");
        let run_id = RunId::parse(submit_json["run_id"].as_str().expect("run_id string"))
            .expect("run_id parses");

        // (b) The repository round trip: the run is attributed to the KEY's configured
        // tenant (acme), never the spoofed globex from the request (D-02, D-09).
        let stored = repository
            .get(&run_id)
            .await
            .expect("repository read succeeds")
            .expect("run exists");
        let expected_attribution = RunAttribution::new(TenantId::new("acme").unwrap(), "svc-a");
        assert_eq!(stored.submitted_by, Some(expected_attribution.clone()));

        // (c) A second submission by the same principal on a different thread carries
        // IDENTICAL attribution (edge TENANT-02/adjacency).
        let second_submit_body = serde_json::to_vec(&serde_json::json!({
            "assistant_id": "tenant-wf",
            "thread_id": "tenant-thread-2",
            "input": {}
        }))
        .unwrap();
        let second_submit_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/runs")
                    .header("content-type", "application/json")
                    .header("x-api-key", "tracer-key-a")
                    .body(Body::from(second_submit_body))
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(second_submit_response.status(), StatusCode::ACCEPTED);
        let second_submit_bytes =
            axum::body::to_bytes(second_submit_response.into_body(), usize::MAX)
                .await
                .expect("read second submit body");
        let second_submit_json: serde_json::Value =
            serde_json::from_slice(&second_submit_bytes).expect("second submit body is JSON");
        let second_run_id = RunId::parse(
            second_submit_json["run_id"]
                .as_str()
                .expect("run_id string"),
        )
        .expect("run_id parses");
        let second_stored = repository
            .get(&second_run_id)
            .await
            .expect("repository read succeeds")
            .expect("run exists");
        assert_eq!(second_stored.submitted_by, Some(expected_attribution));

        // (d) GET /v1/runs/{run_id}: the owner (svc-a) and an Admin of another tenant
        // (ops) both see it (D-11's Admin arm).
        let owner_get = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/runs/{run_id}"))
                    .header("x-api-key", "tracer-key-a")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(owner_get.status(), StatusCode::OK);

        let admin_get = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/runs/{run_id}"))
                    .header("x-api-key", "tracer-key-ops")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(admin_get.status(), StatusCode::OK);

        // (e) GET /v1/runs/{run_id} with a DIFFERENT tenant's key (svc-b, globex) is
        // the SAME missing-run 404 a genuinely unknown run id gets (PLAT-07, D-12): no
        // 403, no shape/timing difference between "hidden" and "missing".
        let hidden_get = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/runs/{run_id}"))
                    .header("x-api-key", "tracer-key-b")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(hidden_get.status(), StatusCode::NOT_FOUND);
        let hidden_bytes = axum::body::to_bytes(hidden_get.into_body(), usize::MAX)
            .await
            .expect("read hidden body");

        let missing_run_id = RunId::new_v7();
        let missing_get = app
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/runs/{missing_run_id}"))
                    .header("x-api-key", "tracer-key-b")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(missing_get.status(), StatusCode::NOT_FOUND);
        let missing_bytes = axum::body::to_bytes(missing_get.into_body(), usize::MAX)
            .await
            .expect("read missing body");

        let hidden_text = String::from_utf8(hidden_bytes.to_vec()).expect("utf8 body");
        let missing_text = String::from_utf8(missing_bytes.to_vec()).expect("utf8 body");
        let normalized_hidden =
            hidden_text.replace(&run_id.to_string(), &missing_run_id.to_string());
        assert_eq!(
            normalized_hidden, missing_text,
            "a hidden run's 404 body must be byte-identical to a genuinely missing run's, \
             once the two ids are swapped (PLAT-07, D-12)"
        );

        cleanup(&repo_path);
    })
    .await
    .expect("tenant_scoped_run_read_tracer did not hang");
}
