//! Web server adapters for the Paladin AI orchestration framework.
//!
//! Provides HTTP server components built with `axum`, including user management
//! REST endpoints and content-delivery routes/adapters.

#![warn(missing_docs)]

/// Web-facing adapter integrations.
#[allow(missing_docs)]
pub mod adapters;
/// Authentication & authorization for the agent API.
pub mod agent_auth;
/// Agent-execution HTTP controller (wire types, state, handlers, router).
pub mod agent_controller;
/// Resident agent registry and provisioning seam for the HTTP service-host topology.
pub mod agent_registry;
/// Application router composition for the user REST API.
pub mod app;
/// Authentication and RBAC middleware for the user REST API.
pub mod auth_middleware;
/// Content-delivery controller handlers (axum).
pub mod delivery_controller;
/// Unified API error model (structured JSON error envelope).
pub mod error;
/// Liveness and readiness endpoints.
pub mod health;
/// Cross-cutting HTTP layers (CORS, body limit, timeout, rate limiting).
pub mod http_layers;
/// In-memory async job store for fire-and-poll execution.
pub mod job_store;
/// OpenAPI spec assembly and interactive (Swagger UI) docs serving.
pub mod openapi;
/// Request-logging middleware with request-id correlation.
pub mod request_log;
/// Thread inspection/resume/history HTTP controller (wire types, state, handlers, router).
pub mod thread_controller;
/// Execution timeout policy and resolution.
pub mod timeout;
/// User management controller handlers.
#[allow(missing_docs)]
pub mod user_controller;

pub use agent_auth::{AgentAuthConfig, Principal};
pub use agent_controller::{
    AgentApiState, AgentSummary, ExecuteRequest, ExecuteResponse, agent_router,
};
pub use agent_registry::{
    AgentEntry, AgentProvisioner, AgentRegistry, AgentSpec, ProvisionError, ProvisionedAgent,
};
pub use error::ApiError;
pub use http_layers::{HttpLayersConfig, RateLimitConfig, with_http_layers};
pub use job_store::{JobRecord, JobStatus, JobStore};
pub use thread_controller::{ThreadApiState, thread_router};
pub use timeout::TimeoutPolicy;
