---
phase: 44-legacy-clean-break-removal
plan: 01
subsystem: battalion-orchestration
tags: [aegis, campaign, per-attempt-timeout, retry, commander, tracer, validation]
status: complete

requires:
  - phase: 25-26
    provides: Aegis policy types, engine::retry backoff/predicate, typed NodeError taxonomy
provides:
  - BattalionConfig.aegis (#[serde(default)]), with_aegis and validate_aegis (paladin-core)
  - crate-private aegis_attempt runner (AttemptBound, attempt_bound, AttemptOutcome, AttemptFailure, attempt_once, run_with_aegis) in paladin-battalion
  - Campaign bounded per attempt by Aegis, retrying under aegis.retry, with its whole-run wall clock deleted
  - the per-pattern aegis contract rustdoc that 44-04, 44-05 and 44-07 implement against
affects: [44-04, 44-05, 44-07, 44-10, 44-11]

tech-stack:
  added: []
  patterns:
    - "One crate-private per-attempt runner owns the only tokio::time::timeout of the legacy patterns"
    - "Fail-fast timeout -> BattalionError::Node; any other failure keeps the v0.10 BattalionError::PaladinError text"
    - "Pure policy validation beside the type in paladin-core (validate_aegis), called at service entry"

key-files:
  created:
    - crates/paladin-battalion/src/aegis_attempt.rs
  modified:
    - crates/paladin-core/src/platform/container/battalion/mod.rs
    - crates/paladin-battalion/src/lib.rs
    - crates/paladin-battalion/src/campaign_service.rs
    - crates/paladin-battalion/src/commander.rs

key-decisions:
  - "Campaign ignores aegis.on_error (no continue mode) and warns once per execution"
  - "validate_aegis rejects Route/Custom handlers, Custom retry predicates and zero durations; cache is accepted and ignored"
  - "idle_timeout degrades to a per-attempt wall clock; tighter bound wins, a tie is TimeoutKind::Run"
  - "A timed-out attempt reports PaladinError::Timeout(max(1, whole seconds)); the typed NodeErrorSource::Timeout is authoritative"

patterns-established:
  - "Paused-clock scripted PaladinPort mock (per-call delay + result) for runner and service retry/timeout tests"

requirements-completed: []
requirements-advanced: [LEGACY-01, LEGACY-02]  # Campaign slice only; REQUIREMENTS.md left unchecked until the removal plans (44-04..44-10) land

duration: ~2h of wall clock across two executor sessions (tracer checkpoint in between)
completed: 2026-10-09
---

# Phase 44 Plan 01: Aegis per-attempt runner on Campaign Summary

**Campaign is now bounded per attempt by `BattalionConfig.aegis` through one crate-private runner (`aegis_attempt::run_with_aegis`), retries Transient failures with the engine's backoff, surfaces a timeout as a structured `BattalionError::Node`, and rejects unsupported Aegis policies before any Paladin runs.**

## Performance

- **Duration:** about 2h wall clock (Task 1 at 21:48 UTC, Task 2 at 23:29 UTC, final checks until ~23:50 UTC); includes the user-approval checkpoint between the tasks and disk-space recovery.
- **Tasks:** 2 of 2
- **Files:** 1 created, 4 modified (exactly the plan's five files)

## Accomplishments

- Task 1 (tracer): `BattalionConfig.aegis` + `with_aegis`, the `aegis_attempt` runner, Campaign on the runner with the whole-run `timeout(...)` wrapper deleted, and the end-to-end test `commander_campaign_honours_aegis_per_attempt_timeout_end_to_end` (three 600 ms Paladins complete under a 1 s bound; a 1.5 s Paladin yields `BattalionError::Node` with `Timeout(Run)`, attempt 1, Transient). Approved at the tracer gate.
- Task 2: `BattalionConfig::validate_aegis` (5 table-style tests + 2 serde tests), the full per-pattern contract rustdoc on the `aegis` field, a rewritten struct-level doctest built on `with_aegis`, the `Default` rustdoc updated, the 10-test paused-clock runner matrix, and Campaign wiring (`validate_aegis` at the top of `execute`, one `warn!` when `on_error` is set) with 5 service tests.

## Task Commits

1. **Task 1 (tracer): bound Campaign attempts per Aegis through one runner** - `4d4d272f` (feat)
2. **Task 2: validate Aegis policies and pin the runner retry/timeout matrix** - `3cbe5dcc` (feat)
3. **Task 2 follow-up: keep "per attempt" on one rustdoc line in the aegis contract** - `694cb094` (docs; the plan's acceptance grep needs the literal phrase on one line)

## Verification Results (all actually run)

| Check | Result |
| ----- | ------ |
| `cargo test -p paladin-ai-core --lib validate_aegis` | 5 passed |
| `cargo test -p paladin-ai-core --lib` | 713 passed, 0 failed |
| `cargo test -p paladin-ai-core --doc battalion` | 17 passed, 34 ignored (pre-existing `ignore`s), 0 failed; includes the rewritten `BattalionConfig`, `with_aegis` and `validate_aegis` doctests |
| `cargo test -p paladin-battalion --lib aegis_attempt` | 10 passed |
| `cargo test -p paladin-battalion --lib campaign_service` | 12 passed (includes the 4 plan-named Campaign tests plus `campaign_timeout_surfaces_a_structured_node_error`) |
| `cargo test -p paladin-battalion --lib` | 860 passed, 0 failed |
| `cargo fmt --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean (run after Task 2; the last doc-only commit re-checked with `cargo clippy -p paladin-ai-core ...`, clean) |
| `cargo check --workspace --all-targets --all-features` | exit 0 |
| `cargo doc --workspace --no-deps` (zero `warning:` lines) and `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps` | both clean (the two cargo-doc halves of `make doc-check`; the third half, `cargo test --workspace --doc`, is covered by the workspace test run below) |
| `make api-surface` | "API surface unchanged" |
| `cargo test --workspace --no-fail-fast` | 6870 passed, **2 failed** (see Issues) |

Plan acceptance greps: single `tokio::time::timeout(` site in the non-test runner (1); 0 `timeout(`/`timeout_seconds` in non-test `campaign_service.rs`; 0 `unwrap()/expect(/panic!/unreachable!` in the non-test runner; `validate_aegis` present once; the `aegis` rustdoc contains `per attempt`, `Absorb`, `ChainOfCommand`, `cache`.

## Deviations from Plan

### Auto-fixed / environmental

**1. [Rule 3 - Blocking] Disk exhaustion during the workspace build**
- **Found during:** the plan-level verification (`cargo clippy --workspace`, then `cargo test --workspace`)
- **Issue:** "No space left on device" while ~4 GB was free.
- **Fix:** removed `target/debug/examples` (as the environment notes allow), then `target/doc` and `target/debug/incremental` (~9.7 GB of regenerable build cache), and ran the remaining workspace test with `CARGO_INCREMENTAL=0`. No source change.
- **Commit:** none.

**2. [Rule 1 - Plan-acceptance mismatch] Rustdoc phrase wrapped across lines**
- **Found during:** acceptance greps after Task 2
- **Issue:** the `aegis` field rustdoc said "applied *per / attempt*" across a line break, so the plan's `per attempt` grep matched 0.
- **Fix:** reworded so "per attempt" sits on one line.
- **Files modified:** `crates/paladin-core/src/platform/container/battalion/mod.rs`
- **Commit:** `694cb094`

**3. [Plan reading] TDD red phase not committed separately**
- Task 2 is `tdd="true"`; the tests and the implementation were written together and committed as one `feat` commit, so there is no separate `test(...)` RED commit for it. The runner implementation already existed from the tracer commit, so the runner matrix and Campaign tests pinned existing behaviour; the `validate_aegis` tests were written alongside the function. All tests were run green before commit. No RED failure was observed for `validate_aegis`.

No architectural (Rule 4) changes. Nothing outside the plan's five files changed.

## Issues Encountered

- **2 pre-existing-looking workspace test failures, not fixed (out of scope):**
  `infrastructure::web::run_api_wiring::tests::build_run_api_persists_run_traces_when_trace_persist_is_set` and `..._persists_no_run_traces_by_default` in the root `paladin-ai` lib panic at `src/infrastructure/web/run_api_wiring.rs:2369` ("the run never reached a terminal status"), deterministically on re-run. They drive an agent-kind run through the Run API worker and `WarEngine`, which does not touch Campaign, `aegis_attempt` or `BattalionConfig`. This was NOT confirmed against a pre-phase baseline (a baseline build did not fit the disk budget), so it is reported as "not caused by this plan as far as could be determined", not as "pre-existing". Logged in `deferred-items.md`. The plan's `<verification>` line `cargo test --workspace --no-fail-fast exits 0` is therefore not met as written.

## Known Stubs

None.

## Threat Flags

None. No new network endpoint, auth path or file-access surface. Threat-model mitigations applied: T-44-01 (per-attempt bound through the runner), T-44-02 (`validate_aegis` rejects zero attempts, zero bounds and `Custom` predicates; default `TransientOnly`; backoff via `backoff_delay`), T-44-03 (retry `warn!` carries Paladin name, attempt, transience and delay only), T-44-04 (`validate_aegis` runs before any port call; tested by `campaign_rejects_invalid_aegis_before_any_node_runs`).

## Requirements

LEGACY-01 and LEGACY-02 are advanced, not completed: only Campaign moved to Aegis here, and the legacy `RetryPolicy` / `ErrorStrategy` / `NodeError` / timeout surfaces are still present. `requirements.mark-complete` was run by the state step and then reverted so REQUIREMENTS.md does not claim removal that has not happened.

## Next Phase Readiness

44-04 (Formation), 44-05 (Phalanx) and 44-07 (Commander and Conclave) can call `aegis_attempt::run_with_aegis` / `attempt_once` unchanged and implement against the `BattalionConfig.aegis` rustdoc. The legacy `timeout_seconds`, `retry_policy` and `error_strategy` fields remain compiled and unread by Campaign until 44-10.

## Self-Check: PASSED

Verified: `crates/paladin-battalion/src/aegis_attempt.rs` exists; commits `4d4d272f`, `3cbe5dcc` and `694cb094` are on `claude/laughing-dirac-e0h2ax`; `deferred-items.md` exists. (The workspace test failures above are reported, not hidden; they do not indicate a missing artifact.)
