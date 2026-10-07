---
phase: 42-mid-run-halt-sse-terminal-status
verified: 2026-10-07T02:35:00Z
status: human_needed
score: 4/4 roadmap success criteria verified (plus plan-level truths sampled against code and named tests)
behavior_unverified: 0
overrides_applied: 0
human_verification:
  - test: "Review the judgment-tier prohibition verdicts below (about 20 must_haves.prohibitions across plans 42-01..42-12, all carried as `status: unresolved`, `verification: null`)"
    expected: "Each MUST NOT either confirmed as not-happening (the evidence column below points at code and a named passing test for every one) or redirected. These are NON-AUTHORITATIVE LLM-judge verdicts: unverified-prohibition, human review recommended."
    why_human: "The prohibitions are descriptor-less (no `verification: test` tier, no enforcement descriptor), so by the ADR-550 D3/D4 soft-gate they cannot be closed by an automated verifier; they surface as flagged items, never a silent pass. No prohibition was found violated."
  - test: "Decide whether ROADMAP SC1/SC2 and REQUIREMENTS ALLOW-03 wording ('last checkpoint kept ... on both engine and agent-loop runs') should be amended or an override recorded for the agent loop"
    expected: "Agent-kind runs have no checkpoint by construction (CONTEXT D-08, WINDOWS.md row 64); the roadmap text was never amended to say so. Suggested override below."
    why_human: "A recorded, operator-accepted scope narrowing (D-08) versus literal roadmap text is a product decision, not something grep can settle."
---

# Phase 42: Mid-Run Halt & SSE Terminal Status Verification Report

**Phase Goal:** A run that would overspend mid-flight stops cleanly and resumably instead of running unchecked, and the SSE stream reports the run's real terminal status.
**Verified:** 2026-10-07
**Status:** human_needed (no failed truth, no blocker, no missing artifact; the only open items are flagged judgment-tier prohibitions and one wording decision)
**Re-verification:** No, initial verification

Approach: SUMMARY claims were not trusted. I read the engine, guard, worker, limits, derive, SSE, storage and web code directly, then ran the named tests myself (`CARGO_INCREMENTAL=0`).

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | In-flight run whose next draw would overspend halts cleanly (typed Treasurer error, `Halted`, last checkpoint kept) on both `WarEngine` and `PaladinExecutionService` runs | VERIFIED (with caveat C1, C2) | Engine: `crates/paladin-battalion/src/engine/superstep.rs` ~2255-2290 consults the `SpendGuard` once per boundary after the cancel signals, writes a `WaypointStatus::Halted` Waypoint and returns `RunOutcome::Halted { waypoint, cause: HaltCause::Spend(reason) }`. `TreasurerSpendGuard::check` (`src/application/services/treasurer/guard.rs`) reads balance via the shared `Treasurer::evaluate`, fails closed to `LedgerUnavailable`, memoises its first halt. Worker `map_outcome` records `Halted`, `error: None`, `final_waypoint_id`, typed `halt_reason`; the guard is attached per run at `worker.rs:1281`, and `run_api_wiring.rs` hands the same `Arc<Treasurer>` to the pool (`with_treasurer`). Agent loop: `TokenBudget::after_model` (`limits.rs:173-205`) ends the run with typed `StopReason::AllowanceHalted(AllowanceRefusal)`; worker `run_agent` (`worker.rs:1675`) records `Halted` plus reason and keeps partial output. Ran and passed: `engine_spend_halt_tracer`, `agent_kind_run_halts_on_the_derived_budget`, `guard_halting_on_the_first/second_boundary...`, `cancel_wins_over_spend_at_the_same_boundary`, `child_battalion_halt_on_spend_halts_the_parent`, all `treasurer::tests::guard_*`. |
| 2 | A halted run resumes and completes once allowance replenished / window resets, continuing from last checkpoint without re-executing | VERIFIED for the engine path (caveat C2 for the agent loop) | `halted_run_resumes_by_fork_after_window_reset` (ran, passed): fork refused `429` + `Retry-After` while exhausted; after the store clock passes the window end the same fork is admitted, new run id (no `Halted -> Queued` edge; `run.rs` transition list unchanged), forked run `Completed`, `fork_from` recorded, per-node counters `[1,1,1]` (n0 not re-run). `ledger_unavailable_halt_resumes_after_recovery` (ran, passed). `resume_continues_a_halted_thread_to_normal_completion` (ran, passed). |
| 3 | Treasurer derives per-run `TokenBudget` from remaining allowance; works alongside `TokenBudget`, `ModelCallLimit`, `ToolCallLimit`, Commissary without replacing them; guard test keeps `Treasurer` framework-only | VERIFIED | `src/application/services/treasurer/derive.rs`: `i128` floor of `min(ceiling - balance) * 1e6 / dearest` over all five price axes, saturating `u32`, free model -> none, zero -> `Refused`, missing price row -> `ModelUnpriced`. One `TokenBudget` takes the tighter of operator and derived figure, tie to Treasurer (`limits.rs`). Ran and passed: `a_tie_goes_to_the_treasurer`, `operator_figure_below_the_derived_one_wins...`, `operator_figure_above_the_derived_one_never_loosens_it`, `model_call_limit_still_binds_first_under_a_derived_figure`, `tool_call_limit_still_denies_under_a_derived_figure`, `derived_budget_composes_with_the_commissary_rationed_rag_context` (in the 146-test treasurer/limits run), `the_tightest_ceiling_binds_the_derived_budget`. `git diff` shows no Phase 42 change under `crates/paladin-llm`. `cargo test --test treasurer_vocabulary_guard`: 3 passed (clean tree, planted-violation control, real repo scan). |
| 4 | SSE `done` matches persisted status: `Cancelled` for caller cancel, `Halted` with Treasurer reason for a spend halt | VERIFIED | `RunFinishStatus::Cancelled` added; `events.rs:226` maps it to `done`/`cancelled`; `run_finish_status` (`engine/mod.rs:422-437`) maps `CancelRequested` to `Cancelled`, `Spend(reason)` to `Halted` plus reason. Worker writes `record_outcome` before `update_status` for halting transitions (`worker.rs:1408-1413`). Ran and passed: `every_halt_cause_maps_to_one_status_on_every_leg` (row, live, degraded, replay all equal, for spend, ledger-unavailable, same-instance cancel and cross-instance cancel), `halted_done_agrees_on_live_degraded_and_replay`, `same_instance_cancel_streams_done_cancelled`, `cross_instance_cancel_streams_done_cancelled`, `drain_streams_no_done_and_leaves_the_run_running`. |

**Score:** 4/4 roadmap truths verified; 0 present-but-behavior-unverified (every behavior-dependent truth has a passing named test).

### Caveats on literal wording (WARNING, not FAILED)

- **C1. "next draw would overspend".** The boundary check halts once `balance >= ceiling`; it cannot predict the cost of the next draw. Overshoot is bounded to one superstep (engine) or one model response (agent loop), and one whole call for a true stream. This is the ADR-0057 D-01/D-09 decision, recorded plainly in the ADR, WINDOWS.md row 65/66, CHANGELOG and docs; I found no text promising an allowance can never be overspent.
- **C2. "last checkpoint kept ... on both".** A worker-dispatched agent-kind run writes no Waypoint, so continue-from-checkpoint is met on the engine path and is not applicable by construction on the agent loop (CONTEXT D-08, WINDOWS.md row 64, `persist_agent_halt`). ROADMAP/REQUIREMENTS text was not amended to say so. Suggested override if the operator accepts it:

```yaml
overrides:
  - must_have: "last checkpoint kept ... on both an engine-driven and an agent-loop run"
    reason: "Agent-kind runs have no Waypoint by construction (D-08, WINDOWS.md row 64); halted agent-kind run resumes by a fresh POST /v1/runs. Engine path continues from its Halted Waypoint."
    accepted_by: "{name}"
    accepted_at: "{ISO timestamp}"
```

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/paladin-ports/src/output/spend_guard.rs` | `SpendGuard`, `SpendDecision`, `NeverHalts` port | VERIFIED | Substantive, doctests, wired into engine (`with_spend_guard`, passed to start/resume/fork and child battalion runs) |
| `src/application/services/treasurer/guard.rs`, `evaluate.rs`, `derive.rs` | per-run guard, shared evaluation, budget derivation | VERIFIED | Real ledger reads, fail-closed, sticky memo, notice claims; no `unwrap`/`expect`/`panic!` in non-test code |
| `crates/paladin-battalion/src/engine/{mod,superstep}.rs` | `HaltCause`, boundary check, `RunFinished.halt_reason` | VERIFIED | Cancel wins over spend; `run_finish_status` maps all causes |
| `crates/paladin-storage/migrations/{sqlite,postgres}/013_*.sql`, `014_*.sql` | `runs.halt_reason`; `treasury_notices.notice_kind` | VERIFIED (SQLite ran; Postgres legs not runnable here, CI authority) | Pre-existing migrations 001-012 untouched (empty diff) |
| `src/application/services/run/worker.rs` | guard attach, `PerRunCancelProbe`, halt/cancel mapping, agent-kind re-derive | VERIFIED | Wired via `build_run_api` |
| `crates/paladin-web/src/agent_controller.rs` | `allowance_halted` stop label, `halt_reason` on execute/stream/job, true-stream informational `halt_reason` (option-b) | VERIFIED | Unit tests for buffered fallback and true-stream crossing exist and pass in the workspace gate |
| `tests/treasurer_vocabulary_guard.rs` | ALLOW-05 vocabulary guard | VERIFIED | Passes; has positive control |
| `.planning/WINDOWS.md` rows 64-67, `MIGRATION.md`, `CHANGELOG.md`, `ADR-0057` | registers and decision record | VERIFIED | Present and consistent with the code |

### Key Link Verification

| From | To | Status | Details |
|------|----|--------|---------|
| `run_api_wiring::build_run_api` | `RunWorkerPool::with_treasurer` and `RunSubmissionService::with_treasurer` | WIRED | Same `Arc<Treasurer>`, built before the pool (G3) |
| `RunWorkerPool` -> engine | `engine.with_spend_guard(treasurer.spend_guard_with_emitter(..))` | WIRED | Only for runs with a recorded submitter; unattributed runs make no ledger read by design |
| Engine `Halted{cause}` -> worker `map_outcome` -> run row -> `GET /runs/{id}` | typed `halt_reason`, `final_waypoint_id` | WIRED | `engine_spend_halt_tracer` end to end over real SQLite and `run_router` |
| `RunFinished` -> `map_trace_event` / degraded `terminal_payload` / `replay_stream` | one `done` status | WIRED | `every_halt_cause_maps_to_one_status_on_every_leg` |
| Worker agent-kind dispatch -> `RunScope.derived_token_budget` -> `TokenBudget::after_model` | `AllowanceHalted` -> `Halted` row | WIRED | `agent_kind_run_halts_on_the_derived_budget` |
| Halt -> operator `allowance_halted` webhook | `notice_kind = 'halt'`, once per window | WIRED | `three_guards_halting_in_one_window_enqueue_one_allowance_halted_delivery`, `mid_run_warn_and_halt_notices_reach_the_operator_once` |
| `Halted` fork -> `POST /threads/{id}/fork` | admission re-check, `fork_from` | WIRED | `halted_run_resumes_by_fork_after_window_reset` |

### Data-Flow Trace (Level 4)

Balance reads in the guard come from the real `TreasuryLedgerPort` (SQLite in the tracers); `halt_reason` flows engine -> row -> `wire_json` -> HTTP/SSE/webhook, observed in assertions on served JSON. No static or hollow source found.

### Behavioral Spot-Checks (single named tests / filtered groups, run by me)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Engine halt, fork-resume, ledger-unavailable resume, SSE agreement | `cargo test -p paladin-ai --lib --features web-server -- halted_run_resumes_by_fork_after_window_reset ledger_unavailable_halt_resumes_after_recovery engine_spend_halt_tracer halted_done_agrees_on_live_degraded_and_replay` | 4 passed | PASS |
| Treasurer, config and stream-leg group | `... -- every_halt_cause_maps drain_streams_no_done agent_kind_run_halts mid_run_warn_and_halt_notices middleware::limits treasurer::` | 146 passed, 0 failed | PASS |
| Cancel `done` and limits composition | `... -- agent_kind_run_halts same_instance_cancel_streams cross_instance_cancel_streams mid_run_warn_and_halt middleware::limits::tests the_tightest` | 32 passed | PASS |
| Engine boundary semantics | `cargo test -p paladin-battalion --lib -- spend halt` | 23 passed | PASS |
| Vocabulary guard | `cargo test --test treasurer_vocabulary_guard` | 3 passed | PASS |
| Formatting | `cargo fmt --check` | exit 0 | PASS |

I did not re-run the full workspace suite; the orchestrator's gate (4,411 passed, two known sandbox-network cases in `deferred-items.md`) stands as supplied.

### Probe Execution

SKIPPED: no probe scripts declared by the phase plans.

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|--------------|-------------|--------|----------|
| ALLOW-03 | 42-01..42-05, 07-12 | Mid-run halt, `Halted`, checkpoint kept, resumable, engine and agent loop, ADR recorded | SATISFIED (caveats C1, C2) | SC1, SC2 above; ADR-0057 |
| ALLOW-05 | 42-01, 42-07, 42-08, 42-09, 42-11, 42-12 | Derived `TokenBudget`, composes with existing limits and Commissary, framework-only `Treasurer` guard | SATISFIED | SC3 above |
| PLAT-09 | 42-01, 42-03, 42-05, 42-06, 42-12 | SSE `done` matches persisted status | SATISFIED | SC4 above |

All three IDs appear in plan frontmatter and in REQUIREMENTS.md (still `Pending`, as instructed). No orphaned Phase 42 requirement.

### Anti-Patterns Found

None. Added lines in the phase diff contain no `TBD`, `FIXME`, `XXX`, `TODO`, `HACK`, `todo!` or `unimplemented!`; new production files have no `unwrap`/`expect`/`panic!`. Deferred items (all in `deferred-items.md`, none goal-blocking): `paladin-eval` `RunStatusValue` lacks `Cancelled`; the pre-existing load-sensitive `local_cancel_signals_token`; the two network-dependent `run_api_wiring` tests; Postgres/Redis legs are CI-only.

### Prohibition verdicts (NON-AUTHORITATIVE LLM-judge; unverified-prohibition, human review recommended)

| Plan | MUST NOT | Verdict | Evidence |
|------|----------|---------|----------|
| 01 | Promise an allowance can never be overspent | Not violated | Grep of docs/src/crates finds no such promise; ADR-0057 "Overshoot, stated exactly"; WINDOWS rows 65-66 |
| 01 | Write a `reserve` row / change 007 schema or settlement key | Not violated | guard/evaluate call no reserve/settle/release; no diff to migrations 001-012 |
| 02 | Role bypass of the boundary check | Not violated | Guard takes `RunAttribution` only |
| 02 | Key value in logs/events/halt payloads | Not violated | `fail_closed_message` and halt log carry scope kind, tenant, run id; `details_json` has no identity; `fail_closed_log_line_names_run_scope_tenant_and_error_only` passes |
| 03 | Spend halt as `Failed` / populate `error` | Not violated | `map_outcome` Spend arm: `Halted`, `error: None`; ledger test asserts `error` null |
| 03 | Expose other scopes' figures or identity in `halt_reason` | Not violated | `wire_json` = `details_json` + `reason` only |
| 04 | Continue past an unevaluable ceiling | Not violated | `Err` arm halts `LedgerUnavailable`; guard fail-closed tests pass |
| 04 | Resume by re-enqueue / `Halted -> Queued` | Not violated | Transition table unchanged; resume test asserts a new run id |
| 05 | Live/degraded/replay disagree; halt as `error` event | Not violated | `every_halt_cause_maps...`, `ledger_unavailable_done_is_done_not_error...` |
| 06 | `done` for a still-Running (drain) run; cancel reported as halted | Not violated | `drain_streams_no_done...`, `cancelled_run_finished_maps_to_done_cancelled...` |
| 07 | Replace/loosen other limits; add a second budget mechanism | Not violated | Single `after_model` comparison in `limits.rs`; composition tests pass |
| 08 | Run unpriced model for a ceilinged principal | Not violated | `ModelUnpriced` -> 422 `model_unpriced`; agent-route and `POST /runs` tests |
| 09 | Cap engine nodes via shared service; record agent halt as Completed/Failed | Not violated | Derived figure only on a scope that carries it; `facade_provisioner` engine-node test; `agent_kind_run_halts...` asserts `halted` |
| 11 | More than one operator halt notice per window | Not violated | Store-deduped `halt` rung; three-guards test passes |
| 12 | `Treasurer` as downstream term / reuse of fixture vocabulary | Not violated | `treasurer_vocabulary_guard` passes with positive control |

## Gaps Summary

No gaps. All four ROADMAP success criteria hold in code and under tests I ran myself, all three requirement IDs are accounted for, the design-gate option-b decision (true-stream informational `halt_reason`) is implemented and tested, and the registers (ADR-0057, MIGRATION, CHANGELOG, WINDOWS rows 64-67) match the tree. Status is `human_needed` solely because the descriptor-less prohibitions must be surfaced as flagged items under the soft-gate rule, and because the agent-loop checkpoint wording (C2) is a recorded scope narrowing the operator should either amend or accept via override. Postgres contract legs and Redis integration tests could not run in this sandbox; CI is their authority.

---

_Verified: 2026-10-07_
_Verifier: Claude (gsd-verifier)_
