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
//! - **`PricingLlmAdapter`, outside the chain, installs its pricer first**
//!   ([`HoppedBills::install_pricer`]) before it delegates; the first pricer installed wins, so
//!   the outermost decorator's table prices every hop bill, each against the entry's OWN model,
//!   never the served model's rate.
//! - **`FallbackLlmAdapter` fills it** through [`record_hopped_bill`] at its single hop decision
//!   point, only when it really hops past a billed error. The bill is priced and counted into
//!   [`ATTEMPT_SPEND`] right then.
//! - **The execution service takes the entries** with [`HoppedBills::count_and_take`] when the
//!   call ends, folds each into the run's tallies, and settles them in the call's own
//!   settlement row with a per-model breakdown. It does not count them again: each entry carries
//!   a counted flag, and `count_and_take` counts only the entries that were not counted yet.
//!
//! Entries are [`BilledUsage`] values: a model id, token counts and a [`Cost`], never refusal
//! text or a credential (T-50-30-04).
//!
//! ### Counted at the hop, however the attempt ends
//!
//! A hopped-past bill reaches the attempt scope the moment the chain hops past it, priced, not
//! when the whole model call returns. So a deadline or idle drop that lands after hop 1
//! returned a billed refusal, while hop 2 of the same call is still in flight, still leaves
//! hop 1's priced bill in the attempt scope; the engine reads it with the rest of the attempt's
//! spend and the superstep settle charges it once. A stack with no pricing decorator, or a bill
//! on a model with no price row, counts with `cost: None`, exactly as the end-of-call fold
//! would have counted it.
//!
//! An entry recorded where no attempt scope was visible (a bare [`HoppedBills::push`], or a
//! record made outside [`ATTEMPT_SPEND`]) is counted by [`HoppedBills::count_and_take`] if a
//! scope is visible when the collector is taken. A decorator that prices later with
//! [`HoppedBills::price_unpriced`] reaches the call's ledger row but not the attempt scope, so
//! an out-of-tree pricing decorator calls [`HoppedBills::install_pricer`] before it delegates.

use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError, TryLockError};

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
/// call (G-50-5a remainder (a), plan 50-30; counted at the hop by plan 50-32, WR-03).
///
/// A `FallbackLlmAdapter` built with `hop_on_refusal(true)` hops past a [`LlmError::Refusal`]
/// the vendor billed; the refused hop's [`BilledUsage`] would otherwise be dropped with the error
/// when a later hop answers, or when the chain ends in `AllProvidersFailed` (which keeps only the
/// last hop's bill). The chain records each hopped-past bill here at its one hop decision point
/// through [`record_hopped_bill`]. `PricingLlmAdapter`, which sits outside the chain, installs a
/// pricer on the collector before it delegates ([`HoppedBills::install_pricer`]), so the bill is
/// priced against its own model AND counted into the active [`ATTEMPT_SPEND`] scope the moment it
/// is recorded: an attempt the engine drops while a later hop is still in flight has already been
/// charged for it. `PaladinExecutionService`, which installs a fresh collector around every model
/// call (all of one call's retries), takes the entries with [`HoppedBills::count_and_take`] and
/// settles them in the call's own settlement row beside the served call. Every entry carries a
/// private counted flag, so a bill is counted into the attempt scope exactly once.
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
/// // A decorator that could not install a pricer first prices each unpriced entry against its
/// // own model after the call (the ledger row sees it; an attempt scope does not).
/// bills.price_unpriced(|bill| (bill.model == "m1").then(|| Cost::new(900, usd.clone())));
///
/// let taken = bills.take();
/// assert_eq!(taken.len(), 1);
/// assert_eq!(taken[0].cost, Some(Cost::new(900, usd)));
/// assert!(shared.take().is_empty(), "take empties the shared list");
/// # Ok::<(), CostError>(())
/// ```
#[derive(Clone, Default)]
pub struct HoppedBills {
    entries: Arc<Mutex<Vec<HopEntry>>>,
    pricer: Arc<OnceLock<HopPricer>>,
}

/// The pricing rule a decorator installs on a collector: one bill in, its cost out (`None` for
/// an unpriced model).
type HopPricer = Box<dyn Fn(&BilledUsage) -> Option<Cost> + Send + Sync>;

/// One recorded bill and whether the attempt scope already counted it.
#[derive(Debug)]
struct HopEntry {
    bill: BilledUsage,
    counted: bool,
}

impl fmt::Debug for HoppedBills {
    /// Entries (model, counts, cost, counted flag) and whether a pricer is installed; never the
    /// pricer closure. `try_lock`, so formatting never blocks and never panics.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut shown = f.debug_struct("HoppedBills");
        match self.entries.try_lock() {
            Ok(entries) => shown.field("entries", &*entries),
            Err(TryLockError::Poisoned(poisoned)) => {
                shown.field("entries", &*poisoned.into_inner())
            }
            Err(TryLockError::WouldBlock) => shown.field("entries", &format_args!("<locked>")),
        };
        shown
            .field("pricer_installed", &self.pricer.get().is_some())
            .finish()
    }
}

impl HoppedBills {
    /// A fresh, empty collector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one hopped-past bill, after every bill already recorded. The entry is neither
    /// priced nor counted into an attempt scope: only [`record_hopped_bill`] (through the chain)
    /// does that. [`HoppedBills::count_and_take`] counts such an entry when it is taken.
    pub fn push(&self, billed: BilledUsage) {
        self.lock().push(HopEntry {
            bill: billed,
            counted: false,
        });
    }

    /// Install the pricing rule [`record_hopped_bill`] applies to every bill that arrives with
    /// `cost: None`, BEFORE it counts the bill into the attempt scope. Returns `true` when this
    /// call installed the pricer and `false` when one was already installed (the new one is
    /// dropped).
    ///
    /// A pricing decorator calls this before it delegates to the chain it wraps, so every billed
    /// hop the chain moves past is priced against its own model at the hop. An unpriced bill
    /// counted into an attempt scope would make the attempt's whole cost unknown for good, which
    /// is why the price must exist before the bill is counted.
    ///
    /// first pricer installed wins (the outermost PricingLlmAdapter's table prices every hop
    /// bill). The outermost decorator runs first, so its table is installed first; a decorator
    /// nested inside the chain prices its own hop's error bill before the chain records it, so
    /// the collector's pricer is never consulted for that bill.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_core::platform::container::cost::{Cost, CostError, CurrencyCode};
    /// use paladin_ports::output::llm_port::{BilledUsage, TokenUsage};
    /// use paladin_ports::output::spend_scope::{
    ///     ATTEMPT_SPEND, HOPPED_BILLS, HoppedBills, SpendScope, record_hopped_bill,
    /// };
    ///
    /// let usd = CurrencyCode::new("USD")?;
    /// let bills = HoppedBills::new();
    /// assert!(bills.install_pricer(move |bill| {
    ///     (bill.model == "m1").then(|| Cost::new(900, usd.clone()))
    /// }));
    /// assert!(!bills.install_pricer(|_| None), "the first pricer wins");
    ///
    /// let scope = SpendScope::new();
    /// ATTEMPT_SPEND.sync_scope(scope.clone(), || {
    ///     HOPPED_BILLS.sync_scope(bills.clone(), || {
    ///         record_hopped_bill(BilledUsage::new("m1", TokenUsage::new(40, 2)))
    ///     })
    /// });
    /// // Priced and counted at the hop, before the model call ended.
    /// let spend = scope.snapshot();
    /// assert_eq!(spend.calls(), 1);
    /// assert_eq!(spend.cost().map(|cost| cost.nanos()), Some(900));
    /// # Ok::<(), CostError>(())
    /// ```
    pub fn install_pricer(
        &self,
        pricer: impl Fn(&BilledUsage) -> Option<Cost> + Send + Sync + 'static,
    ) -> bool {
        self.pricer.set(Box::new(pricer)).is_ok()
    }

    /// Price, in place and in order, every entry whose `cost` is still `None`: `price` is asked
    /// once per such entry and its `Some` answer is set on it. Entries that already carry a cost
    /// are never re-priced, so calling this again (a later call of the same collector, a
    /// stream's terminal item) is idempotent.
    ///
    /// This prices the entry for the call's ledger row and the run's tallies. An entry that an
    /// attempt scope already counted keeps what the scope counted, so a decorator should install
    /// its pricer with [`HoppedBills::install_pricer`] before it delegates and keep this as an
    /// after-call safety net only.
    pub fn price_unpriced(&self, mut price: impl FnMut(&BilledUsage) -> Option<Cost>) {
        for entry in self
            .lock()
            .iter_mut()
            .filter(|entry| entry.bill.cost.is_none())
        {
            if let Some(cost) = price(&entry.bill) {
                entry.bill.cost = Some(cost);
            }
        }
    }

    /// Remove and return every recorded entry, in recording order, leaving the collector empty.
    /// Counts nothing: see [`HoppedBills::count_and_take`] for the consumer that settles them.
    pub fn take(&self) -> Vec<BilledUsage> {
        std::mem::take(&mut *self.lock())
            .into_iter()
            .map(|entry| entry.bill)
            .collect()
    }

    /// Remove and return every recorded entry, in recording order, counting each one the active
    /// [`ATTEMPT_SPEND`] scope has NOT yet counted (an entry recorded at its hop inside the scope
    /// is already counted and is not counted again). The collector's lock is released before any
    /// bill is counted.
    ///
    /// The execution service takes every call's collector with this method, so a bill is counted
    /// exactly once whether the chain recorded it inside the scope or it was only `push`ed.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_ports::output::llm_port::{BilledUsage, TokenUsage};
    /// use paladin_ports::output::spend_scope::{ATTEMPT_SPEND, HoppedBills, SpendScope};
    ///
    /// let bills = HoppedBills::new();
    /// bills.push(BilledUsage::new("m1", TokenUsage::new(40, 2)));
    ///
    /// let scope = SpendScope::new();
    /// let taken = ATTEMPT_SPEND.sync_scope(scope.clone(), || bills.count_and_take());
    /// assert_eq!(taken.len(), 1);
    /// assert_eq!(scope.snapshot().calls(), 1, "counted by the take");
    /// assert!(bills.count_and_take().is_empty(), "and never again");
    /// ```
    pub fn count_and_take(&self) -> Vec<BilledUsage> {
        let entries = std::mem::take(&mut *self.lock());
        entries
            .into_iter()
            .map(|entry| {
                if !entry.counted {
                    record_incurred_spend(&entry.bill.usage, entry.bill.cost.as_ref());
                }
                entry.bill
            })
            .collect()
    }

    /// Record one bill at its hop: price it through the installed pricer (when it has no cost
    /// and a pricer is installed), count it into the active attempt scope at once, and keep it
    /// with its counted flag. Neither the pricer nor the scope runs under the entries lock.
    fn record(&self, mut billed: BilledUsage) {
        if billed.cost.is_none()
            && let Some(pricer) = self.pricer.get()
            && let Some(cost) = pricer(&billed)
        {
            billed.cost = Some(cost);
        }
        let counted = record_incurred_spend(&billed.usage, billed.cost.as_ref());
        self.lock().push(HopEntry {
            bill: billed,
            counted,
        });
    }

    fn lock(&self) -> MutexGuard<'_, Vec<HopEntry>> {
        self.entries.lock().unwrap_or_else(PoisonError::into_inner)
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

/// Record one bill a fallback chain moved past against the active call's collector, if any, and
/// count it into the active attempt scope at once (WR-03, plan 50-32).
///
/// A bill with no cost is first priced through the collector's installed pricer
/// ([`HoppedBills::install_pricer`]), against the bill's own model. Only then is it counted into
/// the [`ATTEMPT_SPEND`] scope, so a pricing decorator in the stack means the bill is never
/// counted unpriced. With no pricer installed, or for a model the pricer has no price for, the
/// bill counts with `cost: None`, exactly as the end-of-call fold would have counted it.
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
    HOPPED_BILLS.try_with(|bills| bills.record(billed)).is_ok()
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
            let _guard = poisoner.entries.lock().expect("first lock is clean");
            panic!("poison the collector");
        }));
        bills.push(bill("m1", 10));
        assert_eq!(bills.take(), vec![bill("m1", 10)]);
    }

    /// WR-03 (plan 50-32), A1-50-32: a bill recorded while an attempt scope is active is counted
    /// at once, even when nothing prices it (no pricing decorator, or an unpriced model). The
    /// attempt's cost is then unknown, exactly what the end-of-call fold would have recorded.
    #[test]
    fn a_bill_with_no_pricer_is_counted_unpriced() {
        let scope = SpendScope::new();
        let bills = HoppedBills::new();
        let recorded = ATTEMPT_SPEND.sync_scope(scope.clone(), || {
            HOPPED_BILLS.sync_scope(bills.clone(), || record_hopped_bill(bill("m1", 10)))
        });
        assert!(recorded);

        let spend = scope.snapshot();
        assert_eq!(spend.calls(), 1, "counted at the hop");
        assert_eq!(spend.usage(), &TokenUsage::new(10, 1));
        assert_eq!(spend.cost(), None, "no pricer: counted unpriced");
    }

    /// Guard (plan 50-32; green before and after): `push` only appends. It neither prices nor
    /// counts into an attempt scope.
    #[test]
    fn push_neither_prices_nor_counts() {
        let scope = SpendScope::new();
        let bills = HoppedBills::new();
        ATTEMPT_SPEND.sync_scope(scope.clone(), || bills.push(bill("m1", 10)));

        assert_eq!(scope.snapshot().calls(), 0, "push never counts");
        let taken = bills.take();
        assert_eq!(taken.len(), 1);
        assert_eq!(taken[0].cost, None, "push never prices");
    }

    fn m1_pricer(bill: &BilledUsage) -> Option<Cost> {
        (bill.model == "m1").then(|| Cost::new(i64::from(bill.usage.prompt_tokens) * 100, usd()))
    }

    /// WR-03 (plan 50-32): with a pricer installed, a recorded hop bill is priced against its
    /// own model and counted into the active attempt scope at once.
    #[test]
    fn a_recorded_hop_bill_is_priced_and_counted_at_once() {
        let scope = SpendScope::new();
        let bills = HoppedBills::new();
        assert!(bills.install_pricer(m1_pricer));

        ATTEMPT_SPEND.sync_scope(scope.clone(), || {
            HOPPED_BILLS.sync_scope(bills.clone(), || record_hopped_bill(bill("m1", 10)))
        });

        let spend = scope.snapshot();
        assert_eq!(spend.calls(), 1);
        assert_eq!(spend.usage(), &TokenUsage::new(10, 1));
        assert_eq!(spend.cost(), Some(Cost::new(1_000, usd())));
        let taken = bills.take();
        assert_eq!(
            taken[0].cost,
            Some(Cost::new(1_000, usd())),
            "entry is priced"
        );
    }

    /// An entry recorded with no scope visible is counted once by `count_and_take` inside a
    /// scope; an entry recorded inside the scope is not counted again.
    #[test]
    fn count_and_take_counts_only_what_was_not_counted() {
        let scope = SpendScope::new();
        let bills = HoppedBills::new();
        assert!(bills.install_pricer(m1_pricer));

        // Recorded where no attempt scope is visible: held, uncounted.
        HOPPED_BILLS.sync_scope(bills.clone(), || record_hopped_bill(bill("m1", 10)));
        // Recorded inside the scope: counted at the hop.
        ATTEMPT_SPEND.sync_scope(scope.clone(), || {
            HOPPED_BILLS.sync_scope(bills.clone(), || record_hopped_bill(bill("m1", 20)))
        });
        assert_eq!(scope.snapshot().calls(), 1);

        let taken = ATTEMPT_SPEND.sync_scope(scope.clone(), || bills.count_and_take());
        assert_eq!(taken.len(), 2, "every bill is returned, in recording order");
        assert_eq!(taken[0].usage.prompt_tokens, 10);
        assert_eq!(taken[1].usage.prompt_tokens, 20);
        let spend = scope.snapshot();
        assert_eq!(
            spend.calls(),
            2,
            "the first entry counted once, the second not again"
        );
        assert_eq!(spend.cost(), Some(Cost::new(3_000, usd())));
        assert!(bills.count_and_take().is_empty(), "take emptied the list");
    }

    #[test]
    fn the_first_installed_pricer_wins() {
        let bills = HoppedBills::new();
        assert!(bills.install_pricer(|_| Some(Cost::new(1, usd()))));
        assert!(
            !bills.install_pricer(|_| Some(Cost::new(2, usd()))),
            "a second pricer is refused"
        );

        HOPPED_BILLS.sync_scope(bills.clone(), || record_hopped_bill(bill("m1", 10)));
        assert_eq!(bills.take()[0].cost, Some(Cost::new(1, usd())));
    }
}
