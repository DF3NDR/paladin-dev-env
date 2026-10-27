---
phase: 43-rate-pacing
plan: 08
subsystem: llm-pacing
tags: [cadence, resilience, degraded-mode, redis-fallback, outage-latch, probe-backoff, never-unpaced]

requires:
  - phase: 43-rate-pacing
    provides: CadencePort, CadencePolicy and InMemoryCadence (43-01); the shared contract suite run_all / run_all_paused (43-05); RedisCadence with its lazy timed connection and URL-free errors (43-07)
provides:
  - ResilientCadence in paladin-storage (new, with_probe_interval, is_degraded, degraded_transitions), always compiled, wrapping any Arc<dyn CadencePort> primary
  - DEFAULT_PROBE_INTERVAL (5 s)
  - InMemoryCadence::with_multiplier and InMemoryCadence::multiplier (stricter degraded delays, applied once, clamped at the 24 h ceiling)
  - a shared test-only capturing logger (cadence/test_logger.rs) for the cadence tests
  - tests unreachable_redis_degrades_within_the_timeout_budget, black_holed_redis_degrades_within_the_timeout_budget (no Redis server needed) and cadence_with_redis_down_still_paces
affects: [43-09, 43-10, 43-13]

tech-stack:
  added: []
  patterns:
    - "Outage latch in a composite adapter, not the decorator: the decorator stays oblivious and the behaviour is testable with an injected failing port"
    - "Healthy-to-degraded edge only emits the warning (one per outage); a single AtomicBool claim with a Drop guard makes at most one caller probe the primary per interval while everyone else is served by the fallback"
    - "Max rule on recovery: gate returns the larger wait and streak of primary and fallback, so a gate recorded during the outage is never released early"

key-files:
  created:
    - crates/paladin-storage/src/cadence/resilient.rs
    - crates/paladin-storage/src/cadence/test_logger.rs
  modified:
    - crates/paladin-storage/src/cadence/mod.rs
    - crates/paladin-storage/src/cadence/in_memory.rs
    - crates/paladin-storage/src/cadence/redis.rs
    - src/infrastructure/cadence.rs
    - MIGRATION.md
    - CHANGELOG.md

key-decisions:
  - "The 'trace warning' of ROADMAP criterion 5 is a log::warn! under paladin::cadence (plus the existing per-call log::trace!); no TraceEvent variant is added (Open Question 5, to be recorded in ADR-0058 by 43-13)"
  - "last_probe is set when the latch trips and again when a probe is claimed, so a slow or failing probe never makes the next one due early"
  - "Only the healthy-to-degraded edge warns and counts; callers already inside the primary when an outage begins may all fail, and only the first latches"
  - "A successful operation on the healthy path also clears the fallback's streak for the key (record_success) and merges the fallback's gate (gate), so nothing recorded in-process outlives its usefulness or is released early"
  - "InMemoryCadence gained a public multiplier() getter so the one outage warning can name the degraded factor without ResilientCadence duplicating state"

patterns-established:
  - "One process-wide capturing logger per test crate (log::set_logger is once per process), per-thread buffers, Info verbosity, shared by every cadence test module"

requirements-completed: [PACE-05]

duration: about 1 h
completed: 2026-10-08
status: complete
---

# Phase 43 Plan 08: Resilient Cadence Summary

**A Redis outage is now a degradation, never an outage and never a hole in pacing: `ResilientCadence` latches on the first primary error, serves the same call and every later one from an in-process fallback with delays multiplied (default 2.0), warns once per outage, probes the primary at most every 5 s without making anyone wait, and recovers automatically without releasing locally recorded gates early.**

## Performance

- **Tasks:** 2 of 2
- **Commits:** 37fddc72 (Task 1), 55aabe0e (Task 2)
- **Files:** 2 created, 6 modified
- **Redis:** none started or needed; every new test uses an injected failing port, a refused port or a local listener that never answers.

## Accomplishments

- `ResilientCadence { primary, fallback, probe_interval, state, probing, transitions }` implements `CadencePort` and never returns `Err`. Healthy: every operation goes to the primary (`gate` returns the larger wait and streak of primary and fallback, `record_success` also clears the fallback streak). First primary error (backend or timeout): latch, one `warn` under `paladin::cadence` naming only the URL-free `CadenceError` and the multiplier, and the same call is served by the fallback. Degraded: the fallback serves everything; when the probe interval has elapsed exactly one caller claims the probe (compare-and-swap flag released by a `Drop` guard, so a cancelled probe never wedges recovery), the others are served by the fallback and never wait; a successful probe clears the latch with one `info`.
- `InMemoryCadence::with_multiplier(f64)`: every delay recorded (computed, explicit, or an in-flight extension) is multiplied exactly once with a panic-free `try_from_secs_f64` and clamped at `CADENCE_DELAY_CEILING`; a non-finite multiplier or one below 1.0 is treated as 1.0, so the fallback can never pace less than the policy. With 2.0 a delay-less first 429 gates for `2 * base` and `Some(3 s)` gates for 6 s.
- Gates recorded during the outage survive recovery: after the recovering probe `gate` returns the fallback's remaining wait (test: 10 s gate, 5 s elapsed, recovery reads 5 s).
- Dead-Redis timing proven without a server: `RedisCadence` at `redis://127.0.0.1:1/0` (refused) and at a local listener that accepts and holds connections without writing (black hole), both with 200 ms timeouts, behind a `ResilientCadence`; `record_rate_limited` then `gate` returns the fallback's multiplied gate in well under 1.5 s (the whole pair ran in about 0.4 s) and leaves `is_degraded()` true.
- End to end in the root crate: `cadence_with_redis_down_still_paces` composes `compose_llm` over a `CadenceWiring` whose port is `ResilientCadence(always-failing, InMemoryCadence x2.0)`; the provider's 429 reaches the caller unchanged, then call 2 reaches the provider no sooner than `2 * base` (paused clock), and no more than 150 ms later.
- The shared contract suite passes unchanged against the composite healthy (in-memory primary, both jitter ends, plus the paused clause) and degraded (always-failing primary, fallback multiplier 1.0).
- Registered: MIGRATION 9.1 (the M-B-05 row now describes the degraded mode and names `degraded_multiplier`), a 9.2 row (`N`, allowlist stays set-equal), CHANGELOG Phase 43 bullet.

## Red / green record

- **Task 1, with_multiplier:** the six multiplier tests were written first and failed to compile (`no method named with_multiplier`); 28 in-memory tests pass after.
- **Task 1, ResilientCadence:** the implementation file was written before its test module (a departure from strict test-first order), so the tests were validated by mutation instead of a red run. Backup and copy-back for each (never `git checkout`): ignoring the fallback in the gate merge failed `gate_recorded_during_the_outage_survives_recovery`; making the probe always due failed `primary_is_probed_at_most_once_per_interval_while_degraded` and `first_primary_error_latches_and_serves_the_same_call_from_the_fallback`; routing the degraded path to the primary failed `one_warning_per_outage_and_one_recovery_line`, `resilient_cadence_passes_the_contract_degraded` and `degraded_delays_use_the_multiplier`. The fourth mutation (warning on every latch call, not just the edge) initially SURVIVED, because the sequential tests never have two callers observe the same outage. `concurrent_failures_in_one_outage_warn_once` was added (two callers held inside the primary, then both fail), and the same mutation now fails it. File restored byte for byte after each, diff against the backup clean.
- **Task 2:** the end-to-end test passed on first run; mutation check: changing the fallback multiplier to 1.0 failed it ("call 2 reached the provider only 535ms after the 429 ... 2 x base = 1s"), then restored.

## Task Commits

1. **Task 1: ResilientCadence, with_multiplier, shared test logger** - `37fddc72`
2. **Task 2: dead-Redis timing tests, never-unpaced end-to-end test, registration** - `55aabe0e`

## Verification

- `cargo test -p paladin-storage --lib cadence::resilient`: 15 passed (acceptance asked for at least 9), including `one_warning_per_outage_and_one_recovery_line`. `--lib cadence`: 43 passed; with `--features redis-cadence`: 63 passed (live-server tests self-skip without a server, as before). `cargo test -p paladin-storage --lib`: 274 passed. Doc tests: 4 passed (two new on `ResilientCadence`, one on `with_multiplier`).
- `cargo test -p paladin-storage --features redis-cadence --lib degrades_within_the_timeout_budget -- --nocapture`: 2 passed, no `SKIP:` line. `cargo test -p paladin-ai --lib cadence_with_redis_down_still_paces`: 1 passed; `--lib cadence`: 15 passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean. `cargo clippy -p paladin-storage --all-targets -- -D warnings`: clean. `cargo fmt --check`: clean. `RUSTDOCFLAGS='-D warnings' cargo doc -p paladin-storage --no-deps --features redis-cadence`: clean.
- `./scripts/check-migration-allowlist.sh`: set-equal. 9.1 `degraded_multiplier` mentions: 1. `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface`: exit 0, "API surface unchanged" (see Deviation 1).
- `make security` was not re-run: no dependency, feature or lockfile change in this plan (it exited 0 at 43-07).
- The CI count guard for `redis-cadence-integration` counts `#[test]`/`#[tokio::test]` attributes in `cadence/redis.rs` and sums the passed lines of the same filter; the two new tests are in that module, run in that job and print no `SKIP:`, so declared and passed stay equal.
- Manual credential-handling review: the outage warning interpolates only the `CadenceError` (URL-free by construction in 43-07) and the multiplier; `one_warning_per_outage_and_one_recovery_line` asserts the captured line carries the stub's marker and no `redis://`. `Debug` for `ResilientCadence` prints the degraded flag and transition count only (test asserts a model string is absent). No log line carries a key, header or URL.

## Deviations from Plan

### Auto-fixed Issues

**1. [Plan acceptance wording] `.project/current-exports.txt` does not contain `ResilientCadence`**
- **Found during:** Task 2
- **Issue:** the acceptance line expects the baseline to contain `ResilientCadence`. The baseline covers only the `paladin` facade crate, which does not re-export `paladin-storage::cadence`, so no sub-crate symbol can appear (same as 43-01 to 43-07).
- **Fix:** none; `make api-surface` exits 0 ("API surface unchanged") and the symbols are covered by the 9.2 register. No re-export added. `.project/current-exports.txt` is unchanged.

**2. [Rule 2 - Correctness] Shared capturing logger for the cadence tests**
- **Issue:** `log::set_logger` may be called once per process, and `in_memory.rs` already installed its own Warn-level capturer; a second one in `resilient.rs` would have panicked or silently captured nothing, and could not see `info` lines.
- **Fix:** a test-only `cadence/test_logger.rs` (per-thread buffers, `Info` verbosity, level and message recorded); the in-memory capacity test now uses it, unchanged in behaviour.
- **Commit:** 37fddc72

**3. [Extra beyond the plan] `InMemoryCadence::multiplier()` getter and extra tests**
- The warning names the degraded factor, which lives in the fallback, so a public getter was added (registered in the 9.2 row). Extra tests beyond the plan's list: a timeout latches like a backend error, concurrent failures in one outage warn once, a cancelled probe releases its claim, a success through a recovering probe clears the fallback streak, `Debug` shows latch state only, and the probe interval is configurable.

**4. [Process note] Implementation written before its tests for `ResilientCadence`** (see the red / green record); mutation checks stood in for a red run.

**Total deviations:** 4 (1 acceptance wording, 1 Rule 2, 1 additive, 1 process note). **Impact:** none on planned behaviour.

## Authentication Gates

None.

## Issues Encountered

- None blocking. No ENOSPC, no `target/` cleanup needed. No Redis server was started, so there was nothing to shut down.
- The `rustfmt` pass (`cargo fmt`) reformatted one assertion in the root-crate test after the first clippy run; fixed before the Task 2 commit.

## Flagged Assumptions

- A1, A2 and A3 of the plan are recorded in the `resilient.rs` module docs: during an outage a worker cannot see other workers' 429s (the multiplier is a heuristic, not a guarantee); gates written to Redis before the outage are invisible to the fallback so the first post-latch call may send without waiting out a pre-outage fleet gate (it is still paced, reactively); recovery is detected within one probe interval plus one round trip and local gates are not copied back into Redis.
- Not wired yet: nothing builds a `ResilientCadence` from configuration. Plan 43-09 constructs it around `RedisCadence` for `backend: redis` and passes `degraded_multiplier` to `with_multiplier`. The lock half of fail-open (D-13) arrives with the lock methods in 43-10.

## Known Stubs

None.

## Threat Flags

None beyond the plan's threat model. Mitigations implemented: T-43-29 (the composite never returns `Err`; bounded 200 ms timeouts proven against a refused and a black-holed server; probe back-off), T-43-30 (stricter multiplier, reactive in-process gating continues, gates survive recovery through the max rule), T-43-31 (one `warn` per outage, one `info` per recovery, `degraded_transitions()`), T-43-32 (URL-free error text, asserted).

## Next Phase Readiness

43-09 wires `backend: redis` to `ResilientCadence(RedisCadence, InMemoryCadence.with_multiplier(degraded_multiplier))` and the matching `APP_TREASURER_CADENCE_*` settings. 43-10 adds the lock methods to `CadencePort`; `ResilientCadence` will need a fail-open lock arm (D-13) alongside the three it has now. 43-13 should record Open Question 5 (log warning, no `TraceEvent` variant) in ADR-0058.

## Self-Check: PASSED

`crates/paladin-storage/src/cadence/resilient.rs` and `crates/paladin-storage/src/cadence/test_logger.rs` exist; commits 37fddc72 and 55aabe0e are present in `git log` on `claude/laughing-dirac-e0h2ax`.
