//! Axum HTTP controller for inspecting, resuming, and paging through threads
//! (HITL-05, HITL-FR-16, D-24/D-25/D-27).
//!
//! This module mirrors [`crate::agent_controller`]'s conventions on a router
//! and state struct of its own, so [`crate::agent_controller::AgentApiState`]
//! stays untouched (D-24, X-10.3):
//!
//! | Method & path | Description |
//! |---------------|-------------|
//! | `GET /threads/{id}/state` | The thread's latest status, plus outstanding parleys/responses when suspended |
//! | `POST /threads/{id}/resume` | Deliver parley responses; `202 Accepted`, poll `state` for the outcome |
//! | `GET /threads/{id}/history` | Paginated Chronicle history (`limit` + opaque `cursor`) |
//!
//! [`ThreadApiState`] holds `Option<Arc<dyn WaypointPort>>` and
//! `Option<Arc<dyn ParleyPort>>` -- both `paladin-ports` trait objects, never
//! a `paladin-battalion` type -- so this crate takes no dependency on
//! `paladin-battalion` in its default build (ADR-0031). When either field is
//! `None` (no waypoint backend configured), every route in this module
//! answers `501 not_implemented` naming the config key to set, per D-24.
//!
//! A success body is the serialized payload; failures use the unified
//! [`ApiError`](crate::error::ApiError) envelope
//! (`{ "error": { "code", "message", "details" } }`), exactly as
//! [`crate::agent_controller`] does. `POST /threads/{id}/resume`'s status
//! mapping follows D-25 precisely: `404` for an unknown thread; `409` for
//! both `thread_not_awaiting_input` and `graph_not_registered` (distinct
//! envelope `code`s, same HTTP status); `400` for `unknown_parley_id` /
//! `parley_already_answered` / `response_shape_invalid` / `parley_expired`
//! (the offending `parley_id` in `details`); `501` when unwired.
//!
//! ## Authorization (narrows D-24 pending PLAT-06)
//!
//! D-24 scoped these routes to "the same auth middleware as `/v1/agents/*`,
//! authenticated callers, any role; scopes are PLAT-06 (Phase 27)". Per
//! CR-01 (24-REVIEW.md), that was too permissive for the one *mutating*
//! route: any authenticated credential -- including the lowest-privileged
//! configured role -- could drive `POST .../resume` for a thread it had no
//! relationship to, since neither `ThreadId` nor `Waypoint` carry an
//! owner/principal to scope against yet. Until PLAT-06 lands real per-thread
//! ownership, `resume_thread` additionally requires [`crate::agent_auth::require_admin`]
//! (matching `agent_controller.rs`'s own admin-gated routes); `get_thread_state`
//! and `get_thread_history` remain authenticated-any-role, as D-24 specified --
//! reads carry lower blast radius than driving execution forward, and a
//! coarser per-thread read scope is exactly what PLAT-06 is expected to add.
//! Every credential configured for `/v1/threads/*/resume` must be treated as
//! admin-equivalent until then.

use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use paladin_core::platform::container::parley::{ParleyKind, ParleyRequest, ParleyResponse};
use paladin_core::platform::container::waypoint::{ThreadId, Waypoint, WaypointId, WaypointStatus};
use paladin_ports::input::parley_port::{ParleyError, ParleyPort};
use paladin_ports::output::waypoint_port::{WaypointPort, WaypointSummary};

use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::agent_auth::{HasAgentAuth, Principal, require_admin};
use crate::agent_controller::{JsonValue, ok_body};
use crate::error::{ApiError, ApiErrorBody};

/// Shared state for the thread routes.
///
/// Mirrors [`crate::agent_controller::AgentApiState`]'s injection-only
/// trait-object shape (D-24): both `waypoints` and `parley` are `None` until
/// a durable Waypoint backend is configured, at which point
/// `src/bin/paladin-server.rs` wires both from the SAME store. Kept as its
/// own struct -- never a field added to `AgentApiState` -- so the
/// pre-existing agent surface is untouched (X-10.3).
#[derive(Clone)]
pub struct ThreadApiState {
    /// Reads the latest Waypoint / paginated history for `GET .../state` and
    /// `GET .../history`. `None` when no waypoint backend is configured.
    pub waypoints: Option<Arc<dyn WaypointPort>>,
    /// Delivers parley responses for `POST .../resume`. `None` when no
    /// waypoint backend is configured (the facade adapter needs one to
    /// resolve a thread's latest Waypoint).
    pub parley: Option<Arc<dyn ParleyPort>>,
    /// Authentication configuration -- the SAME [`crate::agent_auth::AgentAuthConfig`]
    /// shape `AgentApiState` carries, so the thread routes are authenticated
    /// exactly like `/v1/agents/*` (D-24).
    pub auth: crate::agent_auth::AgentAuthConfig,
}

impl ThreadApiState {
    /// Construct state with no waypoint backend wired (every route answers
    /// `501` until [`Self::with_waypoints`]/[`Self::with_parley`] are
    /// applied).
    pub fn new() -> Self {
        Self {
            waypoints: None,
            parley: None,
            auth: crate::agent_auth::AgentAuthConfig::default(),
        }
    }

    /// Wire a durable [`WaypointPort`], enabling `GET .../state` and
    /// `GET .../history`.
    pub fn with_waypoints(mut self, waypoints: Arc<dyn WaypointPort>) -> Self {
        self.waypoints = Some(waypoints);
        self
    }

    /// Wire a [`ParleyPort`], enabling `POST .../resume`.
    pub fn with_parley(mut self, parley: Arc<dyn ParleyPort>) -> Self {
        self.parley = Some(parley);
        self
    }

    /// Set the authentication configuration.
    pub fn with_auth(mut self, auth: crate::agent_auth::AgentAuthConfig) -> Self {
        self.auth = auth;
        self
    }
}

impl Default for ThreadApiState {
    fn default() -> Self {
        Self::new()
    }
}

impl HasAgentAuth for ThreadApiState {
    fn agent_auth(&self) -> &crate::agent_auth::AgentAuthConfig {
        &self.auth
    }
}

// --- Config hints (501 envelopes name the config KEY, never a value) -------

/// Message for every thread route when [`ThreadApiState::waypoints`] is
/// `None`. Names the config key an operator sets, per D-24 -- never a
/// connection string or other value (T-24-49).
const WAYPOINT_BACKEND_HINT: &str =
    "no waypoint backend is configured; set APP_WAYPOINT_STORE_BACKEND (sqlite|postgres)";

/// Message for `POST .../resume` when [`ThreadApiState::parley`] is `None`.
const PARLEY_PORT_HINT: &str =
    "no waypoint backend is configured; resume requires APP_WAYPOINT_STORE_BACKEND to be set";

/// Maximum `limit` accepted by `GET .../history` (D-27, PLAT-FR-16
/// pre-conformance); a larger value is a `400`.
const MAX_HISTORY_LIMIT: u32 = 100;

// --- Label helpers (stable, lowercase wire labels) -------------------------

/// Map a [`WaypointStatus`] to a stable, lowercase wire label.
///
/// `WaypointStatus` is `#[non_exhaustive]`: the wildcard arm fails safe to
/// `"unknown"` for a future variant this crate does not yet recognise,
/// rather than refusing to compile against a widened core enum.
fn waypoint_status_label(status: &WaypointStatus) -> &'static str {
    match status {
        WaypointStatus::Running => "running",
        WaypointStatus::Completed => "completed",
        WaypointStatus::Failed { .. } => "failed",
        WaypointStatus::AwaitingInput { .. } => "awaiting_input",
        WaypointStatus::Halted => "halted",
        _ => "unknown",
    }
}

/// Map a [`ParleyKind`] to a stable, lowercase wire label. `#[non_exhaustive]`,
/// mirroring [`waypoint_status_label`]'s own fail-safe wildcard.
fn parley_kind_label(kind: &ParleyKind) -> String {
    match kind {
        ParleyKind::Approval => "approval".to_string(),
        ParleyKind::Choice => "choice".to_string(),
        ParleyKind::FreeText => "free_text".to_string(),
        ParleyKind::StateEdit => "state_edit".to_string(),
        other => format!("{other:?}").to_lowercase(),
    }
}

// --- Parsing helpers --------------------------------------------------------

/// Parse a path segment as a [`ThreadId`], mapping a validation failure to
/// `400 bad_request`.
fn parse_thread_id(raw: &str) -> Result<ThreadId, ApiError> {
    ThreadId::new(raw).map_err(|e| ApiError::bad_request(e.to_string()))
}

/// Parse a `WaypointId` from its serialized (UUID) string form -- used for
/// the opaque `cursor` query parameter. `WaypointId` is `#[serde(transparent)]`
/// over a `Uuid`, so round-tripping through a JSON string value reuses its
/// own `Deserialize` impl rather than requiring a public `from_uuid`
/// constructor this crate cannot add (core type, ADR-0016).
fn parse_waypoint_id(raw: &str) -> Result<WaypointId, ApiError> {
    serde_json::from_value(serde_json::Value::String(raw.to_string()))
        .map_err(|_| ApiError::bad_request(format!("invalid cursor: '{raw}'")))
}

/// Parse a `ParleyId` from its serialized (UUID) string form, mirroring
/// [`parse_waypoint_id`]'s `#[serde(transparent)]` trick.
fn parse_parley_id(
    raw: &str,
) -> Result<paladin_core::platform::container::parley::ParleyId, ApiError> {
    serde_json::from_value(serde_json::Value::String(raw.to_string()))
        .map_err(|_| ApiError::bad_request(format!("invalid parley_id: '{raw}'")))
}

// --- DTOs --------------------------------------------------------------

/// Wire projection of a [`ParleyRequest`] (a `paladin-core` type) into a
/// `utoipa::ToSchema` type this crate owns (ADR-0038).
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ParleyRequestDto {
    /// The outstanding parley's identity.
    pub parley_id: String,
    /// The node that raised this parley.
    pub node_id: String,
    /// The shape of input awaited (`"approval"`, `"choice"`, `"free_text"`, `"state_edit"`).
    pub kind: String,
    /// Free-form prompt describing what input is being awaited.
    pub prompt: String,
    /// Author-supplied context rendered alongside `prompt`.
    #[schema(value_type = Object)]
    pub payload: serde_json::Value,
    /// Valid choices, populated for `"choice"`; `None` otherwise.
    pub choices: Option<Vec<String>>,
    /// When this parley expires, if ever.
    pub expires_at: Option<DateTime<Utc>>,
    /// When this parley was raised.
    pub created_at: DateTime<Utc>,
}

impl From<&ParleyRequest> for ParleyRequestDto {
    fn from(r: &ParleyRequest) -> Self {
        Self {
            parley_id: r.parley_id.to_string(),
            node_id: r.node_id.to_string(),
            kind: parley_kind_label(&r.kind),
            prompt: r.prompt.clone(),
            payload: r.payload.clone(),
            choices: r.choices.clone(),
            expires_at: r.expires_at,
            created_at: r.created_at,
        }
    }
}

/// Wire projection of a [`ParleyResponse`].
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ParleyResponseDto {
    /// The parley this response answers.
    pub parley_id: String,
    /// The originating parley's `kind`.
    pub kind: String,
    /// The originating parley's `prompt`.
    pub prompt: String,
    /// The submitted value.
    #[schema(value_type = Object)]
    pub value: serde_json::Value,
    /// Who submitted this response, if known.
    pub responded_by: Option<String>,
    /// When this response was recorded.
    pub responded_at: DateTime<Utc>,
    /// Whether this response was substituted by an `on_expire` default
    /// rather than submitted by a caller.
    pub defaulted: bool,
}

impl From<&ParleyResponse> for ParleyResponseDto {
    fn from(r: &ParleyResponse) -> Self {
        Self {
            parley_id: r.parley_id.to_string(),
            kind: parley_kind_label(&r.kind),
            prompt: r.prompt.clone(),
            value: r.value.clone(),
            responded_by: r.responded_by.clone(),
            responded_at: r.responded_at,
            defaulted: r.defaulted,
        }
    }
}

/// Response body for `GET /threads/{id}/state`.
///
/// `parleys`/`responses` are always present, empty unless `status` is
/// `"awaiting_input"` (Test 2: a running/completed thread carries the
/// summary fields with no outstanding parleys, not an omitted key).
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ThreadStateResponse {
    /// The thread's identity.
    pub thread_id: String,
    /// Stable, lowercase status label.
    pub status: String,
    /// The superstep index of the returned Waypoint.
    pub superstep: u64,
    /// The returned Waypoint's own identity.
    pub waypoint_id: String,
    /// The ROOT `WaypointId` of the branch this Waypoint belongs to;
    /// `None` for a mainline Waypoint (HITL-03, D-14).
    pub fork_of: Option<String>,
    /// Every parley raised in the suspending superstep, when `status` is
    /// `"awaiting_input"`; empty otherwise.
    pub parleys: Vec<ParleyRequestDto>,
    /// The accepted subset of responses so far, when `status` is
    /// `"awaiting_input"`; empty otherwise.
    pub responses: Vec<ParleyResponseDto>,
}

impl From<&Waypoint> for ThreadStateResponse {
    fn from(wp: &Waypoint) -> Self {
        let (parleys, responses) = match &wp.status {
            WaypointStatus::AwaitingInput { parleys, responses } => (
                parleys.iter().map(ParleyRequestDto::from).collect(),
                responses.iter().map(ParleyResponseDto::from).collect(),
            ),
            _ => (Vec::new(), Vec::new()),
        };
        Self {
            thread_id: wp.thread_id.as_str().to_string(),
            status: waypoint_status_label(&wp.status).to_string(),
            superstep: wp.superstep,
            waypoint_id: wp.waypoint_id.to_string(),
            fork_of: wp.fork_of.map(|id| id.to_string()),
            parleys,
            responses,
        }
    }
}

/// One submitted answer inside a [`ResumeRequest`].
///
/// `kind`/`prompt` are intentionally absent: `WarEngine::resume_with`
/// stamps both onto the persisted response from the matching request's own
/// fields, regardless of what a caller supplies (D-07) -- so this DTO never
/// asks a client to know or repeat them.
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct ParleyResponseInput {
    /// The parley this response answers.
    pub parley_id: String,
    /// The submitted value, validated per the parley's own `kind`.
    #[schema(value_type = Object)]
    pub value: serde_json::Value,
    /// Who is submitting this response, if the caller wants it recorded.
    #[serde(default)]
    pub responded_by: Option<String>,
}

/// Request body for `POST /threads/{id}/resume`.
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct ResumeRequest {
    /// Every response to submit in this call.
    #[serde(default)]
    pub responses: Vec<ParleyResponseInput>,
}

/// Response body for a successful `POST /threads/{id}/resume` (`202 Accepted`).
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ResumeAcceptedResponse {
    /// The thread whose resume was accepted.
    pub thread_id: String,
    /// The URL a client polls (`GET .../state`) to observe the outcome.
    pub state_url: String,
}

/// Wire projection of a [`WaypointSummary`].
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct WaypointSummaryDto {
    /// The summarized waypoint's identity.
    pub waypoint_id: String,
    /// The waypoint this one was checkpointed from.
    pub parent_waypoint_id: Option<String>,
    /// The superstep index that produced this waypoint.
    pub superstep: u64,
    /// Stable, lowercase status label.
    pub status: String,
    /// When this waypoint was created.
    pub created_at: DateTime<Utc>,
    /// The ROOT `WaypointId` of the branch this waypoint belongs to;
    /// `None` for a mainline waypoint.
    pub fork_of: Option<String>,
}

impl From<&WaypointSummary> for WaypointSummaryDto {
    fn from(s: &WaypointSummary) -> Self {
        Self {
            waypoint_id: s.waypoint_id.to_string(),
            parent_waypoint_id: s.parent_waypoint_id.map(|id| id.to_string()),
            superstep: s.superstep,
            status: waypoint_status_label(&s.status).to_string(),
            created_at: s.created_at,
            fork_of: s.fork_of.map(|id| id.to_string()),
        }
    }
}

/// Response body for `GET /threads/{id}/history`.
///
/// `next_cursor` is `None` on the last page; otherwise it is the last
/// returned item's `waypoint_id`, opaque to clients (D-27) -- its internal
/// structure may change (PLAT-06) without this being a breaking change.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct HistoryResponse {
    /// The page of waypoint summaries, newest-first.
    pub items: Vec<WaypointSummaryDto>,
    /// Opaque cursor for the next page, or `None` on the last page.
    pub next_cursor: Option<String>,
}

/// Query parameters for `GET /threads/{id}/history`.
#[derive(Debug, Clone, Deserialize)]
pub struct HistoryQuery {
    /// Maximum number of items to return (at most [`MAX_HISTORY_LIMIT`]).
    #[serde(default)]
    pub limit: Option<u32>,
    /// Opaque pagination cursor from a previous page's `next_cursor`.
    #[serde(default)]
    pub cursor: Option<String>,
}

// --- Error mapping (D-25) ---------------------------------------------------

/// Map a [`ParleyError`] onto the [`ApiError`] status/code D-25 specifies:
/// `404` for `ThreadNotFound`; `409` for `ThreadNotAwaitingInput` AND
/// `GraphNotRegistered` with DISTINCT `code`s; `400` for
/// `UnknownParleyId`/`ParleyAlreadyAnswered`/`ResponseShapeInvalid`/
/// `ParleyExpired` with the parley id in `details`; anything else (a genuine
/// backend failure, or a future variant `ParleyError`'s `#[non_exhaustive]`
/// status permits) is `500 internal` -- never silently 200'd or panicked on.
fn map_parley_error(err: ParleyError) -> ApiError {
    match err {
        ParleyError::ThreadNotFound(thread) => {
            ApiError::not_found(format!("unknown thread '{thread}'"))
        }
        ParleyError::GraphNotRegistered { fingerprint } => ApiError::new(
            StatusCode::CONFLICT,
            "graph_not_registered",
            format!("no graph registered for fingerprint {fingerprint}"),
        ),
        ParleyError::ThreadNotAwaitingInput { thread, status } => ApiError::new(
            StatusCode::CONFLICT,
            "thread_not_awaiting_input",
            format!("thread '{thread}' is not awaiting input"),
        )
        .with_details(serde_json::json!({ "status": status })),
        ParleyError::UnknownParleyId { parley_id } => {
            ApiError::bad_request(format!("unknown parley id: {parley_id}"))
                .with_details(serde_json::json!({ "parley_id": parley_id.to_string() }))
        }
        ParleyError::ParleyAlreadyAnswered { parley_id } => {
            ApiError::bad_request(format!("parley already answered: {parley_id}"))
                .with_details(serde_json::json!({ "parley_id": parley_id.to_string() }))
        }
        ParleyError::ResponseShapeInvalid { parley_id, reason } => ApiError::bad_request(format!(
            "parley {parley_id} response shape invalid: {reason}"
        ))
        .with_details(serde_json::json!({ "parley_id": parley_id.to_string() })),
        ParleyError::ParleyExpired {
            parley_id,
            expires_at,
        } => ApiError::bad_request(format!("parley {parley_id} expired at {expires_at}"))
            .with_details(serde_json::json!({ "parley_id": parley_id.to_string() })),
        other => ApiError::internal(other.to_string()),
    }
}

// --- Handlers ---------------------------------------------------------------

/// `GET /threads/{id}/state` -- the thread's latest status, plus outstanding
/// parleys/responses when suspended.
///
/// Authenticated, any role (D-24) -- reads carry lower blast radius than
/// `resume_thread`; per-thread ownership scoping is PLAT-06 (Phase 27). The
/// `Principal` is extracted (unused today) so this handler's signature is
/// already shaped for that scoping to land as a body-only change.
///
/// Returns:
/// - `200 OK` with [`ThreadStateResponse`] on success;
/// - `400 Bad Request` for an invalid thread id;
/// - `404 Not Found` if no Waypoint exists for `id`;
/// - `501 Not Implemented` if no waypoint backend is configured.
#[utoipa::path(
    get,
    path = "/threads/{id}/state",
    tag = "threads",
    params(("id" = String, Path, description = "Thread id")),
    responses(
        (status = 200, description = "Thread state", body = ThreadStateResponse),
        (status = 400, description = "Invalid thread id", body = ApiErrorBody),
        (status = 401, description = "Missing/invalid credentials", body = ApiErrorBody),
        (status = 404, description = "Unknown thread", body = ApiErrorBody),
        (status = 501, description = "No waypoint backend configured", body = ApiErrorBody),
    ),
    security(("api_key" = []), ("bearer_token" = [])),
)]
pub async fn get_thread_state(
    State(state): State<ThreadApiState>,
    Extension(_principal): Extension<Principal>,
    Path(id): Path<String>,
) -> Result<(StatusCode, JsonValue), ApiError> {
    let waypoints = state
        .waypoints
        .as_ref()
        .ok_or_else(|| ApiError::not_implemented(WAYPOINT_BACKEND_HINT))?;
    let thread = parse_thread_id(&id)?;

    let waypoint = waypoints
        .latest(&thread)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or_else(|| ApiError::not_found(format!("unknown thread '{id}'")))?;

    Ok((
        StatusCode::OK,
        ok_body(&ThreadStateResponse::from(&waypoint)),
    ))
}

/// `POST /threads/{id}/resume` -- deliver parley responses.
///
/// Requires an admin principal ([`require_admin`]) -- see the module-level
/// "Authorization" docs: this narrows D-24's "any role" default for the one
/// *mutating* thread route (CR-01, 24-REVIEW.md), pending PLAT-06's real
/// per-thread ownership scoping.
///
/// Validates synchronously through [`ParleyPort::resume_with`] and returns
/// `202 Accepted` immediately; the run itself continues on a background task
/// (D-25) -- a client polls `GET .../state` to observe the outcome. Maps
/// [`ParleyError`] per [`map_parley_error`].
#[utoipa::path(
    post,
    path = "/threads/{id}/resume",
    tag = "threads",
    params(("id" = String, Path, description = "Thread id")),
    request_body = ResumeRequest,
    responses(
        (status = 202, description = "Resume accepted; poll GET /threads/{id}/state for the outcome", body = ResumeAcceptedResponse),
        (status = 400, description = "One of unknown_parley_id / parley_already_answered / response_shape_invalid / parley_expired; the offending parley_id is in `details`", body = ApiErrorBody),
        (status = 401, description = "Missing/invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Admin role required", body = ApiErrorBody),
        (status = 404, description = "Unknown thread", body = ApiErrorBody),
        (status = 409, description = "Same HTTP status, two distinct envelope codes: `thread_not_awaiting_input` (the thread is not suspended) or `graph_not_registered` (the thread's graph fingerprint has no registered WarGraph in this process)", body = ApiErrorBody),
        (status = 501, description = "No waypoint backend configured", body = ApiErrorBody),
    ),
    security(("api_key" = []), ("bearer_token" = [])),
)]
pub async fn resume_thread(
    State(state): State<ThreadApiState>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    Json(body): Json<ResumeRequest>,
) -> Result<(StatusCode, JsonValue), ApiError> {
    require_admin(&principal)?;
    let parley = state
        .parley
        .as_ref()
        .ok_or_else(|| ApiError::not_implemented(PARLEY_PORT_HINT))?;
    let thread = parse_thread_id(&id)?;

    let mut responses = Vec::with_capacity(body.responses.len());
    for input in body.responses {
        let parley_id = parse_parley_id(&input.parley_id)?;
        responses.push(ParleyResponse {
            parley_id,
            // Placeholders: `WarEngine::resume_with` stamps both from the
            // matching request regardless of what is supplied here (D-07).
            kind: ParleyKind::FreeText,
            prompt: String::new(),
            value: input.value,
            responded_by: input.responded_by,
            responded_at: Utc::now(),
            defaulted: false,
        });
    }

    let accepted = parley
        .resume_with(&thread, responses)
        .await
        .map_err(map_parley_error)?;

    let thread_id = accepted.thread_id().as_str().to_string();
    let state_url = format!(
        "{}/threads/{thread_id}/state",
        crate::agent_controller::API_V1_PREFIX
    );
    Ok((
        StatusCode::ACCEPTED,
        ok_body(&ResumeAcceptedResponse {
            thread_id,
            state_url,
        }),
    ))
}

/// `GET /threads/{id}/history` -- paginated Chronicle history.
///
/// Authenticated, any role (D-24) -- see [`get_thread_state`]'s docs on why
/// reads are not admin-gated the way [`resume_thread`] is.
///
/// `limit` (at most [`MAX_HISTORY_LIMIT`]) and an opaque `cursor` whose
/// content is the last returned item's `waypoint_id` (D-27). Returns
/// `400 Bad Request` for `limit > 100` or an unparseable `cursor`.
#[utoipa::path(
    get,
    path = "/threads/{id}/history",
    tag = "threads",
    params(
        ("id" = String, Path, description = "Thread id"),
        ("limit" = Option<u32>, Query, description = "Max items to return (at most 100)"),
        ("cursor" = Option<String>, Query, description = "Opaque pagination cursor from a previous page's next_cursor"),
    ),
    responses(
        (status = 200, description = "A page of waypoint summaries", body = HistoryResponse),
        (status = 400, description = "limit exceeds 100, or cursor is not a valid waypoint id", body = ApiErrorBody),
        (status = 401, description = "Missing/invalid credentials", body = ApiErrorBody),
        (status = 501, description = "No waypoint backend configured", body = ApiErrorBody),
    ),
    security(("api_key" = []), ("bearer_token" = [])),
)]
pub async fn get_thread_history(
    State(state): State<ThreadApiState>,
    Extension(_principal): Extension<Principal>,
    Path(id): Path<String>,
    Query(params): Query<HistoryQuery>,
) -> Result<(StatusCode, JsonValue), ApiError> {
    let waypoints = state
        .waypoints
        .as_ref()
        .ok_or_else(|| ApiError::not_implemented(WAYPOINT_BACKEND_HINT))?;
    let thread = parse_thread_id(&id)?;

    if let Some(limit) = params.limit
        && limit > MAX_HISTORY_LIMIT
    {
        return Err(ApiError::bad_request(format!(
            "limit must be at most {MAX_HISTORY_LIMIT}, got {limit}"
        )));
    }
    let before = params
        .cursor
        .as_deref()
        .map(parse_waypoint_id)
        .transpose()?;

    let page = waypoints
        .history(&thread, params.limit, before)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    // A full page (== the requested limit) may have more behind it; a
    // shorter page (or no limit at all, i.e. "everything") is the last one.
    let is_full_page = params
        .limit
        .map(|limit| page.len() as u32 >= limit && limit > 0)
        .unwrap_or(false);
    let next_cursor = if is_full_page {
        page.last().map(|s| s.waypoint_id.to_string())
    } else {
        None
    };

    let items: Vec<WaypointSummaryDto> = page.iter().map(WaypointSummaryDto::from).collect();
    Ok((
        StatusCode::OK,
        ok_body(&HistoryResponse { items, next_cursor }),
    ))
}

// --- Router -----------------------------------------------------------------

/// Build the thread API as a `utoipa-axum` [`OpenApiRouter`], mirroring
/// [`crate::agent_controller::agent_openapi_router`]'s composition 1:1:
/// routes declared unprefixed (`/threads/...`; the `/v1` segment is added on
/// nesting), the SAME [`crate::agent_auth::require_authentication`]
/// middleware layered via [`crate::agent_auth::HasAgentAuth`], and the
/// document assembled from the SAME `#[utoipa::path]` annotations the routes
/// themselves carry -- so the served API and the generated spec cannot
/// drift, and the spec lists these paths even when no backend is wired
/// (D-24).
pub fn thread_openapi_router(state: ThreadApiState) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(get_thread_state))
        .routes(routes!(resume_thread))
        .routes(routes!(get_thread_history))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::agent_auth::require_authentication::<ThreadApiState>,
        ))
        .with_state(state)
}

/// Assemble the thread API nested under [`crate::agent_controller::API_V1_PREFIX`]
/// into an `axum` [`axum::Router`] and its raw OpenAPI document, mirroring
/// [`crate::agent_controller::versioned_agent_parts`].
pub(crate) fn versioned_thread_parts(
    state: ThreadApiState,
) -> (axum::Router, utoipa::openapi::OpenApi) {
    OpenApiRouter::new()
        .nest(
            crate::agent_controller::API_V1_PREFIX,
            thread_openapi_router(state),
        )
        .split_for_parts()
}

/// Build the thread router as a plain `axum` [`axum::Router`]
/// (`/v1/threads/...`). Merged by `src/bin/paladin-server.rs` ALONGSIDE
/// [`crate::agent_controller::agent_router`]'s output -- never inside it --
/// so `AgentApiState` stays untouched (D-24).
pub fn thread_router(state: ThreadApiState) -> axum::Router {
    let (routes, _api) = versioned_thread_parts(state);
    routes
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use axum::Extension;
    use axum::body::Body;
    use axum::http::Request;
    use paladin_core::platform::container::battlefield::{Battlefield, BattlefieldSchema};
    use paladin_core::platform::container::parley::{OnExpire, ParleyId};
    use paladin_core::platform::container::user::UserRole;
    use paladin_core::platform::container::waypoint::{FrontierSnapshot, GraphFingerprint, NodeId};
    use paladin_ports::input::parley_port::ResumeAccepted;
    use paladin_ports::output::waypoint_port::{ThreadSummary, WaypointError};
    use std::collections::BTreeMap;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use tower::ServiceExt; // for `Router::oneshot`

    use crate::agent_auth::Principal;

    /// An admin `Principal` extension for direct handler calls (bypasses
    /// HTTP auth plumbing), mirroring `agent_controller`'s own `admin()`.
    fn admin() -> Extension<Principal> {
        Extension(Principal {
            id: "test-admin".to_string(),
            role: UserRole::Admin,
        })
    }

    fn thread(name: &str) -> ThreadId {
        ThreadId::new(name).unwrap()
    }

    fn sample_waypoint(thread: &ThreadId, superstep: u64, status: WaypointStatus) -> Waypoint {
        let mut wp = Waypoint::new_root(
            thread.clone(),
            superstep,
            GraphFingerprint::from_canonical_bytes(b"thread-controller-test-graph"),
            Battlefield::new(BattlefieldSchema::new(Vec::new())),
            Vec::new(),
            Vec::new(),
            status,
            BTreeMap::new(),
            FrontierSnapshot::default(),
        );
        wp.waypoint_id = WaypointId::generate();
        wp
    }

    fn sample_parley_request(expires_at: Option<DateTime<Utc>>) -> ParleyRequest {
        ParleyRequest {
            parley_id: ParleyId::new(),
            node_id: NodeId::new("approve"),
            kind: ParleyKind::Approval,
            prompt: "confirm?".to_string(),
            payload: serde_json::json!({}),
            choices: None,
            expires_at,
            created_at: Utc::now(),
            on_expire: OnExpire::FailRun,
        }
    }

    // --- Mock `WaypointPort` -------------------------------------------

    #[derive(Default)]
    struct MockWaypointStore {
        latest: Mutex<HashMap<String, Waypoint>>,
        history: Mutex<HashMap<String, Vec<WaypointSummary>>>,
    }

    impl MockWaypointStore {
        fn seed_latest(&self, wp: Waypoint) {
            self.latest
                .lock()
                .unwrap()
                .insert(wp.thread_id.as_str().to_string(), wp);
        }

        fn seed_history(&self, thread: &ThreadId, items: Vec<WaypointSummary>) {
            self.history
                .lock()
                .unwrap()
                .insert(thread.as_str().to_string(), items);
        }
    }

    #[async_trait]
    impl WaypointPort for MockWaypointStore {
        async fn save(&self, wp: &Waypoint) -> Result<(), WaypointError> {
            self.seed_latest(wp.clone());
            Ok(())
        }

        async fn latest(&self, thread: &ThreadId) -> Result<Option<Waypoint>, WaypointError> {
            Ok(self.latest.lock().unwrap().get(thread.as_str()).cloned())
        }

        async fn get(
            &self,
            _thread: &ThreadId,
            _id: &WaypointId,
        ) -> Result<Option<Waypoint>, WaypointError> {
            Ok(None)
        }

        async fn history(
            &self,
            thread: &ThreadId,
            limit: Option<u32>,
            before: Option<WaypointId>,
        ) -> Result<Vec<WaypointSummary>, WaypointError> {
            let all = self
                .history
                .lock()
                .unwrap()
                .get(thread.as_str())
                .cloned()
                .unwrap_or_default();
            let start = match before {
                Some(cursor) => all
                    .iter()
                    .position(|s| s.waypoint_id == cursor)
                    .map(|i| i + 1)
                    .unwrap_or(all.len()),
                None => 0,
            };
            let slice = &all[start.min(all.len())..];
            let page: Vec<WaypointSummary> = match limit {
                Some(l) => slice.iter().take(l as usize).cloned().collect(),
                None => slice.to_vec(),
            };
            Ok(page)
        }

        async fn list_threads(
            &self,
            _limit: Option<u32>,
            _before: Option<DateTime<Utc>>,
        ) -> Result<Vec<ThreadSummary>, WaypointError> {
            Ok(vec![])
        }

        async fn delete_thread(&self, _thread: &ThreadId) -> Result<u64, WaypointError> {
            Ok(0)
        }

        async fn delete_waypoint(
            &self,
            _thread: &ThreadId,
            _id: &WaypointId,
        ) -> Result<bool, WaypointError> {
            Ok(false)
        }
    }

    // --- Mock `ParleyPort` -----------------------------------------------

    enum MockOutcome {
        Accepted,
        ThreadNotFound,
        GraphNotRegistered,
        ThreadNotAwaitingInput,
        UnknownParleyId,
        ParleyAlreadyAnswered,
        ResponseShapeInvalid,
        ParleyExpired,
    }

    struct MockParleyPort {
        outcome: MockOutcome,
        parley_id: ParleyId,
    }

    #[async_trait]
    impl ParleyPort for MockParleyPort {
        async fn resume_with(
            &self,
            thread: &ThreadId,
            _responses: Vec<ParleyResponse>,
        ) -> Result<ResumeAccepted, ParleyError> {
            match self.outcome {
                MockOutcome::Accepted => Ok(ResumeAccepted::new(thread.clone())),
                MockOutcome::ThreadNotFound => Err(ParleyError::ThreadNotFound(thread.clone())),
                MockOutcome::GraphNotRegistered => Err(ParleyError::GraphNotRegistered {
                    fingerprint: GraphFingerprint::from_canonical_bytes(b"unregistered"),
                }),
                MockOutcome::ThreadNotAwaitingInput => Err(ParleyError::ThreadNotAwaitingInput {
                    thread: thread.clone(),
                    status: "Running".to_string(),
                }),
                MockOutcome::UnknownParleyId => Err(ParleyError::UnknownParleyId {
                    parley_id: self.parley_id,
                }),
                MockOutcome::ParleyAlreadyAnswered => Err(ParleyError::ParleyAlreadyAnswered {
                    parley_id: self.parley_id,
                }),
                MockOutcome::ResponseShapeInvalid => Err(ParleyError::ResponseShapeInvalid {
                    parley_id: self.parley_id,
                    reason: "bad shape".to_string(),
                }),
                MockOutcome::ParleyExpired => Err(ParleyError::ParleyExpired {
                    parley_id: self.parley_id,
                    expires_at: Utc::now(),
                }),
            }
        }
    }

    fn state_with_waypoints(store: Arc<MockWaypointStore>) -> ThreadApiState {
        ThreadApiState::new().with_waypoints(store)
    }

    fn state_with_parley(outcome: MockOutcome, parley_id: ParleyId) -> ThreadApiState {
        // A resume call also needs a waypoint backend wired (D-24: `parley`
        // and `waypoints` come from the SAME store in production; the mocks
        // are independent here since only `parley` is exercised).
        ThreadApiState::new()
            .with_waypoints(Arc::new(MockWaypointStore::default()))
            .with_parley(Arc::new(MockParleyPort { outcome, parley_id }))
    }

    fn resume_body(parley_id: ParleyId, value: serde_json::Value) -> ResumeRequest {
        ResumeRequest {
            responses: vec![ParleyResponseInput {
                parley_id: parley_id.to_string(),
                value,
                responded_by: Some("alice".to_string()),
            }],
        }
    }

    // --- Test 1/2: state, suspended vs. not ------------------------------

    #[tokio::test]
    async fn get_thread_state_returns_status_and_parleys_when_suspended() {
        let store = Arc::new(MockWaypointStore::default());
        let t = thread("suspended");
        let request = sample_parley_request(None);
        let mut wp = sample_waypoint(
            &t,
            2,
            WaypointStatus::AwaitingInput {
                parleys: vec![request.clone()],
                responses: vec![],
            },
        );
        wp.parent_waypoint_id = Some(WaypointId::generate());
        let waypoint_id = wp.waypoint_id;
        store.seed_latest(wp);
        let state = state_with_waypoints(store);

        let (status, Json(body)) =
            get_thread_state(State(state), admin(), Path(t.as_str().to_string()))
                .await
                .expect("ok");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["thread_id"], t.as_str());
        assert_eq!(body["status"], "awaiting_input");
        assert_eq!(body["superstep"], 2);
        assert_eq!(body["waypoint_id"], waypoint_id.to_string());
        assert_eq!(body["parleys"].as_array().unwrap().len(), 1);
        assert_eq!(
            body["parleys"][0]["parley_id"],
            request.parley_id.to_string()
        );
        assert_eq!(body["responses"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn get_thread_state_omits_parleys_when_not_suspended() {
        let store = Arc::new(MockWaypointStore::default());
        let t = thread("running");
        store.seed_latest(sample_waypoint(&t, 1, WaypointStatus::Running));
        let state = state_with_waypoints(store);

        let (status, Json(body)) =
            get_thread_state(State(state), admin(), Path(t.as_str().to_string()))
                .await
                .expect("ok");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "running");
        assert_eq!(body["parleys"].as_array().unwrap().len(), 0);
        assert_eq!(body["responses"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn get_thread_state_unknown_thread_is_404() {
        let state = state_with_waypoints(Arc::new(MockWaypointStore::default()));
        let err = get_thread_state(State(state), admin(), Path("no-such-thread".to_string()))
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
    }

    // --- Test 4-7: resume status mapping ----------------------------------

    #[tokio::test]
    async fn post_resume_returns_202_with_thread_and_state_url() {
        let t = thread("resumable");
        let parley_id = ParleyId::new();
        let state = state_with_parley(MockOutcome::Accepted, parley_id);

        let (status, Json(body)) = resume_thread(
            State(state),
            admin(),
            Path(t.as_str().to_string()),
            Json(resume_body(parley_id, serde_json::json!(true))),
        )
        .await
        .expect("accepted");
        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(body["thread_id"], t.as_str());
        assert!(
            body["state_url"]
                .as_str()
                .unwrap()
                .ends_with(&format!("/threads/{}/state", t.as_str())),
            "state_url: {body}"
        );
    }

    #[tokio::test]
    async fn post_resume_unknown_thread_is_404() {
        let t = thread("unknown-thread");
        let parley_id = ParleyId::new();
        let state = state_with_parley(MockOutcome::ThreadNotFound, parley_id);

        let err = resume_thread(
            State(state),
            admin(),
            Path(t.as_str().to_string()),
            Json(resume_body(parley_id, serde_json::json!(true))),
        )
        .await
        .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn post_resume_on_running_thread_is_409_thread_not_awaiting_input() {
        let t = thread("running-thread");
        let parley_id = ParleyId::new();
        let state = state_with_parley(MockOutcome::ThreadNotAwaitingInput, parley_id);

        let err = resume_thread(
            State(state),
            admin(),
            Path(t.as_str().to_string()),
            Json(resume_body(parley_id, serde_json::json!(true))),
        )
        .await
        .unwrap_err();
        assert_eq!(err.status(), StatusCode::CONFLICT);
        assert_eq!(err.to_body()["error"]["code"], "thread_not_awaiting_input");
    }

    #[tokio::test]
    async fn post_resume_with_unregistered_graph_is_409_graph_not_registered() {
        let t = thread("unregistered-graph");
        let parley_id = ParleyId::new();
        let state = state_with_parley(MockOutcome::GraphNotRegistered, parley_id);

        let err = resume_thread(
            State(state),
            admin(),
            Path(t.as_str().to_string()),
            Json(resume_body(parley_id, serde_json::json!(true))),
        )
        .await
        .unwrap_err();
        assert_eq!(err.status(), StatusCode::CONFLICT);
        assert_eq!(err.to_body()["error"]["code"], "graph_not_registered");
        assert_ne!(
            err.to_body()["error"]["code"],
            serde_json::json!("thread_not_awaiting_input"),
            "the two 409 cases must carry distinct codes"
        );
    }

    #[tokio::test]
    async fn post_resume_bad_response_is_400_with_parley_id_in_details() {
        let t = thread("bad-response");
        let parley_id = ParleyId::new();
        for outcome in [
            MockOutcome::UnknownParleyId,
            MockOutcome::ParleyAlreadyAnswered,
            MockOutcome::ResponseShapeInvalid,
            MockOutcome::ParleyExpired,
        ] {
            let state = state_with_parley(outcome, parley_id);
            let err = resume_thread(
                State(state),
                admin(),
                Path(t.as_str().to_string()),
                Json(resume_body(parley_id, serde_json::json!(true))),
            )
            .await
            .unwrap_err();
            assert_eq!(err.status(), StatusCode::BAD_REQUEST);
            assert_eq!(
                err.to_body()["error"]["details"]["parley_id"],
                parley_id.to_string()
            );
        }
    }

    // --- Test 8/9: history pagination -------------------------------------

    fn summary_at(id: WaypointId, superstep: u64, created_at: DateTime<Utc>) -> WaypointSummary {
        WaypointSummary {
            waypoint_id: id,
            parent_waypoint_id: None,
            superstep,
            status: WaypointStatus::Completed,
            created_at,
            fork_of: None,
        }
    }

    #[tokio::test]
    async fn get_thread_history_paginates_with_limit_and_cursor() {
        let store = Arc::new(MockWaypointStore::default());
        let t = thread("history-thread");
        let now = Utc::now();
        let ids: Vec<WaypointId> = (0..5).map(|_| WaypointId::generate()).collect();
        let items: Vec<WaypointSummary> = ids
            .iter()
            .enumerate()
            .map(|(i, id)| summary_at(*id, i as u64, now))
            .collect();
        store.seed_history(&t, items);
        let state = state_with_waypoints(store);

        // First page.
        let (status, Json(page1)) = get_thread_history(
            State(state.clone()),
            admin(),
            Path(t.as_str().to_string()),
            Query(HistoryQuery {
                limit: Some(2),
                cursor: None,
            }),
        )
        .await
        .expect("ok");
        assert_eq!(status, StatusCode::OK);
        let page1_items = page1["items"].as_array().unwrap();
        assert_eq!(page1_items.len(), 2);
        assert_eq!(page1_items[0]["waypoint_id"], ids[0].to_string());
        assert_eq!(page1_items[1]["waypoint_id"], ids[1].to_string());
        let cursor = page1["next_cursor"]
            .as_str()
            .expect("more pages")
            .to_string();
        assert_eq!(cursor, ids[1].to_string());

        // Second page: no overlap with the first.
        let (_, Json(page2)) = get_thread_history(
            State(state.clone()),
            admin(),
            Path(t.as_str().to_string()),
            Query(HistoryQuery {
                limit: Some(2),
                cursor: Some(cursor),
            }),
        )
        .await
        .expect("ok");
        let page2_items = page2["items"].as_array().unwrap();
        assert_eq!(page2_items.len(), 2);
        assert_eq!(page2_items[0]["waypoint_id"], ids[2].to_string());
        assert_eq!(page2_items[1]["waypoint_id"], ids[3].to_string());
        let cursor2 = page2["next_cursor"]
            .as_str()
            .expect("more pages")
            .to_string();

        // Last page: fewer than `limit` items, next_cursor is null.
        let (_, Json(page3)) = get_thread_history(
            State(state),
            admin(),
            Path(t.as_str().to_string()),
            Query(HistoryQuery {
                limit: Some(2),
                cursor: Some(cursor2),
            }),
        )
        .await
        .expect("ok");
        let page3_items = page3["items"].as_array().unwrap();
        assert_eq!(page3_items.len(), 1);
        assert_eq!(page3_items[0]["waypoint_id"], ids[4].to_string());
        assert!(page3["next_cursor"].is_null());
    }

    #[tokio::test]
    async fn get_thread_history_rejects_limit_above_100() {
        let state = state_with_waypoints(Arc::new(MockWaypointStore::default()));
        let err = get_thread_history(
            State(state),
            admin(),
            Path("any-thread".to_string()),
            Query(HistoryQuery {
                limit: Some(101),
                cursor: None,
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
    }

    // --- Test 10/11: unwired backend + auth -------------------------------

    #[tokio::test]
    async fn thread_routes_return_501_when_no_backend_is_wired() {
        let state = ThreadApiState::new();

        let err = get_thread_state(State(state.clone()), admin(), Path("t".to_string()))
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_IMPLEMENTED);

        let err = resume_thread(
            State(state.clone()),
            admin(),
            Path("t".to_string()),
            Json(ResumeRequest { responses: vec![] }),
        )
        .await
        .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_IMPLEMENTED);

        let err = get_thread_history(
            State(state),
            admin(),
            Path("t".to_string()),
            Query(HistoryQuery {
                limit: None,
                cursor: None,
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn thread_routes_require_authentication() {
        let auth = crate::agent_auth::AgentAuthConfig {
            enabled: true,
            api_keys: std::collections::HashMap::new(),
            token_verifier: None,
        };
        let state = ThreadApiState::new().with_auth(auth);
        let app = thread_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/threads/some-thread/state")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    // --- CR-01 regression: resume is admin-gated, reads are not -----------

    /// Build an authenticated router wired with both a waypoint backend and
    /// a `parley` port that always accepts, plus two API keys -- one
    /// `UserRole::User`, one `UserRole::Admin` -- for role-scoped HTTP tests.
    fn authed_thread_app(parley_id: ParleyId) -> axum::Router {
        let mut api_keys = HashMap::new();
        api_keys.insert(
            "user-key".to_string(),
            Principal {
                id: "u".to_string(),
                role: UserRole::User,
            },
        );
        api_keys.insert(
            "admin-key".to_string(),
            Principal {
                id: "a".to_string(),
                role: UserRole::Admin,
            },
        );
        let auth = crate::agent_auth::AgentAuthConfig {
            enabled: true,
            api_keys,
            token_verifier: None,
        };
        let state = ThreadApiState::new()
            .with_waypoints(Arc::new(MockWaypointStore::default()))
            .with_parley(Arc::new(MockParleyPort {
                outcome: MockOutcome::Accepted,
                parley_id,
            }))
            .with_auth(auth);
        thread_router(state)
    }

    #[tokio::test]
    async fn post_resume_with_non_admin_role_is_403() {
        let parley_id = ParleyId::new();
        let app = authed_thread_app(parley_id);
        let body = serde_json::to_vec(&serde_json::json!({
            "responses": [
                { "parley_id": parley_id.to_string(), "value": true, "responded_by": "alice" }
            ]
        }))
        .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/threads/scoped-thread/resume")
                    .header("x-api-key", "user-key")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn post_resume_with_admin_role_is_202() {
        let parley_id = ParleyId::new();
        let app = authed_thread_app(parley_id);
        let body = serde_json::to_vec(&serde_json::json!({
            "responses": [
                { "parley_id": parley_id.to_string(), "value": true, "responded_by": "alice" }
            ]
        }))
        .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/threads/scoped-thread/resume")
                    .header("x-api-key", "admin-key")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }

    #[tokio::test]
    async fn get_thread_state_with_non_admin_role_is_200_not_403() {
        // Reads stay authenticated-any-role (D-24); only `resume` is
        // narrowed to admin by CR-01.
        let parley_id = ParleyId::new();
        let app = authed_thread_app(parley_id);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/threads/no-such-thread/state")
                    .header("x-api-key", "user-key")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        // `404` (unknown thread), not `403` -- proves the User-role
        // credential cleared authorization and reached the handler.
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn thread_router_returns_501_over_http_when_unwired_and_auth_disabled() {
        let state = ThreadApiState::new(); // auth disabled by default
        let app = thread_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/threads/some-thread/state")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[test]
    fn thread_openapi_router_contains_thread_paths() {
        let state = ThreadApiState::new();
        let (_router, api) = thread_openapi_router(state).split_for_parts();
        for expected in [
            "/threads/{id}/state",
            "/threads/{id}/resume",
            "/threads/{id}/history",
        ] {
            assert!(
                api.paths.paths.contains_key(expected),
                "spec missing path {expected}"
            );
        }
    }

    // A quiet reference to `admin()` so the helper is not flagged unused if
    // future tests need direct authenticated handler calls (mirrors
    // `agent_controller.rs`'s own helper -- kept for parity/discoverability).
    #[test]
    fn admin_principal_has_admin_role() {
        let Extension(p) = admin();
        assert_eq!(p.role, UserRole::Admin);
    }
}
