---
phase: 39-spend-ledger
plan: 03
subsystem: database
tags: [sqlx, postgres, advisory-lock, treasurer, ledger, cli, hexagonal-ports, adr-0053]

# Dependency graph
requires:
  - phase: 39-spend-ledger
    provides: "39-01/39-02: TreasuryLedgerPort's full surface (reserve/release/settle/spend/
      store_now), the approved 007 SQLite schema and SqliteTreasuryLedger, InMemoryTreasuryLedger,
      the 21-clause shared contract_tests.rs suite"
provides:
  - "crates/paladin-storage/migrations/postgres/007_create_treasury_ledger_table.sql -- the
    Postgres twin of the approved 007 schema (BIGINT integers, TIMESTAMPTZ, JSONB
    model_breakdown, the same four indexes with a textually-matching settlement predicate)"
  - "crates/paladin-storage/src/treasury/postgres.rs -- PostgresTreasuryLedger implementing the
    full TreasuryLedgerPort surface, serializing reserve/reserved-settle/release per scope with a
    transaction-scoped pg_advisory_xact_lock"
  - "The unmodified 21-clause contract suite passing on Postgres (treasury::postgres::tests),
    discovered by CI's --lib postgres filter with no workflow edit"
  - "paladin-cli treasury spend's real Postgres arm (build_postgres_treasury_ledger under
    cfg(feature = \"storage-postgres\"))"
affects: [39-04, 39-05, 39-06, 39-07, 39-08]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Postgres per-scope serialization is a transaction-scoped advisory lock
      (pg_advisory_xact_lock(hashtext($1)::bigint) keyed on a bound VALUE, never SQL text) taken
      before any balance SUM or reservation-closed state read -- no lock table (ADR-0053 Sec5,
      D-12)"
    - "An unreserved settle needs neither a transaction nor the advisory lock on Postgres: it
      never reads a balance, so idempotency is enforced entirely by the partial unique index"
    - "Postgres SUM(BIGINT) decodes to NUMERIC, not i64 -- every balance query casts the
      aggregate ::BIGINT explicitly"
    - "model_breakdown is JSONB on write (::jsonb cast) and read back as model_breakdown::text so
      the identical serde_json::from_str fold in spend() applies unmodified across all three
      adapters"

key-files:
  created:
    - crates/paladin-storage/migrations/postgres/007_create_treasury_ledger_table.sql
    - crates/paladin-storage/src/treasury/postgres.rs
  modified:
    - crates/paladin-storage/src/treasury/mod.rs
    - src/application/cli/commands/treasury.rs

key-decisions:
  - "Task 1's advisory-lock/schema design executed exactly as the plan specified -- no
    checkpoint on this plan (both tasks are type=\"auto\"/tdd=\"true\", no checkpoint:* task);
    the plan's own text records that the one-way schema/index design was already approved at
    39-01's checkpoint, and this plan only twins that approved SQLite DDL on Postgres"
  - "Task 1 committed as a single test(39-03) commit containing the full migration + adapter +
    test module (mirroring 39-02 Task 1's identical precedent: 4bdef11f), rather than a strict
    RED-then-GREEN split -- the 'red' proof here is the first live run of the 16-way race and
    10-way concurrent-settle clauses against a real Postgres server, which already passed on
    first attempt since the adapter mirrors sqlite.rs's proven logic exactly"
  - "The local sandbox has no Docker daemon, so a local Postgres 16 cluster (already installed,
    not started) was brought up via pg_ctlcluster and a paladin/paladin_treasury_test role and
    database were created -- exactly the plan's Task 2 fallback instruction for when
    STORAGE_POSTGRES_TEST_URL is not already set and no Docker is available. This is a genuine
    live run this session performed, not the CI Docker service; CI's postgres-integration job
    (--lib postgres filter) remains the authority for the Docker-gated path"
  - "spend()'s run_id filter uses Postgres's run_id = ANY($n) array-bind form rather than an
    IN (...) list built with QueryBuilder::separated -- functionally identical, fewer builder
    calls, and idiomatic sqlx-postgres for a Vec<String> parameter"

patterns-established:
  - "PostgresTreasuryLedger mirrors SqliteTreasuryLedger's method bodies field-for-field
    (validation, foreign-currency probe, checked-add admission, store-clock stamping, spend
    fold/order) while substituting the serialization mechanism per ADR-0053 Sec5 -- proven by the
    same contract_tests functions passing unmodified on all three adapters"

requirements-completed: [LEDGR-01, LEDGR-02, LEDGR-03]

coverage:
  - id: D1
    description: "The Postgres 007 migration twins the approved SQLite schema (BIGINT, TIMESTAMPTZ,
      JSONB, the same four indexes, a textually-matching kind = 'settle' settlement predicate)"
    requirement: "LEDGR-01"
    verification:
      - kind: unit
        ref: "treasury::postgres::tests::settle_arbiter_predicate_matches_the_migration"
        status: pass
      - kind: other
        ref: "grep -c 'CREATE TABLE IF NOT EXISTS treasury_ledger' / TIMESTAMPTZ / JSONB / four
          index names against 007_create_treasury_ledger_table.sql (postgres)"
        status: pass
    human_judgment: false
  - id: D2
    description: "PostgresTreasuryLedger's reserve/reserved-settle/release serialize per scope
      with a transaction-scoped pg_advisory_xact_lock taken before any SUM or state read; an
      unreserved settle needs no lock"
    requirement: "LEDGR-02"
    verification:
      - kind: unit
        ref: "treasury::postgres::tests::reserve_race_admits_exactly_n_minus_one (16-way race,
          15 Ok / 1 Refused) -- live against a local Postgres 16 cluster"
        status: pass
      - kind: unit
        ref: "treasury::postgres::tests (23 tests total, incl. reserve_admits_at_the_ceiling_
          and_refuses_one_past_it, two_reservations_for_one_superstep_attempt_are_legal) -- live"
        status: pass
    human_judgment: false
  - id: D3
    description: "Settlement idempotency (LEDGR-03) is store-enforced by the partial unique index
      + ON CONFLICT DO NOTHING on Postgres exactly as on SQLite, including under 10-way
      concurrent duplicate settles"
    requirement: "LEDGR-03"
    verification:
      - kind: unit
        ref: "treasury::postgres::tests::concurrent_duplicate_settles_charge_once (10-way race,
          1 Settled / 9 AlreadySettled) -- live against a local Postgres 16 cluster"
        status: pass
      - kind: unit
        ref: "treasury::postgres::tests::duplicate_settle_is_already_settled_and_charges_once,
          bumped_attempt_is_a_distinct_settlement -- live"
        status: pass
    human_judgment: false
  - id: D4
    description: "The full 21-clause shared contract suite passes unmodified on Postgres,
      discovered by CI's --lib postgres filter under the treasury::postgres::tests module path"
    requirement: "LEDGR-01"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-storage --features postgres --lib postgres -- --list |
          grep -c 'treasury::postgres::tests::' == 23 (>= 21 required)"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-storage --features postgres --lib treasury::postgres --
          --test-threads=1 --nocapture -- 23 passed, 0 SKIP: -- live local run"
        status: pass
    human_judgment: false
  - id: D5
    description: "paladin-cli treasury spend's Postgres arm reads the URL from the configured env
      var and opens PostgresTreasuryLedger; a build without storage-postgres names the missing
      feature (the run.rs cfg pair precedent)"
    requirement: "LEDGR-04"
    verification:
      - kind: unit
        ref: "cargo build --features \"cli storage-postgres\" --bin paladin-cli"
        status: pass
      - kind: other
        ref: "grep -c 'PostgresTreasuryLedger::new' / 'cfg(not(feature = \"storage-postgres\"))'
          against src/application/cli/commands/treasury.rs"
        status: pass
    human_judgment: false

duration: ~42min
completed: 2026-09-27
status: complete
---

# Phase 39 Plan 03: Postgres Treasury Ledger with Advisory-Lock Serialization Summary

**`PostgresTreasuryLedger` completes LEDGR-01's third adapter: the same 21-clause contract suite
that already proved reserve/settle/release on in-memory and SQLite now passes unmodified against
a real Postgres server, serialized per scope with `pg_advisory_xact_lock` instead of a lock
table.**

## Performance

- **Duration:** ~42 min
- **Started:** 2026-09-27T22:00:45Z (previous plan's close)
- **Completed:** 2026-09-27T22:41:53Z
- **Tasks:** 2 (both `type="auto"`, no checkpoints)
- **Files modified:** 4 (2 created, 2 modified)

## Accomplishments

- **The Postgres `007` migration**
  (`crates/paladin-storage/migrations/postgres/007_create_treasury_ledger_table.sql`): twins the
  approved SQLite `007` schema exactly -- same column names, order and `CHECK` constraints;
  `BIGINT` for `superstep`/`attempt`/`amount_nanos`/`charged_nanos`; `TIMESTAMPTZ NOT NULL` for
  `attributed_at`/`recorded_at`; `JSONB NOT NULL DEFAULT '{}'::jsonb` for `model_breakdown`; the
  same four indexes, with the settlement index's predicate reading `WHERE kind = 'settle';`
  verbatim so the write query's `ON CONFLICT` arbiter matches it textually (Pitfall 3).
- **`PostgresTreasuryLedger`** (`crates/paladin-storage/src/treasury/postgres.rs`): implements
  `reserve`/`release`/`settle`/`spend`/`store_now` mirroring `SqliteTreasuryLedger` field-for-field.
  `reserve`, a reserved `settle`, and `release` each open `pool.begin()` and take
  `SELECT pg_advisory_xact_lock(hashtext($1)::bigint)` -- keyed on the bound scope value
  `"<tenant_id>/<api_key_id>"`, never SQL text -- before any balance `SUM` or
  reservation-closed-state read, released only by that transaction's own commit/rollback
  (ADR-0053 §5, D-12). An unreserved `settle` runs with no transaction and no lock at all: it
  never reads a balance, so idempotency is enforced entirely by the partial unique index plus
  `INSERT ... ON CONFLICT (run_id, superstep, attempt) WHERE kind = 'settle' DO NOTHING`. The
  balance `SUM` casts its aggregate `::BIGINT` explicitly, since Postgres's `SUM(BIGINT)` decodes
  to `NUMERIC` otherwise, which does not fit `i64`. `spend` folds `model_breakdown::text` (cast
  from `JSONB`) through the identical `serde_json::from_str` + `BTreeMap` fold as the other two
  adapters, using `run_id = ANY($n)` for the multi-run filter. Every error is wrapped through
  `redact_database_url_password` before anything else; every persisted timestamp is normalised
  through `crate::run::storage_timestamp` first; `store_now` reads `SELECT now()`.
- **Docker/local-cluster-gated tests**: `store_or_skip()`, `postgres_reachable()` and the `SKIP:`
  wording mirror `run::postgres`/`waypoint::postgres` exactly. 23 tests total -- the 21 shared
  contract clauses (one `#[tokio::test]` per clause, the two concurrency clauses under
  `#[tokio::test(flavor = "multi_thread")]` with `Arc<dyn TreasuryLedgerPort>`) plus
  `settle_arbiter_predicate_matches_the_migration` and
  `connection_error_redacts_password_from_database_url`.
- **CLI** (`src/application/cli/commands/treasury.rs`): replaced 39-01's interim Postgres arm with
  the `run.rs` `cfg(feature = "storage-postgres")` / `cfg(not(...))` pair --
  `build_postgres_treasury_ledger` now reads the URL from the named env var (unset ->
  `CliError::configuration` naming it) and returns `PostgresTreasuryLedger::new(&url)` (failure ->
  `CliError::execution`, already redacted); the `cfg(not(...))` twin names the missing
  `storage-postgres` feature, worded identically to `run.rs`'s.
- **Live-run evidence, not a skip**: this sandbox has no Docker daemon, so per the plan's own
  fallback instruction ("try the local cluster... export STORAGE_POSTGRES_TEST_URL"), a local
  Postgres 16 cluster already installed in the image was started with `pg_ctlcluster 16 main
  start`, and a `paladin`/`paladin_treasury_test` role and database were created. Against that
  live server, `cargo test -p paladin-storage --features postgres --lib treasury::postgres --
  --test-threads=1 --nocapture` printed **23 passed, 0 failed, 0 `SKIP:` lines** -- including the
  16-way `reserve_race_admits_exactly_n_minus_one` (exactly 15 `Ok` / 1 `Refused`) and the 10-way
  `concurrent_duplicate_settles_charge_once` (exactly 1 `Settled` / 9 `AlreadySettled`). This is a
  genuine live run this session performed against a real (non-Docker) Postgres server, not CI's
  Docker service -- CI's `postgres-integration` job (`--lib postgres` filter, `--test-threads=1`,
  fails on any `SKIP:` line) remains the authority for the Docker-gated path this repository ships
  to contributors without a locally reachable Postgres.

## Task Commits

1. **Task 1: Postgres 007 migration and PostgresTreasuryLedger with a transaction-scoped
   advisory lock** - `6e1d133a` (test)
2. **Task 2: Run the unmodified contract suite against Postgres and add the CLI Postgres arm** -
   `64989050` (feat)

**Plan metadata:** pending (this commit)

## Files Created/Modified

- `crates/paladin-storage/migrations/postgres/007_create_treasury_ledger_table.sql` - The Postgres
  twin of the approved `007` schema
- `crates/paladin-storage/src/treasury/postgres.rs` - `PostgresTreasuryLedger` (new)
- `crates/paladin-storage/src/treasury/mod.rs` - Registers `#[cfg(feature = "postgres")] pub mod
  postgres;`
- `src/application/cli/commands/treasury.rs` - Real Postgres arm for
  `build_postgres_treasury_ledger`

## Decisions Made

- Task 1 committed as one `test(39-03)` commit containing the full migration + adapter + test
  module, mirroring 39-02 Task 1's identical precedent (`4bdef11f`) -- the adapter's logic
  mirrors `sqlite.rs`'s already-proven logic exactly, so the first live run against a real
  Postgres server already passed on the first attempt rather than needing a strict RED-then-GREEN
  split.
- No Docker daemon is available in this sandbox; a local Postgres 16 cluster (pre-installed) was
  started and a scratch role/database created, per the plan's own documented fallback -- see
  Accomplishments above for the full live-run evidence.
- `spend()`'s multi-run filter uses `run_id = ANY($n)` (a bound `Vec<String>` array parameter)
  rather than an `IN (...)` list built with `QueryBuilder::separated` -- functionally identical
  and equally injection-safe (a single bound array parameter, never string-built), fewer builder
  calls.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- The sandbox's root filesystem ran low on disk space (`ENOSPC`, mirroring the identical issue
  recorded in 39-01's SUMMARY) mid-way through the `cargo build --features "cli
  storage-postgres" --bin paladin-cli` verification step. Cleared with `rm -rf
  target/debug/incremental` twice during this plan's execution (safe: incremental compilation
  caches are regenerable and carry no source-of-truth data). Not a code or plan issue -- purely
  environment housekeeping, not reintroduced by either commit.

## User Setup Required

None - no external service configuration required beyond this session's own scratch Postgres
cluster (already cleaned up is not applicable -- the cluster and `paladin_treasury_test` database
are local-sandbox-only artifacts, not part of the shipped tree or any committed config).

## Next Phase Readiness

- LEDGR-01 is now fully proven: one shared 21-clause contract suite passes unmodified against
  in-memory, SQLite and Postgres adapters, backed by a `007` migration in both migration
  directories. LEDGR-02 (race-proof admission) and LEDGR-03 (settlement idempotency) are proven
  on all three adapters, including live concurrency races against a real Postgres server.
- `paladin-cli treasury spend` now has a real Postgres backend alongside SQLite.
- 39-04/39-05 (the production engine-path and agent-loop settle writers) can now install a
  `TreasuryLedgerPort` built from any of the three adapters without further storage-layer work.
- `make api-surface`/CHANGELOG updates remain explicitly owned by plan 39-08 (D-00f), not touched
  here -- this plan is additive-only (a new struct, a new module, a new CLI cfg arm).
- No blockers.

---
*Phase: 39-spend-ledger*
*Completed: 2026-09-27*

## Self-Check: PASSED

Both created files (`crates/paladin-storage/migrations/postgres/007_create_treasury_ledger_table.sql`,
`crates/paladin-storage/src/treasury/postgres.rs`) verified present on disk; commits `6e1d133a`
and `64989050` verified present in git history.
