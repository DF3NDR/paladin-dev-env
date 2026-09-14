//! Retry and fallback as **port-shaping** middleware (Doc 05 D-11, RT-02,
//! RT-FR-09).
//!
//! [`ModelRetryMiddleware`] and [`ModelFallbackMiddleware`] never retry or
//! hop a call themselves -- they set what the run's ONE model-call site
//! (`PaladinExecutionService::execute_with_retry_and_temperature`) should
//! use for this call, and that call site honors it. Every calculation is
//! delegated to Phase 25's existing implementations, never re-derived here:
//!
//! - Fallback hopping, the streaming first-chunk rule and the `served_by`
//!   provenance stamp all come from [`paladin_llm::fallback::FallbackLlmAdapter`]
//!   -- [`ModelFallbackMiddleware`] builds exactly ONE of these at
//!   construction and never re-implements chain-hopping logic.
//! - The backoff delay for attempt `n` comes from
//!   [`paladin_battalion::engine::retry::backoff_delay`], and the retry
//!   predicate comes from
//!   [`paladin_core::platform::container::aegis::RetryPredicate::admits`]
//!   (Task 1's refactor target) -- [`ModelRetryMiddleware`] itself performs
//!   no retrying; it only sets the policy the call site reads.
//!
//! # The assumption-delta promote: one port-resolution point
//!
//! Before this module, a run had exactly one port: the service's own. After
//! it, a run's port may come from the service default, a fallback chain, or
//! (Phase 27) a per-run scope. Rather than adding an
//! if-override-else-service branch at the call site, the resolution is
//! promoted to
//! [`super::ModelCallContext::effective_llm`] -- a single accessor the call
//! site invokes exactly once, with the service's own port SEEDED as the
//! default rather than sitting on the far side of an `else`. See that
//! method's rustdoc for the full decision record and the invariant test
//! name (`model_call_port_is_resolved_at_exactly_one_point`).
//!
//! # Why not an `around_model` hook?
//!
//! An `around_model` hook -- wrapping the model call itself, the way
//! `around_tool` wraps a tool dispatch -- was considered and rejected. It
//! cannot express fallback: fallback substitutes a DIFFERENT port for the
//! call, it does not repeat the same call. Adding one would also change
//! PRD 05's three-hook trait (`before_model`/`after_model`/`around_tool`),
//! which is a closed surface (D-01). Port-shaping through
//! `before_model` plus a single call-site resolution point covers both
//! retry and fallback without touching the trait at all.

use std::sync::Arc;

use async_trait::async_trait;

use paladin_core::platform::container::aegis::RetryPolicy;
use paladin_llm::fallback::{FallbackChainError, FallbackLlmAdapter};
use paladin_ports::output::llm_port::LlmPort;

use crate::application::services::paladin::error::PaladinError;

use super::{ExecutionMiddleware, MiddlewareFlow, ModelCallContext};

/// Sets a fallback chain as this run's effective port (D-11, RT-FR-09).
///
/// Builds exactly ONE [`FallbackLlmAdapter`] at construction -- reusing its
/// own empty-chain validation rather than re-checking here -- and installs
/// it as [`super::ModelCallContext::llm_override`] in `before_model`. Every
/// hop, the streaming first-chunk rule, and the `served_by` metadata stamp
/// are the adapter's; this middleware re-implements none of it.
///
/// Stateless (D-03): the SAME `Arc<dyn ExecutionMiddleware>` can back many
/// concurrent runs -- each run's `ModelCallContext` gets its own
/// `llm_override`, so no run can leak its override to another
/// (`concurrent_runs_do_not_share_an_override`, T-26-34).
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use paladin::application::services::paladin::middleware::ModelFallbackMiddleware;
/// use paladin_llm::mock::MockLlmAdapter;
/// use paladin_ports::output::llm_port::LlmPort;
///
/// let primary: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new().with_provider_name("openai"));
/// let backup: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new().with_provider_name("anthropic"));
/// let middleware = ModelFallbackMiddleware::new(vec![primary, backup])?;
/// # Ok::<(), paladin_llm::fallback::FallbackChainError>(())
/// ```
pub struct ModelFallbackMiddleware {
    adapter: Arc<dyn LlmPort>,
}

impl ModelFallbackMiddleware {
    /// Build a fallback chain from `chain[0]` (tried first) to `chain[n-1]`
    /// (tried last).
    ///
    /// # Errors
    ///
    /// [`FallbackChainError::EmptyChain`] if `chain` is empty -- rejected
    /// here at construction (via [`FallbackLlmAdapter::new`]'s own check),
    /// never deferred to the first model call
    /// (`fallback_chain_construction_validates_up_front`).
    pub fn new(chain: Vec<Arc<dyn LlmPort>>) -> Result<Self, FallbackChainError> {
        let adapter = FallbackLlmAdapter::new(chain)?;
        Ok(Self {
            adapter: Arc::new(adapter),
        })
    }
}

#[async_trait]
impl ExecutionMiddleware for ModelFallbackMiddleware {
    async fn before_model(
        &self,
        cx: &mut ModelCallContext<'_>,
    ) -> Result<MiddlewareFlow, PaladinError> {
        cx.llm_override = Some(Arc::clone(&self.adapter));
        // WR-01 (28-REVIEW): this only INSTALLS the fallback chain as this
        // call's effective port -- it does not mean a hop actually
        // happened. Every call through a Paladin configured with this
        // middleware would otherwise emit a misleading
        // `MiddlewareAction::Fallback` even on a healthy, first-attempt,
        // no-error call. `FallbackLlmAdapter::record_hop`
        // (`crates/paladin-llm/src/fallback.rs`) already emits
        // `TraceEvent::FallbackHop` -- a distinct event that fires ONLY on
        // a real hop -- so that is this run's source of truth for fallback
        // activity; no `MiddlewareAction::Fallback` hint is set here.
        Ok(MiddlewareFlow::Continue)
    }

    fn name(&self) -> &str {
        "model_fallback"
    }
}

/// Sets a [`RetryPolicy`] as this run's retry policy (D-11, RT-FR-09).
///
/// Performs no retrying itself: it only sets
/// [`super::ModelCallContext::retry_policy`] in `before_model`. The
/// service's own call site
/// (`PaladinExecutionService::execute_with_retry_and_temperature`) reads it
/// and drives attempts, delays (via
/// `paladin_battalion::engine::retry::backoff_delay`) and the retry
/// predicate (via
/// `paladin_core::platform::container::aegis::RetryPredicate::admits`)
/// from the policy.
///
/// Stateless (D-03): the SAME `Arc<dyn ExecutionMiddleware>` can back many
/// concurrent runs -- each sets its own context's `retry_policy`
/// independently.
///
/// # Examples
///
/// ```
/// use paladin::application::services::paladin::middleware::ModelRetryMiddleware;
/// use paladin_core::platform::container::aegis::RetryPolicy;
///
/// let middleware = ModelRetryMiddleware::new(RetryPolicy::default());
/// ```
pub struct ModelRetryMiddleware {
    policy: RetryPolicy,
}

impl ModelRetryMiddleware {
    /// Construct a middleware that installs `policy` as every run's retry
    /// policy.
    pub fn new(policy: RetryPolicy) -> Self {
        Self { policy }
    }
}

#[async_trait]
impl ExecutionMiddleware for ModelRetryMiddleware {
    async fn before_model(
        &self,
        cx: &mut ModelCallContext<'_>,
    ) -> Result<MiddlewareFlow, PaladinError> {
        cx.retry_policy = Some(self.policy.clone());
        // WR-01 (28-REVIEW): this only INSTALLS the retry policy -- it does
        // not mean a retry actually fired. Setting the hint here
        // unconditionally made every successful, first-attempt call emit a
        // misleading `MiddlewareAction::Retry`. The real signal now comes
        // from `PaladinExecutionService::execute_with_retry_and_temperature`'s
        // own retry arm, which emits `MiddlewareAction::Retry` (named
        // `"model_retry"`, this middleware's own `name()`) at the exact
        // moment a retry is about to happen, gated on `retry_policy` being
        // `Some` (i.e. this middleware actually being installed).
        Ok(MiddlewareFlow::Continue)
    }

    fn name(&self) -> &str {
        "model_retry"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::services::paladin::middleware::PromptAssembly;
    use crate::application::services::paladin::paladin_execution_service::PaladinExecutionService;
    use crate::core::base::entity::node::Node;
    use crate::core::platform::container::paladin::{MaxLoops, Paladin, PaladinData};
    use crate::infrastructure::resilience::circuit_breaker::CircuitBreaker;
    use paladin_core::platform::container::aegis::RetryPredicate;
    use paladin_llm::mock::{MockLlmAdapter, MockScriptEntry};
    use paladin_ports::output::llm_port::LlmError;
    use paladin_ports::output::trace_sink_port::MiddlewareAction;
    use std::time::Duration;

    fn make_paladin(max_loops: u32) -> Paladin {
        let data = PaladinData {
            system_prompt: "system".to_string(),
            max_loops: MaxLoops::Fixed(max_loops),
            ..Default::default()
        };
        Node::new(data, None)
    }

    fn make_service(llm: Arc<dyn LlmPort>) -> PaladinExecutionService {
        PaladinExecutionService::new(
            llm,
            Arc::new(CircuitBreaker::new(50, 25, Duration::from_secs(60))),
            None,
            None,
        )
    }

    fn transient(provider: &str) -> LlmError {
        LlmError::ProviderError {
            provider: provider.to_string(),
            status: 503,
            message: "upstream unavailable".to_string(),
        }
    }

    /// Test 1 (assumption-delta invariant): with nothing set on a fresh
    /// context, `effective_llm` returns the service default -- the seeded
    /// default source, not a branch. Tests 2/3/6/7 below exercise the two
    /// real override sources (fallback, retry policy) end-to-end through
    /// the ONE real call site
    /// (`execute_with_retry_and_temperature`); the structural half of this
    /// invariant ("there is exactly one accessor") is a grep-based
    /// acceptance criterion on the accessor's own defining line, in
    /// `context.rs`.
    #[test]
    fn model_call_port_is_resolved_at_exactly_one_point() {
        let paladin = make_paladin(1);
        let assembly = PromptAssembly::new("system", "input", "", vec![], None);
        let mut cx = ModelCallContext::new(uuid::Uuid::new_v4(), &paladin, assembly);
        let service_default: Arc<dyn LlmPort> =
            Arc::new(MockLlmAdapter::new().with_provider_name("service-default"));

        assert_eq!(
            cx.effective_llm(&service_default).get_provider_name(),
            "service-default",
            "with no override, the seeded service default is the effective port"
        );

        let override_port: Arc<dyn LlmPort> =
            Arc::new(MockLlmAdapter::new().with_provider_name("override"));
        cx.llm_override = Some(Arc::clone(&override_port));
        assert_eq!(
            cx.effective_llm(&service_default).get_provider_name(),
            "override",
            "once set, the override -- not the default -- is the effective port"
        );
    }

    /// Test 2: a chain of two ports where the first always fails
    /// transiently -- the run succeeds through the second, and
    /// `served_by` names it, entirely through `FallbackLlmAdapter`'s own
    /// metadata stamp.
    #[tokio::test]
    async fn fallback_middleware_routes_through_the_fallback_adapter() {
        let primary: Arc<dyn LlmPort> = Arc::new(
            MockLlmAdapter::new()
                .with_provider_name("primary")
                .with_error(transient("primary")),
        );
        let backup: Arc<dyn LlmPort> = Arc::new(
            MockLlmAdapter::new()
                .with_provider_name("backup")
                .with_response("served by backup"),
        );
        let middleware = Arc::new(ModelFallbackMiddleware::new(vec![primary, backup]).unwrap());
        let service_default: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new());
        let service = make_service(service_default).with_middleware(middleware);
        let paladin = make_paladin(1);

        let result = service.execute(&paladin, "hi").await.unwrap();

        assert_eq!(result.served_by.as_deref(), Some("backup"));
        assert!(result.output.contains("served by backup"));
    }

    /// Test 3: `RetryPolicy { max_attempts: 4, jitter: false, .. }` against
    /// a mock failing transiently 3 times then succeeding makes exactly 4
    /// attempts, with the policy's own delay sequence (500/1000/2000ms) --
    /// under a paused clock (the Phase 25 idiom).
    #[tokio::test(start_paused = true)]
    async fn retry_middleware_uses_the_policy_attempts_and_delays() {
        let llm = Arc::new(MockLlmAdapter::new().with_script(vec![
            MockScriptEntry::Error(transient("p")),
            MockScriptEntry::Error(transient("p")),
            MockScriptEntry::Error(transient("p")),
            MockScriptEntry::Text("finally!".to_string()),
        ]));
        let policy = RetryPolicy {
            max_attempts: 4,
            jitter: false,
            ..RetryPolicy::default()
        };
        let middleware = Arc::new(ModelRetryMiddleware::new(policy.clone()));
        let service = make_service(llm.clone()).with_middleware(middleware);
        let paladin = make_paladin(1);

        let start = tokio::time::Instant::now();
        let result = service.execute(&paladin, "hi").await.unwrap();
        let elapsed = start.elapsed();

        assert_eq!(llm.call_count(), 4, "3 failures + 1 success == 4 attempts");
        assert!(result.output.contains("finally!"));
        let expected: Duration = (2..=4)
            .map(|attempt| paladin_battalion::engine::retry::backoff_delay(&policy, attempt))
            .sum();
        assert_eq!(
            elapsed, expected,
            "the paused clock must advance by exactly the policy's own delay sequence"
        );
    }

    /// Test 4: with no resilience middleware installed, a transiently
    /// failing mock produces exactly `max_loops.min(10)` attempts with
    /// `100ms * 2^n` delays, and a Permanent error short-circuits after
    /// one -- byte-for-byte today's behavior (D-11, X-03).
    #[tokio::test(start_paused = true)]
    async fn no_resilience_middleware_keeps_todays_retry_shape() {
        let llm = Arc::new(MockLlmAdapter::new().with_error(transient("p")));
        let service = make_service(llm.clone());
        let mut paladin = make_paladin(1);
        paladin.node.max_loops = MaxLoops::Fixed(3);

        let start = tokio::time::Instant::now();
        let result = service.execute(&paladin, "hi").await;
        let elapsed = start.elapsed();

        assert!(result.is_err(), "exhausted retries must fail the run");
        assert_eq!(llm.call_count(), 3, "max_loops.min(10) attempts, unchanged");
        assert_eq!(
            elapsed,
            Duration::from_millis(100) + Duration::from_millis(200),
            "today's 100ms * 2^(attempt-1) delays before attempts 2 and 3"
        );

        // A Permanent failure short-circuits after exactly one attempt.
        let llm_permanent = Arc::new(
            MockLlmAdapter::new().with_error(LlmError::AuthenticationError("bad key".to_string())),
        );
        let service = make_service(llm_permanent.clone());
        let result = service.execute(&paladin, "hi").await;
        assert!(result.is_err());
        assert_eq!(llm_permanent.call_count(), 1);
    }

    /// Test 5: `ModelFallbackMiddleware::new(vec![])` fails AT
    /// CONSTRUCTION with `FallbackLlmAdapter`'s own empty-chain error, not
    /// at first use.
    #[test]
    fn fallback_chain_construction_validates_up_front() {
        let result = ModelFallbackMiddleware::new(Vec::new());
        assert!(matches!(result, Err(FallbackChainError::EmptyChain)));
    }

    /// Test 6 (T-26-34): two concurrent runs on two DIFFERENT services --
    /// one carrying a fallback override, one with no middleware at all --
    /// each use their own effective port. The non-override run's calls
    /// never reach the other service's ports, proving the override lives
    /// on the per-run context and cannot leak across runs.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_runs_do_not_share_an_override() {
        let primary: Arc<dyn LlmPort> = Arc::new(
            MockLlmAdapter::new()
                .with_provider_name("primary")
                .with_error(transient("primary")),
        );
        let backup: Arc<dyn LlmPort> = Arc::new(
            MockLlmAdapter::new()
                .with_provider_name("backup")
                .with_response("from backup"),
        );
        let fallback_middleware =
            Arc::new(ModelFallbackMiddleware::new(vec![primary, backup]).unwrap());
        let fallback_service_default: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new());
        let fallback_service =
            make_service(fallback_service_default).with_middleware(fallback_middleware);

        let plain_llm = Arc::new(MockLlmAdapter::new().with_response("from plain service"));
        let plain_service = make_service(plain_llm.clone());

        let paladin = make_paladin(1);
        let (fallback_result, plain_result) = tokio::join!(
            fallback_service.execute(&paladin, "hi"),
            plain_service.execute(&paladin, "hi")
        );

        assert_eq!(
            fallback_result.unwrap().served_by.as_deref(),
            Some("backup")
        );
        assert!(plain_result.unwrap().output.contains("from plain service"));
        assert_eq!(
            plain_llm.call_count(),
            1,
            "the plain service's own port must be the only one it ever calls"
        );
    }

    /// Test 7: both middlewares installed -- the retry policy applies to
    /// the fallback adapter AS the effective port, and the attempt count
    /// is the policy's own `max_attempts`, not doubled by the chain's own
    /// two providers.
    #[tokio::test(start_paused = true)]
    async fn retry_and_fallback_compose() {
        let p1 = Arc::new(
            MockLlmAdapter::new()
                .with_provider_name("p1")
                .with_error(transient("p1")),
        );
        let p2 = Arc::new(
            MockLlmAdapter::new()
                .with_provider_name("p2")
                .with_error(transient("p2")),
        );
        let chain: Vec<Arc<dyn LlmPort>> = vec![p1.clone(), p2.clone()];
        let fallback = Arc::new(ModelFallbackMiddleware::new(chain).unwrap());
        let policy = RetryPolicy {
            max_attempts: 2,
            jitter: false,
            retry_on: RetryPredicate::TransientOnly,
            ..RetryPolicy::default()
        };
        let retry = Arc::new(ModelRetryMiddleware::new(policy));
        let service_default: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new());
        let service = make_service(service_default)
            .with_middleware(fallback)
            .with_middleware(retry);
        let paladin = make_paladin(1);

        let result = service.execute(&paladin, "hi").await;

        assert!(result.is_err(), "both providers always fail");
        // 2 outer retry attempts x 2 providers per fallback-adapter call ==
        // 4 total provider calls -- exactly the policy's own attempt
        // count, not the chain length multiplied by itself.
        assert_eq!(p1.call_count(), 2);
        assert_eq!(p2.call_count(), 2);
    }

    // ── WR-01 (28-REVIEW): Retry/Fallback events fire only when the
    // action actually happens, never unconditionally on every call ────────

    use paladin_core::platform::container::waypoint::ThreadId;
    use paladin_ports::output::trace_sink_port::{
        StandaloneEmitter, TraceEvent, TraceRecord, TraceSink, TraceSinkError,
    };

    #[derive(Default)]
    struct RecordingSink {
        events: std::sync::Mutex<Vec<TraceRecord>>,
    }

    #[async_trait]
    impl TraceSink for RecordingSink {
        async fn on_event(&self, record: TraceRecord) -> Result<(), TraceSinkError> {
            self.events.lock().unwrap().push(record);
            Ok(())
        }
    }

    async fn middleware_events_eventually(
        sink: &RecordingSink,
        min_len: usize,
    ) -> Vec<(String, MiddlewareAction)> {
        for _ in 0..300 {
            let events = sink.events.lock().unwrap().clone();
            let middleware_events: Vec<(String, MiddlewareAction)> = events
                .iter()
                .filter_map(|r| match &r.event {
                    TraceEvent::MiddlewareEvent { name, action } => Some((name.clone(), *action)),
                    _ => None,
                })
                .collect();
            if middleware_events.len() >= min_len {
                return middleware_events;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        sink.events
            .lock()
            .unwrap()
            .iter()
            .filter_map(|r| match &r.event {
                TraceEvent::MiddlewareEvent { name, action } => Some((name.clone(), *action)),
                _ => None,
            })
            .collect()
    }

    /// WR-01: a call that succeeds on its FIRST attempt with
    /// `ModelRetryMiddleware` installed must emit ZERO
    /// `MiddlewareAction::Retry` events -- before the fix, `before_model`
    /// set the hint unconditionally, so even a healthy call emitted one.
    #[tokio::test]
    async fn retry_middleware_emits_no_event_when_first_attempt_succeeds() {
        let sink = Arc::new(RecordingSink::default());
        let emitter: Arc<dyn paladin_ports::output::trace_sink_port::TraceEmitter> =
            Arc::new(StandaloneEmitter::new(
                ThreadId::new("wr-01-retry-no-op").unwrap(),
                None,
                Some(sink.clone() as Arc<dyn TraceSink>),
            ));
        let llm = Arc::new(MockLlmAdapter::new().with_response("first try"));
        let middleware = Arc::new(ModelRetryMiddleware::new(RetryPolicy::default()));
        let service = make_service(llm.clone())
            .with_middleware(middleware)
            .with_trace_emitter(emitter);
        let paladin = make_paladin(1);

        let result = service.execute(&paladin, "hi").await.unwrap();

        assert!(result.output.contains("first try"));
        assert_eq!(llm.call_count(), 1);
        // Give the fire-and-forget dispatcher a moment to drain, then
        // confirm no Retry event ever arrived.
        tokio::time::sleep(Duration::from_millis(50)).await;
        let events = sink.events.lock().unwrap().clone();
        let retry_events: Vec<&TraceRecord> = events
            .iter()
            .filter(|r| {
                matches!(
                    &r.event,
                    TraceEvent::MiddlewareEvent {
                        action: MiddlewareAction::Retry,
                        ..
                    }
                )
            })
            .collect();
        assert!(
            retry_events.is_empty(),
            "a first-attempt success must emit no Retry event, got: {retry_events:?}"
        );
    }

    /// WR-01: a call that fails transiently 3 times then succeeds must
    /// emit exactly 3 `MiddlewareAction::Retry` events -- one per actual
    /// retry, named for the middleware's own `name()` (`"model_retry"`) --
    /// never one per `before_model` invocation (4, counting the final
    /// successful attempt).
    #[tokio::test(start_paused = true)]
    async fn retry_middleware_emits_exactly_one_event_per_actual_retry() {
        let sink = Arc::new(RecordingSink::default());
        let emitter: Arc<dyn paladin_ports::output::trace_sink_port::TraceEmitter> =
            Arc::new(StandaloneEmitter::new(
                ThreadId::new("wr-01-retry-count").unwrap(),
                None,
                Some(sink.clone() as Arc<dyn TraceSink>),
            ));
        let llm = Arc::new(MockLlmAdapter::new().with_script(vec![
            MockScriptEntry::Error(transient("p")),
            MockScriptEntry::Error(transient("p")),
            MockScriptEntry::Error(transient("p")),
            MockScriptEntry::Text("finally!".to_string()),
        ]));
        let policy = RetryPolicy {
            max_attempts: 4,
            jitter: false,
            ..RetryPolicy::default()
        };
        let middleware = Arc::new(ModelRetryMiddleware::new(policy));
        let service = make_service(llm.clone())
            .with_middleware(middleware)
            .with_trace_emitter(emitter);
        let paladin = make_paladin(1);

        let result = service.execute(&paladin, "hi").await.unwrap();
        assert!(result.output.contains("finally!"));
        assert_eq!(llm.call_count(), 4);

        let observed = middleware_events_eventually(&sink, 3).await;
        let retry_events: Vec<&(String, MiddlewareAction)> = observed
            .iter()
            .filter(|(_, action)| *action == MiddlewareAction::Retry)
            .collect();
        assert_eq!(
            retry_events.len(),
            3,
            "exactly 3 actual retries, not 4 before_model calls: {observed:?}"
        );
        assert!(
            retry_events.iter().all(|(name, _)| name == "model_retry"),
            "every Retry event must name the middleware's own name(): {observed:?}"
        );
    }

    /// WR-01: a call whose primary port succeeds immediately with
    /// `ModelFallbackMiddleware` installed must emit ZERO
    /// `MiddlewareAction::Fallback` events -- before the fix, `before_model`
    /// set the hint unconditionally on every call, healthy or not.
    #[tokio::test]
    async fn fallback_middleware_emits_no_event_when_primary_succeeds() {
        let sink = Arc::new(RecordingSink::default());
        let emitter: Arc<dyn paladin_ports::output::trace_sink_port::TraceEmitter> =
            Arc::new(StandaloneEmitter::new(
                ThreadId::new("wr-01-fallback-no-op").unwrap(),
                None,
                Some(sink.clone() as Arc<dyn TraceSink>),
            ));
        let primary: Arc<dyn LlmPort> = Arc::new(
            MockLlmAdapter::new()
                .with_provider_name("primary")
                .with_response("served by primary"),
        );
        let backup: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new().with_provider_name("backup"));
        let middleware = Arc::new(ModelFallbackMiddleware::new(vec![primary, backup]).unwrap());
        let service_default: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new());
        let service = make_service(service_default)
            .with_middleware(middleware)
            .with_trace_emitter(emitter);
        let paladin = make_paladin(1);

        let result = service.execute(&paladin, "hi").await.unwrap();
        assert!(result.output.contains("served by primary"));

        tokio::time::sleep(Duration::from_millis(50)).await;
        let events = sink.events.lock().unwrap().clone();
        let fallback_events: Vec<&TraceRecord> = events
            .iter()
            .filter(|r| {
                matches!(
                    &r.event,
                    TraceEvent::MiddlewareEvent {
                        action: MiddlewareAction::Fallback,
                        ..
                    }
                )
            })
            .collect();
        assert!(
            fallback_events.is_empty(),
            "a healthy primary-served call must emit no Fallback event, got: {fallback_events:?}"
        );
    }

    /// WR-01: even when a real hop DOES occur, `ModelFallbackMiddleware`
    /// itself no longer emits a `MiddlewareAction::Fallback` event --
    /// `FallbackLlmAdapter`'s own `TraceEvent::FallbackHop` (a distinct
    /// event, always accurate to a real hop) is the source of truth.
    #[tokio::test]
    async fn fallback_middleware_emits_no_middleware_action_event_on_a_real_hop() {
        let sink = Arc::new(RecordingSink::default());
        let emitter: Arc<dyn paladin_ports::output::trace_sink_port::TraceEmitter> =
            Arc::new(StandaloneEmitter::new(
                ThreadId::new("wr-01-fallback-real-hop").unwrap(),
                None,
                Some(sink.clone() as Arc<dyn TraceSink>),
            ));
        let primary: Arc<dyn LlmPort> = Arc::new(
            MockLlmAdapter::new()
                .with_provider_name("primary")
                .with_error(transient("primary")),
        );
        let backup: Arc<dyn LlmPort> = Arc::new(
            MockLlmAdapter::new()
                .with_provider_name("backup")
                .with_response("served by backup"),
        );
        let middleware = Arc::new(ModelFallbackMiddleware::new(vec![primary, backup]).unwrap());
        let service_default: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new());
        let service = make_service(service_default)
            .with_middleware(middleware)
            .with_trace_emitter(emitter.clone());
        let paladin = make_paladin(1);

        // `FallbackLlmAdapter::record_hop` (no explicit `with_trace_emitter`
        // of its own here, since `ModelFallbackMiddleware` never sets one)
        // falls back to the AMBIENT `current_trace_emitter()` a real run's
        // worker dispatch sets via this task-local -- so the scope is
        // required for `FallbackHop` itself to be observable here, mirroring
        // `fallback_hop_lands_in_the_run_sequence` in
        // `crates/paladin-llm/src/fallback.rs`.
        let result = paladin_ports::output::trace_sink_port::RUN_TRACE_EMITTER
            .scope(emitter, service.execute(&paladin, "hi"))
            .await
            .unwrap();
        assert_eq!(result.served_by.as_deref(), Some("backup"));

        tokio::time::sleep(Duration::from_millis(50)).await;
        let events = sink.events.lock().unwrap().clone();
        assert!(
            events
                .iter()
                .any(|r| matches!(r.event, TraceEvent::FallbackHop { .. })),
            "a real hop must still produce FallbackHop: {events:?}"
        );
        let fallback_middleware_events: Vec<&TraceRecord> = events
            .iter()
            .filter(|r| {
                matches!(
                    &r.event,
                    TraceEvent::MiddlewareEvent {
                        action: MiddlewareAction::Fallback,
                        ..
                    }
                )
            })
            .collect();
        assert!(
            fallback_middleware_events.is_empty(),
            "ModelFallbackMiddleware must never emit its own MiddlewareAction::Fallback: {fallback_middleware_events:?}"
        );
    }
}
