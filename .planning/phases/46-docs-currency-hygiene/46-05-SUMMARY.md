---
phase: 46-docs-currency-hygiene
plan: 05
subsystem: infra
tags: [adr-0015, dependency-allowlist, shell-gate, windows-ledger, curr-25, tomllib]

requires:
  - phase: 07
    provides: ADR-0015 (core/ports dependency allowlist and the purity invariant), the measured 14 / 11 baseline
  - phase: 45
    provides: WINDOWS rows 35 / 61 (accepted tracing overhead in the dispatcher/sink path) cited by the clone/lock waiver
provides:
  - ADR-0015 dated amendment (2026-10-24): paladin-core 14 keys, paladin-ports 12 (adds `log`)
  - scripts/check-dependency-allowlist.sh, an exact-key guard over [dependencies] plus build/target tables
  - tests/scripts/check-dependency-allowlist_test.sh, a 15-assertion regression harness run by make test-shell-guards
  - ninth check-gates member (check-dependency-allowlist) and a CI cargo-deny job step
  - WINDOWS rows 80 (waived, oversized files), 81 (waived, clone/lock), 82 (fixed, allowlist drift)
affects: [46-13, release-gate, audit-milestone tech-debt reading]

tech-stack:
  added: []
  patterns: ["house shell gate: python3 tomllib heredoc, env-overridable inputs, sorted violations, harness with fire/silent asserts and a no-mutation check"]

key-files:
  created:
    - scripts/check-dependency-allowlist.sh
    - tests/scripts/check-dependency-allowlist_test.sh
    - .planning/phases/46-docs-currency-hygiene/deferred-items.md
  modified:
    - .planning/decisions/0015-core-ports-dependency-allowlist.md
    - Makefile
    - .github/workflows/ci.yml
    - .planning/WINDOWS.md
    - .planning/STATE.md
    - .planning/ROADMAP.md

key-decisions:
  - "ADR-0015 is amended (dated 2026-10-24) before the guard is pinned; the amendment does not date the addition of `log`, only that it is absent from the 2026-08-06 measurement (RESEARCH A5). No new ADR, PROMOTION.md untouched, no 0063 (D-00c)"
  - "The guard compares the [dependencies] table keys exactly and also rejects any key in [build-dependencies] and [target.*.dependencies] / [target.*.build-dependencies]; [dev-dependencies] stay out of scope"
  - "The clone/lock waiver states that no benchmark attributes a regression to cloning or lock wait (not that no contention exists) and names the dispatcher/sink path of WINDOWS rows 35/61 as the first place to look"

patterns-established:
  - "Pin a manifest set with a script mirrored in an ADR amendment; the guard header requires both to change in one commit"

requirements-completed: [CURR-25]

duration: ~12 min
completed: 2026-10-24
status: complete
---

# Phase 46 Plan 05: Dependency-allowlist guard and the three v2 debt dispositions Summary

**ADR-0015 re-measured at 14 / 12 (paladin-ports gained `log`) and pinned by an exact-key `tomllib` guard in `make check-gates` and CI's cargo-deny job, with the three v2 debt lines recorded as WINDOWS rows 80 (waived), 81 (waived) and 82 (fixed).**

## Performance

- **Duration:** ~12 min
- **Started:** 2026-10-24T13:42Z (approx., after plan 46-04's close-out commit)
- **Completed:** 2026-10-24T13:52Z
- **Tasks:** 2 (1 tracer, 1 auto)
- **Files modified:** 9 (3 created, 6 modified)

## Accomplishments

- ADR-0015 carries a dated amendment listing both sets literally and stating why `log` does not break invariant (i).
- A mechanical guard now fails a planted surplus (or removed, or near-spelled) dependency in either inward crate, locally and in CI.
- The three "deferred to v2" lines each have a ledger row written only through `gsd-tools windows`, and STATE.md and ROADMAP.md cite the row ids.

## Task Commits

1. **Task 1 (tracer): ADR-0015 amendment, harness, guard, make target, CI step** - `badbb418` (feat)
2. **Task 2: ledger rows 80/81/82, STATE and ROADMAP pointers** - `d43fecc7` (docs)

**Plan metadata:** the SUMMARY / STATE / ROADMAP commit follows this file.

## Tracer gate

RED: `bash tests/scripts/check-dependency-allowlist_test.sh` failed before the guard existed ("guard script not found", exit 1). GREEN: the guard was written and the harness passed 15/15 (after one harness-fixture fix, below). The tracer `<verify>` was re-run from the committed state and passed end to end (harness, guard, `make check-gates`, `make test-shell-guards` 54 assertions in the aggregate leg shown, `make lint-shell`, the grep checks, no 0063 ADR, PROMOTION.md unchanged), so expansion proceeded.

## Ledger evidence (Task 2)

Re-measured 2026-10-24 with `wc -l` (production-only = raw minus every `#[cfg(test)]` module body and cfg(test) import line):

| File | Raw | Production-only | Test module starts |
|------|-----|-----------------|--------------------|
| `crates/paladin-battalion/src/engine/superstep.rs` | 16,018 | ~5,363 | line 5364 |
| `crates/paladin-battalion/src/engine/mod.rs` | 14,324 | ~3,251 | line 3258 (plus cfg(test) lines 90, 105, 110) |
| `src/application/services/paladin/paladin_execution_service.rs` | 10,858 | ~4,419 | line 4382 (ten cfg(test) modules) |

These match the planning-time raw figures, so the plan's numbers were used unchanged.

`windows status` before and after: total 79 to 82, open 13 to 13, waived 37 to 39, fixed 29 to 30. The three ids were assigned by the tool, in order: 80 (oversized files, waived), 81 (clone/lock, waived), 82 (allowlist drift, fixed). Each was written as `append` then `waive` / `fixed`, strictly sequentially, with a `status` read after each step.

## Files Created/Modified

- `scripts/check-dependency-allowlist.sh` - the ADR-0015 purity guard (python3 `tomllib`, env overrides `CORE_MANIFEST` / `PORTS_MANIFEST`, violations sorted by crate then key, read-only)
- `tests/scripts/check-dependency-allowlist_test.sh` - 15 assertions: surplus core/ports key, missing key, near-spelling, build-dependencies, target tables, empty/absent/nonexistent manifest, sort order, determinism, dev-only pass, real tree, no mutation
- `.planning/decisions/0015-core-ports-dependency-allowlist.md` - dated amendment and the two literal sets
- `Makefile` - `check-dependency-allowlist` target, ninth `check-gates` member, comment "nine members"
- `.github/workflows/ci.yml` - step "Check core/ports dependency allowlist (ADR-0015)" between "Check publish order" and "Run shell-guard regression tests"
- `.planning/WINDOWS.md` - rows 80-82 (tool-written)
- `.planning/STATE.md`, `.planning/ROADMAP.md` - the three v2 debt rows and bullets now name their ledger rows
- `.planning/phases/46-docs-currency-hygiene/deferred-items.md` - the out-of-scope advisory finding below

## Decisions Made

See `key-decisions` in the frontmatter. The two-sided pin (guard literal sets plus the ADR amendment) is what keeps the guard from silently blessing drift.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Harness dev-dependencies fixture appended a duplicate table header**
- **Found during:** Task 1 (GREEN)
- **Issue:** the real paladin-core manifest already has `[dev-dependencies]`, so appending a second header made the fixture invalid TOML and the guard (correctly) reported a parse error; the `[dev-dependencies]` pass case failed.
- **Fix:** the fixture now plants the key under the existing `[dev-dependencies]` header with `sed` and aborts if the key was not planted.
- **Files modified:** `tests/scripts/check-dependency-allowlist_test.sh`
- **Commit:** `badbb418` (fixed before the commit; never committed broken)

**2. [Plan drift, not a rule] Line numbers in the plan's `<interfaces>` had drifted**
- STATE.md deferred rows were at 804 / 805 / 823 (plan said 796 / 797 / 815) and the ROADMAP tech-debt section at 990 (plan said 957). Content matched; the rows were located by their text, not line number. `git diff -U0 .planning/STATE.md` shows exactly the three intended rows changed.

**3. [Interpretation] Where the oversized-files figures live**
- The plan puts the measured raw and production-only figures in the row 80 description (step 1) and a figures-are-measured sentence in its waiver reason; the acceptance bullet says "reason". I followed the step text: the figures are in the row's description (and the STATE.md status cell), and the reason points at them. `windows waive` cannot be re-run on an already-waived row without changing the ledger counts, so the reason text was not duplicated.

**Total deviations:** 1 auto-fixed (Rule 1), 2 notes. **Impact:** none on the delivered behaviour.

## Verification

- Task 1 verify: pass (tracer gate above). Task 2 verify: pass (`ok [80, 81, 82]`, STATE and ROADMAP greps).
- `make check-gates`, `make test-shell-guards`, `make lint-shell`: exit 0 (re-run after the commit, so the workflow-suppressions harness's clean-`.github/workflows/` assertion also passes; it failed only while `ci.yml` was uncommitted).
- `make clean-code` (fmt, clippy, shellcheck, check, doc-check, public-API examples gate): exit 0, once, for D-00a (no Rust changed).
- `make security`: the advisory DB fetch works and `cargo deny` fails the advisories leg on RUSTSEC-2026-0386 (`tokio::io::unix::AsyncFd`, an `unsound` advisory) while bans, licenses and sources are ok. Plan 46-05 changed no `Cargo.toml` / `Cargo.lock`, so this is pre-existing; logged in `deferred-items.md` and left to CI's Security Audit job. Not fixed here (scope boundary).
- Credential-handling review: the new scripts read two manifests and print dependency key names; no secret or environment value is read or echoed.
- mdbook is not installed here; this plan touches no mdBook page, so no mdbook leg applies.

## Known Stubs

None.

## Threat Flags

None. The new surface is an offline read-only gate; it mitigates T-46-05..T-46-08 as planned (exact-key guard with a harness proving each case, ADR mirror, sequential ledger writes with before/after counts, a dedicated CI step in the always-on cargo-deny job).

## Issues Encountered

`cargo deny` advisories failure on RUSTSEC-2026-0386, pre-existing and unrelated (see Verification and `deferred-items.md`).

## Next Phase Readiness

Plan 46-13 can cite WINDOWS rows 80-82 and the amended ADR-0015. No blockers from this plan.

## Self-Check: PASSED

- FOUND: scripts/check-dependency-allowlist.sh, tests/scripts/check-dependency-allowlist_test.sh, .planning/decisions/0015-core-ports-dependency-allowlist.md
- FOUND commits: badbb418, d43fecc7
- PROMOTION.md unchanged; no `0063-*.md` exists
