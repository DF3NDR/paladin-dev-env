# Phase 33: Commissary In-Tree Adoption - Pattern Map

**Mapped:** 2026-09-16
**Files analyzed:** 15 (7 modified Rust, 1 new Rust, 7 doc/register)
**Analogs found:** 15 / 15

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/paladin-memory/src/services/rag_retrieval_service.rs` | service | transform (retrieve → rank → ration → render) | itself (in-place refactor) + `crates/paladin-llm/src/services/commissary.rs` (algorithm it now calls) | exact (self) / role-match (Commissary) |
| `crates/paladin-memory/src/services/mod.rs` | module (re-export barrel) | — | itself, current 4-line `pub use` block | exact |
| `crates/paladin-memory/src/prelude.rs` | module (re-export barrel) | — | itself, `// Services` section | exact |
| `crates/paladin-memory/src/lib.rs` | config (crate narrative doc) | — | itself, existing crate-doc header | exact |
| `crates/paladin-memory/Cargo.toml` | config | — | `crates/paladin-content/Cargo.toml:23,28` (optional `paladin-llm` dep) — closer shape once `default-features=false` swapped in; `crates/paladin-battalion/Cargo.toml:67-69` (dev-dep, wrong tier but has the "no cycle" comment to copy) | role-match |
| `src/application/services/paladin/paladin_execution_service.rs` (call site + error mapping + renderer) | controller/service (facade orchestration) | request-response | itself, `retrieve_context_with_timeout` / `format_retrieved_context` / `with_token_counter` (all already in file) | exact |
| `src/application/services/sanctum/mod.rs` | module (re-export barrel, backward-compat facade) | — | itself, current `pub use paladin_memory::services::{...}` block | exact |
| `crates/paladin-llm/src/services/commissary.rs` (doc-only edit, lines 22-25) | service (doc comment only) | — | itself | exact |
| `tests/integration/rag_commissary_test.rs` (new) | test (integration) | request-response | `tests/integration/rag_integration_tests.rs` (mock `EmbeddingPort`, ~200-255) + `tests/integration/in_memory_sanctum_tests.rs` (ungated registration pattern) | exact (combined) |
| `tests/integration/mod.rs` | config (test registry) | — | itself, line 55 `in_memory_sanctum_tests` registration | exact |
| `MIGRATION.md` §9.2 | doc (register) | — | `paladin-llm \| Commissary` row (line 205, N/A template) and `paladin-memory \| TokenCounter` rows (203-204, Y-with-allowlist template) | exact |
| `.cargo/semver-checks-allowlist.toml` | config (register) | — | existing `[[entry]]` blocks | exact |
| `CHANGELOG.md` | doc (register) | — | `[0.10.0]` Phase 32 Commissary bullet (~99-114), Phase 31 `TokenUsage` bullets (~70-77) | exact |
| `docs/src/architecture/crate-map.md`, `commissary.md`, `configuration.md`, `upgrading.md`/`migration-guide.md` | doc | — | existing mermaid block / existing "Token primitives" subsection (Phase 32) | exact |
| `.project/v0.10.0/09-program-acceptance-audit.md` §11 (new) + `33-CI-EVIDENCE.md` (new) | doc (release evidence) | batch | `.project/v0.10.0/09-program-acceptance-audit.md` §10 (~1291-1400) as template; `.planning/phases/29-program-gates-release/29-CI-EVIDENCE.md` (Local sweep + CI-run tables) as template | exact |

## Pattern Assignments

### `crates/paladin-memory/src/services/rag_retrieval_service.rs` (service, transform)

**Analog:** itself (current file, read in full) + `crates/paladin-llm/src/services/commissary.rs` for the algorithm being adopted.

**Imports pattern** (current lines 14-23 — extend, do not replace):
```rust
use std::collections::HashSet;
use std::sync::Arc;

use paladin_ports::output::embedding_port::EmbeddingPort;
use paladin_ports::output::sanctum_port::{
    SanctumError, SanctumFilter, SanctumPort, SanctumQuery, SanctumSearchResult,
};

// RagConfig and RetrievalTrigger moved to crates/paladin-memory/src/config/rag.rs (Task 6.0)
pub use crate::config::rag::{RagConfig, RetrievalTrigger};

// NEW imports for Commissary adoption:
use paladin_llm::services::commissary::{
    Commissary, CommissaryError, CommissaryPlan, Consignment, ConsignmentItem, ShedItem,
};
use paladin_ports::output::llm_port::ProviderCapabilities;
use paladin_ports::output::token_counter_port::TokenCounterPort;
```

**Constructor / builder pattern to add** (mirrors `RagRetrievalService::new`, current lines 58-68, and the `with_token_counter` builder shape from `paladin_execution_service.rs:849-861` below):
```rust
pub struct RagRetrievalService {
    sanctum: Arc<dyn SanctumPort>,
    embedding: Arc<dyn EmbeddingPort>,
    config: RagConfig,
    token_counter: Arc<dyn TokenCounterPort>, // NEW field, D-07
}

impl RagRetrievalService {
    pub fn new(
        sanctum: Arc<dyn SanctumPort>,
        embedding: Arc<dyn EmbeddingPort>,
        config: RagConfig,
    ) -> Self {
        Self {
            sanctum,
            embedding,
            config,
            token_counter: Arc::new(crate::token_counter::HeuristicTokenCounter), // D-07 default
        }
    }

    pub fn with_token_counter(mut self, counter: Arc<dyn TokenCounterPort>) -> Self {
        self.token_counter = counter;
        self
    }
}
```

**Core pattern — synthetic-Commissary construction + dispense** (new `ration` seam, D-14; verified signatures from RESEARCH.md "Code Examples", sourced from `commissary.rs`):
```rust
fn ration(
    &self,
    ranked: Vec<SanctumSearchResult>,
) -> Result<(Vec<RagRetrievedMemory>, Vec<ShedItem>, u32, u32, bool), RagRetrievalError> {
    let budget_u32 = u32::try_from(self.config.max_tokens)
        .map_err(|_| RagRetrievalError::BudgetTooLarge { max_tokens: self.config.max_tokens })?;

    let capabilities = ProviderCapabilities {
        max_context_tokens: Some(budget_u32),
        ..ProviderCapabilities::default()
    };
    let plan = CommissaryPlan {
        reserved_completion_tokens: 0,
        fallback_context_tokens: None,
        ..CommissaryPlan::default()
    };
    let commissary = Commissary::new("rag", capabilities, self.token_counter.clone(), plan)
        .map_err(RagRetrievalError::Commissary)?;

    let mut consignment = Consignment::new();
    for (rank, result) in ranked.iter().enumerate() {
        consignment.push(ConsignmentItem {
            label: result.entry.memory.id.to_string(), // D-09: id, never content
            body: result.entry.memory.content.clone(),
            priority: u8::try_from(rank).unwrap_or(u8::MAX), // D-08: rank order
        });
    }

    let stockpile = commissary.dispense("", &consignment).map_err(RagRetrievalError::Commissary)?;
    // unpack stockpile.dispensed (zip with `ranked` by label/index) + stockpile.shed
    // + stockpile.prompt_tokens/allotted_tokens/exact_tally into the new result struct.
    todo!("unpack per D-12 shape")
}
```

**Call-site replacement** in `retrieve_context` (current lines 112-118 — the four post-processing steps):
```rust
results = self.filter_by_similarity(results);
results = self.deduplicate_memories(results);
results = self.rank_by_relevance(results);
// OLD: results = self.truncate_to_token_budget(results);
// NEW: replace the whole tail with a call into `ration(results)` and build the D-12 result struct.
```

**Old code being fully removed** (current lines 187-212, the anti-pattern; keep this excerpt only as the "before" reference for the exit-grep/CHANGELOG note, never copy it forward):
```rust
fn truncate_to_token_budget(&self, results: Vec<SanctumSearchResult>) -> Vec<SanctumSearchResult> {
    let mut total_tokens = 0;
    let mut truncated = Vec::new();
    for result in results {
        let estimated_tokens = result.entry.memory.content.len() / 4;
        if total_tokens + estimated_tokens <= self.config.max_tokens {
            total_tokens += estimated_tokens;
            truncated.push(result);
        } else {
            log::debug!("Truncating memories at token budget: {} tokens used of {} max", total_tokens, self.config.max_tokens);
            break; // silent drop — this is the line COMM-03's exit grep proves gone
        }
    }
    truncated
}
```

**Error handling pattern — `#[from]` boundary enum** (house pattern; matches other layer-specific `thiserror` enums in `paladin-memory`, e.g. vault/garrison error types — same shape as `PaladinError`/`BattalionError` in CLAUDE.md's own example):
```rust
#[derive(Debug, thiserror::Error)]
pub enum RagRetrievalError {
    #[error("sanctum error: {0}")]
    Sanctum(#[from] SanctumError),
    #[error("commissary error: {0}")]
    Commissary(#[from] CommissaryError),
    #[error("rag.max_tokens ({max_tokens}) exceeds u32::MAX")]
    BudgetTooLarge { max_tokens: usize },
}
```
Note (Pitfall 2, RESEARCH.md): `ShedItem`/`Stockpile`/`ConsignmentItem`/`DispensedItem` derive only `Debug, Clone` — no `PartialEq`. The new D-12 result struct should derive only `Debug, Clone` to match, and all new tests must assert on individual fields, never `assert_eq!` on the whole struct/`Stockpile`.

**Renderer pattern to extend** — `format_for_prompt` (current lines 226-252): keep the per-memory loop shape, change the source field from `memory.content` to the new struct's post-dispense `body`, and append the D-15 marker as a single trailing line when `shed` is non-empty, sourced from one shared `pub` helper/constant in this same module (not re-typed at the facade — see next entry).

**Test migration note (Pitfall 3):** the five existing tests in `#[cfg(test)]` (`test_successful_retrieval_with_multiple_memories`, `test_filtering_by_min_similarity`, `test_format_for_prompt`, `test_empty_results_graceful_handling`, plus `test_rag_config_*`/`test_retrieval_trigger_*` which are untouched) must be updated in the SAME commit as the signature change — reuse `MockEmbeddingPort`, `MockSanctumPort { results }`, `create_test_entry(paladin_id, content, importance, score)` exactly as they exist today (lines 294-375); `test_format_for_prompt` in particular builds its `memories` vec directly and needs a small local wrapper into the new per-item type with `body: memory.content.clone(), truncated: false`.

**Property test pattern (D-17)** — new, no existing `proptest!` usage in this workspace to copy verbatim (root `Cargo.toml:246` only pins the version); use the CONTEXT.md-supplied strategy sketch directly:
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn ration_respects_budget_and_rank_order(
        items in prop::collection::vec((".{1,600}", 0.0f32..=1.0f32), 0..=20),
        budget in 1u32..=2_000u32,
    ) {
        // build ranked SanctumSearchResults from `items` (shuffled before ranking),
        // call the private `ration` seam directly (D-14 — no async runtime needed),
        // assert: (i) prompt_tokens <= budget, (ii) every shed score <= every retained score,
        // (iii) retained ∪ shed == input by id.
    }
}
```
Use distinct non-round numbers in example-based tests (budget `1_234`, bodies of `987`/`654`/`321` bytes — house style per Phase 31, `<specifics>`).

---

### `crates/paladin-memory/Cargo.toml` (config)

**Analog A — the dev-dependency edge to copy the comment shape from, NOT the dependency tier** (`crates/paladin-battalion/Cargo.toml:67-69`):
```toml
[dev-dependencies]
# No cycle: paladin-llm depends only on core + ports (used here for MockLlmAdapter in tests)
paladin-llm = { version = "0.10.0", path = "../paladin-llm" }
```

**Analog B — the production-but-optional edge, closer in tier** (`crates/paladin-content/Cargo.toml:23,28`):
```toml
[features]
llm = ["dep:paladin-llm"]

[dependencies]
paladin-llm = { version = "0.10.0", path = "../paladin-llm", optional = true }
```

**Pattern to actually write (D-01) — unconditional production dependency, the FIRST of its kind in the workspace; say so plainly, do not call it a repeat of either analog above (RESEARCH.md Pitfall 1):**
```toml
[dependencies]
# First unconditional production lateral adapter->adapter edge in the workspace (Phase 33).
# No cycle: paladin-llm depends only on paladin-core + paladin-ports.
# default-features = false keeps reqwest/rand out; `commissary`/`window` are unconditional modules.
paladin-llm = { version = "0.10.0", path = "../paladin-llm", default-features = false }

[dev-dependencies]
proptest = "1.4"
```

---

### `crates/paladin-memory/src/services/mod.rs` (module barrel)

**Analog:** itself, current block:
```rust
pub mod rag_retrieval_service;
pub use rag_retrieval_service::{
    RagConfig, RagRetrievalService, RetrievalTrigger, retrieve_context_with_timeout,
};
```
Extend the `pub use` list with the new D-12 result struct, the D-13 error enum, and re-export `ShedItem` (from `paladin_llm::services::commissary`) so downstream callers name one path (D-12).

---

### `crates/paladin-memory/src/prelude.rs` (module barrel)

**Analog:** itself, current `// Services` section:
```rust
pub use crate::services::{
    MemoryExtractionService, MemoryExtractionStrategy, RagConfig, RagRetrievalService,
};
```
Add the new result/error type names here too (crate-root convenience re-export — Claude's discretion per CONTEXT.md whether `ShedItem` also lands here).

---

### `src/application/services/paladin/paladin_execution_service.rs` (facade — three edits)

**Analog:** itself; all three sites already exist and are the direct pattern to extend, not replace wholesale.

**1. Call site** (current lines ~1297-1330, the `match self.retrieve_context_with_timeout(...)` block) — keep the `Ok(results) => { ...; format }` / `Err(e) => { warn!; None }` shape; only the type flowing through `results` changes (new D-12 struct instead of `Vec<SanctumSearchResult>`), and `info!` gains the shed count (D-16):
```rust
Ok(results) => {
    _retrieval_latency_ms = retrieval_start.elapsed().as_millis() as u64;
    _memories_retrieved_count = results.len(); // uses new struct's len() accessor (D-12)
    let context = if results.is_empty() {
        String::new()
    } else {
        self.format_retrieved_context(&results)
    };
    info!(
        "RAG retrieval succeeded: execution_id={}, memories={}, shed={}, latency_ms={}",
        execution_id, _memories_retrieved_count, results.shed.len(), _retrieval_latency_ms
    );
    Some(context)
}
```

**2. `retrieve_context_with_timeout`** (current lines 1923-1962) — keep exactly the same `timeout(...).await` / `Ok(Ok(...))` / `Ok(Err(...))` / `Err(_)` structure and the same error mapping (`PaladinError::ExecutionError` / `PaladinError::Timeout(5)`); only the `Ok<...>` payload type and the `rag_service.retrieve_context(...)` return type change:
```rust
async fn retrieve_context_with_timeout(
    &self,
    paladin: &Paladin,
    query: &str,
    execution_id: uuid::Uuid,
) -> Result<paladin_memory::services::RagRetrievalResult, PaladinError> { // new struct name, D-12
    if let Some(ref rag_service) = self.rag_retrieval_service {
        let paladin_id = paladin.uuid.to_string();
        match timeout(Duration::from_secs(5), rag_service.retrieve_context(&paladin_id, query)).await {
            Ok(Ok(results)) => { debug!(...); Ok(results) }
            Ok(Err(e)) => {
                warn!("RAG retrieval failed: execution_id={}, error={}", execution_id, e);
                Err(PaladinError::ExecutionError(format!("RAG retrieval failed: {}", e)))
            }
            Err(_) => { warn!(...); Err(PaladinError::Timeout(5)) }
        }
    } else {
        Err(PaladinError::ConfigurationError("RAG retrieval service not configured".to_string()))
    }
}
```

**3. `format_retrieved_context`** (current lines 1969-1985) — keep the per-result loop shape, source `result.body` instead of `result.entry.memory.content`, and append the D-15 marker (imported from the shared `paladin-memory` helper, never re-typed):
```rust
fn format_retrieved_context(&self, results: &paladin_memory::services::RagRetrievalResult) -> String {
    if results.is_empty() {
        return String::new();
    }
    let mut context = String::new();
    for (i, m) in results.memories.iter().enumerate() {
        context.push_str(&format!("{}. [Score: {:.2}] {}\n", i + 1, m.result.score, m.body));
    }
    if !results.shed.is_empty() {
        context.push_str(&paladin_memory::services::rag_omission_marker(results.shed.len(), /* budget */ 0));
    }
    context
}
```

---

### `src/application/services/sanctum/mod.rs` (facade re-export barrel)

**Analog:** itself, current block:
```rust
pub use paladin_memory::services::{
    ExtractedMemory, MemoryExtractionService, MemoryExtractionStrategy, RagConfig,
    RagRetrievalService, RetrievalTrigger, retrieve_context_with_timeout,
};
```
Extend with the new D-12 result struct name, the D-13 error enum, and `ShedItem` — the same `pub use` list pattern, plus the matching `pub mod rag_retrieval_service { pub use paladin_memory::services::{...}; }` backward-compat sub-module block already present just below it.

---

### `tests/integration/rag_commissary_test.rs` (new integration test)

**Analog 1 — mock EmbeddingPort pattern** (`tests/integration/rag_integration_tests.rs:~200-255`): copy the deterministic mock `EmbeddingPort` struct/impl shape verbatim (same `Embedding { vector, model, dimension, token_count }` construction as `MockEmbeddingPort` in `rag_retrieval_service.rs` itself, lines 294-326).

**Analog 2 — ungated registration** (`tests/integration/mod.rs`, `in_memory_sanctum_tests` at line 55): register `pub mod rag_commissary_test;` beside it, NOT beside `rag_integration_tests` (line 78, `#[cfg(feature = "qdrant")]` gated).

**Analog 3 — store construction** (`crates/paladin-memory/src/sanctum/in_memory_adapter.rs` / `tests/integration/in_memory_sanctum_tests.rs`): `InMemorySanctum::new(max_entries)` constructor pattern.

**Entry path to exercise (must go through the facade, not the crate directly):**
```rust
use paladin::application::services::sanctum::{RagConfig, RagRetrievalService};
```

**Assertions to structure per D-18** (three cases in one file): small budget → `shed` non-empty AND marker present in `format_for_prompt`; large budget → `shed` empty AND no marker; one oversized memory → `truncated == true`, per-item marker present, `shed` empty (the D-10(a) edge, named explicitly per `<specifics>`).

---

### `crates/paladin-llm/src/services/commissary.rs` (doc-only, lines 22-25)

**Analog:** itself — this is a rewrite, not a new pattern. Exact replacement text supplied in CONTEXT.md `<specifics>`:
> "the anti-pattern this module was built to replace — `RagRetrievalService::truncate_to_token_budget`'s silent drop — was retired in v0.10.0 when RAG became this module's first production caller (Phase 33)"

---

## Shared Patterns

### `#[from]` error-boundary enum (house pattern)
**Source:** `PaladinError`/`BattalionError` shape from `/workspace/CLAUDE.md`'s own Code Conventions section; concretely mirrored by any existing `paladin-memory` error enum with `#[from]` variants.
**Apply to:** the new `RagRetrievalError` (D-13) in `rag_retrieval_service.rs`.
```rust
#[derive(Debug, thiserror::Error)]
pub enum RagRetrievalError {
    #[error("sanctum error: {0}")]
    Sanctum(#[from] SanctumError),
    #[error("commissary error: {0}")]
    Commissary(#[from] CommissaryError),
    #[error("rag.max_tokens ({max_tokens}) exceeds u32::MAX")]
    BudgetTooLarge { max_tokens: usize },
}
```

### Builder pattern for optional collaborators
**Source:** `PaladinExecutionService::with_token_counter` (`src/application/services/paladin/paladin_execution_service.rs:849-861`, verified in RESEARCH.md):
```rust
pub fn with_token_counter(mut self, counter: Arc<dyn TokenCounterPort>) -> Self {
    info!("Setting PaladinExecutionService token counter: {}", counter.name());
    self.token_counter = counter;
    self
}
```
**Apply to:** `RagRetrievalService::with_token_counter` (D-07) — copy the exact `mut self -> Self` fluent shape; the `info!` line is optional but consistent with house style.

### Never clamp on integer conversion (ADR-0004)
**Source:** ADR-0004, restated in RESEARCH.md Anti-Patterns / Don't Hand-Roll.
**Apply to:** the D-05 `usize -> u32` budget conversion — `u32::try_from(x).map_err(|_| RagRetrievalError::BudgetTooLarge { max_tokens: x })`, never `as u32` or `.min(u32::MAX as usize) as u32`.

### Sensitive-content-never-in-logs
**Source:** `.github/instructions/security.instructions.md` general principle; concretely, `ConsignmentItem.label`/`ShedItem.label` design in `commissary.rs`.
**Apply to:** D-09 — `ConsignmentItem.label = entry.memory.id.to_string()`, never `memory.content`; the D-16 `info!` line logs counts only (`retained`, `shed`, `prompt_tokens`, `allotted_tokens`, `exact_tally`), never memory text.

### Release register four-way sync
**Source:** `MIGRATION.md` §9.2 ↔ `.cargo/semver-checks-allowlist.toml` ↔ `ci.yml` `semver` job ↔ `CHANGELOG.md` `[0.10.0]`, as already practiced in Phase 29 (`29-CI-EVIDENCE.md`) and Phase 32 (`32-05-SUMMARY.md`).
**Apply to:** every plan touching a public-signature break in this phase (D-22/D-23/D-24) — the row/entry/regenerated-baseline/CHANGELOG-bullet land in the SAME commit as the breaking code change, never split across commits.

## No Analog Found

None. Every file this phase touches has either a direct existing analog (itself, being refactored) or a close cross-crate precedent (`paladin-battalion`/`paladin-content` dependency shapes; Phase 29/32 release-evidence documents).

## Metadata

**Analog search scope:** `crates/paladin-memory/`, `crates/paladin-llm/src/services/commissary.rs`, `crates/paladin-battalion/Cargo.toml`, `crates/paladin-content/Cargo.toml`, `src/application/services/paladin/paladin_execution_service.rs`, `src/application/services/sanctum/mod.rs`, `tests/integration/`, `MIGRATION.md`, `.cargo/semver-checks-allowlist.toml`, `.project/v0.10.0/09-program-acceptance-audit.md`, `.planning/phases/29-program-gates-release/29-CI-EVIDENCE.md`
**Files scanned:** ~18 (all read directly or via targeted grep/sed excerpts; no analog search left unresolved)
**Pattern extraction date:** 2026-09-16
