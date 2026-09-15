//! Formation Sequential Execution Example
//!
//! Demonstrates how to use Formation pattern to orchestrate multiple Paladins
//! in a sequential pipeline where output from one feeds into the next.
//!
//! Run with: cargo run --example formation_sequential

use async_trait::async_trait;
use paladin::application::services::battalion::formation_service::FormationExecutionService;
use paladin::application::services::paladin::error::PaladinError;
use paladin::core::base::entity::node::Node;
use paladin::core::platform::container::battalion::formation::Formation;
use paladin::core::platform::container::battalion::{BattalionConfig, ErrorStrategy};
use paladin::core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus};
use paladin::core::platform::container::token_usage::TokenUsage;
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, StopReason};
use std::sync::Arc;

/// Example mock implementation of PaladinPort
/// In a real application, this would call actual LLM APIs
struct ExamplePaladinPort;

#[async_trait]
impl PaladinPort for ExamplePaladinPort {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        println!("\n🤖 {} executing...", paladin.node.name);
        println!("   Input: {}", input);

        // Simulate processing
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        // Generate output based on Paladin role
        let output = match paladin.node.name.as_str() {
            "Researcher" => format!(
                "Research findings on: {}\n- Key concept identified\n- Multiple perspectives explored",
                input
            ),
            "Analyst" => format!(
                "Analysis of {}:\n- Pattern recognition completed\n- Insights extracted",
                input
            ),
            "Summarizer" => format!(
                "Summary: {}\n- Concise overview prepared\n- Key takeaways highlighted",
                input
            ),
            _ => format!("{} processed: {}", paladin.node.name, input),
        };

        println!("   Output: {}", output);

        Ok(PaladinResult {
            output,
            usage: TokenUsage::new(100, 0),
            execution_time_ms: 500,
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
    println!("🎯 Formation Sequential Execution Example");
    println!("==========================================\n");

    // Example 1: Research Pipeline
    println!("📚 Example 1: Research → Analysis → Summary Pipeline\n");

    let researcher = create_paladin(
        "Researcher",
        "You are a thorough researcher who explores topics deeply",
    );

    let analyst = create_paladin(
        "Analyst",
        "You are a data analyst who identifies patterns and insights",
    );

    let summarizer = create_paladin("Summarizer", "You create concise, actionable summaries");

    let config = BattalionConfig::new("research_pipeline")
        .with_description("Research → Analysis → Summary workflow")
        .with_timeout(60); // 60 second timeout

    let formation = Formation::new(vec![researcher, analyst, summarizer], config)?;

    let paladin_port = Arc::new(ExamplePaladinPort);
    let service = FormationExecutionService::new(paladin_port.clone());

    let result = service
        .execute(&formation, "Artificial General Intelligence")
        .await?;

    println!("\n✅ Formation completed successfully!");
    println!("   Paladins executed: {}", result.paladin_results.len());
    println!("   Total duration: {:.2}s", result.duration().as_secs_f64());
    println!("   Final output: {}", result.final_output);

    // Example 2: With Shared Context
    println!("\n\n📝 Example 2: Formation with Shared Context\n");

    let writer = create_paladin("Writer", "You write initial drafts");
    let editor = create_paladin("Editor", "You edit and refine text");
    let reviewer = create_paladin("Reviewer", "You provide final review");

    let formation_with_context = Formation::new(
        vec![writer, editor, reviewer],
        BattalionConfig::new("content_pipeline"),
    )?
    .with_shared_context(
        "Project: Technical Documentation\nTone: Professional\nAudience: Developers".to_string(),
    );

    let result2 = service
        .execute(&formation_with_context, "Explain Rust ownership")
        .await?;

    println!("\n✅ Formation with context completed!");
    println!("   Final output: {}", result2.final_output);

    // Example 3: Error Handling
    println!("\n\n⚠️  Example 3: Error Handling Strategies\n");
    println!("Demonstrating ContinueOnError strategy...");

    let step1 = create_paladin("Step1", "First processing step");
    let step2 = create_paladin("Step2", "Second processing step");
    let step3 = create_paladin("Step3", "Final processing step");

    let resilient_config = BattalionConfig::new("resilient_pipeline")
        .with_error_strategy(ErrorStrategy::ContinueOnError);

    let resilient_formation = Formation::new(vec![step1, step2, step3], resilient_config)?;

    let result3 = service
        .execute(&resilient_formation, "Process this data")
        .await?;

    println!("✅ Resilient formation completed despite potential errors!");
    println!("   Results collected: {}", result3.paladin_results.len());

    println!("\n\n🎉 All examples completed successfully!");
    println!("=====================================");
    println!("\nKey Takeaways:");
    println!("1. Formation executes Paladins sequentially (A → B → C)");
    println!("2. Output from each Paladin becomes input for the next");
    println!("3. Shared context can be injected for all Paladins");
    println!("4. Error strategies control failure handling");
    println!("5. Timeout enforcement prevents runaway executions");

    Ok(())
}
