---
phase: 57-register-hygiene-close-out
plan: 15
subsystem: cadence-rate-pacing
tags: [cadence, rate-pacing, retry-after, cancellation-safety, windows-ledger, d-14]
status: complete

requires:
  - phase: 57-register-hygiene-close-out
    provides: "57-10 triage (owner option-a) that sent 43-REVIEW WR-05 and WR-04 to fix-in-phase as correctness items"
provides:
  - "A zero provider delay (Retry-After 0, a past HTTP-date, a zero reset) is no provider delay: CadenceLlmAdapter::minimum_delay returns None, so the port gates on its own back-off (at least base, streak escalating) instead of opening a zero-length gate (43 WR-05)"
  - "note_failure records the 429 in a task spawned on the current tokio runtime (Handle::try_current, inline fallback) that the call awaits, so a caller dropped after the provider's response cannot lose the record (43 WR-04)"
  - "WINDOWS rows 98 (43-REVIEW WR-05) and 99 (43-REVIEW WR-04), open, for 57-11 to mark fixed on this plan's tests and commits"
  - "paladin-llm CHANGELOG [Unreleased] Fixed, root CHANGELOG [Unreleased] Fixed x2, MIGRATION 11.1 rows M-B-45 and M-B-46, ADR-0058 (d) and (i) dated amendments"
affects: [57-11]

tech-stack:
  added: []
  patterns:
    - "Cancellation-safe port record: clone the Arc port and key into a spawned task, await the JoinHandle, fall back to the inline call when no runtime handle exists (same shape as 57-14's run_detached)"
    - "A zero provider delay is filtered where the adapter reads it (minimum_delay), leaving CadencePolicy::delay_for and its pinned contract untouched"

key-files:
  created: []
  modified:
    - .planning/WINDOWS.md
    - crates/paladin-llm/src/cadence.rs
    - crates/paladin-llm/CHANGELOG.md
    - CHANGELOG.md
    - MIGRATION.md
    - .planning/decisions/0058-rate-pacing-cadence.md

key-decisions:
  - "PD-15-1: the zero filter lives in minimum_delay, not CadencePolicy::delay_for, so paladin-ports stays untouched (git diff 920d631ec -- crates/paladin-ports/ is empty)"
  - "PD-15-2: spawn and await. Awaiting keeps the next call's gate read ordered after the record; spawning means a dropped caller cannot cancel it. The at-most-once port-error warn flag is passed into the task and handed back"

patterns-established:
  - "A 429 is recorded on every path but a drop before the record task is spawned and a process exit while it runs (EA-57-17)"

requirements-completed: [REG-05, REG-02]

duration: ~42min
completed: 2026-11-04
---

# Phase 57 Plan 15: Cadence zero-delay gate and cancellation-safe 429 record Summary

**A zero provider delay (`Retry-After: 0`, a past HTTP-date, a zero reset) no longer opens a zero-length pacing gate, and a 429 is recorded in a spawned, awaited task so a caller dropped after the provider's response cannot lose it.**

## Performance

- **Duration:** ~42 min (two executors: Task 1 and its tracer checkpoint, then Tasks 2 and 3)
- **Started:** 2026-11-04T20:22Z (plan-start SHA `920d631ecd708c05b8e5956bb3aab7e4d1e9deb1`)
- **Completed:** 2026-11-04T21:05Z
- **Tasks:** 3 (1 tracer, 1 tdd auto, 1 auto)
- **Files modified:** 6

## Accomplishments

- 43-REVIEW WR-05: in `minimum_delay`, `err.retry_after().filter(|delay| !delay.is_zero())?` runs before the reset-derived clamp, so `Some(Duration::ZERO)` becomes `None` and `CadencePolicy::delay_for` takes its back-off branch (at least the base delay, streak incremented). A non-zero provider delay is still returned exactly. The rustdoc cites 43-REVIEW WR-05.
- 43-REVIEW WR-04: `note_failure` keeps the `RateLimitExceeded` check and `minimum_delay`, then, when `tokio::runtime::Handle::try_current()` gives a handle, clones the `Arc<dyn CadencePort>` and the `CadenceKey`, spawns `record_rate_limited` and awaits the `JoinHandle`. A port error is logged by the task through `warn_port_error` (the call's once-per-call `warned` flag goes in and comes back, so the "at most once per call" rule still holds); a `JoinError` is logged at `warn` under `CADENCE_LOG_TARGET` naming provider and model only. With no runtime handle the record runs inline as before. The call's result is never altered.
- No public item changed: `git diff 920d631ec -- crates/paladin-ports/` is empty, `make api-surface` reports unchanged (4331 items) and the `paladin-llm` semver run reports `0 failure(s)`.
- Records: `paladin-llm` CHANGELOG `## [Unreleased]` `### Fixed` (new section above `## [0.11.0]`), root `[Unreleased]` `### Fixed` x2, MIGRATION 11.1 `M-B-45` and `M-B-46` (`## 11.2` untouched), ADR-0058 `Amended 2026-11-04 (Phase 57).` under Status plus dated amendments in (d) and (i) with no line deleted (`--numstat`: 5 added, 0 deleted). `# 10.` onward in MIGRATION.md and `## [0.11.0]` onward in both changelogs are byte-identical to the pre-commit versions (D-00c).

## Task Commits

1. **Task 1 (tracer): 43 WR-05, zero provider delay never opens a zero-length gate** - `5adfda4d9` (fix)
2. **Task 2: 43 WR-04, a 429 is recorded even when the paced call's future is dropped** - `a10f57c7e` (fix)
3. **Task 3: changelogs, MIGRATION and ADR-0058 amendments** - `97db8fe04` (docs)

## WINDOWS rows minted

| Id | Description starts | File:line | Kind / phase | Status |
|----|--------------------|-----------|--------------|--------|
| 98 | `43-REVIEW WR-05 (found live by the Phase 57 sweep, D-14): minimum_delay returns err.retry_after() unfiltered ...` | `crates/paladin-llm/src/cadence.rs:291` | `deviation` / `43` | `open` |
| 99 | `43-REVIEW WR-04 (found live by the Phase 57 sweep, D-14): note_failure awaits record_rate_limited inline after the provider's 429 ...` | `crates/paladin-llm/src/cadence.rs:275` | `deviation` / `43` | `open` |

Both were minted before their fix commits (EA-57-16 guard: `windows status` showed no earlier `43-REVIEW WR-05` or `43-REVIEW WR-04` row, so no duplicate) and stay `open`; 57-11 marks them `fixed` on this plan's tests and commits.

## Red-green

Tests are `cadence::tests::<name>` in `crates/paladin-llm/src/cadence.rs`.

| Test | RED on the pre-fix head | GREEN |
|------|-------------------------|-------|
| `a_zero_retry_after_gates_at_least_the_base_delay` | `a zero Retry-After is no provider delay; the gate was 0ns` | passed |
| `a_past_http_date_retry_after_gates_at_least_the_base_delay` | `a past HTTP-date is no provider delay; the gate was 0ns` | passed |
| `a_zero_reset_gates_at_least_the_base_delay` | `a zero reset is no provider delay; the gate was 0ns` | passed |
| `repeated_zero_delay_429s_escalate_the_streak` | `the gate was 0ns` | passed |
| `a_nonzero_retry_after_below_base_is_still_honoured_exactly` | passed on the head (guard test, P-15-2) | passed |
| `a_429_is_recorded_when_the_generate_future_is_dropped_mid_record` | `the 429 was lost: the record died with the dropped caller` (`recorded` false after the release) | passed |
| `a_429_is_recorded_when_the_stream_future_is_dropped_mid_record` | `the stream's 429 was lost: the record died with the dropped caller` (`recorded` false after the release) | passed |

Task 1 head run: `cargo test -p paladin-llm --lib cadence::tests::` gave 29 passed, 4 failed. Task 2 head run: `cargo test -p paladin-llm --lib cadence::tests::a_429_is_recorded_when_the_` gave 0 passed, 2 failed; after the fix both pass.

The Task 2 tests use a test-only `ParkedRecordPort` (wraps an `InMemoryCadence`; `record_rate_limited` signals `entered`, waits on `release`, delegates, then sets `recorded`). Each spawns the paced call, awaits `entered`, aborts the task (the dropped caller), sends `release.notify_one()` and yields in a bounded loop until `recorded`, then reads the inner gate for streak 1.

Suites after the fixes: `cargo test -p paladin-llm --lib cadence::` 35 passed, 0 failed (33 after Task 1, plus the two drop tests); `cargo test -p paladin-llm --lib fallback::` 63 passed, 0 failed; `cargo test -p paladin-ports --lib cadence_port::` 10 passed (Task 1; `paladin-ports` is unchanged since), including `policy_retry_after_is_returned_exactly_and_clamped_to_the_ceiling`.

## Gates

`df -BM --output=avail /` was printed before the builds (7337M at start, 7336M before clippy, 7330M after `make clean-code`, 7511M before the semver run). Every cargo run used `CARGO_INCREMENTAL=0`.

| Gate | Result |
|------|--------|
| `cargo fmt --all -- --check` | clean (Tasks 1 and 2) |
| `cargo clippy -p paladin-llm --lib --tests -- -D warnings` | exit 0 (`Finished dev profile`) |
| `make clean-code` | exit 0 (doc-check passed, ADR-0033; all 105 D-05 public API entry points carry a plural `# Examples` heading) |
| `make api-surface` | `API surface unchanged` (4331 items) |
| `SEMVER_PRUNE_LOCAL_CACHE=1 CARGO_INCREMENTAL=0 ./scripts/check-semver.sh --package paladin-llm` | `paladin-llm: 223 checks: 223 pass, 31 skip`; `1 package(s) checked against 0.11.0 at --release-type patch; 0 failure(s)` |
| `make security` | exit 0 (`advisories ok, bans ok, licenses ok, sources ok`); run although the plan does not list it, per CLAUDE.md |
| `make check-gates` | exit 0 (12 crates in publish order; version strings equal `0.11.0`; stub-panic scan `0 finding(s)`) |
| `git diff 920d631ec -- crates/paladin-ports/` | empty (P-15-1, P-15-4) |

## Evidence for 57-11

- **43 WR-05:** WINDOWS row **98** (open, kind `deviation`, phase `43`); fix commit `5adfda4d9` (Task 1). Tests: `cadence::tests::a_zero_retry_after_gates_at_least_the_base_delay`, `cadence::tests::a_past_http_date_retry_after_gates_at_least_the_base_delay`, `cadence::tests::a_zero_reset_gates_at_least_the_base_delay`, `cadence::tests::repeated_zero_delay_429s_escalate_the_streak`, `cadence::tests::a_nonzero_retry_after_below_base_is_still_honoured_exactly`. Command: `cargo test -p paladin-llm --lib cadence::`.
- **43 WR-04:** WINDOWS row **99** (open, kind `deviation`, phase `43`); fix commit `a10f57c7e` (Task 2). Tests: `cadence::tests::a_429_is_recorded_when_the_generate_future_is_dropped_mid_record`, `cadence::tests::a_429_is_recorded_when_the_stream_future_is_dropped_mid_record`. Command: `cargo test -p paladin-llm --lib cadence::tests::a_429_is_recorded_when_the_`.
- **Both:** records commit `97db8fe04` (Task 3): `crates/paladin-llm/CHANGELOG.md` `## [Unreleased]`, root `CHANGELOG.md` `[Unreleased]` `### Fixed`, `MIGRATION.md` 11.1 `M-B-45` and `M-B-46`, `.planning/decisions/0058-rate-pacing-cadence.md` (d) and (i) amendments.

## Deviations from Plan

None - plan executed exactly as written. Two notes, neither a deviation: `make security` was run although the plan's gate list omits it (CLAUDE.md requires it on modified code); and the `warned` flag is threaded through the spawned task and handed back, which the plan described as "a fresh `warned` flag", so the existing once-per-call logging contract is preserved rather than allowing a second warn after a failed `gate`.

## Auth gates

None.

## Known Stubs

None. `ParkedRecordPort` is a test-only double by design.

## Threat Flags

None. The change adds no endpoint, auth path, file access or schema change. The new `JoinError` log line names provider, model and the join error only; no header value, API key or response body is interpolated (T-57-52..T-57-55 mitigations hold). One short task per observed 429, bounded by the port's own timeout (T-57-54, accepted).

## Issues Encountered

None. Residual windows (EA-57-17), stated in the ADR-0058 (i) amendment: a drop before the record task is spawned, and a process exit while it runs, can still lose the 429.

## Self-Check: PASSED

- FOUND: `Handle::try_current` in `note_failure` and `is_zero` in `minimum_delay` (`crates/paladin-llm/src/cadence.rs`)
- FOUND: the seven new tests and `ParkedRecordPort` in `crates/paladin-llm/src/cadence.rs`
- FOUND: `## [Unreleased]` in `crates/paladin-llm/CHANGELOG.md`; `M-B-45` and `M-B-46` in MIGRATION.md; both `Phase 57, 43-REVIEW` amendments in ADR-0058
- FOUND: WINDOWS rows 98 and 99, `open`
- FOUND: commits `5adfda4d9`, `a10f57c7e` and `97db8fe04`
