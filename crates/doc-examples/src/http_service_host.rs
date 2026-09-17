//! Compiled example for `docs/src/deployment-topologies/http-service-host.md`
//! (Milestone 12).
//!
//! Pulled into the page via mdBook `{{#include}}`, so `cargo check
//! -p paladin-doc-examples` keeps it matching the current Paladin API. It shows embedding
//! Paladin's **shipped** agent, thread and run routers (the same three routers, in the same
//! merge order, `paladin-server` runs) inside your own `axum` process — you don't write the
//! handlers. The example compiles in full (including the `axum::serve` bind) because `cargo
//! check` never runs it.
#![allow(unused_variables, unused_imports, dead_code)]

// ANCHOR: http_host
use std::sync::Arc;
use std::time::Duration;

use paladin::MockLlmAdapter;
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin::infrastructure::web::{
    AgentApiState, AgentRegistry, HttpLayersConfig, RunApiState, ThreadApiState, agent_router,
    run_router, thread_router, with_http_layers,
};
use paladin_ports::output::llm_port::LlmPort;
use paladin_ports::output::paladin_executor_port::PaladinExecutorPort;
use paladin_ports::output::streaming_executor_port::StreamingExecutorPort;

/// Build a resident agent registry and serve Paladin's shipped agent, thread and run routers —
/// `/v1/agents/…` (buffered, streaming, async jobs, discovery, registration), `/v1/threads/…`
/// and `/v1/runs/…` (both `501 not_implemented` until a waypoint/run store is wired, the same
/// off-by-default behavior `paladin-server` has out of the box) — plus `/health` and `/ready`,
/// inside your own `axum` process. This is the same router assembly, in the same merge order,
/// the `paladin-server` binary uses, so the endpoints are provided for you rather than
/// hand-written.
pub async fn serve_agents() -> Result<(), Box<dyn std::error::Error>> {
    let llm: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new());
    let breaker = Arc::new(CircuitBreaker::new(5, 2, Duration::from_secs(30)));

    // One execution service backs both the buffered and streaming handles.
    let service = Arc::new(PaladinExecutionService::new(
        llm.clone(),
        breaker,
        None,
        None,
    ));
    let executor: Arc<dyn PaladinExecutorPort> = service.clone();
    let streamer: Arc<dyn StreamingExecutorPort> = service;

    let paladin = PaladinBuilder::new(llm)
        .name("researcher")
        .system_prompt("You research topics thoroughly.")
        .build()
        .await?;

    // Resident agents, keyed by id, shared across concurrent requests.
    let registry = AgentRegistry::new();
    registry.insert_with_streaming("researcher", Arc::new(paladin), executor, Some(streamer));

    // `agent_router` mounts the agent API under `/v1` plus the unversioned health probes;
    // `thread_router`/`run_router` mount `/v1/threads/…`/`/v1/runs/…` — merged ALONGSIDE
    // `agent_router`'s output, never inside it, exactly as `paladin-server` does. Both states
    // are unwired here (no waypoint/run store), so their routes answer `501` rather than
    // being absent; wire a store via `ThreadApiState::with_waypoints`/`RunApiState::with_repository`
    // (and friends) to make them live. `with_http_layers` adds the cross-cutting layers
    // (request-id, CORS, body limit, timeout, rate limit). Auth is open here (the library
    // default); `paladin-server` enables it from config. To also serve the OpenAPI spec +
    // Swagger UI, merge `openapi::docs_router`.
    let state = AgentApiState::new(Arc::new(registry));
    let thread_state = ThreadApiState::new();
    let run_state = RunApiState::new();
    let routes = agent_router(state)
        .merge(thread_router(thread_state))
        .merge(run_router(run_state));
    let app = with_http_layers(routes, &HttpLayersConfig::default());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
// ANCHOR_END: http_host
