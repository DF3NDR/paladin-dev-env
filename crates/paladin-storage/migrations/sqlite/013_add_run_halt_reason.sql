-- Migration: Add Run Halt Reason (SQLite)
-- Purpose: Record the typed reason a run halted with (ALLOW-03, Phase 42 D-06, ADR-0057 group c)
-- Version: 013
-- Date: 2026-10-06
--
-- `halt_reason` holds a serialized `HaltReason` JSON object, tagged by `reason`:
-- `{"reason":"allowance_exhausted", ...}` carrying the halted ceiling's own figures as integer
-- nano-units plus a currency (never display strings, never floating point), or
-- `{"reason":"ledger_unavailable"}` when the ledger could not be read and the run halted
-- fail-closed. It is NULL for every run that did not halt on spend, and for every row written
-- before this migration -- a NULL reads back as "no reason recorded", never as an error.
--
-- Additive only: one nullable column. `007_create_treasury_ledger_table.sql` and every other
-- table are untouched, `RUN_SCHEMA_VERSION` stays `v1`, and the column is TEXT on this backend
-- exactly as `output` is (see `002_create_runs_table.sql`).

ALTER TABLE runs ADD COLUMN halt_reason TEXT NULL;
