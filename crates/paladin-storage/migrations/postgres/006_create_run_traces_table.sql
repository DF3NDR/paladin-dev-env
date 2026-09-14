-- Migration: Create Run Traces Table (PostgreSQL)
-- Purpose: Durable, append-only per-record trace persistence (OBS-02, D-17)
-- Version: 006
-- Date: 2026-09-08
--
-- Same logical schema as the SQLite sibling migration
-- (crates/paladin-storage/migrations/sqlite/006_create_run_traces_table.sql),
-- except `at` is a native TIMESTAMPTZ and `record` is JSONB rather than
-- opaque TEXT -- this column is never re-signed or byte-compared (unlike
-- `webhook_deliveries.payload`), so JSONB's normalization is safe here and
-- lets an operator query it directly if needed.

CREATE TABLE IF NOT EXISTS run_traces (
    thread_id       TEXT NOT NULL,
    seq             BIGINT NOT NULL,
    run_id          TEXT NULL,
    superstep       BIGINT NOT NULL,
    at              TIMESTAMPTZ NOT NULL,
    schema_version  TEXT NOT NULL,
    record          JSONB NOT NULL,
    PRIMARY KEY (thread_id, seq)
);

-- Serves `RunTracePort::read`'s documented `(thread_id, seq > ?)` range scan.
CREATE INDEX IF NOT EXISTS idx_run_traces_thread_seq
ON run_traces(thread_id, seq);

-- Serves `RunTracePort::prune_thread`'s documented
-- `(thread_id, superstep < ?)` deletion predicate.
CREATE INDEX IF NOT EXISTS idx_run_traces_thread_superstep
ON run_traces(thread_id, superstep);
