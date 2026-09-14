-- Migration: Create vault_records table for the Vault (cross-thread namespaced key/value memory)
-- Purpose: RT-FR-13...16 (D-18, D-23) -- SqliteVault's persistent backing store, sharing this
-- crate's one embedded migrator with SqliteGarrison (D-17/D-23). `ns` is the Namespace's
-- `/`-joined path -- unambiguous because a Namespace segment can never contain `/`, so two
-- distinct namespaces can never collide on their joined form (see Namespace::is_prefix_of's own
-- rustdoc for the segment-wise reasoning this column's format relies on).
-- Version: 003
-- Additive only: no down migration. 001 and 002 are untouched.
--
-- IF NOT EXISTS on both statements is what makes the shared-migrator arrangement harmless: a
-- database that only ever constructed a SqliteGarrison gains this table too (empty), and a
-- database that only ever constructed a SqliteVault gains garrison_entries too (empty) --
-- documented, harmless (D-23).

CREATE TABLE IF NOT EXISTS vault_records (
    ns TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (ns, key)
);

CREATE INDEX IF NOT EXISTS idx_vault_records_ns_key ON vault_records (ns, key);
