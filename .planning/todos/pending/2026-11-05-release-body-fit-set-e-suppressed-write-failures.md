---
created: 2026-11-05T00:00:00Z
title: "52 WR-01: set -e is suppressed inside the release-body size-fit functions, so write failures are swallowed"
area: release-tooling
severity: minor
files:
  - .planning/phases/52-ci-release-pipeline-repair/52-REVIEW.md:59
  - scripts/release-body-fit.sh
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-34)
---

## Problem

52 WR-01: set -e is suppressed inside the release-body size-fit functions, so write failures are swallowed.

Carried into the v0.11.1 pre-tag audit register as R-34; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: read scripts/release-body-fit.sh:274-276 on 9eba21d9b: cp -- in_file out_file is still unchecked before return 0

## What to do

check the writes explicitly in release-body-fit.sh, create-or-reuse-release.sh and finalize-release-body.sh; reproduced by the reviewer with a nonexistent output directory.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner v0.12.0 Phase 1.

## Source

- Finding: 52-REVIEW.md:59 (WR-01)
- Register row: R-34 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
