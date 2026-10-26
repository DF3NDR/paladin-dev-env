---
created: 2026-10-26T16:49:29Z
title: Record the v0.11.0 Coverage job's `Lines:` figure in the Phase 47 evidence
area: release-evidence
severity: minor
files:
  - .planning/phases/47-v0-11-0-crate-release/47-CI-EVIDENCE.md
  - .planning/phases/47-v0-11-0-crate-release/47-ACCEPTANCE-AUDIT.md
owner: repo maintainer
resolves_phase: 47
recheck_by: 2026-11-02
---

## Problem

The Phase 47 re-seal records the 82% workspace line-coverage floor (ADR-0006, D-00e) as
**CI-attributed with no figure**. The `Coverage` job concluded `success` under
`--fail-under-lines 82` on every candidate head (SHA-A0 d4e66b47 PR run 40233745953 job
121290924766; SHA-A3 02f6ee2e; final head SHA-A4 72e13f95 PR run 40332201657, `Measure coverage`
step success), but the printed `Lines:` figure was never captured: the job log and the
`coverage-summary` artifact both redirect to an Azure blob host the authoring sandbox's egress
proxy denies, and no check-run output, commit status or PR comment carries the number.

The maintainer was asked for the figure at the 47-04, 47-06 and 47-07 checkpoints and deferred it
(2026-10-26: "Create a ToDo for the Coverage Lines or it will have to be picked up in the
verification step").

## Resolution

1. Open the `Coverage` job of PR run 40332201657 (final head 72e13f95) in the Actions UI and copy
   the line beginning `Lines:` from the job summary or log.
2. Append a dated entry to `47-CI-EVIDENCE.md` under Findings (D-00d, append-only) quoting the
   line verbatim with run id, job id and the date read, and update the audit's "Outside the table"
   coverage paragraph from "no figure" to the quoted figure.
3. If the figure is below 82, the gate would have failed; it did not, so a figure below the floor
   means the gate or the figure was misread and must be escalated, not recorded.

Picked up by `/gsd-verify-work 47` if still open when Phase 47 is verified.
