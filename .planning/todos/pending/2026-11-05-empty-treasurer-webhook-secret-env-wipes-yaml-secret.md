---
created: 2026-11-05T00:00:00Z
title: "57-REVIEW WR-01: an empty APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET silently wipes a YAML-configured secret"
area: config
severity: major
files:
  - .planning/phases/57-register-hygiene-close-out/57-REVIEW.md:105
  - src/config/treasurer.rs
owner: repo maintainer
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-02)
---

## Problem

57-REVIEW WR-01: an empty APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET silently wipes a YAML-configured secret.

Carried into the v0.11.1 pre-tag audit register as R-02; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: read src/config/treasurer.rs:869-875 on 9eba21d9b: webhook.secret = Some(v).filter(non-empty) still assigns None for an empty env value, discarding a YAML secret

## What to do

target v0.12.0 (D-07, maintainer-owned): a behaviour change in a published crate.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner repo maintainer.

## Source

- Finding: 57-REVIEW.md:105 (WR-01); STATE Blockers/Concerns (Phase 57 close)
- Register row: R-02 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
