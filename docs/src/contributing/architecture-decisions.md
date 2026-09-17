# Adapter Development Guide

Guide for creating custom adapters for Paladin's ports (interfaces). For the project's
architecture decision records, see [Architecture Decisions](adr-index.md).

## Table of Contents

- [Overview](#overview)
- [Port Architecture](#port-architecture)
- [LLM Adapter Development](#llm-adapter-development)
- [Garrison Adapter Development](#garrison-adapter-development)
- [Arsenal Adapter Development](#arsenal-adapter-development)
- [Citadel Adapter Development](#citadel-adapter-development)
- [Testing Adapters](#testing-adapters)
- [Publishing Adapters](#publishing-adapters)

## Overview

Paladin uses **Hexagonal Architecture** (Ports and Adapters) to enable pluggable implementations for external systems.

### Core Concepts

```
┌─────────────────────────────────────────┐
│         Application Core                │
│  ┌──────────────────────────────────┐  │
│  │      Domain Logic (Core)          │  │
│  │  - Paladin, Battalion, etc.       │  │
│  └──────────────────────────────────┘  │
│               ▲                          │
│               │ Uses                     │
│  ┌──────────────────────────────────┐  │
│  │      Ports (Interfaces)           │  │
│  │  - LlmPort, GarrisonPort, etc.    │  │
│  └──────────────────────────────────┘  │
└─────────────────────────────────────────┘
                │ Implemented by
                ▼
┌─────────────────────────────────────────┐
│         Adapters (Infrastructure)        │
│  - OpenAI, DeepSeek, Anthropic           │
│  - SQLite, Redis, PostgreSQL             │
│  - MCP, Custom Tools                     │
└─────────────────────────────────────────┘
```

### Adapter Lifecycle

1. **Define Port Trait** (application layer)
2. **Implement Adapter** (infrastructure layer)
3. **Register Adapter** (dependency injection)
4. **Test Adapter** (unit + integration tests)
5. **Document Adapter** (usage examples)

## Port Architecture

### Existing Ports

| Port | Crate / Location | Purpose |
|------|-----------------|---------|
| `LlmPort` | `crates/paladin-ports/src/output/llm_port.rs` | LLM provider abstraction |
| `GarrisonPort` | `crates/paladin-ports/src/output/garrison_port.rs` | Short-term memory |
| `LongTermGarrisonPort` | `crates/paladin-ports/src/output/garrison_port.rs` | Vector-backed long-term memory |
| `ArsenalPort` | `crates/paladin-ports/src/output/arsenal_port.rs` | Tool / armament execution |
| `CitadelPort` | `crates/paladin-ports/src/output/citadel_port.rs` | State persistence |
| `FileStoragePort` | `crates/paladin-ports/src/output/file_storage_port.rs` | Object/file storage |
| `NotificationPort` | `crates/paladin-ports/src/output/notification_port.rs` | Notifications |

### Port Requirements

All ports must be:
- **`Send + Sync`**: Thread-safe for async
- **Async**: Use `#[async_trait]`
- **Error handling**: Return `Result<T, SpecificError>`
- **Well documented**: Rustdoc comments with examples

## LLM Adapter Development

### 1. Define Custom LLM Provider

```rust,ignore
// crates/paladin-llm/src/custom/mod.rs
// Enable via a feature flag in crates/paladin-llm/Cargo.toml

use async_trait::async_trait;
use crate::paladin_ports::output::llm_port::{LlmPort, Message, LlmResponse};
use crate::core::platform::container::paladin::PaladinError;

pub struct CustomLlmAdapter {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl CustomLlmAdapter {
    pub fn new(api_key: String, base_url: String) -> Self {
        Self {
            api_key,
            base_url,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LlmPort for CustomLlmAdapter {
    async fn generate(
        &self,
        messages: &[Message],
        config: &LlmConfig,
    ) -> Result<LlmResponse, PaladinError> {
        // 1. Transform messages to provider format
        let request_body = self.build_request(messages, config)?;

        // 2. Make API call
        let response = self.client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request_body)
            .send()
            .await
            .map_err(|e| PaladinError::LlmError(e.to_string()))?;

        // 3. Parse response
        let response_data: CustomApiResponse = response
            .json()
            .await
            .map_err(|e| PaladinError::LlmError(e.to_string()))?;

        // 4. Transform to LlmResponse
        Ok(LlmResponse {
            content: response_data.message.content,
            model: response_data.model,
            usage: response_data.usage.into(),
            tool_calls: self.parse_tool_calls(&response_data),
        })
    }

    async fn generate_stream(
        &self,
        messages: &[Message],
        config: &LlmConfig,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<LlmChunk>>>>, PaladinError> {
        // Implement streaming if supported
        todo!("Streaming implementation")
    }

    fn validate_model(&self, model: &str) -> Result<(), PaladinError> {
        const SUPPORTED_MODELS: &[&str] = &[
            "custom-model-v1",
            "custom-model-v2",
        ];

        if SUPPORTED_MODELS.contains(&model) {
            Ok(())
        } else {
            Err(PaladinError::ConfigurationError(
                format!("Unsupported model: {}", model)
            ))
        }
    }
}

impl CustomLlmAdapter {
    fn build_request(
        &self,
        messages: &[Message],
        config: &LlmConfig,
    ) -> Result<serde_json::Value, PaladinError> {
        // Provider-specific request format
        Ok(serde_json::json!({
            "model": config.model,
            "messages": messages,
            "temperature": config.temperature,
            "max_tokens": config.max_tokens,
        }))
    }

    fn parse_tool_calls(&self, response: &CustomApiResponse) -> Vec<ToolCall> {
        // Extract tool calls if provider supports them
        vec![]
    }
}
```

### 2. Handle Tool Calling

```rust,ignore
#[derive(Debug, Deserialize)]
struct CustomToolCall {
    id: String,
    function: FunctionCall,
}

#[derive(Debug, Deserialize)]
struct FunctionCall {
    name: String,
    arguments: String,
}

impl CustomLlmAdapter {
    fn parse_tool_calls(&self, response: &CustomApiResponse) -> Vec<ToolCall> {
        response.tool_calls
            .iter()
            .map(|tc| ToolCall {
                id: tc.id.clone(),
                name: tc.function.name.clone(),
                arguments: serde_json::from_str(&tc.function.arguments)
                    .unwrap_or_default(),
            })
            .collect()
    }
}
```

### 3. Configuration

```yaml
# config.yml
llm:
  provider: "custom"
  custom:
    api_key: "${CUSTOM_API_KEY}"
    base_url: "https://api.custom-provider.com/v1"
    default_model: "custom-model-v1"
    timeout: 30s
```

### 4. Registration

```rust,ignore
// crates/paladin-llm/src/mod.rs  (feature-gated provider registration)

pub fn create_llm_adapter(config: &LlmConfig) -> Result<Arc<dyn LlmPort>> {
    match config.provider.as_str() {
        "openai" => Ok(Arc::new(OpenAiAdapter::new(config)?)),
        "deepseek" => Ok(Arc::new(DeepSeekAdapter::new(config)?)),
        "anthropic" => Ok(Arc::new(AnthropicAdapter::new(config)?)),
        "custom" => Ok(Arc::new(CustomLlmAdapter::new(
            config.custom.api_key.clone(),
            config.custom.base_url.clone(),
        ))),
        _ => Err(Error::UnsupportedProvider(config.provider.clone())),
    }
}
```

## Garrison Adapter Development

### 1. Implement Custom Storage Backend

```rust,ignore
// crates/paladin-memory/src/garrison/redis_garrison.rs

use async_trait::async_trait;
use redis::AsyncCommands;
use crate::paladin_ports::output::garrison_port::GarrisonPort;

pub struct RedisGarrison {
    client: redis::Client,
    prefix: String,
}

impl RedisGarrison {
    pub fn new(redis_url: &str, prefix: &str) -> Result<Self> {
        Ok(Self {
            client: redis::Client::open(redis_url)?,
            prefix: prefix.to_string(),
        })
    }

    fn make_key(&self, session_id: &Uuid) -> String {
        format!("{}:garrison:{}", self.prefix, session_id)
    }
}

#[async_trait]
impl GarrisonPort for RedisGarrison {
    async fn add_entry(
        &self,
        session_id: Uuid,
        entry: GarrisonEntry,
    ) -> Result<(), GarrisonError> {
        let mut conn = self.client.get_async_connection().await?;
        let key = self.make_key(&session_id);

        // Serialize entry
        let value = serde_json::to_string(&entry)?;

        // Add to list
        conn.rpush(key, value).await?;

        // Set expiration
        conn.expire(key, 3600).await?;

        Ok(())
    }

    async fn get_entries(
        &self,
        session_id: Uuid,
        limit: Option<usize>,
    ) -> Result<Vec<GarrisonEntry>, GarrisonError> {
        let mut conn = self.client.get_async_connection().await?;
        let key = self.make_key(&session_id);

        // Get entries
        let values: Vec<String> = if let Some(limit) = limit {
            conn.lrange(key, -(limit as isize), -1).await?
        } else {
            conn.lrange(key, 0, -1).await?
        };

        // Deserialize
        values.iter()
            .map(|v| serde_json::from_str(v).map_err(Into::into))
            .collect()
    }

    async fn search(
        &self,
        session_id: Uuid,
        query: &str,
    ) -> Result<Vec<GarrisonEntry>, GarrisonError> {
        // Implement semantic search using Redis Search module
        // or fallback to simple filtering
        let entries = self.get_entries(session_id, None).await?;
        Ok(entries.into_iter()
            .filter(|e| e.content.contains(query))
            .collect())
    }

    async fn clear(&self, session_id: Uuid) -> Result<(), GarrisonError> {
        let mut conn = self.client.get_async_connection().await?;
        let key = self.make_key(&session_id);
        conn.del(key).await?;
        Ok(())
    }
}
```

### 2. Add Vector Search Support

```rust,ignore
use crate::infrastructure::embeddings::EmbeddingProvider;

pub struct VectorGarrison {
    storage: Arc<dyn GarrisonPort>,
    embeddings: Arc<dyn EmbeddingProvider>,
}

#[async_trait]
impl GarrisonPort for VectorGarrison {
    async fn search(
        &self,
        session_id: Uuid,
        query: &str,
    ) -> Result<Vec<GarrisonEntry>, GarrisonError> {
        // 1. Generate query embedding
        let query_embedding = self.embeddings.embed(query).await?;

        // 2. Get all entries
        let entries = self.storage.get_entries(session_id, None).await?;

        // 3. Compute similarity scores
        let mut scored: Vec<_> = entries.into_iter()
            .map(|entry| {
                let score = cosine_similarity(&query_embedding, &entry.embedding);
                (entry, score)
            })
            .collect();

        // 4. Sort by relevance
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // 5. Return top results
        Ok(scored.into_iter()
            .take(10)
            .map(|(entry, _)| entry)
            .collect())
    }
}
```

## Arsenal Adapter Development

### 1. Create Custom Tool

```rust,ignore
// src/infrastructure/adapters/arsenal/weather_tool.rs
use async_trait::async_trait;
use crate::paladin_ports::output::arsenal_port::{ArsenalPort, ToolDefinition};

pub struct WeatherTool {
    api_key: String,
    client: reqwest::Client,
}

impl WeatherTool {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl ArsenalPort for WeatherTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "get_weather".into(),
            description: "Get current weather for a location".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "City name or coordinates"
                    }
                },
                "required": ["location"]
            }),
        }
    }

    async fn execute(
        &self,
        arguments: serde_json::Value,
    ) -> Result<ToolResult, ArsenalError> {
        // 1. Parse arguments
        let location = arguments["location"]
            .as_str()
            .ok_or(ArsenalError::InvalidArguments)?;

        // 2. Call weather API
        let response = self.client
            .get("https://api.weather.com/v1/current")
            .query(&[
                ("location", location),
                ("apikey", &self.api_key),
            ])
            .send()
            .await?;

        // 3. Parse response
        let weather: WeatherData = response.json().await?;

        // 4. Return result
        Ok(ToolResult {
            content: serde_json::to_string(&weather)?,
            metadata: Some(serde_json::json!({
                "provider": "weather.com",
                "location": location,
            })),
        })
    }
}
```

### 2. Implement MCP Tool Wrapper

```rust,ignore
// src/infrastructure/adapters/arsenal/mcp_wrapper.rs

pub struct McpToolWrapper {
    server_url: String,
    tool_name: String,
    client: reqwest::Client,
}

#[async_trait]
impl ArsenalPort for McpToolWrapper {
    fn definition(&self) -> ToolDefinition {
        // Fetch tool definition from MCP server
        // Cache for performance
        todo!()
    }

    async fn execute(
        &self,
        arguments: serde_json::Value,
    ) -> Result<ToolResult, ArsenalError> {
        // Forward to MCP server
        let response = self.client
            .post(format!("{}/tools/{}/execute", self.server_url, self.tool_name))
            .json(&arguments)
            .send()
            .await?;

        let result: McpToolResult = response.json().await?;
        Ok(result.into())
    }
}
```

## Citadel Adapter Development

### 1. Implement Custom Persistence

```rust,ignore
// src/infrastructure/adapters/citadel/s3_citadel.rs

use async_trait::async_trait;
use crate::paladin_ports::output::citadel_port::CitadelPort;

pub struct S3Citadel {
    bucket: String,
    client: aws_sdk_s3::Client,
}

impl S3Citadel {
    pub async fn new(bucket: String) -> Result<Self> {
        let config = aws_config::load_from_env().await;
        let client = aws_sdk_s3::Client::new(&config);
        Ok(Self { bucket, client })
    }
}

#[async_trait]
impl CitadelPort for S3Citadel {
    async fn save_state(
        &self,
        session_id: Uuid,
        state: PaladinState,
    ) -> Result<(), CitadelError> {
        let key = format!("paladin-state/{}.json", session_id);
        let body = serde_json::to_vec(&state)?;

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body.into())
            .send()
            .await?;

        Ok(())
    }

    async fn load_state(
        &self,
        session_id: Uuid,
    ) -> Result<Option<PaladinState>, CitadelError> {
        let key = format!("paladin-state/{}.json", session_id);

        match self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(output) => {
                let bytes = output.body.collect().await?.into_bytes();
                let state = serde_json::from_slice(&bytes)?;
                Ok(Some(state))
            }
            Err(_) => Ok(None),
        }
    }
}
```

## Testing Adapters

### Unit Tests

```rust,ignore
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_custom_llm_adapter() {
        let adapter = CustomLlmAdapter::new(
            "test-key".into(),
            "http://localhost:8080".into(),
        );

        let messages = vec![Message::user("Hello")];
        let config = LlmConfig::default();

        let response = adapter.generate(&messages, &config).await;
        assert!(response.is_ok());
    }

    #[test]
    fn test_model_validation() {
        let adapter = CustomLlmAdapter::new(
            "test-key".into(),
            "http://localhost".into(),
        );

        assert!(adapter.validate_model("custom-model-v1").is_ok());
        assert!(adapter.validate_model("invalid-model").is_err());
    }
}
```

### Integration Tests

```rust,ignore
#[tokio::test]
async fn test_garrison_roundtrip() {
    let garrison = RedisGarrison::new("redis://localhost:6379", "test").unwrap();
    let session_id = Uuid::new_v4();

    // Add entry
    let entry = GarrisonEntry {
        role: "user".into(),
        content: "Test message".into(),
        timestamp: Utc::now(),
    };
    garrison.add_entry(session_id, entry.clone()).await.unwrap();

    // Retrieve
    let entries = garrison.get_entries(session_id, None).await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].content, "Test message");

    // Clear
    garrison.clear(session_id).await.unwrap();
    let entries = garrison.get_entries(session_id, None).await.unwrap();
    assert_eq!(entries.len(), 0);
}
```

## Publishing Adapters

### 1. Create Separate Crate

```toml
# Cargo.toml for adapter crate
[package]
name = "paladin-custom-llm"
version = "0.1.0"
edition = "2021"

[dependencies]
paladin-ai = { version = "0.5", default-features = false }
async-trait = "0.1"
reqwest = { version = "0.11", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### 2. Documentation

```rust,ignore
//! # Custom LLM Adapter for Paladin
//!
//! This adapter provides integration with CustomProvider's LLM API.
//!
//! ## Installation
//!
//! ```toml
//! [dependencies]
//! paladin-custom-llm = "0.1"
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use paladin_custom_llm::CustomLlmAdapter;
//!
//! let adapter = CustomLlmAdapter::new(api_key, base_url);
//! let paladin = PaladinBuilder::new(Arc::new(adapter))
//!     .build()?;
//! ```
```

### 3. Examples

Provide complete working examples in `examples/` directory.

## Next Steps

- **[Testing Guide](testing-guide.md)** - Test your adapters
- **[Contributing Guide](development-setup.md)** - Contribution guidelines
- **[Contributing Providers](contributing-providers.md)** - Provider-specific guides
