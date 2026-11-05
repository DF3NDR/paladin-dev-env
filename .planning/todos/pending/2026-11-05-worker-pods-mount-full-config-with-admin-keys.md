---
created: 2026-11-05T00:00:00Z
title: "54 IN-02: worker pods mount the full config including literal admin API keys they never use"
area: k8s
severity: minor
files:
  - .planning/phases/54-first-run-usability/54-REVIEW.md:192
  - k8s/README.md
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-30)
---

## Problem

54 IN-02: worker pods mount the full config including literal admin API keys they never use.

Carried into the v0.11.1 pre-tag audit register as R-30; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: read k8s/server/worker-deployment.yaml:145-154 on 9eba21d9b: the worker still mounts secret paladin-server-secrets, items config.yml

## What to do

give workers a separate Secret key with auth disabled, or note the exposure in k8s/README.md.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner v0.12.0 Phase 1.

## Source

- Finding: 54-REVIEW.md:192 (IN-02); STATE Blockers/Concerns (Phase 54 close)
- Register row: R-30 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
