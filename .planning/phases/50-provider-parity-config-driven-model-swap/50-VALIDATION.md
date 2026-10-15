---
phase: 50
slug: provider-parity-config-driven-model-swap
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-10-15
---

# Phase 50 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `cargo test` (unit tests in `#[cfg(test)]` modules, integration tests in `tests/`), `mockito` 1.7 mock transports, `tokio::test`; doc tests via `cargo test --doc` |
| **Config file** | root `Cargo.toml` `[[test]]` entries (`provider_swap_matrix` with `required-features = ["llm-all"]`, `consumer_surface_muster`); `crates/paladin-llm/Cargo.toml` `[[example]] live_vendor_smoke` |
| **Quick run command** | `cargo test -p paladin-llm --all-features --lib <module>` (e.g. `sampling::`, `registry::`, `conformance_suite`, `fallback::`), `cargo test --lib paladin_builder` |
| **Full suite command** | `cargo test --workspace --features integration-tests,llm-all` then `make clean-code check-gates api-surface security check-consumer-surface check-migration-allowlist check-api-examples` |
| **Estimated runtime** | ~50 seconds per quick command after the first build; the full suite runs at plan and phase gates |

---

## Sampling Rate

- **After every task commit:** Run the task's `<automated>` command (a targeted `cargo test` filter), plus `cargo fmt --check` and `cargo clippy -p <crate> --all-features -- -D warnings`
- **After every plan wave:** Run `cargo test --workspace --features integration-tests,llm-all` and `cargo test --test provider_swap_matrix --features llm-all`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 50 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 50-01-01 | 01 | 1 | PARITY-01 | T-50-01-01 | see plan threat model | e2e (mock transport) | `cargo test --test provider_swap_matrix --features llm-all openai_prompt_reaches_the_wire && cargo test -p paladin-llm --lib openai_user_t...` | ✅ | ⬜ pending |
| 50-01-02 | 01 | 1 | PARITY-01 | T-50-01-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib user_turn_appends_context && cargo test -p paladin-llm --all-features --lib` | ✅ | ⬜ pending |
| 50-01-03 | 01 | 1 | PARITY-01 | T-50-01-01 | see plan threat model | unit/integration | `cargo test --test provider_swap_matrix --features llm-all && test -f .planning/phases/50-provider-parity-config-driven-model-swap/50-MATR...` | ✅ | ⬜ pending |
| 50-02-01 | 02 | 1 | PARITY-02 | T-50-02-01 | see plan threat model | unit/integration | `test "$(ls .planning/decisions/0061-*.md \| wc -l)" -eq 1 && grep -q 'Amends ADR-0004' .planning/decisions/0061-per-model-parameter-suppo...` | ✅ | ⬜ pending |
| 50-02-02 | 02 | 1 | PARITY-02 | T-50-02-01 | see plan threat model | unit/integration | `cargo test -p paladin-ports --lib model_profile && cargo test -p paladin-ports --lib provider_capabilities_gained_no_field && cargo test ...` | ✅ | ⬜ pending |
| 50-02-03 | 02 | 1 | PARITY-02 | T-50-02-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib forwards_model_profile && cargo test -p paladin-llm --all-features --lib fallback_model_pr...` | ✅ | ⬜ pending |
| 50-03-01 | 03 | 2 | PARITY-02 | T-50-03-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib sampling:: && cargo test -p paladin-llm --all-features --lib registry::` | ✅ | ⬜ pending |
| 50-03-02 | 03 | 2 | PARITY-02 | T-50-03-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib compat:: && cargo test -p paladin-llm --all-features --lib kimi:: && cargo test -p paladin...` | ✅ | ⬜ pending |
| 50-03-03 | 03 | 2 | PARITY-02 | T-50-03-01 | see plan threat model | unit/integration | `cargo test --lib paladin_builder && cargo test --test provider_swap_matrix --features llm-all && make api-surface` | ✅ | ⬜ pending |
| 50-04-01 | 04 | 1 | PARITY-07 | T-50-04-01 | see plan threat model | unit/integration | `cargo build -p paladin-llm --example live_vendor_smoke --features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatibl...` | ✅ | ⬜ pending |
| 50-04-02 | 04 | 1 | PARITY-07 | T-50-04-01 | see plan threat model | manual (operator keys) | `test -f .planning/phases/50-provider-parity-config-driven-model-swap/50-SEED-PROBE.raw.md && grep -c '^## ' .planning/phases/50-provider-...` | ✅ | ⬜ pending |
| 50-04-03 | 04 | 1 | PARITY-07 | T-50-04-01 | see plan threat model | unit/integration | `test -f .planning/phases/50-provider-parity-config-driven-model-swap/50-SEED-PROBE.md && grep -c 'PROBED\\|SKIPPED' .planning/phases/50-p...` | ✅ | ⬜ pending |
| 50-05-01 | 05 | 3 | PARITY-02 | T-50-05-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib openai:: && cargo test -p paladin-llm --all-features --lib deepseek::` | ✅ | ⬜ pending |
| 50-05-02 | 05 | 3 | PARITY-02 | T-50-05-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib anthropic:: && cargo test -p paladin-llm --all-features --lib gemini::` | ✅ | ⬜ pending |
| 50-05-03 | 05 | 3 | PARITY-02 | T-50-05-01 | see plan threat model | unit/integration | `cargo test --test provider_swap_matrix --features llm-all && test "$(grep -c 'red until plan 50-05' tests/provider_swap_matrix.rs)" -eq 0` | ✅ | ⬜ pending |
| 50-06-01 | 06 | 4 | PARITY-04 | T-50-06-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib seeded_tables_are_dated_unique_and_consistent && cargo test -p paladin-llm --all-features ...` | ✅ | ⬜ pending |
| 50-06-02 | 06 | 4 | PARITY-04 | T-50-06-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib default_model_is_seeded && cargo test -p paladin-llm --all-features --lib && cargo test --...` | ✅ | ⬜ pending |
| 50-06-03 | 06 | 4 | PARITY-04 | T-50-06-01 | see plan threat model | unit/integration | `make api-surface && sed -n '/^## 9.1/,/^## 9.2/p' MIGRATION.md \| grep -c 'Phase 50 plan 50-06'` | ✅ | ⬜ pending |
| 50-07-01 | 07 | 5 | PARITY-04 | T-50-07-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib window:: && cargo test -p paladin-llm --all-features --lib commissary && cargo test -p pal...` | ✅ | ⬜ pending |
| 50-07-02 | 07 | 5 | PARITY-04 | T-50-07-01 | see plan threat model | unit/integration | `cargo test --lib history && make api-surface` | ✅ | ⬜ pending |
| 50-08-01 | 08 | 6 | PARITY-03 | T-50-08-01 | see plan threat model | decision | `manual` | ✅ | ⬜ pending |
| 50-08-02 | 08 | 6 | PARITY-03 | T-50-08-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib compat:: && cargo test -p paladin-llm --all-features --lib debug_redacts_the_api_key && ca...` | ✅ | ⬜ pending |
| 50-08-03 | 08 | 6 | PARITY-03 | T-50-08-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib openai:: && cargo test -p paladin-llm --all-features --lib anthropic:: && cargo test -p pa...` | ✅ | ⬜ pending |
| 50-09-01 | 09 | 7 | PARITY-03 | T-50-09-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib config::llm && cargo test --lib config_bridge && cargo test --test provider_swap_matrix --...` | ✅ | ⬜ pending |
| 50-09-02 | 09 | 7 | PARITY-03 | T-50-09-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib config::layers && cargo test -p paladin-llm --all-features --lib config::bridge && cargo t...` | ✅ | ⬜ pending |
| 50-09-03 | 09 | 7 | PARITY-03 | T-50-09-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib provider_factory && make api-surface` | ✅ | ⬜ pending |
| 50-10-01 | 10 | 8 | PARITY-03 | T-50-10-01 | see plan threat model | unit/integration | `cargo test --lib agent_host && cargo test --lib facade_provisioner` | ✅ | ⬜ pending |
| 50-10-02 | 10 | 8 | PARITY-03 | T-50-10-01 | see plan threat model | unit/integration | `cargo test --test provider_swap_matrix --features llm-all && cargo check --no-default-features --features llm-kimi && cargo check --no-de...` | ✅ | ⬜ pending |
| 50-10-03 | 10 | 8 | PARITY-03 | T-50-10-01 | see plan threat model | unit/integration | `make check-migration-allowlist && grep -q 'Phase 50 (Provider Parity' .cargo/semver-checks-allowlist.toml && grep -q 'cargo-semver-checks...` | ✅ | ⬜ pending |
| 50-11-01 | 11 | 9 | PARITY-05 | T-50-11-01 | see plan threat model | unit/integration | `cargo test -p paladin-ports --lib streaming && cargo test -p paladin-ports --doc LlmResponse && cargo build --workspace --all-features` | ✅ | ⬜ pending |
| 50-11-02 | 11 | 9 | PARITY-05 | T-50-11-01 | see plan threat model | unit/integration | `cargo build --workspace --all-targets --all-features && cargo test -p paladin-ports --lib llm_response_without_reasoning_key_deserializes...` | ✅ | ⬜ pending |
| 50-11-03 | 11 | 9 | PARITY-05 | T-50-11-01 | see plan threat model | unit/integration | `make check-migration-allowlist && make api-surface && grep -q 'PARITY-05' .cargo/semver-checks-allowlist.toml` | ✅ | ⬜ pending |
| 50-12-01 | 12 | 10 | PARITY-05 | T-50-12-01 | see plan threat model | unit/integration | `cargo test -p paladin-ports --lib llm_error_transience_table && cargo test -p paladin-ports --doc Refusal && cargo test -p paladin-battal...` | ✅ | ⬜ pending |
| 50-12-02 | 12 | 10 | PARITY-05 | T-50-12-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib response_guards && cargo test -p paladin-llm --all-features --lib compat:: && cargo test -...` | ✅ | ⬜ pending |
| 50-12-03 | 12 | 10 | PARITY-05 | T-50-12-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib openai:: && cargo test -p paladin-llm --all-features --lib deepseek:: && cargo test --test...` | ✅ | ⬜ pending |
| 50-13-01 | 13 | 11 | PARITY-05 | T-50-13-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib anthropic::` | ✅ | ⬜ pending |
| 50-13-02 | 13 | 11 | PARITY-05 | T-50-13-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib gemini:: && cargo test --test provider_swap_matrix --features llm-all empty_stop_completio...` | ✅ | ⬜ pending |
| 50-13-03 | 13 | 11 | PARITY-05 | T-50-13-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib usage_cap && cargo test -p paladin-llm --all-features --lib && sed -n '/^## 9.1/,/^## 9.2/...` | ✅ | ⬜ pending |
| 50-14-01 | 14 | 12 | PARITY-06 | T-50-14-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib fallback:: && cargo test -p paladin-llm --all-features --doc FallbackStep` | ✅ | ⬜ pending |
| 50-14-02 | 14 | 12 | PARITY-06 | T-50-14-01 | see plan threat model | unit/integration | `cargo test --lib agent_runtime && cargo test --lib resilience && cargo test --test provider_swap_matrix --features llm-all && test "$(car...` | ✅ | ⬜ pending |
| 50-14-03 | 14 | 12 | PARITY-06 | T-50-14-01 | see plan threat model | unit/integration | `make check-migration-allowlist && make api-surface && make check-consumer-surface && sed -n '/^## 9.2/,/^## 9.3/p' MIGRATION.md \| grep -...` | ✅ | ⬜ pending |
| 50-15-01 | 15 | 13 | PARITY-07 | T-50-15-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib conformance_suite` | ✅ | ⬜ pending |
| 50-15-02 | 15 | 13 | PARITY-07 | T-50-15-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib conformance && cargo test -p paladin-llm --all-features --lib conformance_suite 2>&1 \| gr...` | ✅ | ⬜ pending |
| 50-16-01 | 16 | 14 | PARITY-07 | T-50-16-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib switching_providers_page_lists_every_seeded_model && grep -q 'user-guides/switching-provid...` | ✅ | ⬜ pending |
| 50-16-02 | 16 | 14 | PARITY-07 | T-50-16-01 | see plan threat model | unit/integration | `./scripts/check-changelogs.sh && make check-migration-allowlist && make check-consumer-surface && sed -n '/### Library consumers/,$p' MIG...` | ✅ | ⬜ pending |
| 50-16-03 | 16 | 14 | PARITY-07 | T-50-16-01 | see plan threat model | unit/integration | `cargo test --workspace --features integration-tests,llm-all && make clean-code && make check-gates && make api-surface && make security &...` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `tests/provider_swap_matrix.rs` + `[[test]] provider_swap_matrix` (`required-features = ["llm-all"]`) — PARITY-01, PARITY-08 (plan 50-01, wave 1; red cases ignored with their fixing plan, red run recorded in `50-MATRIX-RED-EVIDENCE.md`)
- [ ] `openai_user_turn_carries_query_on_the_execution_service_path` (red first) — PARITY-08 (plan 50-01)
- [ ] `crates/paladin-ports/src/output/model_profile.rs` table tests — PARITY-02 (plan 50-02, wave 1)
- [ ] `crates/paladin-llm/src/sampling.rs`, `crates/paladin-llm/src/registry/mod.rs` table tests — PARITY-02, PARITY-04 (plan 50-03)
- [ ] `seeded_tables_are_dated_unique_and_consistent` — PARITY-04 (plan 50-06)
- [ ] `Wire::Anthropic` fixture and six new conformance cases, `CASE_COUNT` 16 — PARITY-07 (plan 50-15)
- [ ] `switching_providers_page_lists_every_seeded_model` drift test — PARITY-07 (plan 50-16)

Existing infrastructure (cargo test, mockito, the conformance macro, the consumer muster) covers everything else; no framework install is needed.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Operator runs the seed probe on real vendor keys and hands back the transcript | PARITY-04, PARITY-05 | no vendor key exists in this environment; vendor doc hosts are blocked (D-08, D-11a) | plan 50-04 Task 2 instructions; Task 3 scrubs and commits `50-SEED-PROBE.md` |
| Owner chooses the public shape of the nine `*Config` types | PARITY-03 | one-way public-API door left open at discuss (D-12, RESEARCH OQ2) | plan 50-08 Task 1 checkpoint: option-a (`#[non_exhaustive]` + builders) or option-b (plain fields) |
| Live smoke over the nine providers on real keys | PARITY-07 | credential-gated, never run in CI | `cargo run -q -p paladin-llm --example live_vendor_smoke --features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatible`; every configured provider passes, absent ones read `SKIPPED` |
| 82 % workspace line-coverage floor | all | `cargo-llvm-cov` is not installed locally | read from CI's `coverage` job after push (ADR-0006) |
| mdBook build of the Switching providers page | PARITY-07 | `mdbook` is not installed locally | CI docs job after push |

*If none: "All phase behaviors have automated verification."*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (checkpoint tasks verify their hand-back)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (no `MISSING` automated commands)
- [x] No watch-mode flags
- [ ] Feedback latency < 50s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** {pending / approved YYYY-MM-DD}
