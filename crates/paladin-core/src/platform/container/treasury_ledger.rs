//! Treasurer spend-ledger domain types (LEDGR-01..04, ADR-0053).
//!
//! Pure value types with no I/O: the append-only ledger's row kinds, its identity types
//! ([`crate::platform::container::treasury_ledger::ReservationId`],
//! [`crate::platform::container::treasury_ledger::SettlementKey`]), and the request/query/response shapes
//! `TreasuryLedgerPort` (`paladin-ports`) and its adapters (`paladin-storage`) exchange.
//!
//! ADR-0053 governs the model implemented here (cited, not re-argued, D-00a): the ledger is
//! append-only and derive-on-read -- a scope+window balance is a plain `SUM` of every row's
//! signed contribution (`reserve` = `+hold`, `settle` = `actual - hold` or `actual` when
//! unreserved, `release` = `-hold`); a settle/release row is attributed to its reservation's
//! window; every amount is a [`crate::platform::container::cost::Cost`] nano-unit figure
//! carrying its own currency, and a `SUM` across mismatched currencies is a typed refusal,
//! never a silent conversion; the settlement identity is `(run_id, superstep, attempt)` with
//! **no** `node_id` -- one settlement row aggregates every Paladin node dispatched inside that
//! superstep attempt, and per-node cost stays in `TraceEvent::NodeFinished::cost` only.
//!
//! `Treasurer` is a framework-only word (ADR-0050); nothing in this module -- or anywhere else
//! in this codebase -- is named `GarrisonTreasury`.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::platform::container::cost::Cost;
use crate::platform::container::run::RunId;

/// A tenant + API-key pair every ledger row is scoped to (D-01).
///
/// Every reserve/settle/query call takes a `LedgerScope`. Until Phase 40 records the
/// submitting principal's tenant on the `Run` row, every Phase 39 production writer stamps the
/// literal sentinel [`LedgerScope::unattributed`] -- Phase 40 replaces only the *source* of
/// these two strings, never the schema, the port, or the queries built against it. `api_key_id`
/// is an opaque key label, never the secret key value itself.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::treasury_ledger::LedgerScope;
///
/// let scope = LedgerScope::new("tenant-1", "key-1");
/// assert_eq!(scope.tenant_id, "tenant-1");
/// assert!(!scope.is_unattributed());
///
/// let sentinel = LedgerScope::unattributed();
/// assert!(sentinel.is_unattributed());
/// assert_eq!(sentinel.tenant_id, LedgerScope::UNATTRIBUTED);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LedgerScope {
    /// The tenant this row (or query) is scoped to.
    pub tenant_id: String,
    /// The API key label this row (or query) is scoped to.
    pub api_key_id: String,
}

impl LedgerScope {
    /// The literal sentinel every Phase 39 production writer stamps until Phase 40 records a
    /// real tenant/API-key on the submitting principal (D-01).
    pub const UNATTRIBUTED: &'static str = "unattributed";

    /// Construct a scope from a caller-known tenant and API key.
    pub fn new(tenant_id: impl Into<String>, api_key_id: impl Into<String>) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            api_key_id: api_key_id.into(),
        }
    }

    /// The documented sentinel scope (D-01): both fields set to
    /// [`LedgerScope::UNATTRIBUTED`].
    pub fn unattributed() -> Self {
        Self::new(Self::UNATTRIBUTED, Self::UNATTRIBUTED)
    }

    /// Whether this scope is the [`LedgerScope::unattributed`] sentinel.
    pub fn is_unattributed(&self) -> bool {
        self.tenant_id == Self::UNATTRIBUTED && self.api_key_id == Self::UNATTRIBUTED
    }
}

/// Identity of a reservation: a UUIDv7 (time-ordered) value, mirroring
/// [`crate::platform::container::run::RunId`]'s convention. Not yet produced by any adapter in
/// this phase (39-02 adds `reserve`); defined here so the type is stable across the phase.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::treasury_ledger::ReservationId;
///
/// let id = ReservationId::new_v7();
/// let round_tripped = ReservationId::parse(id.as_str())?;
/// assert_eq!(id, round_tripped);
/// assert!(ReservationId::parse("not-a-uuid").is_err());
/// # Ok::<(), paladin_core::platform::container::treasury_ledger::ReservationIdError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ReservationId(String);

/// Error returned by [`ReservationId::parse`] when the supplied string is not a valid UUID.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ReservationIdError {
    /// The supplied reservation id was empty.
    #[error("reservation id must not be empty")]
    Empty,
    /// The supplied reservation id was not a valid UUID.
    #[error("reservation id {value:?} is not a valid UUID")]
    InvalidUuid {
        /// The rejected value.
        value: String,
    },
}

impl ReservationId {
    /// Generate a fresh, time-ordered `ReservationId` (UUIDv7).
    pub fn new_v7() -> Self {
        Self(Uuid::now_v7().to_string())
    }

    /// Parse a `ReservationId` from a caller-supplied string, validating it is a well-formed
    /// UUID.
    ///
    /// # Errors
    ///
    /// Returns [`ReservationIdError::Empty`] for an empty string, or
    /// [`ReservationIdError::InvalidUuid`] if the string is not a valid UUID.
    pub fn parse(id: impl Into<String>) -> Result<Self, ReservationIdError> {
        let id = id.into();
        if id.is_empty() {
            return Err(ReservationIdError::Empty);
        }
        Uuid::parse_str(&id).map_err(|_| ReservationIdError::InvalidUuid { value: id.clone() })?;
        Ok(Self(id))
    }

    /// Borrow the reservation id as a `&str`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ReservationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The settlement idempotency key (ADR-0053 §4, D-06): one settlement row per superstep
/// attempt, with **no** `node_id` -- every Paladin node dispatched inside that superstep
/// attempt (including every lease redelivery, resume, retry and model-fallback hop that stays
/// within the same attempt) folds into this one row.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::run::RunId;
/// use paladin_core::platform::container::treasury_ledger::SettlementKey;
///
/// let key = SettlementKey::new(RunId::new_v7(), 1, 1);
/// assert_eq!(key.superstep, 1);
/// assert_eq!(key.attempt, 1);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SettlementKey {
    /// The run this settlement belongs to.
    pub run_id: RunId,
    /// The engine's superstep number (or, on the agent loop, the model-call ordinal, D-07).
    pub superstep: u64,
    /// The persisted `Run.attempt` counter on the engine path; `1` on the agent loop (D-07).
    pub attempt: u32,
}

impl SettlementKey {
    /// Construct a settlement key from its three components.
    pub fn new(run_id: RunId, superstep: u64, attempt: u32) -> Self {
        Self {
            run_id,
            superstep,
            attempt,
        }
    }
}

/// The three row kinds ADR-0053's append-only ledger stores.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LedgerEntryKind {
    /// A hold placed against a scope+window's ceiling (`amount_nanos = +hold`).
    Reserve,
    /// A settlement: `actual - hold` for a reserved settle, `actual` when unreserved.
    Settle,
    /// A released hold (`amount_nanos = -hold`).
    Release,
}

impl LedgerEntryKind {
    /// The lowercase string this kind serializes to and the SQL `kind` column stores.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_core::platform::container::treasury_ledger::LedgerEntryKind;
    ///
    /// assert_eq!(LedgerEntryKind::Settle.as_str(), "settle");
    /// assert_eq!(LedgerEntryKind::parse_str("settle"), Some(LedgerEntryKind::Settle));
    /// ```
    pub fn as_str(self) -> &'static str {
        match self {
            LedgerEntryKind::Reserve => "reserve",
            LedgerEntryKind::Settle => "settle",
            LedgerEntryKind::Release => "release",
        }
    }

    /// Parse a kind from its stored string, `None` for anything else.
    pub fn parse_str(value: &str) -> Option<Self> {
        match value {
            "reserve" => Some(LedgerEntryKind::Reserve),
            "settle" => Some(LedgerEntryKind::Settle),
            "release" => Some(LedgerEntryKind::Release),
            _ => None,
        }
    }
}

/// The parameters a `reserve` call takes (D-03): the port itself knows no allowance policy --
/// the caller supplies the ceiling and the half-open window `[window_start, window_end)`, and
/// should compute the window from `store_now()` so it contains the present instant. Not yet
/// implemented by any adapter in this phase (39-02 adds `reserve`); defined here so the type is
/// stable across the phase.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReserveRequest {
    /// The scope this hold is placed against.
    pub scope: LedgerScope,
    /// The amount to hold.
    pub hold: Cost,
    /// The ceiling the scope+window's balance may not exceed once this hold is admitted.
    pub ceiling: Cost,
    /// The window's start instant (inclusive).
    pub window_start: DateTime<Utc>,
    /// The window's end instant (exclusive).
    pub window_end: DateTime<Utc>,
    /// The settlement this reservation is placed for, if known at reserve time.
    pub key: Option<SettlementKey>,
}

/// The parameters a `settle` call takes (D-04).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettleRequest {
    /// The scope this settlement is attributed to.
    pub scope: LedgerScope,
    /// The settlement idempotency key.
    pub key: SettlementKey,
    /// The actual charge.
    pub amount: Cost,
    /// Bare model name -> nano-units; values sum to `amount` (D-02). May be empty only when
    /// `amount` is zero.
    pub model_breakdown: BTreeMap<String, i64>,
    /// The reservation this settle draws down, if any (`None` for an unreserved settle, D-04).
    pub reservation: Option<ReservationId>,
}

impl SettleRequest {
    /// Construct an unreserved settle request (`reservation: None`) -- the shape every Phase 39
    /// production writer uses.
    pub fn unreserved(
        scope: LedgerScope,
        key: SettlementKey,
        amount: Cost,
        model_breakdown: BTreeMap<String, i64>,
    ) -> Self {
        Self {
            scope,
            key,
            amount,
            model_breakdown,
            reservation: None,
        }
    }
}

/// The result of a `settle` call (D-04, LEDGR-03): a duplicate settlement key is a *success*,
/// never an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettleOutcome {
    /// This call's settlement row was newly inserted.
    Settled,
    /// A settlement already existed for this key; nothing changed.
    AlreadySettled,
}

/// The dimension `spend` groups its rows by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SpendGroupBy {
    /// Group by `tenant_id`.
    #[default]
    Tenant,
    /// Group by `api_key_id`.
    ApiKey,
    /// Group by `run_id`.
    Run,
    /// Group by each model name inside `model_breakdown` (D-02).
    Model,
}

impl SpendGroupBy {
    /// The CLI/API label this dimension renders as.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_core::platform::container::treasury_ledger::SpendGroupBy;
    ///
    /// assert_eq!(SpendGroupBy::ApiKey.as_str(), "api-key");
    /// ```
    pub fn as_str(self) -> &'static str {
        match self {
            SpendGroupBy::Tenant => "tenant",
            SpendGroupBy::ApiKey => "api-key",
            SpendGroupBy::Run => "run",
            SpendGroupBy::Model => "model",
        }
    }
}

/// The filter/grouping parameters a `spend` call takes (D-09, LEDGR-04).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SpendQuery {
    /// The dimension to group rows by.
    pub group_by: SpendGroupBy,
    /// The half-open window's start instant (inclusive); `None` is unbounded.
    pub since: Option<DateTime<Utc>>,
    /// The half-open window's end instant (exclusive); `None` is unbounded.
    pub until: Option<DateTime<Utc>>,
    /// Restrict to one tenant.
    pub tenant_id: Option<String>,
    /// Restrict to one API key.
    pub api_key_id: Option<String>,
    /// Restrict to these runs; empty means no run filter.
    pub run_ids: Vec<RunId>,
}

/// One row of a `spend` result: one (group value, currency) pair (LEDGR-04). `spend` never
/// combines two currencies into one row (D-09).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpendRow {
    /// The group's value (a tenant id, API key id, run id, or model name).
    pub group: String,
    /// The settled spend for this group and currency.
    pub amount: Cost,
    /// The number of settle rows folded into this figure.
    pub settlements: u64,
}

/// The per-run identity an engine-path (or agent-loop) writer settles under (D-07); consumed by
/// later plans wiring the production settle writers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementContext {
    /// The scope to settle under.
    pub scope: LedgerScope,
    /// The run this settlement belongs to.
    pub run_id: RunId,
    /// The persisted attempt counter (or `1` on the agent loop).
    pub attempt: u32,
}

/// Render a [`Cost`] as four decimals followed by a space and the currency code
/// (`"0.0450 USD"`) -- the single D-04 format, reused verbatim by the CLI table and matching
/// [`crate::platform::container::herald::ExecutionMetadata::cost_display`] byte-for-byte
/// (D-00c). The nanos-to-float conversion happens exactly once, here.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::cost::{Cost, CurrencyCode};
/// use paladin_core::platform::container::treasury_ledger::format_cost;
///
/// let cost = Cost::new(45_000_000, CurrencyCode::new("USD")?);
/// assert_eq!(format_cost(&cost), "0.0450 USD");
/// # Ok::<(), paladin_core::platform::container::cost::CostError>(())
/// ```
pub fn format_cost(cost: &Cost) -> String {
    let value = cost.nanos() as f64 / 1e9;
    format!("{value:.4} {code}", code = cost.currency().as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::container::cost::CurrencyCode;
    use crate::platform::container::herald::ExecutionMetadata;
    use crate::platform::container::token_usage::TokenUsage;

    fn usd() -> CurrencyCode {
        CurrencyCode::new("USD").expect("USD is a valid currency code")
    }

    #[test]
    fn unattributed_scope_uses_the_literal_sentinel() {
        let scope = LedgerScope::unattributed();
        assert_eq!(scope.tenant_id, "unattributed");
        assert_eq!(scope.api_key_id, "unattributed");
        assert!(scope.is_unattributed());
        assert!(!LedgerScope::new("t", "k").is_unattributed());
    }

    #[test]
    fn reservation_id_round_trips_and_rejects_non_uuid() {
        let id = ReservationId::new_v7();
        let parsed = ReservationId::parse(id.as_str()).expect("valid uuid round-trips");
        assert_eq!(id, parsed);
        assert!(matches!(
            ReservationId::parse(""),
            Err(ReservationIdError::Empty)
        ));
        assert!(matches!(
            ReservationId::parse("not-a-uuid"),
            Err(ReservationIdError::InvalidUuid { .. })
        ));
    }

    #[test]
    fn ledger_entry_kind_round_trips_through_as_str() {
        for kind in [
            LedgerEntryKind::Reserve,
            LedgerEntryKind::Settle,
            LedgerEntryKind::Release,
        ] {
            assert_eq!(LedgerEntryKind::parse_str(kind.as_str()), Some(kind));
        }
        assert_eq!(LedgerEntryKind::parse_str("bogus"), None);
    }

    #[test]
    fn spend_group_by_labels() {
        assert_eq!(SpendGroupBy::Tenant.as_str(), "tenant");
        assert_eq!(SpendGroupBy::ApiKey.as_str(), "api-key");
        assert_eq!(SpendGroupBy::Run.as_str(), "run");
        assert_eq!(SpendGroupBy::Model.as_str(), "model");
        assert_eq!(SpendGroupBy::default(), SpendGroupBy::Tenant);
    }

    #[test]
    fn format_cost_matches_the_herald_cost_display() {
        for (nanos, expected) in [
            (45_000_000_i64, "0.0450 USD"),
            (1_500_000_i64, "0.0015 USD"),
            (0_i64, "0.0000 USD"),
        ] {
            let cost = Cost::new(nanos, usd());
            assert_eq!(format_cost(&cost), expected);

            let metadata = ExecutionMetadata::builder()
                .execution_id(uuid::Uuid::new_v4())
                .start_time(Utc::now())
                .model_used("gpt-4".to_string())
                .token_usage(TokenUsage::new(1_000, 2_000))
                .cost(&cost)
                .build()
                .expect("every required field is set");
            assert_eq!(metadata.cost_display().as_deref(), Some(expected));
        }
    }

    #[test]
    fn settle_request_serde_round_trip() {
        let request = SettleRequest::unreserved(
            LedgerScope::unattributed(),
            SettlementKey::new(RunId::new_v7(), 1, 1),
            Cost::new(45_000_000, usd()),
            BTreeMap::from([("gpt-4".to_string(), 45_000_000_i64)]),
        );
        let json = serde_json::to_string(&request).expect("SettleRequest serializes");
        let round_tripped: SettleRequest =
            serde_json::from_str(&json).expect("SettleRequest deserializes");
        assert_eq!(request, round_tripped);
    }
}
