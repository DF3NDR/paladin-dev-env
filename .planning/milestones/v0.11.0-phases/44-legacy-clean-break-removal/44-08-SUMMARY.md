---
phase: 44-legacy-clean-break-removal
plan: 08
subsystem: cli-and-examples
tags: [aegis, cli, examples, doc-examples, per-attempt-timeout, clean-break]
status: complete

requires:
  - phase: 44-01
    provides: BattalionConfig.aegis, with_aegis, validate_aegis and the per-pattern contract
  - phase: 44-06
    provides: BattalionResult.node_errors as Vec<node_error::NodeError>
  - phase: 44-07
    provides: Commander with no execute() wrapper; aegis-sourced Conclave and Maneuver bridges
provides:
  - CLI Formation, Phalanx and Conclave runs configured through BattalionConfig.aegis (per-attempt bound), YAML schema unchanged
  - Eight examples (and the doc-examples formation anchor) on with_aegis, with no legacy builder call or removed identifier
  - examples/commander_full_config.rs as the downstream migration guide from the v0.10 strategies to Aegis
affects: [44-09, 44-10, 44-11, 44-13]

tech-stack:
  added: []
  patterns:
    - "Private per-file helper returning an Aegis whose only policy is a per-attempt TimeoutPolicy (CLI, Campaign and Conclave examples)"

key-files:
  created: []
  modified:
    - src/application/cli/commands/battalion.rs
    - src/application/cli/config/battalion_config.rs
    - src/application/cli/templates/battalion_template.rs
    - crates/doc-examples/src/orchestration.rs
    - examples/commander_full_config.rs
    - examples/commander_basic.rs
    - examples/commander_auto.rs
    - examples/commander_with_metadata_export.rs
    - examples/formation_sequential.rs
    - examples/phalanx_parallel.rs
    - examples/campaign_workflow.rs
    - examples/conclave_expert_panel.rs
    - examples/maneuver_nested_flow.rs

key-decisions:
  - "Aegis types are imported from paladin_core directly (the root crate does not re-export the aegis module under crate::core), matching src/config/agent_runtime.rs and examples/node_result_cache.rs"
  - "Phalanx and Campaign examples use small local helpers (fail_fast_aegis / absorbing_aegis / per_attempt_timeout) instead of repeating the struct literal"
  - "The CLI Maneuver site keeps its unused _battalion_config but drops the timeout call outright (a Maneuver has no per-attempt bound to configure)"

patterns-established:
  - "CLI maps timeout_seconds onto aegis.timeout.run_timeout per attempt; the existing zero rejection stays in CLI validation"

requirements-completed: []
requirements-advanced: [LEGACY-04]  # REQUIREMENTS.md left unchecked; ticked by the removal and docs plans

duration: ~50min
completed: 2026-10-11
---

# Phase 44 Plan 08: CLI, Examples and doc-examples on Aegis Summary

**The CLI's Formation, Phalanx and Conclave runs now bound each Paladin attempt through `BattalionConfig::with_aegis` (YAML keys unchanged), and the eight Battalion examples plus the mdBook `formation` anchor show the Aegis forms, with `commander_full_config.rs` serving as the v0.10-to-Aegis migration guide.**

## Accomplishments

- **Task 1 (commit `dc0b155c`)** -- D-20, Open Question 8.
  - Private `per_attempt_timeout(secs) -> Aegis` in `src/application/cli/commands/battalion.rs` (documented as per attempt, not per run). Formation and Phalanx use `per_attempt_timeout(600)`; Conclave puts `per_attempt_timeout(config.timeout_seconds.unwrap_or(300))` on the `BattalionConfig` and keeps `with_retry_attempts` and `with_observability` on the domain `ConclaveConfig`; the unused Maneuver `_battalion_config` lost its timeout call.
  - CLI `timeout_seconds` rustdoc and the battalion template comment now say "Per-attempt timeout ..." (diff against `b362d6cb` of `battalion_config.rs` is rustdoc only; no key, type or default changed). The CLI's `timeout_seconds: 0` rejection is untouched.
  - `crates/doc-examples/src/orchestration.rs` `formation` anchor builds `BattalionConfig::new("earnings_pipeline").with_aegis(..60 s per attempt..)`; the `phalanx` anchor is untouched.
- **Task 2 (commit `efcfb089`)** -- D-20 Commander examples.
  - `commander_full_config.rs`: `demonstrate_policy(label, aegis)` run three times -- fail fast (no handler), Absorb, and `max_attempts: 4` + `RetryPredicate::TransientAndUnknown` + Absorb -- each with a 120 s per-attempt `TimeoutPolicy`. Comments state that `max_attempts` counts total attempts including the first and that `TransientAndUnknown` is an opt-in (default `TransientOnly`). It prints the configured policy from `config.aegis` and each `node_errors` entry with its `Display`; the closing summary and "recommended production config" text are in the Aegis form.
  - `commander_basic`, `commander_auto`, `commander_with_metadata_export` use `with_aegis`; the metadata-export example uses the Absorb handler (capture every Paladin's metrics).
- **Task 3 (commit `82137b49`)** -- D-20 pattern examples.
  - `formation_sequential` (per-attempt 60 s, Absorb with a comment that the next Paladin gets an empty input), `phalanx_parallel` (helpers `fail_fast_aegis` / `absorbing_aegis`, prints `node_errors`), `campaign_workflow` (helper `per_attempt_timeout`, comment that a Campaign has no continue mode and ignores `on_error`), `conclave_expert_panel` (timeout moved to the `BattalionConfig`, `retry_attempts` kept on `ConclaveConfig`, comment that `aegis.retry` only shapes the backoff).
  - `maneuver_nested_flow` module doc and printed line now name `IgnoreErrors`, the Maneuver strategy it configures.

## Verification Results (all actually run)

| Check | Result |
| ----- | ------ |
| `cargo check -p paladin-ai --all-targets --all-features` | exit 0 |
| `cargo test -p paladin-ai --lib cli` | 8 passed |
| `make check-doc-examples` | exit 0 (README sync, doc-examples crate compiles, snippet scan) |
| `cargo test -p paladin-doc-examples` | 1 passed |
| `cargo clippy -p paladin-ai --examples --all-features -- -D warnings` | exit 0 |
| `make check-examples` | exit 0 (62 of 62 example binaries present, CI feature-split matrix) |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | exit 0 |
| `cargo fmt --check` | exit 0 |
| `make api-surface` | unchanged (4275 items) |
| `cargo test --workspace --no-fail-fast` | 6899 passed, **2 failed**: only `run_api_wiring::tests::build_run_api_persists_no_run_traces_by_default` and `..._persists_run_traces_when_trace_persist_is_set` |
| Runtime smoke runs (`cargo run --example` for commander_full_config, commander_basic, commander_auto, commander_with_metadata_export, formation_sequential, phalanx_parallel, campaign_workflow, conclave_expert_panel) | all exit 0; `commander_full_config` shows the retry then continue run succeeding on attempt 3 and the Absorb run recording a `node_errors` entry |

The two workspace failures are the known pre-existing `paladin-ai` lib failures recorded in `deferred-items.md` (identical on the pre-phase baseline). Per the brief, a run whose only failures are those two counts as passing; they were not touched.

Acceptance greps: `with_timeout(` count 0 and `per_attempt_timeout(` count 4 in the CLI file; `ErrorStrategy` count 0 and `with_aegis` count 1 in `orchestration.rs`; every Battalion example file has `with_aegis`, zero `with_timeout(` / `with_error_strategy(` / `with_retry_policy(` / `ContinueOnError` / `RetryThenContinue` hits; `grep -cw ContinueOnError examples/maneuver_nested_flow.rs` prints 0. The Maneuver examples' own `with_error_strategy(ErrorStrategy::IgnoreErrors/FailFast)` calls (a separate `ManeuverConfig` API) are unchanged and out of this plan.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Aegis import path in the CLI**
- **Found during:** Task 1
- **Issue:** The plan's interface note implies `crate::core::platform::container::aegis`, which does not exist in the root crate (the module is not re-exported there).
- **Fix:** Imported from `paladin_core::platform::container::aegis` (same as `src/config/agent_runtime.rs`).
- **Files modified:** `src/application/cli/commands/battalion.rs`
- **Commit:** `dc0b155c`

**2. [Environment] Disk exhaustion during the workspace test run**
- **Found during:** final verification
- **Issue:** First `cargo test --workspace` died with `No space left on device` while linking examples.
- **Fix:** Ran `rm -rf target/debug/examples target/debug/incremental` (as the brief allows) and reran; the second run completed. Not a code change.

Otherwise the plan was executed as written. Two small formatting passes (`cargo fmt`) were folded into the Task 1 commit before any later task started.

## Known Stubs

None.

## Threat Flags

None. T-44-21 (loss of the CLI whole-run 600 s / 300 s bound) is accepted as planned: the bound is now per attempt and the CLI rustdoc, template comment and `per_attempt_timeout` doc all say so; the 9.1 row is 44-11's. T-44-22 holds: the CLI's zero check is unchanged and `validate_aegis` rejects a zero bound.

## Next Phase Readiness

No in-tree code consumer calls `with_timeout(`, `with_error_strategy(` or `with_retry_policy(` on a Battalion or Conclave config. The remaining legacy references are the definitions themselves, the core `test_conclave_config_builder` (`.with_timeout(600)`) and docs pages (44-09 and 44-10 own those).

## Self-Check: PASSED

- Commits `dc0b155c`, `efcfb089`, `82137b49` exist in `git log`.
- All 13 files in the plan's `files_modified` exist and were modified by these commits.
