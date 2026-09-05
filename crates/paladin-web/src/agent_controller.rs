//! Axum HTTP controller for the agent-execution API (the HTTP service-host topology).
//!
//! This module defines the wire types, shared state, handlers, and router for running
//! resident agents over HTTP:
//!
//! | Method & path | Description |
//! |---------------|-------------|
//! | `POST /agents/{id}/execute` | Run an agent and return its output |
//! | `POST /agents/{id}/execute/stream` | Run an agent, streaming output over SSE |
//! | `GET /agents` | List registered agents |
//! | `GET /agents/{id}` | Describe a single agent |
//! | `POST /agents` | Register an agent at runtime |
//! | `DELETE /agents/{id}` | Deregister an agent |
//! | `POST /agents/{id}/jobs` | Enqueue an async run (fire-and-poll) |
//! | `GET /agents/{id}/jobs/{job_id}` | Poll an async job's status/result |
//!
//! A success body is the serialized payload; failures use the unified
//! [`ApiError`](crate::error::ApiError) envelope
//! (`{ "error": { "code", "message", "details" } }`).

use std::sync::Arc;

use std::convert::Infallible;
use std::pin::Pin;
use std::time::Duration;

use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response, Sse, sse::Event},
};
use futures::Stream;
use serde::{Deserialize, Serialize};
use serde_json::json;

use paladin_core::platform::container::execution_result::{PaladinResult, StopReason};
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_ports::output::paladin_port::{PaladinStream, PaladinStreamChunk};

use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::agent_auth::{Principal, authorize_invoke, require_admin};
use crate::agent_registry::{AgentEntry, AgentProvisioner, AgentRegistry, AgentSpec};
use crate::error::{ApiError, ApiErrorBody};
use crate::job_store::{JobRecord, JobStore};
use crate::timeout::{TimeoutPolicy, resolve_timeout};

/// Shared state for the agent routes.
///
/// Cloned into every handler by `axum`. The `registry` is always present; the
/// `provisioner` is optional — when it is `None`, runtime registration
/// (`POST /agents`) fails closed rather than panicking, while execution and discovery
/// remain fully functional.
#[derive(Clone)]
pub struct AgentApiState {
    /// The resident agent registry shared across requests.
    pub registry: Arc<AgentRegistry>,
    /// Optional provisioner used to build agents for `POST /agents` (injected by Epic 2).
    pub provisioner: Option<Arc<dyn AgentProvisioner>>,
    /// Server-wide execution timeout policy (default + max).
    pub timeouts: TimeoutPolicy,
    /// In-memory store for async (`POST /agents/{id}/jobs`) execution.
    pub jobs: Arc<JobStore>,
    /// Authentication configuration (disabled = open; see `agent_auth`).
    pub auth: crate::agent_auth::AgentAuthConfig,
}

impl AgentApiState {
    /// Create state with a registry, no provisioner, the default timeout policy, and a
    /// default-capacity job store.
    pub fn new(registry: Arc<AgentRegistry>) -> Self {
        Self {
            registry,
            provisioner: None,
            timeouts: TimeoutPolicy::default(),
            jobs: Arc::new(JobStore::default()),
            auth: crate::agent_auth::AgentAuthConfig::default(),
        }
    }

    /// Attach a provisioner, enabling runtime registration via `POST /agents`.
    pub fn with_provisioner(mut self, provisioner: Arc<dyn AgentProvisioner>) -> Self {
        self.provisioner = Some(provisioner);
        self
    }

    /// Set the server-wide timeout policy.
    pub fn with_timeouts(mut self, timeouts: TimeoutPolicy) -> Self {
        self.timeouts = timeouts;
        self
    }

    /// Set the authentication configuration.
    pub fn with_auth(mut self, auth: crate::agent_auth::AgentAuthConfig) -> Self {
        self.auth = auth;
        self
    }
}

/// Request body for `POST /agents/{id}/execute`.
///
/// Only `input` is required today; later epics may add optional fields (streaming
/// flags, per-call overrides) without breaking this contract.
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct ExecuteRequest {
    /// The task / prompt to run the agent against.
    pub input: String,
    /// Optional per-request timeout (seconds), clamped to the server max. `0` is rejected.
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
}

/// Response body for a successful agent execution.
///
/// Carries the agent output plus the safe execution metadata from
/// [`PaladinResult`]. The `stop_reason` is rendered as a stable lowercase label
/// (`"completed"`, `"max_loops"`, `"stop_word"`, `"timeout"`) rather than the raw
/// serde enum shape, so the wire contract is stable.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ExecuteResponse {
    /// The generated output text.
    pub output: String,
    /// Total tokens used (prompt + completion).
    pub token_count: u32,
    /// Wall-clock execution time in milliseconds.
    pub execution_time_ms: u64,
    /// Number of reasoning loops executed.
    pub loop_count: u32,
    /// Why execution stopped, as a stable label.
    pub stop_reason: String,
}

impl From<PaladinResult> for ExecuteResponse {
    fn from(result: PaladinResult) -> Self {
        Self {
            output: result.output,
            token_count: result.token_count,
            execution_time_ms: result.execution_time_ms,
            loop_count: result.loop_count,
            stop_reason: stop_reason_label(&result.stop_reason).to_string(),
        }
    }
}

/// Map a [`StopReason`] to a stable, lowercase wire label.
fn stop_reason_label(reason: &StopReason) -> &'static str {
    match reason {
        StopReason::MaxLoops => "max_loops",
        StopReason::StopWord(_) => "stop_word",
        StopReason::Completed => "completed",
        StopReason::Timeout => "timeout",
    }
}

/// Safe, public-facing summary of an agent for the discovery endpoints.
///
/// Deliberately omits anything sensitive. Note that secrets (API keys, provider
/// credentials) never live on the [`Paladin`] entity in the first place — they are
/// supplied to executors at composition time — so none can leak here. The raw system
/// prompt is reduced to a short `description` preview rather than returned verbatim
/// (see PRD Open Question 1 on whether to omit it entirely).
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct AgentSummary {
    /// Registry id (the `{id}` path segment).
    pub id: String,
    /// Human-friendly display name.
    pub name: String,
    /// LLM model identifier.
    pub model: String,
    /// Short, single-line preview derived from the system prompt.
    pub description: String,
}

impl AgentSummary {
    /// Build a summary from a registry id and its agent.
    ///
    /// This is not a `From` impl because a summary needs the registry id, which lives
    /// in the registry key rather than on the [`Paladin`] itself.
    pub fn from_agent(id: impl Into<String>, paladin: &Paladin) -> Self {
        Self {
            id: id.into(),
            name: paladin.node.name.clone(),
            model: paladin.node.model.clone(),
            description: prompt_preview(&paladin.node.system_prompt),
        }
    }
}

/// Maximum number of characters from the system prompt to expose as a description.
const DESCRIPTION_PREVIEW_LEN: usize = 140;

/// Derive a short, single-line preview of a system prompt for discovery responses.
///
/// Takes the first line, trims it, and truncates to [`DESCRIPTION_PREVIEW_LEN`]
/// characters (on a char boundary), appending an ellipsis when truncated.
fn prompt_preview(system_prompt: &str) -> String {
    let first_line = system_prompt.lines().next().unwrap_or("").trim();
    if first_line.chars().count() <= DESCRIPTION_PREVIEW_LEN {
        return first_line.to_string();
    }
    let truncated: String = first_line.chars().take(DESCRIPTION_PREVIEW_LEN).collect();
    format!("{truncated}…")
}

// --- Response helpers (interim) ---------------------------------------------
//
// These mirror `delivery_controller`'s helpers and are kept local on purpose:
// failures use the unified [`ApiError`] (Epic 4); success bodies use [`ok_body`].

/// JSON response body type used by every agent handler's success path.
pub(crate) type JsonValue = Json<serde_json::Value>;

/// Serialize a successful payload to a JSON body. Serialization of the crate's own DTOs
/// is infallible in practice; on the unreachable error path we emit `null` rather than
/// fabricate an error envelope.
pub(crate) fn ok_body<T: Serialize>(value: &T) -> JsonValue {
    Json(serde_json::to_value(value).unwrap_or(serde_json::Value::Null))
}

// --- Handlers ---------------------------------------------------------------

/// `POST /agents/{id}/execute` — look the agent up by id and run it.
///
/// Returns:
/// - `200 OK` with [`ExecuteResponse`] on success;
/// - `404 Not Found` if no agent is registered under `id`;
/// - `502 Bad Gateway` if execution fails; `504` on timeout;
/// - `400 Bad Request` (via the `Json` extractor) if the body is missing/invalid.
#[utoipa::path(
    post,
    path = "/agents/{id}/execute",
    tag = "agents",
    params(("id" = String, Path, description = "Registry id of the agent to run")),
    request_body = ExecuteRequest,
    responses(
        (status = 200, description = "Execution result", body = ExecuteResponse),
        (status = 400, description = "Invalid timeout", body = ApiErrorBody),
        (status = 401, description = "Missing/invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Role not permitted for this agent", body = ApiErrorBody),
        (status = 404, description = "Unknown agent", body = ApiErrorBody),
        (status = 502, description = "Upstream execution failure", body = ApiErrorBody),
        (status = 504, description = "Execution timed out", body = ApiErrorBody),
    ),
    security(("api_key" = []), ("bearer_token" = [])),
)]
pub async fn execute_agent(
    State(state): State<AgentApiState>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    Json(request): Json<ExecuteRequest>,
) -> Result<(StatusCode, JsonValue), ApiError> {
    let entry = state
        .registry
        .get(&id)
        .ok_or_else(|| ApiError::not_found(format!("unknown agent '{id}'")))?;
    authorize_invoke(&principal, &entry.allowed_roles)?;

    let timeout = resolve_timeout(request.timeout_seconds, entry.timeout_secs, &state.timeouts)
        .map_err(|_| ApiError::bad_request("timeout_seconds must be a positive integer"))?;

    let run = entry
        .executor
        .execute(entry.paladin.as_ref(), &request.input);
    match tokio::time::timeout(timeout, run).await {
        Ok(Ok(result)) => Ok((StatusCode::OK, ok_body(&ExecuteResponse::from(result)))),
        Ok(Err(error)) => Err(ApiError::bad_gateway(error.to_string())),
        // The future is dropped on elapse — the in-flight execution is cancelled.
        Err(_elapsed) => Err(ApiError::gateway_timeout(format!(
            "agent '{id}' timed out after {}s",
            timeout.as_secs()
        ))),
    }
}

/// `GET /agents` — list every registered agent as a safe [`AgentSummary`].
///
/// Always returns `200 OK` with a JSON array (empty when no agents are registered).
/// Order is unspecified.
#[utoipa::path(
    get,
    path = "/agents",
    tag = "agents",
    responses(
        (status = 200, description = "Registered agents", body = [AgentSummary]),
        (status = 401, description = "Missing/invalid credentials", body = ApiErrorBody),
    ),
    security(("api_key" = []), ("bearer_token" = [])),
)]
pub async fn list_agents(State(state): State<AgentApiState>) -> (StatusCode, JsonValue) {
    let summaries: Vec<AgentSummary> = state
        .registry
        .list()
        .into_iter()
        .map(|(id, paladin)| AgentSummary::from_agent(id, paladin.as_ref()))
        .collect();
    (StatusCode::OK, ok_body(&summaries))
}

/// `GET /agents/{id}` — describe a single agent.
///
/// Returns `200 OK` with the agent's [`AgentSummary`], or `404 Not Found` with
/// `{ "error": ... }` if no agent is registered under `id`.
#[utoipa::path(
    get,
    path = "/agents/{id}",
    tag = "agents",
    params(("id" = String, Path, description = "Registry id of the agent")),
    responses(
        (status = 200, description = "Agent summary", body = AgentSummary),
        (status = 401, description = "Missing/invalid credentials", body = ApiErrorBody),
        (status = 404, description = "Unknown agent", body = ApiErrorBody),
    ),
    security(("api_key" = []), ("bearer_token" = [])),
)]
pub async fn describe_agent(
    State(state): State<AgentApiState>,
    Path(id): Path<String>,
) -> Result<(StatusCode, JsonValue), ApiError> {
    let entry = state
        .registry
        .get(&id)
        .ok_or_else(|| ApiError::not_found(format!("unknown agent '{id}'")))?;
    Ok((
        StatusCode::OK,
        ok_body(&AgentSummary::from_agent(id, entry.paladin.as_ref())),
    ))
}

/// `POST /agents` — register a new agent at runtime from an [`AgentSpec`].
///
/// Because `paladin-web` cannot build a [`Paladin`] itself, this delegates to the
/// injected [`AgentProvisioner`]. Returns:
/// - `201 Created` with the new agent's [`AgentSummary`] on success;
/// - `409 Conflict` if an agent is already registered under the spec's id;
/// - `422 Unprocessable Entity` if provisioning fails;
/// - `400 Bad Request` (via the `Json` extractor) if the body is missing/invalid;
/// - `501 Not Implemented` if no provisioner is wired (registration disabled).
#[utoipa::path(
    post,
    path = "/agents",
    tag = "agents",
    request_body = AgentSpec,
    responses(
        (status = 201, description = "Agent registered", body = AgentSummary),
        (status = 401, description = "Missing/invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Admin role required", body = ApiErrorBody),
        (status = 409, description = "Agent id already exists", body = ApiErrorBody),
        (status = 422, description = "Provisioning failed", body = ApiErrorBody),
        (status = 501, description = "Runtime registration not enabled", body = ApiErrorBody),
    ),
    security(("api_key" = []), ("bearer_token" = [])),
)]
pub async fn register_agent(
    State(state): State<AgentApiState>,
    Extension(principal): Extension<Principal>,
    Json(spec): Json<AgentSpec>,
) -> Result<(StatusCode, JsonValue), ApiError> {
    require_admin(&principal)?;

    let provisioner = state
        .provisioner
        .as_ref()
        .ok_or_else(|| ApiError::not_implemented("runtime agent registration is not enabled"))?;

    // Cheap early rejection before paying to provision an agent we'd discard.
    if state.registry.contains(&spec.id) {
        return Err(ApiError::conflict(format!(
            "agent '{}' already exists",
            spec.id
        )));
    }

    let provisioned = provisioner
        .provision(&spec)
        .await
        .map_err(|error| ApiError::unprocessable(error.to_string()))?;

    let paladin = Arc::new(provisioned.paladin);
    // Re-check on insert closes the race between the `contains` check and here.
    if !state.registry.insert_entry(
        spec.id.clone(),
        AgentEntry {
            paladin: Arc::clone(&paladin),
            executor: provisioned.executor,
            streamer: provisioned.streamer,
            timeout_secs: spec.timeout_seconds,
            allowed_roles: spec.allowed_roles.clone(),
        },
    ) {
        return Err(ApiError::conflict(format!(
            "agent '{}' already exists",
            spec.id
        )));
    }
    Ok((
        StatusCode::CREATED,
        ok_body(&AgentSummary::from_agent(spec.id, paladin.as_ref())),
    ))
}

/// `DELETE /agents/{id}` — deregister an agent.
///
/// Returns `204 No Content` (empty body) on success, or `404 Not Found` with
/// `{ "error": ... }` if no agent is registered under `id`.
#[utoipa::path(
    delete,
    path = "/agents/{id}",
    tag = "agents",
    params(("id" = String, Path, description = "Registry id of the agent to remove")),
    responses(
        (status = 204, description = "Agent deregistered"),
        (status = 401, description = "Missing/invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Admin role required", body = ApiErrorBody),
        (status = 404, description = "Unknown agent", body = ApiErrorBody),
    ),
    security(("api_key" = []), ("bearer_token" = [])),
)]
pub async fn deregister_agent(
    State(state): State<AgentApiState>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    require_admin(&principal)?;
    if state.registry.remove(&id) {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found(format!("unknown agent '{id}'")))
    }
}

// --- Streaming --------------------------------------------------------------

/// A boxed SSE event stream (the two streaming backends produce different concrete
/// stream types, so both are boxed to one type for the handler's return).
type SseEventStream = Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>>;

/// Render a streaming chunk (or error) as an SSE event.
///
/// Emits `chunk` events with `{ "text": ... }`, a terminal `done` event, and an `error`
/// event for a mid-stream failure (after which the stream closes).
fn chunk_to_event(item: Result<PaladinStreamChunk, PaladinError>) -> Event {
    match item {
        Ok(chunk) if chunk.is_final => Event::default()
            .event("done")
            .data(json!({ "done": true }).to_string()),
        Ok(chunk) => Event::default()
            .event("chunk")
            .data(json!({ "text": chunk.text }).to_string()),
        Err(error) => Event::default().event("error").data(
            ApiError::bad_gateway(error.to_string())
                .to_body()
                .to_string(),
        ),
    }
}

/// Adapt an agent [`PaladinStream`] into an SSE event stream bounded by `timeout`.
///
/// Races each chunk against a single deadline: on the deadline it yields a terminal
/// `error` event and stops (dropping the receiver, which cancels the producer). On a
/// final chunk or channel close it stops normally.
fn timed_event_stream(
    rx: PaladinStream,
    timeout: Duration,
) -> impl Stream<Item = Result<Event, Infallible>> + Send {
    async_stream::stream! {
        let sleep = tokio::time::sleep(timeout);
        tokio::pin!(sleep);
        let mut rx = rx;
        loop {
            tokio::select! {
                _ = &mut sleep => {
                    yield Ok(Event::default()
                        .event("error")
                        .data(ApiError::gateway_timeout("stream timed out").to_body().to_string()));
                    break;
                }
                item = rx.recv() => match item {
                    Some(result) => {
                        let is_final = matches!(&result, Ok(chunk) if chunk.is_final);
                        yield Ok(chunk_to_event(result));
                        if is_final {
                            break;
                        }
                    }
                    None => break,
                },
            }
        }
    }
}

/// `POST /agents/{id}/execute/stream` — run an agent, streaming its output over SSE.
///
/// Returns a `text/event-stream` of `chunk` events ending with a `done` event. For an
/// agent with a streaming backend these are real incremental LLM tokens; for an agent
/// without one, the buffered result is framed as a single `chunk` + `done` (so the
/// endpoint always works). Execution is bounded by the resolved timeout: on expiry the
/// stream yields a terminal `error` event (streaming) or returns `504` (buffered
/// fallback). Maps unknown id → `404`, invalid `timeout_seconds` → `400`, and an
/// up-front execution failure → `502`.
#[utoipa::path(
    post,
    path = "/agents/{id}/execute/stream",
    tag = "agents",
    params(("id" = String, Path, description = "Registry id of the agent to run")),
    request_body = ExecuteRequest,
    responses(
        (status = 200, description = "Server-Sent Events stream: `chunk` events carry \
            `{ \"text\": ... }`, a terminal `done` event carries the final result, and an \
            `error` event (with the standard error envelope) is emitted on mid-stream \
            failure or timeout.", content_type = "text/event-stream"),
        (status = 400, description = "Invalid timeout", body = ApiErrorBody),
        (status = 401, description = "Missing/invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Role not permitted for this agent", body = ApiErrorBody),
        (status = 404, description = "Unknown agent", body = ApiErrorBody),
        (status = 502, description = "Upstream execution failure", body = ApiErrorBody),
        (status = 504, description = "Execution timed out", body = ApiErrorBody),
    ),
    security(("api_key" = []), ("bearer_token" = [])),
)]
pub async fn execute_agent_stream(
    State(state): State<AgentApiState>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    Json(request): Json<ExecuteRequest>,
) -> Response {
    let Some(entry) = state.registry.get(&id) else {
        return ApiError::not_found(format!("unknown agent '{id}'")).into_response();
    };
    if let Err(error) = authorize_invoke(&principal, &entry.allowed_roles) {
        return error.into_response();
    }

    let timeout =
        match resolve_timeout(request.timeout_seconds, entry.timeout_secs, &state.timeouts) {
            Ok(d) => d,
            Err(_) => {
                return ApiError::bad_request("timeout_seconds must be a positive integer")
                    .into_response();
            }
        };

    // Real token streaming when the agent has a streaming-capable executor.
    if let Some(streamer) = entry.streamer.clone() {
        return match streamer
            .execute_stream(entry.paladin.as_ref(), &request.input)
            .await
        {
            Ok(rx) => {
                let boxed: SseEventStream = Box::pin(timed_event_stream(rx, timeout));
                Sse::new(boxed).into_response()
            }
            Err(error) => ApiError::bad_gateway(error.to_string()).into_response(),
        };
    }

    // Fallback (no streaming backend): run buffered under the timeout, then frame as SSE.
    let run = entry
        .executor
        .execute(entry.paladin.as_ref(), &request.input);
    match tokio::time::timeout(timeout, run).await {
        Ok(Ok(result)) => {
            let response = ExecuteResponse::from(result);
            let chunk = Event::default()
                .event("chunk")
                .data(json!({ "text": response.output }).to_string());
            let done = Event::default().event("done").data(
                serde_json::to_string(&response).unwrap_or_else(|_| "{\"done\":true}".to_string()),
            );
            let events: Vec<Result<Event, Infallible>> = vec![Ok(chunk), Ok(done)];
            let boxed: SseEventStream = Box::pin(futures::stream::iter(events));
            Sse::new(boxed).into_response()
        }
        Ok(Err(error)) => ApiError::bad_gateway(error.to_string()).into_response(),
        Err(_elapsed) => ApiError::gateway_timeout(format!(
            "agent '{id}' timed out after {}s",
            timeout.as_secs()
        ))
        .into_response(),
    }
}

// --- Async jobs -------------------------------------------------------------

/// `POST /agents/{id}/jobs` — enqueue an async run and return its job id immediately.
///
/// Spawns a task that runs the agent (buffered) under the resolved timeout, recording
/// the outcome in the job store. Returns `202 Accepted` with `{ "job_id": ... }`. Maps
/// unknown id → `404` and invalid `timeout_seconds` → `400`.
#[utoipa::path(
    post,
    path = "/agents/{id}/jobs",
    tag = "agents",
    params(("id" = String, Path, description = "Registry id of the agent to run")),
    request_body = ExecuteRequest,
    responses(
        (status = 202, description = "Job accepted; body carries `{ \"job_id\": ... }`", body = Object),
        (status = 400, description = "Invalid timeout", body = ApiErrorBody),
        (status = 401, description = "Missing/invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Role not permitted for this agent", body = ApiErrorBody),
        (status = 404, description = "Unknown agent", body = ApiErrorBody),
    ),
    security(("api_key" = []), ("bearer_token" = [])),
)]
pub async fn enqueue_job(
    State(state): State<AgentApiState>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    Json(request): Json<ExecuteRequest>,
) -> Result<(StatusCode, JsonValue), ApiError> {
    let entry = state
        .registry
        .get(&id)
        .ok_or_else(|| ApiError::not_found(format!("unknown agent '{id}'")))?;
    authorize_invoke(&principal, &entry.allowed_roles)?;

    let timeout = resolve_timeout(request.timeout_seconds, entry.timeout_secs, &state.timeouts)
        .map_err(|_| ApiError::bad_request("timeout_seconds must be a positive integer"))?;

    let job_id = state.jobs.create();
    let jobs = Arc::clone(&state.jobs);
    let jid = job_id.clone();
    let agent_id = id.clone();
    tokio::spawn(async move {
        let run = entry
            .executor
            .execute(entry.paladin.as_ref(), &request.input);
        match tokio::time::timeout(timeout, run).await {
            Ok(Ok(result)) => {
                let value = serde_json::to_value(ExecuteResponse::from(result))
                    .unwrap_or_else(|_| json!({}));
                jobs.complete(&jid, value);
            }
            Ok(Err(error)) => jobs.fail(&jid, error.to_string()),
            Err(_elapsed) => jobs.time_out(
                &jid,
                format!("agent '{agent_id}' timed out after {}s", timeout.as_secs()),
            ),
        }
    });

    Ok((StatusCode::ACCEPTED, ok_body(&json!({ "job_id": job_id }))))
}

/// `GET /agents/{id}/jobs/{job_id}` — poll an async job's status and result.
///
/// Returns `200 OK` with the [`JobRecord`], or `404` if no
/// job is found under `job_id` (jobs are ephemeral and may have been evicted).
#[utoipa::path(
    get,
    path = "/agents/{id}/jobs/{job_id}",
    tag = "agents",
    params(
        ("id" = String, Path, description = "Registry id of the agent"),
        ("job_id" = String, Path, description = "Job id returned by enqueue"),
    ),
    responses(
        (status = 200, description = "Job status/result", body = JobRecord),
        (status = 401, description = "Missing/invalid credentials", body = ApiErrorBody),
        (status = 404, description = "Unknown job", body = ApiErrorBody),
    ),
    security(("api_key" = []), ("bearer_token" = [])),
)]
pub async fn get_job(
    State(state): State<AgentApiState>,
    Path((_id, job_id)): Path<(String, String)>,
) -> Result<(StatusCode, JsonValue), ApiError> {
    match state.jobs.get(&job_id) {
        Some(record) => Ok((StatusCode::OK, ok_body(&record))),
        None => Err(ApiError::not_found(format!("unknown job '{job_id}'"))),
    }
}

// --- Router -----------------------------------------------------------------

/// Build the agent-execution sub-router and bind it to its [`AgentApiState`].
///
/// Mounts the five agent routes:
///
/// - `GET    /agents` — list agents
/// - `POST   /agents` — register an agent at runtime
/// - `GET    /agents/{id}` — describe an agent
/// - `DELETE /agents/{id}` — deregister an agent
/// - `POST   /agents/{id}/execute` — run an agent
///
/// The returned `Router` has its state already applied, so it can be `merge`d into the
/// application router alongside the user/auth and delivery routers (see
/// [`create_app_router_with_agents`](crate::app::create_app_router_with_agents)).
///
/// These routes are intentionally **unauthenticated** in Milestone 12, Epic 1;
/// authentication and per-agent authorization are layered on in Epic 5 without changing
/// these handler signatures.
/// Build the agent API as a `utoipa-axum` [`OpenApiRouter`] — the routes and the OpenAPI
/// paths come from one definition (the `#[utoipa::path]` annotations), so the served API
/// and the generated spec cannot drift.
///
/// The authentication middleware is applied here (a no-op when auth is disabled); the
/// unversioned, unauthenticated health probes are mounted separately by [`agent_router`].
/// Paths are declared unprefixed (`/agents/...`); the `/v1` segment is added when this
/// router is nested (see [`agent_router`] / `crate::openapi`).
pub fn agent_openapi_router(state: AgentApiState) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(list_agents, register_agent))
        .routes(routes!(describe_agent, deregister_agent))
        .routes(routes!(execute_agent))
        .routes(routes!(execute_agent_stream))
        .routes(routes!(enqueue_job))
        .routes(routes!(get_job))
        // Authenticate the agent routes (no-op when auth is disabled). `route_layer`
        // applies only to these routes, so the merged health probes stay open.
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::agent_auth::require_authentication::<AgentApiState>,
        ))
        .with_state(state)
}

/// API version prefix under which the agent routes are served.
pub const API_V1_PREFIX: &str = "/v1";

/// Assemble the agent API nested under [`API_V1_PREFIX`] into an `axum` [`Router`] and its
/// raw OpenAPI document (before [`crate::openapi`] adds info/security schemes).
///
/// This is the single source of the `/v1` nesting, shared by [`agent_router`] (which keeps
/// the router and drops the spec) and the spec builder (which keeps the spec).
pub(crate) fn versioned_agent_parts(state: AgentApiState) -> (Router, utoipa::openapi::OpenApi) {
    OpenApiRouter::new()
        .nest(API_V1_PREFIX, agent_openapi_router(state))
        .split_for_parts()
}

/// Build the agent router as a plain `axum` [`Router`]: the annotated agent API nested under
/// [`API_V1_PREFIX`] (`/v1/agents/...`), plus the unversioned health probes at the root.
///
/// The OpenAPI document produced alongside is discarded here; the binary builds and serves
/// the spec via [`crate::openapi`].
pub fn agent_router(state: AgentApiState) -> Router {
    let (routes, _api) = versioned_agent_parts(state.clone());
    // Mount the liveness/readiness probes alongside (unversioned, unauthenticated).
    routes.merge(crate::health::health_routes(state))
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use axum::body::Body;
    use axum::http::Request;
    use axum::routing::post;
    use paladin_core::platform::container::paladin::PaladinData;
    use paladin_core::platform::container::user::UserRole;
    use paladin_ports::output::paladin_executor_port::PaladinExecutorPort;
    use tower::ServiceExt; // for `Router::oneshot`

    /// An admin `Principal` extension for direct handler calls (passes all authz checks).
    fn admin() -> Extension<Principal> {
        Extension(Principal {
            id: "test-admin".to_string(),
            role: UserRole::Admin,
        })
    }

    /// A non-admin (`User`) `Principal` extension for authz tests.
    fn user() -> Extension<Principal> {
        Extension(Principal {
            id: "test-user".to_string(),
            role: UserRole::User,
        })
    }

    /// Configurable in-test executor: succeeds with a fixed output, fails, or stalls
    /// (sleeps far longer than any test timeout, to exercise cancellation).
    enum MockExecutor {
        Succeeds(String),
        Fails(String),
        Slow,
    }

    #[async_trait]
    impl PaladinExecutorPort for MockExecutor {
        async fn execute(
            &self,
            _paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinResult, PaladinError> {
            match self {
                MockExecutor::Succeeds(output) => Ok(PaladinResult::new(
                    output.clone(),
                    5,
                    10,
                    1,
                    StopReason::Completed,
                )),
                MockExecutor::Fails(message) => Err(PaladinError::ExecutionError(message.clone())),
                MockExecutor::Slow => {
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    Ok(PaladinResult::new(
                        "late".to_string(),
                        0,
                        0,
                        0,
                        StopReason::Completed,
                    ))
                }
            }
        }
    }

    fn test_agent(name: &str) -> Arc<Paladin> {
        let data = PaladinData {
            system_prompt: "You are a test agent.".to_string(),
            name: name.to_string(),
            model: "gpt-4".to_string(),
            ..Default::default()
        };
        Arc::new(Paladin::new(data, Some(name.to_string())))
    }

    /// State holding a single agent `id` backed by `executor`.
    fn state_with_agent(id: &str, executor: MockExecutor) -> AgentApiState {
        let registry = AgentRegistry::new();
        registry.insert(id, test_agent(id), Arc::new(executor));
        AgentApiState::new(Arc::new(registry))
    }

    use paladin_ports::output::paladin_port::PaladinStream;
    use paladin_ports::output::streaming_executor_port::StreamingExecutorPort;

    /// In-test streamer that emits the given text chunks then a final marker.
    struct MockStreamer {
        chunks: Vec<String>,
    }

    #[async_trait]
    impl StreamingExecutorPort for MockStreamer {
        async fn execute_stream(
            &self,
            _paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinStream, PaladinError> {
            let (tx, rx) = tokio::sync::mpsc::channel(8);
            let chunks = self.chunks.clone();
            tokio::spawn(async move {
                for c in chunks {
                    let _ = tx
                        .send(Ok(PaladinStreamChunk {
                            text: c,
                            is_final: false,
                            metadata: None,
                        }))
                        .await;
                }
                let _ = tx
                    .send(Ok(PaladinStreamChunk {
                        text: String::new(),
                        is_final: true,
                        metadata: None,
                    }))
                    .await;
            });
            Ok(rx)
        }
    }

    async fn read_body(response: axum::response::Response) -> String {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        String::from_utf8(bytes.to_vec()).expect("utf8 body")
    }

    fn state_with_streaming_agent(id: &str, chunks: Vec<String>) -> AgentApiState {
        let registry = AgentRegistry::new();
        let streamer: Arc<dyn StreamingExecutorPort> = Arc::new(MockStreamer { chunks });
        registry.insert_with_streaming(
            id,
            test_agent(id),
            Arc::new(MockExecutor::Succeeds("buffered".to_string())),
            Some(streamer),
        );
        AgentApiState::new(Arc::new(registry))
    }

    #[tokio::test]
    async fn stream_emits_chunk_events_then_done() {
        let state = state_with_streaming_agent("r", vec!["Hel".to_string(), "lo".to_string()]);
        let response = execute_agent_stream(
            State(state),
            admin(),
            Path("r".to_string()),
            Json(ExecuteRequest {
                input: "hi".to_string(),
                timeout_seconds: None,
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);

        let body = read_body(response).await;
        assert!(
            body.contains("event: chunk"),
            "expected chunk events: {body}"
        );
        assert!(body.contains(r#"{"text":"Hel"}"#), "first chunk: {body}");
        assert!(body.contains(r#"{"text":"lo"}"#), "second chunk: {body}");
        assert!(
            body.contains("event: done"),
            "expected a done event: {body}"
        );
    }

    #[tokio::test]
    async fn stream_falls_back_to_buffered_when_no_streamer() {
        // Agent registered without a streaming handle (MockExecutor returns "buffered").
        let state = state_with_agent("r", MockExecutor::Succeeds("buffered".to_string()));
        let response = execute_agent_stream(
            State(state),
            admin(),
            Path("r".to_string()),
            Json(ExecuteRequest {
                input: "hi".to_string(),
                timeout_seconds: None,
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);

        let body = read_body(response).await;
        assert!(
            body.contains(r#"{"text":"buffered"}"#),
            "fallback chunk: {body}"
        );
        assert!(
            body.contains("event: done"),
            "expected a done event: {body}"
        );
    }

    #[tokio::test]
    async fn stream_unknown_id_returns_404() {
        let state = state_with_agent("r", MockExecutor::Succeeds("x".to_string()));
        let response = execute_agent_stream(
            State(state),
            admin(),
            Path("missing".to_string()),
            Json(ExecuteRequest {
                input: "hi".to_string(),
                timeout_seconds: None,
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    /// A streamer that emits one chunk then stalls (never sends a final marker).
    struct StallStreamer;

    #[async_trait]
    impl StreamingExecutorPort for StallStreamer {
        async fn execute_stream(
            &self,
            _paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinStream, PaladinError> {
            let (tx, rx) = tokio::sync::mpsc::channel(8);
            tokio::spawn(async move {
                let _ = tx
                    .send(Ok(PaladinStreamChunk {
                        text: "partial".to_string(),
                        is_final: false,
                        metadata: None,
                    }))
                    .await;
                // Hold the sender open without finalizing, forcing a timeout.
                tokio::time::sleep(Duration::from_secs(60)).await;
            });
            Ok(rx)
        }
    }

    #[tokio::test]
    async fn execute_times_out_with_504() {
        let registry = AgentRegistry::new();
        registry.insert("r", test_agent("r"), Arc::new(MockExecutor::Slow));
        let state = AgentApiState::new(Arc::new(registry));

        let err = execute_agent(
            State(state),
            admin(),
            Path("r".to_string()),
            Json(ExecuteRequest {
                input: "hi".to_string(),
                timeout_seconds: Some(1), // 1s; the Slow executor sleeps 60s
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(err.status(), StatusCode::GATEWAY_TIMEOUT);
    }

    #[tokio::test]
    async fn execute_zero_timeout_is_400() {
        let state = state_with_agent("r", MockExecutor::Succeeds("ok".to_string()));
        let err = execute_agent(
            State(state),
            admin(),
            Path("r".to_string()),
            Json(ExecuteRequest {
                input: "hi".to_string(),
                timeout_seconds: Some(0),
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn stream_times_out_with_terminal_error_event() {
        let registry = AgentRegistry::new();
        let streamer: Arc<dyn StreamingExecutorPort> = Arc::new(StallStreamer);
        registry.insert_with_streaming(
            "r",
            test_agent("r"),
            Arc::new(MockExecutor::Succeeds("x".to_string())),
            Some(streamer),
        );
        let state = AgentApiState::new(Arc::new(registry));

        let response = execute_agent_stream(
            State(state),
            admin(),
            Path("r".to_string()),
            Json(ExecuteRequest {
                input: "hi".to_string(),
                timeout_seconds: Some(1),
            }),
        )
        .await;

        let body = read_body(response).await;
        assert!(
            body.contains("partial"),
            "should emit the partial chunk: {body}"
        );
        assert!(
            body.contains("event: error") && body.contains("timed out"),
            "should emit a terminal timeout error event: {body}"
        );
    }

    /// Poll a job until it leaves `running` (or give up), returning the final record body.
    async fn poll_job(state: &AgentApiState, agent_id: &str, job_id: &str) -> serde_json::Value {
        for _ in 0..50 {
            let (status, Json(body)) = get_job(
                State(state.clone()),
                Path((agent_id.to_string(), job_id.to_string())),
            )
            .await
            .expect("job lookup ok");
            assert_eq!(status, StatusCode::OK);
            if body["status"] != "running" {
                return body;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("job did not reach a terminal state in time");
    }

    #[tokio::test]
    async fn job_enqueue_completes_with_result() {
        let state = state_with_agent("r", MockExecutor::Succeeds("done".to_string()));

        let (status, Json(body)) = enqueue_job(
            State(state.clone()),
            admin(),
            Path("r".to_string()),
            Json(ExecuteRequest {
                input: "hi".to_string(),
                timeout_seconds: None,
            }),
        )
        .await
        .expect("accepted");
        assert_eq!(status, StatusCode::ACCEPTED);
        let job_id = body["job_id"].as_str().expect("job_id present").to_string();

        let record = poll_job(&state, "r", &job_id).await;
        assert_eq!(record["status"], "completed");
        assert_eq!(record["result"]["output"], "done");
    }

    #[tokio::test]
    async fn job_enqueue_unknown_agent_returns_404() {
        let state = state_with_agent("r", MockExecutor::Succeeds("x".to_string()));
        let err = enqueue_job(
            State(state),
            admin(),
            Path("missing".to_string()),
            Json(ExecuteRequest {
                input: "hi".to_string(),
                timeout_seconds: None,
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_job_unknown_returns_404() {
        let state = state_with_agent("r", MockExecutor::Succeeds("x".to_string()));
        let err = get_job(
            State(state),
            Path(("r".to_string(), "no-such-job".to_string())),
        )
        .await
        .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn job_times_out() {
        let registry = AgentRegistry::new();
        registry.insert("r", test_agent("r"), Arc::new(MockExecutor::Slow));
        let state = AgentApiState::new(Arc::new(registry));

        let (status, Json(body)) = enqueue_job(
            State(state.clone()),
            admin(),
            Path("r".to_string()),
            Json(ExecuteRequest {
                input: "hi".to_string(),
                timeout_seconds: Some(1),
            }),
        )
        .await
        .expect("accepted");
        assert_eq!(status, StatusCode::ACCEPTED);
        let job_id = body["job_id"].as_str().unwrap().to_string();

        let record = poll_job(&state, "r", &job_id).await;
        assert_eq!(record["status"], "timed_out");
    }

    #[tokio::test]
    async fn execute_success_returns_200_with_output_and_metadata() {
        let state = state_with_agent("r", MockExecutor::Succeeds("done".to_string()));
        let (status, Json(body)) = execute_agent(
            State(state),
            admin(),
            Path("r".to_string()),
            Json(ExecuteRequest {
                input: "hi".to_string(),
                timeout_seconds: None,
            }),
        )
        .await
        .expect("ok");

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["output"], "done");
        assert_eq!(body["token_count"], 5);
        assert_eq!(body["execution_time_ms"], 10);
        assert_eq!(body["loop_count"], 1);
        assert_eq!(body["stop_reason"], "completed");
    }

    #[tokio::test]
    async fn execute_unknown_id_returns_404() {
        let state = state_with_agent("r", MockExecutor::Succeeds("done".to_string()));
        let err = execute_agent(
            State(state),
            admin(),
            Path("missing".to_string()),
            Json(ExecuteRequest {
                input: "hi".to_string(),
                timeout_seconds: None,
            }),
        )
        .await
        .unwrap_err();

        assert_eq!(err.status(), StatusCode::NOT_FOUND);
        assert_eq!(err.to_body()["error"]["code"], "not_found");
    }

    #[tokio::test]
    async fn execute_executor_error_returns_502() {
        let state = state_with_agent("r", MockExecutor::Fails("upstream down".to_string()));
        let err = execute_agent(
            State(state),
            admin(),
            Path("r".to_string()),
            Json(ExecuteRequest {
                input: "hi".to_string(),
                timeout_seconds: None,
            }),
        )
        .await
        .unwrap_err();

        assert_eq!(err.status(), StatusCode::BAD_GATEWAY);
        let body = err.to_body();
        assert_eq!(body["error"]["code"], "bad_gateway");
        assert_eq!(body["error"]["message"], "Execution error: upstream down");
    }

    #[tokio::test]
    async fn execute_invalid_body_returns_400_through_router() {
        let state = state_with_agent("r", MockExecutor::Succeeds("done".to_string()));
        // Use the real router so the (default-disabled) auth layer attaches a principal;
        // the malformed body must still be rejected with 400 by the JSON extractor.
        let app = agent_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/agents/r/execute")
                    .header("content-type", "application/json")
                    .body(Body::from("{ not valid json "))
                    .expect("request builds"),
            )
            .await
            .expect("router responds");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    use crate::agent_registry::{ProvisionError, ProvisionedAgent};

    /// Configurable in-test provisioner.
    enum MockProvisioner {
        Succeeds,
        Fails(String),
    }

    #[async_trait]
    impl AgentProvisioner for MockProvisioner {
        async fn provision(&self, spec: &AgentSpec) -> Result<ProvisionedAgent, ProvisionError> {
            match self {
                MockProvisioner::Succeeds => {
                    let data = PaladinData {
                        system_prompt: spec.system_prompt.clone(),
                        name: spec.name.clone(),
                        model: spec.model.clone(),
                        ..Default::default()
                    };
                    let paladin = Paladin::new(data, Some(spec.id.clone()));
                    let executor: Arc<dyn PaladinExecutorPort> =
                        Arc::new(MockExecutor::Succeeds("ok".to_string()));
                    Ok(ProvisionedAgent {
                        paladin,
                        executor,
                        streamer: None,
                    })
                }
                MockProvisioner::Fails(message) => Err(ProvisionError::Failed(message.clone())),
            }
        }
    }

    fn sample_spec(id: &str) -> AgentSpec {
        AgentSpec {
            id: id.to_string(),
            name: "Researcher".to_string(),
            model: "gpt-4".to_string(),
            system_prompt: "You research topics.".to_string(),
            temperature: None,
            stop_words: vec![],
            timeout_seconds: None,
            allowed_roles: vec![],
        }
    }

    /// State with an empty registry and the given provisioner.
    fn state_with_provisioner(provisioner: MockProvisioner) -> AgentApiState {
        AgentApiState::new(Arc::new(AgentRegistry::new())).with_provisioner(Arc::new(provisioner))
    }

    #[tokio::test]
    async fn register_success_returns_201_and_is_retrievable() {
        let state = state_with_provisioner(MockProvisioner::Succeeds);

        let (status, Json(body)) =
            register_agent(State(state.clone()), admin(), Json(sample_spec("new")))
                .await
                .expect("created");
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(body["id"], "new");
        assert_eq!(body["name"], "Researcher");

        // The shared registry now resolves the new agent.
        let (status, _) = describe_agent(State(state), Path("new".to_string()))
            .await
            .expect("ok");
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn register_duplicate_id_returns_409() {
        let state = state_with_provisioner(MockProvisioner::Succeeds);
        // First registration succeeds.
        let _ = register_agent(State(state.clone()), admin(), Json(sample_spec("dup"))).await;
        // Second with the same id conflicts.
        let err = register_agent(State(state), admin(), Json(sample_spec("dup")))
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::CONFLICT);
        assert_eq!(err.to_body()["error"]["code"], "conflict");
    }

    #[tokio::test]
    async fn register_provision_failure_returns_422() {
        let state = state_with_provisioner(MockProvisioner::Fails("no such model".to_string()));
        let err = register_agent(State(state), admin(), Json(sample_spec("x")))
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            err.to_body()["error"]["message"],
            "provisioning failed: no such model"
        );
    }

    #[tokio::test]
    async fn register_without_provisioner_returns_501() {
        // No provisioner wired: registration must fail closed, not panic.
        let state = AgentApiState::new(Arc::new(AgentRegistry::new()));
        let err = register_agent(State(state), admin(), Json(sample_spec("x")))
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn register_invalid_body_returns_400_through_router() {
        let state = state_with_provisioner(MockProvisioner::Succeeds);
        // Use the real router so the (default-disabled) auth layer attaches a principal.
        let app = agent_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/agents")
                    .header("content-type", "application/json")
                    .body(Body::from("{ not valid json "))
                    .expect("request builds"),
            )
            .await
            .expect("router responds");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn deregister_known_id_returns_204_then_404() {
        let state = registry_state(vec![("r", test_agent("Researcher"))]);

        let result = deregister_agent(State(state.clone()), admin(), Path("r".to_string())).await;
        match result {
            Ok(status) => assert_eq!(status, StatusCode::NO_CONTENT),
            Err(e) => panic!("expected 204, got {:?}", e.status()),
        }

        // The agent is gone afterward.
        let err = describe_agent(State(state), Path("r".to_string()))
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn deregister_unknown_id_returns_404() {
        let state = AgentApiState::new(Arc::new(AgentRegistry::new()));
        let result = deregister_agent(State(state), admin(), Path("missing".to_string())).await;
        match result {
            Err(e) => assert_eq!(e.status(), StatusCode::NOT_FOUND),
            Ok(other) => panic!("expected 404, got {other:?}"),
        }
    }

    // --- Authorization (per-agent allowed_roles + admin gate) ---

    /// State with one agent restricted to the given roles.
    fn state_with_restricted_agent(id: &str, roles: Vec<UserRole>) -> AgentApiState {
        let registry = AgentRegistry::new();
        registry.insert_entry(
            id.to_string(),
            AgentEntry {
                paladin: test_agent(id),
                executor: Arc::new(MockExecutor::Succeeds("ok".to_string())),
                streamer: None,
                timeout_secs: None,
                allowed_roles: roles,
            },
        );
        AgentApiState::new(Arc::new(registry))
    }

    #[tokio::test]
    async fn execute_forbidden_for_disallowed_role() {
        let state = state_with_restricted_agent("r", vec![UserRole::Admin]);
        let err = execute_agent(
            State(state),
            user(), // role User, not in [Admin]
            Path("r".to_string()),
            Json(ExecuteRequest {
                input: "hi".to_string(),
                timeout_seconds: None,
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(err.status(), StatusCode::FORBIDDEN);
        assert_eq!(err.to_body()["error"]["code"], "forbidden");
    }

    #[tokio::test]
    async fn execute_allowed_for_listed_role() {
        let state = state_with_restricted_agent("r", vec![UserRole::User]);
        let (status, _) = execute_agent(
            State(state),
            user(),
            Path("r".to_string()),
            Json(ExecuteRequest {
                input: "hi".to_string(),
                timeout_seconds: None,
            }),
        )
        .await
        .expect("ok");
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn register_forbidden_for_non_admin() {
        let state = state_with_provisioner(MockProvisioner::Succeeds);
        let err = register_agent(State(state), user(), Json(sample_spec("x")))
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn deregister_forbidden_for_non_admin() {
        let state = registry_state(vec![("r", test_agent("Researcher"))]);
        let err = deregister_agent(State(state), user(), Path("r".to_string()))
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn agent_router_merges_with_other_routes_without_conflict() {
        let state = registry_state(vec![("r", test_agent("Researcher"))]);
        // A stand-in for the user/auth router, with its own state already applied.
        let other = Router::new().route("/users/login", post(|| async { StatusCode::OK }));
        let app = other.merge(agent_router(state));

        // An agent route resolves.
        let agents = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/agents")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(agents.status(), StatusCode::OK);

        // The merged-in placeholder route also resolves.
        let login = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/users/login")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(login.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn agent_api_is_versioned_under_v1() {
        let state = registry_state(vec![("r", test_agent("Researcher"))]);
        let app = agent_router(state);

        // Versioned path resolves.
        let v1 = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/agents")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(v1.status(), StatusCode::OK);

        // Unversioned path no longer exists.
        let unversioned = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/agents")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(unversioned.status(), StatusCode::NOT_FOUND);

        // Health stays unversioned.
        let health = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(health.status(), StatusCode::OK);
    }

    /// Build an agent with a multi-line prompt whose second line is a leak canary.
    fn agent_with_secret_second_line(name: &str) -> Arc<Paladin> {
        let data = PaladinData {
            system_prompt: "Public first line.\nLEAK_CANARY second line.".to_string(),
            name: name.to_string(),
            model: "gpt-4".to_string(),
            ..Default::default()
        };
        Arc::new(Paladin::new(data, Some(name.to_string())))
    }

    fn registry_state(agents: Vec<(&str, Arc<Paladin>)>) -> AgentApiState {
        let registry = AgentRegistry::new();
        for (id, paladin) in agents {
            registry.insert(
                id,
                paladin,
                Arc::new(MockExecutor::Succeeds("x".to_string())),
            );
        }
        AgentApiState::new(Arc::new(registry))
    }

    #[tokio::test]
    async fn list_agents_returns_200_with_summaries_and_no_prompt_leak() {
        let state = registry_state(vec![
            ("researcher", agent_with_secret_second_line("Researcher")),
            ("summarizer", test_agent("Summarizer")),
        ]);

        let (status, Json(body)) = list_agents(State(state)).await;
        assert_eq!(status, StatusCode::OK);

        let arr = body.as_array().expect("body is a JSON array");
        assert_eq!(arr.len(), 2);

        let mut ids: Vec<&str> = arr
            .iter()
            .map(|a| a["id"].as_str().expect("id is a string"))
            .collect();
        ids.sort();
        assert_eq!(ids, vec!["researcher", "summarizer"]);

        // The full multi-line prompt must never appear in a discovery response.
        assert!(
            !body.to_string().contains("LEAK_CANARY"),
            "discovery response leaked the raw system prompt"
        );
    }

    #[tokio::test]
    async fn list_agents_empty_registry_returns_empty_array() {
        let state = AgentApiState::new(Arc::new(AgentRegistry::new()));
        let (status, Json(body)) = list_agents(State(state)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.as_array().map(|a| a.len()), Some(0));
    }

    #[tokio::test]
    async fn describe_agent_returns_200_for_known_id_without_prompt_leak() {
        let state = registry_state(vec![("r", agent_with_secret_second_line("Researcher"))]);
        let (status, Json(body)) = describe_agent(State(state), Path("r".to_string()))
            .await
            .expect("ok");

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["id"], "r");
        assert_eq!(body["name"], "Researcher");
        assert_eq!(body["model"], "gpt-4");
        assert_eq!(body["description"], "Public first line.");
        assert!(!body.to_string().contains("LEAK_CANARY"));
    }

    #[tokio::test]
    async fn describe_agent_unknown_id_returns_404() {
        let state = registry_state(vec![("r", test_agent("Researcher"))]);
        let err = describe_agent(State(state), Path("missing".to_string()))
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
        assert_eq!(err.to_body()["error"]["code"], "not_found");
    }

    #[test]
    fn openapi_spec_contains_agent_operation_paths() {
        let state = AgentApiState::new(Arc::new(AgentRegistry::new()));
        let (_router, api) = agent_openapi_router(state).split_for_parts();
        let paths = &api.paths.paths;
        for expected in [
            "/agents",
            "/agents/{id}",
            "/agents/{id}/execute",
            "/agents/{id}/execute/stream",
            "/agents/{id}/jobs",
            "/agents/{id}/jobs/{job_id}",
        ] {
            assert!(paths.contains_key(expected), "spec missing path {expected}");
        }
    }

    #[test]
    fn execute_response_from_paladin_result_maps_fields_and_label() {
        let result = PaladinResult::new("hi".to_string(), 7, 42, 2, StopReason::MaxLoops);
        let response = ExecuteResponse::from(result);
        assert_eq!(response.output, "hi");
        assert_eq!(response.token_count, 7);
        assert_eq!(response.execution_time_ms, 42);
        assert_eq!(response.loop_count, 2);
        assert_eq!(response.stop_reason, "max_loops");
    }

    #[test]
    fn stop_reason_labels_are_stable() {
        assert_eq!(stop_reason_label(&StopReason::Completed), "completed");
        assert_eq!(stop_reason_label(&StopReason::MaxLoops), "max_loops");
        assert_eq!(
            stop_reason_label(&StopReason::StopWord("x".to_string())),
            "stop_word"
        );
        assert_eq!(stop_reason_label(&StopReason::Timeout), "timeout");
    }

    #[test]
    fn agent_summary_previews_prompt_and_omits_full_text() {
        let data = paladin_core::platform::container::paladin::PaladinData {
            system_prompt: "First line of behavior.\nSecret second line.".to_string(),
            name: "Researcher".to_string(),
            model: "gpt-4".to_string(),
            ..Default::default()
        };
        let paladin = Paladin::new(data, Some("researcher".to_string()));
        let summary = AgentSummary::from_agent("researcher", &paladin);

        assert_eq!(summary.id, "researcher");
        assert_eq!(summary.name, "Researcher");
        assert_eq!(summary.model, "gpt-4");
        // Only the first line is previewed; the second line is not exposed.
        assert_eq!(summary.description, "First line of behavior.");
        assert!(!summary.description.contains("Secret second line"));
    }

    #[test]
    fn long_prompt_preview_is_truncated_with_ellipsis() {
        let long = "a".repeat(DESCRIPTION_PREVIEW_LEN + 50);
        let preview = prompt_preview(&long);
        assert_eq!(preview.chars().count(), DESCRIPTION_PREVIEW_LEN + 1); // +1 for the ellipsis
        assert!(preview.ends_with('…'));
    }
}
