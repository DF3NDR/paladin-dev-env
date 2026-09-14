/*
In-Memory Run Repository

An `Arc<tokio::sync::RwLock<HashMap<RunId, Run>>>`-backed implementation of
`RunRepositoryPort`, for tests and local development (D-01's InMemory
convention). Every status write routes through `RunStatus::try_transition`
so this adapter and a future SQL adapter agree on legality from day one
(T-27-01), and `insert` rejects with `ThreadBusy` under its write lock
whenever the target thread already has a run in an active status -- the
in-memory twin of the future partial unique index (D-17/D-18).
*/

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

use paladin_core::platform::container::assistant::AssistantId;
use paladin_core::platform::container::parley::ParleyResponse;
use paladin_core::platform::container::run::{RUN_SCHEMA_VERSION, Run, RunId, RunStatus};
use paladin_core::platform::container::waypoint::ThreadId;
use paladin_ports::output::run_repository_port::{
    RunOutcomeRecord, RunPage, RunQuery, RunRepositoryError, RunRepositoryPort,
};

use crate::assistant::in_memory::InMemoryAssistantRepository;

/// In-memory `RunRepositoryPort` implementation.
///
/// Cloning is cheap and shares the same underlying store (the inner `Arc` is
/// cloned).
#[derive(Clone, Default)]
pub struct InMemoryRunRepository {
    runs: Arc<RwLock<HashMap<RunId, Run>>>,
    /// Wired by [`InMemoryRunRepository::with_assistants`]; when present,
    /// [`RunRepositoryPort::insert_with_latest`] resolves the real D-30
    /// freeze-at-submit behavior instead of the port's default (verbatim
    /// insert) implementation.
    assistants: Option<Arc<InMemoryAssistantRepository>>,
}

impl InMemoryRunRepository {
    /// Construct a new, empty repository, with no assistant repository
    /// wired in (so `insert_with_latest` falls back to the port's default,
    /// verbatim-insert implementation).
    pub fn new() -> Self {
        Self::default()
    }

    /// Wire in an assistant repository so `insert_with_latest` resolves and
    /// freezes the real `latest` version at submit time (D-30), rather than
    /// falling back to the port's default implementation.
    ///
    /// Lock order: the assistants repository's own read lock is always
    /// acquired and released BEFORE this repository's write lock is taken
    /// in [`RunRepositoryPort::insert_with_latest`] — never the reverse —
    /// so a concurrent `AssistantRepositoryPort::append_version` call
    /// (which takes the assistants write lock for its whole mutation) is
    /// observed either strictly before or strictly after `latest` is read
    /// here, never torn.
    pub fn with_assistants(mut self, assistants: Arc<InMemoryAssistantRepository>) -> Self {
        self.assistants = Some(assistants);
        self
    }
}

#[async_trait]
impl RunRepositoryPort for InMemoryRunRepository {
    async fn insert(&self, run: &Run) -> Result<(), RunRepositoryError> {
        let mut runs = self.runs.write().await;
        if runs
            .values()
            .any(|r| r.thread_id == run.thread_id && r.status.is_active())
        {
            return Err(RunRepositoryError::ThreadBusy {
                thread_id: run.thread_id.clone(),
            });
        }
        runs.insert(run.run_id.clone(), run.clone());
        Ok(())
    }

    async fn get(&self, run_id: &RunId) -> Result<Option<Run>, RunRepositoryError> {
        let run = self.runs.read().await.get(run_id).cloned();
        match run {
            // X-04: a row whose schema_version this build does not
            // recognize must fail loudly rather than silently misparse --
            // the SQL adapters enforce the identical check on read.
            Some(run) if run.schema_version != RUN_SCHEMA_VERSION => {
                Err(RunRepositoryError::UnknownSchemaVersion {
                    found: run.schema_version,
                })
            }
            other => Ok(other),
        }
    }

    async fn update_status(
        &self,
        run_id: &RunId,
        from: RunStatus,
        to: RunStatus,
        at: DateTime<Utc>,
    ) -> Result<(), RunRepositoryError> {
        let mut runs = self.runs.write().await;
        let run = runs
            .get_mut(run_id)
            .ok_or_else(|| RunRepositoryError::NotFound {
                run_id: run_id.clone(),
            })?;
        // The CAS check (D-04): a status that no longer equals `from`
        // affects nothing, mirroring `UPDATE ... WHERE status = ?from`
        // matching zero rows on a SQL backend.
        if run.status != from {
            return Err(RunRepositoryError::IllegalTransition {
                from: run.status,
                to,
            });
        }
        let new_status = RunStatus::try_transition(from, to)
            .map_err(|_| RunRepositoryError::IllegalTransition { from, to })?;
        run.status = new_status;
        if new_status == RunStatus::Running {
            run.started_at = Some(at);
        }
        if new_status.is_terminal() {
            run.finished_at = Some(at);
        }
        Ok(())
    }

    async fn record_outcome(
        &self,
        run_id: &RunId,
        outcome: RunOutcomeRecord,
    ) -> Result<(), RunRepositoryError> {
        let mut runs = self.runs.write().await;
        let run = runs
            .get_mut(run_id)
            .ok_or_else(|| RunRepositoryError::NotFound {
                run_id: run_id.clone(),
            })?;
        run.error = outcome.error;
        run.output = outcome.output;
        run.final_waypoint_id = outcome.final_waypoint_id;
        Ok(())
    }

    async fn list(&self, query: RunQuery) -> Result<RunPage, RunRepositoryError> {
        let runs = self.runs.read().await;
        let mut items: Vec<Run> = runs
            .values()
            .filter(|r| query.thread_id.as_ref().is_none_or(|t| *t == r.thread_id))
            .filter(|r| {
                query
                    .assistant_id
                    .as_deref()
                    .is_none_or(|a| a == r.assistant.assistant_id)
            })
            .filter(|r| query.status.is_none_or(|s| s == r.status))
            .cloned()
            .collect();
        items.sort_by(|a, b| {
            b.submitted_at
                .cmp(&a.submitted_at)
                .then_with(|| b.run_id.as_str().cmp(a.run_id.as_str()))
        });

        if let Some(cursor) = &query.cursor {
            let cut = items
                .iter()
                .position(|r| r.submitted_at == cursor.submitted_at && r.run_id == cursor.run_id)
                .map(|idx| idx + 1)
                .unwrap_or(0);
            items = items.split_off(cut.min(items.len()));
        }

        let limit = if query.limit == 0 {
            items.len()
        } else {
            query.limit as usize
        };
        let next_cursor = if items.len() > limit {
            items
                .get(limit - 1)
                .map(|last| paladin_core::platform::container::run::RunCursor {
                    submitted_at: last.submitted_at,
                    run_id: last.run_id.clone(),
                })
        } else {
            None
        };
        items.truncate(limit);
        Ok(RunPage { items, next_cursor })
    }

    async fn active_run_for_thread(
        &self,
        thread_id: &ThreadId,
    ) -> Result<Option<Run>, RunRepositoryError> {
        let runs = self.runs.read().await;
        Ok(runs
            .values()
            .find(|r| &r.thread_id == thread_id && r.status.is_active())
            .cloned())
    }

    async fn request_cancel(&self, run_id: &RunId) -> Result<RunStatus, RunRepositoryError> {
        let mut runs = self.runs.write().await;
        let run = runs
            .get_mut(run_id)
            .ok_or_else(|| RunRepositoryError::NotFound {
                run_id: run_id.clone(),
            })?;
        if run.status.is_terminal() {
            return Err(RunRepositoryError::AlreadyTerminal {
                run_id: run_id.clone(),
                status: run.status,
            });
        }
        run.cancel_requested = true;
        Ok(run.status)
    }

    async fn is_cancel_requested(&self, thread_id: &ThreadId) -> Result<bool, RunRepositoryError> {
        let runs = self.runs.read().await;
        Ok(runs
            .values()
            .find(|r| &r.thread_id == thread_id && r.status.is_active())
            .map(|r| r.cancel_requested)
            .unwrap_or(false))
    }

    async fn bump_attempt(&self, run_id: &RunId) -> Result<u32, RunRepositoryError> {
        let mut runs = self.runs.write().await;
        let run = runs
            .get_mut(run_id)
            .ok_or_else(|| RunRepositoryError::NotFound {
                run_id: run_id.clone(),
            })?;
        run.attempt += 1;
        Ok(run.attempt)
    }

    async fn record_resume(
        &self,
        run_id: &RunId,
        responses: Vec<ParleyResponse>,
    ) -> Result<u32, RunRepositoryError> {
        let mut runs = self.runs.write().await;
        let run = runs
            .get_mut(run_id)
            .ok_or_else(|| RunRepositoryError::NotFound {
                run_id: run_id.clone(),
            })?;
        if run.status != RunStatus::AwaitingInput {
            return Err(RunRepositoryError::IllegalTransition {
                from: run.status,
                to: RunStatus::Running,
            });
        }
        run.pending_responses = responses;
        run.attempt += 1;
        Ok(run.attempt)
    }

    async fn clear_pending_responses(&self, run_id: &RunId) -> Result<(), RunRepositoryError> {
        let mut runs = self.runs.write().await;
        let run = runs
            .get_mut(run_id)
            .ok_or_else(|| RunRepositoryError::NotFound {
                run_id: run_id.clone(),
            })?;
        run.pending_responses.clear();
        Ok(())
    }

    async fn insert_with_latest(&self, run: &Run) -> Result<u32, RunRepositoryError> {
        let Some(assistants) = self.assistants.as_ref() else {
            // No assistants repository wired in: fall back to the port's
            // own default (verbatim insert), rather than duplicating it
            // here.
            self.insert(run).await?;
            return Ok(run.assistant.version);
        };

        let assistant_id = AssistantId::new(run.assistant.assistant_id.clone()).map_err(|e| {
            RunRepositoryError::Serialization {
                message: format!("invalid assistant_id {:?}: {e}", run.assistant.assistant_id),
            }
        })?;

        // Lock order: assistants read -> runs write (documented on
        // `with_assistants`). Resolving `latest` happens entirely before
        // `insert`'s own write-lock acquisition, so a concurrent
        // `append_version` (which holds the assistants write lock for its
        // whole mutation) is observed strictly before or strictly after
        // this read, never torn.
        let resolved_version = assistants
            .latest_version_if_active(&assistant_id)
            .await
            .ok_or_else(|| RunRepositoryError::UnknownAssistant {
                assistant_id: run.assistant.assistant_id.clone(),
            })?;

        let mut resolved_run = run.clone();
        resolved_run.assistant.version = resolved_version;
        self.insert(&resolved_run).await?;
        Ok(resolved_version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::run::AssistantRef;

    fn sample_run(thread: &str) -> Run {
        Run::new(
            RunId::new_v7(),
            ThreadId::new(thread).unwrap(),
            AssistantRef {
                assistant_id: "a1".to_string(),
                version: 1,
            },
            serde_json::json!({}),
        )
    }

    #[tokio::test]
    async fn insert_then_get_round_trips() {
        let repo = InMemoryRunRepository::new();
        let run = sample_run("t1");
        repo.insert(&run).await.unwrap();
        let loaded = repo.get(&run.run_id).await.unwrap().unwrap();
        assert_eq!(loaded.run_id, run.run_id);
        assert_eq!(loaded.status, RunStatus::Queued);
    }

    #[tokio::test]
    async fn insert_rejects_second_active_run_on_same_thread() {
        let repo = InMemoryRunRepository::new();
        let first = sample_run("busy-thread");
        repo.insert(&first).await.unwrap();

        let second = sample_run("busy-thread");
        let err = repo.insert(&second).await.unwrap_err();
        assert!(matches!(err, RunRepositoryError::ThreadBusy { .. }));
    }

    #[tokio::test]
    async fn update_status_applies_cas_and_stamps_timestamps() {
        let repo = InMemoryRunRepository::new();
        let run = sample_run("t2");
        repo.insert(&run).await.unwrap();

        let now = Utc::now();
        repo.update_status(&run.run_id, RunStatus::Queued, RunStatus::Running, now)
            .await
            .unwrap();
        let loaded = repo.get(&run.run_id).await.unwrap().unwrap();
        assert_eq!(loaded.status, RunStatus::Running);
        assert_eq!(loaded.started_at, Some(now));

        repo.update_status(&run.run_id, RunStatus::Running, RunStatus::Completed, now)
            .await
            .unwrap();
        let loaded = repo.get(&run.run_id).await.unwrap().unwrap();
        assert_eq!(loaded.status, RunStatus::Completed);
        assert_eq!(loaded.finished_at, Some(now));
    }

    #[tokio::test]
    async fn update_status_rejects_stale_from() {
        let repo = InMemoryRunRepository::new();
        let run = sample_run("t3");
        repo.insert(&run).await.unwrap();
        let now = Utc::now();
        repo.update_status(&run.run_id, RunStatus::Queued, RunStatus::Running, now)
            .await
            .unwrap();

        // The row is already Running; a stale CAS claiming it is still
        // Queued must fail rather than silently overwrite.
        let err = repo
            .update_status(&run.run_id, RunStatus::Queued, RunStatus::Cancelled, now)
            .await
            .unwrap_err();
        assert!(matches!(err, RunRepositoryError::IllegalTransition { .. }));
    }

    #[tokio::test]
    async fn record_outcome_sets_error_output_and_waypoint() {
        let repo = InMemoryRunRepository::new();
        let run = sample_run("t4");
        repo.insert(&run).await.unwrap();

        repo.record_outcome(
            &run.run_id,
            RunOutcomeRecord {
                error: Some("boom".to_string()),
                output: Some(serde_json::json!({"ok": false})),
                final_waypoint_id: Some("wp-1".to_string()),
            },
        )
        .await
        .unwrap();

        let loaded = repo.get(&run.run_id).await.unwrap().unwrap();
        assert_eq!(loaded.error.as_deref(), Some("boom"));
        assert_eq!(loaded.final_waypoint_id.as_deref(), Some("wp-1"));
    }

    #[tokio::test]
    async fn active_run_for_thread_finds_only_active_runs() {
        let repo = InMemoryRunRepository::new();
        let run = sample_run("t5");
        repo.insert(&run).await.unwrap();

        let active = repo
            .active_run_for_thread(&ThreadId::new("t5").unwrap())
            .await
            .unwrap();
        assert!(active.is_some());

        repo.update_status(
            &run.run_id,
            RunStatus::Queued,
            RunStatus::Cancelled,
            Utc::now(),
        )
        .await
        .unwrap();
        let active = repo
            .active_run_for_thread(&ThreadId::new("t5").unwrap())
            .await
            .unwrap();
        assert!(active.is_none());
    }

    #[tokio::test]
    async fn request_cancel_is_idempotent_and_rejects_terminal() {
        let repo = InMemoryRunRepository::new();
        let run = sample_run("t6");
        repo.insert(&run).await.unwrap();

        let status = repo.request_cancel(&run.run_id).await.unwrap();
        assert_eq!(status, RunStatus::Queued);
        let status_again = repo.request_cancel(&run.run_id).await.unwrap();
        assert_eq!(status_again, RunStatus::Queued);

        repo.update_status(
            &run.run_id,
            RunStatus::Queued,
            RunStatus::Cancelled,
            Utc::now(),
        )
        .await
        .unwrap();
        let err = repo.request_cancel(&run.run_id).await.unwrap_err();
        assert!(matches!(err, RunRepositoryError::AlreadyTerminal { .. }));
    }
}

#[cfg(test)]
mod contract_suite {
    use super::*;
    use crate::run::contract_tests;
    use std::sync::Arc;

    // One #[tokio::test] per shared contract function (D-09 precedent), each
    // against a fresh `InMemoryRunRepository`, so a failure names the
    // violated contract clause. See `contract_tests` for the assertions
    // themselves -- this module only wires `InMemoryRunRepository` into
    // them, unchanged.

    #[tokio::test]
    async fn insert_then_get_round_trips_every_field() {
        contract_tests::insert_then_get_round_trips_every_field(&InMemoryRunRepository::new())
            .await;
    }

    #[tokio::test]
    async fn update_status_queued_to_running_sets_started_at_then_stale_cas_fails() {
        contract_tests::update_status_queued_to_running_sets_started_at_then_stale_cas_fails(
            &InMemoryRunRepository::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn update_status_running_to_completed_sets_finished_at_then_terminal_is_absorbing() {
        contract_tests::update_status_running_to_completed_sets_finished_at_then_terminal_is_absorbing(
            &InMemoryRunRepository::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn update_status_self_transition_fails() {
        contract_tests::update_status_self_transition_fails(&InMemoryRunRepository::new()).await;
    }

    #[tokio::test]
    async fn insert_rejects_second_active_run_then_succeeds_after_terminal() {
        contract_tests::insert_rejects_second_active_run_then_succeeds_after_terminal(
            &InMemoryRunRepository::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn list_paginates_by_submitted_at_and_run_id_with_no_overlap_or_gap() {
        contract_tests::list_paginates_by_submitted_at_and_run_id_with_no_overlap_or_gap(
            &InMemoryRunRepository::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn list_filters_by_thread_assistant_and_status() {
        contract_tests::list_filters_by_thread_assistant_and_status(&InMemoryRunRepository::new())
            .await;
    }

    #[tokio::test]
    async fn request_cancel_is_idempotent_and_rejects_terminal() {
        contract_tests::request_cancel_is_idempotent_and_rejects_terminal(
            &InMemoryRunRepository::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn is_cancel_requested_reflects_active_run_flag() {
        contract_tests::is_cancel_requested_reflects_active_run_flag(&InMemoryRunRepository::new())
            .await;
    }

    #[tokio::test]
    async fn bump_attempt_increments_and_persists() {
        contract_tests::bump_attempt_increments_and_persists(&InMemoryRunRepository::new()).await;
    }

    #[tokio::test]
    async fn record_resume_on_awaiting_input_then_clear_pending_responses() {
        contract_tests::record_resume_on_awaiting_input_then_clear_pending_responses(
            &InMemoryRunRepository::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn record_outcome_persists_fields_without_touching_status() {
        contract_tests::record_outcome_persists_fields_without_touching_status(
            &InMemoryRunRepository::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn get_on_unsupported_schema_version_fails() {
        contract_tests::get_on_unsupported_schema_version_fails(&InMemoryRunRepository::new())
            .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn ten_concurrent_inserts_one_thread_exactly_one_accepted() {
        let repo: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        contract_tests::ten_concurrent_inserts_one_thread_exactly_one_accepted(repo).await;
    }

    // ── insert_with_latest / freeze-at-submit (D-30) ─────────────────────
    // These four clauses require the assistants repository wired in via
    // `with_assistants` -- the plain `InMemoryRunRepository::new()` used
    // above falls back to the port's default (verbatim-insert)
    // implementation, which has no concept of `latest` to freeze.

    fn repo_with_assistants() -> (
        InMemoryRunRepository,
        Arc<crate::assistant::in_memory::InMemoryAssistantRepository>,
    ) {
        let assistants = Arc::new(crate::assistant::in_memory::InMemoryAssistantRepository::new());
        let repo = InMemoryRunRepository::new().with_assistants(Arc::clone(&assistants));
        (repo, assistants)
    }

    #[tokio::test]
    async fn insert_with_latest_resolves_current_latest_and_freezes_it() {
        let (run_repo, assistant_repo) = repo_with_assistants();
        contract_tests::insert_with_latest_resolves_current_latest_and_freezes_it(
            &run_repo,
            assistant_repo.as_ref(),
        )
        .await;
    }

    #[tokio::test]
    async fn insert_with_latest_unknown_assistant_fails() {
        let (run_repo, _assistant_repo) = repo_with_assistants();
        contract_tests::insert_with_latest_unknown_assistant_fails(&run_repo).await;
    }

    #[tokio::test]
    async fn insert_with_latest_soft_deleted_assistant_fails() {
        let (run_repo, assistant_repo) = repo_with_assistants();
        contract_tests::insert_with_latest_soft_deleted_assistant_fails(
            &run_repo,
            assistant_repo.as_ref(),
        )
        .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn assistant_version_freeze_at_submit() {
        let (run_repo, assistant_repo) = repo_with_assistants();
        let run_repo: Arc<dyn RunRepositoryPort> = Arc::new(run_repo);
        let assistant_repo: Arc<
            dyn paladin_ports::output::assistant_repository_port::AssistantRepositoryPort,
        > = assistant_repo;
        contract_tests::assistant_version_freeze_at_submit(run_repo, assistant_repo).await;
    }
}
