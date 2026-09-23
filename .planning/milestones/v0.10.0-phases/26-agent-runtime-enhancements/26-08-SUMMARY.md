---
phase: 26-agent-runtime-enhancements
plan: 08
subsystem: agent-runtime
tags: [guardrail, regex, redos, middleware, paladin-error, rust]

requires:
  - phase: 26-01
    provides: "ExecutionMiddleware trait, MiddlewareFlow/ToolFlow, FinalResult, ModelCallContext/ToolCallContext/LlmResponseView/PromptAssembly/PromptSection context types"
  - phase: 26-02
    provides: "GuardrailConfig/GuardrailRuleConfig/GuardrailTarget/GuardrailOnMatch sub-structs on AgentRuntimeConfig, including the documented pattern_size_limit_bytes default"
  - phase: 26-05
    provides: "ExecutionMiddleware::around_tool's &mut ToolCallContext signature (not consumed by this plan, but confirms the current trait shape) and the StopReason::Completed usage pattern Finish relies on"
provides:
  - "Guardrail middleware in src/application/services/paladin/middleware/guardrail.rs: GuardrailRule, GuardrailTarget, GuardrailMatcher (Regex | Predicate), GuardrailAction (Fail | Redact | Finish), Guardrail::new/from_rules"
  - "PaladinError::GuardrailTripped { rule, target } -- free under the pre-existing #[non_exhaustive] attribute, no new MIGRATION.md §9.2 row"
  - "Construction-time regex compilation through RegexBuilder::size_limit(pattern_size_limit_bytes) with a typed GuardrailBuildError on syntax or size failure"
affects: [26-20, 26-21]

tech-stack:
  added: []
  patterns:
    - "Config-supplied Regex matchers vs code-only Predicate matchers as two variants of one runtime enum, with a From<config type> conversion at the config/runtime boundary"
    - "Screen every text-bearing part of a structured PromptAssembly individually (never a flattened string) so a redaction lands in the section that actually matched"
    - "A middleware's Finish action must write its message into the mutable response-view field the service's call site actually reads (LlmResponseView::content), not just into FinalResult::output -- the same rule TokenBudget::after_model documents"

key-files:
  created:
    - src/application/services/paladin/middleware/guardrail.rs
  modified:
    - crates/paladin-core/src/platform/container/paladin_error.rs
    - src/application/services/paladin/middleware/mod.rs

key-decisions:
  - "GuardrailTripped.target is a String (\"prompt\" or \"response\"), not a core-side enum -- keeps the guardrail vocabulary out of paladin-core (per the plan's own stated choice), and it names the SIDE ACTUALLY SCREENED at the moment of the trip rather than the rule's (possibly Both) configured target, which is more diagnostically precise"
  - "GuardrailTarget/GuardrailMatcher/GuardrailAction are runtime types distinct from config::agent_runtime's GuardrailTarget/GuardrailOnMatch, converted via GuardrailRule::from_config -- mirrors how the plan already specifies GuardrailAction as tuple variants (Redact(String), Finish(String)) distinct from the config's struct-variant GuardrailOnMatch"
  - "A Predicate matcher's Redact action replaces the WHOLE screened field rather than a partial match, since a bool-returning closure carries no match span to redact -- documented inline as a deliberate difference from a Regex matcher's partial substitution"
  - "Guardrail and CompiledRule derive Debug (via a manual Debug impl on GuardrailMatcher, since Arc<dyn Fn> has none) so construction-failure tests can use expect_err ergonomically"

patterns-established:
  - "The redact-then-continue / fail-or-finish-then-stop ordering rule (EDGE(RT-02/ordering)) is implemented as one shared apply_to_field helper returning a RuleOutcome enum (Fail | Finish | Redacted), consumed identically by the prompt sweep and the response screen"

requirements-completed: [RT-02]

coverage:
  - id: D1
    description: "GuardrailRule/GuardrailTarget/GuardrailMatcher/GuardrailAction exist in D-09's exact locked shape; Regex patterns compile once at construction through RegexBuilder::size_limit(pattern_size_limit_bytes) with a typed GuardrailBuildError naming the rule on syntax or size failure; PaladinError::GuardrailTripped { rule, target } exists as a structured, free (#[non_exhaustive]) variant"
    requirement: "RT-02"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/middleware/guardrail.rs#tests::valid_rules_compile_at_construction, ::invalid_regex_is_a_typed_construction_error, ::oversized_pattern_is_rejected_at_the_documented_bound, ::guardrail_tripped_is_structured, ::predicate_matcher_is_code_only, ::empty_rule_set_is_the_default"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/paladin_error.rs#tests::paladin_error_transience_table"
        status: pass
    human_judgment: false
  - id: D2
    description: "before_model screens Prompt|Both rules over every PromptAssembly text part individually (redaction lands in the matching section, other sections byte-identical); after_model screens Response|Both rules over LlmResponseView::content"
    requirement: "RT-02"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/middleware/guardrail.rs#tests::prompt_screen_redacts_in_the_matching_section, ::response_screen_redacts_response_content, ::both_target_screens_prompt_and_response"
        status: pass
    human_judgment: false
  - id: D3
    description: "Fail returns PaladinError::GuardrailTripped with zero LlmPort calls for that iteration; Finish finishes the run with the message and StopReason::Completed; declaration order is honored with the first Fail/Finish winning and Redact not stopping the sweep"
    requirement: "RT-02"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/middleware/guardrail.rs#tests::fail_action_returns_guardrail_tripped, ::finish_action_finishes_with_the_message_and_completed, ::rules_apply_in_declaration_order_and_first_terminal_action_wins"
        status: pass
    human_judgment: false
  - id: D4
    description: "A Predicate matcher produces the same three actions as an equivalent Regex matcher, and an installed-but-non-matching Guardrail is byte-identical to no Guardrail at all"
    requirement: "RT-02"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/middleware/guardrail.rs#tests::predicate_rule_screens_like_a_regex_rule, ::no_match_is_a_pure_pass_through"
        status: pass
    human_judgment: false
  - id: D5
    description: "Full workspace is green: no new StopReason::Guardrail variant introduced, no bare Regex::new outside doc comments, all named test suites pass, formatting and lints clean"
    requirement: "RT-02"
    verification:
      - kind: other
        ref: "cargo check --workspace --all-targets --all-features (clean); cargo clippy --workspace --all-targets --all-features -- -D warnings (clean); cargo fmt --all --check (clean); cargo test -p paladin-ai --lib (606 pass); cargo test -p paladin-ai-core --lib (509 pass); cargo test -p paladin-ai --doc (120 pass); cargo doc -p paladin-ai --no-deps (no new broken links)"
        status: pass
    human_judgment: false

duration: ~35min
completed: 2026-09-07
status: complete
---

# Phase 26 Plan 08: Guardrail Middleware Summary

**`Guardrail` middleware: named rules screen outbound prompt sections and inbound responses with a regex (compiled once at construction under an explicit size bound) or a code predicate, acting by structured `PaladinError::GuardrailTripped` failure, in-place redaction, or a `StopReason::Completed` finish.**

## Performance

- **Duration:** ~35min
- **Tasks:** 2 (both `tdd="true"`, combined into one RED/GREEN commit pair -- see Task Commits note)
- **Files modified:** 3 (1 created, 2 modified)

## Accomplishments

- `GuardrailRule { name, target, matcher, on_match }`, `GuardrailTarget { Prompt | Response | Both }`, `GuardrailMatcher { Regex(String) | Predicate(Arc<dyn Fn(&str) -> bool + Send + Sync>) }` and `GuardrailAction { Fail | Redact(String) | Finish(String) }` exist in D-09's exact locked shape (`src/application/services/paladin/middleware/guardrail.rs`).
- `Guardrail::new(GuardrailConfig)` and `Guardrail::from_rules(Vec<GuardrailRule>, size_limit)` compile every `Regex` matcher exactly once, at construction, through `RegexBuilder::new(pattern).size_limit(pattern_size_limit_bytes).build()` -- never a bare `Regex::new` relying on the crate's generous internal default. A pattern that fails to parse or exceeds the configured bound fails construction with a typed `GuardrailBuildError::InvalidPattern { rule, source }` naming the offending rule; the `regex::Error::CompiledTooBig(limit)` source names the configured bound in its own `Display`.
- `PaladinError::GuardrailTripped { rule, target }` exists on `paladin-core`'s already-`#[non_exhaustive]` `PaladinError` -- free, no new `MIGRATION.md` §9.2 row. `target` names the side actually screened ("prompt" or "response") when the rule tripped, not the rule's (possibly `Both`) configured target.
- `before_model` screens `Prompt`/`Both` rules over every one of `PromptAssembly`'s text-bearing parts individually (system, retrieved context, each Garrison history entry, input, accumulated output, every pushed `PromptSection` body) -- a redaction lands in the section that matched, leaving every untouched section byte-identical. `after_model` screens `Response`/`Both` rules over `LlmResponseView::content`.
- `Fail` returns `MiddlewareFlow::Fail(PaladinError::GuardrailTripped { .. })` with zero `LlmPort` calls for that iteration (the service's `run_before` call site propagates the error via `?` before the prompt is even rendered). `Finish(msg)` finishes the run with `msg` as the output and `StopReason::Completed` -- no `StopReason::Guardrail` variant was added (a Deferred Idea, not an RT FR). `Redact(r)` replaces every match with `r` in place and does not stop the sweep -- a later rule in the same pass sees the already-redacted text, proven by a rule ordering test where a `Redact("foo"->"bar")` rule is followed by a `Fail` rule matching `"bar"` (which only appears after the redaction).
- A `Predicate` matcher produces the same three actions as an equivalent `Regex` matcher; an installed-but-non-matching `Guardrail` is byte-identical to no `Guardrail` at all (rendered prompt, port call count, and `PaladinResult` all match a plain baseline run).
- The module's security paragraph states the argument explicitly: the `regex` crate is a finite-automata, no-backtracking engine with a linear-time matching guarantee (no haystack-driven ReDoS surface); the residual concern is an adversarial *pattern* producing a large compiled program, mitigated by the explicit, config-driven `size_limit` rather than the crate's generous default; substituting `fancy-regex` (a backtracking engine) would invalidate the argument and is a recorded prohibition.

## Task Commits

1. **Task 1 + Task 2 (combined RED/GREEN, same file):**
   - RED: `9d75e957` -- test(26-08): add failing tests for Guardrail construction and screening
   - GREEN: `edbbef87` -- feat(26-08): Guardrail middleware compiles patterns under an explicit size bound

_Note on the RED/GREEN split: both plan tasks (Task 1 "construction-time regex compilation and the typed error" and Task 2 "prompt and response screens") modify the exact same new file (`guardrail.rs`) and Task 2's screening logic is written against Task 1's types -- an artificial file-level split would have required temporarily stubbing the `ExecutionMiddleware` impl. Both tasks' full 14-test suite was written together against a deliberately incomplete first draft carrying three isolated defects: (1) `compile()` used a bare `Regex::new(pattern)` instead of `RegexBuilder::new(pattern).size_limit(size_limit)`, so it silently relied on the crate's generous internal default instead of the configured bound; (2) `before_model` checked `compiled.rule.target != GuardrailTarget::Prompt` (exact equality) instead of `screens_prompt()`, so a `Both`-target rule never screened the prompt side; (3) `GuardrailAction::Finish`'s branch returned the message as `RuleOutcome::Finish` without also writing it into the screened `&mut String`, which for the response side is `resp.content` -- the exact field `paladin_execution_service.rs`'s `run_after` call site reads into `PaladinResult.output` (`accumulated_output = response_view.content;`, not `final_result.output`). Confirmed a genuine RED state (`10 passed; 4 failed`: `oversized_pattern_is_rejected_at_the_documented_bound`, `both_target_screens_prompt_and_response`, `finish_action_finishes_with_the_message_and_completed`, `rules_apply_in_declaration_order_and_first_terminal_action_wins`) before fixing all three defects in the GREEN commit, which brought all 14 tests to green. Defect (3) was discovered during this RED run rather than planted deliberately -- see Deviations._

## Files Created/Modified

- `src/application/services/paladin/middleware/guardrail.rs` (new) -- `GuardrailRule`, `GuardrailTarget`, `GuardrailMatcher`, `GuardrailAction`, `GuardrailBuildError`, `Guardrail` (`new`/`from_rules`/`compile`/screening logic), `ExecutionMiddleware` impl, 14 unit tests
- `crates/paladin-core/src/platform/container/paladin_error.rs` -- `PaladinError::GuardrailTripped { rule, target }` variant, a `transience()` match arm classifying it `Permanent`, one new row in `paladin_error_transience_table`
- `src/application/services/paladin/middleware/mod.rs` -- `pub mod guardrail;` + re-exports of `Guardrail`, `GuardrailAction`, `GuardrailBuildError`, `GuardrailMatcher`, `GuardrailRule`, `GuardrailTarget`

## Decisions Made

- **`GuardrailTripped.target` is a `String`, not a core-side enum.** The plan offered either shape ("`target: GuardrailTargetLabel` -- or `target: String` with a documented stable value set if a core-side enum would pull the guardrail vocabulary into core unnecessarily"). A `String` keeps `paladin-core` free of any Guardrail-specific type, and its two stable values (`"prompt"`, `"response"`) are documented on the variant.
- **`target` names the side actually screened, not the rule's configured target.** A `Both`-target rule can trip on either side; reporting the literal `GuardrailTarget::Both` configuration would be less useful for diagnosing a trip than reporting which side matched. `apply_to_field`/`apply_to_prompt` take a `side: &'static str` parameter fixed at each call site ("prompt" in `before_model`, "response" in `after_model`) rather than deriving it from `rule.target`.
- **Runtime `GuardrailTarget`/`GuardrailMatcher`/`GuardrailAction` are distinct types from `config::agent_runtime`'s `GuardrailTarget`/`GuardrailOnMatch`**, converted via `GuardrailRule::from_config`. This mirrors the plan's own literal shape for `GuardrailAction` (tuple variants `Redact(String)`/`Finish(String)`) versus the config's struct-variant `GuardrailOnMatch` (`Redact { replacement }`/`Finish { message }`) -- the plan explicitly specifies both shapes differently, so a shared type was never an option.
- **A `Predicate` matcher's `Redact` action replaces the whole screened field**, not a partial match -- a `bool`-returning closure carries no match span to redact, unlike a `Regex`'s capture. Documented inline in `redact_in_place` as a deliberate difference from `Regex`'s `replace_all`.
- **`Guardrail` and `CompiledRule` derive `Debug`** via a hand-written `Debug` for `GuardrailMatcher` (since `Arc<dyn Fn(&str) -> bool + Send + Sync>` has none), so construction-failure tests can use `expect_err` ergonomically rather than manual `match`-and-panic boilerplate.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `GuardrailAction::Finish`'s message was not reaching `PaladinResult.output` for a response-side trip**
- **Found during:** Task 2, first RED test run (`finish_action_finishes_with_the_message_and_completed` and half of `rules_apply_in_declaration_order_and_first_terminal_action_wins` failed)
- **Issue:** `apply_to_field`'s `Finish` branch returned `RuleOutcome::Finish(message.clone())` without mutating the `text: &mut String` parameter. For a response-side (`after_model`) trip, that parameter is `resp.content` -- and `paladin_execution_service.rs`'s `run_after` call site reads the run's final output from `response_view.content` (`accumulated_output = response_view.content;`), never from `FinalResult::output`. `TokenBudget::after_model` (plan 26-05) already documents and follows this exact rule; `Guardrail`'s first draft did not.
- **Fix:** `Finish`'s branch now also writes `*text = message.clone();` before returning `RuleOutcome::Finish(message.clone())`. Harmless on the `before_model` (prompt) path, where `FinalResult::output` builds the synthetic response view directly and the assembly field is discarded either way.
- **Files modified:** `src/application/services/paladin/middleware/guardrail.rs`
- **Verification:** `finish_action_finishes_with_the_message_and_completed` and `rules_apply_in_declaration_order_and_first_terminal_action_wins` both pass
- **Committed in:** `edbbef87` (GREEN)

---

**Total deviations:** 1 auto-fixed (1 bug found during the plan's own RED phase, not planted deliberately alongside the two intentional defects)
**Impact on plan:** Necessary for `Finish` to behave correctly on the response side at all; no scope creep -- fixed entirely within `guardrail.rs`, the plan's own declared file for Task 2.

## Issues Encountered

None beyond the deviation above. Every acceptance criterion in the plan (struct/enum shape greps, `RegexBuilder`/`pattern_size_limit_bytes` presence, bare-`Regex::new` absence, `GuardrailTripped` presence, `StopReason::Guardrail` absence, all fourteen named test functions, `cargo test -p paladin-ai --doc`, `cargo fmt --check`, `cargo clippy -- -D warnings`) was verified directly.

## Known Stubs

None. Every type, the construction path, and both screening hooks are real, wired, production code.

## User Setup Required

None -- no external service configuration required.

## Next Phase Readiness

- `Guardrail`, `GuardrailRule`, `GuardrailTarget`, `GuardrailMatcher`, `GuardrailAction` and `GuardrailBuildError` are locked in their final public shape and ready for `AgentRuntimeConfig::build_chain` (plan 26-20) to construct via `Guardrail::new(config.guardrail)` in the documented `limits -> guardrail -> ...` assembly order.
- `PaladinError::GuardrailTripped`'s existing `#[non_exhaustive]` row is ready for plan 26-21 to extend its `MIGRATION.md` §9.2 Change cell -- no new row needed.
- No blockers for wave 4 sibling plans or later waves.

---
*Phase: 26-agent-runtime-enhancements*
*Completed: 2026-09-07*

## Self-Check: PASSED

All created/modified files verified present on disk with the expected content; both commits (`9d75e957`, `edbbef87`) verified present in `git log`; `cargo check --workspace --all-targets --all-features`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo fmt --all --check`, `cargo test -p paladin-ai --lib` (606/606), `cargo test -p paladin-ai-core --lib` (509/509) and `cargo test -p paladin-ai --doc` (120/120) all pass on the final tree.
