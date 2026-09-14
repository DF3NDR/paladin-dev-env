//! Built-in budget middlewares: [`ModelCallLimit`], [`TokenBudget`] and
//! [`ToolCallLimit`] (Doc 05 D-08, RT-02/RT-FR-04/RT-FR-06).
//!
//! All three end a run or a call **gracefully**; none of them is an error
//! path. `ModelCallLimit` and `TokenBudget` finish the run with
//! [`StopReason::CallLimit`]/[`StopReason::TokenBudget`] -- both
//! `is_successful() == true` (the model's last answer stands, a budget
//! stopped the run, not a failure). `ToolCallLimit` denies the individual
//! call through [`ToolFlow::Deny`] and lets the run continue. None of the
//! three ever returns [`MiddlewareFlow::Fail`].
//!
//! Every one of the three is constructed from its own `AgentRuntimeConfig`
//! sub-struct (`ModelCallLimitConfig`, `TokenBudgetConfig`,
//! `ToolCallLimitConfig`, D-10) and is **inert by default**: each config
//! sub-struct's `enabled` field defaults to `false`, and every `before_model`
//! / `after_model` / `around_tool` implementation below checks it first and
//! returns `Continue`/`Allow` immediately when disabled -- installing the
//! chain on a v0.9 deployment (whose config carries no `agent_runtime:`
//! section) changes nothing.
//!
//! # No counter lives on the middleware struct (D-03)
//!
//! `ModelCallLimit` and `TokenBudget` keep their running counts on
//! [`ModelCallContext`]'s typed state bag and its `cumulative_tokens` field
//! respectively -- both per-run scratch that a fresh `ModelCallContext`
//! resets to zero every `execute_internal` call. `ToolCallLimit` keeps its
//! counters in [`ToolCallContext::scratch`], which the service copies back
//! onto the run's own `ModelCallContext::scratch` after every `around_tool`
//! dispatch (see that field's doc comment). The SAME `Arc<dyn
//! ExecutionMiddleware>` therefore backs many concurrent runs through one
//! `PaladinExecutionService` with fully independent per-run counts and no
//! `MiddlewareFactory`.

use std::collections::HashMap;

use async_trait::async_trait;

use crate::application::services::paladin::error::PaladinError;
use crate::config::agent_runtime::{ModelCallLimitConfig, TokenBudgetConfig, ToolCallLimitConfig};
use paladin_ports::output::paladin_port::StopReason;

use super::{
    ExecutionMiddleware, FinalResult, LlmResponseView, MiddlewareFlow, ModelCallContext,
    ToolCallContext, ToolFlow,
};

/// The text appended to the accumulated output when [`ModelCallLimit`]
/// finishes a run. Written once so the implementation and any test
/// asserting on it cannot drift.
const CALL_LIMIT_NOTICE: &str =
    "\n\n[budget] Model call limit reached — the response above is this run's final answer.";

/// The text appended to the crossing response's content when [`TokenBudget`]
/// finishes a run. Written once so the implementation and any test
/// asserting on it cannot drift.
const TOKEN_BUDGET_NOTICE: &str =
    "\n\n[budget] Token budget reached — the response above is this run's final answer.";

/// Per-run call count for [`ModelCallLimit`], stored on
/// [`ModelCallContext`]'s typed state bag (D-03) -- never a field on the
/// middleware struct.
#[derive(Default)]
struct ModelCallCount(u32);

/// Caps the number of model calls the reasoning loop makes (D-08,
/// RT-FR-04).
///
/// Counts once per `before_model`, which fires once per reasoning-loop
/// iteration -- **after** `execute_with_retry_and_temperature`'s own
/// buffered retries have already resolved to a single outcome for that
/// iteration. A retried-but-eventually-successful iteration therefore still
/// consumes exactly one of `max_calls`; the service's internal retry
/// attempts and a summarizer's own model call (outside any run's middleware
/// chain entirely) are never counted.
///
/// On the call that would make the count exceed `max_calls`, `before_model`
/// finishes the run with [`StopReason::CallLimit`] instead of proceeding to
/// that model call -- so a `max_calls: 3` run makes exactly 3 model calls,
/// never 2 and never 4.
pub struct ModelCallLimit {
    config: ModelCallLimitConfig,
}

impl ModelCallLimit {
    /// Construct a `ModelCallLimit` from its config sub-struct (D-10).
    pub fn new(config: ModelCallLimitConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ExecutionMiddleware for ModelCallLimit {
    async fn before_model(
        &self,
        cx: &mut ModelCallContext<'_>,
    ) -> Result<MiddlewareFlow, PaladinError> {
        if !self.config.enabled {
            return Ok(MiddlewareFlow::Continue);
        }

        let count = {
            let counter = cx.state_mut::<ModelCallCount>(self.name());
            counter.0 += 1;
            counter.0
        };

        if count > self.config.max_calls {
            let output = format!("{}{}", cx.assembly.accumulated_output, CALL_LIMIT_NOTICE);
            return Ok(MiddlewareFlow::Finish(FinalResult::new(
                output,
                StopReason::CallLimit,
            )));
        }

        Ok(MiddlewareFlow::Continue)
    }

    fn name(&self) -> &str {
        "model_call_limit"
    }
}

/// Caps the accumulated `total_tokens` a run may spend (D-08, RT-FR-06).
///
/// Reads [`ModelCallContext::cumulative_tokens`] in `after_model` -- the
/// existing running sum the service updates immediately after every model
/// call returns and before `after_model` fires, so this middleware keeps
/// no separate counter of its own. Once the total crosses `max_tokens`,
/// `after_model` finishes the run with [`StopReason::TokenBudget`], keeping
/// the crossing response's content (plus a truncation notice) as the run's
/// output. Because the budget is only checked AFTER a response has already
/// arrived, the documented overshoot is at most one response's worth of
/// tokens.
pub struct TokenBudget {
    config: TokenBudgetConfig,
}

impl TokenBudget {
    /// Construct a `TokenBudget` from its config sub-struct (D-10).
    pub fn new(config: TokenBudgetConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ExecutionMiddleware for TokenBudget {
    async fn after_model(
        &self,
        cx: &mut ModelCallContext<'_>,
        resp: &mut LlmResponseView,
    ) -> Result<MiddlewareFlow, PaladinError> {
        if !self.config.enabled {
            return Ok(MiddlewareFlow::Continue);
        }

        if cx.cumulative_tokens > self.config.max_tokens {
            // The service's `run_after` call site reads the FINAL output
            // from `resp.content` (the mutable response view), not from
            // `FinalResult::output` -- so the notice must land on `resp`
            // itself to reach the returned `PaladinResult`.
            resp.content.push_str(TOKEN_BUDGET_NOTICE);
            return Ok(MiddlewareFlow::Finish(FinalResult::new(
                resp.content.clone(),
                StopReason::TokenBudget,
            )));
        }

        Ok(MiddlewareFlow::Continue)
    }

    fn name(&self) -> &str {
        "token_budget"
    }
}

/// The fixed, model-facing denial message [`ToolCallLimit`] returns through
/// [`ToolFlow::Deny`]. Written once as a formatter so the implementation and
/// any test asserting on it cannot drift (Doc 05 D-08).
fn tool_budget_exhausted_message(tool_name: &str) -> String {
    format!("tool budget exhausted for `{tool_name}`")
}

/// The scratch key [`ToolCallLimit`] stores its global call count under, on
/// [`ToolCallContext::scratch`] (D-08 -- never a field on the middleware
/// struct itself).
const TOOL_CALL_LIMIT_TOTAL_KEY: &str = "tool_call_limit.total";
/// The scratch key [`ToolCallLimit`] stores its per-tool call counts under.
const TOOL_CALL_LIMIT_PER_TOOL_KEY: &str = "tool_call_limit.per_tool";

/// Caps the number of tool (Armament/handoff) calls a run may make,
/// overall and per named tool (D-08, RT-FR-05).
///
/// Implements `around_tool` only: reads the per-run global count and the
/// per-run per-tool map from [`ToolCallContext::scratch`] (a working copy
/// the service persists back onto the run's `ModelCallContext::scratch`
/// after every dispatch), increments both for `cx.call.tool_name`, and
/// denies through [`ToolFlow::Deny`] -- naming the tool in a fixed message
/// -- when either the global cap or that tool's own cap would be exceeded.
/// A denial never fails the run and never rewrites the call; the Arsenal
/// (or `HandoffService`) is simply never invoked for that one call.
///
/// This middleware does **not** distinguish
/// [`super::ToolCallKind::Armament`] from [`super::ToolCallKind::Handoff`]
/// -- both count against the same budget, per D-04's rule that a handoff is
/// a tool call the model made, not a distinct kind of call exempt from tool
/// budgeting. That is a deliberate decision, not an oversight.
pub struct ToolCallLimit {
    config: ToolCallLimitConfig,
}

impl ToolCallLimit {
    /// Construct a `ToolCallLimit` from its config sub-struct (D-10).
    pub fn new(config: ToolCallLimitConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ExecutionMiddleware for ToolCallLimit {
    async fn around_tool(&self, cx: &mut ToolCallContext) -> Result<ToolFlow, PaladinError> {
        if !self.config.enabled {
            return Ok(ToolFlow::Allow);
        }

        let tool_name = cx.call.tool_name.clone();

        let total = cx
            .scratch
            .get(TOOL_CALL_LIMIT_TOTAL_KEY)
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32
            + 1;

        let mut per_tool: HashMap<String, u32> = cx
            .scratch
            .get(TOOL_CALL_LIMIT_PER_TOOL_KEY)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        let tool_count = per_tool.entry(tool_name.clone()).or_insert(0);
        *tool_count += 1;
        let tool_count = *tool_count;

        // Persist the incremented counters into the working copy BEFORE any
        // early return, so a denied call still advances the counters the
        // way a real dispatch attempt should.
        cx.scratch.insert(
            TOOL_CALL_LIMIT_TOTAL_KEY.to_string(),
            serde_json::json!(total),
        );
        cx.scratch.insert(
            TOOL_CALL_LIMIT_PER_TOOL_KEY.to_string(),
            serde_json::to_value(&per_tool).unwrap_or_default(),
        );

        if total > self.config.max_calls {
            return Ok(ToolFlow::Deny {
                reason: tool_budget_exhausted_message(&tool_name),
            });
        }

        if let Some(per_tool_max) = self.config.per_tool.get(&tool_name)
            && tool_count > *per_tool_max
        {
            return Ok(ToolFlow::Deny {
                reason: tool_budget_exhausted_message(&tool_name),
            });
        }

        Ok(ToolFlow::Allow)
    }

    fn name(&self) -> &str {
        "tool_call_limit"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::services::paladin::paladin_execution_service::PaladinExecutionService;
    use crate::core::base::entity::node::Node;
    use crate::core::platform::container::arsenal::{ArmamentCall, ArmamentResult, ArsenalError};
    use crate::core::platform::container::paladin::{MaxLoops, Paladin, PaladinData};
    use crate::infrastructure::resilience::circuit_breaker::CircuitBreaker;
    use async_trait::async_trait;
    use paladin_llm::mock::{MockLlmAdapter, MockScriptEntry};
    use paladin_ports::output::arsenal_port::ArsenalPort;
    use paladin_ports::output::llm_port::LlmError;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    fn make_paladin(max_loops: u32) -> Paladin {
        let data = PaladinData {
            system_prompt: "system".to_string(),
            max_loops: MaxLoops::Fixed(max_loops),
            ..Default::default()
        };
        Node::new(data, None)
    }

    fn make_service(llm: Arc<MockLlmAdapter>) -> PaladinExecutionService {
        PaladinExecutionService::new(
            llm,
            Arc::new(CircuitBreaker::new(50, 25, Duration::from_secs(60))),
            None,
            None,
        )
    }

    /// An `ArsenalPort` that records every call it receives and always
    /// succeeds.
    #[derive(Default)]
    struct RecordingArsenal {
        calls: Mutex<Vec<ArmamentCall>>,
    }

    #[async_trait]
    impl ArsenalPort for RecordingArsenal {
        async fn list_armaments(&self) -> Vec<crate::core::platform::container::arsenal::Armament> {
            Vec::new()
        }

        async fn invoke(&self, call: ArmamentCall) -> Result<ArmamentResult, ArsenalError> {
            self.calls.lock().unwrap().push(call.clone());
            Ok(ArmamentResult::success(
                call.call_id,
                serde_json::json!("ok"),
                0,
            ))
        }

        fn validate_call(&self, _call: &ArmamentCall) -> Result<(), ArsenalError> {
            Ok(())
        }
    }

    fn make_service_with_arsenal(
        llm: Arc<MockLlmAdapter>,
        arsenal: Arc<dyn ArsenalPort>,
    ) -> PaladinExecutionService {
        PaladinExecutionService::new(
            llm,
            Arc::new(CircuitBreaker::new(50, 25, Duration::from_secs(60))),
            None,
            Some(arsenal),
        )
    }

    // ── ModelCallLimit ───────────────────────────────────────────────────

    /// Test 1: `max_calls: 3` over a mock that never emits a final answer
    /// (plain text, so the loop always continues) yields exactly 3 LlmPort
    /// calls, `StopReason::CallLimit`, and the accumulated output plus the
    /// truncation notice.
    #[tokio::test]
    async fn model_call_limit_finishes_at_exactly_max_calls() {
        let llm = Arc::new(MockLlmAdapter::new().with_response("still thinking"));
        let middleware = Arc::new(ModelCallLimit::new(ModelCallLimitConfig {
            enabled: true,
            max_calls: 3,
        }));
        let service = make_service(llm.clone()).with_middleware(middleware);
        let paladin = make_paladin(10);

        let result = service.execute(&paladin, "hi").await.unwrap();

        assert_eq!(llm.call_count(), 3, "exactly 3 model calls, not 2 or 4");
        assert_eq!(result.stop_reason, StopReason::CallLimit);
        assert!(result.stop_reason.is_successful());
        assert!(result.output.contains("still thinking"));
        assert!(result.output.contains(CALL_LIMIT_NOTICE));
    }

    /// Test 2: two transient errors followed by a success on the SAME
    /// loop iteration still consumes exactly one of the three budgeted
    /// calls -- the service's own buffered retries never reach
    /// `before_model` again mid-iteration.
    #[tokio::test]
    async fn model_call_limit_does_not_count_buffered_retries() {
        let llm = Arc::new(MockLlmAdapter::new().with_script(vec![
            MockScriptEntry::Error(LlmError::NetworkError("blip 1".to_string())),
            MockScriptEntry::Error(LlmError::NetworkError("blip 2".to_string())),
            MockScriptEntry::Text("recovered".to_string()),
            MockScriptEntry::Text("still going".to_string()),
            MockScriptEntry::Text("still going".to_string()),
        ]));
        let middleware = Arc::new(ModelCallLimit::new(ModelCallLimitConfig {
            enabled: true,
            max_calls: 3,
        }));
        let service = make_service(llm.clone()).with_middleware(middleware);
        // max_loops also caps the service's own retry budget
        // (`max_loops.min(10)`), so it must comfortably exceed the 3
        // retry attempts this test's first iteration needs.
        let paladin = make_paladin(10);

        let result = service.execute(&paladin, "hi").await.unwrap();

        // The mock's own call_count includes the 2 failed retry attempts
        // PLUS the 3 budgeted before_model iterations (1 recovered + 2
        // more before the limit fires) = 5 raw LlmPort calls...
        assert_eq!(llm.call_count(), 5);
        // ...but ModelCallLimit's own before_model-scoped count reached
        // exactly 3 (CallLimit fired), proving the 2 retry attempts inside
        // the FIRST iteration were invisible to the budget.
        assert_eq!(result.stop_reason, StopReason::CallLimit);
    }

    /// Test 6 (shared by both budget middlewares): a disabled config
    /// installs nothing and changes nothing -- the run's port call count
    /// and `PaladinResult` match a run with no middleware at all.
    #[tokio::test]
    async fn disabled_limit_config_installs_nothing_and_changes_nothing() {
        let llm_plain = Arc::new(MockLlmAdapter::new().with_response("ack"));
        let plain_result = make_service(llm_plain.clone())
            .execute(&make_paladin(2), "hi")
            .await
            .unwrap();

        let llm_with_disabled = Arc::new(MockLlmAdapter::new().with_response("ack"));
        let disabled_call_limit = Arc::new(ModelCallLimit::new(ModelCallLimitConfig {
            enabled: false,
            max_calls: 1,
        }));
        let disabled_token_budget = Arc::new(TokenBudget::new(TokenBudgetConfig {
            enabled: false,
            max_tokens: 1,
        }));
        let with_disabled = make_service(llm_with_disabled.clone())
            .with_middleware(disabled_call_limit)
            .with_middleware(disabled_token_budget)
            .execute(&make_paladin(2), "hi")
            .await
            .unwrap();

        assert_eq!(llm_plain.call_count(), llm_with_disabled.call_count());
        assert_eq!(plain_result.stop_reason, with_disabled.stop_reason);
        assert_eq!(plain_result.output, with_disabled.output);
        assert_eq!(plain_result.loop_count, with_disabled.loop_count);
    }

    /// Test 7: two sequential runs through ONE service instance carrying
    /// one `ModelCallLimit { max_calls: 3 }` each make exactly 3 calls --
    /// 6 total, not 3 (the second run does not inherit the first's count).
    #[tokio::test]
    async fn sequential_runs_reset_the_counters() {
        let llm = Arc::new(MockLlmAdapter::new().with_response("ack"));
        let middleware = Arc::new(ModelCallLimit::new(ModelCallLimitConfig {
            enabled: true,
            max_calls: 3,
        }));
        let service = make_service(llm.clone()).with_middleware(middleware);
        let paladin = make_paladin(10);

        let r1 = service.execute(&paladin, "hi").await.unwrap();
        let r2 = service.execute(&paladin, "hi").await.unwrap();

        assert_eq!(r1.stop_reason, StopReason::CallLimit);
        assert_eq!(r2.stop_reason, StopReason::CallLimit);
        assert_eq!(llm.call_count(), 6, "3 + 3, not 3 total");
    }

    /// Test 8 (RT-FR-02, X-05): ten concurrent runs through ONE service
    /// instance, sharing one `ModelCallLimit` `Arc`, each make exactly 3
    /// model calls with `StopReason::CallLimit` and 30 port calls total.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_runs_keep_independent_limit_counters() {
        let llm = Arc::new(MockLlmAdapter::new().with_response("ack"));
        let middleware = Arc::new(ModelCallLimit::new(ModelCallLimitConfig {
            enabled: true,
            max_calls: 3,
        }));
        let service = Arc::new(make_service(llm.clone()).with_middleware(middleware));
        let paladin = Arc::new(make_paladin(10));

        let handles = (0..10).map(|_| {
            let service = service.clone();
            let paladin = paladin.clone();
            tokio::spawn(async move { service.execute(&paladin, "hi").await })
        });

        let results =
            tokio::time::timeout(Duration::from_secs(10), futures::future::join_all(handles))
                .await
                .expect("all ten concurrent runs must complete within the 10s guard");

        let mut call_limit_count = 0;
        for result in results {
            let result = result.expect("run task must not panic").unwrap();
            assert_eq!(result.stop_reason, StopReason::CallLimit);
            call_limit_count += 1;
        }

        assert_eq!(call_limit_count, 10);
        assert_eq!(llm.call_count(), 30, "10 runs x 3 calls each == 30");
    }

    // ── TokenBudget ──────────────────────────────────────────────────────

    /// Test 3: `max_tokens: 250` with a mock reporting 100 tokens per call
    /// yields exactly 3 calls (the third crosses 250), `token_count ==
    /// 300`, `StopReason::TokenBudget`, and `is_successful()` is true.
    #[tokio::test]
    async fn token_budget_keeps_the_crossing_response_and_finishes() {
        let llm = Arc::new(
            MockLlmAdapter::new()
                .with_response("chunk")
                .with_token_usage(0, 100, 100),
        );
        let middleware = Arc::new(TokenBudget::new(TokenBudgetConfig {
            enabled: true,
            max_tokens: 250,
        }));
        let service = make_service(llm.clone()).with_middleware(middleware);
        let paladin = make_paladin(10);

        let result = service.execute(&paladin, "hi").await.unwrap();

        assert_eq!(llm.call_count(), 3);
        assert_eq!(result.token_count, 300);
        assert_eq!(result.stop_reason, StopReason::TokenBudget);
        assert!(result.stop_reason.is_successful());
        assert!(result.output.contains(TOKEN_BUDGET_NOTICE));
    }

    /// Test 4: a mock reporting 400 tokens on its first call against
    /// `max_tokens: 250` makes exactly 1 call and finishes with
    /// `TokenBudget` -- the overshoot is at most one response.
    #[tokio::test]
    async fn token_budget_overshoot_is_at_most_one_response() {
        let llm = Arc::new(
            MockLlmAdapter::new()
                .with_response("big chunk")
                .with_token_usage(0, 400, 400),
        );
        let middleware = Arc::new(TokenBudget::new(TokenBudgetConfig {
            enabled: true,
            max_tokens: 250,
        }));
        let service = make_service(llm.clone()).with_middleware(middleware);
        let paladin = make_paladin(10);

        let result = service.execute(&paladin, "hi").await.unwrap();

        assert_eq!(llm.call_count(), 1);
        assert_eq!(result.stop_reason, StopReason::TokenBudget);
    }

    /// Test 5: neither middleware ever fails the run -- both scenarios
    /// above return `Ok(PaladinResult)`.
    #[tokio::test]
    async fn limits_never_fail_the_run() {
        let llm = Arc::new(MockLlmAdapter::new().with_response("chunk"));
        let middleware = Arc::new(ModelCallLimit::new(ModelCallLimitConfig {
            enabled: true,
            max_calls: 1,
        }));
        let service = make_service(llm).with_middleware(middleware);
        let paladin = make_paladin(5);

        let result = service.execute(&paladin, "hi").await;
        assert!(result.is_ok());

        let llm2 = Arc::new(
            MockLlmAdapter::new()
                .with_response("chunk")
                .with_token_usage(0, 300, 300),
        );
        let middleware2 = Arc::new(TokenBudget::new(TokenBudgetConfig {
            enabled: true,
            max_tokens: 100,
        }));
        let service2 = make_service(llm2).with_middleware(middleware2);
        let result2 = service2.execute(&paladin, "hi").await;
        assert!(result2.is_ok());
    }

    // ── ToolCallLimit ────────────────────────────────────────────────────

    /// Test 1: `max_calls: 2` over a scripted tool-calling mock -- the
    /// first two invocations reach the Arsenal, the third is denied, and
    /// the run reaches a normal completion.
    #[tokio::test]
    async fn tool_call_limit_denies_the_call_past_the_budget() {
        let llm = Arc::new(MockLlmAdapter::new().with_script(vec![
            MockScriptEntry::ToolCall {
                name: "search".to_string(),
                arguments: "{}".to_string(),
            },
            MockScriptEntry::ToolCall {
                name: "search".to_string(),
                arguments: "{}".to_string(),
            },
            MockScriptEntry::ToolCall {
                name: "search".to_string(),
                arguments: "{}".to_string(),
            },
            MockScriptEntry::Text("done".to_string()),
        ]));
        let arsenal = Arc::new(RecordingArsenal::default());
        let middleware = Arc::new(ToolCallLimit::new(ToolCallLimitConfig {
            enabled: true,
            max_calls: 2,
            per_tool: HashMap::new(),
        }));
        let service = make_service_with_arsenal(llm, arsenal.clone() as Arc<dyn ArsenalPort>)
            .with_middleware(middleware);
        let paladin = make_paladin(4);

        let result = service.execute(&paladin, "hi").await;

        assert!(result.is_ok(), "the run reaches a normal completion");
        assert_eq!(arsenal.calls.lock().unwrap().len(), 2);
    }

    /// Test 2: the denial text names the tool and appears in the
    /// accumulated output where a tool error appears today.
    #[tokio::test]
    async fn denied_tool_call_reason_reaches_the_model() {
        // A single loop iteration: the reasoning loop overwrites
        // `accumulated_output` with each new response's content at the
        // TOP of every iteration, so the denial text (appended during
        // THIS iteration's tool-call handling) only survives into the
        // final `PaladinResult` if the run ends on this same iteration
        // (max_loops: 1) -- mirroring
        // `tool_flow_deny_injects_the_reason_where_a_tool_error_is_injected_today`'s
        // own single-loop shape in `paladin_execution_service.rs`.
        let llm = Arc::new(
            MockLlmAdapter::new().with_script(vec![MockScriptEntry::ToolCall {
                name: "search".to_string(),
                arguments: "{}".to_string(),
            }]),
        );
        let arsenal = Arc::new(RecordingArsenal::default());
        let middleware = Arc::new(ToolCallLimit::new(ToolCallLimitConfig {
            enabled: true,
            max_calls: 0,
            per_tool: HashMap::new(),
        }));
        let service = make_service_with_arsenal(llm, arsenal.clone() as Arc<dyn ArsenalPort>)
            .with_middleware(middleware);
        let paladin = make_paladin(1);

        let result = service.execute(&paladin, "hi").await.unwrap();

        assert_eq!(arsenal.calls.lock().unwrap().len(), 0);
        assert!(
            result.output.contains(&format!(
                "Error: {}",
                tool_budget_exhausted_message("search")
            )),
            "unexpected output: {}",
            result.output
        );
    }

    /// Test 3: `max_calls: 10` with `per_tool: {"search": 1}` -- the
    /// second `search` call is denied while a `calculate` call still
    /// succeeds.
    #[tokio::test]
    async fn per_tool_caps_are_independent_of_the_global_cap() {
        let llm = Arc::new(MockLlmAdapter::new().with_script(vec![
            MockScriptEntry::ToolCall {
                name: "search".to_string(),
                arguments: "{}".to_string(),
            },
            MockScriptEntry::ToolCall {
                name: "search".to_string(),
                arguments: "{}".to_string(),
            },
            MockScriptEntry::ToolCall {
                name: "calculate".to_string(),
                arguments: "{}".to_string(),
            },
            MockScriptEntry::Text("done".to_string()),
        ]));
        let arsenal = Arc::new(RecordingArsenal::default());
        let mut per_tool = HashMap::new();
        per_tool.insert("search".to_string(), 1u32);
        let middleware = Arc::new(ToolCallLimit::new(ToolCallLimitConfig {
            enabled: true,
            max_calls: 10,
            per_tool,
        }));
        let service = make_service_with_arsenal(llm, arsenal.clone() as Arc<dyn ArsenalPort>)
            .with_middleware(middleware);
        let paladin = make_paladin(4);

        service.execute(&paladin, "hi").await.unwrap();

        let calls = arsenal.calls.lock().unwrap();
        assert_eq!(
            calls.len(),
            2,
            "1 search + 1 calculate; the 2nd search denied"
        );
        assert_eq!(calls.iter().filter(|c| c.tool_name == "search").count(), 1);
        assert_eq!(
            calls.iter().filter(|c| c.tool_name == "calculate").count(),
            1
        );
    }

    /// Test 4: a handoff dispatch counts against the same budget and is
    /// denied past it (D-04: a handoff is a tool call the model made).
    #[tokio::test]
    async fn tool_call_limit_applies_to_handoff_calls() {
        let llm = Arc::new(
            MockLlmAdapter::new().with_script(vec![MockScriptEntry::ToolCall {
                name: "handoff_to_specialist".to_string(),
                arguments: r#"{"specialist_name":"x","task_description":"y"}"#.to_string(),
            }]),
        );
        let middleware = Arc::new(ToolCallLimit::new(ToolCallLimitConfig {
            enabled: true,
            max_calls: 0,
            per_tool: HashMap::new(),
        }));
        let service = make_service(llm).with_middleware(middleware);
        let paladin = make_paladin(1);

        let result = service.execute(&paladin, "hi").await.unwrap();

        assert!(
            result
                .output
                .contains(&tool_budget_exhausted_message("handoff_to_specialist")),
            "unexpected output: {}",
            result.output
        );
    }

    /// Test 5: no scenario above produces a `PaladinError` -- every one
    /// returns `Ok(PaladinResult)`.
    #[tokio::test]
    async fn tool_call_limit_never_fails_the_run() {
        let llm = Arc::new(MockLlmAdapter::new().with_script(vec![
            MockScriptEntry::ToolCall {
                name: "search".to_string(),
                arguments: "{}".to_string(),
            },
            MockScriptEntry::Text("done".to_string()),
        ]));
        let middleware = Arc::new(ToolCallLimit::new(ToolCallLimitConfig {
            enabled: true,
            max_calls: 0,
            per_tool: HashMap::new(),
        }));
        let service = make_service(llm).with_middleware(middleware);
        let paladin = make_paladin(2);

        assert!(service.execute(&paladin, "hi").await.is_ok());
    }

    /// Test 6: two sequential runs each get a fresh per-tool map -- a tool
    /// exhausted in run 1 is available again in run 2.
    #[tokio::test]
    async fn per_tool_counters_are_per_run() {
        let llm = Arc::new(MockLlmAdapter::new().with_script(vec![
            MockScriptEntry::ToolCall {
                name: "search".to_string(),
                arguments: "{}".to_string(),
            },
            MockScriptEntry::Text("done".to_string()),
        ]));
        let arsenal = Arc::new(RecordingArsenal::default());
        let middleware = Arc::new(ToolCallLimit::new(ToolCallLimitConfig {
            enabled: true,
            max_calls: 1,
            per_tool: HashMap::new(),
        }));
        let service =
            make_service_with_arsenal(llm.clone(), arsenal.clone() as Arc<dyn ArsenalPort>)
                .with_middleware(middleware);
        let paladin = make_paladin(2);

        service.execute(&paladin, "hi").await.unwrap();
        llm.reset();
        service.execute(&paladin, "hi").await.unwrap();

        // Both runs' single `search` call reached the Arsenal -- run 2 did
        // not inherit run 1's exhausted count.
        assert_eq!(arsenal.calls.lock().unwrap().len(), 2);
    }
}
