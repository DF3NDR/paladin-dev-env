---
created: 2026-11-05T00:00:00Z
title: "A registry-membership model check for the other ids (gpt-4o, gpt-3.5-turbo, gpt-4-turbo and the rest): serving status..."
area: docs-models
severity: minor
files:
  - scripts/check-doc-models.sh
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-80)
---

## Problem

A registry-membership model check for the other ids (gpt-4o, gpt-3.5-turbo, gpt-4-turbo and the rest): serving status was never measured.

Carried into the v0.11.1 pre-tag audit register as R-80; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: read 58-03-SUMMARY.md Hand-offs on 9eba21d9b: gpt-4o 66 sites, gpt-3.5-turbo 6, gpt-4-turbo 3 (58-03 figures, not re-measured here)

## What to do

extend scripts/check-doc-models.sh to check every copyable id against the seeded registry.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner v0.12.0 Phase 1.

## Source

- Finding: 58-03 hand-off; plan 58-05 sweep
- Register row: R-80 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
