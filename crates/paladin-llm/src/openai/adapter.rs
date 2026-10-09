//! OpenAI GPT adapter.
//!
//! Implements [`LlmPort`] for the OpenAI Chat Completions API.
//! Supports GPT-3.5-Turbo, GPT-4, GPT-4o, and other compatible models.

use async_trait::async_trait;
use chrono::Utc;
use futures::{Stream, StreamExt};
use paladin_core::platform::container::content::{ContentItem, ContentType};
use paladin_core::platform::container::prompt::{PromptItem, PromptRole, PromptType};
use paladin_ports::output::llm_port::{
    FinishReason, LlmError, LlmPort, LlmRequest, LlmResponse, ProviderCapabilities, ResponseFormat,
    StreamingResponse, TokenUsage,
};
use rand::Rng;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::pin::Pin;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

use crate::http_status::map_http_status_with_hints;
use crate::rate_limit_headers::{RateLimitHeaderFamily, hints_from_headers};
use paladin_ports::output::rate_limit_hints::RateLimitHints;

/// The provider name this adapter reports through [`LlmPort::get_provider_name`]
/// and stamps on every [`LlmError::ProviderError`] it emits.
const OPENAI_PROVIDER: &str = "openai";

/// The OpenAI error identifier of an account-level quota wall, carried as `error.code` (and, on
/// some responses, `error.type`).
///
/// Verified against <https://platform.openai.com/docs/guides/error-codes> on 2026-10-09 (plan 43-12; evidence:
/// .planning/phases/43-rate-pacing/43-PROVIDER-HEADER-EVIDENCE.md): the operator confirmed that a
/// quota/billing 429 carries the code `insufficient_quota`. Were the string ever to change, the
/// mapping would simply stop firing and the 429 would be paced and bounded by `max_backoff_ms`.
const INSUFFICIENT_QUOTA: &str = "insufficient_quota";

/// Whether a `429` body identifies an exhausted quota rather than a transient rate limit.
///
/// Compares only the short `error.code` / `error.type` identifiers of the parsed JSON; it never
/// renders or stores any body text, and a body that is not JSON (or has no such field) is simply
/// "not a quota error".
fn signals_insufficient_quota(body: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return false;
    };
    let Some(error) = value.get("error") else {
        return false;
    };
    ["code", "type"]
        .iter()
        .any(|field| error.get(field).and_then(|v| v.as_str()) == Some(INSUFFICIENT_QUOTA))
}

/// Parse a `429` response's rate-limit headers. Must run BEFORE `response.text()` consumes the
/// response (research Pitfall 2). Only a 429 is parsed: no other status carries hints, and no
/// credential header is read.
fn snapshot_rate_limit_hints(
    status: reqwest::StatusCode,
    headers: &reqwest::header::HeaderMap,
) -> Option<RateLimitHints> {
    if status != reqwest::StatusCode::TOO_MANY_REQUESTS {
        return None;
    }
    hints_from_headers(RateLimitHeaderFamily::OpenAi, SystemTime::now(), |name| {
        headers.get(name).and_then(|v| v.to_str().ok())
    })
}

/// Configuration for the OpenAI adapter.
#[derive(Debug, Clone)]
pub struct OpenAIConfig {
    /// OpenAI API key.
    pub api_key: String,
    /// Base URL for the API (default: `https://api.openai.com/v1`).
    pub base_url: String,
    /// Optional organisation ID.
    pub organization: Option<String>,
    /// Request timeout in seconds (default: 300).
    pub timeout_seconds: u64,
    /// Maximum retry attempts (default: 3).
    pub max_retries: u32,
}

impl OpenAIConfig {
    /// Load configuration from environment variables.
    ///
    /// Required:
    /// - `OPENAI_API_KEY`
    ///
    /// Optional:
    /// - `OPENAI_BASE_URL` (default: `https://api.openai.com/v1`)
    /// - `OPENAI_ORGANIZATION`
    /// - `OPENAI_TIMEOUT_SECONDS` (default: 300)
    /// - `OPENAI_MAX_RETRIES` (default: 3)
    pub fn from_env() -> Result<Self, String> {
        let api_key = env::var("OPENAI_API_KEY")
            .map_err(|_| "OPENAI_API_KEY environment variable not set")?;

        let base_url =
            env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1".to_string());

        let organization = env::var("OPENAI_ORGANIZATION").ok();

        let timeout_seconds = env::var("OPENAI_TIMEOUT_SECONDS")
            .unwrap_or_else(|_| "300".to_string())
            .parse()
            .map_err(|_| "Invalid OPENAI_TIMEOUT_SECONDS value")?;

        let max_retries = env::var("OPENAI_MAX_RETRIES")
            .unwrap_or_else(|_| "3".to_string())
            .parse()
            .map_err(|_| "Invalid OPENAI_MAX_RETRIES value")?;

        Ok(Self {
            api_key,
            base_url,
            organization,
            timeout_seconds,
            max_retries,
        })
    }

    /// Create a configuration with the given API key and sensible defaults.
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.openai.com/v1".to_string(),
            organization: None,
            timeout_seconds: 300,
            max_retries: 3,
        }
    }

    /// Validate the configuration fields.
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
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Internal API structures
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    stream: bool,
    /// `LlmRequest.response_format` on the wire (RT-FR-17, D-28). Omitted
    /// entirely when the caller sets no hint, keeping the body byte
    /// -identical to a pre-0.10 request (X-03).
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<OpenAIResponseFormat>,
    /// Requests the trailing `usage` frame on a streaming call (D-13/D-15).
    /// `Some({"include_usage": true})` on every streaming request; omitted
    /// entirely on a non-streaming one, keeping that body byte-identical to
    /// a pre-0.10 request (X-03).
    #[serde(skip_serializing_if = "Option::is_none")]
    stream_options: Option<OpenAIStreamOptions>,
}

/// `OpenAIRequest.stream_options`'s only shape this adapter sends (D-15).
#[derive(Debug, Serialize)]
struct OpenAIStreamOptions {
    include_usage: bool,
}

/// OpenAI's own two native JSON-mode wire shapes (RT-FR-17, D-28).
///
/// `{"type":"json_object"}` for [`ResponseFormat::JsonObject`], or
/// `{"type":"json_schema","json_schema":{"name":..,"schema":..,"strict":..}}`
/// for [`ResponseFormat::JsonSchema`] — OpenAI's documented `json_schema`
/// response-format shape.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum OpenAIResponseFormat {
    JsonObject,
    JsonSchema { json_schema: OpenAIJsonSchemaSpec },
}

#[derive(Debug, Serialize)]
struct OpenAIJsonSchemaSpec {
    name: String,
    schema: serde_json::Value,
    strict: bool,
}

/// Convert the provider-agnostic [`ResponseFormat`] hint into OpenAI's wire
/// shape.
///
/// `ResponseFormat` is `#[non_exhaustive]` (D-28), so a future variant this
/// match has not been taught degrades to the plain JSON-object form rather
/// than silently dropping the field — EDGE(RT-05/wire shape) requires
/// `response_format` is never omitted once the caller asked for JSON.
fn to_openai_response_format(format: &ResponseFormat) -> OpenAIResponseFormat {
    match format {
        ResponseFormat::JsonObject => OpenAIResponseFormat::JsonObject,
        ResponseFormat::JsonSchema {
            name,
            schema,
            strict,
        } => OpenAIResponseFormat::JsonSchema {
            json_schema: OpenAIJsonSchemaSpec {
                name: name.clone(),
                schema: schema.clone(),
                strict: *strict,
            },
        },
        _ => OpenAIResponseFormat::JsonObject,
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    #[allow(dead_code)]
    id: String,
    model: String,
    choices: Vec<OpenAIChoice>,
    usage: OpenAIUsage,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    #[allow(dead_code)]
    index: u32,
    message: OpenAIMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    // Deliberately unread: `TokenUsage::new` recomputes `total_tokens` as
    // `prompt_tokens + completion_tokens` (D-02), so the provider's own
    // reported total is discarded rather than trusted. Kept on the struct so
    // the deserializer still matches the full wire shape for debugging.
    #[allow(dead_code)]
    total_tokens: u32,
    /// `prompt_tokens_details.cached_tokens` (D-20). Absent on a response
    /// that reports no cache split -- `None` in that case (D-03), never a
    /// fabricated `Some(0)`.
    #[serde(default)]
    prompt_tokens_details: Option<OpenAIPromptTokensDetails>,
    /// `completion_tokens_details.reasoning_tokens` (D-20).
    #[serde(default)]
    completion_tokens_details: Option<OpenAICompletionTokensDetails>,
}

/// `OpenAIUsage.prompt_tokens_details` (D-20).
#[derive(Debug, Deserialize)]
struct OpenAIPromptTokensDetails {
    #[serde(default)]
    cached_tokens: Option<u32>,
}

/// `OpenAIUsage.completion_tokens_details` (D-20).
#[derive(Debug, Deserialize)]
struct OpenAICompletionTokensDetails {
    #[serde(default)]
    reasoning_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamChunk {
    /// `#[serde(default)]` because the trailing empty-`choices` usage frame
    /// (D-14/D-15) is not guaranteed to repeat the stream's `id`.
    #[allow(dead_code)]
    #[serde(default)]
    id: String,
    choices: Vec<OpenAIStreamChoice>,
    /// Present only on the trailing empty-`choices` frame a
    /// `stream_options: {"include_usage": true}` request elicits (D-14/D-15).
    #[serde(default)]
    usage: Option<OpenAIUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamChoice {
    #[allow(dead_code)]
    index: u32,
    delta: OpenAIStreamDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamDelta {
    #[allow(dead_code)]
    role: Option<String>,
    content: Option<String>,
}

// ---------------------------------------------------------------------------
// Adapter
// ---------------------------------------------------------------------------

/// OpenAI LLM adapter implementing [`LlmPort`].
pub struct OpenAIAdapter {
    pub(crate) config: OpenAIConfig,
    pub(crate) client: Client,
}

impl OpenAIAdapter {
    /// Create a new adapter from explicit configuration.
    pub fn new(config: OpenAIConfig) -> Result<Self, String> {
        config.validate()?;
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            // CR-02 (`25-REVIEW.md`): `OPENAI_BASE_URL` is
            // operator-configurable and every request carries the
            // `Authorization: Bearer` credential header. Refusing redirects
            // means a `3xx` from whatever host it resolves to can never
            // replay that header to a different, attacker-influenced host —
            // matches every `CompatEngine`-based preset and the bespoke
            // Gemini adapter (T-17-18/T-17-52). A refused redirect surfaces
            // via `Self::map_error`'s `300..=399` arm.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
        Ok(Self { config, client })
    }

    /// Map a non-2xx OpenAI response to [`LlmError`] with no rate-limit headers.
    ///
    /// A thin wrapper over [`Self::map_error_with_hints`], kept so the status-mapping unit
    /// tests read the same as before PACE-01. Production paths call the hints variant.
    #[cfg(test)]
    fn map_error(&self, status: u16, body: &str) -> LlmError {
        self.map_error_with_hints(status, body, None)
    }

    /// Map a non-2xx OpenAI response to [`LlmError`], carrying the parsed rate-limit headers
    /// of a `429` (PACE-01).
    ///
    /// `300..=399` is named explicitly (mirroring
    /// `CompatEngine::map_error`/`GeminiAdapter::map_error`) because this
    /// client's redirect policy is `none` (see [`Self::new`]), so a `3xx`
    /// response is never followed — it arrives here as an ordinary
    /// non-success status instead.
    ///
    /// A `429` whose body names `insufficient_quota` is an account-level quota wall, not a
    /// rate limit (research Pitfall 6): it maps to the permanent
    /// [`LlmError::UsageLimitExceeded`] so it is neither retried nor paced. Everything else is
    /// the crate-wide [`map_http_status_with_hints`] (Phase 25 D-03, FT-FR-01).
    fn map_error_with_hints(
        &self,
        status: u16,
        body: &str,
        hints: Option<RateLimitHints>,
    ) -> LlmError {
        match status {
            300..=399 => LlmError::ProviderError {
                provider: OPENAI_PROVIDER.to_string(),
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
            429 if signals_insufficient_quota(body) => LlmError::UsageLimitExceeded {
                provider: OPENAI_PROVIDER.to_string(),
                regain_hint: None,
            },
            _ => map_http_status_with_hints(
                OPENAI_PROVIDER,
                status,
                body,
                &self.config.api_key,
                hints,
            ),
        }
    }

    /// Create an adapter by loading configuration from environment variables.
    pub fn from_env() -> Result<Self, String> {
        Self::new(OpenAIConfig::from_env()?)
    }

    /// Convert a [`PromptItem`] and optional attachments into OpenAI messages.
    fn convert_to_messages(
        &self,
        prompt: &PromptItem,
        attachments: &[ContentItem],
    ) -> Result<Vec<OpenAIMessage>, LlmError> {
        let mut messages = Vec::new();

        match prompt.prompt_type() {
            PromptType::System(system_prompt) => {
                let mut content = system_prompt.instructions.clone();
                if let Some(constraints) = &system_prompt.constraints
                    && !constraints.is_empty()
                {
                    content.push_str("\n\nConstraints:\n");
                    for constraint in constraints {
                        content.push_str(&format!("- {}\n", constraint));
                    }
                }
                messages.push(OpenAIMessage {
                    role: "system".to_string(),
                    content,
                });
            }
            PromptType::User(user_prompt) => {
                messages.push(OpenAIMessage {
                    role: "user".to_string(),
                    content: user_prompt.context.clone().unwrap_or_default(),
                });
            }
            PromptType::Assistant(assistant_prompt) => {
                let mut content = assistant_prompt.response.clone();
                if let Some(reasoning) = &assistant_prompt.reasoning {
                    content.push_str(&format!("\n\nReasoning: {}", reasoning));
                }
                messages.push(OpenAIMessage {
                    role: "assistant".to_string(),
                    content,
                });
            }
            PromptType::Text(text_prompt) => {
                let role = match text_prompt.role {
                    PromptRole::System => "system",
                    PromptRole::User => "user",
                    PromptRole::Assistant => "assistant",
                    PromptRole::Function => "function",
                };
                messages.push(OpenAIMessage {
                    role: role.to_string(),
                    content: text_prompt.content.clone(),
                });
            }
            PromptType::Function(function_prompt) => {
                messages.push(OpenAIMessage {
                    role: "function".to_string(),
                    content: function_prompt.function_name.clone(),
                });
            }
        }

        for content in attachments {
            if let Ok(content_text) = self.convert_content_to_text(content)
                && !content_text.is_empty()
            {
                messages.push(OpenAIMessage {
                    role: "user".to_string(),
                    content: format!("Content to analyze:\n{}", content_text),
                });
            }
        }

        Ok(messages)
    }

    fn convert_content_to_text(&self, content: &ContentItem) -> Result<String, LlmError> {
        match content.content() {
            ContentType::Text(text_content) => {
                Ok(text_content.content.as_deref().unwrap_or("").to_string())
            }
            ContentType::Video(video_content) => Ok(format!(
                "Video: {} (Duration: {}s)",
                content.title().unwrap_or(&"Untitled".to_string()),
                video_content.duration
            )),
            ContentType::Audio(audio_content) => Ok(format!(
                "Audio: {} (Duration: {}s)",
                content.title().unwrap_or(&"Untitled".to_string()),
                audio_content.duration
            )),
            ContentType::Image(image_content) => Ok(format!(
                "Image: {} ({}x{})",
                content.title().unwrap_or(&"Untitled".to_string()),
                image_content.resolution.0,
                image_content.resolution.1
            )),
        }
    }

    fn convert_finish_reason(&self, reason: Option<String>) -> FinishReason {
        match reason.as_deref() {
            Some("stop") => FinishReason::Stop,
            Some("length") => FinishReason::Length,
            Some("content_filter") => FinishReason::ContentFilter,
            Some("function_call") => FinishReason::FunctionCall,
            Some(other) => FinishReason::Error(format!("Unknown: {}", other)),
            None => FinishReason::Stop,
        }
    }

    /// Map a wire-reported [`OpenAIUsage`] into [`TokenUsage`], applying the
    /// D-20 cache/reasoning sub-count builders only when the payload
    /// actually carried the figure (D-03: an absent figure is `None`, never
    /// a fabricated `Some(0)`). Shared by both the non-streaming
    /// usage-construction site and the streaming terminal-chunk usage, so
    /// the two paths cannot drift.
    fn map_usage(usage: OpenAIUsage) -> TokenUsage {
        let mut mapped = TokenUsage::new(usage.prompt_tokens, usage.completion_tokens);
        if let Some(cached) = usage
            .prompt_tokens_details
            .and_then(|details| details.cached_tokens)
        {
            mapped = mapped.with_cache_read(cached);
        }
        if let Some(reasoning) = usage
            .completion_tokens_details
            .and_then(|details| details.reasoning_tokens)
        {
            mapped = mapped.with_reasoning(reasoning);
        }
        mapped
    }

    /// Send `request`, retrying transient failures with jittered exponential back-off.
    ///
    /// Network, timeout and 5xx failures are retried up to `max_retries` times. An
    /// authentication failure, a rate limit ([`LlmError::RateLimitExceeded`]) and a spend cap
    /// ([`LlmError::UsageLimitExceeded`]) are NOT: they return on the first attempt, with no
    /// sleep and no second request (D-02, PACE-02). Retrying a 429 inside this loop would
    /// multiply the attempts `RetryPolicy` and `FallbackLlmAdapter` count above the adapter;
    /// the Cadence decorator paces the next call instead.
    async fn make_request_with_retries(
        &self,
        request: &OpenAIRequest,
    ) -> Result<OpenAIResponse, LlmError> {
        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            match self.make_single_request(request).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    last_error = Some(e.clone());

                    // Surfaced on the first attempt, with no sleep and no further request
                    // (D-02, PACE-02): a rate limit or a spend cap cannot be cured by
                    // re-asking within seconds, and retrying it here would multiply the
                    // attempts `RetryPolicy` and `FallbackLlmAdapter` count above this
                    // adapter. The Cadence decorator paces the NEXT call instead.
                    if matches!(
                        e,
                        LlmError::AuthenticationError(_)
                            | LlmError::RateLimitExceeded { .. }
                            | LlmError::UsageLimitExceeded { .. }
                    ) {
                        return Err(e);
                    }

                    if attempt < self.config.max_retries {
                        let base_delay = Duration::from_secs(1);
                        let exponential_delay = base_delay * 2_u32.pow(attempt);
                        let max_delay = Duration::from_secs(10);
                        let delay = exponential_delay.min(max_delay);

                        let jitter_ms = {
                            let mut rng = rand::thread_rng();
                            rng.gen_range(0..=(delay.as_millis() / 5)) as u64
                        };
                        let total_delay = delay + Duration::from_millis(jitter_ms);

                        tokio::time::sleep(total_delay).await;
                    }
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| LlmError::ProcessingError("Maximum retries exceeded".to_string())))
    }

    async fn make_single_request(
        &self,
        request: &OpenAIRequest,
    ) -> Result<OpenAIResponse, LlmError> {
        let url = format!("{}/chat/completions", self.config.base_url);

        let mut req = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json");

        if let Some(org) = &self.config.organization {
            req = req.header("OpenAI-Organization", org);
        }

        let response = req
            .json(request)
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(format!("Request failed: {}", e)))?;

        let status = response.status();
        // Snapshot the 429 headers BEFORE `.text()` consumes the response (research Pitfall 2).
        let hints = snapshot_rate_limit_hints(status, response.headers());
        let response_text = response
            .text()
            .await
            .map_err(|e| LlmError::ProcessingError(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            // Shared status-to-variant mapping for every adapter (D-03,
            // FT-FR-01), with a `300..=399` pre-check (CR-02) since this
            // client refuses to follow redirects.
            return Err(self.map_error_with_hints(status.as_u16(), &response_text, hints));
        }

        serde_json::from_str::<OpenAIResponse>(&response_text)
            .map_err(|e| LlmError::ProcessingError(format!("Failed to parse response: {}", e)))
    }

    async fn make_streaming_request(
        &self,
        request: &OpenAIRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamingResponse, LlmError>> + Send>>, LlmError>
    {
        let url = format!("{}/chat/completions", self.config.base_url);

        let mut req = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json");

        if let Some(org) = &self.config.organization {
            req = req.header("OpenAI-Organization", org);
        }

        let response = req
            .json(request)
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            // Same header snapshot as the generate path, taken before the body read consumes
            // the response (research Pitfall 2).
            let hints = snapshot_rate_limit_hints(status, response.headers());
            let error_text = response.text().await.unwrap_or_default();
            // Same shared mapping as the generate path, so a status yields
            // the same typed variant whether or not the call streams.
            return Err(self.map_error_with_hints(status.as_u16(), &error_text, hints));
        }

        // `flat_map` rather than `map`: a single network chunk can carry more
        // than one complete SSE `data: {...}` event (this is common when a
        // mock transport, or a provider whose TCP framing does not align to
        // event boundaries, writes the whole body at once) -- every `data:`
        // line found is emitted as its own stream item, mirroring
        // `CompatEngine::generate_stream` (D-14/plan 31-03 Task 1).
        //
        // D-14 hold-and-emit terminal-chunk contract: the `finish_reason`
        // frame and the trailing empty-`choices` usage frame can arrive on
        // separate SSE frames, both strictly before `[DONE]`. Both are held
        // in state captured by this `move` closure (a `Stream::map`/
        // `flat_map` closure is `FnMut`, so ordinary mutable locals persist
        // correctly across calls) and emitted together on the ONE `[DONE]`
        // terminal chunk.
        let mut held_finish_reason: Option<FinishReason> = None;
        let mut held_usage: Option<TokenUsage> = None;

        let stream = response.bytes_stream().flat_map(move |chunk_result| {
            let items: Vec<Result<StreamingResponse, LlmError>> = match chunk_result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes).into_owned();
                    let mut items = Vec::new();

                    for line in text.lines() {
                        let Some(data) = line.strip_prefix("data: ") else {
                            continue;
                        };

                        if data.trim() == "[DONE]" {
                            let mut terminal = StreamingResponse::terminal(
                                held_finish_reason.take().unwrap_or(FinishReason::Stop),
                            );
                            if let Some(usage) = held_usage.take() {
                                terminal = terminal.with_usage(usage);
                            }
                            items.push(Ok(terminal));
                            continue;
                        }

                        match serde_json::from_str::<OpenAIStreamChunk>(data) {
                            Ok(chunk) => {
                                if let Some(usage) = chunk.usage {
                                    held_usage = Some(Self::map_usage(usage));
                                }
                                if let Some(choice) = chunk.choices.first() {
                                    if let Some(reason) = &choice.finish_reason {
                                        held_finish_reason = Some(match reason.as_str() {
                                            "stop" => FinishReason::Stop,
                                            "length" => FinishReason::Length,
                                            "content_filter" => FinishReason::ContentFilter,
                                            "function_call" => FinishReason::FunctionCall,
                                            other => {
                                                FinishReason::Error(format!("Unknown: {}", other))
                                            }
                                        });
                                    }
                                    let delta = choice.delta.content.clone().unwrap_or_default();
                                    items.push(Ok(StreamingResponse::delta(delta)));
                                }
                            }
                            Err(e) => {
                                items.push(Err(LlmError::ProcessingError(format!(
                                    "Failed to parse stream chunk: {}",
                                    e
                                ))));
                            }
                        }
                    }

                    items
                }
                Err(e) => vec![Err(LlmError::NetworkError(format!("Stream error: {}", e)))],
            };

            futures::stream::iter(items)
        });

        Ok(Box::pin(stream))
    }
}

#[async_trait]
impl LlmPort for OpenAIAdapter {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let messages = self.convert_to_messages(&request.prompt, &request.attachments)?;

        let temperature = request
            .prompt
            .node
            .node
            .parameters
            .temperature
            .unwrap_or(0.7);
        let max_tokens = request
            .prompt
            .node
            .node
            .parameters
            .max_tokens
            .unwrap_or(4096);

        let openai_request = OpenAIRequest {
            model: request.model.clone(),
            messages,
            temperature: Some(temperature),
            max_tokens: Some(max_tokens),
            top_p: Some(1.0),
            stream: false,
            response_format: request
                .response_format
                .as_ref()
                .map(to_openai_response_format),
            stream_options: None,
        };

        let response = self.make_request_with_retries(&openai_request).await?;

        if response.choices.is_empty() {
            return Err(LlmError::ProcessingError(
                "No choices in response".to_string(),
            ));
        }

        let choice = &response.choices[0];
        let finish_reason = self.convert_finish_reason(choice.finish_reason.clone());
        let content = choice.message.content.clone();
        let usage = Self::map_usage(response.usage);

        Ok(LlmResponse {
            id: Uuid::new_v4(),
            request_id: request.id,
            model: response.model,
            content,
            finish_reason,
            usage,
            cost: None,
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: None,
        })
    }

    async fn generate_stream(
        &self,
        request: LlmRequest,
    ) -> Result<Box<dyn Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError> {
        let messages = self.convert_to_messages(&request.prompt, &request.attachments)?;

        let temperature = request
            .prompt
            .node
            .node
            .parameters
            .temperature
            .unwrap_or(0.7);
        let max_tokens = request
            .prompt
            .node
            .node
            .parameters
            .max_tokens
            .unwrap_or(4096);

        let openai_request = OpenAIRequest {
            model: request.model.clone(),
            messages,
            temperature: Some(temperature),
            max_tokens: Some(max_tokens),
            top_p: Some(1.0),
            stream: true,
            response_format: request
                .response_format
                .as_ref()
                .map(to_openai_response_format),
            stream_options: Some(OpenAIStreamOptions {
                include_usage: true,
            }),
        };

        let stream = self.make_streaming_request(&openai_request).await?;
        Ok(Box::new(stream))
    }

    async fn validate_model(&self, model: &str) -> Result<bool, LlmError> {
        let available_models = self.get_available_models().await?;
        Ok(available_models.contains(&model.to_string()))
    }

    async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
        let url = format!("{}/models", self.config.base_url);

        let mut req = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key));

        if let Some(org) = &self.config.organization {
            req = req.header("OpenAI-Organization", org);
        }

        let response = req
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(format!("Failed to fetch models: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let hints = snapshot_rate_limit_hints(status, response.headers());
            let error_text = response.text().await.unwrap_or_default();
            return Err(self.map_error_with_hints(status.as_u16(), &error_text, hints));
        }

        let response_text = response
            .text()
            .await
            .map_err(|e| LlmError::ProcessingError(format!("Failed to read response: {}", e)))?;

        let models_response: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| LlmError::ProcessingError(format!("Failed to parse response: {}", e)))?;

        let models = models_response["data"]
            .as_array()
            .ok_or_else(|| LlmError::ProcessingError("Invalid models response format".to_string()))?
            .iter()
            .filter_map(|model| model["id"].as_str().map(String::from))
            .collect();

        Ok(models)
    }

    fn get_provider_name(&self) -> &'static str {
        OPENAI_PROVIDER
    }

    fn get_capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_streaming: true,
            // `LlmRequest` carries no field through which a tool definition could
            // travel, and this adapter neither sends `tools` nor parses `tool_calls`
            // out of a response. The flag describes what this adapter does, not what
            // the vendor's API offers (WEB-03, D-14). This adapter's `generate()` also
            // hard-codes the absent function call in the response it builds, so the
            // function-calling flag describes what this adapter does rather than what
            // the vendor's API offers (WEB-03, D-12).
            supports_tool_calling: false,
            supports_function_calling: false,
            supports_vision: true,
            max_context_tokens: Some(128000),
            supports_embeddings: true,
            supports_system_messages: true,
            temperature_range: Some((0.0, 1.0)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = OpenAIConfig::new("test-key".to_string());
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.base_url, "https://api.openai.com/v1");
        assert_eq!(config.timeout_seconds, 300);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_config_validation() {
        let valid_config = OpenAIConfig::new("test-key".to_string());
        assert!(valid_config.validate().is_ok());

        let invalid_config = OpenAIConfig {
            api_key: String::new(),
            base_url: "https://api.openai.com/v1".to_string(),
            organization: None,
            timeout_seconds: 300,
            max_retries: 3,
        };
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_adapter_creation() {
        let config = OpenAIConfig::new("test-key".to_string());
        let adapter = OpenAIAdapter::new(config);
        assert!(adapter.is_ok());
    }

    #[test]
    fn test_get_provider_name() {
        let config = OpenAIConfig::new("test-key".to_string());
        let adapter = OpenAIAdapter::new(config).unwrap();
        assert_eq!(adapter.get_provider_name(), "openai");
    }

    #[test]
    fn test_get_capabilities() {
        let config = OpenAIConfig::new("test-key".to_string());
        let adapter = OpenAIAdapter::new(config).unwrap();
        let caps = adapter.get_capabilities();
        assert!(caps.supports_streaming);
        // `LlmRequest` has no field through which a tool definition could travel, and
        // this adapter neither sends `tools` nor parses `tool_calls` (WEB-03, D-14).
        assert!(!caps.supports_tool_calling);
        // This adapter hard-codes the absent function call in the response it
        // builds, so the flag describes what this adapter does rather than what
        // the vendor's API offers (WEB-03, D-12).
        assert!(!caps.supports_function_calling);
        assert!(caps.supports_vision);
        assert_eq!(caps.max_context_tokens, Some(128000));
        assert_eq!(caps.temperature_range, Some((0.0, 1.0)));
    }

    #[test]
    fn test_config_with_organization() {
        let mut config = OpenAIConfig::new("test-key".to_string());
        config.organization = Some("org-123".to_string());
        assert_eq!(config.organization, Some("org-123".to_string()));
    }

    #[test]
    fn test_config_validation_empty_base_url() {
        let config = OpenAIConfig {
            api_key: "test-key".to_string(),
            base_url: String::new(),
            organization: None,
            timeout_seconds: 300,
            max_retries: 3,
        };
        assert!(config.validate().is_err());
    }

    // ── CR-02 (`25-REVIEW.md`): refused-redirect mapping ───────────────────

    #[test]
    fn map_error_maps_a_redirect_status_to_an_actionable_provider_error() {
        let config = OpenAIConfig::new("test-key".to_string());
        let adapter = OpenAIAdapter::new(config).unwrap();

        for expected in [301u16, 302, 307] {
            match adapter.map_error(expected, "moved") {
                LlmError::ProviderError {
                    provider,
                    status,
                    message,
                } => {
                    assert_eq!(provider, "openai");
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

    // ── Phase 25 (FT-FR-01, D-03): non-2xx routes through map_http_status ──

    mod status_mapping {
        use super::*;
        use mockito::Server;
        use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};

        /// `max_retries: 0` so a retryable status surfaces after one attempt
        /// instead of sleeping through the adapter's 1s-base backoff.
        fn adapter_at(base_url: &str) -> OpenAIAdapter {
            OpenAIAdapter::new(OpenAIConfig {
                api_key: "test-key".to_string(),
                base_url: base_url.to_string(),
                organization: None,
                timeout_seconds: 5,
                max_retries: 0,
            })
            .expect("test config must build a valid adapter")
        }

        fn build_request(stream: bool) -> LlmRequest {
            LlmRequest::new(
                "gpt-4o",
                PromptItem::new(PromptType::User(UserPrompt {
                    query: "Hello".to_string(),
                    context: None,
                }))
                .expect("a user prompt must build"),
            )
            .with_stream(stream)
        }

        #[tokio::test]
        async fn openai_non_2xx_routes_through_the_shared_mapper() {
            let mut server = Server::new_async().await;
            server
                .mock("POST", "/chat/completions")
                .with_status(503)
                .with_body(r#"{"error":{"message":"overloaded"}}"#)
                .create_async()
                .await;

            let result = adapter_at(&server.url())
                .generate(build_request(false))
                .await;
            match result {
                Err(LlmError::ProviderError {
                    provider, status, ..
                }) => {
                    assert_eq!(provider, "openai");
                    assert_eq!(status, 503);
                }
                other => panic!("expected ProviderError {{ status: 503 }}, got {other:?}"),
            }
        }

        #[tokio::test]
        async fn openai_dedicated_status_mappings_are_unchanged() {
            for (status, body, expect) in [
                (401u16, r#"{"error":"invalid key"}"#, "AuthenticationError"),
                (429, r#"{"error":"slow down"}"#, "RateLimitExceeded"),
                (400, r#"{"error":"bad request"}"#, "InvalidPrompt"),
                (
                    400,
                    r#"{"error":"This model's maximum context length is 8192 tokens"}"#,
                    "TokenLimitExceeded",
                ),
            ] {
                let mut server = Server::new_async().await;
                server
                    .mock("POST", "/chat/completions")
                    .with_status(status.into())
                    .with_body(body)
                    .create_async()
                    .await;

                let err = adapter_at(&server.url())
                    .generate(build_request(false))
                    .await
                    .expect_err("non-2xx must be an error");
                let ok = matches!(
                    (expect, &err),
                    ("AuthenticationError", LlmError::AuthenticationError(_))
                        | ("RateLimitExceeded", LlmError::RateLimitExceeded { .. })
                        | ("InvalidPrompt", LlmError::InvalidPrompt(_))
                        | ("TokenLimitExceeded", LlmError::TokenLimitExceeded)
                );
                assert!(ok, "status {status}: expected {expect}, got {err:?}");
            }
        }

        /// D-02, PACE-02: with `max_retries: 3` a 429 is surfaced on the FIRST attempt -- the
        /// mock is hit exactly once and no back-off sleep is spent.
        #[tokio::test]
        async fn openai_429_is_surfaced_on_the_first_attempt() {
            let mut server = Server::new_async().await;
            let mock = server
                .mock("POST", "/chat/completions")
                .with_status(429)
                .with_body(r#"{"error":{"message":"slow down"}}"#)
                .expect(1)
                .create_async()
                .await;
            let adapter = OpenAIAdapter::new(OpenAIConfig {
                api_key: "test-key".to_string(),
                base_url: server.url(),
                organization: None,
                timeout_seconds: 5,
                max_retries: 3,
            })
            .expect("test config must build a valid adapter");

            let started = std::time::Instant::now();
            let result = adapter.generate(build_request(false)).await;

            assert!(
                matches!(result, Err(LlmError::RateLimitExceeded { .. })),
                "expected RateLimitExceeded, got {result:?}"
            );
            assert!(
                started.elapsed() < Duration::from_millis(900),
                "a surfaced 429 must not sleep through the retry back-off ({:?})",
                started.elapsed()
            );
            mock.assert_async().await;
        }

        #[tokio::test]
        async fn openai_client_refuses_to_follow_a_redirect() {
            // CR-02 (`25-REVIEW.md`) end-to-end regression: a `302` from the
            // configured base URL must surface as a refused-redirect
            // `ProviderError`, and the redirect target must never receive a
            // request — proving the `Authorization` header was never
            // replayed rather than merely asserting on the returned error
            // shape.
            let mut server = Server::new_async().await;
            let redirect_target = server
                .mock("POST", "/redirected")
                .expect(0)
                .create_async()
                .await;
            server
                .mock("POST", "/chat/completions")
                .with_status(302)
                .with_header("Location", "/redirected")
                .with_body("moved")
                .create_async()
                .await;

            let result = adapter_at(&server.url())
                .generate(build_request(false))
                .await;

            match result {
                Err(LlmError::ProviderError {
                    provider,
                    status,
                    message,
                }) => {
                    assert_eq!(provider, "openai");
                    assert_eq!(status, 302);
                    assert!(message.contains("redirect"), "got: {message}");
                }
                other => panic!("expected ProviderError {{ status: 302 }}, got {other:?}"),
            }
            redirect_target.assert_async().await;
        }

        #[tokio::test]
        async fn streaming_non_2xx_routes_through_the_shared_mapper() {
            let mut server = Server::new_async().await;
            server
                .mock("POST", "/chat/completions")
                .with_status(503)
                .with_body(r#"{"error":{"message":"overloaded"}}"#)
                .create_async()
                .await;

            let result = adapter_at(&server.url())
                .generate_stream(build_request(true))
                .await;
            // `Ok` carries a boxed `dyn Stream` with no `Debug`, so match by hand.
            match &result {
                Err(LlmError::ProviderError {
                    provider, status, ..
                }) => {
                    assert_eq!(provider, "openai");
                    assert_eq!(*status, 503);
                }
                Ok(_) => panic!("expected Err(ProviderError), got Ok(<stream>)"),
                Err(other) => panic!("expected ProviderError {{ status: 503 }}, got {other:?}"),
            }
        }
    }

    // ── Phase 43 (PACE-01, D-02, Pitfall 6): 429 headers and quota-class bodies ──

    mod rate_limit_wiring {
        use super::*;
        use mockito::Server;
        use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
        use paladin_ports::output::rate_limit_hints::{RateLimitDimensionKind, RetryDelaySource};

        /// `max_retries: 3` on purpose: a 429 mock with `.expect(1)` then proves the adapter's
        /// own loop never re-asks, and the back-off would make a regression visibly slow.
        fn adapter_at(base_url: &str) -> OpenAIAdapter {
            OpenAIAdapter::new(OpenAIConfig {
                api_key: "test-key".to_string(),
                base_url: base_url.to_string(),
                organization: None,
                timeout_seconds: 5,
                max_retries: 3,
            })
            .expect("test config must build a valid adapter")
        }

        fn build_request(stream: bool) -> LlmRequest {
            LlmRequest::new(
                "gpt-4o",
                PromptItem::new(PromptType::User(UserPrompt {
                    query: "Hello".to_string(),
                    context: None,
                }))
                .expect("a user prompt must build"),
            )
            .with_stream(stream)
        }

        const SECS: fn(u64) -> Duration = Duration::from_secs;

        fn with_ratelimit_headers(mock: mockito::Mock, retry_after: Option<&str>) -> mockito::Mock {
            let mock = mock
                .with_header("x-ratelimit-limit-requests", "60")
                .with_header("x-ratelimit-remaining-requests", "0")
                .with_header("x-ratelimit-reset-requests", "6m0s")
                .with_header("x-ratelimit-limit-tokens", "150000")
                .with_header("x-ratelimit-remaining-tokens", "149984")
                .with_header("x-ratelimit-reset-tokens", "1s");
            match retry_after {
                Some(v) => mock.with_header("Retry-After", v),
                None => mock,
            }
        }

        fn assert_dimensions(err: &LlmError) {
            let hints = err.rate_limit_hints().expect("the 429 headers were parsed");
            let requests = hints.dimension(RateLimitDimensionKind::Requests);
            assert_eq!(requests.limit(), Some(60));
            assert_eq!(requests.remaining(), Some(0));
            assert_eq!(requests.reset_after(), Some(SECS(360)));
            let tokens = hints.dimension(RateLimitDimensionKind::Tokens);
            assert_eq!(tokens.limit(), Some(150_000));
            assert_eq!(tokens.remaining(), Some(149_984));
            assert_eq!(tokens.reset_after(), Some(SECS(1)));
        }

        #[tokio::test]
        async fn openai_429_carries_retry_after_and_ratelimit_dimensions() {
            let mut server = Server::new_async().await;
            let mock = with_ratelimit_headers(
                server
                    .mock("POST", "/chat/completions")
                    .with_status(429)
                    .with_body(r#"{"error":{"message":"Rate limit reached","code":"rate_limit_exceeded"}}"#),
                Some("7"),
            )
            .expect(1)
            .create_async()
            .await;

            let err = adapter_at(&server.url())
                .generate(build_request(false))
                .await
                .expect_err("a 429 is an error");

            assert!(matches!(err, LlmError::RateLimitExceeded { .. }), "{err:?}");
            assert_eq!(err.retry_after(), Some(SECS(7)));
            assert_eq!(
                err.rate_limit_hints()
                    .and_then(|h| h.explicit_retry_after()),
                Some((SECS(7), RetryDelaySource::RetryAfter))
            );
            assert_dimensions(&err);
            // The raw header strings never reach a rendered error (T-43-10): only the parsed
            // numbers do, so the Go-duration text `6m0s` must not appear anywhere.
            for rendered in [format!("{err}"), format!("{err:?}")] {
                assert!(!rendered.contains("6m0s"), "{rendered}");
            }
            mock.assert_async().await;
        }

        #[tokio::test]
        async fn openai_429_without_retry_after_falls_back_to_the_exhausted_reset() {
            let mut server = Server::new_async().await;
            let mock = with_ratelimit_headers(
                server
                    .mock("POST", "/chat/completions")
                    .with_status(429)
                    .with_body(r#"{"error":{"message":"Rate limit reached"}}"#),
                None,
            )
            .expect(1)
            .create_async()
            .await;

            let err = adapter_at(&server.url())
                .generate(build_request(false))
                .await
                .expect_err("a 429 is an error");

            // 6m0s on the exhausted requests dimension, tagged as derived from a reset.
            assert_eq!(err.retry_after(), Some(SECS(360)));
            let hints = err.rate_limit_hints().expect("hints");
            assert_eq!(hints.explicit_retry_after(), None);
            assert_eq!(
                hints.effective_retry_after(),
                Some((SECS(360), RetryDelaySource::ResetHeader))
            );
            mock.assert_async().await;
        }

        #[tokio::test]
        async fn openai_429_with_no_rate_limit_header_carries_no_hints() {
            let mut server = Server::new_async().await;
            let mock = server
                .mock("POST", "/chat/completions")
                .with_status(429)
                .with_body(r#"{"error":{"message":"slow down"}}"#)
                .expect(1)
                .create_async()
                .await;

            let err = adapter_at(&server.url())
                .generate(build_request(false))
                .await
                .expect_err("a 429 is an error");

            assert!(matches!(err, LlmError::RateLimitExceeded { .. }), "{err:?}");
            assert_eq!(err.retry_after(), None);
            assert!(err.rate_limit_hints().is_none());
            mock.assert_async().await;
        }

        #[tokio::test]
        async fn openai_stream_429_carries_the_same_hints() {
            let mut server = Server::new_async().await;
            let mock = with_ratelimit_headers(
                server
                    .mock("POST", "/chat/completions")
                    .with_status(429)
                    .with_body(r#"{"error":{"message":"Rate limit reached"}}"#),
                Some("7"),
            )
            .expect(1)
            .create_async()
            .await;

            let result = adapter_at(&server.url())
                .generate_stream(build_request(true))
                .await;
            let err = match result {
                Err(err) => err,
                Ok(_) => panic!("expected Err(RateLimitExceeded), got Ok(<stream>)"),
            };

            assert!(matches!(err, LlmError::RateLimitExceeded { .. }), "{err:?}");
            assert_eq!(err.retry_after(), Some(SECS(7)));
            assert_dimensions(&err);
            mock.assert_async().await;
        }

        #[tokio::test]
        async fn openai_insufficient_quota_429_maps_to_usage_limit_exceeded_once() {
            // Both spellings OpenAI uses for the same condition: the `code` and the `type`.
            for body in [
                r#"{"error":{"message":"You exceeded your current quota","type":"insufficient_quota","code":"insufficient_quota"}}"#,
                r#"{"error":{"message":"You exceeded your current quota","type":"insufficient_quota"}}"#,
                r#"{"error":{"message":"You exceeded your current quota","code":"insufficient_quota"}}"#,
            ] {
                let mut server = Server::new_async().await;
                // A Retry-After on a quota error must not turn it into a paced rate limit.
                let mock = server
                    .mock("POST", "/chat/completions")
                    .with_status(429)
                    .with_header("Retry-After", "7")
                    .with_body(body)
                    .expect(1)
                    .create_async()
                    .await;

                let err = adapter_at(&server.url())
                    .generate(build_request(false))
                    .await
                    .expect_err("a quota 429 is an error");

                match &err {
                    LlmError::UsageLimitExceeded {
                        provider,
                        regain_hint,
                    } => {
                        assert_eq!(provider, "openai");
                        assert_eq!(*regain_hint, None);
                    }
                    other => panic!("expected UsageLimitExceeded for {body}, got {other:?}"),
                }
                assert!(err.retry_after().is_none());
                mock.assert_async().await;
            }
        }

        #[tokio::test]
        async fn openai_stream_insufficient_quota_429_maps_to_usage_limit_exceeded() {
            let mut server = Server::new_async().await;
            let mock = server
                .mock("POST", "/chat/completions")
                .with_status(429)
                .with_body(r#"{"error":{"type":"insufficient_quota","code":"insufficient_quota"}}"#)
                .expect(1)
                .create_async()
                .await;

            let result = adapter_at(&server.url())
                .generate_stream(build_request(true))
                .await;
            assert!(
                matches!(&result, Err(LlmError::UsageLimitExceeded { .. })),
                "expected UsageLimitExceeded, got {:?}",
                result.as_ref().err()
            );
            mock.assert_async().await;
        }

        #[tokio::test]
        async fn openai_ordinary_rate_limit_codes_are_not_mistaken_for_a_quota() {
            for body in [
                r#"{"error":{"code":"rate_limit_exceeded","type":"requests"}}"#,
                r#"{"error":{"message":"mentions insufficient_quota in prose only"}}"#,
                "not json at all",
                "",
                r#"{"error":"insufficient_quota"}"#,
            ] {
                let mut server = Server::new_async().await;
                let mock = server
                    .mock("POST", "/chat/completions")
                    .with_status(429)
                    .with_body(body)
                    .expect(1)
                    .create_async()
                    .await;
                let err = adapter_at(&server.url())
                    .generate(build_request(false))
                    .await
                    .expect_err("a 429 is an error");
                assert!(
                    matches!(err, LlmError::RateLimitExceeded { .. }),
                    "body {body:?}: {err:?}"
                );
                mock.assert_async().await;
            }
        }

        #[tokio::test]
        async fn openai_non_429_errors_ignore_rate_limit_headers() {
            let mut server = Server::new_async().await;
            let mock = with_ratelimit_headers(
                server
                    .mock("POST", "/chat/completions")
                    .with_status(401)
                    .with_body(r#"{"error":"invalid key"}"#),
                Some("7"),
            )
            .expect(1)
            .create_async()
            .await;
            let err = adapter_at(&server.url())
                .generate(build_request(false))
                .await
                .expect_err("a 401 is an error");
            assert!(matches!(err, LlmError::AuthenticationError(_)), "{err:?}");
            assert!(err.rate_limit_hints().is_none());
            mock.assert_async().await;
        }
    }

    // ── Phase 26 (RT-05, D-28): response_format reaches the wire ──────────

    mod response_format_wiring {
        use super::*;
        use mockito::Server;
        use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
        use serde_json::Value;
        use std::collections::BTreeSet;
        use std::sync::{Arc, Mutex};

        fn adapter_at(base_url: &str) -> OpenAIAdapter {
            OpenAIAdapter::new(OpenAIConfig {
                api_key: "test-key".to_string(),
                base_url: base_url.to_string(),
                organization: None,
                timeout_seconds: 5,
                max_retries: 0,
            })
            .expect("test config must build a valid adapter")
        }

        fn build_request() -> LlmRequest {
            LlmRequest::new(
                "gpt-4o",
                PromptItem::new(PromptType::User(UserPrompt {
                    query: "Hello".to_string(),
                    context: None,
                }))
                .expect("a user prompt must build"),
            )
        }

        /// Runs `generate()` against a mock `/chat/completions` endpoint,
        /// capturing the raw outgoing request body as parsed JSON — mirrors
        /// `CompatEngine`'s `generate_and_capture_body` test helper
        /// (`compat/engine.rs`). Asserting on the parsed JSON rather than a
        /// raw-string substring keeps this immune to key-ordering changes.
        async fn generate_and_capture_body(request: LlmRequest) -> Value {
            let mut server = Server::new_async().await;
            let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
            let captured_clone = Arc::clone(&captured);

            server
                .mock("POST", "/chat/completions")
                .with_status(200)
                .with_body_from_request(move |req| {
                    let body_text = req.utf8_lossy_body().unwrap_or_default().into_owned();
                    *captured_clone.lock().unwrap() = Some(body_text);
                    serde_json::json!({
                        "id": "cmpl-1",
                        "model": "gpt-4o",
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

            let result = adapter_at(&server.url()).generate(request).await;
            assert!(
                result.is_ok(),
                "mock server returned a well-formed response: {result:?}"
            );

            let body_text = captured
                .lock()
                .unwrap()
                .take()
                .expect("mock must have been called exactly once");
            serde_json::from_str(&body_text).expect("captured body must be valid JSON")
        }

        #[tokio::test]
        async fn openai_request_carries_response_format_json_object() {
            let request = build_request().with_response_format(ResponseFormat::JsonObject);
            let body = generate_and_capture_body(request).await;

            assert_eq!(
                body.get("response_format"),
                Some(&serde_json::json!({"type": "json_object"}))
            );
        }

        #[tokio::test]
        async fn openai_request_carries_response_format_json_schema() {
            let schema = serde_json::json!({
                "type": "object",
                "properties": {"answer": {"type": "string"}}
            });
            let request = build_request().with_response_format(ResponseFormat::JsonSchema {
                name: "answer_schema".to_string(),
                schema: schema.clone(),
                strict: true,
            });

            let body = generate_and_capture_body(request).await;

            assert_eq!(body["response_format"]["type"], "json_schema");
            assert_eq!(
                body["response_format"]["json_schema"]["name"],
                "answer_schema"
            );
            assert_eq!(body["response_format"]["json_schema"]["schema"], schema);
            assert_eq!(body["response_format"]["json_schema"]["strict"], true);
        }

        #[tokio::test]
        async fn openai_request_without_response_format_is_byte_identical_to_today() {
            let body = generate_and_capture_body(build_request()).await;
            let obj = body.as_object().expect("body must be a JSON object");

            assert!(
                !obj.contains_key("response_format"),
                "absent response_format must not appear on the wire, got: {obj:?}"
            );
            // The full key set must be exactly what a pre-0.10 request
            // produced -- proving this is an additive, X-03-compliant change
            // rather than an accidental reshape of the existing fields.
            let keys: BTreeSet<&str> = obj.keys().map(String::as_str).collect();
            assert_eq!(
                keys,
                BTreeSet::from([
                    "model",
                    "messages",
                    "temperature",
                    "max_tokens",
                    "top_p",
                    "stream"
                ])
            );
        }
    }

    // ── Phase 31 (D-13, D-14, D-15, D-20): streaming usage terminal-chunk
    //    contract, and cache/reasoning sub-count mapping on both paths ────

    mod streaming_usage_wiring {
        use super::*;
        use futures::StreamExt;
        use mockito::Server;
        use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
        use serde_json::{Value, json};
        use std::sync::{Arc, Mutex};

        fn adapter_at(base_url: &str) -> OpenAIAdapter {
            OpenAIAdapter::new(OpenAIConfig {
                api_key: "test-key".to_string(),
                base_url: base_url.to_string(),
                organization: None,
                timeout_seconds: 5,
                max_retries: 0,
            })
            .expect("test config must build a valid adapter")
        }

        fn build_request() -> LlmRequest {
            LlmRequest::new(
                "gpt-4o",
                PromptItem::new(PromptType::User(UserPrompt {
                    query: "Hello".to_string(),
                    context: None,
                }))
                .expect("a user prompt must build"),
            )
        }

        /// Distinct, non-round figures so a swapped or dropped field cannot
        /// pass by coincidence (house style, `table_herald.rs`/`json_herald.rs`).
        fn detail_bearing_usage_json() -> Value {
            json!({
                "prompt_tokens": 800,
                "completion_tokens": 900,
                "total_tokens": 1700,
                "prompt_tokens_details": {"cached_tokens": 128},
                "completion_tokens_details": {"reasoning_tokens": 640}
            })
        }

        #[tokio::test]
        async fn generate_maps_cache_and_reasoning_when_the_payload_carries_them() {
            let mut server = Server::new_async().await;
            server
                .mock("POST", "/chat/completions")
                .with_status(200)
                .with_body(
                    json!({
                        "id": "cmpl-1",
                        "model": "gpt-4o",
                        "choices": [{
                            "index": 0,
                            "message": {"role": "assistant", "content": "ok"},
                            "finish_reason": "stop"
                        }],
                        "usage": detail_bearing_usage_json()
                    })
                    .to_string(),
                )
                .create_async()
                .await;

            let response = adapter_at(&server.url())
                .generate(build_request())
                .await
                .unwrap();

            assert_eq!(response.usage.cache_read_tokens, Some(128));
            assert_eq!(response.usage.reasoning_tokens, Some(640));
            assert_eq!(response.usage.cache_write_tokens, None);
        }

        #[tokio::test]
        async fn generate_leaves_cache_and_reasoning_none_when_the_payload_omits_them() {
            let mut server = Server::new_async().await;
            server
                .mock("POST", "/chat/completions")
                .with_status(200)
                .with_body(
                    json!({
                        "id": "cmpl-1",
                        "model": "gpt-4o",
                        "choices": [{
                            "index": 0,
                            "message": {"role": "assistant", "content": "ok"},
                            "finish_reason": "stop"
                        }],
                        "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
                    })
                    .to_string(),
                )
                .create_async()
                .await;

            let response = adapter_at(&server.url())
                .generate(build_request())
                .await
                .unwrap();

            assert_eq!(response.usage.cache_read_tokens, None);
            assert_eq!(response.usage.cache_write_tokens, None);
            assert_eq!(response.usage.reasoning_tokens, None);
        }

        #[tokio::test]
        async fn streaming_request_carries_stream_options_include_usage() {
            let mut server = Server::new_async().await;
            let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
            let captured_clone = Arc::clone(&captured);

            server
                .mock("POST", "/chat/completions")
                .with_status(200)
                .with_header("content-type", "text/event-stream")
                .with_body_from_request(move |req| {
                    let body_text = req.utf8_lossy_body().unwrap_or_default().into_owned();
                    *captured_clone.lock().unwrap() = Some(body_text);
                    b"data: [DONE]\n\n".to_vec()
                })
                .create_async()
                .await;

            let stream = adapter_at(&server.url())
                .generate_stream(build_request())
                .await
                .unwrap();
            let mut stream = Box::into_pin(stream);
            while stream.next().await.is_some() {}

            let body_text = captured
                .lock()
                .unwrap()
                .clone()
                .expect("request must have been captured");
            let body: Value = serde_json::from_str(&body_text).unwrap();
            assert_eq!(
                body.get("stream_options"),
                Some(&json!({"include_usage": true}))
            );
        }

        #[tokio::test]
        async fn streaming_terminal_chunk_carries_the_same_usage_as_the_non_streaming_body() {
            let mut server = Server::new_async().await;
            server
                .mock("POST", "/chat/completions")
                .with_status(200)
                .with_body(
                    json!({
                        "id": "cmpl-1",
                        "model": "gpt-4o",
                        "choices": [{
                            "index": 0,
                            "message": {"role": "assistant", "content": "Hello world"},
                            "finish_reason": "stop"
                        }],
                        "usage": detail_bearing_usage_json()
                    })
                    .to_string(),
                )
                .create_async()
                .await;
            let non_streaming = adapter_at(&server.url())
                .generate(build_request())
                .await
                .unwrap();

            let mut stream_server = Server::new_async().await;
            let sse_body = format!(
                "data: {{\"id\":\"1\",\"choices\":[{{\"index\":0,\"delta\":{{\"content\":\"Hel\"}},\"finish_reason\":null}}]}}\n\n\
                 data: {{\"id\":\"1\",\"choices\":[{{\"index\":0,\"delta\":{{\"content\":\"lo\"}},\"finish_reason\":null}}]}}\n\n\
                 data: {{\"id\":\"1\",\"choices\":[{{\"index\":0,\"delta\":{{\"content\":\"\"}},\"finish_reason\":\"stop\"}}]}}\n\n\
                 data: {{\"id\":\"1\",\"choices\":[],\"usage\":{}}}\n\n\
                 data: [DONE]\n\n",
                detail_bearing_usage_json()
            );
            stream_server
                .mock("POST", "/chat/completions")
                .with_status(200)
                .with_header("content-type", "text/event-stream")
                .with_body(sse_body)
                .create_async()
                .await;

            let stream = adapter_at(&stream_server.url())
                .generate_stream(build_request())
                .await
                .unwrap();
            let mut stream = Box::into_pin(stream);

            let mut chunks = Vec::new();
            while let Some(item) = stream.next().await {
                chunks.push(item.unwrap());
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
            assert_eq!(finish_indices.len(), 1);
            assert_eq!(usage_indices.len(), 1);
            assert_eq!(
                finish_indices, usage_indices,
                "the finish-reason chunk and the usage chunk must be the SAME chunk"
            );
            assert_eq!(chunks[usage_indices[0]].usage, Some(non_streaming.usage));

            let assembled: String = chunks.iter().map(|c| c.delta.as_str()).collect();
            assert_eq!(assembled, "Hello");
        }
    }

    // ── Shared conformance suite (D-19, plan 31-04) ──
    //
    // Nested in its own module (rather than inline in `mod tests`) so every generated test's
    // full path contains "conformance" -- `cargo test --lib conformance` (the plan's own
    // acceptance criterion) selects it by that substring. Bodies below mirror the
    // `streaming_usage_wiring` module's own hand-written tests above, adapted to
    // `ConformanceFixture`'s shape.
    mod conformance_suite {
        use super::*;
        use serde_json::json;
        use std::sync::Arc;

        struct OpenAiFixture;

        impl crate::conformance::ConformanceFixture for OpenAiFixture {
            const WIRE: crate::conformance::Wire = crate::conformance::Wire::OpenAiChat;

            fn adapter(base_url: &str) -> Arc<dyn LlmPort> {
                Arc::new(
                    OpenAIAdapter::new(OpenAIConfig {
                        api_key: "test-key".to_string(),
                        base_url: base_url.to_string(),
                        organization: None,
                        timeout_seconds: 5,
                        max_retries: 0,
                    })
                    .expect("test config must build a valid adapter"),
                )
            }

            fn success_body() -> String {
                json!({
                    "id": "cmpl-1",
                    "model": "gpt-4o",
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
                // `OpenAIStreamChoice.index` is a required (non-`Option`) field on this
                // adapter's OWN stream struct (unlike `CompatEngine`'s shape), so every choice
                // object below carries it.
                concat!(
                    "data: {\"id\":\"1\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hel\"},\"finish_reason\":null}]}\n\n",
                    "data: {\"id\":\"1\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"lo \"},\"finish_reason\":null}]}\n\n",
                    "data: {\"id\":\"1\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"world\"},\"finish_reason\":\"stop\"}]}\n\n",
                    "data: {\"id\":\"1\",\"choices\":[],\"usage\":{\"prompt_tokens\":5,\"completion_tokens\":3,\"total_tokens\":8}}\n\n",
                    "data: [DONE]\n\n",
                )
                .to_string()
            }

            fn error_body(status: u16) -> String {
                json!({"error": {"message": format!("mock error for status {status}"), "type": "mock_error"}})
                    .to_string()
            }
        }

        crate::llm_conformance_suite!(OpenAiFixture);
    }
}
