---
phase: 57
slug: register-hygiene-close-out
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-11-04
validated: 2026-11-05
---

# Phase 57 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution. Source: `57-RESEARCH.md`
> § Validation Architecture, as planned in `57-01`..`57-11-PLAN.md`, extended for the gap-closure
> plans `57-12`..`57-16-PLAN.md` (waves 4-8, from `57-10-SUMMARY.md` `## Gaps for closure`); 57-11
> seals last, in wave 9.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` (libtest, tokio tests, doctests); the bash harness `tests/scripts/*_test.sh` (new: `check-review-dispositions_test.sh`); tool read-backs used by the milestone audit itself (`gsd-tools windows status`, `query summary-extract`, `audit-open`); `make security` (cargo-audit + cargo-deny); `scripts/check-advisory-register.sh`; the new `scripts/check-review-dispositions.sh` |
| **Config file** | `Cargo.toml` workspace; `.cargo/audit.toml`; `deny.toml`; `Makefile` (`test-shell-guards` globs `tests/scripts/*_test.sh`) |
| **Quick run command** | the task's targeted command, e.g. `cargo test -p paladin-storage --features sqlite --lib owner_run_for_thread`, `cargo test -p paladin-web --lib thread_controller::`, `cargo test --lib --features web-server <name>`, `cargo test -p paladin-storage --features s3 --lib minio::`, `bash tests/scripts/check-review-dispositions_test.sh`, `./scripts/check-review-dispositions.sh --phases <list> --pending-ok` |
| **Full suite command** | `make clean-code && make check-gates && make test-shell-guards && make security && make api-surface && make check-migration-allowlist && make check-version-strings && CARGO_INCREMENTAL=0 make test && CARGO_INCREMENTAL=0 make test-doc && SEMVER_PRUNE_LOCAL_CACHE=1 CARGO_INCREMENTAL=0 ./scripts/check-semver.sh` plus the targeted crate suites (57-11 Task 3; `make test`/`make test-doc` are the full workspace `cargo test` that CLAUDE.md's pre-commit protocol starts with and that CI's `Unit Tests` job runs) |
| **Estimated runtime** | Quick: 5-90 s (register checks are seconds; a root-crate `--features web-server` test build is about 60-90 s incremental). Full: about 40 min locally (the semver run alone is about 12 min; `make test` and `make test-doc` add a full workspace test build) plus CI on the pushed head (about 60+ min including the Docker build) |

**Environment constraints:**
- About 3.8 GB is free on `/`: every Rust task prints `df -BM --output=avail /` first and uses `CARGO_INCREMENTAL=0`; on ENOSPC it deletes `target/debug/incremental` and retries once.
- There is no `cargo-llvm-cov`, no `actionlint` and no Docker daemon locally: coverage, Workflow Lint, the Postgres legs and RustFS are read from CI (57-11).
- The clone is shallow (boundary 2026-10-08): older commits and the Phase 40 diff are read through `gh api repos/DF3NDR/paladin-dev-env/...`.

---

## Sampling Rate

- **After every task commit:** the task's `<verify><automated>` command; `cargo fmt --all -- --check` for Rust tasks; `shellcheck --severity=warning` for any script edit.
- **After every plan wave:** `make check-gates` and `make security` (green from 57-01 on), plus `make test-shell-guards` after 57-08.
- **Before `/gsd-verify-work`:** 57-11 Task 3. All local gates green (including the full workspace `make test` and `make test-doc`), the 12-package semver run at `0 failure(s)`, `windows status` `open_count: 0`, the strict `check-review-dispositions.sh` green, and the required CI jobs green on the pushed head.
- **Max feedback latency:** 120 seconds per task-level check. The long runs (semver, CI) happen only at plan or phase gates.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 57-01-01 | 01 | 1 | REG-06 | T-57-01, T-57-02 | RUSTSEC-2026-0429 suppressed only with a measured compensating control | tool | `./scripts/check-advisory-register.sh` (16/16/9) and `make security` | ✅ | ✅ green |
| 57-01-02 | 01 | 1 | REG-06 | T-57-02 | four zip rows carry the D-31 fields; mirrors agree | tool + grep | `grep -c` of the D-31 strings = 4; `make security` | ✅ | ✅ green |
| 57-01-03 | 01 | 1 | REG-01, REG-03 | T-57-03, T-57-04 | frontmatter claims backed by appended evidence; sessions archived, not deleted | tool read-back | `gsd-tools query summary-extract` x4; `gsd-tools audit-open` | ✅ | ✅ green |
| 57-02-01 | 02 | 1 | REG-02 | T-57-05 | one-way door confirmed by the owner before code; fails closed on any option but option-a | checkpoint:decision | — (blocking); then `grep -qx 'Selected: option-a' .planning/phases/57-register-hygiene-close-out/57-02-SUMMARY.md` (asserted again by the 57-03 and 57-04 Task 1 preconditions) | n/a | ✅ green |
| 57-02-02 | 02 | 1 | REG-02 | T-57-05 | a later Admin or schedule run never moves a thread's owner | contract (in-memory, SQLite) + doctest | `cargo test -p paladin-storage --features sqlite --lib owner_run_for_thread_ignores_later_admin_and_unattributed_runs`; `cargo test -p paladin-ai-core --doc permits_thread` | ❌ W0 (clause written first; compile-fail RED) | ✅ green |
| 57-02-03 | 02 | 1 | REG-02 | T-57-06, T-57-07, T-57-08 | single-row owner read; schedule-only threads stay Admin-only; additive surface | contract x5 on 3 adapters + semver | `cargo test -p paladin-storage --features sqlite --lib run::`; `--features postgres --lib run::postgres`; `make api-surface`; `check-semver.sh --package` x3 | ❌ W0 | ✅ green |
| 57-03-01 | 03 | 2 | REG-02 | T-57-09 | foreign tenant cannot read or submit to acme's thread after an Admin or schedule run | precondition + HTTP over SQLite | precondition `grep -qx 'Selected: option-a'` on 57-02-SUMMARY (else STOP for replan); `cargo test --lib --features web-server thread_owner_is_the_earliest_attributed_run_over_sqlite` | ❌ W0 | ✅ green |
| 57-03-02 | 03 | 2 | REG-02 | T-57-10, T-57-11, T-57-12 | newer-schema owner row skipped in the list, hidden on the gate; no backend text in bodies | unit (web, service) | `cargo test -p paladin-web --lib thread_controller::`; `cargo test --lib --features web-server submit_guard_reads_the_owner_run_not_the_newest_run` | ❌ W0 | ✅ green |
| 57-03-03 | 03 | 2 | REG-02 | T-57-13 | published API text matches behaviour; no public surface change | baseline + lint | `cargo test -p paladin-web --lib openapi_matches_committed_baseline`; clippy; `make api-surface`; `check-semver.sh --package paladin-web/paladin-ai` | ✅ | ✅ green |
| 57-04-01 | 04 | 3 | REG-02 | T-57-14 | ADR-0063, guide and MIGRATION state the owner rule | precondition + grep | precondition `grep -qx 'Selected: option-a'` on 57-02-SUMMARY (else STOP for replan); `grep 'Amended .*(Phase 57, D-02'` ADR-0063; `M-B-24`/`M-B-25` contain `earliest attributed run` | ✅ | ✅ green |
| 57-04-02 | 04 | 3 | REG-02 | T-57-15, T-57-16 | operators told which legacy threads change hands; frozen sections untouched | grep + gate | section-scoped awk/grep; `make check-version-strings` | ✅ | ✅ green |
| 57-05-01 | 05 | 1 | REG-05, REG-02 | T-57-17 | failure text redacted, then bounded, before persist and log | unit (root) | `cargo test --lib --features web-server agent_kind_run_emits_exactly_one_terminal_event`; `... bounded_run_error_redacts_before_bounding` | ❌ W0 | ✅ green |
| 57-05-02 | 05 | 1 | REG-05, REG-02 | T-57-18, T-57-19 | empty secret is absent; the boot warning names the key, never the URL | unit (root) | `cargo test --lib --features web-server operator_webhook_secret_warning_line_`; `... empty_env_operator_webhook_secret_is_treated_as_absent` | ❌ W0 | ✅ green |
| 57-05-03 | 05 | 1 | REG-05 | T-57-20 | records only; no surface change | gate | `make api-surface`; `make check-gates`; clippy | ✅ | ✅ green |
| 57-06-01 | 06 | 2 | REG-05, REG-02 | T-57-23 | the limit is never a delimiter; no key dropped; truncation reported | unit + RustFS case (CI) | `cargo test -p paladin-storage --features s3 --lib minio::tests::list_request_sends_the_limit_as_max_keys_never_as_a_delimiter`; `cargo test --test lib --features integration-tests,s3-storage file_storage_integration_tests -- --list` | ❌ W0 | ✅ green (RustFS leg on CI `Integration Tests` job 126917997844) |
| 57-06-02 | 06 | 2 | REG-05, REG-02 | T-57-21, T-57-24 | no S3 body or access-key id in error text | unit | `cargo test -p paladin-storage --features s3 --lib minio::tests::s3_err_summary_never_echoes_the_response_body` | ❌ W0 | ✅ green |
| 57-06-03 | 06 | 2 | REG-05 | T-57-22 | no presigned URL printed from a failing test | compile + grep | `grep -c 'without_url()'` >= 3; `cargo test --test lib --features integration-tests,s3-storage --no-run` | ✅ | ✅ green |
| 57-07-01 | 07 | 1 | REG-04 | T-57-25 | 20 prohibitions listed verbatim; Phase 42 accepted as-is | file check | `grep -cE '^\| P40-[0-9]{2} \|'` = 20; 42-VALIDATION row present | ❌ W0 (worksheet created) | ✅ green |
| 57-07-02 | 07 | 1 | REG-04 | T-57-26, T-57-27 | fresh evidence per row; credential checklist over the Phase 40 diff | file check + test re-runs | `grep -cE '^\| C[1-6] \|'` = 6; no placeholder evidence | ✅ | ✅ green |
| 57-07-03 | 07 | 1 | REG-04 | T-57-25 | the owner's verdict per row, never auto-filled | checkpoint:human-verify | — (blocking) | n/a | ✅ green |
| 57-08-01 | 08 | 1 | REG-05 | T-57-28 | register and checker end to end on Phase 40 | script | `./scripts/check-review-dispositions.sh --phases 40` | ❌ W0 | ✅ green |
| 57-08-02 | 08 | 1 | REG-05 | T-57-28 | the checker cannot pass a vacuous register | shell test | `bash tests/scripts/check-review-dispositions_test.sh`; `make test-shell-guards`; shellcheck | ❌ W0 | ✅ green |
| 57-08-03 | 08 | 1 | REG-05 | T-57-29, T-57-30 | 20 findings dispositioned; archive append-only | script + diff | `./scripts/check-review-dispositions.sh --phases 40,42,46,48,50,50.1 --pending-ok` | ✅ | ✅ green |
| 57-09-01 | 09 | 2 | REG-05 | T-57-31 | Phase 38 (incl. CR-01) on hunted evidence | script | `./scripts/check-review-dispositions.sh --phases 38 --pending-ok` | ✅ | ✅ green |
| 57-09-02 | 09 | 2 | REG-05 | T-57-32, T-57-33 | 32 findings across 10 files dispositioned | script + diff | `./scripts/check-review-dispositions.sh --phases 38,40,42,44,46,47,48,50,50.1,51 --pending-ok` | ✅ | ✅ green |
| 57-10-01 | 10 | 3 | REG-05 | T-57-35 | Phase 45 closed on this phase's fixes; WR-04/WR-06 measured | script + test re-runs | `./scripts/check-review-dispositions.sh --phases 45 --pending-ok` | ✅ | ✅ green |
| 57-10-02 | 10 | 3 | REG-05 | T-57-35, T-57-36 | the full register at 51 findings | script | `./scripts/check-review-dispositions.sh --pending-ok` (51/13/0) | ✅ | ✅ green |
| 57-10-03 | 10 | 3 | REG-05 | T-57-34 | the owner decides fix or waive for every live item; fix-in-phase items get `## Gaps for closure` and the expected `--gaps` pass | checkpoint:decision | — (blocking); then `grep -q '^## Gaps for closure' 57-10-SUMMARY.md` when any item is fix-in-phase | n/a | ✅ green |
| 57-12-01 | 12 | 4 | REG-05, REG-02 | T-57-41, T-57-43 | a failed reasoning check reaches the caller as a fixed error-kind label, with no vendor text, directly and through a fallback chain | unit (root, tracer) | `cargo test --lib --features web-server effort_refusal_`; `cargo test --lib --features web-server application::services::paladin::paladin_builder::`; `grep -q 'llm_error_label(&other)'`; `windows status` JSON check (one open `51-REVIEW WR-01` row) | ❌ W0 | ✅ green |
| 57-12-02 | 12 | 4 | REG-05, REG-02 | T-57-41, T-57-42 | credential-handling review recorded; MIGRATION and `[Unreleased]` name the fix; frozen sections untouched | grep + diff + gate | section-scoped awk/grep for `51-REVIEW WR-01` in MIGRATION 11.1 and `[Unreleased]`; frozen-section `diff`; `cargo clippy --lib --tests --features web-server -- -D warnings`; `make api-surface`; `make security`; `make check-gates` | ✅ | ✅ green |
| 57-13-01 | 13 | 5 | REG-05, REG-02 | T-57-44, T-57-45, T-57-47 | every `run_agent` exit publishes its terminal event only after its durable write and ack | unit (root, tracer) | `cargo test --lib --features web-server _emits_no_terminal_event`; `... agent_kind_run_emits_exactly_one_terminal_event`; `... application::services::run::`; `grep -q 'on_committed'`; `windows status` JSON check (one open `45-REVIEW WR-04` row) | ❌ W0 | ✅ green |
| 57-13-02 | 13 | 5 | REG-05, REG-02 | T-57-46 | a failed agent run reports the spend it incurred before failing | unit (root) | `cargo test --lib --features web-server failed_agent_run_`; `... non_llm_failure_after_billed_calls_reports_their_spend`; `... application::services::run::`; `grep -q 'ATTEMPT_SPEND'`; `windows status` JSON check (one open `45-REVIEW WR-06` row) | ❌ W0 | ✅ green |
| 57-13-03 | 13 | 5 | REG-05, REG-02 | T-57-44, T-57-46 | MIGRATION and `[Unreleased]` name both fixes; frozen sections untouched | grep + diff + gate | section-scoped awk/grep for `45-REVIEW WR-04`/`WR-06`; frozen-section `diff`; `cargo clippy --lib --tests --features web-server -- -D warnings`; `make api-surface`; `make check-gates` | ✅ | ✅ green |
| 57-14-01 | 14 | 6 | REG-05, REG-02 | T-57-48, T-57-49 | a failed operator enqueue gives the notice back, so the next admission in the window wins it again | unit (root, tracer) | `cargo test --lib --features web-server enqueue_failure_gives_the_notice_back_so_the_next_admission_wins_it`; `... application::services::treasurer::`; `windows status` JSON check (one open `41-REVIEW WR-01` row) | ❌ W0 | ✅ green |
| 57-14-02 | 14 | 6 | REG-05, REG-02 | T-57-48, T-57-50, T-57-51 | `confirm`/`abandon` survive a dropped caller; a dropped submission settles its admission through a drop guard | unit (root, web) | `cargo test --lib --features web-server _completes_when_its_caller_is_dropped`; `... a_submission_dropped_`; `... application::services::treasurer::`; `... application::services::run::submission`; `cargo test -p paladin-web --lib agent_controller`; `grep -q 'struct AdmissionSettlement'` | ❌ W0 | ✅ green |
| 57-14-03 | 14 | 6 | REG-05, REG-02 | T-57-48 | API docs say the notice is given back; MIGRATION and `[Unreleased]` name the fix | grep + diff + gate | `grep -q 'given back' docs/src/api-reference/platform-api.md` and the old sentence absent; section-scoped awk/grep for `41-REVIEW WR-01`; frozen-section `diff`; clippy `-D warnings`; `make api-surface`; `make check-gates` | ✅ | ✅ green |
| 57-15-01 | 15 | 7 | REG-05, REG-02 | T-57-52, T-57-55 | a zero provider delay leaves a gate of at least the base delay; non-zero delays are honoured exactly | unit (paladin-llm, tracer) | `cargo test -p paladin-llm --lib cadence::tests::a_zero_`; `... cadence::tests::a_past_http_date_retry_after_gates_at_least_the_base_delay`; `... repeated_zero_delay_429s_escalate_the_streak`; `... a_nonzero_retry_after_below_base_is_still_honoured_exactly`; `cargo test -p paladin-ports --lib cadence_port::`; `windows status` JSON check (one open `43-REVIEW WR-05` row) | ❌ W0 | ✅ green |
| 57-15-02 | 15 | 7 | REG-05, REG-02 | T-57-53, T-57-54 | a 429 is recorded in a spawned, awaited task, so a dropped caller cannot lose it (`generate` and stream) | unit (paladin-llm) | `cargo test -p paladin-llm --lib cadence::tests::a_429_is_recorded_when_the_`; `... cadence::`; `... fallback::`; `grep -q 'Handle::try_current'`; `windows status` JSON check (one open `43-REVIEW WR-04` row) | ❌ W0 | ✅ green |
| 57-15-03 | 15 | 7 | REG-05, REG-02 | T-57-52, T-57-53 | changelogs, MIGRATION 11.1 and ADR-0058 name both fixes; frozen sections untouched | grep + diff + gate | section-scoped awk/grep for `43-REVIEW WR-05`/`WR-04` in both changelogs and MIGRATION; `grep -q 'Phase 57, 43-REVIEW WR-0[45]'` ADR-0058; frozen-section `diff` x3; `cargo clippy -p paladin-llm --lib --tests -- -D warnings`; `make api-surface`; `make check-gates` | ✅ | ✅ green |
| 57-16-01 | 16 | 8 | REG-05, REG-02 | T-57-56 | the node-cache fence marker never outlives the counter that numbers it; both TTLs have one definition | unit (paladin-storage, tracer) + lint | `cargo test -p paladin-storage --features redis-cache --lib fence_marker_never_outlives_the_counter_that_numbers_it`; `... node_cache::redis::`; `--features redis-cadence --lib cadence::redis::`; clippy `-D warnings` for `redis-cache`, `redis-cadence`, both and none; no `MIN_FENCE_TTL`/`MIN_FENCE_TTL_MS`/`FENCE_TTL_FACTOR` const left in either file; `windows status` JSON check (one open `43-REVIEW WR-01` row) | ❌ W0 | ✅ green |
| 57-16-02 | 16 | 8 | REG-05, REG-02 | T-57-56, T-57-58 | a fenced write after the counter expires is accepted (live Redis); locally the test takes its `SKIP:` path (EA-57-19), and 57-11 Task 3 reads it `ok` from CI | unit (paladin-storage) + CI read (57-11) | `cargo test -p paladin-storage --features redis-cache --lib node_cache::redis:: -- --nocapture` (`test result: ok`); `grep -q 'async fn fenced_write_after_the_counter_expires_is_accepted'`; clippy `-D warnings` (`redis-cache`) | ❌ W0 | ✅ green (live leg on CI `Redis Node Cache Contract Suite` job 126917997409; local run takes `SKIP:`) |
| 57-16-03 | 16 | 8 | REG-05, REG-02 | T-57-57 | changelogs, MIGRATION 11.1 and ADR-0058 name the fix and the accepted late-holder duplicate; frozen sections untouched | grep + diff + gate | section-scoped awk/grep for `43-REVIEW WR-01` in both changelogs and MIGRATION; `grep -q 'Phase 57, 43-REVIEW WR-01'` ADR-0058; frozen-section `diff` x3; `make api-surface`; `make check-gates` | ✅ | ✅ green |
| 57-11-01 | 11 | 9 | REG-02 | T-57-37, T-57-38 | a row closed only on re-run evidence; ledger round-trips | tool + node | `windows status` JSON check for row 56; `parseLedger`/`renderLedger` byte-identical | ✅ | ✅ green |
| 57-11-02 | 11 | 9 | REG-02, REG-04, REG-05 | T-57-37, T-57-39 | D-03 waiver shape; register clean except the CI-gated 43 WR-01 row; judgment read recorded; Docker-gated rows left open for Task 3 (PD-11-4) | tool + script | `windows status` JSON check (open set is exactly 68, 90, 93 and `43-REVIEW WR-01`); `./scripts/check-review-dispositions.sh --pending-ok` (51/13/0); strict run whose only failure is `FAIL OPEN 43 WR-01` (51/13/1); `40-VALIDATION.md` one-cell diff | ✅ | ✅ green |
| 57-11-03 | 11 | 9 | REG-02, REG-05, REG-06 | T-57-37, T-57-40 | `open_count: 0` on CI evidence from the pushed head; Docker-gated rows flipped only after their job log is read; full workspace suite green | tool + test + CI read | `windows status` `open_count == 0`, with rows 90/93 citing the `Integration Tests` job and `43-REVIEW WR-01` citing the `Redis Node Cache Contract Suite (live server)` job; strict `./scripts/check-review-dispositions.sh` (51/13/0); `make security`; `make test`; `make test-doc`; CI job conclusions via `gh api`, including `Unit Tests (stable)`, `Unit Tests (beta)` and `Integration Tests` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Each RED below is written inside its own task before the fix (TDD, D-00a); no separate Wave 0 plan is needed:

- [x] `crates/paladin-storage/src/run/contract_tests.rs`: five `owner_run_for_thread_*` clauses wired into the in-memory, SQLite and Postgres test modules (57-02)
- [x] `src/application/services/run/http_surface_tests.rs`: `thread_owner_is_the_earliest_attributed_run_over_sqlite` (57-03)
- [x] `crates/paladin-web/src/thread_controller.rs`: the `MockRunRepository::owner_run_for_thread` override, `ThreadRunFailure::UnknownSchemaVersion`, and five new or renamed tests (57-03)
- [x] `src/application/services/run/submission.rs`: `submit_guard_reads_the_owner_run_not_the_newest_run` (57-03)
- [x] `worker.rs`/`worker_tests.rs`, `run_api_wiring.rs`, `treasurer.rs`: redaction and webhook-secret tests (57-05)
- [x] `minio.rs` unit tests and the RustFS case `test_list_files_honours_limit_and_drops_no_keys` (57-06)
- [x] `scripts/check-review-dispositions.sh`, `tests/scripts/check-review-dispositions_test.sh` and fixtures (57-08)
- [x] `src/application/services/paladin/paladin_builder.rs`: the three `effort_refusal_*` tests (57-12)
- [x] `src/application/services/run/worker_tests.rs`: the `*_emits_no_terminal_event` write-failure tests and `failed_agent_run_*` (57-13)
- [x] `src/application/services/treasurer/tests.rs` and `src/application/services/run/submission.rs`: `enqueue_failure_gives_the_notice_back_so_the_next_admission_wins_it`, `*_completes_when_its_caller_is_dropped` and `a_submission_dropped_*` (57-14)
- [x] `crates/paladin-llm/src/cadence.rs`: the `a_zero_*` zero-delay, escalation and non-zero-delay tests, and the `a_429_is_recorded_when_the_*` drop tests (57-15)
- [x] `crates/paladin-storage/src/node_cache/redis.rs`: `fence_marker_never_outlives_the_counter_that_numbers_it` (pure, the watched RED) and `fenced_write_after_the_counter_expires_is_accepted` (live, Docker-gated, read from CI by 57-11) (57-16)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Thread-ownership door (D-02, one-way) and the port shape (D-01, costly) | REG-02 | an authorization semantic published in ADR-0063 needs the owner's confirmation | 57-02 Task 1 `checkpoint:decision`; 57-02-SUMMARY `## Decision` opens with the line `Selected: option-a` (or `option-b`, `option-c`), then the reply verbatim. Fails closed: anything but option-a stops 57-02 Tasks 2-3, 57-03 and 57-04 for a `/gsd-plan-phase 57` revision |
| Bounded human read of the 20 Phase 40 prohibitions and the 6 credential checks | REG-04 | LLM-judge verdicts are non-authoritative (ADR-550 D4); D-20 makes the owner's per-row read the record | 57-07 Task 3 `checkpoint:human-verify`; 0 `pending` cells before 57-11 |
| Fix-or-waive triage of every live review finding and every reject | REG-05 | D-14/D-21 classification decides scope; the owner owns any exception | 57-10 Task 3 `checkpoint:decision`; `## Triage (Phase 57, <date>)` in the register; fix-in-phase items take the expected `/gsd-plan-phase 57 --gaps` pass (gap source `57-10-SUMMARY.md` `## Gaps for closure`) before 57-11 seals (PD-10-1) |
| Postgres contract legs, RustFS list and presigned-URL cases, the live Redis node-cache case (57-16), 82 % coverage, Workflow Lint | REG-02, REG-05 | no Docker, `cargo-llvm-cov` or `actionlint` in the sandbox | 57-11 Task 3 reads the job conclusions and ids on the pushed head (`gh api .../actions/runs/<id>/jobs`), and the job logs for the named tests (no `SKIP:` on the Redis job); WINDOWS rows 90, 93 and `43-REVIEW WR-01` are flipped only after that read (PD-11-4) |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (the three checkpoint tasks are blocking human gates, each followed by automated checks in 57-11)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 120s per task-level check
- [x] `nyquist_compliant: true` set in frontmatter (set by `/gsd-validate-phase 57` on 2026-11-05)

**Approval:** approved 2026-11-05 (`/gsd-validate-phase 57`)

---

## Validation Audit 2026-11-05

| Metric | Count |
|--------|-------|
| Requirements audited | 6 (REG-01..REG-06) |
| Task rows audited | 45 |
| Gaps found | 0 |
| Resolved | 0 |
| Escalated | 0 |

**Method.** Every named test, marker and script in the Per-Task Verification Map was located in the tree (33 distinct symbols, all present), then the automated commands were re-run on head `ea658e4d0` with `CARGO_INCREMENTAL=0`. No test was generated and no implementation file was touched: the phase's TDD REDs (Wave 0 list above) were all written inside their tasks and are green on the head.

**Re-run on this head (all `0 failed`):**

| Command | Result |
|---------|--------|
| `./scripts/check-advisory-register.sh` | 16 rows against 16 deny.toml and 9 audit.toml entries; all clauses satisfied |
| `./scripts/check-review-dispositions.sh` (strict) | checked 51 finding(s) across 13 review file(s); 0 failure(s) |
| `bash tests/scripts/check-review-dispositions_test.sh` | 25 assertions passed; no fixture mutated |
| `gsd-tools windows status` | `open_count: 0`, waived 53, fixed 55, total 108 |
| `make security` | advisories ok, bans ok, licenses ok, sources ok; exit 0 |
| `cargo test -p paladin-storage --features sqlite --lib owner_run_for_thread` / `run::` | 10 passed / 84 passed |
| `cargo test -p paladin-storage --features s3 --lib minio::` | 17 passed |
| `cargo test -p paladin-storage --features redis-cache --lib node_cache::redis::` | 21 passed (local `SKIP:` path for the live case) |
| `cargo test -p paladin-storage --features redis-cadence --lib cadence::redis::` | 31 passed |
| `cargo test -p paladin-ai-core --doc permits_thread` | 1 passed |
| `cargo test -p paladin-ports --lib cadence_port::` | 10 passed |
| `cargo test -p paladin-llm --lib cadence::` / `fallback::` | 35 passed / 63 passed |
| `cargo test -p paladin-web --lib thread_controller::` / `openapi_matches_committed_baseline` / `agent_controller` | 56 / 1 / 71 passed |
| `cargo test --lib --features web-server thread_owner_is_the_earliest_attributed_run_over_sqlite` | 1 passed |
| `cargo test --lib --features web-server application::services::run::submission` / `run::worker` / `run::webhook::` | 38 / 79 / 37 passed |
| `cargo test --lib --features web-server infrastructure::web::run_api_wiring::` / `config::treasurer::` | 51 / 50 passed |
| `cargo test --lib --features web-server application::services::paladin::paladin_builder::` / `effort_refusal_` | 59 / 3 passed |
| `cargo test --lib --features web-server application::services::treasurer::` / `non_llm_failure_after_billed_calls_reports_their_spend` | 85 / 1 passed |
| `cargo test --test lib --features integration-tests,s3-storage --no-run` | compiles; exit 0 |

**Grep assertions re-checked:** `Selected: option-a` in 57-02-SUMMARY; 20 `P40-NN` rows and 6 `C1-6` rows with 0 `pending` cells in `57-JUDGMENT-READ.md`; `## Gaps for closure` present in 57-10-SUMMARY; 4 D-31 zip rows in `SECURITY-EXCEPTIONS.md`; ADR-0063 `Amended ... (Phase 57, D-02` present; ADR-0058 carries the `Phase 57, 43-REVIEW WR-0[145]` references; `given back` in `docs/src/api-reference/platform-api.md`; `without_url()` x6 in the S3 integration test; `42-VALIDATION.md` carries the `accepted as-is` row.

**Manual-only rows stand as recorded.** The three human checkpoints (57-02-01, 57-07-03, 57-10-03) are owner gates by design (D-02, D-20, D-14/D-21), each followed by the automated post-check above. The Docker-gated legs (Postgres contract suites, RustFS list/presigned cases, live Redis fence case, coverage, Workflow Lint) are read from CI run 41927940508 on head 4b115d511 by 57-11 Task 3; this audit confirms the pure legs locally and leaves the CI read as the record.
