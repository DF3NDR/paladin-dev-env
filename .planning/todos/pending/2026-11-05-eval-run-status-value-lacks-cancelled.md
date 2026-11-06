---
created: 2026-11-05T00:00:00Z
title: "FIXES-04: paladin-eval RunStatusValue has no Cancelled value"
area: paladin-eval
severity: minor
files:
  - crates/paladin-eval/src/scenario.rs
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-13)
---

## Problem

FIXES-04: paladin-eval RunStatusValue has no Cancelled value.

Carried into the v0.11.1 pre-tag audit register as R-13; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: read crates/paladin-eval/src/scenario.rs:566-578 on 9eba21d9b: RunStatusValue still has no Cancelled variant (the only mentions are the rustdoc at :566 and :572 naming the gap) and the enum is still exhaustive, so adding it is a major change at the patch gate

## What to do

Phase 55 D-05: one variant plus UPDATE_EVAL_SCHEMA=1 cargo test -p paladin-eval --test schema_golden.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner v0.12.0 Phase 1.

## Source

- Finding: 55 deferred-items.md (55-07); 55-VERIFICATION.md deferred; REQUIREMENTS.md FIXES-04 (unchecked, re-homed)
- Register row: R-13 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
