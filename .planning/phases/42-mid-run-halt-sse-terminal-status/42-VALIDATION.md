---
phase: 42
slug: mid-run-halt-sse-terminal-status
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-10-06
---

# Phase 42 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` -- native Rust `#[cfg(test)]` modules, doctests, `#[tokio::test]`; shared storage contract suites as plain `pub async fn` clauses; root `tests/*.rs` auto-discovered |
| **Config file** | none -- workspace `Cargo.toml` (no `autotests = false`) |
| **Quick run command** | the task's targeted filter, e.g. `cargo test -p paladin-battalion --lib spend_guard`, `cargo test -p paladin-ai --lib application::services::treasurer`, `cargo test -p paladin-ai --lib engine_spend_halt_tracer` |
| **Full suite command** | `cargo test --workspace` (PostgreSQL legs: CI `postgres-integration` only) |
| **Estimated runtime** | ~1-3 min per incremental single-crate filter; ~10-15 min cold workspace |

---

## Sampling Rate

- **After every task commit:** the task's `<automated>` verify (targeted `cargo test -p <crate> --lib <filter>` + `cargo clippy ... -D warnings`; register tasks add `./scripts/check-migration-allowlist.sh` and `make api-surface`)
- **After every plan wave:** `cargo test --workspace`
- **Before `/gsd-verify-work`:** 42-12 Task 3's full gate (`cargo test --workspace`, `make clean-code`, `make check-gates`, `make security`, `make api-surface`, the v0.9 golden and the OpenAPI baseline) green; CI `coverage` (>= 82 %) and `postgres-integration` (no `SKIP:`) green
- **Max feedback latency:** ~3 minutes for a targeted filter (Rust incremental compile bound)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 42-01-01 | 01 | 1 | ALLOW-03, ALLOW-05, PLAT-09 | T-42-02 | design doors confirmed before any one-way change | checkpoint | n/a (operator decision recorded in 42-01-SUMMARY.md) | n/a | ⬜ pending |
| 42-01-02 | 01 | 1 | ALLOW-03, ALLOW-05, PLAT-09 | T-42-01 | overshoot stated as bounded | doc gate | `test -f .planning/decisions/0057-mid-run-halt-contract.md && grep -q 'Next free ADR number: 0058' .planning/decisions/PROMOTION.md` | ❌ W0 (ADR created) | ⬜ pending |
| 42-02-01 | 02 | 2 | ALLOW-03 | T-42-04 | engine halts at the boundary via the port only | unit (engine) | `cargo test -p paladin-battalion --lib spend_guard` | ❌ W0 (`spend_guard_tests`) | ⬜ pending |
| 42-02-02 | 02 | 2 | ALLOW-03 | T-42-04, T-42-05 | spending run halts; no key value in body | e2e (tracer) | `cargo test -p paladin-ai --lib engine_spend_halt_tracer` | ❌ W0 | ⬜ pending |
| 42-02-03 | 02 | 2 | ALLOW-03 | -- | registers match the surface | gate | `./scripts/check-migration-allowlist.sh && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | ✅ | ⬜ pending |
| 42-03-01 | 03 | 3 | ALLOW-03 | T-42-10, T-42-11 | bound SQL; corrupt value is a typed error | contract | `cargo test -p paladin-storage --features sqlite --lib run::` | ✅ (extend) | ⬜ pending |
| 42-03-02 | 03 | 3 | ALLOW-03, PLAT-09 | T-42-09, T-42-12, T-42-13 | reason before status; signature over stored bytes | unit + e2e | `cargo test -p paladin-ai --lib application::services::run::worker && cargo test -p paladin-ai --lib engine_spend_halt_tracer` | ✅ (extend) | ⬜ pending |
| 42-03-03 | 03 | 3 | ALLOW-03 | -- | registers | gate | `./scripts/check-migration-allowlist.sh && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | ✅ | ⬜ pending |
| 42-04-01 | 04 | 4 | ALLOW-03 | T-42-15, T-42-16 | fail closed; sticky child halt | unit | `cargo test -p paladin-ai --lib application::services::treasurer && cargo test -p paladin-battalion --lib child_battalion_halt_on_spend_halts_the_parent` | ✅ (extend) | ⬜ pending |
| 42-04-02 | 04 | 4 | ALLOW-03 | T-42-14 | fork re-admits (429 while exhausted) | e2e | `cargo test -p paladin-ai --lib halted_run_resumes_by_fork_after_window_reset` | ❌ W0 | ⬜ pending |
| 42-05-01 | 05 | 5 | PLAT-09 | T-42-21 | legacy traces still read | unit + contract | `cargo test -p paladin-ai-core --lib trace && cargo test -p paladin-storage --features sqlite --lib run_trace::` | ✅ (extend) | ⬜ pending |
| 42-05-02 | 05 | 5 | PLAT-09 | T-42-19, T-42-20 | live = degraded = replay | stream | `cargo test -p paladin-ai --lib application::services::run::stream_tests` | ✅ (extend) | ⬜ pending |
| 42-05-03 | 05 | 5 | PLAT-09 | -- | registers | gate | `./scripts/check-migration-allowlist.sh && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | ✅ | ⬜ pending |
| 42-06-01 | 06 | 6 | PLAT-09 | T-42-24 | caller cancel reported cancelled | unit | `cargo test -p paladin-ai --lib application::services::run::cancel && cargo test -p paladin-ai --lib application::services::run::events` | ✅ (extend) | ⬜ pending |
| 42-06-02 | 06 | 6 | PLAT-09 | T-42-23 | drain emits no done | stream + e2e | `cargo test -p paladin-ai --lib application::services::run::stream_tests && cargo test -p paladin-ai --lib application::services::run::cancel_tests` | ✅ (extend) | ⬜ pending |
| 42-07-01 | 07 | 7 | ALLOW-05 | T-42-29 | integer derivation; unpriced and zero budgets refused at admission | unit + doc | `cargo test -p paladin-ai --lib application::services::treasurer && cargo test -p paladin-ai --doc treasurer` | ✅ (extend) | ⬜ pending |
| 42-07-02 | 07 | 7 | ALLOW-05, ALLOW-03 | T-42-27, T-42-48 | one cutoff, tightest wins; call limits and Commissary untouched | unit + composition | `cargo test -p paladin-ai --lib application::services::paladin::middleware && cargo test -p paladin-ai --lib derived_budget_composes_with_the_commissary_rationed_rag_context && cargo test -p paladin-ai --test rag_commissary` | ✅ (extend) | ⬜ pending |
| 42-07-03 | 07 | 7 | ALLOW-05 | -- | registers | gate | `./scripts/check-migration-allowlist.sh && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | ✅ | ⬜ pending |
| 42-08-01 | 08 | 8 | ALLOW-05 | T-42-30 | per-agent cutoff installed; inert without either figure | unit | `cargo test -p paladin-ai --lib infrastructure::web::agent_host && cargo test -p paladin-ai --lib run_api_wiring` | ✅ (extend) | ⬜ pending |
| 42-08-02 | 08 | 8 | ALLOW-05, ALLOW-03 | T-42-28, T-42-31 | HTTP halt + 422; buffered stream done carries halt_reason; golden gate | e2e + golden | `cargo test -p paladin-ai --lib agent_execute_halts_on_the_derived_budget && cargo test -p paladin-web --lib execute_stream_buffered_fallback_done_carries_the_halt_reason && cargo test -p paladin-web --test openapi_golden_v0_9` | ❌ W0 | ⬜ pending |
| 42-08-03 | 08 | 8 | ALLOW-05 | -- | registers and docs | gate | `./scripts/check-migration-allowlist.sh && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | ✅ | ⬜ pending |
| 42-09-01 | 09 | 9 | ALLOW-03, ALLOW-05 | T-42-33, T-42-34 | engine nodes never capped | unit | `cargo test -p paladin-ai --lib application::services::run::worker && cargo test -p paladin-ai --lib infrastructure::web::facade_provisioner` | ✅ (extend) | ⬜ pending |
| 42-09-02 | 09 | 9 | ALLOW-03, ALLOW-05 | T-42-35 | agent-kind refusal before any row | e2e | `cargo test -p paladin-ai --lib agent_kind_run_halts_on_the_derived_budget` | ❌ W0 | ⬜ pending |
| 42-10-01 | 10 | 10 | ALLOW-03 | T-42-42 | store-enforced halt-notice dedup | contract | `cargo test -p paladin-storage --features sqlite --lib treasury::` | ✅ (extend) | ⬜ pending |
| 42-10-02 | 10 | 10 | ALLOW-03 | T-42-40, T-42-41 | operator secret signs allowance_halted | unit + contract | `cargo test -p paladin-ai --lib application::services::run::webhook && cargo test -p paladin-storage --features sqlite --lib webhook::` | ✅ (extend) | ⬜ pending |
| 42-10-03 | 10 | 10 | ALLOW-03 | -- | registers | gate | `./scripts/check-migration-allowlist.sh && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | ✅ | ⬜ pending |
| 42-11-01 | 11 | 11 | ALLOW-03, ALLOW-05 | T-42-38, T-42-39 | one claim per ceiling and window per run; spend_guard signature unchanged | unit | `cargo test -p paladin-ai --lib application::services::treasurer` | ✅ (extend) | ⬜ pending |
| 42-11-02 | 11 | 11 | ALLOW-03, ALLOW-05 | T-42-38 | one operator notice per window, end to end | e2e | `cargo test -p paladin-ai --lib mid_run_warn_and_halt_notices_reach_the_operator_once` | ❌ W0 | ⬜ pending |
| 42-11-03 | 11 | 11 | ALLOW-03 | T-42-43 | rollout caveat documented; no public drift | gate | `./scripts/check-migration-allowlist.sh && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | ✅ | ⬜ pending |
| 42-12-01 | 12 | 12 | PLAT-09, ALLOW-03 | T-42-44 | herald line names no identity | unit | `cargo test -p paladin-herald --lib && cargo test -p paladin-ai --lib infrastructure::telemetry::herald_sink` | ✅ (extend) | ⬜ pending |
| 42-12-02 | 12 | 12 | ALLOW-05 | T-42-46 | guard proves it can fail | integration | `cargo test -p paladin-ai --test treasurer_vocabulary_guard` | ❌ W0 | ⬜ pending |
| 42-12-03 | 12 | 12 | ALLOW-03, ALLOW-05, PLAT-09 | T-42-45, T-42-47 | full gate | gate | `cargo test --workspace && make check-gates && make security && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

No separate Wave 0 plan: every new test file or module below is created test-first by the task that owns it (marked `❌ W0` in the map), and existing infrastructure (cargo test, the shared contract suites, `http_surface_tests.rs`, `stream_tests.rs`) covers the rest.

- [ ] `crates/paladin-battalion/src/engine/mod.rs` `mod spend_guard_tests` -- 42-02 Task 1 (ALLOW-03)
- [ ] `src/application/services/run/http_surface_tests.rs` `engine_spend_halt_tracer` -- 42-02 Task 2 (ALLOW-03)
- [ ] `crates/paladin-storage/src/run/contract_tests.rs` halt_reason clauses -- 42-03 Task 1 (ALLOW-03)
- [ ] `crates/paladin-storage/src/treasury/notice_contract_tests.rs` halt-rung clauses -- 42-10 Task 1 (ALLOW-03)
- [ ] `tests/treasurer_vocabulary_guard.rs` -- 42-12 Task 2 (ALLOW-05)
- [ ] No framework install needed

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Design gate outcome | ALLOW-03, ALLOW-05, PLAT-09 | operator decision on one-way doors (42-01 Task 1) | record the selected option verbatim in 42-01-SUMMARY.md |
| Credential-handling review | ALLOW-03, PLAT-09 | security.instructions.md makes the manual review the primary control (no merge-gating Rust SAST) | review every Phase 42 log line, error body, trace event, webhook payload and herald line for key values; record in 42-12-SUMMARY.md |
| PostgreSQL contract legs and the 82 % coverage floor | ALLOW-03, PLAT-09 | no local PostgreSQL server or cargo-llvm-cov in the devcontainer | confirm CI `postgres-integration` (no `SKIP:`) and `coverage` jobs are green before `/gsd-verify-work` |
| Over-admission race bound | ALLOW-03, ALLOW-05 | concurrency backstop truths (42-04, 42-07) cannot be reproduced deterministically | reviewer confirms each run's guard halts at its first boundary after exhaustion (42-04 `two_guards_on_one_scope_both_halt_once_the_shared_balance_crosses`) |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 3 min per targeted filter (the Sampling Rate figure; Rust incremental compile bound)
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** {pending / approved YYYY-MM-DD}
