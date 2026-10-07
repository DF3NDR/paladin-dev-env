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

use std::collections::BTreeSet;
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
use paladin_core::platform::container::principal::TenantId;
use paladin_core::platform::container::run::AssistantRef;
use paladin_core::platform::container::waypoint::{ThreadId, Waypoint, WaypointId};
use paladin_ports::input::allowance_admission_port::AllowanceAdmissionPort;
use paladin_ports::input::assistant_admin_port::AssistantAdminPort;
use paladin_ports::input::run_event_stream_port::RunEventStreamPort;
use paladin_ports::input::run_submission_port::RunSubmissionPort;
use paladin_ports::input::schedule_admin_port::ScheduleAdminPort;
use paladin_ports::output::assistant_repository_port::AssistantRepositoryPort;
use paladin_ports::output::run_queue_port::RunQueuePort;
use paladin_ports::output::run_repository_port::RunRepositoryPort;
use paladin_ports::output::run_schedule_repository_port::RunScheduleRepositoryPort;
use paladin_ports::output::run_trace_port::RunTracePort;
use paladin_ports::output::treasury_ledger_port::TreasuryLedgerPort;
use paladin_ports::output::treasury_notice_port::TreasuryNoticePort;
use paladin_ports::output::waypoint_port::{
    ThreadSummary, WaypointError, WaypointPort, WaypointSummary,
};
use paladin_ports::output::webhook_delivery_port::WebhookDeliveryRepositoryPort;
use paladin_web::agent_auth::OPEN_ACCESS_PRINCIPAL_ID;
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
use crate::application::services::treasurer::{OperatorNoticeTarget, Treasurer};
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
use crate::infrastructure::web::facade_provisioner::paladin_port_from_settings_with_ledger;

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
    /// The Treasurer's admission port (D-06), shared with paladin-server's `AgentApiState`
    /// (41-04). `None` when `treasurer.allowance` has no entries.
    pub treasurer: Option<Arc<dyn AllowanceAdmissionPort>>,
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

/// Build the Treasurer's durable spend ledger from the SAME [`RunStoreBackend`] selection
/// `build_run_api` already reads for the run repository (D-00g: no new store config) --
/// `Disabled` -> `Ok(None)` (D-08: no ledger installed when no backend is configured);
/// `Sqlite { path }` -> a [`SqliteTreasuryLedger`](paladin_storage::treasury::sqlite::SqliteTreasuryLedger)
/// sharing that exact database file; `Postgres { url_env }` -> a
/// [`PostgresTreasuryLedger`](paladin_storage::treasury::postgres::PostgresTreasuryLedger)
/// sharing that exact database, on a build with `storage-postgres` (a named-feature error
/// otherwise, mirroring `build_postgres_quartet`'s own precedent).
///
/// # Errors
///
/// Returns an error naming the sqlite path, the postgres env var, or (without
/// `storage-postgres`) the missing cargo feature -- never the database URL's own value
/// (T-39-04).
pub async fn build_treasury_ledger(
    config: &RunStoreConfig,
) -> Result<Option<Arc<dyn TreasuryLedgerPort>>, Box<dyn std::error::Error>> {
    match &config.backend {
        RunStoreBackend::Disabled => Ok(None),
        RunStoreBackend::Sqlite { path } => {
            let ledger = paladin_storage::treasury::sqlite::SqliteTreasuryLedger::new(path)
                .await
                .map_err(|e| format!("failed to open sqlite treasury ledger at '{path}': {e}"))?;
            Ok(Some(Arc::new(ledger) as Arc<dyn TreasuryLedgerPort>))
        }
        RunStoreBackend::Postgres { url_env } => build_postgres_treasury_ledger(url_env).await,
    }
}

#[cfg(feature = "storage-postgres")]
async fn build_postgres_treasury_ledger(
    url_env: &str,
) -> Result<Option<Arc<dyn TreasuryLedgerPort>>, Box<dyn std::error::Error>> {
    let url = std::env::var(url_env).map_err(|_| {
        format!("run store postgres backend names env var '{url_env}', which is not set")
    })?;
    let ledger = paladin_storage::treasury::postgres::PostgresTreasuryLedger::new(&url)
        .await
        .map_err(|e| format!("failed to open postgres treasury ledger: {e}"))?;
    Ok(Some(Arc::new(ledger) as Arc<dyn TreasuryLedgerPort>))
}

/// When this binary is built without `storage-postgres`, a configured `Postgres` run store
/// backend is a startup error naming the missing feature, never a silent fallback --
/// mirrors [`build_postgres_quartet`]'s own twin.
#[cfg(not(feature = "storage-postgres"))]
async fn build_postgres_treasury_ledger(
    url_env: &str,
) -> Result<Option<Arc<dyn TreasuryLedgerPort>>, Box<dyn std::error::Error>> {
    Err(format!(
        "run_store.backend is configured as 'postgres' (env var '{url_env}') but this binary \
         was built without the 'storage-postgres' feature; rebuild with \
         --features storage-postgres,web-server, or set APP_RUN_STORE_BACKEND=disabled or \
         =sqlite"
    )
    .into())
}

/// Build the Treasurer's once-per-window notice store (ALLOW-04, D-16) from the SAME
/// [`RunStoreBackend`] selection [`build_treasury_ledger`] reads, arm for arm: `Disabled` ->
/// `Ok(None)`; `Sqlite { path }` -> a
/// [`SqliteTreasuryLedger`](paladin_storage::treasury::sqlite::SqliteTreasuryLedger) on that
/// exact database file (it implements [`TreasuryNoticePort`] beside the ledger port, and its
/// construction applies the `treasury_notices` migration); `Postgres { url_env }` -> a
/// [`PostgresTreasuryLedger`](paladin_storage::treasury::postgres::PostgresTreasuryLedger) on
/// that exact database, on a build with `storage-postgres` (a named-feature error otherwise).
///
/// # Errors
///
/// Returns an error naming the sqlite path, the postgres env var, or (without
/// `storage-postgres`) the missing cargo feature -- never the database URL's own value
/// (T-39-04).
pub async fn build_treasury_notices(
    config: &RunStoreConfig,
) -> Result<Option<Arc<dyn TreasuryNoticePort>>, Box<dyn std::error::Error>> {
    match &config.backend {
        RunStoreBackend::Disabled => Ok(None),
        RunStoreBackend::Sqlite { path } => {
            let store = paladin_storage::treasury::sqlite::SqliteTreasuryLedger::new(path)
                .await
                .map_err(|e| format!("failed to open sqlite treasury notices at '{path}': {e}"))?;
            Ok(Some(Arc::new(store) as Arc<dyn TreasuryNoticePort>))
        }
        RunStoreBackend::Postgres { url_env } => build_postgres_treasury_notices(url_env).await,
    }
}

#[cfg(feature = "storage-postgres")]
async fn build_postgres_treasury_notices(
    url_env: &str,
) -> Result<Option<Arc<dyn TreasuryNoticePort>>, Box<dyn std::error::Error>> {
    let url = std::env::var(url_env).map_err(|_| {
        format!("run store postgres backend names env var '{url_env}', which is not set")
    })?;
    let store = paladin_storage::treasury::postgres::PostgresTreasuryLedger::new(&url)
        .await
        .map_err(|e| format!("failed to open postgres treasury notices: {e}"))?;
    Ok(Some(Arc::new(store) as Arc<dyn TreasuryNoticePort>))
}

/// Without `storage-postgres` a configured `Postgres` run store backend is a startup error
/// naming the missing feature, never a silent fallback -- mirrors
/// [`build_postgres_treasury_ledger`]'s own twin.
#[cfg(not(feature = "storage-postgres"))]
async fn build_postgres_treasury_notices(
    url_env: &str,
) -> Result<Option<Arc<dyn TreasuryNoticePort>>, Box<dyn std::error::Error>> {
    Err(format!(
        "run_store.backend is configured as 'postgres' (env var '{url_env}') but this binary \
         was built without the 'storage-postgres' feature; rebuild with \
         --features storage-postgres,web-server, or set APP_RUN_STORE_BACKEND=disabled or \
         =sqlite"
    )
    .into())
}

/// Build the durable trace store (OBS-02, D-17) from the SAME [`RunStoreBackend`] selection
/// [`build_treasury_ledger`] reads, arm for arm: `Disabled` -> `Ok(None)`; `Sqlite { path }` ->
/// a [`SqliteRunTraceStore`](paladin_storage::run_trace::sqlite::SqliteRunTraceStore) on that
/// exact database file (its construction applies the `run_traces` migration); `Postgres {
/// url_env }` -> a
/// [`PostgresRunTraceStore`](paladin_storage::run_trace::postgres::PostgresRunTraceStore) on
/// that exact database, on a build with `storage-postgres` (a named-feature error otherwise).
///
/// # Errors
///
/// Returns an error naming the sqlite path, the postgres env var, or (without
/// `storage-postgres`) the missing cargo feature -- never the database URL's own value
/// (T-39-04).
async fn build_run_trace_store(
    config: &RunStoreConfig,
) -> Result<Option<Arc<dyn RunTracePort>>, Box<dyn std::error::Error>> {
    match &config.backend {
        RunStoreBackend::Disabled => Ok(None),
        RunStoreBackend::Sqlite { path } => {
            let store = paladin_storage::run_trace::sqlite::SqliteRunTraceStore::new(path)
                .await
                .map_err(|e| format!("failed to open sqlite trace store at '{path}': {e}"))?;
            Ok(Some(Arc::new(store) as Arc<dyn RunTracePort>))
        }
        RunStoreBackend::Postgres { url_env } => build_postgres_run_trace_store(url_env).await,
    }
}

#[cfg(feature = "storage-postgres")]
async fn build_postgres_run_trace_store(
    url_env: &str,
) -> Result<Option<Arc<dyn RunTracePort>>, Box<dyn std::error::Error>> {
    let url = std::env::var(url_env).map_err(|_| {
        format!("run store postgres backend names env var '{url_env}', which is not set")
    })?;
    let store = paladin_storage::run_trace::postgres::PostgresRunTraceStore::new(&url)
        .await
        .map_err(|e| format!("failed to open postgres trace store: {e}"))?;
    Ok(Some(Arc::new(store) as Arc<dyn RunTracePort>))
}

/// Without `storage-postgres` a configured `Postgres` run store backend is a startup error
/// naming the missing feature, never a silent fallback -- mirrors
/// [`build_postgres_treasury_ledger`]'s own twin.
#[cfg(not(feature = "storage-postgres"))]
async fn build_postgres_run_trace_store(
    url_env: &str,
) -> Result<Option<Arc<dyn RunTracePort>>, Box<dyn std::error::Error>> {
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
/// (an unresolvable default LLM provider); or `treasurer.allowance` is incoherent (D-11):
/// it has entries while `run_store.backend` is `disabled`, an `api_keys.<name>` entry names
/// no key in `http.auth.api_keys`, or a `tenants.<id>` entry names a tenant no key maps to.
/// The coherence check runs first, so a disabled run store cannot hide it.
pub async fn build_run_api(
    configs: RunApiConfigs,
    settings: &Settings,
    coordinator: ShutdownCoordinator,
    waypoint_store: Option<Arc<dyn WaypointPort>>,
    auth: AgentAuthConfig,
    code_registry: Arc<AgentRegistry>,
) -> Result<RunApiHandles, Box<dyn std::error::Error>> {
    // D-11: an allowance that would silently enforce nothing is a boot error, checked BEFORE
    // the disabled-store early return so a disabled store cannot hide a configured allowance.
    let treasurer_config = settings.get_treasurer_config();
    let (known_tenants, known_api_keys) = known_allowance_targets(&auth);
    treasurer_config.allowance.validate_against(
        &known_tenants,
        &known_api_keys,
        matches!(configs.run_store.backend, RunStoreBackend::Disabled),
    )?;

    if matches!(configs.run_store.backend, RunStoreBackend::Disabled) {
        return Ok(RunApiHandles {
            run_state: RunApiState::new().with_auth(auth),
            thread_run_submission: None,
            run_repository: None,
            parley_extras: None,
            treasurer: None,
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

    // ALLOW-04, D-17, T-41-38: the operator webhook URL passes the SSRF guard at boot, failing
    // closed with the config key path in the message, BEFORE anything is spawned. The delivery
    // service re-checks it at send time through its own guard. A private or loopback target
    // needs `webhooks.allow_private`; the cloud metadata address is always rejected. Only
    // checked when a Treasurer will actually be built (an allowance entry exists).
    if !treasurer_config.allowance.is_empty()
        && let Some(webhook) = &treasurer_config.allowance.webhook
    {
        SsrfGuard::new(configs.webhooks.allow_private)
            .check_url(&webhook.url)
            .await
            .map_err(|rejection| {
                format!(
                    "treasurer.allowance.webhook.url rejected by the SSRF guard: {rejection} -- a \
                     private or loopback target needs webhooks.allow_private: true (the cloud \
                     metadata address is always rejected)"
                )
            })?;
    }

    let (run_repository, assistant_repository, schedule_repository, webhook_repository) =
        match &configs.run_store.backend {
            RunStoreBackend::Disabled => unreachable!("handled above"),
            RunStoreBackend::Sqlite { path } => build_sqlite_quartet(path).await?,
            RunStoreBackend::Postgres { url_env } => build_postgres_quartet(url_env).await?,
        };

    // D-08, D-10, 39-07: the Treasurer's spend ledger, sharing this exact run store's
    // backend selection -- `None` only when `build_treasury_ledger` itself hits the
    // `Disabled` arm, which is unreachable here (handled by the early return above), so a
    // configured run store always yields `Some` or a hard startup error.
    let treasury_ledger = build_treasury_ledger(&configs.run_store).await?;

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
    // SAME default-provider resolution `FacadeProvisioner` uses. 39-07: installs
    // `treasury_ledger` under `AgentLoopSettlement::PlatformRunsOnly` when `Some`, so an
    // agent-kind run's own dispatch (never the shared engine's own superstep settlements)
    // settles through this port.
    let paladin_port = paladin_port_from_settings_with_ledger(settings, treasury_ledger.clone())
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

    // D-12: attach an operator-configured herald (`settings.herald` is `Some`) so every
    // engine run this pool dispatches hands its `RunFinished` event to it (D-11b). Absent
    // a `herald:` config section, no herald is attached and the untraced-for-cost path is
    // unchanged.
    let herald = match settings.herald {
        Some(_) => Some(
            settings
                .create_default_herald()
                .map_err(|e| format!("invalid herald configuration: {e}"))?,
        ),
        None => None,
    };

    // ALLOW-04, D-18, C6: the once-per-window notice store, opened once whenever a run store is
    // configured -- independent of whether any allowance exists (reading an empty table is
    // cheap, and it keeps the worker correct across a config change). The worker reads a
    // run's notices on its first dispatch and emits one allowance trace event per row; the
    // Treasurer below claims through the same store.
    let treasury_notices = build_treasury_notices(&configs.run_store).await?;
    // D-06, C14: one Treasurer over the run store's own ledger, built only when an allowance
    // entry exists (no entries => admission is a no-op and nothing is built). Phase 42 G3: it
    // is built BEFORE the worker pool so the pool and the submission service share the one
    // instance (the pool attaches its per-run spend guard, the service admits through it).
    let treasurer: Option<Arc<Treasurer>> = if treasurer_config.allowance.is_empty() {
        None
    } else {
        let policy = treasurer_config
            .allowance_policy()
            .map_err(|e| format!("invalid treasurer configuration: {e}"))?;
        // Enforcement must never be skipped: a configured run store always yields a ledger, so
        // `None` here is an error, not a reason to run unguarded.
        let ledger = treasury_ledger.as_ref().ok_or(
            "treasurer.allowance has entries but no treasury ledger was built from run_store.backend",
        )?;
        // ALLOW-04, D-16: the once-per-window notice store beside the ledger. A configured run
        // store always yields one, exactly like the ledger.
        let notices = treasury_notices.as_ref().ok_or(
            "treasurer.allowance has entries but no treasury notice store was built from run_store.backend",
        )?;
        // D-09 (Phase 42): the price table the agent routes' admission derives a per-run token
        // budget from. A malformed `treasurer.pricing` row fails boot exactly like the
        // allowance errors above; an empty table is fine (every ceilinged model is then
        // refused as unpriced at admission, D-10).
        let price_table = treasurer_config
            .price_table()
            .map_err(|e| format!("invalid treasurer configuration: {e}"))?;
        let mut treasurer = Treasurer::new(policy, Arc::clone(ledger))
            .with_notices(Arc::clone(notices))
            .with_pricing(Arc::new(price_table));
        // ALLOW-04, D-17: the operator webhook rides the SAME durable delivery queue the run
        // webhooks use (never a second HTTP client). The URL was SSRF-checked above; the
        // signing secret is attached to the delivery service below, never to this target.
        if let Some(webhook) = &treasurer_config.allowance.webhook {
            treasurer = treasurer.with_operator_webhook(OperatorNoticeTarget::new(
                webhook.url.clone(),
                Arc::clone(&webhook_repository),
            ));
        }
        Some(Arc::new(treasurer))
    };

    let mut pool = RunWorkerPool::new(
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
    .with_webhook_deliveries(Arc::clone(&webhook_repository))
    // OBS-02, D-11: the operator's `trace:` section reaches every per-run sink this pool
    // builds; without it the pool would run on `TraceConfig::default()` whatever was configured.
    .with_trace_config(settings.trace.clone());
    // OBS-02, D-17: the durable trace store, opened only when `trace.persist` asks for it --
    // with the flag off the port would be a no-op, so no connection is spent on it.
    if settings.trace.persist
        && let Some(traces) = build_run_trace_store(&configs.run_store).await?
    {
        pool = pool.with_run_trace_port(traces);
    }
    if let Some(herald) = herald {
        pool = pool.with_herald(herald);
    }
    if let Some(ledger) = &treasury_ledger {
        pool = pool.with_treasury_ledger(Arc::clone(ledger));
    }
    if let Some(notices) = &treasury_notices {
        pool = pool.with_treasury_notices(Arc::clone(notices));
    }
    // ALLOW-03, Phase 42 D-04, G3: the SAME Treasurer the submission service admits through
    // also reaches the pool, which attaches a per-run spend guard to every engine it builds.
    if let Some(treasurer) = &treasurer {
        pool = pool.with_treasurer(Arc::clone(treasurer));
    }
    let pool = Arc::new(pool);

    let mut tasks = Arc::clone(&pool).spawn(RunWorkerOptions::from(&configs.run_worker));

    let mut submission_service = RunSubmissionService::new(
        Arc::clone(&run_repository),
        Arc::clone(&run_queue),
        Arc::clone(&resolver),
    )
    .with_local_tokens(pool.local_tokens())
    .with_waypoints(Arc::clone(&waypoint_store))
    .with_ssrf_guard(SsrfGuard::new(configs.webhooks.allow_private));
    let treasurer: Option<Arc<dyn AllowanceAdmissionPort>> =
        treasurer.map(|treasurer| treasurer as Arc<dyn AllowanceAdmissionPort>);
    if let Some(treasurer) = &treasurer {
        submission_service = submission_service.with_treasurer(Arc::clone(treasurer));
    }
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
    .map_err(|e| format!("failed to build the webhook delivery HTTP client: {e}"))?
    // ALLOW-04, D-17, C3: the operator target's HMAC key, held on the service and signed with
    // before any run lookup -- never written to a delivery row.
    .with_operator_notice_secret(
        treasurer_config
            .allowance
            .webhook
            .as_ref()
            .and_then(|webhook| webhook.secret.clone()),
    );
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
    if let Some(ledger) = &treasury_ledger {
        run_state = run_state.with_treasury_ledger(Arc::clone(ledger));
    }

    Ok(RunApiHandles {
        run_state,
        thread_run_submission: Some(Arc::clone(&run_submission)),
        run_repository: Some(Arc::clone(&run_repository)),
        parley_extras: Some((run_repository, run_queue)),
        treasurer,
        tasks,
    })
}

/// The tenants and API key names the authentication configuration knows -- the targets a
/// `treasurer.allowance` entry may name (D-11).
///
/// Key names are the configured principal ids (never the key values); tenants come from every
/// API key's principal plus the bearer-token tenant. With authentication disabled the
/// open-access principal and tenant are valid targets (Open Question 4).
fn known_allowance_targets(auth: &AgentAuthConfig) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut tenants = BTreeSet::new();
    let mut keys = BTreeSet::new();
    for principal in auth.api_keys.values() {
        keys.insert(principal.id.clone());
        tenants.insert(principal.tenant_id.as_str().to_string());
    }
    if let Some(tenant) = &auth.bearer_tenant {
        tenants.insert(tenant.as_str().to_string());
    }
    if !auth.enabled {
        keys.insert(OPEN_ACCESS_PRINCIPAL_ID.to_string());
        tenants.insert(TenantId::OPEN_ACCESS.to_string());
    }
    (tenants, keys)
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
        assert!(
            handles.run_state.treasury_ledger.is_none(),
            "a disabled run store must wire no treasury ledger either"
        );

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
        assert!(
            handles.run_state.treasury_ledger.is_some(),
            "a configured sqlite run store must wire a treasury ledger too (D-08, 39-07)"
        );

        cleanup(&run_path);
        cleanup(&wp_path);
    }

    /// `build_run_api` wires a treasury ledger onto `RunApiState` when a run store backend
    /// is configured, and wires none when it is disabled (D-08, D-10, 39-07) -- a focused
    /// pairing of the two config shapes `defaults_wire_nothing_and_answer_501` and
    /// `sqlite_and_in_memory_wires_three_tasks_and_every_state_field` already exercise for
    /// every OTHER `RunApiState` field, now asserted specifically for `treasury_ledger`.
    #[tokio::test]
    #[serial_test::serial(paladin_run_api_wiring_openai_api_key)]
    async fn build_run_api_wires_the_treasury_ledger() {
        unsafe {
            std::env::set_var("OPENAI_API_KEY", "sk-test-run-api-wiring-hermetic");
        }

        let (run_path, run_url) = temp_sqlite_url("wires_treasury_ledger");
        let (wp_path, wp_url) = temp_sqlite_url("wires_treasury_ledger_waypoints");

        let waypoint_store: Arc<dyn WaypointPort> = Arc::new(
            paladin_storage::waypoint::sqlite::SqliteWaypointStore::new(&wp_url)
                .await
                .expect("waypoint store opens"),
        );

        let mut configs = default_configs();
        configs.run_store.backend = RunStoreBackend::Sqlite {
            path: run_url.clone(),
        };

        let handles = build_run_api(
            configs,
            &Settings::default(),
            ShutdownCoordinator::new(),
            Some(waypoint_store),
            AgentAuthConfig::default(),
            Arc::new(AgentRegistry::new()),
        )
        .await
        .expect("sqlite run store wires successfully");

        unsafe {
            std::env::remove_var("OPENAI_API_KEY");
        }

        assert!(
            handles.run_state.treasury_ledger.is_some(),
            "a configured sqlite run store must wire a treasury ledger"
        );

        cleanup(&run_path);
        cleanup(&wp_path);

        // The Disabled arm (mirrors `defaults_wire_nothing_and_answer_501`, restated here so
        // this test alone proves both halves of the D-08 contract by name).
        let disabled_handles = build_run_api(
            default_configs(),
            &Settings::default(),
            ShutdownCoordinator::new(),
            None,
            AgentAuthConfig::default(),
            Arc::new(AgentRegistry::new()),
        )
        .await
        .expect("disabled run store never fails to build");
        assert!(
            disabled_handles.run_state.treasury_ledger.is_none(),
            "a disabled run store must wire no treasury ledger"
        );
    }

    // ---- Phase 41 (41-03): allowance coherence (D-11) and Treasurer wiring (D-06, C14) ----

    fn principal_auth(keys: &[(&str, &str, &str)]) -> AgentAuthConfig {
        use paladin_core::platform::container::user::UserRole;
        let api_keys = keys
            .iter()
            .map(|(secret, name, tenant)| {
                (
                    (*secret).to_string(),
                    paladin_web::Principal::new(
                        *name,
                        UserRole::User,
                        TenantId::new(*tenant).expect("tenant id"),
                    ),
                )
            })
            .collect();
        AgentAuthConfig {
            enabled: true,
            api_keys,
            token_verifier: None,
            bearer_tenant: None,
        }
    }

    fn allowance_settings(
        api_keys: &[(&str, &str, &str)],
        tenants: &[(&str, &str, &str)],
    ) -> Settings {
        use crate::config::treasurer::AllowanceEntryConfig;
        let entry = |period: &str, amount: &str| AllowanceEntryConfig {
            period: period.to_string(),
            amount: amount.to_string(),
            lifetime: None,
            warn_at: None,
        };
        let mut settings = Settings::default();
        // An agent-kind assistant submitted under a ceiling is admitted with its model (Phase 42
        // D-10), so the default `gpt-4` agent the wiring tests register must carry a price row
        // or the submission is refused `422 model_unpriced`. Workflow assistants never read it.
        settings.treasurer.pricing.insert(
            "gpt-4".to_string(),
            crate::config::treasurer::PriceRowConfig {
                prompt: "10.00".to_string(),
                completion: "10.00".to_string(),
                cache_read: None,
                cache_write: None,
                reasoning: None,
            },
        );
        for (name, period, amount) in api_keys {
            settings
                .treasurer
                .allowance
                .api_keys
                .insert((*name).to_string(), entry(period, amount));
        }
        for (id, period, amount) in tenants {
            settings
                .treasurer
                .allowance
                .tenants
                .insert((*id).to_string(), entry(period, amount));
        }
        settings
    }

    async fn build_err(
        configs: RunApiConfigs,
        settings: &Settings,
        auth: AgentAuthConfig,
    ) -> String {
        build_run_api(
            configs,
            settings,
            ShutdownCoordinator::new(),
            None,
            auth,
            Arc::new(AgentRegistry::new()),
        )
        .await
        .err()
        .expect("an incoherent allowance configuration must stop the boot")
        .to_string()
    }

    fn sqlite_configs(url: &str) -> RunApiConfigs {
        let mut configs = default_configs();
        configs.run_store.backend = RunStoreBackend::Sqlite {
            path: url.to_string(),
        };
        configs.run_queue.backend = RunQueueBackend::InMemory;
        configs.run_worker.concurrency = 1;
        configs
    }

    async fn sqlite_waypoints(url: &str) -> Arc<dyn WaypointPort> {
        Arc::new(
            paladin_storage::waypoint::sqlite::SqliteWaypointStore::new(url)
                .await
                .expect("waypoint store opens"),
        )
    }

    #[tokio::test]
    async fn build_run_api_refuses_allowances_without_a_run_store() {
        let settings = allowance_settings(&[("svc-a", "1d", "2.50")], &[]);
        let err = build_err(
            default_configs(),
            &settings,
            principal_auth(&[("secret-a", "svc-a", "acme")]),
        )
        .await;
        assert!(err.contains("treasurer.allowance"), "{err}");
        assert!(err.contains("run_store.backend"), "{err}");
        assert!(
            !err.contains("secret-a"),
            "a key value must never be echoed: {err}"
        );
    }

    #[tokio::test]
    async fn build_run_api_with_a_disabled_store_and_no_allowance_is_unchanged() {
        let handles = build_run_api(
            default_configs(),
            &Settings::default(),
            ShutdownCoordinator::new(),
            None,
            principal_auth(&[("secret-a", "svc-a", "acme")]),
            Arc::new(AgentRegistry::new()),
        )
        .await
        .expect("disabled store, no allowance: boots as before");
        assert!(handles.treasurer.is_none());
        assert!(handles.tasks.is_empty());
    }

    #[tokio::test]
    async fn build_run_api_rejects_an_allowance_for_an_unknown_api_key() {
        let settings = allowance_settings(&[("ghost", "1d", "2.50")], &[]);
        let err = build_err(
            sqlite_configs("sqlite::memory:"),
            &settings,
            principal_auth(&[("secret-a", "svc-a", "acme")]),
        )
        .await;
        assert!(err.contains("treasurer.allowance.api_keys.ghost"), "{err}");
        assert!(err.contains("http.auth.api_keys"), "{err}");
    }

    #[tokio::test]
    async fn build_run_api_rejects_an_allowance_for_an_unknown_tenant() {
        let settings = allowance_settings(&[], &[("globex", "1d", "25.00")]);
        let err = build_err(
            sqlite_configs("sqlite::memory:"),
            &settings,
            principal_auth(&[("secret-a", "svc-a", "acme")]),
        )
        .await;
        assert!(err.contains("treasurer.allowance.tenants.globex"), "{err}");
    }

    #[test]
    fn open_access_targets_are_known_only_when_auth_is_disabled() {
        let (tenants, keys) = known_allowance_targets(&AgentAuthConfig::default());
        assert!(keys.contains(OPEN_ACCESS_PRINCIPAL_ID));
        assert!(tenants.contains(TenantId::OPEN_ACCESS));
        let (tenants, keys) = known_allowance_targets(&principal_auth(&[("s", "svc-a", "acme")]));
        assert!(!keys.contains(OPEN_ACCESS_PRINCIPAL_ID));
        assert!(!tenants.contains(TenantId::OPEN_ACCESS));
        assert!(keys.contains("svc-a") && tenants.contains("acme"));
        assert!(
            !keys.contains("s"),
            "the key VALUE is never a known allowance target"
        );
    }

    #[tokio::test]
    #[serial_test::serial(paladin_run_api_wiring_openai_api_key)]
    async fn build_run_api_accepts_open_access_targets_when_auth_is_disabled() {
        unsafe {
            std::env::set_var("OPENAI_API_KEY", "sk-test-run-api-wiring-hermetic");
        }
        let (run_path, run_url) = temp_sqlite_url("open_access_allowance");
        let (wp_path, wp_url) = temp_sqlite_url("open_access_allowance_wp");
        let settings = allowance_settings(
            &[("anonymous", "1d", "2.50")],
            &[("open-access", "1d", "25.00")],
        );
        let coordinator = ShutdownCoordinator::new();
        let handles = build_run_api(
            sqlite_configs(&run_url),
            &settings,
            coordinator.clone(),
            Some(sqlite_waypoints(&wp_url).await),
            AgentAuthConfig::default(),
            Arc::new(AgentRegistry::new()),
        )
        .await
        .expect("open-access allowance targets are accepted with auth disabled");
        unsafe {
            std::env::remove_var("OPENAI_API_KEY");
        }
        assert!(handles.treasurer.is_some());
        coordinator.cancel_and_wait(Duration::from_secs(1)).await;
        cleanup(&run_path);
        cleanup(&wp_path);
    }

    #[tokio::test]
    #[serial_test::serial(paladin_run_api_wiring_openai_api_key)]
    async fn build_run_api_wires_a_treasurer_only_when_allowances_are_configured() {
        unsafe {
            std::env::set_var("OPENAI_API_KEY", "sk-test-run-api-wiring-hermetic");
        }
        let auth = || principal_auth(&[("secret-a", "svc-a", "acme")]);

        let (run_path, run_url) = temp_sqlite_url("no_allowance");
        let (wp_path, wp_url) = temp_sqlite_url("no_allowance_wp");
        let coordinator = ShutdownCoordinator::new();
        let without = build_run_api(
            sqlite_configs(&run_url),
            &Settings::default(),
            coordinator.clone(),
            Some(sqlite_waypoints(&wp_url).await),
            auth(),
            Arc::new(AgentRegistry::new()),
        )
        .await
        .expect("sqlite run store without allowances wires");
        assert!(without.treasurer.is_none(), "no entries: no Treasurer");
        coordinator.cancel_and_wait(Duration::from_secs(1)).await;
        cleanup(&run_path);
        cleanup(&wp_path);

        let (run_path, run_url) = temp_sqlite_url("with_allowance");
        let (wp_path, wp_url) = temp_sqlite_url("with_allowance_wp");
        let coordinator = ShutdownCoordinator::new();
        let with = build_run_api(
            sqlite_configs(&run_url),
            &allowance_settings(&[("svc-a", "1d", "2.50")], &[("acme", "7d", "25.00")]),
            coordinator.clone(),
            Some(sqlite_waypoints(&wp_url).await),
            auth(),
            Arc::new(AgentRegistry::new()),
        )
        .await
        .expect("sqlite run store with allowances wires");
        assert!(with.treasurer.is_some(), "entries present: one Treasurer");
        coordinator.cancel_and_wait(Duration::from_secs(1)).await;
        cleanup(&run_path);
        cleanup(&wp_path);

        unsafe {
            std::env::remove_var("OPENAI_API_KEY");
        }
    }

    /// D-09 (Phase 42): the Treasurer the agent state admits through carries the operator's
    /// `treasurer.pricing` table, so `admit_for_model` derives a budget for a priced model
    /// and refuses an unpriced one (D-10) instead of silently admitting it.
    #[tokio::test]
    #[serial_test::serial(paladin_run_api_wiring_openai_api_key)]
    async fn build_run_api_prices_the_treasurer() {
        use paladin_core::platform::container::principal::{RunAttribution, TenantId};
        use paladin_ports::input::allowance_admission_port::AdmissionError;

        unsafe {
            std::env::set_var("OPENAI_API_KEY", "sk-test-run-api-wiring-hermetic");
        }
        let (run_path, run_url) = temp_sqlite_url("prices_treasurer");
        let (wp_path, wp_url) = temp_sqlite_url("prices_treasurer_wp");
        let mut settings = allowance_settings(&[("svc-a", "1d", "2.50")], &[]);
        settings.treasurer.pricing.insert(
            "gpt-4".to_string(),
            crate::config::PriceRowConfig {
                prompt: "10.00".to_string(),
                completion: "30.00".to_string(),
                cache_read: None,
                cache_write: None,
                reasoning: None,
            },
        );
        let coordinator = ShutdownCoordinator::new();
        let handles = build_run_api(
            sqlite_configs(&run_url),
            &settings,
            coordinator.clone(),
            Some(sqlite_waypoints(&wp_url).await),
            principal_auth(&[("key-a", "svc-a", "acme")]),
            Arc::new(AgentRegistry::new()),
        )
        .await
        .expect("sqlite run store with an allowance and a price table wires");
        let treasurer = handles.treasurer.expect("an allowance builds a Treasurer");
        let subject = RunAttribution::new(TenantId::new("acme").expect("tenant"), "svc-a");

        let admission = treasurer
            .admit_for_model(&subject, None, "gpt-4")
            .await
            .expect("a priced model under a fresh allowance is admitted");
        let derived = admission
            .derived_budget()
            .expect("a priced model derives a token budget");
        assert!(
            derived.max_tokens > 0,
            "a fresh $2.50 allowance buys tokens"
        );

        let err = treasurer
            .admit_for_model(&subject, None, "unpriced-model")
            .await
            .expect_err("an unpriced model under a ceiling is refused");
        assert!(
            matches!(&err, AdmissionError::ModelUnpriced { model } if model == "unpriced-model"),
            "got {err:?}"
        );

        coordinator.cancel_and_wait(Duration::from_secs(1)).await;
        unsafe {
            std::env::remove_var("OPENAI_API_KEY");
        }
        cleanup(&run_path);
        cleanup(&wp_path);
    }

    /// D-09: an invalid `treasurer.pricing` row aborts `build_run_api`, naming the offending
    /// config path. The run engine's LLM port is built (and its price table checked) before the
    /// Treasurer, so that earlier existing error is the one reported; the Treasurer's own
    /// `price_table()` mapping is the same defensive error for any ordering change.
    #[tokio::test]
    #[serial_test::serial(paladin_run_api_wiring_openai_api_key)]
    async fn build_run_api_rejects_an_invalid_price_row() {
        unsafe {
            std::env::set_var("OPENAI_API_KEY", "sk-test-run-api-wiring-hermetic");
        }
        let (run_path, run_url) = temp_sqlite_url("bad_price");
        let (wp_path, wp_url) = temp_sqlite_url("bad_price_wp");
        let mut settings = allowance_settings(&[("svc-a", "1d", "2.50")], &[]);
        settings.treasurer.pricing.insert(
            "gpt-4".to_string(),
            crate::config::PriceRowConfig {
                prompt: "-1".to_string(),
                completion: "30.00".to_string(),
                cache_read: None,
                cache_write: None,
                reasoning: None,
            },
        );
        let err = match build_run_api(
            sqlite_configs(&run_url),
            &settings,
            ShutdownCoordinator::new(),
            Some(sqlite_waypoints(&wp_url).await),
            principal_auth(&[("key-a", "svc-a", "acme")]),
            Arc::new(AgentRegistry::new()),
        )
        .await
        {
            Ok(_) => panic!("an invalid price row must abort build_run_api"),
            Err(err) => err.to_string(),
        };
        unsafe {
            std::env::remove_var("OPENAI_API_KEY");
        }
        cleanup(&wp_path);
        assert!(err.contains("treasurer.pricing.gpt-4.prompt"), "{err}");
        cleanup(&run_path);
    }

    /// The production wiring, end to end: an exhausted key is refused through the real run
    /// router, and a key with no entry still submits. A UTC window rollover between seeding
    /// and the POST (Pitfall 10) re-runs the scenario once against the new window.
    #[tokio::test(flavor = "multi_thread")]
    #[serial_test::serial(paladin_run_api_wiring_openai_api_key)]
    async fn wired_treasurer_refuses_an_exhausted_key_through_the_run_router() {
        use crate::application::services::treasurer::window_for;
        use paladin_core::platform::container::cost::{Cost, CurrencyCode};
        use paladin_core::platform::container::paladin::PaladinData;
        use paladin_core::platform::container::run::RunId;
        use paladin_core::platform::container::treasury_ledger::{
            LedgerScope, SettleRequest, SettlementKey,
        };

        struct NoopExecutor;
        #[async_trait]
        impl paladin_ports::output::paladin_executor_port::PaladinExecutorPort for NoopExecutor {
            async fn execute(
                &self,
                _paladin: &paladin_core::platform::container::paladin::Paladin,
                _input: &str,
            ) -> Result<
                paladin_ports::output::paladin_port::PaladinResult,
                paladin_core::platform::container::paladin_error::PaladinError,
            > {
                Err(
                    paladin_core::platform::container::paladin_error::PaladinError::ExecutionError(
                        "the allowance wiring test never runs the agent".to_string(),
                    ),
                )
            }
        }

        unsafe {
            std::env::set_var("OPENAI_API_KEY", "sk-test-run-api-wiring-hermetic");
        }
        let (run_path, run_url) = temp_sqlite_url("wired_treasurer");
        let (wp_path, wp_url) = temp_sqlite_url("wired_treasurer_wp");

        let registry = Arc::new(AgentRegistry::new());
        let paladin = Arc::new(paladin_core::base::entity::node::Node::new(
            PaladinData {
                system_prompt: "hi".to_string(),
                name: "AllowanceAgent".to_string(),
                ..Default::default()
            },
            Some("AllowanceAgent".to_string()),
        ));
        registry.insert("allowance-agent", paladin, Arc::new(NoopExecutor));

        let configs = sqlite_configs(&run_url);
        let ledger = build_treasury_ledger(&configs.run_store)
            .await
            .expect("ledger opens")
            .expect("a sqlite run store yields a ledger");
        let coordinator = ShutdownCoordinator::new();
        let handles = build_run_api(
            configs,
            // 1d per RESEARCH Pitfall 10: a short window would roll over mid-test.
            &allowance_settings(&[("svc-a", "1d", "2.50")], &[]),
            coordinator.clone(),
            Some(sqlite_waypoints(&wp_url).await),
            principal_auth(&[("key-a", "svc-a", "acme"), ("key-b", "svc-b", "acme")]),
            registry,
        )
        .await
        .expect("sqlite run store with an allowance wires");
        assert!(handles.treasurer.is_some());
        let app = paladin_web::run_router(handles.run_state);

        let post = |key: &'static str| {
            let app = app.clone();
            async move {
                app.oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/v1/runs")
                        .header("content-type", "application/json")
                        .header("x-api-key", key)
                        .body(Body::from(
                            r#"{"assistant_id":"allowance-agent","input":{}}"#,
                        ))
                        .expect("request builds"),
                )
                .await
                .expect("router responds")
            }
        };

        let mut settled = false;
        for _ in 0..2 {
            let before =
                window_for(ledger.store_now().await.expect("store clock"), 86_400).expect("window");
            let usd = CurrencyCode::new("USD").expect("usd");
            ledger
                .settle(SettleRequest::unreserved(
                    LedgerScope::new("acme", "svc-a"),
                    SettlementKey::new(RunId::new_v7(), 0, 0),
                    Cost::new(2_500_000_000, usd),
                    std::collections::BTreeMap::from([("gpt-4".to_string(), 2_500_000_000_i64)]),
                ))
                .await
                .expect("seed the key at its ceiling");
            let refused = post("key-a").await;
            let after =
                window_for(ledger.store_now().await.expect("store clock"), 86_400).expect("window");
            if before != after {
                continue;
            }
            assert_eq!(refused.status(), StatusCode::TOO_MANY_REQUESTS);
            let bytes = axum::body::to_bytes(refused.into_body(), usize::MAX)
                .await
                .expect("read body");
            let body: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON body");
            assert_eq!(body["error"]["code"], "allowance_exhausted");

            // A key with no entry (and a tenant with none) is admitted (D-03).
            let admitted = post("key-b").await;
            assert_eq!(admitted.status(), StatusCode::ACCEPTED);
            settled = true;
            break;
        }
        assert!(
            settled,
            "a UTC window boundary was crossed on both attempts"
        );

        coordinator.cancel_and_wait(Duration::from_secs(1)).await;
        unsafe {
            std::env::remove_var("OPENAI_API_KEY");
        }
        cleanup(&run_path);
        cleanup(&wp_path);
    }

    /// The production wiring records a warn notice: a key at 80% of its allowance is admitted
    /// through the real run router and the durable once-per-window notice names the persisted
    /// run, read back through a store opened by `build_treasury_notices` on the same file. A UTC
    /// window rollover between seeding and the POST (Pitfall 10) re-runs the scenario once.
    #[tokio::test(flavor = "multi_thread")]
    #[serial_test::serial(paladin_run_api_wiring_openai_api_key)]
    async fn wired_treasurer_records_a_warn_notice_for_the_admitted_run() {
        use crate::application::services::treasurer::window_for;
        use paladin_core::platform::container::cost::{Cost, CurrencyCode};
        use paladin_core::platform::container::paladin::PaladinData;
        use paladin_core::platform::container::run::RunId;
        use paladin_core::platform::container::treasury_ledger::{
            LedgerScope, SettleRequest, SettlementKey,
        };

        struct NoopExecutor;
        #[async_trait]
        impl paladin_ports::output::paladin_executor_port::PaladinExecutorPort for NoopExecutor {
            async fn execute(
                &self,
                _paladin: &paladin_core::platform::container::paladin::Paladin,
                _input: &str,
            ) -> Result<
                paladin_ports::output::paladin_port::PaladinResult,
                paladin_core::platform::container::paladin_error::PaladinError,
            > {
                Err(
                    paladin_core::platform::container::paladin_error::PaladinError::ExecutionError(
                        "the notice wiring test never runs the agent".to_string(),
                    ),
                )
            }
        }

        unsafe {
            std::env::set_var("OPENAI_API_KEY", "sk-test-run-api-wiring-hermetic");
        }
        let (run_path, run_url) = temp_sqlite_url("wired_notice");
        let (wp_path, wp_url) = temp_sqlite_url("wired_notice_wp");

        let registry = Arc::new(AgentRegistry::new());
        let paladin = Arc::new(paladin_core::base::entity::node::Node::new(
            PaladinData {
                system_prompt: "hi".to_string(),
                name: "NoticeAgent".to_string(),
                ..Default::default()
            },
            Some("NoticeAgent".to_string()),
        ));
        registry.insert("notice-agent", paladin, Arc::new(NoopExecutor));

        let configs = sqlite_configs(&run_url);
        let ledger = build_treasury_ledger(&configs.run_store)
            .await
            .expect("ledger opens")
            .expect("a sqlite run store yields a ledger");
        let notices = build_treasury_notices(&configs.run_store)
            .await
            .expect("notice store opens")
            .expect("a sqlite run store yields a notice store");
        let coordinator = ShutdownCoordinator::new();
        let handles = build_run_api(
            configs,
            &allowance_settings(&[("svc-a", "1d", "2.50")], &[]),
            coordinator.clone(),
            Some(sqlite_waypoints(&wp_url).await),
            principal_auth(&[("key-a", "svc-a", "acme")]),
            registry,
        )
        .await
        .expect("sqlite run store with an allowance wires");
        let app = paladin_web::run_router(handles.run_state);

        let mut recorded = false;
        for _ in 0..2 {
            let before =
                window_for(ledger.store_now().await.expect("store clock"), 86_400).expect("window");
            let usd = CurrencyCode::new("USD").expect("usd");
            ledger
                .settle(SettleRequest::unreserved(
                    LedgerScope::new("acme", "svc-a"),
                    SettlementKey::new(RunId::new_v7(), 0, 0),
                    // 2.00 of a 2.50 window: exactly the default 80% warn threshold.
                    Cost::new(2_000_000_000, usd),
                    std::collections::BTreeMap::from([("gpt-4".to_string(), 2_000_000_000_i64)]),
                ))
                .await
                .expect("seed the key at its warn threshold");
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/v1/runs")
                        .header("content-type", "application/json")
                        .header("x-api-key", "key-a")
                        .body(Body::from(r#"{"assistant_id":"notice-agent","input":{}}"#))
                        .expect("request builds"),
                )
                .await
                .expect("router responds");
            let after =
                window_for(ledger.store_now().await.expect("store clock"), 86_400).expect("window");
            if before != after {
                continue;
            }
            assert_eq!(response.status(), StatusCode::ACCEPTED);
            let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("read body");
            let body: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON body");
            let run_id = RunId::parse(body["run_id"].as_str().expect("run_id in the body"))
                .expect("a valid run id");

            let rows = notices.notices_for_run(&run_id).await.expect("read back");
            assert_eq!(
                rows.len(),
                1,
                "the crossing's notice names the admitted run"
            );
            assert_eq!(rows[0].tenant_id, "acme");
            assert_eq!(rows[0].api_key_id.as_deref(), Some("svc-a"));
            assert_eq!(rows[0].warning.balance.nanos(), 2_000_000_000);
            assert_eq!(rows[0].warning.ceiling.nanos(), 2_500_000_000);
            recorded = true;
            break;
        }
        assert!(
            recorded,
            "a UTC window boundary was crossed on both attempts"
        );

        coordinator.cancel_and_wait(Duration::from_secs(1)).await;
        unsafe {
            std::env::remove_var("OPENAI_API_KEY");
        }
        cleanup(&run_path);
        cleanup(&wp_path);
    }

    // ---- Phase 41 (41-08): the operator webhook leg (D-17, T-41-38, C3) ----

    /// `allowance_settings` for `svc-a` plus an operator webhook target.
    fn operator_webhook_settings(url: &str, secret: Option<&str>) -> Settings {
        use crate::config::treasurer::AllowanceWebhookConfig;
        let mut settings = allowance_settings(&[("svc-a", "1d", "2.50")], &[]);
        settings.treasurer.allowance.webhook = Some(AllowanceWebhookConfig {
            url: url.to_string(),
            secret: secret.map(str::to_string),
        });
        settings
    }

    /// Boot `build_run_api` over fresh SQLite files, returning the result and the temp paths.
    async fn build_with_webhook_settings(
        label: &str,
        settings: &Settings,
        allow_private: bool,
    ) -> (
        Result<RunApiHandles, Box<dyn std::error::Error>>,
        ShutdownCoordinator,
        std::path::PathBuf,
        std::path::PathBuf,
    ) {
        let (run_path, run_url) = temp_sqlite_url(label);
        let (wp_path, wp_url) = temp_sqlite_url(&format!("{label}_wp"));
        let mut configs = sqlite_configs(&run_url);
        configs.webhooks.allow_private = allow_private;
        let coordinator = ShutdownCoordinator::new();
        let result = build_run_api(
            configs,
            settings,
            coordinator.clone(),
            Some(sqlite_waypoints(&wp_url).await),
            principal_auth(&[("key-a", "svc-a", "acme")]),
            Arc::new(AgentRegistry::new()),
        )
        .await;
        (result, coordinator, run_path, wp_path)
    }

    #[tokio::test]
    #[serial_test::serial(paladin_run_api_wiring_openai_api_key)]
    async fn build_run_api_rejects_a_private_operator_webhook_without_allow_private() {
        unsafe {
            std::env::set_var("OPENAI_API_KEY", "sk-test-run-api-wiring-hermetic");
        }
        let settings = operator_webhook_settings("http://127.0.0.1:9/hook", Some("op-secret-xyz"));
        let (result, coordinator, run_path, wp_path) =
            build_with_webhook_settings("op_hook_private", &settings, false).await;
        let message = result
            .err()
            .expect("a loopback operator webhook must stop the boot")
            .to_string();
        assert!(
            message.contains("treasurer.allowance.webhook.url"),
            "names the config key: {message}"
        );
        assert!(
            message.contains("webhooks.allow_private"),
            "points at the escape hatch: {message}"
        );
        assert!(
            !message.contains("op-secret-xyz"),
            "never echoes the secret: {message}"
        );
        coordinator.cancel_and_wait(Duration::from_secs(1)).await;
        unsafe {
            std::env::remove_var("OPENAI_API_KEY");
        }
        cleanup(&run_path);
        cleanup(&wp_path);
    }

    #[tokio::test]
    #[serial_test::serial(paladin_run_api_wiring_openai_api_key)]
    async fn build_run_api_accepts_a_private_operator_webhook_with_allow_private() {
        unsafe {
            std::env::set_var("OPENAI_API_KEY", "sk-test-run-api-wiring-hermetic");
        }
        let settings = operator_webhook_settings("http://127.0.0.1:9/hook", Some("op-secret-xyz"));
        let (result, coordinator, run_path, wp_path) =
            build_with_webhook_settings("op_hook_allowed", &settings, true).await;
        let handles = result.expect("allow_private admits a loopback operator webhook");
        assert!(handles.treasurer.is_some());
        coordinator.cancel_and_wait(Duration::from_secs(1)).await;
        unsafe {
            std::env::remove_var("OPENAI_API_KEY");
        }
        cleanup(&run_path);
        cleanup(&wp_path);
    }

    #[tokio::test]
    #[serial_test::serial(paladin_run_api_wiring_openai_api_key)]
    async fn build_run_api_rejects_the_metadata_address_even_with_allow_private() {
        unsafe {
            std::env::set_var("OPENAI_API_KEY", "sk-test-run-api-wiring-hermetic");
        }
        let settings = operator_webhook_settings("http://169.254.169.254/latest", None);
        let (result, coordinator, run_path, wp_path) =
            build_with_webhook_settings("op_hook_metadata", &settings, true).await;
        let message = result
            .err()
            .expect("the cloud metadata address is always rejected")
            .to_string();
        assert!(
            message.contains("treasurer.allowance.webhook.url"),
            "{message}"
        );
        coordinator.cancel_and_wait(Duration::from_secs(1)).await;
        unsafe {
            std::env::remove_var("OPENAI_API_KEY");
        }
        cleanup(&run_path);
        cleanup(&wp_path);
    }

    #[tokio::test]
    #[serial_test::serial(paladin_run_api_wiring_openai_api_key)]
    async fn build_run_api_without_an_operator_webhook_builds_no_target() {
        unsafe {
            std::env::set_var("OPENAI_API_KEY", "sk-test-run-api-wiring-hermetic");
        }
        // An allowance and no webhook: the Treasurer is built without a target and the delivery
        // drain loop still spawns (run webhooks need it). A private default needs no
        // `allow_private` because nothing is checked.
        let settings = allowance_settings(&[("svc-a", "1d", "2.50")], &[]);
        let (result, coordinator, run_path, wp_path) =
            build_with_webhook_settings("op_hook_none", &settings, false).await;
        let handles = result.expect("an allowance without a webhook wires");
        assert!(handles.treasurer.is_some());
        assert!(
            handles.tasks.len() >= 2,
            "the worker pool and the webhook delivery loop both spawn"
        );
        coordinator.cancel_and_wait(Duration::from_secs(1)).await;
        unsafe {
            std::env::remove_var("OPENAI_API_KEY");
        }
        cleanup(&run_path);
        cleanup(&wp_path);
    }

    /// The production wiring delivers one signed operator notice: a key at exactly its 80% warn
    /// threshold is admitted through the real run router, exactly one notice row names the
    /// admitted run, and the spawned drain loop (with the Treasurer's operator target and the
    /// delivery service's operator secret both wired by `build_run_api`) delivers exactly one
    /// `allowance_warning` POST whose HMAC verifies over the captured raw bytes. A UTC window
    /// rollover between seeding and the POST (Pitfall 10) re-runs the scenario once.
    #[tokio::test(flavor = "multi_thread")]
    #[serial_test::serial(paladin_run_api_wiring_openai_api_key)]
    async fn build_run_api_wires_the_allowance_warn_path() {
        use crate::application::services::treasurer::window_for;
        use hmac::{Hmac, Mac};
        use paladin_core::platform::container::cost::{Cost, CurrencyCode};
        use paladin_core::platform::container::paladin::PaladinData;
        use paladin_core::platform::container::run::RunId;
        use paladin_core::platform::container::treasury_ledger::{
            LedgerScope, SettleRequest, SettlementKey,
        };
        use sha2::Sha256;
        use std::sync::Mutex;

        struct NoopExecutor;
        #[async_trait]
        impl paladin_ports::output::paladin_executor_port::PaladinExecutorPort for NoopExecutor {
            async fn execute(
                &self,
                _paladin: &paladin_core::platform::container::paladin::Paladin,
                _input: &str,
            ) -> Result<
                paladin_ports::output::paladin_port::PaladinResult,
                paladin_core::platform::container::paladin_error::PaladinError,
            > {
                Err(
                    paladin_core::platform::container::paladin_error::PaladinError::ExecutionError(
                        "the operator webhook wiring test never runs the agent".to_string(),
                    ),
                )
            }
        }

        // (raw body, signature header, event header) of every POST the receiver saw.
        type Captured = Arc<Mutex<Vec<(Vec<u8>, String, String)>>>;
        let mut receiver = mockito::Server::new_async().await;
        let captured: Captured = Arc::new(Mutex::new(Vec::new()));
        {
            let sink = Arc::clone(&captured);
            receiver
                .mock("POST", "/hook")
                .with_status_code_from_request(move |req| {
                    let header = |name: &str| {
                        req.header(name)
                            .first()
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or_default()
                            .to_string()
                    };
                    sink.lock().expect("not poisoned").push((
                        req.body().cloned().unwrap_or_default(),
                        header("x-paladin-signature"),
                        header("x-paladin-event"),
                    ));
                    200
                })
                .expect_at_least(0)
                .create_async()
                .await;
        }

        unsafe {
            std::env::set_var("OPENAI_API_KEY", "sk-test-run-api-wiring-hermetic");
        }
        let (run_path, run_url) = temp_sqlite_url("wired_operator");
        let (wp_path, wp_url) = temp_sqlite_url("wired_operator_wp");

        let registry = Arc::new(AgentRegistry::new());
        let paladin = Arc::new(paladin_core::base::entity::node::Node::new(
            PaladinData {
                system_prompt: "hi".to_string(),
                name: "OperatorAgent".to_string(),
                ..Default::default()
            },
            Some("OperatorAgent".to_string()),
        ));
        registry.insert("operator-agent", paladin, Arc::new(NoopExecutor));

        let mut configs = sqlite_configs(&run_url);
        configs.webhooks.allow_private = true;
        let ledger = build_treasury_ledger(&configs.run_store)
            .await
            .expect("ledger opens")
            .expect("a sqlite run store yields a ledger");
        let notices = build_treasury_notices(&configs.run_store)
            .await
            .expect("notice store opens")
            .expect("a sqlite run store yields a notice store");
        let mut settings = allowance_settings(&[("svc-w", "1d", "1.00")], &[]);
        settings.treasurer.allowance.webhook =
            Some(crate::config::treasurer::AllowanceWebhookConfig {
                url: format!("{}/hook", receiver.url()),
                secret: Some("op-secret".to_string()),
            });
        let coordinator = ShutdownCoordinator::new();
        let handles = build_run_api(
            configs,
            &settings,
            coordinator.clone(),
            Some(sqlite_waypoints(&wp_url).await),
            principal_auth(&[("key-w", "svc-w", "acme")]),
            registry,
        )
        .await
        .expect("sqlite run store with an allowance and an operator webhook wires");
        let app = paladin_web::run_router(handles.run_state);

        let mut delivered = false;
        for _ in 0..2 {
            let before =
                window_for(ledger.store_now().await.expect("store clock"), 86_400).expect("window");
            let usd = CurrencyCode::new("USD").expect("usd");
            ledger
                .settle(SettleRequest::unreserved(
                    LedgerScope::new("acme", "svc-w"),
                    SettlementKey::new(RunId::new_v7(), 0, 0),
                    // 0.80 of a 1.00 window: exactly the default 80% warn threshold.
                    Cost::new(800_000_000, usd),
                    std::collections::BTreeMap::from([("gpt-4".to_string(), 800_000_000_i64)]),
                ))
                .await
                .expect("seed the key at its warn threshold");
            captured.lock().expect("not poisoned").clear();
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/v1/runs")
                        .header("content-type", "application/json")
                        .header("x-api-key", "key-w")
                        .body(Body::from(
                            r#"{"assistant_id":"operator-agent","input":{}}"#,
                        ))
                        .expect("request builds"),
                )
                .await
                .expect("router responds");
            let after =
                window_for(ledger.store_now().await.expect("store clock"), 86_400).expect("window");
            if before != after {
                continue;
            }
            assert_eq!(response.status(), StatusCode::ACCEPTED);
            let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("read body");
            let body: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON body");
            let run_id = RunId::parse(body["run_id"].as_str().expect("run_id in the body"))
                .expect("a valid run id");

            let rows = notices.notices_for_run(&run_id).await.expect("read back");
            assert_eq!(rows.len(), 1, "exactly one notice row for the admitted run");

            // The spawned drain loop delivers it: poll every 100 ms for at most 10 s.
            let deadline = std::time::Instant::now() + Duration::from_secs(10);
            while captured.lock().expect("not poisoned").is_empty()
                && std::time::Instant::now() < deadline
            {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            // Let a (wrong) duplicate delivery surface before asserting exactly one.
            tokio::time::sleep(Duration::from_millis(400)).await;
            let seen = captured.lock().expect("not poisoned").clone();
            assert_eq!(seen.len(), 1, "exactly one operator POST");
            let (raw, signature, event) = &seen[0];
            assert_eq!(event, "allowance_warning");
            let mut mac =
                <Hmac<Sha256> as Mac>::new_from_slice(b"op-secret").expect("any key length");
            mac.update(raw);
            let expected = format!(
                "sha256={}",
                mac.finalize()
                    .into_bytes()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            );
            assert_eq!(signature, &expected, "signed with the operator secret");
            let payload: serde_json::Value = serde_json::from_slice(raw).expect("JSON payload");
            assert_eq!(payload["run_id"], serde_json::json!(run_id));
            assert_eq!(payload["tenant_id"], "acme");
            assert_eq!(payload["api_key_id"], "svc-w");
            delivered = true;
            break;
        }
        assert!(
            delivered,
            "a UTC window boundary was crossed on both attempts"
        );

        coordinator.cancel_and_wait(Duration::from_secs(1)).await;
        unsafe {
            std::env::remove_var("OPENAI_API_KEY");
        }
        cleanup(&run_path);
        cleanup(&wp_path);
    }

    /// Submits one agent-kind run through the real run router of a `build_run_api` wired with
    /// `trace`, waits for it to end, and returns how many records `run_traces` holds for its
    /// thread -- counted straight from the SQLite file, exactly as an operator's `sqlite3`
    /// would.
    async fn persisted_trace_records(label: &str, trace: crate::config::trace::TraceConfig) -> i64 {
        use paladin_core::platform::container::paladin::PaladinData;
        use paladin_core::platform::container::run::RunId;

        struct FailingExecutor;
        #[async_trait]
        impl paladin_ports::output::paladin_executor_port::PaladinExecutorPort for FailingExecutor {
            async fn execute(
                &self,
                _paladin: &paladin_core::platform::container::paladin::Paladin,
                _input: &str,
            ) -> Result<
                paladin_ports::output::paladin_port::PaladinResult,
                paladin_core::platform::container::paladin_error::PaladinError,
            > {
                Err(
                    paladin_core::platform::container::paladin_error::PaladinError::ExecutionError(
                        "the trace wiring test needs no model answer".to_string(),
                    ),
                )
            }
        }

        unsafe {
            std::env::set_var("OPENAI_API_KEY", "sk-test-run-api-wiring-hermetic");
        }
        let (run_path, run_url) = temp_sqlite_url(label);
        let (wp_path, wp_url) = temp_sqlite_url(&format!("{label}_wp"));

        let registry = Arc::new(AgentRegistry::new());
        let paladin = Arc::new(paladin_core::base::entity::node::Node::new(
            PaladinData {
                system_prompt: "hi".to_string(),
                name: "TracedAgent".to_string(),
                ..Default::default()
            },
            Some("TracedAgent".to_string()),
        ));
        registry.insert("traced-agent", paladin, Arc::new(FailingExecutor));

        let settings = Settings {
            trace,
            ..Settings::default()
        };
        let coordinator = ShutdownCoordinator::new();
        let handles = build_run_api(
            sqlite_configs(&run_url),
            &settings,
            coordinator.clone(),
            Some(sqlite_waypoints(&wp_url).await),
            AgentAuthConfig::default(),
            registry,
        )
        .await
        .expect("sqlite run store wires");
        let repository = handles.run_repository.clone().expect("a run repository");
        let app = paladin_web::run_router(handles.run_state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/runs")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"assistant_id":"traced-agent","input":{}}"#))
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let body: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON body");
        let run_id =
            RunId::parse(body["run_id"].as_str().expect("run_id in the body")).expect("run id");
        let thread_id = ThreadId::new(body["thread_id"].as_str().expect("thread_id in the body"))
            .expect("thread id");

        // The spawned worker picks the run up: poll every 100 ms for at most 10 s.
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        loop {
            let run = repository.get(&run_id).await.expect("read the run");
            if run.is_some_and(|run| run.status.is_terminal()) {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "the run never reached a terminal status"
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        // Stopping the pool lets every per-run trace dispatcher drain before the read-back.
        coordinator.cancel_and_wait(Duration::from_secs(1)).await;
        unsafe {
            std::env::remove_var("OPENAI_API_KEY");
        }

        let pool = sqlx::SqlitePool::connect(&run_url)
            .await
            .expect("run store file opens");
        let records: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM run_traces WHERE thread_id = ?")
                .bind(thread_id.as_str())
                .fetch_one(&pool)
                .await
                .expect("count the persisted trace rows");
        pool.close().await;
        cleanup(&run_path);
        cleanup(&wp_path);
        records
    }

    /// `trace.persist: true` in the operator's config reaches the production worker pool: a run
    /// dispatched by a `build_run_api`-built server leaves its trace records in `run_traces`.
    #[tokio::test(flavor = "multi_thread")]
    #[serial_test::serial(paladin_run_api_wiring_openai_api_key)]
    async fn build_run_api_persists_run_traces_when_trace_persist_is_set() {
        let records = persisted_trace_records(
            "trace_persist_on",
            crate::config::trace::TraceConfig {
                persist: true,
                ..Default::default()
            },
        )
        .await;
        assert!(
            records > 0,
            "trace.persist: true must leave the run's trace records in run_traces"
        );
    }

    /// The default (`trace.persist: false`) stays off: the same run persists no trace record.
    #[tokio::test(flavor = "multi_thread")]
    #[serial_test::serial(paladin_run_api_wiring_openai_api_key)]
    async fn build_run_api_persists_no_run_traces_by_default() {
        let records = persisted_trace_records(
            "trace_persist_off",
            crate::config::trace::TraceConfig::default(),
        )
        .await;
        assert_eq!(records, 0, "trace.persist defaults to off");
    }

    #[tokio::test]
    async fn build_treasury_notices_disabled_is_none() {
        let notices = build_treasury_notices(&RunStoreConfig::default())
            .await
            .expect("a disabled run store never fails to build");
        assert!(
            notices.is_none(),
            "RunStoreBackend::Disabled must wire no notice store"
        );
    }

    #[tokio::test]
    async fn build_treasury_notices_sqlite_round_trips() {
        use paladin_core::platform::container::allowance::{
            AllowanceLimitKind, AllowanceScopeKind, AllowanceWarning, NoticeOutcome, NoticeRecord,
        };
        use paladin_core::platform::container::cost::{Cost, CurrencyCode};
        use paladin_core::platform::container::run::RunId;

        let (path, url) = temp_sqlite_url("treasury_notices");
        let config = RunStoreConfig {
            backend: RunStoreBackend::Sqlite { path: url },
        };
        let notices = build_treasury_notices(&config)
            .await
            .expect("a configured sqlite backend must open")
            .expect("a configured sqlite backend must wire Some");

        let usd = CurrencyCode::new("USD").unwrap();
        let run = RunId::new_v7();
        let record = NoticeRecord {
            notice_id: "wired-notice-1".to_string(),
            tenant_id: "acme".to_string(),
            api_key_id: None,
            warning: AllowanceWarning {
                scope_kind: AllowanceScopeKind::Tenant,
                limit_kind: AllowanceLimitKind::Lifetime,
                balance: Cost::new(80, usd.clone()),
                ceiling: Cost::new(100, usd),
                window_start: None,
                window_end: None,
                warn_at: 80,
            },
            run_id: Some(run.clone()),
            recorded_at: chrono::DateTime::from_timestamp(1_000_000_000, 0).unwrap(),
            kind: Default::default(),
        };
        assert_eq!(
            notices.record(&record).await.unwrap(),
            NoticeOutcome::Recorded
        );
        assert_eq!(
            notices.notices_for_run(&run).await.unwrap(),
            vec![record],
            "the wired store round-trips a notice"
        );
        cleanup(&path);
    }

    #[tokio::test]
    async fn build_treasury_ledger_disabled_is_none() {
        let ledger = build_treasury_ledger(&RunStoreConfig::default())
            .await
            .expect("a disabled run store never fails to build");
        assert!(
            ledger.is_none(),
            "RunStoreBackend::Disabled must wire no treasury ledger"
        );
    }

    #[tokio::test]
    async fn build_treasury_ledger_sqlite_round_trips() {
        let (path, url) = temp_sqlite_url("treasury_ledger");
        let config = RunStoreConfig {
            backend: RunStoreBackend::Sqlite { path: url },
        };

        let ledger = build_treasury_ledger(&config)
            .await
            .expect("a configured sqlite backend must open")
            .expect("a configured sqlite backend must wire Some");

        let key = paladin_core::platform::container::treasury_ledger::SettlementKey {
            run_id: paladin_core::platform::container::run::RunId::new_v7(),
            superstep: 1,
            attempt: 1,
        };
        let amount = paladin_core::platform::container::cost::Cost::new(
            45_000_000,
            paladin_core::platform::container::cost::CurrencyCode::new("USD").unwrap(),
        );
        let mut breakdown = std::collections::BTreeMap::new();
        breakdown.insert("gpt-4".to_string(), 45_000_000i64);
        let scope = paladin_core::platform::container::treasury_ledger::LedgerScope::unattributed();
        let outcome = ledger
            .settle(
                paladin_core::platform::container::treasury_ledger::SettleRequest::unreserved(
                    scope.clone(),
                    key,
                    amount.clone(),
                    breakdown,
                ),
            )
            .await
            .expect("an unreserved settle must succeed against a fresh ledger");
        assert_eq!(
            outcome,
            paladin_core::platform::container::treasury_ledger::SettleOutcome::Settled
        );

        let rows = ledger
            .spend(
                paladin_core::platform::container::treasury_ledger::SpendQuery {
                    group_by:
                        paladin_core::platform::container::treasury_ledger::SpendGroupBy::Tenant,
                    tenant_id: Some(scope.tenant_id.clone()),
                    ..Default::default()
                },
            )
            .await
            .expect("spend must read back the settlement just written");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].amount, amount);

        cleanup(&path);
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
