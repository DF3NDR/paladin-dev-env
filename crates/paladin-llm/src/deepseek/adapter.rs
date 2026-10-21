//! DeepSeek LLM Adapter
//!
//! Provides integration with DeepSeek's API, which is OpenAI-compatible.
//! Supports standard completions, streaming, and all core LlmPort functionality.

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use reqwest::{
    Client,
    header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::time::{Duration, SystemTime};

use paladin_core::platform::container::prompt::{PromptItem, PromptType};
use paladin_ports::output::llm_port::ReasoningEffort;
use paladin_ports::output::llm_port::{
    FinishReason, LlmError, LlmPort, LlmRequest, LlmResponse, ModelProfile, ParamSupport,
    ProviderCapabilities, StopSupport, StreamingResponse, TokenUsage,
};
use paladin_ports::output::model_profile::{EffortLevels, EffortSupport, ResolvedEffort};
use paladin_ports::output::rate_limit_hints::RateLimitHints;

use crate::backoff::exponential_backoff;
use crate::http_status::map_http_status_with_hints;
use crate::rate_limit_headers::{RateLimitHeaderFamily, hints_from_headers};
use crate::reasoning::{self, SubstitutionLog, WireCaps, mask_effort};
use crate::redaction::RedactedKey;
use crate::registry::{
    DEFAULT_MAX_RETRIES, ModelOverride, ModelRegistry, config_knob_builders,
    default_timeout_seconds, is_http_url, seeded, seeded_window,
};
use crate::response_guards::{StreamCompletion, bill, classify_completion};
use crate::sampling::{self, FormatLog, OmissionLog, RequestNotes};
use crate::sse::SseLines;
use crate::usage_reconcile::{USAGE_UNRECONCILED_METADATA_KEY, UsageLog};
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

/// Default DeepSeek model requested when `DEEPSEEK_MODEL` is unset.
///
/// `deepseek-flash`: the general-purpose model in the live `GET /models` listing of the
/// 2026-10-15 probe and the operator-identified current default (`registry/seeded.rs`,
/// `verified_on` 2026-10-15). It replaces `deepseek-chat`, which is absent from that listing
/// and from the vendor's pricing page (an undocumented alias that still answered 200 at probe
/// time). Pin `DEEPSEEK_MODEL` to keep `deepseek-chat`.
pub const DEEPSEEK_DEFAULT_MODEL: &str = "deepseek-flash";

/// The DeepSeek endpoint [`DeepSeekConfig::new`] and [`DeepSeekConfig::from_env`] use when no
/// other is chosen, and the compiled default of the layered `llm.deepseek` configuration.
///
/// Replace it with [`DeepSeekConfig::with_base_url`] (or `DEEPSEEK_BASE_URL` for `from_env`).
pub const DEEPSEEK_DEFAULT_BASE_URL: &str = "https://api.deepseek.com/v1";

/// Parse a `429` response's rate-limit headers through the generic family (PACE-01).
///
/// Must run BEFORE `response.text()` consumes the response (research Pitfall 2). Only a 429
/// is parsed, and no credential header is read.
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

/// Configuration for DeepSeek LLM adapter.
///
/// Marked `#[non_exhaustive]` (owner decision, Phase 50 plan 50-08, option-a): build one with
/// [`DeepSeekConfig::new`] (an API key and a model) or [`DeepSeekConfig::from_env`] and set
/// fields or call the `with_*` builders (`with_base_url`, `with_timeout_seconds`, ...), so a later field is not another break for callers.
#[derive(Clone)]
#[non_exhaustive]
pub struct DeepSeekConfig {
    /// API key for DeepSeek authentication.
    pub api_key: String,
    /// Base URL for DeepSeek API; [`DEEPSEEK_DEFAULT_BASE_URL`] unless replaced with
    /// [`with_base_url`](Self::with_base_url) (or `DEEPSEEK_BASE_URL` for `from_env`).
    pub base_url: String,
    /// The adapter's default model, which selects the timeout default;
    /// [`DEEPSEEK_DEFAULT_MODEL`] when constructed from the environment with `DEEPSEEK_MODEL`
    /// unset. Each request's own `LlmRequest.model` decides what is sent on the wire.
    pub model: String,
    /// Request timeout in seconds. [`DeepSeekConfig::new`] and [`DeepSeekConfig::from_env`]
    /// default it to 300 for a seeded reasoning model and to 60 otherwise (D-13); an explicit
    /// `DEEPSEEK_TIMEOUT_SECONDS`, [`with_timeout_seconds`](Self::with_timeout_seconds) call or
    /// field assignment always wins. A model marked `reasoning`
    /// only through [`model_overrides`](Self::model_overrides) does not change this default
    /// here: set this field in the same place.
    pub timeout_seconds: u64,
    /// Default output budget (`max_tokens`) sent when a request sets none. `None` (the default)
    /// sends no `max_tokens`. The value is checked against the model's profiled maximum output
    /// exactly as a request's own `max_tokens` is (ADR-0061 (i)): above it, the request is
    /// refused before any HTTP call.
    pub max_tokens: Option<u32>,
    /// How many times a retryable error is retried after the first attempt. Defaults to `3`
    /// (up to four calls); `0` sends each request once. A 429 is never retried here (the Cadence
    /// decorator waits) and a permanent error is never retried.
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
}

/// Prints every field except the API key, which is shown as `<redacted>` (D-00a): the key is a
/// public `String`, so a derived `Debug` would put it into any log line or panic message that
/// formats the config.
impl std::fmt::Debug for DeepSeekConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeepSeekConfig")
            .field("api_key", &RedactedKey(&self.api_key))
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("timeout_seconds", &self.timeout_seconds)
            .field("max_tokens", &self.max_tokens)
            .field("max_retries", &self.max_retries)
            .field("context_window", &self.context_window)
            .field("model_overrides", &self.model_overrides)
            .finish()
    }
}

config_knob_builders!(DeepSeekConfig);

impl DeepSeekConfig {
    /// Load configuration from environment variables.
    ///
    /// # Environment Variables
    /// - `DEEPSEEK_API_KEY` (required): DeepSeek API key
    /// - `DEEPSEEK_BASE_URL` (optional): API base URL
    /// - `DEEPSEEK_MODEL` (optional): Default model
    /// - `DEEPSEEK_TIMEOUT_SECONDS` (optional): Request timeout, 300 for a seeded reasoning
    ///   model and 60 otherwise
    ///
    /// # Errors
    /// Returns error if required environment variables are missing or invalid.
    pub fn from_env() -> Result<Self, String> {
        let api_key = env::var("DEEPSEEK_API_KEY")
            .map_err(|_| "DEEPSEEK_API_KEY environment variable not set")?;

        let base_url =
            env::var("DEEPSEEK_BASE_URL").unwrap_or_else(|_| DEEPSEEK_DEFAULT_BASE_URL.to_string());

        let model =
            env::var("DEEPSEEK_MODEL").unwrap_or_else(|_| DEEPSEEK_DEFAULT_MODEL.to_string());

        let timeout_seconds = match env::var("DEEPSEEK_TIMEOUT_SECONDS") {
            Ok(value) => value
                .parse()
                .map_err(|_| "Invalid DEEPSEEK_TIMEOUT_SECONDS value")?,
            Err(_) => default_timeout_seconds(DEEPSEEK_PROVIDER, &model, 60),
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
        };

        config.validate()?;
        Ok(config)
    }

    /// Create a configuration for `model`, authenticating with `api_key`.
    ///
    /// - `api_key`: the DeepSeek API key, never a URL.
    /// - `model`: the adapter's default model. It selects the timeout default (300 s for a
    ///   seeded reasoning model, 60 s otherwise); each request's own `LlmRequest.model`
    ///   decides what is sent on the wire.
    ///
    /// The endpoint is [`DEEPSEEK_DEFAULT_BASE_URL`] (replace it with
    /// [`with_base_url`](Self::with_base_url)) and the timeout is 300 s for a seeded reasoning
    /// model and 60 s otherwise (replace it with
    /// [`with_timeout_seconds`](Self::with_timeout_seconds)). Construction is infallible: an
    /// empty key or model, or a non-`http(s)` endpoint, is refused when the adapter is built.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_llm::deepseek::DeepSeekConfig;
    /// use paladin_llm::deepseek::adapter::DEEPSEEK_DEFAULT_BASE_URL;
    ///
    /// let key = "sk-example".to_string();
    /// let config = DeepSeekConfig::new(key, "deepseek-flash".to_string()).with_timeout_seconds(600);
    /// assert_eq!(config.base_url, DEEPSEEK_DEFAULT_BASE_URL);
    /// assert_eq!(config.timeout_seconds, 600);
    /// ```
    pub fn new(api_key: String, model: String) -> Self {
        let timeout_seconds = default_timeout_seconds(DEEPSEEK_PROVIDER, &model, 60);
        Self {
            api_key,
            base_url: DEEPSEEK_DEFAULT_BASE_URL.to_string(),
            model,
            timeout_seconds,
            max_tokens: None,
            max_retries: DEFAULT_MAX_RETRIES,
            context_window: None,
            model_overrides: BTreeMap::new(),
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
        if !is_http_url(&self.base_url) {
            return Err("Base URL must start with http:// or https://".to_string());
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
    /// The caller's stop sequences, in declared order with duplicates kept, when the model's
    /// profile admits their count (ADR-0061 (a), D-11a). Absent otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
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
    /// `LlmRequest.reasoning` as a listed level, on the wire as `reasoning_effort`
    /// (Phase 51, D-00f). Omitted entirely when no effort is set or the effort is `Off` /
    /// `Auto`, which travel as the `thinking` switch alone, keeping an unset request's body
    /// byte-identical (D-00i). Placed last so the existing field order is untouched.
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<&'static str>,
    /// DeepSeek's thinking switch: `enabled` beside a level whose profile says
    /// `enable_thinking_with_level`, and for `Auto`; `disabled` for `Off`. Omitted otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking: Option<DeepSeekThinking>,
}

/// `DeepSeekRequest.thinking`'s only shape this adapter sends: `{"type": "enabled"}` or
/// `{"type": "disabled"}`.
#[derive(Debug, Serialize)]
struct DeepSeekThinking {
    #[serde(rename = "type")]
    kind: &'static str,
}

/// What DeepSeek's request body can carry of a profile's effort data (Phase 51, D-00f): a listed
/// level (`reasoning_effort`), `off` and `auto` (the `thinking` switch). It has no field for a
/// reasoning-token budget, so a budget is masked out of the profile this adapter reports and is
/// never sent. Plan 51-12 confirms each form against the dated probe or sets this to
/// [`WireCaps::NONE`] (D-04).
const DEEPSEEK_REASONING_WIRE: WireCaps = WireCaps {
    levels: true,
    off: true,
    auto: true,
    budget: false,
};

/// Turn an admitted effort into DeepSeek's two wire fields.
///
/// `levels` is the model's (masked) effort data: its `enable_thinking_with_level` flag decides
/// whether a level also sends `thinking: {type: enabled}`.
///
/// # Errors
///
/// [`LlmError::UnsupportedParameter`] for an effort this wire cannot carry. The profile masking
/// and `admit` already refuse those, so this is the last line of defence: it can never put a
/// field on the wire that the request did not ask for.
fn map_reasoning(
    model: &str,
    resolved: &ResolvedEffort,
    levels: Option<&EffortLevels>,
) -> Result<(Option<&'static str>, Option<DeepSeekThinking>), LlmError> {
    if !resolved.on_wire {
        return Ok((None, None));
    }
    if let Some(level) = reasoning::level_spelling(resolved.effective) {
        let thinking = levels
            .filter(|l| l.enable_thinking_with_level)
            .map(|_| DeepSeekThinking { kind: "enabled" });
        return Ok((Some(level), thinking));
    }
    // `Off` switches thinking off and `Auto` switches it on at the vendor's own depth; neither
    // carries a level.
    match resolved.effective {
        ReasoningEffort::Off => {
            return Ok((None, Some(DeepSeekThinking { kind: "disabled" })));
        }
        ReasoningEffort::Auto => {
            return Ok((None, Some(DeepSeekThinking { kind: "enabled" })));
        }
        _ => {}
    }
    Err(LlmError::unsupported_parameter(
        "reasoning_effort",
        resolved.effective.to_string(),
        DEEPSEEK_PROVIDER,
        model,
        "this adapter has no wire form for that effort",
    ))
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
    /// DeepSeek's own reported total (G-50-5a). Parsed and handed to the
    /// reconciliation ([`crate::usage_reconcile`]), never copied:
    /// `TokenUsage::new` derives `total_tokens` as `prompt_tokens +
    /// completion_tokens` (D-02), and a disagreement is recorded under
    /// [`USAGE_UNRECONCILED_METADATA_KEY`]. `None` when the key is absent --
    /// a usage object without it still parses, and nothing is fabricated.
    #[serde(default)]
    total_tokens: Option<u32>,
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
    /// Hidden chain-of-thought text streamed on this delta, in wire order (D-11).
    #[serde(default)]
    reasoning_content: Option<String>,
}

/// Map a wire-reported [`DeepSeekUsage`] into [`TokenUsage`], applying the
/// D-20 cache/reasoning sub-count builders only when the payload actually
/// carried the figure (D-03: an absent figure is `None`, never a fabricated
/// `Some(0)`). Shared by both the non-streaming usage-construction site and
/// the streaming terminal-chunk usage, so the two paths cannot drift.
///
/// Returns the mapped usage together with DeepSeek's own `total_tokens` (when
/// reported), which the caller reconciles rather than copies (D-02, G-50-5a).
///
/// Reasoning is read inside `completion_tokens`, live-measured 2026-10-18
/// (`50-USAGE-PROBE.md`, plan 50-24): deepseek-flash reads `reasoning inside
/// completion` (50 + 32 = 82, reasoning 30).
fn map_usage(usage: DeepSeekUsage) -> (TokenUsage, Option<u32>) {
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
    (mapped, usage.total_tokens)
}

/// Deserialize a possibly-`null` (or absent) string field as an empty string.
///
/// DeepSeek's reasoning models split their `max_tokens` budget between hidden
/// `reasoning_content` and visible `content`. When the hidden reasoning
/// consumes the entire budget, the API may report the empty answer as either
/// `"content": ""` or `"content": null`. The `""` form is classified by
/// `classify_completion` into an actionable
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
    /// Resolves the request model to the profile that decides what reaches the wire (ADR-0061).
    model_registry: ModelRegistry,
    /// Which (model, parameter) omissions this instance has already warned about.
    omissions: OmissionLog,
    /// Which (model, adjustment) `response_format` changes this instance has already warned
    /// about.
    format_log: FormatLog,
    /// Which (model, check) usage findings this instance has already warned about (G-50-5a).
    usage_log: UsageLog,
    /// Which (model, requested, effective) effort substitutions this instance has already warned
    /// about (Phase 51, D-06).
    substitutions: SubstitutionLog,
}

/// What this adapter's request builder carries today, stated once so the adapter-wide fallback
/// profile and [`LlmPort::get_capabilities`] cannot drift apart.
fn declared_capabilities(context_window: Option<u32>) -> ProviderCapabilities {
    ProviderCapabilities {
        supports_streaming: true,
        supports_tool_calling: false,
        supports_function_calling: false,
        supports_vision: false,
        supports_embeddings: false,
        // The default model's seeded window (`seeded::DEEPSEEK`, `deepseek-flash`, verified
        // 2026-10-15: listing `context_window`), replacing the unevidenced 64,000.
        // The operator's adapter-wide window (PARITY-03) replaces it when set.
        max_context_tokens: context_window
            .or(seeded_window(seeded::DEEPSEEK, DEEPSEEK_DEFAULT_MODEL)),
        supports_system_messages: true,
        temperature_range: Some((0.0, 2.0)),
    }
}

/// The adapter-wide fallback profile, derived from what this adapter's request builder sends
/// (ADR-0061, D-05): a field it sends maps to `Range`, a field it never sends maps to `NotSent`.
///
/// - `temperature`: `Range` over the declared `temperature_range`; `top_p`: `Range(0.0, 1.0)`;
///   both penalties: `Range(-2.0, 2.0)` -- [`DeepSeekRequest`] carries all four.
/// - `stop`: `Supported { max: None }` (the vendor's documented limit is not carried here; the
///   per-model seeded entries of plan 50-06 may state one).
/// - `json_object` true and `json_schema` false: every `ResponseFormat` compiles down to
///   DeepSeek's plain JSON-object form today, so a schema request is a recorded downgrade.
fn fallback_profile(context_window: Option<u32>) -> ModelProfile {
    let caps = declared_capabilities(context_window);
    let (low, high) = caps.temperature_range.unwrap_or((0.0, 1.0));
    let mut profile = ModelProfile::new()
        .with_temperature(ParamSupport::Range(low, high))
        .with_top_p(ParamSupport::Range(0.0, 1.0))
        .with_frequency_penalty(ParamSupport::Range(-2.0, 2.0))
        .with_presence_penalty(ParamSupport::Range(-2.0, 2.0))
        .with_stop_sequences(StopSupport::Supported { max: None })
        .with_json_object(true)
        .with_json_schema(false);
    if let Some(window) = caps.max_context_tokens {
        profile = profile.with_context_window(window);
    }
    profile
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

        // PARITY-03: the operator's adapter-wide window, then the per-model overrides, are laid
        // over the seeded table; either one failing construction names the provider and model.
        let model_registry = ModelRegistry::new(
            DEEPSEEK_PROVIDER,
            seeded::DEEPSEEK,
            fallback_profile(config.context_window),
        )
        .with_context_window(config.context_window)
        .and_then(|registry| registry.with_overrides(config.model_overrides.clone()))
        .map_err(|e| {
            LlmError::AuthenticationError(format!("Invalid DeepSeek configuration: {e}"))
        })?;

        Ok(Self {
            client,
            config,
            model_registry,
            omissions: OmissionLog::new(),
            format_log: FormatLog::new(),
            usage_log: UsageLog::new(),
            substitutions: SubstitutionLog::new(),
        })
    }

    /// The request model's profile as this adapter's wire can honour it: the effort data has
    /// whatever DeepSeek's body cannot carry removed ([`DEEPSEEK_REASONING_WIRE`]), so a caller
    /// reading [`LlmPort::model_profile`] judges exactly what [`Self::build_request`] will send.
    fn wire_profile(&self, model: &str) -> ModelProfile {
        let mut profile = self.model_registry.profile(model);
        profile.effort = mask_effort(std::mem::take(&mut profile.effort), DEEPSEEK_REASONING_WIRE);
        profile
    }

    /// Build DeepSeek API request from LlmRequest, plus the notes the buffered path records on
    /// its response.
    ///
    /// What reaches the wire is decided by the request model's [`ModelProfile`] (ADR-0061,
    /// D-01): [`sampling::apply`] sends a sampling parameter or the stop sequences only when
    /// the caller set them **and** the profile admits them; a set but inadmissible one is
    /// omitted and recorded (one `warn!` per instance per model and parameter, and
    /// `paladin.params_omitted` on the response), never clamped or swapped. A `max_tokens`
    /// above the profile's maximum output is refused before anything is sent. The
    /// `response_format` follows the profile through [`sampling::apply_response_format`].
    fn build_request(
        &self,
        request: &LlmRequest,
    ) -> Result<(DeepSeekRequest, RequestNotes), LlmError> {
        let messages = self.convert_prompt_to_messages(&request.prompt)?;
        let params = &request.prompt.entity.data.parameters;
        let profile = self.wire_profile(&request.model);

        // The caller's value, else the configured default; either is held to the model's
        // maximum output before anything is sent.
        let max_tokens = params.max_tokens.or(self.config.max_tokens);
        sampling::check_max_tokens(DEEPSEEK_PROVIDER, &request.model, &profile, max_tokens)?;

        // Phase 51 (D-00f): an explicit effort is sent exactly or refused here, before any HTTP
        // call and before any retry loop -- never routed through the omit-and-log path below.
        let resolved = reasoning::admit(
            DEEPSEEK_PROVIDER,
            &request.model,
            &profile,
            request.reasoning,
            request.reasoning_fallback,
            max_tokens,
            &self.substitutions,
        )?;
        let (reasoning_effort, thinking) = match &resolved {
            Some(resolved) => {
                let levels = match &profile.effort {
                    EffortSupport::Supported(levels) => Some(levels),
                    _ => None,
                };
                map_reasoning(&request.model, resolved, levels)?
            }
            None => (None, None),
        };

        let applied = sampling::apply(&profile, params);
        let params_omitted = self
            .omissions
            .record(DEEPSEEK_PROVIDER, &request.model, &applied);

        // D-28 / ADR-0061 (i): DeepSeek has one native JSON mode, the plain JSON-object form,
        // so an effective format -- object or schema -- maps to it. The fallback profile
        // (`json_object` true, `json_schema` false) makes a schema request a recorded
        // downgrade (`paladin.response_format_adjusted = json_object`).
        let (effective_format, adjustment) =
            sampling::apply_response_format(&profile, request.response_format.as_ref());
        let format_adjusted = self
            .format_log
            .record(DEEPSEEK_PROVIDER, &request.model, adjustment);
        let response_format = effective_format.map(|_| DeepSeekResponseFormat {
            kind: "json_object",
        });

        let unprofiled_model = self
            .model_registry
            .note_unprofiled(&request.model)
            .then(|| request.model.clone());

        let body = DeepSeekRequest {
            model: request.model.clone(),
            messages,
            temperature: applied.temperature,
            max_tokens,
            top_p: applied.top_p,
            frequency_penalty: applied.frequency_penalty,
            presence_penalty: applied.presence_penalty,
            stop: applied.stop,
            stream: request.stream,
            response_format,
            // Set unconditionally to `None` here; `generate_stream` forces
            // it to `Some` right after building this request, mirroring the
            // existing defensive `api_request.stream = true` override below
            // (D-15).
            stream_options: None,
            reasoning_effort,
            thinking,
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

    /// Convert PromptItem to DeepSeek messages.
    fn convert_prompt_to_messages(
        &self,
        prompt: &PromptItem,
    ) -> Result<Vec<DeepSeekMessage>, LlmError> {
        let mut messages = Vec::new();

        match &prompt.entity.data.prompt_type {
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
                    content: crate::turn::user_turn_text(user_prompt),
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

    /// Map a non-2xx DeepSeek response to [`LlmError`] with no rate-limit headers.
    ///
    /// A thin wrapper over [`Self::map_error_with_hints`], kept so the status-mapping unit
    /// tests read the same as before PACE-01. Production paths call the hints variant.
    #[cfg(test)]
    fn map_error(&self, status: u16, body: &str) -> LlmError {
        self.map_error_with_hints(status, body, None)
    }

    /// Map a non-2xx DeepSeek response to [`LlmError`], carrying the parsed rate-limit
    /// headers of a `429` (PACE-01).
    ///
    /// `300..=399` is named explicitly (CR-02, mirroring
    /// `CompatEngine::map_error`/`GeminiAdapter::map_error`) because this
    /// client's redirect policy is `none` (see [`Self::new`]), so a `3xx`
    /// response is never followed — it arrives here as an ordinary
    /// non-success status instead. Everything else delegates wholesale to
    /// the crate-wide [`map_http_status_with_hints`] (Phase 25 D-03, FT-FR-01): `body`
    /// is the RAW response text, redacted and bounded once inside the
    /// helper — never pre-excerpted here, which would bound twice.
    /// DeepSeek's documented insufficient-balance status is 402; the
    /// helper's 402 arm carries `regain_hint: None` because DeepSeek's 402
    /// body shape is not first-party-confirmed (Phase 41 RESEARCH
    /// Assumption A1), so there is no prose to parse yet.
    fn map_error_with_hints(
        &self,
        status: u16,
        body: &str,
        hints: Option<RateLimitHints>,
    ) -> LlmError {
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
            _ => map_http_status_with_hints(
                DEEPSEEK_PROVIDER,
                status,
                body,
                &self.config.api_key,
                hints,
            ),
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
    ///   ModelNotAvailable | TokenLimitExceeded` and never retry
    ///   `AuthenticationError | InvalidPrompt | EmptyCompletion |
    ///   UsageLimitExceeded | RateLimitExceeded`.
    ///
    /// **A 429 is surfaced on the first attempt (Phase 43 D-02).** Before
    /// PACE-01 this loop retried `RateLimitExceeded` with a 100 ms-scale
    /// backoff, which re-sent into a limit the provider had just announced.
    /// A 429 is the one error whose cure is waiting, and the Cadence
    /// decorator above this adapter owns waiting with a gate shared by every
    /// caller of the same provider and model; the error now carries the
    /// provider's own `Retry-After` for it to honour. Network, timeout, 5xx
    /// and decode retries are unchanged.
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

                    let backoff = exponential_backoff(
                        Duration::from_millis(100),
                        attempt,
                        Duration::from_secs(10),
                    );
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
        let (api_request, notes) = self.build_request(&request)?;
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
                // Snapshot the 429 headers BEFORE `.text()` consumes the response
                // (research Pitfall 2).
                let hints = snapshot_rate_limit_hints(status, response.headers());
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                return Err(self.map_error_with_hints(status.as_u16(), &error_text, hints));
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

            // G-50-5a: usage is mapped BEFORE the no-choices check and the guard, so a call
            // DeepSeek billed but that is classified as empty carries the same typed usage a
            // success would have, on the served model (else the request model, A3-50-26). The
            // counts are typed on the error now, so the detail text no longer embeds them (D-09:
            // nothing parses text).
            let (usage, vendor_total) = map_usage(api_response.usage);
            let billed_model = if api_response.model.is_empty() {
                request.model.as_str()
            } else {
                api_response.model.as_str()
            };

            // WR-04: a body with no choices is an `EmptyCompletion` carrying the bill, as on the
            // compat engine and Gemini -- not a `ProcessingError`, which dropped the usage and
            // read as `Transience::Unknown` to the retry loop (a billed request re-sent).
            let Some(choice) = api_response.choices.first() else {
                return Err(bill(
                    LlmError::empty_completion("Response contained no choices".to_string()),
                    billed_model,
                    Some(usage),
                ));
            };

            let finish_reason = Self::map_finish_reason(choice.finish_reason.clone());

            // One rule for refusal, content filter, function call and empty text on any
            // finish reason (PARITY-05, D-11a); this adapter's private copy is gone. DeepSeek
            // sends no dedicated refusal field, so there is none to pass.
            classify_completion(
                self.get_provider_name(),
                &request.model,
                &choice.message.content,
                &finish_reason,
                None,
                &self.config.api_key,
            )
            .map_err(|err| bill(err, billed_model, Some(usage.clone())))?;

            let content = choice.message.content.clone();
            // G-50-5a: DeepSeek's own total is the check on the mapping -- a disagreement is
            // warned once and recorded, never written over the mapped total (D-02).
            let unreconciled = self.usage_log.record(
                self.get_provider_name(),
                &request.model,
                &usage,
                vendor_total,
            );

            let mut llm_response = LlmResponse::new(
                request.id,
                api_response.model,
                content,
                finish_reason,
                usage,
            );
            if let Some(value) = unreconciled {
                llm_response
                    .metadata
                    .insert(USAGE_UNRECONCILED_METADATA_KEY.to_string(), value);
            }
            if let Some(reasoning) = &choice.message.reasoning_content {
                llm_response = llm_response.with_reasoning(reasoning.as_str());
            }
            Ok(llm_response)
        };

        let mut response = self
            .call_api_with_retry(operation, self.config.max_retries)
            .await?;
        // The request-time notes (ADR-0061 visibility rule) ride on the successful response,
        // beside any usage finding the parse recorded.
        response.metadata.extend(notes.into_metadata());
        Ok(response)
    }

    async fn generate_stream(
        &self,
        request: LlmRequest,
    ) -> Result<Box<dyn Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError> {
        // ADR-0061 (h), owner reading R-1 (option-b): the omissions, format changes and
        // unprofiled model `build_request` `warn!`s about are also stamped on the stream's
        // terminal chunk (only that chunk), as the buffered path stamps `LlmResponse.metadata`.
        let (mut api_request, notes) = self.build_request(&request)?;
        let stream_metadata = notes.into_metadata();
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
            // Same header snapshot as the buffered path, taken before the body read.
            let hints = snapshot_rate_limit_hints(status, response.headers());
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(self.map_error_with_hints(status.as_u16(), &error_text, hints));
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
        // The vendor's own total from the held usage frame, reconciled at the terminal chunk
        // (G-50-5a). A clone shares this instance's usage log.
        let mut held_vendor_total: Option<u32> = None;
        let usage_log = self.usage_log.clone();

        // D-11a on the streaming path: the terminal chunk is decided by the same
        // `classify_completion` the buffered path uses, over what the stream accumulated.
        let provider = self.get_provider_name();
        let model = request.model.clone();
        let api_key = self.config.api_key.clone();
        let mut completion = StreamCompletion::default();

        // A network read can end inside a `data:` line (or inside a multi-byte character), so the
        // bytes are reassembled into whole lines before any line is parsed (CR-02).
        let mut sse = SseLines::default();
        let llm_stream = stream
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
                        let mut items = Vec::new();

                        for line in &lines {
                            let Some(json_str) = line.strip_prefix("data: ") else {
                                continue;
                            };

                            if json_str.trim() == "[DONE]" {
                                let finish_reason =
                                    held_finish_reason.take().unwrap_or(FinishReason::Stop);
                                match completion.finish(provider, &model, &finish_reason, &api_key)
                                {
                                    Ok(()) => {
                                        let mut terminal =
                                            StreamingResponse::terminal(finish_reason);
                                        if let Some(usage) = held_usage.take() {
                                            // G-50-5a: only the `Ok` terminal chunk carries a
                                            // finding; an `Err` terminal carries no metadata.
                                            if let Some(value) = usage_log.record(
                                                provider,
                                                &model,
                                                &usage,
                                                held_vendor_total.take(),
                                            ) {
                                                terminal = terminal.with_metadata(
                                                    USAGE_UNRECONCILED_METADATA_KEY,
                                                    value,
                                                );
                                            }
                                            terminal = terminal.with_usage(usage);
                                        }
                                        for (key, value) in &stream_metadata {
                                            terminal = terminal
                                                .with_metadata(key.as_str(), value.as_str());
                                        }
                                        items.push(Ok(terminal));
                                    }
                                    // An empty answer replaces the terminal chunk; deltas already
                                    // yielded (reasoning included) stay yielded. G-50-5a: the
                                    // error carries the usage frame's bill on the request
                                    // model, `None` when the stream reported none.
                                    Err(error) => {
                                        items.push(Err(bill(error, &model, held_usage.take())))
                                    }
                                }
                                continue;
                            }

                            match serde_json::from_str::<DeepSeekStreamResponse>(json_str) {
                                Ok(response) => {
                                    if let Some(usage) = response.usage {
                                        let (mapped, vendor_total) = map_usage(usage);
                                        held_usage = Some(mapped);
                                        held_vendor_total = vendor_total;
                                    }
                                    if let Some(choice) = response.choices.first() {
                                        if let Some(reason) = &choice.finish_reason {
                                            held_finish_reason =
                                                Some(Self::map_finish_reason(Some(reason.clone())));
                                        }
                                        let content =
                                            choice.delta.content.clone().unwrap_or_default();
                                        completion.observe_content(&content);
                                        let mut delta = StreamingResponse::delta(content);
                                        if let Some(reasoning) = &choice.delta.reasoning_content {
                                            delta = delta.with_reasoning(reasoning.as_str());
                                        }
                                        items.push(Ok(delta));
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

    /// The ids in the dated DeepSeek table (`seeded::DEEPSEEK`), so the list always includes
    /// [`DEEPSEEK_DEFAULT_MODEL`]. `deepseek-chat` and `deepseek-coder` are not listed: the
    /// 2026-10-15 probe found them in neither the live listing nor the vendor's pricing page.
    async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
        Ok(seeded::DEEPSEEK
            .iter()
            .map(|entry| entry.model.to_string())
            .collect())
    }

    fn get_provider_name(&self) -> &'static str {
        DEEPSEEK_PROVIDER
    }

    fn get_capabilities(&self) -> ProviderCapabilities {
        declared_capabilities(self.config.context_window)
    }

    fn model_profile(&self, model: &str) -> Option<ModelProfile> {
        Some(self.wire_profile(model))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_ports::output::llm_port::ResponseFormat;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn test_deepseek_config_validation() {
        let config = DeepSeekConfig::new("test-key".to_string(), "deepseek-chat".to_string());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_deepseek_config_empty_api_key() {
        let config = DeepSeekConfig::new("".to_string(), "deepseek-chat".to_string());
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_deepseek_config_invalid_url() {
        let config = DeepSeekConfig::new("test-key".to_string(), "deepseek-chat".to_string())
            .with_base_url("invalid-url");
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_deepseek_adapter_creation() {
        let config = DeepSeekConfig::new("test-key".to_string(), "deepseek-chat".to_string());
        let adapter = DeepSeekAdapter::new(config);
        assert!(adapter.is_ok());
    }

    #[test]
    fn new_defaults_the_endpoint_and_keeps_the_model() {
        let config = DeepSeekConfig::new("k".to_string(), "deepseek-chat".to_string());
        assert_eq!(config.base_url, DEEPSEEK_DEFAULT_BASE_URL);
        assert_eq!(config.model, "deepseek-chat");
        assert_eq!(DEEPSEEK_DEFAULT_BASE_URL, "https://api.deepseek.com/v1");
    }

    #[test]
    fn new_defaults_the_timeout_by_whether_the_model_is_a_seeded_reasoning_model() {
        // `deepseek-flash` is seeded as a reasoning model (D-13): 300 s.
        let reasoning = DeepSeekConfig::new("k".to_string(), "deepseek-flash".to_string());
        assert_eq!(reasoning.timeout_seconds, 300);
        // `deepseek-v4-pro` is seeded as a reasoning model too (Phase 50.1, probe 1 returned
        // reasoning text): 300 s.
        let pro = DeepSeekConfig::new("k".to_string(), "deepseek-v4-pro".to_string());
        assert_eq!(pro.timeout_seconds, 300);
        // An unseeded model keeps the provider's own default of 60 s.
        let plain = DeepSeekConfig::new("k".to_string(), "unseeded-model".to_string());
        assert_eq!(plain.timeout_seconds, 60);
    }

    #[test]
    fn the_endpoint_and_timeout_builders_are_order_independent() {
        let a = DeepSeekConfig::new("k".to_string(), "m".to_string())
            .with_base_url("https://example.invalid/v1")
            .with_timeout_seconds(7);
        let b = DeepSeekConfig::new("k".to_string(), "m".to_string())
            .with_timeout_seconds(7)
            .with_base_url("https://example.invalid/v1");
        assert_eq!(
            (&a.base_url, a.timeout_seconds, &a.model),
            (&b.base_url, b.timeout_seconds, &b.model)
        );
        assert_eq!(a.base_url, "https://example.invalid/v1");
        assert_eq!(a.timeout_seconds, 7);
    }

    #[test]
    fn the_last_endpoint_or_timeout_call_wins_and_beats_the_reasoning_default() {
        let config = DeepSeekConfig::new("k".to_string(), "deepseek-flash".to_string())
            .with_base_url("https://first.invalid/v1")
            .with_base_url("https://second.invalid/v1")
            .with_timeout_seconds(7)
            .with_timeout_seconds(9);
        assert_eq!(config.base_url, "https://second.invalid/v1");
        // An explicit value beats the 300 s reasoning default `new` chose.
        assert_eq!(config.timeout_seconds, 9);
    }

    #[test]
    fn empty_arguments_build_a_config_but_not_an_adapter() {
        // `new` is infallible; the empty key and model are refused when the adapter is built.
        let config = DeepSeekConfig::new(String::new(), String::new());
        assert!(config.validate().is_err());
        assert!(DeepSeekAdapter::new(config).is_err());
    }

    #[test]
    fn a_builder_set_non_http_base_url_fails_deepseek_construction() {
        for bad in [
            "ftp://x",
            "",
            "api.deepseek.com/v1",
            "httpx://x",
            "http-gateway.internal/v1",
            "httpfoo:",
        ] {
            let config = DeepSeekConfig::new("k".to_string(), "deepseek-chat".to_string())
                .with_base_url(bad);
            assert!(
                DeepSeekAdapter::new(config).is_err(),
                "base URL {bad:?} must fail adapter construction"
            );
        }
    }

    #[tokio::test]
    async fn test_deepseek_provider_capabilities() {
        let config = DeepSeekConfig::new("test-key".to_string(), "deepseek-chat".to_string());
        let adapter = DeepSeekAdapter::new(config).unwrap();
        let capabilities = adapter.get_capabilities();

        assert!(capabilities.supports_streaming);
        assert!(!capabilities.supports_tool_calling);
        assert!(!capabilities.supports_vision);
        assert!(capabilities.supports_system_messages);
        assert_eq!(capabilities.max_context_tokens, Some(1_048_576));
        assert_eq!(capabilities.temperature_range, Some((0.0, 2.0)));
        assert_eq!(adapter.get_provider_name(), "deepseek");
    }

    #[test]
    fn deepseek_default_model_is_seeded() {
        let adapter = DeepSeekAdapter::new(DeepSeekConfig::new(
            "test-key".to_string(),
            DEEPSEEK_DEFAULT_MODEL.to_string(),
        ))
        .unwrap();
        let (profile, origin) = adapter.model_registry.resolve(DEEPSEEK_DEFAULT_MODEL);
        assert_eq!(
            origin,
            crate::registry::ProfileOrigin::Seeded {
                verified_on: "2026-10-15"
            }
        );
        assert_eq!(profile.context_window, Some(1_048_576));
        assert_eq!(profile.max_output_tokens, Some(393_216));
    }

    #[test]
    fn deepseek_default_model_is_the_operator_identified_current_model() {
        assert_eq!(DEEPSEEK_DEFAULT_MODEL, "deepseek-flash");
    }

    #[tokio::test]
    async fn deepseek_available_models_come_from_the_seeded_table_and_include_the_default() {
        let adapter = DeepSeekAdapter::new(DeepSeekConfig::new(
            "test-key".to_string(),
            DEEPSEEK_DEFAULT_MODEL.to_string(),
        ))
        .unwrap();
        let models = adapter.get_available_models().await.unwrap();
        assert!(models.contains(&DEEPSEEK_DEFAULT_MODEL.to_string()));
        assert!(
            adapter
                .validate_model(DEEPSEEK_DEFAULT_MODEL)
                .await
                .unwrap()
        );
        assert!(!adapter.validate_model("deepseek-coder").await.unwrap());
    }

    /// The shared guard, as this adapter calls it: no refusal field, the adapter's own key.
    fn classify(content: &str, finish: &FinishReason) -> Result<(), LlmError> {
        classify_completion(
            DEEPSEEK_PROVIDER,
            "deepseek-test",
            content,
            finish,
            None,
            "k",
        )
    }

    #[test]
    fn test_empty_completion_length_and_empty_is_truncation() {
        let result = classify("", &FinishReason::Length);
        assert!(matches!(result, Err(LlmError::EmptyCompletion { .. })));
    }

    #[test]
    fn test_empty_completion_length_and_whitespace_is_truncation() {
        let result = classify("   \n\t", &FinishReason::Length);
        assert!(matches!(result, Err(LlmError::EmptyCompletion { .. })));
    }

    #[test]
    fn test_empty_completion_non_empty_content_is_never_truncation() {
        // Non-empty content is not a truncation regardless of finish_reason.
        assert!(classify("some answer", &FinishReason::Length).is_ok());
        assert!(classify("some answer", &FinishReason::Stop).is_ok());
    }

    /// D-11a deliberate flip: this pinned the opposite before Phase 50 plan 50-12 (an empty
    /// `stop` was a legitimate empty answer). Empty text is now an `EmptyCompletion` on any
    /// finish reason.
    #[test]
    fn test_empty_completion_empty_but_stop_is_empty_completion() {
        let result = classify("", &FinishReason::Stop);
        assert!(matches!(result, Err(LlmError::EmptyCompletion { .. })));
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
    /// handled by `classify_completion`; the `null` form must not blow up
    /// in DESERIALIZATION, because that happens strictly before
    /// `classify_completion` ever runs.
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
            "null content must normalize to empty string so classify_completion can \
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

        let detected = classify(&message.content, &FinishReason::Length);
        assert!(matches!(detected, Err(LlmError::EmptyCompletion { .. })));
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
        let config = DeepSeekConfig::new(secret.to_string(), "deepseek-chat".to_string());
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
        let config = DeepSeekConfig::new("sk-test-key".to_string(), "deepseek-chat".to_string());
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
        let config = DeepSeekConfig::new("test-key".to_string(), "deepseek-chat".to_string());
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
            LlmError::RateLimitExceeded { .. }
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
    async fn call_api_with_retry_surfaces_rate_limit_exceeded_on_the_first_attempt() {
        let adapter = test_adapter();
        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = Arc::clone(&calls);

        let result: Result<(), LlmError> = adapter
            .call_api_with_retry(
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
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "a 429 is surfaced on the first attempt (D-02): the Cadence decorator owns waiting, \
             so this loop must not re-send into a limit the provider just announced"
        );
    }

    // ── PACE-01: a 429 carries the provider's Retry-After, buffered and streamed ──

    #[tokio::test]
    async fn deepseek_429_carries_retry_after_from_the_header() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/chat/completions")
            .with_status(429)
            .with_header("retry-after", "7")
            .with_body(r#"{"error":{"message":"rate limit"}}"#)
            .expect(1)
            .create_async()
            .await;

        let adapter = test_adapter_at(&server.url());
        let err = adapter
            .generate(build_response_format_request(None))
            .await
            .expect_err("a 429 must surface as an error");

        mock.assert_async().await;
        assert!(matches!(err, LlmError::RateLimitExceeded { .. }));
        assert_eq!(err.retry_after(), Some(Duration::from_secs(7)));
    }

    #[tokio::test]
    async fn deepseek_stream_open_429_carries_retry_after_from_the_header() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/chat/completions")
            .with_status(429)
            .with_header("retry-after", "7")
            .with_body(r#"{"error":{"message":"rate limit"}}"#)
            .expect(1)
            .create_async()
            .await;

        let adapter = test_adapter_at(&server.url());
        let result = adapter
            .generate_stream(build_response_format_request(None))
            .await;

        mock.assert_async().await;
        match &result {
            Err(err @ LlmError::RateLimitExceeded { .. }) => {
                assert_eq!(err.retry_after(), Some(Duration::from_secs(7)));
            }
            Ok(_) => panic!("expected a rate-limit error, got Ok(<stream>)"),
            Err(other) => panic!("expected a rate-limit error, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn deepseek_429_without_retry_after_carries_no_delay() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(429)
            .with_body("{}")
            .expect(1)
            .create_async()
            .await;

        let adapter = test_adapter_at(&server.url());
        let err = adapter
            .generate(build_response_format_request(None))
            .await
            .expect_err("a 429 must surface as an error");

        assert!(matches!(err, LlmError::RateLimitExceeded { .. }));
        assert_eq!(err.retry_after(), None);
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

        let api_request = adapter.build_request(&request).unwrap().0;
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

        let api_request = adapter.build_request(&request).unwrap().0;
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

        let api_request = adapter.build_request(&request).unwrap().0;
        let body = serde_json::to_value(&api_request).unwrap();

        assert!(
            body.as_object().unwrap().get("response_format").is_none(),
            "absent response_format must not appear on the wire, got: {body:?}"
        );
    }

    /// Phase 50 (PARITY-08): a present context is appended to the query by the shared
    /// `user_turn_text` helper; an absent one leaves the query unchanged.
    #[test]
    fn deepseek_user_turn_appends_context() {
        use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};

        let adapter = test_adapter();
        let user_turn = |context: Option<&str>| -> serde_json::Value {
            let request = LlmRequest::new(
                "deepseek-chat",
                PromptItem::new(PromptType::User(UserPrompt {
                    query: "What is 2+2?".to_string(),
                    context: context.map(str::to_string),
                }))
                .unwrap(),
            );
            let body = serde_json::to_value(adapter.build_request(&request).unwrap().0).unwrap();
            body["messages"][0]["content"].clone()
        };

        assert_eq!(user_turn(None), "What is 2+2?");
        assert_eq!(
            user_turn(Some("Extra context.")),
            "What is 2+2?\n\nExtra context."
        );
    }

    // ── Phase 31 (D-13, D-14, D-15, D-20): streaming usage terminal-chunk
    //    contract, and cache/reasoning sub-count mapping on both paths ────

    fn test_adapter_at(base_url: &str) -> DeepSeekAdapter {
        let config = DeepSeekConfig::new("test-key".to_string(), "deepseek-chat".to_string())
            .with_base_url(base_url);
        DeepSeekAdapter::new(config).expect("test config must build a valid adapter")
    }

    /// Direct unit test on the shared mapping function -- no network needed
    /// (matches this file's own pre-existing style: `build_request`/
    /// `map_error` are all tested this way, never
    /// through a real HTTP round trip). Distinct, non-round figures so a
    /// swapped or dropped field cannot pass by coincidence.
    #[test]
    fn map_usage_maps_cache_hit_and_reasoning_when_the_payload_carries_them() {
        let usage = DeepSeekUsage {
            prompt_tokens: 800,
            completion_tokens: 900,
            total_tokens: Some(1700),
            prompt_cache_hit_tokens: Some(64),
            completion_tokens_details: Some(DeepSeekCompletionTokensDetails {
                reasoning_tokens: Some(320),
            }),
        };

        let (mapped, _) = map_usage(usage);

        assert_eq!(mapped.cache_read_tokens, Some(64));
        assert_eq!(mapped.reasoning_tokens, Some(320));
        assert_eq!(mapped.cache_write_tokens, None);
    }

    #[test]
    fn map_usage_leaves_optionals_none_when_the_payload_omits_them() {
        let usage = DeepSeekUsage {
            prompt_tokens: 1,
            completion_tokens: 1,
            total_tokens: Some(2),
            prompt_cache_hit_tokens: None,
            completion_tokens_details: None,
        };

        let (mapped, _) = map_usage(usage);

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

    // ── Phase 50 (PARITY-02, ADR-0061, D-01): the per-model parameter policy ──

    mod parameter_policy {
        use super::*;
        use crate::registry::UNPROFILED_MODEL_METADATA_KEY;
        use crate::sampling::{PARAMS_OMITTED_METADATA_KEY, RESPONSE_FORMAT_ADJUSTED_METADATA_KEY};
        use paladin_core::platform::container::prompt::{PromptParameters, UserPrompt};
        use serde_json::{Value, json};
        use std::sync::Mutex;

        fn request_with(parameters: PromptParameters) -> LlmRequest {
            let mut request = LlmRequest::new(
                "deepseek-chat",
                PromptItem::new(PromptType::User(UserPrompt {
                    query: "Hello".to_string(),
                    context: None,
                }))
                .expect("a user prompt must build"),
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
                "id": "cmpl-1",
                "model": "deepseek-chat",
                "choices": [{
                    "index": 0,
                    "message": {"role": "assistant", "content": "ok"},
                    "finish_reason": "stop"
                }],
                "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
            })
            .to_string()
            .into_bytes()
        }

        /// Runs `generate()` against a mock endpoint, returning the outgoing JSON body and the
        /// response metadata.
        async fn generate_capture(
            request: LlmRequest,
        ) -> (Value, std::collections::HashMap<String, String>) {
            let mut server = mockito::Server::new_async().await;
            let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
            let slot = Arc::clone(&captured);
            server
                .mock("POST", "/chat/completions")
                .with_status(200)
                .with_body_from_request(move |req| {
                    *slot.lock().unwrap() =
                        Some(req.utf8_lossy_body().unwrap_or_default().into_owned());
                    completion_json()
                })
                .create_async()
                .await;

            let response = test_adapter_at(&server.url())
                .generate(request)
                .await
                .expect("the mock answers a well-formed completion");
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

        async fn stream_capture(request: LlmRequest) -> Value {
            let mut server = mockito::Server::new_async().await;
            let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
            let slot = Arc::clone(&captured);
            server
                .mock("POST", "/chat/completions")
                .with_status(200)
                .with_header("content-type", "text/event-stream")
                .with_body_from_request(move |req| {
                    *slot.lock().unwrap() =
                        Some(req.utf8_lossy_body().unwrap_or_default().into_owned());
                    b"data: [DONE]\n\n".to_vec()
                })
                .create_async()
                .await;

            let stream = test_adapter_at(&server.url())
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

        #[tokio::test]
        async fn deepseek_stop_sequences_keep_declared_order() {
            let mut parameters = unset();
            // A duplicate, deliberately: order and duplicates are the caller's.
            parameters.stop_sequences = Some(vec![
                "END".to_string(),
                "STOP".to_string(),
                "END".to_string(),
            ]);
            let (body, metadata) = generate_capture(request_with(parameters)).await;

            assert_eq!(body["stop"], json!(["END", "STOP", "END"]));
            assert!(!metadata.contains_key(PARAMS_OMITTED_METADATA_KEY));
        }

        #[tokio::test]
        async fn deepseek_stream_stop_sequences_keep_declared_order() {
            let mut parameters = unset();
            parameters.stop_sequences = Some(vec!["END".to_string(), "STOP".to_string()]);
            let body = stream_capture(request_with(parameters)).await;

            assert_eq!(body["stop"], json!(["END", "STOP"]));
            assert_eq!(body["stream"], true);
        }

        #[tokio::test]
        async fn deepseek_unset_sampling_values_are_absent() {
            let (body, metadata) = generate_capture(request_with(unset())).await;
            let obj = body.as_object().expect("body must be a JSON object");

            for key in [
                "temperature",
                "top_p",
                "frequency_penalty",
                "presence_penalty",
                "stop",
                "max_tokens",
            ] {
                assert!(!obj.contains_key(key), "{key} was never set: {obj:?}");
            }
            assert!(!metadata.contains_key(PARAMS_OMITTED_METADATA_KEY));
        }

        #[tokio::test]
        async fn deepseek_out_of_range_temperature_is_omitted_and_recorded() {
            let mut at_edge = unset();
            at_edge.temperature = Some(2.0);
            let (body, metadata) = generate_capture(request_with(at_edge)).await;
            assert_eq!(body["temperature"], 2.0, "the declared maximum is admitted");
            assert!(!metadata.contains_key(PARAMS_OMITTED_METADATA_KEY));

            let mut past_edge = unset();
            past_edge.temperature = Some(2.1);
            let (body, metadata) = generate_capture(request_with(past_edge)).await;
            assert!(
                body.get("temperature").is_none(),
                "2.1 is omitted, never clamped to 2.0: {body}"
            );
            assert_eq!(
                metadata
                    .get(PARAMS_OMITTED_METADATA_KEY)
                    .map(String::as_str),
                Some("temperature")
            );
        }

        #[tokio::test]
        async fn deepseek_schema_request_is_recorded_as_downgraded() {
            let request = request_with(unset()).with_response_format(ResponseFormat::JsonSchema {
                name: "answer".to_string(),
                schema: json!({"type": "object"}),
                strict: true,
            });
            let (body, metadata) = generate_capture(request).await;

            assert_eq!(body["response_format"], json!({"type": "json_object"}));
            assert_eq!(
                metadata
                    .get(RESPONSE_FORMAT_ADJUSTED_METADATA_KEY)
                    .map(String::as_str),
                Some("json_object"),
                "the schema-to-object downgrade is now visible on the response"
            );

            let request = request_with(unset()).with_response_format(ResponseFormat::JsonObject);
            let (_, metadata) = generate_capture(request).await;
            assert!(
                !metadata.contains_key(RESPONSE_FORMAT_ADJUSTED_METADATA_KEY),
                "a plain object request is sent as asked, so nothing is recorded"
            );
        }

        /// Phase 50, owner reading R-1 (option-b): a streamed call stamps the visibility keys
        /// the buffered path would, on the terminal chunk only.
        #[tokio::test]
        async fn deepseek_stream_terminal_chunk_carries_metadata() {
            let mut server = mockito::Server::new_async().await;
            server
                .mock("POST", "/chat/completions")
                .with_status(200)
                .with_header("content-type", "text/event-stream")
                .with_body(
                    "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"Hi\"},\"finish_reason\":null}]}\n\n\
                     data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"\"},\"finish_reason\":\"stop\"}]}\n\n\
                     data: [DONE]\n\n",
                )
                .create_async()
                .await;

            let mut parameters = unset();
            parameters.temperature = Some(2.5); // above the profile's (0.0, 2.0)
            let stream = test_adapter_at(&server.url())
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
                Some("deepseek-chat")
            );
            for content in &chunks[..chunks.len() - 1] {
                assert!(content.metadata.is_empty(), "content chunk: {content:?}");
            }
        }

        #[tokio::test]
        async fn deepseek_unprofiled_model_is_stamped() {
            let (_, metadata) = generate_capture(request_with(unset())).await;
            assert_eq!(
                metadata
                    .get(UNPROFILED_MODEL_METADATA_KEY)
                    .map(String::as_str),
                Some("deepseek-chat"),
                "`deepseek-chat` has no seeded entry (an undocumented alias), so it runs on the adapter-wide fallback"
            );
        }

        /// SURFACE-06 (D-14): `deepseek-v4-pro` is in the dated table, so a call to it carries no
        /// `paladin.unprofiled_model` stamp (the converse of `deepseek_unprofiled_model_is_stamped`).
        #[tokio::test]
        async fn deepseek_v4_pro_is_not_stamped_unprofiled() {
            let mut request = request_with(unset());
            request.model = "deepseek-v4-pro".to_string();
            let (_, metadata) = generate_capture(request).await;
            assert!(
                !metadata.contains_key(UNPROFILED_MODEL_METADATA_KEY),
                "deepseek-v4-pro is seeded, so no unprofiled stamp: {metadata:?}"
            );
        }

        #[test]
        fn deepseek_model_profile_is_always_some_and_matches_what_the_wire_sends() {
            let profile = test_adapter()
                .model_profile("any-model")
                .expect("deepseek profiles every model");
            assert_eq!(profile.temperature, ParamSupport::Range(0.0, 2.0));
            assert_eq!(profile.top_p, ParamSupport::Range(0.0, 1.0));
            assert_eq!(profile.frequency_penalty, ParamSupport::Range(-2.0, 2.0));
            assert_eq!(profile.presence_penalty, ParamSupport::Range(-2.0, 2.0));
            assert_eq!(profile.stop_sequences, StopSupport::Supported { max: None });
            assert!(profile.json_object);
            assert!(!profile.json_schema);
        }

        #[test]
        fn deepseek_max_tokens_above_a_profiled_maximum_is_refused_by_name() {
            use crate::registry::{ModelOverride, ModelRegistry};
            use std::collections::BTreeMap;

            let mut adapter = test_adapter();
            let over = ModelOverride {
                max_output_tokens: Some(1000),
                ..ModelOverride::default()
            };
            adapter.model_registry =
                ModelRegistry::new(DEEPSEEK_PROVIDER, &[], fallback_profile(None))
                    .with_overrides(BTreeMap::from([("deepseek-chat".to_string(), over)]))
                    .expect("a plain max_output_tokens override is valid");

            let mut parameters = unset();
            parameters.max_tokens = Some(4000);
            match adapter.build_request(&request_with(parameters)) {
                Err(LlmError::InvalidPrompt(message)) => {
                    assert!(message.contains("deepseek"), "{message}");
                    assert!(message.contains("deepseek-chat"), "{message}");
                    assert!(message.contains("max_tokens"), "{message}");
                }
                other => panic!("expected InvalidPrompt, got {other:?}"),
            }
        }
    }

    // ── Phase 51 (REASON-02, REASON-03, D-00f): an explicit effort reaches the wire or is refused ──

    mod reasoning_effort {
        use super::*;
        use crate::reasoning::{
            REASONING_EFFECTIVE_METADATA_KEY, REASONING_REQUESTED_METADATA_KEY,
        };
        use paladin_core::platform::container::prompt::UserPrompt;
        use paladin_core::platform::container::transience::Transience;
        use paladin_ports::output::llm_port::ReasoningEffort::{High, Low, Max};
        use paladin_ports::output::llm_port::{ReasoningEffort, ReasoningFallback};
        use paladin_ports::output::model_profile::{EffortLevels, EffortSupport};
        use serde_json::json;
        use std::sync::Mutex;

        const PRO: &str = "deepseek-v4-pro";

        fn request_for(model: &str) -> LlmRequest {
            LlmRequest::new(
                model,
                PromptItem::new(PromptType::User(UserPrompt {
                    query: "Hello".to_string(),
                    context: None,
                }))
                .expect("a user prompt must build"),
            )
        }

        fn ok_body() -> Vec<u8> {
            json!({
                "id": "cmpl-1",
                "model": PRO,
                "choices": [{
                    "index": 0,
                    "message": {"role": "assistant", "content": "ok"},
                    "finish_reason": "stop"
                }],
                "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
            })
            .to_string()
            .into_bytes()
        }

        /// The operator's statement for the pro model used by most cases: three levels, off,
        /// auto, and the explicit thinking flag sent beside a level.
        fn pro_levels() -> EffortSupport {
            EffortSupport::Supported(
                EffortLevels::new([Low, High, Max])
                    .with_off(true)
                    .with_auto(true)
                    .with_enable_thinking_with_level(true),
            )
        }

        /// Send `request` through an adapter whose `llm.deepseek.models` carries `overrides`,
        /// against a mock; returns the raw request body (`None` when no HTTP call was made) and
        /// the result.
        async fn send_raw(
            overrides: Vec<(&str, EffortSupport)>,
            request: LlmRequest,
        ) -> (Option<String>, Result<LlmResponse, LlmError>) {
            let mut server = mockito::Server::new_async().await;
            let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
            let slot = Arc::clone(&captured);
            server
                .mock("POST", "/chat/completions")
                .with_status(200)
                .with_body_from_request(move |req| {
                    *slot.lock().unwrap() =
                        Some(req.utf8_lossy_body().unwrap_or_default().into_owned());
                    ok_body()
                })
                .create_async()
                .await;

            let mut map = BTreeMap::new();
            for (model, effort) in overrides {
                map.insert(
                    model.to_string(),
                    ModelOverride {
                        effort: Some(effort),
                        ..ModelOverride::default()
                    },
                );
            }
            let config = DeepSeekConfig::new("test-key".to_string(), "deepseek-flash".to_string())
                .with_base_url(server.url())
                .with_model_overrides(map);
            let adapter = DeepSeekAdapter::new(config).expect("a valid override builds");
            let result = adapter.generate(request).await;
            let body = captured.lock().unwrap().take();
            (body, result)
        }

        #[tokio::test]
        async fn a_listed_level_adds_exactly_the_effort_and_thinking_fields() {
            let (unset, _) = send_raw(vec![(PRO, pro_levels())], request_for(PRO)).await;
            let (set, result) = send_raw(
                vec![(PRO, pro_levels())],
                request_for(PRO).with_reasoning(High),
            )
            .await;
            result.expect("a listed level is accepted");

            let unset = unset.expect("the unset request reached the wire");
            let set = set.expect("the effort request reached the wire");
            // The effort body is the unset body plus two trailing fields, in this order: nothing
            // else moved, so the existing field order is intact (D-00i).
            let prefix = unset.strip_suffix('}').expect("a JSON object");
            assert_eq!(
                set,
                format!(
                    "{prefix},\"reasoning_effort\":\"high\",\"thinking\":{{\"type\":\"enabled\"}}}}"
                )
            );
        }

        #[tokio::test]
        async fn a_level_without_the_thinking_flag_sends_only_the_effort() {
            let plain = EffortSupport::Supported(EffortLevels::new([Low, High]));
            let (body, result) =
                send_raw(vec![(PRO, plain)], request_for(PRO).with_reasoning(Low)).await;
            result.expect("accepted");
            let body: serde_json::Value = serde_json::from_str(&body.unwrap()).unwrap();
            assert_eq!(body["reasoning_effort"], "low");
            assert!(body.get("thinking").is_none(), "{body}");
        }

        #[tokio::test]
        async fn the_response_records_requested_and_effective_effort() {
            let (_, result) = send_raw(
                vec![(PRO, pro_levels())],
                request_for(PRO).with_reasoning(High),
            )
            .await;
            let metadata = result.expect("accepted").metadata;
            assert_eq!(
                metadata
                    .get(REASONING_REQUESTED_METADATA_KEY)
                    .map(String::as_str),
                Some("high")
            );
            assert_eq!(
                metadata
                    .get(REASONING_EFFECTIVE_METADATA_KEY)
                    .map(String::as_str),
                Some("high")
            );
        }

        #[tokio::test]
        async fn an_unset_effort_stamps_nothing_and_sends_nothing() {
            let (body, result) = send_raw(vec![(PRO, pro_levels())], request_for(PRO)).await;
            let metadata = result.expect("accepted").metadata;
            assert!(!metadata.contains_key(REASONING_REQUESTED_METADATA_KEY));
            assert!(!metadata.contains_key(REASONING_EFFECTIVE_METADATA_KEY));
            let body = body.unwrap();
            assert!(
                !body.contains("reasoning_effort") && !body.contains("thinking"),
                "{body}"
            );
        }

        #[tokio::test]
        async fn an_effort_on_an_unmeasured_model_is_refused_before_any_http_call() {
            let (body, result) = send_raw(
                Vec::new(),
                request_for("deepseek-flash").with_reasoning(High),
            )
            .await;
            assert!(
                body.is_none(),
                "no request may reach the endpoint: {body:?}"
            );
            let error = result.expect_err("an unmeasured model refuses an explicit effort");
            assert_eq!(error.transience(), Transience::Permanent);
            match error {
                LlmError::UnsupportedParameter {
                    reason,
                    provider,
                    model,
                    ..
                } => {
                    assert_eq!(provider, "deepseek");
                    assert_eq!(model, "deepseek-flash");
                    assert!(
                        reason.contains("llm.deepseek.models.deepseek-flash.effort"),
                        "{reason}"
                    );
                }
                other => panic!("expected UnsupportedParameter, got {other:?}"),
            }
        }

        fn adapter_over(base_url: &str, model: &str, effort: EffortSupport) -> DeepSeekAdapter {
            let over = ModelOverride {
                effort: Some(effort),
                ..ModelOverride::default()
            };
            let config = DeepSeekConfig::new("test-key".to_string(), "deepseek-flash".to_string())
                .with_base_url(base_url)
                .with_model_overrides(BTreeMap::from([(model.to_string(), over)]));
            DeepSeekAdapter::new(config).expect("a valid override builds")
        }

        fn off_auto_coerced() -> EffortSupport {
            EffortSupport::Supported(
                EffortLevels::new([Low, High, Max])
                    .with_off(true)
                    .with_auto(true)
                    .with_coerced([
                        ReasoningEffort::Minimal,
                        ReasoningEffort::Medium,
                        ReasoningEffort::XHigh,
                    ]),
            )
        }

        #[tokio::test]
        async fn off_sends_thinking_disabled_and_no_effort() {
            let (body, result) = send_raw(
                vec![(PRO, off_auto_coerced())],
                request_for(PRO).with_reasoning(ReasoningEffort::Off),
            )
            .await;
            let metadata = result.expect("off is accepted").metadata;
            let body: serde_json::Value = serde_json::from_str(&body.unwrap()).unwrap();
            assert_eq!(body["thinking"], json!({"type": "disabled"}));
            assert!(body.get("reasoning_effort").is_none(), "{body}");
            assert_eq!(metadata[REASONING_REQUESTED_METADATA_KEY], "off");
            assert_eq!(metadata[REASONING_EFFECTIVE_METADATA_KEY], "off");
        }

        #[tokio::test]
        async fn auto_sends_thinking_enabled_and_no_effort() {
            let (body, result) = send_raw(
                vec![(PRO, off_auto_coerced())],
                request_for(PRO).with_reasoning(ReasoningEffort::Auto),
            )
            .await;
            let metadata = result.expect("auto is accepted").metadata;
            let body: serde_json::Value = serde_json::from_str(&body.unwrap()).unwrap();
            assert_eq!(body["thinking"], json!({"type": "enabled"}));
            assert!(body.get("reasoning_effort").is_none(), "{body}");
            assert_eq!(metadata[REASONING_EFFECTIVE_METADATA_KEY], "auto");
        }

        #[tokio::test]
        async fn a_coerced_level_is_refused_even_with_a_fallback() {
            let (body, result) = send_raw(
                vec![(PRO, off_auto_coerced())],
                request_for(PRO)
                    .with_reasoning(ReasoningEffort::Medium)
                    .with_reasoning_fallback(ReasoningFallback::NearestLower),
            )
            .await;
            assert!(body.is_none(), "no HTTP call: {body:?}");
            assert!(
                matches!(result, Err(LlmError::UnsupportedParameter { .. })),
                "{result:?}"
            );
        }

        #[tokio::test]
        async fn a_substituting_fallback_sends_the_nearest_level_and_records_both() {
            let mut server = mockito::Server::new_async().await;
            let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
            let slot = Arc::clone(&captured);
            server
                .mock("POST", "/chat/completions")
                .with_status(200)
                .with_body_from_request(move |req| {
                    *slot.lock().unwrap() =
                        Some(req.utf8_lossy_body().unwrap_or_default().into_owned());
                    ok_body()
                })
                .expect(2)
                .create_async()
                .await;
            let adapter = adapter_over(
                &server.url(),
                PRO,
                EffortSupport::Supported(EffortLevels::new([Low, High])),
            );
            let request = || {
                request_for(PRO)
                    .with_reasoning(ReasoningEffort::XHigh)
                    .with_reasoning_fallback(ReasoningFallback::NearestLower)
            };

            let first = adapter.generate(request()).await.expect("substituted");
            let body: serde_json::Value =
                serde_json::from_str(&captured.lock().unwrap().take().unwrap()).unwrap();
            assert_eq!(body["reasoning_effort"], "high");
            assert_eq!(first.metadata[REASONING_REQUESTED_METADATA_KEY], "xhigh");
            assert_eq!(first.metadata[REASONING_EFFECTIVE_METADATA_KEY], "high");

            // The first request spent the warning; an identical one stays quiet but is still
            // stamped.
            let second = adapter
                .generate(request())
                .await
                .expect("substituted again");
            assert_eq!(second.metadata[REASONING_REQUESTED_METADATA_KEY], "xhigh");
            assert_eq!(second.metadata[REASONING_EFFECTIVE_METADATA_KEY], "high");
            assert!(!adapter.substitutions.record(
                DEEPSEEK_PROVIDER,
                PRO,
                ReasoningEffort::XHigh,
                High
            ));
        }

        #[tokio::test]
        async fn without_a_fallback_the_missing_level_is_refused() {
            let (body, result) = send_raw(
                vec![(
                    PRO,
                    EffortSupport::Supported(EffortLevels::new([Low, High])),
                )],
                request_for(PRO).with_reasoning(ReasoningEffort::XHigh),
            )
            .await;
            assert!(body.is_none());
            assert!(result.is_err());
        }

        #[tokio::test]
        async fn a_budget_is_masked_away_and_refused() {
            let with_budget = EffortSupport::Supported(EffortLevels::new([High]).with_budget(
                paladin_ports::output::model_profile::BudgetBounds::new(1, 4096),
            ));
            let (body, result) = send_raw(
                vec![(PRO, with_budget)],
                request_for(PRO).with_reasoning(ReasoningEffort::Budget(1000)),
            )
            .await;
            assert!(body.is_none(), "{body:?}");
            match result {
                Err(LlmError::UnsupportedParameter { reason, value, .. }) => {
                    assert_eq!(value, "budget:1000");
                    assert!(reason.contains("budget"), "{reason}");
                }
                other => panic!("expected UnsupportedParameter, got {other:?}"),
            }
        }

        async fn stream_body_and_chunks(
            effort: EffortSupport,
            request: LlmRequest,
        ) -> (Option<String>, Result<Vec<StreamingResponse>, LlmError>) {
            let mut server = mockito::Server::new_async().await;
            let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
            let slot = Arc::clone(&captured);
            server
                .mock("POST", "/chat/completions")
                .with_status(200)
                .with_header("content-type", "text/event-stream")
                .with_body_from_request(move |req| {
                    *slot.lock().unwrap() =
                        Some(req.utf8_lossy_body().unwrap_or_default().into_owned());
                    concat!(
                        "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"Hi\"},\"finish_reason\":null}]}\n\n",
                        "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"\"},\"finish_reason\":\"stop\"}]}\n\n",
                        "data: [DONE]\n\n",
                    )
                    .as_bytes()
                    .to_vec()
                })
                .create_async()
                .await;
            let adapter = adapter_over(&server.url(), PRO, effort);
            let outcome = match adapter.generate_stream(request).await {
                Err(error) => Err(error),
                Ok(stream) => {
                    let mut stream = Box::into_pin(stream);
                    let mut chunks = Vec::new();
                    while let Some(item) = stream.next().await {
                        chunks.push(item.expect("every scripted item is Ok"));
                    }
                    Ok(chunks)
                }
            };
            let body = captured.lock().unwrap().take();
            (body, outcome)
        }

        #[tokio::test]
        async fn the_stream_sends_the_same_fields_and_stamps_only_the_terminal_chunk() {
            let (body, chunks) =
                stream_body_and_chunks(pro_levels(), request_for(PRO).with_reasoning(High)).await;
            let body: serde_json::Value = serde_json::from_str(&body.unwrap()).unwrap();
            assert_eq!(body["reasoning_effort"], "high");
            assert_eq!(body["thinking"], json!({"type": "enabled"}));
            assert_eq!(body["stream"], true);

            let chunks = chunks.expect("the stream opens");
            let (terminal, earlier) = chunks.split_last().expect("a terminal chunk");
            assert_eq!(terminal.metadata[REASONING_REQUESTED_METADATA_KEY], "high");
            assert_eq!(terminal.metadata[REASONING_EFFECTIVE_METADATA_KEY], "high");
            for chunk in earlier {
                assert!(chunk.metadata.is_empty(), "earlier chunk: {chunk:?}");
            }
        }

        #[tokio::test]
        async fn a_refused_effort_fails_the_stream_before_any_http_call() {
            let (body, outcome) = stream_body_and_chunks(
                EffortSupport::Supported(EffortLevels::new([Low])),
                request_for(PRO).with_reasoning(Max),
            )
            .await;
            assert!(body.is_none(), "{body:?}");
            assert!(matches!(
                outcome,
                Err(LlmError::UnsupportedParameter { .. })
            ));
        }

        /// T-51-12: the refusal names provider, model and effort; a prompt never reaches it.
        #[tokio::test]
        async fn a_refusal_carries_no_prompt_content() {
            let request = LlmRequest::new(
                "deepseek-flash",
                PromptItem::new(PromptType::User(UserPrompt {
                    query: "PROMPT-MARKER-7f3a".to_string(),
                    context: Some("CONTEXT-MARKER-91bc".to_string()),
                }))
                .expect("a user prompt must build"),
            )
            .with_reasoning(High);
            let (_, result) = send_raw(Vec::new(), request).await;
            let error = result.expect_err("refused");
            let text = format!("{error} | {error:?}");
            assert!(!text.contains("PROMPT-MARKER-7f3a"), "{text}");
            assert!(!text.contains("CONTEXT-MARKER-91bc"), "{text}");
            assert!(!text.contains("test-key"), "{text}");
        }

        #[test]
        fn a_budget_cannot_reach_the_wire_so_the_profile_does_not_offer_one() {
            // Task 1 pin of the masking seam: `model_profile` is what `PaladinBuilder` reads.
            let adapter = {
                let over = ModelOverride {
                    effort: Some(EffortSupport::Supported(
                        EffortLevels::new([High]).with_budget(
                            paladin_ports::output::model_profile::BudgetBounds::new(1, 4096),
                        ),
                    )),
                    ..ModelOverride::default()
                };
                let config = DeepSeekConfig::new("k".to_string(), "deepseek-flash".to_string())
                    .with_model_overrides(BTreeMap::from([(PRO.to_string(), over)]));
                DeepSeekAdapter::new(config).expect("builds")
            };
            let EffortSupport::Supported(levels) = adapter.model_profile(PRO).unwrap().effort
            else {
                panic!("a level-carrying wire keeps the override Supported");
            };
            assert_eq!(levels.budget, None);
            assert_eq!(levels.levels, [High]);
        }
    }

    // ── Shared conformance suite (D-19, plan 31-04) ──
    //
    /// PARITY-05: reasoning text and the empty-output rule on both paths (D-11, D-11a).
    mod response_guard_wiring {
        use super::*;
        use futures::StreamExt;
        use serde_json::{Value, json};

        fn body(message: Value, finish_reason: &str) -> String {
            json!({
                "id": "cmpl-1",
                "model": "deepseek-chat",
                "choices": [{"index": 0, "message": message, "finish_reason": finish_reason}],
                "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
            })
            .to_string()
        }

        async fn generate_from(body: String) -> Result<LlmResponse, LlmError> {
            let mut server = mockito::Server::new_async().await;
            server
                .mock("POST", "/chat/completions")
                .with_status(200)
                .with_body(body)
                .create_async()
                .await;
            test_adapter_at(&server.url())
                .generate(build_response_format_request(None))
                .await
        }

        async fn stream_from(sse: &str) -> Vec<Result<StreamingResponse, LlmError>> {
            let mut server = mockito::Server::new_async().await;
            server
                .mock("POST", "/chat/completions")
                .with_status(200)
                .with_body(sse)
                .create_async()
                .await;
            let stream = test_adapter_at(&server.url())
                .generate_stream(build_response_format_request(None))
                .await
                .unwrap();
            let mut stream = Box::into_pin(stream);
            let mut items = Vec::new();
            while let Some(item) = stream.next().await {
                items.push(item);
            }
            items
        }

        #[tokio::test]
        async fn deepseek_buffered_reasoning_text_is_surfaced() {
            let response = generate_from(body(
                json!({"role": "assistant", "content": "ok", "reasoning_content": "think"}),
                "stop",
            ))
            .await
            .unwrap();
            assert_eq!(response.reasoning.as_deref(), Some("think"));

            let blank = generate_from(body(
                json!({"role": "assistant", "content": "ok", "reasoning_content": "  "}),
                "stop",
            ))
            .await
            .unwrap();
            assert_eq!(blank.reasoning, None);
        }

        /// G-50-5a deliberate flip: the buffered path used to append the usage counts to the
        /// `detail` text (`annotate_with_usage`); the counts are typed on the error now
        /// ([`LlmError::billed_usage`]), so the text no longer embeds them (D-09).
        #[tokio::test]
        async fn deepseek_empty_stop_completion_is_empty_completion_with_usage() {
            let result =
                generate_from(body(json!({"role": "assistant", "content": ""}), "stop")).await;
            match result {
                Err(err @ LlmError::EmptyCompletion { .. }) => {
                    let LlmError::EmptyCompletion { detail: msg, .. } = &err else {
                        unreachable!("matched as EmptyCompletion above");
                    };
                    assert!(msg.contains("finish_reason=stop"), "{msg}");
                    assert!(
                        !msg.contains("provider-reported usage"),
                        "the counts are typed now, never embedded in the detail: {msg}"
                    );
                    assert_eq!(
                        err.billed_usage().map(|bill| bill.usage.total_tokens),
                        Some(2)
                    );
                }
                other => panic!("expected EmptyCompletion, got {other:?}"),
            }
        }

        /// G-50-5a: a 63_000-token reasoning truncation DeepSeek billed (`finish_reason:
        /// length`, empty content) carries its usage on the error as a typed bill on the served
        /// model, and its `detail` names no token counts. A body that omits `total_tokens`
        /// still bills `prompt + completion` (D-02), never a fabricated zero.
        #[tokio::test]
        async fn deepseek_empty_completion_carries_its_billed_usage() {
            let truncated = |usage: Value| {
                json!({
                    "id": "cmpl-1",
                    "model": "deepseek-served-test",
                    "choices": [{
                        "index": 0,
                        "message": {"role": "assistant", "content": ""},
                        "finish_reason": "length"
                    }],
                    "usage": usage
                })
                .to_string()
            };

            let err = generate_from(truncated(json!({
                "prompt_tokens": 31_000,
                "completion_tokens": 32_000,
                "total_tokens": 63_000,
                "completion_tokens_details": {"reasoning_tokens": 31_500}
            })))
            .await
            .expect_err("an empty length completion is an error");
            let LlmError::EmptyCompletion { detail, .. } = &err else {
                panic!("expected EmptyCompletion, got {err:?}");
            };
            for count in [
                "31000",
                "32000",
                "63000",
                "31_000",
                "provider-reported usage",
            ] {
                assert!(
                    !detail.contains(count),
                    "the detail must not embed the token counts ({count}): {detail}"
                );
            }
            let billed = err
                .billed_usage()
                .expect("a billed empty completion carries its usage");
            assert_eq!(billed.model, "deepseek-served-test");
            assert_eq!(billed.usage.prompt_tokens, 31_000);
            assert_eq!(billed.usage.completion_tokens, 32_000);
            assert_eq!(billed.usage.reasoning_tokens, Some(31_500));
            assert_eq!(billed.usage.total_tokens, 63_000);
            assert_eq!(billed.cost, None, "pricing is the decorator's job");

            let err = generate_from(truncated(json!({
                "prompt_tokens": 31_000,
                "completion_tokens": 32_000
            })))
            .await
            .expect_err("an empty length completion is an error");
            assert_eq!(
                err.billed_usage().map(|bill| bill.usage.total_tokens),
                Some(63_000),
                "an absent vendor total bills prompt + completion"
            );
        }

        /// WR-04: a no-choices body DeepSeek billed is an `EmptyCompletion` carrying the bill
        /// on the served model (as the compat engine and Gemini do), not a `ProcessingError`
        /// that drops the usage and reads as `Transience::Unknown` to the retry loop.
        #[tokio::test]
        async fn deepseek_no_choices_body_is_a_billed_empty_completion() {
            let err = generate_from(
                json!({
                    "id": "cmpl-1",
                    "model": "deepseek-served-test",
                    "choices": [],
                    "usage": {"prompt_tokens": 40, "completion_tokens": 0, "total_tokens": 40}
                })
                .to_string(),
            )
            .await
            .expect_err("a body with no choices is an error");
            let LlmError::EmptyCompletion { detail, .. } = &err else {
                panic!("expected EmptyCompletion, got {err:?}");
            };
            assert!(detail.contains("no choices"), "{detail}");
            let billed = err
                .billed_usage()
                .expect("a billed no-choices body carries its usage");
            assert_eq!(billed.model, "deepseek-served-test");
            assert_eq!(billed.usage.total_tokens, 40);
            assert_eq!(billed.cost, None, "pricing is the decorator's job");
        }

        #[tokio::test]
        async fn deepseek_stream_reasoning_keeps_wire_order() {
            let sse = concat!(
                "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"reasoning_content\":\"a \"},\"finish_reason\":null}]}\n\n",
                "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"Hi\"},\"finish_reason\":null}]}\n\n",
                "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"reasoning_content\":\"b\"},\"finish_reason\":\"stop\"}]}\n\n",
                "data: [DONE]\n\n",
            );
            let chunks: Vec<StreamingResponse> = stream_from(sse)
                .await
                .into_iter()
                .map(|item| item.unwrap())
                .collect();
            let reasoning: Vec<Option<&str>> =
                chunks.iter().map(|c| c.reasoning.as_deref()).collect();
            assert_eq!(reasoning, vec![Some("a "), None, Some("b"), None]);
            let content: Vec<&str> = chunks.iter().map(|c| c.delta.as_str()).collect();
            assert_eq!(content, vec!["", "Hi", "", ""]);
        }

        #[tokio::test]
        async fn deepseek_empty_stream_is_empty_completion() {
            let sse = concat!(
                "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"reasoning_content\":\"hmm\"},\"finish_reason\":null}]}\n\n",
                "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\" \"},\"finish_reason\":\"stop\"}]}\n\n",
                "data: [DONE]\n\n",
            );
            let items = stream_from(sse).await;
            let (last, yielded) = items.split_last().unwrap();
            assert!(
                matches!(last, Err(LlmError::EmptyCompletion { .. })),
                "got {last:?}"
            );
            assert_eq!(
                yielded[0].as_ref().unwrap().reasoning.as_deref(),
                Some("hmm"),
                "reasoning already yielded stays yielded"
            );
            assert!(
                items
                    .iter()
                    .all(|item| !matches!(item, Ok(chunk) if chunk.finish_reason.is_some())),
                "no Ok terminal chunk is emitted"
            );
        }

        #[tokio::test]
        async fn deepseek_content_filter_stream_with_no_text_stays_ok() {
            let sse = concat!(
                "data: {\"id\":\"1\",\"choices\":[{\"delta\":{},\"finish_reason\":\"content_filter\"}]}\n\n",
                "data: [DONE]\n\n",
            );
            let items = stream_from(sse).await;
            let terminal = items.last().unwrap().as_ref().expect("no error item");
            assert!(matches!(
                terminal.finish_reason,
                Some(FinishReason::ContentFilter)
            ));
        }
    }

    // Nested in its own module (rather than inline in `mod tests`) so every generated test's
    // full path contains "conformance" -- `cargo test --lib conformance` (the plan's own
    // acceptance criterion) selects it by that substring.
    /// Plan 50-23 (G-50-5a): DeepSeek's own `total_tokens` is read and reconciled against the
    /// mapped `prompt + completion` on both paths, never copied (D-02).
    mod usage_reconciliation {
        use super::*;
        use crate::usage_reconcile::USAGE_UNRECONCILED_METADATA_KEY;
        use futures::StreamExt;
        use serde_json::{Value, json};

        async fn generate_with_usage(usage: Value) -> LlmResponse {
            let mut server = mockito::Server::new_async().await;
            server
                .mock("POST", "/chat/completions")
                .with_status(200)
                .with_body(
                    json!({
                        "id": "cmpl-1",
                        "model": "deepseek-chat",
                        "choices": [{
                            "index": 0,
                            "message": {"role": "assistant", "content": "ok"},
                            "finish_reason": "stop"
                        }],
                        "usage": usage
                    })
                    .to_string(),
                )
                .create_async()
                .await;
            test_adapter_at(&server.url())
                .generate(build_response_format_request(None))
                .await
                .expect("a well-formed body must parse")
        }

        /// G-50-5a (plan 50-24): the live-measured deepseek-flash usage object
        /// (`50-USAGE-PROBE.md`, 2026-10-18: prompt 50, completion 32 of which reasoning 30,
        /// total 82) is read inside completion and reconciles with DeepSeek's own total. Read
        /// beside completion it would map to 112 and disagree.
        #[tokio::test]
        async fn deepseek_measured_usage_is_read_inside_completion() {
            let response = generate_with_usage(json!({
                "prompt_tokens": 50,
                "completion_tokens": 32,
                "total_tokens": 82,
                "completion_tokens_details": {"reasoning_tokens": 30}
            }))
            .await;
            let usage = &response.usage;
            assert_eq!(usage.prompt_tokens, 50);
            assert_eq!(usage.completion_tokens, 32, "{usage:?}");
            assert_eq!(usage.reasoning_tokens, Some(30));
            assert_eq!(usage.total_tokens, 82);
            assert!(
                !response
                    .metadata
                    .contains_key(USAGE_UNRECONCILED_METADATA_KEY),
                "{:?}",
                response.metadata
            );
        }

        #[tokio::test]
        async fn deepseek_agreeing_total_records_no_finding() {
            let response = generate_with_usage(
                json!({"prompt_tokens": 800, "completion_tokens": 900, "total_tokens": 1700}),
            )
            .await;
            assert_eq!(response.usage.total_tokens, 1700);
            assert!(
                !response
                    .metadata
                    .contains_key(USAGE_UNRECONCILED_METADATA_KEY)
            );
        }

        #[tokio::test]
        async fn deepseek_disagreeing_total_is_recorded_and_not_copied() {
            let response = generate_with_usage(
                json!({"prompt_tokens": 800, "completion_tokens": 900, "total_tokens": 1707}),
            )
            .await;
            assert_eq!(response.usage, TokenUsage::new(800, 900));
            assert_eq!(
                response
                    .metadata
                    .get(USAGE_UNRECONCILED_METADATA_KEY)
                    .map(String::as_str),
                Some("total:vendor=1707,mapped=1700")
            );
        }

        #[tokio::test]
        async fn deepseek_usage_without_a_total_parses_with_no_finding() {
            let response =
                generate_with_usage(json!({"prompt_tokens": 800, "completion_tokens": 900})).await;
            assert_eq!(response.usage, TokenUsage::new(800, 900));
            assert!(
                !response
                    .metadata
                    .contains_key(USAGE_UNRECONCILED_METADATA_KEY)
            );
        }

        #[tokio::test]
        async fn deepseek_stream_terminal_records_a_disagreeing_total() {
            let sse = concat!(
                "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"Hi\"},\"finish_reason\":\"stop\"}]}\n\n",
                "data: {\"id\":\"1\",\"choices\":[],\"usage\":{\"prompt_tokens\":5,\"completion_tokens\":3,\"total_tokens\":15}}\n\n",
                "data: [DONE]\n\n",
            );
            let mut server = mockito::Server::new_async().await;
            server
                .mock("POST", "/chat/completions")
                .with_status(200)
                .with_body(sse)
                .create_async()
                .await;
            let stream = test_adapter_at(&server.url())
                .generate_stream(build_response_format_request(None))
                .await
                .unwrap();
            let mut stream = Box::into_pin(stream);
            let mut chunks = Vec::new();
            while let Some(item) = stream.next().await {
                chunks.push(item.expect("every item must be Ok"));
            }

            let (terminal, earlier) = chunks.split_last().expect("a terminal chunk");
            assert_eq!(terminal.usage, Some(TokenUsage::new(5, 3)));
            assert_eq!(
                terminal
                    .metadata
                    .get(USAGE_UNRECONCILED_METADATA_KEY)
                    .map(String::as_str),
                Some("total:vendor=15,mapped=8")
            );
            assert!(
                earlier
                    .iter()
                    .all(|chunk| !chunk.metadata.contains_key(USAGE_UNRECONCILED_METADATA_KEY))
            );
        }
    }

    mod conformance_suite {
        use super::*;
        use serde_json::json;

        struct DeepSeekFixture;

        impl crate::conformance::ConformanceFixture for DeepSeekFixture {
            const WIRE: crate::conformance::Wire = crate::conformance::Wire::OpenAiChat;

            fn default_model() -> &'static str {
                DEEPSEEK_DEFAULT_MODEL
            }

            fn provider_name() -> &'static str {
                "deepseek"
            }

            fn adapter(base_url: &str) -> Arc<dyn LlmPort> {
                let config =
                    DeepSeekConfig::new("test-key".to_string(), "deepseek-chat".to_string())
                        .with_base_url(base_url);
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

            // DeepSeek's wire has no dedicated refusal field (A3-50-12): a blocked answer is
            // `finish_reason: content_filter` with no text, which stays `Ok(ContentFilter)`.
            fn refusal_shape(_secret: &str) -> crate::conformance::RefusalShape {
                crate::conformance::RefusalShape::ContentFilterOnly(
                    crate::conformance::wire_defaults::chat_content_filter_body(),
                )
            }

            // DeepSeek reports the cache hit as top-level `prompt_cache_hit_tokens` (with a
            // `prompt_cache_miss_tokens` companion), not OpenAI's `prompt_tokens_details`.
            fn usage_body() -> String {
                json!({
                    "id": "cmpl-1",
                    "model": DEEPSEEK_DEFAULT_MODEL,
                    "choices": [{
                        "index": 0,
                        "message": {"role": "assistant", "content": "Hi there"},
                        "finish_reason": "stop"
                    }],
                    "usage": {
                        "prompt_tokens": 100,
                        "completion_tokens": 40,
                        "total_tokens": 140,
                        "prompt_cache_hit_tokens": 30,
                        "prompt_cache_miss_tokens": 70,
                        "completion_tokens_details": {"reasoning_tokens": 15}
                    }
                })
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

/// The PARITY-03 knobs on `DeepSeekConfig` (plan 50-08, D-12): each one is read by the adapter.
#[cfg(test)]
mod knob_tests {
    use super::*;
    use crate::test_support::{
        attempts_against_a_failing_endpoint, mock_completion, send_with_max_tokens, server,
    };
    use crate::window::effective_capabilities;

    const UNSEEDED: &str = "deepseek-unseeded-test-model";

    fn config(base_url: &str, model: &str) -> DeepSeekConfig {
        DeepSeekConfig::new("test-key".to_string(), model.to_string()).with_base_url(base_url)
    }

    fn window_override(window: u32) -> ModelOverride {
        ModelOverride {
            context_window: Some(window),
            ..ModelOverride::default()
        }
    }

    fn error_of(config: DeepSeekConfig) -> String {
        match DeepSeekAdapter::new(config) {
            Err(error) => error.to_string(),
            Ok(_) => panic!("construction must fail"),
        }
    }

    #[tokio::test]
    async fn deepseek_max_tokens_default_reaches_the_wire() {
        let mut server = server().await;
        let captured = mock_completion(&mut server).await;
        let url = server.url();

        let adapter = DeepSeekAdapter::new(config(&url, UNSEEDED).with_max_tokens(2048)).unwrap();
        let body = send_with_max_tokens(&adapter, &captured, UNSEEDED, None)
            .await
            .unwrap();
        assert_eq!(body["max_tokens"], 2048, "{body}");
        // A request's own value wins.
        let body = send_with_max_tokens(&adapter, &captured, UNSEEDED, Some(512))
            .await
            .unwrap();
        assert_eq!(body["max_tokens"], 512, "{body}");

        // No configured default: no `max_tokens` key, as before PARITY-03.
        let plain = DeepSeekAdapter::new(config(&url, UNSEEDED)).unwrap();
        let body = send_with_max_tokens(&plain, &captured, UNSEEDED, None)
            .await
            .unwrap();
        assert!(body.get("max_tokens").is_none(), "{body}");
    }

    #[tokio::test]
    async fn deepseek_max_tokens_default_is_checked_against_the_profile() {
        let mut server = server().await;
        let captured = mock_completion(&mut server).await;
        // `deepseek-flash` is seeded with a 393,216-token maximum output.
        let adapter =
            DeepSeekAdapter::new(config(&server.url(), "deepseek-flash").with_max_tokens(500_000))
                .unwrap();
        let Err(LlmError::InvalidPrompt(message)) =
            send_with_max_tokens(&adapter, &captured, "deepseek-flash", None).await
        else {
            panic!("a default above the model's maximum output must be InvalidPrompt");
        };
        for needle in ["max_tokens", "500000", "393216"] {
            assert!(
                message.contains(needle),
                "'{needle}' missing from: {message}"
            );
        }
        assert!(captured.lock().unwrap().is_none(), "nothing was sent");
    }

    #[tokio::test]
    async fn deepseek_max_retries_is_read_by_the_adapter() {
        // The loop retries up to `max_retries` times after the first attempt.
        for retries in [0_u32, 1] {
            let server = server().await;
            let adapter =
                DeepSeekAdapter::new(config(&server.url(), UNSEEDED).with_max_retries(retries))
                    .unwrap();
            attempts_against_a_failing_endpoint(server, &adapter, UNSEEDED, retries as usize + 1)
                .await;
        }
    }

    #[test]
    fn deepseek_context_window_override_is_declared() {
        let adapter = DeepSeekAdapter::new(
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
    fn context_window_override_beats_a_seeded_window_on_deepseek() {
        let base = || config("https://example.invalid/v1", "deepseek-flash");
        let seeded = DeepSeekAdapter::new(base()).unwrap();
        assert_eq!(
            seeded
                .model_profile("deepseek-flash")
                .unwrap()
                .context_window,
            Some(1_048_576)
        );

        // The operator's adapter-wide statement beats the compiled seeded fact ...
        let adapter = DeepSeekAdapter::new(base().with_context_window(500_000)).unwrap();
        assert_eq!(
            adapter
                .model_profile("deepseek-flash")
                .unwrap()
                .context_window,
            Some(500_000)
        );
        assert_eq!(
            effective_capabilities(&adapter, "deepseek-flash").max_context_tokens,
            Some(500_000)
        );

        // ... and a per-model override beats both.
        let adapter =
            DeepSeekAdapter::new(base().with_context_window(500_000).with_model_overrides(
                BTreeMap::from([("deepseek-flash".to_string(), window_override(900_000))]),
            ))
            .unwrap();
        assert_eq!(
            adapter
                .model_profile("deepseek-flash")
                .unwrap()
                .context_window,
            Some(900_000)
        );

        // A window below the seeded maximum output is refused, never clamped.
        let message = error_of(base().with_context_window(100_000));
        for needle in ["deepseek", "deepseek-flash", "100000", "393216"] {
            assert!(
                message.contains(needle),
                "'{needle}' missing from: {message}"
            );
        }
    }

    #[test]
    fn deepseek_invalid_override_fails_construction_naming_provider_and_model() {
        let with_override = |over: ModelOverride| {
            config("https://example.invalid/v1", UNSEEDED)
                .with_model_overrides(BTreeMap::from([(UNSEEDED.to_string(), over)]))
        };
        let adapter = DeepSeekAdapter::new(with_override(window_override(900_000))).unwrap();
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
                message.contains("deepseek") && message.contains(UNSEEDED),
                "the error must name the provider and the model: {message}"
            );
        }
    }

    #[test]
    fn deepseek_config_defaults_a_reasoning_model_to_300s() {
        // (model, expected default timeout): `deepseek-flash` is a seeded reasoning model.
        for (model, expected) in [
            ("deepseek-flash", 300),
            ("deepseek-chat", 60),
            (UNSEEDED, 60),
        ] {
            assert_eq!(
                config("https://example.invalid/v1", model).timeout_seconds,
                expected,
                "new({model})"
            );
        }
        assert_eq!(
            config("https://example.invalid/v1", DEEPSEEK_DEFAULT_MODEL).timeout_seconds,
            300,
            "the default model is a seeded reasoning model"
        );
        let mut explicit = config("https://example.invalid/v1", "deepseek-flash");
        explicit.timeout_seconds = 45;
        assert_eq!(explicit.timeout_seconds, 45);
    }

    #[test]
    fn deepseek_config_debug_redacts_the_api_key() {
        let config = config("https://api.example.invalid/v1", "deepseek-flash");
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
