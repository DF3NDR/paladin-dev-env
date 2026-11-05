---
created: 2026-11-05T00:00:00Z
title: "57-REVIEW IN-01: a doc comment is split across the wrong item in worker.rs"
area: docs
severity: info
files:
  - .planning/phases/57-register-hygiene-close-out/57-REVIEW.md:150
  - src/application/services/run/worker.rs
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-04)
---

## Problem

57-REVIEW IN-01: a doc comment is split across the wrong item in worker.rs.

Carried into the v0.11.1 pre-tag audit register as R-04; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: read src/application/services/run/worker.rs:139-145 on 9eba21d9b: the line about the graph_fingerprint a RunStarted event carries still sits directly above RUN_ERROR_CHAR_BUDGET's docs

## What to do

src/application/services/run/worker.rs doc-comment placement.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner v0.12.0 Phase 1.

## Source

- Finding: 57-REVIEW.md:150 (IN-01)
- Register row: R-04 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
