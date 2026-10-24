//! Commander Full Configuration Example
//!
//! Demonstrates comprehensive Commander configuration including:
//! - The three failure behaviours a Battalion can have, each as an `Aegis` policy:
//!   fail fast, continue past a failed Paladin, and retry then continue
//! - A per-attempt timeout
//! - A retry policy with exponential backoff
//! - Metadata output directory for checkpointing
//!
//! This example shows production-grade configuration for robust deployments, and
//! doubles as the migration guide for code that used the earlier error strategies:
//! every one of them is now a combination of the `retry`, `timeout` and `on_error`
//! fields of `BattalionConfig.aegis`.
//!
//! Run with: cargo run --example commander_full_config

use async_trait::async_trait;
use paladin::application::services::battalion::commander::CommanderBuilder;
use paladin::application::services::paladin::error::PaladinError;
use paladin::core::base::entity::Entity;
use paladin::core::platform::container::aegis::{
    Aegis, ErrorHandlerSpec, RetryPolicy, RetryPredicate, TimeoutPolicy,
};
use paladin::core::platform::container::battalion::{BattalionConfig, BattalionStrategy};
use paladin::core::platform::container::battlefield::StateDelta;
use paladin::core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream, StopReason};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

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
        println!("   ⚙️  {} executing...", paladin.data.name);

        // Simulate occasional failures for the second Paladin
        if paladin.data.name == "ProcessorB" {
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

        let output = format!("{} processed: {}", paladin.data.name, input);
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
    Entity::new(
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

/// Run the three-Paladin Formation under one `Aegis` policy and report the outcome.
async fn demonstrate_policy(label: &str, aegis: Aegis) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{}", "═".repeat(70));
    println!("🔧 Configuration: {}", label);
    println!("{}", "═".repeat(70));

    let paladin_port = Arc::new(ConfigurableMockPort::new());

    // Create a comprehensive BattalionConfig using builder pattern
    let config = BattalionConfig::new(format!("config_example_{}", label.replace(' ', "_")))
        .with_aegis(aegis) // retry, timeout and failure handling for each Paladin
        .with_metadata_dir(PathBuf::from("./battalion_metadata")); // Directory for checkpointing

    println!("\n⚙️  Configuration Details:");
    println!("{}", "-".repeat(70));
    println!("   Battalion Name: {}", config.name);
    println!(
        "   Per-attempt Timeout: {}",
        config
            .aegis
            .timeout
            .as_ref()
            .and_then(|t| t.run_timeout)
            .map(|d| format!("{} seconds", d.as_secs()))
            .unwrap_or_else(|| "None".to_string())
    );
    println!(
        "   Max Attempts: {}",
        config
            .aegis
            .retry
            .as_ref()
            .map_or(1, |retry| retry.max_attempts)
    );
    println!(
        "   Failures Absorbed: {}",
        matches!(config.aegis.on_error, Some(ErrorHandlerSpec::Absorb { .. }))
    );
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
        println!("   {}. {}", i + 1, p.data.name);
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
            for node_error in &battalion_result.node_errors {
                println!("   Recorded failure: {}", node_error);
            }
            println!("\n   Final Output:");
            println!("      {}", battalion_result.final_output);
        }
        Err(e) => {
            println!("\n❌ Execution failed (as expected without an on_error handler)");
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

    // The timeout bounds each Paladin attempt, not the whole run. It is the same
    // in all three configurations below.
    let per_attempt_timeout = TimeoutPolicy {
        run_timeout: Some(Duration::from_secs(120)),
        idle_timeout: None,
    };

    // Example 1: retry, then continue (recommended for production)
    // Retries a failed Paladin, then continues even if it still fails.
    //
    // `max_attempts` counts TOTAL attempts including the first, so a policy that
    // allowed three retries is `max_attempts: 4`.
    //
    // `TransientAndUnknown` is an opt-in: it also retries failures that cannot be
    // classified (the mock's plain execution error is one). The default,
    // `TransientOnly`, retries only failures known to be transient.
    demonstrate_policy(
        "retry then continue",
        Aegis {
            retry: Some(RetryPolicy {
                max_attempts: 4,
                initial_interval: Duration::from_millis(100),
                backoff_factor: 2.0,
                max_interval: Duration::from_secs(10),
                jitter: true,
                retry_on: RetryPredicate::TransientAndUnknown,
            }),
            timeout: Some(per_attempt_timeout.clone()),
            on_error: Some(ErrorHandlerSpec::Absorb {
                fallback_delta: StateDelta::new(),
            }),
            ..Aegis::default()
        },
    )
    .await?;

    // Example 2: continue past a failed Paladin
    // Never retries; an absorbed failure lets the next Paladin run with an empty input.
    demonstrate_policy(
        "continue past a failed Paladin",
        Aegis {
            timeout: Some(per_attempt_timeout.clone()),
            on_error: Some(ErrorHandlerSpec::Absorb {
                fallback_delta: StateDelta::new(),
            }),
            ..Aegis::default()
        },
    )
    .await?;

    // Example 3: fail fast
    // With no on_error handler the first failure stops the Formation.
    demonstrate_policy(
        "fail fast",
        Aegis {
            timeout: Some(per_attempt_timeout),
            ..Aegis::default()
        },
    )
    .await?;

    // Summary and best practices
    println!("\n\n{}", "═".repeat(70));
    println!("📚 Configuration Best Practices");
    println!("{}", "═".repeat(70));
    println!("\n🔧 Failure Handling (BattalionConfig.aegis):");
    println!(
        "   • No on_error handler: use for critical pipelines where any failure is unacceptable"
    );
    println!("   • on_error Absorb: use when some failures are acceptable");
    println!("   • retry plus Absorb: recommended for production - resilient and robust");

    println!("\n⏱️  Timeout Configuration:");
    println!("   • The timeout bounds each Paladin attempt, not the whole run");
    println!("   • Set based on the expected max time of one Paladin call");
    println!("   • Typical values: 60-300 seconds for complex workflows");

    println!("\n🔄 Retry Policy:");
    println!("   • No retry policy (or max_attempts = 1): a single attempt");
    println!("   • max_attempts = 4: three retries after the first attempt");
    println!("   • max_attempts counts total attempts, including the first");
    println!("   • retry_on TransientOnly is the default; TransientAndUnknown is an opt-in");

    println!("\n💾 Metadata Output:");
    println!("   • Enable checkpointing by setting metadata_output_dir");
    println!("   • Allows recovery from failures in long-running workflows");
    println!("   • Stores execution state, timing, and intermediate results");

    println!("\n🎯 Recommended Production Config:");
    println!("   ```rust");
    println!("   BattalionConfig::new(\"production_battalion\")");
    println!("       .with_aegis(Aegis {{");
    println!("           retry: Some(RetryPolicy {{");
    println!("               max_attempts: 4,");
    println!("               retry_on: RetryPredicate::TransientAndUnknown,");
    println!("               ..RetryPolicy::default()");
    println!("           }}),");
    println!("           timeout: Some(TimeoutPolicy {{");
    println!("               run_timeout: Some(Duration::from_secs(300)), // per attempt");
    println!("               idle_timeout: None,");
    println!("           }}),");
    println!("           on_error: Some(ErrorHandlerSpec::Absorb {{");
    println!("               fallback_delta: StateDelta::new(),");
    println!("           }}),");
    println!("           ..Aegis::default()");
    println!("       }})");
    println!("       .with_metadata_dir(\"./checkpoints\".into())");
    println!("   ```");

    println!("\n{}", "═".repeat(70));
    println!("✅ All configuration examples completed successfully!\n");

    Ok(())
}
