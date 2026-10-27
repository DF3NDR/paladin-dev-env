# Phase 50: Live vendor usage-containment probe (G-50-5a)

This file records, for each candidate vendor model, how the vendor's usage object relates reasoning tokens to
`completion_tokens` and to the vendor's own total. It is the evidence plan 50-24 applies to the per-provider usage
semantics (compat presets' `CompatUsageSemantics`, the bespoke `map_usage` / `map_gemini_usage` mappers). Plan 50-22
produced it. Everything below the `## Transcript` heading is the operator's output, copied verbatim and never edited.

## Provenance

- **Run by:** the operator, on their own vendor keys and machine (no vendor key exists in the Claude environment).
- **Transcript date (UTC):** 2026-10-18 02:53:08Z.
- **Harness commit:** `8b113aea` (`live_vendor_smoke --usage`, plan 50-22 Task 1). Committed by the operator as
  `50-USAGE-PROBE.raw.md` in `b249d25e`; this file replaces that raw file.
- **Command:** `cargo run -q -p paladin-llm --example live_vendor_smoke --features
  openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatible -- --usage --models
  openai=gpt-5.4-nano,gpt-4o-mini --models anthropic=claude-haiku-5-5 --models deepseek=deepseek-flash --models
  kimi=kimi-k2.6,kimi-k3 --models qwen=qwen-plus,qwen3.8-flash --models grok=grok-4.3,grok-4.5,grok-4.6 --models
  gemini=gemini-3.5-flash,gemini-3.5-flash-lite,gemini-3.6-flash` (one call per model, raw vendor wire, 1024-token
  cap, no body extras, no sampling parameters, vendor-default reasoning effort).
- **Measured (7):** OpenAI, Anthropic, DeepSeek, Kimi, Qwen, Grok, Gemini. Every one of the 15 seeded calls returned
  `ACCEPTED (200)`.
- **Skipped (2):** Ollama (`OLLAMA_BASE_URL` not set) and OpenAI-compatible (`OPENAI_COMPATIBLE_API_KEY` not set).
  These were not configured, so their lines read `not measured (not configured)`.
- **Grok's reasoning containment is live-measured.** All three seeded Grok models report reasoning beside completion,
  and the billed cost matches that reading exactly (see the cross-check below).
- **Scrub:** this file was scanned before commit with every marker `redact_secret_patterns`
  (`crates/paladin-llm/src/redaction.rs`) redacts: bearer-token values, OpenAI/Anthropic-style secret-key prefixes,
  AWS access-key ids, key and token query parameters and JWT-shaped triples. The scan also covered Google API-key and
  xAI key prefixes, and the literal value of every credential variable set in the Claude environment (none of the
  nine was set there). The scan found no match. The transcript carries header names only, never header values, and
  no URL query strings.

### Containment verdicts

- openai gpt-5.4-nano: indeterminate (no reasoning reported) (2026-10-18)
- openai gpt-4o-mini: indeterminate (no reasoning reported) (2026-10-18)
- anthropic claude-haiku-5-5: no vendor total (2026-10-18)
- deepseek deepseek-flash: reasoning inside completion (2026-10-18)
- kimi kimi-k2.6: reasoning inside completion (2026-10-18)
- kimi kimi-k3: reasoning inside completion (2026-10-18)
- qwen qwen3.8-flash: reasoning inside completion (2026-10-18)
- qwen qwen-plus: indeterminate (no reasoning reported) (2026-10-18)
- grok grok-4.3: reasoning beside completion (2026-10-18)
- grok grok-4.5: reasoning beside completion (2026-10-18)
- grok grok-4.6: reasoning beside completion (2026-10-18)
- gemini gemini-3.5-flash-lite: indeterminate (no reasoning reported) (2026-10-18)
- gemini gemini-3.5-flash: reasoning beside completion (2026-10-18)
- gemini gemini-3.6-flash: reasoning beside completion (2026-10-18)
- ollama llama3: not measured (not configured) (2026-10-18)
- openai-compatible <OPENAI_COMPATIBLE_MODEL>: not measured (not configured) (2026-10-18)

Reading notes. These do not change any verdict above.

- **Gemini:** the harness computes the verdict with prompt = `promptTokenCount + toolUsePromptTokenCount` and
  completion = `candidatesTokenCount`. So `reasoning beside completion` means `thoughtsTokenCount` sits outside
  `candidatesTokenCount`; for example 23 + 3 + 192 = 218 on `gemini-3.5-flash`. This is the reading the Gemini
  adapter's `map_gemini_usage` already applies, because it adds thoughts into the completion count.
- **Anthropic:** sends no vendor total, so no containment arithmetic is possible. The recorded `thinking_tokens` (61)
  is at most `output_tokens` (64), which agrees with the inclusive reading plan 50-23 guards at runtime. It is not a
  measurement of containment.
- **`indeterminate` rows** reported no reasoning count, or a count of 0. For each of them the vendor total equals
  prompt + completion: gpt-5.4-nano 26 + 4 = 30, gpt-4o-mini 27 + 1 = 28, qwen-plus 30 + 3 = 33 and
  gemini-3.5-flash-lite 23 + 58 = 81.

### Grok billed-cost cross-check

**Unit assumption, stated explicitly:** the xAI `/v1/models` listing prices in `50-SEED-PROBE.md` (`## Grok`) are
taken as `cost_in_usd_ticks` per token, with 1 tick = 10^-10 USD. On that reading, `prompt_text_token_price=12500` is
$1.25 per million tokens. Every prompt here is far below the listing's `long_context_threshold=200000`, so the
short-context prices apply. The reported `cached` count is treated as a subset of `prompt`, so uncached = prompt −
cached. If reasoning is billed at the completion rate, the expected cost is:

`expected = (prompt − cached) × prompt_text_token_price + cached × cached_prompt_text_token_price + (completion + reasoning) × completion_text_token_price`

| model | prices (prompt / cached / completion) | arithmetic | expected | reported | result |
|---|---|---|---|---|---|
| grok-4.3 | 12500 / 2000 / 25000 | (212 − 128) × 12500 + 128 × 2000 + (1 + 457) × 25000 = 1 050 000 + 256 000 + 11 450 000 | 12 756 000 | 12 756 000 | consistent |
| grok-4.5 | 20000 / 3000 / 60000 | (514 − 384) × 20000 + 384 × 3000 + (1 + 129) × 60000 = 2 600 000 + 1 152 000 + 7 800 000 | 11 552 000 | 11 552 000 | consistent |
| grok-4.6 | 20000 / 5000 / 60000 | (656 − 512) × 20000 + 512 × 5000 + (1 + 273) × 60000 = 2 880 000 + 2 560 000 + 16 440 000 | 21 880 000 | 21 880 000 | consistent |

All three are **consistent**, to the tick: xAI bills Grok's reasoning tokens at the completion rate, on top of a
`completion_tokens` figure that excludes them. Without the reasoning tokens the expected figures would be 1 331 000,
3 812 000 and 5 500 000 ticks, which do not match. So the cost evidence independently confirms the `reasoning beside
completion` verdicts, and it supports plan 50-21's `ReasoningBesideCompletion` fold, which prices reasoning at the
completion rate.

This file is the evidence plan 50-24 applies to the per-provider usage semantics.

## Transcript

# Live vendor usage probe

- date (UTC): 2026-10-18 02:53:08Z
- commit: 8b113aea
- credential variables found (names only, never values): OPENAI_API_KEY, ANTHROPIC_API_KEY, DEEPSEEK_API_KEY, MOONSHOT_API_KEY, DASHSCOPE_API_KEY, XAI_API_KEY, GEMINI_API_KEY
- prompt: "What is 17 multiplied by 23? Work it out, then answer with the number only." (numbers are recorded; response content is never printed)

## OpenAI

- base URL: `https://api.openai.com/v1`
- chat endpoint: `POST https://api.openai.com/v1/chat/completions`; credential header `Authorization` (name only, value never printed)
- every call carries `max_completion_tokens` = 1024, a 120 s timeout, no body extras and no sampling parameters; redirects are never followed

| model | outcome | prompt | completion | reasoning | vendor total | verdict | extras |
|---|---|---|---|---|---|---|---|
| gpt-5.4-nano | ACCEPTED (200) | 26 | 4 | 0 | 30 | indeterminate (no reasoning reported) | cached=0 |
| gpt-4o-mini | ACCEPTED (200) | 27 | 1 | 0 | 28 | indeterminate (no reasoning reported) | cached=0 |

## Anthropic

- base URL: `https://api.anthropic.com/v1`
- chat endpoint: `POST https://api.anthropic.com/v1/messages`; credential header `x-api-key` (name only, value never printed)
- every call carries `max_tokens` = 1024, a 120 s timeout, no body extras and no sampling parameters; redirects are never followed

| model | outcome | prompt | completion | reasoning | vendor total | verdict | extras |
|---|---|---|---|---|---|---|---|
| claude-haiku-5-5 | ACCEPTED (200) | 34 | 64 | 61 | - | no vendor total | cached=0; cache_write=0 |

## DeepSeek

- base URL: `https://api.deepseek.com/v1`
- chat endpoint: `POST https://api.deepseek.com/v1/chat/completions`; credential header `Authorization` (name only, value never printed)
- every call carries `max_tokens` = 1024, a 120 s timeout, no body extras and no sampling parameters; redirects are never followed

| model | outcome | prompt | completion | reasoning | vendor total | verdict | extras |
|---|---|---|---|---|---|---|---|
| deepseek-flash | ACCEPTED (200) | 50 | 32 | 30 | 82 | reasoning inside completion | cached=0 |

## Kimi

- base URL: `https://api.moonshot.ai/v1`
- chat endpoint: `POST https://api.moonshot.ai/v1/chat/completions`; credential header `Authorization` (name only, value never printed)
- every call carries `max_tokens` = 1024, a 120 s timeout, no body extras and no sampling parameters; redirects are never followed

| model | outcome | prompt | completion | reasoning | vendor total | verdict | extras |
|---|---|---|---|---|---|---|---|
| kimi-k2.6 | ACCEPTED (200) | 27 | 147 | 144 | 174 | reasoning inside completion | - |
| kimi-k3 | ACCEPTED (200) | 105 | 158 | 142 | 263 | reasoning inside completion | cache_write=0 |

## Qwen

- base URL: `https://dashscope-intl.aliyuncs.com/compatible-mode/v1`
- chat endpoint: `POST https://dashscope-intl.aliyuncs.com/compatible-mode/v1/chat/completions`; credential header `Authorization` (name only, value never printed)
- every call carries `max_tokens` = 1024, a 120 s timeout, no body extras and no sampling parameters; redirects are never followed

| model | outcome | prompt | completion | reasoning | vendor total | verdict | extras |
|---|---|---|---|---|---|---|---|
| qwen3.8-flash | ACCEPTED (200) | 83 | 75 | 69 | 158 | reasoning inside completion | cached=0 |
| qwen-plus | ACCEPTED (200) | 30 | 3 | - | 33 | indeterminate (no reasoning reported) | cached=0 |

## Grok

- base URL: `https://api.x.ai/v1`
- chat endpoint: `POST https://api.x.ai/v1/chat/completions`; credential header `Authorization` (name only, value never printed)
- every call carries `max_tokens` = 1024, a 120 s timeout, no body extras and no sampling parameters; redirects are never followed

| model | outcome | prompt | completion | reasoning | vendor total | verdict | extras |
|---|---|---|---|---|---|---|---|
| grok-4.3 | ACCEPTED (200) | 212 | 1 | 457 | 670 | reasoning beside completion | cached=128; cost_in_usd_ticks=12756000; num_sources_used=0 |
| grok-4.5 | ACCEPTED (200) | 514 | 1 | 129 | 644 | reasoning beside completion | cached=384; cost_in_usd_ticks=11552000; num_sources_used=0 |
| grok-4.6 | ACCEPTED (200) | 656 | 1 | 273 | 930 | reasoning beside completion | cached=512; cost_in_usd_ticks=21880000; num_sources_used=0 |

## Gemini

- base URL: `https://generativelanguage.googleapis.com/v1beta`
- chat endpoint: `POST https://generativelanguage.googleapis.com/v1beta/models/gemini-3.5-flash-lite:generateContent`; credential header `x-goog-api-key` (name only, value never printed)
- every call carries `generationConfig.maxOutputTokens` = 1024, a 120 s timeout, no body extras and no sampling parameters; redirects are never followed

| model | outcome | prompt | completion | reasoning | vendor total | verdict | extras |
|---|---|---|---|---|---|---|---|
| gemini-3.5-flash-lite | ACCEPTED (200) | 23 | 58 | - | 81 | indeterminate (no reasoning reported) | - |
| gemini-3.5-flash | ACCEPTED (200) | 23 | 3 | 192 | 218 | reasoning beside completion | - |
| gemini-3.6-flash | ACCEPTED (200) | 23 | 3 | 202 | 228 | reasoning beside completion | - |

## Ollama

SKIPPED (OLLAMA_BASE_URL not set)

## OpenAI-compatible

SKIPPED (OPENAI_COMPATIBLE_API_KEY not set)

---

7 of 9 providers measured (2 not configured); 0 unreachable or not buildable
