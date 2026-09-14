//! Schedule Admin Port — validate-then-persist run schedules (PLAT-05, D-36..D-39, D-42, D-46)
//!
//! [`ScheduleAdminPort`] is the whole surface the `paladin-web` schedule routes
//! (`crates/paladin-web/src/schedule_controller.rs`) drive: create, read, page, patch and
//! delete a persisted [`RunSchedule`]. An implementation
//! (`src/application/services/run/schedule/admin.rs`'s `ScheduleService`) validates a
//! [`CreateRunSchedule`]/[`RunScheduleUpdate`] BEFORE any repository write — the cron (5 or
//! 6 fields), the IANA timezone, the assistant reference and the webhook URL (a write-time
//! SSRF guard, D-42) all reject with [`ScheduleAdminError::Invalid`], carrying a non-empty,
//! machine-readable [`ValidationViolation`] list (the SAME type
//! [`crate::input::assistant_admin_port::ValidationViolation`] already defines) — nothing is
//! ever persisted on that path.

use async_trait::async_trait;

use paladin_core::platform::container::run::WebhookSpec;
use paladin_core::platform::container::run_schedule::{
    OnMissed, RunSchedule, RunScheduleId, RunScheduleUpdate, ThreadStrategy,
};
use thiserror::Error;

use crate::input::assistant_admin_port::ValidationViolation;
use crate::output::run_schedule_repository_port::RunSchedulePage;

/// The fields a caller supplies to create a schedule through [`ScheduleAdminPort::create`].
/// The port assigns `schedule_id`, `last_tick`, `next_tick`, `skipped_ticks`, `created_at`
/// and `updated_at`.
#[derive(Debug, Clone)]
pub struct CreateRunSchedule {
    /// The assistant this schedule submits runs against.
    pub assistant_id: String,
    /// A specific version, or `None` to resolve `latest` at each tick.
    pub version: Option<u32>,
    /// The cron expression (5- or 6-field, `paladin_storage::cron::parse_run_cron` parses
    /// either form — D-38).
    pub cron: String,
    /// The IANA timezone the cron expression is evaluated in, or `None` for `"UTC"`.
    pub timezone: Option<String>,
    /// The input every tick's submitted run receives.
    pub input: serde_json::Value,
    /// Whether this schedule starts out active.
    pub enabled: bool,
    /// Which thread each tick's run executes against, or `None` for the default
    /// (`NewThreadPerTick`).
    pub thread_strategy: Option<ThreadStrategy>,
    /// What happens when a tick is discovered late, or `None` for the default (`Skip`).
    pub on_missed: Option<OnMissed>,
    /// An optional webhook delivery target, validated by the write-time SSRF guard (D-42)
    /// before the schedule is ever persisted.
    pub webhook: Option<WebhookSpec>,
}

/// Errors returned by [`ScheduleAdminPort`] methods (X-06 — structured, never a bare
/// `bool`/`String`). `#[non_exhaustive]`: a future variant must not be a breaking change.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ScheduleAdminError {
    /// The submitted schedule (create or patch) failed validation — nothing was persisted.
    #[error("run schedule failed validation ({} violation(s))", violations.len())]
    Invalid {
        /// Every violation found, never empty.
        violations: Vec<ValidationViolation>,
    },
    /// No schedule exists with the given id.
    #[error("run schedule not found: {schedule_id}")]
    NotFound {
        /// The requested schedule id.
        schedule_id: RunScheduleId,
    },
    /// The underlying backend failed.
    #[error("schedule admin backend error: {source}")]
    Backend {
        /// The underlying backend error.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// No schedule admin backend is configured (D-44's 501 precedent — the HTTP layer maps
    /// this to `501 not_implemented` naming `schedules.enabled`).
    #[error("schedule admin port not wired")]
    NotWired,
}

/// Port trait for validating and persisting [`RunSchedule`]s (D-36..D-39, D-42, D-46).
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync`: schedules are created, read and ticked
/// concurrently across HTTP handlers and the background tick loop.
#[async_trait]
pub trait ScheduleAdminPort: Send + Sync {
    /// Validate `create`, then persist a brand-new schedule with its first `next_tick`
    /// computed from the current time.
    ///
    /// # Errors
    ///
    /// Returns [`ScheduleAdminError::Invalid`] if the cron, timezone, assistant reference or
    /// webhook URL fails validation — nothing is persisted.
    async fn create(&self, create: CreateRunSchedule) -> Result<RunSchedule, ScheduleAdminError>;

    /// Load a schedule by id. `Ok(None)` if it does not exist — never an error on its own.
    async fn get(
        &self,
        schedule_id: &RunScheduleId,
    ) -> Result<Option<RunSchedule>, ScheduleAdminError>;

    /// Page through schedules, ordered ascending by `schedule_id`.
    async fn list(
        &self,
        limit: u32,
        cursor: Option<RunScheduleId>,
    ) -> Result<RunSchedulePage, ScheduleAdminError>;

    /// Apply a partial update. A `cron`/`timezone` change recomputes `next_tick` from the
    /// current time; an `enabled`-only change leaves `next_tick` untouched (a disabled
    /// schedule is simply skipped by `due`); a `webhook` change re-runs the write-time SSRF
    /// guard (D-42).
    ///
    /// # Errors
    ///
    /// Returns [`ScheduleAdminError::Invalid`] if a changed cron, timezone or webhook URL
    /// fails validation — nothing is persisted. Returns [`ScheduleAdminError::NotFound`]
    /// when `schedule_id` does not exist.
    async fn patch(
        &self,
        schedule_id: &RunScheduleId,
        update: RunScheduleUpdate,
    ) -> Result<RunSchedule, ScheduleAdminError>;

    /// Delete a schedule.
    ///
    /// # Errors
    ///
    /// Returns [`ScheduleAdminError::NotFound`] when `schedule_id` does not exist.
    async fn delete(&self, schedule_id: &RunScheduleId) -> Result<(), ScheduleAdminError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn trait_is_object_safe() {
        let _: Option<Arc<dyn ScheduleAdminPort>> = None;
    }
}
