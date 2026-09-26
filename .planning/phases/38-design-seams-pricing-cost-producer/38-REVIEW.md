---
phase: 38-design-seams-pricing-cost-producer
reviewed: 2026-09-26T19:39:30Z
depth: standard
files_reviewed: 56
files_reviewed_list:
  - .cargo/semver-checks-allowlist.toml
  - .project/current-exports.txt
  - crates/paladin-battalion/CHANGELOG.md
  - crates/paladin-battalion/src/engine/export/overlay.rs
  - crates/paladin-battalion/src/engine/hooks.rs
  - crates/paladin-battalion/src/engine/mod.rs
  - crates/paladin-battalion/src/engine/superstep.rs
  - crates/paladin-battalion/src/engine/test_support.rs
  - crates/paladin-battalion/src/grove_service.rs
  - crates/paladin-battalion/tests/export_golden.rs
  - crates/paladin-content/src/services/content_llm_analysis_service.rs
  - crates/paladin-core/CHANGELOG.md
  - crates/paladin-core/src/platform/container/cost.rs
  - crates/paladin-core/src/platform/container/execution_result.rs
  - crates/paladin-core/src/platform/container/herald.rs
  - crates/paladin-core/src/platform/container/mod.rs
  - crates/paladin-core/src/platform/container/trace.rs
  - crates/paladin-eval/src/assertion.rs
  - crates/paladin-eval/src/scripted_llm.rs
  - crates/paladin-eval/tests/assertion_snapshots.rs
  - crates/paladin-herald/CHANGELOG.md
  - crates/paladin-herald/src/json_herald.rs
  - crates/paladin-herald/src/markdown_herald.rs
  - crates/paladin-herald/src/table_herald.rs
  - crates/paladin-llm/CHANGELOG.md
  - crates/paladin-llm/benches/llm_serialization_benchmarks.rs
  - crates/paladin-llm/src/fallback.rs
  - crates/paladin-llm/src/lib.rs
  - crates/paladin-llm/src/pricing.rs
  - crates/paladin-memory/src/services/memory_extraction_service.rs
  - crates/paladin-ports/CHANGELOG.md
  - crates/paladin-ports/Cargo.toml
  - crates/paladin-ports/src/output/llm_port.rs
  - crates/paladin-ports/src/output/paladin_port.rs
  - crates/paladin-ports/src/output/trace_sink_port.rs
  - crates/paladin-web/src/agent_controller.rs
  - src/application/services/paladin/paladin_execution_service.rs
  - src/application/services/run/events.rs
  - src/application/services/run/inspector.rs
  - src/application/services/run/stream_tests.rs
  - src/application/services/run/worker.rs
  - src/bin/paladin-server.rs
  - src/config/mod.rs
  - src/config/settings.rs
  - src/config/treasurer.rs
  - src/config/user_config.rs
  - src/core/platform/mod.rs
  - src/infrastructure/telemetry/herald_sink.rs
  - src/infrastructure/telemetry/mod.rs
  - src/infrastructure/telemetry/otel_sink.rs
  - src/infrastructure/telemetry/persisting_sink.rs
  - src/infrastructure/web/agent_host.rs
  - src/infrastructure/web/facade_provisioner.rs
  - src/infrastructure/web/run_api_wiring.rs
  - tests/cli/eval_run_test.rs
  - tests/cli/run_export_test.rs
  - tests/helpers/mock_llm_adapter.rs
  - tests/integration/otel_transport_test.rs
  - tests/unit/handoff_service_test.rs
findings:
  critical: 1
  warning: 2
  info: 1
  total: 4
status: issues_found  # CR-01 resolved in-phase; WR-01/WR-02/IN-01 remain advisory
---

# Phase 38: Code Review Report

**Reviewed:** 2026-09-26T19:39:30Z
**Depth:** standard
**Files Reviewed:** 56
**Status:** issues_found

## Summary

Phase 38 adds the Treasurer pricing seam: a fixed-point `Cost` type in `paladin-core`
(`cost.rs`), a decimal-string `treasurer:` config section with exact integer parsing
(`treasurer.rs`), a `PricingLlmAdapter` `LlmPort` decorator composed *outside*
`FallbackLlmAdapter` (`pricing.rs`), additive `cost: Option<Cost>` fields threaded through
`LlmResponse`, `StreamingResponse`, `PaladinResult`, `TraceEvent::NodeFinished`/`RunFinished`,
and two `ExecutionMetadata` producers (the engine's `HeraldTraceSink`/`from_run_finished`, and
the agent loop's `stream_execution_metadata`).

The pure arithmetic in `cost.rs` (`cost_of_call`, `CostTally`, `Cost::checked_add`) is careful
and well-tested: i128 intermediate products, single half-up rounding pass, saturating i64
output, correct sub-count clamping, and a poison-on-any-`None` tally that never produces a
partial sum. `treasurer.rs`'s decimal parser is exact (no floats), rejects malformed/overflow/
too-fine input with named error paths, and is covered by targeted boundary tests. Both run
paths (the code-registered `Agent`-kind path via `agent_host.rs::build_agent` and the
`WarGraph`/engine path via `facade_provisioner.rs::paladin_port_from_settings`) correctly wrap
their resolved `LlmPort` with `with_pricing` before constructing the execution service, using
the *same* `TreasurerConfig::price_table()` validation. The engine's `TraceDispatcher` and the
SSE bridge (`events.rs::map_trace_event`) correctly implement the two most safety-critical
invariants for this phase: cost never appears on the Run API's SSE payloads (D-11, explicitly
tested by `sse_payloads_carry_no_spend_field`), and the OTel span exporter never surfaces cost
in span attributes.

One correctness bug was found in the agent-loop cost aggregation (`PaladinExecutionService::
execute_internal`): the early-return path taken when an `ExecutionMiddleware`'s `before_model`
hook finishes the run hard-codes `cost: None` instead of `cost_tally.total()`, unlike every
other return path in the same function. On any loop iteration after the first, this silently
discards the already-accumulated priced cost of prior loop iterations' model calls, reporting
a real, known cost as unpriced (`None`) rather than the CostTally-poison-only contract this
phase otherwise enforces everywhere else.

## Critical Issues

### CR-01: Agent-loop early return drops already-accumulated cost instead of `cost_tally.total()`

> **Resolved 2026-09-26 by the execute-phase orchestrator** — the `before_model` early return now
> reports `cost: cost_tally.total()` like the other three return paths (commit noted in the phase
> git log as `fix(38): ...`). Verified: `cargo test -p paladin-ai --lib paladin_execution_service`
> (65 passed), `cargo clippy -p paladin-ai --lib -- -D warnings` clean, `cargo fmt --check` clean.

**File:** `src/application/services/paladin/paladin_execution_service.rs:1513-1523`
**Issue:** Inside `execute_internal`'s reasoning loop, when `run_before(...)` returns
`BeforeOutcome::Finish` (a middleware short-circuits the run before this iteration's own model
call), the returned `PaladinResult` hard-codes `cost: None`:

```rust
return Ok(PaladinResult {
    output: accumulated_output,
    usage: usage.clone(),
    cost: None,
    execution_time_ms: start_time.elapsed().as_millis() as u64,
    loop_count: loop_num,
    stop_reason: effective_result.stop_reason,
    plan: task_plan,
    handoff_history,
    served_by,
});
```

Every other return path in this same function (`after_model` `Finish` at line 1586, `MaxLoops`
at line 1873, and the loop-exhausted fallback at line 1901) correctly uses
`cost: cost_tally.total()`. `cost_tally` (declared line 1301) accumulates
`response.cost.as_ref()` via `cost_tally.record_call` on every completed model call in the
loop (line 1553), *before* the `before_model` hook runs on the next iteration. If a middleware
finishes the run on loop 2+ (e.g. a `ModelCallLimit` or custom middleware that decides to stop
after observing the prior iteration's response), any priced cost recorded from loop 1 (or
later) is silently discarded and reported as `None` — the run is reported unpriced even though
every model call it made was actually priced. This directly contradicts the module's own D-00c
contract ("never a fabricated zero" / never silently drop a known cost) which every other exit
path in the file honors.

On loop 1 the bug is masked (nothing has been recorded into `cost_tally` yet, so `None` happens
to be correct), which is likely why it was not caught by the existing test suite — no test in
`agent_loop_cost_tests` drives a `before_model`-`Finish` middleware after a prior priced loop
iteration.

**Fix:**
```rust
return Ok(PaladinResult {
    output: accumulated_output,
    usage: usage.clone(),
    cost: cost_tally.total(),
    execution_time_ms: start_time.elapsed().as_millis() as u64,
    loop_count: loop_num,
    stop_reason: effective_result.stop_reason,
    plan: task_plan,
    handoff_history,
    served_by,
});
```
Add a regression test alongside `agent_loop_sums_cost_across_calls` that drives a two-iteration
run where a `before_model` middleware finishes the run on loop 2, asserting
`result.cost == Some(<loop 1's priced cost>)`.

## Warnings

### WR-01: `Cost::new` and the unconditional `Add`/`AddAssign` impls accept a negative amount silently

**File:** `crates/paladin-core/src/platform/container/cost.rs:145-149, 179-199`
**Issue:** `Cost::new(nanos: i64, currency: CurrencyCode)` performs no validation on `nanos` —
unlike `PriceRow`'s constructors, which reject a negative price at every call site. Nothing in
this module stops a caller from constructing (or `checked_add`ing into) a negative `Cost`, and
`impl Add for Cost`/`impl AddAssign for Cost` will happily combine two mismatched-sign amounts
without complaint (they only guard currency via `checked_add`/`Sum`, never sign). In practice
every producer in this codebase (`cost_of_call`, `PricingLlmAdapter`) only ever constructs
non-negative amounts, so this is not exploitable today, but it is a latent invariant gap: a
future producer (or a manually-constructed `Cost` in a test fixture or a deserialized value from
an untrusted/legacy JSON document) can silently introduce a negative running total that the
type system does nothing to prevent, and `CostTally`/`Cost::checked_add`'s "never a fabricated
zero" guarantees say nothing about sign.
**Fix:** Either document explicitly that `Cost` is a bare signed integer with no non-negativity
invariant (so callers know not to rely on `nanos() >= 0`), or add a `Cost::new_nonneg`/validated
constructor mirroring `PriceRow`'s pattern for any future producer that accepts external input
into a `Cost` directly (e.g. a future refund/credit feature) rather than only via
`cost_of_call`.

### WR-02: `UnpricedModelWarnings` capacity is a fixed, unconfigurable module constant shared process-wide

**File:** `crates/paladin-llm/src/pricing.rs:43-93`
**Issue:** `UNPRICED_MODEL_WARNING_CAPACITY` (256) is a hard-coded `const`, and
`UNPRICED_MODEL_WARNINGS` is a single process-wide `LazyLock` shared by every
`PricingLlmAdapter` instance in the process, per the module's own doc comment. This is
intentional design (documented as "D-08's dedup unit is per process, not per adapter
instance"), but it means: (1) an operator running many independently-configured
`PricingLlmAdapter`s over different price tables (e.g. multi-tenant, one table per tenant) will
have unrelated tenants' unpriced-model names compete for the same 256-slot budget, so tenant A
exhausting the cap silently suppresses tenant B's own first-time warning for an unrelated model
name; and (2) there is no way to reset or inspect the set at runtime (e.g. for a long-lived
server process that reloads its price table without restarting), so a model that was
transiently unpriced during a brief misconfiguration warns exactly once for the rest of the
process's life even after the price table is corrected.
**Fix:** Not necessarily a code change for this phase (the docs already flag this as
deliberate), but worth a follow-up decision record if multi-tenant pricing or hot config reload
is ever added — a per-`PriceTable`-instance (or per-adapter) warning set would avoid the
cross-tenant interference described above.

## Info

### IN-01: `PaladinResult.cost`/`TraceEvent::NodeFinished.cost` are not rendered by `JsonHerald::format_paladin_result` or `MarkdownHerald::format_paladin_result`

**File:** `crates/paladin-herald/src/json_herald.rs:119-135`, `crates/paladin-herald/src/markdown_herald.rs:316-343`
**Issue:** `PaladinResult` now carries a `cost: Option<Cost>` field (this phase), but
`JsonHerald::paladin_result_to_json` and `MarkdownHerald::format_paladin_result` only ever
render `output`/`usage`/`execution_time_ms`/`loop_count`/`stop_reason` — cost is never surfaced
by either herald's non-streaming result formatter, only through `finalize_stream`'s
`ExecutionMetadata` path. This is confirmed intentional by `paladin-herald/CHANGELOG.md` ("All
three heralds render a run's cost exclusively through `ExecutionMetadata::cost_display()`"), so
this is not a bug, but it is worth flagging for the next phase: a caller that formats a
completed, priced `PaladinResult` directly (rather than going through the streamed-completion
`ExecutionMetadata` producer) currently has no herald-rendered way to see that run's cost at
all, even though the value is sitting right there on the struct.
**Fix:** No action required for this phase; consider whether `format_paladin_result`/
`format_battalion_result` should also render `result.cost` in a follow-up phase, for parity with
the streaming path.

---

_Reviewed: 2026-09-26T19:39:30Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
