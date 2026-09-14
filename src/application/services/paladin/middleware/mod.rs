//! The `ExecutionMiddleware` seam: a hook chain running INSIDE
//! [`crate::application::services::paladin::paladin_execution_service::PaladinExecutionService`]'s
//! reasoning loop (`execute_internal`), beside the loop it wraps -- the same
//! house pattern `crate::engine::hooks::NodeInterceptor` puts beside the
//! superstep loop it wraps.
//!
//! # Cost model: an empty chain reproduces today's bytes exactly
//!
//! With no middleware attached, the rendered prompt, the `LlmPort` call
//! count and the returned `PaladinResult` are byte-identical to today's
//! behaviour (D-02's locked invariant, proven by
//! `empty_chain_renders_byte_identical_prompt`). This is the same "the
//! untraced path costs nothing" framing `TraceDispatcher`'s module doc
//! states for its own empty-sink case.
//!
//! # Two layers, not one (D-05)
//!
//! | | [`crate::engine::hooks::NodeInterceptor`] (`paladin-battalion`) | [`ExecutionMiddleware`] (here) |
//! |---|---|---|
//! | Scope | The whole node, once per Aegis attempt | One model call, or one tool call |
//! | Frequency | Once per node execution attempt | Once per reasoning-loop iteration / once per tool dispatch |
//! | Owner crate | `paladin-battalion` (engine-side) | This facade crate, beside the service |
//! | What it can do | Skip/fail the node before it runs, mutate its resulting delta | Rewrite the prompt, inspect/rewrite a model response, allow/deny/rewrite a tool call |
//!
//! A `WarEngine` dispatches every `NodeSpec::Paladin` node through
//! `PaladinPort::execute_observed`, so a `PaladinExecutionService` carrying
//! middleware applies its chain unchanged when running as a node -- no
//! engine-side middleware registry exists or is needed (D-05).

pub mod chain;
pub mod context;
pub mod guardrail;
/// Keeps a run's Garrison history within a model's context window
/// (Doc 05 RT-FR-08/10/11/12, D-14, D-15).
pub mod history;
pub mod limits;
pub mod resilience;
/// Compresses an over-long conversation into a compounding Garrison
/// summary, degrading to trimming rather than ever failing the run
/// (Doc 05 RT-FR-08/11/12, D-16).
pub mod summarization;
/// A prompt-level tool-call protocol (`ToolCallProtocolMiddleware`) and the
/// separate `FinishOnPlainAnswerMiddleware` opt-in (Doc 05 RT-FR-23/24,
/// D-36).
pub mod tool_protocol;
/// Recalls long-term Vault memory into a delimited prompt section on the
/// first loop iteration, best-effort (Doc 05 RT-FR-13…16, D-25, D-41).
pub mod vault_recall;

pub use chain::{BeforeOutcome, run_after, run_around_tool, run_before};
pub use context::{
    LlmResponseView, ModelCallContext, PromptAssembly, PromptSection, SectionPlacement,
    ToolCallContext, ToolCallKind,
};
pub use guardrail::{
    Guardrail, GuardrailAction, GuardrailBuildError, GuardrailMatcher, GuardrailRule,
    GuardrailTarget,
};
pub use history::HistoryTrimmer;
pub use limits::{ModelCallLimit, TokenBudget, ToolCallLimit};
pub use resilience::{ModelFallbackMiddleware, ModelRetryMiddleware};
pub use summarization::{SUMMARIZATION_DEGRADED_KEY, SummarizationMiddleware};
pub use tool_protocol::{FinishOnPlainAnswerMiddleware, ToolCallProtocolMiddleware};
pub use vault_recall::VaultRecallMiddleware;

use crate::application::services::paladin::error::PaladinError;
use crate::core::platform::container::arsenal::ArmamentCall;
use paladin_ports::output::paladin_port::StopReason;

/// The outcome of running through the whole `before_model`/`after_model`
/// chain and finishing the run early, without a further model call.
#[derive(Debug, Clone)]
pub struct FinalResult {
    /// The output to return as the run's `PaladinResult::output`.
    pub output: String,
    /// The `StopReason` to record for the run.
    pub stop_reason: StopReason,
}

impl FinalResult {
    /// Construct a `FinalResult`.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin::application::services::paladin::middleware::FinalResult;
    /// use paladin_ports::output::paladin_port::StopReason;
    ///
    /// let result = FinalResult::new("done", StopReason::Completed);
    /// assert_eq!(result.output, "done");
    /// ```
    pub fn new(output: impl Into<String>, stop_reason: StopReason) -> Self {
        Self {
            output: output.into(),
            stop_reason,
        }
    }
}

/// What a `before_model`/`after_model` hook decides.
///
/// `#[non_exhaustive]`: new variants may be added later without breaking
/// every existing `match`.
#[derive(Debug)]
#[non_exhaustive]
pub enum MiddlewareFlow {
    /// Proceed to the next middleware (or, at the end of the chain, to the
    /// model call / to returning the response).
    Continue,
    /// Finish the run now with `FinalResult`, without a further model call.
    /// Skips every remaining middleware's `before_model` and the finishing
    /// middleware's own `after_model`, but still runs `after_model` for
    /// every middleware BEFORE it in the chain (D-06).
    Finish(FinalResult),
    /// Fail the run with this error, unchanged and not retried (D-06). No
    /// `catch_unwind` converts a panic inside a hook into this variant --
    /// a panic propagates like any other library panic.
    Fail(PaladinError),
}

/// What an `around_tool` hook decides for one tool/handoff dispatch.
///
/// `#[non_exhaustive]`: new variants may be added later without breaking
/// every existing `match`.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ToolFlow {
    /// Allow the call to dispatch unchanged.
    Allow,
    /// Deny the call. `reason` is injected into the accumulated output at
    /// exactly the position the existing tool-error arm writes to; the
    /// Arsenal (or `HandoffService`) is never invoked.
    Deny {
        /// The model-facing reason the call was denied.
        reason: String,
    },
    /// Replace the call with `ArmamentCall` before dispatch.
    Rewrite(ArmamentCall),
}

/// The seam a call site hooks into: an ordered `Vec<Arc<dyn
/// ExecutionMiddleware>>` running inside the reasoning loop, once per model
/// call (`before_model`/`after_model`) and once per tool/handoff dispatch
/// (`around_tool`).
///
/// Every hook has a default no-op body (`Continue` / `Allow`); `name()` has
/// no default. Middleware are stateless `Arc<dyn>` values -- all per-run
/// mutable state lives on [`ModelCallContext`]/[`ToolCallContext`] (D-03),
/// so the SAME `Arc<dyn ExecutionMiddleware>` can back many concurrent runs
/// through one [`crate::application::services::paladin::paladin_execution_service::PaladinExecutionService`]
/// instance with independent per-run state and no `MiddlewareFactory`.
#[async_trait::async_trait]
pub trait ExecutionMiddleware: Send + Sync {
    /// Runs once per reasoning-loop iteration, after the
    /// [`PromptAssembly`] is built and before the model call.
    async fn before_model(
        &self,
        cx: &mut ModelCallContext<'_>,
    ) -> Result<MiddlewareFlow, PaladinError> {
        let _ = cx;
        Ok(MiddlewareFlow::Continue)
    }

    /// Runs once per reasoning-loop iteration, on the FINAL response for
    /// that iteration -- after the service's own buffered retry and circuit
    /// breaker, never per attempt.
    async fn after_model(
        &self,
        cx: &mut ModelCallContext<'_>,
        resp: &mut LlmResponseView,
    ) -> Result<MiddlewareFlow, PaladinError> {
        let _ = (cx, resp);
        Ok(MiddlewareFlow::Continue)
    }

    /// Runs once per tool/handoff dispatch, wrapping both the Arsenal
    /// branch and the handoff branch of the reasoning loop.
    ///
    /// `cx` is `&mut` (Doc 05 D-08): a per-tool-call middleware like
    /// `ToolCallLimit` reads and writes its counters through
    /// [`ToolCallContext::scratch`], which the service copies back into
    /// the run's own [`ModelCallContext::scratch`] after this hook chain
    /// returns -- so a counter incremented on call N is visible on call
    /// N+1, still entirely off the middleware struct (D-03).
    async fn around_tool(&self, cx: &mut ToolCallContext) -> Result<ToolFlow, PaladinError> {
        let _ = cx;
        Ok(ToolFlow::Allow)
    }

    /// This middleware's name, used as part of the typed-state bag's key
    /// and in logs. No default -- every middleware must name itself.
    fn name(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::services::paladin::paladin_execution_service::PaladinExecutionService;
    use crate::core::base::entity::node::Node;
    use crate::core::platform::container::paladin::{MaxLoops, Paladin, PaladinData};
    use crate::infrastructure::resilience::circuit_breaker::CircuitBreaker;
    use paladin_llm::mock::MockLlmAdapter;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration;

    fn make_paladin(max_loops: u32) -> Paladin {
        let data = PaladinData {
            system_prompt: "system".to_string(),
            max_loops: MaxLoops::Fixed(max_loops),
            ..Default::default()
        };
        Node::new(data, None)
    }

    /// Counts `before_model` calls and records the `loop_index` sequence
    /// observed under each distinct `run_id` -- proving per-run isolation
    /// falls out of D-03's "state lives on the context" decision with NO
    /// factory trait: this middleware is a single `Arc` shared by every
    /// concurrent run in `concurrent_runs_keep_independent_context_state`,
    /// and it keeps no run-keyed state of its own beyond this test's own
    /// observation log (a real built-in would use `cx.scratch` instead).
    struct CountingMiddleware {
        observed_by_run: Mutex<HashMap<uuid::Uuid, Vec<u32>>>,
        total_calls: AtomicU32,
    }

    #[async_trait::async_trait]
    impl ExecutionMiddleware for CountingMiddleware {
        async fn before_model(
            &self,
            cx: &mut ModelCallContext<'_>,
        ) -> Result<MiddlewareFlow, PaladinError> {
            self.total_calls.fetch_add(1, Ordering::SeqCst);
            self.observed_by_run
                .lock()
                .unwrap()
                .entry(cx.run_id)
                .or_default()
                .push(cx.loop_index);
            Ok(MiddlewareFlow::Continue)
        }

        fn name(&self) -> &str {
            "counting"
        }
    }

    /// RT-FR-02's contract and this phase's first X-05 stress obligation:
    /// ten concurrent runs through ONE `PaladinExecutionService` instance,
    /// carrying one `CountingMiddleware` `Arc` shared by all ten, each keep
    /// exactly their own `loop_index` sequence and the total `LlmPort` call
    /// count is exactly 30 -- never "at least".
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_runs_keep_independent_context_state() {
        let llm = Arc::new(MockLlmAdapter::new().with_response("ack"));
        let middleware = Arc::new(CountingMiddleware {
            observed_by_run: Mutex::new(HashMap::new()),
            total_calls: AtomicU32::new(0),
        });
        let service = Arc::new(
            PaladinExecutionService::new(
                llm.clone(),
                Arc::new(CircuitBreaker::new(50, 25, Duration::from_secs(60))),
                None,
                None,
            )
            .with_middleware(middleware.clone() as Arc<dyn ExecutionMiddleware>),
        );
        let paladin = Arc::new(make_paladin(3));

        let handles = (0..10).map(|_| {
            let service = service.clone();
            let paladin = paladin.clone();
            tokio::spawn(async move { service.execute(&paladin, "hi").await })
        });

        let results =
            tokio::time::timeout(Duration::from_secs(10), futures::future::join_all(handles))
                .await
                .expect("all ten concurrent runs must complete within the 10s guard");

        for result in results {
            result
                .expect("run task must not panic")
                .expect("run must succeed");
        }

        assert_eq!(
            llm.call_count(),
            30,
            "10 runs x 3 loop iterations each == 30 LlmPort calls"
        );
        assert_eq!(middleware.total_calls.load(Ordering::SeqCst), 30);

        let observed = middleware.observed_by_run.lock().unwrap();
        assert_eq!(observed.len(), 10, "ten distinct run_ids");
        for sequence in observed.values() {
            assert_eq!(
                *sequence,
                vec![0, 1, 2],
                "each run must observe loop_index 0,1,2 exactly, with no cross-run leakage"
            );
        }
    }

    /// Increments a scratch counter each `before_model` and records the
    /// value it saw at the START of every run (its first iteration).
    struct ScratchStartRecorder {
        starts: Mutex<Vec<i64>>,
    }

    #[async_trait::async_trait]
    impl ExecutionMiddleware for ScratchStartRecorder {
        async fn before_model(
            &self,
            cx: &mut ModelCallContext<'_>,
        ) -> Result<MiddlewareFlow, PaladinError> {
            let current = cx
                .scratch
                .get("count")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            if cx.loop_index == 0 {
                self.starts.lock().unwrap().push(current);
            }
            cx.scratch
                .insert("count".to_string(), serde_json::json!(current + 1));
            Ok(MiddlewareFlow::Continue)
        }

        fn name(&self) -> &str {
            "scratch-start-recorder"
        }
    }

    /// Two sequential runs through ONE service instance observe the SAME
    /// starting scratch value -- the second run does not inherit the
    /// first's scratch (D-03: scratch lives on a fresh `ModelCallContext`
    /// per `execute_internal` call, not on the stateless middleware).
    #[tokio::test]
    async fn sequential_runs_do_not_leak_scratch() {
        let llm = Arc::new(MockLlmAdapter::new().with_response("ack"));
        let middleware = Arc::new(ScratchStartRecorder {
            starts: Mutex::new(Vec::new()),
        });
        let service = PaladinExecutionService::new(
            llm,
            Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60))),
            None,
            None,
        )
        .with_middleware(middleware.clone() as Arc<dyn ExecutionMiddleware>);
        let paladin = make_paladin(2);

        service.execute(&paladin, "hi").await.unwrap();
        service.execute(&paladin, "hi").await.unwrap();

        assert_eq!(
            *middleware.starts.lock().unwrap(),
            vec![0, 0],
            "the second run must start from the same scratch value as the first"
        );
    }

    /// Two middleware with different `name()` values storing the same `T`
    /// see independent values; one middleware storing two different `T`s
    /// sees both, independently.
    #[tokio::test]
    async fn typed_state_is_keyed_by_middleware_name_and_type() {
        #[derive(Default)]
        struct CounterA(i32);
        #[derive(Default)]
        struct CounterB(i32);

        let paladin = make_paladin(1);
        let assembly = PromptAssembly::new("system", "input", "", vec![], None);
        let mut cx = ModelCallContext::new(uuid::Uuid::new_v4(), &paladin, assembly);

        cx.state_mut::<CounterA>("mw-a").0 = 1;
        cx.state_mut::<CounterA>("mw-b").0 = 2;
        assert_eq!(cx.state::<CounterA>("mw-a").0, 1);
        assert_eq!(cx.state::<CounterA>("mw-b").0, 2);

        cx.state_mut::<CounterB>("mw-a").0 = 42;
        assert_eq!(
            cx.state::<CounterA>("mw-a").0,
            1,
            "a different T under the same middleware name stays independent"
        );
        assert_eq!(cx.state::<CounterB>("mw-a").0, 42);
    }
}
