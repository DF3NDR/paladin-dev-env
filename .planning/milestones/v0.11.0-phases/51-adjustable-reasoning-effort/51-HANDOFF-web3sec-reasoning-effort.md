# Handoff: Adjustable reasoning effort

**From:** Web3 Security Paladin ("web3sec"), the downstream consumer of `DF3NDR/paladin-dev-env`
**To:** the Paladin-dedicated agent, working in a standalone `paladin-dev-env` checkout
**Date:** 2026-10-13
**Code basis:** Paladin `origin/main` @ `df24689c` (2026-09-23, post-`v0.10.1`). Every `path:line` below was read at that commit. Re-verify against your current `main` before planning.
**Depends on:** `../Provider-Parity-Config-Driven-Model-Swap/PALADIN-HANDOFF.md`. Plan that phase first, or at least its Plans 2 and 4 (§9).
**Purpose:** give you enough context to open a phase in Paladin's GSD roadmap that adds a provider-neutral, per-agent **reasoning-effort** setting, **without access to web3sec's planning files**.

---

## 1. TL;DR

- **Paladin has no way to control how hard a model reasons.**
  - The request path carries no effort, thinking or budget parameter for any of the nine adapters.
  - Reasoning is handled only on the response side: reasoning-token counts, empty-output detection, and parsed-then-discarded reasoning text.
  - The model's vendor default therefore always applies. For Moonshot `kimi-k3` that default is **`max`**. Measured: on a one-line question it spent the whole 16,000-token output budget on reasoning in about 8 minutes and **returned no answer**. `low` answered in 56 s, and `disabled` in 2 s (§4.1).
- **Goal.** One provider-neutral setting, settable per agent (and per request), that each adapter translates exactly to its vendor's native form. Where a translation is impossible, the adapter **refuses loudly before the HTTP call**.
- **web3sec wants per-role settings** for its deductive, fuzz-author, Tribunal-judge and PoC-Prover agents, for cost/latency control and A/B benchmarking.
- **Central hazard: silent no-ops.** Several vendors accept a wrong or unsupported effort field with HTTP 200 and ignore or coerce it (§5). Measured live: `kimi-k3` silently ignores an OpenAI-style top-level `reasoning_effort`; only `thinking.effort` works. The design must be built around this.
- **Recommended shape:**
  - A `#[non_exhaustive]` `ReasoningEffort` enum on **`LlmRequest`**. `LlmRequest` was made `#[non_exhaustive]` with builders in v0.10.0 precisely so its next field is free (D-28).
  - A per-Paladin default.
  - A per-preset wire declaration in the compat engine that the compiler forces every preset author to state.
  - Model-aware mapping in the native adapters.
  - A new additive error variant for unsupported values.
  - A live conformance check per vendor proving the effort field is actually parsed.
- About five plans, new requirement prefix **`REASON-*`** (unused).

---

## 2. Background

**web3sec's runs today.** Roles use `claude-opus-4-8` and DeepSeek `deepseek-v4-pro`/`-flash`, all at vendor-default reasoning depth.

**The trigger.** The operator wants to run roles on `kimi-k3`, which is thinking-only and defaults to `max` effort. They also want to tune effort per role, so a cheap triage-style role and a deep proving role don't pay the same reasoning bill.

**Why effort and output budget must be designed together.** Every vendor surveyed counts reasoning against the output budget (`max_tokens` or equivalent), or bills it as output. web3sec has already lost runs to reasoning models spending their whole budget on reasoning and returning empty text. That is why Paladin has `LlmError::EmptyCompletion` with its reasoning hint (`crates/paladin-ports/src/output/llm_port.rs:413-439`). Raising effort without raising the output budget turns into empty completions.

**How web3sec consumes Paladin.** It builds `AnthropicAdapter`/`DeepSeekAdapter` directly and calls `LlmPort::generate` with requests it constructs itself. It needs the setting reachable at **request level**, not only through `PaladinBuilder` or the YAML agent host.

**Binding policy** (from `.project/Milestone_13-Token-Economy/overview/` §5):
- Clean break, no shims (pre-1.0, one coordinated consumer).
- Units-plain / roles-medieval vocabulary: the enum and config fields take plain names.
- Do not rewrite `.planning/` history.

---

## 3. Current state in Paladin (`df24689c`)

### 3.1 Nothing request-side

No request struct serializes any reasoning field. A grep for `reasoning_effort|budget_tokens|thinkingConfig|thinkingBudget|enable_thinking|"think"|output_config|\beffort\b` over `crates/`, `src/` and `tests/` finds no request field. The request structs are:

| Adapter | Request struct |
|---|---|
| Anthropic | `ClaudeRequest`, `crates/paladin-llm/src/anthropic/adapter.rs:636-648` |
| DeepSeek | `DeepSeekRequest`, `deepseek/adapter.rs:120-148` |
| OpenAI | `OpenAIRequest`, `openai/adapter.rs:114-135` |
| Compat (Kimi, Qwen, Grok, Ollama, OpenAI-compatible) | `CompatRequest`, `compat/types.rs:13-38` |
| Gemini | `GeminiRequest` / `GeminiGenerationConfig`, `gemini/adapter.rs:969-1020` |

### 3.2 Response-side handling that already exists (keep it, and build on it)

- **`TokenUsage.reasoning_tokens`** (`token_usage.rs:39-44`, `with_reasoning` `:114`) is populated by:
  - compat `engine.rs:539-544`
  - DeepSeek `:299-303`
  - OpenAI `:459-463`
  - Anthropic `thinking_tokens` `:748-753`
  - Gemini `thoughtsTokenCount` `:1078,1099-1100`
- **`reasoning_content` is parsed and discarded.** It is parsed in compat (`types.rs:76-80`) and DeepSeek (`:178-184`) but never reaches callers. The streaming delta has only `content` (`types.rs:163-166`).
- **Anthropic thinking blocks are skipped.** `detect_no_text_content` (`anthropic/adapter.rs:858-890`) produces `EmptyCompletion` with a census of block types. The captured fixtures show current Claude models thinking **with no thinking parameter sent** (`:1023,1039`).
- **`detect_empty_completion`** (compat `engine.rs:627-640`; DeepSeek `:263-287`) fires only on `finish_reason == length` with empty content. The parity handoff, G-2, proposes widening it.
- The Phase 26 "one-line reasoning agent" (`src/presets/mod.rs:288`) is a **tool-loop preset**. It is unrelated to effort. Its `ReasoningAgentOptions` (`:62-99`, private fields, `Default`, builder setters) is a good pattern for option types.

### 3.3 Type constraints that decide where the setting goes

| Type | Facts | Verdict |
|---|---|---|
| `PromptParameters` (`crates/paladin-core/src/platform/container/prompt.rs:42-50`) | Not `#[non_exhaustive]`. Hand-written `PartialEq/Eq/Ord/Hash` list every field (`:52-145`), so a forgotten field is silently excluded. `Default` invents values (`:147-157`). Persisted inside `PromptData`. **12 struct literals** would break. | **Do not use** |
| `ProviderCapabilities` (`llm_port.rs:1169-1213`) | 92 literal sites. Adding a field is **deliberately blocked** by the compile-time guard `provider_capabilities_gained_no_field` (`llm_port.rs:1778-1793`; D-28, `.planning/milestones/v0.10.0-phases/26-agent-runtime-enhancements/26-CONTEXT.md:433-455`). | **Do not use.** Per-model supported levels belong in the parity phase's `ModelProfile` |
| `LlmRequest` (`llm_port.rs:644-664`) | `#[non_exhaustive]`, built via `LlmRequest::new` + `with_*` builders (`:695-792`). Its rustdoc (`:598-606`) says the constructor "keeps the *next* field free". `response_format` is the precedent: an additive `#[serde(default)]` field plus `with_response_format`, threaded from the run context at `paladin_execution_service.rs:2502-2508`. | **Use this** |
| `PaladinData` (`crates/paladin-core/src/platform/container/paladin.rs:141-200`) | Per-agent knobs are added as `#[serde(default)]` fields. | Per-agent default lives here |
| `CompatRequestParameters` (`compat/engine.rs:108-151`) | Five `bool`s with **no `Default` and no struct-update spread**, so "a new preset must be a compile error until its author states a position for every field" (`:100-107`; restated at `kimi/adapter.rs:237-244`). | Extend with a **non-bool** reasoning-wire field |

### 3.4 Unsupported parameters today: dropped with a `debug!`

`build_request` (`compat/engine.rs:452-524`) filters each parameter by the preset's allow-list (`:457-463`). The dropped names are reported **only via `log::debug!`** (`:482-488`). ADR-0004 (`.planning/decisions/0004-temperature-validation.md:89-94`) rejects undiscoverable substitution. The engine reads that as "omitting a key is acceptable, substituting a value is not" (`engine.rs:111-127`).

**For reasoning effort, omission is not harmless.** It silently changes cost, latency and answer quality. **Do not reuse the omit-plus-`debug!` path for this setting** (§6.4).

---

## 4. Vendor survey

**Kimi rows are measured live on the operator's key. The other rows are desk research from the official documentation pages cited, fetched 2026-10-13.** Re-verify each row against its source and a live probe before encoding it into model profiles. Vendors change these fields often.

| Vendor | Wire field and shape | Levels | Default | Counts against output budget? | Silent behaviour / rejections | Source |
|---|---|---|---|---|---|---|
| **Moonshot Kimi `kimi-k3`** | `thinking: {"type":"enabled","effort":…}` | `low`, `high`, `max` | `max` (`/v1/models`) | yes: `completion_tokens_details.reasoning_tokens` | **Top-level `reasoning_effort` is silently ignored (HTTP 200).** An invalid `thinking.effort` returns HTTP 400. `thinking:{type:"disabled"}` **works** (measured), even though the model lists `supports_thinking_type: "only"` | live, 2026-10-07 / 2026-10-13 |
| **Anthropic** | `output_config: {effort}`; `thinking: {type: "adaptive"\|"enabled"\|"disabled", budget_tokens}` | `low`, `medium`, `high`, `xhigh`, `max` (model-gated) | `high` on most models; `medium` on some | yes; `budget_tokens` ≥ 1024 and < `max_tokens` | `thinking.enabled` + `budget_tokens` deprecated on 4.6 and **400 on 4.7+**; `type:"disabled"` rejected on some current models / effort levels | [1] [2] |
| **OpenAI** | Chat: top-level `reasoning_effort`; Responses: `reasoning: {effort}` | `none`, `minimal`, `low`, `medium`, `high`, `xhigh`, `max` (model-gated) | `medium` | yes (`output_tokens_details.reasoning_tokens`) | Some models return 400 on `none` / `minimal` | [3] |
| **DeepSeek** | `thinking: {type: "enabled"\|"disabled"}` + `reasoning_effort` | `low`, `high`, `max` | thinking on, `high` | unconfirmed | **Silently coerces** minimal→low, medium→high, xhigh→high. Sampling params "will not trigger an error but … no effect" | [4] [5] |
| **xAI Grok** | Responses `reasoning: {effort}`; chat-completions field **unconfirmed** | `low`, `medium`, `high`, `xhigh` (4.6+) | `high`; cannot be disabled | yes | **`xhigh` silently treated as `high` on grok-4.5** | [6] [7] |
| **Google Gemini** | Native `generationConfig.thinkingConfig.thinkingLevel` (3.x) / `thinkingBudget` (2.5); OpenAI-compat `reasoning_effort` | per model | on | yes ("including thought tokens") | Can't disable on 2.5 Pro / 3.x; compat endpoint silently maps `minimal`→`low` on 3.1 Pro; don't mix `reasoning_effort` with `thinking_config` | [8] [9] |
| **Qwen / DashScope** | `enable_thinking: bool`, `thinking_budget: int` | toggle + budget 1–32768 | model-dependent | billed as output | Thinking-only models can't be disabled; some models are **stream-only** with thinking on | [10] |
| **Ollama** | Native `think: bool\|"low"\|"medium"\|"high"`; `/v1`: `reasoning_effort` | model-defined (`/api/show`) | gpt-oss: `medium` | unconfirmed | **Paladin's Ollama adapter uses `/v1`**, so `reasoning_effort` applies, not `think` | [11] [12] |

Sources:
1. https://platform.claude.com/docs/en/build-with-claude/effort
2. https://platform.claude.com/docs/en/build-with-claude/extended-thinking
3. https://developers.openai.com/api/docs/guides/reasoning
4. https://api-docs.deepseek.com/guides/thinking_mode
5. https://api-docs.deepseek.com/quick_start/pricing
6. https://docs.x.ai/docs/guides/reasoning
7. https://docs.x.ai/docs/api-reference
8. https://ai.google.dev/gemini-api/docs/thinking
9. https://ai.google.dev/gemini-api/docs/openai
10. https://www.alibabacloud.com/help/en/model-studio/deep-thinking
11. https://docs.ollama.com/capabilities/thinking
12. https://docs.ollama.com/api/openai-compatibility

### 4.1 Measured cost of effort on `kimi-k3` (2026-10-13)

The same short math question ("how many n in 1..1000 make n²+n+41 prime"), `max_tokens: 16000`, non-streaming:

| `thinking` sent | finish | wall time | reasoning tokens | completion tokens | answer |
|---|---|---|---|---|---|
| `{type:enabled, effort:low}` | stop | 56 s | 1,906 | 1,922 | 581 |
| `{type:enabled, effort:high}` | stop | 359 s | 13,164 | 13,180 | 582 |
| `{type:enabled, effort:max}` | **length** | 473 s | 15,997 | 16,000 | **empty** |
| *(no `thinking` field: vendor default)* | **length** | 492 s | 15,997 | 16,000 | **empty** |
| `{type:disabled}` | stop | 2 s | — | 11 | 261 |
| `{type:enabled, effort:low}`, `max_tokens: 64` | **length** | 5 s | 61 | 64 | **empty** |

What this shows:
- **An unconfigured `kimi-k3` request is a `max`-effort request.** On a one-line question, `max` burned the entire 16,000-token output budget on reasoning in about 8 minutes and returned **no answer**. In Paladin that surfaces as `EmptyCompletion` at best (finish `length`), after eight times the adapter's **60 s default timeout** (`kimi/adapter.rs:118`). Without this setting, Kimi is close to unusable for short-answer agents.
- **Effort is a real cost and latency lever.** `low` to `high` was about 7× the reasoning tokens and about 6× the wall time.
- **`low` ran for 56 s**, already close to the 60 s timeout. Effort, output budget and timeout must be configured together (parity G-3, G-4).
- **Thinking can be disabled on `kimi-k3`** despite the model listing `supports_thinking_type: "only"`. That means `Off` is supported (2 s, no reasoning). Trust probes over metadata.
- The answers differ between levels (581 / 582 / 261). This is not a correctness benchmark, but it shows effort changes outputs, not just cost.

---

## 5. The silent-no-op hazard

At least four vendor behaviours return HTTP 200 while the requested effort is **not** what ran:
1. **Kimi** ignores a top-level `reasoning_effort` (measured).
2. **DeepSeek** coerces levels it doesn't have.
3. **xAI** coerces `xhigh`→`high` on grok-4.5.
4. **Gemini's** OpenAI-compatible endpoint coerces `minimal`→`low` on 3.1 Pro.

A tempting design, "send OpenAI-style `reasoning_effort` from the shared compat engine for every preset", would be a **silent no-op on Kimi**. Kimi is the provider that most needs the setting, because its default is `max`. Consequences for the design:

- **(a) Declare wire field names per adapter or preset, and verify them live.** Never inherit them from "OpenAI compatibility".
- **(b) Paladin validates the level against the model's supported set before sending.** Vendor 400s are not a reliable backstop.
- **(c) Add a live conformance check per vendor** (credential-gated, in `crates/paladin-llm/examples/live_vendor_smoke.rs`):
  - an **invalid** effort value must be **rejected** by the vendor, which proves the field is parsed;
  - a valid high effort must report non-zero `reasoning_tokens`.
- **(d) Add an offline mockito case per preset** in `crates/paladin-llm/src/conformance.rs` pinning the exact wire body.
- **(e) Record the effective effort** in response metadata, so a consumer can audit what actually ran.

---

## 6. Recommended design

### 6.1 Type (plain name: it is a unit, not a role)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ReasoningEffort {
    Off,          // thinking disabled
    Auto,         // thinking on at the vendor's default depth (send nothing / plain enable flag)
    Minimal, Low, Medium, High, XHigh, Max,
    Budget(u32),  // explicit reasoning-token budget, only where a vendor takes one
}
```

### 6.2 Placement

- **Per request:** `LlmRequest` gains `#[serde(default)] reasoning: Option<ReasoningEffort>` plus `with_reasoning(..)`. This follows the `response_format` precedent exactly. `None` must produce a **byte-identical** body to today.
- **Per agent:** a `#[serde(default)]` field on `PaladinData`, a `PaladinBuilder` setter, and an optional field in the YAML agent definition (`src/config/agents.rs`). `PaladinExecutionService` threads it onto every `LlmRequest` it builds (`paladin_execution_service.rs:2505,2691,3208`).
- **Per provider default:** an optional `reasoning_effort` in each `llm.<provider>` config block, through the parity phase's config-driven construction.
- **Precedence:** request, then agent, then provider config, then vendor default.

### 6.3 Adapter translation

- **Compat engine.** Extend `CompatRequestParameters` with a non-bool field, e.g. `reasoning: ReasoningWire`. Because the struct has no `Default`, every preset must state a position. Candidate variants:
  - `Unsupported`
  - `TopLevelEffort { allowed }` (OpenAI style)
  - `ThinkingObjectEffort { allowed }` (Kimi: `thinking:{type:"enabled",effort}`)
  - `ThinkingToggle` (DeepSeek style)
  - `EnableThinking { budget_range }` (Qwen)
  - `CompatReasoningEffort { allowed }` (Ollama `/v1`)
  `CompatRequest` gains the matching optional serialized fields.
- **Native adapters** (Anthropic, OpenAI, DeepSeek, Gemini) each get a private, model-aware `map_reasoning(model, effort) -> Result<WireFields, LlmError>`.
- **The supported set per model comes from the parity phase's per-model profile registry**, not from adapter-wide constants. Effort support differs between models of the same vendor.

### 6.4 Policy for values that can't be honoured (extends ADR-0004)

- **Exact or lossless mapping:** send it.
- **Anything else fails loudly before the HTTP call** with a new additive `LlmError::UnsupportedParameter { param, value, provider, model }`, classified `Permanent`. `LlmError` is `#[non_exhaustive]` (`llm_port.rs:283-285`). "Anything else" covers:
  - a level the vendor lacks
  - a level gated off for that model
  - `Off` on an always-thinking model
  - `Budget` where only levels exist
  - a level the vendor is known to coerce silently
- **Optional, opt-in nearest-level fallback** (e.g. `ReasoningFallback::Nearest`) substitutes the closest supported level, logs at `warn!`, and records both requested and effective effort in response metadata. It is off by default.
- **Validate `Budget(n)` against vendor bounds** (Anthropic: n ≥ 1024 and n < `max_tokens`; Qwen: 1–32768). Keep `EmptyCompletion` as the runtime backstop.

### 6.5 Default mapping table (✓ exact · ✗ `UnsupportedParameter` · U unverified)

| Provider | Off | Auto | Minimal | Low | Medium | High | XHigh | Max | Budget(n) |
|---|---|---|---|---|---|---|---|---|---|
| Kimi k3 | ✓ `thinking:{type:disabled}` (measured) | ✓ `thinking:{type:enabled}` | ✗ | ✓ | ✗ | ✓ | ✗ | ✓ | ✗ |
| Anthropic (4.6+) | model-gated | ✓ omit / `adaptive` | ✗ | ✓ `output_config.effort` | ✓ | ✓ | model-gated | model-gated | ✓ only where `budget_tokens` still accepted; ✗ 4.7+ |
| OpenAI | ✓ `none`, model-gated | ✓ omit | model-gated | ✓ | ✓ | ✓ | model-gated | model-gated | ✗ |
| DeepSeek | ✓ `thinking:{type:disabled}` | ✓ omit | ✗ | ✓ | ✗ | ✓ | ✗ | ✓ | ✗ |
| xAI Grok | ✗ | ✓ omit | ✗ | ✓ | ✓ | ✓ | ✓ 4.6+ / ✗ 4.5 | ✗ | ✗ (field name U) |
| Gemini native | 2.5 non-Pro only | ✓ omit | if listed | ✓ | if listed | ✓ | ✗ | ✗ | ✓ 2.5 (`thinkingBudget`); ✗ 3.x |
| Qwen | ✓ `enable_thinking:false` (✗ thinking-only) | ✓ `true` | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✓ `thinking_budget` |
| Ollama (/v1) | U | ✓ omit | ✗ | model-defined | model-defined | model-defined | ✗ | ✗ | ✗ |

### 6.6 Observability

- Write requested and effective effort into `LlmResponse.metadata`, alongside the existing `SERVED_BY_METADATA_KEY`.
- Emit both on the Phase 28 trace stream.
- `TokenUsage.reasoning_tokens` already exists; make sure every adapter fills it when the vendor reports it.
- In fallback chains, each step resolves effort against its own model (parity B-3).

---

## 7. Decisions for the operator

- **D-1. Enum shape.** Keep `Auto` distinct from `None` ("don't touch")? Include `Budget(u32)` now, or defer it?
  - **Recommendation:** keep both. `Auto` is needed for vendors where "on" must be stated explicitly (Kimi's `type:"enabled"`, Qwen's `enable_thinking`). `Budget` is the only control Qwen and Gemini 2.5 offer.
- **D-2. Placement.**
  - **Recommendation:** on `LlmRequest`, plus a per-agent default on `PaladinData`; not on `PromptParameters` (§3.3).
- **D-3. Default policy for unsupported values.**
  - **Recommendation:** fail loudly by default, with nearest-level substitution as an explicit opt-in (§6.4). Record this as an ADR amending 0004, consistent with the parity handoff's D-1.
- **D-4. Surfacing reasoning text.** Should reasoning text become a typed field on `LlmResponse` or go in `metadata`? This also closes parity G-6.
  - **Recommendation:** typed `Option<String>`, and a streaming delta field. Metadata is untyped and easy to miss.
- **D-5. Anthropic mapping strategy.** Should Anthropic use `output_config.effort` on models that support it and `thinking.budget_tokens` only on older models, chosen per model profile?
  - **Recommendation:** yes. Never send the deprecated form to a model that rejects it.

---

## 8. Recommended course of action

**Shape.** One phase, numbered after the parity phase in the same milestone (e.g. Phase 39 if parity is 38, in v0.11.0). Requirement prefix **`REASON-*`**.

**Plan 1: red tests.**
- A mockito wire-body test per adapter/preset, showing that today no effort field is sent.
- A **silent-no-op regression test** asserting that the Kimi preset never emits top-level `reasoning_effort`.
- An execution-service test asserting that a per-agent effort reaches `LlmRequest`.

**Plan 2: types and plumbing.**
- `ReasoningEffort`, `LlmRequest.reasoning` + `with_reasoning`, the `PaladinData` field, the `PaladinBuilder` setter, and the YAML agent field.
- Threading in `PaladinExecutionService` and the provider-config default, using the precedence in §6.2.
- `LlmError::UnsupportedParameter`.
- MIGRATION §9.2 rows and an API-surface refresh.

**Plan 3: compat presets.**
- `ReasoningWire` on `CompatRequestParameters`.
- Positions stated for Kimi, Qwen, Grok, Ollama and generic OpenAI-compatible. Generic should default to `Unsupported` unless configured, because a generic endpoint's field is unknowable.
- `CompatRequest` fields.
- Validation against the model profile.

**Plan 4: native adapters.**
- Model-aware mapping for Anthropic, OpenAI, DeepSeek and Gemini.
- `Budget` validation against `max_tokens`.
- Requested/effective effort recorded in response metadata.
- Typed reasoning text (D-4).

**Plan 5: live conformance, docs, ADR.**
- `live_vendor_smoke.rs`: per vendor, an invalid-effort-rejected probe, plus a check that higher effort raises `reasoning_tokens`.
- An mdBook "Reasoning effort" page with the mapping table and the effort/budget/timeout guidance.
- An ADR amending 0004.
- CHANGELOG.

**Draft requirements.**

| ID | Done when |
|---|---|
| REASON-01 | `ReasoningEffort` is settable per request, per agent and per provider config, with the documented precedence. Unset produces a byte-identical request body. |
| REASON-02 | Every adapter/preset declares its reasoning wire form (compile-enforced in compat) and maps each level exactly, or returns `UnsupportedParameter` before any HTTP call. |
| REASON-03 | No preset emits a field its vendor ignores. A live smoke proves that an invalid effort is rejected by each vendor where a key exists. |
| REASON-04 | Supported levels are per model, read from the parity phase's model-profile registry. |
| REASON-05 | Requested and effective effort, reasoning tokens and reasoning text are surfaced on the response and the trace stream. |
| REASON-06 | `Budget(n)` is validated against vendor bounds and `max_tokens`. Effort, output budget and timeout guidance is documented. |
| REASON-07 | ADR (amending 0004), mdBook page, MIGRATION and CHANGELOG are complete. Coverage floor (82%, ADR-0006) and doc tests are met. |

---

## 9. Dependency on the parity phase

Effort support is **per model** (parity Plan 4: model profiles). The policy for unsupported values extends **parity D-1**. Effort is only usable once output budget and timeout are configurable per provider/model (**parity G-3, G-4**). Effort must resolve per step in fallback chains (**parity B-3**).

If the operator wants effort sooner, Plans 1-3 of this phase can run against adapter-wide supported sets first, then migrate to model profiles. That costs a second pass over every preset, so it isn't recommended.

---

## 10. Out of scope

- Native tool calling and interleaved-thinking-with-tools semantics (ADR-0042).
- Anthropic's per-message effort beta.
- Live discovery of supported levels from vendor `/models` endpoints. Moonshot exposes them, but this is a possible follow-up to parity D-3.
- All web3sec-side changes. web3sec will add `{DEDUCTIVE,FUZZ,JUDGE,PROVER}_REASONING_EFFORT` and record effort in its benchmark history once this lands.

---

## Appendix A — Kimi live probe transcript (2026-10-13, `kimi-k3`, operator key)

| Request addition | Result |
|---|---|
| `"reasoning_effort":"bogus"` | HTTP 200, answered normally, `reasoning_tokens: 42`. **Ignored.** |
| `"thinking":{"type":"enabled","effort":"bogus"}` | HTTP 400 `{"message":"Invalid request: invalid value for thinking effort level: bogus","type":"invalid_request_error"}` |
| `"zzz_unknown":1` | HTTP 200, `reasoning_tokens: 40`. Identical to the `reasoning_effort` case. |

Model listing (`GET /v1/models`, 2026-10-07): `kimi-k3` reports `"reasoning_efforts": {"valid_efforts": ["low","high","max"], "default_effort": "max"}`, `"supports_thinking_type": "only"` and `"context_length": 1048576`.

## Appendix B — Open live-verification items

- Resolved 2026-10-13: Kimi `kimi-k3` **does** accept `thinking:{type:"disabled"}` (§4.1).
- Whether `kimi-k2.6` / `k2.7-code` take the same `thinking` object and levels.
- The xAI chat-completions effort field name, and grok-4.6 behaviour.
- Whether the Paladin DeepSeek default `deepseek-chat` still resolves (parity G-11).
- Gemini 2.5 `thinkingBudget` ranges.
- Re-check every desk-research row in §4 against its source when implementing.
