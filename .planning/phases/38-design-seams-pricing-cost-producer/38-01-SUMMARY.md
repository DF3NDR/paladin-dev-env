---
phase: 38-design-seams-pricing-cost-producer
plan: 01
subsystem: architecture-decisions
tags: [adr, treasurer, ledger, pricing, cost, promotion-index]

# Dependency graph
requires: []
provides:
  - "ADR-0052: mid-run Treasurer enforcement attachment point (D-13) — metering at the LlmPort pricing decorator on both run paths, halt at the WarEngine superstep boundary and the agent-loop TokenBudget cutoff"
  - "ADR-0053: treasury ledger balance model (D-14/D-15) — append-only, derive-on-read; reserve/settle/release row kinds; i64 nano-unit amounts with ISO 4217 currency; settlement idempotency key (run_id, superstep, attempt) with superstep-aggregate granularity"
  - "PROMOTION.md numbering index rows 0052/0053, next-free 0054"
  - "PRICE-02 / ROADMAP Phase 38 success criterion 2 nano-unit wording reconciled at source"
affects: ["39-*", "41-*", "42-*", "43-*"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "ADR checkpoint-decision resolution: a blocking checkpoint's operator selection is recorded inline in the ADR's Decision section, with the rejected alternative kept in Considered Options rather than deleted"

key-files:
  created:
    - .planning/decisions/0052-mid-run-treasurer-enforcement.md
    - .planning/decisions/0053-ledger-balance-model.md
  modified:
    - .planning/decisions/0050-treasurer-reservation.md
    - .planning/decisions/PROMOTION.md
    - .planning/PROJECT.md
    - .planning/REQUIREMENTS.md
    - .planning/ROADMAP.md

key-decisions:
  - "ADR-0052: metering in PricingLlmAdapter at the LlmPort boundary on both run paths (outside FallbackLlmAdapter); engine-path halt at the WarEngine superstep boundary (WaypointStatus::Halted); agent-loop halt reuses the existing TokenBudget after_model cutoff; AgentRuntimeConfig::build_chain confirmed to have zero production callers and stays unwired for the engine path"
  - "ADR-0053 checkpoint decision (operator, 2026-09-25): superstep-aggregate settlement — one settlement per superstep attempt aggregates every Paladin node's Cost dispatched in that superstep; D-15's idempotency key (run_id, superstep, attempt) kept exactly as locked, no node_id added, D-15 not amended; per-node cost stays visible only in NodeFinished.cost trace events, not as a ledger row"
  - "Rejected alternative recorded in ADR-0053: per-node settlement with the key extended to (run_id, superstep, node_id, attempt) — would have amended locked decision D-15, produced more rows, and required Phase 42's halt check to sum several per-node settlements instead of reading one row"
  - "PostgreSQL/SQLite/in-memory SUM-then-reserve serialization named explicitly in ADR-0053 (per-scope row lock or advisory lock before SUM on Postgres since FOR UPDATE is rejected on an aggregate query; BEGIN IMMEDIATE on SQLite; one mutex in-memory)"
  - "PRICE-02 and ROADMAP Phase 38 success criterion 2 amended at source with a dated note: the cost unit is i64 nano-units (1e-9), a finer scale that satisfies the requirement's 'micro-units' wording"

requirements-completed: [PRICE-02, PRICE-03]

coverage:
  - id: D1
    description: "ADR-0052 recorded: mid-run Treasurer enforcement attachment point across WarEngine and PaladinExecutionService, with the build_chain zero-production-callers fact and two rejected alternatives"
    requirement: "PRICE-03"
    verification:
      - kind: other
        ref: "node .claude/gsd-core/bin/lib/adr-parser.cjs --input .planning/decisions/0052-mid-run-treasurer-enforcement.md (status: accepted, 7 headings)"
        status: pass
    human_judgment: false
  - id: D2
    description: "ADR-0053 recorded: append-only derive-on-read ledger, row kinds, i64 nano-unit amounts, and settlement key/granularity resolved by the operator's checkpoint decision (superstep-aggregate)"
    requirement: "PRICE-02"
    verification:
      - kind: other
        ref: "node .claude/gsd-core/bin/lib/adr-parser.cjs --input .planning/decisions/0053-ledger-balance-model.md (status: accepted, 7 headings)"
        status: pass
    human_judgment: false
  - id: D3
    description: "PROMOTION.md numbering index carries rows 0052/0053, next-free line reads 0054, existing rows 0049-0051 unchanged"
    verification:
      - kind: other
        ref: "grep '| 0052 |' / '| 0053 |' / 'Next free ADR number: 0054' .planning/decisions/PROMOTION.md; git diff shows no removed pre-existing rows"
        status: pass
    human_judgment: false
  - id: D4
    description: "PRICE-02 and ROADMAP Phase 38 success criterion 2 nano-unit wording reconciled at source with a dated amend note"
    requirement: "PRICE-02"
    verification:
      - kind: other
        ref: "grep 'Amended 2026-09-25, Phase 38 plan 38-01' .planning/REQUIREMENTS.md .planning/ROADMAP.md"
        status: pass
    human_judgment: false

duration: ~5min execution across two agent sessions (Task 1, then a resolved blocking checkpoint, then Task 2 in this continuation)
completed: 2026-09-25
status: complete
---

# Phase 38 Plan 01: Design Seams ADRs Summary

**ADR-0052 (mid-run Treasurer enforcement attachment point) and ADR-0053 (ledger balance model, superstep-aggregate settlement) recorded before any pricing code lands, per D-16**

## Performance

- **Duration:** ~5min execution time across two agent sessions (Task 1 at 16:58:03Z, checkpoint resolved by operator, Task 2 at 17:02:31Z)
- **Started:** 2026-09-25T16:54:11Z (phase execution start, per STATE.md)
- **Completed:** 2026-09-25T17:02:31Z
- **Tasks:** 2 (plus one blocking `checkpoint:decision` between them)
- **Files modified:** 7 (2 created, 5 modified)

## Accomplishments
- ADR-0052 fixes the D-13 attachment split: metering in the `PricingLlmAdapter` decorator at the
  `LlmPort` boundary on both run paths, halt at the `WarEngine` superstep boundary, and the
  agent-loop `TokenBudget` `after_model` cutoff — with the verified fact that
  `AgentRuntimeConfig::build_chain` has zero production callers today.
- ADR-0053 fixes the D-14/D-15 ledger model: append-only, derive-on-read; `reserve`/`settle`/
  `release` row kinds with signed contributions; `i64` nano-unit amounts with an ISO 4217 currency
  code; and, per the resolved checkpoint, superstep-aggregate settlement under the unamended
  `(run_id, superstep, attempt)` key.
- The blocking `checkpoint:decision` between Task 1 and Task 2 was resolved by the operator
  choosing `superstep-aggregate` (the plan's recommended option): one settlement row per superstep
  attempt aggregates every Paladin node's `Cost` dispatched in that superstep, so two nodes in one
  superstep can never collide on the locked key; per-node spend remains visible only through
  `NodeFinished.cost` trace events, not as a separate ledger row.
- PROMOTION.md's numbering index now carries rows 0052 and 0053 with next-free advanced to 0054,
  each ADR getting its own dated advancing note; PROJECT.md's `## Key Decisions` table gains both
  rows; PRICE-02 and ROADMAP Phase 38 success criterion 2 are reconciled at source with dated
  amend notes for the nano-unit cost scale.

## Task Commits

Each task was committed atomically:

1. **Task 1: ADR-0052 — mid-run Treasurer enforcement attachment point** - `8d76aa2a` (docs) —
   completed by a prior executor session, verified present at the start of this continuation
   (`git show --stat 8d76aa2a` confirmed the expected four files).
2. **Checkpoint: what one settlement row covers under the `(run_id, superstep, attempt)` key** —
   resolved by the operator (`superstep-aggregate`, recommended option) between Task 1 and Task 2;
   see *Checkpoint Status* below.
3. **Task 2: ADR-0053 — ledger balance model, row kinds, amount unit and idempotency key** -
   `c8afc891` (docs)

**Orchestrator tracking commit (not plan work):** `9576f4b1` (docs: begin phase 38 execution
tracking — STATE.md only).

**Plan metadata:** this SUMMARY and the STATE.md/ROADMAP.md updates below are committed separately
per the executor's final-commit step.

_Note: both tasks are `docs`-type commits — this plan changes zero code under `crates/` or `src/`
(confirmed for both commits via `git diff --name-only`)._

## Files Created/Modified
- `.planning/decisions/0052-mid-run-treasurer-enforcement.md` - New ADR-0052 (Task 1)
- `.planning/decisions/0053-ledger-balance-model.md` - New ADR-0053 (Task 2)
- `.planning/decisions/0050-treasurer-reservation.md` - Dated note: Milestone 14 build began Phase
  38 under the reserved name (Task 1)
- `.planning/decisions/PROMOTION.md` - Index rows 0052/0053, next-free 0054, two separate dated
  advancing notes (Task 1, Task 2)
- `.planning/PROJECT.md` - `## Key Decisions` rows for ADR-0052 and ADR-0053; ADR-0050 outcome
  cell appended (Task 1, Task 2)
- `.planning/REQUIREMENTS.md` - PRICE-02 dated amend note reconciling the nano-unit cost scale
  (Task 2)
- `.planning/ROADMAP.md` - Phase 38 success criterion 2 dated amend note (Task 2)

## Decisions Made

- **Settlement granularity (checkpoint, operator, 2026-09-25):** `superstep-aggregate` selected
  over `per-node-extended-key`. Rationale recorded in ADR-0053: keeps D-15's locked key
  `(run_id, superstep, attempt)` unamended, matches ADR-0052's superstep-boundary halt so the draw,
  settle and halt check all happen at one point, and produces fewer ledger rows. The traded-off
  cost — per-node spend is not independently queryable from the ledger — is accepted; it remains
  available via `NodeFinished.cost` trace events.
- All other content follows the plan's `<action>` instructions directly; no additional
  Claude's-Discretion calls were needed beyond what the plan already specified for this plan's
  scope (Task 1's type-homes / decorator-wiring discretion items belong to plan 38-02+, not this
  plan).

## Checkpoint Status

**Checkpoint:** "what one settlement row covers under the `(run_id, superstep, attempt)` key"
(blocking `checkpoint:decision`, between Task 1 and Task 2).

- **Options presented:** `superstep-aggregate` (recommended — settlement per superstep attempt,
  D-15's key unchanged) vs. `per-node-extended-key` (settlement per node attempt, D-15's key
  extended with `node_id`).
- **Resolution:** operator selected `superstep-aggregate`, provided directly in this continuation
  agent's spawn context (not re-asked, per the resume instructions).
- **Applied:** ADR-0053's Decision point 4 records exactly this option, the rejected alternative
  with its concrete costs is recorded in Considered Options, and D-15 itself is explicitly stated
  as **not amended**.

## Deviations from Plan

None — plan executed exactly as written. The `## Considered Options` bullet ordering in ADR-0053
places the checkpoint-rejected `per-node-extended-key` option last (after the two D-15 options the
plan's own bullet list ordered first), which is a presentation choice within the plan's own bulleted-list
requirement, not a deviation from any locked decision.

## Issues Encountered

None. Task 1's prior commit (`8d76aa2a`) was verified present with `git log --oneline -3` and
`git show --stat 8d76aa2a` before any work began in this continuation, confirming the four expected
files were touched and `0052-mid-run-treasurer-enforcement.md` exists on disk — Task 1 was not
redone.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Both gating ADRs (0052, 0053) are on record, `Accepted`, with the PROMOTION.md index and
  PROJECT.md Key Decisions table current. Phase 39, 41 and 42 plans can now cite them by number
  rather than re-opening the attachment-point or ledger-model questions.
- Wave 2 (plan 38-02, the tracer) is unblocked: no code under `crates/` or `src/` was touched by
  this plan, matching D-16's "ADRs first" ordering.
- Open item carried forward explicitly by ADR-0053 for Phase 39/42: the persistence source of the
  engine-path attempt counter is not yet decided — flagged, not invented, in the ADR text.

---
*Phase: 38-design-seams-pricing-cost-producer*
*Completed: 2026-09-25*

## Self-Check: PASSED

- FOUND: `.planning/decisions/0052-mid-run-treasurer-enforcement.md`
- FOUND: `.planning/decisions/0053-ledger-balance-model.md`
- FOUND: `.planning/phases/38-design-seams-pricing-cost-producer/38-01-SUMMARY.md`
- FOUND: commit `8d76aa2a` (Task 1, ADR-0052)
- FOUND: commit `c8afc891` (Task 2, ADR-0053)
- FOUND: commit `0050644f` (this SUMMARY, pre-existing at self-check time)
- FOUND: commit `9576f4b1` (orchestrator tracking, STATE.md only)
