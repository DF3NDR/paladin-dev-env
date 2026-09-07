-- Migration: Add is_summary column to garrison_entries
-- Purpose: RT-FR-12 (D-16, D-17) -- flag an entry as a compressed stand-in for
-- older history so the assembly, trimmer and summarizer can agree on the
-- effective-history rule (latest-summary-wins).
-- Version: 002
-- Additive only: no down migration (D-17). 001 is untouched.

ALTER TABLE garrison_entries ADD COLUMN is_summary INTEGER NOT NULL DEFAULT 0;
