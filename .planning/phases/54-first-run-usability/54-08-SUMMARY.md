---
phase: 54-first-run-usability
plan: 08
subsystem: release-tooling
tags: [changelog, release-toml, comments, cargo-release, first-run]
status: complete

requires:
  - phase: 54-first-run-usability
    provides: "54-07 made the Makefile check-gates and ci.yml MSRV comments count-free"
provides:
  - "CHANGELOG.md without the dangling [Unreleased] footer compare link"
  - "release.toml recording cargo-release 1.1.7 as the version the v0.11.0 bump ran on"
  - "count-free comments in release.toml, finalize-crate-changelogs.sh, the check-release-consistency.sh hint and the publish-crates.sh usage comment"
affects: [54-09, phase 58 lockstep version bump]

tech-stack:
  added: []
  patterns:
    - "Comment-only edits verified by a diff filter that allows only comment and echo-string lines"

key-files:
  created: []
  modified:
    - CHANGELOG.md
    - release.toml
    - scripts/finalize-crate-changelogs.sh
    - scripts/check-release-consistency.sh
    - scripts/publish-crates.sh

key-decisions:
  - "PD-08-1 (D-17): the footer [Unreleased] link is deleted, not retargeted; no compare link is added for any version"
  - "PD-08-2: the check-release-consistency.sh hint and publish-crates.sh comment are swept because no harness needle matches the edited lines"

requirements-completed: [FIRST-07]

duration: ~10min
completed: 2026-11-01
---

# Phase 54 Plan 08: Release comments and changelog footer Summary

**The CHANGELOG footer no longer links "Unreleased" to another repository's v0.2.0 compare page, `release.toml` records `cargo-release 1.1.7` as the version v0.11.0 ran on, and no release comment counts crates or files any more.**

## Performance

- **Plan-start SHA:** `946e953fdc1369f5d8a13af09070876c5386a75a`
- **Tasks:** 2 (1 tracer, 1 auto)
- **Files:** 5 modified, none created

## Accomplishments

- Deleted the one footer line `[Unreleased]: https://github.com/jamatulli/paladin/compare/v0.2.0...HEAD`. The `[0.2.0]:` and `[0.1.0]:` definitions and `## [Unreleased]` (count 1) are unchanged.
- `release.toml`: the rationale now says "every per-crate CHANGELOG.md file" and "a list of distinct target files, one per crate". The Phase 20 `cargo-release 1.1.2` observation is replaced by "The v0.11.0 lockstep bump ran on `cargo-release 1.1.7`, installed with `--locked` in plan 47-01 (47-CI-EVIDENCE.md, Local sweep rows 13-14)". No TOML key or value changed; `tomllib` parses it.
- `finalize-crate-changelogs.sh` header: no file count and no "today" claim.
- `check-release-consistency.sh` hint item 5's last line is `echo "     every changelog."`.
- `publish-crates.sh` usage comment says "the built-in crate dependency order".

## Task Commits

1. **Task 1 (tracer): drop the dangling `[Unreleased]` footer link** - `70a14ba8e`
2. **Task 2: count-free release comments; `cargo-release 1.1.7`** - `f0956ce67`

## Evidence

**Prove-then-lock, Task 1.** `grep -n '^\[' CHANGELOG.md` showed the three footer definitions at l.3413 (`[Unreleased]`), l.3414 (`[0.2.0]`) and l.3415 (`[0.1.0]`), plus two unrelated inline-link lines at l.738 and l.740. The pre-edit extract `extract-changelog-section.sh --version 0.11.0` exited 0 and wrote 108926 bytes.

**Extract comparison.** After deleting l.3413, `cmp before.md after.md` reported the two 0.11.0 extracts byte-identical. `git diff --stat` was `CHANGELOG.md | 1 -` (one deletion, no addition). `extract-changelog-section_test.sh`: 16 assertions passed.

**Tracer feedback gate (auto mode).** The tracer's verify (no `^[Unreleased]:`, `[0.2.0]:` and `[0.1.0]:` present, extract non-empty, harness passes) was re-run before expanding; it passed.

**Prove-then-lock, Task 2.** The count grep hit `release.toml:41` ("ten per-crate"), `release.toml:47` ("list of ten"), `finalize-crate-changelogs.sh:8` ("eleven-file ... all ten crate"), `check-release-consistency.sh:595` ("eleven files") and `publish-crates.sh:114` ("eleven-crate dependency order"). `grep -n 'cargo-release 1' release.toml` hit l.49 (`1.1.2`).

**Harness needles re-measured (step 4).** The `assert_fire` needles in `tests/scripts/check-release-consistency_test.sh` are: `pkg-a`, `pkg-b`, `pkg-c`, `ZERO_PACKAGES`, `MISSING_TAG`, `unknown flag`, `pkg-manifests/pkg-cl2-b/CHANGELOG.md`, `no section for version`, `not found`, `pkg-cl2-d`, `pkg-cl2-e`, `pkg-cl2-f1`, `pkg-cl2-f2`, the CI SHA variable, `re-run CI on main`, `concluded 'failure'` and `no completed ci.yml run found`. None matches the old or the new item 5 line, so the contingency did not fire and the harness is untouched. The harness exits 0 after the edit (32 assertions).

**Static checks.** The plan's `<verify>` greps all pass, `release.toml` parses with `tomllib`, and `shellcheck --severity=warning` is clean on the three scripts. The diff filter (`git diff | grep '^[-+]' | grep -vE '^(\+\+\+|---)' | grep -vE '^[-+]\s*(#|echo )'`) printed nothing, so only comment lines and the one echo string changed.

**Harnesses.** `finalize-crate-changelogs_test.sh` 25 assertions, `check-release-consistency_test.sh` 32, `publish-crates_test.sh` pass; `make test-shell-guards` exit 0 (85 assertions in the final harness) on the committed, clean tree.

## Counts left in place, with reasons

Measured with `cargo metadata --no-deps --format-version 1`: 13 workspace packages, 12 publishable, and 11 `crates/*/CHANGELOG.md` files plus the root one.

- `docs/src/appendix/release-automation.md`: "twelve-crate order" and "twelve publishable crates" (l.76-77, 349, 412) match the 12 publishable packages; "eleven `crates/*` packages" (l.119) and "eleven per-crate" changelogs (l.220) match 11; the "all eleven crates at the time" entries (l.357, 369-370) are historical records of the 0.8.1 publishes.
- `scripts/finalize-release-body.sh:267` ("the eleven-crate workspace"): true, the `crates/*` count, and it is contrasted with the root package.
- `scripts/check-deprecations.sh:49` ("eleven workspace crates were previously unscanned"): a historical sentence about when the scan widened.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] shellcheck severity**
- **Found during:** Task 2
- **Issue:** The plan's verify runs bare `shellcheck`, which fails on pre-existing info and style findings in these scripts (the same issue 54-07 found).
- **Fix:** Verified at `--severity=warning`, the level `make lint-shell` and the pre-commit gate use. No code was changed to chase info-level findings.
- **Files modified:** none

Otherwise none; the plan executed as written. The `check-release-consistency_test.sh` contingency did not fire.

## Known Stubs

None.

## Threat Flags

None. The edits touch only comments, one echo string and one footer line. T-54-24 is mitigated by the byte-identical extract and the three harnesses, T-54-25 by deleting the link, and T-54-26 by the diff filter, `shellcheck` and the harnesses.

## Self-Check: PASSED

- FOUND: CHANGELOG.md, release.toml, scripts/finalize-crate-changelogs.sh, scripts/check-release-consistency.sh, scripts/publish-crates.sh
- FOUND commits: 70a14ba8e, f0956ce67
- `git diff --stat 946e953fd..HEAD` over the plan's code files lists exactly the five `files_modified` entries.
