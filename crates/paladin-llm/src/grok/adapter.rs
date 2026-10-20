//! Grok (xAI) LLM Adapter.
//!
//! A thin preset (D-01, D-05, D-12) sitting entirely on
//! [`crate::compat::CompatEngine`] — the shared OpenAI-compatible protocol
//! engine. This adapter supplies only Grok's `base_url`, credential env var,
//! default model, curated fallback model list and capabilities block; every
//! other behaviour (request shaping, retry, streaming, error mapping,
//! credential redaction, memoized model-list resolution) is inherited from
//! the engine unchanged.

use async_trait::async_trait;
use futures::Stream;
use std::collections::BTreeMap;
use std::env;

use paladin_ports::output::llm_port::{
    LlmError, LlmPort, LlmRequest, LlmResponse, ProviderCapabilities, StreamingResponse,
};

use paladin_ports::output::model_profile::{ModelProfile, StopSupport};

use crate::compat::engine::CompatUsageSemantics;
use crate::compat::{
    CompatCapabilities, CompatEngine, CompatEngineConfig, CompatRequestParameters,
};
use crate::redaction::RedactedKey;
use crate::registry::{
    DEFAULT_MAX_RETRIES, ModelOverride, ModelRegistry, config_knob_builders,
    default_timeout_seconds, fallback_from_compat, is_http_url, seeded, seeded_window,
};

/// Default Grok (xAI) API base URL.
///
/// Live-verified 2026-08-22 (17-18, D-00e): `GET {this}/models` returned
/// HTTP 200 with a live model catalog against this exact URL. Superseding
/// the earlier `[CITED: docs.x.ai]` / "not live-verified" note, which
/// predated this execution environment having network egress.
pub const GROK_DEFAULT_BASE_URL: &str = "https://api.x.ai/v1";

/// Default Grok model requested when `XAI_MODEL` is unset.
///
/// `grok-4.3`: the operator-identified current general-purpose model (its cheapest current
/// tier), measured end to end by the 2026-10-15 live probe (`registry/seeded.rs`, `verified_on`
/// 2026-10-15). It replaces `grok-4.6`, which stays available and seeded; pin `XAI_MODEL` to
/// keep it.
///
/// History (17-18, D-00e, 2026-08-22): the earlier `grok-4` default was absent from the live
/// catalog; the dated `grok-4.20-*` family is excluded from the defaults by the plan's
/// prohibition on dated snapshot identifiers.
pub const GROK_DEFAULT_MODEL: &str = "grok-4.3";

/// Curated fallback model list (D-13), returned when the live `/models`
/// endpoint fails, is unreachable, or returns an empty list. Never reported
/// as authoritative — see [`crate::compat::engine::CompatEngine::available_models`].
///
/// Live-verified 2026-08-22 (17-18, D-00e) and re-measured 2026-10-15: the
/// three stable general-purpose entries from the live catalog, the default
/// first, then the other two newest first — `grok-4.6` (created
/// 2026-08-06), `grok-4.5` (created 2026-06-29) and `grok-4.3` (created
/// 2026-04-17). Excludes the image/video/build-tool models
/// (`grok-imagine-*`, `grok-build-0.1`) and the dated `grok-4.20-*`
/// family, none of which are the general-purpose chat line this preset
/// targets.
pub const GROK_FALLBACK_MODELS: &[&str] = &[GROK_DEFAULT_MODEL, "grok-4.6", "grok-4.5"];

/// Configuration for the Grok (xAI) adapter.
#[derive(Clone)]
#[non_exhaustive]
pub struct GrokConfig {
    /// API key for Grok (xAI) authentication.
    pub api_key: String,
    /// Base URL for the Grok API.
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
impl std::fmt::Debug for GrokConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GrokConfig")
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

impl GrokConfig {
    /// Load configuration from environment variables.
    ///
    /// # Environment Variables
    /// - `XAI_API_KEY` (required): Grok (xAI) API key.
    /// - `XAI_BASE_URL` (optional): API base URL, defaults to
    ///   [`GROK_DEFAULT_BASE_URL`].
    /// - `XAI_MODEL` (optional): Default model, defaults to
    ///   [`GROK_DEFAULT_MODEL`].
    /// - `XAI_TIMEOUT_SECONDS` (optional): Request timeout, defaults to
    ///   `60`.
    ///
    /// # Errors
    /// Returns an error if `XAI_API_KEY` is absent, or another variable's
    /// value fails to parse or validate.
    pub fn from_env() -> Result<Self, String> {
        Self::from_parts(
            env::var("XAI_API_KEY").ok(),
            env::var("XAI_BASE_URL").ok(),
            env::var("XAI_MODEL").ok(),
            env::var("XAI_TIMEOUT_SECONDS").ok(),
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
            api_key.ok_or_else(|| "XAI_API_KEY environment variable not set".to_string())?;
        let base_url = base_url.unwrap_or_else(|| GROK_DEFAULT_BASE_URL.to_string());
        let model = model.unwrap_or_else(|| GROK_DEFAULT_MODEL.to_string());
        let timeout_seconds = match timeout_seconds {
            Some(value) => value
                .parse()
                .map_err(|_| "Invalid XAI_TIMEOUT_SECONDS value".to_string())?,
            None => default_timeout_seconds(GROK_PROVIDER, &model, 60),
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
    /// - `api_key`: the xAI API key, never a URL.
    /// - `model`: the adapter's default model. It selects the timeout default (300 s for a
    ///   seeded reasoning model, 60 s otherwise); each request's own `LlmRequest.model`
    ///   decides what is sent on the wire.
    ///
    /// The endpoint is [`GROK_DEFAULT_BASE_URL`] (replace it with
    /// [`with_base_url`](Self::with_base_url)) and the timeout is 300 s for a seeded reasoning
    /// model and 60 s otherwise (replace it with
    /// [`with_timeout_seconds`](Self::with_timeout_seconds)). Construction is infallible: an
    /// empty key or model, or a non-`http(s)` endpoint, is refused when the adapter is built.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_llm::grok::GrokConfig;
    /// use paladin_llm::grok::adapter::{GROK_DEFAULT_BASE_URL, GROK_DEFAULT_MODEL};
    ///
    /// let key = "grok-example-key".to_string();
    /// let config = GrokConfig::new(key, GROK_DEFAULT_MODEL.to_string()).with_timeout_seconds(600);
    /// assert_eq!(config.base_url, GROK_DEFAULT_BASE_URL);
    /// assert_eq!(config.timeout_seconds, 600);
    /// ```
    pub fn new(api_key: String, model: String) -> Self {
        let timeout_seconds = default_timeout_seconds(GROK_PROVIDER, &model, 60);
        Self {
            api_key,
            base_url: GROK_DEFAULT_BASE_URL.to_string(),
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

config_knob_builders!(GrokConfig);

/// The provider name this adapter reports through [`LlmPort::get_provider_name`]
/// and that its engine stamps on every `LlmError::ProviderError` (Phase 25 D-03).
const GROK_PROVIDER: &str = "grok";

/// Grok (xAI) LLM Adapter implementing [`LlmPort`].
///
/// Every method delegates to an owned [`CompatEngine`] (D-05) — this struct
/// carries no protocol logic of its own.
pub struct GrokAdapter {
    engine: CompatEngine,
}

impl GrokAdapter {
    /// Create a new Grok adapter.
    ///
    /// # Errors
    /// Returns an error if configuration is invalid or the underlying HTTP
    /// client cannot be created.
    pub fn new(config: GrokConfig) -> Result<Self, LlmError> {
        config.validate().map_err(|e| {
            LlmError::AuthenticationError(format!("Invalid Grok configuration: {}", e))
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
                // The default model's seeded window (`seeded::GROK`, `grok-4.3`, verified
                // 2026-10-15: listing `context_length`), replacing the unevidenced 131,072.
                max_context_tokens: config
                    .context_window
                    .or(seeded_window(seeded::GROK, GROK_DEFAULT_MODEL)),
                supports_system_messages: true,
                temperature_range: Some((0.0, 2.0)),
            },
            // Measured against live `api.x.ai` on 2026-08-22 (17-18, D-00e,
            // closing G-17-4a) — this is the actual defect fix, not an
            // assumption. Each request below carried exactly one optional
            // sampling parameter, `model: "grok-4.6"`, no other parameter:
            //   temperature:0.7        -> HTTP 200, real completion (ACCEPTED)
            //   max_tokens:16          -> HTTP 200, real completion (ACCEPTED)
            //   top_p:1.0              -> HTTP 200, real completion (ACCEPTED)
            //   frequency_penalty:0.0  -> HTTP 400 {"code":"invalid-argument",
            //     "error":"Model grok-4.6 does not support parameter
            //     frequencyPenalty."}                          (REJECTED)
            //   presence_penalty:0.0   -> HTTP 400 {"code":"invalid-argument",
            //     "error":"Model grok-4.6 does not support parameter
            //     presencePenalty."}                           (REJECTED)
            // `frequency_penalty` was recorded UNTESTED in `17-UAT.md` and is
            // measured here on its own — its rejection is NOT inferred from
            // `presence_penalty`'s. A follow-up request carrying only the
            // three ACCEPTED parameters together (temperature, max_tokens,
            // top_p) also returned HTTP 200, confirming the combination this
            // declaration now produces on the wire.
            request_parameters: CompatRequestParameters {
                temperature: true,
                max_tokens: true,
                top_p: true,
                frequency_penalty: false,
                presence_penalty: false,
            },
            fallback_models: GROK_FALLBACK_MODELS.iter().map(|s| s.to_string()).collect(),
            // No usage-cap (quota) mapping for grok: its quota-exhaustion body was never captured
            // (50-SEED-PROBE.md, owner reading R-2: accepted unverified 2026-10-16, plan 50-17),
            // so an override would be invented evidence (D-11a). The shared status mapping applies.
            error_override: None,
            // WR-04 (`17-REVIEW.md`, T-17-52/T-17-53), superseding the
            // 17-04 comment this replaces: `GROK_DEFAULT_BASE_URL` is only
            // this preset's *default* — `XAI_BASE_URL` is documented and
            // operator-settable (`GrokConfig::from_env`), so a `3xx` from
            // whatever host it resolves to could otherwise replay the
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
        // Stop sequences: `NotSent` -- xAI's reasoning models reject `stop` (RESEARCH A5, the
        // handoff's G-10 note), so the wire omits `stop` and records the omission while the
        // execution service still enforces a stop-word agent's list client-side. A model that does take it is corrected by its
        // seeded entry (plan 50-06), never by this adapter (D-04a).
        let fallback = fallback_from_compat(
            &engine_config.capabilities.clone().into(),
            &engine_config.request_parameters,
            StopSupport::NotSent,
        );

        // PARITY-03: the operator's adapter-wide window, then the per-model overrides, are laid
        // over the seeded table; either one failing construction names the provider and model.
        let registry = ModelRegistry::new(GROK_PROVIDER, seeded::GROK, fallback)
            .with_context_window(config.context_window)
            .and_then(|registry| registry.with_overrides(config.model_overrides))
            .map_err(|e| {
                LlmError::AuthenticationError(format!("Invalid Grok configuration: {e}"))
            })?;

        Ok(Self {
            // Phase 25 D-03: name the engine so every `ProviderError` it
            // emits carries "grok", not the engine's generic default.
            engine: CompatEngine::new(engine_config)?
                .with_provider_name(GROK_PROVIDER)
                .with_default_max_tokens(config.max_tokens)
                .with_model_registry(registry)
                // G-50-5a: xAI reports `completion_tokens_details.reasoning_tokens` BESIDE
                // `completion_tokens` -- its total is prompt + completion + reasoning -- so the
                // engine folds reasoning into completion and the Treasurer prices what xAI bills.
                // LIVE-MEASURED 2026-10-18 (plan 50-22, `50-USAGE-PROBE.md`, operator run on the
                // raw xAI wire): grok-4.3, grok-4.5 and grok-4.6 each read `reasoning beside
                // completion` (grok-4.3: 212 + 1 + 457 = 670), and the billed-cost cross-check is
                // consistent to the tick on all three -- `cost_in_usd_ticks` equals the listing
                // prices with reasoning billed at the completion rate. Plan 50-24 applied the
                // verdict (this declaration stays). It replaces plan 50-21's secondary sources
                // (A2-50-21: Google Vertex AI 663 + 50 + 124 = 837). Semantics are per provider:
                // a future Grok model that disagrees is flagged at runtime by the vendor-total
                // reconciliation under `paladin.usage_unreconciled`.
                .with_usage_semantics(CompatUsageSemantics::ReasoningBesideCompletion),
        })
    }
}

#[async_trait]
impl LlmPort for GrokAdapter {
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
        GROK_PROVIDER
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
    use crate::sampling::PARAMS_OMITTED_METADATA_KEY;
    use mockito::Server;
    use paladin_core::platform::container::prompt::{
        PromptItem, PromptParameters, PromptType, UserPrompt,
    };
    use paladin_ports::output::llm_port::FinishReason;
    use paladin_ports::output::model_profile::ParamSupport;
    use serde_json::json;

    /// WR-03: the documented `with_base_url` guarantee is an `http://` or `https://` URL. A value
    /// that merely starts with `http` (`httpx://x`, a scheme-less `http-gateway...`) must fail
    /// construction like any other non-HTTP value.
    #[test]
    fn a_builder_set_non_http_base_url_fails_grok_construction() {
        for bad in [
            "ftp://x",
            "",
            "api.example.com/v1",
            "httpx://x",
            "http-gateway.internal/v1",
            "httpfoo:",
        ] {
            let config =
                GrokConfig::new("k".to_string(), "grok-4.3".to_string()).with_base_url(bad);
            assert!(
                config.validate().is_err(),
                "base URL {bad:?} must fail validation"
            );
            assert!(
                GrokAdapter::new(config).is_err(),
                "base URL {bad:?} must fail adapter construction"
            );
        }
        for good in ["http://localhost:1", "https://gateway.example/v1"] {
            let config =
                GrokConfig::new("k".to_string(), "grok-4.3".to_string()).with_base_url(good);
            assert!(
                config.validate().is_ok(),
                "base URL {good:?} must pass validation"
            );
        }
    }

    // ── GrokConfig::new (SURFACE-02, D-05) ──

    #[test]
    fn grok_config_new_takes_key_and_model_and_defaults_the_endpoint() {
        let config = GrokConfig::new("k".to_string(), "grok-4.3".to_string());
        assert_eq!(config.base_url, GROK_DEFAULT_BASE_URL);
        assert_eq!(config.model, "grok-4.3");
        assert_eq!(config.api_key, "k");
        // The timeout default is unchanged: whatever `default_timeout_seconds` answered for the
        // same model before the endpoint left the constructor.
        assert_eq!(
            config.timeout_seconds,
            default_timeout_seconds(GROK_PROVIDER, "grok-4.3", 60)
        );
        let seeded = GrokConfig::new("k".to_string(), GROK_DEFAULT_MODEL.to_string());
        assert_eq!(
            seeded.timeout_seconds,
            default_timeout_seconds(GROK_PROVIDER, GROK_DEFAULT_MODEL, 60)
        );
    }

    #[test]
    fn grok_config_with_base_url_replaces_the_default_endpoint() {
        let config = GrokConfig::new("k".to_string(), "m".to_string())
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

    #[test]
    fn grok_model_profile_does_not_take_stop_or_penalties() {
        let adapter = GrokAdapter::new(
            GrokConfig::new("test-key".to_string(), GROK_DEFAULT_MODEL.to_string())
                .with_base_url("https://example.invalid/v1"),
        )
        .unwrap();
        let profile = adapter
            .model_profile(GROK_DEFAULT_MODEL)
            .expect("Grok profiles every model");
        assert_eq!(profile.stop_sequences, StopSupport::NotSent);
        assert_eq!(profile.frequency_penalty, ParamSupport::NotSent);
        assert_eq!(profile.presence_penalty, ParamSupport::NotSent);
        // The default model's seeded entry (2026-10-15): 2.01 was rejected "must be less than 2".
        assert_eq!(profile.temperature, ParamSupport::Range(0.0, 1.99));
        assert_eq!(profile.verified_on.as_deref(), Some("2026-10-15"));
    }

    #[test]
    fn grok_default_model_is_seeded() {
        let adapter = GrokAdapter::new(
            GrokConfig::new("test-key".to_string(), GROK_DEFAULT_MODEL.to_string())
                .with_base_url("https://example.invalid/v1"),
        )
        .unwrap();
        let profile = adapter
            .model_profile(GROK_DEFAULT_MODEL)
            .expect("Grok profiles every model");
        // A seeded entry stamps its date on the profile; the adapter-wide fallback carries none.
        assert_eq!(profile.verified_on.as_deref(), Some("2026-10-15"));
        assert_eq!(profile.context_window, Some(1_000_000));
        assert_eq!(
            adapter.get_capabilities().max_context_tokens,
            profile.context_window,
            "the declared window is the default model's seeded window"
        );
    }

    /// The recorded `grok-4.7` exception (owner ruling, 2026-10-20): through the adapter, the
    /// seeded profile does not admit `temperature` 2.00, which failed at the vendor identically
    /// in both probe runs, and it is stamped with the Phase 50.1 date rather than the fallback's.
    #[test]
    fn grok_4_7_profile_does_not_admit_temperature_2_00() {
        let adapter = GrokAdapter::new(
            GrokConfig::new("test-key".to_string(), "grok-4.7".to_string())
                .with_base_url("https://example.invalid/v1"),
        )
        .unwrap();
        let profile = adapter
            .model_profile("grok-4.7")
            .expect("Grok profiles every model");
        assert_eq!(profile.verified_on.as_deref(), Some("2026-10-20"));
        assert!(!profile.temperature.admits(2.0));
        assert!(profile.temperature.admits(0.5));
        // The adapter-wide declaration still admits 2.0 for an unseeded model: only the seeded
        // row narrows it.
        let fallback = adapter
            .model_profile(UNSEEDED_FOR_PROFILE)
            .expect("Grok profiles every model");
        assert!(fallback.temperature.admits(2.0));
    }

    const UNSEEDED_FOR_PROFILE: &str = "grok-unseeded-profile-test-model";

    #[test]
    fn grok_default_model_is_the_operator_identified_current_model() {
        assert_eq!(GROK_DEFAULT_MODEL, "grok-4.3");
        assert_eq!(GROK_FALLBACK_MODELS[0], GROK_DEFAULT_MODEL);
    }

    #[tokio::test]
    async fn grok_stop_is_omitted_and_recorded() {
        let mut server = crate::compat::test_support::server().await;
        let captured = crate::compat::test_support::mock_completion(&mut server).await;
        let adapter = GrokAdapter::new(
            GrokConfig::new("test-key".to_string(), GROK_DEFAULT_MODEL.to_string())
                .with_base_url(server.url()),
        )
        .unwrap();
        let mut request = build_request(GROK_DEFAULT_MODEL);
        request.prompt.entity.data.parameters = PromptParameters {
            max_tokens: Some(64),
            temperature: Some(0.5),
            top_p: None,
            frequency_penalty: None,
            presence_penalty: Some(0.0),
            stop_sequences: Some(vec!["END".to_string()]),
        };

        let response = adapter
            .generate(request)
            .await
            .expect("well-formed response");
        let body = captured.lock().unwrap().take().expect("a request was sent");

        assert!(
            body.get("stop").is_none(),
            "no stop key on the Grok wire: {body}"
        );
        assert!(body.get("presence_penalty").is_none(), "{body}");
        assert_eq!(body.get("temperature").and_then(|v| v.as_f64()), Some(0.5));
        assert_eq!(
            response
                .metadata
                .get(PARAMS_OMITTED_METADATA_KEY)
                .map(String::as_str),
            Some("presence_penalty,stop")
        );
    }

    // ── GrokConfig::from_env() defaulting logic ──

    #[test]
    fn grok_config_from_env_errors_when_api_key_absent() {
        let result = GrokConfig::from_parts(None, None, None, None);
        assert!(result.is_err());
    }

    #[test]
    fn grok_config_defaults_base_url_and_model_when_only_key_is_set() {
        let config = GrokConfig::from_parts(Some("test-key".to_string()), None, None, None)
            .expect("key alone must be sufficient to build a valid config");
        assert_eq!(config.base_url, GROK_DEFAULT_BASE_URL);
        assert_eq!(config.model, GROK_DEFAULT_MODEL);
        // The default model is a seeded reasoning model, so its default timeout is the
        // reasoning-model 300 s (D-13), not the hosted-preset 60 s.
        assert_eq!(
            config.timeout_seconds,
            registry::REASONING_DEFAULT_TIMEOUT_SECONDS
        );
    }

    #[test]
    fn grok_config_honors_xai_base_url_override() {
        let config = GrokConfig::from_parts(
            Some("test-key".to_string()),
            Some("https://override.example/v1".to_string()),
            Some("grok-3".to_string()),
            Some("30".to_string()),
        )
        .unwrap();
        assert_eq!(config.base_url, "https://override.example/v1");
        assert_eq!(config.model, "grok-3");
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
                    "model": GROK_DEFAULT_MODEL,
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

        let config = GrokConfig::new("test-key".to_string(), GROK_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = GrokAdapter::new(config).unwrap();

        let response = adapter
            .generate(build_request(GROK_DEFAULT_MODEL))
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

        let config = GrokConfig::new("test-key".to_string(), GROK_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = GrokAdapter::new(config).unwrap();

        let stream = adapter
            .generate_stream(build_request(GROK_DEFAULT_MODEL))
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

    /// Drain one Grok `generate_stream` over a canned SSE body whose usage frame carries the
    /// xAI shape with the given vendor total.
    async fn grok_stream_with_vendor_total(total: u32) -> Vec<StreamingResponse> {
        use futures::StreamExt;

        let sse_body = format!(
            concat!(
                "data: {{\"id\":\"1\",\"choices\":[{{\"delta\":{{\"content\":\"Par\"}},\"finish_reason\":null}}]}}\n\n",
                "data: {{\"id\":\"1\",\"choices\":[{{\"delta\":{{\"content\":\"is\"}},\"finish_reason\":\"stop\"}}]}}\n\n",
                "data: {{\"id\":\"1\",\"choices\":[],\"usage\":{{\"prompt_tokens\":663,\"completion_tokens\":50,",
                "\"total_tokens\":{total},\"completion_tokens_details\":{{\"reasoning_tokens\":124}}}}}}\n\n",
                "data: [DONE]\n\n",
            ),
            total = total
        );
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(sse_body)
            .create_async()
            .await;
        let adapter = GrokAdapter::new(
            GrokConfig::new("test-key".to_string(), GROK_DEFAULT_MODEL.to_string())
                .with_base_url(server.url()),
        )
        .unwrap();
        let stream = adapter
            .generate_stream(build_request(GROK_DEFAULT_MODEL))
            .await
            .unwrap();
        Box::into_pin(stream)
            .map(|item| item.expect("a stream with text ends Ok"))
            .collect()
            .await
    }

    /// G-50-5a on the streamed path: the terminal chunk's usage is folded with Grok's semantics
    /// (completion 50 + reasoning 124) and reconciled against the vendor total; a disagreement
    /// is recorded on the terminal chunk only, never on a content chunk.
    #[tokio::test]
    async fn grok_stream_terminal_usage_is_folded_and_reconciled() {
        use crate::usage_reconcile::USAGE_UNRECONCILED_METADATA_KEY;

        let chunks = grok_stream_with_vendor_total(837).await;
        let terminal = chunks.last().expect("a terminal chunk");
        assert!(terminal.finish_reason.is_some(), "{terminal:?}");
        let usage = terminal.usage.as_ref().expect("the usage frame is held");
        assert_eq!(usage.prompt_tokens, 663);
        assert_eq!(usage.completion_tokens, 174, "{usage:?}");
        assert_eq!(usage.reasoning_tokens, Some(124));
        assert_eq!(usage.total_tokens, 837);
        assert!(
            !terminal
                .metadata
                .contains_key(USAGE_UNRECONCILED_METADATA_KEY),
            "{:?}",
            terminal.metadata
        );

        let chunks = grok_stream_with_vendor_total(900).await;
        let (terminal, content) = chunks.split_last().expect("a terminal chunk");
        assert_eq!(
            terminal
                .metadata
                .get(USAGE_UNRECONCILED_METADATA_KEY)
                .map(String::as_str),
            Some("total:vendor=900,mapped=837")
        );
        assert_eq!(terminal.usage.as_ref().map(|u| u.total_tokens), Some(837));
        assert!(!content.is_empty());
        for chunk in content {
            assert!(
                !chunk.metadata.contains_key(USAGE_UNRECONCILED_METADATA_KEY),
                "content chunk: {chunk:?}"
            );
        }
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

        let config = GrokConfig::new("test-key".to_string(), GROK_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = GrokAdapter::new(config).unwrap();

        let result = adapter.generate(build_request(GROK_DEFAULT_MODEL)).await;
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

        let config = GrokConfig::new("test-key".to_string(), GROK_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = GrokAdapter::new(config).unwrap();

        let result = adapter.generate(build_request(GROK_DEFAULT_MODEL)).await;
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

        let config = GrokConfig::new("test-key".to_string(), GROK_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = GrokAdapter::new(config).unwrap();

        let result = adapter.generate(build_request(GROK_DEFAULT_MODEL)).await;
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

        let config = GrokConfig::new("test-key".to_string(), GROK_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = GrokAdapter::new(config).unwrap();

        let result = adapter.generate(build_request(GROK_DEFAULT_MODEL)).await;
        assert!(matches!(result, Err(LlmError::InvalidPrompt(_))));
    }

    /// Phase 25 (FT-FR-01, D-03): the preset's non-2xx path is the shared
    /// `map_http_status` — proven by comparing the adapter's error against
    /// the helper's own output for the same status, body and key.
    #[tokio::test]
    async fn grok_non_2xx_routes_through_the_shared_mapper() {
        let body = r#"{"error":"overloaded"}"#;
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(503)
            .with_body(body)
            .expect_at_least(1)
            .create_async()
            .await;

        let config = GrokConfig::new("test-key".to_string(), GROK_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = GrokAdapter::new(config).unwrap();

        let result = adapter.generate(build_request(GROK_DEFAULT_MODEL)).await;
        let expected = map_http_status("grok", 503, body, "test-key");
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
    fn get_provider_name_returns_grok() {
        let config = GrokConfig::new("test-key".to_string(), GROK_DEFAULT_MODEL.to_string());
        let adapter = GrokAdapter::new(config).unwrap();
        assert_eq!(adapter.get_provider_name(), "grok");
    }

    #[test]
    fn get_capabilities_reports_no_tool_or_function_calling() {
        let config = GrokConfig::new("test-key".to_string(), GROK_DEFAULT_MODEL.to_string());
        let adapter = GrokAdapter::new(config).unwrap();
        let caps = adapter.get_capabilities();

        assert!(!caps.supports_tool_calling);
        assert!(!caps.supports_function_calling);
        assert!(!caps.supports_vision);
        assert!(!caps.supports_embeddings);
        assert!(caps.supports_streaming);
        assert!(caps.supports_system_messages);
    }

    // ── request_parameters (17-18, closing G-17-4a) ──

    // Test 1: an engine built from `GrokConfig` against a mock server
    // produces a body omitting exactly the parameters task 1 measured as
    // rejected (`frequency_penalty`, `presence_penalty`), and carrying the
    // rest (`temperature`, `max_tokens`, `top_p`) with the caller's values.
    #[tokio::test]
    async fn generate_omits_exactly_the_measured_unsupported_xai_parameters() {
        use serde_json::Value;
        use std::sync::{Arc, Mutex};

        let mut server = Server::new_async().await;
        let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let captured_clone = Arc::clone(&captured);

        server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body_from_request(move |req| {
                let body_text = req.utf8_lossy_body().unwrap_or_default().into_owned();
                *captured_clone.lock().unwrap() = Some(body_text);
                json!({
                    "id": "cmpl-1",
                    "model": GROK_DEFAULT_MODEL,
                    "choices": [{
                        "index": 0,
                        "message": {"role": "assistant", "content": "Hi there"},
                        "finish_reason": "stop"
                    }],
                    "usage": {"prompt_tokens": 5, "completion_tokens": 3, "total_tokens": 8}
                })
                .to_string()
                .into_bytes()
            })
            .create_async()
            .await;

        let config = GrokConfig::new("test-key".to_string(), GROK_DEFAULT_MODEL.to_string())
            .with_base_url(server.url());
        let adapter = GrokAdapter::new(config).unwrap();

        let mut request = build_request(GROK_DEFAULT_MODEL);
        request.prompt.entity.data.parameters =
            paladin_core::platform::container::prompt::PromptParameters {
                max_tokens: Some(16),
                temperature: Some(0.7),
                top_p: Some(1.0),
                frequency_penalty: Some(0.0),
                presence_penalty: Some(0.0),
                stop_sequences: None,
            };

        let result = adapter.generate(request).await;
        assert!(
            result.is_ok(),
            "mock server returned a well-formed response: {result:?}"
        );

        let body_text = captured
            .lock()
            .unwrap()
            .take()
            .expect("mock must have been called exactly once");
        let body: Value = serde_json::from_str(&body_text).expect("captured body must be JSON");
        let obj = body.as_object().expect("body must be a JSON object");

        assert!(
            !obj.contains_key("frequency_penalty"),
            "xAI rejects frequency_penalty by presence — it must be absent, got: {obj:?}"
        );
        assert!(
            !obj.contains_key("presence_penalty"),
            "xAI rejects presence_penalty by presence — it must be absent, got: {obj:?}"
        );
        assert_eq!(obj.get("temperature").and_then(Value::as_f64), Some(0.7));
        assert_eq!(obj.get("max_tokens").and_then(Value::as_u64), Some(16));
        assert_eq!(obj.get("top_p").and_then(Value::as_f64), Some(1.0));
    }

    /// G-50-5a (owner decision, UAT test 5): a Grok reasoning response is billed in full. xAI
    /// reports `completion_tokens_details.reasoning_tokens` beside `completion_tokens` (vendor
    /// total = prompt + completion + reasoning; this body is plan 50-21's secondary-sourced
    /// Vertex example, live-measured 2026-10-18 in `50-USAGE-PROBE.md`), so the adapter
    /// folds the reasoning into `completion_tokens`, keeps `total_tokens == prompt + completion`
    /// (D-02), and the Treasurer prices what xAI bills. Before the fold this call mapped to
    /// completion 50, total 713 and priced at 1_163_000 nanos.
    #[tokio::test]
    async fn grok_reasoning_tokens_reach_completion_and_the_price() {
        use crate::pricing::PricingLlmAdapter;
        use crate::usage_reconcile::USAGE_UNRECONCILED_METADATA_KEY;
        use paladin_core::platform::container::cost::{CurrencyCode, PriceRow, PriceTable};
        use std::sync::Arc;

        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "id": "cmpl-grok-reasoning",
                    "model": GROK_DEFAULT_MODEL,
                    "choices": [{
                        "index": 0,
                        "message": {"role": "assistant", "content": "Paris"},
                        "finish_reason": "stop"
                    }],
                    "usage": {
                        "prompt_tokens": 663,
                        "completion_tokens": 50,
                        "total_tokens": 837,
                        "completion_tokens_details": {"reasoning_tokens": 124}
                    }
                })
                .to_string(),
            )
            .create_async()
            .await;

        let grok = GrokAdapter::new(
            GrokConfig::new("test-key".to_string(), GROK_DEFAULT_MODEL.to_string())
                .with_base_url(server.url()),
        )
        .unwrap();
        // $1 per million input tokens, $10 per million output tokens.
        let table = Arc::new(PriceTable::new(CurrencyCode::new("USD").unwrap()).with_row(
            GROK_DEFAULT_MODEL,
            PriceRow::new(1_000_000_000, 10_000_000_000).unwrap(),
        ));
        let priced = PricingLlmAdapter::new(Arc::new(grok), table);

        let response = priced
            .generate(build_request(GROK_DEFAULT_MODEL))
            .await
            .expect("well-formed response");

        let usage = &response.usage;
        assert_eq!(usage.prompt_tokens, 663);
        assert_eq!(
            usage.completion_tokens, 174,
            "50 + 124 reasoning: {usage:?}"
        );
        assert_eq!(usage.reasoning_tokens, Some(124));
        assert_eq!(
            usage.total_tokens, 837,
            "equals the vendor's total: {usage:?}"
        );
        let cost = response.cost.as_ref().expect("the model is priced");
        // 663 * $1/M + 174 * $10/M = 663_000 + 1_740_000 nanos, what xAI bills at this row.
        assert_eq!(cost.nanos(), 2_403_000);
        assert!(
            !response
                .metadata
                .contains_key(USAGE_UNRECONCILED_METADATA_KEY),
            "the folded usage agrees with the vendor total: {:?}",
            response.metadata
        );
    }

    /// G-50-5a, plan 50-24: the live-measured grok-4.3 usage object (`50-USAGE-PROBE.md`,
    /// 2026-10-18: prompt 212 of which 128 cached, completion 1, reasoning 457, total 670,
    /// `cost_in_usd_ticks` 12_756_000) maps to the inclusive figures and, priced at the xAI
    /// listing prices recorded in `50-SEED-PROBE.md` (12_500 / 2_000 / 25_000 ticks per token,
    /// 1 tick = 1e-10 USD, i.e. $1.25 / $0.20 / $2.50 per million), costs exactly what xAI
    /// billed: 12_756_000 ticks = 1_275_600 nanos. Read inclusively (no fold) it would price at
    /// 133_100 nanos and disagree with the vendor total.
    #[tokio::test]
    async fn grok_measured_usage_prices_to_the_billed_cost() {
        use crate::pricing::PricingLlmAdapter;
        use crate::usage_reconcile::USAGE_UNRECONCILED_METADATA_KEY;
        use paladin_core::platform::container::cost::{CurrencyCode, PriceRow, PriceTable};
        use std::sync::Arc;

        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "id": "cmpl-grok-measured",
                    "model": "grok-4.3",
                    "choices": [{
                        "index": 0,
                        "message": {"role": "assistant", "content": "391"},
                        "finish_reason": "stop"
                    }],
                    "usage": {
                        "prompt_tokens": 212,
                        "completion_tokens": 1,
                        "total_tokens": 670,
                        "prompt_tokens_details": {"cached_tokens": 128},
                        "completion_tokens_details": {"reasoning_tokens": 457},
                        "num_sources_used": 0,
                        "cost_in_usd_ticks": 12_756_000_u64
                    }
                })
                .to_string(),
            )
            .create_async()
            .await;

        let grok = GrokAdapter::new(
            GrokConfig::new("test-key".to_string(), "grok-4.3".to_string())
                .with_base_url(server.url()),
        )
        .unwrap();
        // xAI listing price in ticks per token x 100_000 = nanos per million tokens.
        let row = PriceRow::new(1_250_000_000, 2_500_000_000)
            .unwrap()
            .with_cache_read(200_000_000)
            .unwrap();
        let table =
            Arc::new(PriceTable::new(CurrencyCode::new("USD").unwrap()).with_row("grok-4.3", row));
        let priced = PricingLlmAdapter::new(Arc::new(grok), table);

        let response = priced
            .generate(build_request("grok-4.3"))
            .await
            .expect("well-formed response");

        let usage = &response.usage;
        assert_eq!(usage.prompt_tokens, 212);
        assert_eq!(usage.cache_read_tokens, Some(128));
        assert_eq!(usage.completion_tokens, 458, "1 + 457 reasoning: {usage:?}");
        assert_eq!(usage.reasoning_tokens, Some(457));
        assert_eq!(usage.total_tokens, 670, "the vendor's total: {usage:?}");
        let cost = response.cost.as_ref().expect("the model is priced");
        // 84 x $1.25/M + 128 x $0.20/M + 458 x $2.50/M = 105_000 + 25_600 + 1_145_000 nanos,
        // which is xAI's cost_in_usd_ticks 12_756_000 / 10.
        assert_eq!(cost.nanos(), 1_275_600);
        assert!(
            !response
                .metadata
                .contains_key(USAGE_UNRECONCILED_METADATA_KEY),
            "the measured usage reconciles: {:?}",
            response.metadata
        );
    }

    // Test 2: the curated fallback can never disagree with the default
    // about which model to reach for.
    #[test]
    fn fallback_models_is_non_empty_and_starts_with_the_default_model() {
        assert!(!GROK_FALLBACK_MODELS.is_empty());
        assert_eq!(GROK_FALLBACK_MODELS[0], GROK_DEFAULT_MODEL);
    }

    // ── Shared conformance suite (D-19, plan 31-04) ──
    //
    // Grok delegates every method to `CompatEngine` (D-05), so it speaks the identical
    // OpenAI-compatible wire shape `openai_compatible`/`ollama` already instantiate the suite
    // against; bodies below mirror those fixtures' shape exactly (distinct model/provider names
    // only).
    mod conformance_suite {
        use super::*;
        use std::sync::Arc;

        struct GrokFixture;

        impl crate::conformance::ConformanceFixture for GrokFixture {
            const WIRE: crate::conformance::Wire = crate::conformance::Wire::OpenAiChat;

            fn default_model() -> &'static str {
                GROK_DEFAULT_MODEL
            }

            fn provider_name() -> &'static str {
                "grok"
            }

            fn adapter(base_url: &str) -> Arc<dyn LlmPort> {
                let config =
                    GrokConfig::new("test-key".to_string(), GROK_DEFAULT_MODEL.to_string())
                        .with_base_url(base_url);
                Arc::new(GrokAdapter::new(config).expect("test config must build"))
            }

            fn success_body() -> String {
                json!({
                    "id": "cmpl-1",
                    "model": GROK_DEFAULT_MODEL,
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

            // G-50-5a: xAI reports `reasoning_tokens` BESIDE `completion_tokens` (vendor total =
            // prompt + completion + reasoning; live-measured 2026-10-18, `50-USAGE-PROBE.md`),
            // so this body is xAI-shaped -- completion 25 plus reasoning 15,
            // total 140 -- and must map to the same inclusive figures every other preset's
            // OpenAI-shaped body does (completion 40). It pins Grok's
            // `ReasoningBesideCompletion` declaration by behaviour.
            fn usage_body() -> String {
                json!({
                    "id": "cmpl-1",
                    "model": GROK_DEFAULT_MODEL,
                    "choices": [{
                        "index": 0,
                        "message": {"role": "assistant", "content": "Hi there"},
                        "finish_reason": "stop"
                    }],
                    "usage": {
                        "prompt_tokens": 100,
                        "completion_tokens": 25,
                        "total_tokens": 140,
                        "prompt_tokens_details": {"cached_tokens": 30},
                        "completion_tokens_details": {"reasoning_tokens": 15}
                    }
                })
                .to_string()
            }

            fn expected_usage() -> crate::conformance::ExpectedUsage {
                // prompt 100, completion 25 + 15 = 40, reasoning 15, cache read 30, total 140.
                crate::conformance::wire_defaults::expected_usage(
                    crate::conformance::Wire::OpenAiChat,
                )
            }

            fn error_body(status: u16) -> String {
                json!({"error": format!("mock error for status {status}")}).to_string()
            }
        }

        crate::llm_conformance_suite!(GrokFixture);
    }

    // ── PARITY-03 knobs (plan 50-08, D-12) ──

    const UNSEEDED: &str = "grok-unseeded-test-model";

    fn knob_config(url: &str, model: &str) -> GrokConfig {
        GrokConfig::new("test-key".to_string(), model.to_string()).with_base_url(url)
    }

    #[tokio::test]
    async fn grok_max_tokens_default_reaches_the_wire() {
        use crate::compat::test_support::{mock_completion, send_with_max_tokens, server};

        let mut server = server().await;
        let captured = mock_completion(&mut server).await;
        let adapter =
            GrokAdapter::new(knob_config(&server.url(), UNSEEDED).with_max_tokens(2048)).unwrap();

        let body = send_with_max_tokens(&adapter, &captured, UNSEEDED, None)
            .await
            .unwrap();
        assert_eq!(body["max_tokens"], 2048, "{body}");
        let body = send_with_max_tokens(&adapter, &captured, UNSEEDED, Some(512))
            .await
            .unwrap();
        assert_eq!(body["max_tokens"], 512, "{body}");

        let plain = GrokAdapter::new(knob_config(&server.url(), UNSEEDED)).unwrap();
        let body = send_with_max_tokens(&plain, &captured, UNSEEDED, None)
            .await
            .unwrap();
        assert!(body.get("max_tokens").is_none(), "{body}");
    }

    #[tokio::test]
    async fn grok_max_retries_is_read_by_the_engine() {
        for retries in [0_u32, 1] {
            let server = crate::compat::test_support::server().await;
            let adapter =
                GrokAdapter::new(knob_config(&server.url(), UNSEEDED).with_max_retries(retries))
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
    fn grok_context_window_override_is_declared() {
        let adapter = GrokAdapter::new(
            knob_config("https://example.invalid/v1", UNSEEDED).with_context_window(500_000),
        )
        .unwrap();
        assert_eq!(adapter.get_capabilities().max_context_tokens, Some(500_000));
        let profile = adapter.model_profile(UNSEEDED).unwrap();
        assert_eq!(profile.context_window, Some(500_000));
    }

    #[test]
    fn grok_model_override_reaches_the_profile_and_invalid_one_fails_construction() {
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
        let adapter = GrokAdapter::new(with_override(900_000)).unwrap();
        assert_eq!(
            adapter.model_profile(UNSEEDED).unwrap().context_window,
            Some(900_000)
        );
        let message = match GrokAdapter::new(with_override(0)) {
            Err(error) => error.to_string(),
            Ok(_) => panic!("a zero window must fail construction"),
        };
        assert!(
            message.contains("grok") && message.contains(UNSEEDED),
            "{message}"
        );
    }

    #[test]
    fn grok_config_defaults_a_reasoning_model_to_300s() {
        // (model, expected default timeout): a seeded reasoning model gets 300 s (D-13); any
        // other model keeps this preset's compiled default (60 s).
        for (model, expected) in [
            (GROK_DEFAULT_MODEL, 300),
            ("grok-4.6", 300),
            // The Phase 50.1 rows: the reasoning ones (`reasoning: true`) get 300 s, the
            // non-reasoning alias keeps the hosted-preset 60 s.
            ("grok-4.7", 300),
            ("grok-4.20-reasoning", 300),
            ("grok-4.20-non-reasoning", 60),
            (UNSEEDED, 60),
        ] {
            let url = "https://example.invalid/v1";
            assert_eq!(
                knob_config(url, model).timeout_seconds,
                expected,
                "new({model})"
            );
            let parts =
                GrokConfig::from_parts(Some("k".to_string()), None, Some(model.to_string()), None)
                    .unwrap();
            assert_eq!(parts.timeout_seconds, expected, "from_parts({model})");
        }
        // An explicit environment value always wins.
        for explicit in ["30", "60", "900"] {
            let config = GrokConfig::from_parts(
                Some("k".to_string()),
                None,
                Some("grok-4.3".to_string()),
                Some(explicit.to_string()),
            )
            .unwrap();
            assert_eq!(config.timeout_seconds.to_string(), explicit);
        }
    }

    #[test]
    fn grok_config_debug_redacts_the_api_key() {
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
}
