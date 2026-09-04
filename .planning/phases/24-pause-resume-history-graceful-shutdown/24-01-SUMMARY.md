---
phase: 24-pause-resume-history-graceful-shutdown
plan: 01
subsystem: infra
tags: [rust, tokio, superstep-engine, state-machine, hitl]

# Dependency graph
requires:
  - phase: 23-control-flow-dynamic-routing-fan-out-subgraphs
    provides: "Directive/NextStep routing (NextStep::Parley stub, ParleyNotSupported failure arm), the superstep dispatch/join loop, EngineError conventions"
provides:
  - "paladin-core::platform::container::parley — ParleyId/ParleyKind/OnExpire/ParleyRequest/ParleyResponse value types (D-01)"
  - "WaypointStatus::AwaitingInput{parleys,responses} and RunOutcome::AwaitingInput{parleys,waypoint} reshape (D-02)"
  - "NodeOutcomeKind::Parleyed (D-03)"
  - "Real Parley suspension in superstep.rs: peers and the raising node's own delta merge normally, one multi-parley AwaitingInput Waypoint persisted, vanguard = parleying nodes"
  - "WarEngine::resume_with(graph, thread, responses) — the only path that advances a suspended thread (HITL-02)"
  - "NodeContext.parley_response / parley_response() accessor, populated on the post-resume re-run"
  - "Typed guards: EngineError::ThreadAwaitingInput (plain resume vs. suspended thread), EngineError::UnknownParleyId, EngineError::ThreadNotAwaitingInput, EngineError::ParleyInChildUnsupported"
affects: [24-02, 24-03, 24-04, 24-05, 24-06]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Suspension checked ahead of end_requested in the superstep loop's post-merge section, mirroring End's own StarvedNodeAtCompletion-bypass rationale"
    - "run_with_namespace made pub(crate) with a new trailing initial_parley_responses parameter, consumed via .take() on the first loop iteration only — run()'s own public signature stays unchanged, matching the existing checkpoint_ns precedent"
    - "Engine stamps node_id onto a raised ParleyRequest regardless of what the raising code supplied, so the persisted parleys list is always accurate"

key-files:
  created:
    - crates/paladin-core/src/platform/container/parley.rs
  modified:
    - crates/paladin-core/src/platform/container/waypoint.rs
    - crates/paladin-core/src/platform/container/directive.rs
    - crates/paladin-core/src/platform/container/mod.rs
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/src/engine/superstep.rs
    - crates/paladin-battalion/src/engine/node.rs
    - crates/paladin-battalion/src/engine/graph.rs
    - crates/paladin-battalion/src/engine/hooks.rs
    - crates/paladin-storage/src/waypoint/retention.rs
    - src/application/services/waypoint_retention.rs
    - tests/integration/waypoint_retention_fault_injection_test.rs

key-decisions:
  - "Task 1 checkpoint (proceed-as-locked) auto-resolved by the orchestrator under auto-mode: the AwaitingInput/RunOutcome reshape proceeds exactly as D-01/D-02 lock it, one-way after v0.10.0 ships"
  - "Suspension is checked in the superstep loop ahead of the End check: a same-superstep Parley+End conflict (not exercised by any test) resolves in Parley's favor"
  - "run_with_namespace (not the public run wrapper) gained the new parley-responses parameter and pub(crate) visibility, so WarEngine::resume_with can call it directly without touching run()'s signature or any of its existing call sites"

patterns-established:
  - "Pattern: a suspending NextStep variant marks its own node NotFiring but still merges its delta (mirrors Goto/Muster/End's D-08c uniformity) rather than treating suspension as a routing failure"
  - "Pattern: EngineError variants superseded by later phases are retained unconstructed with an updated rustdoc note, never removed, per X-03"

requirements-completed: [HITL-01, HITL-02]

coverage:
  - id: D1
    description: "A node returning NextStep::Parley suspends the run (not fails it): exactly one multi-parley AwaitingInput Waypoint persisted, RunOutcome::AwaitingInput returned"
    requirement: "HITL-01"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#parley_suspends_run_and_persists_awaiting_input"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#awaiting_input_vanguard_is_exactly_the_parleying_nodes"
        status: pass
    human_judgment: false
  - id: D2
    description: "Peer nodes merge normally alongside a parleying node; the parleying node's own delta also merges at raise time and its outcome is recorded as Parleyed"
    requirement: "HITL-01"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#parley_waypoint_records_parleyed_outcome_and_merges_peer_deltas"
        status: pass
    human_judgment: false
  - id: D3
    description: "WarEngine::resume_with delivers a ParleyResponse to the paused node's continuation via NodeContext.parley_response() and the run reaches RunOutcome::Completed"
    requirement: "HITL-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#parley_suspends_and_resumes_end_to_end"
        status: pass
    human_judgment: false
  - id: D4
    description: "resume_with rejects an unknown parley_id (EngineError::UnknownParleyId) and writes no Waypoint"
    requirement: "HITL-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#resume_with_unknown_parley_id_fails_and_writes_no_waypoint"
        status: pass
    human_judgment: false
  - id: D5
    description: "Plain resume/resume_with_options against an AwaitingInput thread fails closed with EngineError::ThreadAwaitingInput, writing no Waypoint (RESEARCH.md Pitfall 2)"
    requirement: "HITL-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#plain_resume_refuses_awaiting_input_thread"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#plain_resume_still_continues_a_halted_thread"
        status: pass
    human_judgment: false
  - id: D6
    description: "A nested NodeSpec::Battalion child that suspends fails the parent with the structured EngineError::ParleyInChildUnsupported, naming the node and child thread"
    requirement: "HITL-01"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#parley_in_battalion_child_is_typed_error"
        status: pass
    human_judgment: false
  - id: D7
    description: "The reshaped AwaitingInput Waypoint payload round-trips through serde with both parleys and responses preserved"
    requirement: "HITL-01"
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/waypoint.rs#awaiting_input_status_round_trips_through_serde"
        status: pass
    human_judgment: false

duration: ~110min
completed: 2026-09-04
status: complete
---

# Phase 24 Plan 01: Parley Pause/Resume Tracer Summary

**Real suspend-persist-resume spine for HITL-01/HITL-02: `NextStep::Parley` now suspends a run into a multi-parley `AwaitingInput` Waypoint instead of failing it, and `WarEngine::resume_with` delivers a `ParleyResponse` back to the paused node's continuation to completion.**

## Performance

- **Duration:** ~110 min
- **Tasks:** 3 (1 auto-resolved checkpoint, 2 tracer/auto implementation tasks)
- **Files modified:** 11 modified, 1 created

## Accomplishments

- `parley.rs` lands the full D-01 value-type set (`ParleyId`, `ParleyKind`, `OnExpire`, `ParleyRequest`, `ParleyResponse`) in `paladin-core`, replacing the Phase 22 `{ prompt }` stub, re-exported from `waypoint.rs` so every existing `use` path keeps resolving unchanged.
- `WaypointStatus::AwaitingInput` and `RunOutcome::AwaitingInput` reshaped to `{ parleys: Vec<ParleyRequest>, responses: Vec<ParleyResponse> }` / `{ parleys, waypoint }` (D-02); `NodeOutcomeKind::Parleyed` added (D-03).
- `superstep.rs`'s `NextStep::Parley` arm now suspends the run for real: the raising node's own delta merges normally (mirroring Goto/Muster/End's D-08c uniformity), every parley raised in the superstep is collected, sorted by `node_id`, and persisted onto exactly one `AwaitingInput` Waypoint whose `vanguard` is the parleying nodes — checked ahead of `end_requested`, bypassing `StarvedNodeAtCompletion` the same way `End` does.
- `WarEngine::resume_with(graph, thread, responses)` added: validates the loaded thread is `AwaitingInput`, checks every response's `parley_id` against the loaded Waypoint's own `parleys` (never a global lookup), then re-enters the superstep loop with `vanguard` forced to the persisted parleying nodes and each dispatched node's `NodeContext.parley_response` populated with its matching answer.
- Plain `resume`/`resume_with_options` now fails closed against a suspended thread with `EngineError::ThreadAwaitingInput`, inserted **before** the generic vanguard-restore fallthrough — closing RESEARCH.md's Pitfall 2 (the fallthrough would otherwise silently re-run parleying nodes as if nothing were pending).
- A nested `NodeSpec::Battalion` child that suspends now fails the parent with the structured `EngineError::ParleyInChildUnsupported { node, child_thread }` (D-04), replacing a stringly `NodeError`, with an actionable message ("raise the parley in the parent graph instead") and the limit documented on `NodeSpec::Battalion`'s own rustdoc.
- `EngineError::ParleyNotSupported` retained, unconstructed, per X-03 (rustdoc updated to record it as superseded).

## Task Commits

1. **Task 1: Confirm the stored Parley contract before it is written** — checkpoint auto-resolved by the orchestrator under auto-mode (`⚡ Auto-selected: proceed-as-locked`); no code commit, recorded here.
2. **Task 2/3: End-to-end suspend/resume + typed guards** — split into TDD RED/GREEN pairs per this codebase's established convention (see `git log` precedent at `708f07c8`, `bedb94af`, etc.):
   - `19e51fe2` — `test(24-01): reproduce Parley suspend/resume and typed guards on not-yet-existing API (red)` — nine tests added referencing not-yet-existing types/methods/variants; crate fails to compile (28 errors in `paladin-core`, 34 in `paladin-battalion`).
   - `b361f85b` — `feat(24-01): land Parley suspend/resume and typed guards (HITL-01, HITL-02)` — the full implementation; all tests green.

**Plan metadata:** (this commit)

## Files Created/Modified

- `crates/paladin-core/src/platform/container/parley.rs` — new: `ParleyId`, `ParleyKind`, `OnExpire`, `ParleyRequest`, `ParleyResponse`, each with rustdoc doc tests using `?`.
- `crates/paladin-core/src/platform/container/waypoint.rs` — re-exports the parley types; removed the old `ParleyRequest` stub; reshaped `WaypointStatus::AwaitingInput`; added `NodeOutcomeKind::Parleyed`; new serde round-trip test.
- `crates/paladin-core/src/platform/container/directive.rs` — `NextStep::Parley`'s rustdoc now describes the real suspension/resume contract instead of promising a future phase.
- `crates/paladin-core/src/platform/container/mod.rs` — registers `pub mod parley;`.
- `crates/paladin-battalion/src/engine/mod.rs` — `RunOutcome::AwaitingInput` reshape; four new `EngineError` variants; the `ThreadAwaitingInput` guard in `resume_with_options`; the new `resume_with` method; six new tests.
- `crates/paladin-battalion/src/engine/superstep.rs` — the Parley suspension arm rewrite, the suspend-and-return block, `run_with_namespace`'s new `initial_parley_responses` parameter (now `pub(crate)`), the `ParleyInChildUnsupported` arm, `NodeContext.parley_response` wiring; three new tests replacing the old inverted-assertion test.
- `crates/paladin-battalion/src/engine/node.rs` — `NodeContext.parley_response: Option<ParleyResponse>` + `parley_response()` accessor.
- `crates/paladin-battalion/src/engine/graph.rs` — `NodeSpec::Battalion`'s rustdoc documents the D-04 limit.
- `crates/paladin-battalion/src/engine/hooks.rs` — mechanical test-fixture fix (`NodeContext` literal) for the new field.
- `crates/paladin-storage/src/waypoint/retention.rs`, `src/application/services/waypoint_retention.rs`, `tests/integration/waypoint_retention_fault_injection_test.rs` — three existing `AwaitingInput` test fixtures updated to the two-field shape; their protection-logic wildcard matches already used `{ .. }` so retention behavior is unaffected.

## Decisions Made

- **Task 1 checkpoint auto-resolved** (`proceed-as-locked`): finalise the Parley types and the `AwaitingInput`/`RunOutcome` reshape exactly as D-01/D-02 lock them. Reversibility: one-way after v0.10.0 ships (a stored Waypoint contract); free now per `MIGRATION.md` §9.2. No in-workspace consumer outside this plan's own files was affected by the reshape (confirmed via `grep -rn "ParleyRequest"` and `AwaitingInput` before starting).
- Suspension is checked in the superstep loop **ahead of** the `End` check — a same-superstep `Parley` + `End` conflict (not exercised by any test, and not named by any plan decision) resolves in `Parley`'s favor; documented in the suspend block's own comment.
- `run_with_namespace` (the private-turned-`pub(crate)` implementation function), not the public `run` wrapper, gained the new `initial_parley_responses` trailing parameter — `WarEngine::resume_with` calls it directly, so `run()`'s own public signature (and every existing call site: `engine::mod`'s `start`/`resume_with_options`, `engine::graph`'s tests, the recursive Battalion-child dispatch) needed zero changes, matching the plan's explicit instruction and the existing `checkpoint_ns` precedent.
- The engine stamps `node_id` onto a raised `ParleyRequest` regardless of what the raising code supplied (defensive correctness, Rule 2) — a test author or a future Paladin-directive parser cannot desynchronize the persisted `parleys` list's `node_id` from the actual raising node.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Updated `hooks.rs`'s `NodeContext` test fixture**
- **Found during:** Task 2 (adding `NodeContext.parley_response`)
- **Issue:** `crates/paladin-battalion/src/engine/hooks.rs` (not named in the plan's `files_modified` list) constructs a `NodeContext` struct literal in its own `#[cfg(test)]` fixture (`ctx()`, used by `TraceDispatcher`/`NodeInterceptor` tests unrelated to Parley); adding the new field to `NodeContext` broke this literal's exhaustiveness.
- **Fix:** Added `parley_response: None,` to the fixture.
- **Files modified:** `crates/paladin-battalion/src/engine/hooks.rs`
- **Verification:** `cargo test -p paladin-battalion --lib` — all `engine::hooks::tests::*` pass unchanged.
- **Committed in:** `b361f85b` (GREEN commit)

---

**Total deviations:** 1 auto-fixed (1 blocking). `crates/paladin-battalion/src/engine/graph.rs` was already named in Task 3's own `<files>` list, so its rustdoc update is plan-scoped, not a deviation.
**Impact on plan:** Necessary mechanical fixup for compilation; no scope creep.

## Issues Encountered

- **TDD RED/GREEN commit split across shared files.** Tests and production code landed in the same commit-in-progress across `waypoint.rs`, `engine/mod.rs`, and `superstep.rs`. Rather than committing them together, the production-only hunks were temporarily reverted (using the Edit tool, precisely reversing each change) to reproduce a genuine RED state — verified via `cargo test --no-run` (28 compile errors in `paladin-core`, 34 in `paladin-battalion`, matching the exact "reproduce X on not-yet-existing API (red)" convention this codebase already uses, e.g. commit `708f07c8`) — committed, then the production code was re-applied and verified GREEN (`cargo test -p paladin-ai-core` 450+66 passed, `cargo test -p paladin-battalion --lib` 482 passed) before the GREEN commit.
- **Pre-commit hook timeout.** The workspace's pre-commit hook runs a cold `cargo clippy --workspace --all-targets --all-features`, which exceeded the 2-minute command timeout on the first commit attempt. Committed with `--no-verify` per the orchestrator's `workflow.worktree_skip_hooks=true` allowance for this run; `cargo fmt --check` and `cargo clippy -p <crate> -- -D warnings` were run and verified clean for every crate touched (`paladin-ai-core`, `paladin-battalion`, `paladin-storage`, `paladin-ai`) before each commit.

## User Setup Required

None — no external service configuration required.

## Note on REQUIREMENTS.md

`requirements-completed` in this SUMMARY's frontmatter lists `HITL-01`/`HITL-02` per this plan's
own frontmatter, but **`.planning/REQUIREMENTS.md`'s checkboxes were deliberately NOT marked
complete** for either requirement. Both are multi-plan (per this plan's own coverage table:
HITL-01 needs plans 01, 02, 03, 05; HITL-02 needs plans 01, 03, 04, 05) and this plan is
explicitly the tracer slice — `gsd_run query requirements.mark-complete HITL-01 HITL-02` was run,
found it flips both to `[x]`/`Complete` in the traceability table after only this plan, and that
write was reverted before this commit as inaccurate. Whichever later plan is the LAST to land its
share of each requirement (24-03/24-05 for HITL-01 per the coverage table, 24-04/24-05 for
HITL-02) should be the one to run `requirements.mark-complete` for it.

## Next Phase Readiness

- The suspend-persist-resume spine is proven end-to-end over `InMemoryWaypointStore` and is the foundation every later Phase 24 plan builds on: `NodeSpec::Gate` (24-02) dispatches through the same `NextStep::Parley` path; the `parley.` `InputMapping` namespace (24-03) reads `NodeContext.parley_response`; the richer validation matrix — `ParleyAlreadyAnswered`, `ResponseShapeInvalid`, `ParleyExpired`, partial-answer persistence (D-11's second half) — is explicitly deferred to plan 24-04, which this plan's `resume_with` happy-path guards (`ThreadNotAwaitingInput`, `UnknownParleyId`) were designed not to preclude.
- No blockers. The Postgres Tier-2 contract suite carried-forward concern from Phase 23 is unaffected by this plan (no contract-suite changes here; D-02's three-backend round-trip cases are plan 24-06's responsibility per the CONTEXT.md coverage table).

## Self-Check: PASSED

All 12 files verified present on disk; both commit hashes (`19e51fe2` RED, `b361f85b` GREEN) verified present in `git log --oneline --all`.

---
*Phase: 24-pause-resume-history-graceful-shutdown*
*Completed: 2026-09-04*
