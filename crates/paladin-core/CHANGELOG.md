# Changelog

All notable changes to `paladin-core` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project follows lockstep workspace versioning.

## [Unreleased]

## [0.10.0] - 2026-09-10

### Added
- `Waypoint` (`platform::container::waypoint`), the per-superstep `Battlefield` snapshot type
  written by the new `WarEngine`/`WarGraph` execution path (ENG-01…ENG-08); `WaypointStatus::
  AwaitingInput` carries `{ parleys, responses }` (HITL-01) plus an additive
  `#[serde(default)] fork_of: Option<WaypointId>` field (HITL-03).
- Structured `platform::container::node_error::NodeError`, the typed error consumed by
  `BattalionError::Node` below (FT-FR-02).
- `GarrisonEntry::summary(content)` constructor for system-authored conversation summaries
  (`ConversationRole::System`, `is_summary: true`, RT-FR-12).

### Changed
- `StopReason` gained two new variants, `CallLimit` and `TokenBudget`, and is now
  `#[non_exhaustive]` (RT-FR-04, RT-FR-06).
- `BattalionError` gained a new `Node(NodeError)` variant and is now `#[non_exhaustive]`
  (FT-FR-02).
- `PaladinError` gained a `transience()` method and four new structured variants —
  `LlmFailure { transience, status, provider, message }` (FT-FR-01),
  `GuardrailTripped { rule, target }` (RT-FR-06), `StructuredOutputInvalid { attempts,
  last_error, raw_output }` (RT-FR-19), `ArmamentFailed { tool, reason }` (RT-FR-24) — on the
  already-`#[non_exhaustive]` enum.
- `GarrisonEntry` gained an additive `is_summary: bool` field (`#[serde(default)]`, RT-FR-12);
  the SQLite adapter's `002_add_garrison_is_summary.sql` migration keeps existing rows readable
  as `is_summary == false`. Struct is now `#[non_exhaustive]`.
- `PaladinResult` gained an additive `served_by: Option<String>` field
  (`#[serde(default, skip_serializing_if = "Option::is_none")]`), naming the provider that
  actually served a `FallbackLlmAdapter` call (FT-FR-17). Struct stays constructible
  (deliberately not `#[non_exhaustive]`).

See root `MIGRATION.md` §9.2 for the full X-10 compatibility register, mitigation, and
requirement IDs for each entry above.

## [0.9.0] - 2026-09-01

## [0.8.1-rc.5] - 2026-08-31

## [0.8.1-rc.4] - 2026-08-29

### Added
- Crate-level release artifacts for Epic 4 API stabilization.
- Explicit crate documentation and release-readiness linkage from workspace docs.

### Changed
- Public API stability documentation aligned with `STABLE_API.md` crate-tier policy.

### Fixed
- Crate metadata and README linkage validated for crates.io release preparation.
