/*
SQLite Run Trace Store

Concrete `RunTracePort` implementation over SQLite (OBS-02, D-17). `record`
stores the FULL serialized JSON `TraceRecord` envelope; `thread_id`, `seq`,
`run_id`, `superstep`, `at` and `schema_version` are broken out as their own
columns purely to serve `read`'s `(thread_id, seq > ?)` range scan and
`prune_thread`'s `(thread_id, superstep < ?)` deletion, without parsing
`record` for either. Every statement uses bound parameters (mirroring
`waypoint::sqlite`'s T-22-17 convention); no query string is ever built by
formatting a caller-supplied value into it. Migrations follow the
versioned-file convention at `crates/paladin-storage/migrations/sqlite/`,
embedded at compile time via `sqlx::migrate!` and applied automatically on
construction.
*/

use async_trait::async_trait;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::{Row, sqlite::SqliteRow};
use std::str::FromStr;

use paladin_core::platform::container::trace::{TRACE_SCHEMA_VERSION, TraceRecord};
use paladin_core::platform::container::waypoint::ThreadId;
use paladin_ports::output::run_trace_port::{RunTraceError, RunTracePort};

use crate::run_trace::contract_tests::RawSchemaVersionWriter;
use crate::run_trace::superstep_of;
use crate::waypoint::redact::redact_database_url_password;

/// `append`: one row per record, idempotent on `(thread_id, seq)` via
/// `ON CONFLICT ... DO NOTHING` (never a duplicate row, never an error on
/// retry).
const APPEND_QUERY: &str = r#"
    INSERT INTO run_traces (thread_id, seq, run_id, superstep, at, schema_version, record)
    VALUES (?, ?, ?, ?, ?, ?, ?)
    ON CONFLICT(thread_id, seq) DO NOTHING
"#;

/// `read`: ascending `seq`, strictly greater than the caller's cursor,
/// bounded by a bound `LIMIT` parameter.
const READ_QUERY: &str = r#"
    SELECT schema_version, record
    FROM run_traces
    WHERE thread_id = ? AND seq > ?
    ORDER BY seq ASC
    LIMIT ?
"#;

/// `prune_thread`: every row of `thread` whose `superstep` is strictly less
/// than the caller's boundary.
const PRUNE_QUERY: &str = "DELETE FROM run_traces WHERE thread_id = ? AND superstep < ?";

/// Test-only raw upsert (see [`RawSchemaVersionWriter`]): writes `record`
/// with an ARBITRARY `schema_version`, bypassing [`APPEND_QUERY`]'s
/// always-current stamping.
const RAW_WRITE_QUERY: &str = r#"
    INSERT INTO run_traces (thread_id, seq, run_id, superstep, at, schema_version, record)
    VALUES (?, ?, ?, ?, ?, ?, ?)
    ON CONFLICT(thread_id, seq) DO UPDATE SET
        schema_version = excluded.schema_version,
        record         = excluded.record
"#;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("migrations/sqlite");

/// SQLite `RunTracePort` implementation (Tier 1: always exercised in CI, no
/// external service required).
#[derive(Debug)]
pub struct SqliteRunTraceStore {
    pool: SqlitePool,
    /// Kept so every error this store returns can be redacted of the
    /// connection URL's password, not just construction-time connection
    /// errors -- mirrors `SqliteWaypointStore`'s own field (T-22-18).
    database_url: String,
}

impl SqliteRunTraceStore {
    /// Connect to `database_url`, creating the database file if missing, and
    /// apply the versioned migration. Safe to call more than once against
    /// the same database file: the migration is idempotent
    /// (`CREATE TABLE IF NOT EXISTS`/`CREATE INDEX IF NOT EXISTS`) and
    /// `sqlx::migrate::Migrator` itself tracks applied versions.
    pub async fn new(database_url: &str) -> Result<Self, RunTraceError> {
        let options = SqliteConnectOptions::from_str(database_url)
            .map_err(|e| Self::wrap(database_url, e))?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            // Mirrors `SqliteWaypointStore`'s own precedent: a single
            // connection avoids the classic sqlx/SQLite pitfall where
            // `sqlite::memory:` hands out a DIFFERENT, independently empty
            // database per pooled connection.
            .max_connections(1)
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

    /// Wrap a driver/migration error into `RunTraceError::Backend`, with the
    /// connection URL's password redacted from the error text first
    /// (T-22-18) -- redact before any truncation, per this project's
    /// security instructions.
    fn wrap(database_url: &str, err: impl std::error::Error) -> RunTraceError {
        let redacted = redact_database_url_password(&err.to_string(), database_url);
        RunTraceError::Backend {
            source: redacted.into(),
        }
    }

    fn wrap_error(&self, err: sqlx::Error) -> RunTraceError {
        Self::wrap(&self.database_url, err)
    }

    fn row_to_record(row: &SqliteRow) -> Result<TraceRecord, RunTraceError> {
        let schema_version: String = row
            .try_get("schema_version")
            .map_err(|e| RunTraceError::Backend { source: e.into() })?;
        if schema_version != TRACE_SCHEMA_VERSION {
            return Err(RunTraceError::UnsupportedSchemaVersion {
                found: schema_version,
            });
        }

        let record_json: String = row
            .try_get("record")
            .map_err(|e| RunTraceError::Backend { source: e.into() })?;
        serde_json::from_str(&record_json).map_err(|e| RunTraceError::Serialization { source: e })
    }
}

#[async_trait]
impl RunTracePort for SqliteRunTraceStore {
    async fn append(&self, records: &[TraceRecord]) -> Result<(), RunTraceError> {
        if records.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await.map_err(|e| self.wrap_error(e))?;
        for record in records {
            let record_json = serde_json::to_string(record)
                .map_err(|e| RunTraceError::Serialization { source: e })?;

            sqlx::query(APPEND_QUERY)
                .bind(record.thread_id.as_str())
                .bind(record.seq as i64)
                .bind(record.run_id.as_ref().map(|r| r.as_str()))
                .bind(superstep_of(&record.event) as i64)
                .bind(record.at)
                .bind(TRACE_SCHEMA_VERSION)
                .bind(record_json)
                .execute(&mut *tx)
                .await
                .map_err(|e| self.wrap_error(e))?;
        }
        tx.commit().await.map_err(|e| self.wrap_error(e))?;
        Ok(())
    }

    async fn read(
        &self,
        thread: &ThreadId,
        after_seq: u64,
        limit: u32,
    ) -> Result<Vec<TraceRecord>, RunTraceError> {
        let rows = sqlx::query(READ_QUERY)
            .bind(thread.as_str())
            .bind(after_seq as i64)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;

        rows.iter().map(Self::row_to_record).collect()
    }

    async fn prune_thread(
        &self,
        thread: &ThreadId,
        before_superstep: u64,
    ) -> Result<u64, RunTraceError> {
        let result = sqlx::query(PRUNE_QUERY)
            .bind(thread.as_str())
            .bind(before_superstep as i64)
            .execute(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;
        Ok(result.rows_affected())
    }
}

#[async_trait]
impl RawSchemaVersionWriter for SqliteRunTraceStore {
    async fn write_with_schema_version(&self, record: &TraceRecord, schema_version: &str) {
        let record_json = serde_json::to_string(record).expect("TraceRecord always serializes");
        sqlx::query(RAW_WRITE_QUERY)
            .bind(record.thread_id.as_str())
            .bind(record.seq as i64)
            .bind(record.run_id.as_ref().map(|r| r.as_str()))
            .bind(superstep_of(&record.event) as i64)
            .bind(record.at)
            .bind(schema_version)
            .bind(record_json)
            .execute(&self.pool)
            .await
            .expect("raw schema-version write must succeed in tests");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_trace::contract_tests;
    use std::sync::Arc;

    // One #[tokio::test] per shared contract function, each against a fresh
    // in-memory database, so a failure names the violated contract clause.
    // See `contract_tests` for the assertions themselves -- this file only
    // wires SqliteRunTraceStore into them, unchanged.

    async fn fresh_store() -> SqliteRunTraceStore {
        SqliteRunTraceStore::new("sqlite::memory:").await.unwrap()
    }

    #[tokio::test]
    async fn append_then_read_round_trips() {
        contract_tests::append_then_read_round_trips(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn read_paginates_by_after_seq() {
        contract_tests::read_paginates_by_after_seq(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn read_of_unknown_thread_is_empty_not_error() {
        contract_tests::read_of_unknown_thread_is_empty_not_error(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn append_is_idempotent_on_same_seq() {
        contract_tests::append_is_idempotent_on_same_seq(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn records_are_scoped_by_thread() {
        contract_tests::records_are_scoped_by_thread(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn prune_thread_removes_only_older_supersteps() {
        contract_tests::prune_thread_removes_only_older_supersteps(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn unsupported_schema_version_is_typed() {
        contract_tests::unsupported_schema_version_is_typed(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn run_all_contract_functions_smoke_aggregate() {
        contract_tests::run_all(Arc::new(fresh_store().await)).await;
    }

    // ── Backend-specific tests (mirroring waypoint::sqlite's T-22-17/18) ──

    #[tokio::test]
    async fn thread_id_with_sql_metacharacter_round_trips_as_data() {
        let store = fresh_store().await;
        let thread = ThreadId::new("thread-o'brien;DROP-TABLE--comment").unwrap();
        let record = contract_tests::sample_record(&thread, 1, 0);
        store.append(&[record]).await.unwrap();

        let read_back = store.read(&thread, 0, 100).await.unwrap();
        assert_eq!(read_back.len(), 1);
        assert_eq!(read_back[0].thread_id, thread);
    }

    #[tokio::test]
    async fn connection_error_redacts_password_from_database_url() {
        // A syntactically URL-like DSN carrying a password, using a scheme
        // SqliteConnectOptions cannot parse -- guaranteed to fail before any
        // real connection attempt, exactly like a real bad-credential
        // failure would.
        let url = "sqlite://user:hunter2-secret@/nonexistent/path/that/does/not/exist.db";
        let err = SqliteRunTraceStore::new(url).await.unwrap_err();
        let message = err.to_string();
        assert!(
            !message.contains("hunter2-secret"),
            "connection error leaked the password: {message}"
        );
    }
}
