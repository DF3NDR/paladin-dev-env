//! End-to-end tests for the `paladin-server` HTTP API (Milestone 12, Epic 7).
//!
//! Assembles the full application exactly as the binary does — the `/v1` agent router +
//! OpenAPI docs + cross-cutting layers, with authentication enabled — serves it in-process on
//! an ephemeral port, and drives it over real HTTP with `reqwest`. Hermetic: agents are backed
//! by [`MockLlmAdapter`], so no network or provider keys are needed and it runs in normal CI.
#![cfg(feature = "web-server")]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use paladin::MockLlmAdapter;
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin::infrastructure::web::openapi::{build_openapi, docs_router};
use paladin::infrastructure::web::{
    AgentApiState, AgentEntry, AgentRegistry, HttpLayersConfig, Principal, agent_router,
    with_http_layers,
};
use paladin_core::platform::container::principal::TenantId;
use paladin_core::platform::container::user::UserRole;
use paladin_ports::output::llm_port::LlmPort;
use paladin_ports::output::paladin_executor_port::PaladinExecutorPort;
use paladin_ports::output::streaming_executor_port::StreamingExecutorPort;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

const ADMIN_KEY: &str = "sk-admin";
const USER_KEY: &str = "sk-user";

/// A running test server: its base URL + a handle to shut it down.
struct TestServer {
    base: String,
    client: reqwest::Client,
    shutdown: oneshot::Sender<()>,
    handle: JoinHandle<()>,
}

impl TestServer {
    async fn stop(self) {
        let _ = self.shutdown.send(());
        let _ = self.handle.await;
    }
}

/// Build `(paladin, executor, streamer)` for a mock-backed agent named `id`.
async fn mock_agent(
    id: &str,
) -> (
    Arc<paladin_core::platform::container::paladin::Paladin>,
    Arc<dyn PaladinExecutorPort>,
    Arc<dyn StreamingExecutorPort>,
) {
    let llm: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new());
    let breaker = Arc::new(CircuitBreaker::new(5, 2, Duration::from_secs(30)));
    let service = Arc::new(PaladinExecutionService::new(
        Arc::clone(&llm),
        breaker,
        None,
        None,
    ));
    let executor: Arc<dyn PaladinExecutorPort> = service.clone();
    let streamer: Arc<dyn StreamingExecutorPort> = service;
    let paladin = PaladinBuilder::new(llm)
        .name(id)
        .system_prompt("You are an e2e test agent.")
        .model("mock")
        .build()
        .await
        .expect("agent builds");
    (Arc::new(paladin), executor, streamer)
}

/// State with two agents (an open one and an admin-only one) and auth enabled (admin + user
/// keys), assembled like the binary.
async fn build_state() -> AgentApiState {
    let registry = AgentRegistry::new();

    let (paladin, executor, streamer) = mock_agent("researcher").await;
    registry.insert_with_streaming("researcher", paladin, executor, Some(streamer));

    // An agent restricted to admins, for the per-agent authorization check.
    let (paladin, executor, streamer) = mock_agent("classified").await;
    registry.insert_entry(
        "classified".to_string(),
        AgentEntry {
            paladin,
            executor,
            streamer: Some(streamer),
            timeout_secs: None,
            allowed_roles: vec![UserRole::Admin],
        },
    );

    let mut api_keys = HashMap::new();
    api_keys.insert(
        ADMIN_KEY.to_string(),
        Principal::new(
            "admin",
            UserRole::Admin,
            TenantId::new("e2e-tenant").unwrap(),
        ),
    );
    api_keys.insert(
        USER_KEY.to_string(),
        Principal::new("user", UserRole::User, TenantId::new("e2e-tenant").unwrap()),
    );
    AgentApiState::new(Arc::new(registry)).with_auth(
        paladin::infrastructure::web::AgentAuthConfig {
            enabled: true,
            api_keys,
            token_verifier: None,
            bearer_tenant: None,
        },
    )
}

/// Serve the fully-assembled app on an ephemeral port.
async fn serve() -> TestServer {
    let state = build_state().await;
    let spec = build_openapi(state.clone());
    let app = with_http_layers(
        agent_router(state).merge(docs_router(spec)),
        &HttpLayersConfig::default(),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    let (shutdown, shutdown_rx) = oneshot::channel::<()>();
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await;
    });
    TestServer {
        base: format!("http://{addr}"),
        client: reqwest::Client::new(),
        shutdown,
        handle,
    }
}

#[tokio::test]
async fn auth_is_enforced() {
    let s = serve().await;
    let c = &s.client;

    // No credential → 401 (nested envelope).
    let resp = c.get(format!("{}/v1/agents", s.base)).send().await.unwrap();
    assert_eq!(resp.status().as_u16(), 401);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["error"]["code"], "unauthorized");

    // Valid admin key → 200.
    let resp = c
        .get(format!("{}/v1/agents", s.base))
        .header("x-api-key", ADMIN_KEY)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);

    // Non-admin registering an agent → 403 (admin gate runs before provisioning).
    let resp = c
        .post(format!("{}/v1/agents", s.base))
        .header("x-api-key", USER_KEY)
        .json(&serde_json::json!({
            "id": "x", "name": "X", "model": "gpt-4", "system_prompt": "p"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 403);

    // User invoking an admin-only agent → 403.
    let resp = c
        .post(format!("{}/v1/agents/classified/execute", s.base))
        .header("x-api-key", USER_KEY)
        .json(&serde_json::json!({ "input": "hi" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 403);
    assert_eq!(
        resp.json::<serde_json::Value>().await.unwrap()["error"]["code"],
        "forbidden"
    );

    s.stop().await;
}

#[tokio::test]
async fn execution_buffered_streaming_and_jobs() {
    let s = serve().await;
    let c = &s.client;

    // Buffered execute.
    let resp = c
        .post(format!("{}/v1/agents/researcher/execute", s.base))
        .header("x-api-key", USER_KEY)
        .json(&serde_json::json!({ "input": "hello" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    assert!(resp.json::<serde_json::Value>().await.unwrap()["output"].is_string());

    // Streaming (SSE).
    let sse = c
        .post(format!("{}/v1/agents/researcher/execute/stream", s.base))
        .header("x-api-key", USER_KEY)
        .json(&serde_json::json!({ "input": "hello" }))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(sse.contains("event: chunk"), "sse: {sse}");
    assert!(sse.contains("event: done"), "sse: {sse}");

    // Async job → poll to completion.
    let job: serde_json::Value = c
        .post(format!("{}/v1/agents/researcher/jobs", s.base))
        .header("x-api-key", USER_KEY)
        .json(&serde_json::json!({ "input": "hello" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let job_id = job["job_id"].as_str().expect("job_id").to_string();

    let mut completed = false;
    for _ in 0..50 {
        let rec: serde_json::Value = c
            .get(format!("{}/v1/agents/researcher/jobs/{job_id}", s.base))
            .header("x-api-key", USER_KEY)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        if rec["status"] != "running" {
            assert_eq!(rec["status"], "completed", "record: {rec}");
            completed = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(completed, "job did not complete");

    s.stop().await;
}

#[tokio::test]
async fn ops_health_errors_and_docs() {
    let s = serve().await;
    let c = &s.client;

    // Health/readiness open without a credential.
    for path in ["/health", "/ready"] {
        let resp = c.get(format!("{}{path}", s.base)).send().await.unwrap();
        assert_eq!(resp.status().as_u16(), 200, "{path}");
    }

    // Unknown agent → 404 nested envelope.
    let resp = c
        .get(format!("{}/v1/agents/ghost", s.base))
        .header("x-api-key", ADMIN_KEY)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 404);
    assert_eq!(
        resp.json::<serde_json::Value>().await.unwrap()["error"]["code"],
        "not_found"
    );

    // OpenAPI spec + Swagger UI.
    let spec: serde_json::Value = c
        .get(format!("{}/openapi.json", s.base))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(spec["paths"].get("/v1/agents/{id}/execute").is_some());
    assert!(
        spec["components"]["securitySchemes"]
            .get("api_key")
            .is_some()
    );

    let docs = c.get(format!("{}/docs/", s.base)).send().await.unwrap();
    assert_eq!(docs.status().as_u16(), 200);

    s.stop().await;
}
