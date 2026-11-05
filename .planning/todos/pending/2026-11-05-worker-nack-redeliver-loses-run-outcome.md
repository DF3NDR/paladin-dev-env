---
created: 2026-11-05T00:00:00Z
title: "57-REVIEW CR-01: the worker's nack-and-redeliver loses a run's outcome when only the second write fails"
area: run-worker
severity: major
files:
  - .planning/phases/57-register-hygiene-close-out/57-REVIEW.md:75
  - src/application/services/run/worker.rs
owner: repo maintainer
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-01)
---

## Problem

57-REVIEW CR-01: the worker's nack-and-redeliver loses a run's outcome when only the second write fails.

Carried into the v0.11.1 pre-tag audit register as R-01; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: read src/application/services/run/worker.rs on 9eba21d9b: the completion block writes update_status (:1816) before record_outcome (:1824) and persist_failure writes Failed (:2101) before record_outcome (:2104); the status-first order the review names is unchanged

## What to do

target v0.12.0 (D-07, maintainer-owned): a behaviour change in a published crate.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner repo maintainer.

## Source

- Finding: 57-REVIEW.md:75 (CR-01); STATE Blockers/Concerns (Phase 57 close)
- Register row: R-01 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
