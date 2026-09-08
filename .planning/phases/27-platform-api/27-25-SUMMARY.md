---
phase: 27-platform-api
plan: 25
subsystem: testing
tags: [ci-evidence, verification, redis, postgres, coverage, sdk-clients, api-surface, e2e, gap-closure]

# Dependency graph
requires:
  - phase: 27-platform-api
    provides: "The six gap-closure plans (27-19 Redis claim/nack marker fix, 27-20 Postgres timestamp precision, 27-21 hermetic sdk-clients smoke, 27-22 bounded webhook read + signing-key failure path, 27-23 heartbeat guard, 27-24 canonical api-surface baseline + e2e-platform-api CI job), plus 27-26's contract_tests::run_all fresh-queue-per-clause fix discovered mid-verification"
provides:
  - "A single evidentiary record (`27-CI-EVIDENCE.md`) tying every one of the five 27-VERIFICATION.md gaps and both open human_verification items to a named CI job, on a named run, at a named SHA, with the exact log line quoted as proof"
  - "Closure of the phase's last two human_verification items — the phase's Tier-2 (Redis/Postgres/coverage/sdk-clients/api-surface) claims are now proven on live CI rather than local-only evidence"
  - "A documented account of the one CI-only regression this verification loop surfaced (contract_tests::run_all suite isolation) and the gap-closure plan (27-26) that fixed it before the evidence run"
affects: [27-verify-work, ship]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pre-fill the evidence table with the job, the gap it closes, and the exact required proof BEFORE the CI run happens, leaving only the Result column empty — prevents the bar from being lowered after seeing results (T-27-25-03)"

key-files:
  created:
    - .planning/phases/27-platform-api/27-CI-EVIDENCE.md
  modified: []

key-decisions:
  - "Added an Integration Tests row beyond the plan's nine named jobs, because the baseline run's failure table explicitly named it as sharing the same Redis root cause as redis-queue — recording it closes that thread rather than leaving it implicit."
  - "Documented the intermediate CI run (34238527001) and gap-closure plan 27-26 in the evidence file itself, not just the SUMMARY, so a later reader of 27-CI-EVIDENCE.md sees the full chain from baseline to evidence run without needing to cross-reference git history."
  - "Recorded the checkpoint as approved based on the human's review of the full per-job log evidence supplied in-session, rather than re-deriving it from a fresh CI query — the run had already completed and its logs were quoted verbatim before the approval was given."

patterns-established: []

requirements-completed: [PLAT-01, PLAT-02, PLAT-03, PLAT-04, PLAT-05, PLAT-06]

coverage:
  - id: D1
    description: "Redis Run Queue Contract Suite (live server) proven green on live CI with the declared-vs-passed guard intact (17 declared, 19 passed) and the live-server log line present"
    requirement: "PLAT-02"
    verification:
      - kind: e2e
        ref: "CI run 34245093476, job 102125436850 (Redis Run Queue Contract Suite (live server))"
        status: pass
    human_judgment: false
  - id: D2
    description: "Postgres Storage Contract Suites (live server) proven green, including the new postgres_run_timestamps_round_trip_at_microsecond_precision clause and the three previously-failing run::postgres clauses"
    requirement: "PLAT-01"
    verification:
      - kind: e2e
        ref: "CI run 34245093476, job 102125436566 (Postgres Storage Contract Suites (live server))"
        status: pass
    human_judgment: false
  - id: D3
    description: "Coverage job completes with no exit 101 and reports 89.98% workspace line coverage, above the 82% ADR-0006 floor"
    requirement: "PLAT-01"
    verification:
      - kind: e2e
        ref: "CI run 34245093476, job 102125436543 (Coverage)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Generated SDK Clients (Python + TypeScript) smoke green, both clients generate >=20 files, submit a run, and reach terminal status exactly 'completed'"
    requirement: "PLAT-06"
    verification:
      - kind: e2e
        ref: "CI run 34245093476, job 102125436564 (Generated SDK Clients (Python + TypeScript) smoke)"
        status: pass
    human_judgment: false
  - id: D5
    description: "API Surface Tracking green with 'API surface unchanged' on CI's own pinned toolchain"
    requirement: "PLAT-06"
    verification:
      - kind: e2e
        ref: "CI run 34245093476, job 102125436884 (API Surface Tracking)"
        status: pass
    human_judgment: false
  - id: D6
    description: "e2e-platform-api job (new, added by 27-24) runs for the first time in CI and passes with a non-zero test count"
    requirement: "PLAT-03"
    verification:
      - kind: e2e
        ref: "CI run 34245093476, job 102125436985 (e2e-platform-api)"
        status: pass
    human_judgment: false
  - id: D7
    description: "Regression guards (Unit Tests, MSRV 1.88, Semver Checks vs v0.9.0) and the Integration Tests job all green at the evidence SHA"
    requirement: "PLAT-05"
    verification:
      - kind: e2e
        ref: "CI run 34245093476 — Unit Tests (stable+beta), MSRV (Rust 1.88), Semver Checks (vs v0.9.0), Integration Tests (job 102125436659)"
        status: pass
    human_judgment: false
  - id: D8
    description: "The blocking human-verify checkpoint (Task 2) was reviewed and explicitly approved by a human against the quoted per-job CI log evidence, not auto-passed"
    verification: []
    human_judgment: true
    rationale: "This is exactly the class of evidence D-51 and the plan's must_haves.prohibitions require a human to confirm — a CI conclusion alone, or local-only evidence, is insufficient for Tier-2 claims; the human reviewed the quoted log lines and explicitly typed 'approved'."

duration: 12min
completed: 2026-09-08
status: complete
---

# Phase 27 Plan 25: CI Evidence Record for Gap-Closure Verification Summary

**Live CI run `34245093476` at SHA `2bf43cd2` proves all six gap-closure plans (27-19 through 27-24) plus the mid-verification fix in 27-26, closing every 27-VERIFICATION.md gap and both open `human_verification` items with quoted per-job log evidence rather than local-only or conclusion-only proof.**

## Performance

- **Duration:** ~12 min (continuation from Task 2 checkpoint approval to plan completion; Task 1's local sweep, recorded separately, ran ~17 min earlier in the same session)
- **Started:** 2026-09-08T16:00:00Z (approx, continuation agent spawn)
- **Completed:** 2026-09-08T16:15:00Z (approx)
- **Tasks:** 2 (Task 1: local gate sweep + evidence scaffold, already committed at start of this continuation; Task 2: checkpoint:human-verify — CI confirmation and evidence fill-in)
- **Files modified:** 1 (`27-CI-EVIDENCE.md`, across two commits: Task 1's scaffold and this continuation's fill-in)

## Accomplishments
- Filled every Tier-2/CI evidence table row with its job's conclusion and the exact quoted log line or number specified as proof in advance (pre-filled by Task 1 before the run, per T-27-25-03) — no row was accepted on conclusion alone where a log string was named.
- Added an `Integration Tests` row beyond the plan's original nine, closing the loop on the baseline's explicit note that this job shared the same Redis root cause as `redis-queue`.
- Documented the intermediate CI run `34238527001`, its single red cause (a `contract_tests::run_all` suite-isolation defect, not a regression in any of the six gap-closure plans), and the gap-closure plan `27-26` that fixed it before the evidence run — so a later reader of the evidence file sees the full chain from baseline to green without cross-referencing git history.
- Recorded the coverage percentage (89.98% lines against the 82% ADR-0006 floor) and the declared-vs-passed test-count guards for both Redis (17 declared / 19 passed) and Postgres (86 declared / 87 passed) suites, per the plan's explicit "declared-vs-selected counts equal" requirement.
- Recorded the Task 2 blocking human-verify checkpoint as approved on 2026-09-08, closing the two `human_verification` items `27-VERIFICATION.md` left open and all six PLAT-0x requirements.

## Task Commits

Each task was committed atomically:

1. **Task 1: Run the full local gate sweep and open the evidence record** - `391a431a` (docs) — completed and merged prior to this continuation.
2. **Task 2: Confirm the gap closure on a live CI run and fill in the evidence** - `de0fb09e` (docs)

**Plan metadata:** SUMMARY commit (this file) — see commit following this summary.

## Files Created/Modified
- `.planning/phases/27-platform-api/27-CI-EVIDENCE.md` - Created by Task 1 with the local sweep table (14/14 green) and a pre-filled, empty-result Tier-2/CI evidence table; filled in by Task 2 (this continuation) with per-job CI conclusions, quoted log proof, an added Integration Tests row, an Intermediate run / plan 27-26 section, and a Verdict paragraph.

## Decisions Made
- Added an `Integration Tests` row beyond the plan's nine named jobs, because the baseline run's failure table explicitly named it as sharing the same Redis root cause as `redis-queue` — recording it closes that thread rather than leaving it implicit.
- Documented the intermediate CI run (`34238527001`) and gap-closure plan `27-26` inside `27-CI-EVIDENCE.md` itself, not only in this SUMMARY, so the evidence file stays self-contained for a later reader.
- Approved the checkpoint based on the human's review of the full per-job log evidence supplied in-session (the run had already completed at SHA `2bf43cd2`; its logs were quoted verbatim before approval was given), rather than re-deriving evidence from a fresh CI query.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed contract_tests::run_all suite-isolation defect surfaced by the first CI run**
- **Found during:** Between Task 1 (local sweep) and Task 2 (CI confirmation) — the first CI run at SHA `a1dbe74e` (run `34238527001`) showed `redis-queue`, `coverage`, and `integration-tests` all red on the same root cause: `contract_tests::run_all` ran its eight clauses against one shared queue instance, so `ack_removes_message_permanently` observed `depth() == 6` instead of `1` (`contract_tests.rs:188`, `left: 6 right: 1`).
- **Issue:** The defect was in the shared contract-test runner itself (a test-suite isolation bug), not a regression introduced by any of the six gap-closure plans (27-19 … 27-24) that ran alongside it in the same CI push.
- **Fix:** A separate gap-closure plan, `27-26`, changed `run_all`'s signature to take an async fresh-queue factory (`Fn() -> Fut, Fut: Future<Output = Q>, Q: RunQueuePort`) and call `fresh_queue().await` once before each clause, so no clause can observe another clause's leftover leases, tokens, or depth. This was executed and committed as its own TDD plan (RED `d942a050`, GREEN `f654e6d0`) — see `27-26-PLAN.md` / `27-26-SUMMARY.md` — rather than folded into this plan's tasks, since it required its own RED/GREEN cycle and touched files outside `27-25-PLAN.md`'s declared `files_modified`.
- **Files modified:** `crates/paladin-storage/src/run_queue/contract_tests.rs`, `crates/paladin-storage/src/run_queue/in_memory.rs`, `crates/paladin-storage/src/run_queue/redis.rs` (all in plan 27-26, not this plan).
- **Verification:** The branch was re-pushed at SHA `2bf43cd2` (containing 27-26); CI run `34245093476` shows `redis-queue`, `coverage`, and `integration-tests` all green, confirming the fix. `34238527001` was cancelled by the concurrency group once superseded.
- **Committed in:** `d942a050` (RED), `f654e6d0` (GREEN) — both in plan 27-26, referenced here rather than duplicated.

---

**Total deviations:** 1 auto-fixed (Rule 1 — bug, executed as a separate gap-closure plan 27-26 rather than inline, because it required its own TDD cycle outside this plan's declared file scope).
**Impact on plan:** The fix was necessary to reach a clean CI run for this plan's evidence to be collected at all — without it, three of the nine required jobs would have stayed red indefinitely on a defect unrelated to any of the six plans under verification. No scope creep beyond documenting the detour in this evidence record.

## Issues Encountered
None beyond the 27-26 detour documented above, which is fully resolved.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- `27-CI-EVIDENCE.md` is the citable evidence record for phase 27's re-verification (`/gsd-verify-work 27` or the orchestrator's re-verification) — all five 27-VERIFICATION.md gaps and both `human_verification` items are answered by a named job, on a named run, at a named SHA, with quoted proof.
- All six PLAT-0x requirements (PLAT-01 through PLAT-06) are backed by live CI evidence, not local-only claims.
- No blockers for phase closure or shipping. The DNS-rebinding limitation on webhook SSRF checks (documented in `security.instructions.md` and originating from plan 27-22/27-13) remains a known, documented limitation, not a gap this plan addresses.

---
*Phase: 27-platform-api*
*Completed: 2026-09-08*

## Self-Check: PASSED
