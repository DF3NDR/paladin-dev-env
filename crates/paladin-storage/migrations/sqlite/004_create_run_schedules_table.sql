-- Migration: Create Run Schedules Table (SQLite)
-- Purpose: Persist restart- and replica-safe cron run schedules (PLAT-05, D-36, D-37, D-38, D-39)
-- Version: 004
-- Date: 2026-09-08
--
-- `run_schedules` is the whole D-37 restart/replica-safety story: `next_tick`
-- is persisted, and a tick is CLAIMED by the conditional update
-- `UPDATE run_schedules SET last_tick = ?next, next_tick = ?after WHERE
-- schedule_id = ? AND next_tick = ?next` -- exactly one replica's update
-- affects a row, and only that replica submits the run. Restart safety falls
-- out of the SAME mechanism: a restarted `ScheduleService` re-reads
-- `next_tick` from this table rather than any in-process state, so it never
-- double-fires (the claim already advanced `next_tick`) and never
-- missed-then-double-fires (`on_missed` governs the recompute).
--
-- `input`/`thread_strategy`/`webhook` mirror `runs`' TEXT-JSON columns (D-02,
-- `002_create_runs_table.sql`): TEXT holding serialized JSON, for
-- debuggability and because the sqlx `json` feature is already enabled
-- workspace-wide. `thread_strategy` serializes as either the bare string
-- `"new_thread_per_tick"` or `{"fixed_thread":"<thread_id>"}` (the default
-- externally-tagged serde representation, see
-- `crates/paladin-core/src/platform/container/run_schedule.rs`).
-- `on_missed` is a plain TEXT enum column (`'skip'` | `'run_once'`), not
-- JSON, since it carries no payload.
--
-- Not `tokio-cron-scheduler` (D-36): the existing
-- `TokioCronSchedulerAdapter` (`crates/paladin-storage/src/scheduler.rs`) is
-- untouched and shares no table with this one.

CREATE TABLE IF NOT EXISTS run_schedules (
    schedule_id       TEXT PRIMARY KEY NOT NULL,
    assistant_id      TEXT NOT NULL,
    assistant_version INTEGER NULL,
    cron              TEXT NOT NULL,
    timezone          TEXT NOT NULL,
    input             TEXT NOT NULL,
    enabled           INTEGER NOT NULL,
    thread_strategy   TEXT NOT NULL,
    on_missed         TEXT NOT NULL,
    webhook           TEXT NULL,
    last_tick         TEXT NULL,
    next_tick         TEXT NULL,
    skipped_ticks     INTEGER NOT NULL DEFAULT 0,
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL,
    schema_version    TEXT NOT NULL
);

-- Serves `RunScheduleRepositoryPort::due`'s documented `(enabled, next_tick
-- <= now)` filter, ordered ascending by `next_tick` -- and is the index the
-- D-37 conditional-update tick claim's own `WHERE schedule_id = ? AND
-- next_tick = ?` predicate benefits from on repeated polling.
CREATE INDEX IF NOT EXISTS idx_run_schedules_next_tick
ON run_schedules(enabled, next_tick);
