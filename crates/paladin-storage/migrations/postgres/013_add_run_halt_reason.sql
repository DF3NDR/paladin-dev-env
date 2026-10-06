-- Migration: Add Run Halt Reason (PostgreSQL)
-- Purpose: Record the typed reason a run halted with (ALLOW-03, Phase 42 D-06, ADR-0057 group c)
-- Version: 013
-- Date: 2026-10-06
--
-- Mirrors `../sqlite/013_add_run_halt_reason.sql`; see that file for the column's contract.
-- On this backend the column is JSONB exactly as `output` is (see `002_create_runs_table.sql`).
-- NULL for every run that did not halt on spend and for every row written before this
-- migration. Additive only: `IF NOT EXISTS` makes a re-run a no-op, and no other table or
-- column changes.

ALTER TABLE runs ADD COLUMN IF NOT EXISTS halt_reason JSONB NULL;
