---
phase: 25-node-level-fault-tolerance
plan: 04
subsystem: infra
tags: [redis, cache, ttl, hexagonal-ports, contract-tests, config]

# Dependency graph
requires:
  - phase: 25-node-level-fault-tolerance
    provides: "plan 25-01's Aegis skeleton -- CachePolicy { ttl, key: CacheKeySpec } and CacheKeySpec { Default, Fields } already landed in paladin_core::platform::container::aegis, which this plan's NodeCachePort/CachedDelta back"
provides:
  - "paladin_core::platform::container::node_cache::{CachedDelta, NODE_CACHE_SCHEMA_VERSION} -- a top-level persisted record with its own schema_version (X-04) and a closed is_expired_at(now) boundary (FT-06 edge assumption)"
  - "paladin_ports::output::node_cache_port::{NodeCachePort, NodeCacheKey, NodeCacheError} -- an async Send+Sync trait (get/put/invalidate), best-effort by construction (D-29): a get error is a miss, a put error never fails a run"
  - "paladin_storage::node_cache::{InMemoryNodeCache, RedisNodeCache, RedisNodeCacheConfig, contract_tests} -- an ungated in-memory adapter, a Redis adapter behind the new redis-cache feature, and one shared 9-case contract suite both backends run unchanged"
  - "the redis-cache cargo feature on paladin-storage (dep:redis, sharing the already-declared optional dependency with redis-queue) and its facade passthrough on the root Cargo.toml, in no default/storage/full feature set"
  - "the redis-cache-integration CI job (Docker-gated, mirrors postgres-integration's SKIP-detection pattern) -- the only place the Redis tier's contract-suite evidence is provable"
  - "paladin::config::node_cache::{NodeCacheConfig, NodeCacheBackend} -- an X-09-shaped config struct (Default/validate/EnvOverridable), disabled by default, whose redis_password is never Debug-printed"
affects: [25-13-node-cache-engine-integration, 25-14-config-and-migration-docs]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "One shared async contract-test suite (crate::node_cache::contract_tests) parameterised over &dyn NodeCachePort, mirroring waypoint::contract_tests's D-09 precedent exactly -- both InMemoryNodeCache and RedisNodeCache run the identical nine cases from their own #[tokio::test]s"
    - "Tier-2 live-server self-skip convention (waypoint::postgres's store_or_skip, applied here as cache_or_skip): a cheap short-timeout TCP reachability probe before handing the URL to the real client, printing a SKIP: line and returning None rather than failing or hanging when redis-test is absent"
    - "Per-test randomized key_prefix (a fresh UUID per RedisNodeCache construction) for the Tier-2 suite, so concurrently-running #[tokio::test]s sharing one live Redis server never observe or invalidate each other's keys -- avoids the Postgres suite's --test-threads=1 requirement"
    - "Manual (non-derived) Debug impl for a config struct carrying a raw secret field, redacting to a fixed placeholder, when no sibling config already has a redaction convention to copy"

key-files:
  created:
    - crates/paladin-core/src/platform/container/node_cache.rs
    - crates/paladin-ports/src/output/node_cache_port.rs
    - crates/paladin-storage/src/node_cache/mod.rs
    - crates/paladin-storage/src/node_cache/in_memory.rs
    - crates/paladin-storage/src/node_cache/contract_tests.rs
    - crates/paladin-storage/src/node_cache/redis.rs
    - src/config/node_cache.rs
  modified:
    - crates/paladin-core/src/platform/container/mod.rs
    - crates/paladin-ports/src/output/mod.rs
    - crates/paladin-storage/src/lib.rs
    - crates/paladin-storage/Cargo.toml
    - Cargo.toml
    - .github/workflows/ci.yml
    - src/config/mod.rs

key-decisions:
  - "Redis submodule registration deliberately deferred from Task 1's mod.rs edit to Task 2's, exactly as the plan's own action text specifies -- Task 1 registered only in_memory and contract_tests"
  - "Added the redis dependency's safe_iterators feature (shared across redis-queue and redis-cache, since it's the same Cargo.toml dependency line) after the deprecation warning on the unguarded AsyncIter::next_item surfaced during Task 2's first build -- without it, a per-item scan_match conversion error would silently truncate the invalidate() result rather than surfacing as an error"
  - "A zero (or already-elapsed) TTL put() is a silent no-op rather than an attempted PSETEX call in RedisNodeCache, because Redis rejects a non-positive expiry outright; the contract is unaffected since a never-written key is already a miss"
  - "RedisNodeCache's own get() re-checks CachedDelta::is_expired_at(now) after a successful server read (defense in depth beyond the server-side PSETEX expiry), so its hit/miss behavior is identical to InMemoryNodeCache's under clock skew"
  - "New redis-cache-integration CI job added rather than reusing docker-integration's Rust-workspace-wide integration-tests service, because that service's cargo invocation (cargo test --features integration-tests) does not select --features redis-cache -- adding a scoped job mirroring postgres-integration's proven self-skip/detection pattern was the smaller, more legible change"
  - "NodeCacheConfig implements Debug by hand (not derived) to redact redis_password to a fixed placeholder, since no existing sibling config (WaypointStoreConfig, QueueConfig) had a redaction convention for a raw secret field to copy -- WaypointStoreConfig's Postgres variant avoids the problem entirely by naming an env var instead of holding a value, which this config's flat field shape does not do"

requirements-completed: [FT-06]

coverage:
  - id: D1
    description: "CachedDelta persisted record with its own schema_version (X-04) and a closed TTL boundary (an entry read at exactly its expires_at instant is a miss)"
    requirement: FT-06
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/node_cache.rs#tests::cached_delta_round_trips_through_serde_with_schema_version"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/node_cache.rs#tests::is_expired_at_boundary_is_closed"
        status: pass
    human_judgment: false
  - id: D2
    description: "NodeCachePort async trait (get/put/invalidate) in paladin-ports, Send+Sync and object-safe, best-effort by construction per D-29"
    requirement: FT-06
    verification:
      - kind: unit
        ref: "crates/paladin-ports/src/output/node_cache_port.rs#tests::trait_is_object_safe"
        status: pass
      - kind: unit
        ref: "crates/paladin-ports/src/output/node_cache_port.rs#tests::mock_cache_implements_trait"
        status: pass
    human_judgment: false
  - id: D3
    description: "InMemoryNodeCache plus the shared 9-case contract suite (hit, miss, TTL expiry, closed boundary, invalidate-prefix, overwrite, concurrent put, empty delta, cross-graph non-collision) covering all four FT-06 edge assumptions"
    requirement: FT-06
    verification:
      - kind: unit
        ref: "crates/paladin-storage/src/node_cache/in_memory.rs#tests::run_all_contract_functions_smoke_aggregate"
        status: pass
      - kind: unit
        ref: "crates/paladin-storage/src/node_cache/in_memory.rs#tests::entry_at_exactly_expires_at_is_expired"
        status: pass
      - kind: unit
        ref: "crates/paladin-storage/src/node_cache/in_memory.rs#tests::concurrent_put_of_the_same_key_leaves_exactly_one_entry"
        status: pass
      - kind: unit
        ref: "crates/paladin-storage/src/node_cache/in_memory.rs#tests::an_empty_delta_round_trips_as_a_hit_not_a_miss"
        status: pass
      - kind: unit
        ref: "crates/paladin-storage/src/node_cache/in_memory.rs#tests::keys_differing_only_by_graph_prefix_do_not_collide"
        status: pass
    human_judgment: false
  - id: D4
    description: "RedisNodeCache behind the redis-cache feature (over the existing ConnectionManager pattern, server-side PSETEX expiry, cursor-based SCAN invalidate), running the same contract suite, namespaced by key_prefix, self-skipping locally without a server"
    requirement: FT-06
    verification:
      - kind: unit
        ref: "crates/paladin-storage/src/node_cache/redis.rs#tests::redis_node_cache_self_skips_without_a_server"
        status: pass
    human_judgment: true
    rationale: "Docker/Redis are absent from this devcontainer (confirmed via RESEARCH.md and re-confirmed live: no redis-server binary present), so the four live-server Tier-2 cases (redis_node_cache_runs_the_full_contract_suite, redis_keys_are_namespaced_by_the_configured_prefix, redis_ttl_is_set_on_the_server_not_only_in_the_payload, plus the full nine-case contract suite against a real server) took the SKIP path in every local run and have never executed against a real Redis in this environment. Their evidence is CI-only, via the new redis-cache-integration job's SKIP-detection assertion -- a human/CI-run confirmation, not a locally-passing automated test, is required before this deliverable can be considered proven end-to-end."
  - id: D5
    description: "NodeCacheConfig (X-09 shape: Default/validate/EnvOverridable), disabled by default with the InMemory backend, never Debug-printing redis_password, introducing no Aegis policy config surface"
    requirement: FT-06
    verification:
      - kind: unit
        ref: "src/config/node_cache.rs#tests::default_node_cache_config_is_disabled"
        status: pass
      - kind: unit
        ref: "src/config/node_cache.rs#tests::env_overrides_apply_for_every_field"
        status: pass
      - kind: unit
        ref: "src/config/node_cache.rs#tests::validate_rejects_an_empty_key_prefix_and_a_zero_port"
        status: pass
      - kind: unit
        ref: "src/config/node_cache.rs#tests::redis_backend_without_a_host_fails_validation"
        status: pass
      - kind: unit
        ref: "src/config/node_cache.rs#tests::debug_rendering_never_prints_the_password"
        status: pass
    human_judgment: false

duration: 80min
completed: 2026-09-05
status: complete
---

# Phase 25 Plan 04: Node-Cache Persistence Layer Summary

**Landed `CachedDelta`/`NodeCachePort` in core/ports, `InMemoryNodeCache` plus a shared 9-case contract suite covering all four FT-06 cache edge assumptions, a Redis adapter behind a new `redis-cache` feature that touches no default build and self-skips without a server, and an X-09-shaped `NodeCacheConfig` disabled by default.**

## Performance

- **Duration:** ~80 min (across two session interruptions -- a mid-run agent-session crash and a later disk-full crash from a concurrent `cargo clean` elsewhere in the shared devcontainer; both times, work already committed to this worktree's branch survived intact and uncommitted work-in-progress was recovered from the exact point it stopped)
- **Started:** 2026-09-05 (worktree spawn)
- **Completed:** 2026-09-05T21:16:00Z
- **Tasks:** 3
- **Files modified:** 14 (7 created, 7 modified)

## Accomplishments

- `CachedDelta { schema_version, delta, stored_at, expires_at }` landed in `paladin-core` as a top-level persisted record carrying its own `NODE_CACHE_SCHEMA_VERSION` (X-04), with a closed `is_expired_at` boundary (`now >= expires_at` is expired, never `now > expires_at`) -- the FT-06 TTL-boundary edge case, pinned by a dedicated unit test.
- `NodeCachePort { get, put, invalidate }` landed in `paladin-ports` as a `#[async_trait] Send + Sync` object-safe trait, documented and enforced as best-effort by construction (D-29): the engine treats a `get` error exactly like a miss and a `put` error never fails a node's run.
- `InMemoryNodeCache` landed in `paladin-storage` (ungated, no feature flag, mirroring `waypoint::in_memory`'s D-01 precedent) as a hand-rolled `HashMap<NodeCacheKey, CachedDelta>` behind a `tokio::sync::RwLock`, with lazy expiry-on-read plus opportunistic sweep-on-write -- no TTL-cache crate pulled in (confirmed by a negative grep for `moka`/`ttl_cache`/`cached` in `Cargo.toml`).
- One shared contract suite (`crate::node_cache::contract_tests`, mirroring `waypoint::contract_tests`'s D-09 shape) covers all nine named cases -- including the four FT-06 edge assumptions the deterministic edge probe had left unclassified: the closed TTL boundary, concurrent same-key `put` leaving exactly one surviving entry, an empty delta round-tripping as a hit (never conflated with a miss), and two keys differing only by graph-prefix component reading back independently. Both `InMemoryNodeCache` and `RedisNodeCache` run this exact suite unchanged from their own `#[tokio::test]`s.
- `RedisNodeCache` landed behind a new `redis-cache = ["dep:redis"]` feature on `paladin-storage`, sharing the already-declared optional `redis` dependency with `redis-queue` (no new dependency, no version bump), over the identical `redis::aio::ConnectionManager` construction `crate::redis::RedisQueueAdapter` already uses. Each `CachedDelta` is stored as JSON under `{key_prefix}:{key}` with a server-side `PSETEX` expiry (proven directly against the raw key, independent of any client-side `expires_at` re-check); `invalidate(prefix)` uses a cursor-based `SCAN` (`scan_match`), never a blocking full-keyspace or whole-database-wipe command. The facade `redis-cache = ["paladin-storage/redis-cache"]` passthrough on the root `Cargo.toml` is in no `default`/`storage`/`full` set, confirmed by inspecting the resolved `normal` (production) dependency graph directly: no `redis` entry appears under default features.
- A new `redis-cache-integration` CI job (mirroring `postgres-integration`'s SKIP-detection pattern exactly, reusing the existing `redis-test` Docker Compose service rather than standing up a second one) is the only place the Redis tier's contract-suite evidence is provable -- Docker/Redis are confirmed absent from this devcontainer, so every Tier-2 test locally reports `SKIP:` and exits 0 rather than failing or hanging.
- `NodeCacheConfig { enabled, backend, redis_host, redis_port, redis_password, redis_db, key_prefix }` and `NodeCacheBackend { InMemory, Redis }` landed under `src/config/`, mirroring `WaypointStoreConfig`'s X-09 template (`Default` + `validate()` + `EnvOverridable`), disabled by default with the `InMemory` backend so a v0.9 configuration file resolves to identical behavior. `redis_password` is never `Debug`-printed (a hand-written `Debug` impl renders it as a fixed `[REDACTED]` placeholder); no Aegis policy env var or config field was introduced (confirmed by a negative grep across `src/`).

## Task Commits

1. **Task 1: CachedDelta, NodeCachePort, InMemoryNodeCache and the shared contract suite** — `f7c0faa3` (feat)
2. **Task 2: RedisNodeCache behind a redis-cache feature, Tier 2 routed to CI** — `0fdfc7b6` (feat)
3. **Task 3: NodeCacheConfig under src/config/, off by default** — `db2e6806` (feat)

**Plan metadata:** (this commit, `docs(25-04): ...`)

## Files Created/Modified

- `crates/paladin-core/src/platform/container/node_cache.rs` — `CachedDelta`, `NODE_CACHE_SCHEMA_VERSION`, `is_expired_at`
- `crates/paladin-core/src/platform/container/mod.rs` — module registration (alphabetical, before `node_error`)
- `crates/paladin-ports/src/output/node_cache_port.rs` — `NodeCacheKey`, `NodeCacheError`, `NodeCachePort`
- `crates/paladin-ports/src/output/mod.rs` — module registration (alphabetical, before `notification_port`)
- `crates/paladin-storage/src/node_cache/mod.rs` — module layout, feature-gated `redis` registration
- `crates/paladin-storage/src/node_cache/in_memory.rs` — `InMemoryNodeCache`
- `crates/paladin-storage/src/node_cache/contract_tests.rs` — the shared 9-case contract suite
- `crates/paladin-storage/src/node_cache/redis.rs` — `RedisNodeCacheConfig`, `RedisNodeCache`, Tier-2 live-server test module
- `crates/paladin-storage/src/lib.rs` — `node_cache` module registration
- `crates/paladin-storage/Cargo.toml` — `redis-cache` feature, `safe_iterators` added to the shared `redis` dependency
- `Cargo.toml` — facade `redis-cache` passthrough
- `.github/workflows/ci.yml` — new `redis-cache-integration` job
- `src/config/node_cache.rs` — `NodeCacheConfig`, `NodeCacheBackend`
- `src/config/mod.rs` — module registration and re-export

## Decisions Made

See `key-decisions` in the frontmatter for the full list. In brief: the Redis submodule registration was deliberately deferred from Task 1 to Task 2 per the plan's own instruction; `safe_iterators` was added to the shared `redis` dependency after a deprecation warning surfaced on `AsyncIter::next_item`; a zero-TTL `put` is a silent no-op in `RedisNodeCache` (Redis rejects non-positive `PSETEX` expiries); a new CI job was added rather than extending `docker-integration`'s existing `integration-tests` service, which does not select the `redis-cache` feature; and `NodeCacheConfig` implements `Debug` by hand because no sibling config already had a redaction convention for a raw secret field to copy.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Self-referential negative-grep acceptance checks tripped on their own literal strings**
- **Found during:** Task 2 (`redis.rs`) and Task 3 (`node_cache.rs`), while verifying this plan's own acceptance-criteria greps against the files as first written
- **Issue:** Doc comments in `redis.rs` explaining "never the blocking KEYS/FLUSHDB/FLUSHALL commands" contained the literal words `KEYS`, `FLUSHDB`, `FLUSHALL`, which the plan's own acceptance check (`! grep -qE '\bKEYS\b|...' crates/paladin-storage/src/node_cache/redis.rs`) then matched -- the explanatory comment defeated the check meant to prove the commands' absence. A planned self-check test in `redis.rs` (and a similar one drafted for `node_cache.rs` asserting the absence of `APP_AEGIS`/`APP_ENGINE_RETRY`/`APP_ENGINE_ON_ERROR`) had the identical problem: embedding the forbidden strings as literals to assert their absence.
- **Fix:** Reworded the doc comments to describe the commands by category ("a full-keyspace-enumeration or whole-database-wipe command") rather than naming them, and removed the self-referential test assertions entirely (the plan's own acceptance-criteria script already performs this check externally).
- **Files modified:** `crates/paladin-storage/src/node_cache/redis.rs`, `src/config/node_cache.rs` (test not added)
- **Committed in:** `0fdfc7b6`, `db2e6806`

**2. [Rule 1 - Bug] `AsyncIter::next_item` deprecation warning under `-D warnings`**
- **Found during:** Task 2, first `cargo build -p paladin-storage --features redis-cache`
- **Issue:** `redis::AsyncIter` without the `safe_iterators` feature silently stops at the first value that fails `FromRedisValue` conversion rather than surfacing an error, and is deprecated with a warning to that effect -- which `cargo clippy -- -D warnings` would fail on.
- **Fix:** Added `safe_iterators` to the shared `redis` optional dependency's feature list in `paladin-storage/Cargo.toml` (applies to both `redis-queue` and `redis-cache`, since they activate the same dependency line), and updated `invalidate`'s scan loop to handle the now-`Result`-wrapped `next_item()` return.
- **Files modified:** `crates/paladin-storage/Cargo.toml`, `crates/paladin-storage/src/node_cache/redis.rs`
- **Committed in:** `0fdfc7b6`

**3. [Rule 3 - Blocking] `crate::node_cache::contract_tests` not imported in `redis.rs`'s test module**
- **Found during:** Task 2, first `cargo build -p paladin-storage --features redis-cache --all-targets`
- **Issue:** The Tier-2 live-server tests called `contract_tests::run_all`/`contract_tests::sample_delta` without importing the module, producing an unresolved-module compile error.
- **Fix:** Added `use crate::node_cache::contract_tests;` to `redis.rs`'s test module.
- **Files modified:** `crates/paladin-storage/src/node_cache/redis.rs`
- **Committed in:** `0fdfc7b6`

---

**Total deviations:** 3 auto-fixed (2 bugs, 1 blocking compile error). No architectural changes, no scope creep.

**Additional deviation from plan's own RED/GREEN sequencing instruction:** Each task's `<action>` text asks for "write the failing tests first, commit RED, then implement and commit GREEN." As with plan 25-01's precedent, types/adapters/tests were developed together per task and committed as a single `feat` commit rather than as separate RED/GREEN commits, given the size and interdependence of each task's new module tree. Every listed behavior test passes against its task's landed commit; none was skipped or left unverified.

## Issues Encountered

- **Two session interruptions during execution:** a mid-task agent-session crash (before any commit landed) whose uncommitted work-in-progress was inspected as a hint but re-derived from scratch against the current tree per the resume instructions, and a later disk-full crash triggered by a concurrent `cargo clean` elsewhere in the shared devcontainer (unrelated to this plan's own commands, which never ran `cargo clean`). In both cases, this worktree's own git history and working tree were intact on resume; the second interruption's in-flight Task 3 files were recovered exactly as left, verified against the plan's acceptance criteria, and committed immediately.
- **Transient disk exhaustion during the Task 3 workspace-wide verification pass** (`cargo clippy --workspace --all-targets --all-features`, `cargo check --workspace --all-targets --all-features`): concurrent sibling worktree builds temporarily drove available disk space to under 1 GB, failing several crates' compilation with "No space left on device". No file in this worktree was touched to work around it (per the destructive-git-prohibition, sibling worktrees' `target/` directories were never inspected or modified) -- the checks were retried once disk pressure eased and passed cleanly.

## User Setup Required

None — no external service configuration required. Operators who want the Redis-backed cache must explicitly opt in via `NodeCacheConfig { enabled: true, backend: NodeCacheBackend::Redis, .. }` and the `redis-cache` Cargo feature; the default configuration and default feature set are both unaffected.

## Next Phase Readiness

- Plan 25-13 (node-cache engine integration) has a stable `NodeCachePort` to call and two ready backends to call it against: `WarEngine::with_node_cache` can wire either `InMemoryNodeCache` or (behind `redis-cache`) `RedisNodeCache` with no further shape changes expected.
- `NodeCacheKey`'s composition (which fields of the graph fingerprint, node id, input, and Paladin config fingerprint feed into it, per D-28) is explicitly plan 25-13's business, not resolved here -- this plan's `NodeCacheKey` is an opaque `String` wrapper with `as_str()`/`Display`/`starts_with()` only.
- `NodeCacheConfig` resolves into a `RedisNodeCacheConfig` at the wiring layer (plan 25-13/25-14); this plan defines the config surface but does not wire it into `Settings` or the engine.
- The Redis tier's live-server evidence is CI-only: the next engineer touching `redis.rs` should run the new `redis-cache-integration` GitHub Actions job (or `docker compose -f docker/docker-compose.test.yml up -d redis-test` locally if Docker becomes available) rather than assuming the Tier-2 tests have ever executed against a real server in this devcontainer.
- No blockers for the next wave.

---
*Phase: 25-node-level-fault-tolerance*
*Completed: 2026-09-05*

## Self-Check: PASSED

- All 7 created files verified present on disk (`crates/paladin-core/src/platform/container/node_cache.rs`, `crates/paladin-ports/src/output/node_cache_port.rs`, `crates/paladin-storage/src/node_cache/{mod,in_memory,contract_tests,redis}.rs`, `src/config/node_cache.rs`).
- All 3 task commit hashes verified present in `git log` (`f7c0faa3`, `0fdfc7b6`, `db2e6806`).
- `cargo check --workspace --all-targets --all-features` exits 0.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` exits 0.
- `cargo fmt --check` exits 0.
- `cargo test --workspace --lib` exits 0 (all 12 crates, 0 failures).
- `cargo build --workspace --all-features` and `cargo build -p paladin-storage --features redis-cache` both exit 0.
- No `unwrap()`/`expect()`/`panic!` introduced in library (non-test) code across all new/modified files.
