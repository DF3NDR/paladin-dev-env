//! Anthropic Claude LLM Adapter
//!
//! Provides integration with Anthropic's Claude API.
//! Supports standard completions, streaming, and all core LlmPort functionality.
//! Claude has unique requirements: system messages separate from messages array,
//! max_tokens required, and different message structure.

use async_trait::async_trait;
use chrono::Utc;
use futures::{Stream, StreamExt};
use reqwest::{
    Client,
    header::{CONTENT_TYPE, HeaderMap, HeaderValue},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::time::Duration;
use uuid::Uuid;

use paladin_core::platform::container::prompt::PromptType;
use paladin_ports::output::llm_port::{
    FinishReason, LlmError, LlmPort, LlmRequest, LlmResponse, ProviderCapabilities,
    StreamingResponse, TokenUsage,
};

use crate::http_status::map_http_status;

/// The provider name this adapter reports through [`LlmPort::get_provider_name`]
/// and stamps on every [`LlmError::ProviderError`] it emits.
const ANTHROPIC_PROVIDER: &str = "anthropic";

/// The exact phrase observed VERBATIM in Anthropic's HTTP 400
/// `invalid_request_error` body when an account has reached its configured
/// API usage limit (live run `4a3b749d`). Matched narrowly and deliberately:
/// a too-broad match here would silently reclassify a genuine
/// malformed-prompt bug as "wait for quota reset" — a swallow-errors
/// anti-pattern with a friendly name (RESEARCH Pitfall #4 / threat
/// T-41-01). Checked AFTER the pre-existing `max_tokens` branch in
/// [`AnthropicAdapter::map_error`] so that branch is never shadowed.
const ANTHROPIC_USAGE_CAP_SIGNATURE: &str = "You have reached your specified API usage limits";

/// Configuration for Anthropic Claude LLM adapter.
#[derive(Debug, Clone)]
pub struct AnthropicConfig {
    /// API key for Anthropic authentication.
    pub api_key: String,
    /// Base URL for Anthropic API.
    pub base_url: String,
    /// Default model to use (e.g., claude-3-5-sonnet-20241022).
    pub model: String,
    /// Default max tokens for responses (required by Claude API).
    pub max_tokens: u32,
    /// Request timeout in seconds.
    pub timeout_seconds: u64,
}

impl AnthropicConfig {
    /// Load configuration from environment variables.
    ///
    /// # Environment Variables
    /// - `ANTHROPIC_API_KEY` (required): Anthropic API key
    /// - `ANTHROPIC_BASE_URL` (optional): API base URL
    /// - `ANTHROPIC_MODEL` (optional): Default model
    /// - `ANTHROPIC_MAX_TOKENS` (optional): Default max tokens
    /// - `ANTHROPIC_TIMEOUT_SECONDS` (optional): Request timeout
    ///
    /// # Errors
    /// Returns error if required environment variables are missing or invalid.
    pub fn from_env() -> Result<Self, String> {
        let api_key = env::var("ANTHROPIC_API_KEY")
            .map_err(|_| "ANTHROPIC_API_KEY environment variable not set")?;

        let base_url = env::var("ANTHROPIC_BASE_URL")
            .unwrap_or_else(|_| "https://api.anthropic.com/v1".to_string());

        let model = env::var("ANTHROPIC_MODEL")
            .unwrap_or_else(|_| "claude-3-5-sonnet-20241022".to_string());

        let max_tokens = env::var("ANTHROPIC_MAX_TOKENS")
            .unwrap_or_else(|_| "4096".to_string())
            .parse()
            .map_err(|_| "Invalid ANTHROPIC_MAX_TOKENS value")?;

        let timeout_seconds = env::var("ANTHROPIC_TIMEOUT_SECONDS")
            .unwrap_or_else(|_| "60".to_string())
            .parse()
            .map_err(|_| "Invalid ANTHROPIC_TIMEOUT_SECONDS value")?;

        let config = Self {
            api_key,
            base_url,
            model,
            max_tokens,
            timeout_seconds,
        };

        config.validate()?;
        Ok(config)
    }

    /// Create configuration with custom values.
    pub fn new(api_key: String, base_url: String, model: String, max_tokens: u32) -> Self {
        Self {
            api_key,
            base_url,
            model,
            max_tokens,
            timeout_seconds: 60,
        }
    }

    /// Validate configuration.
    pub fn validate(&self) -> Result<(), String> {
        if self.api_key.is_empty() {
            return Err("API key cannot be empty".to_string());
        }
        if self.base_url.is_empty() {
            return Err("Base URL cannot be empty".to_string());
        }
        if !self.base_url.starts_with("http://") && !self.base_url.starts_with("https://") {
            return Err("Base URL must start with http:// or https://".to_string());
        }
        if self.model.is_empty() {
            return Err("Model cannot be empty".to_string());
        }
        if self.max_tokens == 0 {
            return Err("Max tokens must be greater than 0".to_string());
        }
        Ok(())
    }
}

/// Anthropic Claude LLM Adapter implementing [`LlmPort`].
pub struct AnthropicAdapter {
    pub(crate) client: Client,
    pub(crate) config: AnthropicConfig,
}

impl AnthropicAdapter {
    /// Create a new Anthropic adapter with the given configuration.
    ///
    /// # Errors
    /// Returns error if configuration is invalid or HTTP client cannot be created.
    pub fn new(config: AnthropicConfig) -> Result<Self, LlmError> {
        config
            .validate()
            .map_err(|e| LlmError::AuthenticationError(format!("Invalid configuration: {}", e)))?;

        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            // CR-02 (`25-REVIEW.md`): `AnthropicConfig::base_url` is
            // operator-configurable and every request carries the
            // `x-api-key` credential header, which reqwest's built-in
            // cross-host redirect header-stripping does NOT cover (it only
            // strips `Authorization`/`Cookie`/`Cookie2`/`Proxy-Authorization`/
            // `WWW-Authenticate`). Refusing redirects means a `3xx` from
            // whatever host `base_url` resolves to can never replay the key
            // to a different, attacker-influenced host — matches every
            // `CompatEngine`-based preset and the bespoke Gemini adapter
            // (T-17-18/T-17-52). A refused redirect surfaces via
            // [`Self::map_error`]'s `300..=399` arm.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| LlmError::NetworkError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { client, config })
    }

    /// Build HTTP headers for Anthropic API requests.
    fn build_headers(&self) -> Result<HeaderMap, LlmError> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let api_key_header = HeaderValue::from_str(&self.config.api_key)
            .map_err(|e| LlmError::AuthenticationError(format!("Invalid API key format: {}", e)))?;
        headers.insert("x-api-key", api_key_header);
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));

        Ok(headers)
    }

    /// Convert LlmRequest to Claude API format.
    fn build_request(&self, request: &LlmRequest) -> Result<ClaudeRequest, LlmError> {
        let mut system_message = None;
        let mut messages = Vec::new();

        match &request.prompt.node.node.prompt_type {
            PromptType::System(system_prompt) => {
                system_message = Some(system_prompt.instructions.clone());
            }
            PromptType::User(user_prompt) => {
                messages.push(ClaudeMessage {
                    role: "user".to_string(),
                    content: user_prompt.query.clone(),
                });
            }
            PromptType::Text(text_prompt) => {
                let role = match &text_prompt.role {
                    paladin_core::platform::container::prompt::PromptRole::System => {
                        system_message = Some(text_prompt.content.clone());
                        return Ok(ClaudeRequest {
                            model: request.model.clone(),
                            messages: vec![],
                            system: system_message,
                            max_tokens: request
                                .prompt
                                .node
                                .node
                                .parameters
                                .max_tokens
                                .unwrap_or(self.config.max_tokens),
                            // Phase 23.1: current Claude models (e.g. claude-opus-4-8)
                            // reject an explicit `temperature` ("deprecated for this
                            // model") — and restrict other sampling overrides — so omit
                            // both and let the API default apply. `skip_serializing_if`
                            // drops the `None`s from the request body entirely.
                            temperature: None,
                            top_p: None,
                            stream: false,
                        });
                    }
                    paladin_core::platform::container::prompt::PromptRole::User => "user",
                    paladin_core::platform::container::prompt::PromptRole::Assistant => "assistant",
                    paladin_core::platform::container::prompt::PromptRole::Function => "user",
                };
                messages.push(ClaudeMessage {
                    role: role.to_string(),
                    content: text_prompt.content.clone(),
                });
            }
            PromptType::Assistant(assistant_prompt) => {
                messages.push(ClaudeMessage {
                    role: "assistant".to_string(),
                    content: assistant_prompt.response.clone(),
                });
            }
            PromptType::Function(_) => {
                return Err(LlmError::InvalidPrompt(
                    "Function prompts not yet supported for Claude".to_string(),
                ));
            }
        }

        let max_tokens = request
            .prompt
            .node
            .node
            .parameters
            .max_tokens
            .unwrap_or(self.config.max_tokens);

        Ok(ClaudeRequest {
            model: request.model.clone(),
            messages,
            system: system_message,
            max_tokens,
            // Phase 23.1: current Claude models (e.g. claude-opus-4-8) reject an
            // explicit `temperature` ("deprecated for this model") and restrict other
            // sampling overrides — omit both and let the API default apply
            // (`skip_serializing_if` drops the `None`s from the request body).
            temperature: None,
            top_p: None,
            stream: false,
        })
    }

    /// Parse Claude API response into LlmResponse.
    ///
    /// Fails loud via [`detect_no_text_content`] instead of ever returning
    /// `Ok` with empty content — an empty-string success is indistinguishable
    /// from a valid empty answer to every downstream caller, and is exactly
    /// what produced a 0-byte `PoC.t.sol` before this fix. Recovers ALL
    /// text-bearing blocks via [`concat_text_blocks`] in array order, not
    /// just the first, so a text block that follows a `thinking` block is no
    /// longer silently dropped.
    fn parse_response(
        &self,
        request_id: Uuid,
        response: ClaudeResponse,
    ) -> Result<LlmResponse, LlmError> {
        if let Some(err) =
            detect_no_text_content(&response.content, response.stop_reason.as_deref())
        {
            return Err(err);
        }

        let content = concat_text_blocks(&response.content);

        let finish_reason = match response.stop_reason.as_deref() {
            Some("end_turn") => FinishReason::Stop,
            Some("max_tokens") => FinishReason::Length,
            Some("stop_sequence") => FinishReason::Stop,
            _ => FinishReason::Error("unknown".to_string()),
        };

        Ok(LlmResponse {
            id: Uuid::new_v4(),
            request_id,
            model: response.model,
            content,
            finish_reason,
            usage: map_claude_usage(&response.usage),
            cost: None,
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: None,
        })
    }

    /// Map a non-2xx Anthropic response to [`LlmError`].
    ///
    /// Two Anthropic-specific pre-checks run first; everything else is the
    /// crate-wide [`map_http_status`] (Phase 25 D-03, FT-FR-01), which
    /// redacts the raw `body` before bounding it and emits a typed
    /// `ProviderError { status }` for every status without a dedicated
    /// variant.
    ///
    /// - `403` stays [`LlmError::AuthenticationError`]: Anthropic reports a
    ///   key without permission for the resource as `403`, and
    ///   [`Self::execute_with_retry`]'s non-retryable set halts on
    ///   `AuthenticationError`. Letting it fall through to the helper's
    ///   generic `ProviderError { 403 }` would re-transmit a rejected
    ///   credential up to `max_retries` times.
    /// - `400` is disambiguated on the body: a `max_tokens` complaint and
    ///   the usage-cap signature (Phase 41 D-04/D-05) are Anthropic-shaped,
    ///   so they are recognised here; any other `400` reaches the helper's
    ///   own `400` arm.
    /// - `300..=399` is named explicitly (CR-02, mirroring
    ///   `CompatEngine::map_error`/`GeminiAdapter::map_error`) because this
    ///   client's redirect policy is `none` (see [`Self::new`]), so a `3xx`
    ///   response is never followed — it arrives here as an ordinary
    ///   non-success status instead.
    fn map_error(&self, status: u16, body: &str) -> LlmError {
        match status {
            403 => LlmError::AuthenticationError(
                "API key does not have permission for this resource.".to_string(),
            ),
            400 if body.contains("max_tokens") => LlmError::InvalidPrompt(
                "Invalid max_tokens value. Claude requires max_tokens to be set.".to_string(),
            ),
            300..=399 => LlmError::ProviderError {
                provider: ANTHROPIC_PROVIDER.to_string(),
                status,
                message: format!(
                    "the configured base URL responded with a redirect (HTTP {status}), which \
                     this client refuses to follow because doing so would forward the \
                     credential header to a different, potentially attacker-influenced host. \
                     Correct the configured base-URL setting to point directly at the intended \
                     endpoint. Response excerpt: {}",
                    crate::redaction::diagnostic_excerpt(body, &self.config.api_key)
                ),
            },
            400 if body.contains(ANTHROPIC_USAGE_CAP_SIGNATURE) => {
                // Redact before extracting/bounding (load-bearing ordering,
                // see `crate::redaction`'s module doc): `body` is
                // attacker- or third-party-influenceable (a gateway in
                // front of `AnthropicConfig::base_url` that echoes request
                // context), and `regain_hint` is displayed VERBATIM to the
                // operator, so it must go through the same redact-then-bound
                // discipline as every other body-derived string in this
                // crate before `extract_regain_hint` slices and bounds it.
                let redacted = crate::redaction::redact_credentials(body, &self.config.api_key);
                LlmError::UsageLimitExceeded {
                    provider: ANTHROPIC_PROVIDER.to_string(),
                    regain_hint: extract_regain_hint(&redacted),
                }
            }
            _ => map_http_status(ANTHROPIC_PROVIDER, status, body, &self.config.api_key),
        }
    }

    /// Execute request with retry logic.
    async fn execute_with_retry<F, Fut, T>(
        &self,
        operation: F,
        max_retries: u32,
    ) -> Result<T, LlmError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, LlmError>>,
    {
        let mut attempt = 0;
        let mut delay_ms = 1000u64;

        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    attempt += 1;

                    // `EmptyCompletion` is deliberately non-retryable (D-04): the
                    // retried request is byte-for-byte identical, so a
                    // no-text truncation reproduces deterministically, and
                    // each retry is a multi-minute frontier-model
                    // generation. Mirrors the do-not-retry rule already
                    // documented on `LlmError::EmptyCompletion` itself.
                    //
                    // `UsageLimitExceeded` is also deliberately non-retryable: a
                    // usage cap will not clear on backoff — it resets on a
                    // provider-side billing schedule, not a short window. A
                    // per-provider breaker (D-06, a layer up, downstream of this
                    // adapter) is what decides whether to attempt the call at
                    // all; retrying here would burn retries before the breaker
                    // ever sees the error, defeating its "does not burn
                    // retries" guarantee.
                    if matches!(
                        e,
                        LlmError::AuthenticationError(_)
                            | LlmError::InvalidPrompt(_)
                            | LlmError::EmptyCompletion(_)
                            | LlmError::UsageLimitExceeded { .. }
                    ) {
                        return Err(e);
                    }

                    if attempt >= max_retries {
                        return Err(e);
                    }

                    let jitter = (rand::random::<f64>() * 200.0) as u64;
                    tokio::time::sleep(Duration::from_millis(delay_ms + jitter)).await;
                    delay_ms = (delay_ms * 2).min(10000);
                }
            }
        }
    }
}

#[async_trait]
impl LlmPort for AnthropicAdapter {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let claude_request = self.build_request(&request)?;
        let headers = self.build_headers()?;

        let operation = || async {
            let response = self
                .client
                .post(format!("{}/messages", self.config.base_url))
                .headers(headers.clone())
                .json(&claude_request)
                .send()
                .await
                .map_err(|e| LlmError::ProcessingError(format!("Request failed: {}", e)))?;

            let status = response.status().as_u16();

            if !response.status().is_success() {
                let body = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                return Err(self.map_error(status, &body));
            }

            let body = response.text().await.map_err(|e| {
                LlmError::NetworkError(format!("Failed to read Anthropic response body: {}", e))
            })?;

            let claude_response: ClaudeResponse = serde_json::from_str(&body).map_err(|e| {
                LlmError::ProcessingError(format!(
                    "Deserialization of Anthropic response body failed (likely schema drift \
                     — see the `thinking`-block precedent in this adapter's tests): {} — \
                     body excerpt: {}",
                    e,
                    // Redact BEFORE bounding (load-bearing ordering, see
                    // `crate::redaction`): a 2xx body from a gateway in front
                    // of `base_url` can echo request headers -- including the
                    // `x-api-key` credential -- back verbatim.
                    crate::redaction::diagnostic_excerpt(&body, &self.config.api_key)
                ))
            })?;

            self.parse_response(request.id, claude_response)
        };

        self.execute_with_retry(operation, 3).await
    }

    async fn generate_stream(
        &self,
        request: LlmRequest,
    ) -> Result<Box<dyn Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError> {
        let mut claude_request = self.build_request(&request)?;
        claude_request.stream = true;

        let headers = self.build_headers()?;

        let response = self
            .client
            .post(format!("{}/messages", self.config.base_url))
            .headers(headers)
            .json(&claude_request)
            .send()
            .await
            .map_err(|e| LlmError::ProcessingError(format!("Stream request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(self.map_error(status, &body));
        }

        // D-14 terminal-chunk contract: Anthropic's wire has no `[DONE]`
        // sentinel — `message_stop` IS the terminal chunk (D-15). Usage
        // arrives split across two earlier events: `message_start.message.usage`
        // carries the input/cache figures, `message_delta.usage` carries the
        // cumulative output/thinking figures. Both are held in state captured
        // by this `move` closure and merged (`merge_claude_stream_usage`) onto
        // the ONE `TokenUsage` attached to `message_stop`.
        //
        // `flat_map` rather than `map` (Rule 1 auto-fix, mirroring the
        // identical latent bug fixed in `CompatEngine`/`openai`/`deepseek` by
        // plan 31-03): a single network chunk — as `mockito`'s `with_body()`
        // realistically delivers a whole multi-frame SSE body in one chunk —
        // can carry more than one `data:` line. The prior `.map()` returned on
        // the FIRST matching line per chunk, silently dropping every
        // subsequent event (including `message_stop` itself) whenever a test
        // or a real transport delivered more than one frame per chunk.
        let mut held_usage: Option<ClaudeUsage> = None;

        let stream = response.bytes_stream().flat_map(move |chunk_result| {
            let items: Vec<Result<StreamingResponse, LlmError>> = match chunk_result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes).into_owned();
                    let mut items = Vec::new();

                    for line in text.lines() {
                        let Some(json_str) = line.strip_prefix("data: ") else {
                            continue;
                        };
                        if json_str.trim() == "[DONE]" {
                            continue;
                        }

                        let Ok(event) = serde_json::from_str::<ClaudeStreamEvent>(json_str) else {
                            continue;
                        };

                        match event.event_type.as_str() {
                            "content_block_delta" => {
                                if let Some(text) = event.delta.and_then(|d| d.text) {
                                    items.push(Ok(StreamingResponse::delta(text)));
                                }
                            }
                            "message_start" => {
                                if let Some(usage) = event.message.and_then(|m| m.usage) {
                                    held_usage =
                                        Some(merge_claude_stream_usage(held_usage.take(), usage));
                                }
                            }
                            "message_delta" => {
                                if let Some(usage) = event.usage {
                                    held_usage =
                                        Some(merge_claude_stream_usage(held_usage.take(), usage));
                                }
                            }
                            "message_stop" => {
                                let mut terminal = StreamingResponse::terminal(FinishReason::Stop);
                                if let Some(usage) = held_usage.take() {
                                    terminal = terminal.with_usage(map_claude_usage(&usage));
                                }
                                items.push(Ok(terminal));
                            }
                            _ => {}
                        }
                    }

                    items
                }
                Err(e) => vec![Err(LlmError::ProcessingError(format!(
                    "Stream error: {}",
                    e
                )))],
            };

            futures::stream::iter(items)
        });

        Ok(Box::new(stream))
    }

    async fn validate_model(&self, model: &str) -> Result<bool, LlmError> {
        let valid_models = [
            "claude-3-5-sonnet-20241022",
            "claude-3-5-sonnet-20240620",
            "claude-3-opus-20240229",
            "claude-3-sonnet-20240229",
            "claude-3-haiku-20240307",
            "claude-2.1",
            "claude-2.0",
            "claude-instant-1.2",
        ];
        Ok(valid_models.contains(&model))
    }

    async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
        Ok(vec![
            "claude-3-5-sonnet-20241022".to_string(),
            "claude-3-5-sonnet-20240620".to_string(),
            "claude-3-opus-20240229".to_string(),
            "claude-3-sonnet-20240229".to_string(),
            "claude-3-haiku-20240307".to_string(),
            "claude-2.1".to_string(),
            "claude-2.0".to_string(),
            "claude-instant-1.2".to_string(),
        ])
    }

    fn get_provider_name(&self) -> &'static str {
        ANTHROPIC_PROVIDER
    }

    fn get_capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_streaming: true,
            // `LlmRequest` carries no field through which a tool definition could
            // travel, and this adapter neither sends `tools` nor parses `tool_calls`
            // out of a response. The flag describes what this adapter does, not what
            // the vendor's API offers (WEB-03, D-14).
            supports_tool_calling: false,
            supports_function_calling: false,
            supports_vision: true,
            supports_embeddings: false,
            max_context_tokens: Some(200_000),
            supports_system_messages: true,
            temperature_range: Some((0.0, 1.0)),
        }
    }
}

// ── Claude API request/response types ───────────────────────────────────────

#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    messages: Vec<ClaudeMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    #[allow(dead_code)]
    id: String,
    model: String,
    content: Vec<ClaudeContent>,
    stop_reason: Option<String>,
    usage: ClaudeUsage,
}

/// A single content block in an Anthropic response.
///
/// Claude 5 models emit heterogeneous block types in the same `content`
/// array. A `thinking` block (extended/interleaved thinking) carries
/// `thinking` and `signature` fields and NO `text` key at all — verified
/// live against `claude-opus-5` and `claude-sonnet-5` on 2026-07-26 (see
/// this module's captured-fixture tests). `text` must therefore be
/// optional, not required: a required `String` here was the entire root
/// cause of this adapter's original thinking-block deserialization bug.
/// `content_type` is read by the block-type census in
/// [`detect_no_text_content`], so it is genuinely load-bearing, not dead
/// code kept only for documentation.
#[derive(Debug, Deserialize)]
struct ClaudeContent {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct ClaudeUsage {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
    /// Cached tokens read for this call. `#[serde(default)]`+`Option`
    /// (never a bare `u32`) so a provider that omits the key entirely
    /// yields `None` ("not reported", D-03) rather than a fabricated
    /// `Some(0)`; the field arrives as an explicit `0` on every one of the
    /// three pre-existing captured fixtures, so it deserializes to
    /// `Some(0)` for them, exactly as D-03 requires.
    #[serde(default)]
    cache_read_input_tokens: Option<u32>,
    /// Cached tokens written for this call. Same `Option` reasoning as
    /// [`Self::cache_read_input_tokens`].
    #[serde(default)]
    cache_creation_input_tokens: Option<u32>,
    /// Only present on a reasoning/thinking response.
    #[serde(default)]
    output_tokens_details: Option<ClaudeUsageOutputDetails>,
}

/// The `usage.output_tokens_details` sub-object Anthropic includes on a
/// reasoning/thinking response, carrying the thinking-token sub-count of
/// `output_tokens` (D-20: maps to [`TokenUsage::reasoning_tokens`]).
#[derive(Debug, Deserialize)]
struct ClaudeUsageOutputDetails {
    #[serde(default)]
    thinking_tokens: Option<u32>,
}

/// Maps Anthropic's wire-shape [`ClaudeUsage`] onto [`TokenUsage`], applying
/// the D-20 cache-inclusive `prompt_tokens` correction and populating the
/// cache/reasoning sub-counts wherever the payload carries them.
///
/// **The correction:** Anthropic's `input_tokens` EXCLUDES cached tokens —
/// before this correction, `TokenUsage::new(usage.input_tokens, ..)`
/// under-reported billed input on every call that used prompt caching.
/// `prompt_tokens` is now computed as
/// `input_tokens + cache_read_input_tokens + cache_creation_input_tokens`
/// (saturating), matching what Anthropic actually bills. `completion_tokens`
/// is `output_tokens` unchanged — it already includes any thinking tokens.
/// Shared by both [`AnthropicAdapter::parse_response`] (the non-streaming
/// path) and [`AnthropicAdapter::generate_stream`]'s event accumulator, so
/// the two paths cannot drift.
fn map_claude_usage(usage: &ClaudeUsage) -> TokenUsage {
    let cache_read = usage.cache_read_input_tokens.unwrap_or(0);
    let cache_write = usage.cache_creation_input_tokens.unwrap_or(0);
    let prompt_tokens = usage
        .input_tokens
        .saturating_add(cache_read)
        .saturating_add(cache_write);

    let mut mapped = TokenUsage::new(prompt_tokens, usage.output_tokens);
    if let Some(cache_read_input_tokens) = usage.cache_read_input_tokens {
        mapped = mapped.with_cache_read(cache_read_input_tokens);
    }
    if let Some(cache_creation_input_tokens) = usage.cache_creation_input_tokens {
        mapped = mapped.with_cache_write(cache_creation_input_tokens);
    }
    if let Some(thinking_tokens) = usage
        .output_tokens_details
        .as_ref()
        .and_then(|details| details.thinking_tokens)
    {
        mapped = mapped.with_reasoning(thinking_tokens);
    }
    mapped
}

/// Merges a newly-arrived Anthropic streaming usage payload onto the usage
/// accumulated so far (D-15). `message_start.message.usage` carries the
/// input and cache figures; `message_delta.usage` carries the cumulative
/// output and thinking figures. Each event reports only the fields it owns
/// (the other side's numeric fields default to `0`/`None` via
/// `#[serde(default)]`), so a field-level merge — not a blind overwrite —
/// is required to combine both into the one [`ClaudeUsage`] eventually
/// mapped onto the `message_stop` terminal chunk.
fn merge_claude_stream_usage(existing: Option<ClaudeUsage>, incoming: ClaudeUsage) -> ClaudeUsage {
    let Some(mut merged) = existing else {
        return incoming;
    };
    if incoming.input_tokens != 0 {
        merged.input_tokens = incoming.input_tokens;
    }
    if incoming.output_tokens != 0 {
        merged.output_tokens = incoming.output_tokens;
    }
    if incoming.cache_read_input_tokens.is_some() {
        merged.cache_read_input_tokens = incoming.cache_read_input_tokens;
    }
    if incoming.cache_creation_input_tokens.is_some() {
        merged.cache_creation_input_tokens = incoming.cache_creation_input_tokens;
    }
    if incoming.output_tokens_details.is_some() {
        merged.output_tokens_details = incoming.output_tokens_details;
    }
    merged
}

/// Character budget for [`extract_regain_hint`]'s extracted prose. This is
/// the T-41-03 mitigation against an oversized/adversarial provider body
/// flooding the operator's terminal.
const REGAIN_HINT_CHAR_BUDGET: usize = 120;

/// Best-effort extraction of "when access regains" prose from a usage-cap
/// error body, for display to the operator.
///
/// Deliberately tiny and tolerant: finds the case-insensitive substring
/// `"regain access"` in `body`; if absent, returns `None`. Otherwise takes
/// the remainder of that sentence (up to the first `"`, `}`, or the end of
/// the body, whichever comes first), collapses whitespace runs to single
/// ASCII spaces, and bounds the result to [`REGAIN_HINT_CHAR_BUDGET`]
/// CHARACTERS (never a byte slice — a multi-byte payload must not panic on
/// a slice boundary; mirrors the `BatchFailure::error_excerpt` discipline
/// at `crates/audit-agents/src/triage.rs:1010` in the downstream
/// superproject). Returns `None` if the trimmed result is empty — a
/// missing hint is never an error.
///
/// The returned string is best-effort prose displayed VERBATIM to the
/// operator. It must NEVER be parsed into a `DateTime` the pipeline
/// schedules against (D-06) — it is provider free text, not a structured
/// timestamp contract.
fn extract_regain_hint(body: &str) -> Option<String> {
    let lower_body = body.to_lowercase();
    let marker_byte_pos = lower_body.find("regain access")?;

    // Lowercasing can change a character's UTF-8 byte length for some
    // Unicode (rare, but possible). Rather than reuse a byte offset
    // computed against the lowercased string to index the ORIGINAL body
    // (which could land mid-character and panic on multi-byte input),
    // convert to a CHARACTER count and re-locate that many characters into
    // `body` — safe by construction, never a raw byte slice.
    let chars_before_marker = lower_body[..marker_byte_pos].chars().count();
    let remainder: String = body.chars().skip(chars_before_marker).collect();

    // `find` with an ASCII-only pattern always returns a valid char
    // boundary (ASCII bytes never appear inside a multi-byte UTF-8
    // sequence), so slicing `remainder` at `end` is safe.
    let end = remainder.find(['"', '}']).unwrap_or(remainder.len());
    let sentence = &remainder[..end];

    let collapsed = sentence.split_whitespace().collect::<Vec<_>>().join(" ");
    let bounded: String = collapsed.chars().take(REGAIN_HINT_CHAR_BUDGET).collect();
    let trimmed = bounded.trim();

    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Concatenate the text of every text-bearing content block, in array
/// order.
///
/// Any block without a `text` value — `thinking`, `tool_use`,
/// `redacted_thinking`, or any block type invented after this code was
/// written — is skipped by construction. This is deliberate: an unfamiliar
/// block type must degrade to being ignored, never to a hard failure. No
/// separator is inserted between blocks; the Anthropic API already splits
/// blocks at semantic boundaries rather than mid-word, and the streaming
/// path (see `generate_stream` below) likewise accumulates deltas without
/// inserting anything between them.
fn concat_text_blocks(content: &[ClaudeContent]) -> String {
    content
        .iter()
        .filter_map(|block| block.text.as_deref())
        .collect()
}

/// Detect a response with no recoverable text.
///
/// Fires when every content block is skipped by [`concat_text_blocks`] —
/// the concatenated text is empty or whitespace-only. This is the same
/// class of failure DeepSeek's `detect_empty_completion` guards against: a
/// reasoning model can spend its entire `max_tokens` budget on hidden
/// thinking and emit no visible answer at all (verified live against
/// `claude-sonnet-5`, see this module's captured-fixture tests). Returning
/// `Ok("")` here would let that truncation masquerade as a valid empty
/// completion — exactly what produced a 0-byte `PoC.t.sol` in production.
///
/// The message names the block-type census and the stop reason so the next
/// reader learns "the model spent its whole budget thinking" in one step
/// instead of three rounds of debugging. Remediation advice is deliberately
/// NOT repeated here — [`LlmError::EmptyCompletion`]'s own `Display` already
/// appends it, and duplicating it would read as noise.
fn detect_no_text_content(
    content: &[ClaudeContent],
    stop_reason: Option<&str>,
) -> Option<LlmError> {
    if !concat_text_blocks(content).trim().is_empty() {
        return None;
    }

    let block_types = content
        .iter()
        .map(|block| block.content_type.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    Some(LlmError::EmptyCompletion(format!(
        "{} content block(s) with no text (types: [{}]), stop_reason={}",
        content.len(),
        block_types,
        stop_reason.unwrap_or("none")
    )))
}

#[derive(Debug, Deserialize)]
struct ClaudeStreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    delta: Option<ClaudeDelta>,
    /// Present on `message_start`; carries the input/cache usage figures
    /// (D-15).
    #[serde(default)]
    message: Option<ClaudeStreamMessage>,
    /// Present on `message_delta`; carries the cumulative output/thinking
    /// usage figures (D-15). `message_start`'s usage lives one level down,
    /// under [`Self::message`], not here — the two events place their
    /// usage object at different nesting depths on Anthropic's own wire.
    #[serde(default)]
    usage: Option<ClaudeUsage>,
}

/// The `message` payload of a `message_start` event.
#[derive(Debug, Deserialize)]
struct ClaudeStreamMessage {
    #[serde(default)]
    usage: Option<ClaudeUsage>,
}

#[derive(Debug, Deserialize)]
struct ClaudeDelta {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    delta_type: Option<String>,
    text: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::redaction::{RESPONSE_EXCERPT_CHAR_BUDGET, bounded_excerpt};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn test_anthropic_config_validation() {
        let config = AnthropicConfig::new(
            "sk-ant-test123".to_string(),
            "https://api.anthropic.com/v1".to_string(),
            "claude-3-5-sonnet-20241022".to_string(),
            4096,
        );
        assert!(config.validate().is_ok());

        let config_empty_key = AnthropicConfig::new(
            "".to_string(),
            "https://api.anthropic.com/v1".to_string(),
            "claude-3-5-sonnet-20241022".to_string(),
            4096,
        );
        assert!(config_empty_key.validate().is_err());

        let config_invalid_url = AnthropicConfig::new(
            "sk-ant-test123".to_string(),
            "invalid-url".to_string(),
            "claude-3-5-sonnet-20241022".to_string(),
            4096,
        );
        assert!(config_invalid_url.validate().is_err());

        let config_zero_tokens = AnthropicConfig::new(
            "sk-ant-test123".to_string(),
            "https://api.anthropic.com/v1".to_string(),
            "claude-3-5-sonnet-20241022".to_string(),
            0,
        );
        assert!(config_zero_tokens.validate().is_err());
    }

    #[tokio::test]
    async fn test_anthropic_adapter_creation() {
        let config = AnthropicConfig::new(
            "sk-ant-test123".to_string(),
            "https://api.anthropic.com/v1".to_string(),
            "claude-3-5-sonnet-20241022".to_string(),
            4096,
        );
        assert!(AnthropicAdapter::new(config).is_ok());
    }

    #[tokio::test]
    async fn test_anthropic_provider_capabilities() {
        let config = AnthropicConfig::new(
            "sk-ant-test123".to_string(),
            "https://api.anthropic.com/v1".to_string(),
            "claude-3-5-sonnet-20241022".to_string(),
            4096,
        );
        let adapter = AnthropicAdapter::new(config).unwrap();
        let capabilities = adapter.get_capabilities();

        assert!(capabilities.supports_streaming);
        // `LlmRequest` has no field through which a tool definition could travel, and
        // this adapter neither sends `tools` nor parses `tool_calls` (WEB-03, D-14).
        assert!(!capabilities.supports_tool_calling);
        assert!(capabilities.supports_vision);
        assert!(capabilities.supports_system_messages);
        assert_eq!(capabilities.max_context_tokens, Some(200_000));
        assert_eq!(capabilities.temperature_range, Some((0.0, 1.0)));
        assert_eq!(adapter.get_provider_name(), "anthropic");
    }

    // ── Fixtures ─────────────────────────────────────────────────────────
    //
    // The two small bodies below are captured verbatim (byte-for-byte) from
    // real Anthropic API responses recorded against the operator's key on
    // 2026-07-26. See `.planning/quick/260726-hac-...-des/captured/` for
    // the source files. Per D-05, the third (opus-5 thinking+text) body
    // keeps every key and every block but abbreviates the 10,136-character
    // opaque `signature` value and shortens the long markdown `text` value
    // — abbreviating `signature` cannot reopen the fixture-vs-reality gap
    // this bug came from, because no declared field ever reads it.

    /// Captured verbatim: `claude-opus-4-8`, `content` = `[text]`,
    /// `stop_reason` = `end_turn`. Today's working path.
    const TEXT_ONLY_OPUS_4_8_JSON: &str = r#"{"model":"claude-opus-4-8","id":"msg_011CdQ3yemAxSE8oJ3n9XtDa","type":"message","role":"assistant","content":[{"type":"text","text":"OK"}],"stop_reason":"end_turn","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":16,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"cache_creation":{"ephemeral_5m_input_tokens":0,"ephemeral_1h_input_tokens":0},"output_tokens":4,"output_tokens_details":{"thinking_tokens":0},"service_tier":"standard","inference_geo":"global"}}"#;

    /// Captured verbatim: `claude-sonnet-5`, `content` = `[thinking]` and
    /// nothing else, `stop_reason` = `max_tokens`. The model spent its
    /// entire budget thinking and emitted no answer.
    const THINKING_ONLY_SONNET_5_JSON: &str = r#"{"model":"claude-sonnet-5","id":"msg_011CdPt7Pro2EKHRPvJrdHka","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"EvQBCokBCBAYAipA072s7SF3R2MEF7V6RjWZjqAyf1pvJ0UhG0E/yloUS36ysANfS3KsrPpPUV9s/SKSsiscE4UVvGTSvThmP2tWJzIPY2xhdWRlLXNvbm5ldC01OABCCHRoaW5raW5nWiQzNGU2YjdkMC02OGQxLTRiNjktOWJkMS0zMzYyNzdiOTBmY2MSDH6s/Y59weopm1Hw8xoMegJjpp+/E13Deon8IjDQtR8fSoDm3N1Xiw0btRU5hjQ/qkcts/89FAImYi3/ECk9PFmvJk+8txOjPRIU1ZIqGMM90P6i63HaR5a+FeRF7JFRIdZw5ukoEBgB"}],"stop_reason":"max_tokens","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":8,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"cache_creation":{"ephemeral_5m_input_tokens":0,"ephemeral_1h_input_tokens":0},"output_tokens":4,"output_tokens_details":{"thinking_tokens":3},"service_tier":"standard","inference_geo":"global"}}"#;

    /// The recovered text of [`THINKING_TEXT_OPUS_5_JSON`] below — kept as
    /// its own constant so the round-trip assertion reads as an exact
    /// equality against a named value rather than a second inline literal.
    const EXPECTED_OPUS_5_TEXT: &str = "Reasoning (abbreviated for fixture — see \
captured/anthropic-thinking-text-opus-5.json for the full 11873-byte body). Step 1: in \
steady state a donation is pro-rata neutral; every holder gains in proportion to their \
stake, so an attacker donating to himself alone is a strict loss.";

    /// Derived from the real `claude-opus-5` capture: every key and every
    /// block is verbatim, but the 10,136-character opaque `signature` value
    /// and the long markdown `text` value are abbreviated per D-05 (see the
    /// fixtures header comment above). `content` = `[thinking, text]`,
    /// `stop_reason` = `max_tokens` — this is the exact shape that broke
    /// deserialization before this fix.
    const THINKING_TEXT_OPUS_5_JSON: &str = r#"{"model":"claude-opus-5","id":"msg_011CdQ41GjfNCpc1qNdjCKbC","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"CAISqTsKhwEIEBgCKkABoVeK3KR0Vu9cHJkwnm+QmD0N7bLxg9vCCS10YT+h5M820zj1fxEx2WzWTayWtMdJqwnnOTY3eMdd36FcJwN0Mg1jbGF1ZGUtb3B1cy01OAFCCHRoaW5raW5n[ABBREVIATED-10136-chars-see-captured-fixture]EBgB"},{"type":"text","text":"Reasoning (abbreviated for fixture — see captured/anthropic-thinking-text-opus-5.json for the full 11873-byte body). Step 1: in steady state a donation is pro-rata neutral; every holder gains in proportion to their stake, so an attacker donating to himself alone is a strict loss."}],"stop_reason":"max_tokens","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":85,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"cache_creation":{"ephemeral_5m_input_tokens":0,"ephemeral_1h_input_tokens":0},"output_tokens":3000,"output_tokens_details":{"thinking_tokens":2561},"service_tier":"standard","inference_geo":"global"}}"#;

    /// A shape-extension, NOT a capture — no live body of this exact shape
    /// was recorded. Built to prove the durable lesson of this bug: a
    /// `tool_use` block and a `redacted_thinking` block are both skipped,
    /// never fatal, and two text blocks separated by non-text blocks still
    /// concatenate in array order.
    const MIXED_BLOCK_SHAPE_EXTENSION_JSON: &str = r#"{"model":"claude-opus-5","id":"msg_mixed_shape_extension","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"reasoning...","signature":"sig"},{"type":"text","text":"Part one. "},{"type":"tool_use","id":"toolu_01","name":"lookup","input":{"query":"foo"}},{"type":"redacted_thinking","data":"opaque"},{"type":"text","text":"Part two."}],"stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":10,"output_tokens":20}}"#;

    /// Phase 31 (D-20): a NEW captured-style fixture whose `usage` carries
    /// non-zero `cache_read_input_tokens`/`cache_creation_input_tokens` —
    /// none of the three pre-existing fixtures above exercise this (all
    /// three report explicit zeros), so none of them can catch a regression
    /// in the cache-inclusive `prompt_tokens` correction (RESEARCH.md
    /// Pitfall 3/5). Token counts and message scaffolding only — no
    /// credential-shaped literal.
    const CACHED_PROMPT_SONNET_5_JSON: &str = r#"{"model":"claude-sonnet-5","id":"msg_011CachedPromptFixture","type":"message","role":"assistant","content":[{"type":"text","text":"Answered from cache."}],"stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":85,"cache_creation_input_tokens":128,"cache_read_input_tokens":512,"output_tokens":40,"output_tokens_details":{"thinking_tokens":0}}}"#;

    fn test_adapter() -> AnthropicAdapter {
        let config = AnthropicConfig::new(
            "sk-ant-test123".to_string(),
            "https://api.anthropic.com/v1".to_string(),
            "claude-3-5-sonnet-20241022".to_string(),
            4096,
        );
        AnthropicAdapter::new(config).expect("test config must build a valid adapter")
    }

    // ── Task 1: block-type-tolerant deserialization + ordered recovery ────

    #[test]
    fn test_thinking_plus_text_response_deserializes_and_recovers_text_verbatim() {
        let response: ClaudeResponse = serde_json::from_str(THINKING_TEXT_OPUS_5_JSON)
            .expect("captured opus-5 thinking+text body must deserialize");

        let adapter = test_adapter();
        let llm_response = adapter
            .parse_response(Uuid::new_v4(), response)
            .expect("thinking+text response must recover its text, not error");

        assert_eq!(llm_response.content, EXPECTED_OPUS_5_TEXT);
    }

    #[test]
    fn test_thinking_plus_text_usage_tolerates_undeclared_keys() {
        // Verifies the tolerates-unknown-fields assumption against the real
        // body, rather than merely assuming serde's documented default
        // behaviour: six undeclared `usage` keys (cache_creation_input_tokens,
        // cache_read_input_tokens, nested cache_creation, nested
        // output_tokens_details, service_tier, inference_geo) plus four
        // undeclared top-level keys (type, role, stop_sequence, stop_details)
        // surround the declared fields in this fixture.
        let response: ClaudeResponse = serde_json::from_str(THINKING_TEXT_OPUS_5_JSON)
            .expect("captured opus-5 thinking+text body must deserialize");

        assert_eq!(response.usage.input_tokens, 85);
        assert_eq!(response.usage.output_tokens, 3000);
    }

    // ── Plan 31-04 (D-20): cache-inclusive `prompt_tokens` + reasoning ────

    #[test]
    fn test_thinking_text_maps_reasoning_and_reports_explicit_zero_cache_figures() {
        let response: ClaudeResponse = serde_json::from_str(THINKING_TEXT_OPUS_5_JSON)
            .expect("captured opus-5 thinking+text body must deserialize");

        let adapter = test_adapter();
        let llm_response = adapter
            .parse_response(Uuid::new_v4(), response)
            .expect("thinking+text response must parse");

        assert_eq!(llm_response.usage.completion_tokens, 3000);
        assert_eq!(llm_response.usage.reasoning_tokens, Some(2561));
        // This fixture's cache figures are explicit zeros, so the D-20
        // correction is a no-op for it: prompt_tokens == input_tokens.
        assert_eq!(llm_response.usage.prompt_tokens, 85);
        assert_eq!(llm_response.usage.cache_read_tokens, Some(0));
        assert_eq!(llm_response.usage.cache_write_tokens, Some(0));
        assert!(
            llm_response.usage.reasoning_tokens.unwrap() <= llm_response.usage.completion_tokens
        );
    }

    #[test]
    fn test_cache_inclusive_prompt_tokens_correction_fires_on_non_zero_cache_fixture() {
        let response: ClaudeResponse = serde_json::from_str(CACHED_PROMPT_SONNET_5_JSON)
            .expect("new cached-prompt fixture must deserialize");

        let adapter = test_adapter();
        let llm_response = adapter
            .parse_response(Uuid::new_v4(), response)
            .expect("cached-prompt response must parse");

        // input_tokens(85) + cache_read(512) + cache_write(128) == 725.
        assert_eq!(llm_response.usage.prompt_tokens, 725);
        assert_eq!(llm_response.usage.cache_read_tokens, Some(512));
        assert_eq!(llm_response.usage.cache_write_tokens, Some(128));
        assert_eq!(
            llm_response.usage.total_tokens,
            llm_response.usage.prompt_tokens + llm_response.usage.completion_tokens
        );
        assert!(
            llm_response.usage.cache_read_tokens.unwrap()
                + llm_response.usage.cache_write_tokens.unwrap()
                <= llm_response.usage.prompt_tokens
        );
    }

    #[test]
    fn test_text_only_response_is_unchanged_from_todays_behaviour() {
        let response: ClaudeResponse = serde_json::from_str(TEXT_ONLY_OPUS_4_8_JSON)
            .expect("captured opus-4-8 text-only body must deserialize");

        let adapter = test_adapter();
        let llm_response = adapter
            .parse_response(Uuid::new_v4(), response)
            .expect("text-only response must succeed exactly as it does today");

        assert_eq!(llm_response.content, "OK");
        assert!(matches!(llm_response.finish_reason, FinishReason::Stop));
    }

    #[test]
    fn test_thinking_only_response_yields_empty_completion_not_empty_success() {
        let response: ClaudeResponse = serde_json::from_str(THINKING_ONLY_SONNET_5_JSON)
            .expect("captured sonnet-5 thinking-only body must deserialize");

        let adapter = test_adapter();
        let result = adapter.parse_response(Uuid::new_v4(), response);

        match result {
            Err(LlmError::EmptyCompletion(msg)) => {
                assert!(
                    msg.contains("thinking"),
                    "message must name the block type it saw: {msg}"
                );
                assert!(
                    msg.contains("max_tokens"),
                    "message must name the stop reason it arrived with: {msg}"
                );
            }
            other => panic!("expected Err(LlmError::EmptyCompletion(_)), got {other:?}"),
        }
    }

    #[test]
    fn test_mixed_block_shape_recovers_both_text_blocks_in_order_never_fatal() {
        let response: ClaudeResponse = serde_json::from_str(MIXED_BLOCK_SHAPE_EXTENSION_JSON)
            .expect("mixed-block shape-extension body must deserialize without error");

        let recovered = concat_text_blocks(&response.content);
        assert_eq!(recovered, "Part one. Part two.");
    }

    // ── Task 2: diagnosable deserialization failures + bounded excerpts ───

    #[test]
    fn test_bounded_excerpt_returns_input_unchanged_when_shorter_than_budget() {
        let body = "a short body well under the budget";
        assert_eq!(bounded_excerpt(body, RESPONSE_EXCERPT_CHAR_BUDGET), body);
    }

    #[test]
    fn test_bounded_excerpt_is_char_boundary_safe_on_multibyte_input() {
        // Each 'あ' is 3 bytes in UTF-8; a naive byte-index slice at the
        // budget would land mid-character and panic. Build an input well
        // past the budget entirely out of multi-byte characters.
        let body: String = std::iter::repeat_n('あ', 600).collect();
        let budget = 100;

        let excerpt = bounded_excerpt(&body, budget);

        assert!(
            excerpt.contains("truncated"),
            "excerpt must carry an elision marker: {excerpt}"
        );
        let expected_prefix: String = body.chars().take(budget).collect();
        assert!(
            excerpt.starts_with(&expected_prefix),
            "excerpt must not split a character at the boundary"
        );
        assert_eq!(expected_prefix.chars().count(), budget);
    }

    #[tokio::test(start_paused = true)]
    async fn test_execute_with_retry_invokes_operation_exactly_once_on_empty_completion() {
        let adapter = test_adapter();
        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = Arc::clone(&calls);

        let result: Result<(), LlmError> = adapter
            .execute_with_retry(
                move || {
                    let calls = Arc::clone(&calls_clone);
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Err(LlmError::EmptyCompletion("no text".to_string()))
                    }
                },
                3,
            )
            .await;

        assert!(result.is_err());
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "a no-text response must not be retried — the retried request is identical"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn test_execute_with_retry_still_retries_a_retryable_processing_error() {
        let adapter = test_adapter();
        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = Arc::clone(&calls);

        let result: Result<(), LlmError> = adapter
            .execute_with_retry(
                move || {
                    let calls = Arc::clone(&calls_clone);
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Err(LlmError::ProcessingError("retry me".to_string()))
                    }
                },
                3,
            )
            .await;

        assert!(result.is_err());
        assert_eq!(
            calls.load(Ordering::SeqCst),
            3,
            "a genuinely retryable error must still be retried up to max_retries"
        );
    }

    // ── Phase 25 (FT-FR-01, D-03): non-2xx routes through map_http_status ──

    #[test]
    fn anthropic_non_2xx_routes_through_the_shared_mapper() {
        let adapter = test_adapter();
        match adapter.map_error(503, r#"{"error":{"type":"overloaded_error"}}"#) {
            LlmError::ProviderError {
                provider, status, ..
            } => {
                assert_eq!(provider, "anthropic");
                assert_eq!(status, 503);
            }
            other => panic!("expected ProviderError {{ status: 503 }}, got {other:?}"),
        }
    }

    #[test]
    fn anthropic_dedicated_status_mappings_are_unchanged() {
        let adapter = test_adapter();
        assert!(matches!(
            adapter.map_error(401, "bad key"),
            LlmError::AuthenticationError(_)
        ));
        assert!(matches!(
            adapter.map_error(403, "no permission"),
            LlmError::AuthenticationError(_)
        ));
        assert!(matches!(
            adapter.map_error(429, "slow down"),
            LlmError::RateLimitExceeded
        ));
        assert!(matches!(
            adapter.map_error(400, "bad prompt"),
            LlmError::InvalidPrompt(_)
        ));
    }

    // ── Task 41-01/2: usage-cap body classification (D-04/D-05) ───────────

    #[test]
    fn map_error_400_with_usage_cap_body_maps_to_usage_limit_exceeded() {
        let adapter = test_adapter();
        let body = r#"{"type":"error","error":{"type":"invalid_request_error","message":"You have reached your specified API usage limits. You will regain access on 2026-08-01 at 00:00 UTC."}}"#;

        let error = adapter.map_error(400, body);

        match error {
            LlmError::UsageLimitExceeded {
                provider,
                regain_hint,
            } => {
                assert_eq!(provider, "anthropic");
                let hint = regain_hint.expect("regain hint must be extracted from the body");
                assert!(
                    hint.contains("2026-08-01"),
                    "regain hint must contain the date: {hint}"
                );
            }
            other => panic!("expected UsageLimitExceeded, got {other:?}"),
        }
    }

    #[test]
    fn map_error_maps_a_redirect_status_to_an_actionable_provider_error() {
        // CR-02 (`25-REVIEW.md`): named explicitly because this client's
        // redirect policy is `none` (see `AnthropicAdapter::new`), so a
        // `3xx` response is never followed by the underlying HTTP client —
        // it arrives here as an ordinary non-success status instead.
        let adapter = test_adapter();

        for expected in [301u16, 302, 307] {
            match adapter.map_error(expected, "moved") {
                LlmError::ProviderError {
                    provider,
                    status,
                    message,
                } => {
                    assert_eq!(provider, "anthropic");
                    assert_eq!(status, expected, "typed status field must carry the code");
                    assert!(
                        message.contains("redirect"),
                        "status {expected}: message must name the refused redirect, got: {message}"
                    );
                }
                other => panic!("status {expected}: expected ProviderError, got {other:?}"),
            }
        }
    }

    #[test]
    fn map_error_400_ordinary_bad_request_still_maps_to_invalid_prompt() {
        // T-41-01 regression: an ordinary malformed-prompt 400 must never
        // be reclassified as "wait for quota reset".
        let adapter = test_adapter();
        let error = adapter.map_error(400, "Bad request: missing required field 'messages'");

        assert!(
            matches!(error, LlmError::InvalidPrompt(_)),
            "expected InvalidPrompt, got {error:?}"
        );
    }

    #[test]
    fn map_error_400_max_tokens_branch_is_not_shadowed_by_the_usage_cap_check() {
        let adapter = test_adapter();
        let error = adapter.map_error(400, "Invalid max_tokens value");

        assert!(
            matches!(error, LlmError::InvalidPrompt(_)),
            "expected InvalidPrompt (max_tokens branch), got {error:?}"
        );
    }

    #[test]
    fn map_error_400_usage_cap_with_no_parseable_regain_prose_yields_none_hint() {
        let adapter = test_adapter();
        let error = adapter.map_error(400, "You have reached your specified API usage limits.");

        match error {
            LlmError::UsageLimitExceeded { regain_hint, .. } => {
                assert!(
                    regain_hint.is_none(),
                    "a missing hint must never be an error: {regain_hint:?}"
                );
            }
            other => panic!("expected UsageLimitExceeded, got {other:?}"),
        }
    }

    #[test]
    fn map_error_400_usage_cap_redacts_the_configured_api_key_before_extracting_the_regain_hint() {
        // CR-01 regression: a gateway/proxy in front of `base_url` that
        // echoes request context near "regain access" must never have that
        // context (including a live credential) forwarded into the
        // operator-facing `regain_hint` unredacted.
        let adapter = test_adapter();
        let secret = "sk-ant-test123"; // matches `test_adapter()`'s configured api_key
        let body = format!(
            r#"{{"type":"error","error":{{"type":"invalid_request_error","message":"You have reached your specified API usage limits. You will regain access after re-authenticating with key {secret} on 2026-08-01."}}}}"#
        );

        let error = adapter.map_error(400, &body);

        match error {
            LlmError::UsageLimitExceeded { regain_hint, .. } => {
                let hint = regain_hint.expect("regain hint must still be extracted");
                assert!(
                    !hint.contains(secret),
                    "regain hint leaked the configured API key: {hint}"
                );
                assert!(
                    hint.contains("2026-08-01"),
                    "redaction must not destroy surrounding diagnostic prose: {hint}"
                );
            }
            other => panic!("expected UsageLimitExceeded, got {other:?}"),
        }
    }

    #[test]
    fn extract_regain_hint_returns_none_when_the_body_has_no_regain_prose() {
        assert_eq!(
            extract_regain_hint("You have reached your specified API usage limits."),
            None
        );
    }

    #[test]
    fn extract_regain_hint_is_char_bounded_and_never_panics_on_multibyte_input() {
        // Each 'あ' is 3 bytes in UTF-8; a naive byte-index slice built from
        // a byte offset computed against a different string (e.g. the
        // lowercased copy) could land mid-character and panic. Build an
        // adversarially large body of multi-byte characters after the
        // marker to exercise both the char-boundary safety and the
        // REGAIN_HINT_CHAR_BUDGET bound.
        let tail: String = std::iter::repeat_n('あ', 600).collect();
        let body = format!("You will regain access {tail}\"}}");

        let hint = extract_regain_hint(&body).expect("regain prose must be extracted");

        assert!(
            hint.chars().count() <= REGAIN_HINT_CHAR_BUDGET,
            "hint must be bounded to {REGAIN_HINT_CHAR_BUDGET} characters, got {}",
            hint.chars().count()
        );
    }

    #[tokio::test(start_paused = true)]
    async fn execute_with_retry_invokes_operation_exactly_once_on_usage_limit_exceeded() {
        let adapter = test_adapter();
        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = Arc::clone(&calls);

        let result: Result<(), LlmError> = adapter
            .execute_with_retry(
                move || {
                    let calls = Arc::clone(&calls_clone);
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Err(LlmError::UsageLimitExceeded {
                            provider: "anthropic".to_string(),
                            regain_hint: None,
                        })
                    }
                },
                3,
            )
            .await;

        assert!(result.is_err());
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "a usage-cap error must not be retried — it will not clear on backoff"
        );
    }

    // ── Phase 26 (RT-05, D-28): the documented no-native-mode path ─────────

    /// Anthropic has no native structured-output mode (D-28): `ClaudeRequest`
    /// has no `response_format` field at all, so a caller-supplied
    /// [`paladin_ports::output::llm_port::ResponseFormat`] is ignored
    /// harmlessly by construction — there is no code path that could even
    /// read it. This test pins that as an executable fact rather than an
    /// assumption: building and sending a `response_format`-carrying
    /// request produces a wire body with no JSON-mode field and the call
    /// still succeeds, exactly like a request with the field unset.
    #[tokio::test]
    async fn anthropic_ignores_response_format() {
        use mockito::Server;
        use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
        use paladin_ports::output::llm_port::ResponseFormat;
        use std::sync::Mutex as StdMutex;

        let mut server = Server::new_async().await;
        let captured: Arc<StdMutex<Option<String>>> = Arc::new(StdMutex::new(None));
        let captured_clone = Arc::clone(&captured);

        server
            .mock("POST", "/messages")
            .with_status(200)
            .with_body_from_request(move |req| {
                let body_text = req.utf8_lossy_body().unwrap_or_default().into_owned();
                *captured_clone.lock().unwrap() = Some(body_text);
                TEXT_ONLY_OPUS_4_8_JSON.as_bytes().to_vec()
            })
            .create_async()
            .await;

        let config = AnthropicConfig::new(
            "sk-ant-test123".to_string(),
            server.url(),
            "claude-opus-4-8".to_string(),
            4096,
        );
        let adapter =
            AnthropicAdapter::new(config).expect("test config must build a valid adapter");

        let request = LlmRequest::new(
            "claude-opus-4-8",
            PromptItem::new(PromptType::User(UserPrompt {
                query: "Hello".to_string(),
                context: None,
            }))
            .unwrap(),
        )
        .with_response_format(ResponseFormat::JsonObject);

        let result = adapter.generate(request).await;
        assert!(
            result.is_ok(),
            "a response_format-carrying request must still succeed: {result:?}"
        );

        let body_text = captured
            .lock()
            .unwrap()
            .take()
            .expect("mock must have been called exactly once");
        let body: serde_json::Value =
            serde_json::from_str(&body_text).expect("captured body must be valid JSON");
        assert!(
            body.as_object().unwrap().get("response_format").is_none(),
            "Anthropic has no native JSON mode -- response_format must not \
             appear on the wire, got: {body_text}"
        );
    }

    // ── Plan 31-04 (D-14/D-15/D-19): event-accumulation streaming usage ───

    mod streaming_usage_wiring {
        use super::*;
        use futures::StreamExt;
        use mockito::Server;
        use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};

        fn adapter_at(base_url: &str) -> AnthropicAdapter {
            let config = AnthropicConfig::new(
                "sk-ant-test123".to_string(),
                base_url.to_string(),
                "claude-sonnet-5".to_string(),
                4096,
            );
            AnthropicAdapter::new(config).expect("test config must build a valid adapter")
        }

        fn build_request() -> LlmRequest {
            LlmRequest::new(
                "claude-sonnet-5",
                PromptItem::new(PromptType::User(UserPrompt {
                    query: "Hello".to_string(),
                    context: None,
                }))
                .expect("a user prompt must build"),
            )
        }

        /// A `message_start`/`content_block_delta`(s)/`message_delta`/
        /// `message_stop` event sequence whose combined usage figures match
        /// [`super::CACHED_PROMPT_SONNET_5_JSON`]'s non-streaming usage
        /// EXACTLY (`ConformanceFixture::stream_body()`'s D-19 contract:
        /// stream and non-stream must carry the same usage figures) —
        /// `input_tokens: 85`, `cache_read_input_tokens: 512`,
        /// `cache_creation_input_tokens: 128`, `output_tokens: 40`,
        /// `thinking_tokens: 0`.
        fn sse_body_with_usage() -> String {
            "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_1\",\"type\":\"message\",\
             \"role\":\"assistant\",\"model\":\"claude-sonnet-5\",\"content\":[],\"stop_reason\":null,\
             \"stop_sequence\":null,\"usage\":{\"input_tokens\":85,\"cache_creation_input_tokens\":128,\
             \"cache_read_input_tokens\":512,\"output_tokens\":1}}}\n\n\
             data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hel\"}}\n\n\
             data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"lo\"}}\n\n\
             data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null},\
             \"usage\":{\"output_tokens\":40,\"output_tokens_details\":{\"thinking_tokens\":0}}}\n\n\
             data: {\"type\":\"message_stop\"}\n\n"
                .to_string()
        }

        /// Stands in for `crate::conformance::cases::streaming_usage_equals_non_streaming_usage`
        /// (D-19, plan 31-04): Anthropic's event-accumulation wire shape (usage split across
        /// `message_start`/`message_delta`, no `[DONE]` sentinel) diverges too far from
        /// `ConformanceFixture`'s single-`stream_body()`-string assumption to instantiate the
        /// shared macro, so this test asserts the identical three properties by hand: exactly
        /// one chunk carries a finish reason, that SAME chunk is the only one carrying usage,
        /// and its usage equals the non-streaming path's `LlmResponse.usage` field-for-field.
        #[tokio::test]
        async fn message_stop_is_the_only_usage_bearing_chunk_and_equals_the_non_streaming_usage() {
            let mut non_stream_server = Server::new_async().await;
            non_stream_server
                .mock("POST", "/messages")
                .with_status(200)
                .with_body(CACHED_PROMPT_SONNET_5_JSON)
                .create_async()
                .await;
            let non_streaming = adapter_at(&non_stream_server.url())
                .generate(build_request())
                .await
                .expect("non-streaming call must succeed");

            let mut stream_server = Server::new_async().await;
            stream_server
                .mock("POST", "/messages")
                .with_status(200)
                .with_header("content-type", "text/event-stream")
                .with_body(sse_body_with_usage())
                .create_async()
                .await;

            let stream = adapter_at(&stream_server.url())
                .generate_stream(build_request())
                .await
                .expect("streaming call must succeed");
            let mut stream = Box::into_pin(stream);

            let mut chunks = Vec::new();
            while let Some(item) = stream.next().await {
                chunks.push(item.expect("every chunk must parse"));
            }

            let finish_indices: Vec<usize> = chunks
                .iter()
                .enumerate()
                .filter(|(_, c)| c.finish_reason.is_some())
                .map(|(i, _)| i)
                .collect();
            let usage_indices: Vec<usize> = chunks
                .iter()
                .enumerate()
                .filter(|(_, c)| c.usage.is_some())
                .map(|(i, _)| i)
                .collect();

            assert_eq!(
                finish_indices.len(),
                1,
                "exactly one chunk must carry a finish reason"
            );
            assert_eq!(usage_indices.len(), 1, "exactly one chunk must carry usage");
            assert_eq!(
                finish_indices, usage_indices,
                "the finish-reason chunk and the usage chunk must be the SAME chunk (message_stop)"
            );
            assert_eq!(
                chunks[usage_indices[0]].usage,
                Some(non_streaming.usage),
                "streamed usage must equal the non-streaming usage field-for-field"
            );

            let assembled: String = chunks.iter().map(|c| c.delta.as_str()).collect();
            assert_eq!(assembled, "Hello");
        }

        #[tokio::test]
        async fn message_stop_carries_usage_none_when_no_usage_payload_ever_arrives() {
            let mut server = Server::new_async().await;
            let sse_body = "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hi\"}}\n\n\
                 data: {\"type\":\"message_stop\"}\n\n";
            server
                .mock("POST", "/messages")
                .with_status(200)
                .with_header("content-type", "text/event-stream")
                .with_body(sse_body)
                .create_async()
                .await;

            let stream = adapter_at(&server.url())
                .generate_stream(build_request())
                .await
                .expect("streaming call must succeed");
            let mut stream = Box::into_pin(stream);

            let mut chunks = Vec::new();
            while let Some(item) = stream.next().await {
                chunks.push(item.expect("every chunk must parse"));
            }

            let terminal = chunks
                .iter()
                .find(|c| c.finish_reason.is_some())
                .expect("stream must still end with a message_stop terminal chunk");
            assert_eq!(terminal.usage, None);
        }
    }
}
