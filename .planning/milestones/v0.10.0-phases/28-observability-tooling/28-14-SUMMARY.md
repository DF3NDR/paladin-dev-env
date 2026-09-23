---
phase: 28-observability-tooling
plan: 14
subsystem: observability
tags: [rust, hexagonal-architecture, ports, run-inspector, tdd]

# Dependency graph
requires:
  - phase: 28-observability-tooling (plan 10)
    provides: "ExecutionOverlay/OverlaySource/Visit and to_mermaid_overlay (crates/paladin-battalion/src/engine/export/overlay.rs, mermaid.rs), GraphShape::from_graph/observed -- the overlay and diagram this plan's facade builds on"
  - phase: 28-observability-tooling (plan 11)
    provides: "RunTracePort::read(thread, after_seq, limit) for persisted trace rows -- the optional exact-edge upgrade this plan's service wires via with_run_trace_port"
  - phase: 28-observability-tooling (plan 04)
    provides: "RunTracePort/RunTraceError and the InMemoryRunTraceStore adapter used directly in this plan's own tests"
provides:
  - "RunInspectorPort (crates/paladin-ports/src/input/run_inspector_port.rs) -- a one-method, core-typed input port: async fn inspect(&ThreadId) -> Result<InspectorView, InspectorError>, mirroring RunEventStreamPort's convention so paladin-web can render the dev-ui inspector without a default-build edge to the battalion crate (ADR-0031)"
  - "InspectorView/SuperstepRow/CompletedRow/VisitSummary/InspectorSource/SuperstepStatus -- all core value types (ADR-0016), Serialize/Deserialize on the whole chain so the view can be embedded verbatim in the page's <script type=\"application/json\"> tag"
  - "RunInspectorService (src/application/services/run/inspector.rs) -- the facade RunInspectorPort implementation: resolves the thread's latest Run's assistant to a GraphShape via AssistantResolver, builds an ExecutionOverlay from persisted trace records when available (exact edges) or Waypoint history otherwise (derived edges), renders to_mermaid_overlay, and assembles per-superstep rows and aggregate visit summaries"
affects: ["28-15 (the dev-ui inspector page consumes InspectorView directly)"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "SuperstepRow's per-row fired_edges/evaluated_edges are derived by filtering the overlay's own (already deduplicated) global edge sets to the row's own completed-node ids as SOURCE -- not by re-deriving per-transition pairs -- so the same filter works identically whether the overlay came from Waypoints (derived) or a trace (exact)"
    - "field_changes is sourced from TraceEvent::DeltaMerged (grouped by its own superstep field) when the overlay's source is Trace, and by diffing two consecutive Waypoint Battlefields via Battlefield::get_raw (never a typed deserialize) when it is not -- either path only ever touches a field NAME, never a value (T-28-14-01)"
    - "A cache-hit CompletedRow's duration_ms/token_count are None by construction ((!cache_hit).then_some(..)), not a possibly-stale or possibly-zero number, so the page can render a dash without a separate null-check convention per field"

key-files:
  created:
    - crates/paladin-ports/src/input/run_inspector_port.rs
    - src/application/services/run/inspector.rs
  modified:
    - crates/paladin-ports/src/input/mod.rs
    - src/application/services/run/mod.rs

key-decisions:
  - "Graph resolution in this plan is a two-bucket subset of D-22 (no --graph flag exists in a port call): the thread's latest Run's assistant, resolved via AssistantResolver to a Workflow's WarGraph and rendered via GraphShape::from_graph; or, when no Run exists, resolution fails, or the assistant is Agent-kind, GraphShape::observed with observed_only: true. The CLI's --graph <FILE> bucket is 28-13's own scope, not this port's."
  - "SuperstepRow is ALWAYS built from Waypoint history, regardless of overlay source -- a Waypoint is the durability truth (28-04) and carries waypoint_id/vanguard/completed/Battlefield, none of which a trace-only reconstruction can fully replace without also tracking SuperstepStarted/WaypointSaved boundaries. The overlay's source (Waypoints vs Trace) only changes which edge/field-change data backs each row, never whether a row exists."
  - "WaypointPort::history only returns lightweight WaypointSummary rows (no completed/vanguard/battlefield) -- the service resolves each summary to its full Waypoint via WaypointPort::get before building the overlay or supersteps, since ExecutionOverlay::from_waypoints and the superstep-row builder both need the full record."
  - "Task 2's tdd=\"true\" RED commit stubbed evaluated_edges (always empty), status (always Running) and CompletedRow's duration/token figures (always Some, ignoring cache_hit) rather than leaving the new struct fields unimplemented (which would not compile) -- three of the six new <behavior> tests failed against the stubs before the GREEN commit implemented the real logic, keeping the RED/GREEN split meaningful for a structural-addition task rather than purely a type change."

requirements-completed: [OBS-03]

coverage:
  - id: D1
    description: "RunInspectorPort is a one-method core-typed input port (inspect(&ThreadId) -> Result<InspectorView, InspectorError>) with zero battalion-crate references anywhere in the port file or paladin-ports' Cargo.toml"
    requirement: "OBS-03"
    verification:
      - kind: unit
        ref: "crates/paladin-ports/src/input/run_inspector_port.rs#run_inspector_port_is_object_safe"
        status: pass
      - kind: other
        ref: "grep -c 'paladin_battalion\\|paladin-battalion' crates/paladin-ports/src/input/run_inspector_port.rs == 0; grep -c 'paladin-battalion' crates/paladin-ports/Cargo.toml == 0; cargo tree -p paladin-ports --no-default-features -e normal | grep -c -E 'paladin-battalion|paladin-llm|paladin-storage' == 0"
        status: pass
    human_judgment: false
  - id: D2
    description: "RunInspectorService builds a view for a thread with three Waypoints -- three SuperstepRows in superstep order, each with waypoint id, vanguard, completed rows and field-change names, and a non-empty mermaid diagram"
    requirement: "OBS-03"
    verification:
      - kind: unit
        ref: "src/application/services/run/inspector.rs#inspect_returns_a_view_for_a_thread_with_waypoints"
        status: pass
    human_judgment: false
  - id: D3
    description: "A loop node visited at supersteps 2, 4 and 6 answers the OBS-03 acceptance question: VisitSummary.count == 3, supersteps == [2, 4, 6], and every one of those superstep rows names the self-loop edge in its fired_edges"
    requirement: "OBS-03"
    verification:
      - kind: unit
        ref: "src/application/services/run/inspector.rs#visit_summary_answers_the_acceptance_question"
        status: pass
    human_judgment: false
  - id: D4
    description: "field_changes carries FieldName values only, never a Battlefield field value, whether derived by Waypoint-diff or read from a trace's DeltaMerged -- proven over Debug formatting (Task 1) and over the real serialized JSON payload (Task 2)"
    requirement: "OBS-03"
    verification:
      - kind: unit
        ref: "src/application/services/run/inspector.rs#field_changes_are_names_only"
        status: pass
      - kind: unit
        ref: "src/application/services/run/inspector.rs#serialized_view_contains_no_field_values"
        status: pass
    human_judgment: false
  - id: D5
    description: "With run_traces rows present, InspectorView.source is InspectorSource::Trace and fired_edges/evaluated_edges come from EdgeEvaluated records rather than derivation; a Waypoints-only view's evaluated_edges stays empty on every row"
    requirement: "OBS-03"
    verification:
      - kind: unit
        ref: "src/application/services/run/inspector.rs#trace_source_populates_exact_edges"
        status: pass
      - kind: unit
        ref: "src/application/services/run/inspector.rs#view_distinguishes_exact_from_derived_edges"
        status: pass
    human_judgment: false
  - id: D6
    description: "An unknown thread (no Run row, no Waypoints, no trace rows) is InspectorError::ThreadNotFound; a known thread (a Run row exists) with no history yields Ok with empty supersteps/visits, not an error"
    requirement: "OBS-03"
    verification:
      - kind: unit
        ref: "src/application/services/run/inspector.rs#unknown_thread_is_thread_not_found"
        status: pass
      - kind: unit
        ref: "src/application/services/run/inspector.rs#thread_with_no_history_is_an_empty_view"
        status: pass
    human_judgment: false
  - id: D7
    description: "observed_only and source are both correct for a thread with a resolvable graph and for one with none; every state the UI-SPEC contract requires (cache-hit partial figures, an awaiting-input superstep's empty completed list and labelable status, and the whole view's JSON round-trip) is representable from InspectorView alone"
    requirement: "OBS-03"
    verification:
      - kind: unit
        ref: "src/application/services/run/inspector.rs#view_carries_observed_only_and_source"
        status: pass
      - kind: unit
        ref: "src/application/services/run/inspector.rs#completed_row_partial_values_are_representable"
        status: pass
      - kind: unit
        ref: "src/application/services/run/inspector.rs#superstep_awaiting_input_has_an_empty_completed_list"
        status: pass
      - kind: unit
        ref: "src/application/services/run/inspector.rs#view_is_serializable_and_round_trips"
        status: pass
    human_judgment: false

# Metrics
duration: 42min
completed: 2026-09-09
status: complete
---

# Phase 28 Plan 14: Run Inspector Port and Service Summary

**`RunInspectorPort` (core-typed, `paladin-ports`) and its facade `RunInspectorService` (`paladin-ai`), turning the 28-10 execution overlay, Waypoint history and an optional persisted trace into one `InspectorView` — diagram, per-superstep table and aggregate visit summaries — for the `dev-ui` run inspector page.**

## Performance

- **Duration:** ~42 min
- **Started:** 2026-09-09T03:45:30Z (base commit for this wave)
- **Completed:** 2026-09-09T04:25:42Z (Task 2 GREEN commit)
- **Tasks:** 2 (1 tracer, 1 auto/tdd — Task 2's `tdd="true"` attribute produced a RED/GREEN pair) — 3 commits total
- **Files modified:** 4 (2 new, 2 modified)

## Accomplishments
- `crates/paladin-ports/src/input/run_inspector_port.rs`: `RunInspectorPort: Send + Sync { async fn inspect(&self, thread: &ThreadId) -> Result<InspectorView, InspectorError> }` — the whole `InspectorView`/`SuperstepRow`/`CompletedRow`/`VisitSummary`/`InspectorSource`/`SuperstepStatus` value chain built entirely from core types (ADR-0016), `Serialize`/`Deserialize` on every one of them so the page can embed the payload verbatim, and a `#[non_exhaustive]` `InspectorError` (`ThreadNotFound`, `Backend`, `NotWired`) with structured fields (X-06). Zero references to the battalion crate anywhere in the file or `paladin-ports`' `Cargo.toml` (T-28-14-02).
- `src/application/services/run/inspector.rs`: `RunInspectorService` holds a `WaypointPort`, a `RunRepositoryPort`, an `AssistantResolver`, and an optional `RunTracePort` (`with_run_trace_port`, additive builder). `inspect` resolves the thread's latest `Run` (if any), loads its full Waypoint history (`WaypointPort::history` summaries resolved to full records via `WaypointPort::get`), and paginates any persisted trace rows. An unknown thread (no `Run`, no Waypoints, no trace rows) is `InspectorError::ThreadNotFound`; a known thread with none of the above yet is `Ok` with empty `supersteps`/`visits`.
- Graph resolution: the thread's `Run`'s assistant is resolved through `AssistantResolver` to a `Workflow`'s `WarGraph` and rendered via `GraphShape::from_graph`; when there is no `Run`, resolution fails, or the assistant is `Agent`-kind, `GraphShape::observed(&overlay)` is used instead and `observed_only: true` is carried onto the view — the two-bucket subset of D-22's resolution order this port call actually has (no CLI `--graph` flag exists here; that bucket is 28-13's own scope).
- Overlay construction: `ExecutionOverlay::from_trace_records` when persisted rows exist (exact `fired_edges`/`evaluated_edges`), `ExecutionOverlay::from_waypoints` otherwise (derived `fired_edges`, `evaluated_edges` always empty) — `overlay.observed_only` is stamped from the graph-resolution outcome above regardless of edge source, and `to_mermaid_overlay` renders the diagram against whichever `GraphShape` was resolved.
- Superstep rows are **always** built from the thread's full Waypoint history (a Waypoint is the durability truth, 28-04) — each row's `fired_edges`/`evaluated_edges` are the overlay's own global sets filtered to that row's completed-node ids as source, and `field_changes` comes from `TraceEvent::DeltaMerged` (grouped by its own `superstep` field) when the trace source is available, or from diffing two consecutive Waypoint `Battlefield`s via `Battlefield::get_raw` (never a typed deserialize) otherwise — either path touches only a field NAME, never a value (T-28-14-01).
- `CompletedRow.duration_ms`/`token_count` are `Option<u64>`, `None` whenever `cache_hit` is `true` (a cache-served attempt's stored figures are not a meaningful measurement) rather than a possibly-stale or possibly-zero number. `SuperstepRow.status` maps `WaypointStatus` to a payload-free `SuperstepStatus` (`Running`/`Completed`/`Failed`/`AwaitingInput`/`Halted`) so an awaiting-input superstep (empty `completed` list, a Gate suspension) is labelable by the page rather than looking like a missing row.
- Thirteen tests total (3 port-level object-safety/error-enum tests + 12 facade tests, all against in-memory ports — `InMemoryWaypointStore`, `InMemoryRunRepository`, `InMemoryRunTraceStore`, `CodeWorkflowResolver`), plus 1 doctest.

## Task Commits

Each task was committed atomically (Task 2's `tdd="true"` attribute produced a RED/GREEN pair):

1. **Task 1: `RunInspectorPort` and its facade implementation** (tracer) - `96c2e50a` (feat)
2. **Task 2 RED: failing tests for `evaluated_edges`, partial `CompletedRow` figures and awaiting-input rows** - `1497466a` (test)
3. **Task 2 GREEN: implement `evaluated_edges`, cache-hit partial figures and superstep status** - `e396fc17` (feat)

**Plan metadata:** (this commit, following this SUMMARY)

## Files Created/Modified
- `crates/paladin-ports/src/input/run_inspector_port.rs` - New: `RunInspectorPort`, `InspectorView`, `SuperstepRow`, `CompletedRow`, `VisitSummary`, `InspectorSource`, `SuperstepStatus`, `InspectorError`; 3 unit tests + 1 doctest
- `crates/paladin-ports/src/input/mod.rs` - `pub mod run_inspector_port;`
- `src/application/services/run/inspector.rs` - New: `RunInspectorService` implementing `RunInspectorPort`; 12 unit tests
- `src/application/services/run/mod.rs` - `pub mod inspector;`, `pub use inspector::RunInspectorService;`

## Decisions Made

See `key-decisions` in frontmatter. The most consequential: `SuperstepRow`s are always built from Waypoint history regardless of which source backs the overlay's edge data — a trace is a replay convenience layered on top of the Waypoint, never a substitute for it, since only a Waypoint carries `waypoint_id`/`vanguard`/`completed`/`Battlefield` directly. Reconstructing that shape from trace events alone would require tracking `SuperstepStarted`/`WaypointSaved` boundaries for no benefit this plan's `<behavior>` tests needed.

## Deviations from Plan

None - plan executed exactly as written. The plan's own scope note ("the assistant store" in Task 1's action text) resolved cleanly to the existing `AssistantResolver` seam (`src/application/services/run/resolver.rs`) already used by `events.rs`/`worker.rs` — no new port or type was needed to satisfy it.

## Issues Encountered

None. `WaypointPort::history` returning only lightweight `WaypointSummary` rows (not full `Waypoint`s) was anticipated from reading `waypoint_port.rs` during the `<read_first>` pass, not discovered as a surprise mid-implementation — resolved by resolving each summary to its full record via `WaypointPort::get` before building the overlay or supersteps.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `RunInspectorPort`/`InspectorView` are ready for 28-15's `dev-ui` inspector page (`GET /v1/dev-ui/threads/{id}`) to consume directly — every field the UI-SPEC's four panels need (the `mermaid` diagram, `visits` for the node-visits panel, each `SuperstepRow.fired_edges`/`evaluated_edges` for the fired-edge panel, and `supersteps` for the superstep table) already exists on the view, and the whole payload round-trips through `serde_json` for the page's embedded `<script type="application/json">` tag.
- `paladin-web` can depend on `paladin-ports::input::run_inspector_port` with no default-build edge to the battalion crate (ADR-0031), matching the `RunEventStreamPort` precedent 28-15 will follow to implement the route.
- No blockers.

## Self-Check: PASSED

- FOUND: crates/paladin-ports/src/input/run_inspector_port.rs
- FOUND: src/application/services/run/inspector.rs
- FOUND: commit 96c2e50a
- FOUND: commit 1497466a
- FOUND: commit e396fc17

## Verification Commands Run (all green)

- `cargo test -p paladin-ports --lib input::run_inspector_port` -- 3 passed
- `cargo test -p paladin-ports --doc run_inspector_port` -- 1 passed
- `cargo test -p paladin-ai --lib services::run::inspector` -- 12 passed
- `cargo fmt --all --check` -- exit 0
- `cargo check --workspace --all-targets --all-features` -- exit 0
- `cargo clippy --all-targets --all-features -p paladin-ai-core -p paladin-ports -p paladin-ai -- -D warnings` -- exit 0
- `cargo clippy --workspace --all-targets -- -D warnings` -- exit 0
- `cargo tree -p paladin-ports --no-default-features -e normal | grep -c -E "paladin-battalion|paladin-llm|paladin-storage"` -- 0 (ADR-0015/0031 upheld)
- `grep -c 'paladin_battalion\|paladin-battalion' crates/paladin-ports/src/input/run_inspector_port.rs` -- 0
- `grep -c 'paladin-battalion' crates/paladin-ports/Cargo.toml` -- 0

---
*Phase: 28-observability-tooling*
*Completed: 2026-09-09*
