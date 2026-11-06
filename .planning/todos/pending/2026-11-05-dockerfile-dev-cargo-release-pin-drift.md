---
created: 2026-11-05T00:00:00Z
title: ".devcontainer/Dockerfile.dev pins cargo-release 1.1.2 while releases since plan 47-01 use 1.1.7"
area: devcontainer
severity: minor
files:
  - release.toml:49
  - release.toml
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-82)
---

## Problem

.devcontainer/Dockerfile.dev pins cargo-release 1.1.2 while releases since plan 47-01 use 1.1.7.

Carried into the v0.11.1 pre-tag audit register as R-82; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: read .devcontainer/Dockerfile.dev:129 and release.toml:49 on 9eba21d9b: the image pins --version 1.1.2, release.toml records 1.1.7

## What to do

align the Dockerfile.dev pin with release.toml:49 in a devcontainer change.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner v0.12.0 Phase 1.

## Source

- Finding: 58-04 hand-off; inventory C7
- Register row: R-82 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
