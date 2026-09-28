---
phase: 39-spend-ledger
plan: 06
subsystem: api
tags: [axum, utoipa, openapi, treasurer, ledger, hexagonal-ports, adr-0053, d-10]

# Dependency graph
requires:
  - phase: 39-spend-ledger
    provides: "39-01/39-02/39-03: TreasuryLedgerPort's spend(SpendQuery) grouped by
      tenant/api-key/run/model, proven on the in-memory/SQLite/Postgres adapters;
      39-04/39-05: production settle writers filling the ledger on the engine and
      agent-loop paths so a GET /runs/{id} or POST /agents/{id}/execute call can
      observe real settled spend"
provides:
  - "CostDto{nanos, currency, display} (crates/paladin-web/src/run_controller.rs) --
    the one wire projection of paladin_core::cost::Cost every HTTP response in this
    crate uses, display produced by treasury_ledger::format_cost"
  - "RunApiState.treasury_ledger + with_treasury_ledger; RunResponse.cost /
    RunListResponse item cost derived from the ledger at read time (D-10), one
    spend() call per request/page, degrading to null on any ledger problem (D-08)"
  - "ExecuteResponse.cost from PaladinResult.cost via CostDto::from -- the Phase 38
    deferral (execute_response_carries_no_cost_field) is inverted"
  - "Regenerated crates/paladin-web/openapi.json baseline (CostDto schema, cost on
    RunResponse/ExecuteResponse) and a second sanctioned ExecuteResponse exception
    in the SHIP-02 golden v0.9 diff (openapi_golden_v0_9.rs), alongside the Phase 31
    usage/token_count one"
affects: [39-07, 39-08]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "run_costs(ledger, run_ids) -> HashMap<String, CostDto> is the ONE place both
      get_run and list_runs call TreasuryLedgerPort::spend -- always
      SpendGroupBy::Run, always one call per request/page, always degrading to an
      empty map (never an Err) on a ledger failure or a run whose settlements span
      more than one currency"
    - "A second SHIP-02 'sanctioned exception' entry alongside the Phase 31
      usage/token_count one: adding a genuinely new, additive field to
      ExecuteResponse still enters the six-v0.9-path $ref closure the golden diff
      guards, so extending that gate (not disabling it) is the correct response to
      an intentional, planned field addition on a pre-existing path"

key-files:
  created: []
  modified:
    - crates/paladin-web/src/run_controller.rs
    - crates/paladin-web/src/agent_controller.rs
    - crates/paladin-web/openapi.json
    - crates/paladin-web/tests/openapi_golden_v0_9.rs

key-decisions:
  - "crates/paladin-web/tests/openapi_golden_v0_9.rs (SHIP-02's frozen v0.9.0 golden
    diff, not named in the plan's files_modified) required a second sanctioned
    ExecuteResponse exception -- cost/CostDto -- mirrored on the existing Phase 31
    usage/token_count exception, because the new additive cost field still enters
    the six-v0.9-path $ref closure that test protects. Not optional: `cargo test -p
    paladin-web` (the plan's own <verify> command) fails without it."
  - "MockRepository (run_controller.rs test module) gained a list_items field/
    set_list_page setter so list_runs_derives_costs_with_one_spend_call_per_page
    could seed a real three-run page -- the pre-existing double always answered an
    empty page regardless of RunQuery, which cannot exercise a per-page batch spend
    call."

patterns-established:
  - "CostDto::from(&Cost) is the crate's only Cost-to-wire conversion; both
    RunResponse and ExecuteResponse route through it so display formatting can
    never drift between the two response types."

requirements-completed: [LEDGR-04]

coverage:
  - id: D1
    description: "GET /runs/{run_id} returns cost derived from exactly one
      TreasuryLedgerPort::spend(SpendGroupBy::Run) call, null when unwired, when
      the ledger errors, or when the run's settlements span two currencies"
    requirement: "LEDGR-04"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-web --lib run_controller::tests::get_run_includes_ledger_cost"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-web --lib run_controller::tests::get_run_cost_is_null_without_a_ledger"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-web --lib run_controller::tests::get_run_cost_is_null_when_the_ledger_errors"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-web --lib run_controller::tests::get_run_cost_is_null_for_mixed_currencies"
        status: pass
    human_judgment: false
  - id: D2
    description: "GET /runs derives every item's cost from exactly ONE spend call
      per page (run_ids = the page's ids), never one call per item"
    requirement: "LEDGR-04"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-web --lib run_controller::tests::list_runs_derives_costs_with_one_spend_call_per_page"
        status: pass
    human_judgment: false
  - id: D3
    description: "POST /agents/{id}/execute (and every ExecuteResponse::from use)
      carries cost from PaladinResult.cost, null when unpriced; the Phase 38
      deferral test is inverted with an updated doc comment"
    requirement: "LEDGR-04"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-web --lib agent_controller::tests::execute_response_carries_cost_when_priced"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-web --lib agent_controller::tests::execute_response_cost_is_null_when_unpriced"
        status: pass
    human_judgment: false
  - id: D4
    description: "The OpenAPI baseline documents CostDto/cost, and the SHIP-02
      frozen v0.9.0 golden diff still passes with a narrowly-scoped second
      exception for the new field; the Phase 38 SSE no-spend-field regression is
      unchanged"
    requirement: "LEDGR-04"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-web --lib openapi_matches_committed_baseline"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-web --test openapi_golden_v0_9"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ai --lib application::services::run::events::tests::sse_payloads_carry_no_spend_field"
        status: pass
    human_judgment: false

duration: ~20min
completed: 2026-09-28
status: complete
---

# Phase 39 Plan 06: Ledger-Derived HTTP Cost Summary

**`GET /runs/{id}`/`GET /runs` and `POST /agents/{id}/execute` now carry a shared `CostDto` -- run cost derived from `TreasuryLedgerPort::spend` at read time (one query per request/page, never persisted, null on any ledger problem), and the agent execute response finally exposes `PaladinResult.cost`, inverting the Phase 38 deferral test that named this phase.**

## Performance

- **Duration:** ~20 min
- **Tasks:** 2 (both `type="auto"`/`tdd="true"`, no checkpoints)
- **Files modified:** 4 (0 created, 4 modified)

## Accomplishments

- **`CostDto`** (`crates/paladin-web/src/run_controller.rs`): `{ nanos: i64, currency:
  String, display: String }` with `impl From<&Cost>`, `display` produced by
  `treasury_ledger::format_cost` -- byte-identical to the herald and CLI rendering
  (D-00c/D-04). The crate's one Cost-to-wire conversion; both `RunResponse` and
  `ExecuteResponse` route through it.
- **Ledger-derived run cost (D-10, D-08)**: `RunApiState.treasury_ledger: Option<Arc<dyn
  TreasuryLedgerPort>>` + `with_treasury_ledger`; `RunResponse.cost: Option<CostDto>`
  (`None` in `From<&Run>`, filled by the handler). Private `run_costs(ledger, run_ids)`
  makes exactly ONE `spend(SpendQuery { group_by: SpendGroupBy::Run, run_ids, .. })`
  call, keeps a run's entry only when its rows resolve to exactly one currency, and on
  `Err` logs `log::warn!` (run count + error, never the query itself) and returns an
  empty map -- a ledger problem degrades `cost` to `null`, it never fails the read.
  `get_run` calls it with one id; `list_runs` calls it once with the whole page's ids
  and fills each item (T-39-17).
- **`ExecuteResponse.cost`** (`crates/paladin-web/src/agent_controller.rs`): `Option<CostDto>`
  mapped via `result.cost.as_ref().map(CostDto::from)` in `From<PaladinResult>`. Every
  existing `ExecuteResponse::from` call site (`execute_agent`, `enqueue_job`, the SSE
  fallback's `done` event) picks this up automatically since they all construct through
  the same `From` impl.
- **Five new `run_controller` tests** against a local `StubTreasuryLedger` double
  (`spend` returns a canned outcome and records every call; every other method returns
  an inert `Err` so a stray call fails loudly) proving: a priced single-currency run's
  cost, `null` with no ledger wired, `null` on a ledger `Err`, `null` for a run whose
  settlements span two currencies, and exactly one `spend` call per `list_runs` page
  with the query's `run_ids` matching the page and each item resolving its own cost
  (one item with no row -> `null`). `MockRepository` gained a `list_items`/
  `set_list_page` seam so the list test could exercise a real three-run page (the
  pre-existing double always answered empty regardless of `RunQuery`).
- **Two new `agent_controller` tests** replacing the inverted Phase 38 regression:
  `execute_response_carries_cost_when_priced` (priced result -> `cost.nanos`/`currency`/
  `display` all correct) and `execute_response_cost_is_null_when_unpriced` (`cost: None`
  -> `null`), with the test's doc comment rewritten to cite Phase 39 D-10/LEDGR-04
  instead of the old "Phase 39 LEDGR-04, not this phase" deferral note.
- **Regenerated `crates/paladin-web/openapi.json`** (`make openapi`): adds the `CostDto`
  schema and `cost` on `RunResponse`/`ExecuteResponse`. `openapi_matches_committed_baseline`
  passes without `UPDATE_OPENAPI` set.
- **Extended `crates/paladin-web/tests/openapi_golden_v0_9.rs`** (SHIP-02's frozen
  v0.9.0 golden diff -- not in the plan's `files_modified`, discovered by running the
  plan's own `<verify>` command) with a second sanctioned `ExecuteResponse` exception,
  mirroring the existing Phase 31 `usage`/`token_count` one: `cost` is additive and
  genuinely new (never renamed, absent from the frozen baseline entirely), but it still
  enters the six-v0.9-path `$ref` closure the gate protects. `strip_known_v0_10_execute_response_divergence`
  now strips `cost`/`CostDto` alongside `usage`/`token_count`/`TokenUsageResponse`, and
  `execute_response_exception_is_narrowly_scoped` asserts the new exception is exactly
  as narrow as the old one (every other `ExecuteResponse` field still survives; `cost`
  and `CostDto` are the only new removals).

## Task Commits

1. **Task 1: CostDto and ledger-derived cost on GET /runs/{id} and GET /runs** -
   `8f9dcbfb` (test)
2. **Task 2: ExecuteResponse.cost, the inverted Phase 38 regression test, and the
   OpenAPI baseline** - `2f701d2c` (feat)

**Plan metadata:** pending (this commit)

## Files Created/Modified

- `crates/paladin-web/src/run_controller.rs` - `CostDto`, `RunApiState.treasury_ledger`/
  `with_treasury_ledger`, `RunResponse.cost`, `run_costs`, `get_run`/`list_runs` wiring,
  `StubTreasuryLedger` test double, five new tests
- `crates/paladin-web/src/agent_controller.rs` - `ExecuteResponse.cost`, `CostDto`
  import, inverted regression test pair
- `crates/paladin-web/openapi.json` - regenerated baseline (`CostDto`, `cost` on
  `RunResponse`/`ExecuteResponse`)
- `crates/paladin-web/tests/openapi_golden_v0_9.rs` - second sanctioned
  `ExecuteResponse` exception (`cost`/`CostDto`), module doc and test updates

## Decisions Made

- `crates/paladin-web/tests/openapi_golden_v0_9.rs` needed a second sanctioned
  `ExecuteResponse` exception (see key-decisions above) -- the plan named
  `crates/paladin-web/openapi.json` as a file to modify but not this golden-diff test
  file; it surfaced only by actually running `cargo test -p paladin-web` (the plan's
  own `<verify>` command for Task 2), which is exactly the scope boundary that command
  exists to enforce. Extending the exception, mirroring the Phase 31 precedent exactly,
  is the correct response to an intentional, planned additive field -- not a reason to
  weaken or skip the gate.
- `MockRepository::list_items`/`set_list_page` added to the `run_controller` test
  double so `list_runs_derives_costs_with_one_spend_call_per_page` could seed a real
  page; every other existing `run_controller` test still gets the pre-existing empty
  page by default (no seeded items), so no other test's behavior changed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] SHIP-02 golden v0.9 diff test failed after ExecuteResponse.cost was added**
- **Found during:** Task 2 (`cargo test -p paladin-web`, the task's own `<verify>` command)
- **Issue:** `ref_closure_schemas_match_the_frozen_baseline` (`openapi_golden_v0_9.rs`)
  failed: the new `cost`/`CostDto` schema entered the six-v0.9-path `$ref` closure that
  test freezes against `v0.9.0`, and the file had no exception for it.
- **Fix:** Extended `strip_known_v0_10_execute_response_divergence` to also strip
  `cost`/`CostDto`, mirroring the existing Phase 31 `usage`/`token_count`/
  `TokenUsageResponse` exception exactly; updated the module doc comment and
  `execute_response_exception_is_narrowly_scoped` to prove the new exception is
  equally narrow.
- **Files modified:** `crates/paladin-web/tests/openapi_golden_v0_9.rs`
- **Verification:** `cargo test -p paladin-web --test openapi_golden_v0_9` (7/7 pass);
  `cargo test -p paladin-web` (whole crate, 0 failed).
- **Committed in:** `2f701d2c` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 bug fix, Rule 1)
**Impact on plan:** Necessary to satisfy the plan's own `<verify>` command
(`cargo test -p paladin-web`) and must-have truth about the OpenAPI baseline. No scope
creep -- the fix is scoped to exactly the one new field this plan introduced.

## Issues Encountered

None beyond the deviation above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- D-10's HTTP half of LEDGR-04 is fully delivered: `GET /runs/{id}`, `GET /runs`, and
  every `ExecuteResponse`-shaped agent response now carry ledger-derived/priced cost,
  with the OpenAPI contract (both the committed baseline and the frozen v0.9.0 golden
  diff) current.
- `make api-surface`/CHANGELOG updates remain explicitly owned by plan 39-08 (D-00f),
  consistent with every prior plan in this phase -- not touched here. Every new public
  item is additive (`CostDto`, `RunApiState.treasury_ledger`/`with_treasury_ledger`,
  `RunResponse.cost`, `ExecuteResponse.cost`); no existing public signature changed
  (only the frozen-baseline test file's own internal exception list grew).
- No blockers for 39-07/39-08.

---
*Phase: 39-spend-ledger*
*Completed: 2026-09-28*

## Self-Check: PASSED

All four modified files verified present on disk with their new symbols
(`grep -c` for `CostDto`, `with_treasury_ledger`, `execute_response_carries_cost_when_priced`,
and the second `strip_known_v0_10_execute_response_divergence` exception all non-zero); commits
`8f9dcbfb` and `2f701d2c` verified present in git history (`git log --oneline --all | grep -E
"8f9dcbfb|2f701d2c"` both found).
