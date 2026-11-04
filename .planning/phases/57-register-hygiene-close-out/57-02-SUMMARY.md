---
phase: 57-register-hygiene-close-out
plan: 02
subsystem: storage
tags: [run-repository, thread-ownership, tenant-isolation, adr-0063, sqlite, postgres, semver]

requires:
  - phase: 53-tenant-isolation-closure
    provides: RunReadScope::permits_thread and the latest-run thread-ownership rule this plan replaces
provides:
  - "RunRepositoryPort::owner_run_for_thread (defaulted) returning the thread's earliest attributed run"
  - "single-row owner overrides on the in-memory, SQLite and PostgreSQL run repositories"
  - "five owner_run_for_thread_* contract clauses run by all three adapters"
  - "RunReadScope::permits_thread(owner) with the earliest-attributed-run rule documented"
affects: [57-03, 57-04, 57-11]

tech-stack:
  added: []
  patterns:
    - "defaulted additive port method (X-10.4) with single-row adapter overrides"
    - "ORDER BY (tenant_id IS NULL), submitted_at, run_id LIMIT 1 as the owner read"

key-files:
  created: []
  modified:
    - crates/paladin-ports/src/output/run_repository_port.rs
    - crates/paladin-ports/CHANGELOG.md
    - crates/paladin-storage/src/run/contract_tests.rs
    - crates/paladin-storage/src/run/in_memory.rs
    - crates/paladin-storage/src/run/sqlite.rs
    - crates/paladin-storage/src/run/postgres.rs
    - crates/paladin-storage/CHANGELOG.md
    - crates/paladin-core/src/platform/container/principal.rs
    - crates/paladin-core/CHANGELOG.md

key-decisions:
  - "option-a: ownership is the earliest attributed run, one defaulted method owner_run_for_thread, earliest-run fallback"
  - "permits_thread keeps its predicate and test name; only the parameter name and docs change"

patterns-established:
  - "Authorization reads that must not fail on unrelated rows are single-row LIMIT 1 statements, never a list page"

requirements-completed: []  # REG-02 spans 57-03, 57-04 and 57-11; not ticked here

duration: ~70min
completed: 2026-11-04
status: complete
---

# Phase 57 Plan 02: Thread-owner run lookup Summary

**A thread's owner is now its earliest attributed run, served by one defaulted `RunRepositoryPort::owner_run_for_thread` that the SQLite and PostgreSQL adapters answer with a single `LIMIT 1` row, so a later Admin or schedule run cannot move a thread and a corrupt non-owner row cannot lock the owner out.**

Plan-start SHA: `3ddf92c081e3c432ad274a390e5c1a67542661fe`

## Decision

Selected: option-a

Owner's reply, verbatim: "57-02: option-a"

Received 2026-11-04 in chat from the owner (am0rfu5@protonmail.com). The reply was relayed to this executor by the orchestrator in the continuation prompt; this executor did not receive it from the owner directly and could not verify it independently. Option-a is also the option the plan recommended. Under option-a Tasks 2-3 ran as written; 57-03 and 57-04 may proceed (their Task 1 asserts the line above).

## Performance

- **Duration:** ~70 min (this continuation; the earlier executor only took read-only measurements)
- **Completed:** 2026-11-04
- **Tasks:** 3 of 3 (Task 1 decision recorded, Tasks 2 and 3 committed)
- **Files modified:** 9

## Accomplishments

- `RunRepositoryPort::owner_run_for_thread(&self, &ThreadId) -> Result<Option<Run>, RunRepositoryError>` with a default body that pages `list` (limit 100) and keeps the best candidate by (attributed first, `submitted_at`, `run_id`). Adding it breaks no implementor: all 12 in-tree `impl RunRepositoryPort for` blocks compile unchanged.
- The in-memory, SQLite and PostgreSQL repositories override it. The SQL overrides run one bound statement, `ORDER BY (tenant_id IS NULL) ASC, submitted_at ASC, run_id ASC LIMIT 1`, and decode that single row. Neither calls `list`.
- Five shared contract clauses, each wired as a thin test on all three adapters, plus two default-impl tests on a two-page double and one DB-free Postgres query-shape test.
- `RunReadScope::permits_thread` takes `owner`, documents the earliest-attributed-run rule and keeps its predicate. The test name `permits_thread_admits_all_and_only_the_latest_runs_tenant` is unchanged (PD-02-4) and carries a doc comment explaining that.

## Task Commits

1. **Task 1: thread-ownership door decision (checkpoint)** - recorded in `## Decision`; no code commit by design
2. **Task 2 (tracer): owner_run_for_thread through the port, in-memory override and SQLite, with permits_thread fed the owner** - `a02d53140` (feat)
3. **Task 3: single-row SQLite/Postgres overrides, remaining clauses, default-impl tests, changelogs** - `6bec007e9` (test)

**Plan metadata:** recorded by the docs commit that follows this file.

## Tracer gate

tracer gate: verify re-run passed; orchestrator-approved (unattended session), not owner-approved. The re-run executed the Task 2 `<verify>` on the committed state: the clause printed `... ok` on in-memory and SQLite, the `permits_thread` doctest and `principal::` tests passed, and both greps held.

## Red-green

| Step | RED (watched) | GREEN |
|------|---------------|-------|
| Task 2 clause `owner_run_for_thread_ignores_later_admin_and_unattributed_runs` | Compile failure, `error[E0599]: no method named owner_run_for_thread found for reference &dyn RunRepositoryPort` at `contract_tests.rs:1496`, written before the trait method existed | Trait default body added; in-memory and SQLite legs `ok` (SQLite through the default); in-memory override added, re-run `ok` |
| Task 3 clause `owner_run_for_thread_decodes_only_the_owner_row` | SQLite leg FAILED: `a non-owner newer-schema row must not fail the lookup: UnknownSchemaVersion { found: "v99-from-the-future" }`. This is WINDOWS row 86 reproduced: the default walks `list`, which decodes the planted `v99` row. The in-memory leg passed (its `list` does not schema-check). | `SELECT_OWNER_RUN_FOR_THREAD` override on SQLite and PostgreSQL; the SQLite leg `ok` |
| Task 3 clauses `..._returns_the_earliest_attributed_run`, `..._falls_back_to_the_earliest_run_when_none_is_attributed`, `..._is_none_for_a_thread_with_no_runs` | Passed on first run through the default (no other failure appeared); they pin the contract for the overrides | Still `ok` after the overrides, on in-memory, SQLite and live PostgreSQL |
| `default_impl_of_owner_run_for_thread_walks_every_page` | Written with the trait default already present, so it was not a failing-first test; it passed on first run | `ok` |

## Gates

- **api-surface, before:** `make api-surface` was expected to fail on the new method. It did not: `API surface unchanged` (4331 items). `.project/current-exports.txt` records only the root `paladin` crate (it names `RunRepositoryPort` only as an argument type), so a method on `paladin-ports` never enters it.
- **api-surface, after:** the same, `API surface unchanged`. `make api-surface-update` was not run and the baseline is not modified; running it would only have rewritten the timestamp header.
- **Semver (the real public-surface gate for these crates):** `./scripts/check-semver.sh --package <pkg>` with `SEMVER_PRUNE_LOCAL_CACHE=1 CARGO_INCREMENTAL=0`:
  - `paladin-ports`: 223 checks, 223 pass, 31 skip; `0 failure(s)`
  - `paladin-storage`: 223 checks, 223 pass, 31 skip; `0 failure(s)`
  - `paladin-ai-core`: 223 checks, 223 pass, 31 skip; `0 failure(s)`
- **clippy:** `cargo clippy -p paladin-ports -p paladin-storage -p paladin-ai-core --all-features --all-targets -- -D warnings` exit 0, no warnings.
- **fmt:** `cargo fmt --all -- --check` clean.
- **doc:** `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ports -p paladin-storage -p paladin-ai-core --no-deps` exit 0, 0 warnings.
- **Tests run:**
  - `paladin-storage --features sqlite --lib run::`: 84 passed
  - `paladin-storage --features postgres --lib run::postgres`: 40 passed, **run live with no `SKIP:`** (see Deviations item 6)
  - `paladin-ports`: 301 lib passed, 230 doctests passed (94 ignored, pre-existing)
  - `paladin-ai-core --lib principal::`: 10 passed; `permits_thread` doctest passed
  - `cargo check -p paladin-ai -p paladin-web --all-targets`: exit 0, which compiles every other in-tree implementor unchanged
  - Not run: the full workspace `cargo test` and `make test-all`; only the touched crates above.
- **Changelogs:** no line at or below `## [0.11.0]` changed in any of the three files (the only removed lines are the three Phase 53 bullet lines in `paladin-core` `[Unreleased]` that the plan told me to amend in place).

## Evidence for WINDOWS rows 86 and 87

Row 87 (a later Admin or schedule run must not change the owner):
- `owner_run_for_thread_ignores_later_admin_and_unattributed_runs` (`crates/paladin-storage/src/run/contract_tests.rs`), run as `run::in_memory::contract_suite::...`, `run::sqlite::tests::...`, `run::postgres::tests::...`
- `owner_run_for_thread_returns_the_earliest_attributed_run` (includes the equal-`submitted_at` tie-break by smaller `run_id` and a later other-tenant run)
- `owner_run_for_thread_falls_back_to_the_earliest_run_when_none_is_attributed` (asserts `RunReadScope::Tenant(_).permits_thread(Some(&owner))` is false: a schedule-only thread stays Admin-only)
- `owner_run_for_thread_is_none_for_a_thread_with_no_runs`

Row 86 (a corrupt or newer-schema non-owner row must not fail the lookup):
- `owner_run_for_thread_decodes_only_the_owner_row` (planted `schema_version = "v99-from-the-future"` both after the owner and as an earlier unattributed row); SQLite RED then GREEN recorded above
- `run::postgres::tests::owner_query_reads_exactly_one_row_attributed_first_oldest_first` (DB-free shape guard on `SELECT_OWNER_RUN_FOR_THREAD`)

Port crate:
- `output::run_repository_port::tests::default_impl_of_owner_run_for_thread_walks_every_page`
- `output::run_repository_port::tests::default_impl_of_owner_run_for_thread_falls_back_to_the_earliest_unattributed_run`
- trait-level doctest on `RunRepositoryPort` (calls `owner_run_for_thread` through the default)

Core:
- `platform::container::principal::tests::permits_thread_admits_all_and_only_the_latest_runs_tenant` (name kept) and the `permits_thread` doctest

Caveat for 57-11: rows 86 and 87 are not closed by this plan. The web gate and the submission guard still call the old latest-run lookup until 57-03 rewires them. Rows 85-87 stay `open` in the ledger (`windows status` open_count 15 at plan start).

## Files Created/Modified

- `crates/paladin-ports/src/output/run_repository_port.rs` - new defaulted `owner_run_for_thread`, `owner_order_key` helper, module doc line, doctest, two default-impl tests
- `crates/paladin-storage/src/run/contract_tests.rs` - `finish_run` helper and five `pub async fn` owner clauses
- `crates/paladin-storage/src/run/in_memory.rs` - override plus five thin tests
- `crates/paladin-storage/src/run/sqlite.rs` - `SELECT_OWNER_RUN_FOR_THREAD`, override, five thin tests
- `crates/paladin-storage/src/run/postgres.rs` - same constant and override with `$1`, five thin tests, one DB-free shape test
- `crates/paladin-core/src/platform/container/principal.rs` - `permits_thread(owner)`, rewritten rustdoc and doctest, doc comment on the kept test
- `crates/paladin-{ports,storage,core}/CHANGELOG.md` - `[Unreleased]` entries (ports gains the heading)

## Decisions Made

- Followed option-a as selected; no new decision beyond the plan's PD-02-1 to PD-02-4.
- The default body's key is a free function `owner_order_key` in the port module rather than a method on `Run`, to avoid adding a second public item.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Disk: `target/debug/incremental` removed before the first build**
- **Found during:** Task 2 start (872M free, 22G `target`)
- **Fix:** removed the 2.1G gitignored incremental cache up front (the plan's ENOSPC recovery, taken proactively) and built with `CARGO_INCREMENTAL=0`. No ENOSPC occurred; 2.8G+ stayed free throughout.

**2. [Rule 1 - Bug] Existing trait doctest moved `run.run_id`**
- **Found during:** Task 3 (extending the `InMemoryRuns` doctest)
- **Fix:** the pre-existing `Some(run.run_id)` assertion became `Some(run.run_id.clone())`; the example's `list` now filters by `query.thread_id` so the new `owner_run_for_thread` call is meaningful. Committed in `6bec007e9`.

### Plan assumptions that did not hold (recorded, not defects)

3. **`make api-surface` did not fail.** The plan expected a before-failure and an `api-surface-update`. The baseline covers only the root `paladin` crate, so the check stayed green and `.project/current-exports.txt` is unchanged (listed in `files_modified` but not touched). The patch-semver runs are the real gate.
4. **`check-semver.sh` rejects `--release-type`.** It pins `patch` internally; I ran it with `--package` only. Three earlier invocations with the flag exited 2 (`ERROR: unknown argument`) and are not counted; the results above are from the corrected run.
5. **Twelve implementors, not eleven.** `grep` finds 12 `impl RunRepositoryPort for` code blocks (13 matches including the doctest). All compile unchanged.
6. **Postgres ran live.** Docker's daemon is down here, so I started a throwaway PostgreSQL 16 from the installed binaries on port 5433 (role and database matching the default test URL), ran `run::postgres` (40 passed, 0 `SKIP:`), then stopped it and deleted its data directory. CI's `postgres-integration` job remains the authoritative run; its declared-count assertion counts the five new `#[tokio::test]` attributes added to `postgres.rs`.

### Additions beyond the plan

7. Extra tests: `default_impl_of_owner_run_for_thread_falls_back_to_the_earliest_unattributed_run` (port) and `owner_query_reads_exactly_one_row_attributed_first_oldest_first` (Postgres, DB-free). The Task 2 clause's PostgreSQL thin test landed in the Task 3 commit, since Task 2 scoped its thin tests to in-memory and SQLite.

**Total deviations:** 2 auto-fixed (Rule 3, Rule 1), 4 plan-assumption corrections, 1 addition. **Impact:** none on the delivered behavior or on any acceptance criterion.

## Issues Encountered

- First semver invocations used a malformed log path and an unsupported flag; re-run correctly (see item 4).
- The 57-02 plan's `<verify>` for Task 3 greps `fn owner_run_for_thread` in both SQL files: satisfied.

## Known Stubs

None.

## Threat Flags

None. No new endpoint, auth path, file access or schema change; the SQL is a bound single-row read on an existing table and columns. Threats T-57-05, T-57-06, T-57-07 and T-57-08 from the plan's register are mitigated by the clauses and gates above.

## User Setup Required

None.

## Next Phase Readiness

- 57-03 can wire the web gate and submission guard to `owner_run_for_thread` and pass its result to `permits_thread`; `thread_controller.rs` `MockRunRepository` (and `run_controller.rs` `MockRepository` if it needs it) should override the method there.
- 57-04 records the legacy mixed-tenant-thread change as a MIGRATION section 11.1 row and amends ADR-0063.
- 57-11 closes WINDOWS rows 86 and 87 after 57-03 lands, citing the test names above.

## Self-Check: PASSED

All nine modified files, the SUMMARY, and commits `a02d53140` and `6bec007e9` exist; both commits carry the required trailers; `.project/current-exports.txt` is untouched; the only added `unwrap()` calls are in a doctest and `#[cfg(test)]` code.
