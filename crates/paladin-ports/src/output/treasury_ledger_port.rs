//! Treasury Ledger Port — the Treasurer's Durable Spend Ledger (LEDGR-01..04, ADR-0053)
//!
//! [`TreasuryLedgerPort`] is the WHOLE contract every backend adapter (`InMemoryTreasuryLedger`,
//! `SqliteTreasuryLedger`, `PostgresTreasuryLedger`, all `paladin-storage`) implements: an
//! append-only, derive-on-read ledger whose scope+window balance is a plain `SUM` of every
//! row's signed contribution (ADR-0053 §1-2, cited not re-argued, D-00a). `reserve` and
//! `release` (this plan, 39-02) admit or release a hold under per-scope serialization;
//! `settle`/`spend`/`store_now` (39-01) are unchanged.
//!
//! ## Store-enforced settlement idempotency (D-06)
//!
//! [`TreasuryLedgerPort::settle`] must compile to a store-enforced constraint — a partial
//! unique index on `(run_id, superstep, attempt)` restricted to `kind = 'settle'`, with
//! `INSERT ... ON CONFLICT ... DO NOTHING` — never an application-level "have I already settled
//! this?" check. Zero rows affected is [`SettleOutcome::AlreadySettled`], never an error: a
//! duplicate settlement key under lease redelivery, resume, retry or model fallback is success
//! (LEDGR-03).
//!
//! ## Policy-free ledger (D-03)
//!
//! This port knows no allowance policy: [`TreasuryLedgerPort::reserve`] takes the caller's
//! ceiling and window bounds and admits a hold only if the scope+window balance would not
//! exceed the ceiling; Phase 41 computes ceilings and windows from allowance config, this port
//! never reads such config itself.
//!
//! ## Per-scope serialization (LEDGR-02, ADR-0053 §5)
//!
//! `reserve`'s SUM-then-insert is serialized per scope inside one transaction (Postgres: a
//! transaction-scoped advisory lock; SQLite: `BEGIN IMMEDIATE`; in-memory: one mutex across the
//! SUM and the insert) so concurrent draws against a shared ceiling never overspend: `N`
//! concurrent reserves against a ceiling fitting `N-1` yield exactly `N-1` `Ok` and one
//! `Refused`.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use thiserror::Error;

use paladin_core::platform::container::cost::{Cost, CurrencyCode};
use paladin_core::platform::container::treasury_ledger::{
    ReservationId, ReserveRequest, SettleOutcome, SettleRequest, SpendQuery, SpendRow,
};

/// Errors returned by [`TreasuryLedgerPort`] methods (X-06 — structured, never a bare
/// `bool`/`String`).
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TreasuryLedgerError {
    /// A `reserve` call was rejected because admitting the hold would exceed the scope+window
    /// ceiling (D-03). Names the three nano-unit figures verbatim so Phase 41's admission error
    /// and Phase 42's halt reason can carry them unchanged.
    #[error("reserve refused: balance {balance:?} + hold would exceed ceiling {ceiling:?}")]
    Refused {
        /// The scope+window balance before this hold.
        balance: Cost,
        /// The hold this call attempted to place.
        hold: Cost,
        /// The ceiling the balance may not exceed.
        ceiling: Cost,
    },
    /// A `settle` (or `reserve`) currency did not match the currency already recorded for its
    /// reservation or scope+window (ADR-0053 §2: a `SUM` over mixed currencies is always a
    /// refusal, never a conversion).
    #[error("currency mismatch: expected {expected}, found {found}")]
    CurrencyMismatch {
        /// The currency already on record.
        expected: CurrencyCode,
        /// The currency the rejected call carried.
        found: CurrencyCode,
    },
    /// A `reserve`, `settle` or `release` named a `reservation` that does not exist on this
    /// backend — the id was never issued by [`TreasuryLedgerPort::reserve`] on this store.
    #[error("unknown reservation: {reservation}")]
    UnknownReservation {
        /// The reservation id that could not be resolved.
        reservation: ReservationId,
    },
    /// The request itself was malformed (empty scope fields, a negative amount, a
    /// `model_breakdown` that does not sum to the settled amount, an integer overflow
    /// converting `superstep`, …) — rejected before any I/O.
    #[error("invalid treasury ledger request: {message}")]
    InvalidRequest {
        /// Description of what was invalid.
        message: String,
    },
    /// The underlying storage backend failed.
    #[error("treasury ledger backend error: {source}")]
    Backend {
        /// The underlying backend error.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// A stored (or to-be-stored) ledger value could not be (de)serialized (e.g.
    /// `model_breakdown`'s JSON encoding).
    #[error("treasury ledger serialization error: {message}")]
    Serialization {
        /// Description of the serialization failure.
        message: String,
    },
}

/// Port trait for the Treasurer's durable, append-only spend ledger (ADR-0053).
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync`: settlements are written and spend is queried
/// concurrently across HTTP handlers, worker tasks and the CLI.
///
/// # Examples
///
/// ```
/// use std::collections::{BTreeMap, HashSet};
/// use std::sync::Mutex;
///
/// use async_trait::async_trait;
/// use chrono::{DateTime, Utc};
/// use paladin_core::platform::container::cost::Cost;
/// use paladin_core::platform::container::treasury_ledger::{
///     ReservationId, ReserveRequest, SettleOutcome, SettleRequest, SettlementKey, SpendQuery,
///     SpendRow,
/// };
/// use paladin_ports::output::treasury_ledger_port::{TreasuryLedgerError, TreasuryLedgerPort};
///
/// struct MockLedger {
///     settled: Mutex<HashSet<SettlementKey>>,
/// }
///
/// #[async_trait]
/// impl TreasuryLedgerPort for MockLedger {
///     async fn reserve(&self, _request: ReserveRequest) -> Result<ReservationId, TreasuryLedgerError> {
///         // A real adapter serializes the SUM-then-insert per scope (LEDGR-02); this mock has
///         // no policy to enforce, so it always admits.
///         Ok(ReservationId::new_v7())
///     }
///
///     async fn release(&self, _reservation: ReservationId) -> Result<(), TreasuryLedgerError> {
///         Ok(())
///     }
///
///     async fn settle(&self, request: SettleRequest) -> Result<SettleOutcome, TreasuryLedgerError> {
///         let mut settled = self.settled.lock().unwrap();
///         if !settled.insert(request.key) {
///             return Ok(SettleOutcome::AlreadySettled);
///         }
///         Ok(SettleOutcome::Settled)
///     }
///
///     async fn spend(&self, _query: SpendQuery) -> Result<Vec<SpendRow>, TreasuryLedgerError> {
///         Ok(vec![])
///     }
///
///     async fn store_now(&self) -> Result<DateTime<Utc>, TreasuryLedgerError> {
///         Ok(Utc::now())
///     }
/// }
///
/// # use paladin_core::platform::container::cost::CurrencyCode;
/// # use paladin_core::platform::container::run::RunId;
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let ledger = MockLedger {
///         settled: Mutex::new(HashSet::new()),
///     };
///     let key = SettlementKey::new(RunId::new_v7(), 1, 1);
///     let amount = Cost::new(45_000_000, CurrencyCode::new("USD")?);
///     let request = SettleRequest::unreserved(
///         paladin_core::platform::container::treasury_ledger::LedgerScope::unattributed(),
///         key.clone(),
///         amount.clone(),
///         BTreeMap::from([("gpt-4".to_string(), 45_000_000_i64)]),
///     );
///
///     let first = ledger.settle(request.clone()).await?;
///     let second = ledger.settle(request).await?;
///     assert_eq!(first, SettleOutcome::Settled);
///     assert_eq!(second, SettleOutcome::AlreadySettled, "a duplicate settlement key is success");
///     Ok(())
/// }
/// ```
#[async_trait]
pub trait TreasuryLedgerPort: Send + Sync {
    /// Place a hold against a scope+window's ceiling (D-03, D-04, LEDGR-02).
    ///
    /// Admits the hold only if `balance + request.hold <= request.ceiling`, where `balance` is
    /// the plain `SUM` of `amount_nanos` over the scope's rows in `request.hold`'s currency
    /// attributed inside `[request.window_start, request.window_end)`. An overflowing add does
    /// not fit (refused, never a panic). A row of another currency already in that scope and
    /// window is [`TreasuryLedgerError::CurrencyMismatch`] (never converted, never silently
    /// combined). A refusal is [`TreasuryLedgerError::Refused`], never `Ok(false)` (X-06).
    ///
    /// The SUM and the insert are serialized per scope inside one transaction (SQLite `BEGIN
    /// IMMEDIATE`, Postgres a transaction-scoped advisory lock on the scope, in-memory one
    /// mutex) so `N` concurrent reserves against a ceiling fitting `N-1` yield exactly `N-1`
    /// `Ok` and one `Refused` (LEDGR-02). The reservation is attributed to the store's own
    /// clock at the instant it is admitted (ADR-0053 §2) — never the caller's `Utc::now()`. The
    /// port knows no allowance policy (D-03): it never reads config to decide a ceiling or a
    /// window, only compares the values the caller supplies.
    ///
    /// Two reservations may legally carry the same `request.key` (D-06): only `settle` rows are
    /// keyed for idempotency, so a retry after a released hold can reserve again under the same
    /// superstep attempt.
    ///
    /// # Errors
    ///
    /// Returns [`TreasuryLedgerError::InvalidRequest`] for a malformed request (empty scope
    /// fields, a negative `hold` or `ceiling`, or `window_start >= window_end` — validated
    /// before any I/O), [`TreasuryLedgerError::CurrencyMismatch`] when `request.hold`'s currency
    /// differs from `request.ceiling`'s, or from another currency already found in the same
    /// scope+window, and [`TreasuryLedgerError::Refused`] when admitting the hold would exceed
    /// the ceiling.
    async fn reserve(&self, request: ReserveRequest) -> Result<ReservationId, TreasuryLedgerError>;

    /// Release a hold (D-04): idempotent, and a no-op for a reservation that is already settled
    /// or already released — calling `release` twice, or after a `settle` referencing the same
    /// reservation, changes nothing and returns `Ok(())` both times.
    ///
    /// # Errors
    ///
    /// Returns [`TreasuryLedgerError::UnknownReservation`] when `reservation` does not resolve
    /// on this backend (an id this store never issued).
    async fn release(&self, reservation: ReservationId) -> Result<(), TreasuryLedgerError>;

    /// Record a settlement (D-04): the actual charge for one `SettlementKey`
    /// (`run_id`/`superstep`/`attempt`, ADR-0053 §4).
    ///
    /// Store-enforced idempotency (D-06): a duplicate `request.key` returns
    /// `Ok(SettleOutcome::AlreadySettled)` and changes nothing — never an error. An unreserved
    /// settle (`request.reservation: None`) contributes `request.amount` to its scope+window's
    /// balance at the store's current instant. A reserved settle contributes `actual - hold`
    /// while the reservation is still open, or `actual` once it has been closed by an earlier
    /// settle or release, and is attributed to its reservation's own instant — never the
    /// settle's own call time — so a hold placed in one window and settled after that window
    /// closes still counts against the window it was placed in (ADR-0053 §2). `request.scope`
    /// must equal the reservation's own scope, and `request.amount`'s currency must equal the
    /// reservation's currency.
    ///
    /// # Errors
    ///
    /// Returns [`TreasuryLedgerError::InvalidRequest`] for a malformed request (validated before
    /// any I/O — see `paladin_storage::treasury::validate_settle`, and, for a reserved settle, a
    /// `request.scope` that does not match the reservation's own scope),
    /// [`TreasuryLedgerError::UnknownReservation`] when `request.reservation` is `Some` and does
    /// not resolve on this backend, and [`TreasuryLedgerError::CurrencyMismatch`] when
    /// `request.amount`'s currency differs from its reservation's or from another currency
    /// already found in the same scope+window.
    async fn settle(&self, request: SettleRequest) -> Result<SettleOutcome, TreasuryLedgerError>;

    /// Query settled spend (LEDGR-04): settle rows only, grouped per `query.group_by`, summing
    /// the charged amount over the half-open window `[query.since, query.until)` on the
    /// attribution instant. Returns exactly one [`SpendRow`] per (group value, currency) pair,
    /// ordered by group value then currency code ascending — currencies are never combined
    /// (D-09). `Model` grouping folds every row's `model_breakdown` (D-02). An empty window
    /// (or a window with no matching settlements) returns an empty `Vec`, never an error.
    async fn spend(&self, query: SpendQuery) -> Result<Vec<SpendRow>, TreasuryLedgerError>;

    /// The store's own clock (ALLOW-01, ADR-0053 §2) — never the caller's `Utc::now()` — so
    /// Phase 41 can compute window boundaries from one authoritative clock shared by every
    /// worker and the CLI alike.
    async fn store_now(&self) -> Result<DateTime<Utc>, TreasuryLedgerError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::run::RunId;
    use paladin_core::platform::container::treasury_ledger::{LedgerScope, SettlementKey};
    use std::collections::{BTreeMap, HashSet};
    use std::sync::Mutex;

    /// A minimal mock, mirroring `run_repository_port.rs`'s `InMemoryRuns` convention: proves
    /// the trait is implementable and object-safe.
    struct MockLedger {
        settled: Mutex<HashSet<SettlementKey>>,
    }

    #[async_trait]
    impl TreasuryLedgerPort for MockLedger {
        async fn reserve(
            &self,
            _request: paladin_core::platform::container::treasury_ledger::ReserveRequest,
        ) -> Result<ReservationId, TreasuryLedgerError> {
            Ok(ReservationId::new_v7())
        }

        async fn release(&self, _reservation: ReservationId) -> Result<(), TreasuryLedgerError> {
            Ok(())
        }

        async fn settle(
            &self,
            request: SettleRequest,
        ) -> Result<SettleOutcome, TreasuryLedgerError> {
            let mut settled = self.settled.lock().unwrap();
            if !settled.insert(request.key) {
                return Ok(SettleOutcome::AlreadySettled);
            }
            Ok(SettleOutcome::Settled)
        }

        async fn spend(&self, _query: SpendQuery) -> Result<Vec<SpendRow>, TreasuryLedgerError> {
            Ok(vec![])
        }

        async fn store_now(&self) -> Result<DateTime<Utc>, TreasuryLedgerError> {
            Ok(Utc::now())
        }
    }

    #[tokio::test]
    async fn duplicate_settle_key_is_already_settled_never_an_error() {
        let ledger: Box<dyn TreasuryLedgerPort> = Box::new(MockLedger {
            settled: Mutex::new(HashSet::new()),
        });
        let key = SettlementKey::new(RunId::new_v7(), 1, 1);
        let amount = Cost::new(45_000_000, CurrencyCode::new("USD").unwrap());
        let request =
            SettleRequest::unreserved(LedgerScope::unattributed(), key, amount, BTreeMap::new());

        let first = ledger.settle(request.clone()).await.unwrap();
        let second = ledger.settle(request).await.unwrap();

        assert_eq!(first, SettleOutcome::Settled);
        assert_eq!(second, SettleOutcome::AlreadySettled);
    }
}
