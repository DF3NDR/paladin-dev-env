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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    use async_trait::async_trait;
    use paladin_core::platform::container::aegis::{RetryPolicy, RetryPredicate};
    use paladin_core::platform::container::battalion::TokenUsage;
    use paladin_core::platform::container::paladin::PaladinData;
    use paladin_ports::output::paladin_port::{PaladinStream, StopReason};

    /// One scripted call: wait `delay`, then return `result`.
    struct Step {
        delay: Duration,
        result: Result<(), PaladinError>,
    }

    fn ok_after(delay: Duration) -> Step {
        Step {
            delay,
            result: Ok(()),
        }
    }

    fn fail_now(error: PaladinError) -> Step {
        Step {
            delay: Duration::ZERO,
            result: Err(error),
        }
    }

    /// A port that plays back a script, recording the call count and every
    /// input it received. An exhausted script answers with an immediate success.
    struct ScriptedPort {
        script: Mutex<VecDeque<Step>>,
        inputs: Mutex<Vec<String>>,
    }

    impl ScriptedPort {
        fn new(steps: Vec<Step>) -> Arc<Self> {
            Arc::new(Self {
                script: Mutex::new(steps.into()),
                inputs: Mutex::new(Vec::new()),
            })
        }

        fn calls(&self) -> usize {
            self.inputs.lock().map(|i| i.len()).unwrap_or_default()
        }

        fn inputs(&self) -> Vec<String> {
            self.inputs.lock().map(|i| i.clone()).unwrap_or_default()
        }
    }

    #[async_trait]
    impl PaladinPort for ScriptedPort {
        async fn execute(
            &self,
            _paladin: &Paladin,
            input: &str,
        ) -> Result<PaladinResult, PaladinError> {
            let step = {
                if let Ok(mut inputs) = self.inputs.lock() {
                    inputs.push(input.to_string());
                }
                self.script.lock().ok().and_then(|mut s| s.pop_front())
            };
            let step = step.unwrap_or_else(|| ok_after(Duration::ZERO));
            tokio::time::sleep(step.delay).await;
            step.result.map(|()| PaladinResult {
                output: "done".to_string(),
                usage: TokenUsage::new(0, 0),
                execution_time_ms: 0,
                loop_count: 1,
                stop_reason: StopReason::Completed,
                ..Default::default()
            })
        }

        async fn execute_stream(
            &self,
            _paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinStream, PaladinError> {
            Err(PaladinError::ExecutionError("not used".to_string()))
        }

        fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
            Ok(())
        }
    }

    fn paladin() -> Paladin {
        let data = PaladinData {
            name: "scout".to_string(),
            ..Default::default()
        };
        Paladin::new(data, Some("scout".to_string()))
    }

    fn transient() -> PaladinError {
        PaladinError::LlmFailure {
            transience: Transience::Transient,
            status: Some(503),
            provider: None,
            message: "unavailable".to_string(),
        }
    }

    fn retrying(max_attempts: u32, retry_on: RetryPredicate) -> Aegis {
        Aegis {
            retry: Some(RetryPolicy {
                max_attempts,
                jitter: false,
                retry_on,
                ..RetryPolicy::default()
            }),
            ..Aegis::default()
        }
    }

    fn bounded(run: Option<Duration>, idle: Option<Duration>) -> TimeoutPolicy {
        TimeoutPolicy {
            run_timeout: run,
            idle_timeout: idle,
        }
    }

    async fn run(
        port: &Arc<ScriptedPort>,
        aegis: &Aegis,
        cancel: &Option<CancellationToken>,
    ) -> Result<AttemptOutcome, AttemptFailure> {
        let dyn_port: Arc<dyn PaladinPort> = port.clone();
        run_with_aegis(&dyn_port, &paladin(), "in", aegis, cancel).await
    }

    #[test]
    fn attempt_bound_picks_the_tighter_bound() {
        assert_eq!(attempt_bound(None), None);
        assert_eq!(attempt_bound(Some(&bounded(None, None))), None);

        let run_only = attempt_bound(Some(&bounded(Some(Duration::from_secs(10)), None)));
        assert_eq!(
            run_only,
            Some(AttemptBound {
                limit: Duration::from_secs(10),
                kind: TimeoutKind::Run
            })
        );

        let idle_only = attempt_bound(Some(&bounded(None, Some(Duration::from_secs(3)))));
        assert_eq!(
            idle_only,
            Some(AttemptBound {
                limit: Duration::from_secs(3),
                kind: TimeoutKind::Idle
            })
        );

        let both = attempt_bound(Some(&bounded(
            Some(Duration::from_secs(10)),
            Some(Duration::from_secs(2)),
        )));
        assert_eq!(
            both,
            Some(AttemptBound {
                limit: Duration::from_secs(2),
                kind: TimeoutKind::Idle
            })
        );
    }

    #[test]
    fn attempt_bound_tie_is_run() {
        let tie = attempt_bound(Some(&bounded(
            Some(Duration::from_secs(5)),
            Some(Duration::from_secs(5)),
        )));
        assert_eq!(
            tie,
            Some(AttemptBound {
                limit: Duration::from_secs(5),
                kind: TimeoutKind::Run
            })
        );
    }

    #[tokio::test(start_paused = true)]
    async fn transient_failure_is_retried_with_engine_backoff() {
        let port = ScriptedPort::new(vec![fail_now(transient()), ok_after(Duration::ZERO)]);
        let aegis = retrying(3, RetryPredicate::TransientOnly);
        let expected_delay = match &aegis.retry {
            Some(policy) => backoff_delay(policy, 2),
            None => Duration::ZERO,
        };
        assert_eq!(expected_delay, Duration::from_millis(500));

        let started = tokio::time::Instant::now();
        let outcome = run(&port, &aegis, &None)
            .await
            .expect("second attempt succeeds");
        assert_eq!(outcome.attempts, 2);
        assert_eq!(port.calls(), 2);
        assert_eq!(started.elapsed(), expected_delay);
        // The same input is re-sent on every attempt.
        assert_eq!(port.inputs(), vec!["in".to_string(), "in".to_string()]);
    }

    #[tokio::test(start_paused = true)]
    async fn unknown_failure_not_retried_under_transient_only() {
        let port = ScriptedPort::new(vec![
            fail_now(PaladinError::ExecutionError("boom".to_string())),
            ok_after(Duration::ZERO),
        ]);
        let aegis = retrying(3, RetryPredicate::TransientOnly);
        let failure = run(&port, &aegis, &None)
            .await
            .expect_err("Unknown is not retried under TransientOnly");
        assert_eq!(port.calls(), 1);
        assert_eq!(failure.node_error.attempt, 1);
        assert_eq!(failure.node_error.transience, Transience::Unknown);
    }

    #[tokio::test(start_paused = true)]
    async fn unknown_failure_retried_under_transient_and_unknown() {
        let port = ScriptedPort::new(vec![
            fail_now(PaladinError::ExecutionError("a".to_string())),
            fail_now(PaladinError::ExecutionError("b".to_string())),
            fail_now(PaladinError::ExecutionError("c".to_string())),
        ]);
        let aegis = retrying(3, RetryPredicate::TransientAndUnknown);
        let failure = run(&port, &aegis, &None)
            .await
            .expect_err("every attempt fails");
        assert_eq!(port.calls(), 3);
        assert_eq!(failure.node_error.attempt, 3);
    }

    #[tokio::test(start_paused = true)]
    async fn permanent_failure_is_never_retried() {
        let port = ScriptedPort::new(vec![
            fail_now(PaladinError::ConfigurationError("bad".to_string())),
            ok_after(Duration::ZERO),
        ]);
        let aegis = retrying(3, RetryPredicate::TransientAndUnknown);
        let failure = run(&port, &aegis, &None)
            .await
            .expect_err("Permanent is never retried");
        assert_eq!(port.calls(), 1);
        assert_eq!(failure.node_error.transience, Transience::Permanent);
    }

    #[tokio::test(start_paused = true)]
    async fn max_attempts_counts_total_attempts() {
        let port = ScriptedPort::new(vec![
            fail_now(transient()),
            fail_now(transient()),
            fail_now(transient()),
            fail_now(transient()),
        ]);
        let aegis = retrying(3, RetryPredicate::TransientOnly);
        let failure = run(&port, &aegis, &None)
            .await
            .expect_err("transient forever");
        assert_eq!(port.calls(), 3, "max_attempts includes the first attempt");
        assert_eq!(failure.node_error.attempt, 3);
        assert_eq!(failure.node_error.transience, Transience::Transient);
    }

    #[tokio::test(start_paused = true)]
    async fn subsecond_timeout_reports_one_second() {
        let port = ScriptedPort::new(vec![ok_after(Duration::from_secs(1))]);
        let aegis = Aegis {
            timeout: Some(bounded(Some(Duration::from_millis(250)), None)),
            ..Aegis::default()
        };
        let started = tokio::time::Instant::now();
        let failure = run(&port, &aegis, &None).await.expect_err("250 ms bound");
        assert_eq!(started.elapsed(), Duration::from_millis(250));
        assert!(failure.is_timeout());
        assert!(matches!(failure.error, PaladinError::Timeout(1)));
        assert_eq!(
            failure.node_error.source,
            NodeErrorSource::Timeout(TimeoutKind::Run)
        );
        assert_eq!(failure.node_error.transience, Transience::Transient);
        assert!(matches!(
            failure.into_fail_fast_error(),
            BattalionError::Node(_)
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn cancellation_during_backoff_stops_retrying() {
        let port = ScriptedPort::new(vec![fail_now(transient()), ok_after(Duration::ZERO)]);
        let mut aegis = retrying(3, RetryPredicate::TransientOnly);
        if let Some(policy) = aegis.retry.as_mut() {
            policy.initial_interval = Duration::from_secs(10);
        }
        let token = CancellationToken::new();
        let cancel = Some(token.clone());

        let started = tokio::time::Instant::now();
        let (outcome, ()) = tokio::join!(run(&port, &aegis, &cancel), async {
            tokio::time::sleep(Duration::from_secs(1)).await;
            token.cancel();
        });
        let failure = outcome.expect_err("cancelled while backing off");
        assert_eq!(port.calls(), 1, "no second attempt after cancellation");
        assert_eq!(failure.node_error.attempt, 1);
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[tokio::test(start_paused = true)]
    async fn timed_out_attempt_is_retried_like_any_transient_failure() {
        // Attempt 1 outlives the 1 s bound; attempt 2 is fast.
        let port = ScriptedPort::new(vec![
            ok_after(Duration::from_secs(5)),
            ok_after(Duration::ZERO),
        ]);
        let mut aegis = retrying(2, RetryPredicate::TransientOnly);
        aegis.timeout = Some(bounded(Some(Duration::from_secs(1)), None));

        let started = tokio::time::Instant::now();
        let outcome = run(&port, &aegis, &None)
            .await
            .expect("the second, fast attempt succeeds");
        assert_eq!(outcome.attempts, 2);
        assert_eq!(port.calls(), 2);
        // 1 s bound on attempt 1, then the 500 ms initial backoff.
        assert_eq!(started.elapsed(), Duration::from_millis(1500));
    }
}
