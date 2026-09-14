---
status: complete
phase: 23-control-flow-dynamic-routing-fan-out-subgraphs
source: [23-01-SUMMARY.md, 23-02-SUMMARY.md, 23-03-SUMMARY.md, 23-04-SUMMARY.md, 23-05-SUMMARY.md, 23-06-SUMMARY.md, 23-07-SUMMARY.md, 23-08-SUMMARY.md, 23-09-SUMMARY.md, 23-10-SUMMARY.md, 23-11-SUMMARY.md, 23-12-SUMMARY.md, 23-VERIFICATION.md]
started: 2026-09-04T20:19:14Z
updated: 2026-09-04T21:18:26Z
---

## Current Test

[testing complete]

## Tests

### 1. Postgres Tier-2: muster_progress round-trips on a live server (23-06 D6)
expected: The muster_progress Waypoint field round-trips unchanged through the Postgres backend via the shared contract suite (muster_progress_round_trips and muster_progress_none_round_trips_as_none in crates/paladin-storage/src/waypoint/postgres.rs), matching the already-green SQLite and in-memory tiers. Evidence gathered for you: CI run 33901818056 on bdc84946 (HEAD dd1cecf8 differs only in 23-VALIDATION.md), job 'Postgres Waypoint Contract Suite (live server)' succeeded: both tests logged 'ok' against the live container, 32 passed / 0 failed, and the job's own guards ('Fail if the suite took the SKIP path' and 'Fail if the suite selected fewer tests than it declares') both passed. Reply yes if that CI evidence closes the live-server gap, or run it locally first: docker compose -f docker/docker-compose.test.yml up -d postgres-test && WAYPOINT_POSTGRES_TEST_URL=postgres://paladin:paladin@localhost:5433/paladin_waypoint_test cargo test -p paladin-storage --features postgres --lib waypoint::postgres -- --test-threads=1 --nocapture
result: pass
coverage_id: 23-06/D6

### 2. Postgres Tier-2: checkpoint_ns round-trips on a live server (23-09 D2)
expected: The additive #[serde(default)] Waypoint.checkpoint_ns field records the namespace path and round-trips through the Postgres backend via the shared contract suite (checkpoint_ns_round_trips and checkpoint_ns_none_round_trips in crates/paladin-storage/src/waypoint/postgres.rs), matching the SQLite and in-memory tiers. Same CI evidence as test 1: run 33901818056, job 'Postgres Waypoint Contract Suite (live server)', both tests logged 'ok' against the live container with 0 failures and the SKIP-path guard passing. Reply yes if that closes the gap, or run the same local command as test 1.
result: pass
coverage_id: 23-09/D2

### 3. Judgment-tier prohibitions hold with no escape hatch
expected: Sign off on the four prohibition clauses recorded as verification: flagged-unverified in PLAN frontmatter. Fresh evidence: (a) 23-01, no config/env/feature restores BUG-01's always-true behavior: src/config/engine.rs exposes only APP_ENGINE_MAX_SUPERSTEPS / MAX_NODE_VISITS / RUN_TIMEOUT_SECS / WAYPOINT_DURABILITY / MAX_MUSTER_TASKS, 'defaulting to true' appears nowhere in crates/ or src/, and both CampaignExecutionService and WarEngine start from an empty EdgeEvaluatorRegistry::new(). (b) 23-03, LLM prompt/response/credential is never interpolated into errors or logs: llm_error_class() maps every LlmError variant to a fixed static string, never the inner body. (c) 23-03, Semantic/LlmDecision routing is unreachable without in-code configuration: StrategySelection derives Default with #[default] Heuristic, and no call site outside commander.rs and llm_decision.rs constructs StrategySelection::Semantic or LlmDecisionEvaluator. (d) 23-08, unmapped child Battlefield fields never leak to the parent: unmapped_child_fields_stay_private in engine/superstep.rs is a real, passing test. Reply yes to sign off on all four, or name the clause you are not satisfied with.
result: pass

### 4. CF-01 and CF-05 tracking rows reflect what shipped
expected: REQUIREMENTS.md still shows CF-01 and CF-05 as '[ ]' (lines 72, 94) and 'Pending' in the traceability table (lines 359, 363) even though both are fully implemented and tested; ROADMAP.md's plan rows for 23-01 and 23-03 are already [x]. The phase-close step this session runs after UAT (phase.complete) reads ROADMAP's Phase 23 '**Requirements**: CF-01, CF-02, CF-03, CF-04, CF-05' line and flips exactly those checkbox and traceability rows to Complete; I will re-check the four lines afterwards and flip them with 'requirements mark-complete CF-01 CF-05' if the write misses. Reply yes to accept that as the fix, or say if you want them flipped by hand before the phase closes.
result: pass

### 5. Unregistered EdgeCondition::Custom fails validation before any node executes, on both CampaignExecutionService and WarEngine paths, naming every offender sorted and deduped (23-01 D1)
expected: Unregistered EdgeCondition::Custom fails validation before any node executes, on both CampaignExecutionService and WarEngine paths, naming every offender sorted and deduped
result: pass
source: automated
coverage_id: 23-01/D1
covered_by: campaign_service.rs#unregistered_custom_condition_is_rejected_before_any_paladin_executes, engine/graph.rs#unregistered_custom_edge_condition_fails_graph_validation, engine/graph.rs#every_unregistered_custom_name_is_listed_sorted_and_deduped

### 6. A registered evaluator's Ok(true)/Ok(false) verdict routes/does-not-route the edge on both paths (23-01 D2)
expected: A registered evaluator's Ok(true)/Ok(false) verdict routes/does-not-route the edge on both paths
result: pass
source: automated
coverage_id: 23-01/D2
covered_by: campaign_service.rs#registered_true_evaluator_routes_the_custom_edge, campaign_service.rs#registered_false_evaluator_does_not_route_the_custom_edge, engine/mod.rs#registered_engine_evaluator_true_and_false_route_correctly

### 7. A registered evaluator's Err fails the run with a typed error naming the edge and evaluator, never defaulting either branch (23-01 D3)
expected: A registered evaluator's Err fails the run with a typed error naming the edge and evaluator, never defaulting either branch
result: pass
source: automated
coverage_id: 23-01/D3
covered_by: campaign_service.rs#evaluator_error_fails_the_legacy_run_naming_the_edge, engine/mod.rs#engine_evaluator_error_fails_the_run_naming_edge_and_evaluator

### 8. MIGRATION.md M-B-01 worked example and §9.2 CF-01 rows resolved with no remaining CF-01 TBDs (23-01 D4)
expected: MIGRATION.md M-B-01 worked example and §9.2 CF-01 rows resolved with no remaining CF-01 TBDs
result: pass
source: automated
coverage_id: 23-01/D4
covered_by: grep -c 'TBD — owner CF-01, Phase 23' MIGRATION.md == 0

### 9. Directive { delta, next: NextStep } lands in paladin-core with NextStep::{Edges,Goto,Muster,End,Parley}, MusterTask, and impl From<StateDelta> for Directive defaulting to Edges, doc-tested (23-02 D1)
expected: Directive { delta, next: NextStep } lands in paladin-core with NextStep::{Edges,Goto,Muster,End,Parley}, MusterTask, and impl From<StateDelta> for Directive defaulting to Edges, doc-tested
result: pass
source: automated
coverage_id: 23-02/D1
covered_by: crates/paladin-core/src/platform/container/directive.rs#tests::state_delta_converts_to_a_directive_defaulting_to_edges, cargo test -p paladin-ai-core --doc directive (Directive::from doc test)

### 10. StateNode::run returns Result<Directive, NodeError>; every in-tree and workspace-wide implementor adopts .into() (23-02 D2)
expected: StateNode::run returns Result<Directive, NodeError>; every in-tree and workspace-wide implementor adopts .into()
result: pass
source: automated
coverage_id: 23-02/D2
covered_by: cargo test -p paladin-battalion --lib engine::superstep (39/39), cargo check --workspace --tests --examples --benches --all-features

### 11. A Function node's Goto([target]) enters the next Vanguard directly bypassing Frontier::is_ready, while the emitting node's static outgoing edges resolve NotFiring for that superstep (23-02 D3)
expected: A Function node's Goto([target]) enters the next Vanguard directly bypassing Frontier::is_ready, while the emitting node's static outgoing edges resolve NotFiring for that superstep
result: pass
source: automated
coverage_id: 23-02/D3
covered_by: engine::superstep::tests::function_node_goto_sends_control_to_the_named_node_next_superstep, engine::superstep::tests::goto_target_that_is_also_tier_one_ready_is_scheduled_exactly_once

### 12. An undeclared Goto target fails typed (GotoUnknownNode); a Goto-only target must be marked WarGraph::mark_dynamic_target via the existing eligible-set mechanism (23-02 D4)
expected: An undeclared Goto target fails typed (GotoUnknownNode); a Goto-only target must be marked WarGraph::mark_dynamic_target via the existing eligible-set mechanism
result: pass
source: automated
coverage_id: 23-02/D4
covered_by: engine::superstep::tests::goto_to_an_undeclared_node_fails_the_run, engine::superstep::tests::goto_only_target_must_be_declared_dynamic

### 13. A Goto refine loop (writer -> reviewer -> Goto(writer) until satisfied) terminates; an unbounded Goto loop trips NodeVisitLimitExceeded at exactly max_node_visits (23-02 D5)
expected: A Goto refine loop (writer -> reviewer -> Goto(writer) until satisfied) terminates; an unbounded Goto loop trips NodeVisitLimitExceeded at exactly max_node_visits
result: pass
source: automated
coverage_id: 23-02/D5
covered_by: engine::superstep::tests::goto_refine_loop_terminates_on_the_reviewer_verdict, engine::superstep::tests::unbounded_goto_loop_trips_the_node_visit_limit

### 14. NextStep::End completes the run after the emitting superstep's merge (peers still merge), beats a peer's Goto in the same superstep, and which node ended the run is observable from the Waypoint (23-02 D6)
expected: NextStep::End completes the run after the emitting superstep's merge (peers still merge), beats a peer's Goto in the same superstep, and which node ended the run is observable from the Waypoint
result: pass
source: automated
coverage_id: 23-02/D6
covered_by: engine::superstep::tests::end_completes_the_run_after_the_emitting_superstep_merges, engine::superstep::tests::end_beats_goto_in_the_same_superstep, engine::superstep::tests::which_node_ended_the_run_is_observable_from_the_waypoint

### 15. StarvedNodeAtCompletion's suppression is scoped to end_requested specifically -- an End-terminated run with an unrelated unconsumed fired edge completes, but the check still fires loudly when no node ended the run (23-02 D7)
expected: StarvedNodeAtCompletion's suppression is scoped to end_requested specifically -- an End-terminated run with an unrelated unconsumed fired edge completes, but the check still fires loudly when no node ended the run
result: pass
source: automated
coverage_id: 23-02/D7
covered_by: engine::superstep::tests::end_terminated_run_does_not_trip_the_starvation_completion_check, engine::superstep::tests::starvation_completion_check_still_fires_when_no_node_ended_the_run

### 16. A returned NextStep::Parley fails the run with EngineError::ParleyNotSupported, never coerced to Edges, with no AwaitingInput Waypoint written (23-02 D8)
expected: A returned NextStep::Parley fails the run with EngineError::ParleyNotSupported, never coerced to Edges, with no AwaitingInput Waypoint written
result: pass
source: automated
coverage_id: 23-02/D8
covered_by: engine::superstep::tests::parley_returned_this_phase_fails_the_run

### 17. E2E-1 (crash-resume golden) and the Phase 22 tracer test are unaffected by the StateNode/Directive migration (23-02 D9)
expected: E2E-1 (crash-resume golden) and the Phase 22 tracer test are unaffected by the StateNode/Directive migration
result: pass
source: automated
coverage_id: 23-02/D9
covered_by: cargo test --test e2e_crash_resume_test (27/27, including e2e_1_crash_resume_matches_control_run_with_no_reexecution), cargo test --test war_engine_tracer_test (3/3)

### 18. LlmDecisionEvaluator resolves EdgeCondition::Custom(name) edges from a model verdict, matching choices exact-after-trim/case-insensitive, firing exactly the edge whose target maps to the matched choice (23-03 D1)
expected: LlmDecisionEvaluator resolves EdgeCondition::Custom(name) edges from a model verdict, matching choices exact-after-trim/case-insensitive, firing exactly the edge whose target maps to the matched choice
result: pass
source: automated
coverage_id: 23-03/D1
covered_by: crates/paladin-battalion/src/llm_decision.rs#matching_choice_fires_only_the_mapped_edge, crates/paladin-battalion/src/llm_decision.rs#matching_is_exact_after_trim_and_case_insensitive

### 19. Exactly one LlmPort call per decision per superstep regardless of outgoing edge count (D-24 memo), with a fresh call on a new superstep (23-03 D2)
expected: Exactly one LlmPort call per decision per superstep regardless of outgoing edge count (D-24 memo), with a fresh call on a new superstep
result: pass
source: automated
coverage_id: 23-03/D2
covered_by: crates/paladin-battalion/src/llm_decision.rs#one_llm_call_per_decision_per_superstep, crates/paladin-battalion/src/llm_decision.rs#a_different_superstep_re_asks

### 20. Unmatched model answer resolves through OnAmbiguous: Fail returns a typed EdgeEvaluatorError, Default(choice) routes as if the model had answered that choice (23-03 D3)
expected: Unmatched model answer resolves through OnAmbiguous: Fail returns a typed EdgeEvaluatorError, Default(choice) routes as if the model had answered that choice
result: pass
source: automated
coverage_id: 23-03/D3
covered_by: crates/paladin-battalion/src/llm_decision.rs#unmatched_answer_with_on_ambiguous_fail_errors, crates/paladin-battalion/src/llm_decision.rs#unmatched_answer_with_on_ambiguous_default_routes_to_the_default_choice

### 21. An LlmError from the port surfaces as a typed EdgeEvaluatorError naming the evaluator and a fixed failure class, never the rendered prompt or the response body (23-03 D4)
expected: An LlmError from the port surfaces as a typed EdgeEvaluatorError naming the evaluator and a fixed failure class, never the rendered prompt or the response body
result: pass
source: automated
coverage_id: 23-03/D4
covered_by: crates/paladin-battalion/src/llm_decision.rs#llm_error_surfaces_as_a_typed_evaluator_error

### 22. Template rendering supports both the engine path (InputMapping over Battlefield) and the legacy path ({output} substitution from the source Paladin's output) (23-03 D5)
expected: Template rendering supports both the engine path (InputMapping over Battlefield) and the legacy path ({output} substitution from the source Paladin's output)
result: pass
source: automated
coverage_id: 23-03/D5
covered_by: crates/paladin-battalion/src/llm_decision.rs#legacy_path_renders_the_template_from_the_source_output, crates/paladin-battalion/src/llm_decision.rs#engine_path_renders_the_template_from_the_battlefield

### 23. Commander StrategySelection defaults to Heuristic (today's analyze_and_select, unchanged) when no builder call is made (23-03 D6)
expected: Commander StrategySelection defaults to Heuristic (today's analyze_and_select, unchanged) when no builder call is made
result: pass
source: automated
coverage_id: 23-03/D6
covered_by: crates/paladin-battalion/src/commander.rs#default_strategy_selection_is_heuristic_and_unchanged

### 24. Semantic mode prompts the model with the strategy catalog, matches its answer exact-after-trim/case-insensitive, and selects the named strategy (23-03 D7)
expected: Semantic mode prompts the model with the strategy catalog, matches its answer exact-after-trim/case-insensitive, and selects the named strategy
result: pass
source: automated
coverage_id: 23-03/D7
covered_by: crates/paladin-battalion/src/commander.rs#semantic_mode_selects_the_strategy_the_model_names, crates/paladin-battalion/src/commander.rs#semantic_matching_is_exact_after_trim_and_case_insensitive

### 25. Any LlmError or an unrecognized model answer falls back to the heuristic deterministically, with the fallback and its cause class recorded in strategy_selection_reasoning (never the raw answer or error body) (23-03 D8)
expected: Any LlmError or an unrecognized model answer falls back to the heuristic deterministically, with the fallback and its cause class recorded in strategy_selection_reasoning (never the raw answer or error body)
result: pass
source: automated
coverage_id: 23-03/D8
covered_by: crates/paladin-battalion/src/commander.rs#semantic_falls_back_to_heuristic_on_llm_error, crates/paladin-battalion/src/commander.rs#semantic_falls_back_to_heuristic_on_unrecognized_answer

### 26. Commander::new keeps its exact signature and Commander gains no new public field; StrategySelection is reachable only through CommanderBuilder::strategy_selection (23-03 D9)
expected: Commander::new keeps its exact signature and Commander gains no new public field; StrategySelection is reachable only through CommanderBuilder::strategy_selection
result: pass
source: automated
coverage_id: 23-03/D9
covered_by: awk '/pub struct Commander \\\\{/,/^\\\\}/' crates/paladin-battalion/src/commander.rs | grep -c 'pub .*StrategySelection' => 0, grep -c 'pub fn new(paladin_port' crates/paladin-battalion/src/commander.rs => 1

### 27. The existing 52 inline Commander tests plus both tests/integration/commander_integration_tests.rs and commander_error_paths_test.rs pass unmodified (23-03 D10)
expected: The existing 52 inline Commander tests plus both tests/integration/commander_integration_tests.rs and commander_error_paths_test.rs pass unmodified
result: pass
source: automated
coverage_id: 23-03/D10
covered_by: cargo test -p paladin-battalion --lib commander => 57 passed (52 pre-existing + 5 new), 0 failed, cargo test -p paladin-ai --test lib commander => 25 passed, 0 failed; git diff of both test files empty

### 28. LlmDecision and StrategySelection::Semantic are reachable only through in-code configuration -- no APP_* env var, no cargo feature, no config-struct field (23-03 D11)
expected: LlmDecision and StrategySelection::Semantic are reachable only through in-code configuration -- no APP_* env var, no cargo feature, no config-struct field
result: pass
source: automated
coverage_id: 23-03/D11
covered_by: grep -rn 'APP_LLM_DECISION|APP_.*SEMANTIC' src/ crates/ => no matches

### 29. DirectiveParser::PlainOutput is the default, writes the raw Paladin output to output_field and routes via NextStep::Edges, byte-identical to pre-CF-02 behavior; StructuredDirective parses D-11's documented JSON envelope and applies only its delta, with output_field untouched (23-04 D1)
expected: DirectiveParser::PlainOutput is the default, writes the raw Paladin output to output_field and routes via NextStep::Edges, byte-identical to pre-CF-02 behavior; StructuredDirective parses D-11's documented JSON envelope and applies only its delta, with output_field untouched
result: pass
source: automated
coverage_id: 23-04/D1
covered_by: engine::directive_parser::tests::plain_output_is_the_default_and_writes_the_output_field, engine::superstep::tests::structured_directive_parses_a_bare_json_object_output, engine::superstep::tests::structured_directive_does_not_write_the_output_field

### 30. JSON extraction follows the locked order: trimmed whole output as a JSON object, else the first ```json fenced block, else on_parse_error -- pinned against a two-fenced-block input to prove first-wins (23-04 D2)
expected: JSON extraction follows the locked order: trimmed whole output as a JSON object, else the first ```json fenced block, else on_parse_error -- pinned against a two-fenced-block input to prove first-wins
result: pass
source: automated
coverage_id: 23-04/D2
covered_by: engine::directive_parser::tests::structured_directive_parses_a_fenced_json_block, engine::directive_parser::tests::output_with_two_fenced_json_blocks_uses_the_first, engine::directive_parser::tests::empty_output_resolves_through_on_parse_error

### 31. OnParseError::FailRun fails the run with the typed EngineError::DirectiveParseFailed naming the node; OnParseError::FallbackPlain degrades to PlainOutput semantics -- both proven end-to-end through the real engine, not only at the parser's unit boundary (23-04 D3)
expected: OnParseError::FailRun fails the run with the typed EngineError::DirectiveParseFailed naming the node; OnParseError::FallbackPlain degrades to PlainOutput semantics -- both proven end-to-end through the real engine, not only at the parser's unit boundary
result: pass
source: automated
coverage_id: 23-04/D3
covered_by: engine::superstep::tests::malformed_output_under_fail_run_fails_the_run, engine::superstep::tests::malformed_output_under_fallback_plain_writes_the_raw_output

### 32. A StructuredDirective node's Goto/Muster/End next reaches the same NextStep machinery a Function node's Directive does; an envelope delta naming a field the Battlefield schema does not declare fails the run as a schema error; unknown top-level envelope keys are rejected via deny_unknown_fields rather than silently ignored (23-04 D4)
expected: A StructuredDirective node's Goto/Muster/End next reaches the same NextStep machinery a Function node's Directive does; an envelope delta naming a field the Battlefield schema does not declare fails the run as a schema error; unknown top-level envelope keys are rejected via deny_unknown_fields rather than silently ignored
result: pass
source: automated
coverage_id: 23-04/D4
covered_by: engine::superstep::tests::structured_directive_goto_routes_the_run, engine::superstep::tests::envelope_delta_naming_an_unknown_field_fails_the_run, engine::directive_parser::tests::envelope_with_an_unknown_top_level_key_is_rejected

### 33. A parse failure under FailRun leaves no partial state: when one Paladin node's StructuredDirective fails to parse in the same superstep as a sibling node whose delta would otherwise merge, the whole superstep's deltas are discarded together, before merge (23-04 D5)
expected: A parse failure under FailRun leaves no partial state: when one Paladin node's StructuredDirective fails to parse in the same superstep as a sibling node whose delta would otherwise merge, the whole superstep's deltas are discarded together, before merge
result: pass
source: automated
coverage_id: 23-04/D5
covered_by: engine::superstep::tests::structured_directive_parse_failure_does_not_merge_a_partial_delta

### 34. Every in-tree NodeSpec::Paladin construction site migrates to the new NodeSpec::paladin/paladin_with_directive_parser constructors; the PlainOutput default leaves the E2E-1 crash-resume golden and the legacy-pattern bridge-equivalence suite byte-identical (23-04 D6)
expected: Every in-tree NodeSpec::Paladin construction site migrates to the new NodeSpec::paladin/paladin_with_directive_parser constructors; the PlainOutput default leaves the E2E-1 crash-resume golden and the legacy-pattern bridge-equivalence suite byte-identical
result: pass
source: automated
coverage_id: 23-04/D6
covered_by: cargo test --test e2e_crash_resume (27/27, including e2e_1_crash_resume_matches_control_run_with_no_reexecution), cargo test --test golden_bridge_equivalence (31/31), cargo build -p paladin-battalion (clean; no construction site names directive_parser)

### 35. A planner's NextStep::Muster(tasks) in superstep N fans out into N worker-template dispatches that all run concurrently in superstep N+1 through the same snapshot/spawn/semaphore machinery ordinary vanguard nodes use, with the planner's own static outgoing edges resolving NotFiring (23-05 D1)
expected: A planner's NextStep::Muster(tasks) in superstep N fans out into N worker-template dispatches that all run concurrently in superstep N+1 through the same snapshot/spawn/semaphore machinery ordinary vanguard nodes use, with the planner's own static outgoing edges resolving NotFiring
result: pass
source: automated
coverage_id: 23-05/D1
covered_by: engine::superstep::tests::planner_musters_three_workers_that_all_run_in_one_superstep

### 36. Worker deltas merge in lexicographic task_key order regardless of real completion order, proven under actual concurrent execution (deliberately reversed per-task delays) and repeat-tested across 20 seeded-shuffle iterations per CF-FR-11 (23-05 D2)
expected: Worker deltas merge in lexicographic task_key order regardless of real completion order, proven under actual concurrent execution (deliberately reversed per-task delays) and repeat-tested across 20 seeded-shuffle iterations per CF-FR-11
result: pass
source: automated
coverage_id: 23-05/D2
covered_by: engine::superstep::tests::worker_deltas_merge_in_task_key_order_not_completion_order, engine::superstep::tests::task_key_order_is_stable_across_twenty_shuffled_runs

### 37. A worker task's payload is isolated to its own execution (NodeContext.muster) and never enters the Battlefield, never leaks to a sibling task, and is unreachable from a Battlefield-only render context (23-05 D3)
expected: A worker task's payload is isolated to its own execution (NodeContext.muster) and never enters the Battlefield, never leaks to a sibling task, and is unreachable from a Battlefield-only render context
result: pass
source: automated
coverage_id: 23-05/D3
covered_by: engine::superstep::tests::each_worker_sees_only_its_own_payload, engine::superstep::tests::muster_payload_never_enters_the_battlefield

### 38. A worker template may not be an entry node, is exempt from the eligible-set unreachable rejection, may have static outgoing edges, and may not have static incoming edges -- each enforced at WarGraph::validate (23-05 D4)
expected: A worker template may not be an entry node, is exempt from the eligible-set unreachable rejection, may have static outgoing edges, and may not have static incoming edges -- each enforced at WarGraph::validate
result: pass
source: automated
coverage_id: 23-05/D4
covered_by: engine::graph::tests::worker_template_is_exempt_from_the_unreachable_rejection, engine::graph::tests::worker_template_may_not_be_an_entry_node, engine::graph::tests::worker_template_may_not_have_static_incoming_edges, engine::graph::tests::worker_template_may_have_static_outgoing_edges

### 39. A defer: true node downstream of a worker template runs exactly once, only after every mustered task has resolved, strictly in a later superstep than the workers, seeing all results in task_key order (23-05 D5)
expected: A defer: true node downstream of a worker template runs exactly once, only after every mustered task has resolved, strictly in a later superstep than the workers, seeing all results in task_key order
result: pass
source: automated
coverage_id: 23-05/D5
covered_by: engine::superstep::tests::deferred_aggregator_runs_once_after_every_task_resolves

### 40. EngineLimits::max_muster_tasks exists with default 100, is enforced by validate() as a non-zero limit, and is excluded from WarGraph::fingerprint like every other EngineLimits field (23-05 D6)
expected: EngineLimits::max_muster_tasks exists with default 100, is enforced by validate() as a non-zero limit, and is excluded from WarGraph::fingerprint like every other EngineLimits field
result: pass
source: automated
coverage_id: 23-05/D6
covered_by: engine::graph::tests::engine_limits_default_max_muster_tasks_is_100, engine::graph::tests::validate_rejects_zero_max_muster_tasks, engine::graph::tests::fingerprint_is_unchanged_by_prompt_model_input_mapping_and_limits

### 41. Duplicate task_key, a max_muster_tasks breach (both sides of the boundary), an empty task list, an unknown worker, and a worker that is declared but not a template all fail with a typed error naming the mustering node and the offender BEFORE any task starts (zero worker runs) (23-05 D7)
expected: Duplicate task_key, a max_muster_tasks breach (both sides of the boundary), an empty task list, an unknown worker, and a worker that is declared but not a template all fail with a typed error naming the mustering node and the offender BEFORE any task starts (zero worker runs)
result: pass
source: automated
coverage_id: 23-05/D7
covered_by: engine::superstep::tests::duplicate_task_key_fails_before_any_task_starts, engine::superstep::tests::muster_exceeding_the_limit_fails_before_any_task_starts, engine::superstep::tests::muster_of_exactly_the_limit_runs, engine::superstep::tests::empty_muster_fails_with_a_typed_error, engine::superstep::tests::muster_naming_an_unknown_worker_fails, engine::superstep::tests::muster_naming_a_non_template_node_fails

### 42. The max_muster_tasks comparison widens the u32 limit to usize rather than narrowing the task count with `as u32`, so a task list longer than u32::MAX cannot wrap into a passing count (23-05 D8)
expected: The max_muster_tasks comparison widens the u32 limit to usize rather than narrowing the task count with `as u32`, so a task list longer than u32::MAX cannot wrap into a passing count
result: pass
source: automated
coverage_id: 23-05/D8
covered_by: engine::superstep::tests::task_count_check_does_not_narrow_the_length

### 43. A worker Paladin's InputMapping template resolves {muster.payload} and {muster.task_key} from the executing task's context (verified through RecordingPaladinPort's captured rendered input); a schema field named with the muster. prefix is rejected at validation, and with no muster context present the placeholder is a typed error, never a Battlefield read (23-05 D9)
expected: A worker Paladin's InputMapping template resolves {muster.payload} and {muster.task_key} from the executing task's context (verified through RecordingPaladinPort's captured rendered input); a schema field named with the muster. prefix is rejected at validation, and with no muster context present the placeholder is a typed error, never a Battlefield read
result: pass
source: automated
coverage_id: 23-05/D9
covered_by: engine::superstep::tests::worker_input_template_resolves_the_muster_payload_placeholder, engine::superstep::tests::worker_input_template_resolves_the_task_key_placeholder, engine::superstep::tests::muster_placeholders_never_resolve_from_the_battlefield, engine::graph::tests::schema_field_named_with_the_muster_prefix_is_rejected, engine::input_mapping::tests (4 new: renders_muster_payload_placeholder_from_context, renders_muster_task_key_placeholder_from_context, muster_placeholder_with_no_context_is_a_typed_error_not_a_battlefield_read, unrecognized_muster_placeholder_name_is_a_typed_error)

### 44. As mustered tasks complete, the engine persists a Waypoint at the SAME superstep index with status Running carrying an additive muster_progress field with the muster spec and completed tasks' UNMERGED deltas keyed by task_key (23-06 D1)
expected: As mustered tasks complete, the engine persists a Waypoint at the SAME superstep index with status Running carrying an additive muster_progress field with the muster spec and completed tasks' UNMERGED deltas keyed by task_key
result: pass
source: automated
coverage_id: 23-06/D1
covered_by: engine::superstep::tests::progress_waypoints_are_written_at_the_same_superstep_index_with_status_running, engine::superstep::tests::one_progress_waypoint_per_completed_task

### 45. The Battlefield on a mid-muster progress Waypoint is byte-identical to the superstep's START snapshot -- never a partially merged state (23-06 D2)
expected: The Battlefield on a mid-muster progress Waypoint is byte-identical to the superstep's START snapshot -- never a partially merged state
result: pass
source: automated
coverage_id: 23-06/D2
covered_by: engine::superstep::tests::progress_waypoint_battlefield_equals_the_superstep_start_snapshot

### 46. Resuming from a progress Waypoint with 2 of 5 tasks done re-enters the muster, executes exactly the 3 unfinished tasks, merges all 5 deltas in task_key order, and produces a final Battlefield equal to the uninterrupted run's (23-06 D3)
expected: Resuming from a progress Waypoint with 2 of 5 tasks done re-enters the muster, executes exactly the 3 unfinished tasks, merges all 5 deltas in task_key order, and produces a final Battlefield equal to the uninterrupted run's
result: pass
source: automated
coverage_id: 23-06/D3
covered_by: engine::superstep::tests::resume_mid_muster_runs_exactly_the_unfinished_tasks, engine::superstep::tests::resumed_muster_final_battlefield_equals_the_uninterrupted_run

### 47. A Waypoint payload written before this change deserializes with muster_progress defaulting to None (23-06 D4)
expected: A Waypoint payload written before this change deserializes with muster_progress defaulting to None
result: pass
source: automated
coverage_id: 23-06/D4
covered_by: paladin_core::platform::container::waypoint::tests::waypoint_payload_without_muster_progress_field_deserializes_as_none

### 48. ENG-FR-11 is clarified rather than changed: exactly one superstep-COMPLETE Waypoint per superstep, plus zero-or-more progress Waypoints inside a muster's superstep; E2E-1 has no muster and its one-Waypoint-per-superstep assertion is unchanged (23-06 D5)
expected: ENG-FR-11 is clarified rather than changed: exactly one superstep-COMPLETE Waypoint per superstep, plus zero-or-more progress Waypoints inside a muster's superstep; E2E-1 has no muster and its one-Waypoint-per-superstep assertion is unchanged
result: pass
source: automated
coverage_id: 23-06/D5
covered_by: tests/integration/e2e_crash_resume_test.rs#e2e_1_crash_resume_matches_control_run_with_no_reexecution, .project/v0.10.0/01-battlefield-state-and-execution-engine.md ENG-FR-11 clarification note + 08-traceability-matrix.md cross-reference

### 49. Progress-Waypoint cadence is one per completed task, bounded by max_muster_tasks, honoring the configured WaypointDurability -- Strict fails the run on a write error, BestEffort logs and continues (23-06 D7)
expected: Progress-Waypoint cadence is one per completed task, bounded by max_muster_tasks, honoring the configured WaypointDurability -- Strict fails the run on a write error, BestEffort logs and continues
result: pass
source: automated
coverage_id: 23-06/D7
covered_by: engine::superstep::tests::strict_durability_failure_on_a_progress_write_fails_the_run, engine::superstep::tests::best_effort_durability_failure_on_a_progress_write_continues

### 50. EngineConfig exists at src/config/engine.rs with Default, validate(), EnvOverridable, mirroring CitadelConfig/WaypointRetentionConfig field-for-field, carrying all five engine tunables (four documented plus max_muster_tasks) with their APP_ENGINE_* env overrides (23-07 D1)
expected: EngineConfig exists at src/config/engine.rs with Default, validate(), EnvOverridable, mirroring CitadelConfig/WaypointRetentionConfig field-for-field, carrying all five engine tunables (four documented plus max_muster_tasks) with their APP_ENGINE_* env overrides
result: pass
source: automated
coverage_id: 23-07/D1
covered_by: src/config/engine.rs#config::engine::tests::default_engine_config_matches_todays_engine_defaults, src/config/engine.rs#config::engine::tests::validate_rejects_zero_limits, src/config/engine.rs#config::engine::tests::env_overrides_apply_for_every_field, src/config/engine.rs#config::engine::tests::waypoint_durability_parses_both_variants_case_insensitively

### 51. APP_ENGINE_MAX_MUSTER_TASKS reaches a running engine's effective limit end-to-end: env override -> validate -> conversion into EngineLimits -> a muster exceeding it fails with the typed MusterTaskLimitExceeded error naming the configured limit (23-07 D2)
expected: APP_ENGINE_MAX_MUSTER_TASKS reaches a running engine's effective limit end-to-end: env override -> validate -> conversion into EngineLimits -> a muster exceeding it fails with the typed MusterTaskLimitExceeded error naming the configured limit
result: pass
source: automated
coverage_id: 23-07/D2
covered_by: src/config/engine.rs#config::engine::tests::app_engine_max_muster_tasks_reaches_a_running_engines_limit

### 52. MIGRATION.md 9.5 no longer describes EngineConfig as planned-but-absent; records it as landed at its real path with all five fields, and points the identical-boot claim at the named passing test (23-07 D3)
expected: MIGRATION.md 9.5 no longer describes EngineConfig as planned-but-absent; records it as landed at its real path with all five fields, and points the identical-boot claim at the named passing test
result: pass
source: automated
coverage_id: 23-07/D3
covered_by: grep -c 'not yet in the tree' MIGRATION.md == 0; grep -c 'APP_ENGINE_MAX_MUSTER_TASKS' MIGRATION.md >= 1

### 53. NodeSpec::Battalion embeds a child WarGraph as a node; the child runs to completion within ONE parent superstep regardless of how many supersteps the child itself takes, seeded from and returning only its StateMap-mapped fields under the PARENT's dispatch rules (23-08 D1)
expected: NodeSpec::Battalion embeds a child WarGraph as a node; the child runs to completion within ONE parent superstep regardless of how many supersteps the child itself takes, seeded from and returning only its StateMap-mapped fields under the PARENT's dispatch rules
result: pass
source: automated
coverage_id: 23-08/D1
covered_by: engine::superstep::tests::battalion_node_runs_its_child_graph_to_completion, engine::superstep::tests::state_map_inputs_seed_the_child_schema, engine::superstep::tests::state_map_outputs_return_as_the_parent_nodes_delta, engine::superstep::tests::one_parent_superstep_spans_the_whole_child_run

### 54. Unmapped child Battlefield fields never surface in the parent's Battlefield, the Battalion node's own delta, or the parent thread's Waypoint payload (23-08 D2)
expected: Unmapped child Battlefield fields never surface in the parent's Battlefield, the Battalion node's own delta, or the parent thread's Waypoint payload
result: pass
source: automated
coverage_id: 23-08/D2
covered_by: engine::superstep::tests::unmapped_child_fields_stay_private

### 55. The child run inherits the parent engine wholesale (PaladinPort, WaypointPort, dispatch resolver, edge-evaluator registry, trace sink, interceptors, cancellation token) while using its OWN graph's EngineLimits (23-08 D3)
expected: The child run inherits the parent engine wholesale (PaladinPort, WaypointPort, dispatch resolver, edge-evaluator registry, trace sink, interceptors, cancellation token) while using its OWN graph's EngineLimits
result: pass
source: automated
coverage_id: 23-08/D3
covered_by: engine::superstep::tests::child_inherits_every_parent_engine_resource, engine::superstep::tests::child_uses_its_own_engine_limits

### 56. A child run failure surfaces as the Battalion node's structured EngineError::BattalionChildFailed naming the failing child node and the child thread; cancellation is observed by the child at its own superstep boundary, after which the parent halts at its own (23-08 D4)
expected: A child run failure surfaces as the Battalion node's structured EngineError::BattalionChildFailed naming the failing child node and the child thread; cancellation is observed by the child at its own superstep boundary, after which the parent halts at its own
result: pass
source: automated
coverage_id: 23-08/D4
covered_by: engine::superstep::tests::child_failure_surfaces_as_a_structured_node_error, engine::superstep::tests::cancellation_is_observed_at_the_child_superstep_boundary

### 57. Every StateMap-mapped field is checked against both schemas (parent input/output, child input/output), collecting every offender at once rather than failing on the first (23-08 D5)
expected: Every StateMap-mapped field is checked against both schemas (parent input/output, child input/output), collecting every offender at once rather than failing on the first
result: pass
source: automated
coverage_id: 23-08/D5
covered_by: engine::graph::tests::state_map_input_naming_an_unknown_parent_field_fails_validation, engine::graph::tests::state_map_input_naming_an_unknown_child_field_fails_validation, engine::graph::tests::state_map_output_naming_an_unknown_child_field_fails_validation, engine::graph::tests::state_map_output_naming_an_unknown_parent_field_fails_validation, engine::graph::tests::every_offending_mapped_field_is_reported_not_just_the_first

### 58. Each child graph is validated recursively under the parent's SAME dispatch resolver and edge-evaluator registry, extending CF-01's fail-closed contract into subgraphs; a child's own structural defect fails the parent's validate too (23-08 D6)
expected: Each child graph is validated recursively under the parent's SAME dispatch resolver and edge-evaluator registry, extending CF-01's fail-closed contract into subgraphs; a child's own structural defect fails the parent's validate too
result: pass
source: automated
coverage_id: 23-08/D6
covered_by: engine::graph::tests::child_graph_is_validated_with_the_parents_registries, engine::graph::tests::child_graph_with_its_own_structural_defect_fails_the_parent_validate

### 59. Recursive embedding (direct or transitive) is rejected with a typed, path-bearing error via a path-set walk over child fingerprints, before any node executes; deep but genuinely acyclic nesting still validates (23-08 D7)
expected: Recursive embedding (direct or transitive) is rejected with a typed, path-bearing error via a path-set walk over child fingerprints, before any node executes; deep but genuinely acyclic nesting still validates
result: pass
source: automated
coverage_id: 23-08/D7
covered_by: engine::graph::tests::directly_recursive_embedding_is_rejected, engine::graph::tests::transitively_recursive_embedding_is_rejected, engine::graph::tests::deep_but_acyclic_nesting_validates

### 60. The two StateMap shapes the sources leave open are resolved by decision and pinned by tests: mapping one child field to two parent fields is accepted, and an empty inputs list is accepted (23-08 D8)
expected: The two StateMap shapes the sources leave open are resolved by decision and pinned by tests: mapping one child field to two parent fields is accepted, and an empty inputs list is accepted
result: pass
source: automated
coverage_id: 23-08/D8
covered_by: engine::graph::tests::state_map_mapping_one_child_field_to_two_parent_fields_is_accepted, engine::graph::tests::state_map_with_empty_inputs_is_accepted

### 61. ThreadId::child(parent, node) derives an injective, length-prefixed child thread id, proven adversarially against the exact CR-01 collision shape, composing correctly for nested (grandchild) derivation, and failing typed (never truncating) when the result would exceed ThreadId's own limits (23-09 D1)
expected: ThreadId::child(parent, node) derives an injective, length-prefixed child thread id, proven adversarially against the exact CR-01 collision shape, composing correctly for nested (grandchild) derivation, and failing typed (never truncating) when the result would exceed ThreadId's own limits
result: pass
source: automated
coverage_id: 23-09/D1
covered_by: crates/paladin-core/src/platform/container/waypoint.rs#platform::container::waypoint::tests::child_thread_derivation_is_injective_under_adversarial_names, crates/paladin-core/src/platform/container/waypoint.rs#platform::container::waypoint::tests::derived_child_thread_id_passes_thread_id_validation, crates/paladin-core/src/platform/container/waypoint.rs#platform::container::waypoint::tests::nested_child_thread_ids_compose, crates/paladin-core/src/platform/container/waypoint.rs#platform::container::waypoint::tests::derived_child_thread_id_exceeding_max_len_fails_typed_rather_than_truncating

### 62. A parent resumed mid-child resumes the child from latest(child_thread) with zero re-execution of already-completed child work; restart_on_resume: true opts out and runs the child fresh (23-09 D3)
expected: A parent resumed mid-child resumes the child from latest(child_thread) with zero re-execution of already-completed child work; restart_on_resume: true opts out and runs the child fresh
result: pass
source: automated
coverage_id: 23-09/D3
covered_by: crates/paladin-battalion/src/engine/superstep.rs#engine::superstep::tests::resume_of_a_parent_mid_child_resumes_the_child_where_it_stopped, crates/paladin-battalion/src/engine/superstep.rs#engine::superstep::tests::restart_on_resume_true_runs_the_child_fresh, crates/paladin-battalion/src/engine/superstep.rs#engine::superstep::tests::latest_on_the_child_thread_returns_the_childs_own_waypoint, crates/paladin-battalion/src/engine/superstep.rs#engine::superstep::tests::child_threads_are_ordinary_threads_for_retention, tests/integration/subgraph_formation_in_campaign_test.rs#killing_after_the_childs_first_superstep_and_resuming_does_not_repeat_child_work

### 63. A Formation subgraph embedded as a node of a BRANCHING parent graph runs correctly -- the untaken branch is proven NotFiring, the Formation's nodes execute in sequential order, and its mapped output reaches the parent (23-09 D4)
expected: A Formation subgraph embedded as a node of a BRANCHING parent graph runs correctly -- the untaken branch is proven NotFiring, the Formation's nodes execute in sequential order, and its mapped output reaches the parent
result: pass
source: automated
coverage_id: 23-09/D4
covered_by: tests/integration/subgraph_formation_in_campaign_test.rs#formation_subgraph_runs_as_a_node_of_a_branching_parent_graph, tests/integration/subgraph_formation_in_campaign_test.rs#phalanx_and_campaign_bridges_also_embed

### 64. GRAPH_FINGERPRINT_VERSION reads v3; every stored v2-tagged fingerprint is recognised as stale by version-tag mismatch. (23-10 D1)
expected: GRAPH_FINGERPRINT_VERSION reads v3; every stored v2-tagged fingerprint is recognised as stale by version-tag mismatch.
result: pass
source: automated
coverage_id: 23-10/D1
covered_by: crates/paladin-battalion/src/engine/graph.rs#engine::graph::tests::fingerprint_version_tag_is_v3, crates/paladin-core/src/platform/container/waypoint.rs#platform::container::waypoint::tests::graph_fingerprint_is_deterministic_and_versioned

### 65. v3 hashes the worker-template set, sorted and length-prefixed through push_field, with an order-independence test. (23-10 D2)
expected: v3 hashes the worker-template set, sorted and length-prefixed through push_field, with an order-independence test.
result: pass
source: automated
coverage_id: 23-10/D2
covered_by: crates/paladin-battalion/src/engine/graph.rs#engine::graph::tests::fingerprint_differs_when_a_node_is_marked_a_worker_template, crates/paladin-battalion/src/engine/graph.rs#engine::graph::tests::worker_template_section_is_order_independent

### 66. v3 hashes each Battalion node's child fingerprint, StateMap, and restart_on_resume -- one difference test per property. (23-10 D3)
expected: v3 hashes each Battalion node's child fingerprint, StateMap, and restart_on_resume -- one difference test per property.
result: pass
source: automated
coverage_id: 23-10/D3
covered_by: crates/paladin-battalion/src/engine/graph.rs#engine::graph::tests::fingerprint_differs_when_an_embedded_child_graph_differs, crates/paladin-battalion/src/engine/graph.rs#engine::graph::tests::fingerprint_differs_when_a_state_map_differs, crates/paladin-battalion/src/engine/graph.rs#engine::graph::tests::fingerprint_differs_when_restart_on_resume_differs

### 67. v3 hashes each Paladin node's DirectiveParser kind and on_parse_error -- one difference test each. (23-10 D4)
expected: v3 hashes each Paladin node's DirectiveParser kind and on_parse_error -- one difference test each.
result: pass
source: automated
coverage_id: 23-10/D4
covered_by: crates/paladin-battalion/src/engine/graph.rs#engine::graph::tests::fingerprint_differs_when_a_directive_parser_kind_differs, crates/paladin-battalion/src/engine/graph.rs#engine::graph::tests::fingerprint_differs_when_on_parse_error_differs

### 68. ENG-FR-14 exclusions (prompts, models, InputMapping templates, every EngineLimits field including max_muster_tasks) still hold under v3, proven by the extended existing test, not a new sibling. (23-10 D5)
expected: ENG-FR-14 exclusions (prompts, models, InputMapping templates, every EngineLimits field including max_muster_tasks) still hold under v3, proven by the extended existing test, not a new sibling.
result: pass
source: automated
coverage_id: 23-10/D5
covered_by: crates/paladin-battalion/src/engine/graph.rs#engine::graph::tests::fingerprint_is_unchanged_by_prompt_model_input_mapping_and_limits

### 69. Golden-hex test re-pinned to the v3 digest of its unchanged reference graph. (23-10 D6)
expected: Golden-hex test re-pinned to the v3 digest of its unchanged reference graph.
result: pass
source: automated
coverage_id: 23-10/D6
covered_by: crates/paladin-battalion/src/engine/graph.rs#engine::graph::tests::fingerprint_golden_hex_pins_canonical_bytes

### 70. A planner node musters 5 workers that all run in one superstep through the real WarEngine + real Paladin dispatch path (FaultyPaladinPort), and a defer:true aggregator downstream runs exactly once, strictly after every worker's superstep (23-11 D1)
expected: A planner node musters 5 workers that all run in one superstep through the real WarEngine + real Paladin dispatch path (FaultyPaladinPort), and a defer:true aggregator downstream runs exactly once, strictly after every worker's superstep
result: pass
source: automated
coverage_id: 23-11/D1
covered_by: tests/integration/e2e_muster_defer_order_test.rs#planner_musters_five_workers_and_the_deferred_aggregator_runs_once

### 71. The list-dispatch aggregated Battlefield field holds exactly 5 worker results in deterministic task_key order, not completion order (23-11 D2)
expected: The list-dispatch aggregated Battlefield field holds exactly 5 worker results in deterministic task_key order, not completion order
result: pass
source: automated
coverage_id: 23-11/D2
covered_by: tests/integration/e2e_muster_defer_order_test.rs#aggregated_results_are_exactly_five_in_task_key_order

### 72. The recovering-worker half of E2E-3 is exercised via a manually-succeeding-on-attempt-N mock (FaultyPaladinPort's shared global attempt counter, pre-driven before the real run) rather than a real Aegis retry policy, with the run still producing all 5 results, at a clearly marked Phase 25 / FT-FR-06 seam (23-11 D3)
expected: The recovering-worker half of E2E-3 is exercised via a manually-succeeding-on-attempt-N mock (FaultyPaladinPort's shared global attempt counter, pre-driven before the real run) rather than a real Aegis retry policy, with the run still producing all 5 results, at a clearly marked Phase 25 / FT-FR-06 seam
result: pass
source: automated
coverage_id: 23-11/D3
covered_by: tests/integration/e2e_muster_defer_order_test.rs#one_worker_recovers_by_manual_attempt_scripting

### 73. The ENG-FR-11 clarification holds through the full E2E path: exactly one superstep-complete Waypoint (muster_progress: None) per superstep index, with the muster's 5 intra-superstep progress Waypoints (muster_progress: Some) counted separately (23-11 D4)
expected: The ENG-FR-11 clarification holds through the full E2E path: exactly one superstep-complete Waypoint (muster_progress: None) per superstep index, with the muster's 5 intra-superstep progress Waypoints (muster_progress: Some) counted separately
result: pass
source: automated
coverage_id: 23-11/D4
covered_by: tests/integration/e2e_muster_defer_order_test.rs#run_completes_with_a_single_superstep_complete_waypoint_per_superstep

### 74. E2E-1 (tests/integration/e2e_crash_resume_test.rs) remains byte-identical and green -- the golden this phase must not move (23-11 D5)
expected: E2E-1 (tests/integration/e2e_crash_resume_test.rs) remains byte-identical and green -- the golden this phase must not move
result: pass
source: automated
coverage_id: 23-11/D5
covered_by: tests/integration/e2e_crash_resume_test.rs#e2e_1_crash_resume_matches_control_run_with_no_reexecution

### 75. A 50-task muster runs to completion under a real #[tokio::test(flavor = \\"multi_thread\\")] runtime, wrapped in an explicit timeout guard, asserting exactly 50 worker executions, exactly 50 aggregated entries in sorted task_key order, all 50 keys distinct, and exactly 1 aggregator execution -- not in the default suite's #[ignore] list (23-11 D6)
expected: A 50-task muster runs to completion under a real #[tokio::test(flavor = \\"multi_thread\\")] runtime, wrapped in an explicit timeout guard, asserting exactly 50 worker executions, exactly 50 aggregated entries in sorted task_key order, all 50 keys distinct, and exactly 1 aggregator execution -- not in the default suite's #[ignore] list
result: pass
source: automated
coverage_id: 23-11/D6
covered_by: engine::superstep::tests::fifty_task_muster_runs_to_completion_under_multi_thread

### 76. The 50-task muster's final Battlefield is byte-identical across 3 repeated multi-thread runs, proving determinism under real thread interleaving rather than a single lucky run (23-11 D7)
expected: The 50-task muster's final Battlefield is byte-identical across 3 repeated multi-thread runs, proving determinism under real thread interleaving rather than a single lucky run
result: pass
source: automated
coverage_id: 23-11/D7
covered_by: engine::superstep::tests::fifty_task_muster_is_deterministic_across_repeats

### 77. New mdBook page docs/src/user-guides/control-flow.md documents Directives (23-12 D1)
expected: New mdBook page docs/src/user-guides/control-flow.md documents Directives
result: pass
source: automated
coverage_id: 23-12/D1
covered_by: grep -c 'control-flow.md' docs/src/SUMMARY.md == 1; awk User-Guides-block check prints the entry, cargo doc --workspace --no-deps (17 unresolved-link warnings, all pre-existing at HEAD -- no .rs file touched), cargo test --workspace --doc (0 failed)

### 78. MIGRATION.md §9.2 carries no CF-owned TBD and an explicit deliberate-zero note (23-12 D2)
expected: MIGRATION.md §9.2 carries no CF-owned TBD and an explicit deliberate-zero note
result: pass
source: automated
coverage_id: 23-12/D2
covered_by: grep -c 'TBD — owner CF-' MIGRATION.md == 0, grep -c for 'deliberate zero'/'v0.9.0'/'StateNode'/'Waypoint' in the new §9.2 note, all present

### 79. CHANGELOG.md [Unreleased] records M-B-01 (Changed, linking MIGRATION.md §9.1) (23-12 D3)
expected: CHANGELOG.md [Unreleased] records M-B-01 (Changed, linking MIGRATION.md §9.1)
result: pass
source: automated
coverage_id: 23-12/D3
covered_by: "grep -c 'M-B-01' CHANGELOG.md >= 1 and matched entry contains MIGRATION.md; substring, grep -c '^## \\\\[0\\\\.10\\\\.0\\\\]' CHANGELOG.md == 0

### 80. Program-gate evidence recorded on the phase's final commit: cargo semver-checks (23-12 D4)
expected: Program-gate evidence recorded on the phase's final commit: cargo semver-checks
result: pass
source: automated
coverage_id: 23-12/D4
covered_by: commands and verbatim output recorded below under 'Program-Gate Evidence'

## Summary

total: 80
passed: 80
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps

[none yet]
