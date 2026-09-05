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
//!   same shape [`GeminiResponse`] already parses for the non-streaming
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
//! refused redirect surfaces via [`GeminiAdapter::map_error`]'s
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
use std::collections::HashMap;
use std::env;
use std::time::Duration;
use tokio::sync::OnceCell;
use uuid::Uuid;

use paladin_core::platform::container::prompt::{PromptRole, PromptType};
use paladin_ports::output::llm_port::{
    FinishReason, LlmError, LlmPort, LlmRequest, LlmResponse, ProviderCapabilities,
    StreamingResponse, TokenUsage,
};

use crate::http_status::map_http_status;
use crate::redaction::{
    RESPONSE_EXCERPT_CHAR_BUDGET, bounded_excerpt, diagnostic_excerpt, redact_credentials,
};

/// The provider name this adapter reports through [`LlmPort::get_provider_name`]
/// and stamps on every [`LlmError::ProviderError`] it emits (Phase 25 D-03).
const GEMINI_PROVIDER: &str = "gemini";

/// Default Gemini API base URL — the `v1beta` surface, current as of this
/// writing `[CITED: ai.google.dev/api]`.
pub const GEMINI_DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

/// Default Gemini model requested when `GEMINI_MODEL` is unset.
///
/// **Refreshed 2026-08-22** from `gemini-2.5-flash`, which Google retired for
/// new users: a `generateContent` call against it returns *"no longer
/// available to new users \u2014 please update your code to use
/// models/gemini-3.6-flash"*. The model remained present in the live
/// `GET /models` catalog throughout, so the list probe kept passing while
/// every generate call failed \u2014 catalog presence is not callability, and
/// only the generate probe distinguishes them.
///
/// `gemini-3.6-flash` is the identifier Google's own deprecation message
/// names, and it was verified by a live `generateContent` call on
/// 2026-08-22 rather than taken from that message on faith.
pub const GEMINI_DEFAULT_MODEL: &str = "gemini-3.6-flash";

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
pub const GEMINI_FALLBACK_MODELS: &[&str] = &["gemini-3.6-flash", "gemini-3.5-flash"];

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
#[derive(Debug, Clone)]
pub struct GeminiConfig {
    /// API key for Gemini authentication, sent as the `x-goog-api-key`
    /// header on every request.
    pub api_key: String,
    /// Base URL for the Gemini API.
    pub base_url: String,
    /// Default model to use (e.g. `gemini-2.5-flash`).
    pub model: String,
    /// Request timeout in seconds.
    pub timeout_seconds: u64,
}

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
    ///   `60`.
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
        let timeout_seconds = timeout_seconds
            .unwrap_or_else(|| "60".to_string())
            .parse()
            .map_err(|_| "Invalid GEMINI_TIMEOUT_SECONDS value".to_string())?;

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
        Ok(())
    }
}

/// Google Gemini LLM Adapter implementing [`LlmPort`] directly against
/// Gemini's own `generateContent` protocol (D-08).
pub struct GeminiAdapter {
    client: Client,
    config: GeminiConfig,
    models_cache: OnceCell<Vec<String>>,
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

        Ok(Self {
            client,
            config,
            models_cache: OnceCell::new(),
        })
    }

    /// Build the outgoing request body from a port-level [`LlmRequest`].
    ///
    /// `PromptType::System` (and `PromptType::Text` with
    /// [`PromptRole::System`]) map to the top-level `systemInstruction`
    /// field, never to a `contents[]` entry — Gemini's API has no `system`
    /// role inside `contents[]`. `PromptType::Function` (and
    /// `PromptType::Text` with [`PromptRole::Function`]) have no supported
    /// mapping — Gemini's function surface is out of scope for this adapter
    /// (D-08) — and return [`LlmError::InvalidPrompt`] naming the
    /// unsupported prompt type rather than silently dropping it.
    fn build_request(&self, request: &LlmRequest) -> Result<GeminiRequest, LlmError> {
        let mut system_instruction = None;
        let mut contents = Vec::new();

        match &request.prompt.node.node.prompt_type {
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
                        text: user_prompt.query.clone(),
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

        let params = &request.prompt.node.node.parameters;
        let generation_config = GeminiGenerationConfig {
            temperature: params.temperature,
            max_output_tokens: params.max_tokens,
            top_p: params.top_p,
            top_k: None,
            stop_sequences: params.stop_sequences.clone(),
            candidate_count: None,
        };
        let generation_config = if generation_config.temperature.is_some()
            || generation_config.max_output_tokens.is_some()
            || generation_config.top_p.is_some()
            || generation_config.stop_sequences.is_some()
        {
            Some(generation_config)
        } else {
            None
        };

        Ok(GeminiRequest {
            contents,
            system_instruction,
            generation_config,
        })
    }

    /// Parse a well-formed Gemini `generateContent` response into an
    /// [`LlmResponse`].
    ///
    /// Fails with [`LlmError::EmptyCompletion`] under two named conditions
    /// rather than ever returning `Ok` with empty content — an empty-string
    /// success is indistinguishable from a valid empty answer to every
    /// downstream caller:
    ///
    /// 1. `candidates` is empty.
    /// 2. The mapped finish reason is [`FinishReason::Length`] (Gemini's
    ///    `MAX_TOKENS`) and the extracted content trims to empty — the
    ///    reasoning-model truncation signature also detected by
    ///    [`crate::compat::engine::CompatEngine::detect_empty_completion`]
    ///    for every compat-preset adapter in this crate.
    fn parse_response(
        &self,
        request_id: Uuid,
        model: &str,
        response: GeminiResponse,
    ) -> Result<LlmResponse, LlmError> {
        let candidate = response.candidates.first().ok_or_else(|| {
            LlmError::EmptyCompletion("Gemini response contained no candidates".to_string())
        })?;

        let content = candidate_text(candidate);
        let finish_reason = map_finish_reason(candidate.finish_reason.as_deref());

        // Mirrors `crate::compat::engine::CompatEngine::detect_empty_completion` —
        // the two must stay in step by hand (D-08 keeps Gemini bespoke, so
        // this parity is not structural; a divergence between the two is a
        // defect). The predicate is deliberately two conditions, not one: an
        // empty response under a normal `STOP` finish is a legal empty
        // answer, the same case every compat preset also lets through. The
        // finish reason is the discriminator specifically so a `SAFETY` or
        // `RECITATION` refusal is never misreported as a token-budget
        // problem — the model declined to answer, and a larger max_tokens
        // will produce the same refusal.
        if matches!(finish_reason, FinishReason::Length) && content.trim().is_empty() {
            return Err(LlmError::EmptyCompletion(format!(
                "Gemini response finished with MAX_TOKENS and produced no text ({} raw chars) — \
                 reasoning likely consumed the entire max_tokens budget; retry with a larger \
                 max_tokens",
                content.len()
            )));
        }

        let usage = response.usage_metadata.unwrap_or_default();

        Ok(LlmResponse {
            id: Uuid::new_v4(),
            request_id,
            model: model.to_string(),
            content,
            finish_reason,
            usage: TokenUsage {
                prompt_tokens: usage.prompt_token_count,
                completion_tokens: usage.candidates_token_count,
                total_tokens: usage.total_token_count,
            },
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: None,
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
    fn map_error(&self, status: u16, body: &str) -> LlmError {
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
            429 => LlmError::RateLimitExceeded,
            _ if rpc_status == Some("RESOURCE_EXHAUSTED") => LlmError::RateLimitExceeded,
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
                map_http_status(
                    GEMINI_PROVIDER,
                    status,
                    &envelope_text,
                    &self.config.api_key,
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
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(self.map_error(status, &body));
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
        let gemini_request = self.build_request(&request)?;
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
                let body = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                return Err(self.map_error(status, &body));
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

        self.execute_with_retry(operation, 3).await
    }

    /// Generate a streaming completion. POSTs with `alt=sse` and assembles
    /// SSE frames via `parse_sse_chunk`.
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
    async fn generate_stream(
        &self,
        request: LlmRequest,
    ) -> Result<Box<dyn Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError> {
        validate_model_identifier(&request.model)?;
        let gemini_request = self.build_request(&request)?;
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
                let body = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                return Err(self.map_error(status, &body));
            }

            Ok(response)
        };

        let response = self.execute_with_retry(operation, 3).await?;

        let stream = response.bytes_stream().flat_map(|chunk_result| {
            let items: Vec<Result<StreamingResponse, LlmError>> = match chunk_result {
                Ok(bytes) => parse_sse_chunk(&bytes),
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
            max_context_tokens: Some(1_048_576),
            supports_system_messages: true,
            temperature_range: Some((0.0, 2.0)),
        }
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
    #[serde(default, rename = "totalTokenCount")]
    total_token_count: u32,
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
/// [`FinishReason::Length`], `SAFETY` to [`FinishReason::ContentFilter`]
/// (the closest existing variant — Gemini's safety block is a
/// content-policy decision, matching `ContentFilter`'s semantics). `OTHER`
/// and `RECITATION` — and any other value this adapter does not
/// specifically recognise — map to [`FinishReason::Error`] carrying the raw
/// reason string. This is deliberate: neither has a direct `FinishReason`
/// equivalent, and coercing either to `Stop` would report a truncated or
/// blocked generation as a normal completion. An absent `finishReason`
/// (`None`) maps to `Stop` — the non-terminal-streaming-frame case is
/// distinguished by callers never invoking this function with `None` for a
/// frame that has not yet finished.
fn map_finish_reason(reason: Option<&str>) -> FinishReason {
    match reason {
        None => FinishReason::Stop,
        Some("STOP") => FinishReason::Stop,
        Some("MAX_TOKENS") => FinishReason::Length,
        Some("SAFETY") => FinishReason::ContentFilter,
        Some(other) => FinishReason::Error(other.to_string()),
    }
}

/// Parse one network chunk of Gemini's SSE stream into zero or more
/// [`StreamingResponse`] items.
///
/// A single chunk can carry more than one complete `data: {...}` event —
/// common when a mock transport (or a provider whose TCP framing does not
/// align to event boundaries) writes the whole body at once — so every
/// `data:`-prefixed line found is emitted as its own item, mirroring
/// [`crate::compat::engine::CompatEngine::generate_stream`]'s `flat_map`
/// discipline. There is no `[DONE]` sentinel in Gemini's SSE framing; the
/// stream simply ends when the body ends. A frame with no `candidates` at
/// all (e.g. a metadata-only frame) yields no item, not an error.
fn parse_sse_chunk(bytes: &[u8]) -> Vec<Result<StreamingResponse, LlmError>> {
    let text = String::from_utf8_lossy(bytes);
    let mut items = Vec::new();

    for line in text.lines() {
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

                    items.push(Ok(StreamingResponse {
                        id: Uuid::new_v4(),
                        delta,
                        finish_reason,
                    }));
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

    fn test_config(base_url: &str) -> GeminiConfig {
        GeminiConfig::new(
            "test-key-abc123".to_string(),
            base_url.to_string(),
            "gemini-2.5-flash".to_string(),
        )
    }

    fn test_adapter(base_url: &str) -> GeminiAdapter {
        GeminiAdapter::new(test_config(base_url)).expect("test config must build a valid adapter")
    }

    fn build_request(model: &str, prompt_type: PromptType) -> LlmRequest {
        LlmRequest {
            id: Uuid::new_v4(),
            model: model.to_string(),
            prompt: PromptItem::new(prompt_type).unwrap(),
            attachments: vec![],
            stream: false,
            metadata: HashMap::new(),
        }
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
        assert_eq!(config.timeout_seconds, 60);
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

        let gemini_request = adapter.build_request(&request).unwrap();
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
        let user_gemini = adapter.build_request(&user_request).unwrap();
        assert_eq!(user_gemini.contents[0].role, "user");

        let assistant_request = build_request(
            "gemini-2.5-flash",
            PromptType::Assistant(paladin_core::platform::container::prompt::AssistantPrompt {
                response: "Hi".to_string(),
                reasoning: None,
            }),
        );
        let assistant_gemini = adapter.build_request(&assistant_request).unwrap();
        // Gemini's assistant-equivalent role is "model", never "assistant".
        assert_eq!(assistant_gemini.contents[0].role, "model");
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

        let gemini_request = adapter.build_request(&request).unwrap();
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
        match map_finish_reason(Some("RECITATION")) {
            FinishReason::Error(reason) => assert_eq!(reason, "RECITATION"),
            other => panic!("expected FinishReason::Error(\"RECITATION\"), got {other:?}"),
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
        assert!(matches!(result, Err(LlmError::EmptyCompletion(_))));
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
            matches!(result, Err(LlmError::EmptyCompletion(_))),
            "expected Err(EmptyCompletion(_)) for a MAX_TOKENS finish with no parts, got {result:?}"
        );
        if let Err(LlmError::EmptyCompletion(message)) = result {
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
            matches!(result, Err(LlmError::EmptyCompletion(_))),
            "expected Err(EmptyCompletion(_)) for a MAX_TOKENS finish with whitespace-only text, got {result:?}"
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

    /// Adjacency control: exact parity with
    /// `crate::compat::engine::CompatEngine::detect_empty_completion`, which
    /// also lets a `STOP`-finished empty response through as `Ok`. **Passes
    /// today.** Widening the guard to fire on empty content under any finish
    /// reason would make Gemini stricter than every other adapter in this
    /// crate — a different inconsistency, not a fix.
    #[test]
    fn parse_response_keeps_an_empty_response_that_finished_normally() {
        let adapter = test_adapter("https://example.invalid");
        let body = json!({
            "candidates": [{
                "content": {"role": "model", "parts": []},
                "finishReason": "STOP"
            }]
        });
        let response: GeminiResponse = serde_json::from_value(body).unwrap();

        let llm_response = adapter
            .parse_response(Uuid::new_v4(), "gemini-2.5-flash", response)
            .expect("an empty response that finished normally must still be Ok");

        assert!(llm_response.content.is_empty());
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
            !matches!(result, Err(LlmError::EmptyCompletion(_))),
            "a SAFETY refusal must not be reported as EmptyCompletion (token-budget problem), got {result:?}"
        );
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
        assert!(matches!(error, LlmError::RateLimitExceeded));
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
        assert!(matches!(error, LlmError::RateLimitExceeded));
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
    async fn generate_recitation_finish_reason_produces_error_not_stop() {
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
        match response.finish_reason {
            FinishReason::Error(reason) => assert_eq!(reason, "RECITATION"),
            other => panic!("expected FinishReason::Error(\"RECITATION\"), got {other:?}"),
        }
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
        assert!(matches!(result, Err(LlmError::RateLimitExceeded)));
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
}
