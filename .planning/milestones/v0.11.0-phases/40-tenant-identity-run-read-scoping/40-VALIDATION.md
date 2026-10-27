---
phase: 40
slug: tenant-identity-run-read-scoping
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-09-28
validated: 2026-10-27
---

# Phase 40 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (unit `#[cfg(test)]`, doctests, `#[tokio::test]`; storage contract suites on in-memory / SQLite / PostgreSQL; `http_surface_tests.rs` router tracers) |
| **Config file** | none — workspace `Cargo.toml` test targets |
| **Quick run command** | the task's own filter, e.g. `cargo test -p paladin-ai-core --lib principal`, `cargo test -p paladin-storage --features sqlite --lib run::sqlite`, `cargo test -p paladin-web --lib run_controller` |
| **Full suite command** | `cargo test --workspace && cargo fmt --check && make clean-code && make security && make check-gates && make openapi && git diff --exit-code crates/paladin-web/openapi.json` (plan 40-06 Task 3) |
| **Estimated runtime** | ~40 seconds |

---

## Sampling Rate

- **After every task commit:** Run the task's `<automated>` filter plus the touched crate's `cargo clippy -- -D warnings`
- **After every plan wave:** `cargo test --workspace`; register tasks add `./scripts/check-migration-allowlist.sh` and `make api-surface`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 40 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 40-01-01 | 01 | 1 | TENANT-01, TENANT-02, PLAT-07 | — | tenant/key never echoed; cross-tenant reads are 404; fail closed on malformed auth config | checkpoint — Task 1: Confirm the three one-way doors -- Principal shape (D-01), Princ | n/a (checkpoint:decision) | n/a | ✅ manual (three one-way doors confirmed by the operator; recorded in 40-01-SUMMARY.md) |
| 40-01-02 | 01 | 1 | TENANT-01, TENANT-02, PLAT-07 | — | tenant/key never echoed; cross-tenant reads are 404; fail closed on malformed auth config | tracer — Task 2: Tracer -- an API key's tenant is attributed on a SQLite run row  | `cargo test -p paladin-ai-core --lib principal && cargo test -p paladin-ai-core --lib container::run::tests && cargo test -p paladin-ai-core  …` | ✅ | ✅ green (re-run 2026-10-27) |
| 40-01-03 | 01 | 1 | TENANT-01, TENANT-02, PLAT-07 | — | tenant/key never echoed; cross-tenant reads are 404; fail closed on malformed auth config | unit/gate — Task 3: Register the tracer's breaks -- MIGRATION.md 9.2/9.4/9.5 rows an | `./scripts/check-migration-allowlist.sh && ! grep -q 'TBD' MIGRATION.md && grep -c "register-only" .cargo/semver-checks-allowlist.toml` | ✅ | ✅ green (re-run 2026-10-27) |
| 40-02-01 | 02 | 2 | TENANT-02, PLAT-07 | — | tenant/key never echoed; cross-tenant reads are 404; fail closed on malformed auth config | unit/gate — Task 1: Scoped GET /v1/runs on SQLite and in-memory -- RunQuery.scope in | `cargo test -p paladin-ports --lib run_repository_port && cargo test -p paladin-ports --doc run_repository_port && cargo test -p paladin-stor …` | ✅ | ✅ green (re-run 2026-10-27) |
| 40-02-02 | 02 | 2 | TENANT-02, PLAT-07 | — | tenant/key never echoed; cross-tenant reads are 404; fail closed on malformed auth config | unit/gate — Task 2: PostgreSQL parity -- attribution columns on every INSERT/SELECT, | `cargo test -p paladin-storage --features postgres --lib run::postgres && cargo clippy -p paladin-storage --features postgres,sqlite --all-ta …` | ✅ | ✅ green (34 passed; the live-DB clauses self-skip without a server, CI `postgres-integration` is the twin, as the command's own comment states) |
| 40-03-01 | 03 | 2 | TENANT-01 | — | tenant/key never echoed; cross-tenant reads are 404; fail closed on malformed auth config | unit/gate — Task 1: AuthConfig::validate() fails closed on every malformed key, tena | `cargo test -p paladin-ai --lib config::agents && cargo test -p paladin-ai --features web-server --bin paladin-server build_auth_config && ca …` | ✅ | ✅ green (re-run 2026-10-27) |
| 40-03-02 | 03 | 2 | TENANT-01 | — | tenant/key never echoed; cross-tenant reads are 404; fail closed on malformed auth config | unit/gate — Task 2: Document the key-to-tenant mapping and tenant-scoped run reads f | `grep -q 'tenant: "platform-ops"' docs/src/deployment-topologies/http-service-host.md && grep -q "open-access" docs/src/deployment-topologies …` | ✅ | ✅ green for the two docs greps; the closing `! grep -rqE '^\s*api_keys\s*:' src/application/cli` is a regex false positive on the Rust parameter `api_keys: &HashMap<K, String>` in `src/application/cli/templates/env.rs` (present since 2026-02-08); a grep over config content under the CLI finds nothing, so the invariant holds — map correction |
| 40-04-01 | 04 | 2 | TENANT-02 | — | tenant/key never echoed; cross-tenant reads are 404; fail closed on malformed auth config | unit/gate — Task 1: Worker path -- runs settle under their recorded submitter (engin | `cargo test -p paladin-ai-core --lib treasury_ledger && cargo test -p paladin-ai-core --lib run_scope && cargo test -p paladin-ai-core --doc  …` | ✅ | ✅ green (re-run 2026-10-27) |
| 40-04-02 | 04 | 2 | TENANT-02 | — | tenant/key never echoed; cross-tenant reads are 404; fail closed on malformed auth config | unit/gate — Task 2: HTTP agent-execute path -- defaulted scoped executor-port method | `cargo test -p paladin-ports --doc executor_port && cargo test -p paladin-web --lib agent_ && cargo test -p paladin-ai --lib paladin_executio …` | ✅ | ✅ green for every test (94 passed incl. `agent_auth`, `agent_controller`) and clippy; the trailing `git diff --quiet 629ef660 -- <Phase 39 ledger files>` is state-at-the-time (that base SHA is not in this history and Phase 41 later added `balance` to the ledger port by design) |
| 40-05-01 | 05 | 3 | PLAT-07, TENANT-02 | — | tenant/key never echoed; cross-tenant reads are 404; fail closed on malformed auth config | unit/gate — Task 1: Stream, cancel and webhook-deliveries enter through load_visible | `cargo test -p paladin-web --lib run_controller && cargo test -p paladin-ai --lib cross_tenant_cancel_is_a_404_and_writes_no_cancel_flag && c …` | ✅ | ✅ green (re-run 2026-10-27) |
| 40-05-02 | 05 | 3 | PLAT-07, TENANT-02 | — | tenant/key never echoed; cross-tenant reads are 404; fail closed on malformed auth config | unit/gate — Task 2: RunResponse.submitted_by, regenerated openapi.json, and the publ | `cargo test -p paladin-web --lib run_response && cargo test -p paladin-web --lib openapi_matches_committed_baseline && git diff --quiet 629ef …` | ✅ | ✅ green for `run_response`, `openapi_matches_committed_baseline`, the allowlist and `submitted_by`; the `git diff --quiet 629ef660 -- openapi_golden_v0_9.rs` clause is state-at-the-time (Phase 41 added the third golden exception) |
| 40-06-01 | 06 | 4 | TENANT-01, TENANT-02, PLAT-07 | — | tenant/key never echoed; cross-tenant reads are 404; fail closed on malformed auth config | unit/gate — Task 1: Measure every Phase 40 API change with cargo-semver-checks and r | `./scripts/check-migration-allowlist.sh && ! grep -q 'TBD' MIGRATION.md && for p in paladin-ai paladin-ai-core paladin-ports paladin-battalio …` | ✅ | ✅ CI-attributed — 10 of 11 baselined crates measure `0 checks: 0 pass, 254 skip` against `0.9.0` (the known D-09 no-op); the `paladin-web` 0.9.0 baseline cannot build here because `utoipa-swagger-ui`'s build script downloads swagger-ui from GitHub (sandbox egress), exactly as Phase 47 re-seal row 4 recorded; CI `Semver Checks (vs v0.9.0)` is `success` on the release heads |
| 40-06-02 | 06 | 4 | TENANT-01, TENANT-02, PLAT-07 | — | tenant/key never echoed; cross-tenant reads are 404; fail closed on malformed auth config | unit/gate — Task 2: CHANGELOG entries, the facade re-export of the principal module, | `grep -q "pub use paladin_core::platform::container::principal;" src/core/platform/mod.rs && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api …` | ✅ | ✅ green for the facade re-export, `make api-surface` ("unchanged"), the CHANGELOG TENANT-01/PLAT-07 greps and `make check-changelogs`; the `grep RunReadScope .project/current-exports.txt` clause was unsatisfiable by tool design at execution too (cargo-public-api lists the re-exported `principal` module as one line; WINDOWS.md row 59, since waived) |
| 40-06-03 | 06 | 4 | TENANT-01, TENANT-02, PLAT-07 | — | tenant/key never echoed; cross-tenant reads are 404; fail closed on malformed auth config | unit/gate — Task 3: ADR-0054 with PROMOTION.md and PROJECT.md, WINDOWS.md row 32 fix | `test -f .planning/decisions/0054-tenant-scoped-run-reads.md && grep -q "Next free ADR number: 0055" .planning/decisions/PROMOTION.md && grep …` | ✅ | ✅ green for ADR-0054, its PROJECT.md citation and the full gate (`cargo test --workspace` 7,719 passed / 2 row-73 failures, `cargo fmt --check`, `make clean-code`, `make security`, `make check-gates`, `make openapi` with a clean `openapi.json` diff — all re-run 2026-10-27); `Next free ADR number: 0055` is state-at-the-time (now 0063) and the WINDOWS row 32 `fixed` expectation is the documented execution deviation #3 (row already `waived`; closing condition recorded in ADR-0054 and the CHANGELOG) |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] Existing infrastructure covers all phase requirements: the `principal` module tests, the scoped `RunQuery` contract clauses, `AuthConfig::validate()` tests and the `cross_tenant_cancel_is_a_404_and_writes_no_cancel_flag` tracer were written test-first inside their tasks; no framework install.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Three one-way doors (Principal shape D-01, PrincipalRef, attribution columns) | TENANT-01, TENANT-02, PLAT-07 | operator `checkpoint:decision` 40-01-01 | recorded in 40-01-SUMMARY.md |
| PostgreSQL contract legs | TENANT-02, PLAT-07 | no PostgreSQL server in the sandbox (clauses self-skip) | CI `postgres-integration` (fails on any `SKIP:`) |
| `paladin-web` semver measurement against 0.9.0 | all | the 0.9.0 baseline's `utoipa-swagger-ui` build script needs GitHub egress | CI `Semver Checks (vs v0.9.0)` job (known D-09 no-op) |
| ~20 judgment-tier prohibitions | all | LLM-judge verdicts are non-authoritative (ADR-550 D4) | human review recommended; listed in 40-VERIFICATION.md and the milestone audit |
| 82 % coverage floor | all | `cargo-llvm-cov` absent locally | CI `Coverage` job (92.74 % on the v0.11.0 merge commit) |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (13 automated; the one without is the operator checkpoint 40-01-01)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 40s for a built crate (measured 2026-10-27 on a warm cache: 0-22 s once built; first compiles 45-82 s; the semver-checks loop is a wave-level gate at ~7 min)
- [x] `nyquist_compliant: true` set in frontmatter — set by `/gsd-validate-phase 40` on 2026-10-27

**Approval:** approved 2026-10-27 (`/gsd-validate-phase 40`)

---

## Validation Audit 2026-10-27

| Metric | Count |
|--------|-------|
| Tasks in map | 14 (13 automated + 1 operator `checkpoint:decision`), populated from plans 40-01..40-06 |
| Wave 0 files | none required; all task-written tests found on the tree |
| Automated commands re-run literally | 13: **8 green end to end**; 4 green on every test with one trailing state-at-the-time or tool-design clause (40-04-02, 40-05-02, 40-06-02, 40-06-03); 1 CI-attributed (40-06-01, `paladin-web` 0.9.0 baseline needs egress); 1 map correction (40-03-02 regex) |
| Gaps found | 0 |
| Resolved | 0 (none needed) |
| Escalated | 0 |

**Re-run detail (2026-10-27, tree = `v0.11.0` + `.planning` docs):** `paladin-ai-core` (`principal` 25, `container::run::tests` 7, `treasury_ledger` 8, `run_scope` 10, doctests); `paladin-ports` (`run_submission_port`, `run_repository_port` 9 + doctests, `executor_port` doctests); `paladin-storage` (`run::sqlite`, `run::in_memory`, `run::postgres` 34 self-skipping live clauses); `paladin-ai` (`config::agents` 25, `paladin_execution_service` 111, `cross_tenant_cancel_is_a_404_and_writes_no_cancel_flag`; `--features web-server --bin paladin-server build_auth_config` 7); `paladin-web` (`agent_` 94, `run_controller` 56, `run_response` 4, `openapi_matches_committed_baseline` 1); clippy `-D warnings` per crate and workspace `--all-features`; `./scripts/check-migration-allowlist.sh`; `register-only` rows in the semver allowlist; docs greps (`tenant: "platform-ops"`, `open-access`); facade re-export `pub use paladin_core::platform::container::principal;`; `make api-surface` "unchanged"; CHANGELOG `TENANT-01` / `PLAT-07`; `make check-changelogs`; ADR-0054 present and cited in PROJECT.md; full gate (`cargo test --workspace` 7,719 passed / 2 failed on WINDOWS row 73, `cargo fmt --check`, `make clean-code`, `make security`, `make check-gates`, `make openapi` + clean `openapi.json` diff).

**Open items carried (not Nyquist gaps):** WINDOWS.md row 58 (`/v1/threads/*` not tenant-scoped, D-14 deferral) and 40-REVIEW WR-02..04 remain in the milestone audit's tech debt.
