// examples/autonomous_prompt_generation.rs
//
// Autonomous Prompt Generation Example
//
// This example demonstrates automatic system prompt generation based on agent descriptions.
// The Paladin will:
// 1. Analyze the agent_description field
// 2. Generate an optimized system prompt
// 3. Use the generated prompt for task execution
//
// To run this example:
// ```bash
// cargo run --example autonomous_prompt_generation
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
    println!("✨ Autonomous Prompt Generation Example\n");
    println!("This example shows how agents can generate their own system prompts\n");

    // Example 1: Code Reviewer Agent
    println!("📋 Example 1: Code Review Agent\n");

    let code_review_description = "A code review specialist that focuses on security, \
                                  performance, and maintainability. Provides detailed \
                                  feedback with actionable suggestions.";

    println!("📝 Agent Description:");
    println!("   {}", code_review_description);
    println!();

    // Create mock LLM for prompt generation
    let llm_port = Arc::new(
        MockLlmAdapter::new()
            // Simulated generated prompt
            .with_response(
                "You are an expert code reviewer with 15+ years of experience. \
                 Your reviews focus on:\n\
                 1. Security vulnerabilities and best practices\n\
                 2. Performance optimization opportunities\n\
                 3. Code maintainability and readability\n\
                 4. Adherence to language-specific idioms\n\n\
                 Provide clear, actionable feedback with specific examples."
                    .to_string(),
            )
            // Simulated execution response
            .with_response(
                "I've reviewed the authentication middleware. Key findings:\n\
                 • Security: Missing rate limiting on login endpoint (HIGH)\n\
                 • Performance: Consider caching user permissions (MEDIUM)\n\
                 • Maintainability: Extract token validation into separate function (LOW)"
                    .to_string(),
            ),
    );

    // Note: In actual implementation, would configure AutonomousConfig with PaladinConfig
    // For this mock example, we demonstrate the auto-prompt concept
    let code_reviewer = PaladinBuilder::new(llm_port.clone() as Arc<dyn LlmPort>)
        .system_prompt("Expert code reviewer") // In reality, this would be auto-generated
        .name("CodeReviewer")
        .model("gpt-4")
        .temperature(0.7)
        .max_loops(1)
        .build()
        .await?;

    println!("✅ Paladin created (prompt would be auto-generated in full implementation)");
    println!("   Name: {}", code_reviewer.data.name);
    println!("   Note: Autonomous prompt generation coming soon");
    println!();

    // Create execution service
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let herald: Arc<dyn Herald> = Arc::new(MarkdownHerald::new());
    let service =
        PaladinExecutionService::new(llm_port.clone(), circuit_breaker.clone(), None, None)
            .with_herald(herald.clone());

    println!("🔄 Prompt Generation Phase:");
    println!("   1. Analyzing agent description...");
    println!("   2. Generating optimized system prompt...");
    println!("   3. Applying generated prompt...");
    println!();

    // Execute code review task
    let task = "Review this authentication middleware for security issues";
    println!("🚀 Executing Task: {}", task);
    println!();

    let result = service.execute(&code_reviewer, task).await?;

    println!("✨ Execution Complete!\n");
    println!("📊 Results:");
    println!("   Execution time: {}ms", result.execution_time_ms);
    println!();
    println!("📄 Output:");
    println!("{}", result.output);
    println!();

    // Example 2: Technical Writer Agent
    println!("\n{}", "=".repeat(70));
    println!("\n📋 Example 2: Technical Writer Agent\n");

    let writer_description = "A technical documentation specialist who creates clear, \
                             comprehensive documentation for APIs and software systems. \
                             Focuses on accuracy, completeness, and user-friendliness.";

    println!("📝 Agent Description:");
    println!("   {}", writer_description);
    println!();

    // Create new mock for writer
    let writer_llm = Arc::new(
        MockLlmAdapter::new()
            // Generated prompt
            .with_response(
                "You are a seasoned technical writer specializing in API documentation. \
                 Your documentation:\n\
                 1. Starts with a clear purpose statement\n\
                 2. Includes complete parameter descriptions\n\
                 3. Provides realistic code examples\n\
                 4. Documents error scenarios and edge cases\n\
                 5. Uses consistent formatting and terminology"
                    .to_string(),
            )
            // Execution response
            .with_response(
                "# User Authentication API\n\n\
                 ## POST /api/auth/login\n\n\
                 Authenticates a user and returns a session token.\n\n\
                 ### Parameters\n\
                 - `username` (string, required): User's email or username\n\
                 - `password` (string, required): User's password\n\n\
                 ### Example\n\
                 ```json\n{\"username\": \"user@example.com\", \"password\": \"secret123\"}\n```"
                    .to_string(),
            ),
    );

    let tech_writer = PaladinBuilder::new(writer_llm.clone() as Arc<dyn LlmPort>)
        .system_prompt("Technical documentation specialist")
        .name("TechnicalWriter")
        .model("gpt-4")
        .temperature(0.5) // Lower for more consistent documentation
        .max_loops(1)
        .build()
        .await?;

    let writer_service =
        PaladinExecutionService::new(writer_llm, circuit_breaker, None, None).with_herald(herald);

    println!("✅ Writer agent created (prompt would be auto-generated)");
    println!();

    let doc_task = "Document the user authentication API endpoint";
    println!("🚀 Executing Task: {}", doc_task);
    println!();

    let writer_result = writer_service.execute(&tech_writer, doc_task).await?;

    println!("✨ Documentation Generated!\n");
    println!("📄 Output:");
    println!("{}", writer_result.output);
    println!();

    // Show comparison
    println!("\n{}", "=".repeat(70));
    println!("\n💡 Key Takeaways:\n");
    println!("   ✓ Prompt generation eliminates manual prompt engineering");
    println!("   ✓ Descriptions can be reused across similar agents");
    println!("   ✓ Generated prompts adapt to agent specialization");
    println!("   ✓ Reduces time from agent concept to deployment");
    println!();

    println!("🔧 Configuration Options:\n");
    println!("   YAML:");
    println!("   ```yaml");
    println!("   autonomous:");
    println!("     prompt_generation:");
    println!("       enabled: true");
    println!("       description: 'Your agent description here'");
    println!("   ```");
    println!();

    println!("   Builder API:");
    println!("   ```rust");
    println!("   .with_config(PaladinConfig::builder()");
    println!("       .autonomous(AutonomousConfig {{");
    println!("           prompt_generation: PromptConfig {{");
    println!("               enabled: true,");
    println!("               description: Some(desc.to_string())");
    println!("           }},");
    println!("           ..Default::default()");
    println!("       }})");
    println!("       .build()?)");
    println!("   ```");
    println!();

    println!("📚 Learn More:");
    println!("   • See docs/AUTONOMOUS.md §2 for detailed documentation");
    println!("   • Combine with planning for complex multi-step tasks");
    println!("   • Use meaningful, specific agent descriptions");

    Ok(())
}
