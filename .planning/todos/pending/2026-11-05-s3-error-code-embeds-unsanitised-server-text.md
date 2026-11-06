---
created: 2026-11-05T00:00:00Z
title: "57-REVIEW IN-04: s3_error_code embeds up to 64 characters of server-controlled text unsanitised"
area: storage
severity: info
files:
  - .planning/phases/57-register-hygiene-close-out/57-REVIEW.md:174
  - crates/paladin-storage/src/minio.rs
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-07)
---

## Problem

57-REVIEW IN-04: s3_error_code embeds up to 64 characters of server-controlled text unsanitised.

Carried into the v0.11.1 pre-tag audit register as R-07; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: read crates/paladin-storage/src/minio.rs:107-112 on 9eba21d9b: s3_error_code still takes chars().take(64) with no character filter

## What to do

crates/paladin-storage/src/minio.rs s3_error_code character filter.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner v0.12.0 Phase 1.

## Source

- Finding: 57-REVIEW.md:174 (IN-04)
- Register row: R-07 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
