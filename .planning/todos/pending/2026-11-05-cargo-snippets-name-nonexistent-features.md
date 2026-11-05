---
created: 2026-11-05T00:00:00Z
title: "54 WR-01: copy-paste Cargo snippets name features that do not exist"
area: docs
severity: minor
files:
  - .planning/phases/54-first-run-usability/54-REVIEW.md:109
  - docs/src/api-reference/crate-map.md
  - docs/src/getting-started/installation.md
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-26)
---

## Problem

54 WR-01: copy-paste Cargo snippets name features that do not exist.

Carried into the v0.11.1 pre-tag audit register as R-26; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: ran cargo metadata --no-deps on 9eba21d9b: paladin-ai has no storage-sqlite feature, yet docs/src/api-reference/crate-map.md:117,119,201 still name it; docs/src/getting-started/installation.md:57 now reads features = ["openai"] (that half is fixed)

## What to do

drop storage-sqlite from crate-map.md:117,119,201 and extend check-version-strings to verify features = [...] entries against cargo metadata.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner v0.12.0 Phase 1.

## Source

- Finding: 54-REVIEW.md:109 (WR-01); STATE Blockers/Concerns (Phase 54 close)
- Register row: R-26 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
