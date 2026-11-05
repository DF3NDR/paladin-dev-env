---
created: 2026-11-05T00:00:00Z
title: "The pricing-coverage check at graph registration and enumerating workflow assistants' node models for the boot warning"
area: treasurer
severity: minor
files:
  - crates/paladin-llm/src/pricing.rs
  - src/application/services/assistant/tests.rs
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-19)
---

## Problem

The pricing-coverage check at graph registration and enumerating workflow assistants' node models for the boot warning.

Carried into the v0.11.1 pre-tag audit register as R-19; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: ran gsd-tools windows status on 9eba21d9b: row 67 waived by Phase 55 (FIXES-06, D-07: the operator contract stands); the boot warning exists but no registration-time check does

## What to do

a later minor: needs a listing method on AssistantResolver.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner v0.12.0 Phase 1.

## Source

- Finding: 55 deferred-items.md Deferred by CONTEXT; WINDOWS row 67 original closing condition
- Register row: R-19 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
