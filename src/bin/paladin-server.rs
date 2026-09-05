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

use log::{error, info, warn};
use paladin::config::agents::AuthConfig;
use paladin::config::engine::EngineConfig;
use paladin::config::env_utils::EnvOverridable;
use paladin::config::settings::Settings;
use paladin::infrastructure::adapters::auth::InMemoryTokenAuthAdapter;
use paladin::infrastructure::web::agent_host::{bind_address, build_agent_registry};
use paladin::infrastructure::web::facade_provisioner::FacadeProvisioner;
use paladin::infrastructure::web::{
    AgentApiState, AgentAuthConfig, HttpLayersConfig, Principal, RateLimitConfig, TimeoutPolicy,
    agent_router, with_http_layers,
};
use paladin_battalion::engine::shutdown::ShutdownCoordinator;
use paladin_ports::output::auth_port::AuthPort;
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
    let state = AgentApiState::new(Arc::new(registry))
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
    let routes = agent_router(state.clone());
    let routes = if docs_enabled {
        let spec = paladin::infrastructure::web::openapi::build_openapi(state);
        routes.merge(paladin::infrastructure::web::openapi::docs_router(spec))
    } else {
        routes
    };
    let app = with_http_layers(routes, &layers);

    let listener = tokio::net::TcpListener::bind(bind_address(&settings)).await?;
    let bound = listener.local_addr()?;
    info!(
        "paladin-server listening on http://{bound} — serving {} agent(s): {:?}",
        agent_ids.len(),
        agent_ids
    );
    info!(
        "routes: GET /health, GET /ready, GET/POST /v1/agents, GET/DELETE /v1/agents/{{id}}, POST /v1/agents/{{id}}/execute[/stream], POST /v1/agents/{{id}}/jobs, GET /v1/agents/{{id}}/jobs/{{job_id}}"
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
async fn drain_on_shutdown(_coordinator: &ShutdownCoordinator, _grace: Duration, _graceful: bool) {
    // RED: not yet wired to the coordinator (Phase 24 Plan 09, HITL-04, D-22).
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
        use paladin_battalion::engine::node::{NodeContext, NodeError, StateNode};
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
            ) -> Result<Directive, NodeError> {
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
}
