// examples/basic_paladin.rs
//
// Basic Paladin Example
//
// This example demonstrates the simplest way to create and execute a Paladin agent.
// It shows how to:
// 1. Create a Paladin using the builder pattern
// 2. Execute it with a simple input
// 3. Handle the result
//
// To run this example:
// ```bash
// cargo run --example basic_paladin
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
    // Initialize logging to see what's happening (optional)
    // env_logger::init();

    println!("🗡️  Basic Paladin Example\n");
    println!("Creating a simple Paladin agent...\n");

    // Step 1: Create a mock LLM adapter for this example
    // In production, you would use a real LLM adapter (OpenAI, Anthropic, etc.)
    let llm_port = Arc::new(
        MockLlmAdapter::new()
            .with_response(
                "I am a helpful AI assistant. I can help you with various tasks like answering questions, providing explanations, and assisting with problem-solving. How can I help you today?".to_string()
            )
    );

    // Step 2: Build the Paladin using the fluent builder pattern
    // The builder validates all configuration parameters
    let paladin = PaladinBuilder::new(llm_port.clone() as Arc<dyn LlmPort>)
        .system_prompt("You are a helpful AI assistant")
        .name("BasicAssistant")
        .model("gpt-4")
        .temperature(0.7)
        .max_loops(1) // Single iteration for simplicity
        .build()
        .await?;

    println!("✅ Paladin created successfully!");
    println!("   Name: {}", paladin.node.name);
    println!("   Model: {}", paladin.node.model);
    println!("   Temperature: {}", paladin.node.temperature);
    println!();

    // Step 3: Create an execution service with circuit breaker and Herald
    // The circuit breaker prevents cascading failures
    // The Herald formats output in a human-readable way
    let circuit_breaker = Arc::new(CircuitBreaker::new(
        3,                       // Open after 3 failures
        2,                       // Close after 2 successes
        Duration::from_secs(30), // 30 second timeout
    ));

    // Create a Markdown Herald for beautiful formatted output
    let herald: Arc<dyn Herald> = Arc::new(MarkdownHerald::new());

    let service =
        PaladinExecutionService::new(llm_port, circuit_breaker, None, None).with_herald(herald);

    // Step 4: Execute the Paladin with your input
    println!("🚀 Executing Paladin...\n");

    let input = "What can you help me with?";
    println!("Input: {}", input);
    println!();

    let result = service.execute(&paladin, input).await?;

    // Step 5: Display the results
    println!("📊 Execution Results:");
    println!("   Output: {}", result.output);
    println!("   Loops: {}", result.loop_count);
    println!("   Tokens: {}", result.usage.total_tokens);
    println!("   Time: {}ms", result.execution_time_ms);
    println!("   Stop Reason: {:?}", result.stop_reason);
    println!();

    // Step 6: Format with Herald for beautiful output
    println!("📝 Formatted Output (Markdown Herald):");
    println!("{}", "─".repeat(60));
    if let Some(formatted) = service.format_result(&result, &paladin)? {
        println!("{}", formatted);
    }
    println!("{}", "─".repeat(60));
    println!();

    println!("✨ Example completed successfully!");
    println!("\n💡 Try other Herald formatters:");
    println!("   - JsonHerald for structured JSON output");
    println!("   - TableHerald for compact table format");
    println!("   - Or create your own custom formatter!");

    Ok(())
}
