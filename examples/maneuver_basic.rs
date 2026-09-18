//! Basic Maneuver Flow DSL Example
//!
//! This example demonstrates the fundamentals of the Maneuver pattern - declarative
//! multi-agent orchestration using Flow DSL syntax.
//!
//! # Concepts Demonstrated
//!
//! - **Flow DSL Syntax**: Define workflows as text expressions
//!   - Sequential execution with `->`
//!   - Parallel execution with `,`
//!   - Nested patterns combining both
//! - **Declarative Configuration**: Express complex workflows simply
//! - **Visual Feedback**: ASCII tree visualization of flow structure
//! - **Mixed Patterns**: Combine sequential and parallel in one flow
//!
//! # Flow Structure
//!
//! ```text
//! Flow: "intake -> (analyzer, summarizer) -> reviewer"
//!
//! Execution:
//! Input → intake → ┌─ analyzer ─┐
//!                  └─ summarizer ─┘ → reviewer → Output
//! ```
//!
//! # Use Cases
//!
//! - Document processing pipelines with parallel analysis
//! - Multi-perspective review workflows
//! - Data transformation with concurrent operations
//! - Dynamic workflow generation from configuration
//!
//! Run with: `cargo run --example maneuver_basic`

use async_trait::async_trait;
use paladin::application::services::battalion::flow_visualizer::{
    FlowVisualizer, VisualizationFormat,
};
use paladin::application::services::battalion::maneuver_service::ManeuverExecutionService;
use paladin::application::services::paladin::error::PaladinError;
use paladin::core::base::entity::node::Node;
use paladin::core::platform::container::battalion::maneuver::{
    ErrorStrategy, Maneuver, ManeuverConfig, OutputFormat,
};
use paladin::core::platform::container::battalion::parser::FlowParser;
use paladin::core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, StopReason};
use std::collections::HashMap;
use std::sync::Arc;

/// Example mock implementation of PaladinPort
/// In a real application, this would call actual LLM APIs
struct ExamplePaladinPort;

#[async_trait]
impl PaladinPort for ExamplePaladinPort {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        println!("\n🤖 {} executing...", paladin.node.name);
        println!("   Input: {}", input);

        // Simulate processing delay
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Generate output based on Paladin role
        let output = match paladin.node.name.as_str() {
            "intake" => format!(
                "[Intake] Validated and normalized input: \"{}\"",
                input.trim()
            ),
            "analyzer" => format!(
                "[Analyzer] Technical analysis:\n  - Key themes identified\n  - Complexity: Medium\n  - Input: {}",
                input
            ),
            "summarizer" => format!(
                "[Summarizer] Concise summary:\n  - Main point extracted\n  - Length: {} chars\n  - Input: {}",
                input.len(),
                input
            ),
            "reviewer" => format!(
                "[Reviewer] Final synthesis:\n  - Analysis and summary combined\n  - Quality: Approved\n  - Input processed: {}",
                input
            ),
            _ => format!("{} processed: {}", paladin.node.name, input),
        };

        println!("   Output: {}", output);

        Ok(PaladinResult {
            output,
            usage: paladin_ports::output::llm_port::TokenUsage::new(75, 0),
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

/// Helper function to create a Paladin with given name and role
fn create_paladin(name: &str, system_prompt: &str) -> Paladin {
    let data = PaladinData {
        system_prompt: system_prompt.to_string(),
        name: name.to_string(),
        user_name: "User".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(3),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };

    Node::new(data, Some(name.to_string()))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Maneuver Basic Flow DSL Example");
    println!("{}", "=".repeat(60));

    // Step 1: Define the flow using DSL
    println!("\n📝 Step 1: Define Flow Expression");
    let flow_expression = "intake -> (analyzer, summarizer) -> reviewer";
    println!("Flow DSL: {}", flow_expression);

    // Parse the flow
    let flow = FlowParser::parse(flow_expression)?;
    println!("✅ Flow parsed successfully");

    // Step 2: Visualize the flow structure
    println!("\n📊 Step 2: Visualize Flow Structure");
    let ascii_viz = FlowVisualizer::visualize(&flow, VisualizationFormat::Ascii);
    println!("{}", ascii_viz);

    // Step 3: Create Paladins matching flow agent names
    println!("\n🏗️  Step 3: Create Paladins");
    let mut agents = HashMap::new();

    agents.insert(
        "intake".to_string(),
        create_paladin(
            "intake",
            "You are the intake processor. Validate and normalize input data.",
        ),
    );
    println!("   ✓ Created 'intake' paladin");

    agents.insert(
        "analyzer".to_string(),
        create_paladin(
            "analyzer",
            "You are the analyzer. Perform technical analysis and identify key themes.",
        ),
    );
    println!("   ✓ Created 'analyzer' paladin");

    agents.insert(
        "summarizer".to_string(),
        create_paladin(
            "summarizer",
            "You are the summarizer. Create concise summaries of input.",
        ),
    );
    println!("   ✓ Created 'summarizer' paladin");

    agents.insert(
        "reviewer".to_string(),
        create_paladin(
            "reviewer",
            "You are the reviewer. Synthesize analysis and summary into final output.",
        ),
    );
    println!("   ✓ Created 'reviewer' paladin");

    // Step 4: Configure Maneuver
    println!("\n⚙️  Step 4: Configure Maneuver");
    let config = ManeuverConfig::new()
        .with_error_strategy(ErrorStrategy::FailFast)
        .with_output_format(OutputFormat::Concatenate)
        .with_timing_metrics(true);
    println!("   ✓ Error strategy: FailFast");
    println!("   ✓ Output format: Concatenate");
    println!("   ✓ Timing metrics: Enabled");

    // Step 5: Create Maneuver
    println!("\n🎖️  Step 5: Create Maneuver");
    let maneuver = Maneuver::new("document-workflow", agents, flow, config)?;
    println!("   ✓ Maneuver '{}' created", maneuver.name);

    // Step 6: Execute Maneuver
    println!("\n🚀 Step 6: Execute Maneuver");
    println!("{}", "=".repeat(60));

    let paladin_port = Arc::new(ExamplePaladinPort);
    let service = ManeuverExecutionService::new(paladin_port);

    let input = "Analyze the potential impact of quantum computing on cryptography";
    println!("\n📥 Input: {}", input);

    let result = service.execute(&maneuver, input).await?;

    // Step 7: Display Results
    println!("\n{}", "=".repeat(60));
    println!("✅ Execution Complete");
    println!("{}", "=".repeat(60));

    println!("\n📋 Execution Order:");
    for (idx, agent_name) in result.execution_order.iter().enumerate() {
        println!("   {}. {}", idx + 1, agent_name);
    }

    println!("\n⏱️  Timing Metrics:");
    if let Some(metrics) = result.timing_metrics {
        for (agent, duration) in metrics {
            println!("   {} - {}ms", agent, duration.as_millis());
        }
    }

    println!("\n📤 Final Output:");
    println!("{}", "-".repeat(60));
    println!("{}", result.final_output);
    println!("{}", "-".repeat(60));

    println!("\n💡 Key Takeaways:");
    println!("   • Flow DSL enables declarative workflow definition");
    println!("   • Sequential (->) and parallel (,) can be mixed");
    println!("   • Visual feedback helps understand execution flow");
    println!("   • Timing metrics provide performance insights");

    Ok(())
}
