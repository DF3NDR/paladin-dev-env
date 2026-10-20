// examples/llm_provider_selection.rs
//
// LLM Provider Selection Example
//
// This example demonstrates how to select and configure different LLM providers
// (OpenAI, DeepSeek, Anthropic) based on your needs.
//
// To run this example:
// ```bash
// cargo run --example llm_provider_selection
// ```

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🗡️  LLM Provider Selection Guide\n");
    println!("{}\n", "=".repeat(80));

    // Overview of available providers
    println!("📊 Available LLM Providers in Paladin:\n");
    println!("1. OpenAI (GPT-4, GPT-3.5-turbo)");
    println!("   • Strengths: Mature ecosystem, extensive model options, vision support");
    println!("   • Best for: General-purpose AI, production deployments");
    println!("   • Configuration: Requires OPENAI_API_KEY\n");

    println!("2. DeepSeek");
    println!("   • Strengths: Cost-effective, strong reasoning, high throughput");
    println!("   • Best for: Code generation, logical analysis, cost-sensitive apps");
    println!("   • Configuration: Requires DEEPSEEK_API_KEY\n");

    println!("3. Anthropic Claude");
    println!("   • Strengths: Safety-focused, excellent for analysis, long context");
    println!("   • Best for: Safety-critical apps, complex document analysis");
    println!("   • Configuration: Requires ANTHROPIC_API_KEY\n");

    println!("{}\n", "=".repeat(80));

    // Show configuration patterns
    println!("⚙️  Configuration Patterns:\n");

    show_deepseek_config();
    show_anthropic_config();
    show_openai_config();

    println!("{}\n", "=".repeat(80));

    // Show capability-based selection
    println!("🎯 Selecting Providers Based on Capabilities:\n");

    println!("✓ Need vision/image analysis?");
    println!("  → Use OpenAI (GPT-4 Vision)\n");

    println!("✓ Need cost-effective high throughput?");
    println!("  → Use DeepSeek\n");

    println!("✓ Need strong safety guarantees?");
    println!("  → Use Anthropic Claude\n");

    println!("✓ Need tool calling/function execution?");
    println!("  → All providers support tool calling\n");

    println!("✓ Need streaming responses?");
    println!("  → All providers support streaming\n");

    println!("{}\n", "=".repeat(80));

    // Practical usage example
    show_practical_example();

    println!("{}\n", "=".repeat(80));
    println!("✨ Provider selection guide complete!\n");
    println!("💡 See docs/PROVIDER_EXPANSION.md for detailed comparison");
    println!("📖 See docs/CONTRIBUTING_PROVIDERS.md to add new providers");

    Ok(())
}

fn show_deepseek_config() {
    println!("📝 DeepSeek Configuration:");
    println!();
    println!("   // Option 1: From environment variables");
    println!("   let config = DeepSeekConfig::from_env()?;");
    println!("   let adapter = DeepSeekAdapter::new(config)?;");
    println!();
    println!("   // Option 2: Custom configuration");
    println!("   let config = DeepSeekConfig::new(api_key, \"deepseek-flash\".to_string())");
    println!("       .with_timeout_seconds(600); // endpoint: .with_base_url(..)");
    println!("   let adapter = DeepSeekAdapter::new(config)?;");
    println!();
}

fn show_anthropic_config() {
    println!("📝 Anthropic Configuration:");
    println!();
    println!("   // Option 1: From environment variables");
    println!("   let config = AnthropicConfig::from_env()?;");
    println!("   let adapter = AnthropicAdapter::new(config)?;");
    println!();
    println!("   // Option 2: Custom configuration");
    println!("   let config = AnthropicConfig::new(");
    println!("       api_key,");
    println!("       \"https://api.anthropic.com/v1\".to_string(),");
    println!("       \"claude-3-5-sonnet-20241022\".to_string()");
    println!("   );");
    println!("   let adapter = AnthropicAdapter::new(config)?;");
    println!();
}

fn show_openai_config() {
    println!("📝 OpenAI Configuration:");
    println!();
    println!("   // Option 1: From environment");
    println!("   let adapter = OpenAIAdapter::from_env()?;");
    println!();
    println!("   // Option 2: Custom configuration");
    println!("   let config = OpenAIConfig::new(");
    println!("       api_key,");
    println!("       \"gpt-5.4-nano\".to_string()");
    println!("   )");
    println!("   .with_base_url(\"https://api.openai.com/v1\");");
    println!("   let adapter = OpenAIAdapter::new(config)?;");
    println!();
}

fn show_practical_example() {
    println!("💡 Practical Usage Example:\n");
    println!("   // Create provider based on configuration");
    println!("   let llm_port: Arc<dyn LlmPort> = ");
    println!("       if let Ok(config) = DeepSeekConfig::from_env() {{");
    println!("           Arc::new(DeepSeekAdapter::new(config)?)");
    println!("       }} else if let Ok(config) = AnthropicConfig::from_env() {{");
    println!("           Arc::new(AnthropicAdapter::new(config)?)");
    println!("       }} else {{");
    println!("           // Fallback to OpenAI");
    println!("           let config = OpenAIConfig::from_env()?;");
    println!("           Arc::new(OpenAIAdapter::new(config)?)");
    println!("       }};");
    println!();
    println!("   // Use with Paladin builder");
    println!("   let paladin = PaladinBuilder::new(llm_port)");
    println!("       .system_prompt(\"You are a helpful assistant\")");
    println!("       .name(\"MyPaladin\")");
    println!("       .build().await?;");
}
