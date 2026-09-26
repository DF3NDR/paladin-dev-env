---
phase: 38
slug: design-seams-pricing-cost-producer
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
block_on: high
register_authored_at_plan_time: true
created: 2026-09-26
---

# Phase 38 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Register source: the `<threat_model>` blocks of plans 38-01 through 38-09 (35 threats, all
authored at plan time). No plan SUMMARY carries a `## Threat Flags` section. Verification depth:
ASVS L1 — grep-level evidence that each planned mitigation exists in the implementation, plus the
in-phase code review (`38-REVIEW.md`) and the gate records in `38-09-SUMMARY.md`.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| planning record → later phases | Phase 39/41/42 read ADR-0052/0053 as binding decisions | architecture text only, no secrets |
| provider response → pricing decorator | `usage` counts and the echoed `model` string are provider-controlled input to the arithmetic, the price lookup and the warn line | untrusted token counts, model name |
| operator config / environment → `Settings` | `treasurer.*` strings and `APP_TREASURER_CURRENCY` are parsed at boot | operator-supplied prices and currency code |
| HTTP `POST /agents` → `FacadeProvisioner` | an authenticated caller's model string becomes a price-table lookup key | caller-controlled model name |
| engine trace stream → sinks (log, `run_traces`, OTel, SSE bridge) | `RunFinished`/`NodeFinished` now carry `cost` and leave the process through several consumers | spend figures (must not reach the unscoped Run API before Phase 39/40) |
| `PaladinResult` → HTTP agent route (`ExecuteResponse`) | unscoped, authenticated HTTP surface | spend figures (must not leak) |
| `ExecutionMetadata` → herald text / process log | operator-visible output derived from run data | model name, token counts, cost |
| this repo → downstream crate consumers | published API changes across the workspace crates | public API shape |
| dependency graph → build | `make security` guards advisories and licences | third-party code |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-38-01 | Tampering | ADR numbering in `.planning/decisions/PROMOTION.md` | low | mitigate | `0052-mid-run-treasurer-enforcement.md` and `0053-ledger-balance-model.md` are the only 005{2,3} files; PROMOTION.md rows 0052/0053 plus dated next-free notes present | closed |
| T-38-02 | Repudiation | ADR-0053 idempotency key | medium | mitigate | ADR-0053 defines the `(run_id, superstep, attempt)` settlement idempotency key and the attempt-counter open item | closed |
| T-38-03 | Tampering | ADR-0053 serialization guidance | medium | mitigate | ADR-0053 names PostgreSQL `FOR UPDATE`-on-aggregate rejection and SQLite `BEGIN IMMEDIATE` | closed |
| T-38-04 | Information Disclosure | ADR text | low | accept | ADRs describe architecture only — see AR-38-01 | closed |
| T-38-05 | Denial of Service | `cost_of_call` arithmetic (`paladin-core` `cost.rs`) | medium | mitigate | `i128` intermediates, saturating `i64` output; `saturates_at_i64_max` and `containment_violations_are_clamped` tests; zero `unwrap`/`expect`/`panic!` in non-test code | closed |
| T-38-06 | Tampering (spend-governance bypass) | cost propagation (`CostTally`, decorator, `ExecutionMetadata`) | high | mitigate | `Cost` derives no `Default`; `cost_tally_rules` and `streamed_unpriced_call_reports_no_cost` tests; unpriced → `None` on every path (CR-01 early-return gap found by the phase code review and fixed in-phase: all four agent-loop return paths now use `cost_tally.total()`) | closed |
| T-38-07 | DoS / log forging | `UnpricedModelWarnings` + warn line | low | mitigate | model formatted with `{model:?}` in `pricing.rs`; capacity cap with one suppression line; lock poisoning recovered via `unwrap_or_else(PoisonError::into_inner)`; `unpriced_model_warnings_warns_once_per_model_and_stops_at_capacity` test | closed |
| T-38-08 | Information Disclosure | unpriced-model warning | low | mitigate | both `log::warn!` sites interpolate only the model name and capacity; confirmed by the 38-09 credential-handling review | closed |
| T-38-09 | Tampering | `CurrencyCode` deserialization | low | mitigate | `#[serde(try_from = "String")]` on `CurrencyCode` | closed |
| T-38-10 | Denial of Service | `parse_price_nanos_per_million` (`src/config/treasurer.rs`) | medium | mitigate | `Result<i64, PriceParseError>` with checked arithmetic; non-test code has no `unwrap`/`expect`/`panic!` (the two matches are rustdoc examples); boundary tests present | closed |
| T-38-11 | Tampering (silent mis-billing) | `PriceRowConfig` | medium | mitigate | `#[serde(deny_unknown_fields)]` on the config row; verbatim-key tests through `Settings::load_from_file` | closed |
| T-38-12 | Tampering | `APP_TREASURER_CURRENCY` | low | mitigate | env override validated by `CurrencyCode::new`; tests reference the variable | closed |
| T-38-13 | Information Disclosure | `TreasurerConfig` Debug/serialization | low | accept | carries prices and a currency code only — see AR-38-02 | closed |
| T-38-SC | Tampering (supply chain) | dependencies | low | accept | no new crate this phase; `Cargo.lock` unchanged across every gate run (38-09) — see AR-38-03 | closed |
| T-38-14 | Tampering (mis-pricing) | `PricingLlmAdapter::generate` after a fallback hop | medium | mitigate | decorator composed outside `FallbackLlmAdapter`; `prices_the_served_model_after_fallback_hop` test | closed |
| T-38-15 | Repudiation (unrecorded API break) | `LlmResponse` public shape | medium | mitigate | MIGRATION.md §9.2 row for `paladin-ports` `LlmResponse.cost`; `scripts/check-migration-allowlist.sh` exits 0 (38-09) | closed |
| T-38-16 | Information Disclosure | serialized `LlmResponse` | low | accept | `cost` is an integer plus currency code, skipped when `None` — see AR-38-04 | closed |
| T-38-17 | Tampering (misleading display) | herald cost rendering | low | mitigate | single `cost_display()` formatter; `finalize_stream_omits_cost_row_when_unpriced` and `finalize_stream_unpriced_has_null_currency` tests | closed |
| T-38-18 | Information Disclosure | herald output | low | accept | renders model, token counts and cost only — see AR-38-05 | closed |
| T-38-19 | Information Disclosure | SSE `map_trace_event` | medium | mitigate | explicit field selection unchanged; `sse_payloads_carry_no_spend_field` test | closed |
| T-38-20 | Denial of Service | `TraceDispatcher` cost lock | low | mitigate | `PoisonError::into_inner` recovery pattern in the engine hooks | closed |
| T-38-21 | Tampering (spend under-report) | run total aggregation | high | mitigate | `CostTally::record_node` poisons on unpriced; `total_cost_is_none_once_any_call_is_unpriced` and `run_finished_cost_is_none_when_a_paladin_node_is_unpriced` tests | closed |
| T-38-22 | Tampering (persisted compatibility) | legacy `run_traces` rows | low | mitigate | `#[serde(default, skip_serializing_if = "Option::is_none")]`; legacy-JSON deserialization tests | closed |
| T-38-23 | Information Disclosure | `From<PaladinResult> for ExecuteResponse` | medium | mitigate | conversion unchanged; `execute_response_carries_no_cost_field` test | closed |
| T-38-24 | Tampering (spend under-report) | agent-loop and engine aggregation | high | mitigate | `agent_loop_cost_is_none_when_any_call_unpriced` (agent loop) and `unpriced_engine_run_reports_no_cost` (engine) tests; no zero defaults | closed |
| T-38-25 | Tampering (persisted compatibility) | serialized `PaladinResult` | low | mitigate | `#[serde(default, skip_serializing_if = "Option::is_none")]`; MIGRATION.md §9.2 row; legacy-JSON test | closed |
| T-38-26 | Information Disclosure | `HeraldTraceSink` log line | low | mitigate | `finalize_stream` renders metadata only; no prompt/response content or credential in `ExecutionMetadata` | closed |
| T-38-27 | Denial of Service | herald failure inside a run | low | mitigate | errors map to `TraceSinkError::Failed`; engine never fails a run on a sink error | closed |
| T-38-28 | Denial of Service | log volume | low | accept | one info line per dispatch, only when `herald:` is configured — see AR-38-06 | closed |
| T-38-29 | Denial of Service | `from_run_finished` duration arithmetic | low | mitigate | `i64::try_from(*duration_ms).unwrap_or(i64::MAX)` saturating conversion; `from_run_finished_unpriced_has_no_cost` test | closed |
| T-38-30 | Repudiation (unrecorded API break) | MIGRATION §9.2 / allowlist | medium | mitigate | `cargo semver-checks` run against 0.9.0 and 0.10.1; every lint registered; `check-migration-allowlist.sh` + `make check-gates` exit 0 (38-09) | closed |
| T-38-31 | Tampering (public-surface drift) | `.project/current-exports.txt` | medium | mitigate | regenerated with the CI-pinned nightly alongside the CHANGELOG entry; `make api-surface` exits 0 (38-09) | closed |
| T-38-32 | Information Disclosure | HTTP schema | medium | mitigate | `make openapi` leaves `crates/paladin-web/openapi.json` unchanged (38-09) | closed |
| T-38-33 | Elevation / Tampering (vulnerable dependency) | dependency graph | low | mitigate | `make security` (cargo-audit + cargo-deny) exits 0 with no new advisories; zero dependencies added (38-09) | closed |
| T-38-34 | Information Disclosure (credential leakage) | Phase 38 diff | low | mitigate | manual credential-handling review over the full phase diff recorded in 38-09-SUMMARY: no API key logged or Debug-formatted, no new HTTP client | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-38-01 | T-38-04 | ADR-0052/0053 describe architecture only; no credential, key or tenant data is written | plan 38-01 threat model | 2026-09-25 |
| AR-38-02 | T-38-13 | `TreasurerConfig` carries no secret-shaped field (prices and a currency code), matching `AgentRuntimeConfig`'s documented invariant | plan 38-03 threat model | 2026-09-25 |
| AR-38-03 | T-38-SC | No new crate: the decimal parser is hand-rolled; `Cargo.lock` unchanged across the phase | plan 38-03 threat model | 2026-09-25 |
| AR-38-04 | T-38-16 | Serialized `LlmResponse.cost` holds only an integer and a currency code, skipped when `None` | plan 38-04 threat model | 2026-09-25 |
| AR-38-05 | T-38-18 | Herald output adds model name, token counts and cost only; no prompt or response content | plan 38-05 threat model | 2026-09-25 |
| AR-38-06 | T-38-28 | One info line per engine dispatch, only when an operator configures `herald:`; default deployments attach nothing | plan 38-08 threat model | 2026-09-25 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-09-26 | 35 | 35 | 0 | /gsd-secure-phase (L1 grep-depth, orchestrator short-circuit) |

Notes from this audit:

- The phase code review (`38-REVIEW.md`) found one critical correctness bug (CR-01: the
  `before_model` early return hard-coded `cost: None`). It was fixed in-phase; all four return
  paths in `execute_internal` now report `cost_tally.total()`. The two remaining `cost: None`
  literals in that file are test-mock `LlmResponse` constructions.
- Review warnings WR-01 (`Cost::new` accepts a negative amount) and WR-02, plus IN-01, remain
  advisory in `38-REVIEW.md`; none maps to an open register threat at or above the block
  threshold.

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-09-26
