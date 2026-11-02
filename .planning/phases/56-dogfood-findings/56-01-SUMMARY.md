---
phase: 56-dogfood-findings
plan: 01
subsystem: release-gates
tags: [shell-gate, findings-log, dogf-01, tdd, python3-stdlib, makefile]
status: complete

requires:
  - phase: 52-ci-release-pipeline-repair
    provides: "the offline shell-gate house pattern (scripts/check-*.sh + tests/scripts/*_test.sh + make test-shell-guards glob)"
provides:
  - "scripts/check-findings-log.sh: offline, check-only gate that reads one findings log and exits 0 only when every ### DF-nn entry's LAST Disposition bullet is a final form"
  - "tests/scripts/check-findings-log_test.sh: 43-assertion fixture harness, auto-run by make test-shell-guards"
  - "make check-findings-log, deliberately outside check-gates and every workflow"
  - "a recorded red run against the real log (DF-01..DF-04 OPEN)"
affects: [56-04 prove-then-lock dispositions, 56-10 close gate, phase 58 SHIP-10]

tech-stack:
  added: []
  patterns:
    - "A gate whose real input is legitimately red mid-phase: the harness asserts parseability of the real input (parsed N >= 4, no ZERO_ENTRIES/UNREADABLE) and never its exit status; every pass/fail assertion uses a fixture through a test-only path seam"
    - "Last-bullet-wins append-only log parsing with a fence flag, so an unclosed fence is a named failure rather than a way to hide later entries"

key-files:
  created:
    - scripts/check-findings-log.sh
    - tests/scripts/check-findings-log_test.sh
  modified:
    - Makefile

key-decisions:
  - "PD-01-1..PD-01-5 followed as planned: seam FINDINGS_LOG_PATH is a file path; bare 'in v0.11.1' is NO_POINTER; status precedence UNREADABLE, UNCLOSED_FENCE, ZERO_ENTRIES, BAD, OK; ids compare as integers with DUPLICATE_ID beating NOT_INCREASING; the milestone keyword is the single literal token 'in v0.11.1'"
  - "On failure the wrapper prints its two guidance lines BEFORE the detail block so the final line of output is always 'parsed N entr(y|ies) from <path>' (satisfies both PD-01-3 and the acceptance criterion that the last line matches that pattern)"
  - "'**out** (reason)' and 'duplicate of **DF-01**' tolerate emphasis the same way 'in v0.11.1' does (leading * and backticks stripped from the remainder); a small robustness addition, pinned by harness cases"

patterns-established:
  - "Real-log parse case in a gate harness: count only, never exit status (P12)"

requirements-completed: []
requirements-advanced: [DOGF-01]

duration: ~20 min
completed: 2026-11-02
---

# Phase 56 Plan 01: Findings-log disposition checker Summary

**An offline `make check-findings-log` gate (python3 stdlib inside the house bash wrapper) that fails naming every `### DF-nn` entry whose last Disposition bullet is open, missing, malformed, mis-targeted or mis-ordered, fails loudly instead of passing vacuously, and is proven by a 43-assertion fixture harness that runs in `make test-shell-guards` while the real log is still red.**

## Plan-start SHA

`3f5cc1e63affae48a81c37ca96b5cd575e3f201b`

## Performance

- **Tasks:** 2 of 2 (Task 1 is the tracer; its `<verify>` passed end to end in auto mode, so execution expanded: Tracer verified end-to-end, expanding)
- **Commits:** `340fb02a3` (Task 1), `c5558a29d` (Task 2)
- **Files:** 2 created (`scripts/check-findings-log.sh`, `tests/scripts/check-findings-log_test.sh`), 1 modified (`Makefile`, one new target block; the `check-gates:` and `test-shell-guards` lines are untouched)
- **Disk:** no cargo build was run; python3 and bash only

## Accomplishments

- **Checker.** `scripts/check-findings-log.sh` takes no argument, reads one file (default `.project/v0.11.1/02-dogfood-findings-log.md`, relocatable only through the test seam `FINDINGS_LOG_PATH`), writes nothing, and exits 0 only when every entry's last `**Disposition…:**` bullet reads `in v0.11.1 — <pointer>`, `out (<reason>)` or `duplicate of DF-nn`. Per-entry tokens: `OPEN`, `MISSING`, `NO_POINTER`, `BAD_OUT`, `BAD_DUP_TARGET`, `UNRECOGNISED`, `DUPLICATE_ID`, `NOT_INCREASING`. File-level tokens: `UNREADABLE`, `UNCLOSED_FENCE`, `ZERO_ENTRIES`. Output is sorted by line and byte-identical across runs.
- **Harness.** 43 assertions across fire cases for every token (23 `assert_fire` calls, several looped over multiple forms), the named failures, determinism, sort order, no-mutation of a fixture and of the real log, and the silent forms (emphasis, indented continuation value, nested-parenthesis `out`, `duplicate of` by integer id, a heading straight after a fence close, a `####` sub-heading inside an entry).
- **Wiring.** `make check-findings-log` runs the script. It is not a prerequisite of `check-gates` and no file under `.github/workflows/` names it. The harness is picked up by the existing `tests/scripts/*_test.sh` glob (`make test-shell-guards` output names it; exit 0).

## Task Commits

1. **Task 1: tracer, real log through `make check-findings-log` reports DF-01..DF-04 OPEN** - `340fb02a3` (feat)
2. **Task 2: full disposition grammar, id rules and named failures proven on fixtures** - `c5558a29d` (test)

## Watched-failing runs (TDD)

- **Task 1.** Harness written first; `bash tests/scripts/check-findings-log_test.sh` with no guard on disk:
  `ERROR: guard script not found at /home/user/paladin-dev-env/scripts/check-findings-log.sh`, exit 1.
- **Task 2.** The 37 new cases were added to the harness before the classifier changed. Against the Task 1 script the run ended `❌ 18/43 assertion(s) failed.`, failing exactly the new behaviour: five `BAD_OUT` forms, two `BAD_DUP_TARGET` forms, four `DUPLICATE_ID`/`NOT_INCREASING` cases, `UNCLOSED_FENCE`, the multi-finding listing (`DF-03: BAD_OUT` missing), and the five silent forms that need `out` or `duplicate of` (nested-parenthesis `out`, emphasised `out`, `duplicate of DF-01`, `duplicate of DF-1`, the `####` sub-heading case). After the classifier change: `✅ 43 assertion(s) passed.`

## Checker red run (real log)

`./scripts/check-findings-log.sh`, on the real log as it stood at plan start (unchanged by this plan; `cksum` `1736150183 5766` before and after). **Exit code: 1.**

```
🔍 Checking the findings log for entries without a final disposition ...
❌ Findings-log check failed (BAD)

If this failure is unexpected:
  1. Append a dated '- **Disposition (YYYY-MM-DD):**' bullet under the entry
     (in v0.11.1 — <pointer>, out (<reason>) or duplicate of DF-nn). Never
     rewrite an earlier line; the log is append-only (D-01).
  2. This gate never edits the log. It is run at phase verification and by
     SHIP-10, not by check-gates or CI.

.project/v0.11.1/02-dogfood-findings-log.md:61: DF-01: OPEN (the last Disposition bullet reads 'open')
.project/v0.11.1/02-dogfood-findings-log.md:73: DF-02: OPEN (the last Disposition bullet reads 'open')
.project/v0.11.1/02-dogfood-findings-log.md:84: DF-03: OPEN (the last Disposition bullet reads 'open')
.project/v0.11.1/02-dogfood-findings-log.md:93: DF-04: OPEN (the last Disposition bullet reads 'open')
parsed 4 entries from .project/v0.11.1/02-dogfood-findings-log.md
```

The Task 2 re-run after the full classifier landed printed the identical four `OPEN` lines and `parsed 4 entries`. This red is expected until plan 56-04 appends dated closing bullets under DF-01..DF-04.

## Final harness footer

```
✅ 43 assertion(s) passed.
```

`shellcheck --severity=warning` on both files: clean. `make lint-shell`: `✅ shellcheck clean`. `make test-shell-guards`: exit 0, names `tests/scripts/check-findings-log_test.sh`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Harness `line_of` matched the fenced template, not the entry bullet**
- **Found during:** Task 1, first green attempt
- **Issue:** the OPEN needle used `line_of '- **Disposition:** open'`, whose first hit is the template line `- **Disposition:** open | in v0.11.1 | …` at line 5, not the entry bullet at line 12.
- **Fix:** added `last_line` (the disposition bullet is the last line of every baseline fixture) and used it for the needle.
- **Files modified:** `tests/scripts/check-findings-log_test.sh`
- **Commit:** `340fb02a3` (fixed before the commit)

**2. [Rule 1 - Bug] shellcheck SC2034 on an unused `PLANT_LINE`**
- **Found during:** Task 1 lint
- **Issue:** the plan's `plant` helper sets `PLANT_LINE`, which nothing read until Task 2.
- **Fix:** dropped it from `plant`; Task 2 gets the line number it needs from `plant_entry` (`HEADING_LINE`) and `last_line`.
- **Files modified:** `tests/scripts/check-findings-log_test.sh`
- **Commit:** `340fb02a3`

**3. [Plan reading] Guidance lines print before the detail block**
- The plan's bash tail (copied from `check-version-strings.sh`) prints detail then guidance, while PD-01-3 and the Task 1 acceptance criterion require the last line of output to be `parsed N entr…`. The two statements conflict; printing guidance first satisfies both. No behaviour other than ordering differs.

**4. [Rule 2 - small robustness] Emphasis tolerated around `out` and the `duplicate of` target**
- `**out** (reason)` and `duplicate of **DF-01**` are accepted the way `**in v0.11.1** — …` is, so a maintainer's markdown emphasis cannot flip a valid disposition into `BAD_OUT` / `BAD_DUP_TARGET`. Pinned by the `out-simple` and `dup-of-*` harness cases.

None of these changes a decision (D-01..D-03, PD-01-1..5).

### Requirement bookkeeping

**5. [Judgement] DOGF-01 deliberately NOT marked complete**
- `requirements.mark-complete DOGF-01` ran and was reverted in `.planning/REQUIREMENTS.md`. DOGF-01 reads "every DF-nn entry carries a disposition; none is open". This plan delivers the command that proves it, not the dispositions: the checker's own red run shows DF-01..DF-04 still OPEN. Marking it complete would be the false assurance the checker exists to prevent. Plan 56-10's close gate (which lists DOGF-01) marks it once `make check-findings-log` exits 0.

## Issues Encountered

None beyond the deviations above.

## Known Stubs

None. Both files are complete; no placeholder text, empty-value UI wiring or skipped test.

## Threat Flags

None. The only new surface is a local, read-only parser. Threat-register mitigations applied: T-56-01 (`ZERO_ENTRIES`, `UNCLOSED_FENCE`, `UNREADABLE`, the final `parsed N` line, a fixture per name); T-56-02 (`BAD_DUP_TARGET`, `NO_POINTER`, `BAD_OUT`, last-bullet-wins fixtures in both directions); T-56-03 (log path passed as an argument to a quoted heredoc, anchored regexes only, no `eval`, shellcheck clean); T-56-04 (not in `check-gates` or any workflow; the harness's real-log case asserts parseability only); T-56-05 (no write mode; no-mutation checksum cases on a fixture and on the real log).

## Self-Check: PASSED

- FOUND: `scripts/check-findings-log.sh` (mode `100755`)
- FOUND: `tests/scripts/check-findings-log_test.sh` (mode `100755`)
- FOUND: `Makefile` target `check-findings-log:` (not in the `check-gates:` line; `grep -rl check-findings-log .github/workflows` prints nothing)
- FOUND: commits `340fb02a3` and `c5558a29d`
- Real log `cksum` identical before and after: `1736150183 5766`
