---
phase: 23-control-flow-dynamic-routing-fan-out-subgraphs
reviewed: 2026-09-04T03:45:16Z
depth: standard
files_reviewed: 30
files_reviewed_list:
  - benches/engine_benchmarks.rs
  - crates/paladin-battalion/Cargo.toml
  - crates/paladin-battalion/src/campaign_service.rs
  - crates/paladin-battalion/src/commander.rs
  - crates/paladin-battalion/src/edge_evaluator.rs
  - crates/paladin-battalion/src/engine/bridges.rs
  - crates/paladin-battalion/src/engine/directive_parser.rs
  - crates/paladin-battalion/src/engine/graph.rs
  - crates/paladin-battalion/src/engine/hooks.rs
  - crates/paladin-battalion/src/engine/input_mapping.rs
  - crates/paladin-battalion/src/engine/mod.rs
  - crates/paladin-battalion/src/engine/node.rs
  - crates/paladin-battalion/src/engine/superstep.rs
  - crates/paladin-battalion/src/engine/test_support.rs
  - crates/paladin-battalion/src/lib.rs
  - crates/paladin-battalion/src/llm_decision.rs
  - crates/paladin-core/src/platform/container/directive.rs
  - crates/paladin-core/src/platform/container/mod.rs
  - crates/paladin-core/src/platform/container/waypoint.rs
  - crates/paladin-storage/src/waypoint/contract_tests.rs
  - crates/paladin-storage/src/waypoint/in_memory.rs
  - crates/paladin-storage/src/waypoint/postgres.rs
  - crates/paladin-storage/src/waypoint/sqlite.rs
  - docs/src/SUMMARY.md
  - docs/src/user-guides/control-flow.md
  - examples/war_engine_memory_baseline.rs
  - tests/integration/e2e_crash_resume_test.rs
  - tests/integration/e2e_muster_defer_order_test.rs
  - tests/integration/subgraph_formation_in_campaign_test.rs
  - tests/integration/war_engine_tracer_test.rs
  - tests/integration/waypoint_retention_fault_injection_test.rs
findings:
  critical: 1
  warning: 2
  info: 0
  total: 3
status: issues_found
---

# Phase 23: Code Review Report

**Reviewed:** 2026-09-04T03:45:16Z
**Depth:** standard
**Files Reviewed:** 30
**Status:** issues_found

## Summary

Phase 23 adds node-authored routing (`Directive`/`NextStep`), a `DirectiveParser` for
Paladin-node output, Muster fan-out with intra-superstep progress checkpoints, `NodeSpec::Battalion`
subgraph composition, `ThreadId::child`, fingerprint `v3`, and `LlmDecisionEvaluator`/
`StrategySelection::Semantic`. The engineering is unusually careful: extensive rustdoc tracing
every design decision back to a numbered `D-NN`/`CF-FR-NN` requirement, thorough test coverage
(the vast majority of `superstep.rs`'s 7700+ lines are tests), a fail-closed `EdgeEvaluatorRegistry`
replacing the prior BUG-01 always-true placeholder on both execution paths, and deliberate,
well-reasoned privacy/redaction discipline around the two new LLM call sites (`llm_decision.rs`,
`commander.rs`'s `select_strategy_semantic`) — neither ever interpolates a rendered prompt or a
provider's raw error/response text into an error message, satisfying the manual credential-handling
review this project requires for `LlmPort` call sites.

Despite that overall quality, direct tracing of the new `superstep::run_with_namespace` loop found
one confirmed panic path: the `EngineError::RecursionLimitExceeded` branch indexes `vanguard[0]`
unconditionally, but this phase's own Muster feature (`has_pending_muster`) introduces the first
case where the loop can re-enter with an **empty** `vanguard` (a muster-only round with no other
ready node). The sibling branch three call sites below it (the `Battlefield::merge` failure
fallback) was correctly updated in this same diff to use `dispatch_entries[0]` instead of
`vanguard[0]` for exactly this reason — this one branch was missed. This is a genuine, newly
reachable panic in library code, which this project's own conventions (CLAUDE.md: "Avoid
`unwrap()`/`expect()` and `panic!` in library code") explicitly forbid.

A secondary, lower-severity finding: `WarGraph::validate_battalion_children`'s recursive-embedding
check re-validates every descendant subgraph twice per level (once through the plain
`child.validate()` call, which internally re-invokes the same `validate_battalion_children` walk
with a truncated ancestry that can never itself detect anything, and once through the explicit
call passing the correct full-ancestry vector) — for a graph with several sibling `Battalion`
nodes at each nesting level this compounds into exponential-in-depth `validate()` calls on a
`WarGraph::validate` call that is otherwise entirely legitimate and acyclic.

## Critical Issues

### CR-01: `vanguard[0]` panics on an empty Vanguard during a muster-only superstep hitting the recursion limit

**File:** `crates/paladin-battalion/src/engine/superstep.rs:919-935`
**Issue:**

```rust
// --- ENG-FR-03: bounded iteration, checked at the top of the loop
// so a run stops at exactly `max_supersteps` rather than one over.
if superstep_number >= graph.limits().max_supersteps {
    let error = EngineError::RecursionLimitExceeded {
        limit: graph.limits().max_supersteps,
        thread_id: thread.clone(),
    };
    let waypoint = build_waypoint(
        &thread,
        parent_waypoint_id,
        superstep_number,
        graph,
        &battlefield,
        vanguard.clone(),
        Vec::new(),
        WaypointStatus::Failed {
            error: error.to_string(),
            failed_node: vanguard[0].clone(),   // <-- panics if vanguard is empty
        },
        ...
```

Before this phase, the loop could never re-enter an iteration with an empty `vanguard`: the
pre-Phase-23 code returned `RunOutcome::Completed` unconditionally whenever `next_vanguard` was
empty. This phase's Muster feature adds `has_pending_muster` specifically so a muster-only round
(the mustering node's only static successor is a worker template, which by design has no static
incoming edge and is therefore never selected by `compute_next_vanguard`) can produce an *empty*
`next_vanguard` while still carrying more work forward via `pending_muster`:

```rust
let status = if next_vanguard.is_empty() && !has_pending_muster {
    WaypointStatus::Completed
} else {
    WaypointStatus::Running
};
...
vanguard = next_vanguard;
pending_muster = mustered;
...
superstep_number += 1;
```

So `vanguard` can genuinely be `[]` on the next loop iteration while `pending_muster` is `Some`.
If `superstep_number` has now reached `graph.limits().max_supersteps`, the `RecursionLimitExceeded`
branch is taken with `vanguard` empty, and `vanguard[0]` panics — an index-out-of-bounds crash
inside the async `run_with_namespace` future (which, for a `NodeSpec::Battalion` child, is awaited
inline inside a `tokio::spawn`'d parent-node task, so this can also poison a Battalion child run).

This is proven reachable, not theoretical: it requires only a graph where a mustering node's
worker template(s) have no other ready sibling node in the round after the muster is accepted,
combined with `max_supersteps` being tight enough that the following (muster-dispatch) superstep
trips the limit. No existing test exercises this combination — `chain_needing_max_supersteps_trips_recursion_limit`
(the only `RecursionLimitExceeded` regression test) uses a plain linear chain with a permanently
non-empty vanguard, so the gap is untested. The exact sibling bug (using `vanguard[0]` as a
"some node from this superstep" fallback) was independently identified and fixed *in this same
diff* at the `Battlefield::merge` failure path (~line 1510-1513), which now reads
`ran.first().cloned().unwrap_or_else(|| dispatch_entries[0].0.clone())` instead of indexing
`vanguard` — this `RecursionLimitExceeded` branch was simply missed during that fix.

**Fix:** Use the same fallback pattern the merge-failure branch now uses — fall back to a node
that is guaranteed present regardless of whether the round is muster-only, e.g.:

```rust
failed_node: vanguard
    .first()
    .or_else(|| pending_muster.as_ref().map(|(node, _)| node))
    .cloned()
    .unwrap_or_else(|| /* graph-level marker NodeId, or restructure EngineError::RecursionLimitExceeded
                            to make failed_node optional */),
```

or, more simply, thread the mustering node's id (or the dispatch-entries list, already computed a
few lines below) through so this branch never needs to assume `vanguard` is non-empty. At minimum,
guard with `vanguard.first()` and synthesize a safe fallback rather than indexing unconditionally.

## Warnings

### WR-01: `validate_battalion_children` performs redundant, potentially exponential re-validation for nested `Battalion` graphs

**File:** `crates/paladin-battalion/src/engine/graph.rs:386-426` (`validate_battalion_children`)
**Issue:** For each `NodeSpec::Battalion` node, the method does two things per child: it calls
`child.validate(custom_dispatch, edge_evaluators)` (which internally re-runs the FULL validation
stack for `child`, *including* `child`'s own call to `validate_battalion_children` seeded with a
freshly-truncated ancestry `&[child.fingerprint()]` that starts over and can never detect a cycle
spanning more than the immediate child), and *also* explicitly calls
`child.validate_battalion_children(custom_dispatch, edge_evaluators, &next_ancestry)` with the
correct, full-depth ancestry. The first (implicit, via `child.validate()`) traversal is pure
redundant work for the recursive-embedding check specifically — only the second, explicit call can
ever find anything the first didn't already validate through its own structural/eligible/
schedulable checks.

For a chain of `N` nested `Battalion` nodes (or, worse, a shape with several sibling `Battalion`
nodes at each of `N` nesting levels), this redundancy compounds level over level: each `validate()`
call at depth `k` triggers a `validate_battalion_children` walk that itself issues a `validate()`
call at depth `k+1`, which repeats the same pattern. The result is roughly `O(2^N)` total
`validate()` invocations for a chain of depth `N`, not `O(N)` — an entirely legitimate, acyclic,
finite `WarGraph::validate()` call for a moderately deep (but structurally simple) nested-subgraph
configuration can take exponential time.

**Fix:** Have `validate_battalion_children` do the recursive-embedding accounting itself and call
only the non-recursive parts of `child.validate()` (or refactor `validate()` to accept an optional
`ancestry` parameter, defaulting to `&[self.fingerprint()]` for the public entry point and reusing
that single call site for the recursive descent instead of also calling
`validate_battalion_children` a second time explicitly):

```rust
fn validate_battalion_children(
    &self,
    custom_dispatch: &CustomDispatchResolver,
    edge_evaluators: &EdgeEvaluatorRegistry,
    ancestry: &[GraphFingerprint],
) -> Result<(), EngineError> {
    for id in &self.node_order {
        let Some(NodeSpec::Battalion { graph: child, .. }) = self.nodes.get(id) else { continue; };
        let child_fp = child.fingerprint();
        if ancestry.contains(&child_fp) { /* ... existing check ... */ }
        // validate the child's OWN structural/eligible/schedulable/evaluator rules,
        // but do NOT let that call re-walk validate_battalion_children with a
        // truncated ancestry -- only the explicit, full-ancestry call below should.
        child.validate_non_recursive(custom_dispatch, edge_evaluators)?;
        let mut next_ancestry = ancestry.to_vec();
        next_ancestry.push(child_fp);
        child.validate_battalion_children(custom_dispatch, edge_evaluators, &next_ancestry)?;
    }
    Ok(())
}
```

### WR-02: `MusterProgress::unfinished_tasks()` is documented as what resume uses, but production code never calls it

**File:** `crates/paladin-core/src/platform/container/waypoint.rs:359-384`,
`crates/paladin-battalion/src/engine/superstep.rs:1018-1029`
**Issue:** `waypoint.rs`'s module rustdoc states: *"Resume reads `MusterProgress::unfinished_tasks`
to decide which tasks still need to run -- it does NOT reconstruct that set from the Battlefield"*,
and `MusterProgress::unfinished_tasks()` is a `pub fn` with its own doc comment describing exactly
this role. In practice, `engine::superstep::run_with_namespace` never calls this method: it instead
carries the *full* `progress.tasks` list into `pending_muster` and separately re-derives the
identical filtered set inline —

```rust
let dispatch_tasks: Vec<MusterTask> = muster_tasks
    .iter()
    .filter(|task| !muster_carryover_this_round.contains_key(&task.task_key))
    .cloned()
    .collect();
```

— which is logically equivalent to `progress.unfinished_tasks()` but is a second, independent
implementation of the same filter. `unfinished_tasks()` is exercised only by its own unit test
(`muster_progress_unfinished_tasks_excludes_completed_keys`), never by the engine that the type's
own documentation says relies on it. This is a maintenance hazard: if either implementation is
later changed (e.g. the ordering guarantee, or the completed-key comparison), the two can silently
diverge with no test catching the mismatch, and the public API's documented contract becomes
inaccurate.

**Fix:** Either call `progress.unfinished_tasks()` from `run_with_namespace` directly (removing
the hand-rolled filter and the now-unnecessary intermediate `muster_carryover_this_round` filter
step), or update the rustdoc on both `MusterProgress::unfinished_tasks` and the module-level
comment to accurately describe that the engine re-derives the same set inline rather than calling
this method.

---

_Reviewed: 2026-09-04T03:45:16Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
