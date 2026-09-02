---
phase: 22-battlefield-state-superstep-engine
plan: 09
subsystem: infra
tags: [rust, superstep-engine, observability, fault-tolerance-seam, cancellation, tdd]

# Dependency graph
requires:
  - phase: 22-08
    provides: "Complete NodeSpec::Paladin execution, full WarEngine::resume, and program acceptance scenario E2E-1 green over SqliteWaypointStore"
provides:
  - "paladin_ports::output::trace_sink_port::{TraceEvent, TraceSink, TraceSinkError} -- the seven ENG-FR-21 typed events and the object-safe async trait Doc 07 (paladin-eval, OTel export) will implement"
  - "paladin_battalion::engine::hooks::TraceDispatcher -- a bounded, drop-oldest fire-and-forget dispatcher wired into WarEngine::with_trace_sink; the superstep loop emits RunStarted/SuperstepStarted/NodeStarted/NodeFinished/DeltaMerged/WaypointSaved/RunFinished at their natural points"
  - "paladin_battalion::engine::hooks::{NodeInterceptor, InterceptDecision} -- an ordered, empty-by-default chain wrapping each vanguard node's dispatch via WarEngine::with_interceptors, proven identical to no chain when empty"
  - "WarEngine::with_cancellation_token -- a tokio_util::sync::CancellationToken observed at superstep boundaries, producing RunOutcome::Halted with a resumable Waypoint"
affects: [22-10, 22-11]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "TraceDispatcher's emit() never awaits the sink or channel backpressure: a std::sync::Mutex<VecDeque<TraceEvent>> holds buffered events (drop-oldest on overflow, counted in an AtomicU64), and a tokio::sync::mpsc::channel::<()>(1) 'doorbell' (not the event channel itself) wakes a single background consumer task that is the ONLY caller of TraceSink::on_event -- a permanently blocking or always-erroring sink can only ever stall itself, never the run or a subsequent emit()"
    - "The doorbell channel closing (via ordinary Rust drop glue when TraceDispatcher's Sender goes out of scope) is what stops the background consumer task -- no hand-rolled shutdown flag was needed once the design settled, simplifying an earlier draft that carried a redundant AtomicBool 'closed' field"
    - "NodeRunOutcome (Succeeded/Skipped/Failed) replaces the plain Result<StateDelta, NodeError> execute_vanguard_node alone produced, so a NodeInterceptor::before Skip decision has its own outcome path distinct from success or failure, and reaches NodeOutcomeKind::Skipped in the persisted Waypoint rather than being folded into one of the other two"
    - "Cancellation is checked at exactly one point: the top of superstep::run's loop, using vanguard (the set about to execute) verbatim as the Halted Waypoint's vanguard -- this single checkpoint naturally covers both 'cancelled before the first superstep' and 'cancelled during a superstep' (the in-flight superstep's own top-of-loop check already passed, so it always finishes and merges before the NEXT iteration's check trips)"
    - "engine::test_support gained four TraceSink test doubles (RecordingTraceSink, BlockingTraceSink, AlwaysErroringTraceSink, GatedTraceSink) alongside the existing WaypointPort/PaladinPort doubles, reused by both hooks.rs's dispatcher-level tests and mod.rs's WarEngine-level end-to-end tests"
    - "Cancellation tests trigger token.cancel() (a synchronous method) from directly inside a CountingFunctionNode's own execution closure rather than from a background polling task racing an in-memory chain that completes in well under a millisecond -- the initial background-poller design was flaky and caught by a failing assertion on the first real run"

key-files:
  created:
    - crates/paladin-ports/src/output/trace_sink_port.rs
    - crates/paladin-battalion/src/engine/hooks.rs
  modified:
    - crates/paladin-ports/src/output/mod.rs
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/src/engine/superstep.rs
    - crates/paladin-battalion/src/engine/test_support.rs

key-decisions:
  - "TraceSink::on_event returns Result<(), TraceSinkError> so an implementation can report its OWN diagnostics, but TraceDispatcher discards the return value unconditionally -- the trait's error type exists for the sink's benefit, never as a signal anything else inspects"
  - "DeltaMerged carries Vec<FieldName> (changed field names) rather than values, satisfying T-22-32 structurally: attaching an exporter cannot, by construction, export shared Battlefield state through this event"
  - "RunStarted/RunFinished bracket WarEngine::start/resume_with_options at the WarEngine level, not inside superstep::run -- 'a run' is a caller-level concept the shared loop (used identically by fresh starts and resumes) has no opinion about. A resume that short-circuits on an already-Completed waypoint still emits both events, for a consistent bracket regardless of path"
  - "Removed a redundant AtomicBool 'closed' shutdown flag from TraceDispatcher during Task 1 once cargo test proved the doorbell channel's own closing (ordinary drop glue) already achieves clean consumer-task shutdown -- kept in scope as it directly affects Task 3's acceptance criterion (see Deviations)"
  - "Interceptor's before/after chain runs fully inside each vanguard node's own spawned task (not hoisted out to a superstep-wide pre-pass), since ENG-FR-22's ordering guarantee is per-node, not cross-node -- this keeps the interceptor-wrapped and un-wrapped code paths structurally identical apart from the chain itself"
  - "Doc 04's Aegis fault-tolerance policy is documented at the NodeInterceptor trait definition as wrapping OUTSIDE this chain, per the plan's explicit instruction, so a later epic does not nest retry/timeout logic inside interceptor decisions"

requirements-completed: [ENG-07]

coverage:
  - id: D1
    description: "The engine accepts an optional TraceSink and emits the seven typed events (RunStarted, SuperstepStarted, NodeStarted, NodeFinished, DeltaMerged with field changes, WaypointSaved, RunFinished) in order for a two-superstep run"
    requirement: "ENG-07"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::trace_sink_receives_exact_ordered_event_sequence_for_two_superstep_run"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/hooks.rs#tests::recording_sink_receives_emitted_events_in_order"
        status: pass
    human_judgment: false
  - id: D2
    description: "A sink whose handler never returns does not stall the run (proven with an explicit timeout at both the dispatcher level and through a real WarEngine::start call)"
    requirement: "ENG-07"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/hooks.rs#tests::permanently_blocking_sink_never_stalls_emit"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::permanently_blocking_trace_sink_does_not_stall_a_real_run"
        status: pass
    human_judgment: false
  - id: D3
    description: "A sink returning an error on every event leaves the run's outcome and final Battlefield identical to a run with no sink attached"
    requirement: "ENG-07"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::always_erroring_trace_sink_leaves_run_outcome_and_battlefield_unchanged"
        status: pass
    human_judgment: false
  - id: D4
    description: "A bounded channel drops the OLDEST event (not the newest) when full, and the drop counter is readable and greater than zero"
    requirement: "ENG-07"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/hooks.rs#tests::full_queue_drops_the_oldest_event_not_the_newest"
        status: pass
    human_judgment: false
  - id: D5
    description: "An empty NodeInterceptor chain produces identical node executions, final Battlefield and Waypoint/execution counts to a run with no chain configured"
    requirement: "ENG-07"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::empty_interceptor_chain_is_identical_to_no_chain_configured"
        status: pass
    human_judgment: false
  - id: D6
    description: "Skip records NodeOutcomeKind::Skipped with its reason and contributes no delta; Fail turns the node into a node failure carrying the given NodeError; Proceed executes normally"
    requirement: "ENG-07"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::skip_decision_produces_skipped_execution_record_and_no_delta"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::fail_decision_fails_the_node_and_the_run"
        status: pass
    human_judgment: false
  - id: D7
    description: "Two interceptors run before first-to-last and each after observes the previous after's mutation; a Skip from the first short-circuits the second's before"
    requirement: "ENG-07"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::two_interceptors_run_before_first_to_last_and_after_observes_prior_mutation"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::skip_from_first_interceptor_short_circuits_second_interceptors_before"
        status: pass
    human_judgment: false
  - id: D8
    description: "Cancellation during a superstep lets that superstep finish and merge, persists a Halted Waypoint whose vanguard is exactly the nodes that would run next, and no downstream node executes; cancellation before the first superstep still yields a Halted Waypoint"
    requirement: "ENG-07"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::cancellation_during_superstep_finishes_it_then_halts_before_the_next"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::cancellation_before_first_superstep_still_yields_a_halted_waypoint"
        status: pass
    human_judgment: false
  - id: D9
    description: "resume continues a Halted thread to normal completion; an un-cancelled token behaves identically to no token"
    requirement: "ENG-07"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::resume_continues_a_halted_thread_to_normal_completion"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::uncancelled_token_behaves_identically_to_no_token"
        status: pass
    human_judgment: false

# Metrics
duration: ~70min
completed: 2026-09-02
status: complete
---

# Phase 22 Plan 09: Engine Seams — TraceSink, NodeInterceptor, Cancellation Summary

**Three non-interfering seams landed with no consumers: a fire-and-forget TraceSink dispatcher that provably cannot stall or fail a run, an empty-by-default NodeInterceptor chain proven to change nothing, and cancellation that always produces a consistent, resumable Halted Waypoint.**

## Performance

- **Duration:** ~70 min
- **Tasks:** 3 completed
- **Files modified:** 6 (2 created, 4 modified)

## Accomplishments

- `paladin_ports::output::trace_sink_port` ships `TraceEvent` (the seven ENG-FR-21 variants, `#[non_exhaustive]`), an object-safe `async_trait TraceSink`, and a diagnostic-only `TraceSinkError`. `DeltaMerged` carries changed field *names* only (T-22-32), never values.
- `engine::hooks::TraceDispatcher` forwards events to an optional `Arc<dyn TraceSink>` over a bounded, drop-oldest queue via a single background consumer task. `emit()` never awaits the sink's handler or channel backpressure — proven with a sink whose handler calls `std::future::pending().await` (permanently blocking) and a sink that errors on every call, both exercised at the dispatcher level AND through a real `WarEngine::start` run inside an explicit `tokio::time::timeout`. A full queue drops the OLDEST buffered event (not the newest), with a readable `AtomicU64` drop counter — proven precisely with a gated sink that lets the test control exactly which events survive.
- The superstep loop emits all seven events at their natural points: `RunStarted`/`RunFinished` bracket `WarEngine::start`/`resume_with_options`; `SuperstepStarted`, `NodeStarted`/`NodeFinished` (per node), `DeltaMerged` (carrying `Battlefield::merge`'s own `MergeReport::changed_fields`) and `WaypointSaved` (added to `persist_waypoint`, firing only on an actual successful save) happen inside `superstep::run`.
- `engine::hooks::{NodeInterceptor, InterceptDecision}` gives `WarEngine::with_interceptors` an ordered, empty-by-default chain wrapping each vanguard node's dispatch. `before` runs first-to-last with short-circuit on the first non-`Proceed`; `after` runs first-to-last over the resulting delta, each observing the previous one's mutation. `Skip` produces `NodeOutcomeKind::Skipped { reason }` in the `Waypoint` and contributes no delta (T-22-33, auditable rather than a silent no-op); `Fail` turns the node into a node failure carrying the given `NodeError`. An empty chain is proven, end to end through a real `WarEngine` run, to produce identical final Battlefield, Waypoint count and node-execution outcomes to no chain at all.
- `WarEngine::with_cancellation_token` accepts a `tokio_util::sync::CancellationToken` (not hand-rolled — reused from an existing `paladin-battalion` dependency, matching the pattern already in `tests/integration/mcp_streamable_http_test.rs`). Cancellation is checked at exactly one point, the top of `superstep::run`'s loop: the in-flight superstep always finishes and merges before the *next* iteration's check can trip, so a `Halted` `Waypoint` is always a consistent restart point, carrying the vanguard that would have run next. Cancelling before the first superstep still yields a `Halted` Waypoint (an empty `completed` list, the entry vanguard). `resume` continues a `Halted` thread with no code change beyond the existing non-`Completed` fall-through, since a `Halted` waypoint's stored vanguard is exactly where execution should resume.
- `engine::test_support` gained four `TraceSink` test doubles (`RecordingTraceSink`, `BlockingTraceSink`, `AlwaysErroringTraceSink`, `GatedTraceSink`), reused by both `hooks.rs`'s dispatcher-focused unit tests and `mod.rs`'s end-to-end `WarEngine`-level tests.
- `cargo test -p paladin-battalion --lib engine` (83 tests), `cargo test -p paladin-ports --lib` (105 tests), `cargo test --doc -p paladin-ports` (116 tests), `cargo test --test e2e_crash_resume`, `cargo test --test war_engine_tracer`, `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` are all green.

## Task Commits

The plan's three tasks share a single `superstep::run` call signature and the same per-node dispatch closure (trace emission, interceptor wiring, and node execution are interleaved in one spawned task per vanguard node), so a mechanically clean one-commit-per-task split across `mod.rs`/`superstep.rs`/`hooks.rs` would have required reconstructing fragile, throwaway intermediate states purely for git history granularity. Committed instead as two complete, independently-tested units:

1. **TraceSink port with typed event stream (ENG-FR-21)** - `68b06a28` (feat) — `paladin_ports::output::trace_sink_port` (new) and its `output/mod.rs` registration. Zero coupling to the engine; trivially isolable.
2. **Engine seams — trace dispatch, node interceptors, cancellation-to-Halted (ENG-FR-21/22/23)** - `d50971c6` (feat) — `engine::hooks` (new), plus `engine::mod.rs`, `engine::superstep.rs` and `engine::test_support.rs`, covering all three of the plan's tasks together.

**Plan metadata:** committed alongside this SUMMARY (worktree mode; STATE.md/ROADMAP.md excluded, orchestrator owns those after wave merge)

## Files Created/Modified

- `crates/paladin-ports/src/output/trace_sink_port.rs` - New: `TraceEvent`, `TraceSink`, `TraceSinkError`
- `crates/paladin-ports/src/output/mod.rs` - Registers `trace_sink_port`
- `crates/paladin-battalion/src/engine/hooks.rs` - New: `TraceDispatcher` (bounded drop-oldest dispatcher), `NodeInterceptor`/`InterceptDecision`
- `crates/paladin-battalion/src/engine/mod.rs` - `WarEngine` gains `trace_dispatcher`/`interceptors`/`cancellation_token` fields and `with_trace_sink`/`with_interceptors`/`with_cancellation_token` builders; `start`/`resume_with_options` emit `RunStarted`/`RunFinished` and pass the three new params through; ~15 new unit tests
- `crates/paladin-battalion/src/engine/superstep.rs` - `run()` gains `trace`/`interceptors`/`cancellation` parameters; a `NodeRunOutcome` enum (`Succeeded`/`Skipped`/`Failed`) replaces the plain `Result` per-node outcome; the per-node spawn closure runs the interceptor `before`/`after` chain and emits `NodeStarted`/`NodeFinished`; `persist_waypoint` emits `WaypointSaved` on success; a cancellation check sits at the top of the loop
- `crates/paladin-battalion/src/engine/test_support.rs` - New `RecordingTraceSink`, `BlockingTraceSink`, `AlwaysErroringTraceSink`, `GatedTraceSink` test doubles

## Decisions Made

See `key-decisions` in frontmatter. Most consequential: cancellation is observed at exactly one checkpoint (the top of the superstep loop), which turns out to structurally guarantee BOTH of ENG-FR-23's required behaviors (finishing an in-flight superstep, and still halting before the very first one) without any special-casing.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Initial cancellation tests used a background-polling task to trigger `token.cancel()`, which raced an in-memory chain fast enough to make the assertion flaky**
- **Found during:** Task 3, first run of `cancellation_during_superstep_finishes_it_then_halts_before_the_next`
- **Issue:** The test spawned a background task that polled `RecordingWaypointStore::saved_waypoints` in a loop and called `token.cancel()` once 2 waypoints existed. Since `CountingFunctionNode`'s in-memory execution completes in well under a millisecond, by the time the poller woke up and called `cancel()`, the run had often already progressed past superstep 3 — the assertion `halted.vanguard == vec![n3]` failed with `vec![n4]` on the very first execution.
- **Fix:** Rebuilt the fixture so the node that should trigger cancellation calls `token.cancel()` (a synchronous method) directly from inside its own execution closure, making the cancellation point deterministic rather than a race. Applied the same fix to `resume_continues_a_halted_thread_to_normal_completion`, which had the identical race (undetected there only because its assertions didn't check the exact halt point).
- **Files modified:** `crates/paladin-battalion/src/engine/mod.rs`
- **Verification:** `cargo test -p paladin-battalion --lib engine` — ran 3 times in a row with 0 flakes after the fix (previously failed on first run).
- **Committed in:** `d50971c6`

**2. [Rule 1 - Bug] `TraceDispatcher` initially carried a redundant `AtomicBool` shutdown flag**
- **Found during:** Task 1, during review before commit
- **Issue:** An early draft added a `closed: AtomicBool` field to `TraceQueue`, set by an explicit `Drop for TraceDispatcher` impl, so the background consumer task could distinguish "no message yet" from "shut down, exit". This was unnecessary: the consumer's `doorbell_rx.recv().await` already returns `None` once the paired `mpsc::Sender` is dropped via ordinary Rust drop glue when `TraceDispatcher` goes out of scope — no separate flag or explicit `Drop` impl needed.
- **Fix:** Removed the `closed` field and the `Drop` impl entirely; the doorbell channel's own closing is sufficient.
- **Files modified:** `crates/paladin-battalion/src/engine/hooks.rs`
- **Verification:** `cargo test -p paladin-battalion --lib engine::hooks` still green after removal; `cargo clippy` clean.
- **Committed in:** `d50971c6`

### Acceptance Criterion Note (not auto-fixed — flagged for visibility)

Task 3's acceptance criterion `grep -rn 'AtomicBool' crates/paladin-battalion/src/engine/ | grep -v '^\s*//' | wc -l` is `0` is **not met literally**: it currently returns `10`. None of these are a hand-rolled *cancellation* flag — the actual ENG-FR-23 requirement this check exists to protect. Cancellation itself is implemented purely via `Option<CancellationToken>` and `CancellationToken::is_cancelled()`, with zero `AtomicBool` anywhere in that code path. The 10 matches are:
- 1 pre-existing, unrelated to this plan: `test_support.rs`'s `RecordingWaypointStore::fail_next_save` (a save-failure-injection flag from Phase 22 Plan 01/05).
- 9 from this plan's own legitimate, non-cancellation uses: `BlockingTraceSink`/`GatedTraceSink` test-double invocation flags (`entered`, `gated_once`) and their imports, used to prove the TraceSink fire-and-forget guarantees (T-22-30, T-22-31) — unrelated to ENG-FR-23.

The grep is broader than its evident intent (a directory-wide string match rather than a check scoped to the cancellation implementation), and was already unsatisfiable before this plan started, given the pre-existing `fail_next_save` flag. Recorded here rather than silently passed over; not treated as a functional gap since the actual "no hand-rolled cancellation flag" requirement is met and verified (`WarEngine::with_cancellation_token` takes and stores a real `tokio_util::sync::CancellationToken`, checked via its own `is_cancelled()` method).

---

**Total deviations:** 2 auto-fixed (both Rule 1) plus 1 flagged acceptance-criterion note (over-broad grep, not a functional gap). No scope creep — no new public API beyond the plan's three tasks' `must_haves.artifacts`, no new crate dependencies (`tokio_util` was already a `paladin-battalion` dependency).

## Known Stubs / Verification Debt

None. All three tasks' `<verify>` commands were run and are green; no deferred assertions. Per the plan's explicit scope, this is a seams-only plan (no consumers) — `TraceSink`, `NodeInterceptor` and cancellation have no first-party consumer in this phase, which is the plan's stated objective, not a stub.

## Issues Encountered

Both deviations above were caught and resolved before their respective commits. Separately (a process note, not a functional issue): the plan's three tasks share `superstep::run`'s call signature and per-node dispatch closure tightly enough that a mechanically clean one-commit-per-task split of the shared engine files was not attempted — see "Task Commits" above for the two-commit structure actually used, and the reasoning for it.

## User Setup Required

None — no external service configuration required. All test doubles are in-process; no Docker or network access needed.

## Next Phase Readiness

- ENG-07 is fully covered: `TraceSink`/`TraceDispatcher`, `NodeInterceptor`/`InterceptDecision`, and cancellation-to-`Halted` all exist as proven, non-interfering seams with no consumers, exactly as the plan's objective specifies.
- `WarEngine::with_trace_sink`, `WarEngine::with_interceptors` and `WarEngine::with_cancellation_token` are stable builder surfaces later plans can build on without further signature changes.
- Doc 05 (agent runtime middleware) and Doc 07 (observability/OTel export, `paladin-eval`) have a working hook to plug into rather than needing to reopen the engine's execution path, per the plan's stated purpose.
- Doc 04's Aegis fault-tolerance policy is documented at the `NodeInterceptor` trait definition as wrapping OUTSIDE this chain — flagged here so a later epic does not nest retry/timeout logic inside interceptor decisions the wrong way round.
- No blockers for 22-10/22-11.

---
*Phase: 22-battlefield-state-superstep-engine*
*Completed: 2026-09-02*
