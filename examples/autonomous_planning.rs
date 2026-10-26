// examples/autonomous_planning.rs
//
// Autonomous Planning Mode Example
//
// This example demonstrates how to use MaxLoops::Auto to enable autonomous planning.
// The Paladin will:
// 1. Analyze the input task for complexity
// 2. Generate a structured plan with subtasks
// 3. Execute each subtask sequentially
// 4. Synthesize results into a final answer
//
// To run this example:
// ```bash
// cargo run --example autonomous_planning
// ```

use paladin::MockLlmAdapter;
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::core::platform::container::herald::Herald;
use paladin::infrastructure::adapters::herald::MarkdownHerald;
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin_ports::output::llm_port::LlmPort;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🗡️  Autonomous Planning Mode Example\n");
    println!("This example shows how agents can automatically decompose complex tasks\n");

    // Create a mock LLM that simulates planning responses
    let llm_port = Arc::new(
        MockLlmAdapter::new()
            // First response: Generate the plan
            .with_response(
                r#"I'll break this analysis into the following steps:

1. **Identify Key Metrics**: Define performance indicators (response time, throughput, error rate)
2. **Collect Data**: Gather performance logs and metrics from monitoring systems
3. **Analyze Trends**: Identify patterns, bottlenecks, and anomalies in the data
4. **Generate Recommendations**: Provide actionable suggestions for optimization
5. **Summarize Findings**: Create executive summary with key insights

Let me execute each step..."#
                    .to_string(),
            )
            // Subsequent responses: Execute each subtask
            .with_response("Subtask 1 complete: Metrics identified".to_string())
            .with_response("Subtask 2 complete: Data collected".to_string())
            .with_response("Subtask 3 complete: Trends analyzed".to_string())
            .with_response("Subtask 4 complete: Recommendations generated".to_string())
            .with_response(
                "Final synthesis: Performance analysis complete with actionable insights"
                    .to_string(),
            ),
    );

    // Build Paladin with autonomous planning enabled
    // Key setting: MaxLoops::Auto
    let paladin = PaladinBuilder::new(llm_port.clone() as Arc<dyn LlmPort>)
        .system_prompt(
            "You are an expert performance analyst specializing in web applications. \
             You decompose complex analysis tasks into clear, sequential steps.",
        )
        .name("PerformanceAnalyst")
        .model("gpt-4")
        .temperature(0.6)
        .max_loops(5) // Note: In fully integrated version, would use MaxLoops::Auto
        .enable_planning(true) // Enables autonomous planning via config
        .build()
        .await?;

    println!("✅ Paladin created with autonomous planning");
    println!("   Name: {}", paladin.data.name);
    println!("   Planning Enabled: true (via config)");
    println!();

    // Create execution service
    let circuit_breaker = Arc::new(CircuitBreaker::new(
        3,
        2,
        Duration::from_secs(60), // Longer timeout for planning
    ));

    let herald: Arc<dyn Herald> = Arc::new(MarkdownHerald::new());
    let service =
        PaladinExecutionService::new(llm_port.clone(), circuit_breaker.clone(), None, None)
            .with_herald(herald);

    // Execute with a complex task
    println!("🚀 Executing complex analysis task...\n");

    let complex_task = "Analyze the performance of our web application and provide \
                       optimization recommendations";

    println!("📝 Input Task:");
    println!("   {}", complex_task);
    println!();

    println!("🔍 Planning Phase:");
    println!("   The Paladin will automatically:");
    println!("   1. Analyze task complexity");
    println!("   2. Generate a structured plan");
    println!("   3. Determine optimal number of loops");
    println!();

    let result = service.execute(&paladin, complex_task).await?;

    println!("✨ Execution Complete!\n");
    println!("📊 Results:");
    println!("   Loops executed: {}", result.loop_count);
    println!("   Execution time: {}ms", result.execution_time_ms);
    println!("   Stop reason: {:?}", result.stop_reason);
    println!();

    println!("📄 Output:");
    println!("{}", result.output);
    println!();

    // Example 2: Simpler task (should require fewer loops)
    println!("\n{}", "=".repeat(70));
    println!("\n🔄 Example 2: Simple Task\n");

    let simple_task = "What is the capital of France?";
    println!("📝 Input Task: {}", simple_task);
    println!("   (Expected: Fewer loops for simple factual query)");
    println!();

    // Create a new mock for simple response
    let simple_llm = Arc::new(
        MockLlmAdapter::new().with_response("The capital of France is Paris.".to_string()),
    );

    let simple_paladin = PaladinBuilder::new(simple_llm.clone() as Arc<dyn LlmPort>)
        .system_prompt("You are a helpful assistant")
        .name("SimpleAssistant")
        .model("gpt-4")
        .max_loops(2) // Simpler task needs fewer loops
        .enable_planning(true)
        .build()
        .await?;

    let simple_service =
        PaladinExecutionService::new(simple_llm, circuit_breaker.clone(), None, None)
            .with_herald(Arc::new(MarkdownHerald::new()));

    let simple_result = simple_service.execute(&simple_paladin, simple_task).await?;

    println!("✅ Execution Complete!");
    println!(
        "   Loops: {} (optimized for simple task)",
        simple_result.loop_count
    );
    println!("   Output: {}", simple_result.output);
    println!();

    println!("💡 Key Takeaways:");
    println!("   • enable_planning(true) activates autonomous planning");
    println!("   • Planning overhead is proportional to task complexity");
    println!("   • Simple tasks use fewer loops automatically");
    println!("   • Complex tasks are broken into manageable subtasks");
    println!("   • Note: Full MaxLoops::Auto integration coming soon");
    println!();

    println!("📚 Learn More:");
    println!("   • See docs/AUTONOMOUS.md for detailed planning documentation");
    println!("   • Configure max_subtasks in PlanningConfig");
    println!("   • Combine with other autonomous features for best results");

    Ok(())
}
