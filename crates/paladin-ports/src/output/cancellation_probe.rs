//! # Cancellation Probe Port — Cross-Instance Cancellation (PLAT-FR-04, D-14)
//!
//! Defines [`CancellationProbe`], the seam a durable, cross-instance cancel
//! signal attaches to the superstep engine through. `WarEngine::
//! with_cancellation_probe` (`paladin-battalion`) consults an attached probe
//! at every superstep boundary, BESIDE — never instead of — the existing
//! in-process `CancellationToken` (`WarEngine::with_cancellation_token`):
//! either one answering "cancelled" halts the run identically.
//!
//! ## Why a probe, not just the existing token
//!
//! A `CancellationToken` is an in-process signal: only the worker instance
//! that holds the `Arc` can fire it. Phase 27's Platform API runs workers
//! across multiple instances (`RunWorkerPool`), so `POST /runs/{id}/cancel`
//! landing on instance B must still reach a run executing on instance A.
//! [`CancellationProbe`] is the engine-side half of that mechanism: an
//! adapter backed by the run repository (`DbCancellationProbe`, the facade
//! crate) answers "has someone durably requested cancellation for this
//! thread's run?" by reading a flag written through ANY instance's
//! repository handle.
//!
//! ## Infallible by design — a probe failure must never fail a run
//!
//! [`CancellationProbe::is_cancelled`] returns a plain `bool`, never a
//! `Result`. This mirrors [`crate::output::trace_sink_port`]'s "errors are
//! diagnostics only" contract, but goes one step further: there is no
//! `Result` at all for a caller to even consider inspecting. An adapter
//! backed by a database MUST swallow its own read failures internally (log
//! and answer `false`) rather than surface them here — a transient
//! repository error must never halt a run that nobody asked to cancel.
//!
//! ## Policy in the adapter, mechanism in the engine (D-15)
//!
//! The engine calls [`CancellationProbe::is_cancelled`] once per superstep
//! boundary, unconditionally and simply — it has no opinion about how
//! expensive that call is. Debouncing (caching an answer for a configured
//! interval so a fast graph cannot hammer the database) is entirely the
//! adapter's responsibility; see `DbCancellationProbe` in the facade crate.

use async_trait::async_trait;

use paladin_core::platform::container::waypoint::ThreadId;

/// Consulted by the superstep engine at every superstep boundary (D-14) to
/// decide whether a run should halt for a durable, possibly cross-instance,
/// cancellation request.
///
/// # Infallible by design
///
/// This trait returns a plain `bool`, not a `Result` — see the module-level
/// "Infallible by design" section. An implementation backed by a fallible
/// resource (a database, a network call) must catch its own errors, log
/// them, and answer `false`: a probe failure must never fail a run.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync`: a probe may be consulted
/// concurrently across nested `NodeSpec::Battalion` child runs and, in a
/// multi-worker deployment, across every run this process is executing.
#[async_trait]
pub trait CancellationProbe: Send + Sync {
    /// Answer whether `thread`'s current run has been durably requested to
    /// cancel. Called once per superstep boundary by the engine; a `true`
    /// answer produces the same `RunOutcome::Halted` path a cancelled
    /// `CancellationToken` does.
    ///
    /// Must never panic and must never block indefinitely — an
    /// implementation backed by I/O should apply its own timeout and answer
    /// `false` on failure rather than let a probe read stall the run loop.
    async fn is_cancelled(&self, thread: &ThreadId) -> bool;
}

/// A [`CancellationProbe`] that never reports cancellation.
///
/// Useful as an explicit default for a caller that wants to construct a
/// probe-shaped value without attaching real cross-instance cancellation —
/// equivalent in effect to never calling `WarEngine::with_cancellation_probe`
/// at all, but occasionally convenient where a concrete `Arc<dyn
/// CancellationProbe>` is required rather than an `Option`.
///
/// ```
/// use paladin_ports::output::cancellation_probe::{CancellationProbe, NeverCancelled};
/// use paladin_core::platform::container::waypoint::ThreadId;
///
/// #[tokio::main]
/// async fn main() {
///     let probe = NeverCancelled;
///     let thread = ThreadId::new("11111111-1111-7111-8111-111111111111").unwrap();
///     assert!(!probe.is_cancelled(&thread).await);
/// }
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NeverCancelled;

#[async_trait]
impl CancellationProbe for NeverCancelled {
    async fn is_cancelled(&self, _thread: &ThreadId) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    fn thread_id() -> ThreadId {
        ThreadId::new("11111111-1111-7111-8111-111111111111").unwrap()
    }

    #[tokio::test]
    async fn never_cancelled_always_answers_false() {
        let probe = NeverCancelled;
        assert!(!probe.is_cancelled(&thread_id()).await);
        assert!(!probe.is_cancelled(&thread_id()).await);
    }

    /// A mock probe that counts how many times it is consulted and answers
    /// `true` starting from a configured call number — used by
    /// `paladin-battalion`'s engine tests to prove the boundary is consulted
    /// exactly once per iteration.
    struct CountingProbe {
        calls: AtomicUsize,
        cancel_at_call: usize,
    }

    #[async_trait]
    impl CancellationProbe for CountingProbe {
        async fn is_cancelled(&self, _thread: &ThreadId) -> bool {
            let call = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
            call >= self.cancel_at_call
        }
    }

    #[tokio::test]
    async fn counting_probe_reports_cancelled_from_the_configured_call_onward() {
        let probe = CountingProbe {
            calls: AtomicUsize::new(0),
            cancel_at_call: 2,
        };
        let thread = thread_id();
        assert!(!probe.is_cancelled(&thread).await, "call 1: not yet");
        assert!(probe.is_cancelled(&thread).await, "call 2: cancelled");
        assert!(probe.is_cancelled(&thread).await, "call 3: stays cancelled");
        assert_eq!(probe.calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn probe_is_object_safe_and_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Arc<dyn CancellationProbe>>();

        let probe: Arc<dyn CancellationProbe> = Arc::new(NeverCancelled);
        assert!(!probe.is_cancelled(&thread_id()).await);
    }
}
