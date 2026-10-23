//! Google Gemini LLM Adapter (bespoke protocol, D-08).
//!
//! Gemini's `generateContent` API is **not** OpenAI-compatible — this
//! adapter implements [`LlmPort`] directly against Gemini's own wire shape
//! rather than delegating to [`crate::compat::CompatEngine`]. Structural
//! template: `crate::anthropic::adapter` — the "own request/response types,
//! own streaming parse loop, own error mapping" shape, applied here to a
//! different vendor protocol. The only shared code this file consumes is
//! the crate-level redaction trio (`crate::redaction`).
//!
//! Gemini's own divergences from every other adapter in this crate:
//!
//! - **`systemInstruction` is a top-level sibling field**, never a
//!   `contents[]` entry with a system role — Gemini's API has no `system`
//!   role inside `contents[]`; sending one is a request the API rejects.
//! - **Auth is the `x-goog-api-key` header.** Google's docs also show a
//!   `?key=` query-parameter form for some endpoints; this adapter never
//!   uses it — a credential in a URL lands in proxy logs, server access
//!   logs and any diagnostic that echoes the request line (T-17-24).
//! - **The URL carries the operation as a `:generateContent` /
//!   `:streamGenerateContent` suffix**, not a path segment:
//!   `{base_url}/models/{model}:generateContent`.
//! - **Streaming requires the `alt=sse` query parameter.** Without it,
//!   Google's endpoint returns a raw JSON array instead of SSE framing, and
//!   this adapter's line-oriented parse loop would silently produce
//!   nothing.
//! - **Gemini streams partial `GenerateContentResponse` objects** — the
//!   same shape `GeminiResponse` already parses for the non-streaming
//!   path — rather than a distinct delta type. There is no `[DONE]`
//!   sentinel; the stream simply ends when the body ends.
//!
//! This adapter is text-only (D-08): `get_capabilities().supports_vision`
//! is `false`, and `tools`/`toolConfig` are omitted from every request
//! entirely — `LlmRequest` has no field through which a tool definition
//! could travel, so sending an empty value would be a capability signal
//! this adapter cannot honour.
//!
//! ## Trust boundary: the caller-supplied model identifier
//!
//! `LlmRequest.model` crosses from the caller into the request **path**
//! (`{base_url}/models/{model}:generateContent`), not into a
//! serde-encoded JSON body like every `CompatEngine`-based preset in this
//! crate. The `validate_model_identifier` guard is the sole barrier that
//! stops a hostile value from displacing an existing path segment or
//! injecting a query parameter (CR-01, `17-VERIFICATION.md`) — it runs as
//! the first statement of both `generate` and `generate_stream`, before
//! any URL is built. The residual, deliberately out-of-scope trust
//! decision is the operator's own `GEMINI_BASE_URL`: nothing in this
//! module validates where the *host* points, only the model segment
//! appended to it. That surface is plan 17-10's subject.
//!
//! ## Trust boundary: the operator-supplied base URL (T-17-52, WR-04)
//!
//! `GEMINI_BASE_URL` determines where every credential-bearing request
//! goes — the `x-goog-api-key` default header set in [`GeminiAdapter::new`]
//! rides on every request this client sends. The redirect policy is
//! `none` (also set in [`GeminiAdapter::new`]) so a `3xx` response can
//! never move that header to a host the operator did not configure; a
//! refused redirect surfaces via `GeminiAdapter::map_error`'s
//! `300..=399` arm. The residual case — an operator deliberately pointing
//! `base_url` at an internal address — is the operator's own trust
//! decision; no allowlist is introduced, matching
//! [`crate::openai_compatible`]'s documented posture (T-17-18) and
//! PROJECT.md's single-tenant, operator-controlled configuration model.

use async_trait::async_trait;
use chrono::Utc;
use futures::{Stream, StreamExt};
use reqwest::{
    Client,
    header::{CONTENT_TYPE, HeaderMap, HeaderValue},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::time::{Duration, SystemTime};
use tokio::sync::OnceCell;
use uuid::Uuid;

use paladin_core::platform::container::prompt::{PromptRole, PromptType};
use paladin_ports::output::llm_port::ReasoningFallback;
use paladin_ports::output::llm_port::{
    FinishReason, LlmError, LlmPort, LlmRequest, LlmResponse, ModelProfile, ParamSupport,
    ProviderCapabilities, ReasoningEffort, ResponseFormat, StopSupport, StreamingResponse,
    TokenUsage,
};
use paladin_ports::output::model_profile::ResolvedEffort;
use paladin_ports::output::rate_limit_hints::RateLimitHints;

use crate::http_status::map_http_status_with_hints;
use crate::rate_limit_headers::{RateLimitHeaderFamily, hints_from_headers};
use crate::reasoning::{self, SubstitutionLog, WireCaps, mask_effort};
use crate::redaction::{
    RESPONSE_EXCERPT_CHAR_BUDGET, RedactedKey, bounded_excerpt, diagnostic_excerpt,
    redact_credentials,
};
use crate::registry::{
    DEFAULT_MAX_RETRIES, ModelOverride, ModelRegistry, config_knob_builders,
    default_timeout_seconds, is_http_url, seeded, seeded_window,
};
use crate::response_guards::{StreamCompletion, bill, classify_completion};
use crate::sampling::{self, FormatLog, OmissionLog, RequestNotes};
use crate::sse::SseLines;
use crate::usage_reconcile::{USAGE_UNRECONCILED_METADATA_KEY, UsageLog};

/// The provider name this adapter reports through [`LlmPort::get_provider_name`]
/// and stamps on every [`LlmError::ProviderError`] it emits (Phase 25 D-03).
const GEMINI_PROVIDER: &str = "gemini";

/// Parse a `429` response's rate-limit headers through the generic family (PACE-01).
///
/// Must run BEFORE `response.text()` consumes the response (research Pitfall 2). Only a 429
/// is parsed, and no credential header (`x-goog-api-key` included) is read. Gemini's own
/// body-level `RetryInfo` is deliberately not parsed this phase (research OQ6): a Gemini 429
/// carries a delay only if the endpoint also sent a `Retry-After` header.
fn snapshot_rate_limit_hints(
    status: reqwest::StatusCode,
    headers: &HeaderMap,
) -> Option<RateLimitHints> {
    if status != reqwest::StatusCode::TOO_MANY_REQUESTS {
        return None;
    }
    hints_from_headers(RateLimitHeaderFamily::Generic, SystemTime::now(), |name| {
        headers.get(name).and_then(|v| v.to_str().ok())
    })
}

/// Build the rate-limit error for a Gemini `429` / `RESOURCE_EXHAUSTED`, carrying the header
/// snapshot when one was taken.
fn rate_limit_error(hints: Option<RateLimitHints>) -> LlmError {
    match hints {
        Some(hints) => LlmError::rate_limited_with_hints(hints),
        None => LlmError::rate_limited(None),
    }
}

/// Default Gemini API base URL — the `v1beta` surface, current as of this
/// writing `[CITED: ai.google.dev/api]`.
pub const GEMINI_DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

/// Default Gemini model requested when `GEMINI_MODEL` is unset.
///
/// `gemini-3.5-flash-lite`: the operator-identified current general-purpose model (its cheapest
/// current tier), measured end to end by the 2026-10-15 live probe (`registry/seeded.rs`,
/// `verified_on` 2026-10-15). It replaces `gemini-3.6-flash`, which stays available and seeded;
/// pin `GEMINI_MODEL` to keep it.
///
/// History: refreshed 2026-08-22 from `gemini-2.5-flash`, which Google retired for new users (a
/// `generateContent` call against it answers *"no longer available to new users"* while the
/// model stays in the live `GET /models` catalog, so catalog presence is not callability; only a
/// generate call distinguishes the two).
pub const GEMINI_DEFAULT_MODEL: &str = "gemini-3.5-flash-lite";

/// Curated fallback model list (D-13), returned when the live `GET /models`
/// endpoint fails, is unreachable, or returns an empty list. Gemini's
/// catalog moves fastest of the five build-list providers
/// (17-RESEARCH.md Assumptions Log A1), which is exactly why the live fetch
/// is this adapter's primary path — this list is a degrade-gracefully
/// placeholder, not an authoritative catalog.
/// **Refreshed 2026-08-22.** Both prior entries were retired for new users:
/// `gemini-2.5-flash` and `gemini-2.5-pro` each return *"no longer available
/// to new users"* on `generateContent` while still appearing in the live
/// catalog. Both replacements were verified by a live generate call on that
/// date.
///
/// **Why no `-pro` entry:** at the time of refresh no pro-family identifier
/// could be verified callable \u2014 `gemini-2.5-pro` and `gemini-3-pro-preview`
/// are retired, `gemini-3.6-pro` does not exist on `v1beta`, and
/// `gemini-pro-latest` / `gemini-3.1-pro-preview` returned a quota error on
/// the credential available. An unverified identifier is exactly the kind of
/// entry this refresh exists to remove, so the list carries only what was
/// measured.
///
/// **Re-checked 2026-10-15** by the live seed probe: all three entries answered `generateContent`
/// and are in `seeded::GEMINI`; the default comes first.
pub const GEMINI_FALLBACK_MODELS: &[&str] =
    &[GEMINI_DEFAULT_MODEL, "gemini-3.6-flash", "gemini-3.5-flash"];

/// The header name Gemini's API expects the credential on. Never the
/// documented `?key=` query-parameter alternative — see this module's
/// top-level rustdoc for why.
pub const GEMINI_API_KEY_HEADER: &str = "x-goog-api-key";

/// Narrow, named signatures that discriminate a credential-shaped Gemini
/// `400`/`INVALID_ARGUMENT` from a genuine bad-prompt one (closes WR-03,
/// `17-REVIEW.md`).
///
/// **Why this exists:** Google's documented invalid-key response is HTTP
/// `400` with `status: "INVALID_ARGUMENT"` — never a `401` — so the HTTP
/// status alone cannot separate a bad key from a bad prompt. This list, and
/// [`is_credential_failure_message`] which consults it, is that
/// discriminator. Follows this crate's established precedent for a named,
/// documented, narrowly-matched provider message signature:
/// `crate::anthropic::adapter::ANTHROPIC_USAGE_CAP_SIGNATURE`.
///
/// **Why the hyphenated header name `x-goog-api-key` is deliberately
/// absent:** an echoed request body carries the header's *name* for reasons
/// unrelated to credential validity (e.g. a proxy or gateway restating the
/// rejected header), and matching on it would send an operator to rotate a
/// working key.
/// `map_error_400_echoing_the_api_key_header_name_still_maps_to_invalid_prompt`
/// is the test that pins this. Any future addition to this list must come
/// with a corresponding over-trigger control test.
const GEMINI_CREDENTIAL_MESSAGE_SIGNATURES: &[&str] = &[
    "api key",
    "api_key",
    "unauthenticated",
    "invalid authentication",
    "credential",
];

/// Configuration for the Gemini LLM adapter.
///
/// Marked `#[non_exhaustive]` (owner decision, Phase 50 plan 50-08, option-a): build one with
/// [`GeminiConfig::new`] or [`GeminiConfig::from_env`] and set fields or call the `with_*`
/// builders, so a later field is not another break for callers.
#[derive(Clone)]
#[non_exhaustive]
pub struct GeminiConfig {
    /// API key for Gemini authentication, sent as the `x-goog-api-key`
    /// header on every request.
    pub api_key: String,
    /// Base URL for the Gemini API.
    pub base_url: String,
    /// The adapter's default model (e.g. `gemini-2.5-flash`), which selects the timeout default;
    /// each request's own `LlmRequest.model` decides what is sent on the wire.
    pub model: String,
    /// Request timeout in seconds. [`GeminiConfig::new`] and [`GeminiConfig::from_env`] default
    /// it to 300 for a seeded reasoning model and to 60 otherwise (D-13); an explicit
    /// `GEMINI_TIMEOUT_SECONDS` or field assignment always wins. A model marked `reasoning`
    /// only through [`model_overrides`](Self::model_overrides) does not change this default
    /// here: set this field in the same place.
    pub timeout_seconds: u64,
    /// Default output budget (`max_tokens`) sent when a request sets none. `None` (the default)
    /// sends no `maxOutputTokens`. The value is checked against the model's profiled maximum output
    /// exactly as a request's own `max_tokens` is (ADR-0061 (i)): above it, the request is
    /// refused before any HTTP call.
    pub max_tokens: Option<u32>,
    /// The retry budget for a retryable error. This adapter's loop counts total attempts, so the
    /// default `3` sends up to three and `0` or `1` send once. A 429 is never retried here
    /// (the Cadence decorator waits) and a permanent error is never retried.
    pub max_retries: u32,
    /// Adapter-wide context window in tokens, the operator's statement for every model. `None`
    /// (the default) keeps the compiled windows. When set it replaces the adapter's declared
    /// window, the fallback profile's window and every seeded model's window; a
    /// `context_window` in [`model_overrides`](Self::model_overrides) still wins for that model.
    /// Construction fails when it is below a model's maximum output.
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
}

/// Prints every field except the API key, which is shown as `<redacted>` (D-00a): the key is a
/// public `String`, so a derived `Debug` would put it into any log line or panic message that
/// formats the config.
impl std::fmt::Debug for GeminiConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GeminiConfig")
            .field("api_key", &RedactedKey(&self.api_key))
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("timeout_seconds", &self.timeout_seconds)
            .field("max_tokens", &self.max_tokens)
            .field("max_retries", &self.max_retries)
            .field("context_window", &self.context_window)
            .field("model_overrides", &self.model_overrides)
            .field("reasoning_effort", &self.reasoning_effort)
            .field("reasoning_fallback", &self.reasoning_fallback)
            .finish()
    }
}

config_knob_builders!(GeminiConfig);

impl GeminiConfig {
    /// Load configuration from environment variables.
    ///
    /// # Environment Variables
    /// - `GEMINI_API_KEY` (required): Gemini API key.
    /// - `GEMINI_BASE_URL` (optional): API base URL, defaults to
    ///   [`GEMINI_DEFAULT_BASE_URL`].
    /// - `GEMINI_MODEL` (optional): Default model, defaults to
    ///   [`GEMINI_DEFAULT_MODEL`].
    /// - `GEMINI_TIMEOUT_SECONDS` (optional): Request timeout, defaults to
    ///   `300` for a seeded reasoning model and `60` otherwise.
    ///
    /// # Errors
    /// Returns an error if `GEMINI_API_KEY` is absent, or another value
    /// fails to parse or validate.
    pub fn from_env() -> Result<Self, String> {
        Self::from_parts(
            env::var("GEMINI_API_KEY").ok(),
            env::var("GEMINI_BASE_URL").ok(),
            env::var("GEMINI_MODEL").ok(),
            env::var("GEMINI_TIMEOUT_SECONDS").ok(),
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
            api_key.ok_or_else(|| "GEMINI_API_KEY environment variable not set".to_string())?;
        let base_url = base_url.unwrap_or_else(|| GEMINI_DEFAULT_BASE_URL.to_string());
        let model = model.unwrap_or_else(|| GEMINI_DEFAULT_MODEL.to_string());
        let timeout_seconds = match timeout_seconds {
            Some(value) => value
                .parse()
                .map_err(|_| "Invalid GEMINI_TIMEOUT_SECONDS value".to_string())?,
            None => default_timeout_seconds(GEMINI_PROVIDER, &model, 60),
        };

        let config = Self {
            api_key,
            base_url,
            model,
            timeout_seconds,
            max_tokens: None,
            max_retries: DEFAULT_MAX_RETRIES,
            context_window: None,
            model_overrides: BTreeMap::new(),
            reasoning_effort: None,
            reasoning_fallback: None,
        };

        config.validate()?;
        Ok(config)
    }

    /// Create a configuration for `model`, authenticating with `api_key`.
    ///
    /// - `api_key`: the Gemini API key, never a URL.
    /// - `model`: the adapter's default model. It selects the timeout default (300 s for a
    ///   seeded reasoning model, 60 s otherwise); each request's own `LlmRequest.model`
    ///   decides what is sent on the wire.
    ///
    /// The endpoint is [`GEMINI_DEFAULT_BASE_URL`] (replace it with
    /// [`with_base_url`](Self::with_base_url)) and the timeout is 300 s for a seeded reasoning
    /// model and 60 s otherwise (replace it with
    /// [`with_timeout_seconds`](Self::with_timeout_seconds)). Construction is infallible: an
    /// empty key or model, or a non-`http(s)` endpoint, is refused when the adapter is built.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_llm::gemini::GeminiConfig;
    /// use paladin_llm::gemini::adapter::{GEMINI_DEFAULT_BASE_URL, GEMINI_DEFAULT_MODEL};
    ///
    /// let key = "gemini-example-key".to_string();
    /// let config = GeminiConfig::new(key, GEMINI_DEFAULT_MODEL.to_string())
    ///     .with_timeout_seconds(600);
    /// assert_eq!(config.base_url, GEMINI_DEFAULT_BASE_URL);
    /// assert_eq!(config.timeout_seconds, 600);
    /// ```
    pub fn new(api_key: String, model: String) -> Self {
        let timeout_seconds = default_timeout_seconds(GEMINI_PROVIDER, &model, 60);
        Self {
            api_key,
            base_url: GEMINI_DEFAULT_BASE_URL.to_string(),
            model,
            timeout_seconds,
            max_tokens: None,
            max_retries: DEFAULT_MAX_RETRIES,
            context_window: None,
            model_overrides: BTreeMap::new(),
            reasoning_effort: None,
            reasoning_fallback: None,
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
        if !is_http_url(&self.base_url) {
            return Err("Base URL must start with http:// or https://".to_string());
        }
        if self.model.is_empty() {
            return Err("Model cannot be empty".to_string());
        }
        Ok(())
    }
}

/// Google Gemini LLM Adapter implementing [`LlmPort`] directly against
/// Gemini's own `generateContent` protocol (D-08).
pub struct GeminiAdapter {
    client: Client,
    config: GeminiConfig,
    models_cache: OnceCell<Vec<String>>,
    /// Resolves the request model to the profile that decides what reaches the wire (ADR-0061).
    model_registry: ModelRegistry,
    /// Which (model, parameter) omissions this instance has already warned about.
    omissions: OmissionLog,
    /// Which (model, requested, effective) reasoning-effort substitutions this instance has
    /// already warned about (Phase 51, D-06).
    substitutions: SubstitutionLog,
    /// Which (model, adjustment) `response_format` changes this instance has already warned
    /// about.
    format_log: FormatLog,
    /// Which (model, check) usage findings this instance has already warned about (G-50-5a).
    usage_log: UsageLog,
}

/// What this adapter declares to the port, stated once so the adapter-wide fallback profile and
/// [`LlmPort::get_capabilities`] cannot drift apart.
fn declared_capabilities(context_window: Option<u32>) -> ProviderCapabilities {
    ProviderCapabilities {
        supports_streaming: true,
        // `LlmRequest` has no field through which a tool definition
        // could travel, and this adapter neither sends `tools`/
        // `toolConfig` nor parses a function-call part out of a
        // response (D-08).
        supports_tool_calling: false,
        supports_function_calling: false,
        // Text-only (D-08) — a truthful report of what ships, not an
        // omission. A Gemini vision adapter is a recorded, deferred
        // idea, not scope here.
        supports_vision: false,
        supports_embeddings: false,
        // The default model's seeded window (`seeded::GEMINI`, `gemini-3.5-flash-lite`, verified
        // 2026-10-15: listing `inputTokenLimit`).
        // The operator's adapter-wide window (PARITY-03) replaces it when set.
        max_context_tokens: context_window.or(seeded_window(seeded::GEMINI, GEMINI_DEFAULT_MODEL)),
        supports_system_messages: true,
        temperature_range: Some((0.0, 2.0)),
    }
}

/// The adapter-wide fallback profile, derived from what this adapter's request builder sends
/// (ADR-0061, D-05): a field it sends maps to `Range`, a field it never sends maps to `NotSent`.
///
/// - `temperature`: `Range` over the declared `temperature_range`; `top_p`: `Range(0.0, 1.0)`
///   (`generationConfig.topP`).
/// - both penalties: `NotSent` -- `generationConfig` as built here carries no penalty field.
/// - `stop`: `Supported { max: None }` -- `generationConfig.stopSequences`, which this adapter
///   already sent before the profile existed.
/// - `json_object` and `json_schema`: both true -- `responseMimeType` and `responseSchema`.
fn fallback_profile(context_window: Option<u32>) -> ModelProfile {
    let caps = declared_capabilities(context_window);
    let (low, high) = caps.temperature_range.unwrap_or((0.0, 1.0));
    let mut profile = ModelProfile::new()
        .with_temperature(ParamSupport::Range(low, high))
        .with_top_p(ParamSupport::Range(0.0, 1.0))
        .with_frequency_penalty(ParamSupport::NotSent)
        .with_presence_penalty(ParamSupport::NotSent)
        .with_stop_sequences(StopSupport::Supported { max: None })
        .with_json_object(true)
        .with_json_schema(true);
    if let Some(window) = caps.max_context_tokens {
        profile = profile.with_context_window(window);
    }
    profile
}

impl GeminiAdapter {
    /// Create a new Gemini adapter with the given configuration.
    ///
    /// # Errors
    /// Returns an error if configuration is invalid or the HTTP client
    /// cannot be created.
    pub fn new(config: GeminiConfig) -> Result<Self, LlmError> {
        config.validate().map_err(|e| {
            LlmError::AuthenticationError(format!("Invalid Gemini configuration: {e}"))
        })?;

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let key_header_value = HeaderValue::from_str(&config.api_key)
            .map_err(|e| LlmError::AuthenticationError(format!("Invalid API key format: {e}")))?;
        // The `x-goog-api-key` header — never the documented `?key=` query
        // form (T-17-24). Set once as a default header so every request
        // this client sends carries it, rather than rebuilding headers
        // per-call.
        headers.insert(GEMINI_API_KEY_HEADER, key_header_value);

        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .default_headers(headers)
            // WR-04 (`17-REVIEW.md`, T-17-52/T-17-53): `GEMINI_BASE_URL` is
            // documented and operator-settable, and the `x-goog-api-key`
            // default header above rides on every request this client
            // sends — so a `3xx` from whatever host it resolves to could
            // otherwise replay that live credential to a different,
            // attacker-influenced host. Matches every `CompatEngine`-based
            // preset's identical `Policy::none()` (T-17-18) — see this
            // module's own top-level doc for the full trust-boundary
            // rationale. A refused redirect surfaces via `map_error`'s
            // `300..=399` arm.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| LlmError::NetworkError(format!("Failed to create HTTP client: {e}")))?;

        // PARITY-03: the operator's adapter-wide window, then the per-model overrides, are laid
        // over the seeded table; either one failing construction names the provider and model.
        let model_registry = ModelRegistry::new(
            GEMINI_PROVIDER,
            seeded::GEMINI,
            fallback_profile(config.context_window),
        )
        .with_context_window(config.context_window)
        .and_then(|registry| registry.with_overrides(config.model_overrides.clone()))
        .map_err(|e| LlmError::AuthenticationError(format!("Invalid Gemini configuration: {e}")))?;

        Ok(Self {
            client,
            config,
            models_cache: OnceCell::new(),
            model_registry,
            omissions: OmissionLog::new(),
            substitutions: SubstitutionLog::new(),
            format_log: FormatLog::new(),
            usage_log: UsageLog::new(),
        })
    }

    /// The request model's profile as this adapter's wire can honour it: the effort data has
    /// whatever Gemini's body cannot carry removed ([`GEMINI_REASONING_WIRE`]), so a caller
    /// reading [`LlmPort::model_profile`] judges exactly what [`Self::build_request`] will send.
    fn wire_profile(&self, model: &str) -> ModelProfile {
        let mut profile = self.model_registry.profile(model);
        profile.effort = mask_effort(std::mem::take(&mut profile.effort), GEMINI_REASONING_WIRE);
        profile
    }

    /// Build the outgoing request body from a port-level [`LlmRequest`], plus the notes the
    /// buffered path records on its response.
    ///
    /// What reaches `generationConfig` is decided by the request model's [`ModelProfile`]
    /// (ADR-0061, D-01): [`sampling::apply`] sends `temperature`, `topP` and `stopSequences`
    /// only when the caller set them **and** the profile admits them; a set but inadmissible
    /// one is omitted and recorded, never clamped or swapped. A `max_tokens` above the
    /// profile's maximum output is refused before anything is sent. The `response_format`
    /// follows the profile through [`sampling::apply_response_format`].
    ///
    /// `PromptType::System` (and `PromptType::Text` with
    /// [`PromptRole::System`]) map to the top-level `systemInstruction`
    /// field, never to a `contents[]` entry — Gemini's API has no `system`
    /// role inside `contents[]`. `PromptType::Function` (and
    /// `PromptType::Text` with [`PromptRole::Function`]) have no supported
    /// mapping — Gemini's function surface is out of scope for this adapter
    /// (D-08) — and return [`LlmError::InvalidPrompt`] naming the
    /// unsupported prompt type rather than silently dropping it.
    fn build_request(
        &self,
        request: &LlmRequest,
    ) -> Result<(GeminiRequest, RequestNotes), LlmError> {
        let mut system_instruction = None;
        let mut contents = Vec::new();

        match &request.prompt.entity.data.prompt_type {
            PromptType::System(system_prompt) => {
                system_instruction = Some(GeminiSystemInstruction {
                    parts: vec![GeminiPart {
                        text: system_prompt.instructions.clone(),
                    }],
                });
            }
            PromptType::User(user_prompt) => {
                contents.push(GeminiContent {
                    role: "user".to_string(),
                    parts: vec![GeminiPart {
                        text: crate::turn::user_turn_text(user_prompt),
                    }],
                });
            }
            PromptType::Assistant(assistant_prompt) => {
                contents.push(GeminiContent {
                    role: "model".to_string(),
                    parts: vec![GeminiPart {
                        text: assistant_prompt.response.clone(),
                    }],
                });
            }
            PromptType::Text(text_prompt) => match &text_prompt.role {
                PromptRole::System => {
                    system_instruction = Some(GeminiSystemInstruction {
                        parts: vec![GeminiPart {
                            text: text_prompt.content.clone(),
                        }],
                    });
                }
                PromptRole::User => {
                    contents.push(GeminiContent {
                        role: "user".to_string(),
                        parts: vec![GeminiPart {
                            text: text_prompt.content.clone(),
                        }],
                    });
                }
                PromptRole::Assistant => {
                    contents.push(GeminiContent {
                        role: "model".to_string(),
                        parts: vec![GeminiPart {
                            text: text_prompt.content.clone(),
                        }],
                    });
                }
                PromptRole::Function => {
                    return Err(LlmError::InvalidPrompt(
                        "Gemini does not support function-role prompts — LlmRequest has no \
                         tool-definition field for this adapter to carry (D-08)"
                            .to_string(),
                    ));
                }
            },
            PromptType::Function(_) => {
                return Err(LlmError::InvalidPrompt(
                    "Gemini does not support function prompts — LlmRequest has no \
                     tool-definition field for this adapter to carry (D-08)"
                        .to_string(),
                ));
            }
        }

        let params = &request.prompt.entity.data.parameters;
        let profile = self.wire_profile(&request.model);

        // The caller's value, else the configured default; either is held to the model's
        // maximum output before anything is sent.
        let max_tokens = params.max_tokens.or(self.config.max_tokens);
        sampling::check_max_tokens(GEMINI_PROVIDER, &request.model, &profile, max_tokens)?;

        // Phase 51 (REASON-02, REASON-06): an explicit effort is sent in the form the model's
        // profile states or refused here, before any HTTP call and before any retry loop. The
        // cap a budget is held below is `maxOutputTokens` when one is set.
        // The request's own effort and fallback win; otherwise the provider-level default applies
        // (D-00e), judged here against this request's own model.
        let in_force = reasoning::in_force(
            GEMINI_PROVIDER,
            request.reasoning,
            request.reasoning_fallback,
            self.config.reasoning_effort,
            self.config.reasoning_fallback,
        );
        let resolved = reasoning::admit(
            GEMINI_PROVIDER,
            &request.model,
            &profile,
            in_force.effort,
            in_force.fallback,
            max_tokens,
            in_force.source_key.as_deref(),
            &self.substitutions,
        )?;
        let thinking_config = match &resolved {
            Some(resolved) => map_reasoning(&request.model, resolved)?,
            None => None,
        };

        let applied = sampling::apply(&profile, params);
        let params_omitted = self
            .omissions
            .record(GEMINI_PROVIDER, &request.model, &applied);

        let (effective_format, adjustment) =
            sampling::apply_response_format(&profile, request.response_format.as_ref());
        let format_adjusted = self
            .format_log
            .record(GEMINI_PROVIDER, &request.model, adjustment);

        let unprofiled_model = self
            .model_registry
            .note_unprofiled(&request.model)
            .then(|| request.model.clone());

        // RT-FR-17, D-28 / ADR-0061 (i): `responseMimeType` is set for either
        // `ResponseFormat` variant the profile admits; `responseSchema` carries the schema
        // value only for `JsonSchema`. `ResponseFormat` is `#[non_exhaustive]`, but
        // `apply_response_format` already drops a variant it does not know, so what reaches
        // this match is one of the two this adapter can express.
        let (response_mime_type, response_schema) = match &effective_format {
            Some(ResponseFormat::JsonSchema { schema, .. }) => {
                (Some("application/json".to_string()), Some(schema.clone()))
            }
            Some(_) => (Some("application/json".to_string()), None),
            None => (None, None),
        };

        let generation_config = GeminiGenerationConfig {
            temperature: applied.temperature,
            max_output_tokens: max_tokens,
            top_p: applied.top_p,
            top_k: None,
            stop_sequences: applied.stop,
            candidate_count: None,
            response_mime_type,
            response_schema,
            thinking_config,
        };
        let generation_config = if generation_config.temperature.is_some()
            || generation_config.max_output_tokens.is_some()
            || generation_config.top_p.is_some()
            || generation_config.stop_sequences.is_some()
            || generation_config.response_mime_type.is_some()
            || generation_config.thinking_config.is_some()
        {
            Some(generation_config)
        } else {
            None
        };

        let body = GeminiRequest {
            contents,
            system_instruction,
            generation_config,
        };
        let notes = RequestNotes {
            params_omitted,
            format_adjusted,
            unprofiled_model,
            reasoning_requested: resolved.map(|r| r.requested.to_string()),
            reasoning_effective: resolved.map(|r| r.effective.to_string()),
        };
        Ok((body, notes))
    }

    /// Parse a well-formed Gemini `generateContent` response into an
    /// [`LlmResponse`].
    ///
    /// Fails with [`LlmError::EmptyCompletion`] rather than ever returning
    /// `Ok` with empty content — an empty-string success is
    /// indistinguishable from a valid empty answer to every downstream
    /// caller. Two conditions:
    ///
    /// 1. `candidates` is empty.
    /// 2. The extracted content trims to empty on any finish reason that is
    ///    not a blocked-content finish (D-11a): `STOP` and `MAX_TOKENS` both
    ///    qualify, and the payload carries the advice that fits (a larger
    ///    `max_tokens` for `MAX_TOKENS`). The decision is the shared
    ///    [`classify_completion`], the same function every other adapter
    ///    calls, so the rule cannot drift.
    ///
    /// A blocked-content finish (`SAFETY`, `RECITATION`, `BLOCKLIST`,
    /// `PROHIBITED_CONTENT`, `SPII`) maps to [`FinishReason::ContentFilter`]
    /// and stays `Ok` even with no text (D-09): a blocked output is a finish
    /// reason the caller inspects, never reported as a token-budget problem.
    /// Gemini has no refusal signal distinct from those finishes (A2-50-13),
    /// so no refusal text is passed to the guard.
    fn parse_response(
        &self,
        request_id: Uuid,
        model: &str,
        response: GeminiResponse,
    ) -> Result<LlmResponse, LlmError> {
        // G-50-5a: usage is mapped BEFORE the guard (and before the no-candidates check), so a
        // call Gemini billed but that is classified as empty carries the same folded usage a
        // success would have. A body with no `usageMetadata` bills nothing, never a fabricated
        // zero (D-00c); a success without one still maps the default, as before.
        let mapped = response.usage_metadata.as_ref().map(map_gemini_usage);
        let billed = || mapped.as_ref().map(|(usage, _)| usage.clone());

        let Some(candidate) = response.candidates.first() else {
            return Err(bill(
                LlmError::empty_completion("Gemini response contained no candidates".to_string()),
                model,
                billed(),
            ));
        };

        let content = candidate_text(candidate);
        let finish_reason = map_finish_reason(candidate.finish_reason.as_deref());

        classify_completion(
            GEMINI_PROVIDER,
            model,
            &content,
            &finish_reason,
            None,
            &self.config.api_key,
        )
        .map_err(|err| bill(err, model, billed()))?;

        let (usage, vendor_total) =
            mapped.unwrap_or_else(|| map_gemini_usage(&GeminiUsageMetadata::default()));
        // G-50-5a: Gemini's own `totalTokenCount` is the check on the mapping -- a disagreement
        // is warned once and recorded, never written over the mapped total (D-02).
        let mut metadata = HashMap::new();
        if let Some(value) = self
            .usage_log
            .record(GEMINI_PROVIDER, model, &usage, vendor_total)
        {
            metadata.insert(USAGE_UNRECONCILED_METADATA_KEY.to_string(), value);
        }

        Ok(LlmResponse {
            id: Uuid::new_v4(),
            request_id,
            model: model.to_string(),
            content,
            finish_reason,
            usage,
            cost: None,
            created_at: Utc::now(),
            metadata,
            function_call: None,
            reasoning: None,
        })
    }

    /// Map a Gemini API error response to [`LlmError`].
    ///
    /// Switches on **both** the HTTP status and the JSON `error.status` RPC
    /// string, because Google's error envelope carries an RPC-style status
    /// string alongside the HTTP code — HTTP 429 alone is ambiguous
    /// between a transient rate limit and a hard quota exhaustion on
    /// Google's APIs generally. The extracted message is redacted
    /// (credential-shaped tokens stripped) before it is bounded to a
    /// diagnostic excerpt — redact-then-bound, never the reverse, per
    /// `crate::redaction`'s own ordering discipline (T-17-25).
    ///
    /// ## Credential-failure classification (WR-03, `17-REVIEW.md`)
    ///
    /// A `401` or `403` is **always** classified as
    /// [`LlmError::AuthenticationError`], regardless of the RPC status
    /// string and whether or not the envelope parses at all. The
    /// alternative is a fall-through to the shared helper's generic
    /// [`LlmError::ProviderError`] (retried by this adapter), and
    /// [`Self::execute_with_retry`]'s non-retryable set already halts on
    /// `AuthenticationError` — so misclassifying an unrecognised auth
    /// failure would re-transmit a live `x-goog-api-key` credential to an
    /// endpoint that has already rejected it, up to `max_retries` times.
    ///
    /// A `400`/`INVALID_ARGUMENT` is ambiguous between a bad key and a bad
    /// prompt: Google commonly reports an invalid or malformed API key as
    /// `400`/`INVALID_ARGUMENT`, not a `401`. The named, narrow
    /// [`GEMINI_CREDENTIAL_MESSAGE_SIGNATURES`] list —
    /// [`is_credential_failure_message`] — is how the two are separated;
    /// every other `400`/`INVALID_ARGUMENT` stays [`LlmError::InvalidPrompt`].
    ///
    /// The asymmetry decides the default: a bad prompt misread as a bad key
    /// costs the operator a wasted credential rotation, while a bad key
    /// misread as a bad prompt costs three extra transmissions of a live
    /// secret. The discriminator therefore errs narrow, and the unmatched
    /// case stays `InvalidPrompt`.
    ///
    /// ## `RESOURCE_EXHAUSTED` disposition — a documented assumption
    ///
    /// Gemini's `RESOURCE_EXHAUSTED` RPC status covers both a transient
    /// per-minute rate limit and a hard billing-quota exhaustion, and this
    /// adapter cannot distinguish the two without a live key
    /// (17-RESEARCH.md Assumptions Log A4, Open Question 2). This maps
    /// `RESOURCE_EXHAUSTED` conservatively to [`LlmError::RateLimitExceeded`]
    /// (retryable): retrying a true quota exhaustion merely burns the
    /// bounded retry budget, whereas mapping a transient rate limit to the
    /// non-retryable [`LlmError::UsageLimitExceeded`] would fail a request
    /// that would otherwise have succeeded — the asymmetry decides it.
    /// Verification path: the `live-api-tests` feature with a real
    /// `GEMINI_API_KEY` (17-CONTEXT.md D-15 leaves this available and
    /// deliberately unused this phase).
    ///
    /// A thin wrapper over [`Self::map_error_with_hints`], kept so the status-mapping unit
    /// tests read the same as before PACE-01. Production paths call the hints variant.
    #[cfg(test)]
    fn map_error(&self, status: u16, body: &str) -> LlmError {
        self.map_error_with_hints(status, body, None)
    }

    /// Map a non-2xx Gemini response to [`LlmError`], carrying the parsed rate-limit headers
    /// of a `429` (PACE-01). Both the `429` arm and the `RESOURCE_EXHAUSTED` arm build the
    /// rate-limit error from `hints`; see [`Self::map_error`] for every other rule.
    fn map_error_with_hints(
        &self,
        status: u16,
        body: &str,
        hints: Option<RateLimitHints>,
    ) -> LlmError {
        let envelope: Option<GeminiErrorEnvelope> = serde_json::from_str(body).ok();
        let rpc_status = envelope.as_ref().and_then(|e| e.error.status.as_deref());
        let raw_message = envelope
            .as_ref()
            .map(|e| e.error.message.as_str())
            .unwrap_or(body);

        // Redact BEFORE bounding — bounding first could slice a secret in
        // half at the truncation boundary and leak the surviving prefix.
        let redacted_message = redact_credentials(raw_message, &self.config.api_key);
        let excerpt = bounded_excerpt(&redacted_message, RESPONSE_EXCERPT_CHAR_BUDGET);

        match status {
            // WR-03: unconditional — whatever the RPC status string is, and
            // whether or not the envelope parses at all. This is the arm
            // that makes `execute_with_retry`'s existing non-retryable set
            // (which already halts on `AuthenticationError`) correct: a
            // misclassified auth failure would otherwise be retried,
            // re-transmitting a live credential to an endpoint that has
            // already rejected it.
            401 | 403 => {
                LlmError::AuthenticationError(format!("Gemini authentication failed: {excerpt}"))
            }
            // WR-03: Google's documented invalid-key shape is HTTP `400`
            // with `status: "INVALID_ARGUMENT"`, not a `401` — so the HTTP
            // status alone cannot separate a bad key from a bad prompt.
            // `is_credential_failure_message` reads `raw_message` (not
            // `excerpt`) because `bounded_excerpt` truncates at
            // `RESPONSE_EXCERPT_CHAR_BUDGET` and a signature appearing past
            // that boundary would otherwise be missed — but only `excerpt`
            // (redacted, then bounded) is ever emitted below, never
            // `raw_message`.
            400 if rpc_status == Some("INVALID_ARGUMENT")
                && is_credential_failure_message(raw_message) =>
            {
                LlmError::AuthenticationError(format!(
                    "Gemini rejected the configured credential: {excerpt}"
                ))
            }
            400 if rpc_status == Some("INVALID_ARGUMENT") => LlmError::InvalidPrompt(excerpt),
            404 if rpc_status == Some("NOT_FOUND") => LlmError::ModelNotAvailable(excerpt),
            429 => rate_limit_error(hints),
            _ if rpc_status == Some("RESOURCE_EXHAUSTED") => rate_limit_error(hints),
            // WR-04 (`17-REVIEW.md`, T-17-52): this client's redirect
            // policy is `none` (see `GeminiAdapter::new`), so a `3xx`
            // response is never followed — it arrives here as an ordinary
            // non-success status instead. Named explicitly, mirroring
            // `CompatEngine::map_error`'s identical arm, so the operator
            // whose previously-working `GEMINI_BASE_URL` now fails gets an
            // actionable message rather than an opaque one.
            //
            // The VARIANT is the one the shared helper would choose for a
            // `3xx` (`ProviderError`, classified Permanent by value —
            // FT-FR-01); only the message is enriched. It is outside
            // `execute_with_retry`'s non-retryable set, so a redirecting
            // host is still retried up to `max_retries` before this
            // surfaces — the accepted cost recorded as T-17-54. The "no new
            // variant" rationale that once forced this onto
            // `ProcessingError` is superseded by plan 25-02's
            // `ProviderError`.
            300..=399 => LlmError::ProviderError {
                provider: GEMINI_PROVIDER.to_string(),
                status,
                message: format!(
                    "the configured GEMINI_BASE_URL responded with a redirect (HTTP {status}), \
                     which this client refuses to follow because doing so would forward the \
                     x-goog-api-key credential header to a different, potentially \
                     attacker-influenced host. Correct the GEMINI_BASE_URL setting to point \
                     directly at the intended endpoint. Response excerpt: {excerpt}"
                ),
            },
            // Phase 25 (FT-FR-01, D-03): every remaining status goes through
            // the crate-wide helper, which redacts before bounding and
            // carries the status as a typed field. The body handed over is
            // the envelope's extracted `error.message` (not the raw JSON)
            // with Google's RPC status string appended, so the diagnostic
            // keeps both without the helper needing to know the envelope.
            _ => {
                let envelope_text = match rpc_status {
                    Some(rpc) => format!("{raw_message} (status={rpc})"),
                    None => raw_message.to_string(),
                };
                map_http_status_with_hints(
                    GEMINI_PROVIDER,
                    status,
                    &envelope_text,
                    &self.config.api_key,
                    hints,
                )
            }
        }
    }

    /// Execute a Gemini operation with retry-with-backoff.
    ///
    /// This is a separate implementation from
    /// [`crate::compat::CompatEngine`]'s retry loop, because Gemini is a
    /// separate, bespoke adapter (D-08) — but the **non-retryable error
    /// set**, not the loop shape, is what must stay in lockstep with the
    /// rest of the crate: `AuthenticationError`/`InvalidPrompt` (fix
    /// configuration or prompt, retrying is pointless),
    /// `EmptyCompletion` (a retried request is byte-for-byte identical, so
    /// a no-text truncation reproduces deterministically), and
    /// `UsageLimitExceeded` (a usage cap resets on a provider-side billing
    /// schedule, not a short window — retrying here would burn the bounded
    /// retry budget before a higher-level breaker ever sees the error).
    ///
    /// **A 429 is surfaced on the first attempt (Phase 43 D-02).**
    /// `RateLimitExceeded` is in the same set, but for a different reason: a
    /// 429 is the one error whose cure is waiting, and the Cadence decorator
    /// above this adapter owns waiting with a gate shared by every caller of
    /// the same provider and model. Retrying it here would re-send into a
    /// limit the provider just announced and hide the thrash below the
    /// decorator; the error carries the provider's own `Retry-After` for the
    /// decorator to honour. Network, timeout, 5xx and decode retries are
    /// unchanged.
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

                    if matches!(
                        e,
                        LlmError::AuthenticationError(_)
                            | LlmError::InvalidPrompt(_)
                            | LlmError::EmptyCompletion { .. }
                            | LlmError::Refusal { .. }
                            | LlmError::UsageLimitExceeded { .. }
                            // D-02: surface the first 429; the Cadence decorator waits.
                            | LlmError::RateLimitExceeded { .. }
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

    /// Fetch the live model list from `GET {base_url}/models`.
    ///
    /// Google returns fully-qualified names of the form
    /// `models/gemini-2.5-flash`; the leading `models/` segment is
    /// stripped before returning, since that is the bare form callers pass
    /// to `model`.
    async fn fetch_live_models(&self) -> Result<Vec<String>, LlmError> {
        let url = format!("{}/models", self.config.base_url);

        let response = self.client.get(&url).send().await.map_err(|e| {
            if e.is_timeout() {
                LlmError::Timeout(format!(
                    "Gemini model list request timed out after {} seconds",
                    self.config.timeout_seconds
                ))
            } else {
                LlmError::NetworkError(format!("Failed to fetch Gemini model list: {e}"))
            }
        })?;

        let status = response.status().as_u16();
        if !response.status().is_success() {
            let hints = snapshot_rate_limit_hints(response.status(), response.headers());
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(self.map_error_with_hints(status, &body, hints));
        }

        let body = response.text().await.map_err(|e| {
            LlmError::NetworkError(format!("Failed to read Gemini model list body: {e}"))
        })?;

        let parsed: GeminiModelsResponse = serde_json::from_str(&body).map_err(|e| {
            LlmError::ProcessingError(format!(
                "Failed to parse Gemini model list (schema mismatch: {e}) — body excerpt: {}",
                diagnostic_excerpt(&body, &self.config.api_key)
            ))
        })?;

        Ok(parsed
            .models
            .into_iter()
            .map(|entry| match entry.name.strip_prefix("models/") {
                Some(stripped) => stripped.to_string(),
                None => entry.name,
            })
            .collect())
    }

    /// Resolve the model list: live on first call, memoized for this
    /// adapter's lifetime (D-13/D-14). Falls back to
    /// [`GEMINI_FALLBACK_MODELS`] on any failure or an empty live response
    /// — logged at `debug`, never `error`, since offline is a supported
    /// state. `tokio::sync::OnceCell` ensures exactly one fetch even under
    /// concurrent callers.
    async fn available_models(&self) -> Vec<String> {
        self.models_cache
            .get_or_init(|| async {
                match self.fetch_live_models().await {
                    Ok(models) if !models.is_empty() => models,
                    Ok(_) => {
                        log::debug!(
                            "Gemini live model list was empty; falling back to curated list \
                             (not authoritative)"
                        );
                        GEMINI_FALLBACK_MODELS
                            .iter()
                            .map(|s| s.to_string())
                            .collect()
                    }
                    Err(e) => {
                        log::debug!(
                            "Gemini live model list fetch failed ({e}); falling back to \
                             curated list (not authoritative)"
                        );
                        GEMINI_FALLBACK_MODELS
                            .iter()
                            .map(|s| s.to_string())
                            .collect()
                    }
                }
            })
            .await
            .clone()
    }
}

#[async_trait]
impl LlmPort for GeminiAdapter {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        validate_model_identifier(&request.model)?;
        let (gemini_request, notes) = self.build_request(&request)?;
        let url = format!(
            "{}/models/{}:generateContent",
            self.config.base_url, request.model
        );

        let operation = || async {
            let response = self
                .client
                .post(&url)
                .json(&gemini_request)
                .send()
                .await
                .map_err(|e| {
                    if e.is_timeout() {
                        LlmError::Timeout(format!(
                            "Gemini request timed out after {} seconds",
                            self.config.timeout_seconds
                        ))
                    } else {
                        LlmError::NetworkError(format!("Gemini request failed: {e}"))
                    }
                })?;

            let status = response.status().as_u16();

            if !response.status().is_success() {
                // Snapshot the 429 headers BEFORE `.text()` consumes the response
                // (research Pitfall 2).
                let hints = snapshot_rate_limit_hints(response.status(), response.headers());
                let body = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                return Err(self.map_error_with_hints(status, &body, hints));
            }

            // Read the body to a String first, deserialize separately — a
            // transport failure and a schema mismatch must remain
            // distinguishable in the error message.
            let body = response.text().await.map_err(|e| {
                LlmError::NetworkError(format!("Failed to read Gemini response body: {e}"))
            })?;

            let gemini_response: GeminiResponse = serde_json::from_str(&body).map_err(|e| {
                LlmError::ProcessingError(format!(
                    "Failed to parse Gemini response (schema mismatch: {e}) — body excerpt: {}",
                    diagnostic_excerpt(&body, &self.config.api_key)
                ))
            })?;

            self.parse_response(request.id, &request.model, gemini_response)
        };

        let mut response = self
            .execute_with_retry(operation, self.config.max_retries)
            .await?;
        // The request-time notes (ADR-0061 visibility rule) ride on the successful response,
        // beside any usage finding `parse_response` recorded.
        response.metadata.extend(notes.into_metadata());
        Ok(response)
    }

    /// Generate a streaming completion. POSTs with `alt=sse` and assembles
    /// SSE frames via `parse_sse_lines`.
    ///
    /// ## Retry (WR-04, `17-REVIEW.md`)
    ///
    /// Only the connection-opening POST is retried — through the same
    /// `execute_with_retry` helper and the same `max_retries`
    /// literal (`3`) [`generate`](Self::generate) passes — so a transient
    /// failure opening the stream is retried exactly as many times as
    /// `generate()` retries the identical failure. The same non-retryable
    /// set applies: an authentication failure, an invalid prompt, an
    /// already-empty completion or a usage-limit rejection is attempted
    /// exactly once, never replaying a live `x-goog-api-key` credential to
    /// an endpoint that has already rejected it.
    ///
    /// Once the response is opened, `.bytes_stream()` is consumed exactly
    /// once, **outside** the retry loop: the byte stream itself is never
    /// re-read or re-opened, so a caller can never observe a duplicated or
    /// reordered delta as a result of this retry.
    ///
    /// **Cost:** each retried open re-sends the caller's entire prompt to
    /// Gemini, so a transient failure can bill the prompt more than once.
    /// The non-retryable set above exists precisely so a failure the
    /// provider has already answered definitively is never retried.
    ///
    /// ## Empty output (D-11a)
    ///
    /// A stream cannot be classified until it ends, so the text is
    /// accumulated as frames pass and the shared `classify_completion`
    /// rule runs on the frame that carries `finishReason` (the last
    /// candidate frame). A stream that reaches it with no non-blank text
    /// and a finish reason other than a blocked-content one ends with
    /// `Err(`[`LlmError::EmptyCompletion`]`)`; a blocked-content finish
    /// (`SAFETY` and its siblings) stays an `Ok` terminal chunk with
    /// [`FinishReason::ContentFilter`] (D-09). Frames already yielded stay
    /// yielded; only the terminal item changes. That error item carries the
    /// finish frame's `usageMetadata` as its billed usage
    /// ([`LlmError::billed_usage`], G-50-5a) on the request model, or `None`
    /// when the frame reported none.
    async fn generate_stream(
        &self,
        request: LlmRequest,
    ) -> Result<Box<dyn Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError> {
        validate_model_identifier(&request.model)?;
        // ADR-0061 (h), owner reading R-1 (option-b): the omissions, format changes and
        // unprofiled model `build_request` `warn!`s about are also stamped on the stream's
        // terminal (finish-reason) chunk, only that chunk, as the buffered path stamps
        // `LlmResponse.metadata`.
        let (gemini_request, notes) = self.build_request(&request)?;
        let stream_metadata = notes.into_metadata();
        let url = format!(
            "{}/models/{}:streamGenerateContent",
            self.config.base_url, request.model
        );

        // Only the connection-opening POST is retried — the byte stream is
        // deliberately consumed OUTSIDE this closure, once, after the retry
        // loop returns below. Re-opening a stream whose first chunks a
        // caller has already consumed would deliver the same tokens twice
        // with no marker (WR-04, `17-REVIEW.md`; T-17-71).
        let operation = || async {
            // `alt=sse` is mandatory — without it Gemini returns a raw JSON
            // array rather than SSE framing, and the line-oriented parse
            // loop below would silently produce nothing.
            let response = self
                .client
                .post(&url)
                .query(&[("alt", "sse")])
                .json(&gemini_request)
                .send()
                .await
                .map_err(|e| {
                    if e.is_timeout() {
                        LlmError::Timeout(format!(
                            "Gemini stream request timed out after {} seconds",
                            self.config.timeout_seconds
                        ))
                    } else {
                        LlmError::NetworkError(format!("Gemini stream request failed: {e}"))
                    }
                })?;

            if !response.status().is_success() {
                let status = response.status().as_u16();
                // Same header snapshot as the buffered path, taken before the body read.
                let hints = snapshot_rate_limit_hints(response.status(), response.headers());
                let body = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                return Err(self.map_error_with_hints(status, &body, hints));
            }

            Ok(response)
        };

        let response = self
            .execute_with_retry(operation, self.config.max_retries)
            .await?;

        let model = request.model.clone();
        let api_key = self.config.api_key.clone();
        let mut completion = StreamCompletion::default();
        // G-50-5a: the terminal frame's `totalTokenCount`, reconciled on the `Ok` terminal chunk
        // against the usage mapped from the same frame. A clone shares this instance's log.
        let usage_log = self.usage_log.clone();
        let mut held_vendor_total: Option<u32> = None;

        // A network read can end inside a `data:` line (or inside a multi-byte character), so the
        // bytes are reassembled into whole lines before any line is parsed (CR-02).
        let mut sse = SseLines::default();
        let stream = response
            .bytes_stream()
            .map(Some)
            .chain(futures::stream::iter([None]))
            .flat_map(move |chunk_option| {
                // `None` is the end of the body: it releases a final line that had no newline.
                let chunk_result = match chunk_option {
                    Some(read) => read.map(|bytes| sse.push(&bytes)),
                    None => Ok(sse.finish()),
                };
                let items: Vec<Result<StreamingResponse, LlmError>> = match chunk_result {
                    Ok(lines) => {
                        let mut items = parse_sse_lines(&lines, &mut held_vendor_total);
                        classify_stream_items(&mut items, &mut completion, &model, &api_key);
                        reconcile_terminal_usage(
                            &mut items,
                            &usage_log,
                            &model,
                            &mut held_vendor_total,
                        );
                        stamp_terminal_metadata(&mut items, &stream_metadata);
                        items
                    }
                    Err(e) => vec![Err(LlmError::NetworkError(format!(
                        "Gemini stream error: {e}"
                    )))],
                };

                futures::stream::iter(items)
            });

        Ok(Box::new(stream))
    }

    async fn validate_model(&self, model: &str) -> Result<bool, LlmError> {
        Ok(self.available_models().await.iter().any(|m| m == model))
    }

    async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
        Ok(self.available_models().await)
    }

    fn get_provider_name(&self) -> &'static str {
        GEMINI_PROVIDER
    }

    fn get_capabilities(&self) -> ProviderCapabilities {
        declared_capabilities(self.config.context_window)
    }

    fn model_profile(&self, model: &str) -> Option<ModelProfile> {
        Some(self.wire_profile(model))
    }

    /// The pre-judging twin of the send-time `in_force` + `admit` in [`Self::build_request`]:
    /// same precedence over this adapter's `reasoning_effort` / `reasoning_fallback`, same
    /// [`Self::wire_profile`], same verdict (D-09). The per-model wire form chosen in
    /// `map_reasoning` only ever refuses what `admit` already refused.
    fn check_reasoning(
        &self,
        model: &str,
        effort: Option<ReasoningEffort>,
        fallback: Option<ReasoningFallback>,
    ) -> Result<(), LlmError> {
        reasoning::check(
            GEMINI_PROVIDER,
            model,
            || self.wire_profile(model),
            effort,
            fallback,
            self.config.reasoning_effort,
            self.config.reasoning_fallback,
        )
    }
}

// ── Gemini API request/response types ───────────────────────────────────

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(rename = "systemInstruction", skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiSystemInstruction>,
    #[serde(rename = "generationConfig", skip_serializing_if = "Option::is_none")]
    generation_config: Option<GeminiGenerationConfig>,
}

#[derive(Debug, Serialize)]
struct GeminiContent {
    /// `"user"` or `"model"` — Gemini has no `"system"` role inside
    /// `contents[]`.
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiPart {
    #[serde(default)]
    text: String,
}

#[derive(Debug, Serialize)]
struct GeminiSystemInstruction {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(rename = "maxOutputTokens", skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
    #[serde(rename = "topP", skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(rename = "topK", skip_serializing_if = "Option::is_none")]
    top_k: Option<u32>,
    #[serde(rename = "stopSequences", skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
    #[serde(rename = "candidateCount", skip_serializing_if = "Option::is_none")]
    candidate_count: Option<u32>,
    /// `LlmRequest.response_format`'s native Gemini mapping (RT-FR-17,
    /// D-28): `"application/json"` whenever the caller set any
    /// `ResponseFormat` variant. Omitted entirely when the caller sets no
    /// hint, keeping the body byte-identical to a pre-0.10 request (X-03).
    #[serde(rename = "responseMimeType", skip_serializing_if = "Option::is_none")]
    response_mime_type: Option<String>,
    /// `ResponseFormat::JsonSchema`'s schema value, carried verbatim
    /// (RT-FR-17, D-28) — Gemini's own schema-carrying native JSON mode.
    /// Absent for `ResponseFormat::JsonObject` (no schema to carry) and
    /// absent entirely when the caller sets no hint.
    #[serde(rename = "responseSchema", skip_serializing_if = "Option::is_none")]
    response_schema: Option<serde_json::Value>,
    /// `LlmRequest.reasoning` on the wire (Phase 51): a `thinkingLevel` or a `thinkingBudget`,
    /// never both. Omitted when no effort is set or the effort is `Auto`, keeping an unset
    /// request's body byte-identical (D-00i). Placed last so the existing field order is
    /// untouched. Never carries `includeThoughts` (D-00g).
    #[serde(rename = "thinkingConfig", skip_serializing_if = "Option::is_none")]
    thinking_config: Option<GeminiThinkingConfig>,
}

/// `generationConfig.thinkingConfig`: exactly one of the two members is set.
#[derive(Debug, Serialize)]
struct GeminiThinkingConfig {
    #[serde(rename = "thinkingLevel", skip_serializing_if = "Option::is_none")]
    thinking_level: Option<&'static str>,
    #[serde(rename = "thinkingBudget", skip_serializing_if = "Option::is_none")]
    thinking_budget: Option<u32>,
}

/// What Gemini's `generateContent` body can carry of a profile's effort data (Phase 51): a
/// `thinkingLevel`, a `thinkingBudget` (and `0` for `off`), and `auto` as the absence of both.
///
/// Confirmed against the dated probe (plan 51-12, D-04; owner decision of 2026-10-22 at plan
/// 51-11's checkpoint): `51-EFFORT-PROBE.md`'s verdict table marks Gemini's `thinking_level`
/// confirmed (the invalid value drew 400 on 12 of 12 models, so the field is parsed; the highest
/// accepted level reports more reasoning tokens than the lowest on 5 of 5 comparable models), and
/// `thinkingBudget` parsed (400 on 12 of 12). The documented capabilities therefore stand; they
/// are not `WireCaps::NONE`. No seeded row carries a budget span (the probe file holds no
/// vendor-documented bound), so a budget refuses on every seeded model until an operator
/// override states one.
const GEMINI_REASONING_WIRE: WireCaps = WireCaps {
    levels: true,
    off: true,
    auto: true,
    budget: true,
};

/// The `thinkingLevel` spelling of a level, kept in one place so it can change for the whole
/// adapter. Lower-case, confirmed by the dated probe (research A2, plan 51-12): every seeded level
/// was accepted in lower-case on 12 of 12 models, and the upper-case `HIGH` was accepted too, so
/// the lower-case spelling stays. Pinned by a wire-body test.
fn thinking_level_spelling(effort: ReasoningEffort) -> Option<&'static str> {
    reasoning::level_spelling(effort)
}

/// Turn an admitted effort into Gemini's `thinkingConfig`, or `None` when nothing is sent
/// (`Auto`, or `Off` on a model that does not reason).
///
/// A level and a budget never appear together. `Off` is a zero budget.
///
/// # Errors
///
/// [`LlmError::UnsupportedParameter`] for an effort this wire cannot carry. The profile masking
/// and `admit` already refuse those, so this is the last line of defence: it can never put a
/// field on the wire that the request did not ask for.
fn map_reasoning(
    model: &str,
    resolved: &ResolvedEffort,
) -> Result<Option<GeminiThinkingConfig>, LlmError> {
    if !resolved.on_wire {
        return Ok(None);
    }
    if let Some(level) = thinking_level_spelling(resolved.effective) {
        return Ok(Some(GeminiThinkingConfig {
            thinking_level: Some(level),
            thinking_budget: None,
        }));
    }
    match resolved.effective {
        ReasoningEffort::Off => Ok(Some(GeminiThinkingConfig {
            thinking_level: None,
            thinking_budget: Some(0),
        })),
        // The vendor's own default depth: nothing to send.
        ReasoningEffort::Auto => Ok(None),
        ReasoningEffort::Budget(tokens) => Ok(Some(GeminiThinkingConfig {
            thinking_level: None,
            thinking_budget: Some(tokens),
        })),
        other => Err(LlmError::unsupported_parameter(
            "reasoning_effort",
            other.to_string(),
            GEMINI_PROVIDER,
            model,
            "this adapter has no wire form for that effort",
        )),
    }
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    #[serde(default)]
    candidates: Vec<GeminiCandidate>,
    #[serde(default, rename = "usageMetadata")]
    usage_metadata: Option<GeminiUsageMetadata>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    #[serde(default)]
    content: Option<GeminiResponseContent>,
    #[serde(default, rename = "finishReason")]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponseContent {
    #[serde(default)]
    parts: Vec<GeminiPart>,
    /// Present on the wire (`"model"`) but not read by this adapter — kept
    /// for documentation of the full response shape, not dead weight this
    /// adapter depends on.
    #[allow(dead_code)]
    #[serde(default)]
    role: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct GeminiUsageMetadata {
    #[serde(default, rename = "promptTokenCount")]
    prompt_token_count: u32,
    #[serde(default, rename = "candidatesTokenCount")]
    candidates_token_count: u32,
    /// Gemini's own reported total (G-50-5a). Parsed and handed to the
    /// reconciliation ([`crate::usage_reconcile`]), never copied:
    /// `TokenUsage::new` derives `total_tokens` as `prompt_tokens +
    /// completion_tokens` (D-02), and a disagreement is recorded under
    /// [`USAGE_UNRECONCILED_METADATA_KEY`]. `None` when the key is absent --
    /// never a fabricated `0`.
    #[serde(default, rename = "totalTokenCount")]
    total_token_count: Option<u32>,
    /// Prompt tokens Gemini spent on tool-use (built-in tool) results, billed
    /// as input and reported **beside** `promptTokenCount` (G-50-5a): folded
    /// into [`TokenUsage::prompt_tokens`]. Latent today, because
    /// `GeminiRequest` sends no tools, but correct before one lands. `None`
    /// when absent.
    #[serde(default, rename = "toolUsePromptTokenCount")]
    tool_use_prompt_token_count: Option<u32>,
    /// Cached-content tokens counted toward `prompt_token_count` (D-20 ->
    /// [`TokenUsage::cache_read_tokens`]). `Option<u32>`, not a bare `u32`
    /// like the three fields above, so an absent key maps to `None`
    /// ("not reported", D-03) rather than a fabricated `Some(0)` — Pitfall 4
    /// in RESEARCH.md. `#[derive(Default)]` on this struct is unaffected:
    /// `Option<u32>` is itself `Default`-derivable, so a response with no
    /// `usageMetadata` key at all still parses via `unwrap_or_default()`.
    #[serde(default, rename = "cachedContentTokenCount")]
    cached_content_token_count: Option<u32>,
    /// Thinking-token sub-count of `candidates_token_count` (D-20 ->
    /// [`TokenUsage::reasoning_tokens`]). Same `Option` reasoning as
    /// [`Self::cached_content_token_count`].
    #[serde(default, rename = "thoughtsTokenCount")]
    thoughts_token_count: Option<u32>,
}

/// Maps Gemini's wire-shape [`GeminiUsageMetadata`] onto [`TokenUsage`],
/// applying the D-02 inclusive-total contract: `prompt_tokens =
/// promptTokenCount + toolUsePromptTokenCount` and `completion_tokens =
/// candidatesTokenCount + thoughtsTokenCount` (both saturating), so
/// `total_tokens` stays `prompt + completion` and agrees with Gemini's own
/// documented `totalTokenCount = prompt + candidates + toolUsePrompt +
/// thoughts` arithmetic -- every category in that sum lands in a count (plan
/// 50-23 added the tool-use fold, G-50-5a). Gemini reports no cache-write
/// figure, so [`TokenUsage::cache_write_tokens`] always stays `None`.
///
/// The thoughts fold is live-measured 2026-10-18 (`50-USAGE-PROBE.md`, plan
/// 50-24): gemini-3.5-flash (23 + 3 + 192 = 218) and gemini-3.6-flash
/// (23 + 3 + 202 = 228) read `reasoning beside completion`, meaning
/// `thoughtsTokenCount` sits outside `candidatesTokenCount`. That is the reading
/// applied here. gemini-3.5-flash-lite reported no thoughts (`indeterminate`).
/// The tool-use fold is not live-measured, because this adapter sends no tools.
///
/// Returns the mapped usage together with Gemini's `totalTokenCount` (when
/// reported), which the caller reconciles rather than copies (D-02). Shared
/// by both the non-streaming path and the streaming path's terminal-frame
/// attachment (D-15) so the two paths cannot drift.
fn map_gemini_usage(usage: &GeminiUsageMetadata) -> (TokenUsage, Option<u32>) {
    let thoughts = usage.thoughts_token_count.unwrap_or(0);
    let completion_tokens = usage.candidates_token_count.saturating_add(thoughts);
    let prompt_tokens = usage
        .prompt_token_count
        .saturating_add(usage.tool_use_prompt_token_count.unwrap_or(0));

    let mut mapped = TokenUsage::new(prompt_tokens, completion_tokens);
    if let Some(cached) = usage.cached_content_token_count {
        mapped = mapped.with_cache_read(cached);
    }
    if let Some(thoughts_tokens) = usage.thoughts_token_count {
        mapped = mapped.with_reasoning(thoughts_tokens);
    }
    (mapped, usage.total_token_count)
}

#[derive(Debug, Deserialize)]
struct GeminiErrorEnvelope {
    error: GeminiErrorBody,
}

#[derive(Debug, Deserialize)]
struct GeminiErrorBody {
    /// Present on the wire but not read by this adapter — `status` (the
    /// RPC-style string) and `message` are what `map_error` consults.
    #[allow(dead_code)]
    #[serde(default)]
    code: Option<i64>,
    #[serde(default)]
    message: String,
    #[serde(default)]
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiModelsResponse {
    #[serde(default)]
    models: Vec<GeminiModelEntry>,
}

#[derive(Debug, Deserialize)]
struct GeminiModelEntry {
    /// Fully-qualified, e.g. `models/gemini-2.5-flash` — the leading
    /// `models/` segment is stripped in [`GeminiAdapter::fetch_live_models`].
    name: String,
}

// ── Free functions shared between the non-streaming and streaming paths ──

/// Reject a caller-supplied Gemini `model` identifier before it is
/// interpolated into a request URL (closes CR-01,
/// `17-VERIFICATION.md`).
///
/// `request.model` is spliced into the request **path** —
/// `{base_url}/models/{model}:generateContent` — unlike every
/// `CompatEngine`-based preset in this crate, which carries the model
/// inside the serde-encoded JSON request body instead. A path segment is
/// a fundamentally different trust boundary than a body field: a hostile
/// `model` value can displace an existing path segment (`/`), append an
/// operation suffix (`:`), or — on the streaming path — inject a query
/// parameter (`?`) that displaces the mandatory `alt=sse` framing
/// parameter, all on a request that carries the live `x-goog-api-key`
/// credential.
///
/// The permitted set is ASCII letters, digits, `.`, `_` and `-`
/// (`[A-Za-z0-9._-]`). Every character in that set is URL-unreserved, so
/// an already-valid identifier is unaffected by this guard and there is
/// nothing to percent-encode. Encoding an *invalid* value instead of
/// rejecting it would silently rewrite the caller's request into a
/// request for a different model than the one they named — an operator
/// must never receive a completion from a model they did not ask for.
/// Rejecting also adds no dependency: `percent-encoding` is not a
/// declared dependency of this crate, and a new dependency is itself a
/// cost PROV-01's own criteria weigh against `make deny` / `make audit`.
///
/// This is a *character* allow-list, not a membership check against
/// [`GeminiAdapter::available_models`]. Gating `generate()` on the
/// memoized model list would force a network fetch into the hot path of
/// every call and would reject any model the provider ships after this
/// release (D-13) — exactly the failure mode D-13 exists to avoid.
fn validate_model_identifier(model: &str) -> Result<(), LlmError> {
    if model.is_empty() {
        return Err(LlmError::InvalidPrompt(format!(
            "Gemini `model` must be a non-empty identifier made of ASCII letters, digits, \
             '.', '_' or '-' (set via the GEMINI_MODEL environment variable, or the request's \
             model field); got: \"{}\"",
            bounded_excerpt(model, RESPONSE_EXCERPT_CHAR_BUDGET)
        )));
    }

    if let Some(bad) = model
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')))
    {
        return Err(LlmError::InvalidPrompt(format!(
            "Gemini `model` contains a character outside the permitted set (ASCII letters, \
             digits, '.', '_' or '-'): {bad:?}. Correct the GEMINI_MODEL environment variable, \
             or the request's model field; got: \"{}\"",
            bounded_excerpt(model, RESPONSE_EXCERPT_CHAR_BUDGET)
        )));
    }

    if !model.chars().any(|c| c.is_ascii_alphanumeric()) {
        return Err(LlmError::InvalidPrompt(format!(
            "Gemini `model` must contain at least one ASCII letter or digit — a value built \
             only from '.', '_' or '-' is not a meaningful model name. Correct the \
             GEMINI_MODEL environment variable, or the request's model field; got: \"{}\"",
            bounded_excerpt(model, RESPONSE_EXCERPT_CHAR_BUDGET)
        )));
    }

    Ok(())
}

/// Whether a Gemini error envelope's raw message names a credential
/// failure, per [`GEMINI_CREDENTIAL_MESSAGE_SIGNATURES`] (WR-03).
///
/// Lowercases `message` once into a local `String` and checks whether any
/// signature is contained in it — no regex, no new dependency, no
/// `unwrap()`. Callers must pass the raw, unbounded provider message, not
/// the redacted/bounded excerpt: a signature appearing past the excerpt's
/// truncation boundary would otherwise be missed. Only the discriminator
/// itself reads the raw message — the caller must still only ever emit the
/// redacted, bounded excerpt in the resulting error.
fn is_credential_failure_message(message: &str) -> bool {
    let lowered = message.to_ascii_lowercase();
    GEMINI_CREDENTIAL_MESSAGE_SIGNATURES
        .iter()
        .any(|signature| lowered.contains(signature))
}

/// Concatenate the text of every part in a candidate's content, in array
/// order. A candidate with no `content` at all (fully safety-blocked, no
/// recoverable text) yields an empty string rather than erroring here —
/// callers decide whether an empty string plus the mapped `finish_reason`
/// constitutes an error.
fn candidate_text(candidate: &GeminiCandidate) -> String {
    candidate
        .content
        .as_ref()
        .map(|c| c.parts.iter().map(|p| p.text.as_str()).collect::<String>())
        .unwrap_or_default()
}

/// Map a Gemini `finishReason` string to [`FinishReason`], exhaustively.
///
/// `STOP` maps to [`FinishReason::Stop`], `MAX_TOKENS` to
/// [`FinishReason::Length`]. The blocked-content finishes — `SAFETY`,
/// `RECITATION`, `BLOCKLIST`, `PROHIBITED_CONTENT` and `SPII` — map to
/// [`FinishReason::ContentFilter`] (the closest existing variant — each is
/// a content-policy decision, matching `ContentFilter`'s semantics, and the
/// shared guard lets that finish through with no text, D-09). `OTHER` — and
/// any other value this adapter does not specifically recognise — maps to
/// [`FinishReason::Error`] carrying the raw reason string. This is
/// deliberate: coercing either to `Stop` would report a truncated or
/// unexplained generation as a normal completion. An absent `finishReason`
/// (`None`) maps to `Stop` — the non-terminal-streaming-frame case is
/// distinguished by callers never invoking this function with `None` for a
/// frame that has not yet finished.
fn map_finish_reason(reason: Option<&str>) -> FinishReason {
    match reason {
        None => FinishReason::Stop,
        Some("STOP") => FinishReason::Stop,
        Some("MAX_TOKENS") => FinishReason::Length,
        Some("SAFETY" | "RECITATION" | "BLOCKLIST" | "PROHIBITED_CONTENT" | "SPII") => {
            FinishReason::ContentFilter
        }
        Some(other) => FinishReason::Error(other.to_string()),
    }
}

/// Run the shared empty-output rule over the items of one parsed network chunk (D-11a).
///
/// Every `Ok` item's delta is accumulated in `completion`; the item that carries a finish reason
/// is the terminal chunk, and if [`StreamCompletion::finish`] rejects the accumulated text it is
/// replaced by that error. Earlier items are untouched, so text and frames already yielded stay
/// yielded. A blocked-content finish is [`FinishReason::ContentFilter`], which the guard lets
/// through, so it stays an `Ok` terminal chunk.
fn classify_stream_items(
    items: &mut [Result<StreamingResponse, LlmError>],
    completion: &mut StreamCompletion,
    model: &str,
    api_key: &str,
) {
    for item in items.iter_mut() {
        let verdict = match item {
            Ok(chunk) => {
                completion.observe_content(&chunk.delta);
                match &chunk.finish_reason {
                    // G-50-5a: a rejected terminal carries the finish frame's usage as its
                    // bill on the request model, `None` when that frame reported none.
                    Some(finish) => completion
                        .finish(GEMINI_PROVIDER, model, finish, api_key)
                        .map_err(|err| bill(err, model, chunk.usage.clone())),
                    None => Ok(()),
                }
            }
            Err(_) => Ok(()),
        };
        if let Err(err) = verdict {
            *item = Err(err);
        }
    }
}

/// Reconcile the terminal chunk's usage against the vendor total held from the same frame
/// (G-50-5a), recording any finding under [`USAGE_UNRECONCILED_METADATA_KEY`] on that chunk.
///
/// Runs after [`classify_stream_items`], so only an `Ok` terminal chunk is checked and carries a
/// finding; a terminal replaced with an `Err` carries no metadata (the 50-18 rule). The held
/// total is taken by the first terminal chunk seen, so it is reconciled once.
fn reconcile_terminal_usage(
    items: &mut [Result<StreamingResponse, LlmError>],
    usage_log: &UsageLog,
    model: &str,
    held_vendor_total: &mut Option<u32>,
) {
    for item in items.iter_mut() {
        let Ok(chunk) = item else {
            continue;
        };
        if chunk.finish_reason.is_none() {
            continue;
        }
        let vendor_total = held_vendor_total.take();
        let finding = chunk
            .usage
            .as_ref()
            .and_then(|usage| usage_log.record(GEMINI_PROVIDER, model, usage, vendor_total));
        if let Some(value) = finding {
            let taken = std::mem::replace(chunk, StreamingResponse::delta(String::new()));
            *chunk = taken.with_metadata(USAGE_UNRECONCILED_METADATA_KEY, value);
        }
    }
}

/// Stamp `metadata` (the request's `paladin.*` visibility keys, ADR-0061 (h)) on the terminal
/// chunk -- the `Ok` item that carries the finish reason -- and on no other item. A terminal
/// that [`classify_stream_items`] replaced with an `Err` (an empty answer) carries none.
fn stamp_terminal_metadata(
    items: &mut [Result<StreamingResponse, LlmError>],
    metadata: &HashMap<String, String>,
) {
    if metadata.is_empty() {
        return;
    }
    for item in items.iter_mut() {
        if let Ok(chunk) = item
            && chunk.finish_reason.is_some()
        {
            let taken = std::mem::replace(chunk, StreamingResponse::delta(String::new()));
            *chunk = metadata.iter().fold(taken, |stamped, (key, value)| {
                stamped.with_metadata(key.as_str(), value.as_str())
            });
        }
    }
}

/// Parse the complete SSE lines one network read produced into zero or more
/// [`StreamingResponse`] items.
///
/// The lines come from [`SseLines`], which reassembles a frame that straddles
/// two reads, so a frame is never parsed in halves (CR-02). One read can carry
/// more than one complete `data: {...}` event — common when a mock transport
/// (or a provider whose TCP framing does not align to event boundaries)
/// writes the whole body at once — so every
/// `data:`-prefixed line found is emitted as its own item, mirroring
/// [`crate::compat::engine::CompatEngine::generate_stream`]'s `flat_map`
/// discipline. There is no `[DONE]` sentinel in Gemini's SSE framing; the
/// stream simply ends when the body ends. A frame with no `candidates` at
/// all (e.g. a metadata-only frame) yields no item, not an error.
///
/// The finish-reason frame's `totalTokenCount` is written to `held_vendor_total`
/// for [`reconcile_terminal_usage`] (G-50-5a); a frame with no usage leaves it
/// `None`.
fn parse_sse_lines(
    lines: &[String],
    held_vendor_total: &mut Option<u32>,
) -> Vec<Result<StreamingResponse, LlmError>> {
    let mut items = Vec::new();

    for line in lines {
        let Some(json_str) = line.strip_prefix("data: ") else {
            continue;
        };

        // Gemini streams partial `GenerateContentResponse` objects — the
        // same shape `GeminiResponse` already parses for the non-streaming
        // path — rather than a distinct delta type.
        match serde_json::from_str::<GeminiResponse>(json_str) {
            Ok(parsed) => {
                if let Some(candidate) = parsed.candidates.first() {
                    let delta = candidate_text(candidate);
                    let finish_reason = candidate
                        .finish_reason
                        .as_deref()
                        .map(|r| map_finish_reason(Some(r)));

                    // `delta` and `finish_reason` can arrive on the SAME
                    // frame here, which the fixed three constructors don't
                    // express directly; the `finish_reason` field is `pub`,
                    // so it is set after construction rather than losing the
                    // delta text a `terminal()`-only chunk would force to
                    // empty.
                    let mut item = StreamingResponse::delta(delta);
                    item.finish_reason = finish_reason;

                    // D-14/D-15: every SSE frame's `usageMetadata` is
                    // CUMULATIVE, not a per-frame delta, and it arrives
                    // together with the terminal `finishReason` on the SAME
                    // frame -- so attaching usage only on the
                    // finish-reason-bearing frame IS "take the last frame's
                    // value" without needing to hold state across chunks.
                    // Every earlier frame keeps `usage: None`.
                    if item.finish_reason.is_some()
                        && let Some(usage) = parsed.usage_metadata.as_ref()
                    {
                        let (mapped, vendor_total) = map_gemini_usage(usage);
                        *held_vendor_total = vendor_total;
                        item = item.with_usage(mapped);
                    }

                    items.push(Ok(item));
                }
            }
            Err(e) => {
                items.push(Err(LlmError::ProcessingError(format!(
                    "Failed to parse Gemini stream frame: {e}"
                ))));
            }
        }
    }

    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Matcher, Server};
    use paladin_core::platform::container::prompt::{
        FunctionPrompt, PromptItem, SystemPrompt, TextPrompt, UserPrompt,
    };
    use serde_json::json;
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// WR-03: the documented `with_base_url` guarantee is an `http://` or `https://` URL. A value
    /// that merely starts with `http` (`httpx://x`, a scheme-less `http-gateway...`) must fail
    /// construction like any other non-HTTP value.
    #[test]
    fn a_builder_set_non_http_base_url_fails_gemini_construction() {
        for bad in [
            "ftp://x",
            "",
            "api.example.com/v1",
            "httpx://x",
            "http-gateway.internal/v1",
            "httpfoo:",
        ] {
            let config = GeminiConfig::new("k".to_string(), "gemini-3.5-flash-lite".to_string())
                .with_base_url(bad);
            assert!(
                config.validate().is_err(),
                "base URL {bad:?} must fail validation"
            );
            assert!(
                GeminiAdapter::new(config).is_err(),
                "base URL {bad:?} must fail adapter construction"
            );
        }
        for good in ["http://localhost:1", "https://gateway.example/v1"] {
            let config = GeminiConfig::new("k".to_string(), "gemini-3.5-flash-lite".to_string())
                .with_base_url(good);
            assert!(
                config.validate().is_ok(),
                "base URL {good:?} must pass validation"
            );
        }
    }

    fn test_config(base_url: &str) -> GeminiConfig {
        GeminiConfig::new(
            "test-key-abc123".to_string(),
            "gemini-2.5-flash".to_string(),
        )
        .with_base_url(base_url)
    }

    fn test_adapter(base_url: &str) -> GeminiAdapter {
        GeminiAdapter::new(test_config(base_url)).expect("test config must build a valid adapter")
    }

    fn build_request(model: &str, prompt_type: PromptType) -> LlmRequest {
        LlmRequest::new(model, PromptItem::new(prompt_type).unwrap())
    }

    // ── Dated registry wiring (plan 50-06, D-05/D-08) ──

    #[test]
    fn gemini_default_model_is_seeded() {
        let adapter = test_adapter("http://localhost");
        let (profile, origin) = adapter.model_registry.resolve(GEMINI_DEFAULT_MODEL);
        assert_eq!(
            origin,
            crate::registry::ProfileOrigin::Seeded {
                verified_on: "2026-10-15"
            }
        );
        assert_eq!(profile.context_window, Some(1_048_576));
        assert_eq!(
            adapter.get_capabilities().max_context_tokens,
            profile.context_window,
            "the declared window is the default model's seeded window"
        );
    }

    #[test]
    fn gemini_default_model_is_the_operator_identified_current_model() {
        assert_eq!(GEMINI_DEFAULT_MODEL, "gemini-3.5-flash-lite");
        assert_eq!(GEMINI_FALLBACK_MODELS[0], GEMINI_DEFAULT_MODEL);
    }

    #[test]
    fn gemini_unseeded_model_still_resolves_the_adapter_default() {
        // `gemini-2.5-flash` answered 404 "no longer available to new users" in the 2026-10-20
        // probe, so nothing was measured and it stays unseeded (`gemini-3.1-flash-lite`, which
        // this test used before, was seeded by Phase 50.1).
        let (_, origin) = test_adapter("http://localhost")
            .model_registry
            .resolve("gemini-2.5-flash");
        assert_eq!(origin, crate::registry::ProfileOrigin::AdapterDefault);
    }

    // ── GeminiConfig::new (SURFACE-02, D-05) ──

    #[test]
    fn gemini_config_new_takes_key_and_model_and_defaults_the_endpoint() {
        let config = GeminiConfig::new("k".to_string(), "gemini-2.5-flash".to_string());
        assert_eq!(config.base_url, GEMINI_DEFAULT_BASE_URL);
        assert_eq!(config.model, "gemini-2.5-flash");
        assert_eq!(config.api_key, "k");
        // The timeout default is unchanged: 60 s for an unseeded model, 300 s for a seeded
        // reasoning one, exactly what `default_timeout_seconds` answered before.
        assert_eq!(
            config.timeout_seconds,
            default_timeout_seconds(GEMINI_PROVIDER, "gemini-2.5-flash", 60)
        );
        let reasoning = GeminiConfig::new("k".to_string(), GEMINI_DEFAULT_MODEL.to_string());
        assert_eq!(
            reasoning.timeout_seconds,
            default_timeout_seconds(GEMINI_PROVIDER, GEMINI_DEFAULT_MODEL, 60)
        );
    }

    #[test]
    fn gemini_config_with_base_url_replaces_the_default_endpoint() {
        let config = GeminiConfig::new("k".to_string(), "m".to_string())
            .with_base_url("http://localhost:1234");
        assert_eq!(config.base_url, "http://localhost:1234");
    }

    // ── GeminiConfig::from_parts / from_env defaulting ──

    #[test]
    fn gemini_config_from_parts_errors_when_api_key_is_absent() {
        let result = GeminiConfig::from_parts(None, None, None, None);
        assert!(result.is_err(), "GEMINI_API_KEY must be required");
    }

    #[test]
    fn gemini_config_from_parts_defaults_base_url_model_and_timeout_when_only_key_is_set() {
        let config = GeminiConfig::from_parts(Some("live-key".to_string()), None, None, None)
            .expect("must succeed with only the API key set");
        assert_eq!(config.api_key, "live-key");
        assert_eq!(config.base_url, GEMINI_DEFAULT_BASE_URL);
        assert_eq!(config.model, GEMINI_DEFAULT_MODEL);
        // The default model is a seeded reasoning model, so its default timeout is the
        // reasoning-model 300 s (D-13), not the hosted-preset 60 s.
        assert_eq!(config.timeout_seconds, 300);
    }

    /// Every Gemini row seeded by Phase 50.1 has `reasoning: true` (listing `thinking=true`), so
    /// the direct constructor defaults its timeout to the reasoning-model 300 s (D-13); a
    /// 2.5-generation id the probe could not measure stays unseeded and keeps 60 s.
    #[test]
    fn gemini_config_new_defaults_the_phase_50_1_reasoning_rows_to_300_seconds() {
        for model in [
            "gemini-4-argon",
            "gemini-3.8-flash",
            "gemini-3.7-flash",
            "gemini-3.1-pro-preview",
            "gemini-pro-latest",
            "gemini-flash-latest",
            "gemini-flash-lite-latest",
            "gemini-3.1-flash-lite",
            "gemini-3-flash-preview",
        ] {
            let config = GeminiConfig::new("k".to_string(), model.to_string());
            assert_eq!(config.timeout_seconds, 300, "{model}");
        }
        let unseeded = GeminiConfig::new("k".to_string(), "gemini-2.5-flash".to_string());
        assert_eq!(unseeded.timeout_seconds, 60);
    }

    #[test]
    fn gemini_config_from_parts_honors_overrides() {
        let config = GeminiConfig::from_parts(
            Some("live-key".to_string()),
            Some("https://override.example/v1beta".to_string()),
            Some("gemini-3.1-flash-lite".to_string()),
            Some("30".to_string()),
        )
        .unwrap();
        assert_eq!(config.base_url, "https://override.example/v1beta");
        assert_eq!(config.model, "gemini-3.1-flash-lite");
        assert_eq!(config.timeout_seconds, 30);
    }

    // ── Request shaping: systemInstruction, no tools, function rejection ──

    #[test]
    fn build_request_places_system_prompt_in_system_instruction_never_in_contents() {
        let adapter = test_adapter("https://example.invalid");
        let request = build_request(
            "gemini-2.5-flash",
            PromptType::System(SystemPrompt {
                instructions: "Be terse.".to_string(),
                constraints: None,
            }),
        );

        let gemini_request = adapter.build_request(&request).unwrap().0;
        let json = serde_json::to_value(&gemini_request).unwrap();

        assert_eq!(json["systemInstruction"]["parts"][0]["text"], "Be terse.");
        assert!(
            json.get("contents").unwrap().as_array().unwrap().is_empty(),
            "a system-only prompt must not produce a contents[] entry"
        );

        let serialized = serde_json::to_string(&gemini_request).unwrap();
        assert!(
            !serialized.contains(r#""role":"system""#),
            "no contents[] entry may carry a system role: {serialized}"
        );
    }

    #[test]
    fn build_request_maps_user_and_assistant_roles_correctly() {
        let adapter = test_adapter("https://example.invalid");

        let user_request = build_request(
            "gemini-2.5-flash",
            PromptType::User(UserPrompt {
                query: "Hello".to_string(),
                context: None,
            }),
        );
        let user_gemini = adapter.build_request(&user_request).unwrap().0;
        assert_eq!(user_gemini.contents[0].role, "user");

        let assistant_request = build_request(
            "gemini-2.5-flash",
            PromptType::Assistant(paladin_core::platform::container::prompt::AssistantPrompt {
                response: "Hi".to_string(),
                reasoning: None,
            }),
        );
        let assistant_gemini = adapter.build_request(&assistant_request).unwrap().0;
        // Gemini's assistant-equivalent role is "model", never "assistant".
        assert_eq!(assistant_gemini.contents[0].role, "model");
    }

    /// Phase 50 (PARITY-08): a present context is appended to the query by the shared
    /// `user_turn_text` helper; an absent one leaves the query unchanged.
    #[test]
    fn gemini_user_turn_appends_context() {
        let adapter = test_adapter("https://example.invalid");
        let user_turn = |context: Option<&str>| -> serde_json::Value {
            let request = build_request(
                "gemini-2.5-flash",
                PromptType::User(UserPrompt {
                    query: "What is 2+2?".to_string(),
                    context: context.map(str::to_string),
                }),
            );
            let body = serde_json::to_value(adapter.build_request(&request).unwrap().0).unwrap();
            body["contents"][0]["parts"][0]["text"].clone()
        };

        assert_eq!(user_turn(None), "What is 2+2?");
        assert_eq!(
            user_turn(Some("Extra context.")),
            "What is 2+2?\n\nExtra context."
        );
    }

    #[test]
    fn build_request_serializes_only_the_known_gemini_request_fields() {
        // `GeminiRequest` has no field for a tool/function-calling surface
        // at all (D-08) — asserted here as a closed key set rather than a
        // substring search, so this test does not itself have to spell out
        // the very field names this crate's source-wide acceptance-criteria
        // grep forbids appearing anywhere (comments excepted) in this file.
        let adapter = test_adapter("https://example.invalid");
        let request = build_request(
            "gemini-2.5-flash",
            PromptType::User(UserPrompt {
                query: "Hello".to_string(),
                context: None,
            }),
        );

        let gemini_request = adapter.build_request(&request).unwrap().0;
        let value = serde_json::to_value(&gemini_request).unwrap();
        let object = value
            .as_object()
            .expect("GeminiRequest must serialize to a JSON object");

        let known_keys: std::collections::HashSet<&str> =
            ["contents", "systemInstruction", "generationConfig"]
                .into_iter()
                .collect();
        for key in object.keys() {
            assert!(
                known_keys.contains(key.as_str()),
                "unexpected key in serialized Gemini request: {key}"
            );
        }
    }

    // ── Phase 26 (RT-05, D-28): response_format reaches the wire ──────────

    #[test]
    fn gemini_request_sets_response_mime_type_for_json_object() {
        let adapter = test_adapter("https://example.invalid");
        let request = build_request(
            "gemini-3.6-flash",
            PromptType::User(UserPrompt {
                query: "Hello".to_string(),
                context: None,
            }),
        )
        .with_response_format(ResponseFormat::JsonObject);

        let gemini_request = adapter.build_request(&request).unwrap().0;
        let json = serde_json::to_value(&gemini_request).unwrap();

        assert_eq!(
            json["generationConfig"]["responseMimeType"],
            "application/json"
        );
        assert!(
            json["generationConfig"]
                .as_object()
                .unwrap()
                .get("responseSchema")
                .is_none(),
            "JsonObject carries no schema: {json}"
        );
    }

    #[test]
    fn gemini_request_sets_response_schema_for_json_schema() {
        let adapter = test_adapter("https://example.invalid");
        let schema = serde_json::json!({
            "type": "object",
            "properties": {"answer": {"type": "string"}}
        });
        let request = build_request(
            "gemini-3.6-flash",
            PromptType::User(UserPrompt {
                query: "Hello".to_string(),
                context: None,
            }),
        )
        .with_response_format(ResponseFormat::JsonSchema {
            name: "answer_schema".to_string(),
            schema: schema.clone(),
            strict: true,
        });

        let gemini_request = adapter.build_request(&request).unwrap().0;
        let json = serde_json::to_value(&gemini_request).unwrap();

        assert_eq!(
            json["generationConfig"]["responseMimeType"],
            "application/json"
        );
        assert_eq!(json["generationConfig"]["responseSchema"], schema);
    }

    #[test]
    fn gemini_request_without_response_format_is_unchanged() {
        let adapter = test_adapter("https://example.invalid");
        let request = build_request(
            "gemini-3.6-flash",
            PromptType::User(UserPrompt {
                query: "Hello".to_string(),
                context: None,
            }),
        );

        let gemini_request = adapter.build_request(&request).unwrap().0;
        let json = serde_json::to_value(&gemini_request).unwrap();
        let generation_config = json["generationConfig"]
            .as_object()
            .expect("the default prompt parameters already populate generationConfig");

        assert!(
            !generation_config.contains_key("responseMimeType"),
            "absent response_format must not add responseMimeType, got: {json}"
        );
        assert!(
            !generation_config.contains_key("responseSchema"),
            "absent response_format must not add responseSchema, got: {json}"
        );
    }

    #[test]
    fn build_request_rejects_function_prompt_type_with_invalid_prompt() {
        let adapter = test_adapter("https://example.invalid");
        let request = build_request(
            "gemini-2.5-flash",
            PromptType::Function(FunctionPrompt {
                function_name: "lookup".to_string(),
                arguments: BTreeMap::new(),
                description: None,
            }),
        );

        let result = adapter.build_request(&request);
        assert!(matches!(result, Err(LlmError::InvalidPrompt(_))));
    }

    #[test]
    fn build_request_rejects_function_role_text_prompt_with_invalid_prompt() {
        let adapter = test_adapter("https://example.invalid");
        let request = build_request(
            "gemini-2.5-flash",
            PromptType::Text(TextPrompt {
                content: "irrelevant".to_string(),
                role: PromptRole::Function,
            }),
        );

        let result = adapter.build_request(&request);
        assert!(matches!(result, Err(LlmError::InvalidPrompt(_))));
    }

    // ── finishReason mapping (exhaustive, never coerced to Stop) ──

    #[test]
    fn map_finish_reason_covers_every_documented_value() {
        assert!(matches!(map_finish_reason(None), FinishReason::Stop));
        assert!(matches!(
            map_finish_reason(Some("STOP")),
            FinishReason::Stop
        ));
        assert!(matches!(
            map_finish_reason(Some("MAX_TOKENS")),
            FinishReason::Length
        ));
        assert!(matches!(
            map_finish_reason(Some("SAFETY")),
            FinishReason::ContentFilter
        ));

        match map_finish_reason(Some("OTHER")) {
            FinishReason::Error(reason) => assert_eq!(reason, "OTHER"),
            other => panic!("expected FinishReason::Error(\"OTHER\"), got {other:?}"),
        }
        // Plan 50-13 (D-09/D-11a) flipped RECITATION: it used to be pinned as
        // `Error("RECITATION")`. It is a blocked-content finish, so it maps to
        // `ContentFilter` with its siblings -- otherwise the shared guard would
        // report a recitation block with no text as an empty completion.
        for blocked in [
            "SAFETY",
            "RECITATION",
            "BLOCKLIST",
            "PROHIBITED_CONTENT",
            "SPII",
        ] {
            assert!(
                matches!(
                    map_finish_reason(Some(blocked)),
                    FinishReason::ContentFilter
                ),
                "{blocked} must map to ContentFilter"
            );
        }
    }

    // ── Response parsing ──

    #[test]
    fn parse_response_empty_candidates_yields_empty_completion() {
        let adapter = test_adapter("https://example.invalid");
        let response = GeminiResponse {
            candidates: vec![],
            usage_metadata: None,
        };

        let result = adapter.parse_response(Uuid::new_v4(), "gemini-2.5-flash", response);
        assert!(matches!(result, Err(LlmError::EmptyCompletion { .. })));
    }

    #[test]
    fn parse_response_well_formed_candidate_parses_content_usage_and_finish_reason() {
        let adapter = test_adapter("https://example.invalid");
        let body = json!({
            "candidates": [{
                "content": {"role": "model", "parts": [{"text": "Hello there"}]},
                "finishReason": "STOP"
            }],
            "usageMetadata": {
                "promptTokenCount": 5,
                "candidatesTokenCount": 3,
                "totalTokenCount": 8
            }
        });
        let response: GeminiResponse = serde_json::from_value(body).unwrap();

        let llm_response = adapter
            .parse_response(Uuid::new_v4(), "gemini-2.5-flash", response)
            .unwrap();

        assert_eq!(llm_response.content, "Hello there");
        assert!(matches!(llm_response.finish_reason, FinishReason::Stop));
        assert_eq!(llm_response.usage.prompt_tokens, 5);
        assert_eq!(llm_response.usage.completion_tokens, 3);
        assert_eq!(llm_response.usage.total_tokens, 8);
    }

    // ── Plan 31-04 (D-20): cached-content + thoughts sub-count mapping ────

    #[test]
    fn parse_response_maps_cached_content_and_thoughts_into_completion_and_optionals() {
        let adapter = test_adapter("https://example.invalid");
        let body = json!({
            "candidates": [{
                "content": {"role": "model", "parts": [{"text": "Hello there"}]},
                "finishReason": "STOP"
            }],
            "usageMetadata": {
                "promptTokenCount": 310,
                "candidatesTokenCount": 88,
                "cachedContentTokenCount": 96,
                "thoughtsTokenCount": 41,
                "totalTokenCount": 439
            }
        });
        let response: GeminiResponse = serde_json::from_value(body).unwrap();

        let llm_response = adapter
            .parse_response(Uuid::new_v4(), "gemini-2.5-flash", response)
            .unwrap();

        assert_eq!(llm_response.usage.prompt_tokens, 310);
        // completion_tokens = candidates(88) + thoughts(41) (D-02 inclusive total).
        assert_eq!(llm_response.usage.completion_tokens, 129);
        assert_eq!(llm_response.usage.total_tokens, 439);
        assert_eq!(llm_response.usage.cache_read_tokens, Some(96));
        assert_eq!(llm_response.usage.reasoning_tokens, Some(41));
        assert_eq!(llm_response.usage.cache_write_tokens, None);
    }

    #[test]
    fn parse_response_leaves_cache_and_reasoning_none_when_the_payload_omits_them() {
        let adapter = test_adapter("https://example.invalid");
        let body = json!({
            "candidates": [{
                "content": {"role": "model", "parts": [{"text": "Hello there"}]},
                "finishReason": "STOP"
            }],
            "usageMetadata": {
                "promptTokenCount": 5,
                "candidatesTokenCount": 3,
                "totalTokenCount": 8
            }
        });
        let response: GeminiResponse = serde_json::from_value(body).unwrap();

        let llm_response = adapter
            .parse_response(Uuid::new_v4(), "gemini-2.5-flash", response)
            .unwrap();

        // completion_tokens == candidates alone when thoughts is absent.
        assert_eq!(llm_response.usage.completion_tokens, 3);
        assert_eq!(llm_response.usage.cache_read_tokens, None);
        assert_eq!(llm_response.usage.reasoning_tokens, None);
        assert_eq!(llm_response.usage.cache_write_tokens, None);
    }

    #[test]
    fn parse_response_with_no_usage_metadata_key_still_parses_via_default() {
        // GeminiUsageMetadata keeps its `Default` derive (RESEARCH.md
        // Pitfall 4) even with the two new `Option<u32>` fields, so a
        // response with no `usageMetadata` key at all still parses.
        let adapter = test_adapter("https://example.invalid");
        let body = json!({
            "candidates": [{
                "content": {"role": "model", "parts": [{"text": "Hello there"}]},
                "finishReason": "STOP"
            }]
        });
        let response: GeminiResponse = serde_json::from_value(body).unwrap();

        let llm_response = adapter
            .parse_response(Uuid::new_v4(), "gemini-2.5-flash", response)
            .unwrap();

        assert_eq!(llm_response.usage, TokenUsage::new(0, 0));
    }

    // ── Plan 50-23 (G-50-5a): tool-use prompt tokens folded, Gemini's own total reconciled ──

    /// A buffered Gemini success body carrying `usage_metadata` as its `usageMetadata`.
    fn gemini_usage_response_body(usage_metadata: serde_json::Value) -> String {
        json!({
            "candidates": [{
                "content": {"role": "model", "parts": [{"text": "Hi there"}]},
                "finishReason": "STOP"
            }],
            "usageMetadata": usage_metadata
        })
        .to_string()
    }

    /// Serve `body` on the buffered endpoint and return the adapter's response.
    async fn generate_against(body: String) -> LlmResponse {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/models/gemini-2.5-flash:generateContent")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(body)
            .create_async()
            .await;
        let adapter = test_adapter(&server.url());
        let request = build_request(
            "gemini-2.5-flash",
            PromptType::User(UserPrompt {
                query: "Hello".to_string(),
                context: None,
            }),
        );
        adapter.generate(request).await.unwrap()
    }

    /// Tracer (plan 50-23 Task 1): `toolUsePromptTokenCount` is billed input, so it is folded into
    /// `prompt_tokens` and the mapped total agrees with Gemini's `totalTokenCount` (debug repro
    /// 430: prompt 100 + toolUse 300 = 400, candidates 20 + thoughts 10 = 30). A total that still
    /// disagrees is recorded, never copied (D-02).
    #[tokio::test]
    async fn gemini_tool_use_prompt_tokens_reach_prompt_tokens() {
        use crate::usage_reconcile::USAGE_UNRECONCILED_METADATA_KEY;

        let agreeing = generate_against(gemini_usage_response_body(json!({
            "promptTokenCount": 100,
            "candidatesTokenCount": 20,
            "toolUsePromptTokenCount": 300,
            "thoughtsTokenCount": 10,
            "totalTokenCount": 430
        })))
        .await;
        assert_eq!(agreeing.usage.prompt_tokens, 400, "prompt + toolUse");
        assert_eq!(
            agreeing.usage.completion_tokens, 30,
            "candidates + thoughts"
        );
        assert_eq!(agreeing.usage.reasoning_tokens, Some(10));
        assert_eq!(agreeing.usage.total_tokens, 430);
        assert!(
            !agreeing
                .metadata
                .contains_key(USAGE_UNRECONCILED_METADATA_KEY),
            "an agreeing total records no finding: {:?}",
            agreeing.metadata
        );

        let disagreeing = generate_against(gemini_usage_response_body(json!({
            "promptTokenCount": 100,
            "candidatesTokenCount": 20,
            "toolUsePromptTokenCount": 300,
            "thoughtsTokenCount": 10,
            "totalTokenCount": 999
        })))
        .await;
        assert_eq!(
            disagreeing.usage, agreeing.usage,
            "the mapped usage is kept"
        );
        assert_eq!(
            disagreeing
                .metadata
                .get(USAGE_UNRECONCILED_METADATA_KEY)
                .map(String::as_str),
            Some("total:vendor=999,mapped=430")
        );
    }

    /// G-50-5a (plan 50-24): the live-measured gemini-3.5-flash usage metadata
    /// (`50-USAGE-PROBE.md`, 2026-10-18: prompt 23, candidates 3, thoughts 192, total 218) maps
    /// with thoughts folded into completion (3 + 192 = 195) and reconciles with Gemini's own
    /// `totalTokenCount`. Without the fold it would map to 26 and disagree.
    #[tokio::test]
    async fn gemini_measured_thoughts_are_beside_candidates() {
        use crate::usage_reconcile::USAGE_UNRECONCILED_METADATA_KEY;

        let response = generate_against(gemini_usage_response_body(json!({
            "promptTokenCount": 23,
            "candidatesTokenCount": 3,
            "thoughtsTokenCount": 192,
            "totalTokenCount": 218
        })))
        .await;
        assert_eq!(response.usage.prompt_tokens, 23);
        assert_eq!(
            response.usage.completion_tokens, 195,
            "candidates + thoughts"
        );
        assert_eq!(response.usage.reasoning_tokens, Some(192));
        assert_eq!(response.usage.total_tokens, 218);
        assert!(
            !response
                .metadata
                .contains_key(USAGE_UNRECONCILED_METADATA_KEY),
            "{:?}",
            response.metadata
        );
    }

    #[tokio::test]
    async fn gemini_absent_total_and_tool_use_map_as_before_with_no_finding() {
        use crate::usage_reconcile::USAGE_UNRECONCILED_METADATA_KEY;

        let response = generate_against(gemini_usage_response_body(json!({
            "promptTokenCount": 310,
            "candidatesTokenCount": 88,
            "cachedContentTokenCount": 96,
            "thoughtsTokenCount": 41
        })))
        .await;
        assert_eq!(response.usage.prompt_tokens, 310);
        assert_eq!(response.usage.completion_tokens, 129);
        assert_eq!(response.usage.total_tokens, 439);
        assert_eq!(response.usage.cache_read_tokens, Some(96));
        assert_eq!(response.usage.reasoning_tokens, Some(41));
        assert!(
            !response
                .metadata
                .contains_key(USAGE_UNRECONCILED_METADATA_KEY),
            "an absent vendor total is not a finding: {:?}",
            response.metadata
        );
    }

    #[tokio::test]
    async fn gemini_stream_terminal_usage_is_folded_and_reconciled() {
        use crate::usage_reconcile::USAGE_UNRECONCILED_METADATA_KEY;

        let sse_body = concat!(
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"Hel\"}]}}]}\n\n",
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"lo\"}],\"role\":\"model\"},",
            "\"finishReason\":\"STOP\"}],\"usageMetadata\":{\"promptTokenCount\":100,",
            "\"candidatesTokenCount\":20,\"toolUsePromptTokenCount\":300,",
            "\"thoughtsTokenCount\":10,\"totalTokenCount\":999}}\n\n",
        );
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/models/gemini-2.5-flash:streamGenerateContent")
            .match_query(Matcher::Any)
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(sse_body)
            .create_async()
            .await;
        let adapter = test_adapter(&server.url());
        let request = build_request(
            "gemini-2.5-flash",
            PromptType::User(UserPrompt {
                query: "Hello".to_string(),
                context: None,
            }),
        );

        let mut stream = Box::into_pin(adapter.generate_stream(request).await.unwrap());
        let mut chunks = Vec::new();
        while let Some(item) = stream.next().await {
            chunks.push(item.unwrap());
        }

        let (terminal, earlier) = chunks.split_last().expect("at least one chunk");
        assert!(terminal.finish_reason.is_some());
        let usage = terminal
            .usage
            .as_ref()
            .expect("the terminal chunk carries usage");
        assert_eq!(usage.prompt_tokens, 400);
        assert_eq!(usage.completion_tokens, 30);
        assert_eq!(usage.total_tokens, 430);
        assert_eq!(
            terminal
                .metadata
                .get(USAGE_UNRECONCILED_METADATA_KEY)
                .map(String::as_str),
            Some("total:vendor=999,mapped=430")
        );
        for chunk in earlier {
            assert!(
                !chunk.metadata.contains_key(USAGE_UNRECONCILED_METADATA_KEY),
                "only the terminal chunk carries the finding"
            );
        }
    }

    // ── WR-03 (new): a truncated-to-empty completion is an error, not a success ──
    //
    // Wire-level facts resolved by reading (not assuming) the code, both recorded
    // in 17-15-SUMMARY.md per D-00e:
    //   1. `MAX_TOKENS` maps to `FinishReason::Length` — `map_finish_reason`,
    //      crates/paladin-llm/src/gemini/adapter.rs:1074.
    //   2. `SAFETY` maps to `FinishReason::ContentFilter`, not `Length` —
    //      `map_finish_reason`, crates/paladin-llm/src/gemini/adapter.rs:1075 —
    //      so it is the non-truncation reason used for the refusal control below.

    #[test]
    fn parse_response_maps_max_tokens_with_no_parts_to_empty_completion() {
        let adapter = test_adapter("https://example.invalid");
        let body = json!({
            "candidates": [{
                "content": {"role": "model", "parts": []},
                "finishReason": "MAX_TOKENS"
            }]
        });
        let response: GeminiResponse = serde_json::from_value(body).unwrap();

        let result = adapter.parse_response(Uuid::new_v4(), "gemini-2.5-flash", response);

        assert!(
            matches!(result, Err(LlmError::EmptyCompletion { .. })),
            "expected Err(EmptyCompletion {{ .. }}) for a MAX_TOKENS finish with no parts, got {result:?}"
        );
        if let Err(LlmError::EmptyCompletion {
            detail: message, ..
        }) = result
        {
            assert!(
                message.contains("max_tokens"),
                "EmptyCompletion message should name the remedy (a larger max_tokens), got: {message}"
            );
        }
    }

    #[test]
    fn parse_response_maps_max_tokens_with_whitespace_only_text_to_empty_completion() {
        let adapter = test_adapter("https://example.invalid");
        let body = json!({
            "candidates": [{
                "content": {"role": "model", "parts": [{"text": "   \n  "}]},
                "finishReason": "MAX_TOKENS"
            }]
        });
        let response: GeminiResponse = serde_json::from_value(body).unwrap();

        let result = adapter.parse_response(Uuid::new_v4(), "gemini-2.5-flash", response);

        assert!(
            matches!(result, Err(LlmError::EmptyCompletion { .. })),
            "expected Err(EmptyCompletion {{ .. }}) for a MAX_TOKENS finish with whitespace-only text, got {result:?}"
        );
    }

    /// Adjacency control: the guard is two conditions, not one. A truncated
    /// response that still produced real text is a success. **Passes today** —
    /// this pins the boundary so the fix in Task 2 cannot widen into a
    /// catch-all on `finish_reason` alone.
    #[test]
    fn parse_response_keeps_a_truncated_response_that_produced_text() {
        let adapter = test_adapter("https://example.invalid");
        let body = json!({
            "candidates": [{
                "content": {"role": "model", "parts": [{"text": "partial answer"}]},
                "finishReason": "MAX_TOKENS"
            }]
        });
        let response: GeminiResponse = serde_json::from_value(body).unwrap();

        let llm_response = adapter
            .parse_response(Uuid::new_v4(), "gemini-2.5-flash", response)
            .expect("a truncated response with real text must still be Ok");

        assert_eq!(llm_response.content, "partial answer");
        assert!(matches!(llm_response.finish_reason, FinishReason::Length));
    }

    /// D-11a: an empty response under a normal `STOP` is an empty completion, the same as on
    /// every other adapter (it used to be pinned as `Ok("")`, parity with the compat engine's
    /// then-`length`-only check; plan 50-13 flipped it onto the shared guard).
    #[test]
    fn parse_response_maps_an_empty_stop_to_empty_completion() {
        let adapter = test_adapter("https://example.invalid");
        for parts in [json!([]), json!([{"text": ""}]), json!([{"text": "  \n"}])] {
            let body = json!({
                "candidates": [{
                    "content": {"role": "model", "parts": parts},
                    "finishReason": "STOP"
                }]
            });
            let response: GeminiResponse = serde_json::from_value(body).unwrap();

            match adapter.parse_response(Uuid::new_v4(), "gemini-2.5-flash", response) {
                Err(LlmError::EmptyCompletion { detail: msg, .. }) => {
                    assert!(msg.contains("finish_reason=stop"), "{msg}");
                    assert!(!msg.contains("max_tokens"), "{msg}");
                }
                other => panic!("expected Err(EmptyCompletion), got {other:?}"),
            }
        }
    }

    /// Refusal control: a `SAFETY` finish (content-policy refusal, mapped to
    /// `FinishReason::ContentFilter`, never `Length`) with no text must not be
    /// reported as a token-budget problem. **Passes today**, and is the test
    /// that keeps the truncation-only prohibition honest: a refusal reported
    /// as a budget problem sends the operator to buy a larger budget that
    /// will produce the same refusal.
    #[test]
    fn parse_response_does_not_blame_the_token_budget_for_a_refusal() {
        let adapter = test_adapter("https://example.invalid");
        let body = json!({
            "candidates": [{
                "content": {"role": "model", "parts": []},
                "finishReason": "SAFETY"
            }]
        });
        let response: GeminiResponse = serde_json::from_value(body).unwrap();

        let result = adapter.parse_response(Uuid::new_v4(), "gemini-2.5-flash", response);

        assert!(
            !matches!(result, Err(LlmError::EmptyCompletion { .. })),
            "a SAFETY refusal must not be reported as EmptyCompletion (token-budget problem), got {result:?}"
        );
    }

    /// D-09: every blocked-content finish with no text stays an `Ok` `ContentFilter` result,
    /// not an empty completion.
    #[test]
    fn parse_response_keeps_every_blocked_content_finish_as_content_filter() {
        let adapter = test_adapter("https://example.invalid");
        for finish in [
            "SAFETY",
            "RECITATION",
            "BLOCKLIST",
            "PROHIBITED_CONTENT",
            "SPII",
        ] {
            let body = json!({
                "candidates": [{
                    "content": {"role": "model", "parts": []},
                    "finishReason": finish
                }]
            });
            let response: GeminiResponse = serde_json::from_value(body).unwrap();

            let parsed = adapter
                .parse_response(Uuid::new_v4(), "gemini-2.5-flash", response)
                .unwrap_or_else(|e| panic!("{finish} with no text must stay Ok, got {e:?}"));
            assert!(parsed.content.is_empty(), "{finish}");
            assert!(
                matches!(parsed.finish_reason, FinishReason::ContentFilter),
                "{finish}: {:?}",
                parsed.finish_reason
            );
        }
    }

    // ── Stream: empty output through the shared guard (D-11a) ──

    fn frame(text: &str, finish: Option<&str>) -> String {
        let mut candidate = json!({"content": {"role": "model", "parts": [{"text": text}]}});
        if let Some(finish) = finish {
            candidate["finishReason"] = json!(finish);
        }
        format!("data: {}\n\n", json!({"candidates": [candidate]}))
    }

    async fn run_stream(sse: String) -> Vec<Result<StreamingResponse, LlmError>> {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/models/gemini-2.5-flash:streamGenerateContent")
            .match_query(Matcher::Any)
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(sse)
            .create_async()
            .await;
        let request = build_request(
            "gemini-2.5-flash",
            PromptType::User(UserPrompt {
                query: "Hello".to_string(),
                context: None,
            }),
        );
        let stream = test_adapter(&server.url())
            .generate_stream(request)
            .await
            .expect("stream must open");
        let mut stream = Box::into_pin(stream);
        let mut items = Vec::new();
        while let Some(item) = stream.next().await {
            items.push(item);
        }
        items
    }

    #[tokio::test]
    async fn gemini_empty_stream_is_empty_completion() {
        let items = run_stream(format!("{}{}", frame("", None), frame("  ", Some("STOP")))).await;
        assert_eq!(items.len(), 2, "{items:?}");
        assert!(items[0].is_ok(), "the non-terminal frame stays yielded");
        match &items[1] {
            Err(LlmError::EmptyCompletion { detail: msg, .. }) => {
                assert!(msg.contains("finish_reason=stop"), "{msg}");
            }
            other => panic!("expected Err(EmptyCompletion) last, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn gemini_empty_stream_on_max_tokens_advises_a_larger_budget() {
        let items = run_stream(frame("", Some("MAX_TOKENS"))).await;
        match items.last() {
            Some(Err(LlmError::EmptyCompletion { detail: msg, .. })) => {
                assert!(msg.contains("larger max_tokens"), "{msg}");
            }
            other => panic!("expected Err(EmptyCompletion), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn gemini_blocked_stream_stays_an_ok_content_filter_terminal_chunk() {
        for finish in [
            "SAFETY",
            "RECITATION",
            "BLOCKLIST",
            "PROHIBITED_CONTENT",
            "SPII",
        ] {
            let items = run_stream(frame("", Some(finish))).await;
            assert_eq!(items.len(), 1, "{finish}: {items:?}");
            let terminal = items[0]
                .as_ref()
                .unwrap_or_else(|e| panic!("{finish}: {e:?}"));
            assert!(
                matches!(terminal.finish_reason, Some(FinishReason::ContentFilter)),
                "{finish}: {:?}",
                terminal.finish_reason
            );
        }
    }

    #[tokio::test]
    async fn gemini_stream_with_text_before_the_terminal_frame_is_fine() {
        let items = run_stream(format!("{}{}", frame("Hel", None), frame("", Some("STOP")))).await;
        assert!(items.iter().all(Result::is_ok), "{items:?}");
        let text: String = items
            .iter()
            .filter_map(|i| i.as_ref().ok())
            .map(|c| c.delta.as_str())
            .collect();
        assert_eq!(text, "Hel");
    }

    // ── Error mapping ──

    #[test]
    fn map_error_429_with_resource_exhausted_status_maps_to_rate_limit_exceeded() {
        let adapter = test_adapter("https://example.invalid");
        let body = json!({
            "error": {"code": 429, "message": "Quota exceeded", "status": "RESOURCE_EXHAUSTED"}
        })
        .to_string();

        let error = adapter.map_error(429, &body);
        assert!(matches!(error, LlmError::RateLimitExceeded { .. }));
    }

    #[test]
    fn map_error_non_429_with_resource_exhausted_status_still_maps_to_rate_limit_exceeded() {
        let adapter = test_adapter("https://example.invalid");
        let body = json!({
            "error": {"code": 200, "message": "Quota exceeded", "status": "RESOURCE_EXHAUSTED"}
        })
        .to_string();

        // The disposition is keyed on the RPC status, not solely the HTTP
        // code, since Google's error envelope is ambiguous on HTTP 429
        // alone (see this adapter's map_error doc comment).
        let error = adapter.map_error(500, &body);
        assert!(matches!(error, LlmError::RateLimitExceeded { .. }));
    }

    #[test]
    fn map_error_400_invalid_argument_maps_to_invalid_prompt() {
        let adapter = test_adapter("https://example.invalid");
        let body = json!({
            "error": {"code": 400, "message": "Bad request", "status": "INVALID_ARGUMENT"}
        })
        .to_string();

        let error = adapter.map_error(400, &body);
        assert!(matches!(error, LlmError::InvalidPrompt(_)));
    }

    #[test]
    fn map_error_404_not_found_maps_to_model_not_available() {
        let adapter = test_adapter("https://example.invalid");
        let body = json!({
            "error": {"code": 404, "message": "Model not found", "status": "NOT_FOUND"}
        })
        .to_string();

        let error = adapter.map_error(404, &body);
        assert!(matches!(error, LlmError::ModelNotAvailable(_)));
    }

    #[test]
    fn map_error_401_permission_denied_maps_to_authentication_error() {
        let adapter = test_adapter("https://example.invalid");
        let body = json!({
            "error": {"code": 401, "message": "Invalid key", "status": "PERMISSION_DENIED"}
        })
        .to_string();

        let error = adapter.map_error(401, &body);
        assert!(matches!(error, LlmError::AuthenticationError(_)));
    }

    #[test]
    fn map_error_echoing_400_never_leaks_the_configured_api_key() {
        let adapter = test_adapter("https://example.invalid");
        let secret = "test-key-abc123";
        let body = json!({
            "error": {
                "code": 400,
                "message": format!("Bad request, header x-goog-api-key: {secret} was rejected"),
                "status": "INVALID_ARGUMENT"
            }
        })
        .to_string();

        let error = adapter.map_error(400, &body);
        let rendered = error.to_string();
        assert!(
            !rendered.contains(secret),
            "map_error leaked the configured API key: {rendered}"
        );
    }

    /// Phase 25 (FT-FR-01, D-03): the successor to
    /// `map_error_unrecognised_status_maps_to_processing_error_carrying_http_code_and_status`.
    /// Gemini's envelope is still parsed first — its `error.message` and
    /// RPC `status` string reach the helper as the body — and the HTTP
    /// status is read from `ProviderError`'s typed field, never from text.
    #[test]
    fn gemini_error_envelope_still_parses_before_mapping() {
        let adapter = test_adapter("https://example.invalid");
        let body = json!({
            "error": {"code": 500, "message": "Internal error", "status": "INTERNAL"}
        })
        .to_string();

        match adapter.map_error(500, &body) {
            LlmError::ProviderError {
                provider,
                status,
                message,
            } => {
                assert_eq!(provider, "gemini");
                assert_eq!(status, 500);
                assert!(
                    message.contains("Internal error"),
                    "envelope message must reach the helper as the body: {message}"
                );
                assert!(
                    message.contains("INTERNAL"),
                    "RPC status string must survive into the diagnostic: {message}"
                );
                assert!(
                    !message.contains("\"code\""),
                    "raw envelope JSON must not be the body: {message}"
                );
            }
            other => panic!("expected ProviderError {{ status: 500 }}, got {other:?}"),
        }
    }

    #[test]
    fn gemini_non_2xx_routes_through_the_shared_mapper() {
        let adapter = test_adapter("https://example.invalid");
        match adapter.map_error(503, "Service Unavailable") {
            LlmError::ProviderError {
                provider, status, ..
            } => {
                assert_eq!(provider, "gemini");
                assert_eq!(status, 503);
            }
            other => panic!("expected ProviderError {{ status: 503 }}, got {other:?}"),
        }
    }

    #[test]
    fn gemini_refused_redirect_is_a_typed_provider_error_naming_the_redirect() {
        let adapter = test_adapter("https://example.invalid");
        match adapter.map_error(302, "moved") {
            LlmError::ProviderError {
                status, message, ..
            } => {
                assert_eq!(status, 302);
                assert!(message.contains("redirect"), "got {message}");
            }
            other => panic!("expected ProviderError {{ status: 302 }}, got {other:?}"),
        }
    }

    // ── Capabilities / identity ──

    #[test]
    fn get_capabilities_reports_text_only_truthfully() {
        let adapter = test_adapter("https://example.invalid");
        let caps = adapter.get_capabilities();

        assert!(caps.supports_streaming);
        assert!(caps.supports_system_messages);
        assert!(!caps.supports_tool_calling);
        assert!(!caps.supports_function_calling);
        assert!(!caps.supports_vision);
        assert!(!caps.supports_embeddings);
        assert_eq!(caps.temperature_range, Some((0.0, 2.0)));
    }

    #[test]
    fn get_provider_name_returns_gemini() {
        let adapter = test_adapter("https://example.invalid");
        assert_eq!(adapter.get_provider_name(), "gemini");
    }

    // ── Retry semantics ──

    #[tokio::test(start_paused = true)]
    async fn execute_with_retry_invokes_operation_exactly_once_on_non_retryable_error() {
        let adapter = test_adapter("https://example.invalid");
        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = Arc::clone(&calls);

        let result: Result<(), LlmError> = adapter
            .execute_with_retry(
                move || {
                    let calls = Arc::clone(&calls_clone);
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Err(LlmError::empty_completion("no text".to_string()))
                    }
                },
                3,
            )
            .await;

        assert!(result.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn execute_with_retry_retries_a_retryable_error_up_to_max_retries() {
        let adapter = test_adapter("https://example.invalid");
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
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    // ── generate(): request shaping + response parsing over mock transport ──

    #[tokio::test]
    async fn generate_posts_to_generate_content_with_x_goog_api_key_header() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/models/gemini-2.5-flash:generateContent")
            .match_header("x-goog-api-key", "test-key-abc123")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "candidates": [{
                        "content": {"role": "model", "parts": [{"text": "Hi there"}]},
                        "finishReason": "STOP"
                    }],
                    "usageMetadata": {
                        "promptTokenCount": 2,
                        "candidatesTokenCount": 2,
                        "totalTokenCount": 4
                    }
                })
                .to_string(),
            )
            .create_async()
            .await;

        let adapter = test_adapter(&server.url());
        let request = build_request(
            "gemini-2.5-flash",
            PromptType::User(UserPrompt {
                query: "Hello".to_string(),
                context: None,
            }),
        );

        let response = adapter.generate(request).await.unwrap();
        assert_eq!(response.content, "Hi there");
        assert!(matches!(response.finish_reason, FinishReason::Stop));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn generate_recitation_finish_reason_is_content_filter_not_stop() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/models/gemini-2.5-flash:generateContent")
            .with_status(200)
            .with_body(
                json!({
                    "candidates": [{
                        "content": {"role": "model", "parts": [{"text": "partial"}]},
                        "finishReason": "RECITATION"
                    }]
                })
                .to_string(),
            )
            .create_async()
            .await;

        let adapter = test_adapter(&server.url());
        let request = build_request(
            "gemini-2.5-flash",
            PromptType::User(UserPrompt {
                query: "Hello".to_string(),
                context: None,
            }),
        );

        let response = adapter.generate(request).await.unwrap();
        // Plan 50-13 flipped this from `Error("RECITATION")` (D-09): a recitation block is a
        // blocked-content finish, so it is `ContentFilter`, and still never `Stop`.
        assert!(
            matches!(response.finish_reason, FinishReason::ContentFilter),
            "expected ContentFilter, got {:?}",
            response.finish_reason
        );
        assert_eq!(response.content, "partial");
    }

    #[tokio::test]
    async fn generate_429_with_resource_exhausted_body_maps_to_rate_limit_exceeded() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/models/gemini-2.5-flash:generateContent")
            .with_status(429)
            .with_body(
                json!({
                    "error": {"code": 429, "message": "Quota exceeded", "status": "RESOURCE_EXHAUSTED"}
                })
                .to_string(),
            )
            .create_async()
            .await;

        let adapter = test_adapter(&server.url());
        let request = build_request(
            "gemini-2.5-flash",
            PromptType::User(UserPrompt {
                query: "Hello".to_string(),
                context: None,
            }),
        );

        let result = adapter.generate(request).await;
        assert!(matches!(result, Err(LlmError::RateLimitExceeded { .. })));
    }

    // ── PACE-01 / D-02: a 429 is surfaced on the FIRST attempt, with its delay ──
    //
    // The Cadence decorator above this adapter owns waiting; a private retry
    // loop that re-sent a 429 would hide thrash below it. Network, timeout
    // and 5xx retries are unchanged (`execute_with_retry_retries_a_retryable_error_up_to_max_retries`).

    fn user_request() -> LlmRequest {
        build_request(
            "gemini-2.5-flash",
            PromptType::User(UserPrompt {
                query: "Hello".to_string(),
                context: None,
            }),
        )
    }

    #[tokio::test(start_paused = true)]
    async fn execute_with_retry_surfaces_rate_limit_exceeded_on_the_first_attempt() {
        let adapter = test_adapter("https://example.invalid");
        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = Arc::clone(&calls);

        let result: Result<(), LlmError> = adapter
            .execute_with_retry(
                move || {
                    let calls = Arc::clone(&calls_clone);
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Err(LlmError::rate_limited(None))
                    }
                },
                3,
            )
            .await;

        assert!(matches!(result, Err(LlmError::RateLimitExceeded { .. })));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn gemini_429_is_surfaced_on_the_first_attempt_with_its_retry_delay() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/models/gemini-2.5-flash:generateContent")
            .with_status(429)
            .with_header("retry-after", "7")
            .with_body(
                json!({
                    "error": {"code": 429, "message": "Quota exceeded", "status": "RESOURCE_EXHAUSTED"}
                })
                .to_string(),
            )
            .expect(1)
            .create_async()
            .await;

        let adapter = test_adapter(&server.url());
        let err = adapter
            .generate(user_request())
            .await
            .expect_err("a 429 must surface as an error");

        mock.assert_async().await;
        assert!(matches!(err, LlmError::RateLimitExceeded { .. }));
        assert_eq!(err.retry_after(), Some(Duration::from_secs(7)));
    }

    #[tokio::test]
    async fn gemini_stream_open_429_is_surfaced_once_with_its_retry_delay() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/models/gemini-2.5-flash:streamGenerateContent")
            .match_query(Matcher::UrlEncoded("alt".to_string(), "sse".to_string()))
            .with_status(429)
            .with_header("retry-after", "7")
            .with_body(
                json!({
                    "error": {"code": 429, "message": "Quota exceeded", "status": "RESOURCE_EXHAUSTED"}
                })
                .to_string(),
            )
            .expect(1)
            .create_async()
            .await;

        let adapter = test_adapter(&server.url());
        let result = adapter.generate_stream(user_request()).await;

        mock.assert_async().await;
        match &result {
            Err(err @ LlmError::RateLimitExceeded { .. }) => {
                assert_eq!(err.retry_after(), Some(Duration::from_secs(7)));
            }
            Ok(_) => panic!("expected a rate-limit error, got Ok(<stream>)"),
            Err(other) => panic!("expected a rate-limit error, got: {other:?}"),
        }
    }

    #[test]
    fn gemini_resource_exhausted_envelope_still_maps_to_rate_limit_exceeded() {
        // The envelope alone (no 429 status, so no header snapshot) still maps to the
        // rate-limit variant, just without a delay.
        let adapter = test_adapter("https://example.invalid");
        let body = json!({
            "error": {"code": 429, "message": "Quota exceeded", "status": "RESOURCE_EXHAUSTED"}
        })
        .to_string();

        let error = adapter.map_error_with_hints(500, &body, None);
        assert!(matches!(error, LlmError::RateLimitExceeded { .. }));
        assert_eq!(error.retry_after(), None);
    }

    #[tokio::test]
    async fn gemini_429_without_retry_after_carries_no_delay() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/models/gemini-2.5-flash:generateContent")
            .with_status(429)
            .with_body("{}")
            .expect(1)
            .create_async()
            .await;

        let adapter = test_adapter(&server.url());
        let err = adapter
            .generate(user_request())
            .await
            .expect_err("a 429 must surface as an error");

        assert!(matches!(err, LlmError::RateLimitExceeded { .. }));
        assert_eq!(err.retry_after(), None);
    }

    // ── Streaming ──

    #[tokio::test]
    async fn generate_stream_posts_with_alt_sse_query_parameter() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/models/gemini-2.5-flash:streamGenerateContent")
            .match_query(Matcher::UrlEncoded("alt".to_string(), "sse".to_string()))
            .expect(1)
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(concat!(
                "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"Hi\"}]},",
                "\"finishReason\":\"STOP\"}]}\n\n",
            ))
            .create_async()
            .await;

        let adapter = test_adapter(&server.url());
        let request = build_request(
            "gemini-2.5-flash",
            PromptType::User(UserPrompt {
                query: "Hello".to_string(),
                context: None,
            }),
        );

        let stream = adapter.generate_stream(request).await.unwrap();
        let mut stream = Box::into_pin(stream);
        while stream.next().await.is_some() {}

        // Proves the query parameter was actually sent — without it Google
        // returns a raw JSON array, not SSE framing.
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn generate_stream_assembles_three_frames_in_wire_order() {
        let sse_body = concat!(
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"Hel\"}]}}]}\n\n",
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"lo \"}]}}]}\n\n",
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"world\"}],\"role\":\"model\"},",
            "\"finishReason\":\"STOP\"}]}\n\n",
        );

        let mut server = Server::new_async().await;
        server
            .mock("POST", "/models/gemini-2.5-flash:streamGenerateContent")
            // Every `generate_stream()` call carries `?alt=sse`; this test
            // doesn't care about the exact query string, only the frames —
            // `generate_stream_posts_with_alt_sse_query_parameter` above
            // asserts the parameter itself.
            .match_query(Matcher::Any)
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(sse_body)
            .create_async()
            .await;

        let adapter = test_adapter(&server.url());
        let request = build_request(
            "gemini-2.5-flash",
            PromptType::User(UserPrompt {
                query: "Hello".to_string(),
                context: None,
            }),
        );

        let stream = adapter.generate_stream(request).await.unwrap();
        let mut stream = Box::into_pin(stream);

        let mut assembled = String::new();
        let mut last_finish_reason = None;
        let mut item_count = 0;
        while let Some(item) = stream.next().await {
            let chunk = item.unwrap();
            item_count += 1;
            assembled.push_str(&chunk.delta);
            if chunk.finish_reason.is_some() {
                last_finish_reason = chunk.finish_reason;
            }
        }

        assert_eq!(item_count, 3);
        assert_eq!(assembled, "Hello world");
        assert!(matches!(last_finish_reason, Some(FinishReason::Stop)));
    }

    #[tokio::test]
    async fn generate_stream_last_frame_usage_equals_the_non_streaming_usage() {
        let sse_body = concat!(
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"Hel\"}]}}]}\n\n",
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"lo \"}]}}]}\n\n",
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"world\"}],\"role\":\"model\"},",
            "\"finishReason\":\"STOP\"}],\"usageMetadata\":{\"promptTokenCount\":310,",
            "\"candidatesTokenCount\":88,\"cachedContentTokenCount\":96,",
            "\"thoughtsTokenCount\":41,\"totalTokenCount\":439}}\n\n",
        );

        let mut server = Server::new_async().await;
        server
            .mock("POST", "/models/gemini-2.5-flash:streamGenerateContent")
            .match_query(Matcher::Any)
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(sse_body)
            .create_async()
            .await;

        let adapter = test_adapter(&server.url());
        let request = build_request(
            "gemini-2.5-flash",
            PromptType::User(UserPrompt {
                query: "Hello".to_string(),
                context: None,
            }),
        );

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
            "the finish-reason chunk and the usage chunk must be the SAME (last) chunk"
        );

        let (expected, _) = map_gemini_usage(&GeminiUsageMetadata {
            prompt_token_count: 310,
            candidates_token_count: 88,
            total_token_count: Some(439),
            tool_use_prompt_token_count: None,
            cached_content_token_count: Some(96),
            thoughts_token_count: Some(41),
        });
        assert_eq!(chunks[usage_indices[0]].usage, Some(expected));
    }

    #[tokio::test]
    async fn generate_stream_without_usage_metadata_yields_terminal_chunk_with_usage_none() {
        let sse_body = concat!(
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"Hi\"}]}}]}\n\n",
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"\"}],\"role\":\"model\"},",
            "\"finishReason\":\"STOP\"}]}\n\n",
        );

        let mut server = Server::new_async().await;
        server
            .mock("POST", "/models/gemini-2.5-flash:streamGenerateContent")
            .match_query(Matcher::Any)
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(sse_body)
            .create_async()
            .await;

        let adapter = test_adapter(&server.url());
        let request = build_request(
            "gemini-2.5-flash",
            PromptType::User(UserPrompt {
                query: "Hello".to_string(),
                context: None,
            }),
        );

        let stream = adapter.generate_stream(request).await.unwrap();
        let mut stream = Box::into_pin(stream);

        let mut terminal_usage = None;
        let mut saw_finish = false;
        while let Some(item) = stream.next().await {
            let chunk = item.unwrap();
            if chunk.finish_reason.is_some() {
                saw_finish = true;
                terminal_usage = chunk.usage;
            }
        }

        assert!(saw_finish, "stream must still end with a finish reason");
        assert_eq!(terminal_usage, None);
    }

    #[tokio::test]
    async fn generate_stream_safety_blocked_frame_terminates_without_error() {
        let sse_body = "data: {\"candidates\":[{\"finishReason\":\"SAFETY\"}]}\n\n";

        let mut server = Server::new_async().await;
        server
            .mock("POST", "/models/gemini-2.5-flash:streamGenerateContent")
            .match_query(Matcher::Any)
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(sse_body)
            .create_async()
            .await;

        let adapter = test_adapter(&server.url());
        let request = build_request(
            "gemini-2.5-flash",
            PromptType::User(UserPrompt {
                query: "Hello".to_string(),
                context: None,
            }),
        );

        let stream = adapter.generate_stream(request).await.unwrap();
        let mut stream = Box::into_pin(stream);

        let mut last_finish_reason = None;
        while let Some(item) = stream.next().await {
            let chunk = item.expect("a safety-blocked frame must not surface as a stream error");
            if chunk.finish_reason.is_some() {
                last_finish_reason = chunk.finish_reason;
            }
        }

        assert!(matches!(
            last_finish_reason,
            Some(FinishReason::ContentFilter)
        ));
    }

    // ── WR-04 (new): a transient stream-open failure retries like generate() does ──
    //
    // Resolved facts (recorded in 17-16-SUMMARY.md per D-00e), read before
    // writing these tests:
    //   1. `generate()` calls `self.execute_with_retry(operation, 3)` —
    //      gemini/adapter.rs:780 — and `execute_with_retry`'s
    //      `attempt >= max_retries` check (gemini/adapter.rs:625) means
    //      exactly `max_retries` = 3 total attempts on a retryable error.
    //   2. HTTP 500 is retryable: it falls through every named arm in
    //      `map_error` (401/403, 400/INVALID_ARGUMENT, 404/NOT_FOUND, 429,
    //      RESOURCE_EXHAUSTED, 300..=399) into the crate-wide
    //      `map_http_status`, which returns `LlmError::ProviderError
    //      { status: 500 }` — outside `execute_with_retry`'s
    //      non-retryable set, so retried (Phase 25 D-03; this comment
    //      previously named the pre-25-05 `ProcessingError` catch-all).
    //   3. No existing streaming mock-transport scaffolding was reused for
    //      this file — Gemini is a bespoke adapter (D-08), never built on
    //      `CompatEngine`, so these tests are new.
    //
    // The "transient" test below derives the expected attempt count by
    // running the same mock failure through `generate()` first, rather than
    // hardcoding `3` — the assertion is "streaming retries like
    // non-streaming", not "streaming retries three times", and does not
    // silently rot if `execute_with_retry`'s cap ever changes.

    // Not `start_paused = true`: this test exercises real network round
    // -trips against a `mockito` server, and pausing the tokio clock races
    // the retry backoff timer against `reqwest`'s own 60s request timeout —
    // observed directly: the third attempt's response never arrived before
    // the paused clock auto-advanced past the client timeout, so the call
    // failed with `Timeout` instead of exercising all `max_retries`
    // attempts. Real (unpaused) time makes each ~1-3s of backoff actually
    // elapse, which is what the synthetic-closure retry tests above use
    // `start_paused = true` to avoid paying.
    #[tokio::test]
    async fn generate_stream_retries_a_transient_open_failure_as_many_times_as_generate() {
        let mut generate_server = Server::new_async().await;
        let generate_calls = Arc::new(AtomicU32::new(0));
        let generate_calls_clone = Arc::clone(&generate_calls);
        generate_server
            .mock("POST", "/models/gemini-2.5-flash:generateContent")
            .match_query(Matcher::Any)
            .with_status(500)
            .with_body_from_request(move |_req| {
                generate_calls_clone.fetch_add(1, Ordering::SeqCst);
                br#"{"error":{"message":"transient failure"}}"#.to_vec()
            })
            .create_async()
            .await;

        let generate_adapter = test_adapter(&generate_server.url());
        let generate_request = build_request(
            "gemini-2.5-flash",
            PromptType::User(UserPrompt {
                query: "Hello".to_string(),
                context: None,
            }),
        );
        let generate_result = generate_adapter.generate(generate_request).await;
        assert!(generate_result.is_err());
        let generate_attempt_count = generate_calls.load(Ordering::SeqCst);

        let mut stream_server = Server::new_async().await;
        let stream_calls = Arc::new(AtomicU32::new(0));
        let stream_calls_clone = Arc::clone(&stream_calls);
        let stream_mock = stream_server
            .mock("POST", "/models/gemini-2.5-flash:streamGenerateContent")
            .match_query(Matcher::Any)
            .with_status(500)
            .with_body_from_request(move |_req| {
                stream_calls_clone.fetch_add(1, Ordering::SeqCst);
                br#"{"error":{"message":"transient failure"}}"#.to_vec()
            })
            // Load-bearing (D-00e): today `generate_stream` makes exactly
            // ONE request on a transient failure, not
            // `generate_attempt_count` — this `.expect()` is what fails in
            // the RED state, not the `Result` assertion below.
            .expect(generate_attempt_count as usize)
            .create_async()
            .await;

        let stream_adapter = test_adapter(&stream_server.url());
        let stream_request = build_request(
            "gemini-2.5-flash",
            PromptType::User(UserPrompt {
                query: "Hello".to_string(),
                context: None,
            }),
        );
        let stream_result = stream_adapter.generate_stream(stream_request).await;

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
            .mock("POST", "/models/gemini-2.5-flash:streamGenerateContent")
            .match_query(Matcher::Any)
            .with_status(401)
            .with_body(
                json!({
                    "error": {
                        "code": 401,
                        "status": "UNAUTHENTICATED",
                        "message": "Request had invalid authentication credentials."
                    }
                })
                .to_string(),
            )
            // Load-bearing (D-00e): passes today (there is no retry at all)
            // and must keep passing after the fix — this is the
            // credential-replay guard, proving an authentication failure on
            // stream open is attempted exactly once.
            .expect(1)
            .create_async()
            .await;

        let adapter = test_adapter(&server.url());
        let request = build_request(
            "gemini-2.5-flash",
            PromptType::User(UserPrompt {
                query: "Hello".to_string(),
                context: None,
            }),
        );

        let result = adapter.generate_stream(request).await;

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
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"Hel\"}]}}]}\n\n",
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"lo \"}]}}]}\n\n",
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"world\"}],\"role\":\"model\"},",
            "\"finishReason\":\"STOP\"}]}\n\n",
        );

        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/models/gemini-2.5-flash:streamGenerateContent")
            .match_query(Matcher::Any)
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(sse_body)
            // Load-bearing (D-00e): proves the retried-open shape does not
            // double-open a working connection.
            .expect(1)
            .create_async()
            .await;

        let adapter = test_adapter(&server.url());
        let request = build_request(
            "gemini-2.5-flash",
            PromptType::User(UserPrompt {
                query: "Hello".to_string(),
                context: None,
            }),
        );

        let stream = adapter.generate_stream(request).await.unwrap();
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

    // ── Model list: live catalog vs. curated fallback (D-13/D-14) ──

    #[tokio::test]
    async fn get_available_models_returns_two_live_entries_without_models_prefix() {
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/models")
            .with_status(200)
            .with_body(
                json!({
                    "models": [
                        {"name": "models/gemini-2.5-flash"},
                        {"name": "models/gemini-2.5-pro"}
                    ]
                })
                .to_string(),
            )
            .create_async()
            .await;

        let adapter = test_adapter(&server.url());
        let models = adapter.get_available_models().await.unwrap();
        assert_eq!(
            models,
            vec!["gemini-2.5-flash".to_string(), "gemini-2.5-pro".to_string()]
        );
    }

    #[tokio::test]
    async fn get_available_models_second_call_does_not_hit_the_mock_again() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/models")
            .expect(1)
            .with_status(200)
            .with_body(json!({"models": [{"name": "models/gemini-2.5-flash"}]}).to_string())
            .create_async()
            .await;

        let adapter = test_adapter(&server.url());
        let _first = adapter.get_available_models().await.unwrap();
        let _second = adapter.get_available_models().await.unwrap();

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn get_available_models_two_concurrent_first_calls_hit_the_mock_exactly_once() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/models")
            .expect(1)
            .with_status(200)
            .with_body(json!({"models": [{"name": "models/gemini-2.5-flash"}]}).to_string())
            .create_async()
            .await;

        let adapter = test_adapter(&server.url());
        let (first, second) = tokio::join!(
            adapter.get_available_models(),
            adapter.get_available_models()
        );
        first.unwrap();
        second.unwrap();

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn get_available_models_falls_back_to_curated_list_on_failure() {
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/models")
            .with_status(500)
            .with_body("internal error")
            .create_async()
            .await;

        let adapter = test_adapter(&server.url());
        let models = adapter.get_available_models().await.unwrap();
        assert_eq!(
            models,
            GEMINI_FALLBACK_MODELS
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        );

        assert!(adapter.validate_model("gemini-3.5-flash").await.unwrap());
    }

    #[tokio::test]
    async fn validate_model_accepts_a_model_present_only_in_the_live_list() {
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/models")
            .with_status(200)
            .with_body(json!({"models": [{"name": "models/gemini-3.1-flash-lite"}]}).to_string())
            .create_async()
            .await;

        let adapter = test_adapter(&server.url());
        assert!(
            adapter
                .validate_model("gemini-3.1-flash-lite")
                .await
                .unwrap()
        );
    }

    // ── validate_model_identifier: pure-logic tests over the guard itself ──

    #[test]
    fn validate_model_identifier_accepts_the_default_and_every_fallback_model() {
        for model in
            std::iter::once(GEMINI_DEFAULT_MODEL).chain(GEMINI_FALLBACK_MODELS.iter().copied())
        {
            assert!(
                validate_model_identifier(model).is_ok(),
                "expected Ok for shipped default/fallback {model:?}"
            );
        }
    }

    #[test]
    fn validate_model_identifier_rejects_each_url_metacharacter() {
        let metacharacters = ['/', '?', '#', ':', '@', '%', '&', '=', ' ', '\\', '\n'];
        for c in metacharacters {
            let hostile = format!("gemini-2.5-flash{c}x");
            let result = validate_model_identifier(&hostile);
            assert!(
                matches!(result, Err(LlmError::InvalidPrompt(_))),
                "expected LlmError::InvalidPrompt for metacharacter {c:?} in {hostile:?}, got {result:?}"
            );
        }
    }

    #[test]
    fn validate_model_identifier_rejects_a_value_with_no_alphanumeric_character() {
        for value in [".", "..", "---", "_"] {
            let result = validate_model_identifier(value);
            assert!(
                matches!(result, Err(LlmError::InvalidPrompt(_))),
                "expected LlmError::InvalidPrompt for {value:?} (no alphanumeric char), got {result:?}"
            );
        }
    }

    #[test]
    fn validate_model_identifier_rejects_a_long_multibyte_value_without_panicking() {
        // A 2,000-character multi-byte value. The test completing at all —
        // rather than panicking on a mid-codepoint byte slice — is part of
        // what this test proves.
        let hostile: String = "\u{1F5E1}".repeat(2000);
        let result = validate_model_identifier(&hostile);

        let message = match result {
            Err(LlmError::InvalidPrompt(msg)) => msg,
            other => panic!("expected LlmError::InvalidPrompt, got {other:?}"),
        };

        // The embedded excerpt is capped at RESPONSE_EXCERPT_CHAR_BUDGET
        // characters; the surrounding sentence and elision marker add a
        // small, fixed amount on top. The bound below allows for that
        // fixed prefix/suffix without allowing the excerpt itself to grow
        // unbounded.
        assert!(
            message.chars().count() <= RESPONSE_EXCERPT_CHAR_BUDGET + 400,
            "rejection message was not bounded: {} chars",
            message.chars().count()
        );
    }

    #[test]
    fn validate_model_identifier_accepts_every_character_of_the_allowed_set() {
        assert!(validate_model_identifier("aZ0.9_x-1").is_ok());
    }

    // ── CR-01: a caller-supplied model identifier must never reach the wire
    //    unescaped — regression tests proving the guard above is actually
    //    wired into both LlmPort methods. See
    //    `.planning/phases/17-additional-llm-provider-adapters/17-REVIEW.md`
    //    §CR-01 and `17-VERIFICATION.md`. ──

    #[tokio::test]
    async fn generate_rejects_a_model_containing_a_path_separator_without_issuing_a_request() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", Matcher::Any)
            .match_query(Matcher::Any)
            .expect(0)
            .with_status(200)
            .with_body("{}")
            .create_async()
            .await;

        let adapter = test_adapter(&server.url());
        let request = build_request(
            "gemini-2.5-flash/../v1beta/models/other",
            PromptType::Text(TextPrompt {
                content: "Hello".to_string(),
                role: PromptRole::User,
            }),
        );

        let result = adapter.generate(request).await;
        assert!(
            matches!(&result, Err(LlmError::InvalidPrompt(_))),
            "expected LlmError::InvalidPrompt, got {:?}",
            result.err()
        );

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn generate_stream_rejects_a_model_containing_a_query_delimiter_without_issuing_a_request()
     {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", Matcher::Any)
            .match_query(Matcher::Any)
            .expect(0)
            .with_status(200)
            .with_body("{}")
            .create_async()
            .await;

        let adapter = test_adapter(&server.url());
        let request = build_request(
            "gemini-2.5-flash?alt=json",
            PromptType::Text(TextPrompt {
                content: "Hello".to_string(),
                role: PromptRole::User,
            }),
        );

        let result = adapter.generate_stream(request).await;
        assert!(
            matches!(&result, Err(LlmError::InvalidPrompt(_))),
            "expected LlmError::InvalidPrompt, got {:?}",
            result.as_ref().err()
        );

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn generate_rejects_a_model_containing_a_colon_operation_suffix() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", Matcher::Any)
            .match_query(Matcher::Any)
            .expect(0)
            .with_status(200)
            .with_body("{}")
            .create_async()
            .await;

        let adapter = test_adapter(&server.url());
        let request = build_request(
            "gemini-2.5-flash:streamGenerateContent",
            PromptType::Text(TextPrompt {
                content: "Hello".to_string(),
                role: PromptRole::User,
            }),
        );

        let result = adapter.generate(request).await;
        assert!(
            matches!(&result, Err(LlmError::InvalidPrompt(_))),
            "expected LlmError::InvalidPrompt, got {:?}",
            result.err()
        );

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn generate_rejects_a_model_containing_a_fragment_delimiter() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", Matcher::Any)
            .match_query(Matcher::Any)
            .expect(0)
            .with_status(200)
            .with_body("{}")
            .create_async()
            .await;

        let adapter = test_adapter(&server.url());
        let request = build_request(
            "gemini-2.5-flash#anchor",
            PromptType::Text(TextPrompt {
                content: "Hello".to_string(),
                role: PromptRole::User,
            }),
        );

        let result = adapter.generate(request).await;
        assert!(
            matches!(&result, Err(LlmError::InvalidPrompt(_))),
            "expected LlmError::InvalidPrompt, got {:?}",
            result.err()
        );

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn generate_rejects_an_empty_model() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", Matcher::Any)
            .match_query(Matcher::Any)
            .expect(0)
            .with_status(200)
            .with_body("{}")
            .create_async()
            .await;

        let adapter = test_adapter(&server.url());
        let request = build_request(
            "",
            PromptType::Text(TextPrompt {
                content: "Hello".to_string(),
                role: PromptRole::User,
            }),
        );

        let result = adapter.generate(request).await;
        assert!(
            matches!(&result, Err(LlmError::InvalidPrompt(_))),
            "expected LlmError::InvalidPrompt, got {:?}",
            result.err()
        );

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn generate_rejects_a_model_containing_a_non_ascii_homoglyph() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", Matcher::Any)
            .match_query(Matcher::Any)
            .expect(0)
            .with_status(200)
            .with_body("{}")
            .create_async()
            .await;

        let adapter = test_adapter(&server.url());
        // Cyrillic small letter A (U+0430) in place of the second ASCII `a`
        // — written as an escape, not a raw glyph, so a reviewer can see
        // which character is which.
        let request = build_request(
            "g\u{0430}mini-2.5-flash",
            PromptType::Text(TextPrompt {
                content: "Hello".to_string(),
                role: PromptRole::User,
            }),
        );

        let result = adapter.generate(request).await;
        assert!(
            matches!(&result, Err(LlmError::InvalidPrompt(_))),
            "expected LlmError::InvalidPrompt, got {:?}",
            result.err()
        );

        mock.assert_async().await;
    }

    /// Positive control: proves the guard does not over-reject a value whose
    /// characters are all in the allowed set. Must pass both before and
    /// after Task 2 wires the guard in.
    #[tokio::test]
    async fn generate_accepts_a_model_whose_characters_are_all_in_the_allowed_set() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock(
                "POST",
                "/models/gemini-2.5-flash_preview.01-x:generateContent",
            )
            .match_header("x-goog-api-key", "test-key-abc123")
            .expect(1)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "candidates": [{
                        "content": {"role": "model", "parts": [{"text": "Hi there"}]},
                        "finishReason": "STOP"
                    }],
                    "usageMetadata": {
                        "promptTokenCount": 2,
                        "candidatesTokenCount": 2,
                        "totalTokenCount": 4
                    }
                })
                .to_string(),
            )
            .create_async()
            .await;

        let adapter = test_adapter(&server.url());
        let request = build_request(
            "gemini-2.5-flash_preview.01-x",
            PromptType::Text(TextPrompt {
                content: "Hello".to_string(),
                role: PromptRole::User,
            }),
        );

        let result = adapter.generate(request).await;
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());

        mock.assert_async().await;
    }

    // ── WR-04: redirect-following credential replay (plan 17-10) ──

    #[tokio::test]
    async fn gemini_does_not_replay_the_api_key_header_to_a_redirect_target() {
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
                    "candidates": [{
                        "content": {"role": "model", "parts": [{"text": "should never be seen"}]},
                        "finishReason": "STOP"
                    }],
                    "usageMetadata": {
                        "promptTokenCount": 1,
                        "candidatesTokenCount": 1,
                        "totalTokenCount": 2
                    }
                })
                .to_string(),
            )
            .expect(0)
            .create_async()
            .await;

        let mut primary = Server::new_async().await;
        let primary_mock = primary
            .mock("POST", Matcher::Any)
            .match_query(Matcher::Any)
            .with_status(302)
            .with_header(
                "location",
                &format!(
                    "{}/models/gemini-2.5-flash:generateContent",
                    redirect_target.url()
                ),
            )
            // The refused-redirect arm's error (`ProviderError { 3xx }`
            // since Phase 25 D-03; `ProcessingError` before) is outside the
            // adapter's non-retryable set, so the fixed adapter may hit
            // `primary` more than once (up to `max_retries` = 3); before
            // the fix, the redirect was followed transparently and the call
            // succeeded on the first attempt. `expect_at_least(1)` holds in
            // both the RED and GREEN states.
            .expect_at_least(1)
            .create_async()
            .await;

        let adapter = test_adapter(&primary.url());
        let request = build_request(
            "gemini-2.5-flash",
            PromptType::User(UserPrompt {
                query: "Hello".to_string(),
                context: None,
            }),
        );

        let result = adapter.generate(request).await;

        // Load-bearing assertions FIRST (D-00e) — see the sibling Kimi
        // test's comment for why: today the default redirect policy follows
        // the 302 and the redirect target answers with a well-formed
        // response, so `result` is actually `Ok` in the RED state — it is
        // this `.expect(0)` mock assertion, proving the credential-bearing
        // request WAS forwarded, that fails in the RED state, not the
        // `result.is_err()` check below.
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

    // ── WR-03: credential-failure classification (17-11) ──
    //
    // Six regression tests proving two defects at map_error's 401|403 and
    // 400/INVALID_ARGUMENT arms: an unrecognised auth failure (any status
    // other than PERMISSION_DENIED, or no parseable envelope at all) falls
    // through to the retryable ProcessingError catch-all, and Google's
    // documented invalid-key shape (400/INVALID_ARGUMENT with an API-key
    // message) reads as a bad prompt rather than a bad credential. Two
    // controls (already passing) pin the boundary the fix must not cross.

    #[test]
    fn map_error_400_invalid_argument_naming_an_invalid_api_key_maps_to_authentication_error() {
        let adapter = test_adapter("https://example.invalid");
        let body = json!({
            "error": {
                "code": 400,
                "status": "INVALID_ARGUMENT",
                "message": "API key not valid. Please pass a valid API key."
            }
        })
        .to_string();

        let error = adapter.map_error(400, &body);
        assert!(
            matches!(error, LlmError::AuthenticationError(_)),
            "expected AuthenticationError, got: {error:?}"
        );
    }

    #[test]
    fn map_error_401_unauthenticated_maps_to_authentication_error() {
        let adapter = test_adapter("https://example.invalid");
        let body = json!({
            "error": {
                "code": 401,
                "status": "UNAUTHENTICATED",
                "message": "Request had invalid authentication credentials."
            }
        })
        .to_string();

        let error = adapter.map_error(401, &body);
        assert!(
            matches!(error, LlmError::AuthenticationError(_)),
            "expected AuthenticationError, got: {error:?}"
        );
    }

    #[test]
    fn map_error_403_with_no_parseable_envelope_maps_to_authentication_error() {
        let adapter = test_adapter("https://example.invalid");
        // The shape a gateway or proxy returns — not JSON at all — and the
        // one most likely to reach the catch-all today.
        let body = "Forbidden";

        let error = adapter.map_error(403, body);
        assert!(
            matches!(error, LlmError::AuthenticationError(_)),
            "expected AuthenticationError, got: {error:?}"
        );
    }

    #[test]
    fn map_error_400_invalid_argument_with_a_prompt_complaint_still_maps_to_invalid_prompt() {
        // Adjacency/ordering control (must pass both before and after):
        // a genuine Gemini prompt-shaped complaint must never be swallowed
        // by the new credential arm.
        let adapter = test_adapter("https://example.invalid");
        let body = json!({
            "error": {
                "code": 400,
                "status": "INVALID_ARGUMENT",
                "message": "Unable to submit request because it has an empty contents field."
            }
        })
        .to_string();

        let error = adapter.map_error(400, &body);
        assert!(
            matches!(error, LlmError::InvalidPrompt(_)),
            "expected InvalidPrompt, got: {error:?}"
        );
    }

    #[test]
    fn map_error_400_echoing_the_api_key_header_name_still_maps_to_invalid_prompt() {
        // Over-trigger control (must pass both before and after): the
        // hyphenated header name `x-goog-api-key` must never be a
        // credential-shape signature — an echoed request body carries that
        // string for reasons unrelated to credential validity, and matching
        // on it would send an operator to rotate a working key.
        let adapter = test_adapter("https://example.invalid");
        let body = json!({
            "error": {
                "code": 400,
                "status": "INVALID_ARGUMENT",
                "message": "Bad request, header x-goog-api-key was present"
            }
        })
        .to_string();

        let error = adapter.map_error(400, &body);
        assert!(
            matches!(error, LlmError::InvalidPrompt(_)),
            "expected InvalidPrompt, got: {error:?}"
        );
    }

    #[tokio::test]
    async fn generate_does_not_retry_an_unrecognised_authentication_failure() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", Matcher::Any)
            .match_query(Matcher::Any)
            .with_status(401)
            .with_body(
                json!({
                    "error": {
                        "code": 401,
                        "status": "UNAUTHENTICATED",
                        "message": "Request had invalid authentication credentials."
                    }
                })
                .to_string(),
            )
            // Load-bearing (D-00e): today this misclassified error is
            // retryable, so the mock receives four requests, not one — the
            // finding's actual cost made visible in the RED state.
            .expect(1)
            .create_async()
            .await;

        let adapter = test_adapter(&server.url());
        let request = build_request(
            "gemini-2.5-flash",
            PromptType::User(UserPrompt {
                query: "Hello".to_string(),
                context: None,
            }),
        );

        let result = adapter.generate(request).await;

        mock.assert_async().await;

        assert!(
            matches!(result, Err(LlmError::AuthenticationError(_))),
            "expected AuthenticationError, got: {result:?}"
        );
    }

    // ── Phase 50 (PARITY-02, ADR-0061, D-01): the per-model parameter policy ──

    mod parameter_policy {
        use super::*;
        use crate::registry::UNPROFILED_MODEL_METADATA_KEY;
        use crate::sampling::{PARAMS_OMITTED_METADATA_KEY, RESPONSE_FORMAT_ADJUSTED_METADATA_KEY};
        use paladin_core::platform::container::prompt::PromptParameters;
        use serde_json::Value;
        use std::sync::Mutex;

        const MODEL: &str = "gemini-2.5-flash";

        fn request_with(parameters: PromptParameters) -> LlmRequest {
            let mut request = build_request(
                MODEL,
                PromptType::User(UserPrompt {
                    query: "Hello".to_string(),
                    context: None,
                }),
            );
            request.prompt.entity.data.parameters = parameters;
            request
        }

        fn unset() -> PromptParameters {
            PromptParameters {
                max_tokens: None,
                temperature: None,
                top_p: None,
                frequency_penalty: None,
                presence_penalty: None,
                stop_sequences: None,
            }
        }

        fn completion_json() -> Vec<u8> {
            json!({
                "candidates": [{
                    "content": {"role": "model", "parts": [{"text": "ok"}]},
                    "finishReason": "STOP"
                }],
                "usageMetadata": {
                    "promptTokenCount": 1,
                    "candidatesTokenCount": 1,
                    "totalTokenCount": 2
                }
            })
            .to_string()
            .into_bytes()
        }

        /// Runs `generate()` against a mock `generateContent` endpoint, returning the outgoing
        /// JSON body and the response metadata.
        async fn generate_capture(
            request: LlmRequest,
        ) -> (Value, std::collections::HashMap<String, String>) {
            let mut server = Server::new_async().await;
            let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
            let slot = Arc::clone(&captured);
            server
                .mock("POST", "/models/gemini-2.5-flash:generateContent")
                .with_status(200)
                .with_body_from_request(move |req| {
                    *slot.lock().unwrap() =
                        Some(req.utf8_lossy_body().unwrap_or_default().into_owned());
                    completion_json()
                })
                .create_async()
                .await;

            let response = test_adapter(&server.url())
                .generate(request)
                .await
                .expect("the mock answers a well-formed candidate");
            let body = captured
                .lock()
                .unwrap()
                .take()
                .expect("the mock must have been called once");
            (
                serde_json::from_str(&body).expect("the body must be JSON"),
                response.metadata,
            )
        }

        /// Runs `generate_stream()` against a mock endpoint and returns the outgoing JSON body.
        async fn stream_capture(request: LlmRequest) -> Value {
            let mut server = Server::new_async().await;
            let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
            let slot = Arc::clone(&captured);
            server
                .mock("POST", "/models/gemini-2.5-flash:streamGenerateContent?alt=sse")
                .with_status(200)
                .with_header("content-type", "text/event-stream")
                .with_body_from_request(move |req| {
                    *slot.lock().unwrap() =
                        Some(req.utf8_lossy_body().unwrap_or_default().into_owned());
                    b"data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"ok\"}]},\"finishReason\":\"STOP\"}]}\n\n".to_vec()
                })
                .create_async()
                .await;

            let stream = test_adapter(&server.url())
                .generate_stream(request)
                .await
                .expect("the stream must open");
            let mut stream = Box::into_pin(stream);
            while stream.next().await.is_some() {}

            let body = captured
                .lock()
                .unwrap()
                .take()
                .expect("the mock must have been called once");
            serde_json::from_str(&body).expect("the body must be JSON")
        }

        #[test]
        fn gemini_model_profile_is_always_some_and_matches_what_the_wire_sends() {
            let profile = test_adapter("https://example.invalid")
                .model_profile("any-model")
                .expect("gemini profiles every model");
            assert_eq!(profile.temperature, ParamSupport::Range(0.0, 2.0));
            assert_eq!(profile.top_p, ParamSupport::Range(0.0, 1.0));
            assert_eq!(profile.frequency_penalty, ParamSupport::NotSent);
            assert_eq!(profile.presence_penalty, ParamSupport::NotSent);
            assert_eq!(profile.stop_sequences, StopSupport::Supported { max: None });
            assert_eq!(profile.context_window, Some(1_048_576));
            assert!(profile.json_object && profile.json_schema);
        }

        #[tokio::test]
        async fn gemini_stop_sequences_keep_declared_order() {
            let mut parameters = unset();
            // A duplicate, deliberately: order and duplicates are the caller's.
            parameters.stop_sequences = Some(vec![
                "END".to_string(),
                "STOP".to_string(),
                "END".to_string(),
            ]);
            parameters.temperature = Some(0.5);
            let (body, metadata) = generate_capture(request_with(parameters)).await;

            assert_eq!(
                body["generationConfig"]["stopSequences"],
                json!(["END", "STOP", "END"])
            );
            assert_eq!(body["generationConfig"]["temperature"], 0.5);
            assert!(!metadata.contains_key(PARAMS_OMITTED_METADATA_KEY));
        }

        #[tokio::test]
        async fn gemini_stream_follows_the_profile() {
            let mut parameters = unset();
            parameters.temperature = Some(2.5);
            parameters.top_p = Some(0.9);
            parameters.stop_sequences = Some(vec!["END".to_string(), "STOP".to_string()]);
            let body = stream_capture(request_with(parameters)).await;

            let config = body["generationConfig"]
                .as_object()
                .expect("generationConfig is present");
            assert!(
                !config.contains_key("temperature"),
                "the streaming body omits an inadmissible temperature: {body}"
            );
            assert!((config["topP"].as_f64().unwrap() - 0.9).abs() < 1e-6);
            assert_eq!(config["stopSequences"], json!(["END", "STOP"]));
        }

        #[tokio::test]
        async fn gemini_out_of_range_temperature_is_omitted_and_recorded() {
            let mut at_edge = unset();
            at_edge.temperature = Some(2.0);
            let (body, metadata) = generate_capture(request_with(at_edge)).await;
            assert_eq!(body["generationConfig"]["temperature"], 2.0);
            assert!(!metadata.contains_key(PARAMS_OMITTED_METADATA_KEY));

            let mut past_edge = unset();
            past_edge.temperature = Some(2.5);
            past_edge.top_p = Some(0.9);
            let (body, metadata) = generate_capture(request_with(past_edge)).await;
            let config = body["generationConfig"]
                .as_object()
                .expect("topP keeps generationConfig present");
            assert!(
                !config.contains_key("temperature"),
                "2.5 is omitted, never clamped to 2.0: {body}"
            );
            assert_eq!(
                metadata
                    .get(PARAMS_OMITTED_METADATA_KEY)
                    .map(String::as_str),
                Some("temperature")
            );
        }

        #[tokio::test]
        async fn gemini_penalties_never_appear_on_the_wire() {
            // `PromptParameters::default()` carries penalties of 0.0; the generationConfig this
            // adapter builds has no penalty field, so the omission is recorded, not silent.
            let (body, metadata) =
                generate_capture(request_with(PromptParameters::default())).await;

            let text = body.to_string();
            assert!(
                !text.contains("enalty"),
                "no penalty field on the wire: {text}"
            );
            assert_eq!(
                metadata
                    .get(PARAMS_OMITTED_METADATA_KEY)
                    .map(String::as_str),
                Some("frequency_penalty,presence_penalty")
            );
        }

        #[tokio::test]
        async fn gemini_json_schema_still_sets_mime_type_and_schema() {
            let schema = json!({"type": "object"});
            let request = request_with(unset()).with_response_format(ResponseFormat::JsonSchema {
                name: "answer".to_string(),
                schema: schema.clone(),
                strict: true,
            });
            let (body, metadata) = generate_capture(request).await;

            assert_eq!(
                body["generationConfig"]["responseMimeType"],
                "application/json"
            );
            assert_eq!(body["generationConfig"]["responseSchema"], schema);
            assert!(
                !metadata.contains_key(RESPONSE_FORMAT_ADJUSTED_METADATA_KEY),
                "nothing changed on the wire, so nothing is recorded: {metadata:?}"
            );
        }

        /// Phase 50, owner reading R-1 (option-b): a streamed call stamps the visibility keys
        /// the buffered path would, on the terminal (finish-reason) chunk only.
        #[tokio::test]
        async fn gemini_stream_terminal_chunk_carries_metadata() {
            let mut server = Server::new_async().await;
            server
                .mock("POST", "/models/gemini-2.5-flash:streamGenerateContent?alt=sse")
                .with_status(200)
                .with_header("content-type", "text/event-stream")
                .with_body(
                    "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"Hi\"}]}}]}\n\n\
                     data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"\"}]},\"finishReason\":\"STOP\"}]}\n\n",
                )
                .create_async()
                .await;

            let mut parameters = unset();
            parameters.temperature = Some(2.5); // above the profile's (0.0, 2.0)
            let stream = test_adapter(&server.url())
                .generate_stream(request_with(parameters))
                .await
                .expect("the stream must open");
            let mut stream = Box::into_pin(stream);
            let mut chunks = Vec::new();
            while let Some(item) = stream.next().await {
                chunks.push(item.expect("every scripted item is Ok"));
            }

            let terminal = chunks.last().expect("a terminal chunk");
            assert!(terminal.finish_reason.is_some());
            assert_eq!(
                terminal
                    .metadata
                    .get(PARAMS_OMITTED_METADATA_KEY)
                    .map(String::as_str),
                Some("temperature")
            );
            assert_eq!(
                terminal
                    .metadata
                    .get(UNPROFILED_MODEL_METADATA_KEY)
                    .map(String::as_str),
                Some(MODEL)
            );
            for content in &chunks[..chunks.len() - 1] {
                assert!(content.metadata.is_empty(), "content chunk: {content:?}");
            }
        }

        #[tokio::test]
        async fn gemini_unprofiled_model_is_stamped() {
            let (_, metadata) = generate_capture(request_with(unset())).await;
            assert_eq!(
                metadata
                    .get(UNPROFILED_MODEL_METADATA_KEY)
                    .map(String::as_str),
                Some(MODEL),
                "no seeded entry exists yet, so the model runs on the adapter-wide fallback"
            );
        }

        #[test]
        fn gemini_max_tokens_above_a_profiled_maximum_is_refused_by_name() {
            use crate::registry::{ModelOverride, ModelRegistry};

            let mut adapter = test_adapter("https://example.invalid");
            let over = ModelOverride {
                max_output_tokens: Some(1000),
                ..ModelOverride::default()
            };
            adapter.model_registry =
                ModelRegistry::new(GEMINI_PROVIDER, &[], fallback_profile(None))
                    .with_overrides(BTreeMap::from([(MODEL.to_string(), over)]))
                    .expect("a plain max_output_tokens override is valid");

            let mut parameters = unset();
            parameters.max_tokens = Some(4000);
            match adapter.build_request(&request_with(parameters)) {
                Err(LlmError::InvalidPrompt(message)) => {
                    assert!(message.contains("gemini"), "{message}");
                    assert!(message.contains(MODEL), "{message}");
                    assert!(message.contains("max_tokens"), "{message}");
                }
                other => panic!("expected InvalidPrompt, got {other:?}"),
            }
        }
    }

    /// Phase 51 (REASON-02, REASON-03, REASON-06): an explicit effort reaches `generationConfig`
    /// as `thinkingLevel` or `thinkingBudget` exactly as the model's profile states, or is
    /// refused before any HTTP call.
    mod reasoning_effort {
        use super::*;
        use crate::reasoning::{
            REASONING_EFFECTIVE_METADATA_KEY, REASONING_REQUESTED_METADATA_KEY,
        };
        use paladin_core::platform::container::prompt::PromptParameters;
        use paladin_core::platform::container::transience::Transience;
        use paladin_ports::output::llm_port::ReasoningEffort::{High, Low, Medium, Minimal};
        use paladin_ports::output::llm_port::{ReasoningEffort, ReasoningFallback};
        use paladin_ports::output::model_profile::{BudgetBounds, EffortLevels, EffortSupport};
        use serde_json::Value;
        use std::sync::Mutex;

        const LEVELS_MODEL: &str = "gemini-3.8-flash";
        const BUDGET_MODEL: &str = "gemini-2.5-flash";
        const UNMEASURED_MODEL: &str = "gemini-unprofiled-test-model";

        fn request_for(model: &str) -> LlmRequest {
            build_request(
                model,
                PromptType::User(UserPrompt {
                    query: "Hello".to_string(),
                    context: None,
                }),
            )
        }

        fn with_max_tokens(mut request: LlmRequest, max_tokens: u32) -> LlmRequest {
            request.prompt.entity.data.parameters.max_tokens = Some(max_tokens);
            request
        }

        /// The levels shape of the 3.x line: three levels and `auto`, no `off`, no budget.
        fn levels_shape() -> EffortSupport {
            EffortSupport::Supported(EffortLevels::new([Low, Medium, High]).with_auto(true))
        }

        /// The budget shape of the 2.5 line: no levels, `off`, `auto` and a budget span.
        fn budget_shape() -> EffortSupport {
            EffortSupport::Supported(
                EffortLevels::new([])
                    .with_off(true)
                    .with_auto(true)
                    .with_budget(BudgetBounds::new(1, 24_576)),
            )
        }

        fn adapter_over(base_url: &str, overrides: Vec<(&str, EffortSupport)>) -> GeminiAdapter {
            let map: BTreeMap<String, ModelOverride> = overrides
                .into_iter()
                .map(|(model, effort)| {
                    (
                        model.to_string(),
                        ModelOverride {
                            effort: Some(effort),
                            ..ModelOverride::default()
                        },
                    )
                })
                .collect();
            GeminiAdapter::new(test_config(base_url).with_model_overrides(map))
                .expect("a valid override builds")
        }

        fn ok_body() -> Vec<u8> {
            json!({
                "candidates": [{
                    "content": {"role": "model", "parts": [{"text": "ok"}]},
                    "finishReason": "STOP"
                }],
                "usageMetadata": {
                    "promptTokenCount": 1,
                    "candidatesTokenCount": 1,
                    "totalTokenCount": 2
                }
            })
            .to_string()
            .into_bytes()
        }

        /// Send `request` through an adapter carrying `overrides`; returns the raw request body
        /// (`None` when no HTTP call was made) and the result.
        async fn send_raw(
            overrides: Vec<(&str, EffortSupport)>,
            request: LlmRequest,
        ) -> (Option<String>, Result<LlmResponse, LlmError>) {
            let mut server = Server::new_async().await;
            let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
            let slot = Arc::clone(&captured);
            server
                .mock(
                    "POST",
                    Matcher::Regex(r"^/models/[^:/]+:generateContent$".to_string()),
                )
                .with_status(200)
                .with_body_from_request(move |req| {
                    *slot.lock().unwrap() =
                        Some(req.utf8_lossy_body().unwrap_or_default().into_owned());
                    ok_body()
                })
                .create_async()
                .await;
            let adapter = adapter_over(&server.url(), overrides);
            let result = adapter.generate(request).await;
            let body = captured.lock().unwrap().take();
            (body, result)
        }

        /// D-09: `check_reasoning` answers with this adapter's own `llm.gemini.reasoning_*`
        /// defaults, and agrees with what `generate` does for the same effort.
        #[tokio::test]
        async fn gemini_check_reasoning_consults_the_provider_defaults() {
            const PROBE: &str = "probe-model";
            let mut server = Server::new_async().await;
            server
                .mock(
                    "POST",
                    Matcher::Regex(r"^/models/[^:/]+:generateContent$".to_string()),
                )
                .with_status(200)
                .with_body(ok_body())
                .expect_at_least(0)
                .create_async()
                .await;
            let over = ModelOverride {
                effort: Some(EffortSupport::Supported(EffortLevels::new([Low, High]))),
                ..ModelOverride::default()
            };
            let adapter = |effort: Option<ReasoningEffort>, fallback: Option<ReasoningFallback>| {
                let mut config = test_config(&server.url())
                    .with_model_overrides(BTreeMap::from([(PROBE.to_string(), over.clone())]));
                if let Some(effort) = effort {
                    config = config.with_reasoning_effort(effort);
                }
                if let Some(fallback) = fallback {
                    config = config.with_reasoning_fallback(fallback);
                }
                GeminiAdapter::new(config).expect("a valid override builds")
            };

            // A provider fallback substitutes an agent effort the model lacks.
            let substituting = adapter(None, Some(ReasoningFallback::NearestLower));
            assert!(
                substituting
                    .check_reasoning(PROBE, Some(Medium), None)
                    .is_ok()
            );
            assert!(
                substituting
                    .generate(request_for(PROBE).with_reasoning(Medium))
                    .await
                    .is_ok()
            );

            // A provider default effort the model lacks is refused for a caller who sets none.
            let defaulted = adapter(Some(Medium), None);
            let LlmError::UnsupportedParameter {
                reason: checked, ..
            } = defaulted.check_reasoning(PROBE, None, None).unwrap_err()
            else {
                panic!("expected UnsupportedParameter");
            };
            let LlmError::UnsupportedParameter {
                reason: sent,
                param,
                ..
            } = defaulted.generate(request_for(PROBE)).await.unwrap_err()
            else {
                panic!("expected UnsupportedParameter");
            };
            assert_eq!(param, "reasoning_effort");
            assert_eq!(checked, sent);
            assert!(
                checked.ends_with("(set by `llm.gemini.reasoning_effort`)"),
                "{checked}"
            );

            // A listed request effort is served under either default.
            for port in [&substituting, &defaulted] {
                assert!(port.check_reasoning(PROBE, Some(High), None).is_ok());
                assert!(
                    port.generate(request_for(PROBE).with_reasoning(High))
                        .await
                        .is_ok()
                );
            }

            // Nothing in force anywhere: nothing judged.
            assert!(
                adapter(None, None)
                    .check_reasoning(PROBE, None, None)
                    .is_ok()
            );
        }

        /// Run a streaming call whose body is `sse`; returns the raw request body and the outcome.
        async fn stream_raw(
            overrides: Vec<(&str, EffortSupport)>,
            request: LlmRequest,
            sse: &'static str,
        ) -> (
            Option<String>,
            Result<Vec<Result<StreamingResponse, LlmError>>, LlmError>,
        ) {
            let mut server = Server::new_async().await;
            let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
            let slot = Arc::clone(&captured);
            server
                .mock(
                    "POST",
                    Matcher::Regex(r"^/models/[^:/]+:streamGenerateContent".to_string()),
                )
                .match_query(Matcher::Any)
                .with_status(200)
                .with_header("content-type", "text/event-stream")
                .with_body_from_request(move |req| {
                    *slot.lock().unwrap() =
                        Some(req.utf8_lossy_body().unwrap_or_default().into_owned());
                    sse.as_bytes().to_vec()
                })
                .create_async()
                .await;
            let adapter = adapter_over(&server.url(), overrides);
            let outcome = match adapter.generate_stream(request).await {
                Err(error) => Err(error),
                Ok(stream) => {
                    let mut stream = Box::into_pin(stream);
                    let mut items = Vec::new();
                    while let Some(item) = stream.next().await {
                        items.push(item);
                    }
                    Ok(items)
                }
            };
            let body = captured.lock().unwrap().take();
            (body, outcome)
        }

        fn json_of(body: Option<String>) -> Value {
            serde_json::from_str(&body.expect("the request reached the wire"))
                .expect("the body must be JSON")
        }

        async fn refused_before_http(
            overrides: Vec<(&str, EffortSupport)>,
            request: LlmRequest,
        ) -> String {
            let (body, result) = send_raw(overrides, request).await;
            assert!(
                body.is_none(),
                "no request may reach the endpoint: {body:?}"
            );
            match result {
                Err(LlmError::UnsupportedParameter { reason, .. }) => reason,
                other => panic!("expected UnsupportedParameter, got {other:?}"),
            }
        }

        #[tokio::test]
        async fn a_level_sends_thinking_level_and_no_budget() {
            let (body, result) = send_raw(
                vec![(LEVELS_MODEL, levels_shape())],
                request_for(LEVELS_MODEL).with_reasoning(Medium),
            )
            .await;
            let metadata = result.expect("a listed level is accepted").metadata;
            let body = json_of(body);
            assert_eq!(
                body["generationConfig"]["thinkingConfig"],
                json!({"thinkingLevel": "medium"})
            );
            assert_eq!(metadata[REASONING_REQUESTED_METADATA_KEY], "medium");
            assert_eq!(metadata[REASONING_EFFECTIVE_METADATA_KEY], "medium");
        }

        #[tokio::test]
        async fn an_effort_alone_still_produces_a_generation_config_object() {
            // No max_tokens, no sampling: the effort is the only reason `generationConfig` exists.
            let mut request = request_for(LEVELS_MODEL).with_reasoning(High);
            request.prompt.entity.data.parameters = PromptParameters {
                max_tokens: None,
                temperature: None,
                top_p: None,
                frequency_penalty: None,
                presence_penalty: None,
                stop_sequences: None,
            };
            let (body, result) = send_raw(vec![(LEVELS_MODEL, levels_shape())], request).await;
            result.expect("accepted");
            assert_eq!(
                json_of(body)["generationConfig"],
                json!({"thinkingConfig": {"thinkingLevel": "high"}})
            );
        }

        #[tokio::test]
        async fn the_effort_body_adds_only_the_thinking_config() {
            let (unset, _) = send_raw(
                vec![(LEVELS_MODEL, levels_shape())],
                with_max_tokens(request_for(LEVELS_MODEL), 2048),
            )
            .await;
            let (set, _) = send_raw(
                vec![(LEVELS_MODEL, levels_shape())],
                with_max_tokens(request_for(LEVELS_MODEL).with_reasoning(Low), 2048),
            )
            .await;
            let unset = unset.expect("reached the wire");
            let prefix = unset
                .strip_suffix("}}")
                .expect("generationConfig closes the object");
            assert_eq!(
                set.expect("reached the wire"),
                format!("{prefix},\"thinkingConfig\":{{\"thinkingLevel\":\"low\"}}}}}}")
            );
        }

        #[tokio::test]
        async fn auto_sends_no_thinking_config_but_is_still_recorded() {
            let (body, result) = send_raw(
                vec![(LEVELS_MODEL, levels_shape())],
                with_max_tokens(
                    request_for(LEVELS_MODEL).with_reasoning(ReasoningEffort::Auto),
                    2048,
                ),
            )
            .await;
            let metadata = result.expect("auto is accepted").metadata;
            let body = json_of(body);
            assert!(
                body["generationConfig"].get("thinkingConfig").is_none(),
                "{body}"
            );
            assert_eq!(metadata[REASONING_REQUESTED_METADATA_KEY], "auto");
            assert_eq!(metadata[REASONING_EFFECTIVE_METADATA_KEY], "auto");
        }

        #[tokio::test]
        async fn off_on_a_levels_only_profile_is_refused_before_http() {
            refused_before_http(
                vec![(LEVELS_MODEL, levels_shape())],
                request_for(LEVELS_MODEL).with_reasoning(ReasoningEffort::Off),
            )
            .await;
        }

        #[tokio::test]
        async fn a_budget_inside_the_bounds_sends_thinking_budget_and_no_level() {
            let (body, result) = send_raw(
                vec![(BUDGET_MODEL, budget_shape())],
                with_max_tokens(
                    request_for(BUDGET_MODEL).with_reasoning(ReasoningEffort::Budget(1024)),
                    2048,
                ),
            )
            .await;
            result.expect("a budget inside the bounds is accepted");
            let body = json_of(body);
            assert_eq!(
                body["generationConfig"]["thinkingConfig"],
                json!({"thinkingBudget": 1024})
            );
            assert_eq!(body["generationConfig"]["maxOutputTokens"], 2048);
        }

        #[tokio::test]
        async fn on_wire_off_sends_a_zero_budget() {
            let (body, result) = send_raw(
                vec![(BUDGET_MODEL, budget_shape())],
                request_for(BUDGET_MODEL).with_reasoning(ReasoningEffort::Off),
            )
            .await;
            result.expect("off is accepted");
            assert_eq!(
                json_of(body)["generationConfig"]["thinkingConfig"],
                json!({"thinkingBudget": 0})
            );
        }

        #[tokio::test]
        async fn a_budget_that_leaves_no_output_room_is_refused_and_never_raises_the_cap() {
            let reason = refused_before_http(
                vec![(BUDGET_MODEL, budget_shape())],
                with_max_tokens(
                    request_for(BUDGET_MODEL).with_reasoning(ReasoningEffort::Budget(2048)),
                    2048,
                ),
            )
            .await;
            assert!(reason.contains("max_tokens"), "{reason}");
        }

        #[tokio::test]
        async fn a_budget_outside_the_bounds_is_refused_before_http() {
            for budget in [0, 24_577] {
                refused_before_http(
                    vec![(BUDGET_MODEL, budget_shape())],
                    with_max_tokens(
                        request_for(BUDGET_MODEL).with_reasoning(ReasoningEffort::Budget(budget)),
                        32_000,
                    ),
                )
                .await;
            }
        }

        #[tokio::test]
        async fn a_budget_on_a_levels_only_profile_is_refused_and_a_level_on_a_budget_only_one() {
            refused_before_http(
                vec![(LEVELS_MODEL, levels_shape())],
                with_max_tokens(
                    request_for(LEVELS_MODEL).with_reasoning(ReasoningEffort::Budget(1024)),
                    2048,
                ),
            )
            .await;
            // No conversion between the two forms (REASON-06): a level is not turned into a budget.
            refused_before_http(
                vec![(BUDGET_MODEL, budget_shape())],
                request_for(BUDGET_MODEL).with_reasoning(High),
            )
            .await;
        }

        #[tokio::test]
        async fn an_unmeasured_model_is_refused_naming_the_override() {
            let (body, result) = send_raw(
                Vec::new(),
                request_for(UNMEASURED_MODEL).with_reasoning(High),
            )
            .await;
            assert!(body.is_none(), "{body:?}");
            let error = result.expect_err("an unmeasured model refuses an explicit effort");
            assert_eq!(error.transience(), Transience::Permanent);
            match error {
                LlmError::UnsupportedParameter {
                    reason,
                    provider,
                    model,
                    ..
                } => {
                    assert_eq!(provider, "gemini");
                    assert_eq!(model, UNMEASURED_MODEL);
                    assert!(
                        reason.contains("llm.gemini.models.gemini-unprofiled-test-model.effort"),
                        "{reason}"
                    );
                }
                other => panic!("expected UnsupportedParameter, got {other:?}"),
            }
        }

        #[tokio::test]
        async fn a_substituting_fallback_sends_the_nearest_lower_level_and_records_both() {
            let two = EffortSupport::Supported(EffortLevels::new([Low, High]));
            let (body, result) = send_raw(
                vec![(LEVELS_MODEL, two)],
                request_for(LEVELS_MODEL)
                    .with_reasoning(Medium)
                    .with_reasoning_fallback(ReasoningFallback::NearestLower),
            )
            .await;
            let metadata = result.expect("a substitution is accepted").metadata;
            assert_eq!(
                json_of(body)["generationConfig"]["thinkingConfig"],
                json!({"thinkingLevel": "low"})
            );
            assert_eq!(metadata[REASONING_REQUESTED_METADATA_KEY], "medium");
            assert_eq!(metadata[REASONING_EFFECTIVE_METADATA_KEY], "low");
        }

        #[tokio::test]
        async fn an_unset_effort_sends_no_thinking_config_and_stamps_nothing() {
            let (body, result) = send_raw(
                vec![(LEVELS_MODEL, levels_shape())],
                request_for(LEVELS_MODEL),
            )
            .await;
            let metadata = result.expect("accepted").metadata;
            assert!(!metadata.contains_key(REASONING_REQUESTED_METADATA_KEY));
            assert!(!metadata.contains_key(REASONING_EFFECTIVE_METADATA_KEY));
            assert!(!body.expect("reached the wire").contains("thinkingConfig"));
        }

        #[tokio::test]
        async fn no_body_asks_for_thought_text() {
            // D-00g: `includeThoughts` is never sent; the Phase 50 typed fields are the surface.
            for (model, shape, effort) in [
                (LEVELS_MODEL, levels_shape(), High),
                (BUDGET_MODEL, budget_shape(), ReasoningEffort::Off),
                (BUDGET_MODEL, budget_shape(), ReasoningEffort::Budget(512)),
            ] {
                let (body, result) = send_raw(
                    vec![(model, shape)],
                    with_max_tokens(request_for(model).with_reasoning(effort), 2048),
                )
                .await;
                result.expect("accepted");
                assert!(
                    !body.expect("reached the wire").contains("includeThoughts"),
                    "{effort}"
                );
            }
        }

        const STREAM_BODY: &str = concat!(
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"Hel\"}]}}]}\n\n",
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"lo\"}],\"role\":\"model\"},",
            "\"finishReason\":\"STOP\"}],\"usageMetadata\":{\"promptTokenCount\":10,",
            "\"candidatesTokenCount\":8,\"thoughtsTokenCount\":41,\"totalTokenCount\":59}}\n\n",
        );

        #[tokio::test]
        async fn the_stream_sends_the_same_thinking_config_and_stamps_only_the_terminal_chunk() {
            let (body, outcome) = stream_raw(
                vec![(LEVELS_MODEL, levels_shape())],
                request_for(LEVELS_MODEL).with_reasoning(High),
                STREAM_BODY,
            )
            .await;
            assert_eq!(
                json_of(body)["generationConfig"]["thinkingConfig"],
                json!({"thinkingLevel": "high"})
            );
            let items = outcome.expect("the stream opens");
            let chunks: Vec<&StreamingResponse> = items
                .iter()
                .map(|i| i.as_ref().expect("every item is Ok"))
                .collect();
            let (terminal, earlier) = chunks.split_last().expect("a terminal chunk");
            assert_eq!(terminal.metadata[REASONING_REQUESTED_METADATA_KEY], "high");
            assert_eq!(terminal.metadata[REASONING_EFFECTIVE_METADATA_KEY], "high");
            for chunk in earlier {
                assert!(chunk.metadata.is_empty(), "earlier chunk: {chunk:?}");
            }
            // REASON-05: thoughts tokens still reach the typed usage field.
            let usage = terminal
                .usage
                .as_ref()
                .expect("usage rides the terminal chunk");
            assert_eq!(usage.reasoning_tokens, Some(41));
        }

        #[tokio::test]
        async fn the_stream_sends_a_budget_and_a_zero_budget_like_the_buffered_path() {
            let (budget, _) = stream_raw(
                vec![(BUDGET_MODEL, budget_shape())],
                with_max_tokens(
                    request_for(BUDGET_MODEL).with_reasoning(ReasoningEffort::Budget(1024)),
                    2048,
                ),
                STREAM_BODY,
            )
            .await;
            assert_eq!(
                json_of(budget)["generationConfig"]["thinkingConfig"],
                json!({"thinkingBudget": 1024})
            );
            let (off, _) = stream_raw(
                vec![(BUDGET_MODEL, budget_shape())],
                request_for(BUDGET_MODEL).with_reasoning(ReasoningEffort::Off),
                STREAM_BODY,
            )
            .await;
            assert_eq!(
                json_of(off)["generationConfig"]["thinkingConfig"],
                json!({"thinkingBudget": 0})
            );
        }

        #[tokio::test]
        async fn a_refused_effort_fails_the_stream_before_any_http_call() {
            let (body, outcome) = stream_raw(
                vec![(LEVELS_MODEL, levels_shape())],
                request_for(LEVELS_MODEL).with_reasoning(ReasoningEffort::Off),
                STREAM_BODY,
            )
            .await;
            assert!(body.is_none(), "{body:?}");
            assert!(matches!(
                outcome,
                Err(LlmError::UnsupportedParameter { .. })
            ));
        }

        /// The transcript date every Gemini model was probed (run 9).
        const PROBED_ON: &str = "2026-10-22";

        /// `(model, minimal accepted, off, auto)` as `51-EFFORT-PROBE.md` evidences them (run 9):
        /// `low`, `medium` and `high` were accepted on all twelve; `minimal` was rejected (400)
        /// on six and accepted (no count) on six; `off` = `thinkingBudget` 0 accepted with no
        /// reasoning-token count; `auto` = the default call reported more than 0 reasoning
        /// tokens.
        fn transcript() -> [(&'static str, bool, bool, bool); 12] {
            [
                ("gemini-4-argon", false, true, true),
                ("gemini-3.8-flash", false, false, true),
                ("gemini-3.7-flash", false, false, true),
                ("gemini-3.1-pro-preview", false, false, true),
                ("gemini-pro-latest", false, false, true),
                ("gemini-flash-latest", false, false, true),
                ("gemini-flash-lite-latest", true, false, false),
                ("gemini-3.1-flash-lite", true, true, false),
                ("gemini-3-flash-preview", true, true, true),
                ("gemini-3.5-flash-lite", true, false, false),
                ("gemini-3.6-flash", true, true, true),
                ("gemini-3.5-flash", true, true, true),
            ]
        }

        fn expected(minimal: bool, off: bool, auto: bool) -> EffortSupport {
            let levels: Vec<ReasoningEffort> = if minimal {
                vec![Minimal, Low, Medium, High]
            } else {
                vec![Low, Medium, High]
            };
            EffortSupport::Supported(EffortLevels::new(levels).with_off(off).with_auto(auto))
        }

        #[test]
        fn gemini_effort_seed_matches_the_transcript() {
            let adapter = adapter_over("https://example.invalid/v1", Vec::new());
            for (model, minimal, off, auto) in transcript() {
                let profile = adapter
                    .model_profile(model)
                    .unwrap_or_else(|| panic!("{model}: no profile"));
                assert_eq!(profile.effort, expected(minimal, off, auto), "{model}");
                assert_eq!(
                    profile.effort_verified_on.as_deref(),
                    Some(PROBED_ON),
                    "{model}"
                );
                let EffortSupport::Supported(levels) = profile.effort else {
                    panic!("{model}: Supported");
                };
                // Budgets 128 and 1 were accepted and -5 rejected, but no vendor-documented
                // bound is in the probe file, so no span is seeded (judgement call, SUMMARY).
                assert_eq!(levels.budget, None, "{model}");
            }
        }

        /// Research A2: the lowercase spelling is the one pinned. The transcript accepted
        /// lowercase everywhere (12 of 12) and uppercase `HIGH` too; lowercase is kept.
        #[tokio::test]
        async fn thinking_level_is_sent_in_lowercase_on_every_seeded_level() {
            for (model, minimal, ..) in transcript() {
                let mut levels = vec![Low, Medium, High];
                if minimal {
                    levels.insert(0, ReasoningEffort::Minimal);
                }
                for level in levels {
                    let (body, result) =
                        send_raw(Vec::new(), request_for(model).with_reasoning(level)).await;
                    result.unwrap_or_else(|e| panic!("{model}/{level}: {e}"));
                    let body: Value =
                        serde_json::from_str(&body.expect("reached the wire")).unwrap();
                    assert_eq!(
                        body["generationConfig"]["thinkingConfig"],
                        serde_json::json!({"thinkingLevel": level.to_string()}),
                        "{model}/{level}"
                    );
                }
            }
        }

        #[tokio::test]
        async fn off_and_auto_follow_the_seeded_flags() {
            for (model, _, off, auto) in transcript() {
                let (body, result) = send_raw(
                    Vec::new(),
                    request_for(model).with_reasoning(ReasoningEffort::Off),
                )
                .await;
                if off {
                    result.unwrap_or_else(|e| panic!("{model}: {e}"));
                    let body: Value = serde_json::from_str(&body.unwrap()).unwrap();
                    assert_eq!(
                        body["generationConfig"]["thinkingConfig"],
                        serde_json::json!({"thinkingBudget": 0}),
                        "{model}"
                    );
                } else {
                    assert!(body.is_none(), "{model}: off reached the wire");
                    assert!(
                        matches!(result, Err(LlmError::UnsupportedParameter { .. })),
                        "{model}: {result:?}"
                    );
                }
                let (body, result) = send_raw(
                    Vec::new(),
                    request_for(model).with_reasoning(ReasoningEffort::Auto),
                )
                .await;
                if auto {
                    result.unwrap_or_else(|e| panic!("{model}: {e}"));
                    let body: Value = serde_json::from_str(&body.unwrap()).unwrap();
                    assert!(
                        body["generationConfig"].get("thinkingConfig").is_none(),
                        "{model}: {body}"
                    );
                } else {
                    assert!(body.is_none(), "{model}: auto reached the wire");
                    assert!(
                        matches!(result, Err(LlmError::UnsupportedParameter { .. })),
                        "{model}: {result:?}"
                    );
                }
            }
        }

        #[tokio::test]
        async fn a_level_the_probe_rejected_and_every_budget_refuse_before_http() {
            for (model, minimal, ..) in transcript() {
                let mut refused = vec![
                    ReasoningEffort::XHigh,
                    ReasoningEffort::Max,
                    ReasoningEffort::Budget(128),
                ];
                if !minimal {
                    refused.push(ReasoningEffort::Minimal);
                }
                for effort in refused {
                    let (body, result) =
                        send_raw(Vec::new(), request_for(model).with_reasoning(effort)).await;
                    assert!(body.is_none(), "{model}/{effort}: reached the wire");
                    assert!(
                        matches!(result, Err(LlmError::UnsupportedParameter { .. })),
                        "{model}/{effort}: {result:?}"
                    );
                }
            }
        }

        #[test]
        fn the_reported_profile_keeps_levels_auto_off_and_budget() {
            let adapter = adapter_over(
                "http://localhost:1",
                vec![
                    (BUDGET_MODEL, budget_shape()),
                    (LEVELS_MODEL, levels_shape()),
                ],
            );
            let EffortSupport::Supported(budget) =
                adapter.model_profile(BUDGET_MODEL).unwrap().effort
            else {
                panic!("a wire that carries every form keeps the override Supported");
            };
            assert!(budget.off && budget.auto);
            assert_eq!(budget.budget, Some(BudgetBounds::new(1, 24_576)));
            let EffortSupport::Supported(levels) =
                adapter.model_profile(LEVELS_MODEL).unwrap().effort
            else {
                panic!("Supported");
            };
            assert_eq!(levels.levels, [Low, Medium, High]);
            assert_eq!(
                adapter.model_profile(UNMEASURED_MODEL).unwrap().effort,
                EffortSupport::Unmeasured
            );
        }
    }

    // ── Shared conformance suite (RT-06, D-31) ──
    //
    // Nested in its own module (rather than inline in `mod tests`) so every generated test's
    // full path contains "conformance" -- `cargo test --lib conformance` (the plan's own
    // acceptance criterion) selects it by that substring. Bodies below are copied verbatim from
    // this module's own hand-written tests above
    // (`generate_posts_to_generate_content_with_x_goog_api_key_header`,
    // `generate_stream_assembles_three_frames_in_wire_order`) rather than invented for this
    // suite (26-PATTERNS.md).
    mod conformance_suite {
        use super::*;

        struct GeminiFixture;

        impl crate::conformance::ConformanceFixture for GeminiFixture {
            /// Gemini's `generationConfig.thinkingConfig.thinkingLevel`, lower-cased.
            fn effort_wire_value(body: &serde_json::Value) -> Option<String> {
                body.get("generationConfig")
                    .and_then(|config| config.get("thinkingConfig"))
                    .and_then(|config| config.get("thinkingLevel"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_lowercase)
            }

            const WIRE: crate::conformance::Wire = crate::conformance::Wire::Gemini;

            fn default_model() -> &'static str {
                GEMINI_DEFAULT_MODEL
            }

            fn provider_name() -> &'static str {
                "gemini"
            }

            fn api_key() -> &'static str {
                "test-key-abc123"
            }

            fn adapter(base_url: &str) -> Arc<dyn LlmPort> {
                Arc::new(test_adapter(base_url))
            }

            fn success_body() -> String {
                json!({
                    "candidates": [{
                        "content": {"role": "model", "parts": [{"text": "Hi there"}]},
                        "finishReason": "STOP"
                    }],
                    "usageMetadata": {
                        "promptTokenCount": 5,
                        "candidatesTokenCount": 3,
                        "totalTokenCount": 8
                    }
                })
                .to_string()
            }

            fn stream_body() -> String {
                // D-19: the terminal frame carries the SAME usage figures as
                // `success_body()` above (promptTokenCount 5, candidates 3,
                // total 8) -- the shared parity case asserts equality
                // between the two.
                concat!(
                    "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"Hel\"}]}}]}\n\n",
                    "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"lo \"}]}}]}\n\n",
                    "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"world\"}],\"role\":\"model\"},",
                    "\"finishReason\":\"STOP\"}],\"usageMetadata\":{\"promptTokenCount\":5,",
                    "\"candidatesTokenCount\":3,\"totalTokenCount\":8}}\n\n",
                )
                .to_string()
            }

            fn error_body(status: u16) -> String {
                json!({
                    "error": {
                        "code": status,
                        "message": format!("mock error for status {status}"),
                        "status": "UNKNOWN"
                    }
                })
                .to_string()
            }
        }

        crate::llm_conformance_suite!(GeminiFixture);
    }
}

/// The PARITY-03 knobs on `GeminiConfig` (plan 50-08, D-12): each one is read by the adapter.
#[cfg(test)]
mod knob_tests {
    use super::*;
    use crate::test_support::{
        attempts_against_a_failing_endpoint_at, mock_capture, send_with_max_tokens, server,
    };
    use crate::window::effective_capabilities;
    use serde_json::json;

    const UNSEEDED: &str = "gemini-unseeded-test-model";

    fn path_for(model: &str) -> String {
        format!("/models/{model}:generateContent")
    }

    fn config(base_url: &str, model: &str) -> GeminiConfig {
        GeminiConfig::new("test-key".to_string(), model.to_string()).with_base_url(base_url)
    }

    fn window_override(window: u32) -> ModelOverride {
        ModelOverride {
            context_window: Some(window),
            ..ModelOverride::default()
        }
    }

    fn response() -> serde_json::Value {
        json!({
            "candidates": [{
                "content": {"role": "model", "parts": [{"text": "ok"}]},
                "finishReason": "STOP"
            }],
            "usageMetadata": {
                "promptTokenCount": 1,
                "candidatesTokenCount": 1,
                "totalTokenCount": 2
            }
        })
    }

    fn error_of(config: GeminiConfig) -> String {
        match GeminiAdapter::new(config) {
            Err(error) => error.to_string(),
            Ok(_) => panic!("construction must fail"),
        }
    }

    fn sent(body: &serde_json::Value) -> Option<u64> {
        body["generationConfig"]
            .get("maxOutputTokens")
            .and_then(|v| v.as_u64())
    }

    #[tokio::test]
    async fn gemini_max_tokens_default_reaches_the_wire() {
        let mut server = server().await;
        let captured = mock_capture(&mut server, &path_for(UNSEEDED), response()).await;
        let url = server.url();

        let adapter = GeminiAdapter::new(config(&url, UNSEEDED).with_max_tokens(2048)).unwrap();
        let body = send_with_max_tokens(&adapter, &captured, UNSEEDED, None)
            .await
            .unwrap();
        assert_eq!(sent(&body), Some(2048), "{body}");
        // A request's own value wins.
        let body = send_with_max_tokens(&adapter, &captured, UNSEEDED, Some(512))
            .await
            .unwrap();
        assert_eq!(sent(&body), Some(512), "{body}");

        // No configured default: no `maxOutputTokens`, as before PARITY-03.
        let plain = GeminiAdapter::new(config(&url, UNSEEDED)).unwrap();
        let body = send_with_max_tokens(&plain, &captured, UNSEEDED, None)
            .await
            .unwrap();
        assert_eq!(sent(&body), None, "{body}");
    }

    #[tokio::test]
    async fn gemini_max_tokens_default_is_checked_against_the_profile() {
        let mut server = server().await;
        let model = "gemini-3.5-flash-lite";
        let captured = mock_capture(&mut server, &path_for(model), response()).await;
        // The model is seeded with a 65,536-token maximum output.
        let adapter =
            GeminiAdapter::new(config(&server.url(), model).with_max_tokens(100_000)).unwrap();
        let Err(LlmError::InvalidPrompt(message)) =
            send_with_max_tokens(&adapter, &captured, model, None).await
        else {
            panic!("a default above the model's maximum output must be InvalidPrompt");
        };
        for needle in ["max_tokens", "100000", "65536"] {
            assert!(
                message.contains(needle),
                "'{needle}' missing from: {message}"
            );
        }
        assert!(captured.lock().unwrap().is_none(), "nothing was sent");
    }

    #[tokio::test]
    async fn gemini_max_retries_is_read_by_the_adapter() {
        // This adapter's loop counts total attempts (its long-standing convention): the budget
        // `n` sends up to `n` requests on a retryable 5xx.
        for (budget, attempts) in [(1_u32, 1_usize), (2, 2)] {
            let server = server().await;
            let adapter =
                GeminiAdapter::new(config(&server.url(), UNSEEDED).with_max_retries(budget))
                    .unwrap();
            attempts_against_a_failing_endpoint_at(
                server,
                &path_for(UNSEEDED),
                &adapter,
                UNSEEDED,
                attempts,
            )
            .await;
        }
    }

    #[test]
    fn gemini_context_window_override_is_declared() {
        let adapter = GeminiAdapter::new(
            config("https://example.invalid/v1", UNSEEDED).with_context_window(500_000),
        )
        .unwrap();
        assert_eq!(adapter.get_capabilities().max_context_tokens, Some(500_000));
        assert_eq!(
            adapter.model_profile(UNSEEDED).unwrap().context_window,
            Some(500_000)
        );
    }

    #[test]
    fn context_window_override_beats_a_seeded_window_on_gemini() {
        let model = "gemini-3.5-flash-lite";
        let base = || config("https://example.invalid/v1", model);
        let seeded = GeminiAdapter::new(base()).unwrap();
        assert_eq!(
            seeded.model_profile(model).unwrap().context_window,
            Some(1_048_576)
        );

        // The operator's adapter-wide statement beats the compiled seeded fact ...
        let adapter = GeminiAdapter::new(base().with_context_window(500_000)).unwrap();
        assert_eq!(
            adapter.model_profile(model).unwrap().context_window,
            Some(500_000)
        );
        assert_eq!(
            effective_capabilities(&adapter, model).max_context_tokens,
            Some(500_000)
        );

        // ... and a per-model override beats both.
        let adapter = GeminiAdapter::new(base().with_context_window(500_000).with_model_overrides(
            BTreeMap::from([(model.to_string(), window_override(900_000))]),
        ))
        .unwrap();
        assert_eq!(
            adapter.model_profile(model).unwrap().context_window,
            Some(900_000)
        );

        // A window below the seeded maximum output is refused, never clamped.
        let message = error_of(base().with_context_window(40_000));
        for needle in ["gemini", model, "40000", "65536"] {
            assert!(
                message.contains(needle),
                "'{needle}' missing from: {message}"
            );
        }
    }

    #[test]
    fn gemini_invalid_override_fails_construction_naming_provider_and_model() {
        let with_override = |over: ModelOverride| {
            config("https://example.invalid/v1", UNSEEDED)
                .with_model_overrides(BTreeMap::from([(UNSEEDED.to_string(), over)]))
        };
        let adapter = GeminiAdapter::new(with_override(window_override(900_000))).unwrap();
        assert_eq!(
            adapter.model_profile(UNSEEDED).unwrap().context_window,
            Some(900_000)
        );
        for over in [
            window_override(0),
            ModelOverride {
                context_window: Some(1_000),
                max_output_tokens: Some(2_000_000),
                ..ModelOverride::default()
            },
        ] {
            let message = error_of(with_override(over));
            assert!(
                message.contains("gemini") && message.contains(UNSEEDED),
                "the error must name the provider and the model: {message}"
            );
        }
    }

    #[test]
    fn gemini_config_defaults_a_reasoning_model_to_300s() {
        // Every seeded Gemini model is a reasoning model; an unknown one keeps 60 s.
        for (model, expected) in [
            ("gemini-3.5-flash-lite", 300),
            ("gemini-3.6-flash", 300),
            (GEMINI_DEFAULT_MODEL, 300),
            (UNSEEDED, 60),
        ] {
            assert_eq!(
                config("https://example.invalid/v1", model).timeout_seconds,
                expected,
                "new({model})"
            );
            let parts = GeminiConfig::from_parts(
                Some("k".to_string()),
                None,
                Some(model.to_string()),
                None,
            )
            .unwrap();
            assert_eq!(parts.timeout_seconds, expected, "from_parts({model})");
        }
        // An explicit environment value always wins.
        for explicit in ["30", "60", "900"] {
            let parts = GeminiConfig::from_parts(
                Some("k".to_string()),
                None,
                Some("gemini-3.5-flash-lite".to_string()),
                Some(explicit.to_string()),
            )
            .unwrap();
            assert_eq!(parts.timeout_seconds.to_string(), explicit);
        }
    }

    #[test]
    fn gemini_config_debug_redacts_the_api_key() {
        let config = config("https://api.example.invalid/v1", "gemini-3.5-flash-lite");
        let debug = format!("{config:?}");
        assert!(!debug.contains("test-key"), "{debug}");
        assert!(debug.contains("<redacted>"), "{debug}");
        for needle in [
            "https://api.example.invalid/v1",
            "timeout_seconds",
            "max_retries",
            "max_tokens",
        ] {
            assert!(debug.contains(needle), "'{needle}' missing from {debug}");
        }
    }
}
