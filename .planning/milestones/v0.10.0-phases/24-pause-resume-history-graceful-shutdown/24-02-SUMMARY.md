---
phase: 24-pause-resume-history-graceful-shutdown
plan: 02
subsystem: infra
tags: [rust, superstep-engine, gate-node, human-in-the-loop, fingerprint]

# Dependency graph
requires:
  - phase: 24-pause-resume-history-graceful-shutdown
    provides: "Plan 24-01's real Parley suspension/resume spine: NextStep::Parley suspension path, WarEngine::resume_with, NodeContext.parley_response, ParleyRequest/ParleyResponse/ParleyKind/OnExpire value types"
provides:
  - "NodeSpec::Gate { request: GateRequestTemplate, output_field } (D-05) — a first-class human-input node with no run body of its own"
  - "GateRequestTemplate fluent builder (kind, prompt_template, payload_template, choices, expires_in, on_expire)"
  - "WarGraph::validate's Gate well-formedness clause (validate_gates): output_field presence/absence by kind, schema existence, type compatibility inferred from the field's schema default, and on_expire ResumeWithDefault validation via a shared per-kind validator (D-12, T-24-06)"
  - "graph::validate_parley_value_for_kind and graph::normalize_approval_value — pub(crate) shared validators reused by superstep.rs's delivery path and reserved for plan 24-04's resume_with validation matrix"
  - "GateDispatchNode in superstep.rs: raises NextStep::Parley on first visit (rendering prompt_template/payload_template through InputMapping), writes the normalised delivered value to output_field on the post-resume visit, or returns a StateEdit response's StateDelta unmerged"
  - "Custom edge-evaluator arm treats a Gate's output_field like a Paladin's (D-06); Contains/Regex need no Gate-specific code"
  - "GRAPH_FINGERPRINT_VERSION v3 -> v4 with a ;gates: section hashing kind/output_field/choices/on_expire-kind (D-09)"
affects: [24-03, 24-04, 24-05, 24-12]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A Gate is dispatched as an ordinary Function node wrapping a fresh GateDispatchNode (Arc<dyn StateNode>), reusing every existing dispatch/interceptor/trace/suspension code path with zero changes to the per-node spawn loop"
    - "A field's 'type' for Gate output_field compatibility checking is inferred from its BattlefieldSchema default value (Bool/String) — the only per-field type signal FieldSpec carries; a field with no default is rejected as unable to be type-checked"
    - "OnExpire's DISCRIMINANT kind (not its ResumeWithDefault payload) is hashed via a dedicated on_expire_kind_tag() helper, never serde_json::to_string(&on_expire) — mirrors the existing edge-condition/dispatch-rule canonicalization precedent but excludes the payload deliberately (ENG-FR-14)"

key-files:
  created: []
  modified:
    - crates/paladin-battalion/src/engine/graph.rs
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/src/engine/superstep.rs
    - crates/paladin-core/src/platform/container/waypoint.rs

key-decisions:
  - "Task 3 checkpoint (bump-to-v4) auto-resolved by the orchestrator under auto-mode: GRAPH_FINGERPRINT_VERSION bumps v3->v4 with a new ;gates: section, following the v1->v2->v3 precedent exactly; every stored v3 Waypoint fingerprint fails closed with GraphMismatch on resume rather than being silently reinterpreted — one-way after v0.10.0 ships, free now"
  - "Gate output_field 'type' is inferred from the field's schema default value, not a new type-declaration mechanism on FieldSpec (BattlefieldSchema carries no separate type field) — a field with no default cannot be type-checked and is rejected; documented as an explicit design tradeoff in gate_output_field_is_type_compatible's rustdoc"
  - "validate_parley_value_for_kind and normalize_approval_value live in graph.rs (not a new paladin-core module) as pub(crate) functions, so plan 24-04's resume_with validation matrix can import and reuse the exact same validator this plan's on_expire ResumeWithDefault check uses — never a second, weaker check (T-24-06)"
  - "A future ParleyKind or OnExpire variant reaching gate_output_field_is_type_compatible / validate_parley_value_for_kind / on_expire_kind_tag's non_exhaustive catch-all arm fails CLOSED (a typed error), not open — mirrors the codebase's existing fail-closed stance for unregistered custom dispatch/edge conditions"

patterns-established:
  - "Pattern: a first-class 'no run body of its own' node is dispatched by constructing a fresh Arc<dyn StateNode> at the per-node dispatch match arm, not by adding a new NodeDispatch enum variant — reuses the entire existing spawn/interceptor/trace/suspension pipeline with a single added match arm"
  - "Pattern: Contains/Regex edge conditions match against the WHOLE serialized Battlefield (schema + values), so a bare true/false needle collides with any OTHER field's serialized required:false — anchor a Contains needle to the full \"field\":value pair, not a bare boolean word, whenever any schema field carries a boolean default or required flag"

requirements-completed: []

coverage:
  - id: D1
    description: "A Gate node raises a ParleyRequest on first visit, rendering prompt/payload from the Battlefield through InputMapping, and stamps expires_at = now + expires_in"
    requirement: "HITL-01"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#gate_raises_parley_on_first_visit"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#gate_stamps_expires_at_from_expires_in"
        status: pass
    human_judgment: false
  - id: D2
    description: "On the post-resume visit a Gate writes the normalised response value to output_field (Bool or String Approval delivery, Choice/FreeText passthrough) or returns a StateEdit delta, then routes via static edges"
    requirement: "HITL-01"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#gate_writes_normalised_approval_value_on_resume"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#gate_writes_string_true_false_for_string_output_field"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#gate_state_edit_returns_delta_and_writes_no_output_field"
        status: pass
    human_judgment: false
  - id: D3
    description: "An approval gate expressed as one Gate node plus Contains(\"true\")/Contains(\"false\") edges routes to the correct branch on approval and denial (the E2E-2 shape)"
    requirement: "HITL-01"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#approval_gate_routes_both_branches"
        status: pass
    human_judgment: false
  - id: D4
    description: "A registered Custom edge evaluator on an edge whose source is a Gate receives the Gate's output_field value, not the whole serialised Battlefield"
    requirement: "HITL-01"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#gate_source_uses_output_field_for_custom_evaluator"
        status: pass
    human_judgment: false
  - id: D5
    description: "WarGraph::validate rejects every invalid Gate wiring combination (output_field required/absent by kind, unknown field, incompatible type, invalid on_expire default) with a distinct typed EngineError variant, and accepts the valid E2E-2 shape"
    requirement: "HITL-01"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#gate_requires_output_field_for_approval_choice_freetext"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#gate_rejects_output_field_for_state_edit"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#gate_output_field_must_exist_in_schema"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#gate_output_field_type_must_be_compatible"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#gate_resume_with_default_value_is_validated_at_graph_validate_time"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#gate_with_valid_wiring_passes_validation"
        status: pass
    human_judgment: false
  - id: D6
    description: "GRAPH_FINGERPRINT_VERSION is v4 and the fingerprint hashes a sorted, length-prefixed ;gates: section over kind/output_field/choices/on_expire-kind, excluding prompt_template/payload_template/expires_in"
    requirement: "HITL-01"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#fingerprint_version_is_v4"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#fingerprint_golden_hex_v4"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#fingerprint_differs_on_gate_kind"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#fingerprint_differs_on_output_field"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#fingerprint_differs_on_choices"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#fingerprint_differs_on_on_expire_kind"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#fingerprint_ignores_gate_templates_and_expiry"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#fingerprint_gate_section_is_length_prefixed"
        status: pass
    human_judgment: false

duration: ~150min
completed: 2026-09-05
status: complete
---

# Phase 24 Plan 02: Gate Node — Battlefield-Templated Human Input Summary

**First-class `NodeSpec::Gate` node (raise-on-first-visit / deliver-on-resume, no run body of its own) with Battlefield-templated prompts/payloads, validated wiring, HITL-FR-05 Approval normalisation, and a re-pinned `v4` graph fingerprint hashing every Gate routing property.**

## Performance

- **Duration:** ~150 min
- **Tasks:** 4 (Task 1 validation, Task 2 dispatch, Task 3 auto-resolved checkpoint, Task 4 fingerprint bump)
- **Files modified:** 4 (0 created)

## Accomplishments

- `NodeSpec::Gate { request: GateRequestTemplate, output_field: Option<FieldName> }` lands on the already-`#[non_exhaustive]` `NodeSpec` enum with a fluent `GateRequestTemplate` builder (`kind`, `prompt_template`, `payload_template`, `choices`, `expires_in`, `on_expire`) and a `NodeSpec::gate(...)` constructor, matching the codebase's `paladin`/`battalion` constructor convention exactly.
- `WarGraph::validate` gains `validate_gates()`: `output_field` is required for `Approval`/`Choice`/`FreeText` and must be `None` for `StateEdit`; when present, the field must exist in the schema with a type compatible with the Gate's `kind` (inferred from the field's schema `default` — `FieldSpec` carries no separate type declaration); an `on_expire: ResumeWithDefault` value is checked against its own `kind` through the shared `validate_parley_value_for_kind` helper. Five new typed `EngineError` variants (`GateOutputFieldRequired`, `GateOutputFieldMustBeAbsent`, `GateOutputFieldUnknown`, `GateOutputFieldTypeIncompatible`, `GateResumeWithDefaultInvalid`) name the offending node and reason — no stringly errors.
- `superstep.rs`'s `GateDispatchNode` implements `StateNode` and is dispatched exactly like an ordinary `Function` node (a single added match arm constructing `Arc::new(GateDispatchNode { .. })`) — reusing every existing spawn/interceptor/trace/`NextStep::Parley`-suspension code path with zero changes to any of it. On first visit it renders `prompt_template`/`payload_template` through `InputMapping` and raises; on the post-resume visit it normalises and writes the delivered value to `output_field` (Bool gets the JSON boolean, String gets `"true"`/`"false"`) or returns a `StateEdit` response's `StateDelta` unmerged, then routes via `NextStep::Edges`.
- The `Custom`-evaluator edge-condition arm gains `| Some(NodeSpec::Gate { output_field: Some(output_field), .. })` on the existing Paladin pattern (D-06) — `Contains`/`Regex` need no Gate-specific code at all.
- `GRAPH_FINGERPRINT_VERSION` bumps `v3` → `v4` (Task 3's checkpoint, auto-resolved `bump-to-v4`): a new `;gates:` section hashes each Gate's `kind`, `output_field`, `choices` and `on_expire` DISCRIMINANT kind (never the `ResumeWithDefault` payload value) through the existing length-prefixed `push_field` helper — never a delimiter join. Golden hex re-pinned, one difference test per hashed property, one exclusion test for `prompt_template`/`payload_template`/`expires_in`, and a dedicated collision test for the `choices` list mirroring the existing CR-01 two-nodes-vs-one-node pattern.

## Task Commits

1. **Task 1: NodeSpec::Gate, GateRequestTemplate and WarGraph::validate rules** — TDD RED/GREEN pair (graph.rs's tests were red against not-yet-existing production code; the crate would not even compile until Task 2's dispatch arm landed, so GREEN for Task 1 alone still left the crate non-compiling — see Deviations):
   - `2f96df5a` — `test(24-02): reproduce Gate validation on not-yet-existing NodeSpec::Gate API (red)`
   - `968ecd98` — `feat(24-02): land NodeSpec::Gate validation and dispatch (HITL-01, D-05/D-06)` (GREEN for both Task 1 and Task 2 together — see Deviations for why)
2. **Task 2: Gate dispatch — raise on first visit, deliver and route on the post-resume visit** — RED landed as its own commit, GREEN merged into Task 1's GREEN commit above:
   - `ba789827` — `test(24-02): reproduce Gate dispatch on not-yet-existing superstep wiring (red)`
   - (GREEN: `968ecd98`, above)
3. **Task 3: Confirm the graph fingerprint version bump** — checkpoint auto-resolved by the orchestrator under auto-mode (`⚡ Auto-selected: bump-to-v4`); no code commit, recorded here.
4. **Task 4: `;gates:` fingerprint section, v4 bump and golden re-pin** — TDD RED/GREEN pair:
   - `0a1385c9` — `test(24-02): reproduce v4 fingerprint bump on not-yet-existing ;gates: section (red)`
   - `f1ecad52` — `feat(24-02): bump graph fingerprint to v4 for Gate routing properties (D-09)`

**Plan metadata:** (this commit)

## Files Created/Modified

- `crates/paladin-battalion/src/engine/graph.rs` — `NodeSpec::Gate`, `GateRequestTemplate` + builder, `NodeSpec::gate()` constructor with doctest, `Debug` impl arm, `validate_gates()` + its typed error paths, `gate_output_field_is_type_compatible`, `normalize_approval_value`, `validate_parley_value_for_kind`, the `;gates:` fingerprint section, `on_expire_kind_tag` helper, 14 new tests (6 validation, 8 fingerprint).
- `crates/paladin-battalion/src/engine/mod.rs` — five new `EngineError` variants (`GateOutputFieldRequired`, `GateOutputFieldMustBeAbsent`, `GateOutputFieldUnknown`, `GateOutputFieldTypeIncompatible`, `GateResumeWithDefaultInvalid`), `FieldName`/`ParleyKind` imports, 7 new Gate-dispatch tests.
- `crates/paladin-battalion/src/engine/superstep.rs` — `GateDispatchNode` (`StateNode` impl), the `NodeSpec::Gate` dispatch match arm, the `Custom`-evaluator edge-condition Gate pattern.
- `crates/paladin-core/src/platform/container/waypoint.rs` — `GRAPH_FINGERPRINT_VERSION` `"v3"` → `"v4"`, version-history rustdoc `v4` paragraph, `as_str()` doc update, one test literal update (`v3:` → `v4:`).

## Decisions Made

- **Task 3 checkpoint auto-resolved** (`bump-to-v4`): bump `GRAPH_FINGERPRINT_VERSION` with a new `;gates:` section following the `v1`→`v2`→`v3` precedent exactly. Reversibility: one-way after v0.10.0 ships; free now (no released Waypoints exist under `v3` outside this dev tree).
- **Gate output_field "type" is inferred from the field's schema `default` value** (`Bool`/`String`), not a new type-declaration field on `FieldSpec` — `BattlefieldSchema` carries no separate type system, and adding one was out of this plan's file scope (Task 1 is `graph.rs`-only). A field declaring no default cannot be type-checked and is rejected by `validate_gates` with a message telling the author to declare a default of the intended type. This is a genuine design tradeoff, documented in `gate_output_field_is_type_compatible`'s own rustdoc.
- **`validate_parley_value_for_kind`/`normalize_approval_value` live in `graph.rs`** (not a new `paladin-core` module) as `pub(crate)` functions — Task 1's own action items explicitly required extracting a shared validator now rather than writing a second, weaker check later; keeping it in the same crate as `WarGraph::validate` (which needs it now) and `WarEngine::resume_with` (plan 24-04, same crate) avoids a premature cross-crate move.
- **A future `ParleyKind`/`OnExpire` variant reaching a `#[non_exhaustive]`-forced catch-all arm fails CLOSED**, not open, in all three of `gate_output_field_is_type_compatible`, `validate_parley_value_for_kind` and `on_expire_kind_tag` — mirrors the codebase's existing fail-closed stance for unregistered custom dispatch/edge conditions, and specifically closes the T-24-06 elevation-of-privilege risk (an unchecked default silently bypassing an approval gate) for any kind added after this plan.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Task 1's five new `EngineError` variants required editing `mod.rs`, not listed in Task 1's `<files>`**
- **Found during:** Task 1 (implementing `validate_gates`)
- **Issue:** `EngineError` is defined exclusively in `crates/paladin-battalion/src/engine/mod.rs` (confirmed via `grep -rn "pub enum EngineError"` — one definition site in the whole crate). Task 1's `<files>` list names only `graph.rs`, but "each producing its own typed EngineError variant" (the task's own action item) is impossible without touching `mod.rs`.
- **Fix:** Added `GateOutputFieldRequired`, `GateOutputFieldMustBeAbsent`, `GateOutputFieldUnknown`, `GateOutputFieldTypeIncompatible`, `GateResumeWithDefaultInvalid` to `mod.rs`'s `EngineError` enum, plus the `FieldName`/`ParleyKind` imports it needed.
- **Files modified:** `crates/paladin-battalion/src/engine/mod.rs`
- **Verification:** `cargo test -p paladin-battalion gate_` (Task 1's six validation tests, all referencing these variants) passes.
- **Committed in:** `968ecd98` (Task 1/2 combined GREEN commit)

**2. [Rule 1 - Bug] `approval_gate_routes_both_branches`'s original `Contains("true")`/`Contains("false")` edges both fired regardless of the delivered value**
- **Found during:** Task 2 (writing the E2E-2-shape routing test)
- **Issue:** `Contains`/`Regex` edge conditions match against `serde_json::to_string(&battlefield)`, which serializes the WHOLE `Battlefield` including its embedded `BattlefieldSchema` — every non-required field's schema entry always contains the literal text `"required":false`. A bare `Contains("false")` needle therefore always matched, independent of the Gate's actual delivered value; empirically confirmed both `act` and `cancel` fired in the same superstep, producing a `DispatchConflict`. This is a pre-existing characteristic of `Contains`/`Regex`'s whole-Battlefield-JSON matching strategy (present since Phase 22), not a defect introduced by `Gate` dispatch itself.
- **Fix:** Anchored the test's edge needles to the full `"approved":true` / `"approved":false` key-value pairs rather than the bare words `true`/`false`, disambiguating from any other boolean-shaped text the serialised schema happens to carry. Documented as a caveat in the test's own rustdoc, generalizable to any real graph author using `Contains` against a boolean-typed field.
- **Files modified:** `crates/paladin-battalion/src/engine/mod.rs` (test only — no production code changed; `Contains`/`Regex` themselves are correctly unmodified per D-06's own instruction that they need no Gate-specific code).
- **Verification:** `cargo test -p paladin-battalion approval_gate_routes_both_branches` — both branches route correctly.
- **Committed in:** `968ecd98` (Task 1/2 combined GREEN commit)

**3. [Rule 4 - Architectural, self-resolved within plan scope] Tasks 1 and 2 share one GREEN commit because `NodeSpec`'s exhaustive match in `superstep.rs` makes Task 1 alone non-compiling**
- **Found during:** Attempting to commit Task 1's GREEN state in isolation
- **Issue:** Adding `NodeSpec::Gate` (Task 1) makes `superstep.rs`'s per-node dispatch `match spec { .. }` (an exhaustive, same-crate match with no wildcard) fail to compile with `E0004: non-exhaustive patterns` until Task 2's dispatch arm is also added. The plan's per-task RED/GREEN split assumed the two tasks could each reach an independently-compiling GREEN state; Rust's exhaustive-match rule for a same-crate enum does not allow that here.
- **Resolution:** Two separate RED commits (one per task's own tests, each independently verified to fail — Task 1's for the missing type/validation API, Task 2's for the missing dispatch wiring), followed by ONE GREEN commit landing both tasks' implementation together, since that is the first point at which the crate compiles and all of both tasks' tests pass. Not a scope change — every line of implementation still traces to its own task; only the commit boundary merged.
- **Verification:** `cargo test -p paladin-battalion` (all 14 Gate-related tests + no regressions in the other 487).
- **Committed in:** `968ecd98`

---

**Total deviations:** 3 (1 blocking mod.rs edit, 1 bug fix in test-only code, 1 self-resolved commit-boundary architectural note). **Impact on plan:** All three are mechanical necessities of Rust's type/module system or genuine test-design bugs; no scope creep, no production-code architecture change.

## Issues Encountered

- **`Contains`/`Regex`'s whole-Battlefield-JSON matching is a pre-existing footgun for boolean-typed fields.** Documented above (Deviation 2) and in the affected test's own rustdoc. Not fixed at the engine level (out of this plan's scope — `Contains`/`Regex` predate Phase 24 and are used elsewhere in the tree unrelated to `Gate`) but worth flagging for the `docs/src/user-guides/parley-and-chronicle.md` mdBook page a later plan (24-12) writes: a graph author routing on a Gate's Bool `output_field` via `Contains` should anchor the needle to `"field":value`, not a bare `true`/`false`.
- **Full-workspace `cargo test --workspace` reported one flaky, unrelated failure** (`e2e_1_crash_resume_matches_control_run_with_no_reexecution`, a 30-second timeout guard in `tests/integration/e2e_crash_resume_test.rs`) when run under the CPU contention of the full parallel test suite. Re-ran in isolation (`cargo test --test e2e_crash_resume e2e_1_crash_resume_matches_control_run_with_no_reexecution`): passed in 3.03s, well under its 30s guard. Confirmed unrelated to this plan's changes (crash-resume, not Gate/fingerprint) and a timing-sensitive pre-existing flake, not a regression.
- **Pre-commit hook timeout (worktree mode).** Every commit in this plan used `--no-verify` per the orchestrator's `workflow.worktree_skip_hooks=true` allowance (a cold `cargo clippy --workspace --all-targets --all-features` pre-commit hook exceeds the 2-minute command timeout). `cargo fmt --check` and `cargo clippy -p paladin-battalion -p paladin-ai-core -- -D warnings` were verified clean before each commit; `cargo test --workspace` (all crates, 523+ tests) and `cargo clippy --workspace --all-targets -- -D warnings` (zero warnings across the full workspace, including `paladin-web`, `paladin-content`, `paladin-notifications`, the root binary) were both run and verified green before this SUMMARY was written.

## User Setup Required

None — no external service configuration required.

## Note on REQUIREMENTS.md

`requirements-completed` in this SUMMARY's frontmatter is deliberately empty, and `.planning/REQUIREMENTS.md`'s `HITL-01` checkbox was **not** marked complete, following the exact precedent plan 24-01's own SUMMARY recorded: per the phase's coverage table, HITL-01 needs plans 01, 02, 03 and 05 together. This plan lands the `Gate` node half of HITL-01 (raise/deliver/route/validate/fingerprint) but not the multi-parley/multi-process/partial-answer proof (plan 24-03) or the richer resume validation matrix (plan 24-04). Whichever later plan is the LAST to land its share of HITL-01 (24-03 or 24-05 per the coverage table) should be the one to run `gsd_run query requirements.mark-complete HITL-01`.

## Next Phase Readiness

- `NodeSpec::Gate` is proven end-to-end: raise → suspend → resume → deliver → route, over `InMemoryWaypointStore`, with every wiring-invalid shape rejected at `validate` time and every routing-relevant Gate property hashed into the `v4` fingerprint. This is the exact surface plan 24-03's E2E-2 integration test (`tests/integration/e2e_approval_gate_test.rs`) and multi-parley/cross-process suspension tests build on.
- `validate_parley_value_for_kind` is ready for plan 24-04 to import directly for `resume_with`'s richer validation matrix (`ParleyAlreadyAnswered`, `ResponseShapeInvalid`, `ParleyExpired`) — the same function this plan's `on_expire` default check already calls, so no second validator is ever written.
- No blockers. The Postgres Tier-2 contract-suite concern carried forward from Phase 22/23 is unaffected by this plan (no contract-suite changes here).

## Self-Check: PASSED

All 4 modified files verified present on disk; all 5 commit hashes (`2f96df5a`, `ba789827`, `968ecd98`, `0a1385c9`, `f1ecad52`) verified present in `git log --oneline --all`.

---
*Phase: 24-pause-resume-history-graceful-shutdown*
*Completed: 2026-09-05*
