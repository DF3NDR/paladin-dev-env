# Changelog

All notable changes to `paladin-herald` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project follows lockstep workspace versioning.

## [Unreleased]

## [0.10.0] - 2026-09-10

## [0.9.0] - 2026-09-01

## [0.8.1-rc.5] - 2026-08-31

## [0.8.1-rc.4] - 2026-08-29

### Added
- The crate itself, created by the 2026-06-04 facade-cleanup reconciliation (commit `66f6c4e`),
  extracting the `JsonHerald`, `MarkdownHerald` and `TableHerald` output-formatter adapters out of
  the facade into their own publishable, dependency-light leaf crate. The `Herald` trait itself
  remains in `paladin-core`; this crate holds only the formatter implementations.

### Changed
- **Breaking, default-features change (ADR-0023, Site 2).** `paladin-herald` gained its first
  `[features]` section: `default = []`, `table = ["dep:comfy-table"]`,
  `color = ["dep:colored"]`. `TableHerald` — whose rendering is entirely `comfy-table`
  (`Table`, `Cell`, `ContentArrangement`, presets) — now compiles only when the `table` feature is
  enabled. `MarkdownHerald`'s coloured rendering path (`use colored::*;`) now compiles only when the
  `color` feature is enabled; its existing `include_colors: false` runtime switch remains the
  uncoloured behaviour and needs no feature. `JsonHerald` is unaffected — it depends on neither
  `comfy-table` nor `colored`. A consumer taking `paladin-herald` with default features no longer
  compiles the `comfy-table` or `colored` dependencies, and no longer gets `TableHerald`'s table
  rendering or `MarkdownHerald`'s coloured rendering without opting into `table`/`color`. Neither
  `comfy_table::*` nor `colored::*` appears in any public function signature or public struct field,
  so this is an additive feature split rather than a signature-breaking change.

### Fixed
- N/A
