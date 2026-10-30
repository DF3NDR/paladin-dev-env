---
phase: 38-design-seams-pricing-cost-producer
plan: 06
subsystem: infra
tags: [rust, trace-events, cost-arithmetic, treasurer, serde, tdd, engine, observability]

# Dependency graph
requires:
  - phase: 38-design-seams-pricing-cost-producer (plan 02)
    provides: "Cost, CurrencyCode, CostTally in paladin-core (i64 nano-unit fixed-point, None-poisons-the-run accumulation rule) reused directly by this plan's TraceDispatcher::total_cost"
provides:
  - "TraceEvent::NodeFinished.cost / TraceEvent::RunFinished.cost: Option<Cost> additive fields, #[serde(default, skip_serializing_if)] so pre-phase trace JSON, export goldens and eval snapshots stay byte-identical (D-10)"
  - "TraceDispatcher::total_cost() -- the synchronous twin of total_usage(), folding every NodeFinished through CostTally::record_node inside emit() itself, never the async consumer"
  - "All five WarEngine RunFinished emission sites (start, resume_with_options early-complete + main paths, resume_with, fork) populated with cost: trace.total_cost() beside usage: trace.total_usage() (D-11a)"
  - "sse_payloads_carry_no_spend_field: proof that map_trace_event's existing field-selecting `..` patterns already exclude cost from the Run API's SSE stream (T-38-19, D-11)"
affects: [38-07, 38-08, 39-treasurer-ledger]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Fixed-point tally folded synchronously inside TraceDispatcher::emit (never the async consumer task) -- the same guarantee usage/superstep_count already carry, now shared by cost via CostTally::record_node"
    - "cargo check --workspace --all-targets --all-features run to a fixed point (7 rounds) to migrate every NodeFinished/RunFinished construction and exhaustive pattern across the workspace, rather than a single grep pass"

key-files:
  created: []
  modified:
    - crates/paladin-core/src/platform/container/trace.rs
    - crates/paladin-ports/src/output/trace_sink_port.rs
    - crates/paladin-battalion/src/engine/hooks.rs
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/src/engine/superstep.rs
    - crates/paladin-battalion/src/engine/export/overlay.rs
    - crates/paladin-battalion/tests/export_golden.rs
    - crates/paladin-eval/src/assertion.rs
    - crates/paladin-eval/tests/assertion_snapshots.rs
    - crates/paladin-llm/src/fallback.rs
    - src/application/services/run/events.rs
    - src/application/services/run/inspector.rs
    - src/application/services/run/stream_tests.rs
    - src/infrastructure/telemetry/otel_sink.rs
    - src/infrastructure/telemetry/persisting_sink.rs
    - tests/cli/run_export_test.rs
    - tests/cli/eval_run_test.rs
    - tests/integration/otel_transport_test.rs

key-decisions:
  - "Task 1's literal migration used cargo check --workspace --all-targets --all-features iterated to a fixed point (7 rounds) across 18 files rather than a single grep pass -- later-compiling targets (eval snapshot tests, otel/persisting-sink test modules, tests/cli, tests/integration) only surface their own E0063/E0027 diagnostics once earlier crates in the same target compile cleanly, exactly the discovery pattern 38-04's SUMMARY documented for LlmResponse."
  - "The engine's one real Paladin-attempt NodeFinished site (superstep.rs) and all five RunFinished emission sites in engine/mod.rs got cost: None as Task 1's compile-fix placeholder; Task 2 then replaced only the five RunFinished sites with trace.total_cost() -- the Paladin-attempt site stays None until 38-07 supplies the real per-attempt value, matching the plan's explicit expansion-owner boundary."
  - "make api-surface (PUBLIC_API_TOOLCHAIN=nightly-2026-09-20) reports the baseline unchanged (4013 items) after both tasks -- TraceEvent is #[non_exhaustive] (no external construction or exhaustive matching to break) and TraceDispatcher::total_cost is not re-exported through the facade crate, confirming this plan's own stated 'additive, no MIGRATION row expected' framing (D-00g) without needing to touch .project/current-exports.txt."

requirements-completed: [PRICE-03]

coverage:
  - id: D1
    description: "NodeFinished.cost and RunFinished.cost are additive Option<Cost> fields with legacy-compatible serde: a pre-phase trace record with no cost key deserializes to cost == None, and a priced RunFinished serializes one flat cost object with nanos/currency"
    requirement: PRICE-03
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/trace.rs#tests::node_and_run_finished_without_cost_key_deserialize_to_none"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/trace.rs#tests::priced_run_finished_serializes_cost_object"
        status: pass
    human_judgment: false
  - id: D2
    description: "Every in-tree NodeFinished/RunFinished construction and exhaustive pattern across 18 files compiles with the new field, and export goldens and eval snapshots stay byte-identical (no golden/snapshot file touched)"
    requirement: PRICE-03
    verification:
      - kind: other
        ref: "cargo check --workspace --all-targets --all-features"
        status: pass
      - kind: integration
        ref: "cargo test -p paladin-battalion --test export_golden (4/4, git status on tests/golden empty)"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-eval (56+12+1 passed, git status on tests/snapshots empty)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The SSE bridge (map_trace_event) exposes no cost on the Run API's stream in this phase: a priced NodeFinished/RunFinished's node_finished/done payload carries no cost key"
    requirement: PRICE-03
    verification:
      - kind: unit
        ref: "src/application/services/run/events.rs#tests::sse_payloads_carry_no_spend_field"
        status: pass
    human_judgment: false
  - id: D4
    description: "TraceDispatcher::total_cost is the synchronous twin of total_usage: sums priced NodeFinished events and ignores neutral (non-Paladin) ones, is None once any priced call is unpriced (never a partial sum), and is None with no sink configured"
    requirement: PRICE-03
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/hooks.rs#tests::total_cost_sums_priced_nodes_and_ignores_neutral_ones"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/hooks.rs#tests::total_cost_is_none_once_any_call_is_unpriced"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/hooks.rs#tests::total_cost_is_none_without_a_sink"
        status: pass
    human_judgment: false
  - id: D5
    description: "All five WarEngine RunFinished emission sites set cost: trace.total_cost() beside usage: trace.total_usage(), so the run total reaches every trace consumer, with no regression across the whole engine test suite"
    requirement: PRICE-03
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/hooks.rs#tests::run_finished_carries_total_cost"
        status: pass
      - kind: other
        ref: "grep -c 'cost: trace.total_cost()' crates/paladin-battalion/src/engine/mod.rs == grep -c 'usage: trace.total_usage()' (5 == 5)"
        status: pass
      - kind: integration
        ref: "cargo test -p paladin-battalion --lib engine (549/549 passed)"
        status: pass
    human_judgment: false

duration: ~28min
completed: 2026-09-26
status: complete
---

# Phase 38 Plan 06: Design Seams — Cost Carriers on the Trace Stream Summary

**`TraceEvent::NodeFinished.cost` / `RunFinished.cost: Option<Cost>` added with legacy-compatible serde, `TraceDispatcher::total_cost()` folding a run's spend synchronously inside `emit()`, and all five `WarEngine` `RunFinished` sites wired to it — while every pre-phase trace JSON byte, export golden and eval snapshot stays unchanged.**

## Performance

- **Duration:** ~28 min
- **Started:** 2026-09-26T13:07:00Z (approx, per STATE.md's prior session timestamp)
- **Completed:** 2026-09-26T13:35:26Z
- **Tasks:** 2 (Task 1: carriers + literal migration; Task 2 (TDD): `total_cost()` + the five `RunFinished` sites)
- **Files modified:** 18 (Task 1) + 2 (Task 2, hooks.rs + mod.rs, already counted in the 18)

## Accomplishments

- **Task 1 (`10d81c83`):** Added `cost: Option<Cost>` directly after `usage` on both `TraceEvent::NodeFinished` and `TraceEvent::RunFinished`, each with `#[serde(default, skip_serializing_if = "Option::is_none")]` — the exact `served_by`/D-25 precedent. Iterated `cargo check --workspace --all-targets --all-features` to a fixed point across 7 rounds to migrate every construction literal (`cost: None,`) and exhaustive pattern (`..` or `cost: _`) across 18 files: `paladin-ports`' re-export test, `paladin-battalion`'s `superstep.rs` (both `NodeFinished` sites — the cache-hit path and the real per-attempt path, which stays `None` until 38-07), `export/overlay.rs`, `export_golden.rs`, `hooks.rs`'s own test helpers, all five `RunFinished` sites in `engine/mod.rs` (`None` placeholder, replaced in Task 2), `paladin-eval`'s `assertion.rs`/`assertion_snapshots.rs`, `paladin-llm`'s `fallback.rs`, the facade's `events.rs`/`inspector.rs`/`stream_tests.rs`, the telemetry `otel_sink.rs`/`persisting_sink.rs`, and three `tests/` integration/CLI files. Added `node_and_run_finished_without_cost_key_deserialize_to_none` (legacy JSON with no `cost` key round-trips to `None` on both variants) and `priced_run_finished_serializes_cost_object` (a `Some` cost serializes one flat `{"nanos":...,"currency":...}` object; `None` emits no `cost` key). Added `sse_payloads_carry_no_spend_field`, proving `map_trace_event`'s pre-existing explicit-field-selection `..` patterns already exclude `cost` from the Run API's SSE stream with no code change needed there (D-11).
- **Task 2, RED (`863a3edf`):** Wrote the four behavior tests against `TraceDispatcher::total_cost()`, which does not exist yet — fails to compile (`E0599: no method named total_cost`), the expected RED signal for adding new API surface under TDD.
- **Task 2, GREEN (`dabc2b8e`):** Added `cost: Mutex<CostTally>` to `TraceQueue` (mirroring `usage`'s own doc comment and poisoning-recovery pattern), extended `emit()`'s `NodeFinished` arm to fold `(usage, cost)` through `CostTally::record_node` synchronously — never in the async consumer, matching engine/mod.rs's D-02/D-04/D-11 rule — and added `pub fn total_cost(&self) -> Option<Cost>` beside `total_usage()`. Replaced the `cost: None` placeholder on all five `RunFinished` emission sites in `engine/mod.rs` (`start`, `resume_with_options`'s early-complete and main paths, `resume_with`, `fork`) with `cost: trace.total_cost()`. All four new tests plus the whole `engine::hooks` (19/19) and `engine` (549/549) suites pass with zero regressions.

## Task Commits

1. **Task 1: NodeFinished.cost and RunFinished.cost — carriers, migration, legacy-compatible serde** - `10d81c83` (feat)
2. **Task 2 (RED): failing tests for TraceDispatcher::total_cost** - `863a3edf` (test)
3. **Task 2 (GREEN): implement total_cost and wire the five RunFinished sites** - `dabc2b8e` (feat)

**Plan metadata:** (this commit)

_Note: Task 2 used the plan's `tdd="true"` RED/GREEN split (two commits, no REFACTOR needed — the implementation matched the plan's `<action>` on the first pass)._

## Files Created/Modified

- `crates/paladin-core/src/platform/container/trace.rs` - `NodeFinished.cost`/`RunFinished.cost: Option<Cost>`, two new legacy-compatibility/serialization tests
- `crates/paladin-ports/src/output/trace_sink_port.rs` - re-export test literal migrated
- `crates/paladin-battalion/src/engine/hooks.rs` - `TraceQueue.cost: Mutex<CostTally>`, `emit()`'s `record_node` fold, `total_cost()`, four new TDD tests
- `crates/paladin-battalion/src/engine/mod.rs` - all five `RunFinished` sites now set `cost: trace.total_cost()`
- `crates/paladin-battalion/src/engine/superstep.rs` - both `NodeFinished` construction sites carry `cost` (cache-hit `None`; the real attempt site `None` pending 38-07)
- `crates/paladin-battalion/src/engine/export/overlay.rs` - exhaustive pattern gets `..`; four test literals migrated
- `crates/paladin-battalion/tests/export_golden.rs` - three test literals migrated; goldens unchanged
- `crates/paladin-eval/src/assertion.rs`, `tests/assertion_snapshots.rs` - test fixtures migrated; `.snap` artifacts unchanged
- `crates/paladin-llm/src/fallback.rs` - test literal migrated
- `src/application/services/run/events.rs` - test literals migrated; new `sse_payloads_carry_no_spend_field` test
- `src/application/services/run/inspector.rs`, `stream_tests.rs` - test literals migrated
- `src/infrastructure/telemetry/otel_sink.rs`, `persisting_sink.rs` - match arms gain `..`; test literals migrated
- `tests/cli/run_export_test.rs`, `tests/cli/eval_run_test.rs`, `tests/integration/otel_transport_test.rs` - test literals migrated

## Decisions Made

- Iterated `cargo check --workspace --all-targets --all-features` to a fixed point (7 rounds) rather than a single grep pass, since later-compiling targets (eval snapshot tests, telemetry sink test modules, `tests/cli`, `tests/integration`) only surface their own diagnostics once earlier crates in the same target compile cleanly — the same discovery pattern 38-04's SUMMARY documented.
- The engine's one real Paladin-attempt `NodeFinished` site kept `cost: None` (38-07's scope, per the plan's explicit expansion-owner boundary) while all five `RunFinished` emission sites were populated with `trace.total_cost()` in this plan's Task 2.
- `make api-surface` (pinned `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20`) reports the baseline unchanged (4013 items): `TraceEvent` is `#[non_exhaustive]` (no external construction or exhaustive match to break) and `TraceDispatcher::total_cost` is not re-exported through the facade crate — confirming this plan's own "additive, no MIGRATION row expected" framing (D-00g) without touching `.project/current-exports.txt`.

## Deviations from Plan

None - plan executed exactly as written. Every construction/pattern site the plan anticipated cargo check would surface was fixed the same way the plan's `<action>` prescribed (`cost: None,` on literals, `..`/`cost: _` on exhaustive patterns); no bugs, missing functionality, blockers, or architectural questions arose.

## Issues Encountered

- An early regex-based batch edit on `engine/mod.rs` (inserting `cost: trace.total_cost()` on all five `RunFinished` sites) doubled the leading indentation on each inserted line because the capture group used to locate the insertion point was nested inside the replacement string. Caught immediately by `cargo fmt --check`; fixed with a targeted script that copies the sibling `usage:` line's own indentation, then verified clean with a second `cargo fmt --check` pass. No functional impact — caught before any test run or commit.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `TraceEvent::NodeFinished.cost`, `TraceEvent::RunFinished.cost`, and `TraceDispatcher::total_cost()` are complete, tested, and wired through every `WarEngine` `RunFinished` emission site — the run-level audit record 38-08's engine-side `ExecutionMetadata` producer reads from.
- 38-07's remaining scope is now a one-site bridge, exactly as this plan's objective stated: replace the single `cost: None` placeholder on `superstep.rs`'s real Paladin-attempt `NodeFinished` site with the per-attempt `PaladinResult.cost` value.
- The SSE bridge (`map_trace_event`) is proven, by test, to expose no spend on the Run API's stream in this phase (D-11: that surface is Phase 39 LEDGR-04).
- No blockers.

---
*Phase: 38-design-seams-pricing-cost-producer*
*Completed: 2026-09-26*

## Self-Check: PASSED

All modified files and commit hashes verified present on disk / in `git log --oneline --all`:
- `crates/paladin-core/src/platform/container/trace.rs` — FOUND (contains `cost: Option<Cost>` twice)
- `crates/paladin-battalion/src/engine/hooks.rs` — FOUND (contains `pub fn total_cost`, `Mutex<CostTally>`, `record_node(`)
- `crates/paladin-battalion/src/engine/mod.rs` — FOUND (`cost: trace.total_cost()` appears 5 times, matching `usage: trace.total_usage()`'s 5 occurrences)
- `10d81c83` (Task 1) — FOUND
- `863a3edf` (Task 2 RED) — FOUND
- `dabc2b8e` (Task 2 GREEN) — FOUND

Re-ran the plan-level `<verification>` commands on the final tree: `cargo check --workspace --all-targets --all-features` (0 errors), `cargo test -p paladin-battalion --lib engine` (549 passed), `cargo test -p paladin-battalion --test export_golden` (4 passed, goldens unchanged), `cargo test -p paladin-eval` (56+12+1 passed, snapshots unchanged), `cargo test -p paladin-ai --lib run::events` (13 passed, including `sse_payloads_carry_no_spend_field`), `cargo fmt --check` (clean), `cargo clippy --all-targets --all-features -- -D warnings` (clean). All task-level `<acceptance_criteria>` re-verified against the final source.
