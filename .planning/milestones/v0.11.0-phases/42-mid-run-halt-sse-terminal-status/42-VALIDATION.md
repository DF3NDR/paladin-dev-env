---
phase: 42
slug: mid-run-halt-sse-terminal-status
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-10-06
validated: 2026-10-27
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
| 42-01-01 | 01 | 1 | ALLOW-03, ALLOW-05, PLAT-09 | T-42-02 | design doors confirmed before any one-way change | checkpoint | n/a (operator decision recorded in 42-01-SUMMARY.md) | n/a | ✅ manual (option-b recorded under `## Checkpoint decision` in 42-01-SUMMARY.md) |
| 42-01-02 | 01 | 1 | ALLOW-03, ALLOW-05, PLAT-09 | T-42-01 | overshoot stated as bounded | doc gate | `test -f .planning/decisions/0057-mid-run-halt-contract.md && grep -q 'Next free ADR number: 0058' .planning/decisions/PROMOTION.md` | ✅ (`0057-mid-run-halt-contract.md`) | ✅ green (PROMOTION.md next-free number has since advanced to 0063) |
| 42-02-01 | 02 | 2 | ALLOW-03 | T-42-04 | engine halts at the boundary via the port only | unit (engine) | `cargo test -p paladin-battalion --lib spend_guard` | ✅ (`mod spend_guard_tests`) | ✅ green |
| 42-02-02 | 02 | 2 | ALLOW-03 | T-42-04, T-42-05 | spending run halts; no key value in body | e2e (tracer) | `cargo test -p paladin-ai --lib engine_spend_halt_tracer` | ✅ | ✅ green |
| 42-02-03 | 02 | 2 | ALLOW-03 | -- | registers match the surface | gate | `./scripts/check-migration-allowlist.sh && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | ✅ | ✅ green |
| 42-03-01 | 03 | 3 | ALLOW-03 | T-42-10, T-42-11 | bound SQL; corrupt value is a typed error | contract | `cargo test -p paladin-storage --features sqlite --lib run::` | ✅ (extend) | ✅ green |
| 42-03-02 | 03 | 3 | ALLOW-03, PLAT-09 | T-42-09, T-42-12, T-42-13 | reason before status; signature over stored bytes | unit + e2e | `cargo test -p paladin-ai --lib application::services::run::worker && cargo test -p paladin-ai --lib engine_spend_halt_tracer` | ✅ (extend) | ✅ green |
| 42-03-03 | 03 | 3 | ALLOW-03 | -- | registers | gate | `./scripts/check-migration-allowlist.sh && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | ✅ | ✅ green |
| 42-04-01 | 04 | 4 | ALLOW-03 | T-42-15, T-42-16 | fail closed; sticky child halt | unit | `cargo test -p paladin-ai --lib application::services::treasurer && cargo test -p paladin-battalion --lib child_battalion_halt_on_spend_halts_the_parent` | ✅ (extend) | ✅ green |
| 42-04-02 | 04 | 4 | ALLOW-03 | T-42-14 | fork re-admits (429 while exhausted) | e2e | `cargo test -p paladin-ai --lib halted_run_resumes_by_fork_after_window_reset` | ✅ | ✅ green |
| 42-05-01 | 05 | 5 | PLAT-09 | T-42-21 | legacy traces still read | unit + contract | `cargo test -p paladin-ai-core --lib trace && cargo test -p paladin-storage --features sqlite --lib run_trace::` | ✅ (extend) | ✅ green |
| 42-05-02 | 05 | 5 | PLAT-09 | T-42-19, T-42-20 | live = degraded = replay | stream | `cargo test -p paladin-ai --lib application::services::run::stream_tests` | ✅ (extend) | ✅ green |
| 42-05-03 | 05 | 5 | PLAT-09 | -- | registers | gate | `./scripts/check-migration-allowlist.sh && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | ✅ | ✅ green |
| 42-06-01 | 06 | 6 | PLAT-09 | T-42-24 | caller cancel reported cancelled | unit | `cargo test -p paladin-ai --lib application::services::run::cancel && cargo test -p paladin-ai --lib application::services::run::events` | ✅ (extend) | ✅ green |
| 42-06-02 | 06 | 6 | PLAT-09 | T-42-23 | drain emits no done | stream + e2e | `cargo test -p paladin-ai --lib application::services::run::stream_tests && cargo test -p paladin-ai --lib application::services::run::cancel_tests` | ✅ (extend) | ✅ green |
| 42-07-01 | 07 | 7 | ALLOW-05 | T-42-29 | integer derivation; unpriced and zero budgets refused at admission | unit + doc | `cargo test -p paladin-ai --lib application::services::treasurer && cargo test -p paladin-ai --doc treasurer` | ✅ (extend) | ✅ green |
| 42-07-02 | 07 | 7 | ALLOW-05, ALLOW-03 | T-42-27, T-42-48 | one cutoff, tightest wins; call limits and Commissary untouched | unit + composition | `cargo test -p paladin-ai --lib application::services::paladin::middleware && cargo test -p paladin-ai --lib derived_budget_composes_with_the_commissary_rationed_rag_context && cargo test -p paladin-ai --test rag_commissary` (`[[test]]` target at `tests/integration/rag_commissary_test.rs`) | ✅ (extend) | ✅ green |
| 42-07-03 | 07 | 7 | ALLOW-05 | -- | registers | gate | `./scripts/check-migration-allowlist.sh && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | ✅ | ✅ green |
| 42-08-01 | 08 | 8 | ALLOW-05 | T-42-30 | per-agent cutoff installed; inert without either figure | unit | `cargo test -p paladin-ai --features web-server --lib infrastructure::web::agent_host && cargo test -p paladin-ai --features web-server --lib run_api_wiring` (feature flag added at validation: both modules are `web-server`-gated) | ✅ (extend) | ✅ green |
| 42-08-02 | 08 | 8 | ALLOW-05, ALLOW-03 | T-42-28, T-42-31 | HTTP halt + 422; buffered stream done carries halt_reason; golden gate | e2e + golden | `cargo test -p paladin-ai --features web-server --lib agent_execute_halts_on_the_derived_budget && cargo test -p paladin-web --lib execute_stream_buffered_fallback_done_carries_the_halt_reason && cargo test -p paladin-web --test openapi_golden_v0_9` | ✅ | ✅ green |
| 42-08-03 | 08 | 8 | ALLOW-05 | -- | registers and docs | gate | `./scripts/check-migration-allowlist.sh && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | ✅ | ✅ green |
| 42-09-01 | 09 | 9 | ALLOW-03, ALLOW-05 | T-42-33, T-42-34 | engine nodes never capped | unit | `cargo test -p paladin-ai --lib application::services::run::worker && cargo test -p paladin-ai --features web-server --lib infrastructure::web::facade_provisioner` (feature flag added at validation) | ✅ (extend) | ✅ green |
| 42-09-02 | 09 | 9 | ALLOW-03, ALLOW-05 | T-42-35 | agent-kind refusal before any row | e2e | `cargo test -p paladin-ai --lib agent_kind_run_halts_on_the_derived_budget` | ✅ | ✅ green |
| 42-10-01 | 10 | 10 | ALLOW-03 | T-42-42 | store-enforced halt-notice dedup | contract | `cargo test -p paladin-storage --features sqlite --lib treasury::` | ✅ (extend) | ✅ green |
| 42-10-02 | 10 | 10 | ALLOW-03 | T-42-40, T-42-41 | operator secret signs allowance_halted | unit + contract | `cargo test -p paladin-ai --lib application::services::run::webhook && cargo test -p paladin-storage --features sqlite --lib webhook::` | ✅ (extend) | ✅ green |
| 42-10-03 | 10 | 10 | ALLOW-03 | -- | registers | gate | `./scripts/check-migration-allowlist.sh && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | ✅ | ✅ green |
| 42-11-01 | 11 | 11 | ALLOW-03, ALLOW-05 | T-42-38, T-42-39 | one claim per ceiling and window per run; spend_guard signature unchanged | unit | `cargo test -p paladin-ai --lib application::services::treasurer` | ✅ (extend) | ✅ green |
| 42-11-02 | 11 | 11 | ALLOW-03, ALLOW-05 | T-42-38 | one operator notice per window, end to end | e2e | `cargo test -p paladin-ai --lib mid_run_warn_and_halt_notices_reach_the_operator_once` | ✅ | ✅ green |
| 42-11-03 | 11 | 11 | ALLOW-03 | T-42-43 | rollout caveat documented; no public drift | gate | `./scripts/check-migration-allowlist.sh && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | ✅ | ✅ green |
| 42-12-01 | 12 | 12 | PLAT-09, ALLOW-03 | T-42-44 | herald line names no identity | unit | `cargo test -p paladin-herald --lib && cargo test -p paladin-ai --lib infrastructure::telemetry::herald_sink` | ✅ (extend) | ✅ green |
| 42-12-02 | 12 | 12 | ALLOW-05 | T-42-46 | guard proves it can fail | integration | `cargo test -p paladin-ai --test treasurer_vocabulary_guard` | ✅ | ✅ green |
| 42-12-03 | 12 | 12 | ALLOW-03, ALLOW-05, PLAT-09 | T-42-45, T-42-47 | full gate | gate | `cargo test --workspace && make check-gates && make security && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

No separate Wave 0 plan: every new test file or module below is created test-first by the task that owns it (marked `❌ W0` in the map), and existing infrastructure (cargo test, the shared contract suites, `http_surface_tests.rs`, `stream_tests.rs`) covers the rest.

- [x] `crates/paladin-battalion/src/engine/mod.rs` `mod spend_guard_tests` -- 42-02 Task 1 (ALLOW-03)
- [x] `src/application/services/run/http_surface_tests.rs` `engine_spend_halt_tracer` -- 42-02 Task 2 (ALLOW-03)
- [x] `crates/paladin-storage/src/run/contract_tests.rs` halt_reason clauses -- 42-03 Task 1 (ALLOW-03)
- [x] `crates/paladin-storage/src/treasury/notice_contract_tests.rs` halt-rung clauses -- 42-10 Task 1 (ALLOW-03)
- [x] `tests/treasurer_vocabulary_guard.rs` -- 42-12 Task 2 (ALLOW-05)
- [x] No framework install needed

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

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (the only task without one is the `checkpoint:decision` gate 42-01-01)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (every `W0` file and all 9 named red-first tests found by `fn` name on the final tree)
- [x] No watch-mode flags
- [x] Feedback latency < 3 min per targeted filter (measured 2026-10-27: 0-164 s per targeted filter on a warm cache; the one 164 s figure was the first `paladin-battalion` compile)
- [x] `nyquist_compliant: true` set in frontmatter — set by `/gsd-validate-phase 42` on 2026-10-27

**Approval:** approved 2026-10-27 (`/gsd-validate-phase 42`)

---

## Validation Audit 2026-10-27

| Metric | Count |
|--------|-------|
| Tasks in map | 32 (31 automated + 1 `checkpoint:decision` gate, 42-01-01, manual-only by design) |
| Wave 0 files | 5 of 5 present (`spend_guard_tests` module, `engine_spend_halt_tracer`, 17 `halt_reason` clauses in `run/contract_tests.rs`, 22 halt-rung references in `notice_contract_tests.rs`, `tests/treasurer_vocabulary_guard.rs`); all 9 named red-first tests found by `fn` name |
| Verification targets re-run on the final tree | 34 distinct targets + 3 gates + the full workspace gate: **all green**; the full gate's only red tests are the two pre-existing Phase 50 sandbox-deadline tests of WINDOWS.md row 73 |
| Gaps found | 0 |
| Resolved | 0 (none needed) |
| Escalated | 0 |
| Map corrections | 3 (below) |

**Re-run detail (warm cache, 2026-10-27, tree = `v0.11.0` + `.planning` docs):**

- `paladin-battalion` (`spend_guard` 7, `child_battalion_halt_on_spend_halts_the_parent`), `paladin-ai-core --lib trace` (17), `paladin-herald --lib` (60), `paladin-ai` (`engine_spend_halt_tracer`, `halted_run_resumes_by_fork_after_window_reset`, `derived_budget_composes_with_the_commissary_rationed_rag_context`, `agent_kind_run_halts_on_the_derived_budget`, `mid_run_warn_and_halt_notices_reach_the_operator_once`, `two_guards_on_one_scope_both_halt_once_the_shared_balance_crosses`; `application::services::treasurer` 83 + doctests 18; `run::worker` 71, `run::stream_tests` 24, `run::cancel` 13, `run::events` 18, `run::webhook` 37; `paladin::middleware` 111; `telemetry::herald_sink` 7; `--test rag_commissary` 3; `--test treasurer_vocabulary_guard` 3), with `--features web-server`: `agent_execute_halts_on_the_derived_budget` 1, `infrastructure::web::agent_host` 47, `infrastructure::web::facade_provisioner` 17; `paladin-web` (`execute_stream_buffered_fallback_done_carries_the_halt_reason`, `--test openapi_golden_v0_9` 9); `paladin-storage --features sqlite` (`run::` 74, `run_trace::` 29, `treasury::` 112, `webhook::` 25): **all green, 0 failed**.
- Gates: `./scripts/check-migration-allowlist.sh` OK; `make check-gates` OK; `make security` exit 0 (cargo-audit + cargo-deny; the Phase 47 tokio pin clears RUSTSEC-2026-0386); `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` → "API surface unchanged" (run earlier the same day on the identical tree, Phase 41 audit).
- **Full gate (42-12-03)** `cargo test --workspace --no-fail-fast` (`CARGO_INCREMENTAL=0`): 62 suites, **7,719 passed, 2 failed, 229 ignored**. The two failures are `build_run_api_persists_no_run_traces_by_default` and `build_run_api_persists_run_traces_when_trace_persist_is_set` (`run_api_wiring.rs:2369`, "the run never reached a terminal status") — WINDOWS.md row 73 (open, Phase 50, pre-existing sandbox-specific 10 s worker deadline, green in CI). Not Phase 42 tests; every Phase 42 behaviour listed above is green. The first attempt at this gate died with `No space left on device` while linking examples (the sandbox disk hit 100 %); `target/debug/incremental` (9.2 GB, regenerable) was removed and the run repeated — a sandbox event, not a code finding.
- PostgreSQL legs and the 82 % coverage floor: CI-only, as the Manual-Only table states; both green on the v0.11.0 merge commit (`47-CI-EVIDENCE.md`, Coverage 92.74 %).
- Manual-only items: design gate recorded (42-01-SUMMARY `option-b`, all 16 items approved, item 12 extended); credential-handling review recorded in 42-12-SUMMARY ("no credential leak found"); over-admission race bound covered by `two_guards_on_one_scope_both_halt_once_the_shared_balance_crosses` (green).

**Map corrections:** (1) rows 42-08-01, 42-08-02 and 42-09-01 named `paladin-ai` filters (`agent_host`, `run_api_wiring`, `facade_provisioner`, `agent_execute_halts_on_the_derived_budget`) without `--features web-server`; those modules are feature-gated, so the plan-time command matched 0 tests — the feature flag is now in the map and the tests pass under it. (2) `--test rag_commissary` is a `[[test]]` target at `tests/integration/rag_commissary_test.rs`, not a root `tests/rag_commissary.rs`. (3) The 42-01-02 gate's literal `Next free ADR number: 0058` was true at seal and has since advanced to 0063 (ADR-0057 is present).
