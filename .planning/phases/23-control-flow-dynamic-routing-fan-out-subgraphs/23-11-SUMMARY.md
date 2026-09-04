---
phase: 23-control-flow-dynamic-routing-fan-out-subgraphs
plan: 11
subsystem: testing
tags: [rust, paladin-battalion, war-engine, muster, e2e, integration-test, multi-thread-stress, tokio]

# Dependency graph
requires:
  - phase: 23-control-flow-dynamic-routing-fan-out-subgraphs
    provides: "23-05's Muster dynamic fan-out mechanism (worker templates, task_key-ordered aggregation, payload isolation), 23-06's mid-muster progress Waypoints, 23-09's real-Paladin dispatch path and Waypoint history API this plan exercises end-to-end"
provides:
  - "tests/integration/e2e_muster_defer_order_test.rs -- program scenario E2E-3's muster/defer/order half proven end-to-end through the real WarEngine + real Paladin dispatch path, with a clearly marked Phase 25 / FT-FR-06 seam for the recovering-worker half"
  - "engine::superstep::tests::fifty_task_muster_runs_to_completion_under_multi_thread / fifty_task_muster_is_deterministic_across_repeats -- the X-05 50-task multi-thread stress test with exact counts and a timeout guard"
affects: [24-hitl-parley-resume, 25-per-task-retry-and-aegis]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A Function planner (not a Paladin planner) drives an E2E test's Muster directive when the scenario's mock Paladin port cannot script a JSON Directive envelope -- mirrors e2e_crash_resume_test.rs's LoopGateNode precedent for deterministic control flow, while the mustered WORKERS themselves are real Paladin nodes through the mock port, keeping the fan-out/dispatch path itself genuinely exercised."
    - "Manual attempt scripting as the D-17 stand-in for a not-yet-built per-task retry policy: FaultyPaladinPort's fail_until_attempt counter is GLOBAL across every execute() call through one port instance, so pre-driving N-1 dummy calls through that SAME instance before the real run advances the shared counter past the failure threshold -- every real dispatch inside the actual muster then lands post-recovery, with the exact replacement seam for Phase 25's real Aegis retry marked in a fenced comment block."
    - "Superstep-complete vs muster-progress Waypoint disambiguation when reading persisted history back: a Muster's intra-superstep progress Waypoints (muster_progress: Some) each carry a cumulative, still-growing snapshot of `completed` records as tasks finish one at a time -- counting node completions across ALL waypoints in a superstep double/triple/quadruple-counts; filtering to muster_progress.is_none() first (the one final waypoint per superstep) is required before counting completed-node records."
    - "X-05 multi-thread stress test house pattern reused verbatim from src/application/services/orchestration/listener.rs: #[tokio::test(flavor = \"multi_thread\")] + an explicit tokio::time::timeout wrapping the run + exact-count (never lower-bound) assertions, so a deadlock in the muster dispatch/semaphore path fails loudly instead of hanging the suite."

key-files:
  created:
    - tests/integration/e2e_muster_defer_order_test.rs
  modified:
    - crates/paladin-battalion/src/engine/superstep.rs
    - Cargo.toml

key-decisions:
  - "Task 1 is a pure characterization/proof test with zero production-code changes: the plan's own files_modified list is [tests/integration/e2e_muster_defer_order_test.rs, Cargo.toml] -- no crates/ files. All Muster fan-out, deferred aggregation and real-Paladin-dispatch mechanism this test exercises already shipped in Plans 23-05/23-06/23-08/23-09. Per the TDD gate's own fail-fast guidance ('the feature may already exist -- investigate'), this was investigated and confirmed: the test passing on first correct compile is the CORRECT outcome for a task whose entire purpose is proving already-built behavior end-to-end, not adding new behavior. No RED/GREEN split was constructed; a single commit was used."
  - "The recovering-worker half of E2E-3 is exercised via manual attempt scripting (pre-driving FaultyPaladinPort's shared global counter past its fail_until_attempt threshold with dummy calls before the real muster run), not via engine-level resume-after-node-failure. Investigated the alternative (letting a real muster task fail, then calling engine.resume()) and found it unsupported by design: the Failed-status Waypoint node_failure produces carries muster_progress: None (superstep.rs's node_failure branch always passes None to build_waypoint), so WarEngine::resume_with_options's mid-muster branch (which requires latest.muster_progress.is_some()) never triggers on it -- resuming from a genuine muster-task failure is exactly the gap Phase 25's Aegis retry policy is meant to fill, not a path this phase implements or should route a test through."
  - "TASK_KEYS uses \"a\"..\"e\" (5 keys) for the E2E-3 integration test and zero-padded \"000\"..\"049\" for the 50-task stress test, both chosen so lexicographic (task_key) order trivially equals construction order -- keeping the test's OWN assertion construction simple while the underlying engine still sorts by task_key internally (proven under real reversed-delay concurrency by the already-passing unit test worker_deltas_merge_in_task_key_order_not_completion_order, which this plan's tests build on rather than re-derive)."
  - "The 50-task stress test uses lightweight CountingFunctionNode workers, not mock-Paladin round trips, per the plan's own action item 4 -- keeping paladin-battalion --lib's full 474-test suite at ~2-3s (well under the 120s bar), since that suite doubles as this crate's per-task sampling command."

patterns-established:
  - "Reading back Waypoint history to verify muster invariants (worker/aggregator superstep placement, one-superstep-complete-Waypoint-per-superstep) must filter on muster_progress.is_none() before counting completed-node records, or a Muster's own progress Waypoints (which carry cumulative snapshots) will multiply-count the same task completions."

requirements-completed: [CF-03]

# Coverage metadata
coverage:
  - id: D1
    description: "A planner node musters 5 workers that all run in one superstep through the real WarEngine + real Paladin dispatch path (FaultyPaladinPort), and a defer:true aggregator downstream runs exactly once, strictly after every worker's superstep"
    requirement: "CF-03"
    verification:
      - kind: e2e
        ref: "tests/integration/e2e_muster_defer_order_test.rs#planner_musters_five_workers_and_the_deferred_aggregator_runs_once"
        status: pass
    human_judgment: false
  - id: D2
    description: "The list-dispatch aggregated Battlefield field holds exactly 5 worker results in deterministic task_key order, not completion order"
    requirement: "CF-03"
    verification:
      - kind: e2e
        ref: "tests/integration/e2e_muster_defer_order_test.rs#aggregated_results_are_exactly_five_in_task_key_order"
        status: pass
    human_judgment: false
  - id: D3
    description: "The recovering-worker half of E2E-3 is exercised via a manually-succeeding-on-attempt-N mock (FaultyPaladinPort's shared global attempt counter, pre-driven before the real run) rather than a real Aegis retry policy, with the run still producing all 5 results, at a clearly marked Phase 25 / FT-FR-06 seam"
    requirement: "CF-03"
    verification:
      - kind: e2e
        ref: "tests/integration/e2e_muster_defer_order_test.rs#one_worker_recovers_by_manual_attempt_scripting"
        status: pass
    human_judgment: false
  - id: D4
    description: "The ENG-FR-11 clarification holds through the full E2E path: exactly one superstep-complete Waypoint (muster_progress: None) per superstep index, with the muster's 5 intra-superstep progress Waypoints (muster_progress: Some) counted separately"
    requirement: "CF-03"
    verification:
      - kind: e2e
        ref: "tests/integration/e2e_muster_defer_order_test.rs#run_completes_with_a_single_superstep_complete_waypoint_per_superstep"
        status: pass
    human_judgment: false
  - id: D5
    description: "E2E-1 (tests/integration/e2e_crash_resume_test.rs) remains byte-identical and green -- the golden this phase must not move"
    requirement: "CF-03"
    verification:
      - kind: e2e
        ref: "tests/integration/e2e_crash_resume_test.rs#e2e_1_crash_resume_matches_control_run_with_no_reexecution"
        status: pass
    human_judgment: false
  - id: D6
    description: "A 50-task muster runs to completion under a real #[tokio::test(flavor = \"multi_thread\")] runtime, wrapped in an explicit timeout guard, asserting exactly 50 worker executions, exactly 50 aggregated entries in sorted task_key order, all 50 keys distinct, and exactly 1 aggregator execution -- not in the default suite's #[ignore] list"
    requirement: "CF-03"
    verification:
      - kind: unit
        ref: "engine::superstep::tests::fifty_task_muster_runs_to_completion_under_multi_thread"
        status: pass
    human_judgment: false
  - id: D7
    description: "The 50-task muster's final Battlefield is byte-identical across 3 repeated multi-thread runs, proving determinism under real thread interleaving rather than a single lucky run"
    requirement: "CF-03"
    verification:
      - kind: unit
        ref: "engine::superstep::tests::fifty_task_muster_is_deterministic_across_repeats"
        status: pass
    human_judgment: false

# Metrics
duration: ~55min
completed: 2026-09-04
status: complete
---

# Phase 23 Plan 11: E2E-3 Muster/Defer/Order Integration Test and the X-05 50-Task Stress Test Summary

**Program scenario E2E-3's muster/defer/order half proven end-to-end through the real `WarEngine` + real Paladin dispatch path (planner musters 5, `defer: true` aggregator runs once, 5 results in `task_key` order), plus a 50-task `#[tokio::test(flavor = "multi_thread")]` stress test with exact counts and a timeout guard — both exercising already-shipped Muster machinery from Plans 23-05/23-06/23-08/23-09, no new production code.**

## Performance

- **Duration:** ~55 min
- **Completed:** 2026-09-04
- **Tasks:** 2
- **Files modified:** 1 created (`tests/integration/e2e_muster_defer_order_test.rs`), 2 modified (`crates/paladin-battalion/src/engine/superstep.rs`, `Cargo.toml`)

## Accomplishments

- **`tests/integration/e2e_muster_defer_order_test.rs`** (new `[[test]]` target `e2e_muster_defer_order`): a Tier 1 integration test proving E2E-3's muster/defer/order half through the real `WarEngine`, a real `NodeSpec::paladin` worker template, and `FaultyPaladinPort`. A deterministic `Function` planner (mirroring `e2e_crash_resume_test.rs`'s `LoopGateNode`) musters 5 workers keyed `"a"`..`"e"`; the worker template writes each Paladin's raw output into an `Append`-dispatched field; a `defer: true` `Function` aggregator copies the 5 results into the list-dispatch `aggregated` field. Four named tests:
  - `planner_musters_five_workers_and_the_deferred_aggregator_runs_once` — exactly 5 worker executions, all in the SAME superstep, exactly 1 aggregator execution at a strictly later superstep.
  - `aggregated_results_are_exactly_five_in_task_key_order` — the aggregated field holds exactly the 5 expected strings in `task_key` order.
  - `one_worker_recovers_by_manual_attempt_scripting` — the recovering-worker half, scripted via `FaultyPaladinPort`'s shared global `fail_until_attempt` counter pre-driven with 2 dummy warm-up calls before the real run, at an unmistakably fenced Phase 25 / FT-FR-06 comment block naming exactly what a real Aegis retry policy will replace.
  - `run_completes_with_a_single_superstep_complete_waypoint_per_superstep` — the ENG-FR-11 clarification holds end-to-end: exactly one superstep-complete Waypoint (`muster_progress: None`) per superstep, with the muster's 5 progress Waypoints (`muster_progress: Some`) counted separately.
- **`Cargo.toml`**: `[[test]] name = "e2e_muster_defer_order"` entry, same shape as `e2e_crash_resume` (no `required-features`).
- **`crates/paladin-battalion/src/engine/superstep.rs`**: two new tests appended to the existing `engine::superstep::tests` module, following `src/application/services/orchestration/listener.rs`'s X-05 house pattern (`#[tokio::test(flavor = "multi_thread")]` + explicit `tokio::time::timeout` + exact-count assertions):
  - `fifty_task_muster_runs_to_completion_under_multi_thread` — a 50-task muster (inside `EngineLimits::max_muster_tasks`'s default of 100) using lightweight `CountingFunctionNode`s, asserting exactly 50 worker executions, exactly 50 aggregated entries in sorted `task_key` order, all 50 keys distinct, exactly 1 aggregator execution.
  - `fifty_task_muster_is_deterministic_across_repeats` — 3 repeats, each producing a byte-identical final `Battlefield`.
  - Neither is `#[ignore]`d; `paladin-battalion --lib`'s full 474-test suite still runs in ~2-3s.
- E2E-1 (`tests/integration/e2e_crash_resume_test.rs`) verified byte-identical (`git diff` empty) and green throughout.

## Task Commits

Each task was committed atomically:

1. **Task 1: E2E-3's muster/defer/ordering half as an integration test** — `0358b0c1` (test)
2. **Task 2: The 50-task multi-thread muster stress test** — `87daa35d` (test)

**Plan metadata:** this file's own commit (docs), created by the worktree-mode `git_commit_metadata` step.

_Note: Both tasks carry `tdd="true"` in the plan, but neither modifies any production code — both are pure characterization/proof tests of already-shipped Muster mechanism (Plans 23-05/23-06/23-08/23-09). No RED/GREEN split applies; see Decisions Made below for the investigation and rationale._

## TDD Gate Compliance

Not applicable in the traditional sense: neither task's `<files>` list includes a production-code (`crates/`) file requiring new behavior — Task 1's files are `[tests/integration/e2e_muster_defer_order_test.rs, Cargo.toml]` and Task 2's is `[crates/paladin-battalion/src/engine/superstep.rs]` but only ADDS new `#[cfg(test)]` test functions, no non-test production code. Both tasks are `test(23-11): ...` commits proving already-built behavior; there is no corresponding `feat(23-11): ...` commit because no new feature was implemented in this plan. This matches the MVP+TDD gate's own behavior-adding-task predicate (tdd="true" AND `<behavior>` block AND non-test source files in `<files>`) — both tasks are test-only by that predicate and are exempt from the RED/GREEN gate.

## Files Created/Modified

- `tests/integration/e2e_muster_defer_order_test.rs` — new E2E-3 muster/defer/order integration test (4 tests, plus 26 shared `tests/helpers/` tests pulled in by the `[path = "../helpers/mod.rs"]` module).
- `crates/paladin-battalion/src/engine/superstep.rs` — `fifty_task_muster_graph` fixture helper plus 2 new stress tests appended to the existing `engine::superstep::tests` module.
- `Cargo.toml` — `[[test]]` entry for the new integration test target.

## Decisions Made

- **Task 1 has zero production-code changes and needed no RED/GREEN split.** Investigated per the TDD gate's fail-fast guidance ("the feature may already exist") and confirmed: every mechanism this test exercises (worker templates, `NextStep::Muster` validation/dispatch, mid-muster progress Waypoints, `defer: true` aggregation, real-Paladin dispatch through `FaultyPaladinPort`) already shipped in Plans 23-05, 23-06, 23-08 and 23-09. The test compiling and passing on the first correct attempt is the expected, correct outcome for a task whose entire purpose (per the plan's own objective: "Prove the phase's map-reduce capability end-to-end") is proving already-built behavior, not building new behavior. Committed as a single `test(23-11): ...` commit.
- **The recovering-worker half uses manual attempt scripting (pre-driven warm-up calls through the shared `FaultyPaladinPort` counter), not engine-level resume-after-failure.** Investigated the alternative directly: `superstep.rs`'s `node_failure` branch always builds its `Failed`-status Waypoint with `muster_progress: None` (verified by reading the exact `build_waypoint` call site), so `WarEngine::resume_with_options`'s mid-muster branch — which only activates when `latest.muster_progress.is_some()` — never triggers after a genuine muster-task failure. Resuming from a real muster-task failure is precisely the gap Phase 25's Aegis retry policy exists to fill, not a supported path in this phase. The chosen design (pre-advance the port's shared global counter with dummy `execute()` calls before the real muster run, so every real dispatch lands post-recovery) satisfies D-17's literal text ("a manually-succeeding-on-attempt-N mock") without depending on unimplemented retry machinery, and is fenced with an unmistakable comment block naming exactly what FT-FR-06 (Phase 25) will replace.
- **`TASK_KEYS` chosen so lexicographic order trivially equals construction order** (`"a".."e"` for the E2E test, zero-padded `"000".."049"` for the stress test) — keeps each test's OWN assertion construction simple, while the underlying `task_key`-sort-under-real-concurrency guarantee is already proven by the existing unit test `worker_deltas_merge_in_task_key_order_not_completion_order` (Plan 23-05), which this plan's tests build on rather than re-derive.
- **The 50-task stress test uses `CountingFunctionNode`, not mock-Paladin round trips**, per the plan's own action item 4 — `paladin-battalion --lib`'s full 474-test suite (this crate's per-task sampling command) still runs in ~2-3s, well under the plan's 120s bar.

## Deviations from Plan

None — plan executed exactly as written. Both tasks' `<behavior>`/`<acceptance_criteria>` are satisfied by named, passing tests; no auto-fixes were needed (no bugs, no missing critical functionality, no blocking issues encountered).

## Issues Encountered

None. One investigation worth recording (not an issue, since it confirmed the plan's own design rather than surfacing a defect): confirmed by direct code reading, not assumption, that a genuine muster-task failure's `Failed`-status Waypoint carries `muster_progress: None` and therefore cannot be resumed through the existing mid-muster resume branch — this is exactly why D-17 specifies a manually-succeeding mock rather than real resume-after-failure for this phase, and the investigation is recorded under Decisions Made above.

## User Setup Required

None — no external service configuration required. Both tasks are Tier 1 (mocks plus real in-process engine/Battlefield/Waypoint types via `SqliteWaypointStore`/`RecordingWaypointStore`), no Docker or external service dependency (D-30).

## Next Phase Readiness

- E2E-3's muster/defer/order half is fully proven end-to-end (CF-03 / PRD 02 §3 acceptance criterion 2); the recovering-worker half's seam is unmistakably marked for Phase 25 (`FT-FR-06`) to replace the scripted warm-up with a real Aegis retry policy — the seam's own comment block states exactly which assertions the real policy must continue to satisfy.
- The X-05 50-task multi-thread stress test (PRD 02 §4 item 8) runs in the default suite, not `#[ignore]`d, and is fast enough (~2-3s as part of the 474-test `paladin-battalion --lib` suite) not to degrade the crate's per-task sampling command.
- E2E-1 (`tests/integration/e2e_crash_resume_test.rs`) remains the phase's unmoved golden — verified byte-identical and green after both tasks.
- No blockers for Plan 23-12 or Phase 23's remaining wave. No production code was touched by this plan, so no downstream plan needs to account for a new type, error variant, or API surface change from this one.

---
*Phase: 23-control-flow-dynamic-routing-fan-out-subgraphs*
*Completed: 2026-09-04*

## Self-Check: PASSED

- FOUND: `tests/integration/e2e_muster_defer_order_test.rs`
- FOUND: `crates/paladin-battalion/src/engine/superstep.rs`
- FOUND: `Cargo.toml` (contains the `e2e_muster_defer_order` `[[test]]` entry)
- FOUND: commit `0358b0c1` (Task 1, `test(23-11): E2E-3 muster/defer/order integration test`)
- FOUND: commit `87daa35d` (Task 2, `test(23-11): 50-task multi-thread muster stress test`)
- `cargo test --test e2e_muster_defer_order`: 30/30 passed, 0 failed (4 named E2E-3 tests + 26 shared `tests/helpers/` tests).
- `cargo test -p paladin-battalion --lib engine::superstep`: 85/85 passed, 0 failed, 0 ignored.
- `cargo test --test e2e_crash_resume`: 27/27 passed, 0 failed; `git diff HEAD~2 -- tests/integration/e2e_crash_resume_test.rs`: empty (E2E-1 unmoved).
- `cargo test --workspace`: every `test result:` line reports 0 failed (all pre-existing `ignored` counts unrelated to this plan).
- `cargo fmt --check`: clean.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean.
- `cargo test -p paladin-battalion --lib`: 474/474 passed in ~2-3s, well under the plan's 120s bar.
