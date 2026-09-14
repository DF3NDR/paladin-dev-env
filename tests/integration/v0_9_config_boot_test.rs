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

use std::env;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serial_test::serial;
use tower::ServiceExt;

use paladin::config::agent_runtime::AgentRuntimeConfig;
use paladin::config::assistants::AssistantsConfig;
use paladin::config::engine::EngineConfig;
use paladin::config::env_utils::EnvOverridable;
use paladin::config::run_queue::RunQueueConfig;
use paladin::config::run_store::{RunStoreBackend, RunStoreConfig};
use paladin::config::run_stream::RunStreamConfig;
use paladin::config::run_worker::RunWorkerConfig;
use paladin::config::schedules::SchedulesConfig;
use paladin::config::settings::Settings;
use paladin::config::trace::TraceConfig;
use paladin::config::waypoint_store::{WaypointStoreBackend, WaypointStoreConfig};
use paladin::config::web_server::WebServerConfig;
use paladin::config::webhooks::WebhooksConfig;
use paladin::infrastructure::web::run_api_wiring::{RunApiConfigs, build_run_api};
use paladin::infrastructure::web::{
    AgentApiState, AgentAuthConfig, AgentRegistry, ThreadApiState, agent_router, run_router,
    thread_router,
};
use paladin_battalion::engine::shutdown::ShutdownCoordinator;

/// Every `APP_*` variable read by any of the nine platform config structs'
/// `apply_env_overrides()` (26 total, enumerated in 29-01-PLAN.md's `<interfaces>` and
/// re-verified directly against each module's own `apply_env_overrides` body). Isolating
/// exactly this set — no more, no less — is what lets
/// `apply_env_overrides_is_a_no_op_with_no_app_vars` claim a CI runner's ambient
/// environment cannot make this proof diverge from a real v0.9 operator's file.
const ALL_PLATFORM_APP_ENV_VARS: &[&str] = &[
    "APP_ENGINE_GRACEFUL_SHUTDOWN",
    "APP_ENGINE_MAX_MUSTER_TASKS",
    "APP_ENGINE_MAX_NODE_VISITS",
    "APP_ENGINE_MAX_SUPERSTEPS",
    "APP_ENGINE_RUN_TIMEOUT_SECS",
    "APP_ENGINE_SHUTDOWN_GRACE_SECS",
    "APP_ENGINE_WAYPOINT_DURABILITY",
    "APP_WAYPOINT_STORE_BACKEND",
    "APP_WAYPOINT_STORE_POSTGRES_URL_ENV",
    "APP_WAYPOINT_STORE_SQLITE_PATH",
    "APP_RUN_STORE_BACKEND",
    "APP_RUN_STORE_PATH",
    "APP_RUN_STORE_URL_ENV",
    "APP_RUN_QUEUE_BACKEND",
    "APP_RUN_QUEUE_KEY_PREFIX",
    "APP_RUN_QUEUE_URL_ENV",
    "APP_RUN_WORKER_CONCURRENCY",
    "APP_RUN_WORKER_LEASE_SECONDS",
    "APP_RUN_WORKER_MIN_PROBE_INTERVAL_MS",
    "APP_RUN_STREAM_POLL_INTERVAL_MS",
    "APP_ASSISTANTS_EXPOSE_CODE_REGISTRY",
    "APP_SCHEDULES_ENABLED",
    "APP_SCHEDULES_TICK_INTERVAL_MS",
    "APP_WEBHOOKS_ALLOW_PRIVATE",
    "APP_WEBHOOKS_MAX_ATTEMPTS",
    "APP_WEBHOOKS_TIMEOUT_SECS",
];

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

/// Config-resolution level (D-07, part 1), the `Settings`-field half: every v0.10-era field
/// ON `Settings` itself that the v0.9 fixture never mentions resolves to its own `Default`.
/// `WebServerConfig` has no `PartialEq` (X-03 forbids adding one just for this test), so it
/// is compared through serialized JSON instead.
#[test]
fn v0_10_settings_fields_resolve_to_default() {
    let path = v0_9_fixture_path();
    let settings = Settings::load_from_file(path.to_str().expect("path is valid UTF-8"))
        .expect("fixture loads");

    assert_eq!(
        settings.agent_runtime,
        AgentRuntimeConfig::default(),
        "a v0.9 fixture never mentions agent_runtime: it must resolve to AgentRuntimeConfig::default()"
    );
    assert_eq!(
        settings.trace,
        TraceConfig::default(),
        "a v0.9 fixture never mentions trace: it must resolve to TraceConfig::default()"
    );
    assert_eq!(
        serde_json::to_value(&settings.web_server).expect("WebServerConfig serializes"),
        serde_json::to_value(WebServerConfig::default()).expect("WebServerConfig serializes"),
        "a v0.9 fixture never mentions web_server: it must resolve to WebServerConfig::default()"
    );
}

/// Config-resolution level (D-07, part 1), the nine-platform-struct half (Pitfall 3): none
/// of these is a `Settings` field, so each is asserted against its own `::default()`
/// directly, reporting the off/inert state `src/bin/paladin-server.rs` relies on.
#[test]
fn platform_config_defaults_are_inert() {
    assert_eq!(
        RunStoreConfig::default().backend,
        RunStoreBackend::Disabled,
        "RunStoreConfig must default to Disabled"
    );
    assert_eq!(
        WaypointStoreConfig::default().backend,
        WaypointStoreBackend::Disabled,
        "WaypointStoreConfig must default to Disabled"
    );
    assert!(
        !SchedulesConfig::default().enabled,
        "SchedulesConfig must default to disabled -- no v0.9 deployment gains scheduled runs on upgrade"
    );
    // NOTE: unlike every other flag asserted in this function, AssistantsConfig's
    // code-registry exposure defaults to `true`, by its own doc comment ("the one
    // subsystem in this phase that starts ON") -- verified directly against
    // `src/config/assistants.rs::Default for AssistantsConfig`. It is still "inert" in
    // the sense this test cares about: it exposes a read-only synthetic view of agents
    // ALREADY resident in the pre-existing `AgentRegistry`, wiring no new backend and
    // spawning no new task, so it does not violate "a v0.9 config boots with v0.9
    // behavior" the way an enabled run store or schedule loop would.
    assert!(
        AssistantsConfig::default().expose_code_registry,
        "AssistantsConfig deliberately defaults to true (its own doc comment) -- \
         this is the one platform subsystem recorded as starting ON, not off"
    );
    assert!(
        !WebhooksConfig::default().allow_private,
        "WebhooksConfig's SSRF override must default to false"
    );

    assert!(
        RunStoreConfig::default().validate().is_ok(),
        "RunStoreConfig::default() must validate"
    );
    assert!(
        RunQueueConfig::default().validate().is_ok(),
        "RunQueueConfig::default() must validate"
    );
    assert!(
        RunWorkerConfig::default().validate().is_ok(),
        "RunWorkerConfig::default() must validate"
    );
    assert!(
        RunStreamConfig::default().validate().is_ok(),
        "RunStreamConfig::default() must validate"
    );
    assert!(
        AssistantsConfig::default().validate().is_ok(),
        "AssistantsConfig::default() must validate"
    );
    assert!(
        SchedulesConfig::default().validate().is_ok(),
        "SchedulesConfig::default() must validate"
    );
    assert!(
        WebhooksConfig::default().validate().is_ok(),
        "WebhooksConfig::default() must validate"
    );
    assert!(
        WaypointStoreConfig::default().validate().is_ok(),
        "WaypointStoreConfig::default() must validate"
    );
    assert!(
        EngineConfig::default().validate().is_ok(),
        "EngineConfig::default() must validate"
    );
}

/// M-B-02: the one v0.10 default that deliberately differs from v0.9 behavior --
/// `graceful_shutdown` defaults `true`, asserted as that value rather than as "off"
/// (D-07's explicit carve-out).
#[test]
fn graceful_shutdown_defaults_true_by_m_b_02() {
    assert!(
        EngineConfig::default().graceful_shutdown,
        "M-B-02: graceful_shutdown must default to true, the one deliberate exception \
         to 'every new tunable defaults to off'"
    );
}

/// Threat T-29-01-02 mitigation: with every one of the 26 `APP_*` variables the nine
/// platform structs read cleared, `apply_env_overrides()` must leave each struct
/// unchanged from its own `::default()` -- neither a CI runner's ambient environment nor
/// a sibling test can produce a pass for a reason a real v0.9 operator's file would not
/// reproduce. `#[serial]` because this test mutates process-wide environment state.
/// `EngineConfig` and `WaypointStoreConfig` are compared through serialized JSON (no
/// `PartialEq` on the whole struct, and X-03 forbids adding one); the remaining seven
/// already derive `PartialEq` and are compared directly.
#[test]
#[serial]
fn apply_env_overrides_is_a_no_op_with_no_app_vars() {
    let saved: Vec<(&str, Option<String>)> = ALL_PLATFORM_APP_ENV_VARS
        .iter()
        .map(|k| (*k, env::var(k).ok()))
        .collect();
    unsafe {
        for k in ALL_PLATFORM_APP_ENV_VARS {
            env::remove_var(k);
        }
    }

    let mut engine = EngineConfig::default();
    engine.apply_env_overrides();
    assert_eq!(
        serde_json::to_value(&engine).expect("EngineConfig serializes"),
        serde_json::to_value(EngineConfig::default()).expect("EngineConfig serializes"),
        "EngineConfig::apply_env_overrides() must be a no-op with no APP_* vars set"
    );

    let mut waypoint_store = WaypointStoreConfig::default();
    waypoint_store.apply_env_overrides();
    assert_eq!(
        serde_json::to_value(&waypoint_store).expect("WaypointStoreConfig serializes"),
        serde_json::to_value(WaypointStoreConfig::default())
            .expect("WaypointStoreConfig serializes"),
        "WaypointStoreConfig::apply_env_overrides() must be a no-op with no APP_* vars set"
    );

    let mut run_store = RunStoreConfig::default();
    run_store.apply_env_overrides();
    assert_eq!(
        run_store,
        RunStoreConfig::default(),
        "RunStoreConfig::apply_env_overrides() must be a no-op with no APP_* vars set"
    );

    let mut run_queue = RunQueueConfig::default();
    run_queue.apply_env_overrides();
    assert_eq!(
        run_queue,
        RunQueueConfig::default(),
        "RunQueueConfig::apply_env_overrides() must be a no-op with no APP_* vars set"
    );

    let mut run_worker = RunWorkerConfig::default();
    run_worker.apply_env_overrides();
    assert_eq!(
        run_worker,
        RunWorkerConfig::default(),
        "RunWorkerConfig::apply_env_overrides() must be a no-op with no APP_* vars set"
    );

    let mut run_stream = RunStreamConfig::default();
    run_stream.apply_env_overrides();
    assert_eq!(
        run_stream,
        RunStreamConfig::default(),
        "RunStreamConfig::apply_env_overrides() must be a no-op with no APP_* vars set"
    );

    let mut assistants = AssistantsConfig::default();
    assistants.apply_env_overrides();
    assert_eq!(
        assistants,
        AssistantsConfig::default(),
        "AssistantsConfig::apply_env_overrides() must be a no-op with no APP_* vars set"
    );

    let mut schedules = SchedulesConfig::default();
    schedules.apply_env_overrides();
    assert_eq!(
        schedules,
        SchedulesConfig::default(),
        "SchedulesConfig::apply_env_overrides() must be a no-op with no APP_* vars set"
    );

    let mut webhooks = WebhooksConfig::default();
    webhooks.apply_env_overrides();
    assert_eq!(
        webhooks,
        WebhooksConfig::default(),
        "WebhooksConfig::apply_env_overrides() must be a no-op with no APP_* vars set"
    );

    unsafe {
        for (k, v) in saved {
            match v {
                Some(val) => env::set_var(k, val),
                None => env::remove_var(k),
            }
        }
    }
}

/// Behavioral level (D-07, part 2): the composed app mounts exactly the v0.9 route set --
/// the versioned agent API plus the unversioned health/ready/docs routes.
#[tokio::test]
async fn v0_9_routes_are_mounted() {
    let path = v0_9_fixture_path();
    let settings = Settings::load_from_file(path.to_str().expect("path is valid UTF-8"))
        .expect("fixture loads");
    let app = compose_app_from_settings(&settings).await;

    for (method, uri) in [
        ("GET", "/v1/agents"),
        ("GET", "/health"),
        ("GET", "/ready"),
        ("GET", "/openapi.json"),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "{method} {uri} must answer 200 -- part of the v0.9 route set"
        );
    }
}

/// Behavioral level (D-07, part 2), Pitfall 2: every route family new in v0.10 answers
/// `501`, never `404`, when its backing config is unwired -- one assertion per family so a
/// future regression names which family broke.
#[tokio::test]
async fn v0_10_route_families_answer_501() {
    let path = v0_9_fixture_path();
    let settings = Settings::load_from_file(path.to_str().expect("path is valid UTF-8"))
        .expect("fixture loads");
    let app = compose_app_from_settings(&settings).await;

    for (method, uri, family) in [
        ("POST", "/v1/runs", "/v1/runs"),
        ("GET", "/v1/threads/any/history", "/v1/threads"),
        ("GET", "/v1/assistants", "/v1/assistants"),
        ("GET", "/v1/schedules", "/v1/schedules"),
    ] {
        let mut builder = Request::builder().method(method).uri(uri);
        if method == "POST" {
            builder = builder.header("content-type", "application/json");
        }
        let body = if method == "POST" {
            Body::from(r#"{"assistant_id":"any"}"#)
        } else {
            Body::empty()
        };
        let response = app
            .clone()
            .oneshot(builder.body(body).expect("request builds"))
            .await
            .expect("router responds");
        assert_eq!(
            response.status(),
            StatusCode::NOT_IMPLEMENTED,
            "{family} family ({method} {uri}) must answer 501, never 404, when unwired"
        );
    }
}

/// The one place `404` is the correct expectation (Pitfall 2's carve-out): the `dev-ui`
/// router is never merged by this composition (mirroring `paladin-server.rs`, which only
/// merges it behind the `dev-ui` feature and an explicit config flag), so its paths are
/// genuinely unregistered rather than registered-but-unwired.
#[tokio::test]
async fn dev_ui_route_is_unregistered_404() {
    let path = v0_9_fixture_path();
    let settings = Settings::load_from_file(path.to_str().expect("path is valid UTF-8"))
        .expect("fixture loads");
    let app = compose_app_from_settings(&settings).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/dev-ui/threads/some-thread")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");

    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "the dev-ui router is never merged by this composition, unlike /v1/runs etc. \
         which ARE registered and answer 501 -- this is genuinely unregistered, not \
         registered-but-unwired"
    );
}
