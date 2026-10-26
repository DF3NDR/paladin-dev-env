# Phase 38: Design Seams & Pricing/Cost Producer - Pattern Map

**Mapped:** 2026-09-25
**Files analyzed:** 12 (2 ADR docs + 10 code files, per D-16 the ADRs are 38-01 and gate the rest)
**Analogs found:** 10 / 10 code files (ADRs have no code analog; they follow `PROMOTION.md`'s
required-headings template instead)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|--------------------|------|-----------|-----------------|---------------|
| `.planning/decisions/0052-mid-run-treasurer-enforcement.md` (new) | ADR/doc | — | `.planning/decisions/0050-treasurer-reservation.md` / `0051-*.md` | exact (template) |
| `.planning/decisions/0053-ledger-balance-model.md` (new) | ADR/doc | — | `.planning/decisions/0049-commissary-design-and-rename.md` | exact (template) |
| `crates/paladin-core/src/platform/container/cost.rs` (new) | model / value-type | transform | `crates/paladin-core/src/platform/container/token_usage.rs` | exact |
| `src/config/treasurer.rs` (new) | config | request-response (boot-time validate) | `src/config/agent_runtime.rs` (`TokenBudgetConfig`) | exact |
| `src/config/settings.rs` (modified) | config | request-response | itself — `pub agent_runtime: AgentRuntimeConfig` wiring | exact |
| `crates/paladin-llm/src/pricing.rs` (new) | service / decorator | request-response + streaming | `crates/paladin-llm/src/fallback.rs` (`FallbackLlmAdapter`) | exact |
| `crates/paladin-ports/src/output/llm_port.rs` (modified: `LlmResponse.cost`) | model (port DTO) | CRUD-ish (field add) | itself — `LlmResponse.usage: TokenUsage` field | exact |
| `crates/paladin-core/src/platform/container/execution_result.rs` (modified: `PaladinResult.cost`) | model | transform | itself — `PaladinResult.usage: TokenUsage` field (`#[serde(default)]`) | exact |
| `crates/paladin-core/src/platform/container/trace.rs` (modified: `NodeFinished.cost`, `RunFinished.cost`) | model / event | event-driven | itself — `NodeFinished.usage`, `RunFinished.usage` | exact |
| `crates/paladin-battalion/src/engine/hooks.rs` (modified: `TraceDispatcher::total_cost`) | service | event-driven / aggregation | itself — `TraceDispatcher::total_usage()` | exact |
| `crates/paladin-core/src/platform/container/herald.rs` (modified: rustdoc only) | model / docs | — | itself — existing `cost_estimate` field docs | exact |
| `crates/paladin-herald/src/markdown_herald.rs` (modified: `finalize_stream`) | presentation | transform | itself — existing `cost_estimate` branch | exact |
| `crates/paladin-herald/src/json_herald.rs` (modified: `finalize_stream`) | presentation | transform | itself — existing `cost_estimate` JSON field | exact |
| `crates/paladin-herald/src/table_herald.rs` (modified: `finalize_stream`) | presentation | transform | `markdown_herald.rs::finalize_stream` (same trait method, real analog to imitate since table's own body is a stub) | role-match (source stub, not a working analog) |
| `src/infrastructure/web/facade_provisioner.rs` (modified: wrap `LlmPort`) | wiring / composition root | request-response | itself — `paladin_port_from_settings` | exact |
| `src/infrastructure/web/agent_host.rs` (modified: wrap `LlmPort`) | wiring / composition root | request-response | itself — `build_agent` / `build_agent_with_llm` | exact |
| new `ExecutionMetadata` producer (engine: hook into `RunFinished` handling; agent loop: stream-completion handler) | service / producer | event-driven → transform | No analog exists in production (Pitfall 5) — model on `ExecutionMetadataBuilder`'s own test-module usage | no analog (see below) |

## Pattern Assignments

### `crates/paladin-core/src/platform/container/cost.rs` (model, transform)

**Analog:** `crates/paladin-core/src/platform/container/token_usage.rs`

**Struct shape** (lines 22-45 of `token_usage.rs`):
```rust
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    #[serde(default)]
    pub cache_read_tokens: Option<u32>,
    #[serde(default)]
    pub cache_write_tokens: Option<u32>,
    #[serde(default)]
    pub reasoning_tokens: Option<u32>,
}
```
`Cost` copies this one-for-one: `nanos: i64` (saturating, like the `u32` fields) plus
`currency: CurrencyCode`, deriving `Debug, Clone, Copy, PartialEq, Serialize, Deserialize`.
`Add`/`AddAssign`/`Sum` must use `saturating_add` exactly like `TokenUsage`'s `Option<u32>`
merge rule (`None+None=None`, `None+Some=Some`, `Some+Some=saturating_add`) — copy the existing
`option_merge_*` test names/shapes from `token_usage.rs`'s test module for `Cost`'s own
`#[cfg(test)]` module.

**Cost function core pattern (D-06 formula, no existing analog — new pure math):**
```rust
// (prompt − cache_read − cache_write) × prompt_price
//   + cache_read × cache_read_price + cache_write × cache_write_price
//   + (completion − reasoning) × completion_price + reasoning × reasoning_price
// All subtractions: saturating_sub. All products: i128 intermediate, then
// divide-by-1_000_000 with half-up rounding, then cast down to i64 nanos.
```
Pitfall 2 (research): cache/reasoning sub-counts are INCLUSIVE of prompt/completion, not
additive — must subtract before pricing the base rate or tokens are double-billed.

---

### `src/config/treasurer.rs` (config, request-response)

**Analog:** `src/config/agent_runtime.rs` (`TokenBudgetConfig`, lines 492-531)

**Full idiom to mirror:**
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenBudgetConfig {
    pub enabled: bool,
    pub max_tokens: u32,
}

impl Default for TokenBudgetConfig {
    fn default() -> Self {
        Self { enabled: false, max_tokens: 100_000 }
    }
}

impl TokenBudgetConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.enabled && self.max_tokens == 0 {
            return Err(
                "agent_runtime.token_budget.max_tokens must be greater than 0 when enabled"
                    .to_string(),
            );
        }
        Ok(())
    }
}

impl EnvOverridable for TokenBudgetConfig {
    fn apply_env_overrides(&mut self) {
        if let Some(v) = read_env::<bool>("APP_AGENT_RUNTIME_TOKEN_BUDGET_ENABLED") {
            self.enabled = v;
        }
        if let Some(v) = read_env::<u32>("APP_AGENT_RUNTIME_TOKEN_BUDGET_MAX_TOKENS") {
            self.max_tokens = v;
        }
    }
}
```
`TreasurerConfig` mirrors this exactly: `Default` → `currency: "USD"`, `pricing: HashMap::new()`;
`validate()` error strings use the same `"treasurer.<path> must be ..."` prefix convention
(e.g. `"treasurer.currency must be exactly three ASCII uppercase letters"`,
`"treasurer.pricing.<model>.prompt must be a non-negative decimal string"`); `EnvOverridable`
covers `currency` only (`APP_TREASURER_CURRENCY`) — the map-shaped `pricing` field has no env
form, mirroring `ToolCallLimitConfig.per_tool`'s existing config-file-only collection precedent
(`src/config/agent_runtime.rs` line ~538+).

**Settings wiring pattern** (`src/config/settings.rs` lines 4, 29-66, 386):
```rust
use crate::config::agent_runtime::AgentRuntimeConfig;
// ...
pub struct Settings {
    // ...
    pub agent_runtime: AgentRuntimeConfig,
}
// Default impl:
agent_runtime: AgentRuntimeConfig::default(),
```
Add `pub treasurer: TreasurerConfig` beside `agent_runtime` the same way, and call
`TreasurerConfig::validate()` alongside the existing `AgentRuntimeConfig`/`TraceConfig`
validation call (settings.rs line ~98 comment names the validated set).

---

### `crates/paladin-llm/src/pricing.rs` (service/decorator, request-response + streaming)

**Analog:** `crates/paladin-llm/src/fallback.rs` (`FallbackLlmAdapter`)

**Struct + trait impl shape** (lines 134, 254-340):
```rust
pub struct FallbackLlmAdapter {
    chain: Vec<Arc<dyn LlmPort>>,
    trace_emitter: Option<Arc<dyn TraceEmitter>>,
}

#[async_trait]
impl LlmPort for FallbackLlmAdapter {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> { /* ... */ }
    async fn generate_stream(&self, /* ... */) { /* ... */ }
    fn get_provider_name(&self) -> &'static str { /* delegates */ }
}
```
`PricingLlmAdapter` wraps a single `Arc<dyn LlmPort>` (not a `Vec`) plus an immutable
`PriceTable` field, with a `HashSet`/`OnceLock`-backed warn-once set for D-08 — this mutable
field is the ONLY mutable state, matching the codebase rule "per-run scratch state never lives
on a shared middleware/decorator struct" (already established for `limits.rs`). `generate`
prices the returned `LlmResponse` by its own `model`/`usage`; `generate_stream` prices only the
terminal chunk (the one where `StreamingResponse.usage` is `Some`, per `llm_port.rs`'s own
doc comment). Composition order: wrap OUTSIDE any `FallbackLlmAdapter`
(`Pricing(Fallback(...))`, not the reverse) so the priced response is the one actually served.

**Wiring/installation pattern** (verified production call sites):
```rust
// src/infrastructure/web/facade_provisioner.rs — engine path
let llm = factory.create(&provider)
    .map_err(|source| HostBuildError::Provider { id: "run-engine".into(), provider, source })?;
// wrap HERE before PaladinExecutionService::new(llm, ...)

// src/infrastructure/web/agent_host.rs — agent-loop path
pub(crate) async fn build_agent_with_llm(
    def: &AgentDefinition,
    llm: Arc<dyn LlmPort>,   // wrap upstream of this call, at the factory.create() call site
    breaker: Arc<CircuitBreaker>,
) -> Result<BuiltAgent, HostBuildError> { /* ... */ }
```
Skip wrapping when `price_table.is_empty()` (return the inner `llm` unchanged) — avoids a
zero-effect layer, per the discretion note and the illustrative snippet in RESEARCH.md.

---

### `crates/paladin-ports/src/output/llm_port.rs`, `execution_result.rs`, `trace.rs` (model, event-driven field additions)

**Analog:** each file's own existing `usage: TokenUsage` field, using the exact `#[serde(default)]`
additive-field precedent (D-25):
```rust
// crates/paladin-core/src/platform/container/execution_result.rs line ~60
#[serde(default)]
pub usage: TokenUsage,
```
Add `#[serde(default)] pub cost: Option<Cost>` beside `usage` in all four locations:
`LlmResponse` (`llm_port.rs` line 872), `PaladinResult` (`execution_result.rs` line 61),
`TraceEvent::NodeFinished` and `TraceEvent::RunFinished` (`trace.rs` lines 232, 294-300 —
`RunFinished.usage` doc comment already says "`TokenUsage` sum over every `NodeFinished.usage`
this run's..."; `cost` gets an identical sibling doc line).

---

### `crates/paladin-battalion/src/engine/hooks.rs` (service, event-driven aggregation)

**Analog:** itself — `TraceDispatcher::total_usage()` (line 388), called synchronously inside
`emit()` (line 293).

Add `total_cost() -> Option<Cost>` as a direct twin: same synchronous-aggregation-inside-`emit()`
placement, same `None`-propagation rule (if any priced call was unpriced, the run total is
`None` — NOT a partial sum, per Pitfall 3/D-08).

---

### `crates/paladin-core/src/platform/container/herald.rs` (rustdoc-only change)

**Current text to change** (lines 387, 459, 504-505, 532-536):
```rust
/// * `cost_estimate` - Reserved for the Treasurer (Milestone 14 / FUT-08); no in-tree producer yet
///     .cost_estimate(0.045)  // illustrative value; reserved for the Treasurer (Milestone 14 / FUT-08)
/// Reserved for the Treasurer (Milestone 14 / FUT-08); no in-tree producer yet.
pub cost_estimate: Option<f64>,
```
D-12 requires every one of these become "produced by the Treasurer" and the `.cost_estimate(0.045)`
illustrative doc example become a real, executable one built from an actual `Cost` value
(`nanos as f64 / 1e9`, D-03's display-edge conversion). No field/type change — `cost_estimate`
stays `Option<f64>`.

---

### `crates/paladin-herald/src/markdown_herald.rs::finalize_stream` (presentation, transform)

**Current code (lines 412-427, already reads the field):**
```rust
if let Some(cost) = metadata.cost_estimate {
    output.push_str(&self.format_field("Cost", &format!("${:.4}", cost)));
}
```
Change to D-04's currency-aware format (`"0.0450 USD"`, never a hard-coded `$`) — needs the
currency string threaded alongside `cost_estimate` (from `ExecutionMetadata`, which the
producer populates from the run's `Cost.currency`).

### `crates/paladin-herald/src/json_herald.rs::finalize_stream` (presentation, transform)

**Current code (line 212, already emits the field):**
```rust
"cost_estimate": metadata.cost_estimate,
```
Add a `"currency"` sibling field per D-04; keep the numeric field as-is (no API break for
existing JSON consumers).

### `crates/paladin-herald/src/table_herald.rs::finalize_stream` (presentation, transform — STUB, larger diff)

**Current code is a stub (lines 266-290) — ignores `_metadata` entirely:**
```rust
fn finalize_stream(
    &self,
    _metadata: &paladin_core::platform::container::herald::ExecutionMetadata,
) -> Result<String, HeraldError> {
    let mut table = self.create_table();
    table.set_header(vec![Cell::new("Metric")..., Cell::new("Value")...]);
    // Add placeholder metadata (will be replaced with actual metadata)
    table.add_row(vec!["Total Duration", "3.45s"]);
    table.add_row(vec!["Total Tokens", "950"]);
    table.add_row(vec!["Paladins Executed", "2"]);
    table.add_row(vec!["Success Rate", "100%"]);
    // ...
}
```
Pitfall 4 (research, verified): this must be wired to actually consume `metadata` for the first
time (real duration, real token usage, real cost row) as its own task, not folded into "add a
cost row" sizing — imitate `markdown_herald.rs`'s pattern of reading real `ExecutionMetadata`
fields (`metadata.model_used`, `metadata.duration_ms`, `metadata.token_usage`,
`metadata.cost_estimate`) since `table_herald.rs`'s own body has never done this.

---

### New `ExecutionMetadata` producer (engine + agent loop) — NO PRODUCTION ANALOG

Verified (Pitfall 5): `ExecutionMetadata::builder()` and `Herald::finalize_stream(` are called
today ONLY from each herald's own `#[cfg(test)]` module and from `examples/`/`doc-examples/`.
There is no production call to imitate. Model the shape on `ExecutionMetadataBuilder`'s own
fluent-builder API (already defined, lines 542-628 of `herald.rs`):
```rust
ExecutionMetadata::builder()
    .model_used(/* engine: RunFinished's model or "mixed"; agent loop: the served model */)
    .token_usage(/* RunFinished.usage or PaladinResult.usage */)
    .cost_estimate(cost.map(|c| c.nanos as f64 / 1e9).unwrap_or_default_or_none_per_D03)
    .build()
```
Wire the engine producer into the `RunFinished` trace-event handling site in
`crates/paladin-battalion/src/engine/mod.rs` (~line 2089, `RunFinished` emission site named in
CONTEXT.md's canonical refs); wire the agent-loop producer into wherever a streamed execution
currently completes without building metadata (`PaladinExecutionService` / agent-loop
stream-completion path). Each hands its `ExecutionMetadata` to `Herald::finalize_stream`.

## Shared Patterns

### Config sub-struct idiom (`Default` + `validate()` + `EnvOverridable`)
**Source:** `src/config/agent_runtime.rs` (`TokenBudgetConfig`, lines 492-531)
**Apply to:** `src/config/treasurer.rs` (`TreasurerConfig`, `PriceRowConfig`)
See full excerpt under the `treasurer.rs` section above.

### Saturating accumulation with `Option<T>` sub-count merge semantics
**Source:** `crates/paladin-core/src/platform/container/token_usage.rs` (struct + `Add`/`Sum` impls)
**Apply to:** `crates/paladin-core/src/platform/container/cost.rs` (`Cost`)

### `#[serde(default)]` additive-field precedent (D-25)
**Source:** `crates/paladin-core/src/platform/container/execution_result.rs` (`PaladinResult.usage`)
**Apply to:** `LlmResponse.cost`, `PaladinResult.cost`, `TraceEvent::NodeFinished.cost`,
`TraceEvent::RunFinished.cost` — all four new fields, so pre-existing serialized documents
deserialize with `cost == None`.

### `LlmPort`-wrapping decorator with construction-time error type, no cross-call mutable state
**Source:** `crates/paladin-llm/src/fallback.rs` (`FallbackLlmAdapter`)
**Apply to:** `crates/paladin-llm/src/pricing.rs` (`PricingLlmAdapter`)

### `None` propagation, never `unwrap_or(0)` (Pitfall 3/6)
**Source:** D-08's rule, enforced at the decorator itself
**Apply to:** every consumer of `Cost`/`cost_estimate`: `TraceDispatcher::total_cost()`, herald
`finalize_stream` implementations, the future ledger (Phase 39) — none may substitute a zero
default for an unpriced/`None` cost.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| Engine-path `ExecutionMetadata` producer (hook into `RunFinished` handling, `paladin-battalion/src/engine/mod.rs`) | service/producer | event-driven | No production caller of `ExecutionMetadata::builder()`/`Herald::finalize_stream` exists today (Pitfall 5); model on the builder's own test-module usage, not a production analog |
| Agent-loop stream-completion `ExecutionMetadata` producer | service/producer | streaming → transform | Same — genuinely new integration work, not a field addition to an existing call site |
| `.planning/decisions/0052-*.md`, `0053-*.md` (ADR text bodies) | doc | — | Structural template exists (`PROMOTION.md`'s required headings, `0049`/`0050`/`0051` as worked examples) but the Context/Decision/Consequences prose is necessarily new per-ADR content, not copy-from-analog code |

## Metadata

**Analog search scope:** `crates/paladin-core/src/platform/container/`, `crates/paladin-ports/src/output/`,
`crates/paladin-llm/src/`, `crates/paladin-battalion/src/engine/`, `crates/paladin-herald/src/`,
`src/config/`, `src/infrastructure/web/`, `.planning/decisions/`
**Files scanned:** 10 code files read/grepped directly against the shipped tree (2026-09-25) plus
CONTEXT.md/RESEARCH.md for the phase's own prior verified findings
**Pattern extraction date:** 2026-09-25
