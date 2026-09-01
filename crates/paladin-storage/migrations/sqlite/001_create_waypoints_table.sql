-- Migration: Create Waypoints Table (SQLite)
-- Purpose: Persist WarEngine superstep checkpoints (ENG-05, D-01, D-02)
-- Version: 001
-- Date: 2026-09-01
--
-- `payload` holds the full serialized JSON Waypoint (D-02: TEXT, not BLOB,
-- for debuggability and because the sqlx `json` feature is already enabled
-- workspace-wide). `status` duplicates the payload's status as its own
-- serialized-JSON column so history/list_threads summaries can be built
-- without deserializing the (potentially large) full payload. Registered in
-- MIGRATION.md section 9.4.

CREATE TABLE IF NOT EXISTS waypoints (
    waypoint_id TEXT PRIMARY KEY NOT NULL,
    thread_id   TEXT NOT NULL,
    parent_id   TEXT NULL,
    superstep   INTEGER NOT NULL,
    status      TEXT NOT NULL,
    payload     TEXT NOT NULL,
    created_at  TEXT NOT NULL
);

-- Serves history()/list_threads() ordering (created_at DESC, with superstep
-- DESC as the documented tiebreak) and thread-scoped lookups.
CREATE INDEX IF NOT EXISTS idx_waypoints_thread_created
ON waypoints(thread_id, created_at DESC);
