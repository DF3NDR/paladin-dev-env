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
use paladin_core::platform::container::principal::{PrincipalRef, RunAttribution, TenantId};
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

        // D-08: the fork is attributed to the FORKING principal (svc-a of acme), not
        // to whoever submitted the original (unattributed) run; the fork's own
        // latest-run lookup on the thread is unscoped (D-12/Pitfall 8) and still
        // finds the unattributed original. The forking principal is an Admin
        // (`RunReadScope::All`) because a tenant-scoped `User` may not fork a thread
        // whose latest run is unattributed -- the tenant guard treats it exactly
        // like a hidden run (phase 40 review WR-01).
        let forking_principal =
            PrincipalRef::new("svc-a", TenantId::new("acme").unwrap(), UserRole::Admin);
        let forked = submission
            .fork(ForkRun {
                thread_id: thread_id.clone(),
                from_waypoint_id: wp2,
                edit: None,
                webhook: None,
                requested_by: Some(forking_principal),
            })
            .await
            .unwrap();
        assert_eq!(forked.thread_id, thread_id);
        assert_ne!(forked.run_id, accepted.run_id);
        let forked_at_submit = repository.get(&forked.run_id).await.unwrap().unwrap();
        assert_eq!(
            forked_at_submit.submitted_by,
            Some(RunAttribution::new(TenantId::new("acme").unwrap(), "svc-a")),
            "a forked run is attributed to the forking principal (D-08)"
        );

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

/// Phase 40's list half of PLAT-07 (D-02, D-11, D-12, D-14): `GET /v1/runs` through the
/// real `run_router` over an on-disk `SqliteRunRepository` returns only the calling
/// principal's tenant's runs (across two API keys of the same tenant), every run for
/// an Admin, an exact empty page for a tenant with no runs, an empty page when another
/// tenant's `thread_id` is requested, and a gap-free `?limit=1` keyset walk -- with a
/// `tenant_id` query parameter changing nothing.
#[tokio::test(flavor = "multi_thread")]
async fn tenant_scoped_run_list_e2e() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (repo_path, repo_url) = temp_sqlite_url("tenant-list");
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
            "tracer-key-a2".to_string(),
            Principal::new("svc-a2", UserRole::User, TenantId::new("acme").unwrap()),
        );
        api_keys.insert(
            "tracer-key-b".to_string(),
            Principal::new("svc-b", UserRole::User, TenantId::new("globex").unwrap()),
        );
        api_keys.insert(
            "tracer-key-ops".to_string(),
            Principal::new("ops", UserRole::Admin, TenantId::new("ops-tenant").unwrap()),
        );
        api_keys.insert(
            "tracer-key-empty".to_string(),
            Principal::new(
                "svc-empty",
                UserRole::User,
                TenantId::new("empty-tenant").unwrap(),
            ),
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

        async fn submit(app: &axum::Router, key: &str, thread: &str) -> String {
            let body = serde_json::to_vec(&serde_json::json!({
                "assistant_id": "tenant-wf",
                "thread_id": thread,
                "input": {}
            }))
            .unwrap();
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/v1/runs")
                        .header("content-type", "application/json")
                        .header("x-api-key", key)
                        .body(Body::from(body))
                        .expect("request builds"),
                )
                .await
                .expect("router responds");
            assert_eq!(
                response.status(),
                StatusCode::ACCEPTED,
                "submit on {thread}"
            );
            let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("read submit body");
            let json: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON");
            json["run_id"].as_str().expect("run_id string").to_string()
        }

        async fn list(app: &axum::Router, key: &str, uri: &str) -> (Vec<u8>, serde_json::Value) {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(uri)
                        .header("x-api-key", key)
                        .body(Body::empty())
                        .expect("request builds"),
                )
                .await
                .expect("router responds");
            assert_eq!(response.status(), StatusCode::OK, "GET {uri} with {key}");
            let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("read list body");
            let json: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON");
            (bytes.to_vec(), json)
        }

        fn thread_ids(page: &serde_json::Value) -> Vec<String> {
            let mut ids: Vec<String> = page["items"]
                .as_array()
                .expect("items array")
                .iter()
                .map(|item| item["thread_id"].as_str().expect("thread_id").to_string())
                .collect();
            ids.sort();
            ids
        }

        // Two acme runs from two DIFFERENT acme keys (edge PLAT-07/adjacency), one
        // globex run. No worker runs, so every run stays Queued.
        let run_a1 = submit(&app, "tracer-key-a", "list-a-1").await;
        let run_a2 = submit(&app, "tracer-key-a2", "list-a-2").await;
        let run_b1 = submit(&app, "tracer-key-b", "list-b-1").await;

        // (a) A user key lists exactly its own tenant's runs, including the run
        // submitted by the OTHER acme key.
        let (_, page_a) = list(&app, "tracer-key-a", "/v1/runs").await;
        assert_eq!(thread_ids(&page_a), vec!["list-a-1", "list-a-2"]);
        assert!(page_a["next_cursor"].is_null());

        let (_, page_b) = list(&app, "tracer-key-b", "/v1/runs").await;
        assert_eq!(thread_ids(&page_b), vec!["list-b-1"]);
        assert_eq!(page_b["items"][0]["run_id"].as_str(), Some(run_b1.as_str()));

        // (b) An Admin key of an unrelated tenant lists every run (D-11).
        let (_, page_ops) = list(&app, "tracer-key-ops", "/v1/runs").await;
        assert_eq!(
            thread_ids(&page_ops),
            vec!["list-a-1", "list-a-2", "list-b-1"]
        );

        // (c) Edge PLAT-07/empty: a tenant with no runs is 200 with EXACTLY
        // `{"items":[],"next_cursor":null}`, never 404.
        let (empty_bytes, _) = list(&app, "tracer-key-empty", "/v1/runs").await;
        assert_eq!(
            String::from_utf8(empty_bytes).expect("utf8 body"),
            r#"{"items":[],"next_cursor":null}"#
        );

        // (d) D-14: another tenant's thread id under a user scope is an empty page.
        let (_, cross) = list(&app, "tracer-key-b", "/v1/runs?thread_id=list-a-1").await;
        assert!(cross["items"].as_array().expect("items").is_empty());
        assert!(cross["next_cursor"].is_null());

        // (e) D-02: `?tenant_id=` is neither a filter nor an override.
        let (_, spoofed) = list(&app, "tracer-key-a", "/v1/runs?tenant_id=globex").await;
        assert_eq!(thread_ids(&spoofed), vec!["list-a-1", "list-a-2"]);

        // (f) A `?limit=1` keyset walk under a user scope yields both acme runs, one
        // per page, then `next_cursor: null` -- the tenant predicate is inside the
        // SQL, so pages stay full and the cursor stays correct (D-12, T-40-09).
        let (_, first) = list(&app, "tracer-key-a", "/v1/runs?limit=1").await;
        assert_eq!(first["items"].as_array().expect("items").len(), 1);
        let cursor = first["next_cursor"]
            .as_str()
            .expect("first page has a cursor");
        let (_, second) = list(
            &app,
            "tracer-key-a",
            &format!("/v1/runs?limit=1&cursor={cursor}"),
        )
        .await;
        assert_eq!(second["items"].as_array().expect("items").len(), 1);
        assert!(second["next_cursor"].is_null(), "second page is the last");
        let mut walked = vec![
            first["items"][0]["run_id"]
                .as_str()
                .expect("run_id")
                .to_string(),
            second["items"][0]["run_id"]
                .as_str()
                .expect("run_id")
                .to_string(),
        ];
        walked.sort();
        let mut expected = vec![run_a1, run_a2];
        expected.sort();
        assert_eq!(
            walked, expected,
            "the walk covers exactly the two acme runs"
        );

        cleanup(&repo_path);
    })
    .await
    .expect("tenant_scoped_run_list_e2e did not hang");
}

/// Phase 40's cross-tenant cancel proof (D-13, T-40-19, PLAT-07) over a real on-disk
/// `SqliteRunRepository` through the real `run_router` and `RunSubmissionService`: key b's
/// `POST /v1/runs/{id}/cancel` on key a's run is the missing-run 404 and writes NOTHING --
/// the row's `cancel_requested` stays false -- while key a's own cancel then answers 202
/// and sets the flag. A foreign cancel is a cross-tenant mutation, not just a leak, so the
/// visibility gate must answer before `RunSubmissionPort::cancel` is ever reached.
#[tokio::test(flavor = "multi_thread")]
async fn cross_tenant_cancel_is_a_404_and_writes_no_cancel_flag() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (repo_path, repo_url) = temp_sqlite_url("cross-tenant-cancel");
        let repository: Arc<dyn RunRepositoryPort> =
            Arc::new(SqliteRunRepository::new(&repo_url).await.unwrap());
        let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("cancel-wf", build_chain_graph(1)));
        let submission: Arc<dyn RunSubmissionPort> = Arc::new(RunSubmissionService::new(
            repository.clone(),
            queue.clone(),
            resolver.clone(),
        ));

        let mut api_keys = HashMap::new();
        api_keys.insert(
            "cancel-key-a".to_string(),
            Principal::new("svc-a", UserRole::User, TenantId::new("acme").unwrap()),
        );
        api_keys.insert(
            "cancel-key-b".to_string(),
            Principal::new("svc-b", UserRole::User, TenantId::new("globex").unwrap()),
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

        // Key a submits; no worker runs, so the run stays Queued (non-terminal) and a
        // cancel is admissible for whoever may see it.
        let submit_body = serde_json::to_vec(&serde_json::json!({
            "assistant_id": "cancel-wf",
            "thread_id": "cancel-a-1",
            "input": {}
        }))
        .unwrap();
        let submit_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/runs")
                    .header("content-type", "application/json")
                    .header("x-api-key", "cancel-key-a")
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

        // Key b (globex) cancelling acme's run: the missing-run 404, and the row is
        // untouched.
        let foreign_cancel = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/runs/{run_id}/cancel"))
                    .header("x-api-key", "cancel-key-b")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(
            foreign_cancel.status(),
            StatusCode::NOT_FOUND,
            "a foreign tenant's cancel must be the missing-run 404 (D-13, PLAT-07)"
        );
        let after_foreign = repository
            .get(&run_id)
            .await
            .expect("repository read succeeds")
            .expect("run exists");
        assert!(
            !after_foreign.cancel_requested,
            "a foreign cancel must never reach RunSubmissionPort::cancel or write the flag"
        );
        assert_eq!(after_foreign.status, RunStatus::Queued);

        // Key a (the owner) cancels: 202, and the flag is now set.
        let owner_cancel = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/runs/{run_id}/cancel"))
                    .header("x-api-key", "cancel-key-a")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(owner_cancel.status(), StatusCode::ACCEPTED);
        let after_owner = repository
            .get(&run_id)
            .await
            .expect("repository read succeeds")
            .expect("run exists");
        assert!(
            after_owner.cancel_requested,
            "the owner's cancel must set cancel_requested on the row"
        );

        cleanup(&repo_path);
    })
    .await
    .expect("cross_tenant_cancel_is_a_404_and_writes_no_cancel_flag did not hang");
}
