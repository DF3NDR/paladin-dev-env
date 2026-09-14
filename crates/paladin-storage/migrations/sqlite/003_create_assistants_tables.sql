-- Migration: Create Assistants Tables (SQLite)
-- Purpose: Persist append-only immutable assistant versions (PLAT-04, D-28, D-29, D-30)
-- Version: 003
-- Date: 2026-09-08
--
-- `assistants` tracks one row per assistant: its identity and its `latest`
-- published version number. `assistant_versions` is the append-only history
-- itself, keyed by the PRIMARY KEY `(assistant_id, version)` -- this IS the
-- D-29 immutability invariant: no adapter in this tree ever executes an
-- UPDATE against this table's `kind`/`body` columns, and no code path
-- computes a duplicate `(assistant_id, version)` pair without hitting this
-- constraint. There is deliberately no `update_version` method anywhere in
-- `AssistantRepositoryPort` and no `PUT` route above it -- immutability
-- holds because the code to violate it does not exist, not because this
-- constraint is the only thing preventing it.
--
-- `kind`/`body` mirror D-28's opaque tagged-envelope shape: `kind` is
-- `'agent'` or `'workflow'` (`AssistantKind`'s snake_case wire form), `body`
-- is TEXT holding serialized JSON (mirroring `runs.input`'s TEXT-JSON
-- choice from `002_create_runs_table.sql`: debuggability, and the sqlx
-- `json` feature is already enabled workspace-wide).
--
-- `assistants.latest` is what `RunRepositoryPort::insert_with_latest`
-- (D-30) reads inside the run-insert statement to freeze a run's
-- `assistant.version` at submit time -- see
-- `crates/paladin-storage/src/run/sqlite.rs`'s `INSERT ... SELECT latest
-- FROM assistants` statement, added by this same plan.

CREATE TABLE IF NOT EXISTS assistants (
    assistant_id   TEXT PRIMARY KEY NOT NULL,
    latest         INTEGER NOT NULL,
    source         TEXT NOT NULL DEFAULT 'stored',
    created_at     TEXT NOT NULL,
    deleted_at     TEXT NULL,
    schema_version TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS assistant_versions (
    assistant_id   TEXT NOT NULL,
    version        INTEGER NOT NULL,
    kind           TEXT NOT NULL,
    body           TEXT NOT NULL,
    created_at     TEXT NOT NULL,
    created_by     TEXT NULL,
    note           TEXT NULL,
    schema_version TEXT NOT NULL,
    PRIMARY KEY (assistant_id, version)
);

-- Serves `AssistantRepositoryPort::list_versions`'s documented ascending
-- `version` ordering within one assistant -- the primary key already
-- covers this, but an explicit index documents the query shape and keeps
-- the ORDER BY off a table scan on a backend that does not treat the PK as
-- automatically the best access path for a ranged scan.
CREATE INDEX IF NOT EXISTS idx_assistant_versions_assistant_version
ON assistant_versions(assistant_id, version);
