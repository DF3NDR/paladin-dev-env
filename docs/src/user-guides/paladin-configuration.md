# Paladin Configuration Guide

This guide covers how to configure Paladin agents for optimal performance, from basic setup to advanced tuning.

## Table of Contents

- [Basic Configuration](#basic-configuration)
- [System Prompt Best Practices](#system-prompt-best-practices)
- [Model Selection](#model-selection)
- [Temperature and Sampling](#temperature-and-sampling)
- [Stop Words and Termination](#stop-words-and-termination)
- [Timeout and Retry Settings](#timeout-and-retry-settings)
- [Advanced Configuration](#advanced-configuration)

## Basic Configuration

### Minimal Setup

```rust,ignore
use paladin::prelude::*;

let paladin = PaladinBuilder::new(llm_adapter)
    .name("Assistant")
    .system_prompt("You are a helpful assistant.")
    .build().await?;
```

### Common Configuration

```rust,ignore
let paladin = PaladinBuilder::new(llm_adapter)
    .name("DataAnalyst")
    .system_prompt("You are an expert data analyst. Provide clear, data-driven insights.")
    .model("gpt-4")
    .temperature(0.7)
    .max_loops(5)
    .timeout_seconds(120)
    .build().await?;
```

### Full Configuration

`PaladinBuilder` has no `add_armament()` method; tool registration goes through
`.with_arsenal_registry(registry)` (`src/application/services/paladin/paladin_builder.rs:686`) --
see [Tool Integration](tool-integration.md#custom-tool-development) for the registry-wiring detail.

```rust,ignore
let paladin = PaladinBuilder::new(llm_adapter)
    .name("ResearchAssistant")
    .system_prompt("You are a research assistant specializing in academic papers.")
    .user_name("Researcher")
    .model("gpt-4-turbo")
    .temperature(0.8)
    .max_loops(10)
    .add_stop_word("END").add_stop_word("STOP").add_stop_word("FINAL_ANSWER")
    .timeout_seconds(300)
    .retry_attempts(3)
    .with_garrison(garrison)
    .with_arsenal_registry(arsenal_registry)
    .build().await?;
```

## System Prompt Best Practices

The system prompt defines your Paladin's behavior and capabilities. Follow these best practices:

### 1. Be Specific About Role

**❌ Vague:**
```rust,ignore
.system_prompt("You are helpful.")
```

**✅ Specific:**
```rust,ignore
.system_prompt("You are a senior software engineer specializing in Rust. \
                You provide code reviews focused on safety, performance, and idiomatic patterns.")
```

### 2. Define Output Format

```rust,ignore
.system_prompt("You are a JSON API. Always respond with valid JSON. \
                Structure: {\"status\": \"success|error\", \"data\": {...}, \"message\": \"...\"}  \
                Never include markdown code blocks or explanations outside the JSON.")
```

### 3. Set Boundaries

```rust,ignore
.system_prompt("You are a customer support agent for TechCorp. \
                - Only answer questions about our products and services \
                - Escalate billing questions to the finance team \
                - Do not provide medical, legal, or financial advice \
                - Be polite and professional at all times")
```

### 4. Include Examples (Few-Shot)

```rust,ignore
.system_prompt("You categorize customer feedback as: FEATURE_REQUEST, BUG_REPORT, or PRAISE. \
                \
                Examples: \
                Input: 'The app crashes when I upload large files' \
                Output: BUG_REPORT \
                \
                Input: 'It would be great to have dark mode' \
                Output: FEATURE_REQUEST \
                \
                Input: 'Love the new design!' \
                Output: PRAISE")
```

### 5. Specify Tone and Style

```rust,ignore
.system_prompt("You are a technical writer creating documentation for developers. \
                - Use clear, concise language \
                - Prefer active voice \
                - Include code examples \
                - Target audience: junior to mid-level developers \
                - Avoid jargon unless necessary")
```

## Model Selection

Choose the right model for your use case:

### OpenAI Models

```rust,ignore
// GPT-4 Turbo - Best for complex reasoning
.model("gpt-4-turbo")  // Latest turbo model
.model("gpt-4")        // Standard GPT-4

// GPT-3.5 - Fast and cost-effective
.model("gpt-3.5-turbo")  // Recommended for most tasks
```

**When to use:**
- **GPT-4**: Complex reasoning, code generation, detailed analysis
- **GPT-3.5**: Simple queries, classification, summarization

### DeepSeek Models

```rust,ignore
// DeepSeek Chat - Strong coding capabilities
.model("deepseek-chat")

// DeepSeek Coder - Specialized for code
.model("deepseek-coder")
```

**When to use:**
- **deepseek-chat**: General purpose, good for multi-turn conversations
- **deepseek-coder**: Code generation, technical documentation

### Anthropic Models

```rust,ignore
// Claude 3 Family
.model("claude-3-opus")    // Most capable
.model("claude-3-sonnet")  // Balanced
.model("claude-3-haiku")   // Fastest
```

**When to use:**
- **Opus**: Complex analysis, long documents, creative writing
- **Sonnet**: General purpose, good balance of speed and quality
- **Haiku**: Fast responses, simple queries, high throughput

### Model Comparison

| Model | Speed | Cost | Quality | Max Tokens | Best For |
|-------|-------|------|---------|------------|----------|
| GPT-4 Turbo | Medium | High | Excellent | 128K | Complex reasoning |
| GPT-3.5 Turbo | Fast | Low | Good | 16K | Simple tasks |
| Claude 3 Opus | Medium | High | Excellent | 200K | Long documents |
| Claude 3 Sonnet | Fast | Medium | Very Good | 200K | General purpose |
| Claude 3 Haiku | Very Fast | Low | Good | 200K | High throughput |
| DeepSeek Chat | Fast | Very Low | Good | 64K | Cost-sensitive |
| DeepSeek Coder | Fast | Very Low | Very Good | 64K | Code generation |

## Temperature and Sampling

Temperature controls randomness in responses:

### Temperature Scale

```rust,ignore
// 0.0 - Deterministic, focused (best for factual tasks)
.temperature(0.0)

// 0.3-0.5 - Slightly varied (good for classification)
.temperature(0.4)

// 0.7 - Balanced (general purpose)
.temperature(0.7)

// 0.9-1.0 - Creative, diverse (brainstorming, creative writing)
.temperature(0.9)

// >1.0 - Very random (experimental, not recommended)
.temperature(1.2)
```

### Use Cases by Temperature

| Temperature | Use Case | Example |
|-------------|----------|---------|
| 0.0 - 0.3 | Factual, deterministic | Math, code review, data extraction |
| 0.4 - 0.6 | Balanced, consistent | Customer support, Q&A, summarization |
| 0.7 - 0.8 | Creative, natural | Content generation, conversation |
| 0.9 - 1.0 | Highly creative | Brainstorming, storytelling, poetry |

### Example: Task-Specific Configuration

```rust,ignore
// Code Review - Deterministic
let code_reviewer = PaladinBuilder::new(llm_adapter)
    .system_prompt("Review Rust code for safety and best practices.")
    .temperature(0.2)
    .build().await?;

// Content Writer - Creative
let writer = PaladinBuilder::new(llm_adapter)
    .system_prompt("Write engaging blog posts about technology.")
    .temperature(0.9)
    .build().await?;

// Customer Support - Balanced
let support = PaladinBuilder::new(llm_adapter)
    .system_prompt("Help customers with product questions.")
    .temperature(0.7)
    .build().await?;
```

## Stop Words and Termination

Control when a Paladin stops generating:

### Basic Stop Words

```rust,ignore
let paladin = PaladinBuilder::new(llm_adapter)
    .add_stop_word("END").add_stop_word("STOP").add_stop_word("###")
    .build().await?;
```

### Use Cases

#### 1. Structured Output

```rust,ignore
// Stop at delimiter for parsing
.system_prompt("Generate a list of items. End with '---'")
.add_stop_word("---")
```

#### 2. Multi-Step Reasoning

```rust,ignore
// Stop when final answer is reached
.system_prompt("Think step by step. When done, output FINAL_ANSWER: <answer>")
.add_stop_word("FINAL_ANSWER:")
```

#### 3. Dialog Systems

```rust,ignore
// Stop at turn boundaries
.system_prompt("You are user A in a conversation. End each turn with [END_TURN]")
.add_stop_word("[END_TURN]")
```

### Max Loops

Prevent infinite reasoning loops:

```rust,ignore
// Default: 3 loops
.max_loops(3)

// For simple tasks: 1 loop
.max_loops(1)

// For complex reasoning: 10+ loops
.max_loops(15)
```

**What is a loop?**
A loop is one reasoning cycle: prompt → LLM → response → (optional tool calls) → repeat.

## Timeout and Retry Settings

### Timeout Configuration

```rust,ignore
use std::time::Duration;

let paladin = PaladinBuilder::new(llm_adapter)
    .timeout_seconds(60)  // 60 second timeout
    .build().await?;
```

**Recommended Timeouts:**
- Simple queries: 30 seconds
- Complex reasoning: 120 seconds
- With tool calls: 300 seconds

### Retry Configuration

```rust,ignore
let paladin = PaladinBuilder::new(llm_adapter)
    .retry_attempts(3)                        // Retry up to 3 times
    .build().await?;
```

### Error Handling

Reaching `max_loops` is not an error -- there is no `PaladinError::MaxLoopsExceeded` variant.
Execution returns `Ok(PaladinResult { stop_reason: StopReason::MaxLoops, .. })` instead
(`crates/paladin-core/src/platform/container/execution_result.rs:38`); check `stop_reason` on the
success path rather than matching an `Err` arm for it:

```rust,ignore
match paladin.execute(input).await {
    Ok(response) if response.stop_reason == StopReason::MaxLoops => {
        eprintln!("Max reasoning loops exceeded");
        // Increase max_loops or refine system prompt
    }
    Ok(response) => println!("Success: {}", response.content),
    Err(PaladinError::Timeout(secs)) => {
        eprintln!("Request timed out after {} seconds", secs);
        // Increase timeout or simplify prompt
    }
    Err(PaladinError::LlmFailure { transience, provider, message, .. }) => {
        eprintln!("LLM error ({:?}, provider {:?}): {}", transience, provider, message);
        // A `Transient` failure is worth retrying; for a `Permanent` one check
        // the API key, rate limits and model availability
    }
    Err(e) => eprintln!("Other error: {}", e),
}
```

## Advanced Configuration

### Configuration from File

There is no `paladin:` top-level section in `config.yml`, and no `ApplicationSettings` type or
`PaladinBuilder::from_config()` method (`src/config/settings.rs` -- the real top-level type is
`Settings`, with no `paladin` field). A single Paladin is always configured via the
`PaladinBuilder` fluent API shown throughout this guide, not loaded from a config-file section.

The one config-driven path that does exist is different in shape and purpose: the HTTP service
host (`paladin-server`) loads a *list* of agents from `Settings.agents: Vec<AgentDefinition>`
(`src/config/agents.rs:209`), keyed by `id` rather than `name`, and with a narrower field set
(`id`, `model`, `system_prompt`, optional `provider`/`temperature`/`max_loops`/`timeout_seconds`,
`stop_words`, `allowed_roles` -- no `retry_attempts`):

```rust,ignore
use paladin::config::Settings;

let config = Settings::load_from_file("config.yml")?;
// config.agents: Vec<AgentDefinition> -- the HTTP service host's agent registry
```

`config.yml`:
```yaml
agents:
  - id: "assistant"
    model: "gpt-4"
    system_prompt: "You are a helpful assistant."
    temperature: 0.7
    max_loops: 5
    timeout_seconds: 120
    stop_words:
      - "END"
      - "STOP"
```

### Environment-Based Configuration

```rust,ignore
let model = std::env::var("PALADIN_MODEL").unwrap_or("gpt-3.5-turbo".to_string());
let temperature = std::env::var("PALADIN_TEMPERATURE")
    .ok()
    .and_then(|s| s.parse::<f32>().ok())
    .unwrap_or(0.7);

let paladin = PaladinBuilder::new(llm_adapter)
    .model(&model)
    .temperature(temperature)
    .build().await?;
```

### Dynamic Configuration

`PaladinBuilder::build()` is `async` (`paladin_builder.rs:1267`), so a synchronous factory method
cannot return its result directly -- both methods below need `async fn` and `.await`:

```rust,ignore
struct PaladinFactory;

impl PaladinFactory {
    async fn create_for_task(task_type: &str, llm_adapter: Arc<dyn LlmPort>) -> Result<Paladin, PaladinError> {
        match task_type {
            "code_review" => Self::create_code_reviewer(llm_adapter).await,
            "creative_writing" => Self::create_writer(llm_adapter).await,
            "data_analysis" => Self::create_analyst(llm_adapter).await,
            _ => Self::create_default(llm_adapter).await,
        }
    }

    async fn create_code_reviewer(llm_adapter: Arc<dyn LlmPort>) -> Result<Paladin, PaladinError> {
        PaladinBuilder::new(llm_adapter)
            .system_prompt("Expert Rust code reviewer")
            .temperature(0.2)
            .model("gpt-4")
            .build()
            .await
    }

    // ... other factory methods
}
```

### Configuration Validation

Validation happens inside `.build()` (`PaladinBuilder::validate`,
`src/application/services/paladin/paladin_builder.rs:1116`, private to the builder) -- `build()`
itself is the only validation entry point. There is no public `paladin.validate()` on the built
`Paladin` for re-validating after construction:

```rust,ignore
let paladin = PaladinBuilder::new(llm_adapter)
    .temperature(0.7)
    .build()
    .await;  // Validates configuration as part of build()

if let Err(e) = paladin {
    eprintln!("Invalid configuration: {}", e);
}
```

## Configuration Checklist

Before deploying a Paladin, verify:

- [ ] System prompt is clear and specific
- [ ] Appropriate model selected for task
- [ ] Temperature suitable for use case (0.2 for factual, 0.9 for creative)
- [ ] Max loops set appropriately (1-3 for simple, 10+ for complex)
- [ ] Timeout configured (30-300 seconds)
- [ ] Retry logic in place for production
- [ ] Stop words defined if needed
- [ ] Error handling implemented
- [ ] Configuration tested with sample inputs

## Performance Tuning

### For Throughput

```rust,ignore
// Fast model, simple prompts
let paladin = PaladinBuilder::new(llm_adapter)
    .model("gpt-3.5-turbo")
    .temperature(0.7)
    .max_loops(1)
    .timeout_seconds(30)
    .build().await?;
```

### For Quality

```rust,ignore
// Best model, detailed prompts
let paladin = PaladinBuilder::new(llm_adapter)
    .model("gpt-4")
    .temperature(0.5)
    .max_loops(10)
    .timeout_seconds(300)
    .build().await?;
```

### For Cost Efficiency

```rust,ignore
// Cheaper model, efficient prompts
let paladin = PaladinBuilder::new(llm_adapter)
    .model("deepseek-chat")
    .temperature(0.7)
    .max_loops(3)
    .build().await?;
```

## Next Steps

- **[Battalion Patterns](battalion-patterns.md)** - Multi-agent orchestration
- **[Tool Integration](tool-integration.md)** - Add capabilities with Arsenal
- **[Memory Management](memory-management.md)** - Use Garrison for context
- **[Examples](https://github.com/DF3NDR/paladin-dev-env/tree/main/examples)** - See configuration in action

## Related Documentation

- [Quickstart Guide](../getting-started/quickstart.md)
- [API Reference](https://docs.rs/paladin)
- [Performance Tuning](../operations/performance-tuning.md)
