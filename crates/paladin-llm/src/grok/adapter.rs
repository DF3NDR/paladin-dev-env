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
use std::env;

use paladin_ports::output::llm_port::{
    LlmError, LlmPort, LlmRequest, LlmResponse, ProviderCapabilities, StreamingResponse,
};

use crate::compat::{
    CompatCapabilities, CompatEngine, CompatEngineConfig, CompatRequestParameters,
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
/// Live-verified 2026-08-22 (17-18, D-00e) against the real `GET
/// {GROK_DEFAULT_BASE_URL}/models` response, **not** copied from
/// `17-UAT.md`'s 2026-08-22 snapshot (that snapshot is evidence the old
/// `"grok-4"` value was wrong, not authority for this one). At that live
/// snapshot `grok-4` and `grok-3` were both **absent** from the catalog;
/// the highest general-purpose `grok-4.x` line present was `grok-4.6`
/// (created after `grok-4.5` and `grok-4.3`, no embedded date or build
/// suffix — the separately-versioned `grok-4.20-*` reasoning/non-reasoning
/// family carries dated ids like `grok-4.20-0309-reasoning` and was
/// excluded on that basis, per the plan's prohibition on dated snapshot
/// identifiers). A live `generate()` call to this model, with the
/// framework's default prompt parameters and this preset's
/// `request_parameters` declaration below, returned a real completion.
pub const GROK_DEFAULT_MODEL: &str = "grok-4.6";

/// Curated fallback model list (D-13), returned when the live `/models`
/// endpoint fails, is unreachable, or returns an empty list. Never reported
/// as authoritative — see [`crate::compat::engine::CompatEngine::available_models`].
///
/// Live-verified 2026-08-22 (17-18, D-00e), newest first, default first:
/// the three stable general-purpose entries from the same live catalog
/// that produced [`GROK_DEFAULT_MODEL`] — `grok-4.6` (created
/// 2026-08-06), `grok-4.5` (created 2026-06-29) and `grok-4.3` (created
/// 2026-04-17). Excludes the image/video/build-tool models
/// (`grok-imagine-*`, `grok-build-0.1`) and the dated `grok-4.20-*`
/// family, none of which are the general-purpose chat line this preset
/// targets.
pub const GROK_FALLBACK_MODELS: &[&str] = &["grok-4.6", "grok-4.5", "grok-4.3"];

/// Configuration for the Grok (xAI) adapter.
#[derive(Debug, Clone)]
pub struct GrokConfig {
    /// API key for Grok (xAI) authentication.
    pub api_key: String,
    /// Base URL for the Grok API.
    pub base_url: String,
    /// Default model to use.
    pub model: String,
    /// Request timeout in seconds.
    pub timeout_seconds: u64,
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
        let timeout_seconds = timeout_seconds
            .unwrap_or_else(|| "60".to_string())
            .parse()
            .map_err(|_| "Invalid XAI_TIMEOUT_SECONDS value".to_string())?;

        let config = Self {
            api_key,
            base_url,
            model,
            timeout_seconds,
        };

        config.validate()?;
        Ok(config)
    }

    /// Create configuration with custom values.
    pub fn new(api_key: String, base_url: String, model: String) -> Self {
        Self {
            api_key,
            base_url,
            model,
            timeout_seconds: 60,
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
            max_retries: 3,
            capabilities: CompatCapabilities {
                supports_streaming: true,
                supports_tool_calling: false,
                supports_function_calling: false,
                supports_vision: false,
                supports_embeddings: false,
                max_context_tokens: Some(131_072),
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

        Ok(Self {
            // Phase 25 D-03: name the engine so every `ProviderError` it
            // emits carries "grok", not the engine's generic default.
            engine: CompatEngine::new(engine_config)?.with_provider_name(GROK_PROVIDER),
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
        assert_eq!(config.timeout_seconds, 60);
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

        let config = GrokConfig::new(
            "test-key".to_string(),
            server.url(),
            GROK_DEFAULT_MODEL.to_string(),
        );
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

        let config = GrokConfig::new(
            "test-key".to_string(),
            server.url(),
            GROK_DEFAULT_MODEL.to_string(),
        );
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

        let config = GrokConfig::new(
            "test-key".to_string(),
            server.url(),
            GROK_DEFAULT_MODEL.to_string(),
        );
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

        let config = GrokConfig::new(
            "test-key".to_string(),
            server.url(),
            GROK_DEFAULT_MODEL.to_string(),
        );
        let adapter = GrokAdapter::new(config).unwrap();

        let result = adapter.generate(build_request(GROK_DEFAULT_MODEL)).await;
        assert!(matches!(result, Err(LlmError::RateLimitExceeded)));
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

        let config = GrokConfig::new(
            "test-key".to_string(),
            server.url(),
            GROK_DEFAULT_MODEL.to_string(),
        );
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

        let config = GrokConfig::new(
            "test-key".to_string(),
            server.url(),
            GROK_DEFAULT_MODEL.to_string(),
        );
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

        let config = GrokConfig::new(
            "test-key".to_string(),
            server.url(),
            GROK_DEFAULT_MODEL.to_string(),
        );
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
        let config = GrokConfig::new(
            "test-key".to_string(),
            GROK_DEFAULT_BASE_URL.to_string(),
            GROK_DEFAULT_MODEL.to_string(),
        );
        let adapter = GrokAdapter::new(config).unwrap();
        assert_eq!(adapter.get_provider_name(), "grok");
    }

    #[test]
    fn get_capabilities_reports_no_tool_or_function_calling() {
        let config = GrokConfig::new(
            "test-key".to_string(),
            GROK_DEFAULT_BASE_URL.to_string(),
            GROK_DEFAULT_MODEL.to_string(),
        );
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

        let config = GrokConfig::new(
            "test-key".to_string(),
            server.url(),
            GROK_DEFAULT_MODEL.to_string(),
        );
        let adapter = GrokAdapter::new(config).unwrap();

        let mut request = build_request(GROK_DEFAULT_MODEL);
        request.prompt.node.node.parameters =
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

    // Test 2: the curated fallback can never disagree with the default
    // about which model to reach for.
    #[test]
    fn fallback_models_is_non_empty_and_starts_with_the_default_model() {
        assert!(!GROK_FALLBACK_MODELS.is_empty());
        assert_eq!(GROK_FALLBACK_MODELS[0], GROK_DEFAULT_MODEL);
    }
}
