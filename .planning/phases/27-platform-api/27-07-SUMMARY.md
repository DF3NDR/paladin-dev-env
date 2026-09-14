---
phase: 27-platform-api
plan: 07
subsystem: api
tags: [cancellation, war-engine, tokio-util, cross-instance, worker-pool]

requires:
  - phase: 27-platform-api (plan 01)
    provides: "RunSubmissionPort/RunSubmissionService, RunWorkerPool's start-only dispatch"
  - phase: 27-platform-api (plan 02)
    provides: "SQL run repositories (SqliteRunRepository, PostgresRunRepository) implementing request_cancel/is_cancel_requested"
  - phase: 27-platform-api (plan 04)
    provides: "RunWorkerPool's lease heartbeat, Waypoint-dispatch, map_outcome's Halted/Cancelled/shutdown-drain split"
provides:
  - "CancellationProbe port (paladin-ports) and WarEngine::with_cancellation_probe -- an infallible, debounceable seam consulted at every superstep boundary beside the existing CancellationToken"
  - "DbCancellationProbe (facade) -- a debounced CancellationProbe adapter reading RunRepositoryPort::is_cancel_requested, answering false on any backend error"
  - "LocalRunTokens -- the same-instance fast-path registry RunSubmissionService::cancel consults for CancelOutcome::was_local"
  - "RunSubmissionPort::cancel + CancelOutcome -- persisted-flag-first, best-effort-local-signal-second cancellation, idempotent on a non-terminal run"
  - "RunWorkerPool::with_engine_factory / with_cancellation_probing -- opt-in per-run engine construction carrying a child CancellationToken, additive to 27-04's shared-engine default"
affects: [27-15]

tech-stack:
  added: []
  patterns:
    - "Policy in the adapter, mechanism in the engine (D-15): the engine calls CancellationProbe::is_cancelled unconditionally, once per boundary; debouncing lives entirely in DbCancellationProbe's own per-thread cache"
    - "Per-run engine factory over a shared template engine: WarEngine has no Clone and no runtime-mutable cancellation_token, so a per-run CancellationToken is obtained by rebuilding a cheap, all-Arc-fields engine per dispatch through a caller-supplied closure (RunWorkerPool::with_engine_factory), rather than growing paladin-battalion's public surface"
    - "Two independent, non-load-bearing-on-each-other cancellation signals: the durable repository flag (cross-instance, works alone) and the in-process CancellationToken (same-instance, latency-only optimization) -- correctness never depends on the local signal firing"

key-files:
  created:
    - crates/paladin-ports/src/output/cancellation_probe.rs
    - src/application/services/run/cancel.rs
    - src/application/services/run/cancel_tests.rs
  modified:
    - crates/paladin-ports/src/output/mod.rs
    - crates/paladin-ports/src/input/run_submission_port.rs
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/src/engine/superstep.rs
    - crates/paladin-battalion/src/engine/graph.rs
    - crates/paladin-web/src/run_controller.rs
    - src/application/services/run/worker.rs
    - src/application/services/run/submission.rs
    - src/application/services/run/mod.rs

key-decisions:
  - "WarEngine has no Clone and no way to swap its cancellation_token after construction, and this task's declared file scope excludes paladin-battalion further edits beyond Task 1's additive with_cancellation_probe. Rather than add a WarEngine::with_run_token mutator (an engine-level change outside Task 2's scope), RunWorkerPool grew an OPTIONAL with_engine_factory(Arc<dyn Fn(CancellationToken) -> WarEngine<W>>) builder: run_once rebuilds a fresh, all-Arc-fields-cheap engine per dispatch when wired, carrying a CancellationToken::child_token() of the pool's own ShutdownCoordinator (so process shutdown still cascades) and registered in LocalRunTokens for the dispatch's duration. The default (no factory) path is byte-identical to 27-04: one shared engine, no local-token registration."
  - "RunWorkerPool itself calls WarEngine::with_cancellation_probe, not the caller's factory closure. A separate with_cancellation_probing(min_interval) builder constructs the pool's own DbCancellationProbe (over its own repository field) and the pool attaches it to whatever engine the factory returns, inside run_once. This keeps the factory closure minimal (it only wires the per-run token) and matches the plan's own wording that the wiring lives in worker.rs, not in test/caller code."
  - "RunSubmissionService::with_local_tokens is a builder, not a constructor parameter -- RunSubmissionService::new's signature is UNCHANGED, so 27-01's tracer_e2e.rs (out of this task's file scope) needed no update at all. A service that never calls with_local_tokens keeps a fresh, always-empty LocalRunTokens, so cancel's was_local always (and correctly) reports false for it."
  - "The cross-instance test uses SqliteRunRepository/SqliteWaypointStore over real on-disk temp files (never :memory:), not InMemory adapters -- the point being proven is that a durable, persisted flag (not an in-process shortcut) crosses instances; an InMemory adapter would make that indistinguishable from a same-process call."

requirements-completed: [PLAT-02]

coverage:
  - id: D1
    description: "WarEngine::with_cancellation_probe is an added builder method; the engine calls probe.is_cancelled(&thread) once per superstep boundary beside -- not instead of -- the existing CancellationToken check, and a true answer produces the same Halted Waypoint path"
    requirement: "PLAT-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs -- engine::tests::cancellation_probe_tests::{probe_cancelling_on_the_second_boundary_halts_after_one_completed_superstep, probe_that_never_cancels_changes_nothing, probe_is_consulted_exactly_once_per_superstep_boundary, either_token_or_probe_cancelling_halts_the_run}"
        status: pass
    human_judgment: false
  - id: D2
    description: "CancellationProbe::is_cancelled is infallible by signature; the DB-backed adapter (DbCancellationProbe) logs and returns false on any repository error, so a probe failure can never fail a run"
    requirement: "PLAT-02"
    verification:
      - kind: unit
        ref: "src/application/services/run/cancel.rs -- cancel::tests::a_repository_error_yields_false_and_never_panics"
        status: pass
    human_judgment: false
  - id: D3
    description: "The facade adapter debounces: two boundary checks inside min_probe_interval cause one repository read"
    requirement: "PLAT-02"
    verification:
      - kind: unit
        ref: "src/application/services/run/cancel.rs -- cancel::tests::{two_calls_within_the_interval_hit_the_repository_once, a_fresh_read_happens_after_the_interval_elapses}"
        status: pass
    human_judgment: false
  - id: D4
    description: "RunSubmissionPort::cancel(run_id) persists cancel_requested FIRST, then best-effort cancels the in-process token if the run is local; it is idempotent on a non-terminal run and returns AlreadyTerminal on a terminal one"
    requirement: "PLAT-02"
    verification:
      - kind: unit
        ref: "src/application/services/run/cancel_tests.rs#cancel_is_idempotent_on_a_non_terminal_run; src/application/services/run/submission.rs -- request_cancel precedes cancel_if_local in RunSubmissionService::cancel's body (source-order verified)"
        status: pass
    human_judgment: false
  - id: D5
    description: "A run executing on instance A is cancelled by a flag written through instance B's repository handle; A halts at the next superstep boundary with a Halted Waypoint and the run is recorded Cancelled"
    requirement: "PLAT-02"
    verification:
      - kind: integration
        ref: "src/application/services/run/cancel_tests.rs#cross_instance_cancel_probe (two RunWorkerPools over one shared SqliteRunRepository + SqliteWaypointStore temp file; B's service cancels a run only A dispatches -- was_local == false, A halts, run reaches Cancelled with a Halted latest Waypoint, queue depth 0, fewer than 6 of 6 supersteps ran)"
        status: pass
    human_judgment: false

duration: ~2h
completed: 2026-09-08
status: complete
---

# Phase 27 Plan 07: Cross-Instance Cancellation Summary

**A `CancellationProbe` engine seam (infallible, consulted beside the existing `CancellationToken`), a debounced `DbCancellationProbe` adapter reading a durable repository flag, and a persisted-flag-first `RunSubmissionPort::cancel` -- proven by a two-`RunWorkerPool` test over one shared on-disk SQLite repository where instance B's cancel halts a run only instance A is executing.**

## Performance

- **Duration:** ~2h
- **Started:** 2026-09-08 (this session, worktree base `86aa8941`)
- **Completed:** 2026-09-08
- **Tasks:** 2 (both `type="auto" tdd="true"`)
- **Files modified:** 12 (3 created, 9 modified)

## Accomplishments

- `CancellationProbe` (`crates/paladin-ports/src/output/cancellation_probe.rs`) is a new, infallible (`async fn is_cancelled(&self, thread: &ThreadId) -> bool`, no `Result` at all) port trait, with `NeverCancelled` as a documented no-op default and a counting mock precedent for consumers.
- `WarEngine::with_cancellation_probe` is an additive builder method threaded through `start`/`resume`/`resume_with`/`fork`/`replay` and into every nested `NodeSpec::Battalion` child run via `ChildEngineResources`; the superstep boundary check became `token_cancelled || probe_cancelled`, evaluated once per iteration, with the `Halted` branch body byte-identical to before. No existing `WarEngine`/`superstep::run` public call site (`engine/mod.rs`) changed signature; the internal `pub(crate)` `superstep::run`/`run_with_namespace` gained one new parameter, mechanically threaded through all 15 internal call sites (13 test helpers in `superstep.rs`, 1 in `graph.rs`, plus the 4 production call sites in `engine/mod.rs`).
- 4 new engine tests (`engine::tests::cancellation_probe_tests`) prove: a probe cancelling on the second boundary halts after exactly one completed superstep with the correct next-to-run vanguard; an attached-but-never-cancelling probe changes nothing; the probe is consulted exactly 3 times over a 3-superstep chain (documented: the loop's own inline "next_vanguard empty -> Completed" short-circuit means the LAST superstep never triggers a 4th top-of-loop check); either the token or the probe alone is sufficient to halt.
- `DbCancellationProbe` (facade, `src/application/services/run/cancel.rs`) reads `RunRepositoryPort::is_cancel_requested`, caches each thread's answer for a configured `min_interval` (D-15: policy in the adapter, mechanism in the engine), and swallows any repository error into a logged `false` (T-27-07-03).
- `LocalRunTokens` is the same-instance fast-path registry (`register`/`remove`/`cancel_if_local`, keyed by `RunId`) `RunSubmissionService::cancel` consults for `CancelOutcome::was_local`.
- `RunSubmissionPort::cancel` (new port method) + `CancelOutcome { run_id, status, was_local }`: `RunSubmissionService::cancel` calls `repository.request_cancel` FIRST (durability before signalling, D-16), then `local_tokens.cancel_if_local` best-effort -- calling it twice on a non-terminal run is `Ok` both times.
- `RunWorkerPool` gained two additive builders -- `with_engine_factory` (per-run engine construction carrying a `CancellationToken::child_token()` of the pool's shutdown token, registered in `local_tokens` for the dispatch's duration) and `with_cancellation_probing` (the pool builds its own `DbCancellationProbe` over its own `repository` and attaches it via `WarEngine::with_cancellation_probe` to every per-run engine) -- both `None`/no-op by default, preserving 27-04's exact shared-engine behavior verbatim.
- `cross_instance_cancel_probe` and `local_cancel_signals_token` (`src/application/services/run/cancel_tests.rs`) prove the whole mechanism end to end over real on-disk `SqliteRunRepository`/`SqliteWaypointStore` files (Tier 1, no Docker): instance B's `RunSubmissionService::cancel` (its own, always-empty `LocalRunTokens`) durably flags a run only instance A dispatches -- `was_local == false`, A halts at the next boundary within the 50ms debounce window, the run reaches `Cancelled` with the latest Waypoint `Halted`, queue depth returns to 0, and fewer than all 6 supersteps ran; the local variant proves the token path halts near-instantly even against a deliberately long (60s) probe debounce.

## Task Commits

Each task was committed atomically:

1. **Task 1: `CancellationProbe` port and the engine seam** - `dfcb82d6` (feat)
2. **Task 2: `DbCancellationProbe`, `LocalRunTokens`, `RunSubmissionPort::cancel`, and the cross-instance test** - `6505da88` (feat)

**Plan metadata:** this file's own commit (docs: complete plan) -- committed alongside this SUMMARY per worktree execution mode.

_TDD note: both tasks carry `tdd="true"`. Per-task tests were written and passing before each commit; no separate RED-then-GREEN commit pair was produced (test + implementation landed together per task, consistent with 27-01/27-02/27-04's documented convention for this worktree)._

## Files Created/Modified

- `crates/paladin-ports/src/output/cancellation_probe.rs` -- `CancellationProbe` trait, `NeverCancelled`, doc test, unit tests (counting mock, object-safety).
- `crates/paladin-ports/src/output/mod.rs` -- declares `pub mod cancellation_probe;`.
- `crates/paladin-ports/src/input/run_submission_port.rs` -- `CancelOutcome`, `RunSubmissionPort::cancel`; `AlwaysUnwired` test double updated for the new required method.
- `crates/paladin-battalion/src/engine/mod.rs` -- `WarEngine::cancellation_probe` field, `with_cancellation_probe` builder, threaded into `start`/`resume_with_options`/`resume_with`/`replay_or_fork`; 4 new `cancellation_probe_tests`.
- `crates/paladin-battalion/src/engine/superstep.rs` -- `run`/`run_with_namespace` gain a `probe` parameter; the boundary check is `token_cancelled || probe_cancelled`; `ChildEngineResources::cancellation_probe` propagates it to nested Battalion children; all 15 internal call sites updated.
- `crates/paladin-battalion/src/engine/graph.rs` -- one test helper (`run_to_completion`) updated for the new `superstep::run` parameter.
- `crates/paladin-web/src/run_controller.rs` -- `MockSubmissionPort` test double gains `cancel` (Rule 3, the trait grew a required method).
- `src/application/services/run/cancel.rs` -- `DbCancellationProbe`, `LocalRunTokens`, 6 unit tests.
- `src/application/services/run/cancel_tests.rs` -- `cross_instance_cancel_probe`, `local_cancel_signals_token`, `cancel_is_idempotent_on_a_non_terminal_run`.
- `src/application/services/run/worker.rs` -- `RunWorkerPool::{engine_factory, local_tokens, cancellation_probe}` fields, `with_engine_factory`/`local_tokens`/`with_cancellation_probing` builders, `run_once`'s per-run engine construction and `local_tokens` register/remove.
- `src/application/services/run/submission.rs` -- `RunSubmissionService::local_tokens` field, `with_local_tokens` builder, `map_cancel_error`, `RunSubmissionPort::cancel` impl.
- `src/application/services/run/mod.rs` -- declares `cancel`/`cancel_tests` modules, re-exports `DbCancellationProbe`/`LocalRunTokens`.

## Decisions Made

See `key-decisions` in frontmatter. In prose:

1. **`RunWorkerPool::with_engine_factory` instead of a `WarEngine` mutator.** `WarEngine` has no `Clone` and `with_cancellation_token` is by-value, but this task's declared file scope excludes further `paladin-battalion` API growth beyond Task 1's additive `with_cancellation_probe`. Since every `WarEngine` field is a cheap `Arc`/`Copy` value, rebuilding a fresh engine per run through a caller-supplied factory closure (carrying a `CancellationToken::child_token()` of the pool's own shutdown token, so process-wide shutdown still cascades to every in-flight per-run engine) was the smaller, purely-additive change: `RunWorkerPool::new`'s signature is completely unchanged, and the default (no factory) path is byte-identical to 27-04.
2. **The pool itself calls `with_cancellation_probe`, not the caller's factory.** A separate `with_cancellation_probing(min_interval)` builder has the pool build its own `DbCancellationProbe` over its own `repository` field and attach it inside `run_once` to whatever engine the factory returns. This keeps every factory closure trivial (it only ever wires the per-run token) and matches the plan's own wording that this wiring belongs in `worker.rs`.
3. **`RunSubmissionService::with_local_tokens` is a builder, not a constructor argument.** Keeping `RunSubmissionService::new`'s signature unchanged meant 27-01's `tracer_e2e.rs` (outside this task's file list) needed zero updates -- a service that never opts in keeps a private, always-empty `LocalRunTokens`, so `cancel`'s `was_local` correctly and safely defaults to `false`.
4. **The cross-instance proof uses real on-disk SQLite, not InMemory.** `InMemoryRunRepository` shared via one `Arc` between two "instances" would prove nothing about durability -- the whole point of D-14/D-16 is that the flag survives independent of any in-process shortcut. `cross_instance_cancel_probe` and `local_cancel_signals_token` connect through `SqliteRunRepository::new`/`SqliteWaypointStore::new` over real temp files (Tier 1, no Docker, cleaned up on pass).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `MockSubmissionPort` (a `RunSubmissionPort` test double in `paladin-web`, outside this task's declared file scope) needed the new `cancel` method**
- **Found during:** Task 2, `cargo check -p paladin-ai -p paladin-ports -p paladin-web --all-targets`
- **Issue:** `RunSubmissionPort` gained a required `cancel` method (this task's own change). `crates/paladin-web/src/run_controller.rs`'s test-only `MockSubmissionPort` implements the trait and would no longer compile.
- **Fix:** Added a `cancel` impl mirroring the existing `MockOutcome` match arms (`NotWired` -> `Err(RunSubmissionError::NotWired)`; every other outcome -> a synthetic `CancelOutcome`).
- **Files modified:** `crates/paladin-web/src/run_controller.rs`
- **Verification:** `cargo test -p paladin-web --lib` -- 148 passed.
- **Committed in:** `6505da88` (Task 2 commit)

**2. [Rule 3 - Blocking] `superstep::run`'s internal signature change required updating 14 out-of-plan test call sites**
- **Found during:** Task 1, first `cargo check -p paladin-battalion --all-targets` after adding the `probe` parameter
- **Issue:** `pub(crate) fn run`/`run_with_namespace` in `superstep.rs` needed a `probe: &Option<Arc<dyn CancellationProbe>>` parameter threaded alongside `cancellation` (mirroring the token's own threading) so `WarEngine::start`/`resume_with_options` could pass it through. 13 test helper call sites within `superstep.rs`'s own `#[cfg(test)] mod tests` and 1 in `crates/paladin-battalion/src/engine/graph.rs`'s own test module called `run(...)` positionally and needed the new argument inserted.
- **Fix:** Inserted `&None,` (script-verified against each call's exact argument position, confirmed by `cargo check`'s own `argument #19 ... is missing` diagnostics) at each of the 14 call sites; none of these tests exercise probe behavior, so `&None` (identical to no probe attached) is the correct, semantics-preserving default.
- **Files modified:** `crates/paladin-battalion/src/engine/superstep.rs`, `crates/paladin-battalion/src/engine/graph.rs`
- **Verification:** `cargo test -p paladin-battalion --lib engine` -- 503 passed (499 pre-existing + 4 new), unchanged behavior for every pre-existing test.
- **Committed in:** `dfcb82d6` (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 3 blocking fixes, both mechanical consequences of an additive trait/function-signature growth this task's own action explicitly called for)
**Impact on plan:** Neither changed this plan's architecture or scope; both were necessary to reach a compiling, fully-passing state.

## Issues Encountered

None beyond the two auto-fixed deviations above -- both were caught during the first `cargo check`/`cargo test` pass for their respective task, before any commit.

## User Setup Required

None -- no external service configuration required. Every test in this plan runs against InMemory adapters or real on-disk SQLite temp files (Tier 1, D-51), with no Docker dependency.

## Next Phase Readiness

- The `CancellationProbe` port and `WarEngine::with_cancellation_probe` seam are ready for 27-15's `POST /runs/{id}/cancel` HTTP route to call straight through to `RunSubmissionService::cancel` -- the service operation and the cross-instance mechanism both already exist and are proven; 27-15 only needs to wire the route and the authorization tier (D-46, T-27-07-04, deliberately deferred to that plan's own threat register entry).
- `RunWorkerPool::with_engine_factory`/`with_cancellation_probing` are additive opt-ins; whatever plan wires the production `RunWorkerPool` (27-17 per 27-04's own SUMMARY) can adopt them without disturbing any existing call site, or skip them entirely and keep the shared-engine default.
- No blockers. `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo check --workspace --all-targets --all-features` all pass clean on the final commit.

## Self-Check: PASSED

**Files verified to exist:**
- FOUND: `crates/paladin-ports/src/output/cancellation_probe.rs`
- FOUND: `crates/paladin-ports/src/output/mod.rs`
- FOUND: `crates/paladin-ports/src/input/run_submission_port.rs`
- FOUND: `crates/paladin-battalion/src/engine/mod.rs`
- FOUND: `crates/paladin-battalion/src/engine/superstep.rs`
- FOUND: `crates/paladin-battalion/src/engine/graph.rs`
- FOUND: `crates/paladin-web/src/run_controller.rs`
- FOUND: `src/application/services/run/cancel.rs`
- FOUND: `src/application/services/run/cancel_tests.rs`
- FOUND: `src/application/services/run/worker.rs`
- FOUND: `src/application/services/run/submission.rs`
- FOUND: `src/application/services/run/mod.rs`

**Commits verified to exist (git log --oneline):**
- FOUND: `dfcb82d6` feat(27-07): add CancellationProbe port and engine seam
- FOUND: `6505da88` feat(27-07): add DbCancellationProbe, LocalRunTokens and RunSubmissionPort::cancel

**Verification commands re-run and confirmed passing:**
- `cargo test -p paladin-ports --lib cancellation_probe` -> `test result: ok. 3 passed`
- `cargo test -p paladin-battalion --lib cancellation_probe` -> `test result: ok. 4 passed`
- `cargo test -p paladin-battalion --lib engine` -> `test result: ok. 503 passed`
- `cargo test -p paladin-ports --lib` -> `test result: ok. 163 passed`
- `cargo test -p paladin-ai --lib services::run::cancel` -> `test result: ok. 8 passed` (debounce x2, error-yields-false x1, LocalRunTokens x2, idempotent-cancel x1, cross-instance x1, local x1)
- `cargo test -p paladin-ai --lib services::run` -> `test result: ok. 36 passed`
- `cargo test -p paladin-web --lib` -> `test result: ok. 148 passed`
- `grep -c 'pub fn with_cancellation_probe' crates/paladin-battalion/src/engine/mod.rs` -> `1`
- `grep -c 'is_cancelled(&thread)' crates/paladin-battalion/src/engine/superstep.rs` -> `1` (at least 1 required)
- `grep -v '^\s*//' crates/paladin-ports/src/output/cancellation_probe.rs | grep -c 'fn is_cancelled.*Result'` -> `0`
- `grep -c 'fn cross_instance_cancel_probe' src/application/services/run/cancel_tests.rs` -> `1`
- `awk '/fn cancel/,/^    }/' src/application/services/run/submission.rs | grep -n 'request_cancel\|cancel_if_local'` -> `request_cancel` listed before `cancel_if_local`
- `grep -c 'async fn cancel' crates/paladin-ports/src/input/run_submission_port.rs` -> `2` (trait method + `AlwaysUnwired` impl)
- `grep -c 'with_cancellation_probe' src/application/services/run/worker.rs` -> `5`
- `cargo fmt --all -- --check` -> clean
- `cargo clippy --workspace --all-targets -- -D warnings` -> clean
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` -> clean
- `cargo check --workspace --all-targets --all-features` -> exit 0

---
*Phase: 27-platform-api*
*Completed: 2026-09-08*
