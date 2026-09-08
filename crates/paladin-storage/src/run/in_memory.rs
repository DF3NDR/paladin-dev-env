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

use paladin_core::platform::container::parley::ParleyResponse;
use paladin_core::platform::container::run::{Run, RunId, RunStatus};
use paladin_core::platform::container::waypoint::ThreadId;
use paladin_ports::output::run_repository_port::{
    RunOutcomeRecord, RunPage, RunQuery, RunRepositoryError, RunRepositoryPort,
};

/// In-memory `RunRepositoryPort` implementation.
///
/// Cloning is cheap and shares the same underlying store (the inner `Arc` is
/// cloned).
#[derive(Clone, Default)]
pub struct InMemoryRunRepository {
    runs: Arc<RwLock<HashMap<RunId, Run>>>,
}

impl InMemoryRunRepository {
    /// Construct a new, empty repository.
    pub fn new() -> Self {
        Self::default()
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
        Ok(self.runs.read().await.get(run_id).cloned())
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
