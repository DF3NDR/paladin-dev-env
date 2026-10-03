//! In-memory `TreasuryLedgerPort` implementation (LEDGR-01..04, ADR-0053, D-04, D-06, D-12).
//!
//! Always compiled -- no feature gate, mirroring `crate::run::in_memory`'s D-01 precedent: used
//! for tests, local development, and any deployment with no configured ledger backend.
//!
//! One `tokio::sync::Mutex<LedgerState>` is held across each method's whole body -- the SUM and
//! the insert for `reserve` (D-12), and the idempotency-set check-then-insert for `settle` -- so
//! this adapter proves the same per-scope serialization guarantee the SQL adapters prove with
//! `BEGIN IMMEDIATE` / `pg_advisory_xact_lock`. `LedgerState::settled` is the in-memory twin of
//! the SQL adapters' partial unique index (D-06): a `HashSet<SettlementKey>` a settle checks and
//! inserts under the same lock as the entries push. Entries are only ever pushed, never modified
//! or removed (ADR-0053 §1, append-only).

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::sync::Mutex;

use paladin_core::platform::container::cost::{Cost, CurrencyCode};
use paladin_core::platform::container::treasury_ledger::{
    BalanceQuery, LedgerEntryKind, LedgerScope, ReservationId, ReserveRequest, SettleOutcome,
    SettleRequest, SettlementKey, SpendGroupBy, SpendQuery, SpendRow,
};
use paladin_ports::output::treasury_ledger_port::{TreasuryLedgerError, TreasuryLedgerPort};

/// One append-only ledger row. Mirrors the SQL adapters' columns closely enough to prove
/// identical port semantics -- not a shared representation with them.
#[derive(Debug, Clone)]
struct Entry {
    kind: LedgerEntryKind,
    scope: LedgerScope,
    reservation: Option<ReservationId>,
    key: Option<SettlementKey>,
    amount_nanos: i64,
    charged_nanos: i64,
    currency: CurrencyCode,
    model_breakdown: BTreeMap<String, i64>,
    attributed_at: DateTime<Utc>,
}

#[derive(Debug, Default)]
struct LedgerState {
    entries: Vec<Entry>,
    settled: HashSet<SettlementKey>,
}

/// In-memory `TreasuryLedgerPort` implementation (LEDGR-01, always available, no feature gate).
///
/// Cloning shares the underlying store (the `Arc<Mutex<..>>` is shared, not the state copied),
/// exactly like every other in-memory adapter in this crate.
///
/// # Examples
///
/// ```
/// use paladin_ports::output::treasury_ledger_port::TreasuryLedgerPort;
/// use paladin_storage::treasury::in_memory::InMemoryTreasuryLedger;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let ledger = InMemoryTreasuryLedger::new();
/// let _now = ledger.store_now().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Default)]
pub struct InMemoryTreasuryLedger {
    state: Arc<Mutex<LedgerState>>,
}

impl InMemoryTreasuryLedger {
    /// Construct a fresh, empty ledger.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl TreasuryLedgerPort for InMemoryTreasuryLedger {
    async fn reserve(&self, request: ReserveRequest) -> Result<ReservationId, TreasuryLedgerError> {
        crate::treasury::validate_reserve(&request)?;

        let mut state = self.state.lock().await;

        let foreign = state.entries.iter().find(|e| {
            e.scope == request.scope
                && e.attributed_at >= request.window_start
                && e.attributed_at < request.window_end
                && e.currency != *request.hold.currency()
        });
        if let Some(entry) = foreign {
            return Err(TreasuryLedgerError::CurrencyMismatch {
                expected: request.hold.currency().clone(),
                found: entry.currency.clone(),
            });
        }

        let balance: i64 = state
            .entries
            .iter()
            .filter(|e| {
                e.scope == request.scope
                    && e.currency == *request.hold.currency()
                    && e.attributed_at >= request.window_start
                    && e.attributed_at < request.window_end
            })
            .fold(0i64, |acc, e| acc.saturating_add(e.amount_nanos));

        let hold_nanos = request.hold.nanos();
        let ceiling_nanos = request.ceiling.nanos();
        let admitted = balance
            .checked_add(hold_nanos)
            .is_some_and(|sum| sum <= ceiling_nanos);

        if !admitted {
            return Err(TreasuryLedgerError::Refused {
                balance: Cost::new(balance, request.hold.currency().clone()),
                hold: request.hold.clone(),
                ceiling: request.ceiling.clone(),
            });
        }

        let attributed_at = crate::run::storage_timestamp(Utc::now());
        let reservation_id = ReservationId::new_v7();

        state.entries.push(Entry {
            kind: LedgerEntryKind::Reserve,
            scope: request.scope,
            reservation: Some(reservation_id.clone()),
            key: request.key,
            amount_nanos: hold_nanos,
            charged_nanos: 0,
            currency: request.hold.currency().clone(),
            model_breakdown: BTreeMap::new(),
            attributed_at,
        });

        Ok(reservation_id)
    }

    async fn release(&self, reservation: ReservationId) -> Result<(), TreasuryLedgerError> {
        let mut state = self.state.lock().await;

        let Some(reserve_entry) = state
            .entries
            .iter()
            .find(|e| {
                e.kind == LedgerEntryKind::Reserve && e.reservation.as_ref() == Some(&reservation)
            })
            .cloned()
        else {
            return Err(TreasuryLedgerError::UnknownReservation { reservation });
        };

        let closed = state.entries.iter().any(|e| {
            matches!(e.kind, LedgerEntryKind::Settle | LedgerEntryKind::Release)
                && e.reservation.as_ref() == Some(&reservation)
        });
        if closed {
            // Already settled or already released: a no-op, not an error (D-04).
            return Ok(());
        }

        state.entries.push(Entry {
            kind: LedgerEntryKind::Release,
            scope: reserve_entry.scope,
            reservation: Some(reservation),
            key: None,
            amount_nanos: -reserve_entry.amount_nanos,
            charged_nanos: 0,
            currency: reserve_entry.currency,
            model_breakdown: BTreeMap::new(),
            attributed_at: reserve_entry.attributed_at,
        });

        Ok(())
    }

    async fn settle(&self, request: SettleRequest) -> Result<SettleOutcome, TreasuryLedgerError> {
        crate::treasury::validate_settle(&request)?;

        let mut state = self.state.lock().await;

        if let Some(reservation) = &request.reservation {
            let Some(reserve_entry) = state
                .entries
                .iter()
                .find(|e| {
                    e.kind == LedgerEntryKind::Reserve
                        && e.reservation.as_ref() == Some(reservation)
                })
                .cloned()
            else {
                return Err(TreasuryLedgerError::UnknownReservation {
                    reservation: reservation.clone(),
                });
            };

            if reserve_entry.scope != request.scope {
                return Err(TreasuryLedgerError::InvalidRequest {
                    message: format!(
                        "settle scope {:?}/{:?} does not match reservation {reservation}'s scope \
                         {:?}/{:?}",
                        request.scope.tenant_id,
                        request.scope.api_key_id,
                        reserve_entry.scope.tenant_id,
                        reserve_entry.scope.api_key_id
                    ),
                });
            }
            if reserve_entry.currency != *request.amount.currency() {
                return Err(TreasuryLedgerError::CurrencyMismatch {
                    expected: reserve_entry.currency,
                    found: request.amount.currency().clone(),
                });
            }

            if state.settled.contains(&request.key) {
                return Ok(SettleOutcome::AlreadySettled);
            }
            state.settled.insert(request.key.clone());

            let closed = state.entries.iter().any(|e| {
                matches!(e.kind, LedgerEntryKind::Settle | LedgerEntryKind::Release)
                    && e.reservation.as_ref() == Some(reservation)
            });
            let outstanding = if closed {
                0
            } else {
                reserve_entry.amount_nanos
            };
            let actual_nanos = request.amount.nanos();

            state.entries.push(Entry {
                kind: LedgerEntryKind::Settle,
                scope: request.scope,
                reservation: Some(reservation.clone()),
                key: Some(request.key),
                amount_nanos: actual_nanos - outstanding,
                charged_nanos: actual_nanos,
                currency: request.amount.currency().clone(),
                model_breakdown: request.model_breakdown,
                attributed_at: reserve_entry.attributed_at,
            });

            Ok(SettleOutcome::Settled)
        } else {
            if state.settled.contains(&request.key) {
                return Ok(SettleOutcome::AlreadySettled);
            }
            state.settled.insert(request.key.clone());

            let attributed_at = crate::run::storage_timestamp(Utc::now());
            let actual_nanos = request.amount.nanos();

            state.entries.push(Entry {
                kind: LedgerEntryKind::Settle,
                scope: request.scope,
                reservation: None,
                key: Some(request.key),
                amount_nanos: actual_nanos,
                charged_nanos: actual_nanos,
                currency: request.amount.currency().clone(),
                model_breakdown: request.model_breakdown,
                attributed_at,
            });

            Ok(SettleOutcome::Settled)
        }
    }

    async fn spend(&self, query: SpendQuery) -> Result<Vec<SpendRow>, TreasuryLedgerError> {
        let state = self.state.lock().await;

        // Keyed by (group value, currency code) -- never combining two currencies into one
        // figure (D-09). The u64 is the settlement count folded into this (group, currency).
        let mut folded: BTreeMap<(String, String), (i64, u64)> = BTreeMap::new();

        for entry in state
            .entries
            .iter()
            .filter(|e| e.kind == LedgerEntryKind::Settle)
        {
            if let Some(since) = query.since
                && entry.attributed_at < since
            {
                continue;
            }
            if let Some(until) = query.until
                && entry.attributed_at >= until
            {
                continue;
            }
            if let Some(tenant_id) = &query.tenant_id
                && &entry.scope.tenant_id != tenant_id
            {
                continue;
            }
            if let Some(api_key_id) = &query.api_key_id
                && &entry.scope.api_key_id != api_key_id
            {
                continue;
            }
            if !query.run_ids.is_empty() {
                let matches = entry
                    .key
                    .as_ref()
                    .is_some_and(|k| query.run_ids.contains(&k.run_id));
                if !matches {
                    continue;
                }
            }

            let currency = entry.currency.as_str().to_string();
            match query.group_by {
                SpendGroupBy::Tenant => fold_one(
                    &mut folded,
                    entry.scope.tenant_id.clone(),
                    currency,
                    entry.charged_nanos,
                ),
                SpendGroupBy::ApiKey => fold_one(
                    &mut folded,
                    entry.scope.api_key_id.clone(),
                    currency,
                    entry.charged_nanos,
                ),
                SpendGroupBy::Run => {
                    let run_id = entry
                        .key
                        .as_ref()
                        .map(|k| k.run_id.as_str().to_string())
                        .unwrap_or_default();
                    fold_one(&mut folded, run_id, currency, entry.charged_nanos);
                }
                SpendGroupBy::Model => {
                    for (model, nanos) in &entry.model_breakdown {
                        fold_one(&mut folded, model.clone(), currency.clone(), *nanos);
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
        Ok(crate::run::storage_timestamp(Utc::now()))
    }

    async fn balance(&self, query: BalanceQuery) -> Result<Cost, TreasuryLedgerError> {
        crate::treasury::validate_balance(&query)?;

        let state = self.state.lock().await;

        // One predicate for the foreign-currency probe and the SUM, so both see the same rows.
        let in_scope = |e: &&Entry| {
            e.scope.tenant_id == query.tenant_id
                && query
                    .api_key_id
                    .as_ref()
                    .is_none_or(|key| &e.scope.api_key_id == key)
                && query.since.is_none_or(|since| e.attributed_at >= since)
                && query.until.is_none_or(|until| e.attributed_at < until)
        };

        if let Some(entry) = state
            .entries
            .iter()
            .filter(in_scope)
            .find(|e| e.currency != query.currency)
        {
            return Err(TreasuryLedgerError::CurrencyMismatch {
                expected: query.currency,
                found: entry.currency.clone(),
            });
        }

        let sum = state
            .entries
            .iter()
            .filter(in_scope)
            .fold(0i64, |acc, e| acc.saturating_add(e.amount_nanos));
        Ok(Cost::new(sum, query.currency))
    }
}

/// Fold one (group, currency, nanos) contribution into `folded`, incrementing that entry's
/// settlement count by one. Mirrors `sqlite::fold_one` exactly (D-11: identical semantics on
/// every adapter).
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
mod contract_suite {
    use std::sync::Arc;

    use paladin_ports::output::treasury_ledger_port::TreasuryLedgerPort;

    use super::InMemoryTreasuryLedger;
    use crate::treasury::contract_tests;

    fn fresh_store() -> InMemoryTreasuryLedger {
        InMemoryTreasuryLedger::new()
    }

    // ── Task 1 clauses ────────────────────────────────────────────────────

    #[tokio::test]
    async fn reserve_admits_at_the_ceiling_and_refuses_one_past_it() {
        contract_tests::reserve_admits_at_the_ceiling_and_refuses_one_past_it(&fresh_store()).await;
    }

    #[tokio::test]
    async fn reserve_then_settle_contributes_actual_minus_hold() {
        contract_tests::reserve_then_settle_contributes_actual_minus_hold(&fresh_store()).await;
    }

    #[tokio::test]
    async fn unreserved_settle_contributes_actual() {
        contract_tests::unreserved_settle_contributes_actual(&fresh_store()).await;
    }

    #[tokio::test]
    async fn release_returns_the_hold_and_is_idempotent() {
        contract_tests::release_returns_the_hold_and_is_idempotent(&fresh_store()).await;
    }

    #[tokio::test]
    async fn settle_after_release_charges_actual_only() {
        contract_tests::settle_after_release_charges_actual_only(&fresh_store()).await;
    }

    #[tokio::test]
    async fn settle_is_attributed_to_its_reservation_window() {
        contract_tests::settle_is_attributed_to_its_reservation_window(&fresh_store()).await;
    }

    #[tokio::test]
    async fn reserve_refuses_a_second_currency_in_scope_and_window() {
        contract_tests::reserve_refuses_a_second_currency_in_scope_and_window(&fresh_store()).await;
    }

    #[tokio::test]
    async fn reserve_rejects_invalid_requests() {
        contract_tests::reserve_rejects_invalid_requests(&fresh_store()).await;
    }

    #[tokio::test]
    async fn two_reservations_for_one_superstep_attempt_are_legal() {
        contract_tests::two_reservations_for_one_superstep_attempt_are_legal(&fresh_store()).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn reserve_race_admits_exactly_n_minus_one() {
        let store: Arc<dyn TreasuryLedgerPort> = Arc::new(fresh_store());
        contract_tests::reserve_race_admits_exactly_n_minus_one(store).await;
    }

    // ── Task 2 clauses ────────────────────────────────────────────────────

    #[tokio::test]
    async fn duplicate_settle_is_already_settled_and_charges_once() {
        contract_tests::duplicate_settle_is_already_settled_and_charges_once(&fresh_store()).await;
    }

    #[tokio::test]
    async fn bumped_attempt_is_a_distinct_settlement() {
        contract_tests::bumped_attempt_is_a_distinct_settlement(&fresh_store()).await;
    }

    #[tokio::test]
    async fn spend_groups_by_every_dimension_over_a_window() {
        contract_tests::spend_groups_by_every_dimension_over_a_window(&fresh_store()).await;
    }

    #[tokio::test]
    async fn spend_orders_groups_then_currencies_ascending() {
        contract_tests::spend_orders_groups_then_currencies_ascending(&fresh_store()).await;
    }

    #[tokio::test]
    async fn spend_over_an_empty_window_is_empty() {
        contract_tests::spend_over_an_empty_window_is_empty(&fresh_store()).await;
    }

    #[tokio::test]
    async fn spend_window_is_half_open() {
        contract_tests::spend_window_is_half_open(&fresh_store()).await;
    }

    #[tokio::test]
    async fn spend_splits_currencies_into_separate_rows() {
        contract_tests::spend_splits_currencies_into_separate_rows(&fresh_store()).await;
    }

    #[tokio::test]
    async fn settle_rejects_a_breakdown_that_does_not_sum_to_the_amount() {
        contract_tests::settle_rejects_a_breakdown_that_does_not_sum_to_the_amount(&fresh_store())
            .await;
    }

    #[tokio::test]
    async fn store_now_is_non_decreasing() {
        contract_tests::store_now_is_non_decreasing(&fresh_store()).await;
    }

    #[tokio::test]
    async fn unattributed_scope_is_grouped_under_the_sentinel() {
        contract_tests::unattributed_scope_is_grouped_under_the_sentinel(&fresh_store()).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn concurrent_duplicate_settles_charge_once() {
        let store: Arc<dyn TreasuryLedgerPort> = Arc::new(fresh_store());
        contract_tests::concurrent_duplicate_settles_charge_once(store).await;
    }

    // ── Phase 41 (41-01) balance clause ───────────────────────────────────

    #[tokio::test]
    async fn tenant_balance_equals_sum_of_key_balances() {
        contract_tests::tenant_balance_equals_sum_of_key_balances(&fresh_store()).await;
    }

    #[tokio::test]
    async fn balance_sums_signed_contributions_in_window() {
        contract_tests::balance_sums_signed_contributions_in_window(&fresh_store()).await;
    }

    #[tokio::test]
    async fn key_balance_excludes_other_keys_and_tenants() {
        contract_tests::key_balance_excludes_other_keys_and_tenants(&fresh_store()).await;
    }

    #[tokio::test]
    async fn balance_window_is_half_open() {
        contract_tests::balance_window_is_half_open(&fresh_store()).await;
    }

    #[tokio::test]
    async fn balance_unbounded_counts_every_row() {
        contract_tests::balance_unbounded_counts_every_row(&fresh_store()).await;
    }

    #[tokio::test]
    async fn balance_mixed_currency_is_currency_mismatch() {
        contract_tests::balance_mixed_currency_is_currency_mismatch(&fresh_store()).await;
    }

    #[tokio::test]
    async fn balance_of_empty_scope_is_zero_in_requested_currency() {
        contract_tests::balance_of_empty_scope_is_zero_in_requested_currency(&fresh_store()).await;
    }

    #[tokio::test]
    async fn balance_rejects_an_invalid_query() {
        contract_tests::balance_rejects_an_invalid_query(&fresh_store()).await;
    }

    #[tokio::test]
    async fn balance_is_read_only() {
        contract_tests::balance_is_read_only(&fresh_store()).await;
    }

    // ── Exact-instant window edges (adapter-local, 41-02) ─────────────────

    /// Rows attributed exactly at `window_start` and exactly at `window_end` land in the window
    /// that starts, respectively ends, at that instant -- half-open, no gap and no overlap.
    #[tokio::test]
    async fn balance_counts_a_row_at_window_start_and_excludes_one_at_window_end() {
        use chrono::{DateTime, Utc};
        use paladin_core::platform::container::cost::{Cost, CurrencyCode};
        use paladin_core::platform::container::treasury_ledger::{
            BalanceQuery, LedgerEntryKind, LedgerScope,
        };

        use super::Entry;

        let usd = CurrencyCode::new("USD").unwrap();
        let ws = "2026-01-01T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let we = ws + chrono::Duration::hours(1);
        let scope = LedgerScope::new("edge-tenant", "edge-key");

        let store = fresh_store();
        {
            let mut state = store.state.lock().await;
            for (nanos, attributed_at) in [(3, ws), (5, we)] {
                state.entries.push(Entry {
                    kind: LedgerEntryKind::Reserve,
                    scope: scope.clone(),
                    reservation: None,
                    key: None,
                    amount_nanos: nanos,
                    charged_nanos: 0,
                    currency: usd.clone(),
                    model_breakdown: std::collections::BTreeMap::new(),
                    attributed_at,
                });
            }
        }

        let query = |since: DateTime<Utc>, until: DateTime<Utc>| BalanceQuery {
            tenant_id: "edge-tenant".to_string(),
            api_key_id: Some("edge-key".to_string()),
            currency: usd.clone(),
            since: Some(since),
            until: Some(until),
        };

        assert_eq!(
            store.balance(query(ws, we)).await.unwrap(),
            Cost::new(3, usd.clone()),
            "[ws, we) must count the row at ws and exclude the row at we"
        );
        assert_eq!(
            store
                .balance(query(we, we + chrono::Duration::hours(1)))
                .await
                .unwrap(),
            Cost::new(5, usd),
            "[we, we + 1h) must count the row at we and not the row at ws"
        );
    }
}
