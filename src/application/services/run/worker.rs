//! `RunWorkerPool` — dequeues a `LeasedRun`, re-reads the `Run`, drives the
//! engine, applies the resulting status transition, and acks (D-11, D-13).
//!
//! Lives in the facade, never in `paladin-web`: ADR-0031 forbids a
//! `paladin-web -> paladin-battalion` edge in the default build, and
//! driving `WarEngine` needs battalion. This is exactly the Phase 24
//! D-24/D-25 arrangement.
//!
//! ## Dispatch (D-09)
//!
//! The worker has ONE entry point that branches on the thread's latest
//! Waypoint: absent -> [`WarEngine::start`]; present with pending responses
//! parked on the `Run` -> [`WarEngine::resume_with`]; present otherwise ->
//! [`WarEngine::resume`]. [`WorkerDispatch::decide`] is the pure decision
//! function; `run_once` is the only caller.
//!
//! ## Heartbeat (D-10)
//!
//! [`LeaseHeartbeat`] extends a dequeued message's lease every `lease / 4`
//! for as long as it is held -- constructed immediately before an engine
//! call and dropped immediately after, so it never extends a lease the run
//! no longer needs.
//!
//! ## Outcome mapping (D-16, D-22, D-13)
//!
//! `map_outcome` is the one place a [`RunOutcome`] becomes a status
//! transition (or, for a shutdown-halted run, a requeue instead). The
//! `Halted`/`Cancelled` vocabulary split is deliberate: the *waypoint*
//! halted, but the *run* is recorded `Cancelled` only when a caller asked
//! for it.

use std::sync::Arc;
use std::time::Duration;

use thiserror::Error;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use paladin_battalion::engine::shutdown::ShutdownCoordinator;
use paladin_battalion::engine::{EngineError, RunOutcome, WarEngine};
use paladin_core::platform::container::battlefield::{FieldName, StateDelta};
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::parley::{ParleyRequest, ParleyResponse};
use paladin_core::platform::container::run::{
    ForkSpec, Run, RunEventKind, RunId, RunStatus, RunStreamEventKind, RunStreamMode,
};
use paladin_core::platform::container::waypoint::{Waypoint, WaypointId};
use paladin_core::platform::container::webhook::{WebhookDelivery, WebhookDeliveryId};
use paladin_ports::output::cancellation_probe::CancellationProbe;
use paladin_ports::output::paladin_port::PaladinPort;
use paladin_ports::output::run_queue_port::{LeaseToken, LeasedRun, RunQueuePort};
use paladin_ports::output::run_repository_port::{
    RunOutcomeRecord, RunRepositoryError, RunRepositoryPort,
};
use paladin_ports::output::waypoint_port::{WaypointError, WaypointPort};
use paladin_ports::output::webhook_delivery_port::WebhookDeliveryRepositoryPort;

use super::cancel::{DbCancellationProbe, LocalRunTokens};
use super::events::{RunEventBus, RunEventBusSink};
use super::resolver::{AssistantResolver, ResolveError, Runnable};
use super::webhook::{WebhookPayload, WebhookPayloadAssistant};

/// How long [`RunWorkerPool::run_once`] waits, after a dispatch's own
/// repository/queue write completes, before unbinding it from the D-24
/// event bus -- a best-effort window for `paladin-battalion`'s
/// fire-and-forget `TraceDispatcher` (ENG-FR-21) to deliver the last
/// superstep's trailing live trace event before its bus channel disappears.
/// See the `run_once` call site's own comment for the full rationale.
const TRACE_DRAIN_GRACE_PERIOD: Duration = Duration::from_millis(100);

/// Errors a single [`RunWorkerPool::run_once`] iteration can surface.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum WorkerError {
    /// The queue backend failed.
    #[error("run worker queue error: {0}")]
    Queue(#[from] paladin_ports::output::run_queue_port::QueueError),
    /// The repository backend failed.
    #[error("run worker repository error: {0}")]
    Repository(#[from] RunRepositoryError),
    /// The assistant resolver failed.
    #[error("run worker resolve error: {0}")]
    Resolve(#[from] ResolveError),
    /// The engine itself failed before or outside normal `RunOutcome`
    /// reporting (e.g. the graph failed structural validation) -- distinct
    /// from a `RunOutcome::Failed`, which this worker maps to a `Failed`
    /// run rather than surfacing as this variant.
    #[error("run worker engine error: {0}")]
    Engine(#[from] EngineError),
    /// The waypoint backend failed while the worker queried the thread's
    /// latest checkpoint for dispatch (D-09).
    #[error("run worker waypoint error: {0}")]
    Waypoint(#[from] WaypointError),
}

/// Construction knobs for [`RunWorkerPool::spawn`].
///
/// Facade-internal; `src/config/`'s `RunWorkerConfig` (a later plan)
/// converts into this at wiring time. `lease` is carried here for
/// construction-time convenience (so one config struct produces both a
/// [`RunWorkerPool`] and its [`RunWorkerOptions`]) -- `spawn` itself only
/// consumes `concurrency` and `min_probe_interval`; the lease a worker
/// requests on each dequeue is the one the pool was constructed with.
#[derive(Debug, Clone, Copy)]
pub struct RunWorkerOptions {
    /// How many worker tasks [`RunWorkerPool::spawn`] starts.
    pub concurrency: usize,
    /// The lease duration each worker requests when dequeuing.
    pub lease: Duration,
    /// How long an idle worker task sleeps between empty-queue polls.
    pub min_probe_interval: Duration,
}

/// Periodically extends a leased run's queue visibility timeout at
/// `lease / 4` (D-10) for as long as it is held, so a worker taking up to
/// three quarters of the lease duration to process a run is never treated
/// as dead by redelivery.
///
/// Construct immediately before an engine call and drop immediately after
/// it returns: `Drop` aborts the background task, so a dropped
/// `LeaseHeartbeat` never extends a lease the run no longer needs.
///
/// (WR-04) A non-positive `lease` starts no background task at all. A zero
/// interval (`lease / 4` for a zero lease) would make `tokio::time::sleep`
/// resolve immediately, turning the extend-lease loop into a CPU-bound
/// spin rather than a periodic heartbeat. `RunWorkerConfig::validate()`
/// enforces a four-second floor for this crate's one production call
/// site, but `spawn` is public API and must not depend on a caller having
/// gone through the config layer to avoid this failure mode.
pub struct LeaseHeartbeat {
    handle: Option<JoinHandle<()>>,
}

impl LeaseHeartbeat {
    /// Spawn a background task extending `token`'s lease by `lease` every
    /// `lease / 4`, until this handle is dropped.
    ///
    /// A non-positive `lease` starts no task at all (WR-04): a warning is
    /// logged naming the misuse, and dropping the returned handle is a
    /// no-op.
    pub fn spawn(queue: Arc<dyn RunQueuePort>, token: LeaseToken, lease: Duration) -> Self {
        if lease.is_zero() {
            log::warn!(
                "LeaseHeartbeat::spawn called with a non-positive lease ({lease:?}); \
                 no heartbeat task will run for this lease token"
            );
            return Self { handle: None };
        }
        let interval = lease / 4;
        let handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;
                if queue.extend_lease(&token, lease).await.is_err() {
                    // The lease is gone (acked, nacked, or already expired
                    // and reclaimed elsewhere) -- nothing left to extend.
                    break;
                }
            }
        });
        Self {
            handle: Some(handle),
        }
    }
}

impl Drop for LeaseHeartbeat {
    fn drop(&mut self) {
        if let Some(handle) = &self.handle {
            handle.abort();
        }
    }
}

/// What a worker does with a resolved workflow run, decided purely from the
/// thread's latest [`Waypoint`], the run's parked responses, and (D-45) the
/// run's own `fork_from` (D-09).
#[derive(Debug, Clone, PartialEq)]
pub enum WorkerDispatch {
    /// No Waypoint exists yet for this thread: begin a fresh run.
    Start,
    /// A Waypoint exists and the run carries no parked responses: continue
    /// from it.
    Resume,
    /// A Waypoint exists and the run carries parked responses: continue
    /// from it, delivering the responses (D-23).
    ResumeWith(Vec<ParleyResponse>),
    /// `run.fork_from` is `Some` and the thread's latest Waypoint does not
    /// yet reflect this fork point (`fork_of != Some(from)`) -- the fork
    /// itself has not run yet: drive `WarEngine::fork` (D-45).
    Fork {
        /// The Waypoint to fork from.
        from: String,
        /// An optional state edit merged at the fork point.
        edit: Option<serde_json::Value>,
    },
}

impl WorkerDispatch {
    /// Decide dispatch purely from whether a Waypoint exists, whether
    /// responses are parked on the run, and the run's own `fork_from`
    /// (D-09, D-45) -- the worker's single entry point.
    ///
    /// A fork not yet started (`fork_from` is `Some` and the latest
    /// Waypoint's `fork_of` does not already match it) takes priority over
    /// `Start`/`Resume`/`ResumeWith`: redelivery AFTER the fork Waypoint
    /// exists falls through to the normal resume path below, since
    /// `fork_of` then matches.
    pub fn decide(
        latest: Option<&Waypoint>,
        pending: &[ParleyResponse],
        fork_from: Option<&ForkSpec>,
    ) -> WorkerDispatch {
        if let Some(fork) = fork_from {
            let already_forked = latest
                .and_then(|wp| wp.fork_of.as_ref())
                .map(|id| id.to_string() == fork.from_waypoint_id)
                .unwrap_or(false);
            if !already_forked {
                return WorkerDispatch::Fork {
                    from: fork.from_waypoint_id.clone(),
                    edit: fork.edit.clone(),
                };
            }
        }
        match latest {
            None => WorkerDispatch::Start,
            Some(_) if !pending.is_empty() => WorkerDispatch::ResumeWith(pending.to_vec()),
            Some(_) => WorkerDispatch::Resume,
        }
    }
}

/// Parse a Waypoint id string (as stored on [`ForkSpec::from_waypoint_id`])
/// back into a [`WaypointId`], reusing its `#[serde(transparent)]`
/// `Deserialize` impl over a bare JSON string -- `WaypointId` exposes no
/// public string-parsing constructor of its own (core type, ADR-0016),
/// mirroring `thread_controller::parse_waypoint_id`'s identical trick one
/// layer up the stack.
fn parse_fork_waypoint_id(raw: &str) -> Option<WaypointId> {
    serde_json::from_value(serde_json::Value::String(raw.to_string())).ok()
}

/// Merge a fork's optional JSON `edit` object into a [`StateDelta`] (D-45).
/// A non-object (or absent) edit yields an empty delta -- a no-op merge,
/// never a panic; a key that fails [`FieldName::new`] (only the empty
/// string) is silently dropped, since [`WarEngine::fork`] itself already
/// rejects any field name its own schema does not declare via a typed
/// `EngineError::Battlefield` at merge time.
fn fork_edit_to_state_delta(edit: Option<serde_json::Value>) -> StateDelta {
    let mut delta = StateDelta::new();
    let Some(serde_json::Value::Object(map)) = edit else {
        return delta;
    };
    for (key, value) in map {
        if let Ok(field) = FieldName::new(key) {
            delta.values.insert(field, value);
        }
    }
    delta
}

/// What [`RunWorkerPool::run_once`] does with a successful [`RunOutcome`]
/// (D-16, D-22, D-13). Pure and unit-tested independent of any repository
/// or queue.
#[derive(Debug, Clone, PartialEq)]
enum OutcomeAction {
    /// Transition the run `Running -> to`, record the given outcome
    /// fields, then ack the queue message.
    Transition {
        /// The status to transition into.
        to: RunStatus,
        /// The outcome fields to record alongside the transition.
        outcome: RunOutcomeRecord,
    },
    /// Leave the run `Running` and nack the message for immediate
    /// redelivery -- the shutdown-drain path (D-13): a run halted by the
    /// pool's own shutdown token (not by a caller's cancel request) is not
    /// finished, it is a valid restart point another worker should pick
    /// straight back up.
    LeaveRunningAndRequeue,
}

/// Map a terminal/suspension [`RunStatus`] to the [`RunEventKind`] a
/// webhook subscribes to, or `None` for a non-terminal, non-suspension
/// status this hook never fires for (`Queued`/`Running`).
fn run_status_to_event_kind(status: RunStatus) -> Option<RunEventKind> {
    match status {
        RunStatus::AwaitingInput => Some(RunEventKind::AwaitingInput),
        RunStatus::Completed => Some(RunEventKind::Completed),
        RunStatus::Failed => Some(RunEventKind::Failed),
        RunStatus::Halted => Some(RunEventKind::Halted),
        RunStatus::Cancelled => Some(RunEventKind::Cancelled),
        RunStatus::Queued | RunStatus::Running => None,
    }
}

/// Build the `Pending` [`WebhookDelivery`] this outcome implies, or `None`
/// if the run carries no webhook or its `events` list does not subscribe to
/// `kind` (PLAT-FR-14, D-23). Pure and unit-tested independent of any
/// repository -- the caller enqueues the result.
fn webhook_delivery_for_outcome(
    run: &Run,
    kind: RunEventKind,
    status: RunStatus,
    parleys: Option<&[ParleyRequest]>,
    now: chrono::DateTime<chrono::Utc>,
) -> Option<WebhookDelivery> {
    let webhook = run.webhook.as_ref()?;
    if !webhook.events.contains(&kind) {
        return None;
    }

    let payload = WebhookPayload {
        run_id: run.run_id.clone(),
        thread_id: run.thread_id.clone(),
        assistant: WebhookPayloadAssistant {
            assistant_id: run.assistant.assistant_id.clone(),
            version: run.assistant.version,
        },
        status,
        event: kind,
        timestamp: now,
        attempt: run.attempt,
        parleys: parleys.map(|p| p.to_vec()),
    };
    let payload_json = serde_json::to_string(&payload).ok()?;

    Some(WebhookDelivery::new(
        WebhookDeliveryId::new_v7(),
        run.run_id.clone(),
        run.thread_id.clone(),
        kind,
        webhook.url.clone(),
        payload_json,
        now,
    ))
}

/// Map a [`RunOutcome`] to the status transition (or requeue) it implies,
/// given whether the run's cancellation flag is set and whether the pool
/// itself is shutting down (D-16, D-22, D-13).
fn map_outcome(outcome: &RunOutcome, cancel_requested: bool, shutting_down: bool) -> OutcomeAction {
    match outcome {
        RunOutcome::Completed { waypoint, .. } => OutcomeAction::Transition {
            to: RunStatus::Completed,
            outcome: RunOutcomeRecord {
                error: None,
                output: None,
                final_waypoint_id: Some(waypoint.to_string()),
            },
        },
        RunOutcome::Failed { error, waypoint } => OutcomeAction::Transition {
            to: RunStatus::Failed,
            outcome: RunOutcomeRecord {
                error: Some(error.to_string()),
                output: None,
                final_waypoint_id: waypoint.map(|w| w.to_string()),
            },
        },
        // D-22: AwaitingInput releases the worker by ACKing -- a suspended
        // run is durably parked, not unfinished queue work.
        RunOutcome::AwaitingInput { waypoint, .. } => OutcomeAction::Transition {
            to: RunStatus::AwaitingInput,
            outcome: RunOutcomeRecord {
                error: None,
                output: None,
                final_waypoint_id: Some(waypoint.to_string()),
            },
        },
        RunOutcome::Halted { waypoint } => {
            if cancel_requested {
                // D-16: the caller asked for this. The *waypoint* halted;
                // the *run* is recorded Cancelled.
                OutcomeAction::Transition {
                    to: RunStatus::Cancelled,
                    outcome: RunOutcomeRecord {
                        error: None,
                        output: None,
                        final_waypoint_id: Some(waypoint.to_string()),
                    },
                }
            } else if shutting_down {
                // D-13: the pool's own shutdown token fired. This is a
                // drain, not a finish -- leave the run Running and make the
                // message visible again immediately.
                OutcomeAction::LeaveRunningAndRequeue
            } else {
                // Neither a caller cancel nor a shutdown drain: a bare
                // cancellation token with no recorded request. Record it
                // Halted -- still a valid resume point.
                OutcomeAction::Transition {
                    to: RunStatus::Halted,
                    outcome: RunOutcomeRecord {
                        error: None,
                        output: None,
                        final_waypoint_id: Some(waypoint.to_string()),
                    },
                }
            }
        }
    }
}

/// Drives runs dequeued from a [`RunQueuePort`] through a real
/// [`WarEngine`], applying the resulting status transition through a
/// [`RunRepositoryPort`].
pub struct RunWorkerPool<W: WaypointPort> {
    engine: Arc<WarEngine<W>>,
    waypoint_port: Arc<W>,
    repository: Arc<dyn RunRepositoryPort>,
    queue: Arc<dyn RunQueuePort>,
    resolver: Arc<dyn AssistantResolver>,
    lease: Duration,
    coordinator: ShutdownCoordinator,
    paladin_port: Option<Arc<dyn PaladinPort>>,
    // --- D-14, D-16, PLAT-FR-04: when wired (`with_engine_factory`),
    // `run_once` builds a FRESH per-run engine through this factory instead
    // of using `self.engine` directly, so a per-run `CancellationToken`
    // (never shared across concurrent runs) can be attached beside
    // whatever `CancellationProbe` the factory's own closure wires in.
    // `WarEngine::with_cancellation_token` is by-value and this pool holds
    // only ONE shared `Arc<WarEngine<W>>` for the "no factory" path --
    // rebuilding a cheap, all-`Arc`-fields engine per run is the smaller
    // change (documented deviation) rather than adding a
    // `WarEngine::with_run_token` mutator to `paladin-battalion`, a crate
    // outside this task's declared file scope. `None` (the default)
    // preserves 27-04's exact behavior verbatim: every run shares the ONE
    // engine configured at pool construction, and `LocalRunTokens` is never
    // populated for it (a caller relying only on the cross-instance DB flag
    // still works correctly, just without the instant local fast-path).
    engine_factory: Option<Arc<dyn Fn(CancellationToken) -> WarEngine<W> + Send + Sync>>,
    /// Registry [`RunSubmissionService`](super::submission::RunSubmissionService)
    /// shares (via `with_local_tokens`) to observe whether THIS instance is
    /// dispatching a given run right now (D-16). Populated only while
    /// `engine_factory` is `Some` -- see [`RunWorkerPool::local_tokens`].
    local_tokens: LocalRunTokens,
    /// The D-14 cross-instance probe, when [`RunWorkerPool::with_cancellation_probing`]
    /// wires one: attached to every per-run engine `engine_factory`
    /// produces via [`WarEngine::with_cancellation_probe`] BESIDE the
    /// per-run token, so the caller's own factory closure never needs to
    /// know about probes at all -- this pool owns that wiring, reading its
    /// own `repository` field.
    cancellation_probe: Option<Arc<dyn CancellationProbe>>,
    /// The D-24 per-run broadcast bus (PLAT-FR-07), when
    /// [`RunWorkerPool::with_event_bus`] wires one: `run_once` `bind`s the
    /// dispatch's thread/run before driving the engine, attaches
    /// [`RunEventBusSink`] to a per-run `engine_factory` engine so
    /// `superstep`/`node_started`/`node_finished`/`state_delta` bridge live
    /// (D-25 correction), publishes `parley`/`done`/`error` directly from
    /// the `RunOutcome` it already matches on, then `unbind`s. `None`
    /// (the default) preserves every prior plan's behavior verbatim -- no
    /// bind/publish/unbind call happens anywhere in `run_once`.
    event_bus: Option<Arc<RunEventBus>>,
    /// The D-40 durable delivery queue, when
    /// [`RunWorkerPool::with_webhook_deliveries`] wires one: `run_once`
    /// enqueues a `Pending` [`WebhookDelivery`] on every terminal/suspension
    /// transition whose run subscribes to that event (PLAT-FR-14). `None`
    /// (the default) preserves every prior plan's behavior verbatim -- no
    /// enqueue call happens anywhere in `run_once`. A repository error here
    /// is logged and NEVER changes the run's own status (prohibition P2):
    /// it is enqueued strictly after the run's own status write/ack has
    /// already succeeded.
    webhook_deliveries: Option<Arc<dyn WebhookDeliveryRepositoryPort>>,
}

impl<W: WaypointPort + 'static> RunWorkerPool<W> {
    /// Construct a worker pool over the given engine, waypoint store,
    /// repository, queue and resolver, with the given lease duration.
    ///
    /// `waypoint_port` is the SAME store the engine was constructed over --
    /// the pool needs its own handle to it to decide dispatch (D-09)
    /// without `WarEngine` exposing an accessor for its own copy.
    ///
    /// Defaults to a fresh [`ShutdownCoordinator`] (never registered with
    /// anything else) and no wired [`PaladinPort`] (an `Agent`-kind run
    /// nacks rather than executes) -- see
    /// [`RunWorkerPool::with_shutdown_coordinator`] and
    /// [`RunWorkerPool::with_paladin_port`].
    pub fn new(
        engine: Arc<WarEngine<W>>,
        waypoint_port: Arc<W>,
        repository: Arc<dyn RunRepositoryPort>,
        queue: Arc<dyn RunQueuePort>,
        resolver: Arc<dyn AssistantResolver>,
        lease: Duration,
    ) -> Self {
        Self {
            engine,
            waypoint_port,
            repository,
            queue,
            resolver,
            lease,
            coordinator: ShutdownCoordinator::new(),
            paladin_port: None,
            engine_factory: None,
            local_tokens: LocalRunTokens::new(),
            cancellation_probe: None,
            event_bus: None,
            webhook_deliveries: None,
        }
    }

    /// Wire a per-run engine factory (D-14, D-16): instead of dispatching
    /// every run through the ONE shared engine passed to
    /// [`RunWorkerPool::new`], `run_once` calls `factory(child_token)` to
    /// build a fresh engine for THIS run, where `child_token` is a
    /// [`CancellationToken::child_token`] of the pool's own
    /// [`ShutdownCoordinator`] token (so a process shutdown still cascades
    /// to every in-flight per-run engine, exactly as the shared-engine path
    /// already does) -- registered in [`RunWorkerPool::local_tokens`] for
    /// the duration of dispatch, so
    /// [`RunSubmissionService::cancel`](super::submission::RunSubmissionService::cancel)
    /// can fire it directly when this same instance holds the run.
    ///
    /// The caller's closure is responsible for wiring whatever
    /// `CancellationProbe` (typically a shared
    /// [`DbCancellationProbe`](super::cancel::DbCancellationProbe)), node
    /// cache, vault, etc. the production engine needs -- this pool has no
    /// visibility into `WarEngine`'s private construction fields beyond
    /// what the closure itself captures.
    pub fn with_engine_factory(
        mut self,
        factory: Arc<dyn Fn(CancellationToken) -> WarEngine<W> + Send + Sync>,
    ) -> Self {
        self.engine_factory = Some(factory);
        self
    }

    /// This pool's [`LocalRunTokens`] registry -- share the SAME clone with
    /// [`RunSubmissionService::with_local_tokens`](super::submission::RunSubmissionService::with_local_tokens)
    /// so `cancel` can observe local dispatch. A clone made before
    /// [`RunWorkerPool::with_engine_factory`] is ever called observes an
    /// always-empty map (correct: nothing is EVER registered on the
    /// no-factory path).
    pub fn local_tokens(&self) -> LocalRunTokens {
        self.local_tokens.clone()
    }

    /// Enable D-14 cross-instance cancellation: build the pool's own
    /// [`DbCancellationProbe`], reading THIS pool's `repository` and
    /// debounced by `min_probe_interval` (D-15), and attach it to every
    /// per-run engine [`RunWorkerPool::with_engine_factory`] produces via
    /// [`WarEngine::with_cancellation_probe`]. Has no effect unless
    /// `with_engine_factory` is ALSO wired -- the shared-engine ("no
    /// factory") path attaches a probe directly on that engine at
    /// construction instead, exactly like an ordinary
    /// `WarEngine::with_cancellation_probe` call.
    pub fn with_cancellation_probing(mut self, min_probe_interval: Duration) -> Self {
        self.cancellation_probe = Some(Arc::new(DbCancellationProbe::new(
            self.repository.clone(),
            min_probe_interval,
        )));
        self
    }

    /// Wire the D-24 per-run broadcast bus (PLAT-FR-07): `run_once` binds
    /// the dispatch's thread/run before driving the engine, attaches a
    /// fresh [`RunEventBusSink`] to whatever per-run engine
    /// [`Self::with_engine_factory`] produces (mirroring how
    /// [`Self::with_cancellation_probing`] attaches its own probe), and
    /// publishes `parley`/`done`/`error` directly from the `RunOutcome`
    /// this pool already matches on. Has no effect on the shared-engine
    /// ("no factory") path's own trace bridging -- attach `bus`'s own
    /// [`RunEventBusSink`] to that engine directly at construction (mirrors
    /// [`Self::with_cancellation_probing`]'s own documented limitation);
    /// this pool still binds/publishes/unbinds regardless, since those do
    /// not depend on which engine instance is used.
    pub fn with_event_bus(mut self, bus: Arc<RunEventBus>) -> Self {
        self.event_bus = Some(bus);
        self
    }

    /// Wire the D-40 durable webhook delivery queue: `run_once` enqueues a
    /// `Pending` [`WebhookDelivery`] on every terminal/suspension
    /// transition whose run subscribes to that event, straight from the
    /// `RunOutcome`/`RunStatus` this pool already computes. A repository
    /// error enqueueing is logged and NEVER affects the run's own status
    /// (prohibition P2, PLAT-FR-14).
    pub fn with_webhook_deliveries(
        mut self,
        webhook_deliveries: Arc<dyn WebhookDeliveryRepositoryPort>,
    ) -> Self {
        self.webhook_deliveries = Some(webhook_deliveries);
        self
    }

    /// Share an existing [`ShutdownCoordinator`] rather than the pool's own
    /// fresh one -- so [`RunWorkerPool::spawn`]'s tasks drain alongside
    /// every other in-flight engine run the process registers (D-13).
    pub fn with_shutdown_coordinator(mut self, coordinator: ShutdownCoordinator) -> Self {
        self.coordinator = coordinator;
        self
    }

    /// Wire a [`PaladinPort`] so a legacy `Agent`-kind assistant can be
    /// executed directly (not through the engine, which has no Waypoint for
    /// it). Without this, an `Agent`-kind run is nacked for redelivery
    /// rather than executed.
    pub fn with_paladin_port(mut self, paladin_port: Arc<dyn PaladinPort>) -> Self {
        self.paladin_port = Some(paladin_port);
        self
    }

    /// Start `options.concurrency` worker tasks, each registered with this
    /// pool's [`ShutdownCoordinator`] (D-13): on shutdown, a task stops
    /// dequeuing once its current `run_once` iteration returns and drops
    /// its `RunGuard`. An in-flight iteration's own engine call observes
    /// cancellation at its next superstep boundary through whatever
    /// cancellation token the caller wired into the engine -- this pool
    /// only decides what to do with the `Halted` outcome that produces
    /// (see `map_outcome`'s shutdown branch).
    pub fn spawn(self: Arc<Self>, options: RunWorkerOptions) -> Vec<JoinHandle<()>> {
        let mut handles = Vec::with_capacity(options.concurrency);
        for _ in 0..options.concurrency {
            let pool = Arc::clone(&self);
            let (child_token, guard) = self.coordinator.register();
            let min_probe_interval = options.min_probe_interval;
            handles.push(tokio::spawn(async move {
                let _guard = guard;
                loop {
                    if child_token.is_cancelled() {
                        break;
                    }
                    match pool.run_once().await {
                        Ok(true) => {}
                        Ok(false) => {
                            tokio::select! {
                                _ = tokio::time::sleep(min_probe_interval) => {}
                                _ = child_token.cancelled() => break,
                            }
                        }
                        Err(error) => {
                            log::warn!("run worker iteration failed: {error}");
                            tokio::select! {
                                _ = tokio::time::sleep(min_probe_interval) => {}
                                _ = child_token.cancelled() => break,
                            }
                        }
                    }
                }
            }));
        }
        handles
    }

    /// Dequeue and process at most one run.
    ///
    /// Returns `Ok(true)` if a run was processed (including a stale message
    /// for an already-terminal run, or an unwired `Agent`-kind run that was
    /// nacked), `Ok(false)` if the queue was empty.
    pub async fn run_once(&self) -> Result<bool, WorkerError> {
        let Some(leased) = self.queue.dequeue(self.lease).await? else {
            return Ok(false);
        };

        // Never trust the queue message alone -- re-read the Run through
        // the repository, the single source of truth for status (D-07).
        let Some(run) = self.repository.get(&leased.queued.run_id).await? else {
            // The run vanished from the repository between enqueue and
            // dequeue; ack so the queue never spins on a run this worker
            // can never process.
            self.queue.ack(&leased.token).await?;
            return Ok(true);
        };

        match run.status {
            RunStatus::Queued => {
                self.repository
                    .update_status(
                        &run.run_id,
                        RunStatus::Queued,
                        RunStatus::Running,
                        chrono::Utc::now(),
                    )
                    .await?;
            }
            RunStatus::AwaitingInput => {
                self.repository
                    .update_status(
                        &run.run_id,
                        RunStatus::AwaitingInput,
                        RunStatus::Running,
                        chrono::Utc::now(),
                    )
                    .await?;
            }
            RunStatus::Running => {
                // A redelivery: no status change, just the shared attempt
                // counter (D-23).
                self.repository.bump_attempt(&run.run_id).await?;
            }
            _ => {
                // A stale message for an already-terminal run: ack and
                // drop, never touch the engine (D-07).
                self.queue.ack(&leased.token).await?;
                return Ok(true);
            }
        }

        let resolved = self
            .resolver
            .resolve(&run.assistant.assistant_id, Some(run.assistant.version))
            .await?;

        let graph = match resolved.runnable {
            Runnable::Workflow(graph) => graph,
            Runnable::Agent(paladin) => {
                return self.run_agent(&leased, &run, paladin).await;
            }
        };

        let latest = self.waypoint_port.latest(&run.thread_id).await?;
        let dispatch = WorkerDispatch::decide(
            latest.as_ref(),
            &run.pending_responses,
            run.fork_from.as_ref(),
        );

        // --- D-14, D-16: when an `engine_factory` is wired, build a FRESH
        // per-run engine carrying its own child `CancellationToken`,
        // registered in `local_tokens` for the duration of this dispatch --
        // see `RunWorkerPool::with_engine_factory`'s own rustdoc. When
        // `with_cancellation_probing` was ALSO called, this pool attaches
        // its own `DbCancellationProbe` to that same per-run engine here
        // (`with_cancellation_probe`), beside the per-run token -- the
        // caller's factory closure never needs to know about probes at
        // all. Otherwise (the default), fall back to the ONE shared engine
        // exactly as 27-04 left it, with no local-token registration.
        let (run_engine, local_token_guard): (Arc<WarEngine<W>>, Option<RunId>) =
            match &self.engine_factory {
                Some(factory) => {
                    let child_token = self.coordinator.token().child_token();
                    self.local_tokens
                        .register(run.run_id.clone(), child_token.clone())
                        .await;
                    let mut engine = factory(child_token);
                    if let Some(probe) = &self.cancellation_probe {
                        engine = engine.with_cancellation_probe(Arc::clone(probe));
                    }
                    if let Some(bus) = &self.event_bus {
                        engine =
                            engine.with_trace_sink(Arc::new(RunEventBusSink::new(Arc::clone(bus))));
                    }
                    (Arc::new(engine), Some(run.run_id.clone()))
                }
                None => (Arc::clone(&self.engine), None),
            };

        // D-24: bind THIS thread/run on the bus before dispatch, so a
        // `TraceSink` callback firing mid-superstep has somewhere to
        // publish to, and so a subscriber connecting right after this call
        // sees the live path rather than falling back to degraded.
        if let Some(bus) = &self.event_bus {
            bus.bind(run.thread_id.clone(), run.run_id.clone()).await;
        }

        let heartbeat = LeaseHeartbeat::spawn(self.queue.clone(), leased.token.clone(), self.lease);
        let outcome_result = match dispatch {
            WorkerDispatch::Start => {
                run_engine
                    .start(&graph, run.thread_id.clone(), StateDelta::new())
                    .await
            }
            WorkerDispatch::Resume => run_engine.resume(&graph, run.thread_id.clone()).await,
            WorkerDispatch::ResumeWith(responses) => {
                let result = run_engine
                    .resume_with(&graph, run.thread_id.clone(), responses)
                    .await;
                if result.is_ok() {
                    self.repository.clear_pending_responses(&run.run_id).await?;
                }
                result
            }
            WorkerDispatch::Fork { from, edit } => match parse_fork_waypoint_id(&from) {
                Some(waypoint_id) => {
                    let delta = fork_edit_to_state_delta(edit);
                    run_engine
                        .fork(&graph, &run.thread_id, waypoint_id, delta)
                        .await
                }
                None => {
                    // D-45: `from` is this service's own prior write
                    // (`RunSubmissionService::fork` stores
                    // `WaypointId::to_string()`), so a parse failure here
                    // means the persisted row is corrupt -- record it as an
                    // engine failure rather than silently ignoring it or
                    // panicking. `heartbeat` is dropped (and stops) via the
                    // normal early-return Drop, exactly as every other
                    // return path in this function.
                    drop(heartbeat);
                    return self
                        .record_engine_failure(
                            &leased,
                            &run,
                            format!("corrupt fork_from.from_waypoint_id: {from}"),
                        )
                        .await;
                }
            },
        };
        // D-10: stop heartbeating the moment the run returns.
        drop(heartbeat);
        // The run engine's own dispatch has finished one way or another --
        // this instance is no longer the one to signal, so its local
        // registration (if any) is stale from here on.
        if let Some(run_id) = local_token_guard {
            self.local_tokens.remove(&run_id).await;
        }

        let outcome = match outcome_result {
            Ok(outcome) => outcome,
            Err(engine_error) => {
                return self
                    .record_engine_failure(&leased, &run, engine_error.to_string())
                    .await;
            }
        };

        let cancel_requested = self.repository.is_cancel_requested(&run.thread_id).await?;
        let shutting_down = self.coordinator.token().is_cancelled();

        // D-24: publish the terminal/suspension event this dispatch's own
        // `RunOutcome` implies, straight from the `RunOutcome` this worker
        // already matches on -- `parley`/`done`/`error` are NOT bridged by
        // the `TraceSink` adapter (D-25 correction: today's `TraceEvent` can
        // produce none of the three). `unbind` itself is deferred past the
        // repository/queue write below -- see the comment there for why.
        if let Some(bus) = &self.event_bus {
            match &outcome {
                RunOutcome::AwaitingInput { parleys, waypoint } => {
                    bus.publish(
                        &run.run_id,
                        &run.thread_id,
                        RunStreamEventKind::Parley,
                        RunStreamMode::Live,
                        serde_json::json!({
                            "waypoint_id": waypoint.to_string(),
                            "parleys": parleys,
                        }),
                    )
                    .await;
                }
                RunOutcome::Completed { waypoint, .. } => {
                    bus.publish(
                        &run.run_id,
                        &run.thread_id,
                        RunStreamEventKind::Done,
                        RunStreamMode::Live,
                        serde_json::json!({
                            "status": "completed",
                            "waypoint_id": waypoint.to_string(),
                        }),
                    )
                    .await;
                }
                RunOutcome::Halted { waypoint } => {
                    // D-16: the WIRE status mirrors `map_outcome`'s own
                    // cancelled/halted split -- the shutdown-drain case
                    // (`OutcomeAction::LeaveRunningAndRequeue`) still
                    // reports `done` here, since this instance really is
                    // done dispatching it; a later worker resumes it
                    // through a fresh `bind`.
                    let status = if cancel_requested {
                        "cancelled"
                    } else {
                        "halted"
                    };
                    bus.publish(
                        &run.run_id,
                        &run.thread_id,
                        RunStreamEventKind::Done,
                        RunStreamMode::Live,
                        serde_json::json!({ "status": status, "waypoint_id": waypoint.to_string() }),
                    )
                    .await;
                }
                RunOutcome::Failed { error, waypoint } => {
                    bus.publish(
                        &run.run_id,
                        &run.thread_id,
                        RunStreamEventKind::Error,
                        RunStreamMode::Live,
                        serde_json::json!({
                            "status": "failed",
                            "message": error.to_string(),
                            "waypoint_id": waypoint.map(|w| w.to_string()),
                        }),
                    )
                    .await;
                }
            }
        }

        match map_outcome(&outcome, cancel_requested, shutting_down) {
            OutcomeAction::Transition {
                to,
                outcome: record,
            } => {
                self.repository
                    .update_status(&run.run_id, RunStatus::Running, to, chrono::Utc::now())
                    .await?;
                self.repository.record_outcome(&run.run_id, record).await?;
                self.queue.ack(&leased.token).await?;

                // D-40, PLAT-FR-14: enqueue a webhook delivery for this
                // transition, strictly AFTER the run's own status write and
                // ack have already succeeded -- a delivery-repository
                // failure here is logged and never rolls back or changes
                // the run's own status (prohibition P2).
                if let Some(deliveries) = &self.webhook_deliveries
                    && let Some(kind) = run_status_to_event_kind(to)
                {
                    let parleys = match &outcome {
                        RunOutcome::AwaitingInput { parleys, .. } => Some(parleys.as_slice()),
                        _ => None,
                    };
                    if let Some(delivery) =
                        webhook_delivery_for_outcome(&run, kind, to, parleys, chrono::Utc::now())
                        && let Err(error) = deliveries.enqueue(delivery).await
                    {
                        log::warn!(
                            "run worker: failed to enqueue webhook delivery for run {}: {error}",
                            run.run_id
                        );
                    }
                }
            }
            OutcomeAction::LeaveRunningAndRequeue => {
                self.queue.nack(&leased.token, Duration::ZERO).await?;
            }
        }

        if let Some(bus) = &self.event_bus {
            // `paladin-battalion`'s `TraceDispatcher` is deliberately
            // fire-and-forget (ENG-FR-21, T-22-30): the LAST superstep's
            // `state_delta`/`node_finished` may still be queued on its
            // background consumer task when `run_engine.start`/`resume`
            // returns, racing this worker's own synchronous terminal
            // publish above. Unbinding immediately would silently drop
            // that trailing live event the instant it arrives (an unbound
            // thread is a documented no-op, not an error) -- exactly the
            // failure `state_delta_carries_field_names_only` (27-10)
            // caught. `paladin-battalion` exposes no "wait for drain" seam
            // this task's file scope can call, so a short, generous grace
            // period is the smallest available mitigation: it delays only
            // this bus's own cleanup, never the run's repository/queue
            // write above, which has already completed by this point.
            tokio::time::sleep(TRACE_DRAIN_GRACE_PERIOD).await;
            bus.unbind(&run.thread_id).await;
        }

        Ok(true)
    }

    /// Execute a legacy `Agent`-kind assistant directly through the pool's
    /// [`PaladinPort`] (no Waypoint exists for it, so dispatch does not
    /// apply): `input_text` is the run's `input["input"]` string, or the
    /// whole input serialised when that key is absent or not a string. A
    /// redelivery simply restarts it. Without a wired `PaladinPort`, nacks
    /// for redelivery rather than executing (the 27-01 fallback, preserved
    /// until a later plan always wires one).
    async fn run_agent(
        &self,
        leased: &LeasedRun,
        run: &Run,
        paladin: Arc<Paladin>,
    ) -> Result<bool, WorkerError> {
        let Some(paladin_port) = &self.paladin_port else {
            self.queue
                .nack(&leased.token, Duration::from_secs(1))
                .await?;
            return Ok(true);
        };

        let input_text = match run.input.get("input").and_then(|v| v.as_str()) {
            Some(text) => text.to_string(),
            None => run.input.to_string(),
        };

        match paladin_port.execute(paladin.as_ref(), &input_text).await {
            Ok(result) => {
                self.repository
                    .update_status(
                        &run.run_id,
                        RunStatus::Running,
                        RunStatus::Completed,
                        chrono::Utc::now(),
                    )
                    .await?;
                self.repository
                    .record_outcome(
                        &run.run_id,
                        RunOutcomeRecord {
                            error: None,
                            output: Some(serde_json::Value::String(result.output)),
                            final_waypoint_id: None,
                        },
                    )
                    .await?;
                self.queue.ack(&leased.token).await?;
                Ok(true)
            }
            Err(error) => {
                self.record_engine_failure(leased, run, error.to_string())
                    .await
            }
        }
    }

    /// Record an `EngineError` (or a `PaladinError`, for the `Agent`-kind
    /// path) returned outside normal `RunOutcome` reporting as a `Failed`
    /// run -- never a panic. If the repository write itself fails, log at
    /// `warn` and nack with a 1s delay so the run is retried rather than
    /// lost.
    async fn record_engine_failure(
        &self,
        leased: &LeasedRun,
        run: &Run,
        error_text: String,
    ) -> Result<bool, WorkerError> {
        // D-24: an `EngineError` (outside normal `RunOutcome` reporting) is
        // still a run-ending `error` on the stream -- then unbind. A no-op
        // for the `run_agent` caller, which never `bind`s in the first
        // place (`unbind` on an unbound thread is always a safe no-op).
        if let Some(bus) = &self.event_bus {
            bus.publish(
                &run.run_id,
                &run.thread_id,
                RunStreamEventKind::Error,
                RunStreamMode::Live,
                serde_json::json!({ "status": "failed", "message": error_text.clone() }),
            )
            .await;
            bus.unbind(&run.thread_id).await;
        }

        let now = chrono::Utc::now();
        let record_result: Result<(), RunRepositoryError> = async {
            self.repository
                .update_status(&run.run_id, RunStatus::Running, RunStatus::Failed, now)
                .await?;
            self.repository
                .record_outcome(
                    &run.run_id,
                    RunOutcomeRecord {
                        error: Some(error_text.clone()),
                        output: None,
                        final_waypoint_id: None,
                    },
                )
                .await?;
            Ok(())
        }
        .await;

        match record_result {
            Ok(()) => {
                self.queue.ack(&leased.token).await?;
            }
            Err(repo_err) => {
                log::warn!(
                    "run worker: failed to record engine failure for run {}: {repo_err} \
                     (original error: {error_text})",
                    run.run_id
                );
                self.queue
                    .nack(&leased.token, Duration::from_secs(1))
                    .await?;
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    use paladin_core::platform::container::battlefield::{Battlefield, BattlefieldSchema};
    use paladin_core::platform::container::waypoint::{
        FrontierSnapshot, GraphFingerprint, ThreadId, WaypointStatus,
    };
    use paladin_ports::output::run_queue_port::{QueueError, QueuedRun};

    fn sample_waypoint() -> Waypoint {
        let battlefield =
            Battlefield::initialize(BattlefieldSchema::new(vec![]), &StateDelta::new()).unwrap();
        Waypoint::new_root(
            ThreadId::new("t1").unwrap(),
            1,
            GraphFingerprint::from_canonical_bytes(b"g"),
            battlefield,
            vec![],
            vec![],
            WaypointStatus::Running,
            BTreeMap::new(),
            FrontierSnapshot::default(),
        )
    }

    fn sample_response() -> ParleyResponse {
        ParleyResponse {
            parley_id: paladin_core::platform::container::parley::ParleyId::new(),
            kind: paladin_core::platform::container::parley::ParleyKind::Approval,
            prompt: "proceed?".to_string(),
            value: serde_json::json!(true),
            responded_by: Some("tester".to_string()),
            responded_at: chrono::Utc::now(),
            defaulted: false,
        }
    }

    // --- WorkerDispatch::decide -------------------------------------

    #[test]
    fn decide_returns_start_when_no_waypoint_exists() {
        assert_eq!(
            WorkerDispatch::decide(None, &[], None),
            WorkerDispatch::Start
        );
    }

    #[test]
    fn decide_returns_resume_when_waypoint_exists_and_no_pending_responses() {
        let waypoint = sample_waypoint();
        assert_eq!(
            WorkerDispatch::decide(Some(&waypoint), &[], None),
            WorkerDispatch::Resume
        );
    }

    #[test]
    fn decide_returns_resume_with_when_waypoint_exists_and_responses_are_pending() {
        let waypoint = sample_waypoint();
        let responses = vec![sample_response()];
        assert_eq!(
            WorkerDispatch::decide(Some(&waypoint), &responses, None),
            WorkerDispatch::ResumeWith(responses)
        );
    }

    // --- WorkerDispatch::decide (D-45, fork) -----------------------------

    fn sample_fork_spec(from: &WaypointId) -> ForkSpec {
        ForkSpec {
            from_waypoint_id: from.to_string(),
            edit: None,
        }
    }

    #[test]
    fn decide_returns_fork_when_no_waypoint_exists_yet_and_fork_from_is_set() {
        let from = WaypointId::generate();
        let fork = sample_fork_spec(&from);
        assert_eq!(
            WorkerDispatch::decide(None, &[], Some(&fork)),
            WorkerDispatch::Fork {
                from: from.to_string(),
                edit: None,
            }
        );
    }

    #[test]
    fn decide_returns_fork_when_latest_waypoint_fork_of_does_not_match() {
        let from = WaypointId::generate();
        let fork = sample_fork_spec(&from);
        let mut waypoint = sample_waypoint();
        waypoint.fork_of = None; // the mainline Waypoint being forked FROM
        assert_eq!(
            WorkerDispatch::decide(Some(&waypoint), &[], Some(&fork)),
            WorkerDispatch::Fork {
                from: from.to_string(),
                edit: None,
            }
        );
    }

    #[test]
    fn decide_falls_through_to_resume_once_fork_of_already_matches() {
        let from = WaypointId::generate();
        let fork = sample_fork_spec(&from);
        let mut waypoint = sample_waypoint();
        waypoint.fork_of = Some(from); // the fork Waypoint itself already exists
        assert_eq!(
            WorkerDispatch::decide(Some(&waypoint), &[], Some(&fork)),
            WorkerDispatch::Resume
        );
    }

    #[test]
    fn parse_fork_waypoint_id_round_trips_a_valid_id() {
        let id = WaypointId::generate();
        assert_eq!(parse_fork_waypoint_id(&id.to_string()), Some(id));
    }

    #[test]
    fn parse_fork_waypoint_id_rejects_garbage() {
        assert_eq!(parse_fork_waypoint_id("not-a-uuid"), None);
    }

    #[test]
    fn fork_edit_to_state_delta_merges_object_fields() {
        let edit = serde_json::json!({ "field_a": 1, "field_b": "x" });
        let delta = fork_edit_to_state_delta(Some(edit));
        assert_eq!(delta.values.len(), 2);
    }

    #[test]
    fn fork_edit_to_state_delta_is_empty_for_none() {
        let delta = fork_edit_to_state_delta(None);
        assert!(delta.values.is_empty());
    }

    // --- map_outcome ---------------------------------------------------

    fn sample_battlefield() -> Battlefield {
        Battlefield::initialize(BattlefieldSchema::new(vec![]), &StateDelta::new()).unwrap()
    }

    #[test]
    fn map_outcome_completed_transitions_to_completed_and_acks() {
        let waypoint = paladin_core::platform::container::waypoint::WaypointId::generate();
        let outcome = RunOutcome::Completed {
            final_state: sample_battlefield(),
            waypoint,
        };
        let action = map_outcome(&outcome, false, false);
        assert_eq!(
            action,
            OutcomeAction::Transition {
                to: RunStatus::Completed,
                outcome: RunOutcomeRecord {
                    error: None,
                    output: None,
                    final_waypoint_id: Some(waypoint.to_string()),
                },
            }
        );
    }

    #[test]
    fn map_outcome_failed_transitions_to_failed_with_error_text() {
        let error = EngineError::RecursionLimitExceeded {
            limit: 1,
            thread_id: ThreadId::new("t1").unwrap(),
        };
        let expected_text = error.to_string();
        let outcome = RunOutcome::Failed {
            error,
            waypoint: None,
        };
        let action = map_outcome(&outcome, false, false);
        assert_eq!(
            action,
            OutcomeAction::Transition {
                to: RunStatus::Failed,
                outcome: RunOutcomeRecord {
                    error: Some(expected_text),
                    output: None,
                    final_waypoint_id: None,
                },
            }
        );
    }

    #[test]
    fn map_outcome_awaiting_input_transitions_and_acks() {
        let waypoint = paladin_core::platform::container::waypoint::WaypointId::generate();
        let outcome = RunOutcome::AwaitingInput {
            parleys: vec![],
            waypoint,
        };
        let action = map_outcome(&outcome, false, false);
        assert_eq!(
            action,
            OutcomeAction::Transition {
                to: RunStatus::AwaitingInput,
                outcome: RunOutcomeRecord {
                    error: None,
                    output: None,
                    final_waypoint_id: Some(waypoint.to_string()),
                },
            }
        );
    }

    #[test]
    fn map_outcome_halted_with_cancel_requested_transitions_to_cancelled() {
        let waypoint = paladin_core::platform::container::waypoint::WaypointId::generate();
        let outcome = RunOutcome::Halted { waypoint };
        let action = map_outcome(&outcome, true, false);
        assert_eq!(
            action,
            OutcomeAction::Transition {
                to: RunStatus::Cancelled,
                outcome: RunOutcomeRecord {
                    error: None,
                    output: None,
                    final_waypoint_id: Some(waypoint.to_string()),
                },
            }
        );
    }

    #[test]
    fn map_outcome_halted_while_shutting_down_leaves_running_and_requeues() {
        let waypoint = paladin_core::platform::container::waypoint::WaypointId::generate();
        let outcome = RunOutcome::Halted { waypoint };
        let action = map_outcome(&outcome, false, true);
        assert_eq!(action, OutcomeAction::LeaveRunningAndRequeue);
    }

    #[test]
    fn map_outcome_halted_otherwise_transitions_to_halted() {
        let waypoint = paladin_core::platform::container::waypoint::WaypointId::generate();
        let outcome = RunOutcome::Halted { waypoint };
        let action = map_outcome(&outcome, false, false);
        assert_eq!(
            action,
            OutcomeAction::Transition {
                to: RunStatus::Halted,
                outcome: RunOutcomeRecord {
                    error: None,
                    output: None,
                    final_waypoint_id: Some(waypoint.to_string()),
                },
            }
        );
    }

    // --- webhook_delivery_for_outcome / run_status_to_event_kind ---------

    fn sample_run_with_webhook(events: Vec<RunEventKind>) -> Run {
        use paladin_core::platform::container::run::{AssistantRef, WebhookSpec};
        Run::new(
            RunId::new_v7(),
            ThreadId::new("wh-t1").unwrap(),
            AssistantRef {
                assistant_id: "a1".to_string(),
                version: 1,
            },
            serde_json::json!({}),
        )
        .with_webhook(WebhookSpec {
            url: "https://example.com/hook".to_string(),
            secret: None,
            events,
        })
    }

    #[test]
    fn run_status_to_event_kind_maps_terminal_and_awaiting_input() {
        assert_eq!(
            run_status_to_event_kind(RunStatus::AwaitingInput),
            Some(RunEventKind::AwaitingInput)
        );
        assert_eq!(
            run_status_to_event_kind(RunStatus::Completed),
            Some(RunEventKind::Completed)
        );
        assert_eq!(
            run_status_to_event_kind(RunStatus::Failed),
            Some(RunEventKind::Failed)
        );
        assert_eq!(
            run_status_to_event_kind(RunStatus::Halted),
            Some(RunEventKind::Halted)
        );
        assert_eq!(
            run_status_to_event_kind(RunStatus::Cancelled),
            Some(RunEventKind::Cancelled)
        );
        assert_eq!(run_status_to_event_kind(RunStatus::Queued), None);
        assert_eq!(run_status_to_event_kind(RunStatus::Running), None);
    }

    #[test]
    fn webhook_delivery_for_outcome_none_when_run_has_no_webhook() {
        let run = Run::new(
            RunId::new_v7(),
            ThreadId::new("no-hook").unwrap(),
            paladin_core::platform::container::run::AssistantRef {
                assistant_id: "a1".to_string(),
                version: 1,
            },
            serde_json::json!({}),
        );
        let delivery = webhook_delivery_for_outcome(
            &run,
            RunEventKind::Completed,
            RunStatus::Completed,
            None,
            chrono::Utc::now(),
        );
        assert!(delivery.is_none());
    }

    #[test]
    fn webhook_delivery_for_outcome_none_when_event_not_subscribed() {
        let run = sample_run_with_webhook(vec![RunEventKind::Failed]);
        let delivery = webhook_delivery_for_outcome(
            &run,
            RunEventKind::Completed,
            RunStatus::Completed,
            None,
            chrono::Utc::now(),
        );
        assert!(delivery.is_none());
    }

    #[test]
    fn webhook_delivery_for_outcome_builds_pending_delivery_when_subscribed() {
        let run = sample_run_with_webhook(vec![RunEventKind::Completed]);
        let delivery = webhook_delivery_for_outcome(
            &run,
            RunEventKind::Completed,
            RunStatus::Completed,
            None,
            chrono::Utc::now(),
        )
        .unwrap();
        assert_eq!(delivery.run_id, run.run_id);
        assert_eq!(delivery.url, "https://example.com/hook");
        assert!(matches!(
            delivery.status,
            paladin_core::platform::container::webhook::WebhookDeliveryStatus::Pending
        ));
        assert!(delivery.payload.contains(run.run_id.as_str()));
        assert!(!delivery.payload.to_lowercase().contains("secret"));
    }

    // --- LeaseHeartbeat --------------------------------------------------

    #[derive(Default)]
    struct RecordingQueue {
        calls: Mutex<Vec<tokio::time::Instant>>,
    }

    #[async_trait::async_trait]
    impl RunQueuePort for RecordingQueue {
        async fn enqueue(&self, _run: QueuedRun) -> Result<(), QueueError> {
            Ok(())
        }

        async fn dequeue(&self, _lease: Duration) -> Result<Option<LeasedRun>, QueueError> {
            Ok(None)
        }

        async fn extend_lease(
            &self,
            _token: &LeaseToken,
            _lease: Duration,
        ) -> Result<(), QueueError> {
            self.calls.lock().unwrap().push(tokio::time::Instant::now());
            Ok(())
        }

        async fn ack(&self, _token: &LeaseToken) -> Result<(), QueueError> {
            Ok(())
        }

        async fn nack(
            &self,
            _token: &LeaseToken,
            _requeue_delay: Duration,
        ) -> Result<(), QueueError> {
            Ok(())
        }

        async fn depth(&self) -> Result<u64, QueueError> {
            Ok(0)
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn lease_heartbeat_extends_at_lease_over_four_and_stops_after_drop() {
        let queue = Arc::new(RecordingQueue::default());
        let queue_port: Arc<dyn RunQueuePort> = queue.clone();
        let token = LeaseToken::new("t1");
        let lease = Duration::from_millis(400);

        let heartbeat = LeaseHeartbeat::spawn(queue_port, token, lease);
        tokio::time::sleep(Duration::from_secs(1)).await;
        drop(heartbeat);
        let count_at_drop = queue.calls.lock().unwrap().len();
        assert!(
            count_at_drop >= 8,
            "expected at least 8 heartbeats over a 1s run with a 400ms lease, got \
             {count_at_drop}"
        );

        // Never fires again once dropped -- D-10's "stops the moment the
        // run returns".
        tokio::time::sleep(Duration::from_millis(300)).await;
        let count_after = queue.calls.lock().unwrap().len();
        assert_eq!(
            count_after, count_at_drop,
            "heartbeat must stop extending once dropped"
        );
    }
}
