---
created: 2026-11-05T00:00:00Z
title: "57-REVIEW IN-05: the fence marker TTL cap reopens stale-writer overwrite for long-lived node-cache entries"
area: storage
severity: info
files:
  - .planning/phases/57-register-hygiene-close-out/57-REVIEW.md:180
  - crates/paladin-storage/src/redis_fence.rs
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-08)
---

## Problem

57-REVIEW IN-05: the fence marker TTL cap reopens stale-writer overwrite for long-lived node-cache entries.

Carried into the v0.11.1 pre-tag audit register as R-08; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: read crates/paladin-storage/src/redis_fence.rs:45-64 on 9eba21d9b: the 5 to 30 minute clamp and its trade-off for entry TTLs above 15 minutes are documented as accepted by ADR-0058 (g); no code changed

## What to do

crates/paladin-storage/src/redis_fence.rs: no change if ADR-0058 (g) holds; record an ADR exception only if a node can produce a non-deterministic result.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner v0.12.0 Phase 1.

## Source

- Finding: 57-REVIEW.md:180 (IN-05)
- Register row: R-08 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
