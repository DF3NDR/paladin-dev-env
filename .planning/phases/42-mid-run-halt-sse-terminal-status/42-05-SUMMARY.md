---
phase: 42-mid-run-halt-sse-terminal-status
plan: 05
subsystem: run-streaming
tags: [allowance, mid-run-halt, sse, trace, run-finished, halt-reason, replay, tests, docs]

requires:
  - phase: 42-mid-run-halt-sse-terminal-status
    provides: HaltReason with the one wire_json builder, HaltCause on RunOutcome::Halted, SpendGuard and TreasurerSpendGuard (plan 42-02); runs.halt_reason written before the status flips, GET /runs halt_reason and final_waypoint_id (plan 42-03); boundary hardening and resume by fork (plan 42-04); ADR-0057
provides:
  - TraceEvent::RunFinished.halt_reason, filled by the engine from HaltCause::Spend at every RunFinished emit site
  - run_finish_status returning (RunFinishStatus, Option<HaltReason>)
  - SSE done.halt_reason on the live (map_trace_event), degraded (terminal_payload) and replay (replay_stream) paths, all through HaltReason::wire_json
  - a row-trusting replay path that never ends on a non-terminal run row
  - run_trace contract clauses for the halt-reason round trip and the legacy row
  - stream tests proving the three paths agree, plus MIGRATION 9.2/9.6, platform-api.md and CHANGELOG entries
affects: [42-06, 42-09, 42-10, 42-12]

tech-stack:
  added: []
  patterns:
    - "Struct-variant field addition with serde default and skip_serializing_if (the cost precedent), reason nested under its own key so its tag never collides with the record's kind tag"
    - "Replay trusts the run row: a terminal row supplies status and halt_reason, a non-terminal row skips the persisted RunFinished"
    - "Stream agreement rig: one worker-pool run driven to a halt, then the live, degraded and replay terminal events read from the same persisted artefacts and compared on status and halt_reason"

key-files:
  created: []
  modified:
    - crates/paladin-core/src/platform/container/trace.rs
    - crates/paladin-core/src/platform/container/herald.rs
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/src/engine/hooks.rs
    - crates/paladin-ports/src/output/trace_sink_port.rs
    - crates/paladin-eval/src/assertion.rs
    - crates/paladin-eval/tests/assertion_snapshots.rs
    - crates/paladin-storage/src/run_trace/contract_tests.rs
    - crates/paladin-storage/src/run_trace/in_memory.rs
    - crates/paladin-storage/src/run_trace/sqlite.rs
    - crates/paladin-storage/src/run_trace/postgres.rs
    - src/infrastructure/telemetry/herald_sink.rs
    - src/infrastructure/telemetry/otel_sink.rs
    - src/infrastructure/telemetry/persisting_sink.rs
    - src/application/services/run/worker.rs
    - src/application/services/run/events.rs
    - src/application/services/run/stream_tests.rs
    - tests/cli/eval_run_test.rs
    - tests/integration/otel_transport_test.rs
    - docs/src/api-reference/platform-api.md
    - MIGRATION.md
    - CHANGELOG.md
    - .project/current-exports.txt

key-decisions:
  - "The reason is named at the source: the engine's own RunFinished carries halt_reason, so the SSE bridge, the persisted trace and the replay path read one value instead of re-deriving it"
  - "A replayed RunFinished that itself says awaiting_input still ends the replay when the row is not terminal: AwaitingInput is never a terminal row status, and skipping it would leave an awaiting run polling forever"
  - "The legacy run_trace clause parses a literal pre-phase JSON line with serde_json and stores it through the port, the same call every SQL adapter applies to its stored record text on read, instead of adding a raw-JSON writer capability to three adapters"
  - "The OTel exporter's attribute set is unchanged: no figures or key values become span attributes (T-42-22)"

patterns-established:
  - "HaltedRun rig in stream_tests.rs: drive_halted_run builds a real pool (live bus, persisted trace pipeline, run row) for a Treasurer spend halt or a stub-guard halt, and degraded_terminal/replay_terminal read the same run back"

requirements-completed: [PLAT-09, ALLOW-03]

coverage:
  - id: D1
    description: "TraceEvent::RunFinished carries halt_reason: Option<HaltReason>; the engine fills it from HaltCause::Spend at every emit site and leaves it None for a cancel halt, a token halt and every non-halt outcome"
    requirement: "PLAT-09"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#spend_guard_tests::run_finish_status_names_a_reason_only_for_a_spend_halt, #spend_halted_run_finished_carries_the_guards_reason, #non_spend_run_finished_carries_no_halt_reason (cargo test -p paladin-battalion --lib spend_guard: 7 passed)"
        status: pass
    human_judgment: false
  - id: D2
    description: "A pre-phase run_finished JSON object with no halt_reason key reads back with None, a reason-less event serializes byte-identically (no key), and a record carrying either reason round-trips byte-identically with the reason nested under its own key and the record's kind unchanged; every run_trace adapter holds the same two clauses"
    requirement: "PLAT-09"
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/trace.rs#tests::legacy_run_finished_json_reads_back_with_no_halt_reason, #trace_record_round_trips_a_halt_reason, #run_finished_without_halt_reason_serializes_byte_identically (cargo test -p paladin-ai-core --lib trace: 15 passed)"
        status: pass
      - kind: integration
        ref: "crates/paladin-storage/src/run_trace/contract_tests.rs#run_finished_halt_reason_round_trips, #legacy_run_finished_row_reads_back_without_a_halt_reason (in-memory and SQLite legs: 27 passed; the PostgreSQL leg compiles under --all-features and self-skips here, CI only)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The live done of a spend-halted run carries halt_reason equal to HaltReason::wire_json, a halt with no reason keeps today's payload byte-identical, and a ledger-unavailable halt is a done with {reason: ledger_unavailable}, never an error"
    requirement: "PLAT-09"
    verification:
      - kind: unit
        ref: "src/application/services/run/events.rs#tests::map_trace_event_renders_the_halt_reason_on_done, #live_and_degraded_builders_agree_on_status_and_reason"
        status: pass
    human_judgment: false
  - id: D4
    description: "The degraded polling done for a halted run carries the row's halt_reason through wire_json; without a reason it is today's object"
    requirement: "PLAT-09"
    verification:
      - kind: unit
        ref: "src/application/services/run/events.rs#tests::terminal_payload_renders_the_row_reason"
        status: pass
    human_judgment: false
  - id: D5
    description: "Replay trusts the run row: a terminal row supplies status and halt_reason (inserting or removing the key) while the record's usage and trace_seq survive, so a pre-phase cancelled run's stored halted record replays as cancelled; a non-terminal row skips the persisted RunFinished, replay keeps reading, and the stream ends on the row's terminal event once the row turns terminal"
    requirement: "PLAT-09"
    verification:
      - kind: unit
        ref: "src/application/services/run/stream_tests.rs#replay_trusts_the_row_over_the_recorded_status, #replay_skips_a_run_finished_while_the_row_is_not_terminal (a mutation that disables the row consultation turns both red)"
        status: pass
    human_judgment: false
  - id: D6
    description: "For one spend-halted run (real Treasurer over a real SQLite ledger, real worker pool) the status and halt_reason of the done are byte-identical on the live, degraded and replay paths and equal the run row's own wire_json; a ledger-unavailable halt is a done on all three; two successive degraded reads and two successive replays of the same terminal run are byte-identical"
    requirement: "PLAT-09"
    verification:
      - kind: integration
        ref: "src/application/services/run/stream_tests.rs#halted_done_agrees_on_live_degraded_and_replay, #ledger_unavailable_done_is_done_not_error_on_live_and_degraded, #repeated_degraded_and_replay_reads_of_a_halted_run_are_identical (cargo test -p paladin-ai --lib application::services::run: 227 passed, including replay_and_live_produce_the_same_wire_sequence and engine_spend_halt_tracer)"
        status: pass
    human_judgment: false
  - id: D7
    description: "The trace field and the done payload change are on the register, documented and baselined"
    requirement: "PLAT-09"
    verification:
      - kind: other
        ref: "./scripts/check-migration-allowlist.sh exit 0; MIGRATION.md 9.2 RunFinished row and 9.6 SSE entry (ledger_unavailable matched twice); platform-api.md halt_reason (16 lines); PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface unchanged (4195 items)"
        status: pass
    human_judgment: false

duration: ~40min
completed: 2026-10-06
status: complete
---

# Phase 42 Plan 05: SSE terminal status names the halt reason Summary

**A spend-halted run's SSE `done` now carries the Treasurer reason (`allowance_exhausted` with its figures, or `ledger_unavailable`) through the one `wire_json` builder, identical on the live, degraded and replay paths, with the engine's own `RunFinished` naming the reason at the source and stored traces still readable.**

## Performance

- **Duration:** ~40 min (three task commits plus reading, gates and this summary)
- **Started:** 2026-10-06T19:36Z
- **Completed:** 2026-10-06T20:14Z (code); state updates follow
- **Tasks:** 3 (all auto, two TDD)
- **Files modified:** 23, none created

## Accomplishments

- **Reason at the source (Task 1).** `TraceEvent::RunFinished` gains `halt_reason: Option<HaltReason>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`. `run_finish_status` now returns `(RunFinishStatus, Option<HaltReason>)`: a `Halted` run whose cause is `Spend(reason)` yields `Some(reason)`, a cancel or token halt and every other outcome yield `None`. The four emit sites that call it destructure the pair, and the fifth (the already-completed thread) sets `None`. `TraceRecord`'s hand-written serde needed no change: the nested object round-trips byte-identically with the record's `kind` untouched. `RunFinishStatus` itself is unchanged (42-06 adds `Cancelled`).
- **Compiler-forced sites.** `halt_reason: None` was added to every other struct-literal construction (hooks, herald doctest and test, `trace_sink_port`, `paladin-eval` assertion and snapshot tests, the three telemetry sinks, the worker's two `run_agent` emits, `events.rs` and `stream_tests.rs` literals, and the two integration tests). `otel_sink.rs` gained the field in its literals only; its exported attribute set is unchanged.
- **Stored-trace compatibility.** `legacy_run_finished_json_reads_back_with_no_halt_reason`, `trace_record_round_trips_a_halt_reason` and `run_finished_without_halt_reason_serializes_byte_identically` in `trace.rs`, and two new `run_trace` contract clauses registered in `run_all` and in the in-memory, SQLite and PostgreSQL adapters' test modules.
- **Live, degraded and replay `done` (Task 2).** `map_trace_event` inserts `"halt_reason": reason.wire_json()` into the `done` object when the event carries one and builds exactly today's object otherwise (`error` never carries it). `terminal_payload`'s `Halted` arm does the same from the run row. `replay_stream` now consults the row for a mapped `RunFinished` through `replay_terminal_override`: a terminal row supplies `status` (and the `done`/`error` kind, chosen as `terminal_payload` chooses it) and `halt_reason` while the record's `usage`, `trace_seq` and `waypoint_id` are kept; a non-terminal row skips the record so a drained run's persisted `RunFinished` never ends a replay; an unreadable or absent row keeps today's mapped payload.
- **Agreement tests.** A `HaltedRun` rig in `stream_tests.rs` drives one real worker-pool run to a halt (a real `Treasurer` over a real SQLite ledger with a `SpendStep` chain for the exhausted case; a stub `SpendGuard` for `ledger_unavailable`), captures the live terminal event, polls the row until terminal (G14), then reads the degraded and replay terminal events and compares `status` and `halt_reason` byte for byte, including a second read of each path.
- **Registers and docs (Task 3).** MIGRATION.md 9.2 row for `TraceEvent::RunFinished.halt_reason` (no `paladin-battalion` row: `run_finish_status` is private), the 9.6 SSE entry, the platform-api.md `done` table row and the three-path agreement paragraph, and a CHANGELOG extension of the Phase 42 entry.

## Task Commits

1. **Task 1: RunFinished carries the Treasurer halt reason from the engine** - `02a9a3dc` (feat)
2. **Task 2: the SSE done names the halt reason on live, degraded and replay paths** - `190c1154` (feat)
3. **Task 3: register and document the done change and the trace field** - `955de4a9` (docs)

## Gates and tests run

- `cargo test -p paladin-ai-core --lib trace`: 15 passed. `cargo test -p paladin-ai-core --doc from_run_finished`: 1 passed (the doctest literal gained the field). `cargo test -p paladin-battalion --lib spend_guard`: 7 passed. `cargo test -p paladin-storage --features sqlite --lib run_trace::`: 27 passed (in-memory and SQLite legs of both new clauses; the PostgreSQL legs compile under `--all-features` and self-skip because no PostgreSQL server runs here, so they did not execute). `cargo test -p paladin-eval`: all suites green (56, 12, 1 and 4 passed, 2 ignored as before).
- `cargo test -p paladin-ai --lib application::services::run::events`: 16 passed. `cargo test -p paladin-ai --lib application::services::run::stream_tests`: 15 passed, including `replay_and_live_produce_the_same_wire_sequence` and `terminal_run_with_rows_replays`. `cargo test -p paladin-ai --lib application::services::run`: 227 passed, including `engine_spend_halt_tracer`.
- Mutation check: with the row consultation in `replay_terminal_override` temporarily short-circuited, exactly the two replay tests (`replay_skips_a_run_finished_while_the_row_is_not_terminal`, `replay_trusts_the_row_over_the_recorded_status`) turned red; the change was reverted before the commit.
- `cargo check --workspace --all-targets --all-features` exit 0 (this includes `tests/cli/eval_run_test.rs` and `tests/integration/otel_transport_test.rs`). `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean after Task 1; `cargo clippy -p paladin-ai --all-targets --all-features -- -D warnings` clean after Task 2. `cargo fmt --check` clean.
- Orchestrator gate: `cargo build --workspace --all-features` exit 0, then `cargo test --workspace --lib --bins --no-fail-fast`: every suite green except `paladin-ai --lib`, 1224 passed and 2 not passing, the known sandbox-only pair `infrastructure::web::run_api_wiring::tests::build_run_api_persists_no_run_traces_by_default` and `build_run_api_persists_run_traces_when_trace_persist_is_set` (no outbound network; already in this phase's `deferred-items.md`, not touched by this plan).
- `./scripts/check-migration-allowlist.sh` exit 0; `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface-update` then `make api-surface`: surface unchanged at 4195 items (only the baseline's generated-at timestamp line moved, committed with the registers).
- Acceptance greps: `wire_json` occurs in the `map_trace_event` body (4 lines) and the `terminal_payload` body (3 lines); the two named stream tests and the two named contract clauses exist.
- Not run here: the PostgreSQL contract legs and the Redis integration tests (no servers in this sandbox).
- Manual credential-handling review: the `done` object is built only from `HaltReason::wire_json` (the halted ceiling's own figures, no tenant id and no key name or value); no new log line was added; the stream routes stay behind `load_visible_run`; the OTel exporter exports no new attribute.

## Decisions Made

- The reason travels on the engine's own terminal trace event, so the live bridge, the persisted record and the replay path agree without a second derivation (D-05, D-19).
- A replayed `RunFinished` whose own status is `awaiting_input` is not skipped when the row is not terminal: `RunStatus::AwaitingInput` is never terminal, so the plan's literal rule would turn every replay of a suspended run into an endless poll. Spend halts, drained halts and completed runs follow the plan's rule exactly.
- The legacy-row clause goes through the port as a parsed literal rather than a raw-JSON writer on each adapter; it exercises the same `serde_json` read every SQL adapter performs on its stored text, and avoids new test-only write capabilities on three adapters (one of which cannot run here).
- The two enumeration tests in `events.rs` keep their counts; new rows would have changed 13/7/6 for no benefit, so the halt-reason cases are separate tests.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Replay skip rule would hang a suspended run**
- **Found during:** Task 2
- **Issue:** The plan's replay rule skips a mapped `RunFinished` whenever the run row is not terminal. A run that finished with `awaiting_input` has a row of `AwaitingInput`, which is never terminal, so its replay would skip the record and poll forever where today it ends with `done: awaiting_input`.
- **Fix:** `replay_terminal_override` keeps today's mapped payload and ends the stream for a record whose own status is `awaiting_input` while the row is not terminal; every other non-terminal case skips as specified. Documented in the helper's rustdoc.
- **Files modified:** `src/application/services/run/events.rs`
- **Commit:** `190c1154`

### Plan wording interpreted

- **Commit trailers.** The orchestrator's note names `Co-Authored-By: Claude Fable 5.1`; the harness attribution instruction in this session names `Claude Sonnet 5.5` and replaces earlier attribution guidance, so the three task commits carry the Sonnet 5.5 trailer with the same `Claude-Session` line.
- **Files outside the plan's list.** `events.rs` and `stream_tests.rs` also gained compiler-forced `halt_reason: None` literals in the Task 1 commit (they construct `RunFinished`), and the three `run_trace` adapter test modules gained clause registrations; the plan's scope note anticipated the former, and registration in each adapter is what Task 1 step 4 asks for.

**Total deviations:** 1 auto-fixed, 2 wording interpretations; none change a decision or the public surface.

## Auth Gates

None.

## Issues Encountered

- Pre-existing and out of scope: the two `run_api_wiring` tests named above do not pass in this sandbox (no outbound network). They are the only non-passing cases in the orchestrator's gate.
- Disk stayed above 7 GB; no cache deletion was needed and every cargo command ran with `CARGO_INCREMENTAL=0`.

## Known Stubs

None.

## Threat Flags

None. No new endpoint, auth path or file-access pattern; the plan's T-42-19 (replay skips a non-terminal row, status and reason from the row), T-42-20 (stream routes already gated, figures of the halted ceiling only), T-42-21 (serde default plus the legacy-row tests and contract clauses) and T-42-22 (no new span attributes) mitigations are each in place and covered by a named test or the unchanged exporter.

## Self-Check: PASSED

- Files: the SUMMARY target and all 23 modified files exist on disk; no file was created by the plan.
- Commits: `02a9a3dc`, `190c1154` and `955de4a9` are on `claude/laughing-dirac-e0h2ax`.
