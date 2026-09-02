---
phase: 22-battlefield-state-superstep-engine
plan: 08
subsystem: infra
tags: [rust, superstep-engine, paladin-integration, resume, crash-recovery, sqlite, tdd]

# Dependency graph
requires:
  - phase: 22-06
    provides: "SqliteWaypointStore, PostgresWaypointStore and InMemoryWaypointStore, all passing the identical shared WaypointPort contract suite"
  - phase: 22-07
    provides: "The precise per-edge Frontier join/defer semantics (Frontier::is_ready), the engine-owned DispatchRegistry, and proven ENG-FR-08 determinism -- the exact readiness rule Task 3's E2E-1 fixture had to be built around"
provides:
  - "NodeSpec::Paladin execution: engine::input_mapping::InputMapping renders a node's string input from the Battlefield (X-03 bridge), PaladinPort::execute runs it, and the result is written into output_field as a delta under that field's own dispatch rule"
  - "Complete NodeExecutionRecord population for every executed node: real paladin_id and port-reported token_count (Function nodes keep None/0)"
  - "WarEngine::resume fully implemented: restores Battlefield/Vanguard/per-node visit counts from the latest Waypoint and re-enters the SAME superstep loop start() uses, rather than only handling the already-Completed case"
  - "WarEngine::resume_with_options + ResumeOptions{allow_graph_change} and EngineError::VanguardNodeMissing -- the ENG-FR-14 explicit-override path"
  - "engine::test_support::RecordingPaladinPort: configurable per-Paladin output/token-count, exact ordered call log"
  - "Program acceptance scenario E2E-1 as a green integration test (tests/integration/e2e_crash_resume_test.rs, the e2e_crash_resume cargo test target) over a real SqliteWaypointStore file"
affects: [22-09, 22-10, 22-11]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "InputMapping lives in its own engine::input_mapping module (not graph.rs, where Plan 22-07 had landed a placeholder infallible version) with a Result<String, InputMappingError> render() -- undeclared field and no-value-no-default are typed errors, never an empty substitution"
    - "Per-node execution dispatch inside the superstep loop is an internal NodeDispatch enum (Function | Paladin{paladin, input_template, output_field}) resolved once per vanguard node per superstep, so a Paladin node's render/execute/error path reaches the exact same NodeExecutionRecord and node-failure plumbing a Function node's does -- no special-cased Paladin branch downstream of dispatch"
    - "resume_with_options is the single resume implementation; the 2-arg resume() delegates to it with ResumeOptions::default() (allow_graph_change: false) -- one code path, not two divergent ones"
    - "A self-looping node must be a graph ENTRY point, never fed by a separate upstream edge -- Plan 22-07's join-readiness rule (Frontier::is_ready) requires every incoming edge to resolve before a node is placed in the next Vanguard, and a self-loop edge cannot resolve before its own node's first execution. Discovered building the E2E-1 fixture; documented as a load-bearing constraint in the test's build_graph() doc comment for future graph authors"
    - "A crash is simulated by re-seeding a fresh SqliteWaypointStore file with a control run's own real early Waypoints, then reconnecting a second store instance to that file and resuming -- not by racing/aborting a live WarEngine::start task, since superstep::run spawns each vanguard node via a detached tokio::spawn a dropped start future would not transitively cancel"

key-files:
  created:
    - crates/paladin-battalion/src/engine/input_mapping.rs
    - tests/integration/e2e_crash_resume_test.rs
  modified:
    - crates/paladin-battalion/src/engine/graph.rs
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/src/engine/superstep.rs
    - crates/paladin-battalion/src/engine/test_support.rs
    - Cargo.toml

key-decisions:
  - "InputMapping's render() signature is Result<String, InputMappingError> (a dedicated error type converted into EngineError via #[from]), not Result<String, EngineError> as the plan's prose literally suggested -- matches the codebase's established per-module error type convention (BattlefieldError, NodeError, WaypointError all follow this shape) and keeps input_mapping.rs's own unit tests independent of the engine's top-level error enum"
  - "Plan 22-07 had already landed an infallible InputMapping::render (returning empty string on any failure) directly in graph.rs as a placeholder for this plan to wire in. Rather than layering a second type, the placeholder was replaced in place: graph.rs now imports InputMapping from the new module, and its own now-stale infallible-render unit test was removed in favor of input_mapping.rs's own suite"
  - "A self-looping node must be a graph entry point (see tech-stack patterns) -- this is a hard constraint of Plan 22-07's join-readiness rule, not a workaround. The E2E-1 fixture's loop_gate is therefore the graph's ENTRY, with the 5 straight-line Paladin nodes running only after the loop completes; LOOP_BOUND is set to 5 (not 3) so that dropping 'after superstep 3' (E2E-1's own scenario text) still lands mid-loop rather than after it"
  - "E2E-1's simulated crash re-seeds a fresh SqliteWaypointStore file with the control run's own real, durably-persisted first-3 Waypoints rather than racing a live task via tokio::select! -- the superstep loop's per-node tokio::spawn calls are detached tasks a dropped outer future cannot transitively cancel, so a live abort risked a stray background write racing the test's own resume call. Since the graph and mock port are both deterministic, the two techniques are provably equivalent"
  - "Task 1 and Task 2's mod.rs edits were interleaved during development (both touch EngineError and resume); to preserve atomic per-task commits, mod.rs was mechanically reverted to its Task-1-only shape (verified against the same 57-test baseline), committed, then Task 2's additions were restored and re-verified (63 tests) before its own commit -- no functional difference from writing them in strict task order, just deliberate commit hygiene"

requirements-completed: [ENG-03, ENG-04]

coverage:
  - id: D1
    description: "A NodeSpec::Paladin node renders its input through InputMapping, calls PaladinPort::execute, and writes PaladinResult.output into output_field as a delta under that field's own dispatch rule (Append accumulates rather than replaces)"
    requirement: "ENG-03"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::paladin_node_writes_output_into_declared_field"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::paladin_node_append_output_field_accumulates"
        status: pass
    human_judgment: false
  - id: D2
    description: "InputMapping resolution rules: raw string vs JSON-stringified, schema-default fallback, and undeclared-field / no-value-no-default as typed errors rather than empty substitution"
    requirement: "ENG-03"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/input_mapping.rs#tests (9 functions covering every behavior line)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Every executed node's NodeExecutionRecord carries node_id, paladin_id, started_at, measured duration_ms, the port-reported token_count, outcome and attempt:1; a PaladinPort::execute error fails the node and the run exactly like a Function node's own error"
    requirement: "ENG-03"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::paladin_node_execution_record_carries_reported_token_count"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::paladin_port_execute_error_fails_the_node_and_the_run"
        status: pass
    human_judgment: false
  - id: D4
    description: "resume loads the latest Waypoint and restores the Battlefield, Vanguard and per-node visit counts, then continues from the next superstep through the same loop start() uses"
    requirement: "ENG-04"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::resume_restores_visit_counts_and_trips_limit_on_next_post_resume_visit"
        status: pass
      - kind: integration
        ref: "tests/integration/war_engine_tracer_test.rs#start_checkpoints_once_and_resume_never_reexecutes"
        status: pass
    human_judgment: false
  - id: D5
    description: "Nodes already completed before an interruption are not re-executed after resume, observed through a call-recording PaladinPort, at every interruption index of a 5-superstep run"
    requirement: "ENG-04"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::resume_parameterized_at_every_superstep_index_matches_control_and_skips_completed_nodes"
        status: pass
      - kind: e2e
        ref: "tests/integration/e2e_crash_resume_test.rs#e2e_1_crash_resume_matches_control_run_with_no_reexecution (clause a)"
        status: pass
    human_judgment: false
  - id: D6
    description: "Resuming a thread whose latest Waypoint is Completed returns RunOutcome::Completed immediately with no execution and no new Waypoint; resuming an unknown thread returns ThreadNotFound"
    requirement: "ENG-04"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::resume_completed_short_circuit_writes_no_new_waypoint"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::resume_on_unknown_thread_errors"
        status: pass
    human_judgment: false
  - id: D7
    description: "resume against a differing graph fingerprint fails with GraphMismatch unless allow_graph_change is passed, in which case a stored vanguard NodeId absent from the new graph fails precisely with VanguardNodeMissing rather than being silently dropped"
    requirement: "ENG-04"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::resume_with_graph_mismatch_fails_without_allow_graph_change"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::resume_allow_graph_change_missing_vanguard_node_fails_precisely"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::resume_allow_graph_change_proceeds_when_vanguard_node_present"
        status: pass
    human_judgment: false
  - id: D8
    description: "Program scenario E2E-1: a 6-node cyclic workflow with a bounded loop, run against a mock LLM and a durable SQLite backend, dropped after superstep 3 and resumed by a fresh engine, re-executes no completed node, reaches a final Battlefield equal to an uninterrupted control run, and leaves exactly one Waypoint per completed superstep, with the loop node running the same number of times in both runs"
    requirement: "ENG-04"
    verification:
      - kind: e2e
        ref: "tests/integration/e2e_crash_resume_test.rs#e2e_1_crash_resume_matches_control_run_with_no_reexecution"
        status: pass
    human_judgment: false
  - id: D9
    description: "Interrupting the same 5-superstep run at every superstep index in turn and resuming produces the same final Battlefield in every case"
    requirement: "ENG-04"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::resume_parameterized_at_every_superstep_index_matches_control_and_skips_completed_nodes (k in 1..=5)"
        status: pass
    human_judgment: false

# Metrics
duration: ~50min
completed: 2026-09-02
status: complete
---

# Phase 22 Plan 08: Paladin Execution, Full Resume & E2E-1 Summary

**Real `NodeSpec::Paladin` execution through a fallible `InputMapping` X-03 bridge, a fully restored `WarEngine::resume` sharing `start`'s own superstep loop, and program acceptance scenario E2E-1 green over a durable SQLite backend.**

## Performance

- **Duration:** ~50 min
- **Tasks:** 3 completed
- **Files modified:** 7 (2 created, 5 modified)

## Accomplishments

- `NodeSpec::Paladin` nodes now execute for real: `engine::input_mapping::InputMapping::render` resolves `{field}` placeholders against the superstep snapshot (raw for JSON strings, JSON-stringified otherwise, schema-default fallback, typed errors for an undeclared field or a no-value-no-default field — never an empty substitution), `PaladinPort::execute` runs the rendered string, and the result is written into `output_field` as a delta under that field's own dispatch rule (`Append` accumulates, no special-casing).
- Every executed node's `NodeExecutionRecord` now carries its real `paladin_id` and the port-reported `token_count`; a `PaladinPort::execute` error fails the node and the run through the identical path a `Function` node's own error already used.
- `WarEngine::resume` is fully implemented: it loads the latest Waypoint, compares the graph fingerprint (failing `GraphMismatch` unless `allow_graph_change` is set, in which case a missing restored vanguard node fails precisely with the new `EngineError::VanguardNodeMissing`), short-circuits to `Completed` with no new Waypoint when already done, and otherwise restores the Battlefield, Vanguard and per-node visit counts and re-enters the SAME superstep loop `start()` uses — proven at every interruption index of a 5-superstep run, and proven to correctly trip a visit limit on the very next post-resume visit rather than resetting it.
- Program acceptance scenario E2E-1 is a green integration test over a real `SqliteWaypointStore` file: a 6-node graph (5 Paladin nodes via a call-recording port, one bounded self-loop) proves, as three separate assertions plus a fourth, that no already-completed node re-executes, the resumed final Battlefield equals an uninterrupted control run's, exactly one Waypoint exists per completed superstep with an unbroken parent chain, and the loop node ran the same number of times in both runs.
- Discovered and documented a real engine constraint while building the E2E-1 fixture: a self-looping node must be a graph entry point, because Plan 22-07's join-readiness rule requires every incoming edge to resolve before a node is placed in the next Vanguard, and a self-loop edge cannot resolve before its own node's first execution — a node that is both self-looping and fed by a separate upstream edge could never run at all.
- `cargo test --workspace --lib` (1841 tests across 12 crates), `cargo test --test war_engine_tracer`, `cargo test --test e2e_crash_resume`, `cargo test --doc -p paladin-battalion`, `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` are all green.

## Task Commits

Each task was committed atomically:

1. **Task 1: Paladin nodes, InputMapping template resolution and token accounting** - `9fc7e8ca` (feat)
2. **Task 2: Complete resume — restore state, vanguard and visit counts with fingerprint enforcement** - `4b72e3b0` (feat)
3. **Task 3: Program acceptance scenario E2E-1 as an integration test** - `9627de4a` (test)

**Plan metadata:** committed alongside this SUMMARY (worktree mode; STATE.md/ROADMAP.md excluded, orchestrator owns those after wave merge)

## Files Created/Modified

- `crates/paladin-battalion/src/engine/input_mapping.rs` - New: `InputMapping`/`InputMappingError`, the X-03 bridge with fallible `render`
- `crates/paladin-battalion/src/engine/graph.rs` - Removed the Plan 22-07 placeholder infallible `InputMapping`; now imports the real type from `input_mapping`
- `crates/paladin-battalion/src/engine/superstep.rs` - `NodeDispatch` enum and `execute_vanguard_node` resolve `NodeSpec::Paladin` execution inline in the per-vanguard-node dispatch loop; `run()` gained a trailing `paladin_port: &Arc<dyn PaladinPort>` parameter
- `crates/paladin-battalion/src/engine/mod.rs` - `EngineError::InputMapping`/`VanguardNodeMissing`, `ResumeOptions`, `WarEngine::resume_with_options` (full ENG-FR-12 implementation), `resume()` now delegates to it; ~20 new unit tests across both tasks
- `crates/paladin-battalion/src/engine/test_support.rs` - New `RecordingPaladinPort` test double
- `Cargo.toml` - New `[[test]] name = "e2e_crash_resume"` target
- `tests/integration/e2e_crash_resume_test.rs` - New: program acceptance scenario E2E-1

## Decisions Made

See `key-decisions` in frontmatter. The most consequential: a self-looping node must be the graph's entry point under Plan 22-07's join-readiness rule — this is a load-bearing engine constraint discovered while building the E2E-1 fixture (an initial `researcher -> writer -> loop_gate(self-loop) -> reviewer...` design deadlocked: `loop_gate`'s self-edge and its `writer->loop_gate` edge could never BOTH resolve before its first execution), not a workaround; documented in the test file's `build_graph()` doc comment so a future graph author doesn't rediscover it the hard way.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Plan 22-07 had already landed a placeholder, infallible `InputMapping::render` directly in `graph.rs`**
- **Found during:** Task 1, before writing any new code
- **Issue:** The plan's task text describes creating `engine/input_mapping.rs` as if `InputMapping` did not yet exist, but Plan 22-07 (a prior wave in this same phase) had already added a minimal `InputMapping` type to `graph.rs` with an infallible `render(&self, state: &Battlefield) -> String` that silently substituted an empty string on any resolution failure — explicitly flagged in that plan's own doc comment as a placeholder for this plan to wire in. Leaving it in place would have meant two competing `InputMapping` types, and the infallible signature directly contradicts this plan's must-have truth ("an undeclared field is a typed error, not an empty substitution").
- **Fix:** Created the new `engine::input_mapping` module with the real, fallible `InputMapping`/`InputMappingError`; updated `graph.rs` to import it instead of defining its own; removed `graph.rs`'s now-stale infallible-behavior unit test (superseded by `input_mapping.rs`'s own 9-test suite); updated `mod.rs`'s re-export accordingly.
- **Files modified:** `crates/paladin-battalion/src/engine/input_mapping.rs` (new), `crates/paladin-battalion/src/engine/graph.rs`, `crates/paladin-battalion/src/engine/mod.rs`
- **Verification:** `cargo test -p paladin-battalion --lib engine` green; the removed test's coverage is exceeded by `input_mapping.rs`'s own suite.
- **Committed in:** `9fc7e8ca` (Task 1 commit)

**2. [Rule 1 - Bug] Self-loop + external-predecessor graph shape deadlocks under Plan 22-07's join-readiness rule**
- **Found during:** Task 3, first run of the E2E-1 fixture
- **Issue:** The initial fixture wired `writer -> loop_gate` (external feed) plus `loop_gate -> loop_gate` (self-loop). `Frontier::is_ready` requires every incoming edge of a node to be resolved (not `Pending`) before that node can enter the next Vanguard. `loop_gate`'s self-edge is `Pending` until `loop_gate` has executed at least once — so `loop_gate` could never execute at all (its self-edge blocks its first run, and its first run is what would resolve the self-edge). The control run silently completed after only 2 supersteps (`researcher`, `writer`) instead of the intended 8, since `loop_gate` was never ready.
- **Fix:** Restructured the fixture so `loop_gate` is the graph's own entry point (self-loop only, no external predecessor), with the 5 straight-line Paladin nodes running only after the loop reaches `"done"`. Raised `LOOP_BOUND` from 3 to 5 so dropping "after superstep 3" (E2E-1's literal scenario text) still lands mid-loop rather than after it. Documented the constraint in `build_graph()`'s doc comment.
- **Files modified:** `tests/integration/e2e_crash_resume_test.rs`
- **Verification:** `cargo test --test e2e_crash_resume` — control run now produces exactly `LOOP_BOUND + 5` (10) Waypoints as expected; the interrupted-and-resumed run matches.
- **Committed in:** `9627de4a` (Task 3 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 1 — a naming/duplication bug and a graph-shape bug discovered while proving the scenario). No scope creep — no new public API beyond the plan's three tasks, no new crate dependencies.

## Known Stubs / Verification Debt

None. All three tasks' `<verify>` commands were run and are green; no deferred assertions.

## Issues Encountered

Both deviations above were caught and resolved within their originating task, before commit. Separately (a process note, not a functional issue): Task 1 and Task 2's edits to `mod.rs` were written in an interleaved development session; to preserve atomic per-task commits, `mod.rs` was mechanically reverted to its Task-1-only shape (re-verified against the same 57-test baseline observed mid-session), committed, then Task 2's additions were restored and re-verified (63 tests) before its own commit. No functional difference from writing the tasks in strict sequence — recorded here only for commit-history transparency.

## User Setup Required

None — no external service configuration required. `tests/integration/e2e_crash_resume_test.rs` uses temporary SQLite files under `std::env::temp_dir()`, cleaned up by the OS; no Docker or network access needed.

## Next Phase Readiness

- ENG-03 and ENG-04 are both fully covered: Paladin-node execution with the X-03 `InputMapping` bridge and token accounting (ENG-03), and a complete, proven `resume` with fingerprint enforcement and an explicit override path (ENG-04), closing this phase's headline crash-resume claim end to end via program scenario E2E-1.
- `WarEngine::resume_with_options`/`ResumeOptions` and `EngineError::VanguardNodeMissing` are stable surfaces later plans can build on without further signature changes.
- The self-loop-must-be-an-entry-point constraint is now documented in a test's doc comment; a future phase building on dynamic control flow (Doc 02, CF-*) that wants a self-loop fed by an external node will need either a structural workaround (a distinct "gate" node preceding the loop, as this plan's own doc comment suggests was tried and rejected) or an engine-level change to `Frontier::is_ready` — flagged here rather than silently discovered again.
- No blockers for 22-09/22-10/22-11.

---
*Phase: 22-battlefield-state-superstep-engine*
*Completed: 2026-09-02*
