---
phase: 29-program-gates-release
plan: 03
subsystem: infra
tags: [ci, github-actions, semver-checks, makefile, mdbook, release-engineering, awk]

# Dependency graph
requires:
  - phase: 22-battlefield-state-superstep-engine
    provides: "the crate-level allowlist↔MIGRATION.md §9.2 set-equality CI step this plan tightens to row level (D-04)"
  - phase: 20-security-tooling
    provides: "scripts/publish-crates.sh's twelve-crate dependency order, the real publish carrier this plan's docs section cites but does not modify"
provides:
  - "Row-level `crate | type` pair set-equality gate in the `semver` CI job, replacing crate-only comparison"
  - "A working `make publish-dry-run` target (single `cargo publish --workspace --dry-run`, no `|| true`)"
  - "docs/src/appendix/release-checklist.md §5 corrected to the real twelve-crate order and the workspace dry-run command"
affects: [29-07, 29-09]

# Tech tracking
tech-stack:
  added: []
  patterns: ["awk field-range scan for a marker column that can shift under embedded pipes", "deduplicated set-equality (sort -u both sides) instead of row/entry count comparison"]

key-files:
  created: []
  modified:
    - .github/workflows/ci.yml
    - Makefile
    - docs/src/appendix/release-checklist.md

key-decisions:
  - "Implemented D-04 exactly as specified: crate|type pair set-equality, both directions, dedup via sort -u, marker located by scanning fields 6..NF (not a fixed column) so the embedded-pipe row at MIGRATION.md's second paladin-ai|Settings row is not silently skipped."
  - "Implemented D-20 exactly as specified: single `cargo publish --workspace --dry-run`, no `--allow-dirty`, docs pointer corrected to the real file path."
  - "The plan's stated count of 5 `--baseline-version 0.9.0` occurrences (interfaces block) does not match the measured count at this HEAD (3); this plan's task did not touch that part of the file at all, so the only load-bearing invariant — the count is identical before and after this plan's edit — holds and is recorded as a plan-vs-measured discrepancy, not a defect in this plan's own work."

patterns-established:
  - "CI set-equality gates should scan a marker column across a field range (6..NF), never a fixed index, whenever the register they read has any cell that can itself contain the field-delimiter character inside a quoted/backticked span."

requirements-completed: [SHIP-01, SHIP-04]

coverage:
  - id: D1
    description: "The semver allowlist gate compares `crate | type` pairs in both directions with sort -u dedup; a row/entry naming a different type under the same crate now fails, and the two legitimate `paladin-ai | Settings` rows collapse to one set member"
    requirement: "SHIP-01"
    verification:
      - kind: other
        ref: "Task 1 <verify> pipeline run directly against MIGRATION.md and .cargo/semver-checks-allowlist.toml in the worktree — 9 pairs both sides, diff -u exits 0"
        status: pass
      - kind: other
        ref: "Fail-first mismatch probe: scratch copy of MIGRATION.md with StopReason renamed to StopReasonMutated, same pipeline run against it — diff -u exits 1 (mismatch correctly detected)"
        status: pass
    human_judgment: false
  - id: D2
    description: "`make publish-dry-run` runs a single `cargo publish --workspace --dry-run` with no failure suppression and a corrected docs pointer"
    requirement: "SHIP-04"
    verification:
      - kind: other
        ref: "make -n publish-dry-run (dry-run of make itself) — exits 0, prints exactly one cargo publish --workspace --dry-run invocation, no || true, no --allow-dirty"
        status: pass
    human_judgment: false
  - id: D3
    description: "docs/src/appendix/release-checklist.md §5 lists the real twelve-crate order (including paladin-herald and paladin-eval) and the workspace dry-run command, with the stale eleven-crate/per-crate-loop caveat removed"
    requirement: "SHIP-04"
    verification:
      - kind: other
        ref: "grep -c paladin-eval / paladin-herald / eleven-crate / 'publish --workspace --dry-run' against the file"
        status: pass
      - kind: other
        ref: "mdbook build docs/ (with mdbook-mermaid install run first to generate the gitignored mermaid.min.js/mermaid-init.js assets absent in a fresh worktree) — exits 0, 'No broken links found'"
        status: pass
    human_judgment: false

# Metrics
duration: ~10min
completed: 2026-09-10
status: complete
---

# Phase 29 Plan 03: Tighten the semver allowlist gate and fix the dry-run publish target Summary

**Row-level `crate | type` pair set-equality replaces crate-only comparison in the `semver` CI job; `make publish-dry-run` is now one real `cargo publish --workspace --dry-run` instead of an eleven-line `|| true` loop that could never fail.**

## Performance

- **Duration:** ~10 min
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- The `semver` job's "Verify allowlist is set-equal to the MIGRATION.md §9.2 register" step now builds and compares a deduplicated SET of `crate | type` pairs (not crate names alone), in both directions, with the deliberate-breaking marker located by scanning fields 6..NF so the one §9.2 row with an embedded pipe inside its Mitigation cell is no longer silently skipped.
- Verified locally: the register side and allowlist side both produce the identical 9 pairs at HEAD, `diff -u` exits 0.
- Verified fail-first: a scratch copy of `MIGRATION.md` with one `Y`-row's type identifier renamed makes the same pipeline's `diff -u` exit 1 — the gate is proven to actually catch a mismatch, not just observed passing.
- `make publish-dry-run` rewritten from an 11-line per-crate `cargo publish --dry-run -p <crate> || true` loop (which addressed the `paladin-core` directory name that can never resolve, omitted `paladin-herald`, and pointed at a nonexistent `docs/RELEASE_CHECKLIST.md`) to a single `cargo publish --workspace --dry-run`, with no failure suppression and a corrected closing message.
- `docs/src/appendix/release-checklist.md` §5 rewritten: the real twelve-crate dependency order (adding `paladin-herald` and `paladin-eval`), the `cargo publish --workspace --dry-run` command, and the removal of the "expect dependent dry-runs to fail" caveat (replaced with the actual reason it no longer applies — intra-workspace path resolution).

## Task Commits

Each task was committed atomically:

1. **Task 1: Tighten the allowlist gate from crate names to `crate | type` pairs** - `28d8e17f` (feat)
2. **Task 2: Rewrite `publish-dry-run` to the one command that works, and fix the checklist section it points at** - `4dfe9848` (fix)

**Plan metadata:** committed alongside this SUMMARY (see below)

## Files Created/Modified
- `.github/workflows/ci.yml` — the "Verify allowlist is set-equal to the MIGRATION.md §9.2 register" step rewritten to a row-level `crate | type` pair set-equality comparison; nothing else in the `semver` job (packages list, `--baseline-version 0.9.0` occurrences, step count) touched.
- `Makefile` — `publish-dry-run` target rewritten to a single `cargo publish --workspace --dry-run`, `.PHONY`/`release-check` prerequisite preserved.
- `docs/src/appendix/release-checklist.md` — §5 "Dry-Run Publish Validation" rewritten to the twelve-crate order and the workspace command; every other section left untouched.

## Decisions Made
- Followed D-04 and D-20 exactly as specified in `29-CONTEXT.md` and `29-03-PLAN.md` — no architectural deviation.
- The plan's `<interfaces>` block states five `--baseline-version 0.9.0` occurrences in `ci.yml` at lines 276, 304, 334, 339, 357; the measured count at this HEAD is 3 (lines 276, 334, 357 — lines 304 and 339 instead read "vs v0.9.0" and "the published v0.9.0 baseline", which are not the literal string being counted). This plan's task never touches that part of the file, so the count is identical before and after this plan's own edit (verified: 3 both times) — the discrepancy is a plan-authoring imprecision about a section this plan does not modify, not a defect introduced here. Recorded here for the acceptance audit (D-12/29-07) rather than silently adjusted.

## Deviations from Plan

None — plan executed exactly as written. The `mdbook-mermaid install .` step run before `mdbook build docs/` is not a deviation: `docs/mermaid.min.js`/`docs/mermaid-init.js` are gitignored generated assets (`.gitignore` lines 21-22) that a git worktree does not inherit from the main checkout; generating them is a one-time environment-setup step required to run the plan's own prescribed verification command, not a change to any tracked file (confirmed via `git status --short` showing no new untracked entries after generation).

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- The row-level allowlist gate and the working `publish-dry-run` target are both prerequisites the later SHIP-04 plans (bump, changelogs, dry-run evidence, `29-09-PLAN.md`) build on.
- No blockers. The baseline-version-count discrepancy noted above is informational only and does not block any downstream plan.

## Self-Check: PASSED

- FOUND: `.github/workflows/ci.yml`
- FOUND: `Makefile`
- FOUND: `docs/src/appendix/release-checklist.md`
- FOUND: `.planning/phases/29-program-gates-release/29-03-SUMMARY.md`
- FOUND commit: `28d8e17f` (Task 1)
- FOUND commit: `4dfe9848` (Task 2)

---
*Phase: 29-program-gates-release*
*Completed: 2026-09-10*
