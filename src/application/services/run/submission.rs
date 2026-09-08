//! `RunSubmissionService` — the facade [`RunSubmissionPort`] implementation
//! (D-12).
//!
//! Resolves the assistant reference, builds a `Run`, inserts it, enqueues a
//! `QueuedRun`, and returns — one resolve, one insert, one enqueue, no
//! engine type named anywhere in this file. That absence is the 250 ms p99
//! architectural claim (27-CONTEXT `<specifics>`): `POST /runs` cannot
//! consume worker time because nothing on this path can reach the engine.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;

use paladin_core::platform::container::run::{Run, RunId};
use paladin_core::platform::container::waypoint::ThreadId;
use paladin_ports::input::run_submission_port::{
    CancelOutcome, RunAccepted, RunSubmissionError, RunSubmissionPort, SubmitRun,
};
use paladin_ports::output::run_queue_port::{QueueError, QueuedRun, RunQueuePort};
use paladin_ports::output::run_repository_port::{RunRepositoryError, RunRepositoryPort};

use super::cancel::LocalRunTokens;
use super::resolver::{AssistantResolver, ResolveError};

/// Generate a fresh [`ThreadId`] from a UUIDv7 string.
///
/// A UUIDv7 string is always non-empty and free of whitespace, so
/// `ThreadId::new` can never reject it here -- looped rather than
/// `.expect()`ed so this file stays free of `unwrap`/`expect` (CLAUDE.md)
/// while remaining provably total (the loop body always returns on its
/// first iteration).
fn generate_thread_id() -> ThreadId {
    loop {
        if let Ok(id) = ThreadId::new(uuid::Uuid::now_v7().to_string()) {
            return id;
        }
    }
}

impl From<ResolveError> for RunSubmissionError {
    fn from(err: ResolveError) -> Self {
        match err {
            ResolveError::UnknownAssistant { assistant_id } => {
                RunSubmissionError::UnknownAssistant { assistant_id }
            }
            ResolveError::UnknownVersion {
                assistant_id,
                version,
            } => RunSubmissionError::UnknownVersion {
                assistant_id,
                version,
            },
        }
    }
}

// `RunRepositoryError` and `RunSubmissionError` are both foreign types
// (declared in `paladin-ports`), so a `From` impl between them here would
// violate the orphan rule -- unlike `ResolveError` above, which is local to
// this crate. Plain mapping functions instead, applied at each call site
// with `.map_err(...)`.
fn map_repository_error(err: RunRepositoryError) -> RunSubmissionError {
    match err {
        RunRepositoryError::ThreadBusy { thread_id } => {
            RunSubmissionError::ThreadBusy { thread_id }
        }
        other => RunSubmissionError::Backend {
            message: other.to_string(),
        },
    }
}

fn map_queue_error(err: QueueError) -> RunSubmissionError {
    RunSubmissionError::Backend {
        message: err.to_string(),
    }
}

/// Map a [`RunRepositoryPort::request_cancel`] failure specifically:
/// [`RunRepositoryError::NotFound`] and [`RunRepositoryError::AlreadyTerminal`]
/// carry their own typed [`RunSubmissionError`] counterparts (the terminal
/// case is the documented `cancel` behavior, not a generic backend failure);
/// everything else falls through to the same generic `Backend` wrap
/// [`map_repository_error`] uses.
fn map_cancel_error(err: RunRepositoryError) -> RunSubmissionError {
    match err {
        RunRepositoryError::NotFound { run_id } => RunSubmissionError::NotFound { run_id },
        RunRepositoryError::AlreadyTerminal { run_id, status } => {
            RunSubmissionError::AlreadyTerminal { run_id, status }
        }
        other => map_repository_error(other),
    }
}

/// Implements [`RunSubmissionPort`] over a [`RunRepositoryPort`], a
/// [`RunQueuePort`] and an [`AssistantResolver`] (D-11, D-12).
pub struct RunSubmissionService {
    repository: Arc<dyn RunRepositoryPort>,
    queue: Arc<dyn RunQueuePort>,
    resolver: Arc<dyn AssistantResolver>,
    local_tokens: LocalRunTokens,
}

impl RunSubmissionService {
    /// Construct a service over the given repository, queue and resolver.
    ///
    /// `cancel`'s `was_local` always reports `false` until
    /// [`RunSubmissionService::with_local_tokens`] shares the SAME
    /// [`LocalRunTokens`] registry a [`super::worker::RunWorkerPool`]
    /// dispatching this service's runs also holds -- correct default for a
    /// service instance that never itself runs a worker pool.
    pub fn new(
        repository: Arc<dyn RunRepositoryPort>,
        queue: Arc<dyn RunQueuePort>,
        resolver: Arc<dyn AssistantResolver>,
    ) -> Self {
        Self {
            repository,
            queue,
            resolver,
            local_tokens: LocalRunTokens::new(),
        }
    }

    /// Share `local_tokens` with the [`super::worker::RunWorkerPool`]
    /// instance(s) executing runs this service submits/cancels (D-16), so
    /// `cancel` can observe whether THIS process instance is locally
    /// dispatching the target run.
    pub fn with_local_tokens(mut self, local_tokens: LocalRunTokens) -> Self {
        self.local_tokens = local_tokens;
        self
    }
}

#[async_trait]
impl RunSubmissionPort for RunSubmissionService {
    async fn submit(&self, request: SubmitRun) -> Result<RunAccepted, RunSubmissionError> {
        let resolved = self
            .resolver
            .resolve(&request.assistant_id, request.version)
            .await?;

        let thread_id = request.thread_id.unwrap_or_else(generate_thread_id);
        let mut run = Run::new(
            RunId::new_v7(),
            thread_id.clone(),
            resolved.reference,
            request.input,
        );
        if let Some(webhook) = request.webhook {
            run = run.with_webhook(webhook);
        }

        self.repository
            .insert(&run)
            .await
            .map_err(map_repository_error)?;
        self.queue
            .enqueue(QueuedRun {
                run_id: run.run_id.clone(),
                thread_id: run.thread_id.clone(),
                attempt: run.attempt,
                enqueued_at: Utc::now(),
            })
            .await
            .map_err(map_queue_error)?;

        Ok(RunAccepted {
            run_id: run.run_id,
            thread_id: run.thread_id,
        })
    }

    /// D-16: durability first. `request_cancel` persists the flag through
    /// the repository BEFORE `cancel_if_local` ever runs -- so a crash
    /// between the two steps still leaves the durable flag written, which
    /// is all a [`super::cancel::DbCancellationProbe`] on any instance
    /// needs to see the run halt at its next superstep boundary. The local
    /// signal below is a latency optimization only, never load-bearing for
    /// correctness.
    async fn cancel(&self, run_id: &RunId) -> Result<CancelOutcome, RunSubmissionError> {
        let status = self
            .repository
            .request_cancel(run_id)
            .await
            .map_err(map_cancel_error)?;

        let was_local = self.local_tokens.cancel_if_local(run_id).await;

        Ok(CancelOutcome {
            run_id: run_id.clone(),
            status,
            was_local,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::services::run::resolver::CodeWorkflowResolver;
    use paladin_battalion::engine::{EngineLimits, WarGraph};
    use paladin_core::platform::container::battlefield::BattlefieldSchema;
    use paladin_storage::run::in_memory::InMemoryRunRepository;
    use paladin_storage::run_queue::in_memory::InMemoryRunQueue;

    fn empty_graph() -> Arc<WarGraph> {
        Arc::new(WarGraph::new(
            BattlefieldSchema::new(vec![]),
            EngineLimits::default(),
        ))
    }

    fn service_with(
        resolver: CodeWorkflowResolver,
    ) -> (
        RunSubmissionService,
        Arc<dyn RunRepositoryPort>,
        Arc<dyn RunQueuePort>,
    ) {
        let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
        let service =
            RunSubmissionService::new(repository.clone(), queue.clone(), Arc::new(resolver));
        (service, repository, queue)
    }

    #[tokio::test]
    async fn submit_inserts_and_enqueues_exactly_once() {
        let resolver = CodeWorkflowResolver::new().register("wf1", empty_graph());
        let (service, repository, queue) = service_with(resolver);

        let accepted = service
            .submit(SubmitRun {
                assistant_id: "wf1".to_string(),
                version: None,
                thread_id: None,
                input: serde_json::json!({}),
                webhook: None,
                requested_by: None,
            })
            .await
            .unwrap();

        let stored = repository.get(&accepted.run_id).await.unwrap();
        assert!(stored.is_some());
        assert_eq!(queue.depth().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn submit_generates_a_thread_id_when_none_supplied() {
        let resolver = CodeWorkflowResolver::new().register("wf1", empty_graph());
        let (service, _repository, _queue) = service_with(resolver);

        let accepted = service
            .submit(SubmitRun {
                assistant_id: "wf1".to_string(),
                version: None,
                thread_id: None,
                input: serde_json::json!({}),
                webhook: None,
                requested_by: None,
            })
            .await
            .unwrap();
        assert!(!accepted.thread_id.as_str().is_empty());
    }

    #[tokio::test]
    async fn submit_unknown_assistant_errors_and_touches_nothing() {
        let resolver = CodeWorkflowResolver::new();
        let (service, _repository, queue) = service_with(resolver);

        let err = service
            .submit(SubmitRun {
                assistant_id: "nope".to_string(),
                version: None,
                thread_id: None,
                input: serde_json::json!({}),
                webhook: None,
                requested_by: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, RunSubmissionError::UnknownAssistant { .. }));
        assert_eq!(queue.depth().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn submit_second_run_on_busy_thread_returns_thread_busy() {
        let resolver = CodeWorkflowResolver::new().register("wf1", empty_graph());
        let (service, _repository, _queue) = service_with(resolver);
        let thread_id = ThreadId::new("shared-thread").unwrap();

        service
            .submit(SubmitRun {
                assistant_id: "wf1".to_string(),
                version: None,
                thread_id: Some(thread_id.clone()),
                input: serde_json::json!({}),
                webhook: None,
                requested_by: None,
            })
            .await
            .unwrap();

        let err = service
            .submit(SubmitRun {
                assistant_id: "wf1".to_string(),
                version: None,
                thread_id: Some(thread_id),
                input: serde_json::json!({}),
                webhook: None,
                requested_by: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, RunSubmissionError::ThreadBusy { .. }));
    }
}
