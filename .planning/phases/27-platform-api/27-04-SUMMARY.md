---
phase: 27-platform-api
plan: 04
subsystem: api
tags: [tokio, worker-pool, lease-heartbeat, waypoint-dispatch, shutdown-drain, in-memory-queue]

requires:
  - phase: 27-platform-api (plan 01)
    provides: "RunWorkerPool's single-branch start-only dispatch, RunRepositoryPort/RunQueuePort/AssistantResolver contracts, InMemory adapters"
  - phase: 27-platform-api (plan 02)
    provides: "SQL run persistence (already merged into this plan's base, not touched by this plan)"
provides:
  - "LeaseHeartbeat extending a dequeued run's lease every lease/4 (D-10), dropped the instant the engine call returns"
  - "WorkerDispatch::decide -- the pure Waypoint-presence + pending-responses dispatch decision (D-09): start / resume / resume_with"
  - "map_outcome -- every RunOutcome mapped to a status transition, including the Halted/Cancelled/shutdown-drain vocabulary split (D-16, D-22, D-13)"
  - "RunWorkerPool::run_once CAS-transitioning by the run's CURRENT status on dequeue (Queued, AwaitingInput, Running-redelivery, terminal-stale) before ever touching the engine (D-07)"
  - "RunWorkerPool::spawn -- concurrency worker tasks registered with a ShutdownCoordinator, draining (not dropping) on shutdown"
  - "Runnable::Agent executed through an optional wired PaladinPort instead of always nacking"
  - "worker_tests.rs -- the InMemory twin of PRD 06 acceptance 2: kill-mid-run redelivery, AwaitingInput ack, resume-with-pending-responses, heartbeat cadence, shutdown drain"
affects: [27-05, 27-06, 27-07, 27-08, 27-09, 27-17]

tech-stack:
  added: []
  patterns:
    - "PausingAfterSuperstep: a WaypointPort test wrapper that performs the real save() then parks forever inside it once a target superstep persists -- makes 'kill worker A right after superstep N' exact rather than racy, since no subsequent code path (including spawning the next superstep's node task) has run yet when the test aborts the outer future"
    - "Local, per-test-module StateNode doubles (DelayedCountingNode) rather than paladin-battalion's own CountingFunctionNode, which is pub(crate) and unreachable from the facade crate -- mirrors tracer_e2e.rs's own AlwaysFailingNode precedent"
    - "RunWorkerPool::new gains an explicit waypoint_port: Arc<W> parameter (the pool's own handle to decide dispatch) since WarEngine exposes no accessor for its own internal copy"

key-files:
  created:
    - src/application/services/run/worker_tests.rs
  modified:
    - src/application/services/run/worker.rs
    - src/application/services/run/mod.rs
    - src/application/services/run/tracer_e2e.rs

key-decisions:
  - "RunWorkerPool::new's signature grew a waypoint_port: Arc<W> parameter (inserted right after engine) rather than adding a getter to WarEngine in paladin-battalion. WarEngine exposes no accessor for its own internal WaypointPort handle, and the worker's dispatch decision (D-09) requires querying `latest(&thread)` independently of the engine's own internal call. Extending paladin-battalion (a file outside this plan's scope, and one three sibling worktrees in the same wave depend on) was judged riskier than a one-parameter, backward-compatible-in-spirit addition to this plan's own constructor. tracer_e2e.rs (27-01, #[cfg(test)]-only) was updated to pass a cloned Arc -- documented as a Rule 3 (blocking) deviation below."
  - "PaladinPort is wired via an optional with_paladin_port(...) builder rather than a required constructor parameter, so RunWorkerPool::new's arity did not also grow for the Agent-kind path -- an unwired pool still nacks an Agent-kind run for redelivery exactly as 27-01 did, now documented as the explicit fallback rather than the only path."
  - "ShutdownCoordinator defaults to a fresh, never-shared instance inside RunWorkerPool::new, with a with_shutdown_coordinator(...) builder to share the process's real coordinator -- keeps the tracer's own construction call (which never spawns workers or drains) unchanged while still letting 27-04's own tests (and, later, 27-17's wiring) share one coordinator across the whole process."
  - "The kill-mid-run test does not abort a live engine future blindly (the approach `tests/integration/e2e_crash_resume_test.rs` explicitly documents as unsafe, since a spawned node task for the NEXT superstep is not transitively cancelled by aborting the outer future). Instead, a PausingAfterSuperstep WaypointPort wrapper performs the real save() for the target superstep and then parks forever inside that same call -- the abort therefore always lands before any subsequent superstep's node task is ever spawned, eliminating the stray-task race entirely rather than relying on timing margins."

patterns-established:
  - "A WaypointPort wrapper that pauses inside save() (rather than racing a poll against an abort) is the house pattern for deterministic 'observe a durable checkpoint, then kill' tests in this facade -- reusable by 27-08's redelivery/attempt tests without re-deriving the technique."

requirements-completed: [PLAT-02, PLAT-03]

coverage:
  - id: D1
    description: "A worker extends its lease every lease/4 while a run executes and stops the moment the run returns (D-10)"
    requirement: "PLAT-02"
    verification:
      - kind: unit
        ref: "src/application/services/run/worker.rs#tests::lease_heartbeat_extends_at_lease_over_four_and_stops_after_drop"
        status: pass
      - kind: integration
        ref: "src/application/services/run/worker_tests.rs#heartbeat_extends_at_lease_over_four"
        status: pass
    human_judgment: false
  - id: D2
    description: "A run redelivered after its worker died is RESUMED from the thread's latest Waypoint (never restarted): a second worker completes it and no node executes beyond the interrupted superstep (PLAT-FR-03, D-09)"
    requirement: "PLAT-02"
    verification:
      - kind: integration
        ref: "src/application/services/run/worker_tests.rs#worker_pool_lease_expiry_exactly_once"
        status: pass
    human_judgment: false
  - id: D3
    description: "The worker has one entry point that branches on the thread's latest Waypoint: absent -> start; present with pending responses -> resume_with; present otherwise -> resume (D-09)"
    requirement: "PLAT-02"
    verification:
      - kind: unit
        ref: "src/application/services/run/worker.rs#tests::decide_returns_start_when_no_waypoint_exists, decide_returns_resume_when_waypoint_exists_and_no_pending_responses, decide_returns_resume_with_when_waypoint_exists_and_responses_are_pending"
        status: pass
      - kind: integration
        ref: "src/application/services/run/worker_tests.rs#resume_dispatch_uses_pending_responses"
        status: pass
    human_judgment: false
  - id: D4
    description: "RunOutcome::AwaitingInput releases the worker by ACKing the queue message and recording AwaitingInput; queue depth returns to 0 while the run sits suspended (D-22)"
    requirement: "PLAT-03"
    verification:
      - kind: unit
        ref: "src/application/services/run/worker.rs#tests::map_outcome_awaiting_input_transitions_and_acks"
        status: pass
      - kind: integration
        ref: "src/application/services/run/worker_tests.rs#awaiting_input_acks_queue"
        status: pass
    human_judgment: false
  - id: D5
    description: "Redelivery and resume share one attempt counter on the run row (D-23)"
    requirement: "PLAT-02"
    verification:
      - kind: integration
        ref: "src/application/services/run/worker_tests.rs#resume_dispatch_uses_pending_responses"
        status: pass
    human_judgment: false
  - id: D6
    description: "On shutdown a worker stops dequeuing, lets its in-flight run reach a superstep boundary, leaves the run Running with its message NACKed for immediate redelivery, and exits -- draining, not dropping (D-13)"
    requirement: "PLAT-02"
    verification:
      - kind: unit
        ref: "src/application/services/run/worker.rs#tests::map_outcome_halted_while_shutting_down_leaves_running_and_requeues, map_outcome_halted_with_cancel_requested_transitions_to_cancelled, map_outcome_halted_otherwise_transitions_to_halted"
        status: pass
      - kind: integration
        ref: "src/application/services/run/worker_tests.rs#shutdown_drains_in_flight_run"
        status: pass
    human_judgment: false

duration: ~31min
completed: 2026-09-08
status: complete
---

# Phase 27 Plan 04: Durable Worker Pool Summary

**`RunWorkerPool` turned into the durable worker pool PLAT-FR-02/03/06 describe: `lease/4` heartbeats, resume-not-restart dispatch on redelivery, one shared `attempt` counter, `AwaitingInput`-releases-by-ACK, and drain-on-shutdown through the existing `ShutdownCoordinator` -- proven against an InMemory twin of PRD 06 acceptance 2 with a deterministic (non-racy) kill-mid-run test.**

## Performance

- **Duration:** ~31 min
- **Started:** 2026-09-08T03:22:43Z
- **Completed:** 2026-09-08T03:53:35Z
- **Tasks:** 2 (both `type="auto" tdd="true"`)
- **Files modified:** 4 (1 created, 3 modified)

## Accomplishments

- `LeaseHeartbeat` extends a dequeued run's lease every `lease / 4` for as long as it is held, constructed immediately before an engine call and dropped immediately after — proven both by a unit test with a fake `RunQueuePort` and by an integration test with a `WarEngine`-driven slow node (≥8 extensions over a 1s run with a 400ms lease, in both).
- `WorkerDispatch::decide` is the pure, unit-tested D-09 dispatch decision: no Waypoint → `start`; Waypoint + pending responses → `resume_with`; Waypoint alone → `resume`.
- `map_outcome` is the one place a `RunOutcome` becomes a status transition (or, for a shutdown-halted run, a requeue instead), covering all four `RunOutcome` variants and all three `Halted` sub-cases (cancel-requested → `Cancelled`; shutting-down → leave `Running` + `nack(ZERO)`; otherwise → `Halted`) — six dedicated unit tests.
- `run_once` now branches on the run's *current* repository status before ever touching the engine (D-07): `Queued`/`AwaitingInput` CAS to `Running`; a `Running` redelivery bumps `attempt` with no status change; any terminal status acks and drops the stale message untouched.
- `RunWorkerPool::spawn(options)` starts `concurrency` tasks, each registered with the pool's `ShutdownCoordinator`; on cancellation each task finishes its current `run_once` iteration and drops its `RunGuard`, letting `cancel_and_wait` observe a true drain.
- `Runnable::Agent` now executes through an optional, explicitly wired `PaladinPort` (`with_paladin_port`) rather than always nacking — an unwired pool preserves 27-01's exact fallback behavior.
- `worker_tests.rs` proves acceptance 2's whole assertion set locally on the InMemory queue: a killed worker's run is resumed by a second worker with zero node re-execution, `AwaitingInput` releases the worker, resume-with-pending-responses shares the `attempt` counter, heartbeat cadence holds under a real engine run, and shutdown drains an in-flight run leaving it `Running` and immediately redeliverable.

## Task Commits

Each task was committed atomically:

1. **Task 1: Heartbeat, dispatch-by-Waypoint, outcome mapping and drain in `RunWorkerPool`** - `8ce2802a` (feat)
2. **Task 2: Kill-mid-run redelivery, `AwaitingInput` ack, and drain — the InMemory twin of acceptance 2** - `cdcc4853` (test)

**Plan metadata:** this file's own commit (docs: complete plan) — committed alongside this SUMMARY per worktree execution mode.

_TDD note: both tasks carry `tdd="true"`. Per-task tests were written and passing before each commit; no separate RED-then-GREEN commit pair was produced (test + implementation landed together per task, consistent with 27-01/27-02's documented convention for this worktree)._

## Files Created/Modified

- `src/application/services/run/worker.rs` — `RunWorkerOptions`, `LeaseHeartbeat`, `WorkerDispatch` (+`decide`), `OutcomeAction`/`map_outcome`, `RunWorkerPool` (new `waypoint_port` field, `with_shutdown_coordinator`, `with_paladin_port`, `spawn`), reworked `run_once`, new `run_agent`/`record_engine_failure` helpers, 10 unit tests.
- `src/application/services/run/mod.rs` — re-exports `LeaseHeartbeat`, `RunWorkerOptions`, `WorkerDispatch`; declares `#[cfg(test)] mod worker_tests;`.
- `src/application/services/run/tracer_e2e.rs` — passes the new `waypoint_port` argument to `RunWorkerPool::new` (a cloned `Arc<InMemoryWaypointStore>`, taken before the original is moved into `WarEngine::new`).
- `src/application/services/run/worker_tests.rs` — `UnusedPaladinPort`, `DelayedCountingNode`, `PausingAfterSuperstep`, `build_chain_graph`/`build_gate_graph`/`submit` harness helpers, and the five named tests.

## Decisions Made

See `key-decisions` in frontmatter. In prose:

1. **`RunWorkerPool::new` grew a `waypoint_port: Arc<W>` parameter.** D-09's dispatch decision needs the pool to query `latest(&thread)` independently of the engine, but `WarEngine` exposes no accessor for its own internal `WaypointPort` handle, and adding one would mean editing `crates/paladin-battalion/src/engine/mod.rs` — a file outside this plan's stated scope and one three sibling worktrees in the same wave (27-03, 27-05, 27-06) do not expect touched. A one-parameter addition to this plan's own constructor, with the caller supplying the SAME store handed to `WarEngine::new`, was the minimal, contained fix. `tracer_e2e.rs`'s harness (27-01, `#[cfg(test)]`-only) needed a one-line update to pass a cloned `Arc` — verified compiling and all four of its own tests still passing.
2. **`PaladinPort` is optional, wired via a builder.** Keeps `RunWorkerPool::new`'s arity from growing a second time for a code path (`Runnable::Agent`) no test in this plan or 27-01 exercises; an unwired pool nacks exactly as before.
3. **`ShutdownCoordinator` defaults fresh, shareable via a builder.** `tracer_e2e.rs` never spawns workers or drains, so its construction call is untouched; `worker_tests.rs`'s `shutdown_drains_in_flight_run` shares one explicit coordinator between the engine's own cancellation token and the pool.
4. **Kill-mid-run is deterministic, not timing-based.** `tests/integration/e2e_crash_resume_test.rs`'s own module docs explain why aborting a live `WarEngine::start` future is unsafe: a superstep's spawned node tasks are independent of the outer future and are NOT transitively cancelled by aborting it, so a stray task could keep running and corrupt counts. `PausingAfterSuperstep` sidesteps this by performing the real Waypoint `save()` for the target superstep and then parking forever *inside that same call* — the test's `.abort()` therefore always lands before the engine's superstep loop has spawned any node task for the next superstep, so there is no stray task to reason about at all, and the "every node runs exactly once" assertion is exact rather than a timing bet.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `RunWorkerPool::new` needed a direct `WaypointPort` handle for dispatch, which `WarEngine` does not expose**
- **Found during:** Task 1, while implementing `WorkerDispatch`'s consumption in `run_once`
- **Issue:** D-09's dispatch decision requires calling `latest(&thread)` on the SAME `WaypointPort` the engine was constructed over, but `WarEngine<W>` keeps its `waypoint_port: Arc<W>` field private with no accessor, and adding one is an edit to `crates/paladin-battalion` — a crate outside this plan's declared `files_modified` and shared by three concurrently-running sibling worktrees.
- **Fix:** Added a `waypoint_port: Arc<W>` parameter to `RunWorkerPool::new` (the pool's own handle, constructed from the same value the caller hands to `WarEngine::new`), and updated `tracer_e2e.rs`'s one call site (`#[cfg(test)]`-only, 27-01) to pass a cloned `Arc` before the original is moved into the engine.
- **Files modified:** `src/application/services/run/worker.rs`, `src/application/services/run/tracer_e2e.rs`
- **Verification:** `cargo test -p paladin-ai --lib services::run` — 28 passed, including all four pre-existing `tracer_e2e` tests unchanged in behavior.
- **Committed in:** `8ce2802a` (Task 1 commit)

**2. [Rule 1 - Bug] `NodeSpec::gate`'s `output_field` needed a schema default for the field's type to be inferable**
- **Found during:** Task 2, first `cargo test` run of `awaiting_input_acks_queue`
- **Issue:** The gate fixture's `FieldSpec` for `"approved"` declared `None` as its default; the engine's own validation rejected this with "the field declares no schema default, so its type cannot be inferred" — the run ended `Failed` rather than suspending `AwaitingInput`.
- **Fix:** Declared `Some(serde_json::json!(false))` as the field's default, matching `ParleyKind::Approval`'s boolean shape.
- **Files modified:** `src/application/services/run/worker_tests.rs`
- **Verification:** `awaiting_input_acks_queue` and `resume_dispatch_uses_pending_responses` (which reuses the same fixture) both pass.
- **Committed in:** `cdcc4853` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (1 Rule 3 blocking constructor-shape fix, 1 Rule 1 test-fixture bug)
**Impact on plan:** Both were necessary to reach a compiling, passing state exactly matching the plan's own acceptance criteria; neither changed the plan's architecture or scope beyond the one documented constructor-parameter addition.

## Issues Encountered

None beyond the two auto-fixed deviations above — both were caught and resolved during the first `cargo test`/`cargo build` pass for their respective task, before any commit.

## User Setup Required

None — no external service configuration required. Everything in this plan runs against InMemory adapters with no Docker dependency (D-51, Tier 1).

## Next Phase Readiness

- `RunWorkerPool` is now the full durable execution engine PRD 06 §2.2 describes: heartbeats, resume-not-restart dispatch, shared `attempt`, ACK-on-suspend, and drain — ready for 27-05's cancellation-probe wiring (`with_cancellation_probe`/cross-instance cancel observation sits directly on top of the `map_outcome` cancel/shutdown split this plan established) and 27-17's config-driven wiring (`RunWorkerOptions` is already the shape `RunWorkerConfig` converts into).
- `RunWorkerPool::new`'s grown constructor (`waypoint_port` parameter) and the new `with_shutdown_coordinator`/`with_paladin_port` builders are the only public-surface additions; both are additive (no existing call site signature removed), and `tracer_e2e.rs` (27-01) continues to pass unchanged in behavior with the one-line update.
- The `PausingAfterSuperstep` WaypointPort-wrapper technique (deterministic "kill right after superstep N persists") is reusable by 27-08's redelivery/attempt-overflow tests without re-deriving it.
- No blockers. `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo check --workspace --all-targets --all-features` all pass clean on the final commit; `cargo doc -p paladin-ai --no-deps` introduces no new warnings (the 4 pre-existing warnings in `paladin_execution_service.rs`, `parley/adapter.rs`, `config/agent_runtime.rs`, `presets/mod.rs` are unchanged by this plan).

## Self-Check: PASSED

**Files verified to exist:**
- FOUND: `src/application/services/run/worker.rs`
- FOUND: `src/application/services/run/worker_tests.rs`
- FOUND: `src/application/services/run/mod.rs`
- FOUND: `src/application/services/run/tracer_e2e.rs`

**Commits verified to exist (git log --oneline):**
- FOUND: `8ce2802a` feat(27-04): add lease heartbeat, Waypoint dispatch and outcome mapping to RunWorkerPool
- FOUND: `cdcc4853` test(27-04): add InMemory twin of acceptance 2 -- kill-mid-run redelivery, AwaitingInput ack, heartbeat, shutdown drain

**Verification commands re-run and confirmed passing:**
- `cargo test -p paladin-ai --lib services::run::worker` → `test result: ok. 15 passed`
- `cargo test -p paladin-ai --lib services::run::worker_tests` → `test result: ok. 5 passed`
- `cargo test -p paladin-ai --lib services::run` → `test result: ok. 28 passed` (includes 27-01's resolver/submission/tracer_e2e tests, unaffected)
- `grep -c 'pub struct LeaseHeartbeat' src/application/services/run/worker.rs` → `1`
- `grep -c 'lease / 4\|lease.checked_div(4)\|/ 4' src/application/services/run/worker.rs` → `4`
- `grep -c 'resume_with(' src/application/services/run/worker.rs` → `1`; `grep -c '\.resume(' src/application/services/run/worker.rs` → `1`
- `grep -c 'coordinator.register()' src/application/services/run/worker.rs` → `1`
- `grep -v '^\s*//' src/application/services/run/worker.rs | grep -cE 'paladin_web|axum'` → `0`
- `grep -c 'fn worker_pool_lease_expiry_exactly_once' src/application/services/run/worker_tests.rs` → `1`
- `grep -c 'fn awaiting_input_acks_queue' src/application/services/run/worker_tests.rs` → `1`
- `grep -c 'flavor = "multi_thread"' src/application/services/run/worker_tests.rs` → `3`
- `grep -c 'tokio::time::timeout' src/application/services/run/worker_tests.rs` → `4`
- `cargo fmt --all --check` → clean
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` → clean
- `cargo check --workspace --all-targets --all-features` → exit 0 (full workspace)
- `cargo doc -p paladin-ai --no-deps` → no new warnings (4 pre-existing, unrelated warnings unchanged)
- `git diff --stat Cargo.lock` → empty (no new dependency)

---
*Phase: 27-platform-api*
*Completed: 2026-09-08*
