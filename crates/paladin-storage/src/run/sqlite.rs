/*
SQLite Run Repository

Concrete `RunRepositoryPort` implementation over SQLite (PLAT-01, PLAT-02,
D-03, D-04, D-17). Every status write is a single compare-and-set `UPDATE
... WHERE status = ?from` (D-04, Pattern 1); `insert`'s unique-constraint
violation on `idx_runs_thread_active` maps to `ThreadBusy` via
`DatabaseError::is_unique_violation()` BEFORE the generic error wrap
(D-17, 27-RESEARCH.md Pattern 2) -- `wrap_error` itself never sees that
class of error, unlike the waypoint adapters' collapse-everything helper.
Every statement uses bound parameters or `sqlx::QueryBuilder`'s
`push_bind`; no query string is ever built by formatting a caller-supplied
value into it. Migrations follow the versioned-file convention at
`crates/paladin-storage/migrations/sqlite/` (D-03), embedded at compile
time via `sqlx::migrate!` and applied automatically on construction.
*/

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::sqlite::{Sqlite, SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::{QueryBuilder, Row, sqlite::SqliteRow};
use std::str::FromStr;

use paladin_core::platform::container::parley::ParleyResponse;
use paladin_core::platform::container::run::{
    AssistantRef, ForkSpec, RUN_SCHEMA_VERSION, Run, RunCursor, RunId, RunStatus, WebhookSpec,
};
use paladin_core::platform::container::waypoint::ThreadId;
use paladin_ports::output::run_repository_port::{
    RunOutcomeRecord, RunPage, RunQuery, RunRepositoryError, RunRepositoryPort,
};

use crate::waypoint::redact::redact_database_url_password;

// Every top-level query string below is a plain `&'static str` literal (or,
// for `list`'s dynamic filters, an `sqlx::QueryBuilder` seeded with one) --
// never a runtime string-formatting call building a SQL keyword's text --
// so no caller-supplied value can ever be interpolated into SQL text. The
// `('queued','running','awaiting_input')` busy set appears verbatim in
// three of these constants AND in `idx_runs_thread_active`
// (migration `002_create_runs_table.sql`, D-18): keep all four in sync if
// the busy set ever changes.

const INSERT_RUN: &str = "INSERT INTO runs \
     (run_id, thread_id, assistant_id, assistant_version, status, input, submitted_at, \
      started_at, finished_at, attempt, cancel_requested, error, webhook, pending_responses, \
      fork_from, output, final_waypoint_id, schema_version) \
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";

const SELECT_RUN_BY_ID: &str = "SELECT run_id, thread_id, assistant_id, assistant_version, \
     status, input, submitted_at, started_at, finished_at, attempt, cancel_requested, error, \
     webhook, pending_responses, fork_from, output, final_waypoint_id, schema_version \
     FROM runs WHERE run_id = ?";

const SELECT_ACTIVE_RUN_FOR_THREAD: &str = "SELECT run_id, thread_id, assistant_id, \
     assistant_version, status, input, submitted_at, started_at, finished_at, attempt, \
     cancel_requested, error, webhook, pending_responses, fork_from, output, \
     final_waypoint_id, schema_version FROM runs WHERE thread_id = ? \
     AND status IN ('queued','running','awaiting_input') LIMIT 1";

const UPDATE_REQUEST_CANCEL: &str = "UPDATE runs SET cancel_requested = 1 WHERE run_id = ? \
     AND status IN ('queued','running','awaiting_input')";

const SELECT_CANCEL_REQUESTED_FOR_ACTIVE_THREAD: &str = "SELECT cancel_requested FROM runs \
     WHERE thread_id = ? AND status IN ('queued','running','awaiting_input') LIMIT 1";

const LIST_SELECT_PREFIX: &str = "SELECT run_id, thread_id, assistant_id, assistant_version, \
     status, input, submitted_at, started_at, finished_at, attempt, cancel_requested, error, \
     webhook, pending_responses, fork_from, output, final_waypoint_id, schema_version \
     FROM runs WHERE 1 = 1";

/// D-30: resolves and freezes `assistant_version` onto the new row from the
/// referenced assistant's CURRENT `latest` inside this ONE statement --
/// `assistant_version` is never a bound parameter, it is `a.latest`,
/// selected atomically alongside every other column. Zero rows affected
/// means the `WHERE` clause found no matching, non-deleted assistant row
/// (`RunRepositoryError::UnknownAssistant`, checked by the caller); a
/// unique-constraint violation on `idx_runs_thread_active` (the run WOULD
/// have inserted, but the thread is busy) surfaces as a `sqlx::Error` and is
/// mapped to `ThreadBusy` by the same `map_insert_error` `insert` uses.
const INSERT_RUN_WITH_LATEST: &str = "INSERT INTO runs \
     (run_id, thread_id, assistant_id, assistant_version, status, input, submitted_at, \
      started_at, finished_at, attempt, cancel_requested, error, webhook, pending_responses, \
      fork_from, output, final_waypoint_id, schema_version) \
     SELECT ?, ?, ?, a.latest, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ? \
     FROM assistants a WHERE a.assistant_id = ? AND a.deleted_at IS NULL";

const SELECT_RESOLVED_ASSISTANT_VERSION: &str =
    "SELECT assistant_version FROM runs WHERE run_id = ?";

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("migrations/sqlite");

/// SQLite `RunRepositoryPort` implementation (PLAT-01, Tier 1: always
/// exercised in CI, no external service required).
#[derive(Debug)]
pub struct SqliteRunRepository {
    pool: SqlitePool,
    /// Kept so every error this store returns can be redacted of the
    /// connection URL's password, not just construction-time connection
    /// errors -- mirrors `SqliteWaypointStore`'s rationale (T-22-18).
    database_url: String,
}

impl SqliteRunRepository {
    /// Connect to `database_url`, creating the database file if missing, and
    /// apply the versioned migration. Safe to call more than once against
    /// the same database file: the migration is idempotent (`CREATE TABLE
    /// IF NOT EXISTS`/`CREATE INDEX IF NOT EXISTS`) and
    /// `sqlx::migrate::Migrator` itself tracks applied versions.
    pub async fn new(database_url: &str) -> Result<Self, RunRepositoryError> {
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

    /// Connect over a shared on-disk file with WAL journaling, so multiple
    /// pooled connections (needed for the true-concurrency stress test,
    /// D-52) observe each other's writes -- unlike `new`, which callers use
    /// with `sqlite::memory:` and a single connection. Test-only: production
    /// callers always go through `new`.
    #[cfg(test)]
    async fn new_shared_file(database_url: &str) -> Result<Self, RunRepositoryError> {
        let options = SqliteConnectOptions::from_str(database_url)
            .map_err(|e| Self::wrap(database_url, e))?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

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

    /// Wrap a driver/migration error into `RunRepositoryError::Backend`,
    /// with the connection URL's password redacted from the error text
    /// first (redact before any truncation, per this project's security
    /// instructions).
    fn wrap(database_url: &str, err: impl std::error::Error) -> RunRepositoryError {
        let redacted = redact_database_url_password(&err.to_string(), database_url);
        RunRepositoryError::Backend {
            source: redacted.into(),
        }
    }

    fn wrap_error(&self, err: sqlx::Error) -> RunRepositoryError {
        Self::wrap(&self.database_url, err)
    }

    /// Map an `insert` failure: a unique-constraint violation on
    /// `idx_runs_thread_active` is `ThreadBusy` (D-17) -- checked BEFORE the
    /// generic wrap, per 27-RESEARCH.md Pattern 2. Every other `sqlx::Error`
    /// falls through to the generic `Backend` wrap.
    fn map_insert_error(&self, err: sqlx::Error, thread_id: &ThreadId) -> RunRepositoryError {
        match &err {
            sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
                RunRepositoryError::ThreadBusy {
                    thread_id: thread_id.clone(),
                }
            }
            _ => self.wrap_error(err),
        }
    }

    /// `SELECT status FROM runs WHERE run_id = ?`, `None` if the row does
    /// not exist -- used to distinguish `NotFound` from a CAS-affected-zero
    /// -rows failure across `update_status`/`request_cancel`/`record_resume`.
    async fn current_status(
        &self,
        run_id: &RunId,
    ) -> Result<Option<RunStatus>, RunRepositoryError> {
        let row = sqlx::query("SELECT status FROM runs WHERE run_id = ?")
            .bind(run_id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;
        match row {
            Some(row) => {
                let status_str: String = row
                    .try_get("status")
                    .map_err(|e| RunRepositoryError::Backend { source: e.into() })?;
                let status = RunStatus::from_str(&status_str).map_err(|e| {
                    RunRepositoryError::Serialization {
                        message: e.to_string(),
                    }
                })?;
                Ok(Some(status))
            }
            None => Ok(None),
        }
    }

    /// Deserialize a full `Run` from a `runs` row, enforcing the
    /// schema-version check (X-04): a row written by an unrecognised newer
    /// release maps to `UnknownSchemaVersion` rather than a
    /// structurally-successful misparse.
    fn row_to_run(row: &SqliteRow) -> Result<Run, RunRepositoryError> {
        let backend_err = |e: sqlx::Error| RunRepositoryError::Backend { source: e.into() };
        let ser_err = |e: serde_json::Error| RunRepositoryError::Serialization {
            message: e.to_string(),
        };

        let run_id_str: String = row.try_get("run_id").map_err(backend_err)?;
        let run_id = RunId::parse(run_id_str).map_err(|e| RunRepositoryError::Serialization {
            message: format!("invalid run_id: {e}"),
        })?;

        let thread_id_str: String = row.try_get("thread_id").map_err(backend_err)?;
        let thread_id =
            ThreadId::new(thread_id_str).map_err(|e| RunRepositoryError::Serialization {
                message: format!("invalid thread_id: {e}"),
            })?;

        let assistant_id: String = row.try_get("assistant_id").map_err(backend_err)?;
        let assistant_version: i64 = row.try_get("assistant_version").map_err(backend_err)?;

        let status_str: String = row.try_get("status").map_err(backend_err)?;
        let status =
            RunStatus::from_str(&status_str).map_err(|e| RunRepositoryError::Serialization {
                message: e.to_string(),
            })?;

        let input_str: String = row.try_get("input").map_err(backend_err)?;
        let input: serde_json::Value = serde_json::from_str(&input_str).map_err(ser_err)?;

        let submitted_at: DateTime<Utc> = row.try_get("submitted_at").map_err(backend_err)?;
        let started_at: Option<DateTime<Utc>> = row.try_get("started_at").map_err(backend_err)?;
        let finished_at: Option<DateTime<Utc>> = row.try_get("finished_at").map_err(backend_err)?;

        let attempt: i64 = row.try_get("attempt").map_err(backend_err)?;
        let cancel_requested: i64 = row.try_get("cancel_requested").map_err(backend_err)?;
        let error: Option<String> = row.try_get("error").map_err(backend_err)?;

        let webhook_str: Option<String> = row.try_get("webhook").map_err(backend_err)?;
        let webhook: Option<WebhookSpec> = webhook_str
            .map(|s| serde_json::from_str(&s))
            .transpose()
            .map_err(ser_err)?;

        let pending_responses_str: String =
            row.try_get("pending_responses").map_err(backend_err)?;
        let pending_responses: Vec<ParleyResponse> =
            serde_json::from_str(&pending_responses_str).map_err(ser_err)?;

        let fork_from_str: Option<String> = row.try_get("fork_from").map_err(backend_err)?;
        let fork_from: Option<ForkSpec> = fork_from_str
            .map(|s| serde_json::from_str(&s))
            .transpose()
            .map_err(ser_err)?;

        let output_str: Option<String> = row.try_get("output").map_err(backend_err)?;
        let output: Option<serde_json::Value> = output_str
            .map(|s| serde_json::from_str(&s))
            .transpose()
            .map_err(ser_err)?;

        let final_waypoint_id: Option<String> =
            row.try_get("final_waypoint_id").map_err(backend_err)?;
        let schema_version: String = row.try_get("schema_version").map_err(backend_err)?;

        if schema_version != RUN_SCHEMA_VERSION {
            return Err(RunRepositoryError::UnknownSchemaVersion {
                found: schema_version,
            });
        }

        let mut run = Run::new(
            run_id,
            thread_id,
            AssistantRef {
                assistant_id,
                version: assistant_version as u32,
            },
            input,
        );
        run.status = status;
        run.submitted_at = submitted_at;
        run.started_at = started_at;
        run.finished_at = finished_at;
        run.attempt = attempt as u32;
        run.cancel_requested = cancel_requested != 0;
        run.error = error;
        run.webhook = webhook;
        run.pending_responses = pending_responses;
        run.fork_from = fork_from;
        run.output = output;
        run.final_waypoint_id = final_waypoint_id;
        run.schema_version = schema_version;
        Ok(run)
    }
}

#[async_trait]
impl RunRepositoryPort for SqliteRunRepository {
    async fn insert(&self, run: &Run) -> Result<(), RunRepositoryError> {
        let input =
            serde_json::to_string(&run.input).map_err(|e| RunRepositoryError::Serialization {
                message: e.to_string(),
            })?;
        let webhook = run
            .webhook
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| RunRepositoryError::Serialization {
                message: e.to_string(),
            })?;
        let pending_responses = serde_json::to_string(&run.pending_responses).map_err(|e| {
            RunRepositoryError::Serialization {
                message: e.to_string(),
            }
        })?;
        let fork_from = run
            .fork_from
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| RunRepositoryError::Serialization {
                message: e.to_string(),
            })?;
        let output = run
            .output
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| RunRepositoryError::Serialization {
                message: e.to_string(),
            })?;

        sqlx::query(INSERT_RUN)
            .bind(run.run_id.as_str())
            .bind(run.thread_id.as_str())
            .bind(&run.assistant.assistant_id)
            .bind(run.assistant.version as i64)
            .bind(run.status.as_str())
            .bind(input)
            .bind(run.submitted_at)
            .bind(run.started_at)
            .bind(run.finished_at)
            .bind(run.attempt as i64)
            .bind(run.cancel_requested as i64)
            .bind(&run.error)
            .bind(webhook)
            .bind(pending_responses)
            .bind(fork_from)
            .bind(output)
            .bind(&run.final_waypoint_id)
            .bind(&run.schema_version)
            .execute(&self.pool)
            .await
            .map_err(|e| self.map_insert_error(e, &run.thread_id))?;

        Ok(())
    }

    async fn get(&self, run_id: &RunId) -> Result<Option<Run>, RunRepositoryError> {
        let row = sqlx::query(SELECT_RUN_BY_ID)
            .bind(run_id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;

        row.as_ref().map(Self::row_to_run).transpose()
    }

    async fn update_status(
        &self,
        run_id: &RunId,
        from: RunStatus,
        to: RunStatus,
        at: DateTime<Utc>,
    ) -> Result<(), RunRepositoryError> {
        // Legality is checked against the pure state machine FIRST: without
        // this, a stale `from` that happens to equal the row's actual
        // (different) current status could still match the CAS predicate
        // below if the caller passed a `to` that is illegal from the row's
        // real status but legal from the (wrong) `from` supplied.
        RunStatus::try_transition(from, to)
            .map_err(|_| RunRepositoryError::IllegalTransition { from, to })?;

        let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new("UPDATE runs SET status = ");
        builder.push_bind(to.as_str().to_string());
        if to == RunStatus::Running {
            builder.push(", started_at = ");
            builder.push_bind(at);
        }
        if to.is_terminal() {
            builder.push(", finished_at = ");
            builder.push_bind(at);
        }
        builder.push(" WHERE run_id = ");
        builder.push_bind(run_id.as_str().to_string());
        builder.push(" AND status = ");
        builder.push_bind(from.as_str().to_string());

        let result = builder
            .build()
            .execute(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;

        if result.rows_affected() == 0 {
            return match self.current_status(run_id).await? {
                None => Err(RunRepositoryError::NotFound {
                    run_id: run_id.clone(),
                }),
                Some(_) => Err(RunRepositoryError::IllegalTransition { from, to }),
            };
        }
        Ok(())
    }

    async fn record_outcome(
        &self,
        run_id: &RunId,
        outcome: RunOutcomeRecord,
    ) -> Result<(), RunRepositoryError> {
        let output = outcome
            .output
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| RunRepositoryError::Serialization {
                message: e.to_string(),
            })?;

        let result = sqlx::query(
            "UPDATE runs SET error = ?, output = ?, final_waypoint_id = ? WHERE run_id = ?",
        )
        .bind(&outcome.error)
        .bind(output)
        .bind(&outcome.final_waypoint_id)
        .bind(run_id.as_str())
        .execute(&self.pool)
        .await
        .map_err(|e| self.wrap_error(e))?;

        if result.rows_affected() == 0 {
            return Err(RunRepositoryError::NotFound {
                run_id: run_id.clone(),
            });
        }
        Ok(())
    }

    async fn list(&self, query: RunQuery) -> Result<RunPage, RunRepositoryError> {
        let unlimited = query.limit == 0;
        let effective_limit: u32 = if unlimited { u32::MAX } else { query.limit };
        let fetch_limit: i64 = if unlimited {
            effective_limit as i64
        } else {
            effective_limit as i64 + 1
        };

        let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new(LIST_SELECT_PREFIX);
        if let Some(thread_id) = &query.thread_id {
            builder.push(" AND thread_id = ");
            builder.push_bind(thread_id.as_str().to_string());
        }
        if let Some(assistant_id) = &query.assistant_id {
            builder.push(" AND assistant_id = ");
            builder.push_bind(assistant_id.clone());
        }
        if let Some(status) = &query.status {
            builder.push(" AND status = ");
            builder.push_bind(status.as_str().to_string());
        }
        if let Some(cursor) = &query.cursor {
            builder.push(" AND (submitted_at < ");
            builder.push_bind(cursor.submitted_at);
            builder.push(" OR (submitted_at = ");
            builder.push_bind(cursor.submitted_at);
            builder.push(" AND run_id < ");
            builder.push_bind(cursor.run_id.as_str().to_string());
            builder.push("))");
        }
        builder.push(" ORDER BY submitted_at DESC, run_id DESC LIMIT ");
        builder.push_bind(fetch_limit);

        let rows = builder
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;

        let mut items: Vec<Run> = rows
            .iter()
            .map(Self::row_to_run)
            .collect::<Result<_, _>>()?;

        let next_cursor = if !unlimited && items.len() > effective_limit as usize {
            items.pop();
            items.last().map(|last| RunCursor {
                submitted_at: last.submitted_at,
                run_id: last.run_id.clone(),
            })
        } else {
            None
        };

        Ok(RunPage { items, next_cursor })
    }

    async fn active_run_for_thread(
        &self,
        thread_id: &ThreadId,
    ) -> Result<Option<Run>, RunRepositoryError> {
        let row = sqlx::query(SELECT_ACTIVE_RUN_FOR_THREAD)
            .bind(thread_id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;

        row.as_ref().map(Self::row_to_run).transpose()
    }

    async fn request_cancel(&self, run_id: &RunId) -> Result<RunStatus, RunRepositoryError> {
        let result = sqlx::query(UPDATE_REQUEST_CANCEL)
            .bind(run_id.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;

        if result.rows_affected() == 0 {
            return match self.current_status(run_id).await? {
                None => Err(RunRepositoryError::NotFound {
                    run_id: run_id.clone(),
                }),
                Some(status) => Err(RunRepositoryError::AlreadyTerminal {
                    run_id: run_id.clone(),
                    status,
                }),
            };
        }

        self.current_status(run_id)
            .await?
            .ok_or_else(|| RunRepositoryError::NotFound {
                run_id: run_id.clone(),
            })
    }

    async fn is_cancel_requested(&self, thread_id: &ThreadId) -> Result<bool, RunRepositoryError> {
        let row = sqlx::query(SELECT_CANCEL_REQUESTED_FOR_ACTIVE_THREAD)
            .bind(thread_id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;

        match row {
            Some(row) => {
                let cancel_requested: i64 = row
                    .try_get("cancel_requested")
                    .map_err(|e| RunRepositoryError::Backend { source: e.into() })?;
                Ok(cancel_requested != 0)
            }
            None => Ok(false),
        }
    }

    async fn bump_attempt(&self, run_id: &RunId) -> Result<u32, RunRepositoryError> {
        let result = sqlx::query("UPDATE runs SET attempt = attempt + 1 WHERE run_id = ?")
            .bind(run_id.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;

        if result.rows_affected() == 0 {
            return Err(RunRepositoryError::NotFound {
                run_id: run_id.clone(),
            });
        }

        let row = sqlx::query("SELECT attempt FROM runs WHERE run_id = ?")
            .bind(run_id.as_str())
            .fetch_one(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;
        let attempt: i64 = row
            .try_get("attempt")
            .map_err(|e| RunRepositoryError::Backend { source: e.into() })?;
        Ok(attempt as u32)
    }

    async fn record_resume(
        &self,
        run_id: &RunId,
        responses: Vec<ParleyResponse>,
    ) -> Result<u32, RunRepositoryError> {
        let pending_responses =
            serde_json::to_string(&responses).map_err(|e| RunRepositoryError::Serialization {
                message: e.to_string(),
            })?;

        let result = sqlx::query(
            "UPDATE runs SET pending_responses = ?, attempt = attempt + 1 \
             WHERE run_id = ? AND status = 'awaiting_input'",
        )
        .bind(pending_responses)
        .bind(run_id.as_str())
        .execute(&self.pool)
        .await
        .map_err(|e| self.wrap_error(e))?;

        if result.rows_affected() == 0 {
            let from =
                self.current_status(run_id)
                    .await?
                    .ok_or_else(|| RunRepositoryError::NotFound {
                        run_id: run_id.clone(),
                    })?;
            return Err(RunRepositoryError::IllegalTransition {
                from,
                to: RunStatus::AwaitingInput,
            });
        }

        let row = sqlx::query("SELECT attempt FROM runs WHERE run_id = ?")
            .bind(run_id.as_str())
            .fetch_one(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;
        let attempt: i64 = row
            .try_get("attempt")
            .map_err(|e| RunRepositoryError::Backend { source: e.into() })?;
        Ok(attempt as u32)
    }

    async fn clear_pending_responses(&self, run_id: &RunId) -> Result<(), RunRepositoryError> {
        let result = sqlx::query("UPDATE runs SET pending_responses = '[]' WHERE run_id = ?")
            .bind(run_id.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;

        if result.rows_affected() == 0 {
            return Err(RunRepositoryError::NotFound {
                run_id: run_id.clone(),
            });
        }
        Ok(())
    }

    async fn insert_with_latest(&self, run: &Run) -> Result<u32, RunRepositoryError> {
        let input =
            serde_json::to_string(&run.input).map_err(|e| RunRepositoryError::Serialization {
                message: e.to_string(),
            })?;
        let webhook = run
            .webhook
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| RunRepositoryError::Serialization {
                message: e.to_string(),
            })?;
        let pending_responses = serde_json::to_string(&run.pending_responses).map_err(|e| {
            RunRepositoryError::Serialization {
                message: e.to_string(),
            }
        })?;
        let fork_from = run
            .fork_from
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| RunRepositoryError::Serialization {
                message: e.to_string(),
            })?;
        let output = run
            .output
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| RunRepositoryError::Serialization {
                message: e.to_string(),
            })?;

        let result = sqlx::query(INSERT_RUN_WITH_LATEST)
            .bind(run.run_id.as_str())
            .bind(run.thread_id.as_str())
            .bind(&run.assistant.assistant_id)
            .bind(run.status.as_str())
            .bind(input)
            .bind(run.submitted_at)
            .bind(run.started_at)
            .bind(run.finished_at)
            .bind(run.attempt as i64)
            .bind(run.cancel_requested as i64)
            .bind(&run.error)
            .bind(webhook)
            .bind(pending_responses)
            .bind(fork_from)
            .bind(output)
            .bind(&run.final_waypoint_id)
            .bind(&run.schema_version)
            .bind(&run.assistant.assistant_id)
            .execute(&self.pool)
            .await
            .map_err(|e| self.map_insert_error(e, &run.thread_id))?;

        if result.rows_affected() == 0 {
            return Err(RunRepositoryError::UnknownAssistant {
                assistant_id: run.assistant.assistant_id.clone(),
            });
        }

        let row = sqlx::query(SELECT_RESOLVED_ASSISTANT_VERSION)
            .bind(run.run_id.as_str())
            .fetch_one(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;
        let version: i64 = row
            .try_get("assistant_version")
            .map_err(|e| RunRepositoryError::Backend { source: e.into() })?;
        Ok(version as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::contract_tests;
    use std::sync::Arc;

    // One #[tokio::test] per shared contract function (D-09 precedent),
    // each against a fresh in-memory database, so a failure names the
    // violated contract clause. See `contract_tests` for the assertions
    // themselves -- this file only wires `SqliteRunRepository` into them,
    // unchanged.

    async fn fresh_store() -> SqliteRunRepository {
        SqliteRunRepository::new("sqlite::memory:").await.unwrap()
    }

    #[tokio::test]
    async fn insert_then_get_round_trips_every_field() {
        contract_tests::insert_then_get_round_trips_every_field(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn update_status_queued_to_running_sets_started_at_then_stale_cas_fails() {
        contract_tests::update_status_queued_to_running_sets_started_at_then_stale_cas_fails(
            &fresh_store().await,
        )
        .await;
    }

    #[tokio::test]
    async fn update_status_running_to_completed_sets_finished_at_then_terminal_is_absorbing() {
        contract_tests::update_status_running_to_completed_sets_finished_at_then_terminal_is_absorbing(
            &fresh_store().await,
        )
        .await;
    }

    #[tokio::test]
    async fn update_status_self_transition_fails() {
        contract_tests::update_status_self_transition_fails(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn insert_rejects_second_active_run_then_succeeds_after_terminal() {
        contract_tests::insert_rejects_second_active_run_then_succeeds_after_terminal(
            &fresh_store().await,
        )
        .await;
    }

    #[tokio::test]
    async fn list_paginates_by_submitted_at_and_run_id_with_no_overlap_or_gap() {
        contract_tests::list_paginates_by_submitted_at_and_run_id_with_no_overlap_or_gap(
            &fresh_store().await,
        )
        .await;
    }

    #[tokio::test]
    async fn list_filters_by_thread_assistant_and_status() {
        contract_tests::list_filters_by_thread_assistant_and_status(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn request_cancel_is_idempotent_and_rejects_terminal() {
        contract_tests::request_cancel_is_idempotent_and_rejects_terminal(&fresh_store().await)
            .await;
    }

    #[tokio::test]
    async fn is_cancel_requested_reflects_active_run_flag() {
        contract_tests::is_cancel_requested_reflects_active_run_flag(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn bump_attempt_increments_and_persists() {
        contract_tests::bump_attempt_increments_and_persists(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn record_resume_on_awaiting_input_then_clear_pending_responses() {
        contract_tests::record_resume_on_awaiting_input_then_clear_pending_responses(
            &fresh_store().await,
        )
        .await;
    }

    #[tokio::test]
    async fn record_outcome_persists_fields_without_touching_status() {
        contract_tests::record_outcome_persists_fields_without_touching_status(
            &fresh_store().await,
        )
        .await;
    }

    #[tokio::test]
    async fn get_on_unsupported_schema_version_fails() {
        contract_tests::get_on_unsupported_schema_version_fails(&fresh_store().await).await;
    }

    // The one clause that needs a REAL shared on-disk database -- proving
    // the partial unique index (not an in-process lock) enforces the
    // one-active-run-per-thread invariant under true multi-connection
    // concurrency (D-52).
    #[tokio::test(flavor = "multi_thread")]
    async fn ten_concurrent_inserts_one_thread_exactly_one_accepted_on_disk() {
        let path = std::env::temp_dir().join(format!(
            "paladin_run_repo_concurrency_test_{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let url = format!("sqlite://{}", path.display());
        let store: Arc<dyn RunRepositoryPort> =
            Arc::new(SqliteRunRepository::new_shared_file(&url).await.unwrap());

        contract_tests::ten_concurrent_inserts_one_thread_exactly_one_accepted(store).await;

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(format!("{}-wal", path.display()));
        let _ = std::fs::remove_file(format!("{}-shm", path.display()));
    }

    #[tokio::test]
    async fn connection_error_redacts_password_from_database_url() {
        let url = "sqlite://user:hunter2-secret@/nonexistent/path/that/does/not/exist.db";
        let err = SqliteRunRepository::new(url).await.unwrap_err();
        let message = err.to_string();
        assert!(
            !message.contains("hunter2-secret"),
            "connection error leaked the password: {message}"
        );
    }

    // ── insert_with_latest / freeze-at-submit (D-30) ─────────────────────
    // These clauses need a `SqliteRunRepository` AND a
    // `SqliteAssistantRepository` reading/writing the SAME on-disk
    // database -- the `INSERT ... SELECT ... FROM assistants` statement
    // (`INSERT_RUN_WITH_LATEST`) only sees an assistant row committed
    // through the SAME file, and `sqlx::migrate!` embeds every file under
    // `migrations/sqlite/` (so either constructor's own migration run
    // already creates both tables on that shared file).

    async fn shared_file_stores() -> (
        SqliteRunRepository,
        crate::assistant::sqlite::SqliteAssistantRepository,
        std::path::PathBuf,
    ) {
        let path = std::env::temp_dir().join(format!(
            "paladin_run_assistant_freeze_test_{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let url = format!("sqlite://{}", path.display());
        let run_repo = SqliteRunRepository::new_shared_file(&url).await.unwrap();
        let assistant_repo =
            crate::assistant::sqlite::SqliteAssistantRepository::new_shared_file(&url)
                .await
                .unwrap();
        (run_repo, assistant_repo, path)
    }

    fn cleanup_shared_file(path: &std::path::Path) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path.display()));
        let _ = std::fs::remove_file(format!("{}-shm", path.display()));
    }

    #[tokio::test]
    async fn insert_with_latest_resolves_current_latest_and_freezes_it() {
        let (run_repo, assistant_repo, path) = shared_file_stores().await;
        contract_tests::insert_with_latest_resolves_current_latest_and_freezes_it(
            &run_repo,
            &assistant_repo,
        )
        .await;
        cleanup_shared_file(&path);
    }

    #[tokio::test]
    async fn insert_with_latest_unknown_assistant_fails() {
        let (run_repo, _assistant_repo, path) = shared_file_stores().await;
        contract_tests::insert_with_latest_unknown_assistant_fails(&run_repo).await;
        cleanup_shared_file(&path);
    }

    #[tokio::test]
    async fn insert_with_latest_soft_deleted_assistant_fails() {
        let (run_repo, assistant_repo, path) = shared_file_stores().await;
        contract_tests::insert_with_latest_soft_deleted_assistant_fails(&run_repo, &assistant_repo)
            .await;
        cleanup_shared_file(&path);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn assistant_version_freeze_at_submit() {
        let (run_repo, assistant_repo, path) = shared_file_stores().await;
        let run_repo: Arc<dyn RunRepositoryPort> = Arc::new(run_repo);
        let assistant_repo: Arc<
            dyn paladin_ports::output::assistant_repository_port::AssistantRepositoryPort,
        > = Arc::new(assistant_repo);
        contract_tests::assistant_version_freeze_at_submit(run_repo, assistant_repo).await;
        cleanup_shared_file(&path);
    }
}
