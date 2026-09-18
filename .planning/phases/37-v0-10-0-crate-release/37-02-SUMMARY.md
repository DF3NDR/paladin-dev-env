---
phase: 37-v0-10-0-crate-release
plan: 02
subsystem: infra
tags: [release, semver-checks, msrv, evidence, d-14, d-00d, outage-recovery]

# Dependency graph
requires:
  - phase: 37-v0-10-0-crate-release
    plan: 01
    provides: "37-CI-EVIDENCE.md house form, provenance block, D-17 checkpoint resolution, Local
      sweep row 1"
provides:
  - "37-CI-EVIDENCE.md: Local sweep rows for D-06 gate rows 1, 2, 3, 4, 5 and 7, all green"
  - "A dated, append-only record of a host DNS outage + reboot mid-gate-sweep, its classification
    as not-measured (never red), the maintainer's decision, and the single authorized re-run"
affects: [37-03, 37-06, 37-07, 37-08]

# Tech tracking
tech-stack:
  added: []
  patterns: []

key-files:
  created:
    - .planning/phases/37-v0-10-0-crate-release/37-02-SUMMARY.md
  modified:
    - .planning/phases/37-v0-10-0-crate-release/37-CI-EVIDENCE.md

key-decisions:
  - "Interrupted semver-checks run (7/11 packages completed before outage, 3/11 DNS-failed, 1/11
    killed by reboot) classified not-measured, not red — no cargo semver-checks lint ever
    evaluated a diff for the four incomplete packages, so D-14/D-15's 'never re-run a red gate'
    rule does not apply to them"
  - "Maintainer authorized exactly one full 11-package re-run of the unmodified semver-loop.sh,
    verbatim option label 'Record, then full re-run (Recommended)', via AskUserQuestion"
  - "MSRV gate (row 5) recorded from the preserved pre-outage log, not re-run — it completed
    cleanly at 16:12:18Z, before the 16:22 UTC outage, and produced a complete verdict"
  - "Task 3's <automated> verify block was not run verbatim (it would exceed the 600s tool
    ceiling and would re-run MSRV a second time); its two conditions were instead asserted from
    the exit files and logs directly"

patterns-established: []

requirements-completed: []
# SHIP-05 is minted in plan 37-04 and spans all eleven plans in this phase (repo rule 5) — no
# requirements mark-complete step was run for this plan.

# Metrics
duration: ~50min (this continuation, Task 3 only; Tasks 1-2 completed by the prior agent)
completed: 2026-09-18
status: complete
---

# Phase 37 Plan 02: Local D-06 Gate Re-Seal (Rows 1, 2, 3, 4, 5, 7) Summary

**Re-ran six of D-06's seven release gates locally on the phase's pre-merge tree and recorded each
in `37-CI-EVIDENCE.md`'s Local sweep — including recovering Task 3 (semver-checks + MSRV) after a
host DNS outage and reboot killed the first semver-checks sweep mid-run, per an explicit
maintainer decision to record the interruption and then perform exactly one full re-run.**

## Performance

- **Duration:** ~50 min (this continuation agent's portion — Task 3 plus SUMMARY/state work; the
  semver-checks re-run itself was ~11 min of that, hosted detached)
- **Started:** 2026-09-18 (continuation dispatch, post-reboot)
- **Completed:** 2026-09-18
- **Tasks:** 3/3 (Tasks 1 and 2 completed by the prior agent — `cb2ebf3e`, `af21ede9`; Task 3
  completed here — `b95d2af0`)
- **Files modified:** 1 (`37-CI-EVIDENCE.md`), plus this SUMMARY and STATE/ROADMAP metadata

## Checkpoint Status

**Environment interruption checkpoint (workflow safe-resume gate, D-14 temperament) — not a
plan-authored `checkpoint:*` task, but a mid-execution orchestrator stop after a host DNS outage
and reboot killed the first semver-checks sweep.**

**What happened:** at 16:22 UTC on 2026-09-18 the host lost DNS resolution to `index.crates.io`
mid-loop, then rebooted (back up 16:35:02 UTC). The prior executor was lost. Of the 11-package
`cargo semver-checks` loop: 7 packages had already completed with a clean, real verdict
(`paladin-ai`, `paladin-ai-core`, `paladin-ports`, `paladin-battalion`, `paladin-herald`,
`paladin-llm`, `paladin-memory` — all exit `0`, `Summary no semver update required`); 3 packages
(`paladin-storage`, `paladin-notifications`, `paladin-content`) failed at exit `101` with a
verbatim DNS-resolution error, no lint ever evaluated; and 1 package (`paladin-web`) was killed by
the reboot mid-invocation, with no `.exit` file ever written. The MSRV gate (row 5) had already
completed cleanly, in full, before the outage (`Finished ... in 5m 07s`, ended `16:12:18Z`, 0
warnings).

**How the decision was obtained:** the orchestrator stopped at the workflow's safe-resume gate and
put the question to the maintainer through the runtime's interactive question mechanism
(`AskUserQuestion`), offering three options: "Record, then full re-run (Recommended)" / "Record,
re-run only the 4" / "Treat as a D-14 stop".

**Maintainer's selection, verbatim option label:** "Record, then full re-run (Recommended)".

**Reading applied (stated in-session, and recorded in `37-CI-EVIDENCE.md`):** the four incomplete
packages never produced a `cargo semver-checks` verdict — every failure traces to lost DNS
resolution, not to a lint or assertion result. This makes them **not-measured**, not red. D-14's
"never re-run a locally-red gate hoping for a different answer" therefore does not apply to
them — there was no answer to re-hope for, only an environment fault that prevented a measurement
from ever completing. The authorization covers **exactly one** re-run of the full 11-package loop
for this specific outage; it does not relax D-14 for anything else in this phase.

**What was done, in order (per the maintainer's decision, executed by this continuation):**
1. Preserved the interrupted run untouched: all 11 packages' `.log`/`.exit` pairs (or absence, for
   `paladin-web`) plus `semver-loop.log`/`.pid`, moved to
   `target/37-02/interrupted-20260918T1622Z/` (git-ignored `target/`, not committed, but present
   on disk for audit).
2. Recorded the interruption in `37-CI-EVIDENCE.md` as a dated, append-only finding — SHA, loop
   start time, all 11 per-package exit codes, the exact DNS-error lines quoted from the three
   failed logs, the reboot time, the maintainer's decision and its provenance — **before** any
   re-run was started.
3. Confirmed network recovery with one read-only `curl` to the sparse index (mandatory
   `paladin-release-check` User-Agent) → HTTP `200` on `paladin-ai`'s index entry.
4. Re-ran the full 11-package loop once, uninterrupted, using the existing `semver-loop.sh`
   unmodified, hosted detached. Result: **11/11 exit `0`**, identical `major change` / `0 checks:
   0 pass, 254 skip` / `Summary no semver update required` shape for every package, matching
   §11's own recorded shape.
5. Recorded MSRV (row 5) from the preserved pre-outage `g5-msrv.log`/`.exit` — not re-run.

## Accomplishments

- Verified Tasks 1 and 2's commits (`cb2ebf3e`, `af21ede9`) exist on the branch and did not redo
  either task.
- Preserved the interrupted semver-checks run intact under
  `target/37-02/interrupted-20260918T1622Z/` before any further action.
- Recorded the environment-interruption finding in `37-CI-EVIDENCE.md`, append-only, with the
  maintainer's decision and its provenance, before re-running anything.
- Confirmed network recovery with a read-only probe before launching the re-run.
- Re-ran `cargo semver-checks` for all 11 CI-listed packages exactly once, detached, using the
  plan's unmodified `semver-loop.sh`; recorded 11/11 passes as Local sweep rows 17-27 plus a
  tally row (28), with `paladin-eval`'s exclusion reasoned (no published `0.9.0` baseline).
- Recorded the MSRV floor (row 16) from the pre-outage log — `Finished ... in 5m 07s`, 0 warnings,
  `RUSTUP_TOOLCHAIN=1.88`, matching CI's own flag set (no `--locked`).
- Confirmed `git status --porcelain -- crates src Cargo.toml Cargo.lock` empty throughout — no
  source file, manifest, or lockfile touched by this plan.

## Task Commits

Each task was committed atomically:

1. **Task 1: Gate rows 1 and 7 — offline register guards and the changelog completeness reading**
   — `cb2ebf3e` (docs) — completed by the prior executor agent, not redone here.
2. **Task 2: Gate rows 2 and 3 — the two frozen backward-compatibility test targets** — `af21ede9`
   (docs) — completed by the prior executor agent, not redone here.
3. **Task 3: Gate rows 4 and 5 — semver-checks (11 crates) and the MSRV floor** — `b95d2af0`
   (docs) — this continuation; includes the environment-interruption finding, the single
   authorized semver-checks re-run, and the pre-outage MSRV reading.

**Plan metadata:** recorded in the final metadata commit following this SUMMARY.

## Files Created/Modified

- `.planning/phases/37-v0-10-0-crate-release/37-CI-EVIDENCE.md` — appended the Task 3 head-SHA
  note, the environment-interruption finding, Local sweep row 16 (MSRV), rows 17-27 (per-package
  semver-checks re-run), and row 28 (11/11 tally). Zero lines from Tasks 1-2's commits were
  modified or removed (`git diff --numstat cb2ebf3e~1 b95d2af0` on the file: `214  0`, additions
  only).
- `.planning/phases/37-v0-10-0-crate-release/37-02-SUMMARY.md` — this file (new).

## Decisions Made

- **Interrupted run classified not-measured, not red.** No `cargo semver-checks` lint ever
  evaluated a diff for the three DNS-failed packages or the reboot-killed one; the failures trace
  entirely to a network/environment fault, never to an assertion. Recorded plainly, by the
  maintainer's decision, not assumed by the executor.
- **Exactly one re-run, in full, of the unmodified script.** Per the maintainer's authorization
  and D-14's spirit: this is not "re-running a red gate hoping for a different answer" — it is
  completing a measurement an outage prevented from ever producing a verdict, done once, and any
  failure in the re-run itself would be a genuine D-14 stop (not a candidate for a third attempt).
- **MSRV not re-run.** It produced a complete, clean verdict before the outage; re-running it
  would have cost ~5 more minutes for no additional evidentiary value and was explicitly excluded
  by the maintainer's decision.
- **Task 3's `<automated>` verify substituted.** The plan's verify block chains the MSRV check and
  the semver loop into one foreground command, which would (a) exceed the Bash tool's 600s
  ceiling and (b) re-run MSRV a second time, contradicting the decision above. Its two conditions
  — MSRV exit 0, 11/11 semver exit files read 0 — were instead asserted directly from the
  preserved exit files and logs.

## Deviations from Plan

- **[Environment interruption, not a Rule 1-4 deviation]** A host DNS outage and reboot mid-Task-3
  interrupted the first semver-checks sweep. Handled per the maintainer's explicit decision
  (recorded above and in `37-CI-EVIDENCE.md`), not by executor judgment. No code, test, or gate
  logic was touched — this is a re-measurement of an environment fault, not a fix under Rules 1-4.
- **[Substitution, disclosed per plan's own house rules]** Task 3's `<automated>` verify command
  was not run verbatim; its conditions were asserted from exit files and logs instead, for the
  reasons given under "Decisions Made" above. No gate's pass/fail meaning was altered by this
  substitution — the same two facts (MSRV exit 0; 11/11 semver exit 0) were confirmed either way.

No Rule 1-4 auto-fixes were needed or applied. No red gate was encountered in either task's actual
measured output; the only anomaly was the environment interruption, handled per D-14/D-00d and the
maintainer's explicit direction throughout.

## Issues Encountered

- Host DNS outage + reboot at 16:22-16:35 UTC on 2026-09-18, mid-semver-checks-sweep. Resolved per
  the maintainer's decision (see Checkpoint Status above). Fully recovered; no data loss (the
  interrupted run's logs are preserved on disk under `target/37-02/interrupted-20260918T1622Z/`).
- A background-job PID-capture race in this continuation's own tooling (a `nohup ... & echo $! >
  pidfile` chain where a preceding `rm -f` was itself part of the same backgrounded compound list,
  so it raced the `echo $!` write) briefly clobbered the pid file this continuation wrote for its
  own re-run launch. Caught immediately (the pid file went missing on the next poll), the actual
  running process was located via `pgrep`/`ps`, and the correct PID was written back manually
  before continuing to wait on it. This did not affect the semver-checks re-run itself — the
  script had already launched successfully and ran to completion untouched; only this
  continuation's own polling bookkeeping was affected, and it was corrected within the same task
  before any gate result was recorded.

## User Setup Required

None — no external service configuration required by this plan.

## Next Phase Readiness

- `37-CI-EVIDENCE.md` now carries Local sweep rows for all six of D-06's gate rows this plan owns
  (1, 2, 3, 4, 5, 7), all green, plus the environment-interruption finding for full auditability.
- Gate row 7's `make publish-dry-run` (the seventh D-06 gate row) remains plan 37-03's, unchanged
  by this plan.
- The `paladin-eval` D-17 obligation carried from plan 37-01 (owed runbook, hard gate before plan
  37-08's tag hand-off) is untouched by this plan.
- `target/37-02/interrupted-20260918T1622Z/` remains on disk (git-ignored) for any future audit of
  the outage; nothing further references it.

## Self-Check

**Files:**
- FOUND: `.planning/phases/37-v0-10-0-crate-release/37-CI-EVIDENCE.md`
- FOUND: `.planning/phases/37-v0-10-0-crate-release/37-02-SUMMARY.md` (this file)

**Commits:**
- FOUND: `cb2ebf3e` (Task 1, prior agent)
- FOUND: `af21ede9` (Task 2, prior agent)
- FOUND: `b95d2af0` (Task 3, this continuation)

**Diff integrity:** `git diff --numstat cb2ebf3e~1 b95d2af0 -- .planning/phases/37-v0-10-0-crate-release/37-CI-EVIDENCE.md`
reports `214  0` — additions only, zero deletions, confirming no line written by Tasks 1-2 was
modified or removed across the whole plan.

**Gate results:** all 11 semver-checks exit files under `target/37-02/` read `0`; `g5-msrv.exit`
reads `0`; `git status --porcelain -- crates src Cargo.toml Cargo.lock` empty.

## Self-Check: PASSED

---
*Phase: 37-v0-10-0-crate-release*
*Completed: 2026-09-18*
