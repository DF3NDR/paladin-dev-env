---
status: complete
phase: 32-unified-token-primitives
source: 32-01-SUMMARY.md, 32-02-SUMMARY.md, 32-03-SUMMARY.md, 32-04-SUMMARY.md, 32-05-SUMMARY.md
started: 2026-09-16T13:37:08Z
updated: 2026-09-16T14:33:03Z
---

## Current Test

[testing complete]

## Tests

### 1. TokenCounterPort::is_exact defaults to false (32-01 D1)
expected: TokenCounterPort::is_exact defaults to false, proven by a doc test on the trait itself (bare AlwaysOne impl asserting the inherited answer).
result: pass
source: automated
coverage_id: 32-01/D1

### 2. TiktokenCounter is exact; HeuristicTokenCounter inherits false (32-01 D2)
expected: TiktokenCounter::is_exact() returns true unconditionally, scoped to the encoding resolved at new(model); HeuristicTokenCounter inherits the false default with no override written.
result: pass
source: automated
coverage_id: 32-01/D2

### 3. Commissary constructors drop the caller-supplied exactness flag (32-01 D3)
expected: Commissary::new and Commissary::from_port drop the caller-supplied exactness argument; the struct holds no exactness field; Stockpile.exact_tally is read live from self.counter.is_exact(); Debug prints counter.name() and the live is_exact() answer; all ten in-tree construction sites migrated; no forwarding constructor, deprecated alias, or compatibility shim added.
result: pass
source: automated
coverage_id: 32-01/D3

### 4. Commissary architecture docs rewritten for the four-argument constructor (32-01 D4)
expected: docs/src/architecture/commissary.md's usage sketch and 'Honesty about exactness' section rewritten against the new four-argument constructor and the port-sourced exactness signal; the mirrored line-range reference updated to the real post-task-1 location.
result: pass
source: automated
coverage_id: 32-01/D4

### 5. Six pre-resolver equivalence fixtures committed green (32-02 D1)
expected: Six pre-resolver equivalence fixtures (three Commissary window/allowance cases; three HistoryTrimmer kept-set cases) committed green against the two still-unreplaced inline precedence walks, in a commit touching no production code.
result: pass
source: automated
coverage_id: 32-02/D1

### 6. paladin_llm::window module created with the shared resolver (32-02 D2)
expected: paladin_llm::window created: resolve_context_window (four-step precedence, pure, synchronous), WindowFallbackPolicy (Default/Strict, no bool flag), WindowSource (four variants, exhaustive as_str label, ALL array), ResolvedWindow, UnknownContextWindow, all fully rustdoc'd.
result: pass
source: automated
coverage_id: 32-02/D2

### 7. Four named precedence tests individually selectable (32-02 D3)
expected: Four named precedence tests (config-table precedence, capability precedence, lenient default, strict refusal) plus equal-values, absent-vs-empty, purity, caller-fallback-resolution and source-label-invariant tests; nine total.
result: pass
source: automated
coverage_id: 32-02/D3

### 8. window module registered and re-exported from the facade (32-02 D4)
expected: window module registered as a top-level pub mod window in paladin-llm's ungated module block and re-exported from the facade src/lib.rs in the same pub use paladin_llm:: block style as the Commissary types.
result: pass
source: automated
coverage_id: 32-02/D4

### 9. make clean-code green on the resolver commit; fixtures unchanged (32-02 D5)
expected: make clean-code green (fmt, clippy -D warnings, shellcheck, cargo check) on the full resolver commit; the six equivalence fixtures still pass unmodified.
result: pass
source: automated
coverage_id: 32-02/D5

### 10. Legacy TokenCounter trait and TokenCounterFactory deleted outright (32-03 D1)
expected: Legacy TokenCounter trait, its impl for TiktokenCounter, and TokenCounterFactory (for_model/supported_models/is_supported) deleted outright from token_counter.rs, no #[deprecated] retention.
result: pass
source: automated
coverage_id: 32-03/D1

### 11. TiktokenCounter's only counting path is its TokenCounterPort impl (32-03 D2)
expected: count inlines the BPE lookup and per-string cache directly, no fallible count_tokens survives; is_exact/model_name/new/clear_cache/cache_size all present.
result: pass
source: automated
coverage_id: 32-03/D2

### 12. All four re-export sites narrowed to TiktokenCounter only (32-03 D3)
expected: paladin-memory garrison/mod.rs, prelude.rs, facade garrison/mod.rs top-level and its token_counter sub-module narrowed to TiktokenCounter only, each content-processing cfg gate intact.
result: pass
source: automated
coverage_id: 32-03/D3

### 13. Doc sweep names only TiktokenCounter/TokenCounterPort (32-03 D4)
expected: paladin-memory lib.rs narrative/feature table, crate-map.md feature table, memory-management.md token-counting comment all name only TiktokenCounter/TokenCounterPort, not the removed pair; mdbook build clean.
result: pass
source: automated
coverage_id: 32-03/D4

### 14. Counting behaviour unchanged after the removal (32-03 D5)
expected: Cache hit/clear/empty-string/unicode/multi-model coverage preserved in the rewritten port-facing tests (120 passed, 0 failed).
result: pass
source: automated
coverage_id: 32-03/D5

### 15. Commissary::new resolves its window via exactly one resolve_context_window call under Strict (32-04 D1)
expected: Commissary::new calls resolve_context_window once under WindowFallbackPolicy::Strict; the resolved count is stored on a resolved_window field and returned by the private window() accessor without re-walking the precedence order.
result: pass
source: automated
coverage_id: 32-04/D1

### 16. UnknownContextWindow maps to the byte-identical UndeclaredContextWindow variant (32-04 D2)
expected: The resolver's UnknownContextWindow error is mapped inside Commissary::new into the pre-existing CommissaryError::UndeclaredContextWindow variant with Display text and field byte-identical to the pre-plan definition, naming the provider.
result: pass
source: automated
coverage_id: 32-04/D2

### 17. HistoryTrimmer::resolve_limit is a thin call-through; LimitSource deleted (32-04 D3)
expected: HistoryTrimmer::resolve_limit calls resolve_context_window under WindowFallbackPolicy::Default(config.default_context_tokens) with self.config.model_context_limits as the config table; the facade's local LimitSource enum and its label accessor are deleted workspace-wide in favour of the shared WindowSource.
result: pass
source: automated
coverage_id: 32-04/D3

### 18. Six D-13 equivalence fixtures pass unedited after rewiring (32-04 D4)
expected: The six equivalence fixtures from plan 32-02 pass unedited after both consumers are rewired, and no unwrap/expect/panic was introduced in either file outside its #[cfg(test)] module.
result: pass
source: automated
coverage_id: 32-04/D4

### 19. Six empirical cargo-semver-checks discovery runs (32-05 D1)
expected: Six discovery runs (four --default-features, two --features content-processing) at tool version 0.50.0; paladin-memory's content-processing run fires struct_missing (TokenCounterFactory) and trait_missing (TokenCounter); all other five fire nothing, with the Commissary constructor break confirmed as a genuine tool coverage gap.
result: pass
source: automated
coverage_id: 32-05/D1

### 20. MIGRATION.md 9.2 rows and allowlist entries set-equal (32-05 D2)
expected: MIGRATION.md gains two new section 9.2 rows (paladin-memory | TokenCounter, paladin-memory | TokenCounterFactory) with matching .cargo/semver-checks-allowlist.toml entries in the same commit; the CI row-level set-equality step exits 0 in both directions.
result: pass
source: automated
coverage_id: 32-05/D2

### 21. CHANGELOG and reader-facing docs updated for v0.10.0 (32-05 D3)
expected: CHANGELOG.md [0.10.0] gains one Changed, one Removed and one Added bullet; upgrading.md and migration-guide.md gain a Token primitives section linking to MIGRATION.md section 9.2; every new version string reads v0.10.0.
result: pass
source: automated
coverage_id: 32-05/D3

### 22. PRIM-05 phase gate evidence — confirm the cargo doc RED gate matches the Phase 31 precedent (32-05 D4)
expected: 32-05-SUMMARY.md records the full PRIM-05 gate list: make clean-code GREEN; workspace tests 6771 passed / 1 known pre-existing cli_isolation failure; explicit doctests GREEN (137 + 147); mdbook build GREEN; make security GREEN; coverage 90.25% against the 82% floor; final exit grep empty. One gate is honestly RED: RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps fails at paladin-ai-core with 14 unresolved-intra-doc-link errors in the graph-fingerprinting and webhook-delivery doc families, none referencing a Phase 32 symbol. Phase 31's 31-07-SUMMARY.md recorded this identical command RED (77 warnings, same two families) and left it unfixed as out of scope. Confirm this RED is the accepted Phase 31 precedent, not a Phase 32 regression, and that the phase may advance with it carried forward.
result: pass
coverage_id: 32-05/D4
rationale: The cargo doc gate is honestly RED (pre-existing, unrelated); a human should confirm this matches the accepted Phase 31 precedent rather than being auto-classified as a full pass.

## Summary

total: 22
passed: 22
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps

[none yet]
