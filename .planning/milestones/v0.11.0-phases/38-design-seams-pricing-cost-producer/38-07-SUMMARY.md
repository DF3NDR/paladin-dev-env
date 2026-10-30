---
phase: 38-design-seams-pricing-cost-producer
plan: 07
subsystem: infra
tags: [rust, cost-arithmetic, treasurer, pricing, tdd, agent-loop, engine, cost-tally]

# Dependency graph
requires:
  - phase: 38-design-seams-pricing-cost-producer (plan 02)
    provides: "Cost, CurrencyCode, CostTally (None-poisons-the-run accumulation rule) in paladin-core"
  - phase: 38-design-seams-pricing-cost-producer (plan 04)
    provides: "LlmResponse.cost: Option<Cost>, PricingLlmAdapter::generate/generate_stream pricing the served model"
  - phase: 38-design-seams-pricing-cost-producer (plan 06)
    provides: "TraceEvent::NodeFinished.cost/RunFinished.cost carriers, TraceDispatcher::total_cost(), the single cost: None placeholder on superstep.rs's Paladin-attempt NodeFinished site"
provides:
  - "PaladinResult.cost: Option<Cost> additive field, #[serde(default, skip_serializing_if = \"Option::is_none\")], every in-tree literal migrated to cost: None in the same commit"
  - "Agent-loop CostTally accumulation in PaladinExecutionService::execute_internal: cost_tally.record_call(response.cost.as_ref()) folded at the same point usage += response.usage.clone() runs; the three real-model-call PaladinResult returns carry cost: cost_tally.total()"
  - "execute_structured_call forwards its single call's response.cost.clone() beside usage, so the structured-output \"raw\" PaladinResult the engine's structured arm reads (structured.raw.cost) carries a real value"
  - "Engine bridge: NodeDispatchResult extended to a 4-tuple with Option<Cost>; the Paladin arm's result.cost.clone() and the structured-output arm's structured.raw.cost.clone() replace 38-06's cost: None placeholder on the per-attempt TraceEvent::NodeFinished emission"
  - "execute_response_carries_no_cost_field (paladin-web) proves From<PaladinResult> for ExecuteResponse is unchanged -- no cost key reaches the HTTP agent response (D-11)"
affects: [38-08, 38-09, 39-treasurer-ledger]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Cost rides beside usage everywhere usage travels (D-10): every site that folds/forwards a response's TokenUsage into a run-level or attempt-level carrier gets an identical cost fold/forward at the same point -- the agent loop's cost_tally.record_call() beside usage +=, the engine's cost.clone() beside usage.clone() on every NodeDispatchResult arm"
    - "None propagates through CostTally.record_call: one unpriced call in a run poisons the whole run's cost total, never a silently-partial sum"

key-files:
  created: []
  modified:
    - crates/paladin-core/src/platform/container/execution_result.rs
    - crates/paladin-ports/src/output/paladin_port.rs
    - crates/paladin-web/src/agent_controller.rs
    - src/application/services/paladin/paladin_execution_service.rs
    - tests/unit/handoff_service_test.rs
    - crates/paladin-battalion/src/engine/test_support.rs
    - crates/paladin-battalion/src/engine/superstep.rs
    - crates/paladin-battalion/src/engine/mod.rs

key-decisions:
  - "execute_structured_call (the single deterministic model call StructuredExecutorPort dispatches through) forwards response.cost.clone() beside usage, even though Task 2's <action> text names only \"the loop\" (execute_internal) -- required so Task 3's structured-output arm (structured.raw.cost) has a real value to read rather than a permanent None, matching D-10's own \"cost rides beside usage everywhere\" rule."
  - "NodeTaskOutput (the per-node-task struct threading a completed attempt's usage/paladin_id/outcome to the bookkeeping loop) needed no cost field: the real per-attempt TraceEvent::NodeFinished emission happens inside the same spawned task, before NodeTaskOutput is constructed, and NodeExecutionRecord (the persisted Waypoint record) is deliberately not extended -- so the local cost variable is fully consumed at the emit call and never needs to survive past it."
  - "Both TDD tasks (2 and 3) used a verified RED/GREEN split: the failing-test commit was produced by stashing the implementation diff, running the new tests against the untouched tree (confirming the priced-sum test fails and the unpriced/no-pricing tests trivially pass), committing that state, then restoring the implementation for the GREEN commit."

requirements-completed: [PRICE-03]

coverage:
  - id: D1
    description: "PaladinResult.cost: Option<Cost> additive field with legacy-compatible serde (D-25 precedent); PaladinResult::new/Default leave it None; every in-tree literal and doctest migrated"
    requirement: PRICE-03
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/execution_result.rs#tests::paladin_result_without_cost_key_deserializes_to_none"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/execution_result.rs#tests::default_still_constructs"
        status: pass
      - kind: other
        ref: "cargo check --workspace --all-targets --all-features"
        status: pass
    human_judgment: false
  - id: D2
    description: "The agent loop sums each priced model call's cost into PaladinResult.cost via a per-run CostTally; None propagates when any call was unpriced or no pricing is installed, while usage is still reported in full"
    requirement: PRICE-03
    verification:
      - kind: unit
        ref: "src/application/services/paladin/paladin_execution_service.rs#agent_loop_cost_tests::agent_loop_sums_cost_across_calls"
        status: pass
      - kind: unit
        ref: "src/application/services/paladin/paladin_execution_service.rs#agent_loop_cost_tests::agent_loop_cost_is_none_when_any_call_unpriced"
        status: pass
      - kind: unit
        ref: "src/application/services/paladin/paladin_execution_service.rs#agent_loop_cost_tests::agent_loop_cost_is_none_without_pricing"
        status: pass
    human_judgment: false
  - id: D3
    description: "On the engine path each Paladin attempt's NodeFinished.cost is that attempt's own PaladinResult.cost (or the structured result's raw PaladinResult.cost); non-Paladin nodes and unpriced attempts carry None; RunFinished.cost is the sum of priced Paladin nodes, or None when any is unpriced"
    requirement: PRICE-03
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#engine::tests::run_finished_cost_sums_priced_paladin_nodes"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#engine::tests::run_finished_cost_is_none_when_a_paladin_node_is_unpriced"
        status: pass
      - kind: integration
        ref: "cargo test -p paladin-battalion --lib engine (551/551 passed, no regressions)"
        status: pass
    human_judgment: false
  - id: D4
    description: "The HTTP agent response is unchanged: ExecuteResponse built from a priced PaladinResult serializes with no cost key; no Run API or CLI surface starts showing cost this phase (D-11)"
    requirement: PRICE-03
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/agent_controller.rs#agent_controller::tests::execute_response_carries_no_cost_field"
        status: pass
      - kind: other
        ref: "git diff shows no line changed inside impl From<PaladinResult> for ExecuteResponse across this plan's commits"
        status: pass
    human_judgment: false

duration: ~25min
completed: 2026-09-26
status: complete
---

# Phase 38 Plan 07: Design Seams — Cost on PaladinResult and the Engine Bridge Summary

**`PaladinResult.cost: Option<Cost>` added and migrated everywhere, the agent loop's per-run `CostTally` folds each model call's cost beside its usage, and the engine's `NodeDispatchResult` bridges each Paladin attempt's real cost into `NodeFinished.cost` — closing the two gaps 38-06 left so `RunFinished.cost` is a real end-to-end figure.**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-09-26T14:08:00Z (approx)
- **Completed:** 2026-09-26T14:34:04Z
- **Tasks:** 3 (Task 1: auto; Task 2: TDD RED/GREEN; Task 3: TDD RED/GREEN)
- **Files modified:** 8

## Accomplishments

- **Task 1:** Added `pub cost: Option<Cost>` to `PaladinResult` directly after `usage`, with the `served_by`/D-25 `#[serde(default, skip_serializing_if = "Option::is_none")]` precedent. Migrated every in-tree full struct literal cargo check surfaced (5 sites in `paladin_port.rs`'s test module, 4 agent-loop return sites in `paladin_execution_service.rs`, 2 specialist-result mocks in `handoff_service_test.rs`) to carry `cost: None` — every `..Default::default()` site needed no change. Added `RecordingPaladinPort::set_output_with_usage_and_cost` beside `set_output_with_usage` in `engine/test_support.rs` for Task 3. Proved the HTTP boundary is untouched: `execute_response_carries_no_cost_field` converts a `PaladinResult` with `cost: Some(..)` and asserts the serialized `ExecuteResponse` has no `cost` key, and `From<PaladinResult> for ExecuteResponse`'s body shows zero diff across the whole plan.
- **Task 2 (TDD):** Added a per-run `CostTally` beside `usage` in `execute_internal`; `cost_tally.record_call(response.cost.as_ref())` folds at the exact point `usage += response.usage.clone()` already accumulates that call's `TokenUsage`. The three `PaladinResult` returns that follow a real model call now carry `cost: cost_tally.total()`; the one before-model-finish return (no model call made) stays `cost: None`. Also forwarded `execute_structured_call`'s single call's `response.cost.clone()` beside `usage`, so the structured-output "raw" `PaladinResult` Task 3's engine bridge reads from carries a real value rather than a permanent placeholder. RED commit: 3 new behavior tests, one failing as expected (`agent_loop_sums_cost_across_calls`). GREEN commit: all 3 pass, plus the full pre-existing `paladin_execution_service` suite (65/65) and `cargo clippy -p paladin-ai -- -D warnings` clean.
- **Task 3 (TDD):** Extended `type NodeDispatchResult` to a 4-tuple, inserting `Option<Cost>` directly after `TokenUsage`. Every construction site across `execute_vanguard_node` (the Function, Paladin, structured-output, and Battalion arms) and `race_attempt`'s two timeout arms now returns a cost value — `result.cost.clone()` for a plain Paladin call, `structured.raw.cost.clone()` for a structured-output success, `None` everywhere `usage` is `TokenUsage::default()`. Threaded `cost` through the per-attempt `(paladin_id, usage, cost, outcome)` tuple to the real Paladin-attempt `TraceEvent::NodeFinished` emission, replacing 38-06's placeholder; the cache-hit emission keeps `cost: None` unchanged. `NodeExecutionRecord` (the persisted Waypoint record) needed no field and no change — confirmed by an empty `git diff --stat` on `waypoint.rs` across every commit in this plan. RED commit: 2 new behavior tests, one failing as expected (`run_finished_cost_sums_priced_paladin_nodes`). GREEN commit: both pass, plus the full engine suite (551/551, no regressions) and `cargo clippy -p paladin-battalion -- -D warnings` clean (after factoring `RecordedOutput` out of `RecordingPaladinPort`'s output map to satisfy `clippy::type_complexity`).

## Task Commits

1. **Task 1: PaladinResult.cost — additive field, literals migrated, HTTP response unchanged** - `bf545123` (feat)
2. **Task 2 (RED): failing test for agent-loop cost accumulation** - `8594080d` (test)
3. **Task 2 (GREEN): fold each model call's cost into PaladinResult.cost** - `198fc3f7` (feat)
4. **Task 3 (RED): failing test for engine Paladin-attempt cost bridge** - `2cee9859` (test)
5. **Task 3 (GREEN): bridge each Paladin attempt's cost into NodeFinished.cost** - `f294662d` (feat)

**Plan metadata:** (this commit)

_Note: Tasks 2 and 3 both used the plan's `tdd="true"` RED/GREEN split (two commits each, no REFACTOR needed — both implementations were minimal on the first GREEN pass)._

## Files Created/Modified

- `crates/paladin-core/src/platform/container/execution_result.rs` - `PaladinResult.cost: Option<Cost>` additive field, `Default`/`new()` updated, two new legacy-compatibility/serialization tests
- `crates/paladin-ports/src/output/paladin_port.rs` - 5 test-module full literals migrated to `cost: None`
- `crates/paladin-web/src/agent_controller.rs` - `execute_response_carries_no_cost_field` test proving the HTTP boundary carries no `cost` key
- `src/application/services/paladin/paladin_execution_service.rs` - per-run `CostTally`, `record_call` folded beside `usage +=`, three `PaladinResult` returns now carry `cost: cost_tally.total()`, `execute_structured_call` forwards `response.cost.clone()`, new `agent_loop_cost_tests` module (3 tests, 1 `ScriptedCostLlmPort` test double)
- `tests/unit/handoff_service_test.rs` - 2 specialist-result mock literals migrated to `cost: None`
- `crates/paladin-battalion/src/engine/test_support.rs` - `RecordingPaladinPort::set_output_with_usage_and_cost`, `RecordedOutput` type alias (clippy::type_complexity)
- `crates/paladin-battalion/src/engine/superstep.rs` - `NodeDispatchResult` extended to a 4-tuple with `Option<Cost>`; every construction site in `execute_vanguard_node`/`race_attempt` updated; the real Paladin-attempt `TraceEvent::NodeFinished` emission carries the real `cost`
- `crates/paladin-battalion/src/engine/mod.rs` - two new engine tests (`run_finished_cost_sums_priced_paladin_nodes`, `run_finished_cost_is_none_when_a_paladin_node_is_unpriced`)

## Decisions Made

- `execute_structured_call` forwards `response.cost.clone()` beside `usage` even though Task 2's `<action>` text names only the reasoning loop — required so Task 3's structured-output arm (`structured.raw.cost`) has a real value to read, matching D-10's "cost rides beside usage everywhere usage travels" rule rather than leaving a second permanent placeholder for a later plan to discover.
- `NodeTaskOutput` needed no `cost` field: the real per-attempt `TraceEvent::NodeFinished` emission happens inside the same spawned task, before `NodeTaskOutput` is constructed, and the persisted `NodeExecutionRecord` is deliberately not extended (per the plan's own boundary) — so the local `cost` variable is fully consumed at the emit call and never needs to survive past it.
- Both TDD tasks used a verified RED/GREEN split produced by stashing the implementation diff (keeping only the new tests + already-existing test infrastructure), confirming the priced-sum test fails and the unpriced/no-pricing tests trivially pass, committing that state, then restoring the implementation for GREEN — rather than writing tests and implementation in one pass and asserting RED counterfactually.

## Deviations from Plan

None - plan executed exactly as written. The one extension beyond Task 2's literal wording (`execute_structured_call`'s cost forward) is a direct application of D-10's own stated design principle to a call site Task 3 already depends on, not a bug fix, missing-critical-functionality addition, or architectural change — recorded above under Decisions Made rather than as a deviation.

## Issues Encountered

None. `clippy::type_complexity` on `RecordingPaladinPort`'s output map (a three-element tuple inside a `HashMap`) was caught and fixed with a `RecordedOutput` type alias before the Task 3 GREEN commit — a routine clippy fix, not a deviation from the plan's design.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Both run paths now aggregate per-call costs into their run-level carriers end-to-end: `PaladinResult.cost` (agent loop) and `TraceEvent::RunFinished.cost` (engine, via 38-06's `TraceDispatcher::total_cost()` reading the real per-attempt `NodeFinished.cost` this plan supplies). The None-never-zero rule (D-00c) holds on both paths.
- 38-08's engine-side `ExecutionMetadata` producer can now read a real `RunFinished.cost`; the agent-loop producer (already built in 38-02) can now read a real `PaladinResult.cost` from `execute_internal`'s normal completion paths, not just the streamed path.
- `constructible_struct_adds_field = "allow"` already covers `paladin-core` crate-wide and `PaladinResult`'s MIGRATION.md §9.2 row already exists (from `served_by`/`usage`); 38-09 extends that row's Change cell after measuring, per the plan's own stated boundary — no MIGRATION.md/semver-checks-allowlist change was needed in this plan.
- No blockers. `NodeExecutionRecord` (the persisted Waypoint record) remains unextended, as designed — persisting cost is Phase 39's scope.

---
*Phase: 38-design-seams-pricing-cost-producer*
*Completed: 2026-09-26*

## Self-Check: PASSED

All modified files and commit hashes verified present on disk / in `git log --oneline --all`:
- `crates/paladin-core/src/platform/container/execution_result.rs` — FOUND (contains `pub cost: Option<Cost>`)
- `src/application/services/paladin/paladin_execution_service.rs` — FOUND (contains `cost_tally.record_call(`, `cost: cost_tally.total()`)
- `crates/paladin-battalion/src/engine/superstep.rs` — FOUND (`type NodeDispatchResult` contains `Option<Cost>`)
- `bf545123` (Task 1) — FOUND
- `8594080d` (Task 2 RED) — FOUND
- `198fc3f7` (Task 2 GREEN) — FOUND
- `2cee9859` (Task 3 RED) — FOUND
- `f294662d` (Task 3 GREEN) — FOUND

Re-ran the plan-level `<verification>` commands on the final tree: `cargo check --workspace --all-targets --all-features` (0 errors), `cargo test -p paladin-ai --lib paladin_execution_service` (65 passed), `cargo test -p paladin-battalion --lib engine` (551 passed), `cargo test -p paladin-web --lib agent_controller` (41 passed), `cargo test -p paladin-ai-core --doc` (102 passed), `cargo test -p paladin-ports --doc` (151 passed), `cargo fmt --check` (clean), `cargo clippy -p paladin-ai -p paladin-battalion -p paladin-ai-core -p paladin-ports -p paladin-web --all-targets -- -D warnings` (clean). All task-level `<acceptance_criteria>` re-verified against the final source.
