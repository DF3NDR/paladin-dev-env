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

use std::sync::{Arc, Mutex, PoisonError};

use paladin_core::platform::container::cost::{Cost, IncurredSpend};
use paladin_core::platform::container::token_usage::TokenUsage;

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
}
