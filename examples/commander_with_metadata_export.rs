//! Commander with Metadata Export Example
//!
//! Demonstrates how to configure the Commander to export detailed execution
//! metadata to JSON files. This is useful for:
//! - Performance analysis and profiling
//! - Audit trails and compliance
//! - Debugging and troubleshooting
//! - Cost tracking (token usage per Paladin)
//!
//! The metadata export feature creates JSON files with comprehensive information
//! about each Battalion execution, including per-Paladin timing and token metrics.
//!
//! Run with: cargo run --example commander_with_metadata_export

use async_trait::async_trait;
use paladin::application::services::battalion::commander::CommanderBuilder;
use paladin::application::services::paladin::error::PaladinError;
use paladin::core::base::entity::node::Node;
use paladin::core::platform::container::battalion::{
    BattalionConfig, BattalionStrategy, ErrorStrategy,
};
use paladin::core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream, StopReason};
use std::path::PathBuf;
use std::sync::Arc;
use std::{fs, io};

/// Mock PaladinPort that simulates realistic execution with metrics
struct MockPaladinPort;

#[async_trait]
impl PaladinPort for MockPaladinPort {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        println!("\n🎯 {} executing...", paladin.node.name);
        println!("   📝 Input: {}", input);

        // Simulate realistic processing delay (varies by Paladin)
        let delay_ms = match paladin.node.name.as_str() {
            name if name.contains("Analyzer") => 500,
            name if name.contains("Processor") => 800,
            name if name.contains("Synthesizer") => 600,
            _ => 400,
        };
        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;

        // Generate output with realistic token counts
        let output = format!("{} completed: {}", paladin.node.name, input);
        let token_count = match paladin.node.name.as_str() {
            name if name.contains("Analyzer") => 450,
            name if name.contains("Processor") => 650,
            name if name.contains("Synthesizer") => 550,
            _ => 300,
        };

        println!("   ✅ Output: {}", output);
        println!("   🪙 Tokens: {}", token_count);
        println!("   ⏱️  Time: {}ms", delay_ms);

        Ok(PaladinResult {
            output,
            usage: paladin_ports::output::llm_port::TokenUsage::new(token_count, 0),
            execution_time_ms: delay_ms,
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

/// Helper function to create a Paladin with given name and role
fn create_paladin(name: &str, role: &str) -> Paladin {
    let data = PaladinData {
        system_prompt: format!("You are {}, responsible for: {}", name, role),
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

/// Create the metadata output directory
fn ensure_metadata_dir() -> io::Result<PathBuf> {
    let dir = PathBuf::from("./battalion_metadata");
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
        println!("📁 Created metadata directory: {}", dir.display());
    } else {
        println!("📁 Using existing metadata directory: {}", dir.display());
    }
    Ok(dir)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎖️  Commander with Metadata Export Example\n");
    println!("{}", "=".repeat(70));

    // Step 1: Create metadata output directory
    let metadata_dir = ensure_metadata_dir()?;
    println!(
        "\n✅ Metadata will be exported to: {}\n",
        metadata_dir.display()
    );

    // Step 2: Create the PaladinPort adapter
    let paladin_port = Arc::new(MockPaladinPort);

    // Step 3: Create specialized Paladins for parallel analysis
    // Using Phalanx strategy to demonstrate concurrent execution metrics
    let analyzer = create_paladin("DataAnalyzer", "analyzing raw data and extracting insights");
    let processor = create_paladin("DataProcessor", "processing and transforming analyzed data");
    let synthesizer = create_paladin(
        "ReportSynthesizer",
        "synthesizing findings into comprehensive report",
    );

    println!("📋 Created 3 Paladins for parallel execution:");
    println!("   1. {} - Data analysis", analyzer.node.name);
    println!("   2. {} - Data processing", processor.node.name);
    println!("   3. {} - Report synthesis", synthesizer.node.name);

    // Step 4: Create Battalion configuration with metadata export
    let config = BattalionConfig::new("metadata_export_demo")
        .with_timeout(120)
        .with_error_strategy(ErrorStrategy::ContinueOnError) // Continue on errors to capture all metrics
        .with_metadata_dir(metadata_dir.clone()); // Enable metadata export

    println!("\n⚙️  Configuration:");
    println!("   • Strategy: Phalanx (concurrent execution)");
    println!("   • Timeout: 120 seconds");
    println!("   • Error Strategy: ContinueOnError");
    println!("   • Metadata Export: ENABLED ✓");

    // Step 5: Create Commander with Phalanx strategy
    // Phalanx executes all Paladins concurrently, perfect for demonstrating
    // per-Paladin metrics collection
    let commander = CommanderBuilder::new(paladin_port)
        .strategy(BattalionStrategy::Phalanx)
        .paladins(vec![analyzer, processor, synthesizer])
        .config(config)
        .build()?;

    println!("\n{}", "=".repeat(70));
    println!("🚀 Starting Battalion execution...\n");

    // Step 6: Execute the Commander
    let result = commander
        .execute("Analyze quarterly sales data and generate insights")
        .await?;

    // Step 7: Display execution summary
    println!("\n{}", "=".repeat(70));
    println!("📊 Execution Summary:\n");
    println!("   Strategy Used: {:?}", result.strategy_used);
    println!("   Total Duration: {:?}", result.duration());
    println!("   Success Count: {}", result.paladin_success_count);
    println!("   Failure Count: {}", result.paladin_failure_count);
    println!("   Total Tokens: {}", result.total_tokens);

    // Display per-Paladin metrics
    println!("\n   Per-Paladin Metrics:");
    for (name, time_ms) in &result.per_paladin_times {
        let tokens = result
            .per_paladin_tokens
            .get(name)
            .map(|t| t.total_tokens)
            .unwrap_or(0);
        println!("      • {}: {}ms, {} tokens", name, time_ms, tokens);
    }

    // Step 8: Locate and display metadata file
    println!("\n{}", "=".repeat(70));
    println!("📄 Metadata File:");

    // Find the metadata JSON file
    let metadata_files: Vec<_> = fs::read_dir(&metadata_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .collect();

    if let Some(file) = metadata_files.last() {
        let file_path = file.path();
        println!("   📁 File: {}", file_path.display());

        // Display file size
        let metadata = fs::metadata(&file_path)?;
        println!("   📏 Size: {} bytes", metadata.len());

        // Display a sample of the JSON content
        let contents = fs::read_to_string(&file_path)?;
        let json: serde_json::Value = serde_json::from_str(&contents)?;

        println!("\n   🔍 JSON Structure Preview:");
        println!("      • battalion_id: {}", json["battalion_id"]);
        println!("      • battalion_name: {}", json["battalion_name"]);
        println!("      • strategy_used: {}", json["strategy_used"]);
        println!(
            "      • paladin_results: {} entries",
            json["paladin_results"].as_array().unwrap().len()
        );
        println!(
            "      • per_paladin_times: {} entries",
            json["per_paladin_times"].as_object().unwrap().len()
        );
        println!(
            "      • per_paladin_tokens: {} entries",
            json["per_paladin_tokens"].as_object().unwrap().len()
        );
    }

    println!("\n{}", "=".repeat(70));
    println!("✅ Example completed successfully!");
    println!("\n💡 Tips:");
    println!("   • Metadata files are named: {{strategy}}_{{timestamp}}_{{uuid}}.json");
    println!("   • Files contain complete execution history for audit trails");
    println!("   • Use metadata for performance analysis and cost tracking");
    println!("   • Configure metadata_output_dir in config.yml for production use");

    Ok(())
}
