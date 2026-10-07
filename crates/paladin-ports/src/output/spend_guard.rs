//! # Spend Guard Port -- Mid-Run Allowance Halt (ALLOW-03, Phase 42 D-04, ADR-0057)
//!
//! Defines [`SpendGuard`], the seam a per-run allowance check attaches to the superstep engine
//! through. `WarEngine::with_spend_guard` (`paladin-battalion`) consults an attached guard once
//! per superstep boundary at the top of the loop, BESIDE and after the cancellation signals
//! (`CancellationToken`, [`crate::output::cancellation_probe::CancellationProbe`]): a cancel
//! always wins over a spend halt observed at the same boundary.
//!
//! ## Why a port: policy in the facade, mechanism in the engine (D-04)
//!
//! The ceilings, the window arithmetic and the ledger balance live in the facade's `Treasurer`.
//! The engine learns none of that: it asks "may the next superstep run?" and receives a
//! [`SpendDecision`]. A halting decision carries the core
//! [`HaltReason`] value, which the engine passes through untouched to the run's
//! outcome, so the worker, the persisted row and every wire surface read one typed value.
//!
//! ## Fallible by design -- a failed read is a halt, never a swallowed `Continue` (D-03)
//!
//! This is the deliberate difference from [`crate::output::cancellation_probe::CancellationProbe`],
//! which answers a plain `bool` and swallows its own read failures. A guard that could not read
//! the ledger must not let a metered run keep spending: it answers
//! [`SpendDecision::Halt`] with [`HaltReason::LedgerUnavailable`] (fail closed).
//!
//! ## Thread Safety
//!
//! Implementations must be `Send + Sync`: a guard is consulted concurrently across nested
//! `NodeSpec::Battalion` child runs, which inherit the parent's guard.
//!
//! ```
//! use paladin_core::platform::container::waypoint::ThreadId;
//! use paladin_ports::output::spend_guard::{NeverHalts, SpendDecision, SpendGuard};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let guard = NeverHalts;
//!     let thread = ThreadId::new("11111111-1111-7111-8111-111111111111")?;
//!     assert_eq!(guard.check(&thread).await, SpendDecision::Continue);
//!     Ok(())
//! }
//! ```

use async_trait::async_trait;

use paladin_core::platform::container::allowance::HaltReason;
use paladin_core::platform::container::waypoint::ThreadId;

/// The answer a [`SpendGuard`] gives at a superstep boundary.
///
/// `#[non_exhaustive]`: the engine treats `Continue` and any future variant as "proceed", so a
/// new decision kind is not a breaking change.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::allowance::HaltReason;
/// use paladin_ports::output::spend_guard::SpendDecision;
///
/// let decision = SpendDecision::Halt(HaltReason::LedgerUnavailable);
/// assert_ne!(decision, SpendDecision::Continue);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SpendDecision {
    /// The next superstep may run.
    Continue,
    /// The run must stop at this boundary; the reason is carried to the run's outcome.
    Halt(HaltReason),
}

/// Consulted by the superstep engine once per boundary (D-04) to decide whether an
/// allowance-bound run may dispatch its next superstep.
///
/// # Fallible by design
///
/// Unlike a cancellation probe, a failed read is a [`SpendDecision::Halt`] answer, never a
/// swallowed `Continue` (D-03) -- see the module-level section.
///
/// # Overshoot bound and nested runs
///
/// A run can spend at most one **top-level** superstep beyond a ceiling, where that superstep
/// includes any nested `NodeSpec::Battalion` run it contains. A nested child's spend is folded
/// into its parent's accumulator and settled only at the parent's own boundary, so a guard
/// consulted at a child's inner boundary sees the spend of OTHER runs but none of the in-flight
/// spend of the run that hosts it. The inner checks are still valuable (they stop a child as soon
/// as another run exhausts the allowance), but they are not a tighter bound on the hosting run's
/// own spend.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync`; see the module-level section.
#[async_trait]
pub trait SpendGuard: Send + Sync {
    /// Decide whether `thread`'s run may dispatch its next superstep.
    ///
    /// Called once per superstep boundary, including the very first, after the cancellation
    /// signals have already been checked. A [`SpendDecision::Halt`] answer produces the same
    /// halted-Waypoint path a cancel does, with the reason attached to the outcome.
    async fn check(&self, thread: &ThreadId) -> SpendDecision;

    /// Tell the guard that a superstep's priced spend could NOT be written to the ledger for
    /// `thread`'s run (a ledger write error, or a charge whose currencies disagreed).
    ///
    /// The guard's balance reads are only as good as the writes that feed them, so a run whose
    /// settlements keep failing would otherwise spend unmetered while every read still
    /// succeeds. The engine calls this after a boundary settlement it could not write, and an
    /// implementation that meters the run should treat the ledger as unreliable from here on:
    /// answer [`SpendDecision::Halt`] with [`HaltReason::LedgerUnavailable`] at its next
    /// `check` (fail closed, the same posture as an unreadable ledger, D-03). It must not
    /// halt a run it would not otherwise meter.
    ///
    /// The default is a no-op, so an existing guard keeps its behaviour. Synchronous and
    /// infallible: it records a fact and never blocks the superstep loop.
    fn note_unsettled_spend(&self, _thread: &ThreadId) {}
}

/// A [`SpendGuard`] that never halts.
///
/// Equivalent in effect to never calling `WarEngine::with_spend_guard`, but convenient where a
/// concrete `Arc<dyn SpendGuard>` is required rather than an `Option`.
///
/// ```
/// use paladin_core::platform::container::waypoint::ThreadId;
/// use paladin_ports::output::spend_guard::{NeverHalts, SpendDecision, SpendGuard};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let thread = ThreadId::new("11111111-1111-7111-8111-111111111111")?;
///     assert_eq!(NeverHalts.check(&thread).await, SpendDecision::Continue);
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NeverHalts;

#[async_trait]
impl SpendGuard for NeverHalts {
    async fn check(&self, _thread: &ThreadId) -> SpendDecision {
        SpendDecision::Continue
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn thread_id() -> ThreadId {
        ThreadId::new("11111111-1111-7111-8111-111111111111").unwrap()
    }

    #[tokio::test]
    async fn never_halts_always_continues() {
        assert_eq!(
            NeverHalts.check(&thread_id()).await,
            SpendDecision::Continue
        );
        assert_eq!(
            NeverHalts.check(&thread_id()).await,
            SpendDecision::Continue
        );
    }

    #[tokio::test]
    async fn guard_is_object_safe_and_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Arc<dyn SpendGuard>>();

        let guard: Arc<dyn SpendGuard> = Arc::new(NeverHalts);
        assert_eq!(guard.check(&thread_id()).await, SpendDecision::Continue);
    }
}
