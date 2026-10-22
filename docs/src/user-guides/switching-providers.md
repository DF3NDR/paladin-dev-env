# Switching Providers

This page explains how to move an agent from one LLM provider or model to another, what Paladin
does when the target cannot honour a setting, and exactly what each provider and model was
measured to do. Every behaviour stated below is backed by the model registry
(`crates/paladin-llm/src/registry/seeded.rs`), by a test, or by the operator's live seed probe
(`.planning/phases/50-provider-parity-config-driven-model-swap/50-SEED-PROBE.md`, run on
2026-10-15). Where a behaviour was not measured, the page says so instead of presenting it as fact.

## What "switching providers" means

The goal of Phase 50 is that **one agent definition runs on any of the nine providers, changed by
configuration alone, and either behaves equivalently or fails with an error that names the
provider, the model and the parameter**. It never silently runs with a different setting.

The nine providers, in the order every table on this page uses: `openai`, `anthropic`,
`deepseek`, `kimi`, `qwen`, `grok`, `gemini`, `ollama` and `openai-compatible`.

Three mechanisms make that hold:

- **A per-model profile** (`ModelProfile`, ADR-0061) states what each model accepts for
  `temperature`, `top_p`, the two penalties and stop sequences, plus its context window, maximum
  output, reasoning flag and JSON modes.
- **A shared wire filter** applies the profile to every request on every adapter. An explicit value
  the model cannot honour is refused when the agent is built; a framework-default value is omitted
  and recorded.
- **One construction path** (`LlmProviderFactory::create_from_config`) builds any adapter from the
  `llm.<provider>` configuration block, so the only per-provider difference in an application is
  the provider name and model id.

## Switching through configuration

An agent names its provider and model; the `llm.<provider>` block supplies the endpoint, the
credential and the limits:

```yaml
llm:
  default_provider: "openai"

  anthropic:
    api_key: "${ANTHROPIC_API_KEY}"
    default_model: "claude-haiku-5-5"
    timeout_seconds: 300
    max_retries: 3
    max_tokens: 4096          # output budget sent when a request sets none
    context_window: 1000000   # adapter-wide window in tokens
    models:                   # per-model profile overrides, by exact model id
      claude-haiku-5-5:
        max_output_tokens: 64000

agents:
  - id: "researcher"
    provider: "anthropic"     # optional; defaults to llm.default_provider
    model: "claude-haiku-5-5"
    system_prompt: "You research topics thoroughly."
```

Moving that agent to Kimi changes two lines (`provider` and `model`) and, if it is not already
there, adds an `llm.kimi` block. Nothing in application code changes.

**Keys in an `llm.<provider>` block.** Every key is optional.

| Key | Meaning |
|-----|---------|
| `api_key` | The credential. Absent or empty falls back to the provider's own environment variable (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, ...); keyless Ollama needs none. |
| `base_url`, `default_model`, `timeout_seconds`, `max_retries` | Endpoint, model used when an agent names none, HTTP timeout, retry budget. |
| `default_temperature` | A temperature written by the operator. It is judged against the model's profile like any explicit value, so remove it where the default model does not admit it. |
| `max_tokens` | Output budget sent when a request sets none. Checked against the model's maximum output. |
| `context_window` | Adapter-wide window, replacing every compiled window of that provider. |
| `models.<model-id>.*` | Per-model profile override: `context_window`, `max_output_tokens`, `temperature`, `top_p`, `frequency_penalty`, `presence_penalty` (each `{range: [lo, hi]}`, `{fixed: v}` or `not_sent`), `stop_sequences`, `reasoning`, `json_object`, `json_schema`. An unknown key is a load error naming it. |

**`${VAR}` placeholders.** An `api_key` that is exactly `"${NAME}"` is read from the environment
variable `NAME`. Text such as `abc${X}` or `$X` is taken literally. A placeholder whose variable is
unset or blank is a construction error naming the provider, the model and the variable; the
literal placeholder text is never sent to a vendor as a key.

**Precedence (D-13), field by field, the most specific operator statement wins:**

| Order | Source |
|-------|--------|
| 1 | The agent's `model` (only the model) |
| 2 | The `llm.<provider>` block |
| 3 | The provider's own environment variables (the ones its `from_env` reads) |
| 4 | The adapter's compiled default |

An environment variable never overrides a value written in the block.

**The 300 s reasoning default.** When neither the block nor the environment sets
`timeout_seconds` and the resolved model is a reasoning model (a seeded entry, or a
`models.<id>.reasoning: true` override), the HTTP timeout is 300 seconds. Any other model keeps
the adapter's own default. `AgentDefinition.timeout_seconds` is
not the HTTP timeout and is never read when an adapter is built.

## Switching in code

Build an `LlmConfig`, hand it to the factory, and swap providers by changing the provider name and
model id:

```rust,ignore
use paladin::{LlmConfig, LlmProviderConfig, LlmProviderFactory};

let mut config = LlmConfig::default();
config.kimi = Some(
    LlmProviderConfig::new("${MOONSHOT_API_KEY}")
        .with_default_model("kimi-k2.6")
        .with_max_tokens(4096),
);

// provider name and model are the only per-provider inputs
let port = LlmProviderFactory::new().create_from_config(&config, "kimi", Some("kimi-k2.6"))?;
```

`LlmProviderFactory::is_configured` reports whether a provider can be built from a configuration
and the environment without building it.

To construct one adapter directly, each provider has a `*Config` type (`OpenAIConfig`,
`AnthropicConfig`, `DeepSeekConfig`, `KimiConfig`, `QwenConfig`, `GrokConfig`, `GeminiConfig`,
`OllamaConfig`, `OpenAiCompatibleConfig`). All nine are `#[non_exhaustive]`: build one with its
`new(..)` or `from_env()` and set knobs through the builders `with_max_tokens`, `with_max_retries`,
`with_context_window`, `with_model_overrides`, `with_timeout_seconds` and `with_base_url` (or by
assigning the public fields):

```rust,ignore
use std::collections::BTreeMap;
use paladin::{AnthropicAdapter, AnthropicConfig, ModelOverride};

let config = AnthropicConfig::new(api_key, "claude-haiku-5-5".to_string())
    .with_base_url(base_url)
    .with_max_tokens(8192)
    .with_max_retries(2)
    .with_context_window(1_000_000)
    .with_model_overrides(BTreeMap::<String, ModelOverride>::new());
let adapter = AnthropicAdapter::new(config)?;
```

Seven of the nine construct with the same shape, `new(api_key, model)` (Phase 50.1, SURFACE-02,
MIGRATION.md section 9.8 DR-19): `OpenAIConfig`, `AnthropicConfig`, `DeepSeekConfig`,
`KimiConfig`, `QwenConfig`, `GrokConfig` and `GeminiConfig`. The first argument is the API key,
never a URL. The endpoint defaults to the vendor's own (`ANTHROPIC_DEFAULT_BASE_URL` and its
siblings) and is replaced with `with_base_url`. The two endpoint-only configs keep their
endpoint argument: `OllamaConfig::new(base_url, model)` and
`OpenAiCompatibleConfig::new(api_key, base_url, model, capabilities)`.

`AnthropicConfig::new` sends `max_tokens` 4,096 on a request that sets none, for every model, the
same value `from_env()` and the layered `llm.anthropic` block use. A model's larger maximum
output is yours to ask for: `.with_max_tokens(16_000)`.

A direct constructor applies the same 300 s default to a seeded reasoning model, but it cannot see
a model marked reasoning only through `model_overrides`; call `with_timeout_seconds` yourself on the
direct path in that case.

All nine adapters are re-exported from the `paladin` facade, each behind its own feature:
`llm-openai`, `llm-anthropic`, `llm-deepseek`, `llm-kimi`, `llm-qwen`, `llm-grok`, `llm-gemini`,
`llm-ollama` and `llm-openai-compatible` (`llm-all` enables every one). The default feature set is
`openai`, `anthropic` and `deepseek` (ADR-0046); the other six compile in only when named.

## Parameter policy (ADR-0061)

| Situation | What happens |
|-----------|--------------|
| An **explicit** value the model cannot honour: `PaladinBuilder::temperature`, the agent `temperature:` key, `llm.<provider>.default_temperature`, or more `add_stop_word` / `stop_words:` entries than the model's stop maximum | The build fails with an error naming the provider, the model and the parameter. |
| **Stop words** on a model that takes no wire `stop` | The build succeeds. The wire omits `stop` (recorded as `paladin.params_omitted`) and the execution service enforces the list client-side: an output containing a stop word aborts the run with `StopWordDetected`. On a model that does take `stop`, the list is also sent, so the vendor may end generation at the word instead; the client-side check then does not fire. |
| A **framework-default or auto-selected** value the model cannot honour: the `auto_temperature` heuristic, the dynamic-temperature schedule, the `0.7` the default `PaladinData` carries | The value is omitted, with one `warn!` per adapter instance per (model, parameter) and a `paladin.params_omitted` metadata entry. |
| A model with no registry entry and no override | The adapter-wide fallback profile applies, with a `warn!` and the `paladin.unprofiled_model` metadata entry. An unknown model is never refused. |
| A requested JSON mode the model's profile does not support | A JSON-schema request is downgraded to JSON-object, or dropped when the model has no JSON mode, with a `warn!` and the `paladin.response_format_adjusted` metadata entry (`json_object` or `omitted`). |
| An effective `max_tokens` above the model's maximum output | Refused before anything is sent, with `LlmError::InvalidPrompt` naming the provider, the model, `max_tokens`, the value and the limit. Nothing is clamped. |

Nothing in this policy substitutes a value: there is no clamping into a range, no snapping to a
single accepted value and no rounding.

**Window precedence.** For one provider and model the profile is resolved in this order, first
match wins: the per-model override (`llm.<provider>.models.<id>`, `*Config.model_overrides`), then
the adapter-wide `context_window`, then the seeded registry entry, then the adapter-wide
fallback. The `context_window` knob applies to the window field only. The Commissary (through
`Commissary::for_model`) and the history trimmer ration prompts against the resolved window of the
model actually in use.

**Where the keys are visible (owner reading R-1, decided at plan 50-18).** The `paladin.*` keys
reach every caller, buffered and streamed:

| Surface | Carries |
|---------|---------|
| `LlmResponse.metadata` (each buffered call) | `paladin.params_omitted`, `paladin.unprofiled_model`, `paladin.response_format_adjusted`, `paladin.usage_unreconciled` (a call whose mapped usage disagrees with the vendor's own total or breaks containment; see [Responses](#responses)), and for a fallback chain `paladin.served_by` and `paladin.served_model`. `paladin.usage_unreconciled` follows the last-value rule below, so a run's `run_metadata` keeps the last call that recorded a disagreement. |
| `PaladinResult.run_metadata` (a whole run) | The same keys merged across every model call of the run. `paladin.params_omitted` and `paladin.unprofiled_model` hold the de-duplicated union in first-seen order; every other key holds the last call's value; a key without the `paladin.` prefix is never copied. `paladin.served_by` keeps its own `PaladinResult.served_by` field. |
| `StreamingResponse.metadata` (a stream) | The same keys on the stream's **terminal chunk only**, for all nine adapters; content chunks carry an empty map, and an error terminal item (refusal, empty completion) carries none. A fallback chain's stream names the serving hop with `paladin.served_by` and `paladin.served_model` on that chunk. |
| `ChunkMetadata.run_metadata` (a streamed run) | The terminal chunk's keys merged by the same rule, including `paladin.served_by`. |

The values are provider names, model ids, parameter names and token counts only, never request
content or credentials. The HTTP `ExecuteResponse` of `paladin-web` does not carry `run_metadata`; exposing it
over HTTP is a separate decision about a published contract.

**Known limits.**

- A caller that builds an `LlmRequest` directly, bypassing `PaladinBuilder`, gets omit, `warn!` and
  metadata, not a refusal: a bare request carries no record of which values the caller wrote
  themselves.
- Seven of the nine providers were probed; **Ollama and OpenAI-compatible were skipped** (no daemon
  and no endpoint were configured), so they have no seeded models and run on adapter-wide values.
  Every model not in the table below does too, visibly unprofiled.
- The OpenAI adapter sends its output limit as `max_completion_tokens` for every model, on the chat
  and the vision path; Paladin's own names (`OpenAIConfig.max_tokens`, `PromptParameters.max_tokens`)
  are unchanged. `gpt-5.4-nano` and `gpt-4o-mini` were measured accepting that key (2026-10-15),
  and so were the 17 OpenAI ids seeded on 2026-10-20: every one answered `max_tokens` with "Use
  'max_completion_tokens' instead" and none refused `max_completion_tokens` (`gpt-5-mini`
  answered 400 "model output limit was reached" at a 64-token cap, which is a spent budget, not a
  refused key). Other OpenAI model ids, and any non-OpenAI server behind `OPENAI_BASE_URL`, are
  unmeasured; for a server that accepts only `max_tokens`, use the `openai-compatible` preset,
  which still sends it.
- The retry counters are not unified: the compat presets (Kimi, Qwen, Grok, Ollama,
  OpenAI-compatible), OpenAI and DeepSeek retry that many times after the first attempt (`0` sends
  once); Anthropic and Gemini count total attempts (`0` and `1` both send once).
- Grok's vendor-computed `cost_in_usd_ticks` and `num_sources_used` are not parsed, so xAI's own
  cost figure is not compared with the Treasurer's price on any call. The 2026-10-18 usage probe
  recorded xAI's figure once, as a cross-check (see the grok row below). The owner deferred this
  on 2026-10-17, and it stays open on the `WINDOWS.md` ledger.

## Per-provider and per-model behaviour

One row per seeded model, then one adapter-wide fallback row per provider. Rows are ordered by
provider in the fixed order above, then by model id ascending, and a drift test
(`switching_providers_page_lists_every_seeded_model`) fails when a seeded model is missing from the
page, extra, out of order, or listed with a different value or verified-on date than its registry
entry. A seeded row's verified-on date names the seed probe transcript its values come from:
2026-10-15 is `50-SEED-PROBE.md`, 2026-10-20 is `50.1-SEED-PROBE.md`. The fallback rows describe
what each adapter sends today and are **unverified**: they apply to any model the registry does not
list.

| Provider | Model | Verified on | Context window | Max output | Temperature | top_p | Penalties | Stop sequences | Reasoning | JSON mode |
|----------|-------|-------------|----------------|------------|-------------|-------|-----------|----------------|-----------|-----------|
| openai | `chat-latest` | 2026-10-20 [g] | 400,000 [d] | 128,000 [d] | only 1.0 | not sent | not sent | not sent | no | object, schema |
| openai | `gpt-4o-mini` | 2026-10-15 | 128,000 [d] | 16,384 [d] | 0.0 to 1.0 [a] | 0.0 to 1.0 [c] | not sent [e] | up to 4 | no | object, schema |
| openai | `gpt-5` | 2026-10-20 | 400,000 [d] | 128,000 [d] | only 1.0 | not sent | not sent | not sent | no [h] | object, schema |
| openai | `gpt-5-mini` | 2026-10-20 | 400,000 [d] | 128,000 [d] | only 1.0 | not sent | not sent | not sent | no [h] | object, schema |
| openai | `gpt-5-nano` | 2026-10-20 | 400,000 [d] | 128,000 [d] | only 1.0 | not sent | not sent | not sent | no [h] | object, schema |
| openai | `gpt-5.1` | 2026-10-20 | 400,000 [d] | 128,000 [d] | 0.0 to 1.0 [a] | 0.0 to 1.0 [c] | not sent [e] | not sent | no | object, schema |
| openai | `gpt-5.2` | 2026-10-20 | 400,000 [d] | 128,000 [d] | 0.0 to 1.0 [a] | 0.0 to 1.0 [c] | not sent [e] | not sent | no | object, schema |
| openai | `gpt-5.4` | 2026-10-20 | 1,050,000 [d] | 128,000 [d] | 0.0 to 1.0 [a] | 0.0 to 1.0 [c] | not sent [e] | not sent | no | object, schema |
| openai | `gpt-5.4-mini` | 2026-10-20 | 400,000 [d] | 128,000 [d] | 0.0 to 1.0 [a] | 0.0 to 1.0 [c] | not sent [e] | not sent | no | object, schema |
| openai | `gpt-5.4-nano` | 2026-10-15 | 400,000 [d] | 128,000 [d] | 0.0 to 1.0 [a] | 0.0 to 1.0 [c] | not sent [e] | not sent | no | object, schema |
| openai | `gpt-5.5` | 2026-10-20 | 1,050,000 [d] | 128,000 [d] | only 1.0 | not sent | not sent | not sent | no | object, schema |
| openai | `gpt-5.6-luna` | 2026-10-20 | 1,050,000 [d] | 128,000 [d] | only 1.0 | not sent | not sent | not sent | no | object, schema |
| openai | `gpt-5.6-sol` | 2026-10-20 | 1,050,000 [d] | 128,000 [d] | only 1.0 | not sent | not sent | not sent | no | object, schema |
| openai | `gpt-5.6-terra` | 2026-10-20 | 1,050,000 [d] | 128,000 [d] | only 1.0 | not sent | not sent | not sent | no | object, schema |
| openai | `gpt-6-astra` | 2026-10-20 | 1,050,000 [d] | 128,000 [d] | only 1.0 | not sent | not sent | not sent | no | object, schema |
| openai | `gpt-6-luna` | 2026-10-20 | 1,050,000 [d] | 128,000 [d] | only 1.0 | not sent | not sent | not sent | no | object, schema |
| openai | `gpt-6-sol` | 2026-10-20 | 1,050,000 [d] | 128,000 [d] | only 1.0 | not sent | not sent | not sent | no | object, schema |
| openai | `gpt-6.1-sol` | 2026-10-20 | 1,050,000 [d] | 128,000 [d] | only 1.0 | not sent | not sent | not sent | no | object, schema |
| openai | `o3` | 2026-10-20 | 200,000 [d] | 100,000 [d] | only 1.0 | not sent | not sent | not sent | no | object, schema |
| openai | (adapter-wide fallback) | unverified | 128,000 | not set | 0.0 to 1.0 | 0.0 to 1.0 | not sent | up to 4 | no | object, schema |
| anthropic | `claude-fable-5` | 2026-10-20 | 1,000,000 | 128,000 | only 1.0 | not sent | not sent | yes | yes | none |
| anthropic | `claude-fable-5-1` | 2026-10-20 | 1,000,000 | 128,000 | only 1.0 | not sent | not sent | yes | no | none |
| anthropic | `claude-haiku-5-5` | 2026-10-15 | 1,000,000 | 128,000 | only 1.0 | not sent | not sent | yes | no | none |
| anthropic | `claude-opus-4-6` | 2026-10-20 | 1,000,000 | 128,000 | 0.0 to 1.0 | 0.0 to 1.0 [c] | not sent | yes | no | none |
| anthropic | `claude-opus-4-7` | 2026-10-20 | 1,000,000 | 128,000 | only 1.0 | not sent | not sent | yes | no | none |
| anthropic | `claude-opus-4-8` | 2026-10-20 | 1,000,000 | 128,000 | only 1.0 | not sent | not sent | yes | no | none |
| anthropic | `claude-opus-5` | 2026-10-20 | 1,000,000 | 128,000 | only 1.0 | not sent | not sent | yes | yes | none |
| anthropic | `claude-opus-5-5` | 2026-10-20 | 1,000,000 | 128,000 | only 1.0 | not sent | not sent | yes | no | none |
| anthropic | `claude-sonnet-4-6` | 2026-10-20 | 1,000,000 | 128,000 | 0.0 to 1.0 | 0.0 to 1.0 [c] | not sent | yes | no | none |
| anthropic | `claude-sonnet-5` | 2026-10-20 | 1,000,000 | 128,000 | only 1.0 | not sent | not sent | yes | no | none |
| anthropic | `claude-sonnet-5-5` | 2026-10-20 | 1,000,000 | 128,000 | only 1.0 | not sent | not sent | yes | no | none |
| anthropic | (adapter-wide fallback) | unverified | 1,000,000 | not set | not sent | not sent | not sent | yes | no | none |
| deepseek | `deepseek-flash` | 2026-10-15 | 1,048,576 | 393,216 | 0.0 to 2.0 | 0.0 to 1.0 [c] | -2.0 to 2.0 [c] | yes | yes | object |
| deepseek | `deepseek-v4-pro` | 2026-10-20 | 1,048,576 | 393,216 | 0.0 to 2.0 | 0.0 to 1.0 [c] | -2.0 to 2.0 [c] | yes | yes | object |
| deepseek | (adapter-wide fallback) | unverified | 1,048,576 | not set | 0.0 to 2.0 | 0.0 to 1.0 | -2.0 to 2.0 | yes | no | object |
| kimi | `kimi-k2.6` | 2026-10-15 | 262,144 | not set | not sent | not sent | not sent | yes | yes | object, schema |
| kimi | `kimi-k3` | 2026-10-15 | 1,048,576 | not set | not sent | not sent | not sent | yes | yes | object, schema |
| kimi | (adapter-wide fallback) | unverified | 262,144 | not set | not sent [f] | not sent | not sent | yes | no | object |
| qwen | `qwen-flash` | 2026-10-20 | 1,000,000 [d] | 32,768 [d] | 0.0 to 1.99 | 0.0 to 1.0 [c] | -2.0 to 2.0 [c] | yes | no | object, schema |
| qwen | `qwen-max` | 2026-10-20 | 32,768 [d] | 8,192 [d] | 0.0 to 1.99 | 0.0 to 1.0 [c] | -2.0 to 2.0 [c] | yes | no | object, schema |
| qwen | `qwen-plus` | 2026-10-15 | not set | not set | 0.0 to 1.99 | 0.0 to 1.0 [c] | -2.0 to 2.0 [c] | yes | no | object, schema |
| qwen | `qwen-plus-latest` | 2026-10-20 [g] | 1,000,000 [d] | 32,768 [d] | 0.0 to 1.99 | 0.0 to 1.0 [c] | -2.0 to 2.0 [c] | yes | no | object, schema |
| qwen | `qwen3.5-flash` | 2026-10-20 | 1,000,000 [d] | 65,536 [d] | 0.0 to 1.99 | 0.0 to 1.0 [c] | -2.0 to 2.0 [c] | yes | yes | object, schema |
| qwen | `qwen3.5-plus` | 2026-10-20 | 1,000,000 [d] | 65,536 [d] | 0.0 to 1.99 | 0.0 to 1.0 [c] | -2.0 to 2.0 [c] | yes | yes | object, schema |
| qwen | `qwen3.6-flash` | 2026-10-20 | 1,000,000 [d] | 65,536 [d] | 0.0 to 1.99 | 0.0 to 1.0 [c] | -2.0 to 2.0 [c] | yes | yes | object, schema |
| qwen | `qwen3.6-plus` | 2026-10-20 | 1,000,000 [d] | 65,536 [d] | 0.0 to 1.99 | 0.0 to 1.0 [c] | -2.0 to 2.0 [c] | yes | yes | object, schema |
| qwen | `qwen3.7-flash` | 2026-10-20 | 1,000,000 [d] | 131,072 [d] | 0.0 to 1.99 | 0.0 to 1.0 [c] | -2.0 to 2.0 [c] | yes | yes | object, schema |
| qwen | `qwen3.7-max` | 2026-10-20 | 1,000,000 [d] | 131,072 [d] | 0.0 to 1.99 | 0.0 to 1.0 [c] | -2.0 to 2.0 [c] | yes | yes | object, schema |
| qwen | `qwen3.7-plus` | 2026-10-20 | 1,000,000 [d] | 131,072 [d] | 0.0 to 1.99 | 0.0 to 1.0 [c] | -2.0 to 2.0 [c] | yes | yes | object, schema |
| qwen | `qwen3.8-flash` | 2026-10-15 | 1,000,000 [d] | 131,072 [d] | 0.0 to 1.99 | 0.0 to 1.0 [c] | -2.0 to 2.0 [c] | yes | yes | object, schema |
| qwen | `qwen3.8-max` | 2026-10-20 | 1,000,000 [d] | 131,072 [d] | 0.0 to 1.99 | 0.0 to 1.0 [c] | -2.0 to 2.0 [c] | yes | yes | object, schema |
| qwen | (adapter-wide fallback) | unverified | 1,000,000 | not set | 0.0 to 1.99 | 0.0 to 1.0 | -2.0 to 2.0 | yes | no | object |
| grok | `grok-4.20-non-reasoning` | 2026-10-20 | 1,000,000 | not set | 0.0 to 2.0 | 0.0 to 1.0 [c] | not sent | yes | no | object, schema |
| grok | `grok-4.20-reasoning` | 2026-10-20 | 1,000,000 | not set | 0.0 to 2.0 | 0.0 to 1.0 [c] | not sent | not sent | yes | object, schema |
| grok | `grok-4.3` | 2026-10-15 | 1,000,000 | not set | 0.0 to 1.99 [b] | 0.0 to 1.0 [c] | not sent | not sent | yes | object, schema |
| grok | `grok-4.5` | 2026-10-15 | 500,000 | not set | 0.0 to 1.99 [b] | 0.0 to 1.0 [c] | not sent | not sent | yes | object, schema |
| grok | `grok-4.6` | 2026-10-15 | 500,000 | not set | 0.0 to 1.99 [b] | 0.0 to 1.0 [c] | not sent | not sent | yes | object, schema |
| grok | `grok-4.7` | 2026-10-20 | 500,000 | not set | 0.0 to 1.99 [b] [i] | 0.0 to 1.0 [c] | not sent | not sent | yes | object, schema |
| grok | (adapter-wide fallback) | unverified | 1,000,000 | not set | 0.0 to 2.0 | 0.0 to 1.0 | not sent | not sent | no | object |
| gemini | `gemini-3-flash-preview` | 2026-10-20 [g] | 1,048,576 | 65,536 | 0.0 to 2.0 | 0.0 to 1.0 [c] | not sent | yes | yes | object, schema |
| gemini | `gemini-3.1-flash-lite` | 2026-10-20 | 1,048,576 | 65,536 | 0.0 to 2.0 | 0.0 to 1.0 [c] | not sent | yes | yes | object, schema |
| gemini | `gemini-3.1-pro-preview` | 2026-10-20 [g] | 1,048,576 | 65,536 | 0.0 to 2.0 | 0.0 to 1.0 [c] | not sent | yes | yes | object, schema |
| gemini | `gemini-3.5-flash` | 2026-10-15 | 1,048,576 | 65,536 | 0.0 to 2.0 | 0.0 to 1.0 [c] | not sent | yes | yes | object, schema |
| gemini | `gemini-3.5-flash-lite` | 2026-10-15 | 1,048,576 | 65,536 | 0.0 to 2.0 | 0.0 to 1.0 [c] | not sent | yes | yes | object, schema |
| gemini | `gemini-3.6-flash` | 2026-10-15 | 1,048,576 | 65,536 | 0.0 to 2.0 | 0.0 to 1.0 [c] | not sent | yes | yes | object, schema |
| gemini | `gemini-3.7-flash` | 2026-10-20 | 1,048,576 | 65,536 | 0.0 to 2.0 | 0.0 to 1.0 [c] | not sent | yes | yes | object, schema |
| gemini | `gemini-3.8-flash` | 2026-10-20 | 1,048,576 | 65,536 | 0.0 to 2.0 | 0.0 to 1.0 [c] | not sent | yes | yes | object, schema |
| gemini | `gemini-4-argon` | 2026-10-20 | 1,048,576 | 32,768 | 0.0 to 2.0 | 0.0 to 1.0 [c] | not sent | yes | yes | object, schema |
| gemini | `gemini-flash-latest` | 2026-10-20 [g] | 1,048,576 | 65,536 | 0.0 to 2.0 | 0.0 to 1.0 [c] | not sent | yes | yes | object, schema |
| gemini | `gemini-flash-lite-latest` | 2026-10-20 [g] | 1,048,576 | 65,536 | 0.0 to 2.0 | 0.0 to 1.0 [c] | not sent | yes | yes | object, schema |
| gemini | `gemini-pro-latest` | 2026-10-20 [g] | 1,048,576 | 65,536 | 0.0 to 2.0 | 0.0 to 1.0 [c] | not sent | yes | yes | object, schema |
| gemini | (adapter-wide fallback) | unverified | 1,048,576 | not set | 0.0 to 2.0 | 0.0 to 1.0 | not sent | yes | no | object, schema |
| ollama | (adapter-wide fallback) | unverified | not set | not set | 0.0 to 2.0 | 0.0 to 1.0 | -2.0 to 2.0 | yes | no | object |
| openai-compatible | (adapter-wide fallback) | unverified | not set | not set | 0.0 to 1.0 | 0.0 to 1.0 | -2.0 to 2.0 | yes | no | object |

Reading the cells: `0.0 to 1.0` is an inclusive range, `only 1.0` accepts exactly that value, and
`not sent` means the parameter is never put on the wire (the vendor's own default applies, and an
explicit value is refused at build). `not set` means no value is recorded: no limit is assumed. A
window of `not set` is not a statement that the model has none.

- **[a]** OpenAI temperature: the probe accepted 0.0, 0.5, 1.0 and 1.01 (the vendor accepted a value
  beyond the declared end), so the upper bound was not located. The registry holds the highest
  probed value that was accepted, 1.0. This applies to `gpt-5.4-nano` and `gpt-4o-mini` and to the
  2026-10-20 ids `gpt-5.1`, `gpt-5.2`, `gpt-5.4` and `gpt-5.4-mini`; every other OpenAI row
  accepts only 1.0 ("Only the default (1) value is supported", measured) and refuses `top_p`.
- **[b]** Grok temperature: 0.0 and 0.5 were accepted and 2.01 was rejected ("Temperature must be
  less than 2"), but the 2.00 probe was unreachable (a transport error: three times on 2026-10-15
  for the first three rows, and in both 2026-10-20 runs for `grok-4.7`). The registry holds 1.99,
  the largest value under the bound the vendor's own text states. The bound is stated, not
  measured. The two `grok-4.20-*` rows were accepted at 2.00 and read `0.0 to 2.0`.
- **[c]** Contract-derived: `top_p` and the penalties were probed at 0.5 only. An accepted value is
  recorded as the vendor contract's span (`top_p` 0 to 1, penalties -2 to 2), not a measured edge.
- **[d]** The window and maximum output come from the vendor's documentation page (URL and access
  date in the seed probe, section (b)), not from the models listing. The `qwen3.8-flash` figures
  come from the China/Beijing page (the International pages stated no limits on 2026-10-15); the
  OpenAI and Qwen rows dated 2026-10-20 come from the vendor's own model pages (OpenAI's
  `developers.openai.com` model pages and Alibaba Cloud's International Model Info pages, accessed
  2026-10-20). Where the docs state no value the cell reads `not set`.
- **[e]** OpenAI penalties: the vendor accepted 0.5 in the probe, but the adapter's request type
  has no penalty field, so the profile says `not sent` rather than admit a value the wire cannot
  carry. The same holds for Anthropic and Gemini, whose wires have no such field.
- **[f]** Kimi's fallback profile matches the seeded Kimi models: an unseeded Kimi model id sends
  none of `temperature`, `top_p` or the penalties (owner decision, 2026-10-17). The vendor accepts
  exactly one value for each, and that value depends on the thinking mode: `temperature` was "only
  1" on 2026-08-22 and "only 0.6" with thinking disabled on 2026-10-15, so no single value can be
  declared and omitting the parameter lets the vendor apply its own. An explicit value on an
  unseeded Kimi model is refused at build, naming `kimi`, the model and the parameter; a default
  the framework supplies is omitted and recorded in `paladin.params_omitted`.
- **[g]** A moving alias (`*-latest`, `chat-latest`) or a preview id: the row is true on its
  verified-on date only, because the vendor re-points an alias at a newer model without changing
  the id and may change or retire a preview without notice.
- **[h]** These OpenAI models returned no reasoning text (Chat Completions never does, so the
  column reads `no`), but a small output cap is spent before any answer: at `max_completion_tokens`
  64 `gpt-5` and `gpt-5-nano` finished `length` with empty content, and `gpt-5-mini` answered 400
  "model output limit was reached". Give such a model a larger output budget.
- **[i]** The `grok-4.7` exception (owner ruling, 2026-10-20). Its `temperature=2.00` probe failed
  at the vendor in both 2026-10-20 runs (`error sending request`, identical both times); 0.0 and
  0.5 were accepted and 2.01 was a clean 400, so the vendor's range is 0 to 2. The owner ruled that
  a reproducible vendor-side failure at the top endpoint, seen identically in two independent
  runs, counts as a measured outcome for that one row. The registry range therefore stops at 1.99
  and a caller cannot send 2.00 to `grok-4.7` (an explicit 2.0 is refused at build). The same
  probe did not fail identically for the sibling rows on 2026-10-20: `grok-4.3` was unreachable in
  both runs, `grok-4.6` was accepted in run 1 and unreachable in run 2, and `grok-4.5` answered
  500 ("Internal error during token parsing") in run 1 and was unreachable in run 2. Their rows
  are unchanged and still rest on the 2026-10-15 basis. 1.99 itself was not probed for `grok-4.7`. The window is the listing's `context_length`; no maximum output is
  set because the docs state no text output limit.

**Probed but not seeded** (so they run on the adapter-wide fallback): the eight retired Claude ids
(`claude-3-*`, `claude-2.*`, `claude-instant-1.2`, all HTTP 404) and the other Claude ids in the
live listing (the 4.5 generation; not candidates, not probed); `gemini-2.5-pro`, `gemini-2.5-flash`
and `gemini-2.5-flash-lite` (the default call answered HTTP 404, "no longer available to new
users", so nothing was measured); `gpt-5.5-pro`, `gpt-5.4-pro`, `gpt-5.2-pro` and `gpt-5-pro` (the default call answered HTTP 404 on
chat completions; they are served by the Responses API, which this adapter does not call, and are
tracked in `.planning/todos/pending/2026-10-20-openai-responses-api-for-pro-tier.md`);
`deepseek-chat` and `deepseek-coder` (answered 200 but are undocumented aliases);
`kimi-k2.7-code` and `kimi-k2.7-code-highspeed` (every probe was rejected); `qwen-turbo` (rate-limited on every probe, nothing measured) and `qwen3-max` (HTTP 404
on the International endpoint); the other Grok chat ids (`grok-4.20-multi-agent`,
`grok-build-0.1`; not probed). A seeded row matches its model id exactly: the dated
`grok-4.20-0309-*` ids and every other alias of a seeded model run on the fallback, and so does
any other Qwen or OpenAI id in a live listing.

## Responses

Every adapter maps the same outcomes to the same typed results, measured by one 19-case
conformance suite over all nine adapters. The suite drives mock vendor bodies shaped like each
vendor's published response contract; the live probe did not capture a vendor refusal or a
quota-exhaustion body.

- **Empty output** (empty text on any finish reason) is `LlmError::EmptyCompletion`, never an
  empty success.
- **A refusal** is `LlmError::Refusal { provider, model, detail, .. }`, `Permanent`, so a fallback
  chain stops on it by default. The detail is redacted before it is bounded. Match it with
  `LlmError::Refusal { .. }`; the variant is `#[non_exhaustive]`.
- **A blocked answer with no refusal signal** is `Ok` with `FinishReason::ContentFilter`.
- **Reasoning** text is on `LlmResponse.reasoning` (and `StreamingResponse.reasoning` while
  streaming) where the vendor returns it.
- **Usage** is mapped the same way on all nine adapters.
  - Reasoning and cache counts are on `TokenUsage`, and are `None` when the vendor did not report
    them.
  - `TokenUsage.total_tokens` is always `prompt_tokens + completion_tokens`.
  - Every billed category a vendor reports is folded into one of those two counts, so the counts
    the Treasurer prices are what the vendor bills. The folds are:
    - Grok reasoning tokens, which xAI reports beside `completion_tokens`, into
      `completion_tokens`;
    - Gemini `thoughtsTokenCount` into `completion_tokens`, and Gemini `toolUsePromptTokenCount`
      into `prompt_tokens`;
    - Anthropic cache reads and writes (`cache_read_input_tokens`, `cache_creation_input_tokens`)
      into `prompt_tokens`, on the chat path and the vision path alike.
  - Every other adapter's vendor already reports reasoning and cache counts inside their parents,
    and those counts are copied as reported. Sub-counts therefore always sit inside their parents.
- **The vendor's own total** is checked against the mapped total on every adapter except
  anthropic, which reports no total.
  - It is parsed on both paths (buffered and streamed) and compared with the mapped
    `prompt_tokens + completion_tokens`.
  - It is never copied over the mapped total.
  - A usage object without a total still parses, and the missing total is not a finding.
  - These are findings: a disagreement, a reasoning count above `completion_tokens`, or cache
    counts above `prompt_tokens`. On anthropic only the two containment checks apply.
  - A finding is logged once per model (target `paladin::usage`) and recorded under
    `paladin.usage_unreconciled` (for example `total:vendor=900,mapped=837`), on the buffered
    response or on the stream's terminal chunk.
  - So if a vendor changes how it reports reasoning, the change shows up on the first affected
    call; it is never priced silently.
- **Billing.** Every call a vendor bills is counted, priced and charged, whether it succeeds or
  fails.
  - A call the vendor answered and billed but that ends as `EmptyCompletion` or `Refusal` carries
    its bill on the error. `LlmError::billed_usage()` returns the billed model, its `TokenUsage`
    and, behind `PricingLlmAdapter`, its `Cost`. It looks through `AllProvidersFailed` to the last
    hop's error, and it is `None` when the vendor reported no usage object. All nine adapters fill
    it, buffered and streamed.
  - `PaladinExecutionService` folds a billed failure into the run's usage and cost and settles it
    to the treasury ledger under the call's ordinal. It also counts as its Battalion engine
    attempt's spend. This holds for a stream's terminal error and for an error from the call that
    opens the stream.
  - A fallback hop the chain moved past after a billed refusal (`hop_on_refusal(true)`) is priced
    against its own model and settled in its call's ledger row: one row per call, with a
    per-model breakdown. This holds whether a later hop serves the call or the chain ends in
    `AllProvidersFailed`, and on every exit of a stream, a dropped stream included. A streamed
    call's terminal chunk folds the hop bills into its cost, and into its usage when the provider
    reported the served call's usage.
  - A failed or timed-out Battalion attempt counts the spend it recorded (`ATTEMPT_SPEND`), and a
    structured attempt counts every repair call. A successful attempt still reports
    `PaladinResult.cost`, so nothing is counted twice.
  - A bill on a model with no `treasurer.pricing` row keeps `cost: None` and settles nothing, as
    an unpriced success does.
  - `TokenUsage.total_tokens` stays `prompt_tokens + completion_tokens` (D-02); the vendor's own
    total is the check on it, as above.
  - A caller that drives `LlmPort` directly reads a failed call's bill from the error. A caller
    that wants a failed execution's spend installs a `SpendScope` around the call with
    `ATTEMPT_SPEND.scope(..)` (`paladin_ports::output::spend_scope`; `MIGRATION.md` 9.8 DR-18).
- **Which readings are measured.** Each provider's reading of reasoning against
  `completion_tokens` carries a label in the reasoning column below:
  - *measured 2026-10-18*: confirmed by an operator-run probe on the raw vendor wire that day, one
    reasoning-eliciting call per seeded model. The probe recorded only the usage numbers.
  - *not live-measured*: the reading follows the vendor's documented contract. Either the probe
    could not settle it (no reasoning count, or no vendor total), or the preset fronts a server of
    your choosing.

| Provider | Empty output (buffered, streamed) | Refusal | Content filter | Usage cap | Cache read, write | Reasoning tokens, text | Stop sequences |
|----------|-----------------------------------|---------|----------------|-----------|-------------------|------------------------|----------------|
| openai | `EmptyCompletion`, both | typed, from `message.refusal` | `ContentFilter` | `UsageLimitExceeded` on a 429 coded `insufficient_quota` (unit-tested; not part of owner reading R-2; no quota body in the probe) | read yes (`cached_tokens`); write not reported | tokens yes, inside `completion_tokens` (not live-measured: the probe got 0 reasoning tokens from `gpt-5.4-nano` and `gpt-4o-mini`); text not mapped | `stop`, in declared order, up to the profile's maximum |
| anthropic | `EmptyCompletion`, both (streamed: at `message_stop`; a stream that ends without it is not classified) | typed, from `stop_reason: refusal`, checked before emptiness; detail from `stop_details` | not applicable: the refusal stop reason is the typed signal | `UsageLimitExceeded` on a 429 whose `error.details.error_code` is `enforced_spend_limit_reached`, or a 400 carrying the usage-cap signature (unit-tested; not part of R-2) | read yes; write yes (`cache_creation_input_tokens`); both folded into `prompt_tokens`, chat and vision | tokens yes (`thinking_tokens`), inside `output_tokens` (not live-measured: no vendor total to check against; the containment check guards it); text yes (thinking blocks; `thinking_delta` while streaming) | `stop_sequences`, no known maximum |
| deepseek | `EmptyCompletion`, both | no typed refusal field on the wire; a blocked answer arrives as a `content_filter` finish | `ContentFilter` | `UsageLimitExceeded` on HTTP 402 (the shared status mapping, unit-tested; not part of R-2) | read yes (`prompt_cache_hit_tokens`); write not reported | tokens yes, inside `completion_tokens` (measured 2026-10-18: `deepseek-flash`); text yes (`reasoning_content`, both paths) | `stop`, no known maximum |
| kimi | `EmptyCompletion`, both | typed, from `message.refusal` or the stream delta | `ContentFilter` | unverified -- owner accepted (2026-10-16) | read yes (`cached_tokens`); write yes (`cache_write_tokens`, observed on Kimi) | tokens yes, inside `completion_tokens` (measured 2026-10-18: `kimi-k2.6`, `kimi-k3`); text yes (`reasoning_content`) | `stop`, no known maximum |
| qwen | `EmptyCompletion`, both | typed, from `message.refusal` or the stream delta | `ContentFilter` | unverified -- owner accepted (2026-10-16) | read yes (`cached_tokens`); write mapped when reported, not observed | tokens yes, inside `completion_tokens` (measured 2026-10-18: `qwen3.8-flash`; `qwen-plus` reported none); text yes (`reasoning_content`) | `stop`, no known maximum |
| grok | `EmptyCompletion`, both | typed, from `message.refusal` or the stream delta | `ContentFilter` | unverified -- owner accepted (2026-10-16) | read yes (`cached_tokens`); write mapped when reported, not observed | tokens yes: xAI reports `reasoning_tokens` beside `completion_tokens`, and the adapter folds them into `completion_tokens` (measured 2026-10-18: `grok-4.3`, `grok-4.5`, `grok-4.6`; xAI's billed `cost_in_usd_ticks` matched reasoning billed at the completion rate); text yes (`reasoning_content`) | not sent: `stop` is omitted and recorded; an agent's stop words are enforced client-side |
| gemini | `EmptyCompletion`, both (streamed: at the item that carries `finishReason`; a stream with no such frame is not classified) | no refusal signal on the wire; `SAFETY`, `RECITATION`, `BLOCKLIST`, `PROHIBITED_CONTENT` and `SPII` finishes are `Ok(ContentFilter)` | `ContentFilter` | no usage-cap mapping: a 429 is a rate limit (the free-tier body of 2026-10-15 was one) | read yes (`cachedContentTokenCount`); write not reported | tokens yes (`thoughtsTokenCount`), reported beside `candidatesTokenCount` and folded into `completion_tokens` (measured 2026-10-18: `gemini-3.5-flash`, `gemini-3.6-flash`; `gemini-3.5-flash-lite` reported none); text not mapped | `generationConfig.stopSequences`, no known maximum |
| ollama | `EmptyCompletion`, both | typed, from `message.refusal` or the stream delta | `ContentFilter` | unverified -- owner accepted (2026-10-16) | read yes (`cached_tokens`); write mapped when reported, not observed | tokens yes, inside `completion_tokens` (not live-measured: the preset fronts whichever server you point it at); text yes (`reasoning_content`) | `stop`, no known maximum |
| openai-compatible | `EmptyCompletion`, both | typed, from `message.refusal` or the stream delta | `ContentFilter` | unverified -- owner accepted (2026-10-16) | read yes (`cached_tokens`); write mapped when reported, not observed | tokens yes, inside `completion_tokens` (not live-measured: the preset fronts whichever server you point it at); text yes (`reasoning_content`) | `stop`, no known maximum |

The usage-cap column follows owner reading R-2 (plan 50-17, 2026-10-16): no quota-exhaustion body
was captured for any of the five compat presets, so none has a vendor-specific usage-cap mapping.
Each keeps the shared status mapping (HTTP 402 is `UsageLimitExceeded`, a 429 is a rate limit) and
is recorded as unverified. An override is a single `error_override` that can be added on the day a
quota body is captured.

Two stream behaviours are asserted by name in the suite: Anthropic skips an unparsable event line
with a `warn!` and keeps going, where the other eight adapters end the stream with an error; and
DeepSeek has no typed refusal.

Gemini's blocked-content finishes `RECITATION`, `BLOCKLIST`, `PROHIBITED_CONTENT` and `SPII` used
to surface as `Error(<finish reason>)`; they are now `ContentFilter`, like `SAFETY`.

## Fallback chains

A fallback chain whose hops must each send their own model is built from `FallbackStep` values:

```rust,ignore
use paladin::{FallbackLlmAdapter, FallbackStep};

let chain = FallbackLlmAdapter::from_steps(vec![
    FallbackStep::new(anthropic_port, "claude-haiku-5-5"),
    FallbackStep::new(kimi_port, "kimi-k2.6"),
])?
.hop_on_refusal(false);
```

Each hop is sent its own model in place of the request's and is paced under its own
`(provider, model)`. An empty step model fails with `FallbackChainError::EmptyStepModel`. The
chain's profile is the intersection of every hop's profile for that hop's own model, so the build
refuses what any hop could not honour.

In configuration the same chain is `agent_runtime.model_fallback.steps`:

```yaml
agent_runtime:
  model_fallback:
    enabled: true
    steps:
      - { provider: "anthropic", model: "claude-haiku-5-5" }
      - { provider: "kimi", model: "kimi-k2.6" }
```

`steps` and `providers` are two spellings of one chain and are mutually exclusive. Each step is
built from that provider's `llm.<provider>` block, so the credential never appears in the step.

A **refusal stops the chain** by default, because a refusal is `Permanent`. Build the chain with
`hop_on_refusal(true)` to try the next step instead; the configuration has no switch for this.

Every response served through a chain records the serving hop: `paladin.served_by` (the provider)
and `paladin.served_model` (the model that hop was sent), on `LlmResponse.metadata` and, for a
stream, on the terminal chunk.

## How the table is maintained

The seeded rows come from one document: the operator's live seed probe of 2026-10-15
(`50-SEED-PROBE.md`), run on the operator's own vendor keys. A value with no evidence there is not
seeded; a model measured only in part is not listed. Each registry entry carries a `verified_on`
date, and the table repeats it.

To refresh a vendor, re-run the probe (`cargo run -p paladin-llm --example live_vendor_smoke
--features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatible -- --probe`;
a provider with no credential in the environment prints `SKIPPED`), update the vendor's table in `registry/seeded.rs` with
the new date, and update the matching rows above. The drift test fails until the page and the
registry agree, so a row cannot be edited on one side only. The live smoke run over all nine
providers is credential-gated and is a manual check, not part of CI.

See also: [Provider Expansion](../appendix/provider-expansion.md), the contributor guide
[Contributing New LLM Providers](../contributing/contributing-providers.md), and
[Paladin Configuration](paladin-configuration.md).

<!-- Footnote markers [a] to [i] in the behaviour table. They are defined here so
mdbook-linkcheck (warning-policy = error) does not read each one as an incomplete link; the
switching_page_tests drift test still reads the raw markers. -->
[a]: #per-provider-and-per-model-behaviour
[b]: #per-provider-and-per-model-behaviour
[c]: #per-provider-and-per-model-behaviour
[d]: #per-provider-and-per-model-behaviour
[e]: #per-provider-and-per-model-behaviour
[f]: #per-provider-and-per-model-behaviour
[g]: #per-provider-and-per-model-behaviour
[h]: #per-provider-and-per-model-behaviour
[i]: #per-provider-and-per-model-behaviour
