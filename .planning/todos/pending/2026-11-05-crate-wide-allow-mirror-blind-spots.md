---
created: 2026-11-05T00:00:00Z
title: "52 WR-03: the crate-wide-allow mirror cannot see warn, workspace metadata or non-double-quoted forms"
area: release-tooling
severity: minor
files:
  - .planning/phases/52-ci-release-pipeline-repair/52-REVIEW.md:93
  - scripts/check-migration-allowlist.sh
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-36)
---

## Problem

52 WR-03: the crate-wide-allow mirror cannot see warn, workspace metadata or non-double-quoted forms.

Carried into the v0.11.1 pre-tag audit register as R-36; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: read scripts/check-migration-allowlist.sh:155-175 on 9eba21d9b: the scan matches only [package.metadata.cargo-semver-checks.lints] with a value written exactly as "allow"

## What to do

treat any key in any cargo-semver-checks metadata table as an override that needs a mirrored entry.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner v0.12.0 Phase 1.

## Source

- Finding: 52-REVIEW.md:93 (WR-03)
- Register row: R-36 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
