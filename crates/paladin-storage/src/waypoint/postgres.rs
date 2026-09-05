/*
PostgreSQL Waypoint Store

Concrete `WaypointPort` implementation over PostgreSQL, behind the
`postgres` feature (ENG-05, D-01). Mirrors `sqlite.rs` exactly -- same six
methods, same LIMIT/cursor pagination, same total ORDER BY, same
redact-before-truncate error handling -- substituting `$1, $2, ...`
placeholders for `?` and JSONB for the TEXT payload column. Every statement
uses bound parameters (T-22-17); no query string is ever built by formatting
a caller-supplied value into it.
*/

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::postgres::{PgPool, PgPoolOptions, PgRow};

use paladin_core::platform::container::waypoint::{ThreadId, Waypoint, WaypointId, WaypointStatus};
use paladin_ports::output::waypoint_port::{
    ThreadSummary, WaypointError, WaypointPort, WaypointSummary,
};

use crate::waypoint::redact::redact_database_url_password;

/// `history()` with no `before` cursor. See `sqlite.rs`'s sibling constant
/// for the ordering rationale -- identical here, `?` placeholders replaced
/// with `$N`.
/// `fork_of` (HITL-03, D-14) is not a dedicated column -- it rides in the
/// existing `payload` JSONB column exactly like `visit_counts`/`frontier`/
/// `muster_progress`/`checkpoint_ns` do on the full `Waypoint`, so no SQL
/// migration is needed. `payload->>'fork_of'` reads just that one field out
/// of `payload` without deserializing the whole (potentially large)
/// payload, keeping `history`'s summary query cheap.
const HISTORY_QUERY_NO_CURSOR: &str = r#"
    SELECT waypoint_id, parent_id, superstep, status, created_at,
           payload->>'fork_of' AS fork_of
    FROM waypoints
    WHERE thread_id = $1
    ORDER BY created_at DESC, superstep DESC
    LIMIT $2
"#;

/// `history()` with a `before` cursor, resolved to its own
/// `(created_at, superstep)` first (see `history`'s body).
const HISTORY_QUERY_WITH_CURSOR: &str = r#"
    SELECT waypoint_id, parent_id, superstep, status, created_at,
           payload->>'fork_of' AS fork_of
    FROM waypoints
    WHERE thread_id = $1
      AND (created_at < $2 OR (created_at = $3 AND superstep < $4))
    ORDER BY created_at DESC, superstep DESC
    LIMIT $5
"#;

/// `list_threads()` with no `before` cursor.
const LIST_THREADS_QUERY_NO_CURSOR: &str = r#"
    SELECT thread_id, status, created_at FROM (
        SELECT thread_id, status, created_at,
               ROW_NUMBER() OVER (
                   PARTITION BY thread_id
                   ORDER BY created_at DESC, superstep DESC
               ) AS rn
        FROM waypoints
    ) ranked
    WHERE rn = 1
    ORDER BY created_at DESC
    LIMIT $1
"#;

/// `list_threads()` with a `before: DateTime<Utc>` cursor.
const LIST_THREADS_QUERY_WITH_CURSOR: &str = r#"
    SELECT thread_id, status, created_at FROM (
        SELECT thread_id, status, created_at,
               ROW_NUMBER() OVER (
                   PARTITION BY thread_id
                   ORDER BY created_at DESC, superstep DESC
               ) AS rn
        FROM waypoints
    ) ranked
    WHERE rn = 1 AND created_at < $1
    ORDER BY created_at DESC
    LIMIT $2
"#;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("migrations/postgres");

/// PostgreSQL `WaypointPort` implementation, behind the `postgres` feature
/// (ENG-05, D-01, Tier 2: Docker-gated, see `docker/docker-compose.test.yml`'s
/// `postgres-test` service and `make test-integration-docker`).
#[derive(Debug)]
pub struct PostgresWaypointStore {
    pool: PgPool,
    /// Kept so every error this store returns can be redacted of the
    /// connection URL's password (T-22-18), not just construction-time
    /// connection errors.
    database_url: String,
}

impl PostgresWaypointStore {
    /// Connect to `database_url` and apply the versioned migration. Safe to
    /// call more than once against the same database: the migration is
    /// idempotent and `sqlx::migrate::Migrator` tracks applied versions.
    pub async fn new(database_url: &str) -> Result<Self, WaypointError> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            // sqlx's own default (30s) means a genuinely unreachable server
            // -- the exact case this Tier 2 suite's tests exercise when
            // `postgres-test` is not running -- surfaces as a slow hang
            // rather than a fast, clearly-diagnosed error. 5s is generous
            // for a real, healthy server on the same Docker network.
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

    /// Wrap a driver/migration error into `WaypointError::Backend`, with the
    /// connection URL's password redacted from the error text first
    /// (T-22-18).
    fn wrap(database_url: &str, err: impl std::error::Error) -> WaypointError {
        let redacted = redact_database_url_password(&err.to_string(), database_url);
        WaypointError::Backend {
            source: redacted.into(),
        }
    }

    fn wrap_error(&self, err: sqlx::Error) -> WaypointError {
        Self::wrap(&self.database_url, err)
    }

    /// Deserialize a full `Waypoint` from its stored `payload` JSONB value,
    /// enforcing the schema-version check (T-22-19).
    fn parse_payload(payload: serde_json::Value) -> Result<Waypoint, WaypointError> {
        let wp: Waypoint = serde_json::from_value(payload)
            .map_err(|e| WaypointError::Serialization(e.to_string()))?;
        let supported = Waypoint::current_schema_version();
        if wp.schema_version != supported {
            return Err(WaypointError::SchemaVersionUnsupported {
                found: wp.schema_version.clone(),
                supported,
            });
        }
        Ok(wp)
    }

    /// Reconstruct a `WaypointId` from its stored TEXT form (see `sqlite.rs`'s
    /// identical helper for the rationale: `WaypointId` is `#[serde(transparent)]`
    /// over a `Uuid`, so a JSON-string round trip works without a dedicated
    /// paladin-core-side constructor).
    fn parse_waypoint_id(raw: &str) -> Result<WaypointId, WaypointError> {
        serde_json::from_value(serde_json::Value::String(raw.to_string()))
            .map_err(|e| WaypointError::Serialization(format!("invalid waypoint_id: {e}")))
    }

    fn row_to_summary(row: &PgRow) -> Result<WaypointSummary, WaypointError> {
        let waypoint_id_str: String = row
            .try_get("waypoint_id")
            .map_err(|e| WaypointError::Backend { source: e.into() })?;
        let waypoint_id = Self::parse_waypoint_id(&waypoint_id_str)?;

        let parent_id_str: Option<String> = row
            .try_get("parent_id")
            .map_err(|e| WaypointError::Backend { source: e.into() })?;
        let parent_waypoint_id = parent_id_str
            .map(|s| Self::parse_waypoint_id(&s))
            .transpose()?;

        let superstep: i64 = row
            .try_get("superstep")
            .map_err(|e| WaypointError::Backend { source: e.into() })?;

        let status_json: String = row
            .try_get("status")
            .map_err(|e| WaypointError::Backend { source: e.into() })?;
        let status: WaypointStatus = serde_json::from_str(&status_json)
            .map_err(|e| WaypointError::Serialization(e.to_string()))?;

        let created_at: DateTime<Utc> = row
            .try_get("created_at")
            .map_err(|e| WaypointError::Backend { source: e.into() })?;

        // --- HITL-03, D-14: `fork_of` is extracted from the `payload`
        // JSONB column via `->>'fork_of'`, never a dedicated column -- see
        // `HISTORY_QUERY_NO_CURSOR`'s doc comment. `NULL` covers both a
        // pre-D-14 payload (key absent) and a genuine JSON `null`
        // (mainline waypoint), exactly like `parent_id` above.
        let fork_of_str: Option<String> = row
            .try_get("fork_of")
            .map_err(|e| WaypointError::Backend { source: e.into() })?;
        let fork_of = fork_of_str
            .map(|s| Self::parse_waypoint_id(&s))
            .transpose()?;

        Ok(WaypointSummary {
            waypoint_id,
            parent_waypoint_id,
            superstep: superstep as u64,
            status,
            created_at,
            fork_of,
        })
    }

    fn row_to_thread_summary(row: &PgRow) -> Result<ThreadSummary, WaypointError> {
        let thread_id_str: String = row
            .try_get("thread_id")
            .map_err(|e| WaypointError::Backend { source: e.into() })?;
        let thread_id = ThreadId::new(thread_id_str.clone())
            .map_err(|e| WaypointError::Serialization(format!("invalid thread_id: {e}")))?;

        let status_json: String = row
            .try_get("status")
            .map_err(|e| WaypointError::Backend { source: e.into() })?;
        let latest_status: WaypointStatus = serde_json::from_str(&status_json)
            .map_err(|e| WaypointError::Serialization(e.to_string()))?;

        let last_updated_at: DateTime<Utc> = row
            .try_get("created_at")
            .map_err(|e| WaypointError::Backend { source: e.into() })?;

        Ok(ThreadSummary {
            thread_id,
            latest_status,
            last_updated_at,
        })
    }
}

#[async_trait]
impl WaypointPort for PostgresWaypointStore {
    async fn save(&self, wp: &Waypoint) -> Result<(), WaypointError> {
        let payload =
            serde_json::to_string(wp).map_err(|e| WaypointError::Serialization(e.to_string()))?;
        let status_json = serde_json::to_string(&wp.status)
            .map_err(|e| WaypointError::Serialization(e.to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO waypoints
                (waypoint_id, thread_id, parent_id, superstep, status, payload, created_at)
            VALUES ($1, $2, $3, $4, $5, $6::jsonb, $7)
            ON CONFLICT (waypoint_id) DO UPDATE SET
                thread_id  = excluded.thread_id,
                parent_id  = excluded.parent_id,
                superstep  = excluded.superstep,
                status     = excluded.status,
                payload    = excluded.payload,
                created_at = excluded.created_at
            "#,
        )
        .bind(wp.waypoint_id.to_string())
        .bind(wp.thread_id.as_str())
        .bind(wp.parent_waypoint_id.map(|p| p.to_string()))
        .bind(wp.superstep as i64)
        .bind(status_json)
        .bind(payload)
        .bind(wp.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| self.wrap_error(e))?;

        Ok(())
    }

    async fn latest(&self, thread: &ThreadId) -> Result<Option<Waypoint>, WaypointError> {
        let row = sqlx::query(
            "SELECT payload FROM waypoints WHERE thread_id = $1 \
             ORDER BY created_at DESC, superstep DESC LIMIT 1",
        )
        .bind(thread.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| self.wrap_error(e))?;

        match row {
            Some(row) => {
                let payload: serde_json::Value = row
                    .try_get("payload")
                    .map_err(|e| WaypointError::Backend { source: e.into() })?;
                Ok(Some(Self::parse_payload(payload)?))
            }
            None => Ok(None),
        }
    }

    async fn get(
        &self,
        thread: &ThreadId,
        id: &WaypointId,
    ) -> Result<Option<Waypoint>, WaypointError> {
        let row =
            sqlx::query("SELECT payload FROM waypoints WHERE thread_id = $1 AND waypoint_id = $2")
                .bind(thread.as_str())
                .bind(id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| self.wrap_error(e))?;

        match row {
            Some(row) => {
                let payload: serde_json::Value = row
                    .try_get("payload")
                    .map_err(|e| WaypointError::Backend { source: e.into() })?;
                Ok(Some(Self::parse_payload(payload)?))
            }
            None => Ok(None),
        }
    }

    async fn history(
        &self,
        thread: &ThreadId,
        limit: Option<u32>,
        before: Option<WaypointId>,
    ) -> Result<Vec<WaypointSummary>, WaypointError> {
        if limit == Some(0) {
            return Ok(vec![]);
        }
        let limit_value = limit.unwrap_or(u32::MAX) as i64;

        let rows = if let Some(before_id) = before {
            let cursor_row = sqlx::query(
                "SELECT created_at, superstep FROM waypoints WHERE thread_id = $1 AND waypoint_id = $2",
            )
            .bind(thread.as_str())
            .bind(before_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;

            let Some(cursor_row) = cursor_row else {
                return Ok(vec![]);
            };
            let cursor_created_at: DateTime<Utc> = cursor_row
                .try_get("created_at")
                .map_err(|e| WaypointError::Backend { source: e.into() })?;
            let cursor_superstep: i64 = cursor_row
                .try_get("superstep")
                .map_err(|e| WaypointError::Backend { source: e.into() })?;

            sqlx::query(HISTORY_QUERY_WITH_CURSOR)
                .bind(thread.as_str())
                .bind(cursor_created_at)
                .bind(cursor_created_at)
                .bind(cursor_superstep)
                .bind(limit_value)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| self.wrap_error(e))?
        } else {
            sqlx::query(HISTORY_QUERY_NO_CURSOR)
                .bind(thread.as_str())
                .bind(limit_value)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| self.wrap_error(e))?
        };

        rows.iter().map(Self::row_to_summary).collect()
    }

    async fn list_threads(
        &self,
        limit: Option<u32>,
        before: Option<DateTime<Utc>>,
    ) -> Result<Vec<ThreadSummary>, WaypointError> {
        if limit == Some(0) {
            return Ok(vec![]);
        }
        let limit_value = limit.unwrap_or(u32::MAX) as i64;

        let rows = if let Some(before) = before {
            sqlx::query(LIST_THREADS_QUERY_WITH_CURSOR)
                .bind(before)
                .bind(limit_value)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| self.wrap_error(e))?
        } else {
            sqlx::query(LIST_THREADS_QUERY_NO_CURSOR)
                .bind(limit_value)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| self.wrap_error(e))?
        };

        rows.iter().map(Self::row_to_thread_summary).collect()
    }

    async fn delete_thread(&self, thread: &ThreadId) -> Result<u64, WaypointError> {
        let result = sqlx::query("DELETE FROM waypoints WHERE thread_id = $1")
            .bind(thread.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;
        Ok(result.rows_affected())
    }

    async fn delete_waypoint(
        &self,
        thread: &ThreadId,
        id: &WaypointId,
    ) -> Result<bool, WaypointError> {
        let result = sqlx::query("DELETE FROM waypoints WHERE thread_id = $1 AND waypoint_id = $2")
            .bind(thread.as_str())
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;
        Ok(result.rows_affected() > 0)
    }

    async fn prune_thread(
        &self,
        thread: &ThreadId,
        keep: &[WaypointId],
    ) -> Result<u64, WaypointError> {
        // One statement inside the driver's implicit single-statement
        // transaction: `keep` is bound as a text array, so the keep-set
        // size carries no per-element parameter cost -- unlike a per-id
        // `IN` list built with one bound parameter per id, an array bind is
        // a single bound parameter regardless of how many elements it
        // holds. `waypoint_id <> ALL($2)` is "not equal to every element of
        // $2", i.e. "not present in $2"; an empty array makes that
        // vacuously true for every row, so the whole thread is removed --
        // exactly the specified empty-keep-set behaviour, confirmed by the
        // shared contract function rather than reasoned about here.
        let keep_ids: Vec<String> = keep.iter().map(|id| id.to_string()).collect();

        let result = sqlx::query(
            "DELETE FROM waypoints WHERE thread_id = $1 AND waypoint_id <> ALL($2::text[])",
        )
        .bind(thread.as_str())
        .bind(&keep_ids)
        .execute(&self.pool)
        .await
        .map_err(|e| self.wrap_error(e))?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::waypoint::contract_tests;

    // Docker-gated Tier 2 suite (D-10): every test independently probes the
    // `postgres-test` service (see `docker/docker-compose.test.yml`) and
    // prints a named reason then returns early -- never panics or hangs --
    // when it is not reachable, mirroring the Ollama Tier 2 suite's gate
    // (`tests/integration/ollama_docker_test.rs`). This whole module is
    // ALSO compile-time gated behind the `postgres` feature (see
    // `waypoint/mod.rs`), which is not in any default feature set, so a
    // plain `cargo test -p paladin-storage` never attempts to build it, let
    // alone run it.
    //
    // Bring the service up before running this suite:
    // ```sh
    // docker compose -f docker/docker-compose.test.yml up -d postgres-test
    // cargo test -p paladin-storage --features postgres --lib waypoint::postgres
    // ```
    //
    // NOTE: unlike the SQLite suite's per-test `sqlite::memory:` isolation,
    // every test in this module shares the one `postgres-test` database.
    // Each contract function operates under its own uniquely named
    // `ThreadId`, so tests do not interfere with each other -- EXCEPT
    // `list_threads_empty_then_three_threads_newest_activity_first`, which
    // asserts a literally empty store and therefore requires a freshly
    // brought-up (or freshly `down -v`'d) service, exactly what
    // `make test-integration-docker` provides.

    fn postgres_test_url() -> String {
        std::env::var("WAYPOINT_POSTGRES_TEST_URL").unwrap_or_else(|_| {
            "postgres://paladin:paladin@localhost:5433/paladin_waypoint_test".to_string()
        })
    }

    /// A cheap, short-timeout TCP reachability probe, tried BEFORE handing
    /// `url` to `sqlx`'s pool -- `sqlx::Pool::connect` treats a connection
    /// refusal as retryable and can absorb its whole `acquire_timeout`
    /// budget before surfacing an error, which would make every test in
    /// this suite slow (rather than a clean, fast skip) when
    /// `postgres-test` is simply not running.
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
    async fn store_or_skip() -> Option<PostgresWaypointStore> {
        let url = postgres_test_url();
        if !postgres_reachable(&url) {
            println!(
                "SKIP: postgres-test not reachable at {url} -- bring it up with \
                 `docker compose -f docker/docker-compose.test.yml up -d postgres-test`"
            );
            return None;
        }

        match PostgresWaypointStore::new(&url).await {
            Ok(store) => {
                // The shared contract functions assume a logically fresh store:
                // the SQLite and in-memory suites construct one per test, but
                // every test in this module shares the one `paladin_waypoint_test`
                // database, so residue from earlier tests breaks any contract
                // clause that asserts on the whole store (e.g. `list_threads`
                // on an empty store). Start each test from an empty table.
                // Safe: this suite runs single-threaded (`--test-threads=1`)
                // everywhere it is invoked (CI job and Makefile runner).
                if let Err(e) = sqlx::query("TRUNCATE TABLE waypoints")
                    .execute(&store.pool)
                    .await
                {
                    println!("SKIP: could not reset paladin_waypoint_test ({e})");
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

    // One #[tokio::test] per shared contract function (D-09), written out
    // explicitly (not via a macro) so each names the violated contract
    // clause on failure, mirroring sqlite.rs's test module exactly.

    #[tokio::test]
    async fn save_then_latest_returns_saved_waypoint_round_tripped() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::save_then_latest_returns_saved_waypoint_round_tripped(&store).await;
    }

    #[tokio::test]
    async fn latest_on_unknown_thread_is_none() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::latest_on_unknown_thread_is_none(&store).await;
    }

    #[tokio::test]
    async fn get_on_known_thread_unknown_id_is_none() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::get_on_known_thread_unknown_id_is_none(&store).await;
    }

    #[tokio::test]
    async fn get_on_known_thread_known_id_returns_exact_waypoint() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::get_on_known_thread_known_id_returns_exact_waypoint(&store).await;
    }

    #[tokio::test]
    async fn history_with_no_pagination_returns_all_newest_first() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::history_with_no_pagination_returns_all_newest_first(&store).await;
    }

    #[tokio::test]
    async fn history_limit_and_before_paginate_with_no_overlap_or_gap() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::history_limit_and_before_paginate_with_no_overlap_or_gap(&store).await;
    }

    #[tokio::test]
    async fn history_limit_zero_returns_empty() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::history_limit_zero_returns_empty(&store).await;
    }

    #[tokio::test]
    async fn history_on_unknown_thread_returns_empty() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::history_on_unknown_thread_returns_empty(&store).await;
    }

    #[tokio::test]
    async fn history_same_created_at_tiebreaks_by_descending_superstep_stably() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::history_same_created_at_tiebreaks_by_descending_superstep_stably(&store)
            .await;
    }

    #[tokio::test]
    async fn delete_thread_removes_count_and_unknown_returns_zero() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::delete_thread_removes_count_and_unknown_returns_zero(&store).await;
    }

    #[tokio::test]
    async fn resave_existing_waypoint_id_upserts() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::resave_existing_waypoint_id_upserts(&store).await;
    }

    #[tokio::test]
    async fn child_lineage_survives_round_trip() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::child_lineage_survives_round_trip(&store).await;
    }

    // The one clause requiring a literally empty store -- see the module
    // doc comment.
    #[tokio::test]
    async fn list_threads_empty_then_three_threads_newest_activity_first() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::list_threads_empty_then_three_threads_newest_activity_first(&store).await;
    }

    // ── delete_waypoint / prune_thread (Plan 22-13, G-22-2) ──────────────

    #[tokio::test]
    async fn delete_waypoint_removes_named_id_and_leaves_others() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::delete_waypoint_removes_named_id_and_leaves_others(&store).await;
    }

    #[tokio::test]
    async fn delete_waypoint_unknown_id_is_false_and_leaves_thread_unchanged() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::delete_waypoint_unknown_id_is_false_and_leaves_thread_unchanged(&store)
            .await;
    }

    #[tokio::test]
    async fn delete_waypoint_unknown_thread_is_false() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::delete_waypoint_unknown_thread_is_false(&store).await;
    }

    #[tokio::test]
    async fn prune_thread_keeps_named_ids_byte_identical_and_ordered() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::prune_thread_keeps_named_ids_byte_identical_and_ordered(&store).await;
    }

    #[tokio::test]
    async fn prune_thread_empty_keep_removes_everything() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::prune_thread_empty_keep_removes_everything(&store).await;
    }

    #[tokio::test]
    async fn prune_thread_unknown_thread_returns_zero() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::prune_thread_unknown_thread_returns_zero(&store).await;
    }

    #[tokio::test]
    async fn prune_thread_ignores_keep_ids_not_in_thread() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::prune_thread_ignores_keep_ids_not_in_thread(&store).await;
    }

    #[tokio::test]
    async fn prune_thread_idempotent_second_run_removes_nothing() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::prune_thread_idempotent_second_run_removes_nothing(&store).await;
    }

    #[tokio::test]
    async fn prune_thread_converges_from_superset_to_target() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::prune_thread_converges_from_superset_to_target(&store).await;
    }

    #[tokio::test]
    async fn prune_thread_large_keep_set_1200_to_1100() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::prune_thread_large_keep_set_1200_to_1100(&store).await;
    }

    // ── BUG-04 / ENG-FR-12a: FrontierSnapshot (Plan 22.1-06) ─────────────

    #[tokio::test]
    async fn frontier_survives_save_latest_and_get_round_trip() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::frontier_survives_save_latest_and_get_round_trip(&store).await;
    }

    #[tokio::test]
    async fn pre_bug_04_payload_without_frontier_loads_with_an_empty_snapshot() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::pre_bug_04_payload_without_frontier_loads_with_an_empty_snapshot(&store)
            .await;
    }

    // ── CF-FR-12 / D-14: MusterProgress (Plan 23-06) ──────────────────────

    #[tokio::test]
    async fn muster_progress_round_trips() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::muster_progress_round_trips(&store).await;
    }

    #[tokio::test]
    async fn muster_progress_none_round_trips_as_none() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::muster_progress_none_round_trips_as_none(&store).await;
    }

    // ── CF-FR-15 / D-20: checkpoint_ns (Plan 23-09) ───────────────────────

    #[tokio::test]
    async fn checkpoint_ns_round_trips() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::checkpoint_ns_round_trips(&store).await;
    }

    #[tokio::test]
    async fn checkpoint_ns_none_round_trips() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::checkpoint_ns_none_round_trips(&store).await;
    }

    // ── D-02 / D-14 / D-15: AwaitingInput payload, fork_of, branch-aware
    //    latest ordering (Phase 24 Plan 06). Tier 2 -- Docker is unavailable
    //    in this devcontainer, so these three wrappers are skipped locally
    //    (`store_or_skip`) and run only in CI's `postgres-integration` job;
    //    they are NOT recorded as passed from a local run.

    #[tokio::test]
    async fn awaiting_input_payload_round_trips() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::awaiting_input_payload_round_trips(&store).await;
    }

    #[tokio::test]
    async fn fork_of_round_trips() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::fork_of_round_trips(&store).await;
    }

    #[tokio::test]
    async fn latest_prefers_most_recently_created_across_branches() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::latest_prefers_most_recently_created_across_branches(&store).await;
    }

    #[tokio::test]
    async fn prune_thread_thread_id_and_waypoint_id_with_sql_metacharacters_round_trip_as_data() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        let thread = ThreadId::new("thread-o'brien;DROP-TABLE--comment").unwrap();
        let base = Utc::now();
        let mut saved = Vec::new();
        for superstep in 0..3u64 {
            let wp = contract_tests::sample_waypoint_at(
                &thread,
                superstep,
                base + chrono::Duration::seconds(superstep as i64),
            );
            store.save(&wp).await.unwrap();
            saved.push(wp);
        }

        let keep = vec![saved[2].waypoint_id];
        let removed = store.prune_thread(&thread, &keep).await.unwrap();
        assert_eq!(removed, 2);

        let history = store.history(&thread, None, None).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].waypoint_id, saved[2].waypoint_id);
    }

    #[tokio::test]
    async fn payload_written_as_jsonb_reads_back_as_equal_waypoint() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        let thread = ThreadId::new("contract-postgres-jsonb-roundtrip").unwrap();
        let wp = contract_tests::sample_waypoint(&thread, 0);
        store.save(&wp).await.unwrap();

        let loaded = store.get(&thread, &wp.waypoint_id).await.unwrap().unwrap();
        assert_eq!(loaded, wp);
    }

    #[tokio::test]
    async fn connection_error_redacts_password_from_database_url() {
        let url = "postgres://user:hunter2-secret@127.0.0.1:1/nonexistent";
        let err = PostgresWaypointStore::new(url).await.unwrap_err();
        let message = err.to_string();
        assert!(
            !message.contains("hunter2-secret"),
            "connection error leaked the password: {message}"
        );
    }
}
