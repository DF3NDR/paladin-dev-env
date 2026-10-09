# Changelog

All notable changes to `paladin-ports` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project follows lockstep workspace versioning.

## [Unreleased]

### Added

- Phase 43 (rate pacing, the Cadence; PACE-01..PACE-05; ADR-0058): the `CadencePort` output port
  (`gate`, `record_rate_limited`, `record_success`, and the stampede-lock methods `try_lock` and
  `unlock`) with `CadenceKey`, `GateReading`, `CadenceError`, `CadencePolicy` (exponential back-off
  with full jitter), `LockKey`, `FencingToken` (`Distributed` or `Local`, `#[non_exhaustive]`, no
  `Ord`), `CADENCE_LOG_TARGET` and `CADENCE_DELAY_CEILING`. `RateLimitHints`, `RateLimitDimension`,
  `RateLimitDimensionKind` and `RetryDelaySource` carry a provider's parsed rate-limit numbers
  (never a raw header string), read through `LlmError::retry_after()` and
  `LlmError::rate_limit_hints()`. `NodeCachePort::put_fenced`, a defaulted method that delegates to
  `put`, so no implementor breaks (see `MIGRATION.md` section 9.2).

- `RunTracePort::max_seq(&self, &ThreadId)`, the largest persisted `seq` of a thread (`0` when it
  has none), with a defaulted implementation that pages through `read` so an existing implementor
  keeps compiling and stays correct. A run seeds its trace dispatcher from it so a second run on
  one thread no longer loses records to the `(thread_id, seq)` conflict rule (Phase 42 review
  WR-6).

- `SpendGuard::note_unsettled_spend(&self, &ThreadId)`, a defaulted no-op the engine calls when a
  superstep's priced charge could not be written to the ledger, so a metering guard can fail
  closed instead of reading a balance that no longer holds the run's own spend (Phase 42 review
  WR-4). A guard that does not override it keeps its behaviour.

- Phase 42 (mid-run halt and terminal status, ALLOW-03, ALLOW-05, PLAT-09; ADR-0057): the
  `SpendGuard` output port with `SpendDecision` and `NeverHalts`; the defaulted
  `AllowanceAdmissionPort::admit_for_model` with `AdmissionError::ModelUnpriced`;
  `RunSubmissionError::ModelUnpriced`; `RunOutcomeRecord.halt_reason`; the notice kind carried
  through `TreasuryNoticePort` (see `MIGRATION.md` §9.2).

- Phase 41 (allowance admission, ALLOW-01/02/04): `AllowanceAdmissionPort` and `AdmissionError`
  (input), `TreasuryNoticePort` (output), and the defaulted `TreasuryLedgerPort::balance`
  (`BalanceQuery`).
- `RunSubmissionError::AllowanceExhausted(AllowanceRefusal)`, `SubmitRun.attributed_to` and
  `CreateRunSchedule.created_by` (see `MIGRATION.md` §9.2).

- `LlmResponse.cost: Option<Cost>` additive field, priced by `paladin_llm`'s
  `PricingLlmAdapter::generate` from the response's own served model (PRICE-03; see
  `MIGRATION.md` §9.2).
- `StreamingResponse.cost: Option<Cost>` and `ChunkMetadata.cost: Option<Cost>` /
  `ChunkMetadata.execution: Option<ExecutionMetadata>` additive fields on the two
  `#[non_exhaustive]` port types the streaming path carries (PRICE-03).
- `output::treasury_ledger_port` module: `TreasuryLedgerPort`
  (`reserve`/`settle`/`release`/`spend`/`store_now`) and `TreasuryLedgerError`, mirroring
  `RunRepositoryPort`'s error-enum shape (X-06) with a compiling rustdoc mock (LEDGR-01,
  LEDGR-02, LEDGR-03, LEDGR-04).

- `RunQuery.scope: RunReadScope` (`Default` = `All`) — every `RunRepositoryPort::list` adapter
  applies a `Tenant(t)` scope inside its own query; the run contract suite gains attribution
  round-trip and scoped-list clauses (PLAT-07; see root `MIGRATION.md` §9.2).
- `PaladinExecutorPort::execute_scoped` and `StreamingExecutorPort::execute_stream_scoped` —
  defaulted methods that carry a `RunScope` (and its tenant ledger scope) into an executor; every
  existing implementor compiles unchanged (TENANT-02).

### Changed

- `LlmError::RateLimitExceeded` is now the `#[non_exhaustive]` struct variant `{ retry_after:
  Option<Duration>, hints: Option<Box<RateLimitHints>> }` (PACE-01; Phase 43 plan 43-02;
  `MIGRATION.md` section 9.2). Construct it with `LlmError::rate_limited(None)` or
  `rate_limited(Some(delay))` and match it as `LlmError::RateLimitExceeded { .. }`; `Display`
  stays `Rate limit exceeded` and `transience()` stays `Transient`. `CadencePort` gained the
  required methods `try_lock` and `unlock` in the same phase, before any release, so an
  out-of-tree implementor sees the port only in its final shape.

- **Breaking:** `SubmitRun.requested_by`, `ForkRun.requested_by` and `RunSubmissionPort::cancel`'s
  `requested_by` parameter are `Option<PrincipalRef>` (tenant, API key id and role travel together)
  instead of `Option<(String, UserRole)>`; `None` keeps its internal-caller meaning (TENANT-01,
  TENANT-02; see root `MIGRATION.md` §9.2).
- `CompositeSink::on_event` moves the record into its last child instead of cloning it for every
  child; no behaviour change (OBS-05).

## [0.10.1] - 2026-09-20

Patch release carried by the workspace-wide version bump (0.10.0 -> 0.10.1). No source
change in this crate — see the root `CHANGELOG.md`'s `[0.10.1]` section for the two
release-pipeline defects this patch fixes.

## [0.10.0] - 2026-09-10

### Added
- `TokenCounterPort` — `fn count(&self, text: &str, model: &str) -> u32`, synchronous and
  infallible (RT-FR-10).
- `VaultPort` — `put`/`get`/`delete`/`list`/`search` over namespaced key/value memory
  (RT-FR-13, RT-FR-14, RT-FR-15, RT-FR-16).
- `StructuredExecutorPort` — `execute_json_schema`/`execute_json_schema_observed`, object-safe
  at the JSON level, deliberately not a `PaladinPort` method (RT-FR-17, RT-FR-18, RT-FR-19).
- `ParleyPort` — `async fn resume_with(&self, thread: &ThreadId, responses: Vec<ParleyResponse>)
  -> Result<ResumeAccepted, ParleyError>` (HITL-FR-16).

### Changed
- `LlmError` gained new variants `ProviderError { provider, status, message }` and
  `AllProvidersFailed { attempts, last }`, a new `transience()` method, and is now
  `#[non_exhaustive]` (FT-FR-01, FT-FR-16).
- `LlmRequest` gained an additive `response_format: Option<ResponseFormat>` field
  (`#[serde(default)]`), is now `#[non_exhaustive]`, and gained a new
  `LlmRequest::new(model, prompt)` constructor plus chainable `with_attachments`/`with_stream`/
  `with_metadata`/`with_response_format` builders, replacing every in-tree full struct literal
  (RT-FR-17).
- `PaladinPort` gained two new default trait methods: `execute_observed(&self, paladin, input,
  heartbeat: &HeartbeatHandle)` (FT-FR-09) and `execute_scoped(&self, paladin, input, heartbeat,
  scope: &RunScope)` (RT-FR-13…RT-FR-16). Both default to delegating to the pre-existing method
  they wrap, so every existing implementor compiles and behaves unchanged.
- `ResumeAccepted` gained an additive `run_id: Option<RunId>` field with a `run_id()` accessor
  and a new `with_run_id(self, run_id) -> Self` constructor; struct is now `#[non_exhaustive]`
  (PLAT-03, PLAT-FR-06).
- `RunSubmissionPort::cancel` gained a `requested_by: Option<(String, UserRole)>` parameter
  (invocation-shaped authorization), and a new `fork(&self, request: ForkRun) -> Result<RunAccepted,
  RunSubmissionError>` method was added (PLAT-06, PLAT-FR-16).

See root `MIGRATION.md` §9.2 for the full X-10 compatibility register, mitigation, and
requirement IDs for each entry above.

## [0.9.0] - 2026-09-01

## [0.8.1-rc.5] - 2026-08-31

## [0.8.1-rc.4] - 2026-08-29

### Added
- Crate-level release artifacts for Epic 4 API stabilization.
- Hexagonal contract release notes tracking for input/output port changes.

### Changed
- Public contract stability documentation aligned with `STABLE_API.md`.

### Fixed
- Crate metadata and README linkage validated for crates.io release preparation.
