// examples/citadel_restore.rs
//
// Citadel State Restoration Example
//
// This example demonstrates how to restore a Paladin from previously saved state.
// It shows how to:
// 1. Create and execute a Paladin with state persistence
// 2. Save the Paladin's state to disk
// 3. Restore the Paladin from the saved state
// 4. Continue execution from the restored state
//
// State restoration is useful for:
// - Resuming long-running agent workflows after interruptions
// - Implementing fault-tolerant agent systems
// - Debugging by replaying saved states
// - Load balancing by migrating agents between servers
//
// To run this example:
// ```bash
// cargo run --example citadel_restore
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
    println!("🏰 Citadel State Restoration Example\n");
    println!("This example demonstrates saving and restoring Paladin state...\n");

    // Step 1: Setup - Create state directory and Citadel
    let state_dir = "./example-states";
    let citadel: Arc<dyn paladin_ports::output::citadel_port::CitadelPort> =
        Arc::new(FileCitadel::new(state_dir)?);
    println!("✅ Citadel initialized at: {}", state_dir);
    println!();

    // Step 2: Create the first LLM adapter and Paladin
    println!("=== PART 1: Initial Execution and Save ===\n");

    let llm_port = Arc::new(
        MockLlmAdapter::new().with_response(
            "Initial analysis: The system architecture follows hexagonal patterns \
                with clear separation between core domain, application ports, and \
                infrastructure adapters. Key strengths include strong type safety and \
                comprehensive error handling."
                .to_string(),
        ),
    );

    // Create a Paladin and execute it
    let paladin = PaladinBuilder::new(llm_port.clone() as Arc<dyn LlmPort>)
        .system_prompt(
            "You are a software architect reviewing system designs. \
             Provide detailed technical analysis.",
        )
        .name("ArchitectReviewer")
        .model("gpt-4")
        .temperature(0.7)
        .max_loops(1)
        .with_citadel(citadel.clone())
        .enable_autosave() // Enable automatic state saving
        .build()
        .await?;

    let paladin_id = paladin.uuid;

    println!("✅ Paladin created:");
    println!("   Name: {}", paladin.data.name);
    println!("   ID: {}", paladin_id);
    println!("   Autosave: enabled");
    println!();

    // Execute the Paladin
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service =
        PaladinExecutionService::new(llm_port.clone(), circuit_breaker.clone(), None, None);

    println!("🚀 Executing initial analysis...\n");
    let result = service
        .execute(
            &paladin,
            "Review the Paladin multi-agent framework architecture",
        )
        .await?;

    println!("📊 Initial Execution Results:");
    println!("   Output length: {} characters", result.output.len());
    println!("   Loops: {}", result.loop_count);
    println!();

    // Step 3: The Paladin's state is automatically saved after execution
    // because we enabled autosave in the builder above
    println!("💾 Paladin will autosave state after execution");
    println!();

    // Simulate a system restart or interruption
    println!("⏸️  Simulating system restart...");
    println!("   (In production, this could be a crash, deployment, or scaling event)");
    println!();

    // Step 4: Restore the Paladin from saved state
    println!("=== PART 2: State Restoration ===\n");
    println!("🔄 Restoring Paladin from saved state...");

    // Create a new LLM adapter for the restored session
    let llm_port_restored = Arc::new(
        MockLlmAdapter::new()
            .with_response(
                "Follow-up analysis: The framework demonstrates excellent modularity. \
                The Battalion orchestration patterns (Formation, Phalanx, Campaign, Chain of Command) \
                provide flexible agent coordination. The Citadel persistence layer ensures \
                fault tolerance and enables long-running workflows."
                    .to_string(),
            ),
    );

    // Restore using PaladinBuilder's restore_from method
    // Note: with_citadel must be called first to configure the Citadel port
    let restored_paladin = PaladinBuilder::new(llm_port_restored.clone() as Arc<dyn LlmPort>)
        .with_citadel(citadel.clone())
        .restore_from(paladin_id)
        .await?
        .build()
        .await?;

    println!("✅ Paladin restored successfully!");
    println!("   Name: {}", restored_paladin.data.name);
    println!("   ID: {}", restored_paladin.uuid);
    println!("   Status: {:?}", restored_paladin.data.status);
    println!();

    // Verify the restored state matches the original
    println!("🔍 Verifying restored state:");
    println!("   Original ID: {}", paladin_id);
    println!("   Restored ID: {}", restored_paladin.uuid);
    println!("   Match: {}", paladin_id == restored_paladin.uuid);
    println!();

    // Step 5: Continue execution with the restored Paladin
    println!("🚀 Continuing execution with restored Paladin...\n");

    let service_restored =
        PaladinExecutionService::new(llm_port_restored, circuit_breaker, None, None);

    let continued_result = service_restored
        .execute(
            &restored_paladin,
            "Provide additional insights on the framework's scalability",
        )
        .await?;

    println!("📊 Continued Execution Results:");
    println!(
        "   Output length: {} characters",
        continued_result.output.len()
    );
    println!("   Loops: {}", continued_result.loop_count);
    println!();

    // Step 6: Show the complete execution history
    println!("📜 Complete Execution History:");
    let final_state = citadel
        .load_paladin(paladin_id)
        .await?
        .expect("State should exist");
    println!(
        "   Total executions: {}",
        final_state.execution_history.len()
    );
    for (i, record) in final_state.execution_history.iter().enumerate() {
        println!(
            "   {}. {} - Loops: {}",
            i + 1,
            record.timestamp,
            record.loops_used
        );
    }
    println!();

    // Step 7: Demonstrate listing all saved states
    println!("📋 All saved states in Citadel:");
    let saved_states = citadel.list_saved().await?;
    for summary in saved_states {
        println!(
            "   • {} - {:?} - created: {}, updated: {}",
            summary.id, summary.state_type, summary.created_at, summary.updated_at
        );
    }
    println!();

    println!("✨ Example complete!");
    println!("\n💡 Key Takeaways:");
    println!("   • Paladin state can be saved and restored seamlessly");
    println!("   • Execution history is preserved across restarts");
    println!("   • Garrison (memory) is included in the saved state");
    println!("   • Multiple states can be managed in the Citadel");
    println!("   • Restoration enables fault-tolerant agent systems");

    Ok(())
}
