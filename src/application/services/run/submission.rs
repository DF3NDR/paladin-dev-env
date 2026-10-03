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
use paladin_core::platform::container::principal::{PrincipalRef, RunReadScope};
use paladin_core::platform::container::run::{ForkSpec, Run, RunId};
use paladin_core::platform::container::user::UserRole;
use paladin_core::platform::container::waypoint::ThreadId;
use paladin_ports::input::allowance_admission_port::{AdmissionError, AllowanceAdmissionPort};
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

/// Map an [`AllowanceAdmissionPort::admit`] failure (Phase 41): a refusal is the typed
/// [`RunSubmissionError::AllowanceExhausted`]; a backend failure stays a `Backend` error (the
/// request is not admitted -- fail closed, D-10). `AdmissionError` is a foreign
/// `#[non_exhaustive]` enum, so any future variant falls back to its display text.
fn map_admission_error(err: AdmissionError) -> RunSubmissionError {
    match err {
        AdmissionError::Refused(refusal) => RunSubmissionError::AllowanceExhausted(refusal),
        AdmissionError::Backend { message } => RunSubmissionError::Backend { message },
        other => RunSubmissionError::Backend {
            message: other.to_string(),
        },
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
///
/// # Examples
///
/// Constructing a `RunSubmissionService` from its port dependencies: the
/// shipped in-memory run repository, the shipped in-memory run queue, and
/// the shipped [`CodeWorkflowResolver`](super::resolver::CodeWorkflowResolver)
/// -- no separate `AssistantResolver` implementation needs writing here,
/// since this slice's own code-registered resolver already satisfies the
/// constructor's third argument.
///
/// ```
/// use std::sync::Arc;
///
/// use paladin::application::services::run::{CodeWorkflowResolver, RunSubmissionService};
/// use paladin_ports::input::run_submission_port::{RunSubmissionError, RunSubmissionPort, SubmitRun};
/// use paladin_storage::run::in_memory::InMemoryRunRepository;
/// use paladin_storage::run_queue::in_memory::InMemoryRunQueue;
///
/// #[tokio::main]
/// async fn main() {
///     let repository = Arc::new(InMemoryRunRepository::new());
///     let queue = Arc::new(InMemoryRunQueue::new());
///     let resolver = Arc::new(CodeWorkflowResolver::new());
///     let service = RunSubmissionService::new(repository, queue, resolver);
///
///     // No assistant is registered with the resolver, so `submit` fails
///     // fast at the resolve step -- proving the service is fully wired
///     // without a live backend.
///     let err = service
///         .submit(SubmitRun {
///             assistant_id: "unregistered-assistant".to_string(),
///             version: None,
///             thread_id: None,
///             input: serde_json::json!({}),
///             webhook: None,
///             requested_by: None,
///         })
///         .await
///         .unwrap_err();
///     assert!(matches!(err, RunSubmissionError::UnknownAssistant { .. }));
/// }
/// ```
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
    /// The Treasurer's admission check (Phase 41, D-06), wired via
    /// [`RunSubmissionService::with_treasurer`]. `None` (the default) means admission is a
    /// no-op.
    treasurer: Option<Arc<dyn AllowanceAdmissionPort>>,
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
            treasurer: None,
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

    /// Attach the Treasurer's admission check (Phase 41, D-06): `submit` asks it whether the
    /// caller's allowance is exhausted after the SSRF guard, `resolve`, role authorization and
    /// thread-visibility check, and before any run row is written. `None` (the default) means
    /// admission is a no-op; production wiring attaches one only when `treasurer.allowance` has
    /// entries.
    ///
    /// A request with no principal (`requested_by: None`) is never gated (D-07).
    pub fn with_treasurer(mut self, treasurer: Arc<dyn AllowanceAdmissionPort>) -> Self {
        self.treasurer = Some(treasurer);
        self
    }

    /// Wire a [`WaypointPort`] so [`RunSubmissionService::fork`] can
    /// validate that `from_waypoint_id` actually exists on the target
    /// thread before enqueuing a forked run (D-45).
    pub fn with_waypoints(mut self, waypoints: Arc<dyn WaypointPort>) -> Self {
        self.waypoints = Some(waypoints);
        self
    }

    /// Tenant guard for a caller-supplied Thread (phase 40 review WR-01).
    ///
    /// A thread carries no tenant of its own, but every run on it carries
    /// `submitted_by`. When `requested_by` is `Some`, the thread's latest run
    /// (looked up with the unrestricted `All` scope -- this is an internal
    /// authorization read, not a caller-facing one) must be permitted by the
    /// principal's [`RunReadScope`]; otherwise the caller is answered
    /// [`RunSubmissionError::UnknownThread`], the same value a `fork` against a
    /// thread with no runs yields, so a hidden thread is not distinguishable
    /// from a missing one on this path. A thread with no runs stays open, and a
    /// `None` principal (an internal caller) or an `All` scope (Admin) skips
    /// the lookup entirely.
    ///
    /// Known limitation: `submit` accepts a caller-chosen id for a thread that
    /// has no runs, so a Quest submit against an unused id still succeeds while
    /// a hidden thread is refused -- thread ids are unguessable in practice
    /// (UUIDv7 by default) and full tenant-namespacing of threads is the
    /// deferred thread-tenancy work (40-CONTEXT D-14).
    async fn ensure_thread_visible(
        &self,
        thread_id: &ThreadId,
        requested_by: &Option<PrincipalRef>,
    ) -> Result<(), RunSubmissionError> {
        let Some(principal_ref) = requested_by else {
            return Ok(());
        };
        let scope = RunReadScope::for_principal(principal_ref.role, &principal_ref.tenant_id);
        if scope == RunReadScope::All {
            return Ok(());
        }
        let page = self
            .repository
            .list(RunQuery {
                thread_id: Some(thread_id.clone()),
                limit: 1,
                ..Default::default()
            })
            .await
            .map_err(map_repository_error)?;
        match page.items.into_iter().next() {
            Some(latest) if !scope.permits(&latest) => Err(RunSubmissionError::UnknownThread {
                thread_id: thread_id.clone(),
            }),
            _ => Ok(()),
        }
    }

    /// D-46: authorize an invocation-shaped request (`submit`/`cancel`/
    /// `fork`) against `allowed_roles` -- empty means any authenticated
    /// caller, `None` `requested_by` skips the check entirely (an
    /// internal/same-process caller with no principal to authorize
    /// against). Shared by all three call sites so the rule is expressed
    /// exactly once.
    fn authorize_invocation(
        requested_by: &Option<PrincipalRef>,
        allowed_roles: &[UserRole],
    ) -> Result<(), RunSubmissionError> {
        let Some(principal_ref) = requested_by else {
            return Ok(());
        };
        if allowed_roles.is_empty() || allowed_roles.contains(&principal_ref.role) {
            Ok(())
        } else {
            Err(RunSubmissionError::Forbidden {
                reason: "role not permitted for this assistant".to_string(),
            })
        }
    }

    /// Insert `run` (freezing `latest` when `use_latest`) and enqueue it -- the two writes
    /// `submit` makes after admission, kept together so `submit` can confirm or abandon the
    /// admission on the one result.
    async fn persist_and_enqueue(
        &self,
        run: &mut Run,
        use_latest: bool,
    ) -> Result<(), RunSubmissionError> {
        if use_latest {
            let resolved_version = self
                .repository
                .insert_with_latest(run)
                .await
                .map_err(map_repository_error)?;
            run.assistant.version = resolved_version;
        } else {
            self.repository
                .insert(run)
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
        Ok(())
    }

    /// The one admission lifecycle `submit` and `fork` share (Phase 41, D-06, D-07, D-10):
    /// ask the Treasurer BEFORE any row is written, persist, then `confirm` on success or
    /// `abandon` on a persistence failure, so a run that was admitted but never enqueued does
    /// not hold an allowance notice.
    ///
    /// A request with no principal (`run.submitted_by` is `None`, an internal caller) or a
    /// service without a Treasurer is never gated and never touches `confirm` / `abandon`.
    /// A refusal returns [`RunSubmissionError::AllowanceExhausted`]; any other admission error
    /// returns [`RunSubmissionError::Backend`] (fail closed) -- in both cases nothing is
    /// persisted. Admission is a check only (D-05): the role never reaches the Treasurer, so an
    /// `Admin` principal is bound exactly like a `User` (D-09).
    async fn admit_and_persist(
        &self,
        run: &mut Run,
        use_latest: bool,
    ) -> Result<(), RunSubmissionError> {
        let admission = match (&self.treasurer, &run.submitted_by) {
            (Some(treasurer), Some(subject)) => Some(
                treasurer
                    .admit(subject, Some(&run.run_id))
                    .await
                    .map_err(map_admission_error)?,
            ),
            _ => None,
        };

        let persisted = self.persist_and_enqueue(run, use_latest).await;

        if let (Some(treasurer), Some(admission)) = (&self.treasurer, &admission) {
            match &persisted {
                Ok(()) => treasurer.confirm(admission).await,
                Err(_) => treasurer.abandon(admission).await,
            }
        }
        persisted
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

        // WR-01: a caller-supplied thread must not belong to another tenant.
        if let Some(supplied) = &request.thread_id {
            self.ensure_thread_visible(supplied, &request.requested_by)
                .await?;
        }

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
        // D-08: stamp the submitting principal's attribution onto the run BEFORE it is
        // ever inserted -- `None` (an internal/same-process caller) leaves `submitted_by`
        // `None` (D-10).
        if let Some(principal_ref) = &request.requested_by {
            run = run.with_submitted_by(principal_ref.attribution());
        }

        // Phase 41 D-06/D-07: the allowance check, BEFORE any row is written, with confirm /
        // abandon on the persistence result -- the lifecycle `fork` shares.
        self.admit_and_persist(&mut run, use_latest).await?;

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
        requested_by: Option<PrincipalRef>,
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
            // WR-02: enforce the tenant scope here, not only in the HTTP
            // controller -- any other adapter or embedder passing a fully
            // attributed principal gets the same isolation. A run the principal
            // may not see is answered exactly like a missing one.
            if let Some(principal_ref) = &requested_by
                && !RunReadScope::for_principal(principal_ref.role, &principal_ref.tenant_id)
                    .permits(&run)
            {
                return Err(RunSubmissionError::NotFound {
                    run_id: run_id.clone(),
                });
            }
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
    /// [`RunSubmissionPort::submit`] enforces, and to the same Treasurer allowance check
    /// (Phase 41, D-06): an exhausted principal is refused with
    /// [`RunSubmissionError::AllowanceExhausted`] before the fork's run row is written.
    async fn fork(&self, request: ForkRun) -> Result<RunAccepted, RunSubmissionError> {
        // D-42: same write-time SSRF guard `submit` runs.
        if let Some(webhook) = &request.webhook
            && let Err(rejection) = self.ssrf_guard.check_url(&webhook.url).await
        {
            return Err(RunSubmissionError::WebhookRejected {
                reason: rejection.to_string(),
            });
        }

        // WR-01: run the tenant guard first, so neither the busy-thread nor the
        // waypoint checks below can confirm another tenant's thread exists.
        self.ensure_thread_visible(&request.thread_id, &request.requested_by)
            .await?;

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
        if let Some(principal_ref) = &request.requested_by {
            run = run.with_submitted_by(principal_ref.attribution());
        }
        if let Some(webhook) = request.webhook {
            run = run.with_webhook(webhook);
        }

        // Phase 41 D-06/D-07: a fork starts spend exactly like a submit, so it runs the same
        // admit -> insert -> enqueue -> confirm / abandon lifecycle. The fork's run row pins the
        // assistant version it copied, so `use_latest` is `false` (a plain `insert`).
        self.admit_and_persist(&mut run, false).await?;

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

    // --- Phase 40 (TENANT-01/02, D-08): principal attribution stamped on submit ---

    fn principal_ref(tenant: &str, api_key_id: &str, role: UserRole) -> PrincipalRef {
        PrincipalRef::new(
            api_key_id,
            paladin_core::platform::container::principal::TenantId::new(tenant).unwrap(),
            role,
        )
    }

    #[tokio::test]
    async fn submit_records_the_requesting_principal_attribution() {
        let resolver = CodeWorkflowResolver::new().register("wf1", empty_graph());
        let (service, repository, _queue) = service_with(resolver);

        let accepted = service
            .submit(SubmitRun {
                assistant_id: "wf1".to_string(),
                version: None,
                thread_id: None,
                input: serde_json::json!({}),
                webhook: None,
                requested_by: Some(principal_ref("acme", "svc-a", UserRole::User)),
            })
            .await
            .unwrap();

        let stored = repository.get(&accepted.run_id).await.unwrap().unwrap();
        assert_eq!(
            stored.submitted_by,
            Some(
                paladin_core::platform::container::principal::RunAttribution::new(
                    paladin_core::platform::container::principal::TenantId::new("acme").unwrap(),
                    "svc-a",
                )
            )
        );
    }

    #[tokio::test]
    async fn submit_without_a_principal_records_no_attribution() {
        let resolver = CodeWorkflowResolver::new().register("wf1", empty_graph());
        let (service, repository, _queue) = service_with(resolver);

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

        let stored = repository.get(&accepted.run_id).await.unwrap().unwrap();
        assert!(stored.submitted_by.is_none());
    }

    #[tokio::test]
    async fn two_submissions_by_one_principal_carry_identical_attribution() {
        let resolver = CodeWorkflowResolver::new().register("wf1", empty_graph());
        let (service, repository, _queue) = service_with(resolver);
        let principal = principal_ref("acme", "svc-a", UserRole::User);

        let first = service
            .submit(SubmitRun {
                assistant_id: "wf1".to_string(),
                version: None,
                thread_id: None,
                input: serde_json::json!({}),
                webhook: None,
                requested_by: Some(principal.clone()),
            })
            .await
            .unwrap();
        let second = service
            .submit(SubmitRun {
                assistant_id: "wf1".to_string(),
                version: None,
                thread_id: None,
                input: serde_json::json!({}),
                webhook: None,
                requested_by: Some(principal),
            })
            .await
            .unwrap();

        let first_stored = repository.get(&first.run_id).await.unwrap().unwrap();
        let second_stored = repository.get(&second.run_id).await.unwrap().unwrap();
        assert_eq!(first_stored.submitted_by, second_stored.submitted_by);
    }

    #[tokio::test]
    async fn submit_forbidden_role_is_read_from_the_principal_ref() {
        struct StubRoleRestrictedResolver;

        #[async_trait]
        impl AssistantResolver for StubRoleRestrictedResolver {
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
                    allowed_roles: vec![UserRole::Admin],
                    source: AssistantSource::Code,
                })
            }
        }

        let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
        let service =
            RunSubmissionService::new(repository, queue, Arc::new(StubRoleRestrictedResolver));

        let err = service
            .submit(SubmitRun {
                assistant_id: "wf1".to_string(),
                version: None,
                thread_id: None,
                input: serde_json::json!({}),
                webhook: None,
                requested_by: Some(principal_ref("acme", "svc-a", UserRole::User)),
            })
            .await
            .unwrap_err();
        assert!(matches!(err, RunSubmissionError::Forbidden { .. }));
    }

    // --- Phase 40 review-fix (WR-01/WR-02): tenant isolation on the write paths ---

    fn submit_on_thread(thread: &str, requested_by: Option<PrincipalRef>) -> SubmitRun {
        SubmitRun {
            assistant_id: "wf1".to_string(),
            version: None,
            thread_id: Some(ThreadId::new(thread).unwrap()),
            input: serde_json::json!({}),
            webhook: None,
            requested_by,
        }
    }

    /// WR-01: a `User` principal of another tenant must not be able to run
    /// against a Thread whose latest run belongs to a different tenant, and the
    /// rejection is the SAME `UnknownThread` a fork against a thread with no
    /// runs answers -- never `ThreadBusy`, which would confirm the thread
    /// exists.
    #[tokio::test]
    async fn submit_on_another_tenants_thread_is_rejected_as_unknown_thread() {
        let resolver = CodeWorkflowResolver::new().register("wf1", empty_graph());
        let (service, repository, queue) = service_with(resolver);

        service
            .submit(submit_on_thread(
                "acme-thread",
                Some(principal_ref("acme", "svc-a", UserRole::User)),
            ))
            .await
            .unwrap();

        let err = service
            .submit(submit_on_thread(
                "acme-thread",
                Some(principal_ref("globex", "svc-b", UserRole::User)),
            ))
            .await
            .unwrap_err();
        assert!(
            matches!(err, RunSubmissionError::UnknownThread { .. }),
            "cross-tenant thread submit must look like an unknown thread, got {err:?}"
        );
        assert_eq!(queue.depth().await.unwrap(), 1, "nothing may be enqueued");
        let all = repository
            .list(RunQuery {
                limit: 10,
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(all.items.len(), 1, "nothing may be inserted");
    }

    /// WR-01 controls: the owning tenant, an Admin, an internal (`None`)
    /// caller, and a thread with no runs are all NOT blocked by the tenant
    /// guard (the owner and Admin reach the ordinary busy-thread check).
    #[tokio::test]
    async fn submit_thread_tenant_guard_admits_owner_admin_internal_and_fresh_threads() {
        let resolver = CodeWorkflowResolver::new().register("wf1", empty_graph());
        let (service, _repository, _queue) = service_with(resolver);

        service
            .submit(submit_on_thread(
                "acme-thread",
                Some(principal_ref("acme", "svc-a", UserRole::User)),
            ))
            .await
            .unwrap();

        for requester in [
            Some(principal_ref("acme", "svc-a2", UserRole::User)),
            Some(principal_ref("ops-tenant", "ops", UserRole::Admin)),
            None,
        ] {
            let err = service
                .submit(submit_on_thread("acme-thread", requester))
                .await
                .unwrap_err();
            assert!(
                matches!(err, RunSubmissionError::ThreadBusy { .. }),
                "owner/admin/internal must pass the tenant guard and reach the busy check, got {err:?}"
            );
        }

        // A thread with no runs stays open to any principal.
        service
            .submit(submit_on_thread(
                "fresh-thread",
                Some(principal_ref("globex", "svc-b", UserRole::User)),
            ))
            .await
            .unwrap();
    }

    /// WR-01 (fork): a `User` of another tenant forking a thread whose latest
    /// run is not theirs gets the uniform `UnknownThread`, before the
    /// busy-thread / waypoint checks can confirm the thread exists.
    #[tokio::test]
    async fn fork_on_another_tenants_thread_is_rejected_as_unknown_thread() {
        let resolver = CodeWorkflowResolver::new().register("wf1", empty_graph());
        let (service, _repository, queue) = service_with(resolver);

        service
            .submit(submit_on_thread(
                "acme-thread",
                Some(principal_ref("acme", "svc-a", UserRole::User)),
            ))
            .await
            .unwrap();

        let fork_as = |requested_by: Option<PrincipalRef>| ForkRun {
            thread_id: ThreadId::new("acme-thread").unwrap(),
            from_waypoint_id: paladin_core::platform::container::waypoint::WaypointId::new(),
            edit: None,
            webhook: None,
            requested_by,
        };

        let err = service
            .fork(fork_as(Some(principal_ref(
                "globex",
                "svc-b",
                UserRole::User,
            ))))
            .await
            .unwrap_err();
        assert!(
            matches!(err, RunSubmissionError::UnknownThread { .. }),
            "cross-tenant fork must look like an unknown thread, got {err:?}"
        );
        assert_eq!(queue.depth().await.unwrap(), 1, "nothing may be enqueued");

        // Control: the owning tenant passes the guard and reaches the ordinary
        // busy-thread check (the thread's only run is still active).
        let err = service
            .fork(fork_as(Some(principal_ref(
                "acme",
                "svc-a2",
                UserRole::User,
            ))))
            .await
            .unwrap_err();
        assert!(matches!(err, RunSubmissionError::ThreadBusy { .. }));
    }

    /// WR-02: `cancel` enforces the tenant scope itself -- another tenant's
    /// `User` gets the same `NotFound` a missing run yields, and the durable
    /// cancel flag is never written.
    #[tokio::test]
    async fn cancel_of_another_tenants_run_is_not_found_and_writes_nothing() {
        let resolver = CodeWorkflowResolver::new().register("wf1", empty_graph());
        let (service, repository, _queue) = service_with(resolver);

        let accepted = service
            .submit(submit_on_thread(
                "acme-thread",
                Some(principal_ref("acme", "svc-a", UserRole::User)),
            ))
            .await
            .unwrap();

        let err = service
            .cancel(
                &accepted.run_id,
                Some(principal_ref("globex", "svc-b", UserRole::User)),
            )
            .await
            .unwrap_err();
        assert!(
            matches!(&err, RunSubmissionError::NotFound { run_id } if *run_id == accepted.run_id),
            "cross-tenant cancel must look like a missing run, got {err:?}"
        );
        let stored = repository.get(&accepted.run_id).await.unwrap().unwrap();
        assert!(
            !stored.cancel_requested,
            "a rejected cancel must not persist the cancel flag"
        );
    }

    /// WR-02 controls: the owning tenant and an Admin can cancel.
    #[tokio::test]
    async fn cancel_admits_the_owning_tenant_and_admin() {
        let resolver = CodeWorkflowResolver::new().register("wf1", empty_graph());
        let (service, repository, _queue) = service_with(resolver);

        let first = service
            .submit(submit_on_thread(
                "t-own",
                Some(principal_ref("acme", "svc-a", UserRole::User)),
            ))
            .await
            .unwrap();
        service
            .cancel(
                &first.run_id,
                Some(principal_ref("acme", "svc-a2", UserRole::User)),
            )
            .await
            .unwrap();
        assert!(
            repository
                .get(&first.run_id)
                .await
                .unwrap()
                .unwrap()
                .cancel_requested
        );

        let second = service
            .submit(submit_on_thread(
                "t-admin",
                Some(principal_ref("acme", "svc-a", UserRole::User)),
            ))
            .await
            .unwrap();
        service
            .cancel(
                &second.run_id,
                Some(principal_ref("ops-tenant", "ops", UserRole::Admin)),
            )
            .await
            .unwrap();
    }

    // --- Phase 41 (41-01): the Treasurer admission slot --------------------

    use paladin_core::platform::container::allowance::{
        Admission, AllowanceLimitKind, AllowanceRefusal, AllowanceScopeKind,
    };
    use paladin_core::platform::container::cost::{Cost, CurrencyCode};
    use std::sync::Mutex;

    /// What a [`RecordingTreasurer`] answers to `admit`.
    enum Script {
        Admit,
        Refuse,
        Backend,
    }

    /// An `AllowanceAdmissionPort` double that returns a scripted result and records every
    /// `admit` / `confirm` / `abandon` call.
    struct RecordingTreasurer {
        script: Script,
        admits: Mutex<u32>,
        confirms: Mutex<u32>,
        abandons: Mutex<u32>,
        last_run_id: Mutex<Option<RunId>>,
    }

    impl RecordingTreasurer {
        fn new(script: Script) -> Arc<Self> {
            Arc::new(Self {
                script,
                admits: Mutex::new(0),
                confirms: Mutex::new(0),
                abandons: Mutex::new(0),
                last_run_id: Mutex::new(None),
            })
        }

        /// The run id the most recent `admit` was called with.
        fn last_run_id(&self) -> Option<RunId> {
            self.last_run_id.lock().unwrap().clone()
        }

        fn counts(&self) -> (u32, u32, u32) {
            (
                *self.admits.lock().unwrap(),
                *self.confirms.lock().unwrap(),
                *self.abandons.lock().unwrap(),
            )
        }
    }

    #[async_trait]
    impl AllowanceAdmissionPort for RecordingTreasurer {
        async fn admit(
            &self,
            _subject: &paladin_core::platform::container::principal::RunAttribution,
            run_id: Option<&RunId>,
        ) -> Result<Admission, AdmissionError> {
            *self.admits.lock().unwrap() += 1;
            *self.last_run_id.lock().unwrap() = run_id.cloned();
            match self.script {
                Script::Admit => Ok(Admission::none()),
                Script::Refuse => {
                    let usd = CurrencyCode::new("USD").unwrap();
                    Err(AdmissionError::Refused(AllowanceRefusal {
                        scope_kind: AllowanceScopeKind::ApiKey,
                        limit_kind: AllowanceLimitKind::Lifetime,
                        balance: Cost::new(10, usd.clone()),
                        ceiling: Cost::new(10, usd),
                        window: None,
                        evaluated_at: Utc::now(),
                    }))
                }
                Script::Backend => Err(AdmissionError::Backend {
                    message: "ledger down".to_string(),
                }),
            }
        }

        async fn confirm(&self, _admission: &Admission) {
            *self.confirms.lock().unwrap() += 1;
        }

        async fn abandon(&self, _admission: &Admission) {
            *self.abandons.lock().unwrap() += 1;
        }
    }

    fn submit_as(thread: Option<&str>, requested_by: Option<PrincipalRef>) -> SubmitRun {
        SubmitRun {
            assistant_id: "wf1".to_string(),
            version: None,
            thread_id: thread.map(|t| ThreadId::new(t).unwrap()),
            input: serde_json::json!({}),
            webhook: None,
            requested_by,
        }
    }

    #[tokio::test]
    async fn submit_refused_by_the_treasurer_touches_nothing() {
        let resolver = CodeWorkflowResolver::new().register("wf1", empty_graph());
        let (service, repository, queue) = service_with(resolver);
        let treasurer = RecordingTreasurer::new(Script::Refuse);
        let service = service.with_treasurer(treasurer.clone());

        let err = service
            .submit(submit_as(
                None,
                Some(principal_ref("acme", "svc-a", UserRole::User)),
            ))
            .await
            .unwrap_err();

        assert!(
            matches!(err, RunSubmissionError::AllowanceExhausted(_)),
            "got {err:?}"
        );
        assert_eq!(queue.depth().await.unwrap(), 0);
        assert!(
            repository
                .list(RunQuery::default())
                .await
                .unwrap()
                .items
                .is_empty()
        );
        assert_eq!(
            treasurer.counts(),
            (1, 0, 0),
            "a refusal is never confirmed"
        );
    }

    #[tokio::test]
    async fn submit_without_a_principal_never_calls_the_treasurer() {
        let resolver = CodeWorkflowResolver::new().register("wf1", empty_graph());
        let (service, repository, _queue) = service_with(resolver);
        let treasurer = RecordingTreasurer::new(Script::Refuse);
        let service = service.with_treasurer(treasurer.clone());

        let accepted = service.submit(submit_as(None, None)).await.unwrap();

        assert!(repository.get(&accepted.run_id).await.unwrap().is_some());
        assert_eq!(treasurer.counts(), (0, 0, 0));
    }

    #[tokio::test]
    async fn submit_admitted_calls_confirm_after_enqueue() {
        let resolver = CodeWorkflowResolver::new().register("wf1", empty_graph());
        let (service, repository, queue) = service_with(resolver);
        let treasurer = RecordingTreasurer::new(Script::Admit);
        let service = service.with_treasurer(treasurer.clone());

        let accepted = service
            .submit(submit_as(
                None,
                Some(principal_ref("acme", "svc-a", UserRole::User)),
            ))
            .await
            .unwrap();

        assert!(repository.get(&accepted.run_id).await.unwrap().is_some());
        assert_eq!(queue.depth().await.unwrap(), 1);
        assert_eq!(treasurer.counts(), (1, 1, 0));
    }

    #[tokio::test]
    async fn submit_admission_backend_error_fails_closed_as_backend() {
        let resolver = CodeWorkflowResolver::new().register("wf1", empty_graph());
        let (service, repository, queue) = service_with(resolver);
        let treasurer = RecordingTreasurer::new(Script::Backend);
        let service = service.with_treasurer(treasurer.clone());

        let err = service
            .submit(submit_as(
                None,
                Some(principal_ref("acme", "svc-a", UserRole::User)),
            ))
            .await
            .unwrap_err();

        match err {
            RunSubmissionError::Backend { message } => assert_eq!(message, "ledger down"),
            other => panic!("expected a Backend error, got {other:?}"),
        }
        assert_eq!(queue.depth().await.unwrap(), 0);
        assert!(
            repository
                .list(RunQuery::default())
                .await
                .unwrap()
                .items
                .is_empty()
        );
    }

    #[tokio::test]
    async fn submit_insert_failure_after_admission_calls_abandon() {
        let resolver = CodeWorkflowResolver::new().register("wf1", empty_graph());
        let (service, _repository, _queue) = service_with(resolver);
        let treasurer = RecordingTreasurer::new(Script::Admit);
        let service = service.with_treasurer(treasurer.clone());
        let principal = || Some(principal_ref("acme", "svc-a", UserRole::User));

        service
            .submit(submit_as(Some("busy-thread"), principal()))
            .await
            .unwrap();
        let err = service
            .submit(submit_as(Some("busy-thread"), principal()))
            .await
            .unwrap_err();

        assert!(
            matches!(err, RunSubmissionError::ThreadBusy { .. }),
            "{err:?}"
        );
        // First submit: admit + confirm. Second: admit, then the busy insert, then abandon.
        assert_eq!(treasurer.counts(), (2, 1, 1));
    }

    // --- Phase 41 (41-04): fork admission and the Admin binding -------------

    use paladin_core::platform::container::battlefield::Battlefield;
    use paladin_core::platform::container::run::RunStatus;
    use paladin_core::platform::container::waypoint::{
        FrontierSnapshot, GraphFingerprint, Waypoint, WaypointId, WaypointStatus,
    };
    use paladin_ports::output::waypoint_port::WaypointPort;
    use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;
    use std::collections::BTreeMap;

    /// A thread with one completed run and one saved Waypoint -- everything `fork` needs to
    /// reach its admission slot. `service` builds a fresh [`RunSubmissionService`] over the same
    /// repository / queue / waypoints with an optional Treasurer double.
    struct ForkFixture {
        repository: Arc<dyn RunRepositoryPort>,
        queue: Arc<dyn RunQueuePort>,
        resolver: Arc<dyn AssistantResolver>,
        waypoints: Arc<dyn WaypointPort>,
        thread: ThreadId,
        from_waypoint: WaypointId,
    }

    impl ForkFixture {
        async fn new() -> Self {
            let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
            let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
            let resolver: Arc<dyn AssistantResolver> =
                Arc::new(CodeWorkflowResolver::new().register("wf1", empty_graph()));
            let waypoints: Arc<dyn WaypointPort> = Arc::new(InMemoryWaypointStore::new());
            let fixture = Self {
                repository,
                queue,
                resolver,
                waypoints,
                thread: ThreadId::new("fork-thread").unwrap(),
                from_waypoint: WaypointId::generate(),
            };

            // An unattributed original run, driven to `Completed` so the thread is not busy.
            let accepted = fixture
                .service(None)
                .submit(submit_as(Some("fork-thread"), None))
                .await
                .unwrap();
            for (from, to) in [
                (RunStatus::Queued, RunStatus::Running),
                (RunStatus::Running, RunStatus::Completed),
            ] {
                fixture
                    .repository
                    .update_status(&accepted.run_id, from, to, Utc::now())
                    .await
                    .unwrap();
            }

            let mut waypoint = Waypoint::new_root(
                fixture.thread.clone(),
                1,
                GraphFingerprint::from_canonical_bytes(b"submission-fork-test-graph"),
                Battlefield::new(BattlefieldSchema::new(Vec::new())),
                Vec::new(),
                Vec::new(),
                WaypointStatus::Completed,
                BTreeMap::new(),
                FrontierSnapshot::default(),
            );
            waypoint.waypoint_id = fixture.from_waypoint;
            fixture.waypoints.save(&waypoint).await.unwrap();
            fixture
        }

        fn service(&self, treasurer: Option<Arc<RecordingTreasurer>>) -> RunSubmissionService {
            let service = RunSubmissionService::new(
                self.repository.clone(),
                self.queue.clone(),
                self.resolver.clone(),
            )
            .with_waypoints(self.waypoints.clone());
            match treasurer {
                Some(treasurer) => service.with_treasurer(treasurer),
                None => service,
            }
        }

        /// A fork of the fixture's thread by `requested_by`.
        fn fork_by(&self, requested_by: Option<PrincipalRef>) -> ForkRun {
            ForkRun {
                thread_id: self.thread.clone(),
                from_waypoint_id: self.from_waypoint,
                edit: None,
                webhook: None,
                requested_by,
            }
        }

        async fn run_count(&self) -> usize {
            self.repository
                .list(RunQuery::default())
                .await
                .unwrap()
                .items
                .len()
        }
    }

    /// An `Admin` of `acme` -- the original run is unattributed, so only an `Admin`
    /// (`RunReadScope::All`) may fork it (the Phase 40 tenant guard).
    fn acme_admin() -> PrincipalRef {
        principal_ref("acme", "ops", UserRole::Admin)
    }

    #[tokio::test]
    async fn fork_by_an_exhausted_principal_is_refused_and_touches_nothing() {
        let fixture = ForkFixture::new().await;
        let depth_before = fixture.queue.depth().await.unwrap();
        let runs_before = fixture.run_count().await;
        let treasurer = RecordingTreasurer::new(Script::Refuse);
        let service = fixture.service(Some(treasurer.clone()));

        let err = service
            .fork(fixture.fork_by(Some(acme_admin())))
            .await
            .unwrap_err();

        assert!(
            matches!(err, RunSubmissionError::AllowanceExhausted(_)),
            "got {err:?}"
        );
        assert_eq!(fixture.queue.depth().await.unwrap(), depth_before);
        assert_eq!(fixture.run_count().await, runs_before, "no run is inserted");
        assert_eq!(
            treasurer.counts(),
            (1, 0, 0),
            "a refusal is never confirmed"
        );
    }

    #[tokio::test]
    async fn fork_admitted_confirms_after_enqueue() {
        let fixture = ForkFixture::new().await;
        let depth_before = fixture.queue.depth().await.unwrap();
        let treasurer = RecordingTreasurer::new(Script::Admit);
        let service = fixture.service(Some(treasurer.clone()));

        let accepted = service
            .fork(fixture.fork_by(Some(acme_admin())))
            .await
            .unwrap();

        let stored = fixture.repository.get(&accepted.run_id).await.unwrap();
        assert!(stored.is_some(), "the fork's run row is persisted");
        assert_eq!(fixture.queue.depth().await.unwrap(), depth_before + 1);
        assert_eq!(treasurer.counts(), (1, 1, 0));
        assert_eq!(
            treasurer.last_run_id(),
            Some(accepted.run_id),
            "admit is called with the new run's id"
        );
    }

    #[tokio::test]
    async fn fork_failing_after_admission_calls_abandon_once() {
        let fixture = ForkFixture::new().await;
        let treasurer = RecordingTreasurer::new(Script::Admit);
        let service = fixture.service(Some(treasurer.clone()));
        let depth_before = fixture.queue.depth().await.unwrap();

        // The first fork is accepted and leaves a Queued run on the thread. A second run
        // handed straight to the admit-then-persist lifecycle (as if it had passed `fork`'s own
        // busy check a moment earlier -- the insert race) is refused by the repository's
        // one-active-run-per-thread invariant AFTER admission.
        service
            .fork(fixture.fork_by(Some(acme_admin())))
            .await
            .unwrap();
        let first_fork = fixture
            .repository
            .list(RunQuery::default())
            .await
            .unwrap()
            .items
            .into_iter()
            .next()
            .expect("a run exists");
        let mut racing = Run::new(
            RunId::new_v7(),
            fixture.thread.clone(),
            first_fork.assistant,
            serde_json::json!({}),
        )
        .with_submitted_by(acme_admin().attribution());
        let err = service
            .admit_and_persist(&mut racing, false)
            .await
            .unwrap_err();

        assert!(
            matches!(err, RunSubmissionError::ThreadBusy { .. }),
            "{err:?}"
        );
        // fork #1: admit + confirm; the failed persist: admit + abandon.
        assert_eq!(treasurer.counts(), (2, 1, 1));
        assert_eq!(fixture.queue.depth().await.unwrap(), depth_before + 1);
        assert!(
            fixture
                .repository
                .get(&racing.run_id)
                .await
                .unwrap()
                .is_none(),
            "the failed insert persists no run"
        );
    }

    #[tokio::test]
    async fn fork_without_a_principal_never_calls_the_treasurer() {
        let fixture = ForkFixture::new().await;
        let treasurer = RecordingTreasurer::new(Script::Refuse);
        let service = fixture.service(Some(treasurer.clone()));

        let accepted = service.fork(fixture.fork_by(None)).await.unwrap();

        assert!(
            fixture
                .repository
                .get(&accepted.run_id)
                .await
                .unwrap()
                .is_some()
        );
        assert_eq!(treasurer.counts(), (0, 0, 0));
    }

    /// D-09: the Treasurer receives a `RunAttribution` only, so an `Admin` principal with a
    /// configured allowance is refused exactly like a `User` -- through a REAL `Treasurer` over
    /// an in-memory ledger, not a scripted double. The ceiling is a lifetime cap (alongside the
    /// window) so a window boundary crossed mid-test cannot make the balance read zero.
    #[tokio::test]
    async fn admin_principal_is_bound_by_its_allowance() {
        use crate::application::services::treasurer::Treasurer;
        use crate::config::treasurer::TreasurerConfig;
        use paladin_core::platform::container::treasury_ledger::{LedgerScope, SettlementKey};
        use paladin_ports::output::treasury_ledger_port::TreasuryLedgerPort;
        use paladin_storage::treasury::contract_tests::{settle_request, usd};
        use paladin_storage::treasury::in_memory::InMemoryTreasuryLedger;

        let resolver = CodeWorkflowResolver::new().register("wf1", empty_graph());
        let (service, repository, queue) = service_with(resolver);
        let ledger: Arc<dyn TreasuryLedgerPort> = Arc::new(InMemoryTreasuryLedger::new());
        let config: TreasurerConfig = serde_json::from_value(serde_json::json!({
            "currency": "USD",
            "allowance": { "api_keys": { "ops": {
                "period": "1d", "amount": "2.50", "lifetime": "2.50"
            } } }
        }))
        .unwrap();
        let treasurer = Treasurer::new(config.allowance_policy().unwrap(), Arc::clone(&ledger));
        let service = service.with_treasurer(Arc::new(treasurer));

        // `ops` of `acme` has already spent its whole allowance.
        ledger
            .settle(settle_request(
                LedgerScope::new("acme", "ops"),
                SettlementKey::new(RunId::new_v7(), 0, 0),
                2_500_000_000,
                usd(),
                "gpt-4",
            ))
            .await
            .unwrap();

        let err = service
            .submit(submit_as(
                None,
                Some(principal_ref("acme", "ops", UserRole::Admin)),
            ))
            .await
            .unwrap_err();

        assert!(
            matches!(err, RunSubmissionError::AllowanceExhausted(_)),
            "an Admin is bound by its allowance, got {err:?}"
        );
        assert_eq!(queue.depth().await.unwrap(), 0);
        assert!(
            repository
                .list(RunQuery::default())
                .await
                .unwrap()
                .items
                .is_empty()
        );
    }
}
