# Coding Conventions

**Analysis Date:** 2026-07-30

## Naming Patterns

**Files:**
- Snake_case for all Rust source files: `paladin_builder.rs`, `garrison_port.rs`, `arsenal_execution_service.rs`
- Test files: `*_test.rs` (unit tests) or `*_integration_test.rs` (integration tests)
- Module directories use snake_case: `src/platform/container/`, `tests/integration/`

**Functions:**
- Snake_case for all functions and methods: `new()`, `enable_versioning()`, `generate()`, `validate_model()`
- Builder methods return `Self`: `.system_prompt()`, `.temperature()`, `.max_loops()`
- Getter methods use `get_` prefix: `get_call_count()`, `get_available_models()`, `get_capabilities()`
- Predicate methods use `is_` prefix: `is_terminal()`, `is_active()`, `is_versioning_enabled()`

**Types and Structs:**
- PascalCase for all type names: `Paladin`, `Battalion`, `Formation`, `MockLlmAdapter`, `PaladinBuilder`
- Error types: `PaladinError`, `LlmProviderError`, `ArsenalError`, `GarrisonError`
- Port trait types: `LlmPort`, `GarrisonPort`, `CitadelPort`, `ArsenalRegistry`, `SanctumPort`

**Enums and Variants:**
- Type name: PascalCase
- Variants: PascalCase: `PaladinStatus::Idle`, `ConversationRole::User`, `FinishReason::Stop`

**Type Parameters:**
- Single letters for simple generics: `Node<T>`, `Vec<T>`
- Descriptive names for complex scenarios: `Response`, `Port`

## Code Style

**Formatting:**
- Tool: `rustfmt` (standard Rust formatter)
- Line length: Approximately 100 characters (inferred from Cargo.toml profile: `codegen-units = 1` for profile.release)
- Indentation: 4 spaces (Rust standard)

**Linting:**
- Tool: `cargo clippy`
- Warnings treated as errors in CI (see Makefile: `clippy -- -D warnings`)
- All public items must have documentation: `#![warn(missing_docs)]`

**Common Traits to Derive:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]          // Most domain types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]             // Value objects
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]  // Entities with ordering
```

Examples from codebase:
- `crates/paladin-core/src/base/entity/node.rs`: `#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]`
- `crates/paladin-core/src/platform/container/log.rs`: Multiple derive attributes for different purposes

## Import Organization

**Order:**
1. Standard library (`std::`, `std::path::`, etc.)
2. External crate imports (`serde::`, `tokio::`, `uuid::`, etc.)
3. Async trait (`async_trait::`)
4. Internal crate imports (`crate::core::`, `crate::application::`, etc.)
5. Workspace dependencies (`paladin_core::`, `paladin_ports::`, etc.)

**Example from** `src/application/services/paladin/paladin_builder.rs`:
```rust
use crate::application::services::paladin::error::PaladinError;
use crate::core::base::entity::node::Node;
use crate::config::arsenal::MCPServerConfig;
use paladin_ports::output::llm_port::LlmPort;
use std::sync::Arc;
```

**Path Aliases:**
- None detected in standard configuration — all imports use absolute module paths
- Workspace crates imported directly: `use paladin_core::...;`, `use paladin_ports::...;`

## Error Handling

**Pattern:**
- Use `thiserror` crate for error type derivation: `#[derive(Debug, Error)]`
- Custom error enum per module (e.g., `PaladinError`, `LlmProviderError`, `ArsenalError`)
- Implement `From<LocalError>` at domain boundaries for error conversion

**Examples:**

Error type (from `crates/paladin-llm/src/error.rs`):
```rust
#[derive(Debug, Error, Clone)]
pub enum LlmProviderError {
    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,
}

impl From<LlmProviderError> for LlmError {
    fn from(err: LlmProviderError) -> Self {
        match err {
            LlmProviderError::AuthenticationError(msg) => LlmError::AuthenticationError(msg),
            // ... other mappings
        }
    }
}
```

**Guidelines:**
- Return `Result<T, E>` from fallible operations, never `Option`
- Use `?` operator for error propagation in functions returning `Result`
- No `unwrap()` or `panic!()` in library code — always return `Err`
- Provide context-rich error messages (e.g., `format!("Configuration error: {}", msg)`)

## Logging

**Framework:** `log` crate (v0.4.21) with `env_logger` (v0.11.3)

**Patterns:**
- Initialize with `env_logger::init()` in binary entry points
- Use `log::info!`, `log::warn!`, `log::error!`, `log::debug!` macros
- Not extensively used in library code — prefer `Result` types for error propagation
- Tracing available via `tracing-subscriber` (v0.3) for structured logging in specific adapters

## Comments

**When to Comment:**
- Explain *why* a design decision was made, not what the code does
- Document complex algorithms or non-obvious logic
- Explain constraints or workarounds

**JSDoc/TSDoc (Rust equivalent: rustdoc):**
- Use `///` for item documentation (functions, structs, enums, modules)
- Use `//!` for module-level documentation
- Provide examples in doc comments when behavior is non-obvious
- Mark with `#![warn(missing_docs)]` and document all public items
- Use `#[doc(hidden)]` to hide implementation details

**`# Examples` heading spelling (D-06, Phase 16 DOCS-03; refrozen Phase 36.1 D-11):** on the
D-05-enumerated **public API entry points** — `pub *Builder` structs, `pub *Port` traits, `pub
*Service` structs (see
`.planning/phases/36.1-deferred-items-closure/36.1-ENTRY-POINTS.md` for the full 101-item
snapshot at Phase 36.1 close) — the heading is spelled **`# Examples`** (plural), not
`# Example`. Both spellings render identically in rustdoc and neither is a compiler warning, so
this is house style scoped narrowly to those 101 items, not a tree-wide sweep: the roughly 285
other `# Example`/`# Examples` occurrences elsewhere in the tree are left as-is and are not
enforced. `scripts/check-public-api-examples.sh` is the mechanical enforcer for the plural rule on
the enumerated set — there is no stable-Rust lint for this (`rustdoc::missing_doc_code_examples`
is nightly-only), so the script is the honest fallback, not a workaround for a built-in that
exists elsewhere. The check is wired into `make clean-code` (via `make check-api-examples`), a
pre-push git hook, and the CI `lint` job, so drift between the rule and the tree cannot recur
silently (D-12).

**Example from** `src/application/services/paladin/paladin_builder.rs`:
```rust
/// Creates a new PaladinBuilder with default values
///
/// # Arguments
///
/// * `llm_port` - The LLM port implementation to use for this Paladin
///
/// # Examples
///
/// ```rust,no_run
/// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
/// # use paladin_ports::output::llm_port::LlmPort;
/// # use std::sync::Arc;
/// # fn example(llm_port: Arc<dyn LlmPort>) {
/// let builder = PaladinBuilder::new(llm_port);
/// # }
/// ```
pub fn new(llm_port: Arc<dyn LlmPort>) -> Self { ... }
```

*(Note: this map is dated 2026-07-30 and predates Phases 12-17 — most of its content has not been
re-verified against the tree since. This §Comments edit is scoped narrowly to the D-06 heading
rule; a full refresh of the whole map is `/gsd-map-codebase` work, explicitly out of scope for
Phase 16 per `16-CONTEXT.md`'s Deferred Ideas.)*

## Function Design

**Size:**
- Keep functions small and focused — typical range 10–50 lines
- Break complex logic into smaller helper functions
- Example: `PaladinExecutionService` decomposes execution into phases

**Parameters:**
- Prefer borrowing over owned values: use `&T` for reading, `&mut T` for mutation
- Avoid `bool` parameters — use enums or dedicated types instead
- Use `Arc` for shared references in async contexts (e.g., `Arc<dyn LlmPort>`)

**Return Values:**
- Always use `Result<T, E>` for fallible operations
- Async functions return `impl Future` or use `async fn` syntax with `#[async_trait]` for trait methods
- Streaming responses use `Box<dyn Stream<Item = Result<T, E>> + Send>`

**Examples from codebase:**
```rust
// Good: Parameters are references, clear return type
pub async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError>

// Good: Uses Arc for shared port dependency
pub fn new(
    llm_port: Arc<dyn LlmPort>,
    circuit_breaker: Arc<CircuitBreaker>,
    garrison: Option<Arc<dyn GarrisonPort>>,
) -> Self
```

## Module Design

**Exports:**
- Re-export public API from `mod.rs` files in each module
- Use `pub use` for commonly-needed types
- Hide implementation details with private modules

**Barrel Files:**
- Used in `tests/helpers/mod.rs` to re-export mocks:
  ```rust
  pub use mock_llm_adapter::{
      Invocation, MockLlmAdapter, MockResponse,
      create_mock_with_mixed_responses,
  };
  pub use mock_arsenal_adapter::MockArsenalPort;
  ```

**Module Structure Example:**
```
src/application/services/paladin/
├── mod.rs              # Barrel: re-exports builder, execution service, error
├── paladin_builder.rs  # PaladinBuilder implementation
├── error.rs            # PaladinError enum
└── paladin_execution_service.rs  # Service implementation
```

## Dependency Injection

**Pattern:**
- Constructor injection: services accept dependencies as `Arc` pointers
- Trait objects (`dyn Trait`) for polymorphism across provider implementations
- Example: `PaladinExecutionService::new(llm_port, garrison, arsenal_registry, embedding_port)`

**Hexagonal Architecture Constraints:**
- **Core layer** (`paladin-core`): No external dependencies, pure domain logic
- **Application layer** (`paladin-ports`): Defines port traits, depends only on core
- **Infrastructure layer**: Implements ports, depends on both core and ports
- **Never import infrastructure from core or ports** — dependency flows inward only

## Builder Pattern

**When to use:**
- For types with many optional fields (Paladin, Battalion configurations)
- For complex validation logic during construction

**Implementation (from** `PaladinBuilder`):
```rust
pub struct PaladinBuilder {
    llm_port: Arc<dyn LlmPort>,
    data: PaladinData,
    config: PaladinConfig,
    garrison: Option<Arc<dyn GarrisonPort>>,
    // ... more fields
}

impl PaladinBuilder {
    pub fn new(llm_port: Arc<dyn LlmPort>) -> Self { /* ... */ }

    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.data.system_prompt = prompt.into();
        self
    }

    pub async fn build(self) -> Result<Paladin, PaladinError> {
        self.validate()?;
        Ok(Paladin::new(self.data, self.config))
    }

    fn validate(&self) -> Result<(), PaladinError> {
        if self.data.system_prompt.is_empty() {
            return Err(PaladinError::ConfigurationError(
                "System prompt is required".into()
            ));
        }
        Ok(())
    }
}
```

## Async Patterns

**Framework:** `tokio` with `#[tokio::main]` or `#[tokio::test]` for tests

**Trait Pattern:**
- Use `#[async_trait]` from `async-trait` crate for trait methods:
```rust
#[async_trait]
pub trait LlmPort: Send + Sync {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError>;
}
```

**Guidelines:**
- All port traits are `Send + Sync` for thread safety
- Use `await` for blocking on futures
- Prefer `tokio::select!` for timeout logic over manual `Duration` handling

## Medieval Military Naming Convention

**Ubiquitous Language** — consistently used throughout codebase:
- **Paladin**: Autonomous AI agent (`core/platform/container/paladin.rs`)
- **Battalion**: Multi-agent orchestration group (`core/platform/container/battalion/`)
- **Garrison**: Memory/context storage (`core/platform/container/garrison.rs`)
- **Arsenal**: Tool registry and execution (`core/platform/container/arsenal.rs`)
- **Citadel**: State persistence system (`core/platform/container/citadel.rs`)
- **Sanctum**: Vector storage for RAG (`core/platform/container/sanctum.rs`)
- **Herald**: Output formatting (`core/platform/container/herald.rs`)
- **Formation**: Sequential agent execution (`core/platform/container/battalion/formation.rs`)
- **Phalanx**: Concurrent agent execution (`core/platform/container/battalion/phalanx.rs`)
- **Campaign**: Graph/DAG orchestration (`core/platform/container/battalion/campaign.rs`)
- **Armament**: Individual tool or capability (part of Arsenal)

---

*Convention analysis: 2026-07-30*
