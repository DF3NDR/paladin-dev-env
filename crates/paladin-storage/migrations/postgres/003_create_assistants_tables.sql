-- Migration: Create Assistants Tables (PostgreSQL)
-- Purpose: Persist append-only immutable assistant versions (PLAT-04, D-28, D-29, D-30)
-- Version: 003
-- Date: 2026-09-08
--
-- Same logical schema as the SQLite sibling migration
-- (crates/paladin-storage/migrations/sqlite/003_create_assistants_tables.sql),
-- except `body` is JSONB rather than TEXT (bound via an explicit `::jsonb`
-- cast on write, the `runs.input` precedent from `002`), and
-- `created_at`/`deleted_at` are native TIMESTAMPTZ.
--
-- `assistant_versions`'s PRIMARY KEY `(assistant_id, version)` IS the D-29
-- immutability invariant -- see the SQLite migration's header comment for
-- the full rationale (no adapter ever executes an UPDATE against this
-- table's `kind`/`body` columns; no `update_version` method or `PUT` route
-- exists above it).

CREATE TABLE IF NOT EXISTS assistants (
    assistant_id   TEXT PRIMARY KEY,
    latest         INTEGER NOT NULL,
    source         TEXT NOT NULL DEFAULT 'stored',
    created_at     TIMESTAMPTZ NOT NULL,
    deleted_at     TIMESTAMPTZ NULL,
    schema_version TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS assistant_versions (
    assistant_id   TEXT NOT NULL,
    version        INTEGER NOT NULL,
    kind           TEXT NOT NULL,
    body           JSONB NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL,
    created_by     TEXT NULL,
    note           TEXT NULL,
    schema_version TEXT NOT NULL,
    PRIMARY KEY (assistant_id, version)
);

-- Serves `AssistantRepositoryPort::list_versions`'s documented ascending
-- `version` ordering within one assistant.
CREATE INDEX IF NOT EXISTS idx_assistant_versions_assistant_version
ON assistant_versions(assistant_id, version);
