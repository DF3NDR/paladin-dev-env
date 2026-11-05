---
created: 2026-11-05T00:00:00Z
title: "The bare-host coverage path (make services-up, password-protected Redis, trap 2) is still unwalked"
area: coverage
severity: minor
files:
  - Makefile
  - docs/src/contributing/testing-guide.md
owner: repo maintainer
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-55)
---

## Problem

The bare-host coverage path (make services-up, password-protected Redis, trap 2) is still unwalked.

Carried into the v0.11.1 pre-tag audit register as R-55; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: read 56-UAT.md test 2: result pass on the devcontainer figure (local 92.79 percent, CI 92.86 percent); 56-VERIFICATION.md human_verification lists guide step 1 and trap 2 as not exercised

## What to do

walk make services-up then make coverage on a Docker-capable bare host and record the figure.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner repo maintainer.

## Source

- Finding: 56-VERIFICATION.md human_verification (DOGF-05 scope); STATE Blockers/Concerns (Phase 56 close)
- Register row: R-55 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
