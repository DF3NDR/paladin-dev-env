//! The one per-attempt runner for the legacy Battalion patterns (Formation,
//! Phalanx, Campaign and the Conclave).
//!
//! Every Paladin attempt those patterns make goes through [`run_with_aegis`],
//! driven by the Battalion's [`Aegis`] policy (`BattalionConfig.aegis`):
//!
//! - [`attempt_once`] owns the **only** `tokio::time::timeout` the legacy
//!   patterns use. The bound is *per attempt* (D-02): there is no whole-run
//!   wall clock, so a ten-step Formation may run ten times `run_timeout`.
//! - Retry gating goes through [`should_retry`] and the wait between attempts
//!   through [`backoff_delay`] / [`wait_backoff`] from `engine::retry` -- one
//!   backoff implementation (D-12) and one retry predicate (Phase 26 D-11).
//! - A failure is classified by `PaladinError::transience()` and recorded with
//!   [`to_node_error_source`], never by reading message text.
//!
//! `PaladinPort::execute` reports no progress, so an `idle_timeout` degrades to
//! a per-attempt wall clock here (the `TimeoutPolicy` rustdoc rule); the tighter
//! of the two bounds applies, see [`attempt_bound`].
//!
//! The module is crate-private: it adds no public surface.

use std::sync::Arc;
use std::time::Duration;

use log::warn;
use tokio_util::sync::CancellationToken;

use paladin_core::platform::container::aegis::{Aegis, TimeoutPolicy};
use paladin_core::platform::container::battalion::BattalionError;
use paladin_core::platform::container::node_error::{NodeError, NodeErrorSource, TimeoutKind};
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::transience::Transience;
use paladin_core::platform::container::waypoint::NodeId;
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult};

use crate::engine::retry::{backoff_delay, should_retry, wait_backoff};
use crate::llm_failure::to_node_error_source;

/// The wall-clock bound applied to one Paladin attempt and which policy field
/// it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AttemptBound {
    /// How long one attempt may run.
    pub(crate) limit: Duration,
    /// Which `TimeoutPolicy` field supplied the bound.
    pub(crate) kind: TimeoutKind,
}

/// Resolve the per-attempt bound from a [`TimeoutPolicy`].
///
/// `None`, or a policy with neither field set, arms nothing. With both set the
/// tighter wins (`idle_timeout` degrades to a per-attempt wall clock because
/// `PaladinPort::execute` reports no progress); a tie is reported as
/// [`TimeoutKind::Run`].
pub(crate) fn attempt_bound(timeout: Option<&TimeoutPolicy>) -> Option<AttemptBound> {
    let policy = timeout?;
    match (policy.run_timeout, policy.idle_timeout) {
        (None, None) => None,
        (Some(run), None) => Some(AttemptBound {
            limit: run,
            kind: TimeoutKind::Run,
        }),
        (None, Some(idle)) => Some(AttemptBound {
            limit: idle,
            kind: TimeoutKind::Idle,
        }),
        (Some(run), Some(idle)) if idle < run => Some(AttemptBound {
            limit: idle,
            kind: TimeoutKind::Idle,
        }),
        (Some(run), Some(_)) => Some(AttemptBound {
            limit: run,
            kind: TimeoutKind::Run,
        }),
    }
}

/// A successful Paladin execution and how many attempts it took.
#[derive(Debug)]
pub(crate) struct AttemptOutcome {
    /// The Paladin's result.
    pub(crate) result: PaladinResult,
    /// Total attempts made, including the first (always at least 1).
    pub(crate) attempts: u32,
}

/// A failed Paladin attempt: the original error plus its structured record.
#[derive(Debug)]
pub(crate) struct AttemptFailure {
    /// The original error, kept so a non-timeout failure keeps its v0.10
    /// fail-fast text.
    pub(crate) error: PaladinError,
    /// The structured record (`node_id` is the Paladin's name).
    pub(crate) node_error: NodeError,
}

impl AttemptFailure {
    /// Record a `PaladinPort::execute` failure.
    pub(crate) fn from_error(paladin_name: &str, attempt: u32, error: PaladinError) -> Self {
        let node_error = NodeError {
            node_id: NodeId::new(paladin_name),
            attempt,
            transience: error.transience(),
            source: to_node_error_source(&error),
        };
        Self { error, node_error }
    }

    /// Record an attempt that exceeded its bound.
    ///
    /// The legacy `PaladinError::Timeout` payload is whole seconds with a
    /// floor of 1, so a 250 ms bound reports 1, never 0; the typed
    /// `NodeErrorSource::Timeout` is the authoritative record.
    pub(crate) fn timed_out(paladin_name: &str, attempt: u32, bound: AttemptBound) -> Self {
        Self {
            error: PaladinError::Timeout(bound.limit.as_secs().max(1)),
            node_error: NodeError {
                node_id: NodeId::new(paladin_name),
                attempt,
                transience: Transience::Transient,
                source: NodeErrorSource::Timeout(bound.kind),
            },
        }
    }

    /// Whether this attempt failed by exceeding its bound.
    pub(crate) fn is_timeout(&self) -> bool {
        matches!(self.node_error.source, NodeErrorSource::Timeout(_))
    }

    /// The error a fail-fast pattern returns: a timeout becomes the structured
    /// `BattalionError::Node`; anything else keeps the unchanged v0.10
    /// `BattalionError::PaladinError(<message>)` contract.
    pub(crate) fn into_fail_fast_error(self) -> BattalionError {
        if self.is_timeout() {
            BattalionError::Node(self.node_error)
        } else {
            BattalionError::PaladinError(self.error.to_string())
        }
    }
}

/// Run one attempt, racing it against `bound` when one is set.
///
/// This is the only `tokio::time::timeout` call in the legacy patterns.
pub(crate) async fn attempt_once(
    port: &Arc<dyn PaladinPort>,
    paladin: &Paladin,
    input: &str,
    bound: Option<AttemptBound>,
    attempt: u32,
) -> Result<PaladinResult, AttemptFailure> {
    let name = paladin.node.name.as_str();
    match bound {
        Some(bound) => {
            match tokio::time::timeout(bound.limit, port.execute(paladin, input)).await {
                Ok(Ok(result)) => Ok(result),
                Ok(Err(error)) => Err(AttemptFailure::from_error(name, attempt, error)),
                Err(_elapsed) => Err(AttemptFailure::timed_out(name, attempt, bound)),
            }
        }
        None => port
            .execute(paladin, input)
            .await
            .map_err(|error| AttemptFailure::from_error(name, attempt, error)),
    }
}

/// Run a Paladin under an [`Aegis`]: per-attempt timeout plus retry.
///
/// `aegis.retry.max_attempts` counts total attempts including the first. A
/// failed attempt is retried only when a policy exists, attempts remain, the
/// policy's predicate admits the failure's transience, and the backoff wait
/// was not cancelled. Otherwise the last failure is returned.
pub(crate) async fn run_with_aegis(
    port: &Arc<dyn PaladinPort>,
    paladin: &Paladin,
    input: &str,
    aegis: &Aegis,
    cancel: &Option<CancellationToken>,
) -> Result<AttemptOutcome, AttemptFailure> {
    // `Aegis::validate` guarantees max >= 1; a 0 reaching here still makes one
    // attempt rather than panicking.
    let max = aegis.retry.as_ref().map_or(1, |p| p.max_attempts).max(1);
    let bound = attempt_bound(aegis.timeout.as_ref());

    let mut attempt: u32 = 1;
    loop {
        let failure = match attempt_once(port, paladin, input, bound, attempt).await {
            Ok(result) => {
                return Ok(AttemptOutcome {
                    result,
                    attempts: attempt,
                });
            }
            Err(failure) => failure,
        };

        let Some(policy) = aegis.retry.as_ref() else {
            return Err(failure);
        };
        if attempt >= max || !should_retry(policy, &failure.node_error, attempt) {
            return Err(failure);
        }

        let delay = backoff_delay(policy, attempt + 1);
        // Name, attempt, transience and delay only -- never the input, the
        // output or the error's message body (T-44-03).
        warn!(
            "Paladin {} attempt {} failed ({:?}); retrying in {:?}",
            paladin.node.name, attempt, failure.node_error.transience, delay
        );
        if !wait_backoff(delay, cancel).await {
            return Err(failure);
        }
        attempt += 1;
    }
}
