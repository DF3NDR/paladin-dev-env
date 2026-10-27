---
phase: 41-admission-time-allowance-enforcement
plan: 02
subsystem: treasurer
tags: [allowance, admission, treasury-ledger, postgres, sqlite, contract-tests, rust]

requires:
  - phase: 41-admission-time-allowance-enforcement
    plan: 01
    provides: BalanceQuery, defaulted TreasuryLedgerPort::balance, Treasurer and AllowancePolicy, in-memory and SQLite balance
provides:
  - PostgresTreasuryLedger::balance (probe plus ::BIGINT SUM in one read transaction)
  - eight shared balance contract clauses run identically on in-memory, SQLite and PostgreSQL
  - adapter-local exact-instant half-open window tests (in-memory, SQLite)
  - FakeLedger scripted store clock and 18 Treasurer evaluation proofs
affects: [41-03, 41-04, 41-05, 41-06, 41-07, 41-08, 41-09]

tech-stack:
  added: []
  patterns:
    - "Balance SQL is a &'static str prefix plus QueryBuilder::push_bind; no nullable-parameter OR predicates, no caller value in SQL text"
    - "Treasurer proofs run over a scripted ledger whose store clock the test sets, so window edges are exact and never sleep-bracketed"

key-files:
  created:
    - src/application/services/treasurer/tests.rs
  modified:
    - crates/paladin-storage/src/treasury/postgres.rs
    - crates/paladin-storage/src/treasury/contract_tests.rs
    - crates/paladin-storage/src/treasury/in_memory.rs
    - crates/paladin-storage/src/treasury/sqlite.rs
    - src/application/services/treasurer/mod.rs

key-decisions:
  - "No production change to the Treasurer was needed: every boundary, order, short-circuit, fail-closed and read-only proof passed against the 41-01 implementation"
  - "The over-admission race between two same-instant admissions stays an accepted, ADR-recorded backstop (ADR-0056 by 41-09, closed by Phase 42); no test claims to prevent it"
  - "ALLOW-01 and ALLOW-02 are not marked complete: they are proven for the balance read and the Treasurer rule, but fork, agent routes and schedule-fired runs (41-04, 41-05) and the notice legs remain open, as 41-01 recorded"

patterns-established:
  - "Every adapter implements balance behind one contract suite; the PostgreSQL leg is a thin mirror of SQLite (Pitfall 19)"
  - "A comment-filtered grep over the treasurer module's production code confirms zero UserRole and zero paladin_storage references"

requirements-completed: []

duration: ~40min
completed: 2026-10-03
status: complete
---

# Phase 41 Plan 02: Balance on every adapter and the Treasurer's rule proven Summary

**`TreasuryLedgerPort::balance` is now implemented on PostgreSQL and proven by one eight-clause contract on all three adapters, and the Treasurer's admission rule is proven at every boundary, in fixed order, read-only, concurrency-safe and fail-closed against a scripted store clock decades from the real one.**

## Performance

- **Duration:** ~40 min (warm build)
- **Completed:** 2026-10-03
- **Tasks:** 2 (both `type="auto" tdd="true"`)
- **Files:** 5 modified, 1 created

## Accomplishments

- `PostgresTreasuryLedger::balance`: `validate_balance`, one read transaction, a foreign-currency probe built from a `&'static str` prefix with `push_bind` only, then `SELECT COALESCE(SUM(amount_nanos), 0)::BIGINT`; every error through the existing password-redacting `wrap`. It mirrors the SQLite implementation line for line.
- Eight new contract clauses in `contract_tests.rs` (`balance_sums_signed_contributions_in_window`, `key_balance_excludes_other_keys_and_tenants`, `balance_window_is_half_open`, `balance_unbounded_counts_every_row`, `balance_mixed_currency_is_currency_mismatch`, `balance_of_empty_scope_is_zero_in_requested_currency`, `balance_rejects_an_invalid_query`, `balance_is_read_only`), each wired into all three adapters together with the 41-01 clause `tenant_balance_equals_sum_of_key_balances` on PostgreSQL.
- Adapter-local `balance_counts_a_row_at_window_start_and_excludes_one_at_window_end` on in-memory (rows pushed straight into locked state) and SQLite (raw INSERT with `storage_timestamp`-stamped whole-second instants): `[ws, we)` holds only the row at `ws`, `[we, we + 1h)` only the row at `we`.
- `src/application/services/treasurer/tests.rs`: `FakeLedger` (settable store clock, scripted rows, recorded `BalanceQuery` values, `Backend` / `CurrencyMismatch` / `StoreNow` failure modes) and 18 tests. The four 41-01 in-file tests moved here unchanged in behaviour; 14 are new, including `every_ceiling_kind_round_trips_through_the_one_balance_function`, `principal_without_an_entry_never_touches_the_ledger`, `ledger_failure_fails_closed_for_an_allowanced_principal`, `repeated_admission_is_read_only_and_identical` and `concurrent_admissions_write_no_ledger_rows` (sixteen `tokio::spawn`ed admissions, identical decisions, ledger spend view unchanged).

## Task Commits

1. **Task 1: PostgreSQL balance and the balance contract on all three adapters** - `59cc54b` (feat)
2. **Task 2: Treasurer evaluation proofs over a scripted store clock** - `66aa72a` (test)

## TDD / red evidence

- In-memory and SQLite already overrode `balance` (41-01), so the new shared clauses and edge tests were green on first run there; the clauses were not first red on those adapters. To prove they can fail, I mutated the SQLite window bound `attributed_at <` to `<=` and confirmed `balance_counts_a_row_at_window_start_and_excludes_one_at_window_end` failed (`[ws, we) must count the row at ws and exclude the row at we`), then restored the file.
- Treasurer tests: mutated `balance.nanos() >= ceiling.ceiling_nanos` to `>` in `mod.rs`; 11 of 24 tests failed (the D-05 boundary, the order, short-circuit, retry-after and read-only proofs among them); restored.
- PostgreSQL leg was not run red against a defaulted `balance`; it was written after the clauses existed and then run green on a real server (below).

## Verification

- A local PostgreSQL 16 server was available (binaries under `/usr/lib/postgresql/16/bin`, contrary to the dispatch note that none existed). I started a throwaway cluster on `127.0.0.1:5433`, database `paladin_run_test`, user `paladin` (the suite's default `STORAGE_POSTGRES_TEST_URL`), data dir `/var/tmp/pg41`. `cargo test -p paladin-storage --features postgres --lib treasury::postgres` ran **32 passed, 0 SKIP** with every balance clause executing for real. The cluster was left running for later plans in this container.
- `cargo test -p paladin-storage --all-features --lib treasury::` 115 passed.
- `cargo test -p paladin-ai --lib application::services::treasurer` 24 passed (18 in `tests.rs`, 3 in `policy.rs`, 3 in `window.rs`); `allowance_admission_tracer` 1 passed; full `cargo test -p paladin-ai --lib` 1078 passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo fmt --check`, `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p paladin-storage --all-features`: clean.
- `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface`: "API surface unchanged" (4109 items); `.project/current-exports.txt` untouched.
- Acceptance greps: `IS NULL OR` count 0 in `postgres.rs` and `sqlite.rs`; comment-filtered `UserRole` 0 and `paladin_storage` 0 over the treasurer module's production code; every clause name present in all three adapter files.
- Not run: `make security` (no dependency changed).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug in own test] Wrong expected window start in `store_clock_selects_the_window_not_the_local_clock`**
- **Found during:** Task 2 first run
- **Issue:** I asserted a window start string of `2001-09-09T00:00:00Z`; the hourly window containing 01:46:40Z starts at `01:00:00Z`. The production code was right.
- **Fix:** corrected the expected string; no production change.
- **Commit:** `66aa72a`

**2. [Rule 3 - Acceptance check] Reworded two doc comments containing the literal `IS NULL OR`**
- **Found during:** Task 1 acceptance grep (`grep -n "IS NULL OR" postgres.rs sqlite.rs | wc -l` must print 0)
- **Issue:** the new PostgreSQL doc comment, and the 41-01 SQLite one it mirrored, quoted the forbidden predicate while saying it is never used, so the grep matched.
- **Fix:** both comments now say "nullable-parameter OR-predicate". Behaviour unchanged.
- **Files:** `postgres.rs`, `sqlite.rs`
- **Commit:** `59cc54b`

**3. [Note] Acceptance grep `grep -c "async fn balance" postgres.rs` prints 8, not 1**
- The substring also matches the new test functions named `balance_*` required by the same plan. `grep -c "async fn balance(" ...` prints 1, which is the intended check. No change made.

**Total deviations:** 2 small fixes, 1 note; no scope change, no production Treasurer change.

## Authentication Gates

None.

## Known Stubs

None.

## Threat Flags

None beyond the plan's register. T-41-09 (balance SQL) is mitigated and grep-verified; T-41-10 and T-41-11 are covered by `ledger_failure_fails_closed_for_an_allowanced_principal` and the role-free `Treasurer::admit` signature (comment-filtered `UserRole` grep 0); T-41-12 is covered by routing every sqlx error through `wrap`. T-41-13 (same-instant over-admission) is accepted and remains a backstop for ADR-0056 (41-09) and Phase 42, as the plan's prohibition table states.

## Notes for later plans

- PostgreSQL contract tests can now be run for real locally: `runuser -u postgres -- /usr/lib/postgresql/16/bin/pg_ctl -D /var/tmp/pg41/data -o "-p 5433 -k /var/tmp/pg41 -c listen_addresses=127.0.0.1" start` (cluster already initialised, trust auth, user `paladin`, database `paladin_run_test`).
- `requirements-completed` is intentionally empty again, for the reason recorded in 41-01.
- Commit trailers use the attribution reminder's `Co-Authored-By: Claude Sonnet 5.5` line and the session line, not the `Claude Fable 5.1` line in the dispatch note, because the session's attribution instruction is the authoritative one and names the model actually running. The orchestrator may amend if repo policy differs.

## Self-Check: PASSED

`src/application/services/treasurer/tests.rs` exists; commits `59cc54b` and `66aa72a` are present in `git log`.
