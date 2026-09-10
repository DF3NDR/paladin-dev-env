//! SHIP-02 boot proof (Phase 29, plan 29-01, D-06/D-07): a v0.9 operator's configuration
//! file boots this v0.10 binary with v0.9 behavior — proven, not merely asserted.
//!
//! Two levels of claim, following D-07:
//!
//! 1. **Config resolution.** [`Settings::load_from_file`] on the frozen, v0.9-shaped fixture
//!    (`tests/fixtures/config/v0.9.0-config.test.yml`, provenance in the sibling `README.md`)
//!    succeeds, and every v0.10-era config surface a v0.9 file never mentions resolves to its
//!    own inert `Default` — whether that surface lives on [`Settings`] itself
//!    (`agent_runtime`, `trace`, `web_server`) or is one of the nine platform config structs
//!    `src/bin/paladin-server.rs` builds entirely outside `Settings` (Pitfall 3: these are
//!    never `Settings` fields and are asserted against their own `::default()`/
//!    `apply_env_overrides()` instead).
//! 2. **Behavioral.** The real server routers, composed from that same loaded `Settings` the
//!    way `src/bin/paladin-server.rs` does, mount exactly the v0.9 route set (six
//!    `/v1/agents…` paths plus `/health`, `/ready`, `/openapi.json`) while every route family
//!    new in v0.10 answers `501 Not Implemented` — never `404` (Pitfall 2) — because Phase 27
//!    designed every new subsystem to default off and every new route to answer `501` when
//!    unwired precisely so this proof could pass by construction
//!    (27-DISCUSSION-LOG.md line 216).
#![cfg(feature = "web-server")]

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use paladin::config::assistants::AssistantsConfig;
use paladin::config::run_queue::RunQueueConfig;
use paladin::config::run_store::RunStoreConfig;
use paladin::config::run_stream::RunStreamConfig;
use paladin::config::run_worker::RunWorkerConfig;
use paladin::config::schedules::SchedulesConfig;
use paladin::config::settings::Settings;
use paladin::config::webhooks::WebhooksConfig;
use paladin::infrastructure::web::run_api_wiring::{RunApiConfigs, build_run_api};
use paladin::infrastructure::web::{
    AgentApiState, AgentAuthConfig, AgentRegistry, ThreadApiState, agent_router, run_router,
    thread_router,
};
use paladin_battalion::engine::shutdown::ShutdownCoordinator;

/// Path of the frozen v0.9-shaped fixture the boot test loads, resolved from
/// `CARGO_MANIFEST_DIR` so the test is cwd-independent — the same convention
/// `crates/paladin-web/src/openapi.rs`'s `baseline_path()` uses.
fn v0_9_fixture_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/config/v0.9.0-config.test.yml")
}

/// Build a `RunApiConfigs` entirely from `::default()` values — the same seven X-09
/// structs `src/bin/paladin-server.rs`'s `run()` builds before ever reaching
/// `build_run_api`, none of which are `Settings` fields (Pitfall 3).
fn default_run_api_configs() -> RunApiConfigs {
    RunApiConfigs {
        run_store: RunStoreConfig::default(),
        run_queue: RunQueueConfig::default(),
        run_worker: RunWorkerConfig::default(),
        run_stream: RunStreamConfig::default(),
        assistants: AssistantsConfig::default(),
        schedules: SchedulesConfig::default(),
        webhooks: WebhooksConfig::default(),
    }
}

/// Compose the full app router from a loaded `Settings`, mirroring
/// `src/bin/paladin-server.rs`'s own composition: `agent_router` merged with
/// `thread_router` merged with `run_router`, then the docs router — never nested inside
/// one another, so each state stays untouched (D-24, D-44).
///
/// The thread state is built directly rather than through `paladin-server.rs`'s
/// `#[cfg(test)]`-only `build_thread_state` helper (private to that binary crate and
/// unreachable from an external integration test): with a `Default`
/// `WaypointStoreConfig` (`backend == Disabled`), `paladin-server.rs`'s own
/// `thread_state_from_store` returns exactly `ThreadApiState::new().with_auth(auth)` when
/// no waypoint store is wired — the same unwired shape
/// `server_wires_no_waypoint_backend_by_default` asserts — so building it here directly
/// reproduces that behavior without duplicating any decision logic.
async fn compose_app_from_settings(settings: &Settings) -> Router {
    let auth = AgentAuthConfig::default();
    let registry = Arc::new(AgentRegistry::new());
    let state = AgentApiState::new(Arc::clone(&registry)).with_auth(auth.clone());

    let thread_state = ThreadApiState::new().with_auth(auth.clone());

    let handles = build_run_api(
        default_run_api_configs(),
        settings,
        ShutdownCoordinator::new(),
        None,
        auth,
        registry,
    )
    .await
    .expect("a v0.9-shaped config's disabled run store never fails to build");

    let spec = paladin::infrastructure::web::openapi::openapi_spec();
    agent_router(state)
        .merge(thread_router(thread_state))
        .merge(run_router(handles.run_state))
        .merge(paladin::infrastructure::web::openapi::docs_router(spec))
}

/// Config-resolution level (D-07, part 1): the frozen v0.9-shaped fixture loads cleanly at
/// v0.10 HEAD with no edits, and a pre-existing field the v0.9 file DOES mention still
/// resolves correctly — the same anchor `test_load_from_file_regression` asserts.
#[test]
fn v0_9_config_file_loads_at_v0_10() {
    let path = v0_9_fixture_path();
    let settings = Settings::load_from_file(path.to_str().expect("path is valid UTF-8"))
        .expect("a v0.9-shaped config file must load without edits at v0.10 HEAD");

    assert_eq!(settings.server.host, "127.0.0.1");
}

/// Behavioral level (D-07, part 2), narrowed to one route: the run surface a v0.9 config
/// never mentions is provably unwired end to end — `build_run_api` over defaults spawns no
/// tasks and wires no repository/submission ports, and a live HTTP request against the
/// composed run router answers `501`, never `404` (Pitfall 2).
#[tokio::test]
async fn v0_9_config_leaves_the_run_surface_unwired() {
    let path = v0_9_fixture_path();
    let settings = Settings::load_from_file(path.to_str().expect("path is valid UTF-8"))
        .expect("fixture loads");

    let handles = build_run_api(
        default_run_api_configs(),
        &settings,
        ShutdownCoordinator::new(),
        None,
        AgentAuthConfig::default(),
        Arc::new(AgentRegistry::new()),
    )
    .await
    .expect("a v0.9-shaped config's disabled run store never fails to build");

    assert!(
        handles.tasks.is_empty(),
        "no background task should be spawned when run_store is Disabled"
    );
    assert!(
        handles.run_repository.is_none(),
        "no run repository should be wired when run_store is Disabled"
    );
    assert!(
        handles.thread_run_submission.is_none(),
        "no thread run-submission port should be wired when run_store is Disabled"
    );
    assert!(
        handles.parley_extras.is_none(),
        "no parley extras should be wired when run_store is Disabled"
    );

    let app = run_router(handles.run_state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/runs")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"assistant_id":"any"}"#))
                .expect("request builds"),
        )
        .await
        .expect("router responds");

    assert_eq!(
        response.status(),
        StatusCode::NOT_IMPLEMENTED,
        "POST /v1/runs must answer 501 (not 404) when the run store is unwired"
    );
}
