//! Axum HTTP controller for submitting and reading runs (PLAT-01, D-44).
//!
//! This module mirrors [`crate::thread_controller`]'s conventions on a
//! router and state struct of its own, so [`crate::agent_controller::AgentApiState`]
//! and [`crate::thread_controller::ThreadApiState`] stay untouched (X-10.3):
//!
//! | Method & path | Description |
//! |---------------|-------------|
//! | `POST /runs` | Submit a run; `202 Accepted` with `{ run_id, thread_id, state_url }` |
//! | `GET /runs` | Paginated list, filterable by `thread_id`/`assistant_id`/`status` (D-47) |
//! | `GET /runs/{run_id}` | The run's current status |
//! | `GET /runs/{run_id}/stream` | Server-Sent Events stream of the run's progress (PLAT-FR-07, D-24..D-27) |
//! | `POST /runs/{run_id}/cancel` | Idempotent cancel request (D-16); `202` on a non-terminal run, `409` on a terminal one |
//! | `GET /runs/{run_id}/webhook-deliveries` | Paginated delivery attempts, newest-first (D-40, PLAT-FR-14) |
//!
//! [`RunApiState`] holds `Option<Arc<dyn RunSubmissionPort>>`,
//! `Option<Arc<dyn RunRepositoryPort>>`, `Option<Arc<dyn
//! RunEventStreamPort>>` and `Option<Arc<dyn WebhookDeliveryRepositoryPort>>`
//! -- all `paladin-ports` trait objects, never a `paladin-battalion` type --
//! so this crate takes no dependency on `paladin-battalion` in its default
//! build (ADR-0031). When a route's own port is `None`, it answers `501
//! not_implemented` naming the config key to set, per D-44 (the D-24
//! precedent).
//!
//! A success body is the serialized payload; failures use the unified
//! [`ApiError`](crate::error::ApiError) envelope.
//!
//! ## Authorization (D-46)
//!
//! Every route sits behind the same `require_authentication` middleware as
//! `/v1/agents/*` and `/v1/threads/*` (D-44). `POST /runs`, `POST
//! /runs/{run_id}/cancel` and `POST /threads/{id}/fork` are
//! **invocation-shaped**: any authenticated principal, subject to the
//! target assistant's own `allowed_roles` (checked inside
//! `RunSubmissionService`, which is the only layer that ever resolves
//! `allowed_roles` -- `paladin-web` has no visibility into them,
//! ADR-0031). Every read (`GET /runs`, `GET /runs/{run_id}`, `GET
//! /runs/{run_id}/webhook-deliveries`) needs authentication only.
//!
//! ## Read scope (WR-03) -- deployment-wide, not per-caller
//!
//! The three read routes above require only authentication:
//! [`paladin_ports::output::run_repository_port::RunQuery`] carries no
//! caller identity, and neither
//! [`paladin_ports::output::run_repository_port::RunRepositoryPort::list`]/`get`
//! nor
//! [`paladin_ports::output::webhook_delivery_port::WebhookDeliveryRepositoryPort::list_for_run`]
//! applies a requester-derived filter. **Any authenticated principal of any
//! role can list and read every run in the deployment**, including the
//! webhook target URL on `RunResponse::webhook` (secret redacted, URL
//! not), the thread and assistant ids, and the error text of runs it did
//! not submit. `run_id` is a time-ordered UUIDv7 (`RunId::new_v7`), so
//! enumerating the id space by walking `GET /runs` -- or by guessing
//! adjacent, time-clustered ids -- is materially easier than for a random
//! identifier.
//!
//! This is the intended model for v0.10: a **single-tenant or
//! mutually-trusted-principal deployment**, not a multi-tenant one. A
//! per-tenant read scope (filtering `RunQuery`/`get`/`list_for_run` by
//! requester identity) is the remediation, and is tracked as an open item
//! in the project's broken-windows ledger (`.planning/WINDOWS.md`, row 32)
//! rather than left as an implicit assumption.

use std::collections::HashMap;
use std::convert::Infallible;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use axum::Extension;
use axum::extract::{Json, Path, State};
use axum::http::StatusCode;
use axum::response::sse::KeepAlive;
use axum::response::{IntoResponse, Response, Sse, sse::Event};
use chrono::{DateTime, Utc};
use futures::StreamExt;
use serde::{Deserialize, Serialize};

use paladin_core::platform::container::cost::Cost;
use paladin_core::platform::container::principal::PrincipalRef;
use paladin_core::platform::container::run::{
    Run, RunCursor, RunEventKind, RunId, RunStatus, WebhookSpec,
};
use paladin_core::platform::container::treasury_ledger::{
    SpendGroupBy, SpendQuery, SpendRow, format_cost,
};
use paladin_core::platform::container::waypoint::ThreadId;
use paladin_core::platform::container::webhook::{WebhookDeliveryId, WebhookDeliveryStatus};
use paladin_ports::input::run_event_stream_port::{RunEventStreamPort, RunStreamError};
use paladin_ports::input::run_submission_port::{RunSubmissionError, RunSubmissionPort, SubmitRun};
use paladin_ports::output::run_repository_port::{RunPage, RunQuery, RunRepositoryPort};
use paladin_ports::output::treasury_ledger_port::TreasuryLedgerPort;
use paladin_ports::output::webhook_delivery_port::WebhookDeliveryRepositoryPort;

use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::agent_auth::{HasAgentAuth, Principal};
use crate::agent_controller::{API_V1_PREFIX, JsonValue, ok_body};
use crate::error::{ApiError, ApiErrorBody};
use crate::pagination::{PageQuery, decode_cursor, encode_cursor, resolve_limit};

/// The 15s heartbeat interval `GET /runs/{run_id}/stream` keeps alive on
/// both the live and degraded paths (D-26) -- defeats idle proxy timeouts.
pub const RUN_STREAM_HEARTBEAT_SECS: u64 = 15;

/// A boxed SSE event stream, mirroring
/// [`crate::agent_controller`]'s own `SseEventStream` type alias.
type SseEventStream = Pin<Box<dyn futures::Stream<Item = Result<Event, Infallible>> + Send>>;

const SUBMISSION_PORT_HINT: &str =
    "no run submission backend configured: set run_store.backend and run_queue.backend";
const REPOSITORY_PORT_HINT: &str = "no run store backend configured: set run_store.backend";
const RUN_EVENTS_PORT_HINT: &str =
    "no run event stream backend configured: set run_store.backend and run_queue.backend";
const WEBHOOK_DELIVERIES_PORT_HINT: &str =
    "no webhook delivery backend configured: set webhooks.enabled";

/// Shared state for the run routes (D-44).
///
/// Mirrors [`crate::thread_controller::ThreadApiState`]'s injection-only
/// trait-object shape: both fields are `None` until a durable run store and
/// queue are configured, at which point `src/bin/paladin-server.rs` wires
/// both from the facade's `RunSubmissionService`/`RunRepositoryPort`
/// adapter. `#[non_exhaustive]` from its introduction (X-10.3): construct
/// only through [`RunApiState::new`] plus the `with_*` builder methods.
#[derive(Clone)]
#[non_exhaustive]
pub struct RunApiState {
    /// Submits new runs (`POST /runs`). `None` when unwired.
    pub run_submission: Option<Arc<dyn RunSubmissionPort>>,
    /// Reads runs directly (`GET /runs/{run_id}`) -- no engine or queue
    /// dependency, mirroring how thread reads go straight to `WaypointPort`
    /// (D-12). `None` when unwired.
    pub run_repository: Option<Arc<dyn RunRepositoryPort>>,
    /// Opens a run's event stream (`GET /runs/{run_id}/stream`, PLAT-FR-07)
    /// -- names neither the bus, the engine nor `TraceEvent` (D-27). `None`
    /// when unwired.
    pub run_events: Option<Arc<dyn RunEventStreamPort>>,
    /// Authentication configuration -- the SAME [`crate::agent_auth::AgentAuthConfig`]
    /// shape every other stateful router in this crate carries.
    pub auth: crate::agent_auth::AgentAuthConfig,
    /// Validates and publishes assistant definitions (`/assistants*`, PLAT-04, D-31,
    /// D-46). `None` when unwired.
    pub assistants: Option<Arc<dyn paladin_ports::input::assistant_admin_port::AssistantAdminPort>>,
    /// The code-registered agent registry `GET /assistants` merges synthetic entries
    /// from, and every mutating assistant route consults for the `code_registered_immutable`
    /// 409 (D-32). `None` when unwired.
    pub code_registry: Option<Arc<crate::agent_registry::AgentRegistry>>,
    /// Whether `GET /assistants` merges in synthetic code-registry entries (D-32).
    /// Defaults to `true`.
    pub expose_code_registry: bool,
    /// Validates and persists run schedules (`/schedules*`, PLAT-05, D-42, D-46). `None`
    /// when unwired (`schedules.enabled` config gate, D-44/D-50).
    pub schedules: Option<Arc<dyn paladin_ports::input::schedule_admin_port::ScheduleAdminPort>>,
    /// Reads persisted webhook delivery attempts (`GET
    /// /runs/{run_id}/webhook-deliveries`, D-40, PLAT-FR-14). `None` when
    /// unwired.
    pub webhook_deliveries: Option<Arc<dyn WebhookDeliveryRepositoryPort>>,
    /// Reads settled spend for this deployment's runs (D-10, LEDGR-04) --
    /// `RunResponse::cost`/`RunListResponse` items are derived from this
    /// port's `spend` at read time, never persisted on the run row. `None`
    /// when no ledger backend is configured, in which case `cost` is always
    /// `null` and no ledger call is ever made.
    pub treasury_ledger: Option<Arc<dyn TreasuryLedgerPort>>,
}

impl RunApiState {
    /// Construct state with nothing wired (every route answers `501` until
    /// [`Self::with_submission`]/[`Self::with_repository`] are applied).
    pub fn new() -> Self {
        Self {
            run_submission: None,
            run_repository: None,
            run_events: None,
            auth: crate::agent_auth::AgentAuthConfig::default(),
            assistants: None,
            code_registry: None,
            expose_code_registry: true,
            schedules: None,
            webhook_deliveries: None,
            treasury_ledger: None,
        }
    }

    /// Wire a [`WebhookDeliveryRepositoryPort`], enabling `GET
    /// /runs/{run_id}/webhook-deliveries`.
    pub fn with_webhook_deliveries(
        mut self,
        webhook_deliveries: Arc<dyn WebhookDeliveryRepositoryPort>,
    ) -> Self {
        self.webhook_deliveries = Some(webhook_deliveries);
        self
    }

    /// Wire a [`paladin_ports::input::schedule_admin_port::ScheduleAdminPort`], enabling
    /// `/schedules*`.
    pub fn with_schedules(
        mut self,
        schedules: Arc<dyn paladin_ports::input::schedule_admin_port::ScheduleAdminPort>,
    ) -> Self {
        self.schedules = Some(schedules);
        self
    }

    /// Wire an [`paladin_ports::input::assistant_admin_port::AssistantAdminPort`],
    /// enabling `/assistants*`.
    pub fn with_assistants(
        mut self,
        assistants: Arc<dyn paladin_ports::input::assistant_admin_port::AssistantAdminPort>,
    ) -> Self {
        self.assistants = Some(assistants);
        self
    }

    /// Wire the code-registered agent registry (D-32).
    pub fn with_code_registry(
        mut self,
        code_registry: Arc<crate::agent_registry::AgentRegistry>,
    ) -> Self {
        self.code_registry = Some(code_registry);
        self
    }

    /// Override whether `GET /assistants` merges in synthetic code-registry entries
    /// (D-32, default `true`).
    pub fn with_expose_code_registry(mut self, expose: bool) -> Self {
        self.expose_code_registry = expose;
        self
    }

    /// Wire a [`RunSubmissionPort`], enabling `POST /runs`.
    pub fn with_submission(mut self, submission: Arc<dyn RunSubmissionPort>) -> Self {
        self.run_submission = Some(submission);
        self
    }

    /// Wire a [`RunRepositoryPort`], enabling `GET /runs/{run_id}`.
    pub fn with_repository(mut self, repository: Arc<dyn RunRepositoryPort>) -> Self {
        self.run_repository = Some(repository);
        self
    }

    /// Wire a [`RunEventStreamPort`], enabling `GET /runs/{run_id}/stream`.
    pub fn with_run_events(mut self, run_events: Arc<dyn RunEventStreamPort>) -> Self {
        self.run_events = Some(run_events);
        self
    }

    /// Set the authentication configuration.
    pub fn with_auth(mut self, auth: crate::agent_auth::AgentAuthConfig) -> Self {
        self.auth = auth;
        self
    }

    /// Wire a [`TreasuryLedgerPort`], enabling ledger-derived `cost` on `GET
    /// /runs/{run_id}` and `GET /runs` (D-10). Without this, `cost` is
    /// always `null` and no ledger call is ever made.
    pub fn with_treasury_ledger(mut self, treasury_ledger: Arc<dyn TreasuryLedgerPort>) -> Self {
        self.treasury_ledger = Some(treasury_ledger);
        self
    }
}

impl Default for RunApiState {
    fn default() -> Self {
        Self::new()
    }
}

impl HasAgentAuth for RunApiState {
    fn agent_auth(&self) -> &crate::agent_auth::AgentAuthConfig {
        &self.auth
    }
}

// --- DTOs --------------------------------------------------------------

/// Request body for `POST /runs`.
///
/// `input` defaults to an empty object: submit-time validation checks only
/// the assistant reference and this body's shape, never `input`'s own
/// schema -- a schema violation surfaces later as a `Failed` run.
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct SubmitRunRequest {
    /// The assistant to run.
    pub assistant_id: String,
    /// A specific version, or omitted to resolve `latest` at submit time.
    #[serde(default)]
    pub version: Option<u32>,
    /// An existing thread to run against, or omitted to start a fresh one.
    #[serde(default)]
    pub thread_id: Option<String>,
    /// The caller-supplied input.
    #[serde(default)]
    #[schema(value_type = Object)]
    pub input: serde_json::Value,
    /// An optional webhook delivery target for this run's lifecycle events
    /// (D-40) -- validated by the write-time SSRF guard (D-42) before the
    /// run is ever persisted.
    #[serde(default)]
    pub webhook: Option<RunWebhookRequestDto>,
}

/// Response body for a successful `POST /runs` (`202 Accepted`).
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SubmitRunResponse {
    /// The newly created run's identity.
    pub run_id: String,
    /// The thread the run executes against.
    pub thread_id: String,
    /// The URL a client polls (`GET /runs/{run_id}`) to observe status.
    pub state_url: String,
}

/// Wire projection of a [`Cost`] (D-10): the raw, authoritative nano-unit amount plus
/// the same four-decimal-plus-currency rendering the herald and `paladin-cli treasury
/// spend` print ([`format_cost`], D-00c/D-04) -- `display` is derived, never a second
/// source of truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct CostDto {
    /// The exact nano-unit amount. Authoritative; `display` is derived from it.
    pub nanos: i64,
    /// ISO 4217 currency code.
    pub currency: String,
    /// Four-decimal-plus-currency rendering (e.g. `"0.0450 USD"`), byte-identical to
    /// the herald and CLI output for the same amount.
    pub display: String,
}

impl From<&Cost> for CostDto {
    fn from(cost: &Cost) -> Self {
        Self {
            nanos: cost.nanos(),
            currency: cost.currency().as_str().to_string(),
            display: format_cost(cost),
        }
    }
}

/// Response body for `GET /runs/{run_id}`.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct RunResponse {
    /// The run's identity.
    pub run_id: String,
    /// The thread this run executes against.
    pub thread_id: String,
    /// The resolved assistant id.
    pub assistant_id: String,
    /// The resolved, frozen assistant version.
    pub version: u32,
    /// Stable, lowercase status label (the same strings `RunStatus::as_str`
    /// returns).
    pub status: String,
    /// When this run was submitted.
    pub submitted_at: DateTime<Utc>,
    /// When this run entered `Running`, if it has.
    pub started_at: Option<DateTime<Utc>>,
    /// When this run reached a terminal status, if it has.
    pub finished_at: Option<DateTime<Utc>>,
    /// The engine's error, if this run is `Failed`.
    pub error: Option<String>,
    /// The webhook delivery target, if any -- `secret` redacted to `"***"`
    /// (mirrors `schedule_controller::WebhookResponseDto`'s identical
    /// precedent; the raw signing secret is never echoed back).
    pub webhook: Option<RunWebhookDto>,
    /// How many responses are currently parked on this run awaiting worker
    /// consumption -- the COUNT only, never the response values themselves.
    pub pending_responses: usize,
    /// This run's cost, derived from the ledger's settlements for `run_id` at read
    /// time (D-10) -- `null` when no ledger backend is configured, the run has no
    /// settled spend, or its settlements span more than one currency. Never
    /// persisted on the run row; a ledger read failure degrades this to `null`
    /// rather than failing the read (D-08).
    pub cost: Option<CostDto>,
}

impl From<&Run> for RunResponse {
    fn from(run: &Run) -> Self {
        Self {
            run_id: run.run_id.to_string(),
            thread_id: run.thread_id.as_str().to_string(),
            assistant_id: run.assistant.assistant_id.clone(),
            version: run.assistant.version,
            status: run.status.as_str().to_string(),
            submitted_at: run.submitted_at,
            started_at: run.started_at,
            finished_at: run.finished_at,
            error: run.error.clone(),
            webhook: run.webhook.as_ref().map(RunWebhookDto::from),
            pending_responses: run.pending_responses.len(),
            cost: None,
        }
    }
}

/// Wire shape of a webhook delivery target on write (`POST /runs`, `POST
/// /threads/{id}/fork`), mirroring `schedule_controller::WebhookRequestDto`.
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct RunWebhookRequestDto {
    /// The delivery URL -- validated by the write-time SSRF guard (D-42)
    /// before the run is ever persisted.
    pub url: String,
    /// The HMAC signing secret, if any.
    #[serde(default)]
    pub secret: Option<String>,
    /// The lifecycle events this webhook subscribes to (`"awaiting_input"`,
    /// `"completed"`, `"failed"`, `"halted"`, `"cancelled"`).
    #[serde(default)]
    pub events: Vec<String>,
}

/// Wire shape of a webhook delivery target on read -- the SAME shape as
/// [`RunWebhookRequestDto`], except `secret` is redacted (T-27-15-04).
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct RunWebhookDto {
    /// The delivery URL.
    pub url: String,
    /// `"***"` when a secret is set on the run, `null` otherwise -- the raw
    /// secret is never echoed back.
    pub secret: Option<String>,
    /// The lifecycle events this webhook subscribes to.
    pub events: Vec<String>,
}

impl From<&WebhookSpec> for RunWebhookDto {
    fn from(webhook: &WebhookSpec) -> Self {
        Self {
            url: webhook.url.clone(),
            secret: webhook.secret.as_ref().map(|_| "***".to_string()),
            events: webhook
                .events
                .iter()
                .map(|k| event_kind_label(*k))
                .collect(),
        }
    }
}

fn event_kind_label(kind: RunEventKind) -> String {
    match kind {
        RunEventKind::AwaitingInput => "awaiting_input",
        RunEventKind::Completed => "completed",
        RunEventKind::Failed => "failed",
        RunEventKind::Halted => "halted",
        RunEventKind::Cancelled => "cancelled",
    }
    .to_string()
}

fn parse_event_kind(raw: &str) -> Result<RunEventKind, ApiError> {
    match raw {
        "awaiting_input" => Ok(RunEventKind::AwaitingInput),
        "completed" => Ok(RunEventKind::Completed),
        "failed" => Ok(RunEventKind::Failed),
        "halted" => Ok(RunEventKind::Halted),
        "cancelled" => Ok(RunEventKind::Cancelled),
        other => Err(ApiError::bad_request(format!(
            "unknown webhook event kind '{other}': expected one of awaiting_input, completed, \
             failed, halted, cancelled"
        ))),
    }
}

/// Convert a wire [`RunWebhookRequestDto`] into the core [`WebhookSpec`]
/// `RunSubmissionPort::submit`/`fork` consume. `pub(crate)` so
/// `thread_controller::fork_thread` can reuse it verbatim for `POST
/// /threads/{id}/fork`'s own optional `webhook` field, rather than
/// duplicating the event-kind parsing.
pub(crate) fn to_run_webhook_spec(dto: RunWebhookRequestDto) -> Result<WebhookSpec, ApiError> {
    let events = dto
        .events
        .iter()
        .map(|raw| parse_event_kind(raw))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(WebhookSpec {
        url: dto.url,
        secret: dto.secret,
        events,
    })
}

// --- List / cancel / webhook-deliveries DTOs (D-47) -------------------

/// Query parameters for `GET /runs`.
#[derive(Debug, Clone, Deserialize)]
pub struct RunListQuery {
    /// Restrict to a single thread.
    #[serde(default)]
    pub thread_id: Option<String>,
    /// Restrict to a single assistant.
    #[serde(default)]
    pub assistant_id: Option<String>,
    /// Restrict to a single status (`"queued"`, `"running"`,
    /// `"awaiting_input"`, `"completed"`, `"failed"`, `"halted"`,
    /// `"cancelled"`).
    #[serde(default)]
    pub status: Option<String>,
    /// Maximum number of items to return (at most 100).
    #[serde(default)]
    pub limit: Option<u32>,
    /// Opaque pagination cursor from a previous page's `next_cursor`.
    #[serde(default)]
    pub cursor: Option<String>,
}

/// Response body for `GET /runs`.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct RunListResponse {
    /// The page of runs, ordered `(submitted_at DESC, run_id DESC)`. A
    /// cursor walk gives a stable ordering for rows that already existed
    /// when the first page was fetched, but is NOT a point-in-time
    /// snapshot: a row inserted after the first page may be omitted from
    /// the walk (D-47 backstop).
    pub items: Vec<RunResponse>,
    /// Opaque cursor for the next page, `None` on the last page.
    pub next_cursor: Option<String>,
}

/// Response body for a successful `POST /runs/{run_id}/cancel`.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct CancelRunResponse {
    /// The cancelled run's identity.
    pub run_id: String,
    /// The run's status immediately after the durable cancel flag was
    /// written -- always a non-terminal status.
    pub status: String,
    /// Whether THIS process instance held an in-process cancellation
    /// signal for the run and fired it directly (a same-instance fast
    /// path).
    pub was_local: bool,
}

/// Wire projection of one attempt of a
/// [`paladin_core::platform::container::webhook::WebhookDelivery`].
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct WebhookDeliveryDto {
    /// This delivery's identity.
    pub delivery_id: String,
    /// Which lifecycle event this delivery carries.
    pub event: String,
    /// How many attempts have been made so far.
    pub attempt: u32,
    /// This delivery's current status (`"pending"`, `"in_flight"`,
    /// `"delivered"`, `"retrying"`, `"dead"`).
    pub status: String,
    /// When this delivery is next eligible to be claimed, if not yet
    /// terminal.
    pub next_attempt_at: DateTime<Utc>,
    /// The HTTP status the most recent attempt observed, if any.
    pub last_response_status: Option<u16>,
    /// A redacted, bounded diagnostic of the most recent attempt's
    /// failure, if any -- never the payload or a signing secret.
    pub last_error: Option<String>,
    /// When this delivery was enqueued.
    pub created_at: DateTime<Utc>,
}

fn webhook_delivery_status_label(status: WebhookDeliveryStatus) -> String {
    status.as_str().to_string()
}

impl From<&paladin_core::platform::container::webhook::WebhookDelivery> for WebhookDeliveryDto {
    fn from(delivery: &paladin_core::platform::container::webhook::WebhookDelivery) -> Self {
        Self {
            delivery_id: delivery.delivery_id.to_string(),
            event: event_kind_label(delivery.event),
            attempt: delivery.attempt,
            status: webhook_delivery_status_label(delivery.status),
            next_attempt_at: delivery.next_attempt_at,
            last_response_status: delivery.last_response_status,
            last_error: delivery.last_error.clone(),
            created_at: delivery.created_at,
        }
    }
}

/// Response body for `GET /runs/{run_id}/webhook-deliveries`.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct WebhookDeliveryListResponse {
    /// The page of delivery attempts, newest-first.
    pub items: Vec<WebhookDeliveryDto>,
    /// Opaque cursor for the next page, `None` on the last page.
    pub next_cursor: Option<String>,
}

// --- Parsing helpers --------------------------------------------------------

fn parse_run_id(raw: &str) -> Result<RunId, ApiError> {
    RunId::parse(raw).map_err(|e| ApiError::bad_request(e.to_string()))
}

fn parse_thread_id(raw: &str) -> Result<ThreadId, ApiError> {
    ThreadId::new(raw).map_err(|e| ApiError::bad_request(e.to_string()))
}

/// Parse a `status` query parameter into a [`RunStatus`], reusing the
/// type's own `#[serde(rename_all = "snake_case")]` `Deserialize` impl
/// rather than a second, hand-maintained label table.
fn parse_run_status(raw: &str) -> Result<RunStatus, ApiError> {
    serde_json::from_value(serde_json::Value::String(raw.to_string()))
        .map_err(|_| ApiError::bad_request(format!("unknown run status '{raw}'")))
}

// --- Error mapping -----------------------------------------------------

/// Map a [`RunSubmissionError`] onto the [`ApiError`] status/code this
/// route uses. `#[non_exhaustive]`, so a future variant renders `500
/// internal` rather than failing to compile. `pub(crate)` so
/// `thread_controller::fork_thread` reuses it verbatim (`fork` shares the
/// same error enum as `submit`/`cancel`, D-45).
pub(crate) fn map_submission_error(err: RunSubmissionError) -> ApiError {
    match err {
        RunSubmissionError::UnknownAssistant { assistant_id } => {
            ApiError::not_found(format!("unknown assistant '{assistant_id}'"))
        }
        RunSubmissionError::UnknownVersion {
            assistant_id,
            version,
        } => ApiError::not_found(format!(
            "unknown version {version} for assistant '{assistant_id}'"
        )),
        RunSubmissionError::ThreadBusy { thread_id } => ApiError::new(
            StatusCode::CONFLICT,
            "thread_busy",
            format!("thread '{thread_id}' already has an active run -- resume it via POST /threads/{thread_id}/resume"),
        )
        .with_details(serde_json::json!({ "remedy": "resume" })),
        RunSubmissionError::Forbidden { reason } => ApiError::forbidden(reason),
        RunSubmissionError::InvalidInput { message } => ApiError::bad_request(message),
        RunSubmissionError::NotFound { run_id } => {
            ApiError::not_found(format!("unknown run '{run_id}'"))
        }
        RunSubmissionError::AlreadyTerminal { run_id, status } => {
            ApiError::conflict(format!("run '{run_id}' is already terminal ({status})"))
        }
        RunSubmissionError::WebhookRejected { reason } => ApiError::new(
            StatusCode::BAD_REQUEST,
            "webhook_url_rejected",
            format!("webhook URL rejected: {reason}"),
        ),
        RunSubmissionError::UnknownThread { thread_id } => {
            ApiError::not_found(format!("unknown thread '{thread_id}'"))
        }
        RunSubmissionError::UnknownWaypoint {
            thread_id,
            waypoint_id,
        } => ApiError::not_found(format!(
            "unknown waypoint '{waypoint_id}' on thread '{thread_id}'"
        )),
        RunSubmissionError::Backend { message } => ApiError::internal(message),
        RunSubmissionError::NotWired => ApiError::not_implemented(SUBMISSION_PORT_HINT),
        other => ApiError::internal(other.to_string()),
    }
}

// --- Handlers ---------------------------------------------------------------

/// `POST /runs` -- submit a run for execution.
///
/// Returns:
/// - `202 Accepted` with [`SubmitRunResponse`] on success;
/// - `400 Bad Request` for an invalid `thread_id`;
/// - `404 Not Found` for an unknown assistant or version;
/// - `409 Conflict` (`code = "thread_busy"`) if the target thread already
///   has an active run;
/// - `501 Not Implemented` if no run submission backend is configured.
#[utoipa::path(
    post,
    path = "/runs",
    tag = "runs",
    request_body = SubmitRunRequest,
    responses(
        (status = 202, description = "Run accepted", body = SubmitRunResponse),
        (status = 400, description = "Invalid thread id, or webhook_url_rejected (write-time SSRF guard, D-42)", body = ApiErrorBody),
        (status = 401, description = "Missing/invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Role not permitted for this assistant (allowed_roles, D-46)", body = ApiErrorBody),
        (status = 404, description = "Unknown assistant or version", body = ApiErrorBody),
        (status = 409, description = "Thread busy -- the remedy is POST /threads/{id}/resume", body = ApiErrorBody),
        (status = 501, description = "No run submission backend configured", body = ApiErrorBody),
    ),
    security(("api_key" = []), ("bearer_token" = [])),
)]
pub async fn submit_run(
    State(state): State<RunApiState>,
    Extension(principal): Extension<Principal>,
    Json(body): Json<SubmitRunRequest>,
) -> Result<(StatusCode, JsonValue), ApiError> {
    let submission = state
        .run_submission
        .as_ref()
        .ok_or_else(|| ApiError::not_implemented(SUBMISSION_PORT_HINT))?;

    let thread_id = body.thread_id.as_deref().map(parse_thread_id).transpose()?;
    let webhook = body.webhook.map(to_run_webhook_spec).transpose()?;

    let request = SubmitRun {
        assistant_id: body.assistant_id,
        version: body.version,
        thread_id,
        input: body.input,
        webhook,
        requested_by: Some(PrincipalRef::from(&principal)),
    };

    let accepted = submission
        .submit(request)
        .await
        .map_err(map_submission_error)?;
    let run_id = accepted.run_id.to_string();
    let thread_id = accepted.thread_id.as_str().to_string();
    let state_url = format!("{API_V1_PREFIX}/runs/{run_id}");

    Ok((
        StatusCode::ACCEPTED,
        ok_body(&SubmitRunResponse {
            run_id,
            thread_id,
            state_url,
        }),
    ))
}

/// Derive ledger-backed [`CostDto`]s for a batch of run ids in exactly ONE
/// [`TreasuryLedgerPort::spend`] call (D-10, T-39-17) -- `get_run` calls this
/// with one id, `list_runs` with the whole page's ids.
///
/// Returns an empty map (never an error) when `ledger` is `None`, `run_ids`
/// is empty, or the `spend` call itself fails -- a ledger problem degrades
/// the caller's `cost` field to `null`, it never fails the run read (D-08).
/// A run whose settlements span more than one currency is also omitted
/// (T-39-11): `spend` never combines two currencies into one row, and this
/// helper refuses to guess which one to surface, so such a run's entry is
/// simply absent from the returned map.
async fn run_costs(
    ledger: Option<&Arc<dyn TreasuryLedgerPort>>,
    run_ids: Vec<RunId>,
) -> HashMap<String, CostDto> {
    let Some(ledger) = ledger else {
        return HashMap::new();
    };
    if run_ids.is_empty() {
        return HashMap::new();
    }
    let run_count = run_ids.len();

    let rows = match ledger
        .spend(SpendQuery {
            group_by: SpendGroupBy::Run,
            run_ids,
            ..Default::default()
        })
        .await
    {
        Ok(rows) => rows,
        Err(error) => {
            log::warn!(
                "treasury ledger spend query failed for {run_count} run(s), \
                 degrading cost to null: {error}"
            );
            return HashMap::new();
        }
    };

    let mut by_run: HashMap<String, Vec<SpendRow>> = HashMap::new();
    for row in rows {
        by_run.entry(row.group.clone()).or_default().push(row);
    }

    by_run
        .into_iter()
        .filter_map(|(run_id, rows)| match rows.as_slice() {
            [only] => Some((run_id, CostDto::from(&only.amount))),
            _ => None,
        })
        .collect()
}

/// Fetch a run and check the caller's
/// [`RunReadScope`](paladin_core::platform::container::principal::RunReadScope) before
/// returning it -- the single-run
/// half of the one shared run-read authorization function (D-12, D-13).
///
/// A hidden (out-of-scope) run and a genuinely missing run answer with the EXACT SAME
/// [`ApiError::not_found`] (no `403`, no timing/shape difference, PLAT-07): a foreign
/// tenant's run must be indistinguishable from a run that does not exist.
///
/// Every `/runs/{run_id}*` route enters through this helper.
async fn load_visible_run(
    repository: &Arc<dyn RunRepositoryPort>,
    principal: &Principal,
    run_id: &RunId,
) -> Result<Run, ApiError> {
    let run = repository
        .get(run_id)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or_else(|| ApiError::not_found(format!("unknown run '{run_id}'")))?;

    if !principal.read_scope().permits(&run) {
        return Err(ApiError::not_found(format!("unknown run '{run_id}'")));
    }

    Ok(run)
}

/// `GET /runs/{run_id}` -- the run's current status.
///
/// Reads [`RunRepositoryPort`] directly (no submission port, no engine and
/// no queue type named here -- ADR-0031, D-12).
///
/// Returns:
/// - `200 OK` with [`RunResponse`] on success;
/// - `400 Bad Request` for a malformed run id;
/// - `404 Not Found` if no run exists with that id;
/// - `501 Not Implemented` if no run store is configured.
#[utoipa::path(
    get,
    path = "/runs/{run_id}",
    tag = "runs",
    params(("run_id" = String, Path, description = "Run id")),
    responses(
        (status = 200, description = "Run state", body = RunResponse),
        (status = 400, description = "Invalid run id", body = ApiErrorBody),
        (status = 401, description = "Missing/invalid credentials", body = ApiErrorBody),
        (status = 404, description = "Unknown run", body = ApiErrorBody),
        (status = 501, description = "No run store backend configured", body = ApiErrorBody),
    ),
    security(("api_key" = []), ("bearer_token" = [])),
)]
pub async fn get_run(
    State(state): State<RunApiState>,
    Extension(principal): Extension<Principal>,
    Path(run_id): Path<String>,
) -> Result<(StatusCode, JsonValue), ApiError> {
    let repository = state
        .run_repository
        .as_ref()
        .ok_or_else(|| ApiError::not_implemented(REPOSITORY_PORT_HINT))?;
    let id = parse_run_id(&run_id)?;

    let run = load_visible_run(repository, &principal, &id).await?;

    let mut response = RunResponse::from(&run);
    let mut costs = run_costs(state.treasury_ledger.as_ref(), vec![id]).await;
    response.cost = costs.remove(&response.run_id);

    Ok((StatusCode::OK, ok_body(&response)))
}

/// `GET /runs` -- paginated list, filterable by `thread_id`/`assistant_id`/
/// `status` (D-47). Authenticated, any role.
///
/// Ordered `(submitted_at DESC, run_id DESC)`; a cursor walk gives a stable
/// ordering for rows that already existed when the first page was fetched,
/// but is NOT a point-in-time snapshot -- a row inserted after the first
/// page was read may be omitted from the walk (D-47 backstop). An empty
/// result set is `200 { items: [], next_cursor: null }`, never `404`.
///
/// Returns:
/// - `200 OK` with [`RunListResponse`] on success;
/// - `400 Bad Request` for `limit == 0`/`limit > 100`, an unparseable
///   `cursor`, an invalid `thread_id`, or an unknown `status`;
/// - `501 Not Implemented` if no run store backend is configured.
#[utoipa::path(
    get,
    path = "/runs",
    tag = "runs",
    params(
        ("thread_id" = Option<String>, Query, description = "Restrict to a single thread"),
        ("assistant_id" = Option<String>, Query, description = "Restrict to a single assistant"),
        ("status" = Option<String>, Query, description = "Restrict to a single status"),
        ("limit" = Option<u32>, Query, description = "Max items to return, 1..=100 (default 20)"),
        ("cursor" = Option<String>, Query, description = "Opaque pagination cursor from a previous page's next_cursor -- a keyset walk, not a snapshot: rows inserted after the first page was fetched may be omitted"),
    ),
    responses(
        (status = 200, description = "A page of runs; { items: [], next_cursor: null } when empty", body = RunListResponse),
        (status = 400, description = "Invalid limit, cursor, thread_id, or status", body = ApiErrorBody),
        (status = 401, description = "Missing/invalid credentials", body = ApiErrorBody),
        (status = 501, description = "No run store backend configured", body = ApiErrorBody),
    ),
    security(("api_key" = []), ("bearer_token" = [])),
)]
pub async fn list_runs(
    State(state): State<RunApiState>,
    Extension(principal): Extension<Principal>,
    axum::extract::Query(params): axum::extract::Query<RunListQuery>,
) -> Result<(StatusCode, JsonValue), ApiError> {
    let repository = state
        .run_repository
        .as_ref()
        .ok_or_else(|| ApiError::not_implemented(REPOSITORY_PORT_HINT))?;

    let limit = resolve_limit(params.limit)?;
    let thread_id = params
        .thread_id
        .as_deref()
        .map(parse_thread_id)
        .transpose()?;
    let status = params.status.as_deref().map(parse_run_status).transpose()?;
    let cursor = params
        .cursor
        .as_deref()
        .map(decode_cursor::<RunCursor>)
        .transpose()?;

    let page: RunPage = repository
        .list(RunQuery {
            thread_id,
            assistant_id: params.assistant_id,
            status,
            limit,
            cursor,
            // D-12 (list half): the scope comes from the authenticated principal
            // only -- never from a query parameter -- and is applied inside the
            // adapter's own query, not as a post-filter.
            scope: principal.read_scope(),
        })
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let mut items: Vec<RunResponse> = page.items.iter().map(RunResponse::from).collect();
    let page_run_ids: Vec<RunId> = page.items.iter().map(|run| run.run_id.clone()).collect();
    let costs = run_costs(state.treasury_ledger.as_ref(), page_run_ids).await;
    for item in &mut items {
        item.cost = costs.get(&item.run_id).cloned();
    }
    let next_cursor = page.next_cursor.as_ref().map(encode_cursor);
    Ok((
        StatusCode::OK,
        ok_body(&RunListResponse { items, next_cursor }),
    ))
}

/// `POST /runs/{run_id}/cancel` -- idempotently request cancellation
/// (D-16, D-46).
///
/// Invocation-shaped: any authenticated principal, subject to the run's
/// own assistant `allowed_roles`, checked inside
/// [`RunSubmissionPort::cancel`] (`paladin-web` has no visibility into
/// `allowed_roles` itself, ADR-0031).
///
/// Returns:
/// - `202 Accepted` with [`CancelRunResponse`] on a non-terminal run
///   (idempotent -- a second call also answers `202`);
/// - `400 Bad Request` for a malformed run id;
/// - `403 Forbidden` if the principal's role is not permitted;
/// - `404 Not Found` for an unknown run;
/// - `409 Conflict` if the run is already terminal;
/// - `501 Not Implemented` if no run submission backend is configured.
#[utoipa::path(
    post,
    path = "/runs/{run_id}/cancel",
    tag = "runs",
    params(("run_id" = String, Path, description = "Run id")),
    responses(
        (status = 202, description = "Cancel accepted (idempotent on a non-terminal run)", body = CancelRunResponse),
        (status = 400, description = "Invalid run id", body = ApiErrorBody),
        (status = 401, description = "Missing/invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Role not permitted for this run's assistant", body = ApiErrorBody),
        (status = 404, description = "Unknown run", body = ApiErrorBody),
        (status = 409, description = "The run is already terminal", body = ApiErrorBody),
        (status = 501, description = "No run submission backend configured", body = ApiErrorBody),
    ),
    security(("api_key" = []), ("bearer_token" = [])),
)]
pub async fn cancel_run(
    State(state): State<RunApiState>,
    Extension(principal): Extension<Principal>,
    Path(run_id): Path<String>,
) -> Result<(StatusCode, JsonValue), ApiError> {
    let submission = state
        .run_submission
        .as_ref()
        .ok_or_else(|| ApiError::not_implemented(SUBMISSION_PORT_HINT))?;
    let id = parse_run_id(&run_id)?;

    // NOTE (D-13, T-40-07, accepted for this plan): the visibility gate for cancel is wired
    // in 40-05 alongside stream/webhook-deliveries and the full route matrix. This plan only
    // retypes `requested_by` (D-04).
    let outcome = submission
        .cancel(&id, Some(PrincipalRef::from(&principal)))
        .await
        .map_err(map_submission_error)?;

    Ok((
        StatusCode::ACCEPTED,
        ok_body(&CancelRunResponse {
            run_id: outcome.run_id.to_string(),
            status: outcome.status.as_str().to_string(),
            was_local: outcome.was_local,
        }),
    ))
}

/// `GET /runs/{run_id}/webhook-deliveries` -- paginated delivery attempts,
/// newest-first (D-40, PLAT-FR-14). Authenticated, any role.
///
/// Returns:
/// - `200 OK` with [`WebhookDeliveryListResponse`] on success;
/// - `400 Bad Request` for an invalid run id, `limit`, or `cursor`;
/// - `501 Not Implemented` if no webhook delivery backend is configured.
#[utoipa::path(
    get,
    path = "/runs/{run_id}/webhook-deliveries",
    tag = "runs",
    params(
        ("run_id" = String, Path, description = "Run id"),
        ("limit" = Option<u32>, Query, description = "Max items to return, 1..=100 (default 20)"),
        ("cursor" = Option<String>, Query, description = "Opaque pagination cursor from a previous page's next_cursor -- a keyset walk, not a snapshot: deliveries recorded after the first page was fetched may be omitted"),
    ),
    responses(
        (status = 200, description = "A page of delivery attempts, newest-first; { items: [], next_cursor: null } when empty", body = WebhookDeliveryListResponse),
        (status = 400, description = "Invalid run id, limit, or cursor", body = ApiErrorBody),
        (status = 401, description = "Missing/invalid credentials", body = ApiErrorBody),
        (status = 501, description = "No webhook delivery backend configured", body = ApiErrorBody),
    ),
    security(("api_key" = []), ("bearer_token" = [])),
)]
pub async fn list_webhook_deliveries(
    State(state): State<RunApiState>,
    Extension(_principal): Extension<Principal>,
    Path(run_id): Path<String>,
    axum::extract::Query(params): axum::extract::Query<PageQuery>,
) -> Result<(StatusCode, JsonValue), ApiError> {
    let deliveries = state
        .webhook_deliveries
        .as_ref()
        .ok_or_else(|| ApiError::not_implemented(WEBHOOK_DELIVERIES_PORT_HINT))?;
    let id = parse_run_id(&run_id)?;
    let limit = resolve_limit(params.limit)?;
    let cursor = params
        .cursor
        .as_deref()
        .map(decode_cursor::<WebhookDeliveryId>)
        .transpose()?;

    let page = deliveries
        .list_for_run(&id, limit, cursor)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let items: Vec<WebhookDeliveryDto> = page.items.iter().map(WebhookDeliveryDto::from).collect();
    let next_cursor = page.next_cursor.as_ref().map(encode_cursor);
    Ok((
        StatusCode::OK,
        ok_body(&WebhookDeliveryListResponse { items, next_cursor }),
    ))
}

// --- Streaming ----------------------------------------------------------

/// Frame a [`paladin_ports::input::run_event_stream_port::RunEventStream`]
/// as an SSE event stream: each `event:` line is the wire name
/// [`paladin_core::platform::container::run::RunStreamEventKind::as_str`]
/// returns and `data:` is the whole event serialized as JSON (`run_id`,
/// `thread_id`, `kind`, `seq`, `at`, `mode`, `dropped`, `payload`). Names
/// neither the bus, the engine nor `TraceEvent` (D-27) -- this function's
/// only input is the core `RunStreamEvent` type.
fn frame_run_events(
    stream: paladin_ports::input::run_event_stream_port::RunEventStream,
) -> SseEventStream {
    Box::pin(stream.map(|event| {
        let name = event.kind.as_str();
        let data = serde_json::to_string(&event)
            .unwrap_or_else(|_| serde_json::json!({ "error": "serialization_failed" }).to_string());
        Ok(Event::default().event(name).data(data))
    }))
}

/// `GET /runs/{run_id}/stream` -- Server-Sent Events stream of a run's
/// progress (PLAT-FR-07, D-24..D-27).
///
/// Returns:
/// - `200 OK` `text/event-stream` on success, framing the seven wire events
///   this module's own docs table lists;
/// - `400 Bad Request` for a malformed run id;
/// - `404 Not Found` if no run exists with that id;
/// - `501 Not Implemented` if no run event stream backend is configured.
#[utoipa::path(
    get,
    path = "/runs/{run_id}/stream",
    tag = "runs",
    params(("run_id" = String, Path, description = "Run id")),
    responses(
        (status = 200, description = "Server-Sent Events stream of the seven frozen wire \
            events (D-25): `superstep` `{ superstep }`, `node_started` \
            `{ superstep, node_id }`, `node_finished` `{ superstep, node_id, outcome }`, \
            `state_delta` `{ superstep, fields, bytes }` (changed field NAMES and a byte-size \
            count only -- never a value), `parley` `{ waypoint_id, parleys }`, `done` \
            `{ status, waypoint_id }` and `error` `{ status, message, waypoint_id }`. Every \
            event also carries `seq`, `at`, `mode` (`live` or `degraded`) and `dropped`. The \
            degraded path (the run executes on another instance, or is already terminal) \
            gives no ordering guarantee relative to the live path and may coalesce \
            supersteps; `done`/`error` are always eventually delivered on both paths. A 15s \
            heartbeat comment line defeats idle proxy timeouts on both paths.",
            content_type = "text/event-stream"),
        (status = 400, description = "Invalid run id", body = ApiErrorBody),
        (status = 401, description = "Missing/invalid credentials", body = ApiErrorBody),
        (status = 404, description = "Unknown run", body = ApiErrorBody),
        (status = 501, description = "No run event stream backend configured", body = ApiErrorBody),
    ),
    security(("api_key" = []), ("bearer_token" = [])),
)]
pub async fn stream_run(
    State(state): State<RunApiState>,
    Extension(_principal): Extension<Principal>,
    Path(run_id): Path<String>,
) -> Response {
    let Some(run_events) = state.run_events.as_ref() else {
        return ApiError::not_implemented(RUN_EVENTS_PORT_HINT).into_response();
    };
    let id = match parse_run_id(&run_id) {
        Ok(id) => id,
        Err(error) => return error.into_response(),
    };

    match run_events.stream(&id).await {
        Ok(stream) => {
            let boxed = frame_run_events(stream);
            Sse::new(boxed)
                .keep_alive(
                    KeepAlive::new().interval(Duration::from_secs(RUN_STREAM_HEARTBEAT_SECS)),
                )
                .into_response()
        }
        Err(RunStreamError::NotFound { run_id }) => {
            ApiError::not_found(format!("unknown run '{run_id}'")).into_response()
        }
        Err(RunStreamError::NotWired) => {
            ApiError::not_implemented(RUN_EVENTS_PORT_HINT).into_response()
        }
        Err(RunStreamError::Backend { message }) => ApiError::internal(message).into_response(),
        Err(other) => ApiError::internal(other.to_string()).into_response(),
    }
}

// --- Router -----------------------------------------------------------------

/// Build the run API as a `utoipa-axum` [`OpenApiRouter`], mirroring
/// [`crate::thread_controller::thread_openapi_router`]'s composition 1:1:
/// routes declared unprefixed (`/runs...`; the `/v1` segment is added on
/// nesting), the SAME `require_authentication` middleware, and the document
/// assembled from the SAME `#[utoipa::path]` annotations the routes
/// themselves carry.
pub fn run_openapi_router(state: RunApiState) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(submit_run))
        .routes(routes!(list_runs))
        .routes(routes!(get_run))
        .routes(routes!(stream_run))
        .routes(routes!(cancel_run))
        .routes(routes!(list_webhook_deliveries))
        .merge(crate::assistant_controller::assistant_routes())
        .merge(crate::schedule_controller::schedule_routes())
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::agent_auth::require_authentication::<RunApiState>,
        ))
        .with_state(state)
}

/// Assemble the run API nested under [`API_V1_PREFIX`] into an `axum`
/// [`axum::Router`] and its raw OpenAPI document, mirroring
/// [`crate::thread_controller::versioned_thread_parts`].
pub(crate) fn versioned_run_parts(state: RunApiState) -> (axum::Router, utoipa::openapi::OpenApi) {
    OpenApiRouter::new()
        .nest(API_V1_PREFIX, run_openapi_router(state))
        .split_for_parts()
}

/// Build the run router as a plain `axum` [`axum::Router`] (`/v1/runs...`).
/// Merged by `src/bin/paladin-server.rs` alongside the agent/thread
/// routers -- never inside them -- so those states stay untouched (D-44).
pub fn run_router(state: RunApiState) -> axum::Router {
    let (routes, _api) = versioned_run_parts(state);
    routes
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use axum::body::Body;
    use axum::http::Request;
    use paladin_core::platform::container::cost::CurrencyCode;
    use paladin_core::platform::container::parley::ParleyResponse;
    use paladin_core::platform::container::principal::RunReadScope;
    use paladin_core::platform::container::run::{AssistantRef, RunStatus};
    use paladin_core::platform::container::treasury_ledger::{
        ReservationId, ReserveRequest, SettleOutcome, SettleRequest,
    };
    use paladin_ports::output::run_repository_port::{
        RunOutcomeRecord, RunPage, RunQuery, RunRepositoryError,
    };
    use paladin_ports::output::treasury_ledger_port::TreasuryLedgerError;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use tower::ServiceExt; // for `Router::oneshot`

    fn sample_run(thread: &str) -> Run {
        Run::new(
            RunId::new_v7(),
            ThreadId::new(thread).unwrap(),
            AssistantRef {
                assistant_id: "a1".to_string(),
                version: 1,
            },
            serde_json::json!({}),
        )
    }

    // --- Mock `RunSubmissionPort` ---------------------------------------

    enum MockOutcome {
        Accepted,
        ThreadBusy,
        UnknownAssistant,
        NotWired,
        Forbidden,
        AlreadyTerminal,
    }

    struct MockSubmissionPort {
        outcome: MockOutcome,
    }

    #[async_trait]
    impl RunSubmissionPort for MockSubmissionPort {
        async fn submit(
            &self,
            request: SubmitRun,
        ) -> Result<paladin_ports::input::run_submission_port::RunAccepted, RunSubmissionError>
        {
            match self.outcome {
                MockOutcome::Accepted => {
                    Ok(paladin_ports::input::run_submission_port::RunAccepted {
                        run_id: RunId::new_v7(),
                        thread_id: request
                            .thread_id
                            .unwrap_or_else(|| ThreadId::new("generated-thread").unwrap()),
                    })
                }
                MockOutcome::ThreadBusy => Err(RunSubmissionError::ThreadBusy {
                    thread_id: request
                        .thread_id
                        .unwrap_or_else(|| ThreadId::new("t").unwrap()),
                }),
                MockOutcome::UnknownAssistant => Err(RunSubmissionError::UnknownAssistant {
                    assistant_id: request.assistant_id,
                }),
                MockOutcome::NotWired => Err(RunSubmissionError::NotWired),
                MockOutcome::Forbidden => Err(RunSubmissionError::Forbidden {
                    reason: "role not permitted for this assistant".to_string(),
                }),
                MockOutcome::AlreadyTerminal => Err(RunSubmissionError::AlreadyTerminal {
                    run_id: RunId::new_v7(),
                    status: paladin_core::platform::container::run::RunStatus::Completed,
                }),
            }
        }

        async fn cancel(
            &self,
            run_id: &RunId,
            _requested_by: Option<PrincipalRef>,
        ) -> Result<paladin_ports::input::run_submission_port::CancelOutcome, RunSubmissionError>
        {
            match self.outcome {
                MockOutcome::NotWired => Err(RunSubmissionError::NotWired),
                MockOutcome::Forbidden => Err(RunSubmissionError::Forbidden {
                    reason: "role not permitted for this run's assistant".to_string(),
                }),
                MockOutcome::AlreadyTerminal => Err(RunSubmissionError::AlreadyTerminal {
                    run_id: run_id.clone(),
                    status: paladin_core::platform::container::run::RunStatus::Completed,
                }),
                _ => Ok(paladin_ports::input::run_submission_port::CancelOutcome {
                    run_id: run_id.clone(),
                    status: paladin_core::platform::container::run::RunStatus::Running,
                    was_local: false,
                }),
            }
        }

        async fn fork(
            &self,
            request: paladin_ports::input::run_submission_port::ForkRun,
        ) -> Result<paladin_ports::input::run_submission_port::RunAccepted, RunSubmissionError>
        {
            match self.outcome {
                MockOutcome::NotWired => Err(RunSubmissionError::NotWired),
                _ => Ok(paladin_ports::input::run_submission_port::RunAccepted {
                    run_id: RunId::new_v7(),
                    thread_id: request.thread_id,
                }),
            }
        }
    }

    // --- Mock `RunRepositoryPort` ----------------------------------------

    #[derive(Default)]
    struct MockRepository {
        runs: Mutex<HashMap<String, Run>>,
        /// The page `list` returns; test-only -- the real adapters compute a page from
        /// `RunQuery`, which this double ignores entirely.
        list_items: Mutex<Vec<Run>>,
        /// Every `RunQuery` handed to `list`, in call order, so a test can assert on
        /// what the handler asked the repository for (e.g. the D-12 `scope`).
        recorded_queries: Mutex<Vec<RunQuery>>,
    }

    impl MockRepository {
        fn seed(&self, run: Run) {
            self.runs
                .lock()
                .unwrap()
                .insert(run.run_id.as_str().to_string(), run);
        }

        fn set_list_page(&self, items: Vec<Run>) {
            *self.list_items.lock().unwrap() = items;
        }

        fn last_list_query(&self) -> Option<RunQuery> {
            self.recorded_queries.lock().unwrap().last().cloned()
        }
    }

    #[async_trait]
    impl RunRepositoryPort for MockRepository {
        async fn insert(&self, run: &Run) -> Result<(), RunRepositoryError> {
            self.seed(run.clone());
            Ok(())
        }

        async fn get(&self, run_id: &RunId) -> Result<Option<Run>, RunRepositoryError> {
            Ok(self.runs.lock().unwrap().get(run_id.as_str()).cloned())
        }

        async fn update_status(
            &self,
            _run_id: &RunId,
            _from: RunStatus,
            _to: RunStatus,
            _at: DateTime<Utc>,
        ) -> Result<(), RunRepositoryError> {
            Ok(())
        }

        async fn record_outcome(
            &self,
            _run_id: &RunId,
            _outcome: RunOutcomeRecord,
        ) -> Result<(), RunRepositoryError> {
            Ok(())
        }

        async fn list(&self, query: RunQuery) -> Result<RunPage, RunRepositoryError> {
            self.recorded_queries.lock().unwrap().push(query);
            Ok(RunPage {
                items: self.list_items.lock().unwrap().clone(),
                next_cursor: None,
            })
        }

        async fn active_run_for_thread(
            &self,
            _thread_id: &ThreadId,
        ) -> Result<Option<Run>, RunRepositoryError> {
            Ok(None)
        }

        async fn request_cancel(&self, run_id: &RunId) -> Result<RunStatus, RunRepositoryError> {
            Err(RunRepositoryError::NotFound {
                run_id: run_id.clone(),
            })
        }

        async fn is_cancel_requested(
            &self,
            _thread_id: &ThreadId,
        ) -> Result<bool, RunRepositoryError> {
            Ok(false)
        }

        async fn bump_attempt(&self, _run_id: &RunId) -> Result<u32, RunRepositoryError> {
            Ok(1)
        }

        async fn record_resume(
            &self,
            _run_id: &RunId,
            _responses: Vec<ParleyResponse>,
        ) -> Result<u32, RunRepositoryError> {
            Ok(1)
        }

        async fn clear_pending_responses(&self, _run_id: &RunId) -> Result<(), RunRepositoryError> {
            Ok(())
        }
    }

    // --- Mock `TreasuryLedgerPort` (D-10) --------------------------------
    //
    // `paladin-web` has no dependency on `paladin-storage`'s real adapters (verified by
    // `grep -c 'paladin-storage' crates/paladin-web/Cargo.toml` printing `0`), so `cost`
    // is proven entirely against this local double. `spend` returns a canned outcome
    // and counts/records every call; every other method is never exercised by
    // `get_run`/`list_runs` and returns an inert `Err` so a stray call would fail loudly.

    /// What `StubTreasuryLedger::spend` answers.
    enum StubSpendOutcome {
        Rows(Vec<SpendRow>),
        Err,
    }

    struct StubTreasuryLedger {
        outcome: StubSpendOutcome,
        calls: Mutex<Vec<SpendQuery>>,
    }

    impl StubTreasuryLedger {
        fn rows(rows: Vec<SpendRow>) -> Self {
            Self {
                outcome: StubSpendOutcome::Rows(rows),
                calls: Mutex::new(Vec::new()),
            }
        }

        fn erroring() -> Self {
            Self {
                outcome: StubSpendOutcome::Err,
                calls: Mutex::new(Vec::new()),
            }
        }

        fn call_count(&self) -> usize {
            self.calls.lock().unwrap().len()
        }
    }

    #[async_trait]
    impl TreasuryLedgerPort for StubTreasuryLedger {
        async fn reserve(
            &self,
            _request: ReserveRequest,
        ) -> Result<ReservationId, TreasuryLedgerError> {
            Err(TreasuryLedgerError::InvalidRequest {
                message: "StubTreasuryLedger does not implement reserve".to_string(),
            })
        }

        async fn release(&self, _reservation: ReservationId) -> Result<(), TreasuryLedgerError> {
            Err(TreasuryLedgerError::InvalidRequest {
                message: "StubTreasuryLedger does not implement release".to_string(),
            })
        }

        async fn settle(
            &self,
            _request: SettleRequest,
        ) -> Result<SettleOutcome, TreasuryLedgerError> {
            Err(TreasuryLedgerError::InvalidRequest {
                message: "StubTreasuryLedger does not implement settle".to_string(),
            })
        }

        async fn spend(&self, query: SpendQuery) -> Result<Vec<SpendRow>, TreasuryLedgerError> {
            self.calls.lock().unwrap().push(query);
            match &self.outcome {
                StubSpendOutcome::Rows(rows) => Ok(rows.clone()),
                StubSpendOutcome::Err => Err(TreasuryLedgerError::Backend {
                    source: "stub ledger backend failure".into(),
                }),
            }
        }

        async fn store_now(&self) -> Result<DateTime<Utc>, TreasuryLedgerError> {
            Ok(Utc::now())
        }
    }

    #[tokio::test]
    async fn get_run_includes_ledger_cost() {
        let repository = Arc::new(MockRepository::default());
        let run = sample_run("t-cost");
        repository.seed(run.clone());
        let usd = CurrencyCode::new("USD").unwrap();
        let ledger = Arc::new(StubTreasuryLedger::rows(vec![SpendRow {
            group: run.run_id.to_string(),
            amount: Cost::new(45_000_000, usd),
            settlements: 2,
        }]));
        let state = RunApiState::new()
            .with_repository(repository)
            .with_treasury_ledger(ledger);

        let (status, Json(body)) = get_run(
            State(state),
            tester_principal(),
            Path(run.run_id.to_string()),
        )
        .await
        .expect("ok");

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["cost"]["nanos"], 45_000_000);
        assert_eq!(body["cost"]["currency"], "USD");
        assert_eq!(body["cost"]["display"], "0.0450 USD");
    }

    #[tokio::test]
    async fn get_run_cost_is_null_without_a_ledger() {
        let repository = Arc::new(MockRepository::default());
        let run = sample_run("t-no-ledger");
        repository.seed(run.clone());
        let state = RunApiState::new().with_repository(repository);

        let (status, Json(body)) = get_run(
            State(state),
            tester_principal(),
            Path(run.run_id.to_string()),
        )
        .await
        .expect("ok");

        assert_eq!(status, StatusCode::OK);
        assert!(body["cost"].is_null());
        // Every other field is unchanged.
        assert_eq!(body["run_id"], run.run_id.to_string());
        assert_eq!(body["status"], run.status.as_str());
    }

    #[tokio::test]
    async fn get_run_cost_is_null_when_the_ledger_errors() {
        let repository = Arc::new(MockRepository::default());
        let run = sample_run("t-ledger-err");
        repository.seed(run.clone());
        let ledger = Arc::new(StubTreasuryLedger::erroring());
        let state = RunApiState::new()
            .with_repository(repository)
            .with_treasury_ledger(ledger);

        let (status, Json(body)) = get_run(
            State(state),
            tester_principal(),
            Path(run.run_id.to_string()),
        )
        .await
        .expect("ok");

        assert_eq!(status, StatusCode::OK);
        assert!(body["cost"].is_null());
    }

    #[tokio::test]
    async fn get_run_cost_is_null_for_mixed_currencies() {
        let repository = Arc::new(MockRepository::default());
        let run = sample_run("t-mixed");
        repository.seed(run.clone());
        let usd = CurrencyCode::new("USD").unwrap();
        let eur = CurrencyCode::new("EUR").unwrap();
        let ledger = Arc::new(StubTreasuryLedger::rows(vec![
            SpendRow {
                group: run.run_id.to_string(),
                amount: Cost::new(1_000_000, usd),
                settlements: 1,
            },
            SpendRow {
                group: run.run_id.to_string(),
                amount: Cost::new(2_000_000, eur),
                settlements: 1,
            },
        ]));
        let state = RunApiState::new()
            .with_repository(repository)
            .with_treasury_ledger(ledger);

        let (status, Json(body)) = get_run(
            State(state),
            tester_principal(),
            Path(run.run_id.to_string()),
        )
        .await
        .expect("ok");

        assert_eq!(status, StatusCode::OK);
        assert!(body["cost"].is_null());
    }

    #[tokio::test]
    async fn list_runs_derives_costs_with_one_spend_call_per_page() {
        let repository = Arc::new(MockRepository::default());
        let run_a = sample_run("t-list-a");
        let run_b = sample_run("t-list-b");
        let run_c = sample_run("t-list-c");
        repository.set_list_page(vec![run_a.clone(), run_b.clone(), run_c.clone()]);
        let usd = CurrencyCode::new("USD").unwrap();
        let ledger = Arc::new(StubTreasuryLedger::rows(vec![SpendRow {
            group: run_a.run_id.to_string(),
            amount: Cost::new(10_000_000, usd),
            settlements: 1,
        }]));
        let state = RunApiState::new()
            .with_repository(repository)
            .with_treasury_ledger(Arc::clone(&ledger) as Arc<dyn TreasuryLedgerPort>);

        let (status, Json(body)) = list_runs(
            State(state),
            tester_principal(),
            axum::extract::Query(RunListQuery {
                thread_id: None,
                assistant_id: None,
                status: None,
                limit: None,
                cursor: None,
            }),
        )
        .await
        .expect("ok");

        assert_eq!(status, StatusCode::OK);
        assert_eq!(ledger.call_count(), 1, "exactly one spend call per page");

        {
            let recorded = ledger.calls.lock().unwrap();
            let query = recorded.first().expect("one spend call was recorded");
            assert_eq!(query.group_by, SpendGroupBy::Run);
            let mut expected_ids: Vec<String> = vec![
                run_a.run_id.to_string(),
                run_b.run_id.to_string(),
                run_c.run_id.to_string(),
            ];
            expected_ids.sort();
            let mut actual_ids: Vec<String> =
                query.run_ids.iter().map(|id| id.to_string()).collect();
            actual_ids.sort();
            assert_eq!(actual_ids, expected_ids);
        }

        let items = body["items"].as_array().unwrap();
        assert_eq!(items.len(), 3);
        let find = |id: &str| items.iter().find(|item| item["run_id"] == id).unwrap();
        assert_eq!(find(&run_a.run_id.to_string())["cost"]["nanos"], 10_000_000);
        assert!(find(&run_b.run_id.to_string())["cost"].is_null());
        assert!(find(&run_c.run_id.to_string())["cost"].is_null());
    }

    #[tokio::test]
    async fn post_runs_returns_501_when_unwired() {
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
    }

    #[tokio::test]
    async fn get_run_returns_501_when_unwired() {
        let state = RunApiState::new();
        let app = run_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/runs/{}", RunId::new_v7()))
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn post_runs_accepted_returns_202() {
        let state = RunApiState::new().with_submission(Arc::new(MockSubmissionPort {
            outcome: MockOutcome::Accepted,
        }));
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
        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }

    #[tokio::test]
    async fn post_runs_thread_busy_returns_409() {
        let state = RunApiState::new().with_submission(Arc::new(MockSubmissionPort {
            outcome: MockOutcome::ThreadBusy,
        }));
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
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn post_runs_unknown_assistant_returns_404() {
        let state = RunApiState::new().with_submission(Arc::new(MockSubmissionPort {
            outcome: MockOutcome::UnknownAssistant,
        }));
        let app = run_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/runs")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&serde_json::json!({ "assistant_id": "nope" })).unwrap(),
                    ))
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn post_runs_not_wired_submission_returns_501() {
        let state = RunApiState::new().with_submission(Arc::new(MockSubmissionPort {
            outcome: MockOutcome::NotWired,
        }));
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
    }

    #[tokio::test]
    async fn get_run_unknown_returns_404() {
        let repository = Arc::new(MockRepository::default());
        let state = RunApiState::new().with_repository(repository);
        let app = run_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/runs/{}", RunId::new_v7()))
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_run_found_returns_200_with_status() {
        let repository = Arc::new(MockRepository::default());
        let run = sample_run("t1");
        repository.seed(run.clone());
        let state = RunApiState::new().with_repository(repository);
        let app = run_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/runs/{}", run.run_id))
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn run_routes_require_authentication() {
        let auth = crate::agent_auth::AgentAuthConfig {
            enabled: true,
            api_keys: HashMap::new(),
            token_verifier: None,
            bearer_tenant: None,
        };
        let state = RunApiState::new().with_auth(auth);
        let app = run_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/runs/{}", RunId::new_v7()))
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn run_openapi_router_contains_run_paths() {
        let state = RunApiState::new();
        let (_router, api) = run_openapi_router(state).split_for_parts();
        for expected in [
            "/runs",
            "/runs/{run_id}",
            "/runs/{run_id}/stream",
            "/runs/{run_id}/cancel",
            "/runs/{run_id}/webhook-deliveries",
        ] {
            assert!(
                api.paths.paths.contains_key(expected),
                "missing path {expected}: {:?}",
                api.paths.paths.keys().collect::<Vec<_>>()
            );
        }
    }

    // --- stream_run (PLAT-FR-07, D-24..D-27) --------------------------------

    struct MockRunEventStreamPort {
        events: Vec<paladin_core::platform::container::run::RunStreamEvent>,
    }

    #[async_trait]
    impl RunEventStreamPort for MockRunEventStreamPort {
        async fn stream(
            &self,
            _run_id: &RunId,
        ) -> Result<paladin_ports::input::run_event_stream_port::RunEventStream, RunStreamError>
        {
            Ok(Box::pin(futures::stream::iter(self.events.clone())))
        }
    }

    struct AlwaysNotFound;

    #[async_trait]
    impl RunEventStreamPort for AlwaysNotFound {
        async fn stream(
            &self,
            run_id: &RunId,
        ) -> Result<paladin_ports::input::run_event_stream_port::RunEventStream, RunStreamError>
        {
            Err(RunStreamError::NotFound {
                run_id: run_id.clone(),
            })
        }
    }

    fn tester_principal() -> Extension<Principal> {
        Extension(Principal::new(
            "tester",
            paladin_core::platform::container::user::UserRole::Admin,
            paladin_core::platform::container::principal::TenantId::new("tester-tenant").unwrap(),
        ))
    }

    fn sample_stream_event(
        run_id: &RunId,
        kind: paladin_core::platform::container::run::RunStreamEventKind,
        payload: serde_json::Value,
    ) -> paladin_core::platform::container::run::RunStreamEvent {
        paladin_core::platform::container::run::RunStreamEvent::new(
            run_id.clone(),
            ThreadId::new("t-stream").unwrap(),
            kind,
            1,
            paladin_core::platform::container::run::RunStreamMode::Live,
            0,
            payload,
        )
    }

    async fn read_response_body(response: Response) -> String {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        String::from_utf8(bytes.to_vec()).expect("utf8 body")
    }

    #[tokio::test]
    async fn run_stream_sse() {
        use paladin_core::platform::container::run::RunStreamEventKind;

        let run_id = RunId::new_v7();
        let events = vec![
            sample_stream_event(
                &run_id,
                RunStreamEventKind::Superstep,
                serde_json::json!({ "superstep": 1 }),
            ),
            sample_stream_event(
                &run_id,
                RunStreamEventKind::Parley,
                serde_json::json!({ "waypoint_id": "wp-1", "parleys": [] }),
            ),
            sample_stream_event(
                &run_id,
                RunStreamEventKind::Done,
                serde_json::json!({ "status": "completed", "waypoint_id": "wp-1" }),
            ),
        ];
        let state = RunApiState::new().with_run_events(Arc::new(MockRunEventStreamPort { events }));

        let response = stream_run(State(state), tester_principal(), Path(run_id.to_string())).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body = read_response_body(response).await;
        let superstep_pos = body
            .find("event: superstep")
            .expect("superstep event present");
        let parley_pos = body.find("event: parley").expect("parley event present");
        let done_pos = body.find("event: done").expect("done event present");
        assert!(
            superstep_pos < parley_pos && parley_pos < done_pos,
            "events must appear in order: {body}"
        );
    }

    #[tokio::test]
    async fn run_stream_returns_501_when_unwired() {
        let state = RunApiState::new();
        let response = stream_run(
            State(state),
            tester_principal(),
            Path(RunId::new_v7().to_string()),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn run_stream_unknown_run_returns_404() {
        let state = RunApiState::new().with_run_events(Arc::new(AlwaysNotFound));
        let response = stream_run(
            State(state),
            tester_principal(),
            Path(RunId::new_v7().to_string()),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    /// D-26: the 15s heartbeat interval is asserted directly on the
    /// constant and the builder call this handler makes -- a real 15s wait
    /// is not acceptable in CI.
    #[test]
    fn run_stream_heartbeat_interval_is_15_seconds() {
        assert_eq!(RUN_STREAM_HEARTBEAT_SECS, 15);
        let _ = KeepAlive::new().interval(Duration::from_secs(RUN_STREAM_HEARTBEAT_SECS));
    }

    // --- list_runs (D-47) -------------------------------------------------

    #[tokio::test]
    async fn list_runs_returns_501_when_unwired() {
        let state = RunApiState::new();
        let app = run_router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/runs")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn list_runs_empty_is_200_with_empty_items_and_null_cursor() {
        let repository = Arc::new(MockRepository::default());
        let state = RunApiState::new().with_repository(repository);
        let (status, Json(body)) = list_runs(
            State(state),
            tester_principal(),
            axum::extract::Query(RunListQuery {
                thread_id: None,
                assistant_id: None,
                status: None,
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

    /// D-12 (list half): `list_runs` derives `RunQuery.scope` from the caller's
    /// principal -- a `User` in tenant `acme` asks for `Tenant(acme)`, an `Admin`
    /// asks for `All` -- and never from anything on the request.
    #[tokio::test]
    async fn list_runs_passes_the_callers_read_scope() {
        let repository = Arc::new(MockRepository::default());
        let state = RunApiState::new().with_repository(repository.clone());
        let acme = paladin_core::platform::container::principal::TenantId::new("acme").unwrap();
        let empty_query = || {
            axum::extract::Query(RunListQuery {
                thread_id: None,
                assistant_id: None,
                status: None,
                limit: None,
                cursor: None,
            })
        };

        let user = Extension(Principal::new(
            "svc-a",
            paladin_core::platform::container::user::UserRole::User,
            acme.clone(),
        ));
        let (status, _) = list_runs(State(state.clone()), user, empty_query())
            .await
            .expect("ok");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            repository.last_list_query().map(|q| q.scope),
            Some(RunReadScope::Tenant(acme)),
            "a user-role principal lists only its own tenant"
        );

        let (status, _) = list_runs(State(state), tester_principal(), empty_query())
            .await
            .expect("ok");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            repository.last_list_query().map(|q| q.scope),
            Some(RunReadScope::All),
            "an Admin principal lists every run"
        );
    }

    /// D-02/D-12: a `tenant_id` query parameter is not a filter and not an
    /// override -- an `acme` user still asks the repository for `Tenant(acme)`
    /// when the URL says `?tenant_id=globex`.
    #[tokio::test]
    async fn list_runs_ignores_a_tenant_query_parameter() {
        let repository = Arc::new(MockRepository::default());
        let mut api_keys = HashMap::new();
        api_keys.insert(
            "svc-a-key".to_string(),
            Principal::new(
                "svc-a",
                paladin_core::platform::container::user::UserRole::User,
                paladin_core::platform::container::principal::TenantId::new("acme").unwrap(),
            ),
        );
        let auth = crate::agent_auth::AgentAuthConfig {
            enabled: true,
            api_keys,
            token_verifier: None,
            bearer_tenant: None,
        };
        let state = RunApiState::new()
            .with_repository(repository.clone())
            .with_auth(auth);
        let app = run_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/runs?tenant_id=globex")
                    .header("x-api-key", "svc-a-key")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            repository.last_list_query().map(|q| q.scope),
            Some(RunReadScope::Tenant(
                paladin_core::platform::container::principal::TenantId::new("acme").unwrap()
            )),
            "?tenant_id is ignored: the scope still comes from the principal"
        );
    }

    #[tokio::test]
    async fn list_runs_rejects_limit_zero_and_101() {
        let repository = Arc::new(MockRepository::default());
        for bad_limit in [Some(0), Some(101)] {
            let state = RunApiState::new().with_repository(repository.clone());
            let err = list_runs(
                State(state),
                tester_principal(),
                axum::extract::Query(RunListQuery {
                    thread_id: None,
                    assistant_id: None,
                    status: None,
                    limit: bad_limit,
                    cursor: None,
                }),
            )
            .await
            .unwrap_err();
            assert_eq!(err.status(), StatusCode::BAD_REQUEST);
        }
    }

    #[tokio::test]
    async fn list_runs_rejects_malformed_cursor_as_400_never_500() {
        let repository = Arc::new(MockRepository::default());
        let state = RunApiState::new().with_repository(repository);
        let err = list_runs(
            State(state),
            tester_principal(),
            axum::extract::Query(RunListQuery {
                thread_id: None,
                assistant_id: None,
                status: None,
                limit: None,
                cursor: Some("not-a-valid-cursor".to_string()),
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
        assert_eq!(err.to_body()["error"]["code"], "invalid_cursor");
    }

    #[tokio::test]
    async fn list_runs_rejects_unknown_status() {
        let repository = Arc::new(MockRepository::default());
        let state = RunApiState::new().with_repository(repository);
        let err = list_runs(
            State(state),
            tester_principal(),
            axum::extract::Query(RunListQuery {
                thread_id: None,
                assistant_id: None,
                status: Some("not-a-status".to_string()),
                limit: None,
                cursor: None,
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
    }

    // --- cancel_run (D-16, D-46) -------------------------------------------

    #[tokio::test]
    async fn cancel_run_accepted_returns_202() {
        let state = RunApiState::new().with_submission(Arc::new(MockSubmissionPort {
            outcome: MockOutcome::Accepted,
        }));
        let app = run_router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/runs/{}/cancel", RunId::new_v7()))
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }

    #[tokio::test]
    async fn cancel_run_already_terminal_returns_409() {
        let state = RunApiState::new().with_submission(Arc::new(MockSubmissionPort {
            outcome: MockOutcome::AlreadyTerminal,
        }));
        let app = run_router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/runs/{}/cancel", RunId::new_v7()))
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn cancel_run_returns_501_when_unwired() {
        let state = RunApiState::new();
        let app = run_router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/runs/{}/cancel", RunId::new_v7()))
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    }

    // --- list_webhook_deliveries (D-40, PLAT-FR-14) ------------------------

    #[derive(Default)]
    struct MockWebhookDeliveries {
        items: Mutex<Vec<paladin_core::platform::container::webhook::WebhookDelivery>>,
    }

    #[async_trait]
    impl WebhookDeliveryRepositoryPort for MockWebhookDeliveries {
        async fn enqueue(
            &self,
            delivery: paladin_core::platform::container::webhook::WebhookDelivery,
        ) -> Result<(), paladin_ports::output::webhook_delivery_port::WebhookDeliveryRepositoryError>
        {
            self.items.lock().unwrap().push(delivery);
            Ok(())
        }

        async fn get(
            &self,
            delivery_id: &WebhookDeliveryId,
        ) -> Result<
            Option<paladin_core::platform::container::webhook::WebhookDelivery>,
            paladin_ports::output::webhook_delivery_port::WebhookDeliveryRepositoryError,
        > {
            Ok(self
                .items
                .lock()
                .unwrap()
                .iter()
                .find(|d| &d.delivery_id == delivery_id)
                .cloned())
        }

        async fn claim_due(
            &self,
            _now: DateTime<Utc>,
            _limit: u32,
        ) -> Result<
            Vec<paladin_core::platform::container::webhook::WebhookDelivery>,
            paladin_ports::output::webhook_delivery_port::WebhookDeliveryRepositoryError,
        > {
            Ok(vec![])
        }

        async fn record_attempt(
            &self,
            _delivery_id: &WebhookDeliveryId,
            _result: paladin_core::platform::container::webhook::WebhookAttemptResult,
        ) -> Result<(), paladin_ports::output::webhook_delivery_port::WebhookDeliveryRepositoryError>
        {
            Ok(())
        }

        async fn list_for_run(
            &self,
            run_id: &RunId,
            limit: u32,
            _cursor: Option<WebhookDeliveryId>,
        ) -> Result<
            paladin_ports::output::webhook_delivery_port::WebhookDeliveryPage,
            paladin_ports::output::webhook_delivery_port::WebhookDeliveryRepositoryError,
        > {
            let items: Vec<_> = self
                .items
                .lock()
                .unwrap()
                .iter()
                .filter(|d| &d.run_id == run_id)
                .take(limit as usize)
                .cloned()
                .collect();
            Ok(
                paladin_ports::output::webhook_delivery_port::WebhookDeliveryPage {
                    items,
                    next_cursor: None,
                },
            )
        }
    }

    #[tokio::test]
    async fn list_webhook_deliveries_returns_501_when_unwired() {
        let state = RunApiState::new();
        let app = run_router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/runs/{}/webhook-deliveries", RunId::new_v7()))
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn list_webhook_deliveries_empty_is_200_with_empty_items() {
        let run_id = RunId::new_v7();
        let state =
            RunApiState::new().with_webhook_deliveries(Arc::new(MockWebhookDeliveries::default()));
        let (status, Json(body)) = list_webhook_deliveries(
            State(state),
            tester_principal(),
            Path(run_id.to_string()),
            axum::extract::Query(PageQuery {
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
    async fn list_webhook_deliveries_no_secret_in_body() {
        let run_id = RunId::new_v7();
        let repo = Arc::new(MockWebhookDeliveries::default());
        let delivery = paladin_core::platform::container::webhook::WebhookDelivery::new(
            WebhookDeliveryId::new_v7(),
            run_id.clone(),
            ThreadId::new("t-wh").unwrap(),
            RunEventKind::Completed,
            "https://example.com/hook",
            r#"{"run_id":"x"}"#,
            Utc::now(),
        );
        repo.items.lock().unwrap().push(delivery);
        let state = RunApiState::new().with_webhook_deliveries(repo);
        let (status, Json(body)) = list_webhook_deliveries(
            State(state),
            tester_principal(),
            Path(run_id.to_string()),
            axum::extract::Query(PageQuery {
                limit: None,
                cursor: None,
            }),
        )
        .await
        .expect("ok");
        assert_eq!(status, StatusCode::OK);
        let items = body["items"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert!(!body.to_string().to_lowercase().contains("secret"));
    }

    // --- run_controller_auth: auth/scope tests (D-46, T-27-15-01) ---------
    //
    // Named module (not just a flat function set) so `cargo test
    // run_controller_auth` selects every test below by substring match on
    // the fully-qualified test path.
    mod run_controller_auth {
        use super::*;

        fn authed_state(auth: crate::agent_auth::AgentAuthConfig) -> RunApiState {
            RunApiState::new()
                .with_submission(Arc::new(MockSubmissionPort {
                    outcome: MockOutcome::Accepted,
                }))
                .with_auth(auth)
        }

        fn api_key_auth(
            key: &str,
            role: paladin_core::platform::container::user::UserRole,
        ) -> crate::agent_auth::AgentAuthConfig {
            api_key_auth_with_tenant(key, "svc", role, "svc-tenant")
        }

        fn api_key_auth_with_tenant(
            key: &str,
            principal_id: &str,
            role: paladin_core::platform::container::user::UserRole,
            tenant: &str,
        ) -> crate::agent_auth::AgentAuthConfig {
            let mut api_keys = HashMap::new();
            api_keys.insert(
                key.to_string(),
                Principal::new(
                    principal_id,
                    role,
                    paladin_core::platform::container::principal::TenantId::new(tenant).unwrap(),
                ),
            );
            crate::agent_auth::AgentAuthConfig {
                enabled: true,
                api_keys,
                token_verifier: None,
                bearer_tenant: None,
            }
        }

        #[tokio::test]
        async fn unauthenticated_request_is_401() {
            let auth = api_key_auth(
                "sk-abc",
                paladin_core::platform::container::user::UserRole::User,
            );
            let app = run_router(authed_state(auth));
            let response = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/v1/runs")
                        .header("content-type", "application/json")
                        .body(Body::from(
                            serde_json::to_vec(&serde_json::json!({ "assistant_id": "a1" }))
                                .unwrap(),
                        ))
                        .expect("request builds"),
                )
                .await
                .expect("router responds");
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }

        #[tokio::test]
        async fn submit_forbidden_role_is_403() {
            let state = RunApiState::new().with_submission(Arc::new(MockSubmissionPort {
                outcome: MockOutcome::Forbidden,
            }));
            let app = run_router(state);
            let response = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/v1/runs")
                        .header("content-type", "application/json")
                        .body(Body::from(
                            serde_json::to_vec(&serde_json::json!({ "assistant_id": "a1" }))
                                .unwrap(),
                        ))
                        .expect("request builds"),
                )
                .await
                .expect("router responds");
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
        }

        #[tokio::test]
        async fn cancel_forbidden_role_is_403() {
            let state = RunApiState::new().with_submission(Arc::new(MockSubmissionPort {
                outcome: MockOutcome::Forbidden,
            }));
            let app = run_router(state);
            let response = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(format!("/v1/runs/{}/cancel", RunId::new_v7()))
                        .body(Body::empty())
                        .expect("request builds"),
                )
                .await
                .expect("router responds");
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
        }

        /// D-46: registry-shaped mutations (assistant create) require
        /// admin, even though they are merged onto the SAME router this
        /// module tests -- proves the two-tier convention actually holds
        /// at the router level, not just per-file.
        #[tokio::test]
        async fn admin_only_assistant_route_is_403_for_non_admin() {
            let auth = api_key_auth(
                "user-key",
                paladin_core::platform::container::user::UserRole::User,
            );
            let app = run_router(RunApiState::new().with_auth(auth));
            let response = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/v1/assistants")
                        .header("x-api-key", "user-key")
                        .header("content-type", "application/json")
                        .body(Body::from(
                            serde_json::to_vec(&serde_json::json!({
                                "assistant_id": "a1",
                                "definition": { "kind": "workflow", "body": {} }
                            }))
                            .unwrap(),
                        ))
                        .expect("request builds"),
                )
                .await
                .expect("router responds");
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
        }

        /// The existing rate limiter (`with_http_layers`) proven on the new
        /// `/v1/runs` router exactly as it already is on `/v1/agents/*`
        /// (D-44): a 2 req/s, burst-1 governor answers the second
        /// immediate request with `429`.
        #[tokio::test]
        async fn rate_limited_request_is_429() {
            let state = RunApiState::new().with_submission(Arc::new(MockSubmissionPort {
                outcome: MockOutcome::Accepted,
            }));
            let inner = run_router(state);
            let config = crate::http_layers::RateLimitConfig {
                enabled: true,
                per_second: 2,
                burst: 1,
            };
            let app = crate::http_layers::apply_rate_limit(inner, &config);

            let make_req = || {
                Request::builder()
                    .uri(format!("/v1/runs/{}", RunId::new_v7()))
                    .header("x-real-ip", "9.9.9.9")
                    .body(Body::empty())
                    .unwrap()
            };

            let first = app.clone().oneshot(make_req()).await.unwrap();
            assert_eq!(first.status(), StatusCode::NOT_IMPLEMENTED); // unwired repo, still admitted

            let second = app.clone().oneshot(make_req()).await.unwrap();
            let third = app.oneshot(make_req()).await.unwrap();
            assert!(
                second.status() == StatusCode::TOO_MANY_REQUESTS
                    || third.status() == StatusCode::TOO_MANY_REQUESTS,
                "expected a 429 within the burst window"
            );
        }

        // --- Tenant-scoped GET /runs/{id} (Phase 40, D-12/D-13, PLAT-07) ---

        fn authed_state_with_repository(
            auth: crate::agent_auth::AgentAuthConfig,
            repository: Arc<MockRepository>,
        ) -> RunApiState {
            RunApiState::new()
                .with_repository(repository)
                .with_auth(auth)
        }

        #[tokio::test]
        async fn get_run_hides_a_foreign_tenant_run_behind_the_missing_run_404() {
            let repository = Arc::new(MockRepository::default());
            let run = sample_run("t1").with_submitted_by(
                paladin_core::platform::container::principal::RunAttribution::new(
                    paladin_core::platform::container::principal::TenantId::new("acme").unwrap(),
                    "svc-a".to_string(),
                ),
            );
            let run_id = run.run_id.clone();
            repository.seed(run);

            let mut api_keys = HashMap::new();
            api_keys.insert(
                "svc-a-key".to_string(),
                Principal::new(
                    "svc-a",
                    paladin_core::platform::container::user::UserRole::User,
                    paladin_core::platform::container::principal::TenantId::new("acme").unwrap(),
                ),
            );
            api_keys.insert(
                "svc-b-key".to_string(),
                Principal::new(
                    "svc-b",
                    paladin_core::platform::container::user::UserRole::User,
                    paladin_core::platform::container::principal::TenantId::new("globex").unwrap(),
                ),
            );
            let auth = crate::agent_auth::AgentAuthConfig {
                enabled: true,
                api_keys,
                token_verifier: None,
                bearer_tenant: None,
            };
            let app = run_router(authed_state_with_repository(auth, repository));

            let owner_response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/v1/runs/{run_id}"))
                        .header("x-api-key", "svc-a-key")
                        .body(Body::empty())
                        .expect("request builds"),
                )
                .await
                .expect("router responds");
            assert_eq!(owner_response.status(), StatusCode::OK);

            let hidden_response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/v1/runs/{run_id}"))
                        .header("x-api-key", "svc-b-key")
                        .body(Body::empty())
                        .expect("request builds"),
                )
                .await
                .expect("router responds");
            assert_eq!(hidden_response.status(), StatusCode::NOT_FOUND);
            let hidden_body = axum::body::to_bytes(hidden_response.into_body(), usize::MAX)
                .await
                .unwrap();

            let missing_id = RunId::new_v7();
            let missing_response = app
                .oneshot(
                    Request::builder()
                        .uri(format!("/v1/runs/{missing_id}"))
                        .header("x-api-key", "svc-b-key")
                        .body(Body::empty())
                        .expect("request builds"),
                )
                .await
                .expect("router responds");
            assert_eq!(missing_response.status(), StatusCode::NOT_FOUND);
            let missing_body = axum::body::to_bytes(missing_response.into_body(), usize::MAX)
                .await
                .unwrap();

            // Byte-equal once the two ids are swapped (D-12: no shape/timing difference
            // between "hidden" and "genuinely missing").
            let hidden_text = String::from_utf8(hidden_body.to_vec()).unwrap();
            let missing_text = String::from_utf8(missing_body.to_vec()).unwrap();
            let normalized_hidden =
                hidden_text.replace(&run_id.to_string(), &missing_id.to_string());
            assert_eq!(normalized_hidden, missing_text);
        }

        #[tokio::test]
        async fn get_run_lets_an_admin_of_another_tenant_see_the_run() {
            let repository = Arc::new(MockRepository::default());
            let run = sample_run("t1").with_submitted_by(
                paladin_core::platform::container::principal::RunAttribution::new(
                    paladin_core::platform::container::principal::TenantId::new("acme").unwrap(),
                    "svc-a".to_string(),
                ),
            );
            let run_id = run.run_id.clone();
            repository.seed(run);

            let mut api_keys = HashMap::new();
            api_keys.insert(
                "ops-key".to_string(),
                Principal::new(
                    "ops",
                    paladin_core::platform::container::user::UserRole::Admin,
                    paladin_core::platform::container::principal::TenantId::new("ops-tenant")
                        .unwrap(),
                ),
            );
            let auth = crate::agent_auth::AgentAuthConfig {
                enabled: true,
                api_keys,
                token_verifier: None,
                bearer_tenant: None,
            };
            let app = run_router(authed_state_with_repository(auth, repository));

            let response = app
                .oneshot(
                    Request::builder()
                        .uri(format!("/v1/runs/{run_id}"))
                        .header("x-api-key", "ops-key")
                        .body(Body::empty())
                        .expect("request builds"),
                )
                .await
                .expect("router responds");
            assert_eq!(response.status(), StatusCode::OK);
        }

        /// A small recording `RunSubmissionPort` double capturing the `SubmitRun` it received,
        /// so this test can assert the exact `PrincipalRef` the controller forwarded (D-04).
        struct RecordingSubmissionPort {
            captured: std::sync::Mutex<Option<SubmitRun>>,
        }

        #[async_trait]
        impl RunSubmissionPort for RecordingSubmissionPort {
            async fn submit(
                &self,
                request: SubmitRun,
            ) -> Result<paladin_ports::input::run_submission_port::RunAccepted, RunSubmissionError>
            {
                let thread_id = request
                    .thread_id
                    .clone()
                    .unwrap_or_else(|| ThreadId::new("t1").unwrap());
                *self.captured.lock().unwrap() = Some(request);
                Ok(paladin_ports::input::run_submission_port::RunAccepted {
                    run_id: RunId::new_v7(),
                    thread_id,
                })
            }

            async fn cancel(
                &self,
                run_id: &RunId,
                _requested_by: Option<PrincipalRef>,
            ) -> Result<paladin_ports::input::run_submission_port::CancelOutcome, RunSubmissionError>
            {
                Err(RunSubmissionError::NotFound {
                    run_id: run_id.clone(),
                })
            }

            async fn fork(
                &self,
                _request: paladin_ports::input::run_submission_port::ForkRun,
            ) -> Result<paladin_ports::input::run_submission_port::RunAccepted, RunSubmissionError>
            {
                Err(RunSubmissionError::NotWired)
            }
        }

        #[tokio::test]
        async fn submit_run_forwards_the_callers_principal_ref() {
            let submission = Arc::new(RecordingSubmissionPort {
                captured: std::sync::Mutex::new(None),
            });
            let auth = api_key_auth_with_tenant(
                "svc-a-key",
                "svc-a",
                paladin_core::platform::container::user::UserRole::User,
                "acme",
            );
            let state = RunApiState::new()
                .with_submission(Arc::clone(&submission) as Arc<dyn RunSubmissionPort>)
                .with_auth(auth);
            let app = run_router(state);

            let response = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/v1/runs")
                        .header("x-api-key", "svc-a-key")
                        .header("content-type", "application/json")
                        .body(Body::from(
                            serde_json::to_vec(&serde_json::json!({ "assistant_id": "a1" }))
                                .unwrap(),
                        ))
                        .expect("request builds"),
                )
                .await
                .expect("router responds");
            assert_eq!(response.status(), StatusCode::ACCEPTED);

            let captured = submission.captured.lock().unwrap().take().unwrap();
            let requested_by = captured.requested_by.expect("principal ref forwarded");
            assert_eq!(requested_by.api_key_id, "svc-a");
            assert_eq!(
                requested_by.tenant_id,
                paladin_core::platform::container::principal::TenantId::new("acme").unwrap()
            );
            assert_eq!(
                requested_by.role,
                paladin_core::platform::container::user::UserRole::User
            );
        }

        // --- Route matrix (Phase 40, 40-05, D-13, research Pitfall 10, PLAT-07) ---

        /// A `RunSubmissionPort` double that records the `api_key_id` behind every
        /// `cancel` call, so the matrix can prove a foreign caller never reaches the
        /// port at all (D-13, T-40-19) -- the gate answers before `cancel` is invoked.
        struct RecordingCancelPort {
            cancel_callers: Mutex<Vec<String>>,
        }

        #[async_trait]
        impl RunSubmissionPort for RecordingCancelPort {
            async fn submit(
                &self,
                _request: SubmitRun,
            ) -> Result<paladin_ports::input::run_submission_port::RunAccepted, RunSubmissionError>
            {
                Err(RunSubmissionError::NotWired)
            }

            async fn cancel(
                &self,
                run_id: &RunId,
                requested_by: Option<PrincipalRef>,
            ) -> Result<paladin_ports::input::run_submission_port::CancelOutcome, RunSubmissionError>
            {
                self.cancel_callers.lock().unwrap().push(
                    requested_by
                        .map(|principal| principal.api_key_id)
                        .unwrap_or_else(|| "<internal>".to_string()),
                );
                Ok(paladin_ports::input::run_submission_port::CancelOutcome {
                    run_id: run_id.clone(),
                    status: RunStatus::Running,
                    was_local: false,
                })
            }

            async fn fork(
                &self,
                _request: paladin_ports::input::run_submission_port::ForkRun,
            ) -> Result<paladin_ports::input::run_submission_port::RunAccepted, RunSubmissionError>
            {
                Err(RunSubmissionError::NotWired)
            }
        }

        /// Every `(METHOD, path)` operation under `/v1/runs/{run_id}` the router publishes,
        /// read from its OWN OpenAPI document (`versioned_run_parts(..).1`) rather than a
        /// hand-maintained list -- so a route added to `run_openapi_router` without joining
        /// the matrix table fails the test (research Pitfall 10, T-40-22).
        fn published_run_id_operations(api: &utoipa::openapi::OpenApi) -> Vec<(String, String)> {
            let mut operations = Vec::new();
            for (path, item) in &api.paths.paths {
                if !path.starts_with("/v1/runs/{run_id}") {
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

        async fn send_as(app: &axum::Router, method: &str, uri: String, key: &str) -> Response {
            app.clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(uri)
                        .header("x-api-key", key)
                        .body(Body::empty())
                        .expect("request builds"),
                )
                .await
                .expect("router responds")
        }

        /// D-13 / PLAT-07 success criterion 3, on the WHOLE `/runs/{run_id}*` route set:
        /// the operations are enumerated from the router's own OpenAPI document and
        /// compared against this table; for every row the owner key, a second key of
        /// the same tenant and an Admin of another tenant get the success status, a key of
        /// a different tenant gets a 404 whose body is byte-identical to that route's
        /// missing-run 404 once the ids are swapped (D-11, D-12), and the recording cancel
        /// double never sees a call from the foreign key.
        #[tokio::test]
        async fn every_run_id_route_hides_foreign_runs_behind_the_missing_run_404() {
            use paladin_core::platform::container::principal::{RunAttribution, TenantId};
            use paladin_core::platform::container::run::RunStreamEventKind;
            use paladin_core::platform::container::user::UserRole;

            let submission = Arc::new(RecordingCancelPort {
                cancel_callers: Mutex::new(Vec::new()),
            });
            let repository = Arc::new(MockRepository::default());
            let run = sample_run("t-matrix").with_submitted_by(RunAttribution::new(
                TenantId::new("acme").unwrap(),
                "svc-a",
            ));
            let run_id = run.run_id.clone();
            repository.seed(run);

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
            let auth = crate::agent_auth::AgentAuthConfig {
                enabled: true,
                api_keys,
                token_verifier: None,
                bearer_tenant: None,
            };

            let events = vec![sample_stream_event(
                &run_id,
                RunStreamEventKind::Done,
                serde_json::json!({ "status": "completed", "waypoint_id": "wp-1" }),
            )];
            let state = RunApiState::new()
                .with_submission(Arc::clone(&submission) as Arc<dyn RunSubmissionPort>)
                .with_repository(repository)
                .with_run_events(Arc::new(MockRunEventStreamPort { events }))
                .with_webhook_deliveries(Arc::new(MockWebhookDeliveries::default()))
                .with_auth(auth);
            let (app, api) = versioned_run_parts(state);

            // The table every `/v1/runs/{run_id}*` operation must appear in, with the
            // status its owner gets. Adding a route to `run_openapi_router` without adding
            // it here (and routing it through `load_visible_run`) fails the assertion
            // below.
            let table: [(&str, &str, StatusCode); 4] = [
                ("GET", "/v1/runs/{run_id}", StatusCode::OK),
                ("GET", "/v1/runs/{run_id}/stream", StatusCode::OK),
                ("POST", "/v1/runs/{run_id}/cancel", StatusCode::ACCEPTED),
                (
                    "GET",
                    "/v1/runs/{run_id}/webhook-deliveries",
                    StatusCode::OK,
                ),
            ];
            let mut tabled: Vec<(String, String)> = table
                .iter()
                .map(|(method, path, _)| (method.to_string(), path.to_string()))
                .collect();
            tabled.sort();
            let published = published_run_id_operations(&api);
            assert_eq!(
                published, tabled,
                "the /v1/runs/{{run_id}}* operations the router publishes do not match the \
                 matrix table: published={published:?} table={tabled:?}. Every route under \
                 /v1/runs/{{run_id}} must enter through `load_visible_run` (D-13, PLAT-07) \
                 and be added to this table with its owner success status; a route that no \
                 longer exists must be removed from the table."
            );

            for (method, template, success) in table {
                let uri_for = |id: &RunId| template.replace("{run_id}", &id.to_string());

                for key in ["owner-key", "peer-key", "admin-key"] {
                    let response = send_as(&app, method, uri_for(&run_id), key).await;
                    assert_eq!(
                        response.status(),
                        success,
                        "{method} {template} as {key} must succeed (D-11: same tenant or Admin)"
                    );
                }

                let hidden = send_as(&app, method, uri_for(&run_id), "foreign-key").await;
                assert_eq!(
                    hidden.status(),
                    StatusCode::NOT_FOUND,
                    "{method} {template} as a foreign tenant must be the missing-run 404 (D-12)"
                );
                let hidden_text = read_response_body(hidden).await;

                let missing_id = RunId::new_v7();
                let missing = send_as(&app, method, uri_for(&missing_id), "foreign-key").await;
                assert_eq!(
                    missing.status(),
                    StatusCode::NOT_FOUND,
                    "{method} {template} for a missing run must be 404"
                );
                let missing_text = read_response_body(missing).await;

                assert_eq!(
                    hidden_text.replace(&run_id.to_string(), &missing_id.to_string()),
                    missing_text,
                    "{method} {template}: a hidden run's 404 body must be byte-identical to \
                     the missing-run 404 once the ids are swapped (D-12, T-40-21)"
                );
            }

            let mut callers = submission.cancel_callers.lock().unwrap().clone();
            callers.sort();
            assert_eq!(
                callers,
                vec!["ops".to_string(), "svc-a".to_string(), "svc-a2".to_string()],
                "RunSubmissionPort::cancel must be reached only by the owner, the same-tenant \
                 peer and the Admin -- never by the foreign key (D-13, T-40-19)"
            );
        }
    }
}
