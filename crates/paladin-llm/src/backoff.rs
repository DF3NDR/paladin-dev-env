//! The one exponential back-off calculation the retrying adapters share.
//!
//! `max_retries` is an operator knob (a config block, an environment variable, `with_max_retries`),
//! so the attempt number reaching a back-off calculation is not bounded by the code. A raw
//! `2_u32.pow(attempt)` panics in a debug build and silently wraps to a zero wait in a release
//! build once the attempt passes 31, and an uncapped wait grows into hours long before that.
//! [`exponential_backoff`] saturates instead: it never overflows and never exceeds its cap.

use std::time::Duration;

/// `base * 2^attempt`, never more than `cap`.
///
/// An `attempt` too large for the shift, or a product too large for [`Duration`], yields `cap`.
pub(crate) fn exponential_backoff(base: Duration, attempt: u32, cap: Duration) -> Duration {
    let factor = 1_u32.checked_shl(attempt).unwrap_or(u32::MAX);
    base.checked_mul(factor).map_or(cap, |wait| wait.min(cap))
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: Duration = Duration::from_millis(100);
    const CAP: Duration = Duration::from_secs(10);

    #[test]
    fn it_doubles_per_attempt_below_the_cap() {
        assert_eq!(
            exponential_backoff(BASE, 0, CAP),
            Duration::from_millis(100)
        );
        assert_eq!(
            exponential_backoff(BASE, 1, CAP),
            Duration::from_millis(200)
        );
        assert_eq!(
            exponential_backoff(BASE, 3, CAP),
            Duration::from_millis(800)
        );
    }

    #[test]
    fn it_never_exceeds_the_cap() {
        assert_eq!(exponential_backoff(BASE, 7, CAP), CAP);
        assert_eq!(exponential_backoff(BASE, 15, CAP), CAP);
    }

    #[test]
    fn it_does_not_overflow_for_any_attempt() {
        for attempt in [31, 32, 33, 57, 64, 1_000, u32::MAX] {
            assert_eq!(exponential_backoff(BASE, attempt, CAP), CAP, "{attempt}");
            assert_eq!(
                exponential_backoff(Duration::from_secs(1), attempt, CAP),
                CAP,
                "{attempt}"
            );
        }
    }
}
