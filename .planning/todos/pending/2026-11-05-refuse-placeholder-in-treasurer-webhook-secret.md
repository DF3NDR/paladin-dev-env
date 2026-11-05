---
created: 2026-11-05T00:00:00Z
title: "A ${...} placeholder in the Treasurer webhook secret is not refused (54 WR-04)"
area: config
severity: minor
files:
  - .planning/phases/54-first-run-usability/54-REVIEW.md:160
  - src/config/treasurer.rs
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-10)
---

## Problem

A ${...} placeholder in the Treasurer webhook secret is not refused (54 WR-04).

Carried into the v0.11.1 pre-tag audit register as R-10; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: read src/config/treasurer.rs:1494-1504 on 9eba21d9b: a treasurer webhook secret of ${ALLOWANCE_WEBHOOK_SECRET} loads literally and a test asserts it; no refusal exists

## What to do

new behaviour: refuse a ${...} secret at load and pin the other shipped example configs by test.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner v0.12.0 Phase 1.

## Source

- Finding: 57-REGISTER-EVIDENCE.md Orphaned deferrals; 54-REVIEW.md:160 (WR-04); 55 deferred-items.md Deferred by CONTEXT
- Register row: R-10 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
