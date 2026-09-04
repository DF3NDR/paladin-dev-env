---
phase: 22-battlefield-state-superstep-engine
plan: 12
subsystem: infra
tags: [ci, github-actions, postgres, sqlx, tokio, waypoint, integration-testing]

# Dependency graph
requires:
  - phase: 22-battlefield-state-superstep-engine
    provides: "PostgresWaypointStore and its env-gated Tier 2 contract suite (plan 22-06), and docker/docker-compose.test.yml's postgres-test service"
provides:
  - "a postgres-integration CI job in .github/workflows/ci.yml — the only environment where the PostgresWaypointStore Tier 2 contract suite executes"
  - "a Makefile test-integration-docker runner whose Postgres suite invocation matches the CI job's flags exactly"
affects: [22-17]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "live-server integration job: start service -> wait for anchored healthy match -> assert connectivity -> run suite with --nocapture piped to a log -> grep the log for a named SKIP marker -> assert the reported passed-count meets the module's declared test count -> collect logs on failure -> always stop services (mirrors the existing ollama-integration job)"

key-files:
  created: []
  modified:
    - .github/workflows/ci.yml
    - Makefile

key-decisions:
  - "Modeled postgres-integration on the existing ollama-integration job verbatim (same step sequence, same shellcheck-safe healthcheck idiom) rather than inventing a new pattern, per the plan's explicit instruction to copy it"
  - "Selected-count assertion derives its expected N at job runtime via grep -c '#[tokio::test]' against postgres.rs rather than hardcoding it, because sibling gap-closure plan 22-13 adds contract functions to the same module"
  - "Job placed with no needs: edge, immediately after ollama-integration and before coverage, matching how ollama-integration sits in the dependency graph (gates nothing, gated by nothing)"

patterns-established: []

requirements-completed: [ENG-05]

coverage:
  - id: D1
    description: "postgres-integration job added to ci.yml: starts postgres-test, waits for an anchored (-x) healthy match, asserts connectivity with pg_isready, runs waypoint::postgres single-threaded with WAYPOINT_POSTGRES_TEST_URL set and --nocapture, fails on the suite's SKIP: marker, and fails if the reported passed-count is below the module's declared #[tokio::test] count"
    requirement: "ENG-05"
    verification:
      - kind: other
        ref: "python3 YAML structural assertion (22-12-PLAN.md Task 1 <verify>): confirms the job exists, has no needs: edge, contains every required substring (postgres-test, WAYPOINT_POSTGRES_TEST_URL, pg_isready, waypoint::postgres, --test-threads=1, --nocapture, grep -qx healthy, tokio::test, test result: ok), and has a step whose name signals failure detection"
        status: pass
    human_judgment: true
    rationale: "The job's structure is proven by static YAML assertion, but whether it actually runs green against a real live Postgres server on a real PR cannot be verified from this sandboxed environment (no Docker daemon, no CI runner). This is explicitly deferred to the 22-17 checkpoint against a real CI run, per the plan's <output> instruction."
  - id: D2
    description: "Makefile's test-integration-docker Postgres stanza now passes --test-threads=1, identical to the CI job's invocation, and a structural diff proves ci.yml gained exactly the one postgres-integration job with every pre-existing job byte-identical"
    requirement: "ENG-05"
    verification:
      - kind: other
        ref: "python3 structural diff against git show HEAD~1:.github/workflows/ci.yml (22-12-PLAN.md Task 2 <verify>) + grep -c -- '--test-threads=1' Makefile"
        status: pass
    human_judgment: false

# Metrics
duration: ~20min
completed: 2026-09-02
status: complete
---

# Phase 22 Plan 12: Postgres CI Integration Job Summary

**Added a `postgres-integration` CI job that starts a real Postgres server, runs the `PostgresWaypointStore` Tier 2 contract suite against it, and fails loudly if the suite skipped or a filter selected fewer tests than the module declares — closing gap G-22-1.**

## Performance

- **Duration:** ~20 min
- **Tasks:** 2 completed
- **Files modified:** 2 (`.github/workflows/ci.yml`, `Makefile`)

## Accomplishments

- `postgres-integration` job added to `.github/workflows/ci.yml`, modeled on the existing `ollama-integration` job: starts `postgres-test` via `docker/docker-compose.test.yml`, waits for an anchored (`-x`) `healthy` status match, asserts the server accepts connections with `pg_isready` before any test runs, and runs `waypoint::postgres` single-threaded (`--test-threads=1`) with `WAYPOINT_POSTGRES_TEST_URL` set.
- Two independent failure-detection gates beyond the reachability assertion: a step that fails the job if the suite's `SKIP:` early-return marker appears in the captured log, and a step that fails the job if the reported `test result: ok. N passed` count is below the module's declared `#[tokio::test]` count (read at job runtime via `grep -c`, not hardcoded — sibling plan 22-13 adds more contract functions to this module).
- `make test-integration-docker`'s Postgres stanza updated to pass `--test-threads=1`, so the CI job and the local Docker runner issue an identical cargo invocation and a green result means the same thing in both places.
- Confirmed via structural diff that `ci.yml` gained exactly the one job (`postgres-integration`) and every pre-existing job is byte-identical to the pre-Task-1 commit — no other job was touched.

## Task Commits

1. **Task 1: Add the postgres-integration job to ci.yml** - `5782acfe` (feat)
2. **Task 2: Align the Makefile runner with the CI invocation and self-check the workflow** - `264624a7` (fix)

**Plan metadata:** (this commit, following SUMMARY.md write)

## Files Created/Modified

- `.github/workflows/ci.yml` - Adds the `postgres-integration` job (84 lines) between `ollama-integration` and `coverage`
- `Makefile` - Adds `--test-threads=1` to the `test-integration-docker` target's Postgres suite invocation, plus a one-line comment pointing at the CI job as canonical

## Decisions Made

- Copied the `ollama-integration` job's step sequence and shellcheck-safe healthcheck idiom (`grep -qx healthy`, piped rather than command-substituted) verbatim, per the plan's explicit instruction — this is "the same failure class" the plan cites Phase 17 as having found and fixed for Ollama.
- Derived the selected-count assertion's expected N at job runtime (`grep -c '#[tokio::test]' crates/paladin-storage/src/waypoint/postgres.rs`) rather than hardcoding 16, since sibling gap-closure plan 22-13 adds contract functions to this same module and a hardcoded figure would go stale immediately.
- Left the Makefile target's Docker prerequisites unchanged (out of scope here, tracked as a deferred follow-up in `22-UAT.md`); only the flag alignment and comment were added.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None. `actionlint` is not installed in this environment, so workflow linting was not run — this is expected and explicitly acknowledged by the plan itself (Task 2's `<action>`), not a gap introduced here. The job's actual execution against a live Postgres server is likewise unverifiable from this sandbox (no Docker daemon). Both are recorded below as unproven and deferred.

## Verification Debt (explicitly deferred per plan's `<output>` instruction)

- **Workflow linting (`actionlint`) was not run.** Not installed in this environment; the plan directs not to fabricate a passing lint result. Structural YAML parsing was verified instead (both Task 1's and Task 2's automated `<verify>` scripts passed).
- **The `postgres-integration` job's real execution against a live Postgres server is unproven from this environment.** No Docker daemon is available here. This is confirmed at the plan 22-17 checkpoint against a real CI run, per the plan's explicit instruction. Do not treat this SUMMARY as proof the job passes — only that its structure is correct and matches the `ollama-integration` model.
- **The Makefile target's Docker prerequisites remain unverified locally**, as already recorded in `22-UAT.md` (no local Docker daemon in the DevContainer, host-side ownership issues). Out of scope for this plan; only flag alignment was in scope.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Gap G-22-1 is closed structurally: the `postgres-integration` job exists, runs under the same triggers as every other job, and carries three independent gates (reachability, skip-detection, selected-count) against a silently-green non-executing suite.
- Plan 22-17's checkpoint is the correct place to confirm this job actually goes green on a real PR — that confirmation is explicitly NOT claimed here.

---
*Phase: 22-battlefield-state-superstep-engine*
*Completed: 2026-09-02*

## Self-Check: PASSED

- FOUND: `.planning/phases/22-battlefield-state-superstep-engine/22-12-SUMMARY.md`
- FOUND: commit `5782acfe` (Task 1)
- FOUND: commit `264624a7` (Task 2)
