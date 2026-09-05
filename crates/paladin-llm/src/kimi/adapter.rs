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
use std::env;

use paladin_ports::output::llm_port::{
    LlmError, LlmPort, LlmRequest, LlmResponse, ProviderCapabilities, StreamingResponse,
};

use crate::compat::{
    CompatCapabilities, CompatEngine, CompatEngineConfig, CompatRequestParameters,
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
/// `kimi-k3` — the highest-numbered general-purpose line in the live
/// `GET /models` response measured 2026-08-22 (17-19, closing G-17-4b),
/// chosen over the two code-specialised entries (`kimi-k2.7-code`,
/// `kimi-k2.7-code-highspeed`) present in that same response, since this is
/// the default a caller gets when they express no preference. The retired
/// `moonshot-v1-8k` this constant previously held is absent from the live
/// list entirely.
pub const KIMI_DEFAULT_MODEL: &str = "kimi-k3";

/// Curated fallback model list (D-13), returned when the live `/models`
/// endpoint fails, is unreachable, or returns an empty list. Never reported
/// as authoritative — see [`crate::compat::engine::CompatEngine::available_models`].
///
/// Populated from the same 2026-08-22 live `GET /models` response as
/// [`KIMI_DEFAULT_MODEL`] (default first), replacing the retired
/// `moonshot-v1-*` family this constant previously held.
pub const KIMI_FALLBACK_MODELS: &[&str] = &[
    KIMI_DEFAULT_MODEL,
    "kimi-k2.6",
    "kimi-k2.7-code",
    "kimi-k2.7-code-highspeed",
];

/// Configuration for the Kimi (Moonshot AI) adapter.
#[derive(Debug, Clone)]
pub struct KimiConfig {
    /// API key for Kimi authentication.
    pub api_key: String,
    /// Base URL for the Kimi API.
    pub base_url: String,
    /// Default model to use.
    pub model: String,
    /// Request timeout in seconds.
    pub timeout_seconds: u64,
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
        let timeout_seconds = timeout_seconds
            .unwrap_or_else(|| "60".to_string())
            .parse()
            .map_err(|_| "Invalid MOONSHOT_TIMEOUT_SECONDS value".to_string())?;

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
            max_retries: 3,
            capabilities: CompatCapabilities {
                supports_streaming: true,
                supports_tool_calling: false,
                supports_function_calling: false,
                supports_vision: false,
                supports_embeddings: false,
                max_context_tokens: Some(131_072),
                supports_system_messages: true,
                // Live-measured 2026-08-22 (17-19, closing G-17-4b) against
                // `kimi-k3` (`KIMI_DEFAULT_MODEL`) and independently
                // confirmed on `kimi-k2.6`: a request with `temperature`
                // omitted or `1.0` succeeds; `0.7` (the framework's
                // `PromptParameters`/`PaladinData` default) is rejected with
                // the vendor's own `{"message":"invalid temperature: only 1
                // is allowed for this model","type":"invalid_request_error"}`.
                // The declared range is therefore the truthful degenerate
                // `(1.0, 1.0)`, replacing the previous unmeasured
                // `(0.0, 1.0)` — see 17-19-SUMMARY.md for the full probe
                // transcript. `test_every_adapter_declares_a_temperature_range`
                // (`lib.rs`) still holds: `Some(_)`, never `None`.
                temperature_range: Some((1.0, 1.0)),
            },
            // Option (a), chosen by the developer 2026-08-22 against
            // ADR-0004 (17-19, closing G-17-4b): Kimi's request path does
            // NOT carry `temperature` — the key is omitted from the
            // outgoing body entirely rather than a legal value being
            // substituted for another (see
            // `CompatRequestParameters::temperature`'s rustdoc for the full
            // ADR-0004 reasoning). The vendor's own single legal value
            // (`1`) applies server-side once the key is absent.
            //
            // `top_p` is declared unsupported for the same reason, on a
            // separate measured constraint discovered while re-running the
            // live harness after the temperature fix landed (Rule 1,
            // in-scope bugfix, not a plan-scope expansion — ADR-0004 governs
            // `temperature` only; `top_p` has no `ProviderCapabilities`
            // range field and no builder-side gate, so dropping it needs no
            // equivalent to Task 2's narrowing): with `temperature` no
            // longer sent, the framework's `PromptParameters::default()`
            // `top_p: Some(1.0)` became the next value the vendor's
            // per-field validation rejected —
            // `{"message":"invalid top_p: only 0.95 is allowed for this
            // model","type":"invalid_request_error"}` — measured on both
            // `kimi-k3` and `kimi-k2.6`, 2026-08-22. `max_tokens`,
            // `frequency_penalty` and `presence_penalty` remain carried,
            // unchanged from 17-18's behaviour-preserving `all()`
            // declaration.
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
                frequency_penalty: true,
                presence_penalty: true,
            },
            fallback_models: KIMI_FALLBACK_MODELS.iter().map(|s| s.to_string()).collect(),
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

        Ok(Self {
            // Phase 25 D-03: name the engine so every `ProviderError` it
            // emits carries "kimi", not the engine's generic default.
            engine: CompatEngine::new(engine_config)?.with_provider_name(KIMI_PROVIDER),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http_status::map_http_status;
    use mockito::{Matcher, Server};
    use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
    use paladin_ports::output::llm_port::FinishReason;
    use serde_json::json;
    use std::collections::HashMap;
    use uuid::Uuid;

    fn build_request(model: &str) -> LlmRequest {
        LlmRequest {
            id: Uuid::new_v4(),
            model: model.to_string(),
            prompt: PromptItem::new(PromptType::User(UserPrompt {
                query: "Hello".to_string(),
                context: None,
            }))
            .unwrap(),
            attachments: vec![],
            stream: false,
            metadata: HashMap::new(),
        }
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
        assert_eq!(config.timeout_seconds, 60);
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

        let config = KimiConfig::new(
            "test-key".to_string(),
            server.url(),
            KIMI_DEFAULT_MODEL.to_string(),
        );
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

        let config = KimiConfig::new(
            "test-key".to_string(),
            server.url(),
            KIMI_DEFAULT_MODEL.to_string(),
        );
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

        let config = KimiConfig::new(
            "test-key".to_string(),
            server.url(),
            KIMI_DEFAULT_MODEL.to_string(),
        );
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

        let config = KimiConfig::new(
            "test-key".to_string(),
            server.url(),
            KIMI_DEFAULT_MODEL.to_string(),
        );
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

        let config = KimiConfig::new(
            "test-key".to_string(),
            server.url(),
            KIMI_DEFAULT_MODEL.to_string(),
        );
        let adapter = KimiAdapter::new(config).unwrap();

        let response = adapter
            .generate(build_request(KIMI_DEFAULT_MODEL))
            .await
            .unwrap();

        assert!(matches!(response.finish_reason, FinishReason::Stop));
    }

    #[tokio::test]
    async fn null_message_content_deserializes_to_empty_string_without_panicking() {
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

        let config = KimiConfig::new(
            "test-key".to_string(),
            server.url(),
            KIMI_DEFAULT_MODEL.to_string(),
        );
        let adapter = KimiAdapter::new(config).unwrap();

        let response = adapter
            .generate(build_request(KIMI_DEFAULT_MODEL))
            .await
            .unwrap();

        assert_eq!(response.content, "");
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

        let config = KimiConfig::new(
            "test-key".to_string(),
            server.url(),
            KIMI_DEFAULT_MODEL.to_string(),
        );
        let adapter = KimiAdapter::new(config).unwrap();

        let result = adapter.generate(build_request(KIMI_DEFAULT_MODEL)).await;

        assert!(matches!(result, Err(LlmError::EmptyCompletion(_))));
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

        let config = KimiConfig::new(
            "test-key".to_string(),
            server.url(),
            KIMI_DEFAULT_MODEL.to_string(),
        );
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

        let config = KimiConfig::new(
            "test-key".to_string(),
            server.url(),
            KIMI_DEFAULT_MODEL.to_string(),
        );
        let adapter = KimiAdapter::new(config).unwrap();

        let result = adapter.generate(build_request(KIMI_DEFAULT_MODEL)).await;
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

        let config = KimiConfig::new(
            "test-key".to_string(),
            server.url(),
            KIMI_DEFAULT_MODEL.to_string(),
        );
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

        let config = KimiConfig::new(
            "test-key".to_string(),
            server.url(),
            KIMI_DEFAULT_MODEL.to_string(),
        );
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

        let config = KimiConfig::new(
            "test-key".to_string(),
            server.url(),
            KIMI_DEFAULT_MODEL.to_string(),
        );
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

        let config = KimiConfig::new(
            "test-key".to_string(),
            server.url(),
            KIMI_DEFAULT_MODEL.to_string(),
        );
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

        let config = KimiConfig::new(
            secret.to_string(),
            server.url(),
            KIMI_DEFAULT_MODEL.to_string(),
        );
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

        let config = KimiConfig::new(
            "test-key".to_string(),
            server.url(),
            KIMI_DEFAULT_MODEL.to_string(),
        );
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
        let unreachable_config = KimiConfig::new(
            "test-key".to_string(),
            "http://127.0.0.1:1".to_string(),
            KIMI_DEFAULT_MODEL.to_string(),
        );
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

        let config = KimiConfig::new(
            "test-key".to_string(),
            server.url(),
            KIMI_DEFAULT_MODEL.to_string(),
        );
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
    async fn generate_stream_with_only_done_terminates_with_stop_and_no_error() {
        use futures::StreamExt;

        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body("data: [DONE]\n\n")
            .create_async()
            .await;

        let config = KimiConfig::new(
            "test-key".to_string(),
            server.url(),
            KIMI_DEFAULT_MODEL.to_string(),
        );
        let adapter = KimiAdapter::new(config).unwrap();

        let stream = adapter
            .generate_stream(build_request(KIMI_DEFAULT_MODEL))
            .await
            .unwrap();
        let mut stream = Box::into_pin(stream);

        let mut saw_stop = false;
        while let Some(item) = stream.next().await {
            let chunk = item.expect("a DONE-only stream must not error");
            if matches!(chunk.finish_reason, Some(FinishReason::Stop)) {
                saw_stop = true;
            }
        }

        assert!(
            saw_stop,
            "a stream with only [DONE] must still terminate with Stop"
        );
    }

    // ── Capabilities / identity ──

    #[test]
    fn get_provider_name_returns_kimi() {
        let config = KimiConfig::new(
            "test-key".to_string(),
            KIMI_DEFAULT_BASE_URL.to_string(),
            KIMI_DEFAULT_MODEL.to_string(),
        );
        let adapter = KimiAdapter::new(config).unwrap();
        assert_eq!(adapter.get_provider_name(), "kimi");
    }

    #[test]
    fn get_capabilities_reports_no_tool_or_function_calling() {
        let config = KimiConfig::new(
            "test-key".to_string(),
            KIMI_DEFAULT_BASE_URL.to_string(),
            KIMI_DEFAULT_MODEL.to_string(),
        );
        let adapter = KimiAdapter::new(config).unwrap();
        let caps = adapter.get_capabilities();

        assert!(!caps.supports_tool_calling);
        assert!(!caps.supports_function_calling);
        assert!(!caps.supports_vision);
        assert!(!caps.supports_embeddings);
        assert!(caps.supports_streaming);
        assert!(caps.supports_system_messages);
    }

    // ── Live-measured temperature constraint (17-19, closing G-17-4b) ──

    #[test]
    fn get_capabilities_reports_the_measured_degenerate_temperature_range() {
        // The cross-adapter invariant in `lib.rs`
        // (`test_every_adapter_declares_a_temperature_range`) requires
        // `Some(_)`; this pins the exact measured value, not merely its
        // presence.
        let config = KimiConfig::new(
            "test-key".to_string(),
            KIMI_DEFAULT_BASE_URL.to_string(),
            KIMI_DEFAULT_MODEL.to_string(),
        );
        let adapter = KimiAdapter::new(config).unwrap();
        let caps = adapter.get_capabilities();

        assert_eq!(caps.temperature_range, Some((1.0, 1.0)));
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

        let config = KimiConfig::new(
            "test-key".to_string(),
            server.url(),
            KIMI_DEFAULT_MODEL.to_string(),
        );
        let adapter = KimiAdapter::new(config).unwrap();

        let result = adapter.generate(build_request(KIMI_DEFAULT_MODEL)).await;
        assert!(
            result.is_ok(),
            "mock returned a well-formed response: {result:?}"
        );

        let body = captured
            .lock()
            .unwrap()
            .take()
            .expect("mock must have been called exactly once");
        assert!(
            !body.contains("temperature"),
            "the key name itself must not appear on the wire — got body: {body}"
        );
    }

    #[tokio::test]
    async fn generate_omits_top_p_from_the_moonshot_request_body() {
        // Same mechanism, second measured constraint (discovered re-running
        // the live harness after the temperature fix): with `temperature`
        // no longer sent, `PromptParameters::default()`'s `top_p: Some(1.0)`
        // became the next value the vendor's per-field validation rejected.
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
                    "id": "cmpl-notopp",
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

        let config = KimiConfig::new(
            "test-key".to_string(),
            server.url(),
            KIMI_DEFAULT_MODEL.to_string(),
        );
        let adapter = KimiAdapter::new(config).unwrap();

        let result = adapter.generate(build_request(KIMI_DEFAULT_MODEL)).await;
        assert!(
            result.is_ok(),
            "mock returned a well-formed response: {result:?}"
        );

        let body = captured
            .lock()
            .unwrap()
            .take()
            .expect("mock must have been called exactly once");
        assert!(
            !body.contains("top_p"),
            "the key name itself must not appear on the wire — got body: {body}"
        );
        // max_tokens, frequency_penalty and presence_penalty remain
        // carried — proving this is a targeted omission of two fields, not
        // an accidental drop of every sampling parameter.
        assert!(body.contains("max_tokens"));
        assert!(body.contains("frequency_penalty"));
        assert!(body.contains("presence_penalty"));
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

        let config = KimiConfig::new(
            "test-key".to_string(),
            server.url(),
            KIMI_DEFAULT_MODEL.to_string(),
        );
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

        let config = KimiConfig::new(
            "test-key".to_string(),
            server.url(),
            KIMI_DEFAULT_MODEL.to_string(),
        );
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

        let config = KimiConfig::new(
            "test-key".to_string(),
            server.url(),
            KIMI_DEFAULT_MODEL.to_string(),
        );
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

        let config = KimiConfig::new(
            "test-key".to_string(),
            server.url(),
            KIMI_DEFAULT_MODEL.to_string(),
        );
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

        let config = KimiConfig::new(
            "test-key".to_string(),
            server.url(),
            KIMI_DEFAULT_MODEL.to_string(),
        );
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
            primary.url(),
            KIMI_DEFAULT_MODEL.to_string(),
        );
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
}
