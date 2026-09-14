---
phase: 26-agent-runtime-enhancements
plan: 19
subsystem: agent-runtime
tags: [tool-error-policy, redaction, prompt-level-tool-calling, middleware, rust]

requires:
  - phase: 26-agent-runtime-enhancements (plan 02)
    provides: "ToolErrorConfig { mode, per_tool } / ToolErrorMode { FeedToModel, FailRun } under AgentRuntimeConfig -- the service-level config this plan wires PaladinExecutionService::with_tool_error_config against"
  - phase: 26-agent-runtime-enhancements (plan 12)
    provides: "paladin_core::platform::container::structured::extract_json -- the shared envelope extractor ToolCallProtocolMiddleware reuses rather than writing a second parser"
  - phase: 26-agent-runtime-enhancements (plan 16)
    provides: "InProcessArsenal / CompositeArsenalPort -- the ArsenalPort implementations a caller composes ToolCallProtocolMiddleware's arsenal argument from"
provides:
  - "paladin_llm::redaction::redact_secret_patterns(&str) -- the key-less pattern half of redact_credentials, covering Bearer/sk-/sk-ant-/AKIA-style keys, key=/token= query values, and JWT-shaped triples"
  - "ToolResultFormatter::format_error(&ArmamentCall, &str) -- one shared formatter for both the Arsenal and handoff tool-error arms, redact-then-bound sanitized, keeping today's shape plus the PRD's retry-or-proceed sentence"
  - "PaladinError::ArmamentFailed { tool, reason } -- the new FailRun opt-in, free under the pre-existing #[non_exhaustive] attribute"
  - "PaladinExecutionService::with_tool_error_config / effective_tool_error_mode -- service-level tool-error policy with per-tool override, routed through both the Arsenal and handoff error arms"
  - "src/application/services/paladin/middleware/tool_protocol.rs: ToolCallProtocolMiddleware (renders a ## Tools catalogue, synthesizes function_call from a documented envelope) and FinishOnPlainAnswerMiddleware (finishes the run on a plain answer) -- both opt-in, prompt-level, ADR-0042 untouched"
  - "MIGRATION.md M-B-03 rewritten: no behavioral change, with a before/after redaction example"
affects: []

tech-stack:
  added: []
  patterns:
    - "A key-less pattern redactor (redact_secret_patterns) factored out of a key-aware one (redact_credentials) so a caller with no configured credential to match exactly (a tool's error text) still gets pattern-based coverage; the key-aware function composes the pattern-only one plus its own exact-match pass"
    - "A prompt-level tool-call protocol (render a catalogue into the prompt, extract a documented JSON envelope from the plain-text response) as the way to make an existing tool-call branch reachable for providers with no native function-calling wire format, reusing the shared JSON-envelope extractor rather than writing a second one"

key-files:
  created:
    - src/application/services/paladin/middleware/tool_protocol.rs
  modified:
    - crates/paladin-llm/src/redaction.rs
    - crates/paladin-core/src/platform/container/paladin_error.rs
    - src/infrastructure/adapters/arsenal/tool_result_formatter.rs
    - src/application/services/paladin/paladin_execution_service.rs
    - src/application/services/paladin/middleware/mod.rs
    - MIGRATION.md

key-decisions:
  - "Checkpoint resolutions: auto-selected ship-the-protocol (auto-mode) -- Task 1's checkpoint:decision (gate=\"blocking\") was pre-resolved by the orchestrator per D-36: ship ToolCallProtocolMiddleware and FinishOnPlainAnswerMiddleware as opt-in built-ins exactly as specified, rather than shipping the tool loop inert for shipped providers."
  - "PaladinError::ArmamentFailed's second field is named `reason`, not the plan's literal `source` -- a field literally named `source` triggers thiserror's implicit Error::source() derivation, which requires the field's own type to implement std::error::Error; a plain sanitized String summary does not need to and should not have to. The plan's own wording ('-style variant') left this open."
  - "The handoff tool-error arm is unified onto format_error's '🔧 Tool Execution' shape, replacing its previous inline '🤝 Handoff Execution' text -- D-34 explicitly directs one shared formatter used by BOTH arms, and Test 5 (both_arms_use_the_same_formatter) requires their outputs to match one expected shape. This is a deliberate, plan-directed text unification for the FeedToModel path, not an unplanned behavioral change."
  - "redact_secret_patterns's JWT-triple detector uses a hand-written base64url run-scanner (no regex dependency added to paladin-llm, which has none today) with a JWT_MIN_SEGMENT_LEN=10 floor so a dotted version string (1.2.3) or hostname label is never misredacted -- matching the plan's 'reuse the file's char-boundary-safe approach' instruction."
  - "ToolCallProtocolMiddleware's tool catalogue renders at SectionPlacement::End (after input/accumulated output) rather than AfterRetrievedContext/BeforeHistory -- the plan left the wording and placement at the implementer's discretion (D-36); End keeps the call-format instructions closest to where the model must respond next."

patterns-established:
  - "A middleware that must decode a model's plain-text reply into a structured signal (a tool call, in this case) reuses the ONE shared JSON-envelope extractor (extract_json) the structured-output path already established, rather than writing a parser local to the middleware -- enforced here by an explicit acceptance check (grep -rc 'fn extract_envelope') rather than a self-referential Rust unit test, which would have to embed the exact literal it searches for and match itself."

requirements-completed: [RT-07]

coverage:
  - id: D1
    description: "redact_secret_patterns covers bearer tokens, sk-/sk-ant-/AKIA-style keys, key=/token= query values and JWT-shaped triples; a benign string is unchanged; redact_credentials's pre-existing behavior and tests are unaffected by the factoring-out"
    requirement: "RT-07"
    verification:
      - kind: unit
        ref: "crates/paladin-llm/src/redaction.rs#tests::redact_secret_patterns_covers_the_documented_set, tests::redaction_precedes_bounding, tests::redact_credentials_still_behaves_identically"
        status: pass
    human_judgment: false
  - id: D2
    description: "ToolResultFormatter::format_error keeps today's Tool Execution/FAILED shape, appends the PRD's retry-or-proceed sentence, and is used identically by both the Arsenal and handoff tool-error arms"
    requirement: "RT-07"
    verification:
      - kind: unit
        ref: "src/infrastructure/adapters/arsenal/tool_result_formatter.rs#tests::format_error_keeps_todays_shape_and_appends_the_prd_sentence, tests::both_arms_use_the_same_formatter, tests::format_error_redacts_a_secret_in_the_reason_before_bounding"
        status: pass
    human_judgment: false
  - id: D3
    description: "FeedToModel (default) matches v0.9 behavior exactly apart from sanitization; FailRun fails the run with a structured PaladinError::ArmamentFailed naming the tool; a per_tool override beats the global mode; a secret in a tool's error text never reaches the model"
    requirement: "RT-07"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/paladin_execution_service.rs#middleware_wiring_tests::feed_to_model_is_the_default_and_matches_v0_9, ::fail_run_produces_a_structured_error, ::per_tool_override_beats_the_global_mode, ::a_secret_in_a_tool_error_never_reaches_the_model, ::handoff_arm_routes_through_the_same_tool_error_policy"
        status: pass
    human_judgment: false
  - id: D4
    description: "MIGRATION.md M-B-03 is rewritten with no TBD and states the corrected 'no behavioral change' premise, with a before/after sanitization example"
    requirement: "RT-07"
    verification:
      - kind: other
        ref: "grep -A2 'M-B-03' MIGRATION.md | grep -c 'TBD' == 0; grep -c 'no behavioural change\\|no behavioral change' MIGRATION.md >= 1"
        status: pass
    human_judgment: false
  - id: D5
    description: "ToolCallProtocolMiddleware renders a ## Tools catalogue (or none, for an empty arsenal) and synthesizes function_call from a documented envelope only when function_call is None, the named tool is known to the arsenal, and never overwriting a real function_call; the extraction reuses the shared extract_json with no second envelope parser anywhere in the facade"
    requirement: "RT-07"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/middleware/tool_protocol.rs#tests::tool_catalogue_is_rendered_into_a_tools_section, ::empty_arsenal_renders_no_tools_section, ::a_documented_envelope_synthesizes_a_function_call, ::a_non_envelope_json_response_is_left_alone, ::a_response_that_already_has_a_function_call_is_untouched, ::an_unknown_tool_name_in_the_envelope_is_not_synthesized"
        status: pass
      - kind: other
        ref: "grep -c extract_json tool_protocol.rs >= 1; grep -rc 'fn extract_envelope' src/ == 0"
        status: pass
    human_judgment: false
  - id: D6
    description: "FinishOnPlainAnswerMiddleware finishes the run with StopReason::Completed on a response carrying no tool call; without it the loop's existing MaxLoops behavior is unchanged; ADR-0042 stays untouched (no LlmRequest.tools, no adapter file modified, MockLlmAdapter capabilities stay false, the correspondence test still passes)"
    requirement: "RT-07"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/middleware/tool_protocol.rs#tests::finish_on_plain_answer_completes_the_run, ::without_the_middleware_the_loop_still_runs_to_max_loops, ::no_adapter_or_capability_changed"
        status: pass
      - kind: unit
        ref: "crates/paladin-llm/src/lib.rs#capability_invariants::test_capabilities_tool_calling_matches_request_surface"
        status: pass
      - kind: other
        ref: "git diff --name-only HEAD~1 -- crates/paladin-llm/src crates/paladin-ports/src/output/llm_port.rs | wc -l == 0"
        status: pass
    human_judgment: false
  - id: D7
    description: "Full workspace gates stay green: cargo check --workspace --all-targets --all-features, cargo fmt --all --check, cargo clippy --workspace --all-targets --all-features -- -D warnings, cargo test -p paladin-ai --lib (713/713), cargo test -p paladin-ai --doc (128/128, 18 ignored pre-existing), cargo test -p paladin-llm --lib redaction (10/10), PaladinConfig untouched"
    requirement: "RT-07"
    verification:
      - kind: other
        ref: "cargo check --workspace --all-targets --all-features (exit 0); cargo fmt --all --check (exit 0); cargo clippy --workspace --all-targets --all-features -- -D warnings (exit 0); git diff --name-only HEAD -- crates/paladin-core/src/platform/container/paladin_config.rs | wc -l == 0"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ai --lib (713 passed, 0 failed); cargo test -p paladin-ai --doc (128 passed, 0 failed, 18 ignored -- pre-existing); cargo test -p paladin-llm --lib redaction (10 passed, 0 failed)"
        status: pass
    human_judgment: false

duration: ~45min
completed: 2026-09-07
status: complete
---

# Phase 26 Plan 19: Tool-Error Policy and the Prompt-Level Tool-Call Protocol Summary

**Tool failures are fed back through one shared, redact-then-bound-sanitized formatter with a `FailRun` structured-error opt-in and per-tool override, `MIGRATION.md`'s M-B-03 now says what actually changed (nothing, apart from sanitization), and a prompt-level `ToolCallProtocolMiddleware`/`FinishOnPlainAnswerMiddleware` pair makes a shipped LLM provider able to call a tool at all -- without touching a single adapter, request field, or capability flag.**

## Performance

- **Duration:** ~45 min
- **Started:** 2026-09-07T13:07:09Z (approx., first commit after the prior wave's merge)
- **Completed:** 2026-09-07T13:49:20Z
- **Tasks:** 3 (Task 1 checkpoint auto-resolved; Task 2 and Task 3 executed)
- **Files modified:** 7 (1 created, 6 modified)

## Checkpoint resolutions

Task 1 (`checkpoint:decision`, `gate="blocking"`) was pre-resolved by the orchestrator under auto-mode: **auto-selected `ship-the-protocol`** — ship the prompt-level tool-call protocol exactly as D-36 specifies (`ToolCallProtocolMiddleware` and `FinishOnPlainAnswerMiddleware` as opt-in built-ins, inert for anyone who does not install them). No further decision was required from this execution; Task 3 proceeded directly against that resolution.

## Accomplishments

- `paladin_llm::redaction::redact_secret_patterns` (`crates/paladin-llm/src/redaction.rs`): the key-less pattern half of `redact_credentials`, factored out (D-34). Covers `Bearer`/`bearer` headers, `sk-`-prefixed keys (which also covers `sk-ant-`), `AKIA`-prefixed AWS key IDs, `key=`/`token=` query values, and JWT-shaped `header.payload.signature` triples via a hand-written base64url run-scanner (no new dependency, char-boundary-safe). `redact_credentials` now calls it plus its own exact-key pass — its pre-existing tests pass unmodified.
- `ToolResultFormatter::format_error(&ArmamentCall, &str)` (`src/infrastructure/adapters/arsenal/tool_result_formatter.rs`): keeps today's `🔧 Tool Execution … FAILED` shape, appends the PRD's "retry with corrected arguments or proceed without it" sentence, and sanitizes the reason via `redact_secret_patterns` **then** `bounded_excerpt` — redact-then-bound, at the call site, with the ordering rationale as an inline comment (T-26-03).
- `PaladinError::ArmamentFailed { tool, reason }` (`crates/paladin-core/src/platform/container/paladin_error.rs`): the new `FailRun` opt-in, free under the pre-existing `#[non_exhaustive]` attribute, classified `Transience::Unknown`.
- `PaladinExecutionService::with_tool_error_config` / `effective_tool_error_mode` (`paladin_execution_service.rs`): service-level `ToolErrorConfig` (from plan 26-02), with a `per_tool` override beating the global `mode`. **Both** the Arsenal tool-outcome error arm and the handoff execute-error arm now route through `format_error` and branch on the effective mode — `FeedToModel` appends the sanitized text exactly where today's inline `format!` used to; `FailRun` returns `PaladinError::ArmamentFailed`. `PaladinConfig` is untouched (confirmed by `git diff`).
- `MIGRATION.md` §9.1 M-B-03 rewritten: **no behavioral change** — the v0.9 loop already fed tool failures back and continued; v0.10 names the policy (`tool_error_mode`), adds `FailRun`, and sanitizes the fed-back text, with a concrete before/after example showing a `Bearer sk-live-...` token becoming `Bearer [REDACTED]`. The TBD bullet is cleared with the same worked example.
- `src/application/services/paladin/middleware/tool_protocol.rs` (new): `ToolCallProtocolMiddleware` renders the arsenal's `list_armaments()` catalogue (name, description, JSON Schema parameters) plus a documented `{"tool": "<name>", "arguments": {..}}` envelope format into a `## Tools` prompt section on every iteration (no section at all for an empty arsenal). Its `after_model` acts only when `function_call` is `None`, decodes the envelope via the shared `extract_json` (bare or fenced), and synthesizes `function_call` only when the named tool is known to the arsenal — never overwriting a real adapter's or consumer's own call, and leaving non-envelope or unknown-tool responses untouched so the model's text survives. `FinishOnPlainAnswerMiddleware` is a separate, independent opt-in that finishes the run the moment a response carries no tool call; without it the loop's existing `MaxLoops` behavior is byte-for-byte unchanged. ADR-0042's boundary is untouched: no `LlmRequest.tools`, no adapter file modified, `MockLlmAdapter`'s capabilities stay `false`, and `test_capabilities_tool_calling_matches_request_surface` still passes.

## Task Commits

1. **Task 2: Tool-error policy — the formatter, the redaction split, FailRun, and the corrected M-B-03** — `3624527a` (feat)
2. **Task 3: The prompt-level tool-call protocol and FinishOnPlainAnswer** — `b856655e` (feat)

Task 1's `checkpoint:decision` was auto-resolved by the orchestrator and produced no commit of its own.

Both task commits combine their RED and GREEN cycle into a single `feat` commit, not separate `test(...)`-then-`feat(...)` commits: the types under test (`redact_secret_patterns`, `format_error`, `PaladinError::ArmamentFailed`, `with_tool_error_config`, `ToolCallProtocolMiddleware`, `FinishOnPlainAnswerMiddleware`) did not exist anywhere in the tree before each commit, so a genuinely-compiling RED state was impossible — the same precedent already recorded in `26-02-SUMMARY.md`, `26-15-SUMMARY.md`, and `26-16-SUMMARY.md` for this phase. Every named test was written and run to green before its commit landed; the fact-finding (e.g., a self-referential test bug caught below) happened before the commit, not after.

## Files Created/Modified

- `crates/paladin-llm/src/redaction.rs` — `redact_secret_patterns`, `redact_jwt_triples`, `is_b64url_char`, `b64url_run_end`, `JWT_MIN_SEGMENT_LEN`; `redact_credentials` now composes the new function; 3 new tests
- `crates/paladin-core/src/platform/container/paladin_error.rs` — `PaladinError::ArmamentFailed { tool, reason }`, its `Transience::Unknown` classification, a new transience-table test row
- `src/infrastructure/adapters/arsenal/tool_result_formatter.rs` — `ToolResultFormatter::format_error`, 3 new tests
- `src/application/services/paladin/paladin_execution_service.rs` — `tool_error_config` field, `with_tool_error_config`, `effective_tool_error_mode`; both tool-error arms rewritten to route through `format_error`/`ArmamentFailed`; a `FailingArsenal` test double and 5 new tests
- `src/application/services/paladin/middleware/tool_protocol.rs` (new) — `ToolCallProtocolMiddleware`, `FinishOnPlainAnswerMiddleware`, `render_catalogue`, `decode_envelope`, 9 tests
- `src/application/services/paladin/middleware/mod.rs` — registers and re-exports `tool_protocol`
- `MIGRATION.md` — §9.1 M-B-03 row and worked-example bullet rewritten

## Decisions Made

See `key-decisions` in the frontmatter for full reasoning. In short: `ArmamentFailed`'s second field is `reason`, not `source` (thiserror's implicit `Error::source()` magic on a field literally named `source`); the handoff arm's fed-back text is deliberately unified onto the Arsenal arm's `🔧 Tool Execution` shape (D-34's explicit "one shared formatter" directive, proven by `both_arms_use_the_same_formatter`); `redact_secret_patterns`'s JWT detector is hand-written (no new `regex` dependency in `paladin-llm`, matching the plan's "reuse the file's char-boundary-safe approach"); and the tool catalogue renders at `SectionPlacement::End` (the plan left wording/placement at the implementer's discretion, D-36).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] A self-referential test asserted against its own source text via `include_str!`**
- **Found during:** Task 3, first `cargo test` run of `tool_protocol::tests`
- **Issue:** A planned source-level check ("no second envelope-extraction function exists in this file") was first written as a Rust `#[test]` using `include_str!("tool_protocol.rs")` to search for the literal `"fn extract_envelope"`. Since `include_str!` embeds the WHOLE file's raw text — including the test's own source line containing that literal string — the search matched itself (`left: 1, right: 0`), a self-referential bug caught by the test's own first run.
- **Fix:** Removed the Rust unit test; the same invariant is verified at the shell level per the plan's own acceptance criteria (`grep -rc 'fn extract_envelope' src/`), documented in a code comment explaining why a `#[test]` version cannot work. A second occurrence of this same self-reference (this time in the comment explaining the fix) was caught by re-running the actual shell grep before committing, and reworded to avoid embedding the literal.
- **Files modified:** `src/application/services/paladin/middleware/tool_protocol.rs`
- **Verification:** `grep -rc 'fn extract_envelope' src/` returns `0`; `cargo test -p paladin-ai --lib tool_protocol::` (9/9 pass)
- **Committed in:** `b856655e` (Task 3 commit)

**2. [Rule 3 - Blocking] `clippy::useless_format` on a single-argument `format!` call in a test**
- **Found during:** Task 3, `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- **Issue:** `a_documented_envelope_synthesizes_a_function_call`'s fenced-JSON test case used `format!("...")` with no interpolation arguments.
- **Fix:** Replaced with a plain string literal plus `.to_string()`.
- **Files modified:** `src/application/services/paladin/middleware/tool_protocol.rs`
- **Verification:** `cargo clippy --workspace --all-targets --all-features -- -D warnings` exits `0`
- **Committed in:** `b856655e` (Task 3 commit)

---

**Total deviations:** 2 auto-fixed (1 self-referential-test bug caught by the test's own first run, 1 clippy-driven blocking fix). Neither touched planned production behavior.
**Impact on plan:** No scope creep. Both are exactly the kind of "found during execution, fixed inline" issue the deviation rules exist for.

## Issues Encountered

None beyond the two documented deviations above. Every acceptance criterion in the plan (struct/function existence, the redact-then-bound line-ordering grep, `ArmamentFailed` presence, `with_tool_error_config` presence, `PaladinConfig` untouched, the M-B-03 TBD/premise greps, the shared-extractor and no-second-parser greps, the ADR-0042 adapter/request-surface diff, all fourteen plan-named test functions, `cargo fmt`/`cargo clippy -- -D warnings`) was verified directly against this worktree.

## Known Stubs

None. Both tasks are fully implemented per the plan's `<action>`/`<done>` clauses, with real (not placeholder) tests for every `<behavior>` item.

## Threat Flags

None beyond what the plan's own `<threat_model>` already covers (T-26-03, T-26-10, T-26-61, T-26-26, T-26-62) — no new network endpoint, auth path, file-access pattern, or schema change at a trust boundary was introduced.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- `ToolErrorConfig`/`ToolErrorMode` (plan 26-02) now have a real consumer: `PaladinExecutionService::with_tool_error_config`. A future preset (`reasoning_agent`, D-35, later in this phase's wave sequence) can pass its own `ToolErrorConfig` straight through.
- `ToolCallProtocolMiddleware`/`FinishOnPlainAnswerMiddleware` are locked in their final public shape at `src/application/services/paladin/middleware/tool_protocol.rs`, re-exported from `middleware::mod`. A later plan wiring `AgentRuntimeConfig::build_chain` or the `reasoning_agent` preset can install both directly — ordering them `[ToolCallProtocolMiddleware, ..., FinishOnPlainAnswerMiddleware]` so a synthesized `function_call` is visible to `FinishOnPlainAnswerMiddleware`'s own check within the same `after_model` chain pass.
- `redact_secret_patterns` is available to any future formatter or diagnostic path that needs pattern-only redaction with no configured key (e.g., a future built-in tool's own error text).
- No blockers. `.planning/STATE.md` / `.planning/ROADMAP.md` / `.planning/REQUIREMENTS.md` are NOT updated by this worktree-mode executor — the orchestrator owns those writes after all wave 12 worktree agents complete.

---
*Phase: 26-agent-runtime-enhancements*
*Completed: 2026-09-07*

## Self-Check: PASSED

Verified on disk / in git history:
- `crates/paladin-llm/src/redaction.rs` — FOUND, contains `pub fn redact_secret_patterns(`
- `src/infrastructure/adapters/arsenal/tool_result_formatter.rs` — FOUND, contains `pub fn format_error(`
- `crates/paladin-core/src/platform/container/paladin_error.rs` — FOUND, contains `ArmamentFailed`
- `src/application/services/paladin/paladin_execution_service.rs` — FOUND, contains `pub fn with_tool_error_config(`
- `src/application/services/paladin/middleware/tool_protocol.rs` — FOUND, contains `pub struct ToolCallProtocolMiddleware` and `pub struct FinishOnPlainAnswerMiddleware`
- `src/application/services/paladin/middleware/mod.rs` — FOUND, re-exports both new types
- `MIGRATION.md` — FOUND, M-B-03 carries no `TBD` and states "no behavioral change"
- Commit `3624527a` — FOUND in `git log --oneline`
- Commit `b856655e` — FOUND in `git log --oneline`
- `cargo test -p paladin-llm --lib redaction` — 10 passed, 0 failed
- `cargo test -p paladin-ai --lib` (full suite) — 713 passed, 0 failed
- `cargo test -p paladin-ai --doc` — 128 passed, 0 failed, 18 ignored (pre-existing)
- `cargo test -p paladin-llm --all-features --lib test_capabilities_tool_calling_matches_request_surface` — 1 passed
- `cargo check --workspace --all-targets --all-features` — exit 0
- `cargo fmt --all --check` — exit 0
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — exit 0
- `git diff --name-only HEAD~2 -- crates/paladin-core/src/platform/container/paladin_config.rs | wc -l` — 0 (unchanged)
- `git diff --name-only HEAD~1 -- crates/paladin-llm/src crates/paladin-ports/src/output/llm_port.rs | wc -l` — 0 (unchanged)
