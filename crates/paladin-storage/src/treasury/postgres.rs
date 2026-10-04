/*
PostgreSQL Treasury Ledger

Concrete `TreasuryLedgerPort` implementation over PostgreSQL, behind the `postgres` feature
(LEDGR-01..04, ADR-0053, D-03, D-04, D-06, D-11, D-12; Tier 2: Docker-gated, see
`docker/docker-compose.test.yml`'s `postgres-test` service and `make test-integration-docker`).
Mirrors `sqlite.rs` exactly -- same five methods, same store-enforced settlement idempotency via
the partial unique index `idx_treasury_ledger_settlement` on `(run_id, superstep, attempt)
WHERE kind = 'settle'` plus `INSERT ... ON CONFLICT ... DO NOTHING` -- substituting `$N` bind
markers for `?`, `::jsonb` casts on write and a `model_breakdown::text` cast on read for the
JSONB `model_breakdown` column, and native `TIMESTAMPTZ` for the two timestamp columns.

Serialization on this backend is a **transaction-scoped advisory lock**, not a lock table
(ADR-0053 §5, D-12): `reserve`, a reserved `settle`, and `release` each open `pool.begin()` and
take `SELECT pg_advisory_xact_lock(hashtext($1)::bigint)` -- keyed on the bound VALUE
`"<tenant_id>/<api_key_id>"`, never SQL text -- BEFORE any balance `SUM` or reservation-closed
state read, and the lock is released only by that transaction's own commit or rollback.
Deliberate limitation: `hashtext` is a 32-bit hash, so two different scopes can collide on the
same lock key and merely serialize against each other -- never a correctness failure (T-39-14,
accepted). An *unreserved* `settle` needs neither a transaction nor the advisory lock: it never
reads a balance, and duplicate-key idempotency is enforced entirely by the partial unique index,
so it runs as a single store-clock read plus one `INSERT ... ON CONFLICT DO NOTHING` on the pool.

Postgres's `SUM(BIGINT)` decodes to `NUMERIC`, which does not fit `i64` -- every balance query
below casts the aggregate `::BIGINT` explicitly so `sqlx::query_scalar::<_, i64>` can decode it.

Every top-level query string below is a plain `&'static str` literal (or, for `spend`'s dynamic
filters, an `sqlx::QueryBuilder` seeded with one) -- never a runtime string-formatting call
building SQL text -- so no caller-supplied value can ever be interpolated into SQL. Every
persisted timestamp this adapter binds is normalised through `crate::run::storage_timestamp`
first (reused, not duplicated); `store_now` reads `SELECT now()` -- the store's own clock, never
`Utc::now()`. Migrations follow the versioned-file convention at
`crates/paladin-storage/migrations/postgres/`, embedded at compile time via `sqlx::migrate!` and
applied automatically on construction.
*/

use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::postgres::{PgPool, PgPoolOptions, Postgres};
use sqlx::{QueryBuilder, Row};
use uuid::Uuid;

use paladin_core::platform::container::allowance::{
    LIFETIME_WINDOW_START, NoticeOutcome, NoticeRecord,
};
use paladin_core::platform::container::cost::{Cost, CurrencyCode};
use paladin_core::platform::container::run::RunId;
use paladin_core::platform::container::treasury_ledger::{
    BalanceQuery, ReservationId, ReserveRequest, SettleOutcome, SettleRequest, SpendGroupBy,
    SpendQuery, SpendRow,
};
use paladin_ports::output::treasury_ledger_port::{TreasuryLedgerError, TreasuryLedgerPort};
use paladin_ports::output::treasury_notice_port::TreasuryNoticePort;

use crate::waypoint::redact::redact_database_url_password;

/// The schema version this adapter stamps on every row it writes (mirrors
/// `sqlite.rs`'s `TREASURY_LEDGER_SCHEMA_VERSION`).
const TREASURY_LEDGER_SCHEMA_VERSION: &str = "v1";

/// The store's own clock (ALLOW-01, ADR-0053 §2).
const STORE_NOW: &str = "SELECT now()";

/// Take a transaction-scoped advisory lock keyed on the bound scope VALUE
/// (`"<tenant_id>/<api_key_id>"`, never SQL text) -- serializes concurrent `reserve`/reserved-
/// `settle`/`release` calls against the same scope inside the same transaction (ADR-0053 §5,
/// D-12). Released only by this transaction's own commit or rollback.
const ADVISORY_LOCK: &str = "SELECT pg_advisory_xact_lock(hashtext($1)::bigint)";

/// The balance SUM inside a `reserve`'s transaction: every row in `scope`+`currency` attributed
/// inside the half-open window (D-03, D-12). `::BIGINT` is required -- `SUM(BIGINT)` decodes to
/// `NUMERIC` otherwise, which does not fit `i64`. Bound in order: tenant_id, api_key_id,
/// currency, window_start, window_end.
const BALANCE_QUERY: &str = "\
    SELECT COALESCE(SUM(amount_nanos), 0)::BIGINT FROM treasury_ledger \
    WHERE tenant_id = $1 AND api_key_id = $2 AND currency = $3 \
      AND attributed_at >= $4 AND attributed_at < $5";

/// The foreign-currency probe inside a `reserve`'s transaction: any row in `scope` attributed
/// inside the window carrying a currency other than the hold's (ADR-0053 §2 -- a `SUM` over
/// mixed currencies is always a refusal, never a conversion). Bound in order: tenant_id,
/// api_key_id, window_start, window_end, currency.
const FOREIGN_CURRENCY_QUERY: &str = "\
    SELECT currency FROM treasury_ledger \
    WHERE tenant_id = $1 AND api_key_id = $2 AND attributed_at >= $3 AND attributed_at < $4 \
      AND currency <> $5 LIMIT 1";

/// `balance`'s foreign-currency probe prefix (D-00h): any row in the tenant (and optional key,
/// optional half-open window) carrying a currency other than the requested one. Extended with
/// `push_bind` only -- never a nullable-parameter OR-predicate, and never a caller value in SQL text.
const BALANCE_FOREIGN_PREFIX: &str = "\
    SELECT currency FROM treasury_ledger WHERE tenant_id = ";

/// `balance`'s `SUM` prefix: the signed contributions of every row in the same scope, in the
/// requested currency (ADR-0053: reserve, settle and release rows alike). `::BIGINT` is
/// required -- `SUM(BIGINT)` decodes to `NUMERIC` otherwise, which does not fit `i64`.
const BALANCE_SUM_PREFIX: &str = "\
    SELECT COALESCE(SUM(amount_nanos), 0)::BIGINT FROM treasury_ledger WHERE tenant_id = ";

/// Push the optional key and half-open window predicates `balance` shares between its two
/// statements, binding every value (timestamps go through `crate::run::storage_timestamp`
/// exactly like `spend` and `reserve`).
fn push_balance_scope(builder: &mut QueryBuilder<'_, Postgres>, query: &BalanceQuery) {
    if let Some(api_key_id) = &query.api_key_id {
        builder.push(" AND api_key_id = ");
        builder.push_bind(api_key_id.clone());
    }
    if let Some(since) = query.since {
        builder.push(" AND attributed_at >= ");
        builder.push_bind(crate::run::storage_timestamp(since));
    }
    if let Some(until) = query.until {
        builder.push(" AND attributed_at < ");
        builder.push_bind(crate::run::storage_timestamp(until));
    }
}

/// A `reserve` row: `amount_nanos = +hold`, `charged_nanos = 0`, an empty `model_breakdown`
/// (D-06 -- reserve rows are not idempotency-keyed, so `run_id`/`superstep`/`attempt` may be
/// `NULL`). Bound in order: entry_id, tenant_id, api_key_id, reservation_id, run_id, superstep,
/// attempt, amount_nanos, currency, attributed_at, recorded_at, schema_version.
const RESERVE_INSERT: &str = "\
    INSERT INTO treasury_ledger \
      (entry_id, kind, tenant_id, api_key_id, reservation_id, run_id, superstep, attempt, \
       amount_nanos, charged_nanos, currency, model_breakdown, attributed_at, recorded_at, \
       schema_version) \
    VALUES ($1, 'reserve', $2, $3, $4, $5, $6, $7, $8, 0, $9, '{}'::jsonb, $10, $11, $12)";

// The `WHERE kind = 'settle'` arbiter predicate below MUST textually match
// `007_create_treasury_ledger_table.sql`'s `idx_treasury_ledger_settlement` index predicate, or
// Postgres cannot infer that partial index as the conflict target (Pitfall 3;
// `settle_arbiter_predicate_matches_the_migration` proves this stays true).
/// A `settle` row: `amount_nanos = actual - outstanding_hold`, `charged_nanos = actual`. Bound in
/// order: entry_id, tenant_id, api_key_id, reservation_id, run_id, superstep, attempt,
/// amount_nanos, charged_nanos, currency, model_breakdown, attributed_at, recorded_at,
/// schema_version.
const SETTLE_INSERT: &str = "\
    INSERT INTO treasury_ledger \
      (entry_id, kind, tenant_id, api_key_id, reservation_id, run_id, superstep, attempt, \
       amount_nanos, charged_nanos, currency, model_breakdown, attributed_at, recorded_at, \
       schema_version) \
    VALUES ($1, 'settle', $2, $3, $4, $5, $6, $7, $8, $9, $10, $11::jsonb, $12, $13, $14) \
    ON CONFLICT (run_id, superstep, attempt) WHERE kind = 'settle' DO NOTHING";

/// The `kind = 'reserve'` row a `settle` or `release` resolves its `reservation_id` against.
const SELECT_RESERVATION: &str = "\
    SELECT tenant_id, api_key_id, currency, amount_nanos, attributed_at \
    FROM treasury_ledger WHERE kind = 'reserve' AND reservation_id = $1";

/// Whether a reservation is already closed (an earlier `settle` or `release` referencing it
/// already exists): `0` means still open (the full hold is outstanding), `> 0` means closed
/// (the outstanding hold is `0`).
const RESERVATION_CLOSED_QUERY: &str = "\
    SELECT COUNT(*) FROM treasury_ledger WHERE reservation_id = $1 AND kind IN ('settle', 'release')";

/// A `release` row: `amount_nanos = -hold`, `charged_nanos = 0`, an empty `model_breakdown`.
/// Bound in order: entry_id, tenant_id, api_key_id, reservation_id, amount_nanos, currency,
/// attributed_at, recorded_at, schema_version.
const RELEASE_INSERT: &str = "\
    INSERT INTO treasury_ledger \
      (entry_id, kind, tenant_id, api_key_id, reservation_id, run_id, superstep, attempt, \
       amount_nanos, charged_nanos, currency, model_breakdown, attributed_at, recorded_at, \
       schema_version) \
    VALUES ($1, 'release', $2, $3, $4, NULL, NULL, NULL, $5, 0, $6, '{}'::jsonb, $7, $8, $9)";

/// Every settle row's columns `spend` needs, pre-filtered to `kind = 'settle'` so reserve/release
/// rows never reach the fold. `model_breakdown::text` casts the JSONB column to a plain string so
/// the same `serde_json::from_str` fold as the SQLite adapter applies unmodified.
const SPEND_SELECT_PREFIX: &str = "\
    SELECT tenant_id, api_key_id, run_id, currency, charged_nanos, model_breakdown::text \
    FROM treasury_ledger WHERE kind = 'settle'";

// The `ON CONFLICT (...)` column list below MUST textually match
// `011_create_treasury_notices.sql`'s `idx_treasury_notices_once` index column list, or Postgres
// cannot infer that unique index as the conflict target (Pitfall 3;
// `notice_arbiter_matches_the_migration` proves this stays true). `api_key_id` is bound as `''`
// for tenant scope and `window_start` as the epoch for a lifetime notice (C5): no key column is
// ever NULL. Bound in order: notice_id, scope_kind, tenant_id, api_key_id, limit_kind,
// window_start, window_end, ceiling_nanos, currency, balance_nanos, warn_at, run_id,
// recorded_at, schema_version.
const NOTICE_INSERT: &str = "\
    INSERT INTO treasury_notices \
      (notice_id, scope_kind, tenant_id, api_key_id, limit_kind, window_start, window_end, \
       ceiling_nanos, currency, balance_nanos, warn_at, run_id, recorded_at, schema_version) \
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) \
    ON CONFLICT (scope_kind, tenant_id, api_key_id, limit_kind, window_start, ceiling_nanos) \
    DO NOTHING";

/// Every notice recorded for one admitting run, oldest first. Bound: run_id.
const NOTICES_FOR_RUN: &str = "\
    SELECT notice_id, scope_kind, tenant_id, api_key_id, limit_kind, window_start, window_end, \
           ceiling_nanos, currency, balance_nanos, warn_at, run_id, recorded_at \
    FROM treasury_notices WHERE run_id = $1 ORDER BY recorded_at ASC, notice_id ASC";

/// Delete an abandoned admission's own notices by id in one statement. Bound: notice_ids
/// (a `TEXT[]`).
const NOTICE_DELETE: &str = "DELETE FROM treasury_notices WHERE notice_id = ANY($1)";

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("migrations/postgres");

/// PostgreSQL `TreasuryLedgerPort` implementation, behind the `postgres` feature (LEDGR-01,
/// Tier 2: Docker-gated).
#[derive(Debug)]
pub struct PostgresTreasuryLedger {
    pool: PgPool,
    /// Kept so every error this store returns can be redacted of the connection URL's password,
    /// not just construction-time connection errors -- mirrors `PostgresRunRepository`'s
    /// rationale (T-22-18).
    database_url: String,
}

impl PostgresTreasuryLedger {
    /// Connect to `database_url` and apply the versioned migration. Safe to call more than once
    /// against the same database: the migration is idempotent (`CREATE TABLE IF NOT EXISTS`/
    /// `CREATE INDEX IF NOT EXISTS`) and `sqlx::migrate::Migrator` itself tracks applied
    /// versions.
    ///
    /// # Errors
    ///
    /// Returns [`TreasuryLedgerError::Backend`] (connection URL password redacted first) if the
    /// connection cannot be opened or the migration cannot be applied.
    pub async fn new(database_url: &str) -> Result<Self, TreasuryLedgerError> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            // A genuinely unreachable server (this Tier 2 suite's local-skip case) surfaces as a
            // fast, clearly-diagnosed error rather than a slow hang absorbing sqlx's default 30s
            // acquire timeout (mirrors `PostgresRunRepository::new`'s rationale).
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

    /// Wrap a driver/migration error into `TreasuryLedgerError::Backend`, with the connection
    /// URL's password redacted from the error text first (redact before any truncation, per this
    /// project's security instructions).
    fn wrap(database_url: &str, err: impl std::error::Error) -> TreasuryLedgerError {
        let redacted = redact_database_url_password(&err.to_string(), database_url);
        TreasuryLedgerError::Backend {
            source: redacted.into(),
        }
    }

    fn wrap_error(&self, err: sqlx::Error) -> TreasuryLedgerError {
        Self::wrap(&self.database_url, err)
    }
}

/// The advisory lock key: a bound VALUE (never SQL text) uniquely identifying a scope, fed to
/// `hashtext(...)::bigint` in [`ADVISORY_LOCK`]. A 32-bit hash collision between two different
/// scopes only serializes them against each other -- never a correctness failure (T-39-14).
fn scope_lock_key(tenant_id: &str, api_key_id: &str) -> String {
    format!("{tenant_id}/{api_key_id}")
}

#[async_trait]
impl TreasuryLedgerPort for PostgresTreasuryLedger {
    async fn reserve(&self, request: ReserveRequest) -> Result<ReservationId, TreasuryLedgerError> {
        crate::treasury::validate_reserve(&request)?;

        let window_start = crate::run::storage_timestamp(request.window_start);
        let window_end = crate::run::storage_timestamp(request.window_end);

        // Resolve the (optional) settlement key components before opening the transaction --
        // pure computation, nothing that needs the advisory lock.
        let (run_id, superstep, attempt) = match &request.key {
            Some(key) => {
                let superstep = i64::try_from(key.superstep).map_err(|_| {
                    TreasuryLedgerError::InvalidRequest {
                        message: format!(
                            "superstep {} does not fit in a 64-bit signed integer",
                            key.superstep
                        ),
                    }
                })?;
                (
                    Some(key.run_id.as_str().to_string()),
                    Some(superstep),
                    Some(i64::from(key.attempt)),
                )
            }
            None => (None, None, None),
        };

        let mut tx = self.pool.begin().await.map_err(|e| self.wrap_error(e))?;

        // The advisory lock is taken BEFORE any SUM or state read (D-12, must-have truth).
        sqlx::query(ADVISORY_LOCK)
            .bind(scope_lock_key(
                &request.scope.tenant_id,
                &request.scope.api_key_id,
            ))
            .execute(&mut *tx)
            .await
            .map_err(|e| self.wrap_error(e))?;

        let foreign: Option<String> = sqlx::query_scalar(FOREIGN_CURRENCY_QUERY)
            .bind(&request.scope.tenant_id)
            .bind(&request.scope.api_key_id)
            .bind(window_start)
            .bind(window_end)
            .bind(request.hold.currency().as_str())
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| self.wrap_error(e))?;

        if let Some(found) = foreign {
            tx.rollback().await.map_err(|e| self.wrap_error(e))?;
            let found_currency =
                CurrencyCode::new(&found).map_err(|e| TreasuryLedgerError::Serialization {
                    message: format!("stored currency '{found}' is invalid: {e}"),
                })?;
            return Err(TreasuryLedgerError::CurrencyMismatch {
                expected: request.hold.currency().clone(),
                found: found_currency,
            });
        }

        let balance: i64 = sqlx::query_scalar(BALANCE_QUERY)
            .bind(&request.scope.tenant_id)
            .bind(&request.scope.api_key_id)
            .bind(request.hold.currency().as_str())
            .bind(window_start)
            .bind(window_end)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| self.wrap_error(e))?;

        let hold_nanos = request.hold.nanos();
        let ceiling_nanos = request.ceiling.nanos();
        let admitted = balance
            .checked_add(hold_nanos)
            .is_some_and(|sum| sum <= ceiling_nanos);

        if !admitted {
            tx.rollback().await.map_err(|e| self.wrap_error(e))?;
            return Err(TreasuryLedgerError::Refused {
                balance: Cost::new(balance, request.hold.currency().clone()),
                hold: request.hold.clone(),
                ceiling: request.ceiling.clone(),
            });
        }

        let now: DateTime<Utc> = sqlx::query_scalar(STORE_NOW)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| self.wrap_error(e))?;
        let attributed_at = crate::run::storage_timestamp(now);

        let reservation_id = ReservationId::new_v7();
        let entry_id = Uuid::now_v7().to_string();

        sqlx::query(RESERVE_INSERT)
            .bind(entry_id)
            .bind(&request.scope.tenant_id)
            .bind(&request.scope.api_key_id)
            .bind(reservation_id.as_str())
            .bind(run_id)
            .bind(superstep)
            .bind(attempt)
            .bind(hold_nanos)
            .bind(request.hold.currency().as_str())
            .bind(attributed_at)
            .bind(attributed_at)
            .bind(TREASURY_LEDGER_SCHEMA_VERSION)
            .execute(&mut *tx)
            .await
            .map_err(|e| self.wrap_error(e))?;

        tx.commit().await.map_err(|e| self.wrap_error(e))?;

        Ok(reservation_id)
    }

    async fn release(&self, reservation: ReservationId) -> Result<(), TreasuryLedgerError> {
        // Read the reservation on the pool first to learn its scope (D-12's "read on pool, then
        // begin() and take the scope's advisory lock" sequence).
        let Some(row) = sqlx::query(SELECT_RESERVATION)
            .bind(reservation.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?
        else {
            return Err(TreasuryLedgerError::UnknownReservation { reservation });
        };

        let tenant_id: String = row.try_get("tenant_id").map_err(|e| self.wrap_error(e))?;
        let api_key_id: String = row.try_get("api_key_id").map_err(|e| self.wrap_error(e))?;
        let currency: String = row.try_get("currency").map_err(|e| self.wrap_error(e))?;
        let amount_nanos: i64 = row
            .try_get("amount_nanos")
            .map_err(|e| self.wrap_error(e))?;
        let attributed_at: DateTime<Utc> = row
            .try_get("attributed_at")
            .map_err(|e| self.wrap_error(e))?;

        let mut tx = self.pool.begin().await.map_err(|e| self.wrap_error(e))?;

        sqlx::query(ADVISORY_LOCK)
            .bind(scope_lock_key(&tenant_id, &api_key_id))
            .execute(&mut *tx)
            .await
            .map_err(|e| self.wrap_error(e))?;

        let closed_count: i64 = sqlx::query_scalar(RESERVATION_CLOSED_QUERY)
            .bind(reservation.as_str())
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| self.wrap_error(e))?;

        if closed_count > 0 {
            // Already settled or already released: a no-op, not an error (D-04).
            tx.commit().await.map_err(|e| self.wrap_error(e))?;
            return Ok(());
        }

        let now: DateTime<Utc> = sqlx::query_scalar(STORE_NOW)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| self.wrap_error(e))?;
        let recorded_at = crate::run::storage_timestamp(now);

        sqlx::query(RELEASE_INSERT)
            .bind(Uuid::now_v7().to_string())
            .bind(&tenant_id)
            .bind(&api_key_id)
            .bind(reservation.as_str())
            .bind(-amount_nanos)
            .bind(&currency)
            .bind(attributed_at)
            .bind(recorded_at)
            .bind(TREASURY_LEDGER_SCHEMA_VERSION)
            .execute(&mut *tx)
            .await
            .map_err(|e| self.wrap_error(e))?;

        tx.commit().await.map_err(|e| self.wrap_error(e))?;
        Ok(())
    }

    async fn settle(&self, request: SettleRequest) -> Result<SettleOutcome, TreasuryLedgerError> {
        crate::treasury::validate_settle(&request)?;

        let superstep = i64::try_from(request.key.superstep).map_err(|_| {
            TreasuryLedgerError::InvalidRequest {
                message: format!(
                    "superstep {} does not fit in a 64-bit signed integer",
                    request.key.superstep
                ),
            }
        })?;
        let attempt = i64::from(request.key.attempt);
        let model_breakdown_json =
            serde_json::to_string(&request.model_breakdown).map_err(|e| {
                TreasuryLedgerError::Serialization {
                    message: format!("model_breakdown could not be serialized: {e}"),
                }
            })?;
        let entry_id = Uuid::now_v7().to_string();
        let actual_nanos = request.amount.nanos();

        if let Some(reservation) = &request.reservation {
            let Some(row) = sqlx::query(SELECT_RESERVATION)
                .bind(reservation.as_str())
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| self.wrap_error(e))?
            else {
                return Err(TreasuryLedgerError::UnknownReservation {
                    reservation: reservation.clone(),
                });
            };

            let tenant_id: String = row.try_get("tenant_id").map_err(|e| self.wrap_error(e))?;
            let api_key_id: String = row.try_get("api_key_id").map_err(|e| self.wrap_error(e))?;
            let currency: String = row.try_get("currency").map_err(|e| self.wrap_error(e))?;
            let hold_nanos: i64 = row
                .try_get("amount_nanos")
                .map_err(|e| self.wrap_error(e))?;
            let reservation_attributed_at: DateTime<Utc> = row
                .try_get("attributed_at")
                .map_err(|e| self.wrap_error(e))?;

            if tenant_id != request.scope.tenant_id || api_key_id != request.scope.api_key_id {
                return Err(TreasuryLedgerError::InvalidRequest {
                    message: format!(
                        "settle scope {:?}/{:?} does not match reservation {reservation}'s \
                             scope {tenant_id:?}/{api_key_id:?}",
                        request.scope.tenant_id, request.scope.api_key_id
                    ),
                });
            }
            if currency != request.amount.currency().as_str() {
                let expected = CurrencyCode::new(&currency).map_err(|e| {
                    TreasuryLedgerError::Serialization {
                        message: format!("stored currency '{currency}' is invalid: {e}"),
                    }
                })?;
                return Err(TreasuryLedgerError::CurrencyMismatch {
                    expected,
                    found: request.amount.currency().clone(),
                });
            }

            let mut tx = self.pool.begin().await.map_err(|e| self.wrap_error(e))?;

            sqlx::query(ADVISORY_LOCK)
                .bind(scope_lock_key(&tenant_id, &api_key_id))
                .execute(&mut *tx)
                .await
                .map_err(|e| self.wrap_error(e))?;

            let closed_count: i64 = sqlx::query_scalar(RESERVATION_CLOSED_QUERY)
                .bind(reservation.as_str())
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| self.wrap_error(e))?;
            let outstanding_hold = if closed_count > 0 { 0 } else { hold_nanos };

            let now: DateTime<Utc> = sqlx::query_scalar(STORE_NOW)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| self.wrap_error(e))?;
            let recorded_at = crate::run::storage_timestamp(now);

            let result = sqlx::query(SETTLE_INSERT)
                .bind(entry_id)
                .bind(&request.scope.tenant_id)
                .bind(&request.scope.api_key_id)
                .bind(Some(reservation.as_str()))
                .bind(request.key.run_id.as_str())
                .bind(superstep)
                .bind(attempt)
                .bind(actual_nanos - outstanding_hold)
                .bind(actual_nanos)
                .bind(request.amount.currency().as_str())
                .bind(model_breakdown_json)
                .bind(reservation_attributed_at)
                .bind(recorded_at)
                .bind(TREASURY_LEDGER_SCHEMA_VERSION)
                .execute(&mut *tx)
                .await
                .map_err(|e| self.wrap_error(e))?;

            tx.commit().await.map_err(|e| self.wrap_error(e))?;

            return Ok(if result.rows_affected() == 0 {
                SettleOutcome::AlreadySettled
            } else {
                SettleOutcome::Settled
            });
        }

        // Unreserved settle: no balance to check, so no transaction and no advisory lock are
        // needed -- idempotency is enforced entirely by the partial unique index (D-04, D-06).
        let now: DateTime<Utc> = sqlx::query_scalar(STORE_NOW)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;
        let attributed_at = crate::run::storage_timestamp(now);

        let result = sqlx::query(SETTLE_INSERT)
            .bind(entry_id)
            .bind(&request.scope.tenant_id)
            .bind(&request.scope.api_key_id)
            .bind(Option::<&str>::None)
            .bind(request.key.run_id.as_str())
            .bind(superstep)
            .bind(attempt)
            .bind(actual_nanos)
            .bind(actual_nanos)
            .bind(request.amount.currency().as_str())
            .bind(model_breakdown_json)
            .bind(attributed_at)
            .bind(attributed_at)
            .bind(TREASURY_LEDGER_SCHEMA_VERSION)
            .execute(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;

        Ok(if result.rows_affected() == 0 {
            SettleOutcome::AlreadySettled
        } else {
            SettleOutcome::Settled
        })
    }

    async fn spend(&self, query: SpendQuery) -> Result<Vec<SpendRow>, TreasuryLedgerError> {
        let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(SPEND_SELECT_PREFIX);

        if let Some(since) = query.since {
            builder.push(" AND attributed_at >= ");
            builder.push_bind(crate::run::storage_timestamp(since));
        }
        if let Some(until) = query.until {
            builder.push(" AND attributed_at < ");
            builder.push_bind(crate::run::storage_timestamp(until));
        }
        if let Some(tenant_id) = &query.tenant_id {
            builder.push(" AND tenant_id = ");
            builder.push_bind(tenant_id.clone());
        }
        if let Some(api_key_id) = &query.api_key_id {
            builder.push(" AND api_key_id = ");
            builder.push_bind(api_key_id.clone());
        }
        if !query.run_ids.is_empty() {
            let run_ids: Vec<String> = query
                .run_ids
                .iter()
                .map(|run_id| run_id.as_str().to_string())
                .collect();
            builder.push(" AND run_id = ANY(");
            builder.push_bind(run_ids);
            builder.push(")");
        }

        let rows = builder
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;

        // Keyed by (group value, currency code) -- never combining two currencies into one
        // figure (D-09). The u64 is the settlement count folded into this (group, currency).
        let mut folded: BTreeMap<(String, String), (i64, u64)> = BTreeMap::new();

        for row in &rows {
            let tenant_id: String = row.try_get("tenant_id").map_err(|e| self.wrap_error(e))?;
            let api_key_id: String = row.try_get("api_key_id").map_err(|e| self.wrap_error(e))?;
            let run_id: String = row.try_get("run_id").map_err(|e| self.wrap_error(e))?;
            let currency: String = row.try_get("currency").map_err(|e| self.wrap_error(e))?;
            let charged_nanos: i64 = row
                .try_get("charged_nanos")
                .map_err(|e| self.wrap_error(e))?;
            let model_breakdown_json: String = row
                .try_get("model_breakdown")
                .map_err(|e| self.wrap_error(e))?;

            CurrencyCode::new(&currency).map_err(|e| TreasuryLedgerError::Serialization {
                message: format!("stored currency '{currency}' is invalid: {e}"),
            })?;

            match query.group_by {
                SpendGroupBy::Tenant => fold_one(&mut folded, tenant_id, currency, charged_nanos),
                SpendGroupBy::ApiKey => fold_one(&mut folded, api_key_id, currency, charged_nanos),
                SpendGroupBy::Run => fold_one(&mut folded, run_id, currency, charged_nanos),
                SpendGroupBy::Model => {
                    let breakdown: BTreeMap<String, i64> =
                        serde_json::from_str(&model_breakdown_json).map_err(|e| {
                            TreasuryLedgerError::Serialization {
                                message: format!("stored model_breakdown is invalid JSON: {e}"),
                            }
                        })?;
                    for (model, nanos) in breakdown {
                        fold_one(&mut folded, model, currency.clone(), nanos);
                    }
                }
            }
        }

        folded
            .into_iter()
            .map(|((group, currency), (nanos, settlements))| {
                let currency = CurrencyCode::new(&currency).map_err(|e| {
                    TreasuryLedgerError::Serialization {
                        message: format!("stored currency '{currency}' is invalid: {e}"),
                    }
                })?;
                Ok(SpendRow {
                    group,
                    amount: Cost::new(nanos, currency),
                    settlements,
                })
            })
            .collect()
    }

    async fn store_now(&self) -> Result<DateTime<Utc>, TreasuryLedgerError> {
        sqlx::query_scalar(STORE_NOW)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))
    }

    async fn balance(&self, query: BalanceQuery) -> Result<Cost, TreasuryLedgerError> {
        crate::treasury::validate_balance(&query)?;

        // Both statements run inside one read transaction (no advisory lock: this is a read
        // and must never serialize against writers -- admission is check-only, D-05).
        let mut tx = self.pool.begin().await.map_err(|e| self.wrap_error(e))?;

        let mut probe: QueryBuilder<Postgres> = QueryBuilder::new(BALANCE_FOREIGN_PREFIX);
        probe.push_bind(query.tenant_id.clone());
        push_balance_scope(&mut probe, &query);
        probe.push(" AND currency <> ");
        probe.push_bind(query.currency.as_str().to_string());
        probe.push(" LIMIT 1");
        let foreign: Option<String> = probe
            .build_query_scalar()
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| self.wrap_error(e))?;

        if let Some(found) = foreign {
            tx.rollback().await.map_err(|e| self.wrap_error(e))?;
            let found_currency =
                CurrencyCode::new(&found).map_err(|e| TreasuryLedgerError::Serialization {
                    message: format!("stored currency '{found}' is invalid: {e}"),
                })?;
            return Err(TreasuryLedgerError::CurrencyMismatch {
                expected: query.currency,
                found: found_currency,
            });
        }

        let mut sum: QueryBuilder<Postgres> = QueryBuilder::new(BALANCE_SUM_PREFIX);
        sum.push_bind(query.tenant_id.clone());
        push_balance_scope(&mut sum, &query);
        sum.push(" AND currency = ");
        sum.push_bind(query.currency.as_str().to_string());
        let total: i64 = sum
            .build_query_scalar()
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| self.wrap_error(e))?;

        tx.commit().await.map_err(|e| self.wrap_error(e))?;

        Ok(Cost::new(total, query.currency))
    }
}

/// Fold one (group, currency, nanos) contribution into `folded`, incrementing that entry's
/// settlement count by one. Duplicated from `sqlite.rs` rather than shared, mirroring the house
/// convention of one self-contained adapter file per backend.
fn fold_one(
    folded: &mut BTreeMap<(String, String), (i64, u64)>,
    group: String,
    currency: String,
    nanos: i64,
) {
    let entry = folded.entry((group, currency)).or_insert((0, 0));
    entry.0 = entry.0.saturating_add(nanos);
    entry.1 += 1;
}

#[async_trait]
impl TreasuryNoticePort for PostgresTreasuryLedger {
    async fn record(&self, notice: &NoticeRecord) -> Result<NoticeOutcome, TreasuryLedgerError> {
        crate::treasury::validate_notice(notice)?;

        // One statement, so no transaction: the unique index arbitrates every concurrent claim
        // across every replica (D-16).
        let window_start = crate::run::storage_timestamp(
            notice.warning.window_start.unwrap_or(LIFETIME_WINDOW_START),
        );
        let window_end = notice.warning.window_end.map(crate::run::storage_timestamp);
        let result = sqlx::query(NOTICE_INSERT)
            .bind(&notice.notice_id)
            .bind(notice.warning.scope_kind.as_str())
            .bind(&notice.tenant_id)
            .bind(notice.api_key_id.as_deref().unwrap_or(""))
            .bind(notice.warning.limit_kind.as_str())
            .bind(window_start)
            .bind(window_end)
            .bind(notice.warning.ceiling.nanos())
            .bind(notice.warning.ceiling.currency().as_str())
            .bind(notice.warning.balance.nanos())
            .bind(i32::from(notice.warning.warn_at))
            .bind(notice.run_id.as_ref().map(RunId::as_str))
            .bind(crate::run::storage_timestamp(notice.recorded_at))
            .bind(TREASURY_LEDGER_SCHEMA_VERSION)
            .execute(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;

        Ok(if result.rows_affected() == 0 {
            NoticeOutcome::AlreadyRecorded
        } else {
            NoticeOutcome::Recorded
        })
    }

    async fn notices_for_run(
        &self,
        run_id: &RunId,
    ) -> Result<Vec<NoticeRecord>, TreasuryLedgerError> {
        let rows = sqlx::query(NOTICES_FOR_RUN)
            .bind(run_id.as_str())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;

        rows.iter()
            .map(|row| {
                let warn_at: i32 = row.try_get("warn_at").map_err(|e| self.wrap_error(e))?;
                crate::treasury::RawNotice {
                    notice_id: row.try_get("notice_id").map_err(|e| self.wrap_error(e))?,
                    scope_kind: row.try_get("scope_kind").map_err(|e| self.wrap_error(e))?,
                    tenant_id: row.try_get("tenant_id").map_err(|e| self.wrap_error(e))?,
                    api_key_id: row.try_get("api_key_id").map_err(|e| self.wrap_error(e))?,
                    limit_kind: row.try_get("limit_kind").map_err(|e| self.wrap_error(e))?,
                    window_start: row
                        .try_get("window_start")
                        .map_err(|e| self.wrap_error(e))?,
                    window_end: row.try_get("window_end").map_err(|e| self.wrap_error(e))?,
                    ceiling_nanos: row
                        .try_get("ceiling_nanos")
                        .map_err(|e| self.wrap_error(e))?,
                    currency: row.try_get("currency").map_err(|e| self.wrap_error(e))?,
                    balance_nanos: row
                        .try_get("balance_nanos")
                        .map_err(|e| self.wrap_error(e))?,
                    warn_at: i64::from(warn_at),
                    run_id: row.try_get("run_id").map_err(|e| self.wrap_error(e))?,
                    recorded_at: row.try_get("recorded_at").map_err(|e| self.wrap_error(e))?,
                }
                .into_record()
            })
            .collect()
    }

    async fn discard(&self, notice_ids: &[String]) -> Result<(), TreasuryLedgerError> {
        if notice_ids.is_empty() {
            return Ok(());
        }
        sqlx::query(NOTICE_DELETE)
            .bind(notice_ids)
            .execute(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use crate::treasury::contract_tests;
    use crate::treasury::notice_contract_tests as notices;

    // Docker-gated Tier 2 suite (D-51): every test independently probes the shared Postgres
    // service and prints a named `SKIP:` reason then returns early -- never panics or hangs --
    // when it is not reachable, mirroring `run::postgres`/`waypoint::postgres`'s `store_or_skip`
    // gate exactly. This whole module is ALSO compile-time gated behind the `postgres` feature
    // (see `treasury/mod.rs`), which is not in any default feature set.
    //
    // `STORAGE_POSTGRES_TEST_URL` is the storage-wide variable name every `*::postgres` suite in
    // this crate reads (D-00e); CI's `postgres-integration` job's `--lib postgres` filter picks
    // up this module automatically with no workflow edit.
    //
    // Bring the service up before running this suite:
    // ```sh
    // docker compose -f docker/docker-compose.test.yml up -d postgres-test
    // STORAGE_POSTGRES_TEST_URL=postgres://... \
    //   cargo test -p paladin-storage --features postgres --lib treasury::postgres
    // ```

    fn postgres_test_url() -> String {
        std::env::var("STORAGE_POSTGRES_TEST_URL").unwrap_or_else(|_| {
            "postgres://paladin:paladin@localhost:5433/paladin_run_test".to_string()
        })
    }

    /// A cheap, short-timeout TCP reachability probe, tried BEFORE handing `url` to `sqlx`'s
    /// pool -- mirrors `run::postgres`/`waypoint::postgres`'s identical helper and its rationale
    /// (a connection refusal is retryable and can otherwise absorb the whole `acquire_timeout`
    /// budget).
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

    /// Returns a connected, migrated store, or `None` (after printing a named reason) if
    /// `postgres-test` is not reachable.
    async fn store_or_skip() -> Option<PostgresTreasuryLedger> {
        let url = postgres_test_url();
        if !postgres_reachable(&url) {
            println!(
                "SKIP: postgres-test not reachable at {url} -- bring it up with \
                 `docker compose -f docker/docker-compose.test.yml up -d postgres-test`"
            );
            return None;
        }

        match PostgresTreasuryLedger::new(&url).await {
            Ok(store) => Some(store),
            Err(e) => {
                println!("SKIP: postgres-test connection failed at {url} ({e})");
                None
            }
        }
    }

    // One #[tokio::test] per shared contract clause (D-11), written out explicitly (not via a
    // macro) so each names the violated contract clause on failure, mirroring `sqlite.rs`'s and
    // `run::postgres`'s test modules exactly. Do not edit any clause here -- if one fails on
    // Postgres, fix the adapter (Task 2's own instruction).

    #[tokio::test(flavor = "multi_thread")]
    async fn reserve_race_admits_exactly_n_minus_one() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        let store: Arc<dyn TreasuryLedgerPort> = Arc::new(store);
        contract_tests::reserve_race_admits_exactly_n_minus_one(store).await;
    }

    #[tokio::test]
    async fn reserve_admits_at_the_ceiling_and_refuses_one_past_it() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::reserve_admits_at_the_ceiling_and_refuses_one_past_it(&store).await;
    }

    #[tokio::test]
    async fn reserve_then_settle_contributes_actual_minus_hold() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::reserve_then_settle_contributes_actual_minus_hold(&store).await;
    }

    #[tokio::test]
    async fn unreserved_settle_contributes_actual() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::unreserved_settle_contributes_actual(&store).await;
    }

    #[tokio::test]
    async fn release_returns_the_hold_and_is_idempotent() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::release_returns_the_hold_and_is_idempotent(&store).await;
    }

    #[tokio::test]
    async fn settle_after_release_charges_actual_only() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::settle_after_release_charges_actual_only(&store).await;
    }

    #[tokio::test]
    async fn settle_is_attributed_to_its_reservation_window() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::settle_is_attributed_to_its_reservation_window(&store).await;
    }

    #[tokio::test]
    async fn reserve_refuses_a_second_currency_in_scope_and_window() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::reserve_refuses_a_second_currency_in_scope_and_window(&store).await;
    }

    #[tokio::test]
    async fn reserve_rejects_invalid_requests() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::reserve_rejects_invalid_requests(&store).await;
    }

    #[tokio::test]
    async fn two_reservations_for_one_superstep_attempt_are_legal() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::two_reservations_for_one_superstep_attempt_are_legal(&store).await;
    }

    #[tokio::test]
    async fn duplicate_settle_is_already_settled_and_charges_once() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::duplicate_settle_is_already_settled_and_charges_once(&store).await;
    }

    #[tokio::test]
    async fn bumped_attempt_is_a_distinct_settlement() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::bumped_attempt_is_a_distinct_settlement(&store).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn concurrent_duplicate_settles_charge_once() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        let store: Arc<dyn TreasuryLedgerPort> = Arc::new(store);
        contract_tests::concurrent_duplicate_settles_charge_once(store).await;
    }

    #[tokio::test]
    async fn spend_groups_by_every_dimension_over_a_window() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::spend_groups_by_every_dimension_over_a_window(&store).await;
    }

    #[tokio::test]
    async fn spend_orders_groups_then_currencies_ascending() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::spend_orders_groups_then_currencies_ascending(&store).await;
    }

    #[tokio::test]
    async fn spend_over_an_empty_window_is_empty() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::spend_over_an_empty_window_is_empty(&store).await;
    }

    #[tokio::test]
    async fn spend_window_is_half_open() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::spend_window_is_half_open(&store).await;
    }

    #[tokio::test]
    async fn spend_splits_currencies_into_separate_rows() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::spend_splits_currencies_into_separate_rows(&store).await;
    }

    #[tokio::test]
    async fn settle_rejects_a_breakdown_that_does_not_sum_to_the_amount() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::settle_rejects_a_breakdown_that_does_not_sum_to_the_amount(&store).await;
    }

    #[tokio::test]
    async fn store_now_is_non_decreasing() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::store_now_is_non_decreasing(&store).await;
    }

    #[tokio::test]
    async fn unattributed_scope_is_grouped_under_the_sentinel() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::unattributed_scope_is_grouped_under_the_sentinel(&store).await;
    }

    // ── Phase 41 (41-01, 41-02) balance clauses ───────────────────────────

    #[tokio::test]
    async fn tenant_balance_equals_sum_of_key_balances() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::tenant_balance_equals_sum_of_key_balances(&store).await;
    }

    #[tokio::test]
    async fn balance_sums_signed_contributions_in_window() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::balance_sums_signed_contributions_in_window(&store).await;
    }

    #[tokio::test]
    async fn key_balance_excludes_other_keys_and_tenants() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::key_balance_excludes_other_keys_and_tenants(&store).await;
    }

    #[tokio::test]
    async fn balance_window_is_half_open() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::balance_window_is_half_open(&store).await;
    }

    #[tokio::test]
    async fn balance_unbounded_counts_every_row() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::balance_unbounded_counts_every_row(&store).await;
    }

    #[tokio::test]
    async fn balance_mixed_currency_is_currency_mismatch() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::balance_mixed_currency_is_currency_mismatch(&store).await;
    }

    #[tokio::test]
    async fn balance_of_empty_scope_is_zero_in_requested_currency() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::balance_of_empty_scope_is_zero_in_requested_currency(&store).await;
    }

    #[tokio::test]
    async fn balance_rejects_an_invalid_query() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::balance_rejects_an_invalid_query(&store).await;
    }

    #[tokio::test]
    async fn balance_is_read_only() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::balance_is_read_only(&store).await;
    }

    // ── Notice clauses (ALLOW-04, D-16, 41-06) ───────────────────────────

    #[tokio::test]
    async fn first_claim_wins_duplicate_is_already_recorded() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        notices::first_claim_wins_duplicate_is_already_recorded(&store).await;
    }

    #[tokio::test]
    async fn tenant_scope_duplicate_dedups() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        notices::tenant_scope_duplicate_dedups(&store).await;
    }

    #[tokio::test]
    async fn lifetime_notice_dedups_per_ceiling() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        notices::lifetime_notice_dedups_per_ceiling(&store).await;
    }

    #[tokio::test]
    async fn raised_ceiling_rearms_the_same_window() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        notices::raised_ceiling_rearms_the_same_window(&store).await;
    }

    #[tokio::test]
    async fn distinct_window_start_is_a_distinct_notice() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        notices::distinct_window_start_is_a_distinct_notice(&store).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sixteen_concurrent_claims_yield_exactly_one_recorded() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        let store: Arc<dyn TreasuryNoticePort> = Arc::new(store);
        notices::sixteen_concurrent_claims_yield_exactly_one_recorded(store).await;
    }

    #[tokio::test]
    async fn notices_for_run_returns_only_that_runs_rows() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        notices::notices_for_run_returns_only_that_runs_rows(&store).await;
    }

    #[tokio::test]
    async fn discard_removes_only_the_named_rows() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        notices::discard_removes_only_the_named_rows(&store).await;
    }

    #[tokio::test]
    async fn notice_round_trips_every_field() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        notices::notice_round_trips_every_field(&store).await;
    }

    #[tokio::test]
    async fn invalid_notice_is_rejected_before_io() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        notices::invalid_notice_is_rejected_before_io(&store).await;
    }

    /// The `ON CONFLICT` column list must read exactly like the migration's unique index column
    /// list, or Postgres cannot infer the index as the arbiter (Pitfall 3).
    #[test]
    fn notice_arbiter_matches_the_migration() {
        let migration = include_str!("../../migrations/postgres/011_create_treasury_notices.sql");
        let columns =
            "(scope_kind, tenant_id, api_key_id, limit_kind, window_start, ceiling_nanos)";
        assert!(
            migration.contains(&format!(
                "CREATE UNIQUE INDEX IF NOT EXISTS idx_treasury_notices_once ON treasury_notices {columns};"
            )),
            "the migration's unique index must list exactly {columns}"
        );
        assert!(
            NOTICE_INSERT.contains(&format!("ON CONFLICT {columns} DO NOTHING")),
            "NOTICE_INSERT's ON CONFLICT column list must textually match the migration's \
             idx_treasury_notices_once (Pitfall 3)"
        );
    }

    // ── Adapter-local tests (not part of the shared contract suite) ───────

    #[test]
    fn settle_arbiter_predicate_matches_the_migration() {
        let migration =
            include_str!("../../migrations/postgres/007_create_treasury_ledger_table.sql");
        assert!(
            migration.contains("WHERE kind = 'settle';"),
            "the migration's settlement index predicate must read \
             \"WHERE kind = 'settle';\" so the write query's arbiter clause below matches it \
             textually"
        );
        assert!(
            SETTLE_INSERT.contains("WHERE kind = 'settle' DO NOTHING"),
            "SETTLE_INSERT's ON CONFLICT arbiter predicate must textually match the migration's \
             partial unique index predicate (Pitfall 3)"
        );
    }

    /// Migrations `011` and `012` apply after `010`: the notices table, its unique identity
    /// index and the additive tenant-window ledger index all exist.
    #[tokio::test]
    async fn migrations_011_and_012_are_applied() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        for name in [
            "idx_treasury_notices_once",
            "idx_treasury_notices_run",
            "idx_treasury_ledger_tenant_window",
        ] {
            let found: Option<String> =
                sqlx::query_scalar("SELECT indexname FROM pg_indexes WHERE indexname = $1")
                    .bind(name)
                    .fetch_optional(&store.pool)
                    .await
                    .unwrap();
            assert_eq!(
                found.as_deref(),
                Some(name),
                "{name} must exist after migration"
            );
        }
    }

    #[tokio::test]
    async fn connection_error_redacts_password_from_database_url() {
        let url = "postgres://user:hunter2-secret@127.0.0.1:1/nonexistent";
        let err = PostgresTreasuryLedger::new(url).await.unwrap_err();
        let message = err.to_string();
        assert!(
            !message.contains("hunter2-secret"),
            "connection error leaked the password: {message}"
        );
    }
}
