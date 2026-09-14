---
phase: 27-platform-api
plan: 26
subsystem: infra
tags: [run-queue, redis, testing, contract-tests, tdd, gap-closure]

# Dependency graph
requires:
  - phase: 27-platform-api
    provides: RedisRunQueue/InMemoryRunQueue as the two RunQueuePort backends and the shared contract_tests.rs suite (plan 27-03/27-04), plus 27-19's claim/nack marker fix that made this defect reachable
provides:
  - "A `contract_tests::run_all` that takes a fresh-queue factory (`Fn() -> Fut, Fut: Future<Output = Q>, Q: RunQueuePort`) and calls it once per clause, so no clause can observe another clause's leftover leases, tokens, or depth"
  - "`in_memory_run_queue_full_contract_suite_via_run_all` (and an idempotency-probe twin running it twice) proving the fix locally, without Redis"
  - "A Redis `run_all` caller that keeps its SKIP path, its SKIP message, and its `#[tokio::test]` count (17) unchanged, while now building a fresh `RedisRunQueue` per clause via a factory closure"
affects: [27-25]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Contract-suite convenience runner takes an async factory (`Fn() -> Fut`) rather than a single shared instance, so 'fresh fixture per clause' is enforced by the runner's own signature instead of by caller discipline"

key-files:
  created: []
  modified:
    - crates/paladin-storage/src/run_queue/contract_tests.rs
    - crates/paladin-storage/src/run_queue/in_memory.rs
    - crates/paladin-storage/src/run_queue/redis.rs

key-decisions:
  - "Kept `queue_or_skip()` byte-identical (probe function, SKIP message, randomized key_prefix logic) and instead wrapped it in the factory closure `|| async { queue_or_skip().await.expect(\"redis-test was reachable a moment ago\") }` for the Redis run_all caller, exactly as the plan specified, rather than inventing a new probe-once-then-fail-fast helper."
  - "Added a second in-memory test (`..._is_idempotent_across_runs`) that calls `run_all` twice back-to-back against the same factory, pinning the PLAT-02 idempotency probe the plan's must_haves called for, rather than folding it into the single existing test."

patterns-established:
  - "Pattern: when a shared multi-clause contract-test runner (`run_all`) must guarantee fixture isolation across clauses, make the runner generic over an async factory rather than a single instance reference — the type signature itself prevents any future caller from accidentally sharing state across clauses again."

requirements-completed: [PLAT-02]

coverage:
  - id: D1
    description: "run_all provisions a fresh, empty queue for every one of the eight contract clauses (fifo, lease-expiry, extend-lease, ack, nack, expired-token, unknown-token, depth), eliminating the suite-isolation defect that leaked leases from earlier clauses into ack_removes_message_permanently's depth() assertion"
    requirement: "PLAT-02"
    verification:
      - kind: unit
        ref: "crates/paladin-storage/src/run_queue/in_memory.rs#run_queue::in_memory::tests::in_memory_run_queue_full_contract_suite_via_run_all"
        status: pass
    human_judgment: false
  - id: D2
    description: "Running run_all twice against the same factory passes both times (idempotency: no state carried between separate run_all invocations, not just between clauses within one invocation)"
    requirement: "PLAT-02"
    verification:
      - kind: unit
        ref: "crates/paladin-storage/src/run_queue/in_memory.rs#run_queue::in_memory::tests::in_memory_run_queue_full_contract_suite_via_run_all_is_idempotent_across_runs"
        status: pass
    human_judgment: false
  - id: D3
    description: "The Redis backend also exercises the fixed run_all, via a factory that builds a fresh RedisRunQueue with its own randomized key_prefix per clause, without adding or removing any #[tokio::test] in redis.rs and without changing the SKIP path/message"
    requirement: "PLAT-02"
    verification:
      - kind: unit
        ref: "crates/paladin-storage/src/run_queue/redis.rs#run_queue::redis::tests::redis_run_queue_full_contract_suite_via_run_all"
        status: pass
    human_judgment: false
  - id: D4
    description: "The live-server Tier-2 proof (redis-queue, coverage, integration-tests CI jobs at the pushed SHA all green) is not locally re-provable in this devcontainer (Docker unavailable, D-51's own prohibition on recording a local Tier-2 run as passed)"
    verification: []
    human_judgment: true
    rationale: "Docker is unavailable in this devcontainer; all 17 run_queue::redis tests (including the fixed run_all test) ran and passed here exclusively via the SKIP path. Evidence is the CI redis-queue/coverage/integration-tests jobs at the SHA containing this plan, per plan 27-25's checkpoint and D-51."

duration: 17min
completed: 2026-09-08
status: complete
---

# Phase 27 Plan 26: contract_tests::run_all fresh-queue-per-clause fix Summary

**`contract_tests::run_all` now takes a fresh-queue factory (`Fn() -> Fut, Fut: Future<Output = Q>`) and calls it once per clause instead of running all eight clauses on one shared queue, closing the single red cause (`left: 6 right: 1` at contract_tests.rs:188) across CI run 34238527001's `redis-queue`, `coverage` and `integration-tests` jobs.**

## Performance

- **Duration:** ~17 min (from base commit `c12d952c` at 14:59:22Z to GREEN commit `f654e6d0` at 15:15:32Z, including RED reproduction, GREEN fix, and full fmt/clippy/check verification)
- **Started:** 2026-09-08T14:59:22Z
- **Completed:** 2026-09-08T15:15:32Z
- **Tasks:** 1 (TDD: RED test commit, then GREEN fix commit)
- **Files modified:** 3

## Accomplishments
- Reproduced CI's exact failure locally without Redis: `in_memory_run_queue_full_contract_suite_via_run_all` (calling the *old* `run_all(&InMemoryRunQueue::new())`, single shared queue) failed at `contract_tests.rs:188` with `left: 6 right: 1` — proving the defect was a suite-isolation bug in `run_all` itself, not a Redis adapter regression.
- Changed `run_all`'s signature to `pub async fn run_all<F, Fut, Q>(fresh_queue: F) where F: Fn() -> Fut, Fut: Future<Output = Q>, Q: RunQueuePort`, calling `fresh_queue().await` once before each of the eight clauses — no clause can now observe another clause's leftover leases, tokens, or depth.
- Updated `run_all`'s module docs to state the fresh-queue-per-clause contract explicitly, with the CI failure's own numbers (`depth() == 6`, the five leases the fifo/lease-expiry/extend-lease clauses left behind) as the worked example of why a shared queue is wrong.
- `redis_run_queue_full_contract_suite_via_run_all` now probes reachability once with the byte-identical `queue_or_skip()` (SKIP path, SKIP message, randomized `test-run-queue-<uuid>` prefix all unchanged), then hands `run_all` a factory closure that calls `queue_or_skip()` again per clause, unwrapped with `.expect("redis-test was reachable a moment ago")`.
- `in_memory_run_queue_full_contract_suite_via_run_all` now calls the fixed `run_all(|| async { InMemoryRunQueue::new() })` and passes; a second test, `..._is_idempotent_across_runs`, calls `run_all` twice back-to-back against the same factory to pin the PLAT-02 idempotency probe.
- No clause body (`fifo_order_and_distinct_lease_tokens` … `depth_counts_ready_plus_leased`) was touched — confirmed via `git diff -U0 HEAD` (pre-RED) against base, which shows only the new `use std::future::Future;` import and the `run_all` doc/signature/body hunk.
- `redis.rs`'s `#[test]`/`#[tokio::test]` count is unchanged at 17 (verified against the base commit and the final tree), so CI's declared-vs-passed guard stays consistent.

## Task Commits

Each task was committed atomically (TDD RED then GREEN):

1. **Task 1 RED: reproduce the suite-isolation defect on in-memory backend** - `d942a050` (test)
2. **Task 1 GREEN: make run_all provision a fresh queue per clause, exercised on both backends** - `f654e6d0` (fix)

## Files Created/Modified
- `crates/paladin-storage/src/run_queue/contract_tests.rs` - `run_all` changed from `&dyn RunQueuePort` to a generic async factory (`F: Fn() -> Fut, Fut: Future<Output = Q>, Q: RunQueuePort`), calling `fresh_queue().await` before every clause; module docs on `run_all` rewritten to state the fresh-queue-per-clause contract and the CI defect it fixes. No clause body changed.
- `crates/paladin-storage/src/run_queue/in_memory.rs` - Added `in_memory_run_queue_full_contract_suite_via_run_all` (exercises the fixed `run_all` against a fresh `InMemoryRunQueue` per clause) and `..._is_idempotent_across_runs` (runs `run_all` twice to pin the idempotency probe).
- `crates/paladin-storage/src/run_queue/redis.rs` - `redis_run_queue_full_contract_suite_via_run_all` changed to probe once via `queue_or_skip()` then pass `run_all` a factory closure rebuilding a fresh `RedisRunQueue` per clause; `queue_or_skip()` itself, its SKIP message, and every other test in the module are unchanged.

## Decisions Made
- Kept `queue_or_skip()` byte-identical and wrapped it in the factory closure exactly as the plan specified (`|| async { queue_or_skip().await.expect("redis-test was reachable a moment ago") }`), rather than inventing a new probe-once-then-fail-fast helper — this keeps the CI declared-test-count guard and the SKIP-path behavior untouched.
- Added a second in-memory test for the idempotency probe rather than folding a second `run_all` call into the first test, so a failure in "does the suite even pass once" is distinguishable from a failure in "does it pass twice in a row" (matches the granular-failure-diagnostics convention `contract_tests.rs`'s own docs describe for per-clause tests).

## Deviations from Plan

None - plan executed exactly as written. The RED test, the `run_all` factory signature, the Redis caller pattern, and the in-memory caller pattern all match the plan's `<behavior>` block verbatim.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required. (Tier-2 Redis verification remains CI-only per D-51, unchanged by this plan.)

## Next Phase Readiness
- `run_all` can no longer leak state between clauses on any backend; the in-memory backend proves this locally (both a single run and two consecutive runs), and the Redis backend now exercises the identical fixed code path via its own factory.
- `crates/paladin-storage/src/run_queue/redis.rs`'s declared test count (17 `#[test]`/`#[tokio::test]` attributes) is unchanged, so CI's `redis-queue` job declared-vs-passed guard stays consistent.
- `cargo fmt --all -- --check`, `cargo clippy -p paladin-storage --all-features --all-targets -- -D warnings`, and `cargo check --workspace --all-targets --all-features` all exit 0 in this worktree.
- Plan 27-25's checkpoint remains the next place Tier-2 evidence (CI `redis-queue`, `coverage`, `integration-tests` jobs green at the SHA containing this plan) gets asserted — not claimed here, per D-51.
- No blockers for the remaining gap-closure plans in this wave.

---
*Phase: 27-platform-api*
*Completed: 2026-09-08*

## Self-Check: PASSED

- FOUND: `.planning/phases/27-platform-api/27-26-SUMMARY.md`
- FOUND: `d942a050` (RED test commit)
- FOUND: `f654e6d0` (GREEN fix commit)
