# Phase 32: Unified Token Primitives - Research

**Researched:** 2026-09-15
**Domain:** Rust API consolidation — port trait extension, constructor signature break, dead-code removal, shared precedence-resolver extraction (no new external dependencies)
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

D-01 through D-16 from `32-CONTEXT.md` are locked. Summarized here for quick reference; the
planner should still read `32-CONTEXT.md` directly for the full text:

- **D-01:** Strict mode is a fallback-policy enum (`Default(u32)` / `Strict { caller_fallback:
  Option<u32> }`), not a `bool`. Same precedence walk for both; strict errors when step 3 has
  nothing.
- **D-02:** Placement is a top-level `paladin_llm::window` module (not `services::window`, not a
  private helper). Takes `model: &str`, an optional per-model table, `ProviderCapabilities` (or
  its `max_context_tokens`), and the D-01 policy. Returns `Result<ResolvedWindow,
  UnknownContextWindow>` carrying both the token count and which step produced it. Facade
  `src/lib.rs` re-exports the resolver types next to the Commissary block (~line 200).
- **D-03:** `Commissary` gains NO config table — `CommissaryPlan` unchanged; Commissary passes an
  empty/absent table so step 1 is always a no-op (no behavioral drift, PRD §4).
- **D-04:** Commissary calls the resolver ONCE, in `new`, replacing the inline `.or(...)` guard;
  the resolved `u32` is stored, `window()` returns it. The resolver's unknown-window error maps to
  the existing `CommissaryError::UndeclaredContextWindow { provider }` — same variant, same
  Display text. `from_port` unchanged apart from dropping `is_exact_counter`.
- **D-05:** `HistoryTrimmer::resolve_limit` becomes a thin call to the resolver with
  `Default(config.default_context_tokens)` and `&config.model_context_limits`; `LimitSource` is
  either deleted in favor of `WindowSource` or kept as a one-line mapping (planner's discretion),
  but the three logged source strings must stay recognizable to the existing tests
  (`contains("provider")`/`("capabilities")`/`("default")`). Trim semantics unchanged.
- **D-06:** `is_exact` is a property of the adapter instance as constructed, not of a call.
  Default `false`. Rustdoc states this; doc test shows a bare impl inheriting `false`.
- **D-07:** `TiktokenCounter::is_exact` returns `true` unconditionally (rustdoc: exact for the
  encoding resolved at `new(model)`, `count`'s `model` argument is ignored).
  `HeuristicTokenCounter` relies on the trait default (no override); its unit test asserts
  `false`. The tiktoken test lives under `content-processing`.
- **D-08:** `Commissary` drops its private `is_exact_counter` field and reads
  `self.counter.is_exact()` where `Stockpile::exact_tally` is set. `Debug` impl prints
  `counter.name()` and the live `is_exact()`. `new`/`from_port` both lose the argument. Test
  doubles gain a configurable exactness where needed.
- **D-09:** The `#[deprecated]` escape hatch is NOT used — zero in-tree callers of
  `TokenCounter`/`TokenCounterFactory` outside the definition file and the three re-export sites
  (confirmed by this research's own grep). Both removed outright in one commit, including
  `TokenCounterFactory::for_model`, `supported_models`, `is_supported`.
- **D-10:** What survives on `TiktokenCounter`: `new(model) -> Result<Self, GarrisonError>`
  (construction stays fallible), `impl TokenCounterPort` (now the only counting path — inlines the
  BPE lookup and cache directly), an inherent `model_name(&self) -> &str` accessor, and
  `is_exact() -> true`. No fallible inherent `count_tokens` is kept.
- **D-11:** The facade's `token_counter` sub-module and the top-level re-export at line 9 are kept,
  narrowed to `TiktokenCounter`. `paladin-memory`'s `garrison/mod.rs:16` and `prelude.rs:10`
  likewise keep `TiktokenCounter` only.
- **D-12:** Doc sweep for removed names/dropped argument across `paladin-memory/src/lib.rs`,
  `docs/src/architecture/crate-map.md:178`, `docs/src/user-guides/memory-management.md:159`,
  `commissary.rs` module docs, `docs/src/architecture/commissary.md` (usage sketch mirrors a real
  unit test per Phase 30 D-06; "Honesty about exactness" rewritten), and a short "Token
  primitives" subsection in `upgrading.md`/`migration-guide.md` pointing at §9.2. Exit grep:
  `grep -rnE '\bTokenCounterFactory\b|garrison::TokenCounter\b|is_exact_counter' crates src docs examples benches`
  → empty.
- **D-13:** The equivalence snapshot is TDD-ordered fixture tests (no `insta`). First plan writes
  table-driven tests AGAINST PRE-REFACTOR code and commits them green: Commissary windows for
  {capabilities hit, capabilities `None` + fallback hit, both `None` → `UndeclaredContextWindow`}
  and `allotted_tokens` for each, and HistoryTrimmer kept-sets for {config-table hit, capability
  hit, default} over a fixed history — distinct, non-round numbers. Refactor commit(s) keep tests
  byte-identical and green. Four separate precedence tests on the resolver itself.
- **D-14:** Semver lint discovery is empirical, extended for feature gating. Run discovery for
  `paladin-ports`, `paladin-llm`, `paladin-memory`, `paladin-ai` (default-features) AND
  `paladin-memory`/`paladin-ai` again with `--features content-processing`. Record every lint id
  that fires; write allowlist entries naming the feature gate. `--release-type minor` is
  mandatory on every local discovery run.
- **D-15:** `MIGRATION.md` §9.2 gets one row per `crate | Type` pair: `paladin-llm | Commissary`
  (covers both `new`/`from_port`), `paladin-memory | TokenCounter`, `paladin-memory |
  TokenCounterFactory`; `paladin-ai` rows only if a lint fires; `paladin-ports | TokenCounterPort`
  gets a row only if a lint fires (additive defaulted method), else CHANGELOG-only mention. Every
  `Y`-marked row gets its allowlist `[[entry]]` in the SAME commit.
- **D-16:** `CHANGELOG.md` `[0.10.0]`: a `### Changed` bullet for the `Commissary::new`/`from_port`
  signature + shared resolver, a `### Removed` bullet for the legacy pair, an `### Added` bullet
  for `TokenCounterPort::is_exact`. Commit with plain `git commit` (never the GSD commit helper —
  known to time out). PRIM-05 gate evidence goes in the last plan's SUMMARY.

**Breaking, clean break, no shims** — governed by ADR-0051 (X-03 superseded for Phases 31-33
only). No forwarding constructor for `Commissary::new`, no `#[deprecated]` legacy trait, no
compatibility re-export of the removed names.

**Not in this phase:** any change to Commissary's dispensing behaviour or the windows it
resolves; a per-model override table for Commissary; a production caller for Commissary and the
RAG truncation path (Phase 33, COMM-01…03); re-sealing the Phase 29 release gates (Phase 33,
COMM-04); renaming any token type (Phase 30 D-17); pricing, `cost_estimate`, allowances, pacing
(Milestone 14); moving the resolver into `paladin-ports` (the roadmap and PRD lock `paladin-llm`).

### Claude's Discretion

- Exact type and variant names (`WindowFallback`/`WindowPolicy`, `WindowSource`,
  `ResolvedWindow`, `UnknownContextWindow`) and whether the resolver takes
  `&ProviderCapabilities` or the bare `Option<u32>`.
- Whether `history.rs`'s `LimitSource` is deleted or kept as a mapping to `WindowSource` (D-05).
- Whether `TiktokenCounter`'s per-string cache stays an `RwLock<HashMap>` or is simplified while
  the trait is removed — behaviour (same counts, same cache hits) must not change.
- Test-double naming and where the configurable-exactness knob lives on them.
- Plan/wave granularity — natural waves: (1) equivalence fixtures against pre-refactor code +
  `is_exact` on the port and both adapters (PRIM-01); (2) the shared resolver + precedence tests,
  HistoryTrimmer and Commissary consuming it, `Commissary::new` break (PRIM-02, PRIM-04);
  (3) legacy removal + re-export narrowing + doc sweep (PRIM-03); (4) §9.2 rows, allowlist,
  CHANGELOG, upgrading page, gate evidence (PRIM-05).

### Deferred Ideas (OUT OF SCOPE)

- A per-model context-window override table on `CommissaryPlan` — new capability, not this phase;
  backlog / Phase 33+ if a caller needs it.
- A production caller for `Commissary` and replacing RAG's `content.len() / 4` truncation — Phase
  33 (COMM-01…03); re-sealing the Phase 29 release gates — Phase 33 (COMM-04).
- Placing the resolver in `paladin-ports` instead of `paladin-llm` — considered, rejected for this
  phase because ROADMAP success criterion 4 and PRD R4 lock `paladin-llm`; revisit only if a
  `paladin-ports`-only consumer appears.
- `#[non_exhaustive]` on `CommissaryPlan` / `Commissary` — not needed while the milestone is a
  clean-break window; a later phase can decide.
- Treasurer, pricing, `cost_estimate`, allowances, pacing — Milestone 14 (ADR-0050).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PRIM-01 | `TokenCounterPort` gains `fn is_exact(&self) -> bool` defaulting to `false`; tiktoken returns `true`, heuristic returns `false`, each proven by a test | Existing Code Insights (`TokenCounterPort`, `HeuristicTokenCounter`, `TiktokenCounter` sections) give exact current shapes and the doc-test/unit-test extension points; Code Examples gives the doc-test pattern; Validation Architecture maps to `cargo test -p paladin-ports --doc` / `-p paladin-memory --features content-processing` |
| PRIM-02 | `Commissary::new` drops `is_exact_counter`, reads exactness from the port; every in-tree call site compiles | `Commissary` section confirms the exact struct/constructor shape and enumerates all 10 test call sites (zero production callers) that must be updated; Architecture Patterns Pattern 1 shows the mapping from resolver error to `CommissaryError::UndeclaredContextWindow` |
| PRIM-03 | Legacy `garrison::TokenCounter`/`TokenCounterFactory` removed with three re-exports narrowed, every former in-tree caller migrated | Legacy counter section + re-export table give exact removal targets; grep confirms D-09's "zero in-tree callers" claim, eliminating the `#[deprecated]` escape-hatch path; Common Pitfalls 3 covers the feature-gate CI-visibility trap for this removal |
| PRIM-04 | Shared resolver in `paladin-llm` (config table → capabilities → default, explicit strict mode); both `HistoryTrimmer` and `Commissary` consume it; precedence tests + equivalence snapshot | `HistoryTrimmer`/`resolve_limit`/`LimitSource` section gives the exact "resolver in miniature" to generalize; Architecture Patterns Pattern 1 gives a full design sketch (`WindowFallbackPolicy`, `WindowSource`, `ResolvedWindow`, `UnknownContextWindow`, `resolve_context_window`); Code Examples gives the equivalence-fixture shape (D-13); Common Pitfalls 4 covers the source-string regression risk |
| PRIM-05 | `MIGRATION.md` §9.2 rows + semver allowlist rows (row-level gate green); `CHANGELOG.md` `[0.10.0]` entries; `make clean-code` and coverage floor green | Sources section names the exact template rows (`PaladinResult`/`LlmRequest`/`TokenUsage`) and the allowlist schema; Common Pitfalls 1-3 cover the three semver-discovery traps (release-type flag, crate-level-allow invisibility, feature-gate CI blindness) verified against Phase 31's own documented findings; Validation Architecture gives the exact discovery + CI-matching commands |
</phase_requirements>

## Summary

Phase 32 is a pure in-tree Rust refactor with zero new external dependencies. All five success
criteria are mechanical once the exact current shapes are pinned down, which this research does
by reading every file CONTEXT.md's canonical refs name. There is nothing to "discover" about a
third-party library here — `tiktoken-rs` is already a dependency, `thiserror`/`serde` patterns
are already established, and the CONTEXT.md decisions (D-01 through D-16) already fix the type
shapes, module placement and precedence order. The research value-add is: (1) confirming D-09's
"zero in-tree callers of the legacy trait outside its own file and the three re-export sites"
claim by grep, so PRIM-03 can proceed without a `#[deprecated]` escape hatch; (2) confirming
`Commissary::new`/`from_port` have **no production caller today** — every call site is a test or
a doc example — which simplifies PRIM-02's "every call site compiles" claim to ten in-crate test
sites plus one mdBook fenced block; (3) pinning the exact resolver precedence shapes
`HistoryTrimmer::resolve_limit` and `Commissary::new`'s inline guard already implement, so the
new `paladin_llm::window` module is a faithful generalization, not a redesign; and (4) locating
every MIGRATION.md / allowlist / CHANGELOG / docs template site so the release-bookkeeping wave
(PRIM-05) copies proven patterns rather than inventing new ones.

**Primary recommendation:** Implement in the four waves CONTEXT.md's Claude's Discretion already
lays out — (1) equivalence fixtures + `is_exact` on the port and both adapters, (2) the
`paladin_llm::window` resolver + both consumers + the `Commissary::new` break, (3) legacy
removal + re-export narrowing + doc sweep, (4) MIGRATION.md/allowlist/CHANGELOG/upgrading-page +
gate evidence — writing the equivalence fixtures FIRST, against the pre-refactor code, exactly as
D-13 specifies, so their continued-green-ness across the refactor IS the equivalence proof.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Token counting (`count`, `name`, `is_exact`) | API / Backend (`paladin-ports` trait + `paladin-memory` adapters) | — | Pure, local, CPU-bound computation; no I/O, no async — lives in the port/adapter tier, never in application middleware |
| Context-window precedence resolution | API / Backend (`paladin-llm::window`) | Application/Backend (facade `history.rs`, `commissary.rs` as consumers) | The precedence walk (config table → capabilities → fallback) is provider-capability logic that belongs beside `ProviderCapabilities` in `paladin-llm`, not duplicated in the facade's application layer |
| Prompt budget enforcement (`Commissary::verify_fits`/`dispense`) | API / Backend (`paladin-llm::services::commissary`) | — | Consumes the window resolver and the counter port; no change to this phase |
| History trimming policy (`KeepSystemAndRecent`) | Application / Backend (`src/application/services/paladin/middleware/history.rs`) | — | Application-specific trimming algorithm; only the window-*resolution* sub-step moves to `paladin-llm`, trim semantics stay put (D-05) |
| Release bookkeeping (MIGRATION.md, allowlist, CHANGELOG) | N/A (process, not runtime tier) | — | Documentation/tooling artifacts, not application architecture |

## Package Legitimacy Audit

**No external packages are installed by this phase.** `tiktoken-rs = "0.6.0"` (optional, feature
`content-processing`) is an EXISTING dependency in `crates/paladin-memory/Cargo.toml:38`,
unchanged by this phase. No `Cargo.toml` in any touched crate (`paladin-ports`, `paladin-memory`,
`paladin-llm`, root `paladin-ai`) gains a new `[dependencies]` entry. The Package Legitimacy Gate
protocol is therefore not applicable — this section is included per the output contract to record
that the gate was considered and found not to apply, not skipped silently.

## Standard Stack

No new libraries. This phase reuses the exact in-tree patterns already established:

### Core (already in the tree, unchanged)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `thiserror` | workspace-pinned | `CommissaryError`, the new resolver error type | House error pattern (per-module enum, `#[error(...)]` Display) — see `security.instructions.md`/`rust.instructions.md` |
| `tiktoken-rs` | `0.6.0` (existing, `content-processing` feature) | `TiktokenCounter`'s BPE tokenization | Unchanged by this phase — only the trait it implements is touched |
| `serde` | workspace-pinned | `HistoryTrimmerConfig`, `#[serde(default)]` on config fields | Unchanged |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| A `paladin_llm::window` module (D-02, locked) | Placing the resolver in `paladin-ports` as a pure function over `ProviderCapabilities` | Rejected in CONTEXT.md's Deferred Ideas: ROADMAP success criterion 4 and the PRD lock `paladin-llm`; revisit only if a `paladin-ports`-only consumer appears |
| `insta` snapshot testing for the equivalence proof (D-13, locked) | Hand-written table-driven `#[test]` fixtures | Rejected: only `paladin-eval` pins `insta` in this workspace; adding it to `paladin-llm` or the facade for one phase buys nothing over ordinary `assert_eq!` fixtures |

**Installation:** None required — no `cargo add` in this phase.

**Version verification:** Not applicable (no new/bumped dependency). `tiktoken-rs 0.6.0` is
already pinned and unaffected.

## Existing Code Insights (the exact shapes to build against)

### `TokenCounterPort` (`crates/paladin-ports/src/output/token_counter_port.rs`, 77 lines)

```rust
pub trait TokenCounterPort: Send + Sync {
    fn count(&self, text: &str, model: &str) -> u32;
    fn name(&self) -> &str;
}
```

Trait module doc already names the two adapters and the infallibility rationale (lines 1-27). The
doc test at lines 47-66 is a bare `impl TokenCounterPort for AlwaysOne` — this is the exact
example CONTEXT.md's `<specifics>` says to extend with `assert!(!counter.is_exact())` to prove
the default (PRIM-01's own proof-on-the-port requirement). **Add** (D-06):

```rust
fn is_exact(&self) -> bool {
    false
}
```

Rustdoc must state (D-06): `is_exact` is a property of the adapter **instance as constructed**,
not of a `(text, model)` call — `true` means `count` returns the model's own tokenizer tally,
`false` means an approximation.

### `HeuristicTokenCounter` (`crates/paladin-memory/src/token_counter/heuristic.rs`, 87 lines)

`impl TokenCounterPort` at line 19; `count` is `text.chars().count() / 4` rounded up, `name()` ==
`"heuristic"`. **Relies on the trait default** for `is_exact` (D-07) — add no override, add one
test asserting `!HeuristicTokenCounter.count`... i.e. `assert!(!HeuristicTokenCounter.is_exact())`
— this test doubles as PRIM-01's "defaulting to false" proof.

### Legacy `TokenCounter`/`TiktokenCounter`/`TokenCounterFactory` (`crates/paladin-memory/src/garrison/token_counter.rs`, 381 lines)

- `trait TokenCounter` (line 13): `count_tokens(&self, text: &str) -> Result<u32, GarrisonError>`,
  `model_name(&self) -> &str`. **Removed outright** (D-09).
- `struct TiktokenCounter` (line 49): `bpe: CoreBPE`, `model_name: String`, `cache:
  RwLock<HashMap<String, u32>>`. `new(model_name: &str) -> Result<Self, GarrisonError>` (line 74)
  **survives unchanged** — construction stays fallible. `clear_cache`/`cache_size` inherent
  methods survive. `impl TokenCounter for TiktokenCounter` (line 102, `count_tokens` +
  `model_name`) is **removed**; `impl TokenCounterPort for TiktokenCounter` (line 141) already
  exists and **survives**, gaining `is_exact() -> true` unconditionally (D-07) and absorbing the
  BPE-lookup-plus-cache logic directly (currently it just delegates to `count_tokens`, D-10: "no
  fallible inherent `count_tokens` is kept" — the cache/BPE logic inlines straight into `count`).
  An inherent `model_name(&self) -> &str` accessor is **kept** (D-10: already used by tests, cheap,
  useful in logs) — this is now an inherent method, not a trait method.
- `struct TokenCounterFactory` (line 165): `for_model`, `supported_models`, `is_supported` — all
  **removed outright**, no callers (confirmed by grep below).
- Sixteen `#[cfg(test)]` tests in this file use `count_tokens()`/`model_name()` (trait methods) —
  these must be rewritten to use the `TokenCounterPort::count`/inherent `model_name` shapes, or
  deleted where they test factory/trait behavior being removed (`test_factory_*`,
  `test_tiktoken_counter_creation`'s `model_name()` call becomes the inherent accessor, etc). Two
  tests already target the port directly (`tiktoken_counter_implements_the_port`,
  `tiktoken_counter_falls_back_inside_the_adapter_for_an_unknown_model`, lines 342-370) and are
  reusable templates for the exactness test (`assert!(counter.is_exact())`).

### Re-export sites (confirmed by read, D-11: narrow to `TiktokenCounter` only, don't remove the paths)

| File | Line | Current | Target |
|------|------|---------|--------|
| `crates/paladin-memory/src/garrison/mod.rs` | 16 | `pub use token_counter::{TiktokenCounter, TokenCounter, TokenCounterFactory};` | `pub use token_counter::TiktokenCounter;` |
| `crates/paladin-memory/src/prelude.rs` | 10 | `pub use crate::garrison::{TiktokenCounter, TokenCounter, TokenCounterFactory};` | `pub use crate::garrison::TiktokenCounter;` |
| `src/infrastructure/adapters/garrison/mod.rs` | 9 | `pub use paladin_memory::garrison::{TiktokenCounter, TokenCounter, TokenCounterFactory};` | `pub use paladin_memory::garrison::TiktokenCounter;` |
| `src/infrastructure/adapters/garrison/mod.rs` | 22-26 (`token_counter` sub-module) | re-exports all three | re-exports `TiktokenCounter` only |

### `Commissary` (`crates/paladin-llm/src/services/commissary.rs`, 1009 lines)

Current struct (line 300-306):
```rust
pub struct Commissary {
    counter: Arc<dyn TokenCounterPort>,
    capabilities: ProviderCapabilities,
    provider: String,
    config: CommissaryPlan,
    is_exact_counter: bool,       // DROPPED (D-08)
}
```

`new` (line 338-380) takes `(provider, capabilities, counter, is_exact_counter: bool, config)` and
returns `Result<Self, CommissaryError>`. The inline window guard (line 359-364):
```rust
let window = capabilities
    .max_context_tokens
    .or(config.fallback_context_tokens)
    .ok_or_else(|| CommissaryError::UndeclaredContextWindow { provider: provider.clone() })?;
```
This is the exact logic PRIM-04's resolver replaces (D-04) — `window` becomes the resolver's
output, called ONCE in `new`, with the resolver's own error mapped into
`CommissaryError::UndeclaredContextWindow { provider }` (same variant, same Display text: `` "provider
'{provider}' declared no max_context_tokens and no fallback_context_tokens was configured" `` —
line 235-238). `from_port` (line 389-402) forwards to `new`, dropping only `is_exact_counter`.
`window()` (private fn, line 408-413) also duplicates the same `.or(...)` — after the refactor
this becomes a stored field set once in `new`, not recomputed on every `allotted_tokens()` call
(D-04: "the resolved `u32` is stored and `window()` returns it").

`Debug` impl (line 308-317) prints `is_exact_counter` field — replace with a live
`self.counter.is_exact()` call (D-08).

`Stockpile.exact_tally` (line 208-213, set at line 547) is currently `self.is_exact_counter` —
becomes `self.counter.is_exact()` read at construction time and cached, OR read live at dispense
time; D-08 says the field is dropped entirely and `Stockpile::exact_tally` is set "where
`Stockpile::exact_tally` is set" from `self.counter.is_exact()` — i.e. `Commissary` itself keeps
no separate `is_exact_counter`/cached bool field; it just calls `self.counter.is_exact()` whenever
needed (construction-time for the window guard is irrelevant; dispense-time for `exact_tally`).

**Call sites** (verified via grep — `Commissary::new(` / `Commissary::from_port(` appear ONLY in
`commissary.rs` itself, ten occurrences: lines 632, 806, 826, 844, 867, 885, 953, 970, 992
(`from_port`), 995 — all in `#[cfg(test)] mod tests` — plus one in
`docs/src/architecture/commissary.md` (fenced `rust,ignore`, not compiled). **There is no
production caller anywhere in the tree** — confirmed: grep for `Commissary::new(`/`Commissary::
from_port(` across `crates/ src/ examples/ benches/` returns only the test file and the doc page.
This means PRIM-02's "every in-tree call site compiles" scope is exactly: 10 test-fixture calls in
`commissary.rs` (drop the `is_exact_counter` positional argument from each) plus updating the doc
page's fenced example (not compiled, but D-12 requires the sweep anyway).

`MockCounter` test double (line 607-618) and the `commissary()`/`counter()`/
`capabilities_with_window()` test helpers (line 620-640) are the seams for both the exactness
tests (add a configurable-exactness variant) and the equivalence fixtures (D-13's TDD-ordered
table tests reuse these exact helpers).

### `HistoryTrimmer` (`src/application/services/paladin/middleware/history.rs`, 622 lines)

```rust
enum LimitSource { ConfigTable, ProviderCapabilities, Default }   // lines 62-83

fn resolve_limit(&self, model: &str) -> (u32, LimitSource) {      // lines 115-123
    if let Some(&limit) = self.config.model_context_limits.get(model) {
        return (limit, LimitSource::ConfigTable);
    }
    if let Some(max_context_tokens) = self.llm_port.get_capabilities().max_context_tokens {
        return (max_context_tokens, LimitSource::ProviderCapabilities);
    }
    (self.config.default_context_tokens, LimitSource::Default)
}
```

This is the "resolver in miniature" D-01/D-02 generalize. Three existing precedence tests (lines
296-333): `limit_resolution_prefers_the_config_table`, `limit_resolution_falls_back_to_provider_
capabilities`, `limit_resolution_falls_back_to_the_default` — each asserts the resolved `u32` AND
`source.as_str().contains("config"/"provider"/"capabilities"/"default")`. **D-05 requires these
substring assertions to keep passing** — whatever `WindowSource`'s string form is (or however
`LimitSource::as_str()` maps from it), the words `"config"`, `"provider"`/`"capabilities"`, and
`"default"` must still appear in the logged/returned source text. `CapabilityOnlyLlmPort` (lines
218-262) is the minimal `LlmPort` test fixture used by all three — it is also the template for the
resolver's own precedence tests (`&ProviderCapabilities` or an `Option<u32>` argument).

`HistoryTrimmerConfig` (`src/config/agent_runtime.rs:697-720`, confirmed):
```rust
pub struct HistoryTrimmerConfig {
    pub enabled: bool,
    pub reserve_for_response: u32,
    pub default_context_tokens: u32,
    #[serde(default)]
    pub model_context_limits: HashMap<String, u32>,
    pub recall_limit: u32,
}
```
`Default` sets `default_context_tokens: 8192`, `reserve_for_response: 1024`. Two production
`HistoryTrimmer::new` sites: `src/config/agent_runtime.rs:~384` (inside the middleware-chain
assembly `else if self.history_trimmer.enabled { ... }` branch) and
`src/application/services/paladin/middleware/summarization.rs:150-172`
(`SummarizationMiddleware::new`, constructing an `embedded_trimmer`). Both pass
`HistoryTrimmerConfig`/`counter`/`llm_port` positionally — **`HistoryTrimmer::new`'s own signature
does not change** in this phase; only its PRIVATE `resolve_limit` body changes to call the shared
resolver.

### `ProviderCapabilities` (`crates/paladin-ports/src/output/llm_port.rs:~1169-1196`, confirmed)

```rust
pub struct ProviderCapabilities {
    pub supports_streaming: bool,
    pub supports_tool_calling: bool,
    pub supports_function_calling: bool,
    pub supports_vision: bool,
    pub supports_embeddings: bool,
    pub max_context_tokens: Option<u32>,
    pub supports_system_messages: bool,
    pub temperature_range: Option<(f32, f32)>,
}
```
Only `max_context_tokens: Option<u32>` matters to the resolver. Passing the whole
`&ProviderCapabilities` vs. just `Option<u32>` is Claude's Discretion (CONTEXT.md) — either is
compiles-clean; `&ProviderCapabilities` is slightly more future-proof and matches what both
current call sites already have in hand (`Commissary` holds `capabilities: ProviderCapabilities`;
`HistoryTrimmer` calls `self.llm_port.get_capabilities()` fresh each time).

## Architecture Patterns

### System Architecture Diagram

```
┌─────────────────────────────┐        ┌──────────────────────────────┐
│  HistoryTrimmer::resolve_limit │        │  Commissary::new              │
│  (facade, lenient/Default)     │        │  (paladin-llm, strict)        │
│  model, config.model_context_  │        │  provider, capabilities,      │
│  limits, llm_port.capabilities │        │  config.fallback_context_     │
└──────────────┬────────────────┘        │  tokens                       │
               │                          └──────────────┬─────────────────┘
               │  calls                                   │  calls
               ▼                                           ▼
        ┌──────────────────────────────────────────────────────────┐
        │        paladin_llm::window::resolve_context_window        │
        │  1. config table lookup for `model`   -> WindowSource::   │
        │     ConfigTable                                           │
        │  2. capabilities.max_context_tokens   -> WindowSource::   │
        │     ProviderCapabilities                                  │
        │  3. policy fallback:                                      │
        │     - Default(u32)             -> always resolves          │
        │       (WindowSource::Default)                              │
        │     - Strict{caller_fallback}  -> Some -> WindowSource::   │
        │       CallerFallback; None -> Err(UnknownContextWindow)    │
        └───────────────┬─────────────────────────┬──────────────────┘
                         │ Ok(ResolvedWindow)       │ Err(UnknownContextWindow)
                         ▼                          ▼
              stored as trimmer's limit     Commissary::new maps to
              / Commissary's window field   CommissaryError::
                                             UndeclaredContextWindow
                                             { provider }  (same Display
                                             text as today)
```

### Recommended Project Structure

```
crates/paladin-llm/src/
├── services/
│   ├── commissary.rs        # unchanged location; new()/from_port() call window::resolve_context_window
│   └── mod.rs                # unchanged
├── window/                   # NEW top-level module (D-02 — sibling of `services`, not nested under it)
│   └── mod.rs                # resolve_context_window, WindowFallbackPolicy, WindowSource, ResolvedWindow, UnknownContextWindow
└── lib.rs                    # `pub mod window;` added to the module list (~line 53-172)
```

### Pattern 1: Fallback-policy enum, not a bool (D-01)

**What:** Two-variant enum carried into the resolver instead of a `strict: bool` plus a separate
`Option<u32>` fallback (which can disagree with the bool).
**When to use:** Any time "strict mode" needs an associated value only in one branch.
**Example (design sketch, not yet in the tree — D-01/D-02):**
```rust
// Source: CONTEXT.md D-01/D-02 (paladin_llm::window design)
pub enum WindowFallbackPolicy {
    /// Always resolves — HistoryTrimmer passes `config.default_context_tokens`.
    Default(u32),
    /// May refuse — Commissary passes `config.fallback_context_tokens`.
    Strict { caller_fallback: Option<u32> },
}

pub enum WindowSource {
    ConfigTable,
    ProviderCapabilities,
    Default,
    CallerFallback,
}

pub struct ResolvedWindow {
    pub tokens: u32,
    pub source: WindowSource,
}

#[derive(Debug, thiserror::Error, Clone, PartialEq)]
#[error("no context window could be resolved for model '{model}'")]
pub struct UnknownContextWindow {
    pub model: String,
}

pub fn resolve_context_window(
    model: &str,
    config_table: Option<&std::collections::HashMap<String, u32>>,
    capabilities: &paladin_ports::output::llm_port::ProviderCapabilities,
    policy: WindowFallbackPolicy,
) -> Result<ResolvedWindow, UnknownContextWindow> {
    if let Some(&tokens) = config_table.and_then(|t| t.get(model)) {
        return Ok(ResolvedWindow { tokens, source: WindowSource::ConfigTable });
    }
    if let Some(tokens) = capabilities.max_context_tokens {
        return Ok(ResolvedWindow { tokens, source: WindowSource::ProviderCapabilities });
    }
    match policy {
        WindowFallbackPolicy::Default(tokens) => {
            Ok(ResolvedWindow { tokens, source: WindowSource::Default })
        }
        WindowFallbackPolicy::Strict { caller_fallback: Some(tokens) } => {
            Ok(ResolvedWindow { tokens, source: WindowSource::CallerFallback })
        }
        WindowFallbackPolicy::Strict { caller_fallback: None } => {
            Err(UnknownContextWindow { model: model.to_string() })
        }
    }
}
```
This is a design sketch to guide the planner — exact names are Claude's Discretion per CONTEXT.md
(`WindowFallback`/`WindowPolicy`, `WindowSource`, `ResolvedWindow`, `UnknownContextWindow` are all
open to renaming). `Commissary::new` maps `Err(UnknownContextWindow { .. })` to
`CommissaryError::UndeclaredContextWindow { provider: provider.clone() }` — discarding the
resolver's own `model` field, since the existing Display text names the provider, not the model,
and D-04 requires byte-identical Display text.

### Pattern 2: Empty/absent config table for Commissary (D-03)

**What:** `Commissary` passes `None` (or an empty `&HashMap`) as the resolver's config-table
argument — `CommissaryPlan` gains NO new field in this phase. Step 1 is always a no-op for
Commissary, so every window it resolves today keeps resolving identically.
**When to use:** Whenever a consumer of the shared resolver doesn't yet have a per-model override
concept — don't invent one just because the resolver's signature has a slot for it.

### Anti-Patterns to Avoid
- **Re-adding `Commissary`'s own inline `.or(...)` guard "just to be safe":** defeats PRIM-04's
  single-resolver goal; the whole point is Commissary and HistoryTrimmer share ONE precedence
  walk.
- **Giving the resolver an `async` signature:** the counting/resolving seam is deliberately
  synchronous throughout this phase's domain (mirrors `TokenCounterPort`'s own sync design) — no
  I/O is involved in a precedence table lookup.
- **Keeping a cached `is_exact_counter: bool` field on `Commissary` "for performance":** D-08 is
  explicit — one source of truth, no cached duplicate; `self.counter.is_exact()` is a trivial call
  on every implementor (`true`/`false` constant or a stored bool read).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Precedence resolution with a named source | A second bespoke `LimitSource`-shaped enum per consumer | The single `paladin_llm::window::WindowSource` (PRIM-04) | Two independent enums with the same three-branch shape is exactly the duplication this phase exists to collapse |
| Exact-vs-approximate signaling | A parallel `HashMap<TypeId, bool>` or trait-object downcast to infer exactness | `TokenCounterPort::is_exact(&self) -> bool` (PRIM-01) | The port is the one seam every counter already flows through; adding a defaulted method is the minimal, idiomatic Rust way to add a capability flag to an existing trait |

**Key insight:** Every "don't hand-roll" temptation in this phase is a temptation to reintroduce
the exact duplication the phase is chartered to remove — watch for a second precedence enum or a
second exactness signal sneaking back in during implementation.

## Common Pitfalls

### Pitfall 1: `cargo semver-checks` silently skips every lint at `0.9.0 -> 0.10.0`
**What goes wrong:** Running the plan's own discovery command without `--release-type minor`
prints `Checking ... (major change)` / `0 checks: 0 pass, N skip` and exits 0, giving false
confidence that nothing is semver-breaking.
**Why it happens:** `cargo-semver-checks@0.50.0` treats a `0.9.0 -> 0.10.0` bump (already applied
to every crate's `Cargo.toml` per the Phase 31 finding cited in this phase's own brief) as already
major-equivalent, so it skips lint evaluation entirely rather than evaluating against the true
`0.9.0` baseline.
**How to avoid:** Every LOCAL discovery run must carry `--release-type minor` (D-14, confirmed
against Phase 31's own `31-07-SUMMARY.md` "Issue/Fix/Verification" entry, lines 182-185). The
CI job's own invocation (`ci.yml:~357`, no `--release-type` flag) is left as-is — it is the
`.cargo/semver-checks-allowlist.toml` entries, not the CI command, that make the job stay green.
**Warning signs:** `0 checks: 0 pass, N skip` in the discovery output — that is the failure
signature, not a clean pass.

### Pitfall 2: A lint already crate-level-`allow`ed is invisible even when a NEW occurrence fires it
**What goes wrong:** `paladin-ports`' existing `struct_marked_non_exhaustive = "allow"` (from
`LlmRequest`, Phase 26) silently also covers any NEW type this phase marks `#[non_exhaustive]` —
so a genuinely new occurrence produces no visible tool output to confirm it happened.
**Why it happens:** `cargo semver-checks` skips evaluating a lint entirely once it is
crate-level-allowed, rather than evaluating-then-suppressing (Phase 31's own documented finding,
`31-07-SUMMARY.md` tech-stack pattern).
**How to avoid:** This phase is unlikely to need this specific pitfall's workaround since
`TokenCounterPort::is_exact` is an ADDITIVE DEFAULTED trait method (no new lint expected — D-14
says "likely nothing" fires for it) and `paladin-llm` has NO existing lints table to interact with
this way. But if `paladin-memory`'s `content-processing`-gated lints DO need a temporary
`allow -> warn` flip to confirm a new occurrence (Phase 31 precedent), revert it immediately after
confirming — do not leave the crate-level severity flipped.
**Warning signs:** A discovery run reports a lint you expected to fire, but zero new source
locations are listed under it — likely because it was already suppressed, not because nothing
new triggered it.

### Pitfall 3: Feature-gated code is invisible to the CI `semver` job's default-features run
**What goes wrong:** `TokenCounter`/`TokenCounterFactory` sit behind `content-processing` in
`paladin-memory` (and the facade forwards the feature via root `Cargo.toml:515`) — CI's `semver`
job runs `--default-features` only (`ci.yml:~357`), so removing these types produces **zero**
lint output from that job's normal invocation; the CI job cannot observe PRIM-03's removal at all
under its own command.
**Why it happens:** `--default-features` deliberately excludes optional features (documented
rationale in `ci.yml`'s own comment block, ~lines 271-300) to dodge an unrelated pre-existing
qdrant-client rustdoc break.
**How to avoid:** Run the discovery command TWICE for `paladin-memory` (and `paladin-ai`, if the
facade's own feature-forwarded re-export is affected): once `--default-features`, once
`--features content-processing`, both with `--release-type minor` (D-14's exact command pair is
given in `<specifics>` of CONTEXT.md and reproduced in this research's Validation Architecture
section below). Record every lint id that fires on EITHER run.
**Warning signs:** A `--default-features`-only discovery run reports nothing for the legacy-pair
removal — that is not evidence the removal is non-breaking, it is evidence the feature gate hid
it from that one invocation.

### Pitfall 4: A rewritten `HistoryTrimmer` source string silently breaks existing precedence tests
**What goes wrong:** The three existing tests assert `source.as_str().contains("config")`,
`.contains("provider") || .contains("capabilities")`, `.contains("default")` — a resolver-sourced
`WindowSource` whose Display/mapping doesn't happen to contain these exact substrings fails tests
that look unrelated to the refactor.
**Why it happens:** The tests were written against `LimitSource::as_str()`'s specific wording
(`"model_context_limits config table"`, `"provider capabilities (get_capabilities().max_context_
tokens)"`, `"default_context_tokens"`) — a plausible-looking `WindowSource` rename (e.g.
`"ModelTable"`) breaks the substring match silently at compile time... no, at TEST time, not
compile time, since these are string comparisons.
**How to avoid:** Either keep `LimitSource::as_str()` as a one-line mapping FROM `WindowSource`
that preserves the exact wording (D-05's "kept as a one-line mapping" option), or ensure whatever
string `WindowSource`'s own Display/label produces still contains the three required substrings.
**Warning signs:** `limit_resolution_prefers_the_config_table` /
`..._falls_back_to_provider_capabilities` / `..._falls_back_to_the_default` failing after the
resolver swap, with the resolved `u32` correct but the source-string assertion failing.

## Code Examples

### `is_exact` doc test on the port itself (D-06/D-07's proof requirement)
```rust
// Extends the existing doc test in
// crates/paladin-ports/src/output/token_counter_port.rs (lines 47-66)
use paladin_ports::output::token_counter_port::TokenCounterPort;

struct AlwaysOne;

impl TokenCounterPort for AlwaysOne {
    fn count(&self, _text: &str, _model: &str) -> u32 {
        1
    }
    fn name(&self) -> &str {
        "always-one"
    }
    // No `is_exact` override -- inherits the trait default.
}

let counter = AlwaysOne;
assert_eq!(counter.count("hello, world", "any-model"), 1);
assert!(!counter.is_exact(), "the trait default is false");
```

### Equivalence fixture shape (D-13 — write these FIRST, against pre-refactor code)
```rust
// Distinct, non-round numbers per <specifics>: table 1_234, capability 8_765,
// default 4_321, fallback 2_222, reserve 321 -- a swapped precedence step
// cannot pass by coincidence.
#[test]
fn commissary_prefers_capabilities_over_fallback_before_the_refactor() {
    let capabilities = capabilities_with_window(Some(8_765));
    let config = CommissaryPlan {
        fallback_context_tokens: Some(2_222),
        reserved_completion_tokens: 321,
        ..Default::default()
    };
    let commissary = Commissary::new("deepseek", capabilities, counter(), false, config).unwrap();
    assert_eq!(commissary.allotted_tokens(), 8_765 - 321);
}
```
This exact test, written and committed GREEN against today's `Commissary::new` signature, then
kept byte-identical (modulo the dropped `is_exact_counter` positional argument once PRIM-02
lands) through the refactor commit, IS the equivalence snapshot D-13 asks for — the commit order
(test lands green before the refactor) is the proof, not a diffing tool.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| Caller supplies `is_exact_counter: bool` at `Commissary::new` (guessing which concrete counter it injected) | Port declares its own `is_exact()` | This phase (PRIM-01/02) | One source of truth; a caller cannot mis-declare a counter's exactness |
| Two independent three-step precedence walks (`Commissary::new`'s inline `.or(...)`, `HistoryTrimmer::resolve_limit`) | One shared `paladin_llm::window::resolve_context_window` | This phase (PRIM-04) | A future fourth precedence source (e.g. a per-model override) is added once, not twice |
| Fallible `garrison::TokenCounter` trait + `TokenCounterFactory` (dead code, zero in-tree callers) | `TokenCounterPort` is the only counting contract | This phase (PRIM-03) | Removes ~215 lines (trait + factory + their tests) of unreferenced legacy surface |

**Deprecated/outdated:**
- `garrison::TokenCounter` / `TokenCounterFactory`: superseded entirely by
  `TokenCounterPort` (already the production path since the v0.10.0 agent-runtime-enhancements
  phase per `token_counter_port.rs`'s own module doc) — removed, not deprecated, per ADR-0051's
  clean-break exception for Phases 31-33.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `--release-type minor` produces the same lint ids for `paladin-llm`/`paladin-ports`/`paladin-memory` in THIS phase as Phase 31 saw for its own touched crates (i.e. `is_exact` fires no lint, `Commissary::new`'s signature change fires `inherent_method_missing` or a parameter-count-shaped lint, the legacy pair's removal fires `trait_missing`+`struct_missing`) — CONTEXT.md D-14 itself frames this as "expected", not confirmed | Common Pitfalls, Validation Architecture | Low — D-14 already directs the executor to run discovery empirically and record whatever actually fires; this research's expected-lint list is a hint, not a locked requirement, so a mismatch only means the MIGRATION.md/allowlist rows need different lint ids than guessed, not a redesign |
| A2 | `WindowSource`'s Display/label text is free to differ from `LimitSource::as_str()`'s exact wording as long as the three required substrings (`"config"`, `"provider"`/`"capabilities"`, `"default"`) are preserved somewhere in the logged string | Common Pitfalls (Pitfall 4), Architecture Patterns | Low — worst case, the planner keeps `LimitSource` as a one-line mapping (D-05's explicitly offered alternative), so no test breaks regardless |

**If this table is empty:** N/A — two low-risk assumptions recorded above, both explicitly
anticipated and hedged by CONTEXT.md's own decisions (D-05's mapping alternative, D-14's
"empirical, not guessed" instruction).

## Open Questions (RESOLVED)

1. **Exact `WindowFallbackPolicy`/`WindowSource`/`ResolvedWindow`/`UnknownContextWindow` names**
   - What we know: CONTEXT.md explicitly leaves these to Claude's Discretion; a design sketch is
     given above under Architecture Patterns.
   - What's unclear: Nothing blocking — this is a naming choice, not a behavioral one.
   - Recommendation: Use the sketch's names verbatim unless the planner has a house-style reason
     to diverge; consistency with the sketch keeps this research's Code Examples section directly
     reusable.

2. **Whether `resolve_context_window` takes `&ProviderCapabilities` or a bare `Option<u32>`**
   - What we know: Both compile cleanly against the two current call sites; CONTEXT.md leaves this
     to Claude's Discretion.
   - What's unclear: No functional difference in this phase's scope.
   - Recommendation: `&ProviderCapabilities` — matches what both callers already hold, and keeps
     the resolver's signature stable if a future phase wants to consult another capability field.

## Environment Availability

Skipped — this phase has no external service/tool dependency beyond the already-installed Rust
toolchain and `cargo-semver-checks@0.50.0` (pinned via `taiki-e/install-action@v2` in
`ci.yml`, already verified present in CI; local discovery runs need the same pin, verifiable via
`cargo semver-checks --version`).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` / `#[tokio::test]` (`cargo test`), workspace-standard — no new framework |
| Config file | none — no `pytest.ini`/`jest.config` equivalent; feature flags in each crate's `Cargo.toml` govern which tests compile |
| Quick run command | `cargo test -p paladin-ports --doc` / `cargo test -p paladin-memory --features content-processing -- token_counter` / `cargo test -p paladin-llm -- commissary` / `cargo test -p paladin-ai -- history_trimmer` |
| Full suite command | `cargo test --workspace --all-features --no-fail-fast` (matches Phase 31's own gate command) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PRIM-01 | `is_exact` defaults `false`, tiktoken returns `true`, heuristic returns `false` | unit + doctest | `cargo test -p paladin-ports --doc -- token_counter_port` and `cargo test -p paladin-memory --features content-processing -- is_exact` | ✅ port doctest exists (extend it); ❌ new heuristic/tiktoken `is_exact` unit tests — Wave 0/1 |
| PRIM-02 | `Commissary::new`/`from_port` drop `is_exact_counter`; every call site compiles | unit (compile-gate) | `cargo test -p paladin-llm -- commissary` (10 call sites in this module) and `cargo doc -p paladin-llm` (mdBook fenced block is `rust,ignore`, not compiled — verify by eye) | ✅ existing tests to be rewritten in place |
| PRIM-03 | Legacy `TokenCounter`/`TokenCounterFactory` removed with re-exports narrowed | unit + grep gate | `cargo test -p paladin-memory --features content-processing -- token_counter` plus `grep -rnE '\bTokenCounterFactory\b\|garrison::TokenCounter\b\|is_exact_counter' crates src docs examples benches` returning empty (D-12's exit grep) | ✅ existing tests to be rewritten; ❌ the exit-grep step — Wave 3 |
| PRIM-04 | Shared resolver, precedence tests (4 outcomes), equivalence snapshot | unit | `cargo test -p paladin-llm -- window` (new module) and `cargo test -p paladin-ai -- history_trimmer` / `cargo test -p paladin-llm -- commissary` (equivalence fixtures) | ❌ new `window` module + its tests — Wave 0/1; ✅ `history.rs`'s 3 precedence tests exist as the template |
| PRIM-05 | MIGRATION.md/allowlist rows, CHANGELOG, `make clean-code`, coverage floor | other (release gate) | `cargo semver-checks check-release --package <pkg> --default-features --baseline-version 0.9.0 --release-type minor` (discovery) then the literal CI command (no `--release-type`, verifying exit 0); `make clean-code`; `cargo llvm-cov --workspace --fail-under-lines 82` | Process gate, not a test file |

### Sampling Rate
- **Per task commit:** the quick-run command scoped to the crate touched (`cargo test -p <crate>
  -- <filter>`)
- **Per wave merge:** `cargo test --workspace --all-features --no-fail-fast` plus
  `cargo test --workspace --doc` (doctests are skipped by `--tests`/`llvm-cov` — must be run
  explicitly per the phase brief's own tooling note)
- **Phase gate:** Full suite green, `make clean-code`, coverage floor green, both semver
  discovery-run pairs recorded, before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `crates/paladin-llm/src/window/mod.rs` — new module, no existing test file; needs its own
  `#[cfg(test)] mod tests` with the four precedence outcomes (table beats capability, capability
  beats fallback, lenient default when both absent, strict refusal when both absent and no caller
  fallback)
- [ ] Equivalence fixtures in `commissary.rs`'s existing `#[cfg(test)] mod tests` — written against
  PRE-refactor code per D-13, committed green before the resolver lands
- [ ] `crates/paladin-memory/src/token_counter/heuristic.rs` — add one `is_exact` assertion test
  (small addition to existing file, not a new file)
- [ ] `crates/paladin-memory/src/garrison/token_counter.rs` — add `TiktokenCounter::is_exact ==
  true` test (reuse the existing `tiktoken_counter_implements_the_port`-style pattern at line 342)
- [ ] Framework install: none — `cargo test` is already the workspace's test runner

*(No new test framework or fixture harness needed — every gap is a new test function inside an
existing or newly-created module, not new tooling.)*

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | This phase touches no auth surface |
| V3 Session Management | no | No session surface touched |
| V4 Access Control | no | No access-control surface touched |
| V5 Input Validation | partial | `model: &str` strings flow into `TokenCounterPort::count`/the resolver's config-table lookup — both are already documented infallible-for-any-string (no panic, no error path) per the port's own contract; the resolver's `config_table.get(model)` is an ordinary `HashMap` lookup, safe for any string including empty/adversarial input by construction (no parsing, no allocation proportional to attacker-controlled size beyond the string itself) |
| V6 Cryptography | no | No cryptographic material in this phase |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Panic-as-DoS via an unrecognized model string reaching a tokenizer | Denial of Service | Already mitigated by the EXISTING infallibility contract (`TokenCounterPort::count` never panics/errors — `token_counter_port.rs`'s own documented contract, lines 33-40); this phase does not weaken it — `TiktokenCounter::count` after PRIM-03's inlining must retain the exact same "never re-resolve `model` at count-time" behavior (see the existing rustdoc at `token_counter.rs:132-140`, which this research's Existing Code Insights section quotes) |
| A `#[deprecated]` escape hatch silently reintroducing dead legacy code as a supply-chain-adjacent risk | Tampering (of intent, not data) | D-09 confirms zero in-tree callers exist outside the definition file and the three re-export sites, so PRIM-03's `#[deprecated]` fallback path is not exercised in this phase — verified by this research's own grep, not assumed |

No new threat surface is introduced by this phase — it removes dead code and adds a defaulted,
pure-function trait method and a synchronous precedence lookup, neither of which touches
authentication, session state, or untrusted network input.

## Sources

### Primary (HIGH confidence — direct file reads this session)
- `crates/paladin-ports/src/output/token_counter_port.rs` — full read, current trait shape and doc test
- `crates/paladin-memory/src/token_counter/heuristic.rs` — full read, `HeuristicTokenCounter`
- `crates/paladin-memory/src/garrison/token_counter.rs` — full read, legacy trait/factory/`TiktokenCounter`
- `crates/paladin-llm/src/services/commissary.rs` — full read (1009 lines), `Commissary` struct/new/from_port/tests
- `src/application/services/paladin/middleware/history.rs` — full read (622 lines), `HistoryTrimmer`/`LimitSource`/tests
- `src/config/agent_runtime.rs` (lines 370-400, 690-735) — `HistoryTrimmerConfig`, production `HistoryTrimmer::new` site
- `src/application/services/paladin/middleware/summarization.rs` (lines 150-180) — second production site
- `crates/paladin-ports/src/output/llm_port.rs` (lines 1150-1200) — `ProviderCapabilities`
- `crates/paladin-memory/src/garrison/mod.rs`, `prelude.rs`, `src/infrastructure/adapters/garrison/mod.rs`, `crates/paladin-memory/src/lib.rs` — re-export sites and feature table
- `MIGRATION.md` §9.2 (PaladinResult/LlmRequest/TokenUsage rows) — row-format template
- `.cargo/semver-checks-allowlist.toml` — `[[entry]]` schema and Phase 31 entries
- `.github/workflows/ci.yml` (lines 260-430) — `semver` job and row-level set-equality gate
- `CHANGELOG.md` (`[Unreleased]`, `[0.10.0]` sections) — bullet-style template from Phase 31's ACCT bullets
- `.planning/phases/31-lossless-token-accounting/31-07-SUMMARY.md` — empirical lint-discovery record and gate-evidence format
- `.planning/decisions/0051-token-economy-versioning-x03-supersession.md` — ADR-0051 full read
- `docs/src/architecture/commissary.md` — full read, usage sketch and "Honesty about exactness"
- Grep across `crates/ src/ examples/ benches/ docs/` for `TokenCounterFactory`, `garrison::TokenCounter`, `is_exact_counter`, `Commissary::new(`, `Commissary::from_port(` — confirms zero in-tree production callers of any of these

### Secondary (MEDIUM confidence)
- `.planning/phases/32-unified-token-primitives/32-CONTEXT.md` — user decisions D-01 through D-16, treated as locked per the discuss-phase output contract (not independently re-verified against a PRD read in this session, but the CONTEXT.md's own canonical refs point at the PRD and this research trusts the discuss-phase agent's citation of it)

### Tertiary (LOW confidence)
- None — every claim in this research traces to a direct file read or grep in this session.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies; every pattern reused is already in the tree and read directly
- Architecture: HIGH — every type shape (current and target) is read from source, not inferred
- Pitfalls: HIGH — all four pitfalls are drawn from Phase 31's own documented, empirically-verified findings (same tooling, same repo, one phase prior) or from direct inspection of the existing test assertions this phase must preserve

**Research date:** 2026-09-15
**Valid until:** No external dependency drift risk (no new packages); valid until the next phase
touches any of `TokenCounterPort`, `Commissary`, `HistoryTrimmer`, or the legacy `garrison::
token_counter` module — i.e., effectively for the duration of Phase 32 and into Phase 33's
regression check.
