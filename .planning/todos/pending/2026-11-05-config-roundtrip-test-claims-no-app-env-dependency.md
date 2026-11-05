---
created: 2026-11-05T00:00:00Z
title: "56 WR-07: the config round-trip test claims no APP_* dependency but calls Settings::new()"
area: test-hygiene
severity: minor
files:
  - .planning/phases/56-dogfood-findings/56-REVIEW.md:93
  - tests/config_example_roundtrip.rs
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-51)
---

## Problem

56 WR-07: the config round-trip test claims no APP_* dependency but calls Settings::new().

Carried into the v0.11.1 pre-tag audit register as R-51; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: read tests/config_example_roundtrip.rs:21 and :364 on 9eba21d9b: the module doc still says the file depends on no APP_* variable while full_fixture_round_trips_through_settings_new calls Settings::new()

## What to do

run the test under a cleared-APP_* guard or correct the doc claim.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner v0.12.0 Phase 1.

## Source

- Finding: 56-REVIEW.md:93 (WR-07)
- Register row: R-51 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
