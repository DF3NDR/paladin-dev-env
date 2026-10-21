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

- Phase 51 (REASON-01; plan 51-01): `platform::container::reasoning` -- `ReasoningEffort`
  (`Off`, `Auto`, `Minimal`, `Low`, `Medium`, `High`, `XHigh`, `Max`, `Budget(u32)`; `level_rank`,
  `is_level`, `LEVELS`, `Display`, serde spellings `off auto minimal low medium high xhigh max`
  and `{"budget":n}`), `ReasoningFallback` (`Refuse` default, `NearestLower`, `NearestHigher`) and
  the response-metadata keys `REASONING_REQUESTED_METADATA_KEY` /
  `REASONING_EFFECTIVE_METADATA_KEY`. Additive: both enums are `#[non_exhaustive]`.

- Phase 50 (G-50-5a remainder (b), PARITY-05; plan 50-29): `IncurredSpend` in
  `platform::container::cost` -- the spend an operation has incurred so far, every billed call
  counted, whatever its eventual outcome: `new`, `record(usage, cost)` (usage summed saturating,
  cost under the `CostTally` rule so one unpriced call makes `cost()` `None`), `usage()`, `cost()`,
  `calls()` and `is_empty()`. `Default` is the empty spend, the neutral `(TokenUsage::default(),
  None)` pair `CostTally::record_node` ignores. It is the accumulator behind `paladin-ports`'s
  attempt spend scope.

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

- **BREAKING:** `PaladinData` (`platform::container::paladin`) gains `reasoning_effort:
  Option<ReasoningEffort>` and `reasoning_fallback: Option<ReasoningFallback>` (REASON-01, D-00e;
  Phase 51 plan 07). Both are `#[serde(default, skip_serializing_if = "Option::is_none")]` and
  `None` in `Default`; stored snapshots still load. Migration: name both fields in a full struct
  literal, or spread `..PaladinData::default()`.

- Phase 48 (base entity rename, ENTITY-03; ADR-0060): `base::service::node_version_service`
  becomes `entity_version_service` and its family becomes `EntityVersion`,
  `EntityVersionMetadata`, `EntityVersionRepository`, `EntityVersionService` and
  `InMemoryEntityVersionRepository`; `VersioningError::NodeNotFound` becomes `EntityNotFound`; the
  record's `node_id` / `node_data` become `entity_id` / `entity_data` (also on `VersionHistory` and
  `VersionComparison`), and `VersioningConfig.max_versions_per_node` becomes
  `max_versions_per_entity`. `EntityVersion` keeps serde read-aliases for the v0.10 keys so stored
  version records still load. The unused `Field`, `CollectionType` / `Item`, `element`,
  `FieldVersionService` and `CollectionVersionService` are retained pending review (D-05); the
  `base` module rustdoc says why.
- Phase 48 (base entity rename, ENTITY-01; ADR-0060): persisted entity JSON is written with the
  new keys and still reads the v0.10 ones. `Entity<T>` stores its payload under `data` (read-alias
  `node`), `ContentItem` and `PromptItem` store their entity under `entity` (read-alias `node`), and
  `EntityVersion` stores `entity_id` / `entity_data`. A document carrying both spellings is
  rejected; a v0.10 binary cannot read v0.11 output. See `MIGRATION.md` 9.1 M-B-12 and 9.4.
- The `cost_estimate`/`total_cost()` rustdoc reserved-note on `ExecutionMetadata` now reads
  "produced by the Treasurer" instead of "no in-tree producer yet" (D-12).
- Phase 44 (legacy clean break, LEGACY-01, LEGACY-02; ADR-0059): `BattalionConfig` gains
  `#[serde(default)] aegis: Aegis`, `with_aegis` and `validate_aegis` (rejects `max_attempts == 0`,
  `Route` and `Custom` handlers, `RetryPredicate::Custom` and zero-length durations, and ignores
  `cache`). The `aegis` rustdoc is the per-pattern contract: a per-attempt timeout, a retry whose
  `max_attempts` counts total attempts, and `Absorb` to continue past a failure. A v0.10
  `BattalionConfig` document still deserializes; its removed keys are ignored and `aegis` is
  `Aegis::default()`. `BattalionConfig::new` arms no timeout. See `MIGRATION.md` 9.1 M-B-06 and
  M-B-07 and the Phase 44 9.2 rows.
- Phase 44 (LEGACY-01; ADR-0059; one-way): `BattalionResult.node_errors` is retyped from
  `Vec<battalion::NodeError>` to `Vec<node_error::NodeError>`, whose element is
  `{ node_id, attempt, transience, source }`. A stored v0.10 document with a non-empty
  `node_errors` no longer deserializes; an empty or absent array still does. See `MIGRATION.md`
  9.1 M-B-08 and 9.4.

### Removed

- Phase 44 (LEGACY-01, LEGACY-02, LEGACY-03; ADR-0059; a clean break with no shim, see `MIGRATION.md`
  9.2): the legacy Battalion `RetryPolicy`, `ErrorStrategy` and `NodeError`; `BattalionConfig`'s
  `timeout_seconds`, `retry_policy` and `error_strategy` fields and its `with_timeout`,
  `with_retry_policy` and `with_error_strategy` builders; `BattalionError::Timeout`;
  `ConclaveError::Timeout`; `ConclaveConfig::timeout_seconds` and `ConclaveConfig::with_timeout`
  (`Conclave::validate` now delegates the timeout and retry-policy checks to `validate_aegis`); and
  `PaladinError::LlmError`, `PaladinError::is_retryable` and `PaladinError::is_terminal`. Use
  `BattalionConfig.aegis`, the structured `node_error::NodeError`, `PaladinError::LlmFailure` and
  `PaladinError::transience()`.
- Phase 48 (ENTITY-01..05; ADR-0060; a clean break with no shim, see `MIGRATION.md` 9.2): the
  generic base wrapper `Node<T>` and `base::entity::node`, replaced by `base::entity::Entity<T>` with
  payload `data`; the `NodeVersion*` family and `base::service::node_version_service`, replaced by
  `EntityVersion*` in `entity_version_service`; the Citadel module's placeholder `PaladinData`,
  `PaladinStatus` and `Paladin` alias, replaced by the real `platform::container::paladin` types
  (`PaladinState.paladin` is the real `Paladin`; the real `PaladinStatus` adds `Reasoning`); and
  `ContentItem.node` / `PromptItem.node`, replaced by `.entity`. Graph items such as `NodeId`,
  `NodeSpec`, `StateNode` and the structured `NodeError` are different and unchanged.

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
