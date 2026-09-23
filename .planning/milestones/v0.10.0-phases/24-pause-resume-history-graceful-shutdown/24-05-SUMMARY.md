---
phase: 24-pause-resume-history-graceful-shutdown
plan: 05
subsystem: infra
tags: [rust, integration-tests, superstep-engine, hitl, concurrency-stress]

# Dependency graph
requires:
  - phase: 24-pause-resume-history-graceful-shutdown
    provides: "Plans 24-01..24-04's complete Parley pause/resume spine: NextStep::Parley suspension, NodeSpec::Gate, WarEngine::resume_with's full D-10/D-11/D-12 validation matrix, the parley. InputMapping namespace and envelope-raised parleys -- this plan implements nothing new, it proves the assembled spine at the program level"
provides:
  - "tests/integration/e2e_approval_gate_test.rs -- program acceptance scenario E2E-2, registered as the e2e_approval_gate [[test]] target"
  - "tests/integration/multi_parley_suspension_test.rs -- multi-parley suspension proven from the persisted Waypoint alone, registered as the multi_parley_suspension [[test]] target"
  - "tests/integration/parley_resume_stress_test.rs -- X-05 concurrency stress (acceptance criterion 7), registered as the parley_resume_stress [[test]] target"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Cross-process simulation for a suspended thread: construct engine instance A over a fresh SqliteWaypointStore temp-file, run to suspension, drop it, then construct an entirely new engine instance B over the SAME file path and resume through it -- mirrors tests/integration/e2e_crash_resume_test.rs's own documented technique, never reusing one engine/store across the drop"
    - "Every durability assertion reads through a freshly constructed WaypointPort handle (WaypointPort::latest/history), never the engine's own returned RunOutcome value -- proves the suspension/partial-answer state is durable, not merely in-memory"
    - "X-05 concurrency stress with no in-tree analog: #[tokio::test(flavor = \"multi_thread\")], real tokio::spawn per concurrent resume_with call (not just interleaved polling within one task), joined through futures::future::join_all, wrapped in an explicit tokio::time::timeout so a deadlock fails fast"
    - "Local, file-scoped StateNode test doubles (FixedOutputNode, ParleyingFunctionNode) instead of the crate's own pub(crate) test_support.rs fixtures (CountingFunctionNode etc.), which are not visible to an external integration-test binary -- mirrors e2e_crash_resume_test.rs's own LoopGateNode precedent"

key-files:
  created:
    - tests/integration/e2e_approval_gate_test.rs
    - tests/integration/multi_parley_suspension_test.rs
    - tests/integration/parley_resume_stress_test.rs
  modified:
    - Cargo.toml

key-decisions:
  - "Every test in all three files drives a real SqliteWaypointStore on a temp file (never InMemoryWaypointStore), even where D-28 only strictly requires it for the cross-process cases -- a 'fresh store handle' read is only a meaningful durability proof against a real on-disk backend, and this plan's own must-haves repeatedly require asserting from the persisted Waypoint rather than the engine's return value"
  - "The cross-thread-leakage test (parley_resume_stress_test.rs) submits a response naming thread 0's parley id against thread 1 IN THE SAME concurrent batch as two correct resumes, rather than as a separate sequential probe -- proving the UnknownParleyId rejection holds under real concurrent access to the shared backend, not just in isolation"
  - "Task commits intentionally kept the single-line Cargo.toml [[test]] addition for each task scoped to that task's own commit (rather than adding all three [[test]] entries in one edit) so each commit is independently buildable and self-contained, per the atomic per-task commit protocol"

patterns-established: []

requirements-completed: [HITL-01, HITL-02]

coverage:
  - id: D1
    description: "E2E-2 (approval gate, both branches) passes across a real process-drop simulation over a shared on-disk Waypoint store; a fresh engine instance holds no leftover resource for a suspended thread; the graph is expressed as exactly one NodeSpec::Gate plus two Contains edges"
    requirement: "HITL-01, HITL-02"
    verification:
      - kind: integration
        ref: "tests/integration/e2e_approval_gate_test.rs#e2e2_approval_branch_survives_process_drop"
        status: pass
      - kind: integration
        ref: "tests/integration/e2e_approval_gate_test.rs#e2e2_denial_branch_survives_process_drop"
        status: pass
      - kind: integration
        ref: "tests/integration/e2e_approval_gate_test.rs#e2e2_suspended_thread_holds_no_engine_resources"
        status: pass
      - kind: integration
        ref: "tests/integration/e2e_approval_gate_test.rs#e2e2_graph_is_three_lines_of_graph"
        status: pass
    human_judgment: false
  - id: D2
    description: "A superstep in which two nodes parley (one Gate, one Function raising NextStep::Parley directly) persists exactly one AwaitingInput Waypoint with both requests and zero responses; answering one keeps the thread suspended with one persisted response; answering the second continues the run; parley order is stable by node id; a partial answer survives a process drop -- every assertion read through WaypointPort"
    requirement: "HITL-01"
    verification:
      - kind: integration
        ref: "tests/integration/multi_parley_suspension_test.rs#two_parleys_persist_as_one_waypoint_with_two_requests"
        status: pass
      - kind: integration
        ref: "tests/integration/multi_parley_suspension_test.rs#answering_one_of_two_keeps_the_thread_suspended"
        status: pass
      - kind: integration
        ref: "tests/integration/multi_parley_suspension_test.rs#answering_the_second_continues_the_run"
        status: pass
      - kind: integration
        ref: "tests/integration/multi_parley_suspension_test.rs#multi_parley_list_order_is_stable_by_node_id"
        status: pass
      - kind: integration
        ref: "tests/integration/multi_parley_suspension_test.rs#multi_parley_survives_process_drop_mid_partial"
        status: pass
    human_judgment: false
  - id: D3
    description: "Ten suspended threads resumed concurrently on a multi_thread runtime all reach RunOutcome::Completed with an exact count of ten, zero failures, zero still-suspended, no cross-thread response leakage (a parley id from one thread is UnknownParleyId against another), under an explicit timeout guard"
    requirement: "HITL-02"
    verification:
      - kind: integration
        ref: "tests/integration/parley_resume_stress_test.rs#ten_suspended_threads_resume_concurrently"
        status: pass
      - kind: integration
        ref: "tests/integration/parley_resume_stress_test.rs#concurrent_resumes_do_not_leak_responses_across_threads"
        status: pass
      - kind: integration
        ref: "tests/integration/parley_resume_stress_test.rs#stress_run_completes_within_the_timeout_guard"
        status: pass
    human_judgment: false

duration: ~55min
completed: 2026-09-05
status: complete
---

# Phase 24 Plan 05: Parley Program-Level Acceptance Evidence Summary

**Three new integration-test binaries prove the parley pause/resume spine (plans 24-01..24-04) at the program level: E2E-2's approval gate across a real process drop/recreate, multi-parley suspension asserted from the persisted Waypoint alone, and the X-05 ten-thread concurrency stress with exact counts and a timeout guard -- nothing new was implemented, every test is acceptance evidence for HITL-01/HITL-02.**

## Performance

- **Duration:** ~55 min
- **Tasks:** 3 (all `type="auto" tdd="true"`, no checkpoints)
- **Files modified:** 1 (`Cargo.toml`), 3 created

## Accomplishments

- `tests/integration/e2e_approval_gate_test.rs` (E2E-2, `.project/v0.10.0/00-program-overview.md`
  section 6): builds the approval gate exactly as the PRD promises -- one `NodeSpec::Gate` with
  `kind: Approval`, `output_field: "approved"`, plus a `Contains("\"approved\":true")` edge to an
  `act` node and a `Contains("\"approved\":false")` edge to a `cancel` node. Four tests: both
  branches (`approved`/`denied`) route correctly across a simulated process drop (engine instance
  A suspends and is dropped; a brand-new engine instance B, constructed over the same
  `SqliteWaypointStore` file, delivers the response and drives the run to `Completed`); a suspended
  thread holds no leftover task/timer/connection (confirmed by re-reading the Waypoint count from
  the SAME store handle after a 200ms wait, then again from a brand-new store handle after instance
  A is dropped -- both read exactly one `AwaitingInput` Waypoint, never a stray second write); and
  a structural test asserting the graph really is "three lines of graph" (one `Gate` node, exactly
  two `Contains` edges).
- `tests/integration/multi_parley_suspension_test.rs` (D-11's must-have): a graph with two
  independent, terminal entry nodes that both parley in the SAME first superstep -- `gate1` (a
  `NodeSpec::Gate`) and `func1` (a plain `Function` node raising `NextStep::Parley` directly,
  covering the second of the two raise paths D-07/D-08 describe). Five tests assert every state
  transition by re-reading `WaypointPort::latest`, never the engine's own return value: the
  suspending Waypoint carries exactly one `AwaitingInput` status with both requests and zero
  responses; answering one leaves two requests / one response persisted and the returned outcome
  names exactly the one remaining request; answering the second reaches `Completed` with both
  nodes' effects in the final Battlefield; the raised parley order is stable (node-id order) across
  two independent runs of the identical graph; and a partial answer survives a full engine/store
  drop and rebuild between the two answers.
- `tests/integration/parley_resume_stress_test.rs` (X-05, acceptance criterion 7 -- confirmed by
  `24-PATTERNS.md` as a genuine no-analog in this tree): ten independent threads, each with a
  single always-parleying `Function` node, suspended sequentially over one shared on-disk
  `SqliteWaypointStore` file, then resumed CONCURRENTLY via real `tokio::spawn` calls on a
  `#[tokio::test(flavor = "multi_thread")]` runtime, joined through `futures::future::join_all`.
  Three tests: all ten reach `Completed` with an exact completion count of ten and each thread's
  final state carries exactly its own submitted value; a dedicated cross-thread-isolation scenario
  submits a response naming thread 0's parley id against thread 1 IN THE SAME concurrent batch as
  two correct resumes, asserting the spoofed submission fails `EngineError::UnknownParleyId`
  (naming the SPOOFED id, not thread 1's own), thread 1 remains untouched and suspended, and a
  later correct resume of thread 1 carries only its own value; and the whole ten-thread scenario is
  wrapped in an explicit `tokio::time::timeout` proving a deadlocked resume would fail the test
  fast rather than hang CI.

## Task Commits

1. **Task 1: E2E-2 -- approval gate, both branches, across a process drop and recreate**
   - `1cee2733` -- `test(24-05): E2E-2 approval gate integration test across a process drop`
2. **Task 2: Multi-parley suspension asserted from the persisted Waypoint alone**
   - `66222b34` -- `test(24-05): multi-parley suspension asserted from the persisted Waypoint alone`
3. **Task 3: X-05 concurrency stress -- ten suspended threads resumed at once**
   - `9aa5e42b` -- `test(24-05): X-05 concurrency stress -- ten suspended threads resumed at once`

**Plan metadata:** (this commit)

No RED/GREEN split: this plan writes integration tests against an already-fully-implemented
engine (plans 24-01..24-04) rather than driving new production code through TDD -- each task's own
`<verify>` (`cargo test --test <name>`) passing on the first real run IS the task's completion
signal, mirroring plan 24-01's own precedent for a pure-evidence task.

## Files Created/Modified

- `tests/integration/e2e_approval_gate_test.rs` -- new: E2E-2, four tests, a local
  `FixedOutputNode` test double (mirrors `e2e_crash_resume_test.rs`'s own `LoopGateNode` precedent
  since the crate's `pub(crate)` `test_support.rs` fixtures are not visible to an external
  integration-test binary).
- `tests/integration/multi_parley_suspension_test.rs` -- new: five tests, a local
  `ParleyingFunctionNode` test double raising `NextStep::Parley` directly.
- `tests/integration/parley_resume_stress_test.rs` -- new: three tests, a shared
  `run_ten_concurrent_resumes` helper reused by both the primary stress test and the timeout-guard
  test, plus a dedicated 3-thread cross-isolation scenario.
- `Cargo.toml` -- three new `[[test]]` entries (`e2e_approval_gate`, `multi_parley_suspension`,
  `parley_resume_stress`), each added in its own task's commit, following the existing
  `e2e_crash_resume` entry's exact two-key shape (no `required-features`).

## Decisions Made

- **SqliteWaypointStore everywhere, never InMemoryWaypointStore**, even for tests that do not
  strictly require cross-process simulation (D-28 only names SQLite as required for the
  cross-process case). A "read from a fresh store handle" durability assertion is only meaningful
  against a real on-disk backend; this plan's own must-haves require reading from the persisted
  Waypoint repeatedly, so using the same backend class throughout keeps every assertion consistent
  and load-bearing.
- **The cross-thread-leakage probe runs concurrently with two correct resumes**, not as an isolated
  sequential check -- this is what actually exercises T-24-18 (Spoofing: cross-thread response
  delivery under concurrency) rather than merely proving the validation rule holds when called
  alone.
- **Local `StateNode` test doubles per file**, not a shared new test-support module: each of the
  three files' fixtures (`FixedOutputNode`, `ParleyingFunctionNode` x2) are small, single-purpose,
  and file-scoped -- adding a new shared cross-test-binary helper module was out of this plan's own
  file scope (`<files>` names only the three test files and `Cargo.toml`) and every existing E2E
  integration test in this tree (`e2e_crash_resume_test.rs`) already establishes this exact
  per-file-local-fixture precedent.

## Deviations from Plan

None -- plan executed exactly as written. Every task's `<verify>` command passed on its first real
run against the already-complete engine from plans 24-01..24-04; no defect was found in the
suspend/resume/validation spine (an X-03 stop-and-flag condition, per this plan's own objective,
did not trigger).

## Issues Encountered

- **Rust's deref-coercion + `pub(crate)` module boundary.** `crates/paladin-battalion/src/engine/
  test_support.rs` (`CountingFunctionNode`, etc.) is declared `pub(crate) mod test_support`
  (`engine/mod.rs:56`) -- invisible to an external integration-test binary regardless of the plan's
  own "Tier 1: ... `CountingFunctionNode` and the existing test-support fixtures only" phrasing.
  Resolved by following `tests/integration/e2e_crash_resume_test.rs`'s own established precedent:
  each file declares its own small, local `StateNode` test double instead. Not a deviation from the
  plan's INTENT (the same Tier-1 fixture *class* -- deterministic, in-tree, no new dependency -- is
  used), just from the specific named type, which was never actually reachable from `tests/`.

## User Setup Required

None -- no external service configuration required.

## Verification Evidence

- `cargo test --test e2e_approval_gate` -- 30 passed (4 new + 26 shared `tests/helpers/` fixtures
  compiled into the same binary), 0 failed.
- `cargo test --test multi_parley_suspension` -- 31 passed, 0 failed.
- `cargo test --test parley_resume_stress` -- 29 passed, 0 failed.
- `cargo test --workspace --no-fail-fast` -- exit code 0, full suite green (including every
  pre-existing test and doc-test across all ten workspace crates); the `e2e_1_crash_resume` timing
  flake documented in plans 24-02/24-03/24-04's own SUMMARYs did not reproduce on this run.
- `cargo fmt --check` -- clean (after running `cargo fmt` on the three new files once, to match
  this workspace's formatting).
- `cargo clippy --workspace --all-targets -- -D warnings` -- clean, zero warnings across the full
  workspace including the three new test targets.

## Note on REQUIREMENTS.md

This plan is the LAST plan for both requirements per every prior plan's own coverage table
(`24-01-SUMMARY.md`, `24-02-SUMMARY.md`, `24-03-SUMMARY.md`, `24-04-SUMMARY.md` all note HITL-01
needs plans 01/02/03/05 and HITL-02 needs plans 01/03/04/05, deferring the final
`requirements.mark-complete` call to whichever plan lands last). `gsd_run query
requirements.mark-complete HITL-01 HITL-02` was run as part of this plan and flipped both
requirement checkboxes and their traceability-table rows to complete in `.planning/REQUIREMENTS.md`.

## Next Phase Readiness

- HITL-01 and HITL-02 are now fully proven at both the unit level (plans 24-01..24-04) and the
  program-acceptance level (this plan): E2E-2 passes, multi-parley partial answers are durable and
  queryable, and the X-05 concurrency stress meets acceptance criterion 7 with exact counts.
- No blockers. Nothing in this plan's own scope touches Chronicle (HITL-03), graceful shutdown
  (HITL-04) or the HTTP surface (HITL-05) -- those remain for later waves of this phase per
  `24-CONTEXT.md`'s plan/wave decomposition.
- This wave's tests are additive-only (`tests/integration/*.rs` + three `Cargo.toml` `[[test]]`
  entries): no file overlap with the concurrently-running plan 24-06 (core/ports/battalion/
  storage/services), confirmed by this plan's own `<parallel_execution>` scope statement.

## Self-Check: PASSED

All 3 created files verified present on disk; all 3 commit hashes (`1cee2733`, `66222b34`,
`9aa5e42b`) verified present in `git log --oneline --all`.

---
*Phase: 24-pause-resume-history-graceful-shutdown*
*Completed: 2026-09-05*
