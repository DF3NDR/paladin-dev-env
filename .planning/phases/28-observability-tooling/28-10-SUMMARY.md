---
phase: 28-observability-tooling
plan: 10
subsystem: infra
tags: [rust, mermaid, observability, execution-overlay, tdd]

# Dependency graph
requires:
  - phase: 28-observability-tooling (plan 07)
    provides: GraphShape/ShapeNode/ShapeEdge/ShapeKind and the shared to_mermaid renderer this plan extends, plus the export_golden.rs bless idiom and golden corpus this plan adds to
provides:
  - "ExecutionOverlay, Visit, OverlaySource (crates/paladin-battalion/src/engine/export/overlay.rs) -- execution history (visits, fired/evaluated edges, source, observed_only) layered onto a GraphShape, built from either Waypoint history (ExecutionOverlay::from_waypoints, derived fired edges) or a persisted trace record stream (ExecutionOverlay::from_trace_records, exact fired AND evaluated edges)"
  - "to_mermaid_overlay (crates/paladin-battalion/src/engine/export/mermaid.rs) -- renders a GraphShape + ExecutionOverlay as an annotated Mermaid flowchart: outcome classDef coloring by each node's last visit, ×N visit-count badges, duration/token cost figures, bold fired / dotted evaluated-not-fired edges, and the locked observed-only title when observed_only is true"
  - "GraphShape::observed (crates/paladin-battalion/src/engine/export/shape.rs) -- builds the observed-only fallback shape from an ExecutionOverlay's visited nodes and observed edges when no static graph document is available (D-22)"
  - "Two committed overlay goldens (crates/paladin-battalion/tests/golden/export/branch_overlay_{waypoints,trace}.mermaid) over a scripted branch_join run, proving the derived-vs-exact evaluated_edges difference byte-for-byte"
affects: [28-13-cli-graph-export, 28-14-run-inspector-service-and-view, 28-15-graph-inspector]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "ExecutionOverlay's fired-edge derivation from Waypoint history reads BOTH completed and vanguard off the SAME Waypoint object (Waypoint::vanguard already means \"dispatched at superstep n+1\" per engine::superstep::build_waypoint's next_vanguard argument) -- never crosses into a different Waypoint's vanguard field, which would silently derive a wrong (off-by-one) edge set"
    - "Two-commit TDD split within one auto/tdd=true task: a RED commit adding failing <behavior> tests against deliberately-wrong stub implementations (from_trace_records/GraphShape::observed always returning empty), then a GREEN commit implementing the real bodies -- kept the tracer task (Task 1) and the TDD task (Task 2) as cleanly separable, auditable diffs despite touching the same files"
    - "GraphShape::observed reuses ShapeKind::Paladin as the 'kind unknown' badge rather than adding a 6th ShapeKind variant, because a new variant would force a matching exhaustive-match update in dot.rs -- which this plan (D-21's Mermaid-only overlay) deliberately does not touch"

key-files:
  created:
    - crates/paladin-battalion/src/engine/export/overlay.rs
    - crates/paladin-battalion/tests/golden/export/branch_overlay_waypoints.mermaid
    - crates/paladin-battalion/tests/golden/export/branch_overlay_trace.mermaid
  modified:
    - crates/paladin-battalion/src/engine/export/mermaid.rs
    - crates/paladin-battalion/src/engine/export/shape.rs
    - crates/paladin-battalion/src/engine/export/mod.rs
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/tests/export_golden.rs

key-decisions:
  - "Waypoint-sourced fired-edge derivation pairs each Waypoint's own completed x vanguard (not a cross-waypoint completed[n] x vanguard[n+1] pairing) -- confirmed against engine::superstep::build_waypoint's actual call sites, where the SAME build_waypoint call passes both this superstep's completed_records and the freshly-computed next_vanguard as this Waypoint's own vanguard field."
  - "Outcome-to-classDef/glyph mapping treats NodeOutcomeKind::Ended the same as Succeeded (both merged their StateDelta normally; Ended's only difference is also ending the run) and gives cache_hit: true visits the outcomeCacheHit class regardless of the underlying outcome variant, since a cache hit is always Succeeded-shaped in practice but the cache flag is the more specific signal worth coloring by."
  - "The observed-only diagram title is emitted via Mermaid's '---\\ntitle: ...\\n---' frontmatter directive (not a bare text line, which would break Mermaid parsing, and not a '%%' comment, which would not render as a visible title) -- a genuinely renderable Mermaid title, not just a string that happens to contain the locked copy."
  - "The overlay golden's scripted run reuses the branch_join fixture (28-07) rather than inventing a new fixture file, matching 28-CONTEXT D-20's 'the overlay golden is the branching fixture with a scripted run.'"

requirements-completed: [OBS-03]

coverage:
  - id: D1
    description: "ExecutionOverlay::from_waypoints derives fired_edges from Waypoint history alone (evaluated_edges always empty, source Waypoints), reading each Waypoint's own completed x vanguard pair"
    requirement: "OBS-03"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/export/overlay.rs#overlay_from_waypoints_derives_fired_edges"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/export/overlay.rs#overlay_visits_are_ordered_per_node"
        status: pass
    human_judgment: false
  - id: D2
    description: "to_mermaid_overlay colors nodes by last-visit outcome, badges repeat visits, appends cost figures, and renders fired edges bold / evaluated-not-fired edges dotted"
    requirement: "OBS-03"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/export/overlay.rs#overlay_mermaid_colours_by_last_outcome"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/export/overlay.rs#overlay_mermaid_annotates_repeat_visits_and_cost"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/export/overlay.rs#overlay_mermaid_renders_fired_edges_bold"
        status: pass
    human_judgment: false
  - id: D3
    description: "ExecutionOverlay::from_trace_records reads exact fired_edges AND evaluated_edges directly from TraceEvent::EdgeEvaluated, with no derivation"
    requirement: "OBS-03"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/export/overlay.rs#overlay_from_trace_is_exact"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/export/overlay.rs#trace_overlay_shows_evaluated_but_not_fired"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/export/overlay.rs#waypoint_and_trace_overlays_of_the_same_run_differ_only_in_evaluated_edges"
        status: pass
    human_judgment: false
  - id: D4
    description: "GraphShape::observed builds the observed-only fallback shape from an overlay's visited nodes/edges, and to_mermaid_overlay renders the locked observed-only title when observed_only is true"
    requirement: "OBS-03"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/export/overlay.rs#observed_shape_is_built_when_no_graph_is_available"
        status: pass
    human_judgment: false
  - id: D5
    description: "An overlay golden for the branching fixture with a scripted run is committed in both source modes (Waypoints and trace), and the two goldens differ exactly where D-21 says they must (evaluated-but-not-fired rendering)"
    requirement: "OBS-03"
    verification:
      - kind: integration
        ref: "crates/paladin-battalion/tests/export_golden.rs#overlay_goldens"
        status: pass
    human_judgment: false

duration: 21min
completed: 2026-09-09
status: complete
---

# Phase 28 Plan 10: Graph Execution Overlay Summary

**`ExecutionOverlay` answering "which branch fired, and why did node X run 3 times" from either Waypoint history (always available, derived edges) or a persisted trace (exact edges), rendered as an annotated `to_mermaid_overlay` diagram with an observed-only fallback and two byte-frozen goldens.**

## Performance

- **Duration:** ~21 min
- **Started:** 2026-09-09T01:51:36Z (Task 1 commit)
- **Completed:** 2026-09-09T02:05:04Z (Task 2 GREEN commit)
- **Tasks:** 2 (Task 2 split RED/GREEN per its `tdd="true"` attribute -- 3 commits total)
- **Files modified:** 8 (1 new module file, 4 modified export/engine files, 1 modified integration test, 2 new golden files)

## Accomplishments
- `ExecutionOverlay`/`Visit`/`OverlaySource` (`crates/paladin-battalion/src/engine/export/overlay.rs`): ordered (`BTreeMap`/`BTreeSet`, never `HashMap`/`HashSet`) execution history layered onto a `GraphShape` -- `visits` per node in ascending-superstep order, `fired_edges`/`evaluated_edges`, and a `source` flag so a derived overlay is never mistaken for an exact one (T-28-10-02).
- `ExecutionOverlay::from_waypoints(&[Waypoint], &GraphShape)`: always-available source built from Waypoint history alone. Fired edges are DERIVED by pairing each Waypoint's OWN `completed` set with its OWN `vanguard` field -- confirmed against `engine::superstep::build_waypoint`'s actual call sites that `Waypoint::vanguard` already means "dispatched at superstep n+1", so no cross-waypoint pairing is needed (an initial cross-waypoint implementation was caught and fixed by the tracer's own `<behavior>` tests during Task 1, see Deviations). `evaluated_edges` stays empty; `waypoints` is sorted by ascending `superstep` internally so a caller handing over `WaypointPort::history`'s newest-first order still derives correctly.
- `ExecutionOverlay::from_trace_records(&[TraceRecord])`: the exact upgrade. Visits come from `TraceEvent::NodeFinished` (one per attempt); both `fired_edges` and `evaluated_edges` are read directly from `TraceEvent::EdgeEvaluated` with no derivation.
- `to_mermaid_overlay(&GraphShape, &ExecutionOverlay)` (`mermaid.rs`): reuses the static renderer's node/edge emission structure, layering on an outcome `classDef` block (five outcomes, exact light-mode hex values from `28-UI-SPEC.md`'s frozen "Outcome color set" table), a per-node class from that node's LAST visit's outcome, a `×N` visit-count badge for any node visited more than once, `<duration>ms · <tokens>tok` cost figures on every visited node's label, a bold link style (`==>`) for fired edges and a dotted one (`-.->`) for evaluated-but-not-fired edges, and -- when `overlay.observed_only` is `true` -- the locked observed-only title `(observed nodes only — no graph document available)` via Mermaid's `---\ntitle: ...\n---` frontmatter directive.
- `GraphShape::observed(&ExecutionOverlay)` (`shape.rs`, D-22): builds the fallback shape from only the visited node ids and the union of `fired_edges`/`evaluated_edges` when no static graph document is available for a thread, with no entry points.
- Module rustdoc records the deliberate DOT-overlay omission (`overlay.rs` "No DOT overlay" section) -- the PRD names Mermaid for the overlay only, so a future reader does not treat the gap as an oversight.
- Two committed overlay goldens (`tests/golden/export/branch_overlay_{waypoints,trace}.mermaid`) over one scripted run on the `branch_join` fixture (`split` routes to `branch_a`, `branch_b` never fires, `join` completes): the two goldens are byte-identical except for exactly one line -- `split -> branch_b` renders plain (`-->`) in the Waypoints golden and dotted (`-.->`) in the trace golden -- making the derived-vs-exact difference visible in the committed corpus itself, per D-20/D-21.
- `ExecutionOverlay`, `OverlaySource`, `Visit`, and `to_mermaid_overlay` re-exported at both the `engine::export` and `engine` module levels, matching the existing `GraphShape`/`to_mermaid` precedent (needed for `engine/mod.rs`'s own module-doc intra-doc links to resolve -- see Deviations).

## Task Commits

Each task was committed atomically (Task 2's `tdd="true"` attribute produced a RED/GREEN pair):

1. **Task 1: One executed branch, annotated -- `ExecutionOverlay` from Waypoints and its Mermaid rendering** (tracer) - `980b7e99` (feat)
2. **Task 2 RED: failing tests for the trace-exact overlay and observed-only shape** - `fb44a5f8` (test)
3. **Task 2 GREEN: exact trace-sourced overlay, observed-only fallback, and overlay goldens** - `175580f6` (feat)

**Plan metadata:** (this commit, following this SUMMARY)

## Files Created/Modified
- `crates/paladin-battalion/src/engine/export/overlay.rs` - `ExecutionOverlay`, `Visit`, `OverlaySource`, `from_waypoints`, `from_trace_records`, 9 unit tests
- `crates/paladin-battalion/src/engine/export/mermaid.rs` - `to_mermaid_overlay`: overlay-aware node/edge rendering, outcome `classDef`s, observed-only title frontmatter
- `crates/paladin-battalion/src/engine/export/shape.rs` - `GraphShape::observed`
- `crates/paladin-battalion/src/engine/export/mod.rs` - module doc update, `pub mod overlay;`, re-exports (`ExecutionOverlay`, `OverlaySource`, `Visit`, `to_mermaid_overlay`)
- `crates/paladin-battalion/src/engine/mod.rs` - re-exports the same four items at the `engine` module level (matching the existing `GraphShape`/`to_mermaid` precedent)
- `crates/paladin-battalion/tests/export_golden.rs` - scripted `branch_join` Waypoint history + trace record stream builders, `overlay_goldens` integration test
- `crates/paladin-battalion/tests/golden/export/branch_overlay_waypoints.mermaid`, `branch_overlay_trace.mermaid` - the two committed overlay goldens

## Decisions Made
- Fired-edge derivation from Waypoint history pairs each Waypoint's OWN `completed` x `vanguard` (not `completed[n]` x `vanguard[n+1]` across two Waypoint objects) -- verified against `engine::superstep::build_waypoint`'s call sites, where `next_vanguard` (the dispatch set for superstep n+1) is passed as the SAME Waypoint's `vanguard` field alongside that superstep's own `completed_records`.
- `NodeOutcomeKind::Ended` colors/glyphs the same as `Succeeded` (both merged normally; `Ended` differs only in also completing the run); a `cache_hit: true` visit always colors `outcomeCacheHit` regardless of its `outcome` variant, since the cache flag is the more specific signal.
- `GraphShape::observed`'s nodes render as `ShapeKind::Paladin` (the existing least-assuming badge) rather than adding a 6th `ShapeKind` variant for "unknown" -- a new variant would force a matching exhaustive-match update in `dot.rs`, outside this plan's Mermaid-only overlay scope (D-21).
- The observed-only title uses Mermaid's `---\ntitle: ...\n---` frontmatter directive, a genuinely renderable Mermaid title rather than a bare text line (which would break parsing) or a `%%` comment (which would not display as a title).
- The overlay golden's scripted run reuses the `branch_join` fixture from 28-07 rather than a new fixture file, per D-20's "the overlay golden is the branching fixture with a scripted run."

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed fired-edge derivation crossing into the wrong Waypoint's `vanguard` field**
- **Found during:** Task 1, running the tracer's own `<behavior>` test `overlay_from_waypoints_derives_fired_edges`
- **Issue:** The first implementation derived fired edges by pairing waypoint `N`'s `completed` set with waypoint `N+1`'s `vanguard` field (a literal reading of the plan's action text, "each Waypoint's completed node set and the NEXT Waypoint's vanguard"). Tracing `engine::superstep::build_waypoint`'s actual call sites showed `Waypoint::vanguard`'s own contract ("nodes ready for the NEXT superstep") is already the dispatch set for `n+1`, stored on the SAME Waypoint as `completed` for superstep `n` -- so the cross-waypoint pairing was off by one waypoint and produced zero fired edges on the tracer's own test fixture.
- **Fix:** Changed the derivation to pair each Waypoint's own `completed` x its own `vanguard`, documented with the `build_waypoint` call-site evidence in the method's rustdoc.
- **Files modified:** `crates/paladin-battalion/src/engine/export/overlay.rs`
- **Verification:** All 5 Task 1 tests pass; re-confirmed by Task 2's `waypoint_and_trace_overlays_of_the_same_run_differ_only_in_evaluated_edges` test, which asserts the Waypoints-derived and trace-exact `fired_edges` sets are IDENTICAL for the same scripted run.
- **Committed in:** `980b7e99` (Task 1 commit -- fixed before the tracer commit landed, not a follow-up fix)

**2. [Rule 3 - Blocking] Re-exported the new overlay types at the `engine` module level**
- **Found during:** Task 1, `cargo doc -p paladin-battalion --no-deps`
- **Issue:** `crates/paladin-battalion/src/engine/mod.rs` re-exports `export`'s public surface (`pub use export::{GraphShape, ShapeEdge, ShapeKind, ShapeNode, to_dot, to_mermaid};`) but the plan's action text only specified changes to `export/mod.rs`. Without the same re-export at the `engine` level, `export/mod.rs`'s own module-doc intra-doc links to `[`ExecutionOverlay`]`/`[`to_mermaid_overlay`]` failed to resolve (`cargo doc` "unresolved link ... no item named X in scope"), blocking the plan's own `cargo doc -p paladin-battalion --no-deps` (no new warnings) verification requirement.
- **Fix:** Added `ExecutionOverlay, OverlaySource, Visit, to_mermaid_overlay` to `engine/mod.rs`'s existing `pub use export::{...}` line, matching the established `GraphShape`/`to_mermaid` precedent.
- **Files modified:** `crates/paladin-battalion/src/engine/mod.rs`
- **Verification:** `cargo doc -p paladin-battalion --no-deps 2>&1 | grep -i "export\|overlay"` returns no output (zero warnings).
- **Committed in:** `980b7e99` (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (1 bug, 1 blocking)
**Impact on plan:** Both fixes were necessary for correctness (Rule 1) and to satisfy the plan's own doc-warning verification bar (Rule 3). No scope creep -- neither touched `dot.rs` or any file outside this plan's declared `files_modified` list, and both were caught and fixed WITHIN Task 1's own commit before it landed (not as later patches).

## Issues Encountered
None beyond the two auto-fixed deviations above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- `ExecutionOverlay`, `to_mermaid_overlay`, and `GraphShape::observed` are ready for 28-13's `run export` CLI command and 28-14/28-15's inspector page to build on directly -- both are named in `to_mermaid_overlay`'s own rustdoc as the next consumers.
- 28-13's graph resolution order (D-22: `--graph` flag -> the run's assistant version's `WarGraphDoc` -> `observed_only = true`) can call `GraphShape::observed` directly for its third bucket; nothing in this plan needs revisiting for that wiring.
- No blockers.

## Self-Check: PASSED

- FOUND: crates/paladin-battalion/src/engine/export/overlay.rs
- FOUND: crates/paladin-battalion/tests/golden/export/branch_overlay_waypoints.mermaid
- FOUND: crates/paladin-battalion/tests/golden/export/branch_overlay_trace.mermaid
- FOUND: commit 980b7e99
- FOUND: commit fb44a5f8
- FOUND: commit 175580f6

## Verification Commands Run (all green)

- `cargo test -p paladin-battalion --lib engine::export` -- 18 passed
- `cargo test -p paladin-battalion --test export_golden` -- 4 passed
- `cargo test -p paladin-battalion --doc engine::export` -- 4 passed
- `cargo fmt --all --check` -- exit 0
- `cargo check --workspace --all-targets --all-features` -- exit 0
- `cargo clippy -p paladin-battalion --all-targets --all-features -- -D warnings` -- exit 0
- `cargo clippy --workspace --all-targets -- -D warnings` -- exit 0
- `cargo doc -p paladin-battalion --no-deps` -- zero warnings mention `export`/`overlay` (pre-existing unrelated warnings in `input_mapping.rs`/`directive_parser.rs`/`mod.rs` untouched)
- `cargo tree -p paladin-battalion --no-default-features -e normal | grep -c -E "paladin-llm|paladin-storage|paladin-ai "` -- 0 (ADR-0031 upheld)

---
*Phase: 28-observability-tooling*
*Completed: 2026-09-09*
