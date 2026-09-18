// examples/token_economy_commissary.rs
//
// Token Economy: Commissary Prompt-Budgeting
//
// This example demonstrates the Commissary -- the input-side, per-call
// window-rationing officer that measures an assembled prompt against a provider's OWN
// declared context window and dispenses a bounded, priority-ordered stockpile of
// material within it. It shows how to:
// 1. Construct a `Commissary` through the `paladin::` facade re-export, using the
//    post-PRIM-02 `Commissary::new` signature (no `is_exact_counter` argument --
//    exactness is read live from the injected `TokenCounterPort`).
// 2. Compare `TokenCounterPort::is_exact` for a defaulted (approximate) counter
//    against an exact counter, so the defaulted behaviour is visible.
// 3. Resolve a context window through `resolve_context_window`, printing the
//    `ResolvedWindow` together with the `WindowSource` and `WindowFallbackPolicy`
//    that produced it.
// 4. Read an Anthropic-shaped `TokenUsage` back off a `MockLlmAdapter` response,
//    noting that Anthropic's `prompt_tokens` figure includes cache-read and
//    cache-write tokens.
//
// This example is fully offline: it uses `MockLlmAdapter` and needs no
// OPENAI_API_KEY, ANTHROPIC_API_KEY or DEEPSEEK_API_KEY -- and no external service.
//
// To run this example:
// ```bash
// cargo run --example token_economy_commissary
// ```

use paladin::{
    Commissary, CommissaryPlan, Consignment, ConsignmentItem, MockLlmAdapter, ResolvedWindow,
    WindowFallbackPolicy, WindowSource, resolve_context_window,
};
use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
use paladin_core::platform::container::token_usage::TokenUsage;
use paladin_ports::output::llm_port::{LlmPort, LlmRequest, ProviderCapabilities};
use paladin_ports::output::token_counter_port::TokenCounterPort;
use std::sync::Arc;

/// A stand-in for an exact, BPE-based adapter (e.g. `garrison::TiktokenCounter` under
/// the `content-processing` feature): overrides `is_exact` to `true` so its answer can
/// be contrasted with the heuristic default below.
#[derive(Debug, Default, Clone, Copy)]
struct ExactCounter;

impl TokenCounterPort for ExactCounter {
    fn count(&self, text: &str, _model: &str) -> u32 {
        // A whitespace-split word count -- not a real tokenizer, but deterministic and
        // dependency-free, which is all this example needs to demonstrate the `is_exact`
        // contract.
        u32::try_from(text.split_whitespace().count()).unwrap_or(u32::MAX)
    }

    fn name(&self) -> &str {
        "exact-demo"
    }

    fn is_exact(&self) -> bool {
        true
    }
}

/// The phase-wide default counter (`paladin_memory::token_counter::HeuristicTokenCounter`'s
/// own algorithm, reproduced here so this example needs no `paladin-memory` dependency):
/// `text.chars().count() / 4`, rounded up. Leaves `is_exact` at the `TokenCounterPort`
/// trait default (`false`).
#[derive(Debug, Default, Clone, Copy)]
struct HeuristicCounter;

impl TokenCounterPort for HeuristicCounter {
    fn count(&self, text: &str, _model: &str) -> u32 {
        (text.chars().count() as u32).div_ceil(4)
    }

    fn name(&self) -> &str {
        "heuristic-demo"
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Token Economy: Commissary Prompt-Budgeting\n");

    // ------------------------------------------------------------------------------
    // Part 1 -- `is_exact`: the defaulted (approximate) counter vs. an exact counter.
    // ------------------------------------------------------------------------------
    println!("1. TokenCounterPort::is_exact\n");

    let heuristic = HeuristicCounter;
    let exact = ExactCounter;
    println!(
        "   {} counter -- is_exact() = {}",
        heuristic.name(),
        heuristic.is_exact()
    );
    println!(
        "   {} counter    -- is_exact() = {}\n",
        exact.name(),
        exact.is_exact()
    );

    // ------------------------------------------------------------------------------
    // Part 2 -- `resolve_context_window`: the shared precedence resolver.
    // ------------------------------------------------------------------------------
    println!("2. resolve_context_window\n");

    let capabilities = ProviderCapabilities {
        max_context_tokens: Some(8_192),
        ..ProviderCapabilities::default()
    };
    let ResolvedWindow { tokens, source } = resolve_context_window(
        "demo-model",
        None,
        &capabilities,
        WindowFallbackPolicy::Strict {
            caller_fallback: None,
        },
    )?;
    println!("   Resolved window: {tokens} tokens");
    println!("   WindowSource:    {source:?} ({})", source.as_str());
    assert_eq!(source, WindowSource::ProviderCapabilities);
    println!(
        "   (produced by WindowFallbackPolicy::Strict, since the provider's own\n    \
         capabilities already declared a window -- no fallback was needed)\n"
    );

    // ------------------------------------------------------------------------------
    // Part 3 -- `Commissary`: construct, dispense, read the Stockpile back.
    // ------------------------------------------------------------------------------
    println!("3. Commissary::new + Commissary::dispense\n");

    let commissary = Commissary::new(
        "demo-provider",
        capabilities,
        Arc::new(HeuristicCounter) as Arc<dyn TokenCounterPort>,
        CommissaryPlan {
            reserved_completion_tokens: 512,
            model_hint: "demo-model".to_string(),
            ..CommissaryPlan::default()
        },
    )?;
    println!("   Commissary constructed for provider \"demo-provider\", window {tokens} tokens");

    let mut consignment = Consignment::new();
    consignment.push(ConsignmentItem {
        label: "system-instructions".to_string(),
        body: "You are a helpful assistant.".to_string(),
        priority: 0, // lowest number == highest priority == shed last
    });
    consignment.push(ConsignmentItem {
        label: "conversation-history".to_string(),
        body: "User: What is the capital of France?\nAssistant: Paris.".to_string(),
        priority: 1,
    });

    let stockpile = commissary.dispense("Fixed system material. ", &consignment)?;
    println!(
        "   Dispensed {} item(s), shed {} item(s)",
        stockpile.dispensed.len(),
        stockpile.shed.len()
    );
    println!(
        "   Stockpile.prompt_tokens = {}, allotted_tokens = {}, exact_tally = {}\n",
        stockpile.prompt_tokens, stockpile.allotted_tokens, stockpile.exact_tally
    );

    // ------------------------------------------------------------------------------
    // Part 4 -- an Anthropic-shaped TokenUsage, produced through MockLlmAdapter.
    // ------------------------------------------------------------------------------
    println!("4. Anthropic-shaped TokenUsage via MockLlmAdapter (offline)\n");

    // Anthropic's own `prompt_tokens` figure INCLUDES cache-read and cache-write
    // tokens -- it is not a separate, additional count on top of `prompt_tokens`.
    // `cache_read_tokens + cache_write_tokens <= prompt_tokens` always holds (see
    // `TokenUsage`'s own doc comment).
    let anthropic_shaped_usage = TokenUsage::new(1_000, 250)
        .with_cache_read(300)
        .with_cache_write(100);

    let llm = MockLlmAdapter::new()
        .with_response("Paris is the capital of France.")
        .with_token_usage_struct(anthropic_shaped_usage);

    let prompt = PromptItem::new(PromptType::User(UserPrompt {
        query: "What is the capital of France?".to_string(),
        context: None,
    }))?;
    let response = llm.generate(LlmRequest::new("claude-demo", prompt)).await?;

    println!(
        "   response.usage.prompt_tokens     = {}",
        response.usage.prompt_tokens
    );
    println!(
        "   response.usage.cache_read_tokens  = {:?}",
        response.usage.cache_read_tokens
    );
    println!(
        "   response.usage.cache_write_tokens = {:?}",
        response.usage.cache_write_tokens
    );
    println!(
        "   response.usage.completion_tokens  = {}",
        response.usage.completion_tokens
    );
    println!(
        "   (prompt_tokens already includes the {} cache-read and {} cache-write\n    \
         tokens above -- never add them again on top of prompt_tokens)\n",
        response.usage.cache_read_tokens.unwrap_or(0),
        response.usage.cache_write_tokens.unwrap_or(0)
    );

    println!("Done -- fully offline, no provider API key was read.");
    Ok(())
}
