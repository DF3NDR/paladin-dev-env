# Changelog

All notable changes to `paladin-notifications` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project follows lockstep workspace versioning.

## [Unreleased]

## [0.11.1] - 2026-11-05

### Security

- `EmailAdapterConfig`'s `Debug` (feature `email`) prints the SMTP `password` as `<redacted>` (`""` when empty) instead of the secret, so `EmailNotificationAdapter`'s derived `Debug`, a panic message or a log line that formats the config no longer carries it. No field, type or signature changes (`MIGRATION.md` section 11.1 `M-B-36`; DF-07).

- `EmailAdapterConfig.password` (feature `email`) is no longer serialised (`#[serde(skip_serializing)]`), so a dump of a loaded configuration omits the SMTP password; loading still requires the field. No field, type or signature changes (`MIGRATION.md` section 11.1 `M-B-50`; 56-REVIEW WR-09).

## [0.11.0] - 2026-10-26

No functional change; version bumped in lockstep with the workspace.

## [0.10.1] - 2026-09-21

Patch release carried by the workspace-wide version bump (0.10.0 -> 0.10.1). No source
change in this crate — see the root `CHANGELOG.md`'s `[0.10.1]` section for the two
release-pipeline defects this patch fixes.

## [0.10.0] - 2026-09-18

No functional changes; lockstep version bump (see root `CHANGELOG.md`). The only commits
touching this crate between the `v0.9.0` tag and this release raise the workspace MSRV floor
(now `1.88`, §9.3) — no source file under `src/` changed.

## [0.9.0] - 2026-09-01

## [0.8.1-rc.5] - 2026-08-31

## [0.8.1-rc.4] - 2026-08-29

### Added
- Crate-level release artifacts for Epic 4 API stabilization.
- Feature-flag release notes tracking for notification channels (`email`, `push`, `system`).

### Changed
- Notification API stability documentation aligned with crate-tier stability expectations.

### Fixed
- Crate metadata and README linkage validated for crates.io release preparation.
