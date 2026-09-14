//! Presets: one-liner compositions over the pieces Phase 26 built and
//! tested in isolation (Doc 05 RT-07, RT-FR-23/24, D-35, D-36).
//!
//! [`reasoning_agent`] is the phase's "the value here is that a caller does
//! not have to know that" moment: it composes [`PaladinBuilder`]-shaped
//! construction, [`PaladinExecutionService`]'s tool-loop middleware set, and
//! [`StructuredExecutorExt`] into one call that returns a runnable,
//! tool-using agent with documented defaults.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use schemars::JsonSchema;
use serde::de::DeserializeOwned;

use crate::application::services::paladin::error::PaladinError;
use crate::application::services::paladin::middleware::{
    FinishOnPlainAnswerMiddleware, ToolCallLimit, ToolCallProtocolMiddleware,
};
use crate::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use crate::application::services::paladin::structured::StructuredExecutorExt;
use crate::config::agent_runtime::{ToolCallLimitConfig, ToolErrorConfig};
use crate::core::base::entity::node::Node;
use crate::core::platform::container::paladin::{MaxLoops, Paladin, PaladinData};
use crate::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin_ports::output::arsenal_port::ArsenalPort;
use paladin_ports::output::garrison_port::GarrisonPort;
use paladin_ports::output::llm_port::LlmPort;
use paladin_ports::output::paladin_port::PaladinResult;
use paladin_ports::output::structured_executor_port::Structured;

/// The documented tool-use system prompt [`ReasoningAgentOptions::default`]
/// installs. Written once, here, so the doc test on [`reasoning_agent`] and
/// [`ReasoningAgentOptions::default`]'s own doc test can both point at the
/// same literal.
const DEFAULT_SYSTEM_PROMPT: &str = "You are a helpful assistant with access to tools. When a \
    tool would help answer the user's request, call it using the format described in the Tools \
    section of this prompt. Once you have the information you need, answer the user directly, \
    in plain text, without requesting another tool.";

/// The README figures [`ReasoningAgentOptions::default`]'s circuit breaker
/// is built from (D-35): 3 consecutive failures open the circuit, 2
/// consecutive half-open successes close it again, and it waits 30 seconds
/// before probing recovery.
const DEFAULT_CIRCUIT_BREAKER_FAILURE_THRESHOLD: u32 = 3;
const DEFAULT_CIRCUIT_BREAKER_SUCCESS_THRESHOLD: u32 = 2;
const DEFAULT_CIRCUIT_BREAKER_TIMEOUT: Duration = Duration::from_secs(30);

/// The options [`reasoning_agent`] builds a [`ReasoningAgent`] from --
/// `Default` plus a fluent builder (D-35).
///
/// | Field | Default |
/// |---|---|
/// | `system_prompt` | [`DEFAULT_SYSTEM_PROMPT`] -- a documented tool-use prompt |
/// | `model` | `None` -- the built [`Paladin`]'s own default model |
/// | `max_loops` | `5` |
/// | `max_tool_calls` | `20` |
/// | `garrison` | `None` -- no conversation memory |
/// | `tool_errors` | [`ToolErrorConfig::default`] (`FeedToModel`, today's v0.9 behavior) |
/// | `circuit_breaker` | `Some(CircuitBreaker::new(3, 2, 30s))` -- the README figures |
pub struct ReasoningAgentOptions {
    system_prompt: String,
    model: Option<String>,
    max_loops: u32,
    max_tool_calls: u32,
    garrison: Option<Arc<dyn GarrisonPort>>,
    tool_errors: ToolErrorConfig,
    circuit_breaker: Option<Arc<CircuitBreaker>>,
}

impl Default for ReasoningAgentOptions {
    fn default() -> Self {
        Self {
            system_prompt: DEFAULT_SYSTEM_PROMPT.to_string(),
            model: None,
            max_loops: 5,
            max_tool_calls: 20,
            garrison: None,
            tool_errors: ToolErrorConfig::default(),
            circuit_breaker: Some(Arc::new(CircuitBreaker::new(
                DEFAULT_CIRCUIT_BREAKER_FAILURE_THRESHOLD,
                DEFAULT_CIRCUIT_BREAKER_SUCCESS_THRESHOLD,
                DEFAULT_CIRCUIT_BREAKER_TIMEOUT,
            ))),
        }
    }
}

impl ReasoningAgentOptions {
    /// Overrides the documented default system prompt.
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = prompt.into();
        self
    }

    /// Sets the model identifier passed to the underlying [`Paladin`].
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Overrides the default of `5` reasoning-loop iterations.
    pub fn max_loops(mut self, max_loops: u32) -> Self {
        self.max_loops = max_loops;
        self
    }

    /// Overrides the default tool-call budget of `20` (enforced by a
    /// [`ToolCallLimit`] the preset always installs).
    pub fn max_tool_calls(mut self, max_tool_calls: u32) -> Self {
        self.max_tool_calls = max_tool_calls;
        self
    }

    /// Attaches a Garrison for conversation memory. `None` (the default)
    /// runs stateless.
    pub fn garrison(mut self, garrison: Arc<dyn GarrisonPort>) -> Self {
        self.garrison = Some(garrison);
        self
    }

    /// Overrides the default [`ToolErrorConfig`] (`FeedToModel`).
    pub fn tool_errors(mut self, tool_errors: ToolErrorConfig) -> Self {
        self.tool_errors = tool_errors;
        self
    }

    /// Overrides the default `CircuitBreaker::new(3, 2, 30s)`.
    pub fn circuit_breaker(mut self, circuit_breaker: Arc<CircuitBreaker>) -> Self {
        self.circuit_breaker = Some(circuit_breaker);
        self
    }
}

/// A runnable, tool-using agent pair: the [`Paladin`] definition and the
/// [`PaladinExecutionService`] configured to run it, returned by
/// [`reasoning_agent`] (D-35).
///
/// `Deref`s to neither field (D-35) -- [`Self::paladin`] and
/// [`Self::service`] reach them when a caller needs to.
pub struct ReasoningAgent {
    paladin: Paladin,
    service: PaladinExecutionService,
}

impl ReasoningAgent {
    /// The underlying [`Paladin`] definition [`reasoning_agent`] built.
    pub fn paladin(&self) -> &Paladin {
        &self.paladin
    }

    /// The underlying [`PaladinExecutionService`] [`reasoning_agent`]
    /// configured, tool-loop middleware and all.
    pub fn service(&self) -> &PaladinExecutionService {
        &self.service
    }

    /// Runs `input` through the tool loop to completion.
    pub async fn run(&self, input: &str) -> Result<PaladinResult, PaladinError> {
        self.service.execute(&self.paladin, input).await
    }

    /// Runs `input` through the tool loop, then the structured-output
    /// repair loop, deserializing the result into `T` -- a thin delegation
    /// to [`StructuredExecutorExt::execute_structured`] (D-27), not a
    /// second implementation.
    pub async fn run_structured<T>(&self, input: &str) -> Result<Structured<T>, PaladinError>
    where
        T: DeserializeOwned + JsonSchema + Send,
    {
        self.service
            .execute_structured::<T>(&self.paladin, input)
            .await
    }
}

/// Builds a runnable, tool-using [`ReasoningAgent`] from an LLM port, an
/// executable arsenal, and [`ReasoningAgentOptions`] -- the phase's
/// one-liner (Doc 05 RT-07, RT-FR-23, D-35).
///
/// # Deviates from PRD RT-FR-23's `tools: Vec<Armament>` (D-35)
///
/// The PRD's sketch signature accepts `tools: Vec<Armament>`. An
/// [`Armament`](paladin_core::platform::container::arsenal::Armament) is a
/// *definition* -- a name, description and JSON Schema, with no execution
/// behaviour of its own -- so a preset that accepted a list of definitions
/// could never actually run a tool; it cannot execute anything. This
/// preset therefore takes an EXECUTABLE `Arc<dyn ArsenalPort>` instead.
/// Callers build one with
/// [`InProcessArsenal`](crate::application::services::arsenal::in_process_arsenal::InProcessArsenal)
/// (in-process closures, no MCP server needed -- see [`reasoning_agent`]'s
/// own doc test below), an MCP-backed `ArsenalExecutionService`, or a
/// [`CompositeArsenalPort`](crate::application::services::arsenal::composite_arsenal::CompositeArsenalPort)
/// unioning several of either.
///
/// # The tool-loop middleware set this installs (D-35, D-36)
///
/// Every call installs, in this order:
///
/// 1. [`FinishOnPlainAnswerMiddleware`] -- so the run completes the moment
///    the model answers without requesting a tool, rather than always
///    running to `max_loops` (today's default for a plain conversational
///    agent, and the wrong shape for a tool-loop one).
/// 2. [`ToolCallProtocolMiddleware`] over `arsenal` -- renders `arsenal`'s
///    catalogue into the prompt and decodes the documented JSON envelope
///    from a shipped provider's plain-text reply, since no shipped adapter
///    populates `LlmResponse.function_call` natively (ADR-0042).
/// 3. A [`ToolCallLimit`] from `opts.max_tool_calls`.
///
/// Installation order matters here, not just installation: `after_model`
/// hooks run in reverse of their installed order (the onion shape), so
/// `ToolCallProtocolMiddleware` -- installed SECOND -- runs its
/// `after_model` FIRST and can synthesize `function_call` before
/// `FinishOnPlainAnswerMiddleware` -- installed FIRST, so it runs its
/// `after_model` SECOND -- reads it. Reversing this order would make every
/// run finish after one loop, before a synthesized tool call is ever seen.
///
/// `opts.tool_errors` is installed via
/// [`PaladinExecutionService::with_tool_error_config`] (default
/// `FeedToModel`, today's v0.9 behavior). The preset does **not** enable
/// the built-in `vault_get`/`vault_put` Armaments -- that remains a
/// separate opt-in that also depends on a run grant (D-21, D-22).
///
/// # Examples
///
/// ```
/// use paladin::presets::reasoning_agent;
/// use paladin::InProcessArsenal;
/// use paladin_core::platform::container::arsenal::Armament;
/// use paladin_llm::mock::MockLlmAdapter;
/// use paladin_ports::output::paladin_port::StopReason;
/// use serde_json::json;
/// use std::sync::Arc;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let llm = MockLlmAdapter::new().with_responses(vec![
///     r#"{"tool":"add","arguments":{"a":2,"b":2}}"#.to_string(),
///     "The answer is 4".to_string(),
/// ]);
/// let arsenal = InProcessArsenal::new().with_tool(
///     Armament {
///         name: "add".to_string(),
///         description: "Adds two integers".to_string(),
///         parameters: json!({
///             "type": "object",
///             "properties": {"a": {"type": "integer"}, "b": {"type": "integer"}},
///             "required": ["a", "b"]
///         }),
///         required_params: vec![],
///     },
///     |_args| async { Ok(json!({ "sum": 4 })) },
/// );
/// let agent = reasoning_agent(Arc::new(llm), Arc::new(arsenal), Default::default())?;
/// let result = agent.run("What is 2+2?").await?;
/// assert!(result.output.contains('4'));
/// assert_eq!(result.loop_count, 2);
/// assert_eq!(result.stop_reason, StopReason::Completed);
/// # Ok(())
/// # }
/// ```
///
/// An arsenal built with NO registered tools still runs -- no `## Tools`
/// section is rendered (the tool catalogue is empty) and the agent answers
/// in one loop, so the one-liner is not a trap for a caller who has no
/// tools yet (D-36):
///
/// ```
/// use paladin::presets::reasoning_agent;
/// use paladin::InProcessArsenal;
/// use paladin_llm::mock::MockLlmAdapter;
/// use paladin_ports::output::paladin_port::StopReason;
/// use std::sync::Arc;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let llm = Arc::new(MockLlmAdapter::new().with_response("Hello there!"));
/// let arsenal = Arc::new(InProcessArsenal::new());
/// let agent = reasoning_agent(llm, arsenal, Default::default())?;
/// let result = agent.run("hi").await?;
/// assert_eq!(result.output, "Hello there!");
/// assert_eq!(result.loop_count, 1);
/// assert_eq!(result.stop_reason, StopReason::Completed);
/// # Ok(())
/// # }
/// ```
pub fn reasoning_agent(
    llm: Arc<dyn LlmPort>,
    arsenal: Arc<dyn ArsenalPort>,
    opts: ReasoningAgentOptions,
) -> Result<ReasoningAgent, PaladinError> {
    if opts.system_prompt.trim().is_empty() {
        return Err(PaladinError::ConfigurationError(
            "reasoning_agent requires a non-empty system_prompt".to_string(),
        ));
    }

    let mut data = PaladinData {
        system_prompt: opts.system_prompt.clone(),
        max_loops: MaxLoops::Fixed(opts.max_loops),
        ..Default::default()
    };
    if let Some(model) = &opts.model {
        data.model = model.clone();
    }
    let paladin = Node::new(data, None);

    let circuit_breaker = opts.circuit_breaker.clone().unwrap_or_else(|| {
        Arc::new(CircuitBreaker::new(
            DEFAULT_CIRCUIT_BREAKER_FAILURE_THRESHOLD,
            DEFAULT_CIRCUIT_BREAKER_SUCCESS_THRESHOLD,
            DEFAULT_CIRCUIT_BREAKER_TIMEOUT,
        ))
    });

    let tool_call_limit_config = ToolCallLimitConfig {
        enabled: true,
        max_calls: opts.max_tool_calls,
        per_tool: HashMap::new(),
    };

    // Installation order is load-bearing -- see this function's own
    // rustdoc "The tool-loop middleware set" section for why
    // `FinishOnPlainAnswerMiddleware` must be installed BEFORE
    // `ToolCallProtocolMiddleware` for the onion-shaped `after_model` pass
    // to see a synthesized `function_call` before deciding to finish.
    let service = PaladinExecutionService::new(
        llm,
        circuit_breaker,
        opts.garrison.clone(),
        Some(arsenal.clone()),
    )
    .with_middleware(Arc::new(FinishOnPlainAnswerMiddleware::new()))
    .with_middleware(Arc::new(ToolCallProtocolMiddleware::new(arsenal)))
    .with_middleware(Arc::new(ToolCallLimit::new(tool_call_limit_config)))
    .with_tool_error_config(opts.tool_errors.clone());

    Ok(ReasoningAgent { paladin, service })
}

// The end-to-end tool-loop behaviors (Tests 1, 3-8 of this plan's own
// <behavior> list) live in `tests/integration/reasoning_agent_test.rs` --
// this plan's own verification commands run them with `cargo test --test
// integration <name>`, which only finds tests in the `tests/` binary, not
// this crate's own unit-test module. `defaults_match_the_documented_options`
// stays here because it asserts against `ReasoningAgentOptions`' PRIVATE
// fields, which only a same-module test can reach.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::agent_runtime::ToolErrorMode;

    /// Test 2: `ReasoningAgentOptions::default()` matches every documented
    /// figure.
    #[test]
    fn defaults_match_the_documented_options() {
        let opts = ReasoningAgentOptions::default();

        assert_eq!(opts.max_loops, 5);
        assert_eq!(opts.max_tool_calls, 20);
        assert_eq!(opts.tool_errors.mode, ToolErrorMode::FeedToModel);
        assert!(!opts.system_prompt.trim().is_empty());

        let breaker = opts
            .circuit_breaker
            .as_deref()
            .expect("a default circuit breaker must be present");
        assert_eq!(breaker.failure_threshold(), 3);
        assert_eq!(breaker.success_threshold(), 2);
        assert_eq!(breaker.timeout(), Duration::from_secs(30));
    }
}
