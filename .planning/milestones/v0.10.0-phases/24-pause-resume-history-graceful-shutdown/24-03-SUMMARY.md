---
phase: 24-pause-resume-history-graceful-shutdown
plan: 03
subsystem: infra
tags: [rust, input-mapping, directive-parser, hitl, superstep-engine]

# Dependency graph
requires:
  - phase: 24-pause-resume-history-graceful-shutdown
    provides: "Plan 24-01's real Parley suspend/resume spine (NextStep::Parley, WarEngine::resume_with, NodeContext.parley_response, ParleyRequest/ParleyResponse/ParleyKind/OnExpire value types) and Plan 24-02's NodeSpec::Gate + validate_parley_value_for_kind/normalize_approval_value shared validators"
provides:
  - "InputMapping::render's third parameter, Option<&ParleyResponse> (HITL-01, D-07)"
  - "The `parley.` InputMapping namespace: parley.value/prompt/kind/responded_by, resolved ONLY from NodeContext, never the Battlefield"
  - "ParleyResponse gains kind/prompt fields, engine-stamped by WarEngine::resume_with from the matching ParleyRequest regardless of caller input (mirrors ParleyRequest.node_id's own stamped-regardless contract)"
  - "WarGraph::validate_parley_prefix_schema_fields + EngineError::ParleyPrefixSchemaField -- the parley. namespace reservation, mirroring the muster. rule (23 D-15)"
  - "DirectiveParser's structured envelope next.parley key: a Paladin node raises a parley through its own raw output, not just a declarative NodeSpec::Gate"
  - "build_parley_request / envelope_next_step_to_next_step in directive_parser.rs, reusing graph::validate_parley_value_for_kind for on_expire: ResumeWithDefault raise-time validation (T-24-06)"
affects: [24-04, 24-05, 24-12]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A response-shaped value type (ParleyResponse) gains engine-stamped fields (kind, prompt) that mirror data already recorded on its own originating request, populated at the exact point the engine re-keys external input by node id -- never trusted from an external caller, exactly like ParleyRequest.node_id's own engine-stamped-regardless contract (24-01)"
    - "A semantically-invalid-but-successfully-extracted envelope value (on_expire: ResumeWithDefault) fails as a hard error from DirectiveParser::parse regardless of OnParseError, which governs only EXTRACTION failure -- a validation failure is never routed through the fallback-degrade policy"

key-files:
  created: []
  modified:
    - crates/paladin-battalion/src/engine/input_mapping.rs
    - crates/paladin-battalion/src/engine/graph.rs
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/src/engine/superstep.rs
    - crates/paladin-battalion/src/engine/directive_parser.rs
    - crates/paladin-battalion/src/llm_decision.rs
    - crates/paladin-core/src/platform/container/parley.rs
    - crates/paladin-core/src/platform/container/waypoint.rs

key-decisions:
  - "ParleyResponse (paladin-core) gains kind/prompt fields rather than threading a separate ParleyRequest through NodeContext: InputMapping::render's third parameter is literally Option<&ParleyResponse> per the plan's own action text, so all four parley.* keys (value, prompt, kind, responded_by) must resolve from that ONE type. WarEngine::resume_with stamps kind/prompt from the matching request before the response ever reaches NodeContext -- an external caller's own kind/prompt input is always discarded, never trusted."
  - "kind is rendered via ParleyKind's Debug impl (format!(\"{:?}\", kind) -> \"Approval\"/\"Choice\"/\"FreeText\"/\"StateEdit\") rather than adding a Display impl -- ParleyKind has no existing Display, and Debug's derived output already matches the exact PascalCase spelling used elsewhere (e.g. graph.rs error messages)."
  - "The envelope's next.parley entry stamps a placeholder node_id (a literal string, not NodeId::new(\"\")) onto the constructed ParleyRequest -- the superstep engine's suspension arm (24-01) unconditionally overwrites it with the real dispatching node's id, so this value is provably never observed; pinned by the round-trip test asserting the FINAL request.node_id equals the graph's own node id, not the parser's placeholder."
  - "envelope_to_directive and the private From<EnvelopeNextStep> impl became a fallible free function (envelope_next_step_to_next_step) rather than staying an infallible trait impl -- ONLY the Parley variant can fail (T-24-06's raise-time validation), every other variant conversion stays infallible inside the same match."

patterns-established:
  - "Pattern: when a shared per-kind validator already exists for a graph-validate-time check (validate_parley_value_for_kind, 24-02), a new raise-time check on a different code path imports and calls the SAME function rather than duplicating the validation logic -- one validator, multiple call sites, never two independently-drifting checks for the same invariant."

requirements-completed: []

coverage:
  - id: D1
    description: "InputMapping::render resolves parley.value/prompt/kind/responded_by from NodeContext's ParleyResponse, never the Battlefield, even when a schema field shares the name"
    requirement: "HITL-01"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/input_mapping.rs#parley_namespace_resolves_from_node_context"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/input_mapping.rs#parley_namespace_never_reads_battlefield"
        status: pass
    human_judgment: false
  - id: D2
    description: "A parley-namespaced placeholder with no parley context, or an unrecognized key, is the typed InputMappingError::UndeclaredField, never a silent Battlefield fallthrough or empty substitution"
    requirement: "HITL-01"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/input_mapping.rs#parley_namespace_without_context_is_typed_error"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/input_mapping.rs#parley_namespace_unknown_key_is_typed_error"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/input_mapping.rs#responded_by_none_renders_empty_for_defaulted_response"
        status: pass
    human_judgment: false
  - id: D3
    description: "WarGraph::validate rejects any schema field whose name starts with the parley. prefix, mirroring the muster. rule"
    requirement: "HITL-01"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#schema_field_with_parley_prefix_is_rejected_by_validate"
        status: pass
    human_judgment: false
  - id: D4
    description: "A Paladin node raises a parley through the structured directive envelope's next.parley key: kind/prompt required, payload/choices/expires_in_secs/on_expire optional; the parser stamps parley_id/node_id/created_at and computes expires_at"
    requirement: "HITL-01"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/directive_parser.rs#envelope_parley_key_parses_to_next_step_parley"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/directive_parser.rs#envelope_parley_stamps_expires_at_from_expires_in_secs"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/directive_parser.rs#envelope_parley_defaults_on_expire_to_fail_run"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#paladin_node_parley_round_trips_to_awaiting_input"
        status: pass
    human_judgment: false
  - id: D5
    description: "An on_expire: ResumeWithDefault value that fails validate_parley_value_for_kind is a hard DirectiveParseError at raise time, regardless of on_parse_error; a malformed parley shape routes through the existing OnParseError policy instead"
    requirement: "HITL-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/directive_parser.rs#envelope_parley_resume_with_default_is_validated_at_raise_time"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/directive_parser.rs#envelope_parley_malformed_shape_uses_on_parse_error_policy"
        status: pass
    human_judgment: false

duration: ~100min
completed: 2026-09-05
status: complete
---

# Phase 24 Plan 03: Parley InputMapping Namespace and Envelope-Raised Parleys Summary

**`InputMapping::render` gains a `parley.` namespace resolving `value`/`prompt`/`kind`/`responded_by` from a Battlefield-unshadowable `NodeContext.ParleyResponse`, and a Paladin node can now raise a parley through the structured directive envelope's `next.parley` key, sharing the same raise-time validator a `NodeSpec::Gate`'s `on_expire` default uses.**

## Performance

- **Duration:** ~100 min
- **Tasks:** 2 (both TDD RED/GREEN pairs)
- **Files modified:** 8 (0 created)

## Accomplishments

- `InputMapping::render` gains a third parameter, `Option<&ParleyResponse>`, alongside the existing `muster` parameter; a new `resolve_parley` mirrors `resolve_muster`'s shape exactly and resolves `parley.value`/`parley.prompt`/`parley.kind`/`parley.responded_by` from it, NEVER from the Battlefield, even when a schema field shares the name.
- `ParleyResponse` (`paladin-core`) gains `kind`/`prompt` fields, stamped by `WarEngine::resume_with` from the matching `ParleyRequest` regardless of what an external caller supplies -- mirroring `ParleyRequest.node_id`'s own engine-stamped-regardless contract from plan 24-01. This is what lets all four `parley.*` keys resolve from one type, with `responded_by` rendering an empty string (not a panic) for a D-12 `OnExpire::ResumeWithDefault` substitution (`responded_by: None`).
- `WarGraph::validate` gains `validate_parley_prefix_schema_fields`, mirroring `validate_muster_prefix_schema_fields` verbatim (23 D-15's second namespace): a schema field named with the `parley.` prefix is rejected with the new `EngineError::ParleyPrefixSchemaField` (T-24-09).
- Every `InputMapping::render` call site in `paladin-battalion` audited by name (4 total, none in a test file): `GateDispatchNode`'s two first-visit template renders (`superstep.rs`) pass `parley: None` (no context exists on the raising visit); the `NodeSpec::Paladin` dispatch (`execute_vanguard_node`, `superstep.rs`) threads the real `ctx.parley_response()` (the one path both a Paladin's first-raising visit and its post-resume re-run share); `LlmDecision::render_prompt` (`llm_decision.rs`, an edge evaluator with no `NodeContext`) passes `None`.
- `DirectiveParser`'s structured envelope gains a `next.parley` key (`EnvelopeParleyRequest`, `build_parley_request`): `kind`/`prompt` required, `payload`/`choices`/`expires_in_secs`/`on_expire` optional. The parser stamps `parley_id` (fresh), a placeholder `node_id` (re-stamped onto the real dispatching node by the superstep engine's suspension arm regardless -- proven, not assumed, by the round-trip test) and `created_at`, and computes `expires_at` from `expires_in_secs`.
- An `on_expire: ResumeWithDefault` value is validated against its own `kind` through `graph::validate_parley_value_for_kind` -- the SAME shared validator plan 24-02's `NodeSpec::Gate` uses at graph-validate time (T-24-06) -- and fails as a hard `DirectiveParseError` returned directly from `parse()`, regardless of `on_parse_error` (which governs only a failure to EXTRACT a valid envelope shape, never a value that extracted successfully but is unsafe to accept). A malformed parley entry instead fails extraction and routes through the existing `OnParseError` policy, never coerced into `NextStep::Edges`.

## Task Commits

1. **Task 1: `parley.` InputMapping namespace, render signature, and the reserved schema prefix** -- TDD RED/GREEN pair:
   - `d068fe25` -- `test(24-03): reproduce parley. InputMapping namespace on not-yet-existing render signature (red)` -- 6 new tests added; `InputMapping::render`'s new third parameter deliberately left absent (RED-STATE MARKER comment), 23 compile errors across `paladin-battalion`.
   - `58da753d` -- `feat(24-03): land parley. InputMapping namespace, render signature, and reserved schema prefix (HITL-01, HITL-02, D-07)` -- restores the third parameter and `resolve_parley`; all tests green.
2. **Task 2: Structured directive envelope raises a parley** -- TDD RED/GREEN pair:
   - `76ba9066` -- `test(24-03): reproduce envelope next.parley on not-yet-wired serde tag (red)` -- 6 new tests added (5 in `directive_parser.rs`, 1 engine-level round-trip test in `mod.rs`); the `EnvelopeNextStep::Parley` variant's serde tag deliberately mismatched (RED-STATE MARKER comment), 4/5 `directive_parser` tests fail on extraction, the round-trip test observes `RunOutcome::Failed` instead of `AwaitingInput`.
   - `9fc8f2e8` -- `feat(24-03): land envelope next.parley -- structured directive raises a parley (HITL-01, HITL-02, D-07)` -- restores the correct serde tag; also fixes a doctest compile error on `ParleyResponse` (missing `ParleyKind` import) caught only by the full `cargo test --workspace --doc` run.

**Plan metadata:** (this commit)

## Files Created/Modified

- `crates/paladin-battalion/src/engine/input_mapping.rs` -- `render`'s third parameter, `resolve_parley`, module-doc `parley.` namespace bullet, 6 new tests.
- `crates/paladin-battalion/src/engine/graph.rs` -- `validate_parley_prefix_schema_fields`, wired into `validate_non_recursive`; 1 new test.
- `crates/paladin-battalion/src/engine/mod.rs` -- `EngineError::ParleyPrefixSchemaField`; `resume_with`'s `kind`/`prompt` stamping onto the response before it seeds `NodeContext`; 7 existing `ParleyResponse` test literals updated with the two new fields; 1 new engine-level test (`paladin_node_parley_round_trips_to_awaiting_input`).
- `crates/paladin-battalion/src/engine/superstep.rs` -- the two Gate-raise `render` call sites updated to pass `parley: None`; the Paladin-dispatch `render` call site updated to thread `ctx.parley_response()`.
- `crates/paladin-battalion/src/engine/directive_parser.rs` -- `EnvelopeParleyRequest`, `EnvelopeNextStep::Parley`, `envelope_next_step_to_next_step`, `build_parley_request`; module-doc envelope shape + `next.parley` contract paragraph; a new doc test showing a Paladin node asking for approval; 5 new tests.
- `crates/paladin-battalion/src/llm_decision.rs` -- its one `render` call site updated to pass `parley: None` (no `NodeContext` in an edge evaluator).
- `crates/paladin-core/src/platform/container/parley.rs` -- `ParleyResponse` gains `kind`/`prompt` fields; doctest + 2 existing tests updated.
- `crates/paladin-core/src/platform/container/waypoint.rs` -- 1 existing `ParleyResponse` test literal updated with the two new fields.

## Decisions Made

- **`ParleyResponse` gains `kind`/`prompt` fields rather than a separate `NodeContext`-only side channel.** Task 1's own action text fixes `InputMapping::render`'s third parameter type as `Option<&ParleyResponse>`; since all four `parley.*` keys must resolve from whatever that parameter carries, `kind`/`prompt` had to land on `ParleyResponse` itself. `WarEngine::resume_with` stamps both from the matching `ParleyRequest` before the response ever reaches `NodeContext`, so an external caller's own `kind`/`prompt` input for these fields is always discarded -- mirrors `ParleyRequest.node_id`'s existing engine-stamped-regardless contract from plan 24-01. This is pre-release (v0.10.0 has not shipped, per 24-01/24-02's own "free now" precedent for the `AwaitingInput` reshape and the fingerprint `v4` bump), so the two new fields are plain required fields, not `#[serde(default)]` -- no released Waypoint predates them.
- **`kind` renders via `ParleyKind`'s `Debug` impl**, not a new `Display` impl -- `ParleyKind` has none today, and the derived `Debug` output (`"Approval"`, `"Choice"`, `"FreeText"`, `"StateEdit"`) already matches the PascalCase spelling used in existing error messages elsewhere in `graph.rs`.
- **The envelope's placeholder `node_id`** is a literal string (`"__directive_parser_placeholder__"`), not `NodeId::new("")` -- avoids any ambiguity with an empty-string special case, and is provably never observed since the superstep engine's suspension arm (24-01) unconditionally overwrites it with the real dispatching node's id; the round-trip test asserts the FINAL `request.node_id` equals the graph's own node id, not the placeholder.
- **`envelope_to_directive` became fallible**, replacing the previous infallible `From<EnvelopeNextStep> for NextStep` with a free function `envelope_next_step_to_next_step` -- ONLY the `Parley` variant's conversion can fail (T-24-06's raise-time validation); every other variant's conversion is unchanged and infallible inside the same `match`.
- **A validation failure on `on_expire: ResumeWithDefault` is a hard error, never routed through `OnParseError`'s `FallbackPlain` degradation** -- `on_parse_error` governs only a failure to EXTRACT a valid envelope shape; degrading a successfully-extracted-but-unsafe value into `FallbackPlain`'s plain-output/`NextStep::Edges` semantics would silently accept an unvalidated default, exactly the T-24-06 bypass this plan's `<threat_model>` requires closing.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `ParleyResponse` (paladin-core) required new fields not in Task 1's `<files>` list**
- **Found during:** Task 1 (implementing `resolve_parley`)
- **Issue:** Task 1's `<files>` list names only `input_mapping.rs`, `graph.rs`, `superstep.rs`. Resolving `parley.prompt`/`parley.kind` from `InputMapping::render`'s third parameter (fixed by the plan's own action text as `Option<&ParleyResponse>`) is impossible without `ParleyResponse` itself carrying that data -- `ParleyResponse` had no `prompt`/`kind` fields before this plan.
- **Fix:** Added `kind: ParleyKind` and `prompt: String` to `ParleyResponse` (`paladin-core/src/platform/container/parley.rs`), stamped by `WarEngine::resume_with` (`paladin-battalion/src/engine/mod.rs`) from the matching request -- both files outside Task 1's declared scope.
- **Files modified:** `crates/paladin-core/src/platform/container/parley.rs`, `crates/paladin-core/src/platform/container/waypoint.rs` (one existing test literal), `crates/paladin-battalion/src/engine/mod.rs` (7 existing `ParleyResponse` test literals + the `resume_with` stamping logic).
- **Verification:** `cargo test -p paladin-ai-core --lib` (450 passed) and `cargo test -p paladin-battalion --lib` (513 passed) both green; `cargo test --workspace --doc` (after the follow-up fix below) confirms the doctest round-trips.
- **Committed in:** `58da753d` (Task 1 GREEN commit)

**2. [Rule 1 - Bug] `ParleyResponse`'s doctest failed to compile after adding the `kind` field**
- **Found during:** Post-Task-2 verification (`cargo test --workspace`, which runs `--doc` targets `-p paladin-battalion --lib`/`-p paladin-ai-core --lib` checks earlier in this plan did not exercise)
- **Issue:** The doctest example on `ParleyResponse` (added in Deviation 1's fix) constructed a literal with `kind: ParleyKind::Approval` but only imported `ParleyId`/`ParleyResponse`, not `ParleyKind` -- `error[E0433]: cannot find type ParleyKind in this scope`.
- **Fix:** Added `ParleyKind` to the doctest's `use` statement.
- **Files modified:** `crates/paladin-core/src/platform/container/parley.rs`
- **Verification:** `cargo test -p paladin-ai-core --doc` (66 passed) and a full `cargo test --workspace --no-fail-fast` re-run (all 40 test binaries green except the pre-existing `e2e_1_crash_resume` flake, see Issues Encountered).
- **Committed in:** `9fc8f2e8` (Task 2 GREEN commit)

---

**Total deviations:** 2 (1 blocking core-type extension necessary for the plan's own fixed API shape, 1 bug fix in a doctest introduced by that same extension). **Impact on plan:** Both are mechanical necessities of the plan's own `Option<&ParleyResponse>` parameter-type instruction; no scope creep, no production-architecture change beyond what Task 1's action text already called for.

## Issues Encountered

- **`cargo test -p <crate> --lib` does not exercise doctests.** Deviation 2 above was caught only by the FULL `cargo test --workspace` run (which runs every target, including `--doc`), not by the narrower `-p paladin-ai-core --lib`/`-p paladin-battalion --lib` checks run immediately after each GREEN commit. Recorded here as a process note: the plan's own `<verification>` block correctly requires `cargo test --workspace` as a distinct, later gate, and this run is exactly what caught it.
- **`e2e_1_crash_resume_matches_control_run_with_no_reexecution` is flaky under full-workspace parallel test contention** -- the exact pre-existing flake documented in `24-02-SUMMARY.md`'s own "Issues Encountered" (a 30-second timeout guard in `tests/integration/e2e_crash_resume_test.rs`, unrelated to Gate/Parley/InputMapping). Observed BOTH ways across three full-workspace runs in this plan: failed once, passed once, and (run a third time with `--no-fail-fast`) passed alongside every other of the 40 test binaries in the workspace. Confirmed unrelated to this plan's changes by running it in isolation (`cargo test -p paladin-ai --test e2e_crash_resume e2e_1_crash_resume_matches_control_run_with_no_reexecution`), which passed in 1.65s, well under its 30s guard.
- **Pre-commit hook skipped (worktree mode).** All 4 commits in this plan used `--no-verify` per the orchestrator's `workflow.worktree_skip_hooks=true` allowance for this run (a cold `cargo clippy --workspace --all-targets --all-features` pre-commit hook exceeds the 2-minute command timeout). `cargo fmt --check` (workspace-wide) and `cargo clippy --workspace --all-targets -- -D warnings` (workspace-wide, zero warnings) were both run and verified clean before this SUMMARY was written, in addition to `cargo test --workspace --no-fail-fast` (all 40 binaries green modulo the documented pre-existing flake).

## User Setup Required

None -- no external service configuration required.

## Note on REQUIREMENTS.md

`requirements-completed` in this SUMMARY's frontmatter is deliberately empty, and `.planning/REQUIREMENTS.md`'s `HITL-01`/`HITL-02` checkboxes were **not** marked complete, following the exact precedent plans 24-01 and 24-02's own SUMMARYs recorded: per the phase's coverage table, HITL-01 needs plans 01, 02, 03 and 05 together; HITL-02 needs plans 01, 03, 04 and 05. This plan lands the Function/Paladin-node half of HITL-01's `InputMapping` delivery contract (D-07's `parley.` namespace) and the envelope-raised-parley half of HITL-01 (a Paladin node raising a parley without a declarative `NodeSpec::Gate`), plus the raise-time validation half of HITL-02's D-12 contract -- but not the richer `resume_with` validation matrix (`ParleyAlreadyAnswered`, `ResponseShapeInvalid`, `ParleyExpired`, partial-answer persistence) that plan 24-04 owns. Whichever later plan is the LAST to land its share of each requirement (24-05 per the coverage table for both) should be the one to run `gsd_run query requirements.mark-complete HITL-01 HITL-02`.

## Next Phase Readiness

- The `parley.` `InputMapping` namespace and the envelope's `next.parley` key are both proven end-to-end: a Paladin node can now raise a parley either declaratively (`NodeSpec::Gate`, plan 24-02) or through its own raw output (this plan), and any re-run node -- Gate or Paladin -- can read the delivered answer through a namespace the Battlefield can never shadow (T-24-09, closed with two independent controls exactly like the `muster.` precedent).
- `validate_parley_value_for_kind` now has THREE call sites sharing one validator (Gate's graph-validate-time default check, 24-02; this plan's envelope raise-time default check; and plan 24-04's forthcoming `resume_with` real-submission check) -- confirming the "one validator, never a second weaker check" design plan 24-02 first established holds under a second consumer.
- No blockers. The `e2e_1_crash_resume` timing flake (Issues Encountered) is pre-existing, unrelated to this plan, and already documented in `24-02-SUMMARY.md`; it does not gate this plan's completion.

## Self-Check: PASSED

All 8 modified files verified present on disk; all 4 commit hashes (`d068fe25`, `58da753d`, `76ba9066`, `9fc8f2e8`) verified present in `git log --oneline --all`.

---
*Phase: 24-pause-resume-history-graceful-shutdown*
*Completed: 2026-09-05*
