//! Axum HTTP controller for inspecting, resuming, forking and deleting threads
//! (HITL-05, HITL-FR-16, D-24/D-25/D-27/D-45).
//!
//! This module mirrors [`crate::agent_controller`]'s conventions on a router
//! and state struct of its own, so [`crate::agent_controller::AgentApiState`]
//! stays untouched (D-24, X-10.3):
//!
//! | Method & path | Description |
//! |---------------|-------------|
//! | `GET /threads` | Paginated thread list, newest activity first |
//! | `GET /threads/{id}` | One thread's summary |
//! | `GET /threads/{id}/state` | The thread's latest status, plus outstanding parleys/responses when suspended |
//! | `POST /threads/{id}/resume` | Deliver parley responses; `202 Accepted`, poll `state` for the outcome (Admin) |
//! | `POST /threads/{id}/fork` | Submit a new run continuing the thread from a Waypoint |
//! | `GET /threads/{id}/history` | Paginated Chronicle history (`limit` + opaque `cursor`) |
//! | `DELETE /threads/{id}` | Delete a thread's whole Waypoint history (Admin) |
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
//! ## Authorization (tenant visibility, ADR-0063)
//!
//! A thread carries no tenant of its own; it belongs to the tenant of its **latest run**
//! -- the newest by `submitted_at`, with `run_id` as the tiebreak. Every
//! `/v1/threads/{id}*` route (`get_thread`, `get_thread_state`, `get_thread_history`,
//! `resume_thread`, `fork_thread`, `delete_thread`) enters through one private gate,
//! `load_visible_thread`, which applies
//! [`RunReadScope::permits_thread`](paladin_core::platform::container::principal::RunReadScope::permits_thread)
//! to the principal's own scope (server-derived, never from a header, query parameter or
//! body):
//!
//! - an **Admin** sees every thread and the gate performs no run lookup;
//! - a **user-role principal** sees a thread only when its latest run is attributed to
//!   the principal's tenant;
//! - a thread with **no run** has no owner and is visible to an Admin only;
//! - with **no run store wired** ([`ThreadApiState::runs`] is `None`) ownership cannot be
//!   established, so the gate fails closed: a non-admin sees no thread, an Admin is
//!   unaffected (D-07);
//! - a thread the caller may not see answers the **byte-identical `404 unknown thread
//!   '<id>'`** a missing thread gets -- no `403`, no distinct code, message or header --
//!   so a foreign tenant cannot learn that a thread exists (ADR-0054 rule 5);
//! - on `resume` and `delete` the gate runs **before** `require_admin` (D-06): a foreign
//!   non-admin gets the `404`, a same-tenant non-admin the `403`, an Admin passes both;
//!   `fork` runs the gate before the fork is submitted, and
//!   `RunSubmissionService::ensure_thread_visible` re-checks the same rule for library
//!   callers.
//!
//! Store failures never echo backend text: a waypoint-store failure renders the fixed
//! `500 "thread store error"` and a run-store failure the fixed `500 "run store error"`
//! (detail to the server log only). A Waypoint or run row the store cannot read
//! (`Serialization`, or an unsupported Waypoint schema version) is an existence signal, so
//! for a non-admin it is the same missing-thread `404`; an Admin gets the fixed `500`.
//!
//! The route-enumerating test `every_thread_id_route_hides_foreign_threads_behind_the_missing_thread_404`
//! reads the `/v1/threads/{id}*` operations from this router's own OpenAPI document and
//! fails if any is missing from its table, so a future route cannot skip the gate unnoticed.
//!
//! Known limitation (D-14): thread ids are not tenant-namespaced, so two tenants that pick
//! the same id share one thread, and its latest run decides who sees it. `GET /threads`
//! (the list route) is outside this gate: it addresses no single thread, and its
//! tenant-filtered walk is plan 53-04's.

use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use paladin_core::platform::container::parley::{ParleyKind, ParleyRequest, ParleyResponse};
use paladin_core::platform::container::principal::RunReadScope;
use paladin_core::platform::container::run::Run;
use paladin_core::platform::container::waypoint::{ThreadId, Waypoint, WaypointId, WaypointStatus};
use paladin_ports::input::parley_port::{ParleyError, ParleyPort};
use paladin_ports::input::run_submission_port::{ForkRun, RunSubmissionPort};
use paladin_ports::output::run_repository_port::{RunQuery, RunRepositoryError, RunRepositoryPort};
use paladin_ports::output::waypoint_port::{
    ThreadSummary, WaypointError, WaypointPort, WaypointSummary,
};

use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::agent_auth::{HasAgentAuth, Principal, require_admin};
use crate::agent_controller::{JsonValue, ok_body};
use crate::error::{ApiError, ApiErrorBody};
use crate::pagination::{decode_cursor, encode_cursor, resolve_limit};
use crate::run_controller::{
    RunWebhookRequestDto, map_submission_error, redact_store_error_for_scope,
    run_repo_error_for_scope, to_run_webhook_spec,
};

/// Shared state for the thread routes.
///
/// Mirrors [`crate::agent_controller::AgentApiState`]'s injection-only
/// trait-object shape (D-24): `waypoints`/`parley`/`runs`/`run_submission`
/// are `None` until a durable Waypoint backend (and, for the latter two, a
/// run store/submission service) is configured, at which point
/// `src/bin/paladin-server.rs` wires them all from the SAME stores. Kept as
/// its own struct -- never a field added to `AgentApiState` -- so the
/// pre-existing agent surface is untouched (X-10.3).
///
/// `#[non_exhaustive]` from 27-15 (D-45): `runs`/`run_submission` were added
/// to an already-published (this same v0.10 cycle) type after
/// `GET /threads`, `POST /threads/{id}/fork` and `DELETE /threads/{id}`
/// needed the run-submission/run-repository seam this struct already had no
/// slot for. Construction stays possible only through [`Self::new`] plus
/// the `with_*` builder methods, so the next field stays free without a
/// semver-major bump (X-10.3).
#[derive(Clone)]
#[non_exhaustive]
pub struct ThreadApiState {
    /// Reads the latest Waypoint / paginated history for `GET .../state` and
    /// `GET .../history`. `None` when no waypoint backend is configured.
    pub waypoints: Option<Arc<dyn WaypointPort>>,
    /// Delivers parley responses for `POST .../resume`. `None` when no
    /// waypoint backend is configured (the facade adapter needs one to
    /// resolve a thread's latest Waypoint).
    pub parley: Option<Arc<dyn ParleyPort>>,
    /// Reads a thread's active run id for `GET /threads`/`GET /threads/{id}`
    /// and checks the busy invariant for `DELETE /threads/{id}` (D-45).
    /// `None` when no run store is configured -- `active_run_id` is simply
    /// omitted from responses, and `DELETE` skips the busy check (a
    /// pre-run-server deployment, X-03).
    pub runs: Option<Arc<dyn RunRepositoryPort>>,
    /// Submits a forked run for `POST /threads/{id}/fork` (D-45). `None`
    /// when no run submission backend is configured.
    pub run_submission: Option<Arc<dyn RunSubmissionPort>>,
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
            runs: None,
            run_submission: None,
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

    /// Wire a [`RunRepositoryPort`], enabling `active_run_id` on
    /// `GET /threads`/`GET /threads/{id}` and the busy check on
    /// `DELETE /threads/{id}` (D-45).
    pub fn with_runs(mut self, runs: Arc<dyn RunRepositoryPort>) -> Self {
        self.runs = Some(runs);
        self
    }

    /// Wire a [`RunSubmissionPort`], enabling `POST /threads/{id}/fork`
    /// (D-45).
    pub fn with_run_submission(mut self, run_submission: Arc<dyn RunSubmissionPort>) -> Self {
        self.run_submission = Some(run_submission);
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

/// Message for `POST .../fork` when [`ThreadApiState::run_submission`] is
/// `None`.
const RUN_SUBMISSION_PORT_HINT: &str =
    "no run submission backend configured: set run_store.backend and run_queue.backend";

/// Maximum `limit` accepted by `GET .../history` (D-27, PLAT-FR-16
/// pre-conformance); a larger value is a `400`.
const MAX_HISTORY_LIMIT: u32 = 100;

/// The one fixed client-facing message every waypoint-store failure on the thread routes
/// renders (TENANT-06). The detail -- which can carry connection strings, schema names or
/// the shape of a corrupt Waypoint -- goes to the server log only.
const THREAD_STORE_ERROR_MESSAGE: &str = "thread store error";

/// The most threads one tenant-scoped `GET /threads` request examines (D-04, T-53-21).
///
/// Each examined thread costs one latest-run lookup, so this bounds a request to
/// `MAX_EXAMINED_THREADS` run-repository reads and
/// `MAX_EXAMINED_THREADS / THREAD_SCAN_BATCH` waypoint-store list calls however large the
/// store is or however few of its threads the caller may see. A walk that reaches the cap
/// returns what it has (possibly nothing) with a non-null `next_cursor`.
const MAX_EXAMINED_THREADS: usize = 500;

/// How many thread summaries one tenant-scoped list walk asks the waypoint store for per
/// call (the port's `limit`).
const THREAD_SCAN_BATCH: u32 = 100;

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
///
/// `#[non_exhaustive]` (D-21, PLAT-03): `run_id` was added after this type
/// shipped in Phase 24, mirroring
/// [`ResumeAccepted`](paladin_ports::input::parley_port::ResumeAccepted)'s
/// own field of the same name and meaning one level down the stack.
/// Construction stays possible only through [`Self::new`] (X-10.3), so the
/// next field stays free without a semver-major bump.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[non_exhaustive]
pub struct ResumeAcceptedResponse {
    /// The thread whose resume was accepted.
    pub thread_id: String,
    /// The URL a client polls (`GET .../state`) to observe the outcome.
    pub state_url: String,
    /// The run this resume re-enqueued, if the durable run-server path
    /// handled it (`None` for Phase 24's in-process-spawn fallback, X-03).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
}

impl ResumeAcceptedResponse {
    /// Construct a response body (D-21, PLAT-03) -- the only construction
    /// path now that this struct is `#[non_exhaustive]`.
    pub fn new(thread_id: String, state_url: String, run_id: Option<String>) -> Self {
        Self {
            thread_id,
            state_url,
            run_id,
        }
    }
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
    /// Maximum number of items to return (capped by the crate-private history limit).
    #[serde(default)]
    pub limit: Option<u32>,
    /// Opaque pagination cursor from a previous page's `next_cursor`.
    #[serde(default)]
    pub cursor: Option<String>,
}

// --- Thread list/get/fork/delete DTOs (D-45) --------------------------------

/// Wire projection of a [`paladin_ports::output::waypoint_port::ThreadSummary`]
/// for `GET /threads`.
///
/// **Known limitation:** `ThreadSummary` (the port this list is built from)
/// carries no `waypoint_id` field -- only [`ThreadResponse`] (`GET
/// /threads/{id}`, built from a full [`Waypoint`]) can report one. Adding a
/// `waypoint_id` to `ThreadSummary`/`WaypointPort::list_threads` would touch
/// every backend adapter in `paladin-storage`, outside this plan's declared
/// file scope -- documented here rather than silently omitted without
/// explanation.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ThreadSummaryDto {
    /// The thread's identity.
    pub thread_id: String,
    /// Stable, lowercase status label of the thread's latest waypoint.
    pub latest_status: String,
    /// When the thread's latest waypoint was created.
    pub updated_at: DateTime<Utc>,
    /// The thread's currently active run id, if [`ThreadApiState::runs`] is
    /// wired and one exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_run_id: Option<String>,
}

/// Response body for `GET /threads`.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ThreadListResponse {
    /// The page of thread summaries, newest-activity-first.
    pub items: Vec<ThreadSummaryDto>,
    /// Opaque cursor for the next page, `None` on the last page.
    pub next_cursor: Option<String>,
}

/// Query parameters for `GET /threads`.
#[derive(Debug, Clone, Deserialize)]
pub struct ThreadListQuery {
    /// Maximum number of items to return (`1..=100`; omitted defaults to 20).
    #[serde(default)]
    pub limit: Option<u32>,
    /// Opaque pagination cursor from a previous page's `next_cursor`.
    #[serde(default)]
    pub cursor: Option<String>,
}

/// Response body for `GET /threads/{id}`.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ThreadResponse {
    /// The thread's identity.
    pub thread_id: String,
    /// Stable, lowercase status label of the thread's latest waypoint.
    pub latest_status: String,
    /// The thread's latest waypoint id.
    pub latest_waypoint_id: String,
    /// When the latest waypoint was created.
    pub updated_at: DateTime<Utc>,
    /// The thread's currently active run id, if [`ThreadApiState::runs`] is
    /// wired and one exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_run_id: Option<String>,
}

/// Request body for `POST /threads/{id}/fork`.
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct ForkThreadRequest {
    /// The Waypoint to fork from.
    pub from_waypoint_id: String,
    /// An optional state edit merged at the fork point.
    #[serde(default)]
    #[schema(value_type = Object)]
    pub edit: Option<serde_json::Value>,
    /// An optional webhook delivery target for the forked run's lifecycle
    /// events.
    #[serde(default)]
    pub webhook: Option<RunWebhookRequestDto>,
}

/// Response body for a successful `POST /threads/{id}/fork` (`202 Accepted`).
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ForkThreadResponse {
    /// The newly created forked run's identity.
    pub run_id: String,
    /// The thread the fork continues on.
    pub thread_id: String,
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
        other => thread_store_error("thread API resume", other),
    }
}

// --- Visibility gate (TENANT-03, ADR-0054 as amended by ADR-0063) ------------

/// Log a waypoint-store failure server-side and return the fixed `500`
/// `"thread store error"` for every role.
///
/// For a failure that is *not* an existence question: the caller has already been
/// admitted by [`load_visible_thread`], so there is nothing left to hide, and the detail
/// is for the operator's log only. Routed through the shared run-route redaction rule
/// with an `All` scope and `hideable = false`, so one rule renders every store error.
fn thread_store_error(context: &str, err: impl std::fmt::Display) -> ApiError {
    redact_store_error_for_scope(
        &RunReadScope::All,
        context,
        &err,
        false,
        THREAD_STORE_ERROR_MESSAGE,
        || ApiError::internal(THREAD_STORE_ERROR_MESSAGE),
    )
}

/// Classify a [`WaypointError`] raised before the caller's ownership is known.
///
/// A corrupt or newer-schema Waypoint stored under an id is itself an existence signal,
/// so `Serialization` and `SchemaVersionUnsupported` are hideable: a `Tenant` scope gets
/// `hidden()` (the missing-thread `404`), exactly as for an unreadable run row (D-09).
/// `Backend` is an outage and `NotFound` a state error; neither is hideable, so both
/// render the fixed `thread store error` `500`. An `All` (Admin) scope always gets the
/// fixed `500` so an operator learns the row is unreadable.
fn waypoint_error_for_scope(
    scope: &RunReadScope,
    context: &str,
    err: WaypointError,
    hidden: impl FnOnce() -> ApiError,
) -> ApiError {
    let hideable = matches!(
        err,
        WaypointError::Serialization(_) | WaypointError::SchemaVersionUnsupported { .. }
    );
    redact_store_error_for_scope(
        scope,
        context,
        &err,
        hideable,
        THREAD_STORE_ERROR_MESSAGE,
        hidden,
    )
}

/// The newest run on `thread` by `submitted_at` (`run_id` as the tiebreak), or `None`
/// when the thread has no run row.
///
/// An internal authorization read: the query keeps the default `All` scope so the owner
/// is found regardless of who is asking -- the caller's scope is applied afterwards by
/// [`RunReadScope::permits_thread`], never pushed into this query.
async fn latest_run_for_thread(
    runs: &Arc<dyn RunRepositoryPort>,
    thread: &ThreadId,
) -> Result<Option<Run>, RunRepositoryError> {
    let page = runs
        .list(RunQuery {
            thread_id: Some(thread.clone()),
            limit: 1,
            ..Default::default()
        })
        .await?;
    Ok(page.items.into_iter().next())
}

/// Collect up to `limit` threads `scope` may see, walking the waypoint store newest
/// activity first (TENANT-03, D-04) -- the `GET /threads` counterpart of
/// [`load_visible_thread`].
///
/// A thread's owner is its latest run, so each examined thread costs one
/// [`latest_run_for_thread`] lookup (the same read the single-thread gate performs, with
/// the `All` scope) and is kept when [`RunReadScope::permits_thread`] admits it. The walk
/// pages the store in [`THREAD_SCAN_BATCH`]-sized `list_threads` calls and stops as soon
/// as `limit` threads are kept, the store is exhausted, or [`MAX_EXAMINED_THREADS`]
/// threads have been examined.
///
/// Returns the kept threads (newest first) and the cursor to continue from: the
/// `last_updated_at` of the **last examined** thread, or `None` when the store is
/// exhausted. A page may therefore hold fewer than `limit` items while the cursor still
/// advances (a client follows `next_cursor` until it is `null`), and no thread repeats
/// across pages. The cursor is an exclusive timestamp, so, as `WaypointPort::list_threads`
/// itself documents, two threads with an identical `last_updated_at` straddling a page
/// boundary can be skipped; this is a keyset walk, not a snapshot.
///
/// A lookup that fails with [`RunRepositoryError::Serialization`] means that one thread's
/// run row is unreadable. For a `Tenant` scope the thread is skipped (it is hidden, never
/// shown) and the error logged, so one tenant's corrupt row cannot fail another tenant's
/// list. Any other run-repository error, and every waypoint-store error, answers the fixed
/// `500` for every role -- a list has no single object to hide (D-09).
async fn visible_thread_page(
    waypoints: &Arc<dyn WaypointPort>,
    runs: &Arc<dyn RunRepositoryPort>,
    scope: &RunReadScope,
    limit: u32,
    before: Option<DateTime<Utc>>,
) -> Result<(Vec<ThreadSummary>, Option<DateTime<Utc>>), ApiError> {
    let limit = limit as usize;
    let mut visible: Vec<ThreadSummary> = Vec::new();
    let mut examined = 0_usize;
    let mut cursor = before;
    loop {
        let batch = waypoints
            .list_threads(Some(THREAD_SCAN_BATCH), cursor)
            .await
            .map_err(|e| thread_store_error("thread API list", e))?;
        let batch_len = batch.len();
        if batch_len == 0 {
            return Ok((visible, None));
        }
        for summary in batch {
            examined += 1;
            cursor = Some(summary.last_updated_at);
            match latest_run_for_thread(runs, &summary.thread_id).await {
                Ok(latest) => {
                    if scope.permits_thread(latest.as_ref()) {
                        visible.push(summary);
                    }
                }
                Err(RunRepositoryError::Serialization { message })
                    if !matches!(scope, RunReadScope::All) =>
                {
                    // Hidden, not shown: an unreadable row has no attributable owner.
                    log::error!(
                        "thread API list owner lookup failed: run serialization error: \
                         {message} (thread skipped as not visible)"
                    );
                }
                Err(other) => {
                    // The `All` scope never takes the hidden branch, so this is the fixed
                    // `500`; the fallback closure is unreachable by construction.
                    return Err(run_repo_error_for_scope(
                        &RunReadScope::All,
                        "thread API list owner lookup",
                        other,
                        || ApiError::internal(THREAD_STORE_ERROR_MESSAGE),
                    ));
                }
            }
            if visible.len() >= limit || examined >= MAX_EXAMINED_THREADS {
                return Ok((visible, cursor));
            }
        }
        if batch_len < THREAD_SCAN_BATCH as usize {
            return Ok((visible, None));
        }
    }
}

/// Load a thread's latest Waypoint and check the caller may see the thread -- the single
/// entry every `/v1/threads/{id}*` route passes through (TENANT-03, D-05).
///
/// A thread carries no tenant of its own: it belongs to the tenant of its **latest run**
/// (newest by `submitted_at`, `run_id` as the tiebreak), applied through
/// [`RunReadScope::permits_thread`]. In order:
///
/// 1. no waypoint backend -> `501` (same as every thread route);
/// 2. the latest Waypoint is loaded; a missing thread is `hidden()`;
/// 3. an `All` (Admin) scope returns here, performing **no** run lookup, so a run-store
///    outage cannot turn an Admin read into a failure;
/// 4. no run store wired -> `hidden()` for every non-admin (D-07, fail closed: ownership
///    cannot be established, so nothing is shown);
/// 5. the latest run is read and `permits_thread` applied; a thread with no run is
///    `hidden()` (Admin-only), as is a thread whose latest run is another tenant's.
///
/// `hidden()` builds the exact `404 unknown thread '<id>'` a genuinely missing thread
/// gets, so a hidden thread is indistinguishable from a missing one by status, code,
/// headers and body (ADR-0054 rule 5, ADR-0063). Store failures go through the shared
/// role-aware redaction: an unreadable Waypoint or run row is the same `404` for a
/// `Tenant` scope and a fixed `500` for an Admin.
///
/// Known limitation (D-14): thread ids are not tenant-namespaced, so two tenants that
/// choose the same id share one thread; the latest run decides who sees it.
async fn load_visible_thread(
    state: &ThreadApiState,
    principal: &Principal,
    thread: &ThreadId,
) -> Result<Waypoint, ApiError> {
    let waypoints = state
        .waypoints
        .as_ref()
        .ok_or_else(|| ApiError::not_implemented(WAYPOINT_BACKEND_HINT))?;
    let scope = principal.read_scope();
    let hidden = || ApiError::not_found(format!("unknown thread '{thread}'"));

    let waypoint = match waypoints.latest(thread).await {
        Ok(Some(waypoint)) => waypoint,
        Ok(None) => return Err(hidden()),
        Err(e) => {
            return Err(waypoint_error_for_scope(
                &scope,
                "thread API thread lookup",
                e,
                hidden,
            ));
        }
    };

    if matches!(scope, RunReadScope::All) {
        return Ok(waypoint);
    }
    let Some(runs) = state.runs.as_ref() else {
        return Err(hidden());
    };
    let latest = match latest_run_for_thread(runs, thread).await {
        Ok(latest) => latest,
        Err(e) => {
            return Err(run_repo_error_for_scope(
                &scope,
                "thread API thread owner lookup",
                e,
                hidden,
            ));
        }
    };
    if !scope.permits_thread(latest.as_ref()) {
        return Err(hidden());
    }
    Ok(waypoint)
}

// --- Handlers ---------------------------------------------------------------

/// `GET /threads/{id}/state` -- the thread's latest status, plus outstanding
/// parleys/responses when suspended.
///
/// Authenticated, any role, tenant-scoped through [`load_visible_thread`]: a user-role
/// principal sees only a thread whose latest run belongs to its tenant, an Admin sees
/// every thread, and a thread the caller may not see answers the same `404` as a missing
/// one (ADR-0063).
///
/// Returns:
/// - `200 OK` with [`ThreadStateResponse`] on success;
/// - `400 Bad Request` for an invalid thread id;
/// - `404 Not Found` for an unknown thread, or one the caller may not see;
/// - `501 Not Implemented` if no waypoint backend is configured.
#[utoipa::path(
    get,
    path = "/threads/{id}/state",
    tag = "threads",
    params(("id" = String, Path, description = "Thread id")),
    responses(
        (status = 200, description = "Thread state. Tenant-scoped: a user-role principal can read only a thread whose latest run belongs to its tenant; an admin-role principal can read every thread.", body = ThreadStateResponse),
        (status = 400, description = "Invalid thread id", body = ApiErrorBody),
        (status = 401, description = "Missing/invalid credentials", body = ApiErrorBody),
        (status = 404, description = "Unknown thread (also returned for a thread the caller may not see)", body = ApiErrorBody),
        (status = 501, description = "No waypoint backend configured", body = ApiErrorBody),
    ),
    security(("api_key" = []), ("bearer_token" = [])),
)]
pub async fn get_thread_state(
    State(state): State<ThreadApiState>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
) -> Result<(StatusCode, JsonValue), ApiError> {
    if state.waypoints.is_none() {
        return Err(ApiError::not_implemented(WAYPOINT_BACKEND_HINT));
    }
    let thread = parse_thread_id(&id)?;
    let waypoint = load_visible_thread(&state, &principal, &thread).await?;

    Ok((
        StatusCode::OK,
        ok_body(&ThreadStateResponse::from(&waypoint)),
    ))
}

/// `POST /threads/{id}/resume` -- deliver parley responses.
///
/// Requires an admin principal ([`require_admin`], CR-01) **after** the thread
/// visibility gate ([`load_visible_thread`], D-06): a non-admin of another tenant gets
/// the missing-thread `404`, never the `403` that would reveal the thread exists; a
/// same-tenant non-admin gets the `403`; an Admin passes both. See the module-level
/// "Authorization (tenant visibility, ADR-0063)" docs.
///
/// Validates synchronously through [`ParleyPort::resume_with`] and returns
/// `202 Accepted` immediately; the run itself continues on a background task
/// (D-25) -- a client polls `GET .../state` to observe the outcome. Maps
/// [`ParleyError`] per the crate-private `map_parley_error` mapping.
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
        (status = 404, description = "Unknown thread (also returned for a thread the caller may not see)", body = ApiErrorBody),
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
    let parley = state
        .parley
        .as_ref()
        .ok_or_else(|| ApiError::not_implemented(PARLEY_PORT_HINT))?;
    let thread = parse_thread_id(&id)?;
    // The gate runs BEFORE `require_admin` (D-06): a foreign non-admin must see the
    // missing-thread 404, not a 403 that confirms the thread exists.
    load_visible_thread(&state, &principal, &thread).await?;
    require_admin(&principal)?;

    let responses = body
        .responses
        .into_iter()
        .map(|input| {
            Ok(ParleyResponse {
                parley_id: parse_parley_id(&input.parley_id)?,
                // Placeholders: `WarEngine::resume_with` stamps both from the
                // matching request regardless of what is supplied here (D-07).
                kind: ParleyKind::FreeText,
                prompt: String::new(),
                value: input.value,
                responded_by: input.responded_by,
                responded_at: Utc::now(),
                defaulted: false,
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;

    let accepted = parley
        .resume_with(&thread, responses)
        .await
        .map_err(map_parley_error)?;

    let thread_id = accepted.thread_id().as_str().to_string();
    let run_id = accepted.run_id().map(|id| id.to_string());
    let state_url = format!(
        "{}/threads/{thread_id}/state",
        crate::agent_controller::API_V1_PREFIX
    );
    Ok((
        StatusCode::ACCEPTED,
        ok_body(&ResumeAcceptedResponse::new(thread_id, state_url, run_id)),
    ))
}

/// `GET /threads/{id}/history` -- paginated Chronicle history.
///
/// Authenticated, any role, tenant-scoped through [`load_visible_thread`] exactly like
/// [`get_thread_state`]. An unknown thread is `404` for every role (it used to answer
/// `200` with an empty page, which let a caller probe for a thread's existence).
///
/// `limit` (capped by the crate-private history limit) and an opaque `cursor` whose
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
        (status = 200, description = "A page of waypoint summaries. Tenant-scoped: a user-role principal can read only a thread whose latest run belongs to its tenant; an admin-role principal can read every thread.", body = HistoryResponse),
        (status = 400, description = "limit exceeds 100, or cursor is not a valid waypoint id", body = ApiErrorBody),
        (status = 401, description = "Missing/invalid credentials", body = ApiErrorBody),
        (status = 404, description = "Unknown thread (also returned for a thread the caller may not see)", body = ApiErrorBody),
        (status = 501, description = "No waypoint backend configured", body = ApiErrorBody),
    ),
    security(("api_key" = []), ("bearer_token" = [])),
)]
pub async fn get_thread_history(
    State(state): State<ThreadApiState>,
    Extension(principal): Extension<Principal>,
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

    // `limit`/`cursor` validation above says nothing about the thread, so it may precede
    // the gate; everything that touches the thread's data follows it.
    load_visible_thread(&state, &principal, &thread).await?;

    let page = waypoints
        .history(&thread, params.limit, before)
        .await
        .map_err(|e| thread_store_error("thread API history", e))?;

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

/// Look up `thread`'s active run id through [`ThreadApiState::runs`], if
/// wired. A backend error is swallowed to `None` rather than failing the
/// whole list/get response -- `active_run_id` is a convenience projection,
/// never the source of truth `GET /runs/{run_id}` itself is.
async fn active_run_id_for(state: &ThreadApiState, thread: &ThreadId) -> Option<String> {
    let runs = state.runs.as_ref()?;
    runs.active_run_for_thread(thread)
        .await
        .ok()
        .flatten()
        .map(|run| run.run_id.to_string())
}

/// Build the list DTO for one thread summary, with its active run id when a run store
/// is wired.
async fn thread_summary_dto(state: &ThreadApiState, summary: &ThreadSummary) -> ThreadSummaryDto {
    ThreadSummaryDto {
        thread_id: summary.thread_id.as_str().to_string(),
        latest_status: waypoint_status_label(&summary.latest_status).to_string(),
        updated_at: summary.last_updated_at,
        active_run_id: active_run_id_for(state, &summary.thread_id).await,
    }
}

/// `GET /threads` -- paginated thread list, newest-activity-first (D-45,
/// D-47). Authenticated, any role.
///
/// A cursor walk gives a stable ordering for threads that already existed
/// when the first page was fetched, but is NOT a point-in-time snapshot: a
/// thread whose latest waypoint was written after the first page was read
/// may be omitted from the walk (D-47 backstop).
///
/// Returns:
/// - `200 OK` with [`ThreadListResponse`] on success; `{ items: [],
///   next_cursor: null }` when empty;
/// - `400 Bad Request` for `limit == 0`/`limit > 100`, or an unparseable
///   `cursor`;
/// - `501 Not Implemented` if no waypoint backend is configured.
#[utoipa::path(
    get,
    path = "/threads",
    tag = "threads",
    params(
        ("limit" = Option<u32>, Query, description = "Max items to return, 1..=100 (default 20)"),
        ("cursor" = Option<String>, Query, description = "Opaque pagination cursor from a previous page's next_cursor -- a keyset walk, not a snapshot: threads updated after the first page was fetched may be omitted"),
    ),
    responses(
        (status = 200, description = "A page of threads; { items: [], next_cursor: null } when empty", body = ThreadListResponse),
        (status = 400, description = "Invalid limit or cursor", body = ApiErrorBody),
        (status = 401, description = "Missing/invalid credentials", body = ApiErrorBody),
        (status = 501, description = "No waypoint backend configured", body = ApiErrorBody),
    ),
    security(("api_key" = []), ("bearer_token" = [])),
)]
pub async fn list_threads(
    State(state): State<ThreadApiState>,
    Extension(principal): Extension<Principal>,
    Query(params): Query<ThreadListQuery>,
) -> Result<(StatusCode, JsonValue), ApiError> {
    let waypoints = state
        .waypoints
        .as_ref()
        .ok_or_else(|| ApiError::not_implemented(WAYPOINT_BACKEND_HINT))?;
    let limit = resolve_limit(params.limit)?;
    let before = params
        .cursor
        .as_deref()
        .map(decode_cursor::<DateTime<Utc>>)
        .transpose()?;

    // A non-admin sees only threads whose latest run is its tenant's (TENANT-03, D-04).
    let scope = principal.read_scope();
    if !matches!(scope, RunReadScope::All) {
        // No run store: ownership cannot be established, so nothing is visible (D-07).
        let Some(runs) = state.runs.as_ref() else {
            return Ok((
                StatusCode::OK,
                ok_body(&ThreadListResponse {
                    items: Vec::new(),
                    next_cursor: None,
                }),
            ));
        };
        let (visible, cursor) = visible_thread_page(waypoints, runs, &scope, limit, before).await?;
        let mut items = Vec::with_capacity(visible.len());
        for summary in &visible {
            items.push(thread_summary_dto(&state, summary).await);
        }
        let next_cursor = cursor.as_ref().map(encode_cursor);
        return Ok((
            StatusCode::OK,
            ok_body(&ThreadListResponse { items, next_cursor }),
        ));
    }

    // Admin: limit+1 trick -- fetch one extra to know whether a next page exists
    // without a second round trip, and perform no owner lookup.
    let page = waypoints
        .list_threads(Some(limit + 1), before)
        .await
        .map_err(|e| thread_store_error("thread API list", e))?;
    let has_more = page.len() as u32 > limit;
    let page_items = if has_more {
        &page[..limit as usize]
    } else {
        &page[..]
    };

    let mut items = Vec::with_capacity(page_items.len());
    for summary in page_items {
        items.push(thread_summary_dto(&state, summary).await);
    }
    let next_cursor = if has_more {
        page_items.last().map(|s| encode_cursor(&s.last_updated_at))
    } else {
        None
    };

    Ok((
        StatusCode::OK,
        ok_body(&ThreadListResponse { items, next_cursor }),
    ))
}

/// `GET /threads/{id}` -- one thread's summary (D-45). Authenticated, any
/// role, tenant-scoped through [`load_visible_thread`] (ADR-0063).
///
/// Returns:
/// - `200 OK` with [`ThreadResponse`] on success;
/// - `400 Bad Request` for an invalid thread id;
/// - `404 Not Found` if no Waypoint exists for `id`, or the caller may not see the thread;
/// - `501 Not Implemented` if no waypoint backend is configured.
#[utoipa::path(
    get,
    path = "/threads/{id}",
    tag = "threads",
    params(("id" = String, Path, description = "Thread id")),
    responses(
        (status = 200, description = "Thread summary. Tenant-scoped: a user-role principal can read only a thread whose latest run belongs to its tenant; an admin-role principal can read every thread.", body = ThreadResponse),
        (status = 400, description = "Invalid thread id", body = ApiErrorBody),
        (status = 401, description = "Missing/invalid credentials", body = ApiErrorBody),
        (status = 404, description = "Unknown thread (also returned for a thread the caller may not see)", body = ApiErrorBody),
        (status = 501, description = "No waypoint backend configured", body = ApiErrorBody),
    ),
    security(("api_key" = []), ("bearer_token" = [])),
)]
pub async fn get_thread(
    State(state): State<ThreadApiState>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
) -> Result<(StatusCode, JsonValue), ApiError> {
    if state.waypoints.is_none() {
        return Err(ApiError::not_implemented(WAYPOINT_BACKEND_HINT));
    }
    let thread = parse_thread_id(&id)?;
    let wp = load_visible_thread(&state, &principal, &thread).await?;

    let active_run_id = active_run_id_for(&state, &thread).await;

    Ok((
        StatusCode::OK,
        ok_body(&ThreadResponse {
            thread_id: thread.as_str().to_string(),
            latest_status: waypoint_status_label(&wp.status).to_string(),
            latest_waypoint_id: wp.waypoint_id.to_string(),
            updated_at: wp.created_at,
            active_run_id,
        }),
    ))
}

/// `POST /threads/{id}/fork` -- submit a NEW run that continues the thread
/// from a specific Waypoint, optionally editing state at the fork point
/// (D-45, D-46).
///
/// Invocation-shaped: any authenticated principal, subject to the target
/// assistant's own `allowed_roles` (checked inside
/// [`RunSubmissionPort::fork`] -- `paladin-web` has no visibility into
/// `allowed_roles` itself, ADR-0031).
///
/// The caller must be able to see the thread ([`load_visible_thread`]): a thread another
/// tenant owns answers the missing-thread `404` before the fork is submitted.
/// `RunSubmissionService::ensure_thread_visible` re-checks the same rule for library
/// callers that bypass this controller.
///
/// Returns:
/// - `202 Accepted` with [`ForkThreadResponse`] on success;
/// - `400 Bad Request` for an invalid thread id or `webhook_url_rejected`
///   (write-time SSRF guard, D-42);
/// - `403 Forbidden` if the principal's role is not permitted;
/// - `404 Not Found` for an unknown thread (or one the caller may not see) or an unknown
///   `from_waypoint_id`;
/// - `409 Conflict` (`thread_busy`) while a run is already active on the
///   thread;
/// - `501 Not Implemented` if no run submission backend is configured.
#[utoipa::path(
    post,
    path = "/threads/{id}/fork",
    tag = "threads",
    params(("id" = String, Path, description = "Thread id")),
    request_body = ForkThreadRequest,
    responses(
        (status = 202, description = "Fork accepted; a new run continues the thread", body = ForkThreadResponse),
        (status = 400, description = "Invalid thread id, or webhook_url_rejected", body = ApiErrorBody),
        (status = 401, description = "Missing/invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Role not permitted for this assistant", body = ApiErrorBody),
        (status = 404, description = "Unknown thread or waypoint (a thread the caller may not see is also reported as unknown)", body = ApiErrorBody),
        (status = 409, description = "Thread busy -- a run is already active on this thread", body = ApiErrorBody),
        (status = 429, description = "Allowance exhausted (code allowance_exhausted): the caller's tenant or API-key allowance is spent; Retry-After carries the seconds until the window resets and is omitted for a lifetime cap", body = ApiErrorBody),
        (status = 500, description = "The allowance check itself failed; the request is refused and nothing runs (fail closed)", body = ApiErrorBody),
        (status = 501, description = "No run submission backend configured", body = ApiErrorBody),
    ),
    security(("api_key" = []), ("bearer_token" = [])),
)]
pub async fn fork_thread(
    State(state): State<ThreadApiState>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    Json(body): Json<ForkThreadRequest>,
) -> Result<(StatusCode, JsonValue), ApiError> {
    let run_submission = state
        .run_submission
        .as_ref()
        .ok_or_else(|| ApiError::not_implemented(RUN_SUBMISSION_PORT_HINT))?;
    let thread = parse_thread_id(&id)?;
    // The gate precedes every body-dependent check, so a foreign caller learns nothing
    // from a 400 about a thread it cannot see.
    load_visible_thread(&state, &principal, &thread).await?;
    let from_waypoint_id = parse_waypoint_id(&body.from_waypoint_id)?;
    let webhook = body.webhook.map(to_run_webhook_spec).transpose()?;

    let accepted = run_submission
        .fork(ForkRun {
            thread_id: thread,
            from_waypoint_id,
            edit: body.edit,
            webhook,
            requested_by: Some(
                paladin_core::platform::container::principal::PrincipalRef::from(&principal),
            ),
        })
        .await
        .map_err(map_submission_error)?;

    Ok((
        StatusCode::ACCEPTED,
        ok_body(&ForkThreadResponse {
            run_id: accepted.run_id.to_string(),
            thread_id: accepted.thread_id.as_str().to_string(),
        }),
    ))
}

/// `DELETE /threads/{id}` -- delete a thread's whole Waypoint history
/// (admin, D-45).
///
/// The visibility gate ([`load_visible_thread`]) runs **before** the admin check
/// (D-06): a non-admin of another tenant gets the missing-thread `404`, a same-tenant
/// non-admin the `403`. A second `DELETE` of a deleted thread is `404` for every role.
///
/// Returns:
/// - `204 No Content` on success;
/// - `400 Bad Request` for an invalid thread id;
/// - `403 Forbidden` for a non-admin principal who can see the thread;
/// - `404 Not Found` if nothing was deleted, or the thread is not visible to the caller;
/// - `409 Conflict` (`thread_busy`) while a run is active on the thread;
/// - `501 Not Implemented` if no waypoint backend is configured.
#[utoipa::path(
    delete,
    path = "/threads/{id}",
    tag = "threads",
    params(("id" = String, Path, description = "Thread id")),
    responses(
        (status = 204, description = "Thread deleted"),
        (status = 400, description = "Invalid thread id", body = ApiErrorBody),
        (status = 401, description = "Missing/invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Admin role required", body = ApiErrorBody),
        (status = 404, description = "Unknown thread (also returned for a thread the caller may not see)", body = ApiErrorBody),
        (status = 409, description = "Thread busy -- a run is active on this thread", body = ApiErrorBody),
        (status = 501, description = "No waypoint backend configured", body = ApiErrorBody),
    ),
    security(("api_key" = []), ("bearer_token" = [])),
)]
pub async fn delete_thread(
    State(state): State<ThreadApiState>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let waypoints = state
        .waypoints
        .as_ref()
        .ok_or_else(|| ApiError::not_implemented(WAYPOINT_BACKEND_HINT))?;
    let thread = parse_thread_id(&id)?;
    // Gate first, admin check second (D-06): 404 for a hidden thread, 403 only for a
    // caller who may see it.
    load_visible_thread(&state, &principal, &thread).await?;
    require_admin(&principal)?;

    if let Some(runs) = &state.runs
        && runs
            .active_run_for_thread(&thread)
            .await
            .map_err(|e| {
                run_repo_error_for_scope(
                    &principal.read_scope(),
                    "thread API busy check",
                    e,
                    || ApiError::not_found(format!("unknown thread '{id}'")),
                )
            })?
            .is_some()
    {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "thread_busy",
            format!("thread '{id}' has an active run"),
        ));
    }

    let deleted = waypoints
        .delete_thread(&thread)
        .await
        .map_err(|e| thread_store_error("thread API delete", e))?;
    if deleted == 0 {
        return Err(ApiError::not_found(format!("unknown thread '{id}'")));
    }

    Ok(StatusCode::NO_CONTENT)
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
        .routes(routes!(list_threads))
        .routes(routes!(get_thread))
        .routes(routes!(delete_thread))
        .routes(routes!(fork_thread))
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
    use paladin_core::platform::container::run::{Run, RunId, RunStatus};
    use paladin_core::platform::container::user::UserRole;
    use paladin_core::platform::container::waypoint::{FrontierSnapshot, GraphFingerprint, NodeId};
    use paladin_ports::input::parley_port::ResumeAccepted;
    use paladin_ports::input::run_submission_port::{RunSubmissionError, SubmitRun};
    use paladin_ports::output::run_repository_port::RunRepositoryPort;
    use std::collections::BTreeMap;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use tower::ServiceExt; // for `Router::oneshot`

    use crate::agent_auth::Principal;

    /// An admin `Principal` extension for direct handler calls (bypasses
    /// HTTP auth plumbing), mirroring `agent_controller`'s own `admin()`.
    fn admin() -> Extension<Principal> {
        Extension(Principal::new(
            "test-admin",
            UserRole::Admin,
            paladin_core::platform::container::principal::TenantId::new("test-tenant").unwrap(),
        ))
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

    /// The `latest()` failure a [`MockWaypointStore`] can be told to return (a
    /// `WaypointError` is not `Clone`, so the seam stores which variant to build).
    #[derive(Clone, Copy)]
    enum WaypointFailure {
        /// `WaypointError::Serialization` carrying text that must never reach a body.
        Serialization,
        /// `WaypointError::SchemaVersionUnsupported`.
        SchemaVersion,
        /// `WaypointError::Backend`.
        Backend,
    }

    /// The text every injected failure carries; no response body may contain it.
    const INJECTED_TEXT: &str = "db.internal.example corrupt-row-detail";

    /// One recorded `list_threads(limit, before)` call.
    type ListCall = (Option<u32>, Option<DateTime<Utc>>);

    #[derive(Default)]
    struct MockWaypointStore {
        latest: Mutex<HashMap<String, Waypoint>>,
        history: Mutex<HashMap<String, Vec<WaypointSummary>>>,
        /// When set, `latest()` fails with this variant (Phase 53 thread gate tests).
        fail_latest_with: Mutex<Option<WaypointFailure>>,
        /// Seeded `list_threads` rows (Phase 53 list walk tests); empty by default.
        threads: Mutex<Vec<ThreadSummary>>,
        /// Every `list_threads(limit, before)` call, in order.
        list_calls: Mutex<Vec<ListCall>>,
    }

    impl MockWaypointStore {
        fn fail_latest(&self, failure: WaypointFailure) {
            *self.fail_latest_with.lock().unwrap() = Some(failure);
        }

        fn seed_latest(&self, wp: Waypoint) {
            self.latest
                .lock()
                .unwrap()
                .insert(wp.thread_id.as_str().to_string(), wp);
        }

        /// Seed one `list_threads` row.
        fn seed_listed(&self, summary: ThreadSummary) {
            self.threads.lock().unwrap().push(summary);
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
            if let Some(failure) = *self.fail_latest_with.lock().unwrap() {
                return Err(match failure {
                    WaypointFailure::Serialization => {
                        WaypointError::Serialization(INJECTED_TEXT.to_string())
                    }
                    WaypointFailure::SchemaVersion => WaypointError::SchemaVersionUnsupported {
                        found: INJECTED_TEXT.to_string(),
                        supported: "v1".to_string(),
                    },
                    WaypointFailure::Backend => WaypointError::Backend {
                        source: INJECTED_TEXT.into(),
                    },
                });
            }
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

        /// Honors the port contract over the seeded rows: newest `last_updated_at` first,
        /// `before` exclusive, `limit` applied last. Empty when nothing is seeded.
        async fn list_threads(
            &self,
            limit: Option<u32>,
            before: Option<DateTime<Utc>>,
        ) -> Result<Vec<ThreadSummary>, WaypointError> {
            self.list_calls.lock().unwrap().push((limit, before));
            let mut rows: Vec<ThreadSummary> = self
                .threads
                .lock()
                .unwrap()
                .iter()
                .filter(|row| before.is_none_or(|b| row.last_updated_at < b))
                .cloned()
                .collect();
            rows.sort_by_key(|row| std::cmp::Reverse(row.last_updated_at));
            if let Some(limit) = limit {
                rows.truncate(limit as usize);
            }
            Ok(rows)
        }

        async fn delete_thread(&self, thread: &ThreadId) -> Result<u64, WaypointError> {
            let removed_latest = self
                .latest
                .lock()
                .unwrap()
                .remove(thread.as_str())
                .is_some();
            let removed_history = self
                .history
                .lock()
                .unwrap()
                .remove(thread.as_str())
                .is_some();
            Ok(u64::from(removed_latest || removed_history))
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

    fn state_with_parley(
        outcome: MockOutcome,
        parley_id: ParleyId,
        t: &ThreadId,
    ) -> ThreadApiState {
        // A resume call also needs a waypoint backend wired (D-24: `parley`
        // and `waypoints` come from the SAME store in production; the mocks
        // are independent here since only `parley` is exercised). The thread under
        // test has a seeded latest Waypoint so the visibility gate (TENANT-03) admits
        // the Admin principals these tests call with.
        let store = Arc::new(MockWaypointStore::default());
        store.seed_latest(sample_waypoint(t, 1, WaypointStatus::Completed));
        ThreadApiState::new()
            .with_waypoints(store)
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
        let state = state_with_parley(MockOutcome::Accepted, parley_id, &t);

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
        let state = state_with_parley(MockOutcome::ThreadNotFound, parley_id, &t);

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
        let state = state_with_parley(MockOutcome::ThreadNotAwaitingInput, parley_id, &t);

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
        let state = state_with_parley(MockOutcome::GraphNotRegistered, parley_id, &t);

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
            let state = state_with_parley(outcome, parley_id, &t);
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
        store.seed_latest(sample_waypoint(&t, 5, WaypointStatus::Completed));
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
            bearer_tenant: None,
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
        let tenant =
            || paladin_core::platform::container::principal::TenantId::new("test-tenant").unwrap();
        let mut api_keys = HashMap::new();
        api_keys.insert(
            "user-key".to_string(),
            Principal::new("u", UserRole::User, tenant()),
        );
        api_keys.insert(
            "admin-key".to_string(),
            Principal::new("a", UserRole::Admin, tenant()),
        );
        api_keys.insert(
            "foreign-key".to_string(),
            Principal::new(
                "f",
                UserRole::User,
                paladin_core::platform::container::principal::TenantId::new("other-tenant")
                    .unwrap(),
            ),
        );
        let auth = crate::agent_auth::AgentAuthConfig {
            enabled: true,
            api_keys,
            token_verifier: None,
            bearer_tenant: None,
        };
        // `scoped-thread` has a latest Waypoint and one run attributed to `user-key`'s
        // tenant, so the visibility gate admits `user-key` and `admin-key` and hides the
        // thread from `foreign-key` (TENANT-03).
        let store = Arc::new(MockWaypointStore::default());
        let t = thread("scoped-thread");
        store.seed_latest(sample_waypoint(&t, 1, WaypointStatus::Completed));
        let runs = Arc::new(MockRunRepository::default());
        runs.seed_run(run_on_for_tenant(&t, "test-tenant", "u"));
        let state = ThreadApiState::new()
            .with_waypoints(store)
            .with_runs(runs)
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
        // narrowed to admin by CR-01. The user-role key owns `scoped-thread`
        // (its tenant has the latest run), so the read is 200, not 403.
        let parley_id = ParleyId::new();
        let app = authed_thread_app(parley_id);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/threads/scoped-thread/state")
                    .header("x-api-key", "user-key")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(response.status(), StatusCode::OK);
    }

    /// D-06 / T-53-14: the visibility gate runs BEFORE `require_admin` on resume, so a
    /// non-admin of another tenant gets the missing-thread 404 -- never the 403 that
    /// would confirm the thread exists -- and the 404 equals the response for a thread
    /// that does not exist. The owning tenant's user still gets the 403.
    #[tokio::test]
    async fn post_resume_by_a_foreign_user_is_404_not_403() {
        let parley_id = ParleyId::new();
        let app = authed_thread_app(parley_id);
        let body = serde_json::json!({
            "responses": [
                { "parley_id": parley_id.to_string(), "value": true, "responded_by": "alice" }
            ]
        });

        let hidden = send_thread_request(
            &app,
            "POST",
            "/v1/threads/scoped-thread/resume".to_string(),
            "foreign-key",
            Some(body.clone()),
            &[],
        )
        .await;
        assert_eq!(hidden.status(), StatusCode::NOT_FOUND, "never 403");
        let missing = send_thread_request(
            &app,
            "POST",
            "/v1/threads/no-such-9f3a/resume".to_string(),
            "foreign-key",
            Some(body.clone()),
            &[],
        )
        .await;
        assert_hidden_equals_missing(
            hidden,
            missing,
            "scoped-thread",
            "no-such-9f3a",
            "foreign resume",
        )
        .await;

        let same_tenant = send_thread_request(
            &app,
            "POST",
            "/v1/threads/scoped-thread/resume".to_string(),
            "user-key",
            Some(body),
            &[],
        )
        .await;
        assert_eq!(same_tenant.status(), StatusCode::FORBIDDEN);
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
            "/threads",
            "/threads/{id}",
            "/threads/{id}/state",
            "/threads/{id}/resume",
            "/threads/{id}/history",
            "/threads/{id}/fork",
        ] {
            assert!(
                api.paths.paths.contains_key(expected),
                "spec missing path {expected}"
            );
        }
        // `DELETE /threads/{id}` shares the SAME path key as `GET
        // /threads/{id}` -- assert the `delete` operation specifically.
        assert!(
            api.paths.paths["/threads/{id}"].delete.is_some(),
            "spec missing DELETE /threads/{{id}}"
        );
    }

    // --- list_threads / get_thread (D-45) ----------------------------------

    #[derive(Default)]
    struct MockRunRepository {
        active: Mutex<HashMap<String, Run>>,
        /// Seeded run history for the thread visibility gate: `list` honors
        /// `RunQuery.thread_id`, `scope` and `limit` over these, ordered
        /// `(submitted_at DESC, run_id DESC)` like every real repository.
        runs: Mutex<Vec<Run>>,
        /// When set, `get`, `list` and `active_run_for_thread` fail with a
        /// `Serialization` error carrying this text (Phase 53 thread gate tests).
        fail_reads_with: Mutex<Option<String>>,
        /// Number of `list` calls made (the thread list walk's owner lookups).
        list_calls: std::sync::atomic::AtomicUsize,
    }

    impl MockRunRepository {
        fn seed_run(&self, run: Run) {
            self.runs.lock().unwrap().push(run);
        }

        fn fail_reads_with(&self, message: &str) {
            *self.fail_reads_with.lock().unwrap() = Some(message.to_string());
        }

        fn read_failure(
            &self,
        ) -> Result<(), paladin_ports::output::run_repository_port::RunRepositoryError> {
            match self.fail_reads_with.lock().unwrap().as_ref() {
                Some(message) => Err(
                    paladin_ports::output::run_repository_port::RunRepositoryError::Serialization {
                        message: message.clone(),
                    },
                ),
                None => Ok(()),
            }
        }
    }

    #[async_trait]
    impl RunRepositoryPort for MockRunRepository {
        async fn insert(
            &self,
            _run: &Run,
        ) -> Result<(), paladin_ports::output::run_repository_port::RunRepositoryError> {
            Ok(())
        }

        async fn get(
            &self,
            run_id: &RunId,
        ) -> Result<Option<Run>, paladin_ports::output::run_repository_port::RunRepositoryError>
        {
            self.read_failure()?;
            Ok(self
                .runs
                .lock()
                .unwrap()
                .iter()
                .find(|run| &run.run_id == run_id)
                .cloned())
        }

        async fn update_status(
            &self,
            _run_id: &RunId,
            _from: RunStatus,
            _to: RunStatus,
            _at: DateTime<Utc>,
        ) -> Result<(), paladin_ports::output::run_repository_port::RunRepositoryError> {
            Ok(())
        }

        async fn record_outcome(
            &self,
            _run_id: &RunId,
            _outcome: paladin_ports::output::run_repository_port::RunOutcomeRecord,
        ) -> Result<(), paladin_ports::output::run_repository_port::RunRepositoryError> {
            Ok(())
        }

        async fn list(
            &self,
            query: paladin_ports::output::run_repository_port::RunQuery,
        ) -> Result<
            paladin_ports::output::run_repository_port::RunPage,
            paladin_ports::output::run_repository_port::RunRepositoryError,
        > {
            self.list_calls
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.read_failure()?;
            let mut items: Vec<Run> = self
                .runs
                .lock()
                .unwrap()
                .iter()
                .filter(|run| {
                    query
                        .thread_id
                        .as_ref()
                        .is_none_or(|thread| &run.thread_id == thread)
                        && query.scope.permits(run)
                })
                .cloned()
                .collect();
            items.sort_by(|a, b| {
                b.submitted_at
                    .cmp(&a.submitted_at)
                    .then_with(|| b.run_id.to_string().cmp(&a.run_id.to_string()))
            });
            if query.limit > 0 {
                items.truncate(query.limit as usize);
            }
            Ok(paladin_ports::output::run_repository_port::RunPage {
                items,
                next_cursor: None,
            })
        }

        async fn active_run_for_thread(
            &self,
            thread_id: &ThreadId,
        ) -> Result<Option<Run>, paladin_ports::output::run_repository_port::RunRepositoryError>
        {
            self.read_failure()?;
            Ok(self.active.lock().unwrap().get(thread_id.as_str()).cloned())
        }

        async fn request_cancel(
            &self,
            run_id: &RunId,
        ) -> Result<RunStatus, paladin_ports::output::run_repository_port::RunRepositoryError>
        {
            Err(
                paladin_ports::output::run_repository_port::RunRepositoryError::NotFound {
                    run_id: run_id.clone(),
                },
            )
        }

        async fn is_cancel_requested(
            &self,
            _thread_id: &ThreadId,
        ) -> Result<bool, paladin_ports::output::run_repository_port::RunRepositoryError> {
            Ok(false)
        }

        async fn bump_attempt(
            &self,
            _run_id: &RunId,
        ) -> Result<u32, paladin_ports::output::run_repository_port::RunRepositoryError> {
            Ok(1)
        }

        async fn record_resume(
            &self,
            _run_id: &RunId,
            _responses: Vec<ParleyResponse>,
        ) -> Result<u32, paladin_ports::output::run_repository_port::RunRepositoryError> {
            Ok(1)
        }

        async fn clear_pending_responses(
            &self,
            _run_id: &RunId,
        ) -> Result<(), paladin_ports::output::run_repository_port::RunRepositoryError> {
            Ok(())
        }
    }

    fn sample_run_for(thread: &ThreadId) -> Run {
        use paladin_core::platform::container::run::AssistantRef;
        Run::new(
            RunId::new_v7(),
            thread.clone(),
            AssistantRef {
                assistant_id: "a1".to_string(),
                version: 1,
            },
            serde_json::json!({}),
        )
    }

    /// A run on `thread` attributed to `tenant`/`key`, submitted now.
    fn run_on_for_tenant(thread: &ThreadId, tenant: &str, key: &str) -> Run {
        run_on(thread, tenant, key, Utc::now())
    }

    // --- Tenant visibility of single-thread routes (Phase 53, plan 53-03, TENANT-03) ---

    use axum::response::Response;
    use paladin_core::platform::container::principal::{RunAttribution, TenantId};

    /// The four auth-enabled keys every visibility test uses (research Pitfall 6: with
    /// auth disabled every caller is Admin and the gate would never be exercised):
    /// `owner-key` and `peer-key` are `User`s of `acme`, `foreign-key` a `User` of
    /// `globex`, `admin-key` an Admin of `ops-tenant`.
    fn tenant_thread_auth() -> crate::agent_auth::AgentAuthConfig {
        let mut api_keys = HashMap::new();
        api_keys.insert(
            "owner-key".to_string(),
            Principal::new("svc-a", UserRole::User, TenantId::new("acme").unwrap()),
        );
        api_keys.insert(
            "peer-key".to_string(),
            Principal::new("svc-a2", UserRole::User, TenantId::new("acme").unwrap()),
        );
        api_keys.insert(
            "foreign-key".to_string(),
            Principal::new("svc-b", UserRole::User, TenantId::new("globex").unwrap()),
        );
        api_keys.insert(
            "admin-key".to_string(),
            Principal::new("ops", UserRole::Admin, TenantId::new("ops-tenant").unwrap()),
        );
        crate::agent_auth::AgentAuthConfig {
            enabled: true,
            api_keys,
            token_verifier: None,
            bearer_tenant: None,
        }
    }

    /// A run on `thread` attributed to `tenant`/`key`, submitted at `at`.
    fn run_on(thread: &ThreadId, tenant: &str, key: &str, at: DateTime<Utc>) -> Run {
        let mut run = sample_run_for(thread)
            .with_submitted_by(RunAttribution::new(TenantId::new(tenant).unwrap(), key));
        run.submitted_at = at;
        run
    }

    /// The doubles behind a visibility test, kept so a test can seed more runs after the
    /// router is built.
    struct ThreadFixture {
        waypoints: Arc<MockWaypointStore>,
        runs: Arc<MockRunRepository>,
    }

    impl ThreadFixture {
        fn new() -> Self {
            Self {
                waypoints: Arc::new(MockWaypointStore::default()),
                runs: Arc::new(MockRunRepository::default()),
            }
        }

        /// Seed a latest Waypoint for `name`.
        fn waypoint(&self, name: &str) -> ThreadId {
            let t = thread(name);
            self.waypoints
                .seed_latest(sample_waypoint(&t, 1, WaypointStatus::Completed));
            t
        }

        /// Seed one thread for the list walk tests: a latest Waypoint, a `list_threads`
        /// row whose `last_updated_at` is `list_epoch() + rank` seconds (a higher `rank`
        /// is newer), and, for `Some(tenant)`, one run on it attributed to that tenant
        /// submitted at the same instant. `None` leaves the thread with no run.
        fn listed_thread(&self, name: &str, rank: i64, owner: Option<&str>) -> ThreadId {
            let t = self.waypoint(name);
            let at = list_epoch() + chrono::Duration::seconds(rank);
            self.waypoints.seed_listed(ThreadSummary {
                thread_id: t.clone(),
                latest_status: WaypointStatus::Completed,
                last_updated_at: at,
            });
            if let Some(tenant) = owner {
                self.runs.seed_run(run_on(&t, tenant, "svc", at));
            }
            t
        }

        /// A state with the four-key auth, both stores and (when `with_runs`) the run store.
        fn state(&self, with_runs: bool) -> ThreadApiState {
            let state = ThreadApiState::new()
                .with_waypoints(self.waypoints.clone())
                .with_auth(tenant_thread_auth());
            if with_runs {
                state.with_runs(self.runs.clone())
            } else {
                state
            }
        }
    }

    /// The fixed instant list walk tests count seconds from.
    fn list_epoch() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_800_000_000, 0).expect("valid timestamp")
    }

    async fn send_thread_request(
        app: &axum::Router,
        method: &str,
        uri: String,
        key: &str,
        body: Option<serde_json::Value>,
        extra_headers: &[(&str, &str)],
    ) -> Response {
        let mut builder = Request::builder()
            .method(method)
            .uri(uri)
            .header("x-api-key", key);
        for (name, value) in extra_headers {
            builder = builder.header(*name, *value);
        }
        let request = match body {
            Some(json) => builder
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&json).unwrap())),
            None => builder.body(Body::empty()),
        };
        app.clone()
            .oneshot(request.expect("request builds"))
            .await
            .expect("router responds")
    }

    /// `(status, sorted headers, body text)` of a response.
    async fn response_parts(response: Response) -> (StatusCode, Vec<(String, String)>, String) {
        let status = response.status();
        let mut headers: Vec<(String, String)> = response
            .headers()
            .iter()
            .filter(|(name, _)| name.as_str() != "content-length")
            .map(|(name, value)| {
                (
                    name.as_str().to_string(),
                    value.to_str().unwrap_or("<non-utf8>").to_string(),
                )
            })
            .collect();
        headers.sort();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        (
            status,
            headers,
            String::from_utf8(bytes.to_vec()).expect("utf8 body"),
        )
    }

    /// Hidden must equal missing in status, every header and body bytes once the two ids
    /// are swapped (ADR-0054 rule 5). `content-length` is the one derived header left out
    /// of the header comparison: it is a pure function of the body, and the two ids differ
    /// in length, so it differs by exactly that difference; the swapped-body equality below
    /// is what proves the lengths agree, so a length oracle is still caught.
    async fn assert_hidden_equals_missing(
        hidden: Response,
        missing: Response,
        hidden_id: &str,
        missing_id: &str,
        what: &str,
    ) {
        let (h_status, h_headers, h_body) = response_parts(hidden).await;
        let (m_status, m_headers, m_body) = response_parts(missing).await;
        assert_eq!(h_status, m_status, "{what}: status");
        assert_eq!(h_headers, m_headers, "{what}: headers");
        assert_eq!(
            h_body.replace(hidden_id, missing_id),
            m_body,
            "{what}: body bytes after the id swap"
        );
    }

    #[tokio::test]
    async fn get_thread_is_visible_to_its_tenant_and_admin_and_hidden_from_others() {
        let fixture = ThreadFixture::new();
        let t = fixture.waypoint("acme-thread-1");
        fixture
            .runs
            .seed_run(run_on(&t, "acme", "svc-a", Utc::now()));
        let app = thread_router(fixture.state(true));

        for key in ["owner-key", "peer-key", "admin-key"] {
            let response = send_thread_request(
                &app,
                "GET",
                "/v1/threads/acme-thread-1".to_string(),
                key,
                None,
                &[],
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK, "{key} sees its thread");
        }

        let hidden = send_thread_request(
            &app,
            "GET",
            "/v1/threads/acme-thread-1".to_string(),
            "foreign-key",
            None,
            &[],
        )
        .await;
        assert_eq!(hidden.status(), StatusCode::NOT_FOUND);
        let missing = send_thread_request(
            &app,
            "GET",
            "/v1/threads/no-such-9f3a".to_string(),
            "foreign-key",
            None,
            &[],
        )
        .await;
        assert_hidden_equals_missing(
            hidden,
            missing,
            "acme-thread-1",
            "no-such-9f3a",
            "GET /v1/threads/{id} as a foreign tenant",
        )
        .await;
    }

    #[tokio::test]
    async fn zero_run_thread_is_visible_to_admin_only() {
        let fixture = ThreadFixture::new();
        fixture.waypoint("zero-run-thread");
        let app = thread_router(fixture.state(true));

        let admin_response = send_thread_request(
            &app,
            "GET",
            "/v1/threads/zero-run-thread".to_string(),
            "admin-key",
            None,
            &[],
        )
        .await;
        assert_eq!(admin_response.status(), StatusCode::OK);

        for key in ["owner-key", "foreign-key"] {
            let hidden = send_thread_request(
                &app,
                "GET",
                "/v1/threads/zero-run-thread".to_string(),
                key,
                None,
                &[],
            )
            .await;
            let missing = send_thread_request(
                &app,
                "GET",
                "/v1/threads/no-such-9f3a".to_string(),
                key,
                None,
                &[],
            )
            .await;
            assert_eq!(hidden.status(), StatusCode::NOT_FOUND, "{key}");
            assert_hidden_equals_missing(
                hidden,
                missing,
                "zero-run-thread",
                "no-such-9f3a",
                "zero-run thread for a non-admin",
            )
            .await;
        }
    }

    #[tokio::test]
    async fn mixed_tenant_thread_belongs_to_the_latest_runs_tenant() {
        let fixture = ThreadFixture::new();
        let t = fixture.waypoint("legacy-thread");
        let t0 = Utc::now();
        fixture.runs.seed_run(run_on(&t, "acme", "svc-a", t0));
        fixture.runs.seed_run(run_on(
            &t,
            "globex",
            "svc-b",
            t0 + chrono::Duration::seconds(1),
        ));
        let app = thread_router(fixture.state(true));
        let status_for = |key: &'static str| {
            let app = app.clone();
            async move {
                send_thread_request(
                    &app,
                    "GET",
                    "/v1/threads/legacy-thread".to_string(),
                    key,
                    None,
                    &[],
                )
                .await
                .status()
            }
        };

        // The newest run is globex's: globex and Admin see the thread, acme does not.
        assert_eq!(status_for("foreign-key").await, StatusCode::OK);
        assert_eq!(status_for("admin-key").await, StatusCode::OK);
        assert_eq!(status_for("owner-key").await, StatusCode::NOT_FOUND);

        // A still newer acme run moves the thread back to acme.
        fixture.runs.seed_run(run_on(
            &t,
            "acme",
            "svc-a",
            t0 + chrono::Duration::seconds(2),
        ));
        assert_eq!(status_for("owner-key").await, StatusCode::OK);
        assert_eq!(status_for("admin-key").await, StatusCode::OK);
        assert_eq!(status_for("foreign-key").await, StatusCode::NOT_FOUND);
    }

    /// An unreadable Waypoint or run row under a thread id is an existence signal, so for
    /// a `Tenant` scope it is the missing-thread `404` and for an Admin the fixed `500`;
    /// an Admin performs no owner lookup, so a run-store failure does not touch its `GET`
    /// (D-09, PD-03-5). No body carries the injected backend text.
    #[tokio::test]
    async fn unreadable_rows_on_get_thread_follow_the_role_aware_redaction() {
        let get = |app: axum::Router, id: &'static str, key: &'static str| async move {
            send_thread_request(&app, "GET", format!("/v1/threads/{id}"), key, None, &[]).await
        };

        // Waypoint `Serialization` on `latest()`.
        let fixture = ThreadFixture::new();
        let t = fixture.waypoint("acme-thread-1");
        fixture
            .runs
            .seed_run(run_on(&t, "acme", "svc-a", Utc::now()));
        fixture
            .waypoints
            .fail_latest(WaypointFailure::Serialization);
        let app = thread_router(fixture.state(true));
        for key in ["owner-key", "foreign-key"] {
            let hidden = get(app.clone(), "acme-thread-1", key).await;
            let missing = get(app.clone(), "no-such-9f3a", key).await;
            let (status, _, body) = response_parts(hidden).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{key}");
            assert!(!body.contains(INJECTED_TEXT), "{key}: {body}");
            // Both ids fail identically under the seam, so the responses must be equal.
            let missing_parts = response_parts(missing).await;
            assert_eq!(missing_parts.0, StatusCode::NOT_FOUND, "{key}");
            assert_eq!(
                body.replace("acme-thread-1", "no-such-9f3a"),
                missing_parts.2,
                "{key}"
            );
        }
        let (status, _, body) = response_parts(get(app, "acme-thread-1", "admin-key").await).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(body.contains(THREAD_STORE_ERROR_MESSAGE), "{body}");
        assert!(!body.contains(INJECTED_TEXT), "{body}");

        // A newer-schema Waypoint is hideable too; a `Backend` outage never is (it is not an
        // existence signal), so even a `Tenant` scope gets the fixed 500.
        for (failure, tenant_status) in [
            (WaypointFailure::SchemaVersion, StatusCode::NOT_FOUND),
            (WaypointFailure::Backend, StatusCode::INTERNAL_SERVER_ERROR),
        ] {
            let fixture = ThreadFixture::new();
            let t = fixture.waypoint("acme-thread-1");
            fixture
                .runs
                .seed_run(run_on(&t, "acme", "svc-a", Utc::now()));
            fixture.waypoints.fail_latest(failure);
            let app = thread_router(fixture.state(true));
            let (status, _, body) =
                response_parts(get(app.clone(), "acme-thread-1", "owner-key").await).await;
            assert_eq!(status, tenant_status);
            assert!(!body.contains(INJECTED_TEXT), "{body}");
            if tenant_status == StatusCode::INTERNAL_SERVER_ERROR {
                assert!(body.contains(THREAD_STORE_ERROR_MESSAGE), "{body}");
            }
        }

        // Run-store `Serialization` on the owner lookup.
        let fixture = ThreadFixture::new();
        let t = fixture.waypoint("acme-thread-1");
        fixture
            .runs
            .seed_run(run_on(&t, "acme", "svc-a", Utc::now()));
        fixture.runs.fail_reads_with(INJECTED_TEXT);
        let app = thread_router(fixture.state(true));
        for key in ["owner-key", "foreign-key"] {
            let (status, _, body) =
                response_parts(get(app.clone(), "acme-thread-1", key).await).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{key}");
            assert!(!body.contains(INJECTED_TEXT), "{key}: {body}");
        }
        let admin_response = get(app, "acme-thread-1", "admin-key").await;
        assert_eq!(admin_response.status(), StatusCode::OK);
    }

    // --- The `/v1/threads/{id}*` route matrix (Phase 53, plan 53-03, D-08, AC-01, AC-06) ---

    /// Every `(METHOD, path)` operation under `/v1/threads/{id}` the router publishes, read
    /// from its OWN OpenAPI document (`versioned_thread_parts(..).1`) rather than a hand
    /// list -- so a route added to `thread_openapi_router` without joining the table (and
    /// the gate) fails the matrix (research Pitfall 10, AC-06).
    fn published_thread_id_operations(api: &utoipa::openapi::OpenApi) -> Vec<(String, String)> {
        let mut operations = Vec::new();
        for (path, item) in &api.paths.paths {
            if !path.starts_with("/v1/threads/{id}") {
                continue;
            }
            let by_method = [
                ("GET", item.get.is_some()),
                ("PUT", item.put.is_some()),
                ("POST", item.post.is_some()),
                ("DELETE", item.delete.is_some()),
                ("OPTIONS", item.options.is_some()),
                ("HEAD", item.head.is_some()),
                ("PATCH", item.patch.is_some()),
                ("TRACE", item.trace.is_some()),
            ];
            for (method, present) in by_method {
                if present {
                    operations.push((method.to_string(), path.clone()));
                }
            }
        }
        operations.sort();
        operations
    }

    /// `(method, path template, status for the owner as a user, status for an Admin)` of
    /// every `/v1/threads/{id}*` operation. `resume` and `delete` are Admin-only (CR-01), so
    /// a same-tenant user gets `403` (D-06) after passing the visibility gate.
    const THREAD_ROUTE_TABLE: [(&str, &str, u16, u16); 6] = [
        ("GET", "/v1/threads/{id}", 200, 200),
        ("GET", "/v1/threads/{id}/state", 200, 200),
        ("GET", "/v1/threads/{id}/history", 200, 200),
        ("POST", "/v1/threads/{id}/fork", 202, 202),
        ("POST", "/v1/threads/{id}/resume", 403, 202),
        ("DELETE", "/v1/threads/{id}", 403, 204),
    ];

    impl ThreadFixture {
        /// `acme-thread-1` with a latest Waypoint, one history item and one terminal run
        /// attributed to `acme`.
        fn seeded() -> Self {
            let fixture = Self::new();
            let t = fixture.waypoint("acme-thread-1");
            let mut run = run_on(&t, "acme", "svc-a", Utc::now());
            run.status = RunStatus::Completed;
            fixture.runs.seed_run(run);
            fixture
                .waypoints
                .seed_history(&t, vec![summary_at(WaypointId::generate(), 1, Utc::now())]);
            fixture
        }

        /// The full router state: both stores (and, when `with_runs`, the run store), a
        /// parley port and a fork submission port that accept, and the four-key auth.
        fn full_state(&self, with_runs: bool) -> ThreadApiState {
            self.state(with_runs)
                .with_parley(Arc::new(MockParleyPort {
                    outcome: MockOutcome::Accepted,
                    parley_id: ParleyId::new(),
                }))
                .with_run_submission(Arc::new(MockForkSubmissionPort {
                    outcome: ForkOutcome::Accepted,
                }))
        }
    }

    /// Send one table row's operation for thread `id`, with a valid body where the route
    /// takes one.
    async fn send_row(
        app: &axum::Router,
        method: &str,
        template: &str,
        id: &str,
        key: &str,
        extra_headers: &[(&str, &str)],
    ) -> Response {
        let body = if template.ends_with("/fork") {
            Some(serde_json::json!({ "from_waypoint_id": WaypointId::generate().to_string() }))
        } else if template.ends_with("/resume") {
            Some(serde_json::json!({
                "responses": [
                    { "parley_id": ParleyId::new().to_string(), "value": true, "responded_by": "alice" }
                ]
            }))
        } else {
            None
        };
        send_thread_request(
            app,
            method,
            template.replace("{id}", id),
            key,
            body,
            extra_headers,
        )
        .await
    }

    /// D-08 / AC-01 / AC-06 on the WHOLE `/v1/threads/{id}*` route set: the operations are
    /// enumerated from the router's own OpenAPI document and compared against the table;
    /// for every row the owner (and a same-tenant peer) and an Admin of another tenant get
    /// the documented status, and a key of a different tenant gets a `404` whose status,
    /// headers and body bytes (after swapping the two ids) equal the response for a thread
    /// that never existed -- never a `403`, on `resume` and `delete` too.
    #[tokio::test]
    async fn every_thread_id_route_hides_foreign_threads_behind_the_missing_thread_404() {
        let mut tabled: Vec<(String, String)> = THREAD_ROUTE_TABLE
            .iter()
            .map(|(method, path, _, _)| (method.to_string(), path.to_string()))
            .collect();
        tabled.sort();

        for (method, template, owner_status, admin_status) in THREAD_ROUTE_TABLE {
            // A fresh state per row keeps the DELETE row from disturbing the others.
            let fixture = ThreadFixture::seeded();
            let (app, api) = versioned_thread_parts(fixture.full_state(true));
            let published = published_thread_id_operations(&api);
            assert_eq!(
                published, tabled,
                "the /v1/threads/{{id}}* operations the router publishes do not match the \
                 matrix table: published={published:?} table={tabled:?}. Every route under \
                 /v1/threads/{{id}} must enter through `load_visible_thread` (D-05, TENANT-03) \
                 and be added to THREAD_ROUTE_TABLE with its owner and Admin statuses; a route \
                 that no longer exists must be removed from the table."
            );

            for key in ["owner-key", "peer-key"] {
                let response = send_row(&app, method, template, "acme-thread-1", key, &[]).await;
                assert_eq!(
                    response.status().as_u16(),
                    owner_status,
                    "{method} {template} as {key} (a user of the owning tenant)"
                );
            }

            let hidden =
                send_row(&app, method, template, "acme-thread-1", "foreign-key", &[]).await;
            assert_eq!(
                hidden.status(),
                StatusCode::NOT_FOUND,
                "{method} {template} as a foreign tenant must be the missing-thread 404, never 403"
            );
            let missing =
                send_row(&app, method, template, "no-such-9f3a", "foreign-key", &[]).await;
            assert_hidden_equals_missing(
                hidden,
                missing,
                "acme-thread-1",
                "no-such-9f3a",
                &format!("{method} {template} as a foreign tenant"),
            )
            .await;

            // Admin last: on the DELETE row it removes the thread.
            let response =
                send_row(&app, method, template, "acme-thread-1", "admin-key", &[]).await;
            assert_eq!(
                response.status().as_u16(),
                admin_status,
                "{method} {template} as an Admin of another tenant"
            );
        }
    }

    /// AC-05: once an Admin `DELETE` returns 204 the thread is gone, so a second `DELETE`
    /// is the same missing-thread 404 for the Admin and for every other role.
    #[tokio::test]
    async fn second_delete_of_a_thread_is_404_for_every_role() {
        let fixture = ThreadFixture::seeded();
        let app = thread_router(fixture.full_state(true));
        let delete = |key: &'static str| {
            let app = app.clone();
            async move {
                send_thread_request(
                    &app,
                    "DELETE",
                    "/v1/threads/acme-thread-1".to_string(),
                    key,
                    None,
                    &[],
                )
                .await
            }
        };

        assert_eq!(delete("admin-key").await.status(), StatusCode::NO_CONTENT);
        for key in ["admin-key", "owner-key", "foreign-key"] {
            let (status, _, body) = response_parts(delete(key).await).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "second DELETE as {key}");
            let json: serde_json::Value = serde_json::from_str(&body).expect("json body");
            assert_eq!(json["error"]["code"], "not_found", "{key}");
            assert_eq!(
                json["error"]["message"], "unknown thread 'acme-thread-1'",
                "{key}"
            );
        }
    }

    /// D-07: with a waypoint store but no run store, ownership cannot be established, so a
    /// non-admin gets the missing-thread 404 on EVERY `{id}` route (equal to a thread that
    /// never existed) while an Admin is unaffected.
    #[tokio::test]
    async fn thread_routes_fail_closed_for_non_admin_without_a_run_store() {
        for (method, template, _, admin_status) in THREAD_ROUTE_TABLE {
            let fixture = ThreadFixture::seeded();
            let app = thread_router(fixture.full_state(false));
            for key in ["owner-key", "foreign-key"] {
                let hidden = send_row(&app, method, template, "acme-thread-1", key, &[]).await;
                let missing = send_row(&app, method, template, "no-such-9f3a", key, &[]).await;
                assert_eq!(
                    hidden.status(),
                    StatusCode::NOT_FOUND,
                    "{method} {template} as {key} without a run store"
                );
                assert_hidden_equals_missing(
                    hidden,
                    missing,
                    "acme-thread-1",
                    "no-such-9f3a",
                    &format!("{method} {template} as {key}, no run store"),
                )
                .await;
            }
            let response =
                send_row(&app, method, template, "acme-thread-1", "admin-key", &[]).await;
            assert_eq!(
                response.status().as_u16(),
                admin_status,
                "{method} {template} as an Admin without a run store"
            );
        }
    }

    /// The `error.message` of a response body, parsed.
    fn error_message(body: &str) -> String {
        serde_json::from_str::<serde_json::Value>(body)
            .expect("json error body")["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .to_string()
    }

    /// T-53-15 / T-53-16, AC-09 / AC-10 (thread half), on every published operation.
    ///
    /// With the run store failing (`Serialization`): the owner lookup cannot establish
    /// ownership, so BOTH user keys (the owner included) get the missing-thread 404, equal
    /// to a thread that never existed. An Admin performs no owner lookup (PD-03-5), so the
    /// `GET`, fork and resume routes keep their success status, and only `DELETE` -- whose
    /// busy check loads the active run -- answers `500` with exactly `run store error`.
    ///
    /// With the waypoint store failing on `latest()` (`Serialization`, then
    /// `SchemaVersionUnsupported`): the user keys get the same 404 and an Admin gets `500`
    /// `thread store error`. No body contains the injected text.
    #[tokio::test]
    async fn serialization_on_thread_routes_is_hidden_for_tenant_and_redacted_for_admin() {
        // --- run store failing ---
        for (method, template, _, admin_status) in THREAD_ROUTE_TABLE {
            let fixture = ThreadFixture::seeded();
            fixture.runs.fail_reads_with(INJECTED_TEXT);
            let app = thread_router(fixture.full_state(true));
            for key in ["owner-key", "foreign-key"] {
                let hidden = send_row(&app, method, template, "acme-thread-1", key, &[]).await;
                let missing = send_row(&app, method, template, "no-such-9f3a", key, &[]).await;
                assert_eq!(
                    hidden.status(),
                    StatusCode::NOT_FOUND,
                    "{method} {template} as {key}, run store failing"
                );
                let (_, _, body) = response_parts(
                    send_row(&app, method, template, "acme-thread-1", key, &[]).await,
                )
                .await;
                assert!(
                    !body.contains(INJECTED_TEXT),
                    "{method} {template} {key}: {body}"
                );
                assert_hidden_equals_missing(
                    hidden,
                    missing,
                    "acme-thread-1",
                    "no-such-9f3a",
                    &format!("{method} {template} as {key}, run store failing"),
                )
                .await;
            }
            let response =
                send_row(&app, method, template, "acme-thread-1", "admin-key", &[]).await;
            if method == "DELETE" {
                let (status, _, body) = response_parts(response).await;
                assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
                assert_eq!(error_message(&body), "run store error");
                assert!(!body.contains(INJECTED_TEXT), "{body}");
            } else {
                assert_eq!(
                    response.status().as_u16(),
                    admin_status,
                    "{method} {template} as an Admin keeps its success status: the gate does no owner lookup"
                );
            }
        }

        // --- waypoint store failing ---
        for failure in [
            WaypointFailure::Serialization,
            WaypointFailure::SchemaVersion,
        ] {
            for (method, template, _, _) in THREAD_ROUTE_TABLE {
                let fixture = ThreadFixture::seeded();
                fixture.waypoints.fail_latest(failure);
                let app = thread_router(fixture.full_state(true));
                for key in ["owner-key", "foreign-key"] {
                    let hidden = send_row(&app, method, template, "acme-thread-1", key, &[]).await;
                    let missing = send_row(&app, method, template, "no-such-9f3a", key, &[]).await;
                    assert_eq!(
                        hidden.status(),
                        StatusCode::NOT_FOUND,
                        "{method} {template} as {key}, waypoint store failing"
                    );
                    assert_hidden_equals_missing(
                        hidden,
                        missing,
                        "acme-thread-1",
                        "no-such-9f3a",
                        &format!("{method} {template} as {key}, waypoint store failing"),
                    )
                    .await;
                }
                let (status, _, body) = response_parts(
                    send_row(&app, method, template, "acme-thread-1", "admin-key", &[]).await,
                )
                .await;
                assert_eq!(
                    status,
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "{method} {template} as an Admin, waypoint store failing"
                );
                assert_eq!(error_message(&body), THREAD_STORE_ERROR_MESSAGE);
                assert!(!body.contains(INJECTED_TEXT), "{body}");
            }
        }
    }

    /// T-53-17 (ADR-0054 rule 1): the tenant is server-derived. A foreign key that names
    /// the owning tenant in headers or the query still gets the missing-thread 404 on the
    /// three `GET` routes, and the owner naming another tenant still gets `200`.
    #[tokio::test]
    async fn tenant_hints_in_headers_or_query_change_nothing_on_thread_routes() {
        let fixture = ThreadFixture::seeded();
        let app = thread_router(fixture.full_state(true));
        let hints = [("x-tenant-id", "acme"), ("x-paladin-tenant", "acme")];

        for (suffix, query) in [
            ("", "?tenant_id=acme"),
            ("/state", "?tenant_id=acme"),
            ("/history", "?tenant_id=acme&limit=5"),
        ] {
            let hidden = send_thread_request(
                &app,
                "GET",
                format!("/v1/threads/acme-thread-1{suffix}{query}"),
                "foreign-key",
                None,
                &hints,
            )
            .await;
            let missing = send_thread_request(
                &app,
                "GET",
                format!("/v1/threads/no-such-9f3a{suffix}{query}"),
                "foreign-key",
                None,
                &hints,
            )
            .await;
            assert_eq!(
                hidden.status(),
                StatusCode::NOT_FOUND,
                "GET {suffix} with hints"
            );
            assert_hidden_equals_missing(
                hidden,
                missing,
                "acme-thread-1",
                "no-such-9f3a",
                &format!("GET /v1/threads/{{id}}{suffix} with tenant hints"),
            )
            .await;

            let owner = send_thread_request(
                &app,
                "GET",
                format!("/v1/threads/acme-thread-1{suffix}?tenant_id=globex"),
                "owner-key",
                None,
                &[("x-tenant-id", "globex"), ("x-paladin-tenant", "globex")],
            )
            .await;
            assert_eq!(
                owner.status(),
                StatusCode::OK,
                "owner GET {suffix} naming globex"
            );
        }
    }

    #[tokio::test]
    async fn list_threads_returns_501_when_unwired() {
        let state = ThreadApiState::new();
        let err = list_threads(
            State(state),
            admin(),
            Query(ThreadListQuery {
                limit: None,
                cursor: None,
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn list_threads_empty_is_200_with_empty_items() {
        let state = state_with_waypoints(Arc::new(MockWaypointStore::default()));
        let (status, Json(body)) = list_threads(
            State(state),
            admin(),
            Query(ThreadListQuery {
                limit: None,
                cursor: None,
            }),
        )
        .await
        .expect("ok");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["items"].as_array().unwrap().len(), 0);
        assert!(body["next_cursor"].is_null());
    }

    #[tokio::test]
    async fn list_threads_rejects_limit_zero() {
        let state = state_with_waypoints(Arc::new(MockWaypointStore::default()));
        let err = list_threads(
            State(state),
            admin(),
            Query(ThreadListQuery {
                limit: Some(0),
                cursor: None,
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
    }

    // --- Tenant-filtered thread list (Phase 53, plan 53-04, TENANT-03, D-04) ---

    /// `GET /v1/threads{query}` as `key`, returning `(status, parsed JSON body)`.
    async fn list_as(
        app: &axum::Router,
        key: &str,
        query: &str,
    ) -> (StatusCode, serde_json::Value) {
        let response =
            send_thread_request(app, "GET", format!("/v1/threads{query}"), key, None, &[]).await;
        let (status, _headers, body) = response_parts(response).await;
        let json = serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);
        (status, json)
    }

    /// The `thread_id`s of a list body's `items`, in order.
    fn listed_ids(body: &serde_json::Value) -> Vec<String> {
        body["items"]
            .as_array()
            .expect("items is an array")
            .iter()
            .map(|item| item["thread_id"].as_str().expect("thread_id").to_string())
            .collect()
    }

    #[tokio::test]
    async fn list_threads_omits_foreign_and_zero_run_threads() {
        let fixture = ThreadFixture::new();
        fixture.listed_thread("globex-2", 4, Some("globex"));
        fixture.listed_thread("acme-2", 3, Some("acme"));
        fixture.listed_thread("zero-1", 2, None);
        fixture.listed_thread("acme-1", 1, Some("acme"));
        let app = thread_router(fixture.state(true));

        let (status, owner) = list_as(&app, "owner-key", "?limit=10").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            listed_ids(&owner),
            ["acme-2", "acme-1"],
            "acme sees only its own threads, newest activity first"
        );
        assert!(owner["next_cursor"].is_null(), "the store was exhausted");

        let (_, foreign) = list_as(&app, "foreign-key", "?limit=10").await;
        assert_eq!(listed_ids(&foreign), ["globex-2"]);
        assert!(foreign["next_cursor"].is_null());

        let (_, admin) = list_as(&app, "admin-key", "?limit=10").await;
        assert_eq!(
            listed_ids(&admin),
            ["globex-2", "acme-2", "zero-1", "acme-1"],
            "an Admin sees every thread, including the run-less one"
        );
    }

    #[tokio::test]
    async fn list_threads_returns_the_empty_page_shape_when_nothing_is_visible() {
        let fixture = ThreadFixture::new();
        fixture.listed_thread("acme-2", 2, Some("acme"));
        fixture.listed_thread("acme-1", 1, Some("acme"));
        fixture.listed_thread("zero-1", 0, None);
        let app = thread_router(fixture.state(true));

        let (status, body) = list_as(&app, "foreign-key", "").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            body,
            serde_json::json!({ "items": [], "next_cursor": null }),
            "nothing visible is the documented empty page"
        );
    }

    #[tokio::test]
    async fn get_thread_unknown_returns_404() {
        let state = state_with_waypoints(Arc::new(MockWaypointStore::default()));
        let err = get_thread(State(state), admin(), Path("no-such-thread".to_string()))
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_thread_found_returns_waypoint_id_and_active_run() {
        let store = Arc::new(MockWaypointStore::default());
        let t = thread("with-run");
        store.seed_latest(sample_waypoint(&t, 1, WaypointStatus::Running));
        let run_repo = Arc::new(MockRunRepository::default());
        let run = sample_run_for(&t);
        let run_id = run.run_id.to_string();
        run_repo
            .active
            .lock()
            .unwrap()
            .insert(t.as_str().to_string(), run);
        let state = ThreadApiState::new()
            .with_waypoints(store)
            .with_runs(run_repo);

        let (status, Json(body)) = get_thread(State(state), admin(), Path(t.as_str().to_string()))
            .await
            .expect("ok");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["thread_id"], t.as_str());
        assert_eq!(body["active_run_id"], run_id);
        assert!(body["latest_waypoint_id"].is_string());
    }

    // --- fork_thread (D-45, D-46) --------------------------------------

    enum ForkOutcome {
        Accepted,
        ThreadBusy,
        UnknownWaypoint,
        Forbidden,
        AllowanceExhausted,
    }

    /// A window refusal: the key's 2.50 USD daily allowance is spent (Phase 41).
    fn window_allowance_refusal() -> paladin_core::platform::container::allowance::AllowanceRefusal
    {
        use chrono::TimeZone;
        use paladin_core::platform::container::allowance::{
            AllowanceLimitKind, AllowanceRefusal, AllowanceScopeKind,
        };
        use paladin_core::platform::container::cost::{Cost, CurrencyCode};
        let usd = CurrencyCode::new("USD").expect("USD");
        let at = |day, hour| {
            chrono::Utc
                .with_ymd_and_hms(2026, 10, day, hour, 0, 0)
                .single()
                .expect("instant")
        };
        AllowanceRefusal {
            scope_kind: AllowanceScopeKind::ApiKey,
            limit_kind: AllowanceLimitKind::Window,
            balance: Cost::new(2_500_000_000, usd.clone()),
            ceiling: Cost::new(2_500_000_000, usd),
            window: Some((at(3, 0), at(4, 0))),
            evaluated_at: at(3, 23),
        }
    }

    struct MockForkSubmissionPort {
        outcome: ForkOutcome,
    }

    #[async_trait]
    impl RunSubmissionPort for MockForkSubmissionPort {
        async fn submit(
            &self,
            _request: SubmitRun,
        ) -> Result<paladin_ports::input::run_submission_port::RunAccepted, RunSubmissionError>
        {
            Err(RunSubmissionError::NotWired)
        }

        async fn cancel(
            &self,
            _run_id: &RunId,
            _requested_by: Option<paladin_core::platform::container::principal::PrincipalRef>,
        ) -> Result<paladin_ports::input::run_submission_port::CancelOutcome, RunSubmissionError>
        {
            Err(RunSubmissionError::NotWired)
        }

        async fn fork(
            &self,
            request: ForkRun,
        ) -> Result<paladin_ports::input::run_submission_port::RunAccepted, RunSubmissionError>
        {
            match self.outcome {
                ForkOutcome::Accepted => {
                    Ok(paladin_ports::input::run_submission_port::RunAccepted {
                        run_id: RunId::new_v7(),
                        thread_id: request.thread_id,
                    })
                }
                ForkOutcome::ThreadBusy => Err(RunSubmissionError::ThreadBusy {
                    thread_id: request.thread_id,
                }),
                ForkOutcome::UnknownWaypoint => Err(RunSubmissionError::UnknownWaypoint {
                    thread_id: request.thread_id,
                    waypoint_id: request.from_waypoint_id.to_string(),
                }),
                ForkOutcome::Forbidden => Err(RunSubmissionError::Forbidden {
                    reason: "role not permitted for this assistant".to_string(),
                }),
                ForkOutcome::AllowanceExhausted => Err(RunSubmissionError::AllowanceExhausted(
                    window_allowance_refusal(),
                )),
            }
        }
    }

    /// A state with a fork submission double and a seeded latest Waypoint for `name`, so
    /// the visibility gate (TENANT-03) admits the Admin principal these tests call with.
    fn fork_state(outcome: ForkOutcome, name: &str) -> ThreadApiState {
        let store = Arc::new(MockWaypointStore::default());
        store.seed_latest(sample_waypoint(&thread(name), 1, WaypointStatus::Completed));
        ThreadApiState::new()
            .with_waypoints(store)
            .with_run_submission(Arc::new(MockForkSubmissionPort { outcome }))
    }

    fn fork_body() -> ForkThreadRequest {
        ForkThreadRequest {
            from_waypoint_id: WaypointId::generate().to_string(),
            edit: None,
            webhook: None,
        }
    }

    #[tokio::test]
    async fn fork_thread_returns_501_when_unwired() {
        let state = ThreadApiState::new();
        let err = fork_thread(
            State(state),
            admin(),
            Path("t".to_string()),
            Json(fork_body()),
        )
        .await
        .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn fork_thread_accepted_returns_202() {
        let state = fork_state(ForkOutcome::Accepted, "fork-thread");
        let (status, Json(body)) = fork_thread(
            State(state),
            admin(),
            Path("fork-thread".to_string()),
            Json(fork_body()),
        )
        .await
        .expect("accepted");
        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(body["thread_id"], "fork-thread");
        assert!(body["run_id"].is_string());
    }

    #[tokio::test]
    async fn fork_thread_busy_returns_409() {
        let state = fork_state(ForkOutcome::ThreadBusy, "t");
        let err = fork_thread(
            State(state),
            admin(),
            Path("t".to_string()),
            Json(fork_body()),
        )
        .await
        .unwrap_err();
        assert_eq!(err.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn fork_thread_unknown_waypoint_returns_404() {
        let state = fork_state(ForkOutcome::UnknownWaypoint, "t");
        let err = fork_thread(
            State(state),
            admin(),
            Path("t".to_string()),
            Json(fork_body()),
        )
        .await
        .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn fork_thread_forbidden_returns_403() {
        let state = fork_state(ForkOutcome::Forbidden, "t");
        let err = fork_thread(
            State(state),
            admin(),
            Path("t".to_string()),
            Json(fork_body()),
        )
        .await
        .unwrap_err();
        assert_eq!(err.status(), StatusCode::FORBIDDEN);
    }

    /// Phase 41 (D-06, D-12, Pitfall 7): a fork refused by the Treasurer reaches the client as
    /// the same `429 allowance_exhausted` (with `Retry-After`) `POST /v1/runs` answers, through
    /// the shared `map_submission_error`.
    #[tokio::test]
    async fn fork_route_maps_allowance_exhausted_to_429() {
        let state = fork_state(ForkOutcome::AllowanceExhausted, "t");
        let err = fork_thread(
            State(state),
            admin(),
            Path("t".to_string()),
            Json(fork_body()),
        )
        .await
        .unwrap_err();
        assert_eq!(err.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(err.to_body()["error"]["code"], "allowance_exhausted");

        let response = axum::response::IntoResponse::into_response(err);
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            response
                .headers()
                .get("retry-after")
                .and_then(|value| value.to_str().ok()),
            Some("3600"),
            "a window refusal carries Retry-After"
        );
    }

    // --- delete_thread (D-45) -------------------------------------------

    #[tokio::test]
    async fn delete_thread_returns_501_when_unwired() {
        let state = ThreadApiState::new();
        let err = delete_thread(State(state), admin(), Path("t".to_string()))
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn delete_thread_unknown_returns_404() {
        let state = state_with_waypoints(Arc::new(MockWaypointStore::default()));
        let err = delete_thread(State(state), admin(), Path("no-such-thread".to_string()))
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn delete_thread_found_returns_204() {
        let store = Arc::new(MockWaypointStore::default());
        let t = thread("deletable");
        store.seed_latest(sample_waypoint(&t, 1, WaypointStatus::Completed));
        let state = state_with_waypoints(store);
        let status = delete_thread(State(state), admin(), Path(t.as_str().to_string()))
            .await
            .expect("ok");
        assert_eq!(status, StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn delete_thread_busy_returns_409() {
        let store = Arc::new(MockWaypointStore::default());
        let t = thread("busy-thread");
        store.seed_latest(sample_waypoint(&t, 1, WaypointStatus::Running));
        let run_repo = Arc::new(MockRunRepository::default());
        run_repo
            .active
            .lock()
            .unwrap()
            .insert(t.as_str().to_string(), sample_run_for(&t));
        let state = ThreadApiState::new()
            .with_waypoints(store)
            .with_runs(run_repo);
        let err = delete_thread(State(state), admin(), Path(t.as_str().to_string()))
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn delete_thread_non_admin_is_403() {
        // A same-tenant user on a thread its tenant owns passes the visibility gate and
        // is refused by `require_admin` (D-06: 403 only for a caller who can see it).
        let store = Arc::new(MockWaypointStore::default());
        let t = thread("admin-gated");
        store.seed_latest(sample_waypoint(&t, 1, WaypointStatus::Completed));
        let runs = Arc::new(MockRunRepository::default());
        runs.seed_run(run_on_for_tenant(&t, "test-tenant", "u"));
        let state = state_with_waypoints(store).with_runs(runs);
        let user_principal = Extension(Principal::new(
            "u",
            UserRole::User,
            paladin_core::platform::container::principal::TenantId::new("test-tenant").unwrap(),
        ));
        let err = delete_thread(State(state), user_principal, Path(t.as_str().to_string()))
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::FORBIDDEN);
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
