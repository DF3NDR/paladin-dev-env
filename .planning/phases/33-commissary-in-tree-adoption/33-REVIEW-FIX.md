---
phase: 33-commissary-in-tree-adoption
fixed_at: 2026-09-16T20:34:17Z
review_path: .planning/phases/33-commissary-in-tree-adoption/33-REVIEW.md
iteration: 1
findings_in_scope: 3
fixed: 3
skipped: 1
status: all_fixed
---

# Phase 33: Code Review Fix Report

**Fixed at:** 2026-09-16T20:34:17Z
**Source review:** .planning/phases/33-commissary-in-tree-adoption/33-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope (critical_warning): 3 (WR-01, WR-02, WR-03)
- Fixed: 3
- Skipped: 1 (IN-01 — out of scope per `fix_scope: critical_warning`, not attempted)

All work was performed in an isolated git worktree (`gsd-reviewfix/33-812734`, based on
`feature/phase-33`), per the fixer's transactional worktree protocol. The worktree's commits
were fast-forwarded onto `feature/phase-33` and the worktree/branch/sentinel were cleaned up
before this report was written; `feature/phase-33` now contains all three fix commits directly.

## Fixed Issues

### WR-01: `paladin-memory`'s own `prelude.rs` omits the new `ShedItem` type

**Files modified:** `crates/paladin-memory/src/prelude.rs`
**Commit:** `eda96bbd`
**Applied fix:** Added `ShedItem` to the `crate::services` re-export list in `prelude.rs`,
alongside `RagRetainedMemory`, `RagRetrievalError`, `RagRetrievalResult`, and
`rag_omission_marker`, preserving rustfmt's import ordering. `ShedItem` was already publicly
reachable via `paladin_memory::services::ShedItem` (re-exported in `services/mod.rs`), so this
closes the one asymmetry the reviewer named: a caller doing `use paladin_memory::prelude::*;`
now has `ShedItem` in scope for `RagRetrievalResult::shed`'s element type, matching the facade's
existing re-export set.
**Verification:** Tier 1 (re-read) + Tier 2 (`cargo fmt --package paladin-memory --check` clean;
`cargo check -p paladin-memory` clean). `make api-surface` afterward reported "API surface
unchanged" (the item was already publicly reachable by another path, so `cargo-public-api` does
not count the new re-export path as drift) — no `api-surface-update` or CHANGELOG entry was
required.

### WR-02: `ration()` assumes memory ids are unique within one search result set with no dedup guard

**Files modified:** `crates/paladin-memory/src/services/rag_retrieval_service.rs`
**Commit:** `3ed8b9d2`
**Applied fix:** Took the reviewer's structural-defense option. Added a new
`RagRetrievalError::DuplicateMemoryId { label }` variant and changed the `index_by_label.insert`
call in `ration()` to check `.is_some()` (i.e., "was there already a value at this key?") and
return `Err(DuplicateMemoryId { label })` instead of silently overwriting the earlier rank.
`RagRetrievalError` is not `#[non_exhaustive]`, so this is an additive public-surface change
(new enum variant); `make api-surface` did not flag it as drift (see below), so no
`api-surface-update` / CHANGELOG step was triggered.

Followed TDD per the repo's working agreement: wrote
`duplicate_memory_id_in_ranked_returns_typed_error_not_silent_overwrite` first — two
`SanctumSearchResult`s built via the existing `create_test_entry` helper, with the second
entry's `memory.id` overwritten to collide with the first's (constructing the exact "a
`SanctumPort::search` implementation that returns the same stored id twice" scenario the
reviewer named). Ran the test against the pre-fix code (guard temporarily reverted) and
confirmed **red**: the assertion failed with `Ok(RagRetrievalResult { memories: [...both
memories carrying result.entry.memory.id == the SAME uuid...], ... })` — i.e., the exact silent
misattribution bug described in the finding (both retained items' `result` field pointed at
`ranked[1]`, the later, wrong `SanctumSearchResult`). Reapplied the guard and confirmed
**green**.
**Verification:** Tier 1 (re-read) + Tier 2 (`cargo fmt --package paladin-memory --check`
clean; `cargo clippy -p paladin-memory --all-targets -- -D warnings` clean; `cargo clippy -p
paladin-memory --no-default-features --all-targets -- -D warnings` clean) + explicit red→green
TDD cycle described above. `cargo test -p paladin-memory --lib rag_retrieval_service` — 21
passed, 0 failed (includes the new test and the pre-existing proptest/end-to-end tests, all
still green). This is a structural/type-level fix (new error variant, explicit guard), not a
judgment-call logic change, so it is marked plain `fixed`, not `requires human verification`.

### WR-03: `RagRetrievalResult::prompt_tokens` can exceed `allotted_tokens` in the D-10(a) edge, undocumented

**Files modified:** `crates/paladin-memory/src/services/rag_retrieval_service.rs`
**Commit:** `6cd77888`
**Applied fix:** Doc-comment-only change, no behavior change to `Commissary` or `ration()`.
Added a caveat paragraph to `RagRetrievalResult::prompt_tokens`'s doc comment naming the
D-10(a) single-truncated-survivor edge, the mechanism (`Commissary::dispense`'s `n == 1` branch
returns without re-checking the byte budget), the bound
(`allotted_tokens` + up to `truncation_marker.len()` bytes' worth of tokens), and when it
manifests (rare with the default `HeuristicTokenCounter`, no margin with a stricter injected
counter). Added a shorter cross-reference caveat to `allotted_tokens`'s own doc comment pointing
back to `prompt_tokens`'s.
**Verification:** Tier 1 (re-read) + Tier 2 (`cargo fmt --package paladin-memory --check`
clean) + doctest run (`cargo test -p paladin-memory --doc rag_retrieval_service` — 1 passed,
confirming the intra-doc links `[Commissary::dispense]` / `[RagRetrievalResult::prompt_tokens]`
resolve without warnings under clippy).

## Skipped Issues

### IN-01: `ration()`'s empty-input short-circuit reports `allotted_tokens: 0` regardless of configured budget

**File:** `crates/paladin-memory/src/services/rag_retrieval_service.rs:239-241, 330-345`
**Reason:** Out of scope. `fix_scope` for this run is `critical_warning`, which covers only
`CR-*`/`BL-*`/`WR-*` findings; `IN-01` is an Info-severity finding and was not attempted. No
code was read or modified for this finding.
**Original issue:** When no candidate memories survive filtering/dedup,
`RagRetrievalResult::default()` reports `allotted_tokens: 0` even if `rag.max_tokens` is
configured higher, and the trailing `log::info!` line fires unconditionally on this path,
which can read as "budget was configured to zero" during on-call debugging rather than "no
candidates reached rationing."

## Verification Command Output (tail)

All commands run against the phase-33 worktree at HEAD `6cd77888` (now fast-forwarded onto
`feature/phase-33`):

```
$ cargo test -p paladin-memory
test result: ok. 120 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.65s
(Note: this invocation runs lib unit tests only, not doctests -- consistent with the
project's known "doctests escape gates" behavior. Doctests were verified separately:)

$ cargo test -p paladin-memory --doc
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.59s

$ cargo fmt --all --check
(no output, exit 0)

$ cargo clippy -p paladin-memory --all-targets -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.00s
(clean, no warnings)

$ cargo clippy -p paladin-memory --no-default-features --all-targets -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.17s
(clean, no warnings)

$ make api-surface
Checking public API surface...
Extracting public API surface using cargo-public-api...
API surface extracted to /tmp/tmp.4WutJaH3Sw (3959 items)
API surface unchanged
(exit 0 -- no api-surface-update or CHANGELOG entry required)
```

Each of the three fix commits additionally passed the repo's own pre-commit hook in full
(`cargo fmt (check)`, `cargo clippy (workspace, -D warnings)`) before landing — confirmed by
the background commit logs for `eda96bbd`, `3ed8b9d2`, and `6cd77888`.

## Process Note

WR-02 and WR-03 both touch `rag_retrieval_service.rs` with adjacent hunks. An initial attempt
to stage them separately with `git add -p` and then `git commit -m ... -- <file>` produced a
single merged commit, because `git commit` with a trailing pathspec commits the current
working-tree content of that path rather than respecting partial `git add -p` staging. This was
caught immediately (`git status` was unexpectedly clean after the "WR-03" commit), corrected
with `git reset --soft HEAD~1`, and re-done as two properly sequential edit → verify → commit
cycles (strip the WR-03 doc hunk, commit WR-02 alone; reapply the WR-03 doc hunk, commit WR-03
alone). Final history is the three clean, atomic commits listed above.

---

_Fixed: 2026-09-16T20:34:17Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
