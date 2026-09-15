//! Commander Basic Example - Explicit Formation Strategy
//!
//! Demonstrates how to use the Commander with an explicit strategy selection.
//! This example shows the simplest use case: creating a Commander with Formation
//! strategy to process tasks sequentially.
//!
//! Run with: cargo run --example commander_basic

use async_trait::async_trait;
use paladin::application::services::battalion::commander::CommanderBuilder;
use paladin::application::services::paladin::error::PaladinError;
use paladin::core::base::entity::node::Node;
use paladin::core::platform::container::battalion::{BattalionConfig, BattalionStrategy};
use paladin::core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream, StopReason};
use std::sync::Arc;

/// Simple mock PaladinPort for demonstration
/// In production, this would connect to real LLM APIs
struct MockPaladinPort;

#[async_trait]
impl PaladinPort for MockPaladinPort {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        println!("\n🎯 {} executing...", paladin.node.name);
        println!("   📝 Input: {}", input);

        // Simulate processing delay
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        // Generate mock output based on Paladin role
        let output = format!("{} processed: {}", paladin.node.name, input);
        println!("   ✅ Output: {}", output);

        Ok(PaladinResult {
            output,
            usage: paladin_ports::output::llm_port::TokenUsage::new(50, 0),
            execution_time_ms: 300,
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
        unimplemented!("Streaming not needed for this example")
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

/// Helper function to create a Paladin with given name
fn create_paladin(name: &str) -> Paladin {
    let data = PaladinData {
        system_prompt: format!("You are {}, a specialized AI assistant.", name),
        name: name.to_string(),
        user_name: "User".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    Node::new(data, None)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎖️  Commander Basic Example - Formation Strategy\n");
    println!("{}", "=".repeat(60));

    // Step 1: Create the PaladinPort adapter
    // This is the abstraction that executes individual Paladins
    let paladin_port = Arc::new(MockPaladinPort);

    // Step 2: Create three Paladins for sequential processing
    // Each Paladin represents a stage in the pipeline
    let analyzer = create_paladin("Analyzer");
    let enhancer = create_paladin("Enhancer");
    let reviewer = create_paladin("Reviewer");

    println!("\n📋 Created 3 Paladins:");
    println!("   1. {} - First stage processing", analyzer.node.name);
    println!("   2. {} - Enhancement stage", enhancer.node.name);
    println!("   3. {} - Final review stage", reviewer.node.name);

    // Step 3: Create a Battalion configuration
    // This defines behavior like timeouts and error handling
    let config = BattalionConfig::new("basic_formation")
        .with_timeout(60) // 60 second timeout
        .with_error_strategy(
            paladin::core::platform::container::battalion::ErrorStrategy::FailFast,
        );

    println!("\n⚙️  Configuration:");
    println!("   Battalion: {}", config.name);
    println!("   Timeout: {} seconds", config.timeout_seconds);
    println!("   Strategy: Formation (sequential execution)");

    // Step 4: Build the Commander using the builder pattern
    // Commander acts as a high-level orchestrator for Battalion strategies
    let commander = CommanderBuilder::new(paladin_port)
        .strategy(BattalionStrategy::Formation) // Explicit strategy selection
        .paladins(vec![analyzer, enhancer, reviewer])
        .config(config)
        .build()?;

    println!("\n✅ Commander built successfully");

    // Step 5: Execute the Commander with input
    // The input will flow through all three Paladins sequentially
    println!("\n🚀 Executing Commander...\n");
    println!("{}", "=".repeat(60));

    let result = commander
        .execute("Analyze this product review: Great quality, fast shipping!")
        .await?;

    // Step 6: Display results
    println!();
    println!("{}", "=".repeat(60));
    println!("\n📊 Execution Results:");
    println!("   Strategy Used: {:?}", result.strategy_used);
    println!("   Status: {:?}", result.status);
    println!(
        "   Execution Time: {}ms",
        result
            .completed_at
            .signed_duration_since(result.started_at)
            .num_milliseconds()
    );
    println!("   Paladins Succeeded: {}", result.paladin_success_count);
    println!("   Paladins Failed: {}", result.paladin_failure_count);
    println!("\n📝 Final Output:\n   {}", result.final_output);

    println!();
    println!("{}", "=".repeat(60));
    println!("✅ Example completed successfully!");

    Ok(())
}
