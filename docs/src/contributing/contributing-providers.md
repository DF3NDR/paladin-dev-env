# Contributing New LLM Providers

**Guide for Adding New LLM Providers to Paladin**

This guide walks you through implementing a new LLM provider adapter for Paladin. All providers implement the `LlmPort` trait, ensuring consistent behavior across the framework.

---

## Table of Contents

- [Prerequisites](#prerequisites)
- [Implementation Steps](#implementation-steps)
- [Adapter Template](#adapter-template)
- [Testing Requirements](#testing-requirements)
- [Documentation Requirements](#documentation-requirements)
- [Submission Guidelines](#submission-guidelines)

---

## Prerequisites

Before implementing a new provider:

1. **API Documentation**: Have access to the provider's API documentation
2. **API Key**: Obtain an API key for testing
3. **Rust Knowledge**: Familiarity with async Rust and the `tokio` runtime
4. **Project Setup**: Clone and build the Paladin project

---

## Implementation Steps

### Step 1: Create Adapter File

LLM provider adapters live in the `paladin-llm` crate, gated by a feature flag:

```bash
# Create provider directory and adapter
mkdir -p crates/paladin-llm/src/myprovider
touch crates/paladin-llm/src/myprovider/mod.rs
```

Add a feature flag to `crates/paladin-llm/Cargo.toml`:
```toml
[features]
myprovider = []
```

Then gate the module in `crates/paladin-llm/src/lib.rs`:
```rust,ignore
#[cfg(feature = "myprovider")]
pub mod myprovider;
```

The root `paladin-ai` crate then exposes a top-level feature:
```toml
# Cargo.toml (root)
[features]
llm-myprovider = ["paladin-llm/myprovider"]
llm-all = ["llm-openai", "llm-anthropic", "llm-deepseek", "llm-myprovider"]
```

### Step 2: Define Configuration Struct

```rust,ignore
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyProviderConfig {
    /// API key for authentication
    pub api_key: String,
    /// Base URL for API
    pub base_url: String,
    /// Default model to use
    pub model: String,
    /// Request timeout in seconds
    pub timeout_seconds: u64,
}

impl MyProviderConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, String> {
        let api_key = std::env::var("MYPROVIDER_API_KEY")
            .map_err(|_| "MYPROVIDER_API_KEY not set")?;

        let base_url = std::env::var("MYPROVIDER_BASE_URL")
            .unwrap_or_else(|_| "https://api.myprovider.com/v1".to_string());

        let model = std::env::var("MYPROVIDER_MODEL")
            .unwrap_or_else(|_| "default-model".to_string());

        let timeout_seconds = 60;

        Ok(Self {
            api_key,
            base_url,
            model,
            timeout_seconds,
        })
    }

    /// Create custom configuration
    pub fn new(api_key: String, base_url: String, model: String) -> Self {
        Self {
            api_key,
            base_url,
            model,
            timeout_seconds: 60,
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.api_key.is_empty() {
            return Err("API key cannot be empty".to_string());
        }
        if !self.base_url.starts_with("http") {
            return Err("Base URL must start with http/https".to_string());
        }
        Ok(())
    }
}
```

### Step 3: Implement Adapter Struct

```rust,ignore
use crate::paladin_ports::output::llm_port::{
    LlmError, LlmPort, LlmRequest, LlmResponse, ProviderCapabilities
};
use async_trait::async_trait;
use reqwest::{Client, header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE}};
use std::time::Duration;

pub struct MyProviderAdapter {
    client: Client,
    config: MyProviderConfig,
}

impl MyProviderAdapter {
    pub fn new(config: MyProviderConfig) -> Result<Self, LlmError> {
        config.validate()
            .map_err(|e| LlmError::AuthenticationError(e))?;

        let timeout = Duration::from_secs(config.timeout_seconds);

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", config.api_key))
                .map_err(|e| LlmError::AuthenticationError(e.to_string()))?
        );

        let client = Client::builder()
            .timeout(timeout)
            .default_headers(headers)
            .build()
            .map_err(|e| LlmError::ProviderError(e.to_string()))?;

        Ok(Self { client, config })
    }
}
```

### Step 4: Implement LlmPort Trait

Declaring `supports_tool_calling` or `supports_function_calling` as `true` requires your
adapter to actually produce a populated `LlmResponse.function_call` — the shared
correspondence test in `crates/paladin-llm/src/lib.rs`
(`test_capabilities_tool_calling_matches_request_surface`) pins every adapter's declared
capability to what its request/response surface actually carries, and fails the build
otherwise.

```rust,ignore
#[async_trait]
impl LlmPort for MyProviderAdapter {
    async fn generate(&self, request: &LlmRequest) -> Result<LlmResponse, LlmError> {
        // 1. Build provider-specific request
        let provider_request = self.build_request(request)?;

        // 2. Make HTTP request with retry logic
        let response = self.make_request(provider_request).await?;

        // 3. Parse and convert to LlmResponse
        self.parse_response(response, request).await
    }

    async fn generate_stream(
        &self,
        request: &LlmRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, LlmError>> + Send>>, LlmError> {
        // Implement SSE streaming if supported
        unimplemented!("Streaming not yet implemented")
    }

    fn get_capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_streaming: true,  // Set based on provider
            // These flags describe what *this adapter's* `generate()` does, not what the
            // vendor's API offers. Set `true` only once your `parse_response` actually
            // populates `LlmResponse.function_call` from the provider's reply — every
            // shipped adapter declares `false` today because none of them does (ADR-0042).
            supports_tool_calling: false,
            supports_function_calling: false,
            supports_vision: false,  // Set based on provider
            supports_embeddings: false,
            max_context_tokens: Some(128_000),  // Provider's limit
            supports_system_messages: true,
        }
    }

    fn get_provider_name(&self) -> String {
        "myprovider".to_string()
    }

    async fn validate_model(&self, model: &str) -> Result<bool, LlmError> {
        let available = self.get_available_models().await?;
        Ok(available.contains(&model.to_string()))
    }

    async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
        Ok(vec![
            "model-1".to_string(),
            "model-2".to_string(),
            // Add provider's models
        ])
    }
}
```

### Step 5: Add to Module

Update `crates/paladin-llm/src/lib.rs`:

```rust,ignore
pub mod myprovider_adapter;
```

### Step 6: Update Provider Factory

Add to `crates/paladin-llm/src/provider_factory.rs`:

```rust,ignore
"myprovider" => {
    let config = MyProviderConfig::from_env()
        .map_err(|e| LlmError::ConfigurationError(e))?;
    Ok(Arc::new(MyProviderAdapter::new(config)?))
}
```

---

## Adapter Template

See [adapter_template.rs](https://github.com/DF3NDR/paladin-dev-env/blob/main/examples/adapter_template.rs) for a complete template with:
- Full error handling
- Retry logic with exponential backoff
- Request/response serialization
- SSE streaming implementation
- Comprehensive documentation

---

## Testing Requirements

### Unit Tests (Required)

Create `tests/unit/llm/myprovider_adapter_test.rs`:

```rust,ignore
use mockito::Server;
use paladin_llm::myprovider::*;

#[tokio::test]
async fn test_successful_completion() {
    let mut server = Server::new_async().await;

    let mock = server.mock("POST", "/v1/completions")
        .with_status(200)
        .with_body(r#"{"response": "test"}"#)
        .create_async()
        .await;

    let config = MyProviderConfig::new(
        "test-key".to_string(),
        server.url(),
        "test-model".to_string()
    );

    let adapter = MyProviderAdapter::new(config).unwrap();
    // Test adapter functionality

    mock.assert_async().await;
}

#[tokio::test]
async fn test_authentication_error() {
    // Test 401 handling
}

#[tokio::test]
async fn test_rate_limiting() {
    // Test 429 handling
}

// Add tests for all error cases and success paths
```

**Required test coverage:**
- ✅ Successful completion
- ✅ Streaming responses
- ✅ Authentication errors (401)
- ✅ Rate limiting (429)
- ✅ Timeouts
- ✅ Invalid model errors
- ✅ Malformed responses

### Streaming Usage Terminal-Chunk Contract (v0.10.0, ACCT-03)

Every adapter's streaming path must satisfy one contract: **`usage` is `Some` on exactly the
chunk that carries a finish reason, and `None` on every other chunk.** A new adapter's test suite
proves this the same way the shared conformance suite does for every adapter that already ships
(`crates/paladin-llm/src/conformance.rs`'s `streaming_usage_equals_non_streaming_usage` case,
instantiated via `crate::llm_conformance_suite!` where your adapter's wire shape fits the shared
`ConformanceFixture`, or a dedicated test asserting the identical three properties where it does
not):

1. Exactly one chunk in the stream has `usage.is_some()`.
2. That chunk is the SAME chunk whose `finish_reason.is_some()`.
3. That chunk's `usage` equals the non-streaming `LlmResponse.usage`, field-for-field —
   including the three optional sub-counts (`cache_read_tokens`, `cache_write_tokens`,
   `reasoning_tokens`).

If your provider's streaming endpoint cannot report usage at all — or only does so when the
caller opts in and the caller cannot know ahead of time whether the specific configured server
honors that opt-in — document the gap as an explicit exception in TWO places, not one:

- Your adapter's own rustdoc (see `crates/paladin-llm/src/openai_compatible/adapter.rs`'s
  `OpenAiCompatibleAdapter` doc comment for the house pattern).
- The "Streamed usage" table in `docs/src/appendix/provider-expansion.md`, using exactly one of
  the three permitted values that table documents.

Never invent a fourth "partial" value, and never substitute a `TokenCounterPort` estimate for a
missing billed figure — an unreported streamed usage is `None`, not an estimate presented as a
provider-billed count.

### Integration Tests (Optional)

Create `tests/integration/llm/myprovider_integration_test.rs` with tests marked `#[ignore]` for live API testing.

---

## Documentation Requirements

### 1. Rustdoc Comments

Add comprehensive rustdoc to all public items:

```rust,ignore
/// MyProvider LLM adapter
///
/// Implements the LlmPort trait for MyProvider's API.
///
/// # Examples
///
/// ```no_run
/// use paladin_llm::myprovider::*;
///
/// let config = MyProviderConfig::from_env()?;
/// let adapter = MyProviderAdapter::new(config)?;
/// ```
pub struct MyProviderAdapter {
    // ...
}
```

### 2. Configuration Guide

Add section to `docs/PROVIDER_EXPANSION.md`:

- Configuration examples
- Use case recommendations
- Pricing information
- Performance characteristics

### 3. Example Code

Create `examples/myprovider_example.rs` demonstrating usage.

---

## Submission Guidelines

### Checklist

Before submitting a pull request:

- [ ] Adapter implements all `LlmPort` trait methods
- [ ] Configuration struct with `from_env()` and validation
- [ ] Unit tests with ≥80% coverage
- [ ] All tests passing (`cargo test`)
- [ ] Code formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Rustdoc for all public items
- [ ] Added to provider factory
- [ ] Documentation updated
- [ ] Example code created

### Pull Request Template

```markdown
## New Provider: [Provider Name]

### Description
Brief description of the provider and its strengths.

### Changes
- [ ] Adapter implementation
- [ ] Unit tests (XX% coverage)
- [ ] Integration tests
- [ ] Documentation
- [ ] Examples

### Testing
- All unit tests passing
- Integration tests verified with API key
- Tested on: [OS/Platform]

### Documentation
- [ ] PROVIDER_EXPANSION.md updated
- [ ] Rustdoc complete
- [ ] Example added

### Checklist
- [ ] Follows project code style
- [ ] No breaking changes
- [ ] Backward compatible
```

---

## Common Pitfalls

### 1. Incomplete Error Handling

❌ **Bad:**
```rust,ignore
let response = self.client.post(&url).send().await.unwrap();
```

✅ **Good:**
```rust,ignore
let response = self.client.post(&url)
    .send()
    .await
    .map_err(|e| LlmError::NetworkError(e.to_string()))?;
```

### 2. Missing Retry Logic

Implement exponential backoff for rate limits:

```rust,ignore
async fn make_request_with_retry(&self, request: Request) -> Result<Response, LlmError> {
    let mut attempt = 0;
    loop {
        match self.client.execute(request.try_clone()?).await {
            Ok(resp) if resp.status().is_success() => return Ok(resp),
            Ok(resp) if resp.status() == 429 => {
                attempt += 1;
                if attempt >= 3 {
                    return Err(LlmError::RateLimitExceeded { retry_after: 60 });
                }
                tokio::time::sleep(Duration::from_millis(1000 * 2u64.pow(attempt))).await;
            }
            Err(e) => return Err(LlmError::NetworkError(e.to_string())),
        }
    }
}
```

### 3. Hardcoded Values

Use configuration for all provider-specific values.

---

## Getting Help

- **GitHub Discussions**: Ask questions
- **Discord**: Real-time community help
- **GitHub Issues**: Report bugs or request features

---

**Happy Contributing!** 🗡️

Thank you for helping expand Paladin's LLM provider ecosystem.
