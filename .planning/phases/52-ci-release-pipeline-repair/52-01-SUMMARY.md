---
phase: 52-ci-release-pipeline-repair
plan: 01
subsystem: infra
tags: [ci, github-actions, cargo-semver-checks, shell-guards, release-gates]

requires:
  - phase: 37.1
    provides: "scripts/publish-crates.sh CRATES array and the PUBLISH_CRATES_LIB_ONLY sourcing seam; check-publish-order.sh sourcing precedent"
provides:
  - "scripts/check-semver.sh: single implementation of the semver gate (CI and local) with a zero-check tripwire"
  - "tests/scripts/check-semver_test.sh: stubbed-cargo regression harness (70 assertions)"
  - "ci.yml semver job renamed 'Semver Checks (vs v0.11.0)' and calling the script"
  - "make check-semver (deliberately not part of check-gates)"
  - "migration-allowlist gate runs from the required License & Dependency Policy job on every push and PR"
affects: [52-05, 52-06, 52-07, ruleset-contexts]

tech-stack:
  added: []
  patterns:
    - "Gate logic lives in a tested script; the workflow step is a one-line call (no inline awk/loops)"
    - "Package list read from scripts/publish-crates.sh via PUBLISH_CRATES_LIB_ONLY=1 source, never hand-kept"
    - "A tool verdict is never trusted without a parsed non-zero work count (tripwire)"

key-files:
  created:
    - scripts/check-semver.sh
    - tests/scripts/check-semver_test.sh
  modified:
    - .github/workflows/ci.yml
    - Makefile
    - scripts/check-migration-allowlist.sh

key-decisions:
  - "PD-01-1: semver loop moved out of inline YAML into scripts/check-semver.sh so the zero-check tripwire has a regression test"
  - "PD-01-3: allowlist script runs in the required cargo-deny job, not the semver job (semver is not in the ruleset today)"
  - "PD-01-4: placeholder-marker step kept unchanged and moved into the same required job"
  - "--release-type patch is fixed by flag, never inferred from the version delta (manifests equal the 0.11.0 baseline)"

patterns-established:
  - "Zero-check tripwire: no 'N checks:' line, or N == 0, is a named failure"

requirements-completed: [CIREL-01, CIREL-03]

coverage:
  - id: D1
    description: "Semver gate script checks every CRATES member against 0.11.0 at --release-type patch and fails on zero checks or no count"
    requirement: "CIREL-01"
    verification:
      - kind: unit
        ref: "tests/scripts/check-semver_test.sh (70 assertions)"
        status: pass
      - kind: integration
        ref: "SEMVER_PRUNE_LOCAL_CACHE=1 ./scripts/check-semver.sh --package paladin-eval"
        status: pass
    human_judgment: false
  - id: D2
    description: "ci.yml semver job named 'Semver Checks (vs v0.11.0)' runs ./scripts/check-semver.sh with cargo-semver-checks@0.50.0 unchanged; make check-semver calls the script"
    requirement: "CIREL-01"
    verification:
      - kind: other
        ref: "python3 yaml assertions on ci.yml jobs.semver; make -n check-semver"
        status: pass
    human_judgment: false
  - id: D3
    description: "check-migration-allowlist.sh runs from the required License & Dependency Policy job with no job-level if; inline awk step deleted; placeholder-marker step relocated beside it"
    requirement: "CIREL-03"
    verification:
      - kind: other
        ref: "python3 yaml assertions on ci.yml jobs.cargo-deny / jobs.semver"
        status: pass
      - kind: unit
        ref: "tests/scripts/check-migration-allowlist_test.sh (16 assertions)"
        status: pass
    human_judgment: false
  - id: D4
    description: "actionlint acceptance of the edited ci.yml"
    requirement: "CIREL-01"
    verification: []
    human_judgment: true
    rationale: "actionlint is unavailable in this sandbox (A-52-03); CI's required Workflow Lint job is the gate on the pushed branch"

duration: 40min
completed: 2026-10-28
status: complete
---

# Phase 52 Plan 01: Semver gate re-pin and allowlist gate relocation Summary

**One tested `scripts/check-semver.sh` now runs every published crate against the 0.11.0 baseline at `--release-type patch` and fails on zero checks, and the migration-allowlist gate runs from the required `License & Dependency Policy` job.**

## Performance

- **Duration:** about 40 min (Task 1 by the previous executor, Task 2 and close-out by the continuation)
- **Completed:** 2026-10-28
- **Tasks:** 2 of 2 (Task 1 was the tracer)
- **Files:** 2 created, 3 modified

## Accomplishments

- CIREL-01: `scripts/check-semver.sh` reads the twelve published package names from `scripts/publish-crates.sh` (`paladin-eval` included), runs `cargo semver-checks check-release --package <pkg> --default-features --baseline-version 0.11.0 --release-type patch` for each, in `CRATES` order, runs every package before the verdict, and fails a package that printed no `N checks:` line, or N = 0, by name. It emits one `::notice title=Semver check counts` annotation and one `::error` per failing package under GitHub Actions.
- CIREL-01: the CI job is `Semver Checks (vs v0.11.0)`, its check step is `./scripts/check-semver.sh`, the `taiki-e/install-action@v2` pin `cargo-semver-checks@0.50.0` is unchanged, and no `0.9.0` string remains in the `msrv`..`test` region. `make check-semver` runs the same script and is deliberately outside `check-gates` (it builds rustdoc and downloads baselines).
- CIREL-03 / D-06: the inline awk set-equality step is gone from `ci.yml` (0 occurrences of `migration-deliberate-pairs`). The required `License & Dependency Policy` job (no job-level `if:`) runs `./scripts/check-migration-allowlist.sh` as `Check semver allowlist is set-equal to the MIGRATION.md register`, with the placeholder-marker step (name and body unchanged) directly after it. The script header now describes it as the single CI implementation; no logic changed, the `(9|10)` window and its strings are untouched for plan 52-05.

## Task Commits

1. **Task 1: One end-to-end semver path (tracer)** - `bd1665e4b` (ci)
2. **Task 2: Run check-migration-allowlist.sh from the required cargo-deny job** - `be5867729` (ci)

Metadata commit: see git log (docs(52-01)).

## Tracer evidence (Task 1)

- **Red first:** harness run with `scripts/check-semver.sh` absent printed `ERROR: guard script not found` and exited 1.
- **Green:** `./tests/scripts/check-semver_test.sh` reports `✅ 70 assertion(s) passed.` (re-confirmed after Task 2).
- **Mutation check:** a temporarily mutated script failed 8 assertions (zero-check handling) and 2 (release-type flag); the script was restored byte-identical.
- **Live probe against crates.io 0.11.0:** `SEMVER_PRUNE_LOCAL_CACHE=1 ./scripts/check-semver.sh --package paladin-eval` exited 0 and printed `semver: paladin-eval: 223 checks: 223 pass, 31 skip` and `semver: 1 package(s) checked against 0.11.0 at --release-type patch; 0 failure(s)`; the tool line read `Checking paladin-eval v0.11.0 -> v0.11.0 (assume patch change)`.
- **Tracer gate:** the orchestrator verified this evidence and approved continuation to Task 2.

## Verification (plan level, run after Task 2)

- `./tests/scripts/check-semver_test.sh`: 70 assertions passed.
- `./tests/scripts/check-migration-allowlist_test.sh`: 16 assertions passed. `./scripts/check-migration-allowlist.sh` on the real tree: set-equal pairs plus the crate-wide-allow mirror line.
- `./scripts/check-workflow-suppressions.sh` and `./scripts/check-workflow-triggers.sh` pass on the real tree after the commit; their harnesses pass (21 and 11 assertions).
- `make check-gates` exit 0; `make test-shell-guards` exit 0.
- `shellcheck --severity=warning` clean on `scripts/check-semver.sh`, `tests/scripts/check-semver_test.sh`, `scripts/check-migration-allowlist.sh`.
- `git ls-files -s`: both new scripts are mode `100755`.
- Python YAML assertions on `ci.yml` (job name, step, pin, relocated steps, no `awk` in `semver`, no `if` on `cargo-deny`) pass.
- Not run: `actionlint` (unavailable in this sandbox, A-52-03). CI's required `Workflow Lint` job is the gate once the branch is pushed. Recorded in the windows ledger as an unrun verify.

## Decisions Made

- Followed PD-01-1 through PD-01-5 as planned; see `key-decisions` above.
- `--release-type patch` is a fixed flag because every manifest equals the baseline (0.11.0), so tool inference would be meaningless (RESEARCH Pitfall 1).

## Deviations from Plan

None - plan executed exactly as written. Two cosmetic follow-ups inside Task 2's own edit (a missing blank line before the inserted step and a doubled blank line where the relocated block was removed) were corrected before the commit.

## Issues Encountered

None. The workflow-suppression and workflow-trigger harnesses were run only after the Task 2 commit, as they require a clean `.github/workflows/` tree.

## Known Stubs

None.

## Threat Flags

None. No new network endpoint, auth path or trust-boundary file access was introduced beyond the plan's threat model (T-52-01 to T-52-05 mitigated as planned; no `${{ }}` expression in any new `run:`).

## Next Phase Readiness

- The `Semver Checks (vs v0.11.0)` display name is in place for plan 52-06's ruleset context.
- Plan 52-05 can retarget the `(9|10)` window in `scripts/check-migration-allowlist.sh`; this plan left it untouched.
- Plan 52-07 can read the semver evidence from the `Semver check counts` annotation.
- Pending on push: CI `Workflow Lint` confirmation of the edited `ci.yml`.

## Self-Check: PASSED

Files verified present: `scripts/check-semver.sh`, `tests/scripts/check-semver_test.sh`, `.github/workflows/ci.yml`, `Makefile`, `scripts/check-migration-allowlist.sh`. Commits `bd1665e4b` and `be5867729` verified in git log.
