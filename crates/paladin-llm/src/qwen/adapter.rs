//! Qwen (Alibaba DashScope compatible-mode) LLM Adapter.
//!
//! A thin preset (D-01, D-05, D-12) sitting entirely on
//! [`crate::compat::CompatEngine`] — the shared OpenAI-compatible protocol
//! engine. This adapter supplies only Qwen's `base_url`, credential env var,
//! default model, curated fallback model list and capabilities block; every
//! other behaviour (request shaping, retry, streaming, error mapping,
//! credential redaction, memoized model-list resolution) is inherited from
//! the engine unchanged.
//!
//! ## Region default
//!
//! DashScope publishes at least five regional base URLs. The default here is
//! the Singapore (international) compatible-mode endpoint
//! (`https://dashscope-intl.aliyuncs.com/compatible-mode/v1`), per the
//! developer's binding decision of 2026-08-23 — see "Reversal record" below
//! for the two moves this constant has made, and why neither move is a
//! universally correct answer. An operator in any other region reaches
//! their endpoint with `DASHSCOPE_BASE_URL` and no code change — the
//! identical override pattern `DEEPSEEK_BASE_URL` and `ANTHROPIC_BASE_URL`
//! already establish (17-CONTEXT.md D-12; 17-03-PLAN.md "Decisions resolved
//! in this plan").
//!
//! **The override is not optional tuning for every operator — for some it is
//! mandatory.** Alibaba documents that a Base URL must be used together with
//! an API key from the same billing plan, and that **API keys are
//! independent across regions and cannot be used across regions**
//! (alibabacloud.com/help/en/model-studio, Base URL / error-code docs). The
//! three known compatible-mode regional endpoints:
//!
//! | Region | Compatible-mode base URL |
//! |---|---|
//! | Singapore — shipped default | `https://dashscope-intl.aliyuncs.com/compatible-mode/v1` |
//! | US (Virginia) | `https://dashscope-us.aliyuncs.com/compatible-mode/v1` |
//! | China (mainland) | `https://dashscope.aliyuncs.com/compatible-mode/v1` |
//!
//! **If your Alibaba Model Studio workspace is in the US or on the
//! mainland, you MUST set `DASHSCOPE_BASE_URL` to the matching row above.**
//! Presenting a US- or mainland-scoped key to the Singapore endpoint (or
//! vice versa) is not tuning, it is a required step for that credential to
//! reach its own account.
//!
//! What a region mismatch looks like **today** (plan 17-22, closing
//! G-17-4d's diagnosability half): a `warn`-level log line naming the
//! endpoint that rejected the request, alongside a model list that is
//! [`QWEN_FALLBACK_MODELS`] rather than the vendor's own catalog —
//! [`crate::compat::engine::CompatEngine::available_models`] reads its own
//! error classification and raises exactly this failure mode above the
//! `debug` level an offline vendor stays at (see that method's rustdoc for
//! the classification). The remedy is unchanged: point `DASHSCOPE_BASE_URL`
//! at the row above matching your own workspace's region.
//!
//! ### Reversal record — read before touching `QWEN_DEFAULT_BASE_URL` again
//!
//! This constant has moved twice. Neither move means what it might look like
//! from the outside, and reading either one as "the region question is now
//! settled" is exactly the mistake this record exists to prevent.
//!
//! **Move 1 (2026-08-22): Singapore → US (Virginia).** The original Singapore
//! default was defended by a prohibition against changing it, resting on the
//! argument that a `401` carrying Alibaba's documented `invalid_api_key`
//! envelope — returned by both the Singapore and mainland endpoints — proved
//! the URL resolved correctly and only the credential was bad. **That
//! argument was wrong and MUST NOT be reinstated or re-derived.** A
//! region-scoped key produces that exact well-formed `401` from *every*
//! endpoint except its own — the signature is identical for a right URL with
//! a wrong key and a wrong URL with a right key. What settled it: two runs of
//! the same binary with the same credential, differing only in the endpoint
//! (2026-08-22) — `dashscope-intl` (Singapore, the then-shipped default)
//! returned 3 models byte-identical to [`QWEN_FALLBACK_MODELS`] (the live
//! fetch silently failed), while `dashscope-us` (Virginia) returned 92 live
//! models. That credential was Virginia-scoped; it was valid the whole time,
//! and the URL was the defect.
//!
//! **Move 2 (2026-08-23): US (Virginia) → Singapore, again.** One day later
//! the operator's `DASHSCOPE_API_KEY` was replaced with a **Singapore-scoped**
//! key. Against that new credential, `dashscope-us` (the default Move 1 had
//! just shipped) returned a well-formed `401`, and `dashscope-intl` returned
//! the live catalog (162 models, both [`QWEN_DEFAULT_MODEL`] and
//! `qwen3.7-plus` present) and served every measured request. **This is not
//! a rehabilitation of the original Singapore default, and it is not
//! evidence that Move 1 was wrong.** It is a second, independent
//! confirmation of the exact rule Move 1 established, now observed in the
//! opposite direction: a region-scoped key returns a well-formed `401` from
//! every endpoint but its own, whichever region that happens to be. Two
//! credentials, one day apart, produced opposite "correct" answers for this
//! one constant — which is the actual finding: **a single shipped default
//! was never testing the URL. It was only ever testing which region one
//! developer's workspace and credential happened to occupy that day.**
//!
//! **What actually makes this safe is not which endpoint this constant
//! currently names.** It is (a) the `DASHSCOPE_BASE_URL` override documented
//! above, naming all three known regional endpoints, and (b) the `warn`-level
//! diagnostic plan 17-22 added to
//! [`crate::compat::engine::CompatEngine::available_models`], which makes a
//! region mismatch audible instead of silent. Both were already in place
//! before Move 2, and Move 2 is the live proof: on 2026-08-23, with the
//! shipped default still naming Virginia and the new Singapore-scoped
//! credential already in the environment, that exact diagnostic fired
//! unprompted — naming `dashscope-us` as the rejecting endpoint and pointing
//! at a credential "scoped to a different account or region" as the usual
//! cause — on a live, unstaged failure this constant's own shipped value had
//! just created. `dashscope-intl` is named as the shipped default going
//! forward because it is Alibaba's international endpoint serving the
//! broadest non-mainland, non-US audience, not because it is universally
//! correct — no single value here ever will be. A later reader who
//! re-derives "a well-formed `401` proves the URL" meets this paragraph
//! first, from either direction.

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
    DEFAULT_MAX_RETRIES, ModelOverride, ModelRegistry, config_knob_builders,
    default_timeout_seconds, fallback_from_compat, is_http_url, seeded, seeded_window,
};

/// Default Qwen (Alibaba DashScope) compatible-mode API base URL — the
/// Singapore (international) endpoint.
///
/// Live-verified 2026-08-23, against a Singapore-scoped `DASHSCOPE_API_KEY`:
/// this endpoint returned a 162-model catalog including [`QWEN_DEFAULT_MODEL`]
/// and `qwen3.7-plus`, and served every measured `generate()` request. See
/// the module-level "Region default" and "Reversal record" docs above for
/// the two moves this constant has made and what an operator in another
/// region must do.
pub const QWEN_DEFAULT_BASE_URL: &str = "https://dashscope-intl.aliyuncs.com/compatible-mode/v1";

/// Default Qwen model requested when `DASHSCOPE_MODEL` is unset.
///
/// `qwen3.8-flash`: the operator-identified current general-purpose model (its cheapest current
/// tier), measured end to end by the 2026-10-15 live probe (`registry/seeded.rs`, `verified_on`
/// 2026-10-15). It replaces `qwen-plus`, which stays available and seeded; pin `DASHSCOPE_MODEL`
/// to keep it.
///
/// Trade-off, recorded: the previous reasoning for `qwen-plus` (2026-08-23) was that it is a
/// rolling alias Alibaba re-points while keeping the name stable, whereas a generation-pinned
/// name such as this one is a future retirement candidate (the same pattern that forced the
/// `moonshot-v1-8k` and `gemini-2.5-flash` refreshes). The default now follows the vendor's
/// current model; the seeded table's date is what tells an operator when it was last checked.
pub const QWEN_DEFAULT_MODEL: &str = "qwen3.8-flash";

/// Curated fallback model list (D-13), returned when the live `/models`
/// endpoint fails, is unreachable, or returns an empty list. Never reported
/// as authoritative — see [`crate::compat::engine::CompatEngine::available_models`].
///
/// Re-verified live 2026-08-23 at [`QWEN_DEFAULT_BASE_URL`] (162-model
/// catalog): all three entries are present, undated, general-purpose model
/// names (no `-vl`/`-audio`/`-image`/`-mt`/`-coder`/`-tts`/`-asr` specialised
/// suffix), and the first entry matches [`QWEN_DEFAULT_MODEL`].
pub const QWEN_FALLBACK_MODELS: &[&str] =
    &[QWEN_DEFAULT_MODEL, "qwen-plus", "qwen-turbo", "qwen3-max"];

/// Configuration for the Qwen (Alibaba DashScope) adapter.
#[derive(Clone)]
#[non_exhaustive]
pub struct QwenConfig {
    /// API key for Qwen (DashScope) authentication.
    pub api_key: String,
    /// Base URL for the Qwen compatible-mode API.
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
    /// provider. `None` (the default) keeps the compiled windows. When set it replaces the
    /// adapter's declared window, the fallback profile's window and every seeded model's window;
    /// a `context_window` in [`model_overrides`](Self::model_overrides) still wins for that
    /// model. Construction fails when it is below a model's maximum output.
    pub context_window: Option<u32>,
    /// Per-model overrides (D-06), keyed by exact model id and laid over each model's seeded
    /// entry field by field. Empty by default. Validated at construction; an invalid override
    /// fails it with an error naming the provider and the model.
    pub model_overrides: BTreeMap<String, ModelOverride>,
    /// The provider-level default reasoning effort (Phase 51, D-00e): sent for a request that
    /// sets none of its own, resolved per request against that request's own model. A request's
    /// (or an agent's) own effort always wins. `None` (the default, and the only compiled-in
    /// value, D-00i) sends nothing. Set it only where every model this provider serves supports
    /// it, or those requests are refused before any HTTP call naming this setting.
    /// Set with [`with_reasoning_effort`](Self::with_reasoning_effort).
    pub reasoning_effort: Option<ReasoningEffort>,
    /// The provider-level default fallback (Phase 51, D-05) applied when the effort in force is
    /// not one the request's model accepts; a request's own fallback wins. `None` (the default)
    /// means refuse. Set with [`with_reasoning_fallback`](Self::with_reasoning_fallback).
    pub reasoning_fallback: Option<ReasoningFallback>,
}

/// Prints every field except the API key, which is shown as `<redacted>` (D-00a): the key is a
/// public `String`, so a derived `Debug` would put it into any log line or panic message that
/// formats the config.
impl std::fmt::Debug for QwenConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QwenConfig")
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

impl QwenConfig {
    /// Load configuration from environment variables.
    ///
    /// # Environment Variables
    /// - `DASHSCOPE_API_KEY` (required): Qwen (DashScope) API key.
    /// - `DASHSCOPE_BASE_URL` (optional): API base URL, defaults to
    ///   [`QWEN_DEFAULT_BASE_URL`].
    /// - `DASHSCOPE_MODEL` (optional): Default model, defaults to
    ///   [`QWEN_DEFAULT_MODEL`].
    /// - `DASHSCOPE_TIMEOUT_SECONDS` (optional): Request timeout, defaults to
    ///   `60`.
    ///
    /// # Errors
    /// Returns an error if `DASHSCOPE_API_KEY` is absent, or another
    /// variable's value fails to parse or validate.
    pub fn from_env() -> Result<Self, String> {
        Self::from_parts(
            env::var("DASHSCOPE_API_KEY").ok(),
            env::var("DASHSCOPE_BASE_URL").ok(),
            env::var("DASHSCOPE_MODEL").ok(),
            env::var("DASHSCOPE_TIMEOUT_SECONDS").ok(),
        )
    }

    /// The pure defaulting/validation logic behind [`Self::from_env`],
    /// separated out so it is testable without mutating process environment
    /// variables — `std::env::set_var` is `unsafe` under Rust 2024 and this
    /// crate denies `unsafe_code` (`#![deny(unsafe_code)]`).
    fn from_parts(
        api_key: Option<String>,
        base_url: Option<String>,
        model: Option<String>,
        timeout_seconds: Option<String>,
    ) -> Result<Self, String> {
        let api_key =
            api_key.ok_or_else(|| "DASHSCOPE_API_KEY environment variable not set".to_string())?;
        let base_url = base_url.unwrap_or_else(|| QWEN_DEFAULT_BASE_URL.to_string());
        let model = model.unwrap_or_else(|| QWEN_DEFAULT_MODEL.to_string());
        let timeout_seconds = match timeout_seconds {
            Some(value) => value
                .parse()
                .map_err(|_| "Invalid DASHSCOPE_TIMEOUT_SECONDS value".to_string())?,
            None => default_timeout_seconds(QWEN_PROVIDER, &model, 60),
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
    /// - `api_key`: the Qwen (DashScope) API key, never a URL.
    /// - `model`: the adapter's default model. It selects the timeout default (300 s for a
    ///   seeded reasoning model, 60 s otherwise); each request's own `LlmRequest.model`
    ///   decides what is sent on the wire.
    ///
    /// The endpoint is [`QWEN_DEFAULT_BASE_URL`] (replace it with
    /// [`with_base_url`](Self::with_base_url)) and the timeout is 300 s for a seeded reasoning
    /// model and 60 s otherwise (replace it with
    /// [`with_timeout_seconds`](Self::with_timeout_seconds)). Construction is infallible: an
    /// empty key or model, or a non-`http(s)` endpoint, is refused when the adapter is built.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_llm::qwen::QwenConfig;
    /// use paladin_llm::qwen::adapter::{QWEN_DEFAULT_BASE_URL, QWEN_DEFAULT_MODEL};
    ///
    /// let key = "qwen-example-key".to_string();
    /// let config = QwenConfig::new(key, QWEN_DEFAULT_MODEL.to_string()).with_timeout_seconds(600);
    /// assert_eq!(config.base_url, QWEN_DEFAULT_BASE_URL);
    /// assert_eq!(config.timeout_seconds, 600);
    /// ```
    pub fn new(api_key: String, model: String) -> Self {
        let timeout_seconds = default_timeout_seconds(QWEN_PROVIDER, &model, 60);
        Self {
            api_key,
            base_url: QWEN_DEFAULT_BASE_URL.to_string(),
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

config_knob_builders!(QwenConfig);

/// The provider name this adapter reports through [`LlmPort::get_provider_name`]
/// and that its engine stamps on every `LlmError::ProviderError` (Phase 25 D-03).
const QWEN_PROVIDER: &str = "qwen";

/// Qwen (Alibaba DashScope) LLM Adapter implementing [`LlmPort`].
///
/// Every method delegates to an owned [`CompatEngine`] (D-05) — this struct
/// carries no protocol logic of its own.
pub struct QwenAdapter {
    engine: CompatEngine,
}

impl QwenAdapter {
    /// Create a new Qwen adapter.
    ///
    /// # Errors
    /// Returns an error if configuration is invalid or the underlying HTTP
    /// client cannot be created.
    pub fn new(config: QwenConfig) -> Result<Self, LlmError> {
        config.validate().map_err(|e| {
            LlmError::AuthenticationError(format!("Invalid Qwen configuration: {}", e))
        })?;

        let engine_config = CompatEngineConfig {
            base_url: config.base_url,
            api_key: config.api_key,
            model: config.model,
            timeout_seconds: config.timeout_seconds,
            max_retries: config.max_retries,
            capabilities: CompatCapabilities {
                supports_streaming: true,
                supports_tool_calling: false,
                supports_function_calling: false,
                // The `qwen-vl` family exists, but this is the text adapter
                // — it reports what its own request path implements, not
                // what the vendor's product line advertises (D-08's vision
                // boundary; T-17-16).
                supports_vision: false,
                supports_embeddings: false,
                // The default model's seeded window (`seeded::QWEN`, `qwen3.8-flash`, verified
                // 2026-10-15: the vendor's China/Beijing model page, the only page that states
                // one), replacing the unevidenced 131,072.
                max_context_tokens: config
                    .context_window
                    .or(seeded_window(seeded::QWEN, QWEN_DEFAULT_MODEL)),
                supports_system_messages: true,
                // Live-measured 2026-08-23 against `qwen-plus` and
                // `qwen3.7-plus` at the shipped Singapore endpoint:
                // DashScope's own error envelope states the accepted range
                // as the HALF-OPEN interval `[0.0, 2.0)` — `temperature:2.0`
                // returns `HTTP 400 InternalError.Algo.InvalidParameter:
                // "Temperature should be in [0.0, 2.0)"` on both models,
                // while `1.9999` is accepted. `ProviderCapabilities`'s own
                // gate (`PaladinBuilder::validate`, ADR-0004) treats both
                // tuple endpoints as INCLUSIVE, so advertising the
                // vendor-documented `2.0` upper bound verbatim would let a
                // legal-looking request through the local gate only to be
                // rejected on the wire — Kimi's defect class (17-19):
                // declare what was measured, not what the vendor's prose
                // says. `1.99` is comfortably inside the accepted range and
                // round enough to read as "just under 2", not a
                // floating-point accident.
                temperature_range: Some((0.0, 1.99)),
            },
            // Live-measured 2026-08-23 against `qwen-plus` and
            // `qwen3.7-plus` at the shipped Singapore endpoint: each of the
            // five optional sampling parameters (`temperature`, `max_tokens`,
            // `top_p`, `frequency_penalty`, `presence_penalty`), probed
            // individually, returned HTTP 200 with a real completion on both
            // models. No rejection observed — `all()` is the measured
            // declaration, not an unmeasured default.
            // Phase 51 (D-04, REASON-02): Qwen's documented form is `enable_thinking` with an
            // optional `thinking_budget`. Plan 51-11 restates it from `51-EFFORT-PROBE.md`.
            request_parameters: CompatRequestParameters::all(ReasoningWire::EnableThinking),
            fallback_models: QWEN_FALLBACK_MODELS.iter().map(|s| s.to_string()).collect(),
            // No usage-cap (quota) mapping for qwen: its quota-exhaustion body was never captured
            // (50-SEED-PROBE.md, owner reading R-2: accepted unverified 2026-10-16, plan 50-17),
            // so an override would be invented evidence (D-11a). The shared status mapping applies.
            error_override: None,
            // WR-04 (`17-REVIEW.md`, T-17-52/T-17-53), superseding the
            // 17-04 comment this replaces: `QWEN_DEFAULT_BASE_URL` is only
            // this preset's *default* — `DASHSCOPE_BASE_URL` is documented
            // and operator-settable (`QwenConfig::from_env`, and this
            // module's own "Region default" doc section above), so a `3xx`
            // from whatever host it resolves to could otherwise replay the
            // `Authorization` header carrying the operator's credential to
            // a different host. Setting `Policy::none()` here is what
            // prevents that, matching `openai_compatible`'s posture
            // (T-17-18); a refused redirect surfaces via the engine's
            // `300..=399` `map_error` arm.
            redirect_policy: Some(reqwest::redirect::Policy::none()),
        };

        // ADR-0061 (plan 50-03): the adapter-wide fallback profile is this preset's own
        // declaration above, restated as a profile; per-model seeded entries arrive in plan
        // 50-06.
        // Stop sequences: `Supported { max: None }` is the vendor's published request schema
        // (A3-50-03); a vendor that rejects `stop` for a model is corrected by that model's
        // seeded entry (plan 50-06), never by this adapter (D-04a).
        let fallback = fallback_from_compat(
            &engine_config.capabilities.clone().into(),
            &engine_config.request_parameters,
            StopSupport::Supported { max: None },
        );

        // PARITY-03: the operator's adapter-wide window, then the per-model overrides, are laid
        // over the seeded table; either one failing construction names the provider and model.
        let registry = ModelRegistry::new(QWEN_PROVIDER, seeded::QWEN, fallback)
            .with_context_window(config.context_window)
            .and_then(|registry| registry.with_overrides(config.model_overrides))
            .map_err(|e| {
                LlmError::AuthenticationError(format!("Invalid Qwen configuration: {e}"))
            })?;

        Ok(Self {
            // Phase 25 D-03: name the engine so every `ProviderError` it
            // emits carries "qwen", not the engine's generic default.
            engine: CompatEngine::new(engine_config)?
                .with_provider_name(QWEN_PROVIDER)
                .with_default_max_tokens(config.max_tokens)
                .with_default_reasoning(config.reasoning_effort, config.reasoning_fallback)
                // G-50-5a (plan 50-24): no `with_usage_semantics` -- Qwen keeps the engine default
                // `CompatUsageSemantics::ReasoningInsideCompletion`. Live-measured 2026-10-18
                // (`50-USAGE-PROBE.md`, operator run on the raw DashScope wire): qwen3.8-flash
                // reads `reasoning inside completion` (83 + 75 = 158, reasoning 69); qwen-plus
                // reported no reasoning (`indeterminate`, 30 + 3 = 33), which neither confirms
                // nor contradicts it.
                .with_model_registry(registry),
        })
    }
}

#[async_trait]
impl LlmPort for QwenAdapter {
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
        QWEN_PROVIDER
    }

    fn get_capabilities(&self) -> ProviderCapabilities {
        self.engine.capabilities()
    }

    fn model_profile(&self, model: &str) -> Option<ModelProfile> {
        Some(self.engine.model_profile(model))
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

    /// WR-03: the documented `with_base_url` guarantee is an `http://` or `https://` URL. A value
    /// that merely starts with `http` (`httpx://x`, a scheme-less `http-gateway...`) must fail
    /// construction like any other non-HTTP value.
    #[test]
    fn a_builder_set_non_http_base_url_fails_qwen_construction() {
        for bad in [
            "ftp://x",
            "",
            "api.example.com/v1",
            "httpx://x",
            "http-gateway.internal/v1",
            "httpfoo:",
        ] {
            let config =
                QwenConfig::new("k".to_string(), "qwen3.8-flash".to_string()).with_base_url(bad);
            assert!(
                config.validate().is_err(),
                "base URL {bad:?} must fail validation"
            );
            assert!(
                QwenAdapter::new(config).is_err(),
                "base URL {bad:?} must fail adapter construction"
            );
        }
        for good in ["http://localhost:1", "https://gateway.example/v1"] {
            let config =
                QwenConfig::new("k".to_string(), "qwen3.8-flash".to_string()).with_base_url(good);
            assert!(
                config.validate().is_ok(),
                "base URL {good:?} must pass validation"
            );
        }
    }

    // ── QwenConfig::new (SURFACE-02, D-05) ──

    #[test]
    fn qwen_config_new_takes_key_and_model_and_defaults_the_endpoint() {
        let config = QwenConfig::new("k".to_string(), "qwen-plus".to_string());
        assert_eq!(config.base_url, QWEN_DEFAULT_BASE_URL);
        assert_eq!(config.model, "qwen-plus");
        assert_eq!(config.api_key, "k");
        // The timeout default is unchanged: whatever `default_timeout_seconds` answered for the
        // same model before the endpoint left the constructor.
        assert_eq!(
            config.timeout_seconds,
            default_timeout_seconds(QWEN_PROVIDER, "qwen-plus", 60)
        );
        let seeded = QwenConfig::new("k".to_string(), QWEN_DEFAULT_MODEL.to_string());
        assert_eq!(
            seeded.timeout_seconds,
            default_timeout_seconds(QWEN_PROVIDER, QWEN_DEFAULT_MODEL, 60)
        );
    }

    #[test]
    fn qwen_config_with_base_url_replaces_the_default_endpoint() {
        let config = QwenConfig::new("k".to_string(), "m".to_string())
            .with_base_url("http://localhost:1234");
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

    #[tokio::test]
    async fn qwen_stop_reaches_the_wire_in_order() {
        let mut server = crate::compat::test_support::server().await;
        let captured = crate::compat::test_support::mock_completion(&mut server).await;
        let adapter = QwenAdapter::new(
            QwenConfig::new("test-key".to_string(), QWEN_DEFAULT_MODEL.to_string())
                .with_base_url(server.url()),
        )
        .unwrap();
        let mut request = build_request(QWEN_DEFAULT_MODEL);
        request.prompt.entity.data.parameters = PromptParameters {
            max_tokens: Some(64),
            temperature: Some(0.5),
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop_sequences: Some(vec!["END".to_string(), "STOP".to_string()]),
        };

        let response = adapter
            .generate(request)
            .await
            .expect("well-formed response");
        let body = captured.lock().unwrap().take().expect("a request was sent");

        assert_eq!(body.get("stop"), Some(&json!(["END", "STOP"])));
        assert!(
            !response
                .metadata
                .contains_key(crate::sampling::PARAMS_OMITTED_METADATA_KEY)
        );
        let profile = adapter
            .model_profile(QWEN_DEFAULT_MODEL)
            .expect("qwen profiles every model");
        assert_eq!(
            profile.temperature,
            paladin_ports::output::model_profile::ParamSupport::Range(0.0, 1.99)
        );
    }

    /// G-50-5a (plan 50-24): the live-measured qwen3.8-flash usage object
    /// (`50-USAGE-PROBE.md`, 2026-10-18: prompt 83 with 0 cached, completion 75 of which
    /// reasoning 69, total 158) is read with the engine-default inclusive semantics and
    /// reconciles with DashScope's own total. Read beside completion it would map to 227.
    #[tokio::test]
    async fn qwen_measured_usage_is_read_inside_completion() {
        use crate::usage_reconcile::USAGE_UNRECONCILED_METADATA_KEY;

        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "id": "cmpl-qwen-measured",
                    "model": "qwen3.8-flash",
                    "choices": [{
                        "index": 0,
                        "message": {"role": "assistant", "content": "391"},
                        "finish_reason": "stop"
                    }],
                    "usage": {
                        "prompt_tokens": 83,
                        "completion_tokens": 75,
                        "total_tokens": 158,
                        "prompt_tokens_details": {"cached_tokens": 0},
                        "completion_tokens_details": {"reasoning_tokens": 69}
                    }
                })
                .to_string(),
            )
            .create_async()
            .await;
        let adapter = QwenAdapter::new(
            QwenConfig::new("test-key".to_string(), "qwen3.8-flash".to_string())
                .with_base_url(server.url()),
        )
        .unwrap();

        let response = adapter
            .generate(build_request("qwen3.8-flash"))
            .await
            .expect("well-formed response");

        let usage = &response.usage;
        assert_eq!(usage.prompt_tokens, 83);
        assert_eq!(usage.cache_read_tokens, Some(0));
        assert_eq!(usage.completion_tokens, 75, "{usage:?}");
        assert_eq!(usage.reasoning_tokens, Some(69));
        assert_eq!(usage.total_tokens, 158);
        assert!(
            !response
                .metadata
                .contains_key(USAGE_UNRECONCILED_METADATA_KEY),
            "{:?}",
            response.metadata
        );
    }

    #[test]
    fn qwen_default_model_is_seeded() {
        let adapter = QwenAdapter::new(
            QwenConfig::new("test-key".to_string(), QWEN_DEFAULT_MODEL.to_string())
                .with_base_url("https://example.invalid/v1"),
        )
        .unwrap();
        let profile = adapter
            .model_profile(QWEN_DEFAULT_MODEL)
            .expect("qwen profiles every model");
        // A seeded entry stamps its date on the profile; the adapter-wide fallback carries none.
        assert_eq!(profile.verified_on.as_deref(), Some("2026-10-15"));
        assert_eq!(profile.context_window, Some(1_000_000));
        assert_eq!(
            adapter.get_capabilities().max_context_tokens,
            profile.context_window,
            "the declared window is the default model's seeded window"
        );
        // An id outside the table keeps the adapter-wide fallback (D-07).
        let unseeded = adapter
            .model_profile("qwen3-max")
            .expect("qwen profiles every model");
        assert_eq!(unseeded.verified_on, None);
    }

    #[test]
    fn qwen_default_model_is_the_operator_identified_current_model() {
        assert_eq!(QWEN_DEFAULT_MODEL, "qwen3.8-flash");
        assert_eq!(QWEN_FALLBACK_MODELS[0], QWEN_DEFAULT_MODEL);
        assert!(QWEN_FALLBACK_MODELS.contains(&"qwen-plus"));
    }

    // ── QwenConfig::from_env() defaulting logic ──

    #[test]
    fn qwen_config_from_env_errors_when_api_key_absent() {
        let result = QwenConfig::from_parts(None, None, None, None);
        assert!(result.is_err());
    }

    #[test]
    fn qwen_config_defaults_base_url_and_model_when_only_key_is_set() {
        let config = QwenConfig::from_parts(Some("test-key".to_string()), None, None, None)
            .expect("key alone must be sufficient to build a valid config");
        assert_eq!(config.base_url, QWEN_DEFAULT_BASE_URL);
        assert_eq!(config.model, QWEN_DEFAULT_MODEL);
        // The default model is a seeded reasoning model, so its default timeout is the
        // reasoning-model 300 s (D-13), not the hosted-preset 60 s.
        assert_eq!(
            config.timeout_seconds,
            registry::REASONING_DEFAULT_TIMEOUT_SECONDS
        );
    }

    /// Pins the shipped default to the literal Singapore (international)
    /// compatible-mode endpoint, per the developer's binding decision of
    /// 2026-08-23 (Move 2, "Reversal record" above). Asserted against the
    /// literal rather than the constant, so this test fails against the
    /// intermediate Virginia default and only passes once the constant is
    /// actually changed — unlike
    /// `qwen_config_defaults_base_url_and_model_when_only_key_is_set` above,
    /// which follows the constant and would pass regardless of its value.
    #[test]
    fn qwen_config_defaults_to_the_singapore_intl_endpoint_by_literal() {
        let config = QwenConfig::from_parts(Some("test-key".to_string()), None, None, None)
            .expect("key alone must be sufficient to build a valid config");
        assert_eq!(
            config.base_url,
            "https://dashscope-intl.aliyuncs.com/compatible-mode/v1"
        );
    }

    #[test]
    fn qwen_config_honors_dashscope_base_url_override() {
        let config = QwenConfig::from_parts(
            Some("test-key".to_string()),
            Some("https://override.example/v1".to_string()),
            Some("qwen-turbo".to_string()),
            Some("30".to_string()),
        )
        .unwrap();
        assert_eq!(config.base_url, "https://override.example/v1");
        assert_eq!(config.model, "qwen-turbo");
        assert_eq!(config.timeout_seconds, 30);
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
                    "model": "qwen-plus",
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

        let config = QwenConfig::new("test-key".to_string(), "qwen-plus".to_string())
            .with_base_url(server.url());
        let adapter = QwenAdapter::new(config).unwrap();

        let response = adapter
            .generate(build_request("qwen-plus"))
            .await
            .expect("mock server returned a well-formed response");

        assert_eq!(response.content, "Hi there");
        assert!(matches!(response.finish_reason, FinishReason::Stop));

        mock.assert_async().await;
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

        let config = QwenConfig::new("test-key".to_string(), "qwen-plus".to_string())
            .with_base_url(server.url());
        let adapter = QwenAdapter::new(config).unwrap();

        let stream = adapter
            .generate_stream(build_request("qwen-plus"))
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

    // ── Error mapping ──

    #[tokio::test]
    async fn http_401_maps_to_authentication_error() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(401)
            .with_body(r#"{"error":"invalid key"}"#)
            .create_async()
            .await;

        let config = QwenConfig::new("test-key".to_string(), "qwen-plus".to_string())
            .with_base_url(server.url());
        let adapter = QwenAdapter::new(config).unwrap();

        let result = adapter.generate(build_request("qwen-plus")).await;
        assert!(matches!(result, Err(LlmError::AuthenticationError(_))));
    }

    #[tokio::test]
    async fn http_429_maps_to_rate_limit_exceeded() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(429)
            .with_body(r#"{"error":"slow down"}"#)
            .expect_at_least(1)
            .create_async()
            .await;

        let config = QwenConfig::new("test-key".to_string(), "qwen-plus".to_string())
            .with_base_url(server.url());
        let adapter = QwenAdapter::new(config).unwrap();

        let result = adapter.generate(build_request("qwen-plus")).await;
        assert!(matches!(result, Err(LlmError::RateLimitExceeded { .. })));
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

        let config = QwenConfig::new("test-key".to_string(), "qwen-plus".to_string())
            .with_base_url(server.url());
        let adapter = QwenAdapter::new(config).unwrap();

        let result = adapter.generate(build_request("qwen-plus")).await;
        assert!(matches!(result, Err(LlmError::ModelNotAvailable(_))));
    }

    #[tokio::test]
    async fn http_400_maps_to_invalid_prompt() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(400)
            .with_body(r#"{"error":"bad request"}"#)
            .create_async()
            .await;

        let config = QwenConfig::new("test-key".to_string(), "qwen-plus".to_string())
            .with_base_url(server.url());
        let adapter = QwenAdapter::new(config).unwrap();

        let result = adapter.generate(build_request("qwen-plus")).await;
        assert!(matches!(result, Err(LlmError::InvalidPrompt(_))));
    }

    /// Phase 25 (FT-FR-01, D-03): the preset's non-2xx path is the shared
    /// `map_http_status` — proven by comparing the adapter's error against
    /// the helper's own output for the same status, body and key.
    #[tokio::test]
    async fn qwen_non_2xx_routes_through_the_shared_mapper() {
        let body = r#"{"error":"overloaded"}"#;
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(503)
            .with_body(body)
            .expect_at_least(1)
            .create_async()
            .await;

        let config = QwenConfig::new("test-key".to_string(), "qwen-plus".to_string())
            .with_base_url(server.url());
        let adapter = QwenAdapter::new(config).unwrap();

        let result = adapter.generate(build_request("qwen-plus")).await;
        let expected = map_http_status("qwen", 503, body, "test-key");
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

    // ── Capabilities / identity ──

    #[test]
    fn get_provider_name_returns_qwen() {
        let config = QwenConfig::new("test-key".to_string(), QWEN_DEFAULT_MODEL.to_string());
        let adapter = QwenAdapter::new(config).unwrap();
        assert_eq!(adapter.get_provider_name(), "qwen");
    }

    #[test]
    fn get_capabilities_reports_no_vision_and_no_tool_or_function_calling() {
        let config = QwenConfig::new("test-key".to_string(), QWEN_DEFAULT_MODEL.to_string());
        let adapter = QwenAdapter::new(config).unwrap();
        let caps = adapter.get_capabilities();

        assert!(!caps.supports_tool_calling);
        assert!(!caps.supports_function_calling);
        assert!(!caps.supports_vision);
        assert!(!caps.supports_embeddings);
        assert!(caps.supports_streaming);
        assert!(caps.supports_system_messages);
    }

    // ── Shared conformance suite (D-19, plan 31-04) ──
    //
    // Qwen delegates every method to `CompatEngine` (D-05), so it speaks the identical
    // OpenAI-compatible wire shape `openai_compatible`/`ollama` already instantiate the suite
    // against; bodies below mirror those fixtures' shape exactly (distinct model/provider names
    // only).
    mod conformance_suite {
        use super::*;
        use std::sync::Arc;

        struct QwenFixture;

        impl crate::conformance::ConformanceFixture for QwenFixture {
            const WIRE: crate::conformance::Wire = crate::conformance::Wire::OpenAiChat;

            fn default_model() -> &'static str {
                QWEN_DEFAULT_MODEL
            }

            fn provider_name() -> &'static str {
                "qwen"
            }

            fn adapter(base_url: &str) -> Arc<dyn LlmPort> {
                let config =
                    QwenConfig::new("test-key".to_string(), QWEN_DEFAULT_MODEL.to_string())
                        .with_base_url(base_url);
                Arc::new(QwenAdapter::new(config).expect("test config must build"))
            }

            fn success_body() -> String {
                json!({
                    "id": "cmpl-1",
                    "model": QWEN_DEFAULT_MODEL,
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
                // D-19: the trailing empty-`choices` usage frame carries the SAME figures as
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

        crate::llm_conformance_suite!(QwenFixture);
    }

    // ── PARITY-03 knobs (plan 50-08, D-12) ──

    const UNSEEDED: &str = "qwen-unseeded-test-model";

    fn knob_config(url: &str, model: &str) -> QwenConfig {
        QwenConfig::new("test-key".to_string(), model.to_string()).with_base_url(url)
    }

    #[tokio::test]
    async fn qwen_max_tokens_default_reaches_the_wire() {
        use crate::compat::test_support::{mock_completion, send_with_max_tokens, server};

        let mut server = server().await;
        let captured = mock_completion(&mut server).await;
        let adapter =
            QwenAdapter::new(knob_config(&server.url(), UNSEEDED).with_max_tokens(2048)).unwrap();

        let body = send_with_max_tokens(&adapter, &captured, UNSEEDED, None)
            .await
            .unwrap();
        assert_eq!(body["max_tokens"], 2048, "{body}");
        let body = send_with_max_tokens(&adapter, &captured, UNSEEDED, Some(512))
            .await
            .unwrap();
        assert_eq!(body["max_tokens"], 512, "{body}");

        let plain = QwenAdapter::new(knob_config(&server.url(), UNSEEDED)).unwrap();
        let body = send_with_max_tokens(&plain, &captured, UNSEEDED, None)
            .await
            .unwrap();
        assert!(body.get("max_tokens").is_none(), "{body}");
    }

    #[tokio::test]
    async fn qwen_max_retries_is_read_by_the_engine() {
        for retries in [0_u32, 1] {
            let server = crate::compat::test_support::server().await;
            let adapter =
                QwenAdapter::new(knob_config(&server.url(), UNSEEDED).with_max_retries(retries))
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
    fn qwen_context_window_override_is_declared() {
        let adapter = QwenAdapter::new(
            knob_config("https://example.invalid/v1", UNSEEDED).with_context_window(500_000),
        )
        .unwrap();
        assert_eq!(adapter.get_capabilities().max_context_tokens, Some(500_000));
        let profile = adapter.model_profile(UNSEEDED).unwrap();
        assert_eq!(profile.context_window, Some(500_000));
    }

    #[test]
    fn qwen_model_override_reaches_the_profile_and_invalid_one_fails_construction() {
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
        let adapter = QwenAdapter::new(with_override(900_000)).unwrap();
        assert_eq!(
            adapter.model_profile(UNSEEDED).unwrap().context_window,
            Some(900_000)
        );
        let message = match QwenAdapter::new(with_override(0)) {
            Err(error) => error.to_string(),
            Ok(_) => panic!("a zero window must fail construction"),
        };
        assert!(
            message.contains("qwen") && message.contains(UNSEEDED),
            "{message}"
        );
    }

    #[test]
    fn qwen_config_defaults_a_reasoning_model_to_300s() {
        // (model, expected default timeout): a seeded reasoning model gets 300 s (D-13); any
        // other model keeps this preset's compiled default (60 s).
        for (model, expected) in [
            (QWEN_DEFAULT_MODEL, 300),
            ("qwen-plus", 60),
            // The Phase 50.1 rows: the ones whose default call returned reasoning text get
            // 300 s, the others keep the hosted-preset 60 s.
            ("qwen3.8-max", 300),
            ("qwen3.7-flash", 300),
            ("qwen3.6-plus", 300),
            ("qwen3.5-flash", 300),
            ("qwen-max", 60),
            ("qwen-flash", 60),
            ("qwen-plus-latest", 60),
            (UNSEEDED, 60),
        ] {
            let url = "https://example.invalid/v1";
            assert_eq!(
                knob_config(url, model).timeout_seconds,
                expected,
                "new({model})"
            );
            let parts =
                QwenConfig::from_parts(Some("k".to_string()), None, Some(model.to_string()), None)
                    .unwrap();
            assert_eq!(parts.timeout_seconds, expected, "from_parts({model})");
        }
        // An explicit environment value always wins.
        for explicit in ["30", "60", "900"] {
            let config = QwenConfig::from_parts(
                Some("k".to_string()),
                None,
                Some("qwen3.8-flash".to_string()),
                Some(explicit.to_string()),
            )
            .unwrap();
            assert_eq!(config.timeout_seconds.to_string(), explicit);
        }
    }

    #[test]
    fn qwen_context_window_below_a_seeded_max_output_fails_construction() {
        // `qwen3.8-flash` is seeded with a 131,072-token maximum output; a 100,000-token
        // window contradicts it and is refused, never clamped.
        let message = match QwenAdapter::new(
            knob_config("https://example.invalid/v1", QWEN_DEFAULT_MODEL)
                .with_context_window(100_000),
        ) {
            Err(error) => error.to_string(),
            Ok(_) => panic!("a window below a seeded maximum output must fail construction"),
        };
        for needle in ["qwen", "qwen3.8-flash", "100000", "131072"] {
            assert!(
                message.contains(needle),
                "'{needle}' missing from: {message}"
            );
        }
    }

    #[test]
    fn qwen_config_debug_redacts_the_api_key() {
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
    // ── Phase 51 (REASON-02, D-04): the declared reasoning wire ──

    #[test]
    fn qwen_declares_the_enable_thinking_wire() {
        let adapter =
            QwenAdapter::new(QwenConfig::new("k".to_string(), "qwen-plus".to_string())).unwrap();
        assert_eq!(
            adapter.engine.request_parameters().reasoning,
            ReasoningWire::EnableThinking
        );
    }
    mod reasoning_effort {
        use super::*;
        use crate::compat::test_support::{mock_completion, plain_request, send_capturing, server};
        use crate::reasoning::{
            REASONING_EFFECTIVE_METADATA_KEY, REASONING_REQUESTED_METADATA_KEY,
        };
        use paladin_ports::output::llm_port::ReasoningEffort;
        use paladin_ports::output::model_profile::{BudgetBounds, EffortLevels, EffortSupport};

        const MODEL: &str = "qwen3.8-max";

        fn adapter(base_url: &str) -> QwenAdapter {
            let over = registry::ModelOverride {
                effort: Some(EffortSupport::Supported(
                    EffortLevels::new([])
                        .with_off(true)
                        .with_auto(true)
                        .with_budget(BudgetBounds::new(1, 32_768)),
                )),
                ..registry::ModelOverride::default()
            };
            QwenAdapter::new(
                QwenConfig::new("test-key".to_string(), MODEL.to_string())
                    .with_base_url(base_url)
                    .with_model_overrides(BTreeMap::from([(MODEL.to_string(), over)])),
            )
            .unwrap()
        }

        #[tokio::test]
        async fn a_budget_sends_enable_thinking_and_thinking_budget() {
            let mut server = server().await;
            let captured = mock_completion(&mut server).await;
            let adapter = adapter(&server.url());

            let (body, response) = send_capturing(
                &adapter,
                &captured,
                plain_request(MODEL).with_reasoning(ReasoningEffort::Budget(512)),
            )
            .await
            .expect("an in-bounds budget is accepted");
            assert_eq!(body["enable_thinking"], true, "{body}");
            assert_eq!(body["thinking_budget"], 512, "{body}");
            assert!(body.get("reasoning_effort").is_none(), "{body}");
            assert!(body.get("thinking").is_none(), "{body}");
            assert_eq!(
                response.metadata[REASONING_REQUESTED_METADATA_KEY],
                "budget:512"
            );
            assert_eq!(
                response.metadata[REASONING_EFFECTIVE_METADATA_KEY],
                "budget:512"
            );

            let (off, _) = send_capturing(
                &adapter,
                &captured,
                plain_request(MODEL).with_reasoning(ReasoningEffort::Off),
            )
            .await
            .unwrap();
            assert_eq!(off["enable_thinking"], false, "{off}");
            assert!(off.get("thinking_budget").is_none(), "{off}");
        }

        #[tokio::test]
        async fn an_out_of_bounds_budget_and_a_level_are_refused_before_http() {
            let mut server = server().await;
            let captured = mock_completion(&mut server).await;
            let adapter = adapter(&server.url());

            for requested in [ReasoningEffort::Budget(32_769), ReasoningEffort::Low] {
                let error = send_capturing(
                    &adapter,
                    &captured,
                    plain_request(MODEL).with_reasoning(requested),
                )
                .await
                .expect_err("refused");
                assert!(
                    matches!(error, LlmError::UnsupportedParameter { .. }),
                    "{requested}: {error:?}"
                );
                assert!(
                    captured.lock().unwrap().is_none(),
                    "{requested}: no HTTP hit"
                );
            }
        }
    }
}
