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

use paladin_core::platform::container::assistant::AssistantSource;
use paladin_core::platform::container::run::{ForkSpec, Run, RunId};
use paladin_core::platform::container::user::UserRole;
use paladin_core::platform::container::waypoint::ThreadId;
use paladin_ports::input::run_submission_port::{
    CancelOutcome, ForkRun, RunAccepted, RunSubmissionError, RunSubmissionPort, SubmitRun,
};
use paladin_ports::output::run_queue_port::{QueueError, QueuedRun, RunQueuePort};
use paladin_ports::output::run_repository_port::{RunQuery, RunRepositoryError, RunRepositoryPort};
use paladin_ports::output::waypoint_port::WaypointPort;

use super::cancel::LocalRunTokens;
use super::resolver::{AssistantResolver, ResolveError};
use super::webhook::SsrfGuard;

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
        RunRepositoryError::UnknownAssistant { assistant_id } => {
            RunSubmissionError::UnknownAssistant { assistant_id }
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
    /// The write-time SSRF guard (D-42) `submit` runs a caller-supplied
    /// `webhook.url` through before ever persisting the run. Defaults to
    /// `SsrfGuard::new(false)` -- private/loopback/link-local addresses
    /// rejected unless a deployment explicitly opts in via
    /// [`RunSubmissionService::with_ssrf_guard`].
    ssrf_guard: SsrfGuard,
    /// Validates a `fork`'s `from_waypoint_id` actually exists on the
    /// target thread (D-45), wired via [`RunSubmissionService::with_waypoints`].
    /// `None` (the default) means [`RunSubmissionService::fork`] answers
    /// [`RunSubmissionError::NotWired`] -- fork cannot correctly validate
    /// its own precondition without this collaborator, so failing closed
    /// (a genuine 501) is the honest answer, not a silently skipped check.
    waypoints: Option<Arc<dyn WaypointPort>>,
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
            ssrf_guard: SsrfGuard::new(false),
            waypoints: None,
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

    /// Override the default write-time [`SsrfGuard`] (D-42) -- e.g. to
    /// enable `allow_private` for an internal-network deployment, or to
    /// inject a stubbed resolver in tests.
    pub fn with_ssrf_guard(mut self, ssrf_guard: SsrfGuard) -> Self {
        self.ssrf_guard = ssrf_guard;
        self
    }

    /// Wire a [`WaypointPort`] so [`RunSubmissionService::fork`] can
    /// validate that `from_waypoint_id` actually exists on the target
    /// thread before enqueuing a forked run (D-45).
    pub fn with_waypoints(mut self, waypoints: Arc<dyn WaypointPort>) -> Self {
        self.waypoints = Some(waypoints);
        self
    }

    /// D-46: authorize an invocation-shaped request (`submit`/`cancel`/
    /// `fork`) against `allowed_roles` -- empty means any authenticated
    /// caller, `None` `requested_by` skips the check entirely (an
    /// internal/same-process caller with no principal to authorize
    /// against). Shared by all three call sites so the rule is expressed
    /// exactly once.
    fn authorize_invocation(
        requested_by: &Option<(String, UserRole)>,
        allowed_roles: &[UserRole],
    ) -> Result<(), RunSubmissionError> {
        let Some((_, role)) = requested_by else {
            return Ok(());
        };
        if allowed_roles.is_empty() || allowed_roles.contains(role) {
            Ok(())
        } else {
            Err(RunSubmissionError::Forbidden {
                reason: "role not permitted for this assistant".to_string(),
            })
        }
    }
}

#[async_trait]
impl RunSubmissionPort for RunSubmissionService {
    async fn submit(&self, request: SubmitRun) -> Result<RunAccepted, RunSubmissionError> {
        // D-42: the write-time half of the SSRF guard -- checked BEFORE
        // any resolve/insert/enqueue work, so a rejected URL never
        // persists a run at all.
        if let Some(webhook) = &request.webhook
            && let Err(rejection) = self.ssrf_guard.check_url(&webhook.url).await
        {
            return Err(RunSubmissionError::WebhookRejected {
                reason: rejection.to_string(),
            });
        }

        let resolved = self
            .resolver
            .resolve(&request.assistant_id, request.version)
            .await?;

        // D-46: invocation-shaped -- authorize against the assistant's own
        // `allowed_roles` before ANY resolve-consequent work (insert/
        // enqueue). `paladin-web` has no visibility into `allowed_roles`
        // at all (ADR-0031), so this is the only layer that can perform
        // this check.
        Self::authorize_invocation(&request.requested_by, &resolved.allowed_roles)?;

        // D-30: only a `version: None` submission against a STORED
        // assistant freezes `latest` inside the repository's own atomic
        // insert. A pinned `version: Some(v)` uses a plain `insert` with
        // the caller's own reference; a code-registered assistant is
        // always frozen at version 1 by `CodeWorkflowResolver` itself, so
        // `insert_with_latest` would only ever fail `UnknownAssistant`
        // there (no `assistants` row for a code id).
        let use_latest = request.version.is_none() && resolved.source == AssistantSource::Stored;

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

        if use_latest {
            let resolved_version = self
                .repository
                .insert_with_latest(&run)
                .await
                .map_err(map_repository_error)?;
            run.assistant.version = resolved_version;
        } else {
            self.repository
                .insert(&run)
                .await
                .map_err(map_repository_error)?;
        }
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
    ///
    /// D-46: when `requested_by` is `Some`, authorizes against the run's
    /// own assistant `allowed_roles` (resolved fresh -- `allowed_roles` is
    /// not itself persisted on the run row) BEFORE the durable flag is
    /// ever written; a mismatch returns [`RunSubmissionError::Forbidden`]
    /// and touches nothing.
    async fn cancel(
        &self,
        run_id: &RunId,
        requested_by: Option<(String, UserRole)>,
    ) -> Result<CancelOutcome, RunSubmissionError> {
        if requested_by.is_some() {
            let run = self
                .repository
                .get(run_id)
                .await
                .map_err(map_repository_error)?
                .ok_or_else(|| RunSubmissionError::NotFound {
                    run_id: run_id.clone(),
                })?;
            let resolved = self
                .resolver
                .resolve(&run.assistant.assistant_id, Some(run.assistant.version))
                .await?;
            Self::authorize_invocation(&requested_by, &resolved.allowed_roles)?;
        }

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

    /// D-45: validates the fork point exists (via the wired
    /// [`WaypointPort`]), copies the assistant reference from the thread's
    /// most recent run, then inserts and enqueues a NEW run carrying
    /// `fork_from`. Subject to the same busy-thread and SSRF checks
    /// [`RunSubmissionPort::submit`] enforces.
    async fn fork(&self, request: ForkRun) -> Result<RunAccepted, RunSubmissionError> {
        // D-42: same write-time SSRF guard `submit` runs.
        if let Some(webhook) = &request.webhook
            && let Err(rejection) = self.ssrf_guard.check_url(&webhook.url).await
        {
            return Err(RunSubmissionError::WebhookRejected {
                reason: rejection.to_string(),
            });
        }

        // D-17/D-18: a thread with an active run cannot accept a fork
        // either -- the busy invariant is thread-wide, not run-specific.
        if self
            .repository
            .active_run_for_thread(&request.thread_id)
            .await
            .map_err(map_repository_error)?
            .is_some()
        {
            return Err(RunSubmissionError::ThreadBusy {
                thread_id: request.thread_id.clone(),
            });
        }

        // D-45: validate the fork point exists on this thread. Failing
        // closed (`NotWired`) when no `WaypointPort` is wired is the
        // honest answer -- this service cannot correctly skip the check
        // and still claim to have performed it.
        let waypoints = self
            .waypoints
            .as_ref()
            .ok_or(RunSubmissionError::NotWired)?;
        waypoints
            .get(&request.thread_id, &request.from_waypoint_id)
            .await
            .map_err(|e| RunSubmissionError::Backend {
                message: e.to_string(),
            })?
            .ok_or_else(|| RunSubmissionError::UnknownWaypoint {
                thread_id: request.thread_id.clone(),
                waypoint_id: request.from_waypoint_id.to_string(),
            })?;

        // Copy the assistant reference from the thread's most recent run
        // (submitted_at DESC, so `limit: 1` is the latest one).
        let page = self
            .repository
            .list(RunQuery {
                thread_id: Some(request.thread_id.clone()),
                limit: 1,
                ..Default::default()
            })
            .await
            .map_err(map_repository_error)?;
        let Some(latest) = page.items.into_iter().next() else {
            return Err(RunSubmissionError::UnknownThread {
                thread_id: request.thread_id.clone(),
            });
        };

        // D-46: same invocation-shaped authorization `submit`/`cancel`
        // enforce, resolved fresh from the copied assistant reference.
        let resolved = self
            .resolver
            .resolve(
                &latest.assistant.assistant_id,
                Some(latest.assistant.version),
            )
            .await?;
        Self::authorize_invocation(&request.requested_by, &resolved.allowed_roles)?;

        let fork_spec = ForkSpec {
            from_waypoint_id: request.from_waypoint_id.to_string(),
            edit: request.edit,
        };
        let mut run = Run::new(
            RunId::new_v7(),
            request.thread_id.clone(),
            latest.assistant.clone(),
            serde_json::json!({}),
        )
        .with_fork_from(fork_spec);
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::services::run::resolver::{
        AssistantResolver, CodeWorkflowResolver, ResolveError, ResolvedAssistant, Runnable,
    };
    use async_trait::async_trait;
    use paladin_battalion::engine::{EngineLimits, WarGraph};
    use paladin_core::platform::container::assistant::{
        AssistantDefinition, AssistantId, AssistantKind, NewAssistantVersion,
    };
    use paladin_core::platform::container::battlefield::BattlefieldSchema;
    use paladin_ports::output::assistant_repository_port::AssistantRepositoryPort;
    use paladin_storage::assistant::in_memory::InMemoryAssistantRepository;
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

    // --- D-42: write-time SSRF guard ---------------------------------------

    #[tokio::test]
    async fn submit_with_a_loopback_webhook_url_is_rejected_and_touches_nothing() {
        let resolver = CodeWorkflowResolver::new().register("wf1", empty_graph());
        let (service, _repository, queue) = service_with(resolver);

        let err = service
            .submit(SubmitRun {
                assistant_id: "wf1".to_string(),
                version: None,
                thread_id: None,
                input: serde_json::json!({}),
                webhook: Some(paladin_core::platform::container::run::WebhookSpec {
                    url: "http://127.0.0.1/hook".to_string(),
                    secret: None,
                    events: vec![],
                }),
                requested_by: None,
            })
            .await
            .unwrap_err();

        assert!(matches!(err, RunSubmissionError::WebhookRejected { .. }));
        assert_eq!(
            queue.depth().await.unwrap(),
            0,
            "a rejected webhook URL must never enqueue a run"
        );
    }

    #[tokio::test]
    async fn submit_with_allow_private_guard_accepts_a_loopback_webhook_url() {
        let resolver = CodeWorkflowResolver::new().register("wf1", empty_graph());
        let (service, repository, _queue) = service_with(resolver);
        let service = service.with_ssrf_guard(super::super::webhook::SsrfGuard::new(true));

        let accepted = service
            .submit(SubmitRun {
                assistant_id: "wf1".to_string(),
                version: None,
                thread_id: None,
                input: serde_json::json!({}),
                webhook: Some(paladin_core::platform::container::run::WebhookSpec {
                    url: "http://127.0.0.1/hook".to_string(),
                    secret: None,
                    events: vec![],
                }),
                requested_by: None,
            })
            .await
            .unwrap();

        assert!(repository.get(&accepted.run_id).await.unwrap().is_some());
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

    // --- D-30: `insert_with_latest` freezes a stored assistant's version ---

    /// A minimal `AssistantResolver` test double that always resolves
    /// `assistant_id` to the empty graph at whatever version is requested
    /// (or `1` when `None`), with `source: AssistantSource::Stored` --
    /// proving `submit`'s own routing decision (`use_latest`) rather than
    /// re-testing `StoredAssistantResolver` itself (owned by
    /// `services::assistant::resolver`).
    struct StubStoredResolver;

    #[async_trait]
    impl AssistantResolver for StubStoredResolver {
        async fn resolve(
            &self,
            assistant_id: &str,
            version: Option<u32>,
        ) -> Result<ResolvedAssistant, ResolveError> {
            Ok(ResolvedAssistant {
                reference: paladin_core::platform::container::run::AssistantRef {
                    assistant_id: assistant_id.to_string(),
                    version: version.unwrap_or(1),
                },
                runnable: Runnable::Workflow(empty_graph()),
                allowed_roles: Vec::new(),
                source: AssistantSource::Stored,
            })
        }
    }

    #[tokio::test]
    async fn submit_without_version_against_a_stored_assistant_freezes_latest() {
        let assistants = Arc::new(InMemoryAssistantRepository::new());
        let id = AssistantId::new("stored-wf").unwrap();
        assistants
            .create(
                &id,
                NewAssistantVersion {
                    definition: AssistantDefinition {
                        kind: AssistantKind::Workflow,
                        body: serde_json::json!({}),
                    },
                    created_by: None,
                    note: None,
                },
            )
            .await
            .unwrap();
        // Publish a second version so `latest` (2) differs from the
        // resolver's own hardcoded fallback (1) -- proving the run's
        // persisted version comes from the REPOSITORY's atomic freeze, not
        // from whatever the resolver happened to return.
        assistants
            .append_version(
                &id,
                NewAssistantVersion {
                    definition: AssistantDefinition {
                        kind: AssistantKind::Workflow,
                        body: serde_json::json!({}),
                    },
                    created_by: None,
                    note: None,
                },
            )
            .await
            .unwrap();

        let repository: Arc<dyn RunRepositoryPort> =
            Arc::new(InMemoryRunRepository::new().with_assistants(Arc::clone(&assistants)));
        let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
        let service =
            RunSubmissionService::new(Arc::clone(&repository), queue, Arc::new(StubStoredResolver));

        let accepted = service
            .submit(SubmitRun {
                assistant_id: "stored-wf".to_string(),
                version: None,
                thread_id: None,
                input: serde_json::json!({}),
                webhook: None,
                requested_by: None,
            })
            .await
            .unwrap();

        let stored = repository.get(&accepted.run_id).await.unwrap().unwrap();
        assert_eq!(
            stored.assistant.version, 2,
            "the run must be frozen at the repository's own latest (2), not the \
             resolver's hardcoded fallback (1)"
        );
    }

    #[tokio::test]
    async fn submit_with_explicit_version_uses_a_pinned_insert_not_latest() {
        let assistants = Arc::new(InMemoryAssistantRepository::new());
        let id = AssistantId::new("stored-wf-pinned").unwrap();
        assistants
            .create(
                &id,
                NewAssistantVersion {
                    definition: AssistantDefinition {
                        kind: AssistantKind::Workflow,
                        body: serde_json::json!({}),
                    },
                    created_by: None,
                    note: None,
                },
            )
            .await
            .unwrap();
        assistants
            .append_version(
                &id,
                NewAssistantVersion {
                    definition: AssistantDefinition {
                        kind: AssistantKind::Workflow,
                        body: serde_json::json!({}),
                    },
                    created_by: None,
                    note: None,
                },
            )
            .await
            .unwrap();

        let repository: Arc<dyn RunRepositoryPort> =
            Arc::new(InMemoryRunRepository::new().with_assistants(Arc::clone(&assistants)));
        let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
        let service =
            RunSubmissionService::new(Arc::clone(&repository), queue, Arc::new(StubStoredResolver));

        let accepted = service
            .submit(SubmitRun {
                assistant_id: "stored-wf-pinned".to_string(),
                version: Some(1),
                thread_id: None,
                input: serde_json::json!({}),
                webhook: None,
                requested_by: None,
            })
            .await
            .unwrap();

        let stored = repository.get(&accepted.run_id).await.unwrap().unwrap();
        assert_eq!(
            stored.assistant.version, 1,
            "a pinned version: Some(1) must be inserted verbatim, ignoring the \
             assistant's later-published latest (2)"
        );
    }
}
