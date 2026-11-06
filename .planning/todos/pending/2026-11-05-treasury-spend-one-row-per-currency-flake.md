---
created: 2026-11-05T00:00:00Z
title: "F-58-03: treasury_spend_prints_one_row_per_currency is a test-level flake"
area: test-hygiene
severity: minor
files:
  - src/application/cli/commands/treasury.rs
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-77)
---

## Problem

F-58-03: treasury_spend_prints_one_row_per_currency is a test-level flake.

Carried into the v0.11.1 pre-tag audit register as R-77; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: read 58-CI-EVIDENCE.md F-58-03 on 9eba21d9b: one CI failure (run 41946243320, job 126979605660, treasury.rs:496:9 assertion failed: rendered.contains("EUR")), one pass on the same file (run 41949135304), local 0/30 single and 0/10 module; Build & Test (cli) 127040405991 success on 33cdc08e0

## What to do

a test fix under src/application/cli/; the re-seal rule is the maintainer's Task 3 answer (FLAKE).

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner v0.12.0 Phase 1.

## Source

- Finding: 58-CI-EVIDENCE.md F-58-03; plan 58-01
- Register row: R-77 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
