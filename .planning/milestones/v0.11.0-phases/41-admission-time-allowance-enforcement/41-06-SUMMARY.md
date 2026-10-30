---
phase: 41-admission-time-allowance-enforcement
plan: 06
subsystem: treasurer
tags: [allowance, notices, warn-threshold, migration, postgres, sqlite, contract-tests, rust]

requires:
  - phase: 41-admission-time-allowance-enforcement
    plan: 01
    provides: AllowanceWarning / AllowanceNotice / Admission, the recorded design checkpoint (option-b, items 4 and 5), the Treasurer admit loop and AllowanceAdmissionPort confirm / abandon
  - phase: 41-admission-time-allowance-enforcement
    plan: 02
    provides: FakeLedger scripted-clock test harness, balance on every adapter
  - phase: 41-admission-time-allowance-enforcement
    plan: 04
    provides: the shared admit_and_persist lifecycle submit and fork run through
  - phase: 41-admission-time-allowance-enforcement
    plan: 05
    provides: migration 010 (011 and 012 follow it directly on both backends)
provides:
  - NoticeRecord, NoticeOutcome, LIFETIME_WINDOW_START and the integer-only crosses_warn_threshold (paladin-core)
  - AllowanceNotice extended additively with tenant_id, api_key_id, run_id and recorded_at, plus From<&NoticeRecord>
  - TreasuryNoticePort (record / notices_for_run / discard) beside the unchanged TreasuryLedgerPort
  - migration 011 (treasury_notices, idx_treasury_notices_once, idx_treasury_notices_run) on SQLite and PostgreSQL
  - migration 012 (additive idx_treasury_ledger_tenant_window) on SQLite and PostgreSQL
  - in-memory, SQLite and PostgreSQL TreasuryNoticePort implementations and a ten-clause shared notice contract incl. the sixteen-way race
  - Treasurer::with_notices, claims on the admitted path, abandon discards, build_treasury_notices wired in build_run_api
affects: [41-07, 41-08, 41-09]

tech-stack:
  added: []
  patterns:
    - "Once-per-window dedup is a store-enforced unique index plus INSERT ... ON CONFLICT DO NOTHING; rows_affected == 0 is AlreadyRecorded, never an error"
    - "No nullable column in a unique key: '' for tenant scope, the Unix epoch for a lifetime window (C5)"
    - "A notice observes the run and never gates it: claim errors are logged and the run is still admitted"
    - "Claim before insert, give back on abandon: the admission lifecycle that already confirms or abandons carries the notice ids"

key-files:
  created:
    - crates/paladin-ports/src/output/treasury_notice_port.rs
    - crates/paladin-storage/migrations/sqlite/011_create_treasury_notices.sql
    - crates/paladin-storage/migrations/postgres/011_create_treasury_notices.sql
    - crates/paladin-storage/migrations/sqlite/012_add_treasury_ledger_tenant_index.sql
    - crates/paladin-storage/migrations/postgres/012_add_treasury_ledger_tenant_index.sql
    - crates/paladin-storage/src/treasury/notice_contract_tests.rs
  modified:
    - crates/paladin-core/src/platform/container/allowance.rs
    - crates/paladin-ports/src/output/mod.rs
    - crates/paladin-storage/src/treasury/{mod,in_memory,sqlite,postgres}.rs
    - src/application/services/treasurer/mod.rs
    - src/application/services/treasurer/tests.rs
    - src/application/services/run/submission.rs
    - src/infrastructure/web/run_api_wiring.rs
    - MIGRATION.md
    - CHANGELOG.md
    - crates/paladin-storage/CHANGELOG.md
    - .project/current-exports.txt

key-decisions:
  - "Followed the 41-01 checkpoint (option-b, item 4 and item 5) exactly: api_key_id NOT NULL with '' for tenant scope, epoch window_start for lifetime, window_end and warn_at columns outside the identity, six-column unique key, ON CONFLICT DO NOTHING, claim before insert with confirm / abandon"
  - "validate_notice also rejects a Window notice without both bounds and a Lifetime notice with either (beyond the plan's list), so the epoch sentinel can never be reached by a malformed window notice and every adapter agrees on what a lifetime row reads back as"
  - "Notice instants are normalised through crate::run::storage_timestamp on every adapter (the in-memory one too), so a notice reads back identically on all three"
  - "The migration 011 unique index is written on one line so the register's literal text matches the adapters' ON CONFLICT column list; the per-adapter arbiter tests assert that exact one-line form"
  - "build_treasury_notices opens a second adapter on the same URL, as the plan said, rather than reshaping build_treasury_ledger to return both ports"

patterns-established:
  - "One RawNotice row decoder in treasury/mod.rs (cfg sqlite or postgres) owns the '' and epoch to None mapping for both SQL adapters"
  - "A spy TreasuryNoticePort over a real in-memory store lets lifecycle tests assert both deduplication (the adapter's own) and call counts"

requirements-completed: []

duration: ~75min
completed: 2026-10-04
status: complete
---

# Phase 41 Plan 06: Once-per-window notice store and warn-crossing claims Summary

**A balance that reaches `warn_at` percent of an allowance ceiling on an admitted request now claims exactly one durable notice per scope, limit, window and ceiling across every replica (unique index plus `ON CONFLICT DO NOTHING` on all three adapters, proven by a sixteen-way race), the notice names the admitting run, a notice-store failure never blocks the run, and a run that is admitted but never persisted gives its notice back.**

## Performance

- **Duration:** ~75 min (warm workspace)
- **Completed:** 2026-10-04
- **Tasks:** 3 (Tasks 1 and 2 `tdd="true"`, Task 3 additive registers)
- **Files:** 6 created, 18 modified

## Accomplishments

- **Core (Task 1).** `NoticeRecord`, `NoticeOutcome`, `LIFETIME_WINDOW_START` (`DateTime::<Utc>::UNIX_EPOCH`, available at the pinned chrono) and `crosses_warn_threshold` (`i128`, no floating point, `warn_at == 0` never crosses, exact at `i64::MAX`). `AllowanceNotice` gained `tenant_id`, `api_key_id`, `run_id`, `recorded_at` and `From<&NoticeRecord>` so 41-08's `confirm` needs no store re-read.
- **Port and migration 011.** `TreasuryNoticePort` is a sibling of `TreasuryLedgerPort` (no change to its eight implementors) with a mock-driven module doctest. `011_create_treasury_notices.sql` on both backends carries the full `007`-style header explaining every column and both sentinels.
- **Adapters.** In-memory (one lock across identity check and push), SQLite and PostgreSQL (`NOTICE_INSERT` with the six-column `ON CONFLICT` arbiter, `NOTICES_FOR_RUN` ordered `recorded_at, notice_id`, a transactional / `ANY($1)` discard). A shared `RawNotice` decoder maps `''` back to `None` and a lifetime row's bounds back to `None`. `notice_arbiter_matches_the_migration` on each SQL adapter keeps the `ON CONFLICT` list and the index text in sync.
- **Contract (`notice_contract_tests.rs`).** `first_claim_wins_duplicate_is_already_recorded`, `tenant_scope_duplicate_dedups` (the C5 guard), `lifetime_notice_dedups_per_ceiling`, `raised_ceiling_rearms_the_same_window` (also proves `warn_at` alone does not re-arm), `distinct_window_start_is_a_distinct_notice`, `sixteen_concurrent_claims_yield_exactly_one_recorded`, `notices_for_run_returns_only_that_runs_rows`, `discard_removes_only_the_named_rows`, `notice_round_trips_every_field`, `invalid_notice_is_rejected_before_io`; run unchanged on in-memory, SQLite (the race over `new_shared_file` WAL) and PostgreSQL.
- **Treasurer (Task 2).** `Treasurer::with_notices`; in `admit` every ceiling's pre-admission balance is tested with `crosses_warn_threshold`, and only after every ceiling fits are the crossings claimed (fresh UUIDv7 `notice_id`, the admitting `run_id`, `recorded_at` = the truncated store instant). `Recorded` yields the notice, `AlreadyRecorded` nothing, an error is logged (scope kind, tenant id, error) and swallowed. `abandon` discards exactly the admission's own ids; `confirm` stays a no-op (the operator webhook is 41-08). No change to the `RunSubmissionService` lifecycle helper was needed.
- **Wiring.** `build_treasury_notices` mirrors `build_treasury_ledger` arm for arm (including the `storage-postgres` named-feature error twin) and is attached in `build_run_api` beside the ledger.
- **Migration 012 and registers (Task 3).** `idx_treasury_ledger_tenant_window ON treasury_ledger (tenant_id, attributed_at)` (additive, `007` byte-untouched). MIGRATION.md 9.4 bullets for `treasury_notices` and the index, a root CHANGELOG paragraph, a `paladin-storage` CHANGELOG entry, and the public-API baseline (4158 to 4159 items, the only diff is `Treasurer::with_notices`).

## Task Commits

1. **Task 1: store-enforced notices on all three adapters (migration 011)** - `4bbb09d` (feat)
2. **Task 2: claim crossings on the admitted path, abandon discards, wiring** - `ce3c5ba` (feat)
3. **Task 3: tenant-window ledger index (012) and registers** - `30c3487` (feat)

## TDD / red evidence

- **Task 1, adapters:** the ten notice clauses were wired on the in-memory adapter against a deliberately unimplemented `TreasuryNoticePort` first (`record` returned `InvalidRequest "not implemented"`): `10 failed; 31 passed`. They went green once the in-memory implementation landed (41 passed), then ran green unchanged on SQLite (104) and PostgreSQL.
- **Task 1, core:** honest note, `crosses_warn_threshold` and its four named tests were written together and passed on first run; there was no separate red step for this pure function.
- **Task 2:** mutating `abandon` to a no-op (`|| true` guard) failed `abandon_discards_only_this_admissions_notices` and `abandon_of_an_empty_admission_touches_nothing_and_a_discard_error_is_swallowed` (2 failed of 37) and, through the real `RunSubmissionService`, `threadbusy_after_a_won_notice_abandons_it_so_the_next_admission_rewins` (1 failed of 32). The mutation was reverted (`grep -c "|| true"` prints 0). The claim-path tests were written with the implementation, not before it.

## Verification

- **PostgreSQL ran for real** on the throwaway cluster (port 5433, still up): `cargo test -p paladin-storage --features sqlite,postgres --lib treasury:: -- --nocapture` ran **147 passed, 0 failed, 0 SKIP lines**, including all ten notice clauses, `notice_arbiter_matches_the_migration` and `migrations_011_and_012_are_applied` on PostgreSQL. `_sqlx_migrations` on the cluster lists versions 1 to 12 and `pg_indexes` shows `idx_treasury_ledger_tenant_window` on `(tenant_id, attributed_at)`.
- `cargo test -p paladin-ai-core --lib allowance` 13 passed; `cargo test -p paladin-ports --doc treasury_notice_port` 1 passed; `cargo test -p paladin-storage --lib treasury::in_memory` 41 passed; `--features sqlite --lib treasury::` 104 then (with the two migration-apply tests) green.
- `cargo test -p paladin-ai --lib application::services::treasurer` 37 passed (incl. `sixteen_concurrent_crossings_yield_exactly_one_notice`, `notices_store_failure_never_blocks_admission`, `warn_at_zero_and_one_hundred_never_notify`); `--lib application::services::run::submission` 32 passed; `--lib application::services::run` 192; `--features web-server --lib infrastructure::web::run_api_wiring` 19 passed (incl. the new `wired_treasurer_records_a_warn_notice_for_the_admitted_run`, which POSTs through the real run router and reads the notice back through a store `build_treasury_notices` opened); `allowance_admission_tracer` 1 passed; treasurer doctests 10.
- `cargo test --workspace` (default features): **6278 passed, 0 failed**.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo fmt --check`, `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` (core, ports, storage, facade, all features): clean.
- `./scripts/check-migration-allowlist.sh` exit 0; no `TBD` in MIGRATION.md; `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` exits 0 after `make api-surface-update` (4159 items).
- Acceptance greps: `api_key_id` is `NOT NULL` in the SQLite `011` table (the awk check prints 1); both `011` files contain the one-line `CREATE UNIQUE INDEX IF NOT EXISTS idx_treasury_notices_once ON treasury_notices (...)`; both `012` files contain the exact index statement; `git diff --stat` over both `007` files prints nothing; comment-filtered `f64` count in `allowance.rs` is 0; `with_notices`, `crosses_warn_threshold(` and `.discard(` are in the Treasurer; `build_treasury_notices` and `.with_notices(` are in the wiring.
- Not run: `make security` (cargo-audit and cargo-deny): no dependency changed in this plan.
- Manual credential-handling review: notice rows hold tenant ids, API key NAMES and figures only (no key value, no secret); every notice SQL string is a `&'static str` constant with bound parameters (T-41-31); the notice error log line carries scope kind, tenant id and the adapter error (already password-redacted through each adapter's `wrap`); no HTTP client or redirect behaviour changed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical] Stricter `validate_notice`**
- **Found during:** Task 1 adapter design
- **Issue:** the plan's validation list did not stop a `Window` notice with no bounds from being stored under the epoch sentinel, or a `Lifetime` notice with a window from storing a real `window_start` while reading back as `None`.
- **Fix:** `validate_notice` rejects a window notice without both bounds and a lifetime notice with either, with two extra cases in `invalid_notice_is_rejected_before_io`.
- **Files:** `crates/paladin-storage/src/treasury/mod.rs`, `notice_contract_tests.rs`
- **Commit:** `4bbb09d`

**2. [Rule 1 - Bug in own test] Wrong boundary constants and thread fixture**
- `crossing_math_is_exact_at_a_huge_ceiling` first used a boundary one nano too high (the exact boundary for 80 percent of `i64::MAX` is `7_378_697_629_483_820_646`); and the ThreadBusy lifecycle test first occupied the thread with an unattributed run, which the tenant guard turns into `UnknownThread`. Both were test errors, fixed before commit; production code was right.
- **Commit:** `ce3c5ba`

**3. [Rule 3 - Blocking] Migration files are not a rebuild trigger**
- **Found during:** Task 3 verification
- **Issue:** `sqlx::migrate!` embeds the directory at macro expansion and `paladin-storage` has no `build.rs`, so adding `012` did not recompile the adapters; the first suite run silently did not apply it (PostgreSQL's `_sqlx_migrations` stopped at 11).
- **Fix:** touched the files containing `sqlx::migrate!` and re-ran; added `migrations_011_and_012_apply_to_a_fresh_database` (SQLite) and `migrations_011_and_012_are_applied` (PostgreSQL) so a missing migration is now a test failure, not a silent skip. Also, because I edited `011` after its first PostgreSQL apply (one-line index form), I dropped `treasury_notices` and removed `_sqlx_migrations` row 11 on the throwaway cluster before re-running; nothing outside this container's scratch database was touched.
- **Commit:** `30c3487`

**4. [Note - scope] Files beyond the plan's list**
- `crates/paladin-storage/CHANGELOG.md` (the crate has recorded every storage migration since `008`, as 41-05 did for `010`); `submission.rs` and `treasurer/tests.rs` test additions only; a `wired_treasurer_records_a_warn_notice_for_the_admitted_run` wiring test in `run_api_wiring.rs` in addition to the two `build_treasury_notices_*` tests the plan named.

**5. [Note - attribution] Commit trailers**
- The three commits carry `Co-Authored-By: Claude Sonnet 5.5` plus the session line, from the session's attribution reminder (the model that actually ran), not the `Claude Fable 5.1` line in the dispatch note; 41-01 and 41-02 did the same. The orchestrator may amend if repo policy differs.

**Total deviations:** 1 missing-critical, 1 own-test bug, 1 blocking tooling issue, 2 notes; no scope change.

## Authentication Gates

None.

## Known Stubs

None. `Treasurer::confirm` is deliberately still a no-op: the durable notice already exists at `admit` time and the operator webhook leg attaches in 41-08 (documented in its rustdoc).

## Threat Flags

None beyond the plan's register. T-41-28 (duplicate notices across replicas) is mitigated by the unique index, the `''` / epoch sentinels, the sixteen-way race clause on every adapter and the per-adapter arbiter-text tests. T-41-29 (notice store blocking runs) by `notices_store_failure_never_blocks_admission`. T-41-30 (phantom notice from a run that never persisted) by `abandon` and `threadbusy_after_a_won_notice_abandons_it_so_the_next_admission_rewins`. T-41-31 (SQL) by static SQL constants with binds only. T-41-32 and T-41-33 stay accepted; the at-most-once-per-window guarantee on a crash between claim and insert is documented in the migration header, MIGRATION.md and the Treasurer module docs.

## Notes for later plans

- 41-07 (worker trace event, herald, `RunScope` warning) reads `notices_for_run` on a run's first dispatch; the rows carry the full `AllowanceWarning` (window bounds, `warn_at`) so nothing needs re-deriving. 41-08's `confirm` receives `AllowanceNotice` values already carrying `tenant_id`, `api_key_id` (names only) and `run_id`, matching the option-b operator payload.
- `requirements-completed` is intentionally empty: ALLOW-04 also needs the trace event, herald line and operator webhook (41-07, 41-08); only the durable once-per-window notice exists after this plan.
- Adding or editing a migration file does not trigger a `paladin-storage` rebuild (no `build.rs`); touch a file containing `sqlx::migrate!` before testing, and drop the throwaway PostgreSQL cluster's `_sqlx_migrations` row if an already-applied migration's text is edited.

## Self-Check: PASSED

All six new files exist; commits `4bbb09d`, `ce3c5ba` and `30c3487` are present in `git log`; both `007` migration files are byte-untouched.
