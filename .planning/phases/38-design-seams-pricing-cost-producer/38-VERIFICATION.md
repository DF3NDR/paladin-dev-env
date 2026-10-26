---
phase: 38-design-seams-pricing-cost-producer
verified: 2026-09-26T19:46:02Z
status: passed
score: 4/4 must-haves verified (roadmap success criteria); 9/9 plans confirmed against codebase
behavior_unverified: 0
overrides_applied: 0
---

# Phase 38: Design Seams & Pricing/Cost Producer Verification Report

**Phase Goal:** An operator can price every LLM call in a configured currency and see an honest
per-run cost, with the two gating architecture decisions — the mid-run Treasurer enforcement
attachment point across `WarEngine` and `PaladinExecutionService`, and derive-on-read vs a
running-balance ledger model — recorded before any dependent phase begins.

**Verified:** 2026-09-26T19:46:02Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (ROADMAP Phase 38 Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Operator can configure a per-model price table (prompt, completion, cache-read, cache-write, reasoning) as decimal strings; default table empty; negative/malformed price rejected at config validation | ✓ VERIFIED | `src/config/treasurer.rs` — `TreasurerConfig`/`PriceRowConfig` (deny_unknown_fields), `parse_price_nanos_per_million` (exact integer parser, no float), `price_table()`/`validate()`. `TreasurerConfig::default()` is empty pricing + `"USD"`. `paladin-server.rs:79-84` calls `get_treasurer_config().validate()` before any provider is built, refusing boot with `"invalid treasurer configuration: {e}"`. 12/12 `config::treasurer` tests pass (ran live: `cargo test -p paladin-ai --lib config::treasurer::` → 12 passed). Negative prices rejected (leading `-` fails the ASCII-digit-only grammar), malformed/too-fine/overflowing prices rejected with path-precise errors. |
| 2 | For any `TokenUsage`, cost computed as exact fixed-point integer (i64 nano-units per D-02's amendment) with no `f64` accumulation, unit-tested per token type incl. cache/reasoning | ✓ VERIFIED | `crates/paladin-core/src/platform/container/cost.rs` — `cost_of_call` uses `i128` intermediate products, single half-up rounding over the summed axes, saturates to `i64`; no `f32`/`f64` in the module outside doc comments (grep-confirmed). 17/17 `cost::` unit tests pass (ran live), covering: prompt/completion-only, cache-read/write subtraction, 100% cache hit vs. no-cache, omitted-axis fallback to parent price, reasoning axis, None-sub-count treatment, sub-micro rounding (`0.15/1M` → 150 nanos, not 0), half-up boundaries (500_000/499_999/1_500_000), round-once-not-per-axis, zero-token Some(0), containment-violation clamping (no panic), i64::MAX saturation, negative-price rejection, currency-code validation. ROADMAP.md and REQUIREMENTS.md PRICE-02 both carry the required dated amend note reconciling "micro-units" wording to i64 nano-units (D-02, ADR-0053). |
| 3 | A completed run's `ExecutionMetadata.cost_estimate` carries currency cost end-to-end when priced, `None` (never `0`) when not; rustdoc note reads "produced by the Treasurer" | ✓ VERIFIED | `herald.rs` rustdoc at 5 sites (field doc, struct-level list, `total_cost()`, builder doc, example) reads "produced by the Treasurer" (grep-confirmed, no remaining "Reserved for the Treasurer … no in-tree producer yet" text). End-to-end producers exist on both paths: agent loop (`PaladinExecutionService::execute_stream` → `ExecutionMetadataBuilder::cost`) and engine path (`ExecutionMetadata::from_run_finished` in herald.rs + `HeraldTraceSink` in `src/infrastructure/telemetry/herald_sink.rs`). Both wired to `with_pricing`/`PricingLlmAdapter` at `agent_host.rs:181` and `facade_provisioner.rs:131`. `CostTally`'s "any unpriced call poisons the run total" rule (never a partial/zero sum) verified by unit test `cost_tally_rules`. Herald tests confirm `0.0225 USD` rendering and no Cost row (never `$0.00`/`0.0000` fabrication) when unpriced — `paladin-herald` 53/53 tests pass; `herald_sink.rs`'s 3 tests (incl. `priced_engine_run_reaches_the_herald`) pass. |
| 4 | Two ADRs on record — mid-run enforcement attachment point (ADR-0052) and ledger balance model (ADR-0053) — Phase 39/41/42 plans reference them rather than re-opening the question | ✓ VERIFIED (ADRs) / ⏳ backstop (downstream citation) | Both ADRs exist with all 7 PROMOTION.md headings in order (Status/Context/Decision/Considered Options/Code Locations/Code Conformance/Downstream Consumers), `Status: Accepted`, dated 2026-09-25. ADR-0052 records D-13 exactly: `PricingLlmAdapter` decorator at the `LlmPort` boundary on both paths (composed outside `FallbackLlmAdapter`), engine halt at the `WarEngine` superstep boundary (`WaypointStatus::Halted`), agent-loop halt reuses `TokenBudget`'s `after_model`/`StopReason::TokenBudget`, `AgentRuntimeConfig::build_chain` explicitly stays unwired for the engine path, and states the verified zero-production-callers fact plainly (never describes the cutoff as already live). Considered Options names both rejected alternatives with their concrete reasons. ADR-0053 records D-14/D-15: append-only, derive-on-read ledger; row kinds reserve/settle/release with signed contributions; i64 nano-unit amounts + ISO 4217 code; settlement idempotency key `(run_id, superstep, attempt)` with the superstep-aggregate resolution (operator-selected 2026-09-25) and the rejected per-node-extended-key alternative; per-backend SUM-then-reserve serialization (Postgres row/advisory lock, SQLite `BEGIN IMMEDIATE`, in-memory mutex); DDL explicitly left to Phase 39. `PROMOTION.md`'s index carries rows 0052/0053 with dated advance notes and "Next free ADR number: 0054"; no prior row renumbered. ADR-0050 gained the required dated note that Milestone 14 now builds code under the reserved name. Phase 39/41/42 plan directories do not yet exist (correctly out of scope — this is the plan's own declared `verification: backstop` item, confirmable only once those phases are planned). |

**Score:** 4/4 roadmap success criteria verified (criterion 4's downstream-citation half is an explicitly-declared backstop item, not yet checkable — Phase 39/41/42 do not exist yet).

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/paladin-core/src/platform/container/cost.rs` | `Cost`, `CurrencyCode`, `CostError`, `PriceRow`, `PriceTable`, `cost_of_call`, `CostTally` | ✓ VERIFIED | 906 lines, all types/functions present, 17 tests pass live |
| `crates/paladin-llm/src/pricing.rs` | `PricingLlmAdapter` decorator, `with_pricing`, warn-once set | ✓ VERIFIED | 544 lines, `PricingLlmAdapter`/`with_pricing` present, 11 tests pass live |
| `src/config/treasurer.rs` | `TreasurerConfig`, `PriceRowConfig`, exact decimal parser, `validate()`, `EnvOverridable` | ✓ VERIFIED | 509 lines, all present, 12 tests pass live |
| `crates/paladin-core/src/platform/container/herald.rs` | `ExecutionMetadataBuilder::cost`, `cost_currency`/`cost_display`, rustdoc "produced by the Treasurer" | ✓ VERIFIED | Confirmed via grep at 5 sites |
| `.planning/decisions/0052-mid-run-treasurer-enforcement.md` | ADR-0052 | ✓ VERIFIED | All 7 headings, Status Accepted, D-13 content confirmed |
| `.planning/decisions/0053-ledger-balance-model.md` | ADR-0053 | ✓ VERIFIED | All 7 headings, Status Accepted, D-14/D-15 content confirmed |
| `.planning/decisions/PROMOTION.md` | Index rows 0052/0053, next-free 0054 | ✓ VERIFIED | Confirmed |
| `crates/paladin-herald/src/json_herald.rs`, `table_herald.rs` | Currency field, real-metadata table rendering | ✓ VERIFIED | `cost_currency()`/`cost_display()` used; 53/53 herald tests pass |
| `crates/paladin-core/src/platform/container/trace.rs` | `NodeFinished.cost`/`RunFinished.cost` additive fields | ✓ VERIFIED | `#[serde(default, skip_serializing_if)]` confirmed |
| `crates/paladin-core/src/platform/container/execution_result.rs` | `PaladinResult.cost` additive field | ✓ VERIFIED | Confirmed via SUMMARY + MIGRATION.md §9.2 row; 623/623 paladin-ai-core tests pass |
| `src/infrastructure/telemetry/herald_sink.rs` | `HeraldTraceSink` engine-path producer | ✓ VERIFIED | 376 lines, `pub struct HeraldTraceSink`, 3/3 tests pass live |
| `src/application/services/run/worker.rs` | `RunWorkerPool::with_herald` | ✓ VERIFIED | 34/34 `run::worker` tests pass live |
| `.project/current-exports.txt` | Refreshed baseline incl. cost module | ⚠️ MINOR (documented) | Literal substring `cost::Cost` absent — but `pub use paladin::core::platform::container::cost` (module re-export) and `container::cost::PriceTable` (via `TreasurerConfig::price_table`) are both present at lines 5592/6997. SUMMARY 38-09 documents this precisely: `cargo-public-api` never enumerates a re-exported module's descendant items individually (same pre-existing pattern as `TokenUsage`/`garrison`), confirmed by checking that precedent. `TreasurerConfig` (62 hits) and `HeraldTraceSink` (28 hits) — the other two required substrings — both pass. Intent of the artifact ("refreshed baseline including the cost module") is met; the literal grep in the plan's `must_haves.artifacts[].contains` field does not match verbatim. Not a functional gap. |
| `CHANGELOG.md` | `[Unreleased]` Added/Changed entries | ✓ VERIFIED | Confirmed, detailed entries present for every plan |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `src/infrastructure/web/agent_host.rs` | `paladin_llm::pricing::with_pricing` | decorator wrap before any provider call | ✓ WIRED | `agent_host.rs:181` confirmed |
| `src/infrastructure/web/facade_provisioner.rs` | `paladin_llm::pricing::with_pricing` | engine's shared `PaladinPort` | ✓ WIRED | `facade_provisioner.rs:131` confirmed |
| `src/config/settings.rs` | `TreasurerConfig::validate()` | `Settings::validate()` calls `get_treasurer_config().validate()` | ✓ WIRED | `settings.rs:113` confirmed |
| `src/bin/paladin-server.rs` | `Settings::get_treasurer_config().validate()` | boot-time refusal before agent build | ✓ WIRED | `paladin-server.rs:79-84` confirmed |
| `src/infrastructure/telemetry/herald_sink.rs` | `ExecutionMetadata::from_run_finished` → `Herald::finalize_stream` | on `RunFinished` | ✓ WIRED | Confirmed by source read + passing test `priced_engine_run_reaches_the_herald` |
| `crates/paladin-herald/src/table_herald.rs`/`json_herald.rs` | `crates/paladin-core/.../herald.rs` | `cost_display()`/`cost_currency()` shared formatter | ✓ WIRED | Confirmed via grep + passing tests |

### Data-Flow Trace (Level 4)

Cost flows real, non-hardcoded values end to end: `LlmResponse.usage`/`model` (real provider or mock
response) → `PricingLlmAdapter` looks up the model in the operator's `PriceTable` → `Cost` rides on
`LlmResponse.cost` → `PaladinResult.cost` (agent loop, `CostTally` fold) / `NodeFinished.cost` (engine,
per-attempt bridge) → `RunFinished.cost` (`TraceDispatcher::total_cost`) → `ExecutionMetadata.cost_estimate`
(both paths' producers) → herald `cost_display()`. No static/hardcoded stand-in values found; an unpriced
model traces to `None` at every stage (never a fabricated `0`), confirmed by the `CostTally` poisoning
tests and the `priced_engine_run_reaches_the_herald` / unpriced-model sibling test.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `cost.rs` fixed-point arithmetic (rounding, saturation, containment clamp) | `cargo test -p paladin-ai-core --lib cost::` | 17 passed, 0 failed | ✓ PASS |
| `pricing.rs` decorator (streamed pricing, unpriced warn-once, fallback-hop) | `cargo test -p paladin-llm --lib pricing::` | 11 passed, 0 failed | ✓ PASS |
| `treasurer.rs` config validation (negative/malformed/currency rejection) | `cargo test -p paladin-ai --lib config::treasurer::` | 12 passed, 0 failed | ✓ PASS |
| `paladin-core` full crate (cost, trace, herald, execution_result) | `cargo test -p paladin-ai-core` | 623 lib + 103 doctests passed | ✓ PASS |
| `paladin-battalion` full lib (engine bridges, trace dispatcher) | `cargo test -p paladin-battalion --lib` | 806 passed, 0 failed | ✓ PASS |
| `paladin-herald` full lib (json/table/markdown cost rendering) | `cargo test -p paladin-herald` | 53 passed, 0 failed | ✓ PASS |
| `herald_sink`/`worker` (engine-path producer + `with_herald`) | `cargo test -p paladin-ai --lib infrastructure::telemetry::herald_sink` / `application::services::run::worker` | 3 + 34 passed, 0 failed | ✓ PASS |
| `cargo fmt --check` | `cargo fmt --check` | exit 0 | ✓ PASS |
| `cargo doc --no-deps` zero-warning bar (ADR-0033) | `cargo doc -p paladin-ai-core -p paladin-ai -p paladin-llm -p paladin-herald -p paladin-battalion -p paladin-ports --no-deps` | 0 warnings | ✓ PASS |
| MIGRATION.md §9.2 / allowlist set-equality | `./scripts/check-migration-allowlist.sh` | "Allowlist is set-equal to the MIGRATION.md §9.2 deliberate-breaking register" | ✓ PASS |
| No dollar-sign literal remains in herald source | `grep -rn '"\$' crates/paladin-herald/src/*.rs` (excluding tests) | no matches | ✓ PASS |
| No HTTP schema exposes cost | `grep '"cost"' crates/paladin-web/openapi.json` | no matches | ✓ PASS |
| Debt markers in phase-touched files | `grep -E "TBD\|FIXME\|XXX\|TODO\|HACK\|PLACEHOLDER"` across 18 phase files | only pre-existing, unrelated `CREDENTIAL_PLACEHOLDER` identifier references | ✓ PASS (no blocker) |

Full-workspace `cargo test --workspace` was not re-run in this verification pass (each targeted crate's
tests were run individually above, satisfying the "run the full suite at most once" guidance without
needing the Docker-dependent integration tests SUMMARY 38-04 already documented as a pre-existing,
unrelated sandbox limitation — no Docker daemon for `testcontainers`-based Redis/MinIO integration
tests). Every unit/doctest target actually touched by this phase passed with 0 failures.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| PRICE-01 | 38-03 | Operator per-model price table, decimal strings, empty default, negative/malformed rejected | ✓ SATISFIED | `src/config/treasurer.rs`, 12 passing tests, boot-time validation confirmed |
| PRICE-02 | 38-01 (amend), 38-02 | Pure fixed-point cost function, no f64 accumulation, per-token-type unit tests | ✓ SATISFIED | `cost.rs`, 17 passing tests; REQUIREMENTS.md/ROADMAP.md carry the required i64-nano-unit amend note |
| PRICE-03 | 38-02, 38-04..38-09 | `ExecutionMetadata.cost_estimate` end-to-end producer, `None` never `0`, rustdoc updated | ✓ SATISFIED | Both-path producers confirmed wired and tested; rustdoc confirmed changed at all 5 sites |

No orphaned requirements: REQUIREMENTS.md maps only PRICE-01/02/03 to Phase 38, and all three appear
in plan frontmatter `requirements:` fields across 38-01 through 38-09.

### Anti-Patterns Found

None blocking. One pre-existing rustdoc intra-doc-link warning set (introduced by 38-02, logged in
`deferred-items.md`) was fully resolved by 38-09's closeout commit — confirmed by a live zero-warning
`cargo doc` run above. No `TBD`/`FIXME`/`XXX` debt markers in any phase-touched file.

### Human Verification Required

None. All must-haves were verifiable programmatically via source inspection and live targeted test
runs; no visual, real-time, or external-service-dependent behavior is in this phase's scope (design
seams, config, and pure/deterministic cost arithmetic only).

### Gaps Summary

No gaps block phase-goal achievement. One minor, well-documented artifact-match discrepancy: the
38-09 plan's `must_haves.artifacts[].contains: "cost::Cost"` check against `.project/current-exports.txt`
does not match verbatim, because `cargo-public-api`'s extraction tool never lists a `pub use`-re-exported
module's descendant items individually (a pre-existing, systemic tool behavior, not specific to this
phase — the same is true of `TokenUsage`/`garrison`). The underlying artifact intent — a refreshed
public-API baseline that includes the cost module — is satisfied: the module re-export and `PriceTable`
(reached via `TreasurerConfig::price_table`) are both present in the baseline. This does not affect any
ROADMAP success criterion and is not treated as a gap.

The one item not fully closeable at this phase's verification time is the second half of success
criterion 4 (Phase 39/41/42 plans citing ADR-0052/0053) — this is explicitly a `verification: backstop`
item in the 38-01 plan's own frontmatter, since those phases have not been planned yet. Both ADRs
themselves are fully on record, Accepted, and written to be cited (each names its rejected alternatives
and the phases that must reference it).

---

*Verified: 2026-09-26T19:46:02Z*
*Verifier: Claude (gsd-verifier)*
