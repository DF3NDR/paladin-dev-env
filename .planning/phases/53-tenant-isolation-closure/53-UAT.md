---
status: testing
phase: 53-tenant-isolation-closure
source: [53-VERIFICATION.md]
started: 2026-10-31T15:44:24.346Z
updated: 2026-10-31T15:44:24.346Z
---

## Current Test

number: 1
name: P5 / AC-N5 (judgment tier): no configuration key or no-op factory lets a configured Treasurer boot without the engine-factory path
expected: |
  `assert_treasurer_enforced(treasurer.is_some(), pool.has_engine_factory())` in
  `src/infrastructure/web/run_api_wiring.rs` is the only guard and no setting turns it off.
  The guard cannot see a no-op factory closure (`has_engine_factory` only reports
  `Option::is_some`), so a human reads the shipped wiring and confirms no escape hatch.
  The autonomous verifier found none (no settings key, constant refusal text, the shipped
  path always passes a real factory) and recorded that verdict as NON-AUTHORITATIVE.
awaiting: user response

## Tests

### 1. P5 / AC-N5 (judgment tier): no configuration key or no-op factory lets a configured Treasurer boot without the engine-factory path
expected: `assert_treasurer_enforced` is the only guard; no setting disables it; the shipped `build_run_api` path always installs a real engine factory. Reviewer reads `run_api_wiring.rs` around the pool construction and the server's bind order and confirms by judgment.
result: [pending]

### 2. Decide accept-as-residual vs follow-up for the five advisory review warnings in 53-REVIEW.md (WR-01..WR-05)
expected: Each warning is either recorded as a residual in ADR-0063 or scheduled for a later phase. None contradicts a SPEC acceptance criterion. WR-01 (an `UnknownSchemaVersion` run row aborts every non-admin `GET /v1/threads` with a 500) and WR-02 (`list(limit 1)` decodes the second-newest row, so one corrupt older row locks the owner out of their thread) are the two with operational impact; WR-03 (ownership follows the latest run, so an Admin fork transfers a thread) matches SPEC AC-04 as written; WR-04 (submit on a guessable unused id is distinguishable from a hidden one) is the SPEC-accepted D-14 limitation; WR-05 is a timing residual.
result: [pending]

## Summary

total: 2
passed: 0
issues: 0
pending: 2
skipped: 0
blocked: 0

## Gaps
