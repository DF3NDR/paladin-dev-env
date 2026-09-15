# API Coverage — provider token-usage reporting surface (Phase 31)

> Full coverage by default. Opt-outs are explicit, reasoned decisions.
>
> Scope note: this phase does not add a new external API integration. Every provider adapter
> already integrates its vendor API; what changes here is **which usage fields each adapter parses**
> and whether the **streaming** path reports the same figures as the non-streaming one. The
> capability surface below is therefore the per-provider *usage-reporting* surface, enumerated as
> five capabilities per provider: the non-streaming usage object, the streaming usage frame, and the
> three "of which" sub-counts (cache read, cache write, reasoning).
>
> Every `OPT-OUT` reason cites the provider fact that forces it. Per D-03, a figure the provider
> does not report is `None` — never a fabricated `Some(0)` — so an opt-out here means "this provider
> has no such figure to carry", not "we chose not to bother".
>
> Rows are derived from `31-RESEARCH.md` § Architecture Pattern 3 (per-provider mechanisms, verified
> against current provider documentation) and are reconciled against what actually landed by plan
> 31-07, task 2.

| capability | decision | reason |
|---|---|---|
| openai.non_streaming_usage | INTEGRATE | |
| openai.streaming_usage_frame | INTEGRATE | |
| openai.cache_read | INTEGRATE | |
| openai.cache_write | OPT-OUT | OpenAI's usage object reports no cache-write/creation figure — `None` per D-03 |
| openai.reasoning | INTEGRATE | |
| anthropic.non_streaming_usage | INTEGRATE | |
| anthropic.streaming_usage_frame | INTEGRATE | |
| anthropic.cache_read | INTEGRATE | |
| anthropic.cache_write | INTEGRATE | |
| anthropic.reasoning | INTEGRATE | |
| gemini.non_streaming_usage | INTEGRATE | |
| gemini.streaming_usage_frame | INTEGRATE | |
| gemini.cache_read | INTEGRATE | |
| gemini.cache_write | OPT-OUT | Gemini's `usageMetadata` reports no cache-write figure — `None` per D-03 |
| gemini.reasoning | INTEGRATE | |
| deepseek.non_streaming_usage | INTEGRATE | |
| deepseek.streaming_usage_frame | INTEGRATE | |
| deepseek.cache_read | INTEGRATE | |
| deepseek.cache_write | OPT-OUT | DeepSeek reports cache hit/miss on the input side only, with no separate cache-write figure — `None` per D-03 |
| deepseek.reasoning | INTEGRATE | |
| grok.non_streaming_usage | INTEGRATE | |
| grok.streaming_usage_frame | INTEGRATE | |
| grok.cache_read | INTEGRATE | |
| grok.cache_write | OPT-OUT | xAI's `prompt_tokens_details` carries no cache-write figure — `None` per D-03 |
| grok.reasoning | OPT-OUT | No reasoning-token field was confirmed in xAI's usage object (RESEARCH.md assumption A2); the shared OpenAI-shaped parser reads one if the server sends it, and otherwise reports `None` per D-03 |
| kimi.non_streaming_usage | INTEGRATE | |
| kimi.streaming_usage_frame | INTEGRATE | |
| kimi.cache_read | INTEGRATE | |
| kimi.cache_write | OPT-OUT | Moonshot's OpenAI-compatible usage object carries no cache-write figure — `None` per D-03 |
| kimi.reasoning | INTEGRATE | |
| qwen.non_streaming_usage | INTEGRATE | |
| qwen.streaming_usage_frame | INTEGRATE | |
| qwen.cache_read | INTEGRATE | |
| qwen.cache_write | OPT-OUT | DashScope's OpenAI-compatible usage object carries no cache-write figure — `None` per D-03 |
| qwen.reasoning | INTEGRATE | |
| ollama.non_streaming_usage | INTEGRATE | |
| ollama.streaming_usage_frame | INTEGRATE | |
| ollama.cache_read | INTEGRATE | |
| ollama.cache_write | OPT-OUT | Ollama's OpenAI-compatible usage object carries no cache-write figure — `None` per D-03 |
| ollama.reasoning | INTEGRATE | |
| openai_compatible.non_streaming_usage | INTEGRATE | |
| openai_compatible.streaming_usage_frame | INTEGRATE | |
| openai_compatible.streamed_usage_guarantee | OPT-OUT | A third-party server may ignore `stream_options`, so streamed usage cannot be guaranteed; the terminal chunk carries `None` when the frame is omitted — the explicit D-16 exception |
| openai_compatible.cache_read | INTEGRATE | |
| openai_compatible.cache_write | OPT-OUT | The OpenAI-shaped usage object this preset speaks carries no cache-write figure — `None` per D-03 |
| openai_compatible.reasoning | INTEGRATE | |
| mock.non_streaming_usage | INTEGRATE | |
| mock.streaming_usage_frame | INTEGRATE | |
| mock.sub_counts | INTEGRATE | |
| fallback.usage_passthrough | INTEGRATE | |
| vision.usage_sub_counts | OPT-OUT | `VisionTokenUsage` and the vision adapters are out of scope for this phase (D-20); the vision usage maps to a prompt/completion pair with the three optionals left `None` |
| any_provider.estimated_usage_fallback | OPT-OUT | A missing streamed usage records `TokenUsage::default()` plus one warning, never a `TokenCounterPort` estimate (D-17) — deferred as a possible Milestone 14 opt-in |
| any_provider.currency_cost | OPT-OUT | Pricing, `cost_estimate`, allowances and rate pacing are reserved for Milestone 14 (ADR-0050), outside this phase's boundary |
