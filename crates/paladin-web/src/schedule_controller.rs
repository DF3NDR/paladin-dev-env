//! Axum HTTP controller for run schedules (PLAT-05, PLAT-06, D-42, D-44, D-46, D-47).
//!
//! Routes live on the shared [`RunApiState`] (D-44) -- `crate::run_controller::run_openapi_router`
//! merges `schedule_routes` in, so one router carries `/v1/runs*`, `/v1/assistants*` and
//! `/v1/schedules*`.
//!
//! | Method & path | Description |
//! |---------------|-------------|
//! | `POST /schedules` | Create a brand-new schedule (admin) |
//! | `GET /schedules` | Paginated list |
//! | `GET /schedules/{id}` | One schedule (`last_tick`/`next_tick`/`skipped_ticks` included) |
//! | `PATCH /schedules/{id}` | Partial update (admin) -- a cron/timezone change recomputes `next_tick` |
//! | `DELETE /schedules/{id}` | Delete (admin) |
//!
//! ## Authorization
//!
//! Every route sits behind the SAME `require_authentication` middleware `/v1/runs*` already
//! carries (D-44); create/patch/delete additionally call `require_admin` (D-46) -- reads
//! need authentication only.
//!
//! ## The webhook secret is never echoed back (T-27-14-03)
//!
//! `ScheduleResponse`'s `webhook.secret` renders `"***"` when a secret is set on the
//! stored schedule, and `null` when none was set -- the raw secret is accepted on write
//! (`CreateScheduleRequest`/`PatchScheduleRequest`) but never appears in any response
//! body.

use axum::Extension;
use axum::extract::{Json, Path, Query, State};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use paladin_core::platform::container::run::{RunEventKind, WebhookSpec};
use paladin_core::platform::container::run_schedule::{
    OnMissed, RunSchedule, RunScheduleId, RunScheduleUpdate, ThreadStrategy,
};
use paladin_ports::input::schedule_admin_port::{CreateRunSchedule, ScheduleAdminError};

use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::agent_auth::{Principal, require_admin};
use crate::agent_controller::{JsonValue, ok_body};
use crate::assistant_controller::ValidationViolationDto;
use crate::error::{ApiError, ApiErrorBody};
use crate::run_controller::RunApiState;

const SCHEDULE_PORT_HINT: &str = "no schedule admin backend configured: set schedules.enabled";
/// Maximum `limit` `GET /schedules` accepts, mirroring
/// `assistant_controller::MAX_ASSISTANT_LIMIT`'s own precedent (D-47).
const MAX_SCHEDULE_LIMIT: u32 = 100;
const DEFAULT_SCHEDULE_LIMIT: u32 = 20;

// --- DTOs ------------------------------------------------------------------

/// Wire shape of a webhook delivery target on write (`POST`/`PATCH`).
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct WebhookRequestDto {
    /// The delivery URL -- validated by the write-time SSRF guard (D-42) before the
    /// schedule is ever persisted.
    pub url: String,
    /// The HMAC signing secret, if any.
    #[serde(default)]
    pub secret: Option<String>,
    /// The lifecycle events this webhook subscribes to (`"awaiting_input"`, `"completed"`,
    /// `"failed"`, `"halted"`, `"cancelled"`).
    #[serde(default)]
    pub events: Vec<String>,
}

/// Wire shape of a webhook delivery target on read -- the SAME shape as
/// [`WebhookRequestDto`], except `secret` is redacted (T-27-14-03).
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct WebhookResponseDto {
    /// The delivery URL.
    pub url: String,
    /// `"***"` when a secret is set on the stored schedule, `null` otherwise -- the raw
    /// secret is never echoed back.
    pub secret: Option<String>,
    /// The lifecycle events this webhook subscribes to.
    pub events: Vec<String>,
}

impl From<&WebhookSpec> for WebhookResponseDto {
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

fn on_missed_label(on_missed: OnMissed) -> String {
    match on_missed {
        OnMissed::Skip => "skip",
        OnMissed::RunOnce => "run_once",
    }
    .to_string()
}

fn parse_on_missed(raw: &str) -> Result<OnMissed, ApiError> {
    match raw {
        "skip" => Ok(OnMissed::Skip),
        "run_once" => Ok(OnMissed::RunOnce),
        other => Err(ApiError::bad_request(format!(
            "unknown on_missed '{other}': expected 'skip' or 'run_once'"
        ))),
    }
}

/// Request body for `POST /schedules`.
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct CreateScheduleRequest {
    /// The assistant this schedule submits runs against.
    pub assistant_id: String,
    /// A specific version, or omitted to resolve `latest` at each tick.
    #[serde(default)]
    pub version: Option<u32>,
    /// The cron expression (5- or 6-field -- D-38).
    pub cron: String,
    /// The IANA timezone the cron expression is evaluated in, or omitted for `"UTC"`.
    #[serde(default)]
    pub timezone: Option<String>,
    /// The input every tick's submitted run receives.
    #[serde(default)]
    #[schema(value_type = Object)]
    pub input: serde_json::Value,
    /// Whether this schedule starts out active.
    pub enabled: bool,
    /// `"new_thread_per_tick"` or `{ "fixed_thread": "<thread id>" }` (mirrors
    /// `RunSchedule::thread_strategy`'s own wire shape); omitted for the default
    /// (`new_thread_per_tick`).
    #[serde(default)]
    #[schema(value_type = Object)]
    pub thread_strategy: Option<serde_json::Value>,
    /// `"skip"` or `"run_once"`; omitted for the default (`"skip"`).
    #[serde(default)]
    pub on_missed: Option<String>,
    /// An optional webhook delivery target, validated by the write-time SSRF guard (D-42).
    #[serde(default)]
    pub webhook: Option<WebhookRequestDto>,
}

/// Request body for `PATCH /schedules/{id}` -- every field optional; only fields present
/// change (D-39's "`enabled: false` leaves `next_tick` in place" applies at the port layer).
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct PatchScheduleRequest {
    /// A new cron expression, if changing it.
    #[serde(default)]
    pub cron: Option<String>,
    /// A new timezone, if changing it.
    #[serde(default)]
    pub timezone: Option<String>,
    /// A new input, if changing it.
    #[serde(default)]
    #[schema(value_type = Object, nullable)]
    pub input: Option<serde_json::Value>,
    /// A new enabled flag, if changing it.
    #[serde(default)]
    pub enabled: Option<bool>,
    /// A new thread strategy, if changing it -- same wire shape as
    /// [`CreateScheduleRequest::thread_strategy`].
    #[serde(default)]
    #[schema(value_type = Object, nullable)]
    pub thread_strategy: Option<serde_json::Value>,
    /// A new missed-tick policy (`"skip"` or `"run_once"`), if changing it.
    #[serde(default)]
    pub on_missed: Option<String>,
    /// A new webhook target, if changing it -- re-validated by the write-time SSRF guard.
    #[serde(default)]
    pub webhook: Option<WebhookRequestDto>,
}

/// Response body for `GET /schedules/{id}` and one item of `GET /schedules` -- every
/// [`RunSchedule`] field, including `last_tick`/`next_tick`/`skipped_ticks` so "why did
/// nothing run" is answerable without a log dive (D-39).
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ScheduleResponse {
    /// This schedule's identity.
    pub schedule_id: String,
    /// The assistant this schedule submits runs against.
    pub assistant_id: String,
    /// A specific version, or `null` to resolve `latest` at each tick.
    pub version: Option<u32>,
    /// The cron expression.
    pub cron: String,
    /// The IANA timezone the cron expression is evaluated in.
    pub timezone: String,
    /// The input every tick's submitted run receives.
    #[schema(value_type = Object)]
    pub input: serde_json::Value,
    /// Whether this schedule is currently active.
    pub enabled: bool,
    /// `"new_thread_per_tick"` or `{ "fixed_thread": "<thread id>" }`.
    #[schema(value_type = Object)]
    pub thread_strategy: serde_json::Value,
    /// `"skip"` or `"run_once"`.
    pub on_missed: String,
    /// The webhook delivery target, if any -- `secret` redacted to `"***"` (T-27-14-03).
    pub webhook: Option<WebhookResponseDto>,
    /// The last tick this schedule fired (or was claimed) at, if any.
    pub last_tick: Option<DateTime<Utc>>,
    /// The next tick this schedule is due at, if scheduled.
    pub next_tick: Option<DateTime<Utc>>,
    /// How many ticks have been skipped (D-39's counted metric).
    pub skipped_ticks: u64,
    /// When this schedule was created.
    pub created_at: DateTime<Utc>,
    /// When this schedule was last updated.
    pub updated_at: DateTime<Utc>,
}

impl From<&RunSchedule> for ScheduleResponse {
    fn from(schedule: &RunSchedule) -> Self {
        Self {
            schedule_id: schedule.schedule_id.to_string(),
            assistant_id: schedule.assistant_id.clone(),
            version: schedule.version,
            cron: schedule.cron.clone(),
            timezone: schedule.timezone.clone(),
            input: schedule.input.clone(),
            enabled: schedule.enabled,
            thread_strategy: serde_json::to_value(&schedule.thread_strategy)
                .unwrap_or(serde_json::Value::Null),
            on_missed: on_missed_label(schedule.on_missed),
            webhook: schedule.webhook.as_ref().map(WebhookResponseDto::from),
            last_tick: schedule.last_tick,
            next_tick: schedule.next_tick,
            skipped_ticks: schedule.skipped_ticks,
            created_at: schedule.created_at,
            updated_at: schedule.updated_at,
        }
    }
}

/// Response body for `GET /schedules`.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ScheduleListResponse {
    /// The page of schedules, in the documented order.
    pub items: Vec<ScheduleResponse>,
    /// Opaque cursor for the next page, `None` on the last page.
    pub next_cursor: Option<String>,
}

/// Query parameters for `GET /schedules`, mirroring
/// `assistant_controller::PageQuery`'s own `limit`/`cursor` names (D-47).
#[derive(Debug, Clone, Deserialize)]
pub struct PageQuery {
    /// Maximum number of items to return (at most `MAX_SCHEDULE_LIMIT`).
    #[serde(default)]
    pub limit: Option<u32>,
    /// Opaque pagination cursor from a previous page's `next_cursor`.
    #[serde(default)]
    pub cursor: Option<String>,
}

// --- Parsing helpers --------------------------------------------------------

fn parse_schedule_id(raw: &str) -> Result<RunScheduleId, ApiError> {
    RunScheduleId::parse(raw).map_err(|e| ApiError::bad_request(e.to_string()))
}

fn parse_limit(limit: Option<u32>) -> Result<u32, ApiError> {
    match limit {
        None => Ok(DEFAULT_SCHEDULE_LIMIT),
        Some(0) => Ok(DEFAULT_SCHEDULE_LIMIT),
        Some(limit) if limit > MAX_SCHEDULE_LIMIT => Err(ApiError::bad_request(format!(
            "limit must be at most {MAX_SCHEDULE_LIMIT}, got {limit}"
        ))),
        Some(limit) => Ok(limit),
    }
}

fn to_thread_strategy(raw: Option<serde_json::Value>) -> Result<Option<ThreadStrategy>, ApiError> {
    let Some(value) = raw else {
        return Ok(None);
    };
    serde_json::from_value::<ThreadStrategy>(value)
        .map(Some)
        .map_err(|e| {
            ApiError::bad_request("invalid thread_strategy").with_details(serde_json::json!({
                "violations": [{
                    "path": "/thread_strategy",
                    "code": "invalid_thread_strategy",
                    "message": e.to_string(),
                }]
            }))
        })
}

fn to_webhook_spec(dto: WebhookRequestDto) -> Result<WebhookSpec, ApiError> {
    let mut events = Vec::with_capacity(dto.events.len());
    for raw in &dto.events {
        events.push(parse_event_kind(raw)?);
    }
    Ok(WebhookSpec {
        url: dto.url,
        secret: dto.secret,
        events,
    })
}

// --- Error mapping -----------------------------------------------------------

fn map_admin_error(err: ScheduleAdminError) -> ApiError {
    match err {
        ScheduleAdminError::Invalid { violations } => {
            let details: Vec<ValidationViolationDto> = violations
                .iter()
                .map(ValidationViolationDto::from)
                .collect();
            ApiError::bad_request("run schedule failed validation")
                .with_details(serde_json::json!({ "violations": details }))
        }
        ScheduleAdminError::NotFound { schedule_id } => {
            ApiError::not_found(format!("unknown schedule '{schedule_id}'"))
        }
        ScheduleAdminError::Backend { source } => ApiError::internal(source.to_string()),
        ScheduleAdminError::NotWired => ApiError::not_implemented(SCHEDULE_PORT_HINT),
        other => ApiError::internal(other.to_string()),
    }
}

// --- Handlers ------------------------------------------------------------------

/// `POST /schedules` -- create a brand-new schedule (admin).
///
/// Returns:
/// - `201 Created` with [`ScheduleResponse`] on success;
/// - `400 Bad Request` for an invalid cron, timezone, assistant reference, webhook URL
///   (write-time SSRF guard, D-42) or thread strategy -- `error.details.violations` is a
///   non-empty, machine-readable list; nothing is persisted;
/// - `401`/`403` per the module-level authorization docs;
/// - `501 Not Implemented` if no schedule admin backend is configured.
#[utoipa::path(
    post,
    path = "/schedules",
    tag = "schedules",
    request_body = CreateScheduleRequest,
    responses(
        (status = 201, description = "Schedule created", body = ScheduleResponse),
        (status = 400, description = "Invalid cron, timezone, assistant reference, webhook URL, or thread strategy -- violations in `details`", body = ApiErrorBody),
        (status = 401, description = "Missing/invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Admin role required", body = ApiErrorBody),
        (status = 501, description = "No schedule admin backend configured", body = ApiErrorBody),
    ),
    security(("api_key" = []), ("bearer_token" = [])),
)]
pub async fn create_schedule(
    State(state): State<RunApiState>,
    Extension(principal): Extension<Principal>,
    Json(body): Json<CreateScheduleRequest>,
) -> Result<(StatusCode, JsonValue), ApiError> {
    require_admin(&principal)?;
    let schedules = state
        .schedules
        .as_ref()
        .ok_or_else(|| ApiError::not_implemented(SCHEDULE_PORT_HINT))?;

    let thread_strategy = to_thread_strategy(body.thread_strategy)?;
    let on_missed = body.on_missed.as_deref().map(parse_on_missed).transpose()?;
    let webhook = body.webhook.map(to_webhook_spec).transpose()?;

    let create = CreateRunSchedule {
        assistant_id: body.assistant_id,
        version: body.version,
        cron: body.cron,
        timezone: body.timezone,
        input: body.input,
        enabled: body.enabled,
        thread_strategy,
        on_missed,
        webhook,
    };

    let schedule = schedules.create(create).await.map_err(map_admin_error)?;
    Ok((
        StatusCode::CREATED,
        ok_body(&ScheduleResponse::from(&schedule)),
    ))
}

/// `GET /schedules` -- paginated list (D-47). Authenticated, any role.
///
/// Returns `200 OK` with [`ScheduleListResponse`] on success, or `501 Not Implemented` if no
/// schedule admin backend is configured.
#[utoipa::path(
    get,
    path = "/schedules",
    tag = "schedules",
    params(
        ("limit" = Option<u32>, Query, description = "Max items to return (at most 100)"),
        ("cursor" = Option<String>, Query, description = "Opaque pagination cursor from a previous page's next_cursor"),
    ),
    responses(
        (status = 200, description = "A page of schedules", body = ScheduleListResponse),
        (status = 400, description = "limit exceeds 100", body = ApiErrorBody),
        (status = 401, description = "Missing/invalid credentials", body = ApiErrorBody),
        (status = 501, description = "No schedule admin backend configured", body = ApiErrorBody),
    ),
    security(("api_key" = []), ("bearer_token" = [])),
)]
pub async fn list_schedules(
    State(state): State<RunApiState>,
    Extension(_principal): Extension<Principal>,
    Query(params): Query<PageQuery>,
) -> Result<(StatusCode, JsonValue), ApiError> {
    let schedules = state
        .schedules
        .as_ref()
        .ok_or_else(|| ApiError::not_implemented(SCHEDULE_PORT_HINT))?;
    let limit = parse_limit(params.limit)?;
    let cursor = params
        .cursor
        .as_deref()
        .map(parse_schedule_id)
        .transpose()?;

    let page = schedules
        .list(limit, cursor)
        .await
        .map_err(map_admin_error)?;
    let items: Vec<ScheduleResponse> = page.items.iter().map(ScheduleResponse::from).collect();
    let next_cursor = page.next_cursor.map(|id| id.to_string());
    Ok((
        StatusCode::OK,
        ok_body(&ScheduleListResponse { items, next_cursor }),
    ))
}

/// `GET /schedules/{id}` -- one schedule, including `last_tick`/`next_tick`/`skipped_ticks`
/// (D-39). Authenticated, any role.
///
/// Returns:
/// - `200 OK` with [`ScheduleResponse`] on success;
/// - `400 Bad Request` for an invalid id;
/// - `404 Not Found` if `id` does not exist;
/// - `501 Not Implemented` if no schedule admin backend is configured.
#[utoipa::path(
    get,
    path = "/schedules/{schedule_id}",
    tag = "schedules",
    params(("schedule_id" = String, Path, description = "Schedule id")),
    responses(
        (status = 200, description = "The schedule", body = ScheduleResponse),
        (status = 400, description = "Invalid schedule id", body = ApiErrorBody),
        (status = 401, description = "Missing/invalid credentials", body = ApiErrorBody),
        (status = 404, description = "Unknown schedule", body = ApiErrorBody),
        (status = 501, description = "No schedule admin backend configured", body = ApiErrorBody),
    ),
    security(("api_key" = []), ("bearer_token" = [])),
)]
pub async fn get_schedule(
    State(state): State<RunApiState>,
    Extension(_principal): Extension<Principal>,
    Path(schedule_id): Path<String>,
) -> Result<(StatusCode, JsonValue), ApiError> {
    let schedules = state
        .schedules
        .as_ref()
        .ok_or_else(|| ApiError::not_implemented(SCHEDULE_PORT_HINT))?;
    let id = parse_schedule_id(&schedule_id)?;

    let schedule = schedules
        .get(&id)
        .await
        .map_err(map_admin_error)?
        .ok_or_else(|| ApiError::not_found(format!("unknown schedule '{schedule_id}'")))?;

    Ok((StatusCode::OK, ok_body(&ScheduleResponse::from(&schedule))))
}

/// `PATCH /schedules/{id}` -- partial update (admin). A `cron`/`timezone` change
/// recomputes `next_tick`; `enabled: false` leaves `next_tick` in place (D-39); a `webhook`
/// change re-runs the write-time SSRF guard (D-42).
///
/// Returns:
/// - `200 OK` with [`ScheduleResponse`] on success;
/// - `400 Bad Request` for an invalid id, cron, timezone, webhook URL or thread strategy --
///   `error.details.violations` is a non-empty, machine-readable list; nothing is persisted;
/// - `401`/`403` per the module-level authorization docs;
/// - `404 Not Found` if `id` does not exist;
/// - `501 Not Implemented` if no schedule admin backend is configured.
#[utoipa::path(
    patch,
    path = "/schedules/{schedule_id}",
    tag = "schedules",
    params(("schedule_id" = String, Path, description = "Schedule id")),
    request_body = PatchScheduleRequest,
    responses(
        (status = 200, description = "Schedule updated", body = ScheduleResponse),
        (status = 400, description = "Invalid schedule id, cron, timezone, webhook URL, or thread strategy -- violations in `details`", body = ApiErrorBody),
        (status = 401, description = "Missing/invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Admin role required", body = ApiErrorBody),
        (status = 404, description = "Unknown schedule", body = ApiErrorBody),
        (status = 501, description = "No schedule admin backend configured", body = ApiErrorBody),
    ),
    security(("api_key" = []), ("bearer_token" = [])),
)]
pub async fn patch_schedule(
    State(state): State<RunApiState>,
    Extension(principal): Extension<Principal>,
    Path(schedule_id): Path<String>,
    Json(body): Json<PatchScheduleRequest>,
) -> Result<(StatusCode, JsonValue), ApiError> {
    require_admin(&principal)?;
    let schedules = state
        .schedules
        .as_ref()
        .ok_or_else(|| ApiError::not_implemented(SCHEDULE_PORT_HINT))?;
    let id = parse_schedule_id(&schedule_id)?;

    let thread_strategy = to_thread_strategy(body.thread_strategy)?;
    let on_missed = body.on_missed.as_deref().map(parse_on_missed).transpose()?;
    let webhook = body.webhook.map(to_webhook_spec).transpose()?;

    let update = RunScheduleUpdate {
        cron: body.cron,
        timezone: body.timezone,
        input: body.input,
        enabled: body.enabled,
        thread_strategy,
        on_missed,
        webhook,
        next_tick: None,
    };

    let schedule = schedules
        .patch(&id, update)
        .await
        .map_err(map_admin_error)?;
    Ok((StatusCode::OK, ok_body(&ScheduleResponse::from(&schedule))))
}

/// `DELETE /schedules/{id}` -- delete (admin).
///
/// Returns:
/// - `204 No Content` on success;
/// - `400 Bad Request` for an invalid id;
/// - `401`/`403` per the module-level authorization docs;
/// - `404 Not Found` if `id` does not exist;
/// - `501 Not Implemented` if no schedule admin backend is configured.
#[utoipa::path(
    delete,
    path = "/schedules/{schedule_id}",
    tag = "schedules",
    params(("schedule_id" = String, Path, description = "Schedule id")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 400, description = "Invalid schedule id", body = ApiErrorBody),
        (status = 401, description = "Missing/invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Admin role required", body = ApiErrorBody),
        (status = 404, description = "Unknown schedule", body = ApiErrorBody),
        (status = 501, description = "No schedule admin backend configured", body = ApiErrorBody),
    ),
    security(("api_key" = []), ("bearer_token" = [])),
)]
pub async fn delete_schedule(
    State(state): State<RunApiState>,
    Extension(principal): Extension<Principal>,
    Path(schedule_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    require_admin(&principal)?;
    let schedules = state
        .schedules
        .as_ref()
        .ok_or_else(|| ApiError::not_implemented(SCHEDULE_PORT_HINT))?;
    let id = parse_schedule_id(&schedule_id)?;

    schedules.delete(&id).await.map_err(map_admin_error)?;
    Ok(StatusCode::NO_CONTENT)
}

// --- Router -------------------------------------------------------------------

/// Build the schedule API's routes as an [`OpenApiRouter`] over [`RunApiState`], merged by
/// `crate::run_controller::run_openapi_router` so one router carries `/v1/runs*`,
/// `/v1/assistants*` and `/v1/schedules*` (D-44).
pub(crate) fn schedule_routes() -> OpenApiRouter<RunApiState> {
    OpenApiRouter::new()
        .routes(routes!(create_schedule))
        .routes(routes!(list_schedules))
        .routes(routes!(get_schedule))
        .routes(routes!(patch_schedule))
        .routes(routes!(delete_schedule))
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use axum::body::Body;
    use axum::http::Request;
    use axum::response::IntoResponse;
    use paladin_ports::input::schedule_admin_port::ScheduleAdminPort;
    use paladin_ports::output::run_schedule_repository_port::RunSchedulePage;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use tower::ServiceExt;

    use crate::run_controller::run_router;

    // `paladin-web` does not depend on `paladin-ai`/`paladin-storage` (ADR-0031) --
    // this test double implements `ScheduleAdminPort` directly over a plain
    // `Mutex<HashMap<..>>`, with a deliberately permissive "validator" (rejects only a
    // missing cron) -- exactly enough to exercise every HTTP-layer branch this controller
    // owns. `ScheduleService`'s own real validation (cron/timezone/assistant/SSRF/thread-id)
    // is proven by `src/application/services/run/schedule/tests.rs`, not this file.
    #[derive(Default)]
    struct TestAdminPort {
        schedules: Mutex<HashMap<String, RunSchedule>>,
    }

    #[async_trait]
    impl ScheduleAdminPort for TestAdminPort {
        async fn create(
            &self,
            create: CreateRunSchedule,
        ) -> Result<RunSchedule, ScheduleAdminError> {
            if create.cron.trim().is_empty() {
                return Err(ScheduleAdminError::Invalid {
                    violations: vec![
                        paladin_ports::input::assistant_admin_port::ValidationViolation::new(
                            "/cron",
                            "missing_field",
                            "cron must not be empty",
                        ),
                    ],
                });
            }
            let schedule_id = RunScheduleId::new_v7();
            let mut schedule =
                RunSchedule::new(schedule_id.clone(), create.assistant_id, create.cron)
                    .with_input(create.input)
                    .with_next_tick(chrono::Utc::now());
            if let Some(version) = create.version {
                schedule = schedule.with_version(version);
            }
            if let Some(timezone) = create.timezone {
                schedule = schedule.with_timezone(timezone);
            }
            if let Some(strategy) = create.thread_strategy {
                schedule = schedule.with_thread_strategy(strategy);
            }
            if let Some(on_missed) = create.on_missed {
                schedule = schedule.with_on_missed(on_missed);
            }
            if let Some(webhook) = create.webhook {
                schedule = schedule.with_webhook(webhook);
            }
            if !create.enabled {
                schedule = schedule.disabled();
            }
            self.schedules
                .lock()
                .unwrap()
                .insert(schedule_id.to_string(), schedule.clone());
            Ok(schedule)
        }

        async fn get(
            &self,
            schedule_id: &RunScheduleId,
        ) -> Result<Option<RunSchedule>, ScheduleAdminError> {
            Ok(self
                .schedules
                .lock()
                .unwrap()
                .get(schedule_id.as_str())
                .cloned())
        }

        async fn list(
            &self,
            _limit: u32,
            _cursor: Option<RunScheduleId>,
        ) -> Result<RunSchedulePage, ScheduleAdminError> {
            let mut items: Vec<RunSchedule> =
                self.schedules.lock().unwrap().values().cloned().collect();
            items.sort_by(|a, b| a.schedule_id.cmp(&b.schedule_id));
            Ok(RunSchedulePage {
                items,
                next_cursor: None,
            })
        }

        async fn patch(
            &self,
            schedule_id: &RunScheduleId,
            update: RunScheduleUpdate,
        ) -> Result<RunSchedule, ScheduleAdminError> {
            let mut schedules = self.schedules.lock().unwrap();
            let Some(schedule) = schedules.get_mut(schedule_id.as_str()) else {
                return Err(ScheduleAdminError::NotFound {
                    schedule_id: schedule_id.clone(),
                });
            };
            if let Some(cron) = update.cron {
                schedule.cron = cron;
            }
            if let Some(timezone) = update.timezone {
                schedule.timezone = timezone;
            }
            if let Some(input) = update.input {
                schedule.input = input;
            }
            if let Some(enabled) = update.enabled {
                schedule.enabled = enabled;
            }
            if let Some(strategy) = update.thread_strategy {
                schedule.thread_strategy = strategy;
            }
            if let Some(on_missed) = update.on_missed {
                schedule.on_missed = on_missed;
            }
            if update.webhook.is_some() {
                schedule.webhook = update.webhook;
            }
            if let Some(next_tick) = update.next_tick {
                schedule.next_tick = Some(next_tick);
            }
            Ok(schedule.clone())
        }

        async fn delete(&self, schedule_id: &RunScheduleId) -> Result<(), ScheduleAdminError> {
            let mut schedules = self.schedules.lock().unwrap();
            if schedules.remove(schedule_id.as_str()).is_none() {
                return Err(ScheduleAdminError::NotFound {
                    schedule_id: schedule_id.clone(),
                });
            }
            Ok(())
        }
    }

    fn admin_state() -> (RunApiState, Arc<TestAdminPort>) {
        let admin = Arc::new(TestAdminPort::default());
        let port: Arc<dyn ScheduleAdminPort> = Arc::clone(&admin) as _;
        (RunApiState::new().with_schedules(port), admin)
    }

    fn admin_principal() -> Extension<Principal> {
        Extension(Principal {
            id: "admin".to_string(),
            role: paladin_core::platform::container::user::UserRole::Admin,
        })
    }

    fn user_principal() -> Extension<Principal> {
        Extension(Principal {
            id: "user".to_string(),
            role: paladin_core::platform::container::user::UserRole::User,
        })
    }

    fn create_request() -> CreateScheduleRequest {
        CreateScheduleRequest {
            assistant_id: "a1".to_string(),
            version: None,
            cron: "*/5 * * * *".to_string(),
            timezone: None,
            input: serde_json::json!({}),
            enabled: true,
            thread_strategy: None,
            on_missed: None,
            webhook: None,
        }
    }

    async fn read_json(response: axum::response::Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn post_schedules_returns_501_when_unwired() {
        let app = run_router(RunApiState::new());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/schedules")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&serde_json::json!({
                            "assistant_id": "a1",
                            "cron": "*/5 * * * *",
                            "enabled": true
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn post_schedules_creates_and_returns_201() {
        let (state, _admin) = admin_state();
        let response = create_schedule(State(state), admin_principal(), Json(create_request()))
            .await
            .unwrap();
        assert_eq!(response.0, StatusCode::CREATED);
    }

    #[tokio::test]
    async fn post_schedules_non_admin_is_403() {
        let (state, _admin) = admin_state();
        let err = create_schedule(State(state), user_principal(), Json(create_request()))
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn post_schedules_invalid_is_400_with_violations() {
        let (state, _admin) = admin_state();
        let mut request = create_request();
        request.cron = String::new();
        let err = create_schedule(State(state), admin_principal(), Json(request))
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
        let body = err.to_body();
        let violations = body["error"]["details"]["violations"].as_array().unwrap();
        assert!(!violations.is_empty());
    }

    #[tokio::test]
    async fn get_schedule_unknown_is_404() {
        let (state, _admin) = admin_state();
        let err = get_schedule(
            State(state),
            user_principal(),
            Path(RunScheduleId::new_v7().to_string()),
        )
        .await
        .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_schedule_found_returns_200_with_ticks() {
        let (state, _admin) = admin_state();
        let created = create_schedule(
            State(state.clone()),
            admin_principal(),
            Json(create_request()),
        )
        .await
        .unwrap();
        let created_body = read_json(created.1.into_response()).await;
        let schedule_id = created_body["schedule_id"].as_str().unwrap().to_string();

        let response = get_schedule(State(state), user_principal(), Path(schedule_id))
            .await
            .unwrap();
        assert_eq!(response.0, StatusCode::OK);
        let body = read_json(response.1.into_response()).await;
        assert!(body.get("next_tick").is_some());
        assert!(body.get("last_tick").is_some());
        assert!(body.get("skipped_ticks").is_some());
    }

    #[tokio::test]
    async fn patch_schedule_updates_and_returns_200() {
        let (state, _admin) = admin_state();
        let created = create_schedule(
            State(state.clone()),
            admin_principal(),
            Json(create_request()),
        )
        .await
        .unwrap();
        let created_body = read_json(created.1.into_response()).await;
        let schedule_id = created_body["schedule_id"].as_str().unwrap().to_string();

        let patched = patch_schedule(
            State(state),
            admin_principal(),
            Path(schedule_id),
            Json(PatchScheduleRequest {
                cron: Some("0 * * * *".to_string()),
                timezone: None,
                input: None,
                enabled: None,
                thread_strategy: None,
                on_missed: None,
                webhook: None,
            }),
        )
        .await
        .unwrap();
        assert_eq!(patched.0, StatusCode::OK);
        let body = read_json(patched.1.into_response()).await;
        assert_eq!(body["cron"], "0 * * * *");
    }

    #[tokio::test]
    async fn patch_schedule_non_admin_is_403() {
        let (state, _admin) = admin_state();
        let err = patch_schedule(
            State(state),
            user_principal(),
            Path(RunScheduleId::new_v7().to_string()),
            Json(PatchScheduleRequest {
                cron: None,
                timezone: None,
                input: None,
                enabled: None,
                thread_strategy: None,
                on_missed: None,
                webhook: None,
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(err.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn patch_schedule_unknown_is_404() {
        let (state, _admin) = admin_state();
        let err = patch_schedule(
            State(state),
            admin_principal(),
            Path(RunScheduleId::new_v7().to_string()),
            Json(PatchScheduleRequest {
                cron: None,
                timezone: None,
                input: None,
                enabled: None,
                thread_strategy: None,
                on_missed: None,
                webhook: None,
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn delete_schedule_then_get_is_404() {
        let (state, _admin) = admin_state();
        let created = create_schedule(
            State(state.clone()),
            admin_principal(),
            Json(create_request()),
        )
        .await
        .unwrap();
        let created_body = read_json(created.1.into_response()).await;
        let schedule_id = created_body["schedule_id"].as_str().unwrap().to_string();

        let deleted = delete_schedule(
            State(state.clone()),
            admin_principal(),
            Path(schedule_id.clone()),
        )
        .await
        .unwrap();
        assert_eq!(deleted, StatusCode::NO_CONTENT);

        let err = get_schedule(State(state), user_principal(), Path(schedule_id))
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn delete_schedule_non_admin_is_403() {
        let (state, _admin) = admin_state();
        let err = delete_schedule(
            State(state),
            user_principal(),
            Path(RunScheduleId::new_v7().to_string()),
        )
        .await
        .unwrap_err();
        assert_eq!(err.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn webhook_secret_is_redacted_in_response() {
        let (state, _admin) = admin_state();
        let mut request = create_request();
        request.webhook = Some(WebhookRequestDto {
            url: "https://example.com/hook".to_string(),
            secret: Some("super-secret".to_string()),
            events: vec!["completed".to_string()],
        });
        let response = create_schedule(State(state), admin_principal(), Json(request))
            .await
            .unwrap();
        let body = read_json(response.1.into_response()).await;
        assert_eq!(body["webhook"]["secret"], "***");
        assert!(!body.to_string().contains("super-secret"));
    }

    #[tokio::test]
    async fn list_schedules_returns_created_items() {
        let (state, _admin) = admin_state();
        let _ = create_schedule(
            State(state.clone()),
            admin_principal(),
            Json(create_request()),
        )
        .await
        .unwrap();

        let response = list_schedules(
            State(state),
            user_principal(),
            Query(PageQuery {
                limit: None,
                cursor: None,
            }),
        )
        .await
        .unwrap();
        assert_eq!(response.0, StatusCode::OK);
        let body = read_json(response.1.into_response()).await;
        assert_eq!(body["items"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn schedule_routes_require_authentication() {
        let auth = crate::agent_auth::AgentAuthConfig {
            enabled: true,
            api_keys: HashMap::new(),
            token_verifier: None,
        };
        let state = RunApiState::new().with_auth(auth);
        let app = run_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/schedules/{}", RunScheduleId::new_v7()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn schedule_openapi_router_contains_schedule_paths() {
        let state = RunApiState::new();
        let (_router, api) = crate::run_controller::run_openapi_router(state).split_for_parts();
        for expected in ["/schedules", "/schedules/{schedule_id}"] {
            assert!(
                api.paths.paths.contains_key(expected),
                "missing path {expected}: {:?}",
                api.paths.paths.keys().collect::<Vec<_>>()
            );
        }
        let patch_present = api
            .paths
            .paths
            .get("/schedules/{schedule_id}")
            .map(|item| item.patch.is_some())
            .unwrap_or(false);
        assert!(
            patch_present,
            "PATCH /schedules/{{schedule_id}} must be registered"
        );
    }
}
