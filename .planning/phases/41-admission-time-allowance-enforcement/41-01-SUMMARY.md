---
phase: 41-admission-time-allowance-enforcement
plan: 01
subsystem: treasurer
tags: [allowance, admission, treasury-ledger, axum, sqlite, rust]

requires:
  - phase: 39-treasury-ledger
    provides: TreasuryLedgerPort (reserve/release/settle/spend/store_now), SqliteTreasuryLedger, contract suite
  - phase: 40-tenant-scoping
    provides: RunAttribution, PrincipalRef, server-derived tenant on every submission
provides:
  - AllowanceRefusal / AllowanceWarning / AllowanceNotice / Admission core value types (D-14)
  - AllowanceAdmissionPort input port and AdmissionError (C1)
  - TreasuryLedgerPort::balance (defaulted) plus BalanceQuery, on the in-memory and SQLite adapters
  - treasurer.allowance.api_keys.<name> { period, amount } config grammar and AllowancePolicy
  - Treasurer facade service and window_for (store-clock tumbling windows)
  - RunSubmissionService::with_treasurer admission slot and RunSubmissionError::AllowanceExhausted
  - ApiError::allowance_exhausted with Retry-After (429 allowance_exhausted)
  - allowance_admission_tracer, the phase's end-to-end proof
affects: [41-02, 41-03, 41-04, 41-05, 41-06, 41-07, 41-08, 41-09]

tech-stack:
  added: []
  patterns:
    - "Admission is a check only: balance read per ceiling, short-circuit on first refusal, fail closed on any ledger error"
    - "One store-clock read per admission, truncated to whole seconds, feeds window_for, evaluated_at and Retry-After"
    - "Defaulted port method (X-10.4) so adding balance breaks no implementor and the engine test doubles stay untouched (D-00b)"

key-files:
  created:
    - crates/paladin-core/src/platform/container/allowance.rs
    - crates/paladin-ports/src/input/allowance_admission_port.rs
    - src/application/services/treasurer/mod.rs
    - src/application/services/treasurer/policy.rs
    - src/application/services/treasurer/window.rs
  modified:
    - crates/paladin-core/src/platform/container/mod.rs
    - crates/paladin-core/src/platform/container/treasury_ledger.rs
    - crates/paladin-ports/src/input/mod.rs
    - crates/paladin-ports/src/input/run_submission_port.rs
    - crates/paladin-ports/src/output/treasury_ledger_port.rs
    - crates/paladin-storage/src/treasury/{mod,in_memory,sqlite,contract_tests}.rs
    - src/config/treasurer.rs
    - src/application/services/mod.rs
    - src/application/services/run/submission.rs
    - src/application/services/run/http_surface_tests.rs
    - crates/paladin-web/src/error.rs
    - crates/paladin-web/src/run_controller.rs
    - MIGRATION.md
    - CHANGELOG.md
    - .project/current-exports.txt

key-decisions:
  - "Checkpoint decision: option-b (design approved as proposed; D-17 operator payload amended to twelve keys)"
  - "TreasuryLedgerPort::balance is defaulted (InvalidRequest), so MIGRATION.md 9.2 rows are N with no allowlist entry (C11)"
  - "Admission confirm/abandon only fire when admit was actually called (a request with no principal never touches the Treasurer)"
  - "Treasurer confirm/abandon are no-ops until 41-06 gives it a notice store"

patterns-established:
  - "Allowance ceilings are produced by one function (AllowancePolicy::ceilings_for) in the fixed order key-window, key-lifetime, tenant-window, tenant-lifetime"
  - "Refusal body carries only the refused ceiling's own six figures; tenant, key name and key value never appear (D-13, D-00g)"

requirements-completed: []

duration: ~45min
completed: 2026-10-03
status: complete
---

# Phase 41 Plan 01: Admission tracer Summary

**An exhausted per-API-key rolling-window allowance configured through `TreasurerConfig` is refused at `POST /v1/runs` with `429 allowance_exhausted` and a store-clock `Retry-After`, before any run is persisted, proven end to end through the real router, `Treasurer` and an on-disk SQLite ledger.**

## Checkpoint decision

**Selection: `option-b`.** Approve the consolidated design as proposed (items 1-8 unchanged, no redirect) AND amend D-17 so the operator webhook payload also carries `tenant_id` and `api_key_id` (names only, never a key value) -- twelve keys instead of ten.

Pre-resolved by the operator on 2026-10-03 during plan-phase; see 41-CONTEXT.md D-17 amendment.

Plans 41-05 (item 7), 41-06 (item 4) and 41-08 (item 6 and the option-b payload branch) read this recorded outcome. This heading was written before any file in the plan's `files_modified` was touched.

## Performance

- **Duration:** ~45 min (cold workspace build included)
- **Started:** 2026-10-03T16:48Z
- **Completed:** 2026-10-03T17:31Z
- **Tasks:** 4 (Task 1 pre-resolved checkpoint, Tasks 2-4 executed)
- **Files modified:** 23 plus the new `treasurer/` module (5 new source files)

## Accomplishments

- Core refusal shape (`AllowanceRefusal` with `evaluated_at` and `retry_after_secs`, `AllowanceWarning`, `AllowanceNotice`, `Admission`) and `BalanceQuery`, all pure and serde-derived.
- `AllowanceAdmissionPort` input port (admit / confirm / abandon) so `paladin-web` and `RunSubmissionService` hold `Arc<dyn AllowanceAdmissionPort>` without naming the facade.
- `TreasuryLedgerPort::balance` as a defaulted method; in-memory and SQLite adapters override it (static SQL prefixes plus `push_bind`, one read transaction for the currency probe and the SUM).
- `treasurer.allowance.api_keys` grammar (`period` `<int><m|h|d>` 1m..366d, `amount` decimal string), `AllowancePolicy`, `window_for`, and the `Treasurer` service.
- `RunSubmissionService::submit` admission slot after SSRF, resolve, role and thread-visibility checks and before any insert/enqueue, with confirm after enqueue and abandon on persistence failure.
- `ApiError::allowance_exhausted` (429, dedicated code, six-key `details`, `Retry-After` only for window refusals) and the explicit `map_submission_error` arm.
- MIGRATION.md 9.2 rows (`TreasuryLedgerPort` N, `RunSubmissionError` N, `ApiError` N), a 9.6 entry, a CHANGELOG entry and a refreshed `.project/current-exports.txt` (4044 to 4109 items; only this plan's symbols differ).

## Task Commits

1. **Task 1: design checkpoint** -- pre-resolved `option-b`, no commit (recorded above).
2. **Task 2: refusal shape, admission port, ledger balance read** -- `8be4972` (feat)
3. **Task 3: tracer slice (config, policy, window, Treasurer, submit slot, 429 helper, tracer test)** -- `f027cfd` (feat)
4. **Task 4: registers and public-API baseline** -- `78917eb` (docs)

## TDD / red runs

- **Phase's first red test:** `tenant_balance_equals_sum_of_key_balances` was wired for the in-memory and SQLite adapters with only the defaulted `balance` in place and failed on both: `called Result::unwrap() on an Err value: InvalidRequest { message: "this TreasuryLedgerPort does not implement balance" }` (contract_tests.rs). It went green once both adapters overrode `balance`.
- **Tracer red run:** `allowance_admission_tracer` was run with the `RunSubmissionService` built WITHOUT `.with_treasurer(..)`; assertion (a) failed: `assertion left == right failed, left: 202, right: 429` -- the router accepted the exhausted caller, proving the test detects a missing admission slot. With the Treasurer attached it passes (`1 passed; 0 failed`).

## Verification

- `cargo test -p paladin-ai-core --lib allowance` (7), `--lib treasury_ledger` (8), doctests for `allowance` and `treasury_ledger`: pass.
- `cargo test -p paladin-ports --lib`/`--doc` (admission port doctest, balance default test): pass.
- `cargo test -p paladin-storage --features sqlite --lib treasury::` 65 passed, including the new contract clause on both adapters.
- `cargo test -p paladin-ai --lib config::treasurer` (20), `application::services::treasurer` (10), `application::services::run::submission` (22), `allowance_admission_tracer` (1): pass.
- `cargo test -p paladin-web --lib` 264 passed; `cargo test --workspace --lib --all-features` all green.
- Intermediate green gate after Task 2: `cargo check --workspace --all-targets --all-features` exit 0.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo fmt --check`, `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` (changed crates): clean.
- `./scripts/check-migration-allowlist.sh` exit 0; `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` exit 0 after `make api-surface-update`.
- Acceptance greps: zero `paladin_storage`, `Utc::now` and `f64` in the treasurer module's production code; `git diff --stat` over `crates/paladin-battalion/src/engine` and `src/application/services/paladin/middleware/limits.rs` is empty (D-00b).
- Not run: `make security` (cargo-audit and cargo-deny) -- no dependency changed; no Docker/Postgres-gated test touched (Postgres `balance` is 41-02).

## Decisions Made

- Followed the plan as written, including the defaulted `balance` (C11), so the register rows are `N`.
- `submit` confirms or abandons only when `admit` was actually invoked, so the "no principal never calls the treasurer" test holds with zero confirm/abandon calls.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Dropped the outer `///` doc on `pub mod allowance;` in `crates/paladin-core/src/platform/container/mod.rs`**
- **Found during:** Task 3 doc gate (`RUSTDOCFLAGS="-D warnings" cargo doc`)
- **Issue:** the plan asked for a one-line doc comment on the registration, but the module file has its own `//!` docs; rustdoc merges outer and inner docs and resolved the module's intra-doc links in the parent scope (the Phase 40 40-06 lesson in STATE.md), failing `-D warnings`.
- **Fix:** removed the outer comment (the file's `//!` docs carry the description). Separately, the new `BalanceQuery` link in `treasury_ledger.rs` module docs uses the full `crate::platform::container::treasury_ledger::BalanceQuery` path, matching its neighbours.
- **Files modified:** `crates/paladin-core/src/platform/container/mod.rs`, `crates/paladin-core/src/platform/container/treasury_ledger.rs`
- **Commit:** `f027cfd`

**2. [Rule 3 - Blocking] Local tooling install for Task 4**
- **Found during:** Task 4
- **Issue:** the sandbox had neither the pinned nightly nor `cargo-public-api` (the plan anticipated this).
- **Fix:** `rustup toolchain install nightly-2026-09-20 --profile minimal` and `cargo install --locked cargo-public-api@0.52.0` (the version named in the baseline header and CI's `api-surface` job). No repository file changed for this.

**3. [Rule 1 - Lint] Clippy/rustfmt follow-ups inside Task 3**
- `unused import` for `Admission` in `submission.rs` (moved into the test module) and a `doc_lazy_continuation` warning in `treasurer/window.rs` (module docs reflowed). Fixed before the Task 3 commit.

### Open question for the orchestrator

The dispatch note says never to put a model identifier in commit messages, while the session's attribution reminder requires the `Co-Authored-By: Claude Sonnet 5.5` and `Claude-Session` trailers. The three commits carry the trailers from the attribution reminder; if the repo policy is "no model identifier", those three commit messages (`8be4972`, `f027cfd`, `78917eb`) need their trailers amended before push.

**Total deviations:** 3 auto-fixed (two Rule 3, one lint), no scope change.

## Authentication Gates

None.

## Known Stubs

None. `Treasurer::confirm` and `Treasurer::abandon` are deliberate no-ops documented in their rustdoc (no notice store until 41-06); they are not stubs standing in for the plan's goal.

## Threat Flags

None beyond the plan's threat register: the new surface (the 429 body and `Retry-After` header, the `balance` SQL) is exactly T-41-02/T-41-03, both mitigated and asserted (raw-body substring check for tenant, key name and key value in the tracer; bound parameters only in the SQLite `balance`). T-41-07 (PostgreSQL fails closed for allowanced principals until 41-02) and T-41-08 (fork, agent routes, schedules until 41-04/41-05) remain accepted and transient inside the phase.

## Notes for later plans

- `requirements-completed` is intentionally empty and `requirements mark-complete` was not run: ALLOW-01 and ALLOW-02 are only proven on `POST /v1/runs` here; fork, the agent routes and schedule-fired runs (41-04, 41-05) and the notices legs are still open.
- Later plans that change the root crate's public surface must repeat `make api-surface-update` (the toolchain is now installed locally).

## Self-Check: PASSED

All five new source files exist; commits `8be4972`, `f027cfd` and `78917eb` are present in `git log`.
