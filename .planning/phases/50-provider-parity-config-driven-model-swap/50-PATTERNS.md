# Phase 50: Provider Parity & Configuration-Driven Model Swap - Pattern Map

**Mapped:** 2026-10-15
**Files analyzed:** 27 (new and modified)
**Analogs found:** 27 / 27 (3 role-match only, noted)
**Basis:** CONTEXT.md fully read; RESEARCH.md lines 1-476 read (the rest, Open Questions and later sections, not read). Excerpts below were read at HEAD.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match |
|---|---|---|---|---|
| `crates/paladin-ports/src/output/model_profile.rs` (new) | model (value object) | transform | `ProviderCapabilities` in `llm_port.rs` + `CompatRequestParameters` (`compat/engine.rs:129`) | role-match |
| `crates/paladin-ports/src/output/llm_port.rs` (mod: `LlmError::Refusal`, `LlmPort::model_profile`, `LlmResponse.reasoning`, `StreamingResponse.reasoning`, `transience()` arm) | port | request-response | itself (`EmptyCompletion` arm `:638`; `response_format` defaulted method; guard `:1977`) | exact |
| `crates/paladin-core/src/platform/container/transience.rs` (consumer only) | model | transform | `llm_port.rs:621` table | exact |
| `crates/paladin-battalion/src/llm_failure.rs` (+ `llm_decision.rs`) | utility (label table) | transform | `EmptyCompletion` rows (`:287`, `:370`) | exact |
| `crates/paladin-llm/src/sampling.rs` (new) | utility | transform | `CompatRequestParameters` gate + `CompatEngine::build_request` | role-match |
| `crates/paladin-llm/src/turn.rs` (new; `user_turn_text`) | utility | transform | `openai/adapter.rs:432-437` (defect site) | partial |
| `crates/paladin-llm/src/response_guards.rs` (new; `classify_completion`) | utility | transform | `compat/engine.rs:651` and `deepseek/adapter.rs:296` | exact |
| `crates/paladin-llm/src/registry/*.rs` (new) | config (dated static tables) | transform | `KIMI_FALLBACK_MODELS` const + inline capabilities in `kimi/adapter.rs:60-64`, `:190-210` | role-match |
| `crates/paladin-llm/src/window.rs` (+ `effective_capabilities`) | utility | transform | `resolve_context_window` `:149` | exact |
| `crates/paladin-llm/src/services/commissary.rs` and root `HistoryTrimmer` (readers) | service | transform | `resolve_context_window` callers | exact |
| `crates/paladin-llm/src/provider_factory.rs` (+ `create_from_config`) | service (factory) | request-response | `create` `:356` + `provider_registry` `:304` | exact |
| `crates/paladin-llm/src/config/llm.rs`, `config/bridge.rs` (new fields, `from_layers`) | config | transform | `bridge.rs:62-144`; `KimiConfig::from_parts` `kimi/adapter.rs:107` | exact |
| `<vendor>/adapter.rs` x9 (`*Config` fields, `model_profile`, sampling filter, refusal, reasoning) | adapter | request-response + streaming | `kimi/adapter.rs` (compat), `openai/adapter.rs` (bespoke) | exact |
| `crates/paladin-llm/src/compat/engine.rs` (empty any-stop, cache-write, reasoning, stop) | adapter engine | request-response + streaming | itself `:651` | exact |
| `crates/paladin-llm/src/fallback.rs` (`FallbackStep`, `from_steps`, hop rewrite, refusal flag) | service (decorator) | request-response | itself `:418-452` | exact |
| `crates/paladin-llm/src/conformance.rs` (+ Anthropic `Wire`, new cases, `CASE_COUNT`) | test harness | request-response | `KimiFixture` `kimi/adapter.rs:1355-1406` | exact |
| `crates/paladin-llm/examples/live_vendor_smoke.rs` | test (live) | request-response | itself (kimi/qwen/grok/gemini blocks) | exact |
| `src/application/services/paladin/paladin_builder.rs` (refuse explicit, warn auto) | service (validation) | request-response | `:1137-1152`, `:1329-1348` | exact |
| `src/infrastructure/web/agent_host.rs`, `facade_provisioner.rs`, `agent_runtime.rs` (call sites) | composition root | request-response | `agent_host.rs:111,210` | exact |
| `src/lib.rs` (six re-export pairs) | config (facade) | n/a | `src/lib.rs:178-190` | exact |
| `tests/provider_swap_matrix.rs` (new) + `[[test]]` in `Cargo.toml` | test | request-response | `tests/consumer_surface_muster.rs` + `Cargo.toml:453-456` | role-match |
| `tests/consumer_surface_muster.rs` (update) | test | compile-only | itself | exact |
| `.planning/decisions/0061-*.md`, ADR-0004 note, `PROMOTION.md` | docs | n/a | ADR-0006/0046 in-place note style | exact |
| `MIGRATION.md` §9.2/9.5/9.8, `CHANGELOG.md`, `.cargo/semver-checks-allowlist.toml`, `paladin-llm/Cargo.toml` lints table | docs/config | n/a | existing `LlmResponse` §9.2 row; `paladin-ports/Cargo.toml` lints table | exact |
| `docs/src/user-guides/switching-providers.md` + `SUMMARY.md` | docs | n/a | `docs/src/appendix/provider-expansion.md` | role-match |
| `config.example.yml` | config | n/a | itself | exact |

## Pattern Assignments

### `crates/paladin-ports/src/output/llm_port.rs` (port; Refusal, model_profile, reasoning)

**Analog:** the same file.

**Transience table, where `Refusal` joins as `Permanent`** (`llm_port.rs:633-638`):
```rust
LlmError::AuthenticationError(_) => Transience::Permanent,
LlmError::InvalidPrompt(_) => Transience::Permanent,
LlmError::UsageLimitExceeded { .. } => Transience::Permanent,
LlmError::ModelNotAvailable(_) => Transience::Permanent,
LlmError::TokenLimitExceeded => Transience::Permanent,
LlmError::EmptyCompletion(_) => Transience::Permanent,
```
Add `LlmError::Refusal { .. } => Transience::Permanent,` in this block. Rule at `:612-620`: arms read typed fields or variant identity only, never rendered `Display`.

**Test-table rows** (`llm_failure.rs:287`, `:370`): `LlmError::EmptyCompletion("...".to_string())` appears in two tables. Add a `Refusal` row to the variants-with-status/without-status test lists. The `Refusal { provider, model, detail }` variant has no status, so it goes in the "without a status" list at `:367-372`.

**Compile guard that must stay untouched** (`llm_port.rs:1977-1988`):
```rust
#[test]
fn provider_capabilities_gained_no_field() {
    let _capabilities = ProviderCapabilities {
        supports_streaming: true, supports_tool_calling: false, supports_function_calling: false,
        supports_vision: false, supports_embeddings: false, max_context_tokens: None,
        supports_system_messages: true, temperature_range: None,
    };
}
```
Put `ModelProfile` in the new `model_profile.rs`, not here, so this guard keeps passing (D-00e, D-03).

**Trait-default-body precedent:** copy the Phase 26 `response_format` defaulted method on `LlmPort`. Add `fn model_profile(&self, _model: &str) -> Option<ModelProfile> { None }`. Decorators (`PricingLlmAdapter`, `CadenceLlmAdapter`, `FallbackLlmAdapter`) must override and delegate (RESEARCH Pattern 1). `LlmResponse` is NOT `#[non_exhaustive]` (RESEARCH premise correction 1). `StreamingResponse` is `#[non_exhaustive]` with `delta()/terminal()/with_usage()` constructors (`:1183`); add the reasoning field and a `with_reasoning` constructor there.

---

### `crates/paladin-ports/src/output/model_profile.rs` (new value object)

**Analog:** `CompatRequestParameters` (`compat/engine.rs:119-172`). Its deliberate no-`Default`, state-every-field posture is the in-tree precedent being generalised.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompatRequestParameters {
    pub temperature: bool,
    pub max_tokens: bool,
    pub top_p: bool,
    pub frequency_penalty: bool,
    pub presence_penalty: bool,
}
impl CompatRequestParameters {
    pub const fn all() -> Self { Self { temperature: true, max_tokens: true, top_p: true,
        frequency_penalty: true, presence_penalty: true } }
}
```
Follow RESEARCH Pattern 1: `#[non_exhaustive]` serde struct, constructed only via `ModelProfile::new()` and `with_*`, so Phase 51 effort fields are free. Do NOT give it a struct-update `Default` spread. The doc comment at `compat/engine.rs:122-127` explains why a spread caused the Grok defect. `ParamSupport` is `Range(f32,f32) | Fixed(f32) | NotSent` (D-03, D-04a).

---

### `crates/paladin-llm/src/sampling.rs` (new wire-time filter)

**Analog:** the `request_parameters` gate in `CompatEngine::build_request`. Kimi's declaration is the data it consumes (`kimi/adapter.rs:245-251`):
```rust
request_parameters: CompatRequestParameters {
    temperature: false, top_p: false, max_tokens: true,
    frequency_penalty: true, presence_penalty: true,
},
```
Replace the `bool`s with a `ModelProfile` lookup. The filter must take the profile plus `PromptParameters` and return `(sent, omissions)`, with one `warn!` per adapter instance (`AtomicBool`) and a `paladin.*` metadata key stamped on every response (RESEARCH Pattern 2). The existing `debug!` at drop (referenced in `compat/engine.rs:143`) must become `warn!` (D-01).

All five request builders call it: CompatEngine, OpenAI (remove fabricated `0.7`/`1.0`/`4096` defaults at `openai/adapter.rs:~760-790`), Anthropic (`temperature: None` at `:259`, `:303`), DeepSeek, Gemini.

---

### `crates/paladin-llm/src/turn.rs` (new `user_turn_text`) and the OpenAI B-4 fix

**Defect site** (`openai/adapter.rs:432-437`):
```rust
PromptType::User(user_prompt) => {
    messages.push(OpenAIMessage {
        role: "user".to_string(),
        content: user_prompt.context.clone().unwrap_or_default(),
    });
}
```
Replace with `content: user_turn_text(user_prompt)` returning `query` plus `"\n\n" + context` when `context` is `Some`. Every other adapter already reads `query`. The execution service sets `context: None` (`paladin_execution_service.rs:2786`, `:2801`, `:2975`). Route all nine adapters through the one helper. The PARITY-08 mock-transport test is not `#[ignore]`d: run it red, then fix (RESEARCH plan 50-01).

---

### `crates/paladin-llm/src/response_guards.rs` (new `classify_completion`)

**Analog:** `compat/engine.rs:647-661` (DeepSeek `deepseek/adapter.rs:296` and Gemini carry hand-synced copies):
```rust
fn detect_empty_completion(content: &str, finish_reason: &FinishReason) -> Option<LlmError> {
    if matches!(finish_reason, FinishReason::Length) && content.trim().is_empty() {
        Some(LlmError::EmptyCompletion(format!(
            "finish_reason=length with empty content ({} raw chars) — reasoning likely \
             consumed the entire max_tokens budget; retry with a larger max_tokens",
            content.len()
        )))
    } else { None }
}
```
Change per D-11a: drop the `Length` guard so empty text is `EmptyCompletion` on any stop reason. Carve-outs: order refusal first, then `ContentFilter` stays `Ok` (Gemini test at `gemini/adapter.rs:~1910` pins `SAFETY` with no text as not-empty). Ordering is refusal, then content_filter, then empty (RESEARCH Open Question 4).

**Refusal.detail must use** `crate::redaction::redact_and_bound(body, &api_key)` (as in `compat/engine.rs:665-667`):
```rust
fn diagnostic_excerpt(&self, body: &str) -> String {
    redact_and_bound(body, &self.config.api_key)
}
```
Redact BEFORE truncate (D-00a, `security.instructions.md`).

---

### `crates/paladin-llm/src/config/bridge.rs` and `provider_factory.rs::create_from_config`

**Bridge pattern to extend** (`bridge.rs:83-100`, the stale default to replace per D-08):
```rust
#[cfg(feature = "anthropic")]
impl From<&LlmProviderConfig> for AnthropicConfig {
    fn from(cfg: &LlmProviderConfig) -> Self {
        Self {
            api_key: cfg.api_key.clone(),
            base_url: cfg.base_url.clone()
                .unwrap_or_else(|| "https://api.anthropic.com/v1".to_string()),
            model: cfg.default_model.clone()
                .unwrap_or_else(|| "claude-3-5-sonnet-20241022".to_string()),
            max_tokens: 4096,
            timeout_seconds: cfg.timeout_seconds.unwrap_or(300),
        }
    }
}
```
Note the inconsistent timeout defaults (DeepSeek and Kimi use 60, others 300). D-13 requires a 300 s default for reasoning-capable models. Kimi's `From` shows the const-default pattern to copy: `crate::kimi::adapter::KIMI_DEFAULT_BASE_URL.to_string()`. Each impl is `#[cfg(feature = "...")]`-gated.

**Layered-env pattern (Pattern 3)** — copy `KimiConfig::from_parts` (`kimi/adapter.rs:94-131`): the pure function takes `Option<String>` inputs so tests never call `unsafe set_var`; `paladin-llm` has `#![deny(unsafe_code)]`:
```rust
pub fn from_env() -> Result<Self, String> {
    Self::from_parts(env::var("MOONSHOT_API_KEY").ok(), env::var("MOONSHOT_BASE_URL").ok(),
        env::var("MOONSHOT_MODEL").ok(), env::var("MOONSHOT_TIMEOUT_SECONDS").ok())
}
fn from_parts(api_key: Option<String>, base_url: Option<String>, model: Option<String>,
    timeout_seconds: Option<String>) -> Result<Self, String> { /* validate(); Ok(config) */ }
```
Add `from_layers(block, per_agent_model, env: &dyn Fn(&str)->Option<String>)` beside it. Keep every `new()` signature (the muster pins `AnthropicConfig::new` with 4 args at `tests/consumer_surface_muster.rs:319`).

**Factory analog** (`provider_factory.rs:356-374`) — add `create_from_config` beside `create`, same name normalisation and registry lookup:
```rust
pub fn create(&self, provider_name: &str) -> Result<Arc<dyn LlmPort>, ProviderFactoryError> {
    let lower = provider_name.to_lowercase().replace('_', "-");
    provider_registry().iter().find(|row| row.name == lower)
        .map(|row| (row.construct)())
        .unwrap_or_else(|| Err(ProviderFactoryError::UnknownProvider(provider_name.to_string())))
}
```
`ProviderRegistration` gains a `construct_from_config` sibling to `construct`. The error for a missing provider block must still name provider and model. Call sites: `agent_host.rs:210`, `facade_provisioner.rs:348`, `agent_runtime.rs:1247`.

---

### Adapter files x9 (`kimi`, `qwen`, `grok`, `ollama`, `openai_compatible`, `openai`, `anthropic`, `deepseek`, `gemini`)

**Analog:** `kimi/adapter.rs` (compat preset, simplest). Pattern: a `*Config` struct with `new()` + `from_env()` + `validate()`, an adapter wrapping `CompatEngine`, and an inline `CompatCapabilities` with a dated measurement comment (`:184-211`). The inline Kimi `max_context_tokens: Some(131_072)` and `temperature_range: Some((1.0, 1.0))` move into the registry as dated `ModelProfile` entries (kimi-k3 = 1,048,576 per D-05, subject to the live probe). Each compat adapter sets `error_override: None` (`:253`) and `redirect_policy: Some(reqwest::redirect::Policy::none())` (`:264`); keep both. Bespoke adapters (OpenAI, Anthropic, DeepSeek, Gemini) each carry their own copy of request building, empty guard and usage mapping and need separate edits (RESEARCH premise correction 3).

---

### `crates/paladin-llm/src/fallback.rs` (FallbackStep, from_steps)

**Analog:** itself (`:418-452`). Change the hop loop so `request.model` is rewritten before `provider.generate(...)` and the step model goes to `pace_hop`:
```rust
for (index, provider) in self.chain.iter().enumerate() {
    let name = provider.get_provider_name();
    let mut first_429 = None;
    loop {
        match provider.generate(request.clone()).await {           // :425  rewrite model here
            Ok(mut response) => {
                response.metadata.insert(SERVED_BY_METADATA_KEY.to_string(), name.to_string());
                return Ok(response);                               // add served-model key beside
            }
            Err(err) => {
                let paced = self.pace_hop(provider, &request.model, &mut first_429, &err, &mut attempts).await;  // :436 pass step model
```
The identical shape repeats in `generate_stream` (`:467`, `:492`). `SERVED_BY_METADATA_KEY = "paladin.served_by"` is at `:136`; add a sibling constant in the same style. The opt-in refusal-hop flag is read in `after_failure`; default off, so `Permanent` short-circuits as today. `new(Vec<Arc<dyn LlmPort>>)` stays.

---

### `crates/paladin-llm/src/window.rs` and Commissary readers

**Analog:** `resolve_context_window` (`:149-184`), with the three-step order config table, then `capabilities.max_context_tokens`, then `WindowFallbackPolicy`. Add a sibling `effective_capabilities(port, model)` helper that overlays the profile onto `ProviderCapabilities` before this call. Do NOT add a `WindowSource` variant (public enum with `ALL: [WindowSource; 4]` and pinned labels). The unknown-model path (D-07) emits `warn!` plus a metadata entry.

---

### `src/application/services/paladin/paladin_builder.rs`

**Manual branch to extend** (`:1137-1152`):
```rust
if self.manual_temperature_override {
    let (min_temp, max_temp) = self.llm_port.get_capabilities()
        .temperature_range.unwrap_or((0.0, 1.0));
    if !(self.data.temperature >= min_temp && self.data.temperature <= max_temp) {
        return Err(PaladinError::ConfigurationError(format!(
            "temperature must be between {} and {} for provider '{}', got {}",
            min_temp, max_temp, self.llm_port.get_provider_name(), self.data.temperature)));
    }
}
```
Consult `self.llm_port.model_profile(&self.data.model)` first and fall back to `temperature_range` on `None`. The error must name provider, model and parameter (add model).

**Auto branch to invert** (`:1329-1348`): currently returns `ConfigurationError` when the auto-selected value is out of range. Under D-01 this becomes `log::warn!` plus omit, not a refusal. Keep the `log::info!("Auto-selected temperature {} ...")` line that follows.

Also covers `top_p` and the penalties and stop-sequence setters (D-01, D-02): explicit values refused, `PaladinData` shape unchanged (197 literal sites, muster constructs it with thirteen fields).

---

### `crates/paladin-llm/src/conformance.rs` (Anthropic fixture and new cases)

**Analog:** `KimiFixture` (`kimi/adapter.rs:1355-1406`):
```rust
mod conformance_suite {
    use super::*;
    use std::sync::Arc;
    struct KimiFixture;
    impl crate::conformance::ConformanceFixture for KimiFixture {
        const WIRE: crate::conformance::Wire = crate::conformance::Wire::OpenAiChat;
        fn adapter(base_url: &str) -> Arc<dyn LlmPort> { /* Config::new(..) -> Adapter::new(..) */ }
        fn success_body() -> String { /* real vendor-shaped JSON */ }
        fn stream_body() -> String { /* SSE frames; trailing usage frame equals success_body figures */ }
        fn error_body(status: u16) -> String { /* json */ }
    }
    crate::llm_conformance_suite!(KimiFixture);
}
```
Anthropic is the only adapter not instantiating the macro; it needs a new `Wire::Anthropic` variant. The macro asserts `CASE_COUNT == 10` (`conformance.rs:~827`, derived from the case identifier list at `:576`). Update that number and its doc whenever cases are added.

---

### `tests/provider_swap_matrix.rs` (new)

**Analog:** `tests/consumer_surface_muster.rs` registration (`Cargo.toml:450-456`):
```toml
[[test]]
name = "consumer_surface_muster"
path = "tests/consumer_surface_muster.rs"
required-features = ["llm-anthropic", "llm-deepseek"]
```
Register the matrix as `required-features = ["llm-all"]`. Use `mockito`/`wiremock` (already dev-deps) for mock transports and the `KimiConfig::new`/`AnthropicConfig::new` direct-construction path. Commit with not-yet-fixed cases `#[ignore = "red until plan 50-0N"]` and paste the `--ignored` red run into the plan SUMMARY (RESEARCH plan 50-01).

---

### `src/lib.rs` (facade re-exports)

**Pattern** (`src/lib.rs:178-190`):
```rust
#[cfg(feature = "llm-anthropic")]
pub use paladin_llm::anthropic::{AnthropicAdapter, AnthropicConfig};
#[cfg(feature = "llm-deepseek")]
pub use paladin_llm::deepseek::{DeepSeekAdapter, DeepSeekConfig};
```
Add six more blocks with identical shape for kimi, qwen, grok, gemini, ollama and openai-compatible, each behind its existing opt-in `llm-*` feature (D-14). Then run `make api-surface-update` and add a CHANGELOG entry.

## Shared Patterns

### Redact before truncate
**Source:** `crates/paladin-llm/src/redaction.rs` (`redact_and_bound`), used at `compat/engine.rs:665`.
**Apply to:** `Refusal.detail`, every new vendor-text-in-error path.

### HTTP status to LlmError
**Source:** `crate::http_status::map_http_status_with_hints` via `CompatEngine::map_error_with_hints` (`compat/engine.rs:690-702`); vendor quirks go through `error_override: Option<fn(u16,&str)->Option<LlmError>>`.
**Apply to:** per-vendor usage-cap and quota-shape overrides (D-11a; shapes must be live-verified first).

### No redirects on credential-carrying clients
**Source:** `redirect_policy: Some(reqwest::redirect::Policy::none())` (`kimi/adapter.rs:264`).
**Apply to:** every adapter client and any new construction path.

### Pure-function config layering, no env mutation in tests
**Source:** `KimiConfig::from_parts` (`kimi/adapter.rs:107`). Edition 2024 plus `deny(unsafe_code)` forbid `set_var`.
**Apply to:** all nine `from_layers` and `create_from_config` tests.

### Additive public surface and trait defaults
**Source:** `#[non_exhaustive]` on `LlmError`, `LlmRequest`, `ProviderCapabilities`, `StreamingResponse`; defaulted `LlmPort` methods (Phase 26 `response_format`).
**Apply to:** `model_profile`, `Refusal`, `FallbackStep`. Registered breaks (`LlmResponse.reasoning`, `*Config` fields) need a `MIGRATION.md` §9.2 row, a measured lint in `.cargo/semver-checks-allowlist.toml` and a lints table in `crates/paladin-llm/Cargo.toml` (none exists yet; `paladin-ports/Cargo.toml` is the template).

### Visibility rule for unsent or substituted values
**Source:** `SERVED_BY_METADATA_KEY = "paladin.served_by"` (`fallback.rs:136`).
**Apply to:** the D-01 dropped-parameter key and the D-07 unknown-model key (`paladin.*` style, `warn!` plus metadata, never `debug!`).

### Tests inside the file
**Source:** `#[cfg(test)] mod` blocks with `mockito` bodies copied from real vendor responses (e.g., `kimi/adapter.rs:1355`). All public APIs need doc tests.

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `crates/paladin-llm/src/registry/*.rs` dated per-model tables | config | transform | No per-model table exists. Only per-adapter inline `CompatCapabilities` and `*_FALLBACK_MODELS` consts. Use RESEARCH Pattern 1 sketch plus a verified-on date per entry; seed only from the D-08 operator-keys probe, not from RESEARCH vendor facts. |
| Cross-run-metadata keys and one-per-instance `warn!` (`AtomicBool`) | utility | event-driven | No existing once-per-instance warn helper. Design from RESEARCH Pattern 2. |
| Layered config with `${VAR}` placeholders | config | transform | RESEARCH notes no `${VAR}` expansion exists today; Open Question 6. |

## Metadata

**Analog search scope:** `crates/paladin-llm/src`, `crates/paladin-ports/src/output`, `crates/paladin-battalion/src`, `src/application/services/paladin`, `src/infrastructure/web`, `src/lib.rs`, root `Cargo.toml`.
**Files read for excerpts:** kimi/adapter.rs, config/bridge.rs, compat/engine.rs, fallback.rs, provider_factory.rs, llm_port.rs, openai/adapter.rs, window.rs, paladin_builder.rs, src/lib.rs.
**Caveats:** RESEARCH.md lines 477-814 were not read (Open Questions, pitfalls, vendor facts). Line numbers are from HEAD at read time and may drift. Gemini, Anthropic, DeepSeek and Qwen adapters were located by grep only, not read in full; Kimi stands in for the compat shape and OpenAI for the bespoke shape.
**Pattern extraction date:** 2026-10-15
