---
phase: 35-mdbook-currency
plan: 01
subsystem: docs
tags: [mdbook, doc-examples, waypoint, war-engine, battlefield, requirements]

# Dependency graph
requires:
  - phase: 34-documentation-currency-audit
    provides: "34-AUDIT.md §5's 60-row MB-nn work list, including MB-30 (the missing superstep-engine page) and its assigned nav position"
provides:
  - "Minted requirement IDs CURR-06..CURR-10 in REQUIREMENTS.md and the ROADMAP Phase 35 Requirements line"
  - "The new WarEngine superstep-engine guide at docs/src/user-guides/superstep-engine.md, nav-linked in docs/src/SUMMARY.md"
  - "crates/doc-examples/src/superstep_engine.rs — four compile-verified anchors (build_graph, configure_limits, run_engine, inspect_waypoints) registered in lib.rs"
  - "The Phase 35 phase-local deferred register (deferred-items.md) with the contributing-providers.md observation and the standing EX-nn pointer rule"
affects: [35-03, 35-04, "any later Phase 35 plan whose page links to user-guides/superstep-engine.md"]

# Tech tracking
tech-stack:
  added: []
  patterns: ["doc-examples anchor pattern for a new user-guide page (build -> configure -> run -> inspect, mirroring fault_tolerance.rs)"]

key-files:
  created:
    - docs/src/user-guides/superstep-engine.md
    - crates/doc-examples/src/superstep_engine.rs
    - .planning/phases/35-mdbook-currency/deferred-items.md
    - .planning/phases/35-mdbook-currency/35-01-SUMMARY.md
  modified:
    - .planning/REQUIREMENTS.md
    - .planning/ROADMAP.md
    - crates/doc-examples/src/lib.rs
    - docs/src/SUMMARY.md

key-decisions:
  - "The build_graph anchor uses a self-looping Function node (count/status Battlefield fields, Contains(\"looping\") edge condition) rather than Paladin nodes with a mock LLM port, mirroring the in-tree self_loop_graph test pattern in engine/superstep.rs — deterministic termination without needing to reason about mock-port output content for routing."
  - "configure_limits builds an EngineConfig (all five named fields) and converts via impl From<EngineConfig> for EngineLimits, per D-08's naming requirement — EngineLimits itself has no waypoint_durability or run_timeout_secs field, only run_timeout: Option<Duration>, so the app-facing EngineConfig is the vehicle that names all five knobs literally."
  - "Every {{#include}} block on the page is fenced rust,ignore, not bare rust — matching the existing house convention (fault-tolerance.md does the same) so mdBook never attempts its own doctest of a code fragment; compile verification is entirely check-doc-examples.sh's Layer 1 (cargo check -p paladin-doc-examples), not mdBook's own test runner."

requirements-completed: [CURR-06, CURR-07, CURR-08]

coverage:
  - id: D1
    description: "CURR-06..CURR-10 minted in REQUIREMENTS.md (5 new entries + traceability rows + coverage counts) and the ROADMAP Phase 35 Requirements line replaced"
    requirement: "CURR-06"
    verification:
      - kind: other
        ref: "grep -cE '^- \\[ \\] \\*\\*CURR-(06|07|08|09|10)\\*\\*' .planning/REQUIREMENTS.md == 5"
        status: pass
      - kind: other
        ref: "grep -q 'Requirements**: CURR-06, CURR-07, CURR-08, CURR-09, CURR-10' .planning/ROADMAP.md"
        status: pass
    human_judgment: false
  - id: D2
    description: "New WarEngine superstep-engine guide exists at the D-07 path/nav position, with the Since marker, and covers the full D-08 scope"
    requirement: "CURR-06"
    verification:
      - kind: other
        ref: "awk nav-order check on docs/src/SUMMARY.md (Maneuver Flow DSL < superstep-engine.md < Control Flow)"
        status: pass
      - kind: other
        ref: "grep literals BattlefieldSchema/StateDelta/WaypointId/vanguard/max_supersteps/max_node_visits/run_timeout_secs/waypoint_durability/max_muster_tasks/APP_ENGINE_/RecursionLimitExceeded/WaypointRetentionService/GRAPH_FINGERPRINT_VERSION on docs/src/user-guides/superstep-engine.md — all present"
        status: pass
    human_judgment: false
  - id: D3
    description: "Four compile-verified doc-examples anchors (build_graph, configure_limits, run_engine, inspect_waypoints) back the page, registered in lib.rs"
    requirement: "CURR-08"
    verification:
      - kind: other
        ref: "cargo check -p paladin-doc-examples (via ./scripts/check-doc-examples.sh Layer 1)"
        status: pass
      - kind: other
        ref: "grep -c '^// ANCHOR: ' crates/doc-examples/src/superstep_engine.rs == 4"
        status: pass
    human_judgment: false
  - id: D4
    description: "Full docs.yml gate sequence green on the plan's final commit (mdbook-mermaid install with no drift, mdbook build + linkcheck, check-doc-examples.sh, check-doc-config.sh)"
    requirement: "CURR-07"
    verification:
      - kind: other
        ref: "mdbook build docs/ — 'No broken links found'"
        status: pass
      - kind: other
        ref: "./scripts/check-doc-examples.sh — 0 checked/620 skipped/0 failed"
        status: pass
      - kind: other
        ref: "./scripts/check-doc-config.sh — 154 YAML blocks checked, 0 failed"
        status: pass
      - kind: other
        ref: "git status --porcelain -- docs (after mdbook-mermaid install docs/) — empty"
        status: pass
    human_judgment: false
  - id: D5
    description: "Phase-local deferred register seeded with the contributing-providers.md observation (D-27) and the standing D-26 EX-nn pointer rule"
    verification:
      - kind: other
        ref: "grep contributing-providers.md/D-26/D-27/272/367/§2 row 52/Phase 36 on deferred-items.md"
        status: pass
    human_judgment: false

# Metrics
duration: 15min
completed: 2026-09-17
status: complete
---

# Phase 35 Plan 01: Mint CURR-06..CURR-10 and Close MB-30 Summary

**Minted five Phase 35 requirement IDs and closed MB-30 (the missing Phase 22 superstep-engine
guide) with a new `docs/src/user-guides/superstep-engine.md` page backed by four compile-verified
`doc-examples` anchors, unblocking the four dependent pages plans 35-03/35-04 link to it.**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-09-17T13:18:00Z
- **Completed:** 2026-09-17T13:28:50Z
- **Tasks:** 3
- **Files modified:** 8 (4 created, 4 modified)

## Accomplishments

- Minted `CURR-06`, `CURR-07`, `CURR-08`, `CURR-09`, `CURR-10` in `.planning/REQUIREMENTS.md` (new
  entries, traceability rows, coverage counts 71→76, dated `*Extended:*` footer line) and replaced
  the ROADMAP Phase 35 `**Requirements**: TBD` line with the minted list.
- Closed MB-30 end to end: a new `docs/src/user-guides/superstep-engine.md` page at the D-07 path,
  nav-inserted in `docs/src/SUMMARY.md` immediately after `Maneuver Flow DSL` and before
  `Control Flow`, carrying the `**Since:** v0.10.0 (Phase 22)` marker.
- Wrote `crates/doc-examples/src/superstep_engine.rs` — a wholly new module (D-26: additive only)
  with four `// ANCHOR:` regions (`build_graph`, `configure_limits`, `run_engine`,
  `inspect_waypoints`) forming one small program: build a small cyclic `WarGraph`, configure
  `EngineLimits`/`WaypointDurability` via the app-facing `EngineConfig`, run it over an
  `InMemoryWaypointStore`, and read the resulting Waypoint back out — registered as
  `pub mod superstep_engine;` in `lib.rs`.
- Page covers the full D-08 scope: `WarGraph`/`Battlefield` superstep merge semantics, `Waypoint`
  checkpointing and `(ThreadId, WaypointId)` addressing (including the `vanguard: Vec<NodeId>`
  field), the three `WaypointPort` backends, `EngineConfig`/`EngineLimits` with their
  `APP_ENGINE_*` overrides and `EngineError::RecursionLimitExceeded`, `WaypointRetentionService`,
  and the graph-fingerprint scheme (`GRAPH_FINGERPRINT_VERSION = "v6"`) — closing with a "Where to
  Go Next" section linking, never re-explaining, Control Flow, Parley & Chronicle, Aegis and Agent
  Runtime.
- Seeded `.planning/phases/35-mdbook-currency/deferred-items.md` with the
  `contributing-providers.md` relocated-adapter-import observation (D-27, not fixed silently) and
  the standing D-26 rule for Phase 36 `EX-nn` pointers later plans will append under.
- Full `docs.yml` gate sequence verified green on the final commit: `mdbook-mermaid install docs/`
  left `git status --porcelain -- docs` empty, `mdbook build docs/` reported "No broken links
  found", `./scripts/check-doc-examples.sh` reported 0 checked/620 skipped/0 failed, and
  `./scripts/check-doc-config.sh` reported 154 YAML blocks checked/0 failed. `make api-surface`
  confirmed no public-surface drift (`doc-examples` is `publish = false`).

## Task Commits

Each task was committed atomically:

1. **Task 1: Mint CURR-06…CURR-10 and drive one engine-guide slice end to end** —
   `eae9f91d` (docs, requirement minting) + `7cca347d` (docs, MB-30 scaffold: module, lib.rs,
   page, nav)
2. **Task 2: Complete the engine guide to its D-08 scope with three further anchors** —
   `25f0e600` (docs)
3. **Task 3: Seed the phase-local deferred register** — `7f6e9cbb` (docs)

**Formatting fix (not a plan task, `cargo fmt` compliance before self-check):** `3605af20`
(style: reformat two long function signatures `cargo fmt --check` flagged)

_Note: Task 1 is a `type="tracer"` task with two internal commits (requirement minting, then the
end-to-end scaffold) per its own `<action>` instructions._

## Files Created/Modified

- `.planning/REQUIREMENTS.md` - Five new `CURR-0N` entries, traceability rows, coverage counts, footer
- `.planning/ROADMAP.md` - Phase 35 `**Requirements**` line replaced (TBD → minted CURR-06..10)
- `docs/src/user-guides/superstep-engine.md` - New WarEngine guide (created, then completed to full D-08 scope)
- `docs/src/SUMMARY.md` - Nav entry inserted before `Control Flow`
- `crates/doc-examples/src/superstep_engine.rs` - New module: `LoopUntil` StateNode + 4 anchors
- `crates/doc-examples/src/lib.rs` - `pub mod superstep_engine;` registration
- `.planning/phases/35-mdbook-currency/deferred-items.md` - Phase 35 deferred register, seeded

## Decisions Made

- Used a self-looping `Function` node (not a `Paladin` node against the mock port) for the
  `build_graph`/`run_engine` anchors — the mock port's deterministic echo output makes
  content-based edge routing awkward to reason about for a clean 3-iteration termination; the
  `StateNode` trait gives full control over the `Directive` and mirrors an audited in-crate test
  pattern (`engine/superstep.rs`'s `self_loop_graph` + `CountingFunctionNode` shape) exactly.
- `configure_limits` constructs an `EngineConfig` (the app-facing type in `src/config/engine.rs`)
  rather than an `EngineLimits` literal, because D-08 asks the anchor to name all five knobs
  (`max_supersteps`, `max_node_visits`, `run_timeout_secs`, `waypoint_durability`,
  `max_muster_tasks`) and only `EngineConfig` has all five as literal field names —
  `EngineLimits` itself has no `waypoint_durability` field and spells the timeout `run_timeout:
  Option<Duration>`, not `run_timeout_secs`. The page prose and table make this conversion
  explicit rather than leaving it implicit.
- Corrected two `35-CONTEXT.md` canonical-refs file-path assumptions per `35-RESEARCH.md`
  Pitfall 1 (already researched, not re-derived here): `WarEngine`/`WarGraph`/`EngineLimits` live
  in `paladin-battalion::engine`, not `paladin-core`; `WaypointRetentionService` lives in
  `src/application/services/waypoint_retention.rs`, not `paladin-core`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `cargo fmt --check` flagged two function signatures**
- **Found during:** Post-Task-2 quality pass (CLAUDE.md's "before committing a parent task:
  `cargo test` → `cargo fmt --check` → `cargo clippy`" working agreement)
- **Issue:** `configure_limits`'s and `run_engine`'s signatures/bodies did not match `rustfmt`'s
  line-wrap output (two-argument-wrap style rustfmt prefers for long return-type signatures).
- **Fix:** Ran `cargo fmt -p paladin-doc-examples`; re-ran `cargo check`, `./scripts/check-doc-examples.sh`
  and `mdbook build docs/` to confirm the reformat changed nothing semantically.
- **Files modified:** `crates/doc-examples/src/superstep_engine.rs`
- **Verification:** `cargo fmt --check -p paladin-doc-examples` exits 0; all gates re-run green.
- **Committed in:** `3605af20`

---

**Total deviations:** 1 auto-fixed (1 bug/formatting)
**Impact on plan:** Cosmetic only — no behavior or content change. No scope creep.

## Issues Encountered

None. Every API shape used (`WarGraph`, `EngineLimits`, `EngineConfig`, `StateNode`, `Directive`,
`WaypointPort`, `InMemoryWaypointStore`) was verified directly against the live tree before
writing code, and `cargo check -p paladin-doc-examples` compiled clean on the first attempt for
all four anchors.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- MB-30 is closed and `user-guides/superstep-engine.md` is a landed link target: plans 35-03
  (`control-flow.md`, MB-22) and 35-04 (`introduction.md`, `architecture/overview.md`,
  `architecture/domain-model.md`, MB-05/MB-09/MB-11) can now add their forward links to this page
  per D-10 in wave 2.
- CURR-06…CURR-10 exist and are cited by every later Phase 35 plan's frontmatter `requirements`
  field.
- The phase-local `deferred-items.md` register is open for plans 35-02 through 35-09's own
  `## Deferred observations` SUMMARY sections; plan 35-10 folds them in at phase close.
- No blockers for wave 2.

---
*Phase: 35-mdbook-currency*
*Completed: 2026-09-17*

## Self-Check: PASSED

All 8 created/modified files confirmed present on disk (`docs/src/user-guides/superstep-engine.md`,
`crates/doc-examples/src/superstep_engine.rs`, `.planning/phases/35-mdbook-currency/deferred-items.md`,
`.planning/phases/35-mdbook-currency/35-01-SUMMARY.md`, `.planning/REQUIREMENTS.md`,
`.planning/ROADMAP.md`, `crates/doc-examples/src/lib.rs`, `docs/src/SUMMARY.md`). All 5 commit
hashes confirmed present in `git log` (`eae9f91d`, `7cca347d`, `25f0e600`, `7f6e9cbb`, `3605af20`).
