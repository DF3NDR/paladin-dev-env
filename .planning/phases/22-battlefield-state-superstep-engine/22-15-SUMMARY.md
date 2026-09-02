---
phase: 22-battlefield-state-superstep-engine
plan: 15
subsystem: engine
tags: [rust, war-engine, graph-validation, reachability, bug-fix, tdd]

# Dependency graph
requires:
  - phase: 22-battlefield-state-superstep-engine
    provides: "WarGraph, WarGraph::validate, the superstep engine (Plans 01-11)"
provides:
  - "WarGraph::validate eligible-set reachability check (ENG-FR-02a)"
  - "EngineError::UnreachableNode { nodes, reason } listing all offenders at once"
  - "WarGraph::mark_dynamic_target / is_dynamic_target escape hatch for runtime jump targets"
affects: [22-16 (fixture audit for strandedness workarounds)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Fixed-point worklist reachability computation seeded from entry + dynamic_target markers"
    - "Marker-as-method (not a second add_*_node constructor) following the existing add_deferred_node/is_deferred precedent"

key-files:
  created: []
  modified:
    - crates/paladin-battalion/src/engine/graph.rs
    - crates/paladin-battalion/src/engine/mod.rs

key-decisions:
  - "Entry-missing (no add_entry call at all, with nodes declared) is treated as a distinct terminal failure naming the absent entry point, even if some nodes are marked dynamic_target -- a live run has to start somewhere, so an empty entry set makes every node's eligibility moot regardless of markers"
  - "The reachable-from-entry regression test intentionally omits the self-loop from the original stranded fixture: a node with BOTH a self-loop and an external incoming edge can never resolve its own self-loop edge's Pending state before it first runs (ENG-FR-06 join semantics), an unrelated pre-existing engine property this plan does not touch -- combining both in one test would give a false negative unrelated to reachability"
  - "resume_allow_graph_change_proceeds_when_vanguard_node_present's extra unconnected node 'c' was repaired by declaring it its own entry point (not by wiring an edge or marking dynamic_target, since neither reflects why the node exists) -- entry status is not part of WarGraph::fingerprint's hash, so the test's fingerprint-mismatch assertion is unaffected, and 'c' is never part of the RESTORED vanguard resume schedules, so it still never executes"

requirements-completed: [ENG-02]

coverage:
  - id: D1
    description: "WarGraph::validate rejects a graph containing a non-entry node whose only incoming edge is its own self-loop, naming that node, before any node executes"
    requirement: "ENG-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#validate_rejects_self_loop_only_stranded_node_naming_it"
        status: pass
    human_judgment: false
  - id: D2
    description: "Multiple stranded nodes are reported together in one error, in registration order"
    requirement: "ENG-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#validate_rejects_multiple_stranded_nodes_in_one_error_registration_order"
        status: pass
    human_judgment: false
  - id: D3
    description: "The eligible set is entry-reachable nodes unioned with dynamic_target-marked nodes, computed to a fixed point; a formerly-stranded node validates and runs once reachable or once marked dynamic_target"
    requirement: "ENG-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#validate_accepts_and_runs_stranded_node_once_made_reachable_from_entry"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#validate_accepts_and_runs_stranded_node_once_marked_dynamic_target"
        status: pass
    human_judgment: false
  - id: D4
    description: "Self-loops remain legal on entry nodes and on nodes otherwise reachable from entry -- the check rejects strandedness, not loops"
    requirement: "ENG-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#self_loop_on_entry_node_still_validates_and_runs"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#validate_accepts_self_loop_on_node_reachable_from_entry_by_normal_edge"
        status: pass
    human_judgment: false
  - id: D5
    description: "A graph declaring nodes but no entry point fails with a reason naming the absent entry point, distinct from the ordinary stranded-node message"
    requirement: "ENG-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#validate_rejects_graph_with_no_entry_point_naming_absent_entry"
        status: pass
    human_judgment: false
  - id: D6
    description: "Pre-existing validation clauses (limits, unknown node, unregistered custom dispatch) still surface their own errors first, ahead of the new eligible-set check"
    requirement: "ENG-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#validate_prefers_limit_error_over_unreachable_node"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#validate_prefers_unknown_node_error_over_unreachable_node"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#validate_prefers_custom_dispatch_error_over_unreachable_node"
        status: pass
    human_judgment: false
  - id: D7
    description: "Fixture cleanup: the one existing fixture the new check caught (resume_allow_graph_change_proceeds_when_vanguard_node_present) is repaired without weakening the check; workspace stays green; the crash-resume fixture's looping-node-as-entry workaround is left for 22-16"
    verification:
      - kind: integration
        ref: "cargo test --workspace (all 38 test binaries, 512 in the largest suite, including tests/integration/e2e_crash_resume_test.rs)"
        status: pass
    human_judgment: false

# Metrics
duration: 40min
completed: 2026-09-02
status: complete
---

# Phase 22 Plan 15: Eligible-Set Reachability Validation (BUG-02 / ENG-FR-02a) Summary

**`WarGraph::validate` now rejects every declared node outside the eligible set (reachable from entry, or marked `dynamic_target`) in one error listing all offenders, closing the silent-stranded-node defect (BUG-02) test-first.**

## Performance

- **Duration:** ~40 min
- **Tasks:** 3
- **Files modified:** 2 (`crates/paladin-battalion/src/engine/graph.rs`, `crates/paladin-battalion/src/engine/mod.rs`)

## Pre-Fix Evidence Capture (Task 1)

Before writing any permanent test, a temporary evidence test was run against the unmodified code to confirm the defect. Fixture: a 2-node graph — `entry` (a real entry node) and `stranded` (whose only incoming edge is its own self-loop, no edge from `entry`). Captured facts, both true on the pre-fix code:

1. `graph.validate(&CustomDispatchResolver::new())` returned `Ok(())`.
2. A full run through `superstep::run` reported `RunOutcome::Completed { final_state: {"result": "entry-ran"}, waypoint: ... }` — and `stranded`'s `run_count()` was `0`. The run reported success over a node that never executed.

This test was deleted immediately after capturing the two facts above (per the plan: evidence-for-the-SUMMARY, not a permanent test) and replaced with the permanent regression suite in the same commit's parent state — i.e. the next commit is the real red-state commit.

## Accomplishments
- `EngineError::UnreachableNode { nodes: Vec<NodeId>, reason: String }` — carries every offending node in one error, in the graph's registration order, with a reason explaining both fixes (make reachable, or mark `dynamic_target`), or naming an absent entry point as a distinct cause.
- `WarGraph::mark_dynamic_target` / `is_dynamic_target` — the declared escape hatch for runtime jump targets, realized as a marker method (like `add_deferred_node`/`is_deferred`) rather than a new `NodeSpec` field or constructor, so a node can be both `defer`red and a dynamic target.
- `WarGraph::validate`'s eligible-set check: a fixed-point worklist seeded from `entry` ∪ `dynamic_targets`, expanded over declared edges (conditions ignored — a static edge is what proves intent here), run **last** among validate's clauses so more specific structural errors (limits, unknown node, unregistered custom dispatch) still surface first.
- 15 new regression tests in `crates/paladin-battalion/src/engine/graph.rs` covering: single/multiple stranded rejection with registration-order naming, the reachable-from-entry and dynamic-target-marked acceptance/run cases, self-loop-remains-legal pinning (entry and non-entry), the distinct no-entry-point failure, an isolated no-edge node, and three ordering tests proving pre-existing clauses take precedence.
- The one existing fixture the new check caught (in `crates/paladin-battalion/src/engine/mod.rs`) repaired without weakening the check.
- rustdoc updated on `WarGraph`'s type-level doc and `validate`'s doc to describe the new clause and its ordering rationale; `mark_dynamic_target`'s rustdoc documents the CF-FR-07 handoff and the deliberate non-inference-from-parsers decision, plus the two documented future eligibility sources (worker templates / Phase 23, Aegis `on_error` Route targets / Phase 25) as insertion points into the same worklist — nothing fabricated for either.

## Task Commits

Each task was committed atomically:

1. **Task 1: Capture the defect, then write the failing regression tests** - `31f1903e` (test) — red state: 4 of the new tests fail (`validate_rejects_*`), 20 pass (acceptance/ordering pins that already held under the permissive pre-fix `validate`).
2. **Task 2: Implement eligible-set validation and the dynamic-target marker** - `b1ac8668` (feat) — green: all 24 tests in `engine::graph` pass; `cargo clippy -p paladin-battalion --all-targets -- -D warnings` clean.
3. **Task 3: Restore the crate and workspace to green, recording every fixture the check caught** - `bc59e1e9` (fix) — one fixture repaired; `cargo test --workspace` green (38/38 binaries); `cargo fmt --check` clean after `cargo fmt`; `cargo clippy --workspace --all-targets -- -D warnings` clean; `make security` exits 0 (advisories/bans/licenses/sources ok; pre-existing unmaintained/yanked-crate warnings unrelated to this change).

**Plan metadata:** (this commit, docs: complete plan)

_Note: This is a `tdd="true"` plan executed as RED (Task 1) → GREEN (Task 2) → fixture cleanup (Task 3), matching the TDD gate sequence._

## TDD Gate Compliance

- RED gate: `31f1903e` — `test(22-15): add failing eligible-set reachability tests for BUG-02 (red)`. Confirmed 4 failing tests via `cargo test -p paladin-battalion --lib engine::graph`.
- GREEN gate: `b1ac8668` — `feat(22-15): implement eligible-set reachability validation (ENG-FR-02a)`. Confirmed all 24 tests pass.
- No separate REFACTOR commit was needed; `validate_eligible_set` was factored out inline during Task 2 rather than as a follow-up cleanup pass.

## Files Created/Modified
- `crates/paladin-battalion/src/engine/graph.rs` - `WarGraph::validate` gained the eligible-set reachability check (`validate_eligible_set`); `WarGraph` gained `dynamic_targets`/`mark_dynamic_target`/`is_dynamic_target`; 15 new tests plus a `run_to_completion` test helper calling the crate-private `superstep::run` directly.
- `crates/paladin-battalion/src/engine/mod.rs` - `EngineError` gained the `UnreachableNode { nodes, reason }` variant; one existing test (`resume_allow_graph_change_proceeds_when_vanguard_node_present`) repaired to declare its extra node as an entry point instead of leaving it stranded.

## Decisions Made
- **Entry-missing is unconditional, regardless of `dynamic_target` markers.** ENG-FR-02a's acceptance criteria distinguish "no entry point" from the ordinary per-node stranded case. Rather than letting an all-`dynamic_target`-marked graph with zero `add_entry` calls slip through (technically eligible via the worklist seed), `validate_eligible_set` treats `entry.is_empty() && !nodes.is_empty()` as an unconditional, distinct failure: a live run has to start somewhere, and a dynamic target only ever fires from within an already-running graph. This is a deliberate simplification of the literal fixed-point semantics in favor of the acceptance criterion's explicit intent.
- **The "made reachable" regression test does not keep the original self-loop.** The rejection fixture's stranded node has only a self-loop edge. Naively "adding an edge from entry" to that same fixture and then running it hits an independent, pre-existing engine property: `Frontier::is_ready` (ENG-FR-06 join semantics) requires ALL of a node's incoming edges to resolve before it becomes ready, and a self-loop's source is the node itself — so a node with both a self-loop and an external incoming edge can never resolve its self-loop's `Pending` state before its first run, deadlocking permanently regardless of this plan's fix. This was discovered empirically (test failed with `run_count() == 0` even after the fix landed) and resolved by using a self-loop-free "isolated node wired to entry" fixture for the run-assertion, while the self-loop-specific legality is pinned separately by `validate`-only tests (matching the pre-existing `validate_accepts_self_loop_on_node_reachable_from_entry_by_normal_edge`-style test, which also never runs the graph). This is a test-construction detail, not a scope change to the plan's requirement.
- **`resume_allow_graph_change_proceeds_when_vanguard_node_present`'s extra node "c" was given an entry declaration, not an edge or a `dynamic_target` mark.** The node exists purely to perturb the graph's fingerprint; `WarGraph::fingerprint` hashes node ids but not entry status, so declaring "c" as its own entry changes nothing the test asserts on, while making it a legitimately eligible node instead of a stranded one that the new check would otherwise (correctly) reject.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `cargo fmt` reformatting required after Task 3's test additions**
- **Found during:** Task 3 (`cargo fmt --check` step)
- **Issue:** Several `graph.add_node(...)` calls in the new tests exceeded rustfmt's line-wrapping threshold as single-line calls.
- **Fix:** Ran `cargo fmt` (no manual edits); the diff is whitespace/wrapping only.
- **Files modified:** `crates/paladin-battalion/src/engine/graph.rs`
- **Verification:** `cargo fmt --check` clean afterward; `cargo test -p paladin-battalion` still green.
- **Committed in:** `bc59e1e9` (Task 3 commit)

---

**Total deviations:** 1 auto-fixed (blocking, formatting only)
**Impact on plan:** No behavioral change; cosmetic formatting fix required by CLAUDE.md's `cargo fmt --check` gate before committing.

## Issues Encountered
- The reachable-from-entry test initially combined the stranded fixture's self-loop with a new external edge into the same node, following the plan's "same graph, add an edge" wording literally. This produced `run_count() == 0` even after Task 2's fix landed, tracing to `Frontier::is_ready`'s join semantics (a self-loop's own `Pending` state can never resolve before the node's first run, independent of reachability validation). Resolved per "Decisions Made" above by using a self-loop-free fixture for the run assertion; the self-loop-remains-legal claim is still fully pinned by other tests, just without a run assertion (consistent with how the pre-existing `validate_accepts_self_loop_on_node_reachable_from_entry_by_normal_edge`-shaped test in this same file already only validates, never runs, when a self-loop and an external edge coexist on one node).

## Fixtures Handed to Plan 22-16 (Fixture Audit, Acceptance 2a)

Per this plan's scope boundary (Task 3's action explicitly reserves "the deliberate audit of every fixture that works around strandedness ... is plan 22-16's job and must not be pre-empted here"), the following is the list of fixtures observed during this plan's work that route around strandedness rather than being caught as red by the new check (because they already make the looping node a graph entry, which is legal):

- `tests/integration/e2e_crash_resume_test.rs` — `build_graph()`'s `loop_gate` node is deliberately made the graph's entry point (see the function's own doc comment, lines ~112-131) specifically to sidestep the self-loop join-deadlock property described above (`Frontier::is_ready` requiring the self-loop's own edge to resolve before first execution). This is the exact fixture UAT gap G-22-3's `root_cause` field named (`tests/integration/e2e_crash_resume_test.rs:112-127`). It is NOT rejected by the new check (loop_gate is a legitimate entry node), so it does not need repair for THIS plan's tests to pass — but per ENG-FR-02a acceptance 2a ("no remaining fixture works around strandedness"), 22-16 should evaluate whether this arrangement is still necessary now that entry-node self-loops are explicitly pinned as legal, or whether it can be restructured now that the join-deadlock property is documented.
- `crates/paladin-battalion/src/engine/superstep.rs`'s `self_loop_graph` helper (used by `self_loop_runs_exactly_three_times_when_approved_on_third_visit`, `self_loop_never_approved_trips_node_visit_limit_at_five`, `self_loop_at_four_visits_does_not_trip`) — also makes its looping node the graph entry, for the same structural reason. Not rejected by the new check; flagged for the same reason as above.

No other fixture in the crate or the integration `tests/` tree constructs a graph with a non-entry, non-dynamic-target, unreachable node — confirmed by `cargo test --workspace` passing with zero failures after this plan's one repair.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- BUG-02 is closed: `RunOutcome::Completed` can no longer be reported for a graph containing a node that could never have become ready.
- Plan 22-16 (fixture audit, acceptance 2a) can proceed directly from the "Fixtures Handed to Plan 22-16" list above rather than re-discovering it.
- `WarGraph::mark_dynamic_target` is available for any future graph construction (Phase 23 Muster fan-out, Phase 25 Aegis routing) that legitimately needs to declare a runtime-jump-only target ahead of those phases landing the mechanisms that populate the other two documented eligibility sources.

## Self-Check: PASSED

- FOUND: `crates/paladin-battalion/src/engine/graph.rs` (modified, present)
- FOUND: `crates/paladin-battalion/src/engine/mod.rs` (modified, present)
- FOUND: `.planning/phases/22-battlefield-state-superstep-engine/22-15-SUMMARY.md` (this file)
- FOUND: commit `31f1903e` (Task 1, red)
- FOUND: commit `b1ac8668` (Task 2, green)
- FOUND: commit `bc59e1e9` (Task 3, workspace green + fmt)

---
*Phase: 22-battlefield-state-superstep-engine*
*Completed: 2026-09-02*
