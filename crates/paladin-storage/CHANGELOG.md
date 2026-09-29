# Changelog

All notable changes to `paladin-storage` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project follows lockstep workspace versioning.

## [Unreleased]

### Added

- `treasury` module: `InMemoryTreasuryLedger`, `SqliteTreasuryLedger` (`sqlite` feature) and
  `PostgresTreasuryLedger` (`postgres` feature) — three `TreasuryLedgerPort` adapters passing one
  shared, 21-clause `contract_tests.rs` suite unmodified, including the LEDGR-02 race clause (16
  concurrent reserves against a ceiling of 15) and the LEDGR-03 duplicate-settle idempotency clause
  on every adapter (LEDGR-01, LEDGR-02, LEDGR-03).
- `007_create_treasury_ledger_table` migrations (`migrations/{sqlite,postgres}`): a partial unique
  settlement index on `(run_id, superstep, attempt) WHERE kind = 'settle'`, a scope+window covering
  index, and a `model_breakdown` column folded in Rust so `spend`'s per-model view is identical
  across adapters (LEDGR-01, LEDGR-04).

- `008_add_run_attribution_columns` migrations (`migrations/{sqlite,postgres}`): nullable
  `runs.tenant_id`/`runs.api_key_id` columns plus the non-unique `idx_runs_tenant_submitted` index;
  `RUN_SCHEMA_VERSION` is unchanged and a pre-existing row reads back with no attribution
  (TENANT-02).
- `009_add_run_attribution_check` migration (`migrations/postgres` only): `CHECK ((tenant_id IS NULL)
  = (api_key_id IS NULL))` on `runs`, so the schema itself refuses a half-attributed row. SQLite
  has no counterpart (`ALTER TABLE ... ADD COLUMN` cannot add a cross-column CHECK) and relies on
  the read-time guard in `row_to_run` alone (phase 40 review WR-03).
- The in-memory, SQLite and PostgreSQL run repositories persist and read `Run.submitted_by`, reject
  a half-attributed row, and apply `RunQuery.scope` inside `list` (`WHERE tenant_id = ?`) so a
  tenant-scoped page and its cursor stay a correct keyset walk; the shared run contract suite gains
  the attribution and scoped-list clauses every adapter passes (TENANT-02, PLAT-07).

## [0.10.1] - 2026-09-20

Patch release carried by the workspace-wide version bump (0.10.0 -> 0.10.1). No source
change in this crate — see the root `CHANGELOG.md`'s `[0.10.1]` section for the two
release-pipeline defects this patch fixes.

## [0.10.0] - 2026-09-10

### Added
- `waypoint` module: `SqliteWaypointStore` and, behind the new `postgres` feature,
  `PostgresWaypointStore` — durable `WaypointPort` backends for the new `WarEngine`/`WarGraph`
  execution path, plus `WaypointRetentionService` (`max_age_days`, `max_waypoints_per_thread`;
  never prunes a thread's latest Waypoint or one with status `AwaitingInput`) (ENG-05, §9.4,
  §9.5).
- `run`, `run_queue`, `cron` modules: SQLite/Postgres run-store repositories, a Redis-backed
  `run_queue` adapter (`redis-queue` feature) and a `tokio-cron-scheduler`-backed `SchedulerPort`
  adapter (`scheduler` feature), landing the `RunStoreConfig`/`RunQueueConfig`/`RunWorkerConfig`
  platform subsystems (PLAT-01…PLAT-06, §9.5).
- `assistant` module: SQLite and Postgres assistant repositories plus an in-memory contract
  suite (PLAT-03).
- `run_schedule` module: run-schedule types, port, migrations, and a three-adapter contract
  suite (`ScheduleServiceOptions`, §9.5).
- `webhook` module: webhook delivery types, port, migrations, three adapters, an SSRF guard
  covering both write-time and send-time checks, HMAC (`X-Paladin-Signature`) signing over the
  exact stored byte buffer, and a no-redirect HTTP client (`WebhooksConfig`, §9.5; see
  `.github/instructions/security.instructions.md` for the SSRF/redirect threat model).
- `run_trace` module: `RunTracePort`, in-memory and SQLite adapters, and a Postgres adapter
  (Tier 2) joined into the retention service (Phase 28 OBS).
- `node_cache` module: a Redis-backed `NodeCachePort` adapter (`redis-cache` feature, shares the
  existing `redis` dependency edge with `redis-queue` — no new crate name) (FT-FR-18…20, D-27).

See root `MIGRATION.md` §9.4 (persistence & schema migrations) and §9.5 (configuration &
environment) for the full list of new tables, migrations, and config surfaces landed above.

## [0.9.0] - 2026-09-01

## [0.8.1-rc.5] - 2026-08-31

## [0.8.1-rc.4] - 2026-08-29

### Added
- Crate-level release artifacts for Epic 4 API stabilization.
- Feature-flag release notes tracking for storage adapters (`sqlite`, `mysql`).

### Changed
- Storage API stability documentation aligned with crate-tier stability expectations.

### Fixed
- Crate metadata and README linkage validated for crates.io release preparation.
