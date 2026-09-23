---
phase: 36-rustdoc-zero-warning-bar-examples-currency
plan: 07
subsystem: docs
tags: [examples-gallery, human-in-the-loop, gate, parley, chronicle, replay, graceful-shutdown, shutdown-coordinator]

# Dependency graph
requires:
  - phase: 36-rustdoc-zero-warning-bar-examples-currency
    provides: 36-01-SUMMARY.md (house example shape -- header/README pair convention, mock-adapter/no-external-service offline-first pattern)
  - phase: 34-documentation-currency-audit
    provides: 34-AUDIT.md sec4 (EX-71..EX-76 capability rows)
provides:
  - examples/human_in_the_loop_gate.rs -- NodeSpec::Gate pause, WarEngine::resume_with
    typed total-validation rejection of an unrelated parley id, ChronicleService history
    read-back, and WarEngine::replay onto a new branch resumed with the opposite decision
  - examples/graceful_shutdown.rs -- ShutdownCoordinator::register/cancel_and_wait draining
    a fan-out of a fast and a slow node in one Halted checkpoint,
    APP_ENGINE_SHUTDOWN_GRACE_SECS bounding the abort at an overridden value, and
    APP_ENGINE_GRACEFUL_SHUTDOWN's exit-immediately vs. wait-and-drain contrast
  - 36-evidence/36-07-examples.txt -- run output, acceptance-criteria greps, and the
    D-24 closure table for all six EX IDs this plan closes
affects: [36-11]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A capability cluster with no existing runnable program gets one dedicated,
      numbered-parts example whose stdout narrates each capability in the order the
      audit row lists it -- proven twice more (three HITL rows, three graceful-shutdown
      rows) on top of 36-01/36-06's precedent (D-15)."
    - "WarEngine::replay/fork never populate a Gate's ctx.parley_response -- only
      resume_with does -- so replaying from an AwaitingInput Waypoint re-dispatches the
      Gate's own first-visit path and raises a brand-new parley id on the new branch.
      Resuming that branch with a DIFFERENT decision than the mainline's is what
      produces a genuinely divergent result, not a fork edit (which would require the
      fork point to precede the routing decision, not the Gate's own resolution)."
    - "A shutdown demo fans out a fast node and a slow node from one entry so the SAME
      Halted Waypoint's `completed` records show both NodeOutcomeKind::Succeeded and
      NodeOutcomeKind::Skipped { reason: \"shutdown\" } side by side, rather than only
      asserting the run halted."

key-files:
  created:
    - examples/human_in_the_loop_gate.rs
    - examples/graceful_shutdown.rs
    - .planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-07-examples.txt
  modified: []

key-decisions:
  - "Designed the Gate example's 'divergent result' demonstration as replay-then-
    resume-with-the-opposite-decision on a new branch, not fork-with-an-edit. A Gate
    node has no `run` body that reads pre-existing state (`ctx.parley_response()` alone
    decides first-visit vs. post-resume), so an edited battlefield field before the
    fork point is inert for a Gate's own dispatch; a fork/replay from the pause point
    instead raises a fresh parley on the new branch, which this example resumes with
    `false` where the mainline was resumed with `true`, producing a genuinely different
    `path` field value on each branch -- verified by actually running the program
    (confirmed empirically, not assumed from the plan's own README-style description)."
  - "The 'typed rejection of an incomplete resume' truth is demonstrated by submitting a
    response naming a fresh, unrelated ParleyId rather than an empty responses vec.
    WarEngine::resume_with's own doc confirms a genuinely EMPTY submission when one
    parley is outstanding is a valid PARTIAL submission (Ok(RunOutcome::AwaitingInput)),
    not an error -- so the plan's 'omits the real awaited response... typed error'
    language is satisfied by withholding the real response and substituting an unknown
    parley id, which resume_with's total-validation rejects with
    EngineError::UnknownParleyId before any state changes."
  - "graceful_shutdown.rs mirrors paladin-server.rs's own graceful_shutdown ?
    shutdown_grace_secs : 0 derivation in Part 3 so the toggle's documented contrast
    (exit immediately vs. wait for in-flight work) is demonstrated through the same
    ShutdownCoordinator API Part 1 used, rather than only printing the two config
    values before/after."

requirements-completed: [CURR-13, CURR-14, CURR-15]

coverage:
  - id: D1
    description: "human_in_the_loop_gate.rs demonstrates a run pausing at a Gate node, a typed resume_with total-validation rejection of an unrelated parley id followed by the correct response completing the run, and a ChronicleService history read-back plus a replay onto a new branch resumed with the opposite decision to a divergent result (EX-71, EX-72, EX-73)"
    requirement: "CURR-13"
    verification:
      - kind: other
        ref: "env -u OPENAI_API_KEY -u ANTHROPIC_API_KEY -u DEEPSEEK_API_KEY cargo run --example human_in_the_loop_gate (exit 0); stdout inspected for the pause, the typed rejection, the completion, the chronicle history, and the divergent branch result"
        status: pass
    human_judgment: false
  - id: D2
    description: "graceful_shutdown.rs demonstrates ShutdownCoordinator draining a fan-out of in-flight work (one node finishing, one aborted and recorded Skipped in the same Halted checkpoint), the APP_ENGINE_SHUTDOWN_GRACE_SECS override bounding a real run's abort at the new value, and the APP_ENGINE_GRACEFUL_SHUTDOWN toggle's exit-immediately vs. wait-and-drain contrast, all without installing a real signal handler (EX-74, EX-75, EX-76)"
    requirement: "CURR-14"
    verification:
      - kind: other
        ref: "timeout 120 env -u OPENAI_API_KEY -u ANTHROPIC_API_KEY -u DEEPSEEK_API_KEY cargo run --example graceful_shutdown (exit 0, ~1.4s wall time); stdout inspected for the drain report, the grace-period before/after and bounded-abort timing, and the toggle contrast"
        status: pass
    human_judgment: false
  - id: D3
    description: "Both programs are default-feature targets covered by the bulk cargo build --examples selector, neither commit touches src/ or crates/, and make api-surface reports the surface unchanged"
    requirement: "CURR-15"
    verification:
      - kind: other
        ref: "cargo build --examples (exit 0, both binaries present); git diff --stat -- src/ crates/ (empty); ./scripts/check-api-surface.sh .project/current-exports.txt (API surface unchanged, 3959 items)"
        status: pass
    human_judgment: false

# Metrics
duration: ~55min
completed: 2026-09-17
status: complete
---

# Phase 36 Plan 07: Human-in-the-Loop Gate & Graceful Shutdown Examples Summary

**Two new offline example binaries close six of Phase 34's fifty-nine documentation gap rows: human-in-the-loop gating/resume/replay (EX-71, EX-72, EX-73) and graceful shutdown drain/grace/toggle (EX-74, EX-75, EX-76).**

## Performance

- **Duration:** ~55 min
- **Completed:** 2026-09-17T21:26:08Z
- **Tasks:** 2
- **Files modified:** 3 (2 new example binaries, 1 new evidence file)

## Accomplishments
- `examples/human_in_the_loop_gate.rs`: builds a `WarGraph` whose entry point is a
  `NodeSpec::Gate` (Approval) with two `Contains`-conditioned edges to `act`/`cancel`
  branches; runs it to `RunOutcome::AwaitingInput` and prints the thread id, node,
  prompt and parley id; attempts `WarEngine::resume_with` naming a fresh, unrelated
  `ParleyId` and prints the resulting `EngineError::UnknownParleyId` typed rejection;
  submits the correct response and prints the completed `path` result; constructs a
  `ChronicleService` over the same store and prints its `history` read-back
  newest-first; then calls `WarEngine::replay` from the original pause Waypoint,
  showing the new branch raises a fresh parley id (a Gate has no run body that carries
  a prior response across a replay/fork boundary), resumes that branch with the
  opposite decision, and prints the divergent `path` result against the mainline's.
- `examples/graceful_shutdown.rs`: fans out a fast and a slow `Function` node from one
  entry, registers the run with a `ShutdownCoordinator`, triggers
  `cancel_and_wait` directly (never a real signal handler, naming in a header comment
  that a deployed server drives the same coordinator from its own SIGTERM/SIGINT
  handler), and prints the drain outcome plus the Halted Waypoint's `completed` records
  showing `Succeeded` beside `Skipped { reason: "shutdown" }` in the same checkpoint;
  overrides `APP_ENGINE_SHUTDOWN_GRACE_SECS` and re-runs the drain with the new value
  wired into `WarEngine::with_shutdown_grace`, measuring wall-clock elapsed time to
  prove the abort happened at the overridden bound, not the unmodified default; and
  toggles `APP_ENGINE_GRACEFUL_SHUTDOWN` off then on, mirroring the production
  `graceful_shutdown ? shutdown_grace_secs : 0` derivation to make the documented
  exit-immediately-vs-wait contrast concretely observable through the same
  `ShutdownCoordinator` API.
- Both binaries are default-feature targets (no `required-features` manifest entry
  needed) picked up by the bulk `cargo build --examples` selector, run to exit 0 with
  all three provider-key environment variables unset (`graceful_shutdown` terminates on
  its own in ~1.4s, well inside its 120s bound), and neither commit touches `src/` or
  `crates/` (`./scripts/check-api-surface.sh` reports the surface unchanged, 3959 items
  both before and after).

## Task Commits

Each task was committed atomically:

1. **Task 1: examples/human_in_the_loop_gate.rs (EX-71, EX-72, EX-73)** - `fdbd57a5` (docs)
2. **Task 2: examples/graceful_shutdown.rs (EX-74, EX-75, EX-76)** - `da71e046` (docs)

**Plan metadata:** _pending -- this SUMMARY's own commit_

## Files Created/Modified
- `examples/human_in_the_loop_gate.rs` - Gate pause, typed resume_with rejection/acceptance, chronicle read-back, replay-onto-new-branch divergence
- `examples/graceful_shutdown.rs` - ShutdownCoordinator drain of fan-out work, grace-period override, graceful-shutdown toggle contrast
- `.planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-07-examples.txt` - run output, acceptance-criteria grep results, D-24 closure table

## Closure Table (D-24)

| ID | capability | program | commit |
|---|---|---|---|
| EX-71 | Pause at a Gate | examples/human_in_the_loop_gate.rs | fdbd57a5 |
| EX-72 | Resume with typed responses (total validation) | examples/human_in_the_loop_gate.rs | fdbd57a5 |
| EX-73 | Replay the thread onto a new branch | examples/human_in_the_loop_gate.rs | fdbd57a5 |
| EX-74 | Drain in-flight work | examples/graceful_shutdown.rs | da71e046 |
| EX-75 | Configure the grace period from the environment | examples/graceful_shutdown.rs | da71e046 |
| EX-76 | Toggle graceful shutdown off and on | examples/graceful_shutdown.rs | da71e046 |

## Decisions Made

- Designed the Gate example's "divergent result" demonstration as replay-then-resume-
  with-the-opposite-decision on a new branch, not fork-with-an-edit. A `Gate` node has
  no `run` body of its own that reads pre-existing battlefield state
  (`ctx.parley_response()` alone decides first-visit vs. post-resume), so an edited
  battlefield field before the fork point would be inert for a Gate's own dispatch; a
  fork/replay from the pause point instead re-raises a fresh parley on the new branch
  (confirmed by reading `superstep.rs`'s `GateDispatchNode::run` and by actually running
  the program), which this example resumes with `false` where the mainline was resumed
  with `true`, producing a genuinely different `path` result on each branch.
- The "typed rejection of an incomplete resume" truth is demonstrated by submitting a
  response naming a fresh, unrelated `ParleyId` rather than an empty `responses` vec.
  `WarEngine::resume_with`'s own doc confirms a genuinely empty submission when exactly
  one parley is outstanding is a valid PARTIAL submission
  (`Ok(RunOutcome::AwaitingInput)`), not an error -- so the plan's "omits the real
  awaited response... typed error" language is satisfied by withholding the real
  response and substituting an unknown parley id instead, which `resume_with`'s total
  validation rejects with `EngineError::UnknownParleyId` before any state changes.
- `graceful_shutdown.rs`'s Part 3 mirrors `paladin-server.rs`'s own
  `graceful_shutdown ? shutdown_grace_secs : 0` derivation so the toggle's documented
  contrast (exit immediately vs. wait for in-flight work) is demonstrated through the
  same `ShutdownCoordinator` API Part 1 used, rather than only printing the two config
  values before and after -- chosen under Rule 2 (auto-add missing critical
  functionality: making the plan's own must-have independently verifiable from stdout).

## Deviations from Plan

None - plan executed exactly as written. The two "Decisions Made" items above about
*how* to demonstrate EX-73's divergence and EX-72's typed rejection are Claude's
discretion within the plan's own stated intent (the plan names the outcome to show, not
the exact API call sequence), not a deviation from it -- mirroring 36-06's own precedent
for this kind of within-scope design choice.

### Auto-fixed Issues

None - both programs ran correctly on the first attempt after each was fully written;
no bugs, missing functionality, or blocking issues were discovered during execution.

---

**Total deviations:** 0
**Impact on plan:** None.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Sixteen of the fifty-nine Phase 34 audit gap rows are now closed across plans 36-01
  and 36-06/36-07 (EX-62 through EX-76, plus EX-80 and the Commissary cluster). Plan
  36-11, which owns `examples/README.md`, still needs to add a section for each of
  these two new programs -- no README edit was made here per this plan's own scope
  note (README-writing is plan 36-11's job, per the plan's own Output section).
- No blockers for subsequent Phase 36 plans.

---
*Phase: 36-rustdoc-zero-warning-bar-examples-currency*
*Completed: 2026-09-17*

## Self-Check: PASSED

- FOUND: examples/human_in_the_loop_gate.rs
- FOUND: examples/graceful_shutdown.rs
- FOUND: .planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-07-examples.txt
- FOUND: .planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-07-SUMMARY.md
- FOUND commit: fdbd57a5
- FOUND commit: da71e046
