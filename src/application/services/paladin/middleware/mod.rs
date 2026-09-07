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

pub use chain::{BeforeOutcome, run_after, run_around_tool, run_before};
pub use context::{
    LlmResponseView, ModelCallContext, PromptAssembly, PromptSection, SectionPlacement,
    ToolCallContext, ToolCallKind,
};

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
    async fn around_tool(&self, cx: &ToolCallContext) -> Result<ToolFlow, PaladinError> {
        let _ = cx;
        Ok(ToolFlow::Allow)
    }

    /// This middleware's name, used as part of the typed-state bag's key
    /// and in logs. No default -- every middleware must name itself.
    fn name(&self) -> &str;
}
