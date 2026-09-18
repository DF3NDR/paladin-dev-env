---
phase: 37-v0-10-0-crate-release
plan: 03
subsystem: infra
tags: [release, publish-dry-run, security, api-surface, evidence, d-14, d-00d, coverage-attribution]

# Dependency graph
requires:
  - phase: 37-v0-10-0-crate-release
    plan: 02
    provides: "37-CI-EVIDENCE.md Local sweep rows for D-06 gate rows 1, 2, 3, 4, 5 and 7, all
      green, plus the environment-interruption finding"
provides:
  - "37-CI-EVIDENCE.md: Local sweep row 29 (D-06 gate row 6, `make publish-dry-run`, 12/12
    dependency-ordered crates, zero test failures)"
  - "37-CI-EVIDENCE.md: Local sweep rows 30-31 (`make security`, `make api-surface`, both green,
    API-surface baseline unmoved)"
  - "37-CI-EVIDENCE.md: the closing Local sweep verdict tally (31 rows: 29 unconditional passes, 2
    named carried conditions, 0 CI-attributed) and the append-only 'Summary and what remains'
    addendum naming the 82% ADR-0006 coverage floor as CI-attributed"
affects: [37-05, 37-07, 37-09]

# Tech tracking
tech-stack:
  added: []
  patterns: []

key-files:
  created:
    - .planning/phases/37-v0-10-0-crate-release/37-03-SUMMARY.md
  modified:
    - .planning/phases/37-v0-10-0-crate-release/37-CI-EVIDENCE.md

key-decisions:
  - "Gate row 6 (`make publish-dry-run`) and both Task 2 adjacent checks (`make security`, `make
    api-surface`) were each hosted detached via target/37-03/run.sh, polled to completion with
    repeated foreground `timeout 560 tail --pid=... -f /dev/null` calls, and read from their own
    log/exit files — never run in the foreground, since the dry-run gate alone took 33m 32s,
    far past the Bash tool's 600s ceiling"
  - "The plan's own `<automated>` verify blocks were not run verbatim for either task; their
    conditions were asserted directly against the detached run's log/exit files instead, per this
    repo's own execution rules — disclosed as a deviation in both the evidence file and here"
  - "Before launching gate row 6, the 1-minute load average was polled at ~60s intervals (no
    process killed) until it settled from 4.38 to 1.70 over ~7 minutes, per the plan's own
    quiet-machine pitfall about `release-check` stacking `clean-code` immediately before the full
    workspace test suite"
  - "The '## Summary and what remains' section's existing plan-37-01-authored paragraphs were left
    byte-intact; this plan's own closing prose was appended below them as a dated addendum
    (D-00d), not a replacement — git diff --numstat confirms 0 deletions across both commits"

patterns-established: []

requirements-completed: []
# SHIP-05 is minted in plan 37-04 and spans all eleven plans in this phase (repo rule 9) — no
# requirements mark-complete step was run for this plan.

# Metrics
duration: ~53min
completed: 2026-09-18
status: complete
---

# Phase 37 Plan 03: D-06 Gate Row 6 (`make publish-dry-run`) and Local Sweep Close Summary

**Re-ran the heaviest of D-06's seven release gates — the full `release-check` chain plus a
twelve-crate dependency-ordered workspace publish dry run — cleanly on a cold `target/`, then
closed the Local sweep with `make security`/`make api-surface` and an honest 31-row verdict tally
that names the 82% ADR-0006 coverage floor as CI-attributed rather than claiming it as a local
pass.**

## Performance

- **Duration:** ~53 min
- **Started:** 2026-09-18T17:12:57Z
- **Completed:** 2026-09-18T18:05:48Z
- **Tasks:** 2/2
- **Files modified:** 1 (`37-CI-EVIDENCE.md`), plus this SUMMARY and STATE/ROADMAP metadata

## Accomplishments

- Re-measured free space (120G, well above the 40 GiB threshold) and porcelain state (clean)
  immediately before launching gate row 6, per the plan's own re-check instruction (plan 37-02's
  numbers are a baseline, not a licence).
- Applied the plan's quiet-machine pitfall: polled the 1-minute load average at ~60s intervals
  before launching `make publish-dry-run`, waiting ~7 minutes (no process killed) for it to settle
  from `4.38` to `1.70`.
- Hosted `make publish-dry-run` detached at `target/37-03/g6.{log,exit,pid}`; it ran
  `2026-09-18T17:20:22Z`–`17:53:54Z` (33m 32s, cold-build timings — a genuine 13m 12s release-profile
  build and an 8m 02s longest single incremental compile step both appear in the log), exit `0`.
- Confirmed the `release-check` leg's zero-failed-test statement (all 40 `test result:` lines `ok`),
  `cargo audit`'s 10 allowed pre-existing warnings (no new advisory), 12/12 dry-run-abort lines from
  `cargo publish --workspace --dry-run` in dependency order, and `paladin-doc-examples`'s correct
  absence from the `Uploading` list (compiled and doc-tested, never uploaded).
- Hosted `make security` and `make api-surface` detached (after the dry run, per the plan's own
  ordering instruction to avoid stacking heavy gates); both exit `0` — `cargo deny`: `advisories ok,
  bans ok, licenses ok, sources ok`; API surface: 3959 items extracted, unchanged verdict,
  `.project/current-exports.txt` confirmed unmodified.
- Closed the `## Local sweep` section with a 31-row verdict tally (29 unconditional local passes, 2
  named carried conditions carried from plan 37-02, 0 rows claiming the CI-attributed coverage
  gate) and appended (not replaced, D-00d) a dated addendum to `## Summary and what remains` naming
  the 82% ADR-0006 floor as CI-attributed, with Docker's absence as the structural reason, and
  naming plans 37-07/37-09 as the sources for the pre-merge and post-merge CI runs.
- Confirmed `git status --porcelain` empty throughout both tasks — no source file, manifest, or
  lockfile touched; `.project/current-exports.txt` unmoved.

## Task Commits

Each task was committed atomically:

1. **Task 1: Gate row 6 — `make publish-dry-run` on a clean tree with recorded headroom** —
   `d2db617b` (docs)
2. **Task 2: Adjacent checks and the Local sweep's closing verdict tally** — `aa22a101` (docs)

**Plan metadata:** recorded in the final metadata commit following this SUMMARY.

## Files Created/Modified

- `.planning/phases/37-v0-10-0-crate-release/37-CI-EVIDENCE.md` — appended the Task 1 head-SHA
  note, Local sweep row 29 (gate row 6, `make publish-dry-run`) with its deviation disclosure; the
  Task 2 head-SHA note, Local sweep rows 30-31 (`make security`, `make api-surface`); the closing
  Local sweep verdict tally; and a dated, appended addendum to `## Summary and what remains`. Zero
  lines from any prior plan's commits were modified or removed (`git diff --numstat` on both this
  plan's commits: `37 0` then `83 0` — additions only).
- `.planning/phases/37-v0-10-0-crate-release/37-03-SUMMARY.md` — this file (new).

## Decisions Made

- **Both gates hosted detached, exactly once each, per this repo's execution rules.** `make
  publish-dry-run` (33m 32s), `make security` (5s), and `make api-surface` (2m 46s) were each
  launched via `nohup target/37-03/run.sh <name> <command> &` followed by a separate `echo $! >
  target/37-03/<name>.pid` statement (no combined cleanup-plus-launch compound, avoiding the pid-race
  plan 37-02 hit), then polled to completion with repeated foreground `timeout 560 tail --pid=...
  -f /dev/null` calls. Real exit codes and full logs were read from disk, never inferred.
- **Quiet-machine wait before gate row 6, not skipped.** The plan's own pitfall 2 (`release-check`
  stacks `clean-code` immediately before the full workspace test suite, a previously-observed local
  timeout pattern) was honored: load was polled and allowed to settle to `1.70` before launch,
  recorded in the evidence file as readings, not summarized away.
- **`## Summary and what remains` appended to, not replaced.** Per repo rule 8 and the plan's own
  D-00d discipline, plan 37-01's original two paragraphs in that section were left byte-intact; this
  plan's closing prose was added as a dated addendum below them. `git diff --numstat` confirms 0
  deletions across both of this plan's commits.
- **The plan's `<automated>` verify blocks were not run verbatim, for both tasks.** Running them as
  written would have exceeded the Bash tool's 600s ceiling (gate row 6 alone took 33m 32s) and would
  have hosted the gates outside the mandated detached protocol — a second, uncounted run of gates
  this plan's own rules say run exactly once. The same conditions the verify blocks assert (dry-run
  abort count, zero failed tests, `paladin-doc-examples` absence; `make security`/`make api-surface`
  exit codes; baseline-unmoved check) were instead asserted directly against the detached runs' own
  log/exit files after each single run completed. No gate's pass/fail meaning was altered.

## Deviations from Plan

- **[Substitution, disclosed per plan's own house rules]** Neither task's `<automated>` verify block
  was run verbatim; each was substituted with direct assertions against the one detached run's
  log/exit files, for the reasons given under "Decisions Made" above. Disclosed both in
  `37-CI-EVIDENCE.md` (inline, immediately after Local sweep row 29 and again for rows 30-31's
  section) and here. No gate's pass/fail meaning was altered by either substitution — the same
  facts were confirmed either way.

No Rule 1-4 auto-fixes were needed or applied. No red gate was encountered in either task's actual
measured output. No architectural change, no dirty-tree refusal, no ENOSPC, and no network failure
occurred during this plan's execution.

## Issues Encountered

- A `cargo-clippy` process (rust-analyzer's own IDE background check, per repo rule 12 — never
  killed) was found running immediately before Task 2's commit. Per repo rule 1's pre-commit check
  protocol, the commit was deferred and polled (`pgrep -x cargo-clippy` / `pgrep -x pre-commit`)
  until the process exited on its own (~4 polls, ~75s), then the commit proceeded normally through
  the project's own pre-commit hook without contention. No process was killed; no workaround was
  applied beyond waiting.

## User Setup Required

None — no external service configuration required by this plan.

## Next Phase Readiness

- All seven of D-06's gate rows now carry at least one Local sweep entry in `37-CI-EVIDENCE.md`
  (rows 1-2 → gate 1; row 14 → gate 2; row 15 → gate 3; rows 17-28 → gate 4; row 16 → gate 5; row
  29 → gate 6; rows 4-13 → gate 7), plus rows 30-31 for the two adjacent house-sweep checks.
- The Local sweep is closed with an honest 31-row verdict tally and a `## Summary and what remains`
  addendum that names the 82% ADR-0006 coverage floor as CI-attributed, never claimed as a local
  pass — the structural reason (no Docker in this devcontainer) is stated plainly, and plans 37-07
  (pre-merge CI figure) and 37-09 (post-merge CI run) are named as the sources still outstanding.
- `.project/current-exports.txt` is unmoved — the pre-push API-surface gate plan 37-06 relies on
  will not see drift introduced by this plan.
- `target/37-03/{g6,security,apisurface}.{log,exit,pid}` remain on disk (git-ignored) for any
  future audit of these three runs.
- Nothing was pushed, tagged, or published; `cargo publish` was never invoked without `--dry-run`;
  no registry credential was requested, echoed, or stored.

## Self-Check

**Files:**
- FOUND: `.planning/phases/37-v0-10-0-crate-release/37-CI-EVIDENCE.md`
- FOUND: `.planning/phases/37-v0-10-0-crate-release/37-03-SUMMARY.md` (this file)
- FOUND: `target/37-03/g6.log`, `target/37-03/g6.exit` (reads `0`)
- FOUND: `target/37-03/security.log`, `target/37-03/security.exit` (reads `0`)
- FOUND: `target/37-03/apisurface.log`, `target/37-03/apisurface.exit` (reads `0`)

**Commits:**
- FOUND: `d2db617b` (Task 1)
- FOUND: `aa22a101` (Task 2)

**Diff integrity:** `git diff --numstat d2db617b~1 d2db617b` reports `37 0`; `git diff --numstat
aa22a101~1 aa22a101` reports `83 0` — both additions only, zero deletions, confirming no line
written by any prior plan or by this plan's own earlier task was modified or removed.

**Gate results:** `target/37-03/g6.exit` reads `0`; `target/37-03/security.exit` reads `0`;
`target/37-03/apisurface.exit` reads `0`; `git status --porcelain` empty; `git status --porcelain
-- .project/current-exports.txt` empty.

## Self-Check: PASSED

---
*Phase: 37-v0-10-0-crate-release*
*Completed: 2026-09-18*
