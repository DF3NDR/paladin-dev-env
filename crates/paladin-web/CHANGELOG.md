# Changelog

All notable changes to `paladin-web` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project follows lockstep workspace versioning.

## [Unreleased]

## [0.10.0] - 2026-09-10

### Added
- `ThreadApiState` gained additive fields `runs: Option<Arc<dyn RunRepositoryPort>>` and
  `run_submission: Option<Arc<dyn RunSubmissionPort>>`, with `with_runs`/`with_run_submission`
  builders (struct is `#[non_exhaustive]`), wiring `GET /threads`, `GET /threads/{id}`,
  `POST /threads/{id}/fork` and `DELETE /threads/{id}` (PLAT-06, PLAT-FR-16).
- `ResumeAcceptedResponse` gained an additive `run_id: Option<String>` field and a new
  `ResumeAcceptedResponse::new(thread_id, state_url, run_id)` constructor (struct is
  `#[non_exhaustive]`) (PLAT-03, PLAT-FR-06).
- New route families answering `501 not_implemented` (never `404`) when their backing
  platform subsystem is disabled, per the v0.9-boots-unchanged contract: `/v1/runs`,
  `/v1/threads/*/history`, `/v1/assistants`, `/v1/schedules` (see root `MIGRATION.md` §9.5,
  §9.6).

### Changed
- `require_authentication` gained a generic type parameter (`pub async fn
  require_authentication<S>(...) where S: HasAgentAuth + Clone + Send + Sync + 'static`), was
  non-generic at v0.9.0 — a deliberate breaking change so `ThreadApiState`'s routes can reuse the
  same authentication middleware `AgentApiState`'s routes already layer (HITL-FR-16).

See root `MIGRATION.md` §9.2 for the full X-10 compatibility register and requirement IDs for
the entries above, in addition to the opaque bearer-token rename already recorded under
0.8.1-rc.4 below.

## [0.9.0] - 2026-09-01

## [0.8.1-rc.5] - 2026-08-31

## [0.8.1-rc.4] - 2026-08-29

### Added
- Crate-level release artifacts for Epic 4 API stabilization.
- Changelog tracking for web adapter and controller surface changes.

### Changed
- **BREAKING: the opaque bearer-token verifier field and the published OpenAPI security scheme
  are renamed.** `AgentAuthConfig`'s optional verifier field is renamed from `jwt` to
  `token_verifier` (same type, `Option<Arc<dyn AuthPort>>`). The OpenAPI security-scheme id served
  at `GET /openapi.json` changes value from `"jwt"` to `"bearer_token"`, and the scheme's
  `bearerFormat: "JWT"` hint is dropped entirely — an opaque token has no registered format, and
  the prior hint incorrectly implied a signed, self-describing token. **Remedy:** update any code
  constructing `AgentAuthConfig` to use `token_verifier` instead of `jwt`, and update any generated
  client that keys its security requirement off the `"jwt"` scheme id to use `"bearer_token"`. See
  [ADR-0040](../../.planning/decisions/0040-opaque-bearer-token-mechanism.md).
- Web API stability documentation aligned with crate-tier stability expectations.

### Fixed
- Crate metadata and README linkage validated for crates.io release preparation.
