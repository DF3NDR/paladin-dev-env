---
phase: 42-mid-run-halt-sse-terminal-status
plan: 01
subsystem: treasurer
tags: [adr, allowance, mid-run-halt, design-record, treasurer]

requires:
  - phase: 41-admission-time-allowance-enforcement
    provides: ADR-0056 admission model, Treasurer facade, AllowanceRefusal, treasury_notices, operator webhook target
  - phase: 39-treasury-ledger
    provides: ADR-0053 append-only ledger and the (run_id, superstep, attempt) settlement key
provides:
  - ADR-0057 mid-run halt contract (D-01..D-12, D-15, D-18 with RESEARCH gaps G1..G17 resolved)
  - ADR-0056 dated note superseding its reserve-at-the-boundary Downstream Consumers line
  - PROMOTION.md next free ADR number advanced to 0058
  - the recorded design-gate outcome (option-b) every later Phase 42 plan reads
affects: [42-02, 42-03, 42-04, 42-05, 42-06, 42-07, 42-08, 42-09, 42-10, 42-11, 42-12]

tech-stack:
  added: []
  patterns:
    - "One consolidated blocking design gate before any one-way door (Phase 39/40/41 precedent), recorded verbatim in the plan SUMMARY before any files_modified file is touched"
    - "Every overspend guarantee worded as bounded: one superstep on the engine path, one model response on the agent loop, one whole call on a true streamed call"

key-files:
  created:
    - .planning/decisions/0057-mid-run-halt-contract.md
  modified:
    - .planning/decisions/0056-allowance-admission-model.md
    - .planning/decisions/PROMOTION.md

key-decisions:
  - "Checkpoint decision: option-b (all 16 design items approved as written; item 12 extended so a true streamed execute/stream done carries an informational halt_reason when its terminal usage crossed the derived figure)"
  - "ADR-0057 records the boundary check as check-only at every boundary, failing closed with ledger_unavailable, and accepts the same-instant over-admission race (bounded to one superstep per run)"
  - "halt_reason discriminator is named reason, not kind, so the object stays byte-compatible with the Phase 41 429 details (G6)"
  - "Migration 013 (runs.halt_reason) and 014 (treasury_notices.notice_kind) are split so each one-way schema door is opened by one plan"
  - "D-12's stream clause holds for both the buffered fallback and the true stream under option-b"

patterns-established:
  - "ADR written first as the phase's opening plan so every later plan cites one record"

requirements-completed: []

duration: ~10min
completed: 2026-10-06
status: complete
---

# Phase 42 Plan 01: Mid-run halt design gate and ADR-0057 Summary

**ADR-0057 records the Phase 42 mid-run halt contract (check-only superstep boundary, typed `HaltCause`, fork-as-resume, dearest-axis derived agent budget, `reason`-tagged `halt_reason`) as confirmed at the operator's option-b design gate, with ADR-0056 pointing forward to it and the next free ADR number at 0058.**

## Checkpoint decision

**Selection: `option-b`.** Approve the consolidated design as proposed (all 16 items stand as written, no redirect) AND extend item 12 so that a true streamed `execute/stream` call's terminal `done` additionally carries an informational `halt_reason` object when the terminal chunk's usage crossed the derived figure. There is no behavioural halt on that path -- the call already finished -- and non-halt streams stay byte-identical.

Operator response, verbatim: `option-b`.

Meaning for item 12 and D-12: D-12's stream clause ("the agent stream's terminal `done` carries the same object") holds for BOTH the buffered fallback of `execute/stream` (whose `done` is the serialized `ExecuteResponse`, carrying `stop_reason: "allowance_halted"` and `halt_reason`) AND the true stream (informational crossing report only). ADR-0057 records that scope, and the G2 WINDOWS.md row still records that a single streamed call is not cut mid-flight.

Selected by the operator on 2026-10-06 at the plan 42-01 design gate presented by the execute-phase orchestrator.

Plans 42-07 and 42-08 (the derived budget, the agent routes and the streamed `done`) and 42-12 (the WINDOWS.md G2 row) read this recorded outcome; 42-02..42-12 read it through ADR-0057. This heading was written before any file in the plan's `files_modified` was touched.

## Performance

- **Duration:** ~10 min (continuation dispatch; Task 1 had already been presented by the orchestrator)
- **Started:** 2026-10-06T16:54Z
- **Completed:** 2026-10-06T16:59Z
- **Tasks:** 2 (Task 1 pre-resolved checkpoint, Task 2 executed)
- **Files modified:** 3 decision files (1 new)

## Accomplishments

- Recorded the option-b gate outcome verbatim and committed it alone (`56817f5`) before any `files_modified` file changed, so the ordering is provable in git history.
- Wrote ADR-0057 with all seven required headings, a subsection per decision group (a)-(g) plus (h) for the caller legs, registers and vocabulary guard, every D-NN (D-01..D-12, D-15, D-18, plus D-19 and D-21) and every gap G1..G17 named at the decision that resolves it, a Considered Options list drawn from the discussion log and RESEARCH's alternatives table, Code Locations by owning plan 42-02..42-12, and a Code Conformance section naming the test that holds each decision.
- Worded every overspend guarantee as bounded (one superstep on the engine path, one model response on the agent loop, one whole call on a true streamed call); the ADR contains no absolute "cannot overspend" claim.
- Added one dated note under ADR-0056's Phase 42 Downstream Consumers bullet (insertions only, no deleted line); advanced PROMOTION.md to 0058 with a dated note in the 41-09 format and appended the 0057 index row.

## Task Commits

1. **Task 1: design checkpoint** -- pre-resolved `option-b`; recording commit `56817f5` (docs, SUMMARY.md alone).
2. **Task 2: ADR-0057, ADR-0056 dated note, PROMOTION.md at 0058** -- `469511e` (docs, one commit).

## Verification

- Task 2's automated verify command (file exists, title, seven `## ` headings, `Next free ADR number: 0058`, `ADR-0057` in ADR-0056): exit 0.
- The D-NN loop (D-01..D-12, D-15, D-18) and the G-NN loop (G1..G17) print nothing.
- ADR-0057 cites ADR-0052, ADR-0053 and ADR-0056 and contains `one superstep` and `one model response`; a grep for "cannot overspend", "can never overspend" and "never overspend" finds nothing.
- `git diff` of ADR-0056 shows 4 insertions and 0 deleted lines; PROMOTION.md has no line containing `Next free ADR number: 0057`.
- `git log -1 --stat` for `469511e` lists exactly the ADR, the ADR-0056 note and PROMOTION.md; the post-commit deletion check found no deleted files.
- Documentation-only plan: no Rust code changed, so `cargo test`, `cargo fmt`, `cargo clippy`, `make api-surface` and `make security` were not applicable and were not run.

## Decisions Made

- Followed the plan as written. ADR-0057's Code Conformance reads `must change` (the ADR-0052 precedent) because it is written before any Phase 42 code exists; plan 42-12's closeout re-reads it against the tree.
- Added subsection (h) beyond the plan's (a)-(g) so D-19, D-21, G9, G15 and the ALLOW-05 vocabulary guard (gate items 14-16) are recorded in the ADR rather than only in the plan.

## Deviations from Plan

None - plan executed exactly as written.

## Authentication Gates

None.

## Known Stubs

None. No code was written.

## Threat Flags

None. T-42-01 and T-42-02 are mitigated by the ADR's Code Conformance section and by the verbatim decision record above; T-42-03 (the over-admission race, accepted) is recorded in ADR-0057 and its WINDOWS.md row is owned by plan 42-12.

## Notes for later plans

- `requirements-completed` is intentionally empty: ALLOW-03, ALLOW-05 and PLAT-09 are only designed here, not built, so `requirements mark-complete` was not run.
- Plans that hit an item this gate confirmed should cite ADR-0057 rather than 42-RESEARCH.md. Migration `013` is the run column only; the notice rung is migration `014` in plan 42-10.
- Plan 42-08 must implement the option-b branch of item 12 (informational `halt_reason` on the true streamed `done` when terminal usage crossed the derived figure; non-halt streams byte-identical).
- The commit trailers on this plan's two commits use the model name from the session's attribution reminder (`Claude Sonnet 5.5`); the dispatch prompt named a different model in its trailer text. If repo policy wants the dispatch wording, amend `56817f5` and `469511e` before push.

## Self-Check: PASSED

- FOUND: `.planning/decisions/0057-mid-run-halt-contract.md`
- FOUND: commit `56817f5` (decision recorded first) and commit `469511e` (ADR, ADR-0056 note, PROMOTION.md) in `git log`.
