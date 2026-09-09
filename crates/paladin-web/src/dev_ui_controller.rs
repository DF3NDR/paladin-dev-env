//! Feature-gated (`dev-ui`, default off) admin developer tool (28-15, D-25/D-26):
//! `GET /v1/dev-ui/threads/{id}` renders one thread's
//! [`RunInspectorPort`] view as a single static HTML page -- the execution overlay
//! diagram, a per-node visits panel, a per-superstep fired-edge list and a superstep
//! table with field-change NAMES only (never a Battlefield field value, T-28-14-01).
//!
//! ## Feature-gate and API-surface independence (D-25)
//!
//! This module is compiled only under `#[cfg(feature = "dev-ui")]` (the crate's first
//! cargo feature, declared from scratch -- `crates/paladin-web/Cargo.toml`). The route
//! carries no OpenAPI path annotation and is built as a plain [`axum::Router`] (never
//! the OpenAPI-aware router type the agent/thread/run controllers use), so it is
//! structurally incapable of entering `crates/paladin-web/openapi.json` regardless of
//! whether this feature is compiled -- the drift-guard (`crate::openapi::build_openapi`)
//! assembles the document from exactly three named router sources (agent, thread, run),
//! none of which this module is.
//!
//! ## Authorization (D-25)
//!
//! `crate::app::create_dev_ui_router` mounts [`dev_ui_inspector_page`] under the SAME
//! [`crate::auth_middleware::require_auth`] + [`crate::auth_middleware::require_admin`]
//! middleware layers `create_app_router`'s admin routes already use -- this port neither
//! performs nor implies an authorization boundary of its own (see
//! [`paladin_ports::input::run_inspector_port`]'s own module docs).
//!
//! ## No values-shown mode (T-28-15-03)
//!
//! [`InspectorView::supersteps`]'s `field_changes` is `Vec<FieldName>` by TYPE (28-14) --
//! there is nowhere on the view a Battlefield field VALUE could be placed even by
//! mistake, and this module never introduces one.

use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::Response;

use paladin_core::platform::container::waypoint::ThreadId;
use paladin_ports::input::run_inspector_port::{InspectorError, RunInspectorPort};

use crate::error::ApiError;

/// The static page template, embedded at compile time (D-26: no build pipeline).
const INSPECTOR_TEMPLATE: &str = include_str!("dev_ui/inspector.html");

/// Substitution marker for the embedded `InspectorView` JSON payload inside
/// [`INSPECTOR_TEMPLATE`]'s `#inspector-data` `<script type="application/json">` element.
/// Never appears in a real payload -- it shares no syntax with valid JSON.
const INSPECTOR_DATA_MARKER: &str = "__INSPECTOR_DATA__";

/// Substitution marker for the configured Mermaid ESM module URL (`mermaid_url`,
/// `web_server.dev_ui.mermaid_url` upstream) inside [`INSPECTOR_TEMPLATE`]'s
/// `#dev-ui-mermaid-config` element -- substituted by the handler, never hardcoded in the
/// template, so an air-gapped operator's configured local mirror is honored without a
/// crate rebuild.
const MERMAID_URL_MARKER: &str = "__MERMAID_URL__";

/// Copywriting Contract (28-UI-SPEC.md) — error state, inspector not wired (`501`).
/// Matches the existing `NotWired` → `501` precedent
/// ([`paladin_ports::input::run_event_stream_port::RunStreamError::NotWired`], D-25).
const NOT_WIRED_MESSAGE: &str = "Inspector not available. This server was not built with a \
     run inspector. Enable the `dev-ui` feature and wire a `RunInspectorPort` to use this page.";

/// Shared state for the feature-gated dev-ui inspector route (D-25).
///
/// Mirrors [`crate::thread_controller::ThreadApiState`]'s injection-only shape:
/// [`Self::inspector`] is `None` until a [`RunInspectorPort`] backend is wired, at which
/// point every request answers `501` through [`NOT_WIRED_MESSAGE`] rather than panicking
/// or 404-ing.
#[derive(Clone)]
pub struct DevUiState {
    /// The facade this route renders. `None` when no inspector backend is configured.
    pub inspector: Option<Arc<dyn RunInspectorPort>>,
    /// The configured Mermaid ESM module URL, substituted into the served page
    /// (`web_server.dev_ui.mermaid_url`, 28-02) so an air-gapped operator's local mirror
    /// is honored without a crate rebuild (D-26, D-36).
    pub mermaid_url: String,
}

impl DevUiState {
    /// Construct an unwired state carrying only the configured Mermaid URL. Every
    /// request answers `501` until [`Self::with_inspector`] wires a backend.
    pub fn new(mermaid_url: impl Into<String>) -> Self {
        Self {
            inspector: None,
            mermaid_url: mermaid_url.into(),
        }
    }

    /// Wire a [`RunInspectorPort`] backend.
    pub fn with_inspector(mut self, inspector: Arc<dyn RunInspectorPort>) -> Self {
        self.inspector = Some(inspector);
        self
    }
}

/// Escape the sequences that would break out of the surrounding `<script>` element or
/// open an HTML comment (D-26, T-28-15-02): `</` (which would close the `<script>` tag
/// early) and `<!--` (which would open an HTML comment the parser then consumes past the
/// following `</script>`). Order-independent -- neither pattern's replacement introduces
/// an instance of the other.
fn escape_for_script(json: &str) -> String {
    json.replace("</", "<\\/").replace("<!--", "<\\!--")
}

/// Map [`InspectorError`] to the unified [`ApiError`] envelope (D-25).
///
/// `#[non_exhaustive]` on [`InspectorError`] forces the wildcard arm in this
/// downstream crate; a future variant renders as `500 internal` rather than failing to
/// compile.
fn map_inspector_error(id: &str, err: InspectorError) -> ApiError {
    match err {
        InspectorError::ThreadNotFound { .. } => ApiError::not_found(format!(
            "Thread not found. No thread exists with id `{id}`. Check the id and try again."
        )),
        InspectorError::NotWired => ApiError::not_implemented(NOT_WIRED_MESSAGE),
        InspectorError::Backend { message } => ApiError::internal(message),
        _ => ApiError::internal("unknown run inspector error"),
    }
}

/// `GET /v1/dev-ui/threads/{id}` -- render the inspector page for one thread (D-25,
/// D-26, OBS-03).
///
/// Mounted by [`crate::app::create_dev_ui_router`] under the same admin-gated layers the
/// crate's other admin routes use. Maps an unknown thread to `404` and an absent
/// inspector backend to `501`, both through the structured [`ApiError`] envelope; any
/// other backend failure is `500`. On success, returns `200` with a `text/html` body
/// embedding the serialized [`paladin_ports::input::run_inspector_port::InspectorView`]
/// (escaped per [`escape_for_script`]) and the configured Mermaid URL.
pub async fn dev_ui_inspector_page(
    State(state): State<DevUiState>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let inspector = state
        .inspector
        .as_ref()
        .ok_or_else(|| ApiError::not_implemented(NOT_WIRED_MESSAGE))?;

    let thread = ThreadId::new(&id).map_err(|e| ApiError::bad_request(e.to_string()))?;

    let view = inspector
        .inspect(&thread)
        .await
        .map_err(|e| map_inspector_error(&id, e))?;

    let json = serde_json::to_string(&view).map_err(|e| ApiError::internal(e.to_string()))?;
    let escaped_json = escape_for_script(&json);

    // JSON-string-escape the Mermaid URL (quotes included) so it substitutes into
    // `{"url": __MERMAID_URL__}` as a syntactically valid JSON string value, mirroring
    // the same "typed JSON script element" pattern the inspector payload itself uses --
    // never string-interpolated directly into a JS literal.
    let mermaid_url_json =
        serde_json::to_string(&state.mermaid_url).unwrap_or_else(|_| "\"\"".to_string());

    let page = INSPECTOR_TEMPLATE
        .replacen(INSPECTOR_DATA_MARKER, &escaped_json, 1)
        .replacen(MERMAID_URL_MARKER, &mermaid_url_json, 1);

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Body::from(page))
        .map_err(|e| ApiError::internal(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    use async_trait::async_trait;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use chrono::{Duration, Utc};
    use tower::ServiceExt; // for `Router::oneshot`
    use uuid::Uuid;

    use paladin_core::platform::container::run::{RunId, RunStatus};
    use paladin_core::platform::container::user::UserRole;
    use paladin_core::platform::container::waypoint::{NodeId, NodeOutcomeKind, WaypointId};
    use paladin_ports::input::run_inspector_port::{
        CompletedRow, InspectorSource, InspectorView, SuperstepRow, SuperstepStatus, VisitSummary,
    };
    use paladin_ports::output::auth_port::{AuthClaims, AuthError, AuthPort, AuthToken};

    use crate::app::create_dev_ui_router;

    // --- Mock `RunInspectorPort` -------------------------------------------------

    enum MockOutcome {
        View(InspectorView),
        NotFound,
    }

    struct MockInspector {
        outcome: MockOutcome,
    }

    #[async_trait]
    impl RunInspectorPort for MockInspector {
        async fn inspect(&self, thread: &ThreadId) -> Result<InspectorView, InspectorError> {
            match &self.outcome {
                MockOutcome::View(view) => Ok(view.clone()),
                MockOutcome::NotFound => Err(InspectorError::ThreadNotFound {
                    thread_id: thread.clone(),
                }),
            }
        }
    }

    fn node(id: &str) -> NodeId {
        NodeId::new(id)
    }

    fn thread(id: &str) -> ThreadId {
        ThreadId::new(id).unwrap()
    }

    fn completed_row(node_id: &str, outcome: NodeOutcomeKind) -> CompletedRow {
        CompletedRow {
            node_id: node(node_id),
            attempt: 1,
            outcome,
            duration_ms: Some(12),
            token_count: Some(34),
            cache_hit: false,
        }
    }

    /// The branching fixture view (PRD 07 acceptance 4): a node ("loop_node") visited at
    /// supersteps 2, 4 and 6 (the OBS-03 acceptance question), and a fired edge
    /// ("check" -> "retry") the assertion checks for verbatim.
    fn branching_fixture_view() -> InspectorView {
        InspectorView {
            thread_id: thread("t-branch"),
            run_id: Some(RunId::new_v7()),
            status: Some(RunStatus::Running),
            mermaid: "graph TD\ncheck --> retry\ncheck --> done".to_string(),
            observed_only: false,
            source: InspectorSource::Trace,
            supersteps: vec![
                SuperstepRow {
                    superstep: 2,
                    waypoint_id: WaypointId::generate(),
                    vanguard: vec![node("loop_node")],
                    completed: vec![completed_row("loop_node", NodeOutcomeKind::Succeeded)],
                    field_changes: vec![],
                    fired_edges: vec![(node("check"), node("retry"))],
                    evaluated_edges: vec![(node("check"), node("done"))],
                    status: SuperstepStatus::Running,
                },
                SuperstepRow {
                    superstep: 4,
                    waypoint_id: WaypointId::generate(),
                    vanguard: vec![node("loop_node")],
                    completed: vec![completed_row("loop_node", NodeOutcomeKind::Succeeded)],
                    field_changes: vec![],
                    fired_edges: vec![(node("check"), node("retry"))],
                    evaluated_edges: vec![(node("check"), node("done"))],
                    status: SuperstepStatus::Running,
                },
                SuperstepRow {
                    superstep: 6,
                    waypoint_id: WaypointId::generate(),
                    vanguard: vec![node("loop_node")],
                    completed: vec![completed_row("loop_node", NodeOutcomeKind::Ended)],
                    field_changes: vec![],
                    fired_edges: vec![],
                    evaluated_edges: vec![],
                    status: SuperstepStatus::Completed,
                },
            ],
            visits: vec![VisitSummary {
                node_id: node("loop_node"),
                count: 3,
                supersteps: vec![2, 4, 6],
            }],
        }
    }

    /// A view whose data contains the sequences that would terminate a `<script>` element
    /// or open an HTML comment, so [`escape_for_script`] has something real to escape.
    fn breakout_fixture_view() -> InspectorView {
        let mut view = branching_fixture_view();
        view.thread_id = thread("t-breakout");
        view.supersteps[0].field_changes = vec![]; // field NAMES only; nothing to break here
        view.mermaid = "graph TD\n</script><!--pwned-->check --> retry".to_string();
        view
    }

    fn state_with(outcome: MockOutcome) -> DevUiState {
        DevUiState::new("https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs")
            .with_inspector(Arc::new(MockInspector { outcome }))
    }

    // --- Mock `AuthPort` -----------------------------------------------------------

    struct MockAuthPort {
        claims: Option<AuthClaims>,
    }

    #[async_trait]
    impl AuthPort for MockAuthPort {
        async fn issue_token(
            &self,
            _user_id: Uuid,
            _role: UserRole,
        ) -> Result<AuthToken, AuthError> {
            Err(AuthError::Internal("not used".to_string()))
        }

        async fn verify_token(&self, _token: &str) -> Result<AuthClaims, AuthError> {
            self.claims.clone().ok_or(AuthError::InvalidToken)
        }

        async fn revoke_token(&self, _token: &str) -> Result<(), AuthError> {
            Ok(())
        }
    }

    fn admin_claims() -> AuthClaims {
        AuthClaims {
            user_id: Uuid::new_v4(),
            role: UserRole::Admin,
            expires_at: Utc::now() + Duration::hours(1),
        }
    }

    fn user_claims() -> AuthClaims {
        AuthClaims {
            user_id: Uuid::new_v4(),
            role: UserRole::User,
            expires_at: Utc::now() + Duration::hours(1),
        }
    }

    fn auth_port(claims: Option<AuthClaims>) -> Arc<dyn AuthPort> {
        Arc::new(MockAuthPort { claims })
    }

    fn admin_router(state: DevUiState) -> axum::Router {
        create_dev_ui_router(auth_port(Some(admin_claims())), state)
    }

    async fn body_string(response: axum::response::Response) -> String {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    // --- Task 1 <behavior> tests -----------------------------------------------------

    #[tokio::test]
    async fn dev_ui_page_returns_html_for_a_known_thread() {
        let app = admin_router(state_with(MockOutcome::View(branching_fixture_view())));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/dev-ui/threads/t-branch")
                    .header("Authorization", "Bearer admin-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert!(content_type.starts_with("text/html"));

        let body = body_string(response).await;
        assert!(body.contains(r#"id="inspector-data""#));
        assert!(body.contains(r#"type="application/json""#));
    }

    #[tokio::test]
    async fn dev_ui_page_embeds_the_fired_branch_and_the_three_visit_summary() {
        let app = admin_router(state_with(MockOutcome::View(branching_fixture_view())));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/dev-ui/threads/t-branch")
                    .header("Authorization", "Bearer admin-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;

        // The fired edge of the branching fixture.
        assert!(body.contains("\"check\""));
        assert!(body.contains("\"retry\""));
        // The three-visit summary: count == 3, supersteps == [2, 4, 6].
        assert!(body.contains("\"count\":3"));
        assert!(body.contains("\"supersteps\":[2,4,6]"));
        assert!(body.contains("\"loop_node\""));
    }

    #[tokio::test]
    async fn dev_ui_page_escapes_the_embedded_payload() {
        let app = admin_router(state_with(MockOutcome::View(breakout_fixture_view())));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/dev-ui/threads/t-breakout")
                    .header("Authorization", "Bearer admin-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;

        // The raw breakout sequences must never appear verbatim inside the script element.
        assert!(!body.contains("</script><!--pwned-->"));
        assert!(!body.contains("<!--pwned-->"));
        // The escaped forms must be present instead.
        assert!(body.contains("<\\/script>"));
        assert!(body.contains("<\\!--pwned-->"));
    }

    #[tokio::test]
    async fn dev_ui_unknown_thread_is_404() {
        let app = admin_router(state_with(MockOutcome::NotFound));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/dev-ui/threads/no-such-thread")
                    .header("Authorization", "Bearer admin-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = body_string(response).await;
        assert!(body.contains("\"code\":\"not_found\""));
        assert!(body.contains("Thread not found."));
    }

    #[tokio::test]
    async fn dev_ui_without_a_wired_port_is_501() {
        let state =
            DevUiState::new("https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs");
        let app = create_dev_ui_router(auth_port(Some(admin_claims())), state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/dev-ui/threads/any-thread")
                    .header("Authorization", "Bearer admin-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
        let body = body_string(response).await;
        assert!(body.contains("\"code\":\"not_implemented\""));
        assert!(body.contains("Inspector not available."));
    }

    #[tokio::test]
    async fn dev_ui_unauthenticated_request_is_rejected() {
        let app = create_dev_ui_router(
            auth_port(None),
            state_with(MockOutcome::View(branching_fixture_view())),
        );
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/dev-ui/threads/t-branch")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn dev_ui_authenticated_non_admin_request_is_rejected() {
        let app = create_dev_ui_router(
            auth_port(Some(user_claims())),
            state_with(MockOutcome::View(branching_fixture_view())),
        );
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/dev-ui/threads/t-branch")
                    .header("Authorization", "Bearer user-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
