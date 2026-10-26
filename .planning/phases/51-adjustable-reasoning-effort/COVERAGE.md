# API Coverage — LLM vendor reasoning / thinking controls (nine providers: OpenAI, Anthropic, DeepSeek, Kimi/Moonshot, Qwen/DashScope, Grok/xAI, Gemini, Ollama, OpenAI-compatible)

> Full coverage by default. Opt-outs are explicit, reasoned decisions.
> Phase 51 extends the existing chat integrations (Phases 17-19, 26, 31, 43, 50, 50.1) with the vendors' request-side
> reasoning controls. Per D-01 / D-04 a capability is integrated in code for every vendor and is enabled per model only
> where the live effort probe (`51-EFFORT-PROBE.md`) confirms it; elsewhere it refuses before the HTTP call, visibly.
> Reasoning text and reasoning-token usage were integrated by Phase 50 (PARITY-05) and are regression-checked here.

| capability | decision | reason |
|---|---|---|
| effort_levels_request_field | INTEGRATE | |
| thinking_off | INTEGRATE | |
| thinking_auto_on | INTEGRATE | |
| thinking_budget_tokens | INTEGRATE | |
| per_model_effort_support_data | INTEGRATE | |
| invalid_effort_rejection_probe | INTEGRATE | |
| reasoning_tokens_usage | INTEGRATE | |
| reasoning_text_buffered_and_streamed | INTEGRATE | |
| effort_in_fallback_chains | INTEGRATE | |
| self_hosted_declared_reasoning_field | INTEGRATE | |
| openai_responses_api_reasoning_object | OPT-OUT | explicitly out of scope — CONTEXT domain and Reviewed Todos: `/v1/responses` is its own wire; this phase maps Chat Completions `reasoning_effort` only |
| xai_responses_api_reasoning_object | OPT-OUT | explicitly out of scope — Paladin's Grok preset speaks chat completions only; the Responses `reasoning.effort` wire is the same follow-up as OpenAI's |
| anthropic_per_message_effort_beta | OPT-OUT | explicitly out of scope — handoff section 10 and CONTEXT domain |
| interleaved_thinking_with_tools | OPT-OUT | explicitly out of scope — ADR-0042 (native tool calling deferred) |
| live_discovery_of_levels_from_models_endpoint | OPT-OUT | explicitly out of scope — CONTEXT Deferred Ideas (follow-up to Phase 50 D-05): levels come from the dated probe-seeded registry plus overrides |
| thinking_display_and_include_thoughts | OPT-OUT | not needed — D-00g fills the typed reasoning-text surface when text is received and does not request it (research Pitfall 5: Anthropic `display`, Gemini `includeThoughts`) |
| anthropic_between_tools_thinking | OPT-OUT | not needed — not in the `ReasoningEffort` vocabulary (D-00d); `Off` on a model that only offers it refuses |
| ollama_native_think_parameter | OPT-OUT | not needed — Paladin's Ollama adapter speaks `/v1` chat completions, where the operator-declared `ReasoningWire` applies (handoff section 4, D-10) |
| gemini_openai_compat_reasoning_effort | OPT-OUT | not needed — Paladin's Gemini adapter uses the native `generateContent` wire (`thinkingConfig`), never the OpenAI-compatible endpoint |
| per_model_provider_default_effort | OPT-OUT | not needed yet — research Open Question 6: only the provider-wide `llm.<provider>.reasoning_effort` ships; a per-model default is in no decision |
