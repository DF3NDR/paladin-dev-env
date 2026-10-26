---
phase: 45-rustfs-swap-platform-observability-deviations
plan: 02
subsystem: platform-runs
tags: [run-worker, sse, webhooks, trace-dispatcher, agent-runs, windows-ledger, plat-08]

requires:
  - phase: 27-platform-api
    provides: RunEventBus / RunEventBusSink / map_trace_event (D-24) and the persisted webhook delivery queue (D-40)
  - phase: 28-observability
    provides: per-run TraceDispatcher and the RUN_TRACE_EMITTER task-local (D-03)
  - phase: 40-tenant-identity-run-read-scoping
    provides: run_agent's RunScope with the run id and ledger scope (D-15/D-16), preserved verbatim here
provides:
  - "Legacy Runnable::Agent runs stream live SSE (node_started, node_finished, then one done or error) through the one map_trace_event mapping"
  - "Agent runs persist gapless run_traces rows (RunStarted seq 1 .. RunFinished) and feed the OTel and herald sinks when wired"
  - "Agent runs enqueue completed/failed webhook deliveries through the shared webhook_delivery_for_outcome helper"
  - "Private persist_failure shared by record_engine_failure, so the graph Err path and a corrupt fork_from now enqueue their subscribed Failed delivery"
  - "WINDOWS.md row 31's closing condition recorded through the ledger tool (row 60, fixed)"
affects: [45-07, PLAT-09]

tech-stack:
  added: []
  patterns:
    - "Standalone per-run TraceDispatcher for a run with no engine (agent path): the caller emits RunStarted / NodeStarted / NodeFinished / RunFinished itself"
    - "Single terminal wire event: failure goes through RunFinished{Failed} (message: null), never a second direct bus publish"
    - "Captured-then-returned persistence result so a repository error never leaves the bus channel bound"

key-files:
  created: []
  modified:
    - src/application/services/run/worker.rs
    - src/application/services/run/worker_tests.rs
    - src/infrastructure/telemetry/herald_sink.rs
    - src/application/services/run/webhook/mod.rs
    - docs/src/api-reference/platform-api.md
    - CHANGELOG.md
    - .planning/WINDOWS.md

key-decisions:
  - "RunStarted has no wire mapping (D-24), so D-16's RunStarted-derived evidence is asserted on the persisted run_traces row (seq 1, graph_fingerprint agent); the first wire event is node_started"
  - "record_engine_failure keeps its direct bus error publish for its two graph-path callers; run_agent does not call it and routes its failure through RunFinished{Failed} then persist_failure, giving exactly one terminal event per agent run"
  - "The agent run's below-engine emitter is Some only when a sink was composed, so an untraced agent run keeps the zero-cost no-emitter path (D-10) while the dispatcher itself is always built"
  - "Row 31 stays waived (the ledger tool refuses a waived -> fixed transition, 40-06 precedent); closure is recorded as appended-then-fixed row 60"

patterns-established:
  - "agent_pool test harness (AgentOnlyResolver + event bus + optional delivery repository) for agent-path failure and webhook tests"

requirements-completed: [PLAT-08]

duration: 45min
completed: 2026-09-29
status: complete
---

# Phase 45 Plan 02: Agent-run SSE and webhook parity Summary

**A code-registered `Runnable::Agent` run now streams live `node_started`, `node_finished` and exactly one `done`/`error` through the same `TraceDispatcher` -> `RunEventBusSink` -> `map_trace_event` machinery a graph run uses, persists its trace, and enqueues `completed`/`failed` webhook deliveries through the shared `webhook_delivery_for_outcome` helper; `WINDOWS.md` row 31's closing condition is recorded.**

## Performance

- **Duration:** ~45 min
- **Completed:** 2026-09-29
- **Tasks:** 3 (1 tracer, 2 auto/tdd)
- **Files modified:** 7

## Accomplishments

- `run_agent` builds the graph path's per-run sink composition (`build_run_sink` + optional `HeraldTraceSink`, shared `compose_run_sink`) behind a standalone `TraceDispatcher`, binds the bus, wraps the unchanged `execute_scoped` call in `with_run_trace_scope`, emits `RunStarted { graph_fingerprint: "agent" }` / `NodeStarted` / `NodeFinished` / `RunFinished { total_supersteps: 0 }`, then waits `TRACE_DRAIN_GRACE_PERIOD` and unbinds on every return path (T-45-11).
- Failure path: `RunFinished{Failed}` produces the ONE `error` wire event (`message: null`); the private `persist_failure` (status write, outcome, ack/nack, then the `Failed` delivery on the ack path only) is shared with `record_engine_failure`, so the graph `Err` path and the corrupt `fork_from` case now enqueue their subscribed `Failed` delivery too.
- One `enqueue_webhook_delivery` helper serves `run_once`, the agent success arm and `persist_failure`; an enqueue error is logged (run id only) and never changes the run's status (P2).
- The WR-02 tripwire is inverted to `agent_kind_run_with_a_webhook_enqueues_a_delivery`; the `event_bus`/`webhook_deliveries` field docs, `run_agent` rustdoc, `with_*` builder docs, the `WebhookPayload` carve-out and the platform-api.md limitation are rewritten or deleted; CHANGELOG `### Fixed` entry added.

## Task Commits

1. **Task 1 (tracer): live streaming through the per-run trace dispatcher** - `f5c659b9` (feat)
2. **Task 2: single terminal event and shared Failed webhook** - `cda3fe12` (fix)
3. **Task 3: webhook parity, inverted test, docs, CHANGELOG, row 31 ledger closure** - `abb83579` (feat)

Plan metadata commit follows this summary.

## Tracer gate

Auto mode was active (`_auto_chain_active: true`), so the tracer's verify was re-run end to end after its commit: `agent_kind_run_streams_done_live` and the 43 worker tests plus the 23 `stream_tests`/`events` tests passed. Tracer verified end-to-end, expanding.

## Tests added

`agent_kind_run_streams_done_live`, `agent_kind_run_with_a_webhook_enqueues_a_delivery` (inverted), `agent_kind_run_emits_exactly_one_terminal_event`, `agent_kind_run_failure_enqueues_a_failed_delivery`, `failed_delivery_enqueue_error_never_changes_the_failed_status`, `graph_engine_failure_enqueues_failed_delivery`, `agent_model_label_names_the_model_or_none`, `herald_sink_summarises_an_agent_shaped_run`; fixture `AlwaysFailsPaladinPort`; harness `agent_pool`. `AlwaysSucceedsPaladinPort` now returns a non-zero `TokenUsage`.

## Ledger

- Appended row **60** (`deviation`, phase 45, `worker.rs`) stating row 31's closing condition is met and naming the inverted test plus the two sibling tests; then `windows fixed 60` (status `fixed`).
- Row **31 stays `waived`**: the ledger tool only allows `open -> fixed|waived`, and hand edits are forbidden (40-06 row 32 precedent). Open count is unchanged at 2.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `type_complexity` clippy error on the new `agent_pool` test helper**
- **Found during:** Task 2 verification
- **Fix:** added `#[allow(clippy::type_complexity)]` (the existing `build_traced_pool` precedent)
- **Files modified:** `src/application/services/run/worker_tests.rs`
- **Commit:** `cda3fe12`

**2. [Rule 1 - Bug] Docs edit initially truncated `WebhookPayload`'s struct definition**
- **Found during:** Task 3 (my slice anchor matched the prefix of `WebhookPayloadAssistant`); caught by reading the diff before committing
- **Fix:** restored the file from git and re-applied the edit with an exact anchor. Nothing wrong was committed.

**3. [Rule 2 - Missing critical documentation] Public builder docs claimed engine-factory-only behaviour**
- **Found during:** Task 3 (`with_event_bus`, `with_trace_config`, `with_run_trace_port`, `with_herald`, the `trace_config` field doc)
- **Fix:** each now notes that a legacy agent run applies the setting without an engine. Not in the plan's file list beyond `worker.rs`, which already carried them.

### Notes, not deviations

- The Task 1 RED phase was a compile failure (`agent_model_label` absent) rather than a runtime failure; `agent_kind_run_streams_done_live` was written before the implementation.
- `make api-surface` reported the surface unchanged (4044 items) with `nightly-2026-09-20`; `make security` (audit + deny) passed.
- Full workspace `cargo test` was not run (disk budget); `cargo test -p paladin-ai --lib` ran 1036 tests, 0 failed, and `cargo clippy -p paladin-ai --all-targets -- -D warnings`, `cargo fmt --check` and `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ai --no-deps` are clean. No Docker was needed.

## Known Stubs

None.

## Threat Flags

None. Agent runs now feed the OTel, herald and persisted trace sinks; those records carry ids, usage and cost only (T-45-09), and the webhook path is the existing SSRF-guarded one (`ssrf.rs`, `service.rs`, `client.rs` untouched, D-00h).

## Self-Check: PASSED

- `src/application/services/run/worker.rs`, `worker_tests.rs`, `herald_sink.rs`, `webhook/mod.rs`, `platform-api.md`, `CHANGELOG.md`, `.planning/WINDOWS.md`: modified and committed.
- Commits `f5c659b9`, `cda3fe12`, `abb83579` exist on `claude/laughing-dirac-e0h2ax`.
- `grep -rn enqueues_no_delivery src docs` prints nothing; the D-00g `from_attribution(run.submitted_by.as_ref())` count in non-test `worker.rs` is 2.
