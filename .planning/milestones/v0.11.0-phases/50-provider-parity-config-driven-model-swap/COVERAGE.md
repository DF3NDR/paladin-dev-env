# API Coverage — LLM vendor chat APIs (nine providers: OpenAI, Anthropic, DeepSeek, Kimi/Moonshot, Qwen/DashScope, Grok/xAI, Gemini, Ollama, OpenAI-compatible)

> Full coverage by default. Opt-outs are explicit, reasoned decisions.
> Phase 50 extends existing integrations (Phases 17-19, 26, 31, 43) to parity; per-vendor data that needs live
> evidence (seeded limits, quota-error shapes) is integrated where `50-SEED-PROBE.md` evidences it and runs on the
> adapter-wide fallback, visibly, where it does not (D-07, D-08, D-11a).

| capability | decision | reason |
|---|---|---|
| chat_completion_buffered | INTEGRATE | |
| chat_completion_streaming | INTEGRATE | |
| system_messages | INTEGRATE | |
| user_turn_query_and_context | INTEGRATE | |
| sampling_temperature | INTEGRATE | |
| sampling_top_p | INTEGRATE | |
| sampling_frequency_presence_penalty | INTEGRATE | |
| stop_sequences | INTEGRATE | |
| max_output_tokens | INTEGRATE | |
| json_mode_response_format | INTEGRATE | |
| json_schema_response_format | INTEGRATE | |
| usage_prompt_completion_total | INTEGRATE | |
| usage_cache_read_tokens | INTEGRATE | |
| usage_cache_write_tokens | INTEGRATE | |
| usage_reasoning_tokens | INTEGRATE | |
| reasoning_text_buffered_and_streamed | INTEGRATE | |
| refusal_signal | INTEGRATE | |
| content_filter_finish | INTEGRATE | |
| stop_reason_mapping | INTEGRATE | |
| error_status_mapping | INTEGRATE | |
| usage_cap_quota_errors | INTEGRATE | |
| rate_limit_retry_after | INTEGRATE | |
| models_listing_live_probe | INTEGRATE | |
| per_model_limits_and_parameter_support | INTEGRATE | |
| live_models_discovery_at_construction | OPT-OUT | explicitly out of scope — deferred by D-05 (handoff D-3 option c): adds a network dependency to adapter construction; the dated table plus config overrides is the decided source |
| native_tool_function_calling | OPT-OUT | explicitly out of scope — ADR-0042 and handoff §9 |
| vision_multimodal_input_compat | OPT-OUT | explicitly out of scope — ADR-0042 and handoff §9 |
| prompt_caching_cache_control | OPT-OUT | explicitly out of scope — handoff §9; no adapter implements it |
| request_id_trace_propagation | OPT-OUT | explicitly out of scope — handoff §9 |
| reasoning_effort_thinking_controls | OPT-OUT | not needed yet — owned by Phase 51 (REASON-01..07), which builds on this phase's ModelProfile and registry |
| top_k_sampling | OPT-OUT | not needed yet — not carried by `PromptParameters`; `SamplingParam` is `#[non_exhaustive]` so it can be added without a break |
| seed_logprobs_multiple_choices | OPT-OUT | not needed — no Paladin path requests them and no consumer has asked |
| embeddings | OPT-OUT | not needed — unrelated to a chat-provider swap; the existing OpenAI embeddings adapter is unchanged |
| batch_and_files_apis | OPT-OUT | not needed — Paladin issues synchronous per-call requests only |
