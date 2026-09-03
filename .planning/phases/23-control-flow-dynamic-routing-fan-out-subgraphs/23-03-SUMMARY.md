---
phase: 23-control-flow-dynamic-routing-fan-out-subgraphs
plan: 03
subsystem: api
tags: [rust, llm, edge-routing, commander, battalion, async-trait, paladin-llm]

# Dependency graph
requires:
  - phase: 23-01
    provides: "EdgeConditionEvaluator trait, EdgeContext (source/target/battlefield/thread/superstep), EdgeEvaluatorRegistry (crate::edge_evaluator)"
provides:
  - "LlmDecisionEvaluator: an EdgeConditionEvaluator registered under EdgeCondition::Custom(name) that routes an edge from a live LLM's answer against a closed, author-declared choice list, with one memoized LLM call per decision per superstep"
  - "OnAmbiguous::{Fail, Default(String)} for unmatched-answer resolution"
  - "Commander StrategySelection::{Heuristic, Semantic { llm, model }} behind the additive CommanderBuilder::strategy_selection method, with deterministic heuristic fallback on any LLM error or unrecognized answer"
affects: [24-hitl, 25-fault-tolerance]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Registered evaluator over trait-object LLM port (Arc<dyn LlmPort>) held privately, never exposed via Debug"
    - "Single-slot memo (tokio::sync::Mutex<Option<(MemoKey, MemoOutcome)>>) keyed on (thread, superstep, source, rendered prompt) to guarantee one LLM call per decision per superstep"
    - "Short fixed-class error taxonomy (llm_error_class) instead of interpolating a provider's raw error string, to satisfy the egress-boundary privacy rule"
    - "Deterministic fallback-with-cause-recorded pattern for Semantic strategy selection, reusing the existing strategy_selection_reasoning result field rather than adding one"

key-files:
  created:
    - crates/paladin-battalion/src/llm_decision.rs
  modified:
    - crates/paladin-battalion/src/lib.rs
    - crates/paladin-battalion/src/commander.rs
    - crates/paladin-battalion/Cargo.toml
    - MIGRATION.md

key-decisions:
  - "LlmDecisionEvaluator carries a private `name` field (constructor arg) purely for error-message identity -- EdgeContext has no field naming the registry key the evaluator is registered under, so the evaluator cannot otherwise identify itself in a typed EdgeEvaluatorError."
  - "Memo key includes the rendered prompt string alongside (thread, superstep, source) per D-24's exact wording, even though in practice the prompt is deterministic for a fixed source/thread/superstep -- defensive against a future change to render_prompt that could vary within one superstep."
  - "llm_error_class() maps every LlmError variant to a short fixed &'static str (never the inner String payload) and is shared between llm_decision.rs and commander.rs via pub(crate) -- the single place the egress-boundary privacy rule is enforced in code, not just documented."
  - "StrategySelection's manual Debug impl prints only the variant name and model string, matching T-23-10's threat-register mitigation verbatim."

requirements-completed: [CF-05]

coverage:
  - id: D1
    description: "LlmDecisionEvaluator resolves EdgeCondition::Custom(name) edges from a model verdict, matching choices exact-after-trim/case-insensitive, firing exactly the edge whose target maps to the matched choice"
    requirement: CF-05
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/llm_decision.rs#matching_choice_fires_only_the_mapped_edge"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/llm_decision.rs#matching_is_exact_after_trim_and_case_insensitive"
        status: pass
    human_judgment: false
  - id: D2
    description: "Exactly one LlmPort call per decision per superstep regardless of outgoing edge count (D-24 memo), with a fresh call on a new superstep"
    requirement: CF-05
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/llm_decision.rs#one_llm_call_per_decision_per_superstep"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/llm_decision.rs#a_different_superstep_re_asks"
        status: pass
    human_judgment: false
  - id: D3
    description: "Unmatched model answer resolves through OnAmbiguous: Fail returns a typed EdgeEvaluatorError, Default(choice) routes as if the model had answered that choice"
    requirement: CF-05
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/llm_decision.rs#unmatched_answer_with_on_ambiguous_fail_errors"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/llm_decision.rs#unmatched_answer_with_on_ambiguous_default_routes_to_the_default_choice"
        status: pass
    human_judgment: false
  - id: D4
    description: "An LlmError from the port surfaces as a typed EdgeEvaluatorError naming the evaluator and a fixed failure class, never the rendered prompt or the response body"
    requirement: CF-05
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/llm_decision.rs#llm_error_surfaces_as_a_typed_evaluator_error"
        status: pass
    human_judgment: false
  - id: D5
    description: "Template rendering supports both the engine path (InputMapping over Battlefield) and the legacy path ({output} substitution from the source Paladin's output)"
    requirement: CF-05
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/llm_decision.rs#legacy_path_renders_the_template_from_the_source_output"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/llm_decision.rs#engine_path_renders_the_template_from_the_battlefield"
        status: pass
    human_judgment: false
  - id: D6
    description: "Commander StrategySelection defaults to Heuristic (today's analyze_and_select, unchanged) when no builder call is made"
    requirement: CF-05
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/commander.rs#default_strategy_selection_is_heuristic_and_unchanged"
        status: pass
    human_judgment: false
  - id: D7
    description: "Semantic mode prompts the model with the strategy catalog, matches its answer exact-after-trim/case-insensitive, and selects the named strategy"
    requirement: CF-05
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/commander.rs#semantic_mode_selects_the_strategy_the_model_names"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/commander.rs#semantic_matching_is_exact_after_trim_and_case_insensitive"
        status: pass
    human_judgment: false
  - id: D8
    description: "Any LlmError or an unrecognized model answer falls back to the heuristic deterministically, with the fallback and its cause class recorded in strategy_selection_reasoning (never the raw answer or error body)"
    requirement: CF-05
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/commander.rs#semantic_falls_back_to_heuristic_on_llm_error"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/commander.rs#semantic_falls_back_to_heuristic_on_unrecognized_answer"
        status: pass
    human_judgment: false
  - id: D9
    description: "Commander::new keeps its exact signature and Commander gains no new public field; StrategySelection is reachable only through CommanderBuilder::strategy_selection"
    requirement: CF-05
    verification:
      - kind: other
        ref: "awk '/pub struct Commander \\{/,/^\\}/' crates/paladin-battalion/src/commander.rs | grep -c 'pub .*StrategySelection' => 0"
        status: pass
      - kind: other
        ref: "grep -c 'pub fn new(paladin_port' crates/paladin-battalion/src/commander.rs => 1"
        status: pass
    human_judgment: false
  - id: D10
    description: "The existing 52 inline Commander tests plus both tests/integration/commander_integration_tests.rs and commander_error_paths_test.rs pass unmodified"
    requirement: CF-05
    verification:
      - kind: unit
        ref: "cargo test -p paladin-battalion --lib commander => 57 passed (52 pre-existing + 5 new), 0 failed"
        status: pass
      - kind: integration
        ref: "cargo test -p paladin-ai --test lib commander => 25 passed, 0 failed; git diff of both test files empty"
        status: pass
    human_judgment: false
  - id: D11
    description: "LlmDecision and StrategySelection::Semantic are reachable only through in-code configuration -- no APP_* env var, no cargo feature, no config-struct field"
    requirement: CF-05
    verification:
      - kind: other
        ref: "grep -rn 'APP_LLM_DECISION|APP_.*SEMANTIC' src/ crates/ => no matches"
        status: pass
    human_judgment: false

# Metrics
duration: 30min
completed: 2026-09-03
status: complete
---

# Phase 23 Plan 03: LLM-Evaluated Routing (CF-05) Summary

**`LlmDecisionEvaluator` edge routing with a one-call-per-decision-per-superstep memo, and Commander `StrategySelection::Semantic` with a deterministic heuristic fallback -- both off by default and reachable only through in-code configuration.**

## Performance

- **Duration:** ~30 min
- **Started:** 2026-09-03T19:50:00Z (approx)
- **Completed:** 2026-09-03T20:14:04Z
- **Tasks:** 2 completed
- **Files modified:** 5 (1 created, 4 modified)

## Accomplishments

- `LlmDecisionEvaluator` (new `crates/paladin-battalion/src/llm_decision.rs`) implements `EdgeConditionEvaluator`, registered under `EdgeCondition::Custom("<decision name>")` through the CF-01 `EdgeEvaluatorRegistry` -- `paladin-core`'s `EdgeCondition` enum gains no new variant.
- Choice matching is exact-after-trim, case-insensitive; an unmatched answer resolves through `OnAmbiguous::{Fail, Default(String)}`, defaulting to `Fail`.
- A single-slot memo keyed on `(thread, superstep, source, rendered prompt)` guarantees exactly one `LlmPort::generate` call per decision per superstep regardless of outgoing edge count (D-24) -- the routing-corruption class BUG-01 was is structurally prevented, not just tested against.
- Template rendering supports both the `WarEngine` path (`InputMapping::render` over `Battlefield`) and the legacy `CampaignExecutionService` path (`{output}` substitution from the source Paladin's raw output).
- Commander gains `StrategySelection::{Heuristic, Semantic { llm: Arc<dyn LlmPort>, model: String }}` with a manual `Debug` impl (prints only the variant name and model string, never the port). Default is `Heuristic` -- today's `analyze_and_select` keyword heuristic, byte-for-byte unchanged.
- `CommanderBuilder::strategy_selection(selection)` is the sole additive entry point; `Commander::new`'s signature is untouched and `Commander` gains no new public field (private field, defaulted in `Commander::new`, set only through the builder).
- Semantic mode prompts the model with the strategy catalog (`Formation, Phalanx, Campaign, ChainOfCommand, Conclave, Council, Grove` -- `Maneuver` excluded, matching `analyze_and_select`'s own explicit-only note) plus the run's input; any `LlmError` or an answer naming no catalog strategy falls back to the heuristic deterministically, recording the fallback and a short, fixed cause class (never the model's raw answer or a provider's raw error text) in the existing `strategy_selection_reasoning` result field.
- `paladin-llm` added to `paladin-battalion`'s `[dev-dependencies]` only (never `[dependencies]`) for `MockLlmAdapter` in tests; confirmed acyclic and absent from `cargo tree -e normal`.
- `MIGRATION.md` §9.2 `Commander`/`CommanderBuilder` row resolved (mitigation, `N` deliberate-breaking); §9.5 gained a new bullet recording that `LlmDecision`/`StrategySelection::Semantic` introduce no config struct or `APP_*` environment variable (D-26).

## Task Commits

Each task was committed atomically:

1. **Task 1: LlmDecision edge evaluator, end-to-end with one call per decision** - `793bc3c4` (feat)
2. **Task 2: Commander StrategySelection::Semantic with a recorded heuristic fallback** - `bf627526` (feat)

**Plan metadata:** committed alongside this SUMMARY (worktree mode -- STATE.md/ROADMAP.md updates deferred to the orchestrator after wave merge).

_Note: both tasks were `tdd="true"`; tests were written and run alongside the implementation in the same commit per the tracer/auto task types (not a separate RED/GREEN commit pair) -- this matches Plan 23-01's own precedent for this phase's task granularity._

## Files Created/Modified

- `crates/paladin-battalion/src/llm_decision.rs` - `LlmDecisionEvaluator`, `OnAmbiguous`, the D-24 memo, `llm_error_class()`; 9 unit tests + 1 doctest
- `crates/paladin-battalion/src/lib.rs` - registers `pub mod llm_decision;` and re-exports `LlmDecisionEvaluator`/`OnAmbiguous` at crate root beside `edge_evaluator`
- `crates/paladin-battalion/src/commander.rs` - `StrategySelection` enum + manual `Debug`, private `Commander` field, `CommanderBuilder::strategy_selection`, `select_strategy`/`select_strategy_semantic`/`strategy_from_name`/`fall_back_to_heuristic` methods; 5 new tests
- `crates/paladin-battalion/Cargo.toml` - `paladin-llm` added to `[dev-dependencies]`, with an acyclic-edge comment
- `MIGRATION.md` - §9.2 Commander/CommanderBuilder row resolved; §9.5 new bullet for D-26

## Decisions Made

- **`LlmDecisionEvaluator` carries a private `name` field for error-message identity.** `EdgeContext` (Plan 23-01) has no field naming the registry key an evaluator is registered under, so without a self-known name the evaluator could not produce a typed `EdgeEvaluatorError` that "names the evaluator" as CF-FR-18/D-04 require. Set via the constructor's first argument; independent of (though typically matching) the registry name.
- **Memo key includes the rendered prompt, not just `(thread, superstep, source)`.** D-24's wording names all four; in practice the prompt is deterministic for a fixed source/thread/superstep, but including it is defensive against a future `render_prompt` change and costs nothing (the memo is already single-slot).
- **`llm_error_class()` is `pub(crate)` and shared** between `llm_decision.rs` and `commander.rs` -- the one place in this plan's code where the egress-boundary privacy rule (never interpolate a provider's raw error text) is mechanically enforced rather than only documented.
- **Semantic strategy catalog is `Formation, Phalanx, Campaign, ChainOfCommand, Conclave, Council, Grove`** -- `Maneuver` deliberately excluded, mirroring `analyze_and_select`'s own "Maneuver is EXPLICIT-ONLY and NOT selected by Auto mode" comment, so `Semantic` and `Heuristic` never diverge in which strategies Auto mode can reach.

## Deviations from Plan

None — plan executed as written, with two implementation-detail choices left to discretion resolved as noted in "Decisions Made" above (both were explicitly marked Claude's discretion in CONTEXT.md D-23/D-25, not deviations from a locked decision).

One acceptance-criterion wording issue was caught and corrected before commit (not a deviation from the plan's intent, but worth recording): the plan's action item for Task 1 asked the new `Cargo.toml` comment to be "in the style of the existing `paladin-storage` dev-dependency comment", which itself names its own crate inside its prose (and so registers 2 grep hits for `paladin-storage`) — but the plan's acceptance criterion for Task 1 requires `grep -c 'paladin-llm' Cargo.toml` to return exactly `1`. Followed the acceptance criterion (the testable contract) over the literal style precedent: the comment describes the dependency as "it" rather than repeating "paladin-llm" by name, satisfying both the acyclic-edge documentation requirement and the exact grep count.

## Issues Encountered

**Slow full-workspace verification, not a code issue.** This worktree's cold target directory made `cargo clippy --workspace --all-targets --all-features -- -D warnings` and `cargo test --workspace --lib --bins` take several minutes each (consistent with the orchestrator's `workflow.worktree_skip_hooks=true` note). Both were run to completion in the foreground as Self-Check evidence rather than relied upon via background polling, per the orchestrator's explicit redirection mid-execution. Final results: `cargo fmt --check` clean, `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean (exit 0), `cargo test --workspace --lib --bins` exit 0 with zero `FAILED`/`error` lines across the entire log and no `test result:` line reporting a nonzero failed count.

## User Setup Required

None — no external service configuration required. `LlmDecisionEvaluator` and `StrategySelection::Semantic` both require the workflow author to supply their own `Arc<dyn LlmPort>` in code (e.g. a real provider adapter from `paladin-llm`); no environment variable or config file section is consulted.

## Next Phase Readiness

CF-05 is closed: `LlmDecisionEvaluator` and `StrategySelection::Semantic` are both implemented, tested, off by default, and documented per the egress-boundary security note. `MIGRATION.md` §9.2's `Commander`/`CommanderBuilder` row is resolved, leaving no `TBD — owner CF-05, Phase 23` markers in the tree. No blockers for later phases; Phase 25 (fault tolerance) and Phase 24 (HITL) do not depend on this plan's specific deliverables beyond the general `EdgeConditionEvaluator`/`Commander` surface Plan 23-01 already established.

---
*Phase: 23-control-flow-dynamic-routing-fan-out-subgraphs*
*Completed: 2026-09-03*
