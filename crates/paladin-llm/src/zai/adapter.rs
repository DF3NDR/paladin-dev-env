//! Z.ai (Zhipu AI international platform, GLM models) LLM Adapter.
//!
//! A thin preset sitting entirely on [`crate::compat::CompatEngine`] -- the shared
//! OpenAI-compatible protocol engine. This adapter supplies only Z.ai's `base_url`, credential
//! env var, default model, curated fallback model list and capabilities block; every other
//! behaviour (request shaping, retry, streaming, error mapping, credential redaction, memoized
//! model-list resolution) is inherited from the engine unchanged.
//!
//! ## Provenance: nothing here is live-measured
//!
//! Every Z.ai value compiled into this module was read on the vendor's official documentation
//! site, `docs.z.ai`, on **2026-11-17**. No live probe has been run against `api.z.ai`: no seed
//! probe (so the seeded model table is empty and every model is unprofiled, see
//! [`crate::registry`]), no effort probe (so the reasoning wire is
//! [`ReasoningWire::Unsupported`] and every explicit effort is refused), no usage probe and no
//! quota-body capture (so there is no `error_override`). A documented value is a vendor claim,
//! not a measurement; each one below is labelled "vendor-documented, not live-measured".
//!
//! ### Verified facts (read on docs.z.ai)
//!
//! | # | Fact | Value | Source (accessed 2026-11-17) |
//! |---|---|---|---|
//! | F1 | General API base URL | `https://api.z.ai/api/paas/v4` | <https://docs.z.ai/api-reference/introduction> ("API Endpoint") |
//! | F2 | Chat path and body | `POST {base}/chat/completions`, OpenAI chat body (`model`, `messages`, `stream`, ...) | <https://docs.z.ai/api-reference/introduction> (cURL example), <https://docs.z.ai/api-reference/llm/chat-completion> (`POST /paas/v4/chat/completions`) |
//! | F3 | Authentication | `Authorization: Bearer <key>` | <https://docs.z.ai/api-reference/introduction> ("Authentication") |
//! | F4 | Default model | `glm-5.3` | <https://docs.z.ai/api-reference/llm/chat-completion> (every request example and the text-model `default`), <https://docs.z.ai/guides/overview/overview> ("Latest Models", flagship), <https://docs.z.ai/guides/llm/glm-5.3> ("text-only inputs") |
//! | F5 | Fallback models | `glm-5.3`, `glm-5.2`, `glm-5.1`, `glm-5` | <https://docs.z.ai/guides/overview/overview> (model tables, page order), <https://docs.z.ai/api-reference/llm/chat-completion> (text-model `enum`, same order) |
//! | F6 | Other official endpoint | Coding-plan endpoint `https://api.z.ai/api/coding/paas/v4` (see below) | <https://docs.z.ai/devpack/quick-start> ("Endpoint Guide"), <https://docs.z.ai/devpack/notice/api-key-management> |
//! | F7 | Temperature range | inclusive `[0.0, 1.0]` | <https://docs.z.ai/api-reference/llm/chat-completion> (`temperature`: "within the range: `[0.0, 1.0]`") |
//! | F8 | Request fields | `temperature`, `max_tokens`, `top_p` and `stop` are documented; `frequency_penalty` and `presence_penalty` are not | <https://docs.z.ai/api-reference/llm/chat-completion> (`ChatCompletionTextRequest`) |
//! | F9 | Thinking control | `thinking: {"type": "enabled" or "disabled"}`, plus `reasoning_effort` (`GLM-5.3`: `low`/`high`/`max`) | <https://docs.z.ai/api-reference/llm/chat-completion> (`ChatThinking`, `reasoning_effort`) |
//! | F10 | Thinking default | enabled by default; `glm-5.3` supports `thinking.type: enabled` only | <https://docs.z.ai/guides/llm/glm-5.3> (parameter table), <https://docs.z.ai/api-reference/llm/chat-completion> (`ChatThinking`) |
//! | F11 | Streamed usage | `usage` "only appears in the last chunk", the one that also carries `finish_reason`; no `stream_options` is documented | <https://docs.z.ai/guides/capabilities/streaming> ("Response Format Description" and the response example) |
//! | F12 | Models listing endpoint | none documented: the API-reference index lists no `/models` page | <https://docs.z.ai/llms.txt> (documentation index) |
//!
//! ### How each fact is used
//!
//! - **F1-F3** are the engine's fixed shape: the default [`ZAI_DEFAULT_BASE_URL`], the
//!   `/chat/completions` path the engine appends, and its bearer header.
//! - **F4, F5** are [`ZAI_DEFAULT_MODEL`] and [`ZAI_FALLBACK_MODELS`]. The default is the model
//!   the chat-completion reference's own request examples use, and the overview lists it as the
//!   current flagship; its guide states it takes text-only input, so it is a text,
//!   general-purpose model. The fallback list is that model first, then the next three current
//!   text-only general-purpose ids in the overview's page order. Excluded as not text-only:
//!   `glm-5.3-flash`, `glm-5.3-flashx` and `glm-5.3-flashx-legal` (the reference documents them
//!   under its *Vision Model* request, and the overview calls the Flash tier multimodal). Also
//!   not listed: `glm-4.7` and `glm-4.6`, which the page ranks after the cut of four.
//! - **F7** is the declared `temperature_range`. The docs state an inclusive range, used as
//!   written. Vendor-documented, not live-measured.
//! - **F8**: `temperature`, `max_tokens` and `top_p` are carried; the penalties are not
//!   (`false`). `stop` is documented in prose as "currently, only one stop word is supported"
//!   while the same field's schema carries `maxItems: 4`; the two disagree, so the fail-closed
//!   reading wins: [`StopSupport::Supported`] with `max: Some(1)`. A request with a second stop
//!   sequence has `stop` omitted and recorded as omitted, never sent.
//! - **F10** is the timeout default: the reasoning-model value
//!   (`registry::REASONING_DEFAULT_TIMEOUT_SECONDS`, 300 s) rather than the 60 s hosted-preset
//!   default, because the default model reasons before it answers.
//! - **F11**: the engine reads usage from either a trailing empty-`choices` frame or a chunk that
//!   also carries `finish_reason`, so the documented shape needs no special case (see the
//!   `generate_stream_reads_usage_from_the_finish_chunk` test).
//! - **F12**: no listing endpoint is documented, so [`ZAI_FALLBACK_MODELS`] is what
//!   `get_available_models` yields whenever the engine's `/models` request does not answer with a
//!   non-empty list. It is a curated list, never reported as authoritative.
//!
//! ### Reasoning (F9): a candidate wire only
//!
//! The documented `thinking` object and `reasoning_effort` field are *candidates* for a wire, not
//! a wire. Per ADR-0061/ADR-0062 and Phase 51 D-04 a reasoning wire ships only on a live effort
//! probe (the `51-EFFORT-PROBE.md` evidence style). No such probe exists for Z.ai, so the
//! declared wire is [`ReasoningWire::Unsupported`]: nothing is written to the request body and
//! every explicit effort is refused with a `WireUnsupported` error naming the wire, before any
//! HTTP call. The vendor's thinking default (F10) still applies on its side, since the adapter
//! sends neither field.
//!
//! ### Two key families (F6)
//!
//! Z.ai documents a second, separate endpoint for its GLM Coding Plan,
//! `https://api.z.ai/api/coding/paas/v4`, and states that, from 2026-10-16, Standard API keys
//! (pay-as-you-go or usage bundles) and Coding Plan keys are managed separately and that "the key
//! type determines how calls are billed" (<https://docs.z.ai/devpack/notice/api-key-management>).
//! This preset defaults to the general endpoint. An operator with a Coding Plan reaches the
//! other endpoint with `ZAI_BASE_URL`, and is billed against that plan only with a Coding Plan
//! key; a Standard key used there is billed pay-as-you-go. That is the vendor's own billing
//! rule, restated here so the override is not mistaken for a free switch.
//!
//! | Endpoint | Base URL | Source (accessed 2026-11-17) |
//! |---|---|---|
//! | General (shipped default) | `https://api.z.ai/api/paas/v4` | <https://docs.z.ai/api-reference/introduction> |
//! | GLM Coding Plan | `https://api.z.ai/api/coding/paas/v4` | <https://docs.z.ai/devpack/quick-start> |
//!
//! ### Unverified, deliberately not shipped
//!
//! - The mainland (Zhipu BigModel) platform's endpoint: `docs.z.ai` does not document it, so it
//!   is not named here as an endpoint and no key-scoping claim is made about it.
//! - Context windows and maximum outputs: `docs.z.ai` states a 1M-token window and a 128K
//!   maximum output for the default model, but this file's single-source rule (see
//!   `registry/seeded.rs`) seeds nothing from documentation alone, so the declared window is
//!   `None` until the operator sets one (`context_window`).
//! - Prices: the crate carries none (ADR-0052); an operator supplies a `PriceTable`.
//! - Usage semantics: the engine default (`ReasoningInsideCompletion`) is kept and was not
//!   live-measured; a mismatch with the vendor's own total surfaces at runtime as
//!   `paladin.usage_unreconciled` (G-50-5a).
//! - A quota or business-code error body: the vendor documents business codes
//!   (<https://docs.z.ai/api-reference/api-code>), but no real body was captured, so
//!   `error_override` is `None` and the shared status mapping applies.
//! - Tool calling, function calling, vision and embeddings: the vendor documents tool calls and
//!   vision models, but this is the text adapter and reports only what its own request path
//!   implements (the engine request has no tools field), so all four are `false`.

use async_trait::async_trait;
use futures::Stream;
use std::collections::BTreeMap;
use std::env;

use paladin_ports::output::llm_port::{
    LlmError, LlmPort, LlmRequest, LlmResponse, ProviderCapabilities, StreamingResponse,
};
use paladin_ports::output::llm_port::{ReasoningEffort, ReasoningFallback};

use paladin_ports::output::model_profile::{ModelProfile, StopSupport};

use crate::compat::{
    CompatCapabilities, CompatEngine, CompatEngineConfig, CompatRequestParameters, ReasoningWire,
};
use crate::redaction::RedactedKey;
use crate::registry::{
    DEFAULT_MAX_RETRIES, ModelOverride, ModelRegistry, REASONING_DEFAULT_TIMEOUT_SECONDS,
    config_knob_builders, default_timeout_seconds, fallback_from_compat, is_http_url, seeded,
    seeded_window,
};

/// Default Z.ai general API base URL (the OpenAI-compatible chat endpoint lives at
/// `{this}/chat/completions`).
///
/// Source: <https://docs.z.ai/api-reference/introduction> ("API Endpoint"), accessed 2026-11-17.
/// Vendor-documented, not live-measured. The GLM Coding Plan has a different endpoint (see the
/// module docs); an operator reaches it, or any gateway, with `ZAI_BASE_URL` or
/// [`ZaiConfig::with_base_url`].
pub const ZAI_DEFAULT_BASE_URL: &str = "https://api.z.ai/api/paas/v4";

/// Default Z.ai model requested when `ZAI_MODEL` is unset.
///
/// `glm-5.3`: the model every request example on the chat-completion reference uses, listed by
/// the model overview as the current flagship and documented as text-only (sources:
/// <https://docs.z.ai/api-reference/llm/chat-completion>,
/// <https://docs.z.ai/guides/overview/overview>, <https://docs.z.ai/guides/llm/glm-5.3>, all
/// accessed 2026-11-17). It reasons by default (the vendor's thinking default is enabled), which
/// is why the compiled timeout is the reasoning-model one. Vendor-documented, not live-measured;
/// no seed probe exists, so it runs on the adapter-wide fallback profile like every other model.
pub const ZAI_DEFAULT_MODEL: &str = "glm-5.3";

/// Curated fallback model list, returned when the live `/models` request fails, is unreachable,
/// or returns an empty list (the vendor documents no listing endpoint at all, so this is the
/// usual answer). Never reported as authoritative -- see
/// [`crate::compat::engine::CompatEngine::available_models`].
///
/// Source: the model tables of <https://docs.z.ai/guides/overview/overview> and the text-model
/// `enum` of <https://docs.z.ai/api-reference/llm/chat-completion>, accessed 2026-11-17: the
/// first four current, text-only, general-purpose ids in page order. The first entry is
/// [`ZAI_DEFAULT_MODEL`]. Vision, Flash-tier and legal variants are excluded as not text-only.
pub const ZAI_FALLBACK_MODELS: &[&str] = &[ZAI_DEFAULT_MODEL, "glm-5.2", "glm-5.1", "glm-5"];

/// The default request timeout in seconds: the reasoning-model value, because the vendor
/// documents thinking as enabled by default for [`ZAI_DEFAULT_MODEL`] (fact F10, source
/// <https://docs.z.ai/guides/llm/glm-5.3>, accessed 2026-11-17).
const ZAI_DEFAULT_TIMEOUT_SECONDS: u64 = REASONING_DEFAULT_TIMEOUT_SECONDS;

/// Configuration for the Z.ai adapter.
#[derive(Clone)]
#[non_exhaustive]
pub struct ZaiConfig {
    /// API key for Z.ai authentication.
    pub api_key: String,
    /// Base URL for the Z.ai chat API.
    pub base_url: String,
    /// The adapter's default model, which selects the timeout default; each request's own
    /// `LlmRequest.model` decides what is sent on the wire.
    pub model: String,
    /// Request timeout in seconds.
    pub timeout_seconds: u64,
    /// Default output budget (`max_tokens`) sent when a request sets none. `None` (the default)
    /// sends no `max_tokens` key. The value is checked against the model's profiled maximum
    /// output exactly as a request's own `max_tokens` is (ADR-0061 (i)): above it, the request is
    /// refused before any HTTP call.
    pub max_tokens: Option<u32>,
    /// How many times a retryable error is retried after the first attempt. Defaults to `3`;
    /// `0` sends each request once. A 429 is never retried here (the Cadence decorator waits)
    /// and a permanent error is never retried.
    pub max_retries: u32,
    /// Adapter-wide context window in tokens, the operator's statement for every model of this
    /// provider. `None` (the default) keeps the compiled windows, which for Z.ai are none: the
    /// seeded table is empty. When set it replaces the adapter's declared window and the
    /// fallback profile's window; a `context_window` in [`model_overrides`](Self::model_overrides)
    /// still wins for that model. Construction fails when it is below a model's maximum output.
    pub context_window: Option<u32>,
    /// Per-model overrides, keyed by exact model id and laid over each model's profile field by
    /// field. Empty by default. Validated at construction; an invalid override fails it with an
    /// error naming the provider and the model.
    pub model_overrides: BTreeMap<String, ModelOverride>,
    /// The provider-level default reasoning effort: sent for a request that sets none of its
    /// own, resolved per request against that request's own model. A request's (or an agent's)
    /// own effort always wins. `None` (the default, and the only compiled-in value) sends
    /// nothing. Z.ai declares no reasoning wire, so any `Some` value is refused before any HTTP
    /// call. Set with [`with_reasoning_effort`](Self::with_reasoning_effort).
    pub reasoning_effort: Option<ReasoningEffort>,
    /// The provider-level default fallback applied when the effort in force is not one the
    /// request's model accepts; a request's own fallback wins. `None` (the default) means
    /// refuse. Set with [`with_reasoning_fallback`](Self::with_reasoning_fallback).
    pub reasoning_fallback: Option<ReasoningFallback>,
}

/// Prints every field except the API key, which is shown as `<redacted>`: the key is a public
/// `String`, so a derived `Debug` would put it into any log line or panic message that formats
/// the config.
impl std::fmt::Debug for ZaiConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ZaiConfig")
            .field("api_key", &RedactedKey(&self.api_key))
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("timeout_seconds", &self.timeout_seconds)
            .field("max_tokens", &self.max_tokens)
            .field("max_retries", &self.max_retries)
            .field("context_window", &self.context_window)
            .field("model_overrides", &self.model_overrides)
            .field("reasoning_effort", &self.reasoning_effort)
            .field("reasoning_fallback", &self.reasoning_fallback)
            .finish()
    }
}

impl ZaiConfig {
    /// Load configuration from environment variables.
    ///
    /// # Environment Variables
    /// - `ZAI_API_KEY` (required): Z.ai API key.
    /// - `ZAI_BASE_URL` (optional): API base URL, defaults to [`ZAI_DEFAULT_BASE_URL`].
    /// - `ZAI_MODEL` (optional): Default model, defaults to [`ZAI_DEFAULT_MODEL`].
    /// - `ZAI_TIMEOUT_SECONDS` (optional): Request timeout, defaults to `300` (the
    ///   reasoning-model default; see the module docs, fact F10).
    ///
    /// # Errors
    /// Returns an error if `ZAI_API_KEY` is absent, or another variable's value fails to parse
    /// or validate.
    pub fn from_env() -> Result<Self, String> {
        Self::from_parts(
            env::var("ZAI_API_KEY").ok(),
            env::var("ZAI_BASE_URL").ok(),
            env::var("ZAI_MODEL").ok(),
            env::var("ZAI_TIMEOUT_SECONDS").ok(),
        )
    }

    /// The pure defaulting/validation logic behind [`Self::from_env`], separated out so it is
    /// testable without mutating process environment variables -- `std::env::set_var` is
    /// `unsafe` under Rust 2024 and this crate denies `unsafe_code`.
    fn from_parts(
        api_key: Option<String>,
        base_url: Option<String>,
        model: Option<String>,
        timeout_seconds: Option<String>,
    ) -> Result<Self, String> {
        let api_key =
            api_key.ok_or_else(|| "ZAI_API_KEY environment variable not set".to_string())?;
        let base_url = base_url.unwrap_or_else(|| ZAI_DEFAULT_BASE_URL.to_string());
        let model = model.unwrap_or_else(|| ZAI_DEFAULT_MODEL.to_string());
        let timeout_seconds = match timeout_seconds {
            Some(value) => value
                .parse()
                .map_err(|_| "Invalid ZAI_TIMEOUT_SECONDS value".to_string())?,
            None => default_timeout_seconds(ZAI_PROVIDER, &model, ZAI_DEFAULT_TIMEOUT_SECONDS),
        };

        let config = Self {
            api_key,
            base_url,
            model,
            timeout_seconds,
            max_tokens: None,
            max_retries: DEFAULT_MAX_RETRIES,
            context_window: None,
            model_overrides: BTreeMap::new(),
            reasoning_effort: None,
            reasoning_fallback: None,
        };

        config.validate()?;
        Ok(config)
    }

    /// Create a configuration for `model`, authenticating with `api_key`.
    ///
    /// - `api_key`: the Z.ai API key, never a URL.
    /// - `model`: the adapter's default model. It selects the timeout default; each request's own
    ///   `LlmRequest.model` decides what is sent on the wire.
    ///
    /// The endpoint is [`ZAI_DEFAULT_BASE_URL`] (replace it with
    /// [`with_base_url`](Self::with_base_url)) and the timeout is 300 s (replace it with
    /// [`with_timeout_seconds`](Self::with_timeout_seconds)). Construction is infallible: an
    /// empty key or model, or a non-`http(s)` endpoint, is refused when the adapter is built.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_llm::zai::ZaiConfig;
    /// use paladin_llm::zai::adapter::{ZAI_DEFAULT_BASE_URL, ZAI_DEFAULT_MODEL};
    ///
    /// let key = "zai-example-key".to_string();
    /// let config = ZaiConfig::new(key, ZAI_DEFAULT_MODEL.to_string()).with_timeout_seconds(600);
    /// assert_eq!(config.base_url, ZAI_DEFAULT_BASE_URL);
    /// assert_eq!(config.timeout_seconds, 600);
    /// ```
    pub fn new(api_key: String, model: String) -> Self {
        let timeout_seconds =
            default_timeout_seconds(ZAI_PROVIDER, &model, ZAI_DEFAULT_TIMEOUT_SECONDS);
        Self {
            api_key,
            base_url: ZAI_DEFAULT_BASE_URL.to_string(),
            model,
            timeout_seconds,
            max_tokens: None,
            max_retries: DEFAULT_MAX_RETRIES,
            context_window: None,
            model_overrides: BTreeMap::new(),
            reasoning_effort: None,
            reasoning_fallback: None,
        }
    }

    /// Validate the configuration.
    ///
    /// # Errors
    /// Returns a message (never containing the key) when the key, base URL or model is empty, or
    /// the base URL is not an `http://` or `https://` URL.
    pub fn validate(&self) -> Result<(), String> {
        if self.api_key.is_empty() {
            return Err("API key cannot be empty".to_string());
        }
        if self.base_url.is_empty() {
            return Err("Base URL cannot be empty".to_string());
        }
        if !is_http_url(&self.base_url) {
            return Err("Base URL must start with http:// or https://".to_string());
        }
        if self.model.is_empty() {
            return Err("Model name cannot be empty".to_string());
        }
        Ok(())
    }
}

config_knob_builders!(ZaiConfig);

/// The provider name this adapter reports through [`LlmPort::get_provider_name`] and that its
/// engine stamps on every `LlmError::ProviderError`.
const ZAI_PROVIDER: &str = "zai";

/// Z.ai LLM Adapter implementing [`LlmPort`].
///
/// Every method delegates to an owned [`CompatEngine`] -- this struct carries no protocol logic
/// of its own.
pub struct ZaiAdapter {
    engine: CompatEngine,
}

impl ZaiAdapter {
    /// Create a new Z.ai adapter.
    ///
    /// # Errors
    /// Returns an error if configuration is invalid or the underlying HTTP client cannot be
    /// created.
    pub fn new(config: ZaiConfig) -> Result<Self, LlmError> {
        config.validate().map_err(|e| {
            LlmError::AuthenticationError(format!("Invalid Z.ai configuration: {e}"))
        })?;

        let engine_config = CompatEngineConfig {
            base_url: config.base_url,
            api_key: config.api_key,
            model: config.model,
            timeout_seconds: config.timeout_seconds,
            max_retries: config.max_retries,
            capabilities: CompatCapabilities {
                supports_streaming: true,
                // The vendor documents tool calls, but the engine request has no tools field, so
                // this adapter reports what its own request path implements.
                supports_tool_calling: false,
                supports_function_calling: false,
                // The Flash tier is multimodal, but this is the text adapter: it reports what its
                // own request path implements, not what the vendor's product line advertises.
                supports_vision: false,
                supports_embeddings: false,
                // Not verified: no model is seeded (the table is empty), so this is `None` unless
                // the operator states a window. The vendor documents a 1M window for the default
                // model (https://docs.z.ai/guides/llm/glm-5.3, accessed 2026-11-17), but this
                // crate seeds nothing from documentation alone.
                max_context_tokens: config
                    .context_window
                    .or(seeded_window(seeded::ZAI, ZAI_DEFAULT_MODEL)),
                supports_system_messages: true,
                // F7: the inclusive range `[0.0, 1.0]` stated by
                // https://docs.z.ai/api-reference/llm/chat-completion, accessed 2026-11-17.
                // Vendor-documented, not live-measured.
                temperature_range: Some((0.0, 1.0)),
            },
            // F8 (https://docs.z.ai/api-reference/llm/chat-completion, accessed 2026-11-17):
            // `temperature`, `max_tokens` and `top_p` are documented request fields. Neither
            // `frequency_penalty` nor `presence_penalty` appears in the documented schema, so
            // both are `false` (not verified: the vendor may tolerate them, but a field it does
            // not document is not sent). Vendor-documented, not live-measured.
            //
            // Reasoning is `Unsupported` (F9): the documented `thinking` object and
            // `reasoning_effort` field are candidates only. Per ADR-0061/ADR-0062 and Phase 51
            // D-04 a wire ships only on a live effort probe, and none exists for Z.ai, so nothing
            // is sent and every explicit effort is refused before any HTTP call.
            request_parameters: CompatRequestParameters {
                temperature: true,
                max_tokens: true,
                top_p: true,
                frequency_penalty: false,
                presence_penalty: false,
                reasoning: ReasoningWire::Unsupported,
            },
            fallback_models: ZAI_FALLBACK_MODELS.iter().map(|s| s.to_string()).collect(),
            // No usage-cap (quota) mapping for Z.ai: although the vendor documents business
            // codes (https://docs.z.ai/api-reference/api-code, accessed 2026-11-17), no real
            // quota-exhaustion body was captured, so an override would be invented evidence
            // (the D-11a precedent). The shared status mapping applies.
            error_override: None,
            // WR-04: `ZAI_DEFAULT_BASE_URL` is only this preset's *default* -- `ZAI_BASE_URL` is
            // operator-settable (`ZaiConfig::from_env`), so a `3xx` from whatever host it
            // resolves to could otherwise replay the `Authorization` header carrying the
            // operator's credential to a different host. `Policy::none()` prevents that, matching
            // `openai_compatible`'s posture; a refused redirect surfaces via the engine's
            // `300..=399` `map_error` arm.
            redirect_policy: Some(reqwest::redirect::Policy::none()),
        };

        // The adapter-wide fallback profile is this preset's own declaration above, restated as
        // a profile; the seeded table is empty, so it applies to every model.
        // Stop sequences (F8): the vendor's prose says "currently, only one stop word is
        // supported" while the schema says `maxItems: 4`; the fail-closed reading is one.
        let fallback = fallback_from_compat(
            &engine_config.capabilities.clone().into(),
            &engine_config.request_parameters,
            StopSupport::Supported { max: Some(1) },
        );

        // The operator's adapter-wide window, then the per-model overrides, are laid over the
        // (empty) seeded table; either one failing construction names the provider and model.
        let registry = ModelRegistry::new(ZAI_PROVIDER, seeded::ZAI, fallback)
            .with_context_window(config.context_window)
            .and_then(|registry| registry.with_overrides(config.model_overrides))
            .map_err(|e| {
                LlmError::AuthenticationError(format!("Invalid Z.ai configuration: {e}"))
            })?;

        Ok(Self {
            // Name the engine so every `ProviderError` it emits carries "zai", not the engine's
            // generic default.
            engine: CompatEngine::new(engine_config)?
                .with_provider_name(ZAI_PROVIDER)
                .with_default_max_tokens(config.max_tokens)
                .with_default_reasoning(config.reasoning_effort, config.reasoning_fallback)
                // No `with_usage_semantics`: Z.ai keeps the engine default
                // `CompatUsageSemantics::ReasoningInsideCompletion`. NOT live-measured: no usage
                // probe exists for Z.ai. A mismatch with the vendor's own total surfaces at
                // runtime as `paladin.usage_unreconciled` (G-50-5a reconciliation).
                .with_model_registry(registry),
        })
    }
}

#[async_trait]
impl LlmPort for ZaiAdapter {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        self.engine.generate(request).await
    }

    async fn generate_stream(
        &self,
        request: LlmRequest,
    ) -> Result<Box<dyn Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError> {
        self.engine.generate_stream(request).await
    }

    async fn validate_model(&self, model: &str) -> Result<bool, LlmError> {
        Ok(self.engine.validate_model(model).await)
    }

    async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
        Ok(self.engine.available_models().await)
    }

    fn get_provider_name(&self) -> &'static str {
        ZAI_PROVIDER
    }

    fn get_capabilities(&self) -> ProviderCapabilities {
        self.engine.capabilities()
    }

    fn model_profile(&self, model: &str) -> Option<ModelProfile> {
        Some(self.engine.model_profile(model))
    }

    /// The pre-judging twin of the engine's send-time `in_force` + `admit`: this preset's masked
    /// profile and provider defaults, the same verdict.
    fn check_reasoning(
        &self,
        model: &str,
        effort: Option<ReasoningEffort>,
        fallback: Option<ReasoningFallback>,
    ) -> Result<(), LlmError> {
        self.engine.check_reasoning(model, effort, fallback)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http_status::map_http_status;
    use crate::registry;
    use mockito::Server;
    use paladin_core::platform::container::prompt::{
        PromptItem, PromptParameters, PromptType, UserPrompt,
    };
    use paladin_ports::output::llm_port::FinishReason;
    use serde_json::json;

    /// The `with_base_url` guarantee is an `http://` or `https://` URL. A value that merely starts
    /// with `http` (`httpx://x`, a scheme-less `http-gateway...`) must fail construction like any
    /// other non-HTTP value.
    #[test]
    fn a_builder_set_non_http_base_url_fails_zai_construction() {
        for bad in [
            "ftp://x",
            "",
            "api.example.com/v1",
            "httpx://x",
            "http-gateway.internal/v1",
            "httpfoo:",
        ] {
            let config =
                ZaiConfig::new("k".to_string(), ZAI_DEFAULT_MODEL.to_string()).with_base_url(bad);
            assert!(
                config.validate().is_err(),
                "base URL {bad:?} must fail validation"
            );
            assert!(
                ZaiAdapter::new(config).is_err(),
                "base URL {bad:?} must fail adapter construction"
            );
        }
        for good in ["http://localhost:1", "https://gateway.example/v1"] {
            let config =
                ZaiConfig::new("k".to_string(), ZAI_DEFAULT_MODEL.to_string()).with_base_url(good);
            assert!(
                config.validate().is_ok(),
                "base URL {good:?} must pass validation"
            );
        }
    }

    // ── ZaiConfig::new ──

    #[test]
    fn zai_config_new_takes_key_and_model_and_defaults_the_endpoint() {
        let config = ZaiConfig::new("k".to_string(), "glm-5.2".to_string());
        assert_eq!(config.base_url, ZAI_DEFAULT_BASE_URL);
        assert_eq!(config.model, "glm-5.2");
        assert_eq!(config.api_key, "k");
        assert_eq!(
            config.timeout_seconds,
            registry::REASONING_DEFAULT_TIMEOUT_SECONDS,
            "thinking is on by default (F10), so the reasoning-model timeout applies"
        );
    }

    #[test]
    fn zai_config_with_base_url_replaces_the_default_endpoint() {
        let config =
            ZaiConfig::new("k".to_string(), "m".to_string()).with_base_url("http://localhost:1234");
        assert_eq!(config.base_url, "http://localhost:1234");
    }

    fn build_request(model: &str) -> LlmRequest {
        LlmRequest::new(
            model,
            PromptItem::new(PromptType::User(UserPrompt {
                query: "Hello".to_string(),
                context: None,
            }))
            .unwrap(),
        )
    }

    // ── ZaiConfig::from_env() defaulting logic ──

    #[test]
    fn zai_config_from_env_errors_when_api_key_absent() {
        let result = ZaiConfig::from_parts(None, None, None, None);
        assert!(result.is_err());
    }

    #[test]
    fn zai_config_defaults_base_url_model_and_timeout_when_only_key_is_set() {
        let config = ZaiConfig::from_parts(Some("test-key".to_string()), None, None, None)
            .expect("key alone must be sufficient to build a valid config");
        assert_eq!(config.base_url, ZAI_DEFAULT_BASE_URL);
        assert_eq!(config.model, ZAI_DEFAULT_MODEL);
        assert_eq!(
            config.timeout_seconds,
            registry::REASONING_DEFAULT_TIMEOUT_SECONDS
        );
    }

    /// Pins the shipped defaults to the literals read on docs.z.ai (fact F1 and F4). Asserted
    /// against literals rather than the constants, so a drive-by edit of a constant fails here
    /// instead of passing silently.
    #[test]
    fn zai_config_defaults_to_the_documented_endpoint_and_model_by_literal() {
        let config = ZaiConfig::from_parts(Some("test-key".to_string()), None, None, None)
            .expect("key alone must be sufficient to build a valid config");
        assert_eq!(config.base_url, "https://api.z.ai/api/paas/v4");
        assert_eq!(config.model, "glm-5.3");
        assert_eq!(
            ZAI_FALLBACK_MODELS,
            ["glm-5.3", "glm-5.2", "glm-5.1", "glm-5"]
        );
    }

    #[test]
    fn zai_fallback_list_leads_with_the_default_model() {
        assert_eq!(ZAI_FALLBACK_MODELS[0], ZAI_DEFAULT_MODEL);
        assert!((1..=4).contains(&ZAI_FALLBACK_MODELS.len()));
    }

    #[test]
    fn zai_config_honors_zai_base_url_model_and_timeout_override() {
        let config = ZaiConfig::from_parts(
            Some("test-key".to_string()),
            Some("https://override.example/v1".to_string()),
            Some("glm-5.1".to_string()),
            Some("30".to_string()),
        )
        .unwrap();
        assert_eq!(config.base_url, "https://override.example/v1");
        assert_eq!(config.model, "glm-5.1");
        assert_eq!(config.timeout_seconds, 30);
    }

    #[test]
    fn zai_config_rejects_a_non_numeric_timeout_without_echoing_the_value() {
        let error = ZaiConfig::from_parts(
            Some("test-key".to_string()),
            None,
            None,
            Some("sk-leaky".to_string()),
        )
        .unwrap_err();
        assert!(error.contains("ZAI_TIMEOUT_SECONDS"), "{error}");
        assert!(!error.contains("sk-leaky"), "{error}");
    }

    // ── Request shaping / response parsing ──

    #[tokio::test]
    async fn generate_posts_to_chat_completions_with_auth_header_and_matching_body() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/chat/completions")
            .match_header("authorization", "Bearer test-key")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "id": "cmpl-1",
                    "model": ZAI_DEFAULT_MODEL,
                    "choices": [{
                        "index": 0,
                        "message": {"role": "assistant", "content": "Hi there"},
                        "finish_reason": "stop"
                    }],
                    "usage": {"prompt_tokens": 5, "completion_tokens": 3, "total_tokens": 8}
                })
                .to_string(),
            )
            .create_async()
            .await;

        let config = ZaiConfig::new("test-key".to_string(), ZAI_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = ZaiAdapter::new(config).unwrap();

        let response = adapter
            .generate(build_request(ZAI_DEFAULT_MODEL))
            .await
            .expect("mock server returned a well-formed response");

        assert_eq!(response.content, "Hi there");
        assert!(matches!(response.finish_reason, FinishReason::Stop));
        assert_eq!(response.usage.prompt_tokens, 5);
        assert_eq!(response.usage.completion_tokens, 3);
        assert_eq!(response.usage.total_tokens, 8);

        mock.assert_async().await;
    }

    /// Fact F8: `stop` is documented (prose: one stop word supported; the schema's own cap is
    /// looser), so one stop sequence reaches the wire and a second is omitted, never sent.
    #[tokio::test]
    async fn zai_one_stop_sequence_reaches_the_wire_and_a_second_is_omitted() {
        let mut server = crate::compat::test_support::server().await;
        let captured = crate::compat::test_support::mock_completion(&mut server).await;
        let adapter = ZaiAdapter::new(
            ZaiConfig::new("test-key".to_string(), ZAI_DEFAULT_MODEL.to_string())
                .with_base_url(server.url()),
        )
        .unwrap();

        let mut request = build_request(ZAI_DEFAULT_MODEL);
        request.prompt.entity.data.parameters = PromptParameters {
            max_tokens: Some(64),
            temperature: Some(0.5),
            top_p: Some(0.9),
            frequency_penalty: None,
            presence_penalty: None,
            stop_sequences: Some(vec!["END".to_string()]),
        };
        let response = adapter
            .generate(request)
            .await
            .expect("well-formed response");
        let body = captured.lock().unwrap().take().expect("a request was sent");
        assert_eq!(body.get("stop"), Some(&json!(["END"])), "{body}");
        assert_eq!(body.get("top_p"), Some(&json!(0.9)), "{body}");
        assert!(
            !response
                .metadata
                .contains_key(crate::sampling::PARAMS_OMITTED_METADATA_KEY)
        );

        let mut two = build_request(ZAI_DEFAULT_MODEL);
        two.prompt.entity.data.parameters = PromptParameters {
            max_tokens: None,
            temperature: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop_sequences: Some(vec!["END".to_string(), "STOP".to_string()]),
        };
        let response = adapter.generate(two).await.expect("well-formed response");
        let body = captured.lock().unwrap().take().expect("a request was sent");
        assert!(body.get("stop").is_none(), "{body}");
        assert!(
            response
                .metadata
                .contains_key(crate::sampling::PARAMS_OMITTED_METADATA_KEY)
        );
    }

    /// Fact F7: the documented temperature range `[0.0, 1.0]` is the declared range on both the
    /// adapter and the (fallback) model profile.
    #[test]
    fn zai_declares_the_documented_temperature_range() {
        let adapter = ZaiAdapter::new(ZaiConfig::new(
            "test-key".to_string(),
            ZAI_DEFAULT_MODEL.to_string(),
        ))
        .unwrap();
        assert_eq!(
            adapter.get_capabilities().temperature_range,
            Some((0.0, 1.0))
        );
        let profile = adapter.model_profile(ZAI_DEFAULT_MODEL).unwrap();
        assert_eq!(
            profile.temperature,
            paladin_ports::output::model_profile::ParamSupport::Range(0.0, 1.0)
        );
        // Penalties are not in the documented request schema (F8): never sent.
        assert_eq!(
            profile.frequency_penalty,
            paladin_ports::output::model_profile::ParamSupport::NotSent
        );
        assert_eq!(
            profile.presence_penalty,
            paladin_ports::output::model_profile::ParamSupport::NotSent
        );
    }

    /// No operator live seed probe exists: the table is empty, so even the default model is
    /// unprofiled, visibly, on the profile and on every response.
    #[tokio::test]
    async fn zai_default_model_is_unprofiled_and_the_response_says_so() {
        let mut server = crate::compat::test_support::server().await;
        let _captured = crate::compat::test_support::mock_completion(&mut server).await;
        let adapter = ZaiAdapter::new(
            ZaiConfig::new("test-key".to_string(), ZAI_DEFAULT_MODEL.to_string())
                .with_base_url(server.url()),
        )
        .unwrap();

        let profile = adapter
            .model_profile(ZAI_DEFAULT_MODEL)
            .expect("zai profiles every model");
        assert_eq!(profile.verified_on, None);
        assert!(seeded::ZAI.is_empty());

        let response = adapter
            .generate(build_request(ZAI_DEFAULT_MODEL))
            .await
            .expect("well-formed response");
        assert_eq!(
            response
                .metadata
                .get(registry::UNPROFILED_MODEL_METADATA_KEY)
                .map(String::as_str),
            Some(ZAI_DEFAULT_MODEL),
            "{:?}",
            response.metadata
        );
    }

    // ── Streaming ──

    #[tokio::test]
    async fn generate_stream_assembles_deltas_in_wire_order_with_terminal_stop() {
        use futures::StreamExt;

        let sse_body = concat!(
            "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"Hel\"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"lo \"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"world\"},\"finish_reason\":\"stop\"}]}\n\n",
            "data: [DONE]\n\n",
        );

        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(sse_body)
            .create_async()
            .await;

        let config = ZaiConfig::new("test-key".to_string(), ZAI_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = ZaiAdapter::new(config).unwrap();

        let stream = adapter
            .generate_stream(build_request(ZAI_DEFAULT_MODEL))
            .await
            .unwrap();
        let mut stream = Box::into_pin(stream);

        let mut assembled = String::new();
        let mut last_finish_reason = None;
        while let Some(item) = stream.next().await {
            let chunk = item.unwrap();
            assembled.push_str(&chunk.delta);
            if chunk.finish_reason.is_some() {
                last_finish_reason = chunk.finish_reason;
            }
        }

        assert_eq!(assembled, "Hello world");
        assert!(matches!(last_finish_reason, Some(FinishReason::Stop)));
    }

    /// Fact F11: z.ai documents streamed usage as riding on the final chunk, the one that also
    /// carries `finish_reason`, not on a trailing empty-`choices` frame. Both shapes must reach
    /// the caller as usage.
    #[tokio::test]
    async fn generate_stream_reads_usage_from_the_finish_chunk() {
        use futures::StreamExt;

        let sse_body = concat!(
            "data: {\"id\":\"1\",\"created\":1,\"model\":\"glm-5.3\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hi\"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"1\",\"created\":1,\"model\":\"glm-5.3\",\"choices\":[{\"index\":0,\"finish_reason\":\"stop\",\"delta\":{\"role\":\"assistant\",\"content\":\"\"}}],\"usage\":{\"prompt_tokens\":8,\"completion_tokens\":262,\"total_tokens\":270,\"prompt_tokens_details\":{\"cached_tokens\":0}}}\n\n",
            "data: [DONE]\n\n",
        );
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(sse_body)
            .create_async()
            .await;
        let adapter = ZaiAdapter::new(
            ZaiConfig::new("test-key".to_string(), ZAI_DEFAULT_MODEL.to_string())
                .with_base_url(server.url()),
        )
        .unwrap();

        let stream = adapter
            .generate_stream(build_request(ZAI_DEFAULT_MODEL))
            .await
            .unwrap();
        let mut stream = Box::into_pin(stream);
        let mut usage = None;
        while let Some(item) = stream.next().await {
            let chunk = item.unwrap();
            if chunk.usage.is_some() {
                usage = chunk.usage;
            }
        }
        let usage = usage.expect("the finish chunk's usage object is surfaced");
        assert_eq!(usage.prompt_tokens, 8);
        assert_eq!(usage.completion_tokens, 262);
        assert_eq!(usage.total_tokens, 270);
    }

    // ── Error mapping ──

    #[tokio::test]
    async fn http_401_maps_to_authentication_error() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(401)
            .with_body(r#"{"error":{"code":"1001","message":"Authentication parameter not received in Header, unable to authenticate"}}"#)
            .create_async()
            .await;

        let config = ZaiConfig::new("test-key".to_string(), ZAI_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = ZaiAdapter::new(config).unwrap();

        let result = adapter.generate(build_request(ZAI_DEFAULT_MODEL)).await;
        assert!(matches!(result, Err(LlmError::AuthenticationError(_))));
    }

    /// A 429 is surfaced on the first response: the adapter never retries it (the Cadence
    /// decorator owns pacing).
    #[tokio::test]
    async fn http_429_maps_to_rate_limit_exceeded_without_a_retry() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/chat/completions")
            .with_status(429)
            .with_body(r#"{"error":{"code":"1302","message":"Rate limit reached for requests"}}"#)
            .expect(1)
            .create_async()
            .await;

        let config = ZaiConfig::new("test-key".to_string(), ZAI_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = ZaiAdapter::new(config).unwrap();

        let result = adapter.generate(build_request(ZAI_DEFAULT_MODEL)).await;
        assert!(matches!(result, Err(LlmError::RateLimitExceeded { .. })));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn http_404_maps_to_model_not_available() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(404)
            .with_body(r#"{"error":"no such model"}"#)
            .create_async()
            .await;

        let config = ZaiConfig::new("test-key".to_string(), ZAI_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = ZaiAdapter::new(config).unwrap();

        let result = adapter.generate(build_request(ZAI_DEFAULT_MODEL)).await;
        assert!(matches!(result, Err(LlmError::ModelNotAvailable(_))));
    }

    #[tokio::test]
    async fn http_400_maps_to_invalid_prompt() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(400)
            .with_body(r#"{"error":{"code":"1214","message":"Parameter is invalid."}}"#)
            .create_async()
            .await;

        let config = ZaiConfig::new("test-key".to_string(), ZAI_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = ZaiAdapter::new(config).unwrap();

        let result = adapter.generate(build_request(ZAI_DEFAULT_MODEL)).await;
        assert!(matches!(result, Err(LlmError::InvalidPrompt(_))));
    }

    /// The preset's non-2xx path is the shared `map_http_status`, proven by comparing the
    /// adapter's error against the helper's own output for the same status, body and key.
    #[tokio::test]
    async fn zai_non_2xx_routes_through_the_shared_mapper() {
        let body =
            r#"{"error":{"code":"1305","message":"The service may be temporarily overloaded"}}"#;
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(503)
            .with_body(body)
            .expect_at_least(1)
            .create_async()
            .await;

        let config = ZaiConfig::new("test-key".to_string(), ZAI_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = ZaiAdapter::new(config).unwrap();

        let result = adapter.generate(build_request(ZAI_DEFAULT_MODEL)).await;
        let expected = map_http_status("zai", 503, body, "test-key");
        match (result, expected) {
            (
                Err(LlmError::ProviderError {
                    provider,
                    status,
                    message,
                }),
                LlmError::ProviderError {
                    provider: want_provider,
                    status: want_status,
                    message: want_message,
                },
            ) => {
                assert_eq!(provider, want_provider);
                assert_eq!(status, want_status);
                assert_eq!(message, want_message);
            }
            (other, _) => panic!("expected ProviderError {{ status: 503 }}, got {other:?}"),
        }
    }

    /// T-zai-02: a non-2xx body that echoes the API key yields an error whose Display and Debug
    /// do not contain the key (the engine redacts before it bounds).
    #[tokio::test]
    async fn a_non_2xx_body_that_echoes_the_key_does_not_leak_it() {
        const KEY: &str = "zai-secret-key-0123456789";
        for status in [400_u16, 401, 403, 500] {
            let mut server = Server::new_async().await;
            server
                .mock("POST", "/chat/completions")
                .with_status(status as usize)
                .with_body(format!(
                    r#"{{"error":{{"code":"1000","message":"bad credential {KEY} for request"}}}}"#
                ))
                .expect_at_least(1)
                .create_async()
                .await;
            let config = ZaiConfig::new(KEY.to_string(), ZAI_DEFAULT_MODEL.to_string())
                .with_base_url(server.url())
                .with_max_retries(0);
            let adapter = ZaiAdapter::new(config).unwrap();

            let error = adapter
                .generate(build_request(ZAI_DEFAULT_MODEL))
                .await
                .expect_err("a non-2xx response is an error");
            let display = error.to_string();
            let debug = format!("{error:?}");
            assert!(!display.contains(KEY), "status {status}: {display}");
            assert!(!debug.contains(KEY), "status {status}: {debug}");
        }
    }

    /// T-zai-03 (WR-04): a 3xx is never followed, so the `Authorization` header cannot be
    /// replayed to the redirect target.
    #[tokio::test]
    async fn a_redirect_is_not_followed_and_the_second_host_is_never_contacted() {
        let mut elsewhere = Server::new_async().await;
        let second = elsewhere
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body("{}")
            .expect(0)
            .create_async()
            .await;

        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(307)
            .with_header("location", &format!("{}/chat/completions", elsewhere.url()))
            .expect_at_least(1)
            .create_async()
            .await;

        let config = ZaiConfig::new("test-key".to_string(), ZAI_DEFAULT_MODEL.to_string())
            .with_base_url(server.url())
            .with_max_retries(0);
        let adapter = ZaiAdapter::new(config).unwrap();

        let result = adapter.generate(build_request(ZAI_DEFAULT_MODEL)).await;
        assert!(result.is_err(), "a 307 must surface as an error");
        second.assert_async().await;
    }

    // ── Capabilities / identity ──

    #[test]
    fn get_provider_name_returns_zai() {
        let config = ZaiConfig::new("test-key".to_string(), ZAI_DEFAULT_MODEL.to_string());
        let adapter = ZaiAdapter::new(config).unwrap();
        assert_eq!(adapter.get_provider_name(), "zai");
    }

    #[test]
    fn get_capabilities_reports_no_vision_and_no_tool_or_function_calling() {
        let config = ZaiConfig::new("test-key".to_string(), ZAI_DEFAULT_MODEL.to_string());
        let adapter = ZaiAdapter::new(config).unwrap();
        let caps = adapter.get_capabilities();

        assert!(!caps.supports_tool_calling);
        assert!(!caps.supports_function_calling);
        assert!(!caps.supports_vision);
        assert!(!caps.supports_embeddings);
        assert!(caps.supports_streaming);
        assert!(caps.supports_system_messages);
    }

    // ── Shared conformance suite ──
    //
    // Z.ai delegates every method to `CompatEngine`, so it speaks the identical OpenAI-compatible
    // wire shape the other presets instantiate the suite against; bodies below mirror those
    // fixtures' shape (distinct model/provider names only).
    mod conformance_suite {
        use super::*;
        use std::sync::Arc;

        struct ZaiFixture;

        impl crate::conformance::ConformanceFixture for ZaiFixture {
            /// Z.ai ships no reasoning wire (`ReasoningWire::Unsupported`), so no level is ever
            /// written to a body.
            fn effort_wire_value(_body: &serde_json::Value) -> Option<String> {
                None
            }

            const WIRE: crate::conformance::Wire = crate::conformance::Wire::OpenAiChat;

            fn default_model() -> &'static str {
                ZAI_DEFAULT_MODEL
            }

            fn provider_name() -> &'static str {
                "zai"
            }

            fn adapter(base_url: &str) -> Arc<dyn LlmPort> {
                let config = ZaiConfig::new("test-key".to_string(), ZAI_DEFAULT_MODEL.to_string())
                    .with_base_url(base_url);
                Arc::new(ZaiAdapter::new(config).expect("test config must build"))
            }

            fn success_body() -> String {
                json!({
                    "id": "cmpl-1",
                    "model": ZAI_DEFAULT_MODEL,
                    "choices": [{
                        "index": 0,
                        "message": {"role": "assistant", "content": "Hi there"},
                        "finish_reason": "stop"
                    }],
                    "usage": {"prompt_tokens": 5, "completion_tokens": 3, "total_tokens": 8}
                })
                .to_string()
            }

            fn stream_body() -> String {
                // The trailing empty-`choices` usage frame carries the SAME figures as
                // `success_body()` above -- the shared parity case asserts equality.
                concat!(
                    "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"Hel\"},\"finish_reason\":null}]}\n\n",
                    "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"lo \"},\"finish_reason\":null}]}\n\n",
                    "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"world\"},\"finish_reason\":\"stop\"}]}\n\n",
                    "data: {\"id\":\"1\",\"choices\":[],\"usage\":{\"prompt_tokens\":5,\"completion_tokens\":3,\"total_tokens\":8}}\n\n",
                    "data: [DONE]\n\n",
                )
                .to_string()
            }

            fn error_body(status: u16) -> String {
                json!({"error": format!("mock error for status {status}")}).to_string()
            }
        }

        crate::llm_conformance_suite!(ZaiFixture);
    }

    // ── PARITY-03 knobs ──

    const UNSEEDED: &str = "zai-unseeded-test-model";

    fn knob_config(url: &str, model: &str) -> ZaiConfig {
        ZaiConfig::new("test-key".to_string(), model.to_string()).with_base_url(url)
    }

    #[tokio::test]
    async fn zai_max_tokens_default_reaches_the_wire() {
        use crate::compat::test_support::{mock_completion, send_with_max_tokens, server};

        let mut server = server().await;
        let captured = mock_completion(&mut server).await;
        let adapter =
            ZaiAdapter::new(knob_config(&server.url(), UNSEEDED).with_max_tokens(2048)).unwrap();

        let body = send_with_max_tokens(&adapter, &captured, UNSEEDED, None)
            .await
            .unwrap();
        assert_eq!(body["max_tokens"], 2048, "{body}");
        let body = send_with_max_tokens(&adapter, &captured, UNSEEDED, Some(512))
            .await
            .unwrap();
        assert_eq!(body["max_tokens"], 512, "{body}");

        let plain = ZaiAdapter::new(knob_config(&server.url(), UNSEEDED)).unwrap();
        let body = send_with_max_tokens(&plain, &captured, UNSEEDED, None)
            .await
            .unwrap();
        assert!(body.get("max_tokens").is_none(), "{body}");
    }

    #[tokio::test]
    async fn zai_max_retries_is_read_by_the_engine() {
        for retries in [0_u32, 1] {
            let server = crate::compat::test_support::server().await;
            let adapter =
                ZaiAdapter::new(knob_config(&server.url(), UNSEEDED).with_max_retries(retries))
                    .unwrap();
            crate::compat::test_support::attempts_against_a_failing_endpoint(
                server,
                &adapter,
                UNSEEDED,
                retries as usize + 1,
            )
            .await;
        }
    }

    #[test]
    fn zai_context_window_override_is_declared() {
        let adapter = ZaiAdapter::new(
            knob_config("https://example.invalid/v1", UNSEEDED).with_context_window(500_000),
        )
        .unwrap();
        assert_eq!(adapter.get_capabilities().max_context_tokens, Some(500_000));
        let profile = adapter.model_profile(UNSEEDED).unwrap();
        assert_eq!(profile.context_window, Some(500_000));
        // Nothing is declared until the operator states a window: the table is empty.
        let plain = ZaiAdapter::new(knob_config("https://example.invalid/v1", UNSEEDED)).unwrap();
        assert_eq!(plain.get_capabilities().max_context_tokens, None);
    }

    #[test]
    fn zai_model_override_reaches_the_profile_and_invalid_one_fails_construction() {
        let with_override = |window: u32| {
            knob_config("https://example.invalid/v1", UNSEEDED).with_model_overrides(
                std::collections::BTreeMap::from([(
                    UNSEEDED.to_string(),
                    crate::registry::ModelOverride {
                        context_window: Some(window),
                        ..crate::registry::ModelOverride::default()
                    },
                )]),
            )
        };
        let adapter = ZaiAdapter::new(with_override(900_000)).unwrap();
        assert_eq!(
            adapter.model_profile(UNSEEDED).unwrap().context_window,
            Some(900_000)
        );
        let message = match ZaiAdapter::new(with_override(0)) {
            Err(error) => error.to_string(),
            Ok(_) => panic!("a zero window must fail construction"),
        };
        assert!(
            message.contains("zai") && message.contains(UNSEEDED),
            "{message}"
        );
    }

    #[test]
    fn zai_context_window_below_a_max_output_override_fails_construction() {
        // The table is empty, so the contradiction is built from an operator override: a
        // 100,000-token window against a 131,072-token maximum output is refused, never clamped.
        let config = knob_config("https://example.invalid/v1", UNSEEDED)
            .with_context_window(100_000)
            .with_model_overrides(std::collections::BTreeMap::from([(
                UNSEEDED.to_string(),
                crate::registry::ModelOverride {
                    max_output_tokens: Some(131_072),
                    ..crate::registry::ModelOverride::default()
                },
            )]));
        let message = match ZaiAdapter::new(config) {
            Err(error) => error.to_string(),
            Ok(_) => panic!("a window below a maximum output must fail construction"),
        };
        for needle in ["zai", UNSEEDED, "100000", "131072"] {
            assert!(
                message.contains(needle),
                "'{needle}' missing from: {message}"
            );
        }
    }

    /// T-zai-01: the config carries the key, so its Debug output shows `<redacted>` and never
    /// the key.
    #[test]
    fn zai_config_debug_redacts_the_api_key() {
        let config = knob_config("https://api.example.invalid/v1", "m");
        let debug = format!("{config:?}");
        assert!(!debug.contains("test-key"), "{debug}");
        assert!(debug.contains("<redacted>"), "{debug}");
        for needle in [
            "https://api.example.invalid/v1",
            "timeout_seconds",
            "max_retries",
        ] {
            assert!(debug.contains(needle), "'{needle}' missing from {debug}");
        }
    }

    // ── Reasoning: no wire ships without a live effort probe ──

    #[test]
    fn zai_declares_no_reasoning_wire() {
        let adapter = ZaiAdapter::new(ZaiConfig::new(
            "k".to_string(),
            ZAI_DEFAULT_MODEL.to_string(),
        ))
        .unwrap();
        assert_eq!(
            adapter.engine.request_parameters().reasoning,
            ReasoningWire::Unsupported
        );
    }

    /// The compiled `ReasoningWire::Unsupported` refuses an explicit effort before any HTTP call
    /// (mock hit count 0), and `check_reasoning` pre-judges the same verdict.
    #[tokio::test]
    async fn zai_refuses_an_explicit_reasoning_effort_before_any_http_call() {
        use crate::compat::test_support::agreed_verdict;
        use paladin_ports::output::llm_port::ReasoningEffort::High;

        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body("{}")
            .expect(0)
            .create_async()
            .await;
        let adapter = ZaiAdapter::new(knob_config(&server.url(), ZAI_DEFAULT_MODEL)).unwrap();

        let reason = agreed_verdict(&adapter, ZAI_DEFAULT_MODEL, Some(High), None)
            .await
            .expect("an unsupported wire refuses an explicit effort");
        assert!(!reason.is_empty());
        mock.assert_async().await;

        // No effort asked, nothing refused (the verdict path does send a request here, so the
        // mock must now answer one).
        let mut server = crate::compat::test_support::server().await;
        let _captured = crate::compat::test_support::mock_completion(&mut server).await;
        let adapter = ZaiAdapter::new(knob_config(&server.url(), ZAI_DEFAULT_MODEL)).unwrap();
        assert_eq!(
            agreed_verdict(&adapter, ZAI_DEFAULT_MODEL, None, None).await,
            None
        );
    }
}
