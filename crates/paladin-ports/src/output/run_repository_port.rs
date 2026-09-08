//! Run Repository Port — Persisting and Reading Back `Run`s (D-03)
//!
//! [`RunRepositoryPort`] is the WHOLE contract every backend adapter
//! (`InMemoryRunRepository`, `SqliteRunRepository`, `PostgresRunRepository`,
//! all `paladin-storage`) implements, defined here up front (interface
//! first) so plans 27-02…27-15 build against a fixed interface rather than
//! growing it incrementally.
//!
//! ## Compare-and-set, never read-modify-write (D-04)
//!
//! [`RunRepositoryPort::update_status`] must compile to a single
//! conditional update — `UPDATE runs SET status = ?to WHERE run_id = ?id
//! AND status = ?from` on a SQL backend — never a read-then-write. Zero rows
//! affected is [`RunRepositoryError::IllegalTransition`], never a silent
//! no-op. This is what keeps monotonicity true under concurrent workers with
//! no application lock.
//!
//! ## Only `insert`/`get`/`update_status`/`record_outcome` are exercised by
//! this slice
//!
//! This plan's tracer test only drives the four methods above; every other
//! method still needs a real (non-`todo!`) implementation on
//! [`crate::output::run_repository_port`]'s in-memory adapter (a few lines
//! over the map each) so plan 27-02's contract suite runs against it
//! unchanged.

use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use thiserror::Error;

use paladin_core::platform::container::parley::ParleyResponse;
use paladin_core::platform::container::run::{Run, RunCursor, RunId, RunStatus};
use paladin_core::platform::container::waypoint::ThreadId;

/// A page of [`Run`]s returned by [`RunRepositoryPort::list`], ordered
/// `(submitted_at DESC, run_id DESC)`.
#[derive(Debug, Clone)]
pub struct RunPage {
    /// The page's runs, in the documented order.
    pub items: Vec<Run>,
    /// Opaque cursor for the next page, `None` on the last page.
    pub next_cursor: Option<RunCursor>,
}

/// Filter/pagination parameters for [`RunRepositoryPort::list`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunQuery {
    /// Restrict to a single thread.
    pub thread_id: Option<ThreadId>,
    /// Restrict to a single assistant.
    pub assistant_id: Option<String>,
    /// Restrict to a single status.
    pub status: Option<RunStatus>,
    /// Maximum number of items to return.
    pub limit: u32,
    /// Opaque keyset cursor from a previous page's `next_cursor`.
    pub cursor: Option<RunCursor>,
}

/// The terminal outcome fields [`RunRepositoryPort::record_outcome`]
/// persists alongside a status transition.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RunOutcomeRecord {
    /// The engine's error, for a `Failed` outcome.
    pub error: Option<String>,
    /// The final output, for an `Agent`-kind assistant.
    pub output: Option<serde_json::Value>,
    /// The final Waypoint id reached, if any.
    pub final_waypoint_id: Option<String>,
}

/// Errors returned by [`RunRepositoryPort`] methods (X-06 — structured,
/// never a bare `bool`/`String`).
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RunRepositoryError {
    /// No run exists with the given id.
    #[error("run not found: {run_id}")]
    NotFound {
        /// The requested run id.
        run_id: RunId,
    },
    /// A CAS [`RunRepositoryPort::update_status`] call affected zero rows.
    #[error("illegal run status transition from {from} to {to}")]
    IllegalTransition {
        /// The status the transition was attempted from.
        from: RunStatus,
        /// The status the transition was attempted to.
        to: RunStatus,
    },
    /// [`RunRepositoryPort::insert`] was rejected because the target thread
    /// already has an active run (D-17/D-18).
    #[error("thread busy: {thread_id}")]
    ThreadBusy {
        /// The busy thread.
        thread_id: ThreadId,
    },
    /// A mutating call was rejected because the run is already terminal.
    #[error("run {run_id} is already terminal ({status})")]
    AlreadyTerminal {
        /// The terminal run.
        run_id: RunId,
        /// Its current (terminal) status.
        status: RunStatus,
    },
    /// The underlying storage backend failed.
    #[error("run repository backend error: {source}")]
    Backend {
        /// The underlying backend error.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// A stored (or to-be-stored) `Run` could not be (de)serialized.
    #[error("run serialization error: {message}")]
    Serialization {
        /// Description of the serialization failure.
        message: String,
    },
    /// A stored `Run` carries a schema version this build does not know how
    /// to read.
    #[error("unsupported run schema version: found {found}")]
    UnknownSchemaVersion {
        /// The schema version found on the stored data.
        found: String,
    },
}

/// Port trait for persisting and reading back [`Run`]s (D-03).
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync`: runs are inserted, transitioned
/// and read back concurrently across HTTP handlers and worker tasks.
#[async_trait]
pub trait RunRepositoryPort: Send + Sync {
    /// Persist a newly submitted `Run`.
    ///
    /// # Errors
    ///
    /// Returns [`RunRepositoryError::ThreadBusy`] when `run.thread_id`
    /// already has an active run (`Queued`, `Running` or `AwaitingInput`,
    /// D-18).
    async fn insert(&self, run: &Run) -> Result<(), RunRepositoryError>;

    /// Load a run by id. `Ok(None)` if it does not exist — never an error on
    /// its own.
    async fn get(&self, run_id: &RunId) -> Result<Option<Run>, RunRepositoryError>;

    /// Compare-and-set status transition (D-04): sets `started_at` on
    /// entering `Running`, `finished_at` on a terminal `to`.
    ///
    /// # Errors
    ///
    /// Returns [`RunRepositoryError::IllegalTransition`] when the row's
    /// current status is not `from` (zero rows affected) or when `from` ->
    /// `to` is not a legal edge, and [`RunRepositoryError::NotFound`] when
    /// `run_id` does not exist.
    async fn update_status(
        &self,
        run_id: &RunId,
        from: RunStatus,
        to: RunStatus,
        at: DateTime<Utc>,
    ) -> Result<(), RunRepositoryError>;

    /// Record a run's terminal outcome fields alongside (but not instead of)
    /// its own [`update_status`](Self::update_status) call.
    async fn record_outcome(
        &self,
        run_id: &RunId,
        outcome: RunOutcomeRecord,
    ) -> Result<(), RunRepositoryError>;

    /// Page through runs matching `query`, ordered `(submitted_at DESC,
    /// run_id DESC)`.
    async fn list(&self, query: RunQuery) -> Result<RunPage, RunRepositoryError>;

    /// The thread's currently active run (`Queued`, `Running` or
    /// `AwaitingInput`), if any.
    async fn active_run_for_thread(
        &self,
        thread_id: &ThreadId,
    ) -> Result<Option<Run>, RunRepositoryError>;

    /// Idempotently request cancellation of `run_id`, returning its current
    /// status. `AlreadyTerminal` on a terminal run.
    async fn request_cancel(&self, run_id: &RunId) -> Result<RunStatus, RunRepositoryError>;

    /// Whether `thread_id`'s active run has a cancellation requested.
    async fn is_cancel_requested(&self, thread_id: &ThreadId) -> Result<bool, RunRepositoryError>;

    /// Increment and return `run_id`'s `attempt` counter (redelivery path).
    async fn bump_attempt(&self, run_id: &RunId) -> Result<u32, RunRepositoryError>;

    /// Record a resume's responses onto `run_id` (only legal from
    /// `AwaitingInput`), incrementing `attempt` and returning the new value.
    async fn record_resume(
        &self,
        run_id: &RunId,
        responses: Vec<ParleyResponse>,
    ) -> Result<u32, RunRepositoryError>;

    /// Clear `run_id`'s parked `pending_responses` (after a worker consumes
    /// them).
    async fn clear_pending_responses(&self, run_id: &RunId) -> Result<(), RunRepositoryError>;
}

/// A no-op lease duration placeholder some call sites need — re-exported so
/// adapters share one `Duration` type without each importing `std::time`
/// under a different alias.
pub type LeaseDuration = Duration;

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::run::AssistantRef;
    use std::sync::Arc;

    /// A minimal mock, mirroring `waypoint_port.rs`'s `MockWaypointStore`
    /// convention: proves the trait is implementable and object-safe.
    struct MockRepository;

    #[async_trait]
    impl RunRepositoryPort for MockRepository {
        async fn insert(&self, _run: &Run) -> Result<(), RunRepositoryError> {
            Ok(())
        }

        async fn get(&self, _run_id: &RunId) -> Result<Option<Run>, RunRepositoryError> {
            Ok(None)
        }

        async fn update_status(
            &self,
            _run_id: &RunId,
            _from: RunStatus,
            _to: RunStatus,
            _at: DateTime<Utc>,
        ) -> Result<(), RunRepositoryError> {
            Ok(())
        }

        async fn record_outcome(
            &self,
            _run_id: &RunId,
            _outcome: RunOutcomeRecord,
        ) -> Result<(), RunRepositoryError> {
            Ok(())
        }

        async fn list(&self, _query: RunQuery) -> Result<RunPage, RunRepositoryError> {
            Ok(RunPage {
                items: vec![],
                next_cursor: None,
            })
        }

        async fn active_run_for_thread(
            &self,
            _thread_id: &ThreadId,
        ) -> Result<Option<Run>, RunRepositoryError> {
            Ok(None)
        }

        async fn request_cancel(&self, run_id: &RunId) -> Result<RunStatus, RunRepositoryError> {
            Err(RunRepositoryError::NotFound {
                run_id: run_id.clone(),
            })
        }

        async fn is_cancel_requested(
            &self,
            _thread_id: &ThreadId,
        ) -> Result<bool, RunRepositoryError> {
            Ok(false)
        }

        async fn bump_attempt(&self, _run_id: &RunId) -> Result<u32, RunRepositoryError> {
            Ok(1)
        }

        async fn record_resume(
            &self,
            _run_id: &RunId,
            _responses: Vec<ParleyResponse>,
        ) -> Result<u32, RunRepositoryError> {
            Ok(1)
        }

        async fn clear_pending_responses(&self, _run_id: &RunId) -> Result<(), RunRepositoryError> {
            Ok(())
        }
    }

    #[test]
    fn trait_is_object_safe() {
        let _: Option<Arc<dyn RunRepositoryPort>> = None;
    }

    #[tokio::test]
    async fn mock_repository_implements_trait() {
        let repo = MockRepository;
        let thread = ThreadId::new("t1").unwrap();
        assert!(repo.get(&RunId::new_v7()).await.unwrap().is_none());
        assert!(repo.active_run_for_thread(&thread).await.unwrap().is_none());
        assert!(!repo.is_cancel_requested(&thread).await.unwrap());
    }

    #[test]
    fn run_query_default_has_no_filters() {
        let query = RunQuery::default();
        assert!(query.thread_id.is_none());
        assert!(query.assistant_id.is_none());
        assert!(query.status.is_none());
        assert!(query.cursor.is_none());
    }

    #[test]
    fn run_outcome_record_default_is_empty() {
        let record = RunOutcomeRecord::default();
        assert!(record.error.is_none());
        assert!(record.output.is_none());
        assert!(record.final_waypoint_id.is_none());
    }

    #[test]
    fn assistant_ref_is_reachable_from_this_module_scope() {
        // Sanity check that the port module's imports compile against the
        // full `paladin-core` run type set this trait's methods reference.
        let reference = AssistantRef {
            assistant_id: "a".to_string(),
            version: 1,
        };
        assert_eq!(reference.version, 1);
    }
}
