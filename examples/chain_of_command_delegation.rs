//! Chain of Command Delegation Example
//!
//! This example demonstrates the Chain of Command Battalion pattern with different
//! delegation strategies: Automatic, Broadcast, RoundRobin, and Custom.
//!
//! Run with: cargo run --example chain_of_command_delegation

use async_trait::async_trait;
use paladin::application::services::battalion::chain_of_command_service::ChainOfCommandExecutionService;
use paladin::application::services::paladin::error::PaladinError;
use paladin::core::platform::container::battalion::BattalionConfig;
use paladin::core::platform::container::battalion::chain_of_command::{
    ChainOfCommand, DelegationStrategy,
};
use paladin::core::platform::container::paladin::{Paladin, PaladinData};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, StopReason};
use std::sync::Arc;

/// Mock Paladin Port that simulates specialist responses
struct ExampleMockPort;

#[async_trait]
impl PaladinPort for ExampleMockPort {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        // Simulate specialist processing based on their name
        let output = match paladin.node.name.as_str() {
            "commander" => {
                // Commander analyzes and selects specialists
                if input.contains("Query the database") {
                    "SELECT: database_specialist\nREASON: This task requires database query expertise".to_string()
                } else if input.contains("Process payment") {
                    "SELECT: database_specialist, cache_specialist\nREASON: Both database and cache need updating for payment processing".to_string()
                } else {
                    "SELECT: database_specialist\nREASON: Default to database specialist"
                        .to_string()
                }
            }
            "database_specialist" => {
                format!(
                    "✓ Database specialist processed: {}\n  → Executed SQL query successfully\n  → Retrieved 42 records",
                    input
                )
            }
            "cache_specialist" => {
                format!(
                    "✓ Cache specialist processed: {}\n  → Cleared relevant cache entries\n  → Updated cache with new data",
                    input
                )
            }
            "api_specialist" => {
                format!(
                    "✓ API specialist processed: {}\n  → Called external API successfully\n  → Received valid response",
                    input
                )
            }
            "analytics_specialist" => {
                format!(
                    "✓ Analytics specialist processed: {}\n  → Generated analytics report\n  → Identified key trends",
                    input
                )
            }
            _ => format!("{} processed: {}", paladin.node.name, input),
        };

        Ok(PaladinResult {
            output,
            usage: paladin_ports::output::llm_port::TokenUsage::new(100, 0),
            execution_time_ms: 50,
            loop_count: 1,
            stop_reason: StopReason::Completed,
            ..Default::default()
        })
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<paladin_ports::output::paladin_port::PaladinStream, PaladinError> {
        unimplemented!("Streaming not used in example")
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

fn create_paladin(name: &str, description: &str) -> Paladin {
    let data = PaladinData {
        system_prompt: description.to_string(),
        name: name.to_string(),
        user_name: "example_user".to_string(),
        ..Default::default()
    };
    Paladin::new(data, Some(name.to_string()))
}

fn print_section(title: &str) {
    println!("\n{}", "=".repeat(80));
    println!("  {}", title);
    println!("{}\n", "=".repeat(80));
}

fn print_result(
    delegation_result: &paladin::application::services::battalion::chain_of_command_service::DelegationResult,
) {
    println!("📋 Delegation Result:");
    println!(
        "   Selected: {} specialist(s)",
        delegation_result.selected_specialists.len()
    );
    for specialist in &delegation_result.selected_specialists {
        println!("   • {}", specialist);
    }
    println!("\n💭 Reasoning: {}\n", delegation_result.reasoning);
    println!("📤 Outputs:");
    for (i, output) in delegation_result.outputs.iter().enumerate() {
        println!("   [{}] {}", i + 1, output.lines().next().unwrap_or(""));
        for line in output.lines().skip(1) {
            println!("       {}", line);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🎯 Chain of Command Delegation Pattern Examples");
    println!("================================================\n");
    println!("This example demonstrates hierarchical delegation patterns where a");
    println!("commander Paladin coordinates specialist Paladins using different strategies.\n");

    let mock_port = Arc::new(ExampleMockPort);
    let service = ChainOfCommandExecutionService::new(mock_port);

    // Create commander and specialists
    let commander = create_paladin(
        "commander",
        "You are a team coordinator who analyzes tasks and selects the best specialists",
    );
    let db_specialist = create_paladin(
        "database_specialist",
        "Expert in database queries and data management",
    );
    let cache_specialist = create_paladin(
        "cache_specialist",
        "Expert in cache management and optimization",
    );
    let api_specialist = create_paladin(
        "api_specialist",
        "Expert in API integration and external service calls",
    );
    let analytics_specialist = create_paladin(
        "analytics_specialist",
        "Expert in data analysis and reporting",
    );

    // Example 1: Automatic Delegation (Commander selects specialists)
    print_section("Example 1: AUTOMATIC DELEGATION");
    println!(
        "The commander analyzes the task and intelligently selects appropriate specialist(s).\n"
    );

    let config = BattalionConfig::default();
    let chain = ChainOfCommand::new(
        commander.clone(),
        vec![
            db_specialist.clone(),
            cache_specialist.clone(),
            api_specialist.clone(),
        ],
        config.clone(),
    )?
    .with_strategy(DelegationStrategy::Automatic);

    println!("🎯 Task: Query the database for active users");
    let result = service
        .execute(&chain, "Query the database for active users")
        .await?;
    print_result(&result);

    println!("\n{}", "-".repeat(80));
    println!("🎯 Task: Process payment transaction");
    let result = service
        .execute(&chain, "Process payment transaction")
        .await?;
    print_result(&result);

    // Example 2: Broadcast Delegation (All specialists execute)
    print_section("Example 2: BROADCAST DELEGATION");
    println!("All specialists receive and process the same task concurrently.\n");

    let chain = ChainOfCommand::new(
        commander.clone(),
        vec![
            db_specialist.clone(),
            cache_specialist.clone(),
            analytics_specialist.clone(),
        ],
        config.clone(),
    )?
    .with_strategy(DelegationStrategy::Broadcast);

    println!("🎯 Task: Analyze system health from multiple perspectives");
    let result = service
        .execute(&chain, "Analyze system health from multiple perspectives")
        .await?;
    print_result(&result);

    // Example 3: Round-Robin Delegation (Load balancing)
    print_section("Example 3: ROUND-ROBIN DELEGATION");
    println!("Tasks are distributed evenly across specialists in rotation.\n");

    let chain = ChainOfCommand::new(
        commander.clone(),
        vec![
            db_specialist.clone(),
            api_specialist.clone(),
            analytics_specialist.clone(),
        ],
        config.clone(),
    )?
    .with_strategy(DelegationStrategy::RoundRobin);

    for i in 1..=4 {
        println!("🎯 Task {}: Process request #{}", i, i);
        let result = service
            .execute(&chain, &format!("Process request #{}", i))
            .await?;
        println!("   Selected: {}", result.selected_specialists[0]);
        println!(
            "   Result: {}\n",
            result.outputs[0].lines().next().unwrap_or("")
        );
    }

    // Example 4: Custom Delegation (User-defined logic)
    print_section("Example 4: CUSTOM DELEGATION");
    println!("Custom logic determines specialist selection based on business rules.\n");

    let chain = ChainOfCommand::new(
        commander.clone(),
        vec![db_specialist.clone(), cache_specialist.clone()],
        config.clone(),
    )?
    .with_strategy(DelegationStrategy::Custom(
        "Route high-priority tasks to the first specialist".to_string(),
    ));

    println!("🎯 Task: High-priority data operation");
    let result = service
        .execute(&chain, "High-priority data operation")
        .await?;
    print_result(&result);

    // Summary
    print_section("SUMMARY");
    println!("✅ Automatic:  Commander analyzes and selects specialist(s) intelligently");
    println!("✅ Broadcast:  All specialists execute concurrently for comprehensive analysis");
    println!("✅ RoundRobin: Even load distribution across specialists");
    println!("✅ Custom:     User-defined routing logic for specific business needs\n");

    println!("💡 Use Cases:");
    println!("   • Automatic:  Complex decision-making, dynamic routing");
    println!("   • Broadcast:  Consensus building, multi-perspective analysis");
    println!("   • RoundRobin: Load balancing, fair resource distribution");
    println!("   • Custom:     Business-specific routing rules\n");

    Ok(())
}
