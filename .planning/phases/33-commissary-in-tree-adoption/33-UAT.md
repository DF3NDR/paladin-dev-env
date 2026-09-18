---
status: complete
phase: 33-commissary-in-tree-adoption
source: 33-01-SUMMARY.md, 33-02-SUMMARY.md, 33-03-SUMMARY.md, 33-04-SUMMARY.md, 33-05-SUMMARY.md, 33-06-SUMMARY.md
started: 2026-09-17T01:36:28Z
updated: 2026-09-17T01:46:08Z
---

## Current Test

[testing complete]

## Tests

### 1. Human sign-off on the v0.10.0 re-seal (Phase 29 D-17)
expected: Open .project/v0.10.0/09-program-acceptance-audit.md and read the new "## 11. Re-seal after Phases 30-33 (Phase 33, COMM-04)" section, plus 33-CI-EVIDENCE.md (32-row Local sweep table: 30 PASS, 1 carried cargo-doc condition at 73 warnings, 1 CI-attributed 82% coverage floor). The seven pre-existing Phase 29 sign-off boxes in sections 1-10 are untouched; section 11 adds exactly one new, still-unticked box "The v0.10.0 tag may be cut". The evidence reads as honest and complete (every gate carries its exact command, verdict, head SHA 69500c9b and date; nothing is claimed as a local pass that was not measured locally). Reply "yes" if you accept the evidence, or describe what is missing or wrong.
result: pass
coverage_id: 33-06/D3
rationale: The seven pre-existing sign-off boxes and the new eighth box are judgment-tier per Phase 29 D-17 -- a human, not this audit or any agent, ticks them at UAT before the v0.10.0 tag is cut.

### 2. paladin-memory depends unconditionally on paladin-llm; all three crate-isolation build shapes green; no reqwest in the normal dependency graph
expected: cargo build -p paladin-memory (default / --no-default-features / --all-features) all pass; cargo tree -i reqwest finds no match
result: pass
source: automated
coverage_id: 33-01/D1

### 3. RAG retrieval over budget returns prompt_tokens at or below budget with every dropped memory in shed, retained in descending score order, every id exactly once, shed labels are memory UUIDs
expected: commissary_rations_rag_retrieval_end_to_end passes
result: pass
source: automated
coverage_id: 33-01/D2

### 4. rag.max_tokens larger than u32::MAX returns a typed BudgetTooLarge error (no as-cast, no clamp); workspace compiles and lints clean with all features
expected: grep for as-casts returns 0; cargo check + clippy -D warnings pass
result: pass
source: automated
coverage_id: 33-01/D3

### 5. rag_omission_marker renders plural and singular forms; format_for_prompt appends the marker only when shed is non-empty
expected: four named unit tests in rag_retrieval_service.rs pass
result: pass
source: automated
coverage_id: 33-02/D1

### 6. Edge tests: empty retrieval renders empty string with no shed and no marker; single fitting memory retained whole with no marker
expected: two named edge tests pass
result: pass
source: automated
coverage_id: 33-02/D2

### 7. Facade renderer emits the byte-identical marker via the shared helper; RAG-success info! line names the shed count and interpolates no memory body
expected: two facade unit tests pass; awk-scoped negative grep returns 0
result: pass
source: automated
coverage_id: 33-02/D3

### 8. Property test over the ration seam: retained total never exceeds budget (outside D-10a), no shed memory outscores a retained one, retained union shed equals input by id
expected: ration_respects_budget_and_rank_order proptest passes
result: pass
source: automated
coverage_id: 33-03/D1

### 9. One memory larger than the whole budget is retained truncated with the per-item marker and empty shed, not dropped (D-10a)
expected: single_memory_larger_than_budget_is_retained_truncated_not_dropped passes
result: pass
source: automated
coverage_id: 33-03/D2

### 10. rag.max_tokens beyond u32::MAX returns the typed budget-conversion error and never a wrapped, clamped or zero budget (D-05)
expected: budget_beyond_u32_returns_typed_error_and_never_clamps passes
result: pass
source: automated
coverage_id: 33-03/D3

### 11. Equal scores keep insertion order through the rank sort and receive adjacent, distinct priorities (D-08)
expected: equal_scores_keep_insertion_order_and_distinct_priorities passes
result: pass
source: automated
coverage_id: 33-03/D4

### 12. Budget boundary: a set that fits is retained whole with no shed; one memory past the allowance sheds the lowest-priority item
expected: at_the_budget_boundary_nothing_is_shed_and_one_past_it_sheds_the_lowest passes
result: pass
source: automated
coverage_id: 33-03/D5

### 13. Ungated integration test drives Commissary::dispense through the real RAG path over InMemorySanctum and the facade re-export, no Docker service required
expected: cargo test --test rag_commissary -- 3 passed with no --features flag
result: pass
source: automated
coverage_id: 33-04/D1

### 14. Integration test asserts all three directions: small budget sheds and marks, large budget sheds nothing and no marker, one oversized memory retained truncated
expected: three named tests in tests/integration/rag_commissary_test.rs pass
result: pass
source: automated
coverage_id: 33-04/D2

### 15. No silent token-based truncation remains in-tree; both D-19 exit greps are empty (closes Phase 26 D-13 deferral)
expected: grep for truncate_to_token_budget and .len() / 4 both return 0 matches
result: pass
source: automated
coverage_id: 33-04/D3

### 16. Commissary module doc names RAG as its first production caller and no longer points at a live in-tree anti-pattern
expected: grep -c 'first production caller' commissary.rs == 1; cargo test -p paladin-llm --lib commissary -- 20 passed
result: pass
source: automated
coverage_id: 33-04/D4

### 17. Four user-facing doc pages describe the new behaviour (commissary.md, configuration.md, upgrading.md, migration-guide.md); docs build green
expected: mdbook build docs exits 0 with no broken links; no v0.11.0 string in upgrading/migration-guide
result: pass
source: automated
coverage_id: 33-04/D5

### 18. Six cargo-semver-checks 0.50.0 discovery runs (--release-type minor) confirm zero lints fire for the RagRetrievalService return-type break, a genuine tool coverage gap
expected: six captured run transcripts plus catalog inspection recorded in 33-05-SUMMARY.md Task 1
result: pass
source: automated
coverage_id: 33-05/D1

### 19. MIGRATION.md section 9.2 gains two N/A rows for paladin-memory RagRetrievalService and retrieve_context_with_timeout; check-migration-allowlist and check-gates set-equal; no TBD
expected: make check-migration-allowlist and make check-gates exit 0; grep -c TBD MIGRATION.md == 0
result: pass
source: automated
coverage_id: 33-05/D2

### 20. CHANGELOG.md [0.10.0] gains Behavioral changes / Changed / Added entries for the RAG rationing break; [Unreleased] header folded in; Phase 31 and 32 entries verified present
expected: grep -c '^## [Unreleased]' == 0; awk-scoped greps for rag/TokenUsage/Commissary all non-zero; no v0.11.0 string
result: pass
source: automated
coverage_id: 33-05/D3

### 21. .project/current-exports.txt regenerated in the same commit as the CHANGELOG edit; carries RagRetrievalResult/RagRetrievalError/ShedItem/rag_omission_marker; make api-surface shows zero drift
expected: make api-surface-update (3959 items) then make api-surface -- 'API surface unchanged'
result: pass
source: automated
coverage_id: 33-05/D4

### 22. Every Phase 29 D-24 release gate re-run on head 69500c9b with exact command, verdict, SHA and date recorded in 33-CI-EVIDENCE.md
expected: 33-CI-EVIDENCE.md Local sweep table has 32 rows: 30 unconditional PASS, 1 carried condition, 1 CI-attributed
result: pass
source: automated
coverage_id: 33-06/D1

### 23. Phase 32 PRIM-04 regression check green with no code change: limit_resolution (3 passed) and kept_set_equivalence_snapshot_pre_resolver (1 passed)
expected: both cargo test filters report non-zero passed counts
result: pass
source: automated
coverage_id: 33-06/D2

## Summary

total: 23
passed: 23
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps

[none yet]
