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
