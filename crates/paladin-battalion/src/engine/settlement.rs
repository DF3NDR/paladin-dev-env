//! Superstep spend accumulation and the boundary settle writer (LEDGR-03,
//! LEDGR-04, D-07, D-08, ADR-0052, ADR-0053 §4).
//!
//! ADR-0052 names the superstep boundary as the one place the engine both
//! settles spend (this module, observational) and decides whether to halt --
//! so [`SpendHook`] is deliberately shaped as the one hook settlement
//! attaches to, at the same boundary, sharing the same ledger and
//! [`SettlementContext`].
//!
//! Phase 42 (ALLOW-03, ADR-0057 D-00b) kept settlement exactly as it is and
//! put the authoritative read beside it: a check-only
//! [`SpendGuard`](paladin_ports::output::spend_guard::SpendGuard), consulted
//! at the top of the same superstep loop, answers whether the run may start
//! the next superstep. [`SpendHook::settle_boundary`] itself is unchanged: it
//! writes spend and never decides anything.
//!
//! Settlement is synchronous and awaited at the superstep boundary, never
//! routed through a [`crate::engine::hooks::TraceDispatcher`]/`TraceSink`:
//! that channel is drop-oldest by contract (`engine::hooks`'s own module
//! docs), so a dropped `NodeFinished` record would silently lose spend --
//! money is the one thing this engine must never treat as best-effort.
//!
//! Crate-private: `paladin-battalion` never re-exports these types outside
//! `engine::mod`/`engine::superstep`. A ledger failure or duplicate-key
//! outcome (D-08, LEDGR-03) is logged and never changes a run's outcome,
//! retries a node, or halts a run -- settlement stays purely observational;
//! the authoritative halt decision belongs to the `SpendGuard` above. A charge
//! that could not be WRITTEN is reported ([`SettleHealth::ChargeLost`]) so the
//! engine can tell an attached guard its balance reads are no longer fed by
//! every write (Phase 42 review WR-4); the guard, not settlement, decides.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

use paladin_core::platform::container::cost::{Cost, CurrencyCode};
use paladin_core::platform::container::treasury_ledger::{
    SettleOutcome, SettleRequest, SettlementContext, SettlementKey,
};
use paladin_ports::output::treasury_ledger_port::TreasuryLedgerPort;

/// Resolve a per-attempt model label to the key a settlement's
/// `model_breakdown` records it under: `None` or an empty string both fold
/// into the literal `"unknown"` key (an attempt with no resolvable model --
/// today, a `NodeSpec::Paladin` node's own `paladin.node.model` is never
/// empty, but a future dispatch kind might not carry one).
fn resolve_model_key(model: Option<&str>) -> &str {
    match model {
        Some(name) if !name.is_empty() => name,
        _ => "unknown",
    }
}

/// One superstep attempt's accumulated priced spend: the sum of every
/// priced attempt's [`Cost`] dispatched in it, and each attempt's model
/// share (D-02). Combined through [`Cost::checked_add`] rather than a bare
/// `+=` so a currency mismatch is caught, never silently folded into a
/// wrong total (T-39-11).
#[derive(Default)]
pub(crate) struct SuperstepSpend {
    total: Option<Cost>,
    by_model: BTreeMap<String, i64>,
    mismatch: Option<(CurrencyCode, CurrencyCode)>,
}

impl SuperstepSpend {
    /// Fold one attempt's cost into this superstep's running total and its
    /// model's own running share. Once a mismatch has been observed, every
    /// further call still updates `by_model` (so `take`'s `CurrencyMismatch`
    /// path never claims a partial breakdown was charged) but the mismatch
    /// itself is never overwritten -- the FIRST disagreement is the one
    /// reported.
    pub(crate) fn record(&mut self, model: &str, cost: &Cost) {
        match self.total.take() {
            None => self.total = Some(cost.clone()),
            Some(current) => match current.checked_add(cost) {
                Some(sum) => self.total = Some(sum),
                None => {
                    if self.mismatch.is_none() {
                        self.mismatch = Some((current.currency().clone(), cost.currency().clone()));
                    }
                    self.total = Some(current);
                }
            },
        }
        let share = self.by_model.entry(model.to_string()).or_insert(0);
        *share = share.saturating_add(cost.nanos());
    }

    /// Take and reset this superstep's accumulated charge (`std::mem::take`)
    /// -- a second call immediately after always returns
    /// [`SuperstepCharge::Nothing`].
    pub(crate) fn take(&mut self) -> SuperstepCharge {
        let taken = std::mem::take(self);
        if let Some((first, other)) = taken.mismatch {
            return SuperstepCharge::CurrencyMismatch { first, other };
        }
        match taken.total {
            None => SuperstepCharge::Nothing,
            Some(amount) => SuperstepCharge::Charge {
                amount,
                model_breakdown: taken.by_model,
            },
        }
    }
}

/// The result of draining a [`SuperstepSpend`] (`take`): what
/// [`SpendHook::settle_boundary`] does with it.
#[derive(Debug)]
pub(crate) enum SuperstepCharge {
    /// No attempt in this superstep was priced -- no settlement row is
    /// written (D-08).
    Nothing,
    /// At least one priced attempt was recorded, with no currency
    /// disagreement -- settle `amount` with this `model_breakdown`.
    Charge {
        /// The superstep's aggregated priced total.
        amount: Cost,
        /// Bare model name -> nano-units; values sum to `amount` (D-02).
        model_breakdown: BTreeMap<String, i64>,
    },
    /// Two different currencies were recorded in the same superstep attempt
    /// -- never combined into one figure (T-39-11); no settlement row is
    /// written, and the disagreement is logged by the caller.
    CurrencyMismatch {
        /// The first currency this superstep saw.
        first: CurrencyCode,
        /// A later, disagreeing currency this superstep also saw.
        other: CurrencyCode,
    },
}

/// Whether a boundary's accumulated charge reached the ledger (Phase 42 review WR-4).
///
/// [`SpendHook::settle_boundary`] still never fails or halts a run itself; it reports this so
/// the engine can tell an attached [`SpendGuard`](paladin_ports::output::spend_guard::SpendGuard)
/// that its balance reads are no longer fed by every write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub(crate) enum SettleHealth {
    /// Nothing was lost: the charge was written, was already settled, there was nothing to
    /// charge, or this is a child hook whose parent settles.
    Healthy,
    /// A priced charge could not be written (a ledger error, or a currency mismatch).
    ChargeLost,
}

/// The per-run hook threaded through the superstep loop (D-08): every
/// dispatched attempt records its own cost via [`SpendHook::record`], and
/// exactly one [`SpendHook::settle_boundary`] call per superstep drains and
/// settles the accumulated charge.
///
/// A [`SpendHook::child`] shares the SAME accumulator as its parent (`sink`
/// is `Arc`-cloned, never re-constructed) but carries no ledger of its own
/// (`ledger: None`) -- so a nested `NodeSpec::Battalion` child run's attempts
/// fold into the PARENT's superstep total, and the child's own
/// `settle_boundary` calls are no-ops (CF-FR-16): only the top-level hook
/// ever settles.
#[derive(Clone)]
pub(crate) struct SpendHook {
    sink: Arc<Mutex<SuperstepSpend>>,
    ledger: Option<(Arc<dyn TreasuryLedgerPort>, SettlementContext)>,
}

impl SpendHook {
    /// Construct a fresh, top-level hook backed by `ledger`/`context`: a
    /// new, empty accumulator, with a ledger attached so its
    /// `settle_boundary` calls actually settle.
    pub(crate) fn top_level(
        ledger: Arc<dyn TreasuryLedgerPort>,
        context: SettlementContext,
    ) -> Self {
        Self {
            sink: Arc::new(Mutex::new(SuperstepSpend::default())),
            ledger: Some((ledger, context)),
        }
    }

    /// Derive a child hook for a nested `NodeSpec::Battalion` run (CF-FR-16):
    /// the SAME shared accumulator, no ledger of its own -- its
    /// `settle_boundary` is a no-op, and the parent's next boundary drains
    /// whatever the child recorded alongside its own attempts. Wired into
    /// production at `superstep.rs`'s `ChildEngineResources::spend`
    /// construction site (plan 39-04 Task 2).
    pub(crate) fn child(&self) -> Self {
        Self {
            sink: Arc::clone(&self.sink),
            ledger: None,
        }
    }

    /// Record one attempt's own cost against this superstep's running total.
    /// `model` resolves through [`resolve_model_key`] -- `None` or an empty
    /// string folds into `"unknown"`. Recovers a poisoned lock with
    /// [`PoisonError::into_inner`] (T-39-16): a prior panicked holder never
    /// permanently loses this run's spend tracking.
    pub(crate) fn record(&self, model: Option<&str>, cost: &Cost) {
        let key = resolve_model_key(model);
        let mut sink = self.sink.lock().unwrap_or_else(PoisonError::into_inner);
        sink.record(key, cost);
    }

    /// Drain this superstep's accumulated charge and settle it, synchronously
    /// (D-08). A child hook (`ledger: None`) returns at once -- the shared
    /// accumulator is left for the top-level hook's own boundary call to
    /// drain. The mutex guard is dropped BEFORE any `.await` (T-39-16): the
    /// lock is held only long enough to call `take`.
    ///
    /// Never returns an error and never panics: an `Err` from the ledger is
    /// logged at `error`, and [`SettleOutcome::AlreadySettled`] is logged at
    /// `warn` -- neither changes this run's outcome, retries a node, or
    /// halts the run (D-08). Log lines never carry `api_key_id`.
    ///
    /// Returns [`SettleHealth::ChargeLost`] when a priced charge could not be written (a ledger
    /// `Err` or a currency mismatch), so the engine can tell an attached spend guard that it is
    /// now blind to this run's spend (Phase 42 review WR-4). Settlement itself stays
    /// observational: the decision to halt belongs to the guard.
    pub(crate) async fn settle_boundary(&self, superstep: u64) -> SettleHealth {
        let Some((ledger, context)) = &self.ledger else {
            return SettleHealth::Healthy;
        };
        let charge = {
            let mut sink = self.sink.lock().unwrap_or_else(PoisonError::into_inner);
            sink.take()
        };
        match charge {
            SuperstepCharge::Nothing => SettleHealth::Healthy,
            SuperstepCharge::CurrencyMismatch { first, other } => {
                log::error!(
                    target: "paladin::treasury",
                    "superstep spend currency mismatch: run {} superstep {} attempt {}: {} vs {} -- no settlement written",
                    context.run_id, superstep, context.attempt, first, other
                );
                SettleHealth::ChargeLost
            }
            SuperstepCharge::Charge {
                amount,
                model_breakdown,
            } => {
                let key = SettlementKey::new(context.run_id.clone(), superstep, context.attempt);
                let request = SettleRequest::unreserved(
                    context.scope.clone(),
                    key,
                    amount.clone(),
                    model_breakdown,
                );
                match ledger.settle(request).await {
                    Ok(SettleOutcome::Settled) => {
                        log::debug!(
                            target: "paladin::treasury",
                            "settled superstep spend: run {} superstep {} attempt {} amount_nanos {} currency {}",
                            context.run_id, superstep, context.attempt, amount.nanos(), amount.currency()
                        );
                        SettleHealth::Healthy
                    }
                    Ok(SettleOutcome::AlreadySettled) => {
                        log::warn!(
                            target: "paladin::treasury",
                            "superstep spend already settled: run {} superstep {} attempt {} -- duplicate not charged again",
                            context.run_id, superstep, context.attempt
                        );
                        SettleHealth::Healthy
                    }
                    Err(e) => {
                        log::error!(
                            target: "paladin::treasury",
                            "failed to settle superstep spend: run {} superstep {} attempt {} amount_nanos {} currency {}: {e}",
                            context.run_id, superstep, context.attempt, amount.nanos(), amount.currency()
                        );
                        SettleHealth::ChargeLost
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::run::RunId;
    use paladin_core::platform::container::treasury_ledger::{
        LedgerScope, ReservationId, ReserveRequest, SpendGroupBy, SpendQuery, SpendRow,
    };
    use paladin_ports::output::treasury_ledger_port::TreasuryLedgerError;
    use paladin_storage::treasury::in_memory::InMemoryTreasuryLedger;

    fn usd() -> CurrencyCode {
        CurrencyCode::new("USD").expect("USD is a valid currency code")
    }

    fn eur() -> CurrencyCode {
        CurrencyCode::new("EUR").expect("EUR is a valid currency code")
    }

    #[test]
    fn record_accumulates_amount_and_per_model_breakdown_then_take_resets() {
        let mut spend = SuperstepSpend::default();
        spend.record("gpt-4", &Cost::new(10, usd()));
        spend.record("gpt-4o-mini", &Cost::new(5, usd()));
        spend.record("gpt-4", &Cost::new(3, usd()));

        match spend.take() {
            SuperstepCharge::Charge {
                amount,
                model_breakdown,
            } => {
                assert_eq!(amount, Cost::new(18, usd()));
                assert_eq!(model_breakdown.get("gpt-4"), Some(&13));
                assert_eq!(model_breakdown.get("gpt-4o-mini"), Some(&5));
            }
            other => panic!("expected a Charge, got {other:?}"),
        }

        assert!(matches!(spend.take(), SuperstepCharge::Nothing));
    }

    #[test]
    fn resolve_model_key_maps_none_and_empty_to_unknown() {
        assert_eq!(resolve_model_key(None), "unknown");
        assert_eq!(resolve_model_key(Some("")), "unknown");
        assert_eq!(resolve_model_key(Some("gpt-4")), "gpt-4");
    }

    #[test]
    fn mismatched_currencies_report_a_mismatch_without_combining() {
        let mut spend = SuperstepSpend::default();
        spend.record("gpt-4", &Cost::new(10, usd()));
        spend.record("gpt-4", &Cost::new(1, eur()));

        match spend.take() {
            SuperstepCharge::CurrencyMismatch { first, other } => {
                assert_eq!(first, usd());
                assert_eq!(other, eur());
            }
            other => panic!("expected a CurrencyMismatch, got {other:?}"),
        }
    }

    /// The one settling test in this file (per plan): proves a child hook's
    /// `settle_boundary` performs no ledger call and leaves the shared
    /// accumulator intact for the parent's own boundary to drain -- observed
    /// through a real `InMemoryTreasuryLedger` settlement at the top level.
    #[tokio::test]
    async fn child_settle_boundary_makes_no_ledger_call_and_leaves_the_accumulator_intact() {
        let ledger = Arc::new(InMemoryTreasuryLedger::new());
        let context = SettlementContext {
            scope: LedgerScope::unattributed(),
            run_id: RunId::new_v7(),
            attempt: 1,
        };
        let hook = SpendHook::top_level(ledger.clone(), context.clone());
        let child = hook.child();

        child.record(Some("gpt-4"), &Cost::new(10, usd()));
        // A no-op: no ledger, and the shared accumulator is left untouched.
        assert_eq!(child.settle_boundary(1).await, SettleHealth::Healthy);

        hook.record(Some("gpt-4o-mini"), &Cost::new(5, usd()));
        assert_eq!(hook.settle_boundary(1).await, SettleHealth::Healthy);

        let rows = ledger
            .spend(SpendQuery {
                group_by: SpendGroupBy::Run,
                run_ids: vec![context.run_id.clone()],
                ..Default::default()
            })
            .await
            .expect("spend query succeeds");
        assert_eq!(rows.len(), 1, "exactly one settlement for this superstep");
        assert_eq!(
            rows[0].amount,
            Cost::new(15, usd()),
            "the child's recorded amount survived its own no-op settle_boundary"
        );
    }

    /// A ledger whose `settle` always fails: the lost-charge path of WR-4.
    struct FailingLedger;

    #[async_trait::async_trait]
    impl TreasuryLedgerPort for FailingLedger {
        async fn reserve(
            &self,
            _request: ReserveRequest,
        ) -> Result<ReservationId, TreasuryLedgerError> {
            unreachable!("settle-only writer")
        }

        async fn release(&self, _reservation: ReservationId) -> Result<(), TreasuryLedgerError> {
            unreachable!("settle-only writer")
        }

        async fn settle(
            &self,
            _request: SettleRequest,
        ) -> Result<SettleOutcome, TreasuryLedgerError> {
            Err(TreasuryLedgerError::Backend {
                source: Box::new(std::io::Error::other("ledger backend unavailable")),
            })
        }

        async fn spend(&self, _query: SpendQuery) -> Result<Vec<SpendRow>, TreasuryLedgerError> {
            Ok(Vec::new())
        }

        async fn store_now(&self) -> Result<chrono::DateTime<chrono::Utc>, TreasuryLedgerError> {
            Ok(chrono::Utc::now())
        }
    }

    fn context() -> SettlementContext {
        SettlementContext {
            scope: LedgerScope::unattributed(),
            run_id: RunId::new_v7(),
            attempt: 1,
        }
    }

    /// WR-4 (42-REVIEW): a charge that cannot be written is REPORTED, not silently dropped, so
    /// the engine can tell an attached spend guard it has gone blind to this run's spend.
    #[tokio::test]
    async fn a_ledger_error_reports_a_lost_charge() {
        let hook = SpendHook::top_level(Arc::new(FailingLedger), context());
        hook.record(Some("gpt-4"), &Cost::new(10, usd()));
        assert_eq!(hook.settle_boundary(1).await, SettleHealth::ChargeLost);
    }

    #[tokio::test]
    async fn a_currency_mismatch_reports_a_lost_charge() {
        let ledger = Arc::new(InMemoryTreasuryLedger::new());
        let hook = SpendHook::top_level(ledger, context());
        hook.record(Some("gpt-4"), &Cost::new(10, usd()));
        hook.record(Some("gpt-4"), &Cost::new(1, eur()));
        assert_eq!(hook.settle_boundary(1).await, SettleHealth::ChargeLost);
    }

    /// An empty superstep and a duplicate settlement lose nothing.
    #[tokio::test]
    async fn nothing_to_charge_and_a_duplicate_settlement_are_healthy() {
        let ledger = Arc::new(InMemoryTreasuryLedger::new());
        let ctx = context();
        let hook = SpendHook::top_level(ledger.clone(), ctx.clone());
        assert_eq!(hook.settle_boundary(1).await, SettleHealth::Healthy);

        hook.record(Some("gpt-4"), &Cost::new(10, usd()));
        assert_eq!(hook.settle_boundary(2).await, SettleHealth::Healthy);
        // The same (run, superstep, attempt) again: `AlreadySettled`, not a loss.
        hook.record(Some("gpt-4"), &Cost::new(10, usd()));
        assert_eq!(hook.settle_boundary(2).await, SettleHealth::Healthy);
    }
}
