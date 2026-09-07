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
use std::env;

use paladin_ports::output::llm_port::{
    LlmError, LlmPort, LlmRequest, LlmResponse, ProviderCapabilities, StreamingResponse,
};

use crate::compat::{
    CompatCapabilities, CompatEngine, CompatEngineConfig, CompatRequestParameters,
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
pub struct OllamaConfig {
    /// Base URL for the Ollama compat-layer API.
    pub base_url: String,
    /// Default model to use.
    pub model: String,
    /// Request timeout in seconds. Defaults to `120` rather than the
    /// hosted-preset `60` — a local model on cold start is slower than a
    /// hosted endpoint (T-17-17).
    pub timeout_seconds: u64,
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
        let timeout_seconds = timeout_seconds
            .unwrap_or_else(|| "120".to_string())
            .parse()
            .map_err(|_| "Invalid OLLAMA_TIMEOUT_SECONDS value".to_string())?;

        let config = Self {
            base_url,
            model,
            timeout_seconds,
        };

        config.validate()?;
        Ok(config)
    }

    /// Create configuration with custom values.
    pub fn new(base_url: String, model: String) -> Self {
        Self {
            base_url,
            model,
            timeout_seconds: 120,
        }
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), String> {
        if self.base_url.is_empty() {
            return Err("Base URL cannot be empty".to_string());
        }
        if !self.base_url.starts_with("http") {
            return Err("Base URL must start with http or https".to_string());
        }
        if self.model.is_empty() {
            return Err("Model name cannot be empty".to_string());
        }
        Ok(())
    }
}

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
            max_retries: 3,
            capabilities: CompatCapabilities {
                supports_streaming: true,
                supports_tool_calling: false,
                supports_function_calling: false,
                supports_vision: false,
                supports_embeddings: false,
                // The context window depends on whichever model the
                // operator pulled — declaring a number would be a claim
                // nobody asserted.
                max_context_tokens: None,
                supports_system_messages: true,
                temperature_range: Some((0.0, 2.0)),
            },
            // Unchanged pre-existing behaviour (17-18): no vendor-specific
            // sampling-parameter restriction has been measured for Ollama.
            request_parameters: CompatRequestParameters::all(),
            fallback_models: OLLAMA_FALLBACK_MODELS
                .iter()
                .map(|s| s.to_string())
                .collect(),
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

        Ok(Self {
            // Phase 25 D-03: name the engine so every `ProviderError` it
            // emits carries "ollama", not the engine's generic default.
            engine: CompatEngine::new(engine_config)?.with_provider_name(OLLAMA_PROVIDER),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http_status::map_http_status;
    use mockito::Server;
    use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
    use paladin_ports::output::llm_port::FinishReason;
    use serde_json::json;

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
}
