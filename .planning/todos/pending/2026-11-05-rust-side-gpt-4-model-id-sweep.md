---
created: 2026-11-05T00:00:00Z
title: "The deprecated gpt-4 model id in crates/*/src, src/, crates/doc-examples and tests/ (the GarrisonConfig default token..."
area: docs-models
severity: minor
files:
  - tests/fixtures/config/v0.9.0-config.test.yml
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-79)
---

## Problem

The deprecated gpt-4 model id in crates/*/src, src/, crates/doc-examples and tests/ (the GarrisonConfig default tokenizer at crates/paladin-memory/src/config/garrison.rs:35,106 among them).

Carried into the v0.11.1 pre-tag audit register as R-79; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: ran grep for the quoted id over src crates tests on 9eba21d9b: 643 sites in 126 .rs files (663 lines in 137 files of any type); the frozen tests/fixtures/config/v0.9.0-config.test.yml (:19, :76) stays as a historical fixture

## What to do

a Rust-side sweep with the registry-membership gate; the frozen v0.9.0 fixture is named as staying.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner v0.12.0 Phase 1.

## Source

- Finding: 58-03 hand-off; plan 58-05 sweep
- Register row: R-79 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
