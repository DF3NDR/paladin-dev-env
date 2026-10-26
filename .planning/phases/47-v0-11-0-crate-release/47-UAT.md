---
status: complete
phase: 47-v0-11-0-crate-release
source: [47-VERIFICATION.md]
started: 2026-10-26T21:41:29Z
updated: 2026-10-26T22:50:00Z
---

## Current Test

[testing complete]

## Tests

### 1. B-1 condensation read of the root CHANGELOG.md [0.11.0] entry
expected: Read the condensed root CHANGELOG.md [0.11.0] entry against the B-1 ledger (`## B-1 condensation ledger` in 47-CI-EVIDENCE.md) and, where in doubt, against `git show 700a92d86f3d6b2e24bbd2c163daec7c307297e0:CHANGELOG.md`. It adds no claim beyond the uncondensed text and drops no breaking change, security note or removal. Answer recorded as an appended dated entry in 47-CI-EVIDENCE.md.
result: pass

### 2. Coverage job `Lines:` figure recorded
expected: The `Coverage` job of ci.yml run 40368801098 (job 121733645613, merge commit 70657919) printed `Lines:     167104/180183 = 92.74%` (read at the verify step through the GitHub job-logs API, since the Actions log host is blocked from the sandbox). The figure is quoted verbatim in a dated 47-CI-EVIDENCE.md Findings entry ("2026-10-26 — Verify step"), the audit's "Outside the table" paragraph now carries it, and todo 2026-10-26-record-v0-11-0-coverage-lines-figure.md is moved to completed. 92.74 is at or above the 82 percent floor (ADR-0006), so no escalation. Confirm the recorded figure and its placement; say so if you want the figure cross-checked against the Actions UI yourself.
result: pass

## Summary

total: 2
passed: 2
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps

[none]
