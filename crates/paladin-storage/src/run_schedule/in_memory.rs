/*
In-Memory Run Schedule Repository

An `Arc<tokio::sync::RwLock<Store>>`-backed implementation of
`RunScheduleRepositoryPort`, for tests and local development (D-03's
InMemory convention). `claim_tick` runs under the single write lock, so the
whole compare-and-set sequence is already serialized in-process -- the SQL
adapters (sqlite.rs, postgres.rs) are where the CAS primitive does real
cross-connection work.
*/

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

use paladin_core::platform::container::run_schedule::{
    RunSchedule, RunScheduleId, RunScheduleUpdate,
};
use paladin_ports::output::run_schedule_repository_port::{
    RunSchedulePage, RunScheduleRepositoryError, RunScheduleRepositoryPort,
};

#[derive(Default)]
struct Store {
    schedules: HashMap<RunScheduleId, RunSchedule>,
}

/// In-memory `RunScheduleRepositoryPort` implementation.
///
/// Cloning is cheap and shares the same underlying store (the inner `Arc`
/// is cloned).
#[derive(Clone, Default)]
pub struct InMemoryRunScheduleRepository {
    store: Arc<RwLock<Store>>,
}

impl InMemoryRunScheduleRepository {
    /// Construct a new, empty repository.
    pub fn new() -> Self {
        Self::default()
    }
}

fn apply_update(schedule: &mut RunSchedule, update: RunScheduleUpdate) {
    if let Some(cron) = update.cron {
        schedule.cron = cron;
    }
    if let Some(timezone) = update.timezone {
        schedule.timezone = timezone;
    }
    if let Some(input) = update.input {
        schedule.input = input;
    }
    if let Some(enabled) = update.enabled {
        schedule.enabled = enabled;
    }
    if let Some(thread_strategy) = update.thread_strategy {
        schedule.thread_strategy = thread_strategy;
    }
    if let Some(on_missed) = update.on_missed {
        schedule.on_missed = on_missed;
    }
    if let Some(webhook) = update.webhook {
        schedule.webhook = Some(webhook);
    }
    if let Some(next_tick) = update.next_tick {
        schedule.next_tick = Some(next_tick);
    }
    schedule.updated_at = Utc::now();
}

#[async_trait]
impl RunScheduleRepositoryPort for InMemoryRunScheduleRepository {
    async fn insert(&self, schedule: RunSchedule) -> Result<(), RunScheduleRepositoryError> {
        let mut store = self.store.write().await;
        if store.schedules.contains_key(&schedule.schedule_id) {
            return Err(RunScheduleRepositoryError::AlreadyExists {
                schedule_id: schedule.schedule_id.clone(),
            });
        }
        store
            .schedules
            .insert(schedule.schedule_id.clone(), schedule);
        Ok(())
    }

    async fn get(
        &self,
        schedule_id: &RunScheduleId,
    ) -> Result<Option<RunSchedule>, RunScheduleRepositoryError> {
        let store = self.store.read().await;
        Ok(store.schedules.get(schedule_id).cloned())
    }

    async fn list(
        &self,
        limit: u32,
        cursor: Option<RunScheduleId>,
    ) -> Result<RunSchedulePage, RunScheduleRepositoryError> {
        let store = self.store.read().await;
        let mut items: Vec<RunSchedule> = store.schedules.values().cloned().collect();
        items.sort_by(|a, b| a.schedule_id.cmp(&b.schedule_id));

        if let Some(cursor) = &cursor {
            let cut = items
                .iter()
                .position(|s| &s.schedule_id == cursor)
                .map(|idx| idx + 1)
                .unwrap_or(0);
            items = items.split_off(cut.min(items.len()));
        }

        let effective_limit = if limit == 0 {
            items.len()
        } else {
            limit as usize
        };
        let next_cursor = if items.len() > effective_limit {
            items
                .get(effective_limit.saturating_sub(1))
                .map(|last| last.schedule_id.clone())
        } else {
            None
        };
        items.truncate(effective_limit);
        Ok(RunSchedulePage { items, next_cursor })
    }

    async fn update(
        &self,
        schedule_id: &RunScheduleId,
        update: RunScheduleUpdate,
    ) -> Result<(), RunScheduleRepositoryError> {
        let mut store = self.store.write().await;
        let schedule = store.schedules.get_mut(schedule_id).ok_or_else(|| {
            RunScheduleRepositoryError::NotFound {
                schedule_id: schedule_id.clone(),
            }
        })?;
        apply_update(schedule, update);
        Ok(())
    }

    async fn delete(&self, schedule_id: &RunScheduleId) -> Result<(), RunScheduleRepositoryError> {
        let mut store = self.store.write().await;
        if store.schedules.remove(schedule_id).is_none() {
            return Err(RunScheduleRepositoryError::NotFound {
                schedule_id: schedule_id.clone(),
            });
        }
        Ok(())
    }

    async fn due(
        &self,
        now: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<RunSchedule>, RunScheduleRepositoryError> {
        let store = self.store.read().await;
        let mut items: Vec<RunSchedule> = store
            .schedules
            .values()
            .filter(|s| s.enabled && s.next_tick.is_some_and(|t| t <= now))
            .cloned()
            .collect();
        items.sort_by_key(|s| s.next_tick);
        if limit > 0 {
            items.truncate(limit as usize);
        }
        Ok(items)
    }

    async fn claim_tick(
        &self,
        schedule_id: &RunScheduleId,
        expected_next: DateTime<Utc>,
        new_last: DateTime<Utc>,
        new_next: DateTime<Utc>,
    ) -> Result<bool, RunScheduleRepositoryError> {
        let mut store = self.store.write().await;
        let Some(schedule) = store.schedules.get_mut(schedule_id) else {
            return Ok(false);
        };
        if schedule.next_tick != Some(expected_next) {
            return Ok(false);
        }
        schedule.last_tick = Some(new_last);
        schedule.next_tick = Some(new_next);
        schedule.updated_at = Utc::now();
        Ok(true)
    }

    async fn increment_skipped(
        &self,
        schedule_id: &RunScheduleId,
    ) -> Result<u64, RunScheduleRepositoryError> {
        let mut store = self.store.write().await;
        let schedule = store.schedules.get_mut(schedule_id).ok_or_else(|| {
            RunScheduleRepositoryError::NotFound {
                schedule_id: schedule_id.clone(),
            }
        })?;
        schedule.skipped_ticks += 1;
        schedule.updated_at = Utc::now();
        Ok(schedule.skipped_ticks)
    }
}

#[cfg(test)]
mod contract_suite {
    use super::*;
    use crate::run_schedule::contract_tests;
    use std::sync::Arc;

    // One #[tokio::test] per shared contract function (D-09 precedent, per
    // `crate::assistant::in_memory`'s own convention), each against a fresh
    // `InMemoryRunScheduleRepository`, so a failure names the violated
    // contract clause.

    #[tokio::test]
    async fn insert_creates_and_rejects_duplicate() {
        contract_tests::insert_creates_and_rejects_duplicate(&InMemoryRunScheduleRepository::new())
            .await;
    }

    #[tokio::test]
    async fn get_returns_none_for_unknown() {
        contract_tests::get_returns_none_for_unknown(&InMemoryRunScheduleRepository::new()).await;
    }

    #[tokio::test]
    async fn list_paginates_ascending_by_schedule_id() {
        contract_tests::list_paginates_ascending_by_schedule_id(
            &InMemoryRunScheduleRepository::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn update_applies_partial_changes_and_rejects_unknown() {
        contract_tests::update_applies_partial_changes_and_rejects_unknown(
            &InMemoryRunScheduleRepository::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn delete_removes_and_rejects_unknown() {
        contract_tests::delete_removes_and_rejects_unknown(&InMemoryRunScheduleRepository::new())
            .await;
    }

    #[tokio::test]
    async fn due_filters_enabled_and_next_tick_le_now_ordered_ascending() {
        contract_tests::due_filters_enabled_and_next_tick_le_now_ordered_ascending(
            &InMemoryRunScheduleRepository::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn claim_tick_succeeds_once_then_fails_on_stale_expected() {
        contract_tests::claim_tick_succeeds_once_then_fails_on_stale_expected(
            &InMemoryRunScheduleRepository::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn claim_tick_on_unknown_schedule_returns_false() {
        contract_tests::claim_tick_on_unknown_schedule_returns_false(
            &InMemoryRunScheduleRepository::new(),
        )
        .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn claim_tick_race_admits_exactly_one() {
        let repo: Arc<dyn RunScheduleRepositoryPort> =
            Arc::new(InMemoryRunScheduleRepository::new());
        contract_tests::claim_tick_race_admits_exactly_one(repo).await;
    }

    #[tokio::test]
    async fn increment_skipped_increments_and_persists() {
        contract_tests::increment_skipped_increments_and_persists(
            &InMemoryRunScheduleRepository::new(),
        )
        .await;
    }
}
