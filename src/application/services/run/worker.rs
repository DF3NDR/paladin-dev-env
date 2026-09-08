//! `RunWorkerPool` — dequeues a `LeasedRun`, re-reads the `Run`, drives the
//! engine, applies the resulting status transition, and acks (D-11, D-13).
//!
//! Lives in the facade, never in `paladin-web`: ADR-0031 forbids a
//! `paladin-web -> paladin-battalion` edge in the default build, and
//! driving `WarEngine` needs battalion. This is exactly the Phase 24
//! D-24/D-25 arrangement.

use std::sync::Arc;
use std::time::Duration;

use thiserror::Error;

use paladin_battalion::engine::{EngineError, RunOutcome, WarEngine};
use paladin_core::platform::container::battlefield::StateDelta;
use paladin_core::platform::container::run::RunStatus;
use paladin_ports::output::run_queue_port::{QueueError, RunQueuePort};
use paladin_ports::output::run_repository_port::{
    RunOutcomeRecord, RunRepositoryError, RunRepositoryPort,
};
use paladin_ports::output::waypoint_port::WaypointPort;

use super::resolver::{AssistantResolver, ResolveError, Runnable};

/// Errors a single [`RunWorkerPool::run_once`] iteration can surface.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum WorkerError {
    /// The queue backend failed.
    #[error("run worker queue error: {0}")]
    Queue(#[from] QueueError),
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
}

/// Drives runs dequeued from a [`RunQueuePort`] through a real
/// [`WarEngine`], applying the resulting status transition through a
/// [`RunRepositoryPort`].
pub struct RunWorkerPool<W: WaypointPort> {
    engine: Arc<WarEngine<W>>,
    repository: Arc<dyn RunRepositoryPort>,
    queue: Arc<dyn RunQueuePort>,
    resolver: Arc<dyn AssistantResolver>,
    lease: Duration,
}

impl<W: WaypointPort + 'static> RunWorkerPool<W> {
    /// Construct a worker pool over the given engine, repository, queue and
    /// resolver, with the given lease duration.
    pub fn new(
        engine: Arc<WarEngine<W>>,
        repository: Arc<dyn RunRepositoryPort>,
        queue: Arc<dyn RunQueuePort>,
        resolver: Arc<dyn AssistantResolver>,
        lease: Duration,
    ) -> Self {
        Self {
            engine,
            repository,
            queue,
            resolver,
            lease,
        }
    }

    /// Dequeue and process at most one run.
    ///
    /// Returns `Ok(true)` if a run was processed (including a run that was
    /// nacked because it names an `Agent`-kind assistant, not yet wired),
    /// `Ok(false)` if the queue was empty.
    ///
    /// # Dispatch (D-09)
    ///
    /// This slice wires only the `start` branch: a dequeued run always
    /// begins a fresh [`WarEngine::start`]. The resume/resume_with branches
    /// (a thread with no Waypoint vs. one with pending responses vs. one
    /// without) are 27-04's dispatch point, documented here as the seam it
    /// fills -- a functionality gap, not an architectural one.
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

        let resolved = self
            .resolver
            .resolve(&run.assistant.assistant_id, Some(run.assistant.version))
            .await?;
        let Runnable::Workflow(graph) = resolved.runnable else {
            // Agent-kind dispatch is a later plan's work (D-28); nack so
            // the run stays visible for redelivery rather than being lost.
            self.queue
                .nack(&leased.token, Duration::from_secs(1))
                .await?;
            return Ok(true);
        };

        let started_at = chrono::Utc::now();
        self.repository
            .update_status(
                &run.run_id,
                RunStatus::Queued,
                RunStatus::Running,
                started_at,
            )
            .await?;

        let outcome = self
            .engine
            .start(&graph, run.thread_id.clone(), StateDelta::new())
            .await?;

        let (to, outcome_record) = match outcome {
            RunOutcome::Completed { waypoint, .. } => (
                RunStatus::Completed,
                RunOutcomeRecord {
                    error: None,
                    output: None,
                    final_waypoint_id: Some(waypoint.to_string()),
                },
            ),
            RunOutcome::AwaitingInput { waypoint, .. } => (
                RunStatus::AwaitingInput,
                RunOutcomeRecord {
                    error: None,
                    output: None,
                    final_waypoint_id: Some(waypoint.to_string()),
                },
            ),
            RunOutcome::Halted { waypoint } => (
                RunStatus::Halted,
                RunOutcomeRecord {
                    error: None,
                    output: None,
                    final_waypoint_id: Some(waypoint.to_string()),
                },
            ),
            RunOutcome::Failed { error, waypoint } => (
                RunStatus::Failed,
                RunOutcomeRecord {
                    error: Some(error.to_string()),
                    output: None,
                    final_waypoint_id: waypoint.map(|w| w.to_string()),
                },
            ),
        };

        let finished_at = chrono::Utc::now();
        self.repository
            .update_status(&run.run_id, RunStatus::Running, to, finished_at)
            .await?;
        self.repository
            .record_outcome(&run.run_id, outcome_record)
            .await?;

        // D-22: AwaitingInput releases the worker by ACKing, not NACKing --
        // a suspended run is durably parked, not unfinished queue work.
        self.queue.ack(&leased.token).await?;
        Ok(true)
    }
}
