/*
SQLite Treasury Ledger

Concrete `TreasuryLedgerPort` implementation over SQLite (LEDGR-01..04, ADR-0053, D-03, D-04,
D-06, D-11, D-12). `reserve`/`release` and the reserved `settle` path (this plan, 39-02) admit
or draw down a hold under per-scope serialization: the SUM-then-insert runs inside one
`Pool::begin_with("BEGIN IMMEDIATE")` transaction, so no other connection can begin a competing
write transaction until this one commits or rolls back (ADR-0053 §5). Settlement idempotency is
store-enforced (D-06): the partial unique index `idx_treasury_ledger_settlement` on
`(run_id, superstep, attempt) WHERE kind = 'settle'` plus
`INSERT ... ON CONFLICT (run_id, superstep, attempt) WHERE kind = 'settle' DO NOTHING` -- zero
rows affected maps to `SettleOutcome::AlreadySettled`, never an error.

Every top-level query string below is a plain `&'static str` literal (or, for `spend`'s dynamic
filters, an `sqlx::QueryBuilder` seeded with one) -- never a runtime string-formatting call
building SQL text -- so no caller-supplied value can ever be interpolated into SQL. Every
persisted timestamp is bound from a Rust `DateTime<Utc>` truncated through
`crate::run::storage_timestamp` -- no row is ever stamped by an SQL-side clock expression, so
the TEXT column holds one encoding and lexical order is chronological (D-00e). Migrations follow
the versioned-file convention at `crates/paladin-storage/migrations/sqlite/`, embedded at
compile time via `sqlx::migrate!` and applied automatically on construction.

Every `reserve`/`settle`/`release` transaction body is kept to exactly the statements the method
needs plus commit/rollback -- no other `.await` runs while the transaction is open, since
`BEGIN IMMEDIATE` takes the write lock immediately and holding it across unrelated I/O would
stall every other writer against this file (RESEARCH.md Pitfall 2).
*/

use std::collections::BTreeMap;
use std::str::FromStr;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::sqlite::{Sqlite, SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
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
/// `paladin_core::platform::container::run::RUN_SCHEMA_VERSION`'s convention).
const TREASURY_LEDGER_SCHEMA_VERSION: &str = "v1";

// The `WHERE kind = 'settle'` arbiter predicate below MUST textually match
// `007_create_treasury_ledger_table.sql`'s `idx_treasury_ledger_settlement` index predicate, or
// neither SQLite nor Postgres can infer that partial index as the conflict target (Pitfall 3;
// `settle_arbiter_predicate_matches_the_migration` proves this stays true).
const SETTLE_INSERT: &str = "\
    INSERT INTO treasury_ledger \
      (entry_id, kind, tenant_id, api_key_id, reservation_id, run_id, superstep, attempt, \
       amount_nanos, charged_nanos, currency, model_breakdown, attributed_at, recorded_at, \
       schema_version) \
    VALUES (?, 'settle', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
    ON CONFLICT (run_id, superstep, attempt) WHERE kind = 'settle' DO NOTHING";

// The `ON CONFLICT (...)` column list below MUST textually match
// `011_create_treasury_notices.sql`'s `idx_treasury_notices_once` index column list, or SQLite
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
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
    ON CONFLICT (scope_kind, tenant_id, api_key_id, limit_kind, window_start, ceiling_nanos) \
    DO NOTHING";

/// Every notice recorded for one admitting run, oldest first. Bound: run_id.
const NOTICES_FOR_RUN: &str = "\
    SELECT notice_id, scope_kind, tenant_id, api_key_id, limit_kind, window_start, window_end, \
           ceiling_nanos, currency, balance_nanos, warn_at, run_id, recorded_at \
    FROM treasury_notices WHERE run_id = ? ORDER BY recorded_at ASC, notice_id ASC";

/// Delete one notice by id (an abandoned admission's own row). Bound: notice_id.
const NOTICE_DELETE: &str = "DELETE FROM treasury_notices WHERE notice_id = ?";

/// The store's own clock (ALLOW-01, ADR-0053 §2) -- an RFC 3339 string SQLite's `chrono` decode
/// already parses identically to every other `DateTime<Utc>` column in this codebase.
const STORE_NOW: &str = "SELECT strftime('%Y-%m-%dT%H:%M:%fZ','now')";

/// The balance SUM inside a `reserve`'s transaction: every row in `scope`+`currency` attributed
/// inside the half-open window (D-03, D-12). Bound in order: tenant_id, api_key_id, currency,
/// window_start, window_end.
const BALANCE_QUERY: &str = "\
    SELECT COALESCE(SUM(amount_nanos), 0) FROM treasury_ledger \
    WHERE tenant_id = ? AND api_key_id = ? AND currency = ? AND attributed_at >= ? AND attributed_at < ?";

/// The foreign-currency probe inside a `reserve`'s transaction: any row in `scope` attributed
/// inside the window carrying a currency other than the hold's (ADR-0053 §2 -- a `SUM` over
/// mixed currencies is always a refusal, never a conversion). Bound in order: tenant_id,
/// api_key_id, window_start, window_end, currency.
const FOREIGN_CURRENCY_QUERY: &str = "\
    SELECT currency FROM treasury_ledger \
    WHERE tenant_id = ? AND api_key_id = ? AND attributed_at >= ? AND attributed_at < ? \
      AND currency <> ? LIMIT 1";

/// A `reserve` row: `amount_nanos = +hold`, `charged_nanos = 0`, an empty `model_breakdown`
/// (D-06 -- reserve rows are not idempotency-keyed, so `run_id`/`superstep`/`attempt` may be
/// `NULL`). Bound in order: entry_id, tenant_id, api_key_id, reservation_id, run_id, superstep,
/// attempt, amount_nanos, currency, attributed_at, recorded_at, schema_version.
const RESERVE_INSERT: &str = "\
    INSERT INTO treasury_ledger \
      (entry_id, kind, tenant_id, api_key_id, reservation_id, run_id, superstep, attempt, \
       amount_nanos, charged_nanos, currency, model_breakdown, attributed_at, recorded_at, \
       schema_version) \
    VALUES (?, 'reserve', ?, ?, ?, ?, ?, ?, ?, 0, ?, '{}', ?, ?, ?)";

/// The `kind = 'reserve'` row a `settle` or `release` resolves its `reservation_id` against.
const SELECT_RESERVATION: &str = "\
    SELECT tenant_id, api_key_id, currency, amount_nanos, attributed_at \
    FROM treasury_ledger WHERE kind = 'reserve' AND reservation_id = ?";

/// Whether a reservation is already closed (an earlier `settle` or `release` referencing it
/// already exists): `0` means still open (the full hold is outstanding), `> 0` means closed
/// (the outstanding hold is `0`).
const RESERVATION_CLOSED_QUERY: &str = "\
    SELECT COUNT(*) FROM treasury_ledger WHERE reservation_id = ? AND kind IN ('settle', 'release')";

/// A `release` row: `amount_nanos = -hold`, `charged_nanos = 0`, an empty `model_breakdown`.
/// Bound in order: entry_id, tenant_id, api_key_id, reservation_id, amount_nanos, currency,
/// attributed_at, recorded_at, schema_version.
const RELEASE_INSERT: &str = "\
    INSERT INTO treasury_ledger \
      (entry_id, kind, tenant_id, api_key_id, reservation_id, run_id, superstep, attempt, \
       amount_nanos, charged_nanos, currency, model_breakdown, attributed_at, recorded_at, \
       schema_version) \
    VALUES (?, 'release', ?, ?, ?, NULL, NULL, NULL, ?, 0, ?, '{}', ?, ?, ?)";

/// Every settle row's columns `spend` needs, pre-filtered to `kind = 'settle'` so reserve/release
/// rows never reach the fold.
const SPEND_SELECT_PREFIX: &str = "\
    SELECT tenant_id, api_key_id, run_id, currency, charged_nanos, model_breakdown \
    FROM treasury_ledger WHERE kind = 'settle'";

/// `balance`'s foreign-currency probe prefix (D-00h): any row in the tenant (and optional key,
/// optional half-open window) carrying a currency other than the requested one. Extended with
/// `push_bind` only -- never a nullable-parameter OR-predicate, and never a caller value in SQL text.
const BALANCE_FOREIGN_PREFIX: &str = "\
    SELECT currency FROM treasury_ledger WHERE tenant_id = ";

/// `balance`'s `SUM` prefix: the signed contributions of every row in the same scope, in the
/// requested currency (ADR-0053: reserve, settle and release rows alike).
const BALANCE_SUM_PREFIX: &str = "\
    SELECT COALESCE(SUM(amount_nanos), 0) FROM treasury_ledger WHERE tenant_id = ";

/// Push the optional key and half-open window predicates `balance` shares between its two
/// statements, binding every value.
fn push_balance_scope(builder: &mut QueryBuilder<'_, Sqlite>, query: &BalanceQuery) {
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

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("migrations/sqlite");

/// SQLite `TreasuryLedgerPort` implementation (LEDGR-01, Tier 1: always exercised in CI, no
/// external service required).
#[derive(Debug)]
pub struct SqliteTreasuryLedger {
    pool: SqlitePool,
    /// Kept so every error this store returns can be redacted of the connection URL's password,
    /// not just construction-time connection errors -- mirrors `SqliteRunRepository`'s rationale
    /// (T-22-18).
    database_url: String,
}

impl SqliteTreasuryLedger {
    /// Connect to `database_url`, creating the database file if missing, and apply the
    /// versioned migration. Safe to call more than once against the same database file: the
    /// migration is idempotent (`CREATE TABLE IF NOT EXISTS`/`CREATE INDEX IF NOT EXISTS`) and
    /// `sqlx::migrate::Migrator` itself tracks applied versions.
    ///
    /// # Errors
    ///
    /// Returns [`TreasuryLedgerError::Backend`] (connection URL password redacted first) if the
    /// connection cannot be opened or the migration cannot be applied.
    pub async fn new(database_url: &str) -> Result<Self, TreasuryLedgerError> {
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

    /// Connect over a shared on-disk file with WAL journaling, so multiple pooled connections
    /// (needed for the LEDGR-02 race clause's true-concurrency proof) observe each other's
    /// writes -- unlike `new`, which callers use with `sqlite::memory:` and effectively one
    /// connection. Test-only: production callers always go through `new` (mirrors
    /// `SqliteRunRepository::new_shared_file`, D-52 precedent).
    #[cfg(test)]
    async fn new_shared_file(database_url: &str) -> Result<Self, TreasuryLedgerError> {
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

    /// The store's own clock (shared by `store_now` and `settle`'s `attributed_at`/
    /// `recorded_at` stamping, so both read the same query).
    async fn store_clock(&self) -> Result<DateTime<Utc>, TreasuryLedgerError> {
        sqlx::query_scalar(STORE_NOW)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))
    }
}

#[async_trait]
impl TreasuryLedgerPort for SqliteTreasuryLedger {
    async fn reserve(&self, request: ReserveRequest) -> Result<ReservationId, TreasuryLedgerError> {
        crate::treasury::validate_reserve(&request)?;

        let window_start = crate::run::storage_timestamp(request.window_start);
        let window_end = crate::run::storage_timestamp(request.window_end);

        // Resolve the (optional) settlement key components before opening the transaction --
        // pure computation, nothing that needs the write lock (Pitfall 2).
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

        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
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
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|e| self.wrap_error(e))?;

        let Some(row) = sqlx::query(SELECT_RESERVATION)
            .bind(reservation.as_str())
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| self.wrap_error(e))?
        else {
            tx.rollback().await.map_err(|e| self.wrap_error(e))?;
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

        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|e| self.wrap_error(e))?;

        let (attributed_at, recorded_at, outstanding_hold) = if let Some(reservation) =
            &request.reservation
        {
            let Some(row) = sqlx::query(SELECT_RESERVATION)
                .bind(reservation.as_str())
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| self.wrap_error(e))?
            else {
                tx.rollback().await.map_err(|e| self.wrap_error(e))?;
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
                tx.rollback().await.map_err(|e| self.wrap_error(e))?;
                return Err(TreasuryLedgerError::InvalidRequest {
                    message: format!(
                        "settle scope {:?}/{:?} does not match reservation {reservation}'s \
                             scope {tenant_id:?}/{api_key_id:?}",
                        request.scope.tenant_id, request.scope.api_key_id
                    ),
                });
            }
            if currency != request.amount.currency().as_str() {
                tx.rollback().await.map_err(|e| self.wrap_error(e))?;
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

            let closed_count: i64 = sqlx::query_scalar(RESERVATION_CLOSED_QUERY)
                .bind(reservation.as_str())
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| self.wrap_error(e))?;
            let outstanding = if closed_count > 0 { 0 } else { hold_nanos };

            let now: DateTime<Utc> = sqlx::query_scalar(STORE_NOW)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| self.wrap_error(e))?;
            let recorded_at = crate::run::storage_timestamp(now);

            (reservation_attributed_at, recorded_at, outstanding)
        } else {
            let now: DateTime<Utc> = sqlx::query_scalar(STORE_NOW)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| self.wrap_error(e))?;
            let attributed_at = crate::run::storage_timestamp(now);
            (attributed_at, attributed_at, 0)
        };

        let model_breakdown_json =
            serde_json::to_string(&request.model_breakdown).map_err(|e| {
                TreasuryLedgerError::Serialization {
                    message: format!("model_breakdown could not be serialized: {e}"),
                }
            })?;

        let entry_id = Uuid::now_v7().to_string();
        let actual_nanos = request.amount.nanos();
        let amount_nanos = actual_nanos - outstanding_hold;
        let reservation_id = request.reservation.as_ref().map(|r| r.as_str().to_string());

        let result = sqlx::query(SETTLE_INSERT)
            .bind(entry_id)
            .bind(&request.scope.tenant_id)
            .bind(&request.scope.api_key_id)
            .bind(reservation_id)
            .bind(request.key.run_id.as_str())
            .bind(superstep)
            .bind(attempt)
            .bind(amount_nanos)
            .bind(actual_nanos)
            .bind(request.amount.currency().as_str())
            .bind(model_breakdown_json)
            .bind(attributed_at)
            .bind(recorded_at)
            .bind(TREASURY_LEDGER_SCHEMA_VERSION)
            .execute(&mut *tx)
            .await
            .map_err(|e| self.wrap_error(e))?;

        tx.commit().await.map_err(|e| self.wrap_error(e))?;

        Ok(if result.rows_affected() == 0 {
            SettleOutcome::AlreadySettled
        } else {
            SettleOutcome::Settled
        })
    }

    async fn spend(&self, query: SpendQuery) -> Result<Vec<SpendRow>, TreasuryLedgerError> {
        let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new(SPEND_SELECT_PREFIX);

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
            builder.push(" AND run_id IN (");
            let mut separated = builder.separated(", ");
            for run_id in &query.run_ids {
                separated.push_bind(run_id.as_str().to_string());
            }
            separated.push_unseparated(")");
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
        self.store_clock().await
    }

    async fn balance(&self, query: BalanceQuery) -> Result<Cost, TreasuryLedgerError> {
        crate::treasury::validate_balance(&query)?;

        // Both statements run inside one read transaction so they see one snapshot (a plain
        // BEGIN, not IMMEDIATE: this is a read and must not take the write lock).
        let mut tx = self.pool.begin().await.map_err(|e| self.wrap_error(e))?;

        let mut probe: QueryBuilder<Sqlite> = QueryBuilder::new(BALANCE_FOREIGN_PREFIX);
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

        let mut sum: QueryBuilder<Sqlite> = QueryBuilder::new(BALANCE_SUM_PREFIX);
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

#[async_trait]
impl TreasuryNoticePort for SqliteTreasuryLedger {
    async fn record(&self, notice: &NoticeRecord) -> Result<NoticeOutcome, TreasuryLedgerError> {
        crate::treasury::validate_notice(notice)?;

        // One statement, so no explicit transaction: the unique index arbitrates every
        // concurrent claim, and the connection's busy timeout absorbs write-lock contention.
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
            .bind(i64::from(notice.warning.warn_at))
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
                    warn_at: row.try_get("warn_at").map_err(|e| self.wrap_error(e))?,
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
        // One transaction, so a discard never leaves an admission's notices half-released.
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|e| self.wrap_error(e))?;
        for notice_id in notice_ids {
            sqlx::query(NOTICE_DELETE)
                .bind(notice_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| self.wrap_error(e))?;
        }
        tx.commit().await.map_err(|e| self.wrap_error(e))?;
        Ok(())
    }
}

/// Fold one (group, currency, nanos) contribution into `folded`, incrementing that entry's
/// settlement count by one.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use paladin_core::platform::container::run::RunId;
    use paladin_core::platform::container::treasury_ledger::{LedgerScope, SettlementKey};

    use crate::treasury::contract_tests;

    async fn fresh_store() -> SqliteTreasuryLedger {
        SqliteTreasuryLedger::new("sqlite::memory:").await.unwrap()
    }

    fn usd() -> CurrencyCode {
        CurrencyCode::new("USD").unwrap()
    }

    #[tokio::test]
    async fn settle_then_spend_groups_by_run_and_model() {
        let store = fresh_store().await;
        let run_id = RunId::new_v7();

        let first = store
            .settle(SettleRequest::unreserved(
                LedgerScope::unattributed(),
                SettlementKey::new(run_id.clone(), 1, 1),
                Cost::new(45_000_000, usd()),
                BTreeMap::from([("gpt-4".to_string(), 45_000_000_i64)]),
            ))
            .await
            .unwrap();
        let second = store
            .settle(SettleRequest::unreserved(
                LedgerScope::unattributed(),
                SettlementKey::new(run_id.clone(), 2, 1),
                Cost::new(1_500_000, usd()),
                BTreeMap::from([("gpt-4o-mini".to_string(), 1_500_000_i64)]),
            ))
            .await
            .unwrap();
        assert_eq!(first, SettleOutcome::Settled);
        assert_eq!(second, SettleOutcome::Settled);

        let by_model = store
            .spend(SpendQuery {
                group_by: SpendGroupBy::Model,
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(by_model.len(), 2);
        let gpt4 = by_model.iter().find(|row| row.group == "gpt-4").unwrap();
        assert_eq!(gpt4.amount.nanos(), 45_000_000);
        assert_eq!(gpt4.settlements, 1);
        let mini = by_model
            .iter()
            .find(|row| row.group == "gpt-4o-mini")
            .unwrap();
        assert_eq!(mini.amount.nanos(), 1_500_000);
        assert_eq!(mini.settlements, 1);

        let by_run = store
            .spend(SpendQuery {
                group_by: SpendGroupBy::Run,
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(by_run.len(), 1);
        assert_eq!(by_run[0].group, run_id.as_str());
        assert_eq!(by_run[0].amount.nanos(), 46_500_000);
        assert_eq!(by_run[0].settlements, 2);
    }

    #[tokio::test]
    async fn duplicate_settle_is_already_settled_and_counted_once() {
        let store = fresh_store().await;
        let request = SettleRequest::unreserved(
            LedgerScope::unattributed(),
            SettlementKey::new(RunId::new_v7(), 1, 1),
            Cost::new(45_000_000, usd()),
            BTreeMap::from([("gpt-4".to_string(), 45_000_000_i64)]),
        );

        let first = store.settle(request.clone()).await.unwrap();
        let second = store.settle(request).await.unwrap();
        assert_eq!(first, SettleOutcome::Settled);
        assert_eq!(second, SettleOutcome::AlreadySettled);

        let rows = store
            .spend(SpendQuery {
                group_by: SpendGroupBy::Run,
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].amount.nanos(), 45_000_000);
        assert_eq!(rows[0].settlements, 1);
    }

    #[tokio::test]
    async fn store_now_is_non_decreasing() {
        contract_tests::store_now_is_non_decreasing(&fresh_store().await).await;
    }

    // ── Shared contract suite (D-11): Task 1 clauses ─────────────────────

    #[tokio::test]
    async fn reserve_admits_at_the_ceiling_and_refuses_one_past_it() {
        contract_tests::reserve_admits_at_the_ceiling_and_refuses_one_past_it(&fresh_store().await)
            .await;
    }

    #[tokio::test]
    async fn reserve_then_settle_contributes_actual_minus_hold() {
        contract_tests::reserve_then_settle_contributes_actual_minus_hold(&fresh_store().await)
            .await;
    }

    #[tokio::test]
    async fn unreserved_settle_contributes_actual() {
        contract_tests::unreserved_settle_contributes_actual(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn release_returns_the_hold_and_is_idempotent() {
        contract_tests::release_returns_the_hold_and_is_idempotent(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn settle_after_release_charges_actual_only() {
        contract_tests::settle_after_release_charges_actual_only(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn settle_is_attributed_to_its_reservation_window() {
        contract_tests::settle_is_attributed_to_its_reservation_window(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn reserve_refuses_a_second_currency_in_scope_and_window() {
        contract_tests::reserve_refuses_a_second_currency_in_scope_and_window(&fresh_store().await)
            .await;
    }

    #[tokio::test]
    async fn reserve_rejects_invalid_requests() {
        contract_tests::reserve_rejects_invalid_requests(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn two_reservations_for_one_superstep_attempt_are_legal() {
        contract_tests::two_reservations_for_one_superstep_attempt_are_legal(&fresh_store().await)
            .await;
    }

    // The one clause that needs a REAL shared on-disk database -- proving the LEDGR-02
    // N-1-of-N race under true multi-connection concurrency, not a single in-process
    // `sqlite::memory:` connection (D-52 precedent, `run/sqlite.rs`).
    #[tokio::test(flavor = "multi_thread")]
    async fn reserve_race_admits_exactly_n_minus_one_on_disk() {
        let path = std::env::temp_dir().join(format!(
            "paladin_treasury_ledger_race_test_{}.sqlite",
            Uuid::new_v4()
        ));
        let url = format!("sqlite://{}", path.display());
        let store: Arc<dyn TreasuryLedgerPort> =
            Arc::new(SqliteTreasuryLedger::new_shared_file(&url).await.unwrap());

        contract_tests::reserve_race_admits_exactly_n_minus_one(store).await;

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(format!("{}-wal", path.display()));
        let _ = std::fs::remove_file(format!("{}-shm", path.display()));
    }

    // ── Shared contract suite (D-11): Task 2 clauses ─────────────────────

    #[tokio::test]
    async fn duplicate_settle_is_already_settled_and_charges_once() {
        contract_tests::duplicate_settle_is_already_settled_and_charges_once(&fresh_store().await)
            .await;
    }

    #[tokio::test]
    async fn bumped_attempt_is_a_distinct_settlement() {
        contract_tests::bumped_attempt_is_a_distinct_settlement(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn spend_groups_by_every_dimension_over_a_window() {
        contract_tests::spend_groups_by_every_dimension_over_a_window(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn spend_orders_groups_then_currencies_ascending() {
        contract_tests::spend_orders_groups_then_currencies_ascending(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn spend_over_an_empty_window_is_empty() {
        contract_tests::spend_over_an_empty_window_is_empty(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn spend_window_is_half_open() {
        contract_tests::spend_window_is_half_open(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn spend_splits_currencies_into_separate_rows() {
        contract_tests::spend_splits_currencies_into_separate_rows(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn settle_rejects_a_breakdown_that_does_not_sum_to_the_amount() {
        contract_tests::settle_rejects_a_breakdown_that_does_not_sum_to_the_amount(
            &fresh_store().await,
        )
        .await;
    }

    #[tokio::test]
    async fn unattributed_scope_is_grouped_under_the_sentinel() {
        contract_tests::unattributed_scope_is_grouped_under_the_sentinel(&fresh_store().await)
            .await;
    }

    // ── Phase 41 (41-01) balance clause ───────────────────────────────────

    #[tokio::test]
    async fn tenant_balance_equals_sum_of_key_balances() {
        contract_tests::tenant_balance_equals_sum_of_key_balances(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn balance_sums_signed_contributions_in_window() {
        contract_tests::balance_sums_signed_contributions_in_window(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn key_balance_excludes_other_keys_and_tenants() {
        contract_tests::key_balance_excludes_other_keys_and_tenants(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn balance_window_is_half_open() {
        contract_tests::balance_window_is_half_open(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn balance_unbounded_counts_every_row() {
        contract_tests::balance_unbounded_counts_every_row(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn balance_mixed_currency_is_currency_mismatch() {
        contract_tests::balance_mixed_currency_is_currency_mismatch(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn balance_of_empty_scope_is_zero_in_requested_currency() {
        contract_tests::balance_of_empty_scope_is_zero_in_requested_currency(&fresh_store().await)
            .await;
    }

    #[tokio::test]
    async fn balance_rejects_an_invalid_query() {
        contract_tests::balance_rejects_an_invalid_query(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn balance_is_read_only() {
        contract_tests::balance_is_read_only(&fresh_store().await).await;
    }

    // The concurrency clause that needs a REAL shared on-disk database -- ten concurrent
    // settles of one key against a single SQLite file, proving the partial unique index (not
    // an in-process lock) enforces settlement idempotency under true multi-connection
    // concurrency.
    #[tokio::test(flavor = "multi_thread")]
    async fn concurrent_duplicate_settles_charge_once_on_disk() {
        let path = std::env::temp_dir().join(format!(
            "paladin_treasury_ledger_duplicate_settle_test_{}.sqlite",
            Uuid::new_v4()
        ));
        let url = format!("sqlite://{}", path.display());
        let store: Arc<dyn TreasuryLedgerPort> =
            Arc::new(SqliteTreasuryLedger::new_shared_file(&url).await.unwrap());

        contract_tests::concurrent_duplicate_settles_charge_once(store).await;

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(format!("{}-wal", path.display()));
        let _ = std::fs::remove_file(format!("{}-shm", path.display()));
    }

    #[tokio::test]
    async fn connection_error_redacts_password_from_database_url() {
        let url = "sqlite://user:hunter2-secret@/nonexistent/path/that/does/not/exist.db";
        let err = SqliteTreasuryLedger::new(url).await.unwrap_err();
        let message = err.to_string();
        assert!(
            !message.contains("hunter2-secret"),
            "connection error leaked the password: {message}"
        );
    }

    #[test]
    fn settle_arbiter_predicate_matches_the_migration() {
        let migration =
            include_str!("../../migrations/sqlite/007_create_treasury_ledger_table.sql");
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

    // ── Notice clauses (ALLOW-04, D-16, 41-06) ───────────────────────────

    use crate::treasury::notice_contract_tests as notices;
    use paladin_ports::output::treasury_notice_port::TreasuryNoticePort;

    #[tokio::test]
    async fn first_claim_wins_duplicate_is_already_recorded() {
        notices::first_claim_wins_duplicate_is_already_recorded(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn tenant_scope_duplicate_dedups() {
        notices::tenant_scope_duplicate_dedups(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn lifetime_notice_dedups_per_ceiling() {
        notices::lifetime_notice_dedups_per_ceiling(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn raised_ceiling_rearms_the_same_window() {
        notices::raised_ceiling_rearms_the_same_window(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn distinct_window_start_is_a_distinct_notice() {
        notices::distinct_window_start_is_a_distinct_notice(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn notices_for_run_returns_only_that_runs_rows() {
        notices::notices_for_run_returns_only_that_runs_rows(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn discard_removes_only_the_named_rows() {
        notices::discard_removes_only_the_named_rows(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn notice_round_trips_every_field() {
        notices::notice_round_trips_every_field(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn invalid_notice_is_rejected_before_io() {
        notices::invalid_notice_is_rejected_before_io(&fresh_store().await).await;
    }

    // The notice race needs a REAL shared on-disk database: sixteen claims over several pooled
    // connections, so the unique index (not a single in-process connection) arbitrates.
    #[tokio::test(flavor = "multi_thread")]
    async fn sixteen_concurrent_claims_yield_exactly_one_recorded() {
        let path = std::env::temp_dir().join(format!(
            "paladin_treasury_notice_race_test_{}.sqlite",
            Uuid::new_v4()
        ));
        let url = format!("sqlite://{}", path.display());
        let store: Arc<dyn TreasuryNoticePort> =
            Arc::new(SqliteTreasuryLedger::new_shared_file(&url).await.unwrap());

        notices::sixteen_concurrent_claims_yield_exactly_one_recorded(store).await;

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(format!("{}-wal", path.display()));
        let _ = std::fs::remove_file(format!("{}-shm", path.display()));
    }

    /// The `ON CONFLICT` column list must read exactly like the migration's unique index column
    /// list, or SQLite cannot infer the index as the arbiter (Pitfall 3).
    #[test]
    fn notice_arbiter_matches_the_migration() {
        let migration = include_str!("../../migrations/sqlite/011_create_treasury_notices.sql");
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

    /// Migrations `011` and `012` apply after `010` on a fresh database: the notices table, its
    /// unique identity index and the additive tenant-window ledger index all exist.
    #[tokio::test]
    async fn migrations_011_and_012_apply_to_a_fresh_database() {
        let store = fresh_store().await;
        for name in [
            "treasury_notices",
            "idx_treasury_notices_once",
            "idx_treasury_notices_run",
            "idx_treasury_ledger_tenant_window",
        ] {
            let found: Option<String> =
                sqlx::query_scalar("SELECT name FROM sqlite_master WHERE name = ?")
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

    // ── Exact-instant window edges (adapter-local, 41-02) ─────────────────

    /// Rows attributed exactly at `window_start` and exactly at `window_end` land in the window
    /// that starts, respectively ends, at that instant -- half-open, no gap and no overlap. The
    /// rows are inserted raw so their `attributed_at` is a whole-second instant the store clock
    /// could never be made to hit.
    #[tokio::test]
    async fn balance_counts_a_row_at_window_start_and_excludes_one_at_window_end() {
        let store = fresh_store().await;
        let ws = "2026-01-01T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let we = ws + chrono::Duration::hours(1);

        for (entry_id, nanos, attributed_at) in [("edge-ws", 3_i64, ws), ("edge-we", 5_i64, we)] {
            let stamped = crate::run::storage_timestamp(attributed_at);
            sqlx::query(
                "INSERT INTO treasury_ledger \
                   (entry_id, kind, tenant_id, api_key_id, reservation_id, run_id, superstep, \
                    attempt, amount_nanos, charged_nanos, currency, model_breakdown, \
                    attributed_at, recorded_at, schema_version) \
                 VALUES (?, 'reserve', 'edge-tenant', 'edge-key', NULL, NULL, NULL, NULL, ?, 0, \
                         'USD', '{}', ?, ?, 'v1')",
            )
            .bind(entry_id)
            .bind(nanos)
            .bind(stamped)
            .bind(stamped)
            .execute(&store.pool)
            .await
            .unwrap();
        }

        let query = |since: DateTime<Utc>, until: DateTime<Utc>| BalanceQuery {
            tenant_id: "edge-tenant".to_string(),
            api_key_id: Some("edge-key".to_string()),
            currency: usd(),
            since: Some(since),
            until: Some(until),
        };

        assert_eq!(
            store.balance(query(ws, we)).await.unwrap(),
            Cost::new(3, usd()),
            "[ws, we) must count the row at ws and exclude the row at we"
        );
        assert_eq!(
            store
                .balance(query(we, we + chrono::Duration::hours(1)))
                .await
                .unwrap(),
            Cost::new(5, usd()),
            "[we, we + 1h) must count the row at we and not the row at ws"
        );
    }
}
