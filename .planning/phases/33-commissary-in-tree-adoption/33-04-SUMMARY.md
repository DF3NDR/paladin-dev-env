---
phase: 33-commissary-in-tree-adoption
plan: 04
subsystem: memory
tags: [rag, commissary, integration-test, docs, mdbook, paladin-memory, paladin-llm]

# Dependency graph
requires:
  - phase: 33-commissary-in-tree-adoption
    plan: "01"
    provides: "RagRetrievalResult / RagRetainedMemory / RagRetrievalError / ShedItem re-export, the ration seam, RagRetrievalService::with_token_counter, the facade re-export path"
  - phase: 33-commissary-in-tree-adoption
    plan: "02"
    provides: "rag_omission_marker(omitted, budget_tokens), format_for_prompt emitting it on non-empty shed"
provides:
  - "tests/integration/rag_commissary_test.rs -- the ungated F4 integration-test evidence (COMM-03) driving a real Commissary::dispense through the published paladin::application::services::sanctum facade path over InMemorySanctum"
  - "the `[[test]] name = \"rag_commissary\"` Cargo.toml target with no required-features, running in CI's integration-tests job with no Docker service"
  - "both D-19 exit greps empty (truncate_to_token_budget nowhere under crates/src/docs/examples/benches; no `.len() / 4` estimate under crates/paladin-memory/src) -- closes the Phase 26 D-13 deferral"
  - "the Commissary module doc naming RAG as its first production caller in the past tense, with no live in-tree anti-pattern reference"
  - "docs/src/architecture/commissary.md's 'In-tree caller: RAG' section, mirroring the integration test's usage shape"
  - "RAG marker/shed-record language in configuration.md's rag.max_tokens YAML comment and four-meanings table row"
  - "RAG lines under 'Token primitives' in upgrading.md and migration-guide.md, pointing at the MIGRATION.md §9.2 rows plan 33-05 writes"
affects: [33-05-release-gates, 33-06-audit]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A deterministic (non-random) mock EmbeddingPort keyed on a content-prefix match, so an integration test that goes through a real InMemorySanctum cosine-similarity search still produces reproducible, distinct scores -- contrasted with rag_integration_tests.rs's `qdrant`-gated MockEmbeddingPort, which uses `rand` because that suite only needs storage to succeed, not a specific rank order"
    - "Importing test fixtures ONLY through a facade re-export module (paladin::application::services::sanctum) rather than the crate-internal path, so the integration test proves the whole published surface, not just the paladin-memory internals"

key-files:
  created:
    - tests/integration/rag_commissary_test.rs
  modified:
    - tests/integration/mod.rs
    - Cargo.toml
    - crates/paladin-llm/src/services/commissary.rs
    - docs/src/architecture/commissary.md
    - docs/src/getting-started/configuration.md
    - docs/src/api-reference/upgrading.md
    - docs/src/api-reference/migration-guide.md

key-decisions:
  - "D-18: the integration test imports ONLY through paladin::application::services::sanctum (RagConfig, RagRetrievalResult, RagRetrievalService, ShedItem, rag_omission_marker), never the crate-internal paladin_memory::services path, so it is proof of the published surface."
  - "Case (a)'s budget (233 tokens) reuses the exact figure wave 1's end-to-end unit test proved forces a shed under the same 987/654/321-byte body sizes at the Commissary's default pessimistic ratio -- the CONTEXT sketch's property-test figure (budget 1,234) would NOT force a shed at these sizes and was deliberately not used here, matching wave 1's own documented correction."
  - "Deliberate deviation from CONTEXT <specifics> (recorded in Task 2's commit and here): the wording sketch for commissary.rs's module-doc rewrite spells out the retired helper's old function identifier (truncate_to_token_budget). Using it verbatim would keep the D-19 exit grep permanently non-empty. D-19's grep is the locked requirement; the rewritten sentence names the behaviour (\"the RAG retrieval service's own inline byte-length budget estimate\") and the crate instead, never the identifier."

patterns-established:
  - "Pattern: when a doc/module rewrite's own exit grep target is the very identifier the wording sketch would naturally use, name the behaviour and file location instead of the identifier -- the grep is the locked contract, the wording sketch is a suggestion."

requirements-completed: [COMM-03]

coverage:
  - id: D1
    description: "An ungated integration test drives Commissary::dispense through the real RAG path over InMemorySanctum and the facade re-export, running in CI's integration-tests job with no Docker service (F4 production-caller evidence)"
    requirement: "COMM-03"
    verification:
      - kind: integration
        ref: "cargo test --test rag_commissary -- 3 passed, 0 failed, no --features flag"
        status: pass
      - kind: integration
        ref: "cargo test --test lib rag_commissary -- 3 passed via tests/lib.rs's integration:: module tree"
        status: pass
    human_judgment: false
  - id: D2
    description: "The integration test asserts all three directions: small budget sheds and marks, large budget sheds nothing and emits no marker, one oversized memory is retained truncated (not shed)"
    requirement: "COMM-03"
    verification:
      - kind: integration
        ref: "tests/integration/rag_commissary_test.rs#small_budget_sheds_and_marks"
        status: pass
      - kind: integration
        ref: "tests/integration/rag_commissary_test.rs#large_budget_sheds_nothing_and_emits_no_marker"
        status: pass
      - kind: integration
        ref: "tests/integration/rag_commissary_test.rs#single_oversized_memory_is_retained_truncated_not_shed"
        status: pass
    human_judgment: false
  - id: D3
    description: "No silent token-based truncation remains in-tree: both D-19 exit greps (the retired identifier under crates/src/docs/examples/benches; the byte-length-4 estimate under crates/paladin-memory/src) are empty -- closes the Phase 26 D-13 deferral"
    requirement: "COMM-03"
    verification:
      - kind: unit
        ref: "grep -rn 'truncate_to_token_budget' crates src docs examples benches -- 0 matches"
        status: pass
      - kind: unit
        ref: "grep -rnE '\\.len\\(\\) */ *4' crates/paladin-memory/src -- 0 matches"
        status: pass
    human_judgment: false
  - id: D4
    description: "The Commissary module doc names RAG as its first production caller in the past tense and no longer points at a live in-tree anti-pattern"
    requirement: "COMM-03"
    verification:
      - kind: unit
        ref: "grep -c 'first production caller' crates/paladin-llm/src/services/commissary.rs -- 1"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-llm --lib commissary -- 20 passed, 0 failed"
        status: pass
    human_judgment: false
  - id: D5
    description: "The four user-facing doc pages describe the new behaviour: commissary.md's In-tree caller: RAG section, configuration.md's marker/shed mention, and RAG lines under Token primitives in upgrading.md / migration-guide.md, with the docs build staying green"
    requirement: "COMM-03"
    verification:
      - kind: unit
        ref: "mdbook build docs -- exit 0, \"No broken links found\""
        status: pass
      - kind: unit
        ref: "grep -c 'v0.11.0' docs/src/api-reference/upgrading.md docs/src/api-reference/migration-guide.md -- 0 for both"
        status: pass
    human_judgment: false

duration: ~40min
completed: 2026-09-16
status: complete
---

# Phase 33 Plan 04: Ungated F4 Integration Test and Anti-Pattern Retirement Summary

**An ungated `tests/integration/rag_commissary_test.rs` proves a real `Commissary::dispense` through the published RAG facade path over `InMemorySanctum`, and both D-19 exit greps go empty, closing the Phase 26 D-13 silent-truncation deferral.**

## Performance

- **Duration:** ~40 min
- **Started:** 2026-09-16T17:10Z (approx, worktree base commit)
- **Completed:** 2026-09-16T17:18Z
- **Tasks:** 2 code tasks (2 total in plan)
- **Files modified:** 8 (1 created, 7 modified)

## Accomplishments
- `tests/integration/rag_commissary_test.rs` is a new, ungated, service-free integration test importing ONLY through `paladin::application::services::sanctum` (the published facade re-export path), proving `RagRetrievalService::retrieve_context` rations a real `InMemorySanctum` search result through `Commissary::dispense` -- the F4 "production caller exercised by integration tests" evidence for COMM-03.
- Three named cases with distinct, non-round body sizes (987/654/321 bytes) and a deterministic (non-random) mock `EmbeddingPort`: `small_budget_sheds_and_marks` (a tight budget sheds two of three memories and `format_for_prompt` ends with the exact shared omission marker), `large_budget_sheds_nothing_and_emits_no_marker` (a comfortably large budget sheds nothing, truncates nothing, emits no marker), and `single_oversized_memory_is_retained_truncated_not_shed` (the D-10(a) edge at integration level: one memory far larger than the budget is retained truncated with the Commissary's per-item marker, never shed).
- Registered ungated in `tests/integration/mod.rs` (beside `in_memory_sanctum_tests`, not the `qdrant`-gated `rag_integration_tests`) and added as a `[[test]] name = "rag_commissary"` Cargo.toml target with no `required-features` -- confirmed running with zero `--features` flags via both `cargo test --test rag_commissary` and `cargo test --test lib rag_commissary`.
- Both D-19 exit greps are empty: `grep -rn 'truncate_to_token_budget' crates src docs examples benches` and `grep -rnE '\.len\(\) */ *4' crates/paladin-memory/src` produce no output -- the Phase 26 D-13 deferral closes.
- The Commissary module doc (`commissary.rs`) is rewritten to name the retired anti-pattern's behaviour and location in the past tense ("the RAG retrieval service's own inline byte-length budget estimate ... was retired in v0.10.0 when RAG became this module's first production caller (Phase 33)") rather than the retired identifier by name, which would have kept the exit grep permanently non-empty.
- `docs/src/architecture/commissary.md` gains an "In-tree caller: RAG" section whose code sketch mirrors the integration test (synthetic `ProviderCapabilities`, zero-reservation `CommissaryPlan`, one `ConsignmentItem` per ranked memory with UUID label and rank-order priority, `Stockpile` unpacked into the result), and states the two behavioural consequences (oversized memory retained truncated; injected volume planned at the pessimistic ratio).
- `docs/src/getting-started/configuration.md`'s `rag.max_tokens` YAML comment and four-meanings table row gain a sentence naming the marker and shed record; `docs/src/api-reference/upgrading.md` and `migration-guide.md` each gain a RAG line under "Token primitives" pointing at the `paladin-memory | RagRetrievalService` / `| retrieve_context_with_timeout` `MIGRATION.md` §9.2 rows plan 33-05 writes. Written "v0.10.0" throughout, never "v0.11.0".

## Task Commits

Each task was committed atomically:

1. **Task 1: The ungated F4 integration test through the real RAG path** - `0c3845bf` (test)
2. **Task 2: Retire the anti-pattern in prose and prove it with the exit greps** - `2c6d56f2` (docs)

**Plan metadata:** (this commit, once created)

## Files Created/Modified
- `tests/integration/rag_commissary_test.rs` - new ungated integration test (3 cases), deterministic mock `EmbeddingPort`, imports only via the facade re-export path
- `tests/integration/mod.rs` - registers `pub mod rag_commissary_test;` ungated, alphabetically before the `qdrant`-gated `rag_integration_tests`
- `Cargo.toml` - adds `[[test]] name = "rag_commissary"` target, no `required-features`
- `crates/paladin-llm/src/services/commissary.rs` - module-doc anti-pattern sentence rewritten in past tense, naming behaviour and crate rather than the retired identifier
- `docs/src/architecture/commissary.md` - new "In-tree caller: RAG" section after "Usage sketch"
- `docs/src/getting-started/configuration.md` - `rag.max_tokens` YAML comment and four-meanings table row extended with the marker/shed-record sentence
- `docs/src/api-reference/upgrading.md` - RAG line added under "Token primitives"
- `docs/src/api-reference/migration-guide.md` - RAG line added under "Token primitives"

## Decisions Made
- D-18 (import discipline): the integration test imports `RagConfig`, `RagRetrievalResult`, `RagRetrievalService`, `ShedItem`, and `rag_omission_marker` exclusively through `paladin::application::services::sanctum`, never the crate-internal `paladin_memory::services` path -- proving the whole published path, not just the internal implementation.
- Case (a)'s budget of 233 tokens reuses wave 1's own proven figure for the identical 987/654/321-byte body sizes, rather than the CONTEXT `<specifics>` sketch's property-test figure (1,234 tokens), which does not force a shed at these sizes -- consistent with wave 1's own documented correction (33-01-SUMMARY.md).
- Deliberate, plan-directed deviation from CONTEXT `<specifics>`: the module-doc rewrite names the retired behaviour and its crate location rather than the old function identifier (`truncate_to_token_budget`) the wording sketch spells out, because using the identifier verbatim would keep the D-19 exit grep permanently non-empty. D-19's grep is the locked requirement; ADR-0049 is left untouched as history.

## Deviations from Plan

None beyond the plan's own directed deviation from the CONTEXT wording sketch (documented above and in the Task 2 commit message, per the plan's explicit instruction to record it). No Rule 1-4 auto-fixes were needed -- both tasks passed their acceptance criteria and `<verify>` commands on the first implementation.

## Issues Encountered
- The fresh worktree's `docs/` directory was again missing the gitignored, generated `mermaid.min.js`/`mermaid-init.js` assets (`docs/book.toml`'s `additional-js`) that `mdbook build` requires -- the same one-time, idempotent regeneration step wave 1 documented (`mdbook-mermaid install docs`), not a plan deviation.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- COMM-03 is satisfied: a real production caller of `Commissary::dispense` is exercised by an ungated integration test that runs without services, and no silent token-based truncation remains in-tree by the two locked greps.
- Plan 33-05 can write the `paladin-memory | RagRetrievalService` and `| retrieve_context_with_timeout` `MIGRATION.md` §9.2 rows this plan's doc lines already point at, plus the CHANGELOG `[0.10.0]` behavioral-changes bullet and the API-surface baseline regeneration.
- The full local sweep stays green: `cargo test --test rag_commissary` (3/3), `cargo test --test lib rag_commissary` (3/3), `cargo test -p paladin-llm --lib commissary` (20/20), `cargo clippy -p paladin-ai -p paladin-llm --all-targets -- -D warnings` (clean), `mdbook build docs` (0 broken links).
- No blockers.

## Threat Flags

None -- this plan adds a test file and doc-only edits; no new network endpoints, auth paths, or schema changes were introduced. The threat register's T-33-01 (shed-label information disclosure) is mitigated as specified: the new test asserts shed labels parse as UUIDs rather than printing memory bodies, and all fixture content is synthetic (repeated-character strings). T-33-11 (a doc page overstating enforcement) is mitigated by `docs/src/architecture/commissary.md`'s new section, which explicitly states the injected volume is planned at the Commissary's pessimistic ratio, alongside `configuration.md`'s marker/shed-record sentence.

---
*Phase: 33-commissary-in-tree-adoption*
*Completed: 2026-09-16*

## Self-Check: PASSED

- `tests/integration/rag_commissary_test.rs` — FOUND, contains `application::services::sanctum`, `InMemorySanctum`, and all three test function names
- `tests/integration/mod.rs` — FOUND, contains `pub mod rag_commissary_test;` with no `#[cfg(` attribute on the preceding line
- `Cargo.toml` — FOUND, contains a `[[test]]` block with `name = "rag_commissary"` and no `required-features` line in that block
- `crates/paladin-llm/src/services/commissary.rs` — FOUND, module doc contains `first production caller` and `v0.10.0`, no occurrence of `truncate_to_token_budget`
- `docs/src/architecture/commissary.md` — FOUND, contains `## In-tree caller: RAG` naming `ConsignmentItem`, `priority`, and `Stockpile`
- `docs/src/getting-started/configuration.md` — FOUND, `rag.max_tokens` row mentions the marker and the shed record
- `docs/src/api-reference/upgrading.md` and `docs/src/api-reference/migration-guide.md` — FOUND, both contain a RAG line under "Token primitives"; `grep -c 'v0.11.0'` is 0 for both
- Commit `0c3845bf` — FOUND in `git log`
- Commit `2c6d56f2` — FOUND in `git log`
- `cargo test --test rag_commissary` — 3 passed, 0 failed, no `--features` flag
- `cargo test --test lib rag_commissary` — 3 passed, 0 failed
- `cargo test -p paladin-llm --lib commissary` — 20 passed, 0 failed
- `cargo clippy -p paladin-ai -p paladin-llm --all-targets -- -D warnings` — exit 0, no warnings
- `mdbook build docs` — exit 0, "No broken links found"
- Both D-19 exit greps (`truncate_to_token_budget` under crates/src/docs/examples/benches; `.len() / 4` under crates/paladin-memory/src) — empty
