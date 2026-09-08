/*
PostgreSQL Run Trace Store

Concrete `RunTracePort` implementation over PostgreSQL, behind the
`postgres` feature (OBS-02, D-17). Mirrors `sqlite.rs` exactly -- same
`(thread_id, seq > ?)` range scan, same total ORDER BY, same
redact-before-truncate error handling -- substituting `$1, $2, ...`
placeholders for `?` and JSONB for the TEXT `record` column. Every statement
uses bound parameters; no query string is ever built by formatting a
caller-supplied value into it.
*/

use async_trait::async_trait;
use sqlx::Row;
use sqlx::postgres::{PgPool, PgPoolOptions, PgRow};

use paladin_core::platform::container::trace::{TRACE_SCHEMA_VERSION, TraceRecord};
use paladin_core::platform::container::waypoint::ThreadId;
use paladin_ports::output::run_trace_port::{RunTraceError, RunTracePort};

use crate::run_trace::contract_tests::RawSchemaVersionWriter;
use crate::run_trace::superstep_of;
use crate::waypoint::redact::redact_database_url_password;

const APPEND_QUERY: &str = r#"
    INSERT INTO run_traces (thread_id, seq, run_id, superstep, at, schema_version, record)
    VALUES ($1, $2, $3, $4, $5, $6, $7::jsonb)
    ON CONFLICT (thread_id, seq) DO NOTHING
"#;

const READ_QUERY: &str = r#"
    SELECT schema_version, record
    FROM run_traces
    WHERE thread_id = $1 AND seq > $2
    ORDER BY seq ASC
    LIMIT $3
"#;

const PRUNE_QUERY: &str = "DELETE FROM run_traces WHERE thread_id = $1 AND superstep < $2";

/// Test-only raw upsert (see [`RawSchemaVersionWriter`]): writes `record`
/// with an ARBITRARY `schema_version`, bypassing [`APPEND_QUERY`]'s
/// always-current stamping.
const RAW_WRITE_QUERY: &str = r#"
    INSERT INTO run_traces (thread_id, seq, run_id, superstep, at, schema_version, record)
    VALUES ($1, $2, $3, $4, $5, $6, $7::jsonb)
    ON CONFLICT (thread_id, seq) DO UPDATE SET
        schema_version = excluded.schema_version,
        record         = excluded.record
"#;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("migrations/postgres");

/// PostgreSQL `RunTracePort` implementation, behind the `postgres` feature
/// (Tier 2: Docker-gated, see `docker/docker-compose.test.yml`'s
/// `postgres-test` service and `make test-integration-docker`).
#[derive(Debug)]
pub struct PostgresRunTraceStore {
    pool: PgPool,
    /// Kept so every error this store returns can be redacted of the
    /// connection URL's password, not just construction-time connection
    /// errors.
    database_url: String,
}

impl PostgresRunTraceStore {
    /// Connect to `database_url` and apply the versioned migration. Safe to
    /// call more than once against the same database: the migration is
    /// idempotent and `sqlx::migrate::Migrator` tracks applied versions.
    pub async fn new(database_url: &str) -> Result<Self, RunTraceError> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            // sqlx's own default (30s) means a genuinely unreachable server
            // surfaces as a slow hang rather than a fast, clearly-diagnosed
            // error -- mirrors `PostgresWaypointStore`'s own precedent.
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

    /// Wrap a driver/migration error into `RunTraceError::Backend`, with the
    /// connection URL's password redacted from the error text first
    /// (T-22-18).
    fn wrap(database_url: &str, err: impl std::error::Error) -> RunTraceError {
        let redacted = redact_database_url_password(&err.to_string(), database_url);
        RunTraceError::Backend {
            source: redacted.into(),
        }
    }

    fn wrap_error(&self, err: sqlx::Error) -> RunTraceError {
        Self::wrap(&self.database_url, err)
    }

    fn row_to_record(row: &PgRow) -> Result<TraceRecord, RunTraceError> {
        let schema_version: String = row
            .try_get("schema_version")
            .map_err(|e| RunTraceError::Backend { source: e.into() })?;
        if schema_version != TRACE_SCHEMA_VERSION {
            return Err(RunTraceError::UnsupportedSchemaVersion {
                found: schema_version,
            });
        }

        let record_value: serde_json::Value = row
            .try_get("record")
            .map_err(|e| RunTraceError::Backend { source: e.into() })?;
        serde_json::from_value(record_value).map_err(|e| RunTraceError::Serialization { source: e })
    }
}

#[async_trait]
impl RunTracePort for PostgresRunTraceStore {
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
impl RawSchemaVersionWriter for PostgresRunTraceStore {
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

    // Docker-gated Tier 2 suite: every test independently probes the
    // `postgres-test` service (see `docker/docker-compose.test.yml`) and
    // prints a named reason then returns early -- never panics or hangs --
    // when it is not reachable, mirroring `waypoint::postgres`'s identical
    // gate. This whole module is ALSO compile-time gated behind the
    // `postgres` feature (see `run_trace/mod.rs`), which is not in any
    // default feature set, so a plain `cargo test -p paladin-storage` never
    // attempts to build it, let alone run it.
    //
    // Bring the service up before running this suite:
    // ```sh
    // docker compose -f docker/docker-compose.test.yml up -d postgres-test
    // cargo test -p paladin-storage --features postgres --lib run_trace::postgres
    // ```

    fn postgres_test_url() -> String {
        std::env::var("STORAGE_POSTGRES_TEST_URL").unwrap_or_else(|_| {
            "postgres://paladin:paladin@localhost:5433/paladin_run_trace_test".to_string()
        })
    }

    /// A cheap, short-timeout TCP reachability probe, tried BEFORE handing
    /// `url` to `sqlx`'s pool -- mirrors `waypoint::postgres`'s identical
    /// helper and its rationale.
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
    async fn store_or_skip() -> Option<PostgresRunTraceStore> {
        let url = postgres_test_url();
        if !postgres_reachable(&url) {
            println!(
                "SKIP: postgres-test not reachable at {url} -- bring it up with \
                 `docker compose -f docker/docker-compose.test.yml up -d postgres-test`"
            );
            return None;
        }

        match PostgresRunTraceStore::new(&url).await {
            Ok(store) => {
                // Every contract function operates under its own uniquely
                // named `ThreadId`, so tests do not interfere with each
                // other -- but reset the table anyway, mirroring
                // `waypoint::postgres`'s own precedent, since this suite
                // shares the ONE `postgres-test` database with every other
                // Tier 2 suite.
                if let Err(e) = sqlx::query("TRUNCATE TABLE run_traces")
                    .execute(&store.pool)
                    .await
                {
                    println!("SKIP: could not reset run_traces table ({e})");
                    return None;
                }
                Some(store)
            }
            Err(e) => {
                println!("SKIP: postgres-test connection failed at {url} ({e})");
                None
            }
        }
    }

    #[tokio::test]
    async fn append_then_read_round_trips() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::append_then_read_round_trips(&store).await;
    }

    #[tokio::test]
    async fn read_paginates_by_after_seq() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::read_paginates_by_after_seq(&store).await;
    }

    #[tokio::test]
    async fn read_of_unknown_thread_is_empty_not_error() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::read_of_unknown_thread_is_empty_not_error(&store).await;
    }

    #[tokio::test]
    async fn append_is_idempotent_on_same_seq() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::append_is_idempotent_on_same_seq(&store).await;
    }

    #[tokio::test]
    async fn records_are_scoped_by_thread() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::records_are_scoped_by_thread(&store).await;
    }

    #[tokio::test]
    async fn prune_thread_removes_only_older_supersteps() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::prune_thread_removes_only_older_supersteps(&store).await;
    }

    #[tokio::test]
    async fn unsupported_schema_version_is_typed() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::unsupported_schema_version_is_typed(&store).await;
    }

    #[tokio::test]
    async fn run_all_contract_functions_smoke_aggregate() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::run_all(Arc::new(store)).await;
    }

    #[tokio::test]
    async fn record_written_as_jsonb_reads_back_as_equal_record() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        let thread = ThreadId::new("contract-postgres-jsonb-roundtrip").unwrap();
        let record = contract_tests::sample_record(&thread, 1, 0);
        store.append(std::slice::from_ref(&record)).await.unwrap();

        let read_back = store.read(&thread, 0, 100).await.unwrap();
        assert_eq!(read_back, vec![record]);
    }

    #[tokio::test]
    async fn connection_error_redacts_password_from_database_url() {
        let url = "postgres://user:hunter2-secret@127.0.0.1:1/nonexistent";
        let err = PostgresRunTraceStore::new(url).await.unwrap_err();
        let message = err.to_string();
        assert!(
            !message.contains("hunter2-secret"),
            "connection error leaked the password: {message}"
        );
    }
}
