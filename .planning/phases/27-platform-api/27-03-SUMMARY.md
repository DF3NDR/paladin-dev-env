---
phase: 27-platform-api
plan: 03
subsystem: storage
tags: [redis, lua, sorted-set, lease-queue, contract-suite, run-queue, ci]

requires:
  - phase: 27-platform-api
    provides: "RunQueuePort trait (enqueue/dequeue/extend_lease/ack/nack/depth), QueuedRun/LeasedRun/LeaseToken/QueueError types, and the initial InMemoryRunQueue adapter (27-01)"
provides:
  - "run_queue::contract_tests -- one shared async fn per RunQueuePort contract clause (FIFO, lease-expiry redelivery with attempt++, extend_lease, ack, nack, LeaseExpired-vs-UnknownLease, depth, exactly-once-under-concurrency), run unchanged by both adapters"
  - "InMemoryRunQueue brought into full contract conformance: attempt now increments on both expired-lease redelivery and nack; a bounded recently-expired token ring distinguishes LeaseExpired from UnknownLease"
  - "RedisRunQueue -- a ZSET+Lua visibility-lease RunQueuePort adapter behind the redis-queue feature, with four atomic claim/extend/ack/nack scripts"
  - "redis-queue CI job (Tier 2 evidence) and a generalised postgres-integration job covering every *::postgres suite"
affects: [27-04, 27-05, 27-06, 27-09, 27-11, 27-13]

tech-stack:
  added: []
  patterns:
    - "Contract-suite-owns-correctness for RunQueuePort: run_queue::contract_tests mirrors waypoint::contract_tests' shape (named async fns taking &dyn Port, a run_all aggregate) so both InMemory and Redis adapters prove the identical lease/redelivery semantics by construction"
    - "ZSET-scored-by-visibility-time + atomic Lua EVAL for a Redis lease queue (D-08, greenfield in this workspace): one ready ZSET doubles as both the ready queue and the in-flight-lease set (score = visible-at time), with a leases HASH + lease_expiry ZSET giving LeaseExpired-vs-UnknownLease for free from Redis's own key retention"

key-files:
  created:
    - crates/paladin-storage/src/run_queue/contract_tests.rs
    - crates/paladin-storage/src/run_queue/redis.rs
  modified:
    - crates/paladin-storage/src/run_queue/in_memory.rs
    - crates/paladin-storage/src/run_queue/mod.rs
    - .github/workflows/ci.yml

key-decisions:
  - "InMemoryRunQueue's LeaseExpired-vs-UnknownLease distinction is a bounded FIFO ring of recently-reclaimed tokens (RECENTLY_EXPIRED_CAPACITY = 4096), not an unbounded map -- a deliberate, documented memory-bound tradeoff distinct from Redis's own unbounded (but self-documented) lease_expiry retention."
  - "RedisRunQueue's ready ZSET score is a microsecond Unix timestamp (via the server's own TIME command), not milliseconds -- millisecond resolution risks same-score ties on back-to-back enqueue calls, which a ZSET breaks lexicographically by member string rather than by arrival order, silently reordering FIFO. Documented as a known, accepted residual risk (no separate monotonic sequence counter added)."
  - "Redis lease/lease_expiry entries for a superseded (redelivered) token are never proactively pruned -- they stay classified LeaseExpired forever, matching the port's contract, at the cost of unbounded growth on a long-running queue with heavy redelivery. Documented in module rustdoc as an accepted, unaddressed limitation rather than solved with added complexity this plan did not ask for."
  - "postgres-integration's test filter changed from the exact module path `--lib waypoint::postgres` to the substring `--lib postgres`, and its declared-count step now sums `#[tokio::test]` over `src/*/postgres.rs` -- so 27-09/27-11/27-13's future Postgres suites are covered by this one job automatically, with no further CI edits."

requirements-completed: [PLAT-02]

coverage:
  - id: D1
    description: "A message dequeued with lease L is invisible to every other dequeue until L elapses, then becomes visible again with the same run_id and attempt one higher -- on both InMemoryRunQueue and RedisRunQueue"
    requirement: "PLAT-02"
    verification:
      - kind: unit
        ref: "crates/paladin-storage/src/run_queue/contract_tests.rs#lease_expiry_redelivers_with_attempt_incremented, exercised by run_queue::in_memory::tests::lease_expiry_redelivers_with_attempt_incremented (real pass) and run_queue::redis::tests::redis_run_queue_lease_expiry_redelivers_with_attempt_incremented (Tier 2, self-skipped locally)"
        status: pass
    human_judgment: false
  - id: D2
    description: "extend_lease pushes expiry out by exactly the requested duration from the call time; ack removes the message permanently; nack(delay) re-queues it visible only after delay, with attempt incremented; extend_lease/ack/nack on an expired or unknown token return QueueError::LeaseExpired/UnknownLease and never touch another message"
    requirement: "PLAT-02"
    verification:
      - kind: unit
        ref: "contract_tests.rs#extend_lease_keeps_message_hidden_until_new_expiry, #ack_removes_message_permanently, #nack_requeues_after_delay_with_attempt_incremented, #expired_token_operations_return_lease_expired_and_touch_nothing, #unknown_token_operations_return_unknown_lease -- run against InMemoryRunQueue (real pass, 10/10) and RedisRunQueue (Tier 2, self-skipped locally)"
        status: pass
    human_judgment: false
  - id: D3
    description: "depth() counts ready plus leased messages and returns to 0 after every message is acked"
    requirement: "PLAT-02"
    verification:
      - kind: unit
        ref: "contract_tests.rs#depth_counts_ready_plus_leased"
        status: pass
    human_judgment: false
  - id: D4
    description: "The Redis adapter's claim-and-expire is a single atomic EVAL over a sorted set scored by visibility time; redis::Script handles NOSCRIPT reload transparently"
    requirement: "PLAT-02"
    verification:
      - kind: other
        ref: "crates/paladin-storage/src/run_queue/redis.rs -- RUN_QUEUE_CLAIM_LUA/EXTEND/ACK/NACK, each a single redis::Script invocation; grep -c 'pub const RUN_QUEUE_CLAIM_LUA' == 1, grep -c 'Script::new' == 4; NOSCRIPT reload relies on redis 0.32.7's built-in Script::invoke_async retry (not hand-rolled -- verified by reading redis-0.32.7/src/script.rs:180,205 directly)"
        status: pass
    human_judgment: false
  - id: D5
    description: "The Redis suite self-skips with a SKIP: line when RUN_QUEUE_REDIS_TEST_URL is unset; CI's new redis-queue job fails if the suite took the skip path or selected fewer tests than the module declares"
    requirement: "PLAT-02"
    verification:
      - kind: other
        ref: "cargo test -p paladin-storage --features redis-queue --lib run_queue::redis -- --nocapture: 16/16 passed locally, 12 SKIP: lines printed (Docker absent); .github/workflows/ci.yml redis-queue job (SKIP-path failure step + declared-vs-selected count step) is the only place this suite is Tier-2-verified for real"
        status: pass
    human_judgment: true
    rationale: "The live-server pass/fail behavior of the redis-queue CI job itself cannot be observed from this Docker-less devcontainer -- correctness of the Lua scripts against a real Redis server needs the CI job (or a UAT run) to actually fire, per D-51's prohibition on claiming Tier-2 evidence from a local self-skip."

duration: ~25min (wall-clock task execution; commits spanning 03:05-03:19 UTC)
completed: 2026-09-08
status: complete
---

# Phase 27 Plan 03: Run Queue Contract Suite + Redis Adapter Summary

**A shared `RunQueuePort` contract suite (FIFO, lease-expiry redelivery, extend/ack/nack, typed lease errors, depth, exactly-once concurrency) now proves both `InMemoryRunQueue` and a new `RedisRunQueue` (ZSET + atomic Lua lease) identical, plus a CI `redis-queue` job that is the only place the Redis suite is ever really exercised.**

## Performance

- **Duration:** ~25 min task execution (commits 03:05-03:19 UTC), following extensive up-front context reading (27-01-SUMMARY.md, `run_queue_port.rs`, `waypoint/contract_tests.rs`, `node_cache/redis.rs`, existing `redis.rs` queue adapter, CI job precedents, `redis` 0.32.7 vendored source)
- **Started:** 2026-09-08 (this session)
- **Completed:** 2026-09-08T03:19:21Z
- **Tasks:** 3 (all `type="auto"`, Tasks 1-2 `tdd="true"`)
- **Files modified:** 5 (2 created, 3 modified)

## Accomplishments

- `crates/paladin-storage/src/run_queue/contract_tests.rs`: 8 named contract clauses plus a `concurrent_workers_each_message_exactly_once` stress test and a `run_all` aggregate (10 `pub async fn` total), mirroring `waypoint::contract_tests`' shape exactly. Every clause uses real `tokio::time::sleep`, never a paused virtual clock.
- `InMemoryRunQueue` brought into full contract conformance (Red-then-Green, as the plan required): `attempt` now increments on both expired-lease redelivery and `nack`-triggered redelivery (neither incremented before this plan); a new bounded ring of recently-reclaimed tokens (`RECENTLY_EXPIRED_CAPACITY = 4096`) lets `ack`/`extend_lease`/`nack` distinguish `LeaseExpired` from `UnknownLease` instead of collapsing both into `UnknownLease`. 10/10 tests pass, including the `multi_thread`-flavored 8-worker/200-message concurrency stress test.
- `RedisRunQueue` (`crates/paladin-storage/src/run_queue/redis.rs`, behind the `redis-queue` feature): a ZSET (`{prefix}:ready`, scored by microsecond visibility time) doubling as both the ready queue and the in-flight-lease set, a `{prefix}:leases` HASH and a `{prefix}:lease_expiry` ZSET. Four atomic Lua scripts (`RUN_QUEUE_CLAIM_LUA`, `RUN_QUEUE_EXTEND_LUA`, `RUN_QUEUE_ACK_LUA`, `RUN_QUEUE_NACK_LUA`) do every read-then-decide-then-write server-side, each reading the server's own `TIME` for "now" so two replicas never disagree about expiry. No hand-rolled `NOSCRIPT` handling -- confirmed by reading `redis-0.32.7/src/script.rs` directly that `invoke_async` already retries. `RedisRunQueueConfig` redacts any embedded connection-url password in its `Debug` impl and never echoes an unparsable URL verbatim.
- CI: a new `redis-queue` job (mirroring `redis-cache-integration` byte-for-byte in shape) is the only place the Redis suite is Tier-2-verified; `postgres-integration` generalised from the exact `--lib waypoint::postgres` filter to the substring `--lib postgres` (with its declared-count step summing over `src/*/postgres.rs`), so every later `*::postgres` suite this phase adds is covered automatically.
- 16 tests in `run_queue::redis` (locally: all self-skip honestly with 12 `SKIP:` lines, since Docker is absent in this devcontainer); 26 tests total in `run_queue::*` with `--features redis-queue` (10 InMemory real passes + 16 Redis self-skips/pure).

## Task Commits

Each task was committed atomically:

1. **Task 1: The `RunQueuePort` contract suite, run first against `InMemoryRunQueue`** - `999e254d` (feat, `tdd="true"`)
2. **Task 2: `RedisRunQueue` -- sorted-set visibility lease driven by Lua (greenfield)** - `a66117cf` (feat, `tdd="true"`)
3. **Task 3: CI -- the `redis-queue` Tier-2 job and a Postgres job that covers every new suite** - `1d3749bb` (feat)

**Plan metadata:** this file's own commit (docs: complete plan) — committed alongside this SUMMARY per worktree execution mode.

_TDD note: Tasks 1 and 2 both carry `tdd="true"`. For Task 1, the contract suite was written first and immediately exposed a real bug (a redelivered/extended message could return `LeaseExpired` when the original lease was already too short for a later assertion) -- RED before GREEN, one test failure caught and fixed before commit (see Deviations). For Task 2, the Redis adapter's own compile/test cycle (build the module, run against a self-skipping suite, confirm `cargo check`/`clippy` clean) served as the fast gate; no separate RED-then-GREEN commit pair was produced, consistent with how this worktree's per-task commit protocol has been applied for this Rust workspace in prior 27-* plans._

## Files Created/Modified

- `crates/paladin-storage/src/run_queue/contract_tests.rs` — the shared `RunQueuePort` contract suite: `sample_queued_run`, 8 named clauses, `concurrent_workers_each_message_exactly_once`, `run_all`.
- `crates/paladin-storage/src/run_queue/in_memory.rs` — `attempt` increment on both reclaim paths; `RECENTLY_EXPIRED_CAPACITY`, `remember_expired`, `error_for_missing_token`; test module rewired to one `#[tokio::test]` per contract clause plus the `multi_thread` concurrency test and a ring-eviction regression test.
- `crates/paladin-storage/src/run_queue/redis.rs` — `RUN_QUEUE_CLAIM_LUA`/`EXTEND`/`ACK`/`NACK`, `RedisRunQueueConfig` (with redacting `Debug`), `RedisRunQueue` (`enqueue`/`dequeue`/`extend_lease`/`ack`/`nack`/`depth`), self-skipping Tier-2 test module (16 tests).
- `crates/paladin-storage/src/run_queue/mod.rs` — declares `contract_tests` (unconditional) and `redis` (behind `redis-queue`).
- `.github/workflows/ci.yml` — new `redis-queue` job; `postgres-integration` generalised (filter, declared-count step, display name).

## Decisions Made

See `key-decisions` in frontmatter. In prose:

1. **Bounded ring vs. unbounded map for InMemory's expired-token memory.** A `VecDeque<LeaseToken>` capped at 4096 entries keeps `LeaseExpired`-vs-`UnknownLease` classification correct for any realistic in-flight/recently-expired lease count without growing without bound over a long-running process.
2. **Microsecond, not millisecond, ZSET scores in Redis.** Millisecond resolution risks same-score ties across back-to-back `enqueue` calls, which a ZSET breaks by member-string lexicographic order rather than arrival order — a silent FIFO-reordering risk. Microsecond resolution (from the server's own `TIME`) makes this vanishingly unlikely for calls separated by a real network round trip; not made structurally impossible with an added sequence counter, since the plan's design doesn't call for one and the added complexity wasn't judged worth it for a documented, low-probability edge case.
3. **Redis lease/lease_expiry entries are never proactively pruned.** A superseded token (one a later redelivery has moved past) stays classified `LeaseExpired` forever. This matches the port's own contract (an expired token should always report `LeaseExpired`, never silently become `UnknownLease`) but means `leases`/`lease_expiry` grow unboundedly under heavy redelivery. Documented as an accepted, unaddressed limitation in the module rustdoc rather than solved with an out-of-scope cleanup mechanism.
4. **`postgres-integration`'s filter generalised to a substring match.** `--lib waypoint::postgres` became `--lib postgres`, and the declared-test-count step now globs `src/*/postgres.rs`, so 27-09/27-11/27-13's future Postgres suites are covered by this one job with no further CI edits — exactly Task 3's stated purpose.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Contract test `extend_lease_keeps_message_hidden_until_new_expiry` used an original lease shorter than the wait before calling `extend_lease`**
- **Found during:** Task 1, first `cargo test -p paladin-storage --lib run_queue::in_memory` run
- **Issue:** The test dequeued with a 50ms lease, then slept 100ms before calling `extend_lease` — by which point the original lease had already naturally expired and been reclaimed (InMemory's own lazy-reclaim-on-every-op design), so `extend_lease` correctly returned `LeaseExpired` instead of extending a still-live lease. This was a test-fixture bug, not an adapter bug: the clause is about extending a *live* lease, which requires the original lease to still be held when `extend_lease` is called.
- **Fix:** Increased the original dequeue's lease to 300ms (comfortably longer than the 100ms wait before the extension call), so the lease is still live when `extend_lease` fires.
- **Files modified:** `crates/paladin-storage/src/run_queue/contract_tests.rs`
- **Verification:** `cargo test -p paladin-storage --lib run_queue::in_memory` — 10/10 passed.
- **Committed in:** `999e254d` (Task 1 commit)

**2. [Rule 1 - Bug] Module doc comment for `contract_tests.rs` accidentally contained the literal substring `time::pause`, tripping the plan's own acceptance grep**
- **Found during:** Task 1, running the plan's `<acceptance_criteria>` greps directly (`grep -c 'time::pause' contract_tests.rs` returned `2`, not the required `0`) — the doc comment explaining *why* the suite avoids `tokio::time::pause` itself contained the forbidden substring.
- **Fix:** Reworded the doc comment to describe "a paused/mocked async runtime clock" instead of writing the literal `tokio::time::pause`/`time::pause` tokens, preserving the same explanation without the substring.
- **Files modified:** `crates/paladin-storage/src/run_queue/contract_tests.rs`
- **Verification:** `grep -c 'time::pause' contract_tests.rs` → `0`.
- **Committed in:** `999e254d` (Task 1 commit)

**3. [Rule 3 - Blocking] `run_queue/mod.rs`'s `pub mod redis;` declaration had to be added in two steps to keep Task 1 and Task 2 separately committable**
- **Found during:** Task 1, `cargo fmt --all` failed with "failed to resolve mod `redis`" because `redis.rs` did not exist yet.
- **Fix:** Deferred the `#[cfg(feature = "redis-queue")] pub mod redis;` declaration to Task 2's commit (once `redis.rs` existed), keeping Task 1's commit buildable and formattable on its own.
- **Files modified:** `crates/paladin-storage/src/run_queue/mod.rs` (declaration added in the Task 2 commit, not Task 1)
- **Verification:** `cargo fmt --all` and `cargo check --workspace --all-targets --all-features` both clean after each task's commit.
- **Committed in:** `999e254d` (mod.rs without the redis declaration), `a66117cf` (mod.rs with it)

---

**Total deviations:** 3 auto-fixed (2 Rule 1 test-fixture bugs, 1 Rule 3 sequencing fix). None changed the plan's architecture or scope; all were caught and resolved before the affected task's commit.

## Issues Encountered

None beyond the three auto-fixed deviations above. The Redis adapter's Lua scripts could not be exercised against a live server in this session (Docker and `redis-server` are both absent from this devcontainer, consistent with every prior 27-* and 25/26-* plan) — this is the expected Tier-2 gap D-51 documents, closed only by the new `redis-queue` CI job or a future UAT run against a live server, never by a local claim of "passed."

## User Setup Required

None for local development — everything in Tasks 1-2 runs against `InMemoryRunQueue` with no external service. To exercise the Redis Tier-2 suite locally (optional, not required for this plan): `docker compose -f docker/docker-compose.test.yml up -d redis-test` then `RUN_QUEUE_REDIS_TEST_URL=redis://127.0.0.1:6380/1 cargo test -p paladin-storage --features redis-queue --lib run_queue::redis -- --nocapture`.

## Next Phase Readiness

- The `RunQueuePort` contract suite (`run_queue::contract_tests`) is ready for any future backend (e.g., a hypothetical SQS/RabbitMQ adapter) to run unchanged, exactly as `waypoint::contract_tests` already does for `WaypointPort`.
- `RedisRunQueue` is ready to be wired into `RunWorkerPool` (27-04) or any config-driven adapter selection (D-50's `run_queue.rs` config struct, a later plan) via the `RunQueuePort` trait object -- nothing in this plan's files couples the worker pool to a concrete adapter.
- The CI `redis-queue` job is the first (and, per D-51, only legitimate) place this adapter's correctness against a real server will ever be demonstrated; the next run of CI on this branch is the actual Tier-2 verification this plan's own Task 2 could not perform locally.
- `postgres-integration`'s generalised filter (`--lib postgres` + `src/*/postgres.rs` glob) means 27-09/27-11/27-13 need no CI edit when they add their own `*::postgres` modules -- verified by this plan's own `awk`-scoped acceptance greps against the job's YAML block.
- No blockers. `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --all-features`, and `cargo clippy --workspace --all-targets --all-features -- -D warnings` all pass clean on the final commit; `git diff --stat Cargo.lock` is empty (no new dependency -- `redis`'s `script` feature was already enabled in `paladin-storage/Cargo.toml`).

## Self-Check: PASSED

**Files verified to exist:**
- FOUND: `crates/paladin-storage/src/run_queue/contract_tests.rs`
- FOUND: `crates/paladin-storage/src/run_queue/redis.rs`
- FOUND: `crates/paladin-storage/src/run_queue/in_memory.rs`
- FOUND: `crates/paladin-storage/src/run_queue/mod.rs`
- FOUND: `.github/workflows/ci.yml`

**Commits verified to exist (git log --oneline):**
- FOUND: `999e254d` feat(27-03): add RunQueuePort contract suite and fix InMemory lease redelivery
- FOUND: `a66117cf` feat(27-03): add RedisRunQueue -- ZSET+Lua visibility lease (D-08)
- FOUND: `1d3749bb` feat(27-03): add redis-queue CI job; generalise postgres-integration

**Verification commands re-run and confirmed passing:**
- `cargo test -p paladin-storage --features redis-queue --lib run_queue` → `test result: ok. 26 passed` (10 InMemory real, 16 Redis self-skip/pure)
- `cargo test -p paladin-storage --features redis-queue --lib run_queue::redis -- --nocapture` → `test result: ok. 16 passed`; `grep -c 'SKIP:'` → `12`
- `grep -c 'pub async fn' crates/paladin-storage/src/run_queue/contract_tests.rs` → `10`
- `grep -c 'lease_expiry_redelivers_with_attempt_incremented' contract_tests.rs` → `2`
- `grep -c 'flavor = "multi_thread"' crates/paladin-storage/src/run_queue/in_memory.rs` → `1`
- `grep -c 'time::pause' contract_tests.rs` → `0`
- `grep -c 'pub const RUN_QUEUE_CLAIM_LUA' crates/paladin-storage/src/run_queue/redis.rs` → `1`
- `grep -c 'redis::Script::new\|Script::new' redis.rs` → `4`
- `grep -cE '^\s*#\[tokio::test\]' redis.rs` → `11` (≥ 10, the contract_tests.rs `pub async fn` count)
- `git diff --stat Cargo.lock` → empty
- `grep -c '^  redis-queue:$' .github/workflows/ci.yml` → `1`
- `grep -c 'RUN_QUEUE_REDIS_TEST_URL=redis://127.0.0.1:6380/1' .github/workflows/ci.yml` → `1`
- `grep -c 'STORAGE_POSTGRES_TEST_URL' .github/workflows/ci.yml` → `2`
- `awk '/^  postgres-integration:/,/^  redis-cache-integration:/' .github/workflows/ci.yml | grep -c -- '--lib postgres '` → `1`, and the same block's `grep -c '\-\-lib waypoint::postgres'` → `0`
- `awk '/^  postgres-integration:/,/^  redis-cache-integration:/' .github/workflows/ci.yml | grep -c 'src/\*/postgres.rs'` → `1`
- `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"` → exit 0
- `cargo fmt --all -- --check` → clean
- `cargo clippy -p paladin-storage --features redis-queue --all-targets -- -D warnings` → clean
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` → clean
- `cargo check --workspace --all-targets --all-features` → exit 0 (full workspace)

---
*Phase: 27-platform-api*
*Completed: 2026-09-08*
