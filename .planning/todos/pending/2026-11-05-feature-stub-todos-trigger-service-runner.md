---
created: 2026-11-05T00:00:00Z
title: "Feature-stub TODOs in trigger.rs (JSONPath payload matching, cooldown), service_runner.rs (notification adapter wrapp..."
area: run-services
severity: minor
files:
  - crates/paladin-core/src/platform/container/trigger.rs
  - crates/paladin-battalion/src/council_service.rs
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-22)
---

## Problem

Feature-stub TODOs in trigger.rs (JSONPath payload matching, cooldown), service_runner.rs (notification adapter wrappers) and council_service.rs (registry parameter) (E9).

Carried into the v0.11.1 pre-tag audit register as R-22; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: read crates/paladin-core/src/platform/container/trigger.rs:216,261 and crates/paladin-battalion/src/council_service.rs:734 on 9eba21d9b: the TODO comments are still there; none is a reachable panic (FIXES-01 guard held in check-gates)

## What to do

feature work, not a reachable panic.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner v0.12.0 Phase 1.

## Source

- Finding: 55 deferred-items.md Deferred by CONTEXT; inventory E9
- Register row: R-22 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
