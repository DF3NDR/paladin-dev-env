//! Examples for `docs/src/user-guides/paladin-agents.md` (Phase 35, MB-27).
//!
//! Every `// ANCHOR:` region below is pulled into the Paladin Agents user guide via
//! mdBook `{{#include}}`, so a sample in the guide cannot drift from the landed API:
//! `cargo check -p paladin-doc-examples` compiles all of them.
#![allow(unused_variables, unused_imports, dead_code)]

use std::sync::Arc;

use paladin::MockLlmAdapter;
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin_core::platform::container::paladin::Paladin;
use paladin_ports::output::llm_port::LlmPort;

// ANCHOR: build_agent
/// Build a Paladin through `PaladinBuilder` against a mock LLM adapter — the same
/// fluent chain a real `LlmPort` adapter (e.g. OpenAI) is built the same way.
pub async fn build_agent() -> Result<Paladin, Box<dyn std::error::Error>> {
    let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new());

    let paladin = PaladinBuilder::new(llm_port)
        .system_prompt("You are a helpful assistant.")
        .name("Assistant")
        .model("gpt-4o")
        .temperature(0.7)
        .max_loops(3)
        .timeout_seconds(120)
        .build()
        .await?;

    Ok(paladin)
}
// ANCHOR_END: build_agent

// ANCHOR: attach_garrison
use paladin_core::platform::container::garrison::GarrisonConfig;
use paladin_memory::garrison::in_memory_garrison::InMemoryGarrison;
use paladin_ports::output::garrison_port::GarrisonPort;

/// Attach an `InMemoryGarrison` (the one-argument `new(config)` constructor — there is
/// no zero-argument form) for conversation memory, and register specialist agents for
/// delegation via `with_handoffs`, which takes the whole `Vec<Arc<Paladin>>` at once —
/// there is no per-call chainable `with_specialist` method.
pub async fn attach_garrison() -> Result<Paladin, Box<dyn std::error::Error>> {
    let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new());

    let garrison_config = GarrisonConfig::default();
    let garrison: Arc<dyn GarrisonPort> = Arc::new(InMemoryGarrison::new(garrison_config));

    let code_reviewer = PaladinBuilder::new(llm_port.clone())
        .system_prompt("You review Rust code for correctness and style.")
        .name("CodeReviewer")
        .build()
        .await?;
    let security_auditor = PaladinBuilder::new(llm_port.clone())
        .system_prompt("You audit code for security vulnerabilities.")
        .name("SecurityAuditor")
        .build()
        .await?;

    let paladin = PaladinBuilder::new(llm_port)
        .system_prompt("You are a memory-enabled coordinator.")
        .with_garrison(garrison)
        .with_handoffs(vec![Arc::new(code_reviewer), Arc::new(security_auditor)])
        .build()
        .await?;

    Ok(paladin)
}
// ANCHOR_END: attach_garrison
