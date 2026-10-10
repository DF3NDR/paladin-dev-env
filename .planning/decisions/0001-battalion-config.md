# ADR-0001: BattalionConfig field set

## Status

Superseded

Superseded by ADR-0059 on 2026-10-09 (Phase 44): the `timeout_seconds`, `retry_policy` and `error_strategy` fields are replaced by one `aegis: Aegis` policy bundle; the rest of this record is history.

**Date:** 2026-07-31

## Context

Two distinct `BattalionConfig` structs ship in the tree, and three documented positions compete
for the field set: Epic 4's original form, Epic 5's revision, and a run-2 relocation that never
landed. Which struct is the authoritative `BattalionConfig`, and what becomes of the second one?

Run-3 code verification settles the variant choice. The struct at
`crates/paladin-core/src/platform/container/battalion/mod.rs:37` is `REQ-battalion-config-v1`
(Epic 4 FR-4.1) **exactly** — `name`, `description: Option<String>`, `timeout_seconds`,
`retry_policy: RetryPolicy`, `error_strategy: ErrorStrategy`, `metadata_output_dir: Option<PathBuf>`.
Epic 5's `retry_attempts: u32` and `enable_checkpointing: bool` are absent, and `description` was
not dropped. Confirmed by grep on 2026-07-31:
`grep -n "pub struct BattalionConfig" crates/paladin-core/src/platform/container/battalion/mod.rs`
returns exactly one match at line 37 in that file. The third position,
`REQ-commander-config-metadata-dir-v3` (Epic 22 FR-10.1), relocates `metadata_output_dir` to a
`CommanderConfig` type — `grep -rq "CommanderConfig" crates/ src/` returns no matches anywhere in
the tree, so that position was never built.

## Decision

- `crates/paladin-core/src/platform/container/battalion/mod.rs:37` is the one authoritative
  `BattalionConfig`.
- The struct at `crates/paladin-core/src/platform/container/citadel.rs:280` is not a competing
  definition — it is a self-described placeholder whose own doc comment reads: "Configuration
  parameters for Battalion orchestration. Contains settings that control how a Battalion executes
  its Paladins. This is a placeholder and will be expanded in Epic 4." Epic 4 did expand
  `BattalionConfig` — elsewhere, at `battalion/mod.rs:37` — and nobody removed the placeholder
  afterward.
- It is a different concept — checkpoint/resume knobs for `BattalionState`, not orchestration
  config — so it is **renamed** rather than deleted or merged. The chosen identifier is
  `BattalionCheckpointConfig`. CONTEXT.md left the exact name to the planner's discretion; this ADR
  is where it is fixed.
- The rename **keeps all three fields — `max_concurrency`, `timeout_seconds`, `continue_on_error` —
  and their serde shape unchanged**. As `citadel.rs` declares them today: `max_concurrency:
  Option<usize>` (`#[serde(default)]`), `timeout_seconds: Option<u64>` (`#[serde(default)]`),
  `continue_on_error: bool` (`#[serde(default)]`).
- **No persisted-schema change and no migration.** Replacing the placeholder with the real
  `BattalionConfig` would change `BattalionState`'s serialized form — `schema_version` is `"1.0.0"`
  at `citadel.rs:233` (the `BattalionState.config: BattalionConfig` field), consumed by
  `crates/paladin-memory/src/citadel/file_citadel.rs` — and would require a version bump plus a read
  path for existing checkpoints. Renaming the placeholder type in place avoids both.

## Considered Options

- `REQ-battalion-config-v1` (Epic 4 FR-4.1) — **chosen.** This is the field set shipped at
  `battalion/mod.rs:37` exactly: `name`, `description`, `timeout_seconds`, `retry_policy`,
  `error_strategy`, `metadata_output_dir`.
- `REQ-battalion-config-v2` (Epic 5 FR-7) — rejected; not what shipped. Its `retry_attempts: u32`
  and `enable_checkpointing: bool` fields are absent from the tree, and its dropped `description`
  field was in fact kept.
- `REQ-commander-config-metadata-dir-v3` (run 2, Epic 22 FR-10.1) — rejected, and **never built**.
  This position relocates `metadata_output_dir` to a `CommanderConfig` type at
  `src/core/platform/container/battalion/commander_config.rs` with a YAML surface
  `commander.metadata_output_dir`. No such type exists anywhere in `crates/` or `src/`;
  `metadata_output_dir` ships on `BattalionConfig` itself (`battalion/mod.rs:54`), and the export
  path is `crates/paladin-battalion/src/commander.rs:870`. Its absence from the tree is itself the
  finding — the run-2 "three competing owners" warning collapses to exactly one owner.

## Code Locations

- `crates/paladin-core/src/platform/container/battalion/mod.rs:37` — the authoritative
  `BattalionConfig` struct declaration.
- `crates/paladin-core/src/platform/container/citadel.rs:233` — `BattalionState.config:
  BattalionConfig`, the field whose serialized shape the no-migration constraint protects.
- `crates/paladin-core/src/platform/container/citadel.rs:280` — the placeholder duplicate struct,
  to be renamed `BattalionCheckpointConfig`.
- `crates/paladin-memory/src/citadel/file_citadel.rs:507` — `BattalionConfig::default()` in
  `test_save_and_load_battalion`, a consumer of the persisted shape via `BattalionState::new`.
- `crates/paladin-memory/src/citadel/file_citadel.rs:541` — `BattalionConfig::default()` in
  `test_list_saved_multiple`, the second consumer site.

## Code Conformance

must change

Phase 2 **GAP-07** is the requirement that lands the rename of `citadel.rs:280`'s
`BattalionConfig` to `BattalionCheckpointConfig`. Nothing in this phase edits Rust source — this
ADR only records the verdict and the identifier GAP-07 will apply.

## Downstream Consumers

- Phase 2 GAP-07 — executes the `BattalionCheckpointConfig` rename against
  `crates/paladin-core/src/platform/container/citadel.rs:280`.
- `crates/paladin-memory/src/citadel/file_citadel.rs` — the consumer whose serialized
  `BattalionState` shape (via `schema_version: "1.0.0"`) constrains the decision to a rename with an
  unchanged serde shape rather than a replacement.
