---
phase: 36-rustdoc-zero-warning-bar-examples-currency
plan: 13
subsystem: infra
tags: [rustdoc, changelog, windows-ledger, ci-evidence, closure, adr-0033]

# Dependency graph
requires:
  - phase: 36-rustdoc-zero-warning-bar-examples-currency
    provides: 36-01-SUMMARY.md through 36-12-SUMMARY.md (every plan's own per-crate/per-example
      closure table, the 143 RD-nn rustdoc fixes, the 64 EX-nn examples-currency fixes, and the
      36-12 gate wiring plus its closing zero measurement)
provides:
  - .planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-EVIDENCE.md extended into
    the phase-level closure record -- all 143 RD-nn and all 64 EX-nn rows traced to a commit,
    the two drift observations recorded, and the baseline-vs-closing measurement table
  - .planning/WINDOWS.md rows 36 and 37 moved to `fixed` through `gsd-tools windows fixed`
  - CHANGELOG.md [0.10.0] ### Documentation section appended with four reader-facing bullets
    (zero-warning rustdoc bar, fourteen new example programs, HTTP-host router parity, gallery
    index correction)
  - .planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-CI-EVIDENCE.md recording
    the real pushed-branch CI run (35290763563) proving the new all-features documentation gate
    is green, plus the doctest and Example Muster jobs, with every figure matching the local
    closing measurement exactly (D-03: no disagreement found)
affects: [36.1, 37]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A required-check change is never verified on a local measurement alone: D-12's
      Phase-33-derived pattern is push, wait for the real run, record job id + step
      conclusions in an NN-CI-EVIDENCE.md, and treat the CI figure as authoritative over the
      local one on any disagreement (D-03) -- applied here by reading the run live via
      `gh run view --json jobs` and cross-checking every quoted figure against the raw
      per-job logs (`gh api .../jobs/<id>/logs`), not trusting the green checkmark alone."
    - "A closure map's own mechanical verify script can be wrong about the artifact it
      verifies without the artifact itself being incomplete -- confirmed here by re-deriving
      unique-ID coverage independently of the plan's `seq -w`-based loop (see Deviations)."

key-files:
  created:
    - .planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-CI-EVIDENCE.md
  modified:
    - .planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-EVIDENCE.md
    - .planning/WINDOWS.md
    - CHANGELOG.md

key-decisions:
  - "The push and CI trigger were performed by the maintainer (not this executor), per the
    checkpoint's own instructions and the orchestrator's approval message; this executor's
    role was reading the resulting run's job/step data and writing it into
    36-CI-EVIDENCE.md."
  - "No CI-vs-local disagreement existed on any of the four gates this checkpoint verifies
    (default-feature docs, all-features docs, doctests, Example Muster), so D-03's
    'CI figure wins on disagreement' rule had nothing to arbitrate -- recorded as an
    explicit finding rather than silently assumed."
  - "The overall workflow run (35290763563) was still `in_progress` (Integration Tests,
    Docker Build) at the moment this record was written; rather than wait or claim a final
    conclusion the run had not yet reached, the four checkpoint-relevant jobs' own
    individually-`success` conclusions are what this record cites as the acceptance
    evidence, and the overall-run caveat is stated plainly."

requirements-completed: [CURR-11, CURR-12, CURR-14]

coverage:
  - id: D1
    description: "36-EVIDENCE.md carries the full 143 RD-nn + 64 EX-nn closure map, each row
      traced to a commit, with the two documented drift observations and the baseline-vs-closing
      measurement table"
    requirement: CURR-11
    verification:
      - kind: other
        ref: "grep -oE 'RD-[0-9]+' 36-EVIDENCE.md | sort -u | wc -l -> 143 (plus 1 not-minted
          mention of RD-144); grep -oE 'EX-[0-9]+' 36-EVIDENCE.md | sort -u | wc -l -> 64 (plus 1
          not-minted mention of EX-123)"
        status: pass
    human_judgment: false
  - id: D2
    description: "WINDOWS.md rows 36 and 37 both read fixed, moved only through gsd-tools"
    requirement: CURR-12
    verification:
      - kind: other
        ref: "grep -n '^| 3[67] ' .planning/WINDOWS.md -> both rows show status 'fixed' with a
          non-null resolved_at timestamp"
        status: pass
    human_judgment: false
  - id: D3
    description: "CHANGELOG.md [0.10.0] Documentation section carries four reader-facing Phase 36
      bullets, naming no RD-nn/EX-nn identifier"
    requirement: CURR-14
    verification:
      - kind: other
        ref: "git show 12aaf84e -- CHANGELOG.md (four bullets appended after the existing Phase
          35 entries); grep -cE 'RD-[0-9]+|EX-[0-9]+' CHANGELOG.md -> 0"
        status: pass
    human_judgment: false
  - id: D4
    description: "36-CI-EVIDENCE.md records the real pushed-branch CI run proving the new
      all-features documentation gate, the doctest step, and the Example Muster job"
    requirement: CURR-14
    verification:
      - kind: other
        ref: "gh run view 35290763563 --json jobs (Code Quality, Unit Tests (stable), Unit Tests
          (beta), Example Muster (Feature Matrix) all conclusion=success); raw logs
          ci-job-lint.log / ci-job-unit.log / ci-job-examples.log cross-checked for the exact
          figures (0 warnings, 462/0/210 doctests, 62/62 example binaries)"
        status: pass
    human_judgment: false

duration: 25min
completed: 2026-09-18
status: complete
---

# Phase 36 Plan 13: Rustdoc Zero-Warning Bar & Examples Currency — Closure Summary

**Closed the phase's bookkeeping (207-row closure map, two WINDOWS.md rows, four CHANGELOG bullets) and recorded a real pushed-branch CI run proving the new all-features rustdoc gate (CI run 35290763563, every figure matching the local closing measurement exactly).**

## Performance

- **Duration:** ~25 min (this continuation; Tasks 1-2 were completed by a prior executor)
- **Started:** 2026-09-17 (Task 1); this continuation resumed 2026-09-18 at Task 3
- **Completed:** 2026-09-18
- **Tasks:** 3/3 (Tasks 1-2 by the prior executor, Task 3 by this continuation)
- **Files modified:** 4 (`36-EVIDENCE.md`, `.planning/WINDOWS.md`, `CHANGELOG.md`, `36-CI-EVIDENCE.md`)

## Accomplishments

- Assembled `36-EVIDENCE.md` into the full phase-level closure map: all 143 `RD-nn` rustdoc-fix
  rows and all 64 `EX-nn` examples-currency rows from `34-AUDIT.md` §6, each traced to the commit
  that closed it, plus a per-crate ID inventory appendix, the two drift observations (zero drift
  at phase start; the RAG capability-token drift plan 36-08 found and recorded), and a baseline-
  vs-closing measurement table citing every `36-evidence/` capture file.
- Resolved `.planning/WINDOWS.md` rows 36 (the whole 143-row rustdoc enumeration) and 37 (RD-01
  with followers RD-66/RD-126 at `crates/paladin-memory/src/token_counter/mod.rs:3`) to `fixed`,
  moved only through `gsd-tools windows fixed <id>` — never hand-edited.
- Appended four reader-facing bullets to `CHANGELOG.md`'s existing `[0.10.0]` `### Documentation`
  subsection (after the Phase 35 entries): the zero-warning rustdoc bar now enforced in three
  places (local, pre-push, CI); fourteen new example programs with a complete gallery index; the
  two in-process HTTP host examples now mounting the real shipped routers; and the gallery
  index's corrected minimum-Rust-version and result-field claims. No `RD-nn`/`EX-nn` identifier
  appears anywhere in the file.
- Recorded the real pushed-branch CI run (`35290763563` on `feature/phase-33`, head
  `20195975c1c2665abb169b287fa178353d672bd2`) in `36-CI-EVIDENCE.md`: the `lint` job's two
  documentation steps (`Check documentation`, `Check documentation (all features, -D warnings)`),
  the `test` job's `Run doc tests` step, and the `examples` job's seven build steps plus its
  binary-count assertion all concluded `success`, with every figure — 0 `warning:` lines, exit 0
  on the all-features bar, 462 passed / 0 failed / 210 ignored doctests, 62/62 example binaries —
  matching the local closing measurement from `36-evidence/36-12-closing-measurement.txt`
  exactly. Every quoted figure was cross-checked against the raw per-job logs, not read off the
  green checkmark alone.

## Task Commits

1. **Task 1: Assemble 36-EVIDENCE.md** — `b58001e4` (docs) — by the prior executor
2. **Task 2a: Resolve WINDOWS.md rows 36 and 37** — `120324a1` (chore) — by the prior executor
3. **Task 2b: Append CHANGELOG.md Documentation bullets** — `12aaf84e` (docs) — by the prior executor
4. **Task 3 (seed): Seed 36-CI-EVIDENCE.md with local evidence + PENDING CI-run table** — `ea8d5b05` (docs) — by the prior executor
5. **Task 3 (completion): Record the real CI run** — `0834e977` (docs) — this continuation

**Plan metadata:** (this commit — see final_commit below)

## Files Created/Modified

- `.planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-EVIDENCE.md` — extended into the full phase-level closure map and measurement record (prior executor)
- `.planning/WINDOWS.md` — rows 36 and 37 moved to `fixed` via `gsd-tools` (prior executor)
- `CHANGELOG.md` — four Documentation bullets appended under `[0.10.0]` (prior executor)
- `.planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-CI-EVIDENCE.md` — seeded with local sweep + PENDING table (prior executor), completed with the real CI-run table in this continuation

## Decisions Made

- The branch push and CI trigger were the maintainer's own step (never an executor action, per
  the checkpoint's `<what-built>` and this continuation's explicit resume instructions); this
  continuation's job was reading and faithfully transcribing the resulting run's job/step data.
- Every figure in the CI run agreed exactly with the local closing measurement — no D-03
  disagreement existed to arbitrate. This is recorded plainly as a finding (see
  `36-CI-EVIDENCE.md`'s "Local-versus-CI agreement" section) rather than left unstated.
- The overall workflow run was still `in_progress` (`Integration Tests`, `Docker Build` running)
  at the moment this record was written. Rather than poll or claim a final conclusion not yet
  reached, `36-CI-EVIDENCE.md` records the four checkpoint-relevant jobs' own already-`success`
  conclusions as the acceptance evidence and states the overall-run caveat honestly in its own
  section.

## Deviations from Plan

### Documented, no fix required (mechanical-check false positives, not artifact gaps)

**1. [Task 1, prior executor] The verify script's `seq -w 1 143` loop reports 99 spurious
"MISSING" IDs against the real two-digit `RD-nn` IDs (`RD-01`, not `RD-001`)**
- **Found during:** Re-verifying Task 1's closure map in this continuation, before writing this
  SUMMARY.
- **Issue:** The plan's own `<verify><automated>` block for Task 1 loops `for i in $(seq -w 1
  143)` and greps for `"RD-$i"`, which zero-pads to three digits (`001`..`143`) for any run where
  the shell's `seq -w` sees a three-digit upper bound. The actual IDs recorded throughout
  `36-EVIDENCE.md` (and in `34-AUDIT.md` §6, the source of truth) are two-digit for 1-99 (`RD-01`
  … `RD-99`) and three-digit only for 100-143 (`RD-100` … `RD-143`). Running that literal loop
  reports `MISSING RD-001` through `MISSING RD-099` (99 false positives) even though every one of
  those 99 IDs is genuinely present as `RD-01` … `RD-99`.
- **Fix:** No code or evidence-file change was needed — the artifact is complete. The prior
  executor (Task 1) added the "ID inventory" appendix (`36-EVIDENCE.md` lines ~278-294) that
  spells out every one of the 143 IDs literally, grouped by closing commit, specifically so a
  correctly-padded grep (or manual inspection) can confirm completeness independent of the
  verify script's own padding bug. This continuation independently re-derived coverage with
  `grep -oE 'RD-[0-9]+' 36-EVIDENCE.md \| sort -u \| wc -l` (143, plus one incidental mention of
  the not-minted `RD-144`) and confirmed all 143 IDs are present.
- **Files affected:** None (documentation-only finding; the plan's own verify script text is not
  a plan artifact this task is permitted to edit).
- **Impact:** None on the phase's actual correctness — recorded here so a future reader of this
  plan's `<verify>` block does not mistake the script's own padding assumption for a real gap.

**2. [Task 2, prior executor] `CHANGELOG.md` has two `### Documentation` headings file-wide, not
one — the second belongs to the historical `[0.5.0]` section**
- **Found during:** Re-verifying Task 2's CHANGELOG edit in this continuation.
- **Issue:** Task 2's `<verify>` block asserts `grep -c '^### Documentation' CHANGELOG.md` equals
  1 for the whole file. The file actually contains two such headings: one at line 421 under the
  current `[0.10.0]` section (the one this plan's bullets were appended to) and one at line 1118
  under the historical `[0.5.0]` section (dated 2026-06-03, present since long before this
  phase — Phase 5's own mdBook migration entry, unrelated to Phase 36). The plan's acceptance
  criteria actually require "exactly one `### Documentation` heading under `[0.10.0]`" — a
  narrower, correct claim the whole-file grep in the automated `<verify>` block does not express.
- **Fix:** No CHANGELOG change was needed — the `[0.10.0]` section correctly has exactly one
  `### Documentation` heading, and the append landed inside it, after the existing Phase 35
  bullets, exactly as instructed. This continuation confirmed with `awk '/^## \[/{v=$0}
  NR==1118{print v; exit}' CHANGELOG.md` → `## [0.5.0] - 2026-06-03`, proving the second heading
  is pre-existing, historical, and out of this plan's scope.
- **Files affected:** None.
- **Impact:** None on the phase's actual correctness — recorded here for the same reason as
  deviation 1: the mechanical `<verify>` text is narrower/looser than the actual acceptance
  criteria in a way a future reader should not misread as an unresolved defect.

---

**Total deviations:** 2 documented findings, 0 fixes required, 0 auto-fixes under Rules 1-3.
**Impact on plan:** Both deviations are about the plan's own mechanical verify-script text being
imprecise, not about the underlying artifacts being incomplete or wrong. Both closure map and
CHANGELOG section were independently re-verified by this continuation using ID-inventory and
section-scoped checks that do not share the same false-positive shape, and both pass.

## Issues Encountered

None beyond the two documented findings above. The CI run recording itself (this continuation's
Task 3 work) proceeded without incident: the maintainer's push had already produced a completed
run for the four checkpoint-relevant jobs by the time this continuation read it; only two
unrelated jobs (`Integration Tests`, `Docker Build`) were still running.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Phase 36 is fully closed: all 207 audit rows (143 `RD-nn` + 64 `EX-nn`) are traceable by ID
  from `36-EVIDENCE.md` to a commit, `WINDOWS.md` rows 36 and 37 read `fixed`, the changelog
  tells a reader what shipped without exposing an internal identifier, and a real CI run proves
  the new required-check step. Phase 36.1 SC4 and Phase 37 can verify this phase by ID against
  `36-EVIDENCE.md`'s closure map without re-deriving anything.
- `make api-surface` was confirmed unchanged (3959 items) as of the local closing measurement
  (`36-evidence/36-12-closing-measurement.txt`); no further api-surface action is needed for this
  plan (D-28's own gate was already satisfied before this continuation began).
- No blockers. The only open item outside this plan's scope is the pre-existing crate-level
  rustdoc `#[allow(rustdoc::...)]` suppressions (309 hidden content diagnostics), explicitly
  recorded as record-only in `deferred-items.md` by plan 36-12 and not owned by this plan.

---
*Phase: 36-rustdoc-zero-warning-bar-examples-currency*
*Completed: 2026-09-18*

## Self-Check: PASSED

All five referenced files found on disk (`36-EVIDENCE.md`, `.planning/WINDOWS.md`,
`CHANGELOG.md`, `36-CI-EVIDENCE.md`, this `36-13-SUMMARY.md`); all five referenced commit hashes
(`b58001e4`, `120324a1`, `12aaf84e`, `ea8d5b05`, `0834e977`) found in `git log --oneline --all`.
