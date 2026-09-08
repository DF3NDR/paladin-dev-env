---
phase: 27-platform-api
plan: 20
subsystem: database
tags: [postgres, sqlx, chrono, timestamps, run-repository, tdd]

# Dependency graph
requires:
  - phase: 27-platform-api
    provides: "PostgresRunRepository, the shared run::contract_tests suite, and the three failing timestamp round-trip clauses identified in 27-VERIFICATION.md gap 2"
provides:
  - "storage_timestamp: a documented, tested microsecond-truncation contract applied to every Postgres run-store timestamp bind"
  - "contract_timestamp: a shared fixture-clock helper that stamps every contract_tests.rs fixture at storage resolution, keeping every assert_eq! exact instead of tolerance-based"
  - "A dedicated Postgres-only test proving truncation (not rounding) directly against a live TIMESTAMPTZ column"
affects: [27-platform-api, 27-25-verification]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Normalize-on-write timestamp precision contract: a single pub(crate) fn (storage_timestamp) is the sole place that decides persisted resolution, applied at every bind site rather than left to the driver/server"
    - "Fixture-clock helper (contract_timestamp) stamping shared contract-suite fixtures at the narrowest supported backend's resolution, so cross-backend assert_eq! stays exact rather than being weakened to a tolerance"

key-files:
  created: []
  modified:
    - "crates/paladin-storage/src/run/mod.rs - storage_timestamp + STORAGE_TIMESTAMP_SUBSEC_DIGITS with unit tests"
    - "crates/paladin-storage/src/run/postgres.rs - every DateTime<Utc> bind (INSERT_RUN, INSERT_RUN_WITH_LATEST, update_status) routed through storage_timestamp; new postgres_run_timestamps_round_trip_at_microsecond_precision test"
    - "crates/paladin-storage/src/run/contract_tests.rs - contract_timestamp() replaces all 36 Utc::now() call sites so every fixture is stamped at storage resolution"

key-decisions:
  - "Truncation toward zero via chrono::SubsecRound::trunc_subsecs, never round_subsecs -- rounding forward in time would make the persisted value depend on where in its sub-microsecond range the original value fell"
  - "Option<DateTime<Utc>> binds (started_at, finished_at) are normalized via .map(storage_timestamp) (a function reference, not a closure) to keep cargo clippy -D warnings clean -- clippy::redundant_closure fires on the equivalent |t| storage_timestamp(t) form"
  - "update_status normalizes `at` once at function entry (shadowed) rather than at each of the two push_bind call sites, since both started_at and finished_at may bind the identical instant on a single terminal-and-just-started transition"

requirements-completed: [PLAT-01]

coverage:
  - id: D1
    description: "storage_timestamp truncates toward zero at microsecond resolution (999ns past a boundary lands on the boundary, never the next one) and is the identity for values already at that resolution"
    requirement: "PLAT-01"
    verification:
      - kind: unit
        ref: "crates/paladin-storage/src/run/mod.rs#run::tests::storage_timestamp_truncates_sub_microsecond_digits_toward_zero"
        status: pass
      - kind: unit
        ref: "crates/paladin-storage/src/run/mod.rs#run::tests::storage_timestamp_is_identity_at_microsecond_resolution"
        status: pass
    human_judgment: false
  - id: D2
    description: "insert then get through PostgresRunRepository round-trips submitted_at/started_at/finished_at under exact assert_eq! (contract_tests.rs's three previously-failing clauses), with fixtures stamped at storage resolution by contract_timestamp"
    requirement: "PLAT-01"
    verification:
      - kind: unit
        ref: "crates/paladin-storage/src/run/sqlite.rs (same shared contract_tests functions the Postgres suite runs) -- cargo test -p paladin-storage --features sqlite --lib run:: (46 passed)"
        status: pass
      - kind: integration
        ref: "crates/paladin-storage/src/run/postgres.rs#run::postgres::tests::insert_then_get_round_trips_every_field, update_status_queued_to_running_sets_started_at_then_stale_cas_fails, update_status_running_to_completed_sets_finished_at_then_terminal_is_absorbing -- self-skips locally (D-51), evidence is the CI postgres-integration job"
        status: unknown
    human_judgment: true
    rationale: "Docker is unavailable in this devcontainer; the Postgres Tier-2 suite self-skips locally and prints SKIP: postgres-test not reachable. A local pass is not evidence per this plan's own prohibition (D-51) -- the CI postgres-integration job (asserted by plan 27-25) is the actual evidence this clause needs."
  - id: D3
    description: "A dedicated Postgres-only test asserts truncation (not rounding): a sub-microsecond submitted_at/started_at round-trips equal to storage_timestamp(original) and NOT equal to the original itself"
    requirement: "PLAT-01"
    verification:
      - kind: integration
        ref: "crates/paladin-storage/src/run/postgres.rs#run::postgres::tests::postgres_run_timestamps_round_trip_at_microsecond_precision -- self-skips locally (D-51), evidence is the CI postgres-integration job"
        status: unknown
    human_judgment: true
    rationale: "Same D-51 local-Docker gap as D2 -- the test compiles, self-skips cleanly with a named SKIP: reason, and is proven only in the CI postgres-integration job."
  - id: D4
    description: "PLAT-01 adjacency/ordering probes: the keyset pagination clause with two fixtures sharing a submitted_at still separates them with no overlap/gap, tiebreak on descending run_id, unchanged by the precision normalization"
    requirement: "PLAT-01"
    verification:
      - kind: unit
        ref: "crates/paladin-storage/src/run/contract_tests.rs#list_paginates_by_submitted_at_and_run_id_with_no_overlap_or_gap -- cargo test -p paladin-storage --features sqlite --lib list_paginates_by_submitted_at_and_run_id (2 passed)"
        status: pass
    human_judgment: false

# Metrics
duration: 35min
completed: 2026-09-08
status: complete
---

# Phase 27 Plan 20: Postgres Run-Repository Timestamp Precision Summary

**Documented, tested microsecond-truncation contract (`storage_timestamp`) applied to every Postgres run-store timestamp bind, closing verification gap 2's three failing round-trip assertions without weakening any `assert_eq!`.**

## Performance

- **Duration:** ~35 min
- **Completed:** 2026-09-08T13:22:53Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- Added `storage_timestamp` (truncate-toward-zero at microsecond resolution via `chrono::SubsecRound::trunc_subsecs`) and `STORAGE_TIMESTAMP_SUBSEC_DIGITS` to `run/mod.rs`, with two unit tests proving truncation-not-rounding and identity-at-resolution behavior.
- Routed every `DateTime<Utc>` bind in `PostgresRunRepository` through `storage_timestamp`: the three timestamp columns in `INSERT_RUN`, the same three in `INSERT_RUN_WITH_LATEST`, and both `started_at`/`finished_at` pushes in `update_status`'s `QueryBuilder` (normalized once via a shadowed `at`).
- Added `contract_timestamp()` to `contract_tests.rs` and replaced all 36 `Utc::now()` call sites (fixtures, `at` values, and the `ParleyResponse` fixture's `responded_at`) so every clause stamps its inputs at the narrowest resolution any supported backend persists — keeping every `assert_eq!` in the shared suite exact rather than needing a tolerance-based comparison.
- Added a dedicated Postgres-only test, `postgres_run_timestamps_round_trip_at_microsecond_precision`, that deliberately builds a fixture with sub-microsecond digits (bypassing `contract_timestamp`'s own normalization) and asserts the round-tripped value equals `storage_timestamp(original)` and is NOT equal to the raw original — proving truncation, not rounding, directly against Postgres.
- Extended both `postgres.rs`'s module doc comment and the stale "no extra non-contract `#[tokio::test]`s" comment to document the new test and why it is the one sanctioned exception to that convention.

## Task Commits

Each task was committed atomically:

1. **Task 1: A run inserted with nanosecond timestamps reads back equal — normalise on write, stamp fixtures at storage resolution** - `1b91203c` (feat)
2. **Task 2: A dedicated Postgres round-trip precision test pins the documented contract** - `2c6b0911` (test)

_No plan-metadata commit required outside this SUMMARY per the worktree protocol — the wave orchestrator handles STATE.md/ROADMAP.md centrally after merge._

## Files Created/Modified
- `crates/paladin-storage/src/run/mod.rs` - `storage_timestamp` + `STORAGE_TIMESTAMP_SUBSEC_DIGITS`, rustdoc'd precision contract, 2 unit tests
- `crates/paladin-storage/src/run/postgres.rs` - all timestamp binds normalized, module docs extended, new dedicated precision test
- `crates/paladin-storage/src/run/contract_tests.rs` - `contract_timestamp()` fixture helper; all 36 `Utc::now()` sites replaced

## Decisions Made
- Truncation toward zero (`trunc_subsecs`), never `round_subsecs` — matches the plan's explicit interface note and avoids a persisted value depending on exactly where in its sub-microsecond range the original fell.
- `Option<DateTime<Utc>>` binds use `.map(storage_timestamp)` (a bare function reference) rather than `.map(|t| storage_timestamp(t))` — the closure form is caught by `clippy::redundant_closure` under `-D warnings`, which CLAUDE.md mandates stays clean. Behaviorally identical; every one of the 8 timestamp bind sites (3 in `INSERT_RUN`, 3 in `INSERT_RUN_WITH_LATEST`, 2 in `update_status`) is still routed through `storage_timestamp`.
- `update_status`'s `at` parameter is normalized once via a shadowed `let at = storage_timestamp(at);` at function entry rather than at each `push_bind` call, since a single terminal-and-just-started transition binds the identical normalized instant to both `started_at` and `finished_at`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Correctness] `grep -c 'storage_timestamp('` on `postgres.rs` reads 3, not the plan's suggested "at least 8"**
- **Found during:** Task 1 (routing Postgres timestamp binds through `storage_timestamp`)
- **Issue:** The plan's acceptance-criteria grep assumed every one of the 8 timestamp-normalizing call sites would contain the literal substring `storage_timestamp(` (i.e., an explicit call at each site). Writing the two `Option<DateTime<Utc>>` binds that way (`.map(|t| storage_timestamp(t))`) is a manual eta-expansion that `clippy::redundant_closure` flags under `-D warnings` — a hard CLAUDE.md gate ("Coverage floor... `cargo clippy -- -D warnings`... treat as errors during CI").
- **Fix:** Used `.map(storage_timestamp)` (function reference, no closure) for the two `Option` sites in each of `INSERT_RUN` and `INSERT_RUN_WITH_LATEST` (4 sites total), and kept explicit `storage_timestamp(at)` calls for the 3 non-optional sites (`INSERT_RUN`/`INSERT_RUN_WITH_LATEST` `submitted_at`, plus `update_status`'s single normalized `at`). All 8 original bind sites are still routed through `storage_timestamp` — the literal grep count is lower only because 4 of the 8 use idiomatic function-reference syntax instead of an explicit call expression.
- **Files modified:** `crates/paladin-storage/src/run/postgres.rs`
- **Verification:** `cargo clippy -p paladin-storage --features sqlite,postgres --all-targets -- -D warnings` passes with zero warnings; all 8 sites confirmed by direct code inspection (`storage_timestamp` name appears at every one, 3 as `storage_timestamp(x)` and 5 as `.map(storage_timestamp)` / `let at = storage_timestamp(at);`).
- **Committed in:** `1b91203c` (Task 1 commit)

**2. [Rule 1 - Correctness] `grep -c 'Utc::now()'` on `contract_tests.rs` initially read 3 (doc comment prose), not the required "at most 1"**
- **Found during:** Task 1 (adding `contract_timestamp`'s rustdoc)
- **Issue:** The first draft of `contract_timestamp`'s doc comment quoted the literal text `Utc::now()` twice while explaining why the function exists, tripping the plan's own acceptance-criteria grep (which counts matching lines file-wide, not just code).
- **Fix:** Reworded the doc comment to describe "reading the system clock directly" instead of quoting the literal `Utc::now()` token, leaving exactly one real occurrence (the function's own body).
- **Files modified:** `crates/paladin-storage/src/run/contract_tests.rs`
- **Verification:** `grep -c 'Utc::now()' crates/paladin-storage/src/run/contract_tests.rs` returns `1`.
- **Committed in:** `1b91203c` (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 1 — correctness of the literal acceptance-criteria greps vs. the actual, clippy-clean, behaviorally-correct implementation).
**Impact on plan:** No scope creep. Both deviations are grep-literal mismatches against a fully compliant implementation, not gaps in the underlying behavior the plan's `must_haves.truths` describe.

## Issues Encountered
None beyond the two documented deviations above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- `storage_timestamp` and `contract_timestamp` are in place and exercised by 46 passing SQLite/in-memory tests plus 19 self-skipping (locally) Postgres tests, all compiling clean under `cargo check --workspace --all-targets --all-features`, `cargo fmt --all -- --check`, and `cargo clippy -p paladin-storage --features sqlite,postgres --all-targets -- -D warnings`.
- **Not locally verifiable (D-51):** the three previously-failing Postgres clauses (`insert_then_get_round_trips_every_field`, `update_status_queued_to_running_sets_started_at_then_stale_cas_fails`, `update_status_running_to_completed_sets_finished_at_then_terminal_is_absorbing`) and the new `postgres_run_timestamps_round_trip_at_microsecond_precision` test all self-skip in this devcontainer (no Docker/Postgres reachable) and print their `SKIP:` reason. Plan 27-25's checkpoint against the CI `postgres-integration` job is the actual evidence this plan's `<verification>` section requires.
- No blockers for the remaining gap-closure plans (27-21 through 27-25).

---
*Phase: 27-platform-api*
*Completed: 2026-09-08*
