---
created: 2026-11-05T00:00:00Z
title: "Stale CodeQL line anchors in CODEQL-DISMISSALS.md rows 40 and 49 (D-52-03-03)"
area: security-docs
severity: info
files:
  - CODEQL-DISMISSALS.md:154
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-12)
---

## Problem

Stale CodeQL line anchors in CODEQL-DISMISSALS.md rows 40 and 49 (D-52-03-03).

Carried into the v0.11.1 pre-tag audit register as R-12; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: read CODEQL-DISMISSALS.md:154 and :258 on 9eba21d9b: the anchors still read rag_retrieval_service.rs:106 and paladin-server.rs:387; the statements now sit at rag_retrieval_service.rs:360 and paladin-server.rs:522

## What to do

refresh the two anchors when the files are next touched (D-14 froze the rows in Phase 52).

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner v0.12.0 Phase 1.

## Source

- Finding: 52 deferred-items.md D-52-03-03; STATE Blockers/Concerns (Phase 52 close)
- Register row: R-12 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
