// examples/citadel_autosave.rs
//
// Citadel Autosave Example
//
// This example demonstrates how to use the Citadel state persistence system with
// automatic saving enabled. It shows how to:
// 1. Create a Paladin with Citadel autosave enabled
// 2. Execute the Paladin and automatically save its state
// 3. Verify the saved state file exists
//
// The Citadel system saves Paladin state to JSON files, allowing you to:
// - Resume execution after interruptions
// - Debug agent behavior by inspecting saved states
// - Implement checkpoint-based recovery in production systems
//
// To run this example:
// ```bash
// cargo run --example citadel_autosave
// ```

use paladin::MockLlmAdapter;
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::infrastructure::adapters::citadel::file_citadel::FileCitadel;
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin_ports::output::llm_port::LlmPort;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🏰 Citadel Autosave Example\n");
    println!("Creating a Paladin with automatic state persistence...\n");

    // Step 1: Create a directory for state files
    let state_dir = "./example-states";
    println!("📁 State directory: {}", state_dir);
    println!();

    // Step 2: Create a FileCitadel for state persistence
    // FileCitadel automatically creates the directory if it doesn't exist
    let citadel: Arc<dyn paladin_ports::output::citadel_port::CitadelPort> =
        Arc::new(FileCitadel::new(state_dir)?);
    println!("✅ Citadel initialized at: {}", state_dir);
    println!();

    // Step 3: Create a mock LLM adapter for this example
    // In production, use a real LLM adapter (OpenAI, Anthropic, DeepSeek, etc.)
    let llm_port = Arc::new(
        MockLlmAdapter::new()
            .with_response(
                "I am analyzing the climate data. Based on the trends, I observe:\n\
                1. Global temperatures have increased by 1.1°C since pre-industrial times\n\
                2. Arctic ice is melting at an accelerating rate\n\
                3. Sea levels are rising approximately 3.3mm per year\n\
                \n\
                These indicators suggest significant climate change impacts that require urgent action."
                    .to_string(),
            ),
    );

    // Step 4: Build the Paladin with Citadel integration
    // The with_citadel() method enables state persistence
    // The enable_autosave() method turns on automatic saving after execution
    let paladin = PaladinBuilder::new(llm_port.clone() as Arc<dyn LlmPort>)
        .system_prompt(
            "You are a climate scientist analyzing environmental data. \
             Provide clear, evidence-based analysis of climate trends.",
        )
        .name("ClimateAnalyst")
        .model("gpt-4")
        .temperature(0.7)
        .max_loops(1)
        .with_citadel(citadel.clone()) // Enable Citadel
        .enable_autosave() // Enable automatic saving
        .build()
        .await?;

    println!("✅ Paladin created with autosave enabled!");
    println!("   Name: {}", paladin.node.name);
    println!("   ID: {}", paladin.uuid);
    println!("   Autosave: enabled");
    println!();

    // Step 5: Create execution service and run the Paladin
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));

    let service = PaladinExecutionService::new(llm_port, circuit_breaker, None, None);

    println!("🚀 Executing Paladin...\n");

    let input = "Analyze the latest climate change indicators and provide a summary.";
    println!("Input: {}", input);
    println!();

    let result = service.execute(&paladin, input).await?;

    // Step 6: Display execution results
    println!("📊 Execution Results:");
    println!("   Output length: {} characters", result.output.len());
    println!("   Loops: {}", result.loop_count);
    println!("   Tokens: {}", result.usage.total_tokens);
    println!();

    // Step 7: Verify the state was saved automatically
    // The autosave feature saves the state after successful execution
    let paladin_id = &paladin.uuid;
    println!("🔍 Verifying autosave...");

    // Check if the state file exists
    let saved_state = citadel
        .load_paladin(*paladin_id)
        .await?
        .expect("State should have been saved by autosave");
    println!("✅ State automatically saved!");
    println!("   File: paladin-{}.json", paladin_id);
    println!("   Status: {:?}", saved_state.paladin.node.status);
    println!("   Last updated: {}", saved_state.updated_at);
    println!();

    // Step 8: Show the benefits of autosave
    println!("💡 Autosave Benefits:");
    println!("   • State persisted without manual save() calls");
    println!("   • Can resume execution after crashes or interruptions");
    println!("   • Full execution history preserved for debugging");
    println!("   • Garrison (conversation memory) included in state");
    println!();

    println!(
        "✨ Example complete! State saved to: {}/paladin-{}.json",
        state_dir, paladin_id
    );

    Ok(())
}
