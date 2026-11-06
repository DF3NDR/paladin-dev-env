---
phase: 54-first-run-usability
plan: 07
subsystem: infra
tags: [shell-gate, version-strings, makefile, ci, first-run, check-gates]
status: complete

requires:
  - phase: 54-first-run-usability
    provides: "54-02..54-06 sweeps: every copyable string in README, docs/src, k8s/ and openapi.json reads the workspace version"
provides:
  - "scripts/check-version-strings.sh: offline, no-argument, check-only gate over four copyable-string classes"
  - "tests/scripts/check-version-strings_test.sh: 37-assertion prove-it-fails harness on mktemp fixture trees"
  - "make check-version-strings, a member of make check-gates"
  - "ci.yml step 'Check user-facing version strings (FIRST-03)' in the required cargo-deny job"
  - "count-free Makefile check-consumer-surface comment and ci.yml MSRV comment"
affects: [54-08, 54-09, phase 58 lockstep version bump]

tech-stack:
  added: []
  patterns:
    - "Gate never passes vacuously: ZERO_FILES, CLASS_UNMATCHED, STALE_EXEMPTION, NO_VERSION are named failures, each pinned by an assert_fire case"
    - "Test-only VERSION_STRINGS_ROOT seam relocates every input to a fixture tree while the script takes no argument"

key-files:
  created:
    - scripts/check-version-strings.sh
    - tests/scripts/check-version-strings_test.sh
  modified:
    - Makefile
    - .github/workflows/ci.yml

key-decisions:
  - "PD-07-1: status line is the first applicable of NO_VERSION, STALE_EXEMPTION, ZERO_FILES, CLASS_UNMATCHED, MISMATCH, OK, but the detail lists every named failure and every finding"
  - "Class 3 image candidates are tags that start with an optional v, digits, a dot and digits, so paladin-server:v1.2.3 is a candidate and fails the exact comparison while :test and :latest are not candidates"
  - "Class 4 reports the first line at or after the one holding info that names version, so a single-line openapi.json still yields a line"

requirements-completed: [FIRST-03, FIRST-02, FIRST-04, FIRST-07]

duration: ~25min
completed: 2026-11-01
---

# Phase 54 Plan 07: check-version-strings gate Summary

**An offline, check-only gate holds the copyable Cargo pins, `Current version:` lines, k8s labels and image tags, and `openapi.json` `info.version` to the workspace version (0.11.0). It is a member of `make check-gates` and has its own step in the required CI job. It cannot pass by scanning or matching nothing.**

## Performance

- **Plan-start SHA:** `5179e8aec79bb850e3e2eb298b561f85ff24fc29`
- **Tasks:** 3 (1 tracer, 2 auto)
- **Files:** 2 created, 2 modified

## Accomplishments

- `scripts/check-version-strings.sh` reads the wanted version from the root `Cargo.toml` with `tomllib` and the 13 workspace package names from the root and every `crates/*/Cargo.toml`. It takes no argument and has no write mode. It reports every finding as `<path>:<line>: found 'X', want 'Y'`, sorted by path then line, with byte-identical output across runs.
- Four classes: (1) Cargo pins as bare strings, inline tables, `package =` aliases, multi-line inline-table opening lines and `//!` lines; (2) `Current version:` lines; (3) `k8s/` `version:` labels and `paladin-server` / `paladin` image tags; (4) `info.version`.
- The `EXEMPT` list holds exactly one entry, `docs/src/api-reference/migration-guide.md`, with its reason. The harness proves the same `"0.4"` content fails in any other scanned file.
- `make check-version-strings` exists and is the last `check-gates` prerequisite. The CI step sits directly after the dependency-allowlist step, inside `cargo-deny:`, before `Run shell-guard regression tests`.
- The two count comments (Makefile "the nine members", ci.yml MSRV "all ten crate manifests") are count-free.

## Task Commits

1. **Task 1 (tracer): class-1 pins, harness, script, make and CI wiring** - `6bbcee914`
2. **Task 2: classes 2-4, named never-vacuous failures, edge cases** - `d6d44dca8`
3. **Task 3: count-free gate comments, full shell-guard run** - `094a5dda2`

## Evidence

**Prove-then-lock.** Before any change `make -n check-gates | grep -c check-version-strings` printed `0`, and `grep -n 'nine members' Makefile` / `grep -n 'all ten' .github/workflows/ci.yml` each gave one hit (Makefile l.243, ci.yml l.325).

**Watched failing, Task 1.** With no guard script, the harness exited 1:

```
ERROR: guard script not found at /home/user/paladin-dev-env/scripts/check-version-strings.sh
exit=1
```

**Watched failing, Task 2.** With the Task 1 script and the new cases added, `13/37 assertion(s) failed`, for example `FAIL: expected non-zero exit for: class 2: a stale Current version line in README.md (got 0)`, the class 3 and class 4 stale cases, the three exact-match cases and the four CLASS_UNMATCHED / STALE_EXEMPTION cases. After the script was extended: `37 assertion(s) passed`.

**Tracer feedback gate (auto mode).** Re-ran the tracer's verify end to end before expanding: harness, gate, `make check-gates`, `make -n` membership, CI grep and shellcheck all passed (`Tracer verified end-to-end -- expanding`).

**Real-tree OK line:**

```
✅ OK: 128 file(s) scanned, every copyable version string equals '0.11.0' (class 1 Cargo pins: 42; class 2 Current version lines: 1; class 3 k8s labels and image tags: 8; class 4 openapi.json info.version: 1; exempt files: 1)
```

**Committed modes** (`git ls-files -s`): `100755` for both `scripts/check-version-strings.sh` and `tests/scripts/check-version-strings_test.sh`.

**Final runs on a clean tree after Task 3:** `make test-shell-guards` exit 0 with no `FAIL` line (the new harness included, 37 assertions), `make lint-shell` clean, `make check-gates` exit 0, the gate `OK`, `ci.yml` parses as YAML.

**Acceptance checks:** `check-gates:` line matches `^check-gates:.*check-version-strings`; in `ci.yml` the new step (l.181) is after the dependency-allowlist step (l.174) and before `Run shell-guard regression tests` (l.224), all inside `cargo-deny:` (l.125); `grep -nE '\-\-write|getopts|case "\$1"'` on the script finds nothing; the `EXEMPT` literal names one `.md` path; the `ci.yml` non-comment diff against the plan-start SHA is only the new step's `- name:` and `run:` lines; the `release` recipe is untouched.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] shellcheck severity**
- **Found during:** Task 1
- **Issue:** The plan's verify runs bare `shellcheck`, which exits 1 on info and style findings (SC2317 on the cleanup trap, SC2001 on the `sed` indent helper, SC2016 on `printf` of backticks). The committed template harness `check-dependency-allowlist_test.sh` has the identical findings at default severity.
- **Fix:** Verified at `--severity=warning`, the level the repository's own `make lint-shell` and pre-commit gate use. Both new files exit 0 there, and `make lint-shell` is clean. No code was changed to chase info-level findings that the house template shares.
- **Files modified:** none

**2. [Rule 1 - Bug] Plan text contradiction on image tags**
- **Found during:** Task 2
- **Issue:** The plan says a tag that does not begin with "digits, a dot and digits" is not a candidate, and also that `paladin-server:v1.2.3` must fail the exact comparison. A candidate rule without the optional `v` could never report it.
- **Fix:** The candidate pattern is `^v?\d+\.\d+`. `test` and `latest` are still not candidates; `v1.2.3` is, and fails the exact comparison.
- **Files modified:** `scripts/check-version-strings.sh`
- **Commit:** `d6d44dca8`

No stale string turned up in the real-tree run, so no missed-sweep fix was needed. The `k8s/README.md` heading `### Worker replicas (Platform API, v0.10)` is not flagged (outside the four classes), and the full real-tree run is green.

### Notes

- `make test-shell-guards` run with an uncommitted edit under `.github/workflows/` fails the unrelated `check-workflow-suppressions` harness ("`.github/workflows/` was mutated by this test run"), because that harness reads `git status` for that directory. It passed once the Task 3 `ci.yml` comment edit was committed. It is not a defect of this plan; run the shell guards on a clean workflows tree.
- `cargo test` was not part of this plan's checks. The two known failing `run_api_wiring` tests are pre-existing and untouched.

## Known Stubs

None.

## Threat Flags

None. The gate adds no network endpoint, auth path or trust boundary. T-54-20 (vacuity), T-54-21 (rewrite) and T-54-22 (exemption growth) are mitigated by the named failures, the no-write design with fixture and real-tree `cksum` assertions, and the single `EXEMPT` entry. T-54-23 is accepted: the CI step and the Makefile target set no environment.

## Self-Check: PASSED

- FOUND: scripts/check-version-strings.sh (100755)
- FOUND: tests/scripts/check-version-strings_test.sh (100755)
- FOUND commits: 6bbcee914, d6d44dca8, 094a5dda2
