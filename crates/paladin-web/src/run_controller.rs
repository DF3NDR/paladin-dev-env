//! Axum HTTP controller for submitting and reading runs (PLAT-01, D-44).
//!
//! This module mirrors [`crate::thread_controller`]'s conventions on a
//! router and state struct of its own, so [`crate::agent_controller::AgentApiState`]
//! and [`crate::thread_controller::ThreadApiState`] stay untouched (X-10.3):
//!
//! | Method & path | Description |
//! |---------------|-------------|
//! | `POST /runs` | Submit a run; `202 Accepted` with `{ run_id, thread_id, state_url }` |
//! | `GET /runs/{run_id}` | The run's current status |
//!
//! [`RunApiState`] holds `Option<Arc<dyn RunSubmissionPort>>` and
//! `Option<Arc<dyn RunRepositoryPort>>` -- both `paladin-ports` trait
//! objects, never a `paladin-battalion` type -- so this crate takes no
//! dependency on `paladin-battalion` in its default build (ADR-0031). When
//! either field is `None`, every route in this module answers `501
//! not_implemented` naming the config key to set, per D-44 (the D-24
//! precedent).
//!
//! A success body is the serialized payload; failures use the unified
//! [`ApiError`](crate::error::ApiError) envelope.
//!
//! ## Authorization
//!
//! Both routes are behind the same `require_authentication` middleware as
//! `/v1/agents/*` and `/v1/threads/*` (D-44); scope enforcement beyond
//! authentication is completed in a later plan (PLAT-06). The `Principal`
//! is extracted (its role attached to the submitted run's `requested_by`)
//! so per-assistant `allowed_roles` scoping can land as a body-only change.

use std::sync::Arc;

use axum::Extension;
use axum::extract::{Json, Path, State};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use paladin_core::platform::container::run::{Run, RunId};
use paladin_core::platform::container::waypoint::ThreadId;
use paladin_ports::input::run_submission_port::{RunSubmissionError, RunSubmissionPort, SubmitRun};
use paladin_ports::output::run_repository_port::RunRepositoryPort;

use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::agent_auth::{HasAgentAuth, Principal};
use crate::agent_controller::{API_V1_PREFIX, JsonValue, ok_body};
use crate::error::{ApiError, ApiErrorBody};

const SUBMISSION_PORT_HINT: &str =
    "no run submission backend configured: set run_store.backend and run_queue.backend";
const REPOSITORY_PORT_HINT: &str = "no run store backend configured: set run_store.backend";

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
    /// Authentication configuration -- the SAME [`crate::agent_auth::AgentAuthConfig`]
    /// shape every other stateful router in this crate carries.
    pub auth: crate::agent_auth::AgentAuthConfig,
}

impl RunApiState {
    /// Construct state with nothing wired (every route answers `501` until
    /// [`Self::with_submission`]/[`Self::with_repository`] are applied).
    pub fn new() -> Self {
        Self {
            run_submission: None,
            run_repository: None,
            auth: crate::agent_auth::AgentAuthConfig::default(),
        }
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

    /// Set the authentication configuration.
    pub fn with_auth(mut self, auth: crate::agent_auth::AgentAuthConfig) -> Self {
        self.auth = auth;
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
        }
    }
}

// --- Parsing helpers --------------------------------------------------------

fn parse_run_id(raw: &str) -> Result<RunId, ApiError> {
    RunId::parse(raw).map_err(|e| ApiError::bad_request(e.to_string()))
}

fn parse_thread_id(raw: &str) -> Result<ThreadId, ApiError> {
    ThreadId::new(raw).map_err(|e| ApiError::bad_request(e.to_string()))
}

// --- Error mapping -----------------------------------------------------

/// Map a [`RunSubmissionError`] onto the [`ApiError`] status/code this
/// route uses. `#[non_exhaustive]`, so a future variant renders `500
/// internal` rather than failing to compile.
fn map_submission_error(err: RunSubmissionError) -> ApiError {
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
            format!("thread '{thread_id}' already has an active run"),
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
        (status = 400, description = "Invalid thread id", body = ApiErrorBody),
        (status = 401, description = "Missing/invalid credentials", body = ApiErrorBody),
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

    let request = SubmitRun {
        assistant_id: body.assistant_id,
        version: body.version,
        thread_id,
        input: body.input,
        webhook: None,
        requested_by: Some((principal.id.clone(), principal.role)),
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
    Extension(_principal): Extension<Principal>,
    Path(run_id): Path<String>,
) -> Result<(StatusCode, JsonValue), ApiError> {
    let repository = state
        .run_repository
        .as_ref()
        .ok_or_else(|| ApiError::not_implemented(REPOSITORY_PORT_HINT))?;
    let id = parse_run_id(&run_id)?;

    let run = repository
        .get(&id)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or_else(|| ApiError::not_found(format!("unknown run '{run_id}'")))?;

    Ok((StatusCode::OK, ok_body(&RunResponse::from(&run))))
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
        .routes(routes!(get_run))
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
    use paladin_core::platform::container::parley::ParleyResponse;
    use paladin_core::platform::container::run::{AssistantRef, RunStatus};
    use paladin_ports::output::run_repository_port::{
        RunOutcomeRecord, RunPage, RunQuery, RunRepositoryError,
    };
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
            }
        }
    }

    // --- Mock `RunRepositoryPort` ----------------------------------------

    #[derive(Default)]
    struct MockRepository {
        runs: Mutex<HashMap<String, Run>>,
    }

    impl MockRepository {
        fn seed(&self, run: Run) {
            self.runs
                .lock()
                .unwrap()
                .insert(run.run_id.as_str().to_string(), run);
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

        async fn list(&self, _query: RunQuery) -> Result<RunPage, RunRepositoryError> {
            Ok(RunPage {
                items: vec![],
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
        for expected in ["/runs", "/runs/{run_id}"] {
            assert!(
                api.paths.paths.contains_key(expected),
                "missing path {expected}: {:?}",
                api.paths.paths.keys().collect::<Vec<_>>()
            );
        }
    }
}
