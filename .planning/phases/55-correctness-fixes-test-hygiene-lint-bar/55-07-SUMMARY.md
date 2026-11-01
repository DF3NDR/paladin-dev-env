---
phase: 55-correctness-fixes-test-hygiene-lint-bar
plan: 07
subsystem: paladin-eval
tags: [eval-harness, run-status, schema-golden, re-home, fixes-04, d-05]
status: complete

requires:
  - phase: 55-correctness-fixes-test-hygiene-lint-bar
    provides: "plans 55-01..55-06 landed; no code dependency"
provides:
  - "rustdoc on RunStatusValue, Assertion::RunStatus and run_status_matches stating that RunFinishStatus::Cancelled has no RunStatusValue counterpart until the next minor"
  - "docs/schemas/eval-scenario.schema.json regenerated: two description strings changed, nothing else"
  - "eval-harness.md run_status row notes that a cancelled run matches no value until the next minor"
  - "FIXES-04 re-homed with reason, owner and date in REQUIREMENTS.md (v1 note, v2 Eval harness entry, traceability row), ROADMAP Phase 55 SC2, inventory row B5 and the new phase deferred-items.md"
affects: [55-13, next-minor first phase, Phase 58 milestone audit]

tech-stack:
  added: []
  patterns:
    - "A requirement that cannot ship at patch level keeps its ID and checkbox, gains a dated italic note, and moves to a v2 entry that carries the reason and owner"

key-files:
  created:
    - .planning/phases/55-correctness-fixes-test-hygiene-lint-bar/deferred-items.md
  modified:
    - crates/paladin-eval/src/scenario.rs
    - crates/paladin-eval/src/assertion.rs
    - docs/schemas/eval-scenario.schema.json
    - docs/src/user-guides/eval-harness.md
    - .planning/REQUIREMENTS.md
    - .planning/ROADMAP.md
    - .project/v0.11.1/01-carried-gaps-inventory.md

key-decisions:
  - "D-05 applied as locked: record the re-home, add no variant, no #[non_exhaustive], no WINDOWS row, no semver run, no MIGRATION.md row"
  - "PD-07-1: the v1 FIXES-04 line keeps its ID and unchecked box; the v2 entry carries the substance; traceability status is Re-homed (next minor)"
  - "PD-07-2: the schemars description refresh is part of this plan, since RunStatusValue and Assertion::RunStatus rustdoc flows into the golden"

requirements-completed: []

duration: 15min
completed: 2026-11-01
---

# Phase 55 Plan 07: FIXES-04 re-home Summary

**FIXES-04 (`RunStatusValue::Cancelled`) is recorded as deliberately re-homed to the next minor, with its reason and owner in the rustdoc, the schema descriptions, the eval guide and four registers, and no public enum changed.**

Plan-start SHA: `3ca19e5e83110df6e219531f71b7b5300e8e4349`

`requirements-completed` is left empty on purpose: FIXES-04 is re-homed, not delivered, so it must not be ticked off as complete. Its traceability row reads `Re-homed (next minor)`.

## Performance

- **Duration:** about 15 minutes
- **Tasks:** 2 of 2
- **Files modified:** 7, plus 1 created

## Accomplishments

- A `run_status: cancelled` author now finds the reason in three places: the rustdoc, the JSON schema description and the guide table row.
- The requirement cannot read as silently dropped: v1 line note, v2 `### Eval harness` entry naming both lints and the owner, traceability status, ROADMAP SC2 reconciliation note, inventory B5 pointer (closing B5 and 42-06 by reference), and the phase deferred list.
- The phase `deferred-items.md` also carries every item of the CONTEXT `<deferred>` list with its proposed home, for the plan 55-13 carry-forward.

## Task Commits

1. **Task 1 (tracer): rustdoc, schema descriptions, eval guide** - `f70f03deb` (docs)
2. **Task 2: registers (REQUIREMENTS, ROADMAP SC2, inventory B5, deferred-items)** - `718ecd970` (docs)

## Prove-then-lock

Run before any edit:

- `grep -n 'pub enum RunStatusValue' -A12 crates/paladin-eval/src/scenario.rs` showed four variants (`Completed`, `Failed`, `Halted`, `AwaitingInput`) and no `#[non_exhaustive]`.
- `grep -n 'Cancelled' crates/paladin-core/src/platform/container/trace.rs` showed `RunFinishStatus::Cancelled` at line 174 ("A caller asked for the run to stop; recorded `Cancelled` on the run row.").

After the edits the variant count is still 4.

## Schema-golden diff stat

`git diff 3ca19e5e8 f70f03deb -- docs/schemas`: `1 file changed, 2 insertions(+), 2 deletions(-)`. Both changed lines are `"description":` lines: the `Assertion::RunStatus` description and the `RunStatusValue` description. The filter `grep -v '"description":'` over the changed lines printed nothing. `schema_matches_golden` passes after `UPDATE_EVAL_SCHEMA=1` regeneration.

## Verification

- `cargo test -p paladin-eval --offline`: unit 56, integration 12 and 1 (golden), doctests 4 passed with 2 ignored; all green.
- `make check-doc-config`: 161 YAML blocks checked, 0 failed.
- `cargo doc -p paladin-eval --no-deps --offline`: 0 warnings. `cargo fmt --check`: clean.
- Comment-only check: `git diff 3ca19e5e8 -- crates/paladin-eval/src | grep '^[-+]' | grep -v '^[-+][-+]' | grep -v '^[-+]\s*//'` prints nothing.
- Task 2 automated verify: passes. `roadmap.get-phase 55 --pick section | grep -c 'Reconciled 2026-11-01, Phase 55'` prints 1. The ROADMAP diff removes exactly one line (the SC2 line, extended in place). The inventory B5 row keeps its `|` count. `git diff 3ca19e5e8 -- .planning/milestones .planning/WINDOWS.md MIGRATION.md .cargo` is empty.
- `grep -c FIXES-04 .planning/REQUIREMENTS.md` gives hits at the v1 line, the v2 entry, the traceability row and the phase-to-requirements list (the last unchanged).

## Deviations from Plan

None - plan executed exactly as written.

The tracer feedback gate ran in auto mode (`workflow._auto_chain_active` is true): the tracer's `<verify>` had just passed end to end, so expansion to Task 2 continued.

## Issues Encountered

None. No disk-space cleanup was needed; `CARGO_INCREMENTAL=0` was set for every cargo run.

## Known Stubs

None. No WINDOWS row was opened (D-05); the gap is a documented, owned re-home rather than a stub.

## Threat Flags

None. Comment, description and register edits only; no new endpoint, auth path or schema shape.

## Next Phase Readiness

The next minor's first phase owns the variant: add `RunStatusValue::Cancelled`, the matching arm in `run_status_matches`, run `UPDATE_EVAL_SCHEMA=1 cargo test -p paladin-eval --test schema_golden`, and replace the rustdoc and guide notes. Plan 55-13 should carry FIXES-04 as "re-homed" in its `## Carry-forward` section.
