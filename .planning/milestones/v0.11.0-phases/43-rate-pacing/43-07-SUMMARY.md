---
phase: 43-rate-pacing
plan: 07
subsystem: llm-pacing
tags: [cadence, redis, lua, fleet-pacing, server-clock, timeouts, redaction, ci]

requires:
  - phase: 43-rate-pacing
    provides: CadencePort, CadencePolicy and InMemoryCadence (43-01); the shared contract suite run_all / run_all_paused (43-05)
provides:
  - RedisCadence and RedisCadenceConfig in paladin-storage behind the redis-cadence feature (shared pacing state across a worker fleet)
  - CADENCE_RECORD_429_LUA, CADENCE_GATE_LUA, CADENCE_RECORD_SUCCESS_LUA (one atomic server-clock EVAL per operation, relative waits)
  - DEFAULT_CADENCE_KEY_PREFIX ("paladin:cadence")
  - paladin-storage redis-cadence = ["dep:redis"] and the root passthrough redis-cadence = ["paladin-storage/redis-cadence"], outside default and full
  - crate-private redis_url::redact_connection_url shared by the run queue and the cadence adapter
  - tests two_workers_share_one_gate and cadence_fleet_429_on_one_worker_slows_the_other
  - CI job redis-cadence-integration (logical database 2, fails on SKIP and on fewer passed than declared)
affects: [43-08, 43-09, 43-10, 43-13]

tech-stack:
  added: []
  patterns:
    - "One redis::Script EVAL per atomic operation, now read from the server TIME, relative microsecond waits returned; all variable data through KEYS/ARGV"
    - "Lazy OnceCell<ConnectionManager> built with explicit response and connection timeouts, cloned per call, every operation inside tokio::time::timeout"
    - "Injective key layout: percent-escape % and : in the provider half only, so ordinary keys stay verbatim"

key-files:
  created:
    - crates/paladin-storage/src/cadence/redis.rs
    - crates/paladin-storage/src/redis_url.rs
  modified:
    - Cargo.toml
    - crates/paladin-storage/Cargo.toml
    - crates/paladin-storage/src/lib.rs
    - crates/paladin-storage/src/cadence/mod.rs
    - crates/paladin-storage/src/run_queue/redis.rs
    - src/infrastructure/cadence.rs
    - .github/workflows/ci.yml
    - MIGRATION.md
    - CHANGELOG.md

key-decisions:
  - "Every gate is computed inside Lua on the Redis TIME clock and returned as a relative wait; no client clock ever enters a deadline (D-00f, T-43-26)"
  - "RedisCadence::new is synchronous and offline (it only parses the URL); the first operation connects lazily, and a failed connect leaves the cell empty so the next call retries"
  - "The provider half of a Redis key escapes % and : so ('a:b','c') and ('a','b:c') cannot collide, while openai / gpt-4o stays paladin:cadence:openai:gpt-4o"
  - "A reply wait is clamped to the 24 h ceiling client-side, so a forged or corrupt nb in Redis cannot become an unbounded wait"
  - "The CI count step sums the per-binary 'test result: ok. N passed' lines (storage and the facade fleet test) because one log holds both cargo invocations"

patterns-established:
  - "Live-server tests self-skip with a SKIP: line, use a unique key prefix per test, and the CI job greps the log for SKIP and compares passed against declared"

requirements-completed: [PACE-03]

duration: about 1 h 15 min
completed: 2026-10-08
status: complete
---

# Phase 43 Plan 07: Redis-backed fleet pacing Summary

**A 429 recorded by one worker now gates every worker on the same Redis server: `RedisCadence` runs one atomic Lua script per operation on the server clock, connects lazily under hard timeouts, redacts its URL everywhere, and a mandatory CI job proves it on a live server.**

## Performance

- **Tasks:** 2 of 2
- **Commits:** eb33facf (Task 1), 1b51c1e9 (Task 2)
- **Files:** 2 created, 9 modified
- **Redis used for the live runs:** a local `redis-server` v7.0.15 started on `127.0.0.1:6380` (logical database 2 for the cadence tests, database 1 for the run-queue regression run), no persistence. It was shut down with `redis-cli shutdown nosave` before returning. 7.0.15 is the version the research verified and the major `redis:7-alpine` CI runs.

## Accomplishments

- `RedisCadence` implements `CadencePort` with `CADENCE_RECORD_429_LUA` (in-flight rule, the same back-off arithmetic as `CadencePolicy::delay_for`, `HSET nb`/`streak` via `string.format('%d', ..)`, `PEXPIRE` to `delay + ttl`), `CADENCE_GATE_LUA` (read-only `HMGET`, closed boundary, never creates a key) and `CADENCE_RECORD_SUCCESS_LUA` (resets the streak, drops an elapsed key, leaves an unknown key unknown). All three read `redis.call('TIME')`.
- The shared contract suite `contract_tests::run_all` passes unchanged against the live server at both jitter ends (base 50 ms, max 800 ms), including `streak_saturates_without_overflow` (10 000 sequential records, about a second) and the 16-recorder concurrency clause; no clause needed a wider slack or a lower saturation streak.
- Connection behaviour: `ConnectionManagerConfig` with response and connection timeouts (500 ms each) and one reconnect retry, built lazily in a `tokio::sync::OnceCell`, cloned per call, every operation inside `tokio::time::timeout(response + connection)` surfacing `CadenceError::Timeout` / `Backend`. A listener that accepts and never answers (a black hole) errors well inside the deadline.
- `redact_connection_url` was lifted verbatim into `crates/paladin-storage/src/redis_url.rs` (compiled under `redis-queue` or `redis-cadence`) with its own tests; the run queue imports it. `RedisCadenceConfig` and `RedisCadence` have hand-written `Debug`, and no error contains the URL (test with `redis://:hunter2@127.0.0.1:1/0`).
- Features: `redis-cadence = ["dep:redis"]` on `paladin-storage` and the root passthrough; neither is in `default`, `storage` or `full`. `cargo tree -p paladin-llm -e normal | grep -c ' redis v'` prints 0. No new package in `Cargo.lock`.
- Fleet proof in the root crate: two `RedisCadence` instances (two connections, one shared prefix) wrapped by two `CadenceWiring`s over two mock providers named `openai`. Worker A's provider answers a 429 with a 1 s delay; worker B's next call to the same model reaches its provider no sooner than 900 ms later (it measured about 1.0 s), while a different model on worker B returns in under 200 ms.
- CI job `redis-cadence-integration`: compose `redis-test` on logical database 2, `redis-cli ping` assertion, both `cargo test` invocations into one log, a `SKIP:` grep failure step, and a declared-versus-passed count step; plus the log-collection and teardown steps.
- Registered: MIGRATION 9.2 row (`N`, allowlist stays set-equal), the 9.3 `redis-cadence` feature entry, and the CHANGELOG Phase 43 bullet.

## Red / green record

- **Task 1:** the live-server tests and the implementation were written together. The first run found a real defect: `record_success` returns an integer while the shared `evaluate` helper decoded every reply as `Vec<i64>`, so four tests failed with "Response type not vector compatible" (including the contract suite at its first `record_success`). `evaluate` was made generic over the reply type; 18 passed afterwards, three consecutive times. Mutation check (backup and copy back, no `git checkout`): disabling the in-flight branch (`if false and now < nb`) made `redis_cadence_passes_the_shared_contract` fail. A second mutation, removing `string.format('%d', ..)` from the `nb` write, did NOT fail anything: Redis 7.0.15 stringifies numeric arguments with `%.17g`, which prints a 16-digit microsecond timestamp as plain digits. The `%d` formatting stays as a defence against builds that use `%.14g` (research Pitfall 3), and the test pins the observable (an all-digit string), but it cannot prove the guard on this server version.
- **Task 2:** the fleet test passes in 1.0 s. Mutation check: giving each worker its own key prefix made it fail with "worker B reached its provider only 1.004142ms after worker A's 429", then the file was restored byte for byte.

## Task Commits

1. **Task 1: RedisCadence, Lua scripts, lazy timed connection, shared redaction, features** - `eb33facf`
2. **Task 2: fleet test, CI job, registration** - `1b51c1e9`

## Verification

- `cargo test -p paladin-storage --features redis-cadence --lib cadence::redis -- --nocapture` against the live server: 18 passed, no `SKIP:` line. `--lib cadence` (adds the in-memory suite): 40 passed. The CI commands were run exactly as written (database 2): 18 passed, then `cargo test -p paladin-ai --features redis-cadence --lib cadence_fleet`: 1 passed, `SKIP:` count 0; declared 18 + 1 = 19, passed 19.
- `cargo test -p paladin-storage --features redis-queue --lib run_queue` against database 1: 31 passed (the redaction lift changed nothing there). `cargo test -p paladin-storage --lib` (no features): 253 passed. `cargo test -p paladin-ai --lib cadence`: 14 passed.
- `cargo build -p paladin-storage` and `cargo clippy -p paladin-storage --all-targets -- -D warnings` with the feature off: clean. `cargo clippy -p paladin-storage --features redis-cadence,redis-queue --all-targets -- -D warnings`: clean. `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean. `cargo fmt --check`: clean. `RUSTDOCFLAGS='-D warnings' cargo doc -p paladin-storage --no-deps --features redis-cadence,redis-queue`: clean.
- `make security`: exit 0 (advisories, bans, licenses, sources ok). `./scripts/check-migration-allowlist.sh`: set-equal. 9.3 `redis-cadence` mentions: 1. `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface`: "API surface unchanged" (see Deviation 1).
- The workflow YAML parses and the job has 10 steps; `actionlint` is not installed in this sandbox, so it was not run.
- Manual credential-handling review: the URL is held privately; `Debug` of both types routes it through `redact_connection_url`; the only error strings are the redis crate's own descriptions (OS errors, server replies), and a password-bearing URL is asserted absent from `Display`, `Debug`, the config rendering and the unparsable-URL error. No log line interpolates the URL, a key or a header.

## Deviations from Plan

### Auto-fixed Issues

**1. [Plan acceptance wording] `.project/current-exports.txt` is unchanged**
- **Found during:** Task 2
- **Issue:** the plan lists `.project/current-exports.txt` as modified and asks for `make api-surface-update`. The baseline covers only the `paladin` facade crate (and default features), so no `paladin-storage` symbol can appear in it (same as 43-01 to 43-06).
- **Fix:** none. `make api-surface` exits 0 ("API surface unchanged") without an update; the new symbols are covered by the 9.2 register. No re-export was added.

**2. [Rule 2 - Correctness] Injective Redis key layout**
- **Found during:** Task 1
- **Issue:** the plan's layout `<prefix>:<provider>:<model>` is ambiguous across the boundary: `("a:b", "c")` and `("a", "b:c")` would share a hash, so a 429 for one would gate the other. The port says keys are exact, opaque and independent.
- **Fix:** `%` and `:` are percent-escaped in the provider half only, so ordinary keys (`openai`, any model string) are written verbatim and the plan's layout is unchanged for them. Tests: `state_key_is_injective_across_the_provider_model_boundary` and the live `keys_that_differ_across_the_provider_model_boundary_stay_independent`.
- **Files modified:** `crates/paladin-storage/src/cadence/redis.rs`
- **Commit:** eb33facf

**3. [Rule 2 - Defence] Reply wait clamped to the 24 h ceiling**
- **Issue:** a forged or corrupted `nb` in Redis would otherwise be an unbounded wait handed to the decorator.
- **Fix:** `reading_from` clamps the wait to `CADENCE_DELAY_CEILING` and a negative streak to zero (`a_malformed_reply_is_a_backend_error_and_a_forged_wait_is_clamped`).
- **Commit:** eb33facf

**4. [Rule 1 - Bug in my own first run] Single `Vec<i64>` reply type**
- Fixed within Task 1 before the commit (see the red / green record).

**5. [Plan detail adjusted] CI count step**
- **Issue:** the plan runs two `cargo test` invocations into one log (`tee` then `tee -a`), but the house count step reads only the last `test result` line, which would be the fleet test's single pass.
- **Fix:** the step sums every `test result: ok. N passed` line and compares it to the declared count (storage module `#[test]`/`#[tokio::test]` attributes plus the one fleet test).
- **Commit:** 1b51c1e9

**6. [Extra beyond the plan] Additional tests**
- Beyond the plan's behaviour list: `a_success_on_one_worker_clears_the_streak_for_the_other`, `success_drops_an_elapsed_gate_and_keeps_a_live_one`, `a_black_holed_server_errors_within_the_deadline_instead_of_hanging`, `key_ttl_is_twice_the_maximum_back_off_with_a_sixty_second_floor`, `scripts_read_the_server_clock_and_format_integers` and the pure helper tests. They are all counted in the CI declared total.

**Total deviations:** 6 (1 acceptance wording, 2 Rule 2, 1 Rule 1 caught before commit, 1 plan detail, 1 additive tests). **Impact:** none on the planned behaviour.

## Authentication Gates

None.

## Issues Encountered

- None blocking. Disk stayed at about 6.5 GB free throughout; no `target/` cleanup was needed. The two known pre-existing `build_run_api_persists_*` failures under `--features web-server` were not touched and not run.
- The saturation clause (10 000 sequential records) takes about a second against loopback Redis, so `SATURATION_STREAK` was not lowered and `slack()` in `contract_tests.rs` was not widened; a CI runner with a slower docker network path may differ, which is the first thing to look at if the CI job is ever flaky.

## Flagged Assumptions

- A1 (one Redis primary), A2 (Redis 5 or later; verified on 7.0.15) and A3 (the fixed key namespace, so two independent fleets on one Redis share pacing state for the same provider and model) are recorded in the `cadence/redis.rs` module docs and MIGRATION 9.3. A3 still needs a line in the 43-09 configuration docs.

## Known Stubs

None.

## Threat Flags

None beyond the plan's threat model. Mitigations implemented: T-43-23 (private URL, hand-written `Debug`, redacted errors, password-bearing-URL tests), T-43-24 (const scripts; provider, model and numbers only through KEYS/ARGV), T-43-25 (explicit timeouts, outer deadline, lazy connect, black-hole test), T-43-26 (`TIME` inside every script, relative waits), T-43-27 (keys created only by a 429, `PEXPIRE` on every write, gate reads create nothing, `PTTL > 0` test). T-43-28 is accepted as planned.

## Next Phase Readiness

43-08 wraps `RedisCadence` in `ResilientCadence` (degraded mode on `Timeout`/`Backend`); it can run `contract_tests::run_all` over the composite. 43-09 wires `backend: redis` and must read the URL from the configured env var, build `RedisCadenceConfig::new(url)` and never log it. 43-10 can add its lock scripts beside the three here and reuse `evaluate`, `state_key`-style key building and the lazy manager.

## Self-Check: PASSED

`crates/paladin-storage/src/cadence/redis.rs` and `crates/paladin-storage/src/redis_url.rs` exist; commits eb33facf and 1b51c1e9 are present in `git log` on `claude/laughing-dirac-e0h2ax`.
