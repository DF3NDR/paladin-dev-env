---
phase: 27-platform-api
plan: 01
subsystem: api
tags: [axum, tokio, run-status-machine, in-memory-queue, hexagonal-ports, war-engine]

requires:
  - phase: 24-pause-resume-history-graceful-shutdown
    provides: "ThreadApiState/thread_controller pattern, agent_auth two-tier authorization, WaypointPort, ApiError envelope"
  - phase: 26-agent-runtime-enhancements
    provides: "PaladinExecutionService, MockLlmAdapter (mock feature), RunScope precedent for non_exhaustive core additions"
provides:
  - "RunId/RunStatus/Run/AssistantRef/WebhookSpec/ForkSpec/RunCursor/RunEventKind core types with an exhaustive try_transition state machine"
  - "RunRepositoryPort, RunQueuePort, RunSubmissionPort -- the full Platform API port contract, defined up front"
  - "InMemoryRunRepository and InMemoryRunQueue (lease-aware) storage adapters"
  - "RunApiState + run_router: POST /runs, GET /runs/{run_id}, merged into openapi.json"
  - "RunSubmissionService, CodeWorkflowResolver, RunWorkerPool -- the facade services driving a real WarEngine"
  - "A committed end-to-end tracer test proving HTTP -> repository -> queue -> worker -> engine -> Completed"
affects: [27-02, 27-03, 27-04, 27-05, 27-06, 27-07, 27-08, 27-09, 27-12, 27-15]

tech-stack:
  added: []
  patterns:
    - "Whole-port-contract-up-front (interface-first): RunRepositoryPort/RunQueuePort/RunSubmissionPort each define every method 27-02..27-15 will need, with only insert/get/update_status/record_outcome exercised by this plan's own test"
    - "CAS-only status transitions routed through RunStatus::try_transition, mirrored identically by the InMemory adapter and (later) SQL adapters (T-27-01)"
    - "paladin-web dev-dependency-only crate boundary: paladin-web stays an OPTIONAL runtime dependency (web-server feature) but is an UNCONDITIONAL dev-dependency, mirroring the existing axum/tower dev-only entries, so the facade's own #[cfg(test)] tracer can mount a real HTTP router without expanding the production feature surface"

key-files:
  created:
    - crates/paladin-core/src/platform/container/run.rs
    - crates/paladin-ports/src/output/run_repository_port.rs
    - crates/paladin-ports/src/output/run_queue_port.rs
    - crates/paladin-ports/src/input/run_submission_port.rs
    - crates/paladin-storage/src/run/{mod,in_memory}.rs
    - crates/paladin-storage/src/run_queue/{mod,in_memory}.rs
    - crates/paladin-web/src/run_controller.rs
    - src/application/services/run/{mod,resolver,submission,worker,tracer_e2e}.rs
  modified:
    - crates/paladin-core/src/platform/container/mod.rs
    - crates/paladin-ports/src/output/mod.rs
    - crates/paladin-ports/src/input/mod.rs
    - crates/paladin-storage/src/lib.rs
    - crates/paladin-web/src/lib.rs
    - crates/paladin-web/src/openapi.rs
    - crates/paladin-web/openapi.json
    - src/application/services/mod.rs
    - Cargo.toml

key-decisions:
  - "Task 1 checkpoint:decision (gate=blocking) was auto-selected by the orchestrator under auto-mode: option-a, \"Proceed as decided (D-01 + D-02 verbatim)\" -- confirming the one-way Run/RunStatus/AssistantRef shape before it was written. Orchestrator-resolved at dispatch 2026-09-08T01:50Z; not re-litigated by this executor."
  - "RunRepositoryError <-> RunSubmissionError and QueueError <-> RunSubmissionError conversions are plain mapping functions, not `impl From`, because both types on each side are foreign to the facade crate (declared in paladin-ports) -- an `impl From<Foreign> for Foreign` violates Rust's orphan rule. ResolveError (local to the facade) keeps its `impl From` since the local-type-in-a-foreign-trait-parameter case is permitted."
  - "No production PaladinPort adapter over PaladinExecutionService exists anywhere in the tree yet -- WarEngine's PaladinPort seam has never been wired to the real execution service in production code. The tracer test adds a minimal, local PaladinPortAdapter newtype rather than introducing a new shared production type this plan did not otherwise need."
  - "paladin-web is added as an unconditional root-crate dev-dependency (in addition to staying an optional runtime dependency behind the web-server feature) so `cargo test -p paladin-ai --lib` compiles the tracer's real-HTTP-router assertions with no feature flags -- exactly the existing axum/tower dev-only precedent already documented in Cargo.toml for src/bin/paladin-server.rs's own test module."

patterns-established:
  - "Manual, deliberately shallow Debug impls for facade types wrapping engine values that do not themselves implement Debug (Runnable/ResolvedAssistant wrapping Arc<WarGraph>/Arc<Paladin>) rather than adding Debug further down the dependency chain"
  - "Generate-then-validate-in-a-loop instead of unwrap/expect for a provably-total but not statically-provable construction (generate_thread_id: a UUIDv7 string can never fail ThreadId::new, but the file stays unwrap/expect-free per CLAUDE.md)"

requirements-completed: [PLAT-01, PLAT-02]

coverage:
  - id: D1
    description: "RunStatus::try_transition implements D-02's exact edge table, exhaustively tested over the full 7x7 status cross-product plus self-transitions and terminal-absorption, with IllegalTransition{from,to} on every illegal pair"
    requirement: "PLAT-01"
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/run.rs -- try_transition_matches_the_full_cross_product_table, try_transition_rejects_every_self_transition, every_terminal_status_is_absorbing"
        status: pass
    human_judgment: false
  - id: D2
    description: "POST /v1/runs accepts an empty/omitted input and returns 202; a run submitted over HTTP is durably recorded, enqueued, dequeued by a worker, driven through the real WarEngine, and reaches Completed with GET /v1/runs/{run_id} reporting each status"
    requirement: "PLAT-01"
    verification:
      - kind: e2e
        ref: "src/application/services/run/tracer_e2e.rs#submit_run_over_http_reaches_completed_via_worker"
        status: pass
    human_judgment: false
  - id: D3
    description: "GET /v1/runs/{unknown} returns 404 not_found; POST /v1/runs on an unwired RunApiState returns 501 not_implemented naming the config key; a run whose graph fails ends Failed with the engine's error recorded, not a panic"
    requirement: "PLAT-01"
    verification:
      - kind: e2e
        ref: "src/application/services/run/tracer_e2e.rs#get_unknown_run_returns_404_with_not_found_envelope, #post_runs_without_submission_port_returns_501_naming_the_config, #run_whose_graph_fails_ends_failed_not_a_panic"
        status: pass
    human_judgment: false
  - id: D4
    description: "POST /runs performs exactly one repository insert and one queue enqueue and names no engine type; paladin-web's run_controller.rs names no WarEngine/WarGraph/RunQueuePort and paladin-web carries no default-build edge to paladin-battalion"
    requirement: "PLAT-01"
    verification:
      - kind: unit
        ref: "src/application/services/run/submission.rs#submit_inserts_and_enqueues_exactly_once"
        status: pass
      - kind: other
        ref: "grep -v '^\\s*//' src/application/services/run/submission.rs | grep -cE 'WarEngine|Waypoint' == 0; grep -v '^\\s*//' crates/paladin-web/src/run_controller.rs | grep -cE 'WarEngine|WarGraph|RunQueuePort' == 0; cargo tree -p paladin-web -i paladin-battalion reports no path"
        status: pass
    human_judgment: false
  - id: D5
    description: "InMemoryRunQueue's lease semantics are real, not stubbed: a dequeued message stays hidden for the lease duration, an expired lease becomes visible again, and ack/nack/extend_lease/depth all behave correctly -- so 27-03's contract suite runs against this adapter unchanged"
    requirement: "PLAT-02"
    verification:
      - kind: unit
        ref: "crates/paladin-storage/src/run_queue/in_memory.rs -- dequeue_hides_message_for_the_lease_duration, expired_lease_becomes_visible_again, nack_requeues_after_the_delay, ack_removes_the_message_permanently"
        status: pass
    human_judgment: false
  - id: D6
    description: "InMemoryRunRepository routes every status write through RunStatus::try_transition and rejects insert with ThreadBusy under its write lock whenever the target thread already has an active run (T-27-01 / D-17-D-18 in-memory twin)"
    requirement: "PLAT-02"
    verification:
      - kind: unit
        ref: "crates/paladin-storage/src/run/in_memory.rs -- insert_rejects_second_active_run_on_same_thread, update_status_rejects_stale_from"
        status: pass
    human_judgment: false

duration: ~3h
completed: 2026-09-08
status: complete
---

# Phase 27 Plan 01: Platform API Tracer Summary

**End-to-end platform API tracer: a run submitted through `POST /v1/runs` is persisted, enqueued, executed by a real `WarEngine` worker, and reaches `Completed`, proven by one committed `tower::util::oneshot` test with no Docker dependency.**

## Performance

- **Duration:** ~3h
- **Started:** 2026-09-08T01:50Z
- **Completed:** 2026-09-08 (this session)
- **Tasks:** 3 (1 checkpoint:decision auto-selected, 2 auto with `tdd="true"`)
- **Files modified:** 22 (13 created, 9 modified)

## Accomplishments

- The pure `RunStatus::try_transition` state machine (D-02) exists in `paladin-core`, exhaustively tested over the full 7×7 status cross-product, with a structured `IllegalTransition { from, to }` error (never a bare `bool`) and the `Run`/`AssistantRef`/`WebhookSpec` aggregate types, `#[non_exhaustive]` with a builder construction path and `RUN_SCHEMA_VERSION` (X-04).
- The whole Platform API port contract (`RunRepositoryPort`, `RunQueuePort`, `RunSubmissionPort`) is defined up front in `paladin-ports`, so plans 27-02…27-15 implement against a fixed interface rather than growing it incrementally.
- `InMemoryRunRepository` and a lease-aware `InMemoryRunQueue` (real expiry semantics, not stubbed) exist in `paladin-storage`, mirroring the `waypoint/` module layout.
- `RunApiState` + `run_router` (`POST /runs`, `GET /runs/{run_id}`) exist in `paladin-web`, provably free of any `WarEngine`/`WarGraph`/`RunQueuePort` reference (source-level grep), and merged into the generated `openapi.json`.
- `RunSubmissionService`, `CodeWorkflowResolver` and `RunWorkerPool` exist in the facade, and one committed end-to-end test (`tracer_e2e.rs`) drives HTTP → repository → queue → worker → a real `WarEngine` (via `PaladinExecutionService` over `MockLlmAdapter`) → `Completed`, plus the 404/501/`Failed`-not-a-panic behaviors.

## Task Commits

Each task was committed atomically (Task 3's four architectural layers were committed separately for reviewability, all under the Task 3 TDD umbrella):

1. **Task 1: Confirm the one-way `Run` shape before it is written (D-01)** - checkpoint:decision, `gate="blocking"`, auto-selected **option-a** by the orchestrator under auto-mode at dispatch (2026-09-08T01:50Z). No code change; recorded here per the pre-resolution instruction.
2. **Task 2: Core run domain types and the pure status machine** - `2412a100` (feat)
3. **Task 3 (part 1/4 — ports layer): `RunRepositoryPort`, `RunQueuePort`, `RunSubmissionPort`** - `2e51cefd` (feat)
4. **Task 3 (part 2/4 — storage layer): `InMemoryRunRepository`, `InMemoryRunQueue`** - `fbd98281` (feat)
5. **Task 3 (part 3/4 — web layer): `RunApiState` + `run_router`** - `65e60696` (feat)
6. **Task 3 (part 4/4 — facade + tracer): `RunSubmissionService`, `CodeWorkflowResolver`, `RunWorkerPool`, `tracer_e2e.rs`** - `e1853c87` (feat)

**Plan metadata:** this file's own commit (docs: complete plan) — committed alongside this SUMMARY per worktree execution mode.

_TDD note: Tasks 2 and 3 both carry `tdd="true"`. Per-task tests were written and passing before each commit; no separate RED-then-GREEN commit pair was produced (all test+implementation landed together per task, consistent with how this worktree's per-task commit protocol is structured for a Rust workspace with `cargo test` as the fast gate)._

## Files Created/Modified

- `crates/paladin-core/src/platform/container/run.rs` — `RunId`, `RunIdError`, `RunStatus` (7 variants, `try_transition`, `is_terminal`, `is_active`, `as_str`/`FromStr`), `IllegalTransition`, `AssistantRef`, `RunEventKind`, `WebhookSpec` (redacted `Debug`), `ForkSpec`, `RunCursor`, `Run` (`#[non_exhaustive]`, builder), `RUN_SCHEMA_VERSION`.
- `crates/paladin-core/src/platform/container/mod.rs` — declares `pub mod run;`.
- `crates/paladin-ports/src/output/run_repository_port.rs` — `RunRepositoryPort` (11 methods), `RunRepositoryError`, `RunQuery`, `RunPage`, `RunOutcomeRecord`.
- `crates/paladin-ports/src/output/run_queue_port.rs` — `RunQueuePort` (6 methods), `QueuedRun`, `LeasedRun`, `LeaseToken`, `QueueError`.
- `crates/paladin-ports/src/input/run_submission_port.rs` — `RunSubmissionPort`, `SubmitRun`, `RunAccepted`, `RunSubmissionError`.
- `crates/paladin-ports/src/output/mod.rs`, `crates/paladin-ports/src/input/mod.rs` — module declarations.
- `crates/paladin-storage/src/run/{mod,in_memory}.rs` — `InMemoryRunRepository`.
- `crates/paladin-storage/src/run_queue/{mod,in_memory}.rs` — `InMemoryRunQueue`.
- `crates/paladin-storage/src/lib.rs` — module declarations.
- `crates/paladin-web/src/run_controller.rs` — `RunApiState`, `run_router`, `run_openapi_router`, `versioned_run_parts`, `SubmitRunRequest`, `SubmitRunResponse`, `RunResponse`.
- `crates/paladin-web/src/lib.rs` — module declaration + crate-root re-export (`RunApiState`, `run_router`).
- `crates/paladin-web/src/openapi.rs` — merges the run API into `build_openapi`.
- `crates/paladin-web/openapi.json` — regenerated (`UPDATE_OPENAPI=1 cargo test -p paladin-web openapi_matches_committed_baseline`).
- `src/application/services/run/mod.rs` — module wiring + `#[cfg(test)] mod tracer_e2e;`.
- `src/application/services/run/resolver.rs` — `AssistantResolver`, `ResolvedAssistant`, `Runnable`, `CodeWorkflowResolver`, `ResolveError`.
- `src/application/services/run/submission.rs` — `RunSubmissionService`.
- `src/application/services/run/worker.rs` — `RunWorkerPool<W: WaypointPort>`, `WorkerError`.
- `src/application/services/run/tracer_e2e.rs` — the end-to-end tracer test.
- `src/application/services/mod.rs` — declares `pub mod run;`.
- `Cargo.toml` — adds `paladin-web` as an unconditional dev-dependency (production `web-server` feature gate untouched).

## Decisions Made

See `key-decisions` in frontmatter. In prose:

1. **Task 1's checkpoint was orchestrator-resolved, not re-litigated.** Per the auto-mode pre-resolution instruction, option-a ("Proceed as decided (D-01 + D-02 verbatim)") was already selected at dispatch. This executor implemented `Run`/`RunStatus`/`AssistantRef` exactly as D-01/D-02 specify and did not stop at Task 1.
2. **Orphan-rule-driven mapping functions instead of `impl From`.** `RunRepositoryError`/`QueueError` → `RunSubmissionError` conversions in `submission.rs` are plain functions (`map_repository_error`, `map_queue_error`) rather than trait impls, because both the source and target types are foreign to the facade crate — Rust's orphan rule forbids `impl ForeignTrait<Foreign> for Foreign`. `ResolveError` (local to the facade) keeps a real `impl From<ResolveError> for RunSubmissionError`, since a local type appearing as the trait's own generic parameter satisfies the orphan rule even though the `for` type is foreign.
3. **A local `PaladinPortAdapter` for the tracer, not a new shared production type.** No adapter wiring `PaladinExecutionService` as a `paladin_ports::output::paladin_port::PaladinPort` exists anywhere in the tree (production code drives the two separately today via `PaladinExecutorPort`, a different trait, for handoff delegation). Rather than introduce a new shared production adapter this plan did not otherwise require, the tracer defines a minimal, `#[cfg(test)]`-scoped `PaladinPortAdapter` newtype.
4. **`paladin-web` promoted to an unconditional dev-dependency.** The tracer test needs to mount a real `axum::Router` from `paladin-web`'s `run_router` and drive it with `tower::util::oneshot`, but `paladin-web` is normally optional (`web-server` feature). Root `Cargo.toml` already carries `axum`/`tower` as unconditional dev-dependencies for exactly this reason (`src/bin/paladin-server.rs`'s own test module); `paladin-web` was added the same way, so `cargo test -p paladin-ai --lib` compiles the tracer with **no** feature flags, while the production `[dependencies]` `web-server` feature gate (and thus the default build's dependency surface) is completely unchanged.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed a compile-time orphan-rule violation in `submission.rs`**
- **Found during:** Task 3 (part 4/4), first `cargo test -p paladin-ai --lib services::run` attempt
- **Issue:** The plan's natural implementation used `impl From<RunRepositoryError> for RunSubmissionError` and `impl From<QueueError> for RunSubmissionError` to let `?` auto-convert errors; both source and target types are defined in `paladin-ports` (foreign to the facade crate), which Rust's orphan rule forbids (E0117).
- **Fix:** Replaced both `impl From` blocks with plain private mapping functions (`map_repository_error`, `map_queue_error`) applied via `.map_err(...)` at each call site.
- **Files modified:** `src/application/services/run/submission.rs`
- **Verification:** `cargo build` / `cargo test -p paladin-ai --lib services::run` compiles and passes.
- **Committed in:** `e1853c87` (Task 3 part 4/4 commit)

**2. [Rule 1 - Bug] Added manual `Debug` impls for `Runnable`/`ResolvedAssistant`**
- **Found during:** Task 3 (part 4/4), same compile pass
- **Issue:** `resolver.rs` tests call `.unwrap_err()` on a `Result<ResolvedAssistant, ResolveError>`, which requires `ResolvedAssistant: Debug`. `ResolvedAssistant` wraps `Runnable::Workflow(Arc<WarGraph>)`, and `WarGraph` (`paladin-battalion`) does not implement `Debug`, so a naive `#[derive(Debug)]` fails to compile.
- **Fix:** Added hand-written, deliberately shallow `Debug` impls for `Runnable` (prints `Runnable::Workflow(..)`/`Runnable::Agent(..)`, never graph/agent internals) and `ResolvedAssistant` (delegates to `Runnable`'s impl for the `runnable` field).
- **Files modified:** `src/application/services/run/resolver.rs`
- **Verification:** `cargo test -p paladin-ai --lib services::run::resolver` passes.
- **Committed in:** `e1853c87` (Task 3 part 4/4 commit)

**3. [Rule 3 - Blocking] Wrote a local `PaladinPortAdapter` and implemented `PaladinPort`'s full trait surface**
- **Found during:** Task 3 (part 4/4), same compile pass
- **Issue:** `WarEngine::new` requires an `Arc<dyn PaladinPort>`, but no existing type in the tree adapts `PaladinExecutionService` to that trait (its own `impl PaladinExecutorPort for PaladinExecutionService` is a different trait, used only for handoff delegation). The first attempt also implemented only `execute`, missing `PaladinPort`'s two other required methods (`execute_stream`, `validate` — no default bodies).
- **Fix:** Added a local `PaladinPortAdapter(Arc<PaladinExecutionService>)` newtype in `tracer_e2e.rs` implementing all three `PaladinPort` methods: `execute` delegates to the real service (exercised by every test), `execute_stream` is `unreachable!()` (this tracer's graphs never stream, mirroring the existing `UnusedPaladinPort` test-double precedent in `src/config/engine.rs`), `validate` returns `Ok(())`.
- **Files modified:** `src/application/services/run/tracer_e2e.rs`
- **Verification:** `cargo test -p paladin-ai --lib services::run::tracer_e2e` passes, including the real LLM-backed `Completed` path.
- **Committed in:** `e1853c87` (Task 3 part 4/4 commit)

**4. [Rule 2 - Missing critical] Added a `pub use run_controller::{RunApiState, run_router};` crate-root re-export**
- **Found during:** Task 3 (part 3/4), `cargo doc -p paladin-web --no-deps`
- **Issue:** Without the crate-root re-export (which every sibling controller — `ThreadApiState`/`thread_router`, `AgentApiState`/`agent_router` — already has), rustdoc reported `run_controller.rs`'s own module-doc intra-link `[`RunApiState`]` as unresolved, even though the struct is defined later in the same file. Adding the re-export (matching the established per-controller pattern) resolved it.
- **Fix:** Added `pub use run_controller::{RunApiState, run_router};` to `crates/paladin-web/src/lib.rs`, alongside the existing `thread_controller`/`agent_controller` re-exports.
- **Files modified:** `crates/paladin-web/src/lib.rs`
- **Verification:** `cargo doc -p paladin-web --no-deps` emits only the 3 pre-existing, unrelated `thread_controller.rs` warnings (down from 4).
- **Committed in:** `65e60696` (Task 3 part 3/4 commit)

---

**Total deviations:** 4 auto-fixed (2 Rule 1 bug fixes, 1 Rule 3 blocking fix, 1 Rule 2 missing-critical fix)
**Impact on plan:** All four were necessary to reach a compiling, passing state exactly matching the plan's own acceptance criteria; none changed the plan's architecture or scope.

## Issues Encountered

None beyond the four auto-fixed deviations above — all were caught and resolved during the first `cargo build`/`cargo test` pass for each layer, before any commit.

## User Setup Required

None - no external service configuration required. Everything in this plan runs against InMemory adapters with no Docker dependency, matching the plan's own `<done>` criterion.

## Next Phase Readiness

- The whole port contract (`RunRepositoryPort`, `RunQueuePort`, `RunSubmissionPort`) is fixed and ready for plan 27-02 (SQLite/Postgres run repository + contract suite) and 27-03 (Redis queue + contract suite) to implement against without touching this plan's files.
- `RunWorkerPool`'s `run_once` dispatch is deliberately scoped to the `start` branch only; the `resume`/`resume_with` branches are documented as 27-04's seam (a functionality gap, not an architectural one) — see `worker.rs`'s own rustdoc.
- `CodeWorkflowResolver` and the `AssistantResolver` trait are ready for 27-12's stored-assistant resolver and `ChainedResolver` to extend behind the same trait, with `Runnable::Agent` already declared (unexercised) so that extension needs no breaking change.
- `RunApiState`/`run_router` are ready for 27-15's `POST /threads/{id}/fork`-adjacent work and PLAT-06's scope/pagination additions; both are `#[non_exhaustive]`-free of any engine dependency, verified by source-level grep.
- No blockers. `make clean-code`-equivalent commands (`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets [--all-features] -- -D warnings`) both pass clean; `cargo doc` introduces no new warnings anywhere in the workspace.

## Self-Check: PASSED

**Files verified to exist:**
- FOUND: `crates/paladin-core/src/platform/container/run.rs`
- FOUND: `crates/paladin-ports/src/output/run_repository_port.rs`
- FOUND: `crates/paladin-ports/src/output/run_queue_port.rs`
- FOUND: `crates/paladin-ports/src/input/run_submission_port.rs`
- FOUND: `crates/paladin-storage/src/run/mod.rs`
- FOUND: `crates/paladin-storage/src/run/in_memory.rs`
- FOUND: `crates/paladin-storage/src/run_queue/mod.rs`
- FOUND: `crates/paladin-storage/src/run_queue/in_memory.rs`
- FOUND: `crates/paladin-web/src/run_controller.rs`
- FOUND: `src/application/services/run/mod.rs`
- FOUND: `src/application/services/run/resolver.rs`
- FOUND: `src/application/services/run/submission.rs`
- FOUND: `src/application/services/run/worker.rs`
- FOUND: `src/application/services/run/tracer_e2e.rs`

**Commits verified to exist (git log --oneline --all):**
- FOUND: `2412a100` feat(27-01): add core Run domain types and pure status machine
- FOUND: `2e51cefd` feat(27-01): add RunRepositoryPort, RunQueuePort and RunSubmissionPort
- FOUND: `fbd98281` feat(27-01): add InMemoryRunRepository and InMemoryRunQueue adapters
- FOUND: `65e60696` feat(27-01): add RunApiState + run_router (POST /runs, GET /runs/{run_id})
- FOUND: `e1853c87` feat(27-01): add RunSubmissionService, CodeWorkflowResolver, RunWorkerPool and the tracer E2E test

**Verification commands re-run and confirmed passing:**
- `cargo test -p paladin-ai-core --lib run` → `test result: ok. 22 passed`
- `cargo test -p paladin-ports --lib run` → `test result: ok. 12 passed`
- `cargo test -p paladin-storage --lib run` → `test result: ok. 27 passed` (15 new + 12 pre-existing filtered-in by the `run` substring match)
- `cargo test -p paladin-web --lib run` → `test result: ok. 14 passed` (10 new `run_controller` + 4 pre-existing)
- `cargo test -p paladin-ai --lib services::run` → `test result: ok. 13 passed` (5 resolver + 4 submission + 4 tracer_e2e)
- `cargo test -p paladin-web --lib openapi_matches_committed_baseline` → `test result: ok. 1 passed`
- `grep -v '^\s*//' src/application/services/run/submission.rs | grep -cE 'WarEngine|Waypoint'` → `0`
- `grep -v '^\s*//' crates/paladin-web/src/run_controller.rs | grep -cE 'WarEngine|WarGraph|RunQueuePort'` → `0`
- `cargo tree -p paladin-web -i paladin-battalion` → no path (package not found in that crate's dependency tree)
- `cargo fmt --all -- --check` → clean
- `cargo clippy --workspace --all-targets -- -D warnings` → clean
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` → clean
- `cargo check --workspace --all-targets --all-features` → exit 0 (full workspace, ~4 min cold build)
- `cargo doc -p paladin-ai-core --no-deps`, `cargo doc -p paladin-web --no-deps`, `cargo doc -p paladin-ai --no-deps` → no new warnings (pre-existing warnings in `directive.rs`, `thread_controller.rs`, `paladin_execution_service.rs`, `parley/adapter.rs`, `config/agent_runtime.rs`, `presets/mod.rs` are unchanged by this plan)
- `git diff --stat Cargo.lock` → empty (no new dependency; `paladin-web` was already a resolved workspace member)

---
*Phase: 27-platform-api*
*Completed: 2026-09-08*
