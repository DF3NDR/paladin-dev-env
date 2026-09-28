---
phase: 39-spend-ledger
plan: 07
subsystem: platform-api
tags: [treasurer, ledger, worker, run-api, boot-wiring, adr-0052, adr-0053, d-07, d-08]

# Dependency graph
requires:
  - phase: 39-spend-ledger
    provides: "39-04: WarEngine::with_treasury_ledger + SettlementContext (the per-run
      attachment this plan drives from production coordinates); 39-05:
      AgentLoopSettlement::PlatformRunsOnly + RunScope::with_run_id + build_agent_registry_with_ledger
      + paladin_port_from_settings_with_ledger (the agent-loop half this plan finally
      supplies a real RunScope to); 39-06: RunApiState::with_treasury_ledger (the HTTP
      cost surface this plan's production ledger now fills); 39-01/39-03: RunStoreConfig
      as the single backend-selection source this plan's build_treasury_ledger reuses"
provides:
  - "RunWorkerPool::with_treasury_ledger -- attaches WarEngine::with_treasury_ledger to
    every per-run engine_factory-built engine, keyed by SettlementContext { scope:
    LedgerScope::unattributed(), run_id: run.run_id, attempt } where attempt is the
    persisted counter (run.attempt on first dispatch, bump_attempt's return on a
    Running redelivery) -- D-07 finally implemented in production code, not just tests"
  - "run_agent dispatches through PaladinPort::execute_scoped with
    RunScope::default().with_run_id(run.run_id.clone()), so an agent-kind Platform run
    settles under its own run id via 39-05's AgentLoopSettlement::PlatformRunsOnly writer"
  - "build_treasury_ledger(&RunStoreConfig) -- Disabled -> None, Sqlite -> SqliteTreasuryLedger,
    Postgres -> PostgresTreasuryLedger (storage-postgres-gated, named-feature error otherwise)
    -- the one function build_run_api and paladin-server.rs both call to share the ledger
    over the exact same database the run repository already uses (D-00g)"
  - "build_run_api wires the ledger into the worker pool, the run engine's PaladinPort
    (PlatformRunsOnly) and RunApiState, all from the SAME RunStoreConfig, all Some only
    when a backend is configured"
  - "paladin-server.rs builds RunStoreConfig and the ledger ahead of the agent registry
    and hands it to build_agent_registry_with_ledger and FacadeProvisioner::with_treasury_ledger"
affects: [39-08]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "The worker's status-transition match now YIELDS the attempt it just established
      (run.attempt on Queued/AwaitingInput, bump_attempt's return on Running) rather than
      discarding it -- the one place D-07's persisted-attempt contract is threaded from a
      repository call into the engine attachment a few lines later"
    - "execute_scoped's trait-default body delegates straight to execute for any PaladinPort
      that never overrides it, so run_agent's switch from execute to
      execute_scoped(..., &RunScope::default().with_run_id(...)) is behavior-identical for
      every port with no ledger writer installed -- proven by
      pool_without_a_treasury_ledger_settles_nothing and every pre-existing worker test
      passing unchanged"
    - "build_treasury_ledger mirrors build_sqlite_quartet/build_postgres_quartet's own
      cfg(storage-postgres) pair exactly, including the identical named-feature error
      wording -- one house pattern, reused rather than re-invented for a fourth backend
      dimension"

key-files:
  created: []
  modified:
    - src/application/services/run/worker.rs
    - src/application/services/run/tracer_e2e.rs
    - src/infrastructure/web/run_api_wiring.rs
    - src/bin/paladin-server.rs

key-decisions:
  - "build_run_api_wires_the_treasury_ledger added as a new, dedicated test (rather than
    only extending the two existing RunApiState-field tests) so the plan's own acceptance
    criterion -- a test literally named build_run_api_wires_the_treasury_ledger passing --
    is satisfied by name, not just by assertion. The two existing tests
    (defaults_wire_nothing_and_answer_501, sqlite_and_in_memory_wires_three_tasks_and_every_state_field)
    also gained a treasury_ledger assertion each, so both routes are still covered exactly
    where the plan's action text asked."
  - "RecordingTreasuryLedger, PricedPaladinPort and ScopeRecordingPaladinPort are defined
    locally in worker.rs's own test module (and a second, separate PricedPaladinPort in
    tracer_e2e.rs) rather than reused from paladin-battalion's identical 39-04 doubles --
    those are crate-private to paladin-battalion's own test module and unreachable from
    this facade crate, mirroring the plan's own read_first note about worker_tests.rs's
    UnusedPaladinPort precedent."
  - "priced_run_cost_reaches_get_run_through_the_ledger builds its own minimal
    PricedPaladinPort directly (never through PaladinExecutionService/MockLlmAdapter) --
    the tracer needs a WarEngine whose Paladin dispatch returns a fixed Cost, which is
    simpler and more direct than wiring a price table through the full agent-loop stack
    for a single fixed value."

requirements-completed: [LEDGR-03, LEDGR-04]

coverage:
  - id: D1
    description: "RunWorkerPool::with_treasury_ledger attaches the ledger to every per-run
      engine the engine_factory builds, keyed by SettlementContext { scope:
      LedgerScope::unattributed(), run_id, attempt } where attempt is the persisted
      counter -- a first dispatch settles under attempt 1"
    requirement: "LEDGR-03"
    verification:
      - kind: unit
        ref: "application::services::run::worker::tests::engine_run_settles_under_its_run_id_and_first_attempt"
        status: pass
    human_judgment: false
  - id: D2
    description: "A Running redelivery calls bump_attempt and the recorded settlement key
      carries the BUMPED attempt, never a re-invented 1 -- distinct keys for a genuine
      re-execution"
    requirement: "LEDGR-03"
    verification:
      - kind: unit
        ref: "application::services::run::worker::tests::redelivered_running_run_settles_under_the_bumped_attempt"
        status: pass
    human_judgment: false
  - id: D3
    description: "An agent-kind (Runnable::Agent) Platform run is dispatched through
      PaladinPort::execute_scoped with RunScope::default().with_run_id(run.run_id), so
      the recorded scope's run_id is Some(run.run_id) and the run still reaches Completed"
    requirement: "LEDGR-03"
    verification:
      - kind: unit
        ref: "application::services::run::worker::tests::agent_kind_run_passes_its_run_id_in_the_run_scope"
        status: pass
    human_judgment: false
  - id: D4
    description: "A pool with no treasury ledger attached performs no ledger call and
      behaves exactly as before -- every pre-existing worker/worker_tests/stream/http_surface
      test passes unchanged"
    requirement: "LEDGR-03"
    verification:
      - kind: unit
        ref: "application::services::run::worker::tests::pool_without_a_treasury_ledger_settles_nothing"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ai --features web-server --lib application::services::run (148 passed, 0 failed)"
        status: pass
    human_judgment: false
  - id: D5
    description: "End-to-end: a run submitted through POST /v1/runs whose graph has a
      Paladin node priced at 45,000,000 nanos USD, executed by the worker, is reported by
      GET /v1/runs/{id} with cost.display '0.0450 USD' read back from the same
      InMemoryTreasuryLedger"
    requirement: "LEDGR-04"
    verification:
      - kind: unit
        ref: "application::services::run::tracer_e2e::priced_run_cost_reaches_get_run_through_the_ledger"
        status: pass
    human_judgment: false
  - id: D6
    description: "build_treasury_ledger builds the ledger from RunStoreConfig alone
      (Disabled -> None, Sqlite -> SqliteTreasuryLedger, and a real settle/spend round
      trip proves it); build_run_api wires the same instance into the worker pool, the
      run engine's PaladinPort and RunApiState from one RunStoreConfig, Some only when a
      backend is configured"
    requirement: "LEDGR-04"
    verification:
      - kind: unit
        ref: "infrastructure::web::run_api_wiring::tests::build_treasury_ledger_disabled_is_none"
        status: pass
      - kind: unit
        ref: "infrastructure::web::run_api_wiring::tests::build_treasury_ledger_sqlite_round_trips"
        status: pass
      - kind: unit
        ref: "infrastructure::web::run_api_wiring::tests::build_run_api_wires_the_treasury_ledger"
        status: pass
    human_judgment: false
  - id: D7
    description: "paladin-server.rs builds RunStoreConfig and the ledger ahead of the
      agent registry and hands it to build_agent_registry_with_ledger and
      FacadeProvisioner::with_treasury_ledger, so HTTP agents settle every priced call
      when a run store is configured; every pre-existing server test passes unchanged"
    requirement: "LEDGR-04"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-ai --features web-server --bin paladin-server (14 passed, 0 failed)"
        status: pass
      - kind: other
        ref: "cargo build -p paladin-ai --features \"web-server storage-postgres\" --bin paladin-server"
        status: pass
    human_judgment: false

duration: ~50min
completed: 2026-09-28
status: complete
---

# Phase 39 Plan 07: Production Ledger Wiring Summary

**The worker settles every engine superstep and agent-kind run under the run's real, persisted `(run_id, superstep, attempt)`, and the server builds one Treasurer ledger from `RunStoreConfig` shared by the worker, the run engine's HTTP-agent port, and the run API -- turning three phases of ledger machinery into ordinary production operation, proven end-to-end from `POST /v1/runs` to `GET /v1/runs/{id}`'s `cost.display`.**

## Performance

- **Duration:** ~50 min
- **Started:** 2026-09-28T00:15:00Z (approx.)
- **Completed:** 2026-09-28T01:07:29Z
- **Tasks:** 2 (Task 1 `type="auto" tdd="true"`, Task 2 `type="auto"`; no checkpoints)
- **Files modified:** 4

## Accomplishments

- **`RunWorkerPool::with_treasury_ledger`** (`src/application/services/run/worker.rs`): a
  private `treasury_ledger: Option<Arc<dyn TreasuryLedgerPort>>` field (`None` in `new`), set by
  the new public builder. `run_once`'s status-transition `match` now yields the `attempt` it just
  established -- `run.attempt` on the `Queued`/`AwaitingInput` arms, `bump_attempt`'s returned
  value on the `Running` (redelivery) arm -- instead of discarding it. In the `engine_factory`
  branch, right after the cancellation probe is attached, when a ledger is wired the freshly
  built engine gets `.with_treasury_ledger(Arc::clone(ledger), SettlementContext { scope:
  LedgerScope::unattributed(), run_id: run.run_id.clone(), attempt })`. The shared no-factory
  engine is never touched, exactly like every other per-run-only field.
- **`run_agent` carries the Platform run id (D-07)**: replaced `paladin_port.execute(paladin,
  &input_text)` with `paladin_port.execute_scoped(paladin.as_ref(), &input_text,
  &HeartbeatHandle::new(), &RunScope::default().with_run_id(run.run_id.clone()))`. For any port
  that never overrides `execute_scoped`, the trait's default body delegates straight to
  `execute`, so this is behavior-identical when no ledger is installed --
  `pool_without_a_treasury_ledger_settles_nothing` and every pre-existing worker test prove it.
- **Four new worker tests** (`worker.rs`'s own `mod tests`): a local `RecordingTreasuryLedger`
  (records every `settle` call's key/amount, delegating to a real `InMemoryTreasuryLedger`,
  mirroring 39-04's identical crate-private double, re-implemented here since that one is
  unreachable from this facade crate), a `PricedPaladinPort` (always returns 45,000,000 nanos
  USD), and a `ScopeRecordingPaladinPort` (records the `RunScope` handed to `execute_scoped`):
  `engine_run_settles_under_its_run_id_and_first_attempt`,
  `redelivered_running_run_settles_under_the_bumped_attempt`,
  `agent_kind_run_passes_its_run_id_in_the_run_scope`,
  `pool_without_a_treasury_ledger_settles_nothing`.
- **`priced_run_cost_reaches_get_run_through_the_ledger`** (`tracer_e2e.rs`): a harness with its
  own `PricedPaladinPort` and a priced, `"gpt-4"`-modeled Paladin graph, a worker built
  `.with_engine_factory(..)` and `.with_treasury_ledger(ledger)`, and a `RunApiState`
  `.with_treasury_ledger(ledger)` over the SAME `InMemoryTreasuryLedger`. Submits over HTTP,
  runs the worker once, then asserts `GET /v1/runs/{id}` reports `status: "completed"` and
  `cost: { nanos: 45000000, currency: "USD", display: "0.0450 USD" }`.
- **`build_treasury_ledger`** (`src/infrastructure/web/run_api_wiring.rs`, `pub async fn`):
  shares `RunStoreConfig`'s own backend selection (D-00g) -- `Disabled` -> `Ok(None)`,
  `Sqlite { path }` -> a `SqliteTreasuryLedger` over that exact file, `Postgres { url_env }` -> a
  `PostgresTreasuryLedger` over that exact database on a `storage-postgres` build (a
  named-feature error otherwise, mirroring `build_postgres_quartet`'s own precedent and error
  wording verbatim). `build_run_api` calls it right after the repository quartet, builds the run
  engine's `PaladinPort` through `paladin_port_from_settings_with_ledger` in place of the
  ledger-less call, and adds `.with_treasury_ledger` to both the `RunWorkerPool` chain and the
  `RunApiState` chain when `Some`. The `Disabled` early return is untouched.
- **`paladin-server.rs` boot order**: `RunStoreConfig` construction (`default` +
  `apply_env_overrides` + `validate`) moved ahead of the agent registry; `treasury_ledger =
  build_treasury_ledger(&run_store_config).await?` built once; `build_agent_registry(&settings)`
  replaced with `build_agent_registry_with_ledger(&settings, treasury_ledger.clone())`;
  `FacadeProvisioner::from_settings(&settings)` gains `.with_treasury_ledger(ledger)` when
  `Some`. The same `run_store_config` still flows into `RunApiConfigs` unchanged, so the run API
  builds its own ledger handle over the exact same store.
- **Three new `run_api_wiring` tests**: `build_treasury_ledger_disabled_is_none`,
  `build_treasury_ledger_sqlite_round_trips` (a temp-file SQLite backend, an unreserved settle,
  then `spend` reads it back), and `build_run_api_wires_the_treasury_ledger` (a dedicated test
  proving both halves of the D-08 contract by name: a configured sqlite run store wires
  `Some`, a disabled one wires `None`). The two pre-existing `build_run_api` tests
  (`defaults_wire_nothing_and_answer_501`, `sqlite_and_in_memory_wires_three_tasks_and_every_state_field`)
  each gained one more assertion on `handles.run_state.treasury_ledger`.
- Full verification: `cargo test -p paladin-ai --features web-server --lib
  application::services::run` -- 148 passed, 0 failed; `cargo test -p paladin-ai --features
  web-server --lib infrastructure::web::run_api_wiring` -- 8 passed, 0 failed; `cargo test -p
  paladin-ai --features web-server --bin paladin-server` -- 14 passed, 0 failed; `cargo build -p
  paladin-ai --features "web-server storage-postgres" --bin paladin-server` -- succeeds; `cargo
  clippy -p paladin-ai --features web-server --all-targets -- -D warnings` and the same with
  `storage-postgres` added -- both clean; `cargo fmt --check` -- clean.

## Task Commits

1. **Task 1: Worker attaches the ledger per run with the persisted attempt; agent-kind runs
   carry their run id; end-to-end HTTP proof** - `09c1841a` (test)
2. **Task 2: Build the ledger from RunStoreConfig in build_run_api and paladin-server** -
   `294d4a38` (feat)

**Plan metadata:** pending (this commit)

## Files Created/Modified

- `src/application/services/run/worker.rs` - `RunWorkerPool::with_treasury_ledger`, the
  `run_once` status-match attempt threading, the factory-branch engine attachment,
  `run_agent`'s `execute_scoped` switch, four new tests plus their local test doubles
- `src/application/services/run/tracer_e2e.rs` - `PricedPaladinPort`, `build_priced_graph`,
  `priced_run_cost_reaches_get_run_through_the_ledger`
- `src/infrastructure/web/run_api_wiring.rs` - `build_treasury_ledger` + its
  `cfg(storage-postgres)` pair, the `build_run_api` wiring (ledger build,
  `paladin_port_from_settings_with_ledger`, pool/`RunApiState` `.with_treasury_ledger`), three
  new tests plus two extended assertions
- `src/bin/paladin-server.rs` - `RunStoreConfig` moved ahead of the agent registry,
  `build_treasury_ledger` call, `build_agent_registry_with_ledger` in place of
  `build_agent_registry`, `FacadeProvisioner::with_treasury_ledger` wiring

## Decisions Made

- `build_run_api_wires_the_treasury_ledger` added as its own dedicated test (see key-decisions)
  so the plan's literal acceptance criterion -- that exact test name reported passing -- is
  satisfied precisely, while the two pre-existing `RunApiState`-field tests still gained the same
  assertion the plan's action text asked for.
- Every new `TreasuryLedgerPort`/`PaladinPort` test double needed for this plan's own tests is
  defined locally (worker.rs's `mod tests`, tracer_e2e.rs) rather than imported from
  `paladin-battalion`'s identical 39-04 doubles, which are crate-private to that crate's own test
  module and unreachable from this facade crate.
- `priced_run_cost_reaches_get_run_through_the_ledger` uses a minimal, directly-priced
  `PaladinPort` rather than routing through `PaladinExecutionService`/`MockLlmAdapter` -- simpler
  and more direct for proving the engine-to-ledger-to-HTTP chain with one fixed cost value.

## Deviations from Plan

None - plan executed exactly as written. Both tasks' `<behavior>` clauses map one-to-one onto
the tests added, and every acceptance-criteria grep in the plan passes verbatim (`with_treasury_ledger`,
`SettlementContext {`, `LedgerScope::unattributed()`, `execute_scoped(`,
`with_run_id(run.run_id.clone())` in `worker.rs`; `priced_run_cost_reaches_get_run_through_the_ledger`
and `0.0450 USD` in `tracer_e2e.rs`; `build_treasury_ledger`, `paladin_port_from_settings_with_ledger`,
`SqliteTreasuryLedger::new`, `PostgresTreasuryLedger::new` and a zero-count grep for the ledger-less
`paladin_port_from_settings(settings)` call in `run_api_wiring.rs`; `build_treasury_ledger(&run_store_config)`,
`build_agent_registry_with_ledger` and `with_treasury_ledger` in `paladin-server.rs`).

## Issues Encountered

- The sandbox's root filesystem ran low on disk space during this plan's build/test cycles
  (mirroring the identical issue recorded in every other 39-* plan's SUMMARY). Cleared with `rm
  -rf target/debug/incremental` (safe: regenerable, no source-of-truth data) twice during this
  plan. Not a code or plan issue -- environment housekeeping only, not reintroduced by either
  commit. `cargo audit`/`cargo deny` were not re-run this plan: `Cargo.lock`/`Cargo.toml` are
  untouched (`git status --short` confirms no diff), so their results are unaffected by this
  plan's changes and re-running them would only have cost disk headroom this environment does not
  have to spare.

## User Setup Required

None - no external service configuration required. Every new test uses only the in-memory or a
temp-file SQLite adapter (both already normal, non-dev dependencies of the root crate via
`paladin-storage`).

## Next Phase Readiness

- Both production run paths now fill the ledger in ordinary operation (D-08): the engine's
  superstep boundary (39-04) receives the real per-dispatch `SettlementContext` this plan
  computes from `RunRepositoryPort`, and the agent loop's `PlatformRunsOnly` writer (39-05)
  finally receives a real `RunScope.run_id` for an agent-kind Platform run, closing the gap those
  two plans left open for exactly this plan to fill.
- LEDGR-03 (redelivery/resume produce new keys, duplicate settlements charge once) and LEDGR-04
  (the ledger fills in ordinary operation, cost visible over HTTP end-to-end) are now proven
  through production code, not just the engine/agent-loop unit tests those prior plans shipped.
- `make api-surface`/CHANGELOG updates remain explicitly owned by plan 39-08 (D-00f), consistent
  with every prior plan in this phase -- not touched here. Every new public item is additive
  (`RunWorkerPool::with_treasury_ledger`, `build_treasury_ledger`); no existing public signature
  changed.
- `requirements.ready-ids` will correctly hold LEDGR-03/LEDGR-04 at not-yet-`Complete` until
  39-08 (which also declares both IDs) produces its own SUMMARY -- expected, not a gap in this
  plan.
- No blockers for 39-08.

---
*Phase: 39-spend-ledger*
*Completed: 2026-09-28*
