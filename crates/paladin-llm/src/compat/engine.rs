//! Shared OpenAI-compatible protocol engine (D-05).
//!
//! `CompatEngine` owns every part of the OpenAI-compatible chat-completions
//! protocol that is identical across vendors: request shaping, HTTP
//! transport, SSE stream assembly, retry-with-backoff, status-to-`LlmError`
//! mapping, credential redaction and memoized model-list resolution.
//! Extracted and generalized from `deepseek/adapter.rs` (RESEARCH.md finding
//! 1 — that file and `openai/adapter.rs` already implement this same
//! protocol twice).
//!
//! A preset (e.g. `kimi::adapter::KimiAdapter`) supplies only its
//! `base_url`, credential, default model, curated model-list fallback and a
//! [`CompatCapabilities`] block, then delegates every [`LlmPort`] method to
//! an owned `CompatEngine`.
//!
//! [`LlmPort`]: paladin_ports::output::llm_port::LlmPort

use chrono::Utc;
use futures::{Stream, StreamExt};
use reqwest::{
    Client,
    header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue},
};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::OnceCell;
use uuid::Uuid;

use paladin_core::platform::container::prompt::{PromptItem, PromptType};
use paladin_ports::output::llm_port::{
    FinishReason, LlmError, LlmRequest, LlmResponse, ProviderCapabilities, StreamingResponse,
    TokenUsage,
};

use super::types::{CompatMessage, CompatModelsResponse, CompatRequest, CompatResponse};
use crate::redaction::diagnostic_excerpt as redact_and_bound;

/// Capabilities a preset declares for its own request path.
///
/// Per D-04's posture (extended to every preset, not only the generic
/// provider): every flag here must describe what *this adapter's own*
/// `generate()`/`generate_stream()` implements, never what the vendor's API
/// advertises in its own documentation.
#[derive(Debug, Clone)]
pub struct CompatCapabilities {
    /// Whether this preset's `generate_stream()` is implemented.
    pub supports_streaming: bool,
    /// Whether this preset's request path can carry a tool definition.
    /// `LlmRequest` has no tools field today, so every preset built on this
    /// engine must set this `false` (RESEARCH.md Pitfall 4).
    pub supports_tool_calling: bool,
    /// Whether this preset's `generate()` ever returns a populated
    /// `function_call`. No preset built on this engine does today.
    pub supports_function_calling: bool,
    /// Whether this preset's request path can carry image content.
    pub supports_vision: bool,
    /// Whether this preset exposes embeddings generation.
    pub supports_embeddings: bool,
    /// The preset's advertised maximum context window, in tokens.
    pub max_context_tokens: Option<u32>,
    /// Whether this preset's request path supports a system message.
    pub supports_system_messages: bool,
    /// The preset's valid temperature range (ADR-0004).
    pub temperature_range: Option<(f32, f32)>,
}

impl From<CompatCapabilities> for ProviderCapabilities {
    fn from(c: CompatCapabilities) -> Self {
        ProviderCapabilities {
            supports_streaming: c.supports_streaming,
            supports_tool_calling: c.supports_tool_calling,
            supports_function_calling: c.supports_function_calling,
            supports_vision: c.supports_vision,
            supports_embeddings: c.supports_embeddings,
            max_context_tokens: c.max_context_tokens,
            supports_system_messages: c.supports_system_messages,
            temperature_range: c.temperature_range,
        }
    }
}

/// Which of the five optional sampling parameters a preset's own request
/// path puts on the wire (17-18, closing G-17-4a).
///
/// Extends the D-04 posture from [`CompatCapabilities`] into request
/// shaping: each flag describes what **this preset's own request path**
/// carries, established by measurement against the vendor, not by reading
/// the vendor's marketing page. `presence_penalty` and `frequency_penalty`
/// were shipped as unconditionally-carried for every preset before this
/// plan; xAI rejects both **by presence**, for every current model — the
/// measured cause of G-17-4a, not a stale model ID.
///
/// A `false` flag does not omit the caller's value because the caller left
/// it unset — the caller may have set it explicitly, and it is still
/// dropped: the flag is a statement about the *vendor's protocol*, never
/// about what a particular request happened to ask for.
///
/// Deliberately has **no** `Default` impl, matching the same posture on
/// [`CompatEngineConfig`] itself (no `Default`, no struct-update syntax at
/// any construction site): a new preset must be a compile error until its
/// author states a position for every field, one at a time. Silent
/// inheritance of "everything is supported" is exactly how the shipped
/// Grok preset came to send a parameter xAI rejects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompatRequestParameters {
    /// Whether this preset's request path carries `temperature`.
    ///
    /// **Option (a) — chosen by the developer 2026-08-22, recorded here
    /// against ADR-0004 (closing G-17-4b, plan 17-19).** When a preset
    /// declares this `false`, [`CompatEngine::build_request`] omits
    /// `temperature` from the outgoing body entirely; it never substitutes
    /// one legal value for another. ADR-0004's *Considered Options* rejects
    /// adapter-level clamping by name: *"a caller who requested 1.8 and got
    /// 1.0 silently substituted has no way to discover the substitution
    /// happened."* Declaring this field `false` is not that: no value is
    /// swapped in for another, the key is simply absent from the JSON body,
    /// and the vendor's own single legal value applies server-side. The
    /// caller's declared-unsupported value is still discoverable — see the
    /// `debug` log this method emits when a value is dropped. First used by
    /// Kimi's fixed-temperature preset (`kimi::adapter::KimiAdapter`, plan
    /// 17-19), whose live-measured constraint made the framework's
    /// fabricated `0.7` default temperature (`PaladinData::default()`)
    /// unconditionally rejected by the vendor.
    pub temperature: bool,
    /// Whether this preset's request path carries `max_tokens`.
    pub max_tokens: bool,
    /// Whether this preset's request path carries `top_p`.
    pub top_p: bool,
    /// Whether this preset's request path carries `frequency_penalty`.
    pub frequency_penalty: bool,
    /// Whether this preset's request path carries `presence_penalty`.
    pub presence_penalty: bool,
}

impl CompatRequestParameters {
    /// Every optional sampling parameter carried — the OpenAI-compatible
    /// baseline every preset except Grok's xAI preset declares in this
    /// plan (T-17-75's behaviour-preservation requirement).
    pub const fn all() -> Self {
        Self {
            temperature: true,
            max_tokens: true,
            top_p: true,
            frequency_penalty: true,
            presence_penalty: true,
        }
    }
}

/// Configuration a preset supplies to construct a [`CompatEngine`].
pub struct CompatEngineConfig {
    /// The provider's API base URL, e.g. `https://api.moonshot.ai/v1`.
    pub base_url: String,
    /// The credential sent as `Authorization: Bearer {api_key}`.
    pub api_key: String,
    /// The default model identifier this preset requests.
    pub model: String,
    /// Request timeout, in seconds.
    pub timeout_seconds: u64,
    /// Maximum retry attempts for a retryable error.
    pub max_retries: u32,
    /// What this preset's own request path implements (D-04 posture).
    pub capabilities: CompatCapabilities,
    /// Which optional sampling parameters this preset's own request path
    /// carries (17-18). Gates `build_request`'s five optional fields — see
    /// [`CompatRequestParameters`]. Sits beside `error_override` and
    /// `redirect_policy` as a per-vendor protocol knob the engine reads
    /// off its own config rather than branching on provider identity.
    pub request_parameters: CompatRequestParameters,
    /// The curated model list returned when the live `/models` endpoint
    /// fails or returns an empty list (D-13).
    pub fallback_models: Vec<String>,
    /// Preset-specific status-code override, consulted BEFORE the engine's
    /// own mapping. Returning `Some(err)` short-circuits to that error;
    /// returning `None` falls through to the engine's default mapping. This
    /// is how a future preset (e.g. one with its own 402 semantics) adds an
    /// arm without editing the shared engine.
    pub error_override: Option<fn(u16, &str) -> Option<LlmError>>,
    /// Optional override for the underlying HTTP client's redirect policy.
    ///
    /// `None` preserves this engine's original behaviour — no `.redirect()`
    /// call on the client builder, i.e. `reqwest`'s own default policy
    /// (follow up to 10 hops). The field's meaning is **not** inverted by
    /// WR-04 (`17-REVIEW.md`): a future preset that legitimately needs to
    /// follow a redirect can still leave this `None` to get that behaviour.
    ///
    /// `Some(policy)` lets a preset restrict follow behaviour.
    /// `openai_compatible::OpenAiCompatibleAdapter` was the first to set
    /// `Some(Policy::none())` (T-17-18), because its `base_url` is entirely
    /// operator-supplied with no vendor default to fall back on. Every
    /// preset in this crate now sets the identical
    /// `Some(reqwest::redirect::Policy::none())` (T-17-52, plan 17-10):
    /// each one's `*_BASE_URL` is documented and operator-settable too, so
    /// the same reasoning applies uniformly. Setting `Policy::none()` means
    /// a `3xx` response can never cause the `Authorization` header carrying
    /// the operator's API key to be replayed to a different,
    /// attacker-influenced host — see [`CompatEngine::map_error`]'s
    /// `300..=399` arm for what a refused redirect surfaces to the caller
    /// as.
    pub redirect_policy: Option<reqwest::redirect::Policy>,
}

/// Whether a failed `fetch_live_models` call indicates the operator
/// misconfigured something, or is a state this design already supports and
/// deliberately stays quiet about (G-17-4d, 2026-08-22).
///
/// Read by [`CompatEngine::available_models`]'s `Err(e)` arm to choose a log
/// level. Deliberately just two states, not a description of *why* — the
/// reasoning for each variant's placement lives on
/// [`classify_fetch_failure`]'s match arms, not in this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FetchFailureClass {
    /// The operator most likely configured the wrong credential for the
    /// endpoint they pointed it at, or the wrong endpoint for the credential
    /// they hold. Worth surfacing at `warn`.
    Misconfiguration,
    /// A state this design already supports — offline, rate-limited, out of
    /// quota, or any other failure a model-list fetch has no business
    /// diagnosing on the operator's behalf. Stays at `debug`, unchanged from
    /// before this plan.
    Supported,
}

/// Classify a `fetch_live_models` failure for
/// [`CompatEngine::available_models`]'s log level (G-17-4d, 2026-08-22).
///
/// A pure function of the [`LlmError`] variant alone — no provider name, no
/// base-URL host, no feature flag. `CompatEngine` backs six presets (kimi,
/// qwen, grok, ollama, openai-compatible, and the deepseek-style preset
/// family), and this function reaching every one of them without any preset
/// change is exactly what keeps D-05's shared-engine boundary intact: the
/// engine still learns nothing about which vendor is on the other end.
///
/// Exhaustive over every `LlmError` variant — **deliberately no wildcard
/// arm**. A variant added to `paladin-ports::LlmError` later is a compile
/// error here until a person decides which side of the split it falls on,
/// rather than silently landing in a catch-all and being absorbed into the
/// quiet path unnoticed.
fn classify_fetch_failure(error: &LlmError) -> FetchFailureClass {
    match error {
        // The one case this plan closes: a rejected credential (mapped from
        // HTTP 401 by `map_error`) is a configuration problem, not a state
        // the D-13/D-14 fallback design was ever meant to describe quietly.
        LlmError::AuthenticationError(_) => FetchFailureClass::Misconfiguration,

        // Being offline is the exact case D-13/D-14 were written for — a
        // self-hosted Ollama that has not started yet is a normal, SUPPORTED
        // condition, not a misconfiguration. Warn-spam here would train
        // operators to ignore the one message this plan makes audible.
        LlmError::NetworkError(_) => FetchFailureClass::Supported,
        // A slow or unresponsive endpoint is transient, for the same reason
        // as the arm above.
        LlmError::Timeout(_) => FetchFailureClass::Supported,

        // A rate limit says "you called too often", not "you configured the
        // wrong thing" — the credential and the endpoint are both fine.
        LlmError::RateLimitExceeded => FetchFailureClass::Supported,
        // An exhausted quota is an account-billing state, not a
        // credential/endpoint mismatch — nothing about the CONFIGURATION is
        // wrong here either.
        LlmError::UsageLimitExceeded { .. } => FetchFailureClass::Supported,
        // `fetch_live_models` GETs `/models` with no model identifier on the
        // wire, so `map_error`'s 404 arm cannot actually be produced by this
        // call path. Classified anyway because this match must stay
        // exhaustive over the type, not merely over what is reachable
        // today — and an unknown-model 404 is not itself evidence of a
        // wrong credential or endpoint.
        LlmError::ModelNotAvailable(_) => FetchFailureClass::Supported,
        // Never produced by a model-list fetch (no prompt is sent) — quiet
        // for the same exhaustiveness-over-reachability reason as above.
        LlmError::TokenLimitExceeded => FetchFailureClass::Supported,
        // Never produced by a model-list fetch either (no completion is
        // requested) — same reason.
        LlmError::EmptyCompletion(_) => FetchFailureClass::Supported,
        // A malformed prompt cannot reach a model-list fetch at all — this
        // arm exists only so the match stays exhaustive.
        LlmError::InvalidPrompt(_) => FetchFailureClass::Supported,
        // Covers the generic non-success-status catch-all AND the
        // refused-redirect message (`map_error`'s `300..=399` arm, WR-04). A
        // refused redirect is arguably a misconfiguration too, but it is
        // left quiet deliberately: that message already names the exact
        // setting to check on its own, so raising it again here would only
        // duplicate an already-actionable message rather than surface a
        // silent one.
        LlmError::ProcessingError(_) => FetchFailureClass::Supported,
        // FT-01 (25-02 Task 1 landed these variants): a non-2xx status without a
        // dedicated variant, and an exhausted fallback chain, are classified like
        // the generic non-success catch-all above; 25-02 Task 2 / 25-05 own the
        // final status-aware split.
        LlmError::ProviderError { .. } => FetchFailureClass::Supported,
        LlmError::AllProvidersFailed { .. } => FetchFailureClass::Supported,
    }
}

/// Remove a userinfo component (`user:pass@`) from a base URL before it is
/// written into a log line (T-17-91). A configured base URL is
/// operator-supplied, and while no shipped preset's documented format
/// includes one, a base URL is exactly the kind of operator-supplied string
/// that could carry a credential this way.
///
/// Looks only at the authority section — everything between the first
/// `://` and the next `/` (or the end of the string, if there is no path).
/// If that section contains an `@`, everything up to and including it is
/// dropped; otherwise the input is returned unchanged. A string with no
/// `://` at all is returned unchanged rather than guessed at.
fn base_url_without_userinfo(base_url: &str) -> String {
    let Some(scheme_end) = base_url.find("://") else {
        return base_url.to_string();
    };
    let authority_start = scheme_end + 3;
    let authority_end = base_url[authority_start..]
        .find('/')
        .map(|i| authority_start + i)
        .unwrap_or(base_url.len());

    let authority = &base_url[authority_start..authority_end];
    // RFC 3986 terminates userinfo at the LAST '@' in the authority, not the
    // first. `find` here leaked a credential fragment whenever a password
    // contained an unescaped '@': for `https://alice:p@ssw0rd@example.com/v1`
    // it kept everything after the first '@', emitting `ssw0rd@example.com`
    // into the warn diagnostic. `rfind` strips the whole userinfo component.
    let Some(at_idx) = authority.rfind('@') else {
        return base_url.to_string();
    };

    let mut out = String::with_capacity(base_url.len());
    out.push_str(&base_url[..authority_start]);
    out.push_str(&authority[at_idx + 1..]);
    out.push_str(&base_url[authority_end..]);
    out
}

/// The shared OpenAI-compatible engine every preset built on this core
/// delegates to.
///
/// ## A known, recorded asymmetry (D-06, WR-04)
///
/// This engine's [`generate_stream`](Self::generate_stream) and
/// [`GeminiAdapter::generate_stream`](crate::gemini::GeminiAdapter) both
/// retry their connection-opening request, matching their own `generate()`
/// (17-16, closing the new WR-04 recorded in `17-REVIEW.md`). The shipped
/// `openai/`, `anthropic/` and `deepseek/` adapters do **not** — their
/// `generate_stream` implementations still send a single, unretried POST,
/// exactly as `deepseek/adapter.rs` (the file this engine was extracted and
/// generalized from) has always done. This is a **deliberate boundary, not
/// an oversight**: D-06 keeps this phase additive against shipped, covered
/// code, so those three adapters are read but never edited here. A reader
/// comparing this engine's or Gemini's streaming retry behaviour against
/// `openai`/`anthropic`/`deepseek` should read the difference as this
/// recorded decision, not re-file it as a fresh finding. **Trigger to close
/// it:** the next phase that has reason to touch those two adapters anyway
/// (`17-CONTEXT.md` §Deferred Ideas).
pub struct CompatEngine {
    client: Client,
    config: CompatEngineConfig,
    models_cache: OnceCell<Vec<String>>,
}

impl CompatEngine {
    /// Construct a new engine from preset-supplied configuration.
    ///
    /// # Errors
    /// Returns [`LlmError::AuthenticationError`] if the API key cannot be
    /// encoded as a header value, or [`LlmError::NetworkError`] if the
    /// underlying HTTP client cannot be built.
    pub fn new(mut config: CompatEngineConfig) -> Result<Self, LlmError> {
        let timeout = Duration::from_secs(config.timeout_seconds);

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", config.api_key)).map_err(|e| {
                LlmError::AuthenticationError(format!("Invalid API key format: {}", e))
            })?,
        );

        let mut client_builder = Client::builder().timeout(timeout).default_headers(headers);
        if let Some(policy) = config.redirect_policy.take() {
            client_builder = client_builder.redirect(policy);
        }

        let client = client_builder
            .build()
            .map_err(|e| LlmError::NetworkError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            config,
            models_cache: OnceCell::new(),
        })
    }

    /// The preset's declared capabilities, converted to the port's type.
    pub fn capabilities(&self) -> ProviderCapabilities {
        self.config.capabilities.clone().into()
    }

    /// Build the outgoing request body from a port-level [`LlmRequest`].
    ///
    /// Each of the five optional sampling parameters is gated on this
    /// engine's own [`CompatRequestParameters`] declaration (17-18): a
    /// carried parameter passes the caller's `Option` through unchanged; a
    /// parameter the preset does not carry resolves to `None`, which
    /// `CompatRequest`'s existing `skip_serializing_if = "Option::is_none"`
    /// turns into an absent JSON key — never a `null`. The gate reads only
    /// `self.config.request_parameters`, never the provider name or base
    /// URL, so the same mechanism serves the next vendor without editing
    /// this method.
    fn build_request(&self, request: &LlmRequest) -> Result<CompatRequest, LlmError> {
        let messages = Self::convert_prompt_to_messages(&request.prompt)?;
        let params = &request.prompt.node.node.parameters;
        let allowed = &self.config.request_parameters;

        let temperature = params.temperature.filter(|_| allowed.temperature);
        let max_tokens = params.max_tokens.filter(|_| allowed.max_tokens);
        let top_p = params.top_p.filter(|_| allowed.top_p);
        let frequency_penalty = params
            .frequency_penalty
            .filter(|_| allowed.frequency_penalty);
        let presence_penalty = params.presence_penalty.filter(|_| allowed.presence_penalty);

        let mut dropped: Vec<&'static str> = Vec::new();
        if params.temperature.is_some() && temperature.is_none() {
            dropped.push("temperature");
        }
        if params.max_tokens.is_some() && max_tokens.is_none() {
            dropped.push("max_tokens");
        }
        if params.top_p.is_some() && top_p.is_none() {
            dropped.push("top_p");
        }
        if params.frequency_penalty.is_some() && frequency_penalty.is_none() {
            dropped.push("frequency_penalty");
        }
        if params.presence_penalty.is_some() && presence_penalty.is_none() {
            dropped.push("presence_penalty");
        }
        if !dropped.is_empty() {
            log::debug!(
                "this preset's request_parameters declaration does not carry {:?}; \
                 dropped from the outgoing body although the caller supplied a value",
                dropped
            );
        }

        Ok(CompatRequest {
            model: request.model.clone(),
            messages,
            temperature,
            max_tokens,
            top_p,
            frequency_penalty,
            presence_penalty,
            stream: request.stream,
        })
    }

    /// Convert a `PromptItem` into the wire message list. Provider-agnostic
    /// — every `PromptType` arm the DeepSeek adapter covers is handled here.
    fn convert_prompt_to_messages(prompt: &PromptItem) -> Result<Vec<CompatMessage>, LlmError> {
        let mut messages = Vec::new();

        match &prompt.node.node.prompt_type {
            PromptType::System(system_prompt) => {
                messages.push(CompatMessage {
                    role: "system".to_string(),
                    content: system_prompt.instructions.clone(),
                    reasoning_content: None,
                });
            }
            PromptType::User(user_prompt) => {
                messages.push(CompatMessage {
                    role: "user".to_string(),
                    content: user_prompt.query.clone(),
                    reasoning_content: None,
                });
            }
            PromptType::Text(text_prompt) => {
                messages.push(CompatMessage {
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
                messages.push(CompatMessage {
                    role: "assistant".to_string(),
                    content: assistant_prompt.response.clone(),
                    reasoning_content: None,
                });
            }
            PromptType::Function(function_prompt) => {
                messages.push(CompatMessage {
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

    /// Map a compatible-protocol finish reason string to [`FinishReason`].
    ///
    /// Any string this engine does not recognise maps to
    /// [`FinishReason::Error`] rather than silently to `Stop` — an unknown
    /// finish reason is a signal worth surfacing, not swallowing (PROV-02
    /// adjacency edge). An absent/`null` finish reason maps to `Stop`.
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

    /// Detect a completion truncated before any content was produced (the
    /// reasoning-model truncation signature — see
    /// `deepseek/adapter.rs::detect_empty_completion`, the source this was
    /// generalized from).
    fn detect_empty_completion(content: &str, finish_reason: &FinishReason) -> Option<LlmError> {
        if matches!(finish_reason, FinishReason::Length) && content.trim().is_empty() {
            Some(LlmError::EmptyCompletion(format!(
                "finish_reason=length with empty content ({} raw chars) — reasoning likely \
                 consumed the entire max_tokens budget; retry with a larger max_tokens",
                content.len()
            )))
        } else {
            None
        }
    }

    /// Render untrusted provider text as a log-safe diagnostic excerpt.
    /// Redaction runs before truncation — see [`crate::redaction`].
    fn diagnostic_excerpt(&self, body: &str) -> String {
        redact_and_bound(body, &self.config.api_key)
    }

    /// Map an HTTP status + message to [`LlmError`].
    ///
    /// Checks the preset's `error_override` first so a preset-specific
    /// status (e.g. DeepSeek's 402) can be added without editing this
    /// engine.
    fn map_error(&self, status: u16, message: &str) -> LlmError {
        if let Some(override_fn) = self.config.error_override
            && let Some(err) = override_fn(status, message)
        {
            return err;
        }

        match status {
            401 => LlmError::AuthenticationError(format!("Invalid API key. Error: {}", message)),
            429 => LlmError::RateLimitExceeded,
            404 => LlmError::ModelNotAvailable(message.to_string()),
            400 => LlmError::InvalidPrompt(message.to_string()),
            // WR-04 (`17-REVIEW.md`, T-17-52): every preset now builds its
            // client with `redirect_policy: Some(Policy::none())`, so a
            // `3xx` response is never followed — it arrives here as an
            // ordinary non-success status instead. Named explicitly rather
            // than falling into the catch-all below, so the operator whose
            // previously-working endpoint now fails gets an actionable
            // message: which setting to check, not an opaque "API error".
            //
            // `LlmError::ProcessingError` is this engine's retryable set
            // (see `call_api_with_retry`), so a redirecting host is retried
            // up to `max_retries` before this surfaces — deliberate: adding
            // a new `LlmError` variant would breach PROV-02's "errors map
            // into the existing variants, not a new parallel error type"
            // rule, and no existing non-retryable variant means "refused
            // redirect". A few extra requests to a host already answering
            // `3xx` is the accepted cost (T-17-54, `17-REVIEW.md`).
            300..=399 => LlmError::ProcessingError(format!(
                "API error ({status}): the configured base URL responded with a redirect \
                 (HTTP {status}), which this client refuses to follow because doing so would \
                 forward the credential header to a different, potentially attacker-influenced \
                 host. Correct the configured base-URL setting to point directly at the \
                 intended endpoint. Response excerpt: {message}"
            )),
            _ => LlmError::ProcessingError(format!("API error ({}): {}", status, message)),
        }
    }

    /// Perform an API call with exponential-backoff-plus-jitter retry.
    ///
    /// Non-retryable set: `AuthenticationError | InvalidPrompt |
    /// EmptyCompletion | UsageLimitExceeded` — matching
    /// `deepseek/adapter.rs::call_api_with_retry`'s rationale: these need
    /// operator intervention or will not clear on backoff, so retrying
    /// burns attempts for no benefit.
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

    /// Generate a completion. POSTs to `{base_url}/chat/completions`.
    pub async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
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
                            "API request timed out after {} seconds",
                            self.config.timeout_seconds
                        ))
                    } else {
                        LlmError::NetworkError(format!("Failed to send request: {}", e))
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

            // Read the body to text FIRST, then deserialize it separately —
            // never `Response::json()` (RESEARCH.md Pitfall 3). Splitting the
            // two steps distinguishes a transport failure from a schema
            // mismatch, both in the error type and in the message.
            let body = response.text().await.map_err(|e| {
                if e.is_timeout() {
                    LlmError::Timeout(format!(
                        "Response body did not finish streaming within the {}s client timeout: {e}",
                        self.config.timeout_seconds
                    ))
                } else {
                    LlmError::NetworkError(format!("Failed to read response body: {e}"))
                }
            })?;

            let api_response: CompatResponse = serde_json::from_str(&body).map_err(|e| {
                LlmError::ProcessingError(format!(
                    "Failed to parse response (schema mismatch — field: {e}) — body excerpt: {}",
                    self.diagnostic_excerpt(&body)
                ))
            })?;

            let choice = api_response.choices.first().ok_or_else(|| {
                LlmError::EmptyCompletion("Response contained no choices".to_string())
            })?;

            let finish_reason = Self::map_finish_reason(choice.finish_reason.clone());

            if let Some(err) =
                Self::detect_empty_completion(&choice.message.content, &finish_reason)
            {
                return Err(err);
            }

            let prompt_tokens = api_response.usage.prompt_tokens;
            let completion_tokens = api_response.usage.completion_tokens;
            let total_tokens = api_response
                .usage
                .total_tokens
                .unwrap_or(prompt_tokens + completion_tokens);

            Ok(LlmResponse {
                id: Uuid::new_v4(),
                request_id: request.id,
                model: api_response.model,
                content: choice.message.content.clone(),
                finish_reason,
                usage: TokenUsage {
                    prompt_tokens,
                    completion_tokens,
                    total_tokens,
                },
                created_at: Utc::now(),
                metadata: HashMap::new(),
                function_call: None,
            })
        };

        self.call_api_with_retry(operation, self.config.max_retries)
            .await
    }

    /// Generate a streaming completion. POSTs with `stream: true` and
    /// assembles SSE `data: {...}` chunks, terminating on the literal
    /// `[DONE]` sentinel.
    ///
    /// ## Retry (WR-04, `17-REVIEW.md`)
    ///
    /// Only the connection-opening POST is retried — through the same
    /// `call_api_with_retry` helper and the same `self.config.max_retries`
    /// [`generate`](Self::generate) passes — so a transient failure opening
    /// the stream is retried exactly as many times as `generate()` retries
    /// the identical failure. The same non-retryable set applies: an
    /// authentication failure, an invalid prompt, an already-empty
    /// completion or a usage-limit rejection is attempted exactly once,
    /// never replaying a live `Authorization` credential to an endpoint
    /// that has already rejected it.
    ///
    /// Once the response is opened, `.bytes_stream()` is consumed exactly
    /// once, **outside** the retry loop: the byte stream itself is never
    /// re-read or re-opened, so a caller can never observe a duplicated or
    /// reordered delta as a result of this retry.
    ///
    /// **Cost:** each retried open re-sends the caller's entire prompt to
    /// the provider, so a transient failure can bill the prompt more than
    /// once. The non-retryable set above exists precisely so a failure the
    /// provider has already answered definitively is never retried.
    pub async fn generate_stream(
        &self,
        request: LlmRequest,
    ) -> Result<Box<dyn Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError> {
        let mut api_request = self.build_request(&request)?;
        api_request.stream = true;

        let url = format!("{}/chat/completions", self.config.base_url);

        // Only the connection-opening POST is retried — the byte stream is
        // deliberately consumed OUTSIDE this closure, once, after the retry
        // loop returns below. Re-opening a stream whose first chunks a
        // caller has already consumed would deliver the same tokens twice
        // with no marker (WR-04, `17-REVIEW.md`; T-17-71).
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
                            "API request timed out after {} seconds",
                            self.config.timeout_seconds
                        ))
                    } else {
                        LlmError::NetworkError(format!("Failed to send streaming request: {}", e))
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

            Ok(response)
        };

        let response = self
            .call_api_with_retry(operation, self.config.max_retries)
            .await?;

        let stream = response.bytes_stream();

        // `flat_map` rather than `map`: a single network chunk can carry more
        // than one complete SSE `data: {...}` event (this is common when a
        // mock transport — or a provider whose TCP framing does not align to
        // event boundaries — writes the whole body at once). Returning after
        // the FIRST matching line per chunk would silently drop every
        // subsequent delta in that chunk; every `data:` line found is
        // therefore emitted as its own stream item.
        let llm_stream = stream.flat_map(|chunk_result| {
            let items: Vec<Result<StreamingResponse, LlmError>> = match chunk_result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes).into_owned();
                    let mut items = Vec::new();

                    for line in text.lines() {
                        let Some(json_str) = line.strip_prefix("data: ") else {
                            continue;
                        };

                        if json_str.trim() == "[DONE]" {
                            items.push(Ok(StreamingResponse {
                                id: Uuid::new_v4(),
                                delta: String::new(),
                                finish_reason: Some(FinishReason::Stop),
                            }));
                            continue;
                        }

                        match serde_json::from_str::<super::types::CompatStreamResponse>(json_str) {
                            Ok(response) => {
                                if let Some(choice) = response.choices.first() {
                                    let content = choice.delta.content.clone().unwrap_or_default();
                                    items.push(Ok(StreamingResponse {
                                        id: Uuid::new_v4(),
                                        delta: content,
                                        finish_reason: choice
                                            .finish_reason
                                            .as_ref()
                                            .map(|r| Self::map_finish_reason(Some(r.clone()))),
                                    }));
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

    /// Fetch the live model list from `GET {base_url}/models`.
    async fn fetch_live_models(&self) -> Result<Vec<String>, LlmError> {
        let url = format!("{}/models", self.config.base_url);

        let response = self.client.get(&url).send().await.map_err(|e| {
            if e.is_timeout() {
                LlmError::Timeout(format!(
                    "Model list request timed out after {} seconds",
                    self.config.timeout_seconds
                ))
            } else {
                LlmError::NetworkError(format!("Failed to fetch model list: {}", e))
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

        let body = response
            .text()
            .await
            .map_err(|e| LlmError::NetworkError(format!("Failed to read model list body: {e}")))?;

        let parsed: CompatModelsResponse = serde_json::from_str(&body).map_err(|e| {
            LlmError::ProcessingError(format!(
                "Failed to parse model list (schema mismatch: {e}) — body excerpt: {}",
                self.diagnostic_excerpt(&body)
            ))
        })?;

        Ok(parsed.data.into_iter().map(|entry| entry.id).collect())
    }

    /// Resolve the model list: live on first call, memoized for this
    /// engine's lifetime (D-13/D-14). Falls back to the preset's curated
    /// list on any failure or an empty live response — **the fallback
    /// contract itself is deliberately unchanged by G-17-4d (2026-08-22,
    /// plan 17-22)**: this method still returns `Vec<String>`, still never
    /// errors, and still returns the curated list on every failure path. See
    /// the tests below (`available_models_returns_curated_fallback_on_*`)
    /// for the pin.
    ///
    /// What DID change (G-17-4d, 2026-08-22): the `Err(e)` arm used to log
    /// every failure at `debug` with the same sentence, which is precisely
    /// why a region/credential mismatch looked identical to an offline
    /// vendor and G-17-4c was misdiagnosed for five days. It now reads
    /// [`classify_fetch_failure`]'s verdict on `e` and only a
    /// misconfiguration (currently: `AuthenticationError`) is raised to
    /// `warn`; every other failure — including the offline/timeout states
    /// D-13/D-14 were written for — keeps its original `debug` wording
    /// unchanged. The empty-live-list arm above is untouched; an endpoint
    /// answering with an empty catalogue is neither an auth failure nor an
    /// offline one. `tokio::sync::OnceCell` still ensures exactly one fetch
    /// even under concurrent callers, so a rejected credential produces at
    /// most one warning per engine, not one per caller.
    pub async fn available_models(&self) -> Vec<String> {
        self.models_cache
            .get_or_init(|| async {
                match self.fetch_live_models().await {
                    Ok(models) if !models.is_empty() => models,
                    Ok(_) => {
                        log::debug!(
                            "live model list was empty; falling back to curated list (not authoritative)"
                        );
                        self.config.fallback_models.clone()
                    }
                    Err(e) => {
                        match classify_fetch_failure(&e) {
                            FetchFailureClass::Misconfiguration => {
                                log::warn!(
                                    "configured endpoint {} rejected the request while listing \
                                     models ({e}); the returned model list is the curated \
                                     fallback, not this vendor's own catalog — a credential \
                                     scoped to a different account or region is the usual cause",
                                    base_url_without_userinfo(&self.config.base_url)
                                );
                            }
                            FetchFailureClass::Supported => {
                                log::debug!(
                                    "live model list fetch failed ({e}); falling back to curated list (not authoritative)"
                                );
                            }
                        }
                        self.config.fallback_models.clone()
                    }
                }
            })
            .await
            .clone()
    }

    /// `true` when `model` appears in the resolved list (live or fallback).
    pub async fn validate_model(&self, model: &str) -> bool {
        self.available_models().await.iter().any(|m| m == model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use paladin_core::platform::container::prompt::UserPrompt;

    fn test_config() -> CompatEngineConfig {
        CompatEngineConfig {
            base_url: "https://example.invalid/v1".to_string(),
            api_key: "test-key".to_string(),
            model: "test-model".to_string(),
            timeout_seconds: 60,
            max_retries: 3,
            capabilities: CompatCapabilities {
                supports_streaming: true,
                supports_tool_calling: false,
                supports_function_calling: false,
                supports_vision: false,
                supports_embeddings: false,
                max_context_tokens: Some(1000),
                supports_system_messages: true,
                temperature_range: Some((0.0, 1.0)),
            },
            // Unchanged pre-existing behaviour: no vendor-specific
            // restriction has been measured for this synthetic test
            // preset, so it declares full support (17-18).
            request_parameters: CompatRequestParameters::all(),
            fallback_models: vec!["fallback-model".to_string()],
            error_override: None,
            redirect_policy: None,
        }
    }

    #[test]
    fn engine_constructs_from_valid_config() {
        let engine = CompatEngine::new(test_config());
        assert!(engine.is_ok());
    }

    #[test]
    fn map_finish_reason_stop_length_content_filter_and_function_call() {
        assert!(matches!(
            CompatEngine::map_finish_reason(Some("stop".to_string())),
            FinishReason::Stop
        ));
        assert!(matches!(
            CompatEngine::map_finish_reason(Some("length".to_string())),
            FinishReason::Length
        ));
        assert!(matches!(
            CompatEngine::map_finish_reason(Some("content_filter".to_string())),
            FinishReason::ContentFilter
        ));
        assert!(matches!(
            CompatEngine::map_finish_reason(Some("function_call".to_string())),
            FinishReason::FunctionCall
        ));
    }

    #[test]
    fn map_finish_reason_unknown_string_maps_to_error_never_silently_to_stop() {
        let result = CompatEngine::map_finish_reason(Some("bogus".to_string()));
        assert!(matches!(result, FinishReason::Error(_)));
    }

    #[test]
    fn map_finish_reason_absent_maps_to_stop() {
        assert!(matches!(
            CompatEngine::map_finish_reason(None),
            FinishReason::Stop
        ));
    }

    #[test]
    fn detect_empty_completion_length_and_empty_is_truncation() {
        let result = CompatEngine::detect_empty_completion("", &FinishReason::Length);
        assert!(matches!(result, Some(LlmError::EmptyCompletion(_))));
    }

    #[test]
    fn detect_empty_completion_non_empty_content_is_never_truncation() {
        assert!(
            CompatEngine::detect_empty_completion("some answer", &FinishReason::Length).is_none()
        );
        assert!(
            CompatEngine::detect_empty_completion("some answer", &FinishReason::Stop).is_none()
        );
    }

    #[test]
    fn detect_empty_completion_empty_but_stop_is_not_truncation() {
        let result = CompatEngine::detect_empty_completion("", &FinishReason::Stop);
        assert!(result.is_none());
    }

    #[test]
    fn map_error_status_codes() {
        let engine = CompatEngine::new(test_config()).unwrap();
        assert!(matches!(
            engine.map_error(401, "bad key"),
            LlmError::AuthenticationError(_)
        ));
        assert!(matches!(
            engine.map_error(429, "slow down"),
            LlmError::RateLimitExceeded
        ));
        assert!(matches!(
            engine.map_error(404, "no model"),
            LlmError::ModelNotAvailable(_)
        ));
        assert!(matches!(
            engine.map_error(400, "bad prompt"),
            LlmError::InvalidPrompt(_)
        ));
        assert!(matches!(
            engine.map_error(500, "server error"),
            LlmError::ProcessingError(_)
        ));
    }

    #[test]
    fn map_error_maps_a_redirect_status_to_an_actionable_processing_error() {
        let engine = CompatEngine::new(test_config()).unwrap();

        for status in [302u16, 307u16] {
            match engine.map_error(status, "moved") {
                LlmError::ProcessingError(msg) => {
                    assert!(
                        msg.contains("redirect"),
                        "status {status}: message must name the refused redirect, got: {msg}"
                    );
                    assert!(
                        msg.contains(&status.to_string()),
                        "status {status}: message must carry the numeric status, got: {msg}"
                    );
                }
                other => panic!("status {status}: expected ProcessingError, got {other:?}"),
            }
        }
    }

    #[test]
    fn map_error_consults_override_before_default_mapping() {
        fn override_fn(status: u16, _message: &str) -> Option<LlmError> {
            if status == 402 {
                Some(LlmError::UsageLimitExceeded {
                    provider: "test".to_string(),
                    regain_hint: None,
                })
            } else {
                None
            }
        }

        let mut config = test_config();
        config.error_override = Some(override_fn);
        let engine = CompatEngine::new(config).unwrap();

        assert!(matches!(
            engine.map_error(402, "insufficient balance"),
            LlmError::UsageLimitExceeded { .. }
        ));
        // Falls through to default mapping when the override returns None.
        assert!(matches!(
            engine.map_error(401, "bad key"),
            LlmError::AuthenticationError(_)
        ));
    }

    #[test]
    fn capabilities_reports_exactly_what_was_configured() {
        let engine = CompatEngine::new(test_config()).unwrap();
        let caps = engine.capabilities();
        assert!(caps.supports_streaming);
        assert!(!caps.supports_tool_calling);
        assert!(!caps.supports_function_calling);
        assert_eq!(caps.max_context_tokens, Some(1000));
        assert_eq!(caps.temperature_range, Some((0.0, 1.0)));
    }

    #[tokio::test(start_paused = true)]
    async fn call_api_with_retry_invokes_operation_exactly_once_on_empty_completion() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};

        let engine = CompatEngine::new(test_config()).unwrap();
        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = Arc::clone(&calls);

        let result: Result<(), LlmError> = engine
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
    async fn call_api_with_retry_retries_network_error_up_to_max_retries_plus_one() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};

        let engine = CompatEngine::new(test_config()).unwrap();
        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = Arc::clone(&calls);

        let result: Result<(), LlmError> = engine
            .call_api_with_retry(
                move || {
                    let calls = Arc::clone(&calls_clone);
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Err(LlmError::NetworkError("connection reset".to_string()))
                    }
                },
                3,
            )
            .await;

        assert!(result.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 4);
    }

    // ── WR-04 (new): a transient stream-open failure retries like generate() does ──
    //
    // Resolved facts (recorded in 17-16-SUMMARY.md per D-00e), read before
    // writing these tests:
    //   1. `test_config()`'s `max_retries` (this file, above) is `3`,
    //      matching every shipped preset (e.g. `kimi/adapter.rs:157`), and
    //      `call_api_with_retry`'s `for attempt in 0..=max_retries` loop
    //      (compat/engine.rs:357) means `max_retries + 1` = 4 total
    //      attempts on a retryable error — the `+1` semantics this file's
    //      own `call_api_with_retry_retries_network_error_up_to_max_retries_plus_one`
    //      test already pins.
    //   2. HTTP 500 is retryable: it falls through every named arm in
    //      `map_error` (401, 429, 404, 400, 300..=399) into the catch-all
    //      at compat/engine.rs:335, which returns the retryable
    //      `LlmError::ProcessingError`.
    //   3. These tests exercise `CompatEngine` directly (not through the
    //      `KimiAdapter` preset) — Kimi already has `generate_stream` mock
    //      -transport scaffolding (`kimi/adapter.rs`
    //      `generate_stream_assembles_deltas_in_wire_order_with_terminal_stop`),
    //      whose shape (endpoint, SSE body, `[DONE]` sentinel) is reused
    //      here directly against the shared engine, per the plan's "reuse
    //      that scaffolding" instruction. `kimi/adapter.rs` itself is
    //      untouched — this plan's `files_modified` names only this file
    //      and `gemini/adapter.rs`.
    //
    // The "transient" test below derives the expected attempt count by
    // running the same mock failure through `generate()` first, rather than
    // hardcoding `4` — the assertion is "streaming retries like
    // non-streaming", not "streaming retries four times", and does not
    // silently rot if `call_api_with_retry`'s cap ever changes.

    fn test_config_at(base_url: &str) -> CompatEngineConfig {
        let mut config = test_config();
        config.base_url = base_url.to_string();
        config
    }

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

    // Not `start_paused = true`: this test exercises real network round
    // -trips against a `mockito` server, and pausing the tokio clock races
    // the retry backoff timer against `reqwest`'s own request timeout —
    // confirmed directly against the sibling Gemini test in
    // `gemini/adapter.rs`, whose third attempt spuriously timed out under
    // a paused clock instead of exercising all `max_retries` attempts. Real
    // (unpaused) time makes the ~700ms of total backoff actually elapse,
    // which is what the synthetic-closure retry tests above use
    // `start_paused = true` to avoid paying.
    #[tokio::test]
    async fn generate_stream_retries_a_transient_open_failure_as_many_times_as_generate() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};

        let mut generate_server = Server::new_async().await;
        let generate_calls = Arc::new(AtomicU32::new(0));
        let generate_calls_clone = Arc::clone(&generate_calls);
        generate_server
            .mock("POST", "/chat/completions")
            .with_status(500)
            .with_body_from_request(move |_req| {
                generate_calls_clone.fetch_add(1, Ordering::SeqCst);
                br#"{"error":"transient failure"}"#.to_vec()
            })
            .create_async()
            .await;

        let generate_engine = CompatEngine::new(test_config_at(&generate_server.url())).unwrap();
        let generate_result = generate_engine.generate(build_request("test-model")).await;
        assert!(generate_result.is_err());
        let generate_attempt_count = generate_calls.load(Ordering::SeqCst);

        let mut stream_server = Server::new_async().await;
        let stream_calls = Arc::new(AtomicU32::new(0));
        let stream_calls_clone = Arc::clone(&stream_calls);
        let stream_mock = stream_server
            .mock("POST", "/chat/completions")
            .with_status(500)
            .with_body_from_request(move |_req| {
                stream_calls_clone.fetch_add(1, Ordering::SeqCst);
                br#"{"error":"transient failure"}"#.to_vec()
            })
            // Load-bearing (D-00e): today `generate_stream` makes exactly
            // ONE request on a transient failure, not
            // `generate_attempt_count` — this `.expect()` is what fails in
            // the RED state, not the `Result` assertion below.
            .expect(generate_attempt_count as usize)
            .create_async()
            .await;

        let stream_engine = CompatEngine::new(test_config_at(&stream_server.url())).unwrap();
        let stream_result = stream_engine
            .generate_stream(build_request("test-model"))
            .await;

        stream_mock.assert_async().await;
        assert_eq!(
            stream_calls.load(Ordering::SeqCst),
            generate_attempt_count,
            "generate_stream must retry the connection-opening POST exactly \
             as many times as generate() does"
        );
        assert!(stream_result.is_err());
    }

    // Not `start_paused = true` — see the sibling transient test's comment
    // above: a paused clock is unreliable against a real `mockito` network
    // round trip in this suite.
    #[tokio::test]
    async fn generate_stream_does_not_retry_an_authentication_failure_on_open() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/chat/completions")
            .with_status(401)
            .with_body(r#"{"error":"invalid key"}"#)
            // Load-bearing (D-00e): passes today (there is no retry at all)
            // and must keep passing after the fix — this is the
            // credential-replay guard, proving an authentication failure on
            // stream open is attempted exactly once.
            .expect(1)
            .create_async()
            .await;

        let engine = CompatEngine::new(test_config_at(&server.url())).unwrap();
        let result = engine.generate_stream(build_request("test-model")).await;

        mock.assert_async().await;
        // `result`'s `Ok` payload is a boxed `dyn Stream` with no `Debug`
        // impl, so this is matched by hand rather than via a formatted
        // `matches!` assertion.
        match &result {
            Err(LlmError::AuthenticationError(_)) => {}
            Ok(_) => panic!("expected Err(AuthenticationError), got Ok(<stream>)"),
            Err(other) => panic!("expected AuthenticationError, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn generate_stream_opens_exactly_once_and_yields_its_deltas_in_order_on_success() {
        let sse_body = concat!(
            "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"Hel\"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"lo \"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"world\"},\"finish_reason\":\"stop\"}]}\n\n",
            "data: [DONE]\n\n",
        );

        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(sse_body)
            // Load-bearing (D-00e): proves the retried-open shape does not
            // double-open a working connection.
            .expect(1)
            .create_async()
            .await;

        let engine = CompatEngine::new(test_config_at(&server.url())).unwrap();
        let stream = engine
            .generate_stream(build_request("test-model"))
            .await
            .unwrap();
        let mut stream = Box::into_pin(stream);

        let mut deltas = Vec::new();
        while let Some(item) = stream.next().await {
            let chunk = item.unwrap();
            if !chunk.delta.is_empty() {
                deltas.push(chunk.delta);
            }
        }

        mock.assert_async().await;
        // Exactly the three wire deltas, in order, none duplicated — a
        // future change that retried the byte stream would double this.
        assert_eq!(
            deltas,
            vec!["Hel".to_string(), "lo ".to_string(), "world".to_string()]
        );
    }

    // ── CompatRequestParameters (17-18, closing G-17-4a) ──
    //
    // A preset declares which of the five optional sampling parameters its
    // own request path carries; the engine reads only that declaration —
    // never a provider name, base URL, or other vendor identity — to decide
    // what reaches the wire. These five tests exercise the mechanism
    // directly against a synthetic preset carrying no vendor identity at
    // all, per the plan's own edge case: "a synthetic preset carrying no
    // vendor identity at all, declaring one parameter unsupported, produces
    // a body omitting exactly that parameter."
    //
    // All five values below (0.5, 0.25, 0.75, 0.125) are exact sums of
    // negative powers of two, so the `f32` caller value and the `f64`
    // parsed from the captured wire JSON compare exactly — no floating
    // -point tolerance is needed or used.

    use paladin_core::platform::container::prompt::PromptParameters;
    use serde_json::{Value, json};
    use std::sync::{Arc, Mutex};

    fn test_config_with_request_parameters(
        base_url: &str,
        request_parameters: CompatRequestParameters,
    ) -> CompatEngineConfig {
        let mut config = test_config_at(base_url);
        config.request_parameters = request_parameters;
        config
    }

    /// Every one of the five optional sampling parameters set to a distinct,
    /// exactly-representable value, so a captured wire body can be checked
    /// against the caller's actual value, not merely "is present".
    fn fully_specified_parameters() -> PromptParameters {
        PromptParameters {
            max_tokens: Some(123),
            temperature: Some(0.5),
            top_p: Some(0.25),
            frequency_penalty: Some(0.75),
            presence_penalty: Some(0.125),
            stop_sequences: None,
        }
    }

    fn build_request_with_parameters(model: &str, parameters: PromptParameters) -> LlmRequest {
        let mut request = build_request(model);
        request.prompt.node.node.parameters = parameters;
        request
    }

    /// Sets up a mock `/chat/completions` endpoint that captures the raw
    /// outgoing request body (as both parsed JSON and the original text) and
    /// answers with a minimal well-formed completion, and runs `generate()`
    /// against it. Returns the captured body — this is the shared plumbing
    /// under all five tests below; each test differs only in the engine
    /// config and request it builds.
    async fn generate_and_capture_body(
        config: CompatEngineConfig,
        request: LlmRequest,
    ) -> (Value, String) {
        // `server` must outlive the call to `generate()` below (it is
        // dropped at the end of this function's scope, after `.await`
        // returns), so it cannot be constructed inside a nested block.
        let mut server = Server::new_async().await;
        let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let captured_clone = Arc::clone(&captured);

        // `config`'s `base_url` is overwritten to point at this mock server
        // — every caller below builds its config via `test_config_at`-style
        // helpers whose `base_url` is a placeholder replaced here, so the
        // mock is guaranteed to be the one actually hit.
        let mut config = config;
        config.base_url = server.url();

        server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body_from_request(move |req| {
                let body_text = req.utf8_lossy_body().unwrap_or_default().into_owned();
                *captured_clone.lock().unwrap() = Some(body_text);
                json!({
                    "id": "cmpl-1",
                    "model": "test-model",
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

        let engine = CompatEngine::new(config).unwrap();
        let result = engine.generate(request).await;
        assert!(
            result.is_ok(),
            "mock server returned a well-formed response: {result:?}"
        );

        let body_text = captured
            .lock()
            .unwrap()
            .take()
            .expect("mock must have been called exactly once");
        let body_json: Value =
            serde_json::from_str(&body_text).expect("captured body must be valid JSON");
        (body_json, body_text)
    }

    // Test 1 — mechanism is data-driven: a preset declaring
    // `presence_penalty: false` and everything else `true` omits exactly
    // that key while carrying the caller's values for the rest.
    #[tokio::test]
    async fn build_request_omits_exactly_the_declared_unsupported_parameter() {
        let mut request_parameters = CompatRequestParameters::all();
        request_parameters.presence_penalty = false;

        let config =
            test_config_with_request_parameters("https://example.invalid/v1", request_parameters);
        let request = build_request_with_parameters("test-model", fully_specified_parameters());

        let (body, _) = generate_and_capture_body(config, request).await;
        let obj = body.as_object().expect("body must be a JSON object");

        assert!(
            !obj.contains_key("presence_penalty"),
            "declared-unsupported parameter must be absent from the wire body, got: {obj:?}"
        );
        assert_eq!(obj.get("temperature").and_then(Value::as_f64), Some(0.5));
        assert_eq!(obj.get("max_tokens").and_then(Value::as_u64), Some(123));
        assert_eq!(obj.get("top_p").and_then(Value::as_f64), Some(0.25));
        assert_eq!(
            obj.get("frequency_penalty").and_then(Value::as_f64),
            Some(0.75)
        );
    }

    // Test 2 — behaviour preservation: a preset declaring every parameter
    // carried produces a body byte-for-byte identical, in substance, to the
    // pre-17-18 engine — all five keys present with exactly the caller's
    // values.
    #[tokio::test]
    async fn build_request_with_all_declared_carries_every_caller_value_unchanged() {
        let config = test_config_with_request_parameters(
            "https://example.invalid/v1",
            CompatRequestParameters::all(),
        );
        let request = build_request_with_parameters("test-model", fully_specified_parameters());

        let (body, _) = generate_and_capture_body(config, request).await;
        let obj = body.as_object().expect("body must be a JSON object");

        assert_eq!(obj.get("temperature").and_then(Value::as_f64), Some(0.5));
        assert_eq!(obj.get("max_tokens").and_then(Value::as_u64), Some(123));
        assert_eq!(obj.get("top_p").and_then(Value::as_f64), Some(0.25));
        assert_eq!(
            obj.get("frequency_penalty").and_then(Value::as_f64),
            Some(0.75)
        );
        assert_eq!(
            obj.get("presence_penalty").and_then(Value::as_f64),
            Some(0.125)
        );
    }

    // Test 3 — omission is absence, not `null`: the raw wire text contains
    // no occurrence of the dropped key's name at all, so a future change
    // that serialised `"presence_penalty":null` instead of dropping the key
    // would fail this test even though Test 1's `contains_key` check on the
    // parsed object could not tell the difference between "absent" and
    // "present with a null value that happened to parse oddly".
    #[tokio::test]
    async fn build_request_omission_is_absence_never_a_null_value() {
        let mut request_parameters = CompatRequestParameters::all();
        request_parameters.presence_penalty = false;

        let config =
            test_config_with_request_parameters("https://example.invalid/v1", request_parameters);
        let request = build_request_with_parameters("test-model", fully_specified_parameters());

        let (_, body_text) = generate_and_capture_body(config, request).await;

        assert!(
            !body_text.contains("presence_penalty"),
            "the key name itself must not appear on the wire — got body: {body_text}"
        );
    }

    // Test 4 — a caller-supplied value cannot resurrect a declared
    // -unsupported parameter: the declaration is about the vendor's
    // protocol, not about whether the caller happened to supply a value.
    #[tokio::test]
    async fn build_request_declared_unsupported_wins_even_when_caller_sets_a_value() {
        let mut request_parameters = CompatRequestParameters::all();
        request_parameters.presence_penalty = false;

        let config =
            test_config_with_request_parameters("https://example.invalid/v1", request_parameters);
        let mut parameters = fully_specified_parameters();
        parameters.presence_penalty = Some(1.5);
        let request = build_request_with_parameters("test-model", parameters);

        let (body, body_text) = generate_and_capture_body(config, request).await;
        let obj = body.as_object().expect("body must be a JSON object");

        assert!(
            !obj.contains_key("presence_penalty"),
            "an explicit caller value must not resurrect a declared-unsupported \
             parameter, got: {obj:?}"
        );
        assert!(!body_text.contains("presence_penalty"));
    }

    // Test 5 — unset stays unset: a preset declaring a parameter carried,
    // whose caller left it `None`, still omits the key. This is the
    // pre-existing `skip_serializing_if` behaviour, and this test proves the
    // new gating logic does not disturb it.
    #[tokio::test]
    async fn build_request_carried_but_caller_unset_parameter_stays_omitted() {
        let config = test_config_with_request_parameters(
            "https://example.invalid/v1",
            CompatRequestParameters::all(),
        );
        let mut parameters = fully_specified_parameters();
        parameters.presence_penalty = None;
        let request = build_request_with_parameters("test-model", parameters);

        let (body, _) = generate_and_capture_body(config, request).await;
        let obj = body.as_object().expect("body must be a JSON object");

        assert!(
            !obj.contains_key("presence_penalty"),
            "a parameter the caller left unset must stay omitted, got: {obj:?}"
        );
        // The other four, which the caller DID set, must still be present —
        // proving this is "unset stays unset", not an accidental drop of
        // everything.
        assert_eq!(obj.get("temperature").and_then(Value::as_f64), Some(0.5));
        assert_eq!(obj.get("max_tokens").and_then(Value::as_u64), Some(123));
        assert_eq!(obj.get("top_p").and_then(Value::as_f64), Some(0.25));
        assert_eq!(
            obj.get("frequency_penalty").and_then(Value::as_f64),
            Some(0.75)
        );
    }

    // Test 6 (17-19, closing G-17-4b) — option (a) pinned specifically for
    // `temperature`: a preset declaring `temperature: false` produces a body
    // with no `temperature` key at all, while the other four sampling
    // parameters still carry the caller's values unchanged. Tests 1-5 above
    // exercise the mechanism generically through `presence_penalty`; this
    // test exercises the exact field plan 17-19's Kimi preset declares
    // absent, so a future change that special-cased `presence_penalty`
    // without generalizing to `temperature` would be caught here.
    #[tokio::test]
    async fn build_request_omits_exactly_a_declared_absent_temperature() {
        let mut request_parameters = CompatRequestParameters::all();
        request_parameters.temperature = false;

        let config =
            test_config_with_request_parameters("https://example.invalid/v1", request_parameters);
        let request = build_request_with_parameters("test-model", fully_specified_parameters());

        let (body, body_text) = generate_and_capture_body(config, request).await;
        let obj = body.as_object().expect("body must be a JSON object");

        assert!(
            !obj.contains_key("temperature"),
            "declared-absent temperature must not appear on the wire, got: {obj:?}"
        );
        assert!(
            !body_text.contains("temperature"),
            "the key name itself must not appear on the wire — got body: {body_text}"
        );
        // The other four parameters, all declared carried, must still be
        // present with exactly the caller's values — proving this is a
        // targeted omission of one field, not an accidental drop of all.
        assert_eq!(obj.get("max_tokens").and_then(Value::as_u64), Some(123));
        assert_eq!(obj.get("top_p").and_then(Value::as_f64), Some(0.25));
        assert_eq!(
            obj.get("frequency_penalty").and_then(Value::as_f64),
            Some(0.75)
        );
        assert_eq!(
            obj.get("presence_penalty").and_then(Value::as_f64),
            Some(0.125)
        );
    }

    // ── classify_fetch_failure (17-22, closing G-17-4d's diagnosability half) ──
    //
    // One test per `LlmError` variant, matching the plan's own requirement
    // ("Each arm is asserted by its own test") — this is what makes the
    // match's exhaustiveness meaningful: a variant added later without a
    // corresponding test here is still caught by the compiler (no wildcard
    // arm in `classify_fetch_failure` itself), but a variant classified on
    // the WRONG side of the split would only be caught by these.

    #[test]
    fn classify_fetch_failure_authentication_error_is_misconfiguration() {
        assert_eq!(
            classify_fetch_failure(&LlmError::AuthenticationError("bad key".to_string())),
            FetchFailureClass::Misconfiguration
        );
    }

    #[test]
    fn classify_fetch_failure_network_error_is_supported() {
        assert_eq!(
            classify_fetch_failure(&LlmError::NetworkError("connection refused".to_string())),
            FetchFailureClass::Supported
        );
    }

    #[test]
    fn classify_fetch_failure_timeout_is_supported() {
        assert_eq!(
            classify_fetch_failure(&LlmError::Timeout("timed out".to_string())),
            FetchFailureClass::Supported
        );
    }

    #[test]
    fn classify_fetch_failure_rate_limit_exceeded_is_supported() {
        assert_eq!(
            classify_fetch_failure(&LlmError::RateLimitExceeded),
            FetchFailureClass::Supported
        );
    }

    #[test]
    fn classify_fetch_failure_usage_limit_exceeded_is_supported() {
        assert_eq!(
            classify_fetch_failure(&LlmError::UsageLimitExceeded {
                provider: "test".to_string(),
                regain_hint: None,
            }),
            FetchFailureClass::Supported
        );
    }

    #[test]
    fn classify_fetch_failure_model_not_available_is_supported() {
        assert_eq!(
            classify_fetch_failure(&LlmError::ModelNotAvailable("no such model".to_string())),
            FetchFailureClass::Supported
        );
    }

    #[test]
    fn classify_fetch_failure_token_limit_exceeded_is_supported() {
        assert_eq!(
            classify_fetch_failure(&LlmError::TokenLimitExceeded),
            FetchFailureClass::Supported
        );
    }

    #[test]
    fn classify_fetch_failure_empty_completion_is_supported() {
        assert_eq!(
            classify_fetch_failure(&LlmError::EmptyCompletion("no text".to_string())),
            FetchFailureClass::Supported
        );
    }

    #[test]
    fn classify_fetch_failure_invalid_prompt_is_supported() {
        assert_eq!(
            classify_fetch_failure(&LlmError::InvalidPrompt("empty prompt".to_string())),
            FetchFailureClass::Supported
        );
    }

    #[test]
    fn classify_fetch_failure_processing_error_is_supported() {
        assert_eq!(
            classify_fetch_failure(&LlmError::ProcessingError("server error".to_string())),
            FetchFailureClass::Supported
        );
    }

    // ── base_url_without_userinfo (T-17-91) ──

    #[test]
    fn base_url_without_userinfo_strips_a_credential_from_the_authority() {
        assert_eq!(
            base_url_without_userinfo("https://user:secret@example.invalid/v1"),
            "https://example.invalid/v1"
        );
    }

    #[test]
    fn base_url_without_userinfo_strips_a_password_containing_an_unescaped_at() {
        // WR-01: userinfo ends at the LAST '@' per RFC 3986. Stripping to the
        // first one left `ssw0rd@example.invalid` in the warn line.
        assert_eq!(
            base_url_without_userinfo("https://alice:p@ssw0rd@example.invalid/v1"),
            "https://example.invalid/v1"
        );
        assert_eq!(
            base_url_without_userinfo("https://a@b@c@example.invalid:8443/v1"),
            "https://example.invalid:8443/v1"
        );
    }

    #[test]
    fn base_url_without_userinfo_leaves_a_plain_url_unchanged() {
        let url = "https://dashscope-intl.aliyuncs.com/compatible-mode/v1";
        assert_eq!(base_url_without_userinfo(url), url);
    }

    #[test]
    fn base_url_without_userinfo_preserves_port_and_path_after_stripping() {
        assert_eq!(
            base_url_without_userinfo("https://user:secret@example.invalid:8443/v1/models"),
            "https://example.invalid:8443/v1/models"
        );
    }

    #[test]
    fn base_url_without_userinfo_handles_a_string_with_no_scheme_separator() {
        let malformed = "not-a-url";
        assert_eq!(base_url_without_userinfo(malformed), malformed);
    }

    // ── available_models' D-13/D-14 fallback contract, unchanged by this
    //    plan (17-22) ──
    //
    // These pin the exact must_have the plan prohibits touching: the method
    // still returns `Vec<String>`, never errors, and returns the curated
    // list on both a misconfiguration (401) and a transport failure —
    // proven against the RETURNED VALUE, not merely against the log, since
    // no log-capture crate may be added (T-17-SC-22).

    #[tokio::test]
    async fn available_models_returns_curated_fallback_on_authentication_failure() {
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/models")
            .with_status(401)
            .with_body(r#"{"error":"invalid_api_key"}"#)
            .create_async()
            .await;

        let engine = CompatEngine::new(test_config_at(&server.url())).unwrap();
        let models = engine.available_models().await;

        assert_eq!(models, vec!["fallback-model".to_string()]);
    }

    #[tokio::test]
    async fn available_models_returns_curated_fallback_on_connection_failure() {
        // A bound-then-immediately-dropped TCP listener yields a port no
        // process is listening on, so connecting to it fails fast with a
        // real `NetworkError` — no mock server involved, because this test
        // exercises the transport-failure path `fetch_live_models` produces
        // before any HTTP status exists to mock.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        let engine = CompatEngine::new(test_config_at(&format!("http://{addr}"))).unwrap();
        let models = engine.available_models().await;

        assert_eq!(models, vec!["fallback-model".to_string()]);
    }
}
