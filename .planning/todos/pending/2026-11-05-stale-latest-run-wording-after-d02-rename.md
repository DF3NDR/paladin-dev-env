---
created: 2026-11-05T00:00:00Z
title: "57-REVIEW IN-02: stale 'latest run' wording left behind by the D-02 rename"
area: docs
severity: info
files:
  - .planning/phases/57-register-hygiene-close-out/57-REVIEW.md:162
  - crates/paladin-core/src/platform/container/principal.rs
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-05)
---

## Problem

57-REVIEW IN-02: stale 'latest run' wording left behind by the D-02 rename.

Carried into the v0.11.1 pre-tag audit register as R-05; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: read crates/paladin-core/src/platform/container/principal.rs:10-16 on 9eba21d9b: the module doc still says a thread belongs to the tenant of its latest run (:14)

## What to do

crates/paladin-core/src/platform/container/principal.rs module doc.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner v0.12.0 Phase 1.

## Source

- Finding: 57-REVIEW.md:162 (IN-02)
- Register row: R-05 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
