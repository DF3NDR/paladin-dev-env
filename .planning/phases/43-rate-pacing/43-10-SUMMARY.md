---
phase: 43-rate-pacing
plan: 10
subsystem: llm-pacing
tags: [cadence, stampede-lock, fencing-tokens, redis, lua, fail-open, node-cache, put-fenced]

requires:
  - phase: 43-rate-pacing
    provides: CadencePort, InMemoryCadence and the shared contract suite (43-01, 43-05); RedisCadence with one EVAL per operation (43-07); ResilientCadence outage latch and fallback (43-08); build_cadence and the validated-but-unused lock_ttl_secs (43-09)
provides:
  - LockKey and FencingToken (Distributed / Local, no Ord) on paladin-ports, and CadencePort::try_lock / unlock
  - InMemoryCadence stampede lock (Local tokens from an AtomicU64, closed TTL boundary, expired locks pruned on access)
  - ResilientCadence lock arms that fail open onto the in-process lock (D-13) and route unlock by token source; neither returns Err
  - RedisCadence try_lock / unlock through CADENCE_TRY_LOCK_LUA and CADENCE_UNLOCK_LUA (INCR fencing counter, compare-and-delete)
  - NodeCachePort::put_fenced (defaulted, delegates to put) and the RedisNodeCache override backed by NODE_CACHE_PUT_FENCED_LUA
  - shared contract clauses try_lock_is_exclusive, tokens_increase_per_key, unlock_only_by_the_owner, unlock_is_idempotent, unlock_of_a_never_locked_key_is_false, lock_is_not_reentrant, lock_expires_at_its_ttl_paused, concurrent_try_lock_has_exactly_one_winner, run_all_locks
affects: [43-11, 43-13]

tech-stack:
  added: []
  patterns:
    - "Tokens carry their source: an enum with no Ord, so a per-process counter can never be ranked against a fleet-wide one"
    - "Fence checked at the resource: the cache itself refuses a lower distributed token, atomically, in one script"
    - "A defaulted trait method for an optional capability (put_fenced), so every implementor compiles unchanged"
    - "Redis key segments built from a character the pacing key layout can never produce (%lock, %fence) so two key families under one prefix cannot collide"

key-files:
  created: []
  modified:
    - crates/paladin-ports/src/output/cadence_port.rs
    - crates/paladin-ports/src/output/node_cache_port.rs
    - crates/paladin-storage/src/cadence/in_memory.rs
    - crates/paladin-storage/src/cadence/resilient.rs
    - crates/paladin-storage/src/cadence/redis.rs
    - crates/paladin-storage/src/cadence/mod.rs
    - crates/paladin-storage/src/cadence/contract_tests.rs
    - crates/paladin-storage/src/node_cache/redis.rs
    - crates/paladin-storage/src/node_cache/in_memory.rs
    - crates/paladin-llm/src/cadence.rs
    - crates/paladin-llm/src/fallback.rs
    - src/infrastructure/cadence.rs
    - MIGRATION.md
    - CHANGELOG.md

key-decisions:
  - "Open Question 2 resolved as flagged: NodeCachePort gains exactly one DEFAULTED method, put_fenced, delegating to put. No existing method, signature or implementor changes and the D-29 best-effort contract is untouched; the deliberate reading of D-00d/D-11 is registered in MIGRATION 9.2 and is for ADR-0058 (43-13) to record. Operator should confirm or reject it"
  - "Open Question 6 resolved as planned: no lock renewal; lock_ttl_secs stays 120 and must exceed the p99 node duration including retries; fencing makes a late stale write harmless"
  - "Redis lock and counter keys are {prefix}:%lock:{key} and {prefix}:%fence:{key}, not {prefix}:lock:{key}: a pacing key is {prefix}:{provider}:{model} and a provider literally named lock would have collided with the plan's layout (WRONGTYPE on the pacing scripts). A provider half can never contain %l or %f, so the new families cannot equal a pacing key"
  - "A zero lock TTL is raised to one millisecond (Redis rejects PX 0) and an excessive one is clamped at the 24 h delay ceiling, in both adapters, so they agree on edge behaviour"
  - "A Distributed unlock during an outage returns Ok(false) at once without trying the dead primary (the lock expires by its TTL), and a primary error during an unlock latches the outage and also returns Ok(false)"
  - "Fence markers are removed by RedisNodeCache::invalidate (prefix match) and counted in its result; only entries written with put_fenced have one"

patterns-established:
  - "One outage, one warning, whichever method tripped it: lock calls and pacing calls share the latch"
  - "Test double for a shared backend: FlakyCadence issues Distributed tokens over an in-memory table, so the composite's token-source routing is tested without Redis"

requirements-completed: [PACE-04]

duration: about 1 h 45 min
completed: 2026-10-08
status: complete
---

# Phase 43 Plan 10: Stampede lock and fencing tokens Summary

**`CadencePort` can now lock and fence: set-if-absent with expiry, strictly increasing source-tagged tokens (`Distributed` from a Redis `INCR`, `Local` from one process), owner-only release, a lock that fails open onto the in-process table during an outage, and a cache write (`NodeCachePort::put_fenced`) that `RedisNodeCache` refuses when a higher distributed token was already seen.**

## Performance

- **Tasks:** 2 of 2
- **Commits:** b79aad52 (Task 1), 89328240 (Task 2)
- **Files:** 14 modified, none created
- **Redis:** a local `redis-server` 7.0.15 on `127.0.0.1:6380` (no persistence), used by both Redis test modules at their default URLs (`CADENCE_REDIS_TEST_URL` and `NODE_CACHE_REDIS_TEST_URL` both default to port 6380). Shut down with `redis-cli -p 6380 shutdown nosave`; a follow-up `ping` was refused. The verify run printed no `SKIP:` line.

## Accomplishments

- **Port.** `LockKey` (opaque, exact equality) and `FencingToken` with source tagging: `FencingToken::{Distributed(u64), Local(u64)}` is `#[non_exhaustive]`, `Copy`, `Hash`, has `value()` and `is_distributed()` and deliberately no `Ord`; `Local(1) != Distributed(1)` is a unit test. `try_lock` / `unlock` are required trait methods with rustdoc for set-if-absent with expiry, strictly increasing tokens, non-reentrancy, owner-only delete returning `Ok(false)` otherwise, and "any `Err` means proceed without the lock". Module docs and the `CadencePort` doctest extended; every new `paladin-ports` item has a doctest.
- **In-process lock.** `Mutex<HashMap<LockKey, (u64, Instant)>>` plus a per-instance `AtomicU64` starting at 1. Free at exactly `acquired + ttl`, held one tick before (paused-clock clause). Expired entries pruned on every access. `unlock` removes only on an exact `Local` match against a live lock. A zero TTL becomes 1 ms; `Duration::MAX` is clamped, never panics.
- **Fail open (D-13).** `ResilientCadence::try_lock` follows the existing route/latch/probe rules: healthy goes to the primary; an error latches (one warning) and the same call is served by the fallback with a `Local` token; degraded goes to the fallback unless this caller wins the probe. `unlock` routes by token source: `Local` to the fallback; `Distributed` to the primary, `Ok(false)` immediately while degraded, `Ok(false)` plus a latch on a primary error. Neither returns `Err`. A lock call and a pacing call share one outage and one warning (tested both ways round).
- **Redis.** `CADENCE_TRY_LOCK_LUA` (KEYS: lock key, counter key; ARGV: lock TTL ms, counter TTL ms): if the lock exists return nil, else `INCR` the counter, `SET` the lock to the token with `PX`, `PEXPIRE` the counter, return the token. `CADENCE_UNLOCK_LUA`: `DEL` only when `GET` equals the token. Counter TTL = `max(10 * lock TTL, 1 h)`. A `Local` token returns `Ok(false)` with no round trip (tested against a dead server, with a control that a `Distributed` token does try it). Both scripts run in the same timeout wrapper as the pacing scripts via a new `evaluate_keys` that `evaluate` now delegates to. Live tests: the lock contract, tokens across two instances, a TTL on every lock and counter key (with the one-hour floor and the ten-times branch both asserted), real-time expiry with a stale unlock, key-family collision, hostile lock names as data.
- **Fencing at the resource.** `NodeCachePort::put_fenced(key, delta, ttl, &FencingToken)` has a default body that calls `put`. `RedisNodeCache` overrides it: a `Local` token and a zero TTL behave exactly as `put`; a `Distributed(t)` runs `NODE_CACHE_PUT_FENCED_LUA` (read the last fence, ignore if `t` is strictly lower, else `SET` the fence and the entry, both with `PX`; fence TTL `max(2 * entry TTL, 1 h)`); an ignored write is `Ok(())` with one `log::debug!` that names only the token. Live tests: lower ignored, equal and higher accepted, `Local` is a plain put and leaves the fence alone, both keys carry a TTL and a zero TTL writes neither, 16 concurrent writers always leave the highest token's entry, a hostile key is data, `invalidate` still removes a fenced entry. The existing `run_all` node-cache contract passes unchanged on both adapters.
- **Registered.** Three MIGRATION 9.2 rows (all `N`; the allowlist check is set-equal), CHANGELOG Phase 43 bullet extended.

## Red / green record

- The port types, the contract clauses, the in-memory implementation and its tests, and the resilient arms were written in one sitting and first compiled together, so there was no separate red run for Task 1 (a process departure, as in 43-08). The behaviours were validated by mutation instead, restoring each file from a backup copy afterwards (never `git checkout`):
  - in-memory expiry boundary opened (`now <= expires_at`): fails `contract_lock_expires_at_its_ttl_paused`, `in_memory_expired_locks_are_pruned_on_access`, the zero-TTL test and both resilient lock-contract tests
  - in-memory `unlock` accepting any positive token: fails the owner, idempotency, paused-TTL and `run_all_locks` clauses and both resilient lock-contract tests
  - resilient `unlock` sending `Local` tokens to the primary: fails the degraded lock contract, the fail-open test and the routing test
  - Redis lock key without the `%` marker: fails `a_lock_key_can_never_be_a_pacing_key` and the layout test
  - Redis unlock as an unconditional `DEL`: fails the lock contract, the cross-instance test and the stale-holder test
  - fence comparison `token <= last` fails `redis_put_fenced_accepts_an_equal_or_higher_token`; removing the comparison fails the lower-token, local-token and concurrent-writer tests
- Task 2's `put_fenced` default and the Redis override were written test and code together; the mutations above stand in for a red run.

## Task Commits

1. **Task 1: lock methods and fencing tokens, in-process lock, resilient fail-open, lock contract clauses, Redis lock scripts** - `b79aad52`
2. **Task 2: Redis lock live tests, defaulted `put_fenced`, `RedisNodeCache` fenced override, register** - `89328240`

## Verification

- `cargo test -p paladin-ports`: 227 lib + 178 doc passed (94 ignored are pre-existing doctests). `cargo test -p paladin-ports --lib node_cache_port` and `--doc node_cache_port`: 6 and 2 passed.
- `cargo test -p paladin-storage --features redis-cadence,redis-cache --lib -- --nocapture` against the live server: 351 passed, 0 failed, no `SKIP:` line. `--lib cadence` without Redis features: 66 passed (includes `resilient_try_lock_fails_open_onto_the_in_process_lock_with_one_warning`). Doc tests: 4 passed (the extended `ResilientCadence` example runs the fail-open lock).
- `cargo test -p paladin-llm --lib`: 237 passed. `cargo test -p paladin-battalion --lib`: 836 passed (its `RecordingNodeCache` compiles unchanged against the defaulted method). `cargo test -p paladin-ai --lib cadence`: 19 passed.
- `cargo check --workspace --all-targets --all-features`: exit 0. `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean. `cargo fmt --check`: clean.
- `./scripts/check-migration-allowlist.sh`: set-equal. `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface`: exit 0, "API surface unchanged" (see Deviation 1); `make api-surface-update` was therefore not run and `.project/current-exports.txt` is unchanged.
- `make security` was not re-run: no dependency, feature or lockfile change in this plan.
- Manual credential-handling review: no lock or fence log line carries a URL, key, header or token-bearing config; the `put_fenced` debug line names only the numeric token; `RedisCadence` errors keep the 43-07 URL-free construction; lock names and cache keys travel only as `KEYS`/`ARGV` (tested with names containing quotes, Lua fragments, newlines and a NUL).

## Deviations from Plan

### Auto-fixed Issues

**1. [Plan acceptance wording] `.project/current-exports.txt` contains neither `put_fenced` nor `FencingToken`**
- **Found during:** Task 2
- **Issue:** the acceptance line expects the baseline to contain both. The baseline covers only the `paladin` facade crate, which does not re-export `paladin-ports` or `paladin-storage` items, so no sub-crate symbol can appear (same as 43-01 to 43-09).
- **Fix:** none; `make api-surface` exits 0 and the symbols are covered by the 9.2 register. No re-export added.

**2. [Rule 1 - Bug] Redis lock key layout collided with pacing keys**
- **Found during:** Task 1 design, proven in Task 2
- **Issue:** the plan names the lock key `<prefix>:lock:<key>` and the counter `<prefix>:fence:<key>`. A pacing key is `<prefix>:<provider>:<model>` and the provider half is only escaped for `:` and `%`, so a provider named `lock` with model `x` is the same string as the lock for `x`; the pacing scripts (HASH) and the lock scripts (string) would then fail with WRONGTYPE.
- **Fix:** the keys are `<prefix>:%lock:<key>` and `<prefix>:%fence:<key>`. A provider half never contains `%l` or `%f` (every `%` is followed by `25` or `3A`), so no pacing key can equal either family; `LockKey` is appended verbatim last, so lock keys are injective. Tests: `lock_and_fence_keys_are_injective_and_never_equal_a_pacing_key` and the live `a_lock_key_can_never_be_a_pacing_key` (fail under the plan's literal layout).
- **Files modified:** `crates/paladin-storage/src/cadence/redis.rs`
- **Commit:** b79aad52 (code), 89328240 (live tests)

**3. [Process note] Implementation and tests written together** (see the red / green record); mutation checks stood in for a red run.

**4. [Additive, spec detail] Lock TTL edge handling**
- A zero lock TTL is raised to 1 ms and an excessive one clamped at the 24 h ceiling in both adapters (Redis rejects `PX 0`; `Duration::MAX` must not panic). Documented on `CadencePort::try_lock`.

**5. [Additive] `run_all_locks_paused` not created**
- The plan lists only `run_all_locks`; the paused TTL clause is called directly by the paused-clock tests of the in-memory and resilient adapters, as the plan describes. The `redis-cache` feature (not `redis-node-cache`, which the dispatch note mentions) is the real feature name and is what the live runs used.

**6. [Known limitation, documented] `RedisNodeCache::invalidate` counts fence markers**
- The marker sits at `<entry key>:fence`, inside the prefix `invalidate` scans, so it is removed with the entry and included in the returned count. Only entries written with `put_fenced` have one; no existing contract clause is affected and the `put`-only count is unchanged. Documented in the module docs. If an exact count matters later, the marker can move to a separate namespace in a follow-up.

**7. [Extra beyond the plan] Additional tests**
- The `put_fenced` default returns `put`'s error; unlock of a Distributed token during an outage does not touch the primary; the lock returns to the primary after recovery; a primary timeout fails the lock open; fence and lock TTL helper arithmetic; 16-writer race on one entry; hostile names.

**Total deviations:** 7 (1 acceptance wording, 1 Rule 1, 1 process note, 4 additive or documented). **Impact:** none on planned behaviour; the key layout change is internal to the adapter and nothing outside it reads those keys.

## Authentication Gates

None.

## Issues Encountered

- A scripted edit that targeted the Redis `evaluate` helper matched an empty slice and prepended the new helper to the top of the file; the compile error ("unexpected closing delimiter") exposed it, and the block was moved to the right place before any commit.
- No ENOSPC and no `target/` cleanup needed.

## Flagged Assumptions

- **Open Question 2 reading.** `put_fenced` is the one deliberate reading of D-00d/D-11 in this phase (a fence nobody checks is not fencing). If the operator rejects it, the fallback is an inherent `RedisNodeCache::put_fenced` the engine cannot reach through `Arc<dyn NodeCachePort>`, i.e. lock-only protection.
- A counter or fence marker that expires (an idle hour or more) restarts numbering at 1; a stale holder that wakes after that could in principle outrank a fresh holder. The TTL floors (T-43-40) make this require a stall longer than an hour past the lock's own TTL.
- Fence tokens compare as Lua doubles, exact below 2^53; a per-key `INCR` counter will not approach that.

## Known Stubs

None.

## Threat Flags

None beyond the plan's threat model. Mitigations implemented: T-43-37 (Lua compare-and-delete on the exact token; `Local` tokens never reach Redis; `unlock_only_by_the_owner`), T-43-38 (fence checked atomically at the cache; tokens carry their source), T-43-39 (resilient composite fails open; callers need no lock error path), T-43-40 (counter TTL `max(10 * lock TTL, 1 h)`, asserted live), T-43-41 (constant scripts; keys, tokens and payloads only as `KEYS`/`ARGV`, tested with hostile names).

## Next Phase Readiness

43-11 wires `WarEngine::with_cadence` and the private `NodeCacheBinding.lock`: it can call `try_lock` / `unlock` through `Arc<dyn CadencePort>`, treat any `Err` or `None` as "proceed without the lock", and write with `put_fenced` using the token it was given. `lock_ttl_secs` (default 120) is still validated but unused until then. 43-13 should record the Open Question 2 reading (and Open Question 6) in ADR-0058. PACE-04 is complete at the primitive level; the engine's stampede coalescing that satisfies it end to end lands in 43-11.

## Self-Check: PASSED

All modified files listed above exist; commits b79aad52 and 89328240 are present in `git log` on `claude/laughing-dirac-e0h2ax`; the Redis server on port 6380 is stopped.
