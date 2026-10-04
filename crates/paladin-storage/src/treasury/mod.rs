//! Treasury ledger storage adapters.
//!
//! Implementations of `paladin_ports::output::treasury_ledger_port::TreasuryLedgerPort`,
//! mirroring `crate::run`'s module layout (D-11). Persisted instants go through
//! `crate::run::storage_timestamp` (reused, not duplicated) before crossing into a Postgres
//! `TIMESTAMPTZ` column.
//!
//! This plan (39-02) completes the shared contract suite (`reserve`/`release`, idempotency,
//! spend windows/grouping/ordering, validation, store clock) and adds `InMemoryTreasuryLedger`,
//! proving every clause identically on the in-memory and SQLite adapters. The Postgres adapter
//! follows in 39-03. Phase 41 (41-01) adds the admission-time `balance` read to the in-memory and
//! SQLite adapters (the PostgreSQL override follows in 41-02).

/// In-memory `TreasuryLedgerPort` implementation, always available (no feature gate, mirroring
/// `crate::run::in_memory`'s D-01 precedent).
pub mod in_memory;

/// Shared `TreasuryNoticePort` contract suite (ALLOW-04, D-16): one generic async function per
/// notice clause, invoked unchanged by every backend's own `#[tokio::test]`s. Plain module (not
/// `#[cfg(test)]`), registered next to [`contract_tests`].
pub mod notice_contract_tests;

/// Shared `TreasuryLedgerPort` contract suite (D-11): one generic async function per clause,
/// invoked unchanged by every backend's own `#[tokio::test]`s. Plain module (not
/// `#[cfg(test)]`), mirroring `crate::run::contract_tests`.
pub mod contract_tests;

/// SQLite `TreasuryLedgerPort` implementation, behind the `sqlite` feature (LEDGR-01, D-11).
#[cfg(feature = "sqlite")]
pub mod sqlite;

/// PostgreSQL `TreasuryLedgerPort` implementation, behind the `postgres` feature (LEDGR-01,
/// D-11, D-12, ADR-0053 §5).
#[cfg(feature = "postgres")]
pub mod postgres;

use paladin_core::platform::container::allowance::{
    AllowanceLimitKind, AllowanceScopeKind, NoticeRecord,
};
use paladin_core::platform::container::treasury_ledger::{
    BalanceQuery, ReserveRequest, SettleRequest,
};
use paladin_ports::output::treasury_ledger_port::TreasuryLedgerError;

/// Shared, backend-agnostic validation every adapter's `settle` runs before any I/O (D-00e's
/// "validate before touching the backend" convention).
///
/// # Errors
///
/// Returns [`TreasuryLedgerError::InvalidRequest`] when:
/// - `request.scope.tenant_id` or `request.scope.api_key_id` is empty.
/// - `request.amount.nanos()` is negative.
/// - any `model_breakdown` key is empty, or any value is negative.
/// - `model_breakdown`'s values (summed with overflow checking) do not equal
///   `request.amount.nanos()` — this also covers an empty breakdown paired with a non-zero
///   amount, since an empty sum is `0`.
pub(crate) fn validate_settle(request: &SettleRequest) -> Result<(), TreasuryLedgerError> {
    if request.scope.tenant_id.trim().is_empty() {
        return Err(TreasuryLedgerError::InvalidRequest {
            message: "settle request scope.tenant_id must not be empty".to_string(),
        });
    }
    if request.scope.api_key_id.trim().is_empty() {
        return Err(TreasuryLedgerError::InvalidRequest {
            message: "settle request scope.api_key_id must not be empty".to_string(),
        });
    }
    if request.amount.nanos() < 0 {
        return Err(TreasuryLedgerError::InvalidRequest {
            message: format!(
                "settle request amount must not be negative (got {} nanos)",
                request.amount.nanos()
            ),
        });
    }

    let mut breakdown_total: i64 = 0;
    for (model, nanos) in &request.model_breakdown {
        if model.trim().is_empty() {
            return Err(TreasuryLedgerError::InvalidRequest {
                message: "settle request model_breakdown must not contain an empty model name"
                    .to_string(),
            });
        }
        if *nanos < 0 {
            return Err(TreasuryLedgerError::InvalidRequest {
                message: format!(
                    "settle request model_breakdown['{model}'] must not be negative (got {nanos} nanos)"
                ),
            });
        }
        breakdown_total = breakdown_total.checked_add(*nanos).ok_or_else(|| {
            TreasuryLedgerError::InvalidRequest {
                message: "settle request model_breakdown overflowed while summing".to_string(),
            }
        })?;
    }

    if breakdown_total != request.amount.nanos() {
        return Err(TreasuryLedgerError::InvalidRequest {
            message: format!(
                "settle request model_breakdown sums to {breakdown_total} nanos, but amount is \
                 {amount} nanos",
                amount = request.amount.nanos()
            ),
        });
    }

    Ok(())
}

/// Shared, backend-agnostic validation every adapter's `reserve` runs before any I/O (D-00e).
///
/// # Errors
///
/// Returns [`TreasuryLedgerError::InvalidRequest`] when:
/// - `request.scope.tenant_id` or `request.scope.api_key_id` is empty.
/// - `request.hold.nanos()` or `request.ceiling.nanos()` is negative.
/// - `request.window_start >= request.window_end`.
///
/// Returns [`TreasuryLedgerError::CurrencyMismatch`] when `request.hold`'s currency differs
/// from `request.ceiling`'s.
pub(crate) fn validate_reserve(request: &ReserveRequest) -> Result<(), TreasuryLedgerError> {
    if request.scope.tenant_id.trim().is_empty() {
        return Err(TreasuryLedgerError::InvalidRequest {
            message: "reserve request scope.tenant_id must not be empty".to_string(),
        });
    }
    if request.scope.api_key_id.trim().is_empty() {
        return Err(TreasuryLedgerError::InvalidRequest {
            message: "reserve request scope.api_key_id must not be empty".to_string(),
        });
    }
    if request.hold.nanos() < 0 {
        return Err(TreasuryLedgerError::InvalidRequest {
            message: format!(
                "reserve request hold must not be negative (got {} nanos)",
                request.hold.nanos()
            ),
        });
    }
    if request.ceiling.nanos() < 0 {
        return Err(TreasuryLedgerError::InvalidRequest {
            message: format!(
                "reserve request ceiling must not be negative (got {} nanos)",
                request.ceiling.nanos()
            ),
        });
    }
    if request.window_start >= request.window_end {
        return Err(TreasuryLedgerError::InvalidRequest {
            message: "reserve request window_start must be before window_end".to_string(),
        });
    }
    if request.hold.currency() != request.ceiling.currency() {
        return Err(TreasuryLedgerError::CurrencyMismatch {
            expected: request.hold.currency().clone(),
            found: request.ceiling.currency().clone(),
        });
    }

    Ok(())
}

/// Shared, backend-agnostic validation every adapter's `balance` runs before any I/O (D-00e).
///
/// # Errors
///
/// Returns [`TreasuryLedgerError::InvalidRequest`] when:
/// - `query.tenant_id` is empty.
/// - `query.api_key_id` is `Some("")`.
/// - both bounds are present and `since >= until`.
pub(crate) fn validate_balance(query: &BalanceQuery) -> Result<(), TreasuryLedgerError> {
    if query.tenant_id.trim().is_empty() {
        return Err(TreasuryLedgerError::InvalidRequest {
            message: "balance query tenant_id must not be empty".to_string(),
        });
    }
    if query
        .api_key_id
        .as_deref()
        .is_some_and(|key| key.trim().is_empty())
    {
        return Err(TreasuryLedgerError::InvalidRequest {
            message: "balance query api_key_id must not be empty when present".to_string(),
        });
    }
    if let (Some(since), Some(until)) = (query.since, query.until)
        && since >= until
    {
        return Err(TreasuryLedgerError::InvalidRequest {
            message: "balance query since must be before until".to_string(),
        });
    }
    Ok(())
}

/// Shared, backend-agnostic validation every adapter's `TreasuryNoticePort::record` runs before
/// any I/O (D-00e).
///
/// # Errors
///
/// Returns [`TreasuryLedgerError::InvalidRequest`] when:
/// - `notice_id` or `tenant_id` is empty.
/// - an `ApiKey`-scope notice has no (or an empty) `api_key_id`.
/// - a `Tenant`-scope notice carries an `api_key_id`.
/// - `warning.warn_at` is above 100.
/// - a `Window` notice lacks either window bound, or a `Lifetime` notice carries one.
/// - the ceiling's and the balance's currencies differ.
pub(crate) fn validate_notice(notice: &NoticeRecord) -> Result<(), TreasuryLedgerError> {
    if notice.notice_id.trim().is_empty() {
        return Err(TreasuryLedgerError::InvalidRequest {
            message: "notice notice_id must not be empty".to_string(),
        });
    }
    if notice.tenant_id.trim().is_empty() {
        return Err(TreasuryLedgerError::InvalidRequest {
            message: "notice tenant_id must not be empty".to_string(),
        });
    }
    match (notice.warning.scope_kind, notice.api_key_id.as_deref()) {
        (AllowanceScopeKind::ApiKey, None) => {
            return Err(TreasuryLedgerError::InvalidRequest {
                message: "an api_key-scope notice requires an api_key_id".to_string(),
            });
        }
        (AllowanceScopeKind::ApiKey, Some(key)) if key.trim().is_empty() => {
            return Err(TreasuryLedgerError::InvalidRequest {
                message: "an api_key-scope notice requires a non-empty api_key_id".to_string(),
            });
        }
        (AllowanceScopeKind::Tenant, Some(_)) => {
            return Err(TreasuryLedgerError::InvalidRequest {
                message: "a tenant-scope notice must not carry an api_key_id".to_string(),
            });
        }
        _ => {}
    }
    if notice.warning.warn_at > 100 {
        return Err(TreasuryLedgerError::InvalidRequest {
            message: format!(
                "notice warn_at must be at most 100 (got {})",
                notice.warning.warn_at
            ),
        });
    }
    let has_window = notice.warning.window_start.is_some() && notice.warning.window_end.is_some();
    let has_no_window =
        notice.warning.window_start.is_none() && notice.warning.window_end.is_none();
    match notice.warning.limit_kind {
        AllowanceLimitKind::Window if !has_window => {
            return Err(TreasuryLedgerError::InvalidRequest {
                message: "a window notice requires both window_start and window_end".to_string(),
            });
        }
        AllowanceLimitKind::Lifetime if !has_no_window => {
            return Err(TreasuryLedgerError::InvalidRequest {
                message: "a lifetime notice must not carry a window".to_string(),
            });
        }
        _ => {}
    }
    if notice.warning.ceiling.currency() != notice.warning.balance.currency() {
        return Err(TreasuryLedgerError::InvalidRequest {
            message: format!(
                "notice ceiling currency {} does not match balance currency {}",
                notice.warning.ceiling.currency(),
                notice.warning.balance.currency()
            ),
        });
    }
    Ok(())
}

/// SQL-adapter row decoding for `treasury_notices`, compiled only when a SQL backend is.
#[cfg(any(feature = "sqlite", feature = "postgres"))]
mod notice_row {
    use chrono::{DateTime, Utc};
    use paladin_core::platform::container::allowance::{
        AllowanceLimitKind, AllowanceScopeKind, AllowanceWarning, NoticeRecord,
    };
    use paladin_core::platform::container::cost::{Cost, CurrencyCode};
    use paladin_core::platform::container::run::RunId;
    use paladin_ports::output::treasury_ledger_port::TreasuryLedgerError;

    /// One `treasury_notices` row as a SQL adapter reads it, before it is turned back into a
    /// [`NoticeRecord`]. Shared by the SQLite and PostgreSQL adapters so the `''` and epoch
    /// sentinel mapping (C5) lives in exactly one place.
    #[derive(Debug)]
    pub(crate) struct RawNotice {
        pub notice_id: String,
        pub scope_kind: String,
        pub tenant_id: String,
        pub api_key_id: String,
        pub limit_kind: String,
        pub window_start: DateTime<Utc>,
        pub window_end: Option<DateTime<Utc>>,
        pub ceiling_nanos: i64,
        pub currency: String,
        pub balance_nanos: i64,
        pub warn_at: i64,
        pub run_id: Option<String>,
        pub recorded_at: DateTime<Utc>,
    }

    impl RawNotice {
        /// Rebuild the [`NoticeRecord`]: `''` becomes `api_key_id: None`, and a `lifetime` row
        /// reads back with both window bounds `None` (its stored `window_start` is only the epoch
        /// identity sentinel).
        ///
        /// # Errors
        ///
        /// Returns [`TreasuryLedgerError::Serialization`] when a stored column holds a value this
        /// adapter could never have written (an unknown kind, an invalid currency or run id, a
        /// `warn_at` outside `0..=100`).
        pub(crate) fn into_record(self) -> Result<NoticeRecord, TreasuryLedgerError> {
            let bad = |what: &str, value: &str| TreasuryLedgerError::Serialization {
                message: format!("stored notice {what} '{value}' is not valid"),
            };
            let scope_kind = match self.scope_kind.as_str() {
                "tenant" => AllowanceScopeKind::Tenant,
                "api_key" => AllowanceScopeKind::ApiKey,
                other => return Err(bad("scope_kind", other)),
            };
            let limit_kind = match self.limit_kind.as_str() {
                "window" => AllowanceLimitKind::Window,
                "lifetime" => AllowanceLimitKind::Lifetime,
                other => return Err(bad("limit_kind", other)),
            };
            let currency =
                CurrencyCode::new(&self.currency).map_err(|_| bad("currency", &self.currency))?;
            let warn_at = u8::try_from(self.warn_at)
                .ok()
                .filter(|w| *w <= 100)
                .ok_or_else(|| bad("warn_at", &self.warn_at.to_string()))?;
            let run_id = self
                .run_id
                .map(|raw| RunId::parse(raw.as_str()).map_err(|_| bad("run_id", &raw)))
                .transpose()?;
            let (window_start, window_end) = match limit_kind {
                AllowanceLimitKind::Window => (Some(self.window_start), self.window_end),
                AllowanceLimitKind::Lifetime => (None, None),
            };
            Ok(NoticeRecord {
                notice_id: self.notice_id,
                tenant_id: self.tenant_id,
                api_key_id: Some(self.api_key_id).filter(|key| !key.is_empty()),
                warning: AllowanceWarning {
                    scope_kind,
                    limit_kind,
                    balance: Cost::new(self.balance_nanos, currency.clone()),
                    ceiling: Cost::new(self.ceiling_nanos, currency),
                    window_start,
                    window_end,
                    warn_at,
                },
                run_id,
                recorded_at: self.recorded_at,
            })
        }
    }
}

#[cfg(any(feature = "sqlite", feature = "postgres"))]
pub(crate) use notice_row::RawNotice;

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::cost::{Cost, CurrencyCode};
    use paladin_core::platform::container::run::RunId;
    use paladin_core::platform::container::treasury_ledger::{
        LedgerScope, SettleRequest, SettlementKey,
    };
    use std::collections::BTreeMap;

    fn usd() -> CurrencyCode {
        CurrencyCode::new("USD").unwrap()
    }

    fn base_key() -> SettlementKey {
        SettlementKey::new(RunId::new_v7(), 1, 1)
    }

    #[test]
    fn accepts_a_well_formed_unreserved_settle() {
        let request = SettleRequest::unreserved(
            LedgerScope::unattributed(),
            base_key(),
            Cost::new(45_000_000, usd()),
            BTreeMap::from([("gpt-4".to_string(), 45_000_000_i64)]),
        );
        assert!(validate_settle(&request).is_ok());
    }

    #[test]
    fn accepts_zero_amount_with_empty_breakdown() {
        let request = SettleRequest::unreserved(
            LedgerScope::unattributed(),
            base_key(),
            Cost::new(0, usd()),
            BTreeMap::new(),
        );
        assert!(validate_settle(&request).is_ok());
    }

    #[test]
    fn rejects_empty_tenant_id() {
        let request = SettleRequest::unreserved(
            LedgerScope::new("", "key-1"),
            base_key(),
            Cost::new(0, usd()),
            BTreeMap::new(),
        );
        assert!(matches!(
            validate_settle(&request),
            Err(TreasuryLedgerError::InvalidRequest { .. })
        ));
    }

    #[test]
    fn rejects_empty_api_key_id() {
        let request = SettleRequest::unreserved(
            LedgerScope::new("tenant-1", ""),
            base_key(),
            Cost::new(0, usd()),
            BTreeMap::new(),
        );
        assert!(matches!(
            validate_settle(&request),
            Err(TreasuryLedgerError::InvalidRequest { .. })
        ));
    }

    #[test]
    fn rejects_negative_amount() {
        let request = SettleRequest::unreserved(
            LedgerScope::unattributed(),
            base_key(),
            Cost::new(-1, usd()),
            BTreeMap::new(),
        );
        assert!(matches!(
            validate_settle(&request),
            Err(TreasuryLedgerError::InvalidRequest { .. })
        ));
    }

    #[test]
    fn rejects_empty_breakdown_model_name() {
        let request = SettleRequest::unreserved(
            LedgerScope::unattributed(),
            base_key(),
            Cost::new(1_000, usd()),
            BTreeMap::from([(String::new(), 1_000_i64)]),
        );
        assert!(matches!(
            validate_settle(&request),
            Err(TreasuryLedgerError::InvalidRequest { .. })
        ));
    }

    #[test]
    fn rejects_negative_breakdown_value() {
        let request = SettleRequest::unreserved(
            LedgerScope::unattributed(),
            base_key(),
            Cost::new(0, usd()),
            BTreeMap::from([("gpt-4".to_string(), -1_i64)]),
        );
        assert!(matches!(
            validate_settle(&request),
            Err(TreasuryLedgerError::InvalidRequest { .. })
        ));
    }

    #[test]
    fn rejects_breakdown_not_summing_to_amount() {
        let request = SettleRequest::unreserved(
            LedgerScope::unattributed(),
            base_key(),
            Cost::new(45_000_000, usd()),
            BTreeMap::from([("gpt-4".to_string(), 1_000_i64)]),
        );
        assert!(matches!(
            validate_settle(&request),
            Err(TreasuryLedgerError::InvalidRequest { .. })
        ));
    }

    #[test]
    fn rejects_empty_breakdown_with_non_zero_amount() {
        let request = SettleRequest::unreserved(
            LedgerScope::unattributed(),
            base_key(),
            Cost::new(45_000_000, usd()),
            BTreeMap::new(),
        );
        assert!(matches!(
            validate_settle(&request),
            Err(TreasuryLedgerError::InvalidRequest { .. })
        ));
    }

    fn base_reserve_request() -> ReserveRequest {
        let now = chrono::Utc::now();
        ReserveRequest {
            scope: LedgerScope::unattributed(),
            hold: Cost::new(1, usd()),
            ceiling: Cost::new(10, usd()),
            window_start: now - chrono::Duration::hours(1),
            window_end: now + chrono::Duration::hours(1),
            key: None,
        }
    }

    #[test]
    fn accepts_a_well_formed_reserve_request() {
        assert!(validate_reserve(&base_reserve_request()).is_ok());
    }

    #[test]
    fn reserve_rejects_empty_tenant_id() {
        let mut request = base_reserve_request();
        request.scope = LedgerScope::new("", "key-1");
        assert!(matches!(
            validate_reserve(&request),
            Err(TreasuryLedgerError::InvalidRequest { .. })
        ));
    }

    #[test]
    fn reserve_rejects_empty_api_key_id() {
        let mut request = base_reserve_request();
        request.scope = LedgerScope::new("tenant-1", "");
        assert!(matches!(
            validate_reserve(&request),
            Err(TreasuryLedgerError::InvalidRequest { .. })
        ));
    }

    #[test]
    fn reserve_rejects_negative_hold() {
        let mut request = base_reserve_request();
        request.hold = Cost::new(-1, usd());
        assert!(matches!(
            validate_reserve(&request),
            Err(TreasuryLedgerError::InvalidRequest { .. })
        ));
    }

    #[test]
    fn reserve_rejects_negative_ceiling() {
        let mut request = base_reserve_request();
        request.ceiling = Cost::new(-1, usd());
        assert!(matches!(
            validate_reserve(&request),
            Err(TreasuryLedgerError::InvalidRequest { .. })
        ));
    }

    #[test]
    fn reserve_rejects_a_window_that_does_not_start_before_it_ends() {
        let mut request = base_reserve_request();
        request.window_end = request.window_start;
        assert!(matches!(
            validate_reserve(&request),
            Err(TreasuryLedgerError::InvalidRequest { .. })
        ));

        let mut inverted = base_reserve_request();
        std::mem::swap(&mut inverted.window_start, &mut inverted.window_end);
        assert!(matches!(
            validate_reserve(&inverted),
            Err(TreasuryLedgerError::InvalidRequest { .. })
        ));
    }

    #[test]
    fn reserve_rejects_hold_ceiling_currency_mismatch() {
        let mut request = base_reserve_request();
        request.ceiling = Cost::new(10, CurrencyCode::new("EUR").unwrap());
        assert!(matches!(
            validate_reserve(&request),
            Err(TreasuryLedgerError::CurrencyMismatch { .. })
        ));
    }

    #[test]
    fn balance_rejects_an_invalid_query_before_any_io() {
        use chrono::{Duration, Utc};
        use paladin_core::platform::container::treasury_ledger::BalanceQuery;

        let base = BalanceQuery {
            tenant_id: "acme".to_string(),
            api_key_id: None,
            currency: usd(),
            since: None,
            until: None,
        };
        assert!(validate_balance(&base).is_ok());

        let empty_tenant = BalanceQuery {
            tenant_id: String::new(),
            ..base.clone()
        };
        assert!(matches!(
            validate_balance(&empty_tenant),
            Err(TreasuryLedgerError::InvalidRequest { .. })
        ));

        let empty_key = BalanceQuery {
            api_key_id: Some(String::new()),
            ..base.clone()
        };
        assert!(matches!(
            validate_balance(&empty_key),
            Err(TreasuryLedgerError::InvalidRequest { .. })
        ));

        let now = Utc::now();
        let inverted = BalanceQuery {
            since: Some(now),
            until: Some(now),
            ..base.clone()
        };
        assert!(matches!(
            validate_balance(&inverted),
            Err(TreasuryLedgerError::InvalidRequest { .. })
        ));

        let bounded = BalanceQuery {
            since: Some(now),
            until: Some(now + Duration::seconds(1)),
            ..base
        };
        assert!(validate_balance(&bounded).is_ok());
    }
}
