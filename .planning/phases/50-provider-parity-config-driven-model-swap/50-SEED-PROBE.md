# 50-SEED-PROBE: operator live seed probe (Phase 50, D-08 and D-11a)

This file is the sole seed source for plan 50-06 (the dated model registry) and the usage-cap evidence for
plan 50-13; anything that is not in it must be left out.

## Provenance

- Who ran it: the operator (Am0rfu5), on their own vendor keys, from the project devcontainer. No vendor key
  exists in the Claude environment, so Claude did not run, edit or re-measure any value below.
- When: 2026-10-15 22:23:15Z (the transcript's own UTC date line). This is the operator's second run of that
  day; the first (2026-10-15 19:56Z) hit Gemini free-tier rate limits and is preserved only in git history
  (commit `c33022b9`). Everything below is the second run.
- Commit it ran at: `c33022b9` (the transcript's `commit:` line).
- Curated on 2026-10-16 from the operator's raw transcript. The transcript that follows the `---` rule is
  verbatim, including its `## Operator-supplied evidence` section (values there carry their own *observed* or
  *docs* label, source URL and access date) and its `## Owner reading R-2` section. No measured value was edited.
- Scrub check (plan 50-17 Task 2): the whole file was run against every marker and shape that
  `paladin_llm::redaction::redact_secret_patterns` recognises (authorization-header tokens, vendor-prefixed secret keys, AWS access key
  ids, key and token query values, JWT-shaped triples) plus the Google and xAI key shapes, authorization-style header
  lines, and the literal value of every credential variable present in this environment. Nothing matched, so
  nothing was redacted or edited.

## Providers PROBED and SKIPPED

PROBED (7 of 9): OpenAI, Anthropic, DeepSeek, Kimi, Qwen, Grok, Gemini.

SKIPPED (2 of 9), both deliberately, as named by the operator in the resume signal:

- Ollama: SKIPPED (`OLLAMA_BASE_URL` not set); no local daemon is configured.
- OpenAI-compatible: SKIPPED (`OPENAI_COMPATIBLE_API_KEY` not set); no endpoint is configured.

Models listed and candidates probed, per provider, counted from the transcript's tables:

| provider | status | models listed | candidates probed |
|---|---|---|---|
| OpenAI | PROBED | 135 | 2 |
| Anthropic | PROBED | 14 | 9 |
| DeepSeek | PROBED | 2 | 3 |
| Kimi | PROBED | 4 | 4 |
| Qwen | PROBED | 99 | 4 |
| Grok | PROBED | 14 | 3 |
| Gemini | PROBED | 63 | 3 |
| Ollama | SKIPPED | not listed | 0 |
| OpenAI-compatible | SKIPPED | not listed | 0 |

DeepSeek probed three candidates against a two-model listing: `deepseek-chat` and `deepseek-coder` are not in
the listing (the operator's evidence section (b) classes them as undocumented aliases, not to be seeded).
Anthropic probed nine candidates against a fourteen-model listing; the operator's evidence section records
that the eight curated legacy ids returned 404 and only `claude-haiku-5-5` was measured.

### Usage-cap evidence (owner reading R-2)

- kimi: not captured -- owner accepted unverified (2026-10-16)
- qwen: not captured -- owner accepted unverified (2026-10-16)
- grok: not captured -- owner accepted unverified (2026-10-16)
- ollama: not captured -- owner accepted unverified (2026-10-16)
- openai-compatible: not captured -- owner accepted unverified (2026-10-16)

No quota-exhaustion (usage-cap) body exists for any of the five compat presets. The two rate-limit bodies in
the operator's section (a) (Gemini free-tier, DashScope `limit_requests` on `qwen-turbo`) are recorded there
for context only and are not usage-cap evidence. Plan 50-13 therefore encodes no usage-cap `error_override` for
these five vendors; each keeps today's error mapping and is listed as unverified.

---

# Live vendor probe

- date (UTC): 2026-10-15 22:23:15Z
- commit: c33022b9
- credential variables found (names only, never values): OPENAI_API_KEY, ANTHROPIC_API_KEY, DEEPSEEK_API_KEY, MOONSHOT_API_KEY, DASHSCOPE_API_KEY, XAI_API_KEY, GEMINI_API_KEY

## OpenAI

- base URL: `https://api.openai.com/v1`
- chat endpoint: `POST https://api.openai.com/v1/chat/completions`; credential header `Authorization` (name only, value never printed)
- models endpoint: `GET https://api.openai.com/v1/models`
- declared temperature range: 0..1
- every call carries `max_completion_tokens` = 256 and a 120 s timeout; redirects are never followed

### Models listing

| model id | limit / capability fields (as received) |
|---|---|
| gpt-4-turbo-2024-04-09 | created=1712601677 ; object=model ; owned_by=system ; shutdown_date=2026-10-23 |
| gpt-realtime-2.1-mini | created=1782254706 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-realtime-2025-08-28 | created=1756271773 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-4 | created=1687882411 ; object=model ; owned_by=openai ; shutdown_date=2026-10-23 |
| omni-moderation-2024-09-26 | created=1732734466 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-live-transcribe | created=1785168034 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-5.6-luna | created=1782228658 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-3.5-turbo-0125 | created=1706048358 ; object=model ; owned_by=system ; shutdown_date=2026-10-23 |
| gpt-5.4-2026-03-05 | created=1772654062 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-5-nano | created=1754426384 ; object=model ; owned_by=system ; shutdown_date=null |
| tts-1-hd | created=1699046015 ; object=model ; owned_by=system ; shutdown_date=2027-01-06 |
| gpt-realtime-1.5 | created=1771461469 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-realtime | created=1756271701 ; object=model ; owned_by=system ; shutdown_date=2027-01-20 |
| gpt-5.2-pro | created=1765343983 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-5-chat-latest | created=1754073306 ; object=model ; owned_by=system ; shutdown_date=2026-07-23 |
| omni-moderation-latest | created=1731689265 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-5.4-mini | created=1773451123 ; object=model ; owned_by=system ; shutdown_date=null |
| o1-pro | created=1742251791 ; object=model ; owned_by=system ; shutdown_date=2026-10-23 |
| gpt-5.1-codex | created=1762988221 ; object=model ; owned_by=system ; shutdown_date=2026-07-23 |
| gpt-5-2025-08-07 | created=1754075360 ; object=model ; owned_by=system ; shutdown_date=2026-12-11 |
| gpt-realtime-2 | created=1778006032 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-5-search-api | created=1759514629 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-5.5-2026-04-23 | created=1776839241 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-5.5-pro | created=1776894349 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-audio-2025-08-28 | created=1756256146 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-4o-mini-transcribe-2025-03-20 | created=1765610545 ; object=model ; owned_by=system ; shutdown_date=2027-01-20 |
| tts-1-1106 | created=1699053241 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-5.5-pro-2026-04-23 | created=1776894470 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-audio-mini-2025-10-06 | created=1759512137 ; object=model ; owned_by=system ; shutdown_date=2026-07-23 |
| gpt-5.2-pro-2025-12-11 | created=1765343959 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-4o-mini-transcribe-2025-12-15 | created=1765610407 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-4.1-nano | created=1744321707 ; object=model ; owned_by=system ; shutdown_date=2026-10-23 |
| gpt-image-2 | created=1776399795 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-3.5-turbo-16k | created=1683758102 ; object=model ; owned_by=openai-internal ; shutdown_date=null |
| text-embedding-ada-002 | created=1671217299 ; object=model ; owned_by=openai-internal ; shutdown_date=null |
| gpt-realtime-2.1 | created=1782254687 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-4.1 | created=1744316542 ; object=model ; owned_by=system ; shutdown_date=2027-01-14 |
| gpt-5.5 | created=1776824847 ; object=model ; owned_by=system ; shutdown_date=null |
| tts-1 | created=1681940951 ; object=model ; owned_by=openai-internal ; shutdown_date=2027-01-06 |
| gpt-5-mini | created=1754425928 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-5-mini-2025-08-07 | created=1754425867 ; object=model ; owned_by=system ; shutdown_date=2026-12-11 |
| gpt-5.4-mini-2026-03-17 | created=1773451076 ; object=model ; owned_by=system ; shutdown_date=null |
| babbage-002 | created=1692634615 ; object=model ; owned_by=system ; shutdown_date=2026-09-28 |
| gpt-4o-2024-05-13 | created=1715368132 ; object=model ; owned_by=system ; shutdown_date=2026-10-23 |
| text-embedding-3-large | created=1705953180 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-5-pro-2025-10-06 | created=1759469707 ; object=model ; owned_by=system ; shutdown_date=2026-12-11 |
| gpt-5.6-sol | created=1782228018 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-realtime-mini | created=1759517133 ; object=model ; owned_by=system ; shutdown_date=2027-01-20 |
| gpt-image-1-mini | created=1758845821 ; object=model ; owned_by=system ; shutdown_date=2026-12-01 |
| gpt-5.6-terra | created=1782228459 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-3.5-turbo-instruct-0914 | created=1694122472 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-5.1-chat-latest | created=1762547951 ; object=model ; owned_by=system ; shutdown_date=2026-07-23 |
| gpt-5.4-pro | created=1772659601 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-4o-mini-tts | created=1742403959 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-realtime-mini-2025-12-15 | created=1765612007 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-5.1-codex-max | created=1763671532 ; object=model ; owned_by=system ; shutdown_date=2026-07-23 |
| chat-latest | created=1777704602 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-realtime-translate | created=1777950216 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-5.4-pro-2026-03-05 | created=1772659657 ; object=model ; owned_by=system ; shutdown_date=null |
| o3-mini-2025-01-31 | created=1738010200 ; object=model ; owned_by=system ; shutdown_date=2026-10-23 |
| gpt-audio-1.5 | created=1771550885 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-image-1.5 | created=1764030620 ; object=model ; owned_by=system ; shutdown_date=2026-12-01 |
| gpt-4o | created=1715367049 ; object=model ; owned_by=system ; shutdown_date=2027-01-14 |
| tts-1-hd-1106 | created=1699053533 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-3.5-turbo-1106 | created=1698959748 ; object=model ; owned_by=system ; shutdown_date=2026-09-28 |
| gpt-4o-mini-tts-2025-03-20 | created=1765610731 ; object=model ; owned_by=system ; shutdown_date=2027-01-06 |
| gpt-5.2 | created=1765313051 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-5.4-nano-2026-03-17 | created=1773450837 ; object=model ; owned_by=system ; shutdown_date=null |
| o1 | created=1734375816 ; object=model ; owned_by=system ; shutdown_date=2026-10-23 |
| gpt-4-turbo | created=1712361441 ; object=model ; owned_by=system ; shutdown_date=2026-10-23 |
| gpt-4o-2024-08-06 | created=1722814719 ; object=model ; owned_by=system ; shutdown_date=2027-01-14 |
| o3 | created=1744225308 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-4o-mini-search-preview-2025-03-11 | created=1741390858 ; object=model ; owned_by=system ; shutdown_date=2026-07-23 |
| gpt-4.1-mini | created=1744318173 ; object=model ; owned_by=system ; shutdown_date=2027-01-14 |
| gpt-4o-mini | created=1721172741 ; object=model ; owned_by=system ; shutdown_date=2027-01-14 |
| o4-mini-deep-research | created=1749685485 ; object=model ; owned_by=system ; shutdown_date=2026-07-23 |
| gpt-5.4 | created=1772691852 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-4o-search-preview-2025-03-11 | created=1771905621 ; object=model ; owned_by=system ; shutdown_date=2026-07-23 |
| text-embedding-3-small | created=1705948997 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-4o-transcribe-diarize | created=1750798887 ; object=model ; owned_by=system ; shutdown_date=2027-02-26 |
| gpt-5-codex | created=1757527818 ; object=model ; owned_by=system ; shutdown_date=2026-07-23 |
| gpt-4-0613 | created=1686588896 ; object=model ; owned_by=openai ; shutdown_date=2026-10-23 |
| gpt-5.1-2025-11-13 | created=1762800353 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-4o-search-preview | created=1771905534 ; object=model ; owned_by=system ; shutdown_date=null |
| o1-2024-12-17 | created=1734326976 ; object=model ; owned_by=system ; shutdown_date=2026-10-23 |
| gpt-5.3-chat-latest | created=1772236571 ; object=model ; owned_by=system ; shutdown_date=2026-08-10 |
| o3-2025-04-16 | created=1744133301 ; object=model ; owned_by=system ; shutdown_date=2026-12-11 |
| gpt-5.2-2025-12-11 | created=1765313028 ; object=model ; owned_by=system ; shutdown_date=null |
| chatgpt-image-latest | created=1765925279 ; object=model ; owned_by=system ; shutdown_date=2026-12-01 |
| gpt-3.5-turbo | created=1677610602 ; object=model ; owned_by=openai ; shutdown_date=2026-10-23 |
| sora-2 | created=1759708615 ; object=model ; owned_by=system ; shutdown_date=2026-09-24 |
| gpt-live-1 | created=1788889407 ; object=model ; owned_by=system ; shutdown_date=null |
| davinci-002 | created=1692634301 ; object=model ; owned_by=system ; shutdown_date=2026-09-28 |
| gpt-5.4-nano | created=1773450870 ; object=model ; owned_by=system ; shutdown_date=2027-04-01 |
| o3-mini | created=1737146383 ; object=model ; owned_by=system ; shutdown_date=2026-10-23 |
| o4-mini-deep-research-2025-06-26 | created=1750866121 ; object=model ; owned_by=system ; shutdown_date=2026-07-23 |
| gpt-audio-mini-2025-12-15 | created=1765760008 ; object=model ; owned_by=system ; shutdown_date=null |
| o1-pro-2025-03-19 | created=1742251504 ; object=model ; owned_by=system ; shutdown_date=2026-10-23 |
| gpt-4o-mini-tts-2025-12-15 | created=1765610837 ; object=model ; owned_by=system ; shutdown_date=2027-01-06 |
| gpt-image-1 | created=1745517030 ; object=model ; owned_by=system ; shutdown_date=2026-10-23 |
| gpt-audio | created=1756339249 ; object=model ; owned_by=system ; shutdown_date=2027-01-20 |
| gpt-5.3-codex | created=1770537915 ; object=model ; owned_by=system ; shutdown_date=2027-04-01 |
| gpt-5-search-api-2025-10-14 | created=1760043960 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-image-2.5-sunburst | created=1788563162 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-image-2.5-flare-2026-09-08 | created=1788851006 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-5 | created=1754425777 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-4o-mini-transcribe | created=1742068596 ; object=model ; owned_by=system ; shutdown_date=2027-02-26 |
| gpt-image-2-2026-04-21 | created=1776399994 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-5.2-chat-latest | created=1765344352 ; object=model ; owned_by=system ; shutdown_date=2026-08-10 |
| sora-2-pro | created=1759708663 ; object=model ; owned_by=system ; shutdown_date=2026-09-24 |
| gpt-3.5-turbo-instruct | created=1692901427 ; object=model ; owned_by=system ; shutdown_date=2026-09-28 |
| gpt-transcribe | created=1785168027 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-4.1-nano-2025-04-14 | created=1744321025 ; object=model ; owned_by=system ; shutdown_date=2026-10-23 |
| gpt-6-astra | created=1787853604 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-audio-mini | created=1759512027 ; object=model ; owned_by=system ; shutdown_date=2027-01-20 |
| gpt-4.1-mini-2025-04-14 | created=1744317547 ; object=model ; owned_by=system ; shutdown_date=2027-01-14 |
| gpt-4o-mini-2024-07-18 | created=1721172717 ; object=model ; owned_by=system ; shutdown_date=2027-01-14 |
| gpt-4.1-2025-04-14 | created=1744315746 ; object=model ; owned_by=system ; shutdown_date=2027-01-14 |
| gpt-5.2-codex | created=1766164985 ; object=model ; owned_by=system ; shutdown_date=2026-07-23 |
| gpt-4o-mini-search-preview | created=1741391161 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-6-sol | created=1789405937 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-image-2.5-sunburst-2026-09-08 | created=1788851010 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-6.1-sol | created=1790552874 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-5-nano-2025-08-07 | created=1754426303 ; object=model ; owned_by=system ; shutdown_date=2026-12-11 |
| gpt-4o-transcribe | created=1742068463 ; object=model ; owned_by=system ; shutdown_date=2027-02-26 |
| o4-mini | created=1744225351 ; object=model ; owned_by=system ; shutdown_date=2026-10-23 |
| gpt-5.1-codex-mini | created=1763007109 ; object=model ; owned_by=system ; shutdown_date=2026-07-23 |
| o4-mini-2025-04-16 | created=1744133506 ; object=model ; owned_by=system ; shutdown_date=2026-10-23 |
| gpt-4o-2024-11-20 | created=1739331543 ; object=model ; owned_by=system ; shutdown_date=2027-01-14 |
| gpt-5.1 | created=1762800673 ; object=model ; owned_by=system ; shutdown_date=2027-04-01 |
| whisper-1 | created=1677532384 ; object=model ; owned_by=openai-internal ; shutdown_date=2027-02-26 |
| gpt-5-pro | created=1759469822 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-realtime-whisper | created=1778012060 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-6-luna | created=1789406102 ; object=model ; owned_by=system ; shutdown_date=null |
| gpt-image-2.5-flare | created=1788563147 ; object=model ; owned_by=system ; shutdown_date=null |

### Probes: `gpt-4o-mini`

| # | probe | body extras | outcome | finish | content | usage fields | reasoning text |
|---|-------|-------------|---------|--------|---------|--------------|----------------|
| 1 | default parameters | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 2 | temperature=0.00 (declared low end) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 3 | temperature=1.00 (declared high end) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 4 | temperature=0.50 (mid-range) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 5 | temperature=1.01 (just outside the high end) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 6 | top_p=0.5 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 7 | frequency_penalty=0.5 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 8 | presence_penalty=0.5 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 9 | stop sequences: 1 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 10 | stop sequences: 5 | - | REJECTED (400): {   "error": {     "message": "Invalid 'stop': array too long. Expected an array with maximum length 4, but got an array with length 5 instead.",     "type": "invalid_request_error",     "param": "stop",     "code": "array_above_max_length"   } } | - | - | - | - |
| 11 | max_tokens=64 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 12 | max_completion_tokens=64 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 13 | json_object mode (response_format json_object; prompt names JSON) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 14 | json_schema mode (response_format json_schema; prompt names JSON) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |

### Probes: `gpt-5.4-nano`

| # | probe | body extras | outcome | finish | content | usage fields | reasoning text |
|---|-------|-------------|---------|--------|---------|--------------|----------------|
| 1 | default parameters | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 2 | temperature=0.00 (declared low end) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 3 | temperature=1.00 (declared high end) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 4 | temperature=0.50 (mid-range) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 5 | temperature=1.01 (just outside the high end) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 6 | top_p=0.5 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 7 | frequency_penalty=0.5 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 8 | presence_penalty=0.5 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 9 | stop sequences: 1 | - | REJECTED (400): {   "error": {     "message": "Unsupported parameter: 'stop' is not supported with this model.",     "type": "invalid_request_error",     "param": "stop",     "code": "unsupported_parameter"   } } | - | - | - | - |
| 10 | stop sequences: 5 | - | REJECTED (400): {   "error": {     "message": "Unsupported parameter: 'stop' is not supported with this model.",     "type": "invalid_request_error",     "param": "stop",     "code": "unsupported_parameter"   } } | - | - | - | - |
| 11 | max_tokens=64 | - | REJECTED (400): {   "error": {     "message": "Unsupported parameter: 'max_tokens' is not supported with this model. Use 'max_completion_tokens' instead.",     "type": "invalid_request_error",     "param": "max_tokens",     "code": "unsupported_parameter"   } } | - | - | - | - |
| 12 | max_completion_tokens=64 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 13 | json_object mode (response_format json_object; prompt names JSON) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 14 | json_schema mode (response_format json_schema; prompt names JSON) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |

### Quota-shaped errors observed

none observed (a quota error cannot be provoked on demand)

## Anthropic

- base URL: `https://api.anthropic.com/v1`
- chat endpoint: `POST https://api.anthropic.com/v1/messages`; credential header `x-api-key` (name only, value never printed)
- models endpoint: `GET https://api.anthropic.com/v1/models`
- declared temperature range: 0..1
- every call carries `max_tokens` = 256 and a 120 s timeout; redirects are never followed

### Models listing

| model id | limit / capability fields (as received) |
|---|---|
| claude-haiku-5-5 | capabilities={"batch":{"supported":true},"citations":{"supported":true},"code_execution":{"supported":true},"context_management":{"clear_thinking_20251015":{"supported":true},"clear_tool_uses_20250919":{"supported":true},"compact_20260112":{"supported":true},"supported":true},"effort":{"high":{"supported":true},"low":{"supported":true},"max":{"supported":true},"medium":{"supported":true},"supported":true,"xhigh":{"supported":true}},"image_input":{"supported":true},"pdf_input":{"supported":true},"server_tool... [truncated, 808 total bytes] ; created_at=2026-10-07T18:00:00Z ; deprecated_at=null ; display_name=Claude Haiku 5.5 ; lifecycle=active ; line=haiku ; max_input_tokens=1000000 ; max_tokens=128000 ; retires_at=null ; type=model |
| claude-sonnet-5-5 | capabilities={"batch":{"supported":true},"citations":{"supported":true},"code_execution":{"supported":true},"context_management":{"clear_thinking_20251015":{"supported":true},"clear_tool_uses_20250919":{"supported":true},"compact_20260112":{"supported":true},"supported":true},"effort":{"high":{"supported":true},"low":{"supported":true},"max":{"supported":true},"medium":{"supported":true},"supported":true,"xhigh":{"supported":true}},"image_input":{"supported":true},"pdf_input":{"supported":true},"server_tool... [truncated, 808 total bytes] ; created_at=2026-09-28T00:00:00Z ; deprecated_at=null ; display_name=Claude Sonnet 5.5 ; lifecycle=active ; line=sonnet ; max_input_tokens=1000000 ; max_tokens=128000 ; retires_at=null ; type=model |
| claude-opus-5-5 | capabilities={"batch":{"supported":true},"citations":{"supported":true},"code_execution":{"supported":true},"context_management":{"clear_thinking_20251015":{"supported":true},"clear_tool_uses_20250919":{"supported":true},"compact_20260112":{"supported":true},"supported":true},"effort":{"high":{"supported":true},"low":{"supported":true},"max":{"supported":true},"medium":{"supported":true},"supported":true,"xhigh":{"supported":true}},"image_input":{"supported":true},"pdf_input":{"supported":true},"server_tool... [truncated, 809 total bytes] ; created_at=2026-09-21T16:24:00Z ; deprecated_at=null ; display_name=Claude Opus 5.5 ; lifecycle=active ; line=opus ; max_input_tokens=1000000 ; max_tokens=128000 ; retires_at=null ; type=model |
| claude-fable-5-1 | capabilities={"batch":{"supported":true},"citations":{"supported":true},"code_execution":{"supported":true},"context_management":{"clear_thinking_20251015":{"supported":true},"clear_tool_uses_20250919":{"supported":true},"compact_20260112":{"supported":true},"supported":true},"effort":{"high":{"supported":true},"low":{"supported":true},"max":{"supported":true},"medium":{"supported":true},"supported":true,"xhigh":{"supported":true}},"image_input":{"supported":true},"pdf_input":{"supported":true},"server_tool... [truncated, 809 total bytes] ; created_at=2026-08-28T00:00:00Z ; deprecated_at=null ; display_name=Claude Fable 5.1 ; lifecycle=active ; line=fable ; max_input_tokens=1000000 ; max_tokens=128000 ; retires_at=null ; type=model |
| claude-opus-5 | capabilities={"batch":{"supported":true},"citations":{"supported":true},"code_execution":{"supported":true},"context_management":{"clear_thinking_20251015":{"supported":true},"clear_tool_uses_20250919":{"supported":true},"compact_20260112":{"supported":true},"supported":true},"effort":{"high":{"supported":true},"low":{"supported":true},"max":{"supported":true},"medium":{"supported":true},"supported":true,"xhigh":{"supported":true}},"image_input":{"supported":true},"pdf_input":{"supported":true},"server_tool... [truncated, 808 total bytes] ; created_at=2026-07-24T00:00:00Z ; deprecated_at=null ; display_name=Claude Opus 5 ; lifecycle=active ; line=opus ; max_input_tokens=1000000 ; max_tokens=128000 ; retires_at=null ; type=model |
| claude-sonnet-5 | capabilities={"batch":{"supported":true},"citations":{"supported":true},"code_execution":{"supported":true},"context_management":{"clear_thinking_20251015":{"supported":true},"clear_tool_uses_20250919":{"supported":true},"compact_20260112":{"supported":true},"supported":true},"effort":{"high":{"supported":true},"low":{"supported":true},"max":{"supported":true},"medium":{"supported":true},"supported":true,"xhigh":{"supported":true}},"image_input":{"supported":true},"pdf_input":{"supported":true},"server_tool... [truncated, 808 total bytes] ; created_at=2026-06-29T00:00:00Z ; deprecated_at=null ; display_name=Claude Sonnet 5 ; lifecycle=active ; line=sonnet ; max_input_tokens=1000000 ; max_tokens=128000 ; retires_at=null ; type=model |
| claude-fable-5 | capabilities={"batch":{"supported":true},"citations":{"supported":true},"code_execution":{"supported":true},"context_management":{"clear_thinking_20251015":{"supported":true},"clear_tool_uses_20250919":{"supported":true},"compact_20260112":{"supported":true},"supported":true},"effort":{"high":{"supported":true},"low":{"supported":true},"max":{"supported":true},"medium":{"supported":true},"supported":true,"xhigh":{"supported":true}},"image_input":{"supported":true},"pdf_input":{"supported":true},"server_tool... [truncated, 809 total bytes] ; created_at=2026-06-07T00:00:00Z ; deprecated_at=null ; display_name=Claude Fable 5 ; lifecycle=active ; line=fable ; max_input_tokens=1000000 ; max_tokens=128000 ; retires_at=null ; type=model |
| claude-opus-4-8 | capabilities={"batch":{"supported":true},"citations":{"supported":true},"code_execution":{"supported":true},"context_management":{"clear_thinking_20251015":{"supported":true},"clear_tool_uses_20250919":{"supported":true},"compact_20260112":{"supported":true},"supported":true},"effort":{"high":{"supported":true},"low":{"supported":true},"max":{"supported":true},"medium":{"supported":true},"supported":true,"xhigh":{"supported":true}},"image_input":{"supported":true},"pdf_input":{"supported":true},"server_tool... [truncated, 808 total bytes] ; created_at=2026-05-28T00:00:00Z ; deprecated_at=null ; display_name=Claude Opus 4.8 ; lifecycle=active ; line=opus ; max_input_tokens=1000000 ; max_tokens=128000 ; retires_at=null ; type=model |
| claude-opus-4-7 | capabilities={"batch":{"supported":true},"citations":{"supported":true},"code_execution":{"supported":true},"context_management":{"clear_thinking_20251015":{"supported":true},"clear_tool_uses_20250919":{"supported":true},"compact_20260112":{"supported":true},"supported":true},"effort":{"high":{"supported":true},"low":{"supported":true},"max":{"supported":true},"medium":{"supported":true},"supported":true,"xhigh":{"supported":true}},"image_input":{"supported":true},"pdf_input":{"supported":true},"server_tool... [truncated, 808 total bytes] ; created_at=2026-04-14T00:00:00Z ; deprecated_at=null ; display_name=Claude Opus 4.7 ; lifecycle=active ; line=opus ; max_input_tokens=1000000 ; max_tokens=128000 ; retires_at=null ; type=model |
| claude-sonnet-4-6 | capabilities={"batch":{"supported":true},"citations":{"supported":true},"code_execution":{"supported":true},"context_management":{"clear_thinking_20251015":{"supported":true},"clear_tool_uses_20250919":{"supported":true},"compact_20260112":{"supported":true},"supported":true},"effort":{"high":{"supported":true},"low":{"supported":true},"max":{"supported":true},"medium":{"supported":true},"supported":true,"xhigh":{"supported":false}},"image_input":{"supported":true},"pdf_input":{"supported":true},"server_too... [truncated, 808 total bytes] ; created_at=2026-02-17T00:00:00Z ; deprecated_at=null ; display_name=Claude Sonnet 4.6 ; lifecycle=active ; line=sonnet ; max_input_tokens=1000000 ; max_tokens=128000 ; retires_at=null ; type=model |
| claude-opus-4-6 | capabilities={"batch":{"supported":true},"citations":{"supported":true},"code_execution":{"supported":true},"context_management":{"clear_thinking_20251015":{"supported":true},"clear_tool_uses_20250919":{"supported":true},"compact_20260112":{"supported":true},"supported":true},"effort":{"high":{"supported":true},"low":{"supported":true},"max":{"supported":true},"medium":{"supported":true},"supported":true,"xhigh":{"supported":false}},"image_input":{"supported":true},"pdf_input":{"supported":true},"server_too... [truncated, 808 total bytes] ; created_at=2026-02-04T00:00:00Z ; deprecated_at=null ; display_name=Claude Opus 4.6 ; lifecycle=active ; line=opus ; max_input_tokens=1000000 ; max_tokens=128000 ; retires_at=null ; type=model |
| claude-opus-4-5-20251101 | capabilities={"batch":{"supported":true},"citations":{"supported":true},"code_execution":{"supported":true},"context_management":{"clear_thinking_20251015":{"supported":true},"clear_tool_uses_20250919":{"supported":true},"compact_20260112":{"supported":false},"supported":true},"effort":{"high":{"supported":true},"low":{"supported":true},"max":{"supported":false},"medium":{"supported":true},"supported":true,"xhigh":{"supported":false}},"image_input":{"supported":true},"pdf_input":{"supported":true},"server_t... [truncated, 811 total bytes] ; created_at=2025-11-24T00:00:00Z ; deprecated_at=null ; display_name=Claude Opus 4.5 ; lifecycle=active ; line=opus ; max_input_tokens=200000 ; max_tokens=64000 ; retires_at=null ; type=model |
| claude-haiku-4-5-20251001 | capabilities={"batch":{"supported":true},"citations":{"supported":true},"code_execution":{"supported":false},"context_management":{"clear_thinking_20251015":{"supported":true},"clear_tool_uses_20250919":{"supported":true},"compact_20260112":{"supported":false},"supported":true},"effort":{"high":{"supported":false},"low":{"supported":false},"max":{"supported":false},"medium":{"supported":false},"supported":false,"xhigh":{"supported":false}},"image_input":{"supported":true},"pdf_input":{"supported":true},"ser... [truncated, 816 total bytes] ; created_at=2025-10-15T00:00:00Z ; deprecated_at=null ; display_name=Claude Haiku 4.5 ; lifecycle=active ; line=haiku ; max_input_tokens=200000 ; max_tokens=64000 ; retires_at=null ; type=model |
| claude-sonnet-4-5-20250929 | capabilities={"batch":{"supported":true},"citations":{"supported":true},"code_execution":{"supported":true},"context_management":{"clear_thinking_20251015":{"supported":true},"clear_tool_uses_20250919":{"supported":true},"compact_20260112":{"supported":false},"supported":true},"effort":{"high":{"supported":false},"low":{"supported":false},"max":{"supported":false},"medium":{"supported":false},"supported":false,"xhigh":{"supported":false}},"image_input":{"supported":true},"pdf_input":{"supported":true},"serv... [truncated, 815 total bytes] ; created_at=2025-09-29T00:00:00Z ; deprecated_at=null ; display_name=Claude Sonnet 4.5 ; lifecycle=active ; line=sonnet ; max_input_tokens=200000 ; max_tokens=64000 ; retires_at=null ; type=model |

### Probes: `claude-3-5-sonnet-20241022`

| # | probe | body extras | outcome | finish | content | usage fields | reasoning text |
|---|-------|-------------|---------|--------|---------|--------------|----------------|
| 1 | default parameters | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-5-sonnet-20241022"},"request_id":"req_011Cg4ofY3K9EtVEyzfQT85G"} | - | - | - | - |
| 2 | temperature=0.00 (declared low end) | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-5-sonnet-20241022"},"request_id":"req_011Cg4ofYco8k2aVWNUmEsi8"} | - | - | - | - |
| 3 | temperature=1.00 (declared high end) | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-5-sonnet-20241022"},"request_id":"req_011Cg4ofZCGTkb6aifQ3YNqM"} | - | - | - | - |
| 4 | temperature=0.50 (mid-range) | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-5-sonnet-20241022"},"request_id":"req_011Cg4ofZuw5QYSTSLSJ9ZKi"} | - | - | - | - |
| 5 | temperature=1.01 (just outside the high end) | - | REJECTED (400): {"type":"error","error":{"type":"invalid_request_error","message":"temperature: range: 0..1"},"request_id":"req_011Cg4ofaTfzxiB8bsCa14tW"} | - | - | - | - |
| 6 | top_p=0.5 | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-5-sonnet-20241022"},"request_id":"req_011Cg4ofbEoTYcP9UsRPCuH5"} | - | - | - | - |
| 7 | frequency_penalty=0.5 | - | NOT APPLICABLE (this wire has no such request field) | - | - | - | - |
| 8 | presence_penalty=0.5 | - | NOT APPLICABLE (this wire has no such request field) | - | - | - | - |
| 9 | stop sequences: 1 | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-5-sonnet-20241022"},"request_id":"req_011Cg4ofbzCG2WaHWqYiR2tr"} | - | - | - | - |
| 10 | stop sequences: 5 | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-5-sonnet-20241022"},"request_id":"req_011Cg4ofcWhcSGWxTAREd9yj"} | - | - | - | - |
| 11 | max_tokens=64 | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-5-sonnet-20241022"},"request_id":"req_011Cg4ofd4CbUmXjReS3Pkr9"} | - | - | - | - |
| 12 | json_object mode (response_format json_object; prompt names JSON) | - | NOT APPLICABLE (Anthropic has no JSON mode) | - | - | - | - |
| 13 | json_schema mode (response_format json_schema; prompt names JSON) | - | NOT APPLICABLE (Anthropic has no JSON mode) | - | - | - | - |

### Probes: `claude-3-5-sonnet-20240620`

| # | probe | body extras | outcome | finish | content | usage fields | reasoning text |
|---|-------|-------------|---------|--------|---------|--------------|----------------|
| 1 | default parameters | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-5-sonnet-20240620"},"request_id":"req_011Cg4ofdawYcZTDDLmnMQWH"} | - | - | - | - |
| 2 | temperature=0.00 (declared low end) | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-5-sonnet-20240620"},"request_id":"req_011Cg4ofe6hU9J2iSeD1XUGb"} | - | - | - | - |
| 3 | temperature=1.00 (declared high end) | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-5-sonnet-20240620"},"request_id":"req_011Cg4ofeay2HQzNMwwHsYK9"} | - | - | - | - |
| 4 | temperature=0.50 (mid-range) | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-5-sonnet-20240620"},"request_id":"req_011Cg4off7yMU5SjvYdUrueb"} | - | - | - | - |
| 5 | temperature=1.01 (just outside the high end) | - | REJECTED (400): {"type":"error","error":{"type":"invalid_request_error","message":"temperature: range: 0..1"},"request_id":"req_011Cg4ofg4GnbQ2mM4tNrFDS"} | - | - | - | - |
| 6 | top_p=0.5 | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-5-sonnet-20240620"},"request_id":"req_011Cg4ofgamijQXLfQNPtNRs"} | - | - | - | - |
| 7 | frequency_penalty=0.5 | - | NOT APPLICABLE (this wire has no such request field) | - | - | - | - |
| 8 | presence_penalty=0.5 | - | NOT APPLICABLE (this wire has no such request field) | - | - | - | - |
| 9 | stop sequences: 1 | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-5-sonnet-20240620"},"request_id":"req_011Cg4ofh7mZdehtT1gonLPs"} | - | - | - | - |
| 10 | stop sequences: 5 | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-5-sonnet-20240620"},"request_id":"req_011Cg4ofhdHJkTRSEvjEqP9T"} | - | - | - | - |
| 11 | max_tokens=64 | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-5-sonnet-20240620"},"request_id":"req_011Cg4ofi9n3KJQTg2cqy2jM"} | - | - | - | - |
| 12 | json_object mode (response_format json_object; prompt names JSON) | - | NOT APPLICABLE (Anthropic has no JSON mode) | - | - | - | - |
| 13 | json_schema mode (response_format json_schema; prompt names JSON) | - | NOT APPLICABLE (Anthropic has no JSON mode) | - | - | - | - |

### Probes: `claude-3-opus-20240229`

| # | probe | body extras | outcome | finish | content | usage fields | reasoning text |
|---|-------|-------------|---------|--------|---------|--------------|----------------|
| 1 | default parameters | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-opus-20240229"},"request_id":"req_011Cg4ofj6ahstDcudCHqQtr"} | - | - | - | - |
| 2 | temperature=0.00 (declared low end) | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-opus-20240229"},"request_id":"req_011Cg4ofjdq9NCDNCBWjBcMs"} | - | - | - | - |
| 3 | temperature=1.00 (declared high end) | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-opus-20240229"},"request_id":"req_011Cg4ofkRx8Uzrisi42Vpne"} | - | - | - | - |
| 4 | temperature=0.50 (mid-range) | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-opus-20240229"},"request_id":"req_011Cg4ofkzBikYqjvveg5kNM"} | - | - | - | - |
| 5 | temperature=1.01 (just outside the high end) | - | REJECTED (400): {"type":"error","error":{"type":"invalid_request_error","message":"temperature: range: 0..1"},"request_id":"req_011Cg4ofmVSfhTQVG7MDvS1z"} | - | - | - | - |
| 6 | top_p=0.5 | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-opus-20240229"},"request_id":"req_011Cg4ofn4BRiEJCQA3YD1YT"} | - | - | - | - |
| 7 | frequency_penalty=0.5 | - | NOT APPLICABLE (this wire has no such request field) | - | - | - | - |
| 8 | presence_penalty=0.5 | - | NOT APPLICABLE (this wire has no such request field) | - | - | - | - |
| 9 | stop sequences: 1 | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-opus-20240229"},"request_id":"req_011Cg4ofncBAWyco9ZHgsrBB"} | - | - | - | - |
| 10 | stop sequences: 5 | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-opus-20240229"},"request_id":"req_011Cg4ofoYEUihSaRG31Hvue"} | - | - | - | - |
| 11 | max_tokens=64 | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-opus-20240229"},"request_id":"req_011Cg4ofpPZxBaBxo5DxszbK"} | - | - | - | - |
| 12 | json_object mode (response_format json_object; prompt names JSON) | - | NOT APPLICABLE (Anthropic has no JSON mode) | - | - | - | - |
| 13 | json_schema mode (response_format json_schema; prompt names JSON) | - | NOT APPLICABLE (Anthropic has no JSON mode) | - | - | - | - |

### Probes: `claude-3-sonnet-20240229`

| # | probe | body extras | outcome | finish | content | usage fields | reasoning text |
|---|-------|-------------|---------|--------|---------|--------------|----------------|
| 1 | default parameters | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-sonnet-20240229"},"request_id":"req_011Cg4ofpt6D4DzqKzuZ9w3H"} | - | - | - | - |
| 2 | temperature=0.00 (declared low end) | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-sonnet-20240229"},"request_id":"req_011Cg4ofqQLsWg5tVnC3nRLr"} | - | - | - | - |
| 3 | temperature=1.00 (declared high end) | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-sonnet-20240229"},"request_id":"req_011Cg4ofqtce62TU9oByCdhh"} | - | - | - | - |
| 4 | temperature=0.50 (mid-range) | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-sonnet-20240229"},"request_id":"req_011Cg4ofrQcrVqXPvZfZ3Twc"} | - | - | - | - |
| 5 | temperature=1.01 (just outside the high end) | - | REJECTED (400): {"type":"error","error":{"type":"invalid_request_error","message":"temperature: range: 0..1"},"request_id":"req_011Cg4ofrvctWS3yfnkbKRvR"} | - | - | - | - |
| 6 | top_p=0.5 | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-sonnet-20240229"},"request_id":"req_011Cg4ofsSdm2eP1rXK44tmJ"} | - | - | - | - |
| 7 | frequency_penalty=0.5 | - | NOT APPLICABLE (this wire has no such request field) | - | - | - | - |
| 8 | presence_penalty=0.5 | - | NOT APPLICABLE (this wire has no such request field) | - | - | - | - |
| 9 | stop sequences: 1 | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-sonnet-20240229"},"request_id":"req_011Cg4oftLCdLso2ELqcjoyf"} | - | - | - | - |
| 10 | stop sequences: 5 | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-sonnet-20240229"},"request_id":"req_011Cg4ofuB48SHT6ufFrfkda"} | - | - | - | - |
| 11 | max_tokens=64 | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-sonnet-20240229"},"request_id":"req_011Cg4ofuk39x9iZAK3BT3dm"} | - | - | - | - |
| 12 | json_object mode (response_format json_object; prompt names JSON) | - | NOT APPLICABLE (Anthropic has no JSON mode) | - | - | - | - |
| 13 | json_schema mode (response_format json_schema; prompt names JSON) | - | NOT APPLICABLE (Anthropic has no JSON mode) | - | - | - | - |

### Probes: `claude-3-haiku-20240307`

| # | probe | body extras | outcome | finish | content | usage fields | reasoning text |
|---|-------|-------------|---------|--------|---------|--------------|----------------|
| 1 | default parameters | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-haiku-20240307"},"request_id":"req_011Cg4ofvGnw9XGURoVYSMHY"} | - | - | - | - |
| 2 | temperature=0.00 (declared low end) | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-haiku-20240307"},"request_id":"req_011Cg4ofvvjSpVsdKemKT5Fn"} | - | - | - | - |
| 3 | temperature=1.00 (declared high end) | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-haiku-20240307"},"request_id":"req_011Cg4ofwUURrMGvWvpQCdz6"} | - | - | - | - |
| 4 | temperature=0.50 (mid-range) | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-haiku-20240307"},"request_id":"req_011Cg4ofx2iHoT4efPQgMAhF"} | - | - | - | - |
| 5 | temperature=1.01 (just outside the high end) | - | REJECTED (400): {"type":"error","error":{"type":"invalid_request_error","message":"temperature: range: 0..1"},"request_id":"req_011Cg4ofxgtQENr4Dw4XR6iw"} | - | - | - | - |
| 6 | top_p=0.5 | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-haiku-20240307"},"request_id":"req_011Cg4ofyaiRk1KJ7JDe4R6X"} | - | - | - | - |
| 7 | frequency_penalty=0.5 | - | NOT APPLICABLE (this wire has no such request field) | - | - | - | - |
| 8 | presence_penalty=0.5 | - | NOT APPLICABLE (this wire has no such request field) | - | - | - | - |
| 9 | stop sequences: 1 | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-haiku-20240307"},"request_id":"req_011Cg4ofz9TL7ozADw6LuJTi"} | - | - | - | - |
| 10 | stop sequences: 5 | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-haiku-20240307"},"request_id":"req_011Cg4ofzghquUcnFfYkTDzD"} | - | - | - | - |
| 11 | max_tokens=64 | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-3-haiku-20240307"},"request_id":"req_011Cg4og1LeGUQHvHRPuzZso"} | - | - | - | - |
| 12 | json_object mode (response_format json_object; prompt names JSON) | - | NOT APPLICABLE (Anthropic has no JSON mode) | - | - | - | - |
| 13 | json_schema mode (response_format json_schema; prompt names JSON) | - | NOT APPLICABLE (Anthropic has no JSON mode) | - | - | - | - |

### Probes: `claude-2.1`

| # | probe | body extras | outcome | finish | content | usage fields | reasoning text |
|---|-------|-------------|---------|--------|---------|--------------|----------------|
| 1 | default parameters | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-2.1"},"request_id":"req_011Cg4og1uNcupzPLJUfJLVe"} | - | - | - | - |
| 2 | temperature=0.00 (declared low end) | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-2.1"},"request_id":"req_011Cg4og2fkgDe6udSQDMegj"} | - | - | - | - |
| 3 | temperature=1.00 (declared high end) | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-2.1"},"request_id":"req_011Cg4og3X6xkkHjXZB2HzqV"} | - | - | - | - |
| 4 | temperature=0.50 (mid-range) | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-2.1"},"request_id":"req_011Cg4og4DXPPtxTGgZq2nxL"} | - | - | - | - |
| 5 | temperature=1.01 (just outside the high end) | - | REJECTED (400): {"type":"error","error":{"type":"invalid_request_error","message":"temperature: range: 0..1"},"request_id":"req_011Cg4og4kWyCVsCUebfa7o3"} | - | - | - | - |
| 6 | top_p=0.5 | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-2.1"},"request_id":"req_011Cg4og5hpWtqfxHrD5K8GE"} | - | - | - | - |
| 7 | frequency_penalty=0.5 | - | NOT APPLICABLE (this wire has no such request field) | - | - | - | - |
| 8 | presence_penalty=0.5 | - | NOT APPLICABLE (this wire has no such request field) | - | - | - | - |
| 9 | stop sequences: 1 | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-2.1"},"request_id":"req_011Cg4og6EpoPrjbfrSPxAXv"} | - | - | - | - |
| 10 | stop sequences: 5 | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-2.1"},"request_id":"req_011Cg4og6mKRfHmuTYzkt62w"} | - | - | - | - |
| 11 | max_tokens=64 | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-2.1"},"request_id":"req_011Cg4og7TVCeXNUDesdeSn3"} | - | - | - | - |
| 12 | json_object mode (response_format json_object; prompt names JSON) | - | NOT APPLICABLE (Anthropic has no JSON mode) | - | - | - | - |
| 13 | json_schema mode (response_format json_schema; prompt names JSON) | - | NOT APPLICABLE (Anthropic has no JSON mode) | - | - | - | - |

### Probes: `claude-2.0`

| # | probe | body extras | outcome | finish | content | usage fields | reasoning text |
|---|-------|-------------|---------|--------|---------|--------------|----------------|
| 1 | default parameters | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-2.0"},"request_id":"req_011Cg4og7xzXACUKXRi9uwA8"} | - | - | - | - |
| 2 | temperature=0.00 (declared low end) | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-2.0"},"request_id":"req_011Cg4og8bCzzTXPKWwM1MY8"} | - | - | - | - |
| 3 | temperature=1.00 (declared high end) | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-2.0"},"request_id":"req_011Cg4og98SrGvf5GufxZALV"} | - | - | - | - |
| 4 | temperature=0.50 (mid-range) | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-2.0"},"request_id":"req_011Cg4og9w4wJ9AuLVY7RVnQ"} | - | - | - | - |
| 5 | temperature=1.01 (just outside the high end) | - | REJECTED (400): {"type":"error","error":{"type":"invalid_request_error","message":"temperature: range: 0..1"},"request_id":"req_011Cg4ogATpBLxjr6t3c2WjS"} | - | - | - | - |
| 6 | top_p=0.5 | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-2.0"},"request_id":"req_011Cg4ogBLAPr5CWxYADrXCp"} | - | - | - | - |
| 7 | frequency_penalty=0.5 | - | NOT APPLICABLE (this wire has no such request field) | - | - | - | - |
| 8 | presence_penalty=0.5 | - | NOT APPLICABLE (this wire has no such request field) | - | - | - | - |
| 9 | stop sequences: 1 | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-2.0"},"request_id":"req_011Cg4ogBruuA1gXRQA3AAgK"} | - | - | - | - |
| 10 | stop sequences: 5 | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-2.0"},"request_id":"req_011Cg4ogCSdTCn9f3SqevBuH"} | - | - | - | - |
| 11 | max_tokens=64 | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-2.0"},"request_id":"req_011Cg4ogCx8mJ4dLQEJakWki"} | - | - | - | - |
| 12 | json_object mode (response_format json_object; prompt names JSON) | - | NOT APPLICABLE (Anthropic has no JSON mode) | - | - | - | - |
| 13 | json_schema mode (response_format json_schema; prompt names JSON) | - | NOT APPLICABLE (Anthropic has no JSON mode) | - | - | - | - |

### Probes: `claude-instant-1.2`

| # | probe | body extras | outcome | finish | content | usage fields | reasoning text |
|---|-------|-------------|---------|--------|---------|--------------|----------------|
| 1 | default parameters | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-instant-1.2"},"request_id":"req_011Cg4ogDTtrzhR24J74s9om"} | - | - | - | - |
| 2 | temperature=0.00 (declared low end) | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-instant-1.2"},"request_id":"req_011Cg4ogE28wwXR5rNpXBSTn"} | - | - | - | - |
| 3 | temperature=1.00 (declared high end) | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-instant-1.2"},"request_id":"req_011Cg4ogEbraFMMi3Png1g2V"} | - | - | - | - |
| 4 | temperature=0.50 (mid-range) | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-instant-1.2"},"request_id":"req_011Cg4ogF7N3wSdG1fQm8CBN"} | - | - | - | - |
| 5 | temperature=1.01 (just outside the high end) | - | REJECTED (400): {"type":"error","error":{"type":"invalid_request_error","message":"temperature: range: 0..1"},"request_id":"req_011Cg4ogFqX7TiqSWfsJjDN8"} | - | - | - | - |
| 6 | top_p=0.5 | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-instant-1.2"},"request_id":"req_011Cg4ogGet5byxNaBRTQXhS"} | - | - | - | - |
| 7 | frequency_penalty=0.5 | - | NOT APPLICABLE (this wire has no such request field) | - | - | - | - |
| 8 | presence_penalty=0.5 | - | NOT APPLICABLE (this wire has no such request field) | - | - | - | - |
| 9 | stop sequences: 1 | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-instant-1.2"},"request_id":"req_011Cg4ogHErHhLNHtSa77cb5"} | - | - | - | - |
| 10 | stop sequences: 5 | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-instant-1.2"},"request_id":"req_011Cg4ogHkcrH79W7b3GpLcN"} | - | - | - | - |
| 11 | max_tokens=64 | - | REJECTED (404): {"type":"error","error":{"type":"not_found_error","message":"model: claude-instant-1.2"},"request_id":"req_011Cg4ogJHc5hwjeyB8JnJQY"} | - | - | - | - |
| 12 | json_object mode (response_format json_object; prompt names JSON) | - | NOT APPLICABLE (Anthropic has no JSON mode) | - | - | - | - |
| 13 | json_schema mode (response_format json_schema; prompt names JSON) | - | NOT APPLICABLE (Anthropic has no JSON mode) | - | - | - | - |

### Probes: `claude-haiku-5-5`

| # | probe | body extras | outcome | finish | content | usage fields | reasoning text |
|---|-------|-------------|---------|--------|---------|--------------|----------------|
| 1 | default parameters | - | ACCEPTED (200) | end_turn | non-empty | cache_creation.ephemeral_1h_input_tokens, cache_creation.ephemeral_5m_input_tokens, cache_creation_input_tokens, cache_read_input_tokens, inference_geo, input_tokens, output_tokens, output_tokens_details.thinking_tokens, service_tier, speed | absent |
| 2 | temperature=0.00 (declared low end) | - | REJECTED (400): {"type":"error","error":{"type":"invalid_request_error","message":"`temperature` is deprecated for this model."},"request_id":"req_011Cg4ogMycuLvutmsKBBcM3"} | - | - | - | - |
| 3 | temperature=1.00 (declared high end) | - | ACCEPTED (200) | end_turn | non-empty | cache_creation.ephemeral_1h_input_tokens, cache_creation.ephemeral_5m_input_tokens, cache_creation_input_tokens, cache_read_input_tokens, inference_geo, input_tokens, output_tokens, output_tokens_details.thinking_tokens, service_tier, speed | absent |
| 4 | temperature=0.50 (mid-range) | - | REJECTED (400): {"type":"error","error":{"type":"invalid_request_error","message":"`temperature` is deprecated for this model."},"request_id":"req_011Cg4ogRsm44Vt5m1Q7aPw4"} | - | - | - | - |
| 5 | temperature=1.01 (just outside the high end) | - | REJECTED (400): {"type":"error","error":{"type":"invalid_request_error","message":"temperature: range: 0..1"},"request_id":"req_011Cg4ogSTVc7UB9d6WcaL6s"} | - | - | - | - |
| 6 | top_p=0.5 | - | REJECTED (400): {"type":"error","error":{"type":"invalid_request_error","message":"`top_p` is deprecated for this model."},"request_id":"req_011Cg4ogT1UpAeFneVMyf5My"} | - | - | - | - |
| 7 | frequency_penalty=0.5 | - | NOT APPLICABLE (this wire has no such request field) | - | - | - | - |
| 8 | presence_penalty=0.5 | - | NOT APPLICABLE (this wire has no such request field) | - | - | - | - |
| 9 | stop sequences: 1 | - | ACCEPTED (200) | end_turn | non-empty | cache_creation.ephemeral_1h_input_tokens, cache_creation.ephemeral_5m_input_tokens, cache_creation_input_tokens, cache_read_input_tokens, inference_geo, input_tokens, output_tokens, output_tokens_details.thinking_tokens, service_tier, speed | absent |
| 10 | stop sequences: 5 | - | ACCEPTED (200) | end_turn | non-empty | cache_creation.ephemeral_1h_input_tokens, cache_creation.ephemeral_5m_input_tokens, cache_creation_input_tokens, cache_read_input_tokens, inference_geo, input_tokens, output_tokens, output_tokens_details.thinking_tokens, service_tier, speed | absent |
| 11 | max_tokens=64 | - | ACCEPTED (200) | end_turn | non-empty | cache_creation.ephemeral_1h_input_tokens, cache_creation.ephemeral_5m_input_tokens, cache_creation_input_tokens, cache_read_input_tokens, inference_geo, input_tokens, output_tokens, output_tokens_details.thinking_tokens, service_tier, speed | absent |
| 12 | json_object mode (response_format json_object; prompt names JSON) | - | NOT APPLICABLE (Anthropic has no JSON mode) | - | - | - | - |
| 13 | json_schema mode (response_format json_schema; prompt names JSON) | - | NOT APPLICABLE (Anthropic has no JSON mode) | - | - | - | - |

### Quota-shaped errors observed

none observed (a quota error cannot be provoked on demand)

## DeepSeek

- base URL: `https://api.deepseek.com/v1`
- chat endpoint: `POST https://api.deepseek.com/v1/chat/completions`; credential header `Authorization` (name only, value never printed)
- models endpoint: `GET https://api.deepseek.com/v1/models`
- declared temperature range: 0..2
- every call carries `max_tokens` = 256 and a 120 s timeout; redirects are never followed

### Models listing

| model id | limit / capability fields (as received) |
|---|---|
| deepseek-flash | api_capabilities={"anthropic_messages":{"system_prompt_update":"in-history"}} ; context_window=1048576 ; effort={"default_level":"high","supported_levels":["low","high","max"]} ; input_modalities=["text","image"] ; max_output_tokens=393216 ; name=DeepSeek-V4.1-Flash ; object=model ; output_modalities=["text"] ; owned_by=deepseek |
| deepseek-v4-pro | api_capabilities={"anthropic_messages":{"system_prompt_update":"leading-only"}} ; context_window=1048576 ; effort={"default_level":"high","supported_levels":["low","high","max"]} ; input_modalities=["text"] ; max_output_tokens=393216 ; name=DeepSeek-V4-Pro ; object=model ; output_modalities=["text"] ; owned_by=deepseek |

### Probes: `deepseek-chat`

| # | probe | body extras | outcome | finish | content | usage fields | reasoning text |
|---|-------|-------------|---------|--------|---------|--------------|----------------|
| 1 | default parameters | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 2 | temperature=0.00 (declared low end) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 3 | temperature=2.00 (declared high end) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 4 | temperature=0.50 (mid-range) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 5 | temperature=2.01 (just outside the high end) | - | REJECTED (400): {"error":{"message":"Invalid temperature value, the valid range of temperature is [0, 2] (request_id: d2b4ae21-97b5-4e29-8122-0a6780f8632e)","type":"invalid_request_error","param":null,"code":"invalid_request_error"}} | - | - | - | - |
| 6 | top_p=0.5 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 7 | frequency_penalty=0.5 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 8 | presence_penalty=0.5 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 9 | stop sequences: 1 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 10 | stop sequences: 5 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 11 | max_tokens=64 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 12 | json_object mode (response_format json_object; prompt names JSON) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 13 | json_schema mode (response_format json_schema; prompt names JSON) | - | REJECTED (400): {"error":{"message":"This response_format type is unavailable now (request_id: a01a2090-c59d-425f-b832-2620e0fb9893)","type":"invalid_request_error","param":null,"code":"invalid_request_error"}} | - | - | - | - |

### Probes: `deepseek-coder`

| # | probe | body extras | outcome | finish | content | usage fields | reasoning text |
|---|-------|-------------|---------|--------|---------|--------------|----------------|
| 1 | default parameters | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 2 | temperature=0.00 (declared low end) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 3 | temperature=2.00 (declared high end) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 4 | temperature=0.50 (mid-range) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 5 | temperature=2.01 (just outside the high end) | - | REJECTED (400): {"error":{"message":"Invalid temperature value, the valid range of temperature is [0, 2] (request_id: a2287b4a-d638-40cb-a424-41ce4d813e8b)","type":"invalid_request_error","param":null,"code":"invalid_request_error"}} | - | - | - | - |
| 6 | top_p=0.5 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 7 | frequency_penalty=0.5 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 8 | presence_penalty=0.5 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 9 | stop sequences: 1 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 10 | stop sequences: 5 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 11 | max_tokens=64 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 12 | json_object mode (response_format json_object; prompt names JSON) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 13 | json_schema mode (response_format json_schema; prompt names JSON) | - | REJECTED (400): {"error":{"message":"This response_format type is unavailable now (request_id: 3f17569e-e3d0-43a5-8f26-677b827ca86f)","type":"invalid_request_error","param":null,"code":"invalid_request_error"}} | - | - | - | - |

### Probes: `deepseek-flash`

| # | probe | body extras | outcome | finish | content | usage fields | reasoning text |
|---|-------|-------------|---------|--------|---------|--------------|----------------|
| 1 | default parameters | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.reasoning_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | present |
| 2 | temperature=0.00 (declared low end) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.reasoning_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | present |
| 3 | temperature=2.00 (declared high end) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.reasoning_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | present |
| 4 | temperature=0.50 (mid-range) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.reasoning_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | present |
| 5 | temperature=2.01 (just outside the high end) | - | REJECTED (400): {"error":{"message":"Invalid temperature value, the valid range of temperature is [0, 2] (request_id: 88985243-5abe-4261-8a37-6e4b2fc5bd4c)","type":"invalid_request_error","param":null,"code":"invalid_request_error"}} | - | - | - | - |
| 6 | top_p=0.5 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.reasoning_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | present |
| 7 | frequency_penalty=0.5 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.reasoning_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | present |
| 8 | presence_penalty=0.5 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.reasoning_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | present |
| 9 | stop sequences: 1 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.reasoning_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | present |
| 10 | stop sequences: 5 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.reasoning_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | present |
| 11 | max_tokens=64 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.reasoning_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | present |
| 12 | json_object mode (response_format json_object; prompt names JSON) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.reasoning_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | present |
| 13 | json_schema mode (response_format json_schema; prompt names JSON) | - | REJECTED (400): {"error":{"message":"This response_format type is unavailable now (request_id: 8fc66b6c-286f-497a-b6e2-3e3ab6f99803)","type":"invalid_request_error","param":null,"code":"invalid_request_error"}} | - | - | - | - |

### Quota-shaped errors observed

none observed (a quota error cannot be provoked on demand)

## Kimi

- base URL: `https://api.moonshot.ai/v1`
- chat endpoint: `POST https://api.moonshot.ai/v1/chat/completions`; credential header `Authorization` (name only, value never printed)
- models endpoint: `GET https://api.moonshot.ai/v1/models`
- declared temperature range: 1..1
- every call carries `max_tokens` = 256 and a 120 s timeout; redirects are never followed
- body extra on every acceptance probe except `default thinking`: `{"thinking":{"type":"disabled"}}`

### Models listing

| model id | limit / capability fields (as received) |
|---|---|
| kimi-k2.7-code-highspeed | context_length=262144 ; created=1791797366 ; object=model ; owned_by=moonshot ; parent= ; permission=[{"created":0,"group":"moonshot","id":"","is_blocking":false,"object":"","organization":"moonshot"}] ; root= ; supports_image_in=true ; supports_reasoning=true ; supports_video_in=true |
| kimi-k2.7-code | context_length=262144 ; created=1791797366 ; object=model ; owned_by=moonshot ; parent= ; permission=[{"created":0,"group":"moonshot","id":"","is_blocking":false,"object":"","organization":"moonshot"}] ; root= ; supports_image_in=true ; supports_reasoning=true ; supports_video_in=true |
| kimi-k3 | context_length=1048576 ; created=1791797366 ; object=model ; owned_by=moonshot ; parent= ; permission=[{"created":0,"group":"moonshot","id":"","is_blocking":false,"object":"","organization":"moonshot"}] ; reasoning_efforts={"default_effort":"max","support":true,"valid_efforts":["low","high","max"]} ; root= ; supports_dynamic_tools=true ; supports_image_in=true ; supports_reasoning=true ; supports_thinking_type=only ; supports_video_in=true ; think_efforts={"default_effort":"max","support":true,"valid_efforts":["low","high","max"]} |
| kimi-k2.6 | context_length=262144 ; created=1791797366 ; object=model ; owned_by=moonshot ; parent= ; permission=[{"created":0,"group":"moonshot","id":"","is_blocking":false,"object":"","organization":"moonshot"}] ; root= ; supports_image_in=true ; supports_reasoning=true ; supports_video_in=true |

### Probes: `kimi-k3`

| # | probe | body extras | outcome | finish | content | usage fields | reasoning text |
|---|-------|-------------|---------|--------|---------|--------------|----------------|
| 1 | default parameters | `{"thinking":{"type":"disabled"}}` | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_tokens, prompt_tokens_details.cache_write_tokens, total_tokens | absent |
| 2 | default thinking (the thinking body extra is NOT applied) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.reasoning_tokens, prompt_tokens, prompt_tokens_details.cache_write_tokens, total_tokens | present |
| 3 | temperature=1.00 (declared low end = declared high end) | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid temperature: only 0.6 is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 4 | temperature=0.50 (mid-range) | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid temperature: only 0.6 is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 5 | temperature=1.01 (just outside the high end) | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid temperature: only 0.6 is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 6 | top_p=0.5 | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid top_p: only 0.95 is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 7 | frequency_penalty=0.5 | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid frequency_penalty: only 0 is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 8 | presence_penalty=0.5 | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid presence_penalty: only 0 is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 9 | stop sequences: 1 | `{"thinking":{"type":"disabled"}}` | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_tokens, prompt_tokens_details.cache_write_tokens, total_tokens | absent |
| 10 | stop sequences: 5 | `{"thinking":{"type":"disabled"}}` | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_tokens, prompt_tokens_details.cache_write_tokens, total_tokens | absent |
| 11 | max_tokens=64 | `{"thinking":{"type":"disabled"}}` | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_tokens, prompt_tokens_details.cache_write_tokens, total_tokens | absent |
| 12 | json_object mode (response_format json_object; prompt names JSON) | `{"thinking":{"type":"disabled"}}` | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_tokens, prompt_tokens_details.cache_write_tokens, total_tokens | absent |
| 13 | json_schema mode (response_format json_schema; prompt names JSON) | `{"thinking":{"type":"disabled"}}` | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_tokens, prompt_tokens_details.cache_write_tokens, total_tokens | absent |

### Probes: `kimi-k2.6`

| # | probe | body extras | outcome | finish | content | usage fields | reasoning text |
|---|-------|-------------|---------|--------|---------|--------------|----------------|
| 1 | default parameters | `{"thinking":{"type":"disabled"}}` | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.reasoning_tokens, prompt_tokens, total_tokens | absent |
| 2 | default thinking (the thinking body extra is NOT applied) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.reasoning_tokens, prompt_tokens, total_tokens | present |
| 3 | temperature=1.00 (declared low end = declared high end) | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid temperature: only 0.6 is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 4 | temperature=0.50 (mid-range) | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid temperature: only 0.6 is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 5 | temperature=1.01 (just outside the high end) | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid temperature: only 0.6 is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 6 | top_p=0.5 | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid top_p: only 0.95 is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 7 | frequency_penalty=0.5 | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid frequency_penalty: only 0 is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 8 | presence_penalty=0.5 | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid presence_penalty: only 0 is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 9 | stop sequences: 1 | `{"thinking":{"type":"disabled"}}` | ACCEPTED (200) | stop | non-empty | cached_tokens, completion_tokens, completion_tokens_details.reasoning_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 10 | stop sequences: 5 | `{"thinking":{"type":"disabled"}}` | ACCEPTED (200) | stop | non-empty | cached_tokens, completion_tokens, completion_tokens_details.reasoning_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 11 | max_tokens=64 | `{"thinking":{"type":"disabled"}}` | ACCEPTED (200) | stop | non-empty | cached_tokens, completion_tokens, completion_tokens_details.reasoning_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 12 | json_object mode (response_format json_object; prompt names JSON) | `{"thinking":{"type":"disabled"}}` | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.reasoning_tokens, prompt_tokens, total_tokens | absent |
| 13 | json_schema mode (response_format json_schema; prompt names JSON) | `{"thinking":{"type":"disabled"}}` | ACCEPTED (200) | stop | non-empty | cached_tokens, completion_tokens, completion_tokens_details.reasoning_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |

### Probes: `kimi-k2.7-code`

| # | probe | body extras | outcome | finish | content | usage fields | reasoning text |
|---|-------|-------------|---------|--------|---------|--------------|----------------|
| 1 | default parameters | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid thinking: only type=enabled is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 2 | default thinking (the thinking body extra is NOT applied) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.reasoning_tokens, prompt_tokens, total_tokens | present |
| 3 | temperature=1.00 (declared low end = declared high end) | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid thinking: only type=enabled is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 4 | temperature=0.50 (mid-range) | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid thinking: only type=enabled is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 5 | temperature=1.01 (just outside the high end) | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid thinking: only type=enabled is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 6 | top_p=0.5 | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid thinking: only type=enabled is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 7 | frequency_penalty=0.5 | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid thinking: only type=enabled is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 8 | presence_penalty=0.5 | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid thinking: only type=enabled is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 9 | stop sequences: 1 | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid thinking: only type=enabled is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 10 | stop sequences: 5 | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid thinking: only type=enabled is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 11 | max_tokens=64 | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid thinking: only type=enabled is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 12 | json_object mode (response_format json_object; prompt names JSON) | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid thinking: only type=enabled is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 13 | json_schema mode (response_format json_schema; prompt names JSON) | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid thinking: only type=enabled is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |

### Probes: `kimi-k2.7-code-highspeed`

| # | probe | body extras | outcome | finish | content | usage fields | reasoning text |
|---|-------|-------------|---------|--------|---------|--------------|----------------|
| 1 | default parameters | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid thinking: only type=enabled is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 2 | default thinking (the thinking body extra is NOT applied) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.reasoning_tokens, prompt_tokens, total_tokens | present |
| 3 | temperature=1.00 (declared low end = declared high end) | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid thinking: only type=enabled is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 4 | temperature=0.50 (mid-range) | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid thinking: only type=enabled is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 5 | temperature=1.01 (just outside the high end) | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid thinking: only type=enabled is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 6 | top_p=0.5 | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid thinking: only type=enabled is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 7 | frequency_penalty=0.5 | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid thinking: only type=enabled is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 8 | presence_penalty=0.5 | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid thinking: only type=enabled is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 9 | stop sequences: 1 | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid thinking: only type=enabled is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 10 | stop sequences: 5 | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid thinking: only type=enabled is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 11 | max_tokens=64 | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid thinking: only type=enabled is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 12 | json_object mode (response_format json_object; prompt names JSON) | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid thinking: only type=enabled is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |
| 13 | json_schema mode (response_format json_schema; prompt names JSON) | `{"thinking":{"type":"disabled"}}` | REJECTED (400): {"error":{"message":"invalid thinking: only type=enabled is allowed for this model","type":"invalid_request_error"}} | - | - | - | - |

### Quota-shaped errors observed

none observed (a quota error cannot be provoked on demand)

## Qwen

- base URL: `https://dashscope-intl.aliyuncs.com/compatible-mode/v1`
- chat endpoint: `POST https://dashscope-intl.aliyuncs.com/compatible-mode/v1/chat/completions`; credential header `Authorization` (name only, value never printed)
- models endpoint: `GET https://dashscope-intl.aliyuncs.com/compatible-mode/v1/models`
- declared temperature range: 0..1.99
- every call carries `max_tokens` = 256 and a 120 s timeout; redirects are never followed

### Models listing

| model id | limit / capability fields (as received) |
|---|---|
| qwen3.8-omni-flash-realtime | created=1792072989 ; object=model ; owned_by=system |
| glm-5.3-prime | created=1790612978 ; object=model ; owned_by=system |
| qwen-audio-3.1-realtime-plus | created=1791774727 ; object=model ; owned_by=system |
| qwen3.8-omni-flash | created=1791894785 ; object=model ; owned_by=system |
| qwen3.8-livetranslate-flash-realtime | created=1791774729 ; object=model ; owned_by=system |
| glm-5.3 | created=1789525989 ; object=model ; owned_by=system |
| deepseek-v4.1-flash | created=1789453090 ; object=model ; owned_by=system |
| qwen3.8-max-0902 | created=1788312760 ; object=model ; owned_by=system |
| kimi/kimi-k3 | created=1791966238 ; object=model ; owned_by=system |
| qwen3.8-flash | created=1787761130 ; object=model ; owned_by=system |
| kimi-k3 | created=1787645029 ; object=model ; owned_by=system |
| qwen3.8-27b | created=1787712425 ; object=model ; owned_by=system |
| ZHIPU/GLM-5.3 | created=1787029756 ; object=model ; owned_by=system |
| deepseek-v4-pro-0813 | created=1789378036 ; object=model ; owned_by=system |
| qwen3.8-2.4t-a95b | created=1787721476 ; object=model ; owned_by=system |
| qwen3.7-text-embedding | created=1786435205 ; object=model ; owned_by=system |
| qwen-image-3.0-pro | created=1790578335 ; object=model ; owned_by=system |
| qwen-image-3.0 | created=1790578248 ; object=model ; owned_by=system |
| qwen3.8-max | created=1786098182 ; object=model ; owned_by=system |
| deepseek-v4-flash-0731 | created=1788486680 ; object=model ; owned_by=system |
| qwen-audio-3.0-asr-flash | created=1785423064 ; object=model ; owned_by=system |
| qwen3.7-flash-2026-07-15 | created=1785147367 ; object=model ; owned_by=system |
| qwen3.7-flash | created=1785750420 ; object=model ; owned_by=system |
| glm-5.2-fast-preview | created=1788847565 ; object=model ; owned_by=system |
| kimi-k2.7-code | created=1788363170 ; object=model ; owned_by=system |
| glm-5.2 | created=1782886358 ; object=model ; owned_by=system |
| qwen-image-2.0-pro-2026-06-22 | created=1782371883 ; object=model ; owned_by=system |
| qwen3.7-max-2026-06-08 | created=1781073628 ; object=model ; owned_by=system |
| qwen3.7-plus-2026-05-26 | created=1786372125 ; object=model ; owned_by=system |
| qwen3.7-plus | created=1786372099 ; object=model ; owned_by=system |
| glm-5.1 | created=1782217829 ; object=model ; owned_by=system |
| qwen3.7-max-2026-05-17 | created=1781491466 ; object=model ; owned_by=system |
| qwen3.7-max-preview | created=1779702409 ; object=model ; owned_by=system |
| qwen3.7-max-2026-05-20 | created=1781491364 ; object=model ; owned_by=system |
| qwen3.7-max | created=1786420843 ; object=model ; owned_by=system |
| qwen3.5-livetranslate-flash-realtime-2026-05-19 | created=1791774726 ; object=model ; owned_by=system |
| qwen3.5-livetranslate-flash-realtime | created=1791774725 ; object=model ; owned_by=system |
| deepseek-v4-pro | created=1785750167 ; object=model ; owned_by=system |
| qwen-image-2.0-pro-2026-04-22 | created=1784029480 ; object=model ; owned_by=system |
| qwen3.6-27b | created=1782207240 ; object=model ; owned_by=system |
| qwen3.5-plus-2026-04-20 | created=1791808081 ; object=model ; owned_by=system |
| qwen3.6-35b-a3b | created=1778725927 ; object=model ; owned_by=system |
| qwen3.6-flash | created=1785750067 ; object=model ; owned_by=system |
| qwen3.6-flash-2026-04-16 | created=1779992807 ; object=model ; owned_by=system |
| qwen3.5-omni-plus-realtime-2026-03-15 | created=1780984581 ; object=model ; owned_by=system |
| qwen3.5-omni-plus-realtime | created=1790647999 ; object=model ; owned_by=system |
| qwen3.5-omni-plus-2026-03-15 | created=1790648047 ; object=model ; owned_by=system |
| qwen3.5-omni-plus | created=1779438700 ; object=model ; owned_by=system |
| qwen3.5-omni-flash-realtime-2026-03-15 | created=1780984108 ; object=model ; owned_by=system |
| qwen3.5-omni-flash-realtime | created=1790606639 ; object=model ; owned_by=system |
| qwen3.5-omni-flash-2026-03-15 | created=1775748316 ; object=model ; owned_by=system |
| qwen3.5-omni-flash | created=1775748281 ; object=model ; owned_by=system |
| qwen3.6-plus-2026-04-02 | created=1789540632 ; object=model ; owned_by=system |
| qwen3.6-plus | created=1789541156 ; object=model ; owned_by=system |
| wan2.7-image-pro | created=1782100407 ; object=model ; owned_by=system |
| wan2.7-image | created=1782099945 ; object=model ; owned_by=system |
| qwen-image-2.0-2026-03-03 | created=1784029530 ; object=model ; owned_by=system |
| qwen-image-2.0-pro | created=1789645429 ; object=model ; owned_by=system |
| qwen-image-2.0-pro-2026-03-03 | created=1784029510 ; object=model ; owned_by=system |
| qwen-image-2.0 | created=1789612984 ; object=model ; owned_by=system |
| qwen3.5-flash-2026-02-23 | created=1780631532 ; object=model ; owned_by=system |
| qwen3.5-flash | created=1785750474 ; object=model ; owned_by=system |
| qwen3.5-122b-a10b | created=1785845568 ; object=model ; owned_by=system |
| qwen3.5-35b-a3b | created=1782135921 ; object=model ; owned_by=system |
| qwen3.5-27b | created=1773550369 ; object=model ; owned_by=system |
| qwen3.5-397b-a17b | created=1789366788 ; object=model ; owned_by=system |
| qwen3.5-plus-2026-02-15 | created=1791808043 ; object=model ; owned_by=system |
| qwen3.5-plus | created=1791807948 ; object=model ; owned_by=system |
| qwen-plus-character | created=1775811400 ; object=model ; owned_by=system |
| qwen-flash-character | created=1775811437 ; object=model ; owned_by=system |
| qwen-flash | created=1785750213 ; object=model ; owned_by=system |
| z-image-turbo | created=1766371140 ; object=model ; owned_by=system |
| qwen3-livetranslate-flash-2025-12-01 | created=1773991661 ; object=model ; owned_by=system |
| qwen3-livetranslate-flash | created=1764737072 ; object=model ; owned_by=system |
| qwen-mt-lite | created=1764582262 ; object=model ; owned_by=system |
| qwen-plus-2025-12-01 | created=1770026791 ; object=model ; owned_by=system |
| qwen-vl-ocr-2025-11-20 | created=1770024610 ; object=model ; owned_by=system |
| qwen-mt-flash | created=1762433201 ; object=model ; owned_by=system |
| ccai-pro | created=1762328688 ; object=model ; owned_by=system |
| tongyi-tingwu-slp | created=1762328573 ; object=model ; owned_by=system |
| qwen3-asr-flash-realtime | created=1782367385 ; object=model ; owned_by=system |
| text-embedding-v4 | created=1760428930 ; object=model ; owned_by=system |
| qwen3-tts-flash | created=1767618047 ; object=model ; owned_by=system |
| qwen3-tts-flash-realtime | created=1770800230 ; object=model ; owned_by=system |
| qwen3-omni-flash | created=1761550502 ; object=model ; owned_by=system |
| qwen3-omni-flash-realtime-2025-09-15 | created=1770800291 ; object=model ; owned_by=system |
| qwen3-s2s-flash-realtime | created=1760422451 ; object=model ; owned_by=system |
| text-embedding-v3 | created=1761552197 ; object=model ; owned_by=system |
| qwen-plus-latest | created=1760421728 ; object=model ; owned_by=system |
| qwen-plus-2025-01-25 | created=1770026563 ; object=model ; owned_by=system |
| qwq-plus-2025-03-05 | created=1760421663 ; object=model ; owned_by=system |
| qwen-mt-plus | created=1760421630 ; object=model ; owned_by=system |
| qwen-plus-2025-04-28 | created=1770015418 ; object=model ; owned_by=system |
| qwen-plus-2025-07-14 | created=1760413462 ; object=model ; owned_by=system |
| qwen3-coder-flash | created=1778725608 ; object=model ; owned_by=system |
| qwen3-vl-plus | created=1777534314 ; object=model ; owned_by=system |
| qwen2-7b-instruct | created=1755780073 ; object=model ; owned_by=system |
| qwen-max | created=1779934703 ; object=model ; owned_by=system |
| qwen-plus | created=1785750376 ; object=model ; owned_by=system |

### Probes: `qwen-plus`

| # | probe | body extras | outcome | finish | content | usage fields | reasoning text |
|---|-------|-------------|---------|--------|---------|--------------|----------------|
| 1 | default parameters | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 2 | temperature=0.00 (declared low end) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 3 | temperature=1.99 (declared high end) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 4 | temperature=0.50 (mid-range) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 5 | temperature=2.00 (just outside the high end) | - | REJECTED (400): {"error":{"message":"<400> InternalError.Algo.InvalidParameter: Temperature should be in [0.0, 2.0)","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-fd88ea3a-2b71-9c0c-9cdb-8d41726bf596","request_id":"fd88ea3a-2b71-9c0c-9cdb-8d41726bf596"} | - | - | - | - |
| 6 | top_p=0.5 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 7 | frequency_penalty=0.5 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 8 | presence_penalty=0.5 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 9 | stop sequences: 1 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 10 | stop sequences: 5 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 11 | max_tokens=64 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 12 | json_object mode (response_format json_object; prompt names JSON) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |
| 13 | json_schema mode (response_format json_schema; prompt names JSON) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, total_tokens | absent |

### Probes: `qwen-turbo`

| # | probe | body extras | outcome | finish | content | usage fields | reasoning text |
|---|-------|-------------|---------|--------|---------|--------------|----------------|
| 1 | default parameters | - | REJECTED (429): {"error":{"message":"You have exceeded your current request limit. For details, see: https://www.alibabacloud.com/help/en/model-studio/error-code#rate-limit","type":"limit_requests","param":null,"code":"limit_requests"},"request_id":"f77c028a-da52-98f1-96dc-a4988236c23b"} | - | - | - | - |
| 2 | temperature=0.00 (declared low end) | - | REJECTED (429): {"error":{"message":"You have exceeded your current request limit. For details, see: https://www.alibabacloud.com/help/en/model-studio/error-code#rate-limit","type":"limit_requests","param":null,"code":"limit_requests"},"request_id":"7c331827-0ec3-9512-8c45-0f60ce8606a6"} | - | - | - | - |
| 3 | temperature=1.99 (declared high end) | - | REJECTED (429): {"error":{"message":"You have exceeded your current request limit. For details, see: https://www.alibabacloud.com/help/en/model-studio/error-code#rate-limit","type":"limit_requests","param":null,"code":"limit_requests"},"request_id":"9c80c694-daf8-9ead-a105-368154f21735"} | - | - | - | - |
| 4 | temperature=0.50 (mid-range) | - | REJECTED (429): {"error":{"message":"You have exceeded your current request limit. For details, see: https://www.alibabacloud.com/help/en/model-studio/error-code#rate-limit","type":"limit_requests","param":null,"code":"limit_requests"},"request_id":"fca324d9-d9c6-9e8c-96a5-83658e93e658"} | - | - | - | - |
| 5 | temperature=2.00 (just outside the high end) | - | REJECTED (429): {"error":{"message":"You have exceeded your current request limit. For details, see: https://www.alibabacloud.com/help/en/model-studio/error-code#rate-limit","type":"limit_requests","param":null,"code":"limit_requests"},"request_id":"bc3668e6-7516-97d6-aeb1-df54c4f8f58f"} | - | - | - | - |
| 6 | top_p=0.5 | - | REJECTED (429): {"error":{"message":"You have exceeded your current request limit. For details, see: https://www.alibabacloud.com/help/en/model-studio/error-code#rate-limit","type":"limit_requests","param":null,"code":"limit_requests"},"request_id":"a4230e29-cc70-9fa4-89e2-3a3c185cf81b"} | - | - | - | - |
| 7 | frequency_penalty=0.5 | - | REJECTED (429): {"error":{"message":"You have exceeded your current request limit. For details, see: https://www.alibabacloud.com/help/en/model-studio/error-code#rate-limit","type":"limit_requests","param":null,"code":"limit_requests"},"request_id":"e60c6faf-a7b0-9800-bd07-745c8a8972ff"} | - | - | - | - |
| 8 | presence_penalty=0.5 | - | REJECTED (429): {"error":{"message":"You have exceeded your current request limit. For details, see: https://www.alibabacloud.com/help/en/model-studio/error-code#rate-limit","type":"limit_requests","param":null,"code":"limit_requests"},"request_id":"fd627a7d-d09e-9164-b858-30be256fa899"} | - | - | - | - |
| 9 | stop sequences: 1 | - | REJECTED (429): {"error":{"message":"You have exceeded your current request limit. For details, see: https://www.alibabacloud.com/help/en/model-studio/error-code#rate-limit","type":"limit_requests","param":null,"code":"limit_requests"},"request_id":"696ece13-8e38-93a9-ba6c-2e47babe97bd"} | - | - | - | - |
| 10 | stop sequences: 5 | - | REJECTED (429): {"error":{"message":"You have exceeded your current request limit. For details, see: https://www.alibabacloud.com/help/en/model-studio/error-code#rate-limit","type":"limit_requests","param":null,"code":"limit_requests"},"request_id":"847a8fa8-ced1-9ba8-b95a-48ecff62e248"} | - | - | - | - |
| 11 | max_tokens=64 | - | REJECTED (429): {"error":{"message":"You have exceeded your current request limit. For details, see: https://www.alibabacloud.com/help/en/model-studio/error-code#rate-limit","type":"limit_requests","param":null,"code":"limit_requests"},"request_id":"ae8a67c4-bb3f-9a04-9cca-b52ab37cbebd"} | - | - | - | - |
| 12 | json_object mode (response_format json_object; prompt names JSON) | - | REJECTED (429): {"error":{"message":"You have exceeded your current request limit. For details, see: https://www.alibabacloud.com/help/en/model-studio/error-code#rate-limit","type":"limit_requests","param":null,"code":"limit_requests"},"request_id":"9eef5212-5167-9e15-a36a-faa019bd0050"} | - | - | - | - |
| 13 | json_schema mode (response_format json_schema; prompt names JSON) | - | REJECTED (429): {"error":{"message":"You have exceeded your current request limit. For details, see: https://www.alibabacloud.com/help/en/model-studio/error-code#rate-limit","type":"limit_requests","param":null,"code":"limit_requests"},"request_id":"09ac23b4-78bb-9e66-9052-4ece14f4a813"} | - | - | - | - |

### Probes: `qwen3-max`

| # | probe | body extras | outcome | finish | content | usage fields | reasoning text |
|---|-------|-------------|---------|--------|---------|--------------|----------------|
| 1 | default parameters | - | REJECTED (404): {"error":{"message":"Model not exist.","type":"invalid_request_error","param":null,"code":"model_not_found"},"id":"chatcmpl-5057badb-1192-9857-83d9-374026a2deb6","request_id":"5057badb-1192-9857-83d9-374026a2deb6"} | - | - | - | - |
| 2 | temperature=0.00 (declared low end) | - | REJECTED (404): {"error":{"message":"Model not exist.","type":"invalid_request_error","param":null,"code":"model_not_found"},"id":"chatcmpl-2c0d6e74-e984-9b65-bfc6-5a1680a9c909","request_id":"2c0d6e74-e984-9b65-bfc6-5a1680a9c909"} | - | - | - | - |
| 3 | temperature=1.99 (declared high end) | - | REJECTED (404): {"error":{"message":"Model not exist.","type":"invalid_request_error","param":null,"code":"model_not_found"},"id":"chatcmpl-b2cb403c-66ca-95f6-b8e4-af23dcc7e835","request_id":"b2cb403c-66ca-95f6-b8e4-af23dcc7e835"} | - | - | - | - |
| 4 | temperature=0.50 (mid-range) | - | REJECTED (404): {"error":{"message":"Model not exist.","type":"invalid_request_error","param":null,"code":"model_not_found"},"id":"chatcmpl-0549b933-2033-914f-8208-150dabe83200","request_id":"0549b933-2033-914f-8208-150dabe83200"} | - | - | - | - |
| 5 | temperature=2.00 (just outside the high end) | - | REJECTED (404): {"error":{"message":"Model not exist.","type":"invalid_request_error","param":null,"code":"model_not_found"},"id":"chatcmpl-225a9cc7-ec0e-90e1-b062-40cf5dc6d0ca","request_id":"225a9cc7-ec0e-90e1-b062-40cf5dc6d0ca"} | - | - | - | - |
| 6 | top_p=0.5 | - | REJECTED (404): {"error":{"message":"Model not exist.","type":"invalid_request_error","param":null,"code":"model_not_found"},"id":"chatcmpl-4bedf9af-2473-953e-9c89-7bccfde17e83","request_id":"4bedf9af-2473-953e-9c89-7bccfde17e83"} | - | - | - | - |
| 7 | frequency_penalty=0.5 | - | REJECTED (404): {"error":{"message":"Model not exist.","type":"invalid_request_error","param":null,"code":"model_not_found"},"id":"chatcmpl-d4f75feb-c06d-9153-860b-437611973203","request_id":"d4f75feb-c06d-9153-860b-437611973203"} | - | - | - | - |
| 8 | presence_penalty=0.5 | - | REJECTED (404): {"error":{"message":"Model not exist.","type":"invalid_request_error","param":null,"code":"model_not_found"},"id":"chatcmpl-97b1fcc9-f09f-9b49-ba10-c66d9dad0bdc","request_id":"97b1fcc9-f09f-9b49-ba10-c66d9dad0bdc"} | - | - | - | - |
| 9 | stop sequences: 1 | - | REJECTED (404): {"error":{"message":"Model not exist.","type":"invalid_request_error","param":null,"code":"model_not_found"},"id":"chatcmpl-daee09dd-bcfd-96e5-b87c-66e12391b260","request_id":"daee09dd-bcfd-96e5-b87c-66e12391b260"} | - | - | - | - |
| 10 | stop sequences: 5 | - | REJECTED (404): {"error":{"message":"Model not exist.","type":"invalid_request_error","param":null,"code":"model_not_found"},"id":"chatcmpl-477ec797-ab57-9ee7-a9ca-0ad078481a60","request_id":"477ec797-ab57-9ee7-a9ca-0ad078481a60"} | - | - | - | - |
| 11 | max_tokens=64 | - | REJECTED (404): {"error":{"message":"Model not exist.","type":"invalid_request_error","param":null,"code":"model_not_found"},"id":"chatcmpl-cbbb8629-639a-9a47-8502-ce0b7ae4b568","request_id":"cbbb8629-639a-9a47-8502-ce0b7ae4b568"} | - | - | - | - |
| 12 | json_object mode (response_format json_object; prompt names JSON) | - | REJECTED (404): {"error":{"message":"Model not exist.","type":"invalid_request_error","param":null,"code":"model_not_found"},"id":"chatcmpl-fc25b2bc-754e-9bcd-ab48-696b0701cd5e","request_id":"fc25b2bc-754e-9bcd-ab48-696b0701cd5e"} | - | - | - | - |
| 13 | json_schema mode (response_format json_schema; prompt names JSON) | - | REJECTED (404): {"error":{"message":"Model not exist.","type":"invalid_request_error","param":null,"code":"model_not_found"},"id":"chatcmpl-b39493cc-2bff-9830-9f86-915e0b6097c0","request_id":"b39493cc-2bff-9830-9f86-915e0b6097c0"} | - | - | - | - |

### Probes: `qwen3.8-flash`

| # | probe | body extras | outcome | finish | content | usage fields | reasoning text |
|---|-------|-------------|---------|--------|---------|--------------|----------------|
| 1 | default parameters | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.text_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.text_tokens, total_tokens | present |
| 2 | temperature=0.00 (declared low end) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.text_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.text_tokens, total_tokens | present |
| 3 | temperature=1.99 (declared high end) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.text_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.text_tokens, total_tokens | present |
| 4 | temperature=0.50 (mid-range) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.text_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.text_tokens, total_tokens | present |
| 5 | temperature=2.00 (just outside the high end) | - | REJECTED (400): {"error":{"message":"<400> InternalError.Algo.InvalidParameter: Temperature should be in [0.0, 2.0)","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-c3fabb3a-b41b-94ff-9540-8b88b1604a53","request_id":"c3fabb3a-b41b-94ff-9540-8b88b1604a53"} | - | - | - | - |
| 6 | top_p=0.5 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.text_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.text_tokens, total_tokens | present |
| 7 | frequency_penalty=0.5 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.text_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.text_tokens, total_tokens | present |
| 8 | presence_penalty=0.5 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.text_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.text_tokens, total_tokens | present |
| 9 | stop sequences: 1 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.text_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.text_tokens, total_tokens | present |
| 10 | stop sequences: 5 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.text_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.text_tokens, total_tokens | present |
| 11 | max_tokens=64 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.text_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.text_tokens, total_tokens | present |
| 12 | json_object mode (response_format json_object; prompt names JSON) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.text_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.text_tokens, total_tokens | present |
| 13 | json_schema mode (response_format json_schema; prompt names JSON) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.text_tokens, prompt_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.text_tokens, total_tokens | present |

### Quota-shaped errors observed

- `qwen-turbo` / default parameters: status 429, error.type/status `limit_requests`, error.code `limit_requests`: {"error":{"message":"You have exceeded your current request limit. For details, see: https://www.alibabacloud.com/help/en/model-studio/error-code#rate-limit","type":"limit_requests","param":null,"code":"limit_requests"},"request_id":"f77c028a-da52-98f1-96dc-a4988236c23b"}
- `qwen-turbo` / temperature=0.00 (declared low end): status 429, error.type/status `limit_requests`, error.code `limit_requests`: {"error":{"message":"You have exceeded your current request limit. For details, see: https://www.alibabacloud.com/help/en/model-studio/error-code#rate-limit","type":"limit_requests","param":null,"code":"limit_requests"},"request_id":"7c331827-0ec3-9512-8c45-0f60ce8606a6"}
- `qwen-turbo` / temperature=1.99 (declared high end): status 429, error.type/status `limit_requests`, error.code `limit_requests`: {"error":{"message":"You have exceeded your current request limit. For details, see: https://www.alibabacloud.com/help/en/model-studio/error-code#rate-limit","type":"limit_requests","param":null,"code":"limit_requests"},"request_id":"9c80c694-daf8-9ead-a105-368154f21735"}
- `qwen-turbo` / temperature=0.50 (mid-range): status 429, error.type/status `limit_requests`, error.code `limit_requests`: {"error":{"message":"You have exceeded your current request limit. For details, see: https://www.alibabacloud.com/help/en/model-studio/error-code#rate-limit","type":"limit_requests","param":null,"code":"limit_requests"},"request_id":"fca324d9-d9c6-9e8c-96a5-83658e93e658"}
- `qwen-turbo` / temperature=2.00 (just outside the high end): status 429, error.type/status `limit_requests`, error.code `limit_requests`: {"error":{"message":"You have exceeded your current request limit. For details, see: https://www.alibabacloud.com/help/en/model-studio/error-code#rate-limit","type":"limit_requests","param":null,"code":"limit_requests"},"request_id":"bc3668e6-7516-97d6-aeb1-df54c4f8f58f"}
- `qwen-turbo` / top_p=0.5: status 429, error.type/status `limit_requests`, error.code `limit_requests`: {"error":{"message":"You have exceeded your current request limit. For details, see: https://www.alibabacloud.com/help/en/model-studio/error-code#rate-limit","type":"limit_requests","param":null,"code":"limit_requests"},"request_id":"a4230e29-cc70-9fa4-89e2-3a3c185cf81b"}
- `qwen-turbo` / frequency_penalty=0.5: status 429, error.type/status `limit_requests`, error.code `limit_requests`: {"error":{"message":"You have exceeded your current request limit. For details, see: https://www.alibabacloud.com/help/en/model-studio/error-code#rate-limit","type":"limit_requests","param":null,"code":"limit_requests"},"request_id":"e60c6faf-a7b0-9800-bd07-745c8a8972ff"}
- `qwen-turbo` / presence_penalty=0.5: status 429, error.type/status `limit_requests`, error.code `limit_requests`: {"error":{"message":"You have exceeded your current request limit. For details, see: https://www.alibabacloud.com/help/en/model-studio/error-code#rate-limit","type":"limit_requests","param":null,"code":"limit_requests"},"request_id":"fd627a7d-d09e-9164-b858-30be256fa899"}
- `qwen-turbo` / stop sequences: 1: status 429, error.type/status `limit_requests`, error.code `limit_requests`: {"error":{"message":"You have exceeded your current request limit. For details, see: https://www.alibabacloud.com/help/en/model-studio/error-code#rate-limit","type":"limit_requests","param":null,"code":"limit_requests"},"request_id":"696ece13-8e38-93a9-ba6c-2e47babe97bd"}
- `qwen-turbo` / stop sequences: 5: status 429, error.type/status `limit_requests`, error.code `limit_requests`: {"error":{"message":"You have exceeded your current request limit. For details, see: https://www.alibabacloud.com/help/en/model-studio/error-code#rate-limit","type":"limit_requests","param":null,"code":"limit_requests"},"request_id":"847a8fa8-ced1-9ba8-b95a-48ecff62e248"}
- `qwen-turbo` / max_tokens=64: status 429, error.type/status `limit_requests`, error.code `limit_requests`: {"error":{"message":"You have exceeded your current request limit. For details, see: https://www.alibabacloud.com/help/en/model-studio/error-code#rate-limit","type":"limit_requests","param":null,"code":"limit_requests"},"request_id":"ae8a67c4-bb3f-9a04-9cca-b52ab37cbebd"}
- `qwen-turbo` / json_object mode (response_format json_object; prompt names JSON): status 429, error.type/status `limit_requests`, error.code `limit_requests`: {"error":{"message":"You have exceeded your current request limit. For details, see: https://www.alibabacloud.com/help/en/model-studio/error-code#rate-limit","type":"limit_requests","param":null,"code":"limit_requests"},"request_id":"9eef5212-5167-9e15-a36a-faa019bd0050"}
- `qwen-turbo` / json_schema mode (response_format json_schema; prompt names JSON): status 429, error.type/status `limit_requests`, error.code `limit_requests`: {"error":{"message":"You have exceeded your current request limit. For details, see: https://www.alibabacloud.com/help/en/model-studio/error-code#rate-limit","type":"limit_requests","param":null,"code":"limit_requests"},"request_id":"09ac23b4-78bb-9e66-9052-4ece14f4a813"}

## Grok

- base URL: `https://api.x.ai/v1`
- chat endpoint: `POST https://api.x.ai/v1/chat/completions`; credential header `Authorization` (name only, value never printed)
- models endpoint: `GET https://api.x.ai/v1/models`
- declared temperature range: 0..2
- every call carries `max_tokens` = 256 and a 120 s timeout; redirects are never followed

### Models listing

| model id | limit / capability fields (as received) |
|---|---|
| grok-4.20-0309-non-reasoning | aliases=["grok-4.20-non-reasoning","grok-4.20-non-reasoning-latest","grok-4.20-beta-non-reasoning","grok-4.20-beta-latest-non-reasoning","grok-4.20-experimental-beta-0304-non-reasoning","grok-4.20-experimental-beta-non-reasoning-latest","grok-4.20-beta-0309-non-reasoning","grok-4.20-non-reasoning-gv2"] ; cached_prompt_text_token_price=2000 ; cached_prompt_text_token_price_long_context=4000 ; completion_text_token_price=25000 ; completion_text_token_price_long_context=50000 ; context_length=1000000 ; created=1773014400 ; long_context_threshold=200000 ; object=model ; owned_by=xai ; prompt_image_token_price=12500 ; prompt_text_token_price=12500 ; prompt_text_token_price_long_context=25000 |
| grok-4.20-0309-reasoning | aliases=["grok-4.20-reasoning-latest","grok-4.20","grok-4.20-reasoning","grok-4.20-0309","grok-4.20-beta-0309-reasoning","grok-4.20-beta","grok-4.20-beta-0309","grok-4.20-beta-latest","grok-4.20-beta-latest-reasoning","grok-4.20-beta-reasoning","grok-4.20-experimental-beta-0304-reasoning","grok-4.20-experimental-beta-0304","grok-4.20-experimental-beta-reasoning-latest","grok-4.20-experimental-beta-latest","grok-4.20-reasoning-gv2"] ; cached_prompt_text_token_price=2000 ; cached_prompt_text_token_price_long_context=4000 ; completion_text_token_price=25000 ; completion_text_token_price_long_context=50000 ; context_length=1000000 ; created=1773014400 ; long_context_threshold=200000 ; object=model ; owned_by=xai ; prompt_image_token_price=12500 ; prompt_text_token_price=12500 ; prompt_text_token_price_long_context=25000 |
| grok-4.20-multi-agent-0309 | aliases=["grok-4.20-multi-agent","grok-4.20-multi-agent-latest","grok-4.20-multi-agent-beta-latest","grok-4.20-multi-agent-experimental-beta-0304","grok-4.20-multi-agent-experimental-beta-latest","grok-4.20-multi-agent-beta-0309"] ; cached_prompt_text_token_price=2000 ; cached_prompt_text_token_price_long_context=4000 ; completion_text_token_price=25000 ; completion_text_token_price_long_context=50000 ; context_length=1000000 ; created=1773014400 ; long_context_threshold=200000 ; object=model ; owned_by=xai ; prompt_image_token_price=12500 ; prompt_text_token_price=12500 ; prompt_text_token_price_long_context=25000 |
| grok-4.3 | aliases=["grok-4.3-latest"] ; cached_prompt_text_token_price=2000 ; cached_prompt_text_token_price_long_context=4000 ; capabilities={"default_reasoning_effort":"low","reasoning_effort":["none","low","medium","high","xhigh"]} ; completion_text_token_price=25000 ; completion_text_token_price_long_context=50000 ; context_length=1000000 ; created=1776384000 ; long_context_threshold=200000 ; object=model ; owned_by=xai ; prompt_image_token_price=12500 ; prompt_text_token_price=12500 ; prompt_text_token_price_long_context=25000 |
| grok-4.5 | aliases=["grok-4.5-latest","grok-build-latest"] ; cached_prompt_text_token_price=3000 ; cached_prompt_text_token_price_long_context=6000 ; capabilities={"default_reasoning_effort":"high","reasoning_effort":["low","medium","high","xhigh"]} ; completion_text_token_price=60000 ; completion_text_token_price_long_context=120000 ; context_length=500000 ; created=1782691200 ; long_context_threshold=200000 ; object=model ; owned_by=xai ; prompt_image_token_price=20000 ; prompt_text_token_price=20000 ; prompt_text_token_price_long_context=40000 |
| grok-4.6 | aliases=[] ; cached_prompt_text_token_price=5000 ; cached_prompt_text_token_price_long_context=10000 ; capabilities={"default_reasoning_effort":"high","reasoning_effort":["low","medium","high","xhigh"]} ; completion_text_token_price=60000 ; completion_text_token_price_long_context=120000 ; context_length=500000 ; created=1785974400 ; long_context_threshold=200000 ; object=model ; owned_by=xai ; prompt_image_token_price=20000 ; prompt_text_token_price=20000 ; prompt_text_token_price_long_context=40000 |
| grok-4.7 | aliases=[] ; cached_prompt_text_token_price=5000 ; cached_prompt_text_token_price_long_context=10000 ; capabilities={"default_reasoning_effort":"high","reasoning_effort":["low","medium","high","xhigh"]} ; completion_text_token_price=60000 ; completion_text_token_price_long_context=120000 ; context_length=500000 ; created=1788307200 ; long_context_threshold=200000 ; object=model ; owned_by=xai ; prompt_image_token_price=20000 ; prompt_text_token_price=20000 ; prompt_text_token_price_long_context=40000 |
| grok-build-0.1 | aliases=["grok-code-fast-1","grok-code-fast","grok-code-fast-1-0825"] ; cached_prompt_text_token_price=2000 ; cached_prompt_text_token_price_long_context=4000 ; completion_text_token_price=20000 ; completion_text_token_price_long_context=40000 ; context_length=256000 ; created=1776297600 ; long_context_threshold=200000 ; object=model ; owned_by=xai ; prompt_image_token_price=10000 ; prompt_text_token_price=10000 ; prompt_text_token_price_long_context=20000 |
| grok-imagine-image | aliases=["grok-imagine-image-2026-03-02"] ; context_length=16000 ; created=1769558400 ; image_price=200000000 ; object=model ; owned_by=xai |
| grok-imagine-image-2.0 | aliases=[] ; context_length=64000 ; created=1786147200 ; image_price=600000000 ; object=model ; owned_by=xai ; pricing=[{"price_per_image":400000000,"quality":"low","resolution":"1k"},{"price_per_image":600000000,"quality":"low","resolution":"2k"},{"price_per_image":500000000,"quality":"low","resolution":"1.5k"},{"price_per_image":600000000,"quality":"medium","resolution":"1k"},{"price_per_image":800000000,"quality":"medium","resolution":"2k"},{"price_per_image":700000000,"quality":"medium","resolution":"1.5k"}] |
| grok-imagine-image-quality | aliases=["grok-imagine-image-quality-20260403","grok-imagine-image-quality-latest","grok-imagine-image-pro"] ; context_length=16000 ; created=1775174400 ; image_price=500000000 ; object=model ; owned_by=xai |
| grok-imagine-video | aliases=[] ; created=1769558400 ; object=model ; owned_by=xai |
| grok-imagine-video-1.5 | aliases=["grok-imagine-video-1.5-preview","grok-imagine-video-1.5-2026-05-30"] ; created=1779840000 ; object=model ; owned_by=xai |
| grok-imagine-video-1.5-lite | aliases=[] ; created=1787702400 ; object=model ; owned_by=xai |

### Probes: `grok-4.6`

| # | probe | body extras | outcome | finish | content | usage fields | reasoning text |
|---|-------|-------------|---------|--------|---------|--------------|----------------|
| 1 | default parameters | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, cost_in_usd_ticks, num_sources_used, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.image_tokens, prompt_tokens_details.text_tokens, total_tokens | present |
| 2 | temperature=0.00 (declared low end) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, cost_in_usd_ticks, num_sources_used, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.image_tokens, prompt_tokens_details.text_tokens, total_tokens | present |
| 3 | temperature=2.00 (declared high end) | - | UNREACHABLE: error sending request | - | - | - | - |
| 4 | temperature=0.50 (mid-range) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, cost_in_usd_ticks, num_sources_used, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.image_tokens, prompt_tokens_details.text_tokens, total_tokens | present |
| 5 | temperature=2.01 (just outside the high end) | - | REJECTED (400): {"code":"invalid-argument","error":"Temperature must be less than 2 but temperature = 2.01"} | - | - | - | - |
| 6 | top_p=0.5 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, cost_in_usd_ticks, num_sources_used, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.image_tokens, prompt_tokens_details.text_tokens, total_tokens | present |
| 7 | frequency_penalty=0.5 | - | REJECTED (400): {"code":"invalid-argument","error":"Model grok-4.6 does not support parameter frequencyPenalty."} | - | - | - | - |
| 8 | presence_penalty=0.5 | - | REJECTED (400): {"code":"invalid-argument","error":"Model grok-4.6 does not support parameter presencePenalty."} | - | - | - | - |
| 9 | stop sequences: 1 | - | REJECTED (400): {"code":"invalid-argument","error":"Model grok-4.6 does not support parameter stop."} | - | - | - | - |
| 10 | stop sequences: 5 | - | REJECTED (400): {"code":"invalid-argument","error":"Model grok-4.6 does not support parameter stop."} | - | - | - | - |
| 11 | max_tokens=64 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, cost_in_usd_ticks, num_sources_used, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.image_tokens, prompt_tokens_details.text_tokens, total_tokens | present |
| 12 | json_object mode (response_format json_object; prompt names JSON) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, cost_in_usd_ticks, num_sources_used, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.image_tokens, prompt_tokens_details.text_tokens, total_tokens | present |
| 13 | json_schema mode (response_format json_schema; prompt names JSON) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, cost_in_usd_ticks, num_sources_used, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.image_tokens, prompt_tokens_details.text_tokens, total_tokens | present |

### Probes: `grok-4.5`

| # | probe | body extras | outcome | finish | content | usage fields | reasoning text |
|---|-------|-------------|---------|--------|---------|--------------|----------------|
| 1 | default parameters | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, cost_in_usd_ticks, num_sources_used, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.image_tokens, prompt_tokens_details.text_tokens, total_tokens | absent |
| 2 | temperature=0.00 (declared low end) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, cost_in_usd_ticks, num_sources_used, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.image_tokens, prompt_tokens_details.text_tokens, total_tokens | absent |
| 3 | temperature=2.00 (declared high end) | - | UNREACHABLE: error sending request | - | - | - | - |
| 4 | temperature=0.50 (mid-range) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, cost_in_usd_ticks, num_sources_used, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.image_tokens, prompt_tokens_details.text_tokens, total_tokens | absent |
| 5 | temperature=2.01 (just outside the high end) | - | REJECTED (400): {"code":"invalid-argument","error":"Temperature must be less than 2 but temperature = 2.01"} | - | - | - | - |
| 6 | top_p=0.5 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, cost_in_usd_ticks, num_sources_used, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.image_tokens, prompt_tokens_details.text_tokens, total_tokens | absent |
| 7 | frequency_penalty=0.5 | - | REJECTED (400): {"code":"invalid-argument","error":"Model grok-4.5 does not support parameter frequencyPenalty."} | - | - | - | - |
| 8 | presence_penalty=0.5 | - | REJECTED (400): {"code":"invalid-argument","error":"Model grok-4.5 does not support parameter presencePenalty."} | - | - | - | - |
| 9 | stop sequences: 1 | - | REJECTED (400): {"code":"invalid-argument","error":"Model grok-4.5 does not support parameter stop."} | - | - | - | - |
| 10 | stop sequences: 5 | - | REJECTED (400): {"code":"invalid-argument","error":"Model grok-4.5 does not support parameter stop."} | - | - | - | - |
| 11 | max_tokens=64 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, cost_in_usd_ticks, num_sources_used, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.image_tokens, prompt_tokens_details.text_tokens, total_tokens | absent |
| 12 | json_object mode (response_format json_object; prompt names JSON) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, cost_in_usd_ticks, num_sources_used, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.image_tokens, prompt_tokens_details.text_tokens, total_tokens | present |
| 13 | json_schema mode (response_format json_schema; prompt names JSON) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, cost_in_usd_ticks, num_sources_used, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.image_tokens, prompt_tokens_details.text_tokens, total_tokens | present |

### Probes: `grok-4.3`

| # | probe | body extras | outcome | finish | content | usage fields | reasoning text |
|---|-------|-------------|---------|--------|---------|--------------|----------------|
| 1 | default parameters | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, cost_in_usd_ticks, num_sources_used, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.image_tokens, prompt_tokens_details.text_tokens, total_tokens | present |
| 2 | temperature=0.00 (declared low end) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, cost_in_usd_ticks, num_sources_used, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.image_tokens, prompt_tokens_details.text_tokens, total_tokens | present |
| 3 | temperature=2.00 (declared high end) | - | UNREACHABLE: error sending request | - | - | - | - |
| 4 | temperature=0.50 (mid-range) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, cost_in_usd_ticks, num_sources_used, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.image_tokens, prompt_tokens_details.text_tokens, total_tokens | present |
| 5 | temperature=2.01 (just outside the high end) | - | REJECTED (400): {"code":"invalid-argument","error":"Temperature must be less than 2 but temperature = 2.01"} | - | - | - | - |
| 6 | top_p=0.5 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, cost_in_usd_ticks, num_sources_used, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.image_tokens, prompt_tokens_details.text_tokens, total_tokens | present |
| 7 | frequency_penalty=0.5 | - | REJECTED (400): {"code":"invalid-argument","error":"Model grok-4.3 does not support parameter frequencyPenalty."} | - | - | - | - |
| 8 | presence_penalty=0.5 | - | REJECTED (400): {"code":"invalid-argument","error":"Model grok-4.3 does not support parameter presencePenalty."} | - | - | - | - |
| 9 | stop sequences: 1 | - | REJECTED (400): {"code":"invalid-argument","error":"Model grok-4.3 does not support parameter stop."} | - | - | - | - |
| 10 | stop sequences: 5 | - | REJECTED (400): {"code":"invalid-argument","error":"Model grok-4.3 does not support parameter stop."} | - | - | - | - |
| 11 | max_tokens=64 | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, cost_in_usd_ticks, num_sources_used, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.image_tokens, prompt_tokens_details.text_tokens, total_tokens | present |
| 12 | json_object mode (response_format json_object; prompt names JSON) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, cost_in_usd_ticks, num_sources_used, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.image_tokens, prompt_tokens_details.text_tokens, total_tokens | present |
| 13 | json_schema mode (response_format json_schema; prompt names JSON) | - | ACCEPTED (200) | stop | non-empty | completion_tokens, completion_tokens_details.accepted_prediction_tokens, completion_tokens_details.audio_tokens, completion_tokens_details.reasoning_tokens, completion_tokens_details.rejected_prediction_tokens, cost_in_usd_ticks, num_sources_used, prompt_tokens, prompt_tokens_details.audio_tokens, prompt_tokens_details.cached_tokens, prompt_tokens_details.image_tokens, prompt_tokens_details.text_tokens, total_tokens | present |

### Quota-shaped errors observed

none observed (a quota error cannot be provoked on demand)

## Gemini

- base URL: `https://generativelanguage.googleapis.com/v1beta`
- chat endpoint: `POST https://generativelanguage.googleapis.com/v1beta/models/gemini-3.6-flash:generateContent`; credential header `x-goog-api-key` (name only, value never printed)
- models endpoint: `GET https://generativelanguage.googleapis.com/v1beta/models?pageSize=1000`
- declared temperature range: 0..2
- every call carries `generationConfig.maxOutputTokens` = 256 and a 120 s timeout; redirects are never followed

### Models listing

| model id | limit / capability fields (as received) |
|---|---|
| gemini-2.5-flash | description=Stable version of Gemini 2.5 Flash, our mid-size multimodal model that supports up to 1 million tokens, released in June of 2025. ; displayName=Gemini 2.5 Flash ; inputTokenLimit=1048576 ; maxTemperature=2 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens","createCachedContent","batchGenerateContent"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=001 |
| gemini-2.5-pro | description=Stable release (June 17th, 2025) of Gemini 2.5 Pro ; displayName=Gemini 2.5 Pro ; inputTokenLimit=1048576 ; maxTemperature=2 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens","createCachedContent","batchGenerateContent"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=2.5 |
| gemini-2.5-flash-preview-tts | description=Gemini 2.5 Flash Preview TTS ; displayName=Gemini 2.5 Flash Preview TTS ; inputTokenLimit=8192 ; maxTemperature=2 ; outputTokenLimit=16384 ; supportedGenerationMethods=["countTokens","generateContent"] ; temperature=1 ; topK=64 ; topP=0.95 ; version=gemini-2.5-flash-exp-tts-2025-05-19 |
| gemini-2.5-pro-preview-tts | description=Gemini 2.5 Pro Preview TTS ; displayName=Gemini 2.5 Pro Preview TTS ; inputTokenLimit=8192 ; maxTemperature=2 ; outputTokenLimit=16384 ; supportedGenerationMethods=["countTokens","generateContent","batchGenerateContent"] ; temperature=1 ; topK=64 ; topP=0.95 ; version=gemini-2.5-pro-preview-tts-2025-05-19 |
| gemma-4-26b-a4b-it | description=Gemma 4 26B A4B IT ; displayName=Gemma 4 26B A4B IT ; inputTokenLimit=262144 ; maxTemperature=2 ; outputTokenLimit=32768 ; supportedGenerationMethods=["generateContent","countTokens"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=001 |
| gemma-4-31b-it | description=Gemma 4 31B IT ; displayName=Gemma 4 31B IT ; inputTokenLimit=262144 ; maxTemperature=2 ; outputTokenLimit=32768 ; supportedGenerationMethods=["generateContent","countTokens"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=001 |
| gemini-flash-latest | description=Latest release of Gemini Flash ; displayName=Gemini Flash Latest ; inputTokenLimit=1048576 ; maxTemperature=2 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens","createCachedContent","batchGenerateContent"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=Gemini Flash Latest |
| gemini-flash-lite-latest | description=Latest release of Gemini Flash-Lite ; displayName=Gemini Flash-Lite Latest ; inputTokenLimit=1048576 ; maxTemperature=2 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens","createCachedContent","batchGenerateContent"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=Gemini Flash-Lite Latest |
| gemini-pro-latest | description=Latest release of Gemini Pro ; displayName=Gemini Pro Latest ; inputTokenLimit=1048576 ; maxTemperature=2 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens","createCachedContent","batchGenerateContent"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=Gemini Pro Latest |
| gemini-2.5-flash-lite | description=Stable version of Gemini 2.5 Flash-Lite, released in July of 2025 ; displayName=Gemini 2.5 Flash-Lite ; inputTokenLimit=1048576 ; maxTemperature=2 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens","createCachedContent","batchGenerateContent"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=001 |
| gemini-2.5-flash-image | description=Gemini 2.5 Flash Preview Image ; displayName=Nano Banana ; inputTokenLimit=32768 ; maxTemperature=1 ; outputTokenLimit=32768 ; supportedGenerationMethods=["generateContent","countTokens","batchGenerateContent"] ; temperature=1 ; topK=64 ; topP=0.95 ; version=2.0 |
| gemini-3-flash-preview | description=Gemini 3 Flash Preview ; displayName=Gemini 3 Flash Preview ; inputTokenLimit=1048576 ; maxTemperature=2 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens","createCachedContent","batchGenerateContent"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=3-flash-preview-12-2025 |
| gemini-3.1-pro-preview | description=Gemini 3.1 Pro Preview ; displayName=Gemini 3.1 Pro Preview ; inputTokenLimit=1048576 ; maxTemperature=2 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens","createCachedContent","batchGenerateContent"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=3.1-pro-preview-01-2026 |
| gemini-3.1-pro-preview-customtools | description=Gemini 3.1 Pro Preview optimized for custom tool usage ; displayName=Gemini 3.1 Pro Preview Custom Tools ; inputTokenLimit=1048576 ; maxTemperature=2 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens","createCachedContent","batchGenerateContent"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=3.1-pro-preview-01-2026 |
| gemini-3.1-flash-lite-preview | description=Gemini 3.1 Flash Lite Preview ; displayName=Gemini 3.1 Flash Lite Preview ; inputTokenLimit=1048576 ; maxTemperature=2 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens","createCachedContent","batchGenerateContent"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=3.1-flash-lite-preview-03-2026 |
| gemini-3.1-flash-lite | description=Gemini 3.1 Flash Lite ; displayName=Gemini 3.1 Flash Lite ; inputTokenLimit=1048576 ; maxTemperature=2 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens","createCachedContent","batchGenerateContent"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=3.1-flash-lite-05-2026 |
| gemini-3-pro-image-preview | description=Gemini 3 Pro Image Preview ; displayName=Nano Banana Pro ; inputTokenLimit=131072 ; maxTemperature=1 ; outputTokenLimit=32768 ; supportedGenerationMethods=["generateContent","countTokens","batchGenerateContent"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=3.0 |
| gemini-3-pro-image | description=Gemini 3 Pro Image ; displayName=Nano Banana Pro ; inputTokenLimit=131072 ; maxTemperature=1 ; outputTokenLimit=32768 ; supportedGenerationMethods=["generateContent","countTokens","batchGenerateContent"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=3.0 |
| nano-banana-pro-preview | description=Gemini 3 Pro Image Preview ; displayName=Nano Banana Pro ; inputTokenLimit=131072 ; maxTemperature=1 ; outputTokenLimit=32768 ; supportedGenerationMethods=["generateContent","countTokens","batchGenerateContent"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=3.0 |
| gemini-3.1-flash-image-preview | description=Gemini 3.1 Flash Image Preview. ; displayName=Nano Banana 2 ; inputTokenLimit=65536 ; maxTemperature=1 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens","batchGenerateContent"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=3.0 |
| gemini-3.1-flash-image | description=Gemini 3.1 Flash Image. ; displayName=Nano Banana 2 ; inputTokenLimit=65536 ; maxTemperature=1 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens","batchGenerateContent"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=3.0 |
| gemini-3.1-flash-lite-image | description=Gemini 3.1 Flash Lite Image. ; displayName=Nano Banana 2 Lite ; inputTokenLimit=65536 ; maxTemperature=1 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens","batchGenerateContent"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=3.0 |
| gemini-nano-banana-2.1 | description=Gemini Nano Banana 2.1. ; displayName=Nano Banana 2.1 ; inputTokenLimit=65536 ; maxTemperature=1 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens","batchGenerateContent"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=3.0 |
| gemini-3.5-flash | description=Gemini 3.5 Flash ; displayName=Gemini 3.5 Flash ; inputTokenLimit=1048576 ; maxTemperature=2 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens","createCachedContent","batchGenerateContent"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=3.5-flash-05-2026 |
| gemini-3.5-flash-lite | description=Gemini 3.5 Flash Lite ; displayName=Gemini 3.5 Flash Lite ; inputTokenLimit=1048576 ; maxTemperature=2 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens","createCachedContent","batchGenerateContent"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=3.5-flash-lite-07-2026 |
| gemini-omni-flash-preview | description=Gemini Omni Flash Preview ; displayName=Gemini Omni Flash Preview ; inputTokenLimit=131072 ; maxTemperature=2 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=001 |
| gemini-omni-1.1-flash | description=Gemini Omni 1.1 Flash  ; displayName=Gemini Omni 1.1 Flash ; inputTokenLimit=131072 ; maxTemperature=2 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=001 |
| gemini-3.5-transcribe | description=Gemini 3.5 Transcribe ; displayName=Gemini 3.5 Transcribe ; inputTokenLimit=98304 ; maxTemperature=2 ; outputTokenLimit=32768 ; supportedGenerationMethods=["generateContent","countTokens"] ; temperature=1 ; topK=64 ; topP=0.95 ; version=3.5-transcribe-08-2026 |
| gemini-3.6-flash | description=Gemini 3.6 Flash ; displayName=Gemini 3.6 Flash ; inputTokenLimit=1048576 ; maxTemperature=2 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens","createCachedContent","batchGenerateContent"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=3.6-flash-07-2026 |
| gemini-3.7-flash | description=Gemini 3.7 Flash ; displayName=Gemini 3.7 Flash ; inputTokenLimit=1048576 ; maxTemperature=2 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens","createCachedContent","batchGenerateContent"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=3.7-flash-08-2026 |
| gemini-3.8-flash | description=Gemini 3.8 Flash ; displayName=Gemini 3.8 Flash ; inputTokenLimit=1048576 ; maxTemperature=2 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens","createCachedContent","batchGenerateContent"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=3.0 |
| gemini-4-argon | description=Gemini 4 Argon ; displayName=Gemini 4 Argon ; inputTokenLimit=1048576 ; maxTemperature=2 ; outputTokenLimit=32768 ; supportedGenerationMethods=["generateContent","countTokens","batchGenerateContent"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=3.0 |
| lyria-3-clip-preview | description=Lyria 3 30s model Preview ; displayName=Lyria 3 Clip Preview ; inputTokenLimit=1048576 ; maxTemperature=2 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens"] ; temperature=1 ; topK=64 ; topP=0.95 ; version=lyria-3-clip-preview |
| lyria-3-pro-preview | description=Lyria 3 Pro Preview ; displayName=Lyria 3 Pro Preview ; inputTokenLimit=1048576 ; maxTemperature=2 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens"] ; temperature=1 ; topK=64 ; topP=0.95 ; version=lyria-3-pro-preview |
| lyria-3.5 | description=Music Generation model ; displayName=Lyria 3.5 ; inputTokenLimit=1048576 ; maxTemperature=2 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens"] ; temperature=1 ; topK=64 ; topP=0.95 ; version=3.5 |
| gemini-3.1-flash-tts-preview | description=Gemini 3.1 Flash TTS Preview ; displayName=Gemini 3.1 Flash TTS Preview ; inputTokenLimit=8192 ; maxTemperature=2 ; outputTokenLimit=16384 ; supportedGenerationMethods=["generateContent","countTokens","batchGenerateContent"] ; temperature=1 ; topK=64 ; topP=0.95 ; version=3.1-flash-tts-preview |
| gemini-3.8-flash-tts | description=Gemini 3.8 Flash TTS ; displayName=Gemini 3.8 Flash TTS ; inputTokenLimit=8192 ; maxTemperature=2 ; outputTokenLimit=16384 ; supportedGenerationMethods=["generateContent","countTokens","batchGenerateContent"] ; temperature=1 ; topK=64 ; topP=0.95 ; version=3.8-flash-tts |
| gemini-3.8-flash-lite-tts | description=Gemini 3.8 Flash Lite TTS ; displayName=Gemini 3.8 Flash Lite TTS ; inputTokenLimit=8192 ; maxTemperature=2 ; outputTokenLimit=16384 ; supportedGenerationMethods=["generateContent","countTokens","batchGenerateContent"] ; temperature=1 ; topK=64 ; topP=0.95 ; version=3.8-flash-lite-tts |
| gemini-robotics-er-2-preview | description=Gemini Robotics-ER 2 Preview ; displayName=Gemini Robotics-ER 2 Preview ; inputTokenLimit=131072 ; maxTemperature=2 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens","createCachedContent","batchGenerateContent"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=2-preview |
| gemini-2.5-computer-use-preview-10-2025 | description=Gemini 2.5 Computer Use Preview 10-2025 ; displayName=Gemini 2.5 Computer Use Preview 10-2025 ; inputTokenLimit=131072 ; maxTemperature=2 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=Gemini 2.5 Computer Use Preview 10-2025 |
| antigravity-preview-05-2026 | description=Preview release of Antigravity Agent (05-2026) ; displayName=Antigravity Agent Preview ; inputTokenLimit=1048576 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens"] ; version=0.1 |
| antigravity-preview-09-2026 | description=Preview release of Antigravity Agent (09-2026) ; displayName=Antigravity Agent Preview ; inputTokenLimit=1048576 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens"] ; version=0.1 |
| antigravity-preview-latest | description=Latest preview release of Antigravity Agent ; displayName=Antigravity Agent Preview Latest ; inputTokenLimit=1048576 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens"] ; version=Antigravity Agent Preview Latest |
| deep-research-max-preview-04-2026 | description=Preview release (April 21st, 2026) of Deep Research Max ; displayName=Deep Research Max Preview (Apr-21-2026) ; inputTokenLimit=131072 ; maxTemperature=2 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=deepthink-exp-05-20 |
| deep-research-preview-04-2026 | description=Preview release (April 21th, 2026) of Deep Research ; displayName=Deep Research Preview (Apr-21-2026) ; inputTokenLimit=131072 ; maxTemperature=2 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=deepthink-exp-05-20 |
| deep-research-pro-preview-12-2025 | description=Preview release (December 12th, 2025) of Deep Research Pro ; displayName=Deep Research Pro Preview (Dec-12-2025) ; inputTokenLimit=131072 ; maxTemperature=2 ; outputTokenLimit=65536 ; supportedGenerationMethods=["generateContent","countTokens"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=deepthink-exp-05-20 |
| gemini-embedding-001 | description=Obtain a distributed representation of a text. ; displayName=Gemini Embedding 001 ; inputTokenLimit=2048 ; outputTokenLimit=1 ; supportedGenerationMethods=["embedContent","countTextTokens","countTokens","asyncBatchEmbedContent"] ; version=001 |
| gemini-embedding-2-preview | description=Obtain a distributed representation of multimodal content. ; displayName=Gemini Embedding 2 Preview ; inputTokenLimit=8192 ; outputTokenLimit=1 ; supportedGenerationMethods=["embedContent","countTextTokens","countTokens","asyncBatchEmbedContent"] ; version=2 |
| gemini-embedding-2 | description=Obtain a distributed representation of multimodal content. ; displayName=Gemini Embedding 2 ; inputTokenLimit=8192 ; outputTokenLimit=1 ; supportedGenerationMethods=["embedContent","countTextTokens","countTokens","asyncBatchEmbedContent"] ; version=2 |
| aqa | description=Model trained to return answers to questions that are grounded in provided sources, along with estimating answerable probability. ; displayName=Model that performs Attributed Question Answering. ; inputTokenLimit=7168 ; outputTokenLimit=1024 ; supportedGenerationMethods=["generateAnswer"] ; temperature=0.2 ; topK=40 ; topP=1 ; version=001 |
| veo-3.1-generate-preview | description=Veo 3.1 ; displayName=Veo 3.1 ; inputTokenLimit=480 ; outputTokenLimit=8192 ; supportedGenerationMethods=["predictLongRunning"] ; version=3.1 |
| veo-3.1-fast-generate-preview | description=Veo 3.1 fast ; displayName=Veo 3.1 fast ; inputTokenLimit=480 ; outputTokenLimit=8192 ; supportedGenerationMethods=["predictLongRunning"] ; version=3.1 |
| veo-3.1-lite-generate-preview | description=Veo 3.1 lite ; displayName=Veo 3.1 lite ; inputTokenLimit=480 ; outputTokenLimit=8192 ; supportedGenerationMethods=["predictLongRunning"] ; version=3.1 |
| gemini-3.5-transcribe-live | description=Gemini 3.5 Transcribe Live ; displayName=Gemini 3.5 Transcribe Live ; inputTokenLimit=131072 ; maxTemperature=2 ; outputTokenLimit=65536 ; supportedGenerationMethods=["bidiGenerateContent"] ; temperature=1 ; topK=64 ; topP=0.95 ; version=3.5-transcribe-live-08-2026 |
| gemini-2.5-flash-native-audio-latest | description=Latest release of Gemini 2.5 Flash Native Audio ; displayName=Gemini 2.5 Flash Native Audio Latest ; inputTokenLimit=131072 ; maxTemperature=2 ; outputTokenLimit=8192 ; supportedGenerationMethods=["countTokens","bidiGenerateContent"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=Gemini 2.5 Flash Native Audio Latest |
| gemini-2.5-flash-native-audio-preview-09-2025 | description=Gemini 2.5 Flash Native Audio Preview 09-2025 ; displayName=Gemini 2.5 Flash Native Audio Preview 09-2025 ; inputTokenLimit=131072 ; maxTemperature=2 ; outputTokenLimit=8192 ; supportedGenerationMethods=["countTokens","bidiGenerateContent"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=gemini-2.5-flash-preview-native-audio-dialog-2025-05-19 |
| gemini-2.5-flash-native-audio-preview-12-2025 | description=Gemini 2.5 Flash Native Audio Preview 12-2025 ; displayName=Gemini 2.5 Flash Native Audio Preview 12-2025 ; inputTokenLimit=131072 ; maxTemperature=2 ; outputTokenLimit=8192 ; supportedGenerationMethods=["countTokens","bidiGenerateContent"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=12-2025 |
| gemini-3.1-flash-live-preview | description=Gemini 3.1 Flash Live Preview ; displayName=Gemini 3.1 Flash Live Preview ; inputTokenLimit=131072 ; maxTemperature=2 ; outputTokenLimit=65536 ; supportedGenerationMethods=["bidiGenerateContent"] ; temperature=1 ; topK=64 ; topP=0.95 ; version=3.1-flash-live-03-2026 |
| gemini-3.8-live | description=Gemini 3.8 Live ; displayName=Gemini 3.8 Live ; inputTokenLimit=131072 ; maxTemperature=2 ; outputTokenLimit=65536 ; supportedGenerationMethods=["bidiGenerateContent"] ; temperature=1 ; topK=64 ; topP=0.95 ; version=3.1-flash-live-03-2026 |
| gemini-3.8-live-extended-thinking | description=Gemini 3.8 Live Extended Thinking ; displayName=Gemini 3.8 Live Extended Thinking ; inputTokenLimit=131072 ; maxTemperature=2 ; outputTokenLimit=65536 ; supportedGenerationMethods=["bidiGenerateContent"] ; temperature=1 ; thinking=true ; topK=64 ; topP=0.95 ; version=3.1-flash-live-03-2026 |
| gemini-robotics-er-2-streaming-preview | description=Gemini Robotics-ER 2 Streaming Preview ; displayName=Gemini Robotics-ER 2 Streaming Preview ; inputTokenLimit=131072 ; maxTemperature=2 ; outputTokenLimit=65536 ; supportedGenerationMethods=["bidiGenerateContent"] ; temperature=1 ; topK=64 ; topP=0.95 ; version=2-streaming-preview |
| gemini-3.5-live-translate-preview | description=Gemini 3.5 Live Translate Preview ; displayName=Gemini 3.5 Live Translate Preview ; inputTokenLimit=16384 ; maxTemperature=2 ; outputTokenLimit=32768 ; supportedGenerationMethods=["bidiGenerateContent"] ; temperature=1 ; topK=64 ; topP=0.95 ; version=3.5-live-translate-06-2026 |
| lyria-realtime-exp | description=Lyria Realtime Experimental ; displayName=Lyria Realtime Experimental ; inputTokenLimit=1048576 ; outputTokenLimit=65536 ; supportedGenerationMethods=["bidiGenerateMusic"] ; version=lyria-realtime-exp |

### Probes: `gemini-3.6-flash`

| # | probe | body extras | outcome | finish | content | usage fields | reasoning text |
|---|-------|-------------|---------|--------|---------|--------------|----------------|
| 1 | default parameters | - | ACCEPTED (200) | STOP | non-empty | candidatesTokenCount, promptTokenCount, promptTokensDetails, serviceTier, thoughtsTokenCount, totalTokenCount | absent |
| 2 | temperature=0.00 (declared low end) | - | ACCEPTED (200) | STOP | non-empty | candidatesTokenCount, promptTokenCount, promptTokensDetails, serviceTier, thoughtsTokenCount, totalTokenCount | absent |
| 3 | temperature=2.00 (declared high end) | - | ACCEPTED (200) | STOP | non-empty | candidatesTokenCount, promptTokenCount, promptTokensDetails, serviceTier, thoughtsTokenCount, totalTokenCount | absent |
| 4 | temperature=0.50 (mid-range) | - | ACCEPTED (200) | STOP | non-empty | candidatesTokenCount, promptTokenCount, promptTokensDetails, serviceTier, thoughtsTokenCount, totalTokenCount | absent |
| 5 | temperature=2.01 (just outside the high end) | - | REJECTED (400): {   "error": {     "code": 400,     "message": "* [REDACTED]: temperature must be in the range [0.0, 2.0].\n",     "status": "INVALID_ARGUMENT"   } }  | - | - | - | - |
| 6 | top_p=0.5 | - | ACCEPTED (200) | STOP | non-empty | candidatesTokenCount, promptTokenCount, promptTokensDetails, serviceTier, thoughtsTokenCount, totalTokenCount | absent |
| 7 | frequency_penalty=0.5 | - | NOT APPLICABLE (this wire has no such request field) | - | - | - | - |
| 8 | presence_penalty=0.5 | - | NOT APPLICABLE (this wire has no such request field) | - | - | - | - |
| 9 | stop sequences: 1 | - | ACCEPTED (200) | STOP | non-empty | candidatesTokenCount, promptTokenCount, promptTokensDetails, serviceTier, thoughtsTokenCount, totalTokenCount | absent |
| 10 | stop sequences: 5 | - | ACCEPTED (200) | STOP | non-empty | candidatesTokenCount, promptTokenCount, promptTokensDetails, serviceTier, thoughtsTokenCount, totalTokenCount | absent |
| 11 | max_tokens=64 | - | ACCEPTED (200) | STOP | non-empty | candidatesTokenCount, promptTokenCount, promptTokensDetails, serviceTier, thoughtsTokenCount, totalTokenCount | absent |
| 12 | json_object mode (generationConfig.responseMimeType; prompt names JSON) | - | ACCEPTED (200) | STOP | non-empty | candidatesTokenCount, promptTokenCount, promptTokensDetails, serviceTier, thoughtsTokenCount, totalTokenCount | absent |
| 13 | json_schema mode (generationConfig.responseMimeType + responseSchema; prompt names JSON) | - | ACCEPTED (200) | STOP | non-empty | candidatesTokenCount, promptTokenCount, promptTokensDetails, serviceTier, thoughtsTokenCount, totalTokenCount | absent |

### Probes: `gemini-3.5-flash`

| # | probe | body extras | outcome | finish | content | usage fields | reasoning text |
|---|-------|-------------|---------|--------|---------|--------------|----------------|
| 1 | default parameters | - | ACCEPTED (200) | STOP | non-empty | candidatesTokenCount, promptTokenCount, promptTokensDetails, serviceTier, thoughtsTokenCount, totalTokenCount | absent |
| 2 | temperature=0.00 (declared low end) | - | ACCEPTED (200) | STOP | non-empty | candidatesTokenCount, promptTokenCount, promptTokensDetails, serviceTier, thoughtsTokenCount, totalTokenCount | absent |
| 3 | temperature=2.00 (declared high end) | - | ACCEPTED (200) | STOP | non-empty | candidatesTokenCount, promptTokenCount, promptTokensDetails, serviceTier, thoughtsTokenCount, totalTokenCount | absent |
| 4 | temperature=0.50 (mid-range) | - | ACCEPTED (200) | STOP | non-empty | candidatesTokenCount, promptTokenCount, promptTokensDetails, serviceTier, thoughtsTokenCount, totalTokenCount | absent |
| 5 | temperature=2.01 (just outside the high end) | - | REJECTED (400): {   "error": {     "code": 400,     "message": "* [REDACTED]: temperature must be in the range [0.0, 2.0].\n",     "status": "INVALID_ARGUMENT"   } }  | - | - | - | - |
| 6 | top_p=0.5 | - | ACCEPTED (200) | STOP | non-empty | candidatesTokenCount, promptTokenCount, promptTokensDetails, serviceTier, thoughtsTokenCount, totalTokenCount | absent |
| 7 | frequency_penalty=0.5 | - | NOT APPLICABLE (this wire has no such request field) | - | - | - | - |
| 8 | presence_penalty=0.5 | - | NOT APPLICABLE (this wire has no such request field) | - | - | - | - |
| 9 | stop sequences: 1 | - | ACCEPTED (200) | STOP | non-empty | candidatesTokenCount, promptTokenCount, promptTokensDetails, serviceTier, thoughtsTokenCount, totalTokenCount | absent |
| 10 | stop sequences: 5 | - | ACCEPTED (200) | STOP | non-empty | candidatesTokenCount, promptTokenCount, promptTokensDetails, serviceTier, thoughtsTokenCount, totalTokenCount | absent |
| 11 | max_tokens=64 | - | ACCEPTED (200) | MAX_TOKENS | non-empty | candidatesTokenCount, promptTokenCount, promptTokensDetails, serviceTier, thoughtsTokenCount, totalTokenCount | absent |
| 12 | json_object mode (generationConfig.responseMimeType; prompt names JSON) | - | ACCEPTED (200) | STOP | non-empty | candidatesTokenCount, promptTokenCount, promptTokensDetails, serviceTier, thoughtsTokenCount, totalTokenCount | absent |
| 13 | json_schema mode (generationConfig.responseMimeType + responseSchema; prompt names JSON) | - | ACCEPTED (200) | STOP | non-empty | candidatesTokenCount, promptTokenCount, promptTokensDetails, serviceTier, thoughtsTokenCount, totalTokenCount | absent |

### Probes: `gemini-3.5-flash-lite`

| # | probe | body extras | outcome | finish | content | usage fields | reasoning text |
|---|-------|-------------|---------|--------|---------|--------------|----------------|
| 1 | default parameters | - | ACCEPTED (200) | STOP | non-empty | candidatesTokenCount, promptTokenCount, promptTokensDetails, serviceTier, totalTokenCount | absent |
| 2 | temperature=0.00 (declared low end) | - | ACCEPTED (200) | STOP | non-empty | candidatesTokenCount, promptTokenCount, promptTokensDetails, serviceTier, totalTokenCount | absent |
| 3 | temperature=2.00 (declared high end) | - | ACCEPTED (200) | STOP | non-empty | candidatesTokenCount, promptTokenCount, promptTokensDetails, serviceTier, totalTokenCount | absent |
| 4 | temperature=0.50 (mid-range) | - | ACCEPTED (200) | STOP | non-empty | candidatesTokenCount, promptTokenCount, promptTokensDetails, serviceTier, totalTokenCount | absent |
| 5 | temperature=2.01 (just outside the high end) | - | REJECTED (400): {   "error": {     "code": 400,     "message": "* [REDACTED]: temperature must be in the range [0.0, 2.0].\n",     "status": "INVALID_ARGUMENT"   } }  | - | - | - | - |
| 6 | top_p=0.5 | - | ACCEPTED (200) | STOP | non-empty | candidatesTokenCount, promptTokenCount, promptTokensDetails, serviceTier, totalTokenCount | absent |
| 7 | frequency_penalty=0.5 | - | NOT APPLICABLE (this wire has no such request field) | - | - | - | - |
| 8 | presence_penalty=0.5 | - | NOT APPLICABLE (this wire has no such request field) | - | - | - | - |
| 9 | stop sequences: 1 | - | ACCEPTED (200) | STOP | non-empty | candidatesTokenCount, promptTokenCount, promptTokensDetails, serviceTier, totalTokenCount | absent |
| 10 | stop sequences: 5 | - | ACCEPTED (200) | STOP | non-empty | candidatesTokenCount, promptTokenCount, promptTokensDetails, serviceTier, totalTokenCount | absent |
| 11 | max_tokens=64 | - | ACCEPTED (200) | STOP | non-empty | candidatesTokenCount, promptTokenCount, promptTokensDetails, serviceTier, totalTokenCount | absent |
| 12 | json_object mode (generationConfig.responseMimeType; prompt names JSON) | - | ACCEPTED (200) | STOP | non-empty | candidatesTokenCount, promptTokenCount, promptTokensDetails, serviceTier, totalTokenCount | absent |
| 13 | json_schema mode (generationConfig.responseMimeType + responseSchema; prompt names JSON) | - | ACCEPTED (200) | STOP | non-empty | candidatesTokenCount, promptTokenCount, promptTokensDetails, serviceTier, totalTokenCount | absent |

### Quota-shaped errors observed

none observed (a quota error cannot be provoked on demand)

## Ollama

SKIPPED (OLLAMA_BASE_URL not set)

## OpenAI-compatible

SKIPPED (OPENAI_COMPATIBLE_API_KEY not set)

---

7 of 9 providers probed (2 not configured); 0 unreachable or not buildable

## Operator-supplied evidence

Added 2026-10-15 by the operator (Am0rfu5) after skimming the transcript above. Run from the project
devcontainer on the operator's own keys at commit `c33022b9`. This is the second run of the day: the
first (2026-10-15 19:56Z, at `e9a86bf7`, preserved verbatim in git history at commit `c33022b9`) hit
Gemini free-tier rate limits; Gemini billing was upgraded on the same key and the whole probe was
re-run so that one transcript carries every measurement. Candidates were chosen as each vendor's
cheapest current general-purpose tier on purpose; frontier tiers were not probed. Every fact below is
either observed in this run (marked *observed*) or taken from the named vendor page on the stated
access date (marked *docs*). Ollama and OpenAI-compatible were deliberately skipped: no local daemon
and no endpoint are configured here.

### (a) Quota-exhaustion error bodies

No quota-exhaustion (usage-cap) body was captured for any compat preset (kimi, qwen, grok, ollama,
openai-compatible) in this run or from the operator's logs. Two **rate-limit** bodies are recorded
for plan 50-13 context only; neither is usage-cap evidence:

- *observed 2026-10-15 19:56Z, first run, git history at `c33022b9`* Gemini HTTP 429,
  `status: RESOURCE_EXHAUSTED`, message `Quota exceeded for metric:
  generativelanguage.googleapis.com/generate_content_free_tier_requests, limit: 5`, while the key was
  on the free tier (5 requests per minute). Not reproduced in this run after the billing upgrade.
  Gemini is not a compat preset.
- *observed 2026-10-15, both runs* DashScope HTTP 429, `code: limit_requests`, message `You have
  exceeded your current request limit`, on every `qwen-turbo` probe; `qwen-plus` and `qwen3.8-flash`
  were unaffected in the same runs. A per-request rate limit, not quota exhaustion.

### (b) Context window and maximum output not exposed by the models listing

| model | context window | max output | source |
|---|---|---|---|
| `gpt-5.4-nano` | 400,000 | 128,000 | *docs* https://developers.openai.com/api/docs/models/gpt-5.4-nano (accessed 2026-10-15) |
| `gpt-4o-mini` | 128,000 | 16,384 | *docs* https://developers.openai.com/api/docs/models/gpt-4o-mini (accessed 2026-10-15) |
| `kimi-k3` | 1,048,576 (*observed*, listing `context_length`) | not documented | *docs* https://platform.kimi.ai/docs/guide/choosing-an-appropriate-kimi-model-for-your-tasks states "1M-token context window" and no max output (accessed 2026-10-15) |
| `kimi-k2.6` | 262,144 (*observed*, listing `context_length`) | not documented | same page, "256K context window", no max output (accessed 2026-10-15) |
| `qwen3.8-flash` | 1,000,000 | 131,072 (thinking mode) | *docs* https://help.aliyun.com/en/model-studio/qwen3-8-flash (China/Beijing page, accessed 2026-10-15); the International pages https://www.alibabacloud.com/help/en/model-studio/models and .../model-pricing state no limits (accessed 2026-10-15) |
| `qwen-plus` | not found | not found | *docs* https://www.alibabacloud.com/help/en/model-studio/model-pricing lists only input price tiers up to 1M (accessed 2026-10-15) |
| `grok-4.3` | 1,000,000 (*observed*, listing `context_length`; *docs* "1M") | not documented | *docs* https://docs.x.ai/docs/models (accessed 2026-10-15) |
| `grok-4.5`, `grok-4.6` | 500,000 (*observed*, listing `context_length`; *docs* "500k") | not documented | *docs* https://docs.x.ai/docs/models (accessed 2026-10-15) |
| `deepseek-chat`, `deepseek-coder` | not in the listing; both answered 200 | unknown | *docs* https://api-docs.deepseek.com/quick_start/pricing names only `deepseek-flash` and `deepseek-v4-pro` and does not mention these two ids (accessed 2026-10-15); treat as undocumented aliases, do not seed |

Models whose listing exposed both values (no docs needed): `claude-haiku-5-5` (`max_input_tokens`,
`max_tokens`), `deepseek-flash` (`context_window`, `max_output_tokens`), `gemini-3.5-flash`,
`gemini-3.5-flash-lite` and `gemini-3.6-flash` (`inputTokenLimit`, `outputTokenLimit`).

### (c) Stop-sequence maxima for vendors that rejected five

- `gpt-4o-mini`: *observed* body `Invalid 'stop': array too long. Expected an array with maximum length
  4` (`code: array_above_max_length`). Maximum 4.
- `gpt-5.4-nano`: *observed* body `Unsupported parameter: 'stop' is not supported with this model`
  (`code: unsupported_parameter`) for one and for five. No stop support on this model.
- `grok-4.3`, `grok-4.5`, `grok-4.6`: *observed* body `Model <id> does not support parameter stop.`
  (`code: invalid-argument`) for one and for five. No stop support. *docs* https://docs.x.ai/docs/models
  makes no statement about `stop` (accessed 2026-10-15).
- `kimi-k2.7-code`, `kimi-k2.7-code-highspeed`: every probe rejected with `invalid thinking: only
  type=enabled is allowed for this model`; the stop probes never reached the parameter. Not
  general-purpose models; no stop finding.
- Accepted one and five (*observed*): `claude-haiku-5-5`, `deepseek-flash`, `deepseek-chat`,
  `deepseek-coder`, `kimi-k2.6`, `kimi-k3`, `qwen-plus`, `qwen3.8-flash`, `gemini-3.5-flash`,
  `gemini-3.5-flash-lite`, `gemini-3.6-flash`.

### (d) Each vendor's current general-purpose model (becomes the provider default)

Chosen as the cheapest current tier; see the note at the top of this section.

| provider | default |
|---|---|
| openai | `gpt-5.4-nano` |
| anthropic | `claude-haiku-5-5` |
| deepseek | `deepseek-flash` |
| kimi | `kimi-k2.6` |
| qwen | `qwen3.8-flash` |
| grok | `grok-4.3` |
| gemini | `gemini-3.5-flash-lite` |

### Other observations that bear on seeding (all *observed 2026-10-15*, bodies verbatim above)

- `claude-haiku-5-5`: every non-default `temperature` and `top_p` rejected with
  `` `temperature` is deprecated for this model``.
- `kimi-k2.6` and `kimi-k3`: `temperature` 0.5, 1.0 and 1.01 all rejected with `invalid temperature:
  only 0.6 is allowed for this model`; `top_p` and both penalties rejected. This supersedes the
  handoff's 2026-10-07 note that temperature must be omitted or exactly 1.
- `qwen3-max`: `Model not exist.` (404) on the International endpoint; `qwen-turbo`: rate-limited on
  every probe in both runs (see (a)). Neither measured.
- All eight curated Anthropic ids (`claude-3-*`, `claude-2.*`, `claude-instant-1.2`): 404
  `not_found_error`. Retired; do not seed.

## Owner reading R-2

R-2 kimi: accept unverified
R-2 qwen: accept unverified
R-2 grok: accept unverified
R-2 ollama: accept unverified
R-2 openai-compatible: accept unverified
