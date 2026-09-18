//! Phalanx Parallel Execution Example
//!
//! This example demonstrates the Phalanx pattern - concurrent execution of multiple
//! Paladins with different aggregation strategies.
//!
//! # Concepts Demonstrated
//!
//! - **Concurrent Execution**: Multiple Paladins process the same input simultaneously
//! - **Aggregation Strategies**: Different ways to combine results
//!   - CollectAll: Gather all results
//!   - FirstSuccess: Return the first successful result
//!   - Majority: Require consensus (≥50%)
//! - **Concurrency Limiting**: Control max concurrent executions
//! - **Error Handling**: Continue on partial failures
//!
//! # Use Cases
//!
//! - Parallel data analysis with consensus validation
//! - Distributed voting systems requiring majority agreement
//! - Redundant API calls with first-success failover
//! - Load distribution across multiple processing nodes
//!
//! Run with: `cargo run --example phalanx_parallel`

use async_trait::async_trait;
use paladin::application::services::battalion::phalanx_service::PhalanxExecutionService;
use paladin::application::services::paladin::error::PaladinError;
use paladin::core::base::entity::node::Node;
use paladin::core::platform::container::battalion::phalanx::{AggregationStrategy, Phalanx};
use paladin::core::platform::container::battalion::{BattalionConfig, ErrorStrategy};
use paladin::core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, StopReason};
use std::sync::Arc;

/// Example mock implementation of PaladinPort
/// In a real application, this would call actual LLM APIs
struct ExamplePaladinPort;

#[async_trait]
impl PaladinPort for ExamplePaladinPort {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        // Simulate concurrent processing
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        // Generate output based on Paladin role
        let output = match paladin.node.name.as_str() {
            name if name.starts_with("Analyst") => {
                "Analysis: Option A is optimal based on cost-benefit analysis".to_string()
            }
            name if name.starts_with("Voter") => {
                if name.ends_with('1') || name.ends_with('2') {
                    "Vote: Option A".to_string()
                } else {
                    "Vote: Option B".to_string()
                }
            }
            name if name.contains("Fast") => "Quick Result".to_string(),
            name if name.contains("Slow") => {
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                "Delayed Result".to_string()
            }
            _ => format!("{} processed: {}", paladin.node.name, input),
        };

        Ok(PaladinResult {
            output,
            usage: paladin_ports::output::llm_port::TokenUsage::new(50, 0),
            execution_time_ms: 10,
            loop_count: 1,
            stop_reason: StopReason::Completed,
            ..Default::default()
        })
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<
        tokio::sync::mpsc::Receiver<
            Result<paladin_ports::output::paladin_port::PaladinStreamChunk, PaladinError>,
        >,
        PaladinError,
    > {
        let (_tx, rx) = tokio::sync::mpsc::channel(1);
        Ok(rx)
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

fn create_paladin(name: &str, system_prompt: &str) -> Paladin {
    let data = PaladinData {
        system_prompt: system_prompt.to_string(),
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
    Node::new(data, Some(name.to_string()))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🛡️  Phalanx Parallel Execution Examples\n");
    println!("===========================================\n");

    let paladin_port = Arc::new(ExamplePaladinPort);

    // Example 1: CollectAll - Gather all results
    example_1_collect_all(paladin_port.clone()).await?;

    // Example 2: FirstSuccess - Return first successful result
    example_2_first_success(paladin_port.clone()).await?;

    // Example 3: Majority - Require consensus
    example_3_majority_consensus(paladin_port.clone()).await?;

    // Example 4: Concurrency Limiting
    example_4_concurrency_limiting(paladin_port.clone()).await?;

    println!("\n✅ All examples completed successfully!");
    Ok(())
}

async fn example_1_collect_all(
    paladin_port: Arc<ExamplePaladinPort>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("📋 Example 1: CollectAll Aggregation Strategy");
    println!("----------------------------------------------");
    println!("Use case: Parallel data analysis where all perspectives are valuable\n");

    // Create 3 analyst Paladins with different perspectives
    let analysts = vec![
        create_paladin("Analyst 1", "You are a cost-benefit analyst"),
        create_paladin("Analyst 2", "You are a ROI analyst"),
        create_paladin("Analyst 3", "You are a risk analyst"),
    ];

    // Create Phalanx with CollectAll strategy (default)
    let config = BattalionConfig::new("analysis_phalanx")
        .with_timeout(10)
        .with_error_strategy(ErrorStrategy::ContinueOnError);

    let phalanx = Phalanx::new(analysts, config)?;

    println!("Created Phalanx with {} Paladins", phalanx.paladin_count());
    println!("Aggregation: {:?}", phalanx.aggregation_strategy());

    // Execute
    let service = PhalanxExecutionService::new(paladin_port);
    let result = service
        .execute(&phalanx, "Analyze our three strategic options")
        .await?;

    println!("\n✅ Execution completed:");
    println!("   - Paladins executed: {}", result.paladin_results.len());
    println!("   - Status: {:?}", result.status);
    println!("   - Results collected: {}", result.paladin_results.len());

    for (i, paladin_result) in result.paladin_results.iter().enumerate() {
        println!("   Result {}: {}", i + 1, paladin_result.output);
    }

    println!();
    Ok(())
}

async fn example_2_first_success(
    paladin_port: Arc<ExamplePaladinPort>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("⚡ Example 2: FirstSuccess Aggregation Strategy");
    println!("-----------------------------------------------");
    println!("Use case: Redundant API calls with fast failover\n");

    // Create Paladins with varying response times
    let workers = vec![
        create_paladin("Fast Worker", "You process requests quickly"),
        create_paladin("Slow Worker", "You process requests slowly"),
    ];

    let config = BattalionConfig::new("fast_response_phalanx")
        .with_timeout(5)
        .with_error_strategy(ErrorStrategy::FailFast);

    let phalanx =
        Phalanx::new(workers, config)?.with_aggregation(AggregationStrategy::FirstSuccess);

    println!("Created Phalanx with {} Paladins", phalanx.paladin_count());
    println!("Aggregation: {:?}", phalanx.aggregation_strategy());
    println!("Note: Will return as soon as first Paladin succeeds\n");

    let service = PhalanxExecutionService::new(paladin_port);
    let start = std::time::Instant::now();
    let result = service.execute(&phalanx, "Process request").await?;
    let duration = start.elapsed();

    println!("✅ Execution completed in {:?}", duration);
    println!(
        "   - Results returned: {} (only first success)",
        result.paladin_results.len()
    );
    println!("   - Output: {}", result.final_output);
    println!("   - Note: Slower workers were cancelled early\n");

    Ok(())
}

async fn example_3_majority_consensus(
    paladin_port: Arc<ExamplePaladinPort>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🗳️  Example 3: Majority Aggregation Strategy");
    println!("--------------------------------------------");
    println!("Use case: Consensus voting system requiring ≥50% agreement\n");

    // Create 5 voting Paladins (3 will vote for Option A, 2 for Option B)
    let voters = vec![
        create_paladin("Voter 1", "You are voter 1"),
        create_paladin("Voter 2", "You are voter 2"),
        create_paladin("Voter 3", "You are voter 3"),
        create_paladin("Voter 4", "You are voter 4"),
        create_paladin("Voter 5", "You are voter 5"),
    ];

    let config = BattalionConfig::new("consensus_phalanx")
        .with_timeout(10)
        .with_error_strategy(ErrorStrategy::ContinueOnError);

    let phalanx = Phalanx::new(voters, config)?.with_aggregation(AggregationStrategy::Majority);

    println!("Created Phalanx with {} Paladins", phalanx.paladin_count());
    println!("Aggregation: {:?}", phalanx.aggregation_strategy());
    println!("Requires: ≥50% consensus (at least 3/5 Paladins must agree)\n");

    let service = PhalanxExecutionService::new(paladin_port);
    let result = service
        .execute(&phalanx, "Vote on the best strategic option")
        .await?;

    println!("✅ Consensus reached:");
    println!("   - Paladins executed: {}", result.paladin_results.len());
    println!("   - Majority output: {}", result.final_output);
    println!("   - Note: At least 3/5 Paladins agreed on this output\n");

    Ok(())
}

async fn example_4_concurrency_limiting(
    paladin_port: Arc<ExamplePaladinPort>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🚦 Example 4: Concurrency Limiting");
    println!("-----------------------------------");
    println!("Use case: Process 10 tasks but limit to 3 concurrent executions\n");

    // Create 10 worker Paladins
    let workers: Vec<Paladin> = (1..=10)
        .map(|i| create_paladin(&format!("Worker {}", i), "You process tasks"))
        .collect();

    let config = BattalionConfig::new("batch_processing_phalanx")
        .with_timeout(30)
        .with_error_strategy(ErrorStrategy::ContinueOnError);

    let phalanx = Phalanx::new(workers, config)?.with_max_concurrency(3);

    println!("Created Phalanx with {} Paladins", phalanx.paladin_count());
    println!("Max concurrent: {}", phalanx.max_concurrency().unwrap());
    println!("Note: Only 3 Paladins will run simultaneously\n");

    let service = PhalanxExecutionService::new(paladin_port);
    let start = std::time::Instant::now();
    let result = service.execute(&phalanx, "Process batch item").await?;
    let duration = start.elapsed();

    println!("✅ Batch processing completed:");
    println!("   - Total Paladins: {}", result.paladin_results.len());
    println!("   - Results: {}", result.paladin_results.len());
    println!("   - Duration: {:?}", duration);
    println!("   - Note: Concurrency limiting prevents resource exhaustion\n");

    Ok(())
}
