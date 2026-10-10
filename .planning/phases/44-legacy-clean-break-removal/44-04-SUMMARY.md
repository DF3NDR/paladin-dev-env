---
phase: 44-legacy-clean-break-removal
plan: 04
subsystem: battalion-orchestration
tags: [aegis, formation, per-attempt-timeout, retry, absorb, clean-break, test-migration]
status: complete

requires:
  - phase: 44-01
    provides: aegis_attempt runner, BattalionConfig.aegis / with_aegis / validate_aegis
provides:
  - Formation on the shared runner: per-attempt timeout (D-02), Aegis retry (D-01), structured timeout error (D-03)
  - continue-past-failure keyed on aegis.on_error = Absorb (D-05, D-07); unsupported handlers fail closed
  - Formation-routed tests in five files migrated to the Aegis equivalents, v0.10 execution counts preserved
affects: [44-05, 44-06, 44-07, 44-10, 44-11]

tech-stack:
  added: []
  patterns:
    - "Fail-fast timeout -> BattalionError::Node; other fail-fast failures keep BattalionError::PaladinError(text)"
    - "Absorb records the failure in the v0.10 node_errors shape and hands the next Paladin an empty input"
    - "Legacy-to-Aegis test mapping: max_attempts = old + 1, RetryPredicate::TransientAndUnknown for Unknown-failure mocks"

key-files:
  created: []
  modified:
    - crates/paladin-battalion/src/formation_service.rs
    - crates/paladin-battalion/src/commander.rs
    - tests/integration/commander_error_paths_test.rs
    - tests/integration/commander_integration_tests.rs
    - tests/integration/battalion/formation_integration_test.rs
    - tests/integration/battalion_herald_end_to_end_test.rs

key-decisions:
  - "Absorb pushes the v0.10 NodeError summary into node_errors and the structured NodeError into the AggregatedError; 44-06 swaps the former"
  - "An unsupported on_error variant reaching the wildcard arm returns ValidationError, never continues (T-44-11)"
  - "Formation tests that removed with_timeout rely on the Commander's interim 300 s default until 44-07; no assertion changed"

patterns-established:
  - "Per-Paladin scripted RecordingPort (delay + result per call, records name and input) on a paused clock"

requirements-completed: []
requirements-advanced: [LEGACY-01, LEGACY-02]  # Formation slice; REQUIREMENTS.md left unchecked until the removal plans (44-05..44-10) land

duration: ~50min
completed: 2026-10-10
---

# Phase 44 Plan 04: Formation on the Aegis runner Summary

**Formation now runs every Paladin attempt through `aegis_attempt::run_with_aegis`: bounded per attempt (no whole-run clock), retried under `aegis.retry`, failing fast with a structured `BattalionError::Node` on timeout, and continuing past a failure only under `on_error = Absorb`.**

## Accomplishments

- **Task 1 (D-01, D-02, D-03, D-05, D-07):** `execute` calls `validate_aegis()` first and no longer wraps the run in `timeout(...)`; `execute_internal` calls the runner per Paladin and matches `aegis.on_error`: `None` returns `failure.into_fail_fast_error()`, `Absorb` records the failure (`node_errors`, `paladin_failure_count`), continues with an empty input and reports `Completed`, any other variant returns `ValidationError`. `execute_paladin_with_strategy`, the trailing `unreachable!` match and the legacy imports (`crate::retry`, `ErrorStrategy`, `tokio::time::timeout`) are gone. Six named tests replace the legacy-strategy tests on a paused clock with an in-test recording port.
- **Task 2:** the three Formation-routed tests in `commander.rs` and the Formation tests in `commander_error_paths_test.rs`, `commander_integration_tests.rs`, `formation_integration_test.rs` and `battalion_herald_end_to_end_test.rs` configure Aegis (`absorb()` helpers, `max_attempts = old + 1`, `TransientAndUnknown` with a comment saying why). The whole-run `test_formation_timeout_enforcement` is replaced by `test_formation_bounds_each_attempt_not_the_run` and `test_formation_attempt_timeout_surfaces_structured_error`. Every plain `.with_timeout(N)` in the four root files is removed.

## Task Commits

1. **Task 1: run Formation attempts through the Aegis runner** - `204ce5b8` (feat)
2. **Task 2: migrate Formation-routed tests to the Aegis equivalents** - `8e146291` (test)

## Verification Results (all actually run)

| Check | Result |
| ----- | ------ |
| `cargo test -p paladin-battalion --lib formation_service` | 11 passed (the six plan-named tests plus the five kept tests) |
| `cargo test -p paladin-battalion --lib commander` | 58 passed |
| `cargo test -p paladin-ai --test lib -- integration::commander_error_paths_test integration::commander_integration_tests integration::battalion::formation_integration_test` | 33 passed |
| `cargo test -p paladin-ai --features cli --test lib -- battalion_herald_end_to_end` | 2 passed (the module is `cli`-feature gated, so the plan's default-feature filter matches nothing for it) |
| `cargo fmt --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| `cargo check --workspace --all-targets --all-features` | exit 0 |
| `cargo doc -p paladin-battalion --no-deps` | 0 warnings |
| `cargo test --workspace --no-fail-fast` | 6874 passed, **2 failed**: only `run_api_wiring::tests::build_run_api_persists_*` (see below) |

Plan acceptance greps: non-test part of `formation_service.rs` matches 0 for `timeout(|timeout_seconds|error_strategy|retry_policy|unreachable!|crate::retry`; `validate_aegis()` 1, `run_with_aegis` 2; 0 legacy builder calls in `formation_service.rs`; in the four root files 0 `ErrorStrategy::ContinueOnError|RetryThenContinue|with_retry_policy`, 0 `.with_timeout(`; `BattalionError::Timeout` 0 and `TimeoutKind::Run` 1 in `formation_integration_test.rs`.

The two workspace failures are the known pre-existing `paladin-ai` lib failures recorded in `deferred-items.md` (panic at `run_api_wiring.rs:2369`, confirmed on the pre-phase baseline). Per this plan's brief a run whose only failures are those two counts as passing; they were not touched.

## Deviations from Plan

**1. [Rule 1 - Plan-acceptance mismatch] Doc comments reworded to satisfy the acceptance greps**
- **Found during:** acceptance greps after Task 2
- **Issue:** helper doc comments naming `ErrorStrategy::ContinueOnError` and `BattalionError::Timeout` made the "prints 0" greps print 1.
- **Fix:** reworded to "the v0.10 continue-on-error strategy" and "the v0.10 whole-run timeout variant". No code change.
- **Files modified:** the three root test files above; folded into the Task 2 commit.

**2. [Plan reading] Herald end-to-end test needs `--features cli`**
- The plan's verify filter `integration::battalion_herald_end_to_end_test` matches zero tests in a default-feature build because the module is `#![cfg(feature = "cli")]`. Ran it separately with `--features cli` (2 passed).

**3. [Plan reading] TDD red phase not committed separately**
- Task 1 is `tdd="true"`; the tests and the implementation landed in one `feat` commit. The tests were written against the final behaviour and run green before commit; no separate RED failure was observed.

**4. [Rule 2 - correctness note] `commander.rs` Formation tests dropped `.with_timeout(60)`**
- The three Formation-routed tests lost their legacy `.with_timeout(60)` along with the strategy setters, mirroring the root-file rule. The rest of that test module is untouched for 44-07. No assertion changed.

No architectural (Rule 4) changes. No file outside the plan's six was touched; no disk-space recovery was needed.

## Issues Encountered

- The two pre-existing `run_api_wiring` failures above; unchanged and out of scope.

## Known Stubs

None.

## Threat Flags

None. T-44-10 (no whole-run bound) is accepted by D-02 and documented by 44-01/44-09/44-11; T-44-11 is mitigated by `validate_aegis` at entry, the fail-closed wildcard arm and `formation_rejects_invalid_aegis_before_any_paladin_runs`.

## Next Phase Readiness

Formation reads no legacy field (`timeout_seconds`, `error_strategy`, `retry_policy`). 44-05 (Phalanx) can mirror the same runner call; 44-06 swaps the one `NodeError` construction in the Absorb branch for `failure.node_error`; 44-07 sweeps the remaining `commander.rs` tests and replaces the Commander's interim timeout default.

## Self-Check: PASSED

Verified below at write time: `formation_service.rs` and the four root test files exist; commits `204ce5b8` and `8e146291` exist on the branch.
