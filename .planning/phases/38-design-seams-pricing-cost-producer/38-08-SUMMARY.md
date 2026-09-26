---
phase: 38-design-seams-pricing-cost-producer
plan: 08
subsystem: infra
tags: [rust, cost-arithmetic, treasurer, herald, trace-sink, tdd, engine, observability]

# Dependency graph
requires:
  - phase: 38-design-seams-pricing-cost-producer (plan 02)
    provides: "ExecutionMetadataBuilder::cost/cost_currency/cost_display, the agent-loop ExecutionMetadata producer (stream_execution_metadata) and markdown herald cost line this plan's engine producer must match in shape"
  - phase: 38-design-seams-pricing-cost-producer (plan 06)
    provides: "TraceEvent::RunFinished.cost carrier and TraceDispatcher::total_cost() -- the event this plan's producer reads"
  - phase: 38-design-seams-pricing-cost-producer (plan 07)
    provides: "PaladinResult.cost and the engine bridge (NodeDispatchResult) that makes RunFinished.cost a real, non-placeholder figure"
provides:
  - "ExecutionMetadata::from_run_finished(record, model_used) -- the engine-path ExecutionMetadata producer (D-12): builds metadata from a TraceRecord carrying RunFinished (execution_id from the run's UUID, end_time = record.at, start_time = end minus duration, token_usage, cost through the builder's cost(&Cost) conversion when RunFinished.cost is Some, error_count 1 for Failed), returns None for any other event"
  - "HeraldTraceSink (src/infrastructure/telemetry/herald_sink.rs): a TraceSink that hands every RunFinished record to a Herald's finalize_stream, logging the rendered summary under the paladin::herald target at info; a herald error is diagnostics-only (TraceSinkError::Failed), never fails the run"
  - "RunWorkerPool::with_herald + private run_model_label(&WarGraph): run_once composes a HeraldTraceSink alongside build_run_sink's own output into a CompositeSink when both are present; a pool with no herald composes exactly as before"
  - "build_run_api attaches a herald only when settings.herald is Some, built through Settings::create_default_herald; an unknown formatter name fails startup naming the config"
affects: [39-treasurer-ledger]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "TraceSink as an ExecutionMetadata producer boundary: HeraldTraceSink watches the trace stream for one event kind (RunFinished) and is otherwise a no-op, mirroring LogTraceSink/PersistingTraceSink's own single-purpose shape in the same module"
    - "Optional-sink composition via CompositeSink: two independently-optional Arc<dyn TraceSink> values (build_run_sink's own output and an operator herald) combine to None/single/CompositeSink depending on which are present, never allocating a composite for zero or one sink"

key-files:
  created:
    - src/infrastructure/telemetry/herald_sink.rs
  modified:
    - crates/paladin-core/src/platform/container/herald.rs
    - src/infrastructure/telemetry/mod.rs
    - src/application/services/run/worker.rs
    - src/infrastructure/web/run_api_wiring.rs

key-decisions:
  - "The end-to-end engine tests (priced_engine_run_reaches_the_herald, unpriced_engine_run_reports_no_cost) attach HeraldTraceSink directly to a bare WarEngine via with_trace_sink, not through RunWorkerPool -- this is the plan's own designed test shape (mirrors tracer_e2e.rs's PaladinPortAdapter pattern) and proves the producer chain from price table to rendered herald text without needing the full worker/queue/repository harness."
  - "Test Paladin nodes in the engine e2e tests set max_loops: MaxLoops::Fixed(1) explicitly -- PaladinData::default()'s Fixed(3) calls the (mocked) LLM three times per node in this bare harness (no tool call or middleware ever requests an early Finish), which would triple the priced cost the tests assert. This is a test-authoring correction, not a deviation from the plan's design."

requirements-completed: [PRICE-03]

coverage:
  - id: D1
    description: "ExecutionMetadata::from_run_finished builds metadata from a RunFinished TraceRecord (execution_id, end/start time, duration, usage, cost via the builder's single display-edge conversion, error_count), and returns None for any other event"
    requirement: PRICE-03
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/herald.rs#tests::from_run_finished_tests::from_run_finished_builds_priced_metadata"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/herald.rs#tests::from_run_finished_tests::from_run_finished_unpriced_has_no_cost"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/herald.rs#tests::from_run_finished_tests::from_run_finished_ignores_other_events"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/herald.rs#tests::from_run_finished_tests::from_run_finished_counts_failure"
        status: pass
    human_judgment: false
  - id: D2
    description: "HeraldTraceSink hands a RunFinished record's produced ExecutionMetadata to a Herald's finalize_stream, capturing the correct priced cost and currency"
    requirement: PRICE-03
    verification:
      - kind: unit
        ref: "src/infrastructure/telemetry/herald_sink.rs#tests::herald_sink_hands_run_finished_to_the_herald"
        status: pass
    human_judgment: false
  - id: D3
    description: "End-to-end on the engine path: a WarEngine run of one Paladin node (gpt-4) whose LlmPort is priced via PricingLlmAdapter hands HeraldTraceSink an ExecutionMetadata whose cost_estimate is Some(0.0225)/USD and a real MarkdownHerald renders '0.0225 USD'; the same shape on an unpriced model yields cost_estimate None, never Some(0.0)"
    requirement: PRICE-03
    verification:
      - kind: integration
        ref: "src/infrastructure/telemetry/herald_sink.rs#tests::priced_engine_run_reaches_the_herald"
        status: pass
      - kind: integration
        ref: "src/infrastructure/telemetry/herald_sink.rs#tests::unpriced_engine_run_reports_no_cost"
        status: pass
    human_judgment: false
  - id: D4
    description: "run_model_label names an engine run's single declared Paladin model, 'mixed' for more than one distinct model, and 'none' for a graph with no Paladin node"
    requirement: PRICE-03
    verification:
      - kind: unit
        ref: "src/application/services/run/worker.rs#tests::run_model_label_names_single_mixed_or_none"
        status: pass
    human_judgment: false
  - id: D5
    description: "RunWorkerPool::with_herald composes a HeraldTraceSink alongside build_run_sink's own output into a CompositeSink when both are present (single sink when only one is); build_run_api attaches a herald only when settings.herald is Some, via create_default_herald, and fails startup naming the config on an unknown formatter"
    verification: []
    human_judgment: true
    rationale: "No dedicated RunWorkerPool-level integration test exercises the composed-CompositeSink path with a herald wired in -- the plan's own designed test list (herald_sink_hands_run_finished_to_the_herald, priced/unpriced_engine_run_*, run_model_label_names_single_mixed_or_none) proves the producer and its WarEngine wiring directly, not RunWorkerPool's own sink-combination logic. Verified by code review, successful compilation and cargo clippy -D warnings across the composition branch (both-None/one-Some/both-Some), and consistency with the pre-existing build_run_sink combination pattern it mirrors."

duration: ~26min
completed: 2026-09-26
status: complete
---

# Phase 38 Plan 08: Design Seams — Engine-Path Cost Producer (HeraldTraceSink) Summary

**`ExecutionMetadata::from_run_finished` (the engine's real `ExecutionMetadata` producer) wired through a new `HeraldTraceSink` into `RunWorkerPool`, so a completed engine run's priced cost reaches an operator-configured herald end-to-end — proven from a `gpt-4` price table through `PricingLlmAdapter` to a real `MarkdownHerald` rendering `0.0225 USD`.**

## Performance

- **Duration:** ~26 min
- **Started:** 2026-09-26T14:36:00Z (approx, per STATE.md's prior session timestamp)
- **Completed:** 2026-09-26T15:02:00Z
- **Tasks:** 2 (Task 1: TDD RED/GREEN; Task 2: auto)
- **Files modified:** 5 (1 created, 4 modified)

## Accomplishments

- **Task 1, RED (`b96040de`):** Wrote four failing behavior tests directly against `ExecutionMetadata::from_run_finished`, which did not exist yet (`E0599: no associated function`) — the expected RED signal.
- **Task 1, GREEN (`95826734`):** Implemented `ExecutionMetadata::from_run_finished(record, model_used)`: matches `TraceEvent::RunFinished` (returning `None` for any other event), parses `execution_id` from the run's UUID (falling back to a fresh `Uuid::new_v4()`), computes `start_time` from `record.at` minus a saturating `u64`→`i64` `duration_ms` conversion, carries `token_usage` and — through the builder's single display-edge `cost(&Cost)` conversion — the run's priced cost when `Some`, sets `error_count` to `1` for `RunFinishStatus::Failed`, and records `thread_id`/`run_status` as metadata entries. No `.unwrap()`/`.expect(` inside the function (verified by the acceptance grep). All four tests plus a compiling rustdoc example pass.
- **Task 2 (`269c0730`):** Created `HeraldTraceSink` (`src/infrastructure/telemetry/herald_sink.rs`): on a `RunFinished` record, builds `ExecutionMetadata` via `from_run_finished` and hands it to a `Herald`'s `finalize_stream`, logging the rendered text under the `paladin::herald` target at `info`; every other event and any herald failure (`TraceSinkError::Failed`) are diagnostics-only, per the `TraceSink` contract. Exported from `telemetry/mod.rs`. Added `RunWorkerPool::with_herald` and a private `run_model_label(&WarGraph)` (single model / `"mixed"` / `"none"`); `run_once`'s per-run composition combines a herald-labeled `HeraldTraceSink` with `build_run_sink`'s own output into a `CompositeSink` only when both are present. Wired `build_run_api` to attach a herald only when `settings.herald` is `Some`, via `Settings::create_default_herald`, failing startup with `"invalid herald configuration: …"` on an unknown formatter. Proved the whole chain end-to-end: a `WarEngine` run of one Paladin node (`gpt-4`) over a `PricingLlmAdapter`-wrapped `MockLlmAdapter` (a `2.50`/`10.00`-per-1M `gpt-4` price row, 1,000/2,000 mocked tokens) hands `HeraldTraceSink` an `ExecutionMetadata` whose `cost_estimate` is `Some(0.0225)`/`USD`, and a real `MarkdownHerald` (`include_colors: false`) renders `"0.0225 USD"`; the same shape on an unpriced model (`p38-engine-unpriced`) yields `cost_estimate: None`, never a fabricated zero.

## Task Commits

1. **Task 1 (RED): failing tests for ExecutionMetadata::from_run_finished** - `b96040de` (test)
2. **Task 1 (GREEN): implement from_run_finished** - `95826734` (feat)
3. **Task 2: HeraldTraceSink wired into the run worker** - `269c0730` (feat)

**Plan metadata:** (this commit)

_Note: Task 1 used the plan's `tdd="true"` RED/GREEN split (two commits, no REFACTOR needed — the implementation matched the plan's `<action>` on the first pass)._

## Files Created/Modified

- `crates/paladin-core/src/platform/container/herald.rs` - `ExecutionMetadata::from_run_finished`, four new behavior tests, rustdoc with a compiling example
- `src/infrastructure/telemetry/herald_sink.rs` - `HeraldTraceSink`, `HERALD_LOG_TARGET`, three new tests (one unit, two engine end-to-end)
- `src/infrastructure/telemetry/mod.rs` - `pub mod herald_sink;` / `pub use herald_sink::HeraldTraceSink;`, one sentence in the module docs naming it
- `src/application/services/run/worker.rs` - `herald: Option<Arc<dyn Herald>>` field, `RunWorkerPool::with_herald`, private `run_model_label(&WarGraph) -> String`, per-run sink composition combining `HeraldTraceSink` with `build_run_sink`'s output, one new test
- `src/infrastructure/web/run_api_wiring.rs` - herald construction from `settings.herald` via `create_default_herald`, `.with_herald(h)` applied to the pool builder chain when present

## Decisions Made

- The end-to-end engine tests attach `HeraldTraceSink` directly to a bare `WarEngine` via `with_trace_sink` (mirroring `tracer_e2e.rs`'s own local `PaladinPortAdapter` pattern), not through the full `RunWorkerPool`/queue/repository harness — this is the plan's own designed test shape and proves the producer chain from price table to rendered herald text without unrelated harness weight.
- Test Paladin nodes in the engine e2e tests set `max_loops: MaxLoops::Fixed(1)` explicitly: `PaladinData::default()`'s `Fixed(3)` calls the mocked LLM three times per node in this bare harness (no tool call or middleware ever requests an early `Finish`), which would triple the priced cost the tests assert (`0.0675` instead of `0.0225`) — a test-authoring correction, not a deviation from the plan's design.

## Deviations from Plan

None - plan executed exactly as written. Both tasks' `<action>`/`<behavior>` sections were implemented as specified; the `max_loops` test-authoring note above is a test-fixture correction discovered while writing the engine e2e test, not a change to any production code path, deviation rule, or architectural decision.

## Issues Encountered

None. The one thing caught during authoring: the first run of `priced_engine_run_reaches_the_herald` failed its cost assertion (`0.0675` vs the expected `0.0225`) because the default `PaladinData` calls the LLM three times per node (`max_loops: Fixed(3)`) with no early-finish condition in this bare test harness — resolved by fixing the test fixture's `max_loops` to `1`, not by changing any production code.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- ROADMAP Phase 38 success criterion 3 now holds on both run paths: a completed run's `ExecutionMetadata.cost_estimate` carries the currency cost end-to-end when its model is priced and is `None` — never `0` — when it is not (agent loop from 38-02, engine path from this plan).
- `HeraldTraceSink`/`RunWorkerPool::with_herald`/`build_run_api`'s wiring are complete, tested, and available for any later phase that needs to observe a run's rendered cost summary in the process log.
- Persisting cost on the `Run` row, exposing it on `GET /runs*` or the CLI remain Phase 39 LEDGR-04's scope, per this phase's own stated boundary.
- No blockers.

---
*Phase: 38-design-seams-pricing-cost-producer*
*Completed: 2026-09-26*

## Self-Check: PASSED

All created/modified files and commit hashes verified present on disk / in `git log --oneline --all`:
- `crates/paladin-core/src/platform/container/herald.rs` — FOUND (contains `pub fn from_run_finished`)
- `src/infrastructure/telemetry/herald_sink.rs` — FOUND (contains `pub struct HeraldTraceSink`, `impl TraceSink for HeraldTraceSink`, `"paladin::herald"`)
- `src/infrastructure/telemetry/mod.rs` — FOUND (contains `pub use herald_sink::HeraldTraceSink;`)
- `src/application/services/run/worker.rs` — FOUND (contains `pub fn with_herald`, `HeraldTraceSink::new`, `fn run_model_label`)
- `src/infrastructure/web/run_api_wiring.rs` — FOUND (contains `create_default_herald()`, `with_herald(`)
- `b96040de` (Task 1 RED) — FOUND
- `95826734` (Task 1 GREEN) — FOUND
- `269c0730` (Task 2) — FOUND

Re-ran the plan-level `<verification>` commands on the final tree: `cargo test -p paladin-ai-core --lib herald` (16 passed), `cargo test -p paladin-ai-core --doc herald` (10 passed, 3 ignored), `cargo test -p paladin-ai --lib telemetry::herald_sink` (3 passed), `cargo test -p paladin-ai --lib run::worker` (34 passed), `cargo build -p paladin-ai --features web-server` (clean), `cargo clippy -p paladin-ai --all-targets --features web-server -- -D warnings` (clean), `cargo fmt --check -p paladin-ai` and `-p paladin-ai-core` (clean). The `from_run_finished` unwrap/expect acceptance grep prints `0`. All task-level `<acceptance_criteria>` re-verified against the final source.
