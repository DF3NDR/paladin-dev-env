# Hexagonal Architecture (Ports & Adapters)

Paladin uses Hexagonal Architecture to keep the core domain testable, swappable,
and free of infrastructure lock-in.

## Core Concepts

| Term | Paladin meaning |
|------|----------------|
| **Port** | A Rust `trait` in `paladin-ports` that defines an interface |
| **Adapter** | A `struct` in an infrastructure crate that `impl`s a port trait |
| **Core** | `paladin-ai-core` — zero external deps, pure domain logic |
| **Application boundary** | `paladin-ports` + `paladin-battalion` |

The rule: **dependencies point inward only.**

```
External world
   ↓ (adapters in paladin-llm, paladin-memory, …)
paladin-ports   (trait contracts)
   ↓
paladin-ai-core (pure domain)
```

## Port Traits

All port traits live in `crates/paladin-ports/src/output/` (infrastructure-facing
ports) or `src/input/` (ingestion-facing ports).

### LLM Port

```rust,ignore
// crates/paladin-ports/src/output/llm_port.rs
#[async_trait]
pub trait LlmPort: Send + Sync {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError>;

    async fn generate_stream(
        &self,
        request: LlmRequest,
    ) -> Result<Box<dyn futures::Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError>;
}
```

`LlmRequest` is a single builder-constructed request value (`id`, `model`, `prompt`, `attachments`,
`stream`, `metadata`, `response_format`) — not the two-parameter `(messages, config)` form this
page previously showed.

### Garrison Port

```rust,ignore
// crates/paladin-ports/src/output/garrison_port.rs
#[async_trait]
pub trait GarrisonPort: Send + Sync {
    async fn add_entry(&self, entry: GarrisonEntry) -> Result<(), GarrisonError>;
    async fn get_window(&self, max_tokens: usize) -> Result<Vec<GarrisonEntry>, GarrisonError>;
    async fn clear(&self) -> Result<(), GarrisonError>;
}
```

### Sanctum Port

```rust,ignore
// crates/paladin-ports/src/output/sanctum_port.rs
#[async_trait]
pub trait SanctumPort: Send + Sync {
    async fn store(&self, memory: Memory) -> Result<MemoryId, SanctumError>;
    async fn search(&self, query: &str, top_k: usize) -> Result<Vec<Memory>, SanctumError>;
}
```

### Arsenal Port

```rust,ignore
// crates/paladin-ports/src/output/arsenal_port.rs
#[async_trait]
pub trait ArsenalPort: Send + Sync {
    async fn list_tools(&self) -> Result<Vec<ToolDefinition>, ArsenalError>;
    async fn invoke(&self, call: &ToolCall) -> Result<ToolResult, ArsenalError>;
}
```

### File Storage Port

```rust,ignore
// crates/paladin-ports/src/output/file_storage_port.rs
#[async_trait]
pub trait FileStoragePort: Send + Sync {
    async fn upload(&self, key: &str, data: Vec<u8>) -> Result<(), StorageError>;
    async fn download(&self, key: &str) -> Result<Vec<u8>, StorageError>;
}
```

## Adapter Implementations

Each port trait is implemented by one or more adapters in an infrastructure crate.

### LLM Adapters (`crates/paladin-llm`)

| Adapter | Feature flag | Provider |
|---------|-------------|---------|
| `OpenAIAdapter` | `openai` (default) | OpenAI GPT |
| `AnthropicAdapter` | `anthropic` | Anthropic Claude |
| `DeepSeekAdapter` | `deepseek` | DeepSeek Chat |
| `MockLlmAdapter` | `mock` (default) | Testing |

```rust,ignore
// crates/paladin-llm/src/openai/mod.rs
pub struct OpenAIAdapter { /* ... */ }

#[async_trait]
impl LlmPort for OpenAIAdapter {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        // calls https://api.openai.com/v1/chat/completions
    }
}
```

### Memory Adapters (`crates/paladin-memory`)

| Adapter | Feature flag | Backend |
|---------|-------------|---------|
| `InMemoryGarrison` | *(always)* | In-process HashMap |
| `SqliteGarrison` | `sqlite` | SQLite via sqlx |
| `InMemorySanctum` | *(always)* | In-process vector |
| `QdrantSanctumAdapter` | `qdrant` | Qdrant gRPC |

### Storage Adapters (`crates/paladin-storage`)

| Adapter | Feature flag | Backend |
|---------|-------------|---------|
| `SqliteContentRepository` | `sqlite` | SQLite |
| `MySqlContentRepository` | `mysql` | MySQL |
| `SqliteUserRepository` | `sqlite` | SQLite |

## Adding a New Adapter

Follow these steps to add, say, a PostgreSQL Garrison adapter:

1. **Create the adapter file** in the appropriate infrastructure crate:

```
crates/paladin-memory/src/garrison/postgres_garrison.rs
```

2. **Implement the port trait:**

```rust,ignore
// crates/paladin-memory/src/garrison/postgres_garrison.rs
use paladin_ports::output::garrison_port::GarrisonPort;

pub struct PostgresGarrison {
    pool: sqlx::PgPool,
}

#[async_trait]
impl GarrisonPort for PostgresGarrison {
    async fn add_entry(&self, entry: GarrisonEntry) -> Result<(), GarrisonError> {
        // INSERT INTO garrison ...
    }

    async fn get_window(&self, max_tokens: usize) -> Result<Vec<GarrisonEntry>, GarrisonError> {
        // SELECT ... ORDER BY created_at DESC LIMIT ...
    }

    async fn clear(&self) -> Result<(), GarrisonError> {
        // DELETE FROM garrison
    }
}
```

3. **Gate behind a feature flag** in `crates/paladin-memory/Cargo.toml`:

```toml
[features]
postgres = ["sqlx/postgres"]

[dependencies]
sqlx = { version = "0.7", optional = true }
```

4. **Export from `lib.rs`** under the feature gate:

```rust,ignore
#[cfg(feature = "postgres")]
pub mod postgres_garrison;
```

5. **Write tests** using the existing garrison integration test pattern.

6. **Document** the adapter in [Garrison Memory](../user-guides/garrison-memory.md).

## Dependency Injection with `Arc<dyn Port>`

Services receive port implementations via `Arc<dyn Trait>`. This is how the
application layer stays decoupled from concrete adapters:

```rust,ignore
use std::sync::Arc;
use paladin_ports::output::llm_port::LlmPort;
use paladin_ports::output::garrison_port::GarrisonPort;

pub struct PaladinExecutionService {
    llm:      Arc<dyn LlmPort>,
    garrison: Option<Arc<dyn GarrisonPort>>,
}

impl PaladinExecutionService {
    pub fn new(llm: Arc<dyn LlmPort>) -> Self {
        Self { llm, garrison: None }
    }

    pub fn with_garrison(mut self, g: Arc<dyn GarrisonPort>) -> Self {
        self.garrison = Some(g);
        self
    }
}
```

Swap implementations at construction time — no code changes needed in the service.

## Testing with Mock Adapters

Use `MockLlmAdapter` (from `paladin-llm` with the `mock` feature) in unit tests:

```rust,ignore
use paladin_llm::mock::MockLlmAdapter;
use std::sync::Arc;
use paladin_ports::output::llm_port::LlmPort;

let mock: Arc<dyn LlmPort> = Arc::new(
    MockLlmAdapter::new().with_response("Test response".to_string())
);
let service = PaladinExecutionService::new(mock);
```

## See Also

- [Architecture Overview](overview.md)
- [Crate Map](crate-map.md) — workspace dependency graph
- [Design Patterns](design-patterns.md) — `PaladinBuilder`, error enums, service composition
- [Contributing Providers](../contributing/contributing-providers.md) — adding LLM providers
