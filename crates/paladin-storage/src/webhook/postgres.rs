/*
PostgreSQL Webhook Delivery Repository

Concrete `WebhookDeliveryRepositoryPort` implementation over PostgreSQL,
behind the `postgres` feature (PLAT-FR-14/15, D-40, D-43, Tier 2:
Docker-gated, see `docker/docker-compose.test.yml`'s `postgres-test`
service and `make test-integration-docker`). Mirrors `sqlite.rs` exactly --
same five methods, same per-row conditional-`UPDATE` claim -- substituting
`$1, $2, ...` placeholders for `?` and native `TIMESTAMPTZ` for the
timestamp columns. `payload` stays plain TEXT on both backends (D-41: the
exact byte buffer that was signed, not a value to query by shape). Every
statement uses bound parameters; no query string is ever built by
formatting a caller-supplied value into it.
*/

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::postgres::{PgPool, PgPoolOptions, PgRow};

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
     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)";

const SELECT_BY_ID: &str = "SELECT delivery_id, run_id, thread_id, event, url, payload, \
     attempt, max_attempts, status, next_attempt_at, last_response_status, last_error, \
     created_at, updated_at, schema_version FROM webhook_deliveries WHERE delivery_id = $1";

const SELECT_CLAIMABLE_IDS: &str = "SELECT delivery_id FROM webhook_deliveries \
     WHERE status IN ('pending', 'retrying') AND next_attempt_at <= $1 \
     ORDER BY next_attempt_at ASC LIMIT $2";

const CLAIM_ROW: &str = "UPDATE webhook_deliveries SET status = 'in_flight', updated_at = $1 \
     WHERE delivery_id = $2 AND status IN ('pending', 'retrying') AND next_attempt_at <= $3";

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

/// PostgreSQL `WebhookDeliveryRepositoryPort` implementation, behind the
/// `postgres` feature (PLAT-FR-14/15, D-40, Tier 2).
#[derive(Debug)]
pub struct PostgresWebhookDeliveryRepository {
    pool: PgPool,
    /// Kept so every error this store returns can be redacted of the
    /// connection URL's password (mirrors `PostgresAssistantRepository`'s
    /// rationale, T-22-18).
    database_url: String,
}

impl PostgresWebhookDeliveryRepository {
    /// Connect to `database_url` and apply the versioned migration. Safe to
    /// call more than once against the same database: the migration is
    /// idempotent and `sqlx::migrate::Migrator` tracks applied versions.
    pub async fn new(database_url: &str) -> Result<Self, WebhookDeliveryRepositoryError> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
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

    fn wrap(database_url: &str, err: impl std::error::Error) -> WebhookDeliveryRepositoryError {
        let redacted = redact_database_url_password(&err.to_string(), database_url);
        WebhookDeliveryRepositoryError::Backend {
            source: redacted.into(),
        }
    }

    fn wrap_error(&self, err: sqlx::Error) -> WebhookDeliveryRepositoryError {
        Self::wrap(&self.database_url, err)
    }

    fn row_to_delivery(row: &PgRow) -> Result<WebhookDelivery, WebhookDeliveryRepositoryError> {
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
        let attempt: i32 = row.try_get("attempt").map_err(backend_err)?;
        let max_attempts: i32 = row.try_get("max_attempts").map_err(backend_err)?;
        let status_str: String = row.try_get("status").map_err(backend_err)?;
        let status = status_from_str(&status_str)?;
        let next_attempt_at: DateTime<Utc> = row.try_get("next_attempt_at").map_err(backend_err)?;
        let last_response_status: Option<i32> =
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

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("migrations/postgres");

#[async_trait]
impl WebhookDeliveryRepositoryPort for PostgresWebhookDeliveryRepository {
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
            .bind(delivery.attempt as i32)
            .bind(delivery.max_attempts as i32)
            .bind(status_to_str(delivery.status))
            .bind(delivery.next_attempt_at)
            .bind(delivery.last_response_status.map(|v| v as i32))
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

        let mut builder: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
            "UPDATE webhook_deliveries SET attempt = attempt + 1, status = ",
        );
        builder.push_bind(status_to_str(status));
        builder.push(", last_response_status = ");
        builder.push_bind(result.response_status.map(|v| v as i32));
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

        let mut builder: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
            "SELECT delivery_id, run_id, thread_id, event, url, payload, attempt, \
             max_attempts, status, next_attempt_at, last_response_status, last_error, \
             created_at, updated_at, schema_version FROM webhook_deliveries WHERE run_id = ",
        );
        builder.push_bind(run_id.as_str().to_string());

        if let Some(cursor) = &cursor {
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

    // Docker-gated Tier 2 suite (D-51): every test independently probes the
    // shared Postgres service and prints a named `SKIP:` reason then
    // returns early -- never panics or hangs -- when it is not reachable,
    // mirroring `run_schedule::postgres`'s `store_or_skip` gate exactly.

    fn postgres_test_url() -> String {
        std::env::var("STORAGE_POSTGRES_TEST_URL").unwrap_or_else(|_| {
            "postgres://paladin:paladin@localhost:5433/paladin_run_test".to_string()
        })
    }

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

    async fn store_or_skip() -> Option<PostgresWebhookDeliveryRepository> {
        let url = postgres_test_url();
        if !postgres_reachable(&url) {
            println!(
                "SKIP: postgres-test not reachable at {url} -- bring it up with \
                 `docker compose -f docker/docker-compose.test.yml up -d postgres-test`"
            );
            return None;
        }

        match PostgresWebhookDeliveryRepository::new(&url).await {
            Ok(store) => Some(store),
            Err(e) => {
                println!("SKIP: postgres-test connection failed at {url} ({e})");
                None
            }
        }
    }

    #[tokio::test]
    async fn enqueue_creates_and_is_readable() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::enqueue_creates_and_is_readable(&store).await;
    }

    #[tokio::test]
    async fn get_returns_none_for_unknown() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::get_returns_none_for_unknown(&store).await;
    }

    #[tokio::test]
    async fn claim_due_transitions_eligible_rows_to_in_flight() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::claim_due_transitions_eligible_rows_to_in_flight(&store).await;
    }

    #[tokio::test]
    async fn claim_due_respects_limit() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::claim_due_respects_limit(&store).await;
    }

    #[tokio::test]
    async fn claim_due_race_admits_each_row_once() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        let store: Arc<dyn WebhookDeliveryRepositoryPort> = Arc::new(store);
        contract_tests::claim_due_race_admits_each_row_once(store).await;
    }

    #[tokio::test]
    async fn record_attempt_delivered_sets_terminal_status() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::record_attempt_delivered_sets_terminal_status(&store).await;
    }

    #[tokio::test]
    async fn record_attempt_retrying_sets_next_attempt_at() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::record_attempt_retrying_sets_next_attempt_at(&store).await;
    }

    #[tokio::test]
    async fn record_attempt_dead_sets_terminal_status() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::record_attempt_dead_sets_terminal_status(&store).await;
    }

    #[tokio::test]
    async fn record_attempt_on_unknown_returns_not_found() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::record_attempt_on_unknown_returns_not_found(&store).await;
    }

    #[tokio::test]
    async fn list_for_run_orders_descending_by_created_at() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::list_for_run_orders_descending_by_created_at(&store).await;
    }
}
