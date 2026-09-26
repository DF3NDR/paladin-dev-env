---
phase: 38-design-seams-pricing-cost-producer
plan: 04
subsystem: infra
tags: [rust, llm-port, pricing, semver, fallback, tdd, cost-arithmetic]

# Dependency graph
requires:
  - phase: 38-design-seams-pricing-cost-producer (plan 02)
    provides: "Cost, CurrencyCode, PriceRow, PriceTable, cost_of_call in paladin-core; PricingLlmAdapter/with_pricing decorator shape in paladin-llm (D-09)"
provides:
  - "LlmResponse.cost: Option<Cost> additive field (paladin-ports), #[serde(default, skip_serializing_if)] so legacy JSON is byte-identical and deserializes with cost == None (D-10, the served_by precedent)"
  - "Every in-tree LlmResponse struct literal migrated to cost: None in the same commit as the field (provider adapters, mock, conformance harness, services, tests, benches, examples, doctests -- ~37 files)"
  - "PricingLlmAdapter::generate prices response.model/response.usage -- the SERVED model, surviving a FallbackLlmAdapter hop to a differently-named model -- via a shared price_or_warn(table, model, usage) helper also used by the stream path (D-09)"
  - "MIGRATION.md §9.2 row + .cargo/semver-checks-allowlist.toml entry + constructible_struct_adds_field = \"allow\" in crates/paladin-ports/Cargo.toml registering the LlmResponse break (PRICE-03, D-00g)"
affects: [38-05, 38-06, 38-07, 38-08, 39-treasurer-ledger]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "price_or_warn(table, model, usage) -> Option<Cost>: one shared associated function called from both generate (non-streaming) and generate_stream's terminal-chunk mapper, so the pricing/warn-once rule (D-08) cannot diverge between the two run paths (D-09)"
    - "#[serde(default, skip_serializing_if = \"Option::is_none\")] additive-field precedent (D-25, the served_by/PaladinResult shape) extended to a fifth port-layer field, LlmResponse.cost"
    - "A test-only LlmPort stub (NamedModelPort) used where MockLlmAdapter's request-model echo cannot express the behavior under test -- here, a fallback hop served by a DIFFERENTLY-named model"

key-files:
  created: []
  modified:
    - crates/paladin-ports/src/output/llm_port.rs
    - crates/paladin-llm/src/pricing.rs
    - MIGRATION.md
    - .cargo/semver-checks-allowlist.toml
    - crates/paladin-ports/Cargo.toml
    - crates/paladin-llm/src/{anthropic,openai,deepseek,gemini,compat}/{adapter,vision,engine}.rs
    - crates/paladin-llm/src/{mock,conformance}.rs
    - crates/paladin-llm/benches/llm_serialization_benchmarks.rs
    - crates/paladin-battalion/src/grove_service.rs
    - crates/paladin-content/src/services/content_llm_analysis_service.rs
    - crates/paladin-eval/src/scripted_llm.rs
    - crates/paladin-memory/src/services/memory_extraction_service.rs
    - src/application/services/paladin/{paladin_builder,paladin_execution_service,planning_service,prompt_generation_service,temperature_service,middleware/chain}.rs
    - examples/herald_{custom_formatter,json_output,markdown_output}.rs
    - tests/functional/{content_llm_analysis_pipeline_test,paladin_tool_invocation_test}.rs
    - tests/helpers/mock_llm_adapter.rs
    - tests/integration/{arsenal_bridge_regression_test,autonomous_planning_test,battalion_herald_end_to_end_test,context_injection_test,herald_integration_test,paladin_garrison_integration_test,battalion/grove_integration_test}.rs
    - tests/unit/{paladin_builder_arsenal_test,paladin_builder_test,paladin_execution_service_test,arsenal/handoff_tool_test}.rs

key-decisions:
  - "Every in-tree LlmResponse literal was migrated to cost: None in the SAME commit as the field addition (Task 1) -- ~37 files, discovered via cargo check --workspace --all-targets --all-features's E0063 diagnostics run to a fixed point (9 rounds) rather than a single grep pass, because several sites (test aggregator binaries, benches, doc-gated examples) only surface once earlier crates in the same target compile cleanly."
  - "price_or_warn is a private associated fn taking (table: &PriceTable, model: &str, usage: &TokenUsage) rather than &self, so generate_stream's terminal-chunk .map() closure (which only clones an Arc<PriceTable> + String, never self) and generate (which has &self) can call the identical logic without restructuring the streaming closure's capture set."
  - "The fallback-hop test needed a hand-written NamedModelPort stub instead of MockLlmAdapter, because MockLlmAdapter::generate always echoes request.model.clone() into the response -- it cannot express a served model that differs from the requested one, which is exactly the behavior D-05/the fallback-hop acceptance criterion tests."
  - "A pre-existing rustdoc broken_intra_doc_links warning in cost.rs's module doc comment (introduced by plan 38-02, not touched by 38-04) was logged to this phase's new deferred-items.md rather than fixed, per the scope-boundary rule -- it is unrelated to LlmResponse.cost/PricingLlmAdapter::generate."

requirements-completed: [PRICE-03]

coverage:
  - id: D1
    description: "LlmResponse.cost: Option<Cost> is additive and byte-identical for legacy JSON: a document with no cost key deserializes to cost == None, and a None cost never appears as a JSON key"
    requirement: PRICE-03
    verification:
      - kind: unit
        ref: "crates/paladin-ports/src/output/llm_port.rs#tests::llm_response_without_cost_key_deserializes_to_none"
        status: pass
    human_judgment: false
  - id: D2
    description: "Every in-tree LlmResponse struct literal (provider adapters, mock, conformance harness, services, tests, benches, examples, doctests) compiles with cost: None in the same commit as the field"
    requirement: PRICE-03
    verification:
      - kind: other
        ref: "cargo check --workspace --all-targets --all-features"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ports --doc"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-llm --doc"
        status: pass
    human_judgment: false
  - id: D3
    description: "PricingLlmAdapter::generate prices a non-streaming response from its own served model and usage: gpt-4 at 2.50/10.00 per 1M with 1,000/2,000 tokens returns Some(22,500,000 nanos USD); an unpriced model returns None with the same once-per-model warn dedup rule the stream path uses"
    requirement: PRICE-03
    verification:
      - kind: unit
        ref: "crates/paladin-llm/src/pricing.rs#tests::generate_prices_the_response_model"
        status: pass
      - kind: unit
        ref: "crates/paladin-llm/src/pricing.rs#tests::generate_unpriced_model_reports_none"
        status: pass
    human_judgment: false
  - id: D4
    description: "Composed as Pricing(Fallback(primary, backup)), a call that hops from a failing primary to a backup serving a differently-named model is priced against the backup's SERVED model -- or None when that model has no row -- never against the originally requested model"
    requirement: PRICE-03
    verification:
      - kind: unit
        ref: "crates/paladin-llm/src/pricing.rs#tests::prices_the_served_model_after_fallback_hop"
        status: pass
    human_judgment: false
  - id: D5
    description: "PricingLlmAdapter's identity methods (get_provider_name, get_capabilities, validate_model, get_available_models) are unchanged pass-throughs to the inner port"
    requirement: PRICE-03
    verification:
      - kind: unit
        ref: "crates/paladin-llm/src/pricing.rs#tests::pricing_is_transparent"
        status: pass
    human_judgment: false
  - id: D6
    description: "The LlmResponse break is registered in the same commit as the field: MIGRATION.md §9.2 row (paladin-ports | LlmResponse, PRICE-03), a matching .cargo/semver-checks-allowlist.toml entry, and constructible_struct_adds_field = \"allow\" in crates/paladin-ports/Cargo.toml, kept set-equal by the offline gate mirror"
    requirement: PRICE-03
    verification:
      - kind: other
        ref: "./scripts/check-migration-allowlist.sh"
        status: pass
      - kind: other
        ref: "make check-gates"
        status: pass
    human_judgment: false
  - id: D7
    description: "The LlmResponse rustdoc 'Tracking Token Usage' example no longer teaches a bare-total floating-point cost estimate; it reads response.cost instead"
    requirement: PRICE-03
    verification:
      - kind: other
        ref: "grep -c 'total_tokens as f64' crates/paladin-ports/src/output/llm_port.rs"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ports --doc"
        status: pass
    human_judgment: false

duration: ~95min
completed: 2026-09-26
status: complete
---

# Phase 38 Plan 04: Design Seams — Cost on the Non-Streaming Response Summary

**`LlmResponse.cost: Option<Cost>` added and every in-tree literal migrated in one commit, with `PricingLlmAdapter::generate` pricing each response from its own served model — surviving a `FallbackLlmAdapter` hop to a differently-named model — via a `price_or_warn` helper shared with the stream path, and the public-API break registered across `MIGRATION.md`, the semver-checks allowlist and `Cargo.toml`.**

## Performance

- **Duration:** ~95 min
- **Started:** 2026-09-26T01:05Z (approx, per STATE.md's last recorded activity)
- **Completed:** 2026-09-26T02:41:28Z
- **Tasks:** 2 (Task 1: additive field + literal migration + break registration; Task 2 (TDD): generate() pricing across fallback hops)
- **Files modified:** 43 (41 source/doc files across Task 1's field-migration commit + the RED/GREEN pricing.rs commits, plus this SUMMARY and a new deferred-items.md)

## Accomplishments

- **Task 1 (`271c6a7b`):** Added `pub cost: Option<Cost>` to `LlmResponse` directly after `usage`, with `#[serde(default, skip_serializing_if = "Option::is_none")]` so a pre-existing serialized `LlmResponse` document deserializes with `cost == None` and an unpriced response is byte-identical to the pre-field JSON shape (the `served_by`/D-25 precedent). Replaced the "Tracking Token Usage" rustdoc example's floating-point `total_tokens as f64 * 0.03` estimate with one that reads `response.cost`. Added a new unit test, `llm_response_without_cost_key_deserializes_to_none`, proving both halves of the byte-identical claim (missing key deserializes to `None`; a `None` cost never serializes a `cost` key at all). Ran `cargo check --workspace --all-targets --all-features` to a fixed point across 9 rounds, migrating every reported `LlmResponse { .. }` literal (`cost: None,` immediately after `usage`) — 37 files spanning provider adapters (OpenAI, Anthropic, DeepSeek, Gemini, the OpenAI-compatible generic engine), the mock adapter (3 sites), the conformance harness, the eval scripted-LLM stub, `paladin-battalion`'s Grove mock LLMs (5 sites), a memory-extraction test double, five `PaladinExecutionService`-adjacent facade services, three Herald example programs, a serialization benchmark, and every integration/functional/unit test file that constructs a mock `LlmResponse`. Registered the break in the same commit: a new `MIGRATION.md` §9.2 row (`paladin-ports | LlmResponse`, `PRICE-03`), a matching `.cargo/semver-checks-allowlist.toml` entry under a new "Phase 38" header, and `constructible_struct_adds_field = "allow"` added to `crates/paladin-ports/Cargo.toml`'s existing lints table. `./scripts/check-migration-allowlist.sh` confirms set-equality.
- **Task 2, RED (`309bb2b0`):** Wrote 4 tests against the plan's `<behavior>` register — `generate_prices_the_response_model`, `generate_unpriced_model_reports_none`, `prices_the_served_model_after_fallback_hop` (via a new test-only `NamedModelPort` stub, since `MockLlmAdapter` always echoes `request.model` and cannot express a served model different from the requested one), and `pricing_is_transparent`. 2 of the 4 failed as expected against the unmodified `generate` (which still delegated unchanged).
- **Task 2, GREEN (`800e5ef0`):** Extracted `price_or_warn(table: &PriceTable, model: &str, usage: &TokenUsage) -> Option<Cost>`, called from both `generate` and `generate_stream`'s terminal-chunk mapper so the pricing/warn-once rule can never diverge between the two run paths. `generate` now stamps `response.cost = Self::price_or_warn(&self.table, &response.model, &response.usage)` after delegating — pricing the response's own served model, which survives composition as `Pricing(Fallback(primary, backup))` even when the backup serves a differently-named model. All 4 new tests pass (11/11 in the `pricing` module total).
- **Post-implementation doc fix (`d90976ac`):** `cargo doc --workspace --no-deps` flagged a new `rustdoc::private_intra_doc_links` warning (`[`Self::price_or_warn`]` linking to a private fn) introduced by the GREEN commit's doc comment — fixed by switching to a plain code span. The same `cargo doc` run surfaced a *pre-existing, unrelated* `broken_intra_doc_links` warning in `cost.rs`'s module doc comment (from plan 38-02, not touched by 38-04); logged to a new `.planning/phases/38-design-seams-pricing-cost-producer/deferred-items.md` per the scope-boundary rule rather than fixed here.

## Task Commits

1. **Task 1: LlmResponse.cost additive field, every literal migrated, break registered** - `271c6a7b` (feat)
2. **Task 2 (RED): failing tests for non-streaming pricing across fallback hops** - `309bb2b0` (test)
3. **Task 2 (GREEN): price non-streaming responses by their served model** - `800e5ef0` (feat)
4. **Deviation fix: drop private intra-doc link; log pre-existing cost.rs doc warning** - `d90976ac` (fix)

**Plan metadata:** (this commit)

_Note: Task 2 used the plan's `tdd="true"` RED/GREEN split (two commits, no REFACTOR needed — the implementation was already minimal); the fourth commit is a small deviation fix discovered while re-verifying the ADR-0033 rustdoc zero-warning bar against the two files this plan touched._

## Files Created/Modified

- `crates/paladin-ports/src/output/llm_port.rs` - `LlmResponse.cost: Option<Cost>` additive field, rewritten "Tracking Token Usage" doc example, new deserialization unit test
- `crates/paladin-llm/src/pricing.rs` - `price_or_warn` shared helper; `generate` now prices the served response; 4 new tests plus a `NamedModelPort` test stub
- `MIGRATION.md` - new §9.2 row, `paladin-ports | LlmResponse`
- `.cargo/semver-checks-allowlist.toml` - new entry under a "Phase 38" header
- `crates/paladin-ports/Cargo.toml` - `constructible_struct_adds_field = "allow"` added to the existing lints table
- ~35 further files (provider adapters, mocks, services, tests, benches, examples) - each gained `cost: None,` on its `LlmResponse { .. }` literal
- `.planning/phases/38-design-seams-pricing-cost-producer/deferred-items.md` (new) - records the pre-existing, out-of-scope `cost.rs` rustdoc warning found during verification

## Decisions Made

- Migrated every in-tree `LlmResponse` literal in the SAME commit as the field addition, found by iterating `cargo check --workspace --all-targets --all-features` to a fixed point (9 rounds) rather than trusting a single `grep` pass — later-compiling targets (test aggregator binaries, benches, feature-gated examples) only surface their own `E0063` errors once their dependency crates compile cleanly.
- `price_or_warn` takes `(table, model, usage)` rather than `&self`, so it can be called identically from `generate` (which has `&self`) and from `generate_stream`'s `.map()` closure (which only captures a cloned `Arc<PriceTable>` and `String`, never `self`).
- The fallback-hop test needed a hand-written `NamedModelPort` stub rather than `MockLlmAdapter`, because `MockLlmAdapter::generate` always echoes `request.model.clone()` — it cannot express a response served by a model different from the one requested, which is exactly what the fallback-hop acceptance criterion needs to prove.
- Logged (not fixed) a pre-existing, unrelated `cost.rs` rustdoc warning from plan 38-02 to a new phase-level `deferred-items.md`, per the scope-boundary rule.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Private intra-doc link in `generate_stream`'s rustdoc**
- **Found during:** Post-Task-2 verification (`cargo doc --workspace --no-deps`)
- **Issue:** The GREEN commit's updated `generate_stream` doc comment linked `` [`Self::price_or_warn`] ``, but `price_or_warn` is a private associated fn — `rustdoc::private_intra_doc_links` fires, which would break `make doc-check`'s ADR-0033 zero-warning bar.
- **Fix:** Replaced the intra-doc link with a plain code span (`` `price_or_warn` ``).
- **Files modified:** `crates/paladin-llm/src/pricing.rs`
- **Verification:** `cargo doc -p paladin-llm --no-deps` produces zero warnings; `cargo test -p paladin-llm --lib pricing` (11/11) and `cargo fmt --check` still pass.
- **Committed in:** `d90976ac`

---

**Total deviations:** 1 auto-fixed (1 documentation lint). **Impact on plan:** Purely a doc-comment wording fix; no behavior, API, or test changed as a result. One further out-of-scope discovery (a pre-existing `cost.rs` doc-link warning from plan 38-02) was logged to `deferred-items.md` rather than auto-fixed, per the scope-boundary rule — it is unrelated to this plan's `LlmResponse.cost`/`PricingLlmAdapter::generate` changes.

## Issues Encountered

- `cargo-deny` is not installed in this execution environment (consistent with plan 38-03's note); `Cargo.lock`/`Cargo.toml` are unchanged by this plan (verified via `git status --short`), so the dependency graph `cargo-deny`/`cargo-audit` would scan is unchanged from the last known-good state — no new advisory exposure is possible from this plan's changes.
- `cargo test --workspace --test '*' --all-features` reports 16 failures, all under `integration::redis_queue_integration_test::queue_integration_tests::*` — every one is a `testcontainers` Docker-container-spawn failure (`ContainerAsync::expect` panics), because this sandbox has no Docker daemon. This is a pre-existing environment limitation unrelated to this plan; every other test target in the workspace (817 further tests across all `tests/*` targets, including every file this plan modified) passed with 0 failures.
- The pre-existing `cost.rs` rustdoc `broken_intra_doc_links` warning noted above blocks `make doc-check`/`make clean-code`'s full ADR-0033 gate; this plan's own two touched files (`llm_port.rs`, `pricing.rs`) were independently verified warning-free via `cargo doc -p paladin-ports -p paladin-llm --no-deps`.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Non-streaming LLM calls on both run paths (the engine's `PricingLlmAdapter`-wrapped port and the agent loop's) now carry `LlmResponse.cost`, priced from the response's own served model — completing the pricing half CONTEXT.md's D-10 assigned to this plan (38-02 already primed `StreamingResponse.cost`/`ChunkMetadata.cost` for the streaming half).
- The fallback-hop pricing rule (price the SERVED model, never the requested one) is proven and available for 38-06's `TraceDispatcher::total_cost`/`RunFinished.cost` aggregation and 38-07's agent-loop run-level total to build on without further design work.
- `PaladinResult.cost` and `TraceEvent::{NodeFinished,RunFinished}.cost` remain out of this plan's scope, per its own stated boundary — 38-06/38-07 own wiring `LlmResponse.cost` through to those aggregation points.
- JSON/table herald cost rendering remains 38-05's scope (38-02 already updated the markdown herald).
- No blockers. One logged, out-of-scope, pre-existing `cost.rs` rustdoc defect from plan 38-02 is tracked in this phase's `deferred-items.md` for a future hygiene pass.

---
*Phase: 38-design-seams-pricing-cost-producer*
*Completed: 2026-09-26*

## Self-Check: PASSED

All created/modified files and commit hashes verified present on disk / in `git log --oneline --all`:
- `crates/paladin-ports/src/output/llm_port.rs` — FOUND (contains `pub cost: Option<Cost>`)
- `crates/paladin-llm/src/pricing.rs` — FOUND (contains `fn price_or_warn`)
- `.planning/phases/38-design-seams-pricing-cost-producer/deferred-items.md` — FOUND
- `271c6a7b` (Task 1) — FOUND
- `309bb2b0` (Task 2 RED) — FOUND
- `800e5ef0` (Task 2 GREEN) — FOUND
- `d90976ac` (deviation fix) — FOUND

Re-ran plan-level `<verification>` commands on the final tree: `cargo check --workspace --all-targets --all-features` (0 errors), `cargo test -p paladin-ports --doc` (151 passed), `cargo test -p paladin-llm --doc` (8 passed), `cargo test -p paladin-llm --lib pricing` (11 passed), `./scripts/check-migration-allowlist.sh` (set-equal), `make check-gates` (all gates pass). All task-level `<acceptance_criteria>` re-verified against the final source.
