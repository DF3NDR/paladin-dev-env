//! DeepSeek LLM Adapter
//!
//! Provides integration with DeepSeek's API, which is OpenAI-compatible.
//! Supports standard completions, streaming, and all core LlmPort functionality.

use async_trait::async_trait;
use chrono::Utc;
use futures::{Stream, StreamExt};
use reqwest::{
    Client,
    header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::time::Duration;
use uuid::Uuid;

use paladin_core::platform::container::prompt::{PromptItem, PromptType};
use paladin_ports::output::llm_port::{
    FinishReason, LlmError, LlmPort, LlmRequest, LlmResponse, ProviderCapabilities, ResponseFormat,
    StreamingResponse, TokenUsage,
};

use crate::http_status::map_http_status;
// WR-01 (`25-REVIEW.md`): this adapter previously carried its own
// byte-for-byte copy of the crate's shared credential-redaction routine
// (`RESPONSE_EXCERPT_CHAR_BUDGET`, `CREDENTIAL_PLACEHOLDER`,
// `bounded_excerpt`, `redact_token_after`, `redact_credentials`) rather than
// importing it, so a future fix to the shared module would silently not
// apply here. Mirrors `compat/engine.rs` and `gemini/adapter.rs`, which
// already import from `crate::redaction`.
#[cfg(test)]
use crate::redaction::CREDENTIAL_PLACEHOLDER;
use crate::redaction::{RESPONSE_EXCERPT_CHAR_BUDGET, bounded_excerpt, redact_credentials};

/// The provider name this adapter reports through [`LlmPort::get_provider_name`]
/// and stamps on every [`LlmError::ProviderError`] it emits.
const DEEPSEEK_PROVIDER: &str = "deepseek";

/// Configuration for DeepSeek LLM adapter.
#[derive(Debug, Clone)]
pub struct DeepSeekConfig {
    /// API key for DeepSeek authentication.
    pub api_key: String,
    /// Base URL for DeepSeek API.
    pub base_url: String,
    /// Default model to use.
    pub model: String,
    /// Request timeout in seconds.
    pub timeout_seconds: u64,
}

impl DeepSeekConfig {
    /// Load configuration from environment variables.
    ///
    /// # Environment Variables
    /// - `DEEPSEEK_API_KEY` (required): DeepSeek API key
    /// - `DEEPSEEK_BASE_URL` (optional): API base URL
    /// - `DEEPSEEK_MODEL` (optional): Default model
    /// - `DEEPSEEK_TIMEOUT_SECONDS` (optional): Request timeout
    ///
    /// # Errors
    /// Returns error if required environment variables are missing or invalid.
    pub fn from_env() -> Result<Self, String> {
        let api_key = env::var("DEEPSEEK_API_KEY")
            .map_err(|_| "DEEPSEEK_API_KEY environment variable not set")?;

        let base_url = env::var("DEEPSEEK_BASE_URL")
            .unwrap_or_else(|_| "https://api.deepseek.com/v1".to_string());

        let model = env::var("DEEPSEEK_MODEL").unwrap_or_else(|_| "deepseek-chat".to_string());

        let timeout_seconds = env::var("DEEPSEEK_TIMEOUT_SECONDS")
            .unwrap_or_else(|_| "60".to_string())
            .parse()
            .map_err(|_| "Invalid DEEPSEEK_TIMEOUT_SECONDS value")?;

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

// ── DeepSeek API request structures (OpenAI-compatible) ─────────────────────

#[derive(Debug, Serialize)]
struct DeepSeekRequest {
    model: String,
    messages: Vec<DeepSeekMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    presence_penalty: Option<f32>,
    stream: bool,
    /// `LlmRequest.response_format` on the wire (RT-FR-17, D-28). Omitted
    /// entirely when the caller sets no hint, keeping the body byte
    /// -identical to a pre-0.10 request (X-03).
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<DeepSeekResponseFormat>,
    /// Requests the trailing `usage` frame on a streaming call (D-13/D-15).
    /// `Some({"include_usage": true})` on every streaming request; omitted
    /// entirely on a non-streaming one, keeping that body byte-identical to
    /// a pre-0.10 request (X-03).
    #[serde(skip_serializing_if = "Option::is_none")]
    stream_options: Option<DeepSeekStreamOptions>,
}

/// `DeepSeekRequest.stream_options`'s only shape this adapter sends (D-15).
#[derive(Debug, Serialize)]
struct DeepSeekStreamOptions {
    include_usage: bool,
}

/// The provider-agnostic `response_format` hint, compiled down to
/// DeepSeek's own plain JSON-object wire shape (D-28).
///
/// DeepSeek's chat-completions API supports `{"type":"json_object"}`; it has
/// no schema-carrying native mode, so a `ResponseFormat::JsonSchema` request
/// degrades to this same shape rather than being omitted
/// (EDGE(RT-05/wire shape)). Correctness never depends on it — the caller
/// also appends the schema-conformance instruction block (D-27).
#[derive(Debug, Serialize)]
struct DeepSeekResponseFormat {
    #[serde(rename = "type")]
    kind: &'static str,
}

#[derive(Debug, Serialize, Deserialize)]
struct DeepSeekMessage {
    role: String,
    /// The visible answer. Deserialized through
    /// [`deserialize_null_as_empty_string`] because a reasoning model that
    /// spends its whole budget on hidden reasoning may report the empty
    /// answer as `null` rather than `""`. Serialization is unaffected —
    /// outgoing messages always emit a plain JSON string.
    #[serde(default, deserialize_with = "deserialize_null_as_empty_string")]
    content: String,
    /// Hidden chain-of-thought content emitted by DeepSeek's reasoning models
    /// (e.g. `-flash`/`-pro`). Only ever present on responses; omitted from
    /// outgoing requests. Deserialized so a reasoning-model response round-trips
    /// without a parse error, and observed for diagnostics — never executed or
    /// treated as an instruction (see threat register T-16-04-02).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reasoning_content: Option<String>,
}

// ── DeepSeek API response structures ────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct DeepSeekResponse {
    #[serde(rename = "id")]
    #[allow(dead_code)]
    _id: String,
    model: String,
    choices: Vec<DeepSeekChoice>,
    usage: DeepSeekUsage,
}

#[derive(Debug, Deserialize)]
struct DeepSeekChoice {
    #[serde(rename = "index")]
    #[allow(dead_code)]
    _index: u32,
    message: DeepSeekMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeepSeekUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
    /// `prompt_cache_hit_tokens` (D-20, confirmed field name). Absent on a
    /// response that reports no cache split -- `None` in that case (D-03),
    /// never a fabricated `Some(0)`.
    #[serde(default)]
    prompt_cache_hit_tokens: Option<u32>,
    /// `completion_tokens_details.reasoning_tokens` (D-20). RESEARCH.md
    /// flags this field name as **assumed, not confirmed** against a live
    /// fixture; because it is optional behind `#[serde(default)]`, a wrong
    /// name degrades to `None` rather than to a wrong value. A later
    /// live-fixture check should confirm the exact wire name before relying
    /// on this figure for pricing.
    #[serde(default)]
    completion_tokens_details: Option<DeepSeekCompletionTokensDetails>,
}

/// `DeepSeekUsage.completion_tokens_details` (D-20; assumed field name, see
/// `DeepSeekUsage.completion_tokens_details`'s own rustdoc).
#[derive(Debug, Deserialize)]
struct DeepSeekCompletionTokensDetails {
    #[serde(default)]
    reasoning_tokens: Option<u32>,
}

// ── DeepSeek streaming response structures ───────────────────────────────────

#[derive(Debug, Deserialize)]
struct DeepSeekStreamResponse {
    /// `#[serde(default)]` because the trailing empty-`choices` usage frame
    /// (D-14/D-15) is not guaranteed to repeat the stream's `id`.
    #[serde(rename = "id", default)]
    #[allow(dead_code)]
    _id: String,
    choices: Vec<DeepSeekStreamChoice>,
    /// Present only on the trailing empty-`choices` frame a
    /// `stream_options: {"include_usage": true}` request elicits (D-14/D-15).
    #[serde(default)]
    usage: Option<DeepSeekUsage>,
}

#[derive(Debug, Deserialize)]
struct DeepSeekStreamChoice {
    delta: DeepSeekStreamDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeepSeekStreamDelta {
    content: Option<String>,
}

/// Detect a completion truncated before any content was produced.
///
/// Reasoning models (e.g. DeepSeek's `-flash`/`-pro` variants) share their
/// `max_tokens` budget between hidden `reasoning_content` and the visible
/// `content`. When the hidden reasoning alone consumes the whole budget, the
/// API returns `content:""` with `finish_reason:"length"` — a truncation that
/// looks, to a naive caller, like a valid-but-empty answer.
///
/// This detection is deliberately narrow: it only fires when `finish_reason`
/// is [`FinishReason::Length`] AND `content` is empty or whitespace-only.
/// A legitimate empty completion with `finish_reason:"stop"`, or any
/// non-empty content (regardless of finish reason), is left untouched.
///
/// Returns `Some(LlmError::EmptyCompletion(..))` when the truncation
/// signature is detected, `None` otherwise.
fn detect_empty_completion(content: &str, finish_reason: &FinishReason) -> Option<LlmError> {
    if matches!(finish_reason, FinishReason::Length) && content.trim().is_empty() {
        Some(LlmError::EmptyCompletion(format!(
            "finish_reason=length with empty content ({} raw chars) — reasoning likely consumed the entire max_tokens budget; retry with a larger max_tokens",
            content.len()
        )))
    } else {
        None
    }
}

/// Map a wire-reported [`DeepSeekUsage`] into [`TokenUsage`], applying the
/// D-20 cache/reasoning sub-count builders only when the payload actually
/// carried the figure (D-03: an absent figure is `None`, never a fabricated
/// `Some(0)`). Shared by both the non-streaming usage-construction site and
/// the streaming terminal-chunk usage, so the two paths cannot drift.
fn map_usage(usage: DeepSeekUsage) -> TokenUsage {
    let mut mapped = TokenUsage::new(usage.prompt_tokens, usage.completion_tokens);
    if let Some(cached) = usage.prompt_cache_hit_tokens {
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

/// Annotate an [`LlmError::EmptyCompletion`] with the provider's own reported
/// `usage` so a caller's error message names `prompt_tokens`,
/// `completion_tokens`, and `total_tokens` instead of discarding them.
///
/// A reasoning model that returns `finish_reason=length` with empty content
/// is ambiguous between two distinct failure modes: "the prompt itself
/// consumed the whole context window" and "reasoning genuinely overran the
/// completion budget on a well-sized prompt". `prompt_tokens` is the number
/// that distinguishes them — before this function existed, `api_response.usage`
/// was deserialized and then thrown away on exactly the failure path where it
/// mattered (`detect_empty_completion` returns its error, at the call site
/// below, BEFORE the `LlmResponse` carrying `usage` is ever constructed).
///
/// Deliberately additive and narrow: every non-`EmptyCompletion` variant
/// passes through byte-identical (no reconstruction), and this does NOT
/// change [`detect_empty_completion`]'s own signature — five existing tests
/// call it with two arguments and are left untouched.
///
/// Recording `prompt_tokens`/`completion_tokens`/`total_tokens` here is a
/// strictly smaller, separately-scoped change from full [`TokenUsage`]
/// mapping — see [`map_usage`], which is now responsible for the full
/// D-20 cache/reasoning sub-count split (including
/// `completion_tokens_details.reasoning_tokens`).
fn annotate_with_usage(err: LlmError, usage: &DeepSeekUsage) -> LlmError {
    match err {
        LlmError::EmptyCompletion(msg) => LlmError::EmptyCompletion(format!(
            "{msg} (provider-reported usage: prompt_tokens={}, completion_tokens={}, \
             total_tokens={})",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
        )),
        other => other,
    }
}

/// Deserialize a possibly-`null` (or absent) string field as an empty string.
///
/// DeepSeek's reasoning models split their `max_tokens` budget between hidden
/// `reasoning_content` and visible `content`. When the hidden reasoning
/// consumes the entire budget, the API may report the empty answer as either
/// `"content": ""` or `"content": null`. The `""` form is classified by
/// [`detect_empty_completion`] into an actionable
/// [`LlmError::EmptyCompletion`]; the `null` form previously aborted
/// DESERIALIZATION — which happens strictly earlier — so that classifier never
/// ran and the truncation surfaced as an opaque body-decode failure instead.
///
/// Normalizing `null` to `""` routes both spellings of the same provider
/// behavior down the one code path already built to handle it. It never
/// invents content: an absent answer stays absent, it just stops being fatal
/// at the wrong layer.
fn deserialize_null_as_empty_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

/// DeepSeek LLM Adapter implementing [`LlmPort`].
///
/// DeepSeek provides OpenAI-compatible API endpoints.
pub struct DeepSeekAdapter {
    client: Client,
    config: DeepSeekConfig,
}

impl DeepSeekAdapter {
    /// Create a new DeepSeek adapter.
    ///
    /// # Errors
    /// Returns error if configuration is invalid or client cannot be created.
    pub fn new(config: DeepSeekConfig) -> Result<Self, LlmError> {
        config.validate().map_err(|e| {
            LlmError::AuthenticationError(format!("Invalid DeepSeek configuration: {}", e))
        })?;

        let timeout = Duration::from_secs(config.timeout_seconds);

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", config.api_key)).map_err(|e| {
                LlmError::AuthenticationError(format!("Invalid API key format: {}", e))
            })?,
        );

        let client = Client::builder()
            .timeout(timeout)
            .default_headers(headers)
            // CR-02 (`25-REVIEW.md`): `DeepSeekConfig::base_url` is
            // operator-configurable and every request carries the
            // `Authorization: Bearer` default header set above. Refusing
            // redirects means a `3xx` from whatever host it resolves to can
            // never replay that header to a different, attacker-influenced
            // host — matches every `CompatEngine`-based preset and the
            // bespoke Gemini adapter (T-17-18/T-17-52). A refused redirect
            // surfaces via [`Self::map_error`]'s `300..=399` arm.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| LlmError::NetworkError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { client, config })
    }

    /// Build DeepSeek API request from LlmRequest.
    fn build_request(&self, request: &LlmRequest) -> Result<DeepSeekRequest, LlmError> {
        let messages = self.convert_prompt_to_messages(&request.prompt)?;
        let params = &request.prompt.node.node.parameters;

        // D-28: any `ResponseFormat` variant degrades to DeepSeek's plain
        // JSON-object form — see `DeepSeekResponseFormat`'s rustdoc.
        let response_format =
            request
                .response_format
                .as_ref()
                .map(|_: &ResponseFormat| DeepSeekResponseFormat {
                    kind: "json_object",
                });

        Ok(DeepSeekRequest {
            model: request.model.clone(),
            messages,
            temperature: params.temperature,
            max_tokens: params.max_tokens,
            top_p: params.top_p,
            frequency_penalty: params.frequency_penalty,
            presence_penalty: params.presence_penalty,
            stream: request.stream,
            response_format,
            // Set unconditionally to `None` here; `generate_stream` forces
            // it to `Some` right after building this request, mirroring the
            // existing defensive `api_request.stream = true` override below
            // (D-15).
            stream_options: None,
        })
    }

    /// Convert PromptItem to DeepSeek messages.
    fn convert_prompt_to_messages(
        &self,
        prompt: &PromptItem,
    ) -> Result<Vec<DeepSeekMessage>, LlmError> {
        let mut messages = Vec::new();

        match &prompt.node.node.prompt_type {
            PromptType::System(system_prompt) => {
                messages.push(DeepSeekMessage {
                    role: "system".to_string(),
                    content: system_prompt.instructions.clone(),
                    reasoning_content: None,
                });
            }
            PromptType::User(user_prompt) => {
                messages.push(DeepSeekMessage {
                    role: "user".to_string(),
                    content: user_prompt.query.clone(),
                    reasoning_content: None,
                });
            }
            PromptType::Text(text_prompt) => {
                messages.push(DeepSeekMessage {
                    role: match text_prompt.role {
                        paladin_core::platform::container::prompt::PromptRole::System => "system",
                        paladin_core::platform::container::prompt::PromptRole::User => "user",
                        paladin_core::platform::container::prompt::PromptRole::Assistant => {
                            "assistant"
                        }
                        paladin_core::platform::container::prompt::PromptRole::Function => {
                            "function"
                        }
                    }
                    .to_string(),
                    content: text_prompt.content.clone(),
                    reasoning_content: None,
                });
            }
            PromptType::Assistant(assistant_prompt) => {
                messages.push(DeepSeekMessage {
                    role: "assistant".to_string(),
                    content: assistant_prompt.response.clone(),
                    reasoning_content: None,
                });
            }
            PromptType::Function(function_prompt) => {
                messages.push(DeepSeekMessage {
                    role: "function".to_string(),
                    content: function_prompt.function_name.clone(),
                    reasoning_content: None,
                });
            }
        }

        if messages.is_empty() {
            return Err(LlmError::InvalidPrompt(
                "Prompt must contain at least one message".to_string(),
            ));
        }

        Ok(messages)
    }

    /// Map DeepSeek finish reason to our FinishReason enum.
    fn map_finish_reason(reason: Option<String>) -> FinishReason {
        match reason.as_deref() {
            Some("stop") => FinishReason::Stop,
            Some("length") => FinishReason::Length,
            Some("content_filter") => FinishReason::ContentFilter,
            Some("function_call") => FinishReason::FunctionCall,
            Some(other) => FinishReason::Error(format!("Unknown finish reason: {}", other)),
            None => FinishReason::Stop,
        }
    }

    /// Render untrusted provider text as a log-safe diagnostic excerpt:
    /// credentials stripped first, then bounded to
    /// [`RESPONSE_EXCERPT_CHAR_BUDGET`] characters.
    ///
    /// The ordering is load-bearing — truncating first could slice a secret
    /// in half and leak the surviving prefix.
    fn diagnostic_excerpt(&self, body: &str) -> String {
        let redacted = redact_credentials(body, &self.config.api_key);
        bounded_excerpt(&redacted, RESPONSE_EXCERPT_CHAR_BUDGET)
    }

    /// Map a non-2xx DeepSeek response to [`LlmError`].
    ///
    /// `300..=399` is named explicitly (CR-02, mirroring
    /// `CompatEngine::map_error`/`GeminiAdapter::map_error`) because this
    /// client's redirect policy is `none` (see [`Self::new`]), so a `3xx`
    /// response is never followed — it arrives here as an ordinary
    /// non-success status instead. Everything else delegates wholesale to
    /// the crate-wide [`map_http_status`] (Phase 25 D-03, FT-FR-01): `body`
    /// is the RAW response text, redacted and bounded once inside the
    /// helper — never pre-excerpted here, which would bound twice.
    /// DeepSeek's documented insufficient-balance status is 402; the
    /// helper's 402 arm carries `regain_hint: None` because DeepSeek's 402
    /// body shape is not first-party-confirmed (Phase 41 RESEARCH
    /// Assumption A1), so there is no prose to parse yet.
    fn map_error(&self, status: u16, body: &str) -> LlmError {
        match status {
            300..=399 => LlmError::ProviderError {
                provider: DEEPSEEK_PROVIDER.to_string(),
                status,
                message: format!(
                    "the configured base URL responded with a redirect (HTTP {status}), which \
                     this client refuses to follow because doing so would forward the \
                     credential header to a different, potentially attacker-influenced host. \
                     Correct the configured base-URL setting to point directly at the intended \
                     endpoint. Response excerpt: {}",
                    self.diagnostic_excerpt(body)
                ),
            },
            _ => map_http_status(DEEPSEEK_PROVIDER, status, body, &self.config.api_key),
        }
    }

    /// Perform API call with retry logic.
    ///
    /// Two deliberate non-goals, recorded here rather than silently:
    ///
    /// - **Retryable-SET parity with `anthropic/adapter.rs::execute_with_retry`,
    ///   not attempt-COUNT parity (RESEARCH Pitfall #6 / Open Question #1,
    ///   planner's call):** this loop's `for attempt in 0..=max_retries` makes
    ///   up to **4** total calls for `max_retries = 3`, while Anthropic's
    ///   `attempt >= max_retries` check makes up to **3**. D-02 asks for
    ///   parity of *which* errors are retried, and Phase 41 deliberately does
    ///   NOT normalize the counter convention — changing DeepSeek's attempt
    ///   count would alter the latency envelope of every specialist in the
    ///   same change that fixes the retryable set, making a live regression
    ///   impossible to attribute. A future phase may unify both loops onto
    ///   one shared helper.
    /// - The two adapters' retryable SETS must stay in lockstep: changing one
    ///   without the other is the exact bug this change fixed (D-02). Today
    ///   both retry `NetworkError | Timeout | ProcessingError |
    ///   RateLimitExceeded | ModelNotAvailable | TokenLimitExceeded` and
    ///   never retry `AuthenticationError | InvalidPrompt | EmptyCompletion |
    ///   UsageLimitExceeded`.
    async fn call_api_with_retry<F, Fut, T>(
        &self,
        operation: F,
        max_retries: u32,
    ) -> Result<T, LlmError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, LlmError>>,
    {
        let mut last_error: Option<LlmError> = None;

        for attempt in 0..=max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    // `EmptyCompletion` is deliberately non-retryable: a
                    // byte-identical retry reproduces the same truncation.
                    // `AuthenticationError`/`InvalidPrompt` need operator
                    // intervention, not a retry. `UsageLimitExceeded` will
                    // not clear on backoff — it resets on a provider-side
                    // billing schedule, not a short window; a per-provider
                    // breaker (D-06, downstream) decides whether to attempt
                    // the call at all, and retrying here would burn retries
                    // before the breaker ever sees the error.
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

                    let backoff = Duration::from_millis(100 * 2_u64.pow(attempt));
                    let jitter = Duration::from_millis(rand::random::<u64>() % 100);
                    tokio::time::sleep(backoff + jitter).await;
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            LlmError::ProcessingError("Retry logic failed unexpectedly".to_string())
        }))
    }
}

#[async_trait]
impl LlmPort for DeepSeekAdapter {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let api_request = self.build_request(&request)?;
        let url = format!("{}/chat/completions", self.config.base_url);

        let operation = || async {
            let response = self
                .client
                .post(&url)
                .json(&api_request)
                .send()
                .await
                .map_err(|e| {
                    if e.is_timeout() {
                        LlmError::Timeout(format!(
                            "DeepSeek API request timed out after {} seconds",
                            self.config.timeout_seconds
                        ))
                    } else {
                        LlmError::NetworkError(format!("Failed to send request to DeepSeek: {}", e))
                    }
                })?;

            let status = response.status();

            if !status.is_success() {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                return Err(self.map_error(status.as_u16(), &error_text));
            }

            // Read the body to text FIRST, then deserialize it separately.
            //
            // `Response::json()` collapses two unrelated failures into one
            // indistinguishable error: reqwest maps BOTH a body-read failure
            // and a serde failure to `Kind::Decode`, whose `Display` is the
            // constant string "error decoding response body". Formatting that
            // with `{}` discards the source chain, so a client timeout that
            // fires mid-body and a response-shape mismatch produce byte-identical
            // log lines — and the body that would tell them apart is dropped.
            // That ambiguity is what made this failure un-diagnosable across two
            // live runs. Splitting the two steps is the same fix the sibling
            // Anthropic adapter already carries.
            let body = response.text().await.map_err(|e| {
                if e.is_timeout() {
                    LlmError::Timeout(format!(
                        "DeepSeek response body did not finish streaming within the {}s client \
                         timeout — note this timeout covers body streaming, not just the \
                         response headers, so a slow long completion trips it after a \
                         successful status line. Raise this stage's timeout or lower its \
                         max_tokens. Underlying error: {e}",
                        self.config.timeout_seconds
                    ))
                } else {
                    LlmError::NetworkError(format!("Failed to read DeepSeek response body: {e}"))
                }
            })?;

            let api_response: DeepSeekResponse = serde_json::from_str(&body).map_err(|e| {
                LlmError::ProcessingError(format!(
                    "Failed to parse DeepSeek response (likely schema drift — see the \
                     null-`content` reasoning precedent in this adapter's tests): {e} — \
                     body excerpt: {}",
                    self.diagnostic_excerpt(&body)
                ))
            })?;

            let choice = api_response.choices.first().ok_or_else(|| {
                LlmError::ProcessingError("DeepSeek response contained no choices".to_string())
            })?;

            let finish_reason = Self::map_finish_reason(choice.finish_reason.clone());

            if let Some(err) = detect_empty_completion(&choice.message.content, &finish_reason) {
                return Err(annotate_with_usage(err, &api_response.usage));
            }

            let content = choice.message.content.clone();
            let usage = map_usage(api_response.usage);

            Ok(LlmResponse {
                id: Uuid::new_v4(),
                request_id: request.id,
                model: api_response.model,
                content,
                finish_reason,
                usage,
                cost: None,
                created_at: Utc::now(),
                metadata: HashMap::new(),
                function_call: None,
            })
        };

        self.call_api_with_retry(operation, 3).await
    }

    async fn generate_stream(
        &self,
        request: LlmRequest,
    ) -> Result<Box<dyn Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError> {
        let mut api_request = self.build_request(&request)?;
        api_request.stream = true;
        api_request.stream_options = Some(DeepSeekStreamOptions {
            include_usage: true,
        });

        let url = format!("{}/chat/completions", self.config.base_url);

        let response = self
            .client
            .post(&url)
            .json(&api_request)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    LlmError::Timeout(format!(
                        "DeepSeek API request timed out after {} seconds",
                        self.config.timeout_seconds
                    ))
                } else {
                    LlmError::NetworkError(format!(
                        "Failed to send streaming request to DeepSeek: {}",
                        e
                    ))
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(self.map_error(status.as_u16(), &error_text));
        }

        let stream = response.bytes_stream();

        // `flat_map` rather than `map` (D-14/plan 31-03 Task 1): a single
        // network chunk can carry more than one complete SSE `data: {...}`
        // event, so every `data:` line found is emitted as its own stream
        // item, mirroring `CompatEngine::generate_stream`.
        //
        // Hold-and-emit terminal-chunk contract: the `finish_reason` frame
        // and the trailing empty-`choices` usage frame can arrive on
        // separate SSE frames, both strictly before `[DONE]`. Both are held
        // in state captured by this `move` closure (`Stream::flat_map`'s
        // closure is `FnMut`, so ordinary mutable locals persist correctly
        // across calls) and emitted together on the ONE `[DONE]` terminal
        // chunk.
        let mut held_finish_reason: Option<FinishReason> = None;
        let mut held_usage: Option<TokenUsage> = None;

        let llm_stream = stream.flat_map(move |chunk_result| {
            let items: Vec<Result<StreamingResponse, LlmError>> = match chunk_result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes).into_owned();
                    let mut items = Vec::new();

                    for line in text.lines() {
                        let Some(json_str) = line.strip_prefix("data: ") else {
                            continue;
                        };

                        if json_str.trim() == "[DONE]" {
                            let mut terminal = StreamingResponse::terminal(
                                held_finish_reason.take().unwrap_or(FinishReason::Stop),
                            );
                            if let Some(usage) = held_usage.take() {
                                terminal = terminal.with_usage(usage);
                            }
                            items.push(Ok(terminal));
                            continue;
                        }

                        match serde_json::from_str::<DeepSeekStreamResponse>(json_str) {
                            Ok(response) => {
                                if let Some(usage) = response.usage {
                                    held_usage = Some(map_usage(usage));
                                }
                                if let Some(choice) = response.choices.first() {
                                    if let Some(reason) = &choice.finish_reason {
                                        held_finish_reason =
                                            Some(Self::map_finish_reason(Some(reason.clone())));
                                    }
                                    let content = choice.delta.content.clone().unwrap_or_default();
                                    items.push(Ok(StreamingResponse::delta(content)));
                                }
                            }
                            Err(e) => {
                                items.push(Err(LlmError::ProcessingError(format!(
                                    "Failed to parse streaming response: {}",
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

        Ok(Box::new(llm_stream))
    }

    async fn validate_model(&self, model: &str) -> Result<bool, LlmError> {
        let available_models = self.get_available_models().await?;
        Ok(available_models.contains(&model.to_string()))
    }

    async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
        Ok(vec![
            "deepseek-chat".to_string(),
            "deepseek-coder".to_string(),
        ])
    }

    fn get_provider_name(&self) -> &'static str {
        DEEPSEEK_PROVIDER
    }

    fn get_capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_streaming: true,
            supports_tool_calling: false,
            supports_function_calling: false,
            supports_vision: false,
            supports_embeddings: false,
            max_context_tokens: Some(64000),
            supports_system_messages: true,
            temperature_range: Some((0.0, 2.0)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn test_deepseek_config_validation() {
        let config = DeepSeekConfig::new(
            "test-key".to_string(),
            "https://api.deepseek.com/v1".to_string(),
            "deepseek-chat".to_string(),
        );
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_deepseek_config_empty_api_key() {
        let config = DeepSeekConfig::new(
            "".to_string(),
            "https://api.deepseek.com/v1".to_string(),
            "deepseek-chat".to_string(),
        );
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_deepseek_config_invalid_url() {
        let config = DeepSeekConfig::new(
            "test-key".to_string(),
            "invalid-url".to_string(),
            "deepseek-chat".to_string(),
        );
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_deepseek_adapter_creation() {
        let config = DeepSeekConfig::new(
            "test-key".to_string(),
            "https://api.deepseek.com/v1".to_string(),
            "deepseek-chat".to_string(),
        );
        let adapter = DeepSeekAdapter::new(config);
        assert!(adapter.is_ok());
    }

    #[tokio::test]
    async fn test_deepseek_provider_capabilities() {
        let config = DeepSeekConfig::new(
            "test-key".to_string(),
            "https://api.deepseek.com/v1".to_string(),
            "deepseek-chat".to_string(),
        );
        let adapter = DeepSeekAdapter::new(config).unwrap();
        let capabilities = adapter.get_capabilities();

        assert!(capabilities.supports_streaming);
        assert!(!capabilities.supports_tool_calling);
        assert!(!capabilities.supports_vision);
        assert!(capabilities.supports_system_messages);
        assert_eq!(capabilities.max_context_tokens, Some(64000));
        assert_eq!(capabilities.temperature_range, Some((0.0, 2.0)));
        assert_eq!(adapter.get_provider_name(), "deepseek");
    }

    #[test]
    fn test_detect_empty_completion_length_and_empty_is_truncation() {
        let result = detect_empty_completion("", &FinishReason::Length);
        assert!(matches!(result, Some(LlmError::EmptyCompletion(_))));
    }

    #[test]
    fn test_detect_empty_completion_length_and_whitespace_is_truncation() {
        let result = detect_empty_completion("   \n\t", &FinishReason::Length);
        assert!(matches!(result, Some(LlmError::EmptyCompletion(_))));
    }

    #[test]
    fn test_detect_empty_completion_non_empty_content_is_never_truncation() {
        // Non-empty content is not a truncation regardless of finish_reason.
        assert!(detect_empty_completion("some answer", &FinishReason::Length).is_none());
        assert!(detect_empty_completion("some answer", &FinishReason::Stop).is_none());
    }

    #[test]
    fn test_detect_empty_completion_empty_but_stop_is_not_truncation() {
        // A legitimate empty completion with finish_reason=stop is NOT a truncation
        // — detection is narrow to Length+empty only.
        let result = detect_empty_completion("", &FinishReason::Stop);
        assert!(result.is_none());
    }

    #[test]
    fn test_annotate_with_usage_names_all_three_token_counts_on_empty_completion() {
        let err = LlmError::EmptyCompletion("finish_reason=length with empty content".to_string());
        let usage = DeepSeekUsage {
            prompt_tokens: 31_000,
            completion_tokens: 32_000,
            total_tokens: 63_000,
            prompt_cache_hit_tokens: None,
            completion_tokens_details: None,
        };

        let annotated = annotate_with_usage(err, &usage);

        match annotated {
            LlmError::EmptyCompletion(msg) => {
                assert!(
                    msg.contains("finish_reason=length with empty content"),
                    "original message must survive, got: {msg}"
                );
                assert!(msg.contains("31000"), "must name prompt_tokens, got: {msg}");
                assert!(
                    msg.contains("32000"),
                    "must name completion_tokens, got: {msg}"
                );
                assert!(msg.contains("63000"), "must name total_tokens, got: {msg}");
            }
            other => {
                panic!("expected EmptyCompletion to round-trip as EmptyCompletion, got {other:?}")
            }
        }
    }

    #[test]
    fn test_annotate_with_usage_passes_through_non_empty_completion_variants_unchanged() {
        let err = LlmError::Timeout("request timed out".to_string());
        let usage = DeepSeekUsage {
            prompt_tokens: 1,
            completion_tokens: 2,
            total_tokens: 3,
            prompt_cache_hit_tokens: None,
            completion_tokens_details: None,
        };

        let annotated = annotate_with_usage(err, &usage);

        match annotated {
            LlmError::Timeout(msg) => assert_eq!(msg, "request timed out"),
            other => panic!("expected Timeout to round-trip unchanged, got {other:?}"),
        }
    }

    #[test]
    fn test_deepseek_message_deserializes_with_reasoning_content() {
        let json =
            r#"{"role":"assistant","content":"","reasoning_content":"thinking really hard..."}"#;
        let message: DeepSeekMessage = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(message.content, "");
        assert_eq!(
            message.reasoning_content.as_deref(),
            Some("thinking really hard...")
        );
    }

    #[test]
    fn test_deepseek_message_deserializes_without_reasoning_content() {
        let json = r#"{"role":"assistant","content":"the answer"}"#;
        let message: DeepSeekMessage = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(message.content, "the answer");
        assert_eq!(message.reasoning_content, None);
    }

    /// RED probe (debug session `deepseek-body-decode-persist`): a reasoning
    /// model that spends its whole `max_tokens` budget on hidden reasoning can
    /// return `"content": null` rather than `"content": ""`. The `""` form is
    /// handled by `detect_empty_completion`; the `null` form must not blow up
    /// in DESERIALIZATION, because that happens strictly before
    /// `detect_empty_completion` ever runs.
    #[test]
    fn deepseek_response_deserializes_reasoning_payload_with_null_content() {
        let json = r#"{
            "id": "chatcmpl-abc123",
            "model": "deepseek-v4-flash",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "reasoning_content": "long hidden chain of thought..."
                },
                "finish_reason": "length"
            }],
            "usage": {"prompt_tokens": 41000, "completion_tokens": 24000, "total_tokens": 65000}
        }"#;

        let response: DeepSeekResponse =
            serde_json::from_str(json).expect("a null content field must not fail deserialization");
        let choice = response
            .choices
            .first()
            .expect("payload declares one choice");
        assert_eq!(
            choice.message.content, "",
            "null content must normalize to empty string so detect_empty_completion can \
             classify it as a truncation"
        );
    }

    // ── Debug `deepseek-body-decode-persist`: body-decode diagnosability ──

    #[test]
    fn deepseek_message_deserializes_absent_content_as_empty_string() {
        // A missing `content` key must be as survivable as an explicit null.
        let json = r#"{"role":"assistant","reasoning_content":"thinking..."}"#;
        let message: DeepSeekMessage = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(message.content, "");
    }

    #[test]
    fn null_content_truncation_is_classified_as_empty_completion_not_a_decode_failure() {
        // The whole point of tolerating null: it lets the EXISTING truncation
        // classifier see the response, which hands the caller an actionable
        // "raise max_tokens" error instead of an opaque decode failure.
        let json = r#"{"role":"assistant","content":null}"#;
        let message: DeepSeekMessage = serde_json::from_str(json).expect("should deserialize");

        let detected = detect_empty_completion(&message.content, &FinishReason::Length);
        assert!(matches!(detected, Some(LlmError::EmptyCompletion(_))));
    }

    #[test]
    fn deepseek_message_still_serializes_content_as_a_plain_string() {
        // The null-tolerant deserializer must not leak into the REQUEST shape.
        let message = DeepSeekMessage {
            role: "user".to_string(),
            content: "hello".to_string(),
            reasoning_content: None,
        };
        let json = serde_json::to_string(&message).expect("should serialize");
        assert!(json.contains(r#""content":"hello""#), "got {json}");
        assert!(!json.contains("null"), "got {json}");
    }

    #[test]
    fn bounded_excerpt_returns_input_unchanged_when_shorter_than_budget() {
        let body = r#"{"error":"short"}"#;
        assert_eq!(bounded_excerpt(body, RESPONSE_EXCERPT_CHAR_BUDGET), body);
    }

    #[test]
    fn bounded_excerpt_is_char_boundary_safe_on_multibyte_input() {
        // Byte-slicing this would panic mid-character; char-count truncation
        // must not. A production body is full of multi-byte text.
        let body = "\u{1F5E1}\u{FE0F}\u{2694}\u{FE0F}".repeat(64);
        let budget = 5;
        let excerpt = bounded_excerpt(&body, budget);

        assert!(excerpt.starts_with("\u{1F5E1}"));
        assert!(excerpt.contains("[truncated,"));
        assert_eq!(
            excerpt.chars().take(budget).count(),
            budget,
            "must keep exactly `budget` characters before the elision marker"
        );
    }

    #[test]
    fn diagnostic_excerpt_never_echoes_the_configured_api_key() {
        // The constraint that motivated this test: a captured body excerpt is
        // written straight to an operator-facing log line, so it must never
        // carry a credential — asserted, not assumed.
        let secret = "sk-livekey-abcdef0123456789";
        let config = DeepSeekConfig::new(
            secret.to_string(),
            "https://api.deepseek.com/v1".to_string(),
            "deepseek-chat".to_string(),
        );
        let adapter = DeepSeekAdapter::new(config).expect("valid config");

        // A gateway echoing the whole request back, headers included.
        let echoed = format!(
            r#"{{"error":"bad gateway","request":{{"headers":{{"authorization":"Bearer {secret}"}}}}}}"#
        );
        let excerpt = adapter.diagnostic_excerpt(&echoed);

        assert!(
            !excerpt.contains(secret),
            "excerpt leaked the API key: {excerpt}"
        );
        assert!(
            !excerpt.contains("livekey"),
            "excerpt leaked part of the API key: {excerpt}"
        );
        assert!(
            excerpt.contains(CREDENTIAL_PLACEHOLDER),
            "excerpt should show the redaction happened: {excerpt}"
        );
        // The surrounding diagnostic context must survive redaction.
        assert!(excerpt.contains("bad gateway"), "got {excerpt}");
    }

    #[test]
    fn redact_credentials_masks_bearer_and_sk_tokens_it_was_not_configured_with() {
        // Defense in depth: a key OTHER than this adapter's own (e.g. an
        // upstream proxy's) must still be masked by shape.
        let body = r#"{"msg":"denied","auth":"Bearer sk-someoneelses-9876543210"}"#;
        let redacted = redact_credentials(body, "");

        assert!(!redacted.contains("9876543210"), "got {redacted}");
        assert!(redacted.contains(CREDENTIAL_PLACEHOLDER), "got {redacted}");
        assert!(redacted.contains("denied"), "got {redacted}");
    }

    #[test]
    fn redact_credentials_leaves_credential_free_bodies_untouched() {
        let body = r#"{"id":"chatcmpl-1","choices":[{"index":0}]}"#;
        assert_eq!(redact_credentials(body, "sk-not-present"), body);
    }

    #[test]
    fn parse_failure_message_carries_serde_detail_and_a_body_excerpt() {
        // The regression guard for this whole debug session: the message must
        // no longer be the information-free constant reqwest hands back.
        let config = DeepSeekConfig::new(
            "sk-test-key".to_string(),
            "https://api.deepseek.com/v1".to_string(),
            "deepseek-chat".to_string(),
        );
        let adapter = DeepSeekAdapter::new(config).expect("valid config");

        // An HTTP-200 error object — a shape `DeepSeekResponse` cannot accept.
        let body = r#"{"error":{"message":"model overloaded","type":"server_error"}}"#;
        let parse_error = serde_json::from_str::<DeepSeekResponse>(body)
            .expect_err("this body must not deserialize into DeepSeekResponse");

        let rendered = format!(
            "Failed to parse DeepSeek response (likely schema drift): {parse_error} — \
             body excerpt: {}",
            adapter.diagnostic_excerpt(body)
        );

        assert!(
            rendered.contains("model overloaded"),
            "the raw body must survive into the error: {rendered}"
        );
        assert!(
            rendered.contains("missing field"),
            "serde's own diagnosis must survive into the error: {rendered}"
        );
        assert_ne!(
            rendered, LIVE_BODY_DECODE_ERROR,
            "must not collapse back to the information-free constant"
        );
    }

    // ── Task 41-01/3: DeepSeek retryable-set parity + the 402 arm (D-02) ──

    fn test_adapter() -> DeepSeekAdapter {
        let config = DeepSeekConfig::new(
            "test-key".to_string(),
            "https://api.deepseek.com/v1".to_string(),
            "deepseek-chat".to_string(),
        );
        DeepSeekAdapter::new(config).expect("test config must build a valid adapter")
    }

    /// The exact live error string that killed the deductive specialist in
    /// run `4a3b749d` — already used as a fixture at
    /// `crates/audit-agents/src/deductive.rs:1539` and
    /// `crates/audit-agents/src/fuzz.rs:2912` in the downstream superproject.
    const LIVE_BODY_DECODE_ERROR: &str =
        "Failed to parse DeepSeek response: error decoding response body";

    // ── Phase 25 (FT-FR-01, D-03): non-2xx routes through map_http_status ──

    #[test]
    fn deepseek_non_2xx_routes_through_the_shared_mapper() {
        let adapter = test_adapter();
        match adapter.map_error(503, r#"{"error":{"message":"overloaded"}}"#) {
            LlmError::ProviderError {
                provider, status, ..
            } => {
                assert_eq!(provider, "deepseek");
                assert_eq!(status, 503);
            }
            other => panic!("expected ProviderError {{ status: 503 }}, got {other:?}"),
        }
    }

    #[test]
    fn deepseek_dedicated_status_mappings_are_unchanged() {
        let adapter = test_adapter();
        assert!(matches!(
            adapter.map_error(401, "bad key"),
            LlmError::AuthenticationError(_)
        ));
        assert!(matches!(
            adapter.map_error(429, "slow down"),
            LlmError::RateLimitExceeded
        ));
        assert!(matches!(
            adapter.map_error(404, "no model"),
            LlmError::ModelNotAvailable(_)
        ));
        assert!(matches!(
            adapter.map_error(400, "bad prompt"),
            LlmError::InvalidPrompt(_)
        ));
    }

    #[test]
    fn map_error_maps_a_redirect_status_to_an_actionable_provider_error() {
        // CR-02 (`25-REVIEW.md`): named explicitly because this client's
        // redirect policy is `none` (see `DeepSeekAdapter::new`), so a
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
                    assert_eq!(provider, "deepseek");
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
    fn map_error_402_maps_to_usage_limit_exceeded_not_processing_error() {
        let adapter = test_adapter();
        let error = adapter.map_error(402, "Insufficient Balance");

        match error {
            LlmError::UsageLimitExceeded {
                provider,
                regain_hint,
            } => {
                assert_eq!(provider, "deepseek");
                assert_eq!(regain_hint, None);
            }
            other => panic!("expected UsageLimitExceeded, got {other:?}"),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn call_api_with_retry_retries_a_body_decode_processing_error() {
        let adapter = test_adapter();
        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = Arc::clone(&calls);

        let result: Result<(), LlmError> = adapter
            .call_api_with_retry(
                move || {
                    let calls = Arc::clone(&calls_clone);
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Err(LlmError::ProcessingError(
                            LIVE_BODY_DECODE_ERROR.to_string(),
                        ))
                    }
                },
                3,
            )
            .await;

        assert!(result.is_err());
        assert_eq!(
            calls.load(Ordering::SeqCst),
            4,
            "a body-decode ProcessingError must be retried up to (max_retries + 1) attempts \
             — this is the LLMR-01 root cause"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn call_api_with_retry_retries_network_error_and_timeout() {
        let adapter = test_adapter();

        for make_error in [
            || LlmError::NetworkError("connection reset".to_string()),
            || LlmError::Timeout("request timed out".to_string()),
        ] {
            let calls = Arc::new(AtomicU32::new(0));
            let calls_clone = Arc::clone(&calls);

            let result: Result<(), LlmError> = adapter
                .call_api_with_retry(
                    move || {
                        let calls = Arc::clone(&calls_clone);
                        let error = make_error();
                        async move {
                            calls.fetch_add(1, Ordering::SeqCst);
                            Err(error)
                        }
                    },
                    3,
                )
                .await;

            assert!(result.is_err());
            assert_eq!(calls.load(Ordering::SeqCst), 4);
        }
    }

    #[tokio::test(start_paused = true)]
    async fn call_api_with_retry_succeeds_after_one_transient_processing_error() {
        let adapter = test_adapter();
        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = Arc::clone(&calls);

        let result: Result<&'static str, LlmError> = adapter
            .call_api_with_retry(
                move || {
                    let calls = Arc::clone(&calls_clone);
                    async move {
                        let n = calls.fetch_add(1, Ordering::SeqCst);
                        if n == 0 {
                            Err(LlmError::ProcessingError(
                                LIVE_BODY_DECODE_ERROR.to_string(),
                            ))
                        } else {
                            Ok("recovered")
                        }
                    }
                },
                3,
            )
            .await;

        assert!(matches!(result, Ok("recovered")));
        assert_eq!(
            calls.load(Ordering::SeqCst),
            2,
            "must succeed after exactly one transient failure plus one retry"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn call_api_with_retry_invokes_operation_exactly_once_on_empty_completion() {
        let adapter = test_adapter();
        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = Arc::clone(&calls);

        let result: Result<(), LlmError> = adapter
            .call_api_with_retry(
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
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn call_api_with_retry_invokes_operation_exactly_once_on_usage_limit_exceeded() {
        let adapter = test_adapter();
        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = Arc::clone(&calls);

        let result: Result<(), LlmError> = adapter
            .call_api_with_retry(
                move || {
                    let calls = Arc::clone(&calls_clone);
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Err(LlmError::UsageLimitExceeded {
                            provider: "deepseek".to_string(),
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

    #[tokio::test(start_paused = true)]
    async fn call_api_with_retry_still_retries_rate_limit_exceeded() {
        let adapter = test_adapter();
        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = Arc::clone(&calls);

        let result: Result<(), LlmError> = adapter
            .call_api_with_retry(
                move || {
                    let calls = Arc::clone(&calls_clone);
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Err(LlmError::RateLimitExceeded)
                    }
                },
                3,
            )
            .await;

        assert!(result.is_err());
        assert_eq!(
            calls.load(Ordering::SeqCst),
            4,
            "RateLimitExceeded retry behavior must not regress"
        );
    }

    // ── Phase 26 (RT-05, D-28): response_format reaches the wire ──────────

    fn build_response_format_request(response_format: Option<ResponseFormat>) -> LlmRequest {
        use paladin_core::platform::container::prompt::UserPrompt;

        let request = LlmRequest::new(
            "deepseek-chat",
            PromptItem::new(PromptType::User(UserPrompt {
                query: "Hello".to_string(),
                context: None,
            }))
            .unwrap(),
        );
        match response_format {
            Some(format) => request.with_response_format(format),
            None => request,
        }
    }

    #[test]
    fn deepseek_request_carries_json_object_response_format() {
        let adapter = test_adapter();
        let request = build_response_format_request(Some(ResponseFormat::JsonObject));

        let api_request = adapter.build_request(&request).unwrap();
        let body = serde_json::to_value(&api_request).unwrap();

        assert_eq!(
            body.get("response_format"),
            Some(&serde_json::json!({"type": "json_object"}))
        );
    }

    #[test]
    fn deepseek_json_schema_degrades_to_json_object_response_format() {
        let adapter = test_adapter();
        let request = build_response_format_request(Some(ResponseFormat::JsonSchema {
            name: "answer".to_string(),
            schema: serde_json::json!({"type": "object"}),
            strict: true,
        }));

        let api_request = adapter.build_request(&request).unwrap();
        let body = serde_json::to_value(&api_request).unwrap();

        assert_eq!(
            body.get("response_format"),
            Some(&serde_json::json!({"type": "json_object"})),
            "DeepSeek has no schema-carrying native mode -- every ResponseFormat \
             variant degrades to the plain JSON-object form (D-28)"
        );
    }

    #[test]
    fn deepseek_request_without_response_format_is_unchanged() {
        let adapter = test_adapter();
        let request = build_response_format_request(None);

        let api_request = adapter.build_request(&request).unwrap();
        let body = serde_json::to_value(&api_request).unwrap();

        assert!(
            body.as_object().unwrap().get("response_format").is_none(),
            "absent response_format must not appear on the wire, got: {body:?}"
        );
    }

    // ── Phase 31 (D-13, D-14, D-15, D-20): streaming usage terminal-chunk
    //    contract, and cache/reasoning sub-count mapping on both paths ────

    fn test_adapter_at(base_url: &str) -> DeepSeekAdapter {
        let config = DeepSeekConfig::new(
            "test-key".to_string(),
            base_url.to_string(),
            "deepseek-chat".to_string(),
        );
        DeepSeekAdapter::new(config).expect("test config must build a valid adapter")
    }

    /// Direct unit test on the shared mapping function -- no network needed
    /// (matches this file's own pre-existing style: `build_request`/
    /// `map_error`/`annotate_with_usage` are all tested this way, never
    /// through a real HTTP round trip). Distinct, non-round figures so a
    /// swapped or dropped field cannot pass by coincidence.
    #[test]
    fn map_usage_maps_cache_hit_and_reasoning_when_the_payload_carries_them() {
        let usage = DeepSeekUsage {
            prompt_tokens: 800,
            completion_tokens: 900,
            total_tokens: 1700,
            prompt_cache_hit_tokens: Some(64),
            completion_tokens_details: Some(DeepSeekCompletionTokensDetails {
                reasoning_tokens: Some(320),
            }),
        };

        let mapped = map_usage(usage);

        assert_eq!(mapped.cache_read_tokens, Some(64));
        assert_eq!(mapped.reasoning_tokens, Some(320));
        assert_eq!(mapped.cache_write_tokens, None);
    }

    #[test]
    fn map_usage_leaves_optionals_none_when_the_payload_omits_them() {
        let usage = DeepSeekUsage {
            prompt_tokens: 1,
            completion_tokens: 1,
            total_tokens: 2,
            prompt_cache_hit_tokens: None,
            completion_tokens_details: None,
        };

        let mapped = map_usage(usage);

        assert_eq!(mapped.cache_read_tokens, None);
        assert_eq!(mapped.cache_write_tokens, None);
        assert_eq!(mapped.reasoning_tokens, None);
    }

    #[tokio::test]
    async fn generate_stream_request_carries_stream_options_include_usage() {
        let mut server = mockito::Server::new_async().await;
        let captured: Arc<std::sync::Mutex<Option<String>>> = Arc::new(std::sync::Mutex::new(None));
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

        let adapter = test_adapter_at(&server.url());
        let request = build_response_format_request(None);
        let stream = adapter.generate_stream(request).await.unwrap();
        let mut stream = Box::into_pin(stream);
        while stream.next().await.is_some() {}

        let body_text = captured
            .lock()
            .unwrap()
            .clone()
            .expect("request must have been captured");
        let body: serde_json::Value = serde_json::from_str(&body_text).unwrap();
        assert_eq!(
            body.get("stream_options"),
            Some(&serde_json::json!({"include_usage": true}))
        );
    }

    #[tokio::test]
    async fn generate_stream_holds_finish_reason_and_usage_for_the_one_terminal_chunk() {
        let mut server = mockito::Server::new_async().await;
        let usage_json = serde_json::json!({
            "prompt_tokens": 800,
            "completion_tokens": 900,
            "total_tokens": 1700,
            "prompt_cache_hit_tokens": 64
        });
        let sse_body = format!(
            "data: {{\"id\":\"1\",\"choices\":[{{\"delta\":{{\"content\":\"Hel\"}},\"finish_reason\":null}}]}}\n\n\
             data: {{\"id\":\"1\",\"choices\":[{{\"delta\":{{\"content\":\"lo\"}},\"finish_reason\":null}}]}}\n\n\
             data: {{\"id\":\"1\",\"choices\":[{{\"delta\":{{\"content\":\"\"}},\"finish_reason\":\"stop\"}}]}}\n\n\
             data: {{\"id\":\"1\",\"choices\":[],\"usage\":{}}}\n\n\
             data: [DONE]\n\n",
            usage_json
        );

        server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(sse_body)
            .create_async()
            .await;

        let adapter = test_adapter_at(&server.url());
        let request = build_response_format_request(None);
        let stream = adapter.generate_stream(request).await.unwrap();
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
        assert_eq!(
            chunks[usage_indices[0]]
                .usage
                .as_ref()
                .and_then(|u| u.cache_read_tokens),
            Some(64)
        );

        let assembled: String = chunks.iter().map(|c| c.delta.as_str()).collect();
        assert_eq!(assembled, "Hello");
    }

    // ── Shared conformance suite (D-19, plan 31-04) ──
    //
    // Nested in its own module (rather than inline in `mod tests`) so every generated test's
    // full path contains "conformance" -- `cargo test --lib conformance` (the plan's own
    // acceptance criterion) selects it by that substring.
    mod conformance_suite {
        use super::*;
        use serde_json::json;

        struct DeepSeekFixture;

        impl crate::conformance::ConformanceFixture for DeepSeekFixture {
            const WIRE: crate::conformance::Wire = crate::conformance::Wire::OpenAiChat;

            fn adapter(base_url: &str) -> Arc<dyn LlmPort> {
                let config = DeepSeekConfig::new(
                    "test-key".to_string(),
                    base_url.to_string(),
                    "deepseek-chat".to_string(),
                );
                Arc::new(DeepSeekAdapter::new(config).expect("test config must build"))
            }

            fn success_body() -> String {
                json!({
                    "id": "cmpl-1",
                    "model": "deepseek-chat",
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
                json!({"error": {"message": format!("mock error for status {status}"), "type": "mock_error"}})
                    .to_string()
            }
        }

        crate::llm_conformance_suite!(DeepSeekFixture);
    }
}
