---
phase: 24-pause-resume-history-graceful-shutdown
plan: 07
subsystem: infra
tags: [rust, superstep-engine, chronicle, replay, fork, branch-lineage, subgraph-isolation]

# Dependency graph
requires:
  - phase: 24-pause-resume-history-graceful-shutdown
    provides: "Plan 24-06's branch-lineage storage contract: Waypoint.fork_of/WaypointSummary.fork_of threaded through the whole superstep engine, ThreadId::child_on_branch (the primitive, unwired), and the three-backend contract suite pinning both"
provides:
  - "WarEngine::replay(graph, thread, from) -- re-enters superstep::run_with_namespace from WaypointPort::get(thread, from), checking the graph fingerprint first, with parent_waypoint_id = Some(from), fork_of = Some(from), superstep numbering continuing at from.superstep + 1"
  - "WarEngine::fork(graph, thread, from, edit: StateDelta) -- like replay, but merges edit into the starting Battlefield through Battlefield::merge (a synthetic __fork_edit__ writer) before the first forked superstep runs"
  - "EngineError::WaypointNotFound { thread, waypoint } -- new #[non_exhaustive] variant for an unknown from"
  - "NodeSpec::Battalion dispatch in superstep.rs derives a branch's subgraph child thread via ThreadId::child_on_branch(parent, branch_root, node) when the run carries fork_of: Some(branch_root), keeping ThreadId::child unchanged for mainline runs -- wires 24-06's unwired primitive into production"
  - "ChronicleService (src/application/services/chronicle.rs) -- a thin, port-only read facade over Arc<dyn WaypointPort> with no paladin-battalion dependency: history (newest-first summaries with lineage), inspect (full Waypoint, typed NotFound), latest_on_branch (paginated filter over history by fork_of, never a full-Waypoint load)"
affects: [24-11]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A caller-suppliable StateDelta edit merged into a loaded Battlefield through the schema's own Battlefield::merge, under a synthetic writer NodeId (__fork_edit__), so an undeclared field is rejected with the SAME typed BattlefieldError::UnknownField a real node's delta would get -- no bespoke edit-validation path"
    - "A shared private impl method (replay_or_fork) behind two public one-line wrappers (replay/fork) differing only in an Option<StateDelta> -- avoids duplicating the fingerprint-check/validate/re-entry sequence across two near-identical public entry points"
    - "A dyn WaypointPort test double whose get() panics, used to prove an implementation reaches only history() for a read path (chronicle_latest_on_branch_needs_no_full_waypoint_loads) -- a stronger assertion than counting calls after the fact"

key-files:
  created:
    - src/application/services/chronicle.rs
  modified:
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/src/engine/superstep.rs
    - crates/paladin-battalion/src/engine/graph.rs
    - src/application/services/mod.rs
    - tests/integration/subgraph_formation_in_campaign_test.rs

key-decisions:
  - "replay/fork take thread: &ThreadId (not owned ThreadId like start/resume/resume_with) per the plan's own literal action-text signature -- a minor departure from this file's existing start/resume/resume_with convention, accepted as Claude's discretion since the plan spelled the signature out explicitly and a Chronicle-style read-then-maybe-write API reads more naturally borrowing its thread"
  - "fork's edit is merged under a synthetic NodeId::new(\"__fork_edit__\") writer, mirroring the existing __directive_parser_placeholder__ synthetic-id precedent in directive_parser.rs, rather than requiring a caller-supplied 'authoring node' for an edit that by definition names no real graph node"
  - "replay/fork propagate waypoint.muster_progress and re-enter at waypoint.superstep (not +1) when it is Some, mirroring resume_with_options's own mid-muster re-entry rule -- not explicitly required by this plan's tests, but the only self-consistent choice given the existing precedent and left untested here (no muster-mid-fork test exists; flagged in Known Gaps)"
  - "NodeSpec::Battalion's restart_on_resume field gained a rustdoc paragraph documenting that a run entered from a branch always starts its child fresh BY CONSTRUCTION of the distinct derived thread id, independent of the flag's own value -- CONTEXT.md's recommended resolution, applied without adding a second flag"
  - "ChronicleService.latest_on_branch paginates history() in fixed 500-item pages rather than requesting the whole thread unbounded in one call, mirroring paladin-storage's own prune_thread pagination pattern, so a very long thread's branch-latest resolution is still bounded per call"

patterns-established: []

requirements-completed: [HITL-03]

coverage:
  - id: D1
    description: "WarEngine::replay(graph, thread, from) re-enters the superstep loop from get(thread, from) with parent = from, fork_of = Some(from), superstep numbering continuing at from.superstep + 1, after checking the graph fingerprint"
    requirement: "HITL-03"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#replay_creates_a_new_branch_from_the_given_waypoint"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#replay_rejects_unknown_waypoint"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#replay_rejects_fingerprint_mismatch"
        status: pass
    human_judgment: false
  - id: D2
    description: "WarEngine::fork(graph, thread, from, edit) does the same and merges the StateDelta edit through the schema's dispatch rules before the first forked superstep"
    requirement: "HITL-03"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#fork_with_edit_flips_a_conditional_edge"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#fork_merges_the_edit_before_the_first_forked_superstep"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#fork_rejects_an_edit_the_schema_does_not_accept"
        status: pass
    human_judgment: false
  - id: D3
    description: "Immutability is byte-for-byte: every mainline Waypoint serialises to identical bytes before and after replay, and calling replay twice from the same Waypoint leaves the mainline byte-identical after both calls without disturbing the other branch"
    requirement: "HITL-03"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#replay_leaves_the_mainline_byte_identical"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#replay_twice_is_safe"
        status: pass
    human_judgment: false
  - id: D4
    description: "A branch runs its NodeSpec::Battalion children under ThreadId::child_on_branch(parent, branch_root, node), so latest(child_thread) on a fork never resolves the mainline child's history and the mainline child is untouched"
    requirement: "HITL-03"
    verification:
      - kind: unit
        ref: "tests/integration/subgraph_formation_in_campaign_test.rs#fork_child_thread_differs_from_mainline_child_thread"
        status: pass
      - kind: unit
        ref: "tests/integration/subgraph_formation_in_campaign_test.rs#fork_does_not_touch_mainline_child_waypoints"
        status: pass
      - kind: unit
        ref: "tests/integration/subgraph_formation_in_campaign_test.rs#latest_on_a_fork_child_thread_does_not_resolve_the_mainline_child"
        status: pass
      - kind: unit
        ref: "tests/integration/subgraph_formation_in_campaign_test.rs#mainline_runs_keep_using_child"
        status: pass
    human_judgment: false
  - id: D5
    description: "ChronicleService exposes history (newest-first summaries with lineage), inspect (a full Waypoint) and latest_on_branch over Arc<dyn WaypointPort> with no paladin-battalion dependency; latest_on_branch requires no full-Waypoint loads"
    requirement: "HITL-03"
    verification:
      - kind: unit
        ref: "src/application/services/chronicle.rs#chronicle_history_returns_newest_first_summaries_with_lineage"
        status: pass
      - kind: unit
        ref: "src/application/services/chronicle.rs#chronicle_history_honours_limit_and_before"
        status: pass
      - kind: unit
        ref: "src/application/services/chronicle.rs#chronicle_inspect_returns_the_full_waypoint"
        status: pass
      - kind: unit
        ref: "src/application/services/chronicle.rs#chronicle_latest_on_branch_filters_by_fork_of"
        status: pass
      - kind: unit
        ref: "src/application/services/chronicle.rs#chronicle_latest_on_branch_needs_no_full_waypoint_loads"
        status: pass
      - kind: unit
        ref: "src/application/services/chronicle.rs#chronicle_empty_thread_returns_empty_history"
        status: pass
      - kind: other
        ref: "cargo test -p paladin-ai --doc chronicle (4 doc tests)"
        status: pass
    human_judgment: false

duration: ~50min
completed: 2026-09-05
status: complete
---

# Phase 24 Plan 07: Chronicle Forkability — replay/fork, subgraph-fork isolation and ChronicleService Summary

**`WarEngine::replay`/`WarEngine::fork` re-enter the superstep loop from any past Waypoint with proven byte-for-byte mainline immutability, `NodeSpec::Battalion` dispatch now isolates a fork's subgraph children under `ThreadId::child_on_branch`, and `ChronicleService` exposes `history`/`inspect`/`latest_on_branch` as a thin, port-only, `paladin-battalion`-independent read facade.**

## Performance

- **Duration:** ~50 min
- **Tasks:** 3 (all TDD RED/GREEN pairs, one pair per task)
- **Files modified:** 5 modified, 1 created

## Accomplishments

- `WarEngine::replay(graph, thread, from)` and `WarEngine::fork(graph, thread, from, edit)` (`crates/paladin-battalion/src/engine/mod.rs`) load the starting Waypoint via `WaypointPort::get(thread, from)`, check the graph fingerprint before anything else (`EngineError::GraphMismatch`, ENG-FR-14), then re-enter `superstep::run_with_namespace` with `parent_waypoint_id = Some(from)`, `fork_of = Some(from)` and superstep numbering continuing at `from.superstep + 1` (or `from.superstep` unchanged when `from` carries a mid-muster progress record, mirroring `resume_with_options`'s own rule). `fork` additionally merges its `StateDelta` edit into the loaded Battlefield through `Battlefield::merge` — under a synthetic `__fork_edit__` writer `NodeId` — before the first forked superstep runs, so an edit naming a field the schema does not declare fails with the same typed `BattlefieldError::UnknownField` a real node's delta would produce, persisting nothing. A new `EngineError::WaypointNotFound { thread, waypoint }` variant covers an unknown `from`. Neither method mutates, overwrites or deletes an existing Waypoint — proven byte-for-byte: every mainline Waypoint serialises to identical bytes before and after a `replay`, and two independent `replay` calls from the same Waypoint each produce their own distinct branch without disturbing the other or the mainline.
- The `NodeSpec::Battalion` dispatch arm in `superstep.rs` now derives a subgraph child's thread id via `ThreadId::child_on_branch(parent, branch_root, node)` when the run carries `fork_of: Some(branch_root)` — wiring in the injective primitive Plan 24-06 landed but left unwired — and keeps `ThreadId::child(parent, node)` unchanged for every mainline run. A fork's subgraph child therefore always starts fresh: the derived id has no prior history for the resume-mid-child lookup to find, which follows by construction from the distinct id rather than a second `restart_on_resume`-style flag, documented both on the dispatch arm and on `NodeSpec::Battalion`'s `restart_on_resume` field rustdoc.
- `ChronicleService` (`src/application/services/chronicle.rs`, new, registered in `src/application/services/mod.rs`) mirrors `waypoint_retention.rs`'s own shape — a struct over `Arc<dyn WaypointPort>`, a constructor taking it, no other dependency, no `paladin_battalion` import (ADR-0031) — so `paladin-web` can later reuse these same reads through the same port unchanged (plan 24-11). `history(thread, limit, before)` passes straight through to `WaypointPort::history` (widening `limit` to `u32`, saturating rather than wrapping on overflow); `inspect(thread, waypoint)` returns the full `Waypoint` or a typed `WaypointError::NotFound` (deliberately stricter than `WaypointPort::get`'s own "missing is `None`" contract, since for a caller-facing inspect the absence itself is the answer); `latest_on_branch(thread, branch_root)` is a paginated filter over `history` alone by `fork_of` — proven, not merely asserted, to never call `WaypointPort::get` via a `GetPanicsStore` test double whose `get()` panics.
- HITL-03 marked complete in `.planning/REQUIREMENTS.md` (checkbox + traceability row) — this plan lands the engine-level `replay`/`fork` operations and the `ChronicleService` read facade that Plan 24-06 explicitly deferred to "plan 24-07" in its own SUMMARY.

## Task Commits

1. **Task 1: `WarEngine::replay`/`WarEngine::fork` with byte-for-byte mainline immutability** — RED/GREEN pair:
   - `960e7593` — `test(24-07): reproduce replay/fork lineage and immutability on not-yet-existing API (red)` — 8 tests added referencing `WarEngine::replay`/`fork`/`EngineError::WaypointNotFound`, none of which exist yet; 10 compile errors (`E0599` x9, missing enum variant).
   - `1391e77b` — `feat(24-07): land WarEngine::replay/fork with byte-for-byte mainline immutability (HITL-03, D-16/D-17)` — all 8 tests green.
2. **Task 2: Subgraph forks never share child Waypoints** — RED/GREEN pair:
   - `adb836ff` — `test(24-07): reproduce fork subgraph-child isolation gap on current dispatch (red)` — 4 tests added to `tests/integration/subgraph_formation_in_campaign_test.rs`; 2 of 4 fail (assertion failures, not compile errors, since `WarEngine::fork`/`ThreadId::child_on_branch` already exist) because the current dispatch arm ignores `fork_of`.
   - `5322185c` — `feat(24-07): isolate subgraph forks under ThreadId::child_on_branch (HITL-03, D-18)` — all 4 tests green (33/33 in the full test binary).
3. **Task 3: `ChronicleService` — the port-only Chronicle read facade** — RED/GREEN pair:
   - `92be608f` — `test(24-07): reproduce ChronicleService's read contract on not-yet-existing API (red)` — 6 tests added against a not-yet-existing struct; 6 `E0433` compile errors.
   - `74392242` — `feat(24-07): land ChronicleService as a thin port-only read facade (HITL-03, D-16)` — all 6 unit tests + 4 doc tests green; `cargo test --workspace` (44 binaries), `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` all clean.

**Plan metadata:** (this commit)

## Files Created/Modified

- `crates/paladin-battalion/src/engine/mod.rs` — `WarEngine::replay`, `WarEngine::fork`, private shared `replay_or_fork`, `EngineError::WaypointNotFound`; 8 new tests.
- `crates/paladin-battalion/src/engine/superstep.rs` — `NodeSpec::Battalion` dispatch derives `child_thread` via `ThreadId::child_on_branch` when `resources.fork_of` is `Some`.
- `crates/paladin-battalion/src/engine/graph.rs` — `NodeSpec::Battalion.restart_on_resume` field rustdoc documents the branch-always-fresh resolution.
- `src/application/services/chronicle.rs` — new: `ChronicleService` (`history`, `inspect`, `latest_on_branch`); 6 unit tests + 4 doc tests.
- `src/application/services/mod.rs` — `pub mod chronicle;`.
- `tests/integration/subgraph_formation_in_campaign_test.rs` — 4 new tests (`fork_child_thread_differs_from_mainline_child_thread`, `fork_does_not_touch_mainline_child_waypoints`, `latest_on_a_fork_child_thread_does_not_resolve_the_mainline_child`, `mainline_runs_keep_using_child`) plus a `full_history_dyn` helper; no new `[[test]]` entry (already registered).

## Decisions Made

- **`replay`/`fork` take `thread: &ThreadId`**, not the owned `ThreadId` `start`/`resume`/`resume_with` use — the plan's own action text spelled this signature out literally; followed verbatim rather than reconciled with the file's other convention.
- **The fork edit's synthetic writer is `NodeId::new("__fork_edit__")`**, mirroring the existing `__directive_parser_placeholder__` synthetic-id precedent in `directive_parser.rs` — an edit that by definition names no real graph node needed *a* writer identity for `Battlefield::merge`'s conflict bookkeeping, not a caller-supplied one.
- **`replay`/`fork` propagate `muster_progress` and re-enter at the SAME superstep when it is `Some`**, mirroring `resume_with_options`'s own mid-muster rule — the only self-consistent choice, though untested by this plan (see Known Gaps).
- **`NodeSpec::Battalion.restart_on_resume`'s rustdoc now states the branch-always-fresh resolution explicitly** — CONTEXT.md's recommended answer (no second flag; fresh-by-construction of the distinct thread id), applied to the field the plan named as "declared here... but does not itself act on it."
- **`ChronicleService.latest_on_branch` paginates in fixed 500-item pages**, mirroring `paladin-storage`'s own `prune_thread` pagination pattern, so a very long thread's branch-latest resolution stays bounded per call rather than requesting the whole history unbounded.

## Deviations from Plan

None — plan executed exactly as written. Both the RED/GREEN split and the design of Test 4/5's fixture graphs (a `seed`/`router`/`node_a`/`node_b` conditional-routing graph for the edge-flip test, a `seed`/`reader` graph for the visible-to-first-node test) were Claude's discretion within the plan's own behavior specifications, not deviations from them.

## Issues Encountered

None. `cargo test --workspace --no-fail-fast` (44 test binaries) passed with 0 failures on the first run after Task 3's GREEN commit; no flaky test was observed in this plan's own scope (the pre-existing `e2e_crash_resume` timing flake documented in Plans 24-02…24-06 was not re-triggered, since this plan's own full-workspace run completed cleanly).

## Known Gaps

- **No test exercises `replay`/`fork` from a Waypoint carrying `muster_progress: Some(_)`.** The mid-muster re-entry branch (re-enter at `from.superstep`, not `+1`) is implemented by direct analogy to `resume_with_options`'s own precedent but is not covered by a dedicated test in this plan. Low risk (the code path is a straight mirror of already-tested logic), but noted for a future plan or audit to close if `replay`/`fork` are ever invoked against a mid-muster Waypoint in practice.
- **No test exercises `fork` on a graph containing a `NodeSpec::Gate` node.** Task 1's tests use plain `Function` nodes throughout; Gate-specific interaction with `fork`'s edit-merge (e.g. forking with an edit that changes a Gate's `output_field`) is untested here.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- HITL-03 (Chronicle inspectable and forkable, HITL-FR-07…12) is now fully implemented and requirement-complete: `replay`/`fork` on the engine, branch-scoped subgraph isolation, and `ChronicleService`'s port-only read facade all proven by tests, matching Plan 24-06's own forward note that this plan would be "whichever later plan lands `WarEngine::replay`/`WarEngine::fork`/`ChronicleService`."
- `ChronicleService` is ready for Plan 24-11 (`paladin-web` thread routes) to consume unchanged through the same `Arc<dyn WaypointPort>` — no `paladin-battalion` dependency was introduced, preserving ADR-0031's default-build leaf-to-leaf boundary.
- No blockers. The two Known Gaps above (mid-muster fork, Gate-node fork) are low-risk, narrow-scope items for a future plan or audit to close if they become load-bearing.

## Self-Check: PASSED

All 6 files (5 modified, 1 created) verified present on disk; all 6 commit hashes (`960e7593`, `1391e77b`, `adb836ff`, `5322185c`, `92be608f`, `74392242`) verified present in `git log --oneline --all`.

---
*Phase: 24-pause-resume-history-graceful-shutdown*
*Completed: 2026-09-05*
