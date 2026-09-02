---
phase: 22-battlefield-state-superstep-engine
plan: 16
subsystem: engine
tags: [rust, war-engine, graph-validation, self-loop, tdd, audit]

# Dependency graph
requires:
  - phase: 22-battlefield-state-superstep-engine
    provides: "WarGraph::validate eligible-set reachability check (ENG-FR-02a, Plan 22-15)"
provides:
  - "Per-fixture audit classifying every strandedness-adjacent test in the tree (strandedness dodge / readiness dodge / unrelated)"
  - "Corrected, accurate comments on every readiness-dodge fixture naming Frontier::is_ready as the real cause"
  - "A runnable ignored reproduction of a second, distinct truthful-outcome defect (self-loop + upstream edge blocks first execution)"
  - "Confirmed pre-release classification for BUG-02: no migration entry, no compatibility-register row required"
affects: [22-17 (checkpoint to confirm the new defect's disposition)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Ignored-but-correct reproduction test: assert the behavior that SHOULD hold, mark #[ignore] with a reason naming the defect, never invert to match current wrong behavior"

key-files:
  created: []
  modified:
    - crates/paladin-battalion/src/engine/graph.rs
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/src/engine/superstep.rs
    - tests/integration/e2e_crash_resume_test.rs
    - .planning/phases/22-battlefield-state-superstep-engine/22-deferred-items.md

key-decisions:
  - "Nine self-loop-only fixtures across three files (graph.rs x3, mod.rs x1, superstep.rs's self_loop_graph helper covering 3 tests, plus e2e_crash_resume_test.rs's loop_gate) are classified 'readiness dodge', not strandedness -- they declare their looping node as graph entry because Frontier::is_ready leaves a self-loop's own edge Pending until the node has run once, so a non-entry version could never take a first turn regardless of BUG-02. Comments corrected in place; arrangements kept (removing them would deadlock the fixtures)."
  - "engine::mod.rs's resume_restores_visit_counts_and_trips_limit_on_next_post_resume_visit is a newly-discovered readiness-dodge instance, absent from Plan 22-15's SUMMARY handoff list -- found only by the fresh tree-wide search this plan's Task 1 required, confirming that search was necessary rather than redundant."
  - "graph.rs's validate_accepts_self_loop_on_node_reachable_from_entry_by_normal_edge constructs the harder self-loop-plus-upstream-edge shape but only calls validate(), never runs -- classified 'unrelated' since Frontier::is_ready is never consulted; comment added pointing at the new reproduction test rather than repairing anything."
  - "The legacy campaign self_loop test (tests/integration/battalion/campaign_integration_test.rs) is confirmed unrelated by reading it, not assumed: it exercises CampaignExecutionService's cycle-REJECTING validation, the opposite semantics of WarGraph (which permits self-loops by design, ENG-FR-02)."
  - "The crash-resume fixture's module doc comment overgeneralized 'every self-loop test in this workspace' uses the graph-entry arrangement -- corrected to name the one validate-only exception found during the audit, and to point at the new reproduction test for the general self-loop-plus-upstream-edge case it was actually describing."
  - "The residual readiness defect (self-loop + upstream edge blocks first execution) is recorded with a runnable #[ignore]d reproduction rather than prose, asserting correct behavior so it fails today and turns green when fixed -- never inverted to pin today's wrong behavior as expected."
  - "No file under .project/ was edited to register the new defect: proposing its disposition is this plan's job (recorded in 22-deferred-items.md); registering it in the program overview's binding defect register is a developer decision, confirmed at the Plan 22-17 checkpoint."
  - "BUG-02's pre-release classification confirmed from the repository: 16 tags exist total, but v0.9.0 (2026-09-01) is both the most recent by creation date and the highest by semver with nothing tagged after it, and the whole crates/paladin-battalion/src/engine/ directory is absent at that tag (git ls-tree returns nothing; the file's origin traces to this milestone's own Plan 22-01 tracer-slice commit) -- no migration entry or compatibility-register row required."

requirements-completed: [ENG-02]

coverage:
  - id: D1
    description: "Every strandedness-adjacent fixture in the tree (9 self-loop call sites across 4 files, found via files_modified, the 22-15 handoff list, and a fresh tree-wide search) is classified in writing into strandedness dodge / readiness dodge / unrelated, with evidence per row"
    requirement: "ENG-02"
    verification:
      - kind: manual_procedural
        ref: ".planning/phases/22-battlefield-state-superstep-engine/22-deferred-items.md#finding-2-per-fixture-classification-table"
        status: pass
    human_judgment: false
  - id: D2
    description: "No remaining fixture arranges its graph to dodge strandedness; readiness-dodge fixtures keep their arrangement but now carry a comment naming Frontier::is_ready as the real cause, including a newly-discovered instance not in the 22-15 handoff list"
    requirement: "ENG-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#validate_accepts_self_loop, self_loop_on_entry_node_still_validates_and_runs"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#resume_restores_visit_counts_and_trips_limit_on_next_post_resume_visit"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#self_loop_graph (self_loop_runs_exactly_three_times_when_approved_on_third_visit, self_loop_never_approved_trips_node_visit_limit_at_five, self_loop_at_four_visits_does_not_trip)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The crash-resume fixture's loop_gate arrangement is named explicitly and its comment corrected to state it has no separate upstream feed and to fix the 'every self-loop test' overgeneralization"
    requirement: "ENG-02"
    verification:
      - kind: integration
        ref: "tests/integration/e2e_crash_resume_test.rs#e2e_1_crash_resume_matches_control_run_with_no_reexecution"
        status: pass
    human_judgment: false
  - id: D4
    description: "The legacy campaign self-loop test is confirmed, by reading it, to exercise the cycle-rejecting Campaign service rather than WarGraph, and is ruled out of scope"
    requirement: "ENG-02"
    verification:
      - kind: manual_procedural
        ref: "tests/integration/battalion/campaign_integration_test.rs#test_self_loop_detection"
        status: pass
    human_judgment: false
  - id: D5
    description: "A runnable, ignored reproduction of a distinct truthful-outcome defect (self-loop + upstream edge blocks first execution) exists, asserts correct behavior, fails under --ignored, and leaves the default workspace run green"
    requirement: "ENG-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#self_looping_node_fed_by_upstream_edge_can_never_take_first_turn"
        status: pass
    human_judgment: false
  - id: D6
    description: "BUG-02's pre-release classification (no migration entry, no compatibility-register row) is confirmed from the repository with the commands that establish it, not restated"
    requirement: "ENG-02"
    verification:
      - kind: manual_procedural
        ref: ".planning/phases/22-battlefield-state-superstep-engine/22-deferred-items.md#pre-release-compatibility-item--closed-by-confirmation"
        status: pass
    human_judgment: false

# Metrics
duration: ~30min
completed: 2026-09-02
status: complete
---

# Phase 22 Plan 16: ENG-FR-02a Fixture Audit & Readiness Defect Discovery Summary

**Every strandedness-adjacent fixture in the tree (9 self-loop call sites across 4 files) is classified as a strandedness dodge, readiness dodge, or unrelated, with a newly-discovered readiness-dodge instance and a second, distinct truthful-outcome defect (self-loop + upstream edge blocks a node's first execution) captured in a runnable ignored reproduction — closing gap G-22-3.**

## Performance

- **Duration:** ~30 min
- **Tasks:** 3
- **Files modified:** 5 (`crates/paladin-battalion/src/engine/{graph.rs,mod.rs,superstep.rs}`, `tests/integration/e2e_crash_resume_test.rs`, `.planning/phases/22-battlefield-state-superstep-engine/22-deferred-items.md`)

## Accomplishments

- **Complete fixture audit.** Enumerated every self-loop construction in the tree (9 call sites across `crates/paladin-battalion/src/engine/graph.rs`, `mod.rs`, `superstep.rs`, and `tests/integration/e2e_crash_resume_test.rs`) via three independent sources — this plan's `files_modified`, the 22-15 SUMMARY's "Fixtures Handed to Plan 22-16" list, and a fresh tree-wide `EdgeSpec { from: X, to: X }` pattern search — and classified each into exactly one of three buckets in a written table in `22-deferred-items.md`.
- **Zero remaining strandedness dodges.** Confirmed no fixture in the tree still arranges its graph to dodge BUG-02's rejection: the three `graph.rs` rejection-ordering tests demonstrate the rejection rather than avoid it, and the one case Plan 22-15 already caught (`resume_allow_graph_change_proceeds_when_vanguard_node_present`'s node `c`) was fixed there.
- **Nine readiness-dodge fixtures corrected, not repaired.** All nine self-loop-only fixtures (single node, self-loop is its only possible incoming edge) keep their graph-entry arrangement — removing it would deadlock them permanently — but now carry accurate inline comments naming `Frontier::is_ready`'s Pending-self-edge behavior as the real cause, replacing silence or (in the crash-resume case) a mild overgeneralization.
- **One new discovery.** `engine::mod.rs`'s `resume_restores_visit_counts_and_trips_limit_on_next_post_resume_visit` is a readiness-dodge instance absent from Plan 22-15's handoff list — found only because Task 1's fresh tree-wide search was performed rather than trusting the handoff list as complete.
- **A second, distinct truthful-outcome defect surfaced and reproduced.** A node that is both self-looping AND fed by a separate upstream edge can never satisfy `Frontier::is_ready` (its self-edge stays `Pending` forever, since only its own first run could resolve it), so the run reports `RunOutcome::Completed` with that node's `run_count()` at `0` — the same class of defect as BUG-02, reached by a different mechanism, and NOT caught by Plan 22-15's static eligible-set reachability check (the node IS statically reachable via the upstream edge). Captured as `self_looping_node_fed_by_upstream_edge_can_never_take_first_turn`, an `#[ignore]`d test in `engine::superstep`'s test module that asserts correct behavior and fails on demand (`cargo test -p paladin-battalion --lib engine::superstep -- --ignored --nocapture` → `test result: FAILED. 0 passed; 1 failed`).
- **Legacy campaign test confirmed out of scope by reading it.** `tests/integration/battalion/campaign_integration_test.rs::test_self_loop_detection` exercises `CampaignExecutionService`'s cycle-**rejecting** validation — the opposite semantics of `WarGraph`, which permits self-loops by design (ENG-FR-02). Not a stranded-node workaround.
- **Pre-release classification confirmed from the repository.** 16 git tags exist total, but `v0.9.0` (2026-09-01) is both the latest by creation date and the highest by semver with nothing tagged after it, and `git ls-tree -r v0.9.0 -- crates/paladin-battalion/src/engine` returns nothing — the whole engine module postdates every released version, tracing to this milestone's own Plan 22-01 tracer-slice commit. No migration entry or compatibility-register row required; none of the three established facts contradicted the classification.

## Task Commits

Each task was committed atomically:

1. **Task 1: Audit and classify every strandedness-adjacent fixture** — `34630e86` (fix)
2. **Task 2: Record the readiness defect with a runnable reproduction** — `9e9bdb59` (test)
3. **Task 3: Confirm the pre-release classification from the repository** — `a171bbb4` (docs)

**Plan metadata:** (this commit, docs: complete plan)

_Note: Task 2 carries `tdd="true"` in the plan but is not a standard RED→GREEN cycle — there is no
GREEN step in this plan, since fixing the newly-discovered readiness defect is explicitly out of
scope (a frontier semantics change, assigned to a future phase). The single `test` commit is the
entire, intentionally-permanent-until-fixed reproduction; `cargo test --workspace` stays green
because the test is `#[ignore]`d, and `--ignored` reproduces the failure on demand exactly as the
plan's verify step requires. No TDD gate-sequence violation: the plan's own acceptance criteria
call for exactly this shape ("do not invert the assertions to match current behaviour")._

## Files Created/Modified

- `crates/paladin-battalion/src/engine/graph.rs` — Added accurate readiness-dodge comments to `validate_accepts_self_loop` and `self_loop_on_entry_node_still_validates_and_runs`; added an "unrelated, validate-only" comment to `validate_accepts_self_loop_on_node_reachable_from_entry_by_normal_edge` pointing at the new reproduction test. No test logic changed; no assertion weakened.
- `crates/paladin-battalion/src/engine/mod.rs` — Added an accurate readiness-dodge comment to `resume_restores_visit_counts_and_trips_limit_on_next_post_resume_visit` (newly-discovered instance).
- `crates/paladin-battalion/src/engine/superstep.rs` — Added an accurate readiness-dodge comment to the `self_loop_graph` helper (covers three call sites); added the new `#[ignore]`d reproduction test `self_looping_node_fed_by_upstream_edge_can_never_take_first_turn` with a full mechanism doc comment.
- `tests/integration/e2e_crash_resume_test.rs` — Corrected `build_graph`'s module doc comment: `loop_gate` has no separate upstream feed (a simpler bootstrap case than the prior wording implied), and the "every self-loop test in this workspace" claim now names its one validate-only exception and points at the new reproduction test.
- `.planning/phases/22-battlefield-state-superstep-engine/22-deferred-items.md` — New "Phase 22 Plan 16" section: the 15-row per-fixture classification table, the acceptance-2a-satisfied statement, the residual readiness-defect finding (mechanism, reproduction, recommended disposition), and the pre-release-classification confirmation.

## Decisions Made

See `key-decisions` in the frontmatter above — summarized: nine self-loop fixtures classified readiness-dodge and corrected in place (arrangement kept, comment fixed); one new readiness-dodge instance discovered by the required fresh search; one fixture classified unrelated because it never runs; the legacy campaign test ruled out by reading it; the crash-resume module comment's overgeneralization corrected; the new readiness defect recorded with a runnable, non-inverted reproduction and a recommended (not enacted) disposition; and BUG-02's pre-release classification confirmed rather than assumed.

## Deviations from Plan

None — plan executed exactly as written. Task 1's "confirm the [suspicious-fixture] set is complete" instruction surfaced one genuinely new fixture (`engine::mod.rs`'s `resume_restores_visit_counts_and_trips_limit_on_next_post_resume_visit`) not in the 22-15 handoff list; this was anticipated by the plan's own wording ("do not rely on the four files being the complete set, confirm it") rather than being an out-of-plan discovery requiring a deviation rule.

## Issues Encountered

None. `cargo test --workspace` stayed green (38/38 binaries) after every task; `cargo fmt --check` and `cargo clippy -p paladin-battalion --all-targets -- -D warnings` were clean throughout; the reproduction test's failure under `--ignored` was confirmed directly (`test result: FAILED. 0 passed; 1 failed`, panic naming `run_count() == 0`) before committing Task 2.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Acceptance 2a (ENG-FR-02a) is fully satisfied: no remaining fixture in the tree dodges strandedness, and every readiness-dodge arrangement is now accurately documented.
- A new, distinct defect is ready for the Plan 22-17 checkpoint to confirm a disposition for: a self-looping node fed by an upstream edge can never take its first turn (`Frontier::is_ready`'s Pending-self-edge behavior). Recommended in `22-deferred-items.md`: register alongside BUG-01/BUG-02 in `.project/v0.10.0/00-program-overview.md`'s defect register, and assign to whichever phase next touches frontier/routing semantics (Phase 23 Muster fan-out or Phase 25 Aegis are the two candidates already touching this code). **No file under `.project/` was edited by this plan** — that registration is a developer decision.
- The reproduction test (`engine::superstep::tests::self_looping_node_fed_by_upstream_edge_can_never_take_first_turn`) is available on demand (`cargo test -p paladin-battalion --lib engine::superstep -- --ignored --nocapture`) for whichever future phase fixes the defect — it will turn green automatically once `Frontier::is_ready`'s self-edge-gating behavior is corrected, with no further test changes needed.
- BUG-02's compatibility posture is closed: confirmed pre-release, no migration entry, no compatibility-register row.

## Self-Check: PASSED

- FOUND: `crates/paladin-battalion/src/engine/graph.rs` (modified, present)
- FOUND: `crates/paladin-battalion/src/engine/mod.rs` (modified, present)
- FOUND: `crates/paladin-battalion/src/engine/superstep.rs` (modified, present)
- FOUND: `tests/integration/e2e_crash_resume_test.rs` (modified, present)
- FOUND: `.planning/phases/22-battlefield-state-superstep-engine/22-deferred-items.md` (modified, present)
- FOUND: commit `34630e86` (Task 1)
- FOUND: commit `9e9bdb59` (Task 2)
- FOUND: commit `a171bbb4` (Task 3)

---
*Phase: 22-battlefield-state-superstep-engine*
*Completed: 2026-09-02*
