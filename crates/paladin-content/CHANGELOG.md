# Changelog

All notable changes to `paladin-content` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project follows lockstep workspace versioning.

## [Unreleased]

## [0.10.0] - 2026-09-10

No functional changes; lockstep version bump (see root `CHANGELOG.md`). The only commits
touching this crate between the `v0.9.0` tag and this release raise the workspace MSRV floor
(now `1.88`, §9.3) — no source file under `src/` changed.

## [0.9.0] - 2026-09-01

## [0.8.1-rc.5] - 2026-08-31

## [0.8.1-rc.4] - 2026-08-29

### Added
- Crate-level release artifacts for Epic 4 API stabilization.
- Feature-flag release notes tracking for content adapters (`pdf`, `web-scraping`, `rss`, `news-api`, `tiktoken`, `llm`).

### Changed
- Content API stability documentation aligned with crate-tier stability expectations.

### Removed
- The `pdf` feature flag. It gated no dependency (`pdf-extract` was always an unconditional
  dependency of this crate) and no code (no `#[cfg(feature = "pdf")]` site existed anywhere in
  `src/`). PDF extraction is unaffected and remains unconditional in every build of
  `paladin-content`. **Consumer-visible cost:** `cargo build -p paladin-content --features pdf`
  begins to fail where it previously succeeded and did nothing. See
  [ADR-0032](../../.planning/decisions/0032-pdf-extraction-capability.md).

### Fixed
- Crate metadata and README linkage validated for crates.io release preparation.
