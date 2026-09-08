//! Run Schedule Repository Port — Restart- and Replica-Safe Ticks (D-36, D-37)
//!
//! [`RunScheduleRepositoryPort`] is the whole contract every backend adapter
//! (`InMemoryRunScheduleRepository`, `SqliteRunScheduleRepository`,
//! `PostgresRunScheduleRepository`, all `paladin-storage`) implements.
//!
//! ## `claim_tick` is the whole restart/replica-safety story (D-37)
//!
//! [`RunScheduleRepositoryPort::claim_tick`] is a conditional update: it
//! succeeds (`true`) for exactly one caller when several race the same
//! `expected_next`, and every other racing caller sees `false`. A caller only
//! submits a run when its own `claim_tick` call returned `true` — this is
//! what makes two `ScheduleService` instances (or one restarted instance)
//! ticking the same schedule at the same due time produce exactly one fired
//! run, with no leader election anywhere in the picture.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use thiserror::Error;

use paladin_core::platform::container::run_schedule::{
    RunSchedule, RunScheduleId, RunScheduleUpdate,
};

/// A page of [`RunSchedule`]s returned by [`RunScheduleRepositoryPort::list`],
/// ordered ascending by `schedule_id`.
#[derive(Debug, Clone, Default)]
pub struct RunSchedulePage {
    /// The page's schedules, in the documented order.
    pub items: Vec<RunSchedule>,
    /// Opaque cursor for the next page (the last `schedule_id` on this
    /// page), `None` on the last page.
    pub next_cursor: Option<RunScheduleId>,
}

/// Errors returned by [`RunScheduleRepositoryPort`] methods (X-06 —
/// structured, never a bare `bool`/`String`).
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RunScheduleRepositoryError {
    /// No schedule exists with the given id.
    #[error("run schedule not found: {schedule_id}")]
    NotFound {
        /// The requested schedule id.
        schedule_id: RunScheduleId,
    },
    /// [`RunScheduleRepositoryPort::insert`] was called with an id that
    /// already exists.
    #[error("run schedule already exists: {schedule_id}")]
    AlreadyExists {
        /// The already-existing schedule id.
        schedule_id: RunScheduleId,
    },
    /// The underlying storage backend failed.
    #[error("run schedule repository backend error: {source}")]
    Backend {
        /// The underlying backend error.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// A stored (or to-be-stored) schedule could not be (de)serialized.
    #[error("run schedule serialization error: {message}")]
    Serialization {
        /// Description of the serialization failure.
        message: String,
    },
    /// A stored schedule carries a schema version this build does not know
    /// how to read.
    #[error("unsupported run schedule schema version: found {found}")]
    UnknownSchemaVersion {
        /// The schema version found on the stored data.
        found: String,
    },
}

/// Port trait for persisting and reading back [`RunSchedule`]s with a
/// restart- and replica-safe tick claim (D-36, D-37).
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync`: schedules are read and ticked
/// concurrently across `ScheduleService` instances.
#[async_trait]
pub trait RunScheduleRepositoryPort: Send + Sync {
    /// Persist a brand-new schedule.
    ///
    /// # Errors
    ///
    /// Returns [`RunScheduleRepositoryError::AlreadyExists`] if
    /// `schedule.schedule_id` already exists.
    async fn insert(&self, schedule: RunSchedule) -> Result<(), RunScheduleRepositoryError>;

    /// Load a schedule by id. `Ok(None)` if it does not exist — never an
    /// error on its own.
    async fn get(
        &self,
        schedule_id: &RunScheduleId,
    ) -> Result<Option<RunSchedule>, RunScheduleRepositoryError>;

    /// Page through schedules, ordered ascending by `schedule_id`.
    async fn list(
        &self,
        limit: u32,
        cursor: Option<RunScheduleId>,
    ) -> Result<RunSchedulePage, RunScheduleRepositoryError>;

    /// Apply a partial update.
    ///
    /// # Errors
    ///
    /// Returns [`RunScheduleRepositoryError::NotFound`] when `schedule_id`
    /// does not exist.
    async fn update(
        &self,
        schedule_id: &RunScheduleId,
        update: RunScheduleUpdate,
    ) -> Result<(), RunScheduleRepositoryError>;

    /// Delete a schedule.
    ///
    /// # Errors
    ///
    /// Returns [`RunScheduleRepositoryError::NotFound`] when `schedule_id`
    /// does not exist.
    async fn delete(&self, schedule_id: &RunScheduleId) -> Result<(), RunScheduleRepositoryError>;

    /// List every schedule that is `enabled` and due (`next_tick <= now`),
    /// ordered ascending by `next_tick`, at most `limit` rows.
    async fn due(
        &self,
        now: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<RunSchedule>, RunScheduleRepositoryError>;

    /// Claim a tick: a conditional update that sets `last_tick = new_last`,
    /// `next_tick = new_next` only if the row's CURRENT `next_tick` still
    /// equals `expected_next` (D-37). Returns `true` iff exactly one row
    /// changed — the caller that receives `true` is the ONE replica that
    /// submits a run for this tick; every other racing caller receives
    /// `false` (`LostRace`, never an error).
    async fn claim_tick(
        &self,
        schedule_id: &RunScheduleId,
        expected_next: DateTime<Utc>,
        new_last: DateTime<Utc>,
        new_next: DateTime<Utc>,
    ) -> Result<bool, RunScheduleRepositoryError>;

    /// Atomically increment `skipped_ticks` and return the new value
    /// (D-39's counted skip metric).
    async fn increment_skipped(
        &self,
        schedule_id: &RunScheduleId,
    ) -> Result<u64, RunScheduleRepositoryError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // No full mock `impl RunScheduleRepositoryPort` lives in this file: the
    // real proof of implementability is `InMemoryRunScheduleRepository`
    // (`paladin-storage`), mirroring `assistant_repository_port.rs`'s own
    // convention. Object safety alone is checked below.

    #[test]
    fn trait_is_object_safe() {
        let _: Option<Arc<dyn RunScheduleRepositoryPort>> = None;
    }

    #[test]
    fn run_schedule_page_default_is_empty() {
        let page = RunSchedulePage::default();
        assert!(page.items.is_empty());
        assert!(page.next_cursor.is_none());
    }
}
