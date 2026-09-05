//! Graceful-shutdown primitives (HITL-04, D-21): [`ShutdownCoordinator`]
//! tracks every in-flight superstep run and lets a process-level shutdown
//! signal cancel all of them and wait for the batch to drain, or a grace
//! deadline to elapse, whichever comes first.
//!
//! Placed in `paladin-battalion` (not the facade) so embedded-library users
//! and Phase 27's worker pool can reuse it without depending on
//! `paladin-web`/`src/` (D-21). Composes only
//! [`tokio_util::sync::CancellationToken`] and [`tokio::sync::Notify`] --
//! both already dependencies of this crate (`Cargo.toml`) -- introducing no
//! third-party shutdown-orchestration crate (RESEARCH.md's Alternatives
//! Considered table explicitly rejects one for this ~40-line primitive).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

/// Shared state between a [`ShutdownCoordinator`] and every [`RunGuard`] it
/// has handed out.
struct Inner {
    root: CancellationToken,
    in_flight: AtomicUsize,
    idle: Notify,
}

/// Coordinates graceful shutdown across every in-flight superstep run
/// (HITL-04, D-21): a root [`CancellationToken`], an in-flight counter and a
/// [`Notify`] that wakes [`ShutdownCoordinator::cancel_and_wait`] the moment
/// the counter reaches zero.
///
/// Cloning is cheap (an `Arc` internally), so the SAME coordinator is shared
/// across `paladin-server.rs`'s `shutdown_signal` and
/// `ServiceRunner::wait_for_shutdown`, and every in-flight engine run the
/// facade starts registers with it (D-22, a later plan).
///
/// # Examples
///
/// ```
/// use paladin_battalion::engine::shutdown::ShutdownCoordinator;
/// use std::time::Duration;
///
/// # #[tokio::main]
/// # async fn main() {
/// let coordinator = ShutdownCoordinator::new();
/// // No runs registered: cancel_and_wait returns immediately, drained.
/// let outcome = coordinator.cancel_and_wait(Duration::from_secs(30)).await;
/// assert!(outcome.drained());
/// # }
/// ```
#[derive(Clone)]
pub struct ShutdownCoordinator {
    inner: Arc<Inner>,
}

impl Default for ShutdownCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl ShutdownCoordinator {
    /// Construct a coordinator with a fresh, uncancelled root token and no
    /// in-flight runs registered.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_battalion::engine::shutdown::ShutdownCoordinator;
    ///
    /// let coordinator = ShutdownCoordinator::new();
    /// assert_eq!(coordinator.in_flight(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                root: CancellationToken::new(),
                in_flight: AtomicUsize::new(0),
                idle: Notify::new(),
            }),
        }
    }

    /// The root [`CancellationToken`]. A registered run observes a CHILD of
    /// this token (via [`ShutdownCoordinator::register`]), never this one
    /// directly -- so no individual run can cancel every other registered
    /// run by cancelling its own token.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_battalion::engine::shutdown::ShutdownCoordinator;
    ///
    /// let coordinator = ShutdownCoordinator::new();
    /// assert!(!coordinator.token().is_cancelled());
    /// ```
    pub fn token(&self) -> CancellationToken {
        self.inner.root.clone()
    }

    /// Register a new in-flight run: increments the counter and returns a
    /// child [`CancellationToken`] (cancelled the moment the root is)
    /// paired with an RAII [`RunGuard`] whose `Drop` decrements the counter
    /// and wakes any waiter -- including when the guarded future panics or
    /// is aborted, since ordinary drop glue runs in both cases.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_battalion::engine::shutdown::ShutdownCoordinator;
    ///
    /// let coordinator = ShutdownCoordinator::new();
    /// let (child_token, guard) = coordinator.register();
    /// assert_eq!(coordinator.in_flight(), 1);
    /// assert!(!child_token.is_cancelled());
    /// drop(guard);
    /// assert_eq!(coordinator.in_flight(), 0);
    /// ```
    pub fn register(&self) -> (CancellationToken, RunGuard) {
        self.inner.in_flight.fetch_add(1, Ordering::SeqCst);
        let child = self.inner.root.child_token();
        let guard = RunGuard {
            inner: Arc::clone(&self.inner),
        };
        (child, guard)
    }

    /// Cancel the root token, then wait until every registered run has
    /// dropped its [`RunGuard`] (idle) or `grace` elapses, whichever comes
    /// first (D-19, D-21).
    ///
    /// `Duration::ZERO` never waits, even with runs still in flight --
    /// [`ShutdownOutcome::TimedOut`] is returned immediately after
    /// cancelling. Zero registered runs returns
    /// [`ShutdownOutcome::Drained`] immediately, regardless of `grace`.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_battalion::engine::shutdown::ShutdownCoordinator;
    /// use std::time::Duration;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let coordinator = ShutdownCoordinator::new();
    /// let (_token, guard) = coordinator.register();
    ///
    /// // Drop the guard from a background task so cancel_and_wait sees the
    /// // coordinator drain before its grace deadline.
    /// tokio::spawn(async move {
    ///     tokio::time::sleep(Duration::from_millis(10)).await;
    ///     drop(guard);
    /// });
    ///
    /// let outcome = coordinator.cancel_and_wait(Duration::from_secs(5)).await;
    /// assert!(outcome.drained());
    /// assert!(coordinator.token().is_cancelled());
    /// # }
    /// ```
    pub async fn cancel_and_wait(&self, grace: Duration) -> ShutdownOutcome {
        self.inner.root.cancel();

        if self.inner.in_flight.load(Ordering::SeqCst) == 0 {
            return ShutdownOutcome::Drained;
        }
        if grace.is_zero() {
            return ShutdownOutcome::TimedOut;
        }

        let deadline = tokio::time::Instant::now() + grace;
        loop {
            // --- `Notify`'s own check-subscribe-recheck contract: the
            // `notified()` future is created BEFORE re-checking the
            // counter, so a `RunGuard` dropping (and calling `notify_one`)
            // between the check below and the `.await` in `tokio::select!`
            // is never missed -- `Notify` buffers a notification raised
            // before its listener starts waiting on it.
            let notified = self.inner.idle.notified();
            if self.inner.in_flight.load(Ordering::SeqCst) == 0 {
                return ShutdownOutcome::Drained;
            }
            tokio::select! {
                _ = notified => {
                    // A single `notify_one` wakes at most one waiter, and
                    // this coordinator is the only one ever awaiting `idle`
                    // -- looping back to recheck the counter (rather than
                    // assuming idle here) is correct even when several
                    // `RunGuard`s drop in a tight burst, since each drop's
                    // own `notify_one` call is independent of how many
                    // OTHER guards have already dropped by the time this
                    // wakes.
                }
                _ = tokio::time::sleep_until(deadline) => {
                    return ShutdownOutcome::TimedOut;
                }
            }
        }
    }

    /// Current number of registered, not-yet-dropped runs.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_battalion::engine::shutdown::ShutdownCoordinator;
    ///
    /// let coordinator = ShutdownCoordinator::new();
    /// let (_token, guard_a) = coordinator.register();
    /// let (_token, guard_b) = coordinator.register();
    /// assert_eq!(coordinator.in_flight(), 2);
    /// drop(guard_a);
    /// assert_eq!(coordinator.in_flight(), 1);
    /// drop(guard_b);
    /// assert_eq!(coordinator.in_flight(), 0);
    /// ```
    pub fn in_flight(&self) -> usize {
        self.inner.in_flight.load(Ordering::SeqCst)
    }
}

/// RAII registration handle returned by [`ShutdownCoordinator::register`]:
/// dropping it (including via a panic unwind or a [`tokio::task::JoinHandle`]
/// abort, both of which run ordinary drop glue) decrements the coordinator's
/// in-flight counter and wakes [`ShutdownCoordinator::cancel_and_wait`] if it
/// is waiting.
///
/// Carries no public API beyond `Drop` -- callers hold it for its lifetime
/// alone, typically bound to `_` or a task-local variable spanning the
/// guarded run.
///
/// # Examples
///
/// ```
/// use paladin_battalion::engine::shutdown::ShutdownCoordinator;
///
/// let coordinator = ShutdownCoordinator::new();
/// let (_child_token, guard) = coordinator.register();
/// assert_eq!(coordinator.in_flight(), 1);
/// drop(guard);
/// assert_eq!(coordinator.in_flight(), 0);
/// ```
pub struct RunGuard {
    inner: Arc<Inner>,
}

impl Drop for RunGuard {
    fn drop(&mut self) {
        self.inner.in_flight.fetch_sub(1, Ordering::SeqCst);
        self.inner.idle.notify_one();
    }
}

/// Outcome of [`ShutdownCoordinator::cancel_and_wait`]: whether every
/// registered run dropped its [`RunGuard`] before `grace` elapsed, or the
/// deadline fired first with at least one run still outstanding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShutdownOutcome {
    /// Every registered run finished (dropped its `RunGuard`) before the
    /// grace deadline.
    Drained,
    /// The grace deadline elapsed with at least one run still registered.
    TimedOut,
}

impl ShutdownOutcome {
    /// `true` if every run drained before the deadline.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_battalion::engine::shutdown::ShutdownOutcome;
    ///
    /// assert!(ShutdownOutcome::Drained.drained());
    /// assert!(!ShutdownOutcome::TimedOut.drained());
    /// ```
    pub fn drained(&self) -> bool {
        matches!(self, Self::Drained)
    }

    /// `true` if the grace deadline elapsed with runs still outstanding.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_battalion::engine::shutdown::ShutdownOutcome;
    ///
    /// assert!(ShutdownOutcome::TimedOut.timed_out());
    /// assert!(!ShutdownOutcome::Drained.timed_out());
    /// ```
    pub fn timed_out(&self) -> bool {
        matches!(self, Self::TimedOut)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    // --- Test 1: register_returns_a_child_token_cancelled_by_the_root ----

    #[test]
    fn register_returns_a_child_token_cancelled_by_the_root() {
        let coordinator = ShutdownCoordinator::new();
        let (child_a, _guard_a) = coordinator.register();
        let (child_b, _guard_b) = coordinator.register();

        assert!(!child_a.is_cancelled());
        assert!(!child_b.is_cancelled());

        coordinator.token().cancel();

        assert!(
            child_a.is_cancelled(),
            "cancelling the root must cancel every child token handed out by register()"
        );
        assert!(
            child_b.is_cancelled(),
            "cancelling the root must cancel every child token handed out by register()"
        );
    }

    // --- Test 2: run_guard_decrements_in_flight_on_drop ------------------

    #[test]
    fn run_guard_decrements_in_flight_on_drop() {
        let coordinator = ShutdownCoordinator::new();
        let (_token, guard) = coordinator.register();
        assert_eq!(coordinator.in_flight(), 1);
        drop(guard);
        assert_eq!(coordinator.in_flight(), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn run_guard_decrements_in_flight_on_drop_even_when_the_guarded_future_panics() {
        let coordinator = ShutdownCoordinator::new();
        let (_token, guard) = coordinator.register();
        assert_eq!(coordinator.in_flight(), 1);

        let handle = tokio::spawn(async move {
            let _guard = guard;
            panic!("simulated node task panic while a RunGuard is held");
        });
        let result = handle.await;
        assert!(result.is_err(), "the spawned task must have panicked");

        assert_eq!(
            coordinator.in_flight(),
            0,
            "a RunGuard held by a panicking task must still decrement on unwind"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn run_guard_decrements_in_flight_on_drop_even_when_aborted() {
        let coordinator = ShutdownCoordinator::new();
        let (_token, guard) = coordinator.register();
        assert_eq!(coordinator.in_flight(), 1);

        let handle = tokio::spawn(async move {
            let _guard = guard;
            tokio::time::sleep(Duration::from_secs(60)).await;
        });
        // Give the task a chance to start and register its guard's ongoing
        // hold before aborting it mid-flight.
        tokio::task::yield_now().await;
        handle.abort();
        let _ = handle.await;

        assert_eq!(
            coordinator.in_flight(),
            0,
            "a RunGuard held by an aborted task must still decrement on drop"
        );
    }

    // --- Test 3: cancel_and_wait_returns_when_idle -----------------------

    #[tokio::test]
    async fn cancel_and_wait_returns_when_idle() {
        let coordinator = ShutdownCoordinator::new();
        let (_token, guard) = coordinator.register();

        let coordinator_clone = coordinator.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            drop(guard);
            let _ = &coordinator_clone;
        });

        let started = tokio::time::Instant::now();
        let outcome = coordinator.cancel_and_wait(Duration::from_secs(10)).await;
        let elapsed = started.elapsed();

        assert!(outcome.drained());
        assert!(
            elapsed < Duration::from_secs(5),
            "cancel_and_wait must return as soon as the last guard drops, not at the deadline \
             (elapsed: {elapsed:?})"
        );
    }

    // --- Test 4: cancel_and_wait_returns_at_the_deadline_when_not_idle ---

    #[tokio::test]
    async fn cancel_and_wait_returns_at_the_deadline_when_not_idle() {
        let coordinator = ShutdownCoordinator::new();
        let (_token, guard) = coordinator.register();

        let outcome = coordinator
            .cancel_and_wait(Duration::from_millis(50))
            .await;

        assert!(outcome.timed_out());
        assert_eq!(
            coordinator.in_flight(),
            1,
            "the never-dropped guard is still registered"
        );
        drop(guard);
    }

    // --- Test 5: cancel_and_wait_with_zero_registered_runs_returns_immediately

    #[tokio::test]
    async fn cancel_and_wait_with_zero_registered_runs_returns_immediately() {
        let coordinator = ShutdownCoordinator::new();
        let started = tokio::time::Instant::now();
        let outcome = coordinator.cancel_and_wait(Duration::from_secs(30)).await;
        assert!(outcome.drained());
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(coordinator.token().is_cancelled());
    }

    // --- Test 6: cancel_and_wait_with_zero_grace_does_not_wait -----------

    #[tokio::test]
    async fn cancel_and_wait_with_zero_grace_does_not_wait() {
        let coordinator = ShutdownCoordinator::new();
        let (_token, guard) = coordinator.register();

        let started = tokio::time::Instant::now();
        let outcome = coordinator.cancel_and_wait(Duration::ZERO).await;
        let elapsed = started.elapsed();

        assert!(outcome.timed_out());
        assert!(
            elapsed < Duration::from_millis(200),
            "Duration::ZERO must never wait, even with a run still in flight (elapsed: \
             {elapsed:?})"
        );
        drop(guard);
    }

    // --- Test 7: coordinator_is_send_sync_and_shareable ------------------

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn coordinator_is_send_sync_and_shareable() {
        assert_send_sync::<ShutdownCoordinator>();
        assert_send_sync::<RunGuard>();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn coordinator_is_usable_behind_an_arc_from_multiple_tasks() {
        let coordinator = Arc::new(ShutdownCoordinator::new());
        let concurrent_registrations = 8;
        let started = Arc::new(AtomicUsize::new(0));

        let mut handles = Vec::new();
        for _ in 0..concurrent_registrations {
            let coordinator = Arc::clone(&coordinator);
            let started = Arc::clone(&started);
            handles.push(tokio::spawn(async move {
                let (_token, guard) = coordinator.register();
                started.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(20)).await;
                drop(guard);
            }));
        }
        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(started.load(Ordering::SeqCst), concurrent_registrations);
        assert_eq!(coordinator.in_flight(), 0);

        let outcome = coordinator.cancel_and_wait(Duration::from_secs(5)).await;
        assert!(outcome.drained());
    }
}
