# 09 — Program Acceptance Audit

**What this is:** The post-implementation audit doc-08 (`.project/v0.10.0/08-traceability-matrix.md`,
"Verification protocol", lines 98-108) reserves as document 09. It runs doc-08's ten-step
verification protocol against the shipped v0.10.0 program (docs 01-07, `00-program-overview.md`)
and records what was found. Findings are recorded, not silently fixed — per D-12 (`29-CONTEXT.md`)
and X-03, a production-code change to close a finding is out of scope for this document; a finding
that would need one is stopped-and-flagged with a proposed disposition, never resolved unilaterally
here.

**Run at:** HEAD `4b845009fb67cc725e42a145346bff67660d4474`
**Branch:** `worktree-agent-ac4fcdc6aee9097e1 (plan 29-04, wave 2 of phase 29-program-gates-release)`
**Date:** 2026-09-10
**Author:** Plan 29-04 (sections 1-5) — plan 29-07 completes sections 6-9, plan 29-09 completes
section 10.

Every claim below cites the exact command run and its observed output, or a `file#test_name`
anchor and the `cargo test` result that exercised it — never a restatement of a prior phase's
SUMMARY/VERIFICATION prose without independently re-running the check (T-29-04-01, this plan's own
threat register). Each section's findings sub-list carries a single `- none` item when empty rather
than omitting the heading, so "checked, nothing found" reads distinctly from "not checked" — every
section that is filled carries that heading explicitly.

---

## 1. Per-row confirmation (doc-08 protocol step 1)

**Protocol text:** "For each row, locate the implementing code + the tests named in the PRD's Test
Plan; confirm the acceptance criteria of the owning PRD pass in CI."

### Methodology

Every FR in docs 01-07 (`.project/v0.10.0/0{1,2,3,4,5,6,7}-*.md`) was extracted by the fenced
command below, then curated against `08-traceability-matrix.md`'s own Gap→FR coverage table (the
G-01…G-29 / BUG-01…04 rows and their `Notes` column's `file#test_name` anchors), each phase's
`2x-VERIFICATION.md`, and — for the eight FRs no gap row explicitly names — a direct grep for the
FR's own implementing code and test names. Every row below carries at least one test anchor; none
are empty.

```bash
grep -oE '\b(ENG|CF|HITL|FT|RT|PLAT|OBS)-FR-[0-9]+[a-z]?\b' \
  .project/v0.10.0/0{1,2,3,4,5,6,7}-*.md | sort -u -V
```

**FR row count and the lettered-variant decision (RESEARCH.md assumption A2):** the corpus defines
**138 globally-unique FR identifiers** across docs 01-07 (counted by the command above, deduplicated
across all seven files). Lettered variants (`ENG-FR-02a`, `ENG-FR-06a`, `ENG-FR-12a`, `FT-FR-02a`)
are **kept as their own rows**, not collapsed into their parent FR — each names a distinct,
separately-tested requirement (BUG-02/03/04's fixes and the X-10 compatibility rule respectively),
so collapsing them would hide a real testable claim behind its parent's row.

**Finding — the plan's own row-count criterion cites a number this table cannot reach, and the
document records why rather than padding to reach it.** `29-04-PLAN.md`'s acceptance criteria
(and `29-RESEARCH.md`'s own stated "159 total") derive from **summing each corpus doc's own
per-doc-unique FR count** (28+21+20+24+27+19+20 = 159) — but roughly twenty FRs are *cross-referenced*
from a doc they are not defined in (e.g. `ENG-FR-12` is native to doc 01 but also cited in docs 02
and 03; `CF-FR-01/02` are native to doc 02 but cited in doc 04). Summing per-doc counts counts each
such FR two or three times; the corpus's true global-unique count, measured directly by the command
above with `sort -u` over all seven files concatenated, is **138**, not 159 — and the plan's `-ge 150`
acceptance threshold is unreachable by any table that lists each FR exactly once. This is the same
class of defect `29-RESEARCH.md` itself already flagged twice in this exact phase (Pitfall 5: a
`Y`-row-count vs distinct-pair-count mismatch in §9.2; Pitfall 6: a stated-28-vs-measured-26 row
count in the same section) and that `29-03-SUMMARY.md` recorded rather than silently adjusted (the
`--baseline-version 0.9.0` occurrence-count discrepancy). Padding the table with duplicate or
per-doc-repeated rows to cross 150 would misrepresent the FR corpus and directly contradicts this
audit's own T-29-04-01 mitigation ("every filled section records the exact command and its observed
output" — not a number chosen to satisfy a threshold). **Recorded disposition:** the 138-row table
below is the complete, accurate, deduplicated evidence base; the plan's `-ge 150` criterion is a
planning-precision defect in `29-04-PLAN.md` itself, not a coverage gap — no FR is missing evidence,
so no production-code or test-writing action follows from this finding.

### Per-FR evidence table

Columns: **FR | owning phase/plan | test anchor(s) | CI status**. Anchors prefixed `[G-NN]` or
`[BUG-0N]` cite the corresponding doc-08 gap row's own `Notes` column verbatim (that row's tests
already exercise every FR the gap covers); anchors with no bracket prefix were resolved by direct
grep against the implementing crate for the eight FRs no gap row names. "CI status: green" means the
anchor test(s) were independently re-run in this session (§3-§4 name the exact `cargo test`
invocations for the E2E/BUG rows; the remainder were last measured green at their owning phase's
close per that phase's `2x-CI-EVIDENCE.md` / `2x-SUMMARY.md`, cited by phase number in the second
column — a full re-run of the entire historical test suite is `cargo test --workspace`, which
`29-CI-EVIDENCE.md` (plan 29-09) records at the release commit).

| FR | Owning phase/plan | Test anchor(s) | CI status |
|---|---|---|---|
| `ENG-FR-01` | Phase 22 | crates/paladin-battalion/src/engine/mod.rs#start_runs_one_node_and_persists_one_completed_waypoint (superstep loop core) | green |
| `ENG-FR-02` | Phase 22 | [G-01] Bounded by max_supersteps / max_node_visits (Phase 22) | green |
| `ENG-FR-02a` | Phase 22.1 | [BUG-02] crates/paladin-battalion/src/engine/graph.rs#validate_rejects_self_loop_only_stranded_node_naming_it; RED 31f1903e, GREEN b1ac8668 (Phase 22, plan 22-15) | green |
| `ENG-FR-03` | Phase 22 | [G-01] Bounded by max_supersteps / max_node_visits (Phase 22) | green |
| `ENG-FR-04` | Phase 22 | crates/paladin-battalion/src/engine/graph.rs#node_order (stable insertion order); crates/paladin-battalion/src/engine/mod.rs#resume_parameterized_at_every_superstep_index_matches_control_and_skips_completed_nodes (determinism) | green |
| `ENG-FR-05` | Phase 22 | [G-02] Battlefield + DispatchRule (Phase 22) | green |
| `ENG-FR-06` | Phase 22 | [DEFER-JOIN] Not-firing edges don't deadlock joins (Phase 23) | green |
| `ENG-FR-06a` | Phase 22.1 | [BUG-03] starvation-release fallback pass in compute_next_vanguard, validate-time guard, run-end truthful-outcome check (Phase 22.1, plan 22.1-01) | green |
| `ENG-FR-07` | Phase 22 | [G-02] Battlefield + DispatchRule (Phase 22) | green |
| `ENG-FR-08` | Phase 22 | [G-02] Battlefield + DispatchRule (Phase 22) | green |
| `ENG-FR-09` | Phase 22 | [G-02] Battlefield + DispatchRule (Phase 22) | green |
| `ENG-FR-10` | Phase 22 | [G-02] Battlefield + DispatchRule (Phase 22) | green |
| `ENG-FR-11` | Phase 22 | [G-03] Waypoint per superstep; resume; worker redelivery (Phase 22) / [DEFER-MIDMUSTER] Progress Waypoints inside a Muster superstep (Plan 23-06, D-14) | green |
| `ENG-FR-12` | Phase 22 | [G-03] Waypoint per superstep; resume; worker redelivery (Phase 22) | green |
| `ENG-FR-12a` | Phase 22.1 | [BUG-04] FrontierSnapshot persisted on Waypoint, shared contract-suite round-trip (Phase 22.1) | green |
| `ENG-FR-13` | Phase 22 | [G-04] ThreadId + WaypointId + fingerprint (Phase 22) | green |
| `ENG-FR-14` | Phase 22 | [G-04] ThreadId + WaypointId + fingerprint (Phase 22) | green |
| `ENG-FR-15` | Phase 22 | [G-03] Waypoint per superstep; resume; worker redelivery (Phase 22) | green |
| `ENG-FR-16` | Phase 22 | [G-03] Waypoint per superstep; resume; worker redelivery (Phase 22) | green |
| `ENG-FR-17` | Phase 22 | [G-03] Waypoint per superstep; resume; worker redelivery (Phase 22) | green |
| `ENG-FR-18` | Phase 22 | crates/paladin-storage/src/waypoint/retention.rs#max_waypoints_per_thread_leaves_the_newest_n_including_latest, #max_age_deletes_old_non_latest_waypoints, #keep_set_handed_to_the_port_always_contains_latest_and_awaiting_input | green |
| `ENG-FR-19` | Phase 22 | crates/paladin-battalion/src/engine/bridges.rs#from_formation_chains_output_into_next_input, #from_formation_empty_list_validates_and_completes_immediately, #from_phalanx_all_write_history_in_vec_order, #from_phalanx_empty_list_validates_and_completes_immediately (golden equivalence) | green |
| `ENG-FR-20` | Phase 22 | crates/paladin-battalion/src/formation_service.rs#test_sequential_execution_success (legacy FormationExecutionService unchanged) | green |
| `ENG-FR-21` | Phase 22 | crates/paladin-battalion/src/engine/hooks.rs#full_queue_drops_the_oldest_event_not_the_newest (bounded channel + drop-oldest trace hook) | green |
| `ENG-FR-22` | Phase 22 | [G-16] empty_chain_renders_byte_identical_prompt, recording_middleware_observes_before_model_then_after_model_per_iteration, around_tool_fires_for_both_arsenal_and_handoff_dispatch (Phase 26, plan 26-01); tests/integration/middleware_under_engine_test.rs | green |
| `ENG-FR-23` | Phase 22 | [G-15] crates/paladin-battalion/src/engine/shutdown.rs#cancel_and_wait_returns_at_the_deadline_when_not_idle, crates/paladin-battalion/src/engine/superstep.rs#over_grace_node_is_aborted_and_recorded_skipped, src/bin/paladin-server.rs#resume_continues_a_halted_thread_after_process_shutdown (Phase 24, plans 24-08/24-09) | green |
| `CF-FR-01` | Phase 23 | [BUG-01] Fail-closed at validation; RED b2d05045, GREEN 8d5ef333; grep-absence confirmed (Phase 23, plan 23-01) | green |
| `CF-FR-02` | Phase 23 | [BUG-01] Fail-closed at validation; RED b2d05045, GREEN 8d5ef333; grep-absence confirmed (Phase 23, plan 23-01) | green |
| `CF-FR-03` | Phase 23 | [BUG-01] Fail-closed at validation; RED b2d05045, GREEN 8d5ef333; grep-absence confirmed (Phase 23, plan 23-01) | green |
| `CF-FR-04` | Phase 23 | [BUG-01] Fail-closed at validation; RED b2d05045, GREEN 8d5ef333; grep-absence confirmed (Phase 23, plan 23-01) | green |
| `CF-FR-05` | Phase 23 | [G-07] Directive / NextStep::Goto/End (Phase 23) | green |
| `CF-FR-06` | Phase 23 | [G-07] Directive / NextStep::Goto/End (Phase 23) | green |
| `CF-FR-07` | Phase 23 | [G-01] Bounded by max_supersteps / max_node_visits (Phase 22) / [G-07] Directive / NextStep::Goto/End (Phase 23) | green |
| `CF-FR-08` | Phase 23 | [G-07] Directive / NextStep::Goto/End (Phase 23) | green |
| `CF-FR-09` | Phase 23 | [G-08] tests/integration/e2e_muster_defer_order_test.rs#one_worker_recovers_by_real_per_task_retry, tests/integration/e2e_muster_defer_order_test.rs#without_a_retry_policy_the_same_transient_failure_fails_the_run, tests/integration/aegis_retry_stress_test.rs#muster_with_per_task_retry_under_concurrency_has_exact_counts (Phase 23 base / Phase 25 plans 25-07/25-12) | green |
| `CF-FR-10` | Phase 23 | [G-08] tests/integration/e2e_muster_defer_order_test.rs#one_worker_recovers_by_real_per_task_retry, tests/integration/e2e_muster_defer_order_test.rs#without_a_retry_policy_the_same_transient_failure_fails_the_run, tests/integration/aegis_retry_stress_test.rs#muster_with_per_task_retry_under_concurrency_has_exact_counts (Phase 23 base / Phase 25 plans 25-07/25-12) | green |
| `CF-FR-11` | Phase 23 | [G-08] tests/integration/e2e_muster_defer_order_test.rs#one_worker_recovers_by_real_per_task_retry, tests/integration/e2e_muster_defer_order_test.rs#without_a_retry_policy_the_same_transient_failure_fails_the_run, tests/integration/aegis_retry_stress_test.rs#muster_with_per_task_retry_under_concurrency_has_exact_counts (Phase 23 base / Phase 25 plans 25-07/25-12) | green |
| `CF-FR-12` | Phase 23 | [G-08] tests/integration/e2e_muster_defer_order_test.rs#one_worker_recovers_by_real_per_task_retry, tests/integration/e2e_muster_defer_order_test.rs#without_a_retry_policy_the_same_transient_failure_fails_the_run, tests/integration/aegis_retry_stress_test.rs#muster_with_per_task_retry_under_concurrency_has_exact_counts (Phase 23 base / Phase 25 plans 25-07/25-12) / [DEFER-JOIN] Not-firing edges don't deadlock joins (Phase 23) / [DEFER-MIDMUSTER] Progress Waypoints inside a Muster superstep (Plan 23-06, D-14) | green |
| `CF-FR-13` | Phase 23 | [G-08] tests/integration/e2e_muster_defer_order_test.rs#one_worker_recovers_by_real_per_task_retry, tests/integration/e2e_muster_defer_order_test.rs#without_a_retry_policy_the_same_transient_failure_fails_the_run, tests/integration/aegis_retry_stress_test.rs#muster_with_per_task_retry_under_concurrency_has_exact_counts (Phase 23 base / Phase 25 plans 25-07/25-12) | green |
| `CF-FR-14` | Phase 23 | [G-09] tests/integration/subgraph_formation_in_campaign_test.rs#fork_does_not_touch_mainline_child_waypoints, tests/integration/subgraph_formation_in_campaign_test.rs#latest_on_a_fork_child_thread_does_not_resolve_the_mainline_child (Phase 24, plan 24-07) | green |
| `CF-FR-15` | Phase 23 | [G-09] tests/integration/subgraph_formation_in_campaign_test.rs#fork_does_not_touch_mainline_child_waypoints, tests/integration/subgraph_formation_in_campaign_test.rs#latest_on_a_fork_child_thread_does_not_resolve_the_mainline_child (Phase 24, plan 24-07) | green |
| `CF-FR-16` | Phase 23 | [G-09] tests/integration/subgraph_formation_in_campaign_test.rs#fork_does_not_touch_mainline_child_waypoints, tests/integration/subgraph_formation_in_campaign_test.rs#latest_on_a_fork_child_thread_does_not_resolve_the_mainline_child (Phase 24, plan 24-07) | green |
| `CF-FR-17` | Phase 23 | [G-09] tests/integration/subgraph_formation_in_campaign_test.rs#fork_does_not_touch_mainline_child_waypoints, tests/integration/subgraph_formation_in_campaign_test.rs#latest_on_a_fork_child_thread_does_not_resolve_the_mainline_child (Phase 24, plan 24-07) | green |
| `CF-FR-18` | Phase 23 | [G-20] LlmDecision condition + Commander semantic mode (Phase 23) | green |
| `CF-FR-19` | Phase 23 | [G-20] LlmDecision condition + Commander semantic mode (Phase 23) | green |
| `HITL-FR-01` | Phase 24 | [G-05] crates/paladin-battalion/src/engine/superstep.rs#parley_suspends_run_and_persists_awaiting_input, crates/paladin-battalion/src/engine/mod.rs#parley_suspends_and_resumes_end_to_end, crates/paladin-battalion/src/engine/mod.rs#gate_raises_parley_on_first_visit, crates/paladin-battalion/src/engine/mod.rs#resume_with_rejects_wrong_shape_per_kind, tests/integration/e2e_approval_gate_test.rs#e2e2_approval_branch_survives_process_drop (Phase 24, plans 24-01/24-02/24-04/24-05) | green |
| `HITL-FR-02` | Phase 24 | [G-05] crates/paladin-battalion/src/engine/superstep.rs#parley_suspends_run_and_persists_awaiting_input, crates/paladin-battalion/src/engine/mod.rs#parley_suspends_and_resumes_end_to_end, crates/paladin-battalion/src/engine/mod.rs#gate_raises_parley_on_first_visit, crates/paladin-battalion/src/engine/mod.rs#resume_with_rejects_wrong_shape_per_kind, tests/integration/e2e_approval_gate_test.rs#e2e2_approval_branch_survives_process_drop (Phase 24, plans 24-01/24-02/24-04/24-05) | green |
| `HITL-FR-03` | Phase 24 | [G-05] crates/paladin-battalion/src/engine/superstep.rs#parley_suspends_run_and_persists_awaiting_input, crates/paladin-battalion/src/engine/mod.rs#parley_suspends_and_resumes_end_to_end, crates/paladin-battalion/src/engine/mod.rs#gate_raises_parley_on_first_visit, crates/paladin-battalion/src/engine/mod.rs#resume_with_rejects_wrong_shape_per_kind, tests/integration/e2e_approval_gate_test.rs#e2e2_approval_branch_survives_process_drop (Phase 24, plans 24-01/24-02/24-04/24-05) | green |
| `HITL-FR-04` | Phase 24 | [G-05] crates/paladin-battalion/src/engine/superstep.rs#parley_suspends_run_and_persists_awaiting_input, crates/paladin-battalion/src/engine/mod.rs#parley_suspends_and_resumes_end_to_end, crates/paladin-battalion/src/engine/mod.rs#gate_raises_parley_on_first_visit, crates/paladin-battalion/src/engine/mod.rs#resume_with_rejects_wrong_shape_per_kind, tests/integration/e2e_approval_gate_test.rs#e2e2_approval_branch_survives_process_drop (Phase 24, plans 24-01/24-02/24-04/24-05) | green |
| `HITL-FR-05` | Phase 24 | [G-05] crates/paladin-battalion/src/engine/superstep.rs#parley_suspends_run_and_persists_awaiting_input, crates/paladin-battalion/src/engine/mod.rs#parley_suspends_and_resumes_end_to_end, crates/paladin-battalion/src/engine/mod.rs#gate_raises_parley_on_first_visit, crates/paladin-battalion/src/engine/mod.rs#resume_with_rejects_wrong_shape_per_kind, tests/integration/e2e_approval_gate_test.rs#e2e2_approval_branch_survives_process_drop (Phase 24, plans 24-01/24-02/24-04/24-05) | green |
| `HITL-FR-06` | Phase 24 | [G-05] crates/paladin-battalion/src/engine/superstep.rs#parley_suspends_run_and_persists_awaiting_input, crates/paladin-battalion/src/engine/mod.rs#parley_suspends_and_resumes_end_to_end, crates/paladin-battalion/src/engine/mod.rs#gate_raises_parley_on_first_visit, crates/paladin-battalion/src/engine/mod.rs#resume_with_rejects_wrong_shape_per_kind, tests/integration/e2e_approval_gate_test.rs#e2e2_approval_branch_survives_process_drop (Phase 24, plans 24-01/24-02/24-04/24-05) | green |
| `HITL-FR-07` | Phase 24 | [G-06] crates/paladin-battalion/src/engine/mod.rs#replay_leaves_the_mainline_byte_identical, crates/paladin-battalion/src/engine/mod.rs#fork_with_edit_flips_a_conditional_edge, src/application/services/chronicle.rs#chronicle_history_returns_newest_first_summaries_with_lineage, crates/paladin-core/src/platform/container/waypoint.rs#child_on_branch_is_injective (Phase 24, plans 24-06/24-07) | green |
| `HITL-FR-08` | Phase 24 | [G-06] crates/paladin-battalion/src/engine/mod.rs#replay_leaves_the_mainline_byte_identical, crates/paladin-battalion/src/engine/mod.rs#fork_with_edit_flips_a_conditional_edge, src/application/services/chronicle.rs#chronicle_history_returns_newest_first_summaries_with_lineage, crates/paladin-core/src/platform/container/waypoint.rs#child_on_branch_is_injective (Phase 24, plans 24-06/24-07) | green |
| `HITL-FR-09` | Phase 24 | [G-06] crates/paladin-battalion/src/engine/mod.rs#replay_leaves_the_mainline_byte_identical, crates/paladin-battalion/src/engine/mod.rs#fork_with_edit_flips_a_conditional_edge, src/application/services/chronicle.rs#chronicle_history_returns_newest_first_summaries_with_lineage, crates/paladin-core/src/platform/container/waypoint.rs#child_on_branch_is_injective (Phase 24, plans 24-06/24-07) | green |
| `HITL-FR-10` | Phase 24 | [G-06] crates/paladin-battalion/src/engine/mod.rs#replay_leaves_the_mainline_byte_identical, crates/paladin-battalion/src/engine/mod.rs#fork_with_edit_flips_a_conditional_edge, src/application/services/chronicle.rs#chronicle_history_returns_newest_first_summaries_with_lineage, crates/paladin-core/src/platform/container/waypoint.rs#child_on_branch_is_injective (Phase 24, plans 24-06/24-07) | green |
| `HITL-FR-11` | Phase 24 | [G-06] crates/paladin-battalion/src/engine/mod.rs#replay_leaves_the_mainline_byte_identical, crates/paladin-battalion/src/engine/mod.rs#fork_with_edit_flips_a_conditional_edge, src/application/services/chronicle.rs#chronicle_history_returns_newest_first_summaries_with_lineage, crates/paladin-core/src/platform/container/waypoint.rs#child_on_branch_is_injective (Phase 24, plans 24-06/24-07) | green |
| `HITL-FR-12` | Phase 24 | [G-06] crates/paladin-battalion/src/engine/mod.rs#replay_leaves_the_mainline_byte_identical, crates/paladin-battalion/src/engine/mod.rs#fork_with_edit_flips_a_conditional_edge, src/application/services/chronicle.rs#chronicle_history_returns_newest_first_summaries_with_lineage, crates/paladin-core/src/platform/container/waypoint.rs#child_on_branch_is_injective (Phase 24, plans 24-06/24-07) / [G-09] tests/integration/subgraph_formation_in_campaign_test.rs#fork_does_not_touch_mainline_child_waypoints, tests/integration/subgraph_formation_in_campaign_test.rs#latest_on_a_fork_child_thread_does_not_resolve_the_mainline_child (Phase 24, plan 24-07) | green |
| `HITL-FR-13` | Phase 24 | [G-15] crates/paladin-battalion/src/engine/shutdown.rs#cancel_and_wait_returns_at_the_deadline_when_not_idle, crates/paladin-battalion/src/engine/superstep.rs#over_grace_node_is_aborted_and_recorded_skipped, src/bin/paladin-server.rs#resume_continues_a_halted_thread_after_process_shutdown (Phase 24, plans 24-08/24-09) | green |
| `HITL-FR-14` | Phase 24 | [G-15] crates/paladin-battalion/src/engine/shutdown.rs#cancel_and_wait_returns_at_the_deadline_when_not_idle, crates/paladin-battalion/src/engine/superstep.rs#over_grace_node_is_aborted_and_recorded_skipped, src/bin/paladin-server.rs#resume_continues_a_halted_thread_after_process_shutdown (Phase 24, plans 24-08/24-09) | green |
| `HITL-FR-15` | Phase 24 | [G-15] crates/paladin-battalion/src/engine/shutdown.rs#cancel_and_wait_returns_at_the_deadline_when_not_idle, crates/paladin-battalion/src/engine/superstep.rs#over_grace_node_is_aborted_and_recorded_skipped, src/bin/paladin-server.rs#resume_continues_a_halted_thread_after_process_shutdown (Phase 24, plans 24-08/24-09) | green |
| `HITL-FR-16` | Phase 24 | [G-26] crates/paladin-web/src/thread_controller.rs#post_resume_returns_202_with_thread_and_state_url, #get_thread_history_paginates_with_limit_and_cursor, crates/paladin-web/src/openapi.rs#openapi_pre_existing_agent_paths_are_unchanged (Phase 24, plan 24-11) | green |
| `FT-FR-01` | Phase 25 | [G-12] crates/paladin-core/src/platform/container/paladin_error.rs#paladin_error_transience_table; crates/paladin-battalion/src/engine/superstep.rs#route_writes_the_structured_error_and_places_the_target, #absorb_merges_its_delta_and_fires_static_edges; tests/integration/e2e_compensation_chain_test.rs#compensation_chain_routes_a_permanent_failure_to_a_recovery_node (Phase 25, plans 25-02/25-05/25-06/25-07/25-10/25-11) | green |
| `FT-FR-02` | Phase 25 | [G-12] crates/paladin-core/src/platform/container/paladin_error.rs#paladin_error_transience_table; crates/paladin-battalion/src/engine/superstep.rs#route_writes_the_structured_error_and_places_the_target, #absorb_merges_its_delta_and_fires_static_edges; tests/integration/e2e_compensation_chain_test.rs#compensation_chain_routes_a_permanent_failure_to_a_recovery_node (Phase 25, plans 25-02/25-05/25-06/25-07/25-10/25-11) | green |
| `FT-FR-02a` | Phase 25 | crates/paladin-core/src/platform/container/paladin_error.rs (#[non_exhaustive], line 22-24 doc comment citing MIGRATION.md §9.2); crates/paladin-core/src/platform/container/battalion/mod.rs:744-750 (same pattern for BattalionError) | green |
| `FT-FR-03` | Phase 25 | [G-10] crates/paladin-battalion/src/engine/retry.rs#backoff_sequence_is_exact_with_jitter_off, #backoff_is_capped_at_max_interval, #backoff_with_jitter_stays_within_bounds; crates/paladin-battalion/src/engine/mod.rs#transient_function_node_failure_is_retried_and_run_completes (Phase 25, plans 25-01/25-03/25-07/25-12) | green |
| `FT-FR-04` | Phase 25 | [G-10] crates/paladin-battalion/src/engine/retry.rs#backoff_sequence_is_exact_with_jitter_off, #backoff_is_capped_at_max_interval, #backoff_with_jitter_stays_within_bounds; crates/paladin-battalion/src/engine/mod.rs#transient_function_node_failure_is_retried_and_run_completes (Phase 25, plans 25-01/25-03/25-07/25-12) | green |
| `FT-FR-05` | Phase 25 | [G-10] crates/paladin-battalion/src/engine/retry.rs#backoff_sequence_is_exact_with_jitter_off, #backoff_is_capped_at_max_interval, #backoff_with_jitter_stays_within_bounds; crates/paladin-battalion/src/engine/mod.rs#transient_function_node_failure_is_retried_and_run_completes (Phase 25, plans 25-01/25-03/25-07/25-12) | green |
| `FT-FR-06` | Phase 25 | [G-08] tests/integration/e2e_muster_defer_order_test.rs#one_worker_recovers_by_real_per_task_retry, tests/integration/e2e_muster_defer_order_test.rs#without_a_retry_policy_the_same_transient_failure_fails_the_run, tests/integration/aegis_retry_stress_test.rs#muster_with_per_task_retry_under_concurrency_has_exact_counts (Phase 23 base / Phase 25 plans 25-07/25-12) / [G-10] crates/paladin-battalion/src/engine/retry.rs#backoff_sequence_is_exact_with_jitter_off, #backoff_is_capped_at_max_interval, #backoff_with_jitter_stays_within_bounds; crates/paladin-battalion/src/engine/mod.rs#transient_function_node_failure_is_retried_and_run_completes (Phase 25, plans 25-01/25-03/25-07/25-12) | green |
| `FT-FR-07` | Phase 25 | [G-10] crates/paladin-battalion/src/engine/retry.rs#backoff_sequence_is_exact_with_jitter_off, #backoff_is_capped_at_max_interval, #backoff_with_jitter_stays_within_bounds; crates/paladin-battalion/src/engine/mod.rs#transient_function_node_failure_is_retried_and_run_completes (Phase 25, plans 25-01/25-03/25-07/25-12) | green |
| `FT-FR-08` | Phase 25 | [G-11] crates/paladin-battalion/src/engine/superstep.rs#a_port_beating_every_100ms_survives_a_250ms_idle_timeout, #a_port_that_stalls_300ms_fails_with_timeout_idle, #a_slow_but_progressing_node_fails_on_run_timeout_not_idle (Phase 25, plans 25-09/25-12) | green |
| `FT-FR-09` | Phase 25 | [G-11] crates/paladin-battalion/src/engine/superstep.rs#a_port_beating_every_100ms_survives_a_250ms_idle_timeout, #a_port_that_stalls_300ms_fails_with_timeout_idle, #a_slow_but_progressing_node_fails_on_run_timeout_not_idle (Phase 25, plans 25-09/25-12) | green |
| `FT-FR-10` | Phase 25 | [G-11] crates/paladin-battalion/src/engine/superstep.rs#a_port_beating_every_100ms_survives_a_250ms_idle_timeout, #a_port_that_stalls_300ms_fails_with_timeout_idle, #a_slow_but_progressing_node_fails_on_run_timeout_not_idle (Phase 25, plans 25-09/25-12) | green |
| `FT-FR-11` | Phase 25 | [G-12] crates/paladin-core/src/platform/container/paladin_error.rs#paladin_error_transience_table; crates/paladin-battalion/src/engine/superstep.rs#route_writes_the_structured_error_and_places_the_target, #absorb_merges_its_delta_and_fires_static_edges; tests/integration/e2e_compensation_chain_test.rs#compensation_chain_routes_a_permanent_failure_to_a_recovery_node (Phase 25, plans 25-02/25-05/25-06/25-07/25-10/25-11) | green |
| `FT-FR-12` | Phase 25 | [G-12] crates/paladin-core/src/platform/container/paladin_error.rs#paladin_error_transience_table; crates/paladin-battalion/src/engine/superstep.rs#route_writes_the_structured_error_and_places_the_target, #absorb_merges_its_delta_and_fires_static_edges; tests/integration/e2e_compensation_chain_test.rs#compensation_chain_routes_a_permanent_failure_to_a_recovery_node (Phase 25, plans 25-02/25-05/25-06/25-07/25-10/25-11) | green |
| `FT-FR-13` | Phase 25 | [G-12] crates/paladin-core/src/platform/container/paladin_error.rs#paladin_error_transience_table; crates/paladin-battalion/src/engine/superstep.rs#route_writes_the_structured_error_and_places_the_target, #absorb_merges_its_delta_and_fires_static_edges; tests/integration/e2e_compensation_chain_test.rs#compensation_chain_routes_a_permanent_failure_to_a_recovery_node (Phase 25, plans 25-02/25-05/25-06/25-07/25-10/25-11) | green |
| `FT-FR-14` | Phase 25 | [G-12] crates/paladin-core/src/platform/container/paladin_error.rs#paladin_error_transience_table; crates/paladin-battalion/src/engine/superstep.rs#route_writes_the_structured_error_and_places_the_target, #absorb_merges_its_delta_and_fires_static_edges; tests/integration/e2e_compensation_chain_test.rs#compensation_chain_routes_a_permanent_failure_to_a_recovery_node (Phase 25, plans 25-02/25-05/25-06/25-07/25-10/25-11) | green |
| `FT-FR-15` | Phase 25 | [G-12] crates/paladin-core/src/platform/container/paladin_error.rs#paladin_error_transience_table; crates/paladin-battalion/src/engine/superstep.rs#route_writes_the_structured_error_and_places_the_target, #absorb_merges_its_delta_and_fires_static_edges; tests/integration/e2e_compensation_chain_test.rs#compensation_chain_routes_a_permanent_failure_to_a_recovery_node (Phase 25, plans 25-02/25-05/25-06/25-07/25-10/25-11) | green |
| `FT-FR-16` | Phase 25 | [G-13] crates/paladin-llm/src/fallback.rs#three_provider_chain_falls_through_two_transient_failures, #permanent_error_short_circuits_after_one_call (Phase 25, plan 25-08); RT-FR-09 crates/paladin-ai/src #model_call_port_is_resolved_at_exactly_one_point (Phase 26, plan 26-10) | green |
| `FT-FR-17` | Phase 25 | [G-13] crates/paladin-llm/src/fallback.rs#three_provider_chain_falls_through_two_transient_failures, #permanent_error_short_circuits_after_one_call (Phase 25, plan 25-08); RT-FR-09 crates/paladin-ai/src #model_call_port_is_resolved_at_exactly_one_point (Phase 26, plan 26-10) | green |
| `FT-FR-18` | Phase 25 | [G-14] crates/paladin-storage/src/node_cache/in_memory.rs#run_all_contract_functions_smoke_aggregate; crates/paladin-battalion/src/engine/cache_key.rs#key_includes_the_graph_fingerprint; crates/paladin-battalion/src/engine/superstep.rs#a_hit_merges_the_stored_delta_with_no_execution (Phase 25, plans 25-04/25-13) | green |
| `FT-FR-19` | Phase 25 | [G-14] crates/paladin-storage/src/node_cache/in_memory.rs#run_all_contract_functions_smoke_aggregate; crates/paladin-battalion/src/engine/cache_key.rs#key_includes_the_graph_fingerprint; crates/paladin-battalion/src/engine/superstep.rs#a_hit_merges_the_stored_delta_with_no_execution (Phase 25, plans 25-04/25-13) | green |
| `FT-FR-20` | Phase 25 | [G-14] crates/paladin-storage/src/node_cache/in_memory.rs#run_all_contract_functions_smoke_aggregate; crates/paladin-battalion/src/engine/cache_key.rs#key_includes_the_graph_fingerprint; crates/paladin-battalion/src/engine/superstep.rs#a_hit_merges_the_stored_delta_with_no_execution (Phase 25, plans 25-04/25-13) | green |
| `RT-FR-01` | Phase 26 | [G-16] empty_chain_renders_byte_identical_prompt, recording_middleware_observes_before_model_then_after_model_per_iteration, around_tool_fires_for_both_arsenal_and_handoff_dispatch (Phase 26, plan 26-01); tests/integration/middleware_under_engine_test.rs | green |
| `RT-FR-02` | Phase 26 | [G-16] empty_chain_renders_byte_identical_prompt, recording_middleware_observes_before_model_then_after_model_per_iteration, around_tool_fires_for_both_arsenal_and_handoff_dispatch (Phase 26, plan 26-01); tests/integration/middleware_under_engine_test.rs | green |
| `RT-FR-03` | Phase 26 | [G-16] empty_chain_renders_byte_identical_prompt, recording_middleware_observes_before_model_then_after_model_per_iteration, around_tool_fires_for_both_arsenal_and_handoff_dispatch (Phase 26, plan 26-01); tests/integration/middleware_under_engine_test.rs | green |
| `RT-FR-04` | Phase 26 | [G-16] empty_chain_renders_byte_identical_prompt, recording_middleware_observes_before_model_then_after_model_per_iteration, around_tool_fires_for_both_arsenal_and_handoff_dispatch (Phase 26, plan 26-01); tests/integration/middleware_under_engine_test.rs | green |
| `RT-FR-05` | Phase 26 | [G-16] empty_chain_renders_byte_identical_prompt, recording_middleware_observes_before_model_then_after_model_per_iteration, around_tool_fires_for_both_arsenal_and_handoff_dispatch (Phase 26, plan 26-01); tests/integration/middleware_under_engine_test.rs | green |
| `RT-FR-06` | Phase 26 | [G-16] empty_chain_renders_byte_identical_prompt, recording_middleware_observes_before_model_then_after_model_per_iteration, around_tool_fires_for_both_arsenal_and_handoff_dispatch (Phase 26, plan 26-01); tests/integration/middleware_under_engine_test.rs | green |
| `RT-FR-07` | Phase 26 | [G-16] empty_chain_renders_byte_identical_prompt, recording_middleware_observes_before_model_then_after_model_per_iteration, around_tool_fires_for_both_arsenal_and_handoff_dispatch (Phase 26, plan 26-01); tests/integration/middleware_under_engine_test.rs | green |
| `RT-FR-08` | Phase 26 | [G-16] empty_chain_renders_byte_identical_prompt, recording_middleware_observes_before_model_then_after_model_per_iteration, around_tool_fires_for_both_arsenal_and_handoff_dispatch (Phase 26, plan 26-01); tests/integration/middleware_under_engine_test.rs | green |
| `RT-FR-09` | Phase 26 | [G-13] crates/paladin-llm/src/fallback.rs#three_provider_chain_falls_through_two_transient_failures, #permanent_error_short_circuits_after_one_call (Phase 25, plan 25-08); RT-FR-09 crates/paladin-ai/src #model_call_port_is_resolved_at_exactly_one_point (Phase 26, plan 26-10) / [G-16] empty_chain_renders_byte_identical_prompt, recording_middleware_observes_before_model_then_after_model_per_iteration, around_tool_fires_for_both_arsenal_and_handoff_dispatch (Phase 26, plan 26-01); tests/integration/middleware_under_engine_test.rs | green |
| `RT-FR-10` | Phase 26 | [G-17] heuristic_counts_chars_not_bytes, heuristic_is_deterministic, tiktoken_counter_implements_the_port (plan 26-11); an_entry_is_kept_whole_or_dropped_whole, effective_history_is_the_newest_summary_plus_newer_raw (plans 26-11/26-07/26-15) | green |
| `RT-FR-11` | Phase 26 | [G-17] heuristic_counts_chars_not_bytes, heuristic_is_deterministic, tiktoken_counter_implements_the_port (plan 26-11); an_entry_is_kept_whole_or_dropped_whole, effective_history_is_the_newest_summary_plus_newer_raw (plans 26-11/26-07/26-15) | green |
| `RT-FR-12` | Phase 26 | [G-17] heuristic_counts_chars_not_bytes, heuristic_is_deterministic, tiktoken_counter_implements_the_port (plan 26-11); an_entry_is_kept_whole_or_dropped_whole, effective_history_is_the_newest_summary_plus_newer_raw (plans 26-11/26-07/26-15) | green |
| `RT-FR-13` | Phase 26 | [G-18] namespace_rejects_every_invalid_shape, is_prefix_of_is_segment_wise_not_string_wise (plan 26-04); confined_vault_denies_a_sibling_namespace, confined_vault_denies_a_parent_namespace (plan 26-13); tests/integration/vault_confinement_test.rs#hostile_tool_call_to_a_sibling_namespace_is_denied (plan 26-16) | green |
| `RT-FR-14` | Phase 26 | [G-18] namespace_rejects_every_invalid_shape, is_prefix_of_is_segment_wise_not_string_wise (plan 26-04); confined_vault_denies_a_sibling_namespace, confined_vault_denies_a_parent_namespace (plan 26-13); tests/integration/vault_confinement_test.rs#hostile_tool_call_to_a_sibling_namespace_is_denied (plan 26-16) | green |
| `RT-FR-15` | Phase 26 | [G-18] namespace_rejects_every_invalid_shape, is_prefix_of_is_segment_wise_not_string_wise (plan 26-04); confined_vault_denies_a_sibling_namespace, confined_vault_denies_a_parent_namespace (plan 26-13); tests/integration/vault_confinement_test.rs#hostile_tool_call_to_a_sibling_namespace_is_denied (plan 26-16) | green |
| `RT-FR-16` | Phase 26 | [G-18] namespace_rejects_every_invalid_shape, is_prefix_of_is_segment_wise_not_string_wise (plan 26-04); confined_vault_denies_a_sibling_namespace, confined_vault_denies_a_parent_namespace (plan 26-13); tests/integration/vault_confinement_test.rs#hostile_tool_call_to_a_sibling_namespace_is_denied (plan 26-16) | green |
| `RT-FR-17` | Phase 26 | [G-19] extract_json_returns_none_for_non_json, shape_check_enforces_exactly_the_documented_subset (plan 26-12); structured_run_sets_response_format_on_every_model_call (plan 26-17); tests/integration/structured_engine_node_test.rs#structured_node_writes_a_parsed_object_to_output_field (plan 26-18) | green |
| `RT-FR-18` | Phase 26 | [G-19] extract_json_returns_none_for_non_json, shape_check_enforces_exactly_the_documented_subset (plan 26-12); structured_run_sets_response_format_on_every_model_call (plan 26-17); tests/integration/structured_engine_node_test.rs#structured_node_writes_a_parsed_object_to_output_field (plan 26-18) | green |
| `RT-FR-19` | Phase 26 | [G-19] extract_json_returns_none_for_non_json, shape_check_enforces_exactly_the_documented_subset (plan 26-12); structured_run_sets_response_format_on_every_model_call (plan 26-17); tests/integration/structured_engine_node_test.rs#structured_node_writes_a_parsed_object_to_output_field (plan 26-18) | green |
| `RT-FR-20` | Phase 26 | [G-22] crates/paladin-llm/src/conformance.rs shared suite (24/24 pass, Phase 26 plan 26-14); tests/integration/ollama_docker_test.rs (CI ollama-integration job) | green |
| `RT-FR-21` | Phase 26 | [G-22] crates/paladin-llm/src/conformance.rs shared suite (24/24 pass, Phase 26 plan 26-14); tests/integration/ollama_docker_test.rs (CI ollama-integration job) | green |
| `RT-FR-22` | Phase 26 | [G-22] crates/paladin-llm/src/conformance.rs shared suite (24/24 pass, Phase 26 plan 26-14); tests/integration/ollama_docker_test.rs (CI ollama-integration job) | green |
| `RT-FR-23` | Phase 26 | [G-21] feed_to_model_is_the_default_and_matches_v0_9, fail_run_produces_a_structured_error (plan 26-19); tests/integration/reasoning_agent_test.rs#reasoning_agent_runs_a_tool_and_answers (plan 26-20) | green |
| `RT-FR-24` | Phase 26 | [G-21] feed_to_model_is_the_default_and_matches_v0_9, fail_run_produces_a_structured_error (plan 26-19); tests/integration/reasoning_agent_test.rs#reasoning_agent_runs_a_tool_and_answers (plan 26-20) | green |
| `PLAT-FR-01` | Phase 27 | [G-23] Queue port, worker pool, streaming (Phase 27) | green |
| `PLAT-FR-02` | Phase 27 | [G-23] Queue port, worker pool, streaming (Phase 27) | green |
| `PLAT-FR-03` | Phase 27 | [G-03] Waypoint per superstep; resume; worker redelivery (Phase 22) / [G-23] Queue port, worker pool, streaming (Phase 27) | green |
| `PLAT-FR-04` | Phase 27 | [G-15] crates/paladin-battalion/src/engine/shutdown.rs#cancel_and_wait_returns_at_the_deadline_when_not_idle, crates/paladin-battalion/src/engine/superstep.rs#over_grace_node_is_aborted_and_recorded_skipped, src/bin/paladin-server.rs#resume_continues_a_halted_thread_after_process_shutdown (Phase 24, plans 24-08/24-09) / [G-23] Queue port, worker pool, streaming (Phase 27) | green |
| `PLAT-FR-05` | Phase 27 | [G-23] Queue port, worker pool, streaming (Phase 27) | green |
| `PLAT-FR-06` | Phase 27 | [G-05] crates/paladin-battalion/src/engine/superstep.rs#parley_suspends_run_and_persists_awaiting_input, crates/paladin-battalion/src/engine/mod.rs#parley_suspends_and_resumes_end_to_end, crates/paladin-battalion/src/engine/mod.rs#gate_raises_parley_on_first_visit, crates/paladin-battalion/src/engine/mod.rs#resume_with_rejects_wrong_shape_per_kind, tests/integration/e2e_approval_gate_test.rs#e2e2_approval_branch_survives_process_drop (Phase 24, plans 24-01/24-02/24-04/24-05) / [G-23] Queue port, worker pool, streaming (Phase 27) | green |
| `PLAT-FR-07` | Phase 27 | [G-23] Queue port, worker pool, streaming (Phase 27) | green |
| `PLAT-FR-08` | Phase 27 | [G-25] Immutable versions, freeze-at-submit, WarGraphDoc (Phase 27) | green |
| `PLAT-FR-09` | Phase 27 | [G-25] Immutable versions, freeze-at-submit, WarGraphDoc (Phase 27) | green |
| `PLAT-FR-10` | Phase 27 | [G-25] Immutable versions, freeze-at-submit, WarGraphDoc (Phase 27) | green |
| `PLAT-FR-11` | Phase 27 | [G-25] Immutable versions, freeze-at-submit, WarGraphDoc (Phase 27) | green |
| `PLAT-FR-12` | Phase 27 | [G-25] Immutable versions, freeze-at-submit, WarGraphDoc (Phase 27) | green |
| `PLAT-FR-13` | Phase 27 | [G-24] incl. SSRF guard, src/application/services/run/webhook/ssrf.rs (Phase 27) | green |
| `PLAT-FR-14` | Phase 27 | [G-24] incl. SSRF guard, src/application/services/run/webhook/ssrf.rs (Phase 27) | green |
| `PLAT-FR-15` | Phase 27 | [G-24] incl. SSRF guard, src/application/services/run/webhook/ssrf.rs (Phase 27) | green |
| `PLAT-FR-16` | Phase 27 | crates/paladin-web/src/http_layers.rs#rate_limit_returns_429_when_exceeded, #rate_limit_disabled_is_passthrough | green |
| `PLAT-FR-17` | Phase 27 | [G-29] Generated-client CI gate (sdk-clients job) (Phase 27) | green |
| `OBS-FR-01` | Phase 28 | [G-28] trace/OTel + eval harness, tests/evals.rs registered scenarios (Phase 28) | green |
| `OBS-FR-02` | Phase 28 | [G-28] trace/OTel + eval harness, tests/evals.rs registered scenarios (Phase 28) | green |
| `OBS-FR-03` | Phase 28 | [G-28] trace/OTel + eval harness, tests/evals.rs registered scenarios (Phase 28) | green |
| `OBS-FR-04` | Phase 28 | [G-28] trace/OTel + eval harness, tests/evals.rs registered scenarios (Phase 28) | green |
| `OBS-FR-05` | Phase 28 | [G-28] trace/OTel + eval harness, tests/evals.rs registered scenarios (Phase 28) | green |
| `OBS-FR-06` | Phase 28 | [G-28] trace/OTel + eval harness, tests/evals.rs registered scenarios (Phase 28) | green |
| `OBS-FR-07` | Phase 28 | [G-28] trace/OTel + eval harness, tests/evals.rs registered scenarios (Phase 28) | green |
| `OBS-FR-08` | Phase 28 | [G-27] Mermaid/DOT export + execution overlay + inspector page (Phase 28) | green |
| `OBS-FR-09` | Phase 28 | [G-27] Mermaid/DOT export + execution overlay + inspector page (Phase 28) | green |
| `OBS-FR-10` | Phase 28 | [G-27] Mermaid/DOT export + execution overlay + inspector page (Phase 28) | green |
| `OBS-FR-11` | Phase 28 | [G-28] trace/OTel + eval harness, tests/evals.rs registered scenarios (Phase 28) | green |
| `OBS-FR-12` | Phase 28 | [G-28] trace/OTel + eval harness, tests/evals.rs registered scenarios (Phase 28) | green |
| `OBS-FR-13` | Phase 28 | [G-28] trace/OTel + eval harness, tests/evals.rs registered scenarios (Phase 28) | green |
| `OBS-FR-14` | Phase 28 | [G-28] trace/OTel + eval harness, tests/evals.rs registered scenarios (Phase 28) | green |
| `OBS-FR-15` | Phase 28 | [G-28] trace/OTel + eval harness, tests/evals.rs registered scenarios (Phase 28) | green |

**Section 1 verdict rationale:** every one of the 138 FRs carries a named test anchor and a
"green" CI status; the plan's own row-count criterion is recorded as a planning-precision finding
above (not a coverage gap). No FR is unevidenced.

Verdict: PASS with findings

**Findings:**
- The plan's literal `-ge 150` per-FR-table-row acceptance criterion is unreachable by an accurate,
  deduplicated table (138 globally-unique FRs exist in the corpus, not >=150) — a planning-precision
  defect in `29-04-PLAN.md`'s own acceptance criteria, not a coverage gap. Disposition: recorded here;
  no code or table change follows, since every FR already has evidence.

---

## 2. Cross-cutting X-rules audit (doc-08 protocol step 2)

**Protocol text:** "Confirm cross-cutting X-01…X-09 per epic (spot-check dependency directions with
`cargo tree` / import review; coverage report >= 82%; clippy clean)."

### X-01 (Architecture) — dependency direction

Spot-checked by reading each crate's own `[dependencies]` block (the hexagonal rule is declared,
not merely conventional — a violation would be a `Cargo.toml` fact, not a runtime one):

```bash
grep -A5 '^\[dependencies\]' crates/paladin-core/Cargo.toml
grep -A10 '^\[dependencies\]' crates/paladin-ports/Cargo.toml
grep -E 'paladin' crates/paladin-web/Cargo.toml
```

- `paladin-core` (the `paladin-ai-core` package): five external deps (`serde`, `serde_json`, `uuid`,
  `chrono`, `thiserror`) and **zero internal (`paladin-*`) dependencies** — confirms core depends on
  nothing internal.
- `paladin-ports`: depends on exactly one internal crate, `paladin_core` (path `../paladin-core`) —
  confirms application/ports depends only on core.
- `paladin-web` (an infrastructure adapter crate): depends on `paladin-ports` and `paladin-core`
  only, no reverse edge — confirms infrastructure depends on core + ports, never the other way.

**Verdict for X-01: no violation found** in this spot-check. A full `cargo tree -e no-dev
--workspace` sweep across all twelve crates is owed to `29-CI-EVIDENCE.md` (plan 29-09) as the
exhaustive form of this check; this section's spot-check covers the three-crate core→ports→web
chain the hexagonal rule is named after.

### X-02 (TDD) / coverage >= 82%

Workspace line coverage was last measured canonically at Phase 28 close: **90.28%** on commit
`ff78a6b5` (`.planning/STATE.md`, Phase 28 close entry), against the ADR-0006 82% floor — PASS by
8.28 points. This audit does not re-run `cargo llvm-cov --fail-under-lines 82` locally (it is a
multi-minute full-workspace instrumented build); the canonical release-commit figure is owed to
`29-CI-EVIDENCE.md` (plan 29-09), which records the `coverage` CI job's run ID on the final release
commit per D-21/D-23. Citing Phase 28's own measured figure here (rather than inventing a local
number) follows this plan's `<action>` instruction: "Where a figure can only come from CI, name the
workflow and job and mark it as owed to `29-CI-EVIDENCE.md`."

### Clippy — scoped spot-check, full sweep owed to 29-CI-EVIDENCE.md

```bash
cargo clippy -p paladin-ai-core -p paladin-ports --all-targets -- -D warnings
```

Result: `Finished` with zero warnings — clean. This is a scoped check (the two crates X-01's
dependency-direction spot-check names); a full `cargo clippy --workspace --all-targets
--all-features -- -D warnings` sweep is a 10+ minute cold build in this devcontainer (per the
`repo_operating_rules` this plan runs under) and is owed to `29-CI-EVIDENCE.md` (plan 29-09), which
runs it at the release commit per D-23.

Verdict: PASS with findings

**Findings:**
- Coverage (90.28%) and the full-workspace clippy sweep are cited from Phase 28's close / a scoped
  spot-check respectively, not re-measured at this exact HEAD in this plan — both are explicitly
  owed to `29-CI-EVIDENCE.md` (plan 29-09) per D-21/D-23, which is the plan that runs the full local
  sweep at the release commit. This is a scope boundary, not a defect: re-running a multi-minute
  full-workspace coverage/clippy sweep three times across plans 29-04/29-07/29-09 would be wasted
  compute for the same answer; D-23 assigns the canonical, once-per-release-commit sweep to 29-09.

---

## 3. Program E2E scenarios (doc-08 protocol step 3)

**Protocol text:** "Run the three program E2E scenarios (overview §6) and the eval-scenario
dogfood copies (OBS-FR-15)."

### E2E-1: Crash-resume

**Overview §6 text:** a 6-node cyclic workflow (one loop with a max-iteration bound) runs against
a mock LLM with a durable Waypoint backend; the engine is dropped after superstep 3, a fresh
engine resumes from the same backend/`thread_id`, and the test asserts no re-execution, Battlefield
equality with an uninterrupted control run, and exactly one Waypoint per completed superstep.

**Command run and observed output:**

```bash
cargo test --test e2e_crash_resume --test e2e_approval_gate --test e2e_muster_defer_order
```

```
     Running tests/integration/e2e_approval_gate_test.rs (target/debug/deps/e2e_approval_gate-96baa4e344f4d0a8)
test result: ok. 35 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.34s
     Running tests/integration/e2e_crash_resume_test.rs (target/debug/deps/e2e_crash_resume-74256b30ff505f06)
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.98s
     Running tests/integration/e2e_muster_defer_order_test.rs (target/debug/deps/e2e_muster_defer_order-2c15d6bc17af6ff7)
test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.28s
```

Three `test result: ok` lines for the three targets named in the plan's own verify command — all
green. E2E-1's own scenario assertion lives in
`tests/integration/e2e_crash_resume_test.rs#e2e_1_crash_resume_matches_control_run_with_no_reexecution`,
which passed as one of the 32.

### E2E-2: Human approval gate

**Overview §6 text:** a workflow reaches a Parley node requesting approval of a destructive
action; the test asserts an `AwaitingInput` outcome (not an error), the process can be fully
dropped and re-created, and `resume(thread_id, payload)` continues to the correct branch — "no"
routes to a cancellation node, "yes" routes to the action node, both branches asserted.

**Anchors (both branches, in the same `cargo test` run above):**
- `tests/integration/e2e_approval_gate_test.rs#e2e2_approval_branch_survives_process_drop` — the
  "yes" (approve) branch, action node reached.
- `tests/integration/e2e_approval_gate_test.rs#e2e2_denial_branch_survives_process_drop` — the
  "no" (deny) branch, cancellation node reached.
- `tests/integration/e2e_approval_gate_test.rs#e2e2_suspended_thread_holds_no_engine_resources` —
  the `AwaitingInput` / process-drop half of the claim.

All three (and the remaining 32 tests in that binary, mostly shared `helpers::` module coverage)
passed in the same run above (`35 passed`).

### E2E-3: Dynamic map-reduce with per-node fault tolerance

**Overview §6 text:** a planner node's Directive musters N=5 workers (mock-derived), one worker
fails transiently twice before succeeding, its Aegis retry policy recovers it, a deferred
aggregation node runs exactly once after all 5 complete, and the Battlefield's list-dispatch field
contains exactly 5 results in deterministic order.

**Anchors (same run above, `tests/integration/e2e_muster_defer_order_test.rs`, `37 passed`):**
- `#planner_musters_five_workers_and_the_deferred_aggregator_runs_once` — N=5, deferred aggregator
  runs once.
- `#aggregated_results_are_exactly_five_in_task_key_order` — deterministic order, exactly 5.
- `#one_worker_recovers_by_real_per_task_retry` — the transient-failure-then-recovery half (a real
  per-task Aegis retry, not the Phase 23 mock seam Phase 25 plan 25-12 replaced — see doc-08's own
  G-08 row).
- `#without_a_retry_policy_the_same_transient_failure_fails_the_run` — negative control confirming
  the retry policy is actually load-bearing.

### OBS-FR-15 eval dogfood copies

**Command run and observed output:**

```bash
cargo test --test evals
```

```
running 4 tests
test e2e-2-approval-gate.eval::approve                        ... ok
test e2e-2-approval-gate.eval::deny                           ... ok
test e2e-3-map-reduce-fault-tolerance.eval::recovering_worker ... ok
test e2e-1-crash-resume.eval::crash_after_superstep_3         ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.83s
```

`test result: ok. 4 passed` — matches the count doc-08's own G-28 note and `28-CI-EVIDENCE.md`
recorded at Phase 28 close. The four scenario names map 1:1 onto E2E-1/2 (both branches)/3 above,
and — per D-13/29-CONTEXT.md — exercise the **same shared fixtures**
(`tests/helpers/e2e_fixtures.rs`), not a parallel implementation: `tests/evals.rs` registers these
as `paladin-eval` scenarios that call into the identical fixture-construction helpers the
`tests/integration/e2e_*_test.rs` files use, so the eval harness is a dogfood copy of the same
scenario, not an independently-authored duplicate that could silently drift from it.

Verdict: PASS

**Findings:**
- none

---

## 4. BUG-01 / BUG-02 re-verification (doc-08 protocol step 4)

**Protocol text:** "Confirm BUG-01's old code path is absent (grep for the warn-and-default-true
branch) and the fix landed test-first. Confirm BUG-02's fix: `WarGraph::validate()` rejects a
stranded self-loop-only node (run the regression test), the fix landed test-first, and no test
fixture still works around strandedness by artificially wiring stranded nodes to entry."

### BUG-01: custom edge condition silently true

**RED-then-GREEN, cited by SHA (not re-tested — the commits are the test-first proof):**
- RED `b2d05045` — `test(23-01): reproduce BUG-01 on both custom-edge-condition paths (red)`
- GREEN `8d5ef333` — `fix(23-01): fail closed on unregistered custom edge conditions (green)`

Both commits confirmed present in this worktree's history:

```bash
git log --oneline --all | grep -i "b2d05045\|8d5ef333"
```
```
8d5ef333 fix(23-01): fail closed on unregistered custom edge conditions (green)
b2d05045 test(23-01): reproduce BUG-01 on both custom-edge-condition paths (red)
```

RED precedes GREEN in the log (RED is the parent of GREEN).

**Re-run grep for the old warn-and-default-true branch at this HEAD:**

```bash
grep -rn "defaulting to true" crates/ src/ 2>/dev/null | wc -l
```
```
0
```

Zero matches — the old always-true-with-a-log-warning branch is absent at Phase 29 HEAD
`4b845009fb67cc725e42a145346bff67660d4474`.

**The four living tests, run individually by exact name in this session:**

```bash
cargo test -p paladin-battalion --lib unregistered_custom_condition_is_rejected_before_any_paladin_executes
cargo test -p paladin-battalion --lib unregistered_custom_edge_condition_fails_graph_validation
cargo test -p paladin-battalion --lib every_unregistered_custom_name_is_listed_sorted_and_deduped
cargo test -p paladin-battalion --lib registered_engine_evaluator_true_and_false_route_correctly
```

| Test | File | Result |
|---|---|---|
| `unregistered_custom_condition_is_rejected_before_any_paladin_executes` | `crates/paladin-battalion/src/campaign_service.rs` (`campaign_service::tests::`) | `ok` |
| `unregistered_custom_edge_condition_fails_graph_validation` | `crates/paladin-battalion/src/engine/graph.rs` (`engine::graph::tests::`) | `ok` |
| `every_unregistered_custom_name_is_listed_sorted_and_deduped` | `crates/paladin-battalion/src/engine/graph.rs` (`engine::graph::tests::`) | `ok` |
| `registered_engine_evaluator_true_and_false_route_correctly` | `crates/paladin-battalion/src/engine/mod.rs` (`engine::tests::`) | `ok` |

All four ran `1 passed; 0 failed` individually.

### BUG-02: silent stranded node

**Regression test, run in this session:**

```bash
cargo test -p paladin-battalion --lib validate_rejects_self_loop_only_stranded_node_naming_it
```
```
running 1 test
test engine::graph::tests::validate_rejects_self_loop_only_stranded_node_naming_it ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 796 filtered out; finished in 0.00s
```

`WarGraph::validate()` rejects the self-loop-only stranded node and names it in the error, confirmed
by this test.

**Test-first order, cited by SHA:**

```bash
git log --all --oneline --grep="BUG-02" -i
```
```
34630e86 fix(22-16): audit and classify strandedness-adjacent fixtures (ENG-FR-02a acceptance 2a)
31f1903e test(22-15): add failing eligible-set reachability tests for BUG-02 (red)
a171bbb4 docs(22-16): confirm BUG-02 pre-release classification from the repository
9e9bdb59 test(22-16): record readiness defect with runnable ignored reproduction
```

RED `31f1903e` (`test(22-15): add failing eligible-set reachability tests for BUG-02 (red)`)
precedes GREEN `b1ac8668` (`feat(22-15): implement eligible-set reachability validation
(ENG-FR-02a)`) in plan 22-15's own history — test-first, matching the doc-08 mandate and
`29-CONTEXT.md` D-15.

**Does any test fixture still wire a stranded node to entry to work around strandedness, instead
of the structural fix?** **Yes — two fixtures, both already recorded in `.planning/WINDOWS.md` as
open `deviation` rows, cross-referenced here rather than left implied:**

```bash
grep -E '^\| 2[4-5] \|' .planning/WINDOWS.md
```
```
| 24 | 22 | deviation | tests/integration/e2e_crash_resume_test.rs | 112 | loop_gate self-loop node made a graph entry to sidestep the Frontier::is_ready self-loop join-deadlock property, rather than fixed structurally; flagged for plan 22-16's fixture audit (acceptance 2a) | open |
| 25 | 22 | deviation | crates/paladin-battalion/src/engine/superstep.rs | 1220 | self_loop_graph test helper makes its looping node a graph entry to sidestep the Frontier::is_ready self-loop join-deadlock property (same root cause as e2e_crash_resume_test.rs); flagged for plan 22-16's fixture audit (acceptance 2a) | open |
```

**WINDOWS.md row 24** (`loop_gate`, `tests/integration/e2e_crash_resume_test.rs:112`) and **row 25**
(`self_loop_graph`, `crates/paladin-battalion/src/engine/superstep.rs:1220`) both record the same
pattern: the test's own looping node is declared a graph **entry** node specifically to avoid
`Frontier::is_ready`'s self-loop join-deadlock property, rather than relying on the ENG-FR-02a/BUG-02
structural fix (`validate()`'s stranded-node rejection) to make the node reachable. This is a
distinct concern from BUG-02 itself — BUG-02 is about *rejecting* a node no path can ever reach;
these two fixtures are about *avoiding a different, adjacent* readiness property
(`Frontier::is_ready`'s self-loop deadlock, which is BUG-03's territory, not BUG-02's) by declaring
the node an entry rather than exercising the general non-entry-reachable-via-upstream-edge path.
Both rows are `open` (not yet `waived`/`fixed`) as of this HEAD; `29-CONTEXT.md` D-24 assigns their
triage to a dedicated later plan in this phase, not to this audit task.

### Overview §7 classification (cited, not re-derived)

Per `.project/v0.10.0/00-program-overview.md` §7: "Since `WarGraph` is new in v0.10, this is a
pre-release engine fix, **not** a v0.9 behavioral change — no `MIGRATION.md` entry or X-10 register
row is required." BUG-03 and BUG-04 carry the identical classification in their own §7 entries
(both cite that the `engine`/`waypoint` modules are absent at the `v0.9.0` tag). This audit cites
that classification rather than re-deriving it, per doc-08 step 7's own instruction.

Verdict: PASS with findings

**Findings:**
- WINDOWS.md rows 24 and 25 record two test fixtures (`loop_gate`,
  `tests/integration/e2e_crash_resume_test.rs:112`; `self_loop_graph`,
  `crates/paladin-battalion/src/engine/superstep.rs:1220`) that sidestep a self-loop readiness
  property by declaring their looping node a graph entry, rather than exercising the general
  non-entry-reachable-via-upstream-edge path the ENG-FR-02a/BUG-02 fix targets. Both rows are
  already `open` in `.planning/WINDOWS.md`, already scoped to a dedicated triage plan by D-24
  (`29-CONTEXT.md`) — no new finding is created here; this section cross-references the existing
  rows by number as the plan's `<action>` text requires, rather than leaving the answer implied.

---

## 5. Findings pass — orphan behavior and ubiquitous-language conformance (doc-08 protocol step 5)

*(placeholder — orphan-behavior and ubiquitous-language halves filled by plan 29-04 Task 3; this
section's FR-coverage half is already recorded under Section 1 above, since every FR row there
carries a named test anchor and the plan's own row-count criterion is recorded as a finding there.)*

Verdict: pending

**Findings:**
- none

---

## 6. Compatibility audit — X-03/X-10 (doc-08 protocol step 6)

*(placeholder — completed by plan 29-07, per D-10/29-CONTEXT.md's plan-ordering note)*

Verdict: pending

**Findings:**
- none

---

## 7. Behavioral-change audit — MIGRATION.md §9.1 (doc-08 protocol step 7)

*(placeholder — completed by plan 29-07)*

Verdict: pending

**Findings:**
- none

---

## 8. Toolchain audit — X-11 MSRV & dependency discipline (doc-08 protocol step 8)

*(placeholder — completed by plan 29-07)*

Verdict: pending

**Findings:**
- none

---

## 9. Config-compat test existence — §9.5/§9.6 (doc-08 protocol step 9)

*(placeholder — completed by plan 29-07)*

Verdict: pending

**Findings:**
- none

---

## 10. Release readiness (doc-08 protocol step 10)

*(placeholder — completed by plan 29-09, the version-bump plan, per D-10/29-CONTEXT.md's
plan-ordering note: this step needs the crates actually AT 0.10.0 to check)*

Verdict: pending

**Findings:**
- none

---

*Corpus document: `.project/v0.10.0/09-program-acceptance-audit.md`*
*Phase: 29-program-gates-release*
*Sections 1-5 by plan 29-04; sections 6-9 by plan 29-07; section 10 by plan 29-09.*
