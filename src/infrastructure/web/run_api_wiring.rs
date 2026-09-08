//! Wires the whole Platform API (Phase 27) into a running server from configuration:
//! stores, queue, worker pool, scheduler, webhook delivery, resolvers (stored + code-
//! registered), the event bus, and the routers -- all off by default (D-50), all registered
//! with the shutdown coordinator (D-13), all fail-closed on a feature/config mismatch.
//!
//! [`build_run_api`] is the single entry point, mirroring
//! `src/bin/paladin-server.rs`'s `build_thread_state`/`thread_state_over_store` precedent
//! from Phase 24 (HITL-05, D-24..D-26): every new subsystem defaults to
//! [`crate::config::run_store::RunStoreBackend::Disabled`], in which case every new route
//! answers `501 not_implemented` and NOTHING is built beyond an unwired [`RunApiState`].
//!
//! ## Sharing one waypoint store across two engines
//!
//! The thread surface's own `WarEngine` (`paladin-server.rs`) and the run engine this
//! module builds are two SEPARATE `WarEngine` instances over the SAME durable waypoint
//! store -- required because a resume needs the run pipeline's `RunRepositoryPort`/
//! `RunQueuePort` wired onto the thread surface's `ParleyPortAdapter`
//! (`with_run_repository`/`with_run_queue`, PLAT-FR-06), which only that adapter's own
//! constructor can apply. `WarEngine<W: WaypointPort>` and `RunWorkerPool<W: WaypointPort>`
//! are both generic over a *concrete, `Sized`* store type, but the waypoint store this
//! module receives has already been erased to `Arc<dyn WaypointPort>` (the same shape
//! `ThreadApiState::waypoints` carries) by the time it reaches here. [`ErasedWaypointStore`]
//! is a minimal newtype wrapping that trait object and re-implementing
//! [`WaypointPort`] by delegation -- `Arc<ErasedWaypointStore>` is `Sized`, so it satisfies
//! `WarEngine`/`RunWorkerPool`'s generic bound without this module (or its caller) ever
//! needing to know or care whether the underlying backend is SQLite or Postgres.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use paladin_battalion::engine::WarEngine;
use paladin_battalion::engine::registries::EngineRegistries;
use paladin_battalion::engine::shutdown::ShutdownCoordinator;
use paladin_core::platform::container::assistant::AssistantSource;
use paladin_core::platform::container::run::AssistantRef;
use paladin_core::platform::container::waypoint::{ThreadId, Waypoint, WaypointId};
use paladin_ports::input::assistant_admin_port::AssistantAdminPort;
use paladin_ports::input::run_event_stream_port::RunEventStreamPort;
use paladin_ports::input::run_submission_port::RunSubmissionPort;
use paladin_ports::input::schedule_admin_port::ScheduleAdminPort;
use paladin_ports::output::assistant_repository_port::AssistantRepositoryPort;
use paladin_ports::output::run_queue_port::RunQueuePort;
use paladin_ports::output::run_repository_port::RunRepositoryPort;
use paladin_ports::output::run_schedule_repository_port::RunScheduleRepositoryPort;
use paladin_ports::output::waypoint_port::{
    ThreadSummary, WaypointError, WaypointPort, WaypointSummary,
};
use paladin_ports::output::webhook_delivery_port::WebhookDeliveryRepositoryPort;
use paladin_web::{AgentAuthConfig, AgentRegistry, RunApiState};

use crate::application::services::assistant::{
    AssistantService, AssistantValidator, ChainedResolver, StoredAssistantResolver,
};
use crate::application::services::run::resolver::{
    AssistantResolver, ResolveError, ResolvedAssistant, Runnable,
};
use crate::application::services::run::schedule::{ScheduleService, ScheduleServiceOptions};
use crate::application::services::run::submission::RunSubmissionService;
use crate::application::services::run::webhook::{
    SsrfGuard, WebhookDeliveryOptions, WebhookDeliveryService,
};
use crate::application::services::run::worker::{RunWorkerOptions, RunWorkerPool};
use crate::application::services::run::{RunEventBus, RunEventStreamService};
use crate::config::assistants::AssistantsConfig;
use crate::config::engine::EngineConfig;
use crate::config::env_utils::EnvOverridable;
use crate::config::run_queue::{RunQueueBackend, RunQueueConfig};
use crate::config::run_store::{RunStoreBackend, RunStoreConfig};
use crate::config::run_stream::RunStreamConfig;
use crate::config::run_worker::RunWorkerConfig;
use crate::config::schedules::SchedulesConfig;
use crate::config::settings::Settings;
use crate::config::webhooks::WebhooksConfig;
use crate::infrastructure::web::facade_provisioner::paladin_port_from_settings;

/// The seven X-09 config structs `paladin-server.rs` reads (`Default` +
/// `apply_env_overrides()` + `validate()`) and hands to [`build_run_api`] in one bundle.
#[derive(Debug, Clone)]
pub struct RunApiConfigs {
    /// PLAT-01: which durable Run backend (if any) is wired. `Disabled` (the default) means
    /// `build_run_api` returns an unwired [`RunApiState`] and spawns nothing.
    pub run_store: RunStoreConfig,
    /// PLAT-01: which Run dispatch queue backend is wired.
    pub run_queue: RunQueueConfig,
    /// PLAT-02: the worker pool's concurrency/lease/probe tuning.
    pub run_worker: RunWorkerConfig,
    /// PLAT-03: the degraded-mode SSE polling interval.
    pub run_stream: RunStreamConfig,
    /// PLAT-04: whether `GET /assistants` merges in synthetic code-registry entries.
    pub assistants: AssistantsConfig,
    /// PLAT-05: whether the `ScheduleService` tick loop runs at all.
    pub schedules: SchedulesConfig,
    /// PLAT-05: the webhook SSRF guard and retry-schedule tuning.
    pub webhooks: WebhooksConfig,
}

/// Everything [`build_run_api`] produces: the fully-populated [`RunApiState`], the pieces
/// `paladin-server.rs` threads onto the thread surface's `ThreadApiState` for durable
/// resume (PLAT-FR-06), and every background task registered with the caller's
/// [`ShutdownCoordinator`].
pub struct RunApiHandles {
    /// The `/v1/runs*`, `/v1/assistants*` and `/v1/schedules*` state -- merge
    /// `paladin_web::run_router(handles.run_state)` alongside the agent and thread routers.
    pub run_state: RunApiState,
    /// The [`RunSubmissionPort`] `ThreadApiState::with_run_submission` wires, so
    /// `POST /threads/{id}/fork` uses the SAME facade service `POST /runs` does. `None`
    /// when the run store is disabled.
    pub thread_run_submission: Option<Arc<dyn RunSubmissionPort>>,
    /// The [`RunRepositoryPort`] `ThreadApiState::with_runs` wires for direct reads. `None`
    /// when the run store is disabled.
    pub run_repository: Option<Arc<dyn RunRepositoryPort>>,
    /// The `(RunRepositoryPort, RunQueuePort)` pair
    /// `ParleyPortAdapter::with_run_repository`/`with_run_queue` wire so a resume against a
    /// thread with an active run row re-enqueues durably (PLAT-FR-06, D-19..D-23) instead of
    /// spawning in-process. `None` when the run store is disabled.
    pub parley_extras: Option<(Arc<dyn RunRepositoryPort>, Arc<dyn RunQueuePort>)>,
    /// Every background task this call spawned (the worker pool, the webhook delivery
    /// drain loop, and -- when `schedules.enabled` -- the schedule tick loop), all
    /// registered with the caller's own [`ShutdownCoordinator`] (D-13). Empty when the run
    /// store is disabled.
    pub tasks: Vec<JoinHandle<()>>,
}

/// Wraps an already-erased `Arc<dyn WaypointPort>` so it can be handed to APIs
/// (`WarEngine::new`, `RunWorkerPool::new`) that are generic over a concrete, `Sized`
/// `W: WaypointPort` -- delegates every method verbatim. See this module's own docs for why
/// this exists instead of threading a generic `W` through `build_run_api`'s public
/// signature.
pub struct ErasedWaypointStore(Arc<dyn WaypointPort>);

impl ErasedWaypointStore {
    /// Wrap an already-erased waypoint store.
    pub fn new(inner: Arc<dyn WaypointPort>) -> Self {
        Self(inner)
    }
}

// `dyn WaypointPort` does not implement `Debug`, so this is hand-written rather than
// derived -- deliberately shallow (never prints the inner store's internals), mirroring
// `services::run::resolver::Runnable`'s identical precedent for the same problem.
impl std::fmt::Debug for ErasedWaypointStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ErasedWaypointStore(..)")
    }
}

#[async_trait]
impl WaypointPort for ErasedWaypointStore {
    async fn save(&self, wp: &Waypoint) -> Result<(), WaypointError> {
        self.0.save(wp).await
    }

    async fn latest(&self, thread: &ThreadId) -> Result<Option<Waypoint>, WaypointError> {
        self.0.latest(thread).await
    }

    async fn get(
        &self,
        thread: &ThreadId,
        id: &WaypointId,
    ) -> Result<Option<Waypoint>, WaypointError> {
        self.0.get(thread, id).await
    }

    async fn history(
        &self,
        thread: &ThreadId,
        limit: Option<u32>,
        before: Option<WaypointId>,
    ) -> Result<Vec<WaypointSummary>, WaypointError> {
        self.0.history(thread, limit, before).await
    }

    async fn list_threads(
        &self,
        limit: Option<u32>,
        before: Option<DateTime<Utc>>,
    ) -> Result<Vec<ThreadSummary>, WaypointError> {
        self.0.list_threads(limit, before).await
    }

    async fn delete_thread(&self, thread: &ThreadId) -> Result<u64, WaypointError> {
        self.0.delete_thread(thread).await
    }

    async fn delete_waypoint(
        &self,
        thread: &ThreadId,
        id: &WaypointId,
    ) -> Result<bool, WaypointError> {
        self.0.delete_waypoint(thread, id).await
    }
}

/// Exposes code-registered agents (`AgentRegistry`, `paladin-web`) to the run pipeline
/// through the SAME `AssistantResolver` seam `CodeWorkflowResolver` implements (D-32): a
/// `POST /runs` for a code-registered agent id resolves to `Runnable::Agent`, executed
/// directly by the worker's wired `PaladinPort` (never through the engine, which has no
/// Waypoint for a bare agent) -- mirrors `CodeWorkflowResolver`'s identical version-1-only
/// convention. `AgentRegistry` itself is never mutated (X-03).
pub struct CodeAgentResolver {
    registry: Arc<AgentRegistry>,
}

impl CodeAgentResolver {
    /// Construct a resolver over `registry`.
    pub fn new(registry: Arc<AgentRegistry>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl AssistantResolver for CodeAgentResolver {
    async fn resolve(
        &self,
        assistant_id: &str,
        version: Option<u32>,
    ) -> Result<ResolvedAssistant, ResolveError> {
        if let Some(v) = version
            && v != 1
        {
            return Err(ResolveError::UnknownVersion {
                assistant_id: assistant_id.to_string(),
                version: v,
            });
        }
        let entry =
            self.registry
                .get(assistant_id)
                .ok_or_else(|| ResolveError::UnknownAssistant {
                    assistant_id: assistant_id.to_string(),
                })?;
        Ok(ResolvedAssistant {
            reference: AssistantRef {
                assistant_id: assistant_id.to_string(),
                version: 1,
            },
            runnable: Runnable::Agent(entry.paladin),
            allowed_roles: entry.allowed_roles,
            source: AssistantSource::Code,
        })
    }
}

impl From<&RunWorkerConfig> for RunWorkerOptions {
    fn from(cfg: &RunWorkerConfig) -> Self {
        Self {
            concurrency: cfg.concurrency as usize,
            lease: Duration::from_secs(cfg.lease_seconds),
            min_probe_interval: Duration::from_millis(cfg.min_probe_interval_ms),
        }
    }
}

impl From<&WebhooksConfig> for WebhookDeliveryOptions {
    fn from(cfg: &WebhooksConfig) -> Self {
        Self {
            max_attempts: cfg.max_attempts,
            timeout: Duration::from_secs(cfg.timeout_secs),
            allow_private: cfg.allow_private,
            ..Default::default()
        }
    }
}

impl From<&SchedulesConfig> for ScheduleServiceOptions {
    fn from(cfg: &SchedulesConfig) -> Self {
        Self {
            tick_interval: Duration::from_millis(cfg.tick_interval_ms),
            ..Default::default()
        }
    }
}

/// The four repositories sharing the run store's own backend choice (D-50): every one of
/// them connects over the SAME sqlite file / postgres database `configs.run_store` names.
type RepoQuartet = (
    Arc<dyn RunRepositoryPort>,
    Arc<dyn AssistantRepositoryPort>,
    Arc<dyn RunScheduleRepositoryPort>,
    Arc<dyn WebhookDeliveryRepositoryPort>,
);

async fn build_sqlite_quartet(path: &str) -> Result<RepoQuartet, Box<dyn std::error::Error>> {
    let run_repository: Arc<dyn RunRepositoryPort> = Arc::new(
        paladin_storage::run::sqlite::SqliteRunRepository::new(path)
            .await
            .map_err(|e| format!("failed to open sqlite run store at '{path}': {e}"))?,
    );
    let assistant_repository: Arc<dyn AssistantRepositoryPort> = Arc::new(
        paladin_storage::assistant::sqlite::SqliteAssistantRepository::new(path)
            .await
            .map_err(|e| format!("failed to open sqlite assistant store at '{path}': {e}"))?,
    );
    let schedule_repository: Arc<dyn RunScheduleRepositoryPort> = Arc::new(
        paladin_storage::run_schedule::sqlite::SqliteRunScheduleRepository::new(path)
            .await
            .map_err(|e| format!("failed to open sqlite run schedule store at '{path}': {e}"))?,
    );
    let webhook_repository: Arc<dyn WebhookDeliveryRepositoryPort> = Arc::new(
        paladin_storage::webhook::sqlite::SqliteWebhookDeliveryRepository::new(path)
            .await
            .map_err(|e| {
                format!("failed to open sqlite webhook delivery store at '{path}': {e}")
            })?,
    );
    Ok((
        run_repository,
        assistant_repository,
        schedule_repository,
        webhook_repository,
    ))
}

#[cfg(feature = "storage-postgres")]
async fn build_postgres_quartet(url_env: &str) -> Result<RepoQuartet, Box<dyn std::error::Error>> {
    let url = std::env::var(url_env).map_err(|_| {
        format!("run store postgres backend names env var '{url_env}', which is not set")
    })?;
    let run_repository: Arc<dyn RunRepositoryPort> = Arc::new(
        paladin_storage::run::postgres::PostgresRunRepository::new(&url)
            .await
            .map_err(|e| format!("failed to open postgres run store: {e}"))?,
    );
    let assistant_repository: Arc<dyn AssistantRepositoryPort> = Arc::new(
        paladin_storage::assistant::postgres::PostgresAssistantRepository::new(&url)
            .await
            .map_err(|e| format!("failed to open postgres assistant store: {e}"))?,
    );
    let schedule_repository: Arc<dyn RunScheduleRepositoryPort> = Arc::new(
        paladin_storage::run_schedule::postgres::PostgresRunScheduleRepository::new(&url)
            .await
            .map_err(|e| format!("failed to open postgres run schedule store: {e}"))?,
    );
    let webhook_repository: Arc<dyn WebhookDeliveryRepositoryPort> = Arc::new(
        paladin_storage::webhook::postgres::PostgresWebhookDeliveryRepository::new(&url)
            .await
            .map_err(|e| format!("failed to open postgres webhook delivery store: {e}"))?,
    );
    Ok((
        run_repository,
        assistant_repository,
        schedule_repository,
        webhook_repository,
    ))
}

/// When this binary is built without `storage-postgres`, a configured `Postgres` run store
/// backend is a startup error naming the missing feature, never a silent fallback --
/// mirrors `paladin-server.rs`'s own `build_postgres_thread_state` twin.
#[cfg(not(feature = "storage-postgres"))]
async fn build_postgres_quartet(url_env: &str) -> Result<RepoQuartet, Box<dyn std::error::Error>> {
    Err(format!(
        "run_store.backend is configured as 'postgres' (env var '{url_env}') but this binary \
         was built without the 'storage-postgres' feature; rebuild with \
         --features storage-postgres,web-server, or set APP_RUN_STORE_BACKEND=disabled or \
         =sqlite"
    )
    .into())
}

#[cfg(feature = "redis-queue")]
async fn build_redis_run_queue(
    url_env: &str,
    key_prefix: &str,
) -> Result<Arc<dyn RunQueuePort>, Box<dyn std::error::Error>> {
    let url = std::env::var(url_env).map_err(|_| {
        format!("run queue redis backend names env var '{url_env}', which is not set")
    })?;
    let config = paladin_storage::run_queue::redis::RedisRunQueueConfig {
        connection_url: url,
        key_prefix: key_prefix.to_string(),
    };
    let queue = paladin_storage::run_queue::redis::RedisRunQueue::new(config)
        .await
        .map_err(|e| format!("failed to connect to redis run queue: {e}"))?;
    Ok(Arc::new(queue))
}

/// When this binary is built without `redis-queue`, a configured `Redis` run queue backend
/// is a startup error naming the missing feature, never a silent `InMemory` fallback.
#[cfg(not(feature = "redis-queue"))]
async fn build_redis_run_queue(
    url_env: &str,
    _key_prefix: &str,
) -> Result<Arc<dyn RunQueuePort>, Box<dyn std::error::Error>> {
    Err(format!(
        "run_queue.backend is configured as 'redis' (env var '{url_env}') but this binary was \
         built without the 'redis-queue' feature; rebuild with --features redis-queue,web-server, \
         or set APP_RUN_QUEUE_BACKEND=in_memory"
    )
    .into())
}

/// Turn `configs` into the fully wired run API: stores, queue, worker pool, scheduler,
/// webhook delivery, resolvers, the event bus, and the `RunApiState` every new route reads
/// -- off by default (D-50) and fail-closed on a feature/config mismatch.
///
/// `waypoint_store` is the SAME store `paladin-server.rs`'s thread surface uses (already
/// erased to `Arc<dyn WaypointPort>`) -- `None` when no waypoint backend is configured at
/// all. A configured `run_store` backend with no waypoint store wired is a startup error
/// naming `waypoint_store.backend`: the run engine cannot execute a `WarGraph` without
/// somewhere to persist its Waypoints.
///
/// # Errors
///
/// Returns an error if: `run_store` names `postgres`/`run_queue` names `redis` on a binary
/// built without the matching feature (naming the feature); `run_store` is enabled but no
/// waypoint store is wired (naming `waypoint_store.backend`); a configured backend fails to
/// connect or migrate; or the run engine's `PaladinPort` cannot be built from `settings`
/// (an unresolvable default LLM provider).
pub async fn build_run_api(
    configs: RunApiConfigs,
    settings: &Settings,
    coordinator: ShutdownCoordinator,
    waypoint_store: Option<Arc<dyn WaypointPort>>,
    auth: AgentAuthConfig,
    code_registry: Arc<AgentRegistry>,
) -> Result<RunApiHandles, Box<dyn std::error::Error>> {
    if matches!(configs.run_store.backend, RunStoreBackend::Disabled) {
        return Ok(RunApiHandles {
            run_state: RunApiState::new().with_auth(auth),
            thread_run_submission: None,
            run_repository: None,
            parley_extras: None,
            tasks: Vec::new(),
        });
    }

    let Some(waypoint_store) = waypoint_store else {
        return Err(
            "run_store.backend is configured but no waypoint store is wired: set \
             waypoint_store.backend"
                .into(),
        );
    };

    let (run_repository, assistant_repository, schedule_repository, webhook_repository) =
        match &configs.run_store.backend {
            RunStoreBackend::Disabled => unreachable!("handled above"),
            RunStoreBackend::Sqlite { path } => build_sqlite_quartet(path).await?,
            RunStoreBackend::Postgres { url_env } => build_postgres_quartet(url_env).await?,
        };

    let run_queue: Arc<dyn RunQueuePort> = match &configs.run_queue.backend {
        RunQueueBackend::InMemory => {
            Arc::new(paladin_storage::run_queue::in_memory::InMemoryRunQueue::new())
        }
        RunQueueBackend::Redis {
            url_env,
            key_prefix,
        } => build_redis_run_queue(url_env, key_prefix).await?,
    };

    // Resolvers: stored assistants first, code-registered agents second (D-32).
    let engine_registries = Arc::new(EngineRegistries::new());
    let validator = Arc::new(AssistantValidator::new(Arc::clone(&engine_registries)));
    let stored_resolver: Arc<dyn AssistantResolver> = Arc::new(StoredAssistantResolver::new(
        Arc::clone(&assistant_repository),
        Arc::clone(&validator),
    ));
    let code_resolver: Arc<dyn AssistantResolver> =
        Arc::new(CodeAgentResolver::new(Arc::clone(&code_registry)));
    let resolver: Arc<dyn AssistantResolver> =
        Arc::new(ChainedResolver::new(stored_resolver, code_resolver));

    // The run engine's real PaladinPort (D-44's "the run pipeline uses a real port"): the
    // SAME default-provider resolution `FacadeProvisioner` uses.
    let paladin_port = paladin_port_from_settings(settings)
        .map_err(|e| format!("failed to build the run engine's LLM port: {e}"))?;

    let mut engine_config = EngineConfig::default();
    engine_config.apply_env_overrides();
    engine_config
        .validate()
        .map_err(|e| format!("invalid engine configuration: {e}"))?;

    let erased_store = Arc::new(ErasedWaypointStore::new(Arc::clone(&waypoint_store)));

    // D-24: the per-run broadcast bus, read by `GET /runs/{run_id}/stream`.
    let event_bus = Arc::new(RunEventBus::new());

    // D-14/D-16: a fresh, per-run engine carrying its own child CancellationToken of the
    // pool's own coordinator, so process shutdown still cascades. `run_once` itself attaches
    // the cancellation probe and the event-bus trace sink to whatever this factory returns
    // (see `RunWorkerPool::with_cancellation_probing`/`with_event_bus`'s own rustdoc) --
    // this closure only wires the per-run token.
    let engine_factory: Arc<
        dyn Fn(CancellationToken) -> WarEngine<ErasedWaypointStore> + Send + Sync,
    > = {
        let paladin_port = Arc::clone(&paladin_port);
        let store = Arc::clone(&erased_store);
        let durability = engine_config.waypoint_durability;
        let grace = Duration::from_secs(engine_config.shutdown_grace_secs);
        Arc::new(move |token: CancellationToken| {
            WarEngine::new(Arc::clone(&paladin_port), Arc::clone(&store))
                .with_durability(durability)
                .with_shutdown_grace(grace)
                .with_cancellation_token(token)
        })
    };

    let base_engine = Arc::new(
        WarEngine::new(Arc::clone(&paladin_port), Arc::clone(&erased_store))
            .with_durability(engine_config.waypoint_durability)
            .with_shutdown_grace(Duration::from_secs(engine_config.shutdown_grace_secs)),
    );

    let pool = Arc::new(
        RunWorkerPool::new(
            base_engine,
            Arc::clone(&erased_store),
            Arc::clone(&run_repository),
            Arc::clone(&run_queue),
            Arc::clone(&resolver),
            Duration::from_secs(configs.run_worker.lease_seconds),
        )
        .with_shutdown_coordinator(coordinator.clone())
        .with_paladin_port(Arc::clone(&paladin_port))
        .with_engine_factory(engine_factory)
        .with_cancellation_probing(Duration::from_millis(
            configs.run_worker.min_probe_interval_ms,
        ))
        .with_event_bus(Arc::clone(&event_bus))
        .with_webhook_deliveries(Arc::clone(&webhook_repository)),
    );

    let mut tasks = Arc::clone(&pool).spawn(RunWorkerOptions::from(&configs.run_worker));

    let submission_service = RunSubmissionService::new(
        Arc::clone(&run_repository),
        Arc::clone(&run_queue),
        Arc::clone(&resolver),
    )
    .with_local_tokens(pool.local_tokens())
    .with_waypoints(Arc::clone(&waypoint_store))
    .with_ssrf_guard(SsrfGuard::new(configs.webhooks.allow_private));
    let run_submission: Arc<dyn RunSubmissionPort> = Arc::new(submission_service);

    // PLAT-05: the schedule tick loop is spawned only when `schedules.enabled` (D-50) --
    // the persisted schema/repository is unaffected by this flag, only the periodic
    // claim-and-fire loop.
    let schedules: Option<Arc<dyn ScheduleAdminPort>> = if configs.schedules.enabled {
        let schedule_service = Arc::new(
            ScheduleService::new(
                Arc::clone(&schedule_repository),
                Arc::clone(&run_submission),
                ScheduleServiceOptions::from(&configs.schedules),
            )
            .with_resolver(Arc::clone(&resolver))
            .with_ssrf_guard(SsrfGuard::new(configs.webhooks.allow_private)),
        );
        tasks.push(Arc::clone(&schedule_service).spawn(&coordinator));
        Some(schedule_service as Arc<dyn ScheduleAdminPort>)
    } else {
        None
    };

    // PLAT-05: the webhook delivery drain loop is always spawned when the run store is
    // enabled -- delivery is a persisted, durable queue (D-40), not gated on any per-run
    // webhook actually being configured.
    let webhook_service = WebhookDeliveryService::new(
        Arc::clone(&webhook_repository),
        Arc::clone(&run_repository),
        WebhookDeliveryOptions::from(&configs.webhooks),
    )
    .map_err(|e| format!("failed to build the webhook delivery HTTP client: {e}"))?;
    tasks.push(Arc::new(webhook_service).spawn(&coordinator));

    let assistant_service: Arc<dyn AssistantAdminPort> = Arc::new(AssistantService::new(
        Arc::clone(&assistant_repository),
        Arc::clone(&validator),
    ));

    let run_events: Arc<dyn RunEventStreamPort> = Arc::new(RunEventStreamService::new(
        Arc::clone(&event_bus),
        Arc::clone(&run_repository),
        Arc::clone(&waypoint_store),
        Duration::from_millis(configs.run_stream.poll_interval_ms),
    ));

    let mut run_state = RunApiState::new()
        .with_submission(Arc::clone(&run_submission))
        .with_repository(Arc::clone(&run_repository))
        .with_run_events(run_events)
        .with_assistants(assistant_service)
        .with_code_registry(code_registry)
        .with_expose_code_registry(configs.assistants.expose_code_registry)
        .with_webhook_deliveries(Arc::clone(&webhook_repository))
        .with_auth(auth);
    if let Some(schedules) = schedules {
        run_state = run_state.with_schedules(schedules);
    }

    Ok(RunApiHandles {
        run_state,
        thread_run_submission: Some(Arc::clone(&run_submission)),
        run_repository: Some(Arc::clone(&run_repository)),
        parley_extras: Some((run_repository, run_queue)),
        tasks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    fn default_configs() -> RunApiConfigs {
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

    fn temp_sqlite_url(label: &str) -> (std::path::PathBuf, String) {
        let path = std::env::temp_dir().join(format!(
            "paladin_run_api_wiring_test_{label}_{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let url = format!("sqlite://{}", path.display());
        (path, url)
    }

    fn cleanup(path: &std::path::PathBuf) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path.display()));
        let _ = std::fs::remove_file(format!("{}-shm", path.display()));
    }

    #[tokio::test]
    async fn defaults_wire_nothing_and_answer_501() {
        let handles = build_run_api(
            default_configs(),
            &Settings::default(),
            ShutdownCoordinator::new(),
            None,
            AgentAuthConfig::default(),
            Arc::new(AgentRegistry::new()),
        )
        .await
        .expect("disabled run store never fails to build");

        assert!(handles.tasks.is_empty(), "no task spawned when disabled");
        assert!(handles.run_repository.is_none());
        assert!(handles.thread_run_submission.is_none());
        assert!(handles.parley_extras.is_none());

        let app = paladin_web::run_router(handles.run_state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/runs")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"assistant_id":"any"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn missing_waypoint_store_errors_naming_the_config_key() {
        let mut configs = default_configs();
        configs.run_store.backend = RunStoreBackend::Sqlite {
            path: "sqlite::memory:".to_string(),
        };

        let err = build_run_api(
            configs,
            &Settings::default(),
            ShutdownCoordinator::new(),
            None,
            AgentAuthConfig::default(),
            Arc::new(AgentRegistry::new()),
        )
        .await
        .err()
        .expect("a run store without a waypoint store must fail closed");
        assert!(
            err.to_string().contains("waypoint_store.backend"),
            "error must name the config key to set: {err}"
        );
    }

    #[tokio::test]
    #[serial_test::serial(paladin_run_api_wiring_openai_api_key)]
    async fn sqlite_and_in_memory_wires_three_tasks_and_every_state_field() {
        // `paladin_port_from_settings` resolves a real provider adapter through
        // `LlmProviderFactory` (the SAME default-provider resolution `FacadeProvisioner`
        // uses) -- construction only checks that a key is PRESENT, never that it is valid,
        // so a dummy value is enough to prove the wiring without a network call.
        // `#[serial]` avoids racing this env var against any other test in the same binary.
        unsafe {
            std::env::set_var("OPENAI_API_KEY", "sk-test-run-api-wiring-hermetic");
        }

        let (run_path, run_url) = temp_sqlite_url("run_store");
        let (wp_path, wp_url) = temp_sqlite_url("waypoints");

        let waypoint_store: Arc<dyn WaypointPort> = Arc::new(
            paladin_storage::waypoint::sqlite::SqliteWaypointStore::new(&wp_url)
                .await
                .expect("waypoint store opens"),
        );

        let mut configs = default_configs();
        configs.run_store.backend = RunStoreBackend::Sqlite {
            path: run_url.clone(),
        };
        configs.run_queue.backend = RunQueueBackend::InMemory;
        configs.run_worker.concurrency = 1;
        configs.schedules.enabled = true;

        let handles = build_run_api(
            configs,
            &Settings::default(),
            ShutdownCoordinator::new(),
            Some(waypoint_store),
            AgentAuthConfig::default(),
            Arc::new(AgentRegistry::new()),
        )
        .await
        .expect("sqlite + in_memory wires successfully");

        unsafe {
            std::env::remove_var("OPENAI_API_KEY");
        }

        assert_eq!(
            handles.tasks.len(),
            3,
            "expected worker(1) + webhook(1) + schedule(1) tasks"
        );
        assert!(handles.run_state.run_submission.is_some());
        assert!(handles.run_state.run_repository.is_some());
        assert!(handles.run_state.run_events.is_some());
        assert!(handles.run_state.assistants.is_some());
        assert!(handles.run_state.schedules.is_some());
        assert!(handles.run_state.webhook_deliveries.is_some());
        assert!(handles.run_repository.is_some());
        assert!(handles.thread_run_submission.is_some());
        assert!(handles.parley_extras.is_some());

        cleanup(&run_path);
        cleanup(&wp_path);
    }
}

#[cfg(all(test, not(feature = "storage-postgres")))]
mod postgres_feature_gate_tests {
    use super::*;

    #[tokio::test]
    async fn postgres_run_store_without_the_feature_errors_naming_it() {
        let mut configs = RunApiConfigs {
            run_store: RunStoreConfig::default(),
            run_queue: RunQueueConfig::default(),
            run_worker: RunWorkerConfig::default(),
            run_stream: RunStreamConfig::default(),
            assistants: AssistantsConfig::default(),
            schedules: SchedulesConfig::default(),
            webhooks: WebhooksConfig::default(),
        };
        // Named so `validate()` (not exercised here) would also pass, but
        // `build_run_api` itself must fail closed BEFORE ever touching the
        // network, naming the missing cargo feature.
        unsafe {
            std::env::set_var("APP_RUN_API_WIRING_TEST_PG_URL", "postgres://unused/db");
        }
        configs.run_store.backend = RunStoreBackend::Postgres {
            url_env: "APP_RUN_API_WIRING_TEST_PG_URL".to_string(),
        };

        let waypoint_store: Arc<dyn WaypointPort> =
            Arc::new(paladin_storage::waypoint::in_memory::InMemoryWaypointStore::new());

        let err = build_run_api(
            configs,
            &Settings::default(),
            ShutdownCoordinator::new(),
            Some(waypoint_store),
            AgentAuthConfig::default(),
            Arc::new(AgentRegistry::new()),
        )
        .await
        .err()
        .expect("postgres without the feature must fail closed");
        assert!(
            err.to_string().contains("storage-postgres"),
            "error must name the missing feature: {err}"
        );

        unsafe {
            std::env::remove_var("APP_RUN_API_WIRING_TEST_PG_URL");
        }
    }
}

#[cfg(all(test, not(feature = "redis-queue")))]
mod redis_feature_gate_tests {
    use super::*;

    #[tokio::test]
    async fn redis_run_queue_without_the_feature_errors_naming_it() {
        let mut configs = RunApiConfigs {
            run_store: RunStoreConfig::default(),
            run_queue: RunQueueConfig::default(),
            run_worker: RunWorkerConfig::default(),
            run_stream: RunStreamConfig::default(),
            assistants: AssistantsConfig::default(),
            schedules: SchedulesConfig::default(),
            webhooks: WebhooksConfig::default(),
        };
        configs.run_store.backend = RunStoreBackend::Sqlite {
            path: "sqlite::memory:".to_string(),
        };
        unsafe {
            std::env::set_var("APP_RUN_API_WIRING_TEST_REDIS_URL", "redis://unused:6379");
        }
        configs.run_queue.backend = RunQueueBackend::Redis {
            url_env: "APP_RUN_API_WIRING_TEST_REDIS_URL".to_string(),
            key_prefix: "paladin:run_queue".to_string(),
        };

        let waypoint_store: Arc<dyn WaypointPort> =
            Arc::new(paladin_storage::waypoint::in_memory::InMemoryWaypointStore::new());

        let err = build_run_api(
            configs,
            &Settings::default(),
            ShutdownCoordinator::new(),
            Some(waypoint_store),
            AgentAuthConfig::default(),
            Arc::new(AgentRegistry::new()),
        )
        .await
        .err()
        .expect("redis without the feature must fail closed");
        assert!(
            err.to_string().contains("redis-queue"),
            "error must name the missing feature: {err}"
        );

        unsafe {
            std::env::remove_var("APP_RUN_API_WIRING_TEST_REDIS_URL");
        }
    }
}
