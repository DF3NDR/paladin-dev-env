# paladin-llm

LLM provider adapters for the Paladin framework.

## Purpose

`paladin-llm` provides configurable adapters for ten LLM providers, a generic
operator-configured OpenAI-compatible adapter for any endpoint not on that list, and a
mock-backed testing provider. Every hosted-vendor adapter shares an `OPENAI_API_KEY`-style
env-var-only credential posture — no API key is ever read from a config file.

## Providers

| Provider | Feature | Credential env var | Default endpoint |
|---|---|---|---|
| OpenAI | `openai` | `OPENAI_API_KEY` | `https://api.openai.com/v1` |
| Anthropic | `anthropic` | `ANTHROPIC_API_KEY` | `https://api.anthropic.com/v1` |
| DeepSeek | `deepseek` | `DEEPSEEK_API_KEY` | `https://api.deepseek.com/v1` |
| Kimi (Moonshot AI) | `kimi` | `MOONSHOT_API_KEY` | `https://api.moonshot.ai/v1` |
| Qwen (DashScope) | `qwen` | `DASHSCOPE_API_KEY` | `https://dashscope-intl.aliyuncs.com/compatible-mode/v1` |
| Grok (xAI) | `grok` | `XAI_API_KEY` | `https://api.x.ai/v1` |
| Ollama (self-hosted) | `ollama` | none required — self-hosted, no vendor credential | `http://localhost:11434/v1` |
| Gemini (Google) | `gemini` | `GEMINI_API_KEY` | `https://generativelanguage.googleapis.com/v1beta` (bespoke `generateContent` protocol, not OpenAI-compatible) |
| Z.ai (Zhipu AI, GLM) | `zai` | `ZAI_API_KEY` | `https://api.z.ai/api/paas/v4` (environment-only: no `llm.zai` block yet) |
| Generic OpenAI-compatible | `openai-compatible` | `OPENAI_COMPATIBLE_API_KEY` | none — operator-supplied, required |
| Mock (testing) | `mock` | — | — |

> **Live verification status, per vendor (not one blanket disclaimer):** Gemini and Grok
> (xAI) are each live-verified — a model-list fetch and a `generate()` round trip both
> succeeded. Kimi (Moonshot) is live-verified the same way, plus its measured
> fixed-temperature constraint. Qwen (DashScope) is live-verified the same way too
> (2026-08-23): a 162-model catalog fetch and a `generate()` round trip both succeeded at
> the shipped Singapore endpoint. Ollama has no vendor endpoint to verify at all — it is
> self-hosted; its live exercise is the Docker Tier 2 suite (UAT test 3), which passed on
> a GitHub Actions runner on 2026-08-19.
>
> **Z.ai (GLM) is not live-verified.** Its base URL, default model (`glm-5.3`), model list and
> temperature range were read on docs.z.ai on 2026-11-17 and are vendor-documented, not
> live-measured; no seed probe or effort probe has run, so every Z.ai model is unprofiled and an
> explicit reasoning effort is refused. It is configured through `ZAI_API_KEY`, `ZAI_BASE_URL`,
> `ZAI_MODEL` and `ZAI_TIMEOUT_SECONDS` only: an `llm.zai` config section is not read (it is
> planned for a later release). Z.ai documents a separate endpoint and key family for its GLM
> Coding Plan; see the module docs of `src/zai/adapter.rs` before pointing `ZAI_BASE_URL` at it.
>
> **DashScope (Qwen) region constraint:** API keys are scoped to the Model Studio region
> that issued them and are rejected by every other region's endpoint. The base URL above
> is Singapore, the shipped default. If your workspace is in the US or on the mainland,
> you MUST set `DASHSCOPE_BASE_URL` to your own region's endpoint:
>   - Singapore (shipped default): `https://dashscope-intl.aliyuncs.com/compatible-mode/v1`
>   - US (Virginia): `https://dashscope-us.aliyuncs.com/compatible-mode/v1`
>   - China (mainland): `https://dashscope.aliyuncs.com/compatible-mode/v1`

### The generic `openai-compatible` provider

`openai-compatible` (type [`OpenAiCompatibleAdapter`]) is not a named vendor preset — it is
the answer to "does Paladin support &lt;my provider&gt;?" for **any** OpenAI-compatible chat
completions endpoint not already on the list above: a self-hosted vLLM or LiteLLM gateway,
Groq, Together, Mistral, Fireworks, Bedrock's OpenAI-compat mode, or any future one. If your
provider isn't in the table, point this adapter at its `base_url` instead of concluding
Paladin can't reach it. `base_url`, credential, model and every capability flag are supplied
entirely by the operator, with pessimistic (`false`/unset) defaults for any capability not
explicitly declared — it never claims a capability nobody asserted. It is fully usable from
environment variables alone (`OPENAI_COMPATIBLE_API_KEY`, `OPENAI_COMPATIBLE_BASE_URL`,
`OPENAI_COMPATIBLE_MODEL`, plus optional `OPENAI_COMPATIBLE_SUPPORTS_*` capability flags), no
config file required — see the module doc on [`OpenAiCompatibleAdapter`] for the full
environment-variable table.

> **`OPENAI_COMPATIBLE_API_KEY` is not `OPENAI_API_KEY`.** These names are one word apart and
> are two different credentials for two different providers. Setting the wrong one sends a
> real OpenAI key to whatever `OPENAI_COMPATIBLE_BASE_URL` names — read both names
> character-by-character before exporting either.

## Key Modules

- `provider_factory`: Provider selection and construction (table-driven registry —
  `provider_names()` lists only the providers this build actually compiled in).
- `config`: Provider configuration structures (`LlmConfig`, one `Option<LlmProviderConfig>`
  field per registered provider, structurally present regardless of compiled features).
- `compat`: Shared OpenAI-compatible request/response engine used by `kimi`, `qwen`, `grok`,
  `zai`, `ollama`, and `openai-compatible` (Gemini implements the `LlmPort` trait directly instead,
  since its wire protocol is not OpenAI-compatible).
- `error`: Error types for provider operations.
- `llm_analysis_service`: Higher-level LLM orchestration helpers.
- `openai`, `anthropic`, `deepseek`, `kimi`, `qwen`, `grok`, `ollama`, `gemini`, `zai`,
  `openai_compatible`, `mock`: Provider-specific adapters.

## Usage

```rust
use paladin_llm::provider_factory::LlmProviderFactory;

// Create providers by name at runtime.
let _factory = LlmProviderFactory::new();
```

## Feature Flags

- `default = ["openai", "mock"]`
- `openai`: Enable OpenAI provider adapter.
- `anthropic`: Enable Anthropic provider adapter.
- `deepseek`: Enable DeepSeek provider adapter.
- `kimi`: Enable Kimi (Moonshot AI) provider adapter.
- `qwen`: Enable Qwen (DashScope) provider adapter.
- `grok`: Enable Grok (xAI) provider adapter.
- `ollama`: Enable Ollama (self-hosted) provider adapter.
- `gemini`: Enable Gemini provider adapter.
- `zai`: Enable Z.ai (Zhipu AI, GLM) provider adapter.
- `openai-compatible`: Enable the generic operator-configured OpenAI-compatible provider
  adapter (see above).
- `mock`: Enable mock provider for tests.
- `vision`: Enable multimodal support on compatible providers.
- `openai-embeddings`: Enable OpenAI embedding utilities.

At the workspace level, the `paladin-ai` facade crate exposes one `llm-<provider>` flag per
provider above (e.g. `llm-openai-compatible`), each forwarding into the matching
`paladin-llm` feature, plus `llm-all` to enable every provider at once. The facade's compiled
*default* provider set is `openai` + `anthropic` + `deepseek` — unchanged from before this
crate grew six more providers (see the root `Cargo.toml` `[features] default` and its
D-11 comment). Enabling any other provider requires opting in explicitly via
`--features llm-<provider>` or `--features llm-all`.
