---
phase: 33-commissary-in-tree-adoption
plan: 01
subsystem: memory
tags: [rag, commissary, token-budget, paladin-memory, paladin-llm, sanctum]

# Dependency graph
requires:
  - phase: 32-unified-token-primitives
    provides: "TokenCounterPort::is_exact, Commissary::new without is_exact_counter, paladin_llm::window::resolve_context_window"
provides:
  - "paladin-memory -> paladin-llm production crate edge (default-features = false)"
  - "RagRetrievalResult / RagRetainedMemory / RagRetrievalError types carrying the Commissary shed record"
  - "RagRetrievalService::ration — the sync seam that builds a Consignment from ranked memories and calls Commissary::dispense"
  - "RagRetrievalService::with_token_counter builder mirroring PaladinExecutionService's"
  - "Facade migration: retrieve_context_with_timeout / format_retrieved_context now carry RagRetrievalResult"
affects: [33-02-observability-marker, 33-03-property-tests, 33-04-integration-evidence, 33-05-release-gates]

# Tech tracking
tech-stack:
  added: ["paladin-llm (production dependency of paladin-memory)", "proptest 1.4 (paladin-memory dev-dependency, unused by this plan)"]
  patterns:
    - "Per-call Commissary construction over synthetic ProviderCapabilities for a non-LLM budget (RAG injection cap)"
    - "Rank-order priority derivation (u8::try_from(rank)) so 'highest score retained' is structural, not a rounding property"
    - "Label discipline: Consignment/Shed labels are the memory UUID, never content"

key-files:
  created: []
  modified:
    - crates/paladin-memory/Cargo.toml
    - crates/paladin-memory/src/lib.rs
    - crates/paladin-memory/src/services/rag_retrieval_service.rs
    - crates/paladin-memory/src/services/mod.rs
    - crates/paladin-memory/src/prelude.rs
    - docs/src/architecture/crate-map.md
    - src/application/services/paladin/paladin_execution_service.rs
    - src/application/services/sanctum/mod.rs
    - tests/integration/rag_integration_tests.rs

key-decisions:
  - "D-01: paladin-memory takes paladin-llm as an unconditional production dependency (default-features = false) — the workspace's first such lateral adapter-to-adapter edge."
  - "D-12: retrieve_context/format_for_prompt break their published return type with no forwarding shim (ADR-0051 clean break), auto-selected under auto-chain at the plan's checkpoint:decision."
  - "D-08/D-09: ConsignmentItem priority is rank order (u8::try_from(rank)); label is the memory UUID, never content."
  - "Deviation: the plan's suggested end-to-end unit-test figures (budget 1,234; bodies 987/654/321 bytes) do not force shedding under the Commissary's default pessimistic ratio (358 tokens/1000 bytes yields ~3,446 allowance bytes, comfortably fitting the 1,962-byte sum) — the budget was changed to 233 tokens, which measurably forces a shed, while keeping the specified body sizes unchanged."

patterns-established:
  - "Pattern: a caller with only an injection cap (no LLM provider) constructs a Commissary via synthetic ProviderCapabilities{ max_context_tokens: Some(budget) } rather than from_port."

requirements-completed: [COMM-01]

coverage:
  - id: D1
    description: "paladin-memory depends unconditionally on paladin-llm (default-features = false, no feature list); all three crate-isolation build shapes stay green and no reqwest enters the normal dependency graph"
    requirement: "COMM-01"
    verification:
      - kind: unit
        ref: "cargo build -p paladin-memory / --no-default-features / --all-features"
        status: pass
      - kind: unit
        ref: "cargo tree -p paladin-memory -e normal -i reqwest (exits non-zero, no match)"
        status: pass
    human_judgment: false
  - id: D2
    description: "A RAG retrieval whose memories exceed rag.max_tokens returns a result whose prompt_tokens is at or below budget, with every dropped memory recorded in shed; retained memories come back in descending relevance-score order; every input id appears exactly once across memories and shed; shed labels are memory UUIDs"
    requirement: "COMM-01"
    verification:
      - kind: unit
        ref: "crates/paladin-memory/src/services/rag_retrieval_service.rs#commissary_rations_rag_retrieval_end_to_end"
        status: pass
    human_judgment: false
  - id: D3
    description: "rag.max_tokens larger than u32::MAX returns a typed BudgetTooLarge error (no as-cast, no clamp) — the whole workspace still compiles and lints clean with all features"
    requirement: "COMM-01"
    verification:
      - kind: unit
        ref: "grep -cE 'as u32|u32::MAX as usize' crates/paladin-memory/src/services/rag_retrieval_service.rs == 0"
        status: pass
      - kind: unit
        ref: "cargo check --workspace --all-features --all-targets && cargo clippy --workspace --all-targets -- -D warnings"
        status: pass
    human_judgment: false

duration: ~35min
completed: 2026-09-16
status: complete
---

# Phase 33 Plan 01: Commissary In-Tree Adoption — Crate Edge and Tracer Summary

**Opened the paladin-memory -> paladin-llm production crate edge and wired RagRetrievalService's retrieve_context to ration through Commissary::dispense, deleting the old silent byte-length truncation helper.**

## Performance

- **Duration:** ~35 min
- **Started:** 2026-09-16T15:49Z (approx, per STATE.md session marker)
- **Completed:** 2026-09-16T16:23Z
- **Tasks:** 2 code tasks + 1 auto-resolved checkpoint (3 total in plan)
- **Files modified:** 9

## Accomplishments
- `crates/paladin-memory/Cargo.toml` gains the workspace's first unconditional production lateral adapter->adapter dependency on `paladin-llm` (`default-features = false`), proven green across all three `crate-isolation` build shapes with no `reqwest` entering the dependency graph.
- `RagRetrievalService::retrieve_context` now rations retrieved memories through a per-call `Commissary::dispense` call over a rank-derived `Consignment`, returning a new `RagRetrievalResult` (retained memories, `shed: Vec<ShedItem>`, `prompt_tokens`/`allotted_tokens`/`exact_tally`) and a new `RagRetrievalError` enum — the old inline `content.len() / 4` truncation helper and its silent drop are deleted outright.
- The facade (`PaladinExecutionService::retrieve_context_with_timeout` / `format_retrieved_context`) and the `qdrant`-gated integration test suite are migrated in the same commit so the workspace stays green at every commit.

## Task Commits

1. **Task 1: Open the crate edge — paladin-memory depends on paladin-llm** - `dbd8c378` (build)
2. **Task 2: checkpoint:decision (auto-resolved: proceed)** - no code, resolved per auto-chain mode
3. **Task 3: Tracer — one retrieval rationed end to end** - `574ee36a` (feat)

**Plan metadata:** (this commit, once created)

## Files Created/Modified
- `crates/paladin-memory/Cargo.toml` - adds `paladin-llm` (default-features = false) production dependency and `proptest = "1.4"` dev-dependency
- `crates/paladin-memory/src/lib.rs` - crate narrative names the new dependency and reason
- `docs/src/architecture/crate-map.md` - `mem --> llm` mermaid edge plus prose section update
- `crates/paladin-memory/src/services/rag_retrieval_service.rs` - `RagRetrievalResult`, `RagRetainedMemory`, `RagRetrievalError`, the private `ration` seam, `commissary` helper, `with_token_counter`; deletes `truncate_to_token_budget`
- `crates/paladin-memory/src/services/mod.rs` - re-exports the new types plus `ShedItem`
- `crates/paladin-memory/src/prelude.rs` - prelude re-exports extended
- `src/application/services/paladin/paladin_execution_service.rs` - `retrieve_context_with_timeout`/`format_retrieved_context` migrated to `RagRetrievalResult`; two unit tests updated with a `wrap_rag_result` helper
- `src/application/services/sanctum/mod.rs` - facade re-exports extended (top-level and backward-compat sub-module)
- `tests/integration/rag_integration_tests.rs` - the `qdrant`-gated content assertion migrated to the new result type

## Decisions Made
- D-01/D-03 (crate seam): the `Commissary::dispense` call lives inside `RagRetrievalService` via a new unconditional production `paladin-llm` dependency (`default-features = false`), documented as the workspace's first such edge in the crate-map, crate narrative, and commit message.
- D-04/D-06: the Commissary is constructed per call over synthetic `ProviderCapabilities { max_context_tokens: Some(budget) }` with provider label `"rag"` — no new `Commissary` constructor, no change to `paladin-llm`'s public surface.
- D-05: `rag.max_tokens` (`usize`) converts to `u32` via `u32::try_from`, mapping overflow to a typed `BudgetTooLarge` error — never `as`, never clamped.
- D-08/D-09: `ConsignmentItem.priority` is rank order (`u8::try_from(rank)`); `label` is the memory UUID (`entry.memory.id.to_string()`), never memory content.
- D-12 (checkpoint, Task 2): the plan's `checkpoint:decision` asked whether to break `retrieve_context`'s published return type with no forwarding shim. Because this run is an unattended `--auto` chain (`workflow._auto_chain_active=true`), the first option — **proceed with the clean break per ADR-0051 / D-12** — was auto-selected per the orchestrator's auto-mode checkpoint protocol. Logged here: `⚡ Auto-selected: proceed`.
- D-14: the rationing step is a private synchronous seam (`fn ration`) so a future property test can drive it with no async runtime.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Corrected the end-to-end unit test's budget figure so it actually forces a shed**
- **Found during:** Task 3, item (11) — the new end-to-end unit test proving the tracer
- **Issue:** The plan's suggested figures for this test ("budget `1_234`; bodies of `987`/`654`/`321` bytes") do not force any shedding under the Commissary's default pessimistic byte-planning ratio (358 tokens per 1000 bytes): a 1,234-token budget resolves to an allowance of ~3,446 bytes, comfortably fitting the 1,962-byte sum of all three bodies. Written as specified, the test's own `assert!(!result.shed.is_empty(), ...)` would fail — the figures appear to have been carried over from the CONTEXT.md's *property-test* strategy sketch (`prop::collection::vec(..., 0..=20)` plus `budget in 1u32..=2_000`), not verified against this specific fixed-example test.
- **Fix:** Kept the specified, distinct non-round body sizes (987/654/321 bytes, still guarding against a swapped-priority coincidence) but changed `max_tokens` to `233` — hand-traced through the exact `Commissary::dispense` algorithm and confirmed by running the test, which sheds two of the three memories and retains the highest-scoring one (truncated), satisfying every required assertion (shed non-empty, UUID labels, `prompt_tokens <= allotted_tokens`, descending-score order, no id lost or duplicated).
- **Files modified:** `crates/paladin-memory/src/services/rag_retrieval_service.rs`
- **Verification:** `cargo test -p paladin-memory --lib rag_retrieval_service` — 9/9 passed, including `commissary_rations_rag_retrieval_end_to_end`.
- **Committed in:** `574ee36a` (Task 3 commit)

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** The fix corrects a test that would have failed its own assertion as literally specified; no scope creep, no behavior change to `RagRetrievalService` or `Commissary`.

## Issues Encountered
- The fresh worktree's `docs/` directory was missing the gitignored, generated `mermaid.min.js`/`mermaid-init.js` assets `mdbook build` requires (`docs/book.toml`'s `additional-js`). Ran `mdbook-mermaid install docs` once (a documented, idempotent regeneration step, not a plan deviation) before `mdbook build docs` passed with "No broken links found."
- `cargo fmt --all` reordered/reformatted imports and line-wraps in the files touched by both tasks; no semantic change, re-verified with a second test run after formatting.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- The crate edge and the rationed `retrieve_context` path are in place and green (`cargo check --workspace --all-features --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `mdbook build docs` all pass).
- Plan 33-02 can proceed to add the RAG-level omission marker and observability line (COMM-02) on top of `RagRetrievalResult`/`ShedItem` without further crate-graph changes.
- Plan 33-03's property test can drive the new private `ration` seam directly (sync, no runtime needed) exactly as D-14 anticipates.
- No blockers.

## TDD Gate Compliance

This plan's `type="tracer" tdd="true"` task did not follow a literal RED-then-GREEN commit pair (no standalone failing `test(...)` commit before the `feat(...)` commit) — the task's own `<action>` explicitly required "One atomic commit — the return type is published and the facade consumes it, so a partial commit leaves the workspace red." The new end-to-end unit test, the five migrated tests, and the facade's migrated tests were all written and verified passing within the single `feat(33)` commit (`574ee36a`), consistent with the plan's explicit atomic-commit instruction, which supersedes the general RED/GREEN gate for this task.

---
*Phase: 33-commissary-in-tree-adoption*
*Completed: 2026-09-16*
