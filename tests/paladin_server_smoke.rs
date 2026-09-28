//! Boot smoke test for the HTTP service host (Milestone 12, Epic 2).
//!
//! Builds a hermetic agent backed by [`MockLlmAdapter`] (no network / API keys), serves
//! the agent router on an ephemeral port via `axum::serve`, and drives it over real HTTP
//! with `reqwest` — mirroring what the `paladin-server` binary does. Also asserts that
//! the server shuts down cleanly when its graceful-shutdown signal fires.
#![cfg(feature = "web-server")]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use paladin::MockLlmAdapter;
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin::infrastructure::web::{
    AgentApiState, AgentAuthConfig, AgentRegistry, HttpLayersConfig, Principal, agent_router,
    with_http_layers,
};
use paladin_core::platform::container::principal::TenantId;
use paladin_core::platform::container::user::UserRole;
use paladin_ports::output::llm_port::LlmPort;
use paladin_ports::output::paladin_executor_port::PaladinExecutorPort;
use paladin_ports::output::streaming_executor_port::StreamingExecutorPort;

/// Build an `AgentApiState` holding a single mock-backed, streaming-capable agent.
async fn state_with_mock_agent(id: &str) -> AgentApiState {
    let llm: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new());
    let breaker = Arc::new(CircuitBreaker::new(5, 2, Duration::from_secs(30)));
    // One service backs both the buffered and streaming handles.
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
        .system_prompt("You are a smoke-test agent.")
        .model("mock")
        .build()
        .await
        .expect("agent builds");

    let registry = AgentRegistry::new();
    registry.insert_with_streaming(id, Arc::new(paladin), executor, Some(streamer));
    AgentApiState::new(Arc::new(registry))
}

#[tokio::test]
async fn server_boots_serves_agents_and_shuts_down_cleanly() {
    // Wrap with the cross-cutting layers, mirroring the `paladin-server` binary.
    let app = with_http_layers(
        agent_router(state_with_mock_agent("researcher").await),
        &HttpLayersConfig::default(),
    );

    // Bind an ephemeral port and serve with a graceful-shutdown trigger.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await
            .expect("server runs");
    });

    let base = format!("http://{addr}");
    let client = reqwest::Client::new();

    // GET /agents → 200 with the registered agent.
    let resp = client
        .get(format!("{base}/v1/agents"))
        .send()
        .await
        .expect("GET /agents");
    assert_eq!(resp.status().as_u16(), 200);
    // Cross-cutting layer: every response carries a request-id.
    assert!(
        resp.headers().contains_key("x-request-id"),
        "expected x-request-id header"
    );
    let agents: serde_json::Value = resp.json().await.expect("agents json");
    assert_eq!(agents.as_array().map(|a| a.len()), Some(1));
    assert_eq!(agents[0]["id"], "researcher");

    // GET /health and /ready respond with the documented shapes.
    let health: serde_json::Value = client
        .get(format!("{base}/health"))
        .send()
        .await
        .expect("GET /health")
        .json()
        .await
        .expect("health json");
    assert_eq!(health["status"], "ok");

    let ready: serde_json::Value = client
        .get(format!("{base}/ready"))
        .send()
        .await
        .expect("GET /ready")
        .json()
        .await
        .expect("ready json");
    assert_eq!(ready["status"], "ready");
    assert_eq!(ready["agents"], 1);

    // An error response uses the unified nested envelope.
    let resp = client
        .get(format!("{base}/v1/agents/ghost"))
        .send()
        .await
        .expect("GET unknown agent");
    assert_eq!(resp.status().as_u16(), 404);
    let err: serde_json::Value = resp.json().await.expect("error json");
    assert_eq!(err["error"]["code"], "not_found");
    assert!(err["error"]["message"].is_string());

    // POST /agents/researcher/execute → 200 with an output string.
    let resp = client
        .post(format!("{base}/v1/agents/researcher/execute"))
        .json(&serde_json::json!({ "input": "hello" }))
        .send()
        .await
        .expect("POST execute");
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await.expect("execute json");
    assert!(
        body["output"].is_string(),
        "expected an output string, got {body}"
    );

    // Unknown agent → 404.
    let resp = client
        .post(format!("{base}/v1/agents/ghost/execute"))
        .json(&serde_json::json!({ "input": "hi" }))
        .send()
        .await
        .expect("POST unknown");
    assert_eq!(resp.status().as_u16(), 404);

    // POST /agents/researcher/execute/stream → SSE chunk(s) then a done event.
    let resp = client
        .post(format!("{base}/v1/agents/researcher/execute/stream"))
        .json(&serde_json::json!({ "input": "hello" }))
        .send()
        .await
        .expect("POST stream");
    assert_eq!(resp.status().as_u16(), 200);
    let sse = resp.text().await.expect("stream body");
    assert!(sse.contains("event: chunk"), "expected chunk events: {sse}");
    assert!(sse.contains("event: done"), "expected a done event: {sse}");

    // POST /agents/researcher/jobs → 202 + job_id, then poll to completion.
    let resp = client
        .post(format!("{base}/v1/agents/researcher/jobs"))
        .json(&serde_json::json!({ "input": "hello" }))
        .send()
        .await
        .expect("POST jobs");
    assert_eq!(resp.status().as_u16(), 202);
    let body: serde_json::Value = resp.json().await.expect("job json");
    let job_id = body["job_id"].as_str().expect("job_id").to_string();

    let mut completed = false;
    for _ in 0..50 {
        let resp = client
            .get(format!("{base}/v1/agents/researcher/jobs/{job_id}"))
            .send()
            .await
            .expect("GET job");
        assert_eq!(resp.status().as_u16(), 200);
        let record: serde_json::Value = resp.json().await.expect("job record");
        if record["status"] != "running" {
            assert_eq!(record["status"], "completed", "job record: {record}");
            assert!(record["result"]["output"].is_string());
            completed = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(completed, "job did not complete in time");

    // Trigger graceful shutdown; the server task must complete cleanly.
    shutdown_tx.send(()).expect("send shutdown");
    server.await.expect("server task joins after shutdown");
}

#[tokio::test]
async fn server_enforces_authentication_when_enabled() {
    // State with auth enabled + one admin API key, mirroring `paladin-server`'s wiring.
    let mut api_keys = HashMap::new();
    api_keys.insert(
        "sk-smoke-key".to_string(),
        Principal::new(
            "smoke",
            UserRole::Admin,
            TenantId::new("smoke-tenant").unwrap(),
        ),
    );
    let state = state_with_mock_agent("researcher")
        .await
        .with_auth(AgentAuthConfig {
            enabled: true,
            api_keys,
            token_verifier: None,
            bearer_tenant: None,
        });
    let app = with_http_layers(agent_router(state), &HttpLayersConfig::default());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await
            .expect("server runs");
    });

    let base = format!("http://{addr}");
    let client = reqwest::Client::new();

    // No credential → 401 (nested envelope).
    let resp = client
        .get(format!("{base}/v1/agents"))
        .send()
        .await
        .expect("GET /agents no key");
    assert_eq!(resp.status().as_u16(), 401);
    let err: serde_json::Value = resp.json().await.expect("error json");
    assert_eq!(err["error"]["code"], "unauthorized");

    // Valid API key → 200.
    let resp = client
        .get(format!("{base}/v1/agents"))
        .header("x-api-key", "sk-smoke-key")
        .send()
        .await
        .expect("GET /agents with key");
    assert_eq!(resp.status().as_u16(), 200);

    // Execute with the key → 200.
    let resp = client
        .post(format!("{base}/v1/agents/researcher/execute"))
        .header("x-api-key", "sk-smoke-key")
        .json(&serde_json::json!({ "input": "hello" }))
        .send()
        .await
        .expect("POST execute with key");
    assert_eq!(resp.status().as_u16(), 200);

    // Execute without the key → 401.
    let resp = client
        .post(format!("{base}/v1/agents/researcher/execute"))
        .json(&serde_json::json!({ "input": "hello" }))
        .send()
        .await
        .expect("POST execute no key");
    assert_eq!(resp.status().as_u16(), 401);

    // Health probe stays open without a credential.
    let resp = client
        .get(format!("{base}/health"))
        .send()
        .await
        .expect("GET /health");
    assert_eq!(resp.status().as_u16(), 200);

    shutdown_tx.send(()).expect("send shutdown");
    server.await.expect("server task joins after shutdown");
}

#[tokio::test]
async fn server_serves_openapi_spec_and_docs() {
    use paladin::infrastructure::web::openapi::{build_openapi, docs_router};

    let state = state_with_mock_agent("researcher").await;
    let spec = build_openapi(state.clone());
    // Mount the docs alongside the agent API, mirroring the binary when docs are enabled.
    let app = with_http_layers(
        agent_router(state).merge(docs_router(spec)),
        &HttpLayersConfig::default(),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await
            .expect("server runs");
    });

    let base = format!("http://{addr}");
    let client = reqwest::Client::new();

    // GET /openapi.json → 200 with the expected paths + security schemes.
    let resp = client
        .get(format!("{base}/openapi.json"))
        .send()
        .await
        .expect("GET /openapi.json");
    assert_eq!(resp.status().as_u16(), 200);
    let spec: serde_json::Value = resp.json().await.expect("spec json");
    assert_eq!(spec["info"]["title"], "Paladin Agent API");
    assert!(
        spec["paths"].get("/v1/agents/{id}/execute").is_some(),
        "spec missing /v1/agents/{{id}}/execute; paths: {:?}",
        spec["paths"]
            .as_object()
            .map(|o| o.keys().collect::<Vec<_>>())
    );
    let schemes = &spec["components"]["securitySchemes"];
    assert!(schemes.get("api_key").is_some(), "missing api_key scheme");
    assert!(
        schemes.get("bearer_token").is_some(),
        "missing bearer_token scheme"
    );

    // GET /docs/ → 200 (Swagger UI index).
    let resp = client
        .get(format!("{base}/docs/"))
        .send()
        .await
        .expect("GET /docs/");
    assert_eq!(resp.status().as_u16(), 200);

    shutdown_tx.send(()).expect("send shutdown");
    server.await.expect("server task joins after shutdown");
}
