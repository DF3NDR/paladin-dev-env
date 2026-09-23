---
phase: 23-control-flow-dynamic-routing-fan-out-subgraphs
plan: 02
subsystem: orchestration
tags: [paladin-core, paladin-battalion, war-engine, directive, goto, control-flow, tdd]

# Dependency graph
requires:
  - phase: 23-control-flow-dynamic-routing-fan-out-subgraphs
    provides: "23-01's paladin-battalion::edge_evaluator module, WarEngine::with_edge_evaluator, WarGraph::validate's registry parameter (unmodified by this plan)"
provides:
  - "paladin-core::platform::container::directive: Directive { delta, next }, NextStep::{Edges,Goto,Muster,End,Parley}, MusterTask, impl From<StateDelta> for Directive"
  - "StateNode::run returning Result<Directive, NodeError> -- the trait every Function node in the workspace now implements"
  - "Validated Goto routing (EngineError::GotoUnknownNode), bounded by the existing max_node_visits, injected into next_vanguard bypassing Frontier::is_ready"
  - "D-08c's uniform NotFiring rule for any non-Edges NextStep, via Frontier::record_execution's new force_notfiring parameter"
  - "End run-completion semantics (NodeOutcomeKind::Ended, End-over-Goto precedence, scoped StarvedNodeAtCompletion suppression) and typed EngineError::ParleyNotSupported"
affects: [23-03, 23-04, 23-06, 23-08]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "RED/GREEN commit pairs per task (test(23-02) then feat(23-02)), matching 23-01's precedent: RED confines every hunk to new #[cfg(test)] test functions referencing not-yet-existing API; GREEN lands the new module plus every production hunk, including cross-file signature-migration fixups the new module's trait change forces."
    - "CountingFunctionNode::with_directive: a directive_fn(run_index, &Battlefield) -> Directive closure generalizing the existing delta_fn constructor, so a single test fixture can drive Goto/End/Muster/Parley or vary NextStep by run index (a refine-loop reviewer)."
    - "Per-superstep runtime locals (goto_targets, notfiring_nodes, end_requested, routing_failure) collected in the existing per-node accumulation loop, never persisted to Frontier -- Goto injection happens as a union over compute_next_vanguard's result, keeping that function pure over Frontier state."

key-files:
  created:
    - crates/paladin-core/src/platform/container/directive.rs
  modified:
    - crates/paladin-core/src/platform/container/mod.rs
    - crates/paladin-core/src/platform/container/waypoint.rs
    - crates/paladin-battalion/src/engine/node.rs
    - crates/paladin-battalion/src/engine/superstep.rs
    - crates/paladin-battalion/src/engine/graph.rs
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/src/engine/test_support.rs
    - tests/integration/e2e_crash_resume_test.rs
    - tests/integration/war_engine_tracer_test.rs
    - tests/integration/waypoint_retention_fault_injection_test.rs
    - examples/war_engine_memory_baseline.rs
    - benches/engine_benchmarks.rs

key-decisions:
  - "Task boundary reconstructed via git surgery, not a single combined commit: Task 1's GREEN state deliberately treats End/Muster/Parley with only D-08c's NotFiring marking (no run-completion or rejection behavior yet, NodeOutcomeKind::Ended and EngineError::ParleyNotSupported not yet declared) so Task 2's RED tests (referencing those two not-yet-existing symbols) genuinely fail to compile against Task 1's own GREEN state, then Task 2's GREEN adds them. Verified independently: Task 1's 33-test subset passes standalone before Task 2's tests were reintroduced."
  - "Goto/Parley validation failures are checked together with node_failure, before the merge, in the same per-node accumulation loop -- consistent with D-08a's 'validated the moment the Directive is received, before any routing state changes': since goto_targets/notfiring_nodes are local Vec/HashSet not yet applied to Frontier at that point, an early return on failure discards them with no Frontier mutation to undo."
  - "NodeOutcomeKind::Ended (not a boolean field or NodeExecutionRecord.attempt reuse) is the D-09 observability mechanism -- the plan left the exact shape to discretion; a dedicated non-exhaustive enum variant is the smallest addition that is unambiguous when read back from a persisted Waypoint without re-running the graph."
  - "execute_vanguard_node's NodeSpec::Paladin arm wraps its constructed StateDelta via .into() (NextStep::Edges) rather than adding a DirectiveParser stub -- D-11's DirectiveParser is explicitly Plan 23-04's scope, not this plan's; a Paladin node's routing stays PlainOutput-equivalent until that plan lands."

patterns-established:
  - "A crate-wide trait return-type change ripples to every implementor across crate boundaries (tests/, examples/, benches/), not just the files a plan declares -- Rule 3 blocking-issue fixups are unavoidable and mechanical (Ok(delta) -> Ok(delta.into())) once impl From<StateDelta> for Directive exists."

requirements-completed: [CF-02]

# Coverage metadata
coverage:
  - id: D1
    description: "Directive { delta, next: NextStep } lands in paladin-core with NextStep::{Edges,Goto,Muster,End,Parley}, MusterTask, and impl From<StateDelta> for Directive defaulting to Edges, doc-tested"
    requirement: "CF-02"
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/directive.rs#tests::state_delta_converts_to_a_directive_defaulting_to_edges"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ai-core --doc directive (Directive::from doc test)"
        status: pass
    human_judgment: false
  - id: D2
    description: "StateNode::run returns Result<Directive, NodeError>; every in-tree and workspace-wide implementor adopts .into()"
    requirement: "CF-02"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-battalion --lib engine::superstep (39/39)"
        status: pass
      - kind: other
        ref: "cargo check --workspace --tests --examples --benches --all-features"
        status: pass
    human_judgment: false
  - id: D3
    description: "A Function node's Goto([target]) enters the next Vanguard directly bypassing Frontier::is_ready, while the emitting node's static outgoing edges resolve NotFiring for that superstep"
    requirement: "CF-02"
    verification:
      - kind: unit
        ref: "engine::superstep::tests::function_node_goto_sends_control_to_the_named_node_next_superstep"
        status: pass
      - kind: unit
        ref: "engine::superstep::tests::goto_target_that_is_also_tier_one_ready_is_scheduled_exactly_once"
        status: pass
    human_judgment: false
  - id: D4
    description: "An undeclared Goto target fails typed (GotoUnknownNode); a Goto-only target must be marked WarGraph::mark_dynamic_target via the existing eligible-set mechanism"
    requirement: "CF-02"
    verification:
      - kind: unit
        ref: "engine::superstep::tests::goto_to_an_undeclared_node_fails_the_run"
        status: pass
      - kind: unit
        ref: "engine::superstep::tests::goto_only_target_must_be_declared_dynamic"
        status: pass
    human_judgment: false
  - id: D5
    description: "A Goto refine loop (writer -> reviewer -> Goto(writer) until satisfied) terminates; an unbounded Goto loop trips NodeVisitLimitExceeded at exactly max_node_visits"
    requirement: "CF-02"
    verification:
      - kind: unit
        ref: "engine::superstep::tests::goto_refine_loop_terminates_on_the_reviewer_verdict"
        status: pass
      - kind: unit
        ref: "engine::superstep::tests::unbounded_goto_loop_trips_the_node_visit_limit"
        status: pass
    human_judgment: false
  - id: D6
    description: "NextStep::End completes the run after the emitting superstep's merge (peers still merge), beats a peer's Goto in the same superstep, and which node ended the run is observable from the Waypoint"
    requirement: "CF-02"
    verification:
      - kind: unit
        ref: "engine::superstep::tests::end_completes_the_run_after_the_emitting_superstep_merges"
        status: pass
      - kind: unit
        ref: "engine::superstep::tests::end_beats_goto_in_the_same_superstep"
        status: pass
      - kind: unit
        ref: "engine::superstep::tests::which_node_ended_the_run_is_observable_from_the_waypoint"
        status: pass
    human_judgment: false
  - id: D7
    description: "StarvedNodeAtCompletion's suppression is scoped to end_requested specifically -- an End-terminated run with an unrelated unconsumed fired edge completes, but the check still fires loudly when no node ended the run"
    requirement: "CF-02"
    verification:
      - kind: unit
        ref: "engine::superstep::tests::end_terminated_run_does_not_trip_the_starvation_completion_check"
        status: pass
      - kind: unit
        ref: "engine::superstep::tests::starvation_completion_check_still_fires_when_no_node_ended_the_run"
        status: pass
    human_judgment: false
  - id: D8
    description: "A returned NextStep::Parley fails the run with EngineError::ParleyNotSupported, never coerced to Edges, with no AwaitingInput Waypoint written"
    requirement: "CF-02"
    verification:
      - kind: unit
        ref: "engine::superstep::tests::parley_returned_this_phase_fails_the_run"
        status: pass
    human_judgment: false
  - id: D9
    description: "E2E-1 (crash-resume golden) and the Phase 22 tracer test are unaffected by the StateNode/Directive migration"
    requirement: "CF-02"
    verification:
      - kind: integration
        ref: "cargo test --test e2e_crash_resume_test (27/27, including e2e_1_crash_resume_matches_control_run_with_no_reexecution)"
        status: pass
      - kind: integration
        ref: "cargo test --test war_engine_tracer_test (3/3)"
        status: pass
    human_judgment: false

# Metrics
duration: ~110min
completed: 2026-09-03
status: complete
---

# Phase 23 Plan 02: Directive Type and Node-Driven Goto/End/Parley Routing Summary

**`Directive`/`NextStep` land in `paladin-core`, `StateNode::run` now returns one, a validated Goto refine loop runs and terminates bounded by `max_node_visits`, `End` completes a run after its superstep's merge and beats a peer's `Goto`, and a returned `Parley` fails typed — all test-first, RED committed strictly before GREEN per task.**

## Performance

- **Duration:** ~110 min (includes the RED/GREEN git-surgery reconstruction to split one holistic implementation pass into two independently-verified, per-task TDD commit pairs)
- **Completed:** 2026-09-03
- **Tasks:** 2
- **Files modified:** 13 (1 new, 12 modified)

## Accomplishments

- New `crates/paladin-core/src/platform/container/directive.rs`: `Directive { delta, next }`, `NextStep::{Edges, Goto, Muster, End, Parley}`, `MusterTask { worker, payload, task_key }`, `impl From<StateDelta> for Directive` (defaults `NextStep::Edges`) with a passing doc test. No new `paladin-core` dependency (ADR-0015 — `git diff` on `Cargo.toml` empty).
- `engine/node.rs`: `StateNode::run` returns `Result<Directive, NodeError>`. Every implementor across the workspace adopts `.into()` — not just the plan's declared files, but every `StateNode` fixture the trait-signature change forced to update: `graph.rs`'s `NoopNode`, `test_support.rs`'s `CountingFunctionNode`/`ConcurrencyTrackingNode`/`FailingFunctionNode`/`YieldingNode`, `engine/mod.rs`'s `FixedDeltaNode` test double, `tests/integration/e2e_crash_resume_test.rs`'s `LoopGateNode`, `war_engine_tracer_test.rs`'s `CountingNode`, `waypoint_retention_fault_injection_test.rs`'s `SetFieldNode`, `examples/war_engine_memory_baseline.rs`'s `TrackingNode`, and `benches/engine_benchmarks.rs`'s `FixedValueNode`.
- `test_support.rs`: `CountingFunctionNode` gains `with_directive(directive_fn)`, generalizing the existing `new`/`fixed` (`Edges`-only) constructors so a test can drive `Goto`/`End`/`Muster`/`Parley`, or vary a node's `NextStep` by run index (the refine-loop reviewer fixture).
- `engine/superstep.rs`: `execute_vanguard_node` and `NodeRunOutcome::Succeeded` widen to carry `Directive`; `NodeInterceptor::after` still runs against `&mut directive.delta` only (`hooks.rs` untouched — `git diff` empty). The per-node accumulation loop collects `goto_targets`/`notfiring_nodes`/`end_requested`/`routing_failure` as per-superstep locals (never persisted): a `Goto` target is validated the instant its `Directive` is received (`EngineError::GotoUnknownNode { from, to }` on an undeclared target); any non-`Edges` `NextStep` marks the emitting node's static outgoing edges `NotFiring` directly via `Frontier::record_execution`'s new `force_notfiring` parameter, skipping `evaluate_edge_condition` (D-08c). Validated `goto_targets` are unioned into `next_vanguard` after `compute_next_vanguard` returns — bypassing `Frontier::is_ready` (D-08b) but still bounded by the existing `max_node_visits` enforcement — de-duplicated against nodes already tier-1-ready. `end_requested` short-circuits straight to `RunOutcome::Completed` (peers' deltas already merged) ahead of the `StarvedNodeAtCompletion` check, so `End` beats `Goto` and the check's suppression is scoped to exactly that fact rather than to `next_vanguard.is_empty()`. A returned `Parley` fails the run with `EngineError::ParleyNotSupported { node }` through the same pre-merge failure path.
- `engine/mod.rs`: `EngineError::GotoUnknownNode { from, to }` and `EngineError::ParleyNotSupported { node }`.
- `waypoint.rs`: `NodeOutcomeKind::Ended` — the node whose `Directive.next` was `End` is recorded distinctly from `Succeeded`, so which node ended a run is readable from a persisted `Waypoint`'s `completed` records without re-running the graph (D-09).
- 12 new tests across the two tasks, all passing: 6 Goto tests (basic routing + NotFiring, undeclared-target rejection, dynamic-target requirement, refine-loop termination, unbounded-loop visit-limit trip, tier-1-and-Goto-both-scheduled-once) and 6 End/Parley tests (completion after merge, End-over-Goto precedence, End suppresses the starvation check, the check still fires without End, the ending node is Waypoint-observable, Parley fails typed).

## Task Commits

Each task followed RED-then-GREEN, mirroring 23-01's precedent:

1. **Task 1: Directive type and an end-to-end Goto refine loop** (`type="tracer" tdd="true"`)
   - `5dd33359` — `test(23-02): reproduce Directive-driven Goto routing on not-yet-existing API (red)` — six tests added to `engine::superstep::tests` referencing `Directive`/`NextStep`/`CountingFunctionNode::with_directive`/`EngineError::GotoUnknownNode`; crate fails to compile (20 errors).
   - `e1334ce9` — `feat(23-02): land Directive/NextStep and node-driven Goto routing (green)` — `directive.rs`, the `StateNode` trait change and every implementor's `.into()` migration, the Goto mechanism in `superstep.rs`, `EngineError::GotoUnknownNode`. 33/33 `engine::superstep` tests pass; `e2e_crash_resume_test`/`war_engine_tracer_test` green.
2. **Task 2: End semantics, End-over-Goto precedence, and the typed Parley rejection** (`type="auto" tdd="true"`)
   - `708f07c8` — `test(23-02): reproduce End completion, End-over-Goto and Parley rejection on not-yet-existing API (red)` — six tests referencing `NodeOutcomeKind::Ended`/`EngineError::ParleyNotSupported`, neither declared yet; crate fails to compile (2 errors) against Task 1's own GREEN state.
   - `48b0e4f4` — `feat(23-02): wire End's run-completion semantics and typed Parley rejection (green)` — `NodeOutcomeKind::Ended`, `EngineError::ParleyNotSupported`, `end_requested` tracking and the early-Completed return, the scoped `StarvedNodeAtCompletion` suppression, Parley's routing-failure path. 39/39 `engine::superstep` tests pass (373/373 `paladin-battalion` lib tests overall, up from 361 before this plan).

**Plan metadata:** (this commit) `docs(23-02): complete plan 02`

_Note: Both tasks carry `tdd="true"`; each RED commit is confined to new `#[cfg(test)]` test functions referencing not-yet-existing API, and each GREEN commit lands the mechanism (plus, for Task 1, the crate-wide `StateNode` signature migration the new trait return type forces) — no REFACTOR commit was needed for either task._

## Files Created/Modified

- `crates/paladin-core/src/platform/container/directive.rs` — new module: `Directive`, `NextStep`, `MusterTask`, `impl From<StateDelta> for Directive`, plus a doc test and a unit test.
- `crates/paladin-core/src/platform/container/mod.rs` — registers `pub mod directive;` alphabetically.
- `crates/paladin-core/src/platform/container/waypoint.rs` — `NodeOutcomeKind::Ended` variant (Task 2).
- `crates/paladin-battalion/src/engine/node.rs` — `StateNode::run` returns `Result<Directive, NodeError>`.
- `crates/paladin-battalion/src/engine/superstep.rs` — the Goto/End/Parley mechanism (Task 1 + Task 2), plus 12 new tests.
- `crates/paladin-battalion/src/engine/graph.rs` — `NoopNode` test fixture's `.into()` migration.
- `crates/paladin-battalion/src/engine/mod.rs` — `EngineError::GotoUnknownNode` (Task 1), `EngineError::ParleyNotSupported` (Task 2), `FixedDeltaNode` test fixture's `.into()` migration.
- `crates/paladin-battalion/src/engine/test_support.rs` — `CountingFunctionNode::with_directive`, and every `StateNode` test double's `.into()` migration.
- `tests/integration/e2e_crash_resume_test.rs`, `tests/integration/war_engine_tracer_test.rs` — `StateNode` fixture `.into()` migrations (plan-declared files).
- `tests/integration/waypoint_retention_fault_injection_test.rs`, `examples/war_engine_memory_baseline.rs`, `benches/engine_benchmarks.rs` — `StateNode` fixture `.into()` migrations (Rule 3 auto-fix, outside the plan's declared files — see Deviations).

## Decisions Made

- **Task boundary reconstructed via git surgery, verified independently at each stage.** The full mechanism was implemented in one coherent pass, then deliberately split: superstep.rs, engine/mod.rs and waypoint.rs were reverted to a genuine "Task 1 only" intermediate state (End/Muster/Parley matched but only D-08c's NotFiring marking applied, no `NodeOutcomeKind::Ended`/`EngineError::ParleyNotSupported` declared), confirmed to compile and pass its 33-test subset standalone, RED+GREEN committed, then Task 2's content was reapplied and independently RED+GREEN committed. This makes each task's own acceptance criteria genuinely checkable against its own commit pair rather than only against the plan's final state.
- **Goto/Parley validation failures share one pre-merge failure path with `node_failure`.** Both are checked in the same per-node accumulation loop, before `deltas.sort_by`/merge — satisfying D-08a's "validated before any routing state changes" without needing a separate rollback mechanism, since the local `goto_targets`/`notfiring_nodes` bookkeeping is simply discarded on early return.
- **`NodeOutcomeKind::Ended`** (a dedicated non-exhaustive enum variant, not a boolean field) is the D-09 observability mechanism — CONTEXT.md left the exact shape to discretion.
- **`NodeSpec::Paladin`'s dispatch stays `NextStep::Edges`-only this plan** (`delta.into()`, no `DirectiveParser`) — D-11's `DirectiveParser` is explicitly Plan 23-04's scope.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `.into()` migration for `StateNode` implementors outside the plan's declared files**
- **Found during:** Task 1, after the `StateNode::run` signature change
- **Issue:** `paladin-ai`'s crate-wide `cargo check --workspace --tests --examples --benches --all-features` failed to compile: three files not in this plan's `files_modified` list also implement `StateNode` and needed the same `Ok(delta)` → `Ok(delta.into())` migration — `tests/integration/waypoint_retention_fault_injection_test.rs`'s `SetFieldNode`, `examples/war_engine_memory_baseline.rs`'s `TrackingNode`, and `benches/engine_benchmarks.rs`'s `FixedValueNode`. `engine/mod.rs`'s own `#[cfg(test)]` `FixedDeltaNode` fixture (a file already in scope) needed the same one-line change.
- **Fix:** Added the `Directive` import and changed each `async fn run` return type + final `Ok(delta)` to `Ok(delta.into())`, identical in shape to every other in-scope fixture.
- **Files modified:** `tests/integration/waypoint_retention_fault_injection_test.rs`, `examples/war_engine_memory_baseline.rs`, `benches/engine_benchmarks.rs`, `crates/paladin-battalion/src/engine/mod.rs`.
- **Verification:** `cargo check --workspace --tests --examples --benches --all-features` clean; `cargo test --workspace --lib --bins` 13/13 crate test binaries green, 0 failures.
- **Committed in:** `e1334ce9` (Task 1 GREEN commit).

---

**Total deviations:** 1 auto-fixed (Rule 3, blocking — a crate-wide public trait signature change unavoidably ripples to every implementor in the workspace, not just the files a plan can enumerate in advance).
**Impact on plan:** Necessary for the workspace to compile at all; no scope creep beyond the mechanical `.into()` pattern already established by every in-scope fixture.

## Issues Encountered

None beyond the deliberate RED/GREEN reconstruction described in Decisions Made above, which was itself the planned TDD discipline rather than a problem.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- `Directive`/`NextStep` and the Goto/End/Parley mechanism this plan lands are the exact seam CF-03 (Muster dispatch, Plan 23-06/23-08 per the phase's suggested decomposition), CF-04 (subgraph composition) and CF-05 (LLM-evaluated routing) build on: `NextStep::Muster` is already declared and gets D-08c's NotFiring marking, but no worker-dispatch mechanism exists yet — that is explicitly the next plan's job, not a gap in this one.
- `DirectiveParser`/`StructuredDirective` (D-11) — Plan 23-04's scope — plugs into `execute_vanguard_node`'s `NodeDispatch::Paladin` arm exactly where this plan wraps the constructed `StateDelta` via `.into()`.
- No blockers for downstream plans in this phase's wave sequence. `MIGRATION.md` §9.1/§9.2 needs no new row for anything this plan touched (`StateNode`, `NodeSpec`, `NodeContext`, `EngineError`, `Waypoint` are all pre-release per D-07); no edit was made to it.

---
*Phase: 23-control-flow-dynamic-routing-fan-out-subgraphs*
*Completed: 2026-09-03*

## Self-Check: PASSED

All 13 files listed under Files Created/Modified verified present on disk (`[ -f ... ]` per file). All 4 commits (`5dd33359`, `e1334ce9`, `708f07c8`, `48b0e4f4`) verified present in `git log --oneline`. `cargo test -p paladin-battalion --lib`: 373/373 passed. `cargo test --test e2e_crash_resume_test`: 27/27 passed. `cargo test --test war_engine_tracer_test`: 3/3 passed. `cargo test -p paladin-ai-core --doc directive`: 1/1 passed. `cargo test --workspace --lib --bins`: 13/13 crate test binaries green, 0 failures. `cargo fmt --check`: clean. `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean. `git diff HEAD -- crates/paladin-battalion/src/engine/hooks.rs` and `crates/paladin-core/Cargo.toml`: both empty.
