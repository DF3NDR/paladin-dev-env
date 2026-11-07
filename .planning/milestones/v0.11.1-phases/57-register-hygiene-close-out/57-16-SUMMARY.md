---
phase: 57-register-hygiene-close-out
plan: 16
subsystem: node-cache-redis-fence
tags: [node-cache, redis, fencing, ttl, windows-ledger, d-14]
status: complete

requires:
  - phase: 57-register-hygiene-close-out
    provides: "57-10 triage (owner option-a) that sent 43-REVIEW WR-01 to fix-in-phase as a correctness item"
provides:
  - "The node-cache fence marker lives clamp(2 x entry ttl, 5 min, 30 min), so for an on-time lock holder it expires no later than the Redis Cadence fencing counter that numbered it (43-REVIEW WR-01)"
  - "Both fence TTL rules (counter and marker) defined once in the crate-private paladin-storage redis_fence module"
  - "WINDOWS row 100 (43-REVIEW WR-01), open, for 57-11 to mark fixed on this plan's tests and commits"
  - "paladin-storage CHANGELOG [Unreleased] Fixed, root CHANGELOG [Unreleased] Fixed, MIGRATION 11.1 row M-B-47, ADR-0058 (g) dated amendment and (i) residual bullet"
affects: [57-11]

tech-stack:
  added: []
  patterns:
    - "One crate-private module owns two TTL rules that must stay ordered, with a pure invariant test over a grid of (entry ttl, lock ttl) pairs"
    - "A live-server test moves a key's clock with PTTL/PEXPIRE/DEL instead of waiting hours"

key-files:
  created:
    - crates/paladin-storage/src/redis_fence.rs
  modified:
    - .planning/WINDOWS.md
    - crates/paladin-storage/src/lib.rs
    - crates/paladin-storage/src/cadence/redis.rs
    - crates/paladin-storage/src/node_cache/redis.rs
    - crates/paladin-storage/CHANGELOG.md
    - CHANGELOG.md
    - MIGRATION.md
    - .planning/decisions/0058-rate-pacing-cadence.md

key-decisions:
  - "PD-16-1: cap the marker (clamp 5 min .. 30 min = half the one-hour counter floor) rather than lengthen the counter; the cost is that a late duplicate write of the same deterministic node result is accepted after 30 minutes"
  - "PD-16-3: the module is crate-private and feature-gated like redis_url, so no public item or feature flag changes"
  - "EA-57-20: a holder more than max(10 x L, 1 h) - L - 30 min late (28 minutes with the default 120 s lock) can still leave a marker that outlives the counter; the wait is bounded at 30 minutes, was unbounded"

patterns-established:
  - "A fence marker never outlives the counter that numbers it for any on-time holder"

requirements-completed: [REG-05, REG-02]

duration: ~60min
completed: 2026-11-04
---

# Phase 57 Plan 16: Node-cache fence marker no longer outlives the fencing counter Summary

**The Redis node cache's fence marker is capped at `clamp(2 x entry ttl, 5 min, 30 min)` so it can no longer outlive the Redis Cadence's fencing counter and refuse every valid write of the next lock epoch; both TTL rules now live in one crate-private module, pinned by a pure invariant test and a live next-epoch test.**

## Performance

- **Duration:** ~60 min (two executors: Task 1 and its tracer checkpoint, then Tasks 2 and 3)
- **Plan-start SHA:** `d982235764012126da940d227023dfb06188e111`
- **Completed:** 2026-11-04T22:15Z
- **Tasks:** 3 (1 tracer, 1 tdd auto, 1 auto)
- **Files modified:** 9 (1 created)

## Accomplishments

- 43-REVIEW WR-01: `put_fenced` wrote the marker with `max(2 x entry ttl, 1 h)` and no upper bound, while the counter that numbers the next lock epoch lives `max(10 x lock ttl, 1 h)` and then restarts at 1. With a `CachePolicy` TTL above 30 minutes the marker outlived the counter, so every valid write of the next epoch (token 1 against a marker of 3) was refused until the marker expired. The marker is now `entry.saturating_mul(2).clamp(FENCE_MARKER_FLOOR = 5 min, FENCE_MARKER_CEILING = MIN_FENCE_COUNTER_TTL / 2 = 30 min)`.
- Both rules moved into `crates/paladin-storage/src/redis_fence.rs` (`fence_counter_ttl_ms`, `fence_marker_ttl_ms`), declared in `lib.rs` under `#[cfg(any(feature = "redis-cache", feature = "redis-cadence"))]` like `redis_url`. No `MIN_FENCE_TTL`, `MIN_FENCE_TTL_MS` or `FENCE_TTL_FACTOR` remains in either `redis.rs`; no Lua line changed (`git diff d98223576 -- crates/paladin-storage/src/ | grep -c '^[-+].*redis.call'` is 0).
- A live-server regression test `fenced_write_after_the_counter_expires_is_accepted` shifts the marker's TTL by the counter's lifetime and asserts the token-1 write of the next epoch is accepted.
- Records: `paladin-storage` and root CHANGELOG `[Unreleased]` `### Fixed`, MIGRATION 11.1 `M-B-47` (`## 11.2` and everything from `# 10.` / `## [0.11.0]` down byte-identical), ADR-0058 (g) dated amendment and (i) bullet (`--numstat`: 3 added, 0 deleted; the `Amended 2026-11-04 (Phase 57).` Status line was already there from 57-15 and was not added again).
- No public item changed: `make api-surface` reports unchanged (4331 items); the `paladin-storage` semver run reports `0 failure(s)`.

## Task Commits

1. **Task 1 (tracer): 43 WR-01 end to end (row minted, both TTLs into one module, invariant test watched failing, marker capped)** - `f0898155f` (fix)
2. **Task 2: live Redis case for a fenced write after the counter expires** - `aec444449` (test)
3. **Task 3: changelogs, MIGRATION and ADR-0058 amendment** - `aecf886cb` (docs)

## WINDOWS rows minted

| Id | Description starts | File:line | Kind / phase | Status |
|----|--------------------|-----------|--------------|--------|
| 100 | `43-REVIEW WR-01 (found live by the Phase 57 sweep, D-14): the node-cache fence marker TTL max(2 x entry ttl, 1 h) can outlive the cadence fencing counter max(10 x lock ttl, 1 h) ...` | `crates/paladin-storage/src/node_cache/redis.rs:306` | `deviation` / `43` | `open` |

Minted before the fix commit (EA-57-18 guard: no earlier `43-REVIEW WR-01` row existed). It stays `open`; 57-11 marks it `fixed` on this plan's tests and commits.

## Red-green

| Test | RED on the pre-fix head | GREEN |
|------|-------------------------|-------|
| `node_cache::redis::tests::fence_marker_never_outlives_the_counter_that_numbers_it` (pure) | added while the marker was still `max(2 x entry, 1 h)`: `entry ttl 1000 ms, lock ttl 1 ms: lock + marker = 3600001 ms outlives the counter's 3600000 ms` (the first failing pair in iteration order; the plan's example pair, entry 31 min and lock 120 s, also fails by arithmetic: 62 min marker + 2 min lock = 64 min > 60 min counter, but the assertion stops at the first failure). The floor and ceiling assertions were added in GREEN because those constants did not exist yet. | passed |
| `node_cache::redis::tests::fenced_write_after_the_counter_expires_is_accepted` (live) | see below | passed against a local Redis 7.0.15 |

Pure suites: after the extraction (behaviour-preserving) `node_cache::redis::` 19 passed and `cadence::redis::` 31 passed; after the fix `node_cache::redis::` 20 (now 21 with the live test), `cadence::redis::` 31, and `--features redis-cache,redis-cadence --lib redis` 55 passed (54 before the live test was added). `cadence::redis::tests::fence_ttl_is_ten_times_the_lock_and_at_least_an_hour` is unchanged in name, location and assertions (it now calls `fence_counter_ttl_ms`) and passes.

**The live test.** The CI-less local run takes the self-skip path:

```
SKIP: redis-test not reachable at redis://127.0.0.1:6380/0 -- bring it up with `docker compose -f docker/docker-compose.test.yml up -d redis-test` (or point NODE_CACHE_REDIS_TEST_URL at a reachable server)
test node_cache::redis::tests::fenced_write_after_the_counter_expires_is_accepted ... ok
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 303 filtered out
```

(11 of the 21 tests print that `SKIP:` line locally.) The sandbox does have a `redis-server` binary (7.0.15, which Task 1's notes did not know), so the live path was also exercised: `redis-server --port 6380 --save "" --appendonly no` was started on the scratchpad, `cargo test -p paladin-storage --features redis-cache --lib node_cache::redis:: -- --nocapture` printed no `SKIP:` and `test result: ok. 21 passed; 0 failed` including `fenced_write_after_the_counter_expires_is_accepted ... ok`, and the server was stopped afterwards. That is a local live pass, not the CI job's result. To watch the live test fail, the marker rule in `redis_fence.rs` was temporarily put back to `max(2 x entry, 1 h)` (not committed, file restored byte-for-byte; `git status` showed only the test file modified): the test failed with `lock + marker (14520000 ms) outlives the counter (3600000 ms)`.

## Gates

`df -BM --output=avail /` printed 7120M before the gates; every cargo run used `CARGO_INCREMENTAL=0`.

| Gate | Result |
|------|--------|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy -p paladin-storage --lib --tests -- -D warnings` with `redis-cache`, with `redis-cadence`, with both, with neither | exit 0 each (Task 1); `redis-cache` and `redis-cache,redis-cadence` re-run after Task 2: `Finished dev profile` |
| non-test `--lib` clippy with each feature alone (PD-16-3) | clean (Task 1) |
| `make clean-code` | exit 0 (shellcheck clean, doc-check passed ADR-0033, all 105 D-05 public API entry points carry a plural `# Examples` heading) |
| `make api-surface` | `API surface unchanged` (4331 items) |
| `SEMVER_PRUNE_LOCAL_CACHE=1 CARGO_INCREMENTAL=0 ./scripts/check-semver.sh --package paladin-storage` | `paladin-storage: 223 checks: 223 pass, 31 skip`; `1 package(s) checked against 0.11.0 at --release-type patch; 0 failure(s)` |
| `make security` | exit 0 (`advisories ok, bans ok, licenses ok, sources ok`); run although the plan's gate list omits it, per CLAUDE.md |
| `make check-gates` | exit 0 (12 crates in publish order; 128 files carry `0.11.0`; stub-panic scan `0 finding(s)`) |
| frozen-section diffs (MIGRATION from `# 10.`, both changelogs from `## [0.11.0]`) | identical to `HEAD` before the records commit |

## Evidence for 57-11

- **43 WR-01:** WINDOWS row **100** (open, kind `deviation`, phase `43`); fix commit `f0898155f` (Task 1); test commit `aec444449` (Task 2); records commit `aecf886cb` (Task 3). Pure test `node_cache::redis::tests::fence_marker_never_outlives_the_counter_that_numbers_it`, run with `cargo test -p paladin-storage --features redis-cache --lib fence_marker_never_outlives_the_counter_that_numbers_it`. Docker-gated live test `node_cache::redis::tests::fenced_write_after_the_counter_expires_is_accepted`: 57-11 must read it as `ok` with no `SKIP:` line in the `Redis Node Cache Contract Suite (live server)` CI job on the pushed head before it marks row 100 `fixed`. That is the condition to check, not an observed CI result; locally it passed against Redis 7.0.15 and failed against the old marker rule (above).
- Records: `crates/paladin-storage/CHANGELOG.md` and root `CHANGELOG.md` `## [Unreleased]` `### Fixed`, `MIGRATION.md` 11.1 `M-B-47`, `.planning/decisions/0058-rate-pacing-cadence.md` (g) amendment and (i) EA-57-20 bullet.
- The CI declared-count guard for the node-cache job: `grep -cE '^\s*#\[test\]|^\s*#\[tokio::test\]' crates/paladin-storage/src/node_cache/redis.rs` rose by exactly 2 over the plan (declared count 19 at plan start, 20 after Task 1, 21 after Task 2).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] A live test still asserted the old one-hour marker**
- **Found during:** Task 2
- **Issue:** `redis_put_fenced_writes_both_keys_with_a_ttl_and_skips_a_zero_ttl` asserted the marker PTTL was between 3,590,000 and 3,600,000 ms. Task 1's fix made the marker 5 minutes for its 30 s entry, so the assertion would fail in CI's live-server job (it self-skips locally, so Task 1's local runs missed it).
- **Fix:** the assertion now derives the expected value from `fence_marker_ttl_ms(30_000)` (300,000 ms) with a 10 s tolerance. Found by running the live suite against the local Redis; after the change all 21 pass.
- **Files modified:** `crates/paladin-storage/src/node_cache/redis.rs`
- **Commit:** `aec444449`

Otherwise the plan executed as written. Not a deviation: `make security` was run although the gate list omits it.

## Auth gates

None.

## Known Stubs

None.

## Threat Flags

None. The change adds no endpoint, auth path or schema change; the live test touches only keys under its own `cache.config` prefix (T-57-56 mitigated; T-57-57 and T-57-58 accepted as planned).

## Issues Encountered

None blocking. Residual window (EA-57-20), stated in the ADR-0058 (i) amendment: a holder that writes more than `max(10 x L, 1 h) - L - 30 min` after it took its lock (28 minutes with the default 120 s lock) can still leave a marker that outlives the counter; the next epoch's writes are then refused for at most 30 minutes, where before the wait was unbounded.

## Self-Check: PASSED

- FOUND: `crates/paladin-storage/src/redis_fence.rs`; `fenced_write_after_the_counter_expires_is_accepted` and `fence_marker_never_outlives_the_counter_that_numbers_it` in `crates/paladin-storage/src/node_cache/redis.rs`
- FOUND: commits `f0898155f`, `aec444449`, `aecf886cb` in `git log`
- FOUND: WINDOWS row 100 open; `M-B-47` in `MIGRATION.md`; ADR-0058 (g) and (i) amendments
