---
phase: 50
slug: provider-parity-config-driven-model-swap
plan: 01
status: recorded
created: 2026-10-15
---

# Phase 50 — Provider-swap matrix: red-before-fix evidence

> PARITY-01 and ROADMAP Phase 50 success criterion 1 require the provider-swap matrix
> (`tests/provider_swap_matrix.rs`) to be red before the fixes land. A matrix that cannot fail proves
> nothing. This file records (1) the pre-fix failure of the two OpenAI B-4 tests, run on the tree before
> the fix, and (2) the full `--ignored` run of the matrix, case by case, with the plan that fixes each
> red case. A red case is un-ignored only by the plan named here, and its assertion is never weakened.

Base commit (`git rev-parse HEAD` when the matrix was measured): `0206d031c677d4b47a70b6d5abb84dd5bc12703b`
(the Task 1 and Task 2 commits of plan 50-01 are in this base: the user-turn fix is already applied, so
the user-turn cases are green; the matrix file itself is committed immediately after this measurement).

Toolchain: `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1`.

## Commands

```bash
# Non-ignored cases -- green at the plan's last commit
cargo test --test provider_swap_matrix --features llm-all
# The red cases, run once, output recorded below
RUST_BACKTRACE=0 cargo test --test provider_swap_matrix --features llm-all -- --ignored
```

Result of the second command: `test result: FAILED. 0 passed; 19 failed; 0 ignored; 0 measured; 30 filtered out`.
Result of the first: `test result: ok. 30 passed; 0 failed; 19 ignored`.

## Part 1 — The B-4 defect, before the fix (OpenAI)

Run on the tree before the shared helper existed (OpenAI adapter's `PromptType::User` arm read
`user_prompt.context.clone().unwrap_or_default()`).

End to end (`openai_prompt_reaches_the_wire`, config block -> `OpenAIAdapter` -> `PaladinBuilder` ->
`PaladinExecutionService::execute` -> mock transport):

```text
thread 'openai_prompt_reaches_the_wire' panicked at tests/provider_swap_matrix.rs:196:5:
openai: the query must reach the wire inside the last user message; body was
{"max_tokens":4096,"messages":[{"content":"","role":"user"}],"model":"matrix-openai-model","stream":false,"temperature":0.7,"top_p":1.0}
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

The first draft of that assertion (`content == MATRIX_QUERY`) failed the same way (`left: Some("")`); it was
corrected to `content.contains(MATRIX_QUERY)` because the execution service folds the system prompt and the
query into the one string it hands the adapter as `UserPrompt::query` (see the plan SUMMARY, deviations).
The corrected assertion was re-run red against the unfixed adapter (output above).

Adapter level (`openai_user_turn_carries_query_on_the_execution_service_path`, a `UserPrompt { query,
context: None }` exactly as the execution service builds it):

```text
thread 'openai::adapter::tests::user_turn_wiring::openai_user_turn_carries_query_on_the_execution_service_path'
panicked at crates/paladin-llm/src/openai/adapter.rs:1740:13:
assertion `left == right` failed
  left: ""
 right: "What is 2+2?"
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 237 filtered out
```

The four sibling request builders' `*_user_turn_appends_context` tests were also written first and run red
(`0 passed; 4 failed`) before Task 2 switched them onto `crate::turn::user_turn_text`.

## Part 2 — The matrix, case by case

Case letters follow the plan: (a) prompt on the wire, (b) same agent equivalent or refused by name,
(c) fixed temperature, explicit (Kimi), (d) fixed temperature, auto (Kimi), (e) empty stop completion typed,
(f) fallback hop gets its own model. Three further all-nine cases are green and un-ignored:
`*_model_reaches_the_wire` and `*_context_reaches_the_wire` (the shared user-turn rule, PARITY-08), plus
`matrix_covers_every_provider`.

### Red cases (19, every one `#[ignore = "red until plan 50-NN: ..."]`)

| Test | Status | Fixing plan | Assertion message on the current tree |
|------|--------|-------------|---------------------------------------|
| `openai_same_agent_is_equivalent_or_refused_by_name` | red | 50-05 | `openai: the agent was built with stop words [END, STOP] but the wire carries None` (body has `temperature: 0.5`, no `stop`) |
| `anthropic_same_agent_is_equivalent_or_refused_by_name` | red | 50-05 | `anthropic: the agent was built with temperature 0.5 but the wire carries None` (body has no `temperature`, no `stop_sequences`) |
| `deepseek_same_agent_is_equivalent_or_refused_by_name` | red | 50-05 | `deepseek: the agent was built with stop words [END, STOP] but the wire carries None` |
| `kimi_same_agent_is_equivalent_or_refused_by_name` | red | 50-03 | `kimi: a refusal must name the provider, the model 'kimi-k3' and the parameter (temperature or stop); it said: temperature must be between 1 and 1 for provider 'kimi', got 0.5` |
| `qwen_same_agent_is_equivalent_or_refused_by_name` | red | 50-03 | `qwen: the agent was built with stop words [END, STOP] but the wire carries None` |
| `grok_same_agent_is_equivalent_or_refused_by_name` | red | 50-03 | `grok: the agent was built with stop words [END, STOP] but the wire carries None` |
| `ollama_same_agent_is_equivalent_or_refused_by_name` | red | 50-03 | `ollama: the agent was built with stop words [END, STOP] but the wire carries None` |
| `openai_compatible_same_agent_is_equivalent_or_refused_by_name` | red | 50-03 | `openai-compatible: the agent was built with stop words [END, STOP] but the wire carries None` |
| `kimi_fixed_temperature_explicit` | red | 50-03 | `kimi: an explicit temperature 1.0 must be on the wire; body was {"messages":[...],"model":"kimi-k3","stream":false}` (left `None`, right `Some(1.0)`) |
| `kimi_fixed_temperature_auto` | red | 50-03 | `kimi: an auto-temperature agent must build: Configuration error: auto-selected temperature 0.6 is outside the range 1 to 1 declared by provider 'kimi'. ...` |
| `openai_empty_stop_completion_is_typed` | red | 50-12 | `openai: an empty stop completion was returned as a successful answer: ""` |
| `deepseek_empty_stop_completion_is_typed` | red | 50-12 | `deepseek: an empty stop completion was returned as a successful answer: ""` |
| `kimi_empty_stop_completion_is_typed` | red | 50-12 | `kimi: an empty stop completion was returned as a successful answer: ""` |
| `qwen_empty_stop_completion_is_typed` | red | 50-12 | `qwen: an empty stop completion was returned as a successful answer: ""` |
| `grok_empty_stop_completion_is_typed` | red | 50-12 | `grok: an empty stop completion was returned as a successful answer: ""` |
| `ollama_empty_stop_completion_is_typed` | red | 50-12 | `ollama: an empty stop completion was returned as a successful answer: ""` |
| `openai_compatible_empty_stop_completion_is_typed` | red | 50-12 | `openai-compatible: an empty stop completion was returned as a successful answer: ""` |
| `gemini_empty_stop_completion_is_typed` | red | 50-13 | `gemini: an empty stop completion was returned as a successful answer: ""` |
| `fallback_hop_gets_its_own_model` | red | 50-14 | `the second hop must be sent its own model 'kimi-k3', not the primary's; body was {"model":"matrix-anthropic-model",...}` (left `Some("matrix-anthropic-model")`) |

Notes for the fixing plans:

- `kimi_fixed_temperature_explicit` is red for two reasons the fixing plan (50-03) must settle: the compat
  engine omits `temperature` entirely for a declared `(1, 1)` provider (so an explicit `1.0` never reaches the
  wire), and the refusal of `0.5` / just-above-1.0 does not yet name the model. The assertion stands as
  written; 50-03 makes the tree satisfy it.
- The compat presets' stop-word cases (`qwen`, `grok`, `ollama`, `openai_compatible`) are assigned to 50-03
  per the plan's map; the observed failure is that the stop words are neither sent nor refused by name.

### Green cases on the current tree (30, not ignored)

| Test group | Providers | Status | Note |
|------------|-----------|--------|------|
| `*_prompt_reaches_the_wire` (a) | all nine | green | green because the Task 1 and Task 2 user-turn fixes are in the base; `openai` was red before them (Part 1) |
| `*_model_reaches_the_wire` | all nine | green | Gemini reads the model from the request path |
| `*_context_reaches_the_wire` | all nine | green | `query`, one blank line, `context` -- the shared `user_turn_text` rule |
| `gemini_same_agent_is_equivalent_or_refused_by_name` (b) | gemini | green | Gemini already sends `generationConfig.temperature` 0.5 and `stopSequences` `["END","STOP"]`; the plan's map assigned it to 50-05 but it passes today, so it is not ignored |
| `anthropic_empty_stop_completion_is_typed` (e) | anthropic | green | the Anthropic adapter already raises a permanent `Empty completion` on a normal stop with no text; the plan's map assigned it to 50-13 but it passes today, so it is not ignored |
| `matrix_covers_every_provider` | -- | green | guards the nine-row table |

Total listed: 49 tests (30 green + 19 ignored).
