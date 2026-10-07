# Changelog

All notable changes to `paladin-core` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project follows lockstep workspace versioning.

## [Unreleased]

### Fixed

- `HaltReason::herald_line` and `AllowanceWarning::herald_line` no longer render a window ceiling
  that has no recorded window end as a `lifetime cap`; it now reads `window` (Phase 42 review
  IN-4). Lifetime and dated window lines are unchanged.

### Added

- Phase 42 (mid-run halt and terminal status, ALLOW-03, ALLOW-05, PLAT-09; ADR-0057):
  `HaltReason` (`AllowanceExhausted`, `LedgerUnavailable`; `as_str`, `wire_json`, `herald_line`) and
  `AllowanceRefusal::details_json`; `RunFinishStatus::Cancelled` (the enum is now
  `#[non_exhaustive]`) and `TraceEvent::RunFinished.halt_reason`; `StopReason::AllowanceHalted`;
  `DerivedTokenBudget`, `RunScope.derived_token_budget` and `with_derived_token_budget`, and
  `Admission::with_derived_budget` / `derived_budget`; `Run.halt_reason`; `NoticeKind` (a `kind`
  field on `NoticeRecord` and `AllowanceNotice`) and `RunEventKind::AllowanceHalted`; the herald
  helpers `HALT_REASON_METADATA_KEY`, `ExecutionMetadata::with_halt_reason` and
  `halt_reason_display` (see `MIGRATION.md` §9.2).

- Phase 41 (allowance admission, ALLOW-01/02/04): `platform::container::allowance` module with
  `AllowanceScopeKind`, `AllowanceLimitKind`, `AllowanceRefusal`, `AllowanceWarning`,
  `AllowanceNotice`, `Admission`, `NoticeRecord`, `NoticeOutcome`, `LIFETIME_WINDOW_START` and the
  integer-only `crosses_warn_threshold`; `BalanceQuery` in `treasury_ledger`.
- `TraceEvent::AllowanceWarning` (the thirteenth variant) and `RunEventKind::AllowanceWarning`
  (`RunEventKind` is now `#[non_exhaustive]` and gains `as_str`, the one to-string source).
- `RunSchedule.created_by: Option<RunAttribution>` and `with_created_by`; `RunScope.allowance_warnings`
  and `with_allowance_warnings`; `ExecutionMetadata::with_allowance_warnings` and
  `allowance_warning_display` (see `MIGRATION.md` §9.2).

- `platform::container::cost` module: `Cost`, `CurrencyCode`, `CostError`, `PriceRow`,
  `PriceTable`, `cost_of_call`, `CostTally` — pure `i64` nano-unit fixed-point cost arithmetic, no
  floating point anywhere in the module (PRICE-02).
- `PaladinResult.cost: Option<Cost>` and `TraceEvent::NodeFinished`/`RunFinished.cost:
  Option<Cost>` additive fields, riding beside `usage` everywhere it travels (PRICE-03; see the
  root `CHANGELOG.md` and `MIGRATION.md` §9.2).
- `TraceDispatcher::total_cost()`, the synchronous twin of `total_usage()`.
- `ExecutionMetadataBuilder::cost`, `ExecutionMetadata::cost_currency`/`cost_display`, and
  `ExecutionMetadata::from_run_finished(record, model_used)` — the engine-path producer built
  from a completed run's `TraceEvent::RunFinished`.
- `platform::container::treasury_ledger` module: `LedgerScope` (with the `unattributed()`
  sentinel), `ReservationId`, `SettlementKey`, `LedgerEntryKind`, `ReserveRequest`/`SettleRequest`/
  `SettleOutcome`, `SpendGroupBy`/`SpendQuery`/`SpendRow`, `SettlementContext`, and `format_cost`
  — the ledger's domain value types (LEDGR-01), re-exported from the facade as
  `core::platform::container::treasury_ledger`.
- `RunScope.run_id: Option<RunId>` and `RunScope::with_run_id` — additive; `RunScope` stays
  `#[non_exhaustive]` (LEDGR-04).

- `platform::container::principal` module: `TenantId` (with `TENANT_ID_MAX_LEN` and
  `TenantIdError`), `PrincipalRef`, `RunAttribution` and `RunReadScope` — the tenant identifier, the
  principal reference every run submission carries, the persisted attribution and the one shared
  tenant read-scope rule (TENANT-01, PLAT-07); re-exported from the facade as
  `core::platform::container::principal`.
- `Run.submitted_by: Option<RunAttribution>` and `Run::with_submitted_by` — additive under
  `#[non_exhaustive]`, `#[serde(default, skip_serializing_if)]`, `RUN_SCHEMA_VERSION` unchanged
  (TENANT-02).
- `RunScope.ledger_scope: Option<LedgerScope>` and `RunScope::with_ledger_scope` — the ledger
  scope an agent-kind run or an HTTP agent-execute call settles under (TENANT-02).
- `LedgerScope::from_attribution(Option<&RunAttribution>)` — the one mapping from a recorded
  principal to a ledger scope; `None` yields the `unattributed` sentinel (TENANT-02).

### Changed

- The `cost_estimate`/`total_cost()` rustdoc reserved-note on `ExecutionMetadata` now reads
  "produced by the Treasurer" instead of "no in-tree producer yet" (D-12).

## [0.10.1] - 2026-09-20

Patch release carried by the workspace-wide version bump (0.10.0 -> 0.10.1). No source
change in this crate — see the root `CHANGELOG.md`'s `[0.10.1]` section for the two
release-pipeline defects this patch fixes.

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
