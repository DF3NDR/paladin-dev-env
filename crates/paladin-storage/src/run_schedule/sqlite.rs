/*
SQLite Run Schedule Repository

Concrete `RunScheduleRepositoryPort` implementation over SQLite (PLAT-05,
D-36, D-37, D-38, D-39). `claim_tick` is D-37's whole restart/replica-safety
story as ONE conditional `UPDATE run_schedules SET last_tick = ?, next_tick
= ? WHERE schedule_id = ? AND next_tick = ?` -- `rows_affected() == 1` means
this caller won the race, `0` means it lost (a stale `expected_next`, or the
row does not exist), and no other statement in this file ever needs to
inspect it more closely than that boolean. Every statement uses bound
parameters or `sqlx::QueryBuilder`'s `push_bind`; no query string is ever
built by formatting a caller-supplied value into it. Migrations follow the
versioned-file convention at `crates/paladin-storage/migrations/sqlite/`,
embedded at compile time via `sqlx::migrate!` and applied automatically on
construction.
*/

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::sqlite::{Sqlite, SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::{QueryBuilder, Row, sqlite::SqliteRow};
use std::str::FromStr;

use paladin_core::platform::container::run::WebhookSpec;
use paladin_core::platform::container::run_schedule::{
    OnMissed, RUN_SCHEDULE_SCHEMA_VERSION, RunSchedule, RunScheduleId, RunScheduleUpdate,
    ThreadStrategy,
};
use paladin_ports::output::run_schedule_repository_port::{
    RunSchedulePage, RunScheduleRepositoryError, RunScheduleRepositoryPort,
};

use crate::waypoint::redact::redact_database_url_password;

const INSERT_SCHEDULE: &str = "INSERT INTO run_schedules \
     (schedule_id, assistant_id, assistant_version, cron, timezone, input, enabled, \
      thread_strategy, on_missed, webhook, last_tick, next_tick, skipped_ticks, \
      created_at, updated_at, schema_version) \
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";

const SELECT_BY_ID: &str = "SELECT schedule_id, assistant_id, assistant_version, cron, \
     timezone, input, enabled, thread_strategy, on_missed, webhook, last_tick, next_tick, \
     skipped_ticks, created_at, updated_at, schema_version FROM run_schedules \
     WHERE schedule_id = ?";

const LIST_PREFIX: &str = "SELECT schedule_id, assistant_id, assistant_version, cron, \
     timezone, input, enabled, thread_strategy, on_missed, webhook, last_tick, next_tick, \
     skipped_ticks, created_at, updated_at, schema_version FROM run_schedules WHERE 1 = 1";

const DUE_QUERY: &str = "SELECT schedule_id, assistant_id, assistant_version, cron, \
     timezone, input, enabled, thread_strategy, on_missed, webhook, last_tick, next_tick, \
     skipped_ticks, created_at, updated_at, schema_version FROM run_schedules \
     WHERE enabled = 1 AND next_tick IS NOT NULL AND next_tick <= ? \
     ORDER BY next_tick ASC LIMIT ?";

const CLAIM_TICK: &str = "UPDATE run_schedules SET last_tick = ?, next_tick = ? \
     WHERE schedule_id = ? AND next_tick = ?";

const DELETE_SCHEDULE: &str = "DELETE FROM run_schedules WHERE schedule_id = ?";

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("migrations/sqlite");

fn on_missed_to_str(on_missed: OnMissed) -> &'static str {
    match on_missed {
        OnMissed::Skip => "skip",
        OnMissed::RunOnce => "run_once",
    }
}

fn on_missed_from_str(s: &str) -> Result<OnMissed, RunScheduleRepositoryError> {
    match s {
        "skip" => Ok(OnMissed::Skip),
        "run_once" => Ok(OnMissed::RunOnce),
        other => Err(RunScheduleRepositoryError::Serialization {
            message: format!("unknown on_missed value: {other:?}"),
        }),
    }
}

/// SQLite `RunScheduleRepositoryPort` implementation (PLAT-05, Tier 1:
/// always exercised in CI, no external service required).
#[derive(Debug)]
pub struct SqliteRunScheduleRepository {
    pool: SqlitePool,
    /// Kept so every error this store returns can be redacted of the
    /// connection URL's password, mirroring `SqliteAssistantRepository`'s
    /// rationale (T-22-18).
    database_url: String,
}

impl SqliteRunScheduleRepository {
    /// Connect to `database_url`, creating the database file if missing, and
    /// apply the versioned migration. Safe to call more than once against
    /// the same database file: the migration is idempotent and
    /// `sqlx::migrate::Migrator` itself tracks applied versions.
    pub async fn new(database_url: &str) -> Result<Self, RunScheduleRepositoryError> {
        let options = SqliteConnectOptions::from_str(database_url)
            .map_err(|e| Self::wrap(database_url, e))?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .connect_with(options)
            .await
            .map_err(|e| Self::wrap(database_url, e))?;

        MIGRATOR
            .run(&pool)
            .await
            .map_err(|e| Self::wrap(database_url, e))?;

        Ok(Self {
            pool,
            database_url: database_url.to_string(),
        })
    }

    fn wrap(database_url: &str, err: impl std::error::Error) -> RunScheduleRepositoryError {
        let redacted = redact_database_url_password(&err.to_string(), database_url);
        RunScheduleRepositoryError::Backend {
            source: redacted.into(),
        }
    }

    fn wrap_error(&self, err: sqlx::Error) -> RunScheduleRepositoryError {
        Self::wrap(&self.database_url, err)
    }

    /// Map an insert failure: a PRIMARY KEY violation on `schedule_id` is
    /// `AlreadyExists` -- checked BEFORE the generic wrap (27-RESEARCH.md
    /// Pattern 2, mirroring every other adapter in this crate).
    fn map_insert_error(
        &self,
        err: sqlx::Error,
        schedule_id: &RunScheduleId,
    ) -> RunScheduleRepositoryError {
        match &err {
            sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
                RunScheduleRepositoryError::AlreadyExists {
                    schedule_id: schedule_id.clone(),
                }
            }
            _ => self.wrap_error(err),
        }
    }

    fn row_to_schedule(row: &SqliteRow) -> Result<RunSchedule, RunScheduleRepositoryError> {
        let backend_err = |e: sqlx::Error| RunScheduleRepositoryError::Backend { source: e.into() };
        let ser_err = |e: serde_json::Error| RunScheduleRepositoryError::Serialization {
            message: e.to_string(),
        };

        let schedule_id_str: String = row.try_get("schedule_id").map_err(backend_err)?;
        let schedule_id = RunScheduleId::parse(schedule_id_str).map_err(|e| {
            RunScheduleRepositoryError::Serialization {
                message: format!("invalid schedule_id: {e}"),
            }
        })?;
        let assistant_id: String = row.try_get("assistant_id").map_err(backend_err)?;
        let assistant_version: Option<i64> =
            row.try_get("assistant_version").map_err(backend_err)?;
        let cron: String = row.try_get("cron").map_err(backend_err)?;
        let timezone: String = row.try_get("timezone").map_err(backend_err)?;
        let input_str: String = row.try_get("input").map_err(backend_err)?;
        let input: serde_json::Value = serde_json::from_str(&input_str).map_err(ser_err)?;
        let enabled: bool = row.try_get("enabled").map_err(backend_err)?;
        let thread_strategy_str: String = row.try_get("thread_strategy").map_err(backend_err)?;
        let thread_strategy: ThreadStrategy =
            serde_json::from_str(&thread_strategy_str).map_err(ser_err)?;
        let on_missed_str: String = row.try_get("on_missed").map_err(backend_err)?;
        let on_missed = on_missed_from_str(&on_missed_str)?;
        let webhook_str: Option<String> = row.try_get("webhook").map_err(backend_err)?;
        let webhook: Option<WebhookSpec> = webhook_str
            .map(|s| serde_json::from_str(&s))
            .transpose()
            .map_err(ser_err)?;
        let last_tick: Option<DateTime<Utc>> = row.try_get("last_tick").map_err(backend_err)?;
        let next_tick: Option<DateTime<Utc>> = row.try_get("next_tick").map_err(backend_err)?;
        let skipped_ticks: i64 = row.try_get("skipped_ticks").map_err(backend_err)?;
        let created_at: DateTime<Utc> = row.try_get("created_at").map_err(backend_err)?;
        let updated_at: DateTime<Utc> = row.try_get("updated_at").map_err(backend_err)?;
        let schema_version: String = row.try_get("schema_version").map_err(backend_err)?;

        if schema_version != RUN_SCHEDULE_SCHEMA_VERSION {
            return Err(RunScheduleRepositoryError::UnknownSchemaVersion {
                found: schema_version,
            });
        }

        let mut schedule = RunSchedule::new(schedule_id, assistant_id, cron);
        schedule.version = assistant_version.map(|v| v as u32);
        schedule.timezone = timezone;
        schedule.input = input;
        schedule.enabled = enabled;
        schedule.thread_strategy = thread_strategy;
        schedule.on_missed = on_missed;
        schedule.webhook = webhook;
        schedule.last_tick = last_tick;
        schedule.next_tick = next_tick;
        schedule.skipped_ticks = skipped_ticks as u64;
        schedule.created_at = created_at;
        schedule.updated_at = updated_at;
        schedule.schema_version = schema_version;
        Ok(schedule)
    }
}

#[async_trait]
impl RunScheduleRepositoryPort for SqliteRunScheduleRepository {
    async fn insert(&self, schedule: RunSchedule) -> Result<(), RunScheduleRepositoryError> {
        let input = serde_json::to_string(&schedule.input).map_err(|e| {
            RunScheduleRepositoryError::Serialization {
                message: e.to_string(),
            }
        })?;
        let thread_strategy = serde_json::to_string(&schedule.thread_strategy).map_err(|e| {
            RunScheduleRepositoryError::Serialization {
                message: e.to_string(),
            }
        })?;
        let webhook = schedule
            .webhook
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| RunScheduleRepositoryError::Serialization {
                message: e.to_string(),
            })?;

        sqlx::query(INSERT_SCHEDULE)
            .bind(schedule.schedule_id.as_str())
            .bind(&schedule.assistant_id)
            .bind(schedule.version.map(|v| v as i64))
            .bind(&schedule.cron)
            .bind(&schedule.timezone)
            .bind(&input)
            .bind(schedule.enabled)
            .bind(&thread_strategy)
            .bind(on_missed_to_str(schedule.on_missed))
            .bind(&webhook)
            .bind(schedule.last_tick)
            .bind(schedule.next_tick)
            .bind(schedule.skipped_ticks as i64)
            .bind(schedule.created_at)
            .bind(schedule.updated_at)
            .bind(&schedule.schema_version)
            .execute(&self.pool)
            .await
            .map_err(|e| self.map_insert_error(e, &schedule.schedule_id))?;

        Ok(())
    }

    async fn get(
        &self,
        schedule_id: &RunScheduleId,
    ) -> Result<Option<RunSchedule>, RunScheduleRepositoryError> {
        let row = sqlx::query(SELECT_BY_ID)
            .bind(schedule_id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;

        row.as_ref().map(Self::row_to_schedule).transpose()
    }

    async fn list(
        &self,
        limit: u32,
        cursor: Option<RunScheduleId>,
    ) -> Result<RunSchedulePage, RunScheduleRepositoryError> {
        let unlimited = limit == 0;
        let fetch_limit: i64 = if unlimited {
            i64::MAX
        } else {
            limit as i64 + 1
        };

        let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new(LIST_PREFIX);
        if let Some(cursor) = &cursor {
            builder.push(" AND schedule_id > ");
            builder.push_bind(cursor.as_str().to_string());
        }
        builder.push(" ORDER BY schedule_id ASC LIMIT ");
        builder.push_bind(fetch_limit);

        let rows = builder
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;

        let mut items: Vec<RunSchedule> = rows
            .iter()
            .map(Self::row_to_schedule)
            .collect::<Result<_, _>>()?;

        let next_cursor = if !unlimited && items.len() > limit as usize {
            items.pop();
            items.last().map(|last| last.schedule_id.clone())
        } else {
            None
        };

        Ok(RunSchedulePage { items, next_cursor })
    }

    async fn update(
        &self,
        schedule_id: &RunScheduleId,
        update: RunScheduleUpdate,
    ) -> Result<(), RunScheduleRepositoryError> {
        let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new("UPDATE run_schedules SET ");
        let mut first = true;

        macro_rules! set_field {
            ($sql:expr, $value:expr) => {{
                if !first {
                    builder.push(", ");
                }
                first = false;
                builder.push($sql);
                builder.push_bind($value);
            }};
        }

        if let Some(cron) = update.cron {
            set_field!("cron = ", cron);
        }
        if let Some(timezone) = update.timezone {
            set_field!("timezone = ", timezone);
        }
        if let Some(input) = &update.input {
            let input_str = serde_json::to_string(input).map_err(|e| {
                RunScheduleRepositoryError::Serialization {
                    message: e.to_string(),
                }
            })?;
            set_field!("input = ", input_str);
        }
        if let Some(enabled) = update.enabled {
            set_field!("enabled = ", enabled);
        }
        if let Some(thread_strategy) = &update.thread_strategy {
            let ts_str = serde_json::to_string(thread_strategy).map_err(|e| {
                RunScheduleRepositoryError::Serialization {
                    message: e.to_string(),
                }
            })?;
            set_field!("thread_strategy = ", ts_str);
        }
        if let Some(on_missed) = update.on_missed {
            set_field!("on_missed = ", on_missed_to_str(on_missed));
        }
        if let Some(webhook) = &update.webhook {
            let webhook_str = serde_json::to_string(webhook).map_err(|e| {
                RunScheduleRepositoryError::Serialization {
                    message: e.to_string(),
                }
            })?;
            set_field!("webhook = ", webhook_str);
        }
        if let Some(next_tick) = update.next_tick {
            set_field!("next_tick = ", next_tick);
        }

        if !first {
            builder.push(", ");
        }
        builder.push("updated_at = ");
        builder.push_bind(Utc::now());

        builder.push(" WHERE schedule_id = ");
        builder.push_bind(schedule_id.as_str().to_string());

        let result = builder
            .build()
            .execute(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;

        if result.rows_affected() == 0 {
            return Err(RunScheduleRepositoryError::NotFound {
                schedule_id: schedule_id.clone(),
            });
        }
        Ok(())
    }

    async fn delete(&self, schedule_id: &RunScheduleId) -> Result<(), RunScheduleRepositoryError> {
        let result = sqlx::query(DELETE_SCHEDULE)
            .bind(schedule_id.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;

        if result.rows_affected() == 0 {
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
        let fetch_limit: i64 = if limit == 0 { i64::MAX } else { limit as i64 };
        let rows = sqlx::query(DUE_QUERY)
            .bind(now)
            .bind(fetch_limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;

        rows.iter().map(Self::row_to_schedule).collect()
    }

    async fn claim_tick(
        &self,
        schedule_id: &RunScheduleId,
        expected_next: DateTime<Utc>,
        new_last: DateTime<Utc>,
        new_next: DateTime<Utc>,
    ) -> Result<bool, RunScheduleRepositoryError> {
        let result = sqlx::query(CLAIM_TICK)
            .bind(new_last)
            .bind(new_next)
            .bind(schedule_id.as_str())
            .bind(expected_next)
            .execute(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;

        Ok(result.rows_affected() == 1)
    }

    async fn increment_skipped(
        &self,
        schedule_id: &RunScheduleId,
    ) -> Result<u64, RunScheduleRepositoryError> {
        let mut tx = self.pool.begin().await.map_err(|e| self.wrap_error(e))?;

        let result = sqlx::query(
            "UPDATE run_schedules SET skipped_ticks = skipped_ticks + 1, updated_at = ? \
             WHERE schedule_id = ?",
        )
        .bind(Utc::now())
        .bind(schedule_id.as_str())
        .execute(&mut *tx)
        .await
        .map_err(|e| self.wrap_error(e))?;

        if result.rows_affected() == 0 {
            return Err(RunScheduleRepositoryError::NotFound {
                schedule_id: schedule_id.clone(),
            });
        }

        let row = sqlx::query("SELECT skipped_ticks FROM run_schedules WHERE schedule_id = ?")
            .bind(schedule_id.as_str())
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| self.wrap_error(e))?;
        let skipped_ticks: i64 = row
            .try_get("skipped_ticks")
            .map_err(|e| RunScheduleRepositoryError::Backend { source: e.into() })?;

        tx.commit().await.map_err(|e| self.wrap_error(e))?;
        Ok(skipped_ticks as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_schedule::contract_tests;
    use std::sync::Arc;

    // One #[tokio::test] per shared contract function (D-09 precedent),
    // each against a fresh in-memory database, so a failure names the
    // violated contract clause.

    async fn fresh_store() -> SqliteRunScheduleRepository {
        SqliteRunScheduleRepository::new("sqlite::memory:")
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn insert_creates_and_rejects_duplicate() {
        contract_tests::insert_creates_and_rejects_duplicate(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn get_returns_none_for_unknown() {
        contract_tests::get_returns_none_for_unknown(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn list_paginates_ascending_by_schedule_id() {
        contract_tests::list_paginates_ascending_by_schedule_id(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn update_applies_partial_changes_and_rejects_unknown() {
        contract_tests::update_applies_partial_changes_and_rejects_unknown(&fresh_store().await)
            .await;
    }

    #[tokio::test]
    async fn delete_removes_and_rejects_unknown() {
        contract_tests::delete_removes_and_rejects_unknown(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn due_filters_enabled_and_next_tick_le_now_ordered_ascending() {
        contract_tests::due_filters_enabled_and_next_tick_le_now_ordered_ascending(
            &fresh_store().await,
        )
        .await;
    }

    #[tokio::test]
    async fn claim_tick_succeeds_once_then_fails_on_stale_expected() {
        contract_tests::claim_tick_succeeds_once_then_fails_on_stale_expected(&fresh_store().await)
            .await;
    }

    #[tokio::test]
    async fn claim_tick_on_unknown_schedule_returns_false() {
        contract_tests::claim_tick_on_unknown_schedule_returns_false(&fresh_store().await).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn claim_tick_race_admits_exactly_one() {
        // The default-flavor test above uses `sqlite::memory:`, which is a
        // single, private, in-process connection -- fine for sequential
        // clauses, but `claim_tick_race_admits_exactly_one` needs eight
        // tasks racing genuine concurrent connections against the SAME
        // database, so this one clause uses a shared on-disk WAL file
        // (mirrors `SqliteAssistantRepository::new_shared_file`'s exact
        // rationale).
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("run_schedule_claim_race.db");
        let url = format!("sqlite://{}?mode=rwc", path.display());

        let options = SqliteConnectOptions::from_str(&url)
            .unwrap()
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);
        let pool = SqlitePoolOptions::new()
            .connect_with(options)
            .await
            .unwrap();
        MIGRATOR.run(&pool).await.unwrap();

        let store = SqliteRunScheduleRepository {
            pool,
            database_url: url,
        };
        let repo: Arc<dyn RunScheduleRepositoryPort> = Arc::new(store);
        contract_tests::claim_tick_race_admits_exactly_one(repo).await;
    }

    #[tokio::test]
    async fn increment_skipped_increments_and_persists() {
        contract_tests::increment_skipped_increments_and_persists(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn connection_error_redacts_password_from_database_url() {
        let url = "sqlite://user:hunter2-secret@/nonexistent/path/that/does/not/exist.db";
        let err = SqliteRunScheduleRepository::new(url).await.unwrap_err();
        let message = err.to_string();
        assert!(
            !message.contains("hunter2-secret"),
            "connection error leaked the password: {message}"
        );
    }
}
