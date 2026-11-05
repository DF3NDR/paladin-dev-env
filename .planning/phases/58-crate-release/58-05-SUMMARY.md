---
phase: 58-crate-release
plan: 05
subsystem: release-prep
tags: [audit, disposition-register, ship-10, ship-09, ship-08, maintainer-decision]
status: complete

requires:
  - phase: 58-crate-release
    provides: plan 58-01 CI-health baseline; plan 58-02 key scrub; plan 58-03 gate and doc-model fixes; plan 58-04 changelog anchors
provides:
  - "58-AUDIT-DISPOSITIONS.md: 134-row register (R: 94, V: 40), one verdict per row, each with a Re-proof read on the head"
  - "58-check-register.sh: register-completeness verifier (BAD_COLUMNS, WRONG_SECTION, DUP_ID, NO_REPROOF, BAD_VERDICT, FIXED_NO_POINTER, WAIVED_NO_REASON, REHOMED_OWNER, REHOMED_DATE, REHOMED_TODO), `--draft` tolerates the pending-date markers"
  - "58-CI-EVIDENCE.md: F-58-04 (SHA-E runs cancelled; descendant read), F-58-05 (Phase 52 and 56 review warnings), `## Audit sweep (D-06)`, `## Maintainer decisions (58-05)`"
  - "The maintainer's answers (DATE, FLAKE, WORDING, KEY, PURGE) and the D-07 and D-00j amendments, in a form plans 58-06, 58-07, 58-08, 58-10, 58-12 and 58-15 read"
affects: [58-06, 58-07, 58-08, 58-10, 58-12, 58-15]

tech-stack:
  added: []
  patterns:
    - "Prove-then-lock: every register row names the command run or file read, on a stated head, before its verdict is written"
    - "The verifier reads the committed register as the source of truth; it is never regenerated"

key-files:
  created:
    - .planning/phases/58-crate-release/58-check-register.sh
    - .planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md
    - .planning/phases/58-crate-release/58-05-SUMMARY.md
  modified:
    - .planning/phases/58-crate-release/58-CI-EVIDENCE.md

key-decisions:
  - "DATE=2026-11-09: recheck_by for every re-homed row (D-09); plan 58-06 writes the dated todos"
  - "FLAKE=named-rerun amends D-00j for treasury_spend_prints_one_row_per_currency only: one re-run, only for an identical test name and panic text, only in PR or push CI, never in release.yml, run ids and panic line recorded; anything else halts"
  - "WORDING=pointer-list for the SHIP-09 paragraph (MIGRATION 11.8), to be re-read against plan 58-15's final change before 58-07 writes it"
  - "KEY=rotated:2026-11-05 (the maintainer's typo for a future date was corrected by the maintainer): R-84 fixed"
  - "PURGE=none: R-85 waived; the key is rotated, the literal in history since 928c6d5f9 is inert, a history rewrite would force-push main and invalidate every clone"
  - "D-07 amended: 56-REVIEW WR-03, WR-04, WR-09 (credential-class) are fixed in 0.11.1 through inserted plan 58-15, before the tag"

patterns-established:
  - "A decision checkpoint reply is recorded verbatim with its timestamp, then a resolved-value line, then a table of where each answer lands"

requirements-completed: []

duration: "Tasks 1-2 started 2026-11-05T11:36Z; Task 3 committed 2026-11-05T16:20Z (includes the wait for the maintainer's decision)"
completed: 2026-11-05
---

# Phase 58 Plan 05: Pre-tag Audit Sweep and Disposition Register Summary

**A 134-row disposition register with a self-checking verifier, the SHA-E CI read, and the maintainer's pre-bump decisions (DATE 2026-11-09, FLAKE named-rerun, WORDING pointer-list, key rotated 2026-11-05, no history purge, WR-03/04/09 fixed before the tag).**

## Performance

- **Tasks:** 3 of 3 (Task 1 tracer, Task 2 sweep, Task 3 decision checkpoint, answered)
- **Files:** 3 created, 1 modified, all under `.planning/phases/58-crate-release/`. No `src/` or `crates/` file changed.

## Accomplishments

- **Register:** 134 rows (R: 94, V: 40). Verdicts after the maintainer's answer: fixed 48, waived 37, re-homed 49. `bash .planning/phases/58-crate-release/58-check-register.sh --draft` reads `0 failure(s)`.
- **Sources:** every named source is a row in `## Sources` with an item count, including the `0 items` ones. The sweep covered the Phase 57 review and register evidence, the Phase 52, 55 and 56 deferred items, the Phase 54 review and close notes, the Phase 47 carried tooling findings, STATE and this phase's own findings, the seven pending todos, and all 40 `tech_debt` items of the v0.11.0 audit.
- **Sweep results (D-06),** read on head `0e1435f7d`: `./scripts/check-review-dispositions.sh` `checked 51 finding(s) across 13 review file(s); 0 failure(s)`; `make check-findings-log` OK (36 entries); `gsd-tools windows status` `open_count: 0` (waived 53, fixed 55, total 108). Also on `9eba21d9b`: `make security` exit 0, `make audit` exit 0 (no vulnerability), `make check-gates` exit 0, `make test-shell-guards` 85 assertions passed, `check-doc-models.sh` 0 deprecated literals.
- **Verifier proof:** shellcheck clean. On a temporary copy, `BAD_VERDICT` fired for a `maybe` verdict, `REHOMED_TODO` for a missing todo, `NO_REPROOF` for an empty Re-proof, `DUP_ID` for a duplicate id, `FIXED_NO_POINTER` for an unresolvable commit. Row order does not change the verdict. Without `--draft` the real file fails on the pending-date markers, which is intended (plan 58-06 writes the dates and the verifier then runs without `--draft`).
- **SHA-E CI (F-58-04):** all four runs on SHA-E `9166fe1fe` concluded `cancelled` (concurrency group), so SHA-E has no CI conclusion of its own. The nearest completed run is on descendant `33cdc08e0`, with zero diff over `src`, `crates`, `Cargo.lock`, `Cargo.toml`, `scripts/sdk-smoke` and `docs/schemas`: `ci.yml` run 41965173591 `success`, SDK smoke job 127040942208 `success`, `Build & Test (cli)` job 127040405991 `success`; pre-commit, semver, coverage, muster, publish-dry-run and CodeQL `success`. The SDK row is `fixed` on the descendant read.
- **F-58-05:** the Phase 52 and Phase 56 review warnings (12) had never been dispositioned or fixed; all are register rows. Three are credential-class: 56 WR-03 (R-47), WR-04 (R-48), WR-09 (R-53).

## Maintainer decisions (58-05)

The maintainer replied on 2026-11-05 (UTC), in two parts, recorded verbatim here and in `58-CI-EVIDENCE.md` under `## Maintainer decisions (58-05)`.

Part 1 (free text):

```
DATE=2026-11-09
FLAKE=named-rerun
WORDING=pointer-list
KEY=rotated:2026-11-07
PURGE=are you asking me if the git history should be purged by you, and is this related to the already rotated key which is no longer valid?

fix WR-03,WR-04,WR-09 before tag
```

Part 2 (answers to the orchestrator's three clarifying questions):

- `PURGE`: `none`. Reason: the key is rotated and no longer valid; the literal in history (since `928c6d5f9`) is inert; a history rewrite would force-push `main` and invalidate every clone.
- `KEY` date: the key was already rotated; `rotated:2026-11-07` was a typo for a future date; the actual date is **2026-11-05**. Resolved value: `KEY=rotated:2026-11-05`.
- `WR-03`, `WR-04` and `WR-09` land through an inserted gap-closure plan, **58-15** (`58-15-PLAN.md`, wave 3, `gap_closure: true`), executed before plan 58-06's pre-bump sweep; its executor flips the three register rows to `fixed` with commit pointers and adds the changelog and MIGRATION section 11.1 entries.

Resolved decision: `DATE=2026-11-09 FLAKE=named-rerun WORDING=pointer-list KEY=rotated:2026-11-05 PURGE=none SDK=` (`SDK` not asked: the SDK smoke is green on the descendant read and no objection was raised).

How it was applied: R-84 (key liveness and rotation) is `fixed`, pointing at the evidence section; the rotation is the maintainer's own statement and is not re-provable from the repository. R-85 (history purge) is `waived` with the maintainer's reason. R-47, R-48 and R-53 keep verdict `re-homed` (no fix commit exists yet) and their pointer cells now carry the order to fix before the tag through plan 58-15; that plan flips them to `fixed`. The other `re-homed` rows keep `date: pending 58-05 decision` until plan 58-06 writes the dated todos with `recheck_by: 2026-11-09`.

Rules for later plans: the `named-rerun` rule and the `pointer-list` text are written out in full in `58-CI-EVIDENCE.md`. The SHIP-09 paragraph must be re-read against plan 58-15's final change (WR-09 can alter what a library consumer observes) before plan 58-07 writes it.

## Task Commits

1. **Task 1 (tracer): CI read on SHA-E, register verifier, 57-REVIEW rows R-01..R-08** - `0e1435f7d` (docs)
2. **Task 2: complete the sweep, the v0.11.0 reconciliation and the manual checks** - `56a04c20d` (docs)
3. **Task 3: the maintainer's decisions recorded, key rows settled, WR-03/04/09 ordered** - `2fa597af3` (docs)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] SHA-E has no CI conclusion; the descendant read is used**
- **Found during:** Task 1
- **Issue:** all four runs on SHA-E concluded `cancelled`, so the plan's "read the SDK smoke job on SHA-E" had no completed job to read.
- **Fix:** read the nearest completed runs on descendant `33cdc08e0`, after proving zero diff over every path the SDK smoke and `Build & Test (cli)` depend on. Recorded as F-58-04; the SDK row is `fixed` on that read. The maintainer was told in the decision and raised no objection.
- **Files modified:** `58-CI-EVIDENCE.md`
- **Commit:** `0e1435f7d`, `56a04c20d`

**2. [Rule 2 - Missing critical] Sweep extended to the Phase 52 and Phase 56 review files**
- **Found during:** Task 2
- **Issue:** the plan's source list did not name 52-REVIEW or 56-REVIEW, yet both hold warnings never dispositioned (F-58-05). Leaving them out would let a `passed` audit rest on an omitted source (T-58-50).
- **Fix:** added both as sources; 94 R rows against the plan's minimum of 35.
- **Files modified:** `58-AUDIT-DISPOSITIONS.md`, `58-CI-EVIDENCE.md`
- **Commit:** `56a04c20d`

**3. [Plan adjustment] Task 1 committed on its own as `docs(58-05)`**
- Task 1 is the tracer, so it was committed before the sweep, as the tracer gate requires. Task 2's commit carries the plan's named message.

**4. [Plan adjustment] Maintainer decision changes downstream scope**
- The maintainer pulled three credential-class warnings into `0.11.1` (D-07 amended) and inserted plan 58-15. This is the maintainer's direction, not an executor deviation, but it changes the release content, so it is listed here.

No `src/` or `crates/` file changed in this plan.

## Authentication Gates

None.

## Known Stubs

None. The register's pending markers (`date: pending 58-05 decision`, `to be written in 58-06`) are intentional and tolerated only by the verifier's `--draft` mode; plan 58-06 writes the dated todos and rewrites the rows, after which the verifier runs without `--draft`. No WINDOWS ledger entry was added.

## Threat Flags

None. No file under `src/` or `crates/` changed. No key value or fragment appears in any register cell, evidence line or commit message (a count-only scan of both files for a key-shaped string reads 0).

## Issues Encountered

- The maintainer's `KEY` date in the first reply was a typo for a future date; the orchestrator asked and the maintainer corrected it to 2026-11-05 before the row was written.

## Next Phase Readiness

- Plan 58-15 (inserted, wave 3) fixes WR-03, WR-04, WR-09 and flips R-47, R-48, R-53 to `fixed`; it runs before plan 58-06's pre-bump sweep.
- Plan 58-06 writes the dated todos (`recheck_by: 2026-11-09`), rewrites the pending markers and runs the verifier without `--draft`.
- Open at the tag: SHIP-08, SHIP-09, SHIP-10 are not ticked here; they are proven by later plans and the post-tag registry read.

## Self-Check: PASSED

- Files found: `58-check-register.sh`, `58-AUDIT-DISPOSITIONS.md`, `58-CI-EVIDENCE.md`, this SUMMARY.
- Commits found: `0e1435f7d`, `56a04c20d`, `2fa597af3`.
- `58-check-register.sh --draft`: `checked 134 row(s) (R: 94, V: 40); fixed 48, waived 37, re-homed 49; 0 failure(s)`.
