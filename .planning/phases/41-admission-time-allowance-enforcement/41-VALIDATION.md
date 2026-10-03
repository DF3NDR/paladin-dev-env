---
phase: 41
slug: admission-time-allowance-enforcement
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-10-02
---

# Phase 41 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Filled at plan time (2026-10-03, revision iteration 1) from `41-RESEARCH.md` §Validation Architecture
> and the `<automated>` blocks of the nine PLAN.md files. Commands below are abbreviated -- the
> authoritative command for each row is that task's own `<automated>` block. `status` stays `draft`
> and `nyquist_compliant` stays `false` until `/gsd-validate-phase 41` audits the executed phase.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` -- native Rust unit tests (`#[cfg(test)]`), doctests, `#[tokio::test]` (multi-thread flavor for the race clauses); no external test framework |
| **Config file** | none -- workspace `Cargo.toml` defines the test targets; the shared contract suites are plain modules (`crates/paladin-storage/src/treasury/contract_tests.rs`, the new `treasury/notice_contract_tests.rs`, `run_schedule/contract_tests.rs`, `webhook/contract_tests.rs`); coverage is `cargo llvm-cov --fail-under-lines 82` in CI's `coverage` job (ADR-0006) |
| **Quick run command** | per crate, per task: `cargo test -p paladin-storage --features sqlite --lib treasury::`, `cargo test -p paladin-ai --lib application::services::treasurer`, `cargo test -p paladin-ai --lib application::services::run::submission`, `cargo test -p paladin-ai --lib allowance_admission_tracer`, `cargo test -p paladin-web --lib` (each task's `<automated>` block names its own set) |
| **Full suite command** | `cargo test --workspace` (unit + doctests); Postgres leg: `STORAGE_POSTGRES_TEST_URL=postgres://paladin:paladin@localhost:5433/paladin_waypoint_test cargo test -p paladin-storage --features postgres --lib postgres -- --test-threads=1` (CI `postgres-integration` is authoritative and fails on any `SKIP:` line; this devcontainer has no PostgreSQL server) |
| **Estimated runtime** | ~1-3 min per incremental single-crate quick run (targeted `cargo test` plus `cargo clippy -p <crate>`); a task's full `<automated>` chain that ends in workspace-wide clippy takes several minutes; ~10-15 min cold `cargo test --workspace`, a few minutes warm |

---

## Sampling Rate

- **After every task commit:** Run the task's own `<automated>` command (the touched crates' targeted `cargo test ... --lib <module>` plus clippy and `cargo fmt --check`; register tasks add `./scripts/check-migration-allowlist.sh` and `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface`)
- **After every plan wave:** Run `cargo test --workspace` (waves 1-2 at minimum `cargo test -p paladin-storage --features sqlite --lib treasury::` and `cargo test -p paladin-ai --lib allowance_admission_tracer`)
- **Before `/gsd-verify-work`:** `cargo fmt --check`, `cargo test --workspace`, `make clean-code`, `make api-surface` (refreshed per plan with `make api-surface-update` plus a CHANGELOG entry), `make check-gates` (incl. `check-migration-allowlist`), `make security`, `make openapi` with a clean `git diff --exit-code crates/paladin-web/openapi.json`, `cargo test -p paladin-web --test openapi_golden_v0_9`, CI `coverage` (>= 82%) and `postgres-integration` (no `SKIP:`) -- sealed by plan 41-09
- **Max feedback latency:** ~180 s for a per-task quick command (single-crate incremental test plus targeted clippy); minutes, not seconds -- a `cargo test`/`clippy` chain of this size cannot meet a sub-minute target

---

## Per-Task Verification Map

Requirement IDs are from `REQUIREMENTS.md` (ALLOW-01, ALLOW-02, ALLOW-04). Threat refs are the plans'
STRIDE rows (`T-41-NN`). Commands are abbreviated; `clippy`/`fmt` stand for the task's exact
`cargo clippy ... -- -D warnings` / `cargo fmt --check` invocations and `api-surface` for
`PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface`.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 41-01-01 | 01 | 1 | ALLOW-01, ALLOW-02, ALLOW-04 (checkpoint:decision -- consolidated design C1..C14, one-way `010`/`011`, D-17 payload option) | -- | N/A (human gate) | manual gate | -- | -- | ⬜ pending |
| 41-01-02 | 01 | 1 | ALLOW-01 (D-04 `balance`), ALLOW-02 (refusal shape, admission port) -- tracer half 1, intermediate green gate | T-41-03, T-41-04 | `balance` SQL from `&'static str` prefixes + `push_bind` only; a non-overriding adapter fails closed; the refusal names no tenant or key | unit + doc + contract (TDD; the phase's first red test) | `cargo test -p paladin-ai-core --lib allowance && ... --lib treasury_ledger && ... --doc allowance && cargo test -p paladin-ports --lib allowance_admission_port && ... --doc allowance_admission_port && cargo test -p paladin-storage --features sqlite --lib treasury:: && cargo check --workspace --all-targets --all-features && clippy && fmt` | ❌ W0 (new `allowance.rs`, `allowance_admission_port.rs`) | ⬜ pending |
| 41-01-03 | 01 | 1 | ALLOW-01, ALLOW-02 (tracer half 2: 429 `allowance_exhausted` + store-clock `Retry-After`, nothing persisted) | T-41-01, T-41-02, T-41-04, T-41-05, T-41-06 | Admission before any insert/enqueue; six-key body without tenant/key; no local clock in the treasurer module; backend error fails closed | tracer (unit + e2e) | `cargo test -p paladin-ai --lib config::treasurer && ... application::services::treasurer && ... application::services::run::submission && cargo test -p paladin-web --lib && cargo test -p paladin-storage --features sqlite --lib treasury:: && cargo test -p paladin-ai --lib allowance_admission_tracer && clippy && fmt` | ❌ W0 (new `treasurer/{mod,policy,window}.rs`) | ⬜ pending |
| 41-01-04 | 01 | 1 | ALLOW-01, ALLOW-02 (MIGRATION.md 9.2/9.6, CHANGELOG, API baseline) | -- | N/A (registers) | gate | `./scripts/check-migration-allowlist.sh && ! grep -q 'TBD' MIGRATION.md && grep -c allowance_exhausted MIGRATION.md CHANGELOG.md && api-surface` | ✅ | ⬜ pending |
| 41-02-01 | 02 | 2 | ALLOW-01 (Postgres `balance`, full balance contract on three adapters, exact-instant edges) | T-41-09, T-41-12 | No `IS NULL OR` predicate, binds only; connection password redacted before any error text | contract (TDD) | `cargo test -p paladin-storage --lib treasury::in_memory && ... --features sqlite --lib treasury:: && ... --features postgres --lib treasury::postgres && clippy -p paladin-storage` | ✅ (existing modules) | ⬜ pending |
| 41-02-02 | 02 | 2 | ALLOW-01, ALLOW-02 (boundaries, order, short-circuit, no-entry no-read, fail closed, idempotency, concurrency) | T-41-10, T-41-11, T-41-13 | Any ledger, currency or clock failure fails closed; no role input (D-09); the simultaneous-admission race is an accepted backstop (ADR-0056, Phase 42) | unit (scripted store clock, TDD) | `cargo test -p paladin-ai --lib application::services::treasurer && cargo test -p paladin-ai --lib allowance_admission_tracer && clippy && fmt` + `api-surface` (must be unchanged) | ❌ W0 (new `treasurer/tests.rs`) | ⬜ pending |
| 41-03-01 | 03 | 2 | ALLOW-01 (full grammar, strict keys, env overrides, redacted secret, D-11 cross-check) | T-41-14, T-41-15, T-41-16, T-41-17 | `deny_unknown_fields` on every allowance struct; secret absent from Debug/Serialize/errors; exact-integer amounts | unit (TDD) | `cargo test -p paladin-ai --lib config::treasurer && ... --lib config:: && ... --doc treasurer && clippy && fmt` | ✅ (existing module) | ⬜ pending |
| 41-03-02 | 03 | 2 | ALLOW-01 (D-11 boot coherence, Treasurer built in `build_run_api`, registers, API baseline) | T-41-14, T-41-SSRF | An incoherent configuration stops boot naming the path; enforcement never silently skipped | unit + router e2e | `cargo test -p paladin-ai --features web-server --lib infrastructure::web::run_api_wiring && cargo test -p paladin-web --lib agent_auth && ... --bin paladin-server && ./scripts/check-migration-allowlist.sh && clippy && fmt && api-surface` + `make security` | ✅ (existing modules) | ⬜ pending |
| 41-04-01 | 04 | 3 | ALLOW-02 (execute, execute/stream, jobs gated; production Treasurer on `AgentApiState`) | T-41-18, T-41-20, T-41-21, T-41-22 | Executor never invoked on refusal; no job id for refused work; generic 500 on a failed check | unit (TDD) | `cargo test -p paladin-web --lib agent_controller && cargo test -p paladin-ai --features web-server --bin paladin-server && clippy -p paladin-web` | ✅ (existing modules) | ⬜ pending |
| 41-04-02 | 04 | 3 | ALLOW-02 (fork gated, Admin bound, parallel refusals write nothing) | T-41-18, T-41-19 | Admin bound like any key; eight concurrent refusals persist nothing | unit + e2e (TDD) | `cargo test -p paladin-ai --lib application::services::run::submission && cargo test -p paladin-web --lib thread_controller && cargo test -p paladin-ai --lib parallel_submissions_by_an_exhausted_key_are_all_refused_and_write_nothing && ... allowance_admission_tracer && clippy` | ✅ (existing modules) | ⬜ pending |
| 41-04-03 | 04 | 3 | ALLOW-02 (OpenAPI 429 on five routes, third golden exception, registers) | -- | The v0.9 golden gate is narrowed by one named three-entry exception, never loosened | golden + gate | `cargo test -p paladin-web --lib openapi_matches_committed_baseline && cargo test -p paladin-web --test openapi_golden_v0_9 && cargo test -p paladin-web --lib && ./scripts/check-migration-allowlist.sh && clippy -p paladin-web` + `api-surface` (expected unchanged) | ✅ (existing modules) | ⬜ pending |
| 41-05-01 | 05 | 4 | ALLOW-02 (schedule creator persisted: `RunSchedule.created_by`, migration `010`, three adapters) | T-41-25 | Half-attributed rows rejected on read; PostgreSQL CHECK; SQLite `009` gap proven | contract (TDD) | `cargo test -p paladin-ai-core --lib run_schedule && cargo test -p paladin-storage --lib run_schedule::in_memory && ... --features sqlite --lib run_schedule:: && ... --features postgres --lib run_schedule::postgres && cargo check --workspace --all-targets --all-features && clippy` | ❌ W0 (new `010` migrations) | ⬜ pending |
| 41-05-02 | 05 | 4 | ALLOW-02 (creator stamped at `POST /v1/schedules`, fired runs attributed and admitted, role check unchanged) | T-41-23, T-41-24, T-41-26, T-41-27 | Creator never exposed by `ScheduleResponse`; the role check reads `requested_by` only | unit (TDD) | `cargo test -p paladin-ports --lib run_submission_port && ... --doc run_submission_port && ... --doc schedule_admin_port && cargo test -p paladin-ai --lib application::services::run && ... application::services::assistant && cargo test -p paladin-web --lib && clippy && fmt` | ✅ (existing modules) | ⬜ pending |
| 41-05-03 | 05 | 4 | ALLOW-02 (MIGRATION.md 9.2/9.4/9.6, allowlist, WINDOWS.md row, docs, API baseline) | T-41-27 | The legacy NULL-creator gap is an open, owned WINDOWS.md row | gate | `./scripts/check-migration-allowlist.sh && ! grep -q 'TBD' MIGRATION.md && node .claude/gsd-core/bin/gsd-tools.cjs windows status && api-surface` | ✅ | ⬜ pending |
| 41-06-01 | 06 | 5 | ALLOW-04 (once-per-window notices store, migration `011`, sixteen-way race on three adapters) | T-41-28, T-41-31 | Unique index + `ON CONFLICT DO NOTHING`; `''`/epoch sentinels; arbiter text matches the migration | contract (TDD) | `cargo test -p paladin-ai-core --lib allowance && cargo test -p paladin-ports --doc treasury_notice_port && cargo test -p paladin-storage --lib treasury::in_memory && ... --features sqlite --lib treasury:: && ... --features postgres --lib treasury::postgres && clippy -p paladin-storage` | ❌ W0 (new port, `011` migrations, `notice_contract_tests.rs`) | ⬜ pending |
| 41-06-02 | 06 | 5 | ALLOW-04 (crossings claimed on the admitted path, abandon discards, production attaches the store) | T-41-29, T-41-30, T-41-33 | A notice never blocks a run; an abandoned admission gives its notice back | unit (TDD) | `cargo test -p paladin-ai --lib application::services::treasurer && ... application::services::run::submission && ... --features web-server --lib infrastructure::web::run_api_wiring && ... allowance_admission_tracer && clippy && fmt` | ✅ (existing modules) | ⬜ pending |
| 41-06-03 | 06 | 5 | ALLOW-04 (registers, API baseline); ALLOW-01 support (additive `012` tenant-window index, Pitfall 14) | T-41-32 | `007` byte-untouched | contract re-run + gate | `cargo test -p paladin-storage ... treasury::` (in-memory, SQLite, PostgreSQL) `&& ./scripts/check-migration-allowlist.sh && ! grep -q 'TBD' MIGRATION.md && api-surface` | ❌ W0 (new `012` migrations) | ⬜ pending |
| 41-07-01 | 07 | 6 | ALLOW-04 (`TraceEvent::AllowanceWarning`, one shared herald line, three heralds, stateful herald sink) | T-41-34 | The event and line carry figures, window and threshold only -- no tenant, no key | unit (TDD) | `cargo test -p paladin-ai-core --lib trace && ... --lib allowance && ... --lib herald && ... --doc herald && cargo test -p paladin-herald --all-features && cargo test -p paladin-ai --lib infrastructure::telemetry::herald_sink && clippy` | ✅ (existing modules) | ⬜ pending |
| 41-07-02 | 07 | 6 | ALLOW-04 (worker first-dispatch emission, HTTP agent path via `RunScope`, wiring, registers, API baseline) | T-41-35, T-41-36, T-41-37 | `seq` never collides; a notice read failure never fails a run | unit (TDD) | `cargo test -p paladin-ai-core --lib run_scope && cargo test -p paladin-ai --lib application::services::run::worker_tests && ... application::services::run && ... paladin_execution_service && cargo test -p paladin-web --lib agent_controller && ... run_api_wiring && ./scripts/check-migration-allowlist.sh && clippy && fmt && api-surface` | ✅ (existing modules) | ⬜ pending |
| 41-08-01 | 08 | 7 | ALLOW-04 (operator event kind, pinned payload, operator signing branch) | T-41-39, T-41-40, T-41-41 | Signed with the operator secret before any run lookup; never caller-subscribable | unit + contract (TDD) | `cargo test -p paladin-ai-core --lib container::run && ... --doc run && cargo test -p paladin-storage --lib webhook::in_memory && ... --features sqlite --lib webhook:: && cargo test -p paladin-web --lib && cargo test -p paladin-ai --lib application::services::run::webhook && clippy` | ✅ (existing modules) | ⬜ pending |
| 41-08-02 | 08 | 7 | ALLOW-04 (enqueue on confirm, boot-time SSRF check, secret wiring, production builder delivers one signed notice) | T-41-38, T-41-39, T-41-42 | SSRF guard at boot and send time; no second HTTP client; secret never on the row | unit + builder e2e (TDD) | `cargo test -p paladin-ai --lib application::services::treasurer && ... --features web-server --lib infrastructure::web::run_api_wiring && ... allowance_admission_tracer && clippy && fmt` | ✅ (existing modules) | ⬜ pending |
| 41-08-03 | 08 | 7 | ALLOW-04 (integrated warn path: one notice row, one signed operator delivery, one trace event before `RunStarted`, one herald line, nothing on a second admission; registers, API baseline) | T-41-41, T-41-43 | The operator notice is never listed under the run; the run is never blocked | e2e (TDD) + gate | `cargo test -p paladin-ai --lib allowance_warn_path_tracer && ... allowance_admission_tracer && ./scripts/check-migration-allowlist.sh && ! grep -q 'TBD' MIGRATION.md && clippy && fmt && api-surface` + `make security` | ✅ (existing `http_surface_tests.rs`) | ⬜ pending |
| 41-09-01 | 09 | 8 | ALLOW-01, ALLOW-02, ALLOW-04 (ADR-0056, PROMOTION.md to 0057, PROJECT.md row) | -- | N/A (decision record) | doc gate | `test -f .planning/decisions/0056-allowance-admission-model.md && grep -q "Next free ADR number: 0057" .planning/decisions/PROMOTION.md && awk '/^## /' <ADR>` | ❌ W0 (new ADR) | ⬜ pending |
| 41-09-02 | 09 | 8 | ALLOW-01, ALLOW-02, ALLOW-04 (facade re-export, consolidated register row, crate CHANGELOGs, final API baseline, phase gate) | T-41-44, T-41-45 | Every public surface change registered, changelogged and baselined | gate | `grep -q "pub use paladin_core::platform::container::allowance;" src/core/platform/mod.rs && ./scripts/check-migration-allowlist.sh && make check-changelogs && api-surface && cargo test --workspace && fmt && make clean-code && make check-gates` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Wave 0 is **covered by the RED-first steps of 41-01 and 41-02** and by the task that first tests each
new file: every "❌ W0" row above is a new source file created by the task that references it, in TDD
order. No separate Wave 0 plan is needed. The RESEARCH "stubs for the five existing test doubles" gap
does not arise: `TreasuryLedgerPort::balance` is a defaulted method (41-01 Task 1 item 2, C11), so no
existing implementor changes.

- [ ] `crates/paladin-core/src/platform/container/allowance.rs` + unit tests -- created by 41-01 Task 2
- [ ] `crates/paladin-ports/src/input/allowance_admission_port.rs` (compiling rustdoc mock) -- created by 41-01 Task 2
- [ ] `tenant_balance_equals_sum_of_key_balances` in the existing ledger contract suite (the phase's first red test) -- 41-01 Task 2
- [ ] `src/application/services/treasurer/{mod,policy,window}.rs` + in-file tests -- created by 41-01 Task 3
- [ ] `allowance_admission_tracer` in the existing `http_surface_tests.rs`, run red against a Treasurer-less router before green -- 41-01 Task 3
- [ ] `src/application/services/treasurer/tests.rs` (`FakeLedger` scripted store clock) -- created by 41-02 Task 2
- [ ] `yaml_env_placeholder_is_not_expanded` (pins the literal `${VAR}` behaviour) -- 41-03 Task 1
- [ ] `010_add_run_schedule_created_by.sql` on both backends + `migration_010_adds_nullable_creator_columns` (proves the SQLite `009` gap, A1) -- 41-05 Task 1
- [ ] `crates/paladin-ports/src/output/treasury_notice_port.rs`, `011_create_treasury_notices.sql` (both), `crates/paladin-storage/src/treasury/notice_contract_tests.rs` -- 41-06 Task 1; `012_add_treasury_ledger_tenant_index.sql` (both) -- 41-06 Task 3
- [ ] `trace.rs` variant-count test 12 -> 13 and its exhaustive match -- 41-07 Task 1
- [ ] `build_run_api_wires_the_allowance_warn_path` and `allowance_warn_path_tracer` -- 41-08 Tasks 2 and 3
- [ ] Framework install: none -- `cargo test` is native. Gate tooling absent from this devcontainer on 2026-10-03 and installed on first use: the pinned `nightly-2026-09-20` toolchain and `cargo-public-api` (41-01 Task 4), `cargo-audit` and `cargo-deny` (41-03 Task 2)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Operator confirms the consolidated design (C1..C14; one-way `010` and `011`; the D-17 payload option) before any of it is written | ALLOW-01, ALLOW-02, ALLOW-04 | One-way and costly doors need a human decision (`references/planner-reversibility.md`) | Plan 41-01 Task 1 is a blocking `checkpoint:decision`; the selection (option-a/-b/-c) is recorded verbatim in 41-01-SUMMARY.md before Task 2 writes any file |
| Manual credential-handling review of the phase diff | ALLOW-02, ALLOW-04 | No merge-gating Rust SAST exists; CodeQL is advisory-only (`security.instructions.md`) | Per `.github/instructions/security.instructions.md`: the webhook secret is absent from Debug, Serialize, error text and delivery rows; SSRF guard at boot and send time; only the no-redirect webhook client delivers; HMAC over the exact stored bytes; 429/500 bodies carry no key value, tenant or key name -- recorded in the 41-03, 41-08 and 41-09 SUMMARYs |
| PostgreSQL contract legs (`balance`, notices, schedule creator, webhook event) against a live server | ALLOW-01, ALLOW-02, ALLOW-04 | This devcontainer has no Docker socket and no PostgreSQL server | Local runs print `SKIP:`; CI's `postgres-integration` job is authoritative and fails on any `SKIP:` line -- the SUMMARYs say which ran where |
| Accepted over-admission race between two simultaneous admissions | ALLOW-01 | Cannot be prevented mechanically under D-05's check-only admission (backstop truth in 41-02); closed by Phase 42 | Confirm ADR-0056 (41-09 Task 1) records the race and its Phase 42 closure |
| Worker-pool `with_treasury_notices` attachment inside `build_run_api` | ALLOW-04 | No observable output through the production builder (backstop truth in 41-08): the pool's sinks are the event bus (seven SSE wire events) and the herald log target | 41-07's acceptance grep plus its worker tests, and the 41-08 integrated test's pool built with the same builder |
| Operator UAT walkthrough | ALLOW-01, ALLOW-02 | Needs a running server and an operator config | Configure `treasurer.allowance.api_keys.ci-runner: { period: "1h", amount: "2.50" }`, spend to `2.5000 USD`, observe `429 allowance_exhausted` with `Retry-After` equal to the seconds left in the hour, and confirm `paladin-cli treasury spend --api-key ci-runner --since <window_start>` reports the same balance (41-09 verification) |

All other phase behaviors have automated verification.

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (plan-time check: the only task without one is the human `checkpoint:decision` gate 41-01-01)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify (plan-time check)
- [x] Wave 0 covers all MISSING references (plan-time check: each new file is created by the task that first tests it)
- [x] No watch-mode flags
- [ ] Feedback latency < 180 s for per-task quick commands (to be measured during execution)
- [ ] `nyquist_compliant: true` set in frontmatter (set only by `/gsd-validate-phase 41` after execution)

**Approval:** pending
