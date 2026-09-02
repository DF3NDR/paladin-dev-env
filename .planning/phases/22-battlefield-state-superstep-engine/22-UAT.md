---
status: diagnosed
phase: 22-battlefield-state-superstep-engine
source: [22-VERIFICATION.md]
started: 2026-09-02T04:35:00Z
updated: 2026-09-02T17:05:00Z
---

## Current Test

[testing complete]

## Tests

### 1. Postgres Tier 2 contract suite against a real server
expected: Run `make test-integration-docker` (or otherwise bring up `postgres-test`) and confirm `PostgresWaypointStore` passes the full shared contract suite against a real Postgres server, identically to SQLite/InMemory (WINDOWS.md ledger item 22). No Docker daemon was available in the verification sandbox; only compile, lint and the clean-skip path were proven.
result: issue
reported: "This can't run locally in the DevContainer because it is a test of Postgres docker and we are already inside a docker container that doesn't have a proper setup to run docker in docker. It also fails on the host machine because of ownership issues (target owned by root). The `make test-integration-docker` has never run on the local host, only in the CI." (Investigation showed CI does not run it either — see gap G-22-1.)
severity: major

### 2. Retention prune atomicity decision (22-REVIEW.md CR-01/WR-03)
expected: Decide whether `prune`'s delete-then-resave sequence (`crates/paladin-storage/src/waypoint/retention.rs:130-151`) is acceptable to ship as-is — it is not atomic against a crash/backend failure between `delete_thread` and the resave loop and can destroy a thread's protected (latest / AwaitingInput) Waypoints in that window. Either accept the risk explicitly (retention is disabled by default and not wired to any scheduler yet), or require a fix before it ships as a callable production routine.
result: issue
reported: "Require the fix — don't accept it, even disabled by default. It inverts the program's core invariant (goal #2: a crashed process resumes exactly where it stopped); a prune whose crash window destroys latest/AwaitingInput Waypoints is a data-loss path aimed at exactly what the runtime protects, and exposure is anti-correlated with usage (long-suspended Parley threads are what retention most often touches). E2E-1/E2E-2 in .project/v0.10.0 assume protected Waypoints survive anything short of backend loss. 'Disabled and unscheduled' doesn't contain it — it ships as a callable routine and Doc 06 adds schedules. Squarely an X-03 stop-and-flag item; resolution is fix, not waiver. The primitive is wrong, not the feature: replace delete-then-resave with delete-only-unprotected."
severity: blocker

### 3. Silent-stranded-node validation gap decision (22-REVIEW.md CR-02)
expected: Decide whether the stranded-node gap needs a validation fix before Phase 23+ builds on the engine — a non-entry `WarGraph` node whose only incoming edges trace back to itself can never become ready, `WarGraph::validate` does not catch it, and the run reports `RunOutcome::Completed` as if the whole graph executed. Either add a reachability-from-entry check to `WarGraph::validate`, or explicitly accept it as a known, documented limitation (every self-loop test and the E2E-1 fixture already work around it by making the looping node a graph entry — see `tests/integration/e2e_crash_resume_test.rs:112-127`).
result: issue
reported: "Fix required, not a documented limitation. The workarounds in the test suite are the strongest evidence — when E2E-1's fixture has to arrange its looping node specially to avoid the bug, the tests are shaped by the defect rather than guarding against it. 'Completed' lying about a node that could never run breaks the truthful-outcome contract everything downstream (Chronicle, trace export, eval harness) depends on. Specs patched: ENG-FR-02a (eligible-set reachability validation) added to 01-battlefield-state-and-execution-engine.md, registered as BUG-02 in overview §7 and the traceability matrix. Pre-release classification: no MIGRATION.md entry / X-10 register row (WarGraph new in 0.10) — confirmed correct, engine module absent at the v0.9.0 tag and no later tag exists."
severity: major

### 4. ENG-NFR-01 SQLite save-latency miss decision
expected: Decide whether the measured SQLite Waypoint-save p50 of 73.09 ms (7.3x over the 10 ms target; fsync-dominated per 22-10-SUMMARY.md) is acceptable for v0.10.0, or whether it needs a follow-up (e.g. explicit `journal_mode=WAL` / `synchronous=NORMAL` SQLite pragmas) before release. Record the decision either way — this is a headline non-functional claim of a checkpoint-every-superstep design.
result: pass

## Summary

total: 4
passed: 1
issues: 3
pending: 0
skipped: 0
blocked: 0

## Gaps

- gap_id: G-22-1
  truth: "PostgresWaypointStore passes the full shared Tier 2 contract suite against a real Postgres server, identically to SQLite/InMemory (WINDOWS.md ledger item 22)"
  status: failed
  reason: "User reported: can't run in the DevContainer (no docker-in-docker) and fails on the host (target/ owned by root; never run there). Investigation found the suite executes NOWHERE: WAYPOINT_POSTGRES_TEST_URL appears in no CI workflow, so the env-gated suite skips clean in every environment while reporting green."
  severity: major
  test: 1
  root_cause: "The Postgres Tier 2 contract suite is env-gated by WAYPOINT_POSTGRES_TEST_URL and skips clean when unset. No environment sets it: CI has no postgres job (ci.yml never sets the var; the docker-integration job starts only redis-test/minio-test/minio-test-init), the devcontainer has no Docker CLI and no postgres sidecar, and host runs fail on root-owned target/. Same failure shape Phase 17 fixed for Ollama with the ollama-integration job (skip-passing suite that never actually executed)."
  artifacts:
    - path: ".github/workflows/ci.yml"
      issue: "No job starts postgres-test or sets WAYPOINT_POSTGRES_TEST_URL — the Postgres Tier 2 suite never executes in CI"
    - path: "Makefile"
      issue: "test-integration-docker (lines 131-143) is the only runner of the suite and requires a local Docker daemon no dev environment has"
  missing:
    - "postgres-integration CI job in .github/workflows/ci.yml modeled on the existing ollama-integration job (~line 736): start postgres-test from docker/docker-compose.test.yml, wait for healthy, run `cargo test -p paladin-storage --features postgres --lib waypoint::postgres` with WAYPOINT_POSTGRES_TEST_URL=postgres://paladin:paladin@localhost:5433/paladin_waypoint_test, and fail the job if the suite skipped (assert reachability + skip-detection, per the ollama-integration pattern)"
  debug_session: ""

- gap_id: G-22-2
  truth: "Retention prune never destroys protected (latest / AwaitingInput / Parley-referenced / fork-lineage-pinned) Waypoints under any crash or backend failure — prune is monotone and idempotent: the keep-set is always intact and a re-run converges"
  status: failed
  reason: "User reported: require the fix, don't accept even disabled-by-default. The crash window between delete_thread and the resave loop is a data-loss path that inverts the runtime's core crash-resume invariant, violates the E2E-1/E2E-2 assumptions in .project/v0.10.0, and goes live the moment an operator wires prune to a schedule (Doc 06 adds schedules). X-03 stop-and-flag; resolution is fix, not waiver."
  severity: blocker
  test: 2
  root_cause: "Wrong primitive, not a broken feature: prune (crates/paladin-storage/src/waypoint/retention.rs:130-151) implements retention as delete-then-resave — delete_thread wipes ALL of a thread's Waypoints, then a resave loop restores the keep-set. Any crash or backend failure between the two destroys the protected Waypoints. The safe primitive is delete-only-unprotected: protected rows are never touched, so no window exists at all."
  artifacts:
    - path: "crates/paladin-storage/src/waypoint/retention.rs"
      issue: "prune uses delete_thread-then-resave (lines 130-151); crash window destroys protected Waypoints"
    - path: "crates/paladin-ports/src/output/waypoint_port.rs"
      issue: "WaypointPort (line 154) lacks a delete-only-unprotected primitive (prune_thread/delete_waypoints_except); trait is new in 0.10 so adding a method now has no X-10/semver cost — settle before first release"
  missing:
    - "Add `prune_thread(thread_id, keep: &[WaypointId])` (or `delete_waypoints_except`) to WaypointPort with a default implementation that enumerates then deletes unprotected Waypoints individually — a crash mid-way leaves a superset of the keep-set (always safe, idempotent, re-runnable); protected rows are never touched"
    - "SQLite/Postgres backends override with a single transactional `DELETE ... WHERE thread_id = ? AND id NOT IN (...)`; InMemory is trivially atomic"
    - "Rewrite retention.rs prune on top of the new primitive, removing the delete-then-resave sequence entirely"
    - "State the invariant in the FR (next to ENG-FR-18 retention config): prune is monotone and idempotent; under any crash or backend failure the keep-set is intact and a re-run converges. Retention is best-effort space reclamation — leaving extra Waypoints is acceptable; losing protected ones never is"
    - "Fault-injection acceptance test: backend fails (or task aborted) after the first N deletes — assert latest + AwaitingInput Waypoints still load and the thread still resumes; then re-run prune and assert convergence"
    - "Define 'protected' once in the application layer (latest per thread + any Waypoint referenced by an unresolved Parley + anything pinned by an active fork lineage in Chronicle) and pass the keep-set INTO prune rather than recomputing it inside the storage adapter — port stays dumb, policy stays in the application layer (X-01)"
  debug_session: ""

- gap_id: G-22-3
  truth: "WarGraph::validate() rejects, before any node executes, every declared node outside the eligible set — (statically reachable from entry) ∪ (worker_template: true nodes) ∪ (Route { to } targets in any eligible node's Aegis, to a fixed point) ∪ (nodes explicitly marked dynamic_target: true) — listing ALL offenders, so RunOutcome::Completed can never again be reported for a graph containing a node that could never become ready (BUG-02 / ENG-FR-02a)"
  status: failed
  reason: "User reported: fix required, not a documented limitation. The fixture workarounds are the strongest evidence the tests are shaped by the defect; 'Completed' lying about a never-runnable node breaks the truthful-outcome contract Chronicle, trace export, and the eval harness depend on. Spec already patched: ENG-FR-02a in .project/v0.10.0/01-battlefield-state-and-execution-engine.md is the binding requirement; BUG-02 registered in overview §7 and the traceability matrix (verification step 4 greps for both bugs)."
  severity: major
  test: 3
  root_cause: "ENG-FR-02 removed toposort to allow cycles without adding a replacement connectivity check. WarGraph::validate (crates/paladin-battalion/src/engine/graph.rs:211) checks limits and custom-dispatch registration but performs no reachability analysis, so a non-entry node whose only incoming edges trace back to itself passes validation, never becomes ready, and the run reports RunOutcome::Completed. Existing tests route around the defect (E2E-1 fixture makes the looping node a graph entry — tests/integration/e2e_crash_resume_test.rs:112-127)."
  artifacts:
    - path: "crates/paladin-battalion/src/engine/graph.rs"
      issue: "validate() (line 211) has no reachability-from-entry check; no dynamic_target marker exists on NodeSpec yet (grep confirms neither dynamic_target nor worker_template appears in engine sources)"
    - path: "tests/integration/e2e_crash_resume_test.rs"
      issue: "E2E-1 fixture (lines 112-127) works around strandedness by making the looping node a graph entry — fixture cleanup is part of the fix (acceptance 2a)"
  missing:
    - "Implement ENG-FR-02a in WarGraph::validate(): compute the eligible set = statically-reachable-from-entry ∪ worker_template nodes ∪ Route { to } targets in eligible nodes' Aegis on_error (fixed point, since recovery nodes can chain) ∪ nodes marked dynamic_target: true; every declared node outside the set fails validation with EngineError::UnreachableNode / InvalidGraph listing ALL offenders at once, before execution"
    - "Add explicit dynamic_target: true marker to NodeSpec for Goto-only targets. Do NOT infer Goto targets from DirectiveParsers — they are runtime values and inference would be unsound; the marker keeps validation decidable and shifts Goto-target checking to CF-FR-07 (rustdoc the marker as such)"
    - "Self-loops on entry/reachable nodes stay legal — the check rejects strandedness, not loops; completion semantics unchanged (Completed still means Vanguard empty; validation now makes the lie impossible)"
    - "Test-first: red stranded-node fixture first (currently-passing stranded graph), then the fix; regression test asserting a graph with a stranded self-loop-only node is rejected at validation naming the node, while the equivalent graph with the node made reachable (or marked dynamic_target: true) passes; worker-template and Route-target nodes validate without annotation"
    - "Fixture cleanup (acceptance 2a): revisit every test/fixture that works around strandedness, including the E2E-1 looping-node arrangement — no remaining test may route around the bug"
    - "Pre-release classification (verified): engine module absent at the v0.9.0 tag, no later tag — no MIGRATION.md entry, no X-10 register row; in scope for the compatibility audit only as confirmation of the classification"
  debug_session: ""

## Deferred Follow-Ups

- test: 1
  idea: "DevContainer local runnability: add a postgres-test sidecar service to .devcontainer/docker-compose.yml (devcontainer is compose-based, no docker-in-docker needed) so the env-gated suite runs inside the devcontainer via WAYPOINT_POSTGRES_TEST_URL; fold in the uid-1000 fix for root-owned target/ during the required container rebuild."
  deferred_at: 2026-09-02
