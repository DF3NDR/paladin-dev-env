/*
PostgreSQL Run Schedule Repository

Concrete `RunScheduleRepositoryPort` implementation over PostgreSQL, behind
the `postgres` feature (PLAT-05, D-36, D-37, D-38, D-39, Tier 2:
Docker-gated, see `docker/docker-compose.test.yml`'s `postgres-test`
service and `make test-integration-docker`). Mirrors `sqlite.rs` exactly --
same eight methods, same D-37 conditional-`UPDATE` tick claim -- substituting
`$1, $2, ...` placeholders for `?`, `JSONB` (via an explicit `::jsonb` cast
on write) for the TEXT-JSON `input`/`thread_strategy`/`webhook` columns, and
native `BOOLEAN`/`TIMESTAMPTZ` for `enabled` and the timestamp columns.
Every statement uses bound parameters; no query string is ever built by
formatting a caller-supplied value into it.
*/

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::postgres::{PgPool, PgPoolOptions, PgRow};

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
     VALUES ($1, $2, $3, $4, $5, $6::jsonb, $7, $8::jsonb, $9, $10::jsonb, $11, $12, $13, \
             $14, $15, $16)";

const SELECT_BY_ID: &str = "SELECT schedule_id, assistant_id, assistant_version, cron, \
     timezone, input, enabled, thread_strategy, on_missed, webhook, last_tick, next_tick, \
     skipped_ticks, created_at, updated_at, schema_version FROM run_schedules \
     WHERE schedule_id = $1";

const LIST_PREFIX: &str = "SELECT schedule_id, assistant_id, assistant_version, cron, \
     timezone, input, enabled, thread_strategy, on_missed, webhook, last_tick, next_tick, \
     skipped_ticks, created_at, updated_at, schema_version FROM run_schedules WHERE 1 = 1";

const DUE_QUERY: &str = "SELECT schedule_id, assistant_id, assistant_version, cron, \
     timezone, input, enabled, thread_strategy, on_missed, webhook, last_tick, next_tick, \
     skipped_ticks, created_at, updated_at, schema_version FROM run_schedules \
     WHERE enabled = TRUE AND next_tick IS NOT NULL AND next_tick <= $1 \
     ORDER BY next_tick ASC LIMIT $2";

const CLAIM_TICK: &str = "UPDATE run_schedules SET last_tick = $1, next_tick = $2 \
     WHERE schedule_id = $3 AND next_tick = $4";

const DELETE_SCHEDULE: &str = "DELETE FROM run_schedules WHERE schedule_id = $1";

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("migrations/postgres");

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

/// PostgreSQL `RunScheduleRepositoryPort` implementation, behind the
/// `postgres` feature (PLAT-05, D-36, D-37, Tier 2).
#[derive(Debug)]
pub struct PostgresRunScheduleRepository {
    pool: PgPool,
    /// Kept so every error this store returns can be redacted of the
    /// connection URL's password (mirrors `PostgresAssistantRepository`'s
    /// rationale, T-22-18).
    database_url: String,
}

impl PostgresRunScheduleRepository {
    /// Connect to `database_url` and apply the versioned migration. Safe to
    /// call more than once against the same database: the migration is
    /// idempotent and `sqlx::migrate::Migrator` tracks applied versions.
    pub async fn new(database_url: &str) -> Result<Self, RunScheduleRepositoryError> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            // Mirrors `PostgresAssistantRepository::new`'s rationale: a
            // genuinely unreachable server (this Tier 2 suite's local-skip
            // case) surfaces as a fast, clearly-diagnosed error rather than
            // a slow hang absorbing sqlx's default 30s acquire timeout.
            .acquire_timeout(std::time::Duration::from_secs(5))
            .connect(database_url)
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

    /// Map an insert failure: a unique-constraint violation on
    /// `schedule_id` is `AlreadyExists` -- checked BEFORE the generic wrap
    /// (27-RESEARCH.md Pattern 2, SQLSTATE 23505).
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

    fn row_to_schedule(row: &PgRow) -> Result<RunSchedule, RunScheduleRepositoryError> {
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
        let assistant_version: Option<i32> =
            row.try_get("assistant_version").map_err(backend_err)?;
        let cron: String = row.try_get("cron").map_err(backend_err)?;
        let timezone: String = row.try_get("timezone").map_err(backend_err)?;
        let input: serde_json::Value = row.try_get("input").map_err(backend_err)?;
        let enabled: bool = row.try_get("enabled").map_err(backend_err)?;
        let thread_strategy_json: serde_json::Value =
            row.try_get("thread_strategy").map_err(backend_err)?;
        let thread_strategy: ThreadStrategy =
            serde_json::from_value(thread_strategy_json).map_err(ser_err)?;
        let on_missed_str: String = row.try_get("on_missed").map_err(backend_err)?;
        let on_missed = on_missed_from_str(&on_missed_str)?;
        let webhook_json: Option<serde_json::Value> =
            row.try_get("webhook").map_err(backend_err)?;
        let webhook: Option<WebhookSpec> = webhook_json
            .map(serde_json::from_value)
            .transpose()
            .map_err(ser_err)?;
        let last_tick: Option<DateTime<Utc>> = row.try_get("last_tick").map_err(backend_err)?;
        let next_tick: Option<DateTime<Utc>> = row.try_get("next_tick").map_err(backend_err)?;
        let skipped_ticks: i32 = row.try_get("skipped_ticks").map_err(backend_err)?;
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
impl RunScheduleRepositoryPort for PostgresRunScheduleRepository {
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
            .bind(schedule.version.map(|v| v as i32))
            .bind(&schedule.cron)
            .bind(&schedule.timezone)
            .bind(&input)
            .bind(schedule.enabled)
            .bind(&thread_strategy)
            .bind(on_missed_to_str(schedule.on_missed))
            .bind(&webhook)
            .bind(schedule.last_tick)
            .bind(schedule.next_tick)
            .bind(schedule.skipped_ticks as i32)
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

        let mut builder: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(LIST_PREFIX);
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
        let mut builder: sqlx::QueryBuilder<sqlx::Postgres> =
            sqlx::QueryBuilder::new("UPDATE run_schedules SET ");
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
            if !first {
                builder.push(", ");
            }
            first = false;
            builder.push("input = ");
            builder.push_bind(input_str);
            builder.push("::jsonb");
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
            if !first {
                builder.push(", ");
            }
            first = false;
            builder.push("thread_strategy = ");
            builder.push_bind(ts_str);
            builder.push("::jsonb");
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
            if !first {
                builder.push(", ");
            }
            first = false;
            builder.push("webhook = ");
            builder.push_bind(webhook_str);
            builder.push("::jsonb");
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
            "UPDATE run_schedules SET skipped_ticks = skipped_ticks + 1, updated_at = $1 \
             WHERE schedule_id = $2",
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

        let row = sqlx::query("SELECT skipped_ticks FROM run_schedules WHERE schedule_id = $1")
            .bind(schedule_id.as_str())
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| self.wrap_error(e))?;
        let skipped_ticks: i32 = row
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

    // Docker-gated Tier 2 suite (D-51): every test independently probes the
    // shared Postgres service and prints a named `SKIP:` reason then
    // returns early -- never panics or hangs -- when it is not reachable,
    // mirroring `assistant::postgres`'s `store_or_skip` gate exactly. This
    // whole module is ALSO compile-time gated behind the `postgres`
    // feature (see `run_schedule/mod.rs`), which is not in any default
    // feature set.
    //
    // `STORAGE_POSTGRES_TEST_URL` is the SAME storage-wide env var name
    // `run::postgres`/`assistant::postgres` read -- one CI export covers
    // every Tier 2 suite in this crate.
    //
    // Bring the service up before running this suite:
    // ```sh
    // docker compose -f docker/docker-compose.test.yml up -d postgres-test
    // STORAGE_POSTGRES_TEST_URL=postgres://... \
    //   cargo test -p paladin-storage --features postgres --lib run_schedule::postgres
    // ```

    fn postgres_test_url() -> String {
        std::env::var("STORAGE_POSTGRES_TEST_URL").unwrap_or_else(|_| {
            "postgres://paladin:paladin@localhost:5433/paladin_run_test".to_string()
        })
    }

    /// A cheap, short-timeout TCP reachability probe, tried BEFORE handing
    /// `url` to `sqlx`'s pool -- mirrors `assistant::postgres`'s identical
    /// helper.
    fn postgres_reachable(url: &str) -> bool {
        use std::net::ToSocketAddrs;

        let Ok(parsed) = url::Url::parse(url) else {
            return false;
        };
        let Some(host) = parsed.host_str() else {
            return false;
        };
        let port = parsed.port().unwrap_or(5432);

        (host, port)
            .to_socket_addrs()
            .ok()
            .and_then(|mut addrs| addrs.next())
            .is_some_and(|addr| {
                std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(750))
                    .is_ok()
            })
    }

    /// Returns a connected, migrated store, or `None` (after printing a
    /// named reason) if `postgres-test` is not reachable.
    async fn store_or_skip() -> Option<PostgresRunScheduleRepository> {
        let url = postgres_test_url();
        if !postgres_reachable(&url) {
            println!(
                "SKIP: postgres-test not reachable at {url} -- bring it up with \
                 `docker compose -f docker/docker-compose.test.yml up -d postgres-test`"
            );
            return None;
        }

        match PostgresRunScheduleRepository::new(&url).await {
            Ok(store) => Some(store),
            Err(e) => {
                println!("SKIP: postgres-test connection failed at {url} ({e})");
                None
            }
        }
    }

    #[tokio::test]
    async fn insert_creates_and_rejects_duplicate() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::insert_creates_and_rejects_duplicate(&store).await;
    }

    #[tokio::test]
    async fn get_returns_none_for_unknown() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::get_returns_none_for_unknown(&store).await;
    }

    #[tokio::test]
    async fn list_paginates_ascending_by_schedule_id() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::list_paginates_ascending_by_schedule_id(&store).await;
    }

    #[tokio::test]
    async fn update_applies_partial_changes_and_rejects_unknown() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::update_applies_partial_changes_and_rejects_unknown(&store).await;
    }

    #[tokio::test]
    async fn delete_removes_and_rejects_unknown() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::delete_removes_and_rejects_unknown(&store).await;
    }

    #[tokio::test]
    async fn due_filters_enabled_and_next_tick_le_now_ordered_ascending() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::due_filters_enabled_and_next_tick_le_now_ordered_ascending(&store).await;
    }

    #[tokio::test]
    async fn claim_tick_succeeds_once_then_fails_on_stale_expected() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::claim_tick_succeeds_once_then_fails_on_stale_expected(&store).await;
    }

    #[tokio::test]
    async fn claim_tick_on_unknown_schedule_returns_false() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::claim_tick_on_unknown_schedule_returns_false(&store).await;
    }

    #[tokio::test]
    async fn claim_tick_race_admits_exactly_one() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        let store: Arc<dyn RunScheduleRepositoryPort> = Arc::new(store);
        contract_tests::claim_tick_race_admits_exactly_one(store).await;
    }

    #[tokio::test]
    async fn increment_skipped_increments_and_persists() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::increment_skipped_increments_and_persists(&store).await;
    }
}
