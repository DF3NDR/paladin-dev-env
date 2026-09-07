# API Coverage — LLM provider chat-completions surface (RT-05 `response_format`, RT-06 conformance)

> Full coverage by default. Opt-outs are explicit, reasoned decisions.
>
> **Why this file exists for a phase that ships no new adapter.** Phase 26 does not add a provider.
> It (a) measures the already-shipped OpenAI-compatible / Gemini / Ollama adapters against a shared
> conformance suite (RT-06, D-31/D-32) and (b) adds one new wire field — `response_format` — to four
> adapter paths (RT-05, D-28). Both touch the external chat-completions API surface, so the surface
> is enumerated and decided here rather than left implicit. The deterministic detector returns
> `detected: false` on the ROADMAP section alone; it is expected to fire on the plan bodies at seal
> time, and this matrix is the answer it will find.

| capability | decision | reason |
|---|---|---|
| `chat.completions` non-streaming generate | INTEGRATE | |
| `chat.completions` streaming (SSE / chunked assembly, terminal stop) | INTEGRATE | |
| usage / token-count extraction from the response | INTEGRATE | |
| HTTP status → `Transience` mapping (401/404/400/402/408/429/5xx) | INTEGRATE | |
| credential redaction in every rendered provider error | INTEGRATE | |
| redirect handling with a credential header (must not follow) | INTEGRATE | |
| `response_format` — native JSON-object mode (OpenAI, DeepSeek, compat engine: Kimi/Qwen/Grok/Ollama/OpenAI-compatible) | INTEGRATE | |
| `response_format` — native JSON-**schema** mode (OpenAI `json_schema`, Gemini `responseSchema`) | INTEGRATE | |
| `response_format` on Anthropic | OPT-OUT | Anthropic exposes no native constrained-JSON mode; the prompt-level instruction block is the mechanism, documented in the guide's per-provider table (D-28) |
| native wire-level tool calling (`tools` on the request, `function_call`/`tool_calls` on the response) | OPT-OUT | ADR-0042's deferred capability with its trigger unchanged; D-36 adds a **prompt-level** protocol only and must not touch `LlmRequest`'s tool surface or any adapter capability flag |
| provider token-count endpoints (Anthropic `count_tokens`, Gemini `countTokens`) as `TokenCounterPort` adapters | OPT-OUT | named a Deferred Idea by D-13; the heuristic + existing `tiktoken-rs` counter satisfy RT-FR-10 |
| rate-limit header parsing / request pacing | OPT-OUT | FUT-09, out of this phase's scope per CONTEXT `<domain>` |
| per-token cost accounting in currency | OPT-OUT | FUT-08 / PRD 05 §5 explicitly out of scope (token counts only) |
| model listing / capability discovery endpoints | OPT-OUT | not needed — `LlmProviderFactory::create` resolves providers by configured name (D-12); `ProviderCapabilities` is code-declared and D-28 explicitly adds no field to it |
| vision / multimodal attachments | OPT-OUT | shipped separately (`vision_llm_port.rs`, `anthropic/vision.rs`); no RT-FR touches it and X-03 forbids unplanned change |
| embeddings | INTEGRATE | reused unchanged through the existing `EmbeddingPort`; `SemanticVault` composes it rather than calling a provider directly (D-24) |
| fine-tuning / batch / files / assistants provider APIs | OPT-OUT | no RT-FR references them; the Paladin platform surface is Phase 27 (PLAT-*) and is a Paladin-owned HTTP API, not a provider API |
| Ollama-native (non-OpenAI-compatible) endpoints | OPT-OUT | RT-FR-22 requires Ollama to be verified **through** the OpenAI-compatible path, not as a separate adapter; D-32 confirms the existing env-probed suite is that proof |

**Second-integration note (the checkpoint's asymmetry rule).** The `response_format` rows above were
re-decided per provider rather than inherited from the first provider wired: OpenAI, the compat
engine, Gemini and DeepSeek each get their own wire assertion, and Anthropic's opt-out is recorded
with its own reason rather than silently omitted.
