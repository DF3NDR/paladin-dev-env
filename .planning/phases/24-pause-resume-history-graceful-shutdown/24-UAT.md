---
status: complete
phase: 24-pause-resume-history-graceful-shutdown
source: 24-01-SUMMARY.md, 24-02-SUMMARY.md, 24-03-SUMMARY.md, 24-04-SUMMARY.md, 24-05-SUMMARY.md, 24-06-SUMMARY.md, 24-07-SUMMARY.md, 24-08-SUMMARY.md, 24-09-SUMMARY.md, 24-10-SUMMARY.md, 24-11-SUMMARY.md, 24-12-SUMMARY.md, 24-13-SUMMARY.md, 24-14-SUMMARY.md
started: 2026-09-05T12:40:32Z
updated: 2026-09-05T13:58:12Z
---

## Current Test

[testing complete]

## Tests

### 1. Cold Start Smoke Test (paladin-server boots fresh, thread routes answer, SIGTERM drains)
expected: Kill any running paladin-server and clear ephemeral state. Run `cargo run --features web-server --bin paladin-server` from a clean shell (./config.yml unless PALADIN_CONFIG is set). Startup logs "Loading configuration" and a routes line including GET /v1/threads/{id}/state, POST /v1/threads/{id}/resume, GET /v1/threads/{id}/history, and serves without error. GET /health returns 200. GET /v1/threads/any/state returns 401 when http.auth is enabled, or 501 not_implemented naming the waypoint-store config key when auth is disabled (no backend wired by default). On SIGTERM/Ctrl-C the log shows "received SIGTERM; shutting down", "graceful shutdown drain complete: ...", "paladin-server shut down cleanly", and the process exits promptly (nothing in flight, well inside the 30s APP_ENGINE_SHUTDOWN_GRACE_SECS default).
result: pass
note: Verified 2026-09-05 from the tracked config.test.yml with http.auth.enabled=false (PALADIN_CONFIG override, port 18080). Routes line listed all three thread routes; waypoint store backend logged Disabled; GET /health 200; GET /v1/threads/any/state and /history 501 not_implemented naming APP_WAYPOINT_STORE_BACKEND; SIGTERM -> "received SIGTERM; shutting down" -> "graceful shutdown drain complete: Drained" -> "paladin-server shut down cleanly", exit ~1s. Observation (NOT a phase-24 gap): the user's untracked, gitignored local config.yml fails Settings deserialisation on pre-existing required fields (llm.deepseek.api_key; then rag.retrieval_trigger, mandatory since e5c58f4a 2026-01-30 on main). Follow-up candidate outside this phase.
source: cold-start-injection (24-09 and 24-11 modified the server entrypoint src/bin/paladin-server.rs, src/config/setup/service_runner.rs and k8s/*/deployment.yaml)

### 2. CR-02 Human Verdict is the reviewer's own (24-14 D1, HITL-04)
expected: 24-14-SUMMARY.md's "Human Verdict" section records `approved` as the reviewer's verbatim verdict on the CR-02 fix diff (commit 9802ce60, crates/paladin-battalion/src/engine/superstep.rs mid-Muster shutdown-grace abort), with the executor's five Muster-abort shape notes (zero / some / all sibling tasks complete at abort; unrelated non-Muster node aborted in the same superstep; two Musters in flight, shown unreachable) kept visibly separate from that verdict. You stand by that approval as your own end-to-end read of the diff, and the shape analysis matches your understanding of the merge-ordering edge case.
result: pass
source: coverage present[] (reason: human_judgment) -- rationale: the fixer explicitly requested a human diff read (24-REVIEW-FIX.md CR-02); a green regression test is evidence, not a substitute, for the judgment call.

### 3. Automated coverage confirmation (60 deliverables across plans 24-01..24-13 auto-covered by passing tests)
expected: Every deliverable in plans 24-01 through 24-13 is covered by passing automated tests (entries 4-63 below carry the exact test refs). Per plan -- 24-01 (7, HITL-01/02) Parley suspend + resume_with core in WarEngine, fail-closed plain resume, nested-child typed error, serde round trip; 24-02 (6, HITL-01) Gate node raise/deliver, approval-gate routing, Custom evaluator on Gate output_field, validate rejections, fingerprint v4; 24-03 (5, HITL-01/02) parley.* InputMapping namespace, parley.-prefix schema rejection, envelope next.parley, raise-time default validation; 24-04 (3, HITL-02) total pre-persist validation, partial answers at the same superstep, lazy expiry FailRun/ResumeWithDefault; 24-05 (3, HITL-01/02) E2E-2 across a process drop, two parleys in one Waypoint, ten concurrent resumes with no cross-thread leakage; 24-06 (5, HITL-03) additive fork_of, branch queryability from summaries, ThreadId::child_on_branch, three-backend contract suite (Postgres Tier 2 self-skips); 24-07 (5, HITL-03) replay/fork, byte-identical mainline, branch-scoped child threads, ChronicleService; 24-08 (5, HITL-04) ShutdownCoordinator/RunGuard, one shared grace deadline, aborted nodes re-listed and rerun once, dispatch-order bookkeeping, EngineConfig shutdown fields; 24-09 (4, HITL-04) server + ServiceRunner shutdown wiring, resume-after-Halted at process level, k8s terminationGracePeriodSeconds 60, MIGRATION.md; 24-10 (3, HITL-05) ParleyPort, facade adapter + GraphRegistry, WaypointStoreConfig; 24-11 (5, HITL-05) three /v1/threads routes, exact D-25 status mapping, history pagination, regenerated openapi.json, server composition; 24-12 (5, HITL-01) user guide, MIGRATION 9.2/9.6, CHANGELOG, traceability matrix, phase gate evidence; 24-13 (4, HITL-05) 403 admin-gate documented in MIGRATION/CHANGELOG/mdBook and asserted in openapi tests. Confirm you accept these as covered.
result: pass
source: coverage all_auto_covered confirmation summary (workflow rule: never zero checkpoints for auto-covered plans)

### 4. [24-01 D1] A node returning NextStep::Parley suspends the run (not fails it): exactly one multi-parley AwaitingInput Waypoint persisted, RunOutcome::AwaitingInput returned
expected: A node returning NextStep::Parley suspends the run (not fails it): exactly one multi-parley AwaitingInput Waypoint persisted, RunOutcome::AwaitingInput returned
result: pass
source: automated
coverage_id: 24-01/D1
requirement: HITL-01
covered_by: crates/paladin-battalion/src/engine/superstep.rs#parley_suspends_run_and_persists_awaiting_input ; crates/paladin-battalion/src/engine/superstep.rs#awaiting_input_vanguard_is_exactly_the_parleying_nodes

### 5. [24-01 D2] Peer nodes merge normally alongside a parleying node; the parleying node's own delta also merges at raise time and its outcome is recorded as Parleyed
expected: Peer nodes merge normally alongside a parleying node; the parleying node's own delta also merges at raise time and its outcome is recorded as Parleyed
result: pass
source: automated
coverage_id: 24-01/D2
requirement: HITL-01
covered_by: crates/paladin-battalion/src/engine/superstep.rs#parley_waypoint_records_parleyed_outcome_and_merges_peer_deltas

### 6. [24-01 D3] WarEngine::resume_with delivers a ParleyResponse to the paused node's continuation via NodeContext.parley_response() and the run reaches RunOutcome::Completed
expected: WarEngine::resume_with delivers a ParleyResponse to the paused node's continuation via NodeContext.parley_response() and the run reaches RunOutcome::Completed
result: pass
source: automated
coverage_id: 24-01/D3
requirement: HITL-02
covered_by: crates/paladin-battalion/src/engine/mod.rs#parley_suspends_and_resumes_end_to_end

### 7. [24-01 D4] resume_with rejects an unknown parley_id (EngineError::UnknownParleyId) and writes no Waypoint
expected: resume_with rejects an unknown parley_id (EngineError::UnknownParleyId) and writes no Waypoint
result: pass
source: automated
coverage_id: 24-01/D4
requirement: HITL-02
covered_by: crates/paladin-battalion/src/engine/mod.rs#resume_with_unknown_parley_id_fails_and_writes_no_waypoint

### 8. [24-01 D5] Plain resume/resume_with_options against an AwaitingInput thread fails closed with EngineError::ThreadAwaitingInput, writing no Waypoint (RESEARCH.md Pitfall 2)
expected: Plain resume/resume_with_options against an AwaitingInput thread fails closed with EngineError::ThreadAwaitingInput, writing no Waypoint (RESEARCH.md Pitfall 2)
result: pass
source: automated
coverage_id: 24-01/D5
requirement: HITL-02
covered_by: crates/paladin-battalion/src/engine/mod.rs#plain_resume_refuses_awaiting_input_thread ; crates/paladin-battalion/src/engine/mod.rs#plain_resume_still_continues_a_halted_thread

### 9. [24-01 D6] A nested NodeSpec::Battalion child that suspends fails the parent with the structured EngineError::ParleyInChildUnsupported, naming the node and child thread
expected: A nested NodeSpec::Battalion child that suspends fails the parent with the structured EngineError::ParleyInChildUnsupported, naming the node and child thread
result: pass
source: automated
coverage_id: 24-01/D6
requirement: HITL-01
covered_by: crates/paladin-battalion/src/engine/mod.rs#parley_in_battalion_child_is_typed_error

### 10. [24-01 D7] The reshaped AwaitingInput Waypoint payload round-trips through serde with both parleys and responses preserved
expected: The reshaped AwaitingInput Waypoint payload round-trips through serde with both parleys and responses preserved
result: pass
source: automated
coverage_id: 24-01/D7
requirement: HITL-01
covered_by: crates/paladin-core/src/platform/container/waypoint.rs#awaiting_input_status_round_trips_through_serde

### 4. [24-02 D1] A Gate node raises a ParleyRequest on first visit, rendering prompt/payload from the Battlefield through InputMapping, and stamps expires_at = now + expires_in
expected: A Gate node raises a ParleyRequest on first visit, rendering prompt/payload from the Battlefield through InputMapping, and stamps expires_at = now + expires_in
result: pass
source: automated
coverage_id: 24-02/D1
requirement: HITL-01
covered_by: crates/paladin-battalion/src/engine/mod.rs#gate_raises_parley_on_first_visit ; crates/paladin-battalion/src/engine/mod.rs#gate_stamps_expires_at_from_expires_in

### 5. [24-02 D2] On the post-resume visit a Gate writes the normalised response value to output_field (Bool or String Approval delivery, Choice/FreeText passthrough) or returns a StateEdit delta, then routes via static edges
expected: On the post-resume visit a Gate writes the normalised response value to output_field (Bool or String Approval delivery, Choice/FreeText passthrough) or returns a StateEdit delta, then routes via static edges
result: pass
source: automated
coverage_id: 24-02/D2
requirement: HITL-01
covered_by: crates/paladin-battalion/src/engine/mod.rs#gate_writes_normalised_approval_value_on_resume ; crates/paladin-battalion/src/engine/mod.rs#gate_writes_string_true_false_for_string_output_field ; crates/paladin-battalion/src/engine/mod.rs#gate_state_edit_returns_delta_and_writes_no_output_field

### 6. [24-02 D3] An approval gate expressed as one Gate node plus Contains(\"true\")/Contains(\"false\") edges routes to the correct branch on approval and denial (the E2E-2 shape)
expected: An approval gate expressed as one Gate node plus Contains(\"true\")/Contains(\"false\") edges routes to the correct branch on approval and denial (the E2E-2 shape)
result: pass
source: automated
coverage_id: 24-02/D3
requirement: HITL-01
covered_by: crates/paladin-battalion/src/engine/mod.rs#approval_gate_routes_both_branches

### 7. [24-02 D4] A registered Custom edge evaluator on an edge whose source is a Gate receives the Gate's output_field value, not the whole serialised Battlefield
expected: A registered Custom edge evaluator on an edge whose source is a Gate receives the Gate's output_field value, not the whole serialised Battlefield
result: pass
source: automated
coverage_id: 24-02/D4
requirement: HITL-01
covered_by: crates/paladin-battalion/src/engine/mod.rs#gate_source_uses_output_field_for_custom_evaluator

### 8. [24-02 D5] WarGraph::validate rejects every invalid Gate wiring combination (output_field required/absent by kind, unknown field, incompatible type, invalid on_expire default) with a distinct typed EngineError variant, and accepts the valid E2E-2 shape
expected: WarGraph::validate rejects every invalid Gate wiring combination (output_field required/absent by kind, unknown field, incompatible type, invalid on_expire default) with a distinct typed EngineError variant, and accepts the valid E2E-2 shape
result: pass
source: automated
coverage_id: 24-02/D5
requirement: HITL-01
covered_by: crates/paladin-battalion/src/engine/graph.rs#gate_requires_output_field_for_approval_choice_freetext ; crates/paladin-battalion/src/engine/graph.rs#gate_rejects_output_field_for_state_edit ; crates/paladin-battalion/src/engine/graph.rs#gate_output_field_must_exist_in_schema ; crates/paladin-battalion/src/engine/graph.rs#gate_output_field_type_must_be_compatible ; crates/paladin-battalion/src/engine/graph.rs#gate_resume_with_default_value_is_validated_at_graph_validate_time ; crates/paladin-battalion/src/engine/graph.rs#gate_with_valid_wiring_passes_validation

### 9. [24-02 D6] GRAPH_FINGERPRINT_VERSION is v4 and the fingerprint hashes a sorted, length-prefixed ;gates: section over kind/output_field/choices/on_expire-kind, excluding prompt_template/payload_template/expires_in
expected: GRAPH_FINGERPRINT_VERSION is v4 and the fingerprint hashes a sorted, length-prefixed ;gates: section over kind/output_field/choices/on_expire-kind, excluding prompt_template/payload_template/expires_in
result: pass
source: automated
coverage_id: 24-02/D6
requirement: HITL-01
covered_by: crates/paladin-battalion/src/engine/graph.rs#fingerprint_version_is_v4 ; crates/paladin-battalion/src/engine/graph.rs#fingerprint_golden_hex_v4 ; crates/paladin-battalion/src/engine/graph.rs#fingerprint_differs_on_gate_kind ; crates/paladin-battalion/src/engine/graph.rs#fingerprint_differs_on_output_field ; crates/paladin-battalion/src/engine/graph.rs#fingerprint_differs_on_choices ; crates/paladin-battalion/src/engine/graph.rs#fingerprint_differs_on_on_expire_kind ; crates/paladin-battalion/src/engine/graph.rs#fingerprint_ignores_gate_templates_and_expiry ; crates/paladin-battalion/src/engine/graph.rs#fingerprint_gate_section_is_length_prefixed

### 4. [24-03 D1] InputMapping::render resolves parley.value/prompt/kind/responded_by from NodeContext's ParleyResponse, never the Battlefield, even when a schema field shares the name
expected: InputMapping::render resolves parley.value/prompt/kind/responded_by from NodeContext's ParleyResponse, never the Battlefield, even when a schema field shares the name
result: pass
source: automated
coverage_id: 24-03/D1
requirement: HITL-01
covered_by: crates/paladin-battalion/src/engine/input_mapping.rs#parley_namespace_resolves_from_node_context ; crates/paladin-battalion/src/engine/input_mapping.rs#parley_namespace_never_reads_battlefield

### 5. [24-03 D2] A parley-namespaced placeholder with no parley context, or an unrecognized key, is the typed InputMappingError::UndeclaredField, never a silent Battlefield fallthrough or empty substitution
expected: A parley-namespaced placeholder with no parley context, or an unrecognized key, is the typed InputMappingError::UndeclaredField, never a silent Battlefield fallthrough or empty substitution
result: pass
source: automated
coverage_id: 24-03/D2
requirement: HITL-01
covered_by: crates/paladin-battalion/src/engine/input_mapping.rs#parley_namespace_without_context_is_typed_error ; crates/paladin-battalion/src/engine/input_mapping.rs#parley_namespace_unknown_key_is_typed_error ; crates/paladin-battalion/src/engine/input_mapping.rs#responded_by_none_renders_empty_for_defaulted_response

### 6. [24-03 D3] WarGraph::validate rejects any schema field whose name starts with the parley. prefix, mirroring the muster. rule
expected: WarGraph::validate rejects any schema field whose name starts with the parley. prefix, mirroring the muster. rule
result: pass
source: automated
coverage_id: 24-03/D3
requirement: HITL-01
covered_by: crates/paladin-battalion/src/engine/graph.rs#schema_field_with_parley_prefix_is_rejected_by_validate

### 7. [24-03 D4] A Paladin node raises a parley through the structured directive envelope's next.parley key: kind/prompt required, payload/choices/expires_in_secs/on_expire optional; the parser stamps parley_id/node_id/created_at and computes expires_at
expected: A Paladin node raises a parley through the structured directive envelope's next.parley key: kind/prompt required, payload/choices/expires_in_secs/on_expire optional; the parser stamps parley_id/node_id/created_at and computes expires_at
result: pass
source: automated
coverage_id: 24-03/D4
requirement: HITL-01
covered_by: crates/paladin-battalion/src/engine/directive_parser.rs#envelope_parley_key_parses_to_next_step_parley ; crates/paladin-battalion/src/engine/directive_parser.rs#envelope_parley_stamps_expires_at_from_expires_in_secs ; crates/paladin-battalion/src/engine/directive_parser.rs#envelope_parley_defaults_on_expire_to_fail_run ; crates/paladin-battalion/src/engine/mod.rs#paladin_node_parley_round_trips_to_awaiting_input

### 8. [24-03 D5] An on_expire: ResumeWithDefault value that fails validate_parley_value_for_kind is a hard DirectiveParseError at raise time, regardless of on_parse_error; a malformed parley shape routes through the existing OnParseError policy instead
expected: An on_expire: ResumeWithDefault value that fails validate_parley_value_for_kind is a hard DirectiveParseError at raise time, regardless of on_parse_error; a malformed parley shape routes through the existing OnParseError policy instead
result: pass
source: automated
coverage_id: 24-03/D5
requirement: HITL-02
covered_by: crates/paladin-battalion/src/engine/directive_parser.rs#envelope_parley_resume_with_default_is_validated_at_raise_time ; crates/paladin-battalion/src/engine/directive_parser.rs#envelope_parley_malformed_shape_uses_on_parse_error_policy

### 4. [24-04 D1] resume_with validates every submitted response totally before persisting anything: UnknownParleyId, ParleyAlreadyAnswered (new), ResponseShapeInvalid (new, per-kind), any error leaves the thread suspended with no Waypoint written
expected: resume_with validates every submitted response totally before persisting anything: UnknownParleyId, ParleyAlreadyAnswered (new), ResponseShapeInvalid (new, per-kind), any error leaves the thread suspended with no Waypoint written
result: pass
source: automated
coverage_id: 24-04/D1
requirement: HITL-02
covered_by: crates/paladin-battalion/src/engine/mod.rs#resume_with_rejects_unknown_parley_id ; crates/paladin-battalion/src/engine/mod.rs#resume_with_rejects_already_answered_parley ; crates/paladin-battalion/src/engine/mod.rs#resume_with_rejects_wrong_shape_per_kind ; crates/paladin-battalion/src/engine/mod.rs#state_edit_unknown_schema_field_rejects_the_response_not_the_run ; crates/paladin-battalion/src/engine/mod.rs#resume_with_validation_is_total_before_any_write ; crates/paladin-battalion/src/engine/mod.rs#resume_with_checks_graph_fingerprint ; crates/paladin-battalion/src/engine/mod.rs#resume_with_rejects_non_awaiting_input_thread ; crates/paladin-battalion/src/engine/mod.rs#resume_with_parley_ids_are_scoped_to_the_requested_thread

### 5. [24-04 D2] A valid but partial submission persists a new AwaitingInput Waypoint at the SAME superstep with responses extended; answering the last outstanding parley advances the run instead of writing another AwaitingInput Waypoint; a partially-answered suspension is queryable from a cold store handle; re-submission after a simulated save failure is safe (durable consumption); the partial-answer chain is linear (parent_waypoint_id)
expected: A valid but partial submission persists a new AwaitingInput Waypoint at the SAME superstep with responses extended; answering the last outstanding parley advances the run instead of writing another AwaitingInput Waypoint; a partially-answered suspension is queryable from a cold store handle; re-submission after a simulated save failure is safe (durable consumption); the partial-answer chain is linear (parent_waypoint_id)
result: pass
source: automated
coverage_id: 24-04/D2
requirement: HITL-02
covered_by: crates/paladin-battalion/src/engine/mod.rs#partial_answer_persists_new_awaiting_input_at_same_superstep ; crates/paladin-battalion/src/engine/mod.rs#partial_answer_returns_only_remaining_parleys ; crates/paladin-battalion/src/engine/mod.rs#answering_the_last_parley_advances_the_run ; crates/paladin-battalion/src/engine/mod.rs#partial_answer_state_is_queryable_from_the_waypoint_alone ; crates/paladin-battalion/src/engine/mod.rs#resubmitting_responses_after_a_failed_save_is_safe ; crates/paladin-battalion/src/engine/mod.rs#chain_of_partial_answers_is_linear

### 6. [24-04 D3] Lazy expiry evaluated at resume time only: FailRun persists a Failed Waypoint and fails the call, and the thread is thereafter unresumable via resume/resume_with; ResumeWithDefault substitutes a pre-validated default carrying responded_by: None and defaulted: true and lets the run proceed; a future expires_at is not treated as expired; the defaulted marker survives a serde round trip; an expired FailRun parley fails the whole submission even when unrelated to the caller's own responses
expected: Lazy expiry evaluated at resume time only: FailRun persists a Failed Waypoint and fails the call, and the thread is thereafter unresumable via resume/resume_with; ResumeWithDefault substitutes a pre-validated default carrying responded_by: None and defaulted: true and lets the run proceed; a future expires_at is not treated as expired; the defaulted marker survives a serde round trip; an expired FailRun parley fails the whole submission even when unrelated to the caller's own responses
result: pass
source: automated
coverage_id: 24-04/D3
requirement: HITL-02
covered_by: crates/paladin-battalion/src/engine/mod.rs#expired_parley_with_fail_run_persists_failed_waypoint ; crates/paladin-battalion/src/engine/mod.rs#expired_fail_run_thread_is_not_resumable ; crates/paladin-battalion/src/engine/mod.rs#expired_parley_with_resume_with_default_substitutes_value ; crates/paladin-battalion/src/engine/mod.rs#expiry_is_evaluated_only_at_resume_time ; crates/paladin-battalion/src/engine/mod.rs#defaulted_marker_is_persisted_and_queryable ; crates/paladin-battalion/src/engine/mod.rs#expired_and_valid_responses_in_one_submission

### 4. [24-05 D1] E2E-2 (approval gate, both branches) passes across a real process-drop simulation over a shared on-disk Waypoint store; a fresh engine instance holds no leftover resource for a suspended thread; the graph is expressed as exactly one NodeSpec::Gate plus two Contains edges
expected: E2E-2 (approval gate, both branches) passes across a real process-drop simulation over a shared on-disk Waypoint store; a fresh engine instance holds no leftover resource for a suspended thread; the graph is expressed as exactly one NodeSpec::Gate plus two Contains edges
result: pass
source: automated
coverage_id: 24-05/D1
requirement: HITL-01, HITL-02
covered_by: tests/integration/e2e_approval_gate_test.rs#e2e2_approval_branch_survives_process_drop ; tests/integration/e2e_approval_gate_test.rs#e2e2_denial_branch_survives_process_drop ; tests/integration/e2e_approval_gate_test.rs#e2e2_suspended_thread_holds_no_engine_resources ; tests/integration/e2e_approval_gate_test.rs#e2e2_graph_is_three_lines_of_graph

### 5. [24-05 D2] A superstep in which two nodes parley (one Gate, one Function raising NextStep::Parley directly) persists exactly one AwaitingInput Waypoint with both requests and zero responses; answering one keeps the thread suspended with one persisted response; answering the second continues the run; parley order is stable by node id; a partial answer survives a process drop -- every assertion read through WaypointPort
expected: A superstep in which two nodes parley (one Gate, one Function raising NextStep::Parley directly) persists exactly one AwaitingInput Waypoint with both requests and zero responses; answering one keeps the thread suspended with one persisted response; answering the second continues the run; parley order is stable by node id; a partial answer survives a process drop -- every assertion read through WaypointPort
result: pass
source: automated
coverage_id: 24-05/D2
requirement: HITL-01
covered_by: tests/integration/multi_parley_suspension_test.rs#two_parleys_persist_as_one_waypoint_with_two_requests ; tests/integration/multi_parley_suspension_test.rs#answering_one_of_two_keeps_the_thread_suspended ; tests/integration/multi_parley_suspension_test.rs#answering_the_second_continues_the_run ; tests/integration/multi_parley_suspension_test.rs#multi_parley_list_order_is_stable_by_node_id ; tests/integration/multi_parley_suspension_test.rs#multi_parley_survives_process_drop_mid_partial

### 6. [24-05 D3] Ten suspended threads resumed concurrently on a multi_thread runtime all reach RunOutcome::Completed with an exact count of ten, zero failures, zero still-suspended, no cross-thread response leakage (a parley id from one thread is UnknownParleyId against another), under an explicit timeout guard
expected: Ten suspended threads resumed concurrently on a multi_thread runtime all reach RunOutcome::Completed with an exact count of ten, zero failures, zero still-suspended, no cross-thread response leakage (a parley id from one thread is UnknownParleyId against another), under an explicit timeout guard
result: pass
source: automated
coverage_id: 24-05/D3
requirement: HITL-02
covered_by: tests/integration/parley_resume_stress_test.rs#ten_suspended_threads_resume_concurrently ; tests/integration/parley_resume_stress_test.rs#concurrent_resumes_do_not_leak_responses_across_threads ; tests/integration/parley_resume_stress_test.rs#stress_run_completes_within_the_timeout_guard

### 4. [24-06 D1] Waypoint and WaypointSummary each carry an additive fork_of field with serde default; a pre-D-14 payload missing the key deserialises as None
expected: Waypoint and WaypointSummary each carry an additive fork_of field with serde default; a pre-D-14 payload missing the key deserialises as None
result: pass
source: automated
coverage_id: 24-06/D1
requirement: HITL-03
covered_by: crates/paladin-core/src/platform/container/waypoint.rs#waypoint_payload_without_fork_of_deserializes_as_none ; crates/paladin-ports/src/output/waypoint_port.rs#waypoint_summary_payload_without_fork_of_deserializes_as_none

### 5. [24-06 D2] A branch is queryable: the fork's first Waypoint carries fork_of Some(from), every subsequent Waypoint on the branch inherits the same value, mainline stays None, and a fork of a fork carries the newer root
expected: A branch is queryable: the fork's first Waypoint carries fork_of Some(from), every subsequent Waypoint on the branch inherits the same value, mainline stays None, and a fork of a fork carries the newer root
result: pass
source: automated
coverage_id: 24-06/D2
requirement: HITL-03
covered_by: crates/paladin-battalion/src/engine/superstep.rs#mainline_waypoints_carry_no_fork_of ; crates/paladin-battalion/src/engine/superstep.rs#branch_waypoints_inherit_fork_of_from_the_branch_root ; crates/paladin-battalion/src/engine/superstep.rs#fork_of_a_fork_carries_the_newer_root

### 6. [24-06 D3] The whole branch tree is reconstructible from WaypointSummary alone, without loading full Waypoints
expected: The whole branch tree is reconstructible from WaypointSummary alone, without loading full Waypoints
result: pass
source: automated
coverage_id: 24-06/D3
requirement: HITL-03
covered_by: crates/paladin-ports/src/output/waypoint_port.rs#branch_tree_reconstructs_from_summaries_alone

### 7. [24-06 D4] ThreadId::child_on_branch derives an injective, collision-free branch-scoped child thread id using the same length-prefixed encoding as ThreadId::child, extended to three components, never equal to the mainline child's id
expected: ThreadId::child_on_branch derives an injective, collision-free branch-scoped child thread id using the same length-prefixed encoding as ThreadId::child, extended to three components, never equal to the mainline child's id
result: pass
source: automated
coverage_id: 24-06/D4
requirement: HITL-03
covered_by: crates/paladin-core/src/platform/container/waypoint.rs#child_on_branch_is_injective ; crates/paladin-core/src/platform/container/waypoint.rs#child_on_branch_differs_from_child ; crates/paladin-core/src/platform/container/waypoint.rs#child_on_branch_is_deterministic ; crates/paladin-core/src/platform/container/waypoint.rs#child_on_branch_rejects_invalid_inputs ; crates/paladin-core/src/platform/container/waypoint.rs (doc test) ThreadId::child_on_branch

### 8. [24-06 D5] The three-backend contract suite gains round-trip cases for the AwaitingInput { parleys, responses } payload and the fork_of field, and a latest-across-branches ordering case; InMemory and SQLite run locally, Postgres is Tier 2 and self-skips without Docker
expected: The three-backend contract suite gains round-trip cases for the AwaitingInput { parleys, responses } payload and the fork_of field, and a latest-across-branches ordering case; InMemory and SQLite run locally, Postgres is Tier 2 and self-skips without Docker
result: pass
source: automated
coverage_id: 24-06/D5
requirement: HITL-03
covered_by: crates/paladin-storage/src/waypoint/contract_tests.rs#awaiting_input_payload_round_trips (InMemory + SQLite wrappers) ; crates/paladin-storage/src/waypoint/contract_tests.rs#fork_of_round_trips (InMemory + SQLite wrappers) ; crates/paladin-storage/src/waypoint/contract_tests.rs#latest_prefers_most_recently_created_across_branches (InMemory + SQLite wrappers) ; crates/paladin-storage/src/waypoint/retention.rs#retention_protects_awaiting_input_on_any_branch

### 4. [24-07 D1] WarEngine::replay(graph, thread, from) re-enters the superstep loop from get(thread, from) with parent = from, fork_of = Some(from), superstep numbering continuing at from.superstep + 1, after checking the graph fingerprint
expected: WarEngine::replay(graph, thread, from) re-enters the superstep loop from get(thread, from) with parent = from, fork_of = Some(from), superstep numbering continuing at from.superstep + 1, after checking the graph fingerprint
result: pass
source: automated
coverage_id: 24-07/D1
requirement: HITL-03
covered_by: crates/paladin-battalion/src/engine/mod.rs#replay_creates_a_new_branch_from_the_given_waypoint ; crates/paladin-battalion/src/engine/mod.rs#replay_rejects_unknown_waypoint ; crates/paladin-battalion/src/engine/mod.rs#replay_rejects_fingerprint_mismatch

### 5. [24-07 D2] WarEngine::fork(graph, thread, from, edit) does the same and merges the StateDelta edit through the schema's dispatch rules before the first forked superstep
expected: WarEngine::fork(graph, thread, from, edit) does the same and merges the StateDelta edit through the schema's dispatch rules before the first forked superstep
result: pass
source: automated
coverage_id: 24-07/D2
requirement: HITL-03
covered_by: crates/paladin-battalion/src/engine/mod.rs#fork_with_edit_flips_a_conditional_edge ; crates/paladin-battalion/src/engine/mod.rs#fork_merges_the_edit_before_the_first_forked_superstep ; crates/paladin-battalion/src/engine/mod.rs#fork_rejects_an_edit_the_schema_does_not_accept

### 6. [24-07 D3] Immutability is byte-for-byte: every mainline Waypoint serialises to identical bytes before and after replay, and calling replay twice from the same Waypoint leaves the mainline byte-identical after both calls without disturbing the other branch
expected: Immutability is byte-for-byte: every mainline Waypoint serialises to identical bytes before and after replay, and calling replay twice from the same Waypoint leaves the mainline byte-identical after both calls without disturbing the other branch
result: pass
source: automated
coverage_id: 24-07/D3
requirement: HITL-03
covered_by: crates/paladin-battalion/src/engine/mod.rs#replay_leaves_the_mainline_byte_identical ; crates/paladin-battalion/src/engine/mod.rs#replay_twice_is_safe

### 7. [24-07 D4] A branch runs its NodeSpec::Battalion children under ThreadId::child_on_branch(parent, branch_root, node), so latest(child_thread) on a fork never resolves the mainline child's history and the mainline child is untouched
expected: A branch runs its NodeSpec::Battalion children under ThreadId::child_on_branch(parent, branch_root, node), so latest(child_thread) on a fork never resolves the mainline child's history and the mainline child is untouched
result: pass
source: automated
coverage_id: 24-07/D4
requirement: HITL-03
covered_by: tests/integration/subgraph_formation_in_campaign_test.rs#fork_child_thread_differs_from_mainline_child_thread ; tests/integration/subgraph_formation_in_campaign_test.rs#fork_does_not_touch_mainline_child_waypoints ; tests/integration/subgraph_formation_in_campaign_test.rs#latest_on_a_fork_child_thread_does_not_resolve_the_mainline_child ; tests/integration/subgraph_formation_in_campaign_test.rs#mainline_runs_keep_using_child

### 8. [24-07 D5] ChronicleService exposes history (newest-first summaries with lineage), inspect (a full Waypoint) and latest_on_branch over Arc<dyn WaypointPort> with no paladin-battalion dependency; latest_on_branch requires no full-Waypoint loads
expected: ChronicleService exposes history (newest-first summaries with lineage), inspect (a full Waypoint) and latest_on_branch over Arc<dyn WaypointPort> with no paladin-battalion dependency; latest_on_branch requires no full-Waypoint loads
result: pass
source: automated
coverage_id: 24-07/D5
requirement: HITL-03
covered_by: src/application/services/chronicle.rs#chronicle_history_returns_newest_first_summaries_with_lineage ; src/application/services/chronicle.rs#chronicle_history_honours_limit_and_before ; src/application/services/chronicle.rs#chronicle_inspect_returns_the_full_waypoint ; src/application/services/chronicle.rs#chronicle_latest_on_branch_filters_by_fork_of ; src/application/services/chronicle.rs#chronicle_latest_on_branch_needs_no_full_waypoint_loads ; src/application/services/chronicle.rs#chronicle_empty_thread_returns_empty_history ; cargo test -p paladin-ai --doc chronicle (4 doc tests)

### 4. [24-08 D1] ShutdownCoordinator/RunGuard: a root CancellationToken + in-flight counter + Notify; register() returns a child token + RAII guard; cancel_and_wait(grace) cancels the root and waits for idle or the deadline, reporting which
expected: ShutdownCoordinator/RunGuard: a root CancellationToken + in-flight counter + Notify; register() returns a child token + RAII guard; cancel_and_wait(grace) cancels the root and waits for idle or the deadline, reporting which
result: pass
source: automated
coverage_id: 24-08/D1
requirement: HITL-04
covered_by: crates/paladin-battalion/src/engine/shutdown.rs#register_returns_a_child_token_cancelled_by_the_root ; crates/paladin-battalion/src/engine/shutdown.rs#run_guard_decrements_in_flight_on_drop ; crates/paladin-battalion/src/engine/shutdown.rs#run_guard_decrements_in_flight_on_drop_even_when_the_guarded_future_panics ; crates/paladin-battalion/src/engine/shutdown.rs#run_guard_decrements_in_flight_on_drop_even_when_aborted ; crates/paladin-battalion/src/engine/shutdown.rs#cancel_and_wait_returns_when_idle ; crates/paladin-battalion/src/engine/shutdown.rs#cancel_and_wait_returns_at_the_deadline_when_not_idle ; crates/paladin-battalion/src/engine/shutdown.rs#cancel_and_wait_with_zero_registered_runs_returns_immediately ; crates/paladin-battalion/src/engine/shutdown.rs#cancel_and_wait_with_zero_grace_does_not_wait ; crates/paladin-battalion/src/engine/shutdown.rs#coordinator_is_send_sync_and_shareable ; crates/paladin-battalion/src/engine/shutdown.rs#coordinator_is_usable_behind_an_arc_from_multiple_tasks ; cargo test -p paladin-battalion --doc shutdown (10 doc tests)

### 5. [24-08 D2] Mid-superstep grace race: the whole in-flight batch is raced against ONE shared deadline (not per-handle); a node finishing in time merges normally, a node still running at the deadline is aborted and recorded Skipped{reason:\"shutdown\"}, never both
expected: Mid-superstep grace race: the whole in-flight batch is raced against ONE shared deadline (not per-handle); a node finishing in time merges normally, a node still running at the deadline is aborted and recorded Skipped{reason:\"shutdown\"}, never both
result: pass
source: automated
coverage_id: 24-08/D2
requirement: HITL-04
covered_by: crates/paladin-battalion/src/engine/superstep.rs#in_flight_nodes_finishing_inside_grace_merge_normally ; crates/paladin-battalion/src/engine/superstep.rs#over_grace_node_is_aborted_and_recorded_skipped ; crates/paladin-battalion/src/engine/superstep.rs#two_slow_nodes_share_one_deadline ; crates/paladin-battalion/src/engine/superstep.rs#zero_grace_aborts_immediately ; crates/paladin-battalion/src/engine/superstep.rs#boundary_cancellation_behaviour_is_unchanged

### 6. [24-08 D3] Aborted nodes' ids are re-listed in the Halted Waypoint's vanguard alongside the normally computed next Vanguard; resume re-executes exactly once (run_count == 2 across the whole scenario)
expected: Aborted nodes' ids are re-listed in the Halted Waypoint's vanguard alongside the normally computed next Vanguard; resume re-executes exactly once (run_count == 2 across the whole scenario)
result: pass
source: automated
coverage_id: 24-08/D3
requirement: HITL-04
covered_by: crates/paladin-battalion/src/engine/superstep.rs#over_grace_node_is_relisted_in_the_halted_vanguard ; crates/paladin-battalion/src/engine/superstep.rs#resume_reruns_the_skipped_node_exactly_once

### 7. [24-08 D4] Dispatch-order-dependent bookkeeping (completed_records sort, first-failure-wins) is resolved over results re-indexed to dispatch position, never completion order
expected: Dispatch-order-dependent bookkeeping (completed_records sort, first-failure-wins) is resolved over results re-indexed to dispatch position, never completion order
result: pass
source: automated
coverage_id: 24-08/D4
requirement: HITL-04
covered_by: crates/paladin-battalion/src/engine/superstep.rs#completed_records_stay_sorted_by_node_id_after_the_race ; crates/paladin-battalion/src/engine/superstep.rs#first_failure_wins_is_dispatch_order_not_completion_order

### 8. [24-08 D5] EngineConfig gains shutdown_grace_secs (default 30, env APP_ENGINE_SHUTDOWN_GRACE_SECS) and graceful_shutdown (default true, env APP_ENGINE_GRACEFUL_SHUTDOWN); default_engine_config_matches_todays_engine_defaults passes unchanged; neither field ever leaks into EngineLimits or the graph fingerprint
expected: EngineConfig gains shutdown_grace_secs (default 30, env APP_ENGINE_SHUTDOWN_GRACE_SECS) and graceful_shutdown (default true, env APP_ENGINE_GRACEFUL_SHUTDOWN); default_engine_config_matches_todays_engine_defaults passes unchanged; neither field ever leaks into EngineLimits or the graph fingerprint
result: pass
source: automated
coverage_id: 24-08/D5
requirement: HITL-04
covered_by: src/config/engine.rs#engine_config_defaults_shutdown_fields ; src/config/engine.rs#engine_config_reads_shutdown_env_overrides ; src/config/engine.rs#engine_config_validates_shutdown_grace ; src/config/engine.rs#default_engine_config_matches_todays_engine_defaults ; src/config/engine.rs#shutdown_grace_does_not_change_the_graph_fingerprint

### 4. [24-09 D1] Both paladin-server.rs's shutdown_signal and ServiceRunner::wait_for_shutdown cancel a ShutdownCoordinator and wait <= shutdown_grace for in-flight runs (skipped when graceful_shutdown=false)
expected: Both paladin-server.rs's shutdown_signal and ServiceRunner::wait_for_shutdown cancel a ShutdownCoordinator and wait <= shutdown_grace for in-flight runs (skipped when graceful_shutdown=false)
result: pass
source: automated
coverage_id: 24-09/D1
requirement: HITL-04
covered_by: src/bin/paladin-server.rs#shutdown_signal_cancels_the_coordinator ; src/bin/paladin-server.rs#process_waits_up_to_grace_for_in_flight_runs ; src/bin/paladin-server.rs#process_stops_waiting_at_the_grace_deadline ; src/bin/paladin-server.rs#graceful_shutdown_disabled_skips_the_wait ; src/config/setup/service_runner.rs#service_runner_wait_for_shutdown_cancels_the_coordinator

### 5. [24-09 D2] resume continues a Halted thread, asserted explicitly at the process-wiring level (HITL-FR-14)
expected: resume continues a Halted thread, asserted explicitly at the process-wiring level (HITL-FR-14)
result: pass
source: automated
coverage_id: 24-09/D2
requirement: HITL-04
covered_by: src/bin/paladin-server.rs#resume_continues_a_halted_thread_after_process_shutdown

### 6. [24-09 D3] k8s/server/deployment.yaml and k8s/deployment.yaml declare terminationGracePeriodSeconds: 60; k8s/README.md, docs/src/deployment/kubernetes.md and docs/src/deployment/production.md teach the 2x rule and both env vars
expected: k8s/server/deployment.yaml and k8s/deployment.yaml declare terminationGracePeriodSeconds: 60; k8s/README.md, docs/src/deployment/kubernetes.md and docs/src/deployment/production.md teach the 2x rule and both env vars
result: pass
source: automated
coverage_id: 24-09/D3
requirement: HITL-04
covered_by: python3 -c \"import yaml; [yaml.safe_load(open(f)) for f in ['k8s/server/deployment.yaml','k8s/deployment.yaml']]\" (parses, both contain terminationGracePeriodSeconds: 60) ; mdbook build docs (after mdbook-mermaid install) -- 0 broken links

### 7. [24-09 D4] MIGRATION.md M-B-02 worked example, section 9.5 EngineConfig fields, and section 9.8's termination-grace bullet are concrete; sections 9.2/9.6 untouched
expected: MIGRATION.md M-B-02 worked example, section 9.5 EngineConfig fields, and section 9.8's termination-grace bullet are concrete; sections 9.2/9.6 untouched
result: pass
source: automated
coverage_id: 24-09/D4
requirement: HITL-04
covered_by: grep -c APP_ENGINE_SHUTDOWN_GRACE_SECS MIGRATION.md (5 matches) && grep -c terminationGracePeriodSeconds MIGRATION.md (5 matches) ; git diff -U0 MIGRATION.md hunks confined to lines ~16, ~70-116, ~172-175, ~196 -- no touch to the 9.2 register (~74-94) or 9.6 (~139)

### 4. [24-10 D1] ParleyPort trait + ResumeAccepted + ParleyError land in paladin-ports naming only core types, with a typed error for every documented validation case
expected: ParleyPort trait + ResumeAccepted + ParleyError land in paladin-ports naming only core types, with a typed error for every documented validation case
result: pass
source: automated
coverage_id: 24-10/D1
requirement: HITL-05
covered_by: crates/paladin-ports/src/input/parley_port.rs#parley_port_is_object_safe ; crates/paladin-ports/src/input/parley_port.rs#parley_error_covers_every_validation_case ; crates/paladin-ports/src/input/parley_port.rs#parley_error_display_names_the_parley_id ; crates/paladin-ports/src/input/parley_port.rs#resume_accepted_carries_thread_and_state_handle ; cargo test -p paladin-ports --doc parley_port (2 doc tests) ; cargo tree -p paladin-web -e normal (no paladin-battalion edge)

### 5. [24-10 D2] Facade ParleyPortAdapter + GraphRegistry validate synchronously (typed errors before any spawn, nothing persisted on rejection), spawn the continuation registered with ShutdownCoordinator only when valid-and-complete, and resolve graphs strictly by fingerprint with no fallback
expected: Facade ParleyPortAdapter + GraphRegistry validate synchronously (typed errors before any spawn, nothing persisted on rejection), spawn the continuation registered with ShutdownCoordinator only when valid-and-complete, and resolve graphs strictly by fingerprint with no fallback
result: pass
source: automated
coverage_id: 24-10/D2
requirement: HITL-05
covered_by: src/application/services/parley/adapter.rs#adapter_validates_synchronously_and_returns_typed_errors ; src/application/services/parley/adapter.rs#adapter_persists_nothing_on_a_validation_error ; src/application/services/parley/adapter.rs#adapter_spawns_the_continuation_and_returns_immediately ; src/application/services/parley/adapter.rs#spawned_continuation_is_registered_with_the_coordinator ; src/application/services/parley/adapter.rs#unknown_thread_is_thread_not_found ; src/application/services/parley/adapter.rs#unregistered_fingerprint_is_graph_not_registered ; src/application/services/parley/adapter.rs#non_awaiting_thread_is_thread_not_awaiting_input ; src/application/services/parley/adapter.rs#every_engine_error_maps_to_a_distinct_parley_error ; src/application/services/parley/registry.rs#registry_resolves_by_fingerprint ; src/application/services/parley/registry.rs#unregistered_fingerprint_resolves_to_none ; src/application/services/parley/registry.rs#re_registering_the_same_fingerprint_replaces_the_entry ; cargo test --doc -p paladin-ai parley (1 doc test)

### 6. [24-10 D3] WaypointStoreConfig defaults to disabled, validates sqlite path and postgres url_env presence/resolvability, reads APP_-prefixed env overrides, and never touches src/config/settings.rs
expected: WaypointStoreConfig defaults to disabled, validates sqlite path and postgres url_env presence/resolvability, reads APP_-prefixed env overrides, and never touches src/config/settings.rs
result: pass
source: automated
coverage_id: 24-10/D3
requirement: HITL-05
covered_by: src/config/waypoint_store.rs#waypoint_store_config_defaults_to_disabled ; src/config/waypoint_store.rs#waypoint_store_config_reads_env_overrides ; src/config/waypoint_store.rs#waypoint_store_config_validates_backend_parameters ; src/config/waypoint_store.rs#waypoint_store_config_postgres_reads_url_from_env_name_not_inline ; cargo test --doc -p paladin-ai waypoint_store (2 doc tests) ; git diff -- src/config/settings.rs (empty)

### 4. [24-11 D1] ThreadApiState + thread_router + the three handlers, mirroring AgentApiState's shape with no paladin-battalion dependency; every route answers 501 naming the config key when unwired
expected: ThreadApiState + thread_router + the three handlers, mirroring AgentApiState's shape with no paladin-battalion dependency; every route answers 501 naming the config key when unwired
result: pass
source: automated
coverage_id: 24-11/D1
requirement: HITL-05
covered_by: crates/paladin-web/src/thread_controller.rs#get_thread_state_returns_status_and_parleys_when_suspended ; crates/paladin-web/src/thread_controller.rs#get_thread_state_omits_parleys_when_not_suspended ; crates/paladin-web/src/thread_controller.rs#get_thread_state_unknown_thread_is_404 ; crates/paladin-web/src/thread_controller.rs#thread_routes_return_501_when_no_backend_is_wired ; crates/paladin-web/src/thread_controller.rs#thread_routes_require_authentication ; cargo tree -p paladin-web -e normal (no paladin-battalion edge)

### 5. [24-11 D2] POST /threads/{id}/resume status mapping is exact per D-25: 202 accepted, 404 unknown thread, both 409 codes distinct, all four 400 cases carry parley_id in details
expected: POST /threads/{id}/resume status mapping is exact per D-25: 202 accepted, 404 unknown thread, both 409 codes distinct, all four 400 cases carry parley_id in details
result: pass
source: automated
coverage_id: 24-11/D2
requirement: HITL-05
covered_by: crates/paladin-web/src/thread_controller.rs#post_resume_returns_202_with_thread_and_state_url ; crates/paladin-web/src/thread_controller.rs#post_resume_unknown_thread_is_404 ; crates/paladin-web/src/thread_controller.rs#post_resume_on_running_thread_is_409_thread_not_awaiting_input ; crates/paladin-web/src/thread_controller.rs#post_resume_with_unregistered_graph_is_409_graph_not_registered ; crates/paladin-web/src/thread_controller.rs#post_resume_bad_response_is_400_with_parley_id_in_details

### 6. [24-11 D3] GET /threads/{id}/history paginates with limit (<=100) and an opaque waypoint_id cursor, no overlap across pages, null next_cursor on the last page
expected: GET /threads/{id}/history paginates with limit (<=100) and an opaque waypoint_id cursor, no overlap across pages, null next_cursor on the last page
result: pass
source: automated
coverage_id: 24-11/D3
requirement: HITL-05
covered_by: crates/paladin-web/src/thread_controller.rs#get_thread_history_paginates_with_limit_and_cursor ; crates/paladin-web/src/thread_controller.rs#get_thread_history_rejects_limit_above_100

### 7. [24-11 D4] openapi.json is regenerated with the three thread paths (full status sets, both 409 codes documented) and every pre-existing agent path is byte-identical
expected: openapi.json is regenerated with the three thread paths (full status sets, both 409 codes documented) and every pre-existing agent path is byte-identical
result: pass
source: automated
coverage_id: 24-11/D4
requirement: HITL-05
covered_by: crates/paladin-web/src/openapi.rs#openapi_lists_the_three_thread_paths ; crates/paladin-web/src/openapi.rs#openapi_thread_paths_document_every_status ; crates/paladin-web/src/openapi.rs#openapi_matches_committed_baseline ; crates/paladin-web/src/openapi.rs#openapi_pre_existing_agent_paths_are_unchanged ; git diff --stat crates/paladin-web/openapi.json (553 insertions, 0 deletions)

### 8. [24-11 D5] paladin-server composes the thread surface behind the same auth as /v1/agents/*, wires a real backend from WaypointStoreConfig, and leaves AgentApiState untouched
expected: paladin-server composes the thread surface behind the same auth as /v1/agents/*, wires a real backend from WaypointStoreConfig, and leaves AgentApiState untouched
result: pass
source: automated
coverage_id: 24-11/D5
requirement: HITL-05
covered_by: src/bin/paladin-server.rs#server_wires_no_waypoint_backend_by_default ; src/bin/paladin-server.rs#server_wires_sqlite_backend_when_configured ; src/bin/paladin-server.rs#thread_router_is_merged_alongside_agent_router ; src/bin/paladin-server.rs#thread_routes_share_the_agent_auth_middleware ; git diff cc7a3caa..HEAD -- crates/paladin-web/src/agent_controller.rs (only the require_authentication::<AgentApiState> turbofish; AgentApiState's own struct body untouched)

### 4. [24-12 D1] docs/src/user-guides/parley-and-chronicle.md teaches the approval gate, envelope-raised parleys, resume_with, partial answers, expiry, Chronicle history/replay/fork, graceful shutdown, the HTTP surface, and the paladin-notifications composition example; wired into SUMMARY.md after control-flow.md
expected: docs/src/user-guides/parley-and-chronicle.md teaches the approval gate, envelope-raised parleys, resume_with, partial answers, expiry, Chronicle history/replay/fork, graceful shutdown, the HTTP surface, and the paladin-notifications composition example; wired into SUMMARY.md after control-flow.md
result: pass
source: automated
coverage_id: 24-12/D1
requirement: HITL-01
covered_by: grep -c 'parley-and-chronicle' docs/src/SUMMARY.md (1 match) ; cargo test --workspace --doc (0 failed) ; mdbook build docs (0 broken links, after mdbook-mermaid install docs)

### 5. [24-12 D2] MIGRATION.md §9.2/§9.6 filled per D-29: Waypoint row resolved, ParleyPort recorded new, Phase 24 deliberate-zero note added, three thread endpoints listed in §9.6; sections 9.1/9.5/9.8 left untouched (plan 24-09's scope)
expected: MIGRATION.md §9.2/§9.6 filled per D-29: Waypoint row resolved, ParleyPort recorded new, Phase 24 deliberate-zero note added, three thread endpoints listed in §9.6; sections 9.1/9.5/9.8 left untouched (plan 24-09's scope)
result: pass
source: automated
coverage_id: 24-12/D2
requirement: HITL-01
covered_by: grep -c 'ParleyPort' MIGRATION.md (2 matches) && grep -c '/v1/threads/' MIGRATION.md (3 matches) ; git diff -U0 MIGRATION.md hunks confined to §9.2 (~lines 136-146) and §9.6 (~line 191+) plus the require_authentication row — no touch to §9.1/§9.5/§9.8

### 6. [24-12 D3] CHANGELOG.md [Unreleased] records HITL-01..05's user-visible changes and the v3->v4 fingerprint bump
expected: CHANGELOG.md [Unreleased] records HITL-01..05's user-visible changes and the v3->v4 fingerprint bump
result: pass
source: automated
coverage_id: 24-12/D3
requirement: HITL-01
covered_by: grep -c 'Unreleased' CHANGELOG.md (2 matches)

### 7. [24-12 D4] Traceability-matrix rows G-05, G-06, G-09, G-15, G-26 each name a concrete, tree-verified test function and file
expected: Traceability-matrix rows G-05, G-06, G-09, G-15, G-26 each name a concrete, tree-verified test function and file
result: pass
source: automated
coverage_id: 24-12/D4
requirement: HITL-01
covered_by: grep -rl 'fn <name>' for all 18 named test functions across .project/v0.10.0/08-traceability-matrix.md's five new anchor lists — every one found in the tree

### 8. [24-12 D5] Phase gate evidence recorded honestly: cargo test --workspace, cargo fmt --check, cargo clippy -D warnings, make security, cargo doc, MSRV 1.88, cargo semver-checks (all 11 published packages vs 0.9.0), and a manual credential-handling review, with the Postgres Tier-2 cases and the reduced-feature coverage measurement both named as local gaps in WINDOWS.md
expected: Phase gate evidence recorded honestly: cargo test --workspace, cargo fmt --check, cargo clippy -D warnings, make security, cargo doc, MSRV 1.88, cargo semver-checks (all 11 published packages vs 0.9.0), and a manual credential-handling review, with the Postgres Tier-2 cases and the reduced-feature coverage measurement both named as local gaps in WINDOWS.md
result: pass
source: automated
coverage_id: 24-12/D5
requirement: HITL-01
covered_by: cargo test --workspace --no-fail-fast --features web-server (0 failed across every test binary) ; cargo fmt --check ; cargo clippy --workspace --features web-server -- -D warnings ; make security (advisories ok, bans ok, licenses ok, sources ok) ; cargo doc --no-deps --workspace --features web-server (no new warnings; this plan touched no .rs files) ; cargo +1.88 check --workspace --all-features --all-targets --locked (0 errors) ; cargo semver-checks check-release --package <pkg> --default-features --baseline-version 0.9.0 for all 11 published packages (paladin-ai, paladin-ai-core, paladin-ports, paladin-battalion, paladin-herald, paladin-llm, paladin-memory, paladin-storage, paladin-notifications, paladin-content, paladin-web) — all pass clean

### 4. [24-13 D1] MIGRATION.md §9.6 resume status-code row lists 403 in ascending order, with a note naming require_admin and PLAT-06
expected: MIGRATION.md §9.6 resume status-code row lists 403 in ascending order, with a note naming require_admin and PLAT-06
result: pass
source: automated
coverage_id: 24-13/D1
requirement: HITL-05
covered_by: grep '^| `POST` | `/v1/threads/{id}/resume` |' MIGRATION.md | grep -c '`403`' -> 1 ; cargo test -p paladin-web --all-features --lib openapi::tests::openapi_thread_paths_document_every_status

### 5. [24-13 D2] CHANGELOG.md Unreleased HITL-05 bullet states the admin requirement, the 403, the any-role reads, and PLAT-06, with no new Security subsection
expected: CHANGELOG.md Unreleased HITL-05 bullet states the admin requirement, the 403, the any-role reads, and PLAT-06, with no new Security subsection
result: pass
source: automated
coverage_id: 24-13/D2
requirement: HITL-05
covered_by: sed -n '/^- \\\\*\\\\*Threads over HTTP (HITL-05)/,/^- \\\\|^## /p' CHANGELOG.md | grep -c '403' -> 1 ; git diff --stat CHANGELOG.md (confined to [Unreleased] section, no new ### Security)

### 6. [24-13 D3] mdBook user guide's HTTP-surface intro, route table, and Interim authorization posture blockquote describe the shipped admin-gated posture; secret-in-payload blockquote and SUMMARY.md untouched
expected: mdBook user guide's HTTP-surface intro, route table, and Interim authorization posture blockquote describe the shipped admin-gated posture; secret-in-payload blockquote and SUMMARY.md untouched
result: pass
source: automated
coverage_id: 24-13/D3
requirement: HITL-05
covered_by: mdbook build docs/ -> exit 0, 'No broken links found' ; ! grep -q 'no admin/writer scope distinction' docs/src/user-guides/parley-and-chronicle.md -> exit 0

### 7. [24-13 D4] openapi_thread_paths_document_every_status asserts 403 on the resume path; shipped require_admin gate and the three role tests remain unchanged
expected: openapi_thread_paths_document_every_status asserts 403 on the resume path; shipped require_admin gate and the three role tests remain unchanged
result: pass
source: automated
coverage_id: 24-13/D4
requirement: HITL-05
covered_by: cargo test -p paladin-web --all-features --lib thread_controller::tests::post_resume_with_non_admin_role_is_403 (+2 more) -> 3 passed ; cargo clippy -p paladin-web --all-targets --all-features -- -D warnings -> clean

## Summary

total: 63
passed: 63
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps

[none yet]
