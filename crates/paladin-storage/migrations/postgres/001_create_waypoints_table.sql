-- Migration: Create Waypoints Table (PostgreSQL)
-- Purpose: Persist WarEngine superstep checkpoints (ENG-05, D-01, D-02)
-- Version: 001
-- Date: 2026-09-01
--
-- Same logical schema as the SQLite sibling migration
-- (crates/paladin-storage/migrations/sqlite/001_create_waypoints_table.sql),
-- except `payload` is JSONB (per the PRD) rather than TEXT, and `created_at`
-- is a native TIMESTAMPTZ. Registered in MIGRATION.md section 9.4.

CREATE TABLE IF NOT EXISTS waypoints (
    waypoint_id TEXT PRIMARY KEY,
    thread_id   TEXT NOT NULL,
    parent_id   TEXT NULL,
    superstep   BIGINT NOT NULL,
    status      TEXT NOT NULL,
    payload     JSONB NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL
);

-- Serves history()/list_threads() ordering (created_at DESC, with superstep
-- DESC as the documented tiebreak) and thread-scoped lookups.
CREATE INDEX IF NOT EXISTS idx_waypoints_thread_created
ON waypoints(thread_id, created_at DESC);
