/*
PostgreSQL Run Repository

Concrete `RunRepositoryPort` implementation over PostgreSQL, behind the
`postgres` feature (PLAT-01, PLAT-02, D-03, Tier 2: Docker-gated, see
`docker/docker-compose.test.yml`'s `postgres-test` service and `make
test-integration-docker`). Mirrors `sqlite.rs` exactly -- same eleven
methods, same CAS `update_status` (D-04, Pattern 1), same
is_unique_violation()-before-generic-wrap mapping to `ThreadBusy` (D-17,
27-RESEARCH.md Pattern 2) -- substituting `$1, $2, ...` placeholders for
`?`, `JSONB` (via an explicit `::jsonb` cast on write, exactly the
`payload` precedent in `waypoint/postgres.rs`) for the TEXT JSON columns,
native `BOOLEAN` for `cancel_requested`, and native `TIMESTAMPTZ` for the
three timestamp columns. Every statement uses bound parameters; no query
string is ever built by formatting a caller-supplied value into it.

Precision contract (D-01, D-03, D-04): `TIMESTAMPTZ` holds microsecond
resolution, so every `chrono::DateTime<Utc>` bound into `submitted_at`,
`started_at` or `finished_at` -- in `INSERT_RUN`, `INSERT_RUN_WITH_LATEST`
and `update_status`'s `QueryBuilder` pushes -- is normalised through
`crate::run::storage_timestamp` first. That function's own rustdoc is the
single source of truth for the contract (truncation toward zero, not
rounding); this module never reimplements it.
*/

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::postgres::{PgPool, PgPoolOptions, PgRow};

use paladin_core::platform::container::parley::ParleyResponse;
use paladin_core::platform::container::run::{
    AssistantRef, ForkSpec, RUN_SCHEMA_VERSION, Run, RunCursor, RunId, RunStatus, WebhookSpec,
};
use paladin_core::platform::container::waypoint::ThreadId;
use paladin_ports::output::run_repository_port::{
    RunOutcomeRecord, RunPage, RunQuery, RunRepositoryError, RunRepositoryPort,
};

use crate::run::storage_timestamp;
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
     VALUES ($1, $2, $3, $4, $5, $6::jsonb, $7, $8, $9, $10, $11, $12, $13::jsonb, $14::jsonb, \
             $15::jsonb, $16::jsonb, $17, $18)";

const SELECT_RUN_BY_ID: &str = "SELECT run_id, thread_id, assistant_id, assistant_version, \
     status, input, submitted_at, started_at, finished_at, attempt, cancel_requested, error, \
     webhook, pending_responses, fork_from, output, final_waypoint_id, schema_version \
     FROM runs WHERE run_id = $1";

const SELECT_ACTIVE_RUN_FOR_THREAD: &str = "SELECT run_id, thread_id, assistant_id, \
     assistant_version, status, input, submitted_at, started_at, finished_at, attempt, \
     cancel_requested, error, webhook, pending_responses, fork_from, output, \
     final_waypoint_id, schema_version FROM runs WHERE thread_id = $1 \
     AND status IN ('queued','running','awaiting_input') LIMIT 1";

const UPDATE_REQUEST_CANCEL: &str = "UPDATE runs SET cancel_requested = TRUE WHERE run_id = $1 \
     AND status IN ('queued','running','awaiting_input')";

const SELECT_CANCEL_REQUESTED_FOR_ACTIVE_THREAD: &str = "SELECT cancel_requested FROM runs \
     WHERE thread_id = $1 AND status IN ('queued','running','awaiting_input') LIMIT 1";

const LIST_SELECT_PREFIX: &str = "SELECT run_id, thread_id, assistant_id, assistant_version, \
     status, input, submitted_at, started_at, finished_at, attempt, cancel_requested, error, \
     webhook, pending_responses, fork_from, output, final_waypoint_id, schema_version \
     FROM runs WHERE 1 = 1";

/// D-30: resolves and freezes `assistant_version` onto the new row from the
/// referenced assistant's CURRENT `latest` inside this ONE statement --
/// mirrors `sqlite.rs`'s `INSERT_RUN_WITH_LATEST` exactly (see its doc
/// comment for the full rationale), substituting `$N` placeholders and the
/// `::jsonb` casts `INSERT_RUN` already uses.
const INSERT_RUN_WITH_LATEST: &str = "INSERT INTO runs \
     (run_id, thread_id, assistant_id, assistant_version, status, input, submitted_at, \
      started_at, finished_at, attempt, cancel_requested, error, webhook, pending_responses, \
      fork_from, output, final_waypoint_id, schema_version) \
     SELECT $1, $2, $3, a.latest, $4, $5::jsonb, $6, $7, $8, $9, $10, $11, $12::jsonb, \
            $13::jsonb, $14::jsonb, $15::jsonb, $16, $17 \
     FROM assistants a WHERE a.assistant_id = $18 AND a.deleted_at IS NULL";

const SELECT_RESOLVED_ASSISTANT_VERSION: &str =
    "SELECT assistant_version FROM runs WHERE run_id = $1";

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("migrations/postgres");

/// PostgreSQL `RunRepositoryPort` implementation, behind the `postgres`
/// feature (PLAT-01, D-03, Tier 2).
#[derive(Debug)]
pub struct PostgresRunRepository {
    pool: PgPool,
    /// Kept so every error this store returns can be redacted of the
    /// connection URL's password (mirrors `PostgresWaypointStore`'s
    /// rationale, T-22-18).
    database_url: String,
}

impl PostgresRunRepository {
    /// Connect to `database_url` and apply the versioned migration. Safe to
    /// call more than once against the same database: the migration is
    /// idempotent and `sqlx::migrate::Migrator` tracks applied versions.
    pub async fn new(database_url: &str) -> Result<Self, RunRepositoryError> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            // Mirrors `PostgresWaypointStore::new`'s rationale: a genuinely
            // unreachable server (this Tier 2 suite's local-skip case)
            // surfaces as a fast, clearly-diagnosed error rather than a
            // slow hang absorbing sqlx's default 30s acquire timeout.
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
    /// generic wrap, per 27-RESEARCH.md Pattern 2 (SQLSTATE 23505). Every
    /// other `sqlx::Error` falls through to the generic `Backend` wrap.
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

    /// `SELECT status FROM runs WHERE run_id = $1`, `None` if the row does
    /// not exist -- used to distinguish `NotFound` from a CAS-affected-zero
    /// -rows failure across `update_status`/`request_cancel`/`record_resume`.
    async fn current_status(
        &self,
        run_id: &RunId,
    ) -> Result<Option<RunStatus>, RunRepositoryError> {
        let row = sqlx::query("SELECT status FROM runs WHERE run_id = $1")
            .bind(run_id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;
        match row {
            Some(row) => {
                let status_str: String = row
                    .try_get("status")
                    .map_err(|e| RunRepositoryError::Backend { source: e.into() })?;
                let status: RunStatus = status_str.parse().map_err(
                    |e: paladin_core::platform::container::run::RunStatusParseError| {
                        RunRepositoryError::Serialization {
                            message: e.to_string(),
                        }
                    },
                )?;
                Ok(Some(status))
            }
            None => Ok(None),
        }
    }

    /// Deserialize a full `Run` from a `runs` row, enforcing the
    /// schema-version check (X-04): a row written by an unrecognised newer
    /// release maps to `UnknownSchemaVersion` rather than a
    /// structurally-successful misparse.
    fn row_to_run(row: &PgRow) -> Result<Run, RunRepositoryError> {
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
        let assistant_version: i32 = row.try_get("assistant_version").map_err(backend_err)?;

        let status_str: String = row.try_get("status").map_err(backend_err)?;
        let status: RunStatus = status_str.parse().map_err(
            |e: paladin_core::platform::container::run::RunStatusParseError| {
                RunRepositoryError::Serialization {
                    message: e.to_string(),
                }
            },
        )?;

        let input: serde_json::Value = row.try_get("input").map_err(backend_err)?;

        let submitted_at: DateTime<Utc> = row.try_get("submitted_at").map_err(backend_err)?;
        let started_at: Option<DateTime<Utc>> = row.try_get("started_at").map_err(backend_err)?;
        let finished_at: Option<DateTime<Utc>> = row.try_get("finished_at").map_err(backend_err)?;

        let attempt: i32 = row.try_get("attempt").map_err(backend_err)?;
        let cancel_requested: bool = row.try_get("cancel_requested").map_err(backend_err)?;
        let error: Option<String> = row.try_get("error").map_err(backend_err)?;

        let webhook_value: Option<serde_json::Value> =
            row.try_get("webhook").map_err(backend_err)?;
        let webhook: Option<WebhookSpec> = webhook_value
            .map(serde_json::from_value)
            .transpose()
            .map_err(ser_err)?;

        let pending_responses_value: serde_json::Value =
            row.try_get("pending_responses").map_err(backend_err)?;
        let pending_responses: Vec<ParleyResponse> =
            serde_json::from_value(pending_responses_value).map_err(ser_err)?;

        let fork_from_value: Option<serde_json::Value> =
            row.try_get("fork_from").map_err(backend_err)?;
        let fork_from: Option<ForkSpec> = fork_from_value
            .map(serde_json::from_value)
            .transpose()
            .map_err(ser_err)?;

        let output: Option<serde_json::Value> = row.try_get("output").map_err(backend_err)?;

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
        run.cancel_requested = cancel_requested;
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
impl RunRepositoryPort for PostgresRunRepository {
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
            .bind(run.assistant.version as i32)
            .bind(run.status.as_str())
            .bind(input)
            .bind(storage_timestamp(run.submitted_at))
            .bind(run.started_at.map(storage_timestamp))
            .bind(run.finished_at.map(storage_timestamp))
            .bind(run.attempt as i32)
            .bind(run.cancel_requested)
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
        // Legality is checked against the pure state machine FIRST -- see
        // sqlite.rs's identical comment for why this must happen before the
        // CAS predicate is evaluated.
        RunStatus::try_transition(from, to)
            .map_err(|_| RunRepositoryError::IllegalTransition { from, to })?;

        // Normalised once (D-01): both `started_at` and `finished_at` bind
        // this same normalised instant when both branches below fire on a
        // single terminal-and-just-started transition.
        let at = storage_timestamp(at);

        let mut builder: sqlx::QueryBuilder<sqlx::Postgres> =
            sqlx::QueryBuilder::new("UPDATE runs SET status = ");
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
            "UPDATE runs SET error = $1, output = $2::jsonb, final_waypoint_id = $3 \
             WHERE run_id = $4",
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

        let mut builder: sqlx::QueryBuilder<sqlx::Postgres> =
            sqlx::QueryBuilder::new(LIST_SELECT_PREFIX);
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
                let cancel_requested: bool = row
                    .try_get("cancel_requested")
                    .map_err(|e| RunRepositoryError::Backend { source: e.into() })?;
                Ok(cancel_requested)
            }
            None => Ok(false),
        }
    }

    async fn bump_attempt(&self, run_id: &RunId) -> Result<u32, RunRepositoryError> {
        let result = sqlx::query("UPDATE runs SET attempt = attempt + 1 WHERE run_id = $1")
            .bind(run_id.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;

        if result.rows_affected() == 0 {
            return Err(RunRepositoryError::NotFound {
                run_id: run_id.clone(),
            });
        }

        let row = sqlx::query("SELECT attempt FROM runs WHERE run_id = $1")
            .bind(run_id.as_str())
            .fetch_one(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;
        let attempt: i32 = row
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
            "UPDATE runs SET pending_responses = $1::jsonb, attempt = attempt + 1 \
             WHERE run_id = $2 AND status = 'awaiting_input'",
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

        let row = sqlx::query("SELECT attempt FROM runs WHERE run_id = $1")
            .bind(run_id.as_str())
            .fetch_one(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;
        let attempt: i32 = row
            .try_get("attempt")
            .map_err(|e| RunRepositoryError::Backend { source: e.into() })?;
        Ok(attempt as u32)
    }

    async fn clear_pending_responses(&self, run_id: &RunId) -> Result<(), RunRepositoryError> {
        let result =
            sqlx::query("UPDATE runs SET pending_responses = '[]'::jsonb WHERE run_id = $1")
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
            .bind(storage_timestamp(run.submitted_at))
            .bind(run.started_at.map(storage_timestamp))
            .bind(run.finished_at.map(storage_timestamp))
            .bind(run.attempt as i32)
            .bind(run.cancel_requested)
            .bind(&run.error)
            .bind(webhook)
            .bind(pending_responses)
            .bind(fork_from)
            .bind(output)
            .bind(&run.final_waypoint_id)
            .bind(&run.schema_version)
            .bind(&run.assistant.assistant_id)
            // (INSERT_RUN_WITH_LATEST's trailing $18 predicate bind, unchanged)
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
        let version: i32 = row
            .try_get("assistant_version")
            .map_err(|e| RunRepositoryError::Backend { source: e.into() })?;
        Ok(version as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::STORAGE_TIMESTAMP_SUBSEC_DIGITS;
    use crate::run::contract_tests;
    use chrono::SubsecRound;
    use std::sync::Arc;

    // Docker-gated Tier 2 suite (D-51): every test independently probes the
    // shared Postgres service and prints a named `SKIP:` reason then
    // returns early -- never panics or hangs -- when it is not reachable,
    // mirroring `waypoint::postgres`'s `store_or_skip` gate exactly. This
    // whole module is ALSO compile-time gated behind the `postgres` feature
    // (see `run/mod.rs`), which is not in any default feature set.
    //
    // `STORAGE_POSTGRES_TEST_URL` is a NEW, storage-wide variable name (not
    // `WAYPOINT_POSTGRES_TEST_URL`): every new Postgres suite landing in
    // this phase reads this one name, so a single CI export covers all of
    // them (plan 27-02 task 3's own scope note).
    //
    // Bring the service up before running this suite:
    // ```sh
    // docker compose -f docker/docker-compose.test.yml up -d postgres-test
    // STORAGE_POSTGRES_TEST_URL=postgres://... \
    //   cargo test -p paladin-storage --features postgres --lib run::postgres
    // ```
    //
    // Every test in this module shares the one Postgres database. Each
    // contract function operates under its own uniquely named `ThreadId`
    // (and, where relevant, `assistant_id`), so tests do not interfere with
    // each other EXCEPT `list_filters_by_thread_assistant_and_status` and
    // `list_paginates_by_submitted_at_and_run_id_with_no_overlap_or_gap`,
    // which scope themselves via a unique `assistant_id` filter rather than
    // requiring a literally empty table -- unlike the Waypoint suite's
    // `list_threads` clause, no Run contract clause here asserts on a
    // completely empty store, so `store_or_skip` does not need to truncate.

    fn postgres_test_url() -> String {
        std::env::var("STORAGE_POSTGRES_TEST_URL").unwrap_or_else(|_| {
            "postgres://paladin:paladin@localhost:5433/paladin_run_test".to_string()
        })
    }

    /// A cheap, short-timeout TCP reachability probe, tried BEFORE handing
    /// `url` to `sqlx`'s pool -- mirrors `waypoint::postgres`'s identical
    /// helper and its rationale (a connection refusal is retryable and can
    /// otherwise absorb the whole `acquire_timeout` budget).
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
    async fn store_or_skip() -> Option<PostgresRunRepository> {
        let url = postgres_test_url();
        if !postgres_reachable(&url) {
            println!(
                "SKIP: postgres-test not reachable at {url} -- bring it up with \
                 `docker compose -f docker/docker-compose.test.yml up -d postgres-test`"
            );
            return None;
        }

        match PostgresRunRepository::new(&url).await {
            Ok(store) => Some(store),
            Err(e) => {
                println!("SKIP: postgres-test connection failed at {url} ({e})");
                None
            }
        }
    }

    // One #[tokio::test] per shared contract function (D-09 precedent),
    // written out explicitly (not via a macro) so each names the violated
    // contract clause on failure, mirroring `sqlite.rs`'s test module and
    // `waypoint::postgres`'s skip-gate exactly.

    #[tokio::test]
    async fn insert_then_get_round_trips_every_field() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::insert_then_get_round_trips_every_field(&store).await;
    }

    #[tokio::test]
    async fn update_status_queued_to_running_sets_started_at_then_stale_cas_fails() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::update_status_queued_to_running_sets_started_at_then_stale_cas_fails(
            &store,
        )
        .await;
    }

    #[tokio::test]
    async fn update_status_running_to_completed_sets_finished_at_then_terminal_is_absorbing() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::update_status_running_to_completed_sets_finished_at_then_terminal_is_absorbing(&store).await;
    }

    #[tokio::test]
    async fn update_status_self_transition_fails() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::update_status_self_transition_fails(&store).await;
    }

    #[tokio::test]
    async fn insert_rejects_second_active_run_then_succeeds_after_terminal() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::insert_rejects_second_active_run_then_succeeds_after_terminal(&store).await;
    }

    #[tokio::test]
    async fn list_paginates_by_submitted_at_and_run_id_with_no_overlap_or_gap() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::list_paginates_by_submitted_at_and_run_id_with_no_overlap_or_gap(&store)
            .await;
    }

    #[tokio::test]
    async fn list_filters_by_thread_assistant_and_status() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::list_filters_by_thread_assistant_and_status(&store).await;
    }

    #[tokio::test]
    async fn request_cancel_is_idempotent_and_rejects_terminal() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::request_cancel_is_idempotent_and_rejects_terminal(&store).await;
    }

    #[tokio::test]
    async fn is_cancel_requested_reflects_active_run_flag() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::is_cancel_requested_reflects_active_run_flag(&store).await;
    }

    #[tokio::test]
    async fn bump_attempt_increments_and_persists() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::bump_attempt_increments_and_persists(&store).await;
    }

    #[tokio::test]
    async fn record_resume_on_awaiting_input_then_clear_pending_responses() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::record_resume_on_awaiting_input_then_clear_pending_responses(&store).await;
    }

    #[tokio::test]
    async fn record_outcome_persists_fields_without_touching_status() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::record_outcome_persists_fields_without_touching_status(&store).await;
    }

    #[tokio::test]
    async fn get_on_unsupported_schema_version_fails() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::get_on_unsupported_schema_version_fails(&store).await;
    }

    // Plain `#[tokio::test]` (current-thread flavor), not `multi_thread`:
    // this module's declared-test count is asserted equal to
    // `contract_tests`'s `pub async fn` count by regex on the literal
    // `#[tokio::test]` attribute (Task 3's own acceptance criterion), and
    // `tokio::spawn`'s cooperative interleaving across await points still
    // drives ten genuinely concurrent `insert` calls against the real
    // server -- the assertion is a database-level invariant
    // (`idx_runs_thread_active`), not one that depends on OS-thread
    // parallelism, unlike `sqlite.rs`'s on-disk stress test (which proves
    // multi-CONNECTION, not just multi-task, safety).
    #[tokio::test]
    async fn ten_concurrent_inserts_one_thread_exactly_one_accepted() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        let store: Arc<dyn RunRepositoryPort> = Arc::new(store);
        contract_tests::ten_concurrent_inserts_one_thread_exactly_one_accepted(store).await;
    }

    // ── insert_with_latest / freeze-at-submit (D-30), plan 27-09 ─────────
    // These four clauses need a `PostgresRunRepository` AND a
    // `PostgresAssistantRepository` reading/writing the SAME database --
    // both connect through `postgres_test_url()`, and `sqlx::migrate!`
    // embeds every file under `migrations/postgres/` (so either
    // constructor's own migration run already created both `runs` and
    // `assistants` on this shared database).

    /// Returns a connected, migrated assistant store, or `None` (after
    /// printing a named reason) if `postgres-test` is not reachable --
    /// mirrors `store_or_skip` for the sibling `AssistantRepositoryPort`.
    async fn assistant_store_or_skip()
    -> Option<crate::assistant::postgres::PostgresAssistantRepository> {
        let url = postgres_test_url();
        if !postgres_reachable(&url) {
            println!(
                "SKIP: postgres-test not reachable at {url} -- bring it up with \
                 `docker compose -f docker/docker-compose.test.yml up -d postgres-test`"
            );
            return None;
        }

        match crate::assistant::postgres::PostgresAssistantRepository::new(&url).await {
            Ok(store) => Some(store),
            Err(e) => {
                println!("SKIP: postgres-test connection failed at {url} ({e})");
                None
            }
        }
    }

    #[tokio::test]
    async fn insert_with_latest_resolves_current_latest_and_freezes_it() {
        let (Some(run_store), Some(assistant_store)) =
            (store_or_skip().await, assistant_store_or_skip().await)
        else {
            return;
        };
        contract_tests::insert_with_latest_resolves_current_latest_and_freezes_it(
            &run_store,
            &assistant_store,
        )
        .await;
    }

    #[tokio::test]
    async fn insert_with_latest_unknown_assistant_fails() {
        let Some(run_store) = store_or_skip().await else {
            return;
        };
        contract_tests::insert_with_latest_unknown_assistant_fails(&run_store).await;
    }

    #[tokio::test]
    async fn insert_with_latest_soft_deleted_assistant_fails() {
        let (Some(run_store), Some(assistant_store)) =
            (store_or_skip().await, assistant_store_or_skip().await)
        else {
            return;
        };
        contract_tests::insert_with_latest_soft_deleted_assistant_fails(
            &run_store,
            &assistant_store,
        )
        .await;
    }

    #[tokio::test]
    async fn assistant_version_freeze_at_submit() {
        let (Some(run_store), Some(assistant_store)) =
            (store_or_skip().await, assistant_store_or_skip().await)
        else {
            return;
        };
        let run_store: Arc<dyn RunRepositoryPort> = Arc::new(run_store);
        let assistant_store: Arc<
            dyn paladin_ports::output::assistant_repository_port::AssistantRepositoryPort,
        > = Arc::new(assistant_store);
        contract_tests::assistant_version_freeze_at_submit(run_store, assistant_store).await;
    }

    // ── Gap-closure plan 27-20: timestamp precision contract ────────────
    //
    // Deliberate, single exception to the "no extra, non-contract
    // `#[tokio::test]`s in this module" rule below: this clause pins the
    // `storage_timestamp` truncation contract specifically against a real
    // Postgres `TIMESTAMPTZ` column, which no shared `contract_tests`
    // function can do (a fixture built through `contract_timestamp` is
    // ALREADY normalised to microsecond resolution before it reaches any
    // backend, by design -- that is what keeps every backend's
    // `assert_eq!` exact. Proving *this specific* backend performs the
    // truncation itself needs a fixture that deliberately carries
    // sub-microsecond digits, built independently of `sample_run`'s
    // `contract_timestamp` convention).
    //
    // This clause proves NOTHING in a devcontainer with no reachable
    // Postgres -- `store_or_skip` makes it self-skip and print `SKIP:`,
    // exactly like every other clause in this module (D-51). The
    // `postgres-integration` CI job is where it actually speaks.
    #[tokio::test]
    async fn postgres_run_timestamps_round_trip_at_microsecond_precision() {
        let Some(store) = store_or_skip().await else {
            return;
        };

        // A microsecond boundary plus 999ns: deliberately NOT run through
        // `contract_timestamp` (which would normalise it before this test
        // ever saw it), so `insert` then `get` is the thing doing the
        // truncation, not the fixture.
        let boundary = chrono::Utc::now().trunc_subsecs(STORAGE_TIMESTAMP_SUBSEC_DIGITS);
        let sub_microsecond_submitted_at = boundary + chrono::Duration::nanoseconds(999);
        assert_ne!(
            sub_microsecond_submitted_at,
            storage_timestamp(sub_microsecond_submitted_at),
            "fixture must carry sub-microsecond digits for this test to prove anything"
        );

        let thread = ThreadId::new("contract-run-postgres-timestamp-precision-submitted").unwrap();
        let run = contract_tests::sample_run(&thread, "assistant-a", sub_microsecond_submitted_at);
        store.insert(&run).await.unwrap();
        let loaded = store.get(&run.run_id).await.unwrap().unwrap();

        assert_eq!(
            loaded.submitted_at,
            storage_timestamp(sub_microsecond_submitted_at),
            "submitted_at must round-trip truncated to microsecond resolution"
        );
        assert_ne!(
            loaded.submitted_at, sub_microsecond_submitted_at,
            "submitted_at must NOT silently widen to persist the full nanosecond value"
        );

        // Same proof for `started_at`, written through `update_status`
        // rather than `insert`.
        let sub_microsecond_started_at = boundary + chrono::Duration::nanoseconds(999);
        store
            .update_status(
                &run.run_id,
                RunStatus::Queued,
                RunStatus::Running,
                sub_microsecond_started_at,
            )
            .await
            .unwrap();
        let loaded_after_start = store.get(&run.run_id).await.unwrap().unwrap();

        assert_eq!(
            loaded_after_start.started_at,
            Some(storage_timestamp(sub_microsecond_started_at)),
            "started_at must round-trip truncated to microsecond resolution"
        );
        assert_ne!(
            loaded_after_start.started_at,
            Some(sub_microsecond_started_at),
            "started_at must NOT silently widen to persist the full nanosecond value"
        );
    }

    // No extra, non-contract `#[tokio::test]`s in this module BEYOND the
    // one immediately above (by design, and its own doc comment explains
    // why it is the sanctioned exception): CI asserts this module's
    // `#[tokio::test]` count equals `contract_tests`'s `pub async fn` count
    // plus exactly one (Task 3's own acceptance criterion, amended by plan
    // 27-20), so a password-redaction smoke test analogous to `sqlite.rs`'s
    // and `waypoint::postgres`'s would break that assertion.
    // `map_insert_error`/`wrap`/`wrap_error` reuse the same
    // `redact_database_url_password` helper the Waypoint adapters already
    // prove redacts correctly (`waypoint::postgres::tests::connection_error_redacts_password_from_database_url`).
}
