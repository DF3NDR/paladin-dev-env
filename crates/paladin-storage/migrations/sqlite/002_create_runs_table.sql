-- Migration: Create Runs Table (SQLite)
-- Purpose: Persist Platform API run lifecycle state (PLAT-01, PLAT-02, D-04, D-17)
-- Version: 002
-- Date: 2026-09-08
--
-- `idx_runs_thread_active` is the ONLY unique index on this table besides the
-- primary key -- the `RunRepositoryError::ThreadBusy` mapping via
-- `is_unique_violation()` (27-RESEARCH.md Pattern 2) depends on that
-- invariant holding: SQLite's driver never populates `DatabaseError::constraint()`,
-- so a future second unique index on `runs` would make the violation
-- ambiguous and needs its own review before being added.
--
-- No status-history table (D-05): every status transition is a
-- compare-and-set `UPDATE` on this one row (D-04); the Waypoint chain
-- (`GET /threads/{id}/history`) is already the execution audit trail.
--
-- `input`/`webhook`/`pending_responses`/`fork_from`/`output` are TEXT holding
-- serialized JSON (mirroring `001_create_waypoints_table.sql`'s `payload`
-- column: debuggability, and the sqlx `json` feature is already enabled
-- workspace-wide). `submitted_at`/`started_at`/`finished_at` are TEXT RFC
-- 3339 timestamps, mirroring `001`'s `created_at` choice.

CREATE TABLE IF NOT EXISTS runs (
    run_id             TEXT PRIMARY KEY NOT NULL,
    thread_id          TEXT NOT NULL,
    assistant_id       TEXT NOT NULL,
    assistant_version  INTEGER NOT NULL,
    status             TEXT NOT NULL,
    input              TEXT NOT NULL,
    submitted_at       TEXT NOT NULL,
    started_at         TEXT NULL,
    finished_at        TEXT NULL,
    attempt            INTEGER NOT NULL DEFAULT 0,
    cancel_requested   INTEGER NOT NULL DEFAULT 0,
    error              TEXT NULL,
    webhook            TEXT NULL,
    pending_responses  TEXT NOT NULL DEFAULT '[]',
    fork_from          TEXT NULL,
    output             TEXT NULL,
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
