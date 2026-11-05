---
created: 2026-11-05T00:00:00Z
title: "52 WR-02: check-migration-allowlist.sh silently passes when the 11.2 heading is missing or renamed"
area: release-tooling
severity: minor
files:
  - .planning/phases/52-ci-release-pipeline-repair/52-REVIEW.md:76
  - scripts/check-migration-allowlist.sh
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-35)
---

## Problem

52 WR-02: check-migration-allowlist.sh silently passes when the 11.2 heading is missing or renamed.

Carried into the v0.11.1 pre-tag audit register as R-35; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: read scripts/check-migration-allowlist.sh:97 on 9eba21d9b: the window opens on /^## 11\.2 / with no exactly-one check on the heading

## What to do

require exactly one window-opening heading and add a harness case.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner v0.12.0 Phase 1.

## Source

- Finding: 52-REVIEW.md:76 (WR-02)
- Register row: R-35 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
