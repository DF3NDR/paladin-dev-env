---
phase: 38-design-seams-pricing-cost-producer
plan: 02
subsystem: infra
tags: [rust, cost-arithmetic, treasurer, fixed-point, tdd, pricing, herald, streaming]

# Dependency graph
requires:
  - phase: 38-design-seams-pricing-cost-producer (plan 01)
    provides: ADR-0052 (mid-run Treasurer enforcement attachment point), ADR-0053 (append-only ledger model)
provides:
  - "Cost, CurrencyCode, CostError, PriceRow, PriceTable, cost_of_call in paladin-core (i64 nano-unit fixed-point, i128 intermediates, one half-up round per call)"
  - "CostTally run-level accumulator (None-poisons-the-run rule, D-10) for TraceDispatcher::total_cost (38-06) and the agent loop (38-07) to share"
  - "PricingLlmAdapter decorator in paladin-llm (D-09) with warn-once unpriced-model logging (D-08)"
  - "StreamingResponse.cost / ChunkMetadata.cost+execution carriers on the two #[non_exhaustive] port types (D-10)"
  - "ExecutionMetadataBuilder::cost, cost_currency(), cost_display() on herald.rs; rustdoc now says produced by the Treasurer (D-12)"
  - "PaladinExecutionService::finalize_stream_output + stream_execution_metadata agent-loop producer"
  - "MarkdownHerald renders currency-aware cost text (0.0450 USD) instead of a hard-coded dollar sign (D-04)"
affects: [38-03, 38-04, 38-05, 38-06, 38-07, 38-08, 39-treasurer-ledger]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "i128 intermediate products, single half-up rounding per call, saturating i64 (D-02) — the CostTally/Cost accumulation pattern token_usage.rs already established for TokenUsage"
    - "Decorator-at-LlmPort-boundary pricing (D-09), sibling shape to FallbackLlmAdapter"
    - "Poisoned-accumulator pattern (Empty/Priced/Unknown three-state enum) so an unpriced call can never be silently dropped from a run total"

key-files:
  created:
    - crates/paladin-core/src/platform/container/cost.rs
    - crates/paladin-llm/src/pricing.rs
  modified:
    - crates/paladin-core/src/platform/container/mod.rs
    - crates/paladin-core/src/platform/container/herald.rs
    - crates/paladin-ports/src/output/llm_port.rs
    - crates/paladin-ports/src/output/paladin_port.rs
    - crates/paladin-llm/src/lib.rs
    - crates/paladin-herald/src/markdown_herald.rs
    - src/application/services/paladin/paladin_execution_service.rs
    - .project/current-exports.txt

key-decisions:
  - "Task 1's cost_of_call arithmetic needed zero correction — all 17 of Task 2's named tests (every axis, both fallback rules, half-up rounding boundaries, single-rounding-per-call, clamping, i64 saturation) passed on the first run."
  - "CostTally added as a private three-state (Empty/Priced/Unknown) accumulator: a None cost or a currency mismatch poisons the tally permanently, so a run total is never a silently-partial sum (D-10)."
  - "CostTally is not re-exported through the facade crate, so it produces no public API surface change — verified with PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 ./scripts/check-api-surface.sh (3960 items, unchanged); no .project/current-exports.txt regeneration or CHANGELOG entry required for this task."

requirements-completed: [PRICE-02, PRICE-03]

coverage:
  - id: D1
    description: "A priced streamed agent call reaches ExecutionMetadata.cost_estimate (0.0225) and the markdown herald renders '0.0225 USD'"
    requirement: PRICE-03
    verification:
      - kind: unit
        ref: "src/application/services/paladin/paladin_execution_service.rs#streamed_cost_tests::streamed_priced_call_produces_execution_metadata_cost"
        status: pass
  - id: D2
    description: "An unpriced model's streamed call yields cost None end-to-end (never a fabricated zero) with one warn-once log line"
    requirement: PRICE-03
    verification:
      - kind: unit
        ref: "src/application/services/paladin/paladin_execution_service.rs#streamed_cost_tests::streamed_unpriced_call_reports_no_cost"
        status: pass
      - kind: unit
        ref: "crates/paladin-llm/src/pricing.rs#tests::unpriced_model_warnings_warns_once_per_model_and_stops_at_capacity"
        status: pass
  - id: D3
    description: "cost_of_call implements the D-06 formula exactly across every axis, both fallback rules, and the None-sub-count rule"
    requirement: PRICE-02
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/cost.rs#tests (prompt_completion_only, cache_axes_subtract_from_base, full_cache_hit_differs_from_no_cache, omitted_cache_prices_bill_at_prompt_price, reasoning_axis_subtracts_from_base, none_sub_counts_are_zero)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Fixed-point precision: i128 intermediates, single half-up rounding per call (not per axis), sub-micro prices never round to zero"
    requirement: PRICE-02
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/cost.rs#tests (sub_micro_price_does_not_round_to_zero, half_up_rounding_boundaries, rounds_once_per_call_not_per_axis)"
        status: pass
    human_judgment: false
  - id: D5
    description: "Boundary safety: zero-token priced calls yield Some(0) not None; malformed containment (cache/reasoning sub-counts exceeding the parent) is clamped, never panics; u32::MAX tokens at i64::MAX prices saturate without overflow"
    requirement: PRICE-02
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/cost.rs#tests (zero_tokens_on_priced_row_is_some_zero, containment_violations_are_clamped, saturates_at_i64_max)"
        status: pass
    human_judgment: false
  - id: D6
    description: "CostTally: empty run has no cost; priced calls sum; any unpriced call poisons the run total permanently even if later calls are priced; a currency mismatch poisons; a neutral (default-usage, no-cost) node does not affect the tally; the sum saturates at i64::MAX"
    requirement: PRICE-03
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/cost.rs#tests::cost_tally_rules"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/cost.rs#tests::cost_checked_add_and_option_sum"
        status: pass
    human_judgment: false
  - id: D7
    description: "PriceRow rejects negative axis prices naming the axis; zero is a valid (free-tier) price; CurrencyCode validates ISO-4217-shaped codes at construction and at deserialization; PriceTable lookup is exact and case-sensitive"
    requirement: PRICE-01
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/cost.rs#tests (price_row_rejects_negative_axis, currency_code_validation, price_table_lookup_is_exact_and_case_sensitive)"
        status: pass
    human_judgment: false

duration: 57min
completed: 2026-09-25
status: complete
---

# Phase 38 Plan 02: Design Seams — Pricing & Cost Producer Summary

**A pure i64 nano-unit fixed-point cost engine (Cost/CurrencyCode/PriceRow/PriceTable/cost_of_call/CostTally) wired through a PricingLlmAdapter decorator to ExecutionMetadata and a currency-aware markdown herald, proven end-to-end on the streamed agent-loop path and pinned by 17 named arithmetic tests.**

## Performance

- **Duration:** 57 min (Task 1 tracer + tracer sign-off checkpoint + Task 2 continuation)
- **Started:** 2026-09-25T17:03:49Z
- **Completed:** 2026-09-25T18:00:45Z
- **Tasks:** 2 (Task 1: tracer, checkpoint; Task 2: arithmetic contract + CostTally)
- **Files modified:** 10 (9 from Task 1 + cost.rs test/CostTally additions in Task 2)

## Accomplishments

- **Task 1 (tracer, `c57d2127`):** Proved the phase's architecture end-to-end on its thinnest real path — an operator-style price table, wrapped by `PricingLlmAdapter` at the `LlmPort` boundary, prices a streamed `gpt-4` call (1,000 prompt / 2,000 completion tokens) to 22,500,000 nanos; `PaladinExecutionService`'s new streamed-completion producer (`stream_execution_metadata`) attaches that cost to `ExecutionMetadata`; `finalize_stream_output` hands it to `MarkdownHerald`, which renders `0.0225 USD`. An unpriced model's call yields `cost: None` end-to-end with exactly one warn-once log line. Both `StreamingResponse` and `ChunkMetadata` gained additive `#[non_exhaustive]`-safe `cost`/`execution` carriers. `herald.rs`'s five reservation-note doc sites now say the field is "produced by the Treasurer" instead of "no in-tree producer yet."
- **Tracer sign-off checkpoint:** Human reviewed and approved the tracer slice; the orchestrator independently reran the tracer's `<verify>` command (197 tests passed, 0 failed; clippy clean with `-D warnings`) before Task 2 began.
- **Task 2 (`6deeae08`):** Wrote the 17-test TDD contract from the plan's `<behavior>` register directly against Task 1's existing `cost_of_call` — every test passed on the first run, so no arithmetic correction was needed. Added `CostTally`, the run-level accumulator that both `TraceDispatcher::total_cost` (38-06) and the agent loop (38-07) will reuse: a private three-state (`Empty`/`Priced`/`Unknown`) enum where any unpriced call or currency mismatch poisons the running total permanently, so a run cost is never a silently-partial sum (D-10).

## Task Commits

1. **Task 1: Tracer — a priced streamed agent call reaches ExecutionMetadata and the markdown herald** - `c57d2127` (feat)
2. **Task 2: Cost arithmetic contract and the run-level CostTally** - `6deeae08` (test)

**Plan metadata:** (this commit)

## Files Created/Modified

- `crates/paladin-core/src/platform/container/cost.rs` - `Cost`, `CurrencyCode`, `CostError`, `PriceRow`, `PriceTable`, `cost_of_call`, `CostTally` — pure i64 nano-unit fixed-point cost arithmetic, no floating point
- `crates/paladin-core/src/platform/container/mod.rs` - `pub mod cost;`
- `crates/paladin-core/src/platform/container/herald.rs` - `COST_CURRENCY_METADATA_KEY`, `ExecutionMetadataBuilder::cost`, `ExecutionMetadata::cost_currency`/`cost_display`, rustdoc rewritten to "produced by the Treasurer"
- `crates/paladin-ports/src/output/llm_port.rs` - `StreamingResponse.cost: Option<Cost>`, `StreamingResponse::with_cost`
- `crates/paladin-ports/src/output/paladin_port.rs` - `ChunkMetadata.cost`/`.execution`, `with_cost`/`with_execution` builders
- `crates/paladin-llm/src/pricing.rs` - `PricingLlmAdapter` decorator, `with_pricing`, `PRICING_LOG_TARGET`, process-wide warn-once unpriced-model set
- `crates/paladin-llm/src/lib.rs` - `pub mod pricing;`
- `crates/paladin-herald/src/markdown_herald.rs` - `finalize_stream` renders `cost_display()` instead of a hard-coded `$`
- `src/application/services/paladin/paladin_execution_service.rs` - `finalize_stream_output`, `stream_execution_metadata`, `streamed_cost_tests` module
- `.project/current-exports.txt` - regenerated by Task 1 (new `finalize_stream_output` facade surface); unchanged by Task 2 (`CostTally` is not facade-exported)

## Decisions Made

- Task 1's arithmetic (clamping, rounding, fallback prices) required no correction under Task 2's exhaustive test contract — the tracer's careful reading of D-02/D-06 during Task 1 held up against 17 independently-derived test cases.
- `CostTally`'s poisoning semantics chosen so that `record_node` treats a `(TokenUsage::default(), None)` pair (non-Paladin node / cache hit / failed attempt) as neutral, but any other unpriced call poisons the run — this matches the plan's `must_haves` edge truth verbatim.
- Confirmed no public API surface change from Task 2 by rerunning `check-api-surface.sh` with the CI-pinned `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20` (the default unpinned `nightly` toolchain produces spurious `-> Self` vs fully-qualified-type diff noise unrelated to this phase — a known toolchain-drift artifact documented in `scripts/extract-public-api.sh`, not a real API change). No `.project/current-exports.txt` update or CHANGELOG entry needed for Task 2.

## Deviations from Plan

None - plan executed exactly as written. Task 1's tracer implementation and Task 2's test contract both matched the plan's `<action>` and `<behavior>` sections with no auto-fixes, architectural questions, or scope changes.

## Issues Encountered

None. The one operational note: running `make api-surface` without `PUBLIC_API_TOOLCHAIN` set produces false-positive drift (plain `nightly` toolchain renders derived-trait return types differently from the CI-pinned `nightly-2026-09-20`); this is a pre-documented toolchain-pin caveat, not a defect introduced by this plan, and resolved immediately by re-running with the pinned toolchain per `scripts/extract-public-api.sh`'s own comments.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The cost engine (`Cost`/`CurrencyCode`/`PriceRow`/`PriceTable`/`cost_of_call`/`CostTally`) is complete, tested, and available to every downstream 38-xx plan without further arithmetic work.
- `CostTally` is ready for `TraceDispatcher::total_cost` (38-06) and the agent-loop run-level total (38-07) to adopt directly — no further design needed there.
- Non-streaming `generate()` pricing (`LlmResponse.cost`) remains 38-04's scope, per this plan's stated expansion-owner boundary; config parsing and production wiring remain 38-03's scope; JSON/table heralds remain 38-05's scope.
- No blockers.

---
*Phase: 38-design-seams-pricing-cost-producer*
*Completed: 2026-09-25*
