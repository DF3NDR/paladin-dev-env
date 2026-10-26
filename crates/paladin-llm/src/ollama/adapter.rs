//! Ollama (self-hosted, keyless) LLM Adapter.
//!
//! A thin preset (D-01, D-02, D-05, D-12) sitting entirely on
//! [`crate::compat::CompatEngine`] — the shared OpenAI-compatible protocol
//! engine. Ollama is the host that settles the original request's
//! "Meta (Llama)?" row (D-02): Llama names a model family, not a provider,
//! and Ollama is the only candidate testable without anyone's API key —
//! the phase's only end-to-end exercise of the shared compatible core
//! against a real implementation of the protocol.
//!
//! ## Endpoint choice
//!
//! This adapter speaks Ollama's `/v1/*` OpenAI-compatibility layer, **not**
//! the native `/api/chat` endpoint. The compat layer uses standard
//! `data: `-framed SSE, which [`CompatEngine`] already speaks; the native
//! API streams newline-delimited JSON instead, which would fork the engine
//! just for this one preset (D-05).
//!
//! ## Credential
//!
//! Ollama's own docs describe the API key as "required but ignored" —
//! rather than making the engine's `Authorization` header itself
//! `Option` (forking the header-construction path for exactly one
//! provider), this adapter sends the fixed placeholder
//! [`OLLAMA_PLACEHOLDER_API_KEY`]. No operator ever sets a credential for
//! Ollama (D-12).

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
use crate::registry::{
    DEFAULT_MAX_RETRIES, ModelOverride, ModelRegistry, config_knob_builders,
    default_timeout_seconds, fallback_from_compat, is_http_url, seeded,
};

/// Default Ollama base URL — the local OpenAI-compatibility layer
/// (`/v1/*`), not the native `/api/chat` endpoint. See the module-level
/// rustdoc "Endpoint choice" section for why.
pub const OLLAMA_DEFAULT_BASE_URL: &str = "http://localhost:11434/v1";

/// Default Ollama model requested when `OLLAMA_MODEL` is unset. Purely a
/// reasonable placeholder — the real catalog is whatever the operator
/// pulled locally.
pub const OLLAMA_DEFAULT_MODEL: &str = "llama3";

/// Curated fallback model list (D-13), returned when the live `/models`
/// endpoint fails, is unreachable, or returns an empty list. Explicitly a
/// degrade-gracefully placeholder, not an authoritative catalog — the real
/// catalog is whatever the operator has pulled onto their local Ollama
/// instance, which is exactly why the live fetch is the primary path here.
pub const OLLAMA_FALLBACK_MODELS: &[&str] = &["llama3", "qwen", "mistral"];

/// The fixed placeholder credential sent as `Authorization: Bearer
/// {OLLAMA_PLACEHOLDER_API_KEY}`. Ollama's own docs describe the API key
/// as required-but-ignored; this is a compile-time constant, never
/// operator-supplied, so it never becomes a credential-shaped value that
/// gets redacted incorrectly. No operator ever sets it.
pub const OLLAMA_PLACEHOLDER_API_KEY: &str = "ollama";

/// Configuration for the Ollama (self-hosted, keyless) adapter.
///
/// Carries no `api_key` field — Ollama requires no credential (D-12); see
/// [`OLLAMA_PLACEHOLDER_API_KEY`] for how the engine's header-construction
/// path is kept identical across every preset regardless.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct OllamaConfig {
    /// Base URL for the Ollama compat-layer API.
    pub base_url: String,
    /// Default model to use.
    pub model: String,
    /// Request timeout in seconds. Defaults to `120` rather than the
    /// hosted-preset `60` — a local model on cold start is slower than a
    /// hosted endpoint (T-17-17).
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
    /// Where this endpoint parses a reasoning effort in the request body (Phase 51, D-10).
    ///
    /// Declared by the operator, who proves it with `live_vendor_smoke --effort` against the
    /// endpoint: the "invalid value rejected" row of that probe is the proof the field is read.
    /// Defaults to [`ReasoningWire::Unsupported`], which sends nothing and refuses every explicit
    /// effort before any HTTP call. An explicit effort needs both a wire other than
    /// `Unsupported` and the model's `effort` override
    /// ([`model_overrides`](Self::model_overrides)); no network call happens at construction.
    pub reasoning_wire: ReasoningWire,
}

impl OllamaConfig {
    /// Load configuration from environment variables.
    ///
    /// # Environment Variables
    /// - `OLLAMA_BASE_URL` (optional): API base URL, defaults to
    ///   [`OLLAMA_DEFAULT_BASE_URL`].
    /// - `OLLAMA_MODEL` (optional): Default model, defaults to
    ///   [`OLLAMA_DEFAULT_MODEL`].
    /// - `OLLAMA_TIMEOUT_SECONDS` (optional): Request timeout, defaults to
    ///   `120`.
    ///
    /// No credential environment variable exists — this must succeed with
    /// an entirely empty environment (D-12).
    ///
    /// # Errors
    /// Returns an error only if `OLLAMA_TIMEOUT_SECONDS` is set but fails
    /// to parse, or another value fails validation.
    pub fn from_env() -> Result<Self, String> {
        Self::from_parts(
            env::var("OLLAMA_BASE_URL").ok(),
            env::var("OLLAMA_MODEL").ok(),
            env::var("OLLAMA_TIMEOUT_SECONDS").ok(),
        )
    }

    /// The pure defaulting/validation logic behind [`Self::from_env`],
    /// separated out so it is testable without mutating process environment
    /// variables — `std::env::set_var` is `unsafe` under Rust 2024 and this
    /// crate denies `unsafe_code` (`#![deny(unsafe_code)]`).
    fn from_parts(
        base_url: Option<String>,
        model: Option<String>,
        timeout_seconds: Option<String>,
    ) -> Result<Self, String> {
        let base_url = base_url.unwrap_or_else(|| OLLAMA_DEFAULT_BASE_URL.to_string());
        let model = model.unwrap_or_else(|| OLLAMA_DEFAULT_MODEL.to_string());
        let timeout_seconds = match timeout_seconds {
            Some(value) => value
                .parse()
                .map_err(|_| "Invalid OLLAMA_TIMEOUT_SECONDS value".to_string())?,
            None => default_timeout_seconds(OLLAMA_PROVIDER, &model, 120),
        };

        let config = Self {
            base_url,
            model,
            timeout_seconds,
            max_tokens: None,
            max_retries: DEFAULT_MAX_RETRIES,
            context_window: None,
            model_overrides: BTreeMap::new(),
            reasoning_effort: None,
            reasoning_fallback: None,
            reasoning_wire: ReasoningWire::Unsupported,
        };

        config.validate()?;
        Ok(config)
    }

    /// Create configuration with custom values.
    pub fn new(base_url: String, model: String) -> Self {
        let timeout_seconds = default_timeout_seconds(OLLAMA_PROVIDER, &model, 120);
        Self {
            base_url,
            model,
            timeout_seconds,
            max_tokens: None,
            max_retries: DEFAULT_MAX_RETRIES,
            context_window: None,
            model_overrides: BTreeMap::new(),
            reasoning_effort: None,
            reasoning_fallback: None,
            reasoning_wire: ReasoningWire::Unsupported,
        }
    }

    /// Declare where this endpoint parses a reasoning effort (Phase 51, D-10).
    ///
    /// The default is [`ReasoningWire::Unsupported`]: no effort is sent and every explicit effort
    /// is refused before any HTTP call. Declaring a wire is the operator's statement that the
    /// endpoint reads that field; the model's levels still come from its `effort` override.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_llm::reasoning::ReasoningWire;
    /// use paladin_llm::ollama::OllamaConfig;
    ///
    /// let config = OllamaConfig::new("http://localhost:11434/v1".to_string(), "llama3".to_string())
    ///     .with_reasoning_wire(ReasoningWire::TopLevelEffort);
    /// assert_eq!(config.reasoning_wire, ReasoningWire::TopLevelEffort);
    /// ```
    #[must_use]
    pub fn with_reasoning_wire(mut self, wire: ReasoningWire) -> Self {
        self.reasoning_wire = wire;
        self
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), String> {
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

config_knob_builders!(OllamaConfig);

/// The provider name this adapter reports through [`LlmPort::get_provider_name`]
/// and that its engine stamps on every `LlmError::ProviderError` (Phase 25 D-03).
const OLLAMA_PROVIDER: &str = "ollama";

/// Ollama (self-hosted, keyless) LLM Adapter implementing [`LlmPort`].
///
/// Every method delegates to an owned [`CompatEngine`] (D-05) — this struct
/// carries no protocol logic of its own.
pub struct OllamaAdapter {
    engine: CompatEngine,
}

impl OllamaAdapter {
    /// Create a new Ollama adapter.
    ///
    /// # Errors
    /// Returns an error if configuration is invalid or the underlying HTTP
    /// client cannot be created.
    pub fn new(config: OllamaConfig) -> Result<Self, LlmError> {
        config.validate().map_err(|e| {
            LlmError::AuthenticationError(format!("Invalid Ollama configuration: {}", e))
        })?;

        let engine_config = CompatEngineConfig {
            base_url: config.base_url,
            api_key: OLLAMA_PLACEHOLDER_API_KEY.to_string(),
            model: config.model,
            timeout_seconds: config.timeout_seconds,
            max_retries: config.max_retries,
            capabilities: CompatCapabilities {
                supports_streaming: true,
                supports_tool_calling: false,
                supports_function_calling: false,
                supports_vision: false,
                supports_embeddings: false,
                // The context window depends on whichever model the
                // operator pulled — declaring a number would be a claim
                // nobody asserted.
                max_context_tokens: config.context_window,
                supports_system_messages: true,
                temperature_range: Some((0.0, 2.0)),
            },
            // Unchanged pre-existing behaviour (17-18): no vendor-specific
            // sampling-parameter restriction has been measured for Ollama.
            // Phase 51 (D-10): no reasoning wire is verified for a self-hosted endpoint, so
            // the default is `Unsupported` and an explicit effort is refused until the operator
            // declares the field the endpoint parses (`OllamaConfig::reasoning_wire`).
            request_parameters: CompatRequestParameters::all(config.reasoning_wire),
            fallback_models: OLLAMA_FALLBACK_MODELS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            // No usage-cap (quota) mapping for ollama: its quota-exhaustion body was never captured
            // (50-SEED-PROBE.md, owner reading R-2: accepted unverified 2026-10-16, plan 50-17),
            // so an override would be invented evidence (D-11a). The shared status mapping applies.
            error_override: None,
            // WR-04 (`17-REVIEW.md`, T-17-52/T-17-53), superseding the
            // 17-04 comment this replaces: `OLLAMA_DEFAULT_BASE_URL` is
            // only this preset's *default* — `OLLAMA_BASE_URL` is
            // documented and operator-settable (`OllamaConfig::from_env`)
            // and may legitimately point at a remote host, not only the
            // local default. Unlike every other preset, the credential
            // this adapter sends is the fixed, non-secret
            // `OLLAMA_PLACEHOLDER_API_KEY` (never operator-supplied), so a
            // followed redirect here is a request-forwarding exposure, not
            // a credential-disclosure one — but `Policy::none()` is set
            // for uniformity with the rest of the crate and because a
            // redirecting `OLLAMA_BASE_URL` is still not a request this
            // client should silently forward. A refused redirect surfaces
            // via the engine's `300..=399` `map_error` arm.
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

        // The Ollama table is empty by design: the 2026-10-15 probe skipped Ollama (no daemon was
        // configured), so every model runs on the adapter-wide fallback.
        // PARITY-03: the operator's adapter-wide window, then the per-model overrides, are laid
        // over the seeded table; either one failing construction names the provider and model.
        let registry = ModelRegistry::new(OLLAMA_PROVIDER, seeded::OLLAMA, fallback)
            .with_context_window(config.context_window)
            .and_then(|registry| registry.with_overrides(config.model_overrides))
            .map_err(|e| {
                LlmError::AuthenticationError(format!("Invalid Ollama configuration: {e}"))
            })?;

        Ok(Self {
            // Phase 25 D-03: name the engine so every `ProviderError` it
            // emits carries "ollama", not the engine's generic default.
            engine: CompatEngine::new(engine_config)?
                .with_provider_name(OLLAMA_PROVIDER)
                .with_default_max_tokens(config.max_tokens)
                .with_default_reasoning(config.reasoning_effort, config.reasoning_fallback)
                .with_model_registry(registry),
        })
    }
}

#[async_trait]
impl LlmPort for OllamaAdapter {
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
        OLLAMA_PROVIDER
    }

    fn get_capabilities(&self) -> ProviderCapabilities {
        self.engine.capabilities()
    }

    fn model_profile(&self, model: &str) -> Option<ModelProfile> {
        Some(self.engine.model_profile(model))
    }

    /// The pre-judging twin of the engine's send-time `in_force` + `admit`: this preset's
    /// declared-wire-masked profile and provider defaults, the same verdict (D-09).
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
    fn a_builder_set_non_http_base_url_fails_ollama_construction() {
        for bad in [
            "ftp://x",
            "",
            "api.example.com/v1",
            "httpx://x",
            "http-gateway.internal/v1",
            "httpfoo:",
        ] {
            let config =
                OllamaConfig::new("http://localhost:11434".to_string(), "llama3".to_string())
                    .with_base_url(bad);
            assert!(
                config.validate().is_err(),
                "base URL {bad:?} must fail validation"
            );
            assert!(
                OllamaAdapter::new(config).is_err(),
                "base URL {bad:?} must fail adapter construction"
            );
        }
        for good in ["http://localhost:1", "https://gateway.example/v1"] {
            let config =
                OllamaConfig::new("http://localhost:11434".to_string(), "llama3".to_string())
                    .with_base_url(good);
            assert!(
                config.validate().is_ok(),
                "base URL {good:?} must pass validation"
            );
        }
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
    async fn ollama_stop_reaches_the_wire_in_order() {
        let mut server = crate::compat::test_support::server().await;
        let captured = crate::compat::test_support::mock_completion(&mut server).await;
        let adapter = OllamaAdapter::new(OllamaConfig::new(
            server.url(),
            OLLAMA_DEFAULT_MODEL.to_string(),
        ))
        .unwrap();
        let mut request = build_request(OLLAMA_DEFAULT_MODEL);
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
            .model_profile(OLLAMA_DEFAULT_MODEL)
            .expect("ollama profiles every model");
        assert_eq!(
            profile.temperature,
            paladin_ports::output::model_profile::ParamSupport::Range(0.0, 2.0)
        );
    }

    /// The 2026-10-15 probe skipped Ollama (no daemon configured), so its table is empty and its
    /// default model and window keep their previous values: every model runs on the fallback.
    #[test]
    fn ollama_models_run_on_the_adapter_wide_fallback_because_the_probe_skipped_it() {
        let adapter = OllamaAdapter::new(OllamaConfig::new(
            "http://localhost:11434/v1".to_string(),
            OLLAMA_DEFAULT_MODEL.to_string(),
        ))
        .unwrap();
        let profile = adapter
            .model_profile(OLLAMA_DEFAULT_MODEL)
            .expect("ollama profiles every model");
        assert_eq!(profile.verified_on, None);
        assert_eq!(OLLAMA_DEFAULT_MODEL, "llama3");
        assert_eq!(adapter.get_capabilities().max_context_tokens, None);
    }

    // ── OllamaConfig::from_env() defaulting logic ──

    #[test]
    fn ollama_config_from_env_succeeds_with_no_environment_variable_read() {
        let result = OllamaConfig::from_parts(None, None, None);
        assert!(
            result.is_ok(),
            "Ollama requires no credential — from_parts(None, None, None) must succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn ollama_config_defaults_base_url_model_and_timeout_when_nothing_is_set() {
        let config =
            OllamaConfig::from_parts(None, None, None).expect("must succeed with no env vars");
        assert_eq!(config.base_url, OLLAMA_DEFAULT_BASE_URL);
        assert_eq!(config.model, OLLAMA_DEFAULT_MODEL);
        assert_eq!(config.timeout_seconds, 120);
    }

    #[test]
    fn ollama_config_honors_ollama_base_url_override() {
        let config = OllamaConfig::from_parts(
            Some("http://override.local:11434/v1".to_string()),
            Some("mistral".to_string()),
            Some("30".to_string()),
        )
        .unwrap();
        assert_eq!(config.base_url, "http://override.local:11434/v1");
        assert_eq!(config.model, "mistral");
        assert_eq!(config.timeout_seconds, 30);
    }

    // ── Request shaping / response parsing ──

    #[tokio::test]
    async fn generate_posts_with_placeholder_authorization_header_present() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/chat/completions")
            .match_header("authorization", "Bearer ollama")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "id": "cmpl-1",
                    "model": "llama3",
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

        let config = OllamaConfig::new(server.url(), "llama3".to_string());
        let adapter = OllamaAdapter::new(config).unwrap();

        let response = adapter
            .generate(build_request("llama3"))
            .await
            .expect("mock server returned a well-formed response");

        assert_eq!(response.content, "Hi there");
        assert!(matches!(response.finish_reason, FinishReason::Stop));

        // The header is present, not omitted — Ollama's docs describe the
        // key as required-but-ignored, so the engine must still send it.
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

        let config = OllamaConfig::new(server.url(), "llama3".to_string());
        let adapter = OllamaAdapter::new(config).unwrap();

        let stream = adapter
            .generate_stream(build_request("llama3"))
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

    // ── Phase 25 (FT-FR-01, D-03): non-2xx routes through map_http_status ──

    /// The preset's non-2xx path is the shared `map_http_status` — proven
    /// by comparing the adapter's error against the helper's own output for
    /// the same status, body and (placeholder) key.
    #[tokio::test]
    async fn ollama_non_2xx_routes_through_the_shared_mapper() {
        let body = r#"{"error":"model is loading"}"#;
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(503)
            .with_body(body)
            .expect_at_least(1)
            .create_async()
            .await;

        let config = OllamaConfig::new(server.url(), "llama3".to_string());
        let adapter = OllamaAdapter::new(config).unwrap();

        let result = adapter.generate(build_request("llama3")).await;
        let expected = map_http_status("ollama", 503, body, OLLAMA_PLACEHOLDER_API_KEY);
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

    /// A local Ollama server's 4xx semantics are not OpenAI's: a status with
    /// no dedicated variant (here 409) must arrive as a typed
    /// `ProviderError`, never erased into `ProcessingError`. Only one
    /// request is expected: 409 classifies Permanent, but the engine's
    /// retry loop is by-exclusion, so `expect_at_least(1)` is the honest
    /// bound.
    #[tokio::test]
    async fn ollama_local_server_4xx_maps_without_a_dedicated_variant() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(409)
            .with_body(r#"{"error":"conflict"}"#)
            .expect_at_least(1)
            .create_async()
            .await;

        let config = OllamaConfig::new(server.url(), "llama3".to_string());
        let adapter = OllamaAdapter::new(config).unwrap();

        match adapter.generate(build_request("llama3")).await {
            Err(LlmError::ProviderError {
                provider, status, ..
            }) => {
                assert_eq!(provider, "ollama");
                assert_eq!(status, 409);
            }
            Err(LlmError::ProcessingError(msg)) => {
                panic!("409 must not be erased into ProcessingError: {msg}")
            }
            other => panic!("expected ProviderError {{ status: 409 }}, got {other:?}"),
        }
    }

    // ── Model list: live catalog vs. curated fallback (D-13) ──

    #[tokio::test]
    async fn get_available_models_returns_the_two_live_entries() {
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/models")
            .with_status(200)
            .with_body(json!({"data": [{"id": "llama3"}, {"id": "codellama"}]}).to_string())
            .create_async()
            .await;

        let config = OllamaConfig::new(server.url(), "llama3".to_string());
        let adapter = OllamaAdapter::new(config).unwrap();

        let models = adapter.get_available_models().await.unwrap();
        assert_eq!(models, vec!["llama3".to_string(), "codellama".to_string()]);
    }

    #[tokio::test]
    async fn get_available_models_falls_back_to_curated_list_when_live_data_is_empty() {
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/models")
            .with_status(200)
            .with_body(json!({"data": []}).to_string())
            .create_async()
            .await;

        let config = OllamaConfig::new(server.url(), "llama3".to_string());
        let adapter = OllamaAdapter::new(config).unwrap();

        let models = adapter.get_available_models().await.unwrap();
        assert_eq!(
            models,
            OLLAMA_FALLBACK_MODELS
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        );
    }

    // ── Capabilities / identity ──

    #[test]
    fn get_provider_name_returns_ollama() {
        let config = OllamaConfig::new(
            OLLAMA_DEFAULT_BASE_URL.to_string(),
            OLLAMA_DEFAULT_MODEL.to_string(),
        );
        let adapter = OllamaAdapter::new(config).unwrap();
        assert_eq!(adapter.get_provider_name(), "ollama");
    }

    #[test]
    fn get_capabilities_reports_no_max_context_tokens_and_no_tool_or_function_calling() {
        let config = OllamaConfig::new(
            OLLAMA_DEFAULT_BASE_URL.to_string(),
            OLLAMA_DEFAULT_MODEL.to_string(),
        );
        let adapter = OllamaAdapter::new(config).unwrap();
        let caps = adapter.get_capabilities();

        assert!(!caps.supports_tool_calling);
        assert!(!caps.supports_function_calling);
        assert!(!caps.supports_vision);
        assert!(!caps.supports_embeddings);
        assert!(caps.supports_streaming);
        assert!(caps.supports_system_messages);
        assert_eq!(caps.max_context_tokens, None);
    }

    // ── Shared conformance suite (RT-06, D-31) ──
    //
    // Nested in its own module (rather than inline in `mod tests`) so every generated test's
    // full path contains "conformance" -- `cargo test --lib conformance` (the plan's own
    // acceptance criterion) selects it by that substring. Bodies below are copied verbatim from
    // this module's own hand-written tests above
    // (`generate_posts_with_placeholder_authorization_header_present`,
    // `generate_stream_assembles_deltas_in_wire_order_with_terminal_stop`) rather than invented
    // for this suite (26-PATTERNS.md).
    mod conformance_suite {
        use super::*;
        use std::sync::Arc;

        struct OllamaFixture;

        impl crate::conformance::ConformanceFixture for OllamaFixture {
            /// The self-hosted adapter's declared wire (`Unsupported` by default carries no
            /// level; `top_level_effort` and `thinking_object` are each read).
            fn effort_wire_value(body: &serde_json::Value) -> Option<String> {
                crate::conformance::chat_effort_wire_value(body)
                    .or_else(|| crate::conformance::thinking_effort_wire_value(body))
            }

            const WIRE: crate::conformance::Wire = crate::conformance::Wire::OpenAiChat;

            fn default_model() -> &'static str {
                OLLAMA_DEFAULT_MODEL
            }

            fn provider_name() -> &'static str {
                "ollama"
            }

            fn api_key() -> &'static str {
                OLLAMA_PLACEHOLDER_API_KEY
            }

            fn adapter(base_url: &str) -> Arc<dyn LlmPort> {
                let config = OllamaConfig::new(base_url.to_string(), "llama3".to_string());
                Arc::new(OllamaAdapter::new(config).expect("test config must build"))
            }

            fn success_body() -> String {
                json!({
                    "id": "cmpl-1",
                    "model": "llama3",
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

        crate::llm_conformance_suite!(OllamaFixture);
    }

    // ── PARITY-03 knobs (plan 50-08, D-12) ──

    const UNSEEDED: &str = "ollama-unseeded-test-model";

    fn knob_config(url: &str, model: &str) -> OllamaConfig {
        OllamaConfig::new(url.to_string(), model.to_string())
    }

    #[tokio::test]
    async fn ollama_max_tokens_default_reaches_the_wire() {
        use crate::compat::test_support::{mock_completion, send_with_max_tokens, server};

        let mut server = server().await;
        let captured = mock_completion(&mut server).await;
        let adapter =
            OllamaAdapter::new(knob_config(&server.url(), UNSEEDED).with_max_tokens(2048)).unwrap();

        let body = send_with_max_tokens(&adapter, &captured, UNSEEDED, None)
            .await
            .unwrap();
        assert_eq!(body["max_tokens"], 2048, "{body}");
        let body = send_with_max_tokens(&adapter, &captured, UNSEEDED, Some(512))
            .await
            .unwrap();
        assert_eq!(body["max_tokens"], 512, "{body}");

        let plain = OllamaAdapter::new(knob_config(&server.url(), UNSEEDED)).unwrap();
        let body = send_with_max_tokens(&plain, &captured, UNSEEDED, None)
            .await
            .unwrap();
        assert!(body.get("max_tokens").is_none(), "{body}");
    }

    #[tokio::test]
    async fn ollama_max_retries_is_read_by_the_engine() {
        for retries in [0_u32, 1] {
            let server = crate::compat::test_support::server().await;
            let adapter =
                OllamaAdapter::new(knob_config(&server.url(), UNSEEDED).with_max_retries(retries))
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
    fn ollama_context_window_override_is_declared() {
        let adapter = OllamaAdapter::new(
            knob_config("https://example.invalid/v1", UNSEEDED).with_context_window(500_000),
        )
        .unwrap();
        assert_eq!(adapter.get_capabilities().max_context_tokens, Some(500_000));
        let profile = adapter.model_profile(UNSEEDED).unwrap();
        assert_eq!(profile.context_window, Some(500_000));
    }

    #[test]
    fn ollama_model_override_reaches_the_profile_and_invalid_one_fails_construction() {
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
        let adapter = OllamaAdapter::new(with_override(900_000)).unwrap();
        assert_eq!(
            adapter.model_profile(UNSEEDED).unwrap().context_window,
            Some(900_000)
        );
        let message = match OllamaAdapter::new(with_override(0)) {
            Err(error) => error.to_string(),
            Ok(_) => panic!("a zero window must fail construction"),
        };
        assert!(
            message.contains("ollama") && message.contains(UNSEEDED),
            "{message}"
        );
    }

    #[test]
    fn ollama_config_defaults_a_reasoning_model_to_300s() {
        // (model, expected default timeout): a seeded reasoning model gets 300 s (D-13); any
        // other model keeps this preset's compiled default (120 s).
        for (model, expected) in [(OLLAMA_DEFAULT_MODEL, 120), (UNSEEDED, 120)] {
            let url = "https://example.invalid/v1";
            assert_eq!(
                knob_config(url, model).timeout_seconds,
                expected,
                "new({model})"
            );
            let parts = OllamaConfig::from_parts(None, Some(model.to_string()), None).unwrap();
            assert_eq!(parts.timeout_seconds, expected, "from_parts({model})");
        }
        // An explicit environment value always wins.
        for explicit in ["30", "60", "900"] {
            let config = OllamaConfig::from_parts(
                None,
                Some("llama3".to_string()),
                Some(explicit.to_string()),
            )
            .unwrap();
            assert_eq!(config.timeout_seconds.to_string(), explicit);
        }
    }
    // ── Phase 51 (REASON-02, D-10): the declared reasoning wire ──

    #[test]
    fn ollama_declares_no_reasoning_wire_by_default() {
        let adapter = OllamaAdapter::new(OllamaConfig::new(
            "http://localhost:11434/v1".to_string(),
            "llama3".to_string(),
        ))
        .unwrap();
        assert_eq!(
            adapter.engine.request_parameters().reasoning,
            ReasoningWire::Unsupported
        );
    }
    #[test]
    fn ollama_config_reasoning_wire_defaults_to_unsupported_and_is_set_by_the_builder() {
        let config = OllamaConfig::new("http://localhost:11434/v1".to_string(), "m".to_string());
        assert_eq!(config.reasoning_wire, ReasoningWire::Unsupported);
        assert!(format!("{config:?}").contains("reasoning_wire"));
        let config = config.with_reasoning_wire(ReasoningWire::TopLevelEffort);
        assert_eq!(config.reasoning_wire, ReasoningWire::TopLevelEffort);
        assert!(format!("{config:?}").contains("TopLevelEffort"));
        // `from_env` leaves the same default.
        let from_env = OllamaConfig::from_parts(None, None, None).unwrap();
        assert_eq!(from_env.reasoning_wire, ReasoningWire::Unsupported);
    }

    #[tokio::test]
    async fn an_operator_declared_wire_and_override_send_thinking_effort() {
        use crate::compat::test_support::{mock_completion, plain_request, send_capturing, server};
        use paladin_ports::output::llm_port::ReasoningEffort::{High, Low};
        use paladin_ports::output::model_profile::{EffortLevels, EffortSupport};

        let mut server = server().await;
        let captured = mock_completion(&mut server).await;
        let over = ModelOverride {
            effort: Some(EffortSupport::Supported(EffortLevels::new([Low, High]))),
            ..ModelOverride::default()
        };
        let adapter = OllamaAdapter::new(
            knob_config(&server.url(), "llama3")
                .with_reasoning_wire(ReasoningWire::ThinkingObject)
                .with_model_overrides(BTreeMap::from([("llama3".to_string(), over)])),
        )
        .unwrap();
        // Construction made no network call.
        assert!(captured.lock().unwrap().is_none());

        let (body, _) = send_capturing(
            &adapter,
            &captured,
            plain_request("llama3").with_reasoning(High),
        )
        .await
        .expect("a declared wire and a listed level are accepted");
        assert_eq!(
            body["thinking"],
            json!({"type": "enabled", "effort": "high"})
        );
        assert!(body.get("reasoning_effort").is_none(), "{body}");
    }

    #[tokio::test]
    async fn the_default_wire_refuses_an_effort_and_names_the_wire_key() {
        use crate::compat::test_support::{mock_completion, plain_request, send_capturing, server};
        use paladin_ports::output::llm_port::ReasoningEffort::High;

        let mut server = server().await;
        let captured = mock_completion(&mut server).await;
        let adapter = OllamaAdapter::new(knob_config(&server.url(), "llama3")).unwrap();
        let error = send_capturing(
            &adapter,
            &captured,
            plain_request("llama3").with_reasoning(High),
        )
        .await
        .expect_err("an undeclared wire refuses");
        match error {
            LlmError::UnsupportedParameter { reason, .. } => {
                assert!(reason.contains("llm.ollama.reasoning_wire"), "{reason}");
            }
            other => panic!("expected UnsupportedParameter, got {other:?}"),
        }
        assert!(captured.lock().unwrap().is_none());
    }

    /// D-09, G-2: a self-hosted preset's pre-judge reads the declared wire and the provider-level
    /// `reasoning_effort` / `reasoning_fallback`, and `generate` agrees with it case by case.
    #[tokio::test]
    async fn ollama_check_reasoning_consults_the_provider_defaults() {
        use crate::compat::test_support::{agreed_verdict, mock_completion, server};
        use paladin_ports::output::llm_port::ReasoningEffort::{High, Low, Medium};
        use paladin_ports::output::model_profile::{EffortLevels, EffortSupport};

        let mut server = server().await;
        let _captured = mock_completion(&mut server).await;
        let over = ModelOverride {
            effort: Some(EffortSupport::Supported(EffortLevels::new([Low, High]))),
            ..ModelOverride::default()
        };
        let build = |effort: Option<ReasoningEffort>, fallback: Option<ReasoningFallback>| {
            let mut config = knob_config(&server.url(), "llama3")
                .with_reasoning_wire(ReasoningWire::TopLevelEffort)
                .with_model_overrides(BTreeMap::from([("llama3".to_string(), over.clone())]));
            if let Some(effort) = effort {
                config = config.with_reasoning_effort(effort);
            }
            if let Some(fallback) = fallback {
                config = config.with_reasoning_fallback(fallback);
            }
            OllamaAdapter::new(config).unwrap()
        };

        // A provider-level nearest_lower lets medium substitute on a [low, high] model; without
        // it the same request is refused.
        let substituting = build(None, Some(ReasoningFallback::NearestLower));
        assert_eq!(
            agreed_verdict(&substituting, "llama3", Some(Medium), None).await,
            None
        );
        assert!(
            agreed_verdict(&build(None, None), "llama3", Some(Medium), None)
                .await
                .is_some()
        );

        // A provider-level effort medium with no fallback: a no-effort check refuses, naming the
        // line that set the effort. A request-level effort the model lists beats it.
        let defaulted = build(Some(Medium), None);
        let reason = agreed_verdict(&defaulted, "llama3", None, None)
            .await
            .expect("a provider default of medium is refused on a [low, high] model");
        assert!(
            reason.ends_with("(set by `llm.ollama.reasoning_effort`)"),
            "{reason}"
        );
        assert_eq!(
            agreed_verdict(&defaulted, "llama3", Some(High), None).await,
            None
        );
    }

    /// D-10, T-51-54: a preset left at its compiled `ReasoningWire::Unsupported` refuses an
    /// explicit effort in `check_reasoning` exactly as at send time, naming the wire key.
    #[tokio::test]
    async fn ollama_check_reasoning_refuses_on_the_undeclared_wire() {
        use crate::compat::test_support::{agreed_verdict, mock_completion, server};
        use paladin_ports::output::llm_port::ReasoningEffort::High;

        let mut server = server().await;
        let _captured = mock_completion(&mut server).await;
        let adapter = OllamaAdapter::new(knob_config(&server.url(), "llama3")).unwrap();
        let reason = agreed_verdict(&adapter, "llama3", Some(High), None)
            .await
            .expect("an undeclared wire refuses");
        assert!(reason.contains("llm.ollama.reasoning_wire"), "{reason}");
        // Nothing in force: admitted on the same adapter.
        assert_eq!(agreed_verdict(&adapter, "llama3", None, None).await, None);
    }
}
