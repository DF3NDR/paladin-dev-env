---
phase: 39-spend-ledger
plan: 04
subsystem: database
tags: [treasurer, ledger, engine, superstep, hexagonal-ports, adr-0052, adr-0053]

# Dependency graph
requires:
  - phase: 39-spend-ledger
    provides: "39-02: TreasuryLedgerPort's full surface (reserve/release/settle/spend/store_now)
      and InMemoryTreasuryLedger, a dev-dependency-only adapter used by this plan's own
      engine-path tests"
provides:
  - "crates/paladin-battalion/src/engine/settlement.rs -- SuperstepSpend (per-attempt
    accumulator with model breakdown and currency-mismatch detection) and SpendHook
    (top_level/child/record/settle_boundary), crate-private"
  - "WarEngine::with_treasury_ledger(ledger, SettlementContext) and the resulting spend_hook()
    threaded through every start/resume*/replay/fork call into superstep::run/run_with_namespace"
  - "The superstep-boundary settle: hook.record() beside every attempt's own NodeFinished emit,
    hook.settle_boundary(superstep_number).await immediately after completed_records is sorted
    and before any outcome branch persists that superstep's Waypoint"
  - "ChildEngineResources.spend -- a NodeSpec::Battalion child run inherits a CHILD hook
    (SpendHook::child) so its own attempts roll into the PARENT superstep's accumulator and its
    own settle_boundary calls are no-ops"
affects: [39-05, 39-06, 39-07, 39-08]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Settlement is synchronous and awaited at the superstep boundary, never routed through the
      TraceDispatcher/TraceSink drop-oldest channel -- money is the one thing this engine never
      treats as best-effort"
    - "SpendHook::child shares the SAME Arc<Mutex<SuperstepSpend>> as its parent but carries no
      ledger (ledger: None) -- a child's settle_boundary is a no-op by construction, so a nested
      Battalion run never settles on its own; only the top-level hook a run was given ever
      actually calls TreasuryLedgerPort::settle"
    - "The mutex guard around SuperstepSpend::take() is dropped BEFORE any .await -- settle_boundary
      never holds a std::sync::Mutex across an await point"

key-files:
  created:
    - crates/paladin-battalion/src/engine/settlement.rs
  modified:
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/src/engine/superstep.rs
    - crates/paladin-battalion/src/engine/graph.rs

key-decisions:
  - "Task 1's own clippy -D warnings gate would have failed on SpendHook::child being
    unreachable production dead code (its only production call site is Task 2's
    ChildEngineResources construction) -- resolved with a documented #[allow(dead_code)] on
    Task 1's commit, removed in Task 2's commit once the real call site exists. A deliberate,
    minimal Rule 3 auto-fix to unblock Task 1's own standalone verification, not a plan gap left
    unresolved."
  - "Every test-only call site of superstep::run/run_with_namespace across superstep.rs, mod.rs
    and graph.rs (18 total: 4 production WarEngine call sites plus 14 test helpers/inline calls)
    needed a trailing argument added for the new spend parameter -- the plan's read_first pointers
    named only the production call sites near lines 2055/2297/2642/2811; the full sweep across
    every superstep::run/run_with_namespace call site (test helpers included) was necessary to
    keep the crate compiling, verified by grepping every call site rather than trusting the
    named line numbers alone."
  - "The one-settle-call assertions in the new engine tests (nested Battalion roll-up, the
    parallel+retry invariant) use a RecordingTreasuryLedger that counts actual settle() CALLS,
    not settled ROWS -- SettleOutcome alone cannot distinguish a genuine single call from a
    duplicate-key AlreadySettled call, so counting rows would not have proven ADR-0053 SS4's
    exactly-once-per-superstep-attempt invariant as precisely."

patterns-established:
  - "dispatch_paladin_model<W>(&NodeDispatch<W>) -> Option<String> is the one place a dispatch
    entry's configured model is read for the ledger breakdown key -- computed once per dispatch,
    before the entry is moved into its spawned task, mirroring node_trace/node_structured_executor's
    existing clone-out-of-scope pattern"

requirements-completed: [LEDGR-03, LEDGR-04]

coverage:
  - id: D1
    description: "SuperstepSpend accumulates every priced attempt's Cost and per-model share
      within one superstep, detects a currency mismatch without combining figures, and take()
      resets to empty"
    requirement: "LEDGR-04"
    verification:
      - kind: unit
        ref: "engine::settlement::tests::record_accumulates_amount_and_per_model_breakdown_then_take_resets"
        status: pass
      - kind: unit
        ref: "engine::settlement::tests::mismatched_currencies_report_a_mismatch_without_combining"
        status: pass
      - kind: unit
        ref: "engine::settlement::tests::resolve_model_key_maps_none_and_empty_to_unknown"
        status: pass
    human_judgment: false
  - id: D2
    description: "WarEngine::with_treasury_ledger attaches a ledger; the engine settles exactly
      once per superstep attempt, synchronously, at the boundary -- before any outcome branch
      persists that superstep's Waypoint"
    requirement: "LEDGR-03"
    verification:
      - kind: unit
        ref: "engine::tests::treasury_ledger_settles_once_per_superstep_with_model_breakdown"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-battalion --doc with_treasury_ledger"
        status: pass
    human_judgment: false
  - id: D3
    description: "A superstep with no priced attempt writes no settlement row; an engine with no
      treasury ledger attached performs no ledger call at all and every pre-existing engine test
      passes unchanged"
    requirement: "LEDGR-03"
    verification:
      - kind: unit
        ref: "engine::tests::unpriced_superstep_writes_no_settlement"
        status: pass
      - kind: unit
        ref: "engine::tests::engine_without_treasury_ledger_is_unchanged"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-battalion --lib engine (562 passed, 0 failed)"
        status: pass
    human_judgment: false
  - id: D4
    description: "A NodeSpec::Battalion child run's own priced attempts, across however many
      child supersteps it takes, roll into the ONE parent superstep that dispatched it -- never
      settling on their own"
    requirement: "LEDGR-03"
    verification:
      - kind: unit
        ref: "engine::tests::nested_battalion_child_spend_rolls_into_the_parent_superstep"
        status: pass
    human_judgment: false
  - id: D5
    description: "One settlement per superstep attempt holds across parallel priced Paladin
      nodes and an Aegis-retried node in the same superstep -- the settled amount sums every
      priced attempt, including the retry's successful one, never the failed first attempt"
    requirement: "LEDGR-03"
    verification:
      - kind: unit
        ref: "engine::tests::one_settlement_per_superstep_attempt_across_parallel_nodes_and_retries"
        status: pass
    human_judgment: false
  - id: D6
    description: "A ledger settle failure (TreasuryLedgerError::Backend) never fails, retries a
      node in, or halts the run -- RunOutcome::Completed and TraceEvent::RunFinished are still
      produced"
    requirement: "LEDGR-03"
    verification:
      - kind: unit
        ref: "engine::tests::failing_treasury_ledger_never_fails_the_run"
        status: pass
    human_judgment: false
  - id: D7
    description: "A pre-existing settlement at a superstep-attempt key is kept unchanged
      (AlreadySettled, not an error) when the engine settles that same key again; a later
      superstep's settlement is added on top"
    requirement: "LEDGR-03"
    verification:
      - kind: unit
        ref: "engine::tests::already_settled_superstep_is_not_an_error"
        status: pass
    human_judgment: false
  - id: D8
    description: "cargo check -p paladin-battalion (no dev-dependencies) exits 0 -- paladin-storage
      is never imported outside tests, preserving the hexagonal boundary (paladin-battalion must
      not gain a runtime dependency on paladin-storage)"
    human_judgment: false
    verification:
      - kind: other
        ref: "cargo check -p paladin-battalion"
        status: pass

duration: ~55min
completed: 2026-09-27
status: complete
---

# Phase 39 Plan 04: Engine-Path Treasury Ledger Settlement Summary

**`WarEngine::with_treasury_ledger` settles exactly one aggregated ledger row per superstep attempt, synchronously at the superstep boundary, including nested Battalion runs, parallel nodes and Aegis retries -- observational only, never affecting a run's outcome.**

## Performance

- **Duration:** ~55 min
- **Tasks:** 2 (both `type="auto" tdd="true"`, no checkpoints)
- **Files modified:** 4 (1 created, 3 modified)

## Accomplishments

- **`crates/paladin-battalion/src/engine/settlement.rs`** (new, crate-private): `SuperstepSpend`
  (an accumulator combining every recorded attempt's `Cost` through `Cost::checked_add`, tracking
  a per-model `BTreeMap<String, i64>` breakdown and the first currency disagreement it sees,
  never a second) and `SpendHook` (`top_level`/`child`/`record`/`settle_boundary`). `record`
  resolves a `None`/empty model to the literal key `"unknown"`. `settle_boundary` drops its mutex
  guard before any `.await`, calls `TreasuryLedgerPort::settle` with an unreserved
  `SettleRequest` only when a real `Charge` was accumulated, and logs (never propagates) a
  `TreasuryLedgerError` at `error` or a duplicate-key `AlreadySettled` at `warn` -- log lines
  carry the run id, superstep, attempt, nanos and currency, never `api_key_id`. Four unit tests,
  one exercising a real `InMemoryTreasuryLedger` settlement to prove a child hook's
  `settle_boundary` performs no ledger call and leaves the shared accumulator intact for the
  parent's own boundary to drain.
- **`WarEngine::with_treasury_ledger(ledger, context)`** (`engine/mod.rs`): a private
  `treasury: Option<(Arc<dyn TreasuryLedgerPort>, SettlementContext)>` field (initialised `None`
  in the sole constructor) and a private `spend_hook()` helper mapping it through
  `SpendHook::top_level`. Every production call of `superstep::run`/`run_with_namespace` -- `start`,
  `resume_with_options`, `resume_with`, and `replay`/`fork`'s shared `replay_or_fork` -- forwards
  `self.spend_hook()`; every test-only call site (18 total, across `superstep.rs`'s own test
  module, `mod.rs`'s tests, and `graph.rs`'s `run_to_completion` helper) forwards `None`. Full
  rustdoc states the settlement key, the D-07 `attempt`-is-`Run.attempt` contract, the
  fallback-hop model-attribution simplification, and the observational-failure guarantee, with a
  compiling `# Examples` doctest building an `InMemoryTreasuryLedger`-backed engine.
- **`superstep.rs`'s boundary wiring**: a trailing `spend: Option<SpendHook>` parameter on both
  `run` (forwarded) and `run_with_namespace`; `dispatch_paladin_model<W>` resolves a dispatch
  entry's configured model (`Some(paladin.node.model.clone())` for `NodeSpec::Paladin`, `None`
  otherwise) once, before the dispatch entry is moved into its spawned task. Inside the retry
  loop, immediately after each attempt's own `TraceEvent::NodeFinished` emit, `hook.record(model,
  cost)` folds that attempt's priced cost into the shared accumulator (a failed attempt's `cost`
  is always `None`, so nothing is recorded for it -- only the eventual successful retry
  contributes). Immediately after `completed_records.sort_by(...)`, `hook.settle_boundary(superstep_number).await`
  settles the whole superstep -- before any outcome branch (including the engine-run-timeout
  path) persists that superstep's Waypoint.
- **`ChildEngineResources.spend`** (Task 2): a `Option<SpendHook>` field, constructed once at
  `child_resources`'s single construction site as `spend.as_ref().map(SpendHook::child)`. The
  `NodeSpec::Battalion` dispatch arm's recursive `run_with_namespace` call forwards
  `resources.spend.clone()` -- a CHILD hook, never the parent's own top-level instance -- so a
  nested Battalion run's attempts (across however many child supersteps it takes, all awaited
  inline within the ONE parent dispatch) fold into the PARENT superstep's accumulator, and the
  child's own `settle_boundary` calls are no-ops.
- **Seven new engine tests** across both tasks: `treasury_ledger_settles_once_per_superstep_with_model_breakdown`,
  `unpriced_superstep_writes_no_settlement`, `engine_without_treasury_ledger_is_unchanged` (Task 1);
  `nested_battalion_child_spend_rolls_into_the_parent_superstep`,
  `one_settlement_per_superstep_attempt_across_parallel_nodes_and_retries`,
  `failing_treasury_ledger_never_fails_the_run`, `already_settled_superstep_is_not_an_error`
  (Task 2). Three new test doubles: `RecordingTreasuryLedger` (records every `SettleRequest`'s
  key and amount in call order, delegating to a real `InMemoryTreasuryLedger`),
  `FailingTreasuryLedger` (`settle` always returns `TreasuryLedgerError::Backend`), and
  `FlakyOnceThenRecordingPort` (fails one named Paladin's first call, delegates every other call
  including that Paladin's own retried second attempt to a wrapped `RecordingPaladinPort`).
- Full suite: `cargo test -p paladin-battalion --lib engine` reports **562 passed, 0 failed**
  (up from 555 before this plan); `cargo check -p paladin-battalion` (no dev-dependencies) exits
  0; `cargo test -p paladin-battalion --doc with_treasury_ledger` passes; `cargo clippy -p
  paladin-battalion --all-targets -- -D warnings` and `cargo fmt --check -p paladin-battalion`
  are both clean.

## Task Commits

1. **Task 1: Superstep spend accumulator, WarEngine::with_treasury_ledger, and the boundary
   settle** - `8e975d5e` (test)
2. **Task 2: Nested Battalion roll-up, retry invariant, and failure isolation** - `431904e5`
   (feat)

**Plan metadata:** pending (this commit)

## Files Created/Modified

- `crates/paladin-battalion/src/engine/settlement.rs` - `SuperstepSpend`, `SuperstepCharge`,
  `SpendHook` (new, crate-private)
- `crates/paladin-battalion/src/engine/mod.rs` - `WarEngine::with_treasury_ledger`, private
  `treasury` field and `spend_hook()`, every `start`/`resume*`/`replay`/`fork` call site
  forwarding it, 7 new engine tests plus 3 test doubles
- `crates/paladin-battalion/src/engine/superstep.rs` - `spend: Option<SpendHook>` on
  `run`/`run_with_namespace`, `dispatch_paladin_model`, per-attempt `hook.record()`, the boundary
  `hook.settle_boundary(superstep_number).await`, `ChildEngineResources.spend`
- `crates/paladin-battalion/src/engine/graph.rs` - `run_to_completion` test helper forwards
  `None` for the new `spend` parameter

## Decisions Made

- Task 1's `#[allow(dead_code)]` on `SpendHook::child` (see key-decisions above) -- removed in
  Task 2's own commit once `ChildEngineResources.spend`'s construction gave it a real production
  call site.
- Every one of the 18 `superstep::run`/`run_with_namespace` call sites across the crate (not
  just the 4 named production sites) needed the new trailing parameter -- found by grepping every
  call site rather than trusting the plan's named line numbers alone, since 14 test-only helpers
  and inline calls (`superstep.rs`'s own test module, `mod.rs`'s two tracer-style tests,
  `graph.rs`'s `run_to_completion`) also call these functions directly.
- `RecordingTreasuryLedger` counts actual `settle()` CALLS (not settled rows) so the
  one-settlement-per-superstep-attempt tests prove the exact invariant ADR-0053 §4 states, rather
  than a weaker "the final row count is right" proxy that a duplicate-key `AlreadySettled` call
  could satisfy without actually proving single-call behavior.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `SpendHook::child` was unreachable production dead code after Task 1
alone**
- **Found during:** Task 1's own `cargo clippy -p paladin-battalion --all-targets -- -D warnings`
  verification step
- **Issue:** The plan's Task 1 action text deliberately has the `NodeSpec::Battalion` dispatch
  arm pass `None` for the new `spend` parameter ("Task 2 threads the child hook"), so
  `SpendHook::child` -- defined and exercised only by `settlement.rs`'s own unit tests -- has no
  real production call site until Task 2. `clippy --all-targets` builds the plain (non-test) lib
  target too, where `child` is genuinely unreachable, so `-D warnings` promoted the resulting
  `dead_code` lint to a hard error, blocking Task 1's own acceptance gate ("the verify command
  exits 0").
- **Fix:** Added a documented `#[allow(dead_code)]` on `child` in Task 1's commit, stating
  explicitly that Task 2 wires the real call site and removes the attribute -- which Task 2's
  commit then did.
- **Files modified:** `crates/paladin-battalion/src/engine/settlement.rs`
- **Verification:** `cargo clippy -p paladin-battalion --all-targets -- -D warnings` clean after
  Task 1; clean again after Task 2 with the attribute removed and a genuine call site in place.
- **Commit:** `8e975d5e` (added), `431904e5` (removed)

**2. [Rule 1 - Bug] The `already_settled_superstep_is_not_an_error` test's pre-settle request had
an empty `model_breakdown` against a non-zero amount**
- **Found during:** Task 2's own test run
- **Issue:** `paladin_storage::treasury::validate_settle` rejects a `SettleRequest` whose
  `model_breakdown` values do not sum to `amount` when `amount != 0` -- the test's first draft
  passed `BTreeMap::new()` against `Cost::new(1, usd)`, so the pre-settle call itself failed with
  `InvalidRequest`, before the engine ever ran.
- **Fix:** Gave the pre-settle request a matching one-entry breakdown (`{"gpt-4": 1}`).
- **Files modified:** `crates/paladin-battalion/src/engine/mod.rs` (test-only)
- **Verification:** `cargo test -p paladin-battalion --lib engine` -- the test passes.
- **Commit:** `431904e5`

---

**Total deviations:** 2 auto-fixed (1 blocking clippy gate, 1 bug in new test-only code).
**Impact on plan:** Both fixes are necessary for the plan's own verification gates to pass; no
scope creep, no production-code behavior changed beyond what the plan specified.

## Issues Encountered

- The sandbox's root filesystem ran low on disk space during this plan's build/test cycles
  (mirroring the identical issue recorded in 39-01's and 39-03's SUMMARYs). Cleared with `rm -rf
  target/debug/incremental` (safe: regenerable, no source-of-truth data) once mid-plan. Not a
  code or plan issue -- environment housekeeping only, not reintroduced by either commit.

## User Setup Required

None - no external service configuration required. Both new tests and the doctest use only the
in-memory adapter (dev-dependency).

## Next Phase Readiness

- The engine path now has a complete, tested settle-only production writer at the superstep
  boundary (D-08, ADR-0052): `WarEngine::with_treasury_ledger` is the one attachment point
  Phase 41 (allowance/ceiling admission) and Phase 42 (mid-run reservation and halt) will extend
  at the SAME call site, per the phase's own D-08 design note.
- `make api-surface`/CHANGELOG updates remain explicitly owned by plan 39-08 (D-00f), consistent
  with 39-01/39-02/39-03's own disposition -- not touched here. `WarEngine::with_treasury_ledger`
  is a new, additive public method; no MIGRATION §9.2 row is expected (X-03).
- 39-05 (the agent-loop settle writer inside `PaladinExecutionService`) is unblocked: the ledger
  port, the `SettlementContext`/`SpendHook` shapes, and the observational-failure discipline this
  plan established are all reusable as-is -- the agent loop settles per priced model call rather
  than per superstep, but the same `TreasuryLedgerPort::settle`/logging contract applies
  unchanged.
- No blockers.

---
*Phase: 39-spend-ledger*
*Completed: 2026-09-27*

## Self-Check: PASSED

`crates/paladin-battalion/src/engine/settlement.rs` verified present on disk; commits `8e975d5e`
and `431904e5` verified present in git history (`git log --oneline --all | grep -E
"8e975d5e|431904e5"` both found).
