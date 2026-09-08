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

use paladin_battalion::engine::shutdown::ShutdownCoordinator;
use paladin_battalion::engine::{EngineError, RunOutcome, WarEngine};
use paladin_core::platform::container::battlefield::StateDelta;
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::parley::ParleyResponse;
use paladin_core::platform::container::run::{Run, RunStatus};
use paladin_core::platform::container::waypoint::Waypoint;
use paladin_ports::output::paladin_port::PaladinPort;
use paladin_ports::output::run_queue_port::{LeaseToken, LeasedRun, RunQueuePort};
use paladin_ports::output::run_repository_port::{
    RunOutcomeRecord, RunRepositoryError, RunRepositoryPort,
};
use paladin_ports::output::waypoint_port::{WaypointError, WaypointPort};

use super::resolver::{AssistantResolver, ResolveError, Runnable};

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
pub struct LeaseHeartbeat {
    handle: JoinHandle<()>,
}

impl LeaseHeartbeat {
    /// Spawn a background task extending `token`'s lease by `lease` every
    /// `lease / 4`, until this handle is dropped.
    pub fn spawn(queue: Arc<dyn RunQueuePort>, token: LeaseToken, lease: Duration) -> Self {
        let interval = if lease.is_zero() { lease } else { lease / 4 };
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
        Self { handle }
    }
}

impl Drop for LeaseHeartbeat {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

/// What a worker does with a resolved workflow run, decided purely from the
/// thread's latest [`Waypoint`] and the run's parked responses (D-09).
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
}

impl WorkerDispatch {
    /// Decide dispatch purely from whether a Waypoint exists and whether
    /// responses are parked on the run -- the worker's single entry point
    /// (D-09).
    pub fn decide(latest: Option<&Waypoint>, pending: &[ParleyResponse]) -> WorkerDispatch {
        match latest {
            None => WorkerDispatch::Start,
            Some(_) if !pending.is_empty() => WorkerDispatch::ResumeWith(pending.to_vec()),
            Some(_) => WorkerDispatch::Resume,
        }
    }
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
        }
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
        let dispatch = WorkerDispatch::decide(latest.as_ref(), &run.pending_responses);

        let heartbeat = LeaseHeartbeat::spawn(self.queue.clone(), leased.token.clone(), self.lease);
        let outcome_result = match dispatch {
            WorkerDispatch::Start => {
                self.engine
                    .start(&graph, run.thread_id.clone(), StateDelta::new())
                    .await
            }
            WorkerDispatch::Resume => self.engine.resume(&graph, run.thread_id.clone()).await,
            WorkerDispatch::ResumeWith(responses) => {
                let result = self
                    .engine
                    .resume_with(&graph, run.thread_id.clone(), responses)
                    .await;
                if result.is_ok() {
                    self.repository.clear_pending_responses(&run.run_id).await?;
                }
                result
            }
        };
        // D-10: stop heartbeating the moment the run returns.
        drop(heartbeat);

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
            }
            OutcomeAction::LeaveRunningAndRequeue => {
                self.queue.nack(&leased.token, Duration::ZERO).await?;
            }
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
        assert_eq!(WorkerDispatch::decide(None, &[]), WorkerDispatch::Start);
    }

    #[test]
    fn decide_returns_resume_when_waypoint_exists_and_no_pending_responses() {
        let waypoint = sample_waypoint();
        assert_eq!(
            WorkerDispatch::decide(Some(&waypoint), &[]),
            WorkerDispatch::Resume
        );
    }

    #[test]
    fn decide_returns_resume_with_when_waypoint_exists_and_responses_are_pending() {
        let waypoint = sample_waypoint();
        let responses = vec![sample_response()];
        assert_eq!(
            WorkerDispatch::decide(Some(&waypoint), &responses),
            WorkerDispatch::ResumeWith(responses)
        );
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
