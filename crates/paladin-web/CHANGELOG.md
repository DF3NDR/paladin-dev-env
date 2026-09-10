# Changelog

All notable changes to `paladin-web` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project follows lockstep workspace versioning.

## [Unreleased]

## [0.10.0] - 2026-09-10

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
