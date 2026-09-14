# Changelog

All notable changes to `paladin-ports` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project follows lockstep workspace versioning.

## [Unreleased]

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
