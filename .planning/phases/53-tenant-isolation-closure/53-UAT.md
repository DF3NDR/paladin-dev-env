---
status: complete
phase: 53-tenant-isolation-closure
source: [53-VERIFICATION.md]
started: 2026-10-31T15:44:24.346Z
updated: 2026-10-31T17:59:00.000Z
---

## Current Test

[testing complete]

## Tests

### 1. P5 / AC-N5 (judgment tier): no configuration key or no-op factory lets a configured Treasurer boot without the engine-factory path
expected: `assert_treasurer_enforced` is the only guard; no setting disables it; the shipped `build_run_api` path always installs a real engine factory. Reviewer reads `run_api_wiring.rs` around the pool construction and the server's bind order and confirms by judgment.
result: pass
note: "Owner judgment after a code read: the only shipped RunWorkerPool::new is run_api_wiring.rs:992 and it always passes a real per-run factory (l.1002); every other construction is #[cfg(test)]; the guard (l.644) has no config input and runs before pool spawn and listener bind. The no-op-factory blind spot stays the documented ADR-0063 residual (AC-N5), covered by code review."

### 2. Decide accept-as-residual vs follow-up for the five advisory review warnings in 53-REVIEW.md (WR-01..WR-05)
expected: Each warning is either recorded as a residual in ADR-0063 or scheduled for a later phase. None contradicts a SPEC acceptance criterion. WR-01 (an `UnknownSchemaVersion` run row aborts every non-admin `GET /v1/threads` with a 500) and WR-02 (`list(limit 1)` decodes the second-newest row, so one corrupt older row locks the owner out of their thread) are the two with operational impact; WR-03 (ownership follows the latest run, so an Admin fork transfers a thread) matches SPEC AC-04 as written; WR-04 (submit on a guessable unused id is distinguishable from a hidden one) is the SPEC-accepted D-14 limitation; WR-05 is a timing residual.
result: pass
reported: "definitely want to log the known bugs for follow-up. Otherwise, this can pass."
decision: |
  Follow-up (open WINDOWS.md rows, proposed owner Phase 55): WR-01 -> row 85, WR-02 -> row 86,
  WR-03 -> row 87 (an owner loses access to its own thread, the same lockout class as WR-02).
  Accepted residual in ADR-0063: WR-04 (T-53-04 bullet extended with the guessable caller-chosen
  id and probe side effect), WR-05 (new timing bullet).

## Summary

total: 2
passed: 2
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps

[none]

## Deferred Follow-Ups

- test: 2
  idea: "WR-01: skip-and-log UnknownSchemaVersion in the non-admin GET /v1/threads walk (WINDOWS row 85)"
  deferred_at: 2026-10-31
- test: 2
  idea: "WR-02: read exactly the newest run (LIMIT 1 port method) for the thread-owner lookup and ensure_thread_visible (WINDOWS row 86)"
  deferred_at: 2026-10-31
- test: 2
  idea: "WR-03: stop ownership transferring to an Admin fork or an unattributed run (WINDOWS row 87)"
  deferred_at: 2026-10-31
