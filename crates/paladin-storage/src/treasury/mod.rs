//! Treasury ledger storage adapters.
//!
//! Implementations of `paladin_ports::output::treasury_ledger_port::TreasuryLedgerPort`,
//! mirroring `crate::run`'s module layout (D-11). Persisted instants go through
//! `crate::run::storage_timestamp` (reused, not duplicated) before crossing into a Postgres
//! `TIMESTAMPTZ` column.
//!
//! The in-memory backend and the shared `contract_tests` suite arrive in 39-02; this plan
//! (39-01) proves the architecture end-to-end on the SQLite adapter alone.

/// SQLite `TreasuryLedgerPort` implementation, behind the `sqlite` feature (LEDGR-01, D-11).
#[cfg(feature = "sqlite")]
pub mod sqlite;

use paladin_core::platform::container::treasury_ledger::SettleRequest;
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
}
