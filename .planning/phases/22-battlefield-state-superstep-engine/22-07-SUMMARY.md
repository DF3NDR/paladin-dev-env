---
phase: 22-battlefield-state-superstep-engine
plan: 07
subsystem: infra
tags: [rust, superstep-engine, join-semantics, dispatch-registry, determinism, tokio]

# Dependency graph
requires:
  - phase: 22-05
    provides: "The general superstep loop (engine::superstep::run): one Arc<Battlefield> snapshot per superstep, Semaphore-bounded concurrent execution, bounded iteration, the simple 'source ran + condition passed' next-Vanguard heuristic this plan replaces"
  - phase: 22-02
    provides: "Battlefield::merge(deltas, superstep, custom_dispatch) with DispatchConflict/CustomDispatchNotRegistered, and the CustomDispatchResolver lookup type this plan's registry populates"
provides:
  - "A precise, persistent per-edge Frontier (engine::superstep) replacing the 'ran this superstep' heuristic: every incoming edge resolves to Fired/NotFiring/Pending, a node executes once no incoming edge from a run-reachable source is pending and at least one has fired, and a dead-fixpoint propagates a false or unreachable branch so it can never strand a downstream join"
  - "WarGraph::add_deferred_node/is_deferred/node_order: defer-marked nodes hold back an otherwise-executable node until no non-deferred node remains executable, releasing ties in node registration order"
  - "engine::dispatch_registry::DispatchRegistry and WarEngine::with_dispatch_rule (ENG-FR-09): engine-owned Custom(name) -> closure registration, rejecting a name that collides with a built-in DispatchRule variant at registration"
  - "engine::test_support::YieldingNode and shuffle_seeded: seeded scheduling-perturbation doubles for the ENG-FR-08 determinism proof"
  - "20-iteration seeded randomized-scheduling test proving byte-identical serialized Battlefields and identical Vanguard sequences; a 100-iteration #[tokio::test(flavor = \"multi_thread\")] 8-node stress test with exact-count assertions under an explicit timeout"
affects: [22-08, 22-09]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Persistent per-edge resolution state (Fired(superstep)/NotFiring(superstep)/Pending), never recomputed from scratch each superstep -- a join waits on the STATE, not on 'did my source run this exact superstep'"
    - "Dead-node fixpoint: a non-entry node with no incoming edges, or all-incoming-resolved-none-fired, is proven dead and propagated transitively so its own outgoing edges are treated as not-firing without ever running it"
    - "Defer release as a two-pass compute_next_vanguard: non-deferred ready nodes first (edge-insertion order), deferred ready nodes only when that first pass is empty (node-registration order)"
    - "Engine-owned custom dispatch registry (paladin-battalion), never named from paladin-core (X-01) -- paladin-core only ever receives the populated CustomDispatchResolver as a read-only lookup"
    - "Scheduling perturbation via a StateNode-wrapping YieldingNode (seeded yield_now() count) plus a seeded Vec shuffle of spawn order, rather than any timing-based fuzzing"

key-files:
  created:
    - crates/paladin-battalion/src/engine/dispatch_registry.rs
  modified:
    - crates/paladin-battalion/src/engine/graph.rs
    - crates/paladin-battalion/src/engine/superstep.rs
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/src/engine/test_support.rs

key-decisions:
  - "Next-Vanguard readiness keys off a per-node 'freshest fired-edge superstep >= this node's last-executed superstep' comparison (using -1 as the never-executed sentinel) rather than a separate first-execution/re-execution code path -- this single rule handles diamond joins, self-loops and two-node cycles uniformly, verified by one shared is_ready implementation"
  - "Structural deadness (a non-entry node with zero incoming edges) and dynamic deadness (every incoming edge resolves not-firing with none fired) are the SAME fixpoint, run once at Frontier::new() and again after every superstep's edge-state update -- avoids a separate 'orphan node' special case"
  - "is_ready must consult edge_resolution (which treats a Pending edge from a proven-dead source as resolved not-firing), not the raw edge_state map directly -- the first implementation checked raw state and silently reintroduced the exact stranded-join bug this task exists to fix; caught by the false-branch and unreachable-source tests before commit, not left as a discovered issue"
  - "DispatchRegistry lives in a new engine::dispatch_registry module rather than folding the map directly onto WarEngine, so paladin-core's pre-existing CustomDispatchRegistry/CustomDispatchResolver type aliases (Plan 22-02) and this plan's engine-owned DispatchRegistry struct stay clearly distinct despite the name-substring overlap"
  - "Registration-time rejection (EngineError::ReservedDispatchName) for a name colliding with a built-in DispatchRule variant, rather than deferring the check to WarGraph::validate -- catches the mistake at the call site where the schema author is looking, before any graph is even built"
  - "ENG-FR-08's determinism test asserts on the WAYPOINT-persisted Vanguard sequence (not just the final serialized Battlefield) across 20 seeded iterations, per the plan's own rationale that a deterministic final state with a nondeterministic frontier would still break resume"

requirements-completed: [ENG-01, ENG-02]

coverage:
  - id: D1
    description: "A diamond graph (A->B, A->C, B->D, C->D) executes D exactly once, in the superstep after both B and C complete, never once per incoming edge"
    requirement: "ENG-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::diamond_join_executes_target_exactly_once"
        status: pass
    human_judgment: false
  - id: D2
    description: "When the A-to-C edge condition evaluates false, C is proven not-firing and never executes, and D still executes exactly once rather than waiting forever -- guarded by an explicit timeout so a regression deadlocks loudly instead of hanging the suite"
    requirement: "ENG-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::false_branch_is_proven_not_firing_and_join_still_runs_once"
        status: pass
    human_judgment: false
  - id: D3
    description: "A node fed only by a structurally unreachable source (zero incoming edges, not an entry point) never executes, and its own downstream join still resolves once rather than stalling"
    requirement: "ENG-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::node_fed_only_by_an_unreachable_source_never_runs_and_does_not_stall_its_join"
        status: pass
    human_judgment: false
  - id: D4
    description: "A defer-marked node with two sibling entry branches executes exactly once, only after both branches have drained and no non-deferred node remains executable in that superstep"
    requirement: "ENG-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::deferred_node_aggregates_only_after_no_other_node_is_executable"
        status: pass
    human_judgment: false
  - id: D5
    description: "Two deferred nodes released in the same superstep resolve in WarGraph node-registration order, not HashMap or edge-insertion order"
    requirement: "ENG-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::two_deferred_nodes_resolve_in_node_registration_order"
        status: pass
    human_judgment: false
  - id: D6
    description: "The same logical diamond graph, built with nodes registered in two different orders, produces an identical Vanguard sequence across both runs"
    requirement: "ENG-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::insertion_order_does_not_change_the_vanguard_sequence"
        status: pass
    human_judgment: false
  - id: D7
    description: "A two-node cycle (A<->B) terminates on its own edge condition after a bounded number of rounds, not on max_supersteps -- proving cycles are first-class rather than merely tolerated"
    requirement: "ENG-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::two_node_cycle_terminates_on_edge_condition_not_a_limit"
        status: pass
    human_judgment: false
  - id: D8
    description: "A closure registered under a name is applied when a field declares DispatchRule::Custom(name); an unregistered name fails WarGraph::validate with CustomDispatchNotRegistered before any node executes"
    requirement: "ENG-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::engine_with_dispatch_rule_applies_custom_merge_end_to_end, ::engine_start_fails_before_execution_for_unregistered_custom_dispatch"
        status: pass
    human_judgment: false
  - id: D9
    description: "Registering a custom dispatch name that collides with a built-in DispatchRule variant name is rejected at registration"
    requirement: "ENG-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/dispatch_registry.rs#tests::register_rejects_names_colliding_with_built_in_rules"
        status: pass
    human_judgment: false
  - id: D10
    description: "A two-writer superstep on a LastWrite field surfaces DispatchConflict out of the engine, naming the field, the superstep number and both writer NodeIds; a registered custom closure returning an error propagates out of the merge and fails the run rather than being swallowed"
    requirement: "ENG-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::engine_two_writer_last_write_conflict_surfaces_field_superstep_and_writers, ::engine_custom_dispatch_closure_error_fails_the_run_not_swallowed"
        status: pass
    human_judgment: false
  - id: D11
    description: "Two runs with identical node outputs produce byte-identical serialized Battlefields AND an identical Vanguard sequence, over 20 iterations with a seeded shuffle of spawn order and injected yields per iteration"
    requirement: "ENG-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::eng_fr_08_determinism_over_twenty_randomized_scheduling_iterations"
        status: pass
    human_judgment: false
  - id: D12
    description: "An 8-node all-parallel superstep run 100 times under a multi-thread tokio runtime completes within an explicit timeout, with exact-count assertions on node executions (800) and Waypoint saves (100)"
    requirement: "ENG-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::x05_eight_node_parallel_stress_100_iterations_exact_counts"
        status: pass
    human_judgment: false

duration: 13min
completed: 2026-09-02
status: complete
---

# Phase 22 Plan 07: Precise Join/Defer Frontier, Custom Dispatch Registry & Determinism Summary

**A persistent per-edge Frontier resolves every incoming edge to Fired/NotFiring/Pending with a dead-node fixpoint so a false branch can never strand a downstream join, `defer`-marked nodes aggregate after all sibling branches, an engine-owned `DispatchRegistry` registers `Custom` dispatch rules with built-in-name collision rejection, and byte-identical determinism is proven over 20 seeded randomized-scheduling iterations plus a 100-iteration multi-thread stress test.**

## Performance

- **Duration:** ~13 min (first task commit `2026-09-02T00:28:09Z` to last `2026-09-02T00:41:16Z`)
- **Tasks:** 3 completed
- **Files modified:** 5 (1 created, 4 modified)

## Accomplishments

- Replaced the "edge whose source ran this superstep" next-Vanguard heuristic with a persistent `Frontier`: per-edge `Fired(superstep)`/`NotFiring(superstep)`/`Pending` state, a dead-node fixpoint propagating structural and dynamic unreachability, and a unified readiness rule (`no pending incoming edge, at least one fired since last execution`) that handles diamond joins, false branches, orphan sources, self-loops and cycles with one code path.
- Added `defer` support to `WarGraph` (`add_deferred_node`/`is_deferred`/`node_order`): a deferred, otherwise-executable node is held back until no non-deferred node remains executable, releasing ties in node registration order rather than `HashMap` order.
- Added `engine::dispatch_registry::DispatchRegistry` and `WarEngine::with_dispatch_rule`, the engine-owned (never `paladin-core`, X-01) `Custom(name)` rule registration ENG-FR-09 requires, with registration-time rejection of names colliding with a built-in `DispatchRule` variant.
- Proved ENG-FR-08 determinism end to end: a 20-iteration seeded randomized-scheduling test asserting byte-identical serialized Battlefields and identical Vanguard sequences, plus an X-05 house-pattern `#[tokio::test(flavor = "multi_thread")]` 100-iteration 8-node stress test with exact node-execution and Waypoint-save counts under an explicit timeout.

## Task Commits

1. **Task 1: Join, defer and not-firing frontier semantics** - `42987912` (feat)
2. **Task 2: Engine-level custom dispatch registry** - `8c36b621` (feat)
3. **Task 3: Determinism under randomized scheduling and the multi-thread stress test** - `b56d2bdc` (test)

**Plan metadata:** committed as part of this SUMMARY's own commit (worktree mode; STATE.md/ROADMAP.md excluded, orchestrator owns those after wave merge)

## Files Created/Modified

- `crates/paladin-battalion/src/engine/dispatch_registry.rs` - New `DispatchRegistry` type: name -> `(current, delta) -> merged` closure map, built-in-name collision rejection, `resolver()` accessor for `Battlefield::merge`/`WarGraph::validate`
- `crates/paladin-battalion/src/engine/graph.rs` - `WarGraph` gains `node_order` (`Vec<NodeId>`, insertion order) and `defer_flags` (`HashSet<NodeId>`); `add_deferred_node`/`is_deferred`/`node_order` accessors
- `crates/paladin-battalion/src/engine/superstep.rs` - New `Frontier` type (`edge_state`, `dead`, `last_executed`, `incoming`) replaces `compute_next_vanguard`'s old ran-based heuristic; join/defer/not-firing/insertion-order/cycle tests (Task 1); ENG-FR-08 determinism and X-05 stress tests (Task 3)
- `crates/paladin-battalion/src/engine/mod.rs` - `WarEngine::with_dispatch_rule`, `EngineError::ReservedDispatchName`, `start()` resolves through `self.dispatch_registry`; end-to-end custom-dispatch, unregistered-name, DispatchConflict and closure-error-propagation tests (Task 2)
- `crates/paladin-battalion/src/engine/test_support.rs` - `YieldingNode` (seeded `yield_now()` wrapper) and `shuffle_seeded` (seeded `Vec` shuffle), the scheduling-perturbation doubles Task 3's determinism test drives

## Decisions Made

See `key-decisions` in frontmatter. The most consequential: `is_ready` must resolve a `Pending` incoming edge through `edge_resolution` (which treats a proven-dead source as resolved not-firing) rather than reading `edge_state` directly — the first implementation pass used raw state, silently reintroducing the exact stranded-join failure Task 1 exists to fix, and was caught by the false-branch and unreachable-source tests before commit (both failed with `d.run_count() == 0` instead of `1`).

## Deviations from Plan

None — plan executed as written across all three tasks.

## Known False Positive in a Named grep Acceptance Check

Task 2's acceptance criterion `grep -rn 'DispatchRegistry' crates/paladin-core/src/ | wc -l` is `0` **as literally run returns `3`**, not `0`. All three matches are the pre-existing `CustomDispatchRegistry` type alias and its doc comment in `crates/paladin-core/src/platform/container/battlefield.rs` (lines 241/245/250), introduced in Plan 22-02 — `DispatchRegistry` is a substring of `CustomDispatchRegistry`, and the grep pattern is not word-boundary-anchored. `paladin-core` contains **zero** references to this plan's actual `engine::dispatch_registry::DispatchRegistry` type; the X-01 boundary (no application-layer construct reachable from `paladin-core`) is respected in substance. Verified by direct read of all three matched lines. Not treated as a defect to fix (renaming a shipped Plan 22-02 type alias is out of this plan's scope and would be a gratuitous breaking change); recorded here so a future grep-based audit does not mistake the substring collision for a real leak.

## Issues Encountered

None beyond the `is_ready`/`edge_resolution` bug caught and fixed during Task 1's own test-writing (see Decisions Made) — resolved before commit, not left as a deferred issue.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- ENG-02 is now fully covered: bounded iteration and snapshot isolation (Plan 22-05), deterministic multi-writer merge (Plan 22-02), and this plan's precise join/defer/not-firing frontier plus proven determinism. `requirements-completed` marks both ENG-01 and ENG-02 complete per this plan's frontmatter.
- `WarEngine::with_dispatch_rule` and the `Frontier`'s readiness rule are stable surfaces later plans (22-08 Paladin-node execution, 22-09 trace events) can build on without further signature changes -- `compute_next_vanguard`'s shape (`fn(&WarGraph, &Frontier) -> Vec<NodeId>`) is now the plan's stated final form for this phase, not another interim heuristic.
- No blockers for 22-08/22-09.

---
*Phase: 22-battlefield-state-superstep-engine*
*Completed: 2026-09-02*
