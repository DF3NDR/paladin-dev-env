---
status: testing
phase: 22-battlefield-state-superstep-engine
source: [22-VERIFICATION.md]
started: 2026-09-02T04:35:00Z
updated: 2026-09-02T04:35:00Z
---

## Current Test

number: 1
name: Postgres Tier 2 contract suite against a real server
expected: |
  Run `make test-integration-docker` (or otherwise bring up `postgres-test`) and confirm
  `PostgresWaypointStore` passes the full shared contract suite against a real Postgres server —
  all contract functions pass identically to SQLite/InMemory (WINDOWS.md ledger item 22).
awaiting: user response

## Tests

### 1. Postgres Tier 2 contract suite against a real server
expected: Run `make test-integration-docker` (or otherwise bring up `postgres-test`) and confirm `PostgresWaypointStore` passes the full shared contract suite against a real Postgres server, identically to SQLite/InMemory (WINDOWS.md ledger item 22). No Docker daemon was available in the verification sandbox; only compile, lint and the clean-skip path were proven.
result: [pending]

### 2. Retention prune atomicity decision (22-REVIEW.md CR-01/WR-03)
expected: Decide whether `prune`'s delete-then-resave sequence (`crates/paladin-storage/src/waypoint/retention.rs:130-151`) is acceptable to ship as-is — it is not atomic against a crash/backend failure between `delete_thread` and the resave loop and can destroy a thread's protected (latest / AwaitingInput) Waypoints in that window. Either accept the risk explicitly (retention is disabled by default and not wired to any scheduler yet), or require a fix before it ships as a callable production routine.
result: [pending]

### 3. Silent-stranded-node validation gap decision (22-REVIEW.md CR-02)
expected: Decide whether the stranded-node gap needs a validation fix before Phase 23+ builds on the engine — a non-entry `WarGraph` node whose only incoming edges trace back to itself can never become ready, `WarGraph::validate` does not catch it, and the run reports `RunOutcome::Completed` as if the whole graph executed. Either add a reachability-from-entry check to `WarGraph::validate`, or explicitly accept it as a known, documented limitation (every self-loop test and the E2E-1 fixture already work around it by making the looping node a graph entry — see `tests/integration/e2e_crash_resume_test.rs:112-127`).
result: [pending]

### 4. ENG-NFR-01 SQLite save-latency miss decision
expected: Decide whether the measured SQLite Waypoint-save p50 of 73.09 ms (7.3x over the 10 ms target; fsync-dominated per 22-10-SUMMARY.md) is acceptable for v0.10.0, or whether it needs a follow-up (e.g. explicit `journal_mode=WAL` / `synchronous=NORMAL` SQLite pragmas) before release. Record the decision either way — this is a headline non-functional claim of a checkpoint-every-superstep design.
result: [pending]

## Summary

total: 4
passed: 0
issues: 0
pending: 4
skipped: 0
blocked: 0

## Gaps
