-- Migration: Create Run Traces Table (SQLite)
-- Purpose: Durable, append-only per-record trace persistence (OBS-02, D-17)
-- Version: 006
-- Date: 2026-09-08
--
-- `run_traces` is D-17's one-way, append-only home for `TraceRecord`s once
-- persistence is opted into: `(thread_id, seq)` is the primary key, so a
-- retried `append` of the same record is a no-op (`ON CONFLICT DO NOTHING`),
-- never a duplicate row. `record` stores the FULL serialized `TraceRecord`
-- envelope (thread_id, run_id, seq, at, and the flattened event) so `read`
-- can deserialize a row directly with no reconstruction; `thread_id`,
-- `seq`, `run_id`, `superstep`, `at` and `schema_version` are broken out as
-- their own columns purely to serve `read`'s `(thread_id, seq > ?)` range
-- scan and `prune_thread`'s `(thread_id, superstep < ?)` deletion, without
-- parsing `record` for either. `superstep` is derived at write time from
-- whichever `TraceEvent` variants carry one (`SuperstepStarted`/
-- `NodeStarted`/`NodeFinished`/`DeltaMerged`/`WaypointSaved`, plus
-- `RunFinished`'s own `total_supersteps`); every other variant is stamped
-- `0` -- see `paladin_storage::run_trace::superstep_of`'s own doc comment
-- for the full rationale.

CREATE TABLE IF NOT EXISTS run_traces (
    thread_id       TEXT NOT NULL,
    seq             BIGINT NOT NULL,
    run_id          TEXT NULL,
    superstep       BIGINT NOT NULL,
    at              TEXT NOT NULL,
    schema_version  TEXT NOT NULL,
    record          TEXT NOT NULL,
    PRIMARY KEY (thread_id, seq)
);

-- Serves `RunTracePort::read`'s documented `(thread_id, seq > ?)` range scan.
CREATE INDEX IF NOT EXISTS idx_run_traces_thread_seq
ON run_traces(thread_id, seq);

-- Serves `RunTracePort::prune_thread`'s documented
-- `(thread_id, superstep < ?)` deletion predicate.
CREATE INDEX IF NOT EXISTS idx_run_traces_thread_superstep
ON run_traces(thread_id, superstep);
