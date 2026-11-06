---
created: 2026-11-05T00:00:00Z
title: "Retiring or partially retiring the base-layer items (todo options B/C, MIGRATION 12.2 rows, allowlist entries, a guar..."
area: core-base
severity: minor
files:
  - crates/paladin-core/src/base
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-18)
---

## Problem

Retiring or partially retiring the base-layer items (todo options B/C, MIGRATION 12.2 rows, allowlist entries, a guard rule) and the entity-sense identifier renames.

Carried into the v0.11.1 pre-tag audit register as R-18; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: read crates/paladin-core/src/base on 9eba21d9b: service/field_version_service.rs and service/collection_versioning_service.rs still exist, kept and fenced by FIXES-02 (55-BASE-LAYER-REVIEW.md); no retirement was done

## What to do

next minor, on the 55-12 review file's findings; finishing the content / general-data system is a feature milestone.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner v0.12.0 Phase 1.

## Source

- Finding: 55 deferred-items.md Deferred by CONTEXT; 55-BASE-LAYER-REVIEW.md (D-03)
- Register row: R-18 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
