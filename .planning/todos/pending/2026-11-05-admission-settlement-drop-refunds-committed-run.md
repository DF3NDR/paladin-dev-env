---
created: 2026-11-05T00:00:00Z
title: "57-REVIEW IN-03: AdmissionSettlement::drop can refund an admission for a run that then commits"
area: run-submission
severity: info
files:
  - .planning/phases/57-register-hygiene-close-out/57-REVIEW.md:168
  - src/application/services/run/submission.rs
  - src/application/services/treasurer/mod.rs
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-06)
---

## Problem

57-REVIEW IN-03: AdmissionSettlement::drop can refund an admission for a run that then commits.

Carried into the v0.11.1 pre-tag audit register as R-06; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: read src/application/services/treasurer/mod.rs:25-26 on 9eba21d9b: the documented residuals name only a process crash and a drop inside admit; the AdmissionSettlement drop race (submission.rs:194-262) is not listed

## What to do

src/application/services/run/submission.rs and treasurer/mod.rs residual list.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner v0.12.0 Phase 1.

## Source

- Finding: 57-REVIEW.md:168 (IN-03)
- Register row: R-06 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
