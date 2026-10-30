---
phase: 42-mid-run-halt-sse-terminal-status
plan: 06
subsystem: run-streaming
tags: [allowance, mid-run-halt, sse, cancel, drain, run-finished, cancel-probe, tests, docs]

requires:
  - phase: 42-mid-run-halt-sse-terminal-status
    provides: HaltCause on RunOutcome::Halted and the cause-aware map_outcome (plan 42-02); run_finish_status returning (status, reason) and RunFinished.halt_reason, the done builders on the live, degraded and replay paths, replay_terminal_override (plan 42-05); ADR-0057 and the option-b gate decision (plan 42-01)
provides:
  - RunFinishStatus::Cancelled (the enum is now #[non_exhaustive], still Copy)
  - the engine reporting HaltCause::CancelRequested as RunFinished Cancelled, and the mid-superstep aborted-node halt resolving its cause through the probe
  - PerRunCancelProbe, attached by the run worker to every factory-built engine so a same-instance caller cancel reaches the engine as a caller cancel
  - RunEventBusSink::with_shutdown_token, dropping a reasonless Halted RunFinished while the worker is shutting down
  - SSE done with status cancelled for a caller cancel on both routes, and no terminal event for a drain
  - the assumption-delta invariant test every_halt_cause_maps_to_one_status_on_every_leg, plus MIGRATION 9.2/9.6, platform-api.md, CHANGELOG and baseline entries
affects: [42-07, 42-09, 42-10, 42-12]

tech-stack:
  added: []
  patterns:
    - "Per-run probe composition: the durable debounced probe ORed with the run's own child token while the shutdown token (its parent) is not cancelled, so a caller cancel and a drain are distinguishable at the engine"
    - "Terminal-event suppression at the bus sink keyed on the coordinator's shutdown token and the absence of a halt_reason, leaving the run row and queue to the worker's LeaveRunningAndRequeue arm"
    - "Cancel and drain scripts added to the stream_tests HaltedRun rig, so one invariant test drives spend halts and both cancel routes through the same real worker pool"

key-files:
  created: []
  modified:
    - crates/paladin-core/src/platform/container/trace.rs
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/src/engine/superstep.rs
    - src/application/services/run/cancel.rs
    - src/application/services/run/mod.rs
    - src/application/services/run/worker.rs
    - src/application/services/run/events.rs
    - src/application/services/run/cancel_tests.rs
    - src/application/services/run/stream_tests.rs
    - docs/src/api-reference/platform-api.md
    - MIGRATION.md
    - CHANGELOG.md
    - .project/current-exports.txt
    - .planning/phases/42-mid-run-halt-sse-terminal-status/deferred-items.md

key-decisions:
  - "The per-run probe is attached on every factory-built engine, with the pool's debounced durable probe optional inside it, so the same-instance fast path works even when no durable-flag probing is wired"
  - "While the shutdown token is cancelled the local child token says nothing about the caller (it is a child of the shutdown token), so only the durable flag can report a cancel during a drain"
  - "The drain rule applies only to a reasonless Halted: a spend halt that lands during shutdown is a genuine finish and is still emitted and recorded Halted"
  - "RunFinishStatus is registered as N (it did not exist at the v0.9.0 baseline), so no allowlist entry is added and the set-equality check stays satisfied"
  - "The agent-kind worker path also gets with_shutdown_token on its bus sink, for uniformity; a reasonless Halted there is equally not a finish"

patterns-established:
  - "HaltScript::CancelSameInstance and HaltScript::CancelCrossInstance in stream_tests.rs: drive_halted_run cancels once the first superstep event is seen, through a RunSubmissionService wired to the pool's local tokens (same instance) or to nothing (another instance)"

requirements-completed: [PLAT-09]

coverage:
  - id: D1
    description: "RunFinishStatus gains Cancelled and is #[non_exhaustive] while staying Copy; the engine maps HaltCause::CancelRequested to Cancelled, a token halt to Halted and a spend halt to Halted with its reason; a stored halted row still reads back Halted and cancelled round-trips"
    requirement: "PLAT-09"
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/trace.rs#tests::run_finished_cancelled_round_trips, #legacy_run_finished_json_reads_back_with_no_halt_reason (cargo test -p paladin-ai-core --lib trace: 16 passed); crates/paladin-battalion/src/engine/mod.rs#spend_guard_tests::run_finish_status_names_a_reason_only_for_a_spend_halt, #non_spend_run_finished_carries_no_halt_reason (cargo test -p paladin-battalion --lib spend_guard: 7 passed)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The mid-superstep aborted-node halt resolves its cause through the probe: a probe reporting a cancel gives CancelRequested, a quiet probe gives Token"
    requirement: "PLAT-09"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::mid_flight_abort_with_a_cancelling_probe_halts_as_cancel_requested, #mid_flight_abort_with_a_quiet_probe_halts_as_token (cargo test -p paladin-battalion --lib mid_flight: 2 passed)"
        status: pass
    human_judgment: false
  - id: D3
    description: "PerRunCancelProbe answers true for the durable flag, or for a local cancel while the shutdown token is not cancelled, and answers false for a pure drain; its doc test compiles and passes"
    requirement: "PLAT-09"
    verification:
      - kind: unit
        ref: "src/application/services/run/cancel.rs#tests::per_run_probe_reports_a_local_cancel_while_not_shutting_down, #per_run_probe_treats_a_drain_as_not_a_caller_cancel, #per_run_probe_reports_the_durable_flag_even_while_shutting_down (cargo test -p paladin-ai --lib application::services::run::cancel: 11 passed); cargo test -p paladin-ai --doc PerRunCancelProbe: 1 passed"
        status: pass
    human_judgment: false
  - id: D4
    description: "map_trace_event renders RunFinished Cancelled as done with status cancelled and no halt_reason key, and drops a status it does not understand; the bus sink drops a reasonless halt while shutting down, publishes a spend halt, publishes a reasonless halt when not shutting down, and never suppresses a cancelled finish"
    requirement: "PLAT-09"
    verification:
      - kind: unit
        ref: "src/application/services/run/events.rs#tests::cancelled_run_finished_maps_to_done_cancelled_without_a_reason, #bus_sink_drops_a_reasonless_halt_while_shutting_down, #run_finished_status_splits_done_and_error (cargo test -p paladin-ai --lib application::services::run::events: 18 passed); cargo test -p paladin-ai --doc with_shutdown_token: 1 passed"
        status: pass
    human_judgment: false
  - id: D5
    description: "A caller cancel through the dispatching instance (no durable probing wired) and through another instance (debounced probe) both stream done with status cancelled and no halt_reason, and the row is Cancelled"
    requirement: "PLAT-09"
    verification:
      - kind: integration
        ref: "src/application/services/run/cancel_tests.rs#same_instance_cancel_streams_done_cancelled, #cross_instance_cancel_streams_done_cancelled (a mutation disabling the local-token branch of PerRunCancelProbe turned the same-instance test red with done status halted)"
        status: pass
    human_judgment: false
  - id: D6
    description: "A worker drain mid-run streams no done and no error to a subscriber, leaves the run row Running and requeues the message"
    requirement: "PLAT-09"
    verification:
      - kind: integration
        ref: "src/application/services/run/stream_tests.rs#drain_streams_no_done_and_leaves_the_run_running (a mutation disabling the sink's drain rule turned it red with a halted done)"
        status: pass
    human_judgment: false
  - id: D7
    description: "Invariant: an allowance-exhausted halt, a ledger-unavailable halt, a same-instance cancel and a cross-instance cancel each produce the same status string on the run row, the live done, the degraded done and the replay done"
    requirement: "PLAT-09"
    verification:
      - kind: integration
        ref: "src/application/services/run/stream_tests.rs#every_halt_cause_maps_to_one_status_on_every_leg (cargo test -p paladin-ai --lib application::services::run: 236 passed, also under --all-features)"
        status: pass
    human_judgment: false
  - id: D8
    description: "Edge (PLAT-09, concurrency): a halt observed at the exact instant shutdown begins is classified once, either suppressed as a drain or emitted as terminal with a matching row, and a degraded reconnect converges on the row (shutdown-boundary race, not deterministically reproducible in a test; RESEARCH A7)"
    requirement: "PLAT-09"
    verification:
      - kind: backstop
        ref: "42-RESEARCH.md A7; the degraded and replay paths read the run row, which is the single source of truth, so a reconnect converges"
        status: pass
    human_judgment: false
  - id: D9
    description: "The status variant, the new builder and probe, the SSE behaviour change and the baseline are registered and documented"
    requirement: "PLAT-09"
    verification:
      - kind: other
        ref: "./scripts/check-migration-allowlist.sh exit 0 (RunFinishStatus registered N, no allowlist entry); MIGRATION.md 9.2 rows for RunFinishStatus and RunEventBusSink/PerRunCancelProbe and the 9.6 entry; platform-api.md cancelled and drain paragraphs; CHANGELOG Changed bullet; PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface-update then make api-surface unchanged (4203 items)"
        status: pass
    human_judgment: false

duration: ~45min
completed: 2026-10-06
status: complete
---

# Phase 42 Plan 06: SSE says cancelled, and a drain says nothing Summary

**A caller-cancelled run now streams `done` with `status: "cancelled"` on both the same-instance and the cross-instance route, through a new `RunFinishStatus::Cancelled` and a per-run `PerRunCancelProbe`, while a worker drain streams no terminal event and leaves the run `running` and requeued.**

## Performance

- **Duration:** ~45 min (two task commits plus reading, mutation checks, gates and this summary)
- **Started:** 2026-10-06T20:15Z
- **Completed:** 2026-10-06T21:00Z (code); state updates follow
- **Tasks:** 2 (both auto, TDD)
- **Files modified:** 14, none created

## Accomplishments

- **Cancelled at the source (Task 1).** `RunFinishStatus` gains the unit variant `Cancelled` and is marked `#[non_exhaustive]`, staying `Copy` with snake_case serde. `run_finish_status` maps `HaltCause::CancelRequested` to `(Cancelled, None)`, a bare token halt to `(Halted, None)` and a spend halt to `(Halted, Some(reason))`. The mid-superstep aborted-node halt consults the attached probe once to choose between `CancelRequested` and `Token`, so a node aborted by a caller cancel no longer reports a bare token halt.
- **The per-run probe (G1).** A same-instance cancel only fires the run's child token, which is a child of the shutdown token, so the engine could not tell it from a drain. `PerRunCancelProbe { db, local, shutdown }` answers `true` for the durable debounced flag, or for the local token while the shutdown token is not cancelled. The worker attaches it to every factory-built engine in place of the pool-wide probe, wrapping that probe when `with_cancellation_probing` was called. `map_outcome` is unchanged: its `Token` arm keeps the `is_cancel_requested` and `shutting_down` re-query for a residual bare-token halt.
- **Cancelled done.** `map_trace_event` renders `RunFinished { Cancelled }` as `done` with status `cancelled` and no `halt_reason` key, and a status it does not understand (the enum is now `#[non_exhaustive]`) is dropped through a wildcard arm, mirroring the `TraceEvent` wildcard; the row stays the truth and a degraded reconnect resolves it.
- **A drain is not a finish (Task 2).** `RunEventBusSink::with_shutdown_token` makes the sink drop a `RunFinished { status: Halted, halt_reason: None }` while the coordinator's shutdown token is cancelled. Both worker paths (the engine path and the agent-kind path) build their bus sink with it. A spend halt that lands during shutdown names a reason, so it is still emitted and recorded `Halted`.
- **End-to-end proof.** `cancel_tests.rs` drives a slow chain through a real pool with a live bus and cancels once the run is going, by each route (`same_instance_cancel_streams_done_cancelled` with no durable probing wired, `cross_instance_cancel_streams_done_cancelled` with a 50 ms debounced probe). `stream_tests.rs` gains `drain_streams_no_done_and_leaves_the_run_running` and the assumption-delta invariant `every_halt_cause_maps_to_one_status_on_every_leg`, which reuses the `HaltedRun` rig with two new scripts (`CancelSameInstance`, `CancelCrossInstance`) and compares the row, live, degraded and replay statuses for four causes.
- **Registers and docs.** MIGRATION.md 9.2 rows for `RunFinishStatus` (N, absent at the v0.9.0 baseline) and for `RunEventBusSink`/`PerRunCancelProbe`, a 9.6 entry, the platform-api.md `done` status list with a cancel paragraph and a drain paragraph, a CHANGELOG bullet, and the regenerated public-API baseline.

## Task Commits

1. **Task 1: RunFinishStatus::Cancelled and a per-run cancel probe** - `b482701e` (feat)
2. **Task 2: a drain emits no done; cancel routes and the cause-to-status invariant proven** - `6869d5f5` (feat)

## Gates and tests run

- `cargo test -p paladin-ai-core --lib trace`: 16 passed. `cargo test -p paladin-battalion --lib cancellation_probe`: 4 passed. `cargo test -p paladin-battalion --lib spend_guard`: 7 passed. `cargo test -p paladin-battalion --lib mid_flight`: 2 passed.
- `cargo test -p paladin-ai --lib application::services::run::cancel`: 11 passed (3 new `PerRunCancelProbe` tests plus the existing probe and cancel cases). `...::events`: 18 passed. `cargo test -p paladin-ai --lib application::services::run`: 236 passed with default features and 236 passed with `--all-features`, covering the new cancel, drain and invariant tests.
- `cargo test -p paladin-ai --doc PerRunCancelProbe`: 1 passed. `cargo test -p paladin-ai --doc with_shutdown_token`: 1 passed.
- Mutation checks: with the local-token branch of `PerRunCancelProbe` short-circuited, `same_instance_cancel_streams_done_cancelled` turned red with a live done of `halted`; with the shutdown rule in `RunEventBusSink::on_event` short-circuited, `drain_streams_no_done_and_leaves_the_run_running` turned red with a `halted` done. Both changes were reverted before the commits.
- `cargo check --workspace --all-targets --all-features` exit 0 (integration tests under `tests/` compile). `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean. `cargo fmt --check` clean.
- Orchestrator gate: `cargo build --workspace --all-features` exit 0, then the workspace lib and bin test run (every target executes): every suite green except `paladin-ai --lib`, 1233 passed and 2 not passing, the known sandbox-only pair `infrastructure::web::run_api_wiring::tests::build_run_api_persists_no_run_traces_by_default` and `build_run_api_persists_run_traces_when_trace_persist_is_set` (no outbound network; already logged in this phase's `deferred-items.md`, untouched by this plan).
- `./scripts/check-migration-allowlist.sh` exit 0; `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface-update` then `make api-surface`: baseline refreshed (4195 to 4203 items: `PerRunCancelProbe`, its re-export, and `RunEventBusSink::with_shutdown_token`) and then unchanged on re-check.
- Not run here: the PostgreSQL contract legs and the Redis integration tests (no servers in this sandbox). `make security` (cargo-audit and cargo-deny) was not run: this plan adds no dependency (`tokio-util` was already a dependency of the facade crate).
- Manual credential-handling review: the cancelled `done` payload carries only `status`, `waypoint_id`, `usage` and `trace_seq`, no halt reason, tenant id or key name; no log line was added; the cancel route's authorisation gate is unchanged; the probe reads an in-memory token and the existing debounced repository flag.

## Decisions Made

- Attach the per-run probe on every factory-built engine, with the durable probe optional inside it, rather than only when `with_cancellation_probing` was called: the same-instance route must not depend on durable probing being wired (G1b).
- During a drain only the durable flag counts, because the local child token is cancelled by the shutdown token itself and carries no information about the caller.
- Suppress at the bus sink, keyed on the shutdown token and the absence of a reason, instead of adding a new engine outcome: the worker already leaves the run `Running` and requeues it for a token halt during shutdown, so only the stream needed to agree.
- Register `RunFinishStatus` as N with the same disposition as the `TraceEvent` rows (the type did not exist at the v0.9.0 baseline), so the allowlist is untouched.
- Add the shutdown token to the agent-kind worker path's bus sink as well; it costs nothing and keeps the two paths uniform.

## Deviations from Plan

### Auto-fixed Issues

None - the plan was executed as written; the engine-side tests for the mid-superstep cause split use a new low-level helper (`run_with_shutdown_grace_and_probe`) in `superstep.rs`'s test module, which the plan's "add an engine test" instruction implies.

### Plan wording interpreted

- **Extra scope not taken.** `paladin-eval`'s `RunStatusValue` mirrors `RunFinishStatus` and has no `Cancelled` value, so a scenario cannot assert a cancelled finish. Adding it changes a generated schema and `paladin-eval`'s public surface, neither in this plan's file list; logged in `deferred-items.md`.
- **Commit trailers.** The orchestrator's note names `Co-Authored-By: Claude Fable 5.1`, and the two task commits carry that line. The harness attribution instruction for this session names `Claude Sonnet 5.5` (as the 42-05 commits do); rewriting the two task commits to match was denied by the permission classifier as a destructive git action, so they keep the Fable 5.1 trailer and the later tracking commits carry the harness-named trailer. The orchestrator may want to normalise this when it pushes.
- **Files outside the plan's list.** `src/application/services/run/mod.rs` gained the `PerRunCancelProbe` re-export, and `deferred-items.md` gained the entry above.

**Total deviations:** 0 auto-fixed, 3 wording interpretations; none change a decision or the locked design.

## Auth Gates

None.

## Issues Encountered

- Pre-existing and out of scope: the two `run_api_wiring` tests named above do not pass in this sandbox (no outbound network).
- Disk stayed near 7 GB free; every cargo command ran with `CARGO_INCREMENTAL=0` and no cache deletion was needed.

## Known Stubs

None.

## Threat Flags

None. No new endpoint, auth path or file-access pattern. T-42-23 (a terminal done for a drained run) is mitigated by the sink rule and covered by the drain test; T-42-24 (a cancelled run reported as halted) by `PerRunCancelProbe` plus `Cancelled` and both cancel-route tests; T-42-25 and T-42-26 are accepted as planned (an in-memory flag read; the shutdown-instant race backed by the row).

## Self-Check: PASSED

- Files: all 14 modified files exist on disk, including the new tests in `cancel_tests.rs` and `stream_tests.rs`; the plan created no file.
- Commits: `b482701e` and `6869d5f5` are on `claude/laughing-dirac-e0h2ax`.
