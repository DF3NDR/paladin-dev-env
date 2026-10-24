# Changelog

All notable changes to `paladin-ports` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project follows lockstep workspace versioning.

## [Unreleased]

### Added

- Phase 51 (REASON-01, REASON-04; plans 51-17 to 51-20, gap G-2): the provided trait method
  `LlmPort::check_reasoning(&self, model, effort, fallback) -> Result<(), LlmError>`, which
  pre-judges without a network call whether the port's send-time reasoning check would admit that
  effort for that model. The default body is the earlier builder rule (the model's profile alone,
  no provider defaults; a port with no profile refuses an explicit effort), so every implementor
  compiles unchanged. A decorator should forward it as it forwards `model_profile`. Additive: no
  lint against the published 0.10.1 names it.

- Phase 51 (REASON-01, REASON-02, REASON-04, REASON-06; plan 51-01):
  `output::model_profile::{EffortSupport, EffortLevels, BudgetBounds, ResolvedEffort,
  EffortRefusal}` with `EffortSupport::resolve` (the one admission rule) and
  `EffortSupport::intersect` (AND semantics, commutative and associative); `ModelProfile.effort`
  and `effort_verified_on` with `with_effort` / `with_effort_verified_on`;
  `LlmRequest.reasoning` and `reasoning_fallback` with `with_reasoning` /
  `with_reasoning_fallback`; `LlmError::UnsupportedParameter` and
  `LlmError::unsupported_parameter` (`Permanent`); `ReasoningEffort` and `ReasoningFallback`
  re-exported on `output::llm_port`. Additive: every touched type is `#[non_exhaustive]`. The
  nine unset-effort wire bodies are pinned by golden files in `paladin-llm`.

- Phase 50 (G-50-5a, PARITY-05; plans 50-25 and 50-26): the additive half of the billed-failure
  change. `output::llm_port::BilledUsage` (`new`, `with_cost`), the constructors
  `LlmError::empty_completion` and `LlmError::refusal`, and `LlmError::with_billed_usage` /
  `LlmError::billed_usage` (the latter looks through `AllProvidersFailed`), each with a doc
  example. The variant reshaping they go with is under *Breaking Changes*. Every adapter fills the
  bill, and the execution service prices and settles it (plans 50-27 to 50-30).

- Phase 50 (G-50-5a remainder (a), PARITY-05; plan 50-30): `output::spend_scope::HoppedBills` (a
  shared, ordered list of the `BilledUsage` a fallback chain moved past during one model call;
  `new`, `push`, `price_unpriced` (idempotent: only entries at `cost: None`), `take`; a poisoned
  lock is recovered, never unwrapped), the `tokio::task_local!` `HOPPED_BILLS` and
  `record_hopped_bill(billed) -> bool` (`false` with no collector installed). Installed per model
  call by `PaladinExecutionService`, filled by `FallbackLlmAdapter`, priced by
  `PricingLlmAdapter`; entries hold a model id, token counts and a `Cost` only.

- Phase 50 (G-50-5a, review WR-03, PARITY-05, PARITY-06; plan 50-32): two additive methods on
  `output::spend_scope::HoppedBills`. `install_pricer(pricer) -> bool` installs the pricer the
  collector applies to every unpriced bill it records (the first pricer installed wins, so the
  outermost `PricingLlmAdapter`'s table prices every hop bill; it returns whether this call
  installed it). `count_and_take() -> Vec<BilledUsage>` takes the entries and counts into the
  attempt scope only those not yet counted. `record_hopped_bill` now prices the bill through the
  installed pricer and counts it into `ATTEMPT_SPEND` at once, so an attempt dropped while a later
  hop of the same call is in flight is still charged for the earlier hop; with no pricer the bill
  counts with `cost: None`. `price_unpriced` stays as the after-call safety net. `spend_scope` is
  unreleased, so this is not a break.

- Phase 50 (G-50-5a remainder (b), PARITY-05; plan 50-29): the `output::spend_scope` module --
  `SpendScope` (a shared accumulator of `paladin_core`'s `IncurredSpend`; `new`, `record`,
  `snapshot`; a poisoned lock is recovered, never unwrapped), the `tokio::task_local!`
  `ATTEMPT_SPEND`, `record_incurred_spend(usage, cost) -> bool` (the write side; `false` outside a
  scope) and `current_attempt_spend() -> Option<IncurredSpend>` (the read side). It follows the
  `RUN_TRACE_EMITTER` precedent: the Battalion engine installs a fresh scope around every attempt,
  `PaladinExecutionService` records every billed call at the point it folds it, and the engine
  reads the scope for a failed, timed-out or structured attempt. A direct `PaladinPort` caller that
  wants a failed execution's spend installs a scope itself (`ATTEMPT_SPEND.scope(scope.clone(),
  fut).await`, then `scope.snapshot()`).

- Phase 50 (provider parity and configuration-driven model swap; PARITY-02, PARITY-05; ADR-0061):
  `ModelProfile`, `ParamSupport` (`Range`, `Fixed`, `NotSent`), `StopSupport` and `SamplingParam`
  in `output::model_profile` (re-exported from `output::llm_port`), and the defaulted
  `LlmPort::model_profile(&self, model)`, which returns `None` so no implementor breaks and
  `ProviderCapabilities` is untouched. `ParamSupport::admits` is the single admission rule
  (inclusive range, exact `Fixed` equality, `NaN` never admitted, no clamping); `ModelProfile` is
  `#[non_exhaustive]` and `ModelProfile::new()` is fail closed. `LlmError::Refusal { provider,
  model, detail }`, a vendor's own refusal (`Permanent`), distinct from a `ContentFilter` finish
  and from `EmptyCompletion`. `LlmResponse::new(request_id, model, content, finish_reason, usage)`
  and the chainable `LlmResponse::with_reasoning`, which insulate a caller from the next added
  field; `StreamingResponse.reasoning` and `StreamingResponse.metadata` with `with_reasoning` and
  `with_metadata` (additive on a `#[non_exhaustive]` struct), `ChunkMetadata.run_metadata` with
  `with_run_metadata`, and `PaladinResult.run_metadata` (defined in `paladin-core`, re-exported
  here) carrying the run's `paladin.*` visibility keys (owner reading R-1).

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

- Phase 50: `LlmError::EmptyCompletion`'s `Display` is `Empty completion: {0}` without a fixed
  suffix; the payload now carries the advice. `LlmResponse.usage` still guarantees
  `total_tokens == prompt_tokens + completion_tokens`.

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

### Breaking Changes

- Phase 50 (PARITY-05, owner reading R-1): `LlmResponse` gains `reasoning: Option<String>`
  directly after `cost`, so a full `LlmResponse` struct literal no longer compiles; add
  `reasoning: None`, or build with `LlmResponse::new(..)` plus `with_reasoning`. `PaladinResult`
  gains `run_metadata` after `served_by`; use `..Default::default()` or `PaladinResult::new(..)`.
  `LlmError` gains the `Refusal` variant (the enum is `#[non_exhaustive]`; a classifier needs an
  arm). Each is registered in `MIGRATION.md` section 9.2 and the semver allowlist.
- Phase 50 (G-50-5a, PARITY-05; plans 50-25 and 50-26): `LlmError::EmptyCompletion(String)`
  becomes the `#[non_exhaustive]` struct variant `EmptyCompletion { detail, billed }`, and
  `LlmError::Refusal` gains `billed` and is marked `#[non_exhaustive]`. `billed:
  Option<Box<BilledUsage>>` carries the usage of a call the vendor billed although it failed. The
  new `BilledUsage { model, usage, cost }` has `BilledUsage::new` and `with_cost`. Build the
  variants with the new `LlmError::empty_completion` / `LlmError::refusal`, attach a bill with
  `LlmError::with_billed_usage`, and read it with `LlmError::billed_usage`. Both look through
  `AllProvidersFailed` to its last error. Match with `EmptyCompletion { .. }` / `Refusal { .. }`.
  `Display` and `Permanent` are unchanged. Registered in `MIGRATION.md` section 9.2 (the
  `LlmError` row) and the semver allowlist (`enum_tuple_variant_changed_kind`).

## [0.10.1] - 2026-09-21

Patch release carried by the workspace-wide version bump (0.10.0 -> 0.10.1). No source
change in this crate — see the root `CHANGELOG.md`'s `[0.10.1]` section for the two
release-pipeline defects this patch fixes.

## [0.10.0] - 2026-09-18

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
