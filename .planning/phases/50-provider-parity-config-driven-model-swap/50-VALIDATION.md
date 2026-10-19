---
phase: 50
slug: provider-parity-config-driven-model-swap
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-10-15
audited: 2026-10-19
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
| 50-01-01 | 01 | 1 | PARITY-01 | T-50-01-01 | see plan threat model | e2e (mock transport) | `cargo test --test provider_swap_matrix --features llm-all openai_prompt_reaches_the_wire && cargo test -p paladin-llm --lib openai_user_t...` | ✅ | ✅ green |
| 50-01-02 | 01 | 1 | PARITY-01 | T-50-01-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib user_turn_appends_context && cargo test -p paladin-llm --all-features --lib` | ✅ | ✅ green |
| 50-01-03 | 01 | 1 | PARITY-01 | T-50-01-01 | see plan threat model | unit/integration | `cargo test --test provider_swap_matrix --features llm-all && test -f .planning/phases/50-provider-parity-config-driven-model-swap/50-MATR...` | ✅ | ✅ green |
| 50-02-01 | 02 | 1 | PARITY-02 | T-50-02-01 | see plan threat model | unit/integration | `test "$(ls .planning/decisions/0061-*.md \| wc -l)" -eq 1 && grep -q 'Amends ADR-0004' .planning/decisions/0061-per-model-parameter-suppo...` | ✅ | ✅ green |
| 50-02-02 | 02 | 1 | PARITY-02 | T-50-02-01 | see plan threat model | unit/integration | `cargo test -p paladin-ports --lib model_profile && cargo test -p paladin-ports --lib provider_capabilities_gained_no_field && cargo test ...` | ✅ | ✅ green |
| 50-02-03 | 02 | 1 | PARITY-02 | T-50-02-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib forwards_model_profile && cargo test -p paladin-llm --all-features --lib fallback_model_pr...` | ✅ | ✅ green |
| 50-03-01 | 03 | 2 | PARITY-02 | T-50-03-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib sampling:: && cargo test -p paladin-llm --all-features --lib registry::` | ✅ | ✅ green |
| 50-03-02 | 03 | 2 | PARITY-02 | T-50-03-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib compat:: && cargo test -p paladin-llm --all-features --lib kimi:: && cargo test -p paladin...` | ✅ | ✅ green |
| 50-03-03 | 03 | 2 | PARITY-02 | T-50-03-01 | see plan threat model | unit/integration | `cargo test --lib paladin_builder && cargo test --test provider_swap_matrix --features llm-all && make api-surface` | ✅ | ✅ green |
| 50-04-01 | 04 | 1 | PARITY-07 | T-50-04-01 | see plan threat model | unit/integration | `cargo build -p paladin-llm --example live_vendor_smoke --features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatibl...` | ✅ | ✅ green |
| 50-04-02 | 04 | 1 | PARITY-07 | T-50-04-01 | see plan threat model | unit/integration | `cargo build -p paladin-llm --example live_vendor_smoke --features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatibl...` | ✅ | ✅ green |
| 50-05-01 | 05 | 3 | PARITY-02 | T-50-05-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib openai:: && cargo test -p paladin-llm --all-features --lib deepseek::` | ✅ | ✅ green |
| 50-05-02 | 05 | 3 | PARITY-02 | T-50-05-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib anthropic:: && cargo test -p paladin-llm --all-features --lib gemini::` | ✅ | ✅ green |
| 50-05-03 | 05 | 3 | PARITY-02 | T-50-05-01 | see plan threat model | unit/integration | `cargo test --test provider_swap_matrix --features llm-all && test "$(grep -c 'red until plan 50-05' tests/provider_swap_matrix.rs)" -eq 0` | ✅ | ✅ green |
| 50-06-01 | 06 | 4 | PARITY-04 | T-50-06-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib seeded_tables_are_dated_unique_and_consistent && cargo test -p paladin-llm --all-features ...` | ✅ | ✅ green |
| 50-06-02 | 06 | 4 | PARITY-04 | T-50-06-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib default_model_is_seeded && cargo test -p paladin-llm --all-features --lib && cargo test --...` | ✅ | ✅ green |
| 50-06-03 | 06 | 4 | PARITY-04 | T-50-06-01 | see plan threat model | unit/integration | `make api-surface && sed -n '/^## 9.1/,/^## 9.2/p' MIGRATION.md \| grep -c 'Phase 50 plan 50-06'` | ✅ | ✅ green |
| 50-07-01 | 07 | 5 | PARITY-04 | T-50-07-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib window:: && cargo test -p paladin-llm --all-features --lib commissary && cargo test -p pal...` | ✅ | ✅ green |
| 50-07-02 | 07 | 5 | PARITY-04 | T-50-07-01 | see plan threat model | unit/integration | `cargo test --lib history && make api-surface` | ✅ | ✅ green |
| 50-08-01 | 08 | 6 | PARITY-03 | T-50-08-01 | see plan threat model | decision | `manual` (owner chose option-a; all nine `*Config` types are `#[non_exhaustive]` with `with_*` builders, confirmed in 50-VERIFICATION) | ✅ | ✅ green |
| 50-08-02 | 08 | 6 | PARITY-03 | T-50-08-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib compat:: && cargo test -p paladin-llm --all-features --lib debug_redacts_the_api_key && ca...` | ✅ | ✅ green |
| 50-08-03 | 08 | 6 | PARITY-03 | T-50-08-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib openai:: && cargo test -p paladin-llm --all-features --lib anthropic:: && cargo test -p pa...` | ✅ | ✅ green |
| 50-09-01 | 09 | 7 | PARITY-03 | T-50-09-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib config::llm && cargo test --lib config_bridge && cargo test --test provider_swap_matrix --...` | ✅ | ✅ green |
| 50-09-02 | 09 | 7 | PARITY-03 | T-50-09-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib config::layers && cargo test -p paladin-llm --all-features --lib config::bridge && cargo t...` | ✅ | ✅ green |
| 50-09-03 | 09 | 7 | PARITY-03 | T-50-09-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib provider_factory && make api-surface` | ✅ | ✅ green |
| 50-10-01 | 10 | 8 | PARITY-03 | T-50-10-01 | see plan threat model | unit/integration | `cargo test --lib --features web-server,llm-all -- agent_host facade_provisioner` (corrected 2026-10-17: the web module is `#[cfg(feature = "web-server")]`, so the plan's featureless command matched 0 `facade_provisioner` tests and 1 unrelated `agent_host` test; corrected run: 48 passed) | ✅ | ✅ green |
| 50-10-02 | 10 | 8 | PARITY-03 | T-50-10-01 | see plan threat model | unit/integration | `cargo test --test provider_swap_matrix --features llm-all && cargo check --no-default-features --features llm-kimi && cargo check --no-de...` | ✅ | ✅ green |
| 50-10-03 | 10 | 8 | PARITY-03 | T-50-10-01 | see plan threat model | unit/integration | `make check-migration-allowlist && grep -q 'Phase 50 (Provider Parity' .cargo/semver-checks-allowlist.toml && grep -q 'cargo-semver-checks...` | ✅ | ✅ green |
| 50-11-01 | 11 | 9 | PARITY-05 | T-50-11-01 | see plan threat model | unit/integration | `cargo test -p paladin-ports --lib streaming && cargo test -p paladin-ports --doc LlmResponse && cargo build --workspace --all-features` | ✅ | ✅ green |
| 50-11-02 | 11 | 9 | PARITY-05 | T-50-11-01 | see plan threat model | unit/integration | `cargo build --workspace --all-targets --all-features && cargo test -p paladin-ports --lib llm_response_without_reasoning_key_deserializes...` | ✅ | ✅ green |
| 50-11-03 | 11 | 9 | PARITY-05 | T-50-11-01 | see plan threat model | unit/integration | `make check-migration-allowlist && make api-surface && grep -q 'PARITY-05' .cargo/semver-checks-allowlist.toml` | ✅ | ✅ green |
| 50-12-01 | 12 | 10 | PARITY-05 | T-50-12-01 | see plan threat model | unit/integration | `cargo test -p paladin-ports --lib llm_error_transience_table && cargo test -p paladin-ports --doc Refusal && cargo test -p paladin-battal...` | ✅ | ✅ green |
| 50-12-02 | 12 | 10 | PARITY-05 | T-50-12-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib response_guards && cargo test -p paladin-llm --all-features --lib compat:: && cargo test -...` | ✅ | ✅ green |
| 50-12-03 | 12 | 10 | PARITY-05 | T-50-12-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib openai:: && cargo test -p paladin-llm --all-features --lib deepseek:: && cargo test --test...` | ✅ | ✅ green |
| 50-13-01 | 13 | 11 | PARITY-05 | T-50-13-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib anthropic::` | ✅ | ✅ green |
| 50-13-02 | 13 | 11 | PARITY-05 | T-50-13-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib gemini:: && cargo test -p paladin-llm --all-features --lib empty_stream_is_empty_completio...` | ✅ | ✅ green |
| 50-13-03 | 13 | 11 | PARITY-05 | T-50-13-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib usage_cap && cargo test -p paladin-llm --all-features --lib && sed -n '/^## 9.1/,/^## 9.2/...` | ✅ | ✅ green |
| 50-14-01 | 14 | 12 | PARITY-06 | T-50-14-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib fallback:: && cargo test -p paladin-llm --all-features --doc FallbackStep` | ✅ | ✅ green |
| 50-14-02 | 14 | 12 | PARITY-06 | T-50-14-01 | see plan threat model | unit/integration | `cargo test --lib agent_runtime && cargo test --lib resilience && cargo test --test provider_swap_matrix --features llm-all && IGNORED="$(...` | ✅ | ✅ green |
| 50-14-03 | 14 | 12 | PARITY-06 | T-50-14-01 | see plan threat model | unit/integration | `make check-migration-allowlist && make api-surface && make check-consumer-surface && sed -n '/^## 9.2/,/^## 9.3/p' MIGRATION.md \| grep -...` | ✅ | ✅ green |
| 50-15-01 | 15 | 13 | PARITY-07 | T-50-15-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib conformance_suite` | ✅ | ✅ green |
| 50-15-02 | 15 | 13 | PARITY-07 | T-50-15-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib conformance && cargo test -p paladin-llm --all-features --lib conformance_suite 2>&1 \| gr...` | ✅ | ✅ green |
| 50-16-01 | 16 | 15 | PARITY-07 | T-50-16-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib switching_providers_page_lists_every_seeded_model && grep -q 'user-guides/switching-provid...` | ✅ | ✅ green |
| 50-16-02 | 16 | 15 | PARITY-07 | T-50-16-01 | see plan threat model | unit/integration | `./scripts/check-changelogs.sh && make check-migration-allowlist && make check-consumer-surface && sed -n '/### Library consumers/,$p' MIG...` | ✅ | ✅ green |
| 50-16-03 | 16 | 15 | PARITY-07 | T-50-16-01 | see plan threat model | unit/integration | `cargo test --workspace --features integration-tests,llm-all && make clean-code && make check-gates && make api-surface && make security &...` | ✅ | ❌ red (2026-10-17: workspace 7861 passed / 2 failed, the ledgered WINDOWS row 73 `run_api_wiring` pair; `make security` fails on RUSTSEC-2026-0367/0368/0369 in `zip 3.0.0`, a build-dependency of `utoipa-swagger-ui` dating from Phase 43, published 2026-10-09/10 after the 50-16 gate passed; not a Phase 50 change. Every other clause green) |
| 50-17-01 | 17 | 3 | PARITY-04 | T-50-17-01 | see plan threat model | manual (operator keys); post-checkpoint assertion of the five R-2 lines | `F=.planning/phases/50-provider-parity-config-driven-model-swap/50-SEED-PROBE.md; test "$(grep '^## ' $F \| grep -vEc '^## (Owner reading R-2\|Operator-supplied evidence)')" -ge 9 && grep -q '^## Owner reading R-2' $F && <one R-2 line per compat preset>` (corrected 2026-10-17: the raw transcript was renamed into the curated `50-SEED-PROBE.md` by design in `7702a80b`; the original assertion also passes against the raw file at `2e941ef8`) | ✅ | ✅ green |
| 50-17-02 | 17 | 3 | PARITY-04 | T-50-17-01 | see plan threat model | unit/integration | `test -f .planning/phases/50-provider-parity-config-driven-model-swap/50-SEED-PROBE.md && grep -c 'PROBED\\|SKIPPED' .planning/phases/50-p...` | ✅ | ✅ green |
| 50-18-01 | 18 | 14 | PARITY-02 | T-50-18-01 | see plan threat model | decision; post-checkpoint assertion (R-1 row names option-a/option-b with a date, no `pending`) | `test "$(grep -Ec '^\\| R-1 \\|' .planning/decisions/0061-per-model-parameter-support-policy.md)" -eq 1 && grep -E '^\\| R-1 \\|' .plannin...` | ✅ | ✅ green |
| 50-18-02 | 18 | 14 | PARITY-02 | T-50-18-01 | see plan threat model | unit/integration | `grep -Eq '^\\| R-1 \\|' .planning/decisions/0061-per-model-parameter-support-policy.md && test "$(grep -E '^\\| R-1 \\|' .planning/decisi...` | ✅ | ✅ green |
| 50-18-03 | 18 | 14 | PARITY-02 | T-50-18-01 | see plan threat model | unit/integration | `cargo test -p paladin-ports --lib streaming && cargo test -p paladin-llm --all-features --lib stream_terminal_chunk_carries_metadata && c...` | ✅ | ✅ green |
| 50-19-01 | 19 | 1 | PARITY-04, PARITY-03 | T-50-19-01 | see plan threat model | e2e (mock transport) | `cargo test -p paladin-llm --all-features --lib openai::adapter` | ✅ | ✅ green |
| 50-19-02 | 19 | 1 | PARITY-04, PARITY-03 | T-50-19-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib openai:: && cargo test -p paladin-llm --all-features --lib deepseek:: && cargo test -p pal...` | ✅ | ✅ green |
| 50-19-03 | 19 | 1 | PARITY-04, PARITY-03 | T-50-19-01 | see plan threat model | unit/integration | `grep -Eq '^\\| 75 \\|.*\\| fixed \\|' .planning/WINDOWS.md && grep -q 'max_completion_tokens' docs/src/user-guides/switching-providers.md && ...` | ✅ | ✅ green |
| 50-20-01 | 20 | 2 | PARITY-02, PARITY-04 | T-50-20-01 | see plan threat model | e2e (mock transport) | `cargo test -p paladin-llm --features kimi --lib kimi::` | ✅ | ✅ green |
| 50-20-02 | 20 | 2 | PARITY-02, PARITY-04 | T-50-20-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib kimi:: && cargo test -p paladin-llm --all-features --lib capability_invariants && cargo te...` | ✅ | ✅ green |
| 50-20-03 | 20 | 2 | PARITY-02, PARITY-04 | T-50-20-01 | see plan threat model | unit/integration | `grep -Eq '^\\| 76 \\|.*\\| fixed \\|' .planning/WINDOWS.md && grep -E '^\\| kimi \\| \(adapter-wide fallback\)' docs/src/user-guides/switching-...` | ✅ | ✅ green |
| 50-21-01 | 21 | 3 | PARITY-05, PARITY-07 | T-50-21-01 | see plan threat model | e2e (mock transport) | `cargo test -p paladin-llm --features grok --lib grok_reasoning_tokens_reach_completion_and_the_price && cargo test -p paladin-llm --all-f...` | ✅ | ✅ green |
| 50-21-02 | 21 | 3 | PARITY-05, PARITY-07 | T-50-21-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib usage_reconcile && cargo test -p paladin-llm --all-features --lib compat:: && cargo test -...` | ✅ | ✅ green |
| 50-21-03 | 21 | 3 | PARITY-05, PARITY-07 | T-50-21-01 | see plan threat model | unit/integration | `grep -q 'paladin.usage_unreconciled' docs/src/user-guides/switching-providers.md && sed -n '/^## 9.1/,/^## 9.2/p' MIGRATION.md \| grep -q ...` | ✅ | ✅ green |
| 50-22-01 | 22 | 3 | PARITY-05, PARITY-07 | T-50-22-01 | see plan threat model | e2e (mock transport) | `cargo test -p paladin-llm --example live_vendor_smoke --features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatible...` | ✅ | ✅ green |
| 50-22-02 | 22 | 3 | PARITY-05, PARITY-07 | T-50-22-01 | see plan threat model | manual (operator keys); post-checkpoint assertion | `F=.planning/phases/50-provider-parity-config-driven-model-swap/50-USAGE-PROBE.md; test -f $F && test "$(grep -c '^## ' $F)" -ge 9` (corrected 2026-10-19: the raw transcript is deleted by design in Task 3, whose verify asserts its absence; the original assertion passes on the raw file at `b249d25e`, 9 sections) | ✅ | ✅ green |
| 50-22-03 | 22 | 3 | PARITY-05, PARITY-07 | T-50-22-01 | see plan threat model | doc/ledger assertion | `F=.planning/phases/50-provider-parity-config-driven-model-swap/50-USAGE-PROBE.md && test -f "$F" && grep -q '^### Containment verdicts' "...` | ✅ | ✅ green |
| 50-23-01 | 23 | 4 | PARITY-05, PARITY-07 | T-50-23-01 | see plan threat model | e2e (mock transport) | `cargo test -p paladin-llm --features gemini --lib gemini_tool_use_prompt_tokens_reach_prompt_tokens && cargo test -p paladin-llm --all-fe...` | ✅ | ✅ green |
| 50-23-02 | 23 | 4 | PARITY-05, PARITY-07 | T-50-23-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib openai:: && cargo test -p paladin-llm --all-features --lib deepseek:: && cargo test -p pal...` | ✅ | ✅ green |
| 50-24-01 | 24 | 5 | PARITY-05, PARITY-07 | T-50-24-01 | see plan threat model | e2e (mock transport) | `cargo test -p paladin-llm --features grok --lib grok:: && grep -q '50-USAGE-PROBE' crates/paladin-llm/src/grok/adapter.rs` | ✅ | ✅ green |
| 50-24-02 | 24 | 5 | PARITY-05, PARITY-07 | T-50-24-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib && cargo test --test provider_swap_matrix --features llm-all` | ✅ | ✅ green |
| 50-24-03 | 24 | 5 | PARITY-05, PARITY-07 | T-50-24-01 | see plan threat model | unit/integration | `grep -q 'usage_unreconciled' docs/src/user-guides/switching-providers.md && sed -n '/^## 9.1/,/^## 9.2/p' MIGRATION.md \| grep -q 'Phase 5...` | ✅ | ✅ green |
| 50-25-01 | 25 | 6 | PARITY-05 | T-50-25-01 | see plan threat model | e2e (mock transport) | `cargo test -p paladin-ports && cargo test -p paladin-battalion --lib llm_ && cargo test --doc -p paladin-battalion && cargo test -p palad...` | ✅ | ✅ green |
| 50-25-02 | 25 | 6 | PARITY-05 | T-50-25-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib && cargo build --workspace --all-targets --all-features && cargo semver-checks check-relea...` | ✅ | ✅ green |
| 50-26-01 | 26 | 7 | PARITY-05, PARITY-07 | T-50-26-01 | see plan threat model | e2e (mock transport) | `cargo build --workspace --all-targets --all-features && cargo test -p paladin-ports && cargo test -p paladin-llm --all-features --lib com...` | ✅ | ✅ green |
| 50-26-02 | 26 | 7 | PARITY-05, PARITY-07 | T-50-26-01 | see plan threat model | unit/integration | `make check-migration-allowlist && make check-consumer-surface && make api-surface && make check-api-examples && grep -q 'Phase 50 plan 50...` | ✅ | ✅ green |
| 50-27-01 | 27 | 8 | PARITY-05, PARITY-07 | T-50-27-01 | see plan threat model | e2e (mock transport) | `cargo test -p paladin-llm --all-features --lib priced_billed_empty_completion_carries_its_cost && cargo test -p paladin-llm --all-feature...` | ✅ | ✅ green |
| 50-27-02 | 27 | 8 | PARITY-05, PARITY-07 | T-50-27-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib && cargo test -p paladin-llm --all-features --lib billed_failures_carry_their_usage && car...` | ✅ | ✅ green |
| 50-28-01 | 28 | 8 | PARITY-05 | T-50-28-01 | see plan threat model | e2e (mock transport) | `cargo test --lib billed_empty_completion_is_settled && cargo test --lib paladin_execution_service && cargo test --test provider_swap_matr...` | ✅ | ✅ green |
| 50-28-02 | 28 | 8 | PARITY-05 | T-50-28-01 | see plan threat model | unit/integration | `cargo test --lib billed_stream_failure_is_settled && cargo test --lib paladin_execution_service` | ✅ | ✅ green |
| 50-29-01 | 29 | 9 | PARITY-05 | T-50-29-01 | see plan threat model | e2e (mock transport) | `cargo test -p paladin-ai-core && cargo test -p paladin-ports && cargo test -p paladin-battalion --lib failed_paladin_attempt_records_its_...` | ✅ | ✅ green |
| 50-29-02 | 29 | 9 | PARITY-05 | T-50-29-01 | see plan threat model | unit/integration | `cargo test -p paladin-battalion --lib timed_out_attempt_records_its_incurred_spend && cargo test -p paladin-battalion --lib structured_re...` | ✅ | ✅ green |
| 50-29-03 | 29 | 9 | PARITY-05 | T-50-29-01 | see plan threat model | unit/integration | `cargo test --lib non_llm_failure_after_billed_calls_reports_their_spend && cargo test --lib billed_failure_is_recorded_in_the_attempt_spe...` | ✅ | ✅ green |
| 50-30-01 | 30 | 10 | PARITY-05, PARITY-06 | T-50-30-01 | see plan threat model | e2e (mock transport) | `cargo test -p paladin-ports && cargo test -p paladin-llm --all-features --lib fallback:: && cargo test -p paladin-llm --all-features --li...` | ✅ | ✅ green |
| 50-30-02 | 30 | 10 | PARITY-05, PARITY-06 | T-50-30-01 | see plan threat model | unit/integration | `cargo test -p paladin-llm --all-features --lib && cargo test --lib hopped_bills_survive_chain_exhaustion && cargo test --lib hopped_refus...` | ✅ | ✅ green |
| 50-31-01 | 31 | 11 | PARITY-05, PARITY-07 | T-50-31-01 | see plan threat model | e2e (mock transport) | `cargo test --test provider_swap_matrix --features llm-all billed_failures_and_hopped_refusals_reach_the_ledger && cargo test --test provi...` | ✅ | ✅ green |
| 50-31-02 | 31 | 11 | PARITY-05, PARITY-07 | T-50-31-01 | see plan threat model | unit/integration | `grep -q 'billed_usage' docs/src/user-guides/switching-providers.md && grep -q 'cost_in_usd_ticks' docs/src/user-guides/switching-provider...` | ✅ | ✅ green |
| 50-31-03 | 31 | 11 | PARITY-05, PARITY-07 | T-50-31-01 | see plan threat model | doc/ledger assertion | `grep -Eq '^\\| 74 \\|.*\\| fixed \\|' .planning/WINDOWS.md && grep -Eq '^\\| [0-9]+ \\| 50 \\| deviation \\| crates/paladin-llm/src/grok/adapter....` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] `tests/provider_swap_matrix.rs` + `[[test]] provider_swap_matrix` (`required-features = ["llm-all"]`) — PARITY-01, PARITY-08 (plan 50-01, wave 1; red cases ignored with their fixing plan, red run recorded in `50-MATRIX-RED-EVIDENCE.md`)
- [x] `openai_user_turn_carries_query_on_the_execution_service_path` (red first) — PARITY-08 (plan 50-01)
- [x] `crates/paladin-ports/src/output/model_profile.rs` table tests — PARITY-02 (plan 50-02, wave 1)
- [x] `crates/paladin-llm/src/sampling.rs`, `crates/paladin-llm/src/registry/mod.rs` table tests — PARITY-02, PARITY-04 (plan 50-03)
- [x] `seeded_tables_are_dated_unique_and_consistent` — PARITY-04 (plan 50-06)
- [x] `Wire::Anthropic` fixture and seven new conformance cases (empty output buffered and streamed included), `CASE_COUNT` 17 — PARITY-07 (plan 50-15; now 18 after the CR-02 split-SSE-frame case, `50-REVIEW-FIX.md`)
- [x] `switching_providers_page_lists_every_seeded_model` drift test — PARITY-07 (plan 50-16)

Existing infrastructure (cargo test, mockito, the conformance macro, the consumer muster) covers everything else; no framework install is needed.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Operator runs the seed probe on real vendor keys and hands back the transcript | PARITY-04, PARITY-05 | no vendor key exists in this environment; vendor doc hosts are blocked (D-08, D-11a) | plan 50-17 Task 1 instructions (wave 3, beside 50-05; harness from plan 50-04); 50-17 Task 2 scrubs and commits `50-SEED-PROBE.md` |
| Owner reading R-2: per compat vendor, usage cap live-verified or accepted as unverified | PARITY-05 | a quota-exhaustion error cannot be provoked on demand; only the owner can accept a best-effort SC-5 usage-cap clause (D-11a) | plan 50-17 Task 1 resume signal (one `R-2 <preset>: captured <date> / accept unverified / re-probe` line per compat vendor), written to the raw transcript's `## Owner reading R-2` section. Task 1's verify asserts exactly one valid line per preset. Recorded in `50-SEED-PROBE.md`, ADR-0061 `## Owner readings` and the 50-13 SUMMARY |
| Owner chooses the public shape of the nine `*Config` types | PARITY-03 | one-way public-API door left open at discuss (D-12, RESEARCH OQ2) | plan 50-08 Task 1 checkpoint: option-a (`#[non_exhaustive]` + builders) or option-b (plain fields) |
| Owner reading R-1: what D-01/D-07's "run-metadata entry" is | PARITY-02, PARITY-04 | one-way public-API door (a field on the constructible `PaladinResult`); only the owner can say whether per-call `LlmResponse.metadata` suffices | plan 50-18 Task 1 checkpoint: option-a (per-call metadata only) or option-b (also `PaladinResult.run_metadata` and the streaming terminal chunk). The answer is written to ADR-0061's `## Owner readings` R-1 row when the checkpoint resumes. Task 1's verify fails while that row reads `pending` and passes once it names option-a or option-b with a date |
| Live smoke over the nine providers on real keys | PARITY-07 | credential-gated, never run in CI | `cargo run -q -p paladin-llm --example live_vendor_smoke --features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatible`; every configured provider passes, absent ones read `SKIPPED` |
| 82 % workspace line-coverage floor | all | `cargo-llvm-cov` is not installed locally | read from CI's `coverage` job after push (ADR-0006) |
| mdBook build of the Switching providers page | PARITY-07 | `mdbook` is not installed locally | CI docs job after push |
| `run_api_wiring` trace-persistence pair (`build_run_api_persists_no_run_traces_by_default`, `build_run_api_persists_run_traces_when_trace_persist_is_set`) | none (pre-existing) | fail in this sandbox with "the run never reached a terminal status" (10 s worker deadline); reproduced at pre-Phase-50 commit `28b6a69b` (WINDOWS row 73) | CI on the pushed branch is the proof of record |
| Operator runs `live_vendor_smoke --usage` on real vendor keys and hands back the transcript | PARITY-05, PARITY-07 | no vendor key exists in this environment | plan 50-22 Task 2 checkpoint; Task 3 scrubs and commits `50-USAGE-PROBE.md` (done 2026-10-18, `4d76c807`); the dry run (`--usage --dry-run`, 9 sections) is automated |
| `make security` clean | all (phase seal) | blocked by RUSTSEC-2026-0367/0368/0369 (`zip 3.0.0`, build-dependency of `utoipa-swagger-ui` via `paladin-web`), published after the 50-16 gate passed; Phase 50 added no dependency | owner decision: upgrade `utoipa-swagger-ui`/`zip` or allowlist in `.cargo/audit.toml` with a reason. Out of scope for validation, which never modifies implementation files |

*If none: "All phase behaviors have automated verification."*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (checkpoint tasks verify their hand-back)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (no `MISSING` automated commands)
- [x] No watch-mode flags
- [x] Feedback latency < 50s (targeted filters; the unfiltered `paladin-llm --lib` run is 55 s of test time and stays a wave-level command)
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-10-17; re-approved 2026-10-19 (plans 50-19 to 50-31)

---

## Validation Audit 2026-10-17

Every task's automated command was re-run on HEAD `42feb9bd` (after the code-review fixes). Nothing was taken from SUMMARY claims. Each `cargo test` filter was also checked to match at least one test, because a filter that matches nothing still passes.

| Metric | Count |
|--------|-------|
| Gaps found | 3 |
| Resolved | 2 |
| Escalated | 1 |

| Gap | Type | Resolution |
|-----|------|------------|
| 50-10-01 command never ran the config-driven provisioning tests (`web-server` feature missing) | PARTIAL (command) | Command corrected; 48 passed |
| 50-17-01 command targets `50-SEED-PROBE.raw.md`, deleted by design in `7702a80b` | PARTIAL (command) | Re-pointed at the curated `50-SEED-PROBE.md`; passes there and on the raw file at `2e941ef8` |
| 50-16-03 phase gate red: `make security` (`zip 3.0.0` advisories) | Escalated (not a coverage gap) | Recorded under Manual-Only for an owner decision |

| Run | Result |
|-----|--------|
| `cargo test -p paladin-llm --all-features --lib` | 1140 passed, 0 failed |
| `cargo test -p paladin-llm --all-features --doc` | 23 passed |
| `cargo test --test provider_swap_matrix --features llm-all` | 50 passed, 0 ignored (`--ignored --list` is empty) |
| `cargo test -p paladin-ports --lib` / `--doc` | 254 passed / 196 passed, 94 ignored (none of the named `model_profile`, `Refusal`, `LlmResponse` doc tests is ignored) |
| Root crate filters (`paladin_builder`, `history`, `agent_runtime`, `resilience`, `paladin_execution_service`, `config_bridge`, cadence 429) | 216 passed |
| `--features web-server,llm-all` `agent_host`/`facade_provisioner` | 48 passed |
| `cargo test --test unit llm::`; `paladin-battalion` `llm_failure`/`llm_decision`; `paladin-ai-core` `execution_result` | 37 / 21 / 15 passed |
| `cargo test -p paladin-llm --example live_vendor_smoke --features <nine>` | 24 passed (incl. `the_static_model_lists_match_the_adapters`) |
| `live_vendor_smoke` no-key run and `--probe --dry-run` | 9 SKIPPED with non-zero exit / 9 sections with `json_schema` |
| `cargo check --no-default-features` `llm-kimi`, `llm-openai-compatible` | clean |
| `cargo build --workspace --all-targets --all-features` | clean |
| `cargo test --workspace --features integration-tests,llm-all --no-fail-fast` | 7861 passed, 2 failed (WINDOWS row 73), 245 ignored |
| `make clean-code`, `check-gates`, `api-surface`, `check-consumer-surface`, `check-migration-allowlist`, `check-api-examples`, `scripts/check-changelogs.sh`, `cargo fmt --all --check` | exit 0 |
| `make security` | exit 2 (3 `zip 3.0.0` vulnerabilities, see above) |
| Plan grep/file assertions (02-01, 06-03, 10-03, 11-02/03, 13-03, 14-03, 16-01/02/03, 17-02, 18-01/02) | pass |

Not re-run here: the `cargo test --workspace --no-run --features llm-all` clause of 50-08-02/03. It ran out of disk alongside the all-features build, and the full `integration-tests,llm-all` run above builds a feature superset of the same targets.

Observation (not a phase gap): the 24 tests inside `examples/live_vendor_smoke.rs` run only through `make test-llm-examples`. CI never runs example tests (`50-REVIEW-FIX.md` WR-03), so the model-list drift test there has no CI gate.

---

## Validation Audit 2026-10-19

Scope: the 36 tasks of gap-closure plans 50-19 to 50-31, added after the 2026-10-17 audit. Every automated
command was re-run on HEAD `1d195cae`, with nothing taken from SUMMARY claims. Each `cargo test` filter
was checked to match at least one passing test, because a filter that matches nothing still passes.

| Metric | Count |
|--------|-------|
| Gaps found | 1 |
| Resolved | 1 |
| Escalated | 0 |

| Gap | Type | Resolution |
|-----|------|------------|
| 50-22-02 command targets `50-USAGE-PROBE.raw.md`, deleted by design in 50-22 Task 3 | PARTIAL (command) | Re-pointed at the curated `50-USAGE-PROBE.md` (11 sections); the original passes on the raw file at `b249d25e` (9 sections) |

| Run | Result |
|-----|--------|
| `cargo test -p paladin-llm --all-features --lib` | 1202 passed, 0 failed |
| `cargo test -p paladin-llm --all-features --doc` | 23 passed |
| `cargo test -p paladin-llm --example live_vendor_smoke --features <nine>` | 33 passed |
| `cargo test --test provider_swap_matrix --features llm-all` | 52 passed, 0 ignored (incl. `case_billed_failures_and_hopped_refusals_reach_the_ledger`) |
| `cargo test -p paladin-ports` | 479 passed, 94 ignored |
| `cargo test -p paladin-battalion --lib` / `--doc` | 883 passed / 61 passed, 52 ignored |
| `cargo test -p paladin-eval --lib` | 56 passed |
| `cargo test -p paladin-ai-core` | 867 passed, 38 ignored (incl. the `cost_of_call` doc test) |
| `cargo test --lib` (root crate) | 1333 passed, 0 failed |
| Single-feature runs: `--features kimi --lib kimi::`, `--features grok --lib grok::`, `--features gemini --lib gemini_tool_use_prompt_tokens_reach_prompt_tokens` | 79 / 47 / 1 passed |
| `live_vendor_smoke --usage --dry-run` with no keys | exit 0, 9 sections |
| `make api-surface`, `check-consumer-surface`, `check-migration-allowlist`, `check-api-examples` | exit 0 |
| `cargo semver-checks check-release --baseline-version 0.9.0` (`paladin-ports`, `paladin-ai-core`) | exit 0 |
| `cargo check --workspace --all-targets --all-features` | exit 0 |
| Plan grep/file assertions (19-03, 20-03, 21-03, 22-03, 24-01, 24-03, 26-02, 31-02, 31-03) | pass |

Named filters confirmed non-empty, among others: `usage_reconcile` (8), `billed_failures_carry_their_usage` (9),
`switching_page_tests` (13), `capability_invariants` (4), and each single-test filter of plans 50-21, 50-23 and 50-26 to 50-30 (1 each).

Not re-run here: `cargo build --workspace --all-targets --all-features` (50-25-02, 50-26-01). It filled the disk, as in the
2026-10-17 audit. `cargo check` over the same targets and features passed in its place, and every test binary above was built and run.
The 50-16-03 phase gate (`make security`, `zip 3.0.0` advisories) is unchanged and stays under Manual-Only.

Observation (not a phase gap): the `cargo semver-checks` clauses in 50-25, 50-26, 50-29 and 50-30 ran 0 lints and skipped 254. The crates are at
`0.10.1` against a `0.9.0` baseline, and a pre-1.0 minor bump is treated as major, so every lint is skipped. Those clauses pass but check nothing.
The breaks are still measured and gated by `make check-migration-allowlist` and the consumer muster, both green above.
