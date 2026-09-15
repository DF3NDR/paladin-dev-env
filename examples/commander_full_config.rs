//! Commander Full Configuration Example
//!
//! Demonstrates comprehensive Commander configuration including:
//! - Error handling strategies (FailFast, ContinueOnError, RetryThenContinue)
//! - Timeout configuration
//! - Retry policies
//! - Metadata output directory for checkpointing
//!
//! This example shows production-grade configuration for robust deployments.
//!
//! Run with: cargo run --example commander_full_config

use async_trait::async_trait;
use paladin::application::services::battalion::commander::CommanderBuilder;
use paladin::application::services::paladin::error::PaladinError;
use paladin::core::base::entity::node::Node;
use paladin::core::platform::container::battalion::{
    BattalionConfig, BattalionStrategy, ErrorStrategy, RetryPolicy,
};
use paladin::core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream, StopReason};
use std::path::PathBuf;
use std::sync::Arc;

/// Mock PaladinPort with configurable failure simulation
struct ConfigurableMockPort {
    fail_count: std::sync::atomic::AtomicUsize,
}

impl ConfigurableMockPort {
    fn new() -> Self {
        Self {
            fail_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl PaladinPort for ConfigurableMockPort {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        println!("   ⚙️  {} executing...", paladin.node.name);

        // Simulate occasional failures for the second Paladin
        if paladin.node.name == "ProcessorB" {
            let count = self
                .fail_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if count < 2 {
                // Fail first 2 attempts to demonstrate retry
                println!("      ❌ Simulated failure (attempt {})", count + 1);
                return Err(PaladinError::ExecutionError(
                    "Simulated transient failure".to_string(),
                ));
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let output = format!("{} processed: {}", paladin.node.name, input);
        println!("      ✅ Success: {}", output);

        Ok(PaladinResult {
            output,
            usage: paladin_ports::output::llm_port::TokenUsage::new(50, 0),
            execution_time_ms: 100,
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
            system_prompt: format!("You are {}, a specialized processor.", name),
            name: name.to_string(),
            user_name: "System".to_string(),
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

/// Demonstrate different error handling strategies
async fn demonstrate_error_strategy(
    strategy_name: &str,
    strategy: ErrorStrategy,
    max_retries: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{}", "═".repeat(70));
    println!("🔧 Configuration: {}", strategy_name);
    println!("{}", "═".repeat(70));

    let paladin_port = Arc::new(ConfigurableMockPort::new());

    // Create a comprehensive BattalionConfig using builder pattern
    let config = BattalionConfig::new(format!("config_example_{}", strategy_name.to_lowercase()))
        .with_timeout(120) // 2 minute timeout for the entire Battalion execution
        .with_retry_policy(RetryPolicy {
            max_attempts: max_retries, // Number of retry attempts before giving up
            ..Default::default()
        })
        .with_error_strategy(strategy) // How to handle errors: FailFast, ContinueOnError, RetryThenContinue
        .with_metadata_dir(PathBuf::from("./battalion_metadata")); // Directory for checkpointing

    println!("\n⚙️  Configuration Details:");
    println!("{}", "-".repeat(70));
    println!("   Battalion Name: {}", config.name);
    println!("   Timeout: {} seconds", config.timeout_seconds);
    println!(
        "   Max Retry Attempts: {}",
        config.retry_policy.max_attempts
    );
    println!("   Error Strategy: {:?}", config.error_strategy);
    println!(
        "   Metadata Dir: {}",
        config
            .metadata_output_dir
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "None".to_string())
    );

    let paladins = vec![
        create_paladin("ProcessorA"),
        create_paladin("ProcessorB"), // This one will fail initially
        create_paladin("ProcessorC"),
    ];

    println!("\n📋 Paladins: {}", paladins.len());
    for (i, p) in paladins.iter().enumerate() {
        println!("   {}. {}", i + 1, p.node.name);
    }

    // Build Commander with full configuration
    let commander = CommanderBuilder::new(paladin_port)
        .strategy(BattalionStrategy::Formation)
        .paladins(paladins)
        .config(config)
        .build()?;

    println!("\n🚀 Executing Commander...\n");

    let result = commander.execute("Process important data").await;

    // Display results based on success or failure
    match result {
        Ok(battalion_result) => {
            println!("\n✅ Execution completed successfully!");
            println!("{}", "-".repeat(70));
            println!("   Status: {:?}", battalion_result.status);
            println!("   Strategy: {:?}", battalion_result.strategy_used);
            println!(
                "   Total Time: {}ms",
                battalion_result
                    .completed_at
                    .signed_duration_since(battalion_result.started_at)
                    .num_milliseconds()
            );
            println!(
                "   Succeeded: {} | Failed: {}",
                battalion_result.paladin_success_count, battalion_result.paladin_failure_count
            );
            println!("\n   Final Output:");
            println!("      {}", battalion_result.final_output);
        }
        Err(e) => {
            println!("\n❌ Execution failed (as expected with FailFast)");
            println!("{}", "-".repeat(70));
            println!("   Error: {}", e);
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🎖️  Commander Full Configuration Example\n");
    println!("Demonstrating production-grade configuration options\n");

    // Example 1: RetryThenContinue (Recommended for production)
    // Retries failed Paladins, then continues even if they still fail
    demonstrate_error_strategy(
        "RetryThenContinue (Resilient)",
        ErrorStrategy::RetryThenContinue,
        3, // Retry up to 3 times
    )
    .await?;

    // Example 2: ContinueOnError
    // Never retries, just continues to next Paladin
    demonstrate_error_strategy(
        "ContinueOnError (Fault-Tolerant)",
        ErrorStrategy::ContinueOnError,
        1, // No retries (max_attempts = 1)
    )
    .await?;

    // Example 3: FailFast
    // Stops immediately on first error
    demonstrate_error_strategy(
        "FailFast (Strict)",
        ErrorStrategy::FailFast,
        1, // No retries
    )
    .await?;

    // Summary and best practices
    println!("\n\n{}", "═".repeat(70));
    println!("📚 Configuration Best Practices");
    println!("{}", "═".repeat(70));
    println!("\n🔧 Error Handling Strategies:");
    println!("   • FailFast: Use for critical pipelines where any failure is unacceptable");
    println!("   • ContinueOnError: Use when some failures are acceptable");
    println!("   • RetryThenContinue: Recommended for production - resilient and robust");

    println!("\n⏱️  Timeout Configuration:");
    println!("   • Set based on expected max execution time");
    println!("   • Include buffer for retries and network delays");
    println!("   • Typical values: 60-300 seconds for complex workflows");

    println!("\n🔄 Retry Policy:");
    println!("   • max_attempts = 1: No retries (FailFast, ContinueOnError)");
    println!("   • max_attempts = 3: Standard retry count for transient failures");
    println!("   • max_attempts = 5: Aggressive retry for flaky services");

    println!("\n💾 Metadata Output:");
    println!("   • Enable checkpointing by setting metadata_output_dir");
    println!("   • Allows recovery from failures in long-running workflows");
    println!("   • Stores execution state, timing, and intermediate results");

    println!("\n🎯 Recommended Production Config:");
    println!("   ```rust");
    println!("   BattalionConfig {{");
    println!("       name: \"production_battalion\".to_string(),");
    println!("       timeout_seconds: 300,  // 5 minutes");
    println!("       retry_policy: RetryPolicy {{ max_attempts: 3 }},");
    println!("       error_strategy: ErrorStrategy::RetryThenContinue,");
    println!("       metadata_output_dir: Some(\"./checkpoints\".to_string()),");
    println!("   }}");
    println!("   ```");

    println!("\n{}", "═".repeat(70));
    println!("✅ All configuration examples completed successfully!\n");

    Ok(())
}
