//! `paladin-server` — run configured Paladin agents behind an HTTP API.
//!
//! This is the runnable entry point for the **HTTP service-host** deployment topology
//! (Milestone 12). It loads a `config.yml`, builds the configured agents into a
//! `paladin-web` agent registry, and serves the agent-execution API
//! (`/agents/*`) over HTTP with graceful shutdown.
//!
//! ```bash
//! OPENAI_API_KEY=sk-... cargo run --bin paladin-server --features web-server
//! # or point at a specific config:
//! PALADIN_CONFIG=./config.yml paladin-server
//! ```
//!
//! Requires the `web-server` feature (enforced via `required-features` in `Cargo.toml`).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use log::{error, info, warn};
use paladin::application::services::parley::{GraphRegistry, ParleyPortAdapter};
use paladin::config::agents::AuthConfig;
use paladin::config::assistants::AssistantsConfig;
use paladin::config::engine::EngineConfig;
use paladin::config::env_utils::EnvOverridable;
use paladin::config::run_queue::RunQueueConfig;
use paladin::config::run_store::{RunStoreBackend, RunStoreConfig};
use paladin::config::run_stream::RunStreamConfig;
use paladin::config::run_worker::RunWorkerConfig;
use paladin::config::schedules::SchedulesConfig;
use paladin::config::settings::Settings;
use paladin::config::waypoint_store::{WaypointStoreBackend, WaypointStoreConfig};
use paladin::config::webhooks::WebhooksConfig;
use paladin::infrastructure::adapters::auth::InMemoryTokenAuthAdapter;
use paladin::infrastructure::web::agent_host::{bind_address, build_agent_registry};
use paladin::infrastructure::web::facade_provisioner::FacadeProvisioner;
use paladin::infrastructure::web::run_api_wiring::{
    ErasedWaypointStore, RunApiConfigs, build_run_api,
};
use paladin::infrastructure::web::{
    AgentApiState, AgentAuthConfig, HttpLayersConfig, Principal, RateLimitConfig, ThreadApiState,
    TimeoutPolicy, agent_router, run_router, thread_router, with_http_layers,
};
use paladin_battalion::engine::WarEngine;
use paladin_battalion::engine::shutdown::ShutdownCoordinator;
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_ports::input::parley_port::ParleyPort;
use paladin_ports::output::auth_port::AuthPort;
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream};
use paladin_ports::output::run_queue_port::RunQueuePort;
use paladin_ports::output::run_repository_port::RunRepositoryPort;
use paladin_ports::output::waypoint_port::WaypointPort;
use tokio::signal;

#[tokio::main]
async fn main() {
    // Load .env in debug builds; production uses real secrets management.
    #[cfg(debug_assertions)]
    {
        let _ = dotenv::dotenv();
    }
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    if let Err(e) = run().await {
        error!("paladin-server failed to start: {e}");
        std::process::exit(1);
    }
}

/// Load config, build the agent host, and serve until a shutdown signal.
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = config_path();
    info!("Loading configuration from '{config_path}'");
    let settings = Settings::load_from_file(&config_path)?;

    // Build the resident agents and the runtime provisioner from the same config.
    // `build_agent_registry` validates the config first, so misconfiguration fails here
    // with a specific message rather than mid-serve.
    let registry = build_agent_registry(&settings).await?;
    let mut agent_ids: Vec<String> = registry.list().into_iter().map(|(id, _)| id).collect();
    agent_ids.sort();
    // Shared with `build_run_api`'s `CodeAgentResolver` (D-32) below, so a code-registered
    // agent id is runnable through `POST /runs` without a second registry.
    let registry = Arc::new(registry);
    let provisioner = FacadeProvisioner::from_settings(&settings);
    let timeouts = settings.timeouts.clone().unwrap_or_default();
    // Cross-cutting HTTP layers (health routes are merged inside `agent_router`).
    let http = settings.http.clone().unwrap_or_default();

    // Resolve authentication (fail-closed: enabled + no credentials ⇒ refuse to start).
    let auth = build_auth_config(&http.auth)?;

    // Graceful-shutdown coordinator (HITL-04, D-21/D-22): constructed once per
    // process and handed forward to every component that starts an in-flight
    // engine run. Today no such component registers yet -- the resume port's
    // background continuation (plan 24-10) is the first real registrant --
    // but `shutdown_signal` below already cancels this SAME instance on
    // SIGTERM/SIGINT. `EngineConfig` (not `Settings`, X-10 avoidance) is the
    // one config struct feeding both the engine and this process-level wait.
    let mut engine_config = EngineConfig::default();
    engine_config.apply_env_overrides();
    engine_config
        .validate()
        .map_err(|e| format!("invalid engine configuration: {e}"))?;
    let shutdown_coordinator = ShutdownCoordinator::new();
    let shutdown_grace = Duration::from_secs(engine_config.shutdown_grace_secs);
    let graceful_shutdown = engine_config.graceful_shutdown;

    // Thread surface (HITL-05, D-24/D-25/D-26): a durable Waypoint backend is
    // OFF by default (`WaypointStoreConfig::default()` is `Disabled`), in
    // which case every `/v1/threads/*` route answers `501 not_implemented`
    // (D-24). `ThreadApiState` is its own struct with its own `auth` --
    // `AgentApiState` above is not modified (X-10.3).
    let mut waypoint_store_config = WaypointStoreConfig::default();
    waypoint_store_config.apply_env_overrides();
    waypoint_store_config
        .validate()
        .map_err(|e| format!("invalid waypoint store configuration: {e}"))?;

    // Platform API (Phase 27, PLAT-01..06): the seven X-09 config structs, each
    // `Default` + `apply_env_overrides()` + `validate()`'d before ever reaching
    // `build_run_api` -- every one defaults to off / today's-behaviour (D-50), so a
    // v0.9 config boots this v0.10 binary with no run server at all.
    let mut run_store_config = RunStoreConfig::default();
    run_store_config.apply_env_overrides();
    run_store_config
        .validate()
        .map_err(|e| format!("invalid run store configuration: {e}"))?;
    let mut run_queue_config = RunQueueConfig::default();
    run_queue_config.apply_env_overrides();
    run_queue_config
        .validate()
        .map_err(|e| format!("invalid run queue configuration: {e}"))?;
    let mut run_worker_config = RunWorkerConfig::default();
    run_worker_config.apply_env_overrides();
    run_worker_config
        .validate()
        .map_err(|e| format!("invalid run worker configuration: {e}"))?;
    let mut run_stream_config = RunStreamConfig::default();
    run_stream_config.apply_env_overrides();
    run_stream_config
        .validate()
        .map_err(|e| format!("invalid run stream configuration: {e}"))?;
    let mut assistants_config = AssistantsConfig::default();
    assistants_config.apply_env_overrides();
    assistants_config
        .validate()
        .map_err(|e| format!("invalid assistants configuration: {e}"))?;
    let mut schedules_config = SchedulesConfig::default();
    schedules_config.apply_env_overrides();
    schedules_config
        .validate()
        .map_err(|e| format!("invalid schedules configuration: {e}"))?;
    let mut webhooks_config = WebhooksConfig::default();
    webhooks_config.apply_env_overrides();
    webhooks_config
        .validate()
        .map_err(|e| format!("invalid webhooks configuration: {e}"))?;
    let run_configs = RunApiConfigs {
        run_store: run_store_config,
        run_queue: run_queue_config,
        run_worker: run_worker_config,
        run_stream: run_stream_config,
        assistants: assistants_config,
        schedules: schedules_config,
        webhooks: webhooks_config,
    };
    let run_store_backend_label = format!("{:?}", run_configs.run_store.backend);
    let run_store_disabled = matches!(run_configs.run_store.backend, RunStoreBackend::Disabled);

    // One waypoint store, shared by the thread surface's own `WarEngine` AND the run
    // engine `build_run_api` constructs (D-24 precedent extended to the run pipeline):
    // `build_run_api` errors closed, naming `waypoint_store.backend`, if `run_store` is
    // enabled but this is `None`.
    let waypoint_store = build_waypoint_store(&waypoint_store_config).await?;

    let run_handles = build_run_api(
        run_configs,
        &settings,
        shutdown_coordinator.clone(),
        waypoint_store.clone(),
        auth.clone(),
        Arc::clone(&registry),
    )
    .await?;

    // D-24/PLAT-FR-06: thread `parley_extras` into the SAME `ParleyPortAdapter` the thread
    // surface builds, so a resume against a thread with an active run row re-enqueues
    // durably instead of spawning in-process (D-19..D-23); then thread the run
    // repository/submission directly onto `ThreadApiState` for `GET /threads*`/
    // `POST /threads/{id}/fork` (D-45).
    let mut thread_state = thread_state_from_store(
        waypoint_store,
        &engine_config,
        shutdown_coordinator.clone(),
        auth.clone(),
        run_handles.parley_extras.clone(),
    );
    if let Some(run_repository) = run_handles.run_repository.clone() {
        thread_state = thread_state.with_runs(run_repository);
    }
    if let Some(run_submission) = run_handles.thread_run_submission.clone() {
        thread_state = thread_state.with_run_submission(run_submission);
    }

    let state = AgentApiState::new(Arc::clone(&registry))
        .with_provisioner(Arc::new(provisioner))
        .with_timeouts(TimeoutPolicy {
            default_secs: timeouts.default_seconds,
            max_secs: timeouts.max_seconds,
        })
        .with_auth(auth);
    let layers = HttpLayersConfig {
        cors_allow_origins: http.cors_allow_origins.clone(),
        body_limit_bytes: http.body_limit_bytes,
        global_timeout_secs: http.global_timeout_seconds,
        rate_limit: RateLimitConfig {
            enabled: http.rate_limit.enabled,
            per_second: http.rate_limit.per_second,
            burst: http.rate_limit.burst,
        },
    };
    // Optionally serve the OpenAPI spec + Swagger UI (unversioned, unauthenticated).
    let docs_enabled = http.docs.enabled;
    // `thread_router`'s and `run_router`'s output are merged ALONGSIDE `agent_router`'s,
    // never inside it, so `AgentApiState` stays untouched (D-24, D-44).
    let routes = agent_router(state.clone())
        .merge(thread_router(thread_state))
        .merge(run_router(run_handles.run_state));
    let routes = if docs_enabled {
        let spec = paladin::infrastructure::web::openapi::build_openapi(state);
        routes.merge(paladin::infrastructure::web::openapi::docs_router(spec))
    } else {
        routes
    };
    let app = with_http_layers(routes, &layers);
    // Kept alive until this function returns (i.e. until `axum::serve`'s graceful
    // shutdown completes): every task here is already registered with
    // `shutdown_coordinator`, so `drain_on_shutdown`'s `cancel_and_wait` is what actually
    // waits for them -- this binding only needs to outlive that wait (D-13).
    let _run_tasks = run_handles.tasks;

    let listener = tokio::net::TcpListener::bind(bind_address(&settings)).await?;
    let bound = listener.local_addr()?;
    info!(
        "paladin-server listening on http://{bound} — serving {} agent(s): {:?}",
        agent_ids.len(),
        agent_ids
    );
    info!(
        "routes: GET /health, GET /ready, GET/POST /v1/agents, GET/DELETE /v1/agents/{{id}}, POST /v1/agents/{{id}}/execute[/stream], POST /v1/agents/{{id}}/jobs, GET /v1/agents/{{id}}/jobs/{{job_id}}, GET/POST /v1/threads[/{{id}}][/fork], GET /v1/threads/{{id}}/state, POST /v1/threads/{{id}}/resume, GET /v1/threads/{{id}}/history, GET/POST /v1/runs[/{{id}}][/stream|/cancel|/webhook-deliveries], GET/POST /v1/assistants[/{{id}}][/versions...], GET/POST /v1/schedules[/{{id}}]"
    );
    info!(
        "waypoint store backend: {:?} ({})",
        waypoint_store_config.backend,
        if matches!(
            waypoint_store_config.backend,
            WaypointStoreBackend::Disabled
        ) {
            "thread routes answer 501 until a backend is configured"
        } else {
            "thread routes are live"
        }
    );
    info!(
        "run store backend: {run_store_backend_label} ({})",
        if run_store_disabled {
            "run server disabled; POST /v1/runs and every /v1/runs*, /v1/assistants*, \
             /v1/schedules* route answers 501 until run_store.backend is configured"
        } else {
            "run server is live"
        }
    );
    if docs_enabled {
        info!("docs: GET /openapi.json, Swagger UI at /docs");
    } else {
        info!("docs: disabled (http.docs.enabled = false)");
    }
    info!(
        "layers: request-log + CORS + body-limit({}B){}{}",
        layers.body_limit_bytes,
        if layers.global_timeout_secs > 0 {
            format!(" + global-timeout({}s)", layers.global_timeout_secs)
        } else {
            String::new()
        },
        if layers.rate_limit.enabled {
            format!(
                " + rate-limit({}/s, burst {})",
                layers.rate_limit.per_second, layers.rate_limit.burst
            )
        } else {
            String::new()
        }
    );

    // `ConnectInfo` lets the rate limiter key on the peer IP for direct connections.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal(
        shutdown_coordinator,
        shutdown_grace,
        graceful_shutdown,
    ))
    .await?;

    info!("paladin-server shut down cleanly");
    Ok(())
}

/// Warning emitted, unconditionally, every time `build_auth_config` wires the in-process
/// bearer-token store (`http.auth.bearer_token.enabled = true`).
///
/// The store verifies a token only on the process that issued it — it holds no shared state
/// across replicas. A running pod has no built-in way to learn how many peers it has without
/// calling out to the orchestrator's own API, so this warning is not conditioned on an
/// observed replica count (see ADR-0041); it fires on every start that wires the store,
/// whether that deployment runs one replica or many.
const IN_PROCESS_TOKEN_STORE_WARNING: &str = "in-process bearer-token store ENABLED \
     (http.auth.bearer_token.enabled = true) — this is an in-process token store: tokens verify \
     only on the issuing process. Do not scale past one replica while this store is wired. \
     See ADR-0041 (.planning/decisions/0041-in-process-token-store-single-replica-scope.md).";

/// Translate the config `auth` section into the web layer's [`AgentAuthConfig`].
///
/// **Fail-closed:** when auth is enabled but no credential source (API keys or an opaque
/// bearer token) is configured, this returns an error so the server refuses to start rather
/// than silently serving an open API. When auth is disabled, a warning is logged and the API
/// is open.
fn build_auth_config(cfg: &AuthConfig) -> Result<AgentAuthConfig, Box<dyn std::error::Error>> {
    if !cfg.enabled {
        warn!(
            "agent API authentication is DISABLED (http.auth.enabled = false) — all agent routes are open"
        );
        return Ok(AgentAuthConfig {
            enabled: false,
            api_keys: HashMap::new(),
            token_verifier: None,
        });
    }

    let api_keys: HashMap<String, Principal> = cfg
        .api_keys
        .iter()
        .map(|k| {
            (
                k.key.clone(),
                Principal {
                    id: k.name.clone(),
                    role: k.role,
                },
            )
        })
        .collect();

    // The bearer-token path reuses the existing AuthPort against the in-process opaque
    // token store. The in-memory adapter verifies tokens it issued in-process, so it is
    // primarily useful when token issuance is co-located; API keys are the standalone
    // service-to-service mechanism.
    let token_verifier: Option<Arc<dyn AuthPort>> = if cfg.bearer_token.enabled {
        warn!("{IN_PROCESS_TOKEN_STORE_WARNING}");
        Some(Arc::new(InMemoryTokenAuthAdapter::new()))
    } else {
        None
    };

    let auth = AgentAuthConfig {
        enabled: true,
        api_keys,
        token_verifier,
    };

    if !auth.has_credentials() {
        return Err(
            "authentication is enabled but no credentials are configured: set \
             http.auth.api_keys and/or http.auth.bearer_token.enabled, or set http.auth.enabled = false"
                .into(),
        );
    }

    info!(
        "agent API authentication ENABLED ({} API key(s){})",
        auth.api_keys.len(),
        if cfg.bearer_token.enabled {
            " + bearer token"
        } else {
            ""
        }
    );
    Ok(auth)
}

/// A [`PaladinPort`] this binary's own thread-surface `WarEngine` is
/// constructed with but never actually calls (HITL-05, D-26, ADR-0039):
/// `build_thread_state` registers no `WarGraph`s in its [`GraphRegistry`],
/// because HTTP-served agents in this topology are LLM-plus-prompt only, not
/// graph-backed. Every `resume_with` call therefore fails closed with
/// `GraphNotRegistered` until an embedder registers a graph -- the intended
/// topology, not an oversight (see this type's `unreachable!` bodies and
/// `paladin-web`'s own `tower::util::oneshot` tests, which prove end-to-end
/// HTTP resume against a REAL graph using an in-test registry instead).
struct NoRegisteredGraphsPaladinPort;

#[async_trait]
impl PaladinPort for NoRegisteredGraphsPaladinPort {
    async fn execute(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinResult, PaladinError> {
        unreachable!(
            "this process's thread-surface WarEngine has no registered WarGraph, so it can \
             never dispatch a NodeSpec::Paladin node (ADR-0039)"
        )
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinStream, PaladinError> {
        unreachable!(
            "this process's thread-surface WarEngine has no registered WarGraph, so it can \
             never dispatch a NodeSpec::Paladin node (ADR-0039)"
        )
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

/// Build the durable waypoint store from [`WaypointStoreConfig`] (HITL-05, D-24/D-25/D-26;
/// extended by Phase 27 to also feed [`build_run_api`]'s run engine): `Disabled` (the
/// default) yields `None`; `Sqlite`/`Postgres` connect a real store, already erased to
/// `Arc<dyn WaypointPort>` so ONE instance can be shared by both the thread surface's own
/// `WarEngine` ([`thread_state_from_store`]) and the run engine `build_run_api` constructs.
async fn build_waypoint_store(
    waypoint_store_config: &WaypointStoreConfig,
) -> Result<Option<Arc<dyn WaypointPort>>, Box<dyn std::error::Error>> {
    match &waypoint_store_config.backend {
        WaypointStoreBackend::Disabled => Ok(None),
        WaypointStoreBackend::Sqlite { path } => {
            let store = paladin_storage::waypoint::sqlite::SqliteWaypointStore::new(path)
                .await
                .map_err(|e| format!("failed to open sqlite waypoint store at '{path}': {e}"))?;
            Ok(Some(Arc::new(store) as Arc<dyn WaypointPort>))
        }
        WaypointStoreBackend::Postgres { url_env } => build_postgres_waypoint_store(url_env).await,
    }
}

/// The `Postgres` branch of [`build_waypoint_store`], split out so the
/// `#[cfg(feature = "storage-postgres")]` gate (X-11.4: the default
/// `paladin-ai` build gains no Postgres driver) applies to one small
/// function rather than an inline `#[cfg]` block inside a `match` arm.
#[cfg(feature = "storage-postgres")]
async fn build_postgres_waypoint_store(
    url_env: &str,
) -> Result<Option<Arc<dyn WaypointPort>>, Box<dyn std::error::Error>> {
    let url = std::env::var(url_env).map_err(|_| {
        format!("waypoint store postgres backend names env var '{url_env}', which is not set")
    })?;
    let store = paladin_storage::waypoint::postgres::PostgresWaypointStore::new(&url)
        .await
        .map_err(|e| format!("failed to open postgres waypoint store: {e}"))?;
    Ok(Some(Arc::new(store) as Arc<dyn WaypointPort>))
}

/// When this binary is built without `storage-postgres`, a configured
/// `Postgres` backend is a startup error naming the missing feature, never a
/// silent `Disabled` fallback (fail-closed, matching `build_auth_config`'s
/// own precedent elsewhere in this file).
#[cfg(not(feature = "storage-postgres"))]
async fn build_postgres_waypoint_store(
    url_env: &str,
) -> Result<Option<Arc<dyn WaypointPort>>, Box<dyn std::error::Error>> {
    Err(format!(
        "waypoint store backend is configured as 'postgres' (env var '{url_env}') but this \
         binary was built without the 'storage-postgres' feature; rebuild with \
         --features storage-postgres,web-server, or set APP_WAYPOINT_STORE_BACKEND=disabled or \
         =sqlite"
    )
    .into())
}

/// Compose the thread surface's [`ThreadApiState`] over an already-erased waypoint store
/// (`None` when no backend is configured -- every `/v1/threads/*` route then answers `501`
/// naming the config key to set, D-24). `parley_extras`, when `Some`, wires
/// [`ParleyPortAdapter::with_run_repository`]/[`with_run_queue`](ParleyPortAdapter::with_run_queue)
/// so a resume against a thread with an active run row re-enqueues durably instead of
/// spawning in-process (PLAT-FR-06, D-19..D-23) -- `None` (Phase 24's exact prior behavior)
/// when the run store is disabled. [`ErasedWaypointStore`] lets this function build a
/// concrete-typed `WarEngine`/`ParleyPortAdapter` over the SAME trait-object store
/// [`build_run_api`]'s own run engine uses, without either function needing to know the
/// other's concrete backend type.
fn thread_state_from_store(
    waypoint_store: Option<Arc<dyn WaypointPort>>,
    engine_config: &EngineConfig,
    coordinator: ShutdownCoordinator,
    auth: AgentAuthConfig,
    parley_extras: Option<(Arc<dyn RunRepositoryPort>, Arc<dyn RunQueuePort>)>,
) -> ThreadApiState {
    let Some(store) = waypoint_store else {
        return ThreadApiState::new().with_auth(auth);
    };
    let erased = Arc::new(ErasedWaypointStore::new(Arc::clone(&store)));
    let engine = Arc::new(
        WarEngine::new(Arc::new(NoRegisteredGraphsPaladinPort), Arc::clone(&erased))
            .with_durability(engine_config.waypoint_durability)
            .with_shutdown_grace(Duration::from_secs(engine_config.shutdown_grace_secs)),
    );
    // Deliberately empty: this process registers no `WarGraph`s (ADR-0039).
    let registry = Arc::new(GraphRegistry::new());
    let mut adapter = ParleyPortAdapter::new(engine, erased, registry, coordinator);
    if let Some((run_repository, run_queue)) = parley_extras {
        adapter = adapter
            .with_run_repository(run_repository)
            .with_run_queue(run_queue);
    }
    let parley: Arc<dyn ParleyPort> = Arc::new(adapter);
    ThreadApiState::new()
        .with_waypoints(store)
        .with_parley(parley)
        .with_auth(auth)
}

/// Thin wrapper over [`build_waypoint_store`] + [`thread_state_from_store`] with no
/// `parley_extras` -- kept for this file's own pre-Phase-27 test coverage
/// (`server_wires_no_waypoint_backend_by_default`/`server_wires_sqlite_backend_when_configured`),
/// which exercise exactly Phase 24's thread-surface-only behavior. `run()` itself calls
/// [`build_waypoint_store`]/[`thread_state_from_store`] directly (it needs `parley_extras`),
/// so this wrapper is test-only.
#[cfg(test)]
async fn build_thread_state(
    waypoint_store_config: &WaypointStoreConfig,
    engine_config: &EngineConfig,
    coordinator: ShutdownCoordinator,
    auth: AgentAuthConfig,
) -> Result<ThreadApiState, Box<dyn std::error::Error>> {
    let waypoint_store = build_waypoint_store(waypoint_store_config).await?;
    Ok(thread_state_from_store(
        waypoint_store,
        engine_config,
        coordinator,
        auth,
        None,
    ))
}

/// Resolve the config file path: `PALADIN_CONFIG`, else the first CLI argument, else
/// `config.yml`.
fn config_path() -> String {
    std::env::var("PALADIN_CONFIG")
        .ok()
        .or_else(|| std::env::args().nth(1))
        .unwrap_or_else(|| "config.yml".to_string())
}

/// Resolve when the process receives `Ctrl-C` or (on Unix) `SIGTERM`.
async fn wait_for_termination_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("received Ctrl-C; shutting down"),
        _ = terminate => info!("received SIGTERM; shutting down"),
    }
}

/// Cancel `coordinator` and drain every registered in-flight engine run
/// within `grace`, or skip the wait entirely when `graceful` is `false`
/// (the `MIGRATION.md` M-B-02 disable switch for legacy-only deployments,
/// D-20). Split out from [`shutdown_signal`] so the drain behaviour is
/// exercised by a simulated trigger in tests rather than requiring a real
/// OS signal (HITL-04, D-22).
async fn drain_on_shutdown(coordinator: &ShutdownCoordinator, grace: Duration, graceful: bool) {
    if graceful {
        let outcome = coordinator.cancel_and_wait(grace).await;
        info!("graceful shutdown drain complete: {outcome:?}");
    } else {
        coordinator.token().cancel();
        info!(
            "graceful_shutdown disabled (APP_ENGINE_GRACEFUL_SHUTDOWN=false); cancelling \
             in-flight runs without waiting"
        );
    }
}

/// Wait for a termination signal, then cancel `coordinator` and drain every
/// registered in-flight engine run within `grace` (skipped when `graceful`
/// is `false`) before `axum::serve(...).with_graceful_shutdown` completes
/// (HITL-04, D-22).
async fn shutdown_signal(coordinator: ShutdownCoordinator, grace: Duration, graceful: bool) {
    wait_for_termination_signal().await;
    drain_on_shutdown(&coordinator, grace, graceful).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin::config::agents::{ApiKeyConfig, BearerTokenAuthConfig};
    use paladin_core::platform::container::user::UserRole;
    use std::sync::{Mutex, Once};
    use tower::ServiceExt; // for `Router::oneshot`

    /// A `log::Log` implementation that records formatted `(level, message)` pairs instead of
    /// printing them, so tests can assert on what `build_auth_config` actually emits rather
    /// than on documentation about what it emits.
    struct CapturingLogger {
        records: Mutex<Vec<(log::Level, String)>>,
    }

    impl log::Log for CapturingLogger {
        fn enabled(&self, _metadata: &log::Metadata) -> bool {
            true
        }

        fn log(&self, record: &log::Record) {
            if let Ok(mut records) = self.records.lock() {
                records.push((record.level(), record.args().to_string()));
            }
        }

        fn flush(&self) {}
    }

    static CAPTURING_LOGGER: CapturingLogger = CapturingLogger {
        records: Mutex::new(Vec::new()),
    };
    static INIT: Once = Once::new();

    /// Install the capturing logger as the global `log` sink, once per test binary run.
    ///
    /// `log`'s default max level is `Off`, so `set_max_level(Warn)` is required here — without
    /// it every `log::Log::enabled`/`log` call is skipped before it reaches this logger at all,
    /// and the capture would pass vacuously.
    fn install_capturing_logger() {
        INIT.call_once(|| {
            log::set_logger(&CAPTURING_LOGGER).expect("failed to install capturing test logger");
            log::set_max_level(log::LevelFilter::Warn);
        });
    }

    /// Tests in this binary run in parallel and share the one process-global logger, so callers
    /// must search for their own expected substring rather than assert on buffer length or on
    /// an exact index.
    fn captured_records_contain(level: log::Level, needle: &str) -> bool {
        CAPTURING_LOGGER
            .records
            .lock()
            .expect("capturing logger mutex poisoned")
            .iter()
            .any(|(recorded_level, message)| *recorded_level == level && message.contains(needle))
    }

    fn api_key(name: &str) -> ApiKeyConfig {
        ApiKeyConfig {
            key: format!("test-key-{name}"),
            name: name.to_string(),
            role: UserRole::User,
        }
    }

    #[test]
    fn build_auth_config_warns_when_in_process_token_store_is_wired() {
        install_capturing_logger();

        let cfg = AuthConfig {
            enabled: true,
            api_keys: vec![api_key("wired-store-test")],
            bearer_token: BearerTokenAuthConfig { enabled: true },
        };

        let result = build_auth_config(&cfg);

        let auth = result.expect("enabled auth with one API key and the store wired must build");
        assert!(
            auth.token_verifier.is_some(),
            "the bearer-token verifier must be wired when http.auth.bearer_token.enabled = true"
        );
        assert!(
            captured_records_contain(log::Level::Warn, IN_PROCESS_TOKEN_STORE_WARNING),
            "expected a WARN record carrying the in-process token store constraint; captured: {:?}",
            CAPTURING_LOGGER.records.lock().unwrap()
        );
    }

    #[test]
    fn build_auth_config_fails_closed_when_enabled_with_no_credentials() {
        install_capturing_logger();

        let cfg = AuthConfig {
            enabled: true,
            api_keys: vec![],
            bearer_token: BearerTokenAuthConfig { enabled: false },
        };

        let result = build_auth_config(&cfg);

        assert!(
            result.is_err(),
            "authentication enabled with no API keys and the token store disabled must refuse \
             to start, not silently build an unauthenticated-but-enabled config"
        );
    }

    // --- Phase 24 Plan 09: ShutdownCoordinator process wiring (HITL-04,
    // D-22) -----------------------------------------------------------------

    #[tokio::test]
    async fn shutdown_signal_cancels_the_coordinator() {
        let coordinator = ShutdownCoordinator::new();
        let (child_token, guard) = coordinator.register();
        // No real work outstanding for this registration -- drop immediately
        // so the drain below returns fast rather than waiting on it.
        drop(guard);

        drain_on_shutdown(&coordinator, Duration::from_secs(5), true).await;

        assert!(
            child_token.is_cancelled(),
            "a simulated signal must cancel the coordinator's root token, observed here via a \
             registered run's child token"
        );
    }

    #[tokio::test]
    async fn process_waits_up_to_grace_for_in_flight_runs() {
        let coordinator = ShutdownCoordinator::new();
        let (_child_token, guard) = coordinator.register();

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            drop(guard);
        });

        let started = tokio::time::Instant::now();
        drain_on_shutdown(&coordinator, Duration::from_secs(5), true).await;
        let elapsed = started.elapsed();

        assert!(
            elapsed < Duration::from_secs(2),
            "the wait must return as soon as the in-flight run drains, not at the grace \
             deadline (elapsed: {elapsed:?})"
        );
    }

    #[tokio::test]
    async fn process_stops_waiting_at_the_grace_deadline() {
        let coordinator = ShutdownCoordinator::new();
        let (_child_token, guard) = coordinator.register();

        let started = tokio::time::Instant::now();
        drain_on_shutdown(&coordinator, Duration::from_millis(50), true).await;
        let elapsed = started.elapsed();

        assert!(
            elapsed >= Duration::from_millis(50),
            "the process must wait at least the configured grace before giving up on a run \
             that never drains (elapsed: {elapsed:?})"
        );
        assert_eq!(
            coordinator.in_flight(),
            1,
            "the never-dropped run is still registered after the deadline"
        );
        drop(guard);
    }

    #[tokio::test]
    async fn graceful_shutdown_disabled_skips_the_wait() {
        let coordinator = ShutdownCoordinator::new();
        let (_child_token, guard) = coordinator.register();

        let started = tokio::time::Instant::now();
        drain_on_shutdown(&coordinator, Duration::from_secs(30), false).await;
        let elapsed = started.elapsed();

        assert!(
            elapsed < Duration::from_millis(500),
            "graceful_shutdown = false must skip the wait entirely, even with a registered \
             run in flight (elapsed: {elapsed:?})"
        );
        assert!(
            coordinator.token().is_cancelled(),
            "the root token must still be cancelled even when the wait itself is skipped"
        );
        drop(guard);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn resume_continues_a_halted_thread_after_process_shutdown() {
        use async_trait::async_trait;
        use paladin_battalion::engine::node::{NodeContext, StateNode, StateNodeError};
        use paladin_battalion::engine::{EngineLimits, NodeSpec, RunOutcome, WarEngine, WarGraph};
        use paladin_core::platform::container::battlefield::StateDelta;
        use paladin_core::platform::container::battlefield::{Battlefield, BattlefieldSchema};
        use paladin_core::platform::container::directive::Directive;
        use paladin_core::platform::container::paladin::Paladin;
        use paladin_core::platform::container::paladin_error::PaladinError;
        use paladin_core::platform::container::waypoint::{NodeId, ThreadId};
        use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream};
        use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;

        // Minimal StateNode/PaladinPort test doubles built against the same
        // PUBLIC engine API `src/config/engine.rs`'s own WarEngine tests use
        // -- `paladin_battalion::engine::test_support` is `pub(crate)` to
        // that crate and unreachable from here.
        struct NoopNode;

        #[async_trait]
        impl StateNode for NoopNode {
            async fn run(
                &self,
                _state: &Battlefield,
                _ctx: &NodeContext,
            ) -> Result<Directive, StateNodeError> {
                Ok(StateDelta::new().into())
            }
        }

        struct UnusedPaladinPort;

        #[async_trait]
        impl PaladinPort for UnusedPaladinPort {
            async fn execute(
                &self,
                _paladin: &Paladin,
                _input: &str,
            ) -> Result<PaladinResult, PaladinError> {
                unreachable!("this test's WarGraph has no NodeSpec::Paladin nodes")
            }

            async fn execute_stream(
                &self,
                _paladin: &Paladin,
                _input: &str,
            ) -> Result<PaladinStream, PaladinError> {
                unreachable!("this test's WarGraph has no NodeSpec::Paladin nodes")
            }

            fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
                Ok(())
            }
        }

        let schema = BattlefieldSchema::new(vec![]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let node = NodeId::new("only-node");
        graph.add_node(node.clone(), NodeSpec::Function(Arc::new(NoopNode)));
        graph.add_entry(node);

        let store = Arc::new(InMemoryWaypointStore::new());
        let thread =
            ThreadId::new("process-wiring-resume-after-shutdown").expect("valid thread id");

        // Simulate the process-wiring path: a run registers with the SAME
        // ShutdownCoordinator paladin-server.rs cancels on SIGTERM/SIGINT.
        let coordinator = ShutdownCoordinator::new();
        let (child_token, guard) = coordinator.register();
        drop(guard); // no work in flight to wait for in this test
        drain_on_shutdown(&coordinator, Duration::from_secs(5), true).await;

        let halted_engine = WarEngine::new(Arc::new(UnusedPaladinPort), store.clone())
            .with_cancellation_token(child_token);
        let halted = halted_engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .expect("start returns Ok(RunOutcome) even when halted by cancellation");
        assert!(
            matches!(halted, RunOutcome::Halted { .. }),
            "a run registered with an already-cancelled coordinator must Halt at the \
             superstep boundary, got {halted:?}"
        );

        // A fresh engine instance, with no cancellation token, resumes the
        // SAME thread and completes: the explicit HITL-FR-14 assertion at
        // the process-wiring level (plan 24-01 already pins the equivalent
        // engine-level behaviour).
        let fresh_engine = WarEngine::new(Arc::new(UnusedPaladinPort), store);
        let resumed = fresh_engine
            .resume(&graph, thread)
            .await
            .expect("resume returns Ok(RunOutcome)");
        assert!(
            matches!(resumed, RunOutcome::Completed { .. }),
            "resume must continue a Halted thread after a coordinator-driven process shutdown \
             and complete, got {resumed:?}"
        );
    }

    // --- Phase 24 Plan 11: thread surface composition (HITL-05, D-24) -----

    #[tokio::test]
    async fn server_wires_no_waypoint_backend_by_default() {
        let config = WaypointStoreConfig::default();
        assert_eq!(config.backend, WaypointStoreBackend::Disabled);

        let engine_config = EngineConfig::default();
        let coordinator = ShutdownCoordinator::new();
        let thread_state = build_thread_state(
            &config,
            &engine_config,
            coordinator,
            AgentAuthConfig::default(),
        )
        .await
        .expect("disabled backend never fails to build");

        assert!(thread_state.waypoints.is_none());
        assert!(thread_state.parley.is_none());

        // The route itself answers 501, not merely the state fields being
        // `None` in isolation.
        let app = thread_router(thread_state);
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/v1/threads/any-thread/state")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn server_wires_sqlite_backend_when_configured() {
        let config = WaypointStoreConfig {
            backend: WaypointStoreBackend::Sqlite {
                path: "sqlite::memory:".to_string(),
            },
        };
        let engine_config = EngineConfig::default();
        let coordinator = ShutdownCoordinator::new();
        let thread_state = build_thread_state(
            &config,
            &engine_config,
            coordinator,
            AgentAuthConfig::default(),
        )
        .await
        .expect("sqlite backend builds");

        assert!(thread_state.waypoints.is_some());
        assert!(thread_state.parley.is_some());

        // An unknown thread reads through the real store: 404, not 501.
        let waypoints = thread_state.waypoints.clone().unwrap();
        let thread = paladin_core::platform::container::waypoint::ThreadId::new(
            "server-wiring-sqlite-thread",
        )
        .unwrap();
        assert!(waypoints.latest(&thread).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn thread_router_is_merged_alongside_agent_router() {
        let registry = paladin::infrastructure::web::AgentRegistry::new();
        let agent_state = AgentApiState::new(Arc::new(registry));
        let thread_state = ThreadApiState::new();

        let app = agent_router(agent_state).merge(thread_router(thread_state));

        let agents = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/v1/agents")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(agents.status(), axum::http::StatusCode::OK);

        let threads = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/v1/threads/any-thread/state")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        // No backend wired in this test's ThreadApiState -- 501, not 404,
        // proves the route is genuinely reachable and reached its handler.
        assert_eq!(threads.status(), axum::http::StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn thread_routes_share_the_agent_auth_middleware() {
        let auth = AgentAuthConfig {
            enabled: true,
            api_keys: HashMap::new(),
            token_verifier: None,
        };
        let thread_state = ThreadApiState::new().with_auth(auth);
        let app = thread_router(thread_state);

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/v1/threads/any-thread/state")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
    }

    // --- Phase 27 Plan 17: run API wiring (PLAT-01..06, D-44) --------------

    /// `run_router`'s output is merged ALONGSIDE `agent_router`'s and
    /// `thread_router`'s in `run()`, exactly like `thread_router` already is
    /// (D-24, D-44): an unwired `RunApiState` answers `501`, not `404`,
    /// proving the route is genuinely reachable and reached its handler.
    #[tokio::test]
    async fn run_router_is_merged_alongside_agent_and_thread_routers() {
        let registry = paladin::infrastructure::web::AgentRegistry::new();
        let agent_state = AgentApiState::new(Arc::new(registry));
        let thread_state = ThreadApiState::new();
        let run_state = paladin_web::RunApiState::new();

        let app = agent_router(agent_state)
            .merge(thread_router(thread_state))
            .merge(run_router(run_state));

        let agents = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/v1/agents")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(agents.status(), axum::http::StatusCode::OK);

        let runs = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/v1/runs")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(r#"{"assistant_id":"any"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(runs.status(), axum::http::StatusCode::NOT_IMPLEMENTED);
    }

    /// `build_run_api` over every config's `Default` (`run_store.backend =
    /// disabled`) spawns zero background tasks -- the D-50/X-09 contract
    /// this binary's own startup relies on for a v0.9 config to boot
    /// v0.10 with no run server at all.
    #[tokio::test]
    async fn default_config_spawns_no_run_services() {
        use paladin::infrastructure::web::run_api_wiring::{RunApiConfigs, build_run_api};

        let configs = RunApiConfigs {
            run_store: paladin::config::run_store::RunStoreConfig::default(),
            run_queue: paladin::config::run_queue::RunQueueConfig::default(),
            run_worker: paladin::config::run_worker::RunWorkerConfig::default(),
            run_stream: paladin::config::run_stream::RunStreamConfig::default(),
            assistants: paladin::config::assistants::AssistantsConfig::default(),
            schedules: paladin::config::schedules::SchedulesConfig::default(),
            webhooks: paladin::config::webhooks::WebhooksConfig::default(),
        };
        let registry = Arc::new(paladin::infrastructure::web::AgentRegistry::new());

        let handles = build_run_api(
            configs,
            &Settings::default(),
            ShutdownCoordinator::new(),
            None,
            AgentAuthConfig::default(),
            registry,
        )
        .await
        .expect("disabled run store never fails to build");

        assert!(
            handles.tasks.is_empty(),
            "a disabled run store must spawn no background tasks"
        );
    }

    /// The served `/openapi.json` document lists the run paths (PLAT-06,
    /// D-44) regardless of whether a run store is actually configured in
    /// the process building it -- the SAME D-24 precedent
    /// `openapi::build_openapi` already applies to the thread paths.
    #[tokio::test]
    async fn openapi_json_lists_run_paths() {
        let registry = paladin::infrastructure::web::AgentRegistry::new();
        let agent_state = AgentApiState::new(Arc::new(registry));
        let spec = paladin::infrastructure::web::openapi::build_openapi(agent_state);

        assert!(
            spec.paths.paths.contains_key("/v1/runs"),
            "paths: {:?}",
            spec.paths.paths.keys().collect::<Vec<_>>()
        );
    }
}
