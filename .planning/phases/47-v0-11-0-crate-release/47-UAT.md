---
status: testing
phase: 47-v0-11-0-crate-release
source: [47-VERIFICATION.md]
started: 2026-10-26T21:41:29Z
updated: 2026-10-26T21:41:29Z
---

## Current Test

number: 1
name: B-1 condensation read of the root CHANGELOG.md [0.11.0] entry
expected: |
  The condensed entry adds no claim beyond the uncondensed text at 700a92d86f3d6b2e24bbd2c163daec7c307297e0
  and drops no breaking change, security note or removal. The answer is recorded as an appended dated
  entry in 47-CI-EVIDENCE.md, closing the `- B-1 maintainer read: requested` line.
awaiting: user response

## Tests

### 1. B-1 condensation read of the root CHANGELOG.md [0.11.0] entry
expected: Read the condensed root CHANGELOG.md [0.11.0] entry against the B-1 ledger (`## B-1 condensation ledger` in 47-CI-EVIDENCE.md) and, where in doubt, against `git show 700a92d86f3d6b2e24bbd2c163daec7c307297e0:CHANGELOG.md`. It adds no claim beyond the uncondensed text and drops no breaking change, security note or removal. Answer recorded as an appended dated entry in 47-CI-EVIDENCE.md.
result: [pending]

### 2. Coverage job `Lines:` figure recorded
expected: Open the `Coverage` job of ci.yml run 40368801098 (job 121733645613, merge commit 70657919) in the Actions UI, copy the printed `Lines:` figure into 47-CI-EVIDENCE.md Findings and the audit's "Outside the table" paragraph (todo 2026-10-26-record-v0-11-0-coverage-lines-figure.md). The figure is at or above 82 percent (ADR-0006); a figure below 82 means the gate or the figure was misread and must be escalated.
result: [pending]

## Summary

total: 2
passed: 0
issues: 0
pending: 2
skipped: 0
blocked: 0

## Gaps
