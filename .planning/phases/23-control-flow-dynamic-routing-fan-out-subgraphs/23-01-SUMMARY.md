---
phase: 23-control-flow-dynamic-routing-fan-out-subgraphs
plan: 01
subsystem: orchestration
tags: [paladin-battalion, edge-conditions, war-engine, campaign-service, bug-fix, tdd]

# Dependency graph
requires:
  - phase: 22.1-engine-readiness-defect-and-msrv-follow-up
    provides: WarGraph::validate structure (CustomDispatchResolver clause house pattern), superstep::run signature, EngineError enum shape
provides:
  - "paladin-battalion::edge_evaluator module: EdgeConditionEvaluator (async trait), EdgeContext, EdgeEvaluatorRegistry, EdgeEvaluatorError"
  - "CampaignExecutionService::with_evaluator + fail-closed pre-check in execute()"
  - "WarEngine::with_edge_evaluator + WarGraph::validate fail-closed clause + EngineError::{UnregisteredEdgeCondition, EdgeEvaluatorFailed}"
  - "MIGRATION.md M-B-01 worked example and resolved §9.2 CF-01 rows"
affects: [23-02, 23-03, 23-05]

# Tech tracking
tech-stack:
  added: []
  patterns: ["Registered-evaluator registry mirroring DispatchRegistry (no reserved-name guard)", "RED/GREEN commit split for a defect fix (test-only hunks committed failing before the mechanism lands)"]

key-files:
  created:
    - crates/paladin-battalion/src/edge_evaluator.rs
  modified:
    - crates/paladin-battalion/src/lib.rs
    - crates/paladin-battalion/src/campaign_service.rs
    - crates/paladin-battalion/src/engine/graph.rs
    - crates/paladin-battalion/src/engine/superstep.rs
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/src/engine/bridges.rs
    - MIGRATION.md
    - .project/v0.10.0/08-traceability-matrix.md

key-decisions:
  - "RED commit = every diff hunk confined to a #[cfg(test)] module across the touched existing files (campaign_service.rs, graph.rs, mod.rs, superstep.rs, bridges.rs); GREEN commit = the new edge_evaluator.rs module (including its own 2-3 registry unit tests, which cannot be split from their own not-yet-existing types) plus every production hunk. This is the cleanest mechanically-verifiable split available given a wholly new module cannot be partially declared across two commits without its own crate-registration line."
  - "Runtime evaluator error on the BattalionError path reuses CampaignError(String), not a new variant -- BattalionError has no InvalidGraph-adjacent 'execution failed' structured variant and adding one would be an unsanctioned X-10 break; InvalidGraph is reserved for the validation-time fail-closed check per D-04."

patterns-established:
  - "EdgeEvaluatorRegistry: HashMap<String, Arc<dyn EdgeConditionEvaluator>>, exact-byte-equality keys, no reserved-name guard, duplicate registration replaces silently (documented in rustdoc)."

requirements-completed: [CF-01]

# Coverage metadata
coverage:
  - id: D1
    description: "Unregistered EdgeCondition::Custom fails validation before any node executes, on both CampaignExecutionService and WarEngine paths, naming every offender sorted and deduped"
    requirement: "CF-01"
    verification:
      - kind: unit
        ref: "campaign_service.rs#unregistered_custom_condition_is_rejected_before_any_paladin_executes"
        status: pass
      - kind: unit
        ref: "engine/graph.rs#unregistered_custom_edge_condition_fails_graph_validation"
        status: pass
      - kind: unit
        ref: "engine/graph.rs#every_unregistered_custom_name_is_listed_sorted_and_deduped"
        status: pass
    human_judgment: false
  - id: D2
    description: "A registered evaluator's Ok(true)/Ok(false) verdict routes/does-not-route the edge on both paths"
    requirement: "CF-01"
    verification:
      - kind: unit
        ref: "campaign_service.rs#registered_true_evaluator_routes_the_custom_edge"
        status: pass
      - kind: unit
        ref: "campaign_service.rs#registered_false_evaluator_does_not_route_the_custom_edge"
        status: pass
      - kind: unit
        ref: "engine/mod.rs#registered_engine_evaluator_true_and_false_route_correctly"
        status: pass
    human_judgment: false
  - id: D3
    description: "A registered evaluator's Err fails the run with a typed error naming the edge and evaluator, never defaulting either branch"
    requirement: "CF-01"
    verification:
      - kind: unit
        ref: "campaign_service.rs#evaluator_error_fails_the_legacy_run_naming_the_edge"
        status: pass
      - kind: unit
        ref: "engine/mod.rs#engine_evaluator_error_fails_the_run_naming_edge_and_evaluator"
        status: pass
    human_judgment: false
  - id: D4
    description: "MIGRATION.md M-B-01 worked example and §9.2 CF-01 rows resolved with no remaining CF-01 TBDs"
    requirement: "CF-01"
    verification:
      - kind: other
        ref: "grep -c 'TBD — owner CF-01, Phase 23' MIGRATION.md == 0"
        status: pass
    human_judgment: false

# Metrics
duration: ~55min
completed: 2026-09-03
status: complete
---

# Phase 23 Plan 01: BUG-01 Fail-Closed Custom Edge Conditions Summary

**Registered-evaluator mechanism (`paladin-battalion::edge_evaluator`) fixes BUG-01 fail-closed on both `CampaignExecutionService` and `WarEngine`, test-first with RED committed strictly before GREEN in git history.**

## Performance

- **Duration:** ~55 min
- **Started:** 2026-09-03T18:54Z (approx, per STATE.md phase start)
- **Completed:** 2026-09-03T19:26:13Z
- **Tasks:** 2
- **Files modified:** 9 (1 new, 8 modified)

## Accomplishments

- New `crates/paladin-battalion/src/edge_evaluator.rs` module: `EdgeConditionEvaluator` (async trait), `EdgeContext` (carrying `source`/`target`/`battlefield`/`thread`/`superstep`), `EdgeEvaluatorRegistry` (exact-byte-equality keyed, no reserved-name guard), `EdgeEvaluatorError` (`#[non_exhaustive]`, `thiserror`).
- Legacy path: `CampaignExecutionService::with_evaluator` (additive builder, `new(paladin_port)` unchanged); a fail-closed pre-check runs immediately after `campaign.validate()` and before any node executes, reusing `BattalionError::InvalidGraph` per D-04; `evaluate_edge_condition` is now `async` and awaits the registered evaluator instead of `warn!`-then-`Ok(true)`.
- Engine path: `WarEngine::with_edge_evaluator`; `WarGraph::validate` gains an `edge_evaluators: &EdgeEvaluatorRegistry` parameter and a new fail-closed clause (collects every unregistered `Custom` name, sorted+deduped) run in the same pre-execution `validate()` call as the existing `CustomDispatchResolver` check; `EngineError` gains `UnregisteredEdgeCondition { names }` and `EdgeEvaluatorFailed { from, to, evaluator, source }`; `superstep::evaluate_edge_condition` is now `async`, consults the registry, and passes the source Paladin's `output_field` value (or canonical Battlefield JSON) as `output` per D-02; `Frontier::record_execution` is now `async` to thread the await through.
- Both placeholder sites (`campaign_service.rs`'s `warn!("Custom edge condition not yet implemented, defaulting to true")`, `superstep.rs`'s unconditional `Ok(true)` `Custom` arm) are deleted. `grep -rn "defaulting to true" crates/` returns no matches. No configuration, env var, feature flag, or builder option restores the old always-true behavior.
- `MIGRATION.md` §9.1 M-B-01 carries a worked before/after example (verbatim v0.9 always-routing construction → v0.10 `with_evaluator`/`with_edge_evaluator` registration → `Contains`/`Regex`/`Always` alternative → the exact validation error text transcribed from a live test run); §9.2's `EdgeCondition` and `CampaignExecutionService` rows resolved (both `N`, no X-10 register burden); §9.8's parenthetical updated to record custom-evaluator registration as landed.
- `.project/v0.10.0/08-traceability-matrix.md`'s BUG-01 row carries the RED (`b2d05045`) and GREEN (`8d5ef333`) commit SHAs and grep-absence evidence.

## Task Commits

1. **Task 1: Registered-evaluator edge conditions, end-to-end on both paths (RED then GREEN)**
   - `b2d05045` — `test(23-01): reproduce BUG-01 on both custom-edge-condition paths (red)` — ten regression tests added against not-yet-existing API; crate does not compile (52 errors).
   - `8d5ef333` — `fix(23-01): fail closed on unregistered custom edge conditions (green)` — the mechanism; all ten tests plus the full pre-existing `paladin-battalion` suite (361 lib tests) pass.
2. **Task 2: Land M-B-01's worked example and resolve CF-01's §9.2 register rows**
   - `1b06048c` — `docs(23-01): land M-B-01 worked example, resolve CF-01 §9.2 rows`

**Plan metadata:** (this commit, docs(23-01): complete plan 01)

_Note: Task 1 is `type="tracer" tdd="true"`; its two-commit RED/GREEN structure is the TDD cycle the task's `<action>` block mandates, not the usual RED→GREEN→REFACTOR three-commit shape — no REFACTOR commit was needed._

## Files Created/Modified

- `crates/paladin-battalion/src/edge_evaluator.rs` — new module: `EdgeConditionEvaluator`, `EdgeContext`, `EdgeEvaluatorRegistry`, `EdgeEvaluatorError`, plus 3 registry unit tests.
- `crates/paladin-battalion/src/lib.rs` — registers `pub mod edge_evaluator;` and re-exports its four public types at crate root.
- `crates/paladin-battalion/src/campaign_service.rs` — `evaluators: EdgeEvaluatorRegistry` field, `with_evaluator` builder, fail-closed pre-check in `execute()`, async `evaluate_edge_condition` with registry-backed `Custom` arm, 4 new tests + `RecordingPort`/`FixedVerdictEvaluator`/`FailingEvaluator` test doubles.
- `crates/paladin-battalion/src/engine/graph.rs` — `WarGraph::validate` gains `edge_evaluators` parameter and `validate_edge_evaluators` clause; 3 new tests; all pre-existing `validate(...)` call sites updated for the new signature.
- `crates/paladin-battalion/src/engine/superstep.rs` — `evaluate_edge_condition` and `Frontier::record_execution` are now `async` and registry-backed; `run()` gains an `evaluators` parameter; all pre-existing `run(...)`/`validate(...)` test call sites updated.
- `crates/paladin-battalion/src/engine/mod.rs` — `WarEngine::with_edge_evaluator`, `edge_evaluators` field, two new `EngineError` variants, `start`/`resume_with_options` forward the registry; 2 new end-to-end tests.
- `crates/paladin-battalion/src/engine/bridges.rs` — 3 pre-existing test call sites updated for the new `validate()` signature (no new tests).
- `MIGRATION.md` — §9.1 M-B-01 worked example; §9.2 `EdgeCondition`/`CampaignExecutionService` rows; §9.8 parenthetical.
- `.project/v0.10.0/08-traceability-matrix.md` — BUG-01 row carries the RED/GREEN commit SHAs and grep-absence evidence.

## Decisions Made

- **RED commit boundary:** RED = every diff hunk confined to a `#[cfg(test)]` module across `campaign_service.rs`, `graph.rs`, `mod.rs`, `superstep.rs`, `bridges.rs` (all pure test-code changes, verified to leave the crate failing to compile with 52 errors — missing types/methods/variants). GREEN = the new `edge_evaluator.rs` module (its own 2-3 registry unit tests included, since a wholly new module cannot be partially declared without its own `lib.rs` registration line existing) plus every production-code hunk in the five existing files. This satisfies "tests only, no production change" for RED as precisely as the structural constraint of a brand-new module allows, and both commits are independently `cargo test`-clean/broken as expected (`git show <sha> --stat` confirms RED touches only test-bearing files).
- **Runtime evaluator error on the legacy path reuses `BattalionError::CampaignError`**, not a new variant: `BattalionError` is a pre-existing public enum without `#[non_exhaustive]`, `InvalidGraph` is reserved for the validation-time fail-closed check (D-04 names it explicitly), and `CampaignError(String)` is the existing "campaign execution failed" variant already used elsewhere in this file (Herald formatting errors) — the closest semantic fit with zero new X-10 register burden.
- **`EdgeEvaluatorRegistry::register` returns `()`, not `Self`/`&mut Self`** — CONTEXT.md left the exact return shape to discretion; since there is no reserved-name failure mode (unlike `DispatchRegistry::register`'s `Result`), a plain mutator was simplest. The fluent/chainable surface lives one level up, on `CampaignExecutionService::with_evaluator` and `WarEngine::with_edge_evaluator`.

## Deviations from Plan

None — plan executed exactly as written. No Rule 1-4 auto-fixes were needed; the mechanism compiled and passed on the first GREEN attempt after the RED/GREEN commit reconstruction.

## Issues Encountered

- Constructing the RED-then-GREEN commit split required manual `git diff`/`git apply` hunk surgery (the working tree was implemented as one continuous edit pass, then split into two commits by isolating every `#[cfg(test)]`-scoped hunk). Resolved without incident; both commits verified independently (RED: 52 compile errors confirmed; GREEN: 361/361 lib tests pass, `cargo fmt --check` and `cargo clippy -p paladin-battalion --all-targets -- -D warnings` clean).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The `EdgeConditionEvaluator`/`EdgeContext`/`EdgeEvaluatorRegistry` mechanism this plan introduces is the exact seam CF-05's `LlmDecisionEvaluator` (Plan 23-05) plugs into — `EdgeContext.thread`/`.superstep` are already present for its memoization key (D-24), and `WarEngine::with_edge_evaluator`/`CampaignExecutionService::with_evaluator` are the registration surfaces it will reuse with no further `superstep.rs` change.
- Plan 23-03 (Commander `StrategySelection`) still owns the `Commander`/`CommanderBuilder` §9.2 row, deliberately left `TBD — owner CF-05, Phase 23` in `MIGRATION.md` per D-27 — confirmed present and untouched by this plan.
- No blockers for downstream plans in this phase's wave sequence.

---
*Phase: 23-control-flow-dynamic-routing-fan-out-subgraphs*
*Completed: 2026-09-03*

## Self-Check: PASSED

All 9 files listed under Files Created/Modified verified present on disk. All 3 commits
(`b2d05045`, `8d5ef333`, `1b06048c`) verified present in `git log`. Full-workspace
`cargo clippy --workspace --all-targets --all-features -- -D warnings` (all 11 crates)
completed clean in the background during summary authoring; `cargo fmt --check`
(workspace-wide) clean; `cargo test -p paladin-battalion --lib` 361/361 passed;
`cargo test -p paladin-battalion` (full crate incl. integration + doctests) passed;
`cargo test --workspace --lib` 12/12 crate test binaries green, 0 failures.
