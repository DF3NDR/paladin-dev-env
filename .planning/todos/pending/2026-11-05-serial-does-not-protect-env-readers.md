---
created: 2026-11-05T00:00:00Z
title: "56 WR-06: #[serial] does not protect against concurrent env readers (unsafe set_var in edition 2024)"
area: test-hygiene
severity: minor
files:
  - .planning/phases/56-dogfood-findings/56-REVIEW.md:87
  - src/bin/paladin-server.rs
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-50)
---

## Problem

56 WR-06: #[serial] does not protect against concurrent env readers (unsafe set_var in edition 2024).

Carried into the v0.11.1 pre-tag audit register as R-50; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: read src/bin/paladin-server.rs:848 on 9eba21d9b: OVERLAY_PREFIXES is still [PALADIN_TRACE_, APP_AGENT_RUNTIME_, APP_WEB_SERVER_] and does not include APP_TREASURER_

## What to do

mark every env-reading test serial or move env-mutating tests to their own binary.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner v0.12.0 Phase 1.

## Source

- Finding: 56-REVIEW.md:87 (WR-06)
- Register row: R-50 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
