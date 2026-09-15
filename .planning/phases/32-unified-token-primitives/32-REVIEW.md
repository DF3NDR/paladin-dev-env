---
phase: 32-unified-token-primitives
reviewed: 2026-09-15T19:01:41Z
depth: standard
files_reviewed: 18
files_reviewed_list:
  - .cargo/semver-checks-allowlist.toml
  - crates/paladin-llm/src/lib.rs
  - crates/paladin-llm/src/services/commissary.rs
  - crates/paladin-llm/src/window.rs
  - crates/paladin-memory/src/garrison/mod.rs
  - crates/paladin-memory/src/garrison/token_counter.rs
  - crates/paladin-memory/src/lib.rs
  - crates/paladin-memory/src/prelude.rs
  - crates/paladin-memory/src/token_counter/heuristic.rs
  - crates/paladin-ports/src/output/token_counter_port.rs
  - docs/src/api-reference/migration-guide.md
  - docs/src/api-reference/upgrading.md
  - docs/src/architecture/commissary.md
  - docs/src/architecture/crate-map.md
  - docs/src/user-guides/memory-management.md
  - src/application/services/paladin/middleware/history.rs
  - src/infrastructure/adapters/garrison/mod.rs
  - src/lib.rs
findings:
  critical: 0
  warning: 1
  info: 1
  total: 2
status: issues_found
---

# Phase 32: Code Review Report

**Reviewed:** 2026-09-15T19:01:41Z
**Depth:** standard
**Files Reviewed:** 18
**Status:** issues_found

## Summary

Phase 32 does three things: (1) adds `TokenCounterPort::is_exact` as a defaulted trait method
and rewires `Commissary` to read exactness live from the injected counter instead of via a
caller-supplied `is_exact_counter: bool` constructor argument; (2) deletes the legacy fallible
`garrison::TokenCounter` trait and `TokenCounterFactory` struct outright (no deprecation shim),
narrowing `TiktokenCounter` to be the sole `TokenCounterPort` implementor under
`content-processing`; (3) introduces `paladin_llm::window::resolve_context_window` as a single
shared, pure four-step context-window precedence resolver, and rewires both `Commissary::new`
(under `WindowFallbackPolicy::Strict`) and `HistoryTrimmer::resolve_limit` (under
`WindowFallbackPolicy::Default`) to call through it instead of each keeping its own walk.

I traced every changed constructor signature to its call sites: `Commissary::new` and
`Commissary::from_port` have zero production callers outside `commissary.rs` itself (confirmed by
whole-repo grep), so the constructor-arity break is contained. I confirmed no remaining reference
to the deleted `TokenCounter`/`TokenCounterFactory` types or `count_tokens` method exists anywhere
in the tracked source tree outside planning artifacts. I re-derived the arithmetic in both new
"equivalence snapshot" tests (`window_and_allowance_equivalence_snapshot_pre_resolver` in
`commissary.rs` and `kept_set_equivalence_snapshot_pre_resolver` in `history.rs`) by hand against
the documented precedence order and the numbers check out. The new `resolve_context_window`
function is pure, well-documented, and its four precedence branches are each independently
tested. `TokenCounterPort::is_exact`'s default (`false`) is additive and non-breaking, matching
the semver register's empirical claim.

The one concrete defect found is a documentation-accuracy regression: this phase updated one of
two sibling line-number citations in `docs/src/architecture/commissary.md` to account for the
file's line-count growth, but left the other stale, so it now points at the wrong test.

## Warnings

### WR-01: Stale line-number citation in commissary.md now points at the wrong test

**File:** `docs/src/architecture/commissary.md:99`
**Issue:** The prose reads:

```
The `verify_fits` pre-flight guard, mirroring `commissary.rs:911-929`
(`verify_fits_reports_measured_and_allowed_on_overflow`) and the `CommissaryError` variants at
`commissary.rs:230-292`:
```

This `911-929` citation was accurate against the pre-phase file (verified against
`c9d757dc289480ebf80877f3ddd699888488a282`, where `verify_fits_reports_measured_and_allowed_on_overflow`
does sit at lines 911-929). This phase's edits to `Commissary::new`/`from_port` (dropping
`is_exact_counter`, adding the `resolve_context_window` call, the `MockCounter`/`exact_counter`
test helpers, and the new `window_and_allowance_equivalence_snapshot_pre_resolver` test) add a net
+25 lines above that point in `commissary.rs`, so `verify_fits_reports_measured_and_allowed_on_overflow`
now actually sits at **lines 936-953**. Lines 911-929 in the current file instead cover the tail of
`per_item_min_greater_than_max_is_rejected_at_construction` and the whole of
`fixed_material_over_the_allowance_errors_instead_of_clamping` — unrelated tests.

Notably, the sibling citation two lines above this one in the same file (the `commissary()` test
helper / `an_over_budget_consignment_sheds_the_lowest_priority_item_first` reference at line 63)
*was* updated in this same commit, from `631-686` to `644-698`, to account for this exact line
growth — so the drift was known and partially fixed, but this second citation was missed. A reader
following the doc to find `verify_fits_reports_measured_and_allowed_on_overflow` lands on the
wrong test.

(Separately, the updated `644-698` citation is itself imprecise — it starts mid-`MockCounter`-impl
and ends mid-`an_over_budget_consignment_sheds_the_lowest_priority_item_first`, before that test's
closing assertions at line 715. This pre-dates the phase, at slightly different bounds, so it is
noted as IN-01 rather than folded into this WARNING.)

**Fix:**
```diff
-The `verify_fits` pre-flight guard, mirroring `commissary.rs:911-929`
+The `verify_fits` pre-flight guard, mirroring `commissary.rs:936-953`
 (`verify_fits_reports_measured_and_allowed_on_overflow`) and the `CommissaryError` variants at
 `commissary.rs:230-292`:
```

## Info

### IN-01: Truncated line-number citation for the shed-priority test example

**File:** `docs/src/architecture/commissary.md:63`
**Issue:** The citation `commissary.rs:644-698` for the "Constructing a `Commissary` and
dispensing a consignment" example covers `fn commissary(...)` through the middle of
`an_over_budget_consignment_sheds_the_lowest_priority_item_first` (which actually runs
694-715 in the current file, ending with `assert_eq!(stockpile.dispensed[0].label, "high-priority");`
at line 714). The citation was correctly shifted by this phase's edits (from `631-686`) but the
upper bound still doesn't reach the end of the named test. Low-impact — a reader can still locate
the right function by name — but a tighter/looser bound would avoid the appearance of the citation
having been checked line-by-line against the current file.
**Fix:** Widen the upper bound to `commissary.rs:646-715` (or similar) to fully enclose both the
`commissary()` helper and the complete `an_over_budget_consignment_sheds_the_lowest_priority_item_first`
test body.

---

_Reviewed: 2026-09-15T19:01:41Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
