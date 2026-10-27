---
phase: 51-adjustable-reasoning-effort
plan: 05
subsystem: llm-adapters
tags: [reasoning-effort, anthropic, gemini, output-config, thinking-budget, wire-masking]
status: complete

requires:
  - phase: 51-01
    provides: "ReasoningEffort, EffortSupport/EffortLevels/BudgetBounds, LlmRequest.reasoning, LlmError::UnsupportedParameter, unset-effort golden bodies"
  - phase: 51-03
    provides: "paladin_llm::reasoning (admit, WireCaps, mask_effort, SubstitutionLog, level_spelling), RequestNotes.reasoning_requested/effective, ModelOverride.effort, the DeepSeek pattern"
provides:
  - "Anthropic honours LlmRequest.reasoning per model profile: output_config.effort (plus thinking adaptive where the profile pairs them), thinking disabled/adaptive, and the legacy thinking enabled + budget_tokens only on a model profiled with budget bounds"
  - "Gemini honours LlmRequest.reasoning per model profile: generationConfig.thinkingConfig.thinkingLevel or thinkingBudget (0 for off, nothing for auto), never both"
  - "Both adapters report a model_profile() masked by their private caps constants and refuse before HTTP, on generate and generate_stream"
affects: [51-06, 51-07, 51-12, 51-13, 51-14]

tech-stack:
  added: []
  patterns:
    - "wire_profile() = registry profile masked by a private WireCaps constant, used by both build_request and model_profile()"
    - "build_request calls reasoning::admit after check_max_tokens; a private map_reasoning turns the admitted effort into the request fields"
    - "the level spelling lives in one private fn per adapter so plan 51-12 can change it in one place"

key-files:
  created: []
  modified:
    - crates/paladin-llm/src/anthropic/adapter.rs
    - crates/paladin-llm/src/gemini/adapter.rs

key-decisions:
  - "Anthropic Off sends thinking disabled and Auto sends thinking adaptive, each with no output_config; only a Budget needs the profile's budget bounds."
  - "Gemini Auto sends no thinkingConfig at all (still stamped requested/effective); Off is thinkingBudget 0; a request whose only reason for generationConfig is an effort still produces the object."
  - "The REASON-06 bounds are read only from the profile (probe-seeded or operator); no 1024 or other vendor constant is hard-coded in either adapter."

requirements-completed: []
requirements-progress: [REASON-02, REASON-03, REASON-05, REASON-06]

duration: about 2h (cold worktree build, tracer checkpoint round trip)
completed: 2026-10-21
---

# Phase 51 Plan 05: Anthropic and Gemini reasoning effort Summary

**Anthropic (`output_config.effort`, adaptive/disabled thinking, profile-gated legacy `budget_tokens`) and Gemini (`thinkingLevel` / `thinkingBudget`) now send an effort exactly as the model's profile allows or refuse before HTTP, with every unset-effort wire body still byte-identical.**

REASON-02, -03, -05 and -06 are advanced for these two adapters but not marked complete: the builder, configuration, docs and the probe-seeded model data land in plans 51-06 to 51-14, and no model carries seeded effort data yet, so only a `llm.<provider>.models.<model>.effort` override unlocks an explicit effort on these adapters for now. `requirements-completed` is empty and REQUIREMENTS.md is untouched.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Tracer: an Anthropic effort reaches the wire as `output_config.effort` (paired with adaptive thinking where the profile says so) or is refused before HTTP | `0b004bc6` |
| 2 | Anthropic off, auto and the legacy budget form, gated per profile, on both paths | `10d35cbe` |
| 3 | Gemini `thinkingLevel` / `thinkingBudget` per model profile, on both paths | `b7ec8b50` |

The Task 1 tracer was verified by the user at the interactive checkpoint ("approve") before Tasks 2 and 3 began; the orchestrator had re-run the evidence on `0b004bc6`.

## What was built

- **Anthropic** (`crates/paladin-llm/src/anthropic/adapter.rs`): `ClaudeRequest` gains two trailing private fields, `output_config` (`ClaudeOutputConfig { effort }`) and `thinking` (`ClaudeThinking { type, budget_tokens }`), both skip-if-none, so an unset body is unchanged. `ANTHROPIC_REASONING_WIRE` carries levels, off, auto and budget. `wire_profile()` masks the registry profile with it and feeds both `build_request` and `model_profile()`. `build_request` calls `reasoning::admit` with the always-set effective `max_tokens`. Private `map_reasoning`: a listed level gives `output_config.effort` plus `thinking: adaptive` when `enable_thinking_with_level`; `Off` gives `thinking: disabled`; `Auto` gives `thinking: adaptive`; `Budget(n)` gives `thinking: enabled, budget_tokens: n` only when `levels.budget.is_some()` (otherwise the defensive refusal). A per-instance `SubstitutionLog` backs the fallback warn-once. Requested/effective effort are stamped on buffered responses and on the stream's terminal chunk.
- **Gemini** (`crates/paladin-llm/src/gemini/adapter.rs`): `GeminiGenerationConfig.thinking_config` (`GeminiThinkingConfig { thinkingLevel, thinkingBudget }`), `GEMINI_REASONING_WIRE` (all four), `wire_profile()`, `admit` with the effective `maxOutputTokens`, and a private `map_reasoning` (level gives `thinkingLevel`; `Budget(n)` gives `thinkingBudget`; `Off` gives `thinkingBudget: 0`; `Auto` sends nothing; a level and a budget never together). `thinking_level_spelling` is the one place the level spelling lives (lowercase per research A2). `generationConfig` is now also emitted when only a thinking config is present.
- Neither adapter adds `display` or `includeThoughts`. `detect_no_text_content`, the thinking-block to `reasoning` mapping, `thinking_tokens` and `thoughtsTokenCount` handling are untouched.

## TDD record

- **Task 1 RED:** 7 of the 10 new tests failed against the old code (no field reached the wire, an unmeasured model was not refused, no metadata). The 3 that passed were guards for existing behaviour (unset body, profile reporting). GREEN: 141 Anthropic tests, golden 9/9.
- **Task 2 RED:** 5 positive-path tests failed (budget form, `Off`, `Auto`, stream off/budget, no-`display` loop). The refusal tests (out-of-range, no-room, bound-less model, no-`off`, no-`auto`, refused stream, thinking-only `EmptyCompletion`) passed immediately: Task 1's defensive refusal and `admit` already stop them, so they are regression guards for the prohibitions (T-51-17, T-51-18) rather than RED tests. GREEN: 155 Anthropic tests.
- **Task 3 RED:** 15 of the 18 new tests failed. GREEN: 152 Gemini tests, golden 9/9.
- Full `cargo test --offline -p paladin-llm --all-features --lib`: 1351 passed.

## Verification

- Task verify commands (`--lib anthropic`, `--lib gemini`, `unset_reasoning_leaves_the_wire_body_unchanged`): all pass. The test filter for the new modules is `anthropic::adapter::tests::reasoning_effort` and `gemini::adapter::tests::reasoning_effort`.
- `cargo clippy --offline -p paladin-llm --all-targets --all-features -- -D warnings` clean; `cargo fmt --check -p paladin-llm` clean; `make api-surface` reports the surface unchanged (4315 items).
- Not run in the worktree: the full `make clean-code` chain and `make security` (no dependency added; the orchestrator runs the hook chain over the merged wave). Commits used `--no-verify` as the orchestrator allowed.
- Manual credential-handling review (security.instructions.md): no header, client, redirect-policy or `Debug` change in either adapter; vendor 400 bodies still pass through the existing mappers and `diagnostic_excerpt`; the refusal and the `warn!` text are built from provider, model and effort values only (the shared `reasoning` module), never the prompt or the key.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Test bug] Gemini `an_effort_alone_still_produces_a_generation_config_object` assumed an empty `generationConfig`**
- **Found during:** Task 3 GREEN. The default `PromptParameters` carry `temperature`, `topP` and `maxOutputTokens`, so the object was not effort-only.
- **Fix:** the test now clears the parameters before asserting the effort-only object. No adapter change.
- **Commit:** `b7ec8b50`

### Notes against the plan's wording

- **Test placement:** the Anthropic file ends with an unrelated `knob_tests` module, so the new `mod reasoning_effort` sits at the end of `mod tests` rather than at the end of the file. Run it with the `anthropic::adapter::tests::reasoning_effort` filter.
- **Refusal tests that passed at once in Task 2** are regression guards as noted above, not RED tests; this mirrors the plan 51-03 record.
- **Streaming `Off`/`Budget` assertions** check the request body only (the scripted stream is a thinking-and-answer stream regardless of effort), which is what the plan asks; the terminal-chunk stamping and the reasoning/usage fields are asserted on the `High` stream test for each adapter.
- No CHANGELOG, baseline, MIGRATION or allowlist edit, per the plan (the surface is private); plan 51-06 records the behaviour change.
- **Commit trailer:** the system attribution reminder (`Claude Sonnet 5.5`) was used.

## Authentication Gates

None.

## Known Stubs

None. The deliberate limitation is that no Anthropic or Gemini model carries seeded effort data until plans 51-10 to 51-12, so an explicit effort on an unprofiled model refuses naming the override key. A1-51-05 (pairing `thinking: adaptive` with `output_config.effort`) and A2-51-05 (Gemini lowercase `thinkingLevel`) stay open until the probe; both are data or a one-line spelling change, and both shapes are reachable now.

## Threat Flags

None. No new network endpoint, auth path or file access. T-51-17 (deprecated budget form to a model that rejects it), T-51-18 (budget consuming the output cap) and T-51-19 (reasoning text requested beyond what the caller asked for) are mitigated and tested: zero-hit refusal tests for a bound-less model, out-of-range and no-room budgets, and body assertions that neither `display` nor `includeThoughts` is ever sent.

## Self-Check: PASSED

- Files: `crates/paladin-llm/src/anthropic/adapter.rs`, `crates/paladin-llm/src/gemini/adapter.rs` and this SUMMARY present.
- Commits `0b004bc6`, `10d35cbe`, `b7ec8b50` present in `git log` on `worktree-agent-ae8311ea9328be595`.
