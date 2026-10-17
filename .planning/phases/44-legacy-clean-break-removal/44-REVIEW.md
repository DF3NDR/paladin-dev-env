---
phase: 44-legacy-clean-break-removal
reviewed: 2026-10-11T00:00:00Z
depth: standard
files_reviewed: 72
files_reviewed_list:
  - crates/paladin-battalion/src/aegis_attempt.rs
  - crates/paladin-battalion/src/campaign_service.rs
  - crates/paladin-battalion/src/commander.rs
  - crates/paladin-battalion/src/conclave_execution_service.rs
  - crates/paladin-battalion/src/engine/mod.rs
  - crates/paladin-battalion/src/error_aggregation.rs
  - crates/paladin-battalion/src/formation_service.rs
  - crates/paladin-battalion/src/lib.rs
  - crates/paladin-battalion/src/llm_failure.rs
  - crates/paladin-battalion/src/phalanx_service.rs
  - crates/paladin-core/src/platform/container/aegis.rs
  - crates/paladin-core/src/platform/container/battalion/conclave.rs
  - crates/paladin-core/src/platform/container/battalion/mod.rs
  - crates/paladin-core/src/platform/container/paladin_error.rs
  - crates/paladin-eval/src/runner.rs
  - crates/paladin-herald/src/json_herald.rs
  - crates/paladin-herald/src/markdown_herald.rs
  - crates/paladin-herald/src/table_herald.rs
  - crates/paladin-ports/src/output/paladin_port.rs
  - crates/doc-examples/src/orchestration.rs
  - src/application/cli/commands/battalion.rs
  - src/application/cli/config/battalion_config.rs
  - src/application/cli/templates/battalion_template.rs
  - src/application/services/battalion/mod.rs
  - src/application/services/paladin/paladin_execution_service.rs
  - src/application/services/paladin/temperature_service.rs
  - src/infrastructure/resilience/circuit_breaker.rs
  - examples/campaign_workflow.rs
  - examples/commander_auto.rs
  - examples/commander_basic.rs
  - examples/commander_full_config.rs
  - examples/commander_with_metadata_export.rs
  - examples/conclave_expert_panel.rs
  - examples/formation_sequential.rs
  - examples/maneuver_nested_flow.rs
  - examples/phalanx_parallel.rs
  - tests/helpers/mock_paladin_port.rs
  - tests/integration/battalion/campaign_integration_test.rs
  - tests/integration/battalion/formation_integration_test.rs
  - tests/integration/battalion/load_test.rs
  - tests/integration/battalion/phalanx_integration_test.rs
  - tests/integration/battalion_campaign_integration_test.rs
  - tests/integration/battalion_chain_of_command_integration_test.rs
  - tests/integration/battalion_herald_end_to_end_test.rs
  - tests/integration/commander_error_paths_test.rs
  - tests/integration/commander_integration_tests.rs
  - tests/legacy_removal_guard.rs
  - tests/unit/battalion/campaign_service_tests.rs
  - tests/unit/battalion/formation_tests.rs
  - tests/unit/circuit_breaker_test.rs
  - tests/unit/paladin_error_test.rs
  - tests/unit/paladin_execution_service_test.rs
  - crates/paladin-battalion/Cargo.toml
  - crates/paladin-core/Cargo.toml
  - .cargo/semver-checks-allowlist.toml
  - .project/current-exports.txt
  - CHANGELOG.md
  - crates/paladin-battalion/CHANGELOG.md
  - crates/paladin-core/CHANGELOG.md
  - crates/paladin-herald/CHANGELOG.md
  - docs/DEMOS.md
  - docs/src/appendix/battalion-vision-support.md
  - docs/src/appendix/cli-usage.md
  - docs/src/appendix/conclave-pattern.md
  - docs/src/contributing/architecture-decisions.md
  - docs/src/deployment/production.md
  - docs/src/getting-started/configuration.md
  - docs/src/user-guides/battalion-patterns.md
  - docs/src/user-guides/fault-tolerance.md
  - docs/src/user-guides/orchestration.md
  - docs/src/user-guides/paladin-agents.md
  - docs/src/user-guides/paladin-configuration.md
  - docs/src/user-guides/tool-integration.md
findings:
  critical: 0
  warning: 5
  info: 5
  total: 10
status: issues_found
---

# Phase 44: Code Review Report

**Reviewed:** 2026-10-11
**Depth:** standard
**Files Reviewed:** 72
**Status:** issues_found

## Summary

The phase's core is sound. Per-attempt timeout selection in `aegis_attempt.rs` is correct. The timeout in `attempt_once` is the only `tokio::time::timeout` in the five legacy-pattern service files, and the guard test enforces this. The tighter-of-two-bounds logic and the Run tie-break are correct. `max_attempts` counts total attempts. The backoff index (`backoff_delay(policy, attempt + 1)`) matches the engine helper's documented contract. Retry gating reads `transience()` and never message text. Phalanx failures are collected in declaration order, and the join handles are awaited in that order. `unwrap`, `expect` and `panic!` do not appear in any new production code. Credential handling holds: the runner's `warn!` logs only the Paladin name, attempt, transience and delay. The heralds add only node id, attempt and transience to the already-redacted source text. `cargo test -p paladin-battalion --lib` passes (871 tests) and `cargo test --test legacy_removal_guard` passes (3 tests).

No BLOCKER-class defect was found. The remaining risks are:

- **Cancellation:** a Phalanx that is cancelled does not stop Paladins that have not started.
- **Hang:** an accepted-but-degenerate `max_concurrency(0)` now hangs forever, where the removed whole-run clock used to bound it.
- **Silent downgrades:** several places silently disarm timeouts or error handlers.
- **Lint masking:** the new crate-wide semver allow lines mask future accidental API removals.

## Warnings

### WR-01: Phalanx cancellation does not stop Paladins that have not started

**File:** `crates/paladin-battalion/src/phalanx_service.rs:177-198, 337-387, 505-532` and `crates/paladin-battalion/src/aegis_attempt.rs:176-219`

**Issue:** `execute_with_cancellation` drops the `execute_internal` future when the token fires. `execute_collect_all` spawns detached tasks with `tokio::spawn`, so dropping the join handles does not stop them.

Each task passes the token only to `wait_backoff`. Neither `run_paladin` nor `run_with_aegis` checks `cancel.is_cancelled()` after acquiring the semaphore permit or before the first attempt, and the second and later attempts are only guarded indirectly through the backoff wait.

On a concurrency-limited Phalanx (`with_max_concurrency(n)`), Paladins queued behind the semaphore therefore acquire a permit as earlier tasks finish and start full LLM calls after the caller has already received `BattalionError::Cancelled`. Nothing awaits or aborts those calls.

This was latent before the phase, but the phase made it worse in two ways. It added a cancellation token that suggests the tasks are cancel-aware. It also made a permit be held across a Paladin's whole retry sequence, so queued work now starts later and is more likely to start after a cancel.

`phalanx_cancellation_during_backoff_stops_retrying` only covers the unlimited case where every Paladin has already started.

**Fix:** Check the token after the permit is acquired and before each attempt. Optionally abort the spawned tasks when cancelled.

```rust
// run_paladin, after the permit is acquired
if cancel.as_ref().is_some_and(|t| t.is_cancelled()) {
    return Err(AttemptFailure::from_error(
        &name, 1, PaladinError::ExecutionError("cancelled before start".into())));
}
// run_with_aegis: at the top of the loop
if cancel.as_ref().is_some_and(|t| t.is_cancelled()) && attempt > 1 {
    return Err(/* last failure */);
}
```

Alternatively, hold the `JoinHandle`s in a `JoinSet` inside `execute_collect_all`, so that dropping the set aborts every task.

### WR-02: `with_max_concurrency(0)` now hangs forever; the removed whole-run clock was the only backstop

**File:** `crates/paladin-battalion/src/phalanx_service.rs:343-345, 514-526` and `crates/paladin-core/src/platform/container/battalion/phalanx.rs:103-106`

**Issue:** `Phalanx::with_max_concurrency(0)` is accepted without validation (`validate()` only checks the Paladin count and the Majority minimum). `execute_collect_all` then builds `Semaphore::new(0)`, and every task blocks forever in `sem.acquire().await`.

Before Phase 44 the 300 s (CLI 600 s) whole-run timeout around `execute_internal` turned this into `BattalionError::Timeout`. Now the only bound is the per-attempt timeout inside `attempt_once`, and the permit wait happens outside it. `BattalionConfig::new` also arms no timeout. The run therefore never returns, and with no timeout armed it never logs anything either.

**Fix:** Reject zero at construction or in `validate_aggregation_strategy`:

```rust
pub fn with_max_concurrency(mut self, max: usize) -> Self {
    self.max_concurrency = Some(max.max(1));
    self
}
```

Or return `BattalionError::ValidationError("max_concurrency must be at least 1")` from `Phalanx::validate` and from `execute_internal`.

### WR-03: Legacy config keys are silently ignored, so an old document loses its timeout, retry and error handling without a signal

**File:** `crates/paladin-core/src/platform/container/battalion/mod.rs:100-101` (`#[serde(default)] pub aegis`) and `crates/paladin-core/CHANGELOG.md` ("a v0.10 `BattalionConfig` document still deserializes; its removed keys are ignored")

**Issue:** A persisted or hand-written `BattalionConfig` or `ConclaveConfig` document that carries `timeout_seconds`, `retry_policy` or `error_strategy` now deserializes cleanly. The three safety settings are dropped and `aegis` becomes `Aegis::default()`, which arms no timeout, no retry and no handler.

For a deployment that relied on `timeout_seconds: 60`, the effect is a silent fail-open: an unbounded run instead of a bounded one. Nothing is logged and no error is returned.

This follows from the clean-break decision, but "removed keys are ignored" is the opposite of the fail-closed posture the phase takes elsewhere (for example `validate_aegis` rejects `Route` and `Custom` handlers rather than ignoring them).

**Fix:** Either reject the removed keys with a targeted error, or log a `warn!` when they are seen. A deserializer shim that detects `timeout_seconds`, `retry_policy` and `error_strategy` and returns `"removed in 0.11: use aegis.timeout/retry/on_error"` is the smallest change. Alternatively, record in the MIGRATION and CHANGELOG entries that stored documents must be rewritten, since the guarantee is silent today.

### WR-04: The Commander no longer bounds ChainOfCommand/Grove, and its `aegis` bridges to Maneuver have inconsistent semantics

**File:** `crates/paladin-battalion/src/commander.rs:506-545, 931-937, 1811-1821` and `crates/paladin-core/src/platform/container/battalion/mod.rs:63-101`

**Issue:** Three related problems.

1. The old `Commander::execute` wrapped every strategy in `timeout(config.timeout_seconds)`. That wrapper is removed. ChainOfCommand and Grove now run with no bound at all, because their services do not read `aegis`.
   - The builder's default config still advertises "300 s ... applied per attempt", which reads as protection that does not exist for those strategies.
   - The behaviour is documented, but only in prose. Nothing warns at build time or at run time when a bounded `aegis` is paired with an unbounded strategy.
2. `default_maneuver_config` maps `aegis.timeout.run_timeout` onto `ManeuverConfig.timeout`. The Maneuver service wraps the whole flow in `tokio::time::timeout` (`maneuver/service.rs:56`).
   - So the same `run_timeout` is a per-attempt bound for Formation, Phalanx and Campaign but a whole-workflow wall clock for Maneuver.
   - The `BattalionConfig.aegis` rustdoc lists Formation, Phalanx, Campaign, Conclave, ChainOfCommand, Grove and Council, but omits Maneuver entirely.
3. A configuration that previously mapped `ContinueOnError` or `RetryThenContinue` to Maneuver `ContinueParallel` now always becomes `FailFast`. `aegis.on_error = Absorb` and `aegis.retry` are silently ignored for Maneuver. Campaign at least emits a one-time `warn!`; Maneuver does not.

**Fix:**
- Add Maneuver to the per-pattern contract in the `aegis` rustdoc and in `docs/src/user-guides/orchestration.md`.
- Emit the same one-time `warn!` for Maneuver as for Campaign when `on_error` or `retry` is set.
- Either do not derive the Maneuver clock from `run_timeout`, or state clearly in the rustdoc that it becomes a whole-flow clock there.
- Optionally, in `CommanderBuilder::build`, warn (or return a `CommanderValidation` error) when a Chain-of-Command or Grove strategy is given a non-default `aegis.timeout`.

### WR-05: New crate-wide semver lint allows mask any future accidental public-API removal

**File:** `crates/paladin-core/Cargo.toml` (new `struct_missing`, `enum_missing`, `enum_variant_missing`) and `crates/paladin-battalion/Cargo.toml` (new `function_missing`, `module_missing`)

**Issue:** The allows are crate-wide, not per item. After this change `cargo-semver-checks` will no longer report a removed struct, enum, enum variant, function or module anywhere in `paladin-core` or `paladin-battalion`.

The CI control is a set-equality check between the allowlist and the MIGRATION rows. That check validates the entries that exist. It cannot notice a removal that is not listed, because the underlying lint is switched off for the whole crate.

The `.cargo/semver-checks-allowlist.toml` entries therefore document this phase's removals but give no protection against the next one.

**Fix:**
- Prefer scoping the exemption per release. For example, enable these lints with `--baseline-version` set to the post-0.11 release so the allows can be deleted after the release that ships them.
- Alternatively, add a CI step that fails when a crate-level allow line has no matching `[[entry]]` for the removals actually found.
- At minimum, leave a dated `TODO: remove after 0.11.0 ships` next to the three new lines in each manifest.

## Info

### IN-01: The retry `warn!` says "retrying" even when the backoff is immediately cancelled

**File:** `crates/paladin-battalion/src/aegis_attempt.rs:207-216`

**Issue:** The log line "attempt N failed (...); retrying in {delay}" is emitted before `wait_backoff`. If the token is already cancelled, `wait_backoff` returns `false` immediately and no retry happens. The log misreports the outcome during a cancel.

**Fix:** Check `cancel.as_ref().is_some_and(CancellationToken::is_cancelled)` before logging, or log after `wait_backoff` returns `true`.

### IN-02: Dead `_started_at` binding left in `FormationExecutionService::execute`

**File:** `crates/paladin-battalion/src/formation_service.rs:169`

**Issue:** `let _started_at = Utc::now();` is never read. It predates the phase (line 153 at the base commit), but the function was rewritten in this phase and still carries the variable. The real `started_at` is taken inside `execute_internal`.

**Fix:** Delete the line and the now-unneeded `Utc` use in that function.

### IN-03: Weak retry-count assertion in the Formation integration test

**File:** `tests/integration/battalion/formation_integration_test.rs` (`test_formation_retry_then_continue`)

**Issue:** The test asserts `log.len() >= 4`. The configured policy (`max_attempts: 4`, Absorb) should produce an exact count (1 Step1 call plus 4 Step2 attempts). A lower bound cannot catch over-retrying (an off-by-one that makes `max_attempts` mean retries rather than total attempts would still pass).

**Fix:** Assert the exact count (`assert_eq!(log.len(), 5)`), as the unit tests in `aegis_attempt.rs` already do for call counts.

### IN-04: `node_id` is the Paladin name, so duplicate Paladin names are ambiguous in `node_errors` and the per-Paladin maps

**File:** `crates/paladin-battalion/src/aegis_attempt.rs:100-107, 115-125` and `phalanx_service.rs:276-286`

**Issue:** This is the recorded operator decision (option-a), so it is not a defect in the implementation. Formation and Phalanx do not enforce unique Paladin names, and Conclave does. If two Paladins share a name, `node_errors` entries cannot be told apart and `per_paladin_times` / `per_paladin_tokens` silently overwrite each other.

**Fix:** Consider rejecting duplicate names in `Formation::new` and `Phalanx::new` (as `Conclave::validate` already does), or documenting the limitation next to `BattalionResult.node_errors`.

### IN-05: Circuit breaker leaves state unchanged for non-Transient failures

**File:** `src/infrastructure/resilience/circuit_breaker.rs:234-242, 293-301`

**Issue:** With the change to `transience() == Transient`, an `Unknown` or `Permanent` failure neither increments nor resets the counter. Two consequences:
- The "consecutive failures" the docs promise are really "Transient failures since the last success", because a Permanent failure between two Transient ones does not reset the count.
- A HalfOpen probe that fails Permanent leaves the breaker HalfOpen, neither closing nor re-opening it.

The behaviour change from the old `is_retryable()` (an `ExecutionError` previously tripped the breaker) is documented in the CHANGELOG. This item is only about the documented semantics.

**Fix:** State the semantics explicitly in the `CircuitBreaker` rustdoc ("non-Transient failures are returned without touching the state"). If a HalfOpen probe should re-open on any failure, route it through `on_failure()`.

---

_Reviewed: 2026-10-11_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
