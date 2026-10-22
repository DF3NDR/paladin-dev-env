# Reasoning Effort

This page explains how to tell a model how hard to think, what Paladin does when the model cannot
do what you asked, and exactly what each provider and model was measured to accept. Every
behaviour stated below is backed by the model registry (`crates/paladin-llm/src/registry/seeded.rs`),
by a test, or by the operator's live effort probe (`51-EFFORT-PROBE.md`, run on 2026-10-22 against
seven of the nine providers). Where nothing was measured, the page says so instead of presenting a
guess as fact. The [mapping table](#mapping-table) is checked against the built adapters by a test
(`registry::reasoning_page_tests`), so a row cannot say a model accepts an effort that its adapter
refuses.

The short version:

- One setting, `ReasoningEffort`, means the same thing on every provider: `low` is `low`, never
  "whatever the vendor calls its second step".
- It is set per request, per agent or per provider. Left unset at every level, **nothing is sent
  and every request is byte-for-byte what it was before this setting existed**. There is no
  compiled-in effort for any provider.
- A model that cannot honour the effort **refuses the request before any HTTP call** with
  `LlmError::UnsupportedParameter`. Nothing is clamped, rounded or converted between a level and a
  token budget unless you opt in to the nearest-level fallback, and even then only a level is ever
  swapped for a level.
- Reasoning tokens count against the output budget, and a reasoning model is slow. Effort, output
  budget and timeout belong in the same configuration block; see
  [Effort, output budget and timeout](#effort-output-budget-and-timeout).

## Values and spellings

`ReasoningEffort` has nine values. In YAML and JSON they are spelled:

| Value | Spelling | Meaning |
|-------|----------|---------|
| `Off` | `off` | Reasoning switched off, where the model allows it. |
| `Auto` | `auto` | Reasoning on, at the vendor's own default depth. Distinct from leaving the setting unset, which sends nothing at all. |
| `Minimal` | `minimal` | Level 0 of the six ranked levels. |
| `Low` | `low` | Level 1. |
| `Medium` | `medium` | Level 2. |
| `High` | `high` | Level 3. |
| `XHigh` | `xhigh` | Level 4 (not `x_high`). |
| `Max` | `max` | Level 5. |
| `Budget(n)` | `{budget: 2048}` | A reasoning-token budget. Only some wires carry one. |

The six levels run `minimal < low < medium < high < xhigh < max`. `Off`, `Auto` and `Budget` are not
levels: they are never ranked and never substituted for one another or for a level. In metadata and
log text the values print the same way (`xhigh`, `budget:2048`).

## Where to set it

Precedence, most specific first: the **request**, then the **agent**, then the **provider**, then
the vendor's own default (nothing sent). Each level is resolved field by field, so a request can
set only the effort and inherit the agent's fallback.

**1. On a request.** `LlmRequest::with_reasoning` (and `with_reasoning_fallback`). Note the name:
`LlmResponse::with_reasoning` records the reasoning *text* a vendor returned and
`TokenUsage::with_reasoning` records a reasoning-token *count*; only `LlmRequest::with_reasoning`
asks for an effort.

```rust,ignore
use paladin_ports::output::llm_port::{LlmRequest, ReasoningEffort};

let request = LlmRequest::new("deepseek-v4-pro", prompt).with_reasoning(ReasoningEffort::High);
let response = llm.generate(request).await?;
```

**2. On an agent.** `PaladinBuilder::reasoning_effort` (and `reasoning_fallback`) in code, or
`agents[].reasoning_effort` (and `agents[].reasoning_fallback`) in YAML. The agent's value rides on
every request the agent's execution makes, unless that request sets its own.

```rust,ignore
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::core::platform::container::reasoning::ReasoningEffort;

let paladin = PaladinBuilder::new(llm_port)
    .system_prompt("You are a careful reviewer")
    .model("deepseek-v4-pro")
    .reasoning_effort(ReasoningEffort::High)
    .build()
    .await?;
```

```yaml
agents:
  - id: "reviewer"
    provider: "deepseek"
    model: "deepseek-v4-pro"
    system_prompt: "You are a careful reviewer."
    reasoning_effort: high
```

**3. On a provider.** `llm.<provider>.reasoning_effort` (and `llm.<provider>.reasoning_fallback`) in
YAML, or `with_reasoning_effort` / `with_reasoning_fallback` on any of the nine `*Config` types
(`DeepSeekConfig::with_reasoning_effort(ReasoningEffort::High)`). It applies to every request of
that provider that sets no effort of its own, and it is judged against **the request's own model**
each time (see [A provider-wide default](#a-provider-wide-default)).

```yaml
llm:
  deepseek:
    api_key: "${DEEPSEEK_API_KEY}"
    default_model: "deepseek-v4-pro"
    reasoning_effort: high
```

**4. The vendor's default.** With all three unset, nothing is sent. The vendor's own default
applies: for example, Kimi's `kimi-k3` defaults to `max`, which is why it is the model that most
needs this setting (see [the Kimi example](#a-working-kimi-configuration)).

The reasoning keys are block-only: there is no environment variable for them and no per-model
default. Only a provider-wide default exists.

## When the model cannot honour it

Every adapter asks one rule, `EffortSupport::resolve`, before it builds a request: does this
model, on this wire, take exactly this effort? If it does, the effort is sent as the vendor
spells it. If it does not, the call fails **before HTTP** with `LlmError::UnsupportedParameter`,
classified `Permanent` (so it is never retried and never fails over to a chain's next hop), with a
reason that says how to fix it:

```text
Unsupported parameter 'reasoning_effort' = 'medium' for provider 'deepseek' (model
'deepseek-v4-pro'): the vendor accepts that effort level but silently runs it as another; this
model accepts: ...
```

The cases that refuse, whatever the fallback:

- a level the model does not list, or one the vendor **silently coerces** (a request for `medium`
  that quietly ran `high` would change cost and latency without telling anyone);
- `Off` on a model that cannot run with reasoning off, `Auto` on a model with no vendor default
  depth to request;
- any `Budget` on a wire that carries no budget, or outside the span the model accepts, or at or
  above the request's `max_tokens` (the budget is part of the output cap and must leave room to
  answer);
- any explicit effort on a model nobody measured (see
  [Unmeasured models and the override](#unmeasured-models-and-the-override));
- any explicit effort on a provider whose wire is unverified (the mapping table says
  `unverified:` with the reason).

Nothing is converted between a level and a budget, in either direction.

## `reasoning_fallback`: the opt-in nearest level

`reasoning_fallback` is a second setting beside the effort, with the same three places (request,
agent, provider) and the same precedence. It has three values and defaults to `refuse`:

| Value | YAML | Behaviour |
|-------|------|-----------|
| `Refuse` | `refuse` | The default. A level the model lacks is refused. |
| `NearestLower` | `nearest_lower` | Cost- and latency-safe: use the nearest **lower** level the model lists. |
| `NearestHigher` | `nearest_higher` | Quality-safe: use the nearest **higher** level the model lists. |

"Nearest" is measured along `minimal < low < medium < high < xhigh < max` against the model's
measured levels. If no level exists in the chosen direction the request is still refused; the
direction is never flipped. A substitution logs a `warn!` and records both the requested and the
effective effort in the response metadata (see [Observability](#observability)).

```yaml
agents:
  - id: "summariser"
    provider: "gemini"
    model: "gemini-3.5-flash"
    system_prompt: "You summarise."
    reasoning_effort: xhigh
    reasoning_fallback: nearest_lower   # gemini-3.5-flash lists minimal..high, so xhigh runs as high
```

What the fallback **never** substitutes, with the opt-in set: `Off`, `Auto`, any `Budget`, a level the
vendor is known to coerce silently, and any model with no measured levels. Those still refuse.

## Unmeasured models and the override

Paladin writes an effort fact for a model only where a dated live probe measured it. A model with
no measured effort data (`unmeasured` in the table, or any model id the registry has never seen)
refuses every explicit effort rather than receive one by guess. The error names the key that
unlocks it:

```text
Unsupported parameter 'reasoning_effort' = 'high' for provider 'openai' (model 'gpt-4o-mini'):
no measured reasoning-effort data for this model; declare the model's efforts with the
`llm.openai.models.gpt-4o-mini.effort` override (see the Reasoning effort guide)
```

Declare what you know about the model in the `llm.<provider>.models.<model-id>.effort` override:

```yaml
llm:
  openai:
    models:
      my-new-model:            # a model id Paladin has not measured
        effort:
          supported:
            levels: [low, medium, high]    # strictly ascending, levels only
            off: false
            auto: true
            budget: {min: 1024, max: 32000}   # only where the wire carries a budget
```

`effort` also accepts the single words `unmeasured`, `wire_unsupported` and `no_reasoning` (a model
that does not reason: `off` becomes an exact no-op that sends nothing and every other effort is
refused). A malformed override (levels out of order, a coerced level that is also listed, a budget
with `min` above `max`, an unknown key) is a load error naming the field. An override states a
fact about the *model*; it cannot unlock what the adapter's *wire* cannot carry, so a `budget` on
a wire with no budget field stays refused.

The override replaces the seeded effort data for that model only; the probe date in the table is
the seed's own.

## Fallback chains

A `FallbackLlmAdapter` (the Aegis model fallback) checks the request's effort against **every
hop's own model** before the first HTTP call. If any hop can neither honour the effort nor
substitute under the request's fallback, the whole request refuses, and the reason reads
`fallback hop <index>: <reason>` with a zero-based index naming that hop's provider and model. A
misconfigured chain therefore fails on its first call, not during an outage of hop 1. Each hop that
is actually reached sends its own resolved effort (a substitution may differ per hop), and the
response metadata records the effort of the hop that served. A request with no effort skips the
check. The check covers the request's own effort; a hop adapter's provider-level default is resolved
by that adapter at send time and refuses there, naming `llm.<provider>.reasoning_effort`.

## Build-time checks

`PaladinBuilder::build` refuses an agent-level effort that the agent's model cannot honour, with a
`ConfigurationError` naming the provider, the model, `reasoning_effort`, the value and the reason.
The same check judges the agent against its model profile with the agent's `reasoning_fallback`
(`Refuse` when unset); a port that reports no profile refuses an explicit effort, because nothing
was measured. For a fallback chain the check uses the chain's **intersected** profile: a level is
usable only if every hop takes it. With `nearest_lower` or `nearest_higher` the build succeeds and
the substitution happens per request.

When an agent is built from YAML, the host names the line to change: an agent-level refusal ends with
``(set by `agents[<id>].reasoning_effort`)``, and a provider default the agent's model cannot honour
ends with ``(set by `llm.<provider>.reasoning_effort`)``. The adapter's own pre-HTTP check stays the
authority for per-request values.

## Self-hosted endpoints

Ollama and the generic OpenAI-compatible adapter talk to an endpoint only you can describe, so
neither ships a wire form: both declare `unverified` and refuse every explicit effort until you
supply two facts.

1. **The wire.** Where the endpoint parses a reasoning effort, with `llm.<provider>.reasoning_wire`
   (or `with_reasoning_wire` on `OllamaConfig` / `OpenAiCompatibleConfig`):

   | `reasoning_wire` | Field sent |
   |------------------|------------|
   | `top_level_effort` | top-level `reasoning_effort` (and `"none"` for `off`) |
   | `thinking_object` | `thinking: {type, effort}` |
   | `enable_thinking` | `enable_thinking` plus `thinking_budget` (no named levels) |
   | `unsupported` | the default: every explicit effort refuses |

2. **The levels.** Per model, with the `llm.<provider>.models.<model-id>.effort` override above.

Both are required before any effort is sent. The named vendors (OpenAI, Anthropic, DeepSeek,
Gemini, Kimi, Qwen, Grok) take **no** wire override: a `reasoning_wire` on one of those blocks is a
configuration error, because their wire changes only through a Paladin release after a dated
probe, so the table below stays the truth for them.

```yaml
llm:
  ollama:
    base_url: "http://localhost:11434"
    reasoning_wire: top_level_effort      # what you proved below, not a guess
    models:
      my-local-model:
        effort:
          supported:
            levels: [low, medium, high]
```

**Prove the field before you declare it.** The effort probe in the live smoke harness accepts an
`ollama` or `openai-compatible` target. It sends every candidate wire form with a deliberately
invalid value (`bogus-effort-51`) and with real levels, and prints numbers and verdicts only
(never response content):

```text
OLLAMA_BASE_URL=http://localhost:11434 \
cargo run -p paladin-llm --example live_vendor_smoke \
  --features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatible \
  -- --effort --models ollama=my-local-model
```

(`--models openai-compatible=<id>` with `OPENAI_COMPATIBLE_API_KEY` and
`OPENAI_COMPATIBLE_BASE_URL` set does the same for a gateway; `--effort --dry-run` prints the plan
with no network access and no credential read.) Two rows are your evidence:

- **The invalid-value row.** If a form's invalid value is **rejected** (HTTP 400 or 422), the
  endpoint parses that field. If it is **accepted** (2xx), the endpoint silently ignores the field:
  do not declare that wire. This is the Kimi lesson: Kimi answered 200 to an invalid top-level
  `reasoning_effort` because it ignores that field.
- **The low-versus-high effect row.** A real `high` should report more reasoning tokens than `low`.
  A field that parses but changes nothing is not worth declaring.

Paladin makes no network call when it builds an adapter, so declaring a wire verifies nothing: the
operator owns the verification, as with the capabilities preset the generic adapter already requires.

## Observability

When an effort is in force, the response records what was asked and what ran:

- `LlmResponse.metadata` carries `paladin.reasoning_requested` and `paladin.reasoning_effective`
  beside `paladin.served_by` and `paladin.served_model` (the constants are
  `REASONING_REQUESTED_METADATA_KEY` and `REASONING_EFFECTIVE_METADATA_KEY`). The values are the
  spellings above (`xhigh`, `budget:2048`). They differ only when you chose a substituting fallback.
  On a stream they ride on the terminal chunk. With no effort set, neither key is present.
- An agent run keeps the latest call's two keys in `PaladinResult.run_metadata`.
- The trace stream's `TraceEvent::NodeFinished` carries `reasoning_requested` and
  `reasoning_effective` (both absent when no effort was in force, so existing consumers see no
  change).
- `TokenUsage.reasoning_tokens` is filled whenever the vendor reports a count, and
  `LlmResponse.reasoning` holds the reasoning **text** where the vendor returns it.

## Mapping table

The cells below are read from the adapters, not typed by hand. `exact` means the effort reaches the
vendor as asked. `refused` means the request fails before any HTTP call with
`LlmError::UnsupportedParameter`. `refused (coerced)` means the vendor accepts the value but
silently runs it as another, so Paladin refuses it under every fallback. `no-op` would mean an
accepted effort that sends nothing; no seeded model uses it. `budget <min>-<max>` is the span of
reasoning-token budgets the model accepts. `Verified on` is the date of the live probe the row comes
from; `unmeasured` means the model was never probed, so every explicit effort on it is refused until
an operator declares what it takes (D-02), and `unverified` means the provider ships no wire form.

### Declared wire per provider

| Provider | Model | Wire field | Verified on |
|----------|-------|------------|-------------|
| openai | (declared wire) | `reasoning_effort` | 2026-10-22 |
| anthropic | (declared wire) | `output_config.effort` | 2026-10-22 |
| deepseek | (declared wire) | `reasoning_effort` | 2026-10-22 |
| kimi | (declared wire) | `thinking.effort` | 2026-10-22 |
| qwen | (declared wire) | `enable_thinking` + `thinking_budget` | 2026-10-22 |
| grok | (declared wire) | `reasoning_effort` | 2026-10-22 |
| gemini | (declared wire) | `thinkingLevel` | 2026-10-22 |
| ollama | (declared wire) | unverified: no default wire; the 2026-10-22 probe skipped it (no daemon configured) and only the operator knows the field the endpoint parses, so declare it with `llm.ollama.reasoning_wire` | unverified |
| openai-compatible | (declared wire) | unverified: no default wire; the 2026-10-22 probe skipped it (no endpoint configured) and only the operator knows the field the endpoint parses, so declare it with `llm.openai-compatible.reasoning_wire` | unverified |

### Effort per model

| Provider | Model | Off | Auto | Minimal | Low | Medium | High | XHigh | Max | Budget | Verified on |
|----------|-------|-----|------|---------|-----|--------|------|-------|-----|--------|-------------|
| openai | `chat-latest` | refused | refused | refused | refused | refused | refused | refused | refused | refused | unmeasured |
| openai | `gpt-4o-mini` | refused | refused | refused | refused | refused | refused | refused | refused | refused | unmeasured |
| openai | `gpt-5` | refused | exact | exact | exact | exact | exact | refused | refused | refused | 2026-10-22 |
| openai | `gpt-5-mini` | refused | refused | exact | exact | exact | exact | refused | refused | refused | 2026-10-22 |
| openai | `gpt-5-nano` | refused | exact | exact | exact | exact | exact | refused | refused | refused | 2026-10-22 |
| openai | `gpt-5.1` | exact | refused | refused | exact | exact | exact | refused | refused | refused | 2026-10-22 |
| openai | `gpt-5.2` | exact | refused | refused | exact | exact | exact | exact | refused | refused | 2026-10-22 |
| openai | `gpt-5.4` | exact | refused | refused | exact | exact | exact | exact | refused | refused | 2026-10-22 |
| openai | `gpt-5.4-mini` | exact | refused | refused | exact | exact | exact | exact | refused | refused | 2026-10-22 |
| openai | `gpt-5.4-nano` | exact | refused | refused | exact | exact | exact | exact | refused | refused | 2026-10-22 |
| openai | `gpt-5.5` | exact | exact | refused | exact | exact | exact | exact | refused | refused | 2026-10-22 |
| openai | `gpt-5.6-luna` | exact | exact | refused | exact | exact | exact | exact | refused | refused | 2026-10-22 |
| openai | `gpt-5.6-sol` | exact | refused | refused | exact | exact | exact | exact | refused | refused | 2026-10-22 |
| openai | `gpt-5.6-terra` | exact | exact | refused | exact | exact | exact | exact | refused | refused | 2026-10-22 |
| openai | `gpt-6-astra` | refused | exact | refused | exact | exact | exact | exact | refused | refused | 2026-10-22 |
| openai | `gpt-6-luna` | exact | exact | refused | exact | exact | exact | exact | refused | refused | 2026-10-22 |
| openai | `gpt-6-sol` | exact | exact | refused | exact | exact | exact | exact | refused | refused | 2026-10-22 |
| openai | `gpt-6.1-sol` | refused | exact | refused | exact | exact | exact | exact | refused | refused | 2026-10-22 |
| openai | `o3` | refused | refused | refused | exact | exact | exact | exact | refused | refused | 2026-10-22 |
| anthropic | `claude-fable-5` | refused | exact | refused | exact | exact | exact | exact | exact | refused | 2026-10-22 |
| anthropic | `claude-fable-5-1` | refused | exact | refused | exact | exact | exact | exact | exact | refused | 2026-10-22 |
| anthropic | `claude-haiku-5-5` | exact | exact | refused | exact | exact | exact | exact | exact | refused | 2026-10-22 |
| anthropic | `claude-opus-4-6` | exact | exact | refused | exact | exact | exact | refused | exact | refused | 2026-10-22 |
| anthropic | `claude-opus-4-7` | exact | refused | refused | exact | exact | exact | exact | exact | refused | 2026-10-22 |
| anthropic | `claude-opus-4-8` | exact | refused | refused | exact | exact | exact | exact | exact | refused | 2026-10-22 |
| anthropic | `claude-opus-5` | exact | exact | refused | exact | exact | exact | exact | exact | refused | 2026-10-22 |
| anthropic | `claude-opus-5-5` | refused | exact | refused | exact | exact | exact | exact | exact | refused | 2026-10-22 |
| anthropic | `claude-sonnet-4-6` | exact | exact | refused | exact | exact | exact | refused | exact | refused | 2026-10-22 |
| anthropic | `claude-sonnet-5` | exact | refused | refused | exact | exact | exact | exact | exact | refused | 2026-10-22 |
| anthropic | `claude-sonnet-5-5` | refused | refused | refused | exact | exact | exact | exact | exact | refused | 2026-10-22 |
| deepseek | `deepseek-flash` | exact | exact | refused (coerced) | exact | refused (coerced) | exact | refused (coerced) | exact | refused | 2026-10-22 |
| deepseek | `deepseek-v4-pro` | exact | exact | refused (coerced) | exact | refused (coerced) | exact | refused (coerced) | exact | refused | 2026-10-22 |
| kimi | `kimi-k2.6` | exact | exact | refused | exact | refused | exact | refused | exact | refused | 2026-10-22 |
| kimi | `kimi-k3` | exact | exact | refused | exact | refused | exact | refused | exact | refused | 2026-10-22 |
| qwen | `qwen-flash` | exact | exact | refused | refused | refused | refused | refused | refused | budget 1-32768 | 2026-10-22 |
| qwen | `qwen-max` | exact | refused | refused | refused | refused | refused | refused | refused | refused | 2026-10-22 |
| qwen | `qwen-plus` | exact | exact | refused | refused | refused | refused | refused | refused | budget 1-32768 | 2026-10-22 |
| qwen | `qwen-plus-latest` | exact | refused | refused | refused | refused | refused | refused | refused | refused | 2026-10-22 |
| qwen | `qwen3.5-flash` | exact | exact | refused | refused | refused | refused | refused | refused | budget 1-32768 | 2026-10-22 |
| qwen | `qwen3.5-plus` | exact | exact | refused | refused | refused | refused | refused | refused | budget 1-32768 | 2026-10-22 |
| qwen | `qwen3.6-flash` | exact | exact | refused | refused | refused | refused | refused | refused | budget 1-32768 | 2026-10-22 |
| qwen | `qwen3.6-plus` | exact | exact | refused | refused | refused | refused | refused | refused | budget 1-32768 | 2026-10-22 |
| qwen | `qwen3.7-flash` | exact | exact | refused | refused | refused | refused | refused | refused | budget 1-32768 | 2026-10-22 |
| qwen | `qwen3.7-max` | exact | exact | refused | refused | refused | refused | refused | refused | budget 1-32768 | 2026-10-22 |
| qwen | `qwen3.7-plus` | exact | exact | refused | refused | refused | refused | refused | refused | budget 1-32768 | 2026-10-22 |
| qwen | `qwen3.8-flash` | exact | exact | refused | refused | refused | refused | refused | refused | budget 1-32768 | 2026-10-22 |
| qwen | `qwen3.8-max` | exact | exact | refused | refused | refused | refused | refused | refused | budget 1-32768 | 2026-10-22 |
| grok | `grok-4.20-non-reasoning` | refused | refused | refused | refused | refused | refused | refused | refused | refused | unmeasured |
| grok | `grok-4.20-reasoning` | refused | exact | refused | refused | refused | refused | refused | refused | refused | 2026-10-22 |
| grok | `grok-4.3` | refused | exact | exact | exact | exact | exact | exact | refused | refused | 2026-10-22 |
| grok | `grok-4.5` | refused | exact | exact | exact | exact | exact | refused (coerced) | refused | refused | 2026-10-22 |
| grok | `grok-4.6` | refused | exact | exact | exact | exact | exact | exact | refused | refused | 2026-10-22 |
| grok | `grok-4.7` | refused | exact | exact | exact | exact | exact | exact | refused | refused | 2026-10-22 |
| gemini | `gemini-3-flash-preview` | exact | exact | exact | exact | exact | exact | refused | refused | refused | 2026-10-22 |
| gemini | `gemini-3.1-flash-lite` | exact | refused | exact | exact | exact | exact | refused | refused | refused | 2026-10-22 |
| gemini | `gemini-3.1-pro-preview` | refused | exact | refused | exact | exact | exact | refused | refused | refused | 2026-10-22 |
| gemini | `gemini-3.5-flash` | exact | exact | exact | exact | exact | exact | refused | refused | refused | 2026-10-22 |
| gemini | `gemini-3.5-flash-lite` | refused | refused | exact | exact | exact | exact | refused | refused | refused | 2026-10-22 |
| gemini | `gemini-3.6-flash` | exact | exact | exact | exact | exact | exact | refused | refused | refused | 2026-10-22 |
| gemini | `gemini-3.7-flash` | refused | exact | refused | exact | exact | exact | refused | refused | refused | 2026-10-22 |
| gemini | `gemini-3.8-flash` | refused | exact | refused | exact | exact | exact | refused | refused | refused | 2026-10-22 |
| gemini | `gemini-4-argon` | exact | exact | refused | exact | exact | exact | refused | refused | refused | 2026-10-22 |
| gemini | `gemini-flash-latest` | refused | exact | refused | exact | exact | exact | refused | refused | refused | 2026-10-22 |
| gemini | `gemini-flash-lite-latest` | refused | refused | exact | exact | exact | exact | refused | refused | refused | 2026-10-22 |
| gemini | `gemini-pro-latest` | refused | exact | refused | exact | exact | exact | refused | refused | refused | 2026-10-22 |

How to read the table, vendor by vendor:

- **OpenAI.** Levels go in the top-level `reasoning_effort`; `off` is the vendor's `"none"`, accepted only
  where the probe's `none` row was accepted; `auto` sends nothing (the vendor's default depth) and
  is listed only where the default call reported reasoning tokens. `gpt-5`, `gpt-5-mini` and
  `gpt-5-nano` take `minimal` but not `xhigh`; `gpt-5.1` takes `low` to `high`. `gpt-4o-mini` and
  `chat-latest` were never probed and refuse every explicit effort.
- **Anthropic.** Levels go in `output_config.effort`. On `claude-opus-4-8`, `claude-opus-4-7`,
  `claude-opus-4-6` and `claude-sonnet-4-6`, whose thinking is off by default and where the probe
  measured no thinking tokens from `output_config` alone, the adapter sends `thinking: {type:
  adaptive}` beside the level so the level has something to adjust. `off` is `thinking: disabled`
  and `auto` is `thinking: adaptive`; each is listed only where the probe accepted it (the always-on
  models answered 400 to `disabled`). The 4.6 models do not list `xhigh` (400). No native model
  accepts `Budget`: the vendor documents a 1,024 minimum and "less than `max_tokens`" but no fixed
  maximum, and Paladin seeds a bound only from a documented maximum. The legacy `budget_tokens`
  form is never sent to a model that rejects it.
- **DeepSeek.** Levels go in `reasoning_effort`; `off` is `thinking: disabled` and `auto` is
  `thinking: enabled`. DeepSeek documents that `minimal` runs as `low` and `medium` and `xhigh` as
  `high`, so those three are `refused (coerced)`.
- **Kimi.** Levels go in `thinking.effort` (`thinking: {type: enabled, effort}`); `off` is `thinking:
  {type: disabled}` and `auto` is `thinking: {type: enabled}`. The vendor rejected `minimal`,
  `medium` and `xhigh` (400), so they refuse. Kimi's top-level `reasoning_effort` is **never sent**:
  the probe sent it an invalid value and Kimi answered 200, so the field is silently ignored.
- **Qwen.** The wire is `enable_thinking` plus `thinking_budget`. There are **no named levels**:
  every level refuses on every Qwen model, whatever the fallback, and a level is never converted
  to a budget. `off`, `auto` and a budget span of 1 to 32,768 are listed per model; `qwen-max` and
  `qwen-plus-latest` take `off` only. The 32,768 ceiling is the lowest of the figures the vendor
  documents and its rejections name; a budget between that and the larger observed bounds refuses.
- **Grok.** Levels go in the top-level `reasoning_effort`. Grok models cannot turn reasoning off, so
  `off` refuses everywhere. xAI treats `xhigh` as `high` on `grok-4.5`, so it is `refused (coerced)`
  there; `grok-4.20-reasoning` rejected every level and takes `auto` only. `grok-4.20-non-reasoning`
  was never probed.
- **Gemini.** Levels go in `generationConfig.thinkingConfig.thinkingLevel`, spelled in lower case
  (`low`, not `LOW`; the probe accepted both). `off` is `thinkingBudget: 0` and `auto` sends
  nothing. `minimal` is listed per model (six of twelve probed models accepted it). No native `Budget`
  is seeded: the vendor documents no maximum for the models probed.
- **Ollama and OpenAI-compatible.** No wire, so no model row can be `exact`: see
  [Self-hosted endpoints](#self-hosted-endpoints).

A model id with no row (a model released after the probe, or one never probed such as
`kimi-k2.7-code` or `qwen-turbo`) is `unmeasured` too: it refuses an explicit effort until you
declare it with the override.

## Effort, output budget and timeout

Reasoning tokens are **output tokens**: they count against `max_tokens`, they are billed, and a
reasoning model spends them before it writes the first word of the answer. Three settings therefore
have to be chosen together:

- **Effort** sets how many reasoning tokens the model is invited to spend.
- **Output budget** (`max_tokens`, `llm.<provider>.max_tokens`) must be large enough for the reasoning
  *and* the answer. If the model uses the whole cap thinking, the call finishes `length` with no
  text and Paladin returns `LlmError::EmptyCompletion`. That error is a runtime **backstop**, not a
  validation: Paladin cannot know in advance how many tokens a question needs, so it cannot refuse
  a cap that is too small. A `Budget(n)` effort is the one case it can check, and it refuses a
  budget at or above `max_tokens`.
- **Timeout** (`timeout_seconds`) must outlast the slowest thing the effort can do. A seeded
  reasoning model defaults to **300 s**; an unseeded model keeps its provider's shorter default
  (60 s for Kimi).

### What the measurements show

The motivating measurement was taken on `kimi-k3` on 2026-10-13: the same short mathematics
question ("how many n in 1..1000 make n²+n+41 prime"), `max_tokens: 16000`, non-streaming, the
operator's key:

| `thinking` sent | finish | wall time | reasoning tokens | answer |
|-----------------|--------|-----------|------------------|--------|
| `{type: enabled, effort: low}` | stop | 56 s | 1,906 | yes |
| `{type: enabled, effort: high}` | stop | 359 s | 13,164 | yes |
| `{type: enabled, effort: max}` | **length** | 473 s | 15,997 | **empty** |
| no `thinking` field (the vendor default) | **length** | 492 s | 15,997 | **empty** |
| `{type: disabled}` | stop | 2 s | none reported | yes |
| `{type: enabled, effort: low}`, `max_tokens: 64` | **length** | 5 s | 61 | **empty** |

What this shows:

- **An unconfigured `kimi-k3` request is a `max`-effort request.** On a one-line question `max` spent
  the whole 16,000-token cap and returned no answer after about eight minutes.
- **Effort is a real cost and latency lever.** `low` to `high` was about seven times the reasoning
  tokens and about six times the wall time.
- **`high` took 359 s, longer than the 300 s default**, so `high` on a hard question needs a
  timeout above that.
- **A cap that is too small fails quietly**: `low` with `max_tokens: 64` spent all 64 tokens
  thinking.
- **Thinking can be turned off** on `kimi-k3` (2 s), although the model's listing says it only
  thinks. Trust a probe over a listing.

The 2026-10-22 effort probe measured the same model through `thinking.effort` on a one-line
question (17 multiplied by 23) at a 2,048-token cap; every call finished `stop`:

| Request | Result | Reasoning tokens |
|---------|--------|------------------|
| no effort | accepted | 33 |
| `low` | accepted | 40 |
| `high` | accepted | 48 |
| `max` | accepted | 116 |
| `minimal`, `medium`, `xhigh` | rejected, HTTP 400 | n/a |
| `off` | accepted | none reported |
| `auto` | accepted | 71 |
| invalid value | rejected, HTTP 400 | n/a |

These are single calls, not averages. They show which values the vendor takes and that a higher
level reports more reasoning, not what any workload will cost.

### A working Kimi configuration

`kimi-k3` answered a mathematics question at `low` with a 16,000-token cap in 56 s, well inside a
300 s timeout. The block below restates exactly those measured values as **configuration**. None of
it is a compiled default: with `reasoning_effort` unset Paladin sends nothing and `kimi-k3` runs at
its own default, `max`.

```yaml
llm:
  default_provider: "kimi"

  kimi:
    api_key: "${MOONSHOT_API_KEY}"
    default_model: "kimi-k3"
    reasoning_effort: low     # measured: stop in 56 s, 1,906 reasoning tokens (2026-10-13)
    max_tokens: 16000         # the cap that measurement used
    timeout_seconds: 300      # restates the reasoning-model default so it is visible
```

To use `high`, raise `timeout_seconds` above the 359 s measured; to use `max`, expect to spend the
whole cap on a hard question. The seeded `kimi-k3` row lists `low`, `high`, `max`, `off` and `auto`;
a `reasoning_effort` it does not list (`medium`, say) fails when the agent is built, naming
`llm.kimi.reasoning_effort`, instead of at the first call.

## Notes and limits

### A provider-wide default

`llm.<provider>.reasoning_effort` applies to **every** model that provider serves, and each request
is judged against its own model. `llm.openai.reasoning_effort: high` therefore reaches
`gpt-4o-mini` too, which was never measured, and refuses every request for it (the reason ends with
``(set by `llm.openai.reasoning_effort`)``). Set a provider-wide default only where every model you
serve supports it; otherwise set the effort on the agent or the request. A per-model default is not
offered.

### The node cache

The Aegis node cache key includes an agent's `reasoning_effort` and `reasoning_fallback` when they
are set, so an A/B run at `low` and then `high` is not served from the cache. An unset effort adds
nothing, so existing keys are unchanged. A **provider-level** default is not part of the key: after
changing `llm.<provider>.reasoning_effort`, start with a fresh node-cache namespace (change the
`key_prefix`, see [Aegis](fault-tolerance.md)) or clear the cache.

### Reasoning text is not requested

Asking for an effort does not ask the vendor to show its reasoning. Paladin fills
`LlmResponse.reasoning` only when the vendor sends reasoning text unprompted: OpenAI's Chat
Completions never does, Anthropic's thinking text defaults to omitted on current models, and
Gemini returns thought parts only with `includeThoughts`, which the adapter does not send.

### Sampling parameters can change meaning with effort

The probe sent `high` together with `temperature: 0.5` and the vendor refused (400) on all 17
OpenAI models, on 9 of the 11 Anthropic models, on both Kimi models and on `grok-4.20-reasoning`;
it accepted it on both DeepSeek models, all 12 Gemini models and the other four Grok models.
Paladin does not add a rule for this: a model's sampling profile, not its effort, decides what
sampling parameters are sent. If you set an effort on one of those models, leave `temperature` out.

### Not covered

- The OpenAI **Responses API** is not used; the mapping is Chat Completions `reasoning_effort` only.
- The `paladin` command-line tool's own agent YAML (`src/application/cli/config/paladin_config.rs`)
  has no effort key, and agents provisioned through the facade's `AgentSpec` carry none.
- Levels and bounds are not discovered from a vendor's model listing; they come from the dated
  probe or from your override.

See also: [Switching Providers](switching-providers.md) for the sampling and limits profile each
model carries, [Provider Expansion](../appendix/provider-expansion.md), and
[Paladin Configuration](paladin-configuration.md).
