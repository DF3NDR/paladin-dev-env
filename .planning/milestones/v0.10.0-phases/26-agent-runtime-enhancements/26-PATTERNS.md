# Phase 26: Agent Runtime Enhancements - Pattern Map

**Mapped:** 2026-09-07
**Files analyzed:** ~45 new/modified files implied by 26-CONTEXT.md (D-01…D-41)
**Analogs found:** 40+ / 45 (a handful of genuinely new shapes have no in-tree analog — see below)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `src/application/services/paladin/middleware/mod.rs` (trait `ExecutionMiddleware`, `MiddlewareFlow`, `ToolFlow`) | service (hook/port trait) | event-driven | `crates/paladin-battalion/src/engine/hooks.rs` (`NodeInterceptor`) | role-match |
| `src/application/services/paladin/middleware/chain.rs` (onion ordering, `build_chain`) | service | pipeline/event-driven | `crates/paladin-battalion/src/engine/hooks.rs` (`TraceDispatcher`) + `paladin_execution_service.rs` builder chain | role-match |
| `src/application/services/paladin/middleware/context.rs` (`ModelCallContext`, `ToolCallContext`, `LlmResponseView`, `FinalResult`, `PromptAssembly`) | model/value-object | transform | `crates/paladin-battalion/src/engine/node.rs` (`NodeContext`) | role-match |
| `src/application/services/paladin/middleware/limits.rs` (`ModelCallLimit`, `TokenBudget`, `ToolCallLimit`) | middleware | request-response | `crates/paladin-storage/src/... CircuitBreaker` (`src/infrastructure/resilience/circuit_breaker.rs`) for stateful-counter-with-limit shape | role-match |
| `src/application/services/paladin/middleware/guardrail.rs` (`Guardrail`, `GuardrailRule`) | middleware | transform | `crates/paladin-llm/src/redaction.rs` (regex/text-screen shape) | partial-match |
| `src/application/services/paladin/middleware/history.rs` (`HistoryTrimmer`) | service | transform/batch | `crates/paladin-memory/src/token_counter.rs` (`TokenCounter`/`TiktokenCounter`) | role-match |
| `src/application/services/paladin/middleware/summarization.rs` (`SummarizationMiddleware`) | middleware | transform | `src/application/services/memory_extraction_service.rs` (LLM-driven Garrison writer) | role-match |
| `src/application/services/paladin/middleware/vault_recall.rs` (`VaultRecallMiddleware`) | middleware | CRUD/request-response | `src/application/services/rag_retrieval_service.rs` (retrieval → assembly insertion) | exact |
| `src/application/services/paladin/middleware/resilience.rs` (`ModelRetryMiddleware`, `ModelFallbackMiddleware`) | middleware | request-response | `crates/paladin-llm/src/fallback.rs` (`FallbackLlmAdapter`) + `crates/paladin-battalion/src/engine/retry.rs` | exact |
| `src/application/services/paladin/middleware/tool_protocol.rs` (`ToolCallProtocolMiddleware`, `FinishOnPlainAnswerMiddleware`) | middleware | transform | `crates/paladin-battalion/src/engine/directive_parser.rs` (`extract_envelope`/`StructuredDirective`) | role-match |
| `crates/paladin-core/src/platform/container/execution_result.rs` (`StopReason` + `CallLimit`/`TokenBudget`) | model | CRUD | itself (existing enum; extend, non_exhaustive precedent = `PaladinError`/`LlmError`) | exact |
| `src/config/agent_runtime.rs` (`AgentRuntimeConfig` + sub-structs) | config | CRUD | `src/config/node_cache.rs` (`NodeCacheConfig`) | exact |
| `crates/paladin-ports/src/output/token_counter_port.rs` (`TokenCounterPort`) | service (port) | transform | `crates/paladin-ports/src/output/garrison_port.rs` (small sync-ish port trait) | role-match |
| `crates/paladin-memory/src/token_counter/heuristic.rs` (`HeuristicTokenCounter`) | utility | transform | `crates/paladin-memory/src/token_counter.rs` (`TokenCounter`/`TiktokenCounter`) | exact |
| `crates/paladin-core/src/platform/container/garrison.rs` (`GarrisonEntry.is_summary`, `::summary()`) | model | CRUD | itself (existing struct + constructors `new`/`with_metadata`/`with_token_count`) | exact |
| `crates/paladin-memory/src/adapters/sqlite_garrison.rs` (embedded migrator, `002_add_garrison_is_summary.sql`) | migration/adapter | CRUD | `crates/paladin-storage/src/waypoint/sqlite.rs` (`sqlx::migrate!("migrations/sqlite")` embedded migrator) | exact |
| `crates/paladin-core/src/platform/container/vault.rs` (`Namespace`, `VaultRecord`, `ScoredVaultRecord`, `Page`, `VaultError`) | model | CRUD | `crates/paladin-core/src/platform/container/garrison.rs` (value types + `thiserror` error enum) | exact |
| `crates/paladin-ports/src/output/vault_port.rs` (`VaultPort`) | service (port) | CRUD | `crates/paladin-ports/src/output/garrison_port.rs` (`GarrisonPort`) | exact |
| `crates/paladin-memory/src/vault/in_memory.rs` | service (adapter) | CRUD | existing `InMemoryGarrison` adapter (same crate) | exact |
| `crates/paladin-memory/src/vault/sqlite.rs` (+ `003_create_vault_tables.sql`) | service (adapter) | CRUD | `crates/paladin-storage/src/waypoint/sqlite.rs` (embedded-migrator SQLite adapter) | exact |
| `crates/paladin-memory/src/vault/semantic.rs` (`SemanticVault`) | service (adapter) | CRUD/event-driven | `crates/paladin-memory/src/sanctum/{in_memory_adapter,qdrant_adapter}.rs` composition pattern | role-match |
| `crates/paladin-memory/src/vault/contract_tests.rs` | test | CRUD | any existing Garrison contract-test module (adapter parity suite) | role-match |
| `src/application/services/paladin/vault_confined.rs` (`ConfinedVault`) | service (decorator) | CRUD | `crates/paladin-llm/src/fallback.rs` (`FallbackLlmAdapter` — decorator composing a port) | role-match |
| `crates/paladin-core/src/platform/container/run_scope.rs` (`RunScope`) | model | CRUD | `crates/paladin-core/src/platform/container/execution_result.rs` (`#[non_exhaustive]`, `Default` value type) | role-match |
| `src/application/services/arsenal/in_process_arsenal.rs` (`InProcessArsenal`) | service (adapter) | request-response | `src/application/services/arsenal/arsenal_execution_service.rs` (`ArsenalPort` impl routing to MCP) | role-match |
| `src/application/services/arsenal/composite_arsenal.rs` (`CompositeArsenalPort`) | service (adapter) | request-response | `crates/paladin-llm/src/fallback.rs` (chain-of-ports-as-one-port shape) | role-match |
| `src/application/services/arsenal/vault_tools.rs` (`VaultTools`: `vault_get`/`vault_put` Armaments) | service (adapter) | request-response | `src/application/services/arsenal/arsenal_execution_service.rs` (Armament invocation + JSON schema params) | role-match |
| `crates/paladin-core/src/platform/container/structured.rs` (`Structured<T>`, `StructuredOptions`, `SchemaRef`, `extract_json`, `shape_check`, `render_instruction_block`) | model/utility | transform | `crates/paladin-battalion/src/engine/directive_parser.rs` (`extract_envelope`) | exact (for `extract_json`) |
| `crates/paladin-ports/src/output/structured_executor_port.rs` (`StructuredExecutorPort`, `run_structured`) | service (port) | request-response | `crates/paladin-ports/src/output/llm_port.rs` (`LlmPort`, object-safe async trait) | role-match |
| `src/application/services/paladin/structured.rs` (`StructuredExecutorExt`) | service (extension trait) | transform | blanket-impl extension traits elsewhere in facade (none found identical — new shape) | partial-match |
| `crates/paladin-ports/src/output/llm_port.rs` (`LlmRequest::new` + builder methods, `response_format`) | model/builder | CRUD | itself (existing struct) + `PaladinBuilder` fluent-builder pattern (`paladin_builder.rs`) | exact |
| `crates/paladin-battalion/src/engine/graph.rs` (`NodeSpec::Paladin.output_schema`, `with_output_schema`) | model | CRUD | itself (existing `NodeSpec` constructor-preserved pattern) | exact |
| `crates/paladin-battalion/src/engine/mod.rs` (`WarEngine::with_structured_executor`, `with_output_schema`, `with_vault`) | service | CRUD | itself (`WarEngine::with_node_cache` sibling builder) | exact |
| `crates/paladin-llm/src/conformance.rs` (`ConformanceFixture`, `llm_conformance_suite!`) | test | request-response/streaming | existing per-adapter mockito test modules (openai_compatible/gemini/ollama) | exact |
| `src/infrastructure/adapters/arsenal/tool_result_formatter.rs` (`format_error`) | utility | transform | itself (existing formatter) | exact |
| `crates/paladin-llm/src/redaction.rs` (`redact_secret_patterns`) | utility | transform | itself (`redact_credentials`, `bounded_excerpt`) | exact |
| `crates/paladin-battalion/src/engine/retry.rs` (`RetryPredicate::admits`, refactor of `should_retry`) | utility | transform | itself | exact |
| `crates/paladin-core/src/platform/container/aegis.rs` (`RetryPolicy`) | model | CRUD | itself (unmodified, referenced) | exact |
| `src/presets/mod.rs` (`reasoning_agent`, `ReasoningAgentOptions`, `ReasoningAgent`) | service (preset/facade) | request-response | `src/application/use_cases/paladin/paladin_builder.rs` (`PaladinBuilder`) | role-match |
| `crates/doc-examples/src/agent_runtime.rs` | test/doc | request-response | `crates/doc-examples/src/fault_tolerance.rs` | exact |
| `docs/src/user-guides/agent-runtime.md` | doc | n/a | `docs/src/user-guides/fault-tolerance.md` / `parley-and-chronicle.md` | exact |

## Pattern Assignments

### `src/application/services/paladin/middleware/mod.rs` (service/port-trait, event-driven)

**Analog:** `crates/paladin-battalion/src/engine/hooks.rs`

**House pattern for a hook trait beside the thing it wraps** (module doc, lines 1-18):
```rust
//! Engine seams with no consumers yet ... the ordered [`NodeInterceptor`]
//! chain, and the cancellation-to-`Halted` path's supporting types.
```
Use this same "seam beside the loop it wraps" framing for `ExecutionMiddleware`'s module doc, citing `PaladinExecutionService::execute_internal`'s reasoning loop as the wrapped machinery (per CONTEXT D-01).

**Fire-and-forget / non-interference precedent** (lines 8-18): a new hook chain must document its cost model exactly like `TraceDispatcher` does — "the untraced path costs nothing" becomes, for middleware, "an empty chain reproduces today's bytes exactly" (D-02's golden equivalence test).

**Ordered, `Send + Sync` chain composition** — mirror `NodeInterceptor`'s registration idiom (chain stored as `Vec<Arc<dyn Trait>>`, iterated in fixed order) rather than inventing a new collection type.

---

### `src/application/services/paladin/middleware/chain.rs` (chain attachment, request-response)

**Analog:** `src/application/services/paladin/paladin_execution_service.rs` builder methods

**Chainable builder pattern to copy verbatim** (lines 238-259):
```rust
pub fn with_rag_retrieval(mut self, service: Arc<RagRetrievalService>) -> Self {
    info!("Attaching RAG retrieval service to PaladinExecutionService");
    self.rag_retrieval_service = Some(service);
    self
}

pub fn with_memory_extraction(mut self, service: Arc<MemoryExtractionService>) -> Self {
    info!("Attaching memory extraction service to PaladinExecutionService");
    self.memory_extraction_service = Some(service);
    self
}
```
`with_middleware` (appends to `Vec`) and `with_middleware_chain` (replaces) follow this exact shape — `mut self -> Self`, an `info!` log naming what was attached, one field write, return `self`. `with_herald` (line 286) and `with_vision_adapter` (line 319) are further instances of the same idiom to confirm consistency before adding a 12th `with_*` method.

---

### `crates/paladin-core/src/platform/container/execution_result.rs` (`StopReason`, CRUD/value-object)

**Analog:** itself

**Existing enum to extend, plus its exhaustiveness precedent** (lines 102-126):
```rust
pub enum StopReason {
    MaxLoops,
    StopWord(String),
    Completed,
    Timeout,
}

impl StopReason {
    pub fn is_successful(&self) -> bool {
        matches!(self, StopReason::Completed | StopReason::StopWord(_))
    }
    pub fn is_limit(&self) -> bool {
        matches!(self, StopReason::MaxLoops | StopReason::Timeout)
    }
}
```
Add `CallLimit` and `TokenBudget` variants; per D-07/D-08, `is_successful()` must return `true` and `is_limit()` must return `true` for both new variants — extend both `matches!` arms in the same commit. Note the sibling `PaladinResult` doc comment (lines 38-46) explaining why `PaladinResult` itself stays exhaustive/constructible while `StopReason` goes `#[non_exhaustive]` — do not conflate the two enums' semver treatment.

**In-tree exhaustive matchers that need a `_` arm** (named in CONTEXT D-07): `src/application/cli/formatters/output.rs:281-292,496-498` and `crates/paladin-web/src/agent_controller.rs:151-154`.

---

### `src/config/agent_runtime.rs` (`AgentRuntimeConfig`, config/CRUD)

**Analog:** `src/config/node_cache.rs`

**Disabled-by-default sub-config shape to copy exactly** (lines 1-18, 24-70):
```rust
//! Mirrors [`crate::config::waypoint_store::WaypointStoreConfig`]'s shape
//! field-for-field (`Default` + `validate()` + `EnvOverridable`). Defaults
//! to `enabled: false` ... so a v0.9 deployment ... boots v0.10 with
//! identical behavior -- X-09's "new subsystems are disabled by default"
//! requirement.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeCacheBackend { InMemory, Redis }

#[derive(Clone, Serialize, Deserialize)]
pub struct NodeCacheConfig {
    pub enabled: bool,
    pub backend: NodeCacheBackend,
    pub redis_host: String,
    pub redis_port: u16,
    pub redis_password: Option<String>,
    pub redis_db: u8,
    pub key_prefix: String,
}

impl Default for NodeCacheConfig {
    fn default() -> Self {
        Self { enabled: false, backend: NodeCacheBackend::InMemory, .. }
    }
}
```
`AgentRuntimeConfig`'s twelve sub-structs (`ModelCallLimitConfig`, `TokenBudgetConfig`, …) each get their own `enabled: false`, `Default` impl (manual, colocated with `validate()`), and `EnvOverridable` for scalar fields — this file is the template for all twelve.

**Manual `Debug` for a secret-shaped field** (lines ~90-100): copy this convention only if a future field is secret-shaped; per D-10 none is expected in `AgentRuntimeConfig`, so state that explicitly in the module doc the way this file states its own rationale.

---

### `crates/paladin-storage/src/waypoint/sqlite.rs` (embedded migrator, CRUD)

**Analog:** itself, for both `SqliteVault` and the `SqliteGarrison` migration switch

**Embedded-migrator declaration to copy verbatim** (line 95):
```rust
static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("migrations/sqlite");
```
Apply the identical pattern for `crates/paladin-memory`'s migrator (D-17/D-23: `sqlx::migrate!("migrations")` embedded at compile time, replacing `sqlite_garrison.rs:108`'s `Migrator::new("./migrations")` runtime path). One migrator serves both `SqliteGarrison` and `SqliteVault` since they live in the same crate — point both at the same embedded `MIGRATOR` static, one call per constructor, idempotent on repeated construction (this file's own "constructs twice idempotently" test is the precedent named in D-23).

---

### `crates/paladin-llm/src/fallback.rs` (`ModelFallbackMiddleware`/`ModelRetryMiddleware`, request-response)

**Analog:** itself

**Composable-port-over-a-chain shape** (lines 1-50, 140-200):
```rust
pub fn new(chain: Vec<Arc<dyn LlmPort>>) -> Result<Self, FallbackChainError> {
    if chain.is_empty() { return Err(FallbackChainError::EmptyChain); }
    Ok(Self { chain, trace_sink: None })
}

pub fn with_trace_sink(mut self, sink: Arc<dyn TraceSink>) -> Self {
    self.trace_sink = Some(sink);
    self
}
```
`ModelFallbackMiddleware::new(chain: Vec<Arc<dyn LlmPort>>)` builds one `FallbackLlmAdapter` at construction (D-11) — reuse this constructor and its empty-chain validation instead of re-implementing chain logic in the middleware.

**Permanent-short-circuits, Transient-or-Unknown-hops classification rule** (module doc lines 1-8): the same `LlmError::transience()` dispatch that `after_failure` uses is exactly what `RetryPredicate::admits(Transience) -> bool` in `paladin_core::aegis` must express as a pure function — lift, don't duplicate (D-11).

**Per-hop observability via optional trace sink, discarding sink errors** (lines ~175-190):
```rust
async fn record_hop(&self, from: &'static str, to: &'static str, err: &LlmError) {
    log::warn!("Fallback chain hopping from provider '{from}' to '{to}' after {transience:?} error: {err}", ..);
    let Some(sink) = &self.trace_sink else { return; };
    let event = TraceEvent::FallbackHop { .. };
    if let Err(sink_err) = sink.on_event(event).await {
        log::debug!("trace sink rejected FallbackHop event: {sink_err}");
    }
}
```
This "never let a sink failure propagate" idiom applies anywhere a middleware emits an optional trace/log event.

---

### `crates/paladin-llm/src/redaction.rs` (`redact_secret_patterns`, transform)

**Analog:** itself

**Redact-then-bound ordering rule (security-critical, load-bearing)** (module doc lines 1-11):
```rust
//! **Ordering is load-bearing: redact, then bound.** Bounding a response body
//! before redaction can slice a secret in half at the truncation boundary and
//! leak the surviving prefix. Every call site in this crate MUST call
//! [`redact_credentials`] before [`bounded_excerpt`], never the reverse.
```
`redact_secret_patterns` (D-34) is the pattern-only half of `redact_credentials` (lines ~107-118) factored out — no `api_key` argument, same three-pass structure (bearer tokens, `sk-`/`sk-ant-`/`AKIA`-style keys, JWT triples). `ToolResultFormatter::format_error` must call `redact_secret_patterns` before `bounded_excerpt`, exactly this file's own rule.

**Panic-free byte/char-boundary handling** (lines 40-52, `bounded_excerpt`): copy the `.chars().take(budget)` approach — never slice a `&str` by raw byte offset in new formatter code.

---

### `crates/paladin-core/src/platform/container/garrison.rs` (`GarrisonEntry.is_summary`, CRUD)

**Analog:** itself

Existing struct has six pub fields and three constructors (`new`, `with_metadata`, `with_token_count`) per CONTEXT D-17 — add `#[serde(default)] pub is_summary: bool`, mark `#[non_exhaustive]`, add `GarrisonEntry::summary(content)` following the same constructor-function idiom as `with_metadata`/`with_token_count` (read those three constructors directly when implementing; they were not re-excerpted here to avoid a redundant read — same file, adjacent lines).

---

### `crates/paladin-ports/src/output/garrison_port.rs` (`GarrisonPort`, `VaultPort` analog, CRUD)

**Analog:** itself, for `VaultPort`'s shape

**Small async CRUD-ish port trait to mirror** (lines 380-491):
```rust
pub trait GarrisonPort: Send + Sync {
    async fn remember(&self, entry: GarrisonEntry) -> Result<(), GarrisonError>;
    async fn recall_recent(&self, limit: usize) -> Result<Vec<GarrisonEntry>, GarrisonError>;
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<GarrisonEntry>, GarrisonError>;
    async fn forget_all(&self) -> Result<(), GarrisonError>;
    async fn stats(&self) -> Result<GarrisonStats, GarrisonError>;
}
```
`VaultPort` (put/get/delete/list/search) is the same shape at one more level of dimensionality (namespace-scoped). Model `VaultError` on the existing `GarrisonError`'s `thiserror` structure (not excerpted here — same file, read alongside the trait when implementing).

---

### `crates/paladin-ports/src/output/llm_port.rs` (`LlmRequest`, model/builder)

**Analog:** itself

**Struct to extend and the constructor gap it currently has** (lines 627-641):
```rust
pub struct LlmRequest {
    pub id: Uuid,
    pub model: String,
    pub prompt: PromptItem,
    pub attachments: Vec<ContentItem>,
    pub stream: bool,
    pub metadata: HashMap<String, String>,
}
```
No `Default`, no constructor today — every one of the 37 call sites uses a full struct literal (D-28). Add `pub response_format: Option<ResponseFormat>` with `#[serde(default)]`, mark `#[non_exhaustive]`, and add `LlmRequest::new(model, prompt)` + chainable `with_attachments`/`with_stream`/`with_metadata`/`with_response_format` — copy the chainable-builder idiom from `paladin_execution_service.rs`'s `with_*` methods (see chain.rs pattern above) for each `with_*` method's shape (`mut self -> Self`, one field write).

---

### `src/application/services/arsenal/arsenal_execution_service.rs` (`InProcessArsenal`/`VaultTools`, request-response)

**Analog:** itself

Every shipped `ArsenalPort` in the tree routes to an MCP client (`arsenal_execution_service.rs:184-230`) — read that block directly when building `InProcessArsenal`'s `invoke`/`validate_call`/`list_armaments` methods, since `InProcessArsenal` is the in-process counterpart of the same trait with async closures standing in for the MCP round-trip. `VaultTools::new(confined)` builds two `Armament` definitions with JSON-Schema `parameters` the same way the MCP adapter surfaces tool schemas from the server's tool-list response — same `Armament { name, description, parameters }` shape, different origin.

---

### `crates/doc-examples/src/fault_tolerance.rs` (anchor-region doc-example precedent)

**Analog:** itself (pattern only — file exists in `crates/doc-examples/src/`)

Copy the `// ANCHOR:` / `// ANCHOR_END:` region convention and the `cargo check -p paladin-doc-examples` CI gate for `crates/doc-examples/src/agent_runtime.rs`'s `reasoning_agent` example (D-35), and the `{{#include}}` inclusion convention in `docs/src/user-guides/fault-tolerance.md` for the new `docs/src/user-guides/agent-runtime.md` page (D-38).

---

## Shared Patterns

### Chainable builder (`with_*`)
**Source:** `src/application/services/paladin/paladin_execution_service.rs:238-259, 286-300`
**Apply to:** every new `PaladinExecutionService::with_middleware`, `with_middleware_chain`, `with_token_counter`, `with_vault`, `with_tool_error_config` method.
```rust
pub fn with_rag_retrieval(mut self, service: Arc<RagRetrievalService>) -> Self {
    info!("Attaching RAG retrieval service to PaladinExecutionService");
    self.rag_retrieval_service = Some(service);
    self
}
```

### Disabled-by-default config sub-struct
**Source:** `src/config/node_cache.rs` (whole file)
**Apply to:** all twelve `AgentRuntimeConfig` sub-structs — `Default` (manual, colocated with `validate()`), `enabled: false`, `EnvOverridable` for scalar fields, hand-written `Debug` only if a secret-shaped field exists.

### Redact-then-bound
**Source:** `crates/paladin-llm/src/redaction.rs` (module doc + `bounded_excerpt`/`redact_credentials`)
**Apply to:** `ToolResultFormatter::format_error` (D-34), any Guardrail/Vault-recall text inserted into or read from the prompt, any new provider-conformance error mapping.

### Chain-of-ports-as-one-port (fallback/composite)
**Source:** `crates/paladin-llm/src/fallback.rs`
**Apply to:** `ModelFallbackMiddleware`, `CompositeArsenalPort`, `SemanticVault`'s composition of `VaultPort` + `SanctumPort` + `EmbeddingPort`.

### Embedded compile-time SQL migrator
**Source:** `crates/paladin-storage/src/waypoint/sqlite.rs:95` (`static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("migrations/sqlite");`)
**Apply to:** `crates/paladin-memory`'s switch off the runtime `Migrator::new("./migrations")` path (D-17), and `SqliteVault`'s `003_create_vault_tables.sql` (D-23) sharing the same embedded migrator.

### `#[non_exhaustive]` + semver-allowlist treatment
**Source:** `crates/paladin-core/src/platform/container/execution_result.rs` module doc (lines 38-46) explaining why `PaladinResult` stays exhaustive while its sibling enum does not — the same reasoning the D-07/D-17/D-28 decisions apply to `StopReason`, `GarrisonEntry`, `LlmRequest`.
**Apply to:** every struct/enum CONTEXT flags `Y` in §9.2 (`StopReason`, `GarrisonEntry`, `LlmRequest`, new `VaultError`, `RunScope`, `ResponseFormat`).

### Object-safe async port + blanket-impl generic extension
**Source:** `crates/paladin-ports/src/output/llm_port.rs` (`LlmPort` trait) alongside `crates/paladin-ports/src/output/garrison_port.rs` (`GarrisonPort`)
**Apply to:** `StructuredExecutorPort` (JSON-level, object-safe) + `StructuredExecutorExt` (generic blanket impl) — same two-layer shape PRD 05 sketches.

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `src/application/services/paladin/middleware/context.rs`'s typed per-run scratch bag (`state::<T>()`/`state_mut::<T>()` keyed by middleware name + `TypeId`) | model | transform | No existing typed heterogeneous-bag pattern in the tree; nearest relative is the untyped `scratch: HashMap<String, serde_json::Value>` the PRD itself specifies — build from the PRD sketch, not a codebase analog. |
| `crates/paladin-core/src/platform/container/structured.rs`'s `shape_check` (hand-rolled partial JSON-Schema validator) | utility | transform | No JSON-Schema-shaped validator exists anywhere in-tree (the `jsonschema` crate is explicitly rejected, D-30); this is genuinely new logic, guided by the PRD's enumerated rule list, not an analog. |
| `src/presets/mod.rs` (`reasoning_agent`) top-level module | service (preset) | request-response | `paladin-battalion`/facade has no prior "one-liner preset function returning a runnable pair" — `PaladinBuilder` is the closest relative (see Pattern Assignments) but is a multi-step builder, not a single preset function; treat the PRD's ≤15-line example (D-35) as the primary spec. |
| `crates/paladin-llm/src/conformance.rs`'s `ConformanceFixture` trait + `llm_conformance_suite!` macro | test | request-response/streaming | No existing shared-macro test-suite generator in `paladin-llm`; each adapter currently hand-writes its own mockito tests. Building the macro is new infrastructure — use the per-adapter test modules only as the source of *case content* (success/stream/error bodies), not structural pattern. |

## Metadata

**Analog search scope:** `crates/paladin-core`, `crates/paladin-ports`, `crates/paladin-battalion`, `crates/paladin-llm`, `crates/paladin-memory`, `crates/paladin-storage`, `src/application/services`, `src/config`, `src/infrastructure/adapters`
**Files scanned:** ~30 read/grepped directly; ~15 more located by path only (sufficient for role/data-flow classification per PRD 05's own trait sketches)
**Pattern extraction date:** 2026-09-07
