---
phase: 43-rate-pacing
plan: 11
subsystem: war-engine
tags: [cadence, stampede-lock, fencing-tokens, node-cache, war-engine, superstep, cancellation, fail-open]

requires:
  - phase: 43-rate-pacing
    provides: LockKey, FencingToken, CadencePort::try_lock / unlock, the defaulted NodeCachePort::put_fenced, InMemoryCadence (43-10); lock_ttl_secs validated on treasurer.cadence (43-09)
provides:
  - WarEngine::with_cadence(port, lock_ttl), a library builder beside with_node_cache
  - the engine's lock loop around the cache miss-to-put window (private STAMPEDE_POLL_INTERVAL, StampedeOutcome, HeldLock, acquire_stampede_lock)
  - NodeCacheWiring { cache, lock } bundling the node cache with the lock through run / run_with_namespace / ChildEngineResources, so nested Battalion runs inherit it
  - store_node_cache writes through put_fenced when the dispatch holds a token
  - stampede_lock_tests (8) on the engine; a shared capturing test logger in engine::test_support
affects: [43-12, 43-13]

tech-stack:
  added: []
  patterns:
    - "Bundle a new optional capability with the resource it guards (cache + lock) so threading it is a type change, not a new parameter at every call site"
    - "Double-checked locking: the winner re-reads the cache after acquiring, because the previous holder may have stored and released between this dispatch's first miss and its acquisition"
    - "Release once after the attempt loop (let output = loop {...}; release; output) so every exit path unlocks without a guard type"

key-files:
  created: []
  modified:
    - crates/paladin-battalion/src/engine/superstep.rs
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/src/engine/hooks.rs
    - crates/paladin-battalion/src/engine/test_support.rs
    - MIGRATION.md
    - CHANGELOG.md
    - docs/src/getting-started/configuration.md

key-decisions:
  - "The winner re-reads the cache once after acquiring the lock (an extra get per locked miss) and serves the entry if one appeared, releasing the lock it just took. Without it, a dispatch that missed, lost the CPU while the holder stored and released, and then won try_lock would execute a duplicate: exactly the spend the lock exists to prevent. Engines without a lock keep exactly one get per miss"
  - "A cancelled lock wait returns a NodeTaskOutput with outcome Interrupted and attempt 1, the same shape the grace-abort path records, so the bookkeeping loop writes Skipped { reason: shutdown } and re-lists the node for resume (the run ends Halted)"
  - "The deadline is taken with checked_add; a TTL too large for the clock waits without a deadline rather than panicking. Each sleep is min(poll, time remaining) so the TTL is honoured to the millisecond, not rounded up to a poll"
  - "A task aborted at the shutdown grace deadline cannot unlock; its lock expires by TTL and a late write it might have made is refused by put_fenced at a fence-checking cache. Documented beside the release site"
  - "The capturing test logger moved from hooks.rs tests into engine::test_support and now captures Warn from every target: log::set_logger is once per process, so two modules cannot each install one"

patterns-established:
  - "A scripted Cadence test double (Delegate / AlwaysBusy / Failing) counting try_lock, acquired and unlock calls, over a real InMemoryCadence"

requirements-completed: [PACE-04]

duration: about 1 h
completed: 2026-10-09
status: complete
---

# Phase 43 Plan 11: The engine's stampede lock Summary

**`WarEngine::with_cadence(port, lock_ttl)` coalesces identical cache misses: the dispatch that wins the Cadence lock executes the node and stores through `put_fenced` with its token, the rest poll every ~100 ms and serve the stored delta as a cache hit, and the lock is released on every exit path and never fails a node.**

## Performance

- **Tasks:** 2 of 2
- **Commits:** 013f5bbd (Task 1), a7f4cb93 (Task 2)
- **Files:** 7 modified, none created
- **Redis:** not needed and not started; every test uses the in-process adapters.

## Accomplishments

- **Builder.** `WarEngine::with_cadence(Arc<dyn CadencePort>, Duration)` with rustdoc (when the lock engages, TTL sizing against the p99 node duration including retries, the server attaches none) and a compiling doctest that attaches `InMemoryCadence` and `InMemoryNodeCache` with `?`. A private `cadence_lock` field defaults to `None` in the only constructor.
- **Threading.** A `pub(crate) NodeCacheWiring { cache, lock }` replaces the `Option<Arc<dyn NodeCachePort>>` parameter of `run` / `run_with_namespace` and the `ChildEngineResources.node_cache` field. The four engine start sites call a new `node_cache_wiring()`; test helpers that passed `None` compile unchanged. `NodeCacheBinding` gains `lock`. A nested Battalion run inherits the lock with the cache.
- **Lock loop** (`acquire_stampede_lock`), run only after a miss, with a lock configured and a cache key present: `LockKey("node-cache:{key}")`; `try_lock` -> `Some` re-reads the cache once then executes holding a `HeldLock`; `None` -> sleep `STAMPEDE_POLL_INTERVAL` (100 ms plus up to 50 % jitter, capped at TTL/10, never longer than the time left) through `retry::wait_backoff` (so the run's cancellation token wins), re-read the cache (hit: serve), then try the lock again; deadline passed or `Err` -> execute without the lock (the `Err` logs one `warn!` naming the node, never the key).
- **Fenced store and release.** `store_node_cache` takes `Option<&FencingToken>` and calls `put_fenced` when the dispatch holds a token, else `put`. The attempt loop is now `let output = loop {...}; release; output`, so the lock is released once on success, plain and handler-compensated failure, a mid-backoff cancel and a non-`Edges` directive. An unlock error is a `warn!`.
- **Cancellation.** A token fired during the wait returns `Interrupted` (attempt 1, no work done); the run ends `Halted` promptly (test: 300 ms against a 5 s holder and a 60 s TTL).
- **Registered.** MIGRATION 9.2 row (`paladin-battalion | WarEngine`, N, PACE-04), 9.1 M-B-05 extended, 9.5 `lock_ttl_secs` now describes its consumer; CHANGELOG Phase 43 bullet extended; configuration guide gains TTL sizing guidance.

## Tests added (`engine::tests::stampede_lock_tests`)

- `concurrent_identical_cache_misses_execute_the_node_once` (two engines, shared cache and Cadence, paused clock: handler count 1, one `NodeFinished { cache_hit: true }` and one `false`, one fenced put with a `Local` token, no plain put, one unlock)
- `failed_holder_hands_the_lock_over_within_one_poll` (the waiter runs at most 1.5 polls after the holder failed, with a 60 s TTL)
- `ttl_elapsed_without_an_entry_executes_uncached` (always-busy port, 1 s TTL: runs at about 1 s, at least 5 lock attempts, plain put)
- `lock_error_executes_without_the_lock` (one `try_lock`, one warning that names the node and not the key)
- `cancellation_interrupts_a_waiting_dispatch`
- `no_cadence_means_no_lock_calls` (cache without Cadence: one `get`, plain put; Cadence with a no-policy node: no calls; Cadence without a cache: no calls)
- `unlock_runs_on_every_exit` (success, plain failure, `Absorb`-compensated failure, `NextStep::End`)
- `a_winner_that_finds_the_entry_after_acquiring_serves_it` (extra, for the re-read)

## Red / green record

- The implementation and the tests were written in one sitting and first compiled together, so there was no separate red run (the same process departure as 43-08 and 43-10). The tests were validated by mutation instead, restoring the file from a backup copy each time (never `git checkout`):
  - release removed: fails `concurrent_identical_cache_misses_execute_the_node_once`, `unlock_runs_on_every_exit` and `failed_holder_hands_the_lock_over_within_one_poll`
  - losers poll the cache only and never re-try the lock: fails `failed_holder_hands_the_lock_over_within_one_poll`, `ttl_elapsed_without_an_entry_executes_uncached` and `cancellation_interrupts_a_waiting_dispatch`
  - re-read after acquiring removed, and `put_fenced` replaced by `put`: fails `a_winner_that_finds_the_entry_after_acquiring_serves_it` and `concurrent_identical_cache_misses_execute_the_node_once`

## Verification

- `cargo test -p paladin-battalion --lib stampede_lock`: 8 passed. `--lib node_cache`: 7 passed. `--doc with_cadence`: 1 passed. `--lib`: 844 passed (836 before plus 8). `cargo test -p paladin-battalion` (lib, integration, doc): all passed, 63 doctests passed, 52 ignored (pre-existing).
- `cargo check --workspace --all-targets --all-features`: exit 0. `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean. `cargo fmt --check`: clean.
- `cargo tree -p paladin-battalion -e normal --depth 1 | grep -cE 'paladin-storage|paladin-llm'` prints 0: the engine depends on the `CadencePort` trait only.
- `./scripts/check-migration-allowlist.sh`: set-equal. `./scripts/check-doc-config.sh`: 153 blocks, 0 failed. `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface`: exit 0, "API surface unchanged" (see Deviation 1).
- `make security` not re-run: no dependency, feature or lockfile change (`rand`, `tokio-util` and `log` were already normal dependencies of the crate).
- Manual credential-handling review: no log line carries the lock key or cache key (the key embeds a rendered-input hash); the warnings name the node id and the cadence error, whose messages are URL-free by 43-07. Nothing here touches an API key, a response body or a credential header.

## Deviations from Plan

### Auto-fixed Issues

**1. [Plan acceptance wording] `.project/current-exports.txt` does not contain `with_cadence`**
- **Found during:** Task 2
- **Issue:** the acceptance line expects the baseline to contain `with_cadence`. The baseline covers only the `paladin` facade crate with default features, which does not re-export `WarEngine`, so no `paladin-battalion` symbol can appear (the same as 43-01 to 43-10).
- **Fix:** none; `make api-surface` exits 0 and the builder is covered by the 9.2 register. No re-export added, so `make api-surface-update` was not run and the baseline file is unchanged.

**2. [Rule 1 - Bug, found by design] A winner could duplicate work the previous holder just stored**
- **Found during:** Task 1 design
- **Issue:** the plan's loop reads the cache and then tries the lock. A dispatch that misses, then waits while the holder stores and releases, then wins `try_lock` would execute a duplicate.
- **Fix:** the winner re-reads the cache once after acquiring; a hit releases the lock and serves the entry. Costs one extra `get` per locked miss; without a lock the single `get` is unchanged (asserted).
- **Files modified:** `crates/paladin-battalion/src/engine/superstep.rs`
- **Commit:** 013f5bbd

**3. [Process note] Implementation and tests written together** (see the red / green record); mutation checks stood in for a red run.

**4. [Test infrastructure] The capturing logger moved to `engine::test_support`**
- `log::set_logger` is once per process and `hooks.rs` already installed one for its own test, so a second module could not install another. The logger moved to `test_support` (capturing Warn from every target instead of only `paladin::trace`); the `hooks` test imports it and still filters by content. All 844 lib tests pass.

**5. [Test choice] `RecordingNodeCache` was not used**
- The tests wrap `InMemoryNodeCache` in a small `FencedCache` double that records plain and fenced writes and can make an entry appear after the first `get`; the plan's other named fixtures (`InMemoryNodeCache`, `InMemoryCadence`, a counting `CadencePort`) are used as written.

**Total deviations:** 5 (1 acceptance wording, 1 Rule 1, 1 process note, 2 test-infrastructure). **Impact:** none on planned behaviour; one extra cache read per locked miss.

## Authentication Gates

None.

## Issues Encountered

- None blocking. About 7.9 GB free disk throughout; no ENOSPC and no `target/` cleanup. No Redis server was started.

## Flagged Assumptions

- The lock is not renewed (Open Question 6 as resolved in 43-10): `lock_ttl` must exceed the p99 node duration including retries. A holder that outlives it can be overtaken; its late write is refused only by a fence-checking cache (`RedisNodeCache` with `Distributed` tokens). With the in-process Cadence the tokens are `Local`, which no cache compares, so the fence is a no-op there and a late stale write is a plain last-write-wins (the entry is the same deterministic result either way).
- The server attaches no node cache, so `paladin-server` attaches no lock; `lock_ttl_secs` has no effect on it today.

## Known Stubs

None.

## Threat Flags

None beyond the plan's threat model. Mitigations implemented: T-43-42 (losers re-try the lock every poll, unlock on every normal exit, TTL as the abort backstop, deadline then execute; `failed_holder_hands_the_lock_over_within_one_poll`, `ttl_elapsed_without_an_entry_executes_uncached`, `unlock_runs_on_every_exit`), T-43-43 (`select!` on the node cancellation token through `wait_backoff`; `cancellation_interrupts_a_waiting_dispatch`), T-43-44 (the winner stores through `put_fenced` with its token). T-43-45 accepted as planned: the lock key is the existing node-cache key.

## Next Phase Readiness

43-12 (provider header evidence checkpoint) and 43-13 (ADR-0058, the `Cadence` term-table row, PROMOTION next free number, crate CHANGELOG entries) remain. 43-13 should record the Open Question 2 reading (`put_fenced`) and Open Question 6 (no renewal) from 43-10, and may cite the re-read-after-acquire decision above. PACE-04 is complete end to end: primitives (43-10) plus the engine's coalescing (this plan).

## Self-Check: PASSED

All modified files exist; commits 013f5bbd and a7f4cb93 are present in `git log` on `claude/laughing-dirac-e0h2ax`; no Redis server was started.
