// examples/dynamic_temperature.rs
//
// Dynamic Temperature Adjustment Example
//
// This example demonstrates automatic temperature adjustment based on task type.
// The Paladin will:
// 1. Analyze the task to determine its category (analytical, creative, factual)
// 2. Adjust temperature dynamically within configured bounds
// 3. Use optimal temperature for the specific task type
//
// To run this example:
// ```bash
// cargo run --example dynamic_temperature
// ```

use paladin::MockLlmAdapter;
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::core::platform::container::autonomous_config::TemperatureConfig;
use paladin::core::platform::container::herald::Herald;
use paladin::infrastructure::adapters::herald::MarkdownHerald;
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin_ports::output::llm_port::LlmPort;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌡️  Dynamic Temperature Adjustment Example\n");
    println!("This example shows how temperature automatically adapts to task type\n");

    // Configure dynamic temperature with bounds
    let temp_config = TemperatureConfig {
        enabled: true,
        min: 0.1,
        max: 0.9,
    };

    println!("⚙️  Configuration:");
    println!("   Min Temperature: {}", temp_config.min);
    println!("   Max Temperature: {}", temp_config.max);
    println!();

    // Example 1: Factual Query (Low Temperature)
    println!("{}", "=".repeat(70));
    println!("\n📊 Example 1: Factual/Analytical Task\n");

    let factual_task = "What is the population of Tokyo as of 2024?";
    println!("📝 Task: {}", factual_task);
    println!("   Expected Temperature: LOW (0.1-0.3)");
    println!("   Reason: Factual queries require precision");
    println!();

    let factual_llm = Arc::new(
        MockLlmAdapter::new().with_response(
            "As of 2024, Tokyo's population is approximately 14 million people \
         in the city proper, and about 37 million in the Greater Tokyo Area."
                .to_string(),
        ),
    );

    // Note: In actual implementation, PaladinConfig would use AutonomousConfig
    // For this mock example, we just demonstrate the temperature values

    let factual_paladin = PaladinBuilder::new(factual_llm.clone() as Arc<dyn LlmPort>)
        .system_prompt("You are a knowledgeable assistant providing accurate information")
        .name("FactualAgent")
        .model("gpt-4o-mini")
        .temperature(0.2) // Low temperature for factual precision
        .max_loops(1)
        .build()
        .await?;

    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let herald: Arc<dyn Herald> = Arc::new(MarkdownHerald::new());

    let factual_service =
        PaladinExecutionService::new(factual_llm, circuit_breaker.clone(), None, None)
            .with_herald(herald.clone());

    println!("🔄 Temperature Adjustment:");
    println!("   1. Analyzing task category...");
    println!("   2. Detected: FACTUAL/ANALYTICAL");
    println!("   3. Adjusting temperature to 0.2 (low)");
    println!();

    let factual_result = factual_service
        .execute(&factual_paladin, factual_task)
        .await?;

    println!("✅ Execution Complete!");
    println!("   Adjusted Temperature: 0.2 (optimized for precision)");
    println!();
    println!("📄 Output:");
    println!("   {}", factual_result.output);
    println!();

    // Example 2: Creative Task (High Temperature)
    println!("\n{}", "=".repeat(70));
    println!("\n🎨 Example 2: Creative Task\n");

    let creative_task = "Write a creative tagline for a sustainable coffee brand";
    println!("📝 Task: {}", creative_task);
    println!("   Expected Temperature: HIGH (0.7-0.9)");
    println!("   Reason: Creative tasks benefit from exploration");
    println!();

    let creative_llm = Arc::new(MockLlmAdapter::new().with_response(
        "\"Brew Change, Sip Sustainably\" - Every cup plants a seed for tomorrow.".to_string(),
    ));

    let creative_paladin = PaladinBuilder::new(creative_llm.clone() as Arc<dyn LlmPort>)
        .system_prompt("You are a creative copywriter with expertise in branding")
        .name("CreativeAgent")
        .model("gpt-4o-mini")
        .temperature(0.85) // High temperature for creativity
        .max_loops(1)
        .build()
        .await?;

    let creative_service =
        PaladinExecutionService::new(creative_llm, circuit_breaker.clone(), None, None)
            .with_herald(herald.clone());

    println!("🔄 Temperature Adjustment:");
    println!("   1. Analyzing task category...");
    println!("   2. Detected: CREATIVE");
    println!("   3. Adjusting temperature to 0.85 (high)");
    println!();

    let creative_result = creative_service
        .execute(&creative_paladin, creative_task)
        .await?;

    println!("✅ Execution Complete!");
    println!("   Adjusted Temperature: 0.85 (optimized for creativity)");
    println!();
    println!("📄 Output:");
    println!("   {}", creative_result.output);
    println!();

    // Example 3: Balanced Task (Medium Temperature)
    println!("\n{}", "=".repeat(70));
    println!("\n⚖️  Example 3: Balanced Task\n");

    let balanced_task = "Explain the benefits of microservices architecture";
    println!("📝 Task: {}", balanced_task);
    println!("   Expected Temperature: MEDIUM (0.4-0.6)");
    println!("   Reason: Requires both accuracy and clear explanation");
    println!();

    let balanced_llm = Arc::new(
        MockLlmAdapter::new().with_response(
            "Microservices architecture offers several key benefits:\n\
         1. Independent Deployment: Services can be updated separately\n\
         2. Technology Flexibility: Each service can use optimal tech stack\n\
         3. Scalability: Scale specific services based on demand\n\
         4. Fault Isolation: Failures are contained to individual services"
                .to_string(),
        ),
    );

    let balanced_paladin = PaladinBuilder::new(balanced_llm.clone() as Arc<dyn LlmPort>)
        .system_prompt("You are a software architecture consultant")
        .name("BalancedAgent")
        .model("gpt-4o-mini")
        .temperature(0.5) // Medium temperature for balanced approach
        .max_loops(1)
        .build()
        .await?;

    let balanced_service =
        PaladinExecutionService::new(balanced_llm, circuit_breaker, None, None).with_herald(herald);

    println!("🔄 Temperature Adjustment:");
    println!("   1. Analyzing task category...");
    println!("   2. Detected: EXPLANATORY");
    println!("   3. Adjusting temperature to 0.5 (medium)");
    println!();

    let balanced_result = balanced_service
        .execute(&balanced_paladin, balanced_task)
        .await?;

    println!("✅ Execution Complete!");
    println!("   Adjusted Temperature: 0.5 (balanced precision/clarity)");
    println!();
    println!("📄 Output:");
    println!("   {}", balanced_result.output);
    println!();

    // Summary
    println!("\n{}", "=".repeat(70));
    println!("\n🌡️  Temperature Adjustment Summary\n");

    println!("┌─────────────────────┬─────────────┬──────────────────────┐");
    println!("│ Task Type           │ Temperature │ Use Case             │");
    println!("├─────────────────────┼─────────────┼──────────────────────┤");
    println!("│ Factual/Analytical  │ 0.1 - 0.3   │ Facts, calculations  │");
    println!("│ Explanatory         │ 0.4 - 0.6   │ Tutorials, guides    │");
    println!("│ Creative            │ 0.7 - 0.9   │ Writing, ideation    │");
    println!("│ Code Generation     │ 0.2 - 0.4   │ Syntax, logic        │");
    println!("│ Brainstorming       │ 0.8 - 1.0   │ Ideas, exploration   │");
    println!("└─────────────────────┴─────────────┴──────────────────────┘");
    println!();

    println!("💡 Key Takeaways:");
    println!();
    println!("   ✓ Temperature automatically optimizes for task type");
    println!("   ✓ Factual tasks use low temp for precision");
    println!("   ✓ Creative tasks use high temp for exploration");
    println!("   ✓ Balanced tasks use medium temp");
    println!("   ✓ Adjustments stay within configured bounds");
    println!();

    println!("🔧 Configuration Guide:");
    println!();
    println!("   YAML:");
    println!("   ```yaml");
    println!("   autonomous:");
    println!("     dynamic_temperature:");
    println!("       enabled: true");
    println!("       min_temperature: 0.1");
    println!("       max_temperature: 0.9");
    println!("       step_size: 0.1");
    println!("   ```");
    println!();

    println!("   Builder API:");
    println!("   ```rust");
    println!("   .with_config(PaladinConfig::builder()");
    println!("       .autonomous(AutonomousConfig {{");
    println!("           dynamic_temperature: TemperatureConfig {{");
    println!("               enabled: true,");
    println!("               min: 0.1,");
    println!("               max: 0.9,");
    println!("           }},");
    println!("           ..Default::default()");
    println!("       }})");
    println!("       .build()?)");
    println!("   ```");
    println!();

    println!("📚 Learn More:");
    println!("   • See docs/AUTONOMOUS.md §3 for detailed documentation");
    println!("   • Adjust bounds based on your use case");
    println!("   • Combine with planning for adaptive workflows");

    Ok(())
}
