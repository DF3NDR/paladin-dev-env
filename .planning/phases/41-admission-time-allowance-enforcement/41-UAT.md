---
status: testing
phase: 41-admission-time-allowance-enforcement
source: [41-VERIFICATION.md]
started: 2026-10-04T12:45:00Z
updated: 2026-10-04T12:04:53Z
---

## Current Test

number: 1
name: Operator UAT walkthrough (41-VALIDATION manual-only row)
expected: |
  With `treasurer.allowance.api_keys.ci-runner { period: 1h, amount: 2.50 }` and the key's
  ledger balance at 2.5000 USD, the next `POST /v1/runs` answers 429 with
  `error.code == "allowance_exhausted"`, `Retry-After` equals the seconds left in the current
  UTC hour, `error.details` names balance / ceiling / window_start / window_end, and
  `paladin-cli treasury spend --api-key ci-runner --since <window_start>` reports the same
  2.5000 USD figure. Then set an 80% warn crossing with a webhook target and confirm one
  operator POST (twelve-key payload, option-b), one trace event and one herald line, and
  nothing on a second admission in the same window.
awaiting: user response

## Tests

### 1. Operator UAT walkthrough (41-VALIDATION manual-only row)
expected: With `treasurer.allowance.api_keys.ci-runner { period: 1h, amount: 2.50 }` and the key's ledger balance at 2.5000 USD, the next `POST /v1/runs` answers 429 `allowance_exhausted` with a `Retry-After` equal to the seconds left in the current UTC hour and `details` naming balance / ceiling / window_start / window_end; `paladin-cli treasury spend --api-key ci-runner --since <window_start>` reports the same 2.5000 USD. With an 80% warn crossing and a webhook target: exactly one operator POST (twelve keys), one trace event and one herald line, and nothing on a second admission in the same window. Why human: needs a running paladin-server, a real operator config and a receiver; the in-process tracers prove the same chain over on-disk SQLite but nobody has driven the real binary and CLI.
result: [pending]

### 2. CI-authoritative gates: coverage (>= 82% workspace line coverage, ADR-0006) and postgres-integration (no SKIP: lines)
expected: Both jobs green on the pushed branch `claude/laughing-dirac-e0h2ax`. The PostgreSQL legs for balance, treasury_notices (migrations 011/012), run_schedules.created_by (010) and the operator webhook delivery row execute rather than skip. Why human: PostgreSQL is not running in the verifier's environment, so env-gated tests skip there; the SUMMARYs of 41-02, 41-05, 41-06 and 41-08 record real runs on a throwaway cluster (32, 14, 147 and 11 passed, 0 SKIP). The coverage figure is only computed in CI.
result: [pending]

### 3. Accepted over-admission race between two admissions in the same instant (41-02 backstop truth)
expected: Confirm ADR-0056 records the race and its closure by Phase 42's reservation at the superstep boundary, and accept it as a known property of check-only admission (D-05). Why human: cannot be proven or prevented mechanically under D-05; the Treasurer proofs show admission is read-only and concurrency-safe, but two simultaneous admissions can both pass.
result: [pending]

### 4. Backstop truths: crash between a won notice claim and the run insert (41-06) and `RunWorkerPool::with_treasury_notices` inside `build_run_api` (41-08)
expected: Accept that a crash in the window between a won notice claim and the run insert can lose one window's notice but never duplicates it or persists a run, and that the production-builder attachment of the notice store to the worker pool has no observable output of its own. The worker tests and the integrated tracer prove the same code path with the same builder call. Why human: both are `verification: backstop` truths (non-inferable by construction) and so abstain rather than pass.
result: [pending]

## Summary

total: 4
passed: 0
issues: 0
pending: 4
skipped: 0
blocked: 0

## Gaps
