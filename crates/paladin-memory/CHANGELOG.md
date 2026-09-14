# Changelog

All notable changes to `paladin-memory` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project follows lockstep workspace versioning.

## [Unreleased]

## [0.10.0] - 2026-09-10

### Added
- `TokenCounterPort` implementations: `HeuristicTokenCounter` (no new dependency) and
  `TiktokenCounter` (rides the pre-existing `content-processing` feature) (RT-03, RT-FR-10).
- `VaultPort` implementations: `InMemoryVault` (passes the nine-case `VaultPort` contract
  suite), `SqliteVault` (persistent backing, `sqlite` feature), and `SemanticVault` (composes
  `VaultPort` + `SanctumPort` + `EmbeddingPort`, ungated — it holds only trait objects) (RT-04,
  RT-FR-13, RT-FR-14, RT-FR-15, RT-FR-16, D-18…D-24).
- New table `vault_records` (`ns`, `key`, `value`, `created_at`, `updated_at`, primary key
  `(ns, key)`) via migration `003_create_vault_tables.sql`, bounded by the Vault's own
  `max_value_bytes` cap (default 64 KiB per value); no automatic pruning (RT-04, D-18, D-23).

### Changed
- The Garrison/Vault SQLite migrator was consolidated into one shared embedded `MIGRATOR`
  (`crates/paladin-memory/src/migrations.rs`) covering all three migration files, so
  `SqliteGarrison` and `SqliteVault` share a single `_sqlx_migrations` numbering sequence
  (harmless, documented — a database that only ever constructed one of the two gains an empty
  table for the other via `IF NOT EXISTS`, D-23).

See root `MIGRATION.md` §9.4 for migration details and storage-growth guidance.

## [0.9.0] - 2026-09-01

## [0.8.1-rc.5] - 2026-08-31

## [0.8.1-rc.4] - 2026-08-29

### Added
- Crate-level release artifacts for Epic 4 API stabilization.
- Changelog tracking for Garrison and Sanctum adapter evolution.

### Changed
- Memory API stability documentation aligned with crate-tier stability expectations.

### Fixed
- Crate metadata and README linkage validated for crates.io release preparation.
