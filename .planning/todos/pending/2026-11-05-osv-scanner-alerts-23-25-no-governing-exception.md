---
created: 2026-11-05T00:00:00Z
title: "Three OSV-Scanner alerts 23-25 with no governing exception (RUSTSEC-2026-0221, RUSTSEC-2026-0205, GHSA-7gcf-g7xr-8hxj)"
area: security
severity: major
files:
  - SECURITY-EXCEPTIONS.md
owner: repo maintainer
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-11)
---

## Problem

Three OSV-Scanner alerts 23-25 with no governing exception (RUSTSEC-2026-0221, RUSTSEC-2026-0205, GHSA-7gcf-g7xr-8hxj).

Carried into the v0.11.1 pre-tag audit register as R-11; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: read Cargo.lock on 9eba21d9b: event-listener 5.4.1 (:1548) and scc 2.4.0 (:4957) unchanged; ran make audit: exit 0 with the two advisories as warnings among '9 allowed warnings found'; ran make deny: advisories ok

## What to do

dismiss with a row in SECURITY-EXCEPTIONS.md or lift with cargo update -p event-listener and -p scc; alert 23 is not identifiable offline.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner repo maintainer.

## Source

- Finding: 52 deferred-items.md D-52-03-02; STATE Blockers/Concerns (Phase 52 close)
- Register row: R-11 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
