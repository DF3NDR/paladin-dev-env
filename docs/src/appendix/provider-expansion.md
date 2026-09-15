# LLM Provider Expansion Guide

**Paladin Multi-Provider Support**

This document provides a comprehensive comparison of LLM providers supported by Paladin and guidance for configuring and using them effectively.

---

## Table of Contents

- [Overview](#overview)
- [Provider Comparison](#provider-comparison)
- [Configuration Guide](#configuration-guide)
- [Use Case Recommendations](#use-case-recommendations)
- [Migration Guide](#migration-guide)
- [Performance Characteristics](#performance-characteristics)

---

## Overview

Paladin supports multiple LLM providers out of the box, allowing you to choose the best provider for your specific needs. All providers implement the same `LlmPort` trait, making it easy to switch between them without changing your application logic.

### Supported Providers

1. **OpenAI** (GPT-4, GPT-3.5-turbo, GPT-4 Vision)
2. **DeepSeek** (DeepSeek-Chat, DeepSeek-Coder)
3. **Anthropic** (Claude 3.5 Sonnet, Claude 3 Opus/Haiku)

---

## Provider Comparison

| Feature | OpenAI | DeepSeek | Anthropic |
|---------|--------|----------|-----------|
| **Streaming** | ✅ Yes | ✅ Yes | ✅ Yes |
| **Tool Calling** | ✅ Yes | ✅ Yes | ✅ Yes |
| **Function Calling** | ✅ Yes | ✅ Yes | ✅ Yes |
| **Vision/Images** | ✅ GPT-4V | ❌ No | ✅ Claude 3+ |
| **Max Context** | 128K (GPT-4) | 64K | 200K (Claude 3) |
| **Best For** | General purpose, production | Cost-effective, reasoning | Safety-critical, analysis |
| **Pricing** | $$ | $ | $$$ |
| **Latency** | Low | Low | Low-Medium |

### Detailed Feature Matrix

#### OpenAI
- **Strengths**:
  - Most mature ecosystem with extensive tooling
  - Wide range of models (GPT-4, GPT-3.5-turbo, GPT-4 Vision)
  - Excellent for general-purpose applications
  - Strong vision/multimodal capabilities
  - Large community and documentation

- **Limitations**:
  - Higher cost compared to alternatives
  - Context window smaller than Claude
  - Rate limiting on free tier

- **Ideal Use Cases**:
  - Production deployments requiring reliability
  - Applications needing vision/image analysis
  - General-purpose AI assistants
  - Well-documented, standard use cases

#### DeepSeek
- **Strengths**:
  - Most cost-effective option
  - Strong reasoning and code generation
  - High throughput capabilities
  - Good for analytical tasks
  - Competitive performance at lower cost

- **Limitations**:
  - Smaller context window (64K)
  - No vision support
  - Newer ecosystem, less community resources

- **Ideal Use Cases**:
  - Cost-sensitive deployments
  - Code generation and analysis
  - Logical reasoning tasks
  - High-volume/batch processing
  - Internal tooling and development

#### Anthropic Claude
- **Strengths**:
  - Largest context window (200K tokens)
  - Strong safety and ethical guidelines
  - Excellent for complex analysis
  - Superior long-document processing
  - Strong instruction following

- **Limitations**:
  - Higher cost
  - Claude-specific API differences (system messages separate)
  - Requires max_tokens parameter

- **Ideal Use Cases**:
  - Safety-critical applications
  - Complex document analysis
  - Long-context reasoning
  - Compliance and governance
  - Medical/legal/financial applications

---

### Streamed Usage Support

Every adapter's streaming path is audited (ACCT-03) for whether the terminal chunk of a streamed
response carries the same token usage the non-streaming path reports. This table covers every
adapter Paladin ships, not only the three profiled above. Each cell is exactly one of three
values — the three permitted values below, and no fourth "in-between" value:

| Provider | Streamed usage |
|----------|-----------------|
| OpenAI | yes (usage frame) |
| DeepSeek | yes (usage frame) |
| xAI Grok | yes (usage frame) |
| Moonshot Kimi | yes (usage frame) |
| Alibaba Qwen (DashScope compatible-mode) | yes (usage frame) |
| Ollama (OpenAI-compat `/v1/chat/completions`) | yes (usage frame) |
| Anthropic | yes (event accumulation) |
| Google Gemini | yes (event accumulation) |
| Generic OpenAI-compatible (`OpenAiCompatibleAdapter`) | server-dependent — `None` when omitted |

**"yes (usage frame)"** — the OpenAI-compatible family (OpenAI itself, DeepSeek, Grok, Kimi, Qwen,
and Ollama's own OpenAI-compatibility layer) all send `stream_options: {"include_usage": true}`
and parse the trailing usage frame the provider returns before `[DONE]`. For Ollama specifically,
this depends on the pinned dev-stack Ollama version having `stream_options` support in its
OpenAI-compatibility layer — Ollama's own documentation lists it as supported, so this is a
version caveat, not a known gap.

**"yes (event accumulation)"** — Anthropic and Gemini have no `stream_options`-style opt-in and no
single trailing usage frame; each adapter accumulates usage across the event stream itself
(Anthropic's `message_start`/`message_delta` events; Gemini's cumulative per-frame
`usageMetadata`) and attaches the final figure to the terminal chunk.

**The generic preset's own cell** — `OpenAiCompatibleAdapter` is configured against an arbitrary,
operator-chosen base URL, so it cannot know ahead of time whether that third-party or self-hosted
server implements `stream_options` at all. When the server ignores it, no usage frame ever
arrives, and the terminal chunk correctly reports `usage: None` rather than a fabricated or
estimated figure — see the adapter's own rustdoc for the full rationale.

---

## Configuration Guide

### Environment Variables

All providers can be configured via environment variables:

```bash
# OpenAI
export OPENAI_API_KEY="sk-..."

# DeepSeek
export DEEPSEEK_API_KEY="..."
export DEEPSEEK_BASE_URL="https://api.deepseek.com/v1"  # Optional
export DEEPSEEK_MODEL="deepseek-chat"                    # Optional

# Anthropic
export ANTHROPIC_API_KEY="sk-ant-..."
export ANTHROPIC_BASE_URL="https://api.anthropic.com/v1" # Optional
export ANTHROPIC_MODEL="claude-3-5-sonnet-20241022"      # Optional
```

### Configuration Files

Add provider configurations to `config.yml`:

```yaml
llm:
  # Default provider if multiple are configured
  default_provider: "openai"

  openai:
    api_key: "${OPENAI_API_KEY}"
    base_url: "https://api.openai.com/v1"
    model: "gpt-4"
    timeout_seconds: 30

  deepseek:
    api_key: "${DEEPSEEK_API_KEY}"
    base_url: "https://api.deepseek.com/v1"
    model: "deepseek-chat"
    timeout_seconds: 60

  anthropic:
    api_key: "${ANTHROPIC_API_KEY}"
    base_url: "https://api.anthropic.com/v1"
    model: "claude-3-5-sonnet-20241022"
    timeout_seconds: 30
```

### Programmatic Configuration

#### OpenAI

```rust
use paladin::infrastructure::adapters::llm::openai_adapter::OpenAILlmAdapter;
use std::time::Duration;

let adapter = OpenAILlmAdapter::new(
    api_key,
    None, // Use default base URL
    Some(Duration::from_secs(30))
)?;
```

#### DeepSeek

```rust
use paladin::infrastructure::adapters::llm::deepseek_adapter::{
    DeepSeekAdapter, DeepSeekConfig
};

// From environment
let config = DeepSeekConfig::from_env()?;
let adapter = DeepSeekAdapter::new(config)?;

// Or custom
let config = DeepSeekConfig::new(
    api_key,
    "https://api.deepseek.com/v1".to_string(),
    "deepseek-chat".to_string()
);
let adapter = DeepSeekAdapter::new(config)?;
```

#### Anthropic

```rust
use paladin::infrastructure::adapters::llm::anthropic_adapter::{
    AnthropicAdapter, AnthropicConfig
};

// From environment
let config = AnthropicConfig::from_env()?;
let adapter = AnthropicAdapter::new(config)?;

// Or custom
let config = AnthropicConfig::new(
    api_key,
    "https://api.anthropic.com/v1".to_string(),
    "claude-3-5-sonnet-20241022".to_string()
);
let adapter = AnthropicAdapter::new(config)?;
```

---

## Use Case Recommendations

### When to Use OpenAI

**Best for:**
- General-purpose AI applications
- Production deployments requiring proven reliability
- Applications needing vision/image analysis
- Multimodal applications
- Projects with complex tooling requirements

**Example Use Cases:**
- Customer support chatbots
- Content generation systems
- Image analysis and description
- General AI assistants
- Document Q&A systems

### When to Use DeepSeek

**Best for:**
- Cost-sensitive deployments
- Code generation and analysis
- Logical reasoning tasks
- High-volume batch processing
- Internal development tools

**Example Use Cases:**
- Code review automation
- Test generation
- Documentation generation
- Internal knowledge bases
- Analytical pipelines

### When to Use Anthropic Claude

**Best for:**
- Safety-critical applications
- Long-document analysis
- Complex reasoning tasks
- Compliance-sensitive domains
- High-stakes decision support

**Example Use Cases:**
- Legal document analysis
- Medical record processing
- Financial compliance checking
- Research paper analysis
- Complex contract review

---

## Migration Guide

### From OpenAI to DeepSeek

DeepSeek uses an OpenAI-compatible API, making migration straightforward:

```rust
// Before (OpenAI)
let llm_port = Arc::new(OpenAILlmAdapter::new(
    openai_key,
    None,
    Some(Duration::from_secs(30))
)?);

// After (DeepSeek)
let config = DeepSeekConfig::from_env()?;
let llm_port = Arc::new(DeepSeekAdapter::new(config)?);

// Your Paladin code remains the same
let paladin = PaladinBuilder::new(llm_port)
    .system_prompt("Your prompt")
    .build()?;
```

**Considerations:**
- DeepSeek has no vision support
- Context window is 64K vs 128K for GPT-4
- Response style may differ slightly

### From OpenAI to Anthropic

Anthropic Claude requires some adjustments due to API differences:

```rust
// Before (OpenAI)
let llm_port = Arc::new(OpenAILlmAdapter::new(
    openai_key,
    None,
    Some(Duration::from_secs(30))
)?);

// After (Anthropic)
let config = AnthropicConfig::from_env()?;
let llm_port = Arc::new(AnthropicAdapter::new(config)?);

// Your Paladin code remains the same
let paladin = PaladinBuilder::new(llm_port)
    .system_prompt("Your prompt")
    .build()?;
```

**Key Differences:**
- Claude requires `max_tokens` parameter (defaults to 4096)
- System messages are sent separately
- Larger context window (200K tokens)
- Different SSE streaming format

### Provider Fallback Pattern

Implement graceful fallback for higher reliability:

```rust
use paladin::paladin_ports::output::llm_port::LlmPort;
use std::sync::Arc;

fn create_llm_provider() -> Result<Arc<dyn LlmPort>, Box<dyn std::error::Error>> {
    // Try DeepSeek first (cost-effective)
    if let Ok(config) = DeepSeekConfig::from_env() {
        if let Ok(adapter) = DeepSeekAdapter::new(config) {
            return Ok(Arc::new(adapter));
        }
    }

    // Fallback to Anthropic (powerful)
    if let Ok(config) = AnthropicConfig::from_env() {
        if let Ok(adapter) = AnthropicAdapter::new(config) {
            return Ok(Arc::new(adapter));
        }
    }

    // Final fallback to OpenAI (default)
    let api_key = std::env::var("OPENAI_API_KEY")?;
    Ok(Arc::new(OpenAILlmAdapter::new(
        api_key,
        None,
        Some(Duration::from_secs(30))
    )?))
}
```

---

## Performance Characteristics

### Latency Comparison (Approximate)

| Provider | First Token (p50) | First Token (p95) | Throughput |
|----------|-------------------|-------------------|------------|
| OpenAI GPT-4 | 500-800ms | 1-2s | Medium |
| OpenAI GPT-3.5 | 200-400ms | 500ms-1s | High |
| DeepSeek | 300-600ms | 800ms-1.5s | High |
| Anthropic Claude | 400-700ms | 1-2s | Medium |

*Note: Actual performance varies based on request size, load, and region*

### Cost Comparison (Approximate)

**Per 1M Tokens (Input/Output):**

| Provider | Model | Input | Output |
|----------|-------|-------|--------|
| OpenAI | GPT-4 | $10 | $30 |
| OpenAI | GPT-3.5-turbo | $0.50 | $1.50 |
| DeepSeek | deepseek-chat | $0.10 | $0.20 |
| Anthropic | Claude 3.5 Sonnet | $3 | $15 |

*Prices are approximate and subject to change*

### Scaling Considerations

**OpenAI:**
- Rate limits: Tier-based (requests/min, tokens/min)
- Horizontal scaling: Good
- Burst capacity: Moderate

**DeepSeek:**
- Rate limits: Generous
- Horizontal scaling: Excellent (high throughput)
- Burst capacity: High

**Anthropic:**
- Rate limits: Tier-based
- Horizontal scaling: Good
- Burst capacity: Moderate

---

## Best Practices

### 1. Use Provider Capabilities

Query provider capabilities before attempting operations:

```rust
let caps = provider.get_capabilities();

if caps.supports_vision {
    // Send image-based requests
}

if caps.supports_streaming {
    // Use streaming for better UX
}
```

### 2. Set Appropriate Timeouts

Different providers may have different response times:

```rust
// Higher timeout for Claude with long contexts
let claude_config = AnthropicConfig::new(/* ... */);
// Timeout handled internally

// Standard timeout for others
let openai = OpenAILlmAdapter::new(
    api_key,
    None,
    Some(Duration::from_secs(30))
)?;
```

### 3. Handle Provider-Specific Errors

```rust
match provider.generate(&request).await {
    Ok(response) => // Handle response,
    Err(LlmError::RateLimitExceeded { retry_after }) => {
        tokio::time::sleep(Duration::from_secs(retry_after)).await;
        // Retry
    }
    Err(LlmError::AuthenticationError(_)) => {
        // Check API keys
    }
    Err(e) => // Handle other errors
}
```

### 4. Monitor Usage and Costs

```rust
let response = provider.generate(&request).await?;

// Log token usage
println!("Input tokens: {}", response.usage.prompt_tokens);
println!("Output tokens: {}", response.usage.completion_tokens);
println!("Total cost: ${}", calculate_cost(&response, provider_name));
```

---

## Troubleshooting

### Authentication Errors

**Issue:** `LlmError::AuthenticationError`

**Solutions:**
1. Verify API key is set correctly
2. Check API key has necessary permissions
3. Ensure API key hasn't expired
4. Verify base URL is correct for your region

### Rate Limiting

**Issue:** `LlmError::RateLimitExceeded`

**Solutions:**
1. Implement exponential backoff (built-in to adapters)
2. Consider upgrading API tier
3. Implement request queuing
4. Switch to provider with higher limits

### Timeout Errors

**Issue:** `LlmError::Timeout`

**Solutions:**
1. Increase timeout duration
2. Reduce request complexity
3. Check network connectivity
4. Consider switching to streaming mode

### Context Length Errors

**Issue:** `LlmError::InvalidRequest` (context too long)

**Solutions:**
1. Reduce input size
2. Switch to provider with larger context (Claude: 200K)
3. Implement context windowing
4. Summarize older conversation history

---

## Additional Resources

- [Paladin Examples](https://github.com/DF3NDR/paladin-dev-env/tree/main/examples) - Working code examples
- [Contributing Providers Guide](../contributing/contributing-providers.md) - Add new providers
- [API Documentation](https://docs.rs/paladin) - Full API reference
- [GitHub Issues](https://github.com/DF3NDR/paladin/issues) - Report issues

---

**Last Updated:** January 2026  
**Version:** 0.1.0
