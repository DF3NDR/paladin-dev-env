---
phase: 48-base-entity-rename
plan: 02
subsystem: decisions
tags: [adr, x-03-supersession, entity-rename, serde-read-alias, vocabulary-rule, clean-break]

requires:
  - phase: 44-legacy-clean-break-removal
    provides: ADR-0059 as the shape (not the licence) this record copies
provides:
  - "ADR-0060: the Phase 48-only X-03 supersession, the Entity<T> design, the retained-pending-review disposition and the node/Entity vocabulary rule"
  - "PROMOTION.md index row 0060 and next free ADR number 0061"
  - "PROJECT.md Key Decisions row and the Phase 48 sentence in the Out-of-Scope X-03 bullet"
  - "Review-todo section listing the entity-sense identifiers that stay inside the five retained base items"
affects: [48-03, 48-04, 48-05, 48-06, 48-07, 48-08, 48-09, phase-46, phase-47]

tech-stack:
  added: []
  patterns:
    - "Per-phase X-03 supersession as its own ADR, copying the ADR-0059 shape and never inheriting its licence"
    - "Serde read-aliases carved out of the no-API-shim rule as data compatibility for persisted JSON"

key-files:
  created:
    - .planning/decisions/0060-base-entity-rename.md
  modified:
    - .planning/decisions/PROMOTION.md
    - .planning/PROJECT.md
    - .planning/todos/pending/2026-10-13-review-content-data-orchestration-base-items.md

key-decisions:
  - "ADR-0060 supersedes X-03 for Phase 48 only, exactly for the base wrapper's old name and module, the old version family, the citadel placeholder types and alias, and the two wrapper fields; MIGRATION.md 9.7 stays empty"
  - "The deprecated-alias alternative is rejected because the payload field rename cannot be aliased in Rust, so every payload access and struct literal still breaks downstream"
  - "The serde read-aliases (node, node_id, node_data) are data compatibility for persisted JSON kept for the whole 0.x line, not an API shim; removal needs its own ADR"
  - "Field, Collection/Item, the element module, FieldVersionService and CollectionVersionService are retained pending the content / general data orchestration review, with Field.nid, the node_id fields and get_node_field_versions recorded as the one bounded exception to the vocabulary rule"
  - "The vocabulary rule: node means a vertex in a WarGraph or Campaign graph; Entity means the base-layer wrapper, a deliberately generic term"

patterns-established:
  - "Licence plan first: the ADR lands before any removal commit, and every code plan depends on it"

requirements-completed: [ENTITY-02, ENTITY-04, ENTITY-05]

coverage:
  - id: D1
    description: "ADR-0060 exists with the exact PROMOTION.md heading set, a Phase 48-only X-03 supersession, every tracked decision with its recorded reversibility, the rejected-alias reasoning, the retained base items with their bounded exception, the citadel unification and the vocabulary rule"
    requirement: "ENTITY-02"
    verification:
      - kind: other
        ref: "Task 1 automated verify and acceptance_criteria greps in 48-02-PLAN.md (heading set, supersession sentence, vocabulary sentence, D-number coverage, 22 Considered Options bullets, Code Conformance must change)"
        status: pass
    human_judgment: false
  - id: D2
    description: "ADR-0060 is indexed in PROMOTION.md (row 0060, next free 0061, dated note) and PROJECT.md (Key Decisions row, X-03 bullet sentence)"
    requirement: "ENTITY-05"
    verification:
      - kind: other
        ref: "Task 2 automated verify and acceptance_criteria greps in 48-02-PLAN.md"
        status: pass
    human_judgment: false
  - id: D3
    description: "The review todo carries the list of entity-sense identifiers retained inside the five base items, appended without removing any existing line"
    requirement: "ENTITY-04"
    verification:
      - kind: other
        ref: "git diff --numstat on the todo shows 20 added and 0 deleted lines; the section names get_node_field_versions and Field.nid"
        status: pass
    human_judgment: false

duration: 20min
completed: 2026-10-13
status: complete
---

# Phase 48 Plan 02: ADR-0060 Base Entity Rename Summary

**ADR-0060 licenses the Phase 48 clean break before any code moves: X-03 superseded for Phase 48 only, `Entity<T>` with persisted-JSON read-aliases carved out of the no-API-shim rule, five base items retained pending review with their residual `node_id` identifiers listed, and "node" reserved for the graph sense.**

## Performance

- **Duration:** about 20 min
- **Completed:** 2026-10-13
- **Tasks:** 2
- **Files modified:** 4 (one created, three edited)

## Accomplishments

- Authored ADR-0060 (Accepted, 2026-10-13, Code Conformance `must change` until plan 48-09). Decisions (a) to (k) cover the supersession and what still applies, `Entity<T>` and its `wrapper` module, persisted JSON and the downgrade asymmetry, the `entity` wrapper fields, the `EntityVersion*` family, the revisioning contract, the retained base items, the citadel unification (13-field wire identity and the `Reasoning` status asymmetry), the vocabulary rule with the frozen graph list and the four guard rule families, the register, and sequencing.
- Recorded why the deprecated-alias alternative was rejected and kept ADR-0051 and ADR-0059 as shape only, never licence.
- Indexed the record: PROMOTION.md row 0060 directly after 0059, `Next free ADR number: 0061` with a dated note, PROJECT.md Key Decisions row and the Phase 48 sentence in the Out-of-Scope X-03 bullet.
- Appended the residual entity-sense identifiers (`Field.nid`, the three `node_id` fields, both `get_node_field_versions`) to the review todo so the content-system review decides them.

## Task Commits

1. **Task 1: Author ADR-0060** - `f053585` (docs)
2. **Task 2: Index ADR-0060 and record the retained-item residual** - `c06ab43` (docs)

**Plan metadata:** the commit carrying this SUMMARY, STATE.md and ROADMAP.md (docs: complete plan)

## Files Created/Modified

- `.planning/decisions/0060-base-entity-rename.md` - the ADR
- `.planning/decisions/PROMOTION.md` - row 0060, next free 0061, dated note
- `.planning/PROJECT.md` - Key Decisions row and the Phase 48 X-03 supersession sentence
- `.planning/todos/pending/2026-10-13-review-content-data-orchestration-base-items.md` - appended section `## Residual entity-sense identifiers (Phase 48 planning note)`

## Decisions Made

- Where an owner decision carried no recorded reversibility rating in 48-CONTEXT.md (D-04, D-06, D-07, D-11, D-12, D-14, D-15), the ADR states none rather than inventing one; D-01, D-08, D-09 and D-10 are `costly`, D-02 and D-03 `one-way`, D-05 `reversible`.
- ADR-0060 describes the citadel importers by pointing at plan 48-04's measured list instead of repeating CONTEXT's count of five (flagged assumption A1-48-02), and places the graph `node_cache` definitions at `src/config/node_cache.rs` (A2-48-02).
- The ADR's Considered Options list the survey's removal of the five base items under a "void" reason only, and no retained item is described as removed, deprecated or scheduled for removal.

## Deviations from Plan

None - plan executed exactly as written.

Two small wording fixes were made inside Task 1 before its commit, both against the plan's own acceptance greps: the D-00e bullet was reworded to say Phase 44 touched none of this phase's files (an earlier draft said the reverse), and the no-shim rule was phrased with the literal `no API shim` the acceptance criterion counts.

**Total deviations:** 0 auto-fixed.
**Impact on plan:** none.

## Commit trailers

The launching prompt asked for `Co-Authored-By: Claude Fable 5.1` trailers, while this session's attribution reminder specifies `Claude Sonnet 5.5`, which is the model that actually authored these commits. The commits carry `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>` and the shared `Claude-Session` line.

## Issues Encountered

None. The plan is documentation only: no code, configuration or credential path changed, so the D-00a manual credential-handling review has nothing to cover, and cargo was not run.

## Known Stubs

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

The licence precedes every removal, so plans 48-03 onward are unblocked (48-03 and 48-04 depend on this plan; 48-01 runs beside it in wave 1). Plan 48-09 must replace the ADR's Code Conformance verdict and PROJECT.md's Pending outcome with the measured result.

## Self-Check: PASSED

- FOUND: `.planning/decisions/0060-base-entity-rename.md`
- FOUND: commit `f053585`, commit `c06ab43`
- Task 1 and Task 2 automated verifies and acceptance greps all pass; the todo diff is append-only (20 added, 0 deleted).
- `git diff --stat HEAD~2` for the two task commits touches only the four files in `files_modified`.

---
*Phase: 48-base-entity-rename*
*Completed: 2026-10-13*
