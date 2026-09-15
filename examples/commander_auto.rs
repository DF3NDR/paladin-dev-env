//! Commander Auto Mode Example
//!
//! Demonstrates the Commander's Auto mode, which automatically selects the best
//! Battalion strategy based on the Paladins provided and task characteristics.
//!
//! The Auto mode uses heuristics to determine whether Formation, Phalanx, Campaign,
//! or ChainOfCommand is most appropriate for the given scenario.
//!
//! Run with: cargo run --example commander_auto

use async_trait::async_trait;
use paladin::application::services::battalion::commander::CommanderBuilder;
use paladin::application::services::paladin::error::PaladinError;
use paladin::core::base::entity::node::Node;
use paladin::core::platform::container::battalion::{BattalionConfig, BattalionStrategy};
use paladin::core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream, StopReason};
use std::sync::Arc;

/// Mock PaladinPort for demonstration
struct MockPaladinPort;

#[async_trait]
impl PaladinPort for MockPaladinPort {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        println!("   🤖 {} processing...", paladin.node.name);
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let output = format!("[{}]: {}", paladin.node.name, input);

        Ok(PaladinResult {
            output,
            usage: paladin_ports::output::llm_port::TokenUsage::new(50, 0),
            execution_time_ms: 200,
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
        unimplemented!()
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

fn create_paladin(name: &str) -> Paladin {
    Node::new(
        PaladinData {
            system_prompt: format!("You are {}", name),
            name: name.to_string(),
            user_name: "User".to_string(),
            model: "gpt-4".to_string(),
            temperature: 0.7,
            max_loops: MaxLoops::Fixed(1),
            stop_words: vec![],
            status: PaladinStatus::Idle,
            vision_enabled: false,
            ..Default::default()
        },
        None,
    )
}

/// Demonstrate Auto mode with different Paladin configurations
async fn run_auto_example(
    title: &str,
    paladins: Vec<Paladin>,
    task: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{}", "=".repeat(70));
    println!("🎯 {}", title);
    println!("{}", "=".repeat(70));

    let paladin_port = Arc::new(MockPaladinPort);
    let config = BattalionConfig::new("auto_example").with_timeout(30);

    // Build Commander with Auto strategy
    // The Commander will analyze the Paladins and select the best strategy
    let commander = CommanderBuilder::new(paladin_port)
        .strategy(BattalionStrategy::Auto) // Let Commander decide
        .paladins(paladins.clone())
        .config(config)
        .build()?;

    println!("\n📋 Paladins: {}", paladins.len());
    for (i, p) in paladins.iter().enumerate() {
        println!("   {}. {}", i + 1, p.node.name);
    }

    println!("\n🚀 Executing with Auto mode...\n");
    let result = commander.execute(task).await?;

    // Display Auto mode decision and telemetry
    println!();
    println!("📊 Auto Mode Results:");
    println!("{}", "-".repeat(70));
    println!("   🎯 Selected Strategy: {:?}", result.strategy_used);

    // Show the reasoning behind strategy selection
    if let Some(reasoning) = &result.strategy_selection_reasoning {
        println!("\n   🧠 Selection Reasoning:");
        println!("      {}", reasoning);
    }

    // Display telemetry metadata
    println!("\n   ⏱️  Telemetry:");
    println!(
        "      Strategy Selection Time: {}ms",
        result.strategy_selection_time_ms
    );
    println!(
        "      Total Execution Time: {}ms",
        result
            .completed_at
            .signed_duration_since(result.started_at)
            .num_milliseconds()
    );
    println!("      Paladins Succeeded: {}", result.paladin_success_count);
    println!("      Paladins Failed: {}", result.paladin_failure_count);

    // Show per-Paladin timing if available
    if !result.per_paladin_times.is_empty() {
        println!("\n   🕐 Per-Paladin Times:");
        for (name, time_ms) in &result.per_paladin_times {
            println!("      {} - {}ms", name, time_ms);
        }
    }

    println!("\n   📝 Final Output:");
    println!("      {}", result.final_output);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🎖️  Commander Auto Mode Examples\n");
    println!("Demonstrating automatic strategy selection based on context\n");

    // Example 1: Few Paladins (2-3) → Auto selects Formation
    // Sequential processing is optimal for small pipelines
    println!("\n{}", "═".repeat(70));
    println!("Example 1: Small Pipeline (Auto → Formation)");
    println!("{}", "═".repeat(70));

    let pipeline_paladins = vec![
        create_paladin("DataExtractor"),
        create_paladin("DataTransformer"),
    ];

    run_auto_example(
        "Small Sequential Pipeline",
        pipeline_paladins,
        "Process customer data",
    )
    .await?;

    // Example 2: Many similar Paladins (4+) → Auto selects Phalanx
    // Parallel processing is optimal for independent tasks
    println!("\n\n{}", "═".repeat(70));
    println!("Example 2: Parallel Workers (Auto → Phalanx)");
    println!("{}", "═".repeat(70));

    let worker_paladins = vec![
        create_paladin("Worker1"),
        create_paladin("Worker2"),
        create_paladin("Worker3"),
        create_paladin("Worker4"),
        create_paladin("Worker5"),
    ];

    run_auto_example(
        "Parallel Independent Tasks",
        worker_paladins,
        "Process batch of items",
    )
    .await?;

    // Example 3: Complex workflow keywords → Auto selects Campaign
    // Graph-based orchestration for complex dependencies
    println!("\n\n{}", "═".repeat(70));
    println!("Example 3: Complex Workflow (Auto → Campaign)");
    println!("{}", "═".repeat(70));

    let workflow_paladins = vec![
        create_paladin("DataCollector"),
        create_paladin("FeatureExtractor"),
        create_paladin("ModelTrainer"),
        create_paladin("ResultValidator"),
    ];

    run_auto_example(
        "Multi-Stage Workflow",
        workflow_paladins,
        "Execute ML pipeline workflow with dependencies",
    )
    .await?;

    // Summary
    println!("\n\n{}", "═".repeat(70));
    println!("📚 Summary: Auto Mode Strategy Selection");
    println!("{}", "═".repeat(70));
    println!("\nAuto mode uses these heuristics:");
    println!("   • 2-3 Paladins → Formation (sequential pipeline)");
    println!("   • 4+ similar Paladins → Phalanx (parallel execution)");
    println!("   • Workflow keywords → Campaign (graph orchestration)");
    println!("   • Specialist delegation → ChainOfCommand (hierarchical)");
    println!("\nBenefits of Auto mode:");
    println!("   ✓ Simplifies API - no manual strategy selection needed");
    println!("   ✓ Optimizes execution pattern automatically");
    println!("   ✓ Provides reasoning for transparency");
    println!("   ✓ Includes detailed telemetry for monitoring");
    println!("\n{}", "═".repeat(70));
    println!("✅ All examples completed successfully!\n");

    Ok(())
}
