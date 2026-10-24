//! # Rate-limit hints -- the numbers a provider's 429 carries (PACE-01, D-04)
//!
//! A [`RateLimitHints`] is the typed snapshot of what a provider told us when it answered
//! `429 Too Many Requests`: an explicit retry delay (`Retry-After` / `retry-after-ms`) and, per
//! [`RateLimitDimensionKind`], the limit, the remaining allowance and the time until it resets.
//! It rides on [`LlmError::RateLimitExceeded`](crate::output::llm_port::LlmError) so a caller, a
//! `RetryPolicy` above the Cadence decorator and the decorator itself can all read the provider's
//! own number instead of guessing one.
//!
//! ## Parsed numbers only
//!
//! Header parsing happens at the HTTP edge, in `paladin-llm`; this module is the port-layer
//! vocabulary it fills. The types here store parsed integers and [`Duration`]s -- there is
//! deliberately no field that holds a raw header string, because header values are
//! provider-controlled and would otherwise be rendered by `Debug` wherever the error is logged.
//!
//! ## Hints inform; they never gate on their own (D-04)
//!
//! Remaining and reset values are carried for operators and for a future proactive-pacing phase.
//! The Cadence gate opens on a 429 only, so a `remaining` figure alone never closes it. The one
//! number the gate acts on is the one [`RateLimitHints::effective_retry_after`] returns, and it
//! returns `None` -- never a guessed value -- when the provider said nothing usable.
//!
//! # Examples
//!
//! ```
//! use std::time::Duration;
//! use paladin_ports::output::rate_limit_hints::{
//!     RateLimitDimension, RateLimitDimensionKind, RateLimitHints, RetryDelaySource,
//! };
//!
//! // The provider reported zero requests left, resetting in 12 s.
//! let hints = RateLimitHints::default().with_dimension(
//!     RateLimitDimensionKind::Requests,
//!     RateLimitDimension::new(Some(60), Some(0), Some(Duration::from_secs(12))),
//! );
//! assert_eq!(
//!     hints.effective_retry_after(),
//!     Some((Duration::from_secs(12), RetryDelaySource::ResetHeader))
//! );
//! ```

use std::time::Duration;

/// Which quota a [`RateLimitDimension`] describes.
///
/// # Examples
///
/// ```
/// use paladin_ports::output::rate_limit_hints::RateLimitDimensionKind;
///
/// assert_ne!(RateLimitDimensionKind::Requests, RateLimitDimensionKind::Tokens);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum RateLimitDimensionKind {
    /// Requests per window.
    Requests,
    /// Total tokens per window.
    Tokens,
    /// Input (prompt) tokens per window.
    InputTokens,
    /// Output (completion) tokens per window.
    OutputTokens,
}

impl RateLimitDimensionKind {
    /// The slot this kind occupies in [`RateLimitHints`]. Exhaustive on purpose: a new kind
    /// fails to compile here until it is given a slot.
    fn slot(self) -> usize {
        match self {
            RateLimitDimensionKind::Requests => 0,
            RateLimitDimensionKind::Tokens => 1,
            RateLimitDimensionKind::InputTokens => 2,
            RateLimitDimensionKind::OutputTokens => 3,
        }
    }
}

/// Where an explicit retry delay came from. [`RetryDelaySource::ResetHeader`] marks a delay that
/// was *derived* from a reset time rather than stated by the provider, which the Cadence
/// decorator bounds because a full-replenishment reset over-estimates the provider's minimum.
///
/// # Examples
///
/// ```
/// use paladin_ports::output::rate_limit_hints::RetryDelaySource;
///
/// assert_ne!(RetryDelaySource::RetryAfter, RetryDelaySource::ResetHeader);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RetryDelaySource {
    /// The standard `Retry-After` header.
    RetryAfter,
    /// A millisecond-precision `retry-after-ms` header.
    RetryAfterMs,
    /// Derived from a rate-limit reset header, not stated as a retry delay.
    ResetHeader,
}

/// One quota's parsed state: its limit, what remains and the time until it resets. Every field
/// is optional because providers report different subsets.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use paladin_ports::output::rate_limit_hints::RateLimitDimension;
///
/// let dim = RateLimitDimension::new(Some(100), Some(0), Some(Duration::from_secs(3)));
/// assert!(dim.is_exhausted());
/// assert_eq!(dim.reset_after(), Some(Duration::from_secs(3)));
/// assert!(!RateLimitDimension::default().is_exhausted());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub struct RateLimitDimension {
    limit: Option<u64>,
    remaining: Option<u64>,
    reset_after: Option<Duration>,
}

impl RateLimitDimension {
    /// Build a dimension from whatever the provider reported.
    pub fn new(limit: Option<u64>, remaining: Option<u64>, reset_after: Option<Duration>) -> Self {
        Self {
            limit,
            remaining,
            reset_after,
        }
    }

    /// The quota ceiling for the window, when reported.
    pub fn limit(&self) -> Option<u64> {
        self.limit
    }

    /// What is left of the quota, when reported.
    pub fn remaining(&self) -> Option<u64> {
        self.remaining
    }

    /// Time until the quota replenishes, when reported.
    pub fn reset_after(&self) -> Option<Duration> {
        self.reset_after
    }

    /// `true` only when the provider explicitly reported `remaining == 0`.
    pub fn is_exhausted(&self) -> bool {
        self.remaining == Some(0)
    }
}

/// The parsed rate-limit snapshot of one provider response (see the module docs).
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use paladin_ports::output::rate_limit_hints::{RateLimitHints, RetryDelaySource};
///
/// let hints = RateLimitHints::default()
///     .with_retry_after(Duration::from_secs(7), RetryDelaySource::RetryAfter);
/// assert_eq!(
///     hints.explicit_retry_after(),
///     Some((Duration::from_secs(7), RetryDelaySource::RetryAfter))
/// );
/// assert!(!hints.is_empty());
/// assert!(RateLimitHints::default().is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct RateLimitHints {
    explicit: Option<(Duration, RetryDelaySource)>,
    dimensions: [RateLimitDimension; 4],
}

impl RateLimitHints {
    /// Record the provider's explicit retry delay and where it came from.
    #[must_use]
    pub fn with_retry_after(mut self, delay: Duration, source: RetryDelaySource) -> Self {
        self.explicit = Some((delay, source));
        self
    }

    /// Record one quota dimension, replacing any earlier value for that kind.
    #[must_use]
    pub fn with_dimension(
        mut self,
        kind: RateLimitDimensionKind,
        dimension: RateLimitDimension,
    ) -> Self {
        self.dimensions[kind.slot()] = dimension;
        self
    }

    /// The provider's explicit delay and its source, when it stated one.
    pub fn explicit_retry_after(&self) -> Option<(Duration, RetryDelaySource)> {
        self.explicit
    }

    /// One dimension. A kind the provider did not report is the all-`None` default.
    pub fn dimension(&self, kind: RateLimitDimensionKind) -> &RateLimitDimension {
        &self.dimensions[kind.slot()]
    }

    /// `true` when nothing at all was recorded.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// The single delay the Cadence gate may act on, with its source (D-04).
    ///
    /// Precedence:
    ///
    /// 1. An explicit `Retry-After` / `retry-after-ms` delay.
    /// 2. Otherwise the **largest** reset among dimensions reporting `remaining == 0` (every
    ///    exhausted quota must have replenished), tagged [`RetryDelaySource::ResetHeader`].
    /// 3. Otherwise, only when **no** dimension reports `remaining`, the **smallest** reset
    ///    reported, also tagged [`RetryDelaySource::ResetHeader`].
    /// 4. Otherwise `None` -- a delay is never guessed.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use paladin_ports::output::rate_limit_hints::{
    ///     RateLimitDimension, RateLimitDimensionKind, RateLimitHints, RetryDelaySource,
    /// };
    ///
    /// let secs = Duration::from_secs;
    /// let hints = RateLimitHints::default()
    ///     .with_dimension(
    ///         RateLimitDimensionKind::Requests,
    ///         RateLimitDimension::new(None, Some(0), Some(secs(5))),
    ///     )
    ///     .with_dimension(
    ///         RateLimitDimensionKind::Tokens,
    ///         RateLimitDimension::new(None, Some(0), Some(secs(20))),
    ///     );
    /// assert_eq!(
    ///     hints.effective_retry_after(),
    ///     Some((secs(20), RetryDelaySource::ResetHeader))
    /// );
    /// assert_eq!(RateLimitHints::default().effective_retry_after(), None);
    /// ```
    pub fn effective_retry_after(&self) -> Option<(Duration, RetryDelaySource)> {
        if let Some(explicit) = self.explicit {
            return Some(explicit);
        }
        let largest_exhausted_reset = self
            .dimensions
            .iter()
            .filter(|dim| dim.is_exhausted())
            .filter_map(|dim| dim.reset_after)
            .max();
        if let Some(reset) = largest_exhausted_reset {
            return Some((reset, RetryDelaySource::ResetHeader));
        }
        if self.dimensions.iter().all(|dim| dim.remaining.is_none()) {
            return self
                .dimensions
                .iter()
                .filter_map(|dim| dim.reset_after)
                .min()
                .map(|reset| (reset, RetryDelaySource::ResetHeader));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    fn dim(remaining: Option<u64>, reset: Option<u64>) -> RateLimitDimension {
        RateLimitDimension::new(None, remaining, reset.map(secs))
    }

    #[test]
    fn hints_explicit_retry_after_wins_over_resets() {
        let hints = RateLimitHints::default()
            .with_dimension(RateLimitDimensionKind::Requests, dim(Some(0), Some(99)))
            .with_retry_after(secs(7), RetryDelaySource::RetryAfterMs);
        assert_eq!(
            hints.effective_retry_after(),
            Some((secs(7), RetryDelaySource::RetryAfterMs))
        );
    }

    #[test]
    fn hints_largest_reset_among_exhausted_dimensions_when_no_explicit_delay() {
        let hints = RateLimitHints::default()
            .with_dimension(RateLimitDimensionKind::Requests, dim(Some(0), Some(4)))
            .with_dimension(RateLimitDimensionKind::Tokens, dim(Some(0), Some(11)))
            // Not exhausted: its (larger) reset must not count.
            .with_dimension(RateLimitDimensionKind::InputTokens, dim(Some(5), Some(60)));
        assert_eq!(
            hints.effective_retry_after(),
            Some((secs(11), RetryDelaySource::ResetHeader))
        );
    }

    #[test]
    fn hints_smallest_reset_when_no_dimension_reports_remaining() {
        let hints = RateLimitHints::default()
            .with_dimension(RateLimitDimensionKind::Requests, dim(None, Some(30)))
            .with_dimension(RateLimitDimensionKind::Tokens, dim(None, Some(9)));
        assert_eq!(
            hints.effective_retry_after(),
            Some((secs(9), RetryDelaySource::ResetHeader))
        );
    }

    #[test]
    fn hints_none_when_nothing_is_known() {
        assert_eq!(RateLimitHints::default().effective_retry_after(), None);

        // Remaining > 0 everywhere: the provider is not saying "wait".
        let healthy = RateLimitHints::default()
            .with_dimension(RateLimitDimensionKind::Requests, dim(Some(3), Some(10)))
            .with_dimension(RateLimitDimensionKind::Tokens, dim(Some(100), Some(20)));
        assert_eq!(healthy.effective_retry_after(), None);

        // Exhausted but no reset reported: still no guess, even though a sibling reports one.
        let no_reset = RateLimitHints::default()
            .with_dimension(RateLimitDimensionKind::Requests, dim(Some(0), None))
            .with_dimension(RateLimitDimensionKind::Tokens, dim(None, Some(5)));
        assert_eq!(no_reset.effective_retry_after(), None);
    }

    #[test]
    fn is_empty_tracks_every_field() {
        assert!(RateLimitHints::default().is_empty());
        assert!(
            !RateLimitHints::default()
                .with_retry_after(secs(1), RetryDelaySource::RetryAfter)
                .is_empty()
        );
        assert!(
            !RateLimitHints::default()
                .with_dimension(RateLimitDimensionKind::OutputTokens, dim(Some(1), None))
                .is_empty()
        );
    }

    #[test]
    fn dimensions_are_stored_per_kind() {
        let hints = RateLimitHints::default()
            .with_dimension(RateLimitDimensionKind::Tokens, dim(Some(2), Some(3)));
        assert_eq!(
            hints.dimension(RateLimitDimensionKind::Tokens).remaining(),
            Some(2)
        );
        assert_eq!(
            hints.dimension(RateLimitDimensionKind::Requests),
            &RateLimitDimension::default()
        );
    }

    #[test]
    fn debug_renders_numbers_only() {
        let hints = RateLimitHints::default()
            .with_retry_after(secs(2), RetryDelaySource::RetryAfter)
            .with_dimension(RateLimitDimensionKind::Requests, dim(Some(0), Some(1)));
        let rendered = format!("{hints:?}");
        assert!(rendered.contains("RetryAfter"));
        assert!(
            !rendered.contains('"'),
            "no string payload exists: {rendered}"
        );
    }
}
