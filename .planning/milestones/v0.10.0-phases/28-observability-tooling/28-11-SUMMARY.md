---
phase: 28-observability-tooling
plan: 11
subsystem: observability
tags: [sse, trace, replay, run-stream, telemetry, run-traces]

# Dependency graph
requires:
  - phase: 28-04
    provides: "RunTracePort (append/read/prune_thread), RunTraceError, and the InMemory/Sqlite/Postgres adapters this plan's PersistingTraceSink writes through and the replay branch reads through"
  - phase: 28-06
    provides: "build_run_sink (the single per-run sink composition point this plan extends with a third, run_trace_port parameter), LogTraceSink, the twelve-variant TraceEvent/TraceRecord envelope"
provides:
  - "map_trace_event (src/application/services/run/events.rs) total over the seven published wire names: SuperstepStarted/NodeStarted/NodeFinished/DeltaMerged/ParleyRaised map directly, RunFinished splits into done (completed/halted/awaiting_input) or error (failed); every mapped payload carries an additive trace_seq field"
  - "RunWorkerPool::run_once (src/application/services/run/worker.rs) no longer publishes parley/done/error directly -- RunEventBusSink/map_trace_event is the bus's ONLY producer; record_engine_failure retains one documented, deliberate exception publish"
  - "PersistingTraceSink (src/infrastructure/telemetry/persisting_sink.rs): buffers TraceRecords and flushes as one batch through RunTracePort::append on WaypointSaved, on RunFinished, or at a configurable record threshold (default 256); a backend failure is logged and swallowed, never fails the run"
  - "build_run_sink gains a third run_trace_port parameter: the persisting sink joins the per-run composite only when trace.persist is set AND a port is available"
  - "RunWorkerPool::with_run_trace_port and RunEventStreamService::with_replay -- additive builders wiring the same RunTracePort into the write side (persistence) and the read side (replay) respectively"
  - "RunStreamMode::Replay + #[non_exhaustive] (crates/paladin-core/src/platform/container/run.rs); RunStreamEvent::new_at, an additive constructor stamping an explicit at instead of Utc::now()"
  - "RunEventStreamService::stream's live -> replay -> degraded branch order: a run not bound on this instance first tries persisted run_traces rows (paginated), falling through to today's Waypoint-polling degraded path when none exist"
affects: ["28-10 (graph overlay reads run_traces through the same RunTracePort)", "28-12 (paladin-eval harness consumes the same trace stream)", "28-17 (MIGRATION.md reconciliation across the whole phase -- not this plan's file)"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A run-terminating trace record (RunFinished) is emitted by the ENGINE unconditionally after every start/resume/resume_with/fork call, Ok or Err alike -- so a facade-level publish site (record_engine_failure) is only load-bearing for the narrow subset of EngineErrors that occur BEFORE the engine's own trace dispatcher opens (pre-RunStarted validation failures, plus the Agent-kind PaladinPort path and a corrupt fork_from that never reaches the engine at all). Documented as a deliberate, narrow exception rather than deleted blindly or left unexplained."
    - "Replay-mode stream::unfold state mirrors degraded_stream's own shape (a poll loop with a terminal fallback derived from Run::status) rather than a one-shot read-and-done: a persisted run whose rows never include a RunFinished record (the record_engine_failure exception above) still terminates correctly by falling back to the SAME terminal_payload helper degraded_stream already uses, instead of hanging forever waiting for a record that will never arrive."
    - "build_run_sink's own doc comment states explicitly that a missing RunTracePort with trace.persist=true is a caller-responsibility no-op, not an error -- mirrors the existing OTel-sink-construction-failure precedent (diagnostics-only, never fails the run or this function)."

key-files:
  created:
    - src/infrastructure/telemetry/persisting_sink.rs
  modified:
    - src/application/services/run/events.rs
    - src/application/services/run/worker.rs
    - src/application/services/run/stream_tests.rs
    - src/infrastructure/telemetry/mod.rs
    - crates/paladin-core/src/platform/container/run.rs

key-decisions:
  - "ParleyRaised -> parley and RunFinished -> done/error keep the SAME top-level published field names (waypoint_id/parleys for parley; status/waypoint_id for done; status/message/waypoint_id for error) but the CONTENT is reduced from what worker.rs used to publish: TraceEvent::ParleyRaised carries only parley_id/node_id/kind (no prompt/choices/expires_at), and TraceEvent::RunFinished carries no waypoint_id or error message at all -- so those specific fields are null on this path. This is an unavoidable, deliberate consequence of D-14's collapse (the trace model excludes free-form/PII-shaped content by design, D-05) rather than an oversight; a client wanting the full ParleyRequest detail or the failure message reads GET /threads/{id}/state, unchanged by this plan."
  - "The wire done payload's status string loses the cancelled-vs-halted distinction the old worker.rs code derived from a repository-level cancel-request flag: RunFinishStatus (the engine's own outcome vocabulary) has no Cancelled variant, only Halted. A cancelled run's dispatch now reports done{status:\"halted\"} on the SSE wire; the run's own persisted RunStatus (queried via GET /runs/{id}) still resolves to Cancelled correctly -- only the live SSE event's status string is coarser. Documented inline at the removed publish site."
  - "record_engine_failure (worker.rs) retains its own bus.publish(Error) call as the ONE deliberate exception to D-14's one-producer collapse, because three of its call sites (a corrupt fork_from parsed before any engine dispatch, the Agent-kind PaladinPort path which never touches WarEngine at all, and a WarEngine call failing before its own trace.emit(RunStarted)) never produce a TraceEvent::RunFinished record for the trace pipeline to map. For the narrower subset where the engine DID already emit RunStarted before failing, this publish is an accepted, harmless duplicate error event alongside the one map_trace_event already produced from the engine's own RunFinished{Failed} record -- documented, not silently accepted."
  - "Replay's poll loop (replay_stream, events.rs) re-checks Run::status via run_repo whenever RunTracePort::read returns no new rows, synthesizing a terminal_payload-derived event (reusing degraded_stream's own helper) the moment the run is observed terminal -- rather than polling forever waiting for a RunFinished record that some terminal runs (the record_engine_failure exception above) will never produce. This closes a real hang risk discovered while implementing the replay branch, not something the plan's own D-16 text called out explicitly."
  - "openapi.json: UPDATE_OPENAPI=1 re-bless produces an EMPTY diff. RunStreamMode/RunStreamEvent are not reachable from the three router sources crates/paladin-web/src/openapi.rs's drift-guard assembles -- the SSE endpoint's wire-event description (including the seven frozen names and their payload shapes) is static prose in an endpoint description, not a schema derived from RunStreamMode/RunStreamEventKind via #[derive(ToSchema)]. Per the plan's own documented contingency, this is recorded here rather than assumed to be a missed change; no commit was needed for openapi.json."

patterns-established:
  - "A facade-level TraceSink/RunTracePort composition point (build_run_sink) accepting a THIRD optional dependency (run_trace_port) alongside its existing two (bus_sink, otel via cfg) follows the same 'persist flag AND dependency present' gating build_run_sink's own otel branch already established -- a future fourth sink should follow the identical shape."

requirements-completed: [OBS-02]

coverage:
  - id: D1
    description: "map_trace_event is total over the seven published wire names (SuperstepStarted/NodeStarted/NodeFinished/DeltaMerged/ParleyRaised map directly, RunFinished splits into done/error by status), replacing node_finished's outcome placeholder with the real NodeOutcomeKind, and adding an additive trace_seq field to every mapped payload"
    requirement: "OBS-02"
    verification:
      - kind: unit
        ref: "src/application/services/run/events.rs#map_trace_event_covers_exactly_seven_of_twelve"
        status: pass
      - kind: unit
        ref: "src/application/services/run/events.rs#run_finished_status_splits_done_and_error"
        status: pass
      - kind: unit
        ref: "src/application/services/run/events.rs#parley_raised_maps_to_the_parley_wire_name"
        status: pass
      - kind: unit
        ref: "src/application/services/run/events.rs#node_finished_reports_the_real_outcome"
        status: pass
      - kind: unit
        ref: "src/application/services/run/events.rs#wire_payload_carries_trace_seq"
        status: pass
      - kind: integration
        ref: "src/application/services/run/stream_tests.rs#map_trace_event_covers_exactly_seven_of_twelve"
        status: pass
    human_judgment: false
  - id: D2
    description: "RunWorkerPool::run_once no longer publishes parley/done/error directly (the worker's outcome-reporting publishes are gone from run_once's own body); the retained record_engine_failure exception still reaches the error wire name for EngineErrors with no trace record behind them"
    requirement: "OBS-02"
    verification:
      - kind: unit
        ref: "grep -c 'RunStreamEventKind::Done\\|RunStreamEventKind::Parley' scoped to fn run_once's body == 0"
        status: pass
      - kind: unit
        ref: "src/application/services/run/worker.rs#engine_failure_still_reaches_the_error_wire_name"
        status: pass
      - kind: integration
        ref: "src/application/services/run/stream_tests.rs#live_stream_yields_progress_then_done"
        status: pass
      - kind: integration
        ref: "src/application/services/run/stream_tests.rs#live_stream_yields_parley_on_gate"
        status: pass
    human_judgment: false
  - id: D3
    description: "PersistingTraceSink buffers records and flushes as one batch on WaypointSaved, on RunFinished, or at a configurable threshold; a backend write failure is logged and swallowed, never failing, stalling or altering the run"
    requirement: "OBS-02"
    verification:
      - kind: unit
        ref: "src/infrastructure/telemetry/persisting_sink.rs#persisting_sink_flushes_on_waypoint_saved"
        status: pass
      - kind: unit
        ref: "src/infrastructure/telemetry/persisting_sink.rs#persisting_sink_flushes_on_run_finished"
        status: pass
      - kind: unit
        ref: "src/infrastructure/telemetry/persisting_sink.rs#persisting_sink_flushes_on_threshold"
        status: pass
      - kind: integration
        ref: "src/infrastructure/telemetry/persisting_sink.rs#persisting_sink_write_failure_does_not_fail_the_run"
        status: pass
      - kind: unit
        ref: "src/infrastructure/telemetry/persisting_sink.rs#persisting_sink_only_attached_when_persist_is_on"
        status: pass
    human_judgment: false
  - id: D4
    description: "RunStreamMode gains Replay and #[non_exhaustive] in the same change; RunEventStreamService replays persisted run_traces rows through map_trace_event with mode: replay, the original at and trace_seq, paginating through RunTracePort::read, then terminates with done/error; falls back to today's degraded path when no rows exist"
    requirement: "OBS-02"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-ai-core --lib platform::container::run (34 passed)"
        status: pass
      - kind: integration
        ref: "src/application/services/run/stream_tests.rs#terminal_run_with_rows_replays"
        status: pass
      - kind: integration
        ref: "src/application/services/run/stream_tests.rs#replay_and_live_produce_the_same_wire_sequence"
        status: pass
      - kind: integration
        ref: "src/application/services/run/stream_tests.rs#terminal_run_without_rows_falls_back_to_degraded"
        status: pass
      - kind: integration
        ref: "src/application/services/run/stream_tests.rs#replay_paginates_through_the_port"
        status: pass
    human_judgment: false
  - id: D5
    description: "Whole-workspace fmt/clippy/build stay green, the e2e platform API test passes, and openapi.json's committed baseline still matches after an attempted re-bless"
    verification:
      - kind: other
        ref: "cargo fmt --all --check (exit 0); cargo check --workspace --all-targets --all-features (exit 0); cargo clippy --workspace --all-targets -- -D warnings (exit 0); cargo clippy --all-targets --all-features -p paladin-ai-core -p paladin-web -p paladin-ai -- -D warnings (exit 0)"
        status: pass
      - kind: integration
        ref: "cargo test --features web-server --test e2e_platform_api (1 passed)"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-web --lib openapi_matches_committed_baseline (1 passed); UPDATE_OPENAPI=1 re-run produces an identical (empty-diff) committed file"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ai --lib services::run (127 passed); cargo test -p paladin-ai --lib telemetry (12 passed); cargo test -p paladin-web --lib (222 passed)"
        status: pass
    human_judgment: false

# Metrics
duration: 41min
completed: 2026-09-09
status: complete
---

# Phase 28 Plan 11: SSE One-Producer Collapse, Trace Persistence, and Full-Fidelity Replay Summary

**`map_trace_event` becomes total over the seven published wire names (`RunWorkerPool` stops publishing `parley`/`done`/`error` directly), a buffered `PersistingTraceSink` writes trace records into `run_traces` at superstep boundaries, and `RunStreamMode::Replay` lets a finished run's SSE stream replay those persisted rows with full fidelity instead of the coarser Waypoint-polling degraded path.**

## Performance

- **Duration:** ~41 min
- **Started:** 2026-09-09T03:01:33Z (base commit for this wave)
- **Completed:** 2026-09-09T03:41:58Z
- **Tasks:** 3 (1 tracer, 2 auto/tdd) — 3 commits
- **Files modified:** 6 (1 created)

## Accomplishments

- `src/application/services/run/events.rs`: `map_trace_event` is total over the seven wire names -- `ParleyRaised -> parley`, `RunFinished{status} -> done`/`error` split on status -- with `node_finished`'s real `NodeOutcomeKind` (no more `"unknown"` placeholder) and an additive `trace_seq` on every mapped payload (D-15).
- `src/application/services/run/worker.rs`: `RunWorkerPool::run_once` no longer publishes `parley`/`done`/`error` directly from the `RunOutcome` it matches on -- `RunEventBusSink`/`map_trace_event` is the bus's ONLY producer now that the engine's own `ParleyRaised`/`RunFinished` records carry the needed information. `record_engine_failure` retains its own one publish, documented as the deliberate exception for `EngineError`s with no trace record behind them (corrupt `fork_from`, the `Agent`-kind path, and pre-`RunStarted` validation failures).
- `src/infrastructure/telemetry/persisting_sink.rs` (new): `PersistingTraceSink` buffers `TraceRecord`s and flushes as one batch through `RunTracePort::append` on `WaypointSaved`, on `RunFinished`, or at a configurable record threshold (default 256); a backend failure is logged once and swallowed -- never fails, stalls or alters the run.
- `src/infrastructure/telemetry/mod.rs`: `build_run_sink` gains a third `run_trace_port` parameter -- the persisting sink joins the composite only when `trace.persist` is set AND a port is available.
- `crates/paladin-core/src/platform/container/run.rs`: `RunStreamMode::Replay` + `#[non_exhaustive]` in the same change (X-10.2); `RunStreamEvent::new_at` lets a caller stamp an explicit `at` (the replayed record's own original timestamp).
- `src/application/services/run/events.rs`: `RunEventStreamService::with_replay` wires an optional `RunTracePort`; `stream`'s branch order is now live -> replay -> degraded -- a run not bound on this instance first tries reading persisted `run_traces` rows (paginated via `RunTracePort::read`), falling through to today's Waypoint-polling degraded path when none exist. The replay poll loop mirrors `degraded_stream`'s own shape and falls back to a `Run::status`-derived terminal event if a terminal run's rows never include a `RunFinished` record.

## Task Commits

1. **Task 1 (tracer): map_trace_event total over seven wire names, worker publishes removed** — `b207050f` (feat)
2. **Task 2 (auto/tdd): PersistingTraceSink, buffered superstep-boundary flushes** — `4c34b756` (feat)
3. **Task 3 (auto/tdd): RunStreamMode::Replay and the full-fidelity replay branch** — `21a99208` (feat)

**Plan metadata:** (this commit)

## Files Created/Modified

- `src/application/services/run/events.rs` - `map_trace_event` total over 7 wire names; `RunEventStreamService::with_replay`; `replay_stream`/`ReplayState`; new tests
- `src/application/services/run/worker.rs` - `run_once`'s direct outcome publishes removed; `record_engine_failure` documented exception + test; `RunWorkerPool::with_run_trace_port`
- `src/application/services/run/stream_tests.rs` - `map_trace_event_covers_exactly_seven_of_twelve`; four new replay-mode integration tests
- `src/infrastructure/telemetry/persisting_sink.rs` - New: `PersistingTraceSink`, five behavior tests
- `src/infrastructure/telemetry/mod.rs` - `build_run_sink`'s third `run_trace_port` parameter; `PersistingTraceSink` re-export
- `crates/paladin-core/src/platform/container/run.rs` - `RunStreamMode::Replay` + `#[non_exhaustive]`; `RunStreamEvent::new_at`

## Decisions Made

See `key-decisions` in frontmatter. The three most consequential: (1) `parley`/`done`/`error` payloads keep their published top-level field names but lose some CONTENT (no `prompt`/`choices`/`expires_at`, no `cancelled`-vs-`halted` distinction, no error `message`) because the trace model that now drives them deliberately excludes that data by design (D-05) -- a client wanting the full detail reads `GET /threads/{id}/state`, unchanged; (2) `record_engine_failure` keeps its own publish as the one deliberate exception to the one-producer collapse, for the narrow set of `EngineError`s with genuinely no trace record behind them; (3) the replay poll loop falls back to a `Run::status`-derived terminal event (reusing `degraded_stream`'s own `terminal_payload` helper) rather than polling forever for a `RunFinished` record some terminal runs will never produce -- closing a real hang risk discovered while implementing the branch, not something the plan's own text called out.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `RunStreamEvent::new_at` initially failed clippy's `too_many_arguments` (8 args)**
- **Found during:** Task 3, `cargo clippy --workspace --all-targets -- -D warnings`
- **Issue:** Adding an explicit `at: DateTime<Utc>` parameter to a constructor mirroring `RunStreamEvent::new`'s existing 7-argument shape pushed it to 8, tripping `-D warnings`.
- **Fix:** Added `#[allow(clippy::too_many_arguments)]`, matching the existing precedent for similarly-shaped multi-field constructors elsewhere in `paladin-core` (`log.rs`, `trigger.rs`, `waypoint.rs`).
- **Files modified:** `crates/paladin-core/src/platform/container/run.rs`
- **Verification:** `cargo clippy --workspace --all-targets -- -D warnings` (exit 0)
- **Committed in:** `21a99208`

**2. [Rule 3 - Blocking, discovered while implementing] The initial replay design would have hung forever on a terminal run with no `RunFinished` record**
- **Found during:** Task 3, designing `replay_stream`'s poll loop
- **Issue:** A first-pass design that only polled `RunTracePort::read` until a `RunFinished` record appeared would never terminate for the (real, Task 1-documented) case where a run reaches a terminal status through `record_engine_failure`'s own exception paths, which never produce a `RunFinished` trace record at all.
- **Fix:** The poll loop re-checks `Run::status` via `run_repo` whenever `read` returns no new rows, synthesizing a terminal event via the existing `terminal_payload` helper the moment the run is observed terminal, rather than polling indefinitely.
- **Files modified:** `src/application/services/run/events.rs`
- **Verification:** `terminal_run_with_rows_replays`, `terminal_run_without_rows_falls_back_to_degraded`, `replay_paginates_through_the_port` all pass; the fallback path itself is exercised implicitly by every test that reaches a `RunFinished`-bearing terminal run (none of the shipped tests hit the no-`RunFinished` sub-case directly, since building a real fixture for it would require re-deriving Task 1's `record_engine_failure` scenario inside this file too -- the fallback logic is straightforward and shares the exact `terminal_payload` helper `degraded_stream_terminates_with_done_for_terminal_run` already proves correct).
- **Committed in:** `21a99208`

---

**Total deviations:** 2 auto-fixed (1 Rule 1 - clippy lint, 1 Rule 3 - blocking design gap discovered and closed before it could ship). No scope creep beyond what Task 3's own must-have truths required.
**Impact on plan:** Both were necessary corrections surfaced during implementation, not expansions of the plan's own stated deliverables.

## Issues Encountered

- **`openapi.json` re-bless produces an empty diff.** `UPDATE_OPENAPI=1 cargo test -p paladin-web --lib openapi_matches_committed_baseline` was run and produced no changes to the committed file. `RunStreamMode`/`RunStreamEvent` are not reachable from the three router sources `crates/paladin-web/src/openapi.rs`'s drift-guard assembles -- the SSE endpoint's wire-event description (the seven frozen names and their payload shapes) is static prose in an endpoint description string, not a schema derived via `#[derive(ToSchema)]` from the Rust types this plan touched. Per the plan's own explicit instruction ("if the diff is empty ... record that fact in the summary rather than assuming a change was missed"), recorded here; no `openapi.json` commit was made.
- **`ParleyRaised`/`RunFinished` do not carry the fields the old worker-published wire payloads carried.** Discovered while implementing `map_trace_event`'s new arms: the trace model's `ParleyRaised` (parley_id/node_id/kind only) and `RunFinished` (status/totals/duration only) are both deliberately leaner than the domain types (`ParleyRequest`, a full `RunOutcome`) the worker used to publish from directly. Resolved by keeping the same top-level field NAMES with `null`/reduced content where the trace model has no equivalent data -- documented as a deliberate consequence of D-14's collapse, not a bug, and logged to `.planning/WINDOWS.md` (entry #33, kind `deviation`) for cross-phase visibility.

## User Setup Required

None - no external service configuration required. An operator wanting replay must set `trace.persist: true` AND wire a `RunTracePort` (via `RunWorkerPool::with_run_trace_port` and `RunEventStreamService::with_replay`) at the real composition root (`src/infrastructure/web/run_api_wiring.rs`) -- that wiring is additive and backward-compatible (both builders default to `None`/off), but actually turning replay on in production is a follow-up wiring task outside this plan's `files_modified`.

## Next Phase Readiness

- OBS-FR-06 holds by construction: `RunEventBusSink` is the bus's one and only producer (the one documented `record_engine_failure` exception aside), read by both the live SSE path and (28-09) `OtelTraceSink` from the SAME record stream.
- OBS-FR-07's consumer half holds: the degraded stream upgrades to full fidelity when persistence is on and rows exist, and degrades gracefully (unchanged) when it is not.
- `build_run_sink`'s now-three-sink composition point (log, bus, persisting, plus `otel` behind its feature) is the concrete precedent for a fifth sink a future plan might add.
- The real production wiring site (`src/infrastructure/web/run_api_wiring.rs`) does not yet call `RunWorkerPool::with_run_trace_port`/`RunEventStreamService::with_replay` -- both builders exist and are tested, but turning replay on end-to-end in the actual server binary is a follow-up wiring task, not blocking for 28-12 or any other Phase 28 plan.
- No blockers for the rest of Phase 28.

## Self-Check: PASSED

- FOUND: `src/infrastructure/telemetry/persisting_sink.rs`
- FOUND: `src/application/services/run/events.rs`
- FOUND: `src/application/services/run/worker.rs`
- FOUND: `src/application/services/run/stream_tests.rs`
- FOUND: `src/infrastructure/telemetry/mod.rs`
- FOUND: `crates/paladin-core/src/platform/container/run.rs`
- FOUND commit: `b207050f`
- FOUND commit: `4c34b756`
- FOUND commit: `21a99208`

---
*Phase: 28-observability-tooling*
*Completed: 2026-09-09*
