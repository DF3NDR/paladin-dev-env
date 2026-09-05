//! Graceful-shutdown primitives (HITL-04, D-21): [`ShutdownCoordinator`]
//! tracks every in-flight superstep run and lets a process-level shutdown
//! signal cancel all of them and wait for the batch to drain, or a grace
//! deadline to elapse, whichever comes first.
//!
//! Placed in `paladin-battalion` (not the facade) so embedded-library users
//! and Phase 27's worker pool can reuse it without depending on
//! `paladin-web`/`src/` (D-21). Composes only
//! `tokio_util::sync::CancellationToken` and `tokio::sync::Notify` -- both
//! already dependencies of this crate (`Cargo.toml`) -- introducing no
//! third-party shutdown-orchestration crate (RESEARCH.md's Alternatives
//! Considered table explicitly rejects one for this ~40-line primitive).
//!
//! RED state (Task 1, TDD): the tests below reference `ShutdownCoordinator`,
//! `RunGuard` and `ShutdownOutcome`, none of which are implemented yet.

use std::time::Duration;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

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
