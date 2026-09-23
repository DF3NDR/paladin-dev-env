---
phase: 33-commissary-in-tree-adoption
reviewed: 2026-09-16T00:00:00Z
depth: standard
files_reviewed: 18
files_reviewed_list:
  - crates/paladin-memory/src/services/rag_retrieval_service.rs
  - crates/paladin-memory/src/services/mod.rs
  - crates/paladin-memory/src/prelude.rs
  - crates/paladin-memory/src/lib.rs
  - crates/paladin-memory/Cargo.toml
  - crates/paladin-llm/src/services/commissary.rs
  - src/application/services/paladin/paladin_execution_service.rs
  - src/application/services/sanctum/mod.rs
  - tests/integration/rag_commissary_test.rs
  - tests/integration/rag_integration_tests.rs
  - tests/integration/mod.rs
  - docs/src/architecture/commissary.md
  - docs/src/architecture/crate-map.md
  - docs/src/getting-started/configuration.md
  - docs/src/api-reference/upgrading.md
  - docs/src/api-reference/migration-guide.md
  - .project/current-exports.txt
  - .project/v0.10.0/09-program-acceptance-audit.md
findings:
  critical: 0
  warning: 3
  info: 1
  total: 4
status: issues_found
---

# Phase 33: Code Review Report

**Reviewed:** 2026-09-16
**Depth:** standard
**Files Reviewed:** 18
**Status:** issues_found

## Summary

Phase 33 re-routes RAG truncation through `Commissary::dispense`, adding `RagRetrievalResult` /
`RagRetainedMemory` / `RagRetrievalError`, a sync `ration` seam, a shared `rag_omission_marker`
helper, a `with_token_counter` builder, and a new unconditional `paladin-memory → paladin-llm`
production dependency. I read every changed hunk in each listed file (via
`git diff 48a69b06c2b06e3e71fd46a1bef545d0b3e63135..HEAD -- <file>`), traced the call chain from
`RagRetrievalService::ration` through `Commissary::dispense` and back into both renderers
(`RagRetrievalService::format_for_prompt` and `PaladinExecutionService::format_retrieved_context`),
and independently verified the two hard security/correctness constraints named in the review brief:

- **`usize → u32` budget conversion never clamps.** `u32::try_from(self.config.max_tokens)` is
  mapped to a typed `RagRetrievalError::BudgetTooLarge` on failure — confirmed both by reading
  `ration()` and by the dedicated test `budget_beyond_u32_returns_typed_error_and_never_clamps`
  (`usize::MAX` input, asserts `Err`, never a reduced/zero `Ok`).
- **No `reqwest`/`rand` leak into `paladin-memory`.** Verified empirically, not just by reading the
  `default-features = false` declaration: `cargo tree -p paladin-memory --no-default-features`
  shows no `reqwest`/`rand` node anywhere in the graph, and `cargo check -p paladin-memory
  --no-default-features` / `cargo clippy -p paladin-memory --no-default-features --all-targets -- -D
  warnings` both pass clean at this HEAD.
- **`ShedItem.label` and every new/changed log line carry ids/counts only.** `ShedItem.label` is
  `result.entry.memory.id.to_string()` (a UUID), never memory content. Every log line touched or
  added by this phase (`RAG retrieval succeeded: ... memories={}, shed={}, latency_ms={}`, `RAG
  rationing: retained={}, shed={}, prompt_tokens={}, allotted_tokens={}, exact_tally={}`) logs only
  counts/tallies — no memory body ever reaches a log line in the changed hunks. (The one pre-existing
  `log::debug!("Retrieved {} memories for paladin {} with query: {}", ...)` line that logs the raw
  user query is untouched by this diff and outside this review's scope per `diff_base`.)

No blockers found. Three warnings and one info item below are genuine gaps worth closing, none of
which compromise the two named security invariants.

## Warnings

### WR-01: `paladin-memory`'s own `prelude.rs` omits the new `ShedItem` type

**File:** `crates/paladin-memory/src/prelude.rs:24-27`
**Issue:** `RagRetrievalResult::shed` is `Vec<ShedItem>` (a new part of this crate's public surface,
re-exported from `paladin_llm::services::commissary::ShedItem` at `services::mod.rs:13` and at the
facade `src/application/services/sanctum/mod.rs`). `prelude.rs`'s own re-export list adds
`RagRetainedMemory`, `RagRetrievalError`, `RagRetrievalResult`, and `rag_omission_marker`, but not
`ShedItem`:
```rust
pub use crate::services::{
    MemoryExtractionService, MemoryExtractionStrategy, RagConfig, RagRetainedMemory,
    RagRetrievalError, RagRetrievalResult, RagRetrievalService, rag_omission_marker,
};
```
A caller who does `use paladin_memory::prelude::*;` — the crate's documented "import everything at
once" entry point (`prelude.rs:1-3`) — gets `RagRetrievalResult` but has no name in scope for the
type of its own `shed` field, and must additionally reach for `paladin_memory::services::ShedItem`
or the facade re-export. This is the one asymmetry in an otherwise-consistent re-export set (the
facade at `src/application/services/sanctum/mod.rs` DOES re-export `ShedItem` alongside the other
four new names).
**Fix:**
```rust
pub use crate::services::{
    MemoryExtractionService, MemoryExtractionStrategy, RagConfig, RagRetainedMemory,
    RagRetrievalError, RagRetrievalResult, RagRetrievalService, ShedItem, rag_omission_marker,
};
```

### WR-02: `ration()` assumes memory ids are unique within one search result set with no dedup guard

**File:** `crates/paladin-memory/src/services/rag_retrieval_service.rs:250-260`
**Issue:**
```rust
let mut index_by_label: HashMap<String, usize> = HashMap::with_capacity(ranked.len());
for (rank, result) in ranked.iter().enumerate() {
    let label = result.entry.memory.id.to_string();
    index_by_label.insert(label.clone(), rank);
    consignment.push(ConsignmentItem { label, body: ..., priority: ... });
}
```
If `ranked` ever contained two entries sharing the same memory UUID (e.g. a future Sanctum adapter
bug, or a `SanctumPort::search` implementation that legitimately returns the same stored id twice
across different index shards), `index_by_label.insert` silently overwrites the earlier rank with
the later one, and `Consignment` ends up holding two `ConsignmentItem`s with an *identical* label.
`Commissary::dispense` has no concept of label uniqueness, so it would treat them as independent
items; when results come back, `RagRetrievalError::UnmatchedDispensedLabel` would NOT fire (the
label still resolves — just to the wrong, most-recently-inserted rank), and the reconstructed
`RagRetainedMemory::result` could silently point at the wrong `SanctumSearchResult`. Worse, if one
of the two same-labelled items is shed and the other retained, `RagRetrievalResult` would carry the
same id in both `memories` and `shed`, which every test in this file (unit, integration, and the
proptest's property (iii): `output_ids == input_ids` with a `HashSet`) implicitly assumes cannot
happen and would silently pass over via `HashSet` collapsing rather than detecting the collision.
This is a latent, currently-unexercised edge (today's `InMemorySanctum`/`QdrantSanctumAdapter` are
assumed to return unique ids per call), but nothing in `ration()` documents or asserts that
assumption, and no existing test constructs a duplicate-id fixture to prove the failure mode is
actually unreachable.
**Fix:** Either document the invariant explicitly at the `ration()` call site ("assumes
`SanctumPort::search` never returns duplicate memory ids in one call") or defend it structurally,
e.g. dedup `ranked` by id before building the consignment, or return a new
`RagRetrievalError::DuplicateMemoryId { label }` if `index_by_label.insert` would overwrite an
existing key.

### WR-03: `RagRetrievalResult::prompt_tokens` can exceed `allotted_tokens` in the documented D-10(a)
single-truncated-survivor edge, but the field's own doc comment doesn't say so

**File:** `crates/paladin-memory/src/services/rag_retrieval_service.rs:58-66`
**Issue:** The doc comment on `RagRetrievalResult::prompt_tokens` reads only "The final measured
token tally of the rendered context," and `allotted_tokens`'s comment reads "The token allowance
this retrieval was dispensed against" — both read as an implicit "prompt_tokens fits within
allotted_tokens" contract. This module's own tests know that contract is NOT universal: the property
test's doc comment (lines 913-927) and its code (lines 961-969) explicitly carve out and skip the
`prompt_tokens <= budget` assertion for exactly the case where one memory survives and had to be
truncated, because `Commissary::dispense`'s `n == 1` branch returns without re-checking the byte
budget, and a truncated body's length is `min(body.len(), share) + marker.len()` — which can exceed
`share` (and thus the byte allowance) once the marker is added back on. With the default
`HeuristicTokenCounter` (chars/4) this rarely manifests because its ratio is more generous than the
byte-planning ratio (358 tokens/1000 bytes), but a caller who injects a stricter
`with_token_counter` (e.g. a byte-oriented or exact BPE counter close to 1:1) has no such margin, and
nothing in the public struct's documentation warns that `prompt_tokens` is a "usually-bounded, not
strictly-bounded" figure for that one retained-item edge.
**Fix:** Add a doc-comment caveat on `RagRetrievalResult::prompt_tokens` (and/or `allotted_tokens`)
naming the single-truncated-survivor edge, e.g.: "May exceed `allotted_tokens` by up to
`truncation_marker.len()` bytes' worth of tokens in the rare case where exactly one oversized memory
survives rationing — see `Commissary::dispense`'s own `n == 1` note." This keeps the type's public
contract honest without requiring a behavior change to `Commissary` itself (out of this phase's
scope).

## Info

### IN-01: `ration()`'s empty-input short-circuit reports `allotted_tokens: 0` / `prompt_tokens: 0` regardless of the configured budget, and this is logged unconditionally

**File:** `crates/paladin-memory/src/services/rag_retrieval_service.rs:239-241`, `330-345`
**Issue:**
```rust
fn ration(&self, ranked: Vec<SanctumSearchResult>) -> Result<RagRetrievalResult, RagRetrievalError> {
    if ranked.is_empty() {
        return Ok(RagRetrievalResult::default());   // allotted_tokens: 0, prompt_tokens: 0
    }
    ...
```
When no candidate memories survive filtering/dedup (a normal, non-error outcome — e.g. nothing in
Sanctum met `min_similarity`), `RagRetrievalResult::default()` reports `allotted_tokens: 0` even
though `rag.max_tokens` might be configured to, say, 2000. `retrieve_context`'s trailing
`log::info!("RAG rationing: retained={}, shed={}, prompt_tokens={}, allotted_tokens={}, ...")` fires
unconditionally on this path too, so an operator reading logs for a "why did RAG return nothing"
investigation sees `allotted_tokens=0`, which reads as "the budget was configured to zero" rather
than "no candidates reached rationing" — a minor but avoidable source of confusion during
on-call debugging.
**Fix:** Either special-case the log line to omit `allotted_tokens`/`prompt_tokens` when
`ranked.is_empty()`, or populate `RagRetrievalResult::allotted_tokens` from the (u32-converted)
configured budget even on the empty short-circuit, so the field always reflects "what budget this
retrieval ran under" rather than "0 unless something was actually dispensed."

---

_Reviewed: 2026-09-16_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
