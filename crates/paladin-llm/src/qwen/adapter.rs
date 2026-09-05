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
use std::env;

use paladin_ports::output::llm_port::{
    LlmError, LlmPort, LlmRequest, LlmResponse, ProviderCapabilities, StreamingResponse,
};

use crate::compat::{
    CompatCapabilities, CompatEngine, CompatEngineConfig, CompatRequestParameters,
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
/// `qwen-plus` over `qwen3.7-plus`, live-verified 2026-08-23: both are
/// present in the live catalog at [`QWEN_DEFAULT_BASE_URL`] and both accept
/// every sampling parameter this preset declares, so the choice is not about
/// capability. It is about resilience to retirement. The live catalog
/// carries `qwen-plus` alongside dated snapshots of it going back to
/// `qwen-plus-2025-01-25` — evidence that `qwen-plus` is a rolling alias
/// Alibaba has already silently re-pointed multiple times while keeping the
/// name stable, the same pattern a caller-facing "always current" name gives
/// elsewhere. `qwen3.7-plus` is a generation-pinned name: the live catalog
/// already carries a newer `qwen3.8-*` generation alongside it, so
/// `qwen3.7-plus` is a future retirement candidate in exactly the way
/// `moonshot-v1-8k` and `gemini-2.5-flash` were pinned names this same phase
/// had to refresh after the vendor moved on (17-19, 17-20). A rolling alias
/// is the more durable shipped default.
pub const QWEN_DEFAULT_MODEL: &str = "qwen-plus";

/// Curated fallback model list (D-13), returned when the live `/models`
/// endpoint fails, is unreachable, or returns an empty list. Never reported
/// as authoritative — see [`crate::compat::engine::CompatEngine::available_models`].
///
/// Re-verified live 2026-08-23 at [`QWEN_DEFAULT_BASE_URL`] (162-model
/// catalog): all three entries are present, undated, general-purpose model
/// names (no `-vl`/`-audio`/`-image`/`-mt`/`-coder`/`-tts`/`-asr` specialised
/// suffix), and the first entry matches [`QWEN_DEFAULT_MODEL`].
pub const QWEN_FALLBACK_MODELS: &[&str] = &["qwen-plus", "qwen-turbo", "qwen3-max"];

/// Configuration for the Qwen (Alibaba DashScope) adapter.
#[derive(Debug, Clone)]
pub struct QwenConfig {
    /// API key for Qwen (DashScope) authentication.
    pub api_key: String,
    /// Base URL for the Qwen compatible-mode API.
    pub base_url: String,
    /// Default model to use.
    pub model: String,
    /// Request timeout in seconds.
    pub timeout_seconds: u64,
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
        let timeout_seconds = timeout_seconds
            .unwrap_or_else(|| "60".to_string())
            .parse()
            .map_err(|_| "Invalid DASHSCOPE_TIMEOUT_SECONDS value".to_string())?;

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
            max_retries: 3,
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
                max_context_tokens: Some(131_072),
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
            request_parameters: CompatRequestParameters::all(),
            fallback_models: QWEN_FALLBACK_MODELS.iter().map(|s| s.to_string()).collect(),
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

        Ok(Self {
            // Phase 25 D-03: name the engine so every `ProviderError` it
            // emits carries "qwen", not the engine's generic default.
            engine: CompatEngine::new(engine_config)?.with_provider_name(QWEN_PROVIDER),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http_status::map_http_status;
    use mockito::Server;
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
        assert_eq!(config.timeout_seconds, 60);
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

        let config = QwenConfig::new(
            "test-key".to_string(),
            server.url(),
            "qwen-plus".to_string(),
        );
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

        let config = QwenConfig::new(
            "test-key".to_string(),
            server.url(),
            "qwen-plus".to_string(),
        );
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

        let config = QwenConfig::new(
            "test-key".to_string(),
            server.url(),
            "qwen-plus".to_string(),
        );
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

        let config = QwenConfig::new(
            "test-key".to_string(),
            server.url(),
            "qwen-plus".to_string(),
        );
        let adapter = QwenAdapter::new(config).unwrap();

        let result = adapter.generate(build_request("qwen-plus")).await;
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

        let config = QwenConfig::new(
            "test-key".to_string(),
            server.url(),
            "qwen-plus".to_string(),
        );
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

        let config = QwenConfig::new(
            "test-key".to_string(),
            server.url(),
            "qwen-plus".to_string(),
        );
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

        let config = QwenConfig::new(
            "test-key".to_string(),
            server.url(),
            "qwen-plus".to_string(),
        );
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
        let config = QwenConfig::new(
            "test-key".to_string(),
            QWEN_DEFAULT_BASE_URL.to_string(),
            QWEN_DEFAULT_MODEL.to_string(),
        );
        let adapter = QwenAdapter::new(config).unwrap();
        assert_eq!(adapter.get_provider_name(), "qwen");
    }

    #[test]
    fn get_capabilities_reports_no_vision_and_no_tool_or_function_calling() {
        let config = QwenConfig::new(
            "test-key".to_string(),
            QWEN_DEFAULT_BASE_URL.to_string(),
            QWEN_DEFAULT_MODEL.to_string(),
        );
        let adapter = QwenAdapter::new(config).unwrap();
        let caps = adapter.get_capabilities();

        assert!(!caps.supports_tool_calling);
        assert!(!caps.supports_function_calling);
        assert!(!caps.supports_vision);
        assert!(!caps.supports_embeddings);
        assert!(caps.supports_streaming);
        assert!(caps.supports_system_messages);
    }
}
