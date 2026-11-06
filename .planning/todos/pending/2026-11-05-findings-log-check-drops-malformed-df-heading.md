---
created: 2026-11-05T00:00:00Z
title: "56 WR-01: check-findings-log.sh silently drops a malformed ### DF- heading (confirmed false pass)"
area: release-tooling
severity: minor
files:
  - .planning/phases/56-dogfood-findings/56-REVIEW.md:36
  - scripts/check-findings-log.sh
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-45)
---

## Problem

56 WR-01: check-findings-log.sh silently drops a malformed ### DF- heading (confirmed false pass).

Carried into the v0.11.1 pre-tag audit register as R-45; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: ran grep for LOOSE_HEAD, BAD_HEADING and cycle in scripts/check-findings-log.sh on 9eba21d9b: no match, the loose-heading guard the review asks for is not there

## What to do

treat any ### DF- heading that fails the strict form as a named failure and add a fixture.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner v0.12.0 Phase 1.

## Source

- Finding: 56-REVIEW.md:36 (WR-01)
- Register row: R-45 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
