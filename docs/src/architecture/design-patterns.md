# Design Patterns

Reference for the key Rust design patterns used consistently across the Paladin
codebase.

## 1. `Node<T>` Entity Pattern

All persistent domain entities are wrapped in `Node<T>`, which adds identity and
timestamps without polluting the domain data struct:

```rust,ignore
pub struct Node<T> {
    pub id:         Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub node:       T,
}

// Usage — domain type alias
pub type Paladin = Node<PaladinData>;

// Access payload fields through .node
let name = &paladin.node.name;
let model = &paladin.node.model;
```

## 2. Builder Pattern (`PaladinBuilder`)

Complex objects are constructed using a fluent builder that validates configuration
before returning the entity.

```rust,ignore
// crates/paladin-ai-core (application services layer within the crate)
use paladin_ai_core::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin_ports::output::llm_port::LlmPort;
use std::sync::Arc;

let llm: Arc<dyn LlmPort> = Arc::new(my_adapter);

let paladin = PaladinBuilder::new(llm.clone())
    .system_prompt("You are a concise assistant.")
    .name("MyAgent")
    .model("gpt-4")
    .temperature(0.7)       // 0.0–2.0
    .max_loops(3)            // default: 1
    .stop_word("DONE")       // optional stop trigger
    .build()
    .await?;
```

`build()` validates all required fields and returns `Err(PaladinError)` if any
invariant is violated, so the caller never receives an invalid `Paladin`.

### Builder Conventions

- Required fields are set in `new()` (e.g., `llm_port`)
- Optional fields use `fn field(mut self, …) -> Self` (consuming builder)
- `build()` or `build_async()` calls the internal `validate()` method
- Always return `Result<T, Error>` from `build()` — never panic

## 3. Port Trait Pattern

All external integrations are expressed as `async_trait` port traits:

```rust,ignore
use async_trait::async_trait;

#[async_trait]
pub trait LlmPort: Send + Sync {
    async fn generate(
        &self,
        messages: &[Message],
        config: &LlmConfig,
    ) -> Result<LlmResponse, LlmError>;
}
```

Rules for port traits:
- Must be `Send + Sync` — adapters are shared across async tasks via `Arc`
- Return `Result<T, SpecificError>` — never `anyhow::Error` in port signatures
- Use `#[async_trait]` for all `async fn` in traits (until RPITIT stabilises)
- Define in `crates/paladin-ports/src/output/` or `src/input/`

## 4. Error Handling with `thiserror`

Each domain module has its own error enum:

```rust,ignore
#[derive(Debug, thiserror::Error)]
pub enum PaladinError {
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Execution error: {0}")]
    ExecutionError(String),

    #[error("LLM error: {0}")]
    LlmError(#[from] LlmError),

    #[error("Timeout after {0} seconds")]
    Timeout(u64),

    #[error("Stop word detected: {0}")]
    StopWordDetected(String),
}

#[derive(Debug, thiserror::Error)]
pub enum BattalionError {
    #[error("Paladin error: {0}")]
    PaladinError(#[from] PaladinError),

    #[error("Formation error: {0}")]
    FormationError(String),

    #[error("Invalid graph: {0}")]
    InvalidGraph(String),
}
```

**Conventions:**
- Layer-specific error types (core, ports, infrastructure)
- Use `#[from]` for automatic `From` impl when converting inner errors
- Never use `.unwrap()` or `.expect()` in production paths
- Use `?` to propagate errors with context

## 5. Dependency Injection via `Arc<dyn Trait>`

Services receive dependencies at construction time via `Arc<dyn Port>`:

```rust,ignore
// src/application/services/paladin/paladin_execution_service.rs
pub struct PaladinExecutionService {
    llm_port:        Arc<dyn LlmPort>,
    circuit_breaker: Arc<CircuitBreaker>,
    garrison:        Option<Arc<dyn GarrisonPort>>,
    arsenal:         Option<Arc<dyn ArsenalPort>>,
}

impl PaladinExecutionService {
    pub fn new(
        llm_port: Arc<dyn LlmPort>,
        circuit_breaker: Arc<CircuitBreaker>,
        garrison: Option<Arc<dyn GarrisonPort>>,
        arsenal: Option<Arc<dyn ArsenalPort>>,
    ) -> Self { /* … */ }
}
```

This makes swapping implementations (e.g., `MockLlmAdapter` in tests,
`OpenAIAdapter` in production) a construction-time concern only.

## 6. Feature-Gated Modules

Optional infrastructure dependencies are gated behind Cargo feature flags to
keep compilation lean:

```rust,ignore
// crates/paladin-memory/src/lib.rs

/// SQLite-backed garrison (requires feature `sqlite`)
#[cfg(feature = "sqlite")]
pub mod sqlite_garrison;

/// Qdrant-backed sanctum (requires feature `qdrant`)
#[cfg(feature = "qdrant")]
pub mod qdrant_sanctum_adapter;
```

And in `Cargo.toml`:
```toml
[features]
sqlite = ["sqlx/sqlite"]
qdrant = ["qdrant-client"]
```

## 7. Service Composition

Application services compose port dependencies to implement use cases:

```rust,ignore
pub struct FormationService {
    paladins: Vec<Paladin>,
    llm:      Arc<dyn LlmPort>,
    garrison: Option<Arc<dyn GarrisonPort>>,
}

impl FormationService {
    /// Execute the formation: each Paladin's output feeds the next
    pub async fn execute(&self, input: &str) -> Result<FormationResult, BattalionError> {
        let mut current_input = input.to_string();
        let mut results = Vec::new();

        for paladin in &self.paladins {
            let result = self.execute_paladin(paladin, &current_input).await?;
            current_input = result.output.clone();
            results.push(result);
        }

        Ok(FormationResult { results })
    }
}
```

## 8. Circuit Breaker

`PaladinExecutionService` accepts a `CircuitBreaker` to prevent cascading
failures when an LLM provider is unavailable:

```rust,ignore
use paladin_ai_core::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use std::time::Duration;

let circuit_breaker = Arc::new(CircuitBreaker::new(
    3,                        // open after 3 consecutive failures
    2,                        // close after 2 consecutive successes
    Duration::from_secs(30),  // half-open probe interval
));
```

## 9. Testing Conventions

### Unit tests — in the same file

```rust,ignore
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paladin_data_validates_empty_prompt() {
        let data = PaladinData { system_prompt: "".into(), /* … */ };
        assert!(data.validate().is_err());
    }
}
```

### Integration tests — in `tests/`

```rust,ignore
// tests/formation_integration_test.rs
use paladin_ai_core::…;
use paladin_llm::mock::MockLlmAdapter;

#[tokio::test]
async fn test_formation_passes_output_to_next() {
    let mock = Arc::new(MockLlmAdapter::new().with_response("step result".into()));
    // …
}
```

### Doc tests — in rustdoc comments

````rust,ignore
/// Validates the Paladin configuration.
///
/// ```rust
/// # use paladin_ai_core::platform::container::paladin::PaladinData;
/// let data = PaladinData { system_prompt: "Hello".into(), … };
/// assert!(data.validate().is_ok());
/// ```
pub fn validate(&self) -> Result<(), PaladinError> { /* … */ }
````

## See Also

- [Architecture Overview](overview.md)
- [Hexagonal Design](hexagonal-design.md)
- [Domain Model](domain-model.md)
- [Contributing — Development Setup](../contributing/development-setup.md)
