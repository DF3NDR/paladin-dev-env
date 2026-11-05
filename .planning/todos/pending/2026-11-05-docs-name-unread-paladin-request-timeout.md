---
created: 2026-11-05T00:00:00Z
title: "56 WR-08: docs name PALADIN_REQUEST_TIMEOUT, which no code reads, and the paladin-cli rename is partial"
area: docs
severity: minor
files:
  - .planning/phases/56-dogfood-findings/56-REVIEW.md:99
  - docs/src/appendix/cli-setup-check.md
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-52)
---

## Problem

56 WR-08: docs name PALADIN_REQUEST_TIMEOUT, which no code reads, and the paladin-cli rename is partial.

Carried into the v0.11.1 pre-tag audit register as R-52; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: read docs/src/appendix/cli-setup-check.md:367 on 9eba21d9b: the step 'PALADIN_REQUEST_TIMEOUT=10000 paladin-cli setup-check' is still there

## What to do

delete the timeout step or implement the variable; finish the rename or log the remainder as a finding.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner v0.12.0 Phase 1.

## Source

- Finding: 56-REVIEW.md:99 (WR-08)
- Register row: R-52 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
