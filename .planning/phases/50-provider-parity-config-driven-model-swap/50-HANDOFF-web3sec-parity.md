# Handoff: Provider parity and configuration-driven model swap

**From:** Web3 Security Paladin ("web3sec"), the downstream consumer of `DF3NDR/paladin-dev-env`
**To:** the Paladin-dedicated agent, working in a standalone `paladin-dev-env` checkout
**Date:** 2026-10-13
**Code basis:** Paladin `origin/main` @ `df24689c` (2026-09-23, post-`v0.10.1`). Every `path:line` below was read at that commit. Re-verify against your current `main` before planning; line numbers drift.
**Companion:** `../Adjustable-Reasoning-Effort/PALADIN-HANDOFF.md`. That is a separate phase that builds on this one (§8).
**Purpose:** give you enough context to open a phase (or milestone) in Paladin's own GSD roadmap and drive it to completion **without access to web3sec's planning files**.

---

## 1. TL;DR

- **Goal.** Any Paladin agent can be moved from one LLM provider/model to another (Anthropic ⇄ OpenAI ⇄ DeepSeek ⇄ Kimi ⇄ Qwen ⇄ Grok ⇄ Gemini ⇄ Ollama ⇄ OpenAI-compatible) by **changing configuration only**. The behaviour must either be equivalent, or the build must fail early with an error naming the incompatibility. Today neither is true.
- **Trigger.** web3sec tried to run its Anthropic-backed agents (`claude-opus-4-8`) on Moonshot **`kimi-k3`**. Paladin ships a Kimi adapter (Phase 17, PROV-01..04), but a config-only swap fails or silently degrades in **five blocking** and **eleven degrading** ways (§5).
- **The headline finding is not what was expected.** The request was framed as "features the original three adapters have that the newer ones lack". Some gaps do fit that (facade exports, cache-write accounting, empty-output detection, a max-tokens default, timeouts). **But most blockers are cross-cutting**: capability modelling, configuration plumbing and fallback semantics. **Several are defects in the originals themselves**, the worst being an OpenAI adapter that sends an empty user message on the main execution path (§5, B-4).
- **Recommended course.** One phase of about six plans under a new requirement prefix **`PARITY-*`** (`PROV-*` is spent). Ordering: red tests and the OpenAI defect first; then capability modelling; then config-driven construction; then a per-model profile registry; then response/error parity; then conformance and docs (§7).
- **Three decisions need operator sign-off before planning** (§6): the ADR-0004 policy for parameters a provider does not accept, typed refusals, and the source of truth for per-model limits.

---

## 2. Background you need

**What web3sec is.** A Rust, Paladin-orchestrated, multi-agent Solidity/EVM audit system and **Paladin's only current consumer**. It runs specialist agents and proves findings with sandboxed exploit PoCs. Four of its agents are LLM-heavy and each is configured per role: a deductive analyst, a fuzz-harness author, an adversarial "Tribunal" judge, and a PoC "Prover". Today they run on Anthropic `claude-opus-4-8` and DeepSeek `deepseek-v4-pro` / `deepseek-v4-flash`.

**How web3sec consumes Paladin.** It uses a path dependency on the `paladin-ai` facade with features `["llm-deepseek", "llm-anthropic"]`. It builds `AnthropicAdapter` / `DeepSeekAdapter` **directly** from their `*Config` types, with per-role timeouts, max-token budgets and models. It does **not** use `agent_host` or `LlmProviderFactory`. Any fix that only reaches the web server's YAML agent host will not reach this consumer. **Library-level APIs matter as much as server config.**

**What web3sec will do on its side (not your job).** Add per-role `kimi` dispatch, stop unknown provider names silently falling back to DeepSeek, and add per-role model knobs. It needs from Paladin what §5 lists.

**Binding framework policy (from `.project/Milestone_13-Token-Economy/overview/`, §5):**
1. **Clean break, no shims.** Paladin is pre-1.0 with one coordinated consumer.
2. **Coordinated pointer bump.** web3sec adopts a release in one step.
3. **Vocabulary.** Units are plain, roles are medieval (Phase 30 rule). New config/descriptor structs take plain names; new role-bearing services take medieval names.
4. **Do not rewrite `.planning/` history.**

---

## 3. Live vendor facts (dated, re-verify before relying on them)

**Moonshot `/v1/models`, 2026-10-07, on the operator's key:**

| model | context_length | notes |
|---|---|---|
| `kimi-k3` | **1,048,576** | `supports_reasoning`, `supports_dynamic_tools`, `supports_image_in`/`video_in`; `supports_thinking_type: "only"` (thinking always on); efforts `low`/`high`/`max`, **default `max`** |
| `kimi-k2.6` | 262,144 | reasoning, image/video in |
| `kimi-k2.7-code` | 262,144 | reasoning |
| `kimi-k2.7-code-highspeed` | 262,144 | reasoning |

**Moonshot `/v1/chat/completions` probes on `kimi-k3`, 2026-10-13:**
- `thinking: {"type":"enabled","effort":"bogus"}` returns **HTTP 400** `"Invalid request: invalid value for thinking effort level: bogus"`. The field is parsed and validated.
- Top-level `reasoning_effort: "bogus"` returns **HTTP 200**, the same as an unknown field `zzz_unknown`. It is **silently ignored.**
- Usage reports `completion_tokens_details.reasoning_tokens` **and** `prompt_tokens_details.cache_write_tokens`.
- **Unconfigured `kimi-k3` effort is `max`, and it is extreme.** On a one-line question with `max_tokens: 16000` it reasoned for 492 s, hit `finish_reason: length` and returned **empty content**. `effort: low` answered in 56 s; `thinking:{type:"disabled"}` answered in 2 s. That works despite the model listing `supports_thinking_type: "only"`. Full table in the companion handoff, §4.1.
- Paladin already measured (Phase 17-19) that `temperature` must be omitted or exactly `1`, and `top_p` omitted or exactly `0.95`.

Keys for live work use the env vars Paladin already reads: `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `DEEPSEEK_API_KEY`, `MOONSHOT_API_KEY`, `XAI_API_KEY`, `GEMINI_API_KEY`, `DASHSCOPE_API_KEY`. On the operator's machine all except `OPENAI_API_KEY` are provisioned and authenticate (`GET /models` → 200 for each, 2026-10-07).

---

## 4. Parity matrix (as of `df24689c`)

| Capability | anthropic | openai | deepseek | kimi | qwen | grok | gemini | openai_compat | ollama |
|---|---|---|---|---|---|---|---|---|---|
| Facade re-export (`src/lib.rs`) | yes `:186-187` | yes `:180-181` | yes `:189-190` | **no** | **no** | **no** | **no** | **no** | **no** |
| Facade feature (`Cargo.toml:512,523-532`) | default | default | default | opt-in | opt-in | opt-in | opt-in | opt-in | opt-in |
| Shared conformance suite (`conformance.rs`) | **no** (one case by hand, `anthropic:1615`) | yes | yes | yes | yes | yes | yes | yes | yes |
| Live smoke (`crates/paladin-llm/examples/live_vendor_smoke.rs`) | **no** | **no** | **no** | yes | yes | yes | yes | no | no |
| `max_context_tokens` declared | 200,000 `:627` | 128,000 `:821` | 64,000 `:872` | **131,072** `kimi:196` (live: 1,048,576 for k3) | 131,072 | 131,072 | 1,048,576 | env | None |
| `temperature_range` declared | (0,1) `:629` | (0,1) | (0,2) | **(1,1)** `kimi:210` | (0,1.99) | (0,2) | (0,2) | env | (0,2) |
| `temperature` on the wire | **never** `anthropic:218,262` | always | yes | **never** `kimi:246` | yes | yes | yes | yes | yes |
| `stop_sequences` on the wire | dropped | dropped | dropped | dropped | dropped | dropped | **sent** `gemini:418` | dropped | dropped |
| `response_format` | **ignored** (prompt-only, `:1495-1505`) | json_object / json_schema | json_object | json_schema → **downgraded to json_object** `engine.rs:496-501` | same | same | mime + schema | same | same |
| User message content | `query` | **`context`** `openai:363` | `query` | `query` | `query` | `query` | `query` | `query` | `query` |
| `reasoning_tokens` mapped | yes `:748-753` | yes | yes | yes `engine.rs:539-543` | yes | yes | yes | yes | yes |
| `cache_write_tokens` mapped | **yes** `:745-746` | no | no | **no** (vendor sends it) | no | no | no | no | no |
| Reasoning text surfaced | n/a | n/a | parsed, dropped | parsed `types.rs:80`, **dropped** `engine.rs:826` | same | same | n/a | same | same |
| Empty-output guard | **any** empty text `:858-890` | **none** | length + empty `:263-287` | length + empty only `engine.rs:627-640` | same | same | MAX_TOKENS + empty | same | same |
| Refusal detection | **none**: `stop_reason:"refusal"` → `Error("unknown")` `:294` | none | none | `content_filter` → ContentFilter `engine.rs:620` | same | same | SAFETY handled | same | same |
| Usage-cap classification | 400 + body signature `:360-367` | 402 | 402 | 402 only `kimi:253` | 402 | 402 | RESOURCE_EXHAUSTED | 402 | 402 |
| Max-tokens default knob | `ANTHROPIC_MAX_TOKENS` `:80` | — | — | **none** | none | none | — | none | none |
| Default timeout (s) | 60 env; 300 via config bridge `config/bridge.rs:97` | 300 `openai:66` | 60 `deepseek:75` | **60** `kimi:118` | 60 | 60 | 60 | 60 | 120 |

Already at parity, so not gaps: redirect refusal, credential redaction, shared HTTP status mapping including context overflow (`http_status.rs:86-117`), streaming usage, system messages. No adapter implements native tool calling (ADR-0042), prompt caching (`cache_control`) or request-id propagation, so those are not Kimi-specific.

---

## 5. Gaps

Severity: **BLOCKING** means a config-only swap fails or produces wrong results. **DEGRADED** means the swap works but behaves differently without saying so. **HYGIENE** covers tests, docs and stale data.

### Blocking

**B-1. Temperature validation makes Kimi unusable from a normal agent config.**
- `PaladinBuilder::validate` refuses a manual temperature outside the provider's declared range (`src/application/services/paladin/paladin_builder.rs:1137-1153`).
- `build` refuses an auto-selected temperature outside it too (`:1323-1344`, test `:1484-1500`).
- Kimi declares `(1.0, 1.0)`. So **any** explicit temperature other than 1.0, and **any** use of `auto_temperature`, fails the build on Kimi. That happens even though the Kimi adapter never sends temperature (`kimi:246`).
- The same agent config passes on Anthropic, which declares `(0,1)` but **also** never sends temperature (`anthropic:218,262`, because Claude 4.8+ rejects it). An original adapter therefore already discards a validated temperature without telling the caller. That breaks ADR-0004's own principle (`.planning/decisions/0004-temperature-validation.md:89-94`: no undiscoverable substitution).
- **Root cause:** `temperature_range: Option<(f32,f32)>` cannot express "this provider/model does not take temperature".
- **Fix (needs D-1):** model parameter support explicitly, per provider *and per model*. Apply one consistent policy for a configured value the target ignores. See §7 Plan 2 for why this must not be a new `ProviderCapabilities` field.

**B-2. Provider configuration blocks do nothing, and the factory is environment-only.**
- `LlmProviderFactory::create(name)` builds adapters from env vars only and takes no model (`crates/paladin-llm/src/provider_factory.rs:107-120, 356-383`).
- The server's `agent_host` reads only `llm.default_provider` (`src/infrastructure/web/agent_host.rs:100-108`) and calls `factory.create(&provider)` (`:160-175`).
- `llm.<provider>.{base_url, default_model, timeout_seconds, max_retries}` in YAML are parsed but **ignored**. The conversion bridges in `crates/paladin-llm/src/config/bridge.rs` have no production caller.
- The adapter's configured default model is never read on the request path either (declared `compat/engine.rs:161`, `request.model` sent at `:505`).
- **Fix:** a config-driven constructor, e.g. `LlmProviderFactory::create_from_config(&LlmSettings, provider)`. Route it through the existing bridges, and expose it as a **library API**, not only inside `agent_host`. Per-agent `provider` + `model` already exists in YAML (`src/config/agents.rs:208-245`) and should resolve through the same path.

**B-3. A cross-provider fallback chain sends the wrong model to every provider after the first.**
- `FallbackLlmAdapter::generate` resends `request.clone()`, model name included, to every provider in the chain (`crates/paladin-llm/src/fallback.rs:257-268`).
- An Anthropic→Kimi chain therefore sends `claude-opus-4-8` to Moonshot.
- **Fix:** each chain step carries its own model, and parameter profile once B-1 exists. `request.model` is rewritten per step. Record the served model alongside the existing `SERVED_BY_METADATA_KEY`.

**B-4. The OpenAI adapter sends an empty user message on the main execution path.** This is a defect in an original adapter.
- The OpenAI adapter builds the user turn from `user_prompt.context` (`crates/paladin-llm/src/openai/adapter.rs:360-364`).
- `PaladinExecutionService` puts the prompt in `query` and sets `context: None` (`src/application/services/paladin/paladin_execution_service.rs:2471-2475`).
- Every other adapter reads `query`.
- **Fix:** send `query`, with `context` appended when present. **Write the failing mock-transport test first**: an execution-service call through `OpenAIAdapter` must put the prompt text in `messages[].content`. This is the single highest-value fix in this handoff.

**B-5. The facade does not export the six newer adapters.**
- `src/lib.rs:180-190` re-exports only the OpenAI, Anthropic and DeepSeek adapter and config types. The `llm-kimi|qwen|grok|gemini|ollama|openai-compatible` features forward to `paladin-llm`, but their types are unreachable through `paladin-ai` (ADR-0046 defines the nine features).
- A consumer that constructs adapters directly (web3sec does) cannot reach `KimiConfig` without adding a second path dependency on `paladin-llm`.
- **Fix:** `#[cfg(feature = "llm-kimi")] pub use paladin_llm::kimi::{KimiAdapter, KimiConfig};` and the same for the other five. Refresh `make api-surface`.

### Degraded

**G-1. No per-model limits.**
- Kimi declares one 131,072 window for the whole adapter (`kimi/adapter.rs:196`), against 1,048,576 for k3 and 262,144 for k2.x.
- `resolve_context_window` (`window.rs:149`) needs a caller-supplied table. Commissary supplies none and falls back to the adapter-wide value (`services/commissary.rs:364-373`).
- Commissary therefore rations k3 at about 1/8 of its real window.
- The same applies to max output tokens, reasoning support, fixed sampling values and JSON-mode support. All of these vary by **model**, not by adapter.
- **Fix:** a per-model profile registry (§7 Plan 4; D-3).

**G-2. Compat presets can return empty output as success.**
- `detect_empty_completion` fires only on `finish_reason == length` with empty content (`compat/engine.rs:627-640`). Anthropic flags empty text on any stop reason (`anthropic:858-890`). OpenAI has no guard at all.
- A thinking-only model that spends its whole budget on reasoning with `stop` returns `Ok("")`.
- **Fix:** unify on Anthropic's behaviour (empty text is `EmptyCompletion` whatever the stop reason) across all adapters, including OpenAI.

**G-3. There is no default output budget for compat presets.**
- Anthropic has `ANTHROPIC_MAX_TOKENS` (`anthropic:80`). The execution service sends `max_tokens: None` (`paladin_execution_service.rs:2478`).
- With thinking always on, an unbounded or vendor-default budget is a cost and truncation hazard.
- **Fix:** a per-provider default max-tokens setting from config (B-2), overridable per model (G-1).

**G-4. Timeout defaults are inconsistent.**
- Defaults are 60s for every compat preset and DeepSeek, 300s for OpenAI, and 300s for Anthropic only through the unused bridge.
- Measured on `kimi-k3` (2026-10-13): **56 s at `low` effort, 359 s at `high`, 473-492 s at the default `max`**, on a one-line question. Only thinking `disabled` (2 s) fits comfortably in 60 s. web3sec hit the same failure with DeepSeek: 34 of 51 batches died on a 60s client timeout.
- **Fix:** config-driven timeouts (B-2), defaulting to 300s for reasoning-capable models.

**G-5. Compat presets drop cache-write tokens.**
- `CompatPromptTokensDetails` only reads `cached_tokens` (`compat/types.rs:129-131`), and `map_compat_usage` never sets cache-write (`engine.rs:531-546`).
- Moonshot sends `prompt_tokens_details.cache_write_tokens`. Anthropic captures it (`anthropic:745-746`).
- **Fix:** add the field and call `with_cache_write`. Live-check how each compat vendor names cache reads.

**G-6. Reasoning text is parsed and then discarded.**
- `reasoning_content` is parsed (`compat/types.rs:76-80`; `deepseek/adapter.rs:178-184`), then dropped (`engine.rs:826`). The streaming delta has no field for it (`types.rs:163-166`).
- **Fix:** surface it, e.g. `metadata["reasoning_content"]` or a typed optional on `LlmResponse` (D-4 in the companion handoff). Include streaming.

**G-7. Refusals are not typed.**
- Anthropic `stop_reason:"refusal"` becomes `Error("unknown")` (`anthropic:294`), and no adapter has a refusal variant.
- Downstream relevance: web3sec's Prover was refused 4 out of 4 times by `claude-opus-5` on legitimate exploit-PoC authoring, and had to diagnose that from raw stop reasons. A typed refusal lets a consumer fall back to another provider deliberately.
- **Fix:** D-2.

**G-8. Usage-cap classification is per vendor and partly unverified.**
- Kimi maps only HTTP 402 to a usage limit (`kimi:253`, `http_status.rs:99`).
- If Moonshot signals quota exhaustion as a 429, it is retried as a transient rate limit.
- **Fix:** live-check each compat vendor's quota error and add `error_override`s like Anthropic's signature match (`anthropic:360-367`).

**G-9. Structured output differs by provider.**
- `run_structured` always appends a schema instruction to the prompt (`paladin-ports/.../structured_executor_port.rs:186`), so every provider gets prompt-level JSON. On top of that:
  - OpenAI sends native `json_schema`.
  - Compat downgrades `json_schema` to `json_object` (`engine.rs:496-501`).
  - Anthropic sends nothing.
- Whether `kimi-k3` accepts `json_object` while thinking is **unverified**.
- **Fix:** record JSON-mode support per model (G-1). Add a live check per preset.

**G-10. Stop sequences are dropped by everything except Gemini.** The execution service populates them (`paladin_execution_service.rs:2483-2487`). **Fix:** add `stop` to the compat, OpenAI, DeepSeek and Anthropic (`stop_sequences`) request bodies, gated per model where vendors reject it (xAI rejects `stop`).

**G-11. Defaults and model lists are stale.**
- Anthropic's model list is the claude-3 family (`anthropic:585-608`). Its default model is `claude-3-5-sonnet-20241022` (`config/bridge.rs:95`, `config.example.yml:33`).
- DeepSeek's default is `deepseek-chat` (`deepseek/adapter.rs:72`). The vendor's current docs name `deepseek-flash` / `deepseek-v4-pro`; whether `deepseek-chat` still resolves is unverified.
- DeepSeek declares a 64,000 window.
- **Fix:** subsumed by G-1's registry. Date every seeded value.

### Hygiene

**H-1. Anthropic is not in the shared conformance suite.** It has one hand-written case (`anthropic:1615`). The nine conformance cases (`conformance.rs:1-20`) cover none of: declared-capability truthfulness, `response_format` wire shape, empty-completion, refusal, usage-field mapping, parameter-drop behaviour.

**H-2. The live smoke covers only the newer adapters.** `live_vendor_smoke.rs` covers kimi/qwen/grok/gemini but none of the originals.

**H-3. No documentation page explains how to switch providers** or what differs between them. Phase 26 only documented `response_format` in a per-provider table.

---

## 6. Decisions for the operator (resolve before `/gsd-plan-phase`)

**D-1. Policy for a configured parameter that the target provider/model does not accept** (ADR-0004 extension). This covers temperature, top_p, penalties and stop sequences.

Options:
- (a) Refuse at build time whenever a configured value cannot be honoured.
- (b) Refuse only when the operator **explicitly** set the value. Skip auto-selected values for providers that don't take the parameter, and record that in run metadata. Omission is visible through a `warn!` and a metadata entry, not a `debug!`.
- (c) Always omit, with a warning.

**Recommendation: (b).** It keeps ADR-0004's no-hidden-substitution rule and makes a Kimi swap work with the default `auto_temperature`. It also fixes the Anthropic inconsistency, which today validates a temperature it then discards.

Whatever is chosen applies equally to the originals. Record it as an ADR amending 0004.

**D-2. Typed refusal.**
- Options: a new `LlmError::Refusal { provider, model, detail }` variant (`LlmError` is `#[non_exhaustive]`, `llm_port.rs:283-285`, so this is additive), or mapping to the existing `ContentFilter`.
- **Recommendation:** the new variant, classified `Permanent`. A policy refusal and a content filter are different signals to a consumer deciding whether to retry on another provider.

**D-3. Source of truth for per-model limits** (context window, max output, reasoning support, fixed sampling values, JSON mode).
- Options: (a) a static, dated table per adapter; (b) (a) plus config overrides; (c) live discovery from vendor `/models` where offered (Moonshot exposes `context_length` and effort lists), with (b) as fallback.
- **Recommendation: (b) now, (c) as optional later work.** Live discovery adds a network dependency to construction.

**D-4. Default facade features.** Keep `openai + anthropic + deepseek` as default (ADR-0046), or enable all nine.
- **Recommendation:** keep the defaults, and add the missing re-exports (B-5) behind the existing opt-in features.

---

## 7. Recommended course of action

**Shape.** One phase, numbered after the current last roadmap phase (37.1 at `df24689c`, so likely **Phase 38**), in a new milestone, e.g. **v0.11.0 "Provider Interchangeability"**. Requirement prefix **`PARITY-*`** (unused; `PROV-*` is spent). Follow the Roadmap Extension Protocol (`.planning/ROADMAP.md` §"Roadmap Extension Protocol"). The companion reasoning-effort handoff is the natural next phase in the same milestone.

**Plan 1: red tests and the OpenAI defect.**
- Write a **provider-swap matrix test** with mock transports: the same agent definition, the provider switched through config, asserting on the wire body and the outcome. It covers B-1 (manual and auto temperature on a (1,1) provider), B-3 (fallback model per step), B-4 (user content present) and G-2 (empty `stop` completion).
- Then fix B-4. This plan proves the harness and lands the most severe single defect.

**Plan 2: parameter-support modelling and validation (B-1, D-1).**
- **Constraint:** `ProviderCapabilities` gaining a field is deliberately blocked by the compile-time guard `provider_capabilities_gained_no_field` (`crates/paladin-ports/src/output/llm_port.rs:1778-1793`, D-28 in `.planning/milestones/v0.10.0-phases/26-agent-runtime-enhancements/26-CONTEXT.md:433-455`). It has 92 struct-literal sites.
- **Recommended shape:** an additive, plainly-named per-model descriptor (e.g. `ModelProfile`) returned by a **new `LlmPort` method with a default implementation**, e.g. `fn model_profile(&self, model: &str) -> Option<ModelProfile>`. It is non-breaking, and adapters opt in. It states each sampling parameter as `Range(lo, hi)` / `Fixed(v)` / `NotSent`. `PaladinBuilder` validation consults it before falling back to `temperature_range`.
- If the operator prefers a clean break of `ProviderCapabilities` instead, that is an explicit X-10.3 event. Record it as such.

**Plan 3: config-driven construction (B-2, B-5, G-3, G-4).**
- Add `create_from_config` honouring `llm.<provider>` blocks: model, base_url, timeout, max_retries, default max_tokens, context-window override. It goes through `config/bridge.rs`.
- Add a library-level per-agent resolution API that `agent_host` then uses.
- Add the facade re-exports for all nine adapters.
- Existing YAML files must keep loading unchanged (PROV-03's rule).

**Plan 4: per-model profile registry (G-1, G-9, G-11, D-3).**
- Seed dated profiles for every currently listed model of each vendor: context window, max output, reasoning support, fixed sampling values, JSON-mode support.
- Feed Commissary and `resolve_context_window` from it.
- Add config overrides.
- Update stale defaults and model lists.

**Plan 5: response and error parity (G-2, G-5, G-6, G-7, G-8, G-10, B-3).**
- Unified empty-completion guard (including OpenAI).
- Cache-write mapping.
- Surfaced reasoning text (normal and streaming).
- Typed refusal (D-2).
- Live-verified usage-cap overrides per compat vendor.
- Stop sequences on the wire.
- Per-step model in fallback chains.

**Plan 6: conformance, live smoke and docs (H-1..H-3).**
- Add Anthropic to the shared suite.
- Extend the suite with capability truthfulness, response_format wire shape, empty-completion, refusal, usage fields and parameter-drop policy.
- Make the live smoke cover all nine providers behind the existing credential gate.
- Add an mdBook "Switching providers" page with the per-provider/per-model behaviour table.
- Add MIGRATION/CHANGELOG entries.

**Draft requirements.**

| ID | Done when |
|---|---|
| PARITY-01 | The provider-swap matrix test passes for all nine providers: an agent definition switched only through config builds and runs, or fails at build time with an error naming the provider, model and parameter. |
| PARITY-02 | Parameter support is modelled per provider and per model, and one documented policy (D-1, ADR amending 0004) governs every adapter, the originals included. |
| PARITY-03 | Provider construction honours YAML/config blocks (model, base_url, timeout, retries, default max_tokens, window override) through a public library API. The facade exports all nine adapter/config pairs. |
| PARITY-04 | Per-model limits come from a dated registry with config overrides. Commissary rations `kimi-k3` against its real window. |
| PARITY-05 | Every adapter maps empty output, refusal, usage cap, cache read/write and reasoning tokens the same way, and surfaces reasoning text. |
| PARITY-06 | Fallback chains send each provider its own model and record the served model. |
| PARITY-07 | All nine adapters are in the shared conformance suite and the credential-gated live smoke. The mdBook documents switching providers. |
| PARITY-08 | The OpenAI adapter sends the prompt text on the execution-service path, with a regression test. |

Standard bars apply: TDD, the 82% coverage floor (ADR-0006), rustdoc plus doc tests on new public items, `make clean-code`, `make api-surface` with a CHANGELOG entry for intentional surface changes, and no `unwrap`/`expect`/`panic!` in library code.

---

## 8. Relationship to the reasoning-effort handoff

`../Adjustable-Reasoning-Effort/PALADIN-HANDOFF.md` adds a provider-neutral reasoning-effort setting. It depends on this phase in three places, so **sequence this phase first** (or at least Plans 2 and 4):
1. Which effort levels a model accepts is per-model data, and belongs in this phase's model-profile registry.
2. The policy for an unsupported value is the D-1 policy, applied with stricter defaults.
3. Effort only matters if output budgets and timeouts are configurable (G-3, G-4).

---

## 9. Out of scope

- Native tool/function calling and compat vision. These are tracked by ADR-0042. Kimi K3 supports both, but no Paladin adapter implements them, so they don't block a swap.
- Prompt caching (`cache_control`) and request-id/trace propagation. No adapter implements either.
- The reasoning-effort setting itself (companion handoff).
- All web3sec-side changes.

---

## Appendix A — How this was produced

- **Read-only audit** of an export of `origin/main` @ `df24689c`, by two research passes and the coordinating agent.
- **Claims the coordinating agent re-checked line by line before writing this:** B-1 (both paths), B-2 (`agent_host`, factory, bridge callers), B-3, B-4, the Anthropic temperature omission, `ANTHROPIC_MAX_TOKENS`, the compat empty guard, compat cache-details, and the `ProviderCapabilities` guard and D-28.
- **Vendor facts** come from authenticated calls on the operator's keys on the dates given.
- Nothing in the Paladin tree was modified.
