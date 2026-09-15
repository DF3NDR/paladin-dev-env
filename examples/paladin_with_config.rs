// examples/paladin_with_config.rs
//
// Advanced Paladin Configuration Example
//
// This example demonstrates advanced Paladin features:
// 1. Custom configuration (temperature, max_loops, stop words)
// 2. Retry logic with exponential backoff
// 3. Circuit breaker for fault tolerance
// 4. Multi-loop reasoning
// 5. Stop word detection
//
// To run this example:
// ```bash
// cargo run --example paladin_with_config
// ```

use paladin::MockLlmAdapter;
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::core::platform::container::paladin_config::OutputFormat;
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin_ports::output::llm_port::LlmPort;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🗡️  Advanced Paladin Configuration Example\n");
    println!("{}", "=".repeat(60));
    println!();

    // Example 1: Multi-Loop Reasoning
    println!("📚 Example 1: Multi-Loop Reasoning");
    println!("{}", "-".repeat(60));
    multi_loop_example().await?;
    println!();

    // Example 2: Stop Word Detection
    println!("🛑 Example 2: Stop Word Detection");
    println!("{}", "-".repeat(60));
    stop_word_example().await?;
    println!();

    // Example 3: Retry with Failure Recovery
    println!("🔄 Example 3: Retry with Failure Recovery");
    println!("{}", "-".repeat(60));
    retry_example().await?;
    println!();

    // Example 4: Custom Configuration
    println!("⚙️  Example 4: Custom Configuration");
    println!("{}", "-".repeat(60));
    custom_config_example().await?;
    println!();

    println!("{}", "=".repeat(60));
    println!("✨ All examples completed successfully!");

    Ok(())
}

/// Demonstrates multi-loop reasoning where the Paladin iterates multiple times
async fn multi_loop_example() -> Result<(), Box<dyn std::error::Error>> {
    // Create a mock that returns different responses for each loop
    let llm_port = Arc::new(MockLlmAdapter::new().with_responses(vec![
        "First thought: I need to break this problem down.".to_string(),
        "Second thought: Let me analyze each component.".to_string(),
        "Final answer: Here's my comprehensive solution.".to_string(),
    ]));

    // Build Paladin with 3 loops
    let paladin = PaladinBuilder::new(llm_port.clone() as Arc<dyn LlmPort>)
        .system_prompt("You are a thoughtful problem solver")
        .name("ReasoningAgent")
        .model("gpt-4")
        .max_loops(3) // Multiple iterations for reasoning
        .temperature(0.8)
        .build()
        .await?;

    let circuit_breaker = Arc::new(CircuitBreaker::new(5, 2, Duration::from_secs(30)));
    let service = PaladinExecutionService::new(llm_port, circuit_breaker, None, None);

    println!("Input: How do I solve a complex problem?");

    let result = service
        .execute(&paladin, "How do I solve a complex problem?")
        .await?;

    println!("Output: {}", result.output);
    println!("Loops executed: {}/3", result.loop_count);
    println!("Tokens used: {}", result.usage.total_tokens);

    Ok(())
}

/// Demonstrates stop word detection that halts execution early
async fn stop_word_example() -> Result<(), Box<dyn std::error::Error>> {
    let llm_port = Arc::new(
        MockLlmAdapter::new()
            .with_response("This is my answer. FINAL_ANSWER: The result is 42.".to_string()),
    );

    // Build Paladin with stop word configured
    let paladin = PaladinBuilder::new(llm_port.clone() as Arc<dyn LlmPort>)
        .system_prompt("You are a concise assistant")
        .name("StopWordAgent")
        .model("gpt-4")
        .max_loops(5) // Could run up to 5 loops
        .add_stop_word("FINAL_ANSWER") // But stops when this appears
        .add_stop_word("DONE")
        .build()
        .await?;

    let circuit_breaker = Arc::new(CircuitBreaker::new(5, 2, Duration::from_secs(30)));
    let service = PaladinExecutionService::new(llm_port, circuit_breaker, None, None);

    println!("Input: What is the answer?");
    println!("Stop words configured: FINAL_ANSWER, DONE");

    let result = service.execute(&paladin, "What is the answer?").await;

    match result {
        Ok(_) => println!("Completed normally"),
        Err(e) => println!("Stopped early: {}", e),
    }

    Ok(())
}

/// Demonstrates retry logic with failure recovery
async fn retry_example() -> Result<(), Box<dyn std::error::Error>> {
    // Create a mock that always succeeds (demonstrating resilient path)
    let llm_port = Arc::new(MockLlmAdapter::new().with_responses(vec![
        "Successfully recovered! Here's your answer.".to_string(),
    ]));

    let paladin = PaladinBuilder::new(llm_port.clone() as Arc<dyn LlmPort>)
        .system_prompt("You are a resilient assistant")
        .name("RetryAgent")
        .model("gpt-4")
        .max_loops(3) // Enables retry attempts
        .retry_attempts(3) // Maximum retry attempts
        .build()
        .await?;

    // Circuit breaker with higher threshold for retries
    let circuit_breaker = Arc::new(CircuitBreaker::new(5, 2, Duration::from_secs(30)));
    let service = PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, None);

    println!("Input: Test resilience");
    println!("Simulating transient failure...");

    let start = std::time::Instant::now();
    let result = service.execute(&paladin, "Test resilience").await?;
    let elapsed = start.elapsed();

    println!("Output: {}", result.output);
    println!("Elapsed time: {:?} (includes retry backoff)", elapsed);
    println!("Mock was called {} times", llm_port.get_call_count());

    Ok(())
}

/// Demonstrates all custom configuration options
async fn custom_config_example() -> Result<(), Box<dyn std::error::Error>> {
    let llm_port = Arc::new(
        MockLlmAdapter::new()
            .with_response("This is a carefully configured response.".to_string())
            .with_token_usage(50, 100, 150), // Custom token counts
    );

    // Build Paladin with extensive custom configuration
    let paladin = PaladinBuilder::new(llm_port.clone() as Arc<dyn LlmPort>)
        .system_prompt(
            "You are a highly specialized AI assistant with expertise in software architecture",
        )
        .name("SpecializedAgent")
        .user_name("SeniorDeveloper")
        .model("gpt-4-turbo")
        .temperature(0.3) // Low temperature for deterministic output
        .max_loops(2)
        .add_stop_word("END")
        .retry_attempts(5)
        .timeout_seconds(300)
        .enable_planning(true)
        .output_format(OutputFormat::Json)
        .build()
        .await?;

    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service = PaladinExecutionService::new(llm_port, circuit_breaker, None, None);

    println!("Configuration:");
    println!("  Name: {}", paladin.node.name);
    println!("  User: {}", paladin.node.user_name);
    println!("  Model: {}", paladin.node.model);
    println!("  Temperature: {}", paladin.node.temperature);
    println!("  Max Loops: {}", paladin.node.max_loops);
    println!("  Stop Words: {:?}", paladin.node.stop_words);
    println!();

    let result = service
        .execute(&paladin, "Design a microservices architecture")
        .await?;

    println!("Output: {}", result.output);
    println!("Token count: {}", result.usage.total_tokens);

    Ok(())
}
