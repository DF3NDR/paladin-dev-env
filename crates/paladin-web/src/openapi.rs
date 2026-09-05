//! OpenAPI spec assembly and interactive docs serving (Milestone 12, Epic 6).
//!
//! The spec is **derived from the handlers** (`#[utoipa::path]`) and DTOs (`ToSchema`) via
//! `utoipa-axum`, so the served API and the published contract come from one source.
//! `build_openapi` assembles the `/v1` agent API document and decorates it with API info
//! and the two security schemes (API key + opaque bearer token); `docs_router` serves it at
//! `GET /openapi.json` with a Swagger UI at `/docs`.
//!
//! Exposure is gated by the binary on `http.docs.enabled` — when disabled, the docs router
//! is simply not mounted (both routes `404`). The docs endpoints are unversioned and
//! unauthenticated: a consumer needs the contract before they hold credentials, and the
//! spec describes shapes, never secret values.

use std::sync::Arc;

use axum::Router;
use utoipa::openapi::OpenApi;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa_swagger_ui::SwaggerUi;

use crate::agent_controller::{AgentApiState, versioned_agent_parts};
use crate::agent_registry::AgentRegistry;
use crate::thread_controller::{ThreadApiState, versioned_thread_parts};

/// Security-scheme name for the `X-API-Key` header credential (matches the handler annotations).
pub const SEC_API_KEY: &str = "api_key";
/// Security-scheme name for the `Authorization: Bearer` opaque server-issued token credential.
pub const SEC_BEARER_TOKEN: &str = "bearer_token";

/// Path at which the raw OpenAPI document is served.
pub const OPENAPI_JSON_PATH: &str = "/openapi.json";
/// Base path at which the Swagger UI is served.
pub const DOCS_PATH: &str = "/docs";

/// Decorate a generated document with API info and the security schemes.
fn decorate(api: &mut OpenApi) {
    api.info.title = "Paladin Agent API".to_string();
    api.info.version = env!("CARGO_PKG_VERSION").to_string();
    api.info.description = Some(
        "HTTP API for executing and managing resident Paladin agents. \
         Agent routes are served under `/v1`; `/health`, `/ready`, and the docs are unversioned."
            .to_string(),
    );

    let components = api.components.get_or_insert_with(Default::default);
    components.add_security_scheme(
        SEC_API_KEY,
        SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-API-Key"))),
    );
    components.add_security_scheme(
        SEC_BEARER_TOKEN,
        SecurityScheme::Http(HttpBuilder::new().scheme(HttpAuthScheme::Bearer).build()),
    );
}

/// Build the decorated OpenAPI document for the agent + thread APIs (paths under `/v1`).
///
/// The `state` only shapes the (discarded) agent router; the document depends solely on
/// the handler annotations, so any state — including an empty one — yields the same spec.
/// The thread paths (`crate::thread_controller`, HITL-05, D-24) are always merged in from a
/// throwaway, unwired [`ThreadApiState`] — D-24 requires the spec to list them regardless of
/// whether a waypoint backend is actually configured in the process building this document,
/// since the paths and their `#[utoipa::path]` annotations (including the `501` response) are
/// static, independent of any runtime state.
pub fn build_openapi(state: AgentApiState) -> OpenApi {
    let (_router, mut api) = versioned_agent_parts(state);
    let (_thread_router, thread_api) = versioned_thread_parts(ThreadApiState::new());
    api.merge(thread_api);
    decorate(&mut api);
    api
}

/// Build the decorated OpenAPI document using a throwaway empty state.
///
/// Convenience for the drift guard and `/openapi.json` serving where no live state is at
/// hand.
pub fn openapi_spec() -> OpenApi {
    build_openapi(AgentApiState::new(Arc::new(AgentRegistry::new())))
}

/// Router serving the spec at [`OPENAPI_JSON_PATH`] and Swagger UI at [`DOCS_PATH`].
///
/// Merge into the application router only when docs are enabled.
pub fn docs_router(spec: OpenApi) -> Router {
    Router::new().merge(SwaggerUi::new(DOCS_PATH).url(OPENAPI_JSON_PATH, spec))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_has_info_and_security_schemes() {
        let api = openapi_spec();
        assert_eq!(api.info.title, "Paladin Agent API");
        let schemes = &api
            .components
            .as_ref()
            .expect("components present")
            .security_schemes;
        assert!(schemes.contains_key(SEC_API_KEY), "missing api_key scheme");
        assert!(
            schemes.contains_key(SEC_BEARER_TOKEN),
            "missing bearer_token scheme"
        );
    }

    #[test]
    fn spec_paths_are_versioned_under_v1() {
        let api = openapi_spec();
        let paths = &api.paths.paths;
        assert!(
            paths.contains_key("/v1/agents"),
            "paths: {:?}",
            paths.keys().collect::<Vec<_>>()
        );
        assert!(paths.contains_key("/v1/agents/{id}/execute"));
        assert!(!paths.contains_key("/agents"));
    }

    /// Test 1 (Phase 24 Plan 11, D-24/D-27): the built spec lists all three
    /// thread paths under `/v1`, regardless of whether a waypoint backend
    /// is wired in the process building the document.
    #[test]
    fn openapi_lists_the_three_thread_paths() {
        let api = openapi_spec();
        let paths = &api.paths.paths;
        for expected in [
            "/v1/threads/{id}/state",
            "/v1/threads/{id}/resume",
            "/v1/threads/{id}/history",
        ] {
            assert!(
                paths.contains_key(expected),
                "paths: {:?}",
                paths.keys().collect::<Vec<_>>()
            );
        }
    }

    /// Test 2: each thread path documents its full status set, including
    /// `501` for the unwired case (every path), `403` on the resume path for
    /// the admin gate (`require_admin`, narrows D-24 pending PLAT-06), and
    /// both `409` codes on the resume path (as a single response entry
    /// naming both, since OpenAPI's per-status-code response map cannot hold
    /// two distinct entries under the identical `409` key).
    #[test]
    fn openapi_thread_paths_document_every_status() {
        let api = openapi_spec();

        let state_path = api
            .paths
            .paths
            .get("/v1/threads/{id}/state")
            .expect("state path present");
        let get_op = state_path.get.as_ref().expect("GET operation");
        for status in ["200", "400", "401", "404", "501"] {
            assert!(
                get_op.responses.responses.contains_key(status),
                "GET /threads/{{id}}/state missing {status}"
            );
        }

        let resume_path = api
            .paths
            .paths
            .get("/v1/threads/{id}/resume")
            .expect("resume path present");
        let post_op = resume_path.post.as_ref().expect("POST operation");
        for status in ["202", "400", "401", "403", "404", "409", "501"] {
            assert!(
                post_op.responses.responses.contains_key(status),
                "POST /threads/{{id}}/resume missing {status}"
            );
        }
        let conflict = post_op
            .responses
            .responses
            .get("409")
            .expect("409 documented");
        let utoipa::openapi::RefOr::T(conflict) = conflict else {
            panic!("expected an inline 409 response, not a $ref");
        };
        assert!(
            conflict.description.contains("thread_not_awaiting_input")
                && conflict.description.contains("graph_not_registered"),
            "the 409 response must document BOTH distinct codes: {}",
            conflict.description
        );

        let history_path = api
            .paths
            .paths
            .get("/v1/threads/{id}/history")
            .expect("history path present");
        let get_op = history_path.get.as_ref().expect("GET operation");
        for status in ["200", "400", "401", "501"] {
            assert!(
                get_op.responses.responses.contains_key(status),
                "GET /threads/{{id}}/history missing {status}"
            );
        }
    }

    /// Test 4: adding the thread paths must not change a single pre-existing
    /// agent path's operation -- computed by comparing the combined spec
    /// against an agent-only spec built the same way `build_openapi` did
    /// before the thread paths existed.
    #[test]
    fn openapi_pre_existing_agent_paths_are_unchanged() {
        let agent_only = {
            let (_router, mut api) =
                versioned_agent_parts(AgentApiState::new(Arc::new(AgentRegistry::new())));
            decorate(&mut api);
            api
        };
        let combined = openapi_spec();

        for key in [
            "/v1/agents",
            "/v1/agents/{id}",
            "/v1/agents/{id}/execute",
            "/v1/agents/{id}/execute/stream",
            "/v1/agents/{id}/jobs",
            "/v1/agents/{id}/jobs/{job_id}",
        ] {
            // `PathItem` does not implement `Debug` outside the `debug`
            // feature -- compare via JSON (the same representation the
            // committed `openapi.json` drift guard already trusts).
            let before = serde_json::to_value(agent_only.paths.paths.get(key))
                .expect("serialize pre-existing path");
            let after = serde_json::to_value(combined.paths.paths.get(key))
                .expect("serialize combined path");
            assert_eq!(
                before, after,
                "path {key} drifted after adding the thread routes"
            );
        }
    }

    use crate::agent_controller::agent_router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    /// Path of the committed spec baseline (`crates/paladin-web/openapi.json`).
    fn baseline_path() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("openapi.json")
    }

    /// Drift guard: the generated spec must match the committed `openapi.json` baseline.
    ///
    /// Regenerate after an intentional API change with:
    /// `UPDATE_OPENAPI=1 cargo test -p paladin-web openapi_matches_committed_baseline`
    /// (or `make openapi`).
    #[test]
    fn openapi_matches_committed_baseline() {
        let generated = openapi_spec().to_pretty_json().expect("serialize spec");
        let path = baseline_path();

        if std::env::var_os("UPDATE_OPENAPI").is_some() {
            std::fs::write(&path, format!("{generated}\n")).expect("write baseline");
            return;
        }

        let baseline = std::fs::read_to_string(&path).unwrap_or_default();
        assert_eq!(
            generated.trim(),
            baseline.trim(),
            "OpenAPI spec drifted from {}. If the change is intentional, regenerate with: \
             UPDATE_OPENAPI=1 cargo test -p paladin-web openapi_matches_committed_baseline",
            path.display()
        );
    }

    #[tokio::test]
    async fn docs_router_serves_spec_and_ui() {
        let app = docs_router(openapi_spec());

        let spec = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(OPENAPI_JSON_PATH)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(spec.status(), StatusCode::OK);

        // Swagger UI index (the bundle redirects `/docs` → `/docs/`).
        let ui = app
            .oneshot(
                Request::builder()
                    .uri("/docs/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(
            ui.status().is_success() || ui.status().is_redirection(),
            "unexpected /docs status: {}",
            ui.status()
        );
    }

    #[tokio::test]
    async fn without_docs_router_spec_is_404_but_api_works() {
        // Mirrors the binary when `http.docs.enabled = false`: no docs routes mounted.
        let state = AgentApiState::new(Arc::new(AgentRegistry::new()));
        let app = agent_router(state);

        let spec = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(OPENAPI_JSON_PATH)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(spec.status(), StatusCode::NOT_FOUND);

        let health = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(health.status(), StatusCode::OK);
    }
}
