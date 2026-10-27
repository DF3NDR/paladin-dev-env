---
phase: 39
slug: spend-ledger
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-09-27
validated: 2026-09-28
---

# Phase 39 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Ported from `39-RESEARCH.md` §Validation Architecture and the eight PLAN.md files at plan time
> (2026-09-27). `status`/`nyquist_compliant` were set by `/gsd-validate-phase 39` on 2026-09-28
> (see the audit trail at the end of this document).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` — native Rust unit tests (`#[cfg(test)]`), doctests, `#[tokio::test]` (multi-thread flavor for the race clauses); no external test framework |
| **Config file** | none — workspace `Cargo.toml` defines test targets; the shared ledger contract suite is the plain module `crates/paladin-storage/src/treasury/contract_tests.rs`; coverage is `cargo llvm-cov --fail-under-lines 82` in CI's `coverage` job (ADR-0006) |
| **Quick run command** | per-crate: `cargo test -p paladin-storage --features sqlite --lib treasury::`, `cargo test -p paladin-battalion --lib engine`, `cargo test -p paladin-ai --lib paladin_execution_service`, `cargo test -p paladin-web --lib run_controller`, `cargo test -p paladin-ai --features cli --lib application::cli::commands::treasury` |
| **Full suite command** | `cargo test --workspace` (unit + doctests); Postgres leg: `STORAGE_POSTGRES_TEST_URL=… cargo test -p paladin-storage --features postgres --lib postgres -- --test-threads=1` (CI `postgres-integration` job discovers `treasury::postgres` by module path; it fails on any `SKIP:` line) |
| **Estimated runtime** | ~60–120 s per crate quick run (incremental); ~10–15 min cold full workspace `cargo test`, a few minutes warm |

---

## Sampling Rate

- **After every task commit:** Run the task's own `<automated>` command (the touched crate's targeted `cargo test … --lib <module>` plus `cargo clippy -p <crate> --all-targets -- -D warnings`)
- **After every plan wave:** Run `cargo test --workspace` (Wave 1–2: at minimum `cargo test -p paladin-storage --features sqlite --lib treasury::`; Wave 3+: the full workspace)
- **Before `/gsd-verify-work`:** `cargo fmt --check`, `cargo test --workspace`, `make clean-code`, `make api-surface` (refreshed with `make api-surface-update` + CHANGELOG), `make check-gates`, `make security`, `make openapi` diff clean — sealed by plan 39-08
- **Max feedback latency:** ~120 s (single-crate incremental test + clippy)

---

## Per-Task Verification Map

Requirement IDs are from `REQUIREMENTS.md` (LEDGR-01..04). Threat refs are the plans' STRIDE rows
(`T-39-NN`). Commands are abbreviated — the authoritative command is each task's `<automated>` block.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 39-01-01 | 01 | 1 | LEDGR-01 (checkpoint:decision — operator confirms the one-way `007` schema, D-01/D-06) | — | N/A (human gate) | manual gate | — | — | ✅ resolved (option-a, recorded in 39-01-SUMMARY) |
| 39-01-02 | 01 | 1 | LEDGR-01, LEDGR-04 (tracer: settle → SQLite `007` → `paladin-cli treasury spend`) | T-39-01, T-39-03, T-39-04, T-39-11 | Bound parameters only; partial unique index + `ON CONFLICT DO NOTHING`; DB URL redacted before use; one row per currency | tracer (unit + doctest + binary smoke) | `cargo test -p paladin-ai-core --lib treasury_ledger && cargo test -p paladin-ports --doc treasury_ledger && cargo test -p paladin-storage --features sqlite --lib treasury:: && cargo test -p paladin-ai --features cli --lib application::cli::commands::treasury && paladin-cli treasury spend --format json smoke && clippy` | ✅ | ✅ green |
| 39-02-01 | 02 | 2 | LEDGR-02 (first red: 16-way race → 15 Ok / 1 Refused), LEDGR-01 | T-39-01, T-39-02, T-39-05, T-39-07, T-39-11 | `BEGIN IMMEDIATE` serialization proven on an on-disk WAL file; append-only; currency refusal | contract (TDD) | `cargo test -p paladin-storage --features sqlite --lib treasury:: && cargo test -p paladin-ports --doc treasury_ledger && clippy` | ✅ | ✅ green |
| 39-02-02 | 02 | 2 | LEDGR-01, LEDGR-03 (idempotency, spend windows/grouping/ordering) | T-39-03, T-39-05 | Store-enforced duplicate-settle; concurrent duplicates charged once; in-memory mutex twin | contract (TDD) | `cargo test -p paladin-storage --lib treasury::in_memory && cargo test -p paladin-storage --features sqlite --lib treasury:: && clippy` | ✅ | ✅ green |
| 39-03-01 | 03 | 3 | LEDGR-01, LEDGR-02, LEDGR-03 (Postgres adapter, advisory lock) | T-39-01, T-39-02, T-39-03, T-39-04, T-39-14 | `pg_advisory_xact_lock(hashtext(scope))` before the SUM; `::BIGINT` SUM; redaction | unit + contract (TDD) | `cargo test -p paladin-storage --features postgres --lib treasury::postgres && clippy` | ✅ | ✅ green |
| 39-03-02 | 03 | 3 | LEDGR-01 (suite unmodified on Postgres; CLI Postgres arm) | T-39-15 | A SKIP run is never reported as a pass; CI fails on `SKIP:` | contract (Docker/local-cluster gated) | `cargo test -p paladin-storage --features postgres --lib postgres -- --list` (≥ 21 treasury tests) + live run with `STORAGE_POSTGRES_TEST_URL` and no `SKIP:` + `cargo build --features "cli storage-postgres" --bin paladin-cli` | ✅ (after 39-03-01) | ✅ green |
| 39-04-01 | 04 | 3 | LEDGR-04, LEDGR-03 (engine settle once per superstep attempt, model breakdown) | T-39-06, T-39-13, T-39-16, T-39-09, T-39-11 | Settle never fails the run; per-superstep accumulator (no whole-run double count); guard dropped before await | unit + engine (TDD) | `cargo test -p paladin-battalion --lib engine::settlement && cargo test -p paladin-battalion --lib engine && cargo check -p paladin-battalion && cargo test -p paladin-battalion --doc with_treasury_ledger && clippy` | ✅ | ✅ green |
| 39-04-02 | 04 | 3 | LEDGR-03 (retries/child runs roll into one settlement), LEDGR-04 | T-39-06, T-39-13 | Nested Battalion runs never settle on their own; failing ledger never fails a run | engine (TDD) | `cargo test -p paladin-battalion --lib engine && cargo check -p paladin-battalion && clippy` | ✅ | ✅ green |
| 39-05-01 | 05 | 3 | LEDGR-04, LEDGR-03 (agent-loop settle per priced call; RunScope.run_id) | T-39-06, T-39-12, T-39-13, T-39-09 | Settle never fails execution; PlatformRunsOnly never double-charges engine nodes | unit + doctest (TDD) | `cargo test -p paladin-ai-core --lib run_scope && cargo test -p paladin-ai --lib paladin_execution_service && cargo test -p paladin-ai --doc with_treasury_ledger && clippy` | ✅ (existing modules) | ✅ green |
| 39-05-02 | 05 | 3 | LEDGR-04 (writer installed on HTTP agents and the run engine's shared service) | T-39-13 | EngineExecutionPort forwards the scope; signatures unchanged | unit + build | `cargo test -p paladin-ai --features web-server --lib infrastructure::web::agent_host && … facade_provisioner && cargo build -p paladin-ai --features web-server --bin paladin-server && clippy` | ✅ (existing modules) | ✅ green |
| 39-06-01 | 06 | 3 | LEDGR-04 (ledger-derived cost on GET /runs/{id}, GET /runs) | T-39-06, T-39-08, T-39-11, T-39-17 | Ledger error → `cost: null`, never 500; one spend call per page; mixed currency → null | unit (TDD) | `cargo test -p paladin-web --lib run_controller && clippy` | ✅ (existing module) | ✅ green |
| 39-06-02 | 06 | 3 | LEDGR-04 (ExecuteResponse.cost; inverted Phase 38 test; OpenAPI) | T-39-08 | SSE payloads unchanged (`sse_payloads_carry_no_spend_field`) | unit + golden | `cargo test -p paladin-web --lib agent_controller && cargo test -p paladin-web --lib openapi_matches_committed_baseline && cargo test -p paladin-web && clippy` | ✅ (existing module) | ✅ green |
| 39-07-01 | 07 | 4 | LEDGR-03 (redelivery → bumped attempt), LEDGR-04 (e2e HTTP cost) | T-39-03, T-39-12, T-39-06 | Attempt from the persisted counter; run id from the repository row | unit + e2e (TDD) | `cargo test -p paladin-ai --features web-server --lib application::services::run && clippy` | ✅ (existing modules) | ✅ green |
| 39-07-02 | 07 | 4 | LEDGR-04 (ledger built from RunStoreConfig in build_run_api and paladin-server) | T-39-04, T-39-06 | Errors name env var/path, never the URL; disabled store installs nothing | unit + build | `cargo test -p paladin-ai --features web-server --lib infrastructure::web::run_api_wiring && cargo test -p paladin-ai --features web-server --bin paladin-server && cargo build … --features "web-server storage-postgres" && clippy` | ✅ (existing modules) | ✅ green |
| 39-08-01 | 08 | 5 | LEDGR-01..04 (semver-checks vs 0.9.0 and 0.10.1; MIGRATION §9.2 / allowlist set-equality) | T-39-18 | Every reported lint registered; set-equality + `make check-gates` | gate | `for pkg in <CI PACKAGES>; do cargo semver-checks check-release --package "$pkg" --default-features --baseline-version 0.9.0; done && ./scripts/check-migration-allowlist.sh && make check-gates` | ✅ | ✅ green |
| 39-08-02 | 08 | 5 | LEDGR-01..04 (phase gate: fmt, tests, clean-code, api-surface, CHANGELOG, openapi, herald/trace unchanged) | T-39-19, T-39-20, T-39-21, T-39-15 | CI-pinned nightly API baseline; `make security`; manual credential-handling review recorded | gate | `cargo fmt --check && cargo test --workspace && make clean-code && PUBLIC_API_TOOLCHAIN=… make api-surface && make check-gates && make openapi && git diff --exit-code crates/paladin-web/openapi.json && grep … CHANGELOG/current-exports` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Every "❌ W0" row above is a **new source file created by the task that references it**, in TDD
order (the test is written first inside the new file's `#[cfg(test)]` module or the shared contract
module). No separate Wave 0 plan is needed — the LEDGR-02 race clause is written first in 39-02
Task 1 (CONTEXT: "the phase's first red test").

- [x] `crates/paladin-core/src/platform/container/treasury_ledger.rs` — ledger value types + unit tests (D-05) — created by 39-01
- [x] `crates/paladin-ports/src/output/treasury_ledger_port.rs` — `TreasuryLedgerPort` + error enum + compiling rustdoc mock — created by 39-01
- [x] `crates/paladin-storage/src/treasury/{mod,sqlite}.rs` + `migrations/sqlite/007_create_treasury_ledger_table.sql` — created by 39-01
- [x] `src/application/cli/commands/treasury.rs` — `paladin-cli treasury spend` + tracer test — created by 39-01
- [x] `crates/paladin-storage/src/treasury/contract_tests.rs` — shared suite (race clause first) — created by 39-02
- [x] `crates/paladin-storage/src/treasury/in_memory.rs` — `InMemoryTreasuryLedger` + suite wiring — created by 39-02
- [x] `SqliteTreasuryLedger::new_shared_file` (test-only, WAL) for the on-disk race and concurrent-duplicate clauses — created by 39-02
- [x] `crates/paladin-storage/src/treasury/postgres.rs` + `migrations/postgres/007_create_treasury_ledger_table.sql` — created by 39-03
- [x] `crates/paladin-battalion/src/engine/settlement.rs` — superstep accumulator + boundary settle — created by 39-04
- [x] Inverted `execute_response_carries_no_cost_field` → `execute_response_carries_cost_when_priced` in `crates/paladin-web/src/agent_controller.rs` — 39-06
- [x] Shared fixtures: none new beyond the above — `RecordingPaladinPort` (paladin-battalion test_support), `ScriptedCostLlmPort` / `streamed_cost_tests` (execution service), `MockLlmAdapter` and `InMemoryTreasuryLedger` are reused; paladin-web tests use a local stub ledger (no storage dependency)
- [x] Framework install: none — `cargo test` is native; no new crate enters the dependency graph

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Operator confirms the one-way `007` schema (D-01 scope columns + `unattributed` sentinel, D-06 partial unique settlement index, D-02 breakdown column, the attribution-instant column in place of D-12's literal `window_start`) before it is written | LEDGR-01 | CONTEXT was gathered in `--auto` mode; a one-way door needs a human decision (`references/planner-reversibility.md`) | Plan 39-01 Task 1 is a blocking `checkpoint:decision`: review the proposed design and pick option-a/-b/-c; execution applies the choice |
| Manual credential-handling review of the Phase 39 diff | LEDGR-01..04 (security instructions) | No merge-gating Rust SAST exists; CodeQL is advisory-only (`security.instructions.md`) | Per `.github/instructions/security.instructions.md`: DB URLs redacted before any other handling in every new adapter; no log line interpolates a URL, password or key; `api_key_id` is an opaque label; no new HTTP client — recorded in 39-08's SUMMARY |
| Postgres contract suite against a live server when no local cluster can be started | LEDGR-01 | This devcontainer has no Docker; a local PostgreSQL 16 cluster may or may not start | 39-03 Task 2 tries the local cluster; otherwise CI's `postgres-integration` job (fails on any `SKIP:` line) is authoritative and the SUMMARY says so |
| Operator UAT: a real multi-model engine run shows a sane per-model split | LEDGR-04 | Needs real provider credentials and a priced `treasurer:` table (RESEARCH Pitfall 5 warning sign) | With `APP_RUN_STORE_BACKEND=sqlite`, run `paladin-server`, submit a workflow whose Paladin nodes use two models, then `paladin-cli treasury spend --group-by model` and `--group-by run` against the same file: one row per model, run total equal to the run's herald cost |

All other phase behaviors have automated verification.

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (the only task without one is the human `checkpoint:decision` gate 39-01-01)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (each new file is created by the task that first tests it)
- [x] No watch-mode flags
- [x] Feedback latency < 120 s for per-task quick commands
- [x] `nyquist_compliant: true` set in frontmatter — set by `/gsd-validate-phase 39` on 2026-09-28

**Approval:** validated 2026-09-28 (`/gsd-validate-phase 39`)

---

## Validation Audit 2026-09-28

| Metric | Count |
|--------|-------|
| Gaps found | 0 |
| Resolved | 0 |
| Escalated | 0 |

**Method.** State A audit (VALIDATION.md existed). Every requirement in the Per-Task Verification Map
was cross-referenced against the tree by test name: all 16 task rows resolve to existing tests
(`crates/paladin-storage/src/treasury/{contract_tests,in_memory,sqlite,postgres}.rs`,
`crates/paladin-battalion/src/engine/{settlement,mod}.rs`,
`src/application/services/paladin/paladin_execution_service.rs`,
`src/infrastructure/web/{agent_host,facade_provisioner,run_api_wiring}.rs`,
`crates/paladin-web/src/{run_controller,agent_controller,openapi}.rs`,
`src/application/services/run/{worker,tracer_e2e,events}.rs`,
`src/application/cli/commands/treasury.rs`,
`crates/paladin-core/src/platform/container/treasury_ledger.rs`). Every Wave 0 file exists.

**Green evidence.** Each row's status is `✅ green` on the strength of the live runs recorded in
`39-VERIFICATION.md` (2026-09-28T02:29Z, `gsd-verifier`) on this same tree — `git diff --stat
980caf94 HEAD` shows only `.planning/` changes since the 39-08 full-workspace `cargo test` — which
re-ran every quick-run command in the Test Infrastructure table: in-memory + SQLite contract suite
62/62, Postgres contract suite 23/23 live with 0 `SKIP:` lines, engine 561/561, execution service
71/71, worker 32/32, tracer_e2e 5/5, run_api_wiring 8/8, CLI 4/4, paladin-web 233/233 plus 7/7
golden, doctests 6+1, clippy and `cargo fmt --check` clean. This validate-phase session could not
execute shell commands itself (the sandbox's command classifier returned no verdict for every Bash
and subagent call); the statuses therefore rest on the verifier's same-day live run rather than a
fresh re-execution in this session. No test file was generated because no MISSING or PARTIAL
requirement was found; no `gsd-nyquist-auditor` spawn was needed.

**Manual-only rows, resolution state.**

| Behavior | State |
|----------|-------|
| One-way `007` schema checkpoint | Resolved — operator chose option-a (approve as proposed), recorded in `39-01-SUMMARY.md` |
| Manual credential-handling review | Done — recorded in `39-08-SUMMARY.md` §Credential-handling review (redact-first on both SQL adapters, no scope identity in settle logs, no new HTTP client) |
| Postgres contract suite against a live server | Done — live local Postgres 16 run at 39-03 and again by the verifier (23 passed, 0 `SKIP:`); CI `postgres-integration` remains authoritative for the Docker-gated path |
| Operator UAT: real multi-model engine run per-model split | Still manual — needs real provider credentials and a priced `treasurer:` table; not required for Nyquist compliance (the same split is covered under test by `treasury_ledger_settles_once_per_superstep_with_model_breakdown` and the CLI `--group-by model` tests) |
