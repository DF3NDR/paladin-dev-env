---
phase: 33-commissary-in-tree-adoption
plan: 02
subsystem: memory
tags: [rag, commissary, observability, paladin-memory, paladin-llm, sanctum]

# Dependency graph
requires:
  - phase: 33-commissary-in-tree-adoption
    plan: "01"
    provides: "RagRetrievalResult / RagRetainedMemory / RagRetrievalError / ShedItem re-export, the ration seam, RagRetrievalService::with_token_counter"
provides:
  - "rag_omission_marker(omitted, budget_tokens) — the single shared helper both renderers call, re-exported from paladin-memory (services + prelude) and from the facade's sanctum barrel (top-level and rag_retrieval_service sub-module)"
  - "RagRetrievalService::format_for_prompt emits the omission marker whenever result.shed is non-empty"
  - "PaladinExecutionService::format_retrieved_context emits the byte-identical marker from the same helper, reading allotted_tokens off the same RagRetrievalResult"
  - "The facade's RAG-success info! line gains a shed= field (counts only, no memory text)"
affects: [33-03-property-tests, 33-04-integration-evidence, 33-05-release-gates]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "One shared, pub helper in the lower crate (paladin-memory) that BOTH the crate's own renderer and a downstream facade renderer call, so byte-identity between two renderers is a compile-time import, not a doc convention"
    - "Byte-exact string assertions in the facade's negative tests (rather than substring-contains checks) so a file that must never re-type a literal never accidentally re-types it inside a test's assertion string either"

key-files:
  created: []
  modified:
    - crates/paladin-memory/src/services/rag_retrieval_service.rs
    - crates/paladin-memory/src/services/mod.rs
    - crates/paladin-memory/src/prelude.rs
    - src/application/services/paladin/paladin_execution_service.rs
    - src/application/services/sanctum/mod.rs

key-decisions:
  - "D-15: rag_omission_marker lives in paladin-memory's rag_retrieval_service.rs as the ONLY place the omission line's literal text is constructed; both format_for_prompt and the facade's format_retrieved_context import and call it rather than re-typing the string."
  - "D-15 (budget sourcing): both renderers read the token budget from RagRetrievalResult::allotted_tokens (the Stockpile's own accounting), never from a config value, so the facade renderer — which has no RagConfig — produces the byte-identical line."
  - "D-16: the facade's existing 'RAG retrieval succeeded' info! line gains a shed= field sourced from results.shed.len(); no other fields, no memory body, no rendered context, per the awk-scoped negative grep the plan's <verify> runs over that exact statement."

patterns-established:
  - "Pattern: a doc comment on a helper whose exact output string is under a grep-count gate must never repeat that literal string itself — point at the test that demonstrates the shape instead of re-typing an example."

requirements-completed: [COMM-02]

coverage:
  - id: D1
    description: "rag_omission_marker(3, 2000) and rag_omission_marker(1, 1234) render the plural and singular forms respectively; format_for_prompt appends the marker when shed is non-empty and never when it is empty"
    requirement: "COMM-02"
    verification:
      - kind: unit
        ref: "crates/paladin-memory/src/services/rag_retrieval_service.rs#rag_omission_marker_uses_plural_noun_for_multiple_omissions"
        status: pass
      - kind: unit
        ref: "crates/paladin-memory/src/services/rag_retrieval_service.rs#rag_omission_marker_uses_singular_noun_for_one_omission"
        status: pass
      - kind: unit
        ref: "crates/paladin-memory/src/services/rag_retrieval_service.rs#format_for_prompt_ends_with_marker_when_shed_nonempty"
        status: pass
      - kind: unit
        ref: "crates/paladin-memory/src/services/rag_retrieval_service.rs#format_for_prompt_contains_no_marker_when_shed_empty"
        status: pass
    human_judgment: false
  - id: D2
    description: "Named edge tests: an empty retrieval renders the empty string with no shed and no marker; a single memory that fits the budget is retained whole (truncated == false, shed empty) with no marker"
    requirement: "COMM-02"
    verification:
      - kind: unit
        ref: "crates/paladin-memory/src/services/rag_retrieval_service.rs#format_for_prompt_empty_retrieval_has_no_shed_and_no_marker"
        status: pass
      - kind: unit
        ref: "crates/paladin-memory/src/services/rag_retrieval_service.rs#format_for_prompt_single_fitting_memory_has_no_marker"
        status: pass
    human_judgment: false
  - id: D3
    description: "The facade renderer emits the byte-identical marker (via the same shared helper, same allotted_tokens) whenever shed is non-empty, and emits nothing when shed is empty; the RAG-success info! line names the shed count and interpolates no memory body"
    requirement: "COMM-02"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/paladin_execution_service.rs#test_format_retrieved_rag_context_ends_with_shared_omission_marker"
        status: pass
      - kind: unit
        ref: "src/application/services/paladin/paladin_execution_service.rs#test_format_retrieved_rag_context_no_marker_when_shed_empty"
        status: pass
      - kind: unit
        ref: "awk-scoped grep over the RAG-success info! statement for memory.content|.body|format_retrieved_context( -- 0 matches"
        status: pass
    human_judgment: false

duration: ~20min
completed: 2026-09-16
status: complete
---

# Phase 33 Plan 02: RAG Omission Marker and Shed-Count Observability Summary

**One shared `rag_omission_marker` helper in `paladin-memory`, called by both `RagRetrievalService::format_for_prompt` and the facade's `format_retrieved_context`, plus a `shed=` field on the facade's RAG-success log line.**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-09-16T16:36Z (worktree base commit)
- **Completed:** 2026-09-16T16:56Z
- **Tasks:** 2 code tasks (2 total in plan)
- **Files modified:** 5

## Accomplishments
- `rag_omission_marker(omitted: usize, budget_tokens: u32) -> String` added to `crates/paladin-memory/src/services/rag_retrieval_service.rs` as the single place the RAG-level omission line's literal text is constructed (D-15) — distinct from `CommissaryPlan::truncation_marker`, the Commissary's per-item marker that stays inside a cut body (D-10).
- `RagRetrievalService::format_for_prompt` appends the marker after its `---` trailer whenever `result.shed` is non-empty, reading the budget from `result.allotted_tokens` (the Stockpile's own accounting), never `self.config.max_tokens`.
- The marker is re-exported from `paladin-memory`'s `services` module and `prelude`, and from the facade's `sanctum` barrel (both the top-level `pub use` and the backward-compatible `rag_retrieval_service` sub-module), so every caller and test names one path.
- `PaladinExecutionService::format_retrieved_context` calls the same shared helper with the same `RagRetrievalResult` fields, so the two renderers produce byte-identical output for the same result — proven by a test that asserts the facade's rendered tail equals a direct call to the shared helper.
- The facade's "RAG retrieval succeeded" `info!` line gains a `shed=` field sourced from `results.shed.len()`; no other fields were added, and no memory body or rendered context is interpolated (verified by an `awk`-scoped negative grep over the exact statement, matching the plan's `<verify>` command).

## Task Commits

Each task was committed atomically:

1. **Task 1: One shared omission marker, emitted by the paladin-memory renderer** - `027569cf` (feat)
2. **Task 2: The facade renderer emits the same marker and the log line carries the shed count** - `2a9e20f5` (feat)

**Plan metadata:** (this commit, once created)

## Files Created/Modified
- `crates/paladin-memory/src/services/rag_retrieval_service.rs` - `rag_omission_marker` helper, `format_for_prompt` emits it on non-empty `shed`, six new unit tests (plural/singular wording, marker present/absent, two named edge tests)
- `crates/paladin-memory/src/services/mod.rs` - re-exports `rag_omission_marker`
- `crates/paladin-memory/src/prelude.rs` - `// Services` block re-exports `rag_omission_marker`
- `src/application/services/paladin/paladin_execution_service.rs` - `format_retrieved_context` emits the shared marker; the RAG-success `info!` line gains `shed=`; two new unit tests (byte-identity assertion, byte-exact no-marker assertion)
- `src/application/services/sanctum/mod.rs` - facade barrel re-exports `rag_omission_marker` (top-level and `rag_retrieval_service` sub-module)

## Decisions Made
- D-15 (marker construction site and budget source): the literal omission-line text is constructed in exactly one function in `paladin-memory`; both renderers call it and both source the token budget from `RagRetrievalResult::allotted_tokens`, never a config value, so the facade — which holds no `RagConfig` — still produces the byte-identical line.
- D-16 (log line scope): the facade's existing RAG-success `info!` line gains only `shed=`; no `TraceEvent`, no herald field, matching the plan's deferred-idea note.
- Test-naming: the two new facade unit tests were named to include the substring `rag` (`test_format_retrieved_rag_context_...`) so they are picked up by the plan's `<verify>` filter `cargo test -p paladin-ai --lib rag` — the pre-existing `test_format_retrieved_context` (no shed handling) does not match that filter and was left as-is, out of this plan's scope.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Removed a duplicate literal from `rag_omission_marker`'s own doc comment**
- **Found during:** Task 1 acceptance-criteria check
- **Issue:** The rustdoc on `rag_omission_marker` included a worked example (`` `[3 lower-relevance memories omitted to fit the 2000-token RAG budget]` ``) that re-typed the exact literal the function constructs, so `grep -c 'omitted to fit' rag_retrieval_service.rs` returned 2 instead of the required 1 "outside the test module" — the acceptance criterion the plan itself specifies.
- **Fix:** Reworded the doc comment to point at the test demonstrating the exact rendered shape (`rag_omission_marker_uses_plural_noun_for_multiple_omissions`) instead of repeating the string.
- **Files modified:** `crates/paladin-memory/src/services/rag_retrieval_service.rs`
- **Verification:** `awk '/^#\[cfg\(test\)\]/{exit} {print}' ... | grep -c 'omitted to fit'` returns `1`; `cargo test -p paladin-memory --lib rag_retrieval_service` still 15/15 passing.
- **Committed in:** `027569cf` (Task 1 commit)

**2. [Rule 1 - Bug] Rewrote the facade's empty-shed test to avoid a literal the file's own acceptance criterion forbids**
- **Found during:** Task 2 acceptance-criteria check
- **Issue:** The plan's acceptance criteria require `src/application/services/paladin/paladin_execution_service.rs` to "contain no literal `omitted to fit` string" — but the initial test asserted `!formatted.contains("omitted to fit")`, which itself types that exact literal into the file, violating the criterion literally even though the assertion's *intent* (no marker present) was correct.
- **Fix:** Replaced the substring-negative assertion with a byte-exact `assert_eq!(formatted, "1. [Score: 0.90] Kept memory\n")` — a stronger check (proves nothing at all was appended) that never needs to write the marker's wording in this file.
- **Files modified:** `src/application/services/paladin/paladin_execution_service.rs`
- **Verification:** `grep -n "omitted to fit" src/application/services/paladin/paladin_execution_service.rs` returns no matches; `cargo test -p paladin-ai --lib rag` — 7/7 passing.
- **Committed in:** `2a9e20f5` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (2 bugs, both self-caught against the plan's own acceptance criteria before committing)
**Impact on plan:** Both fixes tighten compliance with the plan's literal-count acceptance criteria; no behavior change, no scope creep.

## Issues Encountered
- The plan's `<verify>` filter for Task 2 (`cargo test -p paladin-ai --lib rag`) matches on a name substring, not a topic tag — the two new tests had to include the literal substring `rag` in their names (not just be about RAG) to be selected. Named them `test_format_retrieved_rag_context_...` accordingly. No plan change needed; documented here so a future reader isn't confused by the naming.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Both renderers now emit the identical omission marker from one shared, tested helper, and the facade's RAG-success log line carries the shed count with no memory text — COMM-02's "nothing dropped silently, visible in the rendered prompt" requirement is satisfied at the rendering layer.
- The full workspace stays green: `cargo test -p paladin-memory --lib rag_retrieval_service` (15/15), `cargo test -p paladin-ai --lib rag` (7/7), `cargo clippy --workspace --all-targets -- -D warnings` (clean), `cargo check --workspace --all-features --all-targets` (clean).
- Plan 33-03's property test can exercise the `ration` seam and this plan's marker helper without further crate-graph or public-surface changes.
- No blockers.

---
*Phase: 33-commissary-in-tree-adoption*
*Completed: 2026-09-16*

## Self-Check: PASSED

- `crates/paladin-memory/src/services/rag_retrieval_service.rs` — FOUND, contains `pub fn rag_omission_marker(`
- `crates/paladin-memory/src/services/mod.rs` — FOUND, re-exports `rag_omission_marker`
- `crates/paladin-memory/src/prelude.rs` — FOUND, re-exports `rag_omission_marker`
- `src/application/services/paladin/paladin_execution_service.rs` — FOUND, imports and calls `rag_omission_marker`, contains no literal `omitted to fit` string
- `src/application/services/sanctum/mod.rs` — FOUND, re-exports `rag_omission_marker` in both the top-level barrel and the `rag_retrieval_service` sub-module
- Commit `027569cf` — FOUND in `git log`
- Commit `2a9e20f5` — FOUND in `git log`
- `cargo test -p paladin-memory --lib rag_retrieval_service` — 15 passed, 0 failed
- `cargo test -p paladin-ai --lib rag` — 7 passed, 0 failed
- `cargo clippy --workspace --all-targets -- -D warnings` — exit 0, no warnings
- `cargo check --workspace --all-features --all-targets` — exit 0
