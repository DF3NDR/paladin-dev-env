---
phase: 24-pause-resume-history-graceful-shutdown
plan: 04
subsystem: infra
tags: [rust, superstep-engine, resume-validation, human-in-the-loop, expiry]

# Dependency graph
requires:
  - phase: 24-pause-resume-history-graceful-shutdown
    provides: "Plan 24-01's real Parley suspend/resume spine (NextStep::Parley, WarEngine::resume_with happy-path guards, NodeContext.parley_response, ParleyRequest/ParleyResponse/ParleyKind/OnExpire value types); Plan 24-02's graph::validate_parley_value_for_kind/normalize_approval_value shared validators; Plan 24-03's parley.kind/parley.prompt stamping contract on ParleyResponse"
provides:
  - "WarEngine::resume_with's complete D-10 validation matrix: EngineError::ParleyAlreadyAnswered/ResponseShapeInvalid/ParleyExpired, checked totally before any Waypoint write"
  - "D-11 partial-answer persistence: a valid but incomplete submission writes a new AwaitingInput Waypoint at the SAME superstep with responses extended, returning only the still-outstanding parleys"
  - "D-12 lazy expiry evaluated at resume time only: on_expire FailRun persists a Failed Waypoint and fails the call; ResumeWithDefault substitutes a pre-validated default with responded_by: None, defaulted: true"
  - "EngineError::ThreadAlreadyFailed guard in resume_with_options, closing the same class of fallthrough risk RESEARCH.md Pitfall 2 already closed for AwaitingInput"
  - "superstep::build_waypoint/persist_waypoint promoted to pub(crate) so mod.rs's resume_with can construct partial-answer and expiry-Failed Waypoints directly"
affects: [24-05]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "resume_with's expiry scan runs over every OUTSTANDING (not-yet-answered) request independent of what the caller's own responses list names -- the clock alone decides once expires_at has passed, so a late submission for an already-expired parley never rescues or overrides the FailRun/ResumeWithDefault policy's own outcome"
    - "A schema-aware validation layer (StateEdit field-name checking against graph.schema()) is added ONLY at the resume_with call site, on top of the unchanged, schema-oblivious graph::validate_parley_value_for_kind shared by all three call sites -- never a signature change to the shared validator, which stays reserved for the structural (kind-shape) rule alone"

key-files:
  created: []
  modified:
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/src/engine/superstep.rs

key-decisions:
  - "Duplicate-parley-id-within-one-call resolves to (a) both rejected, per the plan's own flagged-assumption resolution: the first response is accepted into the in-call working set before the second is checked, so the second fails ParleyAlreadyAnswered -- never last-wins or first-wins"
  - "Expiry is evaluated over the FULL outstanding-parley set at the top of resume_with, before the caller's submitted responses are even validated -- not scoped to only the parley_ids the caller happened to name. This is what lets a caller resume with an empty response list and still observe a ResumeWithDefault substitution proceed the run, and what makes a FailRun expiry fail the whole call even when the caller's submission never mentions the expired parley_id at all (the expired_and_valid_responses_in_one_submission test)"
  - "A ResumeWithDefault substitution always overrides a late submission for the same parley_id, rather than arbitrating between the two -- the substituted value was already validated at graph-validate/raise time (T-24-06), and this keeps the expiry policy deterministic on the clock alone, matching FailRun's own all-clock semantics"
  - "Added EngineError::ThreadAlreadyFailed and a matching guard in resume_with_options (Rule 2, auto-added, not explicitly named by the plan's own task text): a thread whose latest Waypoint is Failed was previously subject to the generic vanguard-restore fallthrough on a plain resume, which would silently attempt to continue a run the engine already declared over. This is the SAME fallthrough-risk class RESEARCH.md's Pitfall 2 already named and fixed for AwaitingInput in plan 24-01; a FailRun parley expiry is the first real-world way to reach a Failed status while calling resume_with is a deliberate, expected operation (unlike a Failed run from an ordinary node crash), so the gap needed closing now rather than being left as a silent trap for the very feature this plan adds"
  - "The StateEdit schema-field check (an undeclared field rejects the response, not the run) lives ONLY in a new mod.rs-local fn validate_response_shape, layered on top of the unchanged graph::validate_parley_value_for_kind -- the shared validator's other two call sites (WarGraph::validate's Gate on_expire check, DirectiveParser's raise-time on_expire check) have no live WarGraph schema to check a submitted value against at their own call time, so extending the shared function's signature would have forced an artificial parameter on call sites that cannot use it"

patterns-established: []

requirements-completed: []

coverage:
  - id: D1
    description: "resume_with validates every submitted response totally before persisting anything: UnknownParleyId, ParleyAlreadyAnswered (new), ResponseShapeInvalid (new, per-kind), any error leaves the thread suspended with no Waypoint written"
    requirement: "HITL-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#resume_with_rejects_unknown_parley_id"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#resume_with_rejects_already_answered_parley"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#resume_with_rejects_wrong_shape_per_kind"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#state_edit_unknown_schema_field_rejects_the_response_not_the_run"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#resume_with_validation_is_total_before_any_write"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#resume_with_checks_graph_fingerprint"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#resume_with_rejects_non_awaiting_input_thread"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#resume_with_parley_ids_are_scoped_to_the_requested_thread"
        status: pass
    human_judgment: false
  - id: D2
    description: "A valid but partial submission persists a new AwaitingInput Waypoint at the SAME superstep with responses extended; answering the last outstanding parley advances the run instead of writing another AwaitingInput Waypoint; a partially-answered suspension is queryable from a cold store handle; re-submission after a simulated save failure is safe (durable consumption); the partial-answer chain is linear (parent_waypoint_id)"
    requirement: "HITL-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#partial_answer_persists_new_awaiting_input_at_same_superstep"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#partial_answer_returns_only_remaining_parleys"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#answering_the_last_parley_advances_the_run"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#partial_answer_state_is_queryable_from_the_waypoint_alone"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#resubmitting_responses_after_a_failed_save_is_safe"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#chain_of_partial_answers_is_linear"
        status: pass
    human_judgment: false
  - id: D3
    description: "Lazy expiry evaluated at resume time only: FailRun persists a Failed Waypoint and fails the call, and the thread is thereafter unresumable via resume/resume_with; ResumeWithDefault substitutes a pre-validated default carrying responded_by: None and defaulted: true and lets the run proceed; a future expires_at is not treated as expired; the defaulted marker survives a serde round trip; an expired FailRun parley fails the whole submission even when unrelated to the caller's own responses"
    requirement: "HITL-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#expired_parley_with_fail_run_persists_failed_waypoint"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#expired_fail_run_thread_is_not_resumable"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#expired_parley_with_resume_with_default_substitutes_value"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#expiry_is_evaluated_only_at_resume_time"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#defaulted_marker_is_persisted_and_queryable"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#expired_and_valid_responses_in_one_submission"
        status: pass
    human_judgment: false

duration: ~130min
completed: 2026-09-05
status: complete
---

# Phase 24 Plan 04: resume_with Validation Matrix, Partial Answers and Lazy Expiry Summary

**`WarEngine::resume_with` now validates every submitted response totally before writing anything (unknown-id/already-answered/shape-invalid/expired), persists a valid-but-partial submission as a same-superstep `AwaitingInput` Waypoint chain, and evaluates `on_expire` lazily at resume time with no timer.**

## Performance

- **Duration:** ~130 min
- **Tasks:** 3 (all TDD RED/GREEN, merged into one plan-level RED/GREEN pair — see Deviations)
- **Files modified:** 2 (0 created)

## Accomplishments

- `EngineError` gains three new `#[non_exhaustive]`-safe variants — `ParleyAlreadyAnswered { parley_id }`, `ResponseShapeInvalid { parley_id, reason }`, `ParleyExpired { parley_id, expires_at }` — completing the D-10 validation matrix plan 24-01's happy-path guards (`UnknownParleyId`, `ThreadNotAwaitingInput`) were designed not to preclude.
- `resume_with`'s new order: load `latest(thread)` → `GraphMismatch` check → `AwaitingInput` status check → `graph.validate(...)` → lazy expiry scan over every outstanding (not-yet-answered) request → total per-response validation (`UnknownParleyId` → `ParleyAlreadyAnswered` → `ResponseShapeInvalid`) → partial-vs-complete branch. Any error before the final branch leaves the thread suspended with no Waypoint written, except the `Failed` Waypoint a `FailRun` expiry itself persists as part of its own policy.
- Per-kind shape validation routes through the SAME `graph::validate_parley_value_for_kind` shared validator a Gate's `on_expire` default (24-02) and a Directive's raise-time default (24-03) already call (T-24-06, now three call sites) — plus a NEW, resume-only schema check layered on top for `ParleyKind::StateEdit`: the deserialised `StateDelta`'s field names are checked against `graph.schema()`, so an undeclared field rejects the response, never the run and never a partial edit (T-24-13), never reaching `Battlefield::merge`'s own `UnknownField` error.
- D-11's partial-answer path: when the accepted responses (existing + this call's validated ones) do not cover every outstanding parley, `resume_with` persists a new `AwaitingInput` Waypoint at the **same** superstep (`parent_waypoint_id` pointing at the Waypoint just read, `parleys` unchanged, `responses` extended, `vanguard` unchanged) and returns `RunOutcome::AwaitingInput` naming only the remaining parleys — mirroring D-14's mid-muster progress-Waypoint precedent exactly, via the SAME `build_waypoint`/`persist_waypoint` construction path (now `pub(crate)` in `superstep.rs`). Once every parley is answered, the SAME call proceeds directly into the resume superstep instead of writing a redundant `AwaitingInput` Waypoint.
- D-12's lazy expiry: evaluated only inside `resume_with`, against `Utc::now()`, over the full outstanding-parley set — independent of whether the caller's own `responses` even name the expired parley. `on_expire: FailRun` persists a `Failed` Waypoint (reason naming the parley and node) and returns `Err(ParleyExpired)` before any submitted response is even inspected; `on_expire: ResumeWithDefault(v)` substitutes `v` as a response (`responded_by: None`, `defaulted: true`), unconditionally overriding a late submission for the same `parley_id`, and flows through the ordinary partial-or-complete logic. No timer, no clock abstraction (D-13) — `expires_at` is a plain persisted field compared once, at the top of each `resume_with` call.
- New `EngineError::ThreadAlreadyFailed` guard added to `resume_with_options`, mirroring `ThreadAwaitingInput`'s guard exactly: a thread whose latest Waypoint is `Failed` (the ONLY way to reach that status via `resume_with`, this plan's own FailRun path) is refused by a plain `resume`/`resume_with_options` rather than silently attempting the generic vanguard-restore fallthrough — closing the same fallthrough-risk class RESEARCH.md's Pitfall 2 already fixed for `AwaitingInput`.

## Task Commits

1. **Tasks 1–3 (validation matrix, partial answers, lazy expiry) — one plan-level TDD RED/GREEN pair** (see Deviations for why all three tasks share one RED/GREEN boundary rather than three):
   - `4752a8c2` — `test(24-04): reproduce resume_with validation matrix, partial answers and lazy expiry on not-yet-existing API (red)` — 20 new tests added referencing not-yet-existing `EngineError` variants; crate fails to compile (8 `E0599` "variant not found" errors).
   - `ca6bccf5` — `feat(24-04): land resume_with total validation, partial-answer persistence and lazy expiry (HITL-02, D-10/D-11/D-12)` — the full implementation; all 533 `paladin-battalion` lib tests green, full `cargo test --workspace` green.

**Plan metadata:** (this commit)

## Files Created/Modified

- `crates/paladin-battalion/src/engine/mod.rs` — three new `EngineError` variants (`ParleyAlreadyAnswered`, `ResponseShapeInvalid`, `ParleyExpired`) plus `ThreadAlreadyFailed`; the `Failed`-status guard in `resume_with_options`; `resume_with`'s complete rewrite (expiry scan, total validation, partial/complete branching); the new `validate_response_shape` free function; 20 new tests plus one pre-existing 24-01 test fixture fix (`"approved"` → `"approve"`, the only string in the accepted Approval vocabulary that fixture could have used once shape validation is real).
- `crates/paladin-battalion/src/engine/superstep.rs` — `build_waypoint` and `persist_waypoint` promoted from private `fn` to `pub(crate)` so `mod.rs`'s `resume_with` can construct and persist partial-answer/expiry-`Failed` Waypoints directly, reusing the exact same construction path D-14's mid-muster progress Waypoints already established rather than inventing a new one.

## Decisions Made

- **Duplicate-parley-id-within-one-call resolves to "both rejected"** (the plan's own flagged, planner-resolved reading of the unclassified "review manually" edge probe): the first response in submission order is accepted into an in-call working set before the second is checked, so the second fails `ParleyAlreadyAnswered`. Reversibility: cheap to change later (an internal duplicate-detection rule, not a stored contract) if a developer prefers last-wins or first-wins.
- **Expiry is scanned over the FULL outstanding-parley set, not scoped to the caller's own submitted `parley_id`s.** This is what lets `resume_with(graph, thread, Vec::new())` alone trigger a `ResumeWithDefault` substitution and advance the run (`expired_parley_with_resume_with_default_substitutes_value`), and what makes a `FailRun` expiry fail the WHOLE call even when the caller's submission never references the expired parley at all (`expired_and_valid_responses_in_one_submission`) — matching D-12's "evaluated lazily at resume time" contract literally: every `resume_with` call is a checkpoint against the clock for every outstanding request, not just the ones a caller happens to be answering.
- **A `ResumeWithDefault` substitution always overrides a late submission for the same `parley_id`**, rather than arbitrating between the two. Not explicitly specified by D-12's prose, but the only choice that keeps the policy deterministic on the clock alone (mirroring `FailRun`'s own all-clock semantics) and never lets an attacker-timed late submission race a pre-authored, pre-validated default.
- **[Rule 2 — auto-added] `EngineError::ThreadAlreadyFailed` and its `resume_with_options` guard.** Not named by any of Task 1/2/3's own action text, but required for correctness: this plan is the first to make a `Failed` Waypoint reachable via a NORMAL, expected `resume_with` call path (a `FailRun` expiry) rather than only an unusual node-crash scenario. Without the guard, a plain `resume` against that thread would fall through to the generic vanguard-restore path and silently attempt to re-run the parleying node as ordinary work — exactly the class of bug RESEARCH.md's Pitfall 2 already named and fixed for `AwaitingInput` in plan 24-01. Verified via `expired_fail_run_thread_is_not_resumable`; confirmed via `grep` that no existing test anywhere in the workspace resumes a thread whose latest Waypoint is genuinely `Failed`, so this guard changes no prior passing behavior.
- **The StateEdit schema-field check stays local to `mod.rs`'s new `validate_response_shape`, never added to `graph::validate_parley_value_for_kind`'s own signature.** That shared function's other two call sites (`WarGraph::validate`'s Gate `on_expire` check, `DirectiveParser`'s raise-time `on_expire` check) validate an AUTHORED default value with no live `WarGraph` schema in scope at their own call time — extending the shared signature to carry an optional schema parameter neither of those callers can supply would have been a needless complication of the one function this codebase is careful to keep as "one validator, multiple call sites, never a second weaker check."

## Deviations from Plan

### Architectural / process note (not a Rule 1–4 auto-fix)

**1. Tasks 1, 2 and 3 share ONE RED/GREEN commit pair instead of three, because they modify the SAME function's SAME control flow in sequence**

- **Found during:** Planning the commit boundaries after drafting all three tasks' tests together.
- **Issue:** Task 1's validation matrix, Task 2's partial-answer branch, and Task 3's expiry scan are not three independently-landable slices of `resume_with` — each later task's GREEN state requires the earlier task's own GREEN code already in place (Task 2's "remaining" computation needs Task 1's total-validation loop to have already accepted/rejected every response; Task 3's expiry-substituted responses need to flow through Task 2's partial/complete branch to be tested end-to-end). Splitting into three genuinely separate RED/GREEN pairs would have required either (a) landing intermediate, semantically-incomplete versions of `resume_with` as "GREEN" for Task 1 and Task 2 alone (misleading, since Task 1 alone with no partial-answer branch would still pass Task 1's OWN tests, but that intermediate state was never a real design the plan asked for), or (b) three RED commits interleaved with a single GREEN — which does not match this codebase's own established convention (one RED, one GREEN, verified compiling/passing at each boundary).
- **Resolution:** One RED commit (all 20 tests across all three tasks, referencing the not-yet-existing `EngineError` variants — genuine compile failure, 8 `E0599` errors) followed by ONE GREEN commit landing the complete, final `resume_with` (validation matrix + partial answers + lazy expiry together, since that IS the coherent unit of behavior D-10/D-11/D-12 jointly describe). This mirrors 24-02-SUMMARY.md's own precedent (Deviation 3, "Tasks 1 and 2 share one GREEN commit") for the identical underlying reason: Rust's own compilation model does not allow an intermediate state to be independently green when three tasks all rewrite the same function body in sequence.
- **Verification:** `cargo check -p paladin-battalion --tests` failed with exactly 8 `E0599` errors before the GREEN commit; `cargo test -p paladin-battalion --lib` (533 passed), `cargo test --workspace --no-fail-fast` (all green except the documented pre-existing `e2e_1_crash_resume` flake, see Issues Encountered), `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` (zero warnings) all verified clean after the GREEN commit.
- **Committed in:** `4752a8c2` (RED), `ca6bccf5` (GREEN).

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed a pre-existing 24-01 test fixture that relied on an unvalidated Approval value**

- **Found during:** Post-GREEN full-suite verification (`cargo test -p paladin-battalion --lib`)
- **Issue:** `parley_suspends_and_resumes_end_to_end` (landed in plan 24-01, before any shape validation existed) submitted `serde_json::json!("approved")` as an `Approval` response value and asserted it flowed through to the node's continuation unchanged. `"approved"` is not one of the accepted Approval strings (`true`/`false`/`yes`/`no`/`approve`/`deny`, case-insensitive) this plan's new `ResponseShapeInvalid` check now enforces — the test started failing with exactly the new, correct validation error.
- **Fix:** Changed the submitted value (and the corresponding final-state assertion) from `"approved"` to `"approve"` — a rule-conforming string that preserves the test's actual intent (proving pass-through-to-continuation, not exercising Approval normalisation, which the Gate tests already cover separately).
- **Files modified:** `crates/paladin-battalion/src/engine/mod.rs` (one existing test, no production code)
- **Verification:** `cargo test -p paladin-battalion --lib parley_suspends_and_resumes_end_to_end` passes.
- **Committed in:** `ca6bccf5` (GREEN commit)

**2. [Rule 2 - Missing Critical] `EngineError::ThreadAlreadyFailed` guard in `resume_with_options`**

- Documented above under Decisions Made (its rationale is identical) — recorded here too since it is, formally, an addition beyond any task's own explicit action text.
- **Committed in:** `ca6bccf5` (GREEN commit)

---

**Total deviations:** 1 architectural/process note (commit-boundary merge, mirrors 24-02's own precedent for the identical reason) + 2 auto-fixed (1 bug fix in a pre-existing test fixture, 1 missing-critical-functionality guard). **Impact on plan:** No scope creep — the `ThreadAlreadyFailed` guard is a direct, minimal consequence of this plan's own `FailRun` expiry path making `Failed` a normally-reachable status via `resume_with` for the first time; the test fixture fix is a mechanical necessity of the new (correct) validation this plan is required to add.

## Issues Encountered

- **`e2e_1_crash_resume_matches_control_run_with_no_reexecution` is flaky under full-workspace parallel test contention** — the exact pre-existing flake documented in `24-02-SUMMARY.md`'s and `24-03-SUMMARY.md`'s own "Issues Encountered" sections (a 30-second timeout guard in `tests/integration/e2e_crash_resume_test.rs`, unrelated to `resume_with`/validation/expiry). Observed failing once under `cargo test --workspace` (CPU contention from the full parallel suite), confirmed unrelated to this plan's changes by running it in isolation (`cargo test -p paladin-ai --test e2e_crash_resume e2e_1_crash_resume_matches_control_run_with_no_reexecution`, passed in 1.58s, well under its 30s guard), and confirmed passing alongside every other test in a second full `cargo test --workspace --no-fail-fast` run.
- **Pre-commit hook skipped (worktree mode).** Both commits in this plan used `--no-verify` per the orchestrator's `workflow.worktree_skip_hooks=true` allowance for this run (a cold `cargo clippy --workspace --all-targets --all-features` pre-commit hook exceeds the 2-minute command timeout). `cargo fmt --check` and `cargo clippy -p paladin-battalion --tests -- -D warnings` were verified clean before the GREEN commit; `cargo test --workspace --no-fail-fast` and `cargo clippy --workspace --all-targets -- -D warnings` (zero warnings across the full workspace) were both run and verified green before this SUMMARY was written.

## User Setup Required

None — no external service configuration required.

## Note on REQUIREMENTS.md

`requirements-completed` in this SUMMARY's frontmatter is deliberately empty, and `.planning/REQUIREMENTS.md`'s `HITL-02` checkbox was **not** marked complete, following the exact precedent plans 24-01 and 24-03's own SUMMARYs recorded: per the phase's coverage table, HITL-02 needs plans 01, 03, 04 and 05 together. This plan lands the richer validation-matrix/partial-answer/expiry share of HITL-02 that plans 24-01 and 24-03 explicitly deferred to it, but not the `resume`/`resume_with` behavior plan 24-05 (History/Chronicle, `replay`/`fork`) still owns, per this plan's own Task 3, Test 2 rustdoc ("`replay`/`fork` land in plan 24-07" — later re-scoped to 24-05 per the coverage table cited in 24-01/24-03's SUMMARYs). Whichever plan is the LAST to land its share of HITL-02 (24-05 per the coverage table) should be the one to run `gsd_run query requirements.mark-complete HITL-02`.

## Next Phase Readiness

- `resume_with` is now the complete, trustworthy advancement path D-10/D-11/D-12 specify: total validation before any write, partial answers persisted as a queryable Waypoint chain, and lazy expiry with both `on_expire` policies — no timer, no clock abstraction, matching D-13's explicit constraint.
- `EngineError::ThreadAlreadyFailed` and the `Failed`-status guard in `resume_with_options` are new, general-purpose closes on the fallthrough-risk class RESEARCH.md's Pitfall 2 first named — available to any later plan that reaches a `Failed` Waypoint through a path other than this plan's own `FailRun` expiry.
- No blockers. The `e2e_1_crash_resume` timing flake (Issues Encountered) is pre-existing, unrelated to this plan, and already documented in `24-02-SUMMARY.md`/`24-03-SUMMARY.md`; it does not gate this plan's completion.

## Self-Check: PASSED

Both modified files verified present on disk; both commit hashes (`4752a8c2` RED, `ca6bccf5` GREEN) verified present in `git log --oneline --all`.

---
*Phase: 24-pause-resume-history-graceful-shutdown*
*Completed: 2026-09-05*
