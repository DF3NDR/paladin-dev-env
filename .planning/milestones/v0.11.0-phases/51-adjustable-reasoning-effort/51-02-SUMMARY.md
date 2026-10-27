---
phase: 51-adjustable-reasoning-effort
plan: 02
subsystem: llm-live-harness
tags: [reasoning-effort, live-vendor-smoke, effort-probe, dry-run, candidates]
status: complete

requires:
  - phase: 51-01
    provides: "ReasoningEffort, LlmRequest::with_reasoning, LlmError::UnsupportedParameter, EffortSupport on ModelProfile, the paladin.reasoning_* metadata keys"
provides:
  - "live_vendor_smoke --effort: raw-wire effort probe for all nine providers (numbers and verdicts only), with --dry-run"
  - "live_vendor_smoke --effort-smoke: two adapter-path requests per candidate at the lowest and highest admitted level, with --max-tokens and --timeout-seconds and --dry-run"
  - "51-EFFORT-PROBE-CANDIDATES.md: the D-02 62-id table, owner questions Q1-Q3, commands, spend estimate and the harness's offline dry-run plan"
affects: [51-10, 51-11, 51-12, 51-14]

tech-stack:
  added: []
  patterns:
    - "plan-as-data: each vendor's effort calls are an ordered Vec<EffortCall> (form, value, role, JSON body extra) deep-merged into one base body, so dry-run, tests and live run share one source"
    - "pure row builders (effort_row_from_reply, smoke_model over &dyn LlmPort) so the printed rows are unit-tested without a network"

key-files:
  created:
    - .planning/phases/51-adjustable-reasoning-effort/51-EFFORT-PROBE-CANDIDATES.md
  modified:
    - crates/paladin-llm/examples/live_vendor_smoke.rs

key-decisions:
  - "Verdict grammar: accepted is a 2xx, rejected is only HTTP 400 or 422, everything else (401, 403, 429, 5xx, 3xx, transport) is error. An auth failure or a rate limit must not read as 'the field is parsed'."
  - "The silent-field detector is the top-level reasoning_effort: bogus-effort-51 call. Where the vendor's candidate form already is that field (Kimi, OpenAI, DeepSeek, Grok, the self-hosted plan) its invalid call is the detector; Qwen has a separate 'detector' call. The summary prints 'detector: field not parsed (silent)' on a 2xx."
  - "--effort-smoke --dry-run states the two calls symbolically ('lowest and highest admitted level, read at run time'); it cannot name the levels because that needs a built adapter, which needs a credential."
  - "A model whose model_profile(model).effort is not Supported (or has no levels, or no profile) sends nothing and the row says why; a Supported profile with a single level sends one request, not the same level twice."

patterns-established:
  - "Effort modes probe only --models ids: no curated or default model is ever added, so spend equals the confirmed candidate set."

requirements-completed: []
requirements-progress: [REASON-03, REASON-04]

duration: about 3h (including a cold example build)
completed: 2026-10-21
---

# Phase 51 Plan 02: Effort probe and adapter-path effort smoke Summary

**`live_vendor_smoke --effort` sends every effort level through each candidate wire form of all nine providers over the raw wire, with invalid-value, off/auto, budget-bound, sampling and silent-field-detector rows, numbers and verdicts only; `--effort-smoke` does the same through the shipped adapters; both plan offline under `--dry-run`, and the owner has the costed 62-id candidate list.**

REASON-03 and REASON-04 are advanced, not marked complete. The harness is ready, but the credentialed live run that proves each field is parsed and that a higher level reports more reasoning tokens is plan 51-10 (after the owner's go-ahead), and the seeded effort data is plans 51-11 and 51-12. `requirements-completed` is left empty and REQUIREMENTS.md is untouched.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Tracer: `--effort` for Kimi end to end offline | `2a59fd0d` |
| 2 | Every vendor's candidate forms, detector, budget and sampling rows, self-hosted targets, adapter-path `--effort-smoke` | `af968c01` |
| 3 | The D-02 candidate file with owner questions, commands and the offline dry-run | `c372bab0` |

The Task 1 tracer was verified by the user at the interactive checkpoint ("Approved") before Tasks 2 and 3 began.

## What was built

All code is in `crates/paladin-llm/examples/live_vendor_smoke.rs` (a development example: no library surface changed).

- **Flags.** `--effort`, `--effort-smoke`, `--max-tokens <n>`, `--timeout-seconds <s>`. The four modes `--probe`, `--usage`, `--effort`, `--effort-smoke` are mutually exclusive and the error names both flags; the caps are accepted only with `--effort-smoke` (positive integers); `--models` without a mode or `--dry-run` is still an error; unknown flags are refused by name. USAGE, `--help` and the module docs describe both modes.
- **Constants.** `EFFORT_MAX_TOKENS: u64 = 2048`, `EFFORT_INVALID_VALUE = "bogus-effort-51"`; the existing `USAGE_PROMPT` and the 120 s `PROBE_TIMEOUT_SECS` are reused. `ProbeKind::Effort` builds the base body (usage prompt, 2048 cap, no acceptance extra, no sampling).
- **Plans (calls per model).** Kimi 18 (both `top_level_effort` and `thinking_object` for minimal..max, off, auto, an invalid value per form, high + temperature); OpenAI 10 (`none` as the off form, seven levels, invalid, sampling); DeepSeek 12 (levels, `thinking` off/auto, invalid through `reasoning_effort` and `thinking.type`, sampling); Grok 9; Qwen 8 (`enable_thinking` false/true, `thinking_budget` 1 and 0, invalid -1, detector, sampling); Anthropic 17 (each of low..max under `output_config` alone and `output_config+adaptive`, off, adaptive, invalid, `budget_tokens` 1024 and 1023, sampling); Gemini 12 (lowercase `thinkingLevel` minimal..high plus uppercase `HIGH`, `thinkingBudget` 0/128/1, invalid level and `-5`, sampling inside `generationConfig`); Ollama and OpenAI-compatible (the Kimi forms plus `enable_thinking`/`thinking_budget`, invalid per form, sampling).
- **Rows and summary.** `EffortRow` (form, value, status, finish, reasoning tokens, completion tokens, ms, verdict, bounded excerpt) and per-form summary lines: `field parsed: yes|no|unknown`, `effect: low=<n> <top>=<n>` (or `budget: v=n ...`), and `detector: field not parsed (silent)` / `field parsed (rejected)`.
- **Smoke.** `build_smoke_port` builds each adapter from `*Config::from_env()` with `with_max_tokens` / `with_timeout_seconds` applied when given; `smoke_model` reads `model_profile(model).effort`, sends `LlmRequest::with_reasoning` at the lowest and highest listed level, and prints outcome (`Ok`, `UnsupportedParameter: <reason>`, `error: <excerpt>`, `not sent: <why>`), finish, reasoning tokens and the two `paladin.reasoning_*` metadata values.
- **Candidate file.** `51-EFFORT-PROBE-CANDIDATES.md`: the 62-row table verbatim from the plan, the exclusions, owner questions Q1 (additions), Q2 (D-02 `NoReasoning` against `Unmeasured` for unprobed `reasoning: false` entries, with the Pitfall 1 evidence) and Q3 (where the probe runs), the per-vendor `--models` flags, one combined live command and the per-vendor alternative, a spend estimate, and the full stdout of the combined command with `--dry-run` (every vendor key removed from the environment, exit 0).

## Spend estimate (from the harness's own plan)

710 raw calls over 62 models (the plan estimated about 760): Anthropic 11 x 17, OpenAI 17 x 10, DeepSeek 2 x 12, Kimi 2 x 18, Qwen 13 x 8, Grok 5 x 9, Gemini 12 x 12. At the 2,048-token cap the ceiling is 1,454,080 output tokens; real use is lower because a rejected call costs no output.

## TDD record

- **Task 1:** RED was not run separately. The implementation and its 13 tests were written in one pass, then run (46 passed). The only failure seen was a test miscount (17 planned Kimi calls written, 18 correct), fixed in the test. Disclosed at the checkpoint and accepted.
- **Task 2 stage A (plans), RED then GREEN.** With the plans for the other eight providers still empty, 11 tests failed: `a_self_hosted_target_is_probed_through_all_four_forms`, `budget_forms_report_their_reasoning_tokens_in_the_summary`, `every_openai_chat_vendor_gets_the_silent_field_detector`, `every_provider_has_a_plan_and_every_body_is_capped_and_free_of_the_acceptance_extra`, `the_anthropic_plan_probes_each_level_alone_and_paired_with_adaptive_thinking`, `the_deepseek_plan_probes_levels_off_auto_and_both_invalid_forms`, `the_gemini_plan_probes_lowercase_levels_one_uppercase_variant_and_budgets`, `the_grok_plan_probes_each_level_an_invalid_value_and_the_sampling_row`, `the_openai_plan_sends_none_through_max_an_invalid_value_and_the_sampling_row`, `the_qwen_plan_probes_the_toggle_the_budget_bounds_and_a_detector`, `the_summary_adds_a_detector_line_and_prints_the_silent_phrase_on_a_2xx` (46 passed, 11 failed). After the plans and summary changes: 57 passed.
- **Task 2 stage B (smoke), RED then GREEN.** With only the tests written, the example's test build failed with 14 compile errors (`SmokeKnobs`, `build_smoke_port`, `smoke_model`, `smoke_row_line`, `SmokeRow`, `render_effort_smoke_dry_run`, the two metadata-key imports, `Options.max_tokens`, `Options.timeout_seconds`). After the implementation: 67 passed, 0 failed.

## Verification

- `cargo test --offline -p paladin-llm --example live_vendor_smoke --features <nine>`: 67 passed.
- Task 1 verify: the same test run, then the offline `--effort --dry-run --models kimi=kimi-k3` with all seven keys removed prints `thinking_object`; `grep -n 'EFFORT_MAX_TOKENS: u64 = 2048'` prints one line.
- Task 2 acceptance: `--help` names `--effort`, `--effort-smoke`, `--max-tokens`, `--timeout-seconds`; `--effort-smoke --dry-run --models deepseek=deepseek-v4-pro` with no key set exits 0 and lists 2 adapter calls.
- Task 3 verify (the plan's command, run from a script file because the sandbox refuses it as one compound command): 62 ids extracted from the plan, all 62 present in the dry-run section, `## Dry-run plan (offline)`, `## Owner questions`, `## Spend estimate` present, exit 0.
- `cargo fmt --check -p paladin-llm` clean; `cargo clippy --offline -p paladin-llm --example live_vendor_smoke --features <nine> -- -D warnings` clean (it does not compile the `cfg(test)` module; the test run printed no warnings).
- No network access happened: every run was a unit test, a `--dry-run`, or the offline build. No live vendor call was made and no key value was read into output.
- Not run: the full `make clean-code` chain, `make security`, coverage. The orchestrator's post-wave hook chain covers them; no dependency was added.

## Manual credential-handling review (security.instructions.md)

- Printed fields: form, value label, status, finish, token integers, elapsed ms, verdict. A non-2xx body and every transport error pass `excerpt()` (`diagnostic_excerpt` redacts the key, then bounds, then the cell is made table-safe). A 2xx finish string and the two metadata values also pass `excerpt()`. A finish reason from the adapter is mapped to a fixed label. Response content is never copied into a row; tests plant `SECRET-ANSWER-TEXT`, a reasoning string and key-shaped strings and assert none is printed (`a_canned_kimi_200_...`, `a_canned_kimi_400_...`, `an_ok_row_shows_...`, `a_key_shaped_string_in_an_error_never_reaches_a_printed_smoke_row`).
- No request header is printed (only the header name, as the existing modes do). The client is the existing no-redirect client with the 120 s timeout. `--dry-run` builds no adapter and reads no variable. The effort modes probe only `--models` ids.
- The adapter-path smoke redacts using the first credential variable of the provider (for Ollama this is a URL variable, harmless); the adapters have already routed their own errors through the redact-then-bound excerpt.

## Deviations from Plan

### Auto-fixed Issues

None needing a rule.

### Notes against the plan's wording (disclosed at the tracer checkpoint, accepted)

- **RED not run separately for Task 1** (see TDD record).
- **JSON key order in printed bodies.** `serde_json` keeps keys sorted in this build, so the dry-run prints `{"thinking":{"effort":"low","type":"enabled"}}`; the plan wrote `type` first. The JSON is identical and the tests compare parsed values.
- **Providers other than Kimi printed "no effort plan" after Task 1**; Task 2 filled them in.

### Other notes

- **Task 1 test amended in Task 2.** `the_form_summary_reports_parsed_and_effect_from_the_rows` now expects a third line, `- detector: field not parsed (silent)`, because a top-level invalid value doubles as the detector. This is the Task 2 behaviour, not a regression.
- **Call counts below the planner's estimate** (710 against about 760; Qwen 8 against about 9, Anthropic 17 against about 18, Gemini 12 against about 14) because each plan is exactly the list in the plan's behaviour block.
- **The dry-run prints each plan once per provider** with the per-candidate call counts above it (the `--probe` dry-run precedent), not once per candidate model, since the plan is identical for every model of a provider.
- **The self-hosted plan reuses the Kimi plan** (both `top_level_effort` and `thinking_object` across all six levels) and adds `enable_thinking` off/auto, a `thinking_budget` 1024 and an invalid budget -1, with the sampling row on the top-level field.
- **Today the adapter smoke sends nothing for most models.** Every seeded model still reads `EffortSupport::Unmeasured` until plans 51-11 and 51-12 seed effort data, so `--effort-smoke` prints `not sent: effort unmeasured` and exits non-zero for a provider that sent nothing (the vacuous-pass refusal). That is the intended behaviour until the data lands; the unit tests cover the sending path with an adapter double.
- **The sandbox refused compound `env -u ... && ...` commands**, so the dry-runs were run as single commands, and the Task 3 file was produced by a script that removes the keys from a child environment and asserts exit 0.

## Authentication Gates

None. No live call was made.

## Known Stubs

None.

## Threat Flags

None. The harness already made credentialed raw calls in `--probe` and `--usage`; the new modes add no endpoint or file access beyond that. T-51-06 (printed key) is mitigated by the redact-then-bound excerpt and the planted-string tests; T-51-07 (unbounded spend) by the 2,048-token cap, the 120 s timeout, `--models`-only candidates, the dry-run and the spend estimate; T-51-08 is accepted as planned.

## Open items for the owner

- Q1, Q2 and Q3 in `51-EFFORT-PROBE-CANDIDATES.md`, at plan 51-10's checkpoint, before any spend.

## Self-Check: PASSED

- Files present: `crates/paladin-llm/examples/live_vendor_smoke.rs`, `.planning/phases/51-adjustable-reasoning-effort/51-EFFORT-PROBE-CANDIDATES.md`, this SUMMARY.
- Commits `2a59fd0d`, `af968c01`, `c372bab0` present on `worktree-agent-a35f8ca25c0069d9c`.
