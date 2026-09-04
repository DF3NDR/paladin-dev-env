---
phase: 23-control-flow-dynamic-routing-fan-out-subgraphs
verified: 2026-09-04T05:15:00Z
status: human_needed
score: 5/5 must-haves verified
behavior_unverified: 0
overrides_applied: 0
human_verification:
  - test: "Run the Postgres Tier-2 waypoint contract suite against a live server (muster_progress_round_trips, muster_progress_none_round_trips_as_none, checkpoint_ns_round_trips, checkpoint_ns_none_round_trips in crates/paladin-storage/src/waypoint/postgres.rs) via the postgres-integration CI job or `docker compose -f docker/docker-compose.test.yml up -d postgres-test`."
    expected: "All four Postgres-backed contract tests pass identically to the already-passing SQLite/in-memory runs, confirming the additive muster_progress and checkpoint_ns Waypoint columns round-trip correctly against a real Postgres server."
    why_human: "Docker is unavailable in this devcontainer; `cargo test -p paladin-storage --lib --all-features` compiles the Postgres test bodies but every one prints `SKIP: postgres-test not reachable` and returns early (verified directly: 0 assertions executed against a live server). This is the phase's own documented honest gap, not a fabricated one."
  - test: "Confirm the 5 prohibition clauses recorded as `verification: flagged-unverified` in PLAN frontmatter (23-01: no config/env/feature restores BUG-01's always-true behavior; 23-03 x2: LLM prompt/response/credential never interpolated into errors, and Semantic/LlmDecision unreachable without in-code configuration; 23-08: unmapped child Battlefield fields never leak to parent)."
    expected: "Each prohibition holds with no escape hatch."
    why_human: "This verifier independently re-derived positive evidence for all five (see Prohibitions Reviewed table below — grep for APP_ENGINE_* found no edge/LLM-routing toggle; `llm_error_class()` maps every LlmError to a fixed &'static str never the raw body; InputMappingError only ever carries a field NAME, never a resolved value; `unmapped_child_fields_stay_private` is a real, passing test). None were executor-self-certified as `test`-tier, so per the project's judgment-tier prohibition policy this is recorded as a non-authoritative LLM-judge verdict, not a silent pass — a human should sign off before this is treated as closed."
  - test: "Flip the stale CF-01/CF-05 checkboxes in .planning/REQUIREMENTS.md (lines 72, 94) and .planning/ROADMAP.md (lines 72, 94, 359, 363) from Pending/`[ ]` to Complete/`[x]`."
    expected: "Both tracking documents match what actually shipped."
    why_human: "No plan in this phase's wave sequence (23-01 through 23-12) ever edited these two rows for CF-01 or CF-05 — grep of the plan-history diff of REQUIREMENTS.md shows CF-02/03/04 each got a `mark X complete` commit; CF-01 and CF-05 never did, despite 23-12's own SUMMARY.md claiming `requirements-completed: [CF-01, CF-02, CF-03, CF-04, CF-05]`. This verifier independently confirmed CF-01 and CF-05 are fully implemented and tested in code (see Requirements Coverage below) — the gap is bookkeeping only, not functional, but it is real and will mislead the next phase's context load if left unfixed."
---

# Phase 23: Control Flow — Dynamic Routing, Fan-Out & Subgraphs Verification Report

**Phase Goal:** Nodes steer their own routing at runtime, dynamically fan out into map-reduce workers, nest Battalions as subgraphs, and optionally route by LLM evaluation — with the BUG-01 custom-edge-condition defect fixed fail-closed.
**Verified:** 2026-09-04T05:15Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (ROADMAP Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | BUG-01 fixed fail-closed on both `CampaignExecutionService` and `WarEngine`, with a failing-then-passing test visible in history (CF-01) | ✓ VERIFIED | `git log`: `b2d05045 test(23-01): reproduce BUG-01 ... (red)` precedes `8d5ef333 fix(23-01): fail closed ... (green)`. `grep -rn "defaulting to true" crates/` → empty. `crates/paladin-battalion/src/edge_evaluator.rs` (200 lines) implements `EdgeConditionEvaluator`/`EdgeEvaluatorRegistry`. `WarGraph::validate_edge_evaluators` and `CampaignExecutionService`'s pre-check both fail with `BattalionError::InvalidGraph`/`EngineError::UnregisteredEdgeCondition` before any node executes. Ran 10 named tests live (`unregistered_custom_condition_is_rejected_before_any_paladin_executes`, `every_unregistered_custom_name_is_listed_sorted_and_deduped`, `registered_true/false_evaluator_routes...`, `evaluator_error_fails_the_legacy_run_naming_the_edge`, `engine_evaluator_error_fails_the_run_naming_edge_and_evaluator`, etc.) — all pass. |
| 2 | Node-returned `Directive`/`NextStep::{Edges,Goto,End,Muster,Parley}` steers execution, validated `Goto`, End-over-Goto precedence, `DirectiveParser` defaulting to `PlainOutput` (CF-02) | ✓ VERIFIED | `crates/paladin-core/src/platform/container/directive.rs` defines `Directive`/`NextStep`. Live-ran `goto_to_an_undeclared_node_fails_the_run`, `end_beats_goto_in_the_same_superstep`, `goto_only_target_must_be_declared_dynamic`, `goto_refine_loop_terminates_on_the_reviewer_verdict`, `unbounded_goto_loop_trips_the_node_visit_limit` — all pass. `ParleyNotSupported` confirmed wired at `superstep.rs:1330`. `directive_parser.rs` has 10 tests including `plain_output_is_the_default_and_writes_the_output_field` and `default_on_parse_error_is_fail_run` — all pass. |
| 3 | Muster fan-out: runtime-N workers in one superstep, payload isolation, `task_key`-ordered aggregation, duplicate-key rejection, `max_muster_tasks` limit, mid-muster resume re-running only unfinished tasks (CF-03) | ✓ VERIFIED | Live-ran 22 muster-related unit tests including `duplicate_task_key_fails_before_any_task_starts`, `muster_exceeding_the_limit_fails_before_any_task_starts`, `muster_of_exactly_the_limit_runs`, `task_key_order_is_stable_across_twenty_shuffled_runs`, `worker_deltas_merge_in_task_key_order_not_completion_order`, `muster_payload_never_enters_the_battlefield`, `resume_mid_muster_runs_exactly_the_unfinished_tasks`, `resumed_muster_final_battlefield_equals_the_uninterrupted_run`, `fifty_task_muster_runs_to_completion_under_multi_thread`, `fifty_task_muster_is_deterministic_across_repeats` — all pass. `EngineLimits::max_muster_tasks` defaults to 100 (`engine_limits_default_max_muster_tasks_is_100` passes) and is fingerprint-excluded. `crates/paladin-storage/src/waypoint/contract_tests.rs` carries `muster_progress_round_trips`; passes on SQLite + in-memory. |
| 4 | `NodeSpec::Battalion` embeds a child WarGraph with `StateMap` mapping, namespaced checkpoint inheritance with resume-mid-child, recursive-embedding rejected at validation (CF-04) | ✓ VERIFIED | Live-ran `directly_recursive_embedding_is_rejected`, `transitively_recursive_embedding_is_rejected`, `battalion_node_runs_its_child_graph_to_completion`, `checkpoint_ns_records_the_namespace_path`, `child_threads_are_ordinary_threads_for_retention`, `unmapped_child_fields_stay_private`, `state_map_inputs_seed_the_child_schema`, `state_map_outputs_return_as_the_parent_nodes_delta` — all pass. `tests/integration/subgraph_formation_in_campaign_test.rs` (29 tests, including `formation_subgraph_runs_as_a_node_of_a_branching_parent_graph`, `phalanx_and_campaign_bridges_also_embed`, `killing_after_the_childs_first_superstep_and_resuming_does_not_repeat_child_work`) — all pass, run directly. |
| 5 | `LlmDecision` edge evaluator + Commander `StrategySelection::Semantic`, off by default, falling back to Heuristic on any LLM error, fallback recorded, existing Commander tests pass unmodified (CF-05) | ✓ VERIFIED | `crates/paladin-battalion/src/llm_decision.rs` (560 lines). Live-ran all 9 `llm_decision::tests` (matching, one-call-per-superstep memo, `on_ambiguous` Fail/Default, error-class-only surfacing, both template paths) — all pass. Ran all 57 `commander::tests` including `default_strategy_selection_is_heuristic_and_unchanged`, `semantic_mode_selects_the_strategy_the_model_names`, `semantic_falls_back_to_heuristic_on_llm_error`, `semantic_falls_back_to_heuristic_on_unrecognized_answer` plus the 52 pre-existing `test_*` Commander tests — all pass unmodified. `grep -n "APP_ENGINE" src/config/engine.rs` shows no edge/LLM-routing toggle exists. |

**Score:** 5/5 truths verified, 0 present-but-behavior-unverified.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/paladin-battalion/src/edge_evaluator.rs` | `EdgeConditionEvaluator`, `EdgeContext`, `EdgeEvaluatorRegistry`, `EdgeEvaluatorError` | ✓ VERIFIED | 200 lines, exists, wired into `campaign_service.rs` and `engine/mod.rs`, 3 unit tests pass. |
| `crates/paladin-battalion/src/campaign_service.rs` | `with_evaluator`, fail-closed pre-check, placeholder removed | ✓ VERIFIED | `with_evaluator` at L127, `InvalidGraph` fail-closed check at L230, async `evaluate_edge_condition` at L451. |
| `crates/paladin-battalion/src/engine/{mod.rs,graph.rs,superstep.rs}` | `with_edge_evaluator`, `UnregisteredEdgeCondition`/`EdgeEvaluatorFailed`, `validate_edge_evaluators` clause | ✓ VERIFIED | All symbols present and exercised by passing tests (see Truth 1). |
| `crates/paladin-core/src/platform/container/directive.rs` | `Directive`, `NextStep`, `MusterTask`, `MusterContext` | ✓ VERIFIED | All variants present (`Edges` implicit via `From<StateDelta>`, `Goto`, `Muster`, `End`, `Parley`). |
| `crates/paladin-battalion/src/engine/node.rs` | `StateNode::run` returns `Directive` | ✓ VERIFIED | Confirmed via passing Goto/End/Muster/Parley superstep tests, which depend on this return type. |
| `crates/paladin-battalion/src/engine/directive_parser.rs` | `PlainOutput` default, `StructuredDirective`, `on_parse_error` | ✓ VERIFIED | 10 passing unit tests. |
| `crates/paladin-battalion/src/llm_decision.rs` | `LlmDecisionEvaluator`, `OnAmbiguous`, one-call-per-decision memo | ✓ VERIFIED | 560 lines, 9 passing unit tests. |
| `crates/paladin-battalion/src/commander.rs` | `StrategySelection`, `CommanderBuilder::strategy_selection`, Semantic path | ✓ VERIFIED | 57 passing tests, `Commander::new` signature unchanged (grepped: exactly 1 `pub fn new(paladin_port`), no new public field on `Commander`. |
| `crates/paladin-storage/src/waypoint/{sqlite,in_memory,postgres}.rs` | `muster_progress`/`checkpoint_ns` additive fields round-trip via shared contract suite | ⚠️ PARTIAL (Postgres tier skipped) | SQLite + in-memory: 14 tests pass live. Postgres: 4 tests compile and run but self-skip (`SKIP: postgres-test not reachable`) — no Docker in this devcontainer. See Human Verification. |
| `.project/v0.10.0/08-traceability-matrix.md` | BUG-01 row with RED/GREEN SHAs | ✓ VERIFIED | Row references `b2d05045` (RED) and `8d5ef333` (GREEN), matching git history. |
| `MIGRATION.md` §9.1/§9.2 | M-B-01 worked example, resolved rows | ✓ VERIFIED | §9.1 M-B-01 row present; §9.2 `EdgeCondition`/`CampaignExecutionService`/`Commander`/`CommanderBuilder` rows resolved (`N`, not `TBD`). All remaining `TBD` markers in MIGRATION.md carry an owner + phase number (debt-marker gate satisfied). |
| `docs/src/user-guides/control-flow.md` | mdBook control-flow page | ✓ VERIFIED | 243 lines, linked from `docs/src/SUMMARY.md`. |
| `CHANGELOG.md` [Unreleased] | M-B-01 and CF-02..CF-05 entries | ✓ VERIFIED | All 5 entries present under `### Changed`/`### Added`. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `WarGraph::validate` | `EdgeEvaluatorRegistry` | fail-closed pre-check in the same `validate()` call as `CustomDispatchResolver` | ✓ WIRED | `graph.rs:571` `self.validate_edge_evaluators(edge_evaluators)?` runs inside `validate_non_recursive`, called from public `validate()`. |
| `CampaignExecutionService::execute` | fail-closed pre-check | runs after `campaign.validate()`, before first `paladin_port.execute` | ✓ WIRED | Confirmed by passing `unregistered_custom_condition_is_rejected_before_any_paladin_executes` (name asserts zero executions). |
| `superstep::evaluate_edge_condition` | `EdgeEvaluatorRegistry::get` | Custom arm awaits registered evaluator | ✓ WIRED | `superstep.rs:2344` surfaces `EdgeEvaluatorFailed`; live tests confirm Ok(true)/Ok(false)/Err all route correctly. |
| `LlmDecisionEvaluator` | `EdgeConditionEvaluator` trait (23-01) | registered under `EdgeCondition::Custom(name)` | ✓ WIRED | `llm_decision.rs` implements the trait; `Commander`'s `select_strategy_semantic` and `llm_decision.rs` share `llm_error_class()` via `pub(crate)`. |
| `NextStep::Muster` | validation-before-dispatch | duplicate key / limit breach / empty list all rejected before any task starts | ✓ WIRED | `duplicate_task_key_fails_before_any_task_starts`, `muster_exceeding_the_limit_fails_before_any_task_starts`, `empty_muster_fails_with_a_typed_error` all pass. |
| `NodeSpec::Battalion` child graph | parent `WarEngine` | engine/registry inheritance, `ThreadId::child`, `checkpoint_ns` | ✓ WIRED | `battalion_node_runs_its_child_graph_to_completion`, `checkpoint_ns_records_the_namespace_path` pass; `subgraph_formation_in_campaign_test.rs` (29 tests) passes end-to-end. |

### Behavioral Spot-Checks / Test Execution

| Check | Command | Result | Status |
|-------|---------|--------|--------|
| Full workspace lib/bin test suite (run once) | `cargo test --workspace --lib --bins` | 2,099 tests, 0 failed (523+2+440+475+96+1+43+110+76+0+105+111+117) | ✓ PASS |
| `cargo fmt --check` | workspace root | clean | ✓ PASS |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | workspace root | clean, 0 warnings | ✓ PASS |
| Phase 22 regression: `e2e_crash_resume` | `cargo test --test e2e_crash_resume` | 27 passed, 0 failed | ✓ PASS |
| Phase 22.1 regression: `waypoint_retention_fault_injection` | `cargo test --test waypoint_retention_fault_injection` | 3 passed, 0 failed | ✓ PASS |
| E2E-3 muster/defer/order integration | `cargo test --test e2e_muster_defer_order` | 30 passed, 0 failed | ✓ PASS |
| Formation-inside-Campaign subgraph integration | `cargo test --test subgraph_formation_in_campaign` | 29 passed, 0 failed | ✓ PASS |
| CR-01 regression (post-review-fix) | `cargo test -p paladin-battalion --lib muster_only_round_at_recursion_limit_fails_without_panicking` | 1 passed | ✓ PASS |
| Postgres tier-2 waypoint contract tests | `cargo test -p paladin-storage --lib --all-features round_trips` | SQLite (5) + in-memory (4) pass; Postgres (4) self-skip: `SKIP: postgres-test not reachable at postgres://...5433...` | ? SKIP (documented, Docker unavailable) |

### Code Review Fixes Verified in Tree

| Finding | Commit | Verified |
|---------|--------|----------|
| CR-01 (critical): `vanguard[0]` panics on empty Vanguard during muster-only round hitting the recursion limit | `a33b99e3` | ✓ Landed; fallback to `pending_muster`'s node; new regression test passes. |
| WR-01: exponential re-validation of nested `Battalion` children | `64b6d56c` | ✓ Landed; `validate_non_recursive` split confirmed in `graph.rs`; `graph::` tests (68) pass. |
| WR-02: `MusterProgress::unfinished_tasks()` documented but never called in production | `75ae0b45` | ✓ Landed; `run_with_namespace` now calls `.unfinished_tasks()` directly. |

### Prohibitions Reviewed (judgment-tier, `verification: flagged-unverified` in PLAN frontmatter)

| Statement (owning plan) | This verifier's evidence | Verdict |
|---|---|---|
| No config/env/feature/builder option restores BUG-01's always-true behavior (23-01) | `grep -n "APP_ENGINE" src/config/engine.rs` → only `MAX_SUPERSTEPS`/`MAX_NODE_VISITS`/`RUN_TIMEOUT_SECS`/`WAYPOINT_DURABILITY`/`MAX_MUSTER_TASKS`; no edge-condition/evaluator toggle exists anywhere in the tree. | Compliant (non-authoritative — human review recommended) |
| LLM prompt/response/credential never interpolated into an error/log/trace (23-03) | `llm_error_class()` maps every `LlmError` variant to a fixed `&'static str` (e.g. `"model not available"`), never the inner error string; `evaluation_error()` call sites pass either this fixed class or a field NAME (never a resolved value); `InputMappingError` variants carry only placeholder text, never Battlefield content. | Compliant (non-authoritative — human review recommended) |
| LLM-evaluated routing unreachable without explicit in-code configuration (23-03) | Same `APP_ENGINE_*` grep as above; `StrategySelection` and `LlmDecisionEvaluator` are reachable only via `CommanderBuilder::strategy_selection` / explicit `with_evaluator` registration — no default-on path found. | Compliant (non-authoritative — human review recommended) |
| Unmapped child Battlefield fields never leak to parent (23-08) | `unmapped_child_fields_stay_private` test exists and passes (ran live). | Compliant (non-authoritative — human review recommended) |

Per this project's judgment-tier prohibition policy, these are recorded as an LLM-judge verdict with supporting evidence, not a silent pass — flagged in `human_verification` above for an explicit sign-off.

### Requirements Coverage

| Requirement | Source Plan(s) | Description | Status | Evidence |
|---|---|---|---|---|
| CF-01 | 23-01 | BUG-01 fixed fail-closed, test-first, both paths | ✓ SATISFIED (functionally) / ⚠️ REQUIREMENTS.md still shows "Pending" | Full test/code evidence above. `.planning/REQUIREMENTS.md` line 72 and `.planning/ROADMAP.md` lines 72 & 359 were never flipped to `[x]`/`Complete` by any plan in this phase — a bookkeeping gap, not a functional one. |
| CF-02 | 23-02, 23-04 | Directive/NextStep routing, DirectiveParser | ✓ SATISFIED | REQUIREMENTS.md and ROADMAP.md correctly show `[x]`/Complete (flipped by 23-02's own commit). |
| CF-03 | 23-05, 23-06, 23-07, 23-11 | Muster map-reduce fan-out | ✓ SATISFIED | REQUIREMENTS.md/ROADMAP.md correctly show `[x]`/Complete (flipped by 23-06's commit). |
| CF-04 | 23-08, 23-09 | Battalion subgraph nesting | ✓ SATISFIED | REQUIREMENTS.md/ROADMAP.md correctly show `[x]`/Complete (flipped by 23-08's commit). |
| CF-05 | 23-03 | LLM-evaluated routing | ✓ SATISFIED (functionally) / ⚠️ REQUIREMENTS.md still shows "Pending" | Full test/code evidence above. Never flipped to `[x]`/Complete by any plan, despite 23-12-SUMMARY.md's `requirements-completed: [CF-01, CF-02, CF-03, CF-04, CF-05]` claim. |

No orphaned requirements: `.planning/REQUIREMENTS.md`'s "Phase 23" rows (CF-01…CF-05) exactly match the 5 IDs declared across the 12 plans' `requirements:` frontmatter.

### Anti-Patterns Found

None blocking. `grep -n -E "TBD|FIXME|XXX"` across every file this phase modified returns zero matches. `grep -iE "not yet implemented|placeholder|coming soon"` matches are all legitimate (documentation of the `{muster.payload}`/`{field}` *template placeholder* mechanism, or the already-fixed `MusterProgress::default` sentinel pattern from WR-02) — one pre-existing, out-of-scope "Transformation logic placeholder" comment at `campaign_service.rs:383` belongs to an unrelated edge-transform feature this phase did not touch and is not part of any CF-01..CF-05 must-have.

Remaining `TBD` markers in `MIGRATION.md` (15 total) all carry an explicit owner + phase reference (SHIP-01/02 Phase 29, HITL-04 Phase 24, RT-02/03/05 Phase 26, FT-01/05 Phase 25) — satisfies the debt-marker gate.

### Human Verification Required

See `human_verification` in frontmatter (3 items): the live-Postgres Tier-2 contract-test confirmation (CI-gated, honest environment gap), sign-off on the 4 judgment-tier prohibitions this verifier reviewed with positive evidence, and a trivial doc fix to flip the stale CF-01/CF-05 checkboxes in REQUIREMENTS.md/ROADMAP.md.

### Gaps Summary

No functional gaps. All 5 ROADMAP success criteria are independently verified against live-run tests and inspected code, not SUMMARY.md claims. The three human-verification items above are: (1) an already-documented, environment-caused gap (no Docker) rather than a fabricated shortfall, (2) a request for human sign-off on prohibitions this verifier already found compliant, and (3) a two-line documentation fix that does not affect delivered functionality. None of these block the phase goal — nodes do steer their own runtime routing, Muster fan-out works as map-reduce with tested mid-muster resume, Battalions nest with tested recursive-embedding rejection and resume-mid-child, and LLM-evaluated routing is available, off by default, and falls back deterministically — but the recommended next step is a human confirming item 3 (or accepting it as a fast follow-up commit) before this phase is considered fully closed out.

---

_Verified: 2026-09-04T05:15:00Z_
_Verifier: Claude (gsd-verifier)_
