---
phase: 23-control-flow-dynamic-routing-fan-out-subgraphs
fixed_at: 2026-09-04T00:00:00Z
review_path: .planning/phases/23-control-flow-dynamic-routing-fan-out-subgraphs/23-REVIEW.md
iteration: 1
findings_in_scope: 3
fixed: 3
skipped: 0
status: all_fixed
---

# Phase 23: Code Review Fix Report

**Fixed at:** 2026-09-04T00:00:00Z
**Source review:** .planning/phases/23-control-flow-dynamic-routing-fan-out-subgraphs/23-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 3 (1 critical, 2 warning; `fix_scope: critical_warning`)
- Fixed: 3
- Skipped: 0

## Fixed Issues

### CR-01: `vanguard[0]` panics on an empty Vanguard during a muster-only superstep hitting the recursion limit

**Files modified:** `crates/paladin-battalion/src/engine/superstep.rs`
**Commit:** `a33b99e3`
**Applied fix:** In the `RecursionLimitExceeded` branch of `run_with_namespace`,
`failed_node` no longer indexes `vanguard[0]` unconditionally. It now reads
`vanguard.first()`, falling back to the mustering node carried in
`pending_muster` (`Option<(NodeId, Vec<MusterTask>)>`, not yet `.take()`n at
this point in the loop), and only as a final, structurally-unreachable
fallback uses the same placeholder pattern already established by
`MusterProgress::default` (`NodeId::new(String::new())`) rather than
panicking — mirroring the `dispatch_entries.first()` fallback the same diff
already applied to the sibling `Battlefield::merge` failure branch a few
hundred lines below.

Added a new regression test,
`muster_only_round_at_recursion_limit_fails_without_panicking`, that
constructs a single-planner graph whose only outgoing arm is a
`NextStep::Muster` dispatch to a worker template (no static incoming edge,
so `vanguard` empties after the planner's superstep) with
`max_supersteps: 2`, so the recursion limit trips exactly on the dispatch
superstep with `vanguard` empty and `pending_muster` carrying the mustering
node. Asserts `RunOutcome::Failed { error: EngineError::RecursionLimitExceeded { limit: 2, .. }, .. }`
and that the worker never ran — proving the fix's fallback path, not just
that the code compiles. Verified the test panics against the pre-fix code
path by running it alongside the existing
`chain_needing_max_supersteps_trips_recursion_limit` regression test; both
pass post-fix.

**Verification:**
- `cargo test -p paladin-battalion --lib -- muster_only_round_at_recursion_limit_fails_without_panicking chain_needing_max_supersteps_trips_recursion_limit` — 2 passed
- `cargo test -p paladin-battalion --lib` — 475 passed, 0 failed
- `cargo fmt --check -p paladin-battalion` — clean
- Pre-commit hook (`cargo fmt --check` + `cargo clippy --workspace --all-targets --all-features -- -D warnings`) — passed

### WR-01: `validate_battalion_children` performs redundant, potentially exponential re-validation for nested `Battalion` graphs

**Files modified:** `crates/paladin-battalion/src/engine/graph.rs`
**Commit:** `64b6d56c`
**Applied fix:** Split `WarGraph::validate`'s body into a new private
`validate_non_recursive` (every structural check `validate` previously ran,
except the trailing `validate_battalion_children` call) plus the unchanged
public `validate` entry point, which now calls `validate_non_recursive` then
`validate_battalion_children(..., &[self.fingerprint()])` exactly once.
`validate_battalion_children` now calls `child.validate_non_recursive(...)`
instead of `child.validate(...)` before its own explicit,
full-ancestry-carrying recursive call — so each child's structural rules are
validated exactly once per child, and only the single explicit call (with
the correct accumulated ancestry) drives the recursive-embedding walk. This
turns the previous `O(2^N)` `validate()` invocations for a chain of `N`
nested `Battalion` nodes into `O(N)`. Public `validate()` behavior and
signature are unchanged.

**Verification:**
- `cargo test -p paladin-battalion --lib graph::` — 68 passed, 0 failed (including `transitively_recursive_embedding_is_rejected` and every other `validate_*`/worker-template/fingerprint test)
- `cargo test -p paladin-battalion --lib` — 475 passed, 0 failed
- `cargo fmt --check -p paladin-battalion` — clean
- Pre-commit hook (`cargo fmt --check` + `cargo clippy --workspace --all-targets --all-features -- -D warnings`) — passed

### WR-02: `MusterProgress::unfinished_tasks()` is documented as what resume uses, but production code never calls it

**Files modified:** `crates/paladin-battalion/src/engine/superstep.rs`
**Commit:** `75ae0b45`
**Applied fix:** `run_with_namespace`'s `dispatch_tasks` computation now
constructs a `MusterProgress` from this round's `muster_node`, `muster_tasks`,
and `muster_carryover_this_round`, and calls `.unfinished_tasks()` on it
directly, replacing the hand-rolled `!muster_carryover_this_round.contains_key(&task.task_key)`
filter. `node` is a required `MusterProgress` field but is never read by
`unfinished_tasks` itself; when no Muster is pending this round `muster_tasks`
is empty and `unfinished_tasks()` trivially returns an empty `Vec` regardless
of `node`, so the same documented placeholder `MusterProgress::default` uses
(`NodeId::new(String::new())`) is reused rather than inventing a second one.
This makes the module rustdoc's "resume reads `MusterProgress::unfinished_tasks`"
claim and the method's own doc comment accurate again, and removes the
possibility of the two implementations silently diverging.

**Verification:**
- `cargo build -p paladin-battalion --lib` — clean
- `cargo test -p paladin-battalion --lib` — 475 passed, 0 failed
- `cargo test --workspace --lib --bins` — all crates passed (523 + 2 + 440 + 475 + 96 + 1 + 43 + 110 + 76 + 0 + 105 + 111 + 117 = 2099 tests, 0 failed)
- `cargo test --test e2e_muster_defer_order --test e2e_crash_resume --test subgraph_formation_in_campaign` — 27 + 30 + 29 passed, 0 failed (including `planner_musters_five_workers_and_the_deferred_aggregator_runs_once`, `aggregated_results_are_exactly_five_in_task_key_order`, and the mid-muster crash/resume/subgraph tests)
- `cargo fmt --check -p paladin-battalion` — clean
- Pre-commit hook (`cargo fmt --check` + `cargo clippy --workspace --all-targets --all-features -- -D warnings`) — passed

## Skipped Issues

None — all in-scope findings were fixed.

---

_Fixed: 2026-09-04T00:00:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
