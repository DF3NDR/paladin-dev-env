//! Attempt spend scope (G-50-5a, plan 50-29): the per-attempt accumulator through which a
//! Paladin execution reports every call it was billed for, however the attempt ends.
//!
//! ## The problem
//!
//! The Battalion engine charges one superstep settle per attempt (LEDGR-03/04). A successful
//! attempt reports its [`PaladinResult.cost`](crate::output::paladin_port::PaladinResult); a
//! failed attempt used to report the neutral `(TokenUsage::default(), None)` pair, and an attempt
//! the engine's deadline or idle bound cut off was dropped with its partial work. The calls the
//! Paladin had already made and been billed for -- a priced loop call before a `Timeout`, a
//! billed `EmptyCompletion`, two repair calls before `StructuredOutputInvalid` -- never reached
//! the ledger.
//!
//! ## The carrier
//!
//! A field on each [`PaladinError`](paladin_core::platform::container::paladin_error::PaladinError)
//! variant would change most of the enum and could never report spend for a future the engine
//! dropped. The carrier is a `tokio::task_local!` instead -- the precedent is
//! [`RUN_TRACE_EMITTER`](crate::output::trace_sink_port::RUN_TRACE_EMITTER), whose rustdoc explains
//! why a task-local is the only sound way to reach through the one shared `Arc<dyn PaladinPort>`
//! singleton every concurrent run uses. The rule:
//!
//! - **The engine installs a fresh [`SpendScope`] around every attempt**, outside the attempt
//!   future (`race_attempt` in `paladin-battalion`), so the accumulator outlives a future the
//!   deadline drops. Attempts run in their own spawned tasks, so concurrent nodes and retries never
//!   share a scope.
//! - **The execution service records at every fold** through [`record_incurred_spend`]: each
//!   buffered loop call, each billed failure, each structured call including every repair. Where
//!   it spawns a task that makes model calls it captures the scope first
//!   (`ATTEMPT_SPEND.try_with(Clone::clone)`) and re-installs it inside the task, because a
//!   task-local never crosses a `tokio::spawn`.
//! - **The engine reads the scope only for failed or timed-out attempts and the structured arm**
//!   (A1-50-29). A successful Paladin attempt keeps reporting `PaladinResult.cost`, so one source
//!   per outcome and no double count.
//!
//! Outside any scope -- a direct `PaladinPort` caller, a unit test -- [`record_incurred_spend`]
//! is a no-op returning `false` and [`current_attempt_spend`] is `None`. A caller that wants a
//! failed execution's spend installs a scope itself:
//!
//! ```no_run
//! use paladin_ports::output::spend_scope::{ATTEMPT_SPEND, SpendScope};
//! # use paladin_core::platform::container::paladin_error::PaladinError;
//! # async fn run_paladin() -> Result<(), PaladinError> { Ok(()) }
//! # async fn example() -> Result<(), PaladinError> {
//! let scope = SpendScope::new();
//! let outcome = ATTEMPT_SPEND.scope(scope.clone(), run_paladin()).await;
//! let spent_so_far = scope.snapshot(); // what was billed, whether `outcome` is Ok or Err
//! # let _ = (outcome, spent_so_far);
//! # Ok(())
//! # }
//! ```
//!
//! The scope holds a [`TokenUsage`] sum and a [`Cost`] tally only -- never prompt text, content,
//! a model id or a credential (T-50-29-04).
//!
//! ## Hopped-past bills (G-50-5a remainder (a), plan 50-30)
//!
//! A second, narrower carrier lives beside the attempt scope: [`HoppedBills`], reached through
//! the [`HOPPED_BILLS`] task-local. It spans ONE model call (all of its retries), not a whole
//! attempt. When a fallback chain moves past a hop whose error the vendor billed (a refusal
//! under `hop_on_refusal`), that bill would be dropped with the error -- the served response
//! (or the chain's `AllProvidersFailed`, which keeps only the last hop's bill) has nowhere to
//! carry it without changing `LlmResponse`'s shape. So:
//!
//! - **`PaladinExecutionService` installs a fresh collector around every model call**, buffered
//!   and streamed (the streamed task re-installs the handle it captured before the spawn).
//! - **`FallbackLlmAdapter` fills it** through [`record_hopped_bill`] at its single hop decision
//!   point, only when it really hops past a billed error.
//! - **`PricingLlmAdapter`, outside the chain, prices it**: every entry still at `cost: None` is
//!   priced against the entry's OWN model ([`HoppedBills::price_unpriced`]), never the served
//!   model's rate.
//! - **The execution service takes the entries** when the call ends, folds each into the run's
//!   tallies and the attempt scope above, and settles them in the call's own settlement row with
//!   a per-model breakdown.
//!
//! Entries are [`BilledUsage`] values: a model id, token counts and a [`Cost`], never refusal
//! text or a credential (T-50-30-04).
//!
//! ### The limit: a hop bill is counted when its model call ends
//!
//! The attempt scope above is fed the moment each call is folded, so a deadline or idle drop
//! keeps everything folded before it. A hopped-past bill is different: it sits in the call's
//! [`HoppedBills`] collector and reaches the attempt scope only when the whole model call (every
//! retry and every hop) completes, because it can only be priced once the call has returned to
//! the `PricingLlmAdapter` outside the chain, and recording an unpriced bill into the scope
//! would make the attempt's whole cost unknown. If the engine drops an attempt while a LATER
//! hop of that call is still in flight -- hop 1 returned a billed refusal, hop 2 had not
//! answered -- the collector is dropped with the future and the attempt is not charged for
//! hop 1's refusal. The "however the attempt ends" guarantee therefore covers every call that
//! has completed; it does not extend to a bill held by a model call that has not. A caller that
//! needs the bill of a call it cut off installs its own [`HOPPED_BILLS`] collector around that
//! call and reads it with [`HoppedBills::take`].

use std::sync::{Arc, Mutex, PoisonError};

use paladin_core::platform::container::cost::{Cost, IncurredSpend};
use paladin_core::platform::container::token_usage::TokenUsage;

use crate::output::llm_port::BilledUsage;

/// A shared, clone-to-share accumulator of one attempt's [`IncurredSpend`].
///
/// Every clone points at the same accumulator, so the engine can hold one clone outside the
/// attempt future it installs another clone into, and read what was recorded after that future
/// has been dropped. A poisoned lock is recovered with [`PoisonError::into_inner`] (T-50-29-03):
/// a panicked prior holder never loses the attempt's spend or panics the engine.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::cost::{Cost, CostError, CurrencyCode};
/// use paladin_core::platform::container::token_usage::TokenUsage;
/// use paladin_ports::output::spend_scope::SpendScope;
///
/// let usd = CurrencyCode::new("USD")?;
/// let scope = SpendScope::new();
/// let shared = scope.clone();
/// shared.record(&TokenUsage::new(10, 5), Some(&Cost::new(1_000, usd.clone())));
///
/// let spend = scope.snapshot();
/// assert_eq!(spend.calls(), 1);
/// assert_eq!(spend.usage(), &TokenUsage::new(10, 5));
/// assert_eq!(spend.cost(), Some(Cost::new(1_000, usd)));
/// # Ok::<(), CostError>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct SpendScope {
    spend: Arc<Mutex<IncurredSpend>>,
}

impl SpendScope {
    /// A fresh, empty scope.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one billed call into the shared accumulator (see [`IncurredSpend::record`]).
    pub fn record(&self, usage: &TokenUsage, cost: Option<&Cost>) {
        let mut spend = self.spend.lock().unwrap_or_else(PoisonError::into_inner);
        spend.record(usage, cost);
    }

    /// A copy of everything recorded so far.
    pub fn snapshot(&self) -> IncurredSpend {
        self.spend
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

tokio::task_local! {
    /// The active attempt's [`SpendScope`] (G-50-5a, plan 50-29), installed by the Battalion
    /// engine around every attempt it races (`race_attempt`), fed by `PaladinExecutionService`
    /// through [`record_incurred_spend`] at every point it folds a call's usage and cost, and
    /// read by the engine for a failed, timed-out or structured attempt.
    ///
    /// Why a task-local: the same reason as
    /// [`RUN_TRACE_EMITTER`](crate::output::trace_sink_port::RUN_TRACE_EMITTER) -- the execution
    /// service is reached through one shared `Arc<dyn PaladinPort>` singleton every concurrent
    /// run uses, so "this attempt's accumulator" can live neither on the service nor on the
    /// error. Each tokio task has its own value; the engine runs each attempt in its own task.
    ///
    /// Absent (no active scope) outside an engine attempt. [`record_incurred_spend`] and
    /// [`current_attempt_spend`] are the write and read sides.
    pub static ATTEMPT_SPEND: SpendScope;
}

/// Record one billed call against the active attempt's scope, if any.
///
/// Returns `true` when a scope was active and the call was recorded, `false` when this task is
/// not inside an [`ATTEMPT_SPEND`] scope (nothing is recorded and nothing fails).
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::cost::{Cost, CostError, CurrencyCode};
/// use paladin_core::platform::container::token_usage::TokenUsage;
/// use paladin_ports::output::spend_scope::{ATTEMPT_SPEND, SpendScope, record_incurred_spend};
///
/// let usd = CurrencyCode::new("USD")?;
/// let usage = TokenUsage::new(10, 5);
/// let cost = Cost::new(1_000, usd);
///
/// // No scope: a no-op.
/// assert!(!record_incurred_spend(&usage, Some(&cost)));
///
/// let scope = SpendScope::new();
/// let recorded = ATTEMPT_SPEND.sync_scope(scope.clone(), || {
///     record_incurred_spend(&usage, Some(&cost))
/// });
/// assert!(recorded);
/// assert_eq!(scope.snapshot().cost(), Some(cost));
/// # Ok::<(), CostError>(())
/// ```
pub fn record_incurred_spend(usage: &TokenUsage, cost: Option<&Cost>) -> bool {
    ATTEMPT_SPEND
        .try_with(|scope| scope.record(usage, cost))
        .is_ok()
}

/// A snapshot of what the active attempt has been billed so far, or `None` when this task is not
/// inside an [`ATTEMPT_SPEND`] scope.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::token_usage::TokenUsage;
/// use paladin_ports::output::spend_scope::{
///     ATTEMPT_SPEND, SpendScope, current_attempt_spend, record_incurred_spend,
/// };
///
/// assert!(current_attempt_spend().is_none());
///
/// let spend = ATTEMPT_SPEND.sync_scope(SpendScope::new(), || {
///     record_incurred_spend(&TokenUsage::new(3, 4), None);
///     current_attempt_spend()
/// });
/// let spend = spend.ok_or("inside a scope the snapshot is Some")?;
/// assert_eq!(spend.calls(), 1);
/// assert_eq!(spend.usage().total_tokens, 7);
/// assert_eq!(spend.cost(), None, "an unpriced call: cost unknown");
/// # Ok::<(), &'static str>(())
/// ```
pub fn current_attempt_spend() -> Option<IncurredSpend> {
    ATTEMPT_SPEND.try_with(SpendScope::snapshot).ok()
}

/// A shared, clone-to-share collector of the bills a fallback chain moved past during one model
/// call (G-50-5a remainder (a), plan 50-30).
///
/// A `FallbackLlmAdapter` built with `hop_on_refusal(true)` hops past a [`LlmError::Refusal`]
/// the vendor billed; the refused hop's [`BilledUsage`] would otherwise be dropped with the error
/// when a later hop answers, or when the chain ends in `AllProvidersFailed` (which keeps only the
/// last hop's bill). The chain records each hopped-past bill here at its one hop decision point;
/// `PricingLlmAdapter`, which sits outside the chain, prices every entry still carrying
/// `cost: None` against the entry's own model; and `PaladinExecutionService`, which installs a
/// fresh collector around every model call (all of one call's retries), takes the entries and
/// settles them in the call's own settlement row beside the served call.
///
/// Every clone points at the same list, in recording order. A poisoned lock is recovered with
/// [`PoisonError::into_inner`], never `unwrap`. Entries hold a model id, token counts and a
/// [`Cost`] only -- never refusal text, content or a credential.
///
/// [`LlmError::Refusal`]: crate::output::llm_port::LlmError::Refusal
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::cost::{Cost, CostError, CurrencyCode};
/// use paladin_ports::output::llm_port::{BilledUsage, TokenUsage};
/// use paladin_ports::output::spend_scope::HoppedBills;
///
/// let usd = CurrencyCode::new("USD")?;
/// let bills = HoppedBills::new();
/// let shared = bills.clone();
/// shared.push(BilledUsage::new("m1", TokenUsage::new(40, 2)));
///
/// // The pricing decorator prices each unpriced entry against its own model.
/// bills.price_unpriced(|bill| (bill.model == "m1").then(|| Cost::new(900, usd.clone())));
///
/// let taken = bills.take();
/// assert_eq!(taken.len(), 1);
/// assert_eq!(taken[0].cost, Some(Cost::new(900, usd)));
/// assert!(shared.take().is_empty(), "take empties the shared list");
/// # Ok::<(), CostError>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct HoppedBills {
    bills: Arc<Mutex<Vec<BilledUsage>>>,
}

impl HoppedBills {
    /// A fresh, empty collector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one hopped-past bill, after every bill already recorded.
    pub fn push(&self, billed: BilledUsage) {
        self.lock().push(billed);
    }

    /// Price, in place and in order, every entry whose `cost` is still `None`: `price` is asked
    /// once per such entry and its `Some` answer is set on it. Entries that already carry a cost
    /// are never re-priced, so calling this again (a later call of the same collector, a
    /// stream's terminal item) is idempotent.
    pub fn price_unpriced(&self, mut price: impl FnMut(&BilledUsage) -> Option<Cost>) {
        for bill in self.lock().iter_mut().filter(|bill| bill.cost.is_none()) {
            if let Some(cost) = price(bill) {
                bill.cost = Some(cost);
            }
        }
    }

    /// Remove and return every recorded entry, in recording order, leaving the collector empty.
    pub fn take(&self) -> Vec<BilledUsage> {
        std::mem::take(&mut *self.lock())
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<BilledUsage>> {
        self.bills.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

tokio::task_local! {
    /// The active model call's [`HoppedBills`] collector (G-50-5a remainder (a), plan 50-30),
    /// installed by `PaladinExecutionService` around every model call it makes (all of one
    /// call's retries, buffered and streamed), filled by `FallbackLlmAdapter` through
    /// [`record_hopped_bill`] when it hops past a billed error, and priced by
    /// `PricingLlmAdapter`.
    ///
    /// A task-local for the same reason as [`ATTEMPT_SPEND`]: the chain is reached through one
    /// shared `Arc<dyn LlmPort>` every concurrent call uses, so "this call's hopped bills" can
    /// live neither on the adapter nor on the served response (`LlmResponse` keeps its shape).
    /// Absent outside an execution-service model call.
    pub static HOPPED_BILLS: HoppedBills;
}

/// Record one bill a fallback chain moved past against the active call's collector, if any.
///
/// Returns `true` when a [`HOPPED_BILLS`] collector was active and the bill was recorded, `false`
/// when none is (nothing is recorded and nothing fails; the chain warns once per provider and
/// model so the lost bill is visible).
///
/// # Examples
///
/// ```
/// use paladin_ports::output::llm_port::{BilledUsage, TokenUsage};
/// use paladin_ports::output::spend_scope::{HOPPED_BILLS, HoppedBills, record_hopped_bill};
///
/// let bill = BilledUsage::new("m1", TokenUsage::new(40, 2));
///
/// // No collector: a no-op.
/// assert!(!record_hopped_bill(bill.clone()));
///
/// let bills = HoppedBills::new();
/// let recorded = HOPPED_BILLS.sync_scope(bills.clone(), || record_hopped_bill(bill.clone()));
/// assert!(recorded);
/// assert_eq!(bills.take(), vec![bill]);
/// ```
pub fn record_hopped_bill(billed: BilledUsage) -> bool {
    HOPPED_BILLS.try_with(|bills| bills.push(billed)).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::cost::CurrencyCode;

    fn usd() -> CurrencyCode {
        CurrencyCode::new("USD").expect("USD is a valid currency code")
    }

    #[test]
    fn clones_share_one_accumulator() {
        let scope = SpendScope::new();
        let a = scope.clone();
        let b = scope.clone();
        a.record(&TokenUsage::new(1, 2), Some(&Cost::new(10, usd())));
        b.record(&TokenUsage::new(3, 4), Some(&Cost::new(20, usd())));

        let spend = scope.snapshot();
        assert_eq!(spend.calls(), 2);
        assert_eq!(spend.usage(), &TokenUsage::new(4, 6));
        assert_eq!(spend.cost(), Some(Cost::new(30, usd())));
        assert_eq!(a.snapshot(), b.snapshot());
    }

    #[test]
    fn outside_a_scope_nothing_is_recorded_and_nothing_is_read() {
        assert!(!record_incurred_spend(
            &TokenUsage::new(1, 1),
            Some(&Cost::new(1, usd()))
        ));
        assert!(current_attempt_spend().is_none());
    }

    #[tokio::test]
    async fn inside_a_scope_every_await_point_records_into_it() {
        let scope = SpendScope::new();
        let outcome = ATTEMPT_SPEND
            .scope(scope.clone(), async {
                assert!(record_incurred_spend(
                    &TokenUsage::new(10, 5),
                    Some(&Cost::new(100, usd()))
                ));
                tokio::task::yield_now().await;
                assert!(record_incurred_spend(
                    &TokenUsage::new(1, 1),
                    Some(&Cost::new(1, usd()))
                ));
                current_attempt_spend().expect("inside the scope")
            })
            .await;
        assert_eq!(outcome.calls(), 2);
        assert_eq!(outcome, scope.snapshot());
        assert_eq!(scope.snapshot().cost(), Some(Cost::new(101, usd())));
        assert!(current_attempt_spend().is_none(), "the scope ended");
    }

    /// The engine's whole reason for the scope: a future dropped mid-flight
    /// (a deadline) still leaves what it recorded in the scope held outside.
    #[tokio::test]
    async fn a_dropped_future_leaves_its_recorded_spend_in_the_outer_handle() {
        let scope = SpendScope::new();
        let attempt = ATTEMPT_SPEND.scope(scope.clone(), async {
            record_incurred_spend(&TokenUsage::new(7, 3), Some(&Cost::new(70, usd())));
            std::future::pending::<()>().await;
        });
        let cut = tokio::time::timeout(std::time::Duration::from_millis(20), attempt).await;
        assert!(cut.is_err(), "the attempt was dropped by the deadline");
        let spend = scope.snapshot();
        assert_eq!(spend.calls(), 1);
        assert_eq!(spend.cost(), Some(Cost::new(70, usd())));
    }

    #[tokio::test]
    async fn concurrent_scopes_never_see_each_other() {
        let left = SpendScope::new();
        let right = SpendScope::new();
        let (l, r) = tokio::join!(
            ATTEMPT_SPEND.scope(left.clone(), async {
                tokio::task::yield_now().await;
                record_incurred_spend(&TokenUsage::new(1, 0), Some(&Cost::new(1, usd())));
                tokio::task::yield_now().await;
                current_attempt_spend().expect("left scope")
            }),
            ATTEMPT_SPEND.scope(right.clone(), async {
                record_incurred_spend(&TokenUsage::new(0, 2), Some(&Cost::new(2, usd())));
                tokio::task::yield_now().await;
                record_incurred_spend(&TokenUsage::new(0, 2), Some(&Cost::new(2, usd())));
                current_attempt_spend().expect("right scope")
            }),
        );
        assert_eq!(l.calls(), 1);
        assert_eq!(l.cost(), Some(Cost::new(1, usd())));
        assert_eq!(r.calls(), 2);
        assert_eq!(r.cost(), Some(Cost::new(4, usd())));
        assert_eq!(left.snapshot(), l);
        assert_eq!(right.snapshot(), r);
    }

    #[test]
    fn a_poisoned_lock_is_recovered_not_propagated() {
        let scope = SpendScope::new();
        let poisoner = scope.clone();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let _guard = poisoner.spend.lock().expect("first lock is clean");
            panic!("poison the accumulator");
        }));
        scope.record(&TokenUsage::new(1, 1), Some(&Cost::new(5, usd())));
        assert_eq!(scope.snapshot().cost(), Some(Cost::new(5, usd())));
    }

    fn bill(model: &str, prompt: u32) -> BilledUsage {
        BilledUsage::new(model, TokenUsage::new(prompt, 1))
    }

    #[test]
    fn hopped_bills_clones_share_one_list() {
        let bills = HoppedBills::new();
        let a = bills.clone();
        let b = bills.clone();
        a.push(bill("m1", 10));
        b.push(bill("m2", 20));
        assert_eq!(bills.take(), vec![bill("m1", 10), bill("m2", 20)]);
        assert!(a.take().is_empty(), "take emptied the one shared list");
    }

    #[test]
    fn record_hopped_bill_is_false_with_no_collector() {
        assert!(!record_hopped_bill(bill("m1", 10)));
    }

    #[tokio::test]
    async fn record_hopped_bill_records_into_the_active_collector() {
        let bills = HoppedBills::new();
        let recorded = HOPPED_BILLS
            .scope(bills.clone(), async {
                let first = record_hopped_bill(bill("m1", 10));
                tokio::task::yield_now().await;
                first && record_hopped_bill(bill("m2", 20))
            })
            .await;
        assert!(recorded);
        assert_eq!(bills.take(), vec![bill("m1", 10), bill("m2", 20)]);
        assert!(!record_hopped_bill(bill("m3", 30)), "the collector ended");
    }

    #[test]
    fn entries_priced_in_place_keep_their_order_and_their_cost() {
        let bills = HoppedBills::new();
        let already = bill("m0", 5).with_cost(Cost::new(7, usd()));
        bills.push(bill("m1", 10));
        bills.push(already.clone());
        bills.push(bill("unpriced", 30));
        bills.push(bill("m2", 20));

        let mut asked = Vec::new();
        bills.price_unpriced(|entry| {
            asked.push(entry.model.clone());
            match entry.model.as_str() {
                "m1" => Some(Cost::new(100, usd())),
                "m2" => Some(Cost::new(200, usd())),
                _ => None,
            }
        });
        // A second pass asks only about the entry still unpriced (idempotent).
        bills.price_unpriced(|entry| {
            asked.push(entry.model.clone());
            None
        });

        assert_eq!(asked, ["m1", "unpriced", "m2", "unpriced"]);
        let taken = bills.take();
        let models: Vec<&str> = taken.iter().map(|entry| entry.model.as_str()).collect();
        assert_eq!(models, ["m1", "m0", "unpriced", "m2"]);
        assert_eq!(taken[0].cost, Some(Cost::new(100, usd())));
        assert_eq!(
            taken[1], already,
            "an already-priced entry is never re-priced"
        );
        assert_eq!(taken[2].cost, None, "an unpriced model keeps cost None");
        assert_eq!(taken[3].cost, Some(Cost::new(200, usd())));
    }

    #[test]
    fn a_poisoned_hopped_bills_lock_is_recovered() {
        let bills = HoppedBills::new();
        let poisoner = bills.clone();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let _guard = poisoner.bills.lock().expect("first lock is clean");
            panic!("poison the collector");
        }));
        bills.push(bill("m1", 10));
        assert_eq!(bills.take(), vec![bill("m1", 10)]);
    }
}
