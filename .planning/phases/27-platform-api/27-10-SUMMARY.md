---
phase: 27-platform-api
plan: 10
subsystem: api
tags: [sse, broadcast-bus, trace-sink, axum, tokio-broadcast, run-streaming]

requires:
  - phase: 27-platform-api (plan 01)
    provides: "RunApiState/run_router, RunRepositoryPort, the run_openapi_router openapi baseline test"
  - phase: 27-platform-api (plan 04)
    provides: "RunWorkerPool's run_once dispatch and map_outcome's RunOutcome match this plan's bus publishing hooks onto"
  - phase: 27-platform-api (plan 07)
    provides: "RunWorkerPool::with_engine_factory, the per-run engine construction site this plan attaches RunEventBusSink to"
provides:
  - "RunStreamEventKind/RunStreamMode/RunStreamEvent (paladin-core run.rs) -- the seven frozen wire event names, live/degraded mode, per-run seq and dropped count"
  - "RunEventStreamPort/RunEventStream/RunStreamError (paladin-ports, D-27) -- the core-typed input port paladin-web consumes"
  - "RunEventBus/RunEventBusSink/map_trace_event/RunEventStreamService (facade events.rs) -- the bounded drop-oldest per-run broadcast bus, its TraceSink adapter, and the live/degraded facade service"
  - "RunWorkerPool::with_event_bus -- bind/publish(parley|done|error)/unbind around dispatch, with a grace period before unbind for the fire-and-forget TraceDispatcher"
  - "GET /v1/runs/{run_id}/stream -- SSE route with 15s keep-alive, registered in run_openapi_router, regenerated openapi.json"
affects: [28-observability]

tech-stack:
  added: []
  patterns:
    - "Two-producer bus (D-24, D-25 correction): a TraceSink adapter bridges live progress events (superstep/node_started/node_finished/state_delta) from the engine; the worker publishes terminal/suspension events (parley/done/error) directly from the RunOutcome it already matches on in run_once -- Phase 28 collapses this into one function once the authoritative trace enum lands"
    - "Pre-bind-then-subscribe test pattern for a tokio::sync::broadcast producer under test: bind the bus and subscribe BEFORE spawning the producer task, since broadcast never buffers a publish for a subscriber that connects later -- the ordinary 'connect after it is already live' case a real SSE handler tolerates is not what a live-path test means to prove"
    - "A short, explicitly documented grace period before unbind, to give paladin-battalion's fire-and-forget TraceDispatcher (ENG-FR-21, no synchronization point exposed) a window to deliver the last superstep's trailing live trace event before the bus channel it would publish to disappears"

key-files:
  created:
    - crates/paladin-ports/src/input/run_event_stream_port.rs
    - src/application/services/run/events.rs
    - src/application/services/run/stream_tests.rs
  modified:
    - crates/paladin-core/src/platform/container/run.rs
    - crates/paladin-ports/src/input/mod.rs
    - src/application/services/run/worker.rs
    - src/application/services/run/mod.rs
    - crates/paladin-web/src/run_controller.rs
    - crates/paladin-web/openapi.json

key-decisions:
  - "TRACE_DRAIN_GRACE_PERIOD (100ms) inserted between the run's own repository/queue write and the bus's unbind call in run_once -- not in the plan's own action text, added after state_delta_carries_field_names_only demonstrated the race empirically (see Deviations)."
  - "The wire status for every RunOutcome::Halted case (cancel-requested or bare) is reported as a done event with status cancelled/halted respectively -- including the shutdown-drain case (OutcomeAction::LeaveRunningAndRequeue), where the run's OWN repository status stays Running. The plan's behavior text lists only two Halted sub-cases (\"Halted-as-cancelled/Halted\") mapping to done, with no third exception for shutdown-drain; this plan implements that literally. See Deviations/judgment call below."
  - "node_finished's outcome field is reported \"unknown\": TraceEvent::NodeFinished carries no success/failure signal today (only cache_hit), so guessing would be dishonest. Phase 28's authoritative trace enum is expected to carry the real outcome (D-25 correction)."
  - "state_delta's bytes field is the summed UTF-8 length of the changed field NAMES only (TraceEvent::DeltaMerged carries no value to leak) -- satisfies the no-values-on-the-wire prohibition (T-27-10-01) by construction rather than by redacting anything after the fact."

requirements-completed: [PLAT-03]

coverage:
  - id: D1
    description: "GET /v1/runs/{run_id}/stream is a text/event-stream response whose event: lines are exactly the seven frozen wire names, each documented with its payload schema in the OpenAPI operation"
    requirement: "PLAT-03"
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/run_controller.rs#run_controller::tests::run_stream_sse"
        status: pass
      - kind: unit
        ref: "python3/node openapi.json probe: all(k in description for k in [superstep,node_started,node_finished,state_delta,parley,done,error,degraded]) == True"
        status: pass
    human_judgment: false
  - id: D2
    description: "superstep/node_started/node_finished/state_delta bridge live from a TraceSink adapter feeding the per-run bus; parley/done/error are published by the worker from RunOutcome; unmapped TraceEvent variants are dropped"
    requirement: "PLAT-03"
    verification:
      - kind: unit
        ref: "src/application/services/run/events.rs#tests::map_trace_event_maps_exactly_four_of_eight_variants"
        status: pass
      - kind: integration
        ref: "src/application/services/run/stream_tests.rs#live_stream_yields_progress_then_done, live_stream_yields_parley_on_gate"
        status: pass
    human_judgment: false
  - id: D3
    description: "The bus never blocks the engine: a full per-run channel drops the oldest events and counts them, exposed as dropped on the next event"
    requirement: "PLAT-03"
    verification:
      - kind: unit
        ref: "src/application/services/run/events.rs#tests::bus_lagged_subscriber_reports_a_nonzero_skip_count, receiver_to_stream_folds_lag_into_the_next_events_dropped_field"
        status: pass
    human_judgment: false
  - id: D4
    description: "If the run executes elsewhere or is already terminal, the handler synthesizes events by polling WaypointPort::latest + RunRepositoryPort::get and always terminates with done/error, every event carrying mode: degraded"
    requirement: "PLAT-03"
    verification:
      - kind: integration
        ref: "src/application/services/run/stream_tests.rs#degraded_stream_terminates_with_done_for_terminal_run, degraded_stream_follows_remote_progress"
        status: pass
    human_judgment: false
  - id: D5
    description: "Heartbeat comment lines are emitted every 15s via Sse::keep_alive(KeepAlive::new().interval(15s)) on both paths"
    requirement: "PLAT-03"
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/run_controller.rs#run_controller::tests::run_stream_heartbeat_interval_is_15_seconds"
        status: pass
    human_judgment: false
  - id: D6
    description: "state_delta payloads never carry field values, vault-confined values or full Battlefield state -- only changed field names, superstep and byte-size counts; the run's input is never echoed"
    requirement: "PLAT-03"
    verification:
      - kind: integration
        ref: "src/application/services/run/stream_tests.rs#state_delta_carries_field_names_only"
        status: pass
    human_judgment: false
  - id: D7
    description: "paladin-web consumes RunEventStreamPort (a Stream of core RunStreamEvent) and never names the bus, the engine or TraceEvent"
    requirement: "PLAT-03"
    verification:
      - kind: other
        ref: "grep -v '^\\s*//' crates/paladin-web/src/run_controller.rs | grep -cE 'TraceEvent|RunEventBus|WarEngine' == 0; cargo tree -p paladin-web -i paladin-battalion reports no path"
        status: pass
    human_judgment: false

duration: ~45min
completed: 2026-09-08
status: complete
---

# Phase 27 Plan 10: Run Streaming (SSE) Summary

**`GET /v1/runs/{run_id}/stream` bridges live run progress over SSE through a per-run `tokio::sync::broadcast` bus fed by a `TraceSink` adapter and the worker's own `RunOutcome` match, with a documented degraded polling fallback that always ends in `done`/`error` and a 15s heartbeat on both paths.**

## Performance

- **Duration:** ~45 min
- **Started:** 2026-09-08T05:26:40Z (worktree base)
- **Completed:** 2026-09-08T06:11:29Z
- **Tasks:** 2 (both `type="auto" tdd="true"`)
- **Files modified:** 9 (3 created, 6 modified)

## Accomplishments

- `RunStreamEventKind`/`RunStreamMode`/`RunStreamEvent` (`paladin-core` `run.rs`) freeze the seven wire event names (`superstep`, `node_started`, `node_finished`, `state_delta`, `parley`, `done`, `error`) and carry `seq`/`at`/`mode`/`dropped`/`payload`.
- `RunEventStreamPort`/`RunEventStream`/`RunStreamError` (new `paladin-ports` module) give `paladin-web` a core-typed seam that names neither the bus, `WarEngine`, nor `TraceEvent` (D-27, ADR-0031) -- verified by source grep and `cargo tree -i paladin-battalion` reporting no path.
- `RunEventBus` is a bounded (capacity 64), drop-oldest, `bind`/`unbind`/`subscribe`/`publish` broadcast bus keyed by `RunId`, with a `ThreadId -> RunId` binding table so the `TraceSink` half of the two producers can look up which run a thread's live trace event belongs to.
- `RunEventBusSink` implements `TraceSink`, mapping exactly four of `TraceEvent`'s eight variants (`SuperstepStarted`, `NodeStarted`, `NodeFinished`, `DeltaMerged`) through `map_trace_event`, dropping the other four (`RunStarted`, `WaypointSaved`, `RunFinished`, `FallbackHop`) as documented no-ops (D-25 correction).
- `RunWorkerPool::with_event_bus` wires the bus into `run_once`: `bind`s before dispatch, attaches a fresh `RunEventBusSink` to the per-run engine when `with_engine_factory` is also wired (mirroring `with_cancellation_probing`'s own precedent), publishes `parley`/`done`/`error` straight from the `RunOutcome` it already matches on, then `unbind`s after a short grace period (see Deviations).
- `RunEventStreamService` implements `RunEventStreamPort`: subscribes to the live bus when the run is bound locally, else falls back to a `futures::stream::unfold`-driven degraded poll over `WaypointPort::latest` + `RunRepositoryPort::get`, coalescing superstep jumps, emitting `parley` once on the first observed `AwaitingInput`, and always ending in `done`/`error`.
- `GET /v1/runs/{run_id}/stream` (`run_controller.rs`) frames the stream as SSE (`event:` = `RunStreamEventKind::as_str`, `data:` = the whole event as JSON), returns `404`/`501` per the D-44 precedent, and sets `.keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))` on every response -- registered in `run_openapi_router`, documenting all seven event names, `mode`, `dropped` and the degraded-mode caveat verbatim in the OpenAPI operation description; `openapi.json` regenerated via `make openapi`.

## Task Commits

Each task was committed atomically:

1. **Task 1: `RunStreamEvent`, `RunEventStreamPort`, the bus + sink, and worker publishing** - `d69cbd67` (feat)
2. **Task 2: `GET /runs/{run_id}/stream` -- SSE framing, 15s keep-alive, OpenAPI payload schemas** - `6de79479` (feat)

**Plan metadata:** this file's own commit (docs: complete plan) -- committed alongside this SUMMARY per worktree execution mode.

_TDD note: both tasks carry `tdd="true"`. Per-task tests were written and passing before each commit; no separate RED-then-GREEN commit pair was produced (test + implementation landed together per task, consistent with every prior 27-platform-api plan's documented convention for this worktree)._

## Files Created/Modified

- `crates/paladin-core/src/platform/container/run.rs` -- `RunStreamEventKind` (7 variants, `#[non_exhaustive]`), `RunStreamMode`, `RunStreamEvent` (`#[non_exhaustive]` + `new`), plus 3 new unit tests.
- `crates/paladin-ports/src/input/run_event_stream_port.rs` -- `RunEventStreamPort`, `RunEventStream`, `RunStreamError`; 3 unit tests (object safety, exhaustive match, `Display`).
- `crates/paladin-ports/src/input/mod.rs` -- declares `run_event_stream_port`.
- `src/application/services/run/events.rs` -- `RunEventBus`, `RunEventBusSink`, `map_trace_event`, `RunEventStreamService`, `receiver_to_stream`, `degraded_stream`/`DegradedState`/`terminal_payload`; 6 unit tests.
- `src/application/services/run/worker.rs` -- `event_bus` field, `with_event_bus` builder, `TRACE_DRAIN_GRACE_PERIOD` constant, bind/publish/unbind wiring in `run_once` and `record_engine_failure`.
- `src/application/services/run/mod.rs` -- declares `events`/`stream_tests` modules, re-exports `RunEventBus`/`RunEventBusSink`/`RunEventStreamService`/`map_trace_event`.
- `src/application/services/run/stream_tests.rs` -- `live_stream_yields_progress_then_done`, `live_stream_yields_parley_on_gate`, `degraded_stream_terminates_with_done_for_terminal_run`, `degraded_stream_follows_remote_progress` (real on-disk SQLite), `state_delta_carries_field_names_only`.
- `crates/paladin-web/src/run_controller.rs` -- `RunApiState.run_events` + `with_run_events`, `RUN_STREAM_HEARTBEAT_SECS`, `frame_run_events`, `stream_run` handler, route registration; 4 new tests.
- `crates/paladin-web/openapi.json` -- regenerated, adds `/v1/runs/{run_id}/stream`.

## Decisions Made

See `key-decisions` in frontmatter. In prose:

1. **A 100ms `TRACE_DRAIN_GRACE_PERIOD` before `unbind`, added after empirical failure.** `paladin-battalion`'s `TraceDispatcher` (`engine/hooks.rs`) is deliberately fire-and-forget over a background-task-drained queue with no synchronization point exposed to a caller -- `WarEngine::start`/`resume` can return before the LAST superstep's `DeltaMerged` trace event has actually reached the sink. The worker's own terminal-outcome publish (`done`) is synchronous and could race ahead of that trailing live event, and since an unbound thread is a documented no-op for `publish`, the trailing event would be silently and permanently dropped. `state_delta_carries_field_names_only` caught this empirically (first run: 0/1 passed, "expected a state_delta event", confirmed via a temporary debug print showing `superstep -> node_started -> node_finished -> done` with no `state_delta` at all). The fix moves `unbind` to AFTER the run's own repository/queue write (so the run's real completion is never delayed) and inserts a short, explicitly documented sleep first -- a best-effort mitigation given `paladin-battalion` exposes no "wait for drain" seam within this task's file scope, not a hard guarantee.
2. **Every `RunOutcome::Halted` case (cancel-requested, shutdown-drain, or bare) publishes a `done` event, not just the two the plan's own prose names.** The plan's behavior text says "on Completed/Halted-as-cancelled/Halted publishes done" -- two explicit `Halted` sub-cases (cancelled vs. plain). It does not carve out a third exception for the shutdown-drain case (`OutcomeAction::LeaveRunningAndRequeue`, where the run's repository status actually stays `Running` for redelivery). This plan treats the wire event purely as "this instance is done dispatching it" (status `halted` unless `cancel_requested`), independent of whether the run's OWN status transitioned -- a later worker resumes it through a fresh `bind`. Flagged here since the plan's flagged-assumptions section calls out PLAT-03 as probe-less; a reviewer may want to confirm this reading.
3. **`node_finished`'s `outcome` field is `"unknown"`, not guessed.** `TraceEvent::NodeFinished` carries only `cache_hit`, no success/failure signal -- inventing one would misrepresent data the engine does not yet expose. Documented as a Phase 28 dependency (D-25 correction already anticipates this collapse).
4. **Pre-bind-then-subscribe test pattern.** `tokio::sync::broadcast` never buffers a publish for a subscriber connecting after it was sent; three live-path tests initially raced `bind` (called inside `run_once`) against a polling `subscribe` loop in the test task and intermittently missed early events. Fixed by having the test itself call `bus.bind` and `subscribe` BEFORE spawning `run_once` (idempotent re-bind inside the worker is a no-op on the already-existing channel) -- documented as the house pattern for any future test of this kind.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Trailing live trace event silently dropped by a premature `unbind`**
- **Found during:** Task 1, first `cargo test` pass of `state_delta_carries_field_names_only`
- **Issue:** `run_once` published the terminal `done` event and called `bus.unbind` synchronously, immediately after the engine call returned -- but `paladin-battalion`'s fire-and-forget `TraceDispatcher` had not necessarily delivered the last superstep's `DeltaMerged` (`state_delta`) event to the sink yet. The event arrived at `RunEventBusSink::on_event` after `unbind` had already removed the run's channel, so `run_id_for` returned `None` and the event was silently dropped (a documented no-op, not a crash) -- test failed with 0/1, "expected a state_delta event".
- **Fix:** Moved `bus.unbind` to run AFTER the `map_outcome` match's repository/queue write (so the run's real completion is never delayed), and added a 100ms `TRACE_DRAIN_GRACE_PERIOD` sleep immediately before it -- see key-decisions #1 for the full rationale.
- **Files modified:** `src/application/services/run/worker.rs`
- **Verification:** `cargo test -p paladin-ai --lib services::run::stream_tests` -- 5/5 passed (was 4/5 with the earlier subscribe-race issue also present, see below).
- **Committed in:** `d69cbd67` (Task 1 commit)

**2. [Rule 1 - Bug] `broadcast` subscribe-after-first-publish race in three live-path tests**
- **Found during:** Task 1, same `cargo test` pass (`live_stream_yields_progress_then_done` got 2/3+ superstep events; `live_stream_yields_parley_on_gate` and `state_delta_carries_field_names_only` timed out)
- **Issue:** The tests spawned `run_once` (which calls `bind` internally) and only THEN polled `bus.subscribe` in a loop -- a `tokio::sync::broadcast` channel never buffers a publish for a subscriber that connects later, so an early event published before the polling subscribe won the race was lost forever.
- **Fix:** Restructured all three tests to `bus.bind` and `bus.subscribe` directly, BEFORE spawning `run_once` (its own internal re-`bind` on the same thread/run is an idempotent no-op).
- **Files modified:** `src/application/services/run/stream_tests.rs`
- **Verification:** `cargo test -p paladin-ai --lib services::run::stream_tests` -- 5/5 passed.
- **Committed in:** `d69cbd67` (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 1 bug fixes, both discovered and resolved during the first `cargo test` pass for Task 1, before any commit)
**Impact on plan:** Both were necessary to reach a compiling, fully-passing state matching the plan's own acceptance criteria; neither changed the plan's architecture beyond the one documented `TRACE_DRAIN_GRACE_PERIOD` addition (a mitigation for a real async race in the underlying, out-of-scope `TraceDispatcher`, not a design change to this plan's own bus/port/route surface).

## Issues Encountered

None beyond the two auto-fixed deviations above -- both were caught and resolved during Task 1's own verification loop, before any commit. No engine-error / `EngineError` path (`record_engine_failure`) exercise was needed in this plan's own tests; that path publishes `error` and unbinds immediately (no grace period), since it fires for a structural graph failure with no pending live trace event to race against.

## Flagged Assumptions (carried from PLAN.md)

The plan's own `<flagged_assumptions>` block notes PLAT-03 came back `unclassified — review manually` from the phase's probe (no SPEC.md exists for phase 27), so the truths in this plan were derived by the planner rather than probe-verified. This plan's own contribution to the edges that flag names:
- **A stream opened after the run is already terminal** -- covered: `degraded_stream_terminates_with_done_for_terminal_run` proves the degraded path ends in `done` immediately for an already-`Completed` run.
- **A subscriber slower than the bus** -- covered: `bus_lagged_subscriber_reports_a_nonzero_skip_count` / `receiver_to_stream_folds_lag_into_the_next_events_dropped_field` prove drop-oldest + an exact `dropped` count.
- **A heartbeat with no events for longer than 15s** -- covered at the unit level only: `run_stream_heartbeat_interval_is_15_seconds` asserts the constant and that the `KeepAlive` builder call compiles; no test waits a real 15s (explicitly out of scope for CI per the plan's own acceptance criteria wording).
- **Not covered by this plan, not claimed to be:** a resume arriving while not `AwaitingInput` and a resume with zero responses are 27-08's own edges, not this plan's file scope.

## User Setup Required

None -- no external service configuration required. Every test in this plan runs against InMemory adapters or real on-disk SQLite temp files (Tier 1, D-51), with no Docker dependency.

## Next Phase Readiness

- `RunEventBus`/`RunEventBusSink`/`RunEventStreamService`/`RunWorkerPool::with_event_bus` are ready for whichever later plan wires the production `RunWorkerPool` (27-04's own SUMMARY names 27-17) to also opt into event streaming -- `with_event_bus` is purely additive, `None` by default, and does not disturb any existing call site.
- Phase 28 (observability, OBS-01/OBS-02) is the documented seam for collapsing `map_trace_event`'s two-producer split into one function once the authoritative trace enum carries `parley`/`error`/success-vs-failure information -- `map_trace_event`'s own docs and this SUMMARY's key-decisions #3 name exactly what changes.
- `TRACE_DRAIN_GRACE_PERIOD` is a best-effort mitigation, not a hard guarantee -- a future `paladin-battalion` API (e.g. a `WarEngine::drain_trace()` or similar synchronization point) would let a later plan remove the sleep entirely in favor of an exact wait. Documented here so it is not mistaken for a permanent design choice.
- No blockers. `cargo fmt --all --check`, `cargo clippy --workspace --all-targets [--all-features] -- -D warnings`, and `cargo check --workspace --all-targets --all-features` all pass clean on the final commit; `cargo doc -p paladin-ai -p paladin-ports -p paladin-web --no-deps` introduces no new warnings (all warnings shown are pre-existing, in files this plan did not touch); `git diff --stat Cargo.lock` is empty (no new dependency -- `futures`/`tokio` broadcast/`async-stream`-equivalent `futures::stream::unfold` were all already available).

## Self-Check: PASSED

**Files verified to exist:**
- FOUND: `crates/paladin-core/src/platform/container/run.rs`
- FOUND: `crates/paladin-ports/src/input/run_event_stream_port.rs`
- FOUND: `crates/paladin-ports/src/input/mod.rs`
- FOUND: `src/application/services/run/events.rs`
- FOUND: `src/application/services/run/worker.rs`
- FOUND: `src/application/services/run/mod.rs`
- FOUND: `src/application/services/run/stream_tests.rs`
- FOUND: `crates/paladin-web/src/run_controller.rs`
- FOUND: `crates/paladin-web/openapi.json`

**Commits verified to exist (git log --oneline):**
- FOUND: `d69cbd67` feat(27-10): add RunStreamEvent, RunEventStreamPort, the per-run bus and worker publishing
- FOUND: `6de79479` feat(27-10): add GET /runs/{run_id}/stream SSE route with 15s keep-alive

**Verification commands re-run and confirmed passing:**
- `cargo test -p paladin-ai --lib services::run::events` -> `test result: ok. 6 passed`
- `cargo test -p paladin-ai --lib services::run::stream_tests` -> `test result: ok. 5 passed`
- `cargo test -p paladin-ai --lib services::run` -> `test result: ok. 47 passed`
- `cargo test -p paladin-web --lib run_controller` -> `test result: ok. 14 passed`
- `cargo test -p paladin-web --lib openapi_matches_committed_baseline` -> `test result: ok. 1 passed`
- `grep -c 'fn map_trace_event' src/application/services/run/events.rs` -> `2` (the one function definition + its own test function name substring-matching `map_trace_event`)
- `grep -c 'impl TraceSink for RunEventBusSink' src/application/services/run/events.rs` -> `1`
- `grep -c 'bus.publish' src/application/services/run/worker.rs` -> `5` (parley, completed, halted/cancelled, error (outcome path), error (record_engine_failure path))
- `grep -c 'pub trait RunEventStreamPort' crates/paladin-ports/src/input/run_event_stream_port.rs` -> `1`
- `grep -c 'KeepAlive::new().interval' crates/paladin-web/src/run_controller.rs` -> `2` (handler + heartbeat unit test)
- `grep -c 'RUN_STREAM_HEARTBEAT_SECS' crates/paladin-web/src/run_controller.rs` -> `4` (const def + doc comment + handler use + heartbeat unit test)
- `grep -v '^\s*//' crates/paladin-web/src/run_controller.rs | grep -cE 'TraceEvent|RunEventBus|WarEngine'` -> `0`
- openapi.json description keyword probe (node -e) -> `true`
- `cargo tree -p paladin-web -i paladin-battalion` -> no path (package not found in that crate's dependency tree)
- `cargo fmt --all --check` -> clean
- `cargo clippy --workspace --all-targets -- -D warnings` -> clean
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` -> clean
- `cargo check --workspace --all-targets --all-features` -> exit 0
- `cargo doc -p paladin-ai -p paladin-ports -p paladin-web --no-deps` -> no new warnings (pre-existing warnings in `thread_controller.rs`, `paladin_execution_service.rs`, `parley/adapter.rs`, `config/agent_runtime.rs`, `presets/mod.rs` are unchanged by this plan)
- `git diff --stat Cargo.lock` -> empty (no new dependency)

---
*Phase: 27-platform-api*
*Completed: 2026-09-08*
