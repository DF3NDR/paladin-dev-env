//! Kimi (Moonshot AI) LLM Adapter.
//!
//! A thin preset (D-01, D-05, D-12) sitting entirely on
//! [`crate::compat::CompatEngine`] — the shared OpenAI-compatible protocol
//! engine. This adapter supplies only Kimi's `base_url`, credential env var,
//! default model, curated fallback model list, capabilities block and
//! request-parameters declaration; every other behaviour (request shaping,
//! retry, streaming, error mapping, credential redaction, memoized
//! model-list resolution) is inherited from the engine unchanged.
//!
//! **Live-verified 2026-08-22 (17-19, closing G-17-4b):** the model
//! constants below and the fixed-temperature declaration on
//! [`KimiAdapter::new`] were previously vendor-documentation-derived and not
//! live-checked. Both are now live-verified against
//! `GET https://api.moonshot.ai/v1/models` and three live
//! `POST /chat/completions` temperature probes (17-19-SUMMARY.md carries the
//! full live model list and all three probe verdicts verbatim). The
//! `moonshot-v1-*` family this file previously shipped is retired — the
//! live list returns `404 resource_not_found_error` for every one of them.

use async_trait::async_trait;
use futures::Stream;
use std::collections::BTreeMap;
use std::env;

use paladin_ports::output::llm_port::{
    LlmError, LlmPort, LlmRequest, LlmResponse, ProviderCapabilities, StreamingResponse,
};

use paladin_ports::output::model_profile::{ModelProfile, StopSupport};

use crate::compat::{
    CompatCapabilities, CompatEngine, CompatEngineConfig, CompatRequestParameters,
};
use crate::redaction::RedactedKey;
use crate::registry::{
    DEFAULT_MAX_RETRIES, ModelOverride, ModelRegistry, config_knob_builders,
    default_timeout_seconds, fallback_from_compat, is_http_url, seeded, seeded_window,
};

/// Default Kimi (Moonshot AI) API base URL.
///
/// `[CITED: platform.moonshot.ai]` — not live-verified at plan-authoring
/// time (17-RESEARCH.md Assumptions Log A1); confirmed against the vendor's
/// published API reference before this file was committed (see the plan's
/// SUMMARY.md for the exact URL checked, per D-00e).
pub const KIMI_DEFAULT_BASE_URL: &str = "https://api.moonshot.ai/v1";

/// Default Kimi model requested when `MOONSHOT_MODEL` is unset.
///
/// `kimi-k2.6`: the operator-identified current general-purpose model (its cheapest current
/// tier), measured end to end by the 2026-10-15 live probe (`registry/seeded.rs`, `verified_on`
/// 2026-10-15). It replaces `kimi-k3`, which stays available and seeded; pin `MOONSHOT_MODEL` to
/// keep it. The two code-specialised entries (`kimi-k2.7-code`, `kimi-k2.7-code-highspeed`)
/// answer only requests that enable thinking, so they are never the default.
///
/// History: `kimi-k3` (2026-08-22, 17-19, closing G-17-4b) replaced the retired
/// `moonshot-v1-8k`, which the live `GET /models` no longer listed.
pub const KIMI_DEFAULT_MODEL: &str = "kimi-k2.6";

/// Curated fallback model list (D-13), returned when the live `/models`
/// endpoint fails, is unreachable, or returns an empty list. Never reported
/// as authoritative — see [`crate::compat::engine::CompatEngine::available_models`].
///
/// Populated from the same 2026-08-22 live `GET /models` response as
/// [`KIMI_DEFAULT_MODEL`] (default first), replacing the retired
/// `moonshot-v1-*` family this constant previously held.
pub const KIMI_FALLBACK_MODELS: &[&str] = &[
    KIMI_DEFAULT_MODEL,
    "kimi-k3",
    "kimi-k2.7-code",
    "kimi-k2.7-code-highspeed",
];

/// Configuration for the Kimi (Moonshot AI) adapter.
#[derive(Clone)]
#[non_exhaustive]
pub struct KimiConfig {
    /// API key for Kimi authentication.
    pub api_key: String,
    /// Base URL for the Kimi API.
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
}

/// Prints every field except the API key, which is shown as `<redacted>` (D-00a): the key is a
/// public `String`, so a derived `Debug` would put it into any log line or panic message that
/// formats the config.
impl std::fmt::Debug for KimiConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KimiConfig")
            .field("api_key", &RedactedKey(&self.api_key))
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("timeout_seconds", &self.timeout_seconds)
            .field("max_tokens", &self.max_tokens)
            .field("max_retries", &self.max_retries)
            .field("context_window", &self.context_window)
            .field("model_overrides", &self.model_overrides)
            .finish()
    }
}

impl KimiConfig {
    /// Load configuration from environment variables.
    ///
    /// # Environment Variables
    /// - `MOONSHOT_API_KEY` (required): Kimi API key.
    /// - `MOONSHOT_BASE_URL` (optional): API base URL, defaults to
    ///   [`KIMI_DEFAULT_BASE_URL`].
    /// - `MOONSHOT_MODEL` (optional): Default model, defaults to
    ///   [`KIMI_DEFAULT_MODEL`].
    /// - `MOONSHOT_TIMEOUT_SECONDS` (optional): Request timeout, defaults to
    ///   `60`.
    ///
    /// # Errors
    /// Returns an error if `MOONSHOT_API_KEY` is absent, or another
    /// variable's value fails to parse or validate.
    pub fn from_env() -> Result<Self, String> {
        Self::from_parts(
            env::var("MOONSHOT_API_KEY").ok(),
            env::var("MOONSHOT_BASE_URL").ok(),
            env::var("MOONSHOT_MODEL").ok(),
            env::var("MOONSHOT_TIMEOUT_SECONDS").ok(),
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
            api_key.ok_or_else(|| "MOONSHOT_API_KEY environment variable not set".to_string())?;
        let base_url = base_url.unwrap_or_else(|| KIMI_DEFAULT_BASE_URL.to_string());
        let model = model.unwrap_or_else(|| KIMI_DEFAULT_MODEL.to_string());
        let timeout_seconds = match timeout_seconds {
            Some(value) => value
                .parse()
                .map_err(|_| "Invalid MOONSHOT_TIMEOUT_SECONDS value".to_string())?,
            None => default_timeout_seconds(KIMI_PROVIDER, &model, 60),
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
        };

        config.validate()?;
        Ok(config)
    }

    /// Create a configuration for `model`, authenticating with `api_key`.
    ///
    /// - `api_key`: the Moonshot (Kimi) API key, never a URL.
    /// - `model`: the adapter's default model. It selects the timeout default (300 s for a
    ///   seeded reasoning model, 60 s otherwise); each request's own `LlmRequest.model`
    ///   decides what is sent on the wire.
    ///
    /// The endpoint is [`KIMI_DEFAULT_BASE_URL`] (replace it with
    /// [`with_base_url`](Self::with_base_url)) and the timeout is 300 s for a seeded reasoning
    /// model and 60 s otherwise (replace it with
    /// [`with_timeout_seconds`](Self::with_timeout_seconds)). Construction is infallible: an
    /// empty key or model, or a non-`http(s)` endpoint, is refused when the adapter is built.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_llm::kimi::KimiConfig;
    /// use paladin_llm::kimi::adapter::{KIMI_DEFAULT_BASE_URL, KIMI_DEFAULT_MODEL};
    ///
    /// let key = "kimi-example-key".to_string();
    /// let config = KimiConfig::new(key, KIMI_DEFAULT_MODEL.to_string()).with_timeout_seconds(600);
    /// assert_eq!(config.base_url, KIMI_DEFAULT_BASE_URL);
    /// assert_eq!(config.timeout_seconds, 600);
    /// ```
    pub fn new(api_key: String, model: String) -> Self {
        let timeout_seconds = default_timeout_seconds(KIMI_PROVIDER, &model, 60);
        Self {
            api_key,
            base_url: KIMI_DEFAULT_BASE_URL.to_string(),
            model,
            timeout_seconds,
            max_tokens: None,
            max_retries: DEFAULT_MAX_RETRIES,
            context_window: None,
            model_overrides: BTreeMap::new(),
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

config_knob_builders!(KimiConfig);

/// The provider name this adapter reports through [`LlmPort::get_provider_name`]
/// and that its engine stamps on every `LlmError::ProviderError` (Phase 25 D-03).
const KIMI_PROVIDER: &str = "kimi";

/// Kimi (Moonshot AI) LLM Adapter implementing [`LlmPort`].
///
/// Every method delegates to an owned [`CompatEngine`] (D-05) — this struct
/// carries no protocol logic of its own.
pub struct KimiAdapter {
    engine: CompatEngine,
}

impl KimiAdapter {
    /// Create a new Kimi adapter.
    ///
    /// # Errors
    /// Returns an error if configuration is invalid or the underlying HTTP
    /// client cannot be created.
    pub fn new(config: KimiConfig) -> Result<Self, LlmError> {
        config.validate().map_err(|e| {
            LlmError::AuthenticationError(format!("Invalid Kimi configuration: {}", e))
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
                supports_vision: false,
                supports_embeddings: false,
                // The default model's seeded window (`seeded::KIMI`, `kimi-k2.6`, verified
                // 2026-10-15: listing `context_length`), replacing the unevidenced 131,072.
                max_context_tokens: config
                    .context_window
                    .or(seeded_window(seeded::KIMI, KIMI_DEFAULT_MODEL)),
                supports_system_messages: true,
                // ADR-0004's framework default span `(0.0, 1.0)`, declared explicitly because
                // every shipped adapter must declare `Some(_)` (the `lib.rs` invariant
                // `every_new_provider_declares_a_temperature_range`). It states NO vendor
                // measurement: the vendor accepts one legal temperature per thinking mode (see
                // the `request_parameters` note below), and no admission reads this span while
                // `model_profile` returns `Some`, which `KimiAdapter` always does (the profile
                // declares `temperature` `NotSent`). It replaces the 2026-08-22 degenerate
                // `(1.0, 1.0)`, which the 2026-10-15 probe contradicted.
                temperature_range: Some((0.0, 1.0)),
            },
            // Phase 50 (ADR-0061, plan 50-20, closing G-50-5c): `temperature`, `top_p`,
            // `frequency_penalty` and `presence_penalty` are NOT carried on any Kimi model the
            // registry does not list, so `fallback_from_compat` below declares each `NotSent`,
            // exactly as the seeded `kimi-k2.6` and `kimi-k3` rows do. The vendor answers with a
            // single legal value for each, and that value depends on the thinking mode: on
            // 2026-08-22 `temperature` was "only 1 is allowed" and `top_p` "only 0.95"; on
            // 2026-10-15, with thinking disabled, `temperature` was "only 0.6 is allowed",
            // `top_p` "only 0.95" and both penalties "only 0". No mode-independent value can be
            // declared, so `NotSent` is the only truthful position: the vendor applies its own
            // legal value to an omitted field. An explicit value on such a model is refused at
            // build, and a framework default is omitted and recorded (never swapped for the legal
            // one, ADR-0004). A seeded entry or a `ModelOverride` still decides what is sent for
            // its own model (D-04a): only `max_tokens` is gated directly by its flag here. The
            // 2026-08-22 `Fixed(1.0)` / `Fixed(0.95)` declarations this replaces were
            // contradicted by the 2026-10-15 probe. `max_tokens` stays carried, so the
            // configured default output cap still reaches the wire.
            //
            // WR-02: every field stated explicitly. `..CompatRequestParameters::all()`
            // was a struct-update spread, which the type's own invariant
            // (`compat/engine.rs`) and plan 17-18's prohibition both forbid: the
            // point of having no `Default` and no spread is that a sixth
            // parameter becomes a COMPILE ERROR until each preset's author
            // states a position for it. A spread silently defaults the new
            // field to `true` here, which is exactly how the shipped Grok
            // preset came to send a parameter xAI rejects.
            request_parameters: CompatRequestParameters {
                temperature: false,
                top_p: false,
                max_tokens: true,
                frequency_penalty: false,
                presence_penalty: false,
            },
            fallback_models: KIMI_FALLBACK_MODELS.iter().map(|s| s.to_string()).collect(),
            // No usage-cap (quota) mapping for kimi: its quota-exhaustion body was never captured
            // (50-SEED-PROBE.md, owner reading R-2: accepted unverified 2026-10-16, plan 50-17),
            // so an override would be invented evidence (D-11a). The shared status mapping applies.
            error_override: None,
            // WR-04 (`17-REVIEW.md`, T-17-52/T-17-53), superseding the
            // 17-04 comment this replaces: `KIMI_DEFAULT_BASE_URL` is only
            // this preset's *default* — `MOONSHOT_BASE_URL` is documented
            // and operator-settable (`KimiConfig::from_env`), so a `3xx`
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
        let registry = ModelRegistry::new(KIMI_PROVIDER, seeded::KIMI, fallback)
            .with_context_window(config.context_window)
            .and_then(|registry| registry.with_overrides(config.model_overrides))
            .map_err(|e| {
                LlmError::AuthenticationError(format!("Invalid Kimi configuration: {e}"))
            })?;

        Ok(Self {
            // Phase 25 D-03: name the engine so every `ProviderError` it
            // emits carries "kimi", not the engine's generic default.
            engine: CompatEngine::new(engine_config)?
                .with_provider_name(KIMI_PROVIDER)
                .with_default_max_tokens(config.max_tokens)
                // G-50-5a (plan 50-24): no `with_usage_semantics` -- Kimi keeps the engine default
                // `CompatUsageSemantics::ReasoningInsideCompletion`. Live-measured 2026-10-18
                // (`50-USAGE-PROBE.md`, operator run on the raw Moonshot wire): kimi-k2.6
                // (27 + 147 = 174, reasoning 144) and kimi-k3 (105 + 158 = 263, reasoning 142)
                // both read `reasoning inside completion`.
                .with_model_registry(registry),
        })
    }
}

#[async_trait]
impl LlmPort for KimiAdapter {
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
        KIMI_PROVIDER
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
    use crate::registry::UNPROFILED_MODEL_METADATA_KEY;
    use crate::sampling::PARAMS_OMITTED_METADATA_KEY;
    use mockito::{Matcher, Server};
    use paladin_core::platform::container::prompt::{
        PromptItem, PromptParameters, PromptType, UserPrompt,
    };
    use paladin_ports::output::llm_port::FinishReason;
    use paladin_ports::output::model_profile::{ParamSupport, SamplingParam};
    use serde_json::json;

    /// WR-03: the documented `with_base_url` guarantee is an `http://` or `https://` URL. A value
    /// that merely starts with `http` (`httpx://x`, a scheme-less `http-gateway...`) must fail
    /// construction like any other non-HTTP value.
    #[test]
    fn a_builder_set_non_http_base_url_fails_kimi_construction() {
        for bad in [
            "ftp://x",
            "",
            "api.example.com/v1",
            "httpx://x",
            "http-gateway.internal/v1",
            "httpfoo:",
        ] {
            let config =
                KimiConfig::new("k".to_string(), "kimi-k2.6".to_string()).with_base_url(bad);
            assert!(
                config.validate().is_err(),
                "base URL {bad:?} must fail validation"
            );
            assert!(
                KimiAdapter::new(config).is_err(),
                "base URL {bad:?} must fail adapter construction"
            );
        }
        for good in ["http://localhost:1", "https://gateway.example/v1"] {
            let config =
                KimiConfig::new("k".to_string(), "kimi-k2.6".to_string()).with_base_url(good);
            assert!(
                config.validate().is_ok(),
                "base URL {good:?} must pass validation"
            );
        }
    }

    // ── KimiConfig::new (SURFACE-02, D-05) ──

    #[test]
    fn kimi_config_new_takes_key_and_model_and_defaults_the_endpoint() {
        let config = KimiConfig::new("k".to_string(), "kimi-k3".to_string());
        assert_eq!(config.base_url, KIMI_DEFAULT_BASE_URL);
        assert_eq!(config.model, "kimi-k3");
        assert_eq!(config.api_key, "k");
        // The timeout default is unchanged: a seeded reasoning model keeps its 300 s (D-13), an
        // unseeded one its 60 s.
        assert_eq!(config.timeout_seconds, 300);
        assert_eq!(
            config.timeout_seconds,
            default_timeout_seconds(KIMI_PROVIDER, "kimi-k3", 60)
        );
        let unseeded = KimiConfig::new("k".to_string(), "kimi-unseeded-model".to_string());
        assert_eq!(
            unseeded.timeout_seconds,
            default_timeout_seconds(KIMI_PROVIDER, "kimi-unseeded-model", 60)
        );
    }

    #[test]
    fn kimi_config_with_base_url_replaces_the_default_endpoint() {
        let config = KimiConfig::new("k".to_string(), "m".to_string())
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

    /// G-50-5a (plan 50-24): the live-measured kimi-k2.6 usage object (`50-USAGE-PROBE.md`,
    /// 2026-10-18: prompt 27, completion 147 of which reasoning 144, total 174) is read with the
    /// engine-default inclusive semantics -- `completion_tokens` copied as is -- and reconciles
    /// with Moonshot's own total. Read beside completion it would map to 291 and disagree.
    #[tokio::test]
    async fn kimi_measured_usage_is_read_inside_completion() {
        use crate::usage_reconcile::USAGE_UNRECONCILED_METADATA_KEY;

        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "id": "cmpl-kimi-measured",
                    "model": "kimi-k2.6",
                    "choices": [{
                        "index": 0,
                        "message": {"role": "assistant", "content": "391"},
                        "finish_reason": "stop"
                    }],
                    "usage": {
                        "prompt_tokens": 27,
                        "completion_tokens": 147,
                        "total_tokens": 174,
                        "completion_tokens_details": {"reasoning_tokens": 144}
                    }
                })
                .to_string(),
            )
            .create_async()
            .await;
        let adapter = KimiAdapter::new(
            KimiConfig::new("test-key".to_string(), "kimi-k2.6".to_string())
                .with_base_url(server.url()),
        )
        .unwrap();

        let response = adapter
            .generate(build_request("kimi-k2.6"))
            .await
            .expect("well-formed response");

        let usage = &response.usage;
        assert_eq!(usage.prompt_tokens, 27);
        assert_eq!(usage.completion_tokens, 147, "{usage:?}");
        assert_eq!(usage.reasoning_tokens, Some(144));
        assert_eq!(usage.total_tokens, 174);
        assert!(
            !response
                .metadata
                .contains_key(USAGE_UNRECONCILED_METADATA_KEY),
            "{:?}",
            response.metadata
        );
    }

    // ── KimiConfig::from_env() defaulting logic ──

    #[test]
    fn kimi_config_from_env_errors_when_api_key_absent() {
        let result = KimiConfig::from_parts(None, None, None, None);
        assert!(result.is_err());
    }

    #[test]
    fn kimi_config_defaults_base_url_and_model_when_only_key_is_set() {
        let config = KimiConfig::from_parts(Some("test-key".to_string()), None, None, None)
            .expect("key alone must be sufficient to build a valid config");
        assert_eq!(config.base_url, KIMI_DEFAULT_BASE_URL);
        assert_eq!(config.model, KIMI_DEFAULT_MODEL);
        // The default model is a seeded reasoning model, so its default timeout is the
        // reasoning-model 300 s (D-13), not the hosted-preset 60 s.
        assert_eq!(
            config.timeout_seconds,
            registry::REASONING_DEFAULT_TIMEOUT_SECONDS
        );
    }

    #[test]
    fn kimi_config_honors_explicit_overrides() {
        let config = KimiConfig::from_parts(
            Some("test-key".to_string()),
            Some("https://override.example/v1".to_string()),
            Some("moonshot-v1-32k".to_string()),
            Some("30".to_string()),
        )
        .unwrap();
        assert_eq!(config.base_url, "https://override.example/v1");
        assert_eq!(config.model, "moonshot-v1-32k");
        assert_eq!(config.timeout_seconds, 30);
    }

    // ── Request shaping / response parsing ──

    #[tokio::test]
    async fn generate_posts_to_chat_completions_with_auth_header_and_matching_body() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/chat/completions")
            .match_header("authorization", "Bearer test-key")
            .match_body(Matcher::PartialJson(json!({
                "model": KIMI_DEFAULT_MODEL,
                "stream": false
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "id": "cmpl-1",
                    "model": KIMI_DEFAULT_MODEL,
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

        let config = KimiConfig::new("test-key".to_string(), KIMI_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = KimiAdapter::new(config).unwrap();

        let response = adapter
            .generate(build_request(KIMI_DEFAULT_MODEL))
            .await
            .expect("mock server returned a well-formed response");

        assert_eq!(response.content, "Hi there");
        assert_eq!(response.usage.prompt_tokens, 5);
        assert_eq!(response.usage.completion_tokens, 3);
        assert_eq!(response.usage.total_tokens, 8);
        assert!(matches!(response.finish_reason, FinishReason::Stop));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn generate_computes_total_tokens_when_provider_omits_it() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body(
                json!({
                    "id": "cmpl-2",
                    "model": KIMI_DEFAULT_MODEL,
                    "choices": [{
                        "index": 0,
                        "message": {"role": "assistant", "content": "hi"},
                        "finish_reason": "stop"
                    }],
                    "usage": {"prompt_tokens": 10, "completion_tokens": 4}
                })
                .to_string(),
            )
            .create_async()
            .await;

        let config = KimiConfig::new("test-key".to_string(), KIMI_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = KimiAdapter::new(config).unwrap();

        let response = adapter
            .generate(build_request(KIMI_DEFAULT_MODEL))
            .await
            .unwrap();

        assert_eq!(
            response.usage.total_tokens, 14,
            "must compute prompt+completion when total_tokens is absent, never report zero"
        );
    }

    #[tokio::test]
    async fn finish_reason_length_maps_to_length() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body(
                json!({
                    "id": "cmpl-3",
                    "model": KIMI_DEFAULT_MODEL,
                    "choices": [{
                        "index": 0,
                        "message": {"role": "assistant", "content": "partial answer"},
                        "finish_reason": "length"
                    }],
                    "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
                })
                .to_string(),
            )
            .create_async()
            .await;

        let config = KimiConfig::new("test-key".to_string(), KIMI_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = KimiAdapter::new(config).unwrap();

        let response = adapter
            .generate(build_request(KIMI_DEFAULT_MODEL))
            .await
            .unwrap();

        assert!(matches!(response.finish_reason, FinishReason::Length));
    }

    #[tokio::test]
    async fn finish_reason_unrecognized_maps_to_error_never_silently_to_stop() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body(
                json!({
                    "id": "cmpl-4",
                    "model": KIMI_DEFAULT_MODEL,
                    "choices": [{
                        "index": 0,
                        "message": {"role": "assistant", "content": "answer"},
                        "finish_reason": "bogus"
                    }],
                    "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
                })
                .to_string(),
            )
            .create_async()
            .await;

        let config = KimiConfig::new("test-key".to_string(), KIMI_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = KimiAdapter::new(config).unwrap();

        let response = adapter
            .generate(build_request(KIMI_DEFAULT_MODEL))
            .await
            .unwrap();

        assert!(matches!(response.finish_reason, FinishReason::Error(_)));
    }

    #[tokio::test]
    async fn finish_reason_absent_maps_to_stop() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body(
                json!({
                    "id": "cmpl-5",
                    "model": KIMI_DEFAULT_MODEL,
                    "choices": [{
                        "index": 0,
                        "message": {"role": "assistant", "content": "answer"}
                    }],
                    "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
                })
                .to_string(),
            )
            .create_async()
            .await;

        let config = KimiConfig::new("test-key".to_string(), KIMI_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = KimiAdapter::new(config).unwrap();

        let response = adapter
            .generate(build_request(KIMI_DEFAULT_MODEL))
            .await
            .unwrap();

        assert!(matches!(response.finish_reason, FinishReason::Stop));
    }

    #[tokio::test]
    async fn null_message_content_is_an_empty_completion_not_a_parse_failure() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body(
                json!({
                    "id": "cmpl-6",
                    "model": KIMI_DEFAULT_MODEL,
                    "choices": [{
                        "index": 0,
                        "message": {"role": "assistant", "content": null},
                        "finish_reason": "stop"
                    }],
                    "usage": {"prompt_tokens": 1, "completion_tokens": 0, "total_tokens": 1}
                })
                .to_string(),
            )
            .create_async()
            .await;

        let config = KimiConfig::new("test-key".to_string(), KIMI_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = KimiAdapter::new(config).unwrap();

        // `null` content deserializes to an empty string (no parse failure, no panic) and the
        // shared guard then classifies the empty `stop` as an `EmptyCompletion` (D-11a; this
        // pinned `Ok("")` before Phase 50 plan 50-12).
        let result = adapter.generate(build_request(KIMI_DEFAULT_MODEL)).await;

        assert!(
            matches!(result, Err(LlmError::EmptyCompletion { .. })),
            "got {result:?}"
        );
    }

    #[tokio::test]
    async fn empty_choices_array_yields_empty_completion_error() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body(
                json!({
                    "id": "cmpl-7",
                    "model": KIMI_DEFAULT_MODEL,
                    "choices": [],
                    "usage": {"prompt_tokens": 1, "completion_tokens": 0, "total_tokens": 1}
                })
                .to_string(),
            )
            .create_async()
            .await;

        let config = KimiConfig::new("test-key".to_string(), KIMI_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = KimiAdapter::new(config).unwrap();

        let result = adapter.generate(build_request(KIMI_DEFAULT_MODEL)).await;

        assert!(matches!(result, Err(LlmError::EmptyCompletion { .. })));
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

        let config = KimiConfig::new("test-key".to_string(), KIMI_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = KimiAdapter::new(config).unwrap();

        let result = adapter.generate(build_request(KIMI_DEFAULT_MODEL)).await;
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

        let config = KimiConfig::new("test-key".to_string(), KIMI_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = KimiAdapter::new(config).unwrap();

        let result = adapter.generate(build_request(KIMI_DEFAULT_MODEL)).await;
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

        let config = KimiConfig::new("test-key".to_string(), KIMI_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = KimiAdapter::new(config).unwrap();

        let result = adapter.generate(build_request(KIMI_DEFAULT_MODEL)).await;
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

        let config = KimiConfig::new("test-key".to_string(), KIMI_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = KimiAdapter::new(config).unwrap();

        let result = adapter.generate(build_request(KIMI_DEFAULT_MODEL)).await;
        assert!(matches!(result, Err(LlmError::InvalidPrompt(_))));
    }

    /// Phase 25 (FT-FR-01, D-03): the successor to
    /// `http_500_maps_to_processing_error_carrying_status` — the status is
    /// read from `ProviderError`'s typed field, never from rendered text.
    #[tokio::test]
    async fn kimi_http_500_carries_a_typed_status() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(500)
            .with_body(r#"{"error":"boom"}"#)
            .expect_at_least(1)
            .create_async()
            .await;

        let config = KimiConfig::new("test-key".to_string(), KIMI_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = KimiAdapter::new(config).unwrap();

        let result = adapter.generate(build_request(KIMI_DEFAULT_MODEL)).await;
        match result {
            Err(LlmError::ProviderError {
                provider, status, ..
            }) => {
                assert_eq!(provider, "kimi");
                assert_eq!(status, 500);
            }
            other => panic!("expected ProviderError {{ status: 500 }}, got {other:?}"),
        }
    }

    /// Phase 25 (FT-FR-01, D-03): the preset's non-2xx path is the shared
    /// `map_http_status` — proven by comparing the adapter's error against
    /// the helper's own output for the same status, body and key.
    #[tokio::test]
    async fn kimi_non_2xx_routes_through_the_shared_mapper() {
        let body = r#"{"error":"overloaded"}"#;
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(503)
            .with_body(body)
            .expect_at_least(1)
            .create_async()
            .await;

        let config = KimiConfig::new("test-key".to_string(), KIMI_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = KimiAdapter::new(config).unwrap();

        let result = adapter.generate(build_request(KIMI_DEFAULT_MODEL)).await;
        let expected = map_http_status("kimi", 503, body, "test-key");
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

    #[tokio::test]
    async fn a_400_body_that_echoes_authorization_header_never_leaks_the_configured_key() {
        let secret = "sk-livekey-abcdef0123456789";
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(400)
            .with_body(format!(
                r#"{{"error":"bad gateway","request":{{"headers":{{"authorization":"Bearer {secret}"}}}}}}"#
            ))
            .create_async()
            .await;

        let config = KimiConfig::new(secret.to_string(), KIMI_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = KimiAdapter::new(config).unwrap();

        let result = adapter.generate(build_request(KIMI_DEFAULT_MODEL)).await;
        match result {
            Err(LlmError::InvalidPrompt(msg)) => {
                assert!(
                    !msg.contains(secret),
                    "diagnostic leaked the API key: {msg}"
                );
                assert!(
                    !msg.contains("livekey"),
                    "diagnostic leaked part of the API key: {msg}"
                );
            }
            other => panic!("expected InvalidPrompt, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn schema_mismatch_and_transport_failure_produce_different_error_messages() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body(r#"{"error":{"message":"model overloaded"}}"#)
            .create_async()
            .await;

        let config = KimiConfig::new("test-key".to_string(), KIMI_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = KimiAdapter::new(config).unwrap();

        let schema_result = adapter.generate(build_request(KIMI_DEFAULT_MODEL)).await;
        let schema_msg = match schema_result {
            Err(LlmError::ProcessingError(msg)) => msg,
            other => panic!("expected ProcessingError for schema mismatch, got {other:?}"),
        };
        assert!(
            schema_msg.contains("schema mismatch") || schema_msg.contains("model overloaded"),
            "schema-mismatch message must name the offending body: {schema_msg}"
        );

        // A transport failure: no server listening at this URL at all.
        let unreachable_config =
            KimiConfig::new("test-key".to_string(), KIMI_DEFAULT_MODEL.to_string())
                .with_base_url("http://127.0.0.1:1");
        let unreachable_adapter = KimiAdapter::new(unreachable_config).unwrap();
        let transport_result = unreachable_adapter
            .generate(build_request(KIMI_DEFAULT_MODEL))
            .await;
        let transport_msg = match transport_result {
            Err(LlmError::NetworkError(msg)) => msg,
            Err(LlmError::Timeout(msg)) => msg,
            other => panic!("expected a transport-classed error, got {other:?}"),
        };

        assert_ne!(
            schema_msg, transport_msg,
            "schema mismatch and transport failure must be distinguishable"
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

        let config = KimiConfig::new("test-key".to_string(), KIMI_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = KimiAdapter::new(config).unwrap();

        let stream = adapter
            .generate_stream(build_request(KIMI_DEFAULT_MODEL))
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

    #[tokio::test]
    async fn generate_stream_with_only_done_ends_in_an_empty_completion() {
        use futures::StreamExt;

        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body("data: [DONE]\n\n")
            .create_async()
            .await;

        let config = KimiConfig::new("test-key".to_string(), KIMI_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = KimiAdapter::new(config).unwrap();

        let stream = adapter
            .generate_stream(build_request(KIMI_DEFAULT_MODEL))
            .await
            .unwrap();
        let mut stream = Box::into_pin(stream);

        // A stream that carries no text is an empty completion on any stop reason (D-11a): the
        // `[DONE]`-only stream used to terminate with an `Ok` Stop chunk (pinned before Phase 50
        // plan 50-12); its terminal item is now the typed error and no Ok terminal chunk exists.
        let mut items = Vec::new();
        while let Some(item) = stream.next().await {
            items.push(item);
        }

        assert_eq!(items.len(), 1, "only the terminal item is emitted");
        assert!(
            matches!(items[0], Err(LlmError::EmptyCompletion { .. })),
            "got {:?}",
            items[0]
        );
    }

    // ── Capabilities / identity ──

    #[test]
    fn get_provider_name_returns_kimi() {
        let config = KimiConfig::new("test-key".to_string(), KIMI_DEFAULT_MODEL.to_string());
        let adapter = KimiAdapter::new(config).unwrap();
        assert_eq!(adapter.get_provider_name(), "kimi");
    }

    #[test]
    fn get_capabilities_reports_no_tool_or_function_calling() {
        let config = KimiConfig::new("test-key".to_string(), KIMI_DEFAULT_MODEL.to_string());
        let adapter = KimiAdapter::new(config).unwrap();
        let caps = adapter.get_capabilities();

        assert!(!caps.supports_tool_calling);
        assert!(!caps.supports_function_calling);
        assert!(!caps.supports_vision);
        assert!(!caps.supports_embeddings);
        assert!(caps.supports_streaming);
        assert!(caps.supports_system_messages);
    }

    // ── Declared temperature span (50-20, closing G-50-5c) ──

    #[test]
    fn get_capabilities_reports_the_framework_default_temperature_span() {
        // The cross-adapter invariant in `lib.rs`
        // (`every_new_provider_declares_a_temperature_range`) requires `Some(_)`; this pins the
        // exact value, ADR-0004's framework default. It states no vendor measurement.
        let config = KimiConfig::new("test-key".to_string(), KIMI_DEFAULT_MODEL.to_string());
        let adapter = KimiAdapter::new(config).unwrap();
        let caps = adapter.get_capabilities();

        assert_eq!(caps.temperature_range, Some((0.0, 1.0)));
    }

    #[test]
    fn fallback_models_is_non_empty_and_starts_with_the_default_model() {
        // The curated fallback cannot disagree with the default about which
        // model to reach for.
        assert!(!KIMI_FALLBACK_MODELS.is_empty());
        assert_eq!(KIMI_FALLBACK_MODELS[0], KIMI_DEFAULT_MODEL);
    }

    #[tokio::test]
    async fn generate_omits_temperature_from_the_moonshot_request_body() {
        // Option (a): Kimi's request path does not carry `temperature` at
        // all — asserted by capturing the real outgoing wire body, not by
        // reading the preset's declaration. `build_request`'s default
        // parameters (`PromptParameters::default()`) carry
        // `temperature: Some(0.7)`, so this proves the value is actively
        // dropped, not merely never supplied.
        use std::sync::{Arc, Mutex};

        let mut server = Server::new_async().await;
        let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let captured_clone = Arc::clone(&captured);
        server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body_from_request(move |req| {
                *captured_clone.lock().unwrap() =
                    Some(req.utf8_lossy_body().unwrap_or_default().into_owned());
                json!({
                    "id": "cmpl-notemp",
                    "model": KIMI_DEFAULT_MODEL,
                    "choices": [{
                        "index": 0,
                        "message": {"role": "assistant", "content": "ok"},
                        "finish_reason": "stop"
                    }],
                    "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
                })
                .to_string()
                .into_bytes()
            })
            .create_async()
            .await;

        let config = KimiConfig::new("test-key".to_string(), KIMI_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = KimiAdapter::new(config).unwrap();

        let result = adapter.generate(build_request(KIMI_DEFAULT_MODEL)).await;
        assert!(
            result.is_ok(),
            "mock returned a well-formed response: {result:?}"
        );
        let metadata = result.map(|r| r.metadata).unwrap_or_default();

        let body = captured
            .lock()
            .unwrap()
            .take()
            .expect("mock must have been called exactly once");
        assert!(
            !body.contains("temperature"),
            "the key name itself must not appear on the wire — got body: {body}"
        );
        // ADR-0061: the drop is no longer silent.
        assert!(
            metadata
                .get(PARAMS_OMITTED_METADATA_KEY)
                .is_some_and(|omitted| omitted.contains("temperature")),
            "the omitted default temperature must be recorded: {metadata:?}"
        );
    }

    /// A model id with no seeded entry: it runs on the adapter-wide fallback profile, which is
    /// what the fixed-value tests below describe. The default model has its own seeded entry
    /// (plan 50-06), covered by the `kimi_default_model_*` tests.
    const UNSEEDED_MODEL: &str = "kimi-unseeded-test-model";

    /// Run one request through a Kimi adapter on a mock endpoint; returns the wire body and the
    /// response metadata.
    async fn kimi_wire(
        parameters: PromptParameters,
    ) -> (serde_json::Value, std::collections::HashMap<String, String>) {
        let mut server = crate::compat::test_support::server().await;
        let captured = crate::compat::test_support::mock_completion(&mut server).await;
        let adapter = KimiAdapter::new(
            KimiConfig::new("test-key".to_string(), UNSEEDED_MODEL.to_string())
                .with_base_url(server.url()),
        )
        .unwrap();
        let mut request = build_request(UNSEEDED_MODEL);
        request.prompt.entity.data.parameters = parameters;
        let response = adapter
            .generate(request)
            .await
            .expect("well-formed response");
        let body = captured.lock().unwrap().take().expect("a request was sent");
        (body, response.metadata)
    }

    fn kimi_parameters(temperature: f32, top_p: f32) -> PromptParameters {
        PromptParameters {
            max_tokens: Some(64),
            temperature: Some(temperature),
            top_p: Some(top_p),
            frequency_penalty: None,
            presence_penalty: None,
            stop_sequences: None,
        }
    }

    #[test]
    fn kimi_fallback_matches_the_seeded_sampling_declarations() {
        // The adapter-wide fallback (every Kimi id the registry does not list) must declare
        // the four sampling parameters exactly as each seeded Kimi row does. Compared against
        // the seeded table itself, not against restated literals, so a drift in either place
        // fails here (G-50-5c, owner option (a)).
        let adapter = KimiAdapter::new(
            KimiConfig::new("test-key".to_string(), UNSEEDED_MODEL.to_string())
                .with_base_url("https://example.invalid/v1"),
        )
        .unwrap();
        let fallback = adapter
            .model_profile(UNSEEDED_MODEL)
            .expect("Kimi profiles every model");
        assert!(!seeded::KIMI.is_empty(), "the Kimi seeded table is empty");
        for entry in seeded::KIMI {
            let seeded_profile = (entry.build)();
            assert_eq!(
                fallback.temperature, seeded_profile.temperature,
                "temperature differs from the seeded {}",
                entry.model
            );
            assert_eq!(
                fallback.top_p, seeded_profile.top_p,
                "top_p differs from the seeded {}",
                entry.model
            );
            assert_eq!(
                fallback.frequency_penalty, seeded_profile.frequency_penalty,
                "frequency_penalty differs from the seeded {}",
                entry.model
            );
            assert_eq!(
                fallback.presence_penalty, seeded_profile.presence_penalty,
                "presence_penalty differs from the seeded {}",
                entry.model
            );
        }
    }

    #[test]
    fn kimi_fallback_profile_sends_no_sampling_values() {
        let adapter = KimiAdapter::new(
            KimiConfig::new("test-key".to_string(), UNSEEDED_MODEL.to_string())
                .with_base_url("https://example.invalid/v1"),
        )
        .unwrap();
        let profile = adapter
            .model_profile(UNSEEDED_MODEL)
            .expect("Kimi profiles every model");
        assert_eq!(profile.temperature, ParamSupport::NotSent);
        assert_eq!(profile.top_p, ParamSupport::NotSent);
        assert_eq!(profile.frequency_penalty, ParamSupport::NotSent);
        assert_eq!(profile.presence_penalty, ParamSupport::NotSent);
        assert_eq!(profile.stop_sequences, StopSupport::Supported { max: None });
        for param in [
            SamplingParam::Temperature,
            SamplingParam::TopP,
            SamplingParam::FrequencyPenalty,
            SamplingParam::PresencePenalty,
        ] {
            assert_eq!(
                profile.support_for(param),
                Some(ParamSupport::NotSent),
                "{param:?}"
            );
        }
    }

    #[tokio::test]
    async fn unseeded_kimi_body_carries_no_sampling_values() {
        // `PromptParameters::default()` sets temperature, top_p and both penalties; none of the
        // four may reach the wire on a model the registry does not list, while the configured
        // output cap still does (a targeted omission, not a dropped body).
        let (body, metadata) = kimi_wire(PromptParameters::default()).await;
        for key in [
            "temperature",
            "top_p",
            "frequency_penalty",
            "presence_penalty",
        ] {
            assert!(body.get(key).is_none(), "{key} reached the wire: {body}");
        }
        assert!(body.get("max_tokens").is_some(), "{body}");
        assert!(
            metadata.contains_key(PARAMS_OMITTED_METADATA_KEY),
            "the omitted defaults must be recorded: {metadata:?}"
        );
    }

    #[tokio::test]
    async fn kimi_explicit_sampling_values_on_an_unseeded_model_are_omitted_and_recorded() {
        // Even the vendor's once-measured legal values are not sent: the legal value depends on
        // the thinking mode, so the vendor applies its own to an omitted field.
        let (body, metadata) = kimi_wire(kimi_parameters(1.0, 0.95)).await;
        assert!(body.get("temperature").is_none(), "{body}");
        assert!(body.get("top_p").is_none(), "{body}");
        let omitted = metadata
            .get(PARAMS_OMITTED_METADATA_KEY)
            .expect("the omitted values are recorded");
        assert!(omitted.contains("temperature"), "{omitted}");
        assert!(omitted.contains("top_p"), "{omitted}");
    }

    #[test]
    fn kimi_default_model_is_seeded() {
        let adapter = KimiAdapter::new(
            KimiConfig::new("test-key".to_string(), KIMI_DEFAULT_MODEL.to_string())
                .with_base_url("https://example.invalid/v1"),
        )
        .unwrap();
        // A seeded entry stamps its date on the profile; the adapter-wide fallback carries none.
        let profile = adapter
            .model_profile(KIMI_DEFAULT_MODEL)
            .expect("Kimi profiles every model");
        assert_eq!(profile.verified_on.as_deref(), Some("2026-10-15"));
        assert_eq!(profile.context_window, Some(262_144));
        assert_eq!(
            adapter.get_capabilities().max_context_tokens,
            profile.context_window,
            "the declared window is the default model's seeded window"
        );
    }

    #[tokio::test]
    async fn kimi_default_model_omits_every_sampling_parameter_the_probe_rejected() {
        let mut server = crate::compat::test_support::server().await;
        let captured = crate::compat::test_support::mock_completion(&mut server).await;
        let adapter = KimiAdapter::new(
            KimiConfig::new("test-key".to_string(), KIMI_DEFAULT_MODEL.to_string())
                .with_base_url(server.url()),
        )
        .unwrap();
        let mut request = build_request(KIMI_DEFAULT_MODEL);
        request.prompt.entity.data.parameters = kimi_parameters(0.7, 0.9);
        let response = adapter
            .generate(request)
            .await
            .expect("well-formed response");
        let body = captured.lock().unwrap().take().expect("a request was sent");
        // The 2026-10-15 probe rejected every temperature and top_p value on kimi-k2.6.
        assert!(body.get("temperature").is_none(), "{body}");
        assert!(body.get("top_p").is_none(), "{body}");
        assert_eq!(
            response
                .metadata
                .get(PARAMS_OMITTED_METADATA_KEY)
                .map(String::as_str),
            Some("temperature,top_p")
        );
        assert!(
            !response
                .metadata
                .contains_key(UNPROFILED_MODEL_METADATA_KEY),
            "the default model is profiled"
        );
    }

    #[test]
    fn kimi_default_model_is_the_operator_identified_current_model() {
        assert_eq!(KIMI_DEFAULT_MODEL, "kimi-k2.6");
        assert_eq!(KIMI_FALLBACK_MODELS[0], KIMI_DEFAULT_MODEL);
        assert!(KIMI_FALLBACK_MODELS.contains(&"kimi-k3"));
    }

    // ── Model list: memoization / concurrency (D-13/D-14) ──

    #[tokio::test]
    async fn concurrent_get_available_models_calls_result_in_exactly_one_http_request() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/models")
            .with_status(200)
            .with_body(
                json!({"data": [{"id": KIMI_DEFAULT_MODEL}, {"id": "moonshot-v1-32k"}]})
                    .to_string(),
            )
            .expect(1)
            .create_async()
            .await;

        let config = KimiConfig::new("test-key".to_string(), KIMI_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = KimiAdapter::new(config).unwrap();

        let (a, b) = tokio::join!(
            adapter.get_available_models(),
            adapter.get_available_models()
        );

        assert!(a.is_ok());
        assert!(b.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn get_available_models_falls_back_to_curated_list_when_live_fetch_fails() {
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/models")
            .with_status(500)
            .with_body(r#"{"error":"unavailable"}"#)
            .create_async()
            .await;

        let config = KimiConfig::new("test-key".to_string(), KIMI_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = KimiAdapter::new(config).unwrap();

        let models = adapter.get_available_models().await.unwrap();
        assert_eq!(
            models,
            KIMI_FALLBACK_MODELS
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        );
    }

    #[tokio::test]
    async fn validate_model_accepts_a_model_from_the_curated_fallback_list() {
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/models")
            .with_status(500)
            .with_body(r#"{"error":"unavailable"}"#)
            .create_async()
            .await;

        let config = KimiConfig::new("test-key".to_string(), KIMI_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = KimiAdapter::new(config).unwrap();

        assert!(adapter.validate_model(KIMI_DEFAULT_MODEL).await.unwrap());
        assert!(!adapter.validate_model("no-such-model").await.unwrap());
    }

    // ── Retry behavior ──

    #[tokio::test]
    async fn non_retryable_auth_error_returns_after_exactly_one_attempt() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/chat/completions")
            .with_status(401)
            .with_body(r#"{"error":"invalid key"}"#)
            .expect(1)
            .create_async()
            .await;

        let config = KimiConfig::new("test-key".to_string(), KIMI_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = KimiAdapter::new(config).unwrap();

        let _ = adapter.generate(build_request(KIMI_DEFAULT_MODEL)).await;
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn retryable_server_error_retries_up_to_max_retries_then_returns_last_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/chat/completions")
            .with_status(500)
            .with_body(r#"{"error":"boom"}"#)
            .expect(4)
            .create_async()
            .await;

        let config = KimiConfig::new("test-key".to_string(), KIMI_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = KimiAdapter::new(config).unwrap();

        let result = adapter.generate(build_request(KIMI_DEFAULT_MODEL)).await;
        assert!(result.is_err());
        mock.assert_async().await;
    }

    // ── WR-04: redirect-following credential replay (plan 17-10) ──

    #[tokio::test]
    async fn kimi_does_not_replay_the_authorization_header_to_a_redirect_target() {
        // A 302 to a POST is downgraded to a bodyless GET by the redirect
        // layer (RFC 7231 6.4.2/6.4.3, as implemented by tower-http's
        // follow_redirect — the layer reqwest's default policy runs on),
        // so the redirect target's mock matches "GET", not "POST".
        let mut redirect_target = Server::new_async().await;
        let redirect_target_mock = redirect_target
            .mock("GET", Matcher::Any)
            .match_query(Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "id": "cmpl-redirect",
                    "model": KIMI_DEFAULT_MODEL,
                    "choices": [{
                        "index": 0,
                        "message": {"role": "assistant", "content": "should never be seen"},
                        "finish_reason": "stop"
                    }],
                    "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
                })
                .to_string(),
            )
            .expect(0)
            .create_async()
            .await;

        let mut primary = Server::new_async().await;
        let primary_mock = primary
            .mock("POST", "/chat/completions")
            .with_status(302)
            .with_header(
                "location",
                &format!("{}/chat/completions", redirect_target.url()),
            )
            // The refused-redirect arm's error (`ProviderError { 3xx }`
            // since Phase 25 D-03; `ProcessingError` before) is outside the
            // engine's non-retryable set, so the fixed engine may hit
            // `primary` more than once (up to `max_retries + 1` = 4);
            // before the fix, the redirect was followed transparently and
            // the call succeeded on the first attempt. `expect_at_least(1)`
            // holds in both the RED and GREEN states.
            .expect_at_least(1)
            .create_async()
            .await;

        let config = KimiConfig::new(
            "test-key-abc123".to_string(),
            KIMI_DEFAULT_MODEL.to_string(),
        )
        .with_base_url(primary.url());
        let adapter = KimiAdapter::new(config).unwrap();

        let result = adapter.generate(build_request(KIMI_DEFAULT_MODEL)).await;

        // Load-bearing assertions FIRST (D-00e): today the default redirect
        // policy follows the 302 and the redirect target answers with a
        // well-formed response, so `result` is actually `Ok` in the RED
        // state — it is this `.expect(0)` mock assertion, proving the
        // credential-bearing request WAS forwarded, that fails in the RED
        // state, not the `result.is_err()` check below.
        redirect_target_mock.assert_async().await;
        primary_mock.assert_async().await;

        assert!(
            result.is_err(),
            "a refused redirect must surface as an error, got: {result:?}"
        );
        let message = result.unwrap_err().to_string();
        assert!(
            message.contains("redirect"),
            "refused-redirect error must name the redirect, got: {message}"
        );
    }

    // ── Shared conformance suite (D-19, plan 31-04) ──
    //
    // Kimi delegates every method to `CompatEngine` (D-05), so it speaks the identical
    // OpenAI-compatible wire shape `openai_compatible`/`ollama` already instantiate the suite
    // against; bodies below mirror those fixtures' shape exactly (distinct model/provider names
    // only).
    mod conformance_suite {
        use super::*;
        use std::sync::Arc;

        struct KimiFixture;

        impl crate::conformance::ConformanceFixture for KimiFixture {
            const WIRE: crate::conformance::Wire = crate::conformance::Wire::OpenAiChat;

            fn default_model() -> &'static str {
                KIMI_DEFAULT_MODEL
            }

            fn provider_name() -> &'static str {
                "kimi"
            }

            fn adapter(base_url: &str) -> Arc<dyn LlmPort> {
                let config =
                    KimiConfig::new("test-key".to_string(), KIMI_DEFAULT_MODEL.to_string())
                        .with_base_url(base_url);
                Arc::new(KimiAdapter::new(config).expect("test config must build"))
            }

            fn success_body() -> String {
                json!({
                    "id": "cmpl-1",
                    "model": KIMI_DEFAULT_MODEL,
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

            // Moonshot is the one OpenAI-chat vendor that also reports a cache-write figure,
            // `prompt_tokens_details.cache_write_tokens` (D-11; `50-SEED-PROBE.md`).
            fn usage_body() -> String {
                json!({
                    "id": "cmpl-1",
                    "model": KIMI_DEFAULT_MODEL,
                    "choices": [{
                        "index": 0,
                        "message": {"role": "assistant", "content": "Hi there"},
                        "finish_reason": "stop"
                    }],
                    "usage": {
                        "prompt_tokens": 100,
                        "completion_tokens": 40,
                        "total_tokens": 140,
                        "prompt_tokens_details": {"cached_tokens": 30, "cache_write_tokens": 20},
                        "completion_tokens_details": {"reasoning_tokens": 15}
                    }
                })
                .to_string()
            }

            fn expected_usage() -> crate::conformance::ExpectedUsage {
                crate::conformance::ExpectedUsage {
                    cache_write: Some(20),
                    ..crate::conformance::wire_defaults::expected_usage(
                        crate::conformance::Wire::OpenAiChat,
                    )
                }
            }

            fn error_body(status: u16) -> String {
                json!({"error": format!("mock error for status {status}")}).to_string()
            }
        }

        crate::llm_conformance_suite!(KimiFixture);
    }

    // ── PARITY-03 knobs (plan 50-08, D-12) ──

    fn kimi_config(base_url: &str, model: &str) -> KimiConfig {
        KimiConfig::new("test-key".to_string(), model.to_string()).with_base_url(base_url)
    }

    fn window_override(window: u32) -> ModelOverride {
        ModelOverride {
            context_window: Some(window),
            ..ModelOverride::default()
        }
    }

    fn kimi_error(config: KimiConfig) -> String {
        match KimiAdapter::new(config) {
            Err(error) => error.to_string(),
            Ok(_) => panic!("construction must fail"),
        }
    }

    #[tokio::test]
    async fn kimi_max_tokens_default_reaches_the_wire() {
        use crate::compat::test_support::{mock_completion, send_with_max_tokens, server};

        let mut server = server().await;
        let captured = mock_completion(&mut server).await;
        let adapter =
            KimiAdapter::new(kimi_config(&server.url(), UNSEEDED_MODEL).with_max_tokens(2048))
                .unwrap();

        let body = send_with_max_tokens(&adapter, &captured, UNSEEDED_MODEL, None)
            .await
            .unwrap();
        assert_eq!(body["max_tokens"], 2048, "{body}");
        // A request's own value always wins over the configured default.
        let body = send_with_max_tokens(&adapter, &captured, UNSEEDED_MODEL, Some(512))
            .await
            .unwrap();
        assert_eq!(body["max_tokens"], 512, "{body}");

        // No configured default: no `max_tokens` key, as before PARITY-03.
        let plain = KimiAdapter::new(kimi_config(&server.url(), UNSEEDED_MODEL)).unwrap();
        let body = send_with_max_tokens(&plain, &captured, UNSEEDED_MODEL, None)
            .await
            .unwrap();
        assert!(body.get("max_tokens").is_none(), "{body}");
    }

    #[tokio::test]
    async fn kimi_max_retries_is_read_by_the_engine() {
        for retries in [0_u32, 1] {
            let server = crate::compat::test_support::server().await;
            let adapter = KimiAdapter::new(
                kimi_config(&server.url(), KIMI_DEFAULT_MODEL).with_max_retries(retries),
            )
            .unwrap();
            crate::compat::test_support::attempts_against_a_failing_endpoint(
                server,
                &adapter,
                KIMI_DEFAULT_MODEL,
                retries as usize + 1,
            )
            .await;
        }
    }

    #[test]
    fn kimi_context_window_override_is_declared() {
        let adapter = KimiAdapter::new(
            kimi_config("https://example.invalid/v1", UNSEEDED_MODEL).with_context_window(500_000),
        )
        .unwrap();
        assert_eq!(adapter.get_capabilities().max_context_tokens, Some(500_000));
        // The fallback profile (an unprofiled model) carries the same window.
        let profile = adapter.model_profile(UNSEEDED_MODEL).unwrap();
        assert_eq!(profile.context_window, Some(500_000));
    }

    #[test]
    fn context_window_override_beats_a_seeded_window() {
        use crate::window::effective_capabilities;

        let config = || kimi_config("https://example.invalid/v1", "kimi-k3");

        // The seeded fact, with no knob.
        let seeded_only = KimiAdapter::new(config()).unwrap();
        assert_eq!(
            seeded_only.model_profile("kimi-k3").unwrap().context_window,
            Some(1_048_576)
        );

        // The operator's adapter-wide statement beats the compiled seeded fact ...
        let adapter = KimiAdapter::new(config().with_context_window(500_000)).unwrap();
        assert_eq!(
            adapter.model_profile("kimi-k3").unwrap().context_window,
            Some(500_000)
        );
        assert_eq!(
            effective_capabilities(&adapter, "kimi-k3").max_context_tokens,
            Some(500_000)
        );

        // ... and a per-model override beats both, for that model only.
        let adapter = KimiAdapter::new(config().with_context_window(500_000).with_model_overrides(
            BTreeMap::from([("kimi-k3".to_string(), window_override(900_000))]),
        ))
        .unwrap();
        assert_eq!(
            adapter.model_profile("kimi-k3").unwrap().context_window,
            Some(900_000)
        );
        assert_eq!(
            effective_capabilities(&adapter, "kimi-k3").max_context_tokens,
            Some(900_000)
        );
        assert_eq!(
            adapter.model_profile("kimi-k2.6").unwrap().context_window,
            Some(500_000)
        );
    }

    #[test]
    fn kimi_model_override_wins_and_invalid_override_fails_construction() {
        let with_override = |over: ModelOverride| {
            kimi_config("https://example.invalid/v1", "kimi-k3")
                .with_model_overrides(BTreeMap::from([("kimi-k3".to_string(), over)]))
        };

        let adapter = KimiAdapter::new(with_override(window_override(900_000))).unwrap();
        assert_eq!(
            adapter.model_profile("kimi-k3").unwrap().context_window,
            Some(900_000)
        );

        let invalid = [
            window_override(0),
            ModelOverride {
                max_output_tokens: Some(2_000_000),
                ..ModelOverride::default()
            },
            ModelOverride {
                temperature: Some(ParamSupport::Range(1.0, 0.0)),
                ..ModelOverride::default()
            },
        ];
        for over in invalid {
            let message = kimi_error(with_override(over));
            assert!(
                message.contains("kimi") && message.contains("kimi-k3"),
                "the error must name the provider and the model: {message}"
            );
        }
    }

    #[tokio::test]
    async fn compat_max_tokens_default_is_checked_against_the_profile() {
        use crate::compat::test_support::{mock_completion, send_with_max_tokens, server};

        let mut server = server().await;
        let captured = mock_completion(&mut server).await;
        let over = ModelOverride {
            max_output_tokens: Some(1000),
            ..ModelOverride::default()
        };
        let adapter = KimiAdapter::new(
            kimi_config(&server.url(), "kimi-k3")
                .with_max_tokens(2048)
                .with_model_overrides(BTreeMap::from([("kimi-k3".to_string(), over)])),
        )
        .unwrap();

        // The configured default is checked exactly as a request's value is: refused, not
        // clamped, before any HTTP call.
        let Err(LlmError::InvalidPrompt(message)) =
            send_with_max_tokens(&adapter, &captured, "kimi-k3", None).await
        else {
            panic!("a default above the model's maximum output must be InvalidPrompt");
        };
        for needle in ["max_tokens", "2048", "1000"] {
            assert!(
                message.contains(needle),
                "'{needle}' missing from: {message}"
            );
        }
        assert!(captured.lock().unwrap().is_none(), "nothing was sent");

        // A request value inside the limit still goes through.
        let body = send_with_max_tokens(&adapter, &captured, "kimi-k3", Some(1000))
            .await
            .unwrap();
        assert_eq!(body["max_tokens"], 1000, "{body}");
    }

    #[test]
    fn kimi_config_defaults_a_reasoning_model_to_300s() {
        // (model, expected default timeout): seeded reasoning models get 300 s (D-13), a seeded
        // non-reasoning or unknown model keeps today's 60 s.
        for (model, expected) in [
            (KIMI_DEFAULT_MODEL, 300),
            ("kimi-k3", 300),
            (UNSEEDED_MODEL, 60),
        ] {
            let new = kimi_config("https://example.invalid/v1", model);
            assert_eq!(new.timeout_seconds, expected, "new({model})");
            let parts =
                KimiConfig::from_parts(Some("k".to_string()), None, Some(model.to_string()), None)
                    .unwrap();
            assert_eq!(parts.timeout_seconds, expected, "from_parts({model})");
        }
        // An explicit value always wins, whatever the model.
        for explicit in ["30", "60", "900"] {
            let config = KimiConfig::from_parts(
                Some("k".to_string()),
                None,
                Some("kimi-k3".to_string()),
                Some(explicit.to_string()),
            )
            .unwrap();
            assert_eq!(config.timeout_seconds.to_string(), explicit);
        }
        // An explicit field assignment wins too.
        let mut config = kimi_config("https://example.invalid/v1", "kimi-k3");
        config.timeout_seconds = 45;
        assert_eq!(config.timeout_seconds, 45);
    }

    #[test]
    fn kimi_config_debug_redacts_the_api_key() {
        let config = KimiConfig::new("sk-secret-kimi-key-123".to_string(), "kimi-k3".to_string())
            .with_base_url("https://api.example.invalid/v1");
        let debug = format!("{config:?}");
        assert!(!debug.contains("sk-secret-kimi-key-123"), "{debug}");
        assert!(debug.contains("<redacted>"), "{debug}");
        // Every other field is still printed.
        for needle in [
            "https://api.example.invalid/v1",
            "kimi-k3",
            "timeout_seconds",
        ] {
            assert!(debug.contains(needle), "'{needle}' missing from {debug}");
        }
        // An empty key is printed as empty, not as a redaction marker.
        let empty = format!(
            "{:?}",
            KimiConfig::new(String::new(), "m".into()).with_base_url("https://x")
        );
        assert!(!empty.contains("<redacted>"), "{empty}");
    }
}
