//! End-to-end tracer: a run submitted through `POST /v1/runs` is persisted,
//! enqueued, executed by a worker on the real engine, and reaches
//! `Completed` -- with `GET /v1/runs/{run_id}` reporting each status (Task
//! 3). Assembles InMemory repository + InMemory queue + a real `WarEngine`
//! over a two-graph in-test registry with `MockLlmAdapter`, mounts
//! `run_router` over a `RunApiState` wired to the facade services, and
//! drives the whole behavior list through `tower::util::oneshot`.
//!
//! Lives in the facade crate (D-11): this is the only crate that sees the
//! engine, the queue and the router at once.

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use paladin_battalion::engine::{
    EngineLimits, InputMapping, NodeContext, NodeSpec, StateNode, StateNodeError, WarEngine,
    WarGraph,
};
use paladin_core::base::entity::node::Node;
use paladin_core::platform::container::battlefield::{
    Battlefield, BattlefieldSchema, DispatchRule, FieldName, FieldSpec,
};
use paladin_core::platform::container::directive::Directive;
use paladin_core::platform::container::execution_result::PaladinResult;
use paladin_core::platform::container::paladin::{Paladin, PaladinData};
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::run::RunId;
use paladin_core::platform::container::waypoint::NodeId;
use paladin_llm::mock::MockLlmAdapter;
use paladin_ports::input::run_submission_port::RunSubmissionPort;
use paladin_ports::output::llm_port::LlmPort;
use paladin_ports::output::paladin_port::{PaladinPort, PaladinStream};
use paladin_ports::output::run_queue_port::RunQueuePort;
use paladin_ports::output::run_repository_port::RunRepositoryPort;
use paladin_storage::run::in_memory::InMemoryRunRepository;
use paladin_storage::run_queue::in_memory::InMemoryRunQueue;
use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;
use paladin_web::run_controller::{RunApiState, run_router};

use crate::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use crate::application::services::run::resolver::{AssistantResolver, CodeWorkflowResolver};
use crate::application::services::run::submission::RunSubmissionService;
use crate::application::services::run::worker::RunWorkerPool;
use crate::infrastructure::resilience::circuit_breaker::CircuitBreaker;

/// A `StateNode` that always fails, for the "a run whose graph fails ends
/// `Failed`" behavior -- no LLM involved, so it cannot be affected by the
/// success graph's `MockLlmAdapter` configuration.
struct AlwaysFailingNode;

#[async_trait::async_trait]
impl StateNode for AlwaysFailingNode {
    async fn run(
        &self,
        _state: &Battlefield,
        _ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        Err(StateNodeError("deliberate tracer failure".to_string()))
    }
}

/// Adapts a [`PaladinExecutionService`] to the engine-facing [`PaladinPort`]
/// seam `WarEngine::new` expects. No production adapter of this shape
/// exists elsewhere in the tree yet (the two are always driven separately
/// today); this tracer test needs a real, LLM-backed `PaladinPort`, so it
/// is the minimal one, local to this test module.
struct PaladinPortAdapter(Arc<PaladinExecutionService>);

#[async_trait::async_trait]
impl PaladinPort for PaladinPortAdapter {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        self.0.execute(paladin, input).await
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinStream, PaladinError> {
        unreachable!("this tracer's WarGraph never streams")
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

fn make_paladin(name: &str) -> Paladin {
    let data = PaladinData {
        name: name.to_string(),
        ..Default::default()
    };
    Node::new(data, Some(name.to_string()))
}

/// A single-node, single-superstep workflow: a Paladin renders its input,
/// calls the (mocked) LLM, and writes its output into `summary`. Completes
/// after the first superstep (no outgoing edges).
fn build_success_graph() -> Arc<WarGraph> {
    let schema = BattlefieldSchema::new(vec![FieldSpec::new(
        FieldName::new("summary").unwrap(),
        DispatchRule::LastWrite,
        None,
        false,
    )]);
    let mut graph = WarGraph::new(schema, EngineLimits::default());
    let node_id = NodeId::new("summarizer");
    graph.add_node(
        node_id.clone(),
        NodeSpec::paladin(
            make_paladin("summarizer"),
            InputMapping::new("summarize this"),
            FieldName::new("summary").unwrap(),
        ),
    );
    graph.add_entry(node_id);
    Arc::new(graph)
}

/// A single-node workflow whose only node always fails -- proves a graph
/// failure ends the run `Failed` with the engine's error recorded, not a
/// panic.
fn build_failing_graph() -> Arc<WarGraph> {
    let mut graph = WarGraph::new(BattlefieldSchema::new(vec![]), EngineLimits::default());
    let node_id = NodeId::new("failer");
    graph.add_node(
        node_id.clone(),
        NodeSpec::Function(Arc::new(AlwaysFailingNode)),
    );
    graph.add_entry(node_id);
    Arc::new(graph)
}

/// The assembled tracer harness: an HTTP router wired to the facade
/// services, a worker pool sharing the same repository/queue/resolver, and
/// the queue itself (kept for depth assertions the HTTP surface does not
/// expose in this slice).
struct Harness {
    app: axum::Router,
    worker: RunWorkerPool<InMemoryWaypointStore>,
    queue: Arc<dyn RunQueuePort>,
}

fn build_harness() -> Harness {
    let waypoints = Arc::new(InMemoryWaypointStore::new());
    let llm: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new().with_response("a short summary"));
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let paladin_port: Arc<dyn PaladinPort> = Arc::new(PaladinPortAdapter(Arc::new(
        PaladinExecutionService::new(llm, circuit_breaker, None, None),
    )));
    let engine = Arc::new(WarEngine::new(paladin_port, waypoints));

    let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
    let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
    let resolver: Arc<dyn AssistantResolver> = Arc::new(
        CodeWorkflowResolver::new()
            .register("summarizer-workflow", build_success_graph())
            .register("failing-workflow", build_failing_graph()),
    );

    let submission: Arc<dyn RunSubmissionPort> = Arc::new(RunSubmissionService::new(
        repository.clone(),
        queue.clone(),
        resolver.clone(),
    ));

    let state = RunApiState::new()
        .with_submission(submission)
        .with_repository(repository.clone());
    let app = run_router(state);

    let worker = RunWorkerPool::new(
        engine,
        repository,
        queue.clone(),
        resolver,
        Duration::from_secs(30),
    );

    Harness { app, worker, queue }
}

async fn body_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body reads");
    serde_json::from_slice(&bytes).expect("response body is valid JSON")
}

#[tokio::test]
async fn submit_run_over_http_reaches_completed_via_worker() {
    let harness = build_harness();

    let submit_body = serde_json::to_vec(&serde_json::json!({
        "assistant_id": "summarizer-workflow",
        "input": {}
    }))
    .expect("request body serializes");

    let response = harness
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/runs")
                .header("content-type", "application/json")
                .body(Body::from(submit_body))
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let submitted = body_json(response).await;
    let run_id = submitted["run_id"]
        .as_str()
        .expect("run_id is a string")
        .to_string();

    // Immediately after submit: Queued, queue depth 1.
    assert_eq!(harness.queue.depth().await.unwrap(), 1);
    let response = harness
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/v1/runs/{run_id}"))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    assert_eq!(response.status(), StatusCode::OK);
    let run_body = body_json(response).await;
    assert_eq!(run_body["status"], "queued");

    // After one worker iteration: Completed, finished_at set, depth 0.
    let processed = harness
        .worker
        .run_once()
        .await
        .expect("worker iteration succeeds");
    assert!(processed);
    assert_eq!(harness.queue.depth().await.unwrap(), 0);

    let response = harness
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/v1/runs/{run_id}"))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    assert_eq!(response.status(), StatusCode::OK);
    let run_body = body_json(response).await;
    assert_eq!(run_body["status"], "completed");
    assert!(run_body["finished_at"].is_string());
}

#[tokio::test]
async fn get_unknown_run_returns_404_with_not_found_envelope() {
    let harness = build_harness();

    let response = harness
        .app
        .oneshot(
            Request::builder()
                .uri(format!("/v1/runs/{}", RunId::new_v7()))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = body_json(response).await;
    assert_eq!(body["error"]["code"], "not_found");
}

#[tokio::test]
async fn post_runs_without_submission_port_returns_501_naming_the_config() {
    let state = RunApiState::new();
    let app = run_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/runs")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({ "assistant_id": "a1" })).unwrap(),
                ))
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    let body = body_json(response).await;
    assert_eq!(body["error"]["code"], "not_implemented");
}

#[tokio::test]
async fn run_whose_graph_fails_ends_failed_not_a_panic() {
    let harness = build_harness();

    let submit_body = serde_json::to_vec(&serde_json::json!({
        "assistant_id": "failing-workflow",
        "input": {}
    }))
    .expect("request body serializes");

    let response = harness
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/runs")
                .header("content-type", "application/json")
                .body(Body::from(submit_body))
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let submitted = body_json(response).await;
    let run_id = submitted["run_id"]
        .as_str()
        .expect("run_id is a string")
        .to_string();

    let processed = harness
        .worker
        .run_once()
        .await
        .expect("worker iteration succeeds");
    assert!(processed);

    let response = harness
        .app
        .oneshot(
            Request::builder()
                .uri(format!("/v1/runs/{run_id}"))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    assert_eq!(response.status(), StatusCode::OK);
    let run_body = body_json(response).await;
    assert_eq!(run_body["status"], "failed");
    assert!(run_body["error"].is_string());
}
