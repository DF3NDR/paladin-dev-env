/*
SQLite Webhook Delivery Repository

Concrete `WebhookDeliveryRepositoryPort` implementation over SQLite
(PLAT-FR-14/15, D-40, D-43). `claim_due` is a SELECT of candidate ids
followed by one conditional `UPDATE webhook_deliveries SET status =
'in_flight', updated_at = ? WHERE delivery_id = ? AND status IN
('pending', 'retrying') AND next_attempt_at <= ?` per candidate --
`rows_affected() == 1` means THIS caller won that row's claim, `0` means a
concurrent caller (or a stale predicate) already took it, in which case the
row is simply skipped rather than included in the returned batch. Every
statement uses bound parameters; no query string is ever built by
formatting a caller-supplied value into it. Migrations follow the
versioned-file convention at `crates/paladin-storage/migrations/sqlite/`,
embedded at compile time via `sqlx::migrate!` and applied automatically on
construction.
*/

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::sqlite::{Sqlite, SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::{QueryBuilder, Row, sqlite::SqliteRow};
use std::str::FromStr;

use paladin_core::platform::container::run::{RunEventKind, RunId};
use paladin_core::platform::container::waypoint::ThreadId;
use paladin_core::platform::container::webhook::{
    WEBHOOK_DELIVERY_SCHEMA_VERSION, WebhookAttemptOutcome, WebhookAttemptResult, WebhookDelivery,
    WebhookDeliveryId, WebhookDeliveryStatus,
};
use paladin_ports::output::webhook_delivery_port::{
    WebhookDeliveryPage, WebhookDeliveryRepositoryError, WebhookDeliveryRepositoryPort,
};

use crate::waypoint::redact::redact_database_url_password;

const INSERT_DELIVERY: &str = "INSERT INTO webhook_deliveries \
     (delivery_id, run_id, thread_id, event, url, payload, attempt, max_attempts, status, \
      next_attempt_at, last_response_status, last_error, created_at, updated_at, \
      schema_version) \
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";

const SELECT_BY_ID: &str = "SELECT delivery_id, run_id, thread_id, event, url, payload, \
     attempt, max_attempts, status, next_attempt_at, last_response_status, last_error, \
     created_at, updated_at, schema_version FROM webhook_deliveries WHERE delivery_id = ?";

const SELECT_CLAIMABLE_IDS: &str = "SELECT delivery_id FROM webhook_deliveries \
     WHERE status IN ('pending', 'retrying') AND next_attempt_at <= ? \
     ORDER BY next_attempt_at ASC LIMIT ?";

const CLAIM_ROW: &str = "UPDATE webhook_deliveries SET status = 'in_flight', updated_at = ? \
     WHERE delivery_id = ? AND status IN ('pending', 'retrying') AND next_attempt_at <= ?";

const LIST_FOR_RUN_PREFIX: &str = "SELECT delivery_id, run_id, thread_id, event, url, payload, \
     attempt, max_attempts, status, next_attempt_at, last_response_status, last_error, \
     created_at, updated_at, schema_version FROM webhook_deliveries WHERE run_id = ";

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("migrations/sqlite");

/// `RunEventKind` (paladin-core) carries no `as_str`/`from_str` of its own;
/// these mirror `on_missed_to_str`/`on_missed_from_str`'s precedent
/// (`run_schedule/sqlite.rs`) as a plain TEXT enum column (no payload, so
/// no JSON).
fn event_to_str(event: RunEventKind) -> &'static str {
    match event {
        RunEventKind::AwaitingInput => "awaiting_input",
        RunEventKind::Completed => "completed",
        RunEventKind::Failed => "failed",
        RunEventKind::Halted => "halted",
        RunEventKind::Cancelled => "cancelled",
    }
}

fn event_from_str(s: &str) -> Result<RunEventKind, WebhookDeliveryRepositoryError> {
    match s {
        "awaiting_input" => Ok(RunEventKind::AwaitingInput),
        "completed" => Ok(RunEventKind::Completed),
        "failed" => Ok(RunEventKind::Failed),
        "halted" => Ok(RunEventKind::Halted),
        "cancelled" => Ok(RunEventKind::Cancelled),
        other => Err(WebhookDeliveryRepositoryError::Serialization {
            message: format!("unknown run event kind: {other:?}"),
        }),
    }
}

fn status_to_str(status: WebhookDeliveryStatus) -> &'static str {
    status.as_str()
}

fn status_from_str(s: &str) -> Result<WebhookDeliveryStatus, WebhookDeliveryRepositoryError> {
    match s {
        "pending" => Ok(WebhookDeliveryStatus::Pending),
        "in_flight" => Ok(WebhookDeliveryStatus::InFlight),
        "delivered" => Ok(WebhookDeliveryStatus::Delivered),
        "retrying" => Ok(WebhookDeliveryStatus::Retrying),
        "dead" => Ok(WebhookDeliveryStatus::Dead),
        other => Err(WebhookDeliveryRepositoryError::Serialization {
            message: format!("unknown webhook delivery status: {other:?}"),
        }),
    }
}

/// SQLite `WebhookDeliveryRepositoryPort` implementation (PLAT-FR-14/15,
/// Tier 1: always exercised in CI, no external service required).
#[derive(Debug)]
pub struct SqliteWebhookDeliveryRepository {
    pool: SqlitePool,
    /// Kept so every error this store returns can be redacted of the
    /// connection URL's password, mirroring `SqliteAssistantRepository`'s
    /// rationale (T-22-18).
    database_url: String,
}

impl SqliteWebhookDeliveryRepository {
    /// Connect to `database_url`, creating the database file if missing, and
    /// apply the versioned migration. Safe to call more than once against
    /// the same database file: the migration is idempotent and
    /// `sqlx::migrate::Migrator` itself tracks applied versions.
    pub async fn new(database_url: &str) -> Result<Self, WebhookDeliveryRepositoryError> {
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

    fn wrap(database_url: &str, err: impl std::error::Error) -> WebhookDeliveryRepositoryError {
        let redacted = redact_database_url_password(&err.to_string(), database_url);
        WebhookDeliveryRepositoryError::Backend {
            source: redacted.into(),
        }
    }

    fn wrap_error(&self, err: sqlx::Error) -> WebhookDeliveryRepositoryError {
        Self::wrap(&self.database_url, err)
    }

    fn row_to_delivery(row: &SqliteRow) -> Result<WebhookDelivery, WebhookDeliveryRepositoryError> {
        let backend_err =
            |e: sqlx::Error| WebhookDeliveryRepositoryError::Backend { source: e.into() };

        let delivery_id_str: String = row.try_get("delivery_id").map_err(backend_err)?;
        let delivery_id = WebhookDeliveryId::parse(delivery_id_str).map_err(|e| {
            WebhookDeliveryRepositoryError::Serialization {
                message: format!("invalid delivery_id: {e}"),
            }
        })?;
        let run_id_str: String = row.try_get("run_id").map_err(backend_err)?;
        let run_id = RunId::parse(run_id_str).map_err(|e| {
            WebhookDeliveryRepositoryError::Serialization {
                message: format!("invalid run_id: {e}"),
            }
        })?;
        let thread_id_str: String = row.try_get("thread_id").map_err(backend_err)?;
        let thread_id = ThreadId::new(thread_id_str).map_err(|e| {
            WebhookDeliveryRepositoryError::Serialization {
                message: format!("invalid thread_id: {e}"),
            }
        })?;
        let event_str: String = row.try_get("event").map_err(backend_err)?;
        let event = event_from_str(&event_str)?;
        let url: String = row.try_get("url").map_err(backend_err)?;
        let payload: String = row.try_get("payload").map_err(backend_err)?;
        let attempt: i64 = row.try_get("attempt").map_err(backend_err)?;
        let max_attempts: i64 = row.try_get("max_attempts").map_err(backend_err)?;
        let status_str: String = row.try_get("status").map_err(backend_err)?;
        let status = status_from_str(&status_str)?;
        let next_attempt_at: DateTime<Utc> = row.try_get("next_attempt_at").map_err(backend_err)?;
        let last_response_status: Option<i64> =
            row.try_get("last_response_status").map_err(backend_err)?;
        let last_error: Option<String> = row.try_get("last_error").map_err(backend_err)?;
        let created_at: DateTime<Utc> = row.try_get("created_at").map_err(backend_err)?;
        let updated_at: DateTime<Utc> = row.try_get("updated_at").map_err(backend_err)?;
        let schema_version: String = row.try_get("schema_version").map_err(backend_err)?;

        if schema_version != WEBHOOK_DELIVERY_SCHEMA_VERSION {
            return Err(WebhookDeliveryRepositoryError::UnknownSchemaVersion {
                found: schema_version,
            });
        }

        let mut delivery = WebhookDelivery::new(
            delivery_id,
            run_id,
            thread_id,
            event,
            url,
            payload,
            next_attempt_at,
        );
        delivery.attempt = attempt as u32;
        delivery.max_attempts = max_attempts as u32;
        delivery.status = status;
        delivery.last_response_status = last_response_status.map(|v| v as u16);
        delivery.last_error = last_error;
        delivery.created_at = created_at;
        delivery.updated_at = updated_at;
        delivery.schema_version = schema_version;
        Ok(delivery)
    }
}

#[async_trait]
impl WebhookDeliveryRepositoryPort for SqliteWebhookDeliveryRepository {
    async fn enqueue(
        &self,
        delivery: WebhookDelivery,
    ) -> Result<(), WebhookDeliveryRepositoryError> {
        sqlx::query(INSERT_DELIVERY)
            .bind(delivery.delivery_id.as_str())
            .bind(delivery.run_id.as_str())
            .bind(delivery.thread_id.as_str())
            .bind(event_to_str(delivery.event))
            .bind(&delivery.url)
            .bind(&delivery.payload)
            .bind(delivery.attempt as i64)
            .bind(delivery.max_attempts as i64)
            .bind(status_to_str(delivery.status))
            .bind(delivery.next_attempt_at)
            .bind(delivery.last_response_status.map(|v| v as i64))
            .bind(&delivery.last_error)
            .bind(delivery.created_at)
            .bind(delivery.updated_at)
            .bind(&delivery.schema_version)
            .execute(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;
        Ok(())
    }

    async fn get(
        &self,
        delivery_id: &WebhookDeliveryId,
    ) -> Result<Option<WebhookDelivery>, WebhookDeliveryRepositoryError> {
        let row = sqlx::query(SELECT_BY_ID)
            .bind(delivery_id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;
        row.as_ref().map(Self::row_to_delivery).transpose()
    }

    async fn claim_due(
        &self,
        now: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<WebhookDelivery>, WebhookDeliveryRepositoryError> {
        let fetch_limit: i64 = if limit == 0 { i64::MAX } else { limit as i64 };
        let candidate_rows = sqlx::query(SELECT_CLAIMABLE_IDS)
            .bind(now)
            .bind(fetch_limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;

        let mut claimed = Vec::with_capacity(candidate_rows.len());
        for row in candidate_rows {
            let backend_err =
                |e: sqlx::Error| WebhookDeliveryRepositoryError::Backend { source: e.into() };
            let id_str: String = row.try_get("delivery_id").map_err(backend_err)?;

            let result = sqlx::query(CLAIM_ROW)
                .bind(Utc::now())
                .bind(&id_str)
                .bind(now)
                .execute(&self.pool)
                .await
                .map_err(|e| self.wrap_error(e))?;

            if result.rows_affected() == 1 {
                let claimed_row = sqlx::query(SELECT_BY_ID)
                    .bind(&id_str)
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(|e| self.wrap_error(e))?;
                if let Some(claimed_row) = claimed_row {
                    claimed.push(Self::row_to_delivery(&claimed_row)?);
                }
            }
            // rows_affected() == 0: another caller already claimed this
            // row, or its predicate went stale between the SELECT above
            // and this UPDATE -- simply not included in this batch.
        }
        Ok(claimed)
    }

    async fn record_attempt(
        &self,
        delivery_id: &WebhookDeliveryId,
        result: WebhookAttemptResult,
    ) -> Result<(), WebhookDeliveryRepositoryError> {
        let (status, next_attempt_at) = match result.outcome {
            WebhookAttemptOutcome::Delivered => (WebhookDeliveryStatus::Delivered, None),
            WebhookAttemptOutcome::Retrying { next_attempt_at } => {
                (WebhookDeliveryStatus::Retrying, Some(next_attempt_at))
            }
            WebhookAttemptOutcome::Dead => (WebhookDeliveryStatus::Dead, None),
        };

        let mut builder: QueryBuilder<Sqlite> =
            QueryBuilder::new("UPDATE webhook_deliveries SET attempt = attempt + 1, status = ");
        builder.push_bind(status_to_str(status));
        builder.push(", last_response_status = ");
        builder.push_bind(result.response_status.map(|v| v as i64));
        builder.push(", last_error = ");
        builder.push_bind(result.error);
        if let Some(next_attempt_at) = next_attempt_at {
            builder.push(", next_attempt_at = ");
            builder.push_bind(next_attempt_at);
        }
        builder.push(", updated_at = ");
        builder.push_bind(Utc::now());
        builder.push(" WHERE delivery_id = ");
        builder.push_bind(delivery_id.as_str().to_string());

        let outcome = builder
            .build()
            .execute(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;

        if outcome.rows_affected() == 0 {
            return Err(WebhookDeliveryRepositoryError::NotFound {
                delivery_id: delivery_id.clone(),
            });
        }
        Ok(())
    }

    async fn list_for_run(
        &self,
        run_id: &RunId,
        limit: u32,
        cursor: Option<WebhookDeliveryId>,
    ) -> Result<WebhookDeliveryPage, WebhookDeliveryRepositoryError> {
        let unlimited = limit == 0;
        let fetch_limit: i64 = if unlimited {
            i64::MAX
        } else {
            limit as i64 + 1
        };

        let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new(LIST_FOR_RUN_PREFIX);
        builder.push_bind(run_id.as_str().to_string());
        if let Some(cursor) = &cursor {
            // Keyset on (created_at, delivery_id) descending: strictly
            // "before" the cursor row in that same order.
            let cursor_row = sqlx::query(SELECT_BY_ID)
                .bind(cursor.as_str())
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| self.wrap_error(e))?;
            if let Some(cursor_row) = cursor_row {
                let cursor_created_at: DateTime<Utc> = cursor_row
                    .try_get("created_at")
                    .map_err(|e| WebhookDeliveryRepositoryError::Backend { source: e.into() })?;
                builder.push(" AND (created_at < ");
                builder.push_bind(cursor_created_at);
                builder.push(" OR (created_at = ");
                builder.push_bind(cursor_created_at);
                builder.push(" AND delivery_id < ");
                builder.push_bind(cursor.as_str().to_string());
                builder.push("))");
            }
        }
        builder.push(" ORDER BY created_at DESC, delivery_id DESC LIMIT ");
        builder.push_bind(fetch_limit);

        let rows = builder
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;

        let mut items: Vec<WebhookDelivery> = rows
            .iter()
            .map(Self::row_to_delivery)
            .collect::<Result<_, _>>()?;

        let next_cursor = if !unlimited && items.len() > limit as usize {
            items.pop();
            items.last().map(|last| last.delivery_id.clone())
        } else {
            None
        };

        Ok(WebhookDeliveryPage { items, next_cursor })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::webhook::contract_tests;
    use std::sync::Arc;

    async fn fresh_store() -> SqliteWebhookDeliveryRepository {
        SqliteWebhookDeliveryRepository::new("sqlite::memory:")
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn enqueue_creates_and_is_readable() {
        contract_tests::enqueue_creates_and_is_readable(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn get_returns_none_for_unknown() {
        contract_tests::get_returns_none_for_unknown(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn claim_due_transitions_eligible_rows_to_in_flight() {
        contract_tests::claim_due_transitions_eligible_rows_to_in_flight(&fresh_store().await)
            .await;
    }

    #[tokio::test]
    async fn claim_due_respects_limit() {
        contract_tests::claim_due_respects_limit(&fresh_store().await).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn claim_due_race_admits_each_row_once() {
        // Mirrors `run_schedule::sqlite`'s `claim_tick_race_admits_exactly_one`
        // rationale: `sqlite::memory:` is a single private connection, fine
        // for sequential clauses, but a genuine cross-connection race needs
        // a shared on-disk WAL file.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("webhook_delivery_claim_race.db");
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

        let store = SqliteWebhookDeliveryRepository {
            pool,
            database_url: url,
        };
        let repo: Arc<dyn WebhookDeliveryRepositoryPort> = Arc::new(store);
        contract_tests::claim_due_race_admits_each_row_once(repo).await;
    }

    #[tokio::test]
    async fn record_attempt_delivered_sets_terminal_status() {
        contract_tests::record_attempt_delivered_sets_terminal_status(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn record_attempt_retrying_sets_next_attempt_at() {
        contract_tests::record_attempt_retrying_sets_next_attempt_at(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn record_attempt_dead_sets_terminal_status() {
        contract_tests::record_attempt_dead_sets_terminal_status(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn record_attempt_on_unknown_returns_not_found() {
        contract_tests::record_attempt_on_unknown_returns_not_found(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn list_for_run_orders_descending_by_created_at() {
        contract_tests::list_for_run_orders_descending_by_created_at(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn connection_error_redacts_password_from_database_url() {
        let url = "sqlite://user:hunter2-secret@/nonexistent/path/that/does/not/exist.db";
        let err = SqliteWebhookDeliveryRepository::new(url).await.unwrap_err();
        let message = err.to_string();
        assert!(
            !message.contains("hunter2-secret"),
            "connection error leaked the password: {message}"
        );
    }
}
