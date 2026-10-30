---
phase: 47-v0-11-0-crate-release
plan: 01
subsystem: release
tags: [release, evidence, cargo-release, cargo-deny, changelog, crates-io]

requires:
  - phase: 46
    provides: the v0.11.0 release candidate on PR #59 (branch claude/laughing-dirac-e0h2ax)
provides:
  - "47-CI-EVIDENCE.md in the 37.1 house form with the preflight, the pre-tag registry baseline and 14 seeded findings"
  - "cargo-release 1.1.7 installed outside the repository"
  - "B-1 (release body 156528 bytes) and B-2 (RUSTSEC-2026-0386) reproduced offline, not assumed"
  - "MIGRATION section 10 freeze answer: public surface unmoved, stays frozen at e28e43b5"
  - "The maintainer's six-part pre-build decision, resolved to one value per item"
affects: [47-02, 47-03, 47-04, 47-05, 47-06, 47-07, 47-08]

tech-stack:
  added: [cargo-release 1.1.7 (installed with --locked, outside the repo)]
  patterns: ["append-only dated evidence record (D-00d)", "publishable set derived from cargo metadata (D-00f)"]

key-files:
  created:
    - .planning/phases/47-v0-11-0-crate-release/47-CI-EVIDENCE.md
    - .planning/phases/47-v0-11-0-crate-release/47-01-SUMMARY.md
  modified:
    - .planning/STATE.md
    - .planning/ROADMAP.md

key-decisions:
  - "B1=a: condense the root [Unreleased] Added and Changed to fit 110,000 bytes, chunk by chunk with a ledger"
  - "B2=a: cargo update -p tokio --precise 1.53.3, lockfile only, own fix(deps) commit before the bump"
  - "R7=a: delete the empty [Unreleased] anchor in the stamp commit"
  - "EMPTY=a: one lockstep line under each empty crate heading"
  - "SPACE=a: cargo clean before the first compile and again before row 6; one retry"
  - "DATE=2026-10-25: stamp date equals tag date (D-00n)"

requirements-completed: []

duration: ~1h (Tasks 1-2 at 13:01Z-13:08Z; decision received 18:19Z)
completed: 2026-10-25
status: complete
---

# Phase 47 Plan 01: Pre-build measurement and decision Summary

**The v0.11.0 candidate is measured end to end (tree, GitHub, registry, sandbox), both release blockers are reproduced offline (a 156528-byte release body that the real payload builder rejects, and `cargo deny` red on RUSTSEC-2026-0386), and the maintainer's six pre-build items are decided and recorded before any tracked file outside `.planning/` changed.**

## Performance

- **Duration:** about 1 hour of execution, across a decision checkpoint
- **Completed:** 2026-10-25
- **Tasks:** 3 of 3 (Task 3 is the maintainer decision, recorded here)
- **Files modified outside `.planning/`:** 0

## Accomplishments

- `47-CI-EVIDENCE.md` holds the five house-form sections in order (Provenance, Local sweep, CI-run table, Registry verification (D-00h), Findings carried forward (D-00d)), 18 Local sweep rows, the pre-tag registry baseline (12 derived crates present at 0.10.1, all ABSENT at 0.11.0) and 14 dated findings plus the decision entry.
- B-1 reproduced by the real `_cor_build_payload`: exit 126, `jq: Argument list too long`, on a 156528-byte body (`Added` 84567, `Changed` 36419). `Added` plus `Changed` must shrink to at most 74,458 bytes for a 110,000-byte body with the other subsections kept byte-identical.
- B-2 reproduced: `cargo deny check advisories` exits 1 on RUSTSEC-2026-0386 (tokio 1.52.3); `cargo audit` exits 0 with warnings; the required `License & Dependency Policy` context is red on PR #59's head.
- MIGRATION section 10 freeze answered on evidence: `e28e43b5..HEAD` touched two source files (a test and rustdoc), `API Surface Tracking` is `success` on the PR head, so section 10 stays frozen.
- `cargo release --version` prints 1.1.7.

## Task Commits

1. **Task 1 (tracer): open 47-CI-EVIDENCE.md and record the preflight** - `7101b86f` (docs)
2. **Task 2: install cargo-release 1.1.7, reproduce B-1, answer the section 10 freeze, seed findings** - `efa9bd8f` (docs)
3. **Task 3: maintainer decision** - state checkpoint `3c98f27e` (docs); decision appended to the evidence `b5318327` (docs)

**Plan metadata:** the final docs commit that carries this SUMMARY, STATE.md and ROADMAP.md (hash in the orchestrator return).

## Pre-build decision

Maintainer reply, verbatim, received 2026-10-25T18:19:21Z (UTC):

> DATE=Today
> All (a) choices

Orchestrator resolution (equals the plan's `recommended` option; "Today" is 2026-10-25, the UTC date at the time of the reply). Plans 47-02, 47-03 and 47-04 read these lines:

- B1: a
- B2: a
- R7: a
- EMPTY: a
- SPACE: a
- DATE: 2026-10-25

What each value means for the build wave:

- **B1 = a.** The agent condenses root `CHANGELOG.md` `[Unreleased]` `Added` then `Changed` to a body of at most 110,000 bytes in its own docs commits, chunk by chunk, re-diffing against the pre-condensation commit after each chunk and logging a per-chunk ledger (`## B-1 condensation ledger` in `47-CI-EVIDENCE.md`) for the 47-06 sign-off. `Breaking Changes`, `Security`, `Removed`, `Fixed`, `Known limitations` and `Documentation` stay byte-identical. One bullet per shipped capability grouped by phase, no new claims, every `M-B-` and `DR-` id kept, plus a lead paragraph pointing to each crate's `[0.11.0]` changelog, `MIGRATION.md` section 10 and the uncondensed text at the pre-condensation commit.
- **B2 = a.** `cargo update -p tokio --precise 1.53.3`, lockfile only, as its own `fix(deps)` commit before the bump.
- **R7 = a.** Delete the empty `## [Unreleased]` anchor in the stamp commit; assert `[Unreleased]` count 0 and `[0.11.0]` count 1 per changelog.
- **EMPTY = a.** Under each of `paladin-content`, `paladin-memory` and `paladin-notifications`, one line: "No functional change; version bumped in lockstep with the workspace."
- **SPACE = a.** `cargo clean` before the first compile and again before row 6; an out-of-space abort is recorded as a capacity event, cleaned and re-run exactly once; a second abort hands that row to the maintainer.
- **DATE = 2026-10-25.** The `## [0.11.0] - <DATE>` stamp equals the tag date (D-00n); the merge, the `ci.yml` run on the merge commit and the tag must land the same calendar day, or the date is re-decided.

No hold, `B1=c` or `B2=c` was chosen, so the phase proceeds to plan 47-02.

## Files Created/Modified

- `.planning/phases/47-v0-11-0-crate-release/47-CI-EVIDENCE.md` - the append-only evidence record
- `.planning/phases/47-v0-11-0-crate-release/47-01-SUMMARY.md` - this summary and the decision
- `.planning/STATE.md`, `.planning/ROADMAP.md` - plan progress

## Decisions Made

The six pre-build decisions above (all the recommended defaults), recorded verbatim with their timestamp.

## Deviations from Plan

None - plan executed exactly as written. The tracer's interactive human-verify was folded into the Task 3 decision checkpoint by orchestrator instruction; no Rule 1-4 deviations occurred.

## Issues Encountered

None. The sandbox limits found (2.5 GB free disk, no Docker daemon, no MSRV 1.88 toolchain) are recorded as findings, and SPACE=a governs the disk constraint.

## Authentication Gates

None. `gh auth status` misreports an invalid token while REST reads succeed (known); no registry credential was requested or stored (D-00g).

## Known Stubs

None. This plan produces documents only.

## Threat Flags

None. No new network endpoint, auth path or schema was introduced; the only tool install was `cargo-release` 1.1.7 with `--locked`.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 47-02 may start: its precondition (a recorded B1 and B2 decision) is met. It applies B2=a first, then B1=a. SHIP-07 is not complete until plan 47-09; it is not marked here.

## Self-Check: PASSED

- FOUND: `.planning/phases/47-v0-11-0-crate-release/47-CI-EVIDENCE.md`
- FOUND commits `7101b86f`, `efa9bd8f`, `3c98f27e`, `b5318327`
- `git diff --name-only 7101b86f^..HEAD` lists no path outside `.planning/` (0 files)

---
*Phase: 47-v0-11-0-crate-release*
*Completed: 2026-10-25*
