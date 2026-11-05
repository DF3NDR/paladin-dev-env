---
created: 2026-11-05T00:00:00Z
title: "56 WR-05: onboarding reports 'All API keys validated successfully!' after failed validations"
area: cli
severity: minor
files:
  - .planning/phases/56-dogfood-findings/56-REVIEW.md:81
  - src/application/cli/commands/onboarding.rs
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-49)
---

## Problem

56 WR-05: onboarding reports 'All API keys validated successfully!' after failed validations.

Carried into the v0.11.1 pre-tag audit register as R-49; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: read src/application/cli/commands/onboarding.rs:338 on 9eba21d9b: formatter.success('All API keys validated successfully!') is still unconditional

## What to do

count failures and print the success line only at zero.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner v0.12.0 Phase 1.

## Source

- Finding: 56-REVIEW.md:81 (WR-05)
- Register row: R-49 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
