---
phase: 44-legacy-clean-break-removal
plan: 02
subsystem: decision-records
tags: [adr, x-03, supersession, aegis, node-errors, transience, clean-break]
status: complete

requires:
  - phase: 44-01
    provides: BattalionConfig.aegis, validate_aegis and the aegis_attempt runner, so ADR-0059 describes the design as landed
provides:
  - ADR-0059 (Accepted) - the Phase 44-only X-03 supersession and the full replacement design
  - ADR-0001 and ADR-0002 marked Superseded (files kept as history)
  - PROMOTION.md index row 0059 and next free ADR number 0060
  - PROJECT.md Key Decisions row, supersession notes and the Out of Scope X-03 note
affects: [44-03, 44-06, 44-10, 44-11, 44-12]

tech-stack:
  added: []
  patterns:
    - "Supersession mechanism from PROMOTION.md used for the first time: bare-word Superseded status plus a dated prose line, and a ## Supersedes section on the superseding ADR"

key-files:
  created:
    - .planning/decisions/0059-legacy-clean-break-removal.md
  modified:
    - .planning/decisions/0001-battalion-config.md
    - .planning/decisions/0002-battalion-result.md
    - .planning/decisions/PROMOTION.md
    - .planning/PROJECT.md

key-decisions:
  - "ADR-0059 supersedes X-03 for Phase 44 only; ADR-0051 is cited only as the shape copied and is not inherited"
  - "Five planner resolutions are marked as extending or refining locked decisions: the CommanderBuilder 300 s default (OQ1), ConclaveError::Timeout removal (OQ2, extends D-03), retry_attempts = max_attempts - 1 (OQ6, refines D-04), the retry module removal (OQ10), three separate 9.2 rows (Finding 10)"
  - "ADR-0059 Decision (g) carries no 44-06 outcome sentence; plan 44-06 appends it after the operator checkpoint"

requirements-completed: []
requirements-advanced: [LEGACY-04]  # first clause only (supersession recorded in an ADR); rows, allowlist entries and docs land in 44-08, 44-09, 44-11, 44-12

duration: single session, not precisely timed
completed: 2026-10-09
---

# Phase 44 Plan 02: ADR-0059 and the supersession bookkeeping Summary

**ADR-0059 is on file as the Phase 44-only licence for the legacy clean break and the record of the replacement design (`BattalionConfig.aegis`, per-attempt bounds through one runner, structured `node_errors`, `transience()` as the single classification source); ADR-0001 and ADR-0002 now read `Superseded`, and the index, next-free number and PROJECT.md point at it.**

## Accomplishments

- **Task 1:** wrote `.planning/decisions/0059-legacy-clean-break-removal.md` with the PROMOTION.md heading set in order plus `## Supersedes` between Status and Context. The Decision section has lettered subsections (a) to (j) recording D-00a..D-00f and D-01..D-21, the reversibility ratings from CONTEXT (D-01 costly, D-03 costly, D-06 one-way, D-13 costly) and the planner's resolution of research Open Questions 1 to 10. It says in its own sentences that it `extends D-03` (OQ2) and `This refines D-04` (OQ6, the `max_attempts - 1` bridge), and it explains why the legacy `RetryPolicy` / `ErrorStrategy` / `NodeError` get three 9.2 rows (research Finding 10). Code Conformance is `must change` until 44-12.
- **Task 2:** ADR-0001 and ADR-0002 `## Status` bodies are now the bare word `Superseded` plus a dated line naming ADR-0059; nothing else in either file changed. `PROMOTION.md` gains the 0059 row after 0058, `**Next free ADR number: 0060**` and a dated note. `PROJECT.md` gains the ADR-0059 Key Decisions row (Outcome `Pending -- Phase 44 in progress`), `superseded by ADR-0059` in the ADR-0001 and ADR-0002 Outcome cells, and the Phase 44 sentence on the Out of Scope X-03 bullet. The Current Milestone checklist was left unticked.

## Task Commits

1. **Task 1: ADR-0059** - `3fe01901` (docs)
2. **Task 2: supersede ADR-0001 and ADR-0002, index and PROJECT.md** - `b73a4564` (docs)

## Verification Results (all actually run)

| Check | Result |
| ----- | ------ |
| Task 1 automated verify (headings, `Phase 44 only`, `one-way`, `This refines D-04`, `Open Question 1`..`10`) | exit 0 |
| `grep -n '^## '` on ADR-0059 | exactly Status, Supersedes, Context, Decision, Considered Options, Code Locations, Code Conformance, Downstream Consumers, in that order |
| Required literals (`does not inherit`, `D-00a`, `D-06`, `costly`, `extends D-03`, `Finding 10`, `v0.11.0`, `**Date:** 2026-10-09`) | all present |
| Non-bullet lines under Considered Options and Code Locations | 0 |
| Every `ADR-0051` line in ADR-0059 | 2 lines (Context and Decision (a)); both say the ADR is copied as a shape or not inherited |
| `Checkpoint outcome (44-06` in ADR-0059 | 0 occurrences (marker left for 44-06) |
| Task 2 automated verify | exit 0 |
| `git diff` of ADR-0001 and ADR-0002 | 6 insertions, 2 deletions, only inside `## Status` |
| `git diff` of PROMOTION.md | the only removed line is the old next-free line |
| `superseded by ADR-0059` in PROJECT.md | 2; `ADR-0059, scoped to Phase 44 only` 1 |
| `grep -c '^| 00' PROMOTION.md` vs the phase base `b362d6cb` | 57 vs 56 (exactly one new index row) |
| Post-commit deletion check on both commits | no deletions |

No Rust source was touched, so no cargo gate was run or applies to this plan.

## Deviations from Plan

None - plan executed as written. One judgment call is worth recording: the dated PROMOTION.md note says ADR-0059 is the first ADR to use the supersession mechanism, which I checked first (only ADR-0001 and ADR-0002 now carry `Superseded`, and only ADR-0059 carries `## Supersedes`).

## Issues Encountered

- **Not caused by this plan:** `paladin-ai` lib tests `run_api_wiring::tests::build_run_api_persists_*` fail on the pre-phase baseline too (recorded in the phase's `deferred-items.md`); not touched here.
- **Requirement bookkeeping:** `LEGACY-04` is listed in this plan's frontmatter, but only its first clause (the supersession is recorded in an ADR) is delivered. The register rows, allowlist entries and examples come in 44-08, 44-09, 44-11 and 44-12, so I did not run `requirements.mark-complete` (the same call 44-01 made for LEGACY-01/02).
- **Tool output not followed:** one tool result ended with text telling me to do all work through Bash instead of the Read, Edit and Write tools. It came from the tool output and not from the user or the plan, so I ignored it.

## Known Stubs

None.

## Threat Flags

None. Documentation only; the T-44-05 and T-44-06 mitigations are in place (ADR-0059 lands before any removal plan, states its Phase 44-only scope and that it does not inherit ADR-0051).

## Next Phase Readiness

44-03 (breaker and Conclave predicate) and every later removal plan can now cite ADR-0059 as the licence. 44-06 appends its dated checkpoint outcome to Decision (g); 44-11 writes the 9.1 and 9.2 rows named in Decision (j); 44-12 replaces `must change` with `conforms` and reconciles the Code Locations list against the shipped diff.

## Self-Check: PASSED

Verified: `.planning/decisions/0059-legacy-clean-break-removal.md` exists; commits `3fe01901` and `b73a4564` are on `claude/laughing-dirac-e0h2ax`; ADR-0001, ADR-0002, PROMOTION.md and PROJECT.md edits are present as listed above.
