---
created: 2026-11-05T00:00:00Z
title: "EmbeddingPort rustdoc is one physical line with literal backslash-n text"
area: docs
severity: info
files:
  - crates/paladin-ports/src/output/embedding_port.rs
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-14)
---

## Problem

EmbeddingPort rustdoc is one physical line with literal backslash-n text.

Carried into the v0.11.1 pre-tag audit register as R-14; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: ran grep -c over crates/paladin-ports/src/output/embedding_port.rs on 9eba21d9b for the literal two-character sequence: 1 line holds it

## What to do

re-flow the doc comment, then run cargo test -p paladin-ports --doc.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner v0.12.0 Phase 1.

## Source

- Finding: 55 deferred-items.md (55-08)
- Register row: R-14 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
