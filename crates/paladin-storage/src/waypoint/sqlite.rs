/*
SQLite Waypoint Store

Concrete `WaypointPort` implementation over SQLite (ENG-05, D-01, D-02).
`payload` is stored as TEXT holding the serialized JSON `Waypoint`; `status`
is stored as its own serialized-JSON column so history/list_threads
summaries can be built without deserializing the (potentially large) full
payload. Every statement uses bound parameters (T-22-17); no query string is
ever built by formatting a caller-supplied value into it. Migrations follow
the versioned-file convention at
`crates/paladin-storage/migrations/sqlite/` (D-02), embedded at compile time
via `sqlx::migrate!` and applied automatically on construction.
*/

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::{Row, sqlite::SqliteRow};
use std::str::FromStr;

use paladin_core::platform::container::waypoint::{ThreadId, Waypoint, WaypointId, WaypointStatus};
use paladin_ports::output::waypoint_port::{
    ThreadSummary, WaypointError, WaypointPort, WaypointSummary,
};

use crate::waypoint::redact::redact_database_url_password;

/// `history()` with no `before` cursor: newest-first, tiebreak on
/// descending `superstep`, bounded by a bound `LIMIT` parameter (never an
/// unbounded query — `None` is translated to `u32::MAX` by the caller so a
/// single query shape covers both cases without `format!`-building SQL).
const HISTORY_QUERY_NO_CURSOR: &str = r#"
    SELECT waypoint_id, parent_id, superstep, status, created_at
    FROM waypoints
    WHERE thread_id = ?
    ORDER BY created_at DESC, superstep DESC
    LIMIT ?
"#;

/// `history()` with a `before` cursor, resolved to its own
/// `(created_at, superstep)` first (see `history`'s body): the row-value
/// comparison `created_at < ? OR (created_at = ? AND superstep < ?)` is
/// exactly "strictly older, under the same order the query itself sorts by".
const HISTORY_QUERY_WITH_CURSOR: &str = r#"
    SELECT waypoint_id, parent_id, superstep, status, created_at
    FROM waypoints
    WHERE thread_id = ?
      AND (created_at < ? OR (created_at = ? AND superstep < ?))
    ORDER BY created_at DESC, superstep DESC
    LIMIT ?
"#;

/// `list_threads()` with no `before` cursor: one row per thread (its latest
/// Waypoint, by the same `created_at`/`superstep` order `history` uses),
/// newest-activity-first.
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
    LIMIT ?
"#;

/// `list_threads()` with a `before: DateTime<Utc>` cursor (an exclusive,
/// directly bindable cutoff -- unlike `history`'s id cursor, no lookup is
/// needed first).
const LIST_THREADS_QUERY_WITH_CURSOR: &str = r#"
    SELECT thread_id, status, created_at FROM (
        SELECT thread_id, status, created_at,
               ROW_NUMBER() OVER (
                   PARTITION BY thread_id
                   ORDER BY created_at DESC, superstep DESC
               ) AS rn
        FROM waypoints
    ) ranked
    WHERE rn = 1 AND created_at < ?
    ORDER BY created_at DESC
    LIMIT ?
"#;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("migrations/sqlite");

/// SQLite `WaypointPort` implementation (ENG-05, Tier 1: always exercised in
/// CI, no external service required).
#[derive(Debug)]
pub struct SqliteWaypointStore {
    pool: SqlitePool,
    /// Kept so every error this store returns can be redacted of the
    /// connection URL's password (T-22-18), not just construction-time
    /// connection errors -- defense in depth against a future driver
    /// version that starts embedding the DSN in a query error too.
    database_url: String,
}

impl SqliteWaypointStore {
    /// Connect to `database_url`, creating the database file if missing, and
    /// apply the versioned migration. Safe to call more than once against
    /// the same database file: the migration is idempotent
    /// (`CREATE TABLE IF NOT EXISTS`/`CREATE INDEX IF NOT EXISTS`) and
    /// `sqlx::migrate::Migrator` itself tracks applied versions.
    pub async fn new(database_url: &str) -> Result<Self, WaypointError> {
        let options = SqliteConnectOptions::from_str(database_url)
            .map_err(|e| Self::wrap(database_url, e))?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            // A single connection avoids the classic sqlx/SQLite pitfall
            // where `sqlite::memory:` (or any bare in-memory URI) hands out
            // a *different*, independently empty database per pooled
            // connection -- this backend's own tests rely on
            // `sqlite::memory:` and must observe their own writes.
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

    /// Wrap a driver/migration error into `WaypointError::Backend`, with the
    /// connection URL's password redacted from the error text first
    /// (T-22-18) -- redact before any truncation, per this project's
    /// security instructions.
    fn wrap(database_url: &str, err: impl std::error::Error) -> WaypointError {
        let redacted = redact_database_url_password(&err.to_string(), database_url);
        WaypointError::Backend {
            source: redacted.into(),
        }
    }

    fn wrap_error(&self, err: sqlx::Error) -> WaypointError {
        Self::wrap(&self.database_url, err)
    }

    /// Deserialize a full `Waypoint` from its stored `payload` JSON,
    /// enforcing the schema-version check (T-22-19): a payload written by an
    /// unrecognised newer release maps to `SchemaVersionUnsupported` rather
    /// than a structurally-successful misparse.
    fn parse_payload(payload_json: &str) -> Result<Waypoint, WaypointError> {
        let wp: Waypoint = serde_json::from_str(payload_json)
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

    /// Reconstruct a `WaypointId` from its stored TEXT form. `WaypointId` is
    /// `#[serde(transparent)]` over a `Uuid`, so deserializing a JSON string
    /// through it round-trips without needing a dedicated
    /// paladin-core-side constructor (out of this plan's file scope for
    /// Task 1).
    fn parse_waypoint_id(raw: &str) -> Result<WaypointId, WaypointError> {
        serde_json::from_value(serde_json::Value::String(raw.to_string()))
            .map_err(|e| WaypointError::Serialization(format!("invalid waypoint_id: {e}")))
    }

    fn row_to_summary(row: &SqliteRow) -> Result<WaypointSummary, WaypointError> {
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

        Ok(WaypointSummary {
            waypoint_id,
            parent_waypoint_id,
            superstep: superstep as u64,
            status,
            created_at,
        })
    }

    fn row_to_thread_summary(row: &SqliteRow) -> Result<ThreadSummary, WaypointError> {
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
impl WaypointPort for SqliteWaypointStore {
    async fn save(&self, wp: &Waypoint) -> Result<(), WaypointError> {
        let payload =
            serde_json::to_string(wp).map_err(|e| WaypointError::Serialization(e.to_string()))?;
        let status_json = serde_json::to_string(&wp.status)
            .map_err(|e| WaypointError::Serialization(e.to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO waypoints
                (waypoint_id, thread_id, parent_id, superstep, status, payload, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(waypoint_id) DO UPDATE SET
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
            "SELECT payload FROM waypoints WHERE thread_id = ? \
             ORDER BY created_at DESC, superstep DESC LIMIT 1",
        )
        .bind(thread.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| self.wrap_error(e))?;

        match row {
            Some(row) => {
                let payload: String = row
                    .try_get("payload")
                    .map_err(|e| WaypointError::Backend { source: e.into() })?;
                Ok(Some(Self::parse_payload(&payload)?))
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
            sqlx::query("SELECT payload FROM waypoints WHERE thread_id = ? AND waypoint_id = ?")
                .bind(thread.as_str())
                .bind(id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| self.wrap_error(e))?;

        match row {
            Some(row) => {
                let payload: String = row
                    .try_get("payload")
                    .map_err(|e| WaypointError::Backend { source: e.into() })?;
                Ok(Some(Self::parse_payload(&payload)?))
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
            // Resolve the cursor to its own (created_at, superstep) first --
            // both values read back FROM the database rather than recomputed
            // in Rust, so the equality branch of the row-value comparison
            // below matches the stored representation exactly.
            let cursor_row = sqlx::query(
                "SELECT created_at, superstep FROM waypoints WHERE thread_id = ? AND waypoint_id = ?",
            )
            .bind(thread.as_str())
            .bind(before_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;

            let Some(cursor_row) = cursor_row else {
                // The cursor does not identify a waypoint of this thread:
                // there is no well-defined "next page" to return.
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
        let result = sqlx::query("DELETE FROM waypoints WHERE thread_id = ?")
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
        let result = sqlx::query("DELETE FROM waypoints WHERE thread_id = ? AND waypoint_id = ?")
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
        // sqlx's SQLite driver has no array binding, and a per-id `IN` list
        // built with one bound parameter per id would run into the
        // per-statement bound-parameter limit for a large keep-set (the
        // reason the 1,200-Waypoint contract test exists). Atomicity here
        // comes from wrapping every deletion for this call in one explicit
        // transaction, not from issuing a single statement -- so chunking
        // the deletions costs nothing.
        let keep_set: std::collections::HashSet<String> =
            keep.iter().map(|id| id.to_string()).collect();

        let mut tx = self.pool.begin().await.map_err(|e| self.wrap_error(e))?;

        let existing_rows = sqlx::query("SELECT waypoint_id FROM waypoints WHERE thread_id = ?")
            .bind(thread.as_str())
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| self.wrap_error(e))?;

        let delete_ids: Vec<String> = existing_rows
            .iter()
            .filter_map(|row| {
                let id: String = row.try_get("waypoint_id").ok()?;
                (!keep_set.contains(&id)).then_some(id)
            })
            .collect();

        // Chunk size comfortably under SQLite's historical default
        // per-statement bound-parameter limit of 999
        // (`SQLITE_MAX_VARIABLE_NUMBER`, pre-3.32.0 -- this store does not
        // assume a newer bundled SQLite raising that ceiling to 32,766).
        // One statement binds `thread_id` plus up to this many ids, so 500
        // keeps every chunked DELETE comfortably inside 999 even on the
        // conservative bound -- exactly why the 1,200-to-1,100 contract
        // test exists: 1,100 bound ids in a single `IN` list would exceed
        // it.
        const PRUNE_DELETE_CHUNK_SIZE: usize = 500;

        let mut removed: u64 = 0;
        for chunk in delete_ids.chunks(PRUNE_DELETE_CHUNK_SIZE) {
            let mut builder: sqlx::QueryBuilder<sqlx::Sqlite> =
                sqlx::QueryBuilder::new("DELETE FROM waypoints WHERE thread_id = ");
            builder.push_bind(thread.as_str().to_string());
            builder.push(" AND waypoint_id IN (");
            let mut separated = builder.separated(", ");
            for id in chunk {
                separated.push_bind(id.clone());
            }
            separated.push_unseparated(")");

            let result = builder
                .build()
                .execute(&mut *tx)
                .await
                .map_err(|e| self.wrap_error(e))?;
            removed += result.rows_affected();
        }

        tx.commit().await.map_err(|e| self.wrap_error(e))?;
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::waypoint::contract_tests;

    // One #[tokio::test] per shared contract function (D-09), each against a
    // fresh in-memory database, so a failure names the violated contract
    // clause. See `contract_tests` for the assertions themselves -- this
    // file only wires SqliteWaypointStore into them, unchanged.

    async fn fresh_store() -> SqliteWaypointStore {
        SqliteWaypointStore::new("sqlite::memory:").await.unwrap()
    }

    #[tokio::test]
    async fn save_then_latest_returns_saved_waypoint_round_tripped() {
        contract_tests::save_then_latest_returns_saved_waypoint_round_tripped(&fresh_store().await)
            .await;
    }

    #[tokio::test]
    async fn latest_on_unknown_thread_is_none() {
        contract_tests::latest_on_unknown_thread_is_none(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn get_on_known_thread_unknown_id_is_none() {
        contract_tests::get_on_known_thread_unknown_id_is_none(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn get_on_known_thread_known_id_returns_exact_waypoint() {
        contract_tests::get_on_known_thread_known_id_returns_exact_waypoint(&fresh_store().await)
            .await;
    }

    #[tokio::test]
    async fn history_with_no_pagination_returns_all_newest_first() {
        contract_tests::history_with_no_pagination_returns_all_newest_first(&fresh_store().await)
            .await;
    }

    #[tokio::test]
    async fn history_limit_and_before_paginate_with_no_overlap_or_gap() {
        contract_tests::history_limit_and_before_paginate_with_no_overlap_or_gap(
            &fresh_store().await,
        )
        .await;
    }

    #[tokio::test]
    async fn history_limit_zero_returns_empty() {
        contract_tests::history_limit_zero_returns_empty(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn history_on_unknown_thread_returns_empty() {
        contract_tests::history_on_unknown_thread_returns_empty(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn history_same_created_at_tiebreaks_by_descending_superstep_stably() {
        contract_tests::history_same_created_at_tiebreaks_by_descending_superstep_stably(
            &fresh_store().await,
        )
        .await;
    }

    #[tokio::test]
    async fn list_threads_empty_then_three_threads_newest_activity_first() {
        contract_tests::list_threads_empty_then_three_threads_newest_activity_first(
            &fresh_store().await,
        )
        .await;
    }

    #[tokio::test]
    async fn delete_thread_removes_count_and_unknown_returns_zero() {
        contract_tests::delete_thread_removes_count_and_unknown_returns_zero(&fresh_store().await)
            .await;
    }

    #[tokio::test]
    async fn resave_existing_waypoint_id_upserts() {
        contract_tests::resave_existing_waypoint_id_upserts(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn child_lineage_survives_round_trip() {
        contract_tests::child_lineage_survives_round_trip(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn run_all_contract_functions_smoke_aggregate() {
        contract_tests::run_all(&fresh_store().await).await;
    }

    // ── delete_waypoint / prune_thread (Plan 22-13, G-22-2) ──────────────

    #[tokio::test]
    async fn delete_waypoint_removes_named_id_and_leaves_others() {
        contract_tests::delete_waypoint_removes_named_id_and_leaves_others(&fresh_store().await)
            .await;
    }

    #[tokio::test]
    async fn delete_waypoint_unknown_id_is_false_and_leaves_thread_unchanged() {
        contract_tests::delete_waypoint_unknown_id_is_false_and_leaves_thread_unchanged(
            &fresh_store().await,
        )
        .await;
    }

    #[tokio::test]
    async fn delete_waypoint_unknown_thread_is_false() {
        contract_tests::delete_waypoint_unknown_thread_is_false(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn prune_thread_keeps_named_ids_byte_identical_and_ordered() {
        contract_tests::prune_thread_keeps_named_ids_byte_identical_and_ordered(
            &fresh_store().await,
        )
        .await;
    }

    #[tokio::test]
    async fn prune_thread_empty_keep_removes_everything() {
        contract_tests::prune_thread_empty_keep_removes_everything(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn prune_thread_unknown_thread_returns_zero() {
        contract_tests::prune_thread_unknown_thread_returns_zero(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn prune_thread_ignores_keep_ids_not_in_thread() {
        contract_tests::prune_thread_ignores_keep_ids_not_in_thread(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn prune_thread_idempotent_second_run_removes_nothing() {
        contract_tests::prune_thread_idempotent_second_run_removes_nothing(&fresh_store().await)
            .await;
    }

    #[tokio::test]
    async fn prune_thread_converges_from_superset_to_target() {
        contract_tests::prune_thread_converges_from_superset_to_target(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn prune_thread_large_keep_set_1200_to_1100() {
        contract_tests::prune_thread_large_keep_set_1200_to_1100(&fresh_store().await).await;
    }

    // ── BUG-04 / ENG-FR-12a: FrontierSnapshot (Plan 22.1-06) ─────────────

    #[tokio::test]
    async fn frontier_survives_save_latest_and_get_round_trip() {
        contract_tests::frontier_survives_save_latest_and_get_round_trip(&fresh_store().await)
            .await;
    }

    #[tokio::test]
    async fn pre_bug_04_payload_without_frontier_loads_with_an_empty_snapshot() {
        contract_tests::pre_bug_04_payload_without_frontier_loads_with_an_empty_snapshot(
            &fresh_store().await,
        )
        .await;
    }

    // ── Backend-specific tests (T-22-17, T-22-18) ───────────────────────

    #[tokio::test]
    async fn thread_id_with_sql_metacharacter_round_trips_as_data() {
        let store = fresh_store().await;
        let thread = ThreadId::new("thread-o'brien;DROP-TABLE--comment").unwrap();
        let wp = contract_tests::sample_waypoint(&thread, 0);
        store.save(&wp).await.unwrap();

        let loaded = store.get(&thread, &wp.waypoint_id).await.unwrap().unwrap();
        assert_eq!(loaded.thread_id, thread);

        // The table must still exist and be queryable -- a real injection
        // via string formatting would have executed the embedded DROP TABLE.
        let history = store.history(&thread, None, None).await.unwrap();
        assert_eq!(history.len(), 1);
    }

    #[tokio::test]
    async fn prune_thread_thread_id_and_waypoint_id_with_sql_metacharacters_round_trip_as_data() {
        let store = fresh_store().await;
        let thread = ThreadId::new("thread-o'brien;DROP-TABLE--comment").unwrap();
        let base = chrono::Utc::now();
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

        // Keep only the newest; prune the other two, exercising both the
        // enumeration SELECT and the chunked DELETE with a metacharacter-
        // laden thread_id as bound data, never interpolated text.
        let keep = vec![saved[2].waypoint_id];
        let removed = store.prune_thread(&thread, &keep).await.unwrap();
        assert_eq!(removed, 2);

        // The table must still exist and be queryable -- a real injection
        // via string formatting would have executed the embedded DROP TABLE.
        let history = store.history(&thread, None, None).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].waypoint_id, saved[2].waypoint_id);
    }

    #[tokio::test]
    async fn connection_error_redacts_password_from_database_url() {
        // A syntactically URL-like DSN carrying a password, using a scheme
        // SqliteConnectOptions cannot parse -- guaranteed to fail before any
        // real connection attempt, exactly like a real bad-credential
        // failure would.
        let url = "sqlite://user:hunter2-secret@/nonexistent/path/that/does/not/exist.db";
        let err = SqliteWaypointStore::new(url).await.unwrap_err();
        let message = err.to_string();
        assert!(
            !message.contains("hunter2-secret"),
            "connection error leaked the password: {message}"
        );
    }

    #[tokio::test]
    async fn constructing_store_twice_against_same_file_is_idempotent() {
        let path = std::env::temp_dir().join(format!(
            "paladin_waypoint_idempotent_test_{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let url = format!("sqlite://{}", path.display());

        let first = SqliteWaypointStore::new(&url).await.unwrap();
        let thread = ThreadId::new("idempotent-construction").unwrap();
        first
            .save(&contract_tests::sample_waypoint(&thread, 0))
            .await
            .unwrap();
        drop(first);

        // Second construction against the identical file must succeed (the
        // migration is idempotent) and must see the first construction's
        // write.
        let second = SqliteWaypointStore::new(&url).await.unwrap();
        assert!(second.latest(&thread).await.unwrap().is_some());

        let _ = std::fs::remove_file(&path);
    }
}
