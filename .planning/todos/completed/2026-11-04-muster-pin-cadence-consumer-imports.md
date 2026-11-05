---
completed: 2026-11-05
created: 2026-11-04T00:00:00Z
title: Pin the three Cadence import paths web3sec takes in tests/consumer_surface_muster.rs
area: downstream-consumer
severity: minor
files:
  - tests/consumer_surface_muster.rs
  - MIGRATION.md (section 11.2, only if the pin set changes the documented surface)
owner: repo maintainer
dispositioned_by: Phase 56 plan 56-10 (DF-36)
---

## Problem

web3sec's return (`.planning/phases/56-dogfood-findings/56-WEB3SEC-RETURN-REPORT-50.md`, section 4) lists three
`use` paths its Phase 50 code takes that `tests/consumer_surface_muster.rs` does not pin:

- `paladin::infrastructure::cadence::build_cadence`
- `paladin::infrastructure::cadence::with_cadence`
- `paladin::config::treasurer::CadenceConfig` (`CadenceConfig::default()`)

(`LlmError::Refusal { .. }` and `LlmError::RateLimitExceeded { .. }` from the same list are already pinned.)

## What is already true

All three resolve on HEAD (`src/infrastructure/cadence.rs:86` re-exports `with_cadence`, `:128` defines
`build_cadence`; `src/config/treasurer.rs:124` defines `CadenceConfig`) and MIGRATION section 10.8's Cadence recipe
names them. They are not unguarded: the `build_cadence` doctest compiles `CadenceConfig` and `build_cadence`, and
`.project/current-exports.txt` (the `make api-surface` baseline) lists `build_cadence`, `with_cadence` and
`CadenceConfig`. The gap is only that the muster, the file that exists to mirror the consumer's imports, does not
say so.

## What to do

Add a `use` plus one never-called pin for each path, in the same style as the existing pins (compile-time only, no
execution), then run `make check-consumer-surface`. It needs a compile of the facade with `llm-anthropic` and
`llm-deepseek`, which Phase 56 plan 56-10 did not run (docs-only plan, no cargo build, 3.3 GB free at the time).

Entry condition: a milestone with room for a one-file test change. Not a release blocker.

## Resolved 2026-11-05 (Phase 58)

Plan 58-04 added `cadence_consumer_imports_pin` to `tests/consumer_surface_muster.rs`, pinning `build_cadence`, `with_cadence`
and `CadenceConfig` (commit `364f08607` test(58-04)). The Consumer-Surface Muster run 127040941941 concluded `success` on
`33cdc08e0`. Register row R-61 (`fixed`), closed by plan 58-06.
