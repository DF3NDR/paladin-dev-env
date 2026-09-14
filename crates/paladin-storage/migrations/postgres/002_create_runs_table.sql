-- Migration: Create Runs Table (PostgreSQL)
-- Purpose: Persist Platform API run lifecycle state (PLAT-01, PLAT-02, D-04, D-17)
-- Version: 002
-- Date: 2026-09-08
--
-- Same logical schema as the SQLite sibling migration
-- (crates/paladin-storage/migrations/sqlite/002_create_runs_table.sql), except
-- `input`/`webhook`/`pending_responses`/`fork_from`/`output` are JSONB rather
-- than TEXT, `submitted_at`/`started_at`/`finished_at` are native TIMESTAMPTZ,
-- and `cancel_requested` is a native BOOLEAN -- mirroring `001`'s split.
--
-- `idx_runs_thread_active` is the ONLY unique index on this table besides the
-- primary key -- see the SQLite migration's header comment for why that
-- invariant matters for `is_unique_violation()` disambiguation
-- (27-RESEARCH.md Pattern 2).
--
-- No status-history table (D-05): every status transition is a
-- compare-and-set `UPDATE` on this one row (D-04); the Waypoint chain
-- (`GET /threads/{id}/history`) is already the execution audit trail.

CREATE TABLE IF NOT EXISTS runs (
    run_id             TEXT PRIMARY KEY,
    thread_id          TEXT NOT NULL,
    assistant_id       TEXT NOT NULL,
    assistant_version  INTEGER NOT NULL,
    status             TEXT NOT NULL,
    input              JSONB NOT NULL,
    submitted_at       TIMESTAMPTZ NOT NULL,
    started_at         TIMESTAMPTZ NULL,
    finished_at        TIMESTAMPTZ NULL,
    attempt            INTEGER NOT NULL DEFAULT 0,
    cancel_requested   BOOLEAN NOT NULL DEFAULT FALSE,
    error              TEXT NULL,
    webhook            JSONB NULL,
    pending_responses  JSONB NOT NULL DEFAULT '[]',
    fork_from          JSONB NULL,
    output             JSONB NULL,
    final_waypoint_id  TEXT NULL,
    schema_version     TEXT NOT NULL
);

-- The D-17 invariant itself: at most one row per thread whose status is one
-- of the three "active" statuses (D-18: AwaitingInput counts as busy). An
-- `insert` unique violation on this index is what
-- `RunRepositoryError::ThreadBusy` maps from.
CREATE UNIQUE INDEX IF NOT EXISTS idx_runs_thread_active
ON runs(thread_id) WHERE status IN ('queued','running','awaiting_input');

-- Serves `RunRepositoryPort::list`'s documented ordering
-- `(submitted_at DESC, run_id DESC)`.
CREATE INDEX IF NOT EXISTS idx_runs_submitted
ON runs(submitted_at DESC, run_id DESC);
