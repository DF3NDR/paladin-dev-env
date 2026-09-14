-- Migration: Create Run Schedules Table (PostgreSQL)
-- Purpose: Persist restart- and replica-safe cron run schedules (PLAT-05, D-36, D-37, D-38, D-39)
-- Version: 004
-- Date: 2026-09-08
--
-- Same logical schema as the SQLite sibling migration
-- (crates/paladin-storage/migrations/sqlite/004_create_run_schedules_table.sql),
-- except `input`/`thread_strategy`/`webhook` are JSONB rather than TEXT
-- (bound via an explicit `::jsonb` cast on write, the `runs.input`
-- precedent from `002`), `enabled` is native BOOLEAN, and the timestamp
-- columns are native TIMESTAMPTZ.
--
-- `run_schedules.next_tick` plus the D-37 conditional-update tick claim
-- (`UPDATE run_schedules SET last_tick = $1, next_tick = $2 WHERE
-- schedule_id = $3 AND next_tick = $4`) IS the restart/replica-safety
-- invariant -- see the SQLite migration's header comment for the full
-- rationale.

CREATE TABLE IF NOT EXISTS run_schedules (
    schedule_id       TEXT PRIMARY KEY,
    assistant_id      TEXT NOT NULL,
    assistant_version INTEGER NULL,
    cron              TEXT NOT NULL,
    timezone          TEXT NOT NULL,
    input             JSONB NOT NULL,
    enabled           BOOLEAN NOT NULL,
    thread_strategy   JSONB NOT NULL,
    on_missed         TEXT NOT NULL,
    webhook           JSONB NULL,
    last_tick         TIMESTAMPTZ NULL,
    next_tick         TIMESTAMPTZ NULL,
    skipped_ticks     INTEGER NOT NULL DEFAULT 0,
    created_at        TIMESTAMPTZ NOT NULL,
    updated_at        TIMESTAMPTZ NOT NULL,
    schema_version    TEXT NOT NULL
);

-- Serves `RunScheduleRepositoryPort::due`'s documented `(enabled, next_tick
-- <= now)` filter, ordered ascending by `next_tick`.
CREATE INDEX IF NOT EXISTS idx_run_schedules_next_tick
ON run_schedules(enabled, next_tick);
