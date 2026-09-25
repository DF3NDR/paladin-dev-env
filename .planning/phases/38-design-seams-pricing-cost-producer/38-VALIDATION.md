---
phase: 38
slug: design-seams-pricing-cost-producer
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-09-25
updated: 2026-09-25
---

# Phase 38 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Ported from `38-RESEARCH.md` §Validation Architecture and the nine PLAN.md files after the
> plan-checker pass (2026-09-25) so this file and the research no longer diverge. `status` moves to
> `validated` via `/gsd-validate-phase` after execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` — native Rust unit tests (`#[cfg(test)]`), doctests, integration tests; no external test framework |
| **Config file** | none — workspace `Cargo.toml` defines test targets; coverage is `cargo llvm-cov --fail-under-lines 82` in CI's `coverage` job (ADR-0006) |
| **Quick run command** | per-crate: `cargo test -p paladin-ai-core --lib cost`, `cargo test -p paladin-llm --lib pricing`, `cargo test -p paladin-ai --lib config::treasurer`, `cargo test -p paladin-herald --lib` |
| **Full suite command** | `cargo test --workspace` (unit + doctests); `make test-all` for unit + integration; `make clean-code` for fmt + clippy + check |
| **Estimated runtime** | ~60–120 s per crate quick run (incremental); ~10–15 min full workspace `cargo test` cold, a few minutes warm |

---

## Sampling Rate

- **After every task commit:** Run the task's own `<automated>` command (each is the relevant crate's
  targeted `cargo test … --lib <module>` plus `cargo clippy -p <crate> --all-targets -- -D warnings`)
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd-verify-work`:** `make clean-code`, full `cargo test --workspace`, `make api-surface`
  (additive surface refreshed with `make api-surface-update` + CHANGELOG entry per D-00g/D-00j),
  `make check-gates` and `make security` green — sealed by plan 38-09
- **Max feedback latency:** ~120 s (single-crate incremental test + clippy)

---

## Per-Task Verification Map

Requirement IDs are from `REQUIREMENTS.md` (PRICE-01..03). Threat refs are the plans' STRIDE
register rows (`T-38-NN`). Every task carries its own `<automated>` verify; the commands below are
abbreviated — the authoritative command is the `<automated>` block in the plan.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 38-01-01 | 01 | 1 | PRICE-03 (ADR-0052 attachment point; SC4) | T-38-01, T-38-04 | ADR numbering not tampered: 0052 slot empty before write, index + ADR in one commit; no credential/tenant data in ADR text | doc-structure | `adr-parser.cjs --input .planning/decisions/0052-*.md` + heading/keyword greps + PROMOTION.md row check | ✅ (tooling exists) | ⬜ pending |
| 38-01-02 | 01 | 1 | PRICE-03 (checkpoint:decision — operator confirms ADR-0052 / ADR-0053 leanings) | — | N/A (human gate) | manual gate | — | — | ⬜ pending |
| 38-01-03 | 01 | 1 | PRICE-02 (ADR-0053 ledger model, nano-unit amounts; SC4) | T-38-02, T-38-03 | Idempotency key `(run_id, superstep, attempt)` and `BEGIN IMMEDIATE` / no-`FOR UPDATE`-on-aggregate guidance recorded so Phase 39 cannot copy a racy pattern | doc-structure | `adr-parser.cjs --input .planning/decisions/0053-*.md` + keyword greps + `Next free ADR number: 0054` + amend-at-source greps in REQUIREMENTS/ROADMAP/PROJECT | ✅ (tooling exists) | ⬜ pending |
| 38-02-01 | 02 | 2 | PRICE-02, PRICE-03 (tracer: cost end-to-end through a streamed run) | T-38-06, T-38-07, T-38-08, T-38-09 | Unpriced → `None` never zero; `CostTally` poisons on unpriced/currency mismatch; warn line escapes model name, capped set, no request content | tracer (unit + doctest) | `cargo test -p paladin-ai --lib streamed_cost_tests && cargo test -p paladin-llm --lib pricing && cargo test -p paladin-ai-core --lib cost && cargo test -p paladin-ai-core --doc herald && cargo test -p paladin-ports --doc && cargo test -p paladin-herald --lib markdown_herald && cargo clippy … -D warnings` | ❌ W0 (`cost.rs`, `pricing.rs`) | ⬜ pending |
| 38-02-02 | 02 | 2 | PRICE-02 (pure cost function, per-axis + rounding + saturation) | T-38-05 | `i128` products, saturating `i64` conversion, clamped sub-counts, no `unwrap`/`expect`/`panic!` | unit + doctest (TDD) | `cargo test -p paladin-ai-core --lib cost && cargo test -p paladin-ai-core --doc cost && cargo clippy -p paladin-ai-core --all-targets -- -D warnings` | ❌ W0 (`cost.rs`) | ⬜ pending |
| 38-03-01 | 03 | 3 | PRICE-01 (`treasurer:` config, decimal parsing, validation, env override) | T-38-10, T-38-11, T-38-12, T-38-13, T-38-SC | Result-returning checked parser; `deny_unknown_fields`; required prompt/completion; >9 decimals rejected; env currency validated by same rule; no new crate | unit (TDD) | `cargo test -p paladin-ai --lib config::treasurer && … config::settings && … config::agent_runtime && cargo clippy -p paladin-ai --all-targets -- -D warnings` | ❌ W0 (`src/config/treasurer.rs`) | ⬜ pending |
| 38-03-02 | 03 | 3 | PRICE-03 (decorator installed at both facade call sites) | — | Empty table → decorator inert / not installed; both run paths wrapped identically | unit + build | `cargo test -p paladin-ai --lib --features web-server infrastructure::web::agent_host && … facade_provisioner && cargo build -p paladin-ai --features web-server --bin paladin-server && cargo clippy …` | ✅ (existing modules) | ⬜ pending |
| 38-04-01 | 04 | 3 | PRICE-03 (`LlmResponse.cost` additive field; compiler-driven propagation) | T-38-15, T-38-16 | §9.2 row + semver allowlist entry in the same commit as the field; `cost` skipped when `None`, integer + code only | build + doctest | `cargo check --workspace --all-targets --all-features && cargo test -p paladin-ports --doc && cargo test -p paladin-llm --doc && cargo test -p paladin-ports --lib llm_port && ./scripts/check-migration-allowlist.sh` | ✅ | ⬜ pending |
| 38-04-02 | 04 | 3 | PRICE-03 (fallback hop prices the served model) | T-38-14 | Price from served `response.model`, composed outside `FallbackLlmAdapter` | unit (TDD) | `cargo test -p paladin-llm --lib pricing && cargo clippy -p paladin-llm --all-targets -- -D warnings` | ❌ W0 (`pricing.rs`, from 38-02) | ⬜ pending |
| 38-05-01 | 05 | 3 | PRICE-03 (JSON herald currency-aware cost) | T-38-17, T-38-18 | Single `cost_display()`; currency code, never `$`; unpriced → null, never `0.0000` | unit (TDD) | `cargo test -p paladin-herald --lib json_herald && cargo clippy -p paladin-herald --all-targets -- -D warnings` | ✅ | ⬜ pending |
| 38-05-02 | 05 | 3 | PRICE-03 (table herald consumes real metadata) | T-38-17 | Placeholder rows removed; no Cost row when unpriced | unit (TDD) | `cargo test -p paladin-herald --features table --lib table_herald && cargo clippy -p paladin-herald --all-targets --features table -- -D warnings` | ✅ | ⬜ pending |
| 38-06-01 | 06 | 3 | PRICE-03 (`NodeFinished`/`RunFinished.cost` additive trace fields) | T-38-19, T-38-22 | SSE `map_trace_event` field selection unchanged (`sse_payloads_carry_no_spend_field`); legacy `run_traces` JSON still deserializes; goldens unchanged | build + unit + golden | `cargo check --workspace --all-targets --all-features && cargo test -p paladin-ai-core --lib trace && … --doc && cargo test -p paladin-ports --doc && cargo test -p paladin-battalion --test export_golden && cargo test -p paladin-eval && cargo test -p paladin-ai --lib run::events` | ✅ | ⬜ pending |
| 38-06-02 | 06 | 3 | PRICE-03 (`TraceDispatcher::total_cost` twin of `total_usage`) | T-38-20, T-38-21 | `PoisonError::into_inner` recovery; `CostTally` poisons on unpriced call with non-zero usage (`total_cost_is_none_once_any_call_is_unpriced`) | unit (TDD) | `cargo test -p paladin-battalion --lib engine::hooks && cargo test -p paladin-battalion --lib engine && cargo clippy -p paladin-battalion --all-targets -- -D warnings` | ✅ | ⬜ pending |
| 38-07-01 | 07 | 4 | PRICE-03 (`PaladinResult.cost` additive field) | T-38-23, T-38-25 | `ExecuteResponse` conversion unchanged (`execute_response_carries_no_cost_field`); legacy JSON deserializes | build + unit | `cargo check --workspace --all-targets --all-features && cargo test -p paladin-ai-core --doc && cargo test -p paladin-ports --doc && cargo test -p paladin-ai-core --lib execution_result && cargo test -p paladin-web --lib agent_controller` | ✅ | ⬜ pending |
| 38-07-02 | 07 | 4 | PRICE-03 (agent-loop aggregation into `PaladinResult.cost`) | T-38-24 | `CostTally` poisons on any unpriced call; no zero default | unit (TDD) | `cargo test -p paladin-ai --lib paladin_execution_service && cargo clippy -p paladin-ai --all-targets -- -D warnings` | ✅ | ⬜ pending |
| 38-07-03 | 07 | 4 | PRICE-03 (engine aggregation into `RunFinished.cost`) | T-38-24 | Same poisoning rule on the engine path | unit (TDD) | `cargo test -p paladin-battalion --lib engine && cargo clippy -p paladin-battalion --all-targets -- -D warnings` | ✅ | ⬜ pending |
| 38-08-01 | 08 | 5 | PRICE-03 (`ExecutionMetadata::from_run_finished` producer; rustdoc "produced by the Treasurer") | T-38-29 | Saturating `u64`→`i64` duration; no `unwrap`/`expect` | unit + doctest (TDD) | `cargo test -p paladin-ai-core --lib herald && cargo test -p paladin-ai-core --doc herald && cargo clippy -p paladin-ai-core --all-targets -- -D warnings` | ✅ | ⬜ pending |
| 38-08-02 | 08 | 5 | PRICE-03 (`HeraldTraceSink` hands metadata to `Herald::finalize_stream` on both paths) | T-38-26, T-38-27, T-38-28 | Metadata only in the log line (no prompt/response content); sink errors are `TraceSinkError::Failed`, never fail a run; only attached when `herald:` configured | unit + build | `cargo test -p paladin-ai --lib telemetry::herald_sink && cargo test -p paladin-ai --lib run::worker && cargo build -p paladin-ai --features web-server && cargo clippy … --features web-server -- -D warnings` | ❌ W0 (`telemetry/herald_sink.rs`) | ⬜ pending |
| 38-09-01 | 09 | 6 | PRICE-01..03 (semver-checks against 0.9.0 baseline; MIGRATION §9.2 / allowlist set-equality) | T-38-30 | Every reported lint registered; `check-migration-allowlist.sh` + `make check-gates` | gate | `for pkg in …; do cargo semver-checks check-release --package "$pkg" --default-features --baseline-version 0.9.0; done && ./scripts/check-migration-allowlist.sh && make check-gates` | ✅ | ⬜ pending |
| 38-09-02 | 09 | 6 | PRICE-01..03 (phase gate: fmt, full tests, clean-code, api-surface, CHANGELOG, exports) | T-38-31, T-38-32, T-38-33, T-38-34 | `make api-surface-update` with CI-pinned nightly in the same commit as CHANGELOG; `make openapi` diff clean (no HTTP schema gained `cost`); `make security`; manual credential-handling review recorded in SUMMARY | gate | `cargo fmt --check && cargo test --workspace && make clean-code && PUBLIC_API_TOOLCHAIN=… make api-surface && make check-gates && grep '^## \[Unreleased\]' CHANGELOG.md && grep 'cost::Cost' .project/current-exports.txt && …` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Every "❌ W0" row above is a **new source file created by the task that references it**, in
TDD order (test written first inside the new file's `#[cfg(test)]` module). No separate Wave 0
plan is needed — plan 38-01 is docs-only and plan 38-02 (wave 2) creates the first two files.

- [ ] `crates/paladin-core/src/platform/container/cost.rs` — `Cost` / `CurrencyCode` / `PriceRow` /
      `PriceTable` / pure cost function + unit tests and doctests (PRICE-02) — created by 38-02
- [ ] `crates/paladin-llm/src/pricing.rs` — `PricingLlmAdapter` decorator + `UnpricedModelWarnings`
      + unit tests including `unpriced_model_warns_once` and (38-04)
      `prices_the_served_model_after_fallback_hop` (PRICE-03) — created by 38-02
- [ ] `src/config/treasurer.rs` — `TreasurerConfig` / `PriceRowConfig` + `Default` / `validate()` /
      `EnvOverridable` + unit tests (PRICE-01) — created by 38-03
- [ ] `src/telemetry/herald_sink.rs` (or the module 38-08 names) — `HeraldTraceSink` + unit tests
      (PRICE-03) — created by 38-08
- [x] Shared fixtures: none new. `crates/paladin-llm/src/mock.rs`'s `MockLlmAdapter`
      (`.with_response()`, `.with_provider_name()`, `.with_error()`) already covers the decorator and
      fallback-hop scenarios — reuse, do not duplicate.
- [x] Framework install: none — `cargo test` is native.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Operator confirms the ADR-0052 / ADR-0053 leanings before they are written as `Accepted` | PRICE-03 (SC4) | Architecture decisions are the operator's to lock (CONTEXT D-13/D-14/D-16) | Plan 38-01 Task 2 is a `checkpoint:decision` gate: review the two ADR drafts' Decision and Considered Options sections, confirm or redirect, then execution continues |
| Manual credential-handling review of the Phase 38 diff | PRICE-01..03 (security instructions) | No merge-gating Rust SAST exists; CodeQL is advisory-only (`security.instructions.md`) | Per `.github/instructions/security.instructions.md`: confirm no price-table or currency value is ever logged beside an API key, `TreasurerConfig` carries no secret-shaped field, and the unpriced-model warn line names only the model string; record the review in 38-09's SUMMARY |

All other phase behaviors have automated verification.

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (plan-checker pass 2026-09-25:
      38-01 through 38-09 pass Nyquist checks 8a–8c; the only task without an automated command is
      the human `checkpoint:decision` gate 38-01-02)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (each new file is created by the task that first tests it)
- [x] No watch-mode flags
- [x] Feedback latency < 120 s for per-task quick commands
- [ ] `nyquist_compliant: true` set in frontmatter — set by `/gsd-validate-phase` after execution

**Approval:** pending (set by `/gsd-validate-phase` once the phase has executed)
