//! OpenAI GPT adapter.
//!
//! Implements [`LlmPort`] for the OpenAI Chat Completions API.
//! Supports GPT-3.5-Turbo, GPT-4, GPT-4o, and other compatible models.

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use paladin_core::platform::container::content::{ContentItem, ContentType};
use paladin_core::platform::container::prompt::{PromptItem, PromptRole, PromptType};
use paladin_ports::output::llm_port::{
    FinishReason, LlmError, LlmPort, LlmRequest, LlmResponse, ModelProfile, ParamSupport,
    ProviderCapabilities, ResponseFormat, StopSupport, StreamingResponse, TokenUsage,
};
use rand::Rng;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::pin::Pin;
use std::time::{Duration, SystemTime};

use crate::backoff::exponential_backoff;
use crate::http_status::map_http_status_with_hints;
use crate::rate_limit_headers::{RateLimitHeaderFamily, hints_from_headers};
use crate::redaction::RedactedKey;
use crate::registry::{
    DEFAULT_MAX_RETRIES, ModelOverride, ModelRegistry, config_knob_builders, seeded,
};
use crate::response_guards::{StreamCompletion, bill, classify_completion};
use crate::sampling::{self, FormatLog, OmissionLog, RequestNotes};
use crate::sse::SseLines;
use crate::usage_reconcile::{USAGE_UNRECONCILED_METADATA_KEY, UsageLog};
use paladin_ports::output::rate_limit_hints::RateLimitHints;

/// The provider name this adapter reports through [`LlmPort::get_provider_name`]
/// and stamps on every [`LlmError::ProviderError`] it emits.
const OPENAI_PROVIDER: &str = "openai";

/// The `max_tokens` [`OpenAIConfig::new`] and [`OpenAIConfig::from_env`] configure: a compiled
/// default, not a fabricated sampling value (ADR-0061 permits a documented configuration
/// default). It is the cap OpenAI users have always had, now the documented
/// [`OpenAIConfig::max_tokens`] knob (D-13). The effective value is checked against the model's
/// profiled maximum output like any other.
///
/// Paladin calls the knob `max_tokens`; the OpenAI wire key is `max_completion_tokens` for every
/// model (owner decision 2026-10-17, G-50-5b, UAT test 5). Acceptance of that key was measured
/// for `gpt-5.4-nano` and `gpt-4o-mini` only (2026-10-15).
pub(crate) const OPENAI_DEFAULT_MAX_TOKENS: u32 = 4096;

/// The default model an [`OpenAIConfig`] names when the caller states none:
/// [`OpenAIConfig::from_env`], and the layered path when neither the model argument nor the
/// block names one.
///
/// `gpt-5.4-nano` is the provider default the Phase 50 plan 50-17 probe chose and the operator
/// kept (Phase 50 D-08, Phase 50.1 D-17). It is a label for the adapter's default model only:
/// each request's own `LlmRequest.model` still decides what is sent, and this value changes no
/// adapter-wide limit (the context window stays [`OpenAIConfig::context_window`] or 128,000).
pub const OPENAI_DEFAULT_MODEL: &str = "gpt-5.4-nano";

/// The endpoint [`OpenAIConfig::new`], [`OpenAIConfig::from_env`] (when `OPENAI_BASE_URL` is
/// unset) and the layered path use: the OpenAI API. Replace it with
/// [`OpenAIConfig::with_base_url`].
pub const OPENAI_DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";

/// The most stop sequences the OpenAI chat-completions API accepts (`stop` is an array with
/// `maxItems: 4` in the openai-openapi specification). A longer list is omitted and recorded,
/// never truncated (ADR-0061 (a)).
const OPENAI_MAX_STOP_SEQUENCES: u32 = 4;

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
///
/// Marked `#[non_exhaustive]` (owner decision, Phase 50 plan 50-08, option-a): build one with
/// [`OpenAIConfig::new`] or [`OpenAIConfig::from_env`] and set fields or call the `with_*`
/// builders, so a later field is not another break for callers.
#[derive(Clone)]
#[non_exhaustive]
pub struct OpenAIConfig {
    /// OpenAI API key.
    pub api_key: String,
    /// Base URL for the API ([`OPENAI_DEFAULT_BASE_URL`] unless replaced with
    /// [`with_base_url`](Self::with_base_url) or `OPENAI_BASE_URL`).
    pub base_url: String,
    /// The adapter's default model: what [`OpenAIConfig::new`] was given, what
    /// [`OpenAIConfig::from_env`] sets ([`OPENAI_DEFAULT_MODEL`]), or what
    /// `LlmProviderFactory::create_from_config` resolved. Each request's own model decides
    /// what is sent on the wire, and this field changes no adapter-wide limit: the context
    /// window stays [`context_window`](Self::context_window) or the compiled 128,000.
    pub model: String,
    /// Optional organisation ID.
    pub organization: Option<String>,
    /// Request timeout in seconds (default: 300).
    pub timeout_seconds: u64,
    /// Maximum retry attempts (default: 3).
    pub max_retries: u32,
    /// Default output budget (`max_tokens`) sent when a request sets none. [`OpenAIConfig::new`]
    /// and [`OpenAIConfig::from_env`] set `Some(4096)`, the cap this adapter has always sent;
    /// `None` sends no output-cap key. The value is checked against the model's profiled
    /// maximum output exactly as a request's own `max_tokens` is (ADR-0061 (i)): above it, the
    /// request is refused before any HTTP call.
    ///
    /// Paladin names the knob `max_tokens`; on the wire the adapter sends it as
    /// `max_completion_tokens` for every model (owner decision 2026-10-17, G-50-5b), the key
    /// `gpt-5.4-nano` requires. Acceptance of that key was measured for `gpt-5.4-nano` and
    /// `gpt-4o-mini` only (2026-10-15); a server behind `base_url` that accepts only the old
    /// `max_tokens` key belongs on the `openai-compatible` preset instead.
    pub max_tokens: Option<u32>,
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
impl std::fmt::Debug for OpenAIConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAIConfig")
            .field("api_key", &RedactedKey(&self.api_key))
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("organization", &self.organization)
            .field("timeout_seconds", &self.timeout_seconds)
            .field("max_retries", &self.max_retries)
            .field("max_tokens", &self.max_tokens)
            .field("context_window", &self.context_window)
            .field("model_overrides", &self.model_overrides)
            .finish()
    }
}

config_knob_builders!(OpenAIConfig, organization);

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
    ///
    /// [`model`](Self::model) is the compiled default, [`OPENAI_DEFAULT_MODEL`]; no variable
    /// names it.
    pub fn from_env() -> Result<Self, String> {
        let api_key = env::var("OPENAI_API_KEY")
            .map_err(|_| "OPENAI_API_KEY environment variable not set")?;

        let base_url =
            env::var("OPENAI_BASE_URL").unwrap_or_else(|_| OPENAI_DEFAULT_BASE_URL.to_string());

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
            model: OPENAI_DEFAULT_MODEL.to_string(),
            organization,
            timeout_seconds,
            max_retries,
            max_tokens: Some(OPENAI_DEFAULT_MAX_TOKENS),
            context_window: None,
            model_overrides: BTreeMap::new(),
        })
    }

    /// Create a configuration from an OpenAI API key and the adapter's default model.
    ///
    /// `api_key` is the OpenAI key, never a URL: the endpoint is [`OPENAI_DEFAULT_BASE_URL`]
    /// (replace it with [`with_base_url`](Self::with_base_url)). `model` is the adapter's
    /// default model, stored in [`model`](Self::model); each request's own model still decides
    /// what is sent, and the value changes no adapter-wide limit. The timeout is 300 s,
    /// `max_tokens` is `Some(4096)` and the retry budget is the shared default; set any of them
    /// with the `with_*` builders.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_llm::openai::adapter::{
    ///     OPENAI_DEFAULT_BASE_URL, OPENAI_DEFAULT_MODEL, OpenAIConfig,
    /// };
    ///
    /// let config = OpenAIConfig::new("sk-example".to_string(), OPENAI_DEFAULT_MODEL.to_string())
    ///     .with_timeout_seconds(60);
    /// assert_eq!(config.model, OPENAI_DEFAULT_MODEL);
    /// assert_eq!(config.base_url, OPENAI_DEFAULT_BASE_URL);
    /// assert_eq!(config.timeout_seconds, 60);
    /// assert_eq!(config.max_tokens, Some(4096));
    /// ```
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            base_url: OPENAI_DEFAULT_BASE_URL.to_string(),
            model,
            organization: None,
            timeout_seconds: 300,
            max_retries: DEFAULT_MAX_RETRIES,
            max_tokens: Some(OPENAI_DEFAULT_MAX_TOKENS),
            context_window: None,
            model_overrides: BTreeMap::new(),
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
        if self.model.is_empty() {
            return Err("Model name cannot be empty".to_string());
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
    /// The output-token cap. Paladin calls it `max_tokens` (the field, [`OpenAIConfig::max_tokens`]
    /// and `PromptParameters::max_tokens` keep that name); on the wire it is
    /// `max_completion_tokens` for every OpenAI model, because `gpt-5.4-nano` rejects
    /// `max_tokens` with HTTP 400 `unsupported_parameter` (owner decision 2026-10-17, G-50-5b,
    /// option (a)). Acceptance of the new key was measured for `gpt-5.4-nano` and `gpt-4o-mini`
    /// only (2026-10-15); there is deliberately no per-model switch.
    #[serde(
        rename = "max_completion_tokens",
        skip_serializing_if = "Option::is_none"
    )]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    /// The caller's stop sequences, in declared order with duplicates kept, when the model's
    /// profile admits their count (ADR-0061 (a), D-11a). Absent otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
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

/// A message on the REQUEST path: always a role and a plain string.
#[derive(Debug, Serialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

/// The assistant message on the RESPONSE path.
///
/// Distinct from [`OpenAIMessage`] because the two shapes differ: a response `content` may be
/// `null` or absent (a refusal carries `content: null` and a `refusal` string, and a tool call
/// carries no text), where a request `content` is always a string. Sharing one type made a
/// refusal a parse failure (D-09).
#[derive(Debug, Deserialize)]
struct OpenAIResponseMessage {
    #[allow(dead_code)]
    #[serde(default)]
    role: Option<String>,
    /// `null`, absent and `""` all mean "no text".
    #[serde(default)]
    content: Option<String>,
    /// The vendor's dedicated refusal text; `null` and absent mean the model did not refuse.
    #[serde(default)]
    refusal: Option<String>,
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
    message: OpenAIResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    /// OpenAI's own reported total (G-50-5a). Parsed and handed to the
    /// reconciliation ([`crate::usage_reconcile`]), never copied:
    /// `TokenUsage::new` derives `total_tokens` as `prompt_tokens +
    /// completion_tokens` (D-02), and a disagreement is recorded under
    /// [`USAGE_UNRECONCILED_METADATA_KEY`]. `None` when the key is absent --
    /// a usage object without it still parses, and nothing is fabricated.
    #[serde(default)]
    total_tokens: Option<u32>,
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
    /// A fragment of the vendor's refusal text (D-09); `null` and absent mean none.
    #[serde(default)]
    refusal: Option<String>,
}

// ---------------------------------------------------------------------------
// Adapter
// ---------------------------------------------------------------------------

/// OpenAI LLM adapter implementing [`LlmPort`].
pub struct OpenAIAdapter {
    pub(crate) config: OpenAIConfig,
    pub(crate) client: Client,
    /// Resolves the request model to the profile that decides what reaches the wire (ADR-0061).
    model_registry: ModelRegistry,
    /// Which (model, parameter) omissions this instance has already warned about.
    omissions: OmissionLog,
    /// Which (model, adjustment) `response_format` changes this instance has already warned
    /// about.
    format_log: FormatLog,
    /// Which (model, check) usage findings this instance has already warned about (G-50-5a).
    usage_log: UsageLog,
}

/// What this adapter's request builder carries today, stated once so [`OpenAIAdapter::new`]'s
/// adapter-wide fallback profile and [`LlmPort::get_capabilities`] cannot drift apart.
fn declared_capabilities(context_window: Option<u32>) -> ProviderCapabilities {
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
        // The operator's adapter-wide window (PARITY-03) replaces the compiled 128,000.
        max_context_tokens: context_window.or(Some(128000)),
        supports_embeddings: true,
        supports_system_messages: true,
        temperature_range: Some((0.0, 1.0)),
    }
}

/// The adapter-wide fallback profile, derived from what this adapter's request builder sends
/// (ADR-0061, D-05): a field it sends maps to `Range`, a field it never sends maps to `NotSent`.
///
/// - `temperature`: `Range` over the declared `temperature_range`; `top_p`: `Range(0.0, 1.0)`.
/// - penalties: `NotSent` -- [`OpenAIRequest`] has no penalty field.
/// - `stop`: `Supported { max: Some(4) }` (the openai-openapi `maxItems: 4`).
/// - `json_object` and `json_schema`: both true -- this adapter sends both formats.
fn fallback_profile(context_window: Option<u32>) -> ModelProfile {
    let caps = declared_capabilities(context_window);
    let (low, high) = caps.temperature_range.unwrap_or((0.0, 1.0));
    let mut profile = ModelProfile::new()
        .with_temperature(ParamSupport::Range(low, high))
        .with_top_p(ParamSupport::Range(0.0, 1.0))
        .with_frequency_penalty(ParamSupport::NotSent)
        .with_presence_penalty(ParamSupport::NotSent)
        .with_stop_sequences(StopSupport::Supported {
            max: Some(OPENAI_MAX_STOP_SEQUENCES),
        })
        .with_json_object(true)
        .with_json_schema(true);
    if let Some(window) = caps.max_context_tokens {
        profile = profile.with_context_window(window);
    }
    profile
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
        // PARITY-03: the operator's adapter-wide window, then the per-model overrides, are laid
        // over the seeded table; either one failing construction names the provider and model.
        let model_registry = ModelRegistry::new(
            OPENAI_PROVIDER,
            seeded::OPENAI,
            fallback_profile(config.context_window),
        )
        .with_context_window(config.context_window)
        .and_then(|registry| registry.with_overrides(config.model_overrides.clone()))
        .map_err(|e| format!("Invalid OpenAI configuration: {e}"))?;
        Ok(Self {
            config,
            client,
            // The dated OpenAI table (`seeded::OPENAI`, probed 2026-10-15): the two measured
            // models resolve to their own entry, every other model to the adapter-wide fallback.
            model_registry,
            omissions: OmissionLog::new(),
            format_log: FormatLog::new(),
            usage_log: UsageLog::new(),
        })
    }

    /// Build the outgoing request body for `request`, plus the notes the buffered path records
    /// on its response.
    ///
    /// What reaches the wire is decided by the request model's [`ModelProfile`] (ADR-0061, D-01):
    /// [`sampling::apply`] sends `temperature`, `top_p` and `stop` only when the caller set them
    /// **and** the profile admits them, so an unset parameter is absent from the body -- this
    /// adapter no longer invents a `temperature` of 0.7 or a `top_p` of 1.0 -- and a set but
    /// inadmissible one is omitted and recorded, never clamped or swapped. `max_tokens` keeps
    /// the configured [`OpenAIConfig::max_tokens`] (4096 from `new`/`from_env`, the cap this
    /// adapter has always sent) and the effective value is checked against the
    /// profile's maximum output before anything is sent. It reaches the wire as
    /// `max_completion_tokens` for every model (owner decision 2026-10-17, G-50-5b; measured
    /// accepted on `gpt-5.4-nano` and `gpt-4o-mini` only, 2026-10-15). The `response_format`
    /// follows the profile through [`sampling::apply_response_format`].
    ///
    /// Shared by [`LlmPort::generate`] and [`LlmPort::generate_stream`], so the two paths cannot
    /// drift.
    fn build_request(
        &self,
        request: &LlmRequest,
        stream: bool,
    ) -> Result<(OpenAIRequest, RequestNotes), LlmError> {
        let messages = self.convert_to_messages(&request.prompt, &request.attachments)?;
        let params = &request.prompt.entity.data.parameters;
        let profile = self.model_registry.profile(&request.model);

        // The caller's value, else the configured default (`Some(4096)` from `new`/`from_env`);
        // `None` sends no output-cap key. Either is held to the model's maximum output. The wire
        // name is `max_completion_tokens` (the `OpenAIRequest` field's serde rename).
        let max_tokens = params.max_tokens.or(self.config.max_tokens);
        sampling::check_max_tokens(OPENAI_PROVIDER, &request.model, &profile, max_tokens)?;

        let applied = sampling::apply(&profile, params);
        let params_omitted = self
            .omissions
            .record(OPENAI_PROVIDER, &request.model, &applied);

        let (effective_format, adjustment) =
            sampling::apply_response_format(&profile, request.response_format.as_ref());
        let format_adjusted = self
            .format_log
            .record(OPENAI_PROVIDER, &request.model, adjustment);

        let unprofiled_model = self
            .model_registry
            .note_unprofiled(&request.model)
            .then(|| request.model.clone());

        let body = OpenAIRequest {
            model: request.model.clone(),
            messages,
            temperature: applied.temperature,
            max_tokens,
            top_p: applied.top_p,
            stop: applied.stop,
            stream,
            response_format: effective_format.as_ref().map(to_openai_response_format),
            stream_options: stream.then_some(OpenAIStreamOptions {
                include_usage: true,
            }),
        };
        let notes = RequestNotes {
            params_omitted,
            format_adjusted,
            unprofiled_model,
        };
        Ok((body, notes))
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
                    content: crate::turn::user_turn_text(user_prompt),
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

    /// Map OpenAI's finish-reason string to [`FinishReason`], for the buffered and the streaming
    /// path alike so the two cannot drift.
    ///
    /// `tool_calls` (what the current API sends when the model wants a tool) and the legacy
    /// `function_call` both map to [`FinishReason::FunctionCall`] (RESEARCH Pitfall 6); an
    /// unrecognised string stays [`FinishReason::Error`], never silently `Stop`.
    fn convert_finish_reason(reason: Option<&str>) -> FinishReason {
        match reason {
            Some("stop") => FinishReason::Stop,
            Some("length") => FinishReason::Length,
            Some("content_filter") => FinishReason::ContentFilter,
            Some("function_call") | Some("tool_calls") => FinishReason::FunctionCall,
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
    ///
    /// Returns the mapped usage together with OpenAI's own `total_tokens`
    /// (when reported), which the caller reconciles rather than copies (D-02,
    /// G-50-5a).
    ///
    /// Reasoning is read inside `completion_tokens`, OpenAI's documented
    /// contract. Not live-measured: the 2026-10-18 probe
    /// (`50-USAGE-PROBE.md`, plan 50-24) got no reasoning count above 0 from
    /// gpt-5.4-nano or gpt-4o-mini (`indeterminate`), though both vendor
    /// totals agreed with prompt + completion (26 + 4 = 30, 27 + 1 = 28).
    fn map_usage(usage: OpenAIUsage) -> (TokenUsage, Option<u32>) {
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
        (mapped, usage.total_tokens)
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
                        let max_delay = Duration::from_secs(10);
                        let delay = exponential_backoff(base_delay, attempt, max_delay);

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
        stream_metadata: HashMap<String, String>,
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
        // The vendor's own total from the held usage frame, reconciled at the terminal chunk
        // (G-50-5a). A clone shares this instance's usage log.
        let mut held_vendor_total: Option<u32> = None;
        let usage_log = self.usage_log.clone();

        // D-09 / D-11a on the streaming path: the terminal chunk is decided by the same
        // `classify_completion` the buffered path uses, over what the stream accumulated.
        let provider = self.get_provider_name();
        let model = request.model.clone();
        let api_key = self.config.api_key.clone();
        let mut completion = StreamCompletion::default();

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
                        let mut items = Vec::new();

                        for line in &lines {
                            let Some(data) = line.strip_prefix("data: ") else {
                                continue;
                            };

                            if data.trim() == "[DONE]" {
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
                                    // A refusal or an empty answer replaces the terminal chunk;
                                    // deltas already yielded stay yielded. G-50-5a: the error
                                    // carries the usage frame's bill on the request model,
                                    // `None` when the stream reported none.
                                    Err(error) => {
                                        items.push(Err(bill(error, &model, held_usage.take())))
                                    }
                                }
                                continue;
                            }

                            match serde_json::from_str::<OpenAIStreamChunk>(data) {
                                Ok(chunk) => {
                                    if let Some(usage) = chunk.usage {
                                        let (mapped, vendor_total) = Self::map_usage(usage);
                                        held_usage = Some(mapped);
                                        held_vendor_total = vendor_total;
                                    }
                                    if let Some(choice) = chunk.choices.first() {
                                        if let Some(reason) = &choice.finish_reason {
                                            held_finish_reason =
                                                Some(Self::convert_finish_reason(Some(reason)));
                                        }
                                        let delta =
                                            choice.delta.content.clone().unwrap_or_default();
                                        completion.observe_content(&delta);
                                        if let Some(refusal) = &choice.delta.refusal {
                                            completion.observe_refusal(refusal);
                                        }
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
        let (openai_request, notes) = self.build_request(&request, false)?;

        let response = self.make_request_with_retries(&openai_request).await?;

        // G-50-5a: usage is mapped BEFORE the no-choices check and the guard, so a call OpenAI
        // billed but that is classified as empty or refused carries the same folded, reconciled
        // usage a success would have, on the served model (else the request model, A3-50-26).
        let (usage, vendor_total) = Self::map_usage(response.usage);
        let billed_model = if response.model.is_empty() {
            request.model.as_str()
        } else {
            response.model.as_str()
        };

        // WR-04: a body with no choices is an `EmptyCompletion` carrying the bill, as on the
        // compat engine and Gemini -- not a `ProcessingError`, which dropped the usage and
        // read as `Transience::Unknown` to the retry loop (a billed request re-sent).
        let Some(choice) = response.choices.first() else {
            return Err(bill(
                LlmError::empty_completion("Response contained no choices".to_string()),
                billed_model,
                Some(usage),
            ));
        };
        let finish_reason = Self::convert_finish_reason(choice.finish_reason.as_deref());
        let content = choice.message.content.clone().unwrap_or_default();

        // One rule for refusal, content filter, function call and empty text on any finish
        // reason -- OpenAI included (PARITY-05, D-09/D-11a). It used to return `Ok("")`.
        classify_completion(
            self.get_provider_name(),
            &request.model,
            &content,
            &finish_reason,
            choice.message.refusal.as_deref(),
            &self.config.api_key,
        )
        .map_err(|err| bill(err, billed_model, Some(usage.clone())))?;
        // G-50-5a: OpenAI's own total is the check on the mapping -- a disagreement is warned
        // once and recorded, never written over the mapped total (D-02).
        let unreconciled = self.usage_log.record(
            self.get_provider_name(),
            &request.model,
            &usage,
            vendor_total,
        );

        let mut llm_response =
            LlmResponse::new(request.id, response.model, content, finish_reason, usage);
        llm_response.metadata = notes.into_metadata();
        if let Some(value) = unreconciled {
            llm_response
                .metadata
                .insert(USAGE_UNRECONCILED_METADATA_KEY.to_string(), value);
        }
        Ok(llm_response)
    }

    /// Streams the completion. A stream that ends refused or with no text ends with
    /// `Err(LlmError::Refusal)` / `Err(LlmError::EmptyCompletion)` in place of the terminal
    /// chunk, and that error item carries the usage frame's figures as its billed usage
    /// ([`LlmError::billed_usage`], G-50-5a) on the request model, or `None` when the stream
    /// reported no usage. Deltas already yielded stay yielded.
    async fn generate_stream(
        &self,
        request: LlmRequest,
    ) -> Result<Box<dyn Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError> {
        // ADR-0061 (h), owner reading R-1 (option-b): the omissions, format changes and
        // unprofiled model `build_request` `warn!`s about are also stamped on the stream's
        // terminal chunk (only that chunk), as the buffered path stamps `LlmResponse.metadata`.
        let (openai_request, notes) = self.build_request(&request, true)?;

        let stream = self
            .make_streaming_request(&openai_request, notes.into_metadata())
            .await?;
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
        declared_capabilities(self.config.context_window)
    }

    fn model_profile(&self, model: &str) -> Option<ModelProfile> {
        Some(self.model_registry.profile(model))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = OpenAIConfig::new("test-key".to_string(), OPENAI_DEFAULT_MODEL.to_string());
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.base_url, "https://api.openai.com/v1");
        assert_eq!(config.timeout_seconds, 300);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_config_validation() {
        let valid_config =
            OpenAIConfig::new("test-key".to_string(), OPENAI_DEFAULT_MODEL.to_string());
        assert!(valid_config.validate().is_ok());

        let invalid_config = OpenAIConfig {
            api_key: String::new(),
            base_url: "https://api.openai.com/v1".to_string(),
            organization: None,
            timeout_seconds: 300,
            max_retries: 3,
            max_tokens: Some(4096),
            context_window: None,
            model: OPENAI_DEFAULT_MODEL.to_string(),
            model_overrides: BTreeMap::new(),
        };
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn openai_config_debug_prints_the_model_and_redacts_the_key() {
        let config = OpenAIConfig::new(
            "sk-secret-key-123".to_string(),
            OPENAI_DEFAULT_MODEL.to_string(),
        );
        let printed = format!("{config:?}");
        assert!(printed.contains(OPENAI_DEFAULT_MODEL), "{printed}");
        assert!(printed.contains("<redacted>"), "{printed}");
        assert!(!printed.contains("sk-secret-key-123"), "{printed}");
    }

    #[test]
    fn openai_config_validate_rejects_an_empty_model() {
        let mut config =
            OpenAIConfig::new("test-key".to_string(), OPENAI_DEFAULT_MODEL.to_string());
        config.model = String::new();
        let error = config.validate().unwrap_err();
        assert!(error.contains("Model"), "{error}");
    }

    #[test]
    fn openai_config_model_changes_no_adapter_wide_limit() {
        // D-06 as locked (plan 50.1-02 checkpoint, `proceed-as-locked`): `model` is a stored
        // default-model label. It must not move the adapter-wide window.
        let small = OpenAIConfig::new("test-key".to_string(), "gpt-4o-mini".to_string());
        let nano = OpenAIConfig::new("test-key".to_string(), "gpt-5.4-nano".to_string());
        let small = OpenAIAdapter::new(small).unwrap().get_capabilities();
        let nano = OpenAIAdapter::new(nano).unwrap().get_capabilities();
        assert_eq!(small.max_context_tokens, Some(128_000));
        assert_eq!(small.max_context_tokens, nano.max_context_tokens);
    }

    #[test]
    fn openai_config_with_organization_sets_the_field() {
        let config = OpenAIConfig::new("test-key".to_string(), OPENAI_DEFAULT_MODEL.to_string())
            .with_organization("org-x");
        assert_eq!(config.organization, Some("org-x".to_string()));
        assert_eq!(
            OpenAIConfig::new("k".to_string(), OPENAI_DEFAULT_MODEL.to_string()).organization,
            None
        );
    }

    #[test]
    fn openai_config_new_takes_key_and_model_and_defaults_the_endpoint() {
        let config = OpenAIConfig::new("test-key".to_string(), "gpt-4o-mini".to_string());
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.model, "gpt-4o-mini");
        assert_eq!(config.base_url, OPENAI_DEFAULT_BASE_URL);
        assert_eq!(config.timeout_seconds, 300);
        assert_eq!(config.max_tokens, Some(4096));
        assert_eq!(OPENAI_DEFAULT_MODEL, "gpt-5.4-nano");
        assert_eq!(OPENAI_DEFAULT_BASE_URL, "https://api.openai.com/v1");
    }

    #[test]
    fn test_adapter_creation() {
        let config = OpenAIConfig::new("test-key".to_string(), OPENAI_DEFAULT_MODEL.to_string());
        let adapter = OpenAIAdapter::new(config);
        assert!(adapter.is_ok());
    }

    #[test]
    fn test_get_provider_name() {
        let config = OpenAIConfig::new("test-key".to_string(), OPENAI_DEFAULT_MODEL.to_string());
        let adapter = OpenAIAdapter::new(config).unwrap();
        assert_eq!(adapter.get_provider_name(), "openai");
    }

    #[test]
    fn test_get_capabilities() {
        let config = OpenAIConfig::new("test-key".to_string(), OPENAI_DEFAULT_MODEL.to_string());
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

    /// OpenAI has no adapter-level default model (the model arrives per request), so the
    /// operator-identified current model from the 2026-10-15 probe, `gpt-5.4-nano`, is the one
    /// checked here. `max_context_tokens` stays the adapter-wide 128,000 for the same reason:
    /// no default model's window exists to follow.
    #[test]
    fn openai_default_model_is_seeded() {
        let adapter = OpenAIAdapter::new(OpenAIConfig::new(
            "test-key".to_string(),
            OPENAI_DEFAULT_MODEL.to_string(),
        ))
        .unwrap();
        for (model, window) in [("gpt-5.4-nano", 400_000), ("gpt-4o-mini", 128_000)] {
            let (profile, origin) = adapter.model_registry.resolve(model);
            assert_eq!(
                origin,
                crate::registry::ProfileOrigin::Seeded {
                    verified_on: "2026-10-15"
                },
                "{model}"
            );
            assert_eq!(profile.context_window, Some(window), "{model}");
        }
        let (_, origin) = adapter.model_registry.resolve("gpt-4o");
        assert_eq!(origin, crate::registry::ProfileOrigin::AdapterDefault);
    }

    #[test]
    fn test_config_with_organization() {
        let mut config =
            OpenAIConfig::new("test-key".to_string(), OPENAI_DEFAULT_MODEL.to_string());
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
            max_tokens: Some(4096),
            context_window: None,
            model: OPENAI_DEFAULT_MODEL.to_string(),
            model_overrides: BTreeMap::new(),
        };
        assert!(config.validate().is_err());
    }

    // ── CR-02 (`25-REVIEW.md`): refused-redirect mapping ───────────────────

    #[test]
    fn map_error_maps_a_redirect_status_to_an_actionable_provider_error() {
        let config = OpenAIConfig::new("test-key".to_string(), OPENAI_DEFAULT_MODEL.to_string());
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
                max_tokens: Some(4096),
                context_window: None,
                model: OPENAI_DEFAULT_MODEL.to_string(),
                model_overrides: BTreeMap::new(),
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
                max_tokens: Some(4096),
                context_window: None,
                model: OPENAI_DEFAULT_MODEL.to_string(),
                model_overrides: BTreeMap::new(),
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
                max_tokens: Some(4096),
                context_window: None,
                model: OPENAI_DEFAULT_MODEL.to_string(),
                model_overrides: BTreeMap::new(),
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
                max_tokens: Some(4096),
                context_window: None,
                model: OPENAI_DEFAULT_MODEL.to_string(),
                model_overrides: BTreeMap::new(),
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
                    "max_completion_tokens",
                    "top_p",
                    "stream"
                ])
            );
        }
    }

    // ── Phase 50 (PARITY-08, handoff B-4): the user turn carries the query ─

    mod user_turn_wiring {
        use super::*;
        use mockito::Server;
        use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
        use serde_json::Value;
        use std::sync::{Arc, Mutex};

        fn adapter_at(base_url: &str) -> OpenAIAdapter {
            OpenAIAdapter::new(OpenAIConfig {
                api_key: "test-key".to_string(),
                base_url: base_url.to_string(),
                organization: None,
                timeout_seconds: 5,
                max_retries: 0,
                max_tokens: Some(4096),
                context_window: None,
                model: OPENAI_DEFAULT_MODEL.to_string(),
                model_overrides: BTreeMap::new(),
            })
            .expect("test config must build a valid adapter")
        }

        /// A request whose prompt is a `UserPrompt` with the given query and context.
        fn user_request(query: &str, context: Option<&str>) -> LlmRequest {
            LlmRequest::new(
                "gpt-4o",
                PromptItem::new(PromptType::User(UserPrompt {
                    query: query.to_string(),
                    context: context.map(str::to_string),
                }))
                .expect("a user prompt must build"),
            )
        }

        /// Sends `request` to a mock endpoint and returns the content of the last user message
        /// in the captured request body.
        async fn user_content_on_the_wire(request: LlmRequest) -> String {
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
                "mock returned a well-formed response: {result:?}"
            );

            let body_text = captured
                .lock()
                .unwrap()
                .take()
                .expect("mock must have been called");
            let body: Value = serde_json::from_str(&body_text).expect("body must be JSON");
            body["messages"]
                .as_array()
                .and_then(|messages| {
                    messages
                        .iter()
                        .rev()
                        .find(|m| m["role"] == "user")
                        .and_then(|m| m["content"].as_str().map(str::to_string))
                })
                .expect("the body must carry a user message with string content")
        }

        /// B-4: the execution service builds `UserPrompt { query, context: None }`; the adapter
        /// used to read only `context`, so the user message went out empty.
        #[tokio::test]
        async fn openai_user_turn_carries_query_on_the_execution_service_path() {
            let content = user_content_on_the_wire(user_request("What is 2+2?", None)).await;
            assert_eq!(content, "What is 2+2?");

            let content =
                user_content_on_the_wire(user_request("What is 2+2?", Some("Extra context.")))
                    .await;
            assert_eq!(content, "What is 2+2?\n\nExtra context.");
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
                max_tokens: Some(4096),
                context_window: None,
                model: OPENAI_DEFAULT_MODEL.to_string(),
                model_overrides: BTreeMap::new(),
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

    // ── Phase 50 (PARITY-02, ADR-0061, D-01): the per-model parameter policy ──

    mod parameter_policy {
        use super::*;
        use crate::registry::{ModelOverride, ModelRegistry, UNPROFILED_MODEL_METADATA_KEY};
        use crate::sampling::{PARAMS_OMITTED_METADATA_KEY, RESPONSE_FORMAT_ADJUSTED_METADATA_KEY};
        use mockito::Server;
        use paladin_core::platform::container::prompt::{
            PromptItem, PromptParameters, PromptType, UserPrompt,
        };
        use serde_json::{Value, json};
        use std::collections::{BTreeMap, HashMap};
        use std::sync::{Arc, Mutex};

        fn adapter_at(base_url: &str) -> OpenAIAdapter {
            OpenAIAdapter::new(OpenAIConfig {
                api_key: "test-key".to_string(),
                base_url: base_url.to_string(),
                organization: None,
                timeout_seconds: 5,
                max_retries: 0,
                max_tokens: Some(4096),
                context_window: None,
                model: OPENAI_DEFAULT_MODEL.to_string(),
                model_overrides: BTreeMap::new(),
            })
            .expect("test config must build a valid adapter")
        }

        /// A request whose prompt carries exactly `parameters` -- unlike
        /// `PromptParameters::default()`, nothing here is pre-set.
        fn request_with(parameters: PromptParameters) -> LlmRequest {
            let mut request = LlmRequest::new(
                "gpt-4o",
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
        }

        /// Runs `generate()` against a mock endpoint, returning the outgoing JSON body and the
        /// response metadata.
        async fn generate_on(
            adapter_for: impl FnOnce(&str) -> OpenAIAdapter,
            request: LlmRequest,
        ) -> (Value, HashMap<String, String>) {
            let mut server = Server::new_async().await;
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

            let response = adapter_for(&server.url())
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

        async fn generate_capture(request: LlmRequest) -> (Value, HashMap<String, String>) {
            generate_on(adapter_at, request).await
        }

        /// Runs `generate_stream()` against a mock endpoint and returns the outgoing JSON body.
        async fn stream_capture(request: LlmRequest) -> Value {
            let mut server = Server::new_async().await;
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

            let stream = adapter_at(&server.url())
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
        async fn openai_unset_sampling_values_are_absent() {
            let (body, metadata) = generate_capture(request_with(unset())).await;
            let obj = body.as_object().expect("body must be a JSON object");

            assert!(
                !obj.contains_key("temperature"),
                "no fabricated temperature: {obj:?}"
            );
            assert!(!obj.contains_key("top_p"), "no fabricated top_p: {obj:?}");
            assert!(
                !obj.contains_key("stop"),
                "no stop when none was set: {obj:?}"
            );
            assert_eq!(
                body["max_completion_tokens"], 4096,
                "the output cap keeps its documented compiled default"
            );
            assert!(
                !obj.contains_key("max_tokens"),
                "the old wire key is never sent: {obj:?}"
            );
            assert!(
                !metadata.contains_key(PARAMS_OMITTED_METADATA_KEY),
                "an unset parameter is not an omission: {metadata:?}"
            );
        }

        #[tokio::test]
        async fn openai_sends_what_the_caller_set_and_the_profile_admits() {
            let mut parameters = unset();
            parameters.temperature = Some(0.5);
            parameters.top_p = Some(0.9);
            parameters.max_tokens = Some(777);
            let (body, metadata) = generate_capture(request_with(parameters)).await;

            assert_eq!(body["temperature"], 0.5);
            assert!((body["top_p"].as_f64().unwrap() - 0.9).abs() < 1e-6);
            assert_eq!(
                body["max_completion_tokens"], 777,
                "the caller's max_tokens wins"
            );
            assert!(body.get("max_tokens").is_none(), "{body}");
            assert!(!metadata.contains_key(PARAMS_OMITTED_METADATA_KEY));
        }

        #[tokio::test]
        async fn openai_out_of_range_temperature_is_omitted_and_recorded() {
            let mut parameters = unset();
            parameters.temperature = Some(1.5);
            let (body, metadata) = generate_capture(request_with(parameters)).await;

            assert!(
                body.get("temperature").is_none(),
                "an out-of-range temperature is omitted, never clamped: {body}"
            );
            assert_eq!(
                metadata
                    .get(PARAMS_OMITTED_METADATA_KEY)
                    .map(String::as_str),
                Some("temperature")
            );
        }

        #[tokio::test]
        async fn openai_penalties_are_never_sent_and_are_recorded() {
            // `PromptParameters::default()` carries penalties of 0.0; this adapter has no
            // penalty field, so the fallback profile states them `NotSent` and the omission is
            // visible rather than silent.
            let (body, metadata) =
                generate_capture(request_with(PromptParameters::default())).await;

            assert!(body.get("frequency_penalty").is_none(), "{body}");
            assert!(body.get("presence_penalty").is_none(), "{body}");
            assert_eq!(
                metadata
                    .get(PARAMS_OMITTED_METADATA_KEY)
                    .map(String::as_str),
                Some("frequency_penalty,presence_penalty")
            );
        }

        #[tokio::test]
        async fn openai_stop_sequences_keep_declared_order() {
            let mut parameters = unset();
            // A duplicate, deliberately: order and duplicates are the caller's.
            parameters.stop_sequences = Some(vec![
                "b".to_string(),
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
            ]);
            let (body, metadata) = generate_capture(request_with(parameters)).await;

            assert_eq!(body["stop"], json!(["b", "a", "b", "c"]));
            assert!(!metadata.contains_key(PARAMS_OMITTED_METADATA_KEY));
        }

        #[tokio::test]
        async fn openai_stop_sequences_past_the_maximum_are_omitted_not_truncated() {
            let mut parameters = unset();
            parameters.stop_sequences = Some(
                ["a", "b", "c", "d", "e"]
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect(),
            );
            let (body, metadata) = generate_capture(request_with(parameters)).await;

            assert!(
                body.get("stop").is_none(),
                "five sequences exceed the maximum of four and are omitted whole: {body}"
            );
            assert_eq!(
                metadata
                    .get(PARAMS_OMITTED_METADATA_KEY)
                    .map(String::as_str),
                Some("stop")
            );
        }

        #[tokio::test]
        async fn openai_stream_follows_the_profile() {
            let mut parameters = unset();
            parameters.temperature = Some(1.5);
            parameters.top_p = Some(0.9);
            parameters.stop_sequences = Some(vec!["END".to_string(), "STOP".to_string()]);
            let body = stream_capture(request_with(parameters)).await;

            assert!(
                body.get("temperature").is_none(),
                "the streaming body omits an inadmissible temperature: {body}"
            );
            assert!((body["top_p"].as_f64().unwrap() - 0.9).abs() < 1e-6);
            assert_eq!(body["stop"], json!(["END", "STOP"]));
            assert_eq!(body["max_completion_tokens"], 4096);
            assert!(body.get("max_tokens").is_none(), "{body}");
            assert_eq!(body["stream"], true);

            let mut over = unset();
            over.stop_sequences = Some((0..5).map(|n| n.to_string()).collect());
            let body = stream_capture(request_with(over)).await;
            assert!(
                body.get("stop").is_none(),
                "a streamed over-long stop list is omitted whole: {body}"
            );
        }

        /// Phase 50, owner reading R-1 (option-b): a streamed call stamps the visibility keys
        /// the buffered path would, on the terminal chunk only.
        #[tokio::test]
        async fn openai_stream_terminal_chunk_carries_metadata() {
            let mut server = Server::new_async().await;
            server
                .mock("POST", "/chat/completions")
                .with_status(200)
                .with_header("content-type", "text/event-stream")
                .with_body(
                    "data: {\"id\":\"1\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hi\"},\"finish_reason\":null}]}\n\n\
                     data: {\"id\":\"1\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"\"},\"finish_reason\":\"stop\"}]}\n\n\
                     data: [DONE]\n\n",
                )
                .create_async()
                .await;

            let mut parameters = unset();
            parameters.temperature = Some(1.5); // above the profile's (0.0, 1.0)
            let stream = adapter_at(&server.url())
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
                Some("gpt-4o")
            );
            for content in &chunks[..chunks.len() - 1] {
                assert!(content.metadata.is_empty(), "content chunk: {content:?}");
            }
        }

        #[tokio::test]
        async fn openai_unprofiled_model_is_stamped() {
            let (_, metadata) = generate_capture(request_with(unset())).await;
            assert_eq!(
                metadata
                    .get(UNPROFILED_MODEL_METADATA_KEY)
                    .map(String::as_str),
                Some("gpt-4o"),
                "no seeded entry exists yet, so the model runs on the adapter-wide fallback"
            );
        }

        #[test]
        fn openai_model_profile_is_always_some_and_matches_what_the_wire_sends() {
            let adapter = adapter_at("http://localhost:1");
            let profile = adapter
                .model_profile("any-model")
                .expect("openai profiles every model");
            assert_eq!(profile.temperature, ParamSupport::Range(0.0, 1.0));
            assert_eq!(profile.top_p, ParamSupport::Range(0.0, 1.0));
            assert_eq!(profile.frequency_penalty, ParamSupport::NotSent);
            assert_eq!(profile.presence_penalty, ParamSupport::NotSent);
            assert_eq!(
                profile.stop_sequences,
                StopSupport::Supported { max: Some(4) }
            );
            assert_eq!(profile.context_window, Some(128_000));
            assert!(profile.json_object && profile.json_schema);
        }

        #[tokio::test]
        async fn openai_json_schema_still_reaches_the_wire_as_json_schema() {
            let request = request_with(unset()).with_response_format(ResponseFormat::JsonSchema {
                name: "answer".to_string(),
                schema: json!({"type": "object"}),
                strict: true,
            });
            let (body, metadata) = generate_capture(request).await;

            assert_eq!(body["response_format"]["type"], "json_schema");
            assert!(
                !metadata.contains_key(RESPONSE_FORMAT_ADJUSTED_METADATA_KEY),
                "nothing changed on the wire, so nothing is recorded: {metadata:?}"
            );
        }

        #[tokio::test]
        async fn openai_max_tokens_default_is_checked_against_the_profile() {
            let mut server = Server::new_async().await;
            let mock = server
                .mock("POST", "/chat/completions")
                .with_status(200)
                .with_body(completion_json())
                .expect(0)
                .create_async()
                .await;

            let mut adapter = adapter_at(&server.url());
            let over = ModelOverride {
                max_output_tokens: Some(1000),
                ..ModelOverride::default()
            };
            adapter.model_registry =
                ModelRegistry::new(OPENAI_PROVIDER, &[], fallback_profile(None))
                    .with_overrides(BTreeMap::from([("gpt-4o".to_string(), over)]))
                    .expect("a plain max_output_tokens override is valid");

            // No max_tokens on the request: the 4096 default is what would be sent, and the
            // profile says the model produces at most 1000.
            let err = adapter
                .generate(request_with(unset()))
                .await
                .expect_err("4096 is above the profiled maximum of 1000");
            match err {
                LlmError::InvalidPrompt(message) => {
                    assert!(message.contains("openai"), "{message}");
                    assert!(message.contains("gpt-4o"), "{message}");
                    assert!(message.contains("max_tokens"), "{message}");
                }
                other => panic!("expected InvalidPrompt, got {other:?}"),
            }
            mock.assert_async().await;

            // The streaming path refuses it too.
            let streamed = adapter.generate_stream(request_with(unset())).await;
            assert!(matches!(streamed, Err(LlmError::InvalidPrompt(_))));
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

            // OpenAI has no adapter-level default model (the model arrives per request), so the
            // capability cases measure the adapter-wide declaration the way it applies to an
            // unseeded id: `gpt-4o` resolves to the adapter-wide fallback profile, whose window is
            // the declared 128,000 `get_capabilities` returns. (A seeded id such as `gpt-5.4-nano`
            // legitimately differs from the adapter-wide figure, see
            // `openai_default_model_is_seeded`.)
            fn default_model() -> &'static str {
                "gpt-4o"
            }

            fn provider_name() -> &'static str {
                "openai"
            }

            fn adapter(base_url: &str) -> Arc<dyn LlmPort> {
                Arc::new(
                    OpenAIAdapter::new(OpenAIConfig {
                        api_key: "test-key".to_string(),
                        base_url: base_url.to_string(),
                        organization: None,
                        timeout_seconds: 5,
                        max_retries: 0,
                        max_tokens: Some(4096),
                        context_window: None,
                        model: OPENAI_DEFAULT_MODEL.to_string(),
                        model_overrides: BTreeMap::new(),
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

    /// PARITY-05: refusal, empty output and `tool_calls` on the buffered and streaming paths,
    /// all decided by the shared `classify_completion` (D-09, D-11a).
    mod response_guard_wiring {
        use super::*;
        use futures::StreamExt;
        use mockito::Server;
        use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
        use serde_json::{Value, json};

        fn adapter_at(base_url: &str) -> OpenAIAdapter {
            OpenAIAdapter::new(OpenAIConfig {
                api_key: "test-key".to_string(),
                base_url: base_url.to_string(),
                organization: None,
                timeout_seconds: 5,
                max_retries: 0,
                max_tokens: Some(4096),
                context_window: None,
                model: OPENAI_DEFAULT_MODEL.to_string(),
                model_overrides: BTreeMap::new(),
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
                .unwrap(),
            )
        }

        fn body(message: Value, finish_reason: &str) -> String {
            json!({
                "id": "cmpl-1",
                "model": "gpt-4o",
                "choices": [{"index": 0, "message": message, "finish_reason": finish_reason}],
                "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
            })
            .to_string()
        }

        async fn generate_from(body: String) -> Result<LlmResponse, LlmError> {
            let mut server = Server::new_async().await;
            server
                .mock("POST", "/chat/completions")
                .with_status(200)
                .with_body(body)
                .create_async()
                .await;
            adapter_at(&server.url()).generate(build_request()).await
        }

        async fn stream_from(sse: &str) -> Vec<Result<StreamingResponse, LlmError>> {
            let mut server = Server::new_async().await;
            server
                .mock("POST", "/chat/completions")
                .with_status(200)
                .with_body(sse)
                .create_async()
                .await;
            let stream = adapter_at(&server.url())
                .generate_stream(build_request())
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
        async fn openai_refusal_is_typed_not_a_parse_failure() {
            let result = generate_from(body(
                json!({"role": "assistant", "content": null, "refusal": "I can't help with that."}),
                "stop",
            ))
            .await;
            match result {
                Err(LlmError::Refusal {
                    provider,
                    model,
                    detail,
                    ..
                }) => {
                    assert_eq!(provider, "openai");
                    assert_eq!(model, "gpt-4o");
                    assert_eq!(detail, "I can't help with that.");
                }
                other => panic!("expected Refusal, got {other:?}"),
            }
        }

        #[tokio::test]
        async fn openai_refusal_detail_is_redacted_before_bounding() {
            let long = format!("{} test-key {}", "a".repeat(500), "b".repeat(100));
            let result = generate_from(body(
                json!({"role": "assistant", "content": null, "refusal": long}),
                "stop",
            ))
            .await;
            match result {
                Err(LlmError::Refusal { detail, .. }) => {
                    assert!(!detail.contains("test-key"), "{detail}");
                    assert!(detail.contains("truncated"), "{detail}");
                }
                other => panic!("expected Refusal, got {other:?}"),
            }
        }

        #[tokio::test]
        async fn openai_empty_stop_completion_is_empty_completion() {
            for content in [json!(""), json!("   "), json!(null)] {
                let result = generate_from(body(
                    json!({"role": "assistant", "content": content}),
                    "stop",
                ))
                .await;
                assert!(
                    matches!(result, Err(LlmError::EmptyCompletion { .. })),
                    "got {result:?}"
                );
            }
        }

        /// WR-04: a no-choices body OpenAI billed is an `EmptyCompletion` carrying the bill
        /// on the served model (as the compat engine and Gemini do), not a `ProcessingError`
        /// that drops the usage and reads as `Transience::Unknown` to the retry loop.
        #[tokio::test]
        async fn openai_no_choices_body_is_a_billed_empty_completion() {
            let err = generate_from(
                json!({
                    "id": "cmpl-1",
                    "model": "gpt-4o-served-test",
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
            assert_eq!(billed.model, "gpt-4o-served-test");
            assert_eq!(billed.usage.total_tokens, 40);
            assert_eq!(billed.cost, None, "pricing is the decorator's job");
        }

        #[tokio::test]
        async fn openai_content_filter_with_empty_text_stays_ok() {
            let response = generate_from(body(
                json!({"role": "assistant", "content": ""}),
                "content_filter",
            ))
            .await
            .expect("a content filter is a finish reason, not an error");
            assert!(matches!(
                response.finish_reason,
                FinishReason::ContentFilter
            ));
        }

        #[tokio::test]
        async fn openai_tool_calls_finish_maps_to_function_call() {
            let response = generate_from(body(
                json!({"role": "assistant", "content": null}),
                "tool_calls",
            ))
            .await
            .expect("a tool-call finish carries no text legitimately");
            assert!(matches!(response.finish_reason, FinishReason::FunctionCall));
        }

        #[test]
        fn openai_unknown_finish_reason_stays_an_error_finish() {
            assert!(matches!(
                OpenAIAdapter::convert_finish_reason(Some("mystery")),
                FinishReason::Error(_)
            ));
        }

        #[tokio::test]
        async fn openai_stream_refusal_is_the_last_item() {
            let sse = concat!(
                "data: {\"id\":\"1\",\"choices\":[{\"index\":0,\"delta\":{\"refusal\":\"I can't \"},\"finish_reason\":null}]}\n\n",
                "data: {\"id\":\"1\",\"choices\":[{\"index\":0,\"delta\":{\"refusal\":\"help.\"},\"finish_reason\":\"stop\"}]}\n\n",
                "data: [DONE]\n\n",
            );
            let items = stream_from(sse).await;
            match items.last().unwrap() {
                Err(LlmError::Refusal {
                    provider,
                    model,
                    detail,
                    ..
                }) => {
                    assert_eq!(provider, "openai");
                    assert_eq!(model, "gpt-4o");
                    assert_eq!(detail, "I can't help.");
                }
                other => panic!("expected Refusal as the last item, got {other:?}"),
            }
        }

        /// D-11a on the streaming path ("OpenAI included").
        #[tokio::test]
        async fn openai_empty_stream_is_empty_completion() {
            let sse = concat!(
                "data: {\"id\":\"1\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":null}]}\n\n",
                "data: {\"id\":\"1\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"  \"},\"finish_reason\":\"stop\"}]}\n\n",
                "data: [DONE]\n\n",
            );
            let items = stream_from(sse).await;
            let (last, yielded) = items.split_last().unwrap();
            assert!(
                matches!(last, Err(LlmError::EmptyCompletion { .. })),
                "got {last:?}"
            );
            assert!(yielded.iter().all(|item| item.is_ok()));
            assert!(
                items
                    .iter()
                    .all(|item| !matches!(item, Ok(chunk) if chunk.finish_reason.is_some())),
                "no Ok terminal chunk is emitted"
            );
        }

        #[tokio::test]
        async fn openai_stream_with_text_is_unchanged() {
            let sse = concat!(
                "data: {\"id\":\"1\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hi\"},\"finish_reason\":\"stop\"}]}\n\n",
                "data: [DONE]\n\n",
            );
            let items = stream_from(sse).await;
            let chunks: Vec<StreamingResponse> =
                items.into_iter().map(|item| item.unwrap()).collect();
            assert_eq!(chunks[0].delta, "Hi");
            assert!(matches!(
                chunks.last().unwrap().finish_reason,
                Some(FinishReason::Stop)
            ));
        }

        #[tokio::test]
        async fn openai_content_filter_stream_with_no_text_stays_ok() {
            let sse = concat!(
                "data: {\"id\":\"1\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"content_filter\"}]}\n\n",
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

    // ── Phase 50 (G-50-5b, owner option (a)): the output cap's wire name ──

    /// The OpenAI adapter sends its output cap as `max_completion_tokens` for every model
    /// (owner decision 2026-10-17, UAT test 5). `gpt-5.4-nano` -- the first seeded model --
    /// rejects `max_tokens` with HTTP 400 `unsupported_parameter` (probe 11), and accepts the
    /// new key (probe 12, `50-SEED-PROBE.md`).
    /// Plan 50-23 (G-50-5a): OpenAI's own `total_tokens` is read and reconciled against the
    /// mapped `prompt + completion` on both paths, never copied (D-02).
    mod usage_reconciliation {
        use super::*;
        use crate::usage_reconcile::USAGE_UNRECONCILED_METADATA_KEY;
        use futures::StreamExt;
        use mockito::Server;
        use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
        use serde_json::{Value, json};

        fn adapter_at(base_url: &str) -> OpenAIAdapter {
            OpenAIAdapter::new(OpenAIConfig {
                api_key: "test-key".to_string(),
                base_url: base_url.to_string(),
                organization: None,
                timeout_seconds: 5,
                max_retries: 0,
                max_tokens: Some(4096),
                context_window: None,
                model: OPENAI_DEFAULT_MODEL.to_string(),
                model_overrides: BTreeMap::new(),
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

        async fn generate_with_usage(usage: Value) -> LlmResponse {
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
                        "usage": usage
                    })
                    .to_string(),
                )
                .create_async()
                .await;
            adapter_at(&server.url())
                .generate(build_request())
                .await
                .expect("a well-formed body must parse")
        }

        /// G-50-5a (plan 50-24): the live-measured gpt-5.4-nano usage object
        /// (`50-USAGE-PROBE.md`, 2026-10-18: prompt 26 with 0 cached, completion 4, reasoning
        /// 0, total 30) maps as reported and reconciles with OpenAI's own total.
        #[tokio::test]
        async fn openai_measured_usage_reconciles() {
            let response = generate_with_usage(json!({
                "prompt_tokens": 26,
                "completion_tokens": 4,
                "total_tokens": 30,
                "prompt_tokens_details": {"cached_tokens": 0},
                "completion_tokens_details": {"reasoning_tokens": 0}
            }))
            .await;
            let usage = &response.usage;
            assert_eq!(usage.prompt_tokens, 26);
            assert_eq!(usage.cache_read_tokens, Some(0));
            assert_eq!(usage.completion_tokens, 4);
            assert_eq!(usage.reasoning_tokens, Some(0));
            assert_eq!(usage.total_tokens, 30);
            assert!(
                !response
                    .metadata
                    .contains_key(USAGE_UNRECONCILED_METADATA_KEY),
                "{:?}",
                response.metadata
            );
        }

        #[tokio::test]
        async fn openai_agreeing_total_records_no_finding() {
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
        async fn openai_disagreeing_total_is_recorded_and_not_copied() {
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
        async fn openai_usage_without_a_total_parses_with_no_finding() {
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
        async fn openai_stream_terminal_records_a_disagreeing_total() {
            let sse = concat!(
                "data: {\"id\":\"1\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hi\"},\"finish_reason\":\"stop\"}]}\n\n",
                "data: {\"id\":\"1\",\"choices\":[],\"usage\":{\"prompt_tokens\":5,\"completion_tokens\":3,\"total_tokens\":15}}\n\n",
                "data: [DONE]\n\n",
            );
            let mut server = Server::new_async().await;
            server
                .mock("POST", "/chat/completions")
                .with_status(200)
                .with_body(sse)
                .create_async()
                .await;
            let stream = adapter_at(&server.url())
                .generate_stream(build_request())
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

    mod max_completion_tokens_wiring {
        use super::*;
        use crate::test_support::{mock_completion, server};
        use futures::StreamExt;
        use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
        use serde_json::{Value, json};
        use std::sync::{Arc, Mutex};

        /// A default `PromptItem` for the seeded OpenAI model: nothing about the parameters is
        /// overridden, so the `PromptParameters` default (1000) is the caller's value.
        fn nano_request() -> LlmRequest {
            LlmRequest::new(
                "gpt-5.4-nano",
                PromptItem::new(PromptType::User(UserPrompt {
                    query: "Hello".to_string(),
                    context: None,
                }))
                .expect("a user prompt must build"),
            )
        }

        fn config_at(base_url: &str) -> OpenAIConfig {
            let mut config =
                OpenAIConfig::new("test-key".to_string(), OPENAI_DEFAULT_MODEL.to_string());
            config.base_url = base_url.to_string();
            config
        }

        #[tokio::test]
        async fn openai_gpt_5_4_nano_bodies_carry_max_completion_tokens() {
            let mut mock_server = server().await;
            let captured = mock_completion(&mut mock_server).await;
            let adapter = OpenAIAdapter::new(config_at(&mock_server.url())).unwrap();

            adapter.generate(nano_request()).await.unwrap();
            let body = captured
                .lock()
                .unwrap()
                .take()
                .expect("the mock endpoint received a request");
            assert_eq!(body["model"], "gpt-5.4-nano");
            assert_eq!(body["max_completion_tokens"], 1000, "{body}");
            assert!(body.get("max_tokens").is_none(), "{body}");

            // The streamed body goes through the same `build_request` constructor.
            let mut mock_server = server().await;
            let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
            let slot = Arc::clone(&captured);
            mock_server
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
            let adapter = OpenAIAdapter::new(config_at(&mock_server.url())).unwrap();
            let stream = adapter.generate_stream(nano_request()).await.unwrap();
            let mut stream = Box::into_pin(stream);
            while stream.next().await.is_some() {}
            let text = captured
                .lock()
                .unwrap()
                .take()
                .expect("the mock endpoint received a streamed request");
            let body: Value = serde_json::from_str(&text).unwrap();
            assert_eq!(body["stream"], true, "{body}");
            assert_eq!(body["max_completion_tokens"], 1000, "{body}");
            assert!(body.get("max_tokens").is_none(), "{body}");

            // And so does a structured (json_schema) body.
            let mut mock_server = server().await;
            let captured = mock_completion(&mut mock_server).await;
            let adapter = OpenAIAdapter::new(config_at(&mock_server.url())).unwrap();
            let request = nano_request().with_response_format(ResponseFormat::JsonSchema {
                name: "answer".to_string(),
                schema: json!({"type": "object"}),
                strict: true,
            });
            adapter.generate(request).await.unwrap();
            let body = captured
                .lock()
                .unwrap()
                .take()
                .expect("the mock endpoint received a structured request");
            assert_eq!(body["response_format"]["type"], "json_schema", "{body}");
            assert_eq!(body["max_completion_tokens"], 1000, "{body}");
            assert!(body.get("max_tokens").is_none(), "{body}");
        }
    }
}

/// The PARITY-03 knobs on `OpenAIConfig` (plan 50-08, D-12): each one is read by the adapter.
#[cfg(test)]
mod knob_tests {
    use super::*;
    use crate::test_support::{
        attempts_against_a_failing_endpoint, mock_completion, send_with_max_tokens, server,
    };
    use crate::window::effective_capabilities;

    const UNSEEDED: &str = "gpt-unseeded-test-model";

    fn config(base_url: &str) -> OpenAIConfig {
        let mut config =
            OpenAIConfig::new("test-key".to_string(), OPENAI_DEFAULT_MODEL.to_string());
        config.base_url = base_url.to_string();
        config
    }

    fn window_override(window: u32) -> ModelOverride {
        ModelOverride {
            context_window: Some(window),
            ..ModelOverride::default()
        }
    }

    fn error_of(config: OpenAIConfig) -> String {
        match OpenAIAdapter::new(config) {
            Err(error) => error,
            Ok(_) => panic!("construction must fail"),
        }
    }

    #[test]
    fn openai_config_keeps_the_4096_output_cap_by_default() {
        let config = OpenAIConfig::new("test-key".to_string(), OPENAI_DEFAULT_MODEL.to_string());
        assert_eq!(config.max_tokens, Some(4096));
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.timeout_seconds, 300);
        assert_eq!(config.context_window, None);
        assert!(config.model_overrides.is_empty());
    }

    #[tokio::test]
    async fn openai_max_tokens_default_reaches_the_wire() {
        let mut server = server().await;
        let captured = mock_completion(&mut server).await;
        let url = server.url();

        // The default `new()` gives: 4096 when the request sets none.
        let adapter = OpenAIAdapter::new(config(&url)).unwrap();
        let body = send_with_max_tokens(&adapter, &captured, UNSEEDED, None)
            .await
            .unwrap();
        assert_eq!(body["max_completion_tokens"], 4096, "{body}");
        assert!(body.get("max_tokens").is_none(), "{body}");
        // A request's own value wins.
        let body = send_with_max_tokens(&adapter, &captured, UNSEEDED, Some(512))
            .await
            .unwrap();
        assert_eq!(body["max_completion_tokens"], 512, "{body}");
        assert!(body.get("max_tokens").is_none(), "{body}");

        // The builder replaces the default ...
        let adapter = OpenAIAdapter::new(config(&url).with_max_tokens(2048)).unwrap();
        let body = send_with_max_tokens(&adapter, &captured, UNSEEDED, None)
            .await
            .unwrap();
        assert_eq!(body["max_completion_tokens"], 2048, "{body}");
        assert!(body.get("max_tokens").is_none(), "{body}");

        // ... and `None` sends neither output-cap key at all.
        let mut unbounded = config(&url);
        unbounded.max_tokens = None;
        let adapter = OpenAIAdapter::new(unbounded).unwrap();
        let body = send_with_max_tokens(&adapter, &captured, UNSEEDED, None)
            .await
            .unwrap();
        assert!(body.get("max_completion_tokens").is_none(), "{body}");
        assert!(body.get("max_tokens").is_none(), "{body}");
    }

    #[tokio::test]
    async fn openai_max_retries_is_read_by_the_adapter() {
        // The loop retries up to `max_retries` times after the first attempt.
        for retries in [0_u32, 1] {
            let server = server().await;
            let adapter =
                OpenAIAdapter::new(config(&server.url()).with_max_retries(retries)).unwrap();
            attempts_against_a_failing_endpoint(server, &adapter, UNSEEDED, retries as usize + 1)
                .await;
        }
    }

    #[test]
    fn openai_context_window_override_is_declared() {
        let adapter =
            OpenAIAdapter::new(config("https://example.invalid/v1").with_context_window(500_000))
                .unwrap();
        assert_eq!(adapter.get_capabilities().max_context_tokens, Some(500_000));
        assert_eq!(
            adapter.model_profile(UNSEEDED).unwrap().context_window,
            Some(500_000)
        );
    }

    #[test]
    fn context_window_override_beats_a_seeded_window_on_openai() {
        let base = || config("https://example.invalid/v1");
        let seeded = OpenAIAdapter::new(base()).unwrap();
        assert_eq!(
            seeded.model_profile("gpt-5.4-nano").unwrap().context_window,
            Some(400_000)
        );

        // The operator's adapter-wide statement beats the compiled seeded fact ...
        let adapter = OpenAIAdapter::new(base().with_context_window(500_000)).unwrap();
        assert_eq!(
            adapter
                .model_profile("gpt-5.4-nano")
                .unwrap()
                .context_window,
            Some(500_000)
        );
        assert_eq!(
            effective_capabilities(&adapter, "gpt-5.4-nano").max_context_tokens,
            Some(500_000)
        );

        // ... and a per-model override beats both.
        let adapter = OpenAIAdapter::new(base().with_context_window(500_000).with_model_overrides(
            BTreeMap::from([("gpt-5.4-nano".to_string(), window_override(900_000))]),
        ))
        .unwrap();
        assert_eq!(
            adapter
                .model_profile("gpt-5.4-nano")
                .unwrap()
                .context_window,
            Some(900_000)
        );

        // A window below the seeded maximum output is refused, never clamped.
        let message = error_of(base().with_context_window(100_000));
        for needle in ["openai", "gpt-5.4-nano", "100000", "128000"] {
            assert!(
                message.contains(needle),
                "'{needle}' missing from: {message}"
            );
        }
    }

    #[test]
    fn openai_invalid_override_fails_construction_naming_provider_and_model() {
        let with_override = |over: ModelOverride| {
            config("https://example.invalid/v1")
                .with_model_overrides(BTreeMap::from([(UNSEEDED.to_string(), over)]))
        };
        let adapter = OpenAIAdapter::new(with_override(window_override(900_000))).unwrap();
        assert_eq!(
            adapter.model_profile(UNSEEDED).unwrap().context_window,
            Some(900_000)
        );

        let invalid = [
            window_override(0),
            ModelOverride {
                context_window: Some(1_000),
                max_output_tokens: Some(2_000_000),
                ..ModelOverride::default()
            },
        ];
        for over in invalid {
            let message = error_of(with_override(over));
            assert!(
                message.contains("openai") && message.contains(UNSEEDED),
                "the error must name the provider and the model: {message}"
            );
        }
    }

    #[test]
    fn openai_config_debug_redacts_the_api_key() {
        let config = config("https://api.example.invalid/v1");
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
