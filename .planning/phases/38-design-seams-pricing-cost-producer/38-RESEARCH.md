# Phase 38: Design Seams & Pricing/Cost Producer - Research

**Researched:** 2026-09-25
**Domain:** Fixed-point currency pricing for LLM token usage; two gating architecture ADRs (mid-run
enforcement attachment point, ledger balance model) in a Rust hexagonal multi-agent runtime
**Confidence:** HIGH (every architectural and code-shape claim below is grounded in a direct `Read`/
`Grep` of the shipped tree at this session's HEAD, 2026-09-25); MEDIUM only for the one new
dependency's version currency (web-verified, not Context7-verified)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Carried forward (locked by earlier phases and milestone-level decisions — not re-asked):**
- **D-00a:** Full governance (per-tenant + per-API-key) is in scope for the milestone; prices come
  from operator config only, nothing bundled.
- **D-00b:** One operator currency in v0.11.0. Multi-currency/FX (FUT-12) and long-context tier
  pricing (FUT-11) are v2 — do not design hooks for them.
- **D-00c:** An unpriced model yields `None`, never `0` (PRICE-03). Internal currency math is
  integer fixed-point; `f64` is never accumulated or compared.
- **D-00d:** `Treasurer` is a framework-only word (ADR-0050). `TokenBudget`, `TokenCounterPort`,
  `TokenUsage` and `max_tokens` are not renamed (ADR-0049, ADR-0050). The Commissary is untouched.
- **D-00e:** Vocabulary rules hold in every new line: Medieval-military terms for domain roles; no
  bare token total beside a `TokenUsage` split.
- **D-00f:** ADRs use the flat zero-padded counter in `.planning/decisions/PROMOTION.md`; next free
  numbers are 0052 and 0053, index updated in the same commit.
- **D-00g:** X-03 governs public API: any break needs a `MIGRATION.md` §9.2 row and a
  `cargo semver-checks` allowlist entry. This phase is designed to be **additive** so no such row
  is expected.
- **D-00h:** Config sub-structs mirror `AgentRuntimeConfig`: `Default` + `validate()` +
  `EnvOverridable` with `APP_*` env vars, inert when the section is omitted, `validate()` never
  clamps.
- **D-00i:** Shipped tree outranks any document; the settings root is `src/config/settings.rs`.
- **D-00j:** 82% workspace line-coverage floor (ADR-0006); `make clean-code`, `make api-surface`,
  `make security`, and the manual credential-handling review gate every commit.

**Price units & rounding:**
- **D-01:** Unit prices are written **per 1M tokens** as decimal strings.
- **D-02:** Internal cost unit is **nano-units of the currency (1e-9) in `i64`**. Each LLM call is
  rounded **once, half-up, to the nearest nano-unit**; run totals are exact integer sums
  (saturating). Intermediate products (`u32` tokens × per-1M nano price) must use `i128` before the
  divide-by-1M/rounding step; no `f64` anywhere in the path.
- **D-03:** `ExecutionMetadata.cost_estimate` **stays `Option<f64>`**. The nano-unit `i64` is
  authoritative; `f64` is produced exactly once at the display edge (`nanos as f64 / 1e9`), never
  fed back into comparison/aggregation/enforcement. No public API break, no MIGRATION row.
- **D-04:** The price table carries one **`currency`** field: ISO 4217 3-letter code, default
  `"USD"`, validated (exactly three ASCII uppercase letters). Heralds stop printing a hard-coded
  `$` and render `0.0450 USD`. The JSON herald keeps the numeric field and adds currency beside it.

**Price table keying & gaps:**
- **D-05:** A price row is keyed by the **bare model name**, matched against `LlmResponse.model`.
  No provider plumbing. `provider/model` composite keys deferred.
- **D-06:** A row **requires `prompt` and `completion`**; `cache_read`, `cache_write`, `reasoning`
  are optional and default to billing at the parent axis's price when omitted. Cost per call:
  `(prompt − cache_read − cache_write) × prompt_price + cache_read × cache_read_price +
  cache_write × cache_write_price + (completion − reasoning) × completion_price + reasoning ×
  reasoning_price`, with `None` sub-counts treated as zero.
- **D-07:** The table lives in a **new top-level `treasurer:` config section**
  (`src/config/treasurer.rs`), wired into `Settings` exactly like `agent_runtime`. Omitting the
  section changes nothing. Validation rejects negative, non-decimal, empty, NaN-like strings and
  unknown currency shapes; zero is a valid price (free tier).
- **D-08:** When a call's model has no row, cost is `None` **and one `warn`-level log/trace line
  per unpriced model per process** (deduplicated). Never per-call, never silent.

**Cost producer path:**
- **D-09:** Cost is computed **per call, at the `LlmPort` boundary**, by a pricing decorator
  wrapping `Arc<dyn LlmPort>` in `paladin-llm` (sibling shape to `FallbackLlmAdapter`). Streaming
  responses are priced from the terminal chunk's usage only.
- **D-10:** **Cost rides beside usage everywhere usage travels.** A domain value type (working name
  `Cost { nanos: i64, currency: CurrencyCode }`, saturating `Add`/`AddAssign`/`Sum` like
  `TokenUsage`) is added as an additive `#[serde(default)] Option<Cost>` field next to `usage` on
  `LlmResponse`, `PaladinResult`, `TraceEvent::NodeFinished`, `TraceEvent::RunFinished`.
  `TraceDispatcher::total_usage` gains a `total_cost` twin. `None` propagates: any unpriced call in
  a run makes the run cost `None`.
- **D-11:** Surfaces that must show cost **in this phase**: `TraceEvent::RunFinished` carries the
  run total; heralds (markdown, JSON, table) print `ExecutionMetadata.cost_estimate` with currency.
  `PaladinResult.cost` is populated but HTTP/Run-API/CLI surfaces are Phase 39 LEDGR-04.
- **D-12:** `ExecutionMetadata` gets a **real production producer on both run paths**: engine builds
  it from `RunFinished`; agent loop builds it when a streamed execution finishes; each hands it to
  `Herald::finalize_stream`. The rustdoc on `cost_estimate`, `total_cost()` and the struct-level
  field list changes to "produced by the Treasurer".

**ADR posture:**
- **D-13 (ADR-0052, attachment point):** Metering lives in the `LlmPort` pricing decorator on both
  paths (D-09); the halt is raised where a run can checkpoint — the engine's superstep loop for
  `WarEngine` runs, and the existing `TokenBudget` cutoff for the agent loop. `build_chain` stays
  unwired for the engine path. The ADR must state the rejected alternative (wiring `build_chain`
  into the engine) and why.
- **D-14 (ADR-0053, ledger model):** Append-only ledger, balance derived on read. Every
  reserve/settle/release is an immutable row; window balance is `SUM` over rows in the period
  computed inside the same transaction as the reserve. The rejected alternative (running
  `allowance_windows` balance table) is recorded with its drift/rollover costs.
- **D-15 (ADR-0053 depth):** Fixes balance model, row kinds (reserve/settle/release), amounts in
  `i64` nano-units + currency code, and the settlement idempotency key
  `(run_id, superstep, attempt)`. Column names/indexes/DDL stay Phase 39's.
- **D-16 (timing):** ADRs first, as the phase's opening plan (38-01): write ADR-0052/0053, update
  `PROMOTION.md`'s index, then pricing plans (which may run in parallel with each other, not
  before 38-01 commits).

### Claude's Discretion

- Type homes: `Cost`, `CurrencyCode`, `PriceRow`/`PriceTable` and the pure cost function belong in
  `paladin-core`; `rust_decimal` (or a hand-rolled exact parser) is used only in the facade config
  parser (`src/config/treasurer.rs`).
- Where the pricing decorator is installed so both run paths get it; whether an empty table skips
  installation.
- What `ExecutionMetadata.model_used` reports for a multi-model engine run.
- Env-override shape for the map-shaped price table.
- Dedup mechanism for the one-warning-per-unpriced-model rule.
- Test fixture shape for per-token-type unit tests and the end-to-end herald test.

### Deferred Ideas (OUT OF SCOPE)

- `provider/model` composite price keys — add only on a real cross-provider collision.
- Cost on `PaladinResult` over HTTP, on the `Run` row, `GET /runs/{id}` and the CLI → Phase 39
  LEDGR-04.
- Changing `cost_estimate` to a fixed-point money type — rejected for v0.11.0 (X-03 break for no
  operator-visible gain); revisit only at a 1.0 API reshape.
- Multi-currency/FX (FUT-12), long-context tier pricing (FUT-11) — v2.
- RustFS-for-MinIO evaluation → Phase 45 (unrelated to this phase).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PRICE-01 | Operator can configure a per-model price table (prompt, completion, cache-read, cache-write, reasoning as decimal strings); default table empty; negative/malformed prices rejected at validation | `AgentRuntimeConfig`/`TokenBudgetConfig` gives the exact `Default`+`validate()`+`EnvOverridable` idiom to mirror (see Architecture Patterns, Code Examples); `rust_decimal` 1.43.0 verified OK for parsing decimal strings without float error (see Standard Stack, Package Legitimacy Audit) |
| PRICE-02 | A pure function maps `TokenUsage` × price table to an exact fixed-point cost, no `f64` accumulation, unit-tested per token type incl. cache/reasoning | `TokenUsage`'s five-axis shape and saturating-accumulation pattern (verified in `token_usage.rs`) is the direct model for `Cost`; Pitfall 1 (f64 drift) and Pitfall 8 (cache-inclusive vs. additive double-counting) from prior research are both directly load-bearing here and are restated below with source lines |
| PRICE-03 | Completed run reports currency cost in `ExecutionMetadata.cost_estimate` end-to-end; unpriced model yields `None` never `0`; rustdoc note becomes "produced by the Treasurer" | Verified: **no production code path constructs `ExecutionMetadata` or calls `Herald::finalize_stream` today** — confirmed by a full-tree grep; the two production `Arc<dyn LlmPort>` construction sites (`agent_host.rs::build_agent`/`build_agent_with_llm`, `facade_provisioner.rs::paladin_port_from_settings`) both resolve through `LlmProviderFactory::create()`, giving one confirmed choke point for wrapping the pricing decorator on both run paths (see Architecture Patterns) |
</phase_requirements>

## Summary

This phase is two ADRs plus a well-bounded, additive feature slice, and the codebase gives an
unusually clean model to copy for every piece of it. `AgentRuntimeConfig` in
`src/config/agent_runtime.rs` is a proven, twelve-sub-struct example of exactly the
`Default`/`validate()`/`EnvOverridable` config idiom `treasurer:` must follow, right down to the
"omit the section, nothing changes" contract and the error-message phrasing
(`"agent_runtime.token_budget.max_tokens must be greater than 0 when enabled"`). `TokenUsage` in
`crates/paladin-core/src/platform/container/token_usage.rs` is a proven model for `Cost`'s shape:
saturating `Add`/`AddAssign`/`Sum`, `Option<u32>` sub-counts that merge as `None+None=None`,
`None+Some=Some`, `Some+Some=saturating_add`. `FallbackLlmAdapter` in `crates/paladin-llm/src/
fallback.rs` is a proven model for the pricing decorator: a plain `Arc<dyn LlmPort>`-wrapping struct
with its own construction-time error type, composed transparently below the superstep engine and
the agent loop alike.

The single most valuable finding from direct code verification is the pricing decorator's
attachment point. Two production functions build the run's `Arc<dyn LlmPort>` before handing it to
`PaladinExecutionService::new(llm, breaker, ..)`: `agent_host.rs::build_agent_with_llm` (agent-loop
HTTP path) and `facade_provisioner.rs::paladin_port_from_settings` (engine path, via
`EngineExecutionPort` wrapping the same `PaladinExecutionService`). **Both resolve the LLM through
the identical `LlmProviderFactory::create(&provider)` call.** Wrapping that call's return value with
the pricing decorator is confirmed, by direct code reading, to reach both run paths with one change
— this is exactly what D-09 requires and directly answers the "decorator wiring" discretion item.

A second finding refines the ADR-0052 framing that CONTEXT.md's D-13 describes. A full-tree grep for
`build_chain(` and for `settings.agent_runtime` / `.agent_runtime` shows **zero production callers**
of `AgentRuntimeConfig::build_chain` (only its own unit tests and one `examples/` file use it), and
`Settings.agent_runtime` is read nowhere outside `src/config/agent_runtime.rs`/`settings.rs`
themselves. This means `TokenBudget`'s `StopReason::TokenBudget` cutoff — the mechanism D-13 leans
on for the agent-loop half of the mid-run halt — is fully built and unit-tested but **wired into no
production run today, on either path**. ADR-0052's Context section should state this precisely:
Phase 42 will be the first phase to wire `TokenBudget` enforcement into a real agent-loop run, not
merely extend something already live. This does not change the locked decision (D-13's leaning
still stands), but it changes what the ADR should claim about the "existing" cutoff, and it is
information Phase 42's planner will need.

A third finding is a genuine implementation gap the planner should account for in Wave sizing:
`crates/paladin-herald/src/table_herald.rs::finalize_stream` is a stub — it ignores its `_metadata`
parameter entirely and always renders hard-coded placeholder rows (`"3.45s"`, `"950"`, `"2"`,
`"100%"`). D-11 requires the table herald to print real cost; that requires first making
`finalize_stream` consume its `metadata` argument at all, which is a materially larger change than
the one-line addition needed in `markdown_herald.rs` (which already reads `metadata.cost_estimate`,
just formats it with a hard-coded `$`) or `json_herald.rs` (which already emits `cost_estimate` as a
JSON field, just needs a `currency` sibling).

**Primary recommendation:** Follow `AgentRuntimeConfig`'s exact config idiom for `treasurer:`,
follow `TokenUsage`'s exact accumulation idiom for `Cost`, install the pricing decorator by wrapping
`LlmProviderFactory::create()`'s return value (reaching both `build_agent_with_llm` and
`paladin_port_from_settings` with one change), and budget real work — not a one-liner — for making
`table_herald.rs::finalize_stream` consume real metadata at all.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Price table config (`treasurer:` section, parsing, validation) | Facade config (`src/config/treasurer.rs`) | — | Decimal-string parsing (`rust_decimal`) is a facade-only concern per D-00h/D-07; `paladin-core` stays dependency-pure |
| `Cost`, `CurrencyCode`, `PriceTable` domain types + pure cost function | Domain / `paladin-core` | — | Pure, no I/O, must be usable from `paladin-llm` and `paladin-battalion` without depending on the facade (mirrors `TokenUsage`'s existing placement) |
| Per-call pricing (LlmResponse → Cost) | Output port adapter (`paladin-llm`, pricing decorator) | — | Mirrors `FallbackLlmAdapter`: composes below both run paths at the `LlmPort` boundary, so it is provider- and path-agnostic (D-09) |
| Cost aggregation (run total) | Application / engine (`paladin-battalion::engine::hooks::TraceDispatcher`) | Application / agent loop (`PaladinExecutionService`) | `TraceDispatcher::total_usage()` already aggregates `NodeFinished.usage` synchronously inside `emit()`; `total_cost()` is its direct twin. The agent loop separately owns `PaladinResult.cost` for its own single-call/loop context |
| `ExecutionMetadata` production | Application (engine's `RunFinished` handler + agent loop's stream-completion handler) | — | Both are the facade-adjacent orchestration layer that already owns `Herald::finalize_stream` invocation sites (currently absent — see Common Pitfalls) |
| Cost display (heralds) | Presentation / `paladin-herald` | — | Markdown/JSON/table heralds already read `ExecutionMetadata`; this phase only changes formatting (currency string) and, for table, wires up what is currently a stub |
| Mid-run halt enforcement (ADR-0052 subject, NOT built this phase) | Application (engine superstep loop / agent-loop `TokenBudget` middleware) | — | Attachment point recorded by ADR-0052; implementation is Phase 42 |
| Ledger balance model (ADR-0053 subject, NOT built this phase) | Output port (`paladin-storage`, future `TreasuryLedgerPort`) | — | Schema/idempotency-key decisions recorded by ADR-0053; implementation is Phase 39 |

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `rust_decimal` | 1.43.0 [VERIFIED: crates.io + `gsd-tools package-legitimacy check`] | Parse operator-entered decimal price strings without binary-float rounding error, at the config boundary only | 2,761,074 weekly downloads, published since 2016, MIT-licensed (allowed by `deny.toml`'s permissive allow-list), no `postinstall` script, active repo (`paupino/rust-decimal`) — the de facto standard exact-decimal crate in the Rust ecosystem; not yet in this workspace's `Cargo.lock` (confirmed: `grep -n rust_decimal Cargo.toml Cargo.lock` returns nothing) |
| `i64` fixed-point (nano-units) | std | The persisted/authoritative cost unit everywhere except the facade's decimal-string parse step | Matches `TokenUsage`'s own `u32` "plain units, no floating point" convention; native to `sqlx` on every backend Phase 39 will need (SQLite has no native decimal type) |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| (none new) | — | — | This phase adds exactly one new dependency (`rust_decimal`), and only to the facade. `Cost`/`CurrencyCode`/`PriceTable`/the cost function need nothing beyond `serde` (already a `paladin-core` dependency) |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `rust_decimal` for parsing | Hand-rolled exact decimal-string parser (split on `.`, parse both halves as `u64`, reject anything else) | Explicitly permitted by CONTEXT.md's discretion note ("the planner may substitute a hand-rolled exact decimal-string parser if it keeps the dependency count flat"). A hand-rolled parser only needs to handle a bounded grammar (optional sign is NOT needed — negative prices are rejected — digits, one optional `.`, more digits) and avoids adding a dependency at all; `rust_decimal` is simpler to get exactly right (rounding modes, `Decimal::from_str_exact`) and is already vetted in prior research (`SUMMARY.md`) |
| Nano-unit `i64` | Micro-unit `i64` (PRICE-02's literal wording) | Rejected by locked D-02: at $0.15/M, a single token is 0.15 micro-units and rounds to zero per call — reasonable, since sub-cent-per-million prices are common in this market (DeepSeek, Kimi) |

**Installation:**
```bash
# Only the facade crate (root `paladin-ai` / `src/`) gains a new dependency.
cargo add rust_decimal@1.43 --no-default-features
```

**Version verification:** `rust_decimal` 1.43.0 confirmed via `gsd-tools query package-legitimacy
check --ecosystem crates rust_decimal` → verdict `OK`, `weeklyDownloads: 2761074`,
`repoUrl: https://github.com/paupino/rust-decimal`, `deprecated: false`, `postinstall: null`. Cross-
checked against `deny.toml`'s permissive license allow-list (`MIT`, `Apache-2.0` both present).
Confirmed **not yet present** in this workspace's `Cargo.toml`/`Cargo.lock` (a `grep` for
`rust_decimal`/`rust-decimal` in both files returns nothing), matching CONTEXT.md's "not yet in
`Cargo.lock`" claim.

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| `rust_decimal` | crates.io | ~10 years (published 2016-11-16) | 2,761,074/week | `github.com/paupino/rust-decimal` | OK | Approved — add to facade `Cargo.toml` only |

**Packages removed due to SLOP verdict:** none.
**Packages flagged as suspicious [SUS]:** none.

No other new external packages are introduced by this phase. `Cost`, `CurrencyCode`, `PriceTable`
and the pure cost function are hand-written domain types requiring no new dependency; `serde` is
already a `paladin-core` dependency (`crates/paladin-core/Cargo.toml`).

## Architecture Patterns

### System Architecture Diagram

```
Operator config.yml (treasurer.pricing.<model> = {prompt, completion, cache_read?, ...})
        │
        ▼
Settings::load() → TreasurerConfig::validate()  (src/config/treasurer.rs, boot-time)
        │  (rejects negative/malformed/NaN-like decimal strings; rejects bad currency codes)
        ▼
PriceTable (paladin-core, pure)  ←── read-only, immutable per process
        │
        │  wrapped around ↓ at construction time, in BOTH production LlmPort resolution sites:
        │    - agent_host.rs::build_agent / build_agent_with_llm      (agent-loop HTTP path)
        │    - facade_provisioner.rs::paladin_port_from_settings      (engine path)
        │  both currently call LlmProviderFactory::create(&provider) → Arc<dyn LlmPort>
        ▼
PricingLlmAdapter (new, paladin-llm, sibling to FallbackLlmAdapter)
        │  wraps Arc<dyn LlmPort>; on each generate()/generate_stream() terminal chunk:
        │    - looks up LlmResponse.model in PriceTable
        │    - computes Cost via the pure cost function (TokenUsage × PriceRow → Cost)
        │    - Some(row) found  → Some(Cost)     [D-06 fallback rules applied]
        │    - no row found     → None + one warn-per-model-per-process (D-08)
        │    - stamps Cost onto LlmResponse (new additive Option<Cost> field, D-10)
        ▼
   ┌────────────────────────────┴────────────────────────────┐
   ▼ (engine path)                                             ▼ (agent-loop path)
EngineExecutionPort → PaladinExecutionService::execute()      PaladinExecutionService::execute()
   │  cost copied onto TraceEvent::NodeFinished.cost            │  cost copied onto PaladinResult.cost
   ▼                                                             ▼
TraceDispatcher::emit() (synchronous, inside the dispatcher)   Agent-loop stream-completion handler
   │  total_cost() twin of total_usage(), same None-propagation │  (NEW producer, D-12 — none exists
   │  rule as TokenUsage's saturating sum                       │  in production today)
   ▼                                                             │
TraceEvent::RunFinished { usage, cost, .. }                     │
   │                                                             │
   └──────────────────────┬──────────────────────────────────────┘
                           ▼
         ExecutionMetadata producer (NEW on both paths, D-12)
                           │  builds via ExecutionMetadata::builder()
                           │  .cost_estimate(cost.nanos as f64 / 1e9)  [D-03: f64 only at this edge]
                           ▼
              Herald::finalize_stream(&metadata)
         ┌─────────────────┼─────────────────┐
         ▼                 ▼                 ▼
   markdown_herald    json_herald       table_herald
   (already reads      (already emits    (STUB TODAY — ignores
   cost_estimate;       cost_estimate;    _metadata entirely;
   fix hard-coded $)    add currency)     must be wired to
                                          consume real metadata
                                          first — see Pitfalls)
```

### Recommended Project Structure

```
crates/paladin-core/src/platform/container/
├── cost.rs                # NEW: Cost, CurrencyCode, PriceRow/PriceTable, pure cost fn (D-06's rules)
├── token_usage.rs          # UNCHANGED — model for Cost's saturating-accumulation shape
├── herald.rs                # rustdoc note updated (D-12); cost_estimate/total_cost() unchanged types
├── trace.rs                  # TraceEvent::NodeFinished/RunFinished gain `cost: Option<Cost>`
└── execution_result.rs        # PaladinResult gains `cost: Option<Cost>` (#[serde(default)], D-25 precedent)

crates/paladin-ports/src/output/
└── llm_port.rs              # LlmResponse gains `cost: Option<Cost>` (#[serde(default)])

crates/paladin-llm/src/
├── fallback.rs               # UNCHANGED — the shape to mirror
└── pricing.rs                # NEW: PricingLlmAdapter, sibling to FallbackLlmAdapter

crates/paladin-battalion/src/engine/
└── hooks.rs                  # TraceDispatcher gains total_cost() twin of total_usage()

src/config/
├── agent_runtime.rs          # UNCHANGED — the Default/validate()/EnvOverridable model to mirror
├── treasurer.rs               # NEW: TreasurerConfig { currency, pricing: HashMap<String, PriceRowConfig> }
└── settings.rs                 # Settings gains `pub treasurer: TreasurerConfig`

crates/paladin-herald/src/
├── markdown_herald.rs         # finalize_stream: replace hard-coded "${:.4}" with currency-aware format
├── json_herald.rs              # finalize_stream: add "currency" field beside existing "cost_estimate"
└── table_herald.rs              # finalize_stream: STOP ignoring _metadata; wire real rows (larger diff)
```

### Pattern 1: Config sub-struct idiom (`Default` + `validate()` + `EnvOverridable`)
**What:** Every `AgentRuntimeConfig` sub-struct follows one shape: derive `Default`, a hand-written
`validate() -> Result<(), String>` that rejects invalid combinations but never clamps, and an
`EnvOverridable` impl reading `APP_AGENT_RUNTIME_<SECTION>_<FIELD>`.
**When to use:** `TreasurerConfig` and its nested `PriceRowConfig`/`pricing` map must follow this
exactly (D-00h, D-07), with the caveat that the map-shaped `pricing` field has **no** env-override
form — CONTEXT.md's own discretion note leaves scalar-only env overrides (`currency`) as the
default posture, matching `ToolCallLimitConfig.per_tool`'s existing precedent of a config-file-only
collection field.
**Example:**
```rust
// Source: src/config/agent_runtime.rs (verified in this tree, 2026-09-25)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenBudgetConfig {
    pub enabled: bool,
    pub max_tokens: u32,
}

impl TokenBudgetConfig {
    /// Rejects `max_tokens == 0` when `enabled`; never clamps.
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
The `treasurer:` section's `validate()` should follow the identical message-prefix convention:
`"treasurer.currency must be exactly three ASCII uppercase letters"`,
`"treasurer.pricing.<model>.prompt must be a non-negative decimal string"`, etc.

### Pattern 2: `LlmPort`-wrapping decorator with its own construction-time error type
**What:** `FallbackLlmAdapter` composes `Vec<Arc<dyn LlmPort>>` into one `LlmPort` impl, with a
dedicated `FallbackChainError` for construction-time misconfiguration, `Clone`+`Debug` (with a
`Debug` impl that avoids leaking sensitive internals), and no cross-call mutable state.
**When to use:** `PricingLlmAdapter` should follow the identical shape: `Arc<dyn LlmPort>` inner
field, `PriceTable` field (immutable, cloned at construction), a `HashSet`/`OnceLock`-backed
warn-once set for D-08 (mutable, but scoped to "have we warned about this exact model name" only —
not per-run state, matching the module docs' "per-run scratch state never lives on a shared
middleware/decorator struct" rule already established for `limits.rs`).
**Example:**
```rust
// Source: crates/paladin-llm/src/fallback.rs (verified in this tree, 2026-09-25)
#[derive(Clone)]
pub struct FallbackLlmAdapter {
    chain: Vec<Arc<dyn LlmPort>>,
    trace_emitter: Option<Arc<dyn TraceEmitter>>,
}

#[async_trait]
impl LlmPort for FallbackLlmAdapter {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        // ... tries each chain element, stamps SERVED_BY_METADATA_KEY on success
    }
    // generate_stream, validate_model, get_available_models, get_provider_name,
    // get_capabilities all delegate transparently to the wrapped chain.
}
```
`PricingLlmAdapter` differs in one respect: it must price the **terminal chunk's** usage for
`generate_stream` (D-09), mirroring how `StreamingResponse::usage` is `Some` only on the terminal
chunk (verified in `llm_port.rs`'s `StreamingResponse` docs: "`usage` is `Some` on exactly ONE chunk
per stream -- the terminal chunk").

### Pattern 3: Pricing decorator installation point — verified single choke point for both run paths
**What:** Both production `Arc<dyn LlmPort>` construction sites resolve through
`LlmProviderFactory::create(&provider_name)`.
**When to use:** Wrap the return value of that call (or wrap inside `LlmProviderFactory::create`
itself, gated on whether `treasurer.pricing` is non-empty — CONTEXT.md leaves this as discretion).
**Example (verified, both are real, non-test production code as of this session):**
```rust
// Source: src/infrastructure/web/facade_provisioner.rs (engine path)
pub fn paladin_port_from_settings(
    settings: &Settings,
) -> Result<Arc<dyn PaladinPort>, HostBuildError> {
    let factory = LlmProviderFactory::new();
    let provider = default_provider_name(settings);
    let llm = factory.create(&provider)  // <-- wrap HERE (or return value used HERE)
        .map_err(|source| HostBuildError::Provider { id: "run-engine".into(), provider, source })?;
    let service = Arc::new(PaladinExecutionService::new(llm, default_circuit_breaker(), None, None));
    // ...
}
```
```rust
// Source: src/infrastructure/web/agent_host.rs (agent-loop path)
pub(crate) async fn build_agent_with_llm(
    def: &AgentDefinition,
    llm: Arc<dyn LlmPort>,  // <-- caller already resolved this via factory.create(); wrap upstream
    breaker: Arc<CircuitBreaker>,
) -> Result<BuiltAgent, HostBuildError> {
    let service = Arc::new(PaladinExecutionService::new(Arc::clone(&llm), breaker, None, None));
    // ...
}
```
Both call sites are reachable from `Settings`, so `TreasurerConfig` can be threaded to whichever
function wraps `factory.create()`'s output — most naturally a small helper in `provider_factory.rs`
or a new `paladin-llm` free function `paladin_llm::pricing::maybe_price(llm, price_table)` that
returns the inner `llm` unchanged when `price_table.is_empty()` (the "empty table skips
installation" discretion option), avoiding a wrapping layer with zero effect.

### Anti-Patterns to Avoid
- **Reading `f64` back out of `Cost` for any comparison or accumulation:** D-02/D-03 fix `nanos: i64`
  as authoritative; `f64` is produced exactly once, at the display edge, and never round-trips back
  into the pipeline. This is Pitfall 1 from prior research, restated below.
- **Treating `cost_estimate: None` as `0` in any downstream code** (herald, future ledger, future
  allowance check): Pitfall 6 from prior research — an `unwrap_or(0.0)` anywhere on the enforcement
  side would make an unpriced/mis-keyed model silently free.
- **Assuming `TokenBudget`'s cutoff is "existing" in the sense of already running in production:**
  it is unit-tested and functionally complete, but `AgentRuntimeConfig::build_chain` has zero
  production callers today (verified: `grep -rn "build_chain(" --include="*.rs"` matches only
  `agent_runtime.rs`'s own tests and one `examples/` file). ADR-0052 should describe the mid-run
  halt attachment point as "the cutover point Phase 42 will wire for the first time," not as
  extending live behavior.
- **Writing `table_herald.rs`'s cost row without first checking whether `finalize_stream` reads its
  argument at all today.** It does not (see Common Pitfalls).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Exact decimal string → integer nano-unit parsing | A regex-based decimal parser with manual overflow checking | `rust_decimal::Decimal::from_str_exact` (or the CONTEXT-permitted hand-rolled bounded-grammar parser, since no sign/exponent/locale support is needed) | `rust_decimal` already handles the edge cases (leading/trailing zeros, scale limits) that a naive regex parser gets subtly wrong; if hand-rolling to avoid the dependency, keep the grammar deliberately narrow (digits, one `.`, more digits, no sign) rather than reimplementing general decimal parsing |
| Saturating accumulation of a domain value across an unbounded run | A `checked_add` + manual overflow-to-error path | `TokenUsage`'s existing saturating-`Add`/`AddAssign`/`Sum` pattern, copied for `Cost` | Already proven, already tested (`token_usage.rs`'s `add_saturates_at_u32_max_without_panicking` and `option_merge_*` tests), and matches the sibling type's semantics exactly so a `TokenUsage`+`Cost` pair never disagrees on overflow behavior |
| A new `LlmPort`-wrapping decorator pattern from scratch | Inventing a bespoke pricing-adapter shape | `FallbackLlmAdapter`'s exact structure (immutable chain/config field, `Clone`, dedicated construction-time error enum, transparent delegation of identity methods) | The pattern is proven in this codebase, has 20+ passing unit/integration tests to imitate, and composes correctly with `FallbackLlmAdapter` itself (a fallback hop's `LlmResponse` still needs pricing — wrap OUTSIDE the fallback chain, i.e. `Pricing(Fallback(provider1, provider2))`, not the reverse, so a hop's real served model is what gets priced) |

**Key insight:** Every piece of this phase already has a proven sibling in the shipped tree
(`AgentRuntimeConfig` for config, `TokenUsage` for the value type, `FallbackLlmAdapter` for the
decorator). The only genuinely new engineering is (a) the fixed-point cost math itself (D-02's
`i128`-intermediate, half-up-rounding rule) and (b) making `table_herald.rs::finalize_stream`
consume real data for the first time.

## Common Pitfalls

### Pitfall 1: Currency math done in `f64` (carried from prior research, directly applicable here)
**What goes wrong:** `ExecutionMetadata.cost_estimate: Option<f64>` already ships as a public field
with a builder method `cost_estimate(mut self, cost_estimate: f64)`. If any internal accumulation
(`prompt_tokens * price + completion_tokens * price + ...`) is done in `f64`, results are subject to
binary floating-point rounding error that compounds across a run's multiple calls.
**Why it happens:** `f64` is the path of least resistance because the public field is already `f64`
and Rust's numeric literals default to `f64` in ordinary arithmetic expressions.
**How to avoid:** Do every multiplication/sum in `i64`/`i128`; convert to `f64` exactly once, at the
`ExecutionMetadata::builder().cost_estimate(...)` call site (D-03).
**Warning signs:** Any `as f64` cast appearing before a `+`, `*`, or comparison in the cost pipeline,
rather than as the very last step before a builder call.
**Source:** `.planning/research/PITFALLS.md` Pitfall 1, cross-referenced against
`crates/paladin-core/src/platform/container/herald.rs:505` (verified: field is exactly
`pub cost_estimate: Option<f64>` as described).

### Pitfall 2: Cache-token pricing gets the discount direction or inclusion backwards
**What goes wrong:** `TokenUsage.cache_read_tokens`/`cache_write_tokens` are sub-counts OF
`prompt_tokens` (verified: `token_usage.rs`'s own doc comment: "Of `prompt_tokens`, how many were
served from a provider-side cache read... always `<= prompt_tokens` when `Some`"). A pricing
function that does `prompt_tokens * prompt_price + cache_read_tokens * cache_read_price` (additive,
without subtracting the cache sub-counts from the prompt-priced portion first) double-counts those
tokens — charging both the full prompt price AND the cache price for the same tokens.
**Why it happens:** The field names sound additive ("cache_read tokens ALSO consumed") but the
documented semantics are inclusive ("cache_read tokens are PART OF prompt_tokens").
**How to avoid:** D-06's formula is exactly right and must be implemented literally:
`(prompt − cache_read − cache_write) × prompt_price + cache_read × cache_read_price + cache_write ×
cache_write_price + (completion − reasoning) × completion_price + reasoning × reasoning_price`.
Every subtraction must use `saturating_sub` (a provider that reports `cache_read_tokens >
prompt_tokens`, though it should never happen per the documented invariant, must not panic or
underflow).
**Warning signs:** A cost-function unit test where a call with `cache_read_tokens = prompt_tokens`
(100% cache hit) produces a cost equal to the 0%-cache-hit cost — the discount value is being
computed but not actually subtracted from the base charge.
**Source:** `.planning/research/PITFALLS.md` Pitfall 8, cross-referenced against
`token_usage.rs`'s own doc comments (verified in this tree).

### Pitfall 3: Unpriced model silently treated as free (`cost_estimate.unwrap_or(0.0)`)
**What goes wrong:** Any downstream consumer — a herald, a future ledger accumulator, a future
allowance check — that does `spend += cost_estimate.unwrap_or(0.0)` treats an unpriced or
mis-keyed model's usage as free.
**Why it happens:** `unwrap_or(0.0)` type-checks immediately and is the path of least resistance
compared to explicitly branching on `None`.
**How to avoid:** D-08's rule (one warn-per-model-per-process, cost stays `None`) must be enforced
at the decorator itself, and every consumer of `Cost`/`cost_estimate` downstream must propagate
`None` rather than substitute a default. `None` propagation for run totals means: if ANY priced call
in a run was unpriced, the run's total is `None`, not "zero for the unpriced call, summed with the
rest" — this must be a distinct code path, not an accidental default.
**Warning signs:** Any `.unwrap_or(0)`/`.unwrap_or(0.0)` on a `Cost`/`cost_estimate` value outside of
purely cosmetic display formatting (where "cost unknown" and "0.00" should still render visibly
differently, e.g. `"—"` vs `"0.0000 USD"`).
**Source:** `.planning/research/PITFALLS.md` Pitfall 6.

### Pitfall 4: `table_herald.rs::finalize_stream` is a stub that ignores its argument
**What goes wrong:** Unlike `markdown_herald.rs` (reads `metadata.cost_estimate` today) and
`json_herald.rs` (emits `metadata.cost_estimate` today), `table_herald.rs`'s `finalize_stream`
signature names its parameter `_metadata` and never reads it — it always renders four hard-coded
placeholder rows (`"Total Duration", "3.45s"`, `"Total Tokens", "950"`, `"Paladins Executed", "2"`,
`"Success Rate", "100%"`). A plan that treats "add a Cost row to all three heralds" as one uniform,
equally-sized task across all three formats will under-scope the table herald's share of the work.
**Why it happens:** The table herald was seeded early with placeholder content and never revisited
once real production callers of `finalize_stream` failed to materialize (see Pitfall 5 below — the
whole method has had zero non-test/non-example callers, so the stub was never exercised against real
data).
**How to avoid:** Treat "wire `table_herald::finalize_stream` to consume real `ExecutionMetadata`"
as its own task, separate from "add the cost row," inside whichever plan handles heralds.
**Warning signs:** A table-herald unit test that asserts against the literal strings `"3.45s"` or
`"950"` still passing after the phase's changes — that would mean the stub was never actually fixed.
**Source:** Direct read of `crates/paladin-herald/src/table_herald.rs:266-290`, verified in this
session, 2026-09-25.

### Pitfall 5: Assuming `ExecutionMetadata`/`finalize_stream` already fire in production
**What goes wrong:** A plan might assume "the producer already exists somewhere, I just need to
add a field to it." A full-tree grep confirms `ExecutionMetadata::builder()` and `Herald::
finalize_stream(` are called ONLY from: each herald's own `#[cfg(test)]` module,
`crates/doc-examples/src/herald_output.rs`, and `examples/herald_streaming.rs` /
`examples/herald_custom_formatter.rs`. **No file under `src/` outside `crates/*_herald.rs` test
modules constructs the type or calls the method.**
**Why it happens:** The type and trait method were built ahead of their producer (a common
"ship the port before the adapter" sequencing), and nothing since has closed the gap.
**How to avoid:** D-12 correctly identifies this as needing a real producer on BOTH run paths; the
planner should size this as new integration work (engine: hook into the `RunFinished` trace-event
handler; agent loop: hook into wherever a streamed execution currently completes without building
metadata), not as "wire a field into an existing call."
**Warning signs:** A plan step that says "update the ExecutionMetadata producer" without first
locating one — there isn't one to update; one must be created.
**Source:** Direct `grep -rn "finalize_stream\|ExecutionMetadata::builder"` across the full tree
(excluding herald test modules), verified in this session, 2026-09-25 — see also D-12's own text,
which independently states "Today nothing outside tests constructs the type."

### Pitfall 6: `AgentRuntimeConfig::build_chain` / `TokenBudget` assumed already live in production
**What goes wrong:** ADR-0052 (this phase) leans on "the existing `TokenBudget` cutoff... for the
agent loop" as the mid-run enforcement mechanism for that path. A reader could assume this means
`TokenBudget` already fires on real agent-loop runs today.
**Why it happens:** `TokenBudget` is fully implemented, thoroughly unit-tested (12+ tests in
`limits.rs`), and `AgentRuntimeConfig::build_chain()` correctly assembles it into a chain — but
**nothing in production ever calls `build_chain()` or attaches its result to a real
`PaladinExecutionService`.** Verified: `grep -rn "build_chain(" --include="*.rs" .` matches only
`agent_runtime.rs`'s own test module and one line in `examples/agent_runtime_middleware.rs`.
Verified further: `grep -rn "settings.agent_runtime\|\.agent_runtime\b"` (excluding
`agent_runtime.rs`/`settings.rs` themselves) returns **zero matches** — nothing reads the config
section in production at all.
**How to avoid:** ADR-0052's Context section should state plainly that the agent-loop `TokenBudget`
cutoff, while code-complete, is currently wired into zero production runs, and that Phase 42 will be
the phase that performs that wiring for the first time (alongside whatever new engine-side hook
ADR-0052 specifies). This is a Context-section correction, not a change to the locked D-13 decision.
**Warning signs:** Any Phase 42 plan that describes wiring `TokenBudget` into the agent loop as
"already working, just needs a Treasurer-derived budget value" rather than "wiring the first
production caller of this middleware chain."
**Source:** Direct `grep -rn "build_chain("` and `grep -rn "\.agent_runtime\b"` across the full
tree, verified in this session, 2026-09-25.

## Code Examples

### `Cost`'s expected shape, modeled directly on `TokenUsage`
```rust
// Illustrative — not yet in tree. Modeled on the verified TokenUsage shape
// (crates/paladin-core/src/platform/container/token_usage.rs).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Cost {
    /// 1e-9 of `currency`; saturating like TokenUsage's u32 fields.
    pub nanos: i64,
    pub currency: CurrencyCode,
}

impl std::ops::Add for Cost {
    type Output = Cost;
    fn add(self, rhs: Self) -> Self::Output {
        // Currency mismatch handling is a planner decision not fixed by
        // CONTEXT.md (D-00b keeps v0.11.0 single-currency, so within one
        // process this should never actually differ — but the type should
        // decide explicitly whether to debug_assert or silently keep self's
        // currency).
        Cost { nanos: self.nanos.saturating_add(rhs.nanos), currency: self.currency }
    }
}
```

### The `LlmResponse` fallback-then-price composition order
```rust
// Pricing must wrap OUTSIDE any FallbackLlmAdapter so it sees the response
// that was actually served (and its real `model` string), matching D-09's
// "survives FallbackLlmAdapter provider hops and mixed-model graphs" requirement.
let fallback = Arc::new(FallbackLlmAdapter::new(vec![openai, anthropic])?);
let priced: Arc<dyn LlmPort> = if price_table.is_empty() {
    fallback  // D-09/discretion: skip installing the decorator on an empty table
} else {
    Arc::new(PricingLlmAdapter::new(fallback, price_table))
};
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| `ExecutionMetadata.cost_estimate` field exists with rustdoc "Reserved for the Treasurer... no in-tree producer yet" | This phase adds the first real producer on both run paths | This phase (v0.11.0, Phase 38) | Rustdoc note changes to "produced by the Treasurer" (D-12); the field's illustrative `.cost_estimate(0.045)` doc example becomes a real, executable one |
| `TokenUsage` alone travels port→engine→herald | `Cost` rides beside `TokenUsage` at every one of the same four hop points (`LlmResponse`, `PaladinResult`, `TraceEvent::NodeFinished`, `TraceEvent::RunFinished`) | This phase | Mirrors the exact pattern Phase 31 (ACCT-01..05) established for lossless token accounting — this phase is structurally "ACCT for money" |

**Deprecated/outdated:** none — this is new capability, not a replacement of an existing mechanism.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `rust_decimal` 1.43.0 is the current published version | Standard Stack | Low — verified via `gsd-tools package-legitimacy check` (registry-confirmed) and cross-checked against a WebSearch result; if a newer patch exists, `cargo add rust_decimal` will simply pull it and no behavior in this phase depends on the exact patch version |
| A2 | No env-override form for the map-shaped `treasurer.pricing` field is the right default (only `treasurer.currency` gets `APP_TREASURER_CURRENCY`) | User Constraints (Claude's Discretion) | Low — explicitly left to planner discretion by CONTEXT.md; consistent with the existing `ToolCallLimitConfig.per_tool` precedent (config-file-only) |

**If this table is empty:** N/A — two low-risk assumptions recorded above, both explicitly flagged
as within-discretion or independently registry-verified.

## Open Questions (RESOLVED)

*Both questions below were resolved during planning (plan-checker pass, 2026-09-25); the inline
`RESOLVED` line under each names the plan that implements the answer.*

1. **Where exactly should the pricing decorator be constructed — inside `LlmProviderFactory::create`
   itself, or as a wrapping call at each of the two call sites?**
   - **RESOLVED:** wrap at the two facade call sites (`paladin_port_from_settings` and
     `build_agent`/`build_agent_with_llm`), not inside `LlmProviderFactory::create` — implemented by
     plan 38-03 Task 2 (its must-haves read "wrap at both facade call sites"); `provider_factory.rs`
     stays provider-selection-only.
   - What we know: both `paladin_port_from_settings` (engine) and `build_agent`/`build_agent_with_llm`
     (agent loop) call `LlmProviderFactory::create(&provider)` and both need the same wrapping.
   - What's unclear: `LlmProviderFactory::create` doesn't currently take a `PriceTable` parameter,
     and threading `Settings.treasurer` into it changes its signature (a `paladin-llm` crate function
     taking a `paladin-core` type is fine architecturally, but is a larger diff than wrapping at each
     of the two facade call sites, which already have `&Settings` in scope).
   - Recommendation: wrap at the two facade call sites (`paladin_port_from_settings`,
     `build_agent`/`build_agent_with_llm`), not inside `LlmProviderFactory::create` itself — smaller
     diff, keeps `provider_factory.rs` provider-selection-only, and both call sites already have
     `&Settings` in scope to read `treasurer.pricing`.

2. **Does a fallback hop's `LlmResponse.model` differ from the originally-requested model, and does
   that matter for pricing?**
   - **RESOLVED:** yes, and the decorator prices the *served* response's `model`, composed outside
     `FallbackLlmAdapter` — implemented and proven by plan 38-04 Task 2's
     `prices_the_served_model_after_fallback_hop` test (threat T-38-14).
   - What we know: `FallbackLlmAdapter::generate` returns whichever provider's `LlmResponse` actually
     served the call, with that provider's own `model` field (each provider adapter sets `model` from
     its own response, not the request) and `served_by` metadata recording which provider served it.
   - What's unclear: whether a fallback hop from `gpt-4` (OpenAI, unavailable) to a differently-named
     model on a backup provider produces an `LlmResponse.model` that still matches a price-table key
     at all, or falls through to `None` — this is expected/correct behavior (a different model has a
     different price), but the planner should write a unit test exercising this exact scenario since
     D-09 explicitly calls out "survives `FallbackLlmAdapter` provider hops" as a requirement.
   - Recommendation: add a test fixture where the fallback chain's second provider reports a
     DIFFERENT model name than the first, and assert the returned `Cost` reflects the SECOND model's
     price row (or `None` if that model has no row) — proving the pricing decorator reads the actual
     served response, not the original request.

## Environment Availability

No external tools, services, or runtimes beyond the existing Rust/Cargo toolchain are needed for
this phase — it is entirely in-tree config/domain/adapter code plus documentation ADRs. Skipping
this section's table per the "no external dependencies" exemption; the one new dependency
(`rust_decimal`) is a compile-time Cargo dependency, not a runtime service, and is covered under
Standard Stack / Package Legitimacy Audit above.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (native Rust unit + `#[cfg(test)]` modules + doctests); no external test framework config file |
| Config file | none — `Cargo.toml`'s workspace members define test targets; coverage config is `.github/workflows/*.yml`'s `cargo llvm-cov` invocation |
| Quick run command | `cargo test -p paladin-core cost::` (once `cost.rs` exists) / `cargo test -p paladin-llm pricing::` / `cargo test -p paladin-web-facade treasurer` (adjust crate/module names to actual file locations chosen) |
| Full suite command | `cargo test` (workspace-wide); `make test-all` for unit + integration |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PRICE-01 | `treasurer:` section omitted → inert (empty table, USD default) | unit | `cargo test -p paladin treasurer::tests::default_treasurer_config_is_inert -x` | ❌ Wave 0 — new file `src/config/treasurer.rs` |
| PRICE-01 | Negative/malformed/NaN-like decimal string rejected at `validate()` | unit | `cargo test -p paladin treasurer::tests::validate_rejects_negative_price -x` | ❌ Wave 0 |
| PRICE-01 | Invalid currency code (not 3 uppercase ASCII letters) rejected | unit | `cargo test -p paladin treasurer::tests::validate_rejects_bad_currency -x` | ❌ Wave 0 |
| PRICE-02 | Cost function: prompt+completion only (no cache/reasoning) | unit | `cargo test -p paladin-core cost::tests::prompt_completion_only -x` | ❌ Wave 0 — new file `cost.rs` |
| PRICE-02 | Cost function: cache_read/cache_write present, subtracted from prompt-priced base per D-06 | unit | `cargo test -p paladin-core cost::tests::cache_axes_subtract_from_base -x` | ❌ Wave 0 |
| PRICE-02 | Cost function: reasoning present, subtracted from completion-priced base | unit | `cargo test -p paladin-core cost::tests::reasoning_axis_subtracts_from_base -x` | ❌ Wave 0 |
| PRICE-02 | Rounding: half-up to nearest nano-unit, single low-price token doesn't round to zero | unit | `cargo test -p paladin-core cost::tests::sub_micro_price_does_not_round_to_zero -x` | ❌ Wave 0 |
| PRICE-02 | Run-total saturating sum matches `TokenUsage`'s own saturation semantics | unit | `cargo test -p paladin-core cost::tests::saturating_sum_matches_token_usage_pattern -x` | ❌ Wave 0 |
| PRICE-03 | Unpriced model → `Cost: None`, never `0`, one warn line per model per process | unit | `cargo test -p paladin-llm pricing::tests::unpriced_model_warns_once -x` | ❌ Wave 0 — new file `pricing.rs` |
| PRICE-03 | Fallback-hop-served model is what gets priced, not the original request's model | integration | `cargo test -p paladin-llm pricing::tests::prices_the_served_model_after_fallback_hop -x` | ❌ Wave 0 |
| PRICE-03 | `ExecutionMetadata` real producer, engine path, rustdoc note updated | doctest + unit | `cargo test --doc -p paladin-core herald` / `cargo test -p paladin-battalion engine::tests::run_finished_produces_execution_metadata -x` | ❌ Wave 0 |
| PRICE-03 | `ExecutionMetadata` real producer, agent-loop path | integration | `cargo test -p paladin agent_host::tests::streamed_agent_run_produces_execution_metadata -x` | ❌ Wave 0 |
| PRICE-03 | Heralds render currency-aware cost (`0.0450 USD`, not `$0.0450`) | unit | `cargo test -p paladin-herald markdown_herald::tests::finalize_stream_renders_currency -x` | ❌ Wave 0 |
| PRICE-03 | `table_herald` `finalize_stream` consumes real metadata (stub fixed) | unit | `cargo test -p paladin-herald table_herald::tests::finalize_stream_uses_real_metadata -x` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** the relevant crate's quick command above (e.g. `cargo test -p paladin-core cost::`)
- **Per wave merge:** `cargo test` (full workspace)
- **Phase gate:** `make clean-code` (fmt + clippy + check) and full `cargo test` green, plus
  `make api-surface` (this phase is additive per D-00g, so no allowlist entry is expected — a
  diff here would itself be a signal something unintentionally broke API surface) before
  `/gsd-verify-work`.

### Wave 0 Gaps
- [ ] `crates/paladin-core/src/platform/container/cost.rs` — new file, `Cost`/`CurrencyCode`/
      `PriceRow`/`PriceTable`/pure cost function + unit tests (PRICE-02)
- [ ] `src/config/treasurer.rs` — new file, `TreasurerConfig`/`PriceRowConfig` + `Default`/
      `validate()`/`EnvOverridable` + unit tests (PRICE-01)
- [ ] `crates/paladin-llm/src/pricing.rs` — new file, `PricingLlmAdapter` + unit/integration tests
      (PRICE-03)
- [ ] No shared test fixtures beyond what `crates/paladin-llm/src/mock.rs`'s existing
      `MockLlmAdapter` already provides (has `.with_response()`, `.with_provider_name()`,
      `.with_error()` per the verified `fallback.rs` test module) — reuse rather than duplicate

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | Out of scope — this phase touches no auth surface |
| V3 Session Management | No | Out of scope |
| V4 Access Control | No | Out of scope — tenant/API-key scoping is Phase 40/41, not this phase |
| V5 Input Validation | Yes | `treasurer.pricing.<model>.*` decimal-string parsing must reject malformed input at config-load `validate()`, never at runtime with a panic; `rust_decimal::Decimal::from_str_exact` (or the bounded hand-rolled parser) returns `Result`, never panics on bad input — mirrors the existing `TokenBudgetConfig::validate()` pattern of returning `Err(String)` rather than `unwrap()`ing |
| V6 Cryptography | No | This phase adds no cryptography; the ledger's idempotency key (`(run_id, superstep, attempt)`, D-15) is a database key, not a cryptographic construct, and is Phase 39's implementation concern |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Operator-supplied config string causing a parse panic (`Decimal::from_str().unwrap()` on malformed input) | Denial of Service | `validate()` must use `Result`-returning parse calls only, per `.github/instructions/security.instructions.md`'s "avoid `unwrap()`/`expect()` and `panic!` in library code" rule and per `rust.instructions.md`'s general error-handling convention; every existing `AgentRuntimeConfig` sub-struct's `validate()` already follows this (verified: `TokenBudgetConfig::validate()` returns `Result<(), String>`, no `unwrap()`) |
| Integer overflow in the `u32` tokens × `i64` per-1M-nano-price multiplication before the divide-by-1M step | Tampering (data integrity) / DoS via panic in a debug build | D-02 explicitly mandates `i128` for the intermediate product before dividing/rounding — `u32::MAX` (4.29B) × a plausible nano-price (tens of billions for an expensive model) can overflow `i64`'s ~9.2×10^18 range only in extreme synthetic cases, but `i128` removes the question entirely and costs nothing at this call frequency (per-LLM-call, not per-token) |
| An unpriced/mis-keyed model silently treated as `$0` in a future enforcement path (not built this phase, but the type contract set here determines whether it's even possible later) | Tampering / spend-governance bypass | D-00c/D-08's `None`-never-`0` contract is a security-relevant design choice for the MILESTONE even though this phase builds no enforcement — getting the `Option<Cost>` propagation right now (never defaulting to zero anywhere in the aggregation chain) is what makes Phase 41/42's admission/halt logic safe to build later without an audit of every intermediate hop |

No credential-handling code is touched by this phase (per the manual-review checklist in
`security.instructions.md`) — the pricing decorator reads no secret, and `TreasurerConfig` carries
no key/token/password-shaped field (mirrors `AgentRuntimeConfig`'s own documented "no field in this
tree is secret-shaped" invariant, which `treasurer:` should preserve).

## Sources

### Primary (HIGH confidence — direct `Read`/`Grep` of the shipped tree, this session, 2026-09-25)
- `crates/paladin-core/src/platform/container/token_usage.rs` — `TokenUsage` shape, saturating
  accumulation, `Option<u32>` merge rules (model for `Cost`)
- `crates/paladin-core/src/platform/container/herald.rs` — `ExecutionMetadata`,
  `ExecutionMetadataBuilder`, `cost_estimate` field, reserved rustdoc note, `Herald` trait
- `crates/paladin-core/src/platform/container/trace.rs` — `TraceEvent` twelve-variant enum,
  `NodeFinished`/`RunFinished` shapes, `TraceRecord` envelope
- `crates/paladin-core/src/platform/container/execution_result.rs` — `PaladinResult`, `StopReason`,
  the `served_by`/D-25/D-26 additive-field precedent
- `crates/paladin-ports/src/output/llm_port.rs` — `LlmResponse`, `LlmRequest`, `StreamingResponse`
  (terminal-chunk-only usage contract), `LlmError`/`transience()`
- `crates/paladin-llm/src/fallback.rs` — `FallbackLlmAdapter` full implementation and test suite (the
  decorator pattern to mirror)
- `src/config/agent_runtime.rs` — `AgentRuntimeConfig`, `TokenBudgetConfig`, `ToolCallLimitConfig`,
  the `Default`/`validate()`/`EnvOverridable` idiom, `build_chain`'s documented assembly order
- `src/application/services/paladin/middleware/limits.rs` — `TokenBudget` middleware implementation
- `crates/paladin-battalion/src/engine/hooks.rs` — `TraceDispatcher`, `total_usage()`
- `crates/paladin-battalion/src/engine/mod.rs` — `TraceEvent::RunFinished` emission sites (5 call
  sites, all constructing from `trace.total_usage()`), `NodeSpec::Paladin` execution via `PaladinPort`
- `src/infrastructure/web/facade_provisioner.rs` — `EngineExecutionPort`, `paladin_port_from_settings`
  (verified production engine-path LlmPort resolution)
- `src/infrastructure/web/agent_host.rs` — `build_agent_with_llm`, `default_provider_name` (verified
  production agent-loop-path LlmPort resolution)
- `src/presets/mod.rs` — `reasoning_agent` preset's middleware wiring (confirms `TokenBudget` is not
  wired here either)
- `crates/paladin-herald/src/{markdown,json,table}_herald.rs` — `finalize_stream` implementations
  (confirms table herald is a stub; markdown hard-codes `$`; JSON already emits `cost_estimate`)
- `.planning/config.json` — confirms no `workflow.nyquist_validation` or `security_enforcement` keys
  set (both default enabled)
- `.planning/decisions/PROMOTION.md` — ADR numbering index, confirms next free number is 0052, and
  the required ADR headings (`Status`, `Context`, `Decision`, `Considered Options`, `Code Locations`,
  `Code Conformance`, `Downstream Consumers`) via `0049`/`0050`'s own structure
- `.planning/decisions/0050-treasurer-reservation.md` — `Treasurer` reservation scope and the
  `GarrisonTreasury` guardrail text
- `deny.toml` — permissive license allow-list (`MIT`, `Apache-2.0`) confirming `rust_decimal`'s
  license is pre-approved
- `crates/paladin-core/Cargo.toml` — confirms `paladin-core`'s existing dependency set (`serde` etc.)
- `gsd-tools query package-legitimacy check --ecosystem crates rust_decimal` — verdict `OK`,
  2,761,074 weekly downloads, `github.com/paupino/rust-decimal`, not deprecated, no postinstall

### Secondary (MEDIUM confidence)
- `.planning/research/SUMMARY.md` and `.planning/research/PITFALLS.md` (2026-09-24 milestone-level
  research) — Pitfalls 1, 6, 8 restated above with direct source-line cross-checks against this
  session's own reads; the milestone research's "engine never wires `build_chain`" finding is
  independently reconfirmed and sharpened in this phase-level research (agent loop ALSO wires
  nothing, not only the engine)
- WebSearch confirming `rust_decimal` 1.43.0 as current published version (cross-checked against the
  `gsd-tools` registry lookup, which is the authoritative half of this claim)

### Tertiary (LOW confidence)
- None — no unverified web-only claims are load-bearing in this research.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — one new dependency, registry-verified via the package-legitimacy gate,
  license cross-checked against `deny.toml`
- Architecture: HIGH — every integration point (decorator installation, `ExecutionMetadata`
  producer gap, `build_chain`'s zero production callers) is grounded in a direct grep/read of the
  shipped tree, not inference from documentation
- Pitfalls: HIGH — five of six pitfalls are either directly sourced from the shipped code
  (table_herald stub, ExecutionMetadata producer gap, build_chain wiring gap) or restated from
  prior milestone-level research with independent source-line verification in this session

**Research date:** 2026-09-25
**Valid until:** 30 days (stable domain — no fast-moving external API surface is involved; the one
external dependency, `rust_decimal`, is a mature, slow-moving crate)
