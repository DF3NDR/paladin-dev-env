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
use paladin_battalion::engine::{
    EngineError, HaltCause, NodeSpec, RunOutcome, TraceDispatcher, WarEngine, WarGraph,
};
use paladin_core::platform::container::allowance::HaltReason;
use paladin_core::platform::container::battlefield::{FieldName, StateDelta};
use paladin_core::platform::container::heartbeat::HeartbeatHandle;
use paladin_core::platform::container::herald::Herald;
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::parley::{ParleyRequest, ParleyResponse};
use paladin_core::platform::container::run::{
    ForkSpec, Run, RunEventKind, RunId, RunStatus, RunStreamEventKind, RunStreamMode,
};
use paladin_core::platform::container::run_scope::RunScope;
use paladin_core::platform::container::token_usage::TokenUsage;
use paladin_core::platform::container::trace::{RunFinishStatus, TraceEvent};
use paladin_core::platform::container::treasury_ledger::{LedgerScope, SettlementContext};
use paladin_core::platform::container::waypoint::{NodeId, NodeOutcomeKind, Waypoint, WaypointId};
use paladin_core::platform::container::webhook::{WebhookDelivery, WebhookDeliveryId};
use paladin_ports::output::cancellation_probe::CancellationProbe;
use paladin_ports::output::paladin_port::PaladinPort;
use paladin_ports::output::run_queue_port::{LeaseToken, LeasedRun, RunQueuePort};
use paladin_ports::output::run_repository_port::{
    RunOutcomeRecord, RunRepositoryError, RunRepositoryPort,
};
use paladin_ports::output::run_trace_port::RunTracePort;
use paladin_ports::output::trace_sink_port::{
    CompositeSink, RUN_TRACE_EMITTER, TraceEmitter, TraceSink,
};
use paladin_ports::output::treasury_ledger_port::TreasuryLedgerPort;
use paladin_ports::output::treasury_notice_port::TreasuryNoticePort;
use paladin_ports::output::waypoint_port::{WaypointError, WaypointPort};
use paladin_ports::output::webhook_delivery_port::WebhookDeliveryRepositoryPort;

use crate::application::services::treasurer::Treasurer;
use crate::config::trace::TraceConfig;
use crate::infrastructure::telemetry::{HeraldTraceSink, build_run_sink};

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

/// Runs `fut` inside a [`RUN_TRACE_EMITTER`] scope when `emitter` is
/// `Some`, so every nested `.await` inside `fut` -- including a call
/// through the deeply shared `Arc<dyn PaladinPort>` singleton
/// (`FallbackLlmAdapter`, the middleware chain, `PaladinExecutionService`)
/// -- can read back the SAME per-run handle via
/// `paladin_ports::output::trace_sink_port::current_trace_emitter` (28-06,
/// D-03). Runs `fut` unscoped when `emitter` is `None` (no sink configured
/// for this run -- the untraced path, D-10).
async fn with_run_trace_scope<F: std::future::Future>(
    emitter: &Option<Arc<dyn TraceEmitter>>,
    fut: F,
) -> F::Output {
    match emitter {
        Some(emitter) => RUN_TRACE_EMITTER.scope(Arc::clone(emitter), fut).await,
        None => fut.await,
    }
}

/// This run's model label for [`HeraldTraceSink`] (D-12, the `model_used`
/// discretion item): the distinct `paladin.node.model` values of every
/// [`NodeSpec::Paladin`] node in `graph.node_order()` order -- `"none"` when
/// the graph has no Paladin node, the bare model name when every Paladin
/// node declares the SAME model, `"mixed"` when they declare more than one
/// distinct model. The per-call [`Cost`](paladin_core::platform::container::cost::Cost)
/// already prices each model correctly regardless of this label; it exists
/// only to give the produced [`ExecutionMetadata`](paladin_core::platform::container::herald::ExecutionMetadata)'s
/// `model_used` field a sensible value for a mixed-model graph.
fn run_model_label(graph: &WarGraph) -> String {
    let mut models: Vec<&str> = Vec::new();
    for node_id in graph.node_order() {
        if let Some(NodeSpec::Paladin { paladin, .. }) = graph.node(node_id) {
            let model = paladin.node.model.as_str();
            if !models.contains(&model) {
                models.push(model);
            }
        }
    }
    match models.as_slice() {
        [] => "none".to_string(),
        [only] => only.to_string(),
        _ => "mixed".to_string(),
    }
}

/// The `graph_fingerprint` an agent run's `TraceEvent::RunStarted` carries.
/// A legacy `Runnable::Agent` run has no `WarGraph`, hence no fingerprint;
/// this sentinel stands in. Every sink accepts any string here -- the
/// bus sink maps `RunStarted` to no wire event at all (D-24), and the
/// persisting sink stores it verbatim.
const AGENT_RUN_FINGERPRINT: &str = "agent";

/// This agent run's model label for [`HeraldTraceSink`] (D-12, the
/// `model_used` discretion item): the paladin's own model string, or
/// `"none"` when it is empty -- the agent-path twin of [`run_model_label`].
fn agent_model_label(paladin: &Paladin) -> String {
    let model = paladin.node.model.as_str();
    if model.is_empty() {
        "none".to_string()
    } else {
        model.to_string()
    }
}

/// Compose a run's `TraceSink` from `build_run_sink`'s own output and the
/// optional [`HeraldTraceSink`]: neither -> `None` (the untraced path), one
/// -> that sink alone, both -> a panic-isolated [`CompositeSink`]. Shared by
/// the graph path ([`RunWorkerPool::run_once`]) and the agent path
/// ([`RunWorkerPool::run_agent`]) so both assemble sinks identically.
fn compose_run_sink(
    base: Option<Arc<dyn TraceSink>>,
    herald: Option<Arc<dyn TraceSink>>,
) -> Option<Arc<dyn TraceSink>> {
    match (base, herald) {
        (None, None) => None,
        (Some(sink), None) | (None, Some(sink)) => Some(sink),
        (Some(base), Some(herald)) => {
            Some(Arc::new(CompositeSink::new(vec![base, herald])) as Arc<dyn TraceSink>)
        }
    }
}

/// Elapsed wall-clock milliseconds since `started`, saturating rather than
/// panicking on an (unreachable in practice) overflow.
fn elapsed_ms(started: std::time::Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// The per-run engine [`RunWorkerPool::run_once`] dispatches through, the
/// local-token registration (if any) it must clean up when the dispatch
/// finishes, and the trace emitter (if a sink was configured) below-engine
/// producers can read via `RUN_TRACE_EMITTER` for the duration of this
/// dispatch (28-06). Factored into a named alias purely to keep the
/// three-tuple readable at its one call site.
type RunDispatchEngine<W> = (
    Arc<WarEngine<W>>,
    Option<RunId>,
    Option<Arc<dyn TraceEmitter>>,
);

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
    halt_reason: Option<&HaltReason>,
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
        // D-19: the typed reason rides only a `halted` event, through the one wire builder, so
        // it is byte-identical to `GET /runs` and the 429 details. Every other event omits the
        // key entirely (its payload bytes are unchanged).
        halt_reason: halt_reason
            .filter(|_| status == RunStatus::Halted)
            .map(HaltReason::wire_json),
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
                halt_reason: None,
            },
        },
        RunOutcome::Failed { error, waypoint } => OutcomeAction::Transition {
            to: RunStatus::Failed,
            outcome: RunOutcomeRecord {
                error: Some(error.to_string()),
                output: None,
                final_waypoint_id: waypoint.map(|w| w.to_string()),
                halt_reason: None,
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
                halt_reason: None,
            },
        },
        // D-05 (Phase 42): the engine's typed cause decides how a halt is recorded.
        // A spend halt and a probe-observed cancel are decided by the cause alone; only the
        // residual in-process token halt still consults the side flags (defence in depth, G1a).
        RunOutcome::Halted {
            waypoint,
            cause: HaltCause::Spend(reason),
        } => OutcomeAction::Transition {
            // ALLOW-03: an exhausted allowance halts the run at its next boundary. The run is
            // recorded Halted with NO error -- a halt is a resume point, not a failure -- whether
            // or not the pool is draining or a cancel flag is set (a drain must not requeue a run
            // the Treasurer has already stopped). The typed reason is recorded on the run row
            // (D-06) so `GET /runs`, the webhook and the SSE fallback can say why it halted.
            to: RunStatus::Halted,
            outcome: RunOutcomeRecord {
                error: None,
                output: None,
                final_waypoint_id: Some(waypoint.to_string()),
                halt_reason: Some(reason.clone()),
            },
        },
        RunOutcome::Halted {
            waypoint,
            cause: HaltCause::CancelRequested,
        } => OutcomeAction::Transition {
            // The durable cancel probe observed the caller's request: the *waypoint* halted,
            // the *run* is recorded Cancelled (D-16).
            to: RunStatus::Cancelled,
            outcome: RunOutcomeRecord {
                error: None,
                output: None,
                final_waypoint_id: Some(waypoint.to_string()),
                halt_reason: None,
            },
        },
        // `HaltCause::Token` and any future cause: today's three-way logic, unchanged.
        RunOutcome::Halted { waypoint, .. } => {
            if cancel_requested {
                // D-16: the caller asked for this. The *waypoint* halted;
                // the *run* is recorded Cancelled.
                OutcomeAction::Transition {
                    to: RunStatus::Cancelled,
                    outcome: RunOutcomeRecord {
                        error: None,
                        output: None,
                        final_waypoint_id: Some(waypoint.to_string()),
                        halt_reason: None,
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
                        halt_reason: None,
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
    /// [`RunWorkerPool::with_event_bus`] wires one: both runnable kinds
    /// `bind` the dispatch's thread/run before the call, publish live through
    /// [`RunEventBusSink`] -> `map_trace_event` alone (D-14), wait
    /// [`TRACE_DRAIN_GRACE_PERIOD`], then `unbind`. `run_once` attaches the
    /// sink to a per-run `engine_factory` engine so all seven wire events
    /// bridge from the engine's own records; [`Self::run_agent`] has no
    /// engine, so it emits its own `RunStarted`/`NodeStarted`/`NodeFinished`/
    /// `RunFinished` records into a standalone per-run `TraceDispatcher`
    /// feeding the same sink (PLAT-08, ledger row 31). `None` (the default)
    /// preserves every prior plan's behavior verbatim -- no
    /// bind/publish/unbind call happens anywhere.
    event_bus: Option<Arc<RunEventBus>>,
    /// The D-40 durable delivery queue, when
    /// [`RunWorkerPool::with_webhook_deliveries`] wires one: both runnable
    /// kinds enqueue a `Pending` [`WebhookDelivery`] on every
    /// terminal/suspension transition whose run subscribes to that event
    /// (PLAT-FR-14), through the one `webhook_delivery_for_outcome` helper.
    /// The graph path and the agent's success path enqueue after the status
    /// write and ack; a failure -- an agent run's, or a graph run's
    /// engine-error -- enqueues through `persist_failure` (D-15). `None` (the
    /// default) preserves every prior plan's behavior verbatim -- no enqueue
    /// call happens anywhere. A repository error here is logged and NEVER
    /// changes the run's own status (prohibition P2): it is enqueued strictly
    /// after the run's own status write/ack has already succeeded.
    webhook_deliveries: Option<Arc<dyn WebhookDeliveryRepositoryPort>>,
    /// The trace pipeline configuration (OBS-02, D-11), wired via
    /// [`RunWorkerPool::with_trace_config`]. Defaults to
    /// [`TraceConfig::default`] (`log_sink: true`) -- a pool that never
    /// calls the builder still gets the default-on log sink for every run
    /// dispatched through [`Self::engine_factory`], and for every legacy
    /// `Runnable::Agent` run (which needs no engine to host its standalone
    /// dispatcher). Has no effect on a graph run's shared-engine ("no
    /// factory") path: per-run trace composition needs a per-run engine to
    /// attach to.
    trace_config: TraceConfig,
    /// The durable trace-persistence backend (OBS-02, D-17), wired via
    /// [`RunWorkerPool::with_run_trace_port`]: `run_once` passes this to
    /// [`crate::infrastructure::telemetry::build_run_sink`] so a
    /// [`PersistingTraceSink`](crate::infrastructure::telemetry::PersistingTraceSink)
    /// joins the per-run composite whenever `trace_config.persist` is also
    /// set. `None` (the default) means `build_run_sink` never attaches one,
    /// regardless of `trace_config.persist` -- matching that function's own
    /// documented "no port available is a no-op, not an error" contract.
    run_trace_port: Option<Arc<dyn RunTracePort>>,
    /// The operator-configured [`Herald`] (D-12, D-11b), wired via
    /// [`RunWorkerPool::with_herald`]: `run_once` composes a
    /// [`HeraldTraceSink`] labeled with [`run_model_label`] alongside
    /// whatever [`crate::infrastructure::telemetry::build_run_sink`] itself
    /// produces, so every run this pool dispatches through
    /// [`Self::engine_factory`] hands its `RunFinished` event to this
    /// herald. `None` (the default) means no herald is attached and the
    /// untraced-for-cost path stays exactly as every prior plan left it --
    /// matching `trace_config`/`run_trace_port`'s own documented "a pool
    /// that never calls the builder is unaffected" contract. Has no effect
    /// on the shared-engine ("no factory") path, exactly like
    /// `trace_config`/`run_trace_port` above.
    herald: Option<Arc<dyn Herald>>,
    /// The Treasurer's durable spend ledger (D-08, D-07, 39-07), wired via
    /// [`RunWorkerPool::with_treasury_ledger`]: attaches to every per-run
    /// engine [`Self::engine_factory`] builds, via
    /// [`WarEngine::with_treasury_ledger`] with a [`SettlementContext`]
    /// whose `scope` is the run row's recorded submitter -- its tenant and
    /// API key id via [`LedgerScope::from_attribution`] (Phase 40 D-15), or
    /// the [`LedgerScope::unattributed`] sentinel when `Run.submitted_by`
    /// is `None` (a schedule-fired or internal run, D-10) -- `run_id` is
    /// this dispatch's own `Run.run_id`, and `attempt` is the persisted
    /// counter this same
    /// `run_once` call just computed -- `run.attempt` on a first dispatch,
    /// or [`RunRepositoryPort::bump_attempt`]'s returned value on a
    /// `Running` redelivery (D-07): the engine never invents its own
    /// counter. The shared no-factory engine gets none, exactly like the
    /// trace sinks above. Settlements written through this attachment are
    /// observational only -- a ledger failure is logged and never affects
    /// this run's status, ack or retry (D-08). An agent-kind run does not
    /// settle here at all: [`Self::run_agent`] instead carries this run's
    /// id and the same submitter-derived scope into the run engine's shared
    /// [`PaladinPort`] via [`RunScope::with_run_id`] and
    /// [`RunScope::with_ledger_scope`] (D-16), so the agent loop's own
    /// `AgentLoopSettlement::PlatformRunsOnly` writer (39-05) settles it
    /// under attempt `1` and the same tenant/API key id, never
    /// double-charging a redelivered agent-kind run. `None` (the default)
    /// means no ledger call happens anywhere in
    /// this pool, matching every other optional field's own "a pool that
    /// never calls the builder is unaffected" contract.
    treasury_ledger: Option<Arc<dyn TreasuryLedgerPort>>,
    /// The Treasurer's durable once-per-window notice store (Phase 41 D-18, C6), wired via
    /// [`RunWorkerPool::with_treasury_notices`]: on a run's FIRST dispatch (`Queued`) the pool
    /// reads the notices the admission won for this run id and emits one
    /// [`TraceEvent::AllowanceWarning`] per row through the run's own dispatcher, before
    /// `RunStarted`. `None` (the default) means no notice is ever read.
    treasury_notices: Option<Arc<dyn TreasuryNoticePort>>,
    /// The Treasurer facade service (ALLOW-03, Phase 42 D-04), wired via
    /// [`RunWorkerPool::with_treasurer`]: on the [`Self::engine_factory`] path, every per-run
    /// engine for a run whose row records a submitter gets a per-run
    /// [`Treasurer::spend_guard`] attached via [`WarEngine::with_spend_guard`], so an allowance
    /// exhausted mid-run halts the run at its next superstep boundary. `None` (the default)
    /// attaches no guard and makes no ledger read, matching every other optional field's "a
    /// pool that never calls the builder is unaffected" contract.
    treasurer: Option<Arc<Treasurer>>,
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
            trace_config: TraceConfig::default(),
            run_trace_port: None,
            herald: None,
            treasury_ledger: None,
            treasury_notices: None,
            treasurer: None,
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
    /// the dispatch's thread/run before driving the engine and attaches a
    /// fresh [`RunEventBusSink`] to whatever per-run engine
    /// [`Self::with_engine_factory`] produces (mirroring how
    /// [`Self::with_cancellation_probing`] attaches its own probe). D-14:
    /// this pool no longer publishes `parley`/`done`/`error` directly --
    /// every wire event, including those three, reaches the bus through
    /// [`RunEventBusSink`]/[`super::events::map_trace_event`] alone, now
    /// that the engine's own `ParleyRaised`/`RunFinished` records carry
    /// enough information to produce them (the ONE exception,
    /// `record_engine_failure`'s own retained publish, is
    /// documented at that method). Has no effect on the shared-engine ("no
    /// factory") path's own trace bridging -- attach `bus`'s own
    /// [`RunEventBusSink`] to that engine directly at construction (mirrors
    /// [`Self::with_cancellation_probing`]'s own documented limitation);
    /// this pool still binds/unbinds regardless, since those do not depend
    /// on which engine instance is used. A legacy `Runnable::Agent` run has
    /// no engine at all: `run_agent` attaches its own
    /// [`RunEventBusSink`] to a standalone per-run dispatcher (PLAT-08).
    pub fn with_event_bus(mut self, bus: Arc<RunEventBus>) -> Self {
        self.event_bus = Some(bus);
        self
    }

    /// Wire the trace pipeline configuration (OBS-02, D-11): `run_once`
    /// reads `trace.log_sink`/`channel_capacity` from `config` to build each
    /// per-run `CompositeSink`/`TraceDispatcher` via
    /// [`crate::infrastructure::telemetry::build_run_sink`]. Only takes
    /// effect on the [`Self::with_engine_factory`] path -- see that field's
    /// own doc comment -- but `run_agent` applies it to every legacy agent run.
    pub fn with_trace_config(mut self, config: TraceConfig) -> Self {
        self.trace_config = config;
        self
    }

    /// Wire the durable trace-persistence backend (OBS-02, D-17): `run_once`
    /// passes `port` to [`crate::infrastructure::telemetry::build_run_sink`]
    /// so a
    /// [`PersistingTraceSink`](crate::infrastructure::telemetry::PersistingTraceSink)
    /// joins the per-run composite whenever [`Self::with_trace_config`]'s
    /// `persist` flag is also set. Only takes effect on the
    /// [`Self::with_engine_factory`] path, exactly like
    /// [`Self::with_trace_config`] itself (`run_agent` applies it to every
    /// agent run regardless). A pool that never calls this
    /// builder leaves `trace_config.persist` a no-op, matching
    /// `build_run_sink`'s own "no port available" contract.
    pub fn with_run_trace_port(mut self, port: Arc<dyn RunTracePort>) -> Self {
        self.run_trace_port = Some(port);
        self
    }

    /// Wire an operator-configured [`Herald`] (D-12, D-11b): `run_once` composes a
    /// [`HeraldTraceSink`] labeled with this run's `run_model_label` alongside whatever
    /// [`crate::infrastructure::telemetry::build_run_sink`] itself produces, so every run
    /// dispatched through the per-run `engine_factory` hands its `RunFinished` event to
    /// `herald`. Only takes effect on the [`Self::with_engine_factory`] path, exactly like
    /// [`Self::with_trace_config`]/[`Self::with_run_trace_port`]; `run_agent` composes a
    /// herald sink labeled by `agent_model_label` for every agent run. A pool that never
    /// calls this builder attaches no herald -- the untraced-for-cost path stays zero-cost.
    pub fn with_herald(mut self, herald: Arc<dyn Herald>) -> Self {
        self.herald = Some(herald);
        self
    }

    /// Attach the Treasurer's durable spend ledger (D-08, D-07, 39-07):
    /// `run_once` attaches `ledger` to every per-run engine
    /// [`Self::with_engine_factory`] produces, via
    /// [`WarEngine::with_treasury_ledger`] with a [`SettlementContext`]
    /// scoped to the run row's recorded submitter via
    /// [`LedgerScope::from_attribution`] (Phase 40 D-15; the
    /// [`LedgerScope::unattributed`] sentinel only when the row records no
    /// principal, D-10) and keyed by this dispatch's
    /// own `run_id` and persisted `attempt` (D-07) -- the same persisted
    /// counter [`RunRepositoryPort::bump_attempt`]/`record_resume` already
    /// bump on redelivery/resume, so a genuine re-execution settles under a
    /// fresh key while a repeated settle of an already-settled key is
    /// charged once (LEDGR-03, ADR-0053 §4). Also carries this run's id
    /// and the same submitter-derived scope into the run engine's shared
    /// [`PaladinPort`] for an agent-kind (`Runnable::Agent`) run via
    /// [`RunScope::with_run_id`] and [`RunScope::with_ledger_scope`] in
    /// `Self::run_agent` (D-16), so that port's own `AgentLoopSettlement::
    /// PlatformRunsOnly` writer (39-05) settles it under the Platform run
    /// id and the submitting principal's tenant and API key id rather than
    /// a fresh execution id and the sentinel every dispatch. Only takes
    /// effect on the [`Self::with_engine_factory`] path -- the shared
    /// no-factory engine is never attached to, exactly like
    /// [`Self::with_trace_config`]/[`Self::with_herald`]. A pool that never
    /// calls this builder performs no ledger call at all (D-08: the ledger
    /// is not installed when no backend is configured), and every
    /// pre-existing worker test's behavior is unchanged.
    pub fn with_treasury_ledger(mut self, ledger: Arc<dyn TreasuryLedgerPort>) -> Self {
        self.treasury_ledger = Some(ledger);
        self
    }

    /// Attach the Treasurer's durable notice store (Phase 41 D-18, C6): on a run's first
    /// dispatch (`Queued` only -- never a `Running` redelivery or an `AwaitingInput` resume)
    /// the pool reads `notices_for_run(run_id)` and emits one
    /// [`TraceEvent::AllowanceWarning`] per row through that run's own trace dispatcher,
    /// immediately before `RunStarted`, for graph runs and agent-kind runs alike. Emitting from
    /// the worker, never from the submitting process, is what keeps the run's `seq` counter
    /// collision-free. A read failure is logged and never affects the run (D-15), and a run
    /// with no trace sink emits nothing. A pool that never calls this builder reads no notice.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::sync::Arc;
    /// use paladin::application::services::run::RunWorkerPool;
    /// use paladin_ports::output::treasury_notice_port::TreasuryNoticePort;
    /// use paladin_ports::output::waypoint_port::WaypointPort;
    ///
    /// fn attach<W: WaypointPort + 'static>(
    ///     pool: RunWorkerPool<W>,
    ///     notices: Arc<dyn TreasuryNoticePort>,
    /// ) -> RunWorkerPool<W> {
    ///     pool.with_treasury_notices(notices)
    /// }
    /// ```
    pub fn with_treasury_notices(mut self, notices: Arc<dyn TreasuryNoticePort>) -> Self {
        self.treasury_notices = Some(notices);
        self
    }

    /// Attach the [`Treasurer`] (ALLOW-03, Phase 42 D-04): `run_once` attaches a per-run
    /// [`Treasurer::spend_guard`] to every per-run engine [`Self::with_engine_factory`]
    /// produces, beside `with_cancellation_probe` and `with_treasury_ledger`, whenever the run
    /// row records a submitter. The engine then consults the guard once per superstep boundary
    /// and halts the run (status `halted`, no error, its last Waypoint kept as the restart
    /// point) when an allowance has been exhausted mid-run -- overshoot is at most one
    /// superstep's spend, never absolute.
    ///
    /// A run whose row records no submitter (`submitted_by: None`, a schedule-fired or internal
    /// run) gets no guard and no ledger read (D-00e). Only takes effect on the
    /// [`Self::with_engine_factory`] path, exactly like [`Self::with_treasury_ledger`]. A pool
    /// that never calls this builder is unaffected.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::sync::Arc;
    /// use paladin::application::services::run::RunWorkerPool;
    /// use paladin::application::services::treasurer::Treasurer;
    /// use paladin_ports::output::waypoint_port::WaypointPort;
    ///
    /// fn attach<W: WaypointPort + 'static>(
    ///     pool: RunWorkerPool<W>,
    ///     treasurer: Arc<Treasurer>,
    /// ) -> RunWorkerPool<W> {
    ///     pool.with_treasurer(treasurer)
    /// }
    /// ```
    pub fn with_treasurer(mut self, treasurer: Arc<Treasurer>) -> Self {
        self.treasurer = Some(treasurer);
        self
    }

    /// Emit one [`TraceEvent::AllowanceWarning`] per notice the admission won for `run`
    /// through `emitter`, in recorded order (D-18, C6). Called on a first dispatch only, after
    /// the run's own emitter exists and before `RunStarted`, so the events take the run's own
    /// lowest `seq` values. Silent when no notice store is wired; a read failure is logged
    /// (run id and error, never a figure) and the run proceeds (D-15, T-41-36).
    async fn emit_allowance_warnings(&self, run: &Run, emitter: &dyn TraceEmitter) {
        let Some(notices) = &self.treasury_notices else {
            return;
        };
        match notices.notices_for_run(&run.run_id).await {
            Ok(records) => {
                for record in records {
                    emitter.emit(TraceEvent::from(record.warning));
                }
            }
            Err(error) => {
                log::warn!(
                    "allowance notice read failed for run {}: {error}",
                    run.run_id
                );
            }
        }
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

        // D-18, C6: only a run's first dispatch (`Queued`) carries its admission's allowance
        // warnings; a `Running` redelivery or an `AwaitingInput` resume never re-emits them.
        let first_dispatch = matches!(run.status, RunStatus::Queued);

        // D-07: `attempt` is the persisted `Run.attempt` counter this
        // dispatch settles under (39-07) -- `run.attempt` on a first
        // dispatch or an `AwaitingInput` release, or `bump_attempt`'s own
        // returned value on a `Running` redelivery. The engine never
        // invents its own counter.
        let attempt = match run.status {
            RunStatus::Queued => {
                self.repository
                    .update_status(
                        &run.run_id,
                        RunStatus::Queued,
                        RunStatus::Running,
                        chrono::Utc::now(),
                    )
                    .await?;
                run.attempt
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
                run.attempt
            }
            RunStatus::Running => {
                // A redelivery: no status change, just the shared attempt
                // counter (D-23).
                self.repository.bump_attempt(&run.run_id).await?
            }
            _ => {
                // A stale message for an already-terminal run: ack and
                // drop, never touch the engine (D-07).
                self.queue.ack(&leased.token).await?;
                return Ok(true);
            }
        };

        let resolved = self
            .resolver
            .resolve(&run.assistant.assistant_id, Some(run.assistant.version))
            .await?;

        let graph = match resolved.runnable {
            Runnable::Workflow(graph) => graph,
            Runnable::Agent(paladin) => {
                return self.run_agent(&leased, &run, paladin, attempt).await;
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
        let (run_engine, local_token_guard, run_trace_emitter): RunDispatchEngine<W> =
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
                    if let Some(ledger) = &self.treasury_ledger {
                        engine = engine.with_treasury_ledger(
                            Arc::clone(ledger),
                            // 40-04 (D-15): the run row's recorded submitter is
                            // the scope; the sentinel only when none was recorded.
                            SettlementContext {
                                scope: LedgerScope::from_attribution(run.submitted_by.as_ref()),
                                run_id: run.run_id.clone(),
                                attempt,
                            },
                        );
                    }
                    // --- ALLOW-03, Phase 42 D-04, D-00e: the per-run spend guard,
                    // attached beside the probe and the ledger. Only a run whose
                    // row records a submitter gets one -- an unattributed run has
                    // no allowance identity, so it makes no guard and no ledger
                    // read. The guard takes the recorded identity (tenant and key
                    // NAME), never a role.
                    if let (Some(treasurer), Some(attribution)) =
                        (&self.treasurer, run.submitted_by.as_ref())
                    {
                        engine = engine.with_spend_guard(
                            treasurer.spend_guard(attribution.clone(), run.run_id.clone()),
                        );
                    }
                    // --- 28-06 (OBS-02, D-03, D-11): one CompositeSink, one
                    // TraceDispatcher, per run. `build_run_sink` is the single
                    // place a run's sink fan-out (the default-on log sink
                    // alongside the D-24 bus sink, when wired) is assembled.
                    // The dispatcher is built HERE, before the engine exists,
                    // because the SAME `Arc<dyn TraceEmitter>` handle must also
                    // reach the fallback adapter/middleware chain/execution
                    // service below the engine -- `with_bound_trace_dispatcher`
                    // then hands this exact instance to the engine too, so
                    // every record in this run comes from the ONE counter
                    // (D-03), never two independent dispatchers racing.
                    let bus_sink = self.event_bus.as_ref().map(|bus| {
                        Arc::new(RunEventBusSink::new(Arc::clone(bus))) as Arc<dyn TraceSink>
                    });
                    let base_sink =
                        build_run_sink(&self.trace_config, bus_sink, self.run_trace_port.clone());
                    // D-12: compose a HeraldTraceSink alongside build_run_sink's own
                    // output, only when this pool has an operator herald wired
                    // (RunWorkerPool::with_herald) -- a pool with no herald composes
                    // exactly as before, so the untraced-for-cost path stays zero-cost.
                    let herald_sink = self.herald.as_ref().map(|herald| {
                        Arc::new(HeraldTraceSink::new(
                            Arc::clone(herald),
                            run_model_label(&graph),
                        )) as Arc<dyn TraceSink>
                    });
                    let composed_sink = compose_run_sink(base_sink, herald_sink);
                    let run_trace_emitter = match composed_sink {
                        Some(sink) => {
                            let dispatcher = Arc::new(TraceDispatcher::with_capacity(
                                run.thread_id.clone(),
                                Some(run.run_id.clone()),
                                Some(sink.clone()),
                                self.trace_config.channel_capacity,
                            ));
                            engine = engine
                                .with_trace_sink(sink)
                                .with_trace_capacity(self.trace_config.channel_capacity)
                                .with_bound_trace_dispatcher(run.thread_id.clone(), dispatcher);
                            // `trace_emitter()` returns the SAME dispatcher
                            // `with_bound_trace_dispatcher` just bound
                            // (28-06's own `with_bound_trace`/
                            // `trace_emitter` doc comments): the canonical
                            // accessor, not a second cast of the local
                            // `dispatcher` variable, so this handle is
                            // provably the one `start`/`resume*` itself
                            // will use once dispatch begins below.
                            Some(engine.trace_emitter())
                        }
                        None => None,
                    };
                    (
                        Arc::new(engine),
                        Some(run.run_id.clone()),
                        run_trace_emitter,
                    )
                }
                None => (Arc::clone(&self.engine), None, None),
            };

        // D-24: bind THIS thread/run on the bus before dispatch, so a
        // `TraceSink` callback firing mid-superstep has somewhere to
        // publish to, and so a subscriber connecting right after this call
        // sees the live path rather than falling back to degraded.
        if let Some(bus) = &self.event_bus {
            bus.bind(run.thread_id.clone(), run.run_id.clone()).await;
        }

        // D-18: the admission's allowance warnings, on this run's own stream, before the
        // engine's own `RunStarted`. Needs the run's own emitter (a traced factory-built run).
        if first_dispatch && let Some(emitter) = run_trace_emitter.as_deref() {
            self.emit_allowance_warnings(&run, emitter).await;
        }

        let heartbeat = LeaseHeartbeat::spawn(self.queue.clone(), leased.token.clone(), self.lease);
        let outcome_result = match dispatch {
            WorkerDispatch::Start => {
                with_run_trace_scope(
                    &run_trace_emitter,
                    run_engine.start(&graph, run.thread_id.clone(), StateDelta::new()),
                )
                .await
            }
            WorkerDispatch::Resume => {
                with_run_trace_scope(
                    &run_trace_emitter,
                    run_engine.resume(&graph, run.thread_id.clone()),
                )
                .await
            }
            WorkerDispatch::ResumeWith(responses) => {
                let result = with_run_trace_scope(
                    &run_trace_emitter,
                    run_engine.resume_with(&graph, run.thread_id.clone(), responses),
                )
                .await;
                if result.is_ok() {
                    self.repository.clear_pending_responses(&run.run_id).await?;
                }
                result
            }
            WorkerDispatch::Fork { from, edit } => match parse_fork_waypoint_id(&from) {
                Some(waypoint_id) => {
                    let delta = fork_edit_to_state_delta(edit);
                    with_run_trace_scope(
                        &run_trace_emitter,
                        run_engine.fork(&graph, &run.thread_id, waypoint_id, delta),
                    )
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

        // D-14: this worker does not publish the terminal/suspension event itself. The engine's
        // own `ParleyRaised` and `RunFinished` (emitted at the end of every `start`/`resume`/
        // `resume_with`/`fork` call) already reached the bus through `RunEventBusSink`/
        // `map_trace_event`, inside the `with_run_trace_scope` call this dispatch just returned
        // from. Which status string the SSE `done` carries is owned by 42-05 (the halt reason)
        // and 42-06 (`cancelled`); this worker only guarantees below that a halting run's reason
        // is on the row BEFORE its status flips. `unbind` itself is deferred past the
        // repository/queue write below -- see the comment there for why.

        match map_outcome(&outcome, cancel_requested, shutting_down) {
            OutcomeAction::Transition {
                to,
                outcome: record,
            } => {
                // G14 / PLAT-09: for a halting transition the outcome (carrying the typed
                // reason and the fork-point Waypoint id) is written BEFORE the status flips, so
                // no reader -- the degraded SSE poller included -- can observe `halted` or
                // `cancelled` without them. `record_outcome` has no status guard on any adapter
                // (the contract clause `record_outcome_before_status_flip_is_accepted`), so the
                // reversed order is legal. Every other transition keeps its original order.
                let halt_reason = record.halt_reason.clone();
                if matches!(to, RunStatus::Halted | RunStatus::Cancelled) {
                    self.repository.record_outcome(&run.run_id, record).await?;
                    self.repository
                        .update_status(&run.run_id, RunStatus::Running, to, chrono::Utc::now())
                        .await?;
                } else {
                    self.repository
                        .update_status(&run.run_id, RunStatus::Running, to, chrono::Utc::now())
                        .await?;
                    self.repository.record_outcome(&run.run_id, record).await?;
                }
                self.queue.ack(&leased.token).await?;

                // D-40, PLAT-FR-14: enqueue a webhook delivery for this
                // transition, strictly AFTER the run's own status write and
                // ack have already succeeded -- a delivery-repository
                // failure here is logged and never rolls back or changes
                // the run's own status (prohibition P2).
                let parleys = match &outcome {
                    RunOutcome::AwaitingInput { parleys, .. } => Some(parleys.as_slice()),
                    _ => None,
                };
                self.enqueue_webhook_delivery(&run, to, parleys, halt_reason.as_ref())
                    .await;
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
    /// until a later plan always wires one) -- and a nacked run never binds
    /// the event bus.
    ///
    /// **(PLAT-08, D-14/D-15) Same machinery as a graph run.** No engine hosts
    /// this run, so it assembles the graph path's per-run trace pipeline
    /// itself: `build_run_sink` (+ the herald sink, composed by the same
    /// `compose_run_sink`) behind one standalone `TraceDispatcher`, the event
    /// bus bound before the call, and the `execute_scoped` call wrapped in
    /// `with_run_trace_scope` so the agent loop's own middleware, progress
    /// and fallback records reach the same dispatcher. Because no engine
    /// emits them, this method emits `RunStarted` (graph fingerprint
    /// [`AGENT_RUN_FINGERPRINT`]), one superstep-0 `NodeStarted`/
    /// `NodeFinished` pair for the agent call -- carrying its `PaladinResult`
    /// usage and cost, so the dispatcher's `total_usage()`/`total_cost()`
    /// are real -- and `RunFinished { total_supersteps: 0 }`. The live SSE
    /// `node_started`/`node_finished`/`done`|`error` events therefore come
    /// from the ONE `map_trace_event` mapping, never a hand-published event.
    ///
    /// **Exactly one terminal wire event.** A failure emits `RunFinished
    /// { Failed }`, which maps to the single `error` event with `message:
    /// null` -- the same shape a graph run's engine-emitted failure has.
    /// This method makes no direct bus publish of its own and does not go
    /// through `record_engine_failure` (whose own publish would be a second
    /// `error`); it calls `persist_failure` instead. The failure text stays
    /// readable through the tenant-scoped `GET /runs/{id}` `error` field,
    /// never on the wire, the trace or the webhook payload.
    ///
    /// **Webhooks.** Both return paths enqueue the run's subscribed
    /// `completed`/`failed` delivery strictly after the status write and ack
    /// (D-40), through `enqueue_webhook_delivery` -> `webhook_delivery_for_outcome`;
    /// an enqueue error is logged and never changes the run's status (P2).
    ///
    /// The `RunScope` (run id + ledger scope) and the `execute_scoped`
    /// arguments are unchanged from 39-07/40-04 (D-00g): nothing here changes
    /// what the Treasurer ledger settles for an agent run. The SSE `done`
    /// status for a cancelled or halted agent run is out of scope (PLAT-09).
    async fn run_agent(
        &self,
        leased: &LeasedRun,
        run: &Run,
        paladin: Arc<Paladin>,
        attempt: u32,
    ) -> Result<bool, WorkerError> {
        // A nacked run never binds: keep this early return FIRST.
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

        // 45-02 (D-14): the graph path's per-run trace assembly. No engine
        // hosts this run, so the dispatcher is a standalone one built from
        // the SAME sink composition `run_once` uses.
        let bus_sink = self
            .event_bus
            .as_ref()
            .map(|bus| Arc::new(RunEventBusSink::new(Arc::clone(bus))) as Arc<dyn TraceSink>);
        let base_sink = build_run_sink(&self.trace_config, bus_sink, self.run_trace_port.clone());
        let herald_sink = self.herald.as_ref().map(|herald| {
            Arc::new(HeraldTraceSink::new(
                Arc::clone(herald),
                agent_model_label(&paladin),
            )) as Arc<dyn TraceSink>
        });
        let composed_sink = compose_run_sink(base_sink, herald_sink);
        let traced = composed_sink.is_some();
        let dispatcher = Arc::new(TraceDispatcher::with_capacity(
            run.thread_id.clone(),
            Some(run.run_id.clone()),
            composed_sink,
            self.trace_config.channel_capacity,
        ));
        // `None` for an untraced run, so below-engine producers keep their
        // zero-cost "no emitter" path (D-10) exactly as on the graph path.
        let emitter: Option<Arc<dyn TraceEmitter>> =
            traced.then(|| Arc::clone(&dispatcher) as Arc<dyn TraceEmitter>);

        if let Some(bus) = &self.event_bus {
            bus.bind(run.thread_id.clone(), run.run_id.clone()).await;
        }

        let node_id = NodeId::new(run.assistant.assistant_id.clone());
        // D-18: the admission's allowance warnings first, so they take this run's lowest `seq`
        // values; the `RunScope` below deliberately does NOT carry them (emitted once, here).
        if matches!(run.status, RunStatus::Queued)
            && let Some(emitter) = emitter.as_deref()
        {
            self.emit_allowance_warnings(run, emitter).await;
        }
        dispatcher.emit(TraceEvent::RunStarted {
            run_id: Some(run.run_id.clone()),
            graph_fingerprint: AGENT_RUN_FINGERPRINT.to_string(),
        });
        dispatcher.emit(TraceEvent::NodeStarted {
            superstep: 0,
            node_id: node_id.clone(),
            attempt,
            muster_task_key: None,
        });
        let started = std::time::Instant::now();

        // 39-07: carry this run's Platform API id into the run engine's
        // shared PaladinPort via RunScope::with_run_id, so the agent
        // loop's own AgentLoopSettlement::PlatformRunsOnly writer (39-05)
        // settles this call under the Platform run id -- for any port
        // that does not override `execute_scoped`, the trait's default
        // body delegates straight to `execute`, so this is
        // behavior-identical for a port with no ledger settlement
        // installed. 40-04 (D-15/D-16): the same scope carries the run
        // row's recorded submitter as the ledger scope, so that writer
        // settles under the submitting principal's tenant and API key id
        // (the sentinel only when the row records no principal, D-10).
        let run_scope = RunScope::default()
            .with_run_id(run.run_id.clone())
            .with_ledger_scope(LedgerScope::from_attribution(run.submitted_by.as_ref()));
        let call = with_run_trace_scope(
            &emitter,
            paladin_port.execute_scoped(
                paladin.as_ref(),
                &input_text,
                &HeartbeatHandle::new(),
                &run_scope,
            ),
        )
        .await;

        let persisted = match call {
            Ok(result) => {
                let duration_ms = elapsed_ms(started);
                dispatcher.emit(TraceEvent::NodeFinished {
                    superstep: 0,
                    node_id,
                    attempt,
                    outcome: NodeOutcomeKind::Succeeded,
                    duration_ms,
                    usage: result.usage.clone(),
                    cost: result.cost.clone(),
                    cache_hit: false,
                });
                dispatcher.emit(TraceEvent::RunFinished {
                    status: RunFinishStatus::Completed,
                    total_supersteps: 0,
                    usage: dispatcher.total_usage(),
                    cost: dispatcher.total_cost(),
                    duration_ms,
                    trace_dropped_total: 0,
                });
                async {
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
                                halt_reason: None,
                            },
                        )
                        .await?;
                    self.queue.ack(&leased.token).await?;
                    // D-40, D-15: strictly AFTER the status write and ack;
                    // logged and never propagated (P2).
                    self.enqueue_webhook_delivery(run, RunStatus::Completed, None, None)
                        .await;
                    Ok(true)
                }
                .await
            }
            Err(error) => {
                // D-15 (single terminal event): the ONE `error` wire event
                // comes from `RunFinished { Failed }` through the dispatcher
                // -> `map_trace_event` (`message: null`, exactly like a graph
                // run's engine-emitted failure); this arm makes no direct bus
                // publish of its own. The failure text is persisted on the
                // run row below and stays readable through `GET /runs/{id}`.
                let duration_ms = elapsed_ms(started);
                dispatcher.emit(TraceEvent::NodeFinished {
                    superstep: 0,
                    node_id,
                    attempt,
                    outcome: NodeOutcomeKind::Failed,
                    duration_ms,
                    usage: TokenUsage::default(),
                    cost: None,
                    cache_hit: false,
                });
                dispatcher.emit(TraceEvent::RunFinished {
                    status: RunFinishStatus::Failed,
                    total_supersteps: 0,
                    usage: dispatcher.total_usage(),
                    cost: dispatcher.total_cost(),
                    duration_ms,
                    trace_dropped_total: 0,
                });
                self.persist_failure(leased, run, error.to_string()).await
            }
        };

        // Best-effort drain window, then unbind -- captured-then-returned so
        // a repository error never leaves the channel bound (T-45-11).
        if let Some(bus) = &self.event_bus {
            tokio::time::sleep(TRACE_DRAIN_GRACE_PERIOD).await;
            bus.unbind(&run.thread_id).await;
        }
        persisted
    }

    /// Record an `EngineError` returned outside normal `RunOutcome`
    /// reporting as a `Failed` run -- never a panic. If the repository write
    /// itself fails, log at `warn` and nack with a 1s delay so the run is
    /// retried rather than lost.
    ///
    /// **(D-14) The ONE publish this pool retains outside `map_trace_event`,
    /// deliberately.** Its two callers both reach it through a path that
    /// never gets a `TraceEvent::RunFinished` record: the corrupt `fork_from`
    /// case (`run_once`, before any engine dispatch begins), and a
    /// `WarEngine::start`/`resume`/`resume_with`/`fork` call that itself
    /// returns `Err` before its OWN `trace.emit(TraceEvent::RunStarted)` runs
    /// (graph/battlefield validation failures --
    /// `crates/paladin-battalion/src/engine/mod.rs`'s `start`, for one,
    /// validates the graph and initializes the Battlefield before opening
    /// its trace dispatcher). The `Runnable::Agent` path no longer reaches
    /// this method: [`RunWorkerPool::run_agent`] emits its own
    /// `RunFinished { Failed }` and calls [`Self::persist_failure`]
    /// directly, so an agent run produces exactly one `error` wire event.
    /// A `WarEngine` call that fails AFTER its own `RunStarted` already
    /// reached the trace pipeline DOES still get a `RunFinished{status:
    /// failed}` record (the engine unconditionally emits it right after
    /// `superstep::run` returns, `Ok` or `Err` alike) -- for that narrower
    /// subset this method's own publish is a harmless, accepted duplicate
    /// `error` event alongside the one `map_trace_event` already produced,
    /// not a correctness bug: an SSE consumer treats `error` as terminal
    /// either way. Fixing the duplicate would require distinguishing
    /// "already had a `RunStarted`" inside `paladin-battalion` itself, a
    /// crate outside this plan's file scope.
    ///
    /// The status write, ack and the subscribed `failed` webhook delivery
    /// come from [`Self::persist_failure`] (D-15), so a graph run failing
    /// here gets the delivery the `Ok`-only `Transition` arm never enqueued.
    async fn record_engine_failure(
        &self,
        leased: &LeasedRun,
        run: &Run,
        error_text: String,
    ) -> Result<bool, WorkerError> {
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
        self.persist_failure(leased, run, error_text).await
    }

    /// Persist a run's failure: status `Running -> Failed`, the outcome record
    /// carrying `error_text`, then `ack` -- or, on a repository error, log at
    /// `warn` and `nack` with a 1s delay so the run is retried rather than
    /// lost. On the ack path ONLY, enqueues the run's subscribed `failed`
    /// webhook delivery (D-40 ordering: strictly after the status write and
    /// ack; a delivery error is logged and never changes the run's status,
    /// prohibition P2). Publishes nothing on the bus -- callers own their
    /// wire event.
    async fn persist_failure(
        &self,
        leased: &LeasedRun,
        run: &Run,
        error_text: String,
    ) -> Result<bool, WorkerError> {
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
                        halt_reason: None,
                    },
                )
                .await?;
            Ok(())
        }
        .await;

        match record_result {
            Ok(()) => {
                self.queue.ack(&leased.token).await?;
                self.enqueue_webhook_delivery(run, RunStatus::Failed, None, None)
                    .await;
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

    /// Enqueue the `Pending` webhook delivery `status` implies for `run`, when
    /// a delivery repository is wired and the run's webhook subscribes to the
    /// matching event (D-40, PLAT-FR-14). The one enqueue path every terminal
    /// or suspension transition shares -- graph outcomes, agent outcomes and
    /// [`Self::persist_failure`] alike -- so both runnable kinds deliver
    /// through the identical [`webhook_delivery_for_outcome`] helper and the
    /// SSRF-guarded `WebhookDeliveryService` that drains it (D-00h).
    ///
    /// Callers invoke this strictly AFTER the run's own status write and ack
    /// succeeded. An enqueue error is logged (naming only the run id, never
    /// the URL or secret) and NEVER propagated (prohibition P2).
    async fn enqueue_webhook_delivery(
        &self,
        run: &Run,
        status: RunStatus,
        parleys: Option<&[ParleyRequest]>,
        halt_reason: Option<&HaltReason>,
    ) {
        let Some(deliveries) = &self.webhook_deliveries else {
            return;
        };
        let Some(kind) = run_status_to_event_kind(status) else {
            return;
        };
        if let Some(delivery) = webhook_delivery_for_outcome(
            run,
            kind,
            status,
            parleys,
            halt_reason,
            chrono::Utc::now(),
        ) && let Err(error) = deliveries.enqueue(delivery).await
        {
            log::warn!(
                "run worker: failed to enqueue webhook delivery for run {}: {error}",
                run.run_id
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    use async_trait::async_trait;

    use paladin_battalion::engine::{EngineLimits, InputMapping, NodeSpec, WarEngine, WarGraph};
    use paladin_core::platform::container::battlefield::{
        Battlefield, BattlefieldSchema, DispatchRule, FieldSpec,
    };
    use paladin_core::platform::container::cost::{Cost, CurrencyCode};
    use paladin_core::platform::container::execution_result::PaladinResult;
    use paladin_core::platform::container::paladin::Paladin;
    use paladin_core::platform::container::paladin_error::PaladinError;
    use paladin_core::platform::container::principal::{RunAttribution, TenantId};
    use paladin_core::platform::container::run::AssistantRef;
    use paladin_core::platform::container::treasury_ledger::{
        ReservationId, ReserveRequest, SettleOutcome, SettleRequest, SettlementKey, SpendQuery,
        SpendRow,
    };
    use paladin_core::platform::container::waypoint::{
        FrontierSnapshot, GraphFingerprint, NodeId, ThreadId, WaypointStatus,
    };
    use paladin_ports::output::paladin_port::{PaladinPort, PaladinStream};
    use paladin_ports::output::run_queue_port::{QueueError, QueuedRun};
    use paladin_ports::output::treasury_ledger_port::TreasuryLedgerError;
    use paladin_storage::run::in_memory::InMemoryRunRepository;
    use paladin_storage::run_queue::in_memory::InMemoryRunQueue;
    use paladin_storage::treasury::in_memory::InMemoryTreasuryLedger;
    use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;

    use super::super::events::RunEventBus;
    use super::super::resolver::{AssistantResolver, CodeWorkflowResolver, ResolvedAssistant};

    /// A [`PaladinPort`] that must never be called -- this module's own
    /// graphs never have `NodeSpec::Paladin` nodes (mirrors
    /// `stream_tests.rs`/`worker_tests.rs`'s own `UnusedPaladinPort`
    /// precedent).
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
                    halt_reason: None,
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
                    halt_reason: None,
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
                    halt_reason: None,
                },
            }
        );
    }

    #[test]
    fn map_outcome_halted_with_cancel_requested_transitions_to_cancelled() {
        let waypoint = paladin_core::platform::container::waypoint::WaypointId::generate();
        let outcome = RunOutcome::Halted {
            waypoint,
            cause: paladin_battalion::engine::HaltCause::Token,
        };
        let action = map_outcome(&outcome, true, false);
        assert_eq!(
            action,
            OutcomeAction::Transition {
                to: RunStatus::Cancelled,
                outcome: RunOutcomeRecord {
                    error: None,
                    output: None,
                    final_waypoint_id: Some(waypoint.to_string()),
                    halt_reason: None,
                },
            }
        );
    }

    #[test]
    fn map_outcome_halted_while_shutting_down_leaves_running_and_requeues() {
        let waypoint = paladin_core::platform::container::waypoint::WaypointId::generate();
        let outcome = RunOutcome::Halted {
            waypoint,
            cause: paladin_battalion::engine::HaltCause::Token,
        };
        let action = map_outcome(&outcome, false, true);
        assert_eq!(action, OutcomeAction::LeaveRunningAndRequeue);
    }

    #[test]
    fn map_outcome_halted_otherwise_transitions_to_halted() {
        let waypoint = paladin_core::platform::container::waypoint::WaypointId::generate();
        let outcome = RunOutcome::Halted {
            waypoint,
            cause: paladin_battalion::engine::HaltCause::Token,
        };
        let action = map_outcome(&outcome, false, false);
        assert_eq!(
            action,
            OutcomeAction::Transition {
                to: RunStatus::Halted,
                outcome: RunOutcomeRecord {
                    error: None,
                    output: None,
                    final_waypoint_id: Some(waypoint.to_string()),
                    halt_reason: None,
                },
            }
        );
    }

    fn spend_halt_outcome() -> (
        paladin_core::platform::container::waypoint::WaypointId,
        RunOutcome,
    ) {
        let waypoint = paladin_core::platform::container::waypoint::WaypointId::generate();
        let outcome = RunOutcome::Halted {
            waypoint,
            cause: HaltCause::Spend(
                paladin_core::platform::container::allowance::HaltReason::LedgerUnavailable,
            ),
        };
        (waypoint, outcome)
    }

    fn halted_transition(
        waypoint: paladin_core::platform::container::waypoint::WaypointId,
        to: RunStatus,
        halt_reason: Option<HaltReason>,
    ) -> OutcomeAction {
        OutcomeAction::Transition {
            to,
            outcome: RunOutcomeRecord {
                error: None,
                output: None,
                final_waypoint_id: Some(waypoint.to_string()),
                halt_reason,
            },
        }
    }

    #[test]
    fn map_outcome_spend_halt_transitions_to_halted_even_while_shutting_down() {
        let (waypoint, outcome) = spend_halt_outcome();
        assert_eq!(
            map_outcome(&outcome, false, true),
            halted_transition(
                waypoint,
                RunStatus::Halted,
                Some(HaltReason::LedgerUnavailable)
            ),
            "a spend halt is recorded Halted, never requeued by a drain"
        );
    }

    #[test]
    fn map_outcome_spend_halt_ignores_a_cancel_flag() {
        let (waypoint, outcome) = spend_halt_outcome();
        assert_eq!(
            map_outcome(&outcome, true, false),
            halted_transition(
                waypoint,
                RunStatus::Halted,
                Some(HaltReason::LedgerUnavailable)
            )
        );
        assert_eq!(
            map_outcome(&outcome, true, true),
            halted_transition(
                waypoint,
                RunStatus::Halted,
                Some(HaltReason::LedgerUnavailable)
            )
        );
    }

    #[test]
    fn map_outcome_spend_halt_records_the_reason() {
        // An exhausted-allowance reason (figures and all) is carried to the record verbatim,
        // and the record's `error` stays None: a halt is a resume point, not a failure (D-06).
        let usd = paladin_core::platform::container::cost::CurrencyCode::new("USD").unwrap();
        let reason = HaltReason::AllowanceExhausted(
            paladin_core::platform::container::allowance::AllowanceRefusal {
                scope_kind:
                    paladin_core::platform::container::allowance::AllowanceScopeKind::ApiKey,
                limit_kind:
                    paladin_core::platform::container::allowance::AllowanceLimitKind::Window,
                balance: Cost::new(1_000_000_001, usd.clone()),
                ceiling: Cost::new(1_000_000_001, usd),
                window: None,
                evaluated_at: chrono::Utc::now(),
            },
        );
        let waypoint = paladin_core::platform::container::waypoint::WaypointId::generate();
        let outcome = RunOutcome::Halted {
            waypoint,
            cause: HaltCause::Spend(reason.clone()),
        };
        match map_outcome(&outcome, false, false) {
            OutcomeAction::Transition { to, outcome } => {
                assert_eq!(to, RunStatus::Halted);
                assert_eq!(outcome.halt_reason, Some(reason));
                assert_eq!(outcome.error, None);
                assert_eq!(outcome.final_waypoint_id, Some(waypoint.to_string()));
            }
            other => panic!("expected a Halted transition, got {other:?}"),
        }
    }

    #[test]
    fn map_outcome_cancel_requested_cause_transitions_to_cancelled() {
        let waypoint = paladin_core::platform::container::waypoint::WaypointId::generate();
        let outcome = RunOutcome::Halted {
            waypoint,
            cause: HaltCause::CancelRequested,
        };
        // The cause alone decides: no side flag is needed.
        assert_eq!(
            map_outcome(&outcome, false, false),
            halted_transition(waypoint, RunStatus::Cancelled, None)
        );
        assert_eq!(
            map_outcome(&outcome, false, true),
            halted_transition(waypoint, RunStatus::Cancelled, None)
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

    #[test]
    fn halted_webhook_payload_carries_the_halt_reason_object() {
        let run = sample_run_with_webhook(vec![RunEventKind::Halted]);
        let reason = HaltReason::LedgerUnavailable;
        let delivery = webhook_delivery_for_outcome(
            &run,
            RunEventKind::Halted,
            RunStatus::Halted,
            None,
            Some(&reason),
            chrono::Utc::now(),
        )
        .unwrap();
        let payload: serde_json::Value = serde_json::from_str(&delivery.payload).unwrap();
        assert_eq!(payload["status"], "halted");
        assert_eq!(payload["event"], "halted");
        assert_eq!(payload["halt_reason"], reason.wire_json());
    }

    #[test]
    fn halted_webhook_without_a_reason_omits_the_key() {
        let run = sample_run_with_webhook(vec![RunEventKind::Halted]);
        let delivery = webhook_delivery_for_outcome(
            &run,
            RunEventKind::Halted,
            RunStatus::Halted,
            None,
            None,
            chrono::Utc::now(),
        )
        .unwrap();
        let payload: serde_json::Value = serde_json::from_str(&delivery.payload).unwrap();
        assert!(payload.get("halt_reason").is_none());
    }

    #[test]
    fn non_halted_webhook_payload_never_carries_a_halt_reason_and_is_byte_identical() {
        // Even if a reason were (wrongly) offered, only a `halted` event carries it. The bytes of
        // a completed payload are exactly the pre-change serialization of the same inputs: the
        // fixed key order below is the struct's field order, with no `halt_reason` key.
        let run = sample_run_with_webhook(vec![RunEventKind::Completed]);
        let now = chrono::Utc::now();
        let offered = HaltReason::LedgerUnavailable;
        let with_offer = webhook_delivery_for_outcome(
            &run,
            RunEventKind::Completed,
            RunStatus::Completed,
            None,
            Some(&offered),
            now,
        )
        .unwrap();
        let without = webhook_delivery_for_outcome(
            &run,
            RunEventKind::Completed,
            RunStatus::Completed,
            None,
            None,
            now,
        )
        .unwrap();
        assert_eq!(with_offer.payload, without.payload);

        let expected = format!(
            concat!(
                r#"{{"run_id":"{run_id}","thread_id":"{thread_id}","#,
                r#""assistant":{{"assistant_id":"a1","version":1}},"#,
                r#""status":"completed","event":"completed","#,
                r#""timestamp":{timestamp},"attempt":{attempt}}}"#,
            ),
            run_id = run.run_id,
            thread_id = run.thread_id,
            timestamp = serde_json::to_string(&now).unwrap(),
            attempt = run.attempt,
        );
        assert_eq!(without.payload, expected);
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

    // --- record_engine_failure's retained publish (D-14) -----------------

    /// An `EngineError` outside normal `RunOutcome` reporting -- here, a
    /// corrupt `fork_from.from_waypoint_id` caught before any engine
    /// dispatch begins, so no `TraceEvent::RunFinished` record was ever
    /// going to exist for it -- still reaches the bus as an `error` wire
    /// event, through `record_engine_failure`'s own retained publish
    /// (documented on that method).
    #[tokio::test(flavor = "multi_thread")]
    async fn engine_failure_still_reaches_the_error_wire_name() {
        let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
        let waypoint_store = Arc::new(InMemoryWaypointStore::new());
        let graph = Arc::new(WarGraph::new(
            BattlefieldSchema::new(vec![]),
            EngineLimits::default(),
        ));
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("noop", graph));
        let engine = Arc::new(WarEngine::new(
            Arc::new(UnusedPaladinPort),
            waypoint_store.clone(),
        ));

        let bus = Arc::new(RunEventBus::new());
        let pool = RunWorkerPool::new(
            engine,
            waypoint_store,
            repository.clone(),
            queue.clone(),
            resolver,
            Duration::from_secs(30),
        )
        .with_event_bus(bus.clone());

        let run_id = RunId::new_v7();
        let thread_id = ThreadId::new(format!("thread-{run_id}")).unwrap();
        let run = Run::new(
            run_id.clone(),
            thread_id.clone(),
            AssistantRef {
                assistant_id: "noop".to_string(),
                version: 1,
            },
            serde_json::json!({}),
        )
        .with_fork_from(ForkSpec {
            from_waypoint_id: "not-a-uuid".to_string(),
            edit: None,
        });
        repository.insert(&run).await.unwrap();
        queue
            .enqueue(QueuedRun {
                run_id: run_id.clone(),
                thread_id: thread_id.clone(),
                attempt: 1,
                enqueued_at: chrono::Utc::now(),
            })
            .await
            .unwrap();

        // Pre-bind so `record_engine_failure`'s own publish (fired inside
        // `run_once`, before it unbinds) is not silently dropped -- see the
        // matching pattern in `stream_tests.rs`.
        bus.bind(thread_id.clone(), run_id.clone()).await;
        let mut rx = bus.subscribe(&run_id).await.expect("bus must be bound");

        assert!(pool.run_once().await.unwrap());

        let event = rx
            .recv()
            .await
            .expect("record_engine_failure must publish an error event");
        assert_eq!(event.kind, RunStreamEventKind::Error);
        assert_eq!(
            event.payload.get("status").and_then(|v| v.as_str()),
            Some("failed")
        );

        let run_after = repository.get(&run_id).await.unwrap().unwrap();
        assert_eq!(run_after.status, RunStatus::Failed);
    }

    /// D-15: a graph run failing through `record_engine_failure` (the corrupt
    /// `fork_from` vehicle) now enqueues exactly one `Failed` delivery -- the
    /// graph `Err` path used to enqueue only on the `Ok` path. Its `error`
    /// wire event is pinned separately by
    /// `engine_failure_still_reaches_the_error_wire_name`.
    #[tokio::test(flavor = "multi_thread")]
    async fn graph_engine_failure_enqueues_failed_delivery() {
        use paladin_core::platform::container::run::WebhookSpec;
        use paladin_core::platform::container::webhook::WebhookDeliveryStatus;
        use paladin_storage::webhook::in_memory::InMemoryWebhookDeliveryRepository;

        let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
        let waypoint_store = Arc::new(InMemoryWaypointStore::new());
        let graph = Arc::new(WarGraph::new(
            BattlefieldSchema::new(vec![]),
            EngineLimits::default(),
        ));
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("noop", graph));
        let engine = Arc::new(WarEngine::new(
            Arc::new(UnusedPaladinPort),
            waypoint_store.clone(),
        ));
        let deliveries: Arc<dyn WebhookDeliveryRepositoryPort> =
            Arc::new(InMemoryWebhookDeliveryRepository::new());
        let pool = RunWorkerPool::new(
            engine,
            waypoint_store,
            repository.clone(),
            queue.clone(),
            resolver,
            Duration::from_secs(30),
        )
        .with_webhook_deliveries(Arc::clone(&deliveries));

        let run_id = RunId::new_v7();
        let thread_id = ThreadId::new(format!("thread-{run_id}")).unwrap();
        let run = Run::new(
            run_id.clone(),
            thread_id.clone(),
            AssistantRef {
                assistant_id: "noop".to_string(),
                version: 1,
            },
            serde_json::json!({}),
        )
        .with_fork_from(ForkSpec {
            from_waypoint_id: "not-a-uuid".to_string(),
            edit: None,
        })
        .with_webhook(WebhookSpec {
            url: "https://example.com/hook".to_string(),
            secret: None,
            events: vec![RunEventKind::Failed],
        });
        repository.insert(&run).await.unwrap();
        queue
            .enqueue(QueuedRun {
                run_id: run_id.clone(),
                thread_id,
                attempt: 1,
                enqueued_at: chrono::Utc::now(),
            })
            .await
            .unwrap();

        assert!(pool.run_once().await.unwrap());
        let run_after = repository.get(&run_id).await.unwrap().unwrap();
        assert_eq!(run_after.status, RunStatus::Failed);

        let page = deliveries.list_for_run(&run_id, 10, None).await.unwrap();
        assert_eq!(page.items.len(), 1, "exactly one Failed delivery");
        assert!(matches!(page.items[0].event, RunEventKind::Failed));
        assert!(matches!(
            page.items[0].status,
            WebhookDeliveryStatus::Pending
        ));
        let payload: serde_json::Value = serde_json::from_str(&page.items[0].payload).unwrap();
        assert_eq!(payload["status"], "failed");
    }

    // --- run_model_label (D-12, model_used discretion item) ---------

    fn labeled_paladin(name: &str, model: &str) -> Paladin {
        use paladin_core::base::entity::node::Node;
        use paladin_core::platform::container::paladin::PaladinData;

        let data = PaladinData {
            name: name.to_string(),
            model: model.to_string(),
            ..Default::default()
        };
        Node::new(data, Some(name.to_string()))
    }

    #[test]
    fn run_model_label_names_single_mixed_or_none() {
        use paladin_battalion::engine::{InputMapping, NodeSpec};
        use paladin_core::platform::container::battlefield::{DispatchRule, FieldName, FieldSpec};
        use paladin_core::platform::container::waypoint::NodeId;

        let field = FieldName::new("summary").unwrap();
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);

        // Zero Paladin nodes -> "none".
        let empty_graph = WarGraph::new(schema.clone(), EngineLimits::default());
        assert_eq!(run_model_label(&empty_graph), "none");

        // One Paladin node -> its own model.
        let mut single_graph = WarGraph::new(schema.clone(), EngineLimits::default());
        let n1 = NodeId::new("n1");
        single_graph.add_node(
            n1.clone(),
            NodeSpec::paladin(
                labeled_paladin("n1", "gpt-4"),
                InputMapping::new("go"),
                field.clone(),
            ),
        );
        single_graph.add_entry(n1);
        assert_eq!(run_model_label(&single_graph), "gpt-4");

        // Two Paladin nodes declaring the SAME model -> that model, not "mixed".
        let mut same_model_graph = WarGraph::new(schema.clone(), EngineLimits::default());
        let a1 = NodeId::new("a1");
        let a2 = NodeId::new("a2");
        same_model_graph.add_node(
            a1.clone(),
            NodeSpec::paladin(
                labeled_paladin("a1", "gpt-4"),
                InputMapping::new("go"),
                field.clone(),
            ),
        );
        same_model_graph.add_node(
            a2.clone(),
            NodeSpec::paladin(
                labeled_paladin("a2", "gpt-4"),
                InputMapping::new("go"),
                field.clone(),
            ),
        );
        same_model_graph.add_entry(a1);
        assert_eq!(run_model_label(&same_model_graph), "gpt-4");

        // Two Paladin nodes declaring DIFFERENT models -> "mixed".
        let mut mixed_graph = WarGraph::new(schema, EngineLimits::default());
        let m1 = NodeId::new("m1");
        let m2 = NodeId::new("m2");
        mixed_graph.add_node(
            m1.clone(),
            NodeSpec::paladin(
                labeled_paladin("m1", "gpt-4"),
                InputMapping::new("go"),
                field.clone(),
            ),
        );
        mixed_graph.add_node(
            m2.clone(),
            NodeSpec::paladin(
                labeled_paladin("m2", "claude-3-5-sonnet-20241022"),
                InputMapping::new("go"),
                field,
            ),
        );
        mixed_graph.add_entry(m1);
        assert_eq!(run_model_label(&mixed_graph), "mixed");
    }

    /// 45-02 (D-14): the agent-path label is the paladin's own model, or
    /// `"none"` when the model string is empty.
    #[test]
    fn agent_model_label_names_the_model_or_none() {
        assert_eq!(
            agent_model_label(&labeled_paladin("solo", "gpt-4")),
            "gpt-4"
        );
        assert_eq!(agent_model_label(&labeled_paladin("blank", "")), "none");
    }

    // --- 39-07: the worker attaches the treasury ledger per run with the
    // persisted attempt; agent-kind runs carry their run id -------------

    /// A [`TreasuryLedgerPort`] that records every `settle` call's key and
    /// amount, delegating everything else to a real `InMemoryTreasuryLedger`
    /// -- mirrors `paladin-battalion`'s own `RecordingTreasuryLedger`
    /// (39-04), re-implemented locally since that one is crate-private to
    /// `paladin-battalion`'s own test module.
    #[derive(Default)]
    struct RecordingTreasuryLedger {
        inner: InMemoryTreasuryLedger,
        calls: Mutex<Vec<(SettlementKey, Cost)>>,
        /// 40-04 (D-15): the `LedgerScope` of every `settle` request, in
        /// call order -- the instrument for the attribution tests.
        scopes: Mutex<Vec<LedgerScope>>,
    }

    impl RecordingTreasuryLedger {
        fn calls(&self) -> Vec<(SettlementKey, Cost)> {
            self.calls.lock().expect("calls mutex poisoned").clone()
        }

        fn scopes(&self) -> Vec<LedgerScope> {
            self.scopes.lock().expect("scopes mutex poisoned").clone()
        }
    }

    #[async_trait]
    impl TreasuryLedgerPort for RecordingTreasuryLedger {
        async fn reserve(
            &self,
            request: ReserveRequest,
        ) -> Result<ReservationId, TreasuryLedgerError> {
            self.inner.reserve(request).await
        }

        async fn release(&self, reservation: ReservationId) -> Result<(), TreasuryLedgerError> {
            self.inner.release(reservation).await
        }

        async fn settle(
            &self,
            request: SettleRequest,
        ) -> Result<SettleOutcome, TreasuryLedgerError> {
            self.calls
                .lock()
                .expect("calls mutex poisoned")
                .push((request.key.clone(), request.amount.clone()));
            self.scopes
                .lock()
                .expect("scopes mutex poisoned")
                .push(request.scope.clone());
            self.inner.settle(request).await
        }

        async fn spend(&self, query: SpendQuery) -> Result<Vec<SpendRow>, TreasuryLedgerError> {
            self.inner.spend(query).await
        }

        async fn store_now(&self) -> Result<chrono::DateTime<chrono::Utc>, TreasuryLedgerError> {
            self.inner.store_now().await
        }
    }

    /// A [`PaladinPort`] that always returns a priced result (45,000,000
    /// nanos USD) -- the instrument for every engine-path settlement test
    /// below. The model breakdown key comes from the dispatched Paladin
    /// node's own `model` field (`dispatch_paladin_model`), not from this
    /// port's response.
    struct PricedPaladinPort;

    #[async_trait]
    impl PaladinPort for PricedPaladinPort {
        async fn execute(
            &self,
            _paladin: &Paladin,
            input: &str,
        ) -> Result<PaladinResult, PaladinError> {
            Ok(PaladinResult {
                output: input.to_string(),
                cost: Some(Cost::new(45_000_000, CurrencyCode::new("USD").unwrap())),
                ..Default::default()
            })
        }

        async fn execute_stream(
            &self,
            _paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinStream, PaladinError> {
            unreachable!("this test never streams")
        }

        fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
            Ok(())
        }
    }

    /// A single-node, single-superstep workflow with one Paladin node,
    /// modeled `"gpt-4"`, priced by [`PricedPaladinPort`].
    fn priced_paladin_graph() -> Arc<WarGraph> {
        let field = FieldName::new("summary").unwrap();
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let node_id = NodeId::new("summarizer");
        graph.add_node(
            node_id.clone(),
            NodeSpec::paladin(
                labeled_paladin("summarizer", "gpt-4"),
                InputMapping::new("summarize this"),
                field,
            ),
        );
        graph.add_entry(node_id);
        Arc::new(graph)
    }

    /// Build a `RunWorkerPool` over a fresh in-memory waypoint
    /// store/repository/queue, wired with an `engine_factory` over
    /// [`PricedPaladinPort`] (required for the worker to attach a per-run
    /// treasury ledger -- the shared no-factory engine never gets one) and
    /// `treasury_ledger` when `Some`.
    fn build_ledger_pool(
        treasury_ledger: Option<Arc<dyn TreasuryLedgerPort>>,
    ) -> (
        RunWorkerPool<InMemoryWaypointStore>,
        Arc<dyn RunRepositoryPort>,
        Arc<dyn RunQueuePort>,
    ) {
        let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
        let store = Arc::new(InMemoryWaypointStore::new());
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("priced", priced_paladin_graph()));
        let base_engine = Arc::new(WarEngine::new(Arc::new(PricedPaladinPort), store.clone()));

        let factory_store = store.clone();
        let engine_factory: Arc<
            dyn Fn(CancellationToken) -> WarEngine<InMemoryWaypointStore> + Send + Sync,
        > = Arc::new(move |_token| {
            WarEngine::new(Arc::new(PricedPaladinPort), factory_store.clone())
        });

        let mut pool = RunWorkerPool::new(
            base_engine,
            store,
            repository.clone(),
            queue.clone(),
            resolver,
            Duration::from_secs(30),
        )
        .with_engine_factory(engine_factory);
        if let Some(ledger) = treasury_ledger {
            pool = pool.with_treasury_ledger(ledger);
        }

        (pool, repository, queue)
    }

    /// Insert a fresh `Queued` run against `"priced"` and enqueue it,
    /// returning the run id. The run records no submitting principal
    /// (`submitted_by = None`, D-10).
    async fn submit_priced_run(
        repository: &Arc<dyn RunRepositoryPort>,
        queue: &Arc<dyn RunQueuePort>,
    ) -> RunId {
        submit_priced_run_with(repository, queue, None).await
    }

    /// The `(acme, svc-a)` attribution every 40-04 attribution test stamps.
    fn acme_svc_a() -> RunAttribution {
        RunAttribution::new(TenantId::new("acme").unwrap(), "svc-a")
    }

    /// `submit_priced_run` with an explicit recorded submitter (40-04,
    /// D-15): `Some(a)` inserts the run `with_submitted_by(a)`.
    async fn submit_priced_run_with(
        repository: &Arc<dyn RunRepositoryPort>,
        queue: &Arc<dyn RunQueuePort>,
        submitted_by: Option<RunAttribution>,
    ) -> RunId {
        let run_id = RunId::new_v7();
        let thread_id = ThreadId::new(format!("thread-{run_id}")).unwrap();
        let mut run = Run::new(
            run_id.clone(),
            thread_id.clone(),
            AssistantRef {
                assistant_id: "priced".to_string(),
                version: 1,
            },
            serde_json::json!({}),
        );
        if let Some(attribution) = submitted_by {
            run = run.with_submitted_by(attribution);
        }
        repository.insert(&run).await.unwrap();
        queue
            .enqueue(QueuedRun {
                run_id: run_id.clone(),
                thread_id: thread_id.clone(),
                attempt: 1,
                enqueued_at: chrono::Utc::now(),
            })
            .await
            .unwrap();
        run_id
    }

    #[tokio::test]
    async fn engine_run_settles_under_its_run_id_and_first_attempt() {
        let ledger = Arc::new(RecordingTreasuryLedger::default());
        let treasury_ledger: Arc<dyn TreasuryLedgerPort> = ledger.clone();
        let (pool, repository, queue) = build_ledger_pool(Some(treasury_ledger));

        let run_id = submit_priced_run(&repository, &queue).await;

        assert!(pool.run_once().await.unwrap());

        let run = repository.get(&run_id).await.unwrap().unwrap();
        assert_eq!(run.status, RunStatus::Completed);

        let calls = ledger.calls();
        assert_eq!(calls.len(), 1, "exactly one settlement for one superstep");
        let (key, amount) = &calls[0];
        assert_eq!(key.run_id, run_id);
        assert_eq!(key.superstep, 1);
        assert_eq!(key.attempt, 1);
        assert_eq!(
            amount,
            &Cost::new(45_000_000, CurrencyCode::new("USD").unwrap())
        );
    }

    #[tokio::test]
    async fn redelivered_running_run_settles_under_the_bumped_attempt() {
        let ledger = Arc::new(RecordingTreasuryLedger::default());
        let treasury_ledger: Arc<dyn TreasuryLedgerPort> = ledger.clone();
        let (pool, repository, queue) = build_ledger_pool(Some(treasury_ledger));

        let run_id = submit_priced_run(&repository, &queue).await;

        // Seed the redelivery: a prior worker already claimed this run
        // (`Running`, `attempt == 1`) and this same message is redelivered.
        repository
            .update_status(
                &run_id,
                RunStatus::Queued,
                RunStatus::Running,
                chrono::Utc::now(),
            )
            .await
            .unwrap();

        assert!(pool.run_once().await.unwrap());

        let run = repository.get(&run_id).await.unwrap().unwrap();
        assert_eq!(run.status, RunStatus::Completed);
        assert_eq!(run.attempt, 2, "bump_attempt must have run");

        let calls = ledger.calls();
        assert_eq!(calls.len(), 1);
        let (key, _amount) = &calls[0];
        assert_eq!(key.run_id, run_id);
        assert_eq!(
            key.attempt, 2,
            "the settlement must key on the BUMPED attempt, never a re-invented 1"
        );
    }

    /// D-15: an engine run whose row records `submitted_by = (acme, svc-a)`
    /// settles its superstep under exactly that scope -- the worker builds
    /// the `SettlementContext.scope` from the run row, never the sentinel.
    #[tokio::test]
    async fn attributed_engine_run_settles_under_its_submitting_principal_scope() {
        let ledger = Arc::new(RecordingTreasuryLedger::default());
        let treasury_ledger: Arc<dyn TreasuryLedgerPort> = ledger.clone();
        let (pool, repository, queue) = build_ledger_pool(Some(treasury_ledger));

        let run_id = submit_priced_run_with(&repository, &queue, Some(acme_svc_a())).await;

        assert!(pool.run_once().await.unwrap());

        let run = repository.get(&run_id).await.unwrap().unwrap();
        assert_eq!(run.status, RunStatus::Completed);

        let scopes = ledger.scopes();
        assert_eq!(scopes.len(), 1, "exactly one settlement for one superstep");
        assert_eq!(scopes[0], LedgerScope::new("acme", "svc-a"));
    }

    /// D-10/D-15: a run with no recorded principal (schedule-fired or
    /// internal) still settles under the unattributed sentinel.
    #[tokio::test]
    async fn unattributed_engine_run_settles_under_the_unattributed_sentinel() {
        let ledger = Arc::new(RecordingTreasuryLedger::default());
        let treasury_ledger: Arc<dyn TreasuryLedgerPort> = ledger.clone();
        let (pool, repository, queue) = build_ledger_pool(Some(treasury_ledger));

        let run_id = submit_priced_run(&repository, &queue).await;

        assert!(pool.run_once().await.unwrap());

        let run = repository.get(&run_id).await.unwrap().unwrap();
        assert_eq!(run.status, RunStatus::Completed);
        assert!(run.submitted_by.is_none());

        let scopes = ledger.scopes();
        assert_eq!(scopes.len(), 1);
        assert_eq!(scopes[0], LedgerScope::unattributed());
        assert!(scopes[0].is_unattributed());
    }

    #[tokio::test]
    async fn pool_without_a_treasury_ledger_settles_nothing() {
        let (pool, repository, queue) = build_ledger_pool(None);
        let run_id = submit_priced_run(&repository, &queue).await;

        assert!(pool.run_once().await.unwrap());

        let run = repository.get(&run_id).await.unwrap().unwrap();
        assert_eq!(
            run.status,
            RunStatus::Completed,
            "a pool with no treasury ledger attached behaves exactly as before"
        );
    }

    /// A [`PaladinPort`] that records the [`RunScope`] handed to
    /// `execute_scoped`, then delegates to a plain, unpriced `execute` --
    /// the instrument for `agent_kind_run_passes_its_run_id_in_the_run_scope`.
    #[derive(Default)]
    struct ScopeRecordingPaladinPort {
        recorded: Mutex<Option<RunScope>>,
    }

    impl ScopeRecordingPaladinPort {
        fn recorded_scope(&self) -> Option<RunScope> {
            self.recorded.lock().expect("mutex poisoned").clone()
        }
    }

    #[async_trait]
    impl PaladinPort for ScopeRecordingPaladinPort {
        async fn execute(
            &self,
            _paladin: &Paladin,
            input: &str,
        ) -> Result<PaladinResult, PaladinError> {
            Ok(PaladinResult {
                output: input.to_string(),
                ..Default::default()
            })
        }

        async fn execute_scoped(
            &self,
            paladin: &Paladin,
            input: &str,
            _heartbeat: &HeartbeatHandle,
            scope: &RunScope,
        ) -> Result<PaladinResult, PaladinError> {
            *self.recorded.lock().expect("mutex poisoned") = Some(scope.clone());
            self.execute(paladin, input).await
        }

        async fn execute_stream(
            &self,
            _paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinStream, PaladinError> {
            unreachable!("this test never streams")
        }

        fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
            Ok(())
        }
    }

    /// An [`AssistantResolver`] resolving every id to a code-registered
    /// `Runnable::Agent` -- mirrors `worker_tests.rs`'s own
    /// `AgentOnlyResolver` precedent, local to this module since that one
    /// lives in a sibling test file.
    struct AgentOnlyResolver;

    #[async_trait]
    impl AssistantResolver for AgentOnlyResolver {
        async fn resolve(
            &self,
            assistant_id: &str,
            version: Option<u32>,
        ) -> Result<ResolvedAssistant, ResolveError> {
            Ok(ResolvedAssistant {
                reference: AssistantRef {
                    assistant_id: assistant_id.to_string(),
                    version: version.unwrap_or(1),
                },
                runnable: Runnable::Agent(Arc::new(labeled_paladin(assistant_id, "gpt-4"))),
                allowed_roles: vec![],
                source: paladin_core::platform::container::assistant::AssistantSource::Code,
            })
        }
    }

    #[tokio::test]
    async fn agent_kind_run_passes_its_run_id_in_the_run_scope() {
        let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
        let store = Arc::new(InMemoryWaypointStore::new());
        let resolver: Arc<dyn AssistantResolver> = Arc::new(AgentOnlyResolver);
        let engine = Arc::new(WarEngine::new(Arc::new(UnusedPaladinPort), store.clone()));

        let scope_port = Arc::new(ScopeRecordingPaladinPort::default());
        let paladin_port: Arc<dyn PaladinPort> = scope_port.clone();
        let pool = RunWorkerPool::new(
            engine,
            store,
            repository.clone(),
            queue.clone(),
            resolver,
            Duration::from_secs(30),
        )
        .with_paladin_port(paladin_port);

        let run_id = RunId::new_v7();
        let thread_id = ThreadId::new(format!("thread-{run_id}")).unwrap();
        let run = Run::new(
            run_id.clone(),
            thread_id.clone(),
            AssistantRef {
                assistant_id: "assistant".to_string(),
                version: 1,
            },
            serde_json::json!({}),
        );
        repository.insert(&run).await.unwrap();
        queue
            .enqueue(QueuedRun {
                run_id: run_id.clone(),
                thread_id: thread_id.clone(),
                attempt: 1,
                enqueued_at: chrono::Utc::now(),
            })
            .await
            .unwrap();

        assert!(pool.run_once().await.unwrap());

        let run = repository.get(&run_id).await.unwrap().unwrap();
        assert_eq!(run.status, RunStatus::Completed);

        let scope = scope_port
            .recorded_scope()
            .expect("execute_scoped must have been called");
        assert_eq!(scope.run_id, Some(run_id));
        assert_eq!(
            scope.ledger_scope,
            Some(LedgerScope::unattributed()),
            "a run with no recorded principal carries the sentinel scope (D-10)"
        );
    }

    /// D-16 (run path): an agent-kind run whose row records
    /// `submitted_by = (acme, svc-a)` hands the shared `PaladinPort` a
    /// `RunScope` carrying both its run id and `ledger_scope == (acme, svc-a)`.
    #[tokio::test]
    async fn agent_kind_run_carries_its_attribution_in_the_run_scope() {
        let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
        let store = Arc::new(InMemoryWaypointStore::new());
        let resolver: Arc<dyn AssistantResolver> = Arc::new(AgentOnlyResolver);
        let engine = Arc::new(WarEngine::new(Arc::new(UnusedPaladinPort), store.clone()));

        let scope_port = Arc::new(ScopeRecordingPaladinPort::default());
        let paladin_port: Arc<dyn PaladinPort> = scope_port.clone();
        let pool = RunWorkerPool::new(
            engine,
            store,
            repository.clone(),
            queue.clone(),
            resolver,
            Duration::from_secs(30),
        )
        .with_paladin_port(paladin_port);

        let run_id = RunId::new_v7();
        let thread_id = ThreadId::new(format!("thread-{run_id}")).unwrap();
        let run = Run::new(
            run_id.clone(),
            thread_id.clone(),
            AssistantRef {
                assistant_id: "assistant".to_string(),
                version: 1,
            },
            serde_json::json!({}),
        )
        .with_submitted_by(acme_svc_a());
        repository.insert(&run).await.unwrap();
        queue
            .enqueue(QueuedRun {
                run_id: run_id.clone(),
                thread_id: thread_id.clone(),
                attempt: 1,
                enqueued_at: chrono::Utc::now(),
            })
            .await
            .unwrap();

        assert!(pool.run_once().await.unwrap());

        let run = repository.get(&run_id).await.unwrap().unwrap();
        assert_eq!(run.status, RunStatus::Completed);

        let scope = scope_port
            .recorded_scope()
            .expect("execute_scoped must have been called");
        assert_eq!(scope.run_id, Some(run_id));
        assert_eq!(scope.ledger_scope, Some(LedgerScope::new("acme", "svc-a")));
    }
}
