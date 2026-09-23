---
phase: 28-observability-tooling
plan: 07
subsystem: infra
tags: [rust, mermaid, graphviz, wargraph, observability]

# Dependency graph
requires:
  - phase: 28-observability-tooling (plan 03)
    provides: WarGraphDoc/WarGraph node and edge model this plan reads (NodeSpec, EdgeSpec, EdgeCondition, WarGraph::is_deferred/is_worker_template)
provides:
  - "GraphShape, ShapeNode, ShapeEdge, ShapeKind (crates/paladin-battalion/src/engine/export/shape.rs) -- the rendering-agnostic shape both a WarGraphDoc and a compiled WarGraph produce"
  - "to_mermaid and to_dot (crates/paladin-battalion/src/engine/export/{mermaid,dot}.rs) -- pure, byte-deterministic Mermaid/DOT renderers over GraphShape"
  - "Ten committed golden fixtures (linear/branch_join/loop/muster/subgraph x mermaid/dot) under crates/paladin-battalion/tests/golden/export/"
  - "make bless-golden target regenerating the goldens"
affects: [28-10-graph-execution-overlay, 28-13-cli-graph-export, 28-15-graph-inspector]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "GraphShape as the shared rendering contract over two sources (WarGraphDoc via from_doc, compiled WarGraph via from_graph), so a doc-only exporter's kind gap (no Function nodes, no worker templates, 27-CONTEXT D-33) doesn't limit what the exporters can render"
    - "Pre-order n{i} node-id sanitization with a global counter threaded through recursive subgraph rendering, keeping Mermaid/DOT diagram ids unique across nested Workflow clusters while author-supplied real ids only ever appear inside an escaped label"
    - "UPDATE_GOLDEN=1 bless idiom mirroring UPDATE_WARGRAPH_SCHEMA=1 (graph_doc_round_trip.rs) and UPDATE_OPENAPI=1 (paladin-web), plus a matching make bless-golden target"

key-files:
  created:
    - crates/paladin-battalion/src/engine/export/mod.rs
    - crates/paladin-battalion/src/engine/export/shape.rs
    - crates/paladin-battalion/src/engine/export/mermaid.rs
    - crates/paladin-battalion/src/engine/export/dot.rs
    - crates/paladin-battalion/tests/export_golden.rs
    - crates/paladin-battalion/tests/fixtures/graph_docs/linear.json
    - crates/paladin-battalion/tests/fixtures/graph_docs/branch_join.json
    - crates/paladin-battalion/tests/fixtures/graph_docs/loop.json
    - crates/paladin-battalion/tests/golden/export/{linear,branch_join,loop,muster,subgraph}.{mermaid,dot}
  modified:
    - crates/paladin-battalion/src/engine/mod.rs
    - Makefile

key-decisions:
  - "ShapeKind::WorkerTemplate overrides whichever NodeSpec variant a worker-template node wraps (Paladin or Function), so its badge always reads «worker» -- while ShapeNode::worker_template stays an independent boolean driving dashed styling, matching D-18's literal type shape (both a 5th ShapeKind value AND a separate flag)."
  - "Edge condition labels: contains(\"value\") shows the match value, regex renders bare (the pattern itself is template-shaped content this module deliberately does not carry, T-28-07-02), custom(name) shows the registered evaluator name -- per D-19's exact three quoted forms."
  - "Both Mermaid and DOT sanitize the DIAGRAM's own node identifiers to n{i} (not just Mermaid) -- the real id appears only inside an escaped, quoted label -- for uniform T-28-07-01 tampering mitigation across both formats."
  - "classDef fill/stroke/color values for the five ShapeKind classes are Claude's Discretion (28-UI-SPEC.md explicitly defers this to the golden files); the goldens now freeze the choice."

requirements-completed: [OBS-03]

coverage:
  - id: D1
    description: "GraphShape::from_doc and GraphShape::from_graph produce an equal shape for the same doc-expressible graph"
    requirement: "OBS-03"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/export/shape.rs#linear_doc_and_graph_produce_the_same_shape"
        status: pass
      - kind: integration
        ref: "crates/paladin-battalion/tests/export_golden.rs#doc_and_graph_shapes_agree"
        status: pass
    human_judgment: false
  - id: D2
    description: "to_mermaid and to_dot render a graph deterministically per D-19's rules (flowchart TD / digraph, n{i} ids, guillemet badges, Gate diamond, Workflow cluster, dashed worker-template/deferred styling, conditional edge labels)"
    requirement: "OBS-03"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/export/shape.rs#linear_mermaid_is_deterministic"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/export/shape.rs#linear_dot_is_deterministic"
        status: pass
      - kind: integration
        ref: "crates/paladin-battalion/tests/export_golden.rs#golden_exports"
        status: pass
    human_judgment: false
  - id: D3
    description: "A code-built muster graph (worker template + deferred aggregator, neither expressible in a WarGraphDoc) renders with dashed worker-template styling"
    requirement: "OBS-03"
    verification:
      - kind: integration
        ref: "crates/paladin-battalion/tests/export_golden.rs#muster_renders_worker_template_and_deferred_node"
        status: pass
    human_judgment: false
  - id: D4
    description: "A one-node, zero-edge shape renders a valid diagram with exactly that node and no edge lines (the FLAGGED ASSUMPTION in must_haves)"
    requirement: "OBS-03"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/export/shape.rs#single_node_zero_edge_shape_renders"
        status: pass
    human_judgment: false

duration: 25min
completed: 2026-09-09
status: complete
---

# Phase 28 Plan 07: Graph Export (GraphShape, Mermaid, DOT) Summary

**`GraphShape` built from either a `WarGraphDoc` or a compiled `WarGraph`, rendered by two pure, byte-deterministic exporters (`to_mermaid`, `to_dot`) and frozen by ten golden files over five fixtures.**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-09-09T00:32:17Z (worktree base commit)
- **Completed:** 2026-09-09T00:51:22Z (final task commit)
- **Tasks:** 2
- **Files modified:** 17 (4 new export module files, 1 modified engine/mod.rs, 1 new integration test, 3 new fixture JSON files, 10 new golden files, 1 modified Makefile)

## Accomplishments
- `GraphShape`/`ShapeNode`/`ShapeEdge`/`ShapeKind` (`crates/paladin-battalion/src/engine/export/shape.rs`): the shared rendering contract built from EITHER a `WarGraphDoc` (`from_doc`) or a compiled `WarGraph` (`from_graph`), so `Function` nodes and Muster worker templates -- absent from the v0.10 document boundary (27-CONTEXT D-33) -- still render.
- `to_mermaid` (`flowchart TD`) and `to_dot` (`digraph`) render the shape per D-19: `n{i}`-sanitized diagram ids in declaration order (pre-order across nested Workflow clusters, sharing one global counter), guillemet kind badges, Gate diamond, Workflow nested subgraph/cluster, dashed worker-template/deferred styling, and conditional edge labels (`contains("value")`, `regex`, `custom(name)`).
- Ten golden fixtures under `crates/paladin-battalion/tests/golden/export/` -- linear, branch+join, loop, muster (code-built), subgraph, each in both formats -- generated once via `UPDATE_GOLDEN=1`, inspected by eye against D-19/28-UI-SPEC.md, and proven byte-equal on a clean re-run.
- `make bless-golden` target for one-command golden regeneration, listed in `make help`.
- Wired `pub mod export` and its public re-exports into `engine/mod.rs`; `crates/paladin-battalion/src/maneuver/visualizer.rs` (`FlowVisualizer`, `flowchart LR`) is untouched (X-03, verified via `git diff --exit-code`).

## Task Commits

Each task was committed atomically:

1. **Task 1: One linear graph, two formats -- `GraphShape` and the thinnest render path** - `90083ad4` (feat)
2. **Task 2: Five golden fixtures in two formats, with the bless idiom and a Makefile target** - `52f5d054` (test)

**Plan metadata:** (this commit, following this SUMMARY)

## Files Created/Modified
- `crates/paladin-battalion/src/engine/export/mod.rs` - Module docs, re-exports (`GraphShape`, `ShapeNode`, `ShapeEdge`, `ShapeKind`, `to_mermaid`, `to_dot`), doctest proving the full pipeline
- `crates/paladin-battalion/src/engine/export/shape.rs` - `GraphShape`/`ShapeNode`/`ShapeEdge`/`ShapeKind`, `from_doc`/`from_graph`, condition-label helpers, 5 unit tests
- `crates/paladin-battalion/src/engine/export/mermaid.rs` - `to_mermaid`: flowchart rendering, class/dashed styling, escaping
- `crates/paladin-battalion/src/engine/export/dot.rs` - `to_dot`: digraph rendering, cluster/diamond/dashed styling, escaping
- `crates/paladin-battalion/src/engine/mod.rs` - `pub mod export;` declaration and re-exports at the engine module level
- `crates/paladin-battalion/tests/export_golden.rs` - Golden comparison (`UPDATE_GOLDEN=1` bless), `doc_and_graph_shapes_agree`, code-built muster fixture, `muster_renders_worker_template_and_deferred_node`
- `crates/paladin-battalion/tests/fixtures/graph_docs/linear.json` - New 3-node linear paladin-chain fixture
- `crates/paladin-battalion/tests/fixtures/graph_docs/branch_join.json` - New fan-out/fan-in fixture with `Contains` conditions
- `crates/paladin-battalion/tests/fixtures/graph_docs/loop.json` - New Gate-mediated loop fixture (mirrors `approval_gate.json`'s shape under different node names)
- `crates/paladin-battalion/tests/golden/export/*.{mermaid,dot}` - Ten committed golden files (`nested_workflow.json` reused as the `subgraph` fixture)
- `Makefile` - New `bless-golden` target beside `openapi`, listed in `make help`

## Decisions Made
- `ShapeKind::WorkerTemplate` overrides the underlying `NodeSpec` variant's kind for a worker-template node (badge always `«worker»`), while `ShapeNode::worker_template` stays an independent boolean -- reconciling D-18's literal type shape (a 5th `ShapeKind` value AND a separate flag) rather than treating them as redundant.
- Edge condition labels follow D-19's three literal quoted forms exactly: `contains("value")` (shows the match value), `regex` (bare -- the pattern itself is deliberately not carried, T-28-07-02), `custom(name)` (shows the evaluator name).
- Both Mermaid AND DOT sanitize their own diagram node identifiers to `n{i}` (not just Mermaid) for uniform T-28-07-01 mitigation; the real id is only ever inside an escaped, quoted label in both formats.
- `classDef` fill/stroke/color values for the five `ShapeKind` classes were Claude's Discretion per 28-UI-SPEC.md ("golden files freeze whatever is chosen") -- now locked by the committed goldens.
- `loop.json` intentionally mirrors `approval_gate.json`'s writer/review shape under different node names (`drafter`/`critique`) -- a new, distinct fixture file per the plan's action text, not a structural departure.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- `GraphShape`, `to_mermaid`, and `to_dot` are ready for 28-10 (execution overlay, `to_mermaid_overlay` extends this same shape), 28-13 (CLI `graph export` command), and 28-15 (inspector page) to render through directly.
- No blockers. `crates/paladin-battalion/src/engine/export/overlay.rs` (28-10) is the next file in this module; nothing in this plan needs revisiting for it to land.

## Self-Check: PASSED

- FOUND: crates/paladin-battalion/src/engine/export/mod.rs
- FOUND: crates/paladin-battalion/src/engine/export/shape.rs
- FOUND: crates/paladin-battalion/src/engine/export/mermaid.rs
- FOUND: crates/paladin-battalion/src/engine/export/dot.rs
- FOUND: crates/paladin-battalion/tests/export_golden.rs
- FOUND: crates/paladin-battalion/tests/fixtures/graph_docs/linear.json
- FOUND: crates/paladin-battalion/tests/fixtures/graph_docs/branch_join.json
- FOUND: crates/paladin-battalion/tests/fixtures/graph_docs/loop.json
- FOUND: crates/paladin-battalion/tests/golden/export/linear.mermaid (+ 9 sibling golden files)
- FOUND: commit 90083ad4
- FOUND: commit 52f5d054

## Verification Commands Run (all green)

- `cargo test -p paladin-battalion --lib engine::export` -- 9 passed
- `cargo test -p paladin-battalion --doc engine::export` -- 4 passed
- `cargo test -p paladin-battalion --test export_golden` -- 3 passed
- `cargo test -p paladin-battalion --test graph_doc_round_trip` -- 5 passed (new fixtures don't break the existing corpus round-trip)
- `cargo fmt --all --check` -- exit 0
- `cargo check --workspace --all-targets --all-features` -- exit 0
- `cargo clippy --all-targets --all-features -p paladin-battalion -- -D warnings` -- exit 0
- `cargo doc -p paladin-battalion --no-deps` -- 36 pre-existing warnings in unrelated files (`input_mapping.rs`, `mod.rs` intra-doc links); zero warnings mention `export`
- `make bless-golden` -- exit 0, goldens unchanged on the following clean-run comparison

---
*Phase: 28-observability-tooling*
*Completed: 2026-09-09*
