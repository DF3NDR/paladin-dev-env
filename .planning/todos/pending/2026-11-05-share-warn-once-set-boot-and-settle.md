---
created: 2026-11-05T00:00:00Z
title: "Sharing the warn-once set between boot and settle time through an additive pub fn in paladin-llm"
area: llm
severity: minor
files:
  - crates/paladin-llm/src/pricing.rs
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-20)
---

## Problem

Sharing the warn-once set between boot and settle time through an additive pub fn in paladin-llm.

Carried into the v0.11.1 pre-tag audit register as R-20; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: read crates/paladin-llm/src/pricing.rs on 9eba21d9b: UnpricedModelWarnings appears only in this file (grep over src and crates), so the boot and settle-time sets are still separate

## What to do

only if a plan finds it free; otherwise a later minor.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner v0.12.0 Phase 1.

## Source

- Finding: 55 deferred-items.md Deferred by CONTEXT (D-07)
- Register row: R-20 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
