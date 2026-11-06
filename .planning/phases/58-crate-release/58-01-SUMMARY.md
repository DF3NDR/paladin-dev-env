---
phase: 58-crate-release
plan: 01
subsystem: release-infra
tags: [ci, pre-commit, sdk-smoke, npm, crates-io, sparse-index, release-evidence]

requires:
  - phase: 57-register-hygiene-close-out
    provides: the branch head and the register this release builds on
provides:
  - 58-CI-EVIDENCE.md with the pre-tag registry baseline (12 crates, all 0.11.0, none at 0.11.1) and findings F-58-01..03
  - 58-registry-probe.sh, a read-only sparse-index and Trusted Publishing probe over the cargo-metadata publishable set
  - the required pre-commit context green locally
  - a CI-side fix for the red SDK smoke (awaiting its CI read on SHA-E)
  - SHA-E pushed to origin without a label or pull request
affects: [58-05, 58-12, 58-13, 58-14]

tech-stack:
  added: []
  patterns:
    - "Publishable set derived from cargo metadata at run time, never listed; empty set or row-count mismatch is a named failure"
    - "Build a generated npm client in its own directory (own devDependencies) before linking it into a consumer"

key-files:
  created:
    - .planning/phases/58-crate-release/58-CI-EVIDENCE.md
    - .planning/phases/58-crate-release/58-registry-probe.sh
  modified:
    - scripts/sdk-smoke/run.sh
    - .planning/phases/57-register-hygiene-close-out/57-VALIDATION.md
    - .planning/phases/58-crate-release/58-PATTERNS.md

key-decisions:
  - "SDK smoke is not a required merge context but is a tag precondition (check-release-consistency clause 3 reads the whole ci.yml run); a red job is fixed or the tag is held, never accepted"
  - "SDK diagnosis branch taken: CI-side fix in scripts/sdk-smoke/run.sh only; no src/ or crates/ change"
  - "treasury_spend flake is a test-level flake, not an infrastructure flake under D-00j; its re-seal rule goes to the maintainer at plan 58-05"

patterns-established:
  - "Every new .planning document in this phase must pass pre-commit run --all-files before it is pushed"

requirements-completed: [SHIP-08, SHIP-10]

coverage:
  - id: D1
    description: "Registry probe reads the sparse index for every cargo-metadata publishable crate and fails on an empty set or a row-count mismatch"
    requirement: SHIP-08
    verification:
      - kind: other
        ref: "bash .planning/phases/58-crate-release/58-registry-probe.sh --version 0.11.1 --mode absent -> probe: 12 crate(s), mode absent, version 0.11.1: OK"
        status: pass
    human_judgment: false
  - id: D2
    description: "Required pre-commit context cleared (57-VALIDATION.md single trailing newline; 58-PATTERNS.md trailing space)"
    requirement: SHIP-08
    verification:
      - kind: other
        ref: "pre-commit run --all-files -> all ten hooks Passed"
        status: pass
    human_judgment: false
  - id: D3
    description: "SDK smoke CI-side fix: generated TypeScript client built in target/sdk/typescript before it is linked"
    requirement: SHIP-08
    verification:
      - kind: other
        ref: "Generated SDK Clients (Python + TypeScript) smoke job on SHA-E, read in plan 58-05"
        status: unknown
    human_judgment: false
  - id: D4
    description: "treasury_spend flake characterised under the CI job's flags"
    requirement: SHIP-10
    verification:
      - kind: other
        ref: "cargo test --workspace --lib --no-default-features --features cli ... single 0/30 failed, module 0/10 failed"
        status: pass
    human_judgment: false

duration: 85min
completed: 2026-11-05
status: complete
---

# Phase 58 Plan 01: CI health and registry baseline Summary

**Cleared the one red required context, opened the release evidence record with a probe-derived pre-tag registry baseline (12 crates at 0.11.0, none at 0.11.1), fixed the red SDK smoke on the CI side by building the generated TypeScript client in its own directory, measured the `treasury_spend` flake at 0/40 local failures, and pushed the pre-bump head `SHA-E`.**

## Performance

- **Duration:** about 85 min
- **Completed:** 2026-11-05
- **Tasks:** 3 (one tracer, two auto), executed across two agents (a checkpoint approved after Task 1)
- **Files modified:** 5 tracked files plus two created

## Accomplishments

- `58-registry-probe.sh` derives the publishable set from `cargo metadata` (12 names), reads each crate's sparse-index file with the mandatory `User-Agent`, and ends `probe: 12 crate(s), mode absent, version 0.11.1: OK`. It will be reused by plans 58-12, 58-13 and 58-14.
- The pre-tag baseline in `58-CI-EVIDENCE.md` records every crate's newest line `0.11.0`, `yanked` false, `has_0.11.1=no`.
- `pre-commit run --all-files` is green: `57-VALIDATION.md` now ends in one newline (F-58-01).
- SDK smoke diagnosed from logs (F-58-02): job `126917997570` and the newer `127022155203` both fail with `sh: 1: tsc: not found`, exit 127, inside the generated client's `prepare` during `npm install --no-save ../../target/sdk/typescript` on `ubuntu-26.04`; the green `v0.11.0` job `121733645764` ran on `ubuntu-24.04` with the same Node 20.20.2 and npm 10.8.2. The job is not among the 46 required contexts but is a tag precondition.
- `treasury_spend_prints_one_row_per_currency` (F-58-03): `single: 0/30 failed`, `module: 0/10 failed` under the CI job's own invocation; not reproduced, recorded as a test-level flake for the maintainer's re-seal rule at plan 58-05.
- `SHA-E` pushed with a plain push; its four runs were listed once and not waited for.

## Task Commits

1. **Task 1a: end 57-VALIDATION.md with a single newline** - `99ccf0aa3` (docs)
2. **Task 1b: trim one trailing space in 58-PATTERNS.md** - `2e1338a80` (docs, approved deviation, F-58-01)
3. **Task 1c: open 58-CI-EVIDENCE.md with the pre-tag registry baseline and the probe** - `0e7f0ed86` (docs)
4. **Task 2: build the generated TypeScript client before linking it into the SDK smoke** - `9166fe1fe` (ci); this is `SHA-E`
5. **Task 3: record the SDK-smoke diagnosis, the treasury_spend characterisation and the SHA-E push** - `23e1d3d17` (docs, local, rides the next push)

`SHA-PRE` is `7832f280e0376a8b8a10e91522927e1bf4d16339`; `SHA-E` is `9166fe1fee4a920baf7f481d34b6e05b40b7b3de`.

## Files Created/Modified

- `.planning/phases/58-crate-release/58-CI-EVIDENCE.md` - phase evidence record with baseline and findings F-58-01..03
- `.planning/phases/58-crate-release/58-registry-probe.sh` - read-only registry probe
- `scripts/sdk-smoke/run.sh` - builds the generated client in `target/sdk/typescript`, asserts its `main` file, links with `--ignore-scripts` after `npm ci`
- `.planning/phases/57-register-hygiene-close-out/57-VALIDATION.md` - one surplus trailing newline removed
- `.planning/phases/58-crate-release/58-PATTERNS.md` - one trailing space removed

## Decisions Made

- SDK smoke red job is fixed in the release PR or the tag is held; accepting it red cannot carry to the tag (D-05 as reworded).
- The SDK fix stays in `run.sh`; `smoke.py`, `smoke.ts`, `package.json` and `package-lock.json` are byte-identical to `SHA-PRE`.
- `--ignore-scripts` is kept as the plan requires, but it is not what makes the fix work (see deviations).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Trailing space in 58-PATTERNS.md**
- **Found during:** Task 1
- **Issue:** `trim trailing whitespace` failed on a file committed after RESEARCH read the tree, blocking the must-have that every hook passes
- **Fix:** one-byte whitespace change as its own commit; approved by the maintainer at the Task 1 checkpoint
- **Files modified:** `.planning/phases/58-crate-release/58-PATTERNS.md`
- **Commit:** `2e1338a80`

**2. [Rule 1 - Bug in plan assumption] `--ignore-scripts` does not suppress a directory dependency's `prepare` on local npm 10.9.4**
- **Found during:** Task 2 sandbox check
- **Issue:** a scratch package whose `prepare` exits 1 still failed with the `--ignore-scripts` link form, so the flag alone cannot be what cures the CI failure
- **Fix:** kept the flag as required, corrected the `run.sh` comment, and recorded in F-58-02 that the cure is the client's own `node_modules/.bin/tsc` (a `prepare` re-run at link time now resolves `tsc`). Confirmed with a scratch package whose `prepare` calls a binary in its own `node_modules/.bin` (linked, rc 0)
- **Commit:** `9166fe1fe`

**3. [Rule 3 - Blocking] The flake loop needs `--workspace`**
- **Found during:** Task 3
- **Issue:** `cargo test --lib --no-default-features --features cli` alone does not compile (the `deepseek` module is feature-gated); the CI job uses `--workspace`, whose feature unification supplies it
- **Fix:** ran the loop under the job's exact `cargo test --workspace --lib --no-default-features --features cli` form
- **Commit:** recorded in `23e1d3d17`

**4. [Rule 1 - Bug in tooling default] Requirement tick reverted**
- **Found during:** state updates
- **Issue:** `requirements.mark-complete SHIP-08 SHIP-10` ticked both, but SHIP-08 is proven only by the post-tag registry read (P-08-2) and SHIP-10 by the audit; this plan only prepares the ground
- **Fix:** reverted `.planning/REQUIREMENTS.md` to its committed state; plan 58-14 ticks them through the handler after the registry read

**Total deviations:** 4 (none touch `src/` or `crates/`). **Impact:** none on scope.

## Issues Encountered

- The newest `ci.yml` run on the branch (`41959351837` on `adf9472ea`) was still `in_progress` when read; its SDK job `127022155203` had already failed identically. Recorded in F-58-02.
- The reason `ubuntu-24.04` found a `tsc` is not visible in the green log (RESEARCH assumption A1, not reproduced). The proof of the fix is the SDK job on `SHA-E`, read in plan 58-05; if it is still red there, a second CI cycle is budgeted.

## User Setup Required

None.

## Known Stubs

None.

## Threat Flags

None. No new network endpoint, auth path or trust-boundary file access was introduced; the only remote write was a plain push of the release branch (T-58-01) and no `src/` or `crates/` file changed (T-58-02: `git diff --name-only SHA-PRE HEAD -- src crates` is empty).

## Next Phase Readiness

- Plan 58-05 reads the conclusions of the four runs on `SHA-E`: `ci.yml` `41963325950`, `feature-flags.yml` `41963325969`, `pre-commit` `41963325943`, `codeql.yml` `41963325916`, and the maintainer decision on the `treasury_spend` re-seal rule.
- Commit `23e1d3d17` and this summary are local and ride the next push.

## Self-Check: PASSED

- Files found: `58-CI-EVIDENCE.md`, `58-registry-probe.sh`, `scripts/sdk-smoke/run.sh`, this summary
- Commits found: `99ccf0aa3`, `2e1338a80`, `0e7f0ed86`, `9166fe1fe`, `23e1d3d17`
- `git ls-remote origin refs/heads/claude/laughing-dirac-e0h2ax` equals `SHA-E`; no pull request, no `v0.11.1*` tag
