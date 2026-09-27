---
phase: 39-spend-ledger
plan: 05
subsystem: agent-runtime
tags: [treasurer, ledger, agent-loop, run-scope, hexagonal-ports, adr-0052, adr-0053]

# Dependency graph
requires:
  - phase: 39-spend-ledger
    provides: "39-01/39-02/39-03: TreasuryLedgerPort's full surface (reserve/release/settle/
      spend/store_now) proven on in-memory, SQLite and Postgres adapters; 39-04's
      SettlementContext/SpendHook discipline and observational-failure precedent, mirrored here
      for the agent loop"
provides:
  - "RunScope.run_id: Option<RunId> and RunScope::with_run_id -- additive, #[non_exhaustive]
    stays intact"
  - "PaladinExecutionService::AgentLoopSettlement (EveryCall/PlatformRunsOnly) and
    with_treasury_ledger -- one settle writer for both the buffered reasoning loop and the
    streamed terminal chunk, keyed per D-07"
  - "build_agent_registry_with_ledger (agent_host.rs) and paladin_port_from_settings_with_ledger
    (facade_provisioner.rs) -- the two production installation points, with every existing
    public signature kept unchanged"
  - "EngineExecutionPort::execute_scoped forwards a worker-supplied RunScope run id into the
    run engine's shared execution service"
affects: [39-06, 39-07, 39-08]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Agent-loop settlement shares one free function (settle_agent_loop_call) between the
      buffered reasoning loop (an inherent &self method delegating to it) and the streamed
      path's spawned task (which owns no &self) -- one settle implementation, two call sites,
      mirroring 39-04's SpendHook::settle_boundary discipline for the engine path"
    - "EveryCall vs PlatformRunsOnly is a two-mode enum selected at construction time, not a
      runtime branch on caller identity -- HTTP agent services always install EveryCall, the
      run engine's one shared service always installs PlatformRunsOnly, so no code path can
      accidentally settle the same call under both regimes"
    - "A ledger failure is logged and never propagates into the PaladinResult, the streamed
      chunk, or a retried call -- the same observational-failure discipline 39-04 established
      for the engine path, now proven on the agent loop by a dedicated FailingTreasuryLedger
      test double"

key-files:
  created: []
  modified:
    - crates/paladin-core/src/platform/container/run_scope.rs
    - src/application/services/paladin/paladin_execution_service.rs
    - src/infrastructure/web/agent_host.rs
    - src/infrastructure/web/facade_provisioner.rs

key-decisions:
  - "settle_agent_loop_call extracted as a free function (not an inherent method) specifically
    so execute_stream_inner's tokio::spawn'd task -- which owns no &self, since a spawned task
    outlives the borrow -- can call the exact same settle logic the buffered loop's
    settle_model_call delegates to, rather than duplicating the SettleRequest construction and
    log lines in two places"
  - "The streamed path never carries a RunScope (no execute_stream_scoped exists in this
    codebase), so a streamed call's run id is always resolved from the execution id under
    EveryCall mode only -- PlatformRunsOnly, which requires a scope-carried run id, never
    settles a stream, matching the plan's own must-have truth that a streamed call settles
    under (execution id, 1, 1)"
  - "build_agent_with_llm/build_agent gained a trailing Option<Arc<dyn TreasuryLedgerPort>>
    parameter rather than a builder-pattern overload, mirroring the existing trailing
    price_table: &Arc<PriceTable> parameter shape those same functions already use for D-09
    pricing -- keeps the two Treasurer concerns (pricing, ledger) parallel at the same call
    site instead of introducing a second composition mechanism"

patterns-established:
  - "AgentLoopSettlement::{EveryCall, PlatformRunsOnly} is the one enum both hosts branch on;
    Phase 40/41/42 extending the agent loop's ledger behavior (allowances, halts) attach at the
    same with_treasury_ledger call site per the plan's own design note"

requirements-completed: [LEDGR-03, LEDGR-04]

coverage:
  - id: D1
    description: "RunScope.run_id is additive (Option<RunId>, serde-omitted when None),
      RunScope stays #[non_exhaustive], and RunScope::with_run_id is the only way to set it
      from outside paladin-core"
    requirement: "LEDGR-04"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-ai-core --lib run_scope (5 tests, incl.
          run_scope_run_id_omitted_when_none_and_round_trips_when_some)"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ai-core --doc run_scope (3 doctests)"
        status: pass
    human_judgment: false
  - id: D2
    description: "With PaladinExecutionService::with_treasury_ledger(ledger,
      AgentLoopSettlement::EveryCall), every priced model call of the reasoning loop settles
      exactly once, immediately after its cost is folded into the run's CostTally, under
      (run id, loop_num, 1) with LedgerScope::unattributed() and a model_breakdown keyed on the
      served response's model"
    requirement: "LEDGR-04"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-ai --lib paladin_execution_service -- agent_loop_cost_tests
          (8 tests, incl. every_priced_model_call_settles_under_the_execution_id,
          unpriced_model_call_writes_no_settlement)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The agent-loop run id is RunScope.run_id when the caller supplied one, else
      the service's own execution id (D-07); execute_scoped with a scoped run id settles both
      loop iterations under it, proven by re-settling the exact same keys directly against the
      ledger and observing AlreadySettled"
    requirement: "LEDGR-04"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-ai --lib paladin_execution_service --
          agent_loop_cost_tests::every_call_mode_prefers_the_scope_run_id"
        status: pass
    human_judgment: false
  - id: D4
    description: "With AgentLoopSettlement::PlatformRunsOnly, only calls whose RunScope names a
      run id settle -- a plain execute() call never settles, so the run engine's shared service
      never double-charges an engine node already settled per superstep by 39-04"
    requirement: "LEDGR-03"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-ai --lib paladin_execution_service --
          agent_loop_cost_tests::platform_runs_only_settles_only_scoped_runs"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ai --features web-server --lib infrastructure::web::facade_provisioner --
          engine_execution_port_forwards_the_run_scope_to_the_ledger"
        status: pass
    human_judgment: false
  - id: D5
    description: "A ledger Err never fails, retries or alters an agent-loop execution -- output,
      cost and loop_count are identical with and without a failing ledger installed"
    requirement: "LEDGR-03"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-ai --lib paladin_execution_service --
          agent_loop_cost_tests::ledger_failure_never_fails_the_agent_loop"
        status: pass
    human_judgment: false
  - id: D6
    description: "A streamed priced call settles once at its terminal chunk under
      (execution_id, 1, 1), naming the request's model -- the only model identity a stream
      carries"
    requirement: "LEDGR-04"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-ai --lib paladin_execution_service --
          streamed_cost_tests::streamed_priced_call_settles_once"
        status: pass
    human_judgment: false
  - id: D7
    description: "build_agent_with_llm/build_agent install EveryCall when a ledger is supplied;
      build_agent_registry keeps its signature, delegating to build_agent_registry_with_ledger
      with None; a config-defined agent built with a ledger settles its priced calls, and one
      built without settles nothing"
    requirement: "LEDGR-04"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-ai --features web-server --lib infrastructure::web::agent_host
          (14 tests, incl. build_agent_with_llm_settles_priced_calls_when_a_ledger_is_installed,
          build_agent_with_llm_without_a_ledger_settles_nothing)"
        status: pass
    human_judgment: false
  - id: D8
    description: "FacadeProvisioner::with_treasury_ledger prices-and-settles runtime-provisioned
      agents; paladin_port_from_settings keeps its signature, delegating to
      paladin_port_from_settings_with_ledger with None; EngineExecutionPort::execute_scoped
      forwards the RunScope (heartbeat discarded exactly as the trait default did)"
    requirement: "LEDGR-04"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-ai --features web-server --lib infrastructure::web::facade_provisioner
          (6 tests, incl. engine_execution_port_forwards_the_run_scope_to_the_ledger)"
        status: pass
      - kind: other
        ref: "cargo build -p paladin-ai --features web-server --bin paladin-server"
        status: pass
    human_judgment: false

duration: ~50min
completed: 2026-09-27
status: complete
---

# Phase 39 Plan 05: Agent-Loop Treasury Ledger Settlement Summary

**`PaladinExecutionService::with_treasury_ledger` settles every priced agent-loop model call (buffered or streamed) exactly once, keyed per D-07, installed on HTTP agent services with `AgentLoopSettlement::EveryCall` and on the run engine's shared service with `PlatformRunsOnly` -- the mode that keeps a Platform API run's agent-kind spend from ever being double-charged against 39-04's engine superstep settlements.**

## Performance

- **Duration:** ~50 min
- **Tasks:** 2 (both `type="auto"`/`tdd="true"`, no checkpoints)
- **Files modified:** 4 (0 created, 4 modified)

## Accomplishments

- **`RunScope.run_id`** (`crates/paladin-core/src/platform/container/run_scope.rs`): an
  additive `Option<RunId>` (`#[serde(default, skip_serializing_if = "Option::is_none")]`) and
  `RunScope::with_run_id`, with the struct staying `#[non_exhaustive]` (D-00f). Two new tests
  plus a new doctest prove the default serializes with no `run_id` key at all and a scope
  carrying one round-trips.
- **`AgentLoopSettlement` and `PaladinExecutionService::with_treasury_ledger`**
  (`src/application/services/paladin/paladin_execution_service.rs`): a two-variant enum
  (`EveryCall`, `PlatformRunsOnly`) and a private `treasury_ledger: Option<(Arc<dyn
  TreasuryLedgerPort>, AgentLoopSettlement)>` field. Private `ledger_run_id` resolves the run id
  a call should settle under per D-07 (scope's own run id first, then the execution id under
  `EveryCall`, or nothing at all under `PlatformRunsOnly` with no scoped run id). Private
  `settle_model_call` (buffered path) and the free function `settle_agent_loop_call` it
  delegates to (shared with the streamed path's spawned task, which owns no `&self`) build an
  unreserved `SettleRequest` under `LedgerScope::unattributed()` and log `debug!`/`warn!`/
  `error!` on `Settled`/`AlreadySettled`/`Err` respectively -- never propagating. `execute_scoped`
  resolves the run id once and threads it through `execute_bounded` into `execute_internal` as a
  new trailing `ledger_run_id: Option<RunId>` parameter; the settle call sits immediately after
  `cost_tally.record_call(response.cost.as_ref())`, naming `response.model` (falling back to
  `paladin.node.model` when empty). `execute_stream_inner` resolves a `stream_settlement` (ledger
  + execution-id-derived `RunId`, `EveryCall` only -- the streamed path carries no `RunScope`)
  once before the chunk-forwarding task is spawned, and settles at the terminal chunk's `cost`
  before that chunk is sent, under `(execution_id, 1, 1)` naming `model_used`.
- **Seven new tests** in `agent_loop_cost_tests` (every_priced_model_call_settles_under_the_execution_id,
  unpriced_model_call_writes_no_settlement, every_call_mode_prefers_the_scope_run_id,
  platform_runs_only_settles_only_scoped_runs, ledger_failure_never_fails_the_agent_loop, plus
  the pre-existing three cost tests) and one new test in `streamed_cost_tests`
  (streamed_priced_call_settles_once), all against `InMemoryTreasuryLedger`. A new
  `FailingTreasuryLedger` test double (mirroring 39-04's engine-side one) proves the
  never-fails-the-run guarantee.
- **`build_agent_registry_with_ledger`** (`src/infrastructure/web/agent_host.rs`):
  `build_agent_with_llm`/`build_agent` gain a trailing `treasury_ledger:
  Option<Arc<dyn TreasuryLedgerPort>>` parameter; when `Some`, the one shared
  `PaladinExecutionService` installs `.with_treasury_ledger(ledger, AgentLoopSettlement::EveryCall)`
  before being split into its executor/streamer handles. `build_agent_registry(settings)` is now
  a one-line delegation to `build_agent_registry_with_ledger(settings, None)` -- signature
  unchanged. Two new tests prove a ledger settles a priced call and no ledger settles nothing.
- **`FacadeProvisioner::with_treasury_ledger` and `paladin_port_from_settings_with_ledger`**
  (`src/infrastructure/web/facade_provisioner.rs`): a private `treasury_ledger` field on
  `FacadeProvisioner`, threaded into `provision`'s `build_agent` call.
  `impl PaladinPort for EngineExecutionPort` gains an `execute_scoped` override forwarding
  `scope` into `self.0.execute_scoped(paladin, input, None, scope)` -- heartbeat handling
  unchanged (still discarded, exactly as the trait's own defaulted body did), so a
  worker-supplied `RunScope` run id (39-07) now actually reaches the agent-loop settle writer
  instead of being silently dropped. `paladin_port_from_settings_with_ledger` holds the full
  build, installing `AgentLoopSettlement::PlatformRunsOnly` when `Some`;
  `paladin_port_from_settings(settings)` delegates with `None` -- signature unchanged. One new
  test (`engine_execution_port_forwards_the_run_scope_to_the_ledger`) proves a plain call never
  settles under `PlatformRunsOnly` while a scoped call settles under its own run id.

## Task Commits

1. **Task 1: Agent-loop settle writer in PaladinExecutionService and RunScope.run_id** -
   `503f19f5` (test)
2. **Task 2: Install the agent-loop writer on HTTP agents and the run engine's shared service** -
   `64922987` (feat)

**Plan metadata:** pending (this commit)

## Files Created/Modified

- `crates/paladin-core/src/platform/container/run_scope.rs` - `RunScope.run_id`,
  `RunScope::with_run_id`
- `src/application/services/paladin/paladin_execution_service.rs` - `AgentLoopSettlement`,
  `with_treasury_ledger`, `ledger_run_id`, `settle_model_call`, `settle_agent_loop_call`, the
  buffered- and streamed-path settle call sites, and their tests
- `src/infrastructure/web/agent_host.rs` - `build_agent_registry_with_ledger`, threaded
  `treasury_ledger` parameter on `build_agent_with_llm`/`build_agent`, and their tests
- `src/infrastructure/web/facade_provisioner.rs` - `FacadeProvisioner::with_treasury_ledger`,
  `paladin_port_from_settings_with_ledger`, `EngineExecutionPort::execute_scoped`, and its test

## Decisions Made

- `settle_agent_loop_call` extracted as a free function so the streamed path's spawned task
  (owns no `&self`) shares the exact same settle logic the buffered loop's `settle_model_call`
  delegates to -- one implementation, two call sites, rather than duplicating the
  `SettleRequest`/logging shape.
- The streamed path settles only under `EveryCall` (never `PlatformRunsOnly`): there is no
  `execute_stream_scoped` in this codebase, so a stream never carries a `RunScope` to read a
  Platform run id from -- exactly matching the plan's own must-have truth that a streamed call
  settles under `(execution_id, 1, 1)`.
- `build_agent_with_llm`/`build_agent` gained a trailing `Option<Arc<dyn TreasuryLedgerPort>>`
  parameter (mirroring the existing trailing `price_table` parameter) rather than a second
  builder-style composition mechanism, keeping the Treasurer's two concerns (pricing, ledger)
  parallel at the same call site.

## Deviations from Plan

None - plan executed exactly as written. Both tasks' `<behavior>` clauses map one-to-one onto
the tests added, and every acceptance-criteria grep in the plan passes verbatim.

## Issues Encountered

- The sandbox's root filesystem ran out of disk space (`ENOSPC`) mid-verification, during
  `cargo clippy -p paladin-ai --features web-server --all-targets -- -D warnings` (mirroring the
  identical issue recorded in 39-01/39-03/39-04's SUMMARYs). Cleared with `rm -rf
  target/debug/incremental` (safe: incremental compilation caches are regenerable and carry no
  source-of-truth data); freed the build to complete. Not a code or plan issue -- environment
  housekeeping only, not reintroduced by either commit.

## User Setup Required

None - no external service configuration required. Both new test suites use only the in-memory
ledger adapter (already a normal, non-dev dependency of the root crate via `paladin-storage`).

## Next Phase Readiness

- Both production run paths now have a complete, tested settle writer: the engine's superstep
  boundary (39-04) and the agent loop's per-call boundary (this plan), with `AgentLoopSettlement`
  as the single mechanism that prevents the run engine's shared service from ever double-charging
  an engine node. LEDGR-03 and LEDGR-04 are now proven end-to-end on the agent-loop path.
- 39-06/39-07 (wiring the ledger into the HTTP server boot path and the Platform API worker,
  respectively) can now call `build_agent_registry_with_ledger` and
  `paladin_port_from_settings_with_ledger` directly -- both functions and their
  `AgentLoopSettlement` mode selection are final for this phase.
- `make api-surface`/CHANGELOG updates remain explicitly owned by plan 39-08 (D-00f), consistent
  with every prior plan in this phase -- not touched here. Every new public item is additive
  (`RunScope.run_id`/`with_run_id`, `AgentLoopSettlement`, `with_treasury_ledger`,
  `build_agent_registry_with_ledger`, `FacadeProvisioner::with_treasury_ledger`,
  `paladin_port_from_settings_with_ledger`); no existing public signature changed.
- No blockers.

---
*Phase: 39-spend-ledger*
*Completed: 2026-09-27*

## Self-Check: PASSED

Both modified source files outside `crates/paladin-core` and `src/infrastructure/web` (the
`paladin_execution_service.rs` change) verified present on disk with their new symbols; commits
`503f19f5` and `64922987` verified present in git history (`git log --oneline --all | grep -E
"503f19f5|64922987"` both found).
