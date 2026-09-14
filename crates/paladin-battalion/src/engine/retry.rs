//! The Aegis retry loop's own backoff math and cancellation-aware wait
//! (Doc 04 FT-FR-02, D-15).
//!
//! Owned here, separate from `superstep.rs`'s dispatch loop, so the exact
//! backoff sequence and predicate-gating logic are independently unit
//! testable without spinning up a whole superstep run. `superstep.rs`'s
//! per-node retry loop calls [`should_retry`] to decide whether to retry a
//! failed attempt, then [`backoff_delay`] and [`wait_backoff`] to wait
//! before the next one.
//!
//! # The paused-clock idiom (established here, copied by later plans)
//!
//! This module's own tests are the FIRST use of `tokio::time::pause`
//! anywhere in this repository (RESEARCH.md's Wave-0 Gaps note: zero prior
//! usage). `#[tokio::test(start_paused = true)]` plus measuring elapsed
//! time via `tokio::time::Instant::now()` deltas around an awaited
//! `wait_backoff` call is the shape plan 25-09's idle-timeout tests and
//! plan 25-12's kill-during-backoff test copy, rather than re-deriving
//! their own paused-clock convention.

use std::time::Duration;

use rand::Rng;
use tokio_util::sync::CancellationToken;

use paladin_core::platform::container::aegis::RetryPolicy;
use paladin_core::platform::container::node_error::NodeError;

/// The delay before attempt `attempt` (`attempt >= 2`; the delay before
/// attempt 1 is always zero -- there is no wait before the FIRST attempt).
///
/// `min(initial_interval * backoff_factor^(attempt - 2), max_interval)`,
/// then, if `policy.jitter`, plus a uniform random amount in
/// `[0, delay)` -- so a jittered wait lies in `[base, 2 * base)` (D-15,
/// FT-FR-04).
///
/// # Examples
///
/// ```
/// use paladin_battalion::engine::retry::backoff_delay;
/// use paladin_core::platform::container::aegis::RetryPolicy;
/// use std::time::Duration;
///
/// let policy = RetryPolicy {
///     jitter: false,
///     ..RetryPolicy::default()
/// };
/// assert_eq!(backoff_delay(&policy, 2), Duration::from_millis(500));
/// assert_eq!(backoff_delay(&policy, 3), Duration::from_millis(1000));
/// ```
pub fn backoff_delay(policy: &RetryPolicy, attempt: u32) -> Duration {
    let exponent = attempt.saturating_sub(2);
    let base_millis = policy.initial_interval.as_secs_f64()
        * 1000.0
        * policy.backoff_factor.powi(exponent as i32);
    let max_millis = policy.max_interval.as_secs_f64() * 1000.0;
    let capped_millis = base_millis.min(max_millis).max(0.0);
    let base = Duration::from_millis(capped_millis as u64);

    if !policy.jitter || base.is_zero() {
        return base;
    }
    let base_millis_u64 = base.as_millis() as u64;
    let jitter_millis = rand::thread_rng().gen_range(0..base_millis_u64);
    base + Duration::from_millis(jitter_millis)
}

/// Await `delay`, racing it against `token`'s cancellation (D-15,
/// RESEARCH.md Pitfall 7): a node sleeping in backoff at shutdown is
/// aborted immediately rather than burning the shutdown grace window.
/// This is always an async, cancellation-aware wait -- never a blocking
/// OS-thread sleep -- see the `tokio::select!` below.
///
/// Returns `true` if the full `delay` elapsed, `false` if `token` fired
/// first (in which case the caller must not retry -- the run is shutting
/// down).
pub async fn wait_backoff(delay: Duration, token: &Option<CancellationToken>) -> bool {
    match token {
        Some(token) => {
            tokio::select! {
                biased;
                _ = token.cancelled() => false,
                () = tokio::time::sleep(delay) => true,
            }
        }
        None => {
            tokio::time::sleep(delay).await;
            true
        }
    }
}

/// Whether `policy.retry_on` retries an error at `err.transience`.
///
/// `attempt` is accepted for a stable signature (a future `Custom`
/// predicate evaluator, plan 25-03, will need it) but unused today.
///
/// Delegates entirely to
/// [`RetryPredicate::admits`](paladin_core::platform::container::aegis::RetryPredicate::admits)
/// (Phase 26 D-11): the transience-to-boolean decision has exactly one
/// home, in `paladin-core`'s `aegis` module, and this function is one of
/// its two callers (the other is
/// `paladin::application::services::paladin::middleware::resilience`'s
/// `ModelRetryMiddleware`, plan 26-10). `RetryPredicate` being
/// `#[non_exhaustive]` (D-09) is handled inside `admits` itself, so this
/// call site needs no wildcard arm of its own.
pub fn should_retry(policy: &RetryPolicy, err: &NodeError, _attempt: u32) -> bool {
    policy.retry_on.admits(err.transience)
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::aegis::RetryPredicate;
    use paladin_core::platform::container::node_error::NodeErrorSource;
    use paladin_core::platform::container::transience::Transience;
    use paladin_core::platform::container::waypoint::NodeId;

    fn policy(jitter: bool) -> RetryPolicy {
        RetryPolicy {
            max_attempts: 5,
            jitter,
            ..RetryPolicy::default()
        }
    }

    fn error_with_transience(transience: Transience) -> NodeError {
        NodeError {
            node_id: NodeId::new("n"),
            attempt: 1,
            transience,
            source: NodeErrorSource::Function {
                message: "boom".to_string(),
            },
        }
    }

    #[test]
    fn backoff_delay_before_attempt_one_is_conceptually_unused_but_well_defined() {
        // attempt=2 is the first meaningful call site (no wait precedes
        // attempt 1); backoff_delay(policy, 2) is the initial_interval.
        let p = policy(false);
        assert_eq!(backoff_delay(&p, 2), Duration::from_millis(500));
    }

    #[test]
    fn backoff_delay_is_capped_at_max_interval() {
        let p = RetryPolicy {
            max_interval: Duration::from_millis(1500),
            jitter: false,
            ..policy(false)
        };
        assert_eq!(backoff_delay(&p, 5), Duration::from_millis(1500));
    }

    /// D-15/FT-FR-05: a `Permanent`-classified error is never retried under
    /// `TransientOnly`, at every attempt number below `max_attempts` -- not
    /// just the first.
    #[test]
    fn permanent_error_under_transient_only_takes_one_attempt() {
        let p = RetryPolicy {
            retry_on: RetryPredicate::TransientOnly,
            ..policy(false)
        };
        let err = error_with_transience(Transience::Permanent);
        for attempt in 1..p.max_attempts {
            assert!(
                !should_retry(&p, &err, attempt),
                "attempt {attempt}: Permanent must never retry under TransientOnly"
            );
        }
    }

    /// D-15/FT-FR-05: `Transient` retries under either predicate;
    /// `Unknown` retries only under `TransientAndUnknown`, never under the
    /// default `TransientOnly`.
    #[test]
    fn transient_error_is_retried_and_unknown_is_gated_by_the_predicate() {
        let transient_only = RetryPolicy {
            retry_on: RetryPredicate::TransientOnly,
            ..policy(false)
        };
        let transient_and_unknown = RetryPolicy {
            retry_on: RetryPredicate::TransientAndUnknown,
            ..policy(false)
        };

        assert!(should_retry(
            &transient_only,
            &error_with_transience(Transience::Transient),
            1
        ));
        assert!(should_retry(
            &transient_and_unknown,
            &error_with_transience(Transience::Transient),
            1
        ));
        assert!(!should_retry(
            &transient_only,
            &error_with_transience(Transience::Unknown),
            1
        ));
        assert!(should_retry(
            &transient_and_unknown,
            &error_with_transience(Transience::Unknown),
            1
        ));
    }

    #[test]
    fn should_retry_custom_predicate_never_retries_here() {
        let p = RetryPolicy {
            retry_on: RetryPredicate::Custom("my-predicate".to_string()),
            ..policy(false)
        };
        assert!(!should_retry(
            &p,
            &error_with_transience(Transience::Transient),
            1
        ));
    }

    /// D-09: `Aegis::validate` (the entry point Task 2 created) rejects
    /// `max_attempts == 0` with the typed `AegisError` variant naming the
    /// field -- never a silent no-op, never interpreted as unlimited
    /// retries. Exercised from `paladin-battalion` (not just
    /// `paladin-core`'s own unit test) so this crate's own `cargo test`
    /// run pins the contract its retry loop depends on.
    #[test]
    fn max_attempts_zero_is_a_typed_validation_error() {
        use paladin_core::platform::container::aegis::{Aegis, AegisError};

        let aegis = Aegis {
            retry: Some(RetryPolicy {
                max_attempts: 0,
                ..RetryPolicy::default()
            }),
            ..Default::default()
        };
        assert_eq!(aegis.validate(), Err(AegisError::RetryMaxAttemptsZero));
    }

    #[tokio::test]
    async fn wait_backoff_with_no_token_always_completes() {
        let completed = wait_backoff(Duration::from_millis(1), &None).await;
        assert!(completed);
    }

    #[tokio::test]
    async fn wait_backoff_returns_false_when_already_cancelled() {
        let token = CancellationToken::new();
        token.cancel();
        let completed = wait_backoff(Duration::from_secs(60), &Some(token)).await;
        assert!(!completed);
    }

    // --- Paused-clock tests (established here; copied by plans 25-09/25-12) --

    /// D-15/FT-FR-04: under `RetryPolicy::default()` with `jitter: false`,
    /// a 5-attempt policy waits exactly 500ms, 1000ms, 2000ms, 4000ms
    /// before attempts 2, 3, 4, 5. `#[tokio::test(start_paused = true)]`
    /// auto-advances the virtual clock across each awaited
    /// `wait_backoff`, so the `tokio::time::Instant` delta measured around
    /// it is the code's own delay, never a real-wall-clock measurement.
    #[tokio::test(start_paused = true)]
    async fn backoff_sequence_is_exact_with_jitter_off() {
        let p = policy(false);
        let mut waits = Vec::with_capacity(4);
        for attempt in 2..=5u32 {
            let delay = backoff_delay(&p, attempt);
            let start = tokio::time::Instant::now();
            assert!(wait_backoff(delay, &None).await);
            waits.push(start.elapsed());
        }
        assert_eq!(
            waits,
            vec![
                Duration::from_millis(500),
                Duration::from_millis(1000),
                Duration::from_millis(2000),
                Duration::from_millis(4000),
            ]
        );
    }

    /// D-15: with `max_interval: 3s`, the fourth wait (before attempt 5)
    /// is 3000ms, not 4000ms.
    #[tokio::test(start_paused = true)]
    async fn backoff_is_capped_at_max_interval() {
        let p = RetryPolicy {
            max_interval: Duration::from_secs(3),
            jitter: false,
            ..policy(false)
        };
        let delay = backoff_delay(&p, 5);
        let start = tokio::time::Instant::now();
        assert!(wait_backoff(delay, &None).await);
        assert_eq!(start.elapsed(), Duration::from_millis(3000));
    }

    /// D-15/FT-FR-04: with `jitter: true`, each of 200 sampled waits per
    /// attempt lies in `[base, 2 * base)` -- bounds only, never an exact
    /// value or a statistical distribution.
    #[tokio::test(start_paused = true)]
    async fn backoff_with_jitter_stays_within_bounds() {
        let jittered = policy(true);
        let unjittered = policy(false);
        for attempt in 2..=5u32 {
            let base = backoff_delay(&unjittered, attempt);
            for _ in 0..200 {
                let delay = backoff_delay(&jittered, attempt);
                let start = tokio::time::Instant::now();
                assert!(wait_backoff(delay, &None).await);
                let elapsed = start.elapsed();
                assert!(
                    elapsed >= base && elapsed < base * 2,
                    "attempt {attempt}: {elapsed:?} not in [{base:?}, {:?})",
                    base * 2
                );
            }
        }
    }

    /// D-15, RESEARCH.md Pitfall 7: `wait_backoff` with a long delay whose
    /// `CancellationToken` is cancelled WHILE the wait is outstanding (not
    /// before it starts -- `wait_backoff_returns_false_when_already_cancelled`
    /// above already covers that case) returns immediately reporting the
    /// wait did not complete, rather than burning the full delay.
    #[tokio::test(start_paused = true)]
    async fn backoff_wait_returns_early_when_the_run_is_cancelled() {
        let token = CancellationToken::new();
        let waiter_token = token.clone();
        let handle = tokio::spawn(async move {
            let start = tokio::time::Instant::now();
            let completed = wait_backoff(Duration::from_secs(60), &Some(waiter_token)).await;
            (completed, start.elapsed())
        });
        // Let the spawned task actually start polling (and register its
        // sleep timer) before cancelling, so this genuinely exercises the
        // mid-wait race rather than the already-cancelled case.
        tokio::task::yield_now().await;
        token.cancel();
        let (completed, elapsed) = handle.await.expect("wait_backoff task panicked");
        assert!(!completed);
        assert!(
            elapsed < Duration::from_secs(60),
            "expected an early return, waited {elapsed:?}"
        );
    }
}
