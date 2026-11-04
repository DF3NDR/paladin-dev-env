---
phase: 57
slug: register-hygiene-close-out
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-11-04
---

# Phase 57 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution. Source: `57-RESEARCH.md`
> § Validation Architecture, as planned in `57-01`..`57-11-PLAN.md`.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` (libtest, tokio tests, doctests); the bash harness `tests/scripts/*_test.sh` (new: `check-review-dispositions_test.sh`); tool read-backs used by the milestone audit itself (`gsd-tools windows status`, `query summary-extract`, `audit-open`); `make security` (cargo-audit + cargo-deny); `scripts/check-advisory-register.sh`; the new `scripts/check-review-dispositions.sh` |
| **Config file** | `Cargo.toml` workspace; `.cargo/audit.toml`; `deny.toml`; `Makefile` (`test-shell-guards` globs `tests/scripts/*_test.sh`) |
| **Quick run command** | the task's targeted command, e.g. `cargo test -p paladin-storage --features sqlite --lib owner_run_for_thread`, `cargo test -p paladin-web --lib thread_controller::`, `cargo test --lib --features web-server <name>`, `cargo test -p paladin-storage --features s3 --lib minio::`, `bash tests/scripts/check-review-dispositions_test.sh`, `./scripts/check-review-dispositions.sh --phases <list> --pending-ok` |
| **Full suite command** | `make clean-code && make check-gates && make test-shell-guards && make security && make api-surface && make check-migration-allowlist && make check-version-strings && SEMVER_PRUNE_LOCAL_CACHE=1 CARGO_INCREMENTAL=0 ./scripts/check-semver.sh` plus the targeted crate suites (57-11 Task 3) |
| **Estimated runtime** | Quick: 5-90 s (register checks are seconds; a root-crate `--features web-server` test build is about 60-90 s incremental). Full: about 30 min locally (the semver run alone is about 12 min) plus CI on the pushed head (about 60+ min including the Docker build) |

**Environment constraints:**
- About 3.8 GB is free on `/`: every Rust task prints `df -BM --output=avail /` first and uses `CARGO_INCREMENTAL=0`; on ENOSPC it deletes `target/debug/incremental` and retries once.
- There is no `cargo-llvm-cov`, no `actionlint` and no Docker daemon locally: coverage, Workflow Lint, the Postgres legs and RustFS are read from CI (57-11).
- The clone is shallow (boundary 2026-10-08): older commits and the Phase 40 diff are read through `gh api repos/DF3NDR/paladin-dev-env/...`.

---

## Sampling Rate

- **After every task commit:** the task's `<verify><automated>` command; `cargo fmt --all -- --check` for Rust tasks; `shellcheck --severity=warning` for any script edit.
- **After every plan wave:** `make check-gates` and `make security` (green from 57-01 on), plus `make test-shell-guards` after 57-08.
- **Before `/gsd-verify-work`:** 57-11 Task 3. All local gates green, the 12-package semver run at `0 failure(s)`, `windows status` `open_count: 0`, the strict `check-review-dispositions.sh` green, and the required CI jobs green on the pushed head.
- **Max feedback latency:** 120 seconds per task-level check. The long runs (semver, CI) happen only at plan or phase gates.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 57-01-01 | 01 | 1 | REG-06 | T-57-01, T-57-02 | RUSTSEC-2026-0429 suppressed only with a measured compensating control | tool | `./scripts/check-advisory-register.sh` (16/16/9) and `make security` | ✅ | ⬜ pending |
| 57-01-02 | 01 | 1 | REG-06 | T-57-02 | four zip rows carry the D-31 fields; mirrors agree | tool + grep | `grep -c` of the D-31 strings = 4; `make security` | ✅ | ⬜ pending |
| 57-01-03 | 01 | 1 | REG-01, REG-03 | T-57-03, T-57-04 | frontmatter claims backed by appended evidence; sessions archived, not deleted | tool read-back | `gsd-tools query summary-extract` x4; `gsd-tools audit-open` | ✅ | ⬜ pending |
| 57-02-01 | 02 | 1 | REG-02 | T-57-05 | one-way door confirmed by the owner before code | checkpoint:decision | — (blocking) | n/a | ⬜ pending |
| 57-02-02 | 02 | 1 | REG-02 | T-57-05 | a later Admin or schedule run never moves a thread's owner | contract (in-memory, SQLite) + doctest | `cargo test -p paladin-storage --features sqlite --lib owner_run_for_thread_ignores_later_admin_and_unattributed_runs`; `cargo test -p paladin-ai-core --doc permits_thread` | ❌ W0 (clause written first; compile-fail RED) | ⬜ pending |
| 57-02-03 | 02 | 1 | REG-02 | T-57-06, T-57-07, T-57-08 | single-row owner read; schedule-only threads stay Admin-only; additive surface | contract x5 on 3 adapters + semver | `cargo test -p paladin-storage --features sqlite --lib run::`; `--features postgres --lib run::postgres`; `make api-surface`; `check-semver.sh --package` x3 | ❌ W0 | ⬜ pending |
| 57-03-01 | 03 | 2 | REG-02 | T-57-09 | foreign tenant cannot read or submit to acme's thread after an Admin or schedule run | HTTP over SQLite | `cargo test --lib --features web-server thread_owner_is_the_earliest_attributed_run_over_sqlite` | ❌ W0 | ⬜ pending |
| 57-03-02 | 03 | 2 | REG-02 | T-57-10, T-57-11, T-57-12 | newer-schema owner row skipped in the list, hidden on the gate; no backend text in bodies | unit (web, service) | `cargo test -p paladin-web --lib thread_controller::`; `cargo test --lib --features web-server submit_guard_reads_the_owner_run_not_the_newest_run` | ❌ W0 | ⬜ pending |
| 57-03-03 | 03 | 2 | REG-02 | T-57-13 | published API text matches behaviour; no public surface change | baseline + lint | `cargo test -p paladin-web --lib openapi_matches_committed_baseline`; clippy; `make api-surface`; `check-semver.sh --package paladin-web/paladin-ai` | ✅ | ⬜ pending |
| 57-04-01 | 04 | 3 | REG-02 | T-57-14 | ADR-0063, guide and MIGRATION state the owner rule | grep | `grep 'Amended .*(Phase 57, D-02'` ADR-0063; `M-B-24`/`M-B-25` contain `earliest attributed run` | ✅ | ⬜ pending |
| 57-04-02 | 04 | 3 | REG-02 | T-57-15, T-57-16 | operators told which legacy threads change hands; frozen sections untouched | grep + gate | section-scoped awk/grep; `make check-version-strings` | ✅ | ⬜ pending |
| 57-05-01 | 05 | 1 | REG-05, REG-02 | T-57-17 | failure text redacted, then bounded, before persist and log | unit (root) | `cargo test --lib --features web-server agent_kind_run_emits_exactly_one_terminal_event`; `... bounded_run_error_redacts_before_bounding` | ❌ W0 | ⬜ pending |
| 57-05-02 | 05 | 1 | REG-05, REG-02 | T-57-18, T-57-19 | empty secret is absent; the boot warning names the key, never the URL | unit (root) | `cargo test --lib --features web-server operator_webhook_secret_warning_line_`; `... empty_env_operator_webhook_secret_is_treated_as_absent` | ❌ W0 | ⬜ pending |
| 57-05-03 | 05 | 1 | REG-05 | T-57-20 | records only; no surface change | gate | `make api-surface`; `make check-gates`; clippy | ✅ | ⬜ pending |
| 57-06-01 | 06 | 2 | REG-05, REG-02 | T-57-23 | the limit is never a delimiter; no key dropped; truncation reported | unit + RustFS case (CI) | `cargo test -p paladin-storage --features s3 --lib minio::tests::list_request_sends_the_limit_as_max_keys_never_as_a_delimiter`; `cargo test --test lib --features integration-tests,s3-storage file_storage_integration_tests -- --list` | ❌ W0 | ⬜ pending |
| 57-06-02 | 06 | 2 | REG-05, REG-02 | T-57-21, T-57-24 | no S3 body or access-key id in error text | unit | `cargo test -p paladin-storage --features s3 --lib minio::tests::s3_err_summary_never_echoes_the_response_body` | ❌ W0 | ⬜ pending |
| 57-06-03 | 06 | 2 | REG-05 | T-57-22 | no presigned URL printed from a failing test | compile + grep | `grep -c 'without_url()'` >= 3; `cargo test --test lib --features integration-tests,s3-storage --no-run` | ✅ | ⬜ pending |
| 57-07-01 | 07 | 1 | REG-04 | T-57-25 | 20 prohibitions listed verbatim; Phase 42 accepted as-is | file check | `grep -cE '^\| P40-[0-9]{2} \|'` = 20; 42-VALIDATION row present | ❌ W0 (worksheet created) | ⬜ pending |
| 57-07-02 | 07 | 1 | REG-04 | T-57-26, T-57-27 | fresh evidence per row; credential checklist over the Phase 40 diff | file check + test re-runs | `grep -cE '^\| C[1-6] \|'` = 6; no placeholder evidence | ✅ | ⬜ pending |
| 57-07-03 | 07 | 1 | REG-04 | T-57-25 | the owner's verdict per row, never auto-filled | checkpoint:human-verify | — (blocking) | n/a | ⬜ pending |
| 57-08-01 | 08 | 1 | REG-05 | T-57-28 | register and checker end to end on Phase 40 | script | `./scripts/check-review-dispositions.sh --phases 40` | ❌ W0 | ⬜ pending |
| 57-08-02 | 08 | 1 | REG-05 | T-57-28 | the checker cannot pass a vacuous register | shell test | `bash tests/scripts/check-review-dispositions_test.sh`; `make test-shell-guards`; shellcheck | ❌ W0 | ⬜ pending |
| 57-08-03 | 08 | 1 | REG-05 | T-57-29, T-57-30 | 20 findings dispositioned; archive append-only | script + diff | `./scripts/check-review-dispositions.sh --phases 40,42,46,48,50,50.1 --pending-ok` | ✅ | ⬜ pending |
| 57-09-01 | 09 | 2 | REG-05 | T-57-31 | Phase 38 (incl. CR-01) on hunted evidence | script | `./scripts/check-review-dispositions.sh --phases 38 --pending-ok` | ✅ | ⬜ pending |
| 57-09-02 | 09 | 2 | REG-05 | T-57-32, T-57-33 | 32 findings across 10 files dispositioned | script + diff | `./scripts/check-review-dispositions.sh --phases 38,40,42,44,46,47,48,50,50.1,51 --pending-ok` | ✅ | ⬜ pending |
| 57-10-01 | 10 | 3 | REG-05 | T-57-35 | Phase 45 closed on this phase's fixes; WR-04/WR-06 measured | script + test re-runs | `./scripts/check-review-dispositions.sh --phases 45 --pending-ok` | ✅ | ⬜ pending |
| 57-10-02 | 10 | 3 | REG-05 | T-57-35, T-57-36 | the full register at 51 findings | script | `./scripts/check-review-dispositions.sh --pending-ok` (51/13/0) | ✅ | ⬜ pending |
| 57-10-03 | 10 | 3 | REG-05 | T-57-34 | the owner decides fix or waive for every live item | checkpoint:decision | — (blocking) | n/a | ⬜ pending |
| 57-11-01 | 11 | 4 | REG-02 | T-57-37, T-57-38 | a row closed only on re-run evidence; ledger round-trips | tool + node | `windows status` JSON check for row 56; `parseLedger`/`renderLedger` byte-identical | ✅ | ⬜ pending |
| 57-11-02 | 11 | 4 | REG-02, REG-04, REG-05 | T-57-37, T-57-39 | D-03 waiver shape; strict register; judgment read recorded | tool + script | `windows status` JSON check (only 68 open); `./scripts/check-review-dispositions.sh` strict; `40-VALIDATION.md` one-cell diff | ✅ | ⬜ pending |
| 57-11-03 | 11 | 4 | REG-02, REG-06 | T-57-37, T-57-40 | `open_count: 0` on CI evidence from the pushed head | tool + CI read | `windows status` `open_count == 0`; `make security`; CI job conclusions via `gh api` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Each RED below is written inside its own task before the fix (TDD, D-00a); no separate Wave 0 plan is needed:

- [ ] `crates/paladin-storage/src/run/contract_tests.rs`: five `owner_run_for_thread_*` clauses wired into the in-memory, SQLite and Postgres test modules (57-02)
- [ ] `src/application/services/run/http_surface_tests.rs`: `thread_owner_is_the_earliest_attributed_run_over_sqlite` (57-03)
- [ ] `crates/paladin-web/src/thread_controller.rs`: the `MockRunRepository::owner_run_for_thread` override, `ThreadRunFailure::UnknownSchemaVersion`, and five new or renamed tests (57-03)
- [ ] `src/application/services/run/submission.rs`: `submit_guard_reads_the_owner_run_not_the_newest_run` (57-03)
- [ ] `worker.rs`/`worker_tests.rs`, `run_api_wiring.rs`, `treasurer.rs`: redaction and webhook-secret tests (57-05)
- [ ] `minio.rs` unit tests and the RustFS case `test_list_files_honours_limit_and_drops_no_keys` (57-06)
- [ ] `scripts/check-review-dispositions.sh`, `tests/scripts/check-review-dispositions_test.sh` and fixtures (57-08)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Thread-ownership door (D-02, one-way) and the port shape (D-01, costly) | REG-02 | an authorization semantic published in ADR-0063 needs the owner's confirmation | 57-02 Task 1 `checkpoint:decision`; recorded verbatim in 57-02-SUMMARY `## Decision` |
| Bounded human read of the 20 Phase 40 prohibitions and the 6 credential checks | REG-04 | LLM-judge verdicts are non-authoritative (ADR-550 D4); D-20 makes the owner's per-row read the record | 57-07 Task 3 `checkpoint:human-verify`; 0 `pending` cells before 57-11 |
| Fix-or-waive triage of every live review finding and every reject | REG-05 | D-14/D-21 classification decides scope; the owner owns any exception | 57-10 Task 3 `checkpoint:decision`; `## Triage (Phase 57, <date>)` in the register |
| Postgres contract legs, RustFS list case, 82 % coverage, Workflow Lint | REG-02, REG-05 | no Docker, `cargo-llvm-cov` or `actionlint` in the sandbox | 57-11 Task 3 reads the job conclusions and ids on the pushed head (`gh api .../actions/runs/<id>/jobs`) |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies (the three checkpoint tasks are blocking human gates, each followed by automated checks in 57-11)
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s per task-level check
- [ ] `nyquist_compliant: true` set in frontmatter (by `/gsd-validate-phase` after execution)

**Approval:** pending
