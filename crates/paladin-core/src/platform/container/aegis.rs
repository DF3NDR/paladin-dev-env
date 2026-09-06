//! The whole per-node fault-tolerance policy family (Doc 04 §2.1, D-09):
//! [`Aegis`] attaches to a node as a `NodeId`-keyed sidecar on the
//! superstep engine's `WarGraph` (`paladin_battalion::engine::graph`, D-10),
//! never as a field on any `NodeSpec` variant. Every type here lands in its
//! final shape: no
//! later plan reshapes `Aegis` or any of its policy families, only reads
//! them (retry: this plan; timeout: plan 25-09; `on_error`: plans 25-10/11;
//! `cache`: plans 25-04/13).
//!
//! This is a distinct type family from the pre-existing v0.9
//! `crate::platform::container::battalion::{RetryPolicy, ErrorStrategy}`
//! (D-09, D-06): that legacy pair stays untouched under X-03, and the core
//! prelude (`lib.rs`) re-exports [`Aegis`] and
//! [`crate::platform::container::transience::Transience`] only -- never
//! either `RetryPolicy` -- so no glob import can silently bind the wrong
//! one (RESEARCH.md Pitfall 5).

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::platform::container::battlefield::{FieldName, StateDelta};
use crate::platform::container::waypoint::NodeId;

/// The whole per-node fault-tolerance policy (D-09): every field is
/// independently optional, so a node can opt into e.g. only a timeout with
/// no retry, or only a cache with neither.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::aegis::{Aegis, RetryPolicy};
///
/// let aegis = Aegis {
///     retry: Some(RetryPolicy::default()),
///     ..Default::default()
/// };
/// aegis.validate()?;
/// # Ok::<(), paladin_core::platform::container::aegis::AegisError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Aegis {
    /// Retry policy, if this node retries a failed attempt (Doc 04
    /// FT-FR-02).
    pub retry: Option<RetryPolicy>,
    /// Timeout policy, if this node is time-bounded (plan 25-09).
    pub timeout: Option<TimeoutPolicy>,
    /// Typed error handler, if a failure here should be compensated instead
    /// of failing the run (plans 25-10/11).
    pub on_error: Option<ErrorHandlerSpec>,
    /// Cache policy, if this node's result may be served from a cache
    /// (plans 25-04/13).
    pub cache: Option<CachePolicy>,
}

impl Aegis {
    /// Validate this policy's own invariants -- never the per-graph wiring
    /// (an undeclared route target, a node-kind mismatch): that validation
    /// belongs to `WarGraph::validate` (plan 25-03), which calls this first
    /// and then layers its own checks on top.
    ///
    /// Currently rejects only `RetryPolicy::max_attempts == 0` (D-09): never
    /// interpreted as unlimited retries, and never silently treated as a
    /// single attempt.
    pub fn validate(&self) -> Result<(), AegisError> {
        if let Some(retry) = &self.retry
            && retry.max_attempts == 0
        {
            return Err(AegisError::RetryMaxAttemptsZero);
        }
        Ok(())
    }
}

/// Error returned by [`Aegis::validate`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum AegisError {
    /// `RetryPolicy.max_attempts` was `0`.
    #[error("RetryPolicy.max_attempts must be at least 1, got 0")]
    RetryMaxAttemptsZero,
}

/// How a node retries a failed attempt (Doc 04 FT-FR-02, D-15).
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::aegis::{RetryPolicy, RetryPredicate};
/// use std::time::Duration;
///
/// let default_policy = RetryPolicy::default();
/// assert_eq!(default_policy.max_attempts, 3);
/// assert_eq!(default_policy.initial_interval, Duration::from_millis(500));
/// assert_eq!(default_policy.retry_on, RetryPredicate::TransientOnly);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum number of attempts (including the first), NOT the number of
    /// retries: `max_attempts: 3` means the node runs at most 3 times total.
    /// `0` is invalid (see [`Aegis::validate`]) -- never unlimited, never a
    /// single silent attempt.
    pub max_attempts: u32,
    /// The delay before attempt 2, before any `backoff_factor` scaling.
    pub initial_interval: Duration,
    /// The multiplier applied per additional attempt: the delay before
    /// attempt `n` (n >= 2) is
    /// `min(initial_interval * backoff_factor^(n - 2), max_interval)`.
    pub backoff_factor: f64,
    /// The ceiling every computed delay is capped at.
    pub max_interval: Duration,
    /// Whether to add random jitter (uniformly in `[0, delay)`, added to
    /// the computed delay) to avoid a thundering-herd retry storm.
    pub jitter: bool,
    /// Which classified errors this policy retries.
    pub retry_on: RetryPredicate,
}

impl Default for RetryPolicy {
    /// `max_attempts: 3`, `initial_interval: 500ms`, `backoff_factor: 2.0`,
    /// `max_interval: 60s`, `jitter: true`, `retry_on: TransientOnly` (D-09).
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_interval: Duration::from_millis(500),
            backoff_factor: 2.0,
            max_interval: Duration::from_secs(60),
            jitter: true,
            retry_on: RetryPredicate::TransientOnly,
        }
    }
}

/// Which classified errors a [`RetryPolicy`] retries, matched against a
/// `NodeError.transience` (`paladin_battalion::engine::retry::should_retry`,
/// plan 25-01/25-03).
///
/// `#[non_exhaustive]`: the engine matches this exhaustively, and a future
/// variant (e.g. a registered custom predicate kind) must not silently
/// change behavior at every existing match site.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RetryPredicate {
    /// Retry only `Transience::Transient` errors (the default).
    TransientOnly,
    /// Retry `Transience::Transient` and `Transience::Unknown` errors.
    TransientAndUnknown,
    /// Retry according to a registered predicate evaluator named here
    /// (plan 25-03's `RetryPredicateRegistry`). Never resolved by this
    /// plan's `should_retry` -- always evaluates to "do not retry" until
    /// then.
    Custom(String),
}

/// Per-ATTEMPT run/idle timeout policy (Doc 04 FT-FR-08, FT-FR-09; D-18,
/// D-19, D-20; plan 25-09), nested inside the run-level
/// `EngineLimits::run_timeout`.
///
/// Both bounds are armed fresh for every attempt of the node and fire as a
/// `NodeError { source: Timeout(kind), transience: Transient }` -- so under
/// a `RetryPolicy` a timed-out attempt is retried like any other transient
/// failure, and the timed-out attempt's partial work is discarded exactly
/// as any other failed attempt's is. Which bound fired is carried by the
/// typed `TimeoutKind`, never inferred from a message.
///
/// `TimeoutPolicy { run_timeout: None, idle_timeout: None }` arms nothing
/// and is indistinguishable from having no policy; `Some(Duration::ZERO)`
/// on either field is rejected by `WarGraph::validate` before execution.
///
/// # `idle_timeout` on a node that never reports progress
///
/// The idle bound resets on each observed progress event -- a stream chunk,
/// an Armament invocation or a completed LLM call reported through
/// `PaladinPort::execute_observed`, or an explicit `ctx.heartbeat()` from a
/// Function node. A node whose port never beats (the trait's DEFAULT
/// `execute_observed` delegates to `execute` and correctly claims no
/// progress) therefore has its `idle_timeout` degrade to a per-attempt wall
/// clock: it fires `idle_timeout` after the attempt starts whether or not
/// work is happening -- a bound, never no bound at all.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::aegis::{Aegis, TimeoutPolicy};
/// use std::time::Duration;
///
/// // A stream emitting a chunk every 100 ms is healthy under this policy;
/// // one that stalls 300 ms is not, and a 10 s wall clock caps each attempt.
/// let aegis = Aegis {
///     timeout: Some(TimeoutPolicy {
///         run_timeout: Some(Duration::from_secs(10)),
///         idle_timeout: Some(Duration::from_millis(250)),
///     }),
///     ..Default::default()
/// };
/// assert!(aegis.validate().is_ok());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TimeoutPolicy {
    /// A hard wall-clock cap on EACH attempt, if set: firing cancels the
    /// attempt (discarding its partial work) with `Timeout(Run)`, and
    /// progress cannot extend it. The effective per-attempt deadline is
    /// `min(run_timeout, remaining EngineLimits::run_timeout budget)`, and
    /// an attempt the ENGINE budget cuts records `Timeout(EngineRun)`
    /// instead.
    pub run_timeout: Option<Duration>,
    /// The longest an attempt may go WITHOUT an observed progress event
    /// before failing `Timeout(Idle)`, if set; each progress event restarts
    /// the window. See the type-level note on ports that never beat.
    pub idle_timeout: Option<Duration>,
}

/// A typed error handler (Doc 04 FT-FR-04, plans 25-10/11).
///
/// `#[non_exhaustive]`: the engine matches this exhaustively.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ErrorHandlerSpec {
    /// Route the error to another node instead of failing the run.
    Route {
        /// The node to route to.
        to: NodeId,
        /// The field the error is written into for `to` to read.
        error_field: FieldName,
    },
    /// Absorb the error by merging a fallback delta instead of failing the
    /// run.
    Absorb {
        /// The delta merged in place of the failed node's own delta.
        fallback_delta: StateDelta,
    },
    /// Handle according to a registered handler named here (plan 25-03's
    /// `ErrorHandlerRegistry`).
    Custom(String),
}

/// Cache policy for a node's result (Doc 04 FT-FR-06, plans 25-04/13).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CachePolicy {
    /// How long a cached result stays valid.
    pub ttl: Duration,
    /// How the cache key is composed.
    pub key: CacheKeySpec,
}

/// How a [`CachePolicy`]'s cache key is composed.
///
/// `#[non_exhaustive]`: the engine matches this exhaustively.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum CacheKeySpec {
    /// The engine's own default key composition (plan 25-13's
    /// graph-fingerprint-inclusive default).
    Default,
    /// Key on only these declared fields' values.
    Fields(Vec<FieldName>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_retry_policy_matches_d09() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.initial_interval, Duration::from_millis(500));
        assert_eq!(policy.backoff_factor, 2.0);
        assert_eq!(policy.max_interval, Duration::from_secs(60));
        assert!(policy.jitter);
        assert_eq!(policy.retry_on, RetryPredicate::TransientOnly);
    }

    #[test]
    fn max_attempts_zero_is_a_typed_validation_error() {
        let aegis = Aegis {
            retry: Some(RetryPolicy {
                max_attempts: 0,
                ..RetryPolicy::default()
            }),
            ..Default::default()
        };
        assert_eq!(aegis.validate(), Err(AegisError::RetryMaxAttemptsZero));
    }

    #[test]
    fn max_attempts_one_or_more_validates() {
        let aegis = Aegis {
            retry: Some(RetryPolicy {
                max_attempts: 1,
                ..RetryPolicy::default()
            }),
            ..Default::default()
        };
        assert!(aegis.validate().is_ok());
    }

    #[test]
    fn no_retry_policy_validates() {
        assert!(Aegis::default().validate().is_ok());
    }

    #[test]
    fn aegis_round_trips_through_serde_json() {
        let aegis = Aegis {
            retry: Some(RetryPolicy::default()),
            timeout: Some(TimeoutPolicy {
                run_timeout: Some(Duration::from_secs(30)),
                idle_timeout: None,
            }),
            on_error: Some(ErrorHandlerSpec::Custom("my-handler".to_string())),
            cache: Some(CachePolicy {
                ttl: Duration::from_secs(60),
                key: CacheKeySpec::Default,
            }),
        };
        let json = serde_json::to_string(&aegis).expect("serialize");
        let back: Aegis = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(aegis, back);
    }
}
