---
phase: 42-mid-run-halt-sse-terminal-status
plan: 07
subsystem: treasurer
tags: [allowance, derived-budget, token-budget, agent-loop, stop-reason, commissary, admission-port, tests, docs]

requires:
  - phase: 42-mid-run-halt-sse-terminal-status
    provides: ADR-0057 group (e) and the option-b gate decision (plan 42-01); Treasurer::evaluate, HaltReason and AllowanceRefusal Eq (plan 42-02); the halt wire shapes the new stop reason reports through (plans 42-03 to 42-06)
  - phase: 41-admission-time-allowance-enforcement
    provides: Treasurer, AllowancePolicy and ceiling order, AllowanceAdmissionPort, Admission, AllowanceRefusal
  - phase: 38-cost-and-pricing
    provides: PriceRow, PriceTable and cost_of_call defaults the dearest-axis derivation mirrors
provides:
  - StopReason::AllowanceHalted(AllowanceRefusal) (is_limit true, is_successful false)
  - DerivedTokenBudget { max_tokens, halt_figures }, RunScope.derived_token_budget and with_derived_token_budget, Admission::with_derived_budget and derived_budget
  - AllowanceAdmissionPort::admit_for_model (defaulted) and AdmissionError::ModelUnpriced
  - Treasurer::with_pricing, Treasurer::derive_budget, the overriding admit_for_model over the shared admit_inner, and the crate-private dearest_price_per_million and derive_max_tokens
  - ModelCallContext::derived_token_budget and the service plumbing from RunScope to the per-run context
  - One Treasurer-aware TokenBudget (tightest wins, a tie goes to the Treasurer) proven alongside ModelCallLimit, ToolCallLimit and the Commissary
  - MIGRATION 9.2 rows, the CHANGELOG entry and the refreshed public-API baseline
affects: [42-08, 42-09, 42-10, 42-11, 42-12]

tech-stack:
  added: []
  patterns:
    - "One function per rule, again: admit and admit_for_model share the private admit_inner, and derive_budget and the admission derive through the same derive_from over the same Treasurer::evaluate"
    - "Per-run scratch for a per-run figure: the derived budget rides RunScope to ModelCallContext and is read by the one shared middleware, never stored on it (limits.rs D-03)"
    - "Pessimistic integer derivation: dearest of five axes, i128 floor division, saturating u32; the derived count billed at the dearest axis never exceeds the remaining nanos"

key-files:
  created:
    - src/application/services/treasurer/derive.rs
  modified:
    - crates/paladin-core/src/platform/container/execution_result.rs
    - crates/paladin-core/src/platform/container/allowance.rs
    - crates/paladin-core/src/platform/container/run_scope.rs
    - crates/paladin-ports/src/input/allowance_admission_port.rs
    - src/application/services/treasurer/mod.rs
    - src/application/services/treasurer/tests.rs
    - src/application/services/paladin/middleware/context.rs
    - src/application/services/paladin/middleware/limits.rs
    - src/application/services/paladin/paladin_execution_service.rs
    - MIGRATION.md
    - CHANGELOG.md
    - .project/current-exports.txt

key-decisions:
  - "Admission::is_empty still answers only whether notices were won; a derived budget does not make an admission non-empty, so confirm and abandon callers keep their existing meaning"
  - "An exhausted ceiling is refused by derive_budget with the exhausted figures before any pricing lookup, so an unpriced model never masks an exhausted allowance"
  - "A price table whose currency differs from the policy currency is a Backend error naming both codes, never converted (D-00h)"
  - "The binding ceiling is the least headroom, ties to the first in policy order; the halt figures report that ceiling at balance equal to ceiling (A5), while a zero-figure refusal carries the real balance"
  - "execute_bounded and execute_internal each gained a parameter and carry #[allow(clippy::too_many_arguments)] (eight parameters) rather than a new bundling struct"
  - "The derived figure and crossing test stay public (DerivedTokenBudget fields, strict > on total tokens) so plan 42-08 can implement the option-b true-stream crossing report from Admission::derived_budget"

patterns-established:
  - "Tightest-wins match in TokenBudget::after_model: (operator, derived) pairs resolve to one limit and an optional allowance figure, so the stop reason names who bound"

requirements-completed: [ALLOW-05, ALLOW-03]

coverage:
  - id: D1
    description: "StopReason::AllowanceHalted is a limit and not successful, round-trips through serde with its figures, and the is_successful body does not name it"
    requirement: "ALLOW-05"
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/execution_result.rs#tests::allowance_halted_is_a_limit_and_not_successful, #allowance_halted_round_trips_through_serde (cargo test -p paladin-ai-core --lib: 695 passed)"
        status: pass
    human_judgment: false
  - id: D2
    description: "DerivedTokenBudget, the RunScope carrier and the Admission field exist with doc tests; a RunScope without the field still deserializes and omits it when None"
    requirement: "ALLOW-05"
    verification:
      - kind: unit
        ref: "allowance.rs#tests::derived_budget_is_carried_by_an_admission_and_does_not_make_it_non_empty, run_scope.rs#tests::run_scope_derived_token_budget_default_none_omitted_and_round_trips; cargo test -p paladin-ai-core --doc DerivedTokenBudget and with_derived_token_budget: 1 passed each"
        status: pass
    human_judgment: false
  - id: D3
    description: "Edge (ALLOW-05, precision): derive_max_tokens uses integer floor division in i128, saturates to u32::MAX, treats a negative remainder as 0, returns None for a non-positive price, and cost_of_call at the derived count billed at the dearest axis never exceeds the remaining nanos across a table of divisible and non-divisible pairs"
    requirement: "ALLOW-05"
    verification:
      - kind: unit
        ref: "src/application/services/treasurer/derive.rs#unit_tests::derive_max_tokens_table, #dearest_takes_the_largest_of_the_five_axes_with_cost_of_call_defaults, #cost_at_the_derived_count_never_exceeds_the_remaining_nanos; grep of the file for f32 and f64 prints 0"
        status: pass
    human_judgment: false
  - id: D4
    description: "Edge (ALLOW-05, boundary): a derived figure of 0 is refused with the binding ceiling's real figures, 1 is admitted, a free model gets no budget, an exhausted ceiling is refused with its own figures, and the tightest ceiling binds with a tie going to the first in policy order"
    requirement: "ALLOW-05"
    verification:
      - kind: unit
        ref: "src/application/services/treasurer/tests.rs#a_derived_figure_of_one_is_admitted_and_zero_is_refused_with_the_real_figures, #a_free_model_gets_no_derived_budget, #an_exhausted_ceiling_is_refused_by_admit_for_model_with_the_exhausted_figures, #the_tightest_ceiling_binds_the_derived_budget, #a_tie_between_ceilings_goes_to_the_first_in_policy_order (cargo test -p paladin-ai --lib application::services::treasurer: 70 passed)"
        status: pass
    human_judgment: false
  - id: D5
    description: "Edge (ALLOW-05, idempotency): deriving twice for the same subject and model over an unchanged ledger and clock yields equal budgets, and derivation writes nothing to the ledger"
    requirement: "ALLOW-05"
    verification:
      - kind: unit
        ref: "treasurer/tests.rs#deriving_twice_over_an_unchanged_ledger_and_clock_yields_equal_budgets, #derive_budget_is_idempotent_and_writes_nothing (real InMemoryTreasuryLedger spend view unchanged)"
        status: pass
    human_judgment: false
  - id: D6
    description: "Edge (ALLOW-05, concurrency): two agent calls by one principal admitted at the same instant each derive from the same pre-admission balance, so together they may spend up to twice the remaining allowance plus one response each -- the admission race accepted by D-01 and 41 D-05, not reproducible deterministically in a test"
    requirement: "ALLOW-05"
    verification:
      - kind: backstop
        ref: "ADR-0057 D-01; the accepted race, with its WINDOWS.md row owned by plan 42-12"
        status: pass
    human_judgment: false
  - id: D7
    description: "admit_for_model: no ceiling is admitted with no budget and zero ledger reads (even for an unpriced model); a ceiling plus an unpriced model, or no price table, is ModelUnpriced before any notice claim; a foreign-currency table is a Backend error naming both codes; the plain admit never derives; the trait default delegates to admit"
    requirement: "ALLOW-05"
    verification:
      - kind: unit
        ref: "treasurer/tests.rs#admit_for_model_without_a_ceiling_derives_nothing_and_reads_no_ledger, #an_unpriced_model_under_a_ceiling_is_refused_before_any_notice_claim, #no_price_table_at_all_refuses_a_ceilinged_principal_as_unpriced, #a_price_table_in_another_currency_is_a_backend_error_naming_both_codes, #the_plain_admit_never_derives_a_budget; paladin-ports allowance_admission_port#tests::the_default_admit_for_model_delegates_to_admit_with_no_budget; cargo test -p paladin-ports --doc admit_for_model: 1 passed"
        status: pass
    human_judgment: false
  - id: D8
    description: "One TokenBudget enforces the tighter of the operator and derived figures: a derived win or tie ends the run with AllowanceHalted keeping the crossing response and the notice, an operator win keeps TokenBudget, a derived figure is never loosened by a larger operator figure, a count at the figure continues and one more stops, and no figure with the operator off never cuts"
    requirement: "ALLOW-05"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/middleware/limits.rs#tests::derived_budget_alone_ends_the_run_with_allowance_halted, #cumulative_at_the_derived_figure_continues_and_one_more_stops, #operator_figure_below_the_derived_one_wins_and_keeps_token_budget, #a_tie_goes_to_the_treasurer, #operator_figure_above_the_derived_one_never_loosens_it, #no_derived_figure_and_a_disabled_operator_budget_never_cuts (seven of the new tests were red before the middleware change)"
        status: pass
    human_judgment: false
  - id: D9
    description: "Through a real service with a scripted model: the run halts on the crossing response, continues at exactly the figure and stops one response later, a run without a figure is not cut, and concurrent runs through one service keep their own figure"
    requirement: "ALLOW-05"
    verification:
      - kind: integration
        ref: "limits.rs#tests::a_run_with_a_derived_figure_halts_on_the_crossing_response, #a_run_continues_at_exactly_the_figure_and_stops_one_response_later, #a_run_without_a_derived_figure_is_not_cut, #concurrent_runs_keep_their_own_derived_figure"
        status: pass
    human_judgment: false
  - id: D10
    description: "ALLOW-05 works alongside, replaces none: a run carrying a derived figure still stops on ModelCallLimit and still has the ToolCallLimit deny the over-budget call, and one run with RAG retrieval attached is rationed by the Commissary on the input side (shed context and the shared omission marker in the prompt the model received) and cut by the derived budget on the output side, with the identical first prompt and no halt when no figure is carried"
    requirement: "ALLOW-05"
    verification:
      - kind: integration
        ref: "limits.rs#tests::model_call_limit_still_binds_first_under_a_derived_figure, #tool_call_limit_still_denies_under_a_derived_figure; paladin_execution_service.rs#derived_budget_tests::derived_budget_composes_with_the_commissary_rationed_rag_context (a mutation that stops the service setting the figure on the context turned it red at the call-count assertion); cargo test -p paladin-llm --lib commissary: 20 passed; cargo test -p paladin-ai --test rag_commissary: 3 passed"
        status: pass
    human_judgment: false
  - id: D11
    description: "The surface is registered: MIGRATION 9.2 rows for StopReason, DerivedTokenBudget with RunScope and Admission, the admission port and ModelUnpriced, and Treasurer with ModelCallContext and TokenBudget; a CHANGELOG entry; the refreshed baseline"
    requirement: "ALLOW-05"
    verification:
      - kind: other
        ref: "./scripts/check-migration-allowlist.sh exit 0 (all four rows are N or N/A, no allowlist entry); PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface-update then make api-surface exit 0 (4209 items)"
        status: pass
    human_judgment: false

duration: ~1h
completed: 2026-10-06
status: complete
---

# Phase 42 Plan 07: The derived token budget and the Treasurer-aware cutoff Summary

**The Treasurer now derives a per-run token budget from the tightest remaining allowance at the model's dearest price (i128 floor division, saturating u32), and the one existing `TokenBudget` enforces the tighter of it and the operator's figure, ending the run with the new `StopReason::AllowanceHalted` while `ModelCallLimit`, `ToolCallLimit` and the Commissary are proven untouched and composed.**

## Performance

- **Duration:** about 1 hour
- **Completed:** 2026-10-06
- **Tasks:** 3
- **Files modified:** 12 (1 new)

## Accomplishments

- Landed the value types and the model-aware admission. `StopReason::AllowanceHalted(AllowanceRefusal)`, `DerivedTokenBudget`, the `RunScope` carrier and the `Admission` field exist with doc tests; `AllowanceAdmissionPort::admit_for_model` is defaulted (delegating to `admit`) so no implementor breaks, and `AdmissionError::ModelUnpriced` carries the D-10 refusal.
- Built the derivation in `treasurer/derive.rs` over the same `Treasurer::evaluate` admission uses: dearest of five axes with `cost_of_call`'s defaults, `i128` floor division, saturating `u32`, a free model unbudgeted, a zero figure refused with the binding ceiling's real figures, the least-headroom ceiling binding (ties to policy order), and the halt figures reporting that ceiling at balance equal to ceiling (A5). A refused derivation claims no notice because the derivation sits before the claim in the shared `admit_inner`.
- Made the one `TokenBudget` Treasurer-aware: the figure arrives per run on `ModelCallContext` (set once in `execute_internal` from the scope `execute_scoped` receives), never on the middleware struct; a derived win or tie finishes with `AllowanceHalted`, an operator win keeps `TokenBudget`.
- Proved ALLOW-05's "works alongside, replaces none" on a real service: the composition test shows the Commissary's shed RAG context and its shared omission marker in the prompt the model received while the same run is cut by the derived budget after the crossing response; the call limits are shown still binding under a derived figure.
- Registered the surface (MIGRATION 9.2, CHANGELOG, baseline).

## Task Commits

1. **Task 1: value types, defaulted `admit_for_model`, `ModelUnpriced`, the Treasurer's derivation** -- `816c8cd6` (feat)
2. **Task 2: one `TokenBudget`, tightest wins, the scope-to-context plumbing and the composition proofs** -- `27ac680a` (feat)
3. **Task 3: MIGRATION 9.2 rows, CHANGELOG entry, refreshed API baseline** -- `314cac4d` (docs)

Plan metadata (SUMMARY, STATE, ROADMAP) is committed after this file; its hash is given in the orchestrator report.

## ALLOW-05 prohibition evidence

Both ALLOW-05 prohibitions stay `status: unresolved` per the phase's descriptor-less prohibition policy (42-01 edge_probe_record). Evidence for the values prohibition (no replacement, disabling or loosening of `ModelCallLimit`, `ToolCallLimit`, the Commissary or the operator figure):

- `operator_figure_above_the_derived_one_never_loosens_it` and `a_tie_goes_to_the_treasurer` (the derived figure only ever tightens).
- `model_call_limit_still_binds_first_under_a_derived_figure`, `tool_call_limit_still_denies_under_a_derived_figure`, and `derived_budget_composes_with_the_commissary_rationed_rag_context`.
- Diff checks: no removed line of `limits.rs` names `ModelCallLimit` or `ToolCallLimit` (count 0 against `HEAD` before the Task 2 commit); the Commissary, its window resolver and `rag_retrieval_service.rs` have no uncommitted change and no Phase 42 commit (both greps print nothing).

For the safety prohibition (no second budget mechanism, loop check or mid-stream cut): the only cutoff is `TokenBudget::after_model`, after a response completes, and the streamed path was not touched.

## Deviations from Plan

None of Rules 1 to 4 applied; the points below are departures from the letter of the dispatch or the plan text.

1. **Commit trailer model name.** The dispatch notes asked for a `Claude Fable 5.1` trailer; the session's attribution reminder specifies `Claude Sonnet 5.5`, which is also the actual model. Task 1 was first committed with the dispatch wording and amended once (unpushed) to the reminder's wording; all three task commits carry `Co-Authored-By: Claude Sonnet 5.5` and the session line. This matches the 42-01 precedent, and the orchestrator can re-trailer before push if it wants the dispatch wording.
2. **`#[allow(clippy::too_many_arguments)]`** on `execute_bounded` and `execute_internal`: each now takes eight parameters with the derived budget, and the repo already uses this attribute in several places. A bundling struct was judged more churn than the one-parameter addition warranted.
3. **Acceptance text about the baseline.** Task 3's criterion says `.project/current-exports.txt` contains `with_derived_token_budget`. The baseline lists the `paladin` facade crate's own items only, so methods on re-exported core types (including the existing `RunScope::with_allowance_warnings`) are not in it. The facade items this plan added are present (`Treasurer::derive_budget`, `Treasurer::with_pricing`, `Treasurer::admit_for_model`, `ModelCallContext::derived_token_budget`), and the baseline update was reviewed line by line against the plan's artifacts row. `admit_for_model` is present through the Treasurer implementation.
4. **Doc edit in `limits.rs`.** A first edit re-wrapped a module-doc line naming `ToolCallLimit`, which would have tripped the plan's no-removed-line check; the paragraph was restructured so the original lines are intact and the new text is a separate paragraph.
5. **TDD commit shape.** Tests and implementation are committed together per task (the plan's per-task commit granularity), not as separate RED and GREEN commits. Red-first was observed for the middleware change (seven new `limits.rs` tests were red before it), and the composition test was shown red by a mutation.

## Known Stubs

None.

## Threat Flags

None. No endpoint, auth path, file access or schema change was added. The threat register's T-42-27, T-42-29 and T-42-48 mitigations are covered by D3, D8 to D10 above; T-42-32 is the accepted concurrency backstop (D6). No credential, response body or log line touching a key was added, so the manual credential-handling review found nothing to review.

## Handoff notes for plan 42-08

- `Admission::derived_budget()` returns the `DerivedTokenBudget`; `RunScope::with_derived_token_budget` carries it into `execute_scoped`. The figure and the crossing test (`total tokens > max_tokens`, strict) are public, so the option-b informational `halt_reason` on a true streamed `done` can be implemented by comparing the terminal chunk's usage against `max_tokens` and reporting `halt_figures`.
- Nothing installs `TokenBudget` on a production service yet: no per-agent service is wired, and `stop_reason_label` in `agent_controller.rs` still has its wildcard arm (the `allowance_halted` label, `ExecuteResponse.halt_reason`, the `422` mapping and `Treasurer::with_pricing` in `build_run_api` are 42-08's).

## Gates and tests run

- `cargo test -p paladin-ai-core --lib`: 695 passed; `cargo test -p paladin-ai-core --doc`: 135 passed (38 ignored as before).
- `cargo test -p paladin-ports --lib allowance_admission_port`: 2 passed; `--doc allowance_admission_port`: 2 passed.
- `cargo test -p paladin-ai --lib application::services::treasurer`: 70 passed; `--doc treasurer`: 16 passed; the named doc filters `with_pricing`, `derive_budget`, `DerivedTokenBudget`, `with_derived_token_budget`, `admit_for_model` and `derived_token_budget` each report 1 passed.
- `cargo test -p paladin-ai --lib application::services::paladin`: 265 passed, including the 13 new `limits.rs` tests and the composition test.
- `cargo test -p paladin-llm --lib commissary`: 20 passed; `cargo test -p paladin-ai --test rag_commissary`: 3 passed (both unchanged suites).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean; `cargo fmt --check` clean; `cargo check --workspace --all-targets --all-features` clean (integration tests under `tests/` compile).
- The orchestrator's gate: `cargo build --workspace --all-features` built; `cargo test --workspace --lib --bins` (with the keep-going flag) passed every crate except the two known sandbox-only cases in `run_api_wiring::tests` (`build_run_api_persists_no_run_traces_by_default` and `build_run_api_persists_run_traces_when_trace_persist_is_set`, which need outbound network and are already in the phase's `deferred-items.md`). `application::services::run::cancel_tests::local_cancel_signals_token` passed in the full suite.
- `./scripts/check-migration-allowlist.sh` exit 0; `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` exit 0 after the refresh (4209 items); `make security` exit 0 (advisories, bans, licenses, sources ok; the unmaintained-crate notices are pre-existing and no dependency was added).
- Not run: the Redis integration tests (no Redis here); the PostgreSQL contract legs (PostgreSQL is not running, so they self-skip and are not claimed); `cargo-semver-checks` (not available locally; the 9.2 rows are N or N/A, so the allowlist set is unchanged).

## Self-Check: PASSED

- FOUND: `/home/user/paladin-dev-env/src/application/services/treasurer/derive.rs`
- FOUND: commits `816c8cd6`, `27ac680a` and `314cac4d` in `git log`
- Acceptance greps: `AllowanceHalted` appears in the `is_limit` body and not in the `is_successful` body; `fn derive_max_tokens` and `fn dearest_price_per_million` present; no `f32` or `f64` in `derive.rs`; no per-run field on the `TokenBudget` struct.
