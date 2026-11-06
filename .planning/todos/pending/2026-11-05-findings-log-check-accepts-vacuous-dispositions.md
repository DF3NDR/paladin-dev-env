---
created: 2026-11-05T00:00:00Z
title: "56 WR-02: check-findings-log.sh accepts vacuous final dispositions (out () (x), duplicate cycles, 'in v0.11.1 - TBD')"
area: release-tooling
severity: minor
files:
  - .planning/phases/56-dogfood-findings/56-REVIEW.md:50
  - scripts/check-findings-log.sh
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-46)
---

## Problem

56 WR-02: check-findings-log.sh accepts vacuous final dispositions (out () (x), duplicate cycles, 'in v0.11.1 - TBD').

Carried into the v0.11.1 pre-tag audit register as R-46; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: ran grep for cycle handling in scripts/check-findings-log.sh on 9eba21d9b: none; ran make check-findings-log: OK on 36 entries (the checker's blind spots do not affect the real log, read entry by entry in 56-10)

## What to do

anchor the reason regex, follow duplicate-of chains, reject TBD-style pointers.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner v0.12.0 Phase 1.

## Source

- Finding: 56-REVIEW.md:50 (WR-02)
- Register row: R-46 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
