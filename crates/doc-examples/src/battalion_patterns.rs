//! Examples for `docs/src/user-guides/battalion-patterns.md` (Phase 35, MB-20).
//!
//! Every `// ANCHOR:` region below is pulled into the Battalion Patterns user guide
//! via mdBook `{{#include}}`, so a sample in the guide cannot drift from the landed
//! API: `cargo check -p paladin-doc-examples` compiles all of them.
#![allow(unused_variables, unused_imports, dead_code)]

use crate::support::{create_paladin, mock_paladin_port};

// ANCHOR: commander
use paladin_battalion::commander::CommanderBuilder;
use paladin_core::platform::container::battalion::BattalionStrategy;

/// Build a Commander through `CommanderBuilder` (the builder `orchestration.md` also
/// uses) and run it with the live single-argument `execute` method. The direct
/// `Commander::new` constructor takes five positional arguments (strategy, paladins,
/// config, aggregator, paladin_port) — there is no two-argument form, and `execute`
/// takes only the input string, never a separate strategy/config pair per call.
pub async fn run_commander() -> Result<(), Box<dyn std::error::Error>> {
    let paladin_port = mock_paladin_port();

    let commander = CommanderBuilder::new(paladin_port)
        .strategy(BattalionStrategy::Auto)
        .paladins(vec![
            create_paladin("Analyzer"),
            create_paladin("Processor"),
            create_paladin("Synthesizer"),
        ])
        .build()?;

    let result = commander
        .execute("Analyze and summarize this report")
        .await?;

    println!("Strategy selected: {:?}", result.strategy_used);
    if let Some(reason) = &result.strategy_selection_reasoning {
        println!("Reasoning: {reason}");
    }
    println!("Output: {}", result.final_output);
    Ok(())
}
// ANCHOR_END: commander
