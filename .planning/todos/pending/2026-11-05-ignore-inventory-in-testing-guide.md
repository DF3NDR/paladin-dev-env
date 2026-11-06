---
created: 2026-11-05T00:00:00Z
title: "An #[ignore] inventory in the testing guide (E8, DOCS-09)"
area: docs
severity: minor
files:
  - docs/src/contributing/testing-guide.md
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-21)
---

## Problem

An #[ignore] inventory in the testing guide (E8, DOCS-09).

Carried into the v0.11.1 pre-tag audit register as R-21; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: read docs/src/contributing/testing-guide.md on 9eba21d9b: it explains that #[ignore]-gated tests sit outside the coverage numerator and denominator (:552) but has no inventory of the ignored sites

## What to do

a docs pass: list the ignored sites and what each needs (API key, Docker service, Qdrant, MySQL, live MCP).

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner v0.12.0 Phase 1.

## Source

- Finding: 55 deferred-items.md Deferred by CONTEXT; inventory 01-carried-gaps-inventory.md E8
- Register row: R-21 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
