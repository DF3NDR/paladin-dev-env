# Phase 51 adapter-path effort smoke (REASON-03, REASON-06)

## Provenance

- **Run by:** the plan 51-14 executor agent, in the plan 51-14 worktree, with `live_vendor_smoke --effort-smoke` built from this tree (`cargo build -p paladin-llm --example live_vendor_smoke --features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatible`, debug profile, then the built binary run directly).
- **UTC date:** 2026-10-22 (both runs, started 18:15:33Z and 18:15:58Z).
- **Commit:** `4a9c290e` (the harness prints it in each run header).
- **Owner go-ahead:** "smoke: yes", given by the owner on 2026-10-22 at the plan 51-14 checkpoint, after the orchestrator showed the `--effort-smoke --dry-run` plan (6 adapter calls for the three models, plus the documented `llm.kimi` example). Live adapter-path calls were authorised by that answer and by nothing earlier.
- **Path:** every request goes through the shipped adapter (`*Config::from_env()`, then `generate`) with `LlmRequest::with_reasoning` at the lowest and the highest level the adapter's own `model_profile(model).effort` admits. This is the adapter path; the raw-wire proof is `51-EFFORT-PROBE.md`.
- **What is recorded:** outcome, finish reason, reasoning tokens, completion tokens, the `paladin.reasoning_requested` and `paladin.reasoning_effective` response-metadata values and elapsed milliseconds. Response content is never printed by the harness (so a content length is not in the transcript; see the Kimi example below).
- **Scrub:** each run's stdout was scanned by a scratch scanner (scratchpad, not committed) before it was copied here: key shapes (vendor-prefixed secret keys, Google and xAI key shapes, bearer and authorization-style header values, JWT-shaped triples, cloud and forge token prefixes, key and token query values, long mixed-case tokens) plus the literal value of every credential variable set in the environment and of every file under the operator's `~/.config/paladin/` (26 values, compared programmatically, never printed). The scanner was shown to fire on a planted sample first. Result: `clean` on both runs. The plan's negative key-shape grep also finds nothing in this file. No key value was printed, echoed or logged; credential presence was checked with `${VAR:+set}` only.

## Verdicts

| Model | Adapter | Lowest admitted | Highest admitted | Both requests | Reasoning tokens, lowest to highest | Higher level reported more? |
|-------|---------|-----------------|------------------|---------------|-------------------------------------|-----------------------------|
| `claude-opus-4-8` | Anthropic (`output_config.effort`) | `low` | `max` | `Ok`, finish `stop`, requested equals effective | 0 to 35 | yes |
| `deepseek-v4-pro` | DeepSeek (`reasoning_effort` plus the `thinking` switch) | `low` | `max` | `Ok`, finish `stop`, requested equals effective | 70 to 77 | yes |
| `kimi-k3` | Kimi (`thinking.effort`) | `low` | `max` | `Ok`, finish `stop`, requested equals effective | run 1: 29 to 76; run 2 (documented configuration): 47 to 26 | run 1 yes; run 2 no |

- None of the three refused: every request was admitted by the model's seeded effort row, sent, and answered. No `UnsupportedParameter` row occurred, no `EmptyCompletion`, no timeout, no retry.
- `paladin.reasoning_requested` and `paladin.reasoning_effective` equal the requested spelling on all 8 rows (no fallback substituted a level, because none was configured and every level asked for is an exact entry in the seeded row).
- Reasoning-token counts on a one-line arithmetic prompt are small and noisy. The Anthropic pair (0 at `low`, 35 at `max`) and the DeepSeek pair (70 and 77) agree with the direction the 2026-10-22 raw probe saw. `kimi-k3` reported more at `max` than at `low` in run 1 (29 and 76, matching the probe's 40 and 116 in direction) and fewer in run 2 (47 and 26). The harness makes one call per level, so a single pair is evidence that the level is accepted and reaches the model, not a statistical ordering of effort; the effect claim for Kimi rests on the raw probe's multi-call table (`51-EFFORT-PROBE.md`) and this transcript's run 1, and run 2's inversion is recorded as measured rather than re-rolled.

## The documented `llm.kimi` example (REASON-06)

The Reasoning Effort page (`docs/src/user-guides/reasoning-effort.md`, `## Effort, output budget and timeout`) documents one `llm.kimi` block: `default_model: "kimi-k3"`, `reasoning_effort: low`, `max_tokens: 16000`, `timeout_seconds: 300`. Run 2 below applies that configuration through the shipped builders (`--max-tokens 16000 --timeout-seconds 300` become `with_max_tokens(16000)` and `with_timeout_seconds(300)` on the `KimiConfig`). The harness's lowest admitted level for `kimi-k3` is `low`, so its first row is the page's `reasoning_effort: low`; its second row is `max` at the same cap and timeout.

- **Outcome of the `low` row (the documented configuration): answered.** `Ok`, finish `stop`, 47 reasoning tokens and 63 completion tokens, in 4,464 ms, against a 300 s timeout and a 16,000-token cap. No `EmptyCompletion`.
- **Non-empty content:** the harness prints no content and no content length by design. An `Ok` outcome is itself the non-empty evidence, because every adapter turns empty text on any finish reason into `LlmError::EmptyCompletion` (M-B-16, M-B-17) and the row would then read `EmptyCompletion`, not `Ok`. The completion-token count (63, of which 47 are reasoning tokens) leaves 16 tokens of visible output.
- **Correction needed:** none. The page's example answered with room to spare (4.5 s against 300 s; 63 tokens against 16,000), so the page was not edited.
- The `max` row on the same configuration also answered (finish `stop`, 26 reasoning tokens, 4,027 ms).

## Transcript

Run 1: the three web3sec models at their defaults (no `--max-tokens`, no `--timeout-seconds`).
Command: `live_vendor_smoke --effort-smoke --models anthropic=claude-opus-4-8 --models deepseek=deepseek-v4-pro --models kimi=kimi-k3`. Exit 0.

```text
# Live vendor effort smoke (adapter path)

- date (UTC): 2026-10-22 18:15:33Z
- commit: 4a9c290e
- credential variables found (names only, never values): OPENAI_API_KEY, ANTHROPIC_API_KEY, DEEPSEEK_API_KEY, MOONSHOT_API_KEY, DASHSCOPE_API_KEY, XAI_API_KEY, GEMINI_API_KEY
- prompt: "What is 17 multiplied by 23? Work it out, then answer with the number only."; two requests per candidate through the shipped adapter (numbers and metadata values are recorded; response content is never printed)

## OpenAI

NOT PROBED: no `--models` candidate for this provider

## Anthropic

| model | requested | outcome | finish | reasoning tokens | completion tokens | paladin.reasoning_requested | paladin.reasoning_effective | ms |
|-------|-----------|---------|--------|------------------|-------------------|-----------------------------|-----------------------------|----|
| claude-opus-4-8 | low | Ok | stop | 0 | 3 | low | low | 800 |
| claude-opus-4-8 | max | Ok | stop | 35 | 38 | max | max | 1327 |

## DeepSeek

| model | requested | outcome | finish | reasoning tokens | completion tokens | paladin.reasoning_requested | paladin.reasoning_effective | ms |
|-------|-----------|---------|--------|------------------|-------------------|-----------------------------|-----------------------------|----|
| deepseek-v4-pro | low | Ok | stop | 70 | 72 | low | low | 1786 |
| deepseek-v4-pro | max | Ok | stop | 77 | 79 | max | max | 1754 |

## Kimi

| model | requested | outcome | finish | reasoning tokens | completion tokens | paladin.reasoning_requested | paladin.reasoning_effective | ms |
|-------|-----------|---------|--------|------------------|-------------------|-----------------------------|-----------------------------|----|
| kimi-k3 | low | Ok | stop | 29 | 45 | low | low | 2813 |
| kimi-k3 | max | Ok | stop | 76 | 92 | max | max | 3824 |

## Qwen

NOT PROBED: no `--models` candidate for this provider

## Grok

NOT PROBED: no `--models` candidate for this provider

## Gemini

NOT PROBED: no `--models` candidate for this provider

## Ollama

SKIPPED (OLLAMA_BASE_URL not set)

## OpenAI-compatible

SKIPPED (OPENAI_COMPATIBLE_API_KEY not set)

---

3 of 9 providers smoked (6 not configured or without candidates); 0 without an answer or not buildable
```

Run 1 stderr (the harness's logger at `warn`; the parameters the profile does not admit were omitted from the request, ADR-0061, and recorded in `paladin.params_omitted`; the vendor-default sampling values are the harness's, not the effort path's):

```text
[WARN] provider 'anthropic' model 'claude-opus-4-8': parameter 'temperature' = 0.7 is not accepted by this model's profile and was omitted from the request (ADR-0061); it is recorded in the response metadata under 'paladin.params_omitted'
[WARN] provider 'anthropic' model 'claude-opus-4-8': parameter 'top_p' = 1 is not accepted by this model's profile and was omitted from the request (ADR-0061); it is recorded in the response metadata under 'paladin.params_omitted'
[WARN] provider 'anthropic' model 'claude-opus-4-8': parameter 'frequency_penalty' = 0 is not accepted by this model's profile and was omitted from the request (ADR-0061); it is recorded in the response metadata under 'paladin.params_omitted'
[WARN] provider 'anthropic' model 'claude-opus-4-8': parameter 'presence_penalty' = 0 is not accepted by this model's profile and was omitted from the request (ADR-0061); it is recorded in the response metadata under 'paladin.params_omitted'
[WARN] provider 'kimi' model 'kimi-k3': parameter 'temperature' = 0.7 is not accepted by this model's profile and was omitted from the request (ADR-0061); it is recorded in the response metadata under 'paladin.params_omitted'
[WARN] provider 'kimi' model 'kimi-k3': parameter 'top_p' = 1 is not accepted by this model's profile and was omitted from the request (ADR-0061); it is recorded in the response metadata under 'paladin.params_omitted'
[WARN] provider 'kimi' model 'kimi-k3': parameter 'frequency_penalty' = 0 is not accepted by this model's profile and was omitted from the request (ADR-0061); it is recorded in the response metadata under 'paladin.params_omitted'
[WARN] provider 'kimi' model 'kimi-k3': parameter 'presence_penalty' = 0 is not accepted by this model's profile and was omitted from the request (ADR-0061); it is recorded in the response metadata under 'paladin.params_omitted'
```

Run 2: the documented `llm.kimi` configuration on `kimi-k3`.
Command: `live_vendor_smoke --effort-smoke --models kimi=kimi-k3 --max-tokens 16000 --timeout-seconds 300`. Exit 0.

```text
# Live vendor effort smoke (adapter path)

- date (UTC): 2026-10-22 18:15:58Z
- commit: 4a9c290e
- credential variables found (names only, never values): OPENAI_API_KEY, ANTHROPIC_API_KEY, DEEPSEEK_API_KEY, MOONSHOT_API_KEY, DASHSCOPE_API_KEY, XAI_API_KEY, GEMINI_API_KEY
- prompt: "What is 17 multiplied by 23? Work it out, then answer with the number only."; two requests per candidate through the shipped adapter (numbers and metadata values are recorded; response content is never printed)

## OpenAI

NOT PROBED: no `--models` candidate for this provider

## Anthropic

NOT PROBED: no `--models` candidate for this provider

## DeepSeek

NOT PROBED: no `--models` candidate for this provider

## Kimi

| model | requested | outcome | finish | reasoning tokens | completion tokens | paladin.reasoning_requested | paladin.reasoning_effective | ms |
|-------|-----------|---------|--------|------------------|-------------------|-----------------------------|-----------------------------|----|
| kimi-k3 | low | Ok | stop | 47 | 63 | low | low | 4464 |
| kimi-k3 | max | Ok | stop | 26 | 42 | max | max | 4027 |

## Qwen

NOT PROBED: no `--models` candidate for this provider

## Grok

NOT PROBED: no `--models` candidate for this provider

## Gemini

NOT PROBED: no `--models` candidate for this provider

## Ollama

SKIPPED (OLLAMA_BASE_URL not set)

## OpenAI-compatible

SKIPPED (OPENAI_COMPATIBLE_API_KEY not set)

---

1 of 9 providers smoked (8 not configured or without candidates); 0 without an answer or not buildable
```

Run 2 stderr: the same four `kimi-k3` omission warnings as run 1 (`temperature`, `top_p`, `frequency_penalty`, `presence_penalty`, ADR-0061), nothing else.
