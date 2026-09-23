---
phase: 28-observability-tooling
plan: 01
subsystem: observability
tags: [trace, tracing, serde, tokio, dispatcher, composite-sink, multi-thread-stress]

# Dependency graph
requires:
  - phase: 22
    provides: "The eight-variant TraceEvent seam and the fire-and-forget TraceDispatcher (bounded, drop-oldest queue) in paladin-battalion::engine::hooks, with no real consumer wired yet"
  - phase: 25
    provides: "NodeStarted/NodeFinished attempt+cache_hit fields, FallbackHop emitted below the engine by FallbackLlmAdapter"
  - phase: 27
    provides: "RunEventBusSink/map_trace_event bridging TraceEvent onto the SSE bus; WarGraphDoc"
provides:
  - "The authoritative twelve-variant TraceEvent enum and TraceRecord envelope in paladin-core (new module platform::container::trace)"
  - "TraceEmitter (synchronous, object-safe) and CompositeSink (panic-isolated fan-out) in paladin-ports"
  - "TraceDispatcher stamping seq/at/thread_id/run_id at enqueue time, with sink-panic isolation, drop accounting reconciled onto RunFinished, and a first-drop warning"
  - "PanickingTraceSink test double alongside the four existing ones"
  - "A multi-thread stress test proving per-run seq gaplessness under real concurrency (X-05)"
affects: ["28-02 (four new producers + log sink + facade composition)", "28-03 (RunFinished status/totals)", "28-04 (SSE collapse, RunTracePort)", "28-06 (with_trace_sink -> with_trace_emitter rename)", "28-11 (map_trace_event seven-of-twelve completion)"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Envelope-over-tagged-enum: #[serde(flatten)] TraceRecord wraps a #[serde(tag = \"kind\")] TraceEvent for a single flat JSON object per record"
    - "Per-run dispatcher construction: WarEngine builds a fresh, thread-scoped TraceDispatcher at the top of each entry point (start/resume/resume_with/replay/fork) instead of holding one engine-lifetime dispatcher, so seq starts at 1 per run even when one engine instance serves multiple threads"
    - "In-process OS-thread-scoped log::Log capturer for asserting a specific warn!/error! line without a new test dependency"

key-files:
  created:
    - crates/paladin-core/src/platform/container/trace.rs
  modified:
    - crates/paladin-core/src/platform/container/mod.rs
    - crates/paladin-ports/src/output/trace_sink_port.rs
    - crates/paladin-ports/Cargo.toml
    - crates/paladin-battalion/src/engine/hooks.rs
    - crates/paladin-battalion/src/engine/test_support.rs
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/src/engine/superstep.rs
    - crates/paladin-battalion/src/engine/graph.rs
    - crates/paladin-llm/src/fallback.rs
    - src/application/services/run/events.rs

key-decisions:
  - "Task 1 checkpoint:decision — option-a auto-selected by the orchestrator under --auto (2026-09-08): TraceRecord is a #[serde(flatten)] envelope {thread_id, run_id?, seq, at, kind, ...event fields} over a #[serde(tag = \"kind\", rename_all = \"snake_case\")] TraceEvent, under TRACE_SCHEMA_VERSION = \"1\" (D-02 verbatim)."
  - "Renamed TraceEvent::NodeProgress's and ParleyRaised's payload field from the PRD's literal 'kind' to 'progress'/'parley_kind' respectively — the enclosing TraceEvent is itself tagged #[serde(tag = \"kind\")], so a same-named payload field would collide with the enum's own discriminant key on the wire (two 'kind' entries in one flat object). Same data, collision-free key."
  - "WarEngine no longer holds one TraceDispatcher for its whole lifetime; it holds trace_sink/trace_capacity and constructs a fresh, thread-scoped TraceDispatcher inside each entry point (start/resume_with_options/resume_with/replay_or_fork), matching D-03's 'seq starts at 1 per run' guarantee even though this crate's own unit tests routinely reuse one WarEngine across many different ThreadIds."
  - "RunFinished's status/total_supersteps/total_tokens/duration_ms are placeholder defaults at every engine/mod.rs call site (per the plan text's explicit allowance: '28-03 fills status/totals'); only trace_dropped_total is live, stamped by TraceDispatcher::emit itself at enqueue time."
  - "DeltaMerged's FieldChange.dispatch/writers/value_bytes are placeholder defaults (empty/zero) at the superstep.rs call site, since Battlefield::merge's MergeReport today only tracks changed field NAMES, not per-field dispatch rule/writers/size — enriching MergeReport is out of this plan's scope."
  - "FallbackLlmAdapter keeps with_trace_sink unchanged (the with_trace_sink -> with_trace_emitter rename is 28-06's) and wraps its FallbackHop in a placeholder-thread TraceRecord (ThreadId::new(\"fallback-adapter\")), since the adapter has no real thread context below the engine yet."
  - "The X-05 stress test's cross-child equality check compares CompositeSink's two children's records order-independently (sorted by (thread_id, seq)) rather than raw Vec equality: sixteen independent producer tasks racing through one shared sink give no ordering guarantee between two DIFFERENT records' own on_event calls, only that within any ONE call every child sees the record — asserting raw Vec order would assert a guarantee the design never made."

patterns-established:
  - "New trace producers/consumers should route through TraceRecord (never bare TraceEvent) once past a TraceDispatcher, and any new TraceEvent variant must extend the wildcard-arm-required match sites in hooks.rs, mod.rs and events.rs"

requirements-completed: [OBS-01]

coverage:
  - id: D1
    description: "Twelve-variant TraceEvent + TraceRecord envelope in paladin-core, serializing to one flat JSON object with seq before kind"
    requirement: "OBS-01"
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/trace.rs#all_twelve_event_variants_construct"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/trace.rs#record_serializes_as_one_flat_object"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/trace.rs#record_round_trips_through_serde"
        status: pass
    human_judgment: false
  - id: D2
    description: "TraceEmitter (synchronous, object-safe) and CompositeSink (panic-isolated fan-out, Ok unless every child fails) in paladin-ports"
    requirement: "OBS-01"
    verification:
      - kind: unit
        ref: "crates/paladin-ports/src/output/trace_sink_port.rs#emitter_trait_is_object_safe"
        status: pass
      - kind: unit
        ref: "crates/paladin-ports/src/output/trace_sink_port.rs#composite_sink_forwards_to_every_child_even_when_one_panics"
        status: pass
      - kind: unit
        ref: "crates/paladin-ports/src/output/trace_sink_port.rs#composite_sink_errs_only_when_every_child_fails"
        status: pass
    human_judgment: false
  - id: D3
    description: "TraceDispatcher stamps seq/at/thread_id/run_id at enqueue time; seq is 1-based and gapless per run"
    requirement: "OBS-01"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/hooks.rs#trace_seq_is_gapless_over_twenty_supersteps"
        status: pass
    human_judgment: false
  - id: D4
    description: "Drops are counted and reconcile three ways (observed seq gaps == RunFinished.trace_dropped_total == dispatcher.dropped_count()); RunFinished is never itself the dropped event; first drop logs exactly one warning"
    requirement: "OBS-01"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/hooks.rs#trace_drops_are_counted_and_reconcile"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/hooks.rs#run_finished_is_never_dropped"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/hooks.rs#first_drop_logs_one_warning"
        status: pass
    human_judgment: false
  - id: D5
    description: "A panicking sink is caught, counted (sink_panics), and never kills the dispatcher's consumer task; a slow sink (500ms/event) never inflates the run's own wall clock"
    requirement: "OBS-01"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/hooks.rs#panicking_sink_does_not_kill_the_consumer"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/hooks.rs#slow_sink_does_not_slow_the_run"
        status: pass
    human_judgment: false
  - id: D6
    description: "Sixteen concurrent traced runs on a multi-thread runtime through one CompositeSink of two recording children yield exact per-run record counts and gapless per-run seq on both children (X-05)"
    requirement: "OBS-01"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/hooks.rs#sixteen_concurrent_runs_keep_per_run_seq_gapless"
        status: pass
    human_judgment: false
  - id: D7
    description: "Every existing producer/consumer in the workspace (superstep.rs, WarEngine's four entry points, FallbackLlmAdapter, the facade's RunEventBusSink/map_trace_event) migrated to the new envelope so the whole workspace stays green"
    verification:
      - kind: unit
        ref: "cargo check --workspace --all-targets --all-features (exit 0)"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ai-core -p paladin-ports -p paladin-battalion -p paladin-llm --lib (585+765+183+104 passed, 0 failed)"
        status: pass
    human_judgment: false

# Metrics
duration: 55min
completed: 2026-09-08
status: complete
---

# Phase 28 Plan 01: End-to-End Trace Envelope Summary

**Twelve-variant `TraceEvent`/`TraceRecord` envelope (paladin-core), `TraceEmitter` + panic-isolated `CompositeSink` (paladin-ports), and a per-run-stamping `TraceDispatcher` with drop accounting, sink-panic isolation and a multi-thread stress test (paladin-battalion) — every existing producer and consumer in the workspace migrated in the same change.**

## Performance

- **Duration:** ~55 min
- **Started:** 2026-09-08T21:38:00Z (phase execution start per STATE.md)
- **Completed:** 2026-09-08T22:33:00Z
- **Tasks:** 4 (1 checkpoint:decision auto-resolved, 3 auto)
- **Files modified:** 11 (1 created)

## Checkpoint Status

Task 1 `checkpoint:decision` ("Confirm the one-way `TraceRecord` wire shape") — **option-a auto-selected by the orchestrator under `--auto`** (2026-09-08): proceed with D-02 verbatim (`#[serde(flatten)]` envelope over a `#[serde(tag = "kind", rename_all = "snake_case")]` enum, `TRACE_SCHEMA_VERSION = "1"`). No stop, no `## CHECKPOINT REACHED` — execution proceeded directly to Task 2.

## Accomplishments

- New `crates/paladin-core/src/platform/container/trace.rs`: the authoritative twelve-variant `TraceEvent` (D-01/D-02), the `TraceRecord` envelope, `FieldChange`, `NodeProgressKind`, `MiddlewareAction`, `RunFinishStatus`, `TRACE_SCHEMA_VERSION`. Four unit tests: construction of all twelve variants, flat-object serialization order, round-trip fidelity, schema version.
- `crates/paladin-ports/src/output/trace_sink_port.rs` re-exports the core types (D-01), retargets `TraceSink::on_event` to `TraceRecord`, and adds the synchronous object-safe `TraceEmitter` trait (D-03) plus the panic-isolated `CompositeSink` fan-out (D-08). Eight unit tests.
- `crates/paladin-battalion/src/engine/hooks.rs`: `TraceDispatcher` is constructed with the specific `thread_id`/`run_id` it stamps every record for; `emit` stamps `seq`/`at` at enqueue time, overwrites `RunFinished.trace_dropped_total` from its own `dropped_count()`, logs exactly one `warn!` on the dispatcher's first drop, and wraps the consumer's `sink.on_event` in `catch_unwind` (D-08) so a panicking sink is caught, counted (`sink_panics()`), and never kills the consumer task. Twenty-one tests total in this module (14 existing/updated + 6 new ordering/drop/panic tests + 1 X-05 stress test), all passing, re-run repeatedly with no flakes.
- `crates/paladin-battalion/src/engine/test_support.rs` adds `PanickingTraceSink` alongside the four existing trace test doubles.
- `crates/paladin-battalion/src/engine/mod.rs`/`superstep.rs`/`graph.rs`: every one of the fifteen `trace_dispatcher` reference sites in `mod.rs` and the five emission sites in `superstep.rs` migrated to the new envelope; `WarEngine` now constructs a fresh, thread-scoped dispatcher per entry-point call rather than holding one for its whole lifetime.
- `crates/paladin-llm/src/fallback.rs` and `src/application/services/run/events.rs` updated to compile and pass their own test suites against the new `TraceRecord`-based `TraceSink::on_event` signature and the twelve-variant enum, with `map_trace_event` keeping today's four mapped kinds (per the plan text — the seven-of-twelve completion is 28-11's).

## Task Commits

1. **Task 1: checkpoint:decision (auto-resolved, no commit — resolution recorded in Task 2's commit message)**
2. **Task 2: End-to-end trace envelope — core types, ports re-export, `TraceEmitter`, `CompositeSink`, dispatcher stamping, all call sites migrated** - `077bcfea` (feat)
3. **Task 3: Drop accounting, panic isolation and the ordering property test** - `5e36158d` (feat)
4. **Task 4: X-05 multi-thread stress — 16 concurrent runs through one `CompositeSink`** - `08ba7397` (test)

**Plan metadata:** (this commit)

## Files Created/Modified

- `crates/paladin-core/src/platform/container/trace.rs` - New: `TraceEvent` (12 variants), `TraceRecord`, `FieldChange`, `NodeProgressKind`, `MiddlewareAction`, `RunFinishStatus`, `TRACE_SCHEMA_VERSION`
- `crates/paladin-core/src/platform/container/mod.rs` - `pub mod trace;`
- `crates/paladin-ports/src/output/trace_sink_port.rs` - Re-exports core trace types; `TraceEmitter`, `CompositeSink`; `TraceSink::on_event(TraceRecord)`
- `crates/paladin-ports/Cargo.toml` - Added `log` (workspace) dependency for `CompositeSink`'s panic-log line
- `crates/paladin-battalion/src/engine/hooks.rs` - `TraceDispatcher` construction/stamping/panic-isolation/drop-accounting rewrite; 7 new tests
- `crates/paladin-battalion/src/engine/test_support.rs` - `PanickingTraceSink`
- `crates/paladin-battalion/src/engine/mod.rs` - `WarEngine` holds `trace_sink`/`trace_capacity`; per-call dispatcher construction at all four entry points
- `crates/paladin-battalion/src/engine/superstep.rs` - All five trace emission sites migrated; `node_outcome_kind()` helper
- `crates/paladin-battalion/src/engine/graph.rs` - Test helper `TraceDispatcher::new` call site updated
- `crates/paladin-llm/src/fallback.rs` - `TraceRecord`-wrapped `FallbackHop` emission; test doubles updated
- `src/application/services/run/events.rs` - `map_trace_event(TraceRecord)`; `RunEventBusSink::on_event(TraceRecord)`; test suite rewritten for twelve variants

## Decisions Made

See `key-decisions` in frontmatter. The two most consequential: (1) renaming `NodeProgress`/`ParleyRaised`'s payload field away from the PRD's literal `kind` to avoid colliding with `TraceEvent`'s own `#[serde(tag = "kind")]` discriminant; (2) restructuring `WarEngine` to construct a fresh, thread-scoped `TraceDispatcher` per entry-point call instead of holding one dispatcher for the engine's whole lifetime, so `seq` truly starts at 1 per run even in this crate's own unit tests (which routinely reuse one `WarEngine` across many `ThreadId`s) — matching D-03's guarantee without requiring test-suite changes beyond the trace call sites themselves.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `NodeProgress`/`ParleyRaised` field-name collision with `TraceEvent`'s own serde tag**
- **Found during:** Task 2 (authoring `trace.rs`)
- **Issue:** D-02's prose names `NodeProgress`'s payload field `kind: NodeProgressKind` and `ParleyRaised`'s payload field `kind: ParleyKind`. `TraceEvent` itself is internally tagged `#[serde(tag = "kind")]`; a variant whose own payload also declares a field named `kind` would serialize with the discriminant tag key and the payload field silently colliding on the SAME JSON key, defeating the "one flat object" contract this whole plan exists to prove.
- **Fix:** Renamed the two fields to `progress` (`NodeProgress`) and `parley_kind` (`ParleyRaised`) — same data, collision-free key. Documented in `trace.rs`'s own module docs as a deliberate, explained deviation from the PRD's literal field name.
- **Files modified:** `crates/paladin-core/src/platform/container/trace.rs`
- **Verification:** `record_serializes_as_one_flat_object` and `all_twelve_event_variants_construct` both pass; a round-trip test would have caught the collision immediately had it gone unfixed.
- **Committed in:** `077bcfea` (Task 2 commit)

**2. [Rule 3 - Blocking] `superstep.rs`, `graph.rs`, and the crate's own test modules were not listed in `files_modified` but required edits to compile against the new envelope**
- **Found during:** Task 2 (workspace build)
- **Issue:** The plan's frontmatter `files_modified` list omits `crates/paladin-battalion/src/engine/superstep.rs` and `crates/paladin-battalion/src/engine/graph.rs`, both of which construct `TraceEvent` literals directly (`SuperstepStarted`/`NodeStarted`/`NodeFinished`/`DeltaMerged`/`WaypointSaved`, and a `TraceDispatcher::new(None)` test helper respectively) and would not compile once `trace.rs`'s per-variant `thread_id` fields were removed.
- **Fix:** Migrated every trace call site in both files to the new envelope shape, added a `node_outcome_kind()` helper mapping `NodeRunOutcome` onto the Waypoint's own `NodeOutcomeKind` vocabulary, and added `muster_task_key`/`vanguard` population from data already in scope at each call site.
- **Files modified:** `crates/paladin-battalion/src/engine/superstep.rs`, `crates/paladin-battalion/src/engine/graph.rs`
- **Verification:** `cargo build --workspace --all-targets --all-features` succeeds; the full `paladin-battalion` lib suite (765 tests) passes.
- **Committed in:** `077bcfea` (Task 2 commit)

**3. [Rule 2 - Missing Critical] `paladin-ports` was missing the `log` crate dependency `CompositeSink`'s panic-log line needed**
- **Found during:** Task 2 (authoring `CompositeSink`)
- **Issue:** D-08 requires `CompositeSink` to log at `error` level when a child panics; `paladin-ports`' `Cargo.toml` had no `log` dependency (a workspace-level dependency already used by sibling crates).
- **Fix:** Added `log = { workspace = true }` to `crates/paladin-ports/Cargo.toml`.
- **Files modified:** `crates/paladin-ports/Cargo.toml`
- **Verification:** `cargo build -p paladin-ports` succeeds; the crate's own clippy pass is clean.
- **Committed in:** `077bcfea` (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (1 bug, 1 blocking, 1 missing-critical). All within the same subsystem the plan's own tasks touch; no scope creep beyond what compiling and testing the plan's own deliverable required.
**Impact on plan:** All three fixes were necessary for the plan's own stated acceptance criterion ("`cargo build --workspace --all-targets` succeeds — every existing consumer compiles against the envelope") to hold. No behavior outside the trace subsystem was touched.

## Issues Encountered

None beyond the deviations documented above. The design tension between D-03's "seq starts at 1 per run" (implying a dispatcher scoped to one thread/run) and `WarEngine`'s existing API (a single long-lived engine object whose `start`/`resume`/`resume_with`/`replay`/`fork` methods each take a `thread: ThreadId` parameter, reused across many threads in this crate's own unit tests) was resolved by constructing a fresh `TraceDispatcher` per top-level entry-point call rather than per `WarEngine` instance — documented in `hooks.rs`'s and `mod.rs`'s own doc comments and in the `key-decisions` above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The twelve-variant `TraceEvent`/`TraceRecord` contract this plan establishes is what every remaining Phase 28 plan builds on: the four new producers + log sink + facade composition (28-02), `RunFinished` status/totals (28-03), the OTel sink, SSE collapse, `RunTracePort`, graph export, the inspector and `paladin-eval` harness.
- `WarEngine::trace_emitter()` (the accessor letting a caller wire the SAME per-run handle into `FallbackLlmAdapter`/middleware) is deliberately NOT added by this plan — it is listed under "this plan's own contribution" as absent, and is expected from a later plan (28-02's "four new producers" wave) once there is a real producer needing it.
- `FieldChange.dispatch`/`writers`/`value_bytes` and `RunFinished.status`/totals are placeholder defaults at their respective call sites, exactly as the plan text sanctions ("28-03 fills status/totals"); a later plan enriching `Battlefield::merge`'s `MergeReport` will need to thread real dispatch/writer/size data through to `DeltaMerged` — currently undocumented as a TODO anywhere except this SUMMARY and the inline code comments at the `superstep.rs` call site.
- No blockers for 28-02.

## Self-Check: PASSED

- FOUND: `crates/paladin-core/src/platform/container/trace.rs`
- FOUND: `.planning/phases/28-observability-tooling/28-01-SUMMARY.md`
- FOUND commit: `077bcfea`
- FOUND commit: `5e36158d`
- FOUND commit: `08ba7397`

---
*Phase: 28-observability-tooling*
*Completed: 2026-09-08*
