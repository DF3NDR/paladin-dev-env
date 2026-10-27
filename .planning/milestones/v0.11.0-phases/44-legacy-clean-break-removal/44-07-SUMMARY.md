---
phase: 44-legacy-clean-break-removal
plan: 07
subsystem: battalion-orchestration
tags: [aegis, commander, conclave, per-attempt-timeout, engine-backoff, clean-break]
status: complete

requires:
  - phase: 44-01
    provides: aegis_attempt runner (attempt_bound, attempt_once), BattalionConfig.aegis / validate_aegis
  - phase: 44-03
    provides: Transient-only Conclave retry predicate on PaladinError::transience()
  - phase: 44-04
    provides: Formation on the runner; the Commander's interim 300 s default and the remaining commander.rs test sweep
provides:
  - Commander::execute with no wall clock of its own (D-03); a bare CommanderBuilder keeps a 300 s per-attempt bound
  - Conclave and Maneuver bridges sourced from aegis (D-04, ADR-0059 e)
  - Conclave experts and aggregator on attempt_once with engine::retry::backoff_delay (D-12)
  - Conclave::validate and ConclaveExecutionService::execute on validate_aegis
affects: [44-08, 44-09, 44-10, 44-11]

tech-stack:
  added: []
  patterns:
    - "Private bridge helpers (conclave_config_from_battalion, default_maneuver_config) so D-04 mappings are unit-testable"
    - "Scripted per-Paladin port recording (name, input, Instant) on a paused clock for backoff-shape assertions"

key-files:
  created: []
  modified:
    - crates/paladin-battalion/src/commander.rs
    - crates/paladin-battalion/src/conclave_execution_service.rs
    - crates/paladin-core/src/platform/container/battalion/conclave.rs

key-decisions:
  - "retry_attempts stays the Conclave retry count; aegis.retry contributes only the backoff shape (RetryPolicy::default() when None); the predicate stays Transient-only regardless of retry_on"
  - "Aggregator runs as a single attempt (no retry); a timeout surfaces as AggregatorFailed containing 'Timeout'"
  - "ConclaveExecutionService::execute validates the policy first and maps failure to ConclaveError::ConfigurationError"

patterns-established:
  - "Bridge Aegis -> legacy configs through private &self helpers rather than inline closures"

requirements-completed: []
requirements-advanced: [LEGACY-02, LEGACY-03]  # REQUIREMENTS.md left unchecked until the removal plans (44-08..44-10) land

duration: ~55min
completed: 2026-10-11
---

# Phase 44 Plan 07: Commander and Conclave on Aegis Summary

**The Commander no longer wraps `execute()` in a timeout, bridges Conclave and Maneuver from `aegis` (retry_attempts = max_attempts - 1, FailFast Maneuver), defaults a bare builder to a 300 s per-attempt bound, and the Conclave runs every expert and aggregator attempt through `attempt_once` with the engine's backoff.**

## Accomplishments

- **Task 1 (commit `238436fb`)** -- D-03, D-04, Open Questions 1 and 6.
  - `Commander::execute` awaits `execute_internal` directly. The rustdoc now has a `# Timeouts` section (per attempt, ChainOfCommand / Grove / Council not bounded, wrap `execute()` for a whole-run bound), and the type-level feature bullets and both examples use `with_aegis`.
  - New private helpers `conclave_config_from_battalion` and `default_maneuver_config`; the legacy timeout builder is no longer called, and the legacy `ErrorStrategy` import is gone from non-test code.
  - `CommanderBuilder::build` defaults to `Aegis { timeout: run_timeout 300 s }` and validates via `validate_aegis()`, mapping `ValidationError` to `CommanderValidation`.
  - Tests: `commander_formation_timeout_is_per_attempt` (replaces `test_timeout_enforcement`), `commander_builder_default_bounds_each_attempt`, `commander_builder_rejects_zero_run_timeout`, `commander_maneuver_bridge_defaults_to_fail_fast`, `commander_conclave_bridge_derives_retry_attempts`, plus `commander_bound_is_not_a_whole_run_clock` (two 0.8 s Paladins under a 1 s bound complete in 1.6 s). `create_test_config` rebuilt with `BattalionConfig::new`; all legacy builder calls removed from the test module.
- **Task 2 (commit `21daaba7`)** -- D-12, Open Question 2, T-44-19/20.
  - `execute_expert_with_retry` takes the cloned `Aegis`, computes the bound and the policy once, runs each attempt via `attempt_once`, and waits `backoff_delay(&policy, retries + 1)` through `wait_backoff`. The private delay helper, the `rand` import, the raw `sleep` and the whole-Conclave `timeout` wrapper are deleted; `test_retry_delay_calculation` is deleted.
  - The aggregator is one `attempt_once(.., bound, 1)`.
  - `execute` calls `validate_aegis()` first. `Conclave::validate` keeps `retry_attempts <= 5` and delegates the rest to `validate_aegis()?`; the two timeout range checks and their test are gone; `ConclaveConfig` doctest and rustdoc now use `with_aegis`.
  - Tests (all paused clock): `conclave_backoff_follows_engine_sequence`, `conclave_expert_attempt_times_out_and_retries`, `conclave_aggregator_timeout_is_aggregator_failed`, `conclave_zero_retry_attempts_runs_once`, `conclave_without_aegis_retry_uses_default_backoff_shape`, `conclave_retry_resends_identical_input`, `conclave_rejects_invalid_aegis_before_any_expert_runs` (service), `conclave_validate_delegates_to_validate_aegis` (core). The three pre-existing retrying service tests moved to `start_paused = true`, removing their real 1 s + 2 s sleeps.

## Verification Results (all actually run)

| Check | Result |
| ----- | ------ |
| `cargo test -p paladin-battalion --lib commander` | 61 passed, includes the five plan-named tests |
| `cargo test -p paladin-battalion --lib conclave_execution_service` | 16 passed, includes the seven plan-named tests |
| `cargo test -p paladin-ai-core --lib conclave` | 10 passed (includes `conclave_validate_delegates_to_validate_aegis`) |
| `cargo test -p paladin-ai-core --doc conclave` | 1 passed (the rewritten `ConclaveConfig` doctest), 4 ignored (pre-existing `ignore` examples) |
| `cargo fmt --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| `cargo check --workspace --all-targets --all-features` | exit 0 |
| `cargo test --workspace --no-fail-fast` | 6899 passed, **2 failed**: only `run_api_wiring::tests::build_run_api_persists_*` |
| `make doc-check` | passed |
| `make api-surface` | unchanged (4275 items) |

The two workspace failures are the known pre-existing `paladin-ai` lib failures in `deferred-items.md` (panic at `run_api_wiring.rs:2369`, confirmed on the pre-phase baseline). Per the brief, a run whose only failures are those two counts as passing; they were not touched.

Plan acceptance greps: non-test `commander.rs` matches 0 for `timeout(|timeout_seconds|retry_policy|self.config.error_strategy|with_error_strategy(`, has `validate_aegis()` and `Duration::from_secs(300)`, both helper fns exist; the `commander.rs` test module has 0 legacy builder/field uses and 0 non-`Duration` `.with_timeout(`. Non-test `conclave_execution_service.rs` matches 0 for `timeout(|timeout_seconds|rand::|sleep(|calculate_retry_delay` and has 7 `backoff_delay|wait_backoff|attempt_once` hits; non-test `conclave.rs` has `validate_aegis()`. `git grep 'timeout_seconds\|retry_policy\|error_strategy' crates/paladin-battalion/src` lists only test lines, the Maneuver's own `error_strategy` field/builder, and `engine/superstep.rs` local variables named `retry_policy` (a binding of the Aegis policy, not the legacy field).

## Deviations from Plan

None -- plan executed as written. Notes on scope boundaries:

- `ConclaveConfig::timeout_seconds`, `with_timeout`, `ConclaveError::Timeout` and the core test `test_conclave_config_builder` (which still calls `.with_timeout(600)`) were deliberately left: the plan keeps them defined (unread, unproduced) until 44-10 deletes them.
- The "concurrency" truth (experts retry independently; breaker accounting under concurrent callers) is `verification: backstop` and rests on each expert running in its own spawned task plus the breaker test migrated in 44-03; no new dedicated test was added.
- No disk-space workaround was needed.

## Known Stubs

None.

## Threat Flags

None. T-44-18 (ChainOfCommand / Grove unbounded through the Commander) is accepted per the plan and documented in the new `execute` rustdoc `# Timeouts` section; the 9.1 row is 44-11's. T-44-19 and T-44-20 are mitigated as planned (clamp to 5 retained, engine backoff cap, `validate_aegis` at builder, `Conclave::validate` and service `execute`, each tested).

## Next Phase Readiness

After this plan no production code in `paladin-battalion` reads the legacy `BattalionConfig` timeout, retry or strategy fields. 44-08 / 44-09 can migrate consumers and docs, and 44-10 can delete the definitions (including `ConclaveConfig.timeout_seconds`, `with_timeout`, `ConclaveError::Timeout` and the remaining core `test_conclave_config_builder` call).

## Self-Check: PASSED

- Commits `238436fb` and `21daaba7` exist in `git log`.
- Modified files exist: `crates/paladin-battalion/src/commander.rs`, `crates/paladin-battalion/src/conclave_execution_service.rs`, `crates/paladin-core/src/platform/container/battalion/conclave.rs`.
