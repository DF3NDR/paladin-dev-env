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
    FinishReason, LlmError, LlmPort, LlmRequest, LlmResponse, ProviderCapabilities,
    StreamingResponse, TokenUsage,
};

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
    /// The reasoning/content split within `completion_tokens`. Absent on
    /// non-reasoning responses and on every pre-existing test fixture, so
    /// `#[serde(default)]` is required, not stylistic.
    #[serde(default)]
    completion_tokens_details: Option<DeepSeekCompletionTokensDetails>,
}

/// The reasoning/content split DeepSeek reports on reasoning-model
/// completions.
///
/// Reasoning models (e.g. the `-flash`/`-pro` variants) split their
/// `max_tokens` budget between hidden `reasoning_content` and visible
/// `content` (see [`deserialize_null_as_empty_string`]). Before this type
/// existed, `completion_tokens` was a single opaque number: a caller could
/// not tell "reasoning consumed the whole budget" from "content generation
/// itself ran long" — diagnosis E7. `reasoning_tokens` is the number that
/// makes that split observable.
#[derive(Debug, Deserialize, Default)]
struct DeepSeekCompletionTokensDetails {
    #[serde(default)]
    reasoning_tokens: Option<u32>,
}

// ── DeepSeek streaming response structures ───────────────────────────────────

#[derive(Debug, Deserialize)]
struct DeepSeekStreamResponse {
    #[serde(rename = "id")]
    #[allow(dead_code)]
    _id: String,
    choices: Vec<DeepSeekStreamChoice>,
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
/// **Reasoning-token observability (diagnosis E7).** When the provider
/// reports `completion_tokens_details.reasoning_tokens`, the annotation
/// names it alongside the existing three counts — this is the number that
/// distinguishes "ran out mid-answer" (reasoning consumed the whole budget,
/// content generation never started) from "never started answering" (a
/// prompt-sizing problem), and without it this failure class stayed
/// permanently indirect to diagnose. The clause is appended ONLY when the
/// provider reports it, so the message stays byte-identical to before on
/// every response that omits it (non-reasoning models, and every
/// pre-existing test fixture).
fn annotate_with_usage(err: LlmError, usage: &DeepSeekUsage) -> LlmError {
    match err {
        LlmError::EmptyCompletion(msg) => {
            let reasoning_clause = usage
                .completion_tokens_details
                .as_ref()
                .and_then(|details| details.reasoning_tokens)
                .map(|reasoning_tokens| format!(", reasoning_tokens={reasoning_tokens}"))
                .unwrap_or_default();

            LlmError::EmptyCompletion(format!(
                "{msg} (provider-reported usage: prompt_tokens={}, completion_tokens={}, \
                 total_tokens={}{reasoning_clause})",
                usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
            ))
        }
        other => other,
    }
}

/// Character budget for a diagnostic excerpt of a response body.
///
/// Mirrors `anthropic::adapter::RESPONSE_EXCERPT_CHAR_BUDGET`, deliberately —
/// the two adapters' diagnostics are meant to stay in lockstep for the same
/// reason their retryable sets are (see [`DeepSeekAdapter::call_api_with_retry`]).
const RESPONSE_EXCERPT_CHAR_BUDGET: usize = 512;

/// What a redacted credential is replaced with in a diagnostic excerpt.
const CREDENTIAL_PLACEHOLDER: &str = "[REDACTED]";

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

/// Build a diagnostic excerpt of a response body, bounded by CHARACTER count
/// rather than byte count.
///
/// Slicing a UTF-8 `&str` by byte offset panics when the offset lands
/// mid-character, and panics are forbidden in this library — a captured
/// production response body is full of multi-byte characters. When `body`
/// exceeds `budget` characters, an ASCII elision marker reports the total byte
/// length of the untruncated body so the reader knows how much was withheld.
fn bounded_excerpt(body: &str, budget: usize) -> String {
    if body.chars().count() <= budget {
        return body.to_string();
    }

    let truncated: String = body.chars().take(budget).collect();
    format!("{truncated}... [truncated, {} total bytes]", body.len())
}

/// Replace the token that follows every occurrence of `marker` with
/// [`CREDENTIAL_PLACEHOLDER`].
///
/// The token is taken to run until the first whitespace or JSON delimiter.
/// `marker` must be ASCII so the byte offsets returned by `find` are always
/// character boundaries; every slice is nonetheless taken through the checked
/// `get` API so this function has no panicking path.
fn redact_token_after(body: &str, marker: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut rest = body;

    while let Some(idx) = rest.find(marker) {
        let cut = idx + marker.len();
        let (head, tail) = match (rest.get(..cut), rest.get(cut..)) {
            (Some(head), Some(tail)) => (head, tail),
            // Unreachable for an ASCII `marker` located by `find`, but this
            // library must never panic: stop scanning and emit the remainder
            // verbatim via the trailing `push_str` below.
            _ => break,
        };

        out.push_str(head);

        let end = tail
            .find(|c: char| c.is_whitespace() || matches!(c, '"' | ',' | '}' | ']' | '\\'))
            .unwrap_or(tail.len());

        if end > 0 {
            out.push_str(CREDENTIAL_PLACEHOLDER);
        }

        rest = tail.get(end..).unwrap_or("");
    }

    out.push_str(rest);
    out
}

/// Strip anything credential-shaped out of text destined for a log line.
///
/// Three passes, in order of precision:
/// 1. the adapter's OWN configured `api_key`, matched exactly — this cannot
///    miss, and covers a gateway that echoes the request back verbatim;
/// 2. `Bearer <token>` / `bearer <token>`, the header form;
/// 3. any surviving `sk-`-prefixed token.
///
/// Redaction MUST run before truncation, otherwise a bounded excerpt could
/// slice a secret in half and leak the surviving prefix.
fn redact_credentials(body: &str, api_key: &str) -> String {
    let exact = if api_key.is_empty() {
        body.to_string()
    } else {
        body.replace(api_key, CREDENTIAL_PLACEHOLDER)
    };

    let no_bearer = redact_token_after(&redact_token_after(&exact, "Bearer "), "bearer ");
    redact_token_after(&no_bearer, "sk-")
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
            .build()
            .map_err(|e| LlmError::NetworkError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { client, config })
    }

    /// Build DeepSeek API request from LlmRequest.
    fn build_request(&self, request: &LlmRequest) -> Result<DeepSeekRequest, LlmError> {
        let messages = self.convert_prompt_to_messages(&request.prompt)?;
        let params = &request.prompt.node.node.parameters;

        Ok(DeepSeekRequest {
            model: request.model.clone(),
            messages,
            temperature: params.temperature,
            max_tokens: params.max_tokens,
            top_p: params.top_p,
            frequency_penalty: params.frequency_penalty,
            presence_penalty: params.presence_penalty,
            stream: request.stream,
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

    /// Map DeepSeek API errors to LlmError.
    fn map_error(&self, status: u16, message: &str) -> LlmError {
        match status {
            401 => LlmError::AuthenticationError(format!(
                "Invalid API key for DeepSeek. Check DEEPSEEK_API_KEY. Error: {}",
                message
            )),
            429 => LlmError::RateLimitExceeded,
            // DeepSeek's documented insufficient-balance/quota-exhausted status
            // is 402. `regain_hint` is `None` because DeepSeek's 402 body
            // shape is not first-party-confirmed (RESEARCH Assumption A1 —
            // corroborated by multiple third-party sources, not by
            // DeepSeek's own API reference), so there is no prose to parse
            // yet. This is D-05's "explicitly-empty, documented branch" —
            // expressed as a real arm with a `None` hint, not as an absence.
            402 => LlmError::UsageLimitExceeded {
                provider: "deepseek".to_string(),
                regain_hint: None,
            },
            404 => LlmError::ModelNotAvailable(message.to_string()),
            400 => LlmError::InvalidPrompt(message.to_string()),
            _ => LlmError::ProcessingError(format!("DeepSeek API error ({}): {}", status, message)),
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
                return Err(self.map_error(status.as_u16(), &self.diagnostic_excerpt(&error_text)));
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

            Ok(LlmResponse {
                id: Uuid::new_v4(),
                request_id: request.id,
                model: api_response.model,
                content: choice.message.content.clone(),
                finish_reason,
                usage: TokenUsage {
                    prompt_tokens: api_response.usage.prompt_tokens,
                    completion_tokens: api_response.usage.completion_tokens,
                    total_tokens: api_response.usage.total_tokens,
                },
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

        let llm_stream = stream.map(|chunk_result| match chunk_result {
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes);

                for line in text.lines() {
                    if let Some(json_str) = line.strip_prefix("data: ") {
                        if json_str.trim() == "[DONE]" {
                            return Ok(StreamingResponse {
                                id: Uuid::new_v4(),
                                delta: String::new(),
                                finish_reason: Some(FinishReason::Stop),
                            });
                        }

                        match serde_json::from_str::<DeepSeekStreamResponse>(json_str) {
                            Ok(response) => {
                                if let Some(choice) = response.choices.first() {
                                    let content = choice.delta.content.clone().unwrap_or_default();
                                    return Ok(StreamingResponse {
                                        id: Uuid::new_v4(),
                                        delta: content,
                                        finish_reason: choice
                                            .finish_reason
                                            .as_ref()
                                            .map(|r| Self::map_finish_reason(Some(r.clone()))),
                                    });
                                }
                            }
                            Err(e) => {
                                return Err(LlmError::ProcessingError(format!(
                                    "Failed to parse streaming response: {}",
                                    e
                                )));
                            }
                        }
                    }
                }

                Ok(StreamingResponse {
                    id: Uuid::new_v4(),
                    delta: String::new(),
                    finish_reason: None,
                })
            }
            Err(e) => Err(LlmError::NetworkError(format!("Stream error: {}", e))),
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
        "deepseek"
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
            completion_tokens_details: None,
        };

        let annotated = annotate_with_usage(err, &usage);

        match annotated {
            LlmError::Timeout(msg) => assert_eq!(msg, "request timed out"),
            other => panic!("expected Timeout to round-trip unchanged, got {other:?}"),
        }
    }

    #[test]
    fn test_annotate_with_usage_names_reasoning_tokens_when_present() {
        let err = LlmError::EmptyCompletion("finish_reason=length with empty content".to_string());
        let usage = DeepSeekUsage {
            prompt_tokens: 31_000,
            completion_tokens: 32_000,
            total_tokens: 63_000,
            completion_tokens_details: Some(DeepSeekCompletionTokensDetails {
                reasoning_tokens: Some(32_000),
            }),
        };

        let annotated = annotate_with_usage(err, &usage);

        match annotated {
            LlmError::EmptyCompletion(msg) => {
                assert!(msg.contains("prompt_tokens=31000"), "got: {msg}");
                assert!(msg.contains("completion_tokens=32000"), "got: {msg}");
                assert!(msg.contains("total_tokens=63000"), "got: {msg}");
                assert!(
                    msg.contains("reasoning_tokens=32000"),
                    "must name the reasoning/content split, got: {msg}"
                );
            }
            other => {
                panic!("expected EmptyCompletion to round-trip as EmptyCompletion, got {other:?}")
            }
        }
    }

    #[test]
    fn test_annotate_with_usage_is_byte_identical_when_completion_tokens_details_is_absent() {
        // No `completion_tokens_details` key at all in the JSON — the field must default
        // rather than fail deserialization (backward compatible with every non-reasoning
        // response and every pre-existing fixture).
        let json = r#"{"prompt_tokens":31000,"completion_tokens":32000,"total_tokens":63000}"#;
        let usage: DeepSeekUsage = serde_json::from_str(json).expect("should deserialize");
        assert!(usage.completion_tokens_details.is_none());

        let err = LlmError::EmptyCompletion("finish_reason=length with empty content".to_string());

        let annotated = annotate_with_usage(err, &usage);

        match annotated {
            LlmError::EmptyCompletion(msg) => {
                assert_eq!(
                    msg,
                    "finish_reason=length with empty content (provider-reported usage: \
                     prompt_tokens=31000, completion_tokens=32000, total_tokens=63000)",
                    "message must be byte-identical to the pre-reasoning-token-observability \
                     shape when the provider omits completion_tokens_details"
                );
            }
            other => {
                panic!("expected EmptyCompletion to round-trip as EmptyCompletion, got {other:?}")
            }
        }
    }

    #[test]
    fn test_annotate_with_usage_accepts_completion_tokens_details_omitting_reasoning_tokens() {
        // `completion_tokens_details` present but its own `reasoning_tokens` key absent —
        // must deserialize (not error) and annotate_with_usage must not fabricate a
        // reasoning_tokens clause it was never given.
        let json = r#"{"prompt_tokens":10,"completion_tokens":20,"total_tokens":30,"completion_tokens_details":{}}"#;
        let usage: DeepSeekUsage = serde_json::from_str(json).expect("should deserialize");

        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 20);
        assert_eq!(usage.total_tokens, 30);
        assert_eq!(
            usage
                .completion_tokens_details
                .as_ref()
                .and_then(|d| d.reasoning_tokens),
            None
        );

        let err = LlmError::EmptyCompletion("finish_reason=length with empty content".to_string());
        let annotated = annotate_with_usage(err, &usage);

        match annotated {
            LlmError::EmptyCompletion(msg) => {
                assert!(
                    !msg.contains("reasoning_tokens"),
                    "must not fabricate a reasoning_tokens clause when the key is absent, \
                     got: {msg}"
                );
            }
            other => {
                panic!("expected EmptyCompletion to round-trip as EmptyCompletion, got {other:?}")
            }
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
}
