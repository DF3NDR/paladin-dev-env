// examples/agent_runtime_middleware.rs
//
// Agent Runtime: Execution Middleware
//
// This example demonstrates the Phase 26 agent-runtime middleware seam -- the
// `ExecutionMiddleware` hook chain that runs inside `PaladinExecutionService`'s
// reasoning loop, plus the runtime's built-in configuration, token accounting,
// memory namespacing and tool-error policy layered on top of it. It shows:
//
// 1. A custom `ExecutionMiddleware` whose `before_model`/`after_model`/`around_tool`
//    hooks each print that they fired, attached to a Paladin and executed once.
// 2. `AgentRuntimeConfig::build_chain` resolving several built-in middleware
//    sub-structs from configuration alone, printing the resolved settings.
// 3. A custom `TokenCounterPort` injected via `with_token_counter`, contrasted
//    with the defaulted heuristic counter's `is_exact` answer.
// 4. `HistoryTrimmer` and `SummarizationMiddleware` reducing a long conversation
//    history, with the message count and token split printed before and after.
// 5. `ConfinedVault` namespacing two agents' memories apart over the same
//    underlying store, including the attempted (and denied) cross-namespace read.
// 6. The `fail-run` tool error mode failing a run with a structured
//    `PaladinError::ArmamentFailed` when a tool invocation fails.
//
// This example is fully offline: it uses `MockLlmAdapter` and in-memory ports, and
// reads no LLM provider API key from the environment -- no external service is
// needed.
//
// To run this example:
// ```bash
// cargo run --example agent_runtime_middleware
// ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::json;

use paladin::MockLlmAdapter;
use paladin::application::services::paladin::error::PaladinError;
use paladin::application::services::paladin::middleware::{
    HistoryTrimmer, SummarizationMiddleware,
};
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::config::agent_runtime::{
    AgentRuntimeConfig, AgentRuntimeDeps, HistoryTrimmerConfig, SummarizationConfig,
    ToolErrorConfig, ToolErrorMode,
};
use paladin::core::platform::container::arsenal::{
    Armament, ArmamentCall, ArmamentResult, ArsenalError,
};
use paladin::core::platform::container::garrison::{
    ConversationRole, GarrisonConfig, GarrisonEntry,
};
use paladin::infrastructure::adapters::garrison::InMemoryGarrison;
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin::prelude::{
    ConfinedVault, ExecutionMiddleware, LlmResponseView, MiddlewareFlow, ModelCallContext,
    PromptAssembly, ToolCallContext, ToolFlow,
};
use paladin_core::platform::container::vault::Namespace;
use paladin_llm::mock::MockScriptEntry;
use paladin_memory::token_counter::HeuristicTokenCounter;
use paladin_memory::vault::InMemoryVault;
use paladin_ports::output::arsenal_port::ArsenalPort;
use paladin_ports::output::garrison_port::GarrisonPort;
use paladin_ports::output::llm_port::LlmPort;
use paladin_ports::output::token_counter_port::TokenCounterPort;
use paladin_ports::output::vault_port::VaultPort;

/// Prints each `ExecutionMiddleware` hook as it fires, in the order the
/// reasoning loop calls them (EX-83).
struct RecordingMiddleware;

#[async_trait]
impl ExecutionMiddleware for RecordingMiddleware {
    async fn before_model(
        &self,
        cx: &mut ModelCallContext<'_>,
    ) -> Result<MiddlewareFlow, PaladinError> {
        println!(
            "     [1] before_model fired (loop_index = {})",
            cx.loop_index
        );
        Ok(MiddlewareFlow::Continue)
    }

    async fn after_model(
        &self,
        _cx: &mut ModelCallContext<'_>,
        resp: &mut LlmResponseView,
    ) -> Result<MiddlewareFlow, PaladinError> {
        println!(
            "     [2] after_model fired (finish_reason = {:?})",
            resp.finish_reason
        );
        Ok(MiddlewareFlow::Continue)
    }

    async fn around_tool(&self, cx: &mut ToolCallContext) -> Result<ToolFlow, PaladinError> {
        println!("     [3] around_tool fired (tool = {})", cx.call.tool_name);
        Ok(ToolFlow::Allow)
    }

    fn name(&self) -> &str {
        "recording-example-middleware"
    }
}

/// An `ArsenalPort` that always succeeds, so `around_tool` has a real
/// dispatch to observe.
struct EchoArsenal;

#[async_trait]
impl ArsenalPort for EchoArsenal {
    async fn list_armaments(&self) -> Vec<Armament> {
        Vec::new()
    }

    async fn invoke(&self, call: ArmamentCall) -> Result<ArmamentResult, ArsenalError> {
        Ok(ArmamentResult::success(call.call_id, json!("ok"), 0))
    }

    fn validate_call(&self, _call: &ArmamentCall) -> Result<(), ArsenalError> {
        Ok(())
    }
}

/// An `ArsenalPort` whose only tool always fails, for the fail-run demo (EX-89).
struct FailingArsenal;

#[async_trait]
impl ArsenalPort for FailingArsenal {
    async fn list_armaments(&self) -> Vec<Armament> {
        Vec::new()
    }

    async fn invoke(&self, _call: ArmamentCall) -> Result<ArmamentResult, ArsenalError> {
        Err(ArsenalError::TransportError(
            "upstream gateway rejected the request: Authorization: Bearer sk-live-demo0123456789"
                .to_string(),
        ))
    }

    fn validate_call(&self, _call: &ArmamentCall) -> Result<(), ArsenalError> {
        Ok(())
    }
}

/// A stand-in for an exact counter (e.g. a real BPE tokenizer): overrides
/// `is_exact` to `true` so its answer can be contrasted with the heuristic
/// default (EX-85).
#[derive(Debug, Default, Clone, Copy)]
struct ExactWordCounter;

impl TokenCounterPort for ExactWordCounter {
    fn count(&self, text: &str, _model: &str) -> u32 {
        u32::try_from(text.split_whitespace().count()).unwrap_or(u32::MAX)
    }

    fn name(&self) -> &str {
        "exact-word-demo"
    }

    fn is_exact(&self) -> bool {
        true
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Agent Runtime: Execution Middleware\n");

    // ------------------------------------------------------------------------------
    // Part 1 -- a custom ExecutionMiddleware's three hooks firing in order (EX-83).
    // ------------------------------------------------------------------------------
    println!("1. A custom ExecutionMiddleware -- before_model / after_model / around_tool\n");

    let recording_llm =
        Arc::new(
            MockLlmAdapter::new().with_script(vec![MockScriptEntry::ToolCall {
                name: "lookup".to_string(),
                arguments: "{}".to_string(),
            }]),
        );
    let echo_arsenal: Arc<dyn ArsenalPort> = Arc::new(EchoArsenal);
    let recording_service = PaladinExecutionService::new(
        recording_llm.clone(),
        Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(30))),
        None,
        Some(echo_arsenal),
    )
    .with_middleware(Arc::new(RecordingMiddleware));

    let recording_paladin = PaladinBuilder::new(recording_llm.clone() as Arc<dyn LlmPort>)
        .system_prompt("You are a helpful assistant")
        .name("MiddlewareDemo")
        .max_loops(1)
        .build()
        .await?;

    let recording_result = recording_service
        .execute(&recording_paladin, "please look something up")
        .await?;
    println!(
        "   Run completed -- stop_reason = {:?}\n",
        recording_result.stop_reason
    );

    // ------------------------------------------------------------------------------
    // Part 2 -- AgentRuntimeConfig::build_chain resolving configuration, not
    // hand-wiring (EX-84).
    // ------------------------------------------------------------------------------
    println!("2. AgentRuntimeConfig -- the built-in middleware's single config home\n");

    let mut runtime_config = AgentRuntimeConfig::default();
    runtime_config.model_call_limit.enabled = true;
    runtime_config.model_call_limit.max_calls = 5;
    runtime_config.token_budget.enabled = true;
    runtime_config.token_budget.max_tokens = 20_000;
    runtime_config.history_trimmer.enabled = true;
    runtime_config.history_trimmer.default_context_tokens = 4_096;

    let runtime_deps = AgentRuntimeDeps {
        llm_port: Some(recording_llm.clone() as Arc<dyn LlmPort>),
        ..AgentRuntimeDeps::default()
    };
    let built_chain = runtime_config.build_chain(&runtime_deps)?;
    println!(
        "   build_chain assembled {} middleware from configuration alone (not with_middleware calls)",
        built_chain.len()
    );
    println!(
        "   model_call_limit:  enabled={} max_calls={}",
        runtime_config.model_call_limit.enabled, runtime_config.model_call_limit.max_calls
    );
    println!(
        "   token_budget:      enabled={} max_tokens={}",
        runtime_config.token_budget.enabled, runtime_config.token_budget.max_tokens
    );
    println!(
        "   history_trimmer:   enabled={} default_context_tokens={}\n",
        runtime_config.history_trimmer.enabled,
        runtime_config.history_trimmer.default_context_tokens
    );

    // ------------------------------------------------------------------------------
    // Part 3 -- a custom TokenCounterPort vs. the defaulted heuristic (EX-85).
    // ------------------------------------------------------------------------------
    println!("3. A custom TokenCounterPort vs. the defaulted heuristic counter\n");

    let counter_llm = Arc::new(MockLlmAdapter::new().with_response("ok"));
    let counter_service = PaladinExecutionService::new(
        counter_llm.clone(),
        Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(30))),
        None,
        None,
    );
    println!(
        "   defaulted counter -- name={}, is_exact={}",
        counter_service.token_counter().name(),
        counter_service.token_counter().is_exact()
    );
    let counter_service = counter_service.with_token_counter(Arc::new(ExactWordCounter));
    println!(
        "   injected counter   -- name={}, is_exact={}",
        counter_service.token_counter().name(),
        counter_service.token_counter().is_exact()
    );
    let sample = "the quick brown fox jumps over the lazy dog";
    println!(
        "   heuristic count(\"{sample}\")   = {}",
        HeuristicTokenCounter.count(sample, "demo-model")
    );
    println!(
        "   exact-word count(\"{sample}\") = {}\n",
        ExactWordCounter.count(sample, "demo-model")
    );

    // ------------------------------------------------------------------------------
    // Part 4 -- context-window management: HistoryTrimmer + SummarizationMiddleware
    // reducing a long history (EX-86).
    // ------------------------------------------------------------------------------
    println!("4. Context-window management: HistoryTrimmer + SummarizationMiddleware\n");

    let mut long_history = Vec::new();
    for i in 0..40 {
        long_history.push(GarrisonEntry::new(
            ConversationRole::User,
            format!(
                "Question {i}: explain another facet of Rust ownership, borrowing and lifetimes \
                 in enough detail to be a realistic conversational turn."
            ),
        ));
        long_history.push(GarrisonEntry::new(
            ConversationRole::Assistant,
            format!(
                "Answer {i}: a reasonably detailed explanation of that facet, long enough to \
                 accumulate real token weight across many turns."
            ),
        ));
    }

    let counter: Arc<dyn TokenCounterPort> = Arc::new(HeuristicTokenCounter);
    let before_count = long_history.len();
    let before_tokens: u32 = long_history
        .iter()
        .map(|e| counter.count(&e.content, "demo-model"))
        .sum();
    println!(
        "   before: {before_count} history entries, {before_tokens} tokens (heuristic count, prompt-side)"
    );

    let summarizer_llm = Arc::new(MockLlmAdapter::new().with_response(
        "Summary: the conversation covered many facets of Rust ownership, borrowing and \
         lifetimes across many prior turns.",
    ));
    let summarization_garrison: Arc<dyn GarrisonPort> =
        Arc::new(InMemoryGarrison::new(GarrisonConfig::new(200, None)));

    let trimmer_config = HistoryTrimmerConfig {
        enabled: true,
        reserve_for_response: 256,
        default_context_tokens: 2_048,
        model_context_limits: HashMap::new(),
        recall_limit: 40,
    };
    let summarization_config = SummarizationConfig {
        enabled: true,
        threshold_tokens: None,
        threshold_messages: 30,
        keep_recent: 10,
        summarizer_model: None,
    };
    let keep_recent = summarization_config.keep_recent;

    let summarizer = SummarizationMiddleware::new(
        summarization_config,
        trimmer_config,
        counter.clone(),
        summarizer_llm.clone() as Arc<dyn LlmPort>,
        None,
        summarization_garrison,
    );

    let context_paladin = PaladinBuilder::new(summarizer_llm.clone() as Arc<dyn LlmPort>)
        .system_prompt("system")
        .model("demo-model")
        .build()
        .await?;
    let assembly = PromptAssembly::new("system", "current input", "", long_history, None);
    let mut model_cx = ModelCallContext::new(uuid::Uuid::new_v4(), &context_paladin, assembly);

    summarizer.before_model(&mut model_cx).await?;

    let after_count = model_cx.assembly.history.len();
    let after_tokens: u32 = model_cx
        .assembly
        .history
        .iter()
        .map(|e| counter.count(&e.content, "demo-model"))
        .sum();
    println!(
        "   after:  {after_count} history entries, {after_tokens} tokens (heuristic count) -- \
         folded into 1 Garrison summary + {keep_recent} kept recent\n"
    );

    // Also demonstrate the standalone HistoryTrimmer directly (no summarization),
    // reusing the same before/after shape on a fresh copy of the history.
    let mut second_history = Vec::new();
    for i in 0..40 {
        second_history.push(GarrisonEntry::new(
            ConversationRole::User,
            format!("Turn {i} user message, long enough to matter for trimming."),
        ));
        second_history.push(GarrisonEntry::new(
            ConversationRole::Assistant,
            format!("Turn {i} assistant reply, similarly long."),
        ));
    }
    // `model_context_limits` names the Paladin's own model explicitly: the
    // resolver tries this table BEFORE the provider's own declared
    // capabilities (`MockLlmAdapter::get_capabilities` reports 4096 tokens,
    // which would otherwise comfortably fit this whole history unTrimmed).
    let trimmer_only_config = HistoryTrimmerConfig {
        enabled: true,
        reserve_for_response: 64,
        default_context_tokens: 256,
        model_context_limits: HashMap::from([("demo-model".to_string(), 256)]),
        recall_limit: 80,
    };
    let trimmer = HistoryTrimmer::new(
        trimmer_only_config,
        counter.clone(),
        summarizer_llm.clone() as Arc<dyn LlmPort>,
    );
    let trim_before = second_history.len();
    let trim_assembly = PromptAssembly::new("system", "current input", "", second_history, None);
    let mut trim_cx = ModelCallContext::new(uuid::Uuid::new_v4(), &context_paladin, trim_assembly);
    trimmer.before_model(&mut trim_cx).await?;
    let trim_after = trim_cx.assembly.history.len();
    println!(
        "   HistoryTrimmer alone (tight 256-token budget): {trim_before} entries -> {trim_after} entries\n"
    );

    // ------------------------------------------------------------------------------
    // Part 5 -- ConfinedVault namespacing two agents' memories apart (EX-87).
    // ------------------------------------------------------------------------------
    println!("5. ConfinedVault -- structural memory namespacing\n");

    let inner_vault: Arc<dyn VaultPort> = Arc::new(InMemoryVault::new());
    let alice_ns = Namespace::parse("user/alice")?;
    let bob_ns = Namespace::parse("user/bob")?;
    let alice_vault = ConfinedVault::new(inner_vault.clone(), alice_ns.clone());
    let bob_vault = ConfinedVault::new(inner_vault.clone(), bob_ns.clone());

    alice_vault
        .put(&alice_ns, "favorite_color", json!("blue"))
        .await?;
    bob_vault
        .put(&bob_ns, "favorite_color", json!("green"))
        .await?;

    let alice_own_read = alice_vault.get(&alice_ns, "favorite_color").await?;
    println!(
        "   alice reads her own namespace -- favorite_color = {:?}",
        alice_own_read.map(|r| r.value().clone())
    );

    match alice_vault.get(&bob_ns, "favorite_color").await {
        Ok(value) => println!("   UNEXPECTED: alice's cross-namespace read succeeded: {value:?}"),
        Err(err) => {
            println!("   alice attempts to read bob's namespace -- DENIED: {err}\n");
        }
    }

    // ------------------------------------------------------------------------------
    // Part 6 -- fail-run tool error mode (EX-89).
    // ------------------------------------------------------------------------------
    println!("6. Fail-run tool error mode\n");

    let fail_llm = Arc::new(
        MockLlmAdapter::new().with_script(vec![MockScriptEntry::ToolCall {
            name: "lookup".to_string(),
            arguments: "{}".to_string(),
        }]),
    );
    let failing_arsenal: Arc<dyn ArsenalPort> = Arc::new(FailingArsenal);
    let fail_service = PaladinExecutionService::new(
        fail_llm.clone(),
        Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(30))),
        None,
        Some(failing_arsenal),
    )
    .with_tool_error_config(ToolErrorConfig {
        mode: ToolErrorMode::FailRun,
        per_tool: HashMap::new(),
    });

    let fail_paladin = PaladinBuilder::new(fail_llm.clone() as Arc<dyn LlmPort>)
        .system_prompt("system")
        .max_loops(1)
        .build()
        .await?;

    match fail_service
        .execute(&fail_paladin, "look something up")
        .await
    {
        Ok(result) => println!("   UNEXPECTED: run succeeded -- {:?}", result.output),
        Err(PaladinError::ArmamentFailed { tool, reason }) => {
            println!("   Run FAILED under the fail-run policy -- tool = {tool}");
            println!(
                "   NOTE: this reason is intentionally NOT redacted -- see the defect note below"
            );
            println!("   reason = {reason}");
            println!(
                "\n   NOTE (discovered during this example's own read_first research):\n   \
                 `PaladinError::ArmamentFailed`'s own doc comment states `reason` should be \"a\n   \
                 redacted, human-readable summary\" -- the same sanitized text\n   \
                 `ToolResultFormatter::format_error` produces under the `FeedToModel` mode\n   \
                 (proven by that mode's own `a_secret_in_a_tool_error_never_reaches_the_model`\n   \
                 test). The current implementation's fail-run arms (both the Armament and the\n   \
                 handoff branch in `paladin_execution_service.rs`) build `reason` from the raw\n   \
                 `e.to_string()` directly, bypassing that sanitizer -- so a credential-shaped\n   \
                 string embedded in a failing tool's error text is NOT redacted here, unlike the\n   \
                 `FeedToModel` path. Recorded as a discovered library defect and left unfixed:\n   \
                 this is a docs-only phase and library source under src/ or crates/ is out of\n   \
                 scope (D-18 boundary)."
            );
        }
        Err(other) => println!("   UNEXPECTED error variant: {other:?}"),
    }

    println!("\nDone -- fully offline, no provider API key was read.");
    Ok(())
}
