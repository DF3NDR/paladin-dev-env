//! Treasury Ledger Port — the Treasurer's Durable Spend Ledger (LEDGR-01..04, ADR-0053)
//!
//! [`TreasuryLedgerPort`] is the WHOLE contract every backend adapter (`InMemoryTreasuryLedger`,
//! `SqliteTreasuryLedger`, `PostgresTreasuryLedger`, all `paladin-storage`) implements: an
//! append-only, derive-on-read ledger whose scope+window balance is a plain `SUM` of every
//! row's signed contribution (ADR-0053 §1-2, cited not re-argued, D-00a). `reserve`/`release`
//! and their per-scope serialized admission arrive with 39-02; this plan's contract is
//! `settle`/`spend`/`store_now` only, proven end-to-end on the SQLite adapter.
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
//! This port knows no allowance policy: `reserve` (39-02) takes the caller's ceiling and
//! window bounds and admits a hold only if the scope+window balance would not exceed the
//! ceiling; Phase 41 computes ceilings and windows from allowance config, this port never
//! reads such config itself.
//!
//! ## Per-scope serialization (39-02)
//!
//! `reserve`'s SUM-then-insert is serialized per scope (Postgres: a transaction-scoped
//! advisory lock; SQLite: `BEGIN IMMEDIATE`; in-memory: one mutex) so concurrent draws against
//! a shared ceiling never overspend (LEDGR-02, ADR-0053 §5) — arriving with 39-02's `reserve`.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use thiserror::Error;

use paladin_core::platform::container::cost::{Cost, CurrencyCode};
use paladin_core::platform::container::treasury_ledger::{
    ReservationId, SettleOutcome, SettleRequest, SpendQuery, SpendRow,
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
    /// A `settle` or `release` named a `reservation` that does not exist on this backend. In
    /// this plan (39-01), every `settle` names `reservation: None` (unreserved) — no adapter can
    /// yet hold a `reserve` row, so any `Some` reservation is unconditionally unknown until
    /// 39-02 adds `reserve`.
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
///     SettleOutcome, SettleRequest, SettlementKey, SpendQuery, SpendRow,
/// };
/// use paladin_ports::output::treasury_ledger_port::{TreasuryLedgerError, TreasuryLedgerPort};
///
/// struct MockLedger {
///     settled: Mutex<HashSet<SettlementKey>>,
/// }
///
/// #[async_trait]
/// impl TreasuryLedgerPort for MockLedger {
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
    /// Record a settlement (D-04): the actual charge for one `SettlementKey`
    /// (`run_id`/`superstep`/`attempt`, ADR-0053 §4).
    ///
    /// Store-enforced idempotency (D-06): a duplicate `request.key` returns
    /// `Ok(SettleOutcome::AlreadySettled)` and changes nothing — never an error. An unreserved
    /// settle (`request.reservation: None`) contributes `request.amount` to its scope+window's
    /// balance at the store's current instant; a reserved settle (39-02) contributes
    /// `actual - hold` and is attributed to its reservation's window (ADR-0053 §2).
    ///
    /// # Errors
    ///
    /// Returns [`TreasuryLedgerError::InvalidRequest`] for a malformed request (validated before
    /// any I/O — see `paladin_storage::treasury::validate_settle`),
    /// [`TreasuryLedgerError::UnknownReservation`] when `request.reservation` is `Some` and does
    /// not resolve on this backend (every adapter in this plan, 39-01, since no adapter can yet
    /// hold a `reserve` row), and [`TreasuryLedgerError::CurrencyMismatch`] when
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
