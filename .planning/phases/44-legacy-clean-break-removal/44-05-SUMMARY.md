---
phase: 44-legacy-clean-break-removal
plan: 05
subsystem: battalion-orchestration
tags: [aegis, phalanx, per-attempt-timeout, retry, absorb, clean-break, test-migration]
status: complete

requires:
  - phase: 44-01
    provides: aegis_attempt runner, BattalionConfig.aegis / with_aegis / validate_aegis
  - phase: 44-04
    provides: Formation on the runner (the sibling shape Phalanx mirrors)
provides:
  - Phalanx on the shared runner: per-attempt timeout (D-02), independent Aegis retry per Paladin (D-01), structured timeout text in the aggregate (D-03)
  - continue-past-failure keyed on aegis.on_error = Absorb (D-05, D-07); unsupported handlers fail closed
  - typed AttemptFailure carried end to end in declaration order; the split_once / split(':') name recovery is gone
  - root Phalanx integration tests migrated to the per-attempt and Absorb contracts
affects: [44-06, 44-07, 44-10, 44-11]

tech-stack:
  added: []
  patterns:
    - "One spawned task per Paladin runs run_paladin (permit held across the whole attempt sequence); join handles awaited in declaration order"
    - "Collect-then-fail: on_error None aggregates every failure's NodeError display into BattalionError::AggregationError"
    - "execute_with_cancellation uses a biased select! so a fired token deterministically reports Cancelled"

key-files:
  created: []
  modified:
    - crates/paladin-battalion/src/phalanx_service.rs
    - tests/integration/battalion/phalanx_integration_test.rs

key-decisions:
  - "Phalanx keeps its collect-then-fail contract under on_error None (ADR-0059 (d)); the message is 'Phalanx execution failed with <n> errors: <NodeError displays joined by '; '>'"
  - "A closed concurrency limiter is a recorded AttemptFailure, never an unwrap panic; the permit logic lives in a free run_paladin function so it is testable"
  - "execute_with_cancellation polls the token first (biased select!) so Cancelled wins over a failure produced by the same token stopping a backoff"
  - "node_errors keeps the v0.10 NodeError summary shape (node_name = failure.node_error.node_id, error = failure.error text); 44-06 swaps it for the structured NodeError"

patterns-established:
  - "Per-Paladin scripted RecordingPort with start/end events and finish instants on a paused clock, for concurrency and ordering assertions"

requirements-completed: []
requirements-advanced: [LEGACY-01, LEGACY-02]  # Phalanx slice; REQUIREMENTS.md left unchecked until the removal plans (44-05..44-10) land

duration: ~1h30min
completed: 2026-10-10
---

# Phase 44 Plan 05: Phalanx on the Aegis runner Summary

**Phalanx now runs every Paladin through `aegis_attempt::run_with_aegis`: each attempt is bounded by `aegis.timeout` (no whole-run clock), each Paladin retries independently under `aegis.retry`, and typed failures are collected in declaration order, then aggregated under `on_error: None` or absorbed under `Absorb`.**

## Accomplishments

- **Task 1 (D-01, D-02, D-03, D-05, D-07):** `execute` and `execute_with_cancellation` call `validate_aegis()` first and no longer wrap the run in `timeout(...)`. `execute_internal` takes a cancellation token; `execute_collect_all` / `execute_first_success` / `execute_majority` return `(Vec<(String, PaladinResult)>, Vec<AttemptFailure>)`. Each CollectAll task runs the new free function `run_paladin`, which holds one semaphore permit across the Paladin's whole attempt sequence (a closed limiter becomes an `AttemptFailure`, not a panic). Failures are matched on `aegis.on_error`: `None` returns an `AggregationError` listing every `NodeError` display (a timeout reads `run timeout`), `Absorb` warns and continues, anything else is a `ValidationError`. `per_paladin_times` / `per_paladin_tokens` / `total_tokens` come from the `(name, result)` pairs, so no failure string is parsed back into a name. Every `unwrap()` in the non-test code is gone (majority lookups included). Ten new Aegis tests replace `test_timeout_enforcement` and the two ContinueOnError tests were converted to Absorb.
- **Task 2:** the root Phalanx integration tests use `absorb()` for the two continue-on-error cases; `test_phalanx_timeout_enforcement` is replaced by `test_phalanx_bounds_each_attempt_not_the_run` and `test_phalanx_attempt_timeout_is_aggregated` (two `run timeout` occurrences in the aggregate).

## Task Commits

1. **Task 1: run Phalanx Paladins through the Aegis runner** - `4a6d2604` (feat)
2. **Task 2: migrate root Phalanx tests to the per-attempt and Absorb contracts** - `de85cdb6` (test)
3. **Rustdoc follow-up: drop the redundant Aegis link target** - `5831df08` (docs; `RUSTDOCFLAGS="-D warnings" cargo doc` rejected the explicit link target once `Aegis` was imported)

## Verification Results (all actually run)

| Check | Result |
| ----- | ------ |
| `cargo test -p paladin-battalion --lib phalanx_service` | 24 passed (run 7 times after the biased-select fix, stable) |
| `cargo test -p paladin-battalion --lib` | 871 passed, 0 failed |
| `cargo test -p paladin-battalion --doc` | 63 passed, 52 ignored (pre-existing), 0 failed |
| `cargo test -p paladin-ai --test lib -- integration::battalion::phalanx_integration_test` | 13 passed |
| `cargo fmt --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean (after Task 2; the docs-only follow-up re-checked with `cargo clippy -p paladin-battalion ...`, clean) |
| `cargo check --workspace --all-targets --all-features` | exit 0 |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` | clean |
| `make api-surface` | "API surface unchanged" |
| `cargo test --workspace --no-fail-fast` | all targets pass except `paladin-ai` lib: 1348 passed, **2 failed** -- only `run_api_wiring::tests::build_run_api_persists_*` (known pre-existing, `deferred-items.md`) |

Plan acceptance greps: non-test part of `phalanx_service.rs` matches 0 for `timeout(|timeout_seconds|error_strategy|split_once|unwrap()`; `validate_aegis()` appears 2 times (both entry points); 0 legacy builder calls (`.with_timeout(`, `.with_error_strategy(`, `.with_retry_policy(`) in `phalanx_service.rs`; in the root file 0 for `BattalionError::Timeout|ErrorStrategy::ContinueOnError|.with_timeout(` and the two named tests present (2). The two workspace failures are the known pre-existing ones; per the plan brief a run whose only failures are those counts as passing.

## Tests added (all in `phalanx_service.rs`)

`phalanx_bounds_each_attempt_not_the_run`, `phalanx_fail_fast_aggregates_structured_failures`, `phalanx_absorb_collects_every_failure` (includes a failure text with colons that must not be split into a name), `phalanx_node_errors_follow_declaration_order`, `phalanx_paladins_retry_independently` (also exercises a closed limiter through `run_paladin`), `phalanx_concurrency_limit_holds_a_permit_across_retries`, `phalanx_first_success_reports_all_failures`, `phalanx_cancellation_during_backoff_stops_retrying`, plus `phalanx_rejects_invalid_aegis_before_any_paladin_runs` and `phalanx_timeout_is_typed_as_run_timeout`. `test_phalanx_node_errors_empty_on_full_success` is kept; the two former ContinueOnError tests were converted to Absorb (`test_partial_failures_with_absorb`, `test_phalanx_metrics_with_partial_failures`).

## Deviations from Plan

**1. [Rule 1 - Bug] Nondeterministic `Cancelled` in `execute_with_cancellation`**
- **Found during:** Task 1, `phalanx_cancellation_during_backoff_stops_retrying` failed intermittently.
- **Issue:** now that the token also stops a Paladin's backoff, the Paladin returns its failure at the same moment the token fires, so the plain `tokio::select!` could pick the `execute_internal` arm and return an `AggregationError` instead of `Cancelled`.
- **Fix:** `biased;` with the `cancelled()` arm first. The plan says `execute_with_cancellation` "still returns `BattalionError::Cancelled` when its token fires"; this makes that deterministic.
- **Files modified:** `crates/paladin-battalion/src/phalanx_service.rs`; folded into the Task 1 commit.

**2. [Rule 1 - Rustdoc lint] Redundant explicit link target** fixed in `5831df08` (see above).

**3. [Plan reading] Extra helper `run_paladin`**
- The plan describes the per-Paladin task body inline in `execute_collect_all`. It is a private free function so the closed-limiter path (otherwise unreachable, the semaphore being internal) can be tested directly. Behaviour is as specified.

**4. [Plan reading] TDD red phase not committed separately**
- Task 1 is `tdd="true"`; the tests and the implementation landed in one `feat` commit, tests written against the final behaviour (same as 44-01 and 44-04). One real RED was observed: the first test run failed three tests because `Phalanx::new` requires at least 2 Paladins (test fixtures fixed, no production change).

**5. [Plan reading] Phalanx tests with one Paladin padded to two**
- Phalanx rejects fewer than two Paladins, so the single-Paladin timing checks carry a second, fast Paladin.

No architectural (Rule 4) changes. No file outside the plan's two was touched. No disk-space recovery was needed.

## Known behaviour notes

- `execute_first_success` reports only the last failure in its `All Paladins failed: ...` message (the v0.10 contract, `select_ok` semantics); the test name `phalanx_first_success_reports_all_failures` follows the plan and asserts that every Paladin was tried and the message prefix.
- When `execute_with_cancellation` returns `Cancelled`, spawned Paladin tasks are detached, not aborted (v0.10 behaviour). They stop retrying at the next backoff because the token is passed to the runner, but an in-flight attempt runs to its end.

## Issues Encountered

- The two pre-existing `run_api_wiring` failures; unchanged and out of scope.

## Known Stubs

None.

## Threat Flags

None. T-44-12 (retry amplification) is mitigated: default no retry, `validate_aegis` at both entry points, a permit held across a Paladin's whole attempt sequence (`phalanx_concurrency_limit_holds_a_permit_across_retries`), backoff capped by `max_interval`. T-44-14 is mitigated: no `unwrap()` in the non-test code (acceptance grep), closed limiter and missing majority entry become recorded failures / `BattalionError`. T-44-13 accepted as planned.

## Requirements

LEGACY-01 and LEGACY-02 are advanced, not completed: only Phalanx moved here, and the legacy `RetryPolicy` / `ErrorStrategy` / `NodeError` / timeout surfaces still exist until 44-06 and 44-10. REQUIREMENTS.md was left unchecked.

## Next Phase Readiness

Phalanx reads no legacy field (`timeout_seconds`, `error_strategy`, `retry_policy`). 44-06 swaps the one `NodeError` construction in `execute_internal` for `failure.node_error`; 44-07 handles the Commander and Conclave tests that still use legacy builders.

## Self-Check: PASSED

Verified at write time: `crates/paladin-battalion/src/phalanx_service.rs` and `tests/integration/battalion/phalanx_integration_test.rs` exist; commits `4a6d2604`, `de85cdb6` and `5831df08` exist on `claude/laughing-dirac-e0h2ax`.
