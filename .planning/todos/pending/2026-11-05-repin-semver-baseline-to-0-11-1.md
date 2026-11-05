---
created: 2026-11-05T00:00:00Z
title: "Re-pin the semver baseline to the released 0.11.1: SEMVER_BASELINE_VERSION in scripts/check-semver.sh, the ci.yml job..."
area: release-tooling
severity: minor
files:
  - scripts/check-semver.sh
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-78)
---

## Problem

Re-pin the semver baseline to the released 0.11.1: SEMVER_BASELINE_VERSION in scripts/check-semver.sh, the ci.yml job name and the ruleset context, renamed together.

Carried into the v0.11.1 pre-tag audit register as R-78; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: read scripts/check-semver.sh:86 and .github/workflows/ci.yml:409 on 9eba21d9b: SEMVER_BASELINE_VERSION is 0.11.0 and the job is named Semver Checks (vs v0.11.0); the ruleset context carries the same name

## What to do

rename the three together after 0.11.1 is on crates.io; renaming one alone makes the required context unresolvable.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner v0.12.0 Phase 1.

## Source

- Finding: plan 58-05 sweep (Research Pitfall 9)
- Register row: R-78 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
