//! Anthropic Claude LLM Adapter
//!
//! Provides integration with Anthropic's Claude API.
//! Supports standard completions, streaming, and all core LlmPort functionality.
//! Claude has unique requirements: system messages separate from messages array,
//! max_tokens required, and different message structure.

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use reqwest::{
    Client,
    header::{CONTENT_TYPE, HeaderMap, HeaderValue},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

use paladin_core::platform::container::prompt::PromptType;
use paladin_ports::output::llm_port::{
    FinishReason, LlmError, LlmPort, LlmRequest, LlmResponse, ModelProfile, ParamSupport,
    ProviderCapabilities, StopSupport, StreamingResponse, TokenUsage,
};

use crate::http_status::map_http_status_with_hints;
use crate::rate_limit_headers::{RateLimitHeaderFamily, hints_from_headers};
use crate::redaction::RedactedKey;
use crate::registry::{
    DEFAULT_MAX_RETRIES, ModelOverride, ModelRegistry, config_knob_builders,
    default_timeout_seconds, seeded, seeded_window,
};
use crate::response_guards::{StreamCompletion, bill, classify_completion};
use crate::sampling::{self, FormatLog, OmissionLog, RequestNotes};
use crate::sse::SseLines;
use crate::usage_reconcile::{USAGE_UNRECONCILED_METADATA_KEY, UsageLog};
use paladin_ports::output::rate_limit_hints::RateLimitHints;

/// The provider name this adapter reports through [`LlmPort::get_provider_name`]
/// and stamps on every [`LlmError::ProviderError`] it emits.
const ANTHROPIC_PROVIDER: &str = "anthropic";

/// Default Anthropic model requested when `ANTHROPIC_MODEL` is unset.
///
/// `claude-haiku-5-5`: the operator-identified current general-purpose model (its cheapest
/// current tier), the one Anthropic entry measured end to end by the 2026-10-15 live probe
/// (`registry/seeded.rs`, `verified_on` 2026-10-15). It replaces the retired
/// October-2024 Sonnet id this constant's predecessor named inline, which now answers 404
/// `not_found_error`. Pin `ANTHROPIC_MODEL` to keep any other model.
pub const ANTHROPIC_DEFAULT_MODEL: &str = "claude-haiku-5-5";

/// The Anthropic endpoint [`AnthropicConfig::new`] and [`AnthropicConfig::from_env`] use when no
/// other is chosen, and the compiled default of the layered `llm.anthropic` configuration.
///
/// Replace it with [`AnthropicConfig::with_base_url`] (or `ANTHROPIC_BASE_URL` for `from_env`).
pub const ANTHROPIC_DEFAULT_BASE_URL: &str = "https://api.anthropic.com/v1";

/// The `max_tokens` budget [`AnthropicConfig::new`], [`AnthropicConfig::from_env`] and the
/// layered config path (`config::bridge`) use when neither `ANTHROPIC_MAX_TOKENS`, the config
/// block nor [`AnthropicConfig::with_max_tokens`] sets one (owner decision D-07,
/// `compiled-default`, 2026-10-20: one value on every construction path, whatever the model).
/// The Claude API requires the field, so this adapter always has a value.
pub(crate) const ANTHROPIC_DEFAULT_MAX_TOKENS: u32 = 4096;

/// The exact phrase observed VERBATIM in Anthropic's HTTP 400
/// `invalid_request_error` body when an account has reached its configured
/// API usage limit (live run `4a3b749d`). Matched narrowly and deliberately:
/// a too-broad match here would silently reclassify a genuine
/// malformed-prompt bug as "wait for quota reset" — a swallow-errors
/// anti-pattern with a friendly name (RESEARCH Pitfall #4 / threat
/// T-41-01). Checked AFTER the pre-existing `max_tokens` branch in
/// [`AnthropicAdapter::map_error`] so that branch is never shadowed.
const ANTHROPIC_USAGE_CAP_SIGNATURE: &str = "You have reached your specified API usage limits";

/// The `error.details.error_code` Anthropic documents on a workspace spend-cap `429`
/// (`platform.claude.com/docs/en/api/rate-limits`): the response carries no `retry-after`, and
/// "retrying, including the SDK's automatic retries, fails until access resumes" -- an account
/// wall, not a rate limit, so pacing on it is futile (research Pitfall 6).
const ENFORCED_SPEND_LIMIT_REACHED: &str = "enforced_spend_limit_reached";

/// Whether a `429` body is the spend-cap response rather than a transient rate limit.
///
/// Compares only the short `error.details.error_code` identifier of the parsed JSON; it never
/// renders or stores any body text, and a body that is not JSON (or lacks the field) is simply
/// "not a spend cap".
fn signals_enforced_spend_limit(body: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return false;
    };
    value
        .get("error")
        .and_then(|error| error.get("details"))
        .and_then(|details| details.get("error_code"))
        .and_then(|code| code.as_str())
        == Some(ENFORCED_SPEND_LIMIT_REACHED)
}

/// Parse a `429` response's rate-limit headers. Must run BEFORE `response.text()` consumes the
/// response (research Pitfall 2). Only a 429 is parsed, and no credential header is read.
fn snapshot_rate_limit_hints(
    status: reqwest::StatusCode,
    headers: &HeaderMap,
) -> Option<RateLimitHints> {
    if status != reqwest::StatusCode::TOO_MANY_REQUESTS {
        return None;
    }
    hints_from_headers(
        RateLimitHeaderFamily::Anthropic,
        SystemTime::now(),
        |name| headers.get(name).and_then(|v| v.to_str().ok()),
    )
}

/// Configuration for Anthropic Claude LLM adapter.
///
/// Marked `#[non_exhaustive]` (owner decision, Phase 50 plan 50-08, option-a): build one with
/// [`AnthropicConfig::new`] or [`AnthropicConfig::from_env`] and set fields or call the `with_*`
/// builders, so a later field is not another break for callers.
#[derive(Clone)]
#[non_exhaustive]
pub struct AnthropicConfig {
    /// API key for Anthropic authentication.
    pub api_key: String,
    /// Base URL for Anthropic API.
    pub base_url: String,
    /// The adapter's default model, which selects the timeout default;
    /// [`ANTHROPIC_DEFAULT_MODEL`] when constructed from the environment with `ANTHROPIC_MODEL`
    /// unset. Each request's own `LlmRequest.model` decides what is sent on the wire.
    pub model: String,
    /// Output budget (`max_tokens`, required by the Claude API) sent when a request sets none.
    /// [`AnthropicConfig::new`], [`AnthropicConfig::from_env`] and the layered path all default it
    /// to 4,096 whatever the model; raise it with [`AnthropicConfig::with_max_tokens`] (or
    /// `ANTHROPIC_MAX_TOKENS` for `from_env`). Checked against the model's profiled maximum output exactly as a request's own
    /// `max_tokens` is (ADR-0061 (i)): above it, the request is refused before any HTTP call.
    pub max_tokens: u32,
    /// Request timeout in seconds. [`AnthropicConfig::new`] and [`AnthropicConfig::from_env`]
    /// default it to 300 for a seeded reasoning model and to 60 otherwise (D-13); an explicit
    /// `ANTHROPIC_TIMEOUT_SECONDS`, [`AnthropicConfig::with_timeout_seconds`] or field assignment
    /// always wins. A model marked `reasoning`
    /// only through [`model_overrides`](Self::model_overrides) does not change this default
    /// here: set this field in the same place.
    pub timeout_seconds: u64,
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
}

/// Prints every field except the API key, which is shown as `<redacted>` (D-00a): the key is a
/// public `String`, so a derived `Debug` would put it into any log line or panic message that
/// formats the config.
impl std::fmt::Debug for AnthropicConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnthropicConfig")
            .field("api_key", &RedactedKey(&self.api_key))
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("max_tokens", &self.max_tokens)
            .field("timeout_seconds", &self.timeout_seconds)
            .field("max_retries", &self.max_retries)
            .field("context_window", &self.context_window)
            .field("model_overrides", &self.model_overrides)
            .finish()
    }
}

config_knob_builders!(AnthropicConfig, plain_max_tokens);

impl AnthropicConfig {
    /// Load configuration from environment variables.
    ///
    /// # Environment Variables
    /// - `ANTHROPIC_API_KEY` (required): Anthropic API key
    /// - `ANTHROPIC_BASE_URL` (optional): API base URL
    /// - `ANTHROPIC_MODEL` (optional): Default model
    /// - `ANTHROPIC_MAX_TOKENS` (optional): Default max tokens
    /// - `ANTHROPIC_TIMEOUT_SECONDS` (optional): Request timeout, 300 for a seeded reasoning
    ///   model and 60 otherwise
    ///
    /// # Errors
    /// Returns error if required environment variables are missing or invalid.
    pub fn from_env() -> Result<Self, String> {
        let api_key = env::var("ANTHROPIC_API_KEY")
            .map_err(|_| "ANTHROPIC_API_KEY environment variable not set")?;

        let base_url = env::var("ANTHROPIC_BASE_URL")
            .unwrap_or_else(|_| ANTHROPIC_DEFAULT_BASE_URL.to_string());

        let model =
            env::var("ANTHROPIC_MODEL").unwrap_or_else(|_| ANTHROPIC_DEFAULT_MODEL.to_string());

        let max_tokens = env::var("ANTHROPIC_MAX_TOKENS")
            .unwrap_or_else(|_| ANTHROPIC_DEFAULT_MAX_TOKENS.to_string())
            .parse()
            .map_err(|_| "Invalid ANTHROPIC_MAX_TOKENS value")?;

        let timeout_seconds = match env::var("ANTHROPIC_TIMEOUT_SECONDS") {
            Ok(value) => value
                .parse()
                .map_err(|_| "Invalid ANTHROPIC_TIMEOUT_SECONDS value")?,
            Err(_) => default_timeout_seconds(ANTHROPIC_PROVIDER, &model, 60),
        };

        let config = Self {
            api_key,
            base_url,
            model,
            max_tokens,
            timeout_seconds,
            max_retries: DEFAULT_MAX_RETRIES,
            context_window: None,
            model_overrides: BTreeMap::new(),
        };

        config.validate()?;
        Ok(config)
    }

    /// Create a configuration for `model`, authenticating with `api_key`.
    ///
    /// - `api_key`: the Anthropic API key, never a URL.
    /// - `model`: the adapter's default model. It selects the timeout default (300 s for a
    ///   seeded reasoning model, 60 s otherwise); each request's own `LlmRequest.model`
    ///   decides what is sent on the wire.
    ///
    /// The endpoint is [`ANTHROPIC_DEFAULT_BASE_URL`] (replace it with
    /// [`with_base_url`](Self::with_base_url)); the timeout is 300 s for a seeded reasoning model
    /// and 60 s otherwise (replace it with
    /// [`with_timeout_seconds`](Self::with_timeout_seconds)).
    ///
    /// `max_tokens`, the output budget the Claude API requires on every request, is 4,096 for
    /// every `model`, the same value [`from_env`](Self::from_env) and the layered
    /// `llm.anthropic` configuration use, so the three construction paths agree. A model's
    /// larger seeded maximum output is not used here: ask for it with
    /// [`with_max_tokens`](Self::with_max_tokens). Construction is infallible: an empty key or
    /// model, or a non-`http(s)` endpoint, is refused when the adapter is built.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_llm::anthropic::AnthropicConfig;
    /// use paladin_llm::anthropic::adapter::ANTHROPIC_DEFAULT_BASE_URL;
    ///
    /// let key = "sk-ant-example".to_string();
    /// let config = AnthropicConfig::new(key, "claude-haiku-5-5".to_string()).with_max_tokens(16_000);
    /// assert_eq!(config.base_url, ANTHROPIC_DEFAULT_BASE_URL);
    /// assert_eq!(config.max_tokens, 16_000);
    /// ```
    pub fn new(api_key: String, model: String) -> Self {
        let timeout_seconds = default_timeout_seconds(ANTHROPIC_PROVIDER, &model, 60);
        Self {
            api_key,
            base_url: ANTHROPIC_DEFAULT_BASE_URL.to_string(),
            model,
            max_tokens: ANTHROPIC_DEFAULT_MAX_TOKENS,
            timeout_seconds,
            max_retries: DEFAULT_MAX_RETRIES,
            context_window: None,
            model_overrides: BTreeMap::new(),
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
    /// Resolves the request model to the profile that decides what reaches the wire (ADR-0061).
    model_registry: ModelRegistry,
    /// Which (model, parameter) omissions this instance has already warned about.
    omissions: OmissionLog,
    /// Which (model, adjustment) `response_format` changes this instance has already warned
    /// about.
    format_log: FormatLog,
    /// Which (model, check) usage findings this instance has already warned about (G-50-5a).
    /// Anthropic reports no vendor total, so only the containment checks can fire.
    usage_log: UsageLog,
}

/// What this adapter declares to the port, stated once so the adapter-wide fallback profile and
/// [`LlmPort::get_capabilities`] cannot drift apart.
fn declared_capabilities(context_window: Option<u32>) -> ProviderCapabilities {
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
        // The default model's seeded window (`seeded::ANTHROPIC`, `claude-haiku-5-5`, verified
        // 2026-10-15: listing `max_input_tokens`), so this declaration describes the model a
        // caller gets when it names none.
        // The operator's adapter-wide window (PARITY-03) replaces it when set.
        max_context_tokens: context_window
            .or(seeded_window(seeded::ANTHROPIC, ANTHROPIC_DEFAULT_MODEL)),
        supports_system_messages: true,
        temperature_range: Some((0.0, 1.0)),
    }
}

/// The adapter-wide fallback profile, derived from what this adapter's request builder sends
/// (ADR-0061, D-05): a field it sends maps to `Range`, a field it never sends maps to `NotSent`.
///
/// - `temperature` and `top_p`: `NotSent`. This adapter has never put either on the wire --
///   current Claude models (Opus 4.7 and later) answer `400` to a non-default value -- so the
///   honest fallback states what it sends. An agent that asks for an explicit temperature is
///   therefore refused at build, naming `anthropic`, the model and `temperature`, instead of
///   having the value validated and then silently discarded (the behaviour this replaces).
///   Per-model entries seeded in plan 50-06 may re-open a range for a model the live probe
///   shows accepts one.
/// - both penalties: `NotSent` -- the Messages API has no such field.
/// - `stop`: `Supported { max: None }` -- the Messages API field `stop_sequences`.
/// - `json_object` and `json_schema`: false -- the adapter is prompt-only for JSON today, so a
///   requested format is recorded as `omitted` rather than silently ignored.
fn fallback_profile(context_window: Option<u32>) -> ModelProfile {
    let caps = declared_capabilities(context_window);
    let mut profile = ModelProfile::new()
        .with_temperature(ParamSupport::NotSent)
        .with_top_p(ParamSupport::NotSent)
        .with_frequency_penalty(ParamSupport::NotSent)
        .with_presence_penalty(ParamSupport::NotSent)
        .with_stop_sequences(StopSupport::Supported { max: None })
        .with_json_object(false)
        .with_json_schema(false);
    if let Some(window) = caps.max_context_tokens {
        profile = profile.with_context_window(window);
    }
    profile
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

        // PARITY-03: the operator's adapter-wide window, then the per-model overrides, are laid
        // over the seeded table; either one failing construction names the provider and model.
        let model_registry = ModelRegistry::new(
            ANTHROPIC_PROVIDER,
            seeded::ANTHROPIC,
            fallback_profile(config.context_window),
        )
        .with_context_window(config.context_window)
        .and_then(|registry| registry.with_overrides(config.model_overrides.clone()))
        .map_err(|e| {
            LlmError::AuthenticationError(format!("Invalid Anthropic configuration: {e}"))
        })?;

        Ok(Self {
            client,
            config,
            model_registry,
            omissions: OmissionLog::new(),
            format_log: FormatLog::new(),
            usage_log: UsageLog::new(),
        })
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

    /// Convert LlmRequest to Claude API format, plus the notes the buffered path records on its
    /// response.
    ///
    /// What reaches the wire is decided by the request model's [`ModelProfile`] (ADR-0061,
    /// D-01): [`sampling::apply`] sends `temperature`, `top_p` and `stop_sequences` only when
    /// the caller set them **and** the profile admits them. With the adapter-wide fallback
    /// `temperature` and `top_p` are `NotSent`, so a set value is omitted and recorded (one
    /// `warn!` per instance per model and parameter, and `paladin.params_omitted` on the
    /// response) -- the honest form of what used to be a silent `None`. The effective
    /// `max_tokens` (the caller's, else the configured one) is checked against the profile's
    /// maximum output before anything is sent. A requested `response_format` is routed through
    /// [`sampling::apply_response_format`]; this adapter has no JSON mode, so it is recorded as
    /// `omitted`.
    fn build_request(
        &self,
        request: &LlmRequest,
    ) -> Result<(ClaudeRequest, RequestNotes), LlmError> {
        let mut system_message = None;
        let mut messages = Vec::new();

        match &request.prompt.entity.data.prompt_type {
            PromptType::System(system_prompt) => {
                system_message = Some(system_prompt.instructions.clone());
            }
            PromptType::User(user_prompt) => {
                messages.push(ClaudeMessage {
                    role: "user".to_string(),
                    content: crate::turn::user_turn_text(user_prompt),
                });
            }
            PromptType::Text(text_prompt) => {
                let role = match &text_prompt.role {
                    // A system-role text prompt becomes the `system` field and carries no
                    // message of its own.
                    paladin_core::platform::container::prompt::PromptRole::System => {
                        system_message = Some(text_prompt.content.clone());
                        None
                    }
                    paladin_core::platform::container::prompt::PromptRole::User => Some("user"),
                    paladin_core::platform::container::prompt::PromptRole::Assistant => {
                        Some("assistant")
                    }
                    paladin_core::platform::container::prompt::PromptRole::Function => Some("user"),
                };
                if let Some(role) = role {
                    messages.push(ClaudeMessage {
                        role: role.to_string(),
                        content: text_prompt.content.clone(),
                    });
                }
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

        let params = &request.prompt.entity.data.parameters;
        let profile = self.model_registry.profile(&request.model);

        let max_tokens = params.max_tokens.unwrap_or(self.config.max_tokens);
        sampling::check_max_tokens(
            ANTHROPIC_PROVIDER,
            &request.model,
            &profile,
            Some(max_tokens),
        )?;

        let applied = sampling::apply(&profile, params);
        let params_omitted = self
            .omissions
            .record(ANTHROPIC_PROVIDER, &request.model, &applied);

        // This adapter has no JSON mode (the fallback profile says so), so a requested format
        // is never sent: record it as `omitted` instead of ignoring it silently. The returned
        // effective format is `None` whenever the profile has no JSON mode.
        let (_effective_format, adjustment) =
            sampling::apply_response_format(&profile, request.response_format.as_ref());
        let format_adjusted =
            self.format_log
                .record(ANTHROPIC_PROVIDER, &request.model, adjustment);

        let unprofiled_model = self
            .model_registry
            .note_unprofiled(&request.model)
            .then(|| request.model.clone());

        let body = ClaudeRequest {
            model: request.model.clone(),
            messages,
            system: system_message,
            max_tokens,
            // ADR-0061: the profile decides, not a hard-coded `None`. Current Claude models
            // (e.g. claude-opus-4-8) reject an explicit `temperature` ("deprecated for this
            // model") and restrict other sampling overrides, which is why the adapter-wide
            // fallback states both `NotSent`; `skip_serializing_if` drops a `None` from the
            // request body entirely.
            temperature: applied.temperature,
            top_p: applied.top_p,
            stop_sequences: applied.stop,
            stream: false,
        };
        let notes = RequestNotes {
            params_omitted,
            format_adjusted,
            unprofiled_model,
        };
        Ok((body, notes))
    }

    /// Parse Claude API response into LlmResponse.
    ///
    /// Every outcome is decided by the shared [`classify_completion`] guard (PARITY-05), after
    /// the stop reason has been mapped by name (see [`map_stop_reason`]):
    ///
    /// - `stop_reason: "refusal"` is checked BEFORE emptiness (RESEARCH Pitfall 1). Anthropic
    ///   reports a refusal as HTTP 200 with `content: []`, so an empty check first would report
    ///   it as an empty completion with max-tokens advice. The refusal text is built from
    ///   `stop_details` by [`refusal_text`] and reaches [`LlmError::Refusal`] through
    ///   `diagnostic_excerpt` (redacted, then bounded).
    /// - A response with no text on any other stop reason is [`LlmError::EmptyCompletion`] --
    ///   an empty-string success would be indistinguishable from a valid empty answer to every
    ///   downstream caller, and is exactly what produced a 0-byte `PoC.t.sol` before this
    ///   guard existed. The error also names the content-block types the response carried.
    ///
    /// All text-bearing blocks are recovered in array order by [`concat_text_blocks`], not just
    /// the first, so a text block that follows a `thinking` block is not dropped. The text of
    /// the `thinking` blocks, in the same order, becomes [`LlmResponse::reasoning`] (D-11).
    fn parse_response(
        &self,
        request_id: Uuid,
        response: ClaudeResponse,
    ) -> Result<LlmResponse, LlmError> {
        let content = concat_text_blocks(&response.content);
        let finish_reason = map_stop_reason(response.stop_reason.as_deref());
        let refusal = refusal_text(
            response.stop_reason.as_deref(),
            response.stop_details.as_ref(),
        );

        // G-50-5a: usage is mapped BEFORE the guard, so a call Anthropic billed but the guard
        // classifies as refused or empty carries the same folded usage a success would have,
        // on the served model the response names.
        let usage = map_claude_usage(&response.usage);

        classify_completion(
            ANTHROPIC_PROVIDER,
            &response.model,
            &content,
            &finish_reason,
            refusal.as_deref(),
            &self.config.api_key,
        )
        .map_err(|err| {
            bill(
                with_block_census(err, &response.content, &self.config.api_key),
                &response.model,
                Some(usage.clone()),
            )
        })?;

        // `with_reasoning` stores blank text (every captured thinking block carries an empty
        // `thinking` string) as `None`.
        Ok(
            LlmResponse::new(request_id, response.model, content, finish_reason, usage)
                .with_reasoning(concat_thinking_blocks(&response.content)),
        )
    }

    /// Map a non-2xx Anthropic response to [`LlmError`] with no rate-limit headers.
    ///
    /// A thin wrapper over [`Self::map_error_with_hints`], kept so the status-mapping unit
    /// tests read the same as before PACE-01. Production paths call the hints variant.
    #[cfg(test)]
    fn map_error(&self, status: u16, body: &str) -> LlmError {
        self.map_error_with_hints(status, body, None)
    }

    /// Map a non-2xx Anthropic response to [`LlmError`], carrying the parsed rate-limit
    /// headers of a `429` (PACE-01).
    ///
    /// Two Anthropic-specific pre-checks run first; everything else is the
    /// crate-wide [`map_http_status_with_hints`] (Phase 25 D-03, FT-FR-01), which
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
    /// - a `429` whose body carries `error.details.error_code ==
    ///   "enforced_spend_limit_reached"` is the workspace spend cap, not a rate limit
    ///   (research Pitfall 6): it maps to the permanent [`LlmError::UsageLimitExceeded`], so it
    ///   is neither retried nor paced. Any other `429` carries `hints` through the generic arm.
    fn map_error_with_hints(
        &self,
        status: u16,
        body: &str,
        hints: Option<RateLimitHints>,
    ) -> LlmError {
        match status {
            429 if signals_enforced_spend_limit(body) => LlmError::UsageLimitExceeded {
                provider: ANTHROPIC_PROVIDER.to_string(),
                regain_hint: None,
            },
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
            _ => map_http_status_with_hints(
                ANTHROPIC_PROVIDER,
                status,
                body,
                &self.config.api_key,
                hints,
            ),
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
                    //
                    // `RateLimitExceeded` is surfaced on the FIRST 429 (D-02, PACE-02): one
                    // layer -- the Cadence -- sees every 429 the instant it happens, so
                    // `Retry-After` is honoured on the first hit and no thrash hides below
                    // the decorator. Network and 5xx failures are still retried here.
                    if matches!(
                        e,
                        LlmError::AuthenticationError(_)
                            | LlmError::InvalidPrompt(_)
                            | LlmError::EmptyCompletion { .. }
                            | LlmError::Refusal { .. }
                            | LlmError::UsageLimitExceeded { .. }
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
}

#[async_trait]
impl LlmPort for AnthropicAdapter {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let (claude_request, notes) = self.build_request(&request)?;
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
                // Snapshot the 429 headers BEFORE `.text()` consumes the response
                // (research Pitfall 2).
                let hints = snapshot_rate_limit_hints(response.status(), response.headers());
                let body = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                return Err(self.map_error_with_hints(status, &body, hints));
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

        let mut response = self
            .execute_with_retry(operation, self.config.max_retries)
            .await?;
        // The request-time notes (ADR-0061 visibility rule) ride on the successful response.
        response.metadata = notes.into_metadata();
        // G-50-5a: Anthropic reports no total of its own, so the mapped usage is checked for
        // containment only (`vendor_total = None`); a finding is warned once and recorded.
        if let Some(value) =
            self.usage_log
                .record(ANTHROPIC_PROVIDER, &request.model, &response.usage, None)
        {
            response
                .metadata
                .insert(USAGE_UNRECONCILED_METADATA_KEY.to_string(), value);
        }
        Ok(response)
    }

    /// Stream a completion.
    ///
    /// The stream's real stop reason and refusal details are held from `message_delta` and
    /// applied on the terminal `message_stop` chunk (RESEARCH Pitfall 2), where the shared
    /// `classify_completion` rule runs on the accumulated text. A refusal (which may arrive
    /// after partial output) therefore ends the stream with `Err(`[`LlmError::Refusal`]`)`, and
    /// a stream that carried no text ends with `Err(`[`LlmError::EmptyCompletion`]`)` (D-11a).
    /// Chunks already yielded stay yielded: partial text and `thinking_delta` reasoning are not
    /// withdrawn, only the terminal item becomes the typed error. That error item carries the
    /// merged `message_start`/`message_delta` usage as its billed usage
    /// ([`LlmError::billed_usage`], G-50-5a) on the request model, or `None` when the stream
    /// reported no usage. A `thinking_delta` is yielded as
    /// an empty-delta chunk carrying [`StreamingResponse::reasoning`], in wire order.
    async fn generate_stream(
        &self,
        request: LlmRequest,
    ) -> Result<Box<dyn Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError> {
        // ADR-0061 (h), owner reading R-1 (option-b): the omissions, format changes and
        // unprofiled model `build_request` `warn!`s about are also stamped on the stream's
        // terminal chunk (`message_stop`, only that chunk), as the buffered path stamps
        // `LlmResponse.metadata`.
        let (mut claude_request, notes) = self.build_request(&request)?;
        let stream_metadata = notes.into_metadata();
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
            // Same header snapshot as the generate path, taken before the body read.
            let hints = snapshot_rate_limit_hints(response.status(), response.headers());
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(self.map_error_with_hints(status, &body, hints));
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
        // The real stop reason and refusal details, held beside the usage until `message_stop`
        // (RESEARCH Pitfall 2), the response model for naming it in an error, and the
        // accumulator the shared guard classifies at the terminal chunk.
        let mut held_stop_reason: Option<String> = None;
        let mut held_stop_details: Option<AnthropicStopDetails> = None;
        let mut model = claude_request.model.clone();
        let mut completion = StreamCompletion::default();
        let api_key = self.config.api_key.clone();
        // G-50-5a: the terminal usage is checked for containment (no vendor total) into this
        // instance's log, keyed on the request model as the buffered path is.
        let usage_log = self.usage_log.clone();
        let usage_model = claude_request.model.clone();

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
                            let Some(json_str) = line.strip_prefix("data: ") else {
                                continue;
                            };
                            if json_str.trim() == "[DONE]" {
                                continue;
                            }

                            let Ok(event) = serde_json::from_str::<ClaudeStreamEvent>(json_str)
                            else {
                                // Never skipped silently: name the event type only, never its body
                                // (it can echo a credential) and never serde's message (it can quote
                                // the offending value).
                                log::warn!(
                                    target: "paladin::anthropic",
                                    "skipping an unparsable stream event (type: {})",
                                    unparsable_event_type(json_str)
                                );
                                continue;
                            };

                            match event.event_type.as_str() {
                                "content_block_delta" => {
                                    if let Some(delta) = event.delta {
                                        if let Some(text) = delta.text {
                                            completion.observe_content(&text);
                                            items.push(Ok(StreamingResponse::delta(text)));
                                        }
                                        if let Some(thinking) = delta.thinking {
                                            items.push(Ok(StreamingResponse::delta("")
                                                .with_reasoning(thinking)));
                                        }
                                    }
                                }
                                "message_start" => {
                                    if let Some(message) = event.message {
                                        if let Some(started) = message.model {
                                            model = started;
                                        }
                                        if let Some(usage) = message.usage {
                                            held_usage = Some(merge_claude_stream_usage(
                                                held_usage.take(),
                                                usage,
                                            ));
                                        }
                                    }
                                }
                                "message_delta" => {
                                    if let Some(usage) = event.usage {
                                        held_usage = Some(merge_claude_stream_usage(
                                            held_usage.take(),
                                            usage,
                                        ));
                                    }
                                    if let Some(delta) = event.delta {
                                        if let Some(reason) = delta.stop_reason {
                                            held_stop_reason = Some(reason);
                                        }
                                        if let Some(details) = delta.stop_details {
                                            held_stop_details = Some(details);
                                        }
                                    }
                                }
                                "message_stop" => {
                                    // A stream that never reported a stop reason keeps the
                                    // pre-50-13 terminal `Stop`; a reported one is mapped by name.
                                    let finish = match held_stop_reason.as_deref() {
                                        None => FinishReason::Stop,
                                        reason => map_stop_reason(reason),
                                    };
                                    if let Some(refusal) = refusal_text(
                                        held_stop_reason.as_deref(),
                                        held_stop_details.as_ref(),
                                    ) {
                                        completion.observe_refusal(&refusal);
                                    }
                                    match completion.finish(
                                        ANTHROPIC_PROVIDER,
                                        &model,
                                        &finish,
                                        &api_key,
                                    ) {
                                        Ok(()) => {
                                            let mut terminal = StreamingResponse::terminal(finish);
                                            if let Some(usage) = held_usage.take() {
                                                let mapped = map_claude_usage(&usage);
                                                // Only the `Ok` terminal chunk carries a finding;
                                                // an `Err` terminal carries no metadata.
                                                if let Some(value) = usage_log.record(
                                                    ANTHROPIC_PROVIDER,
                                                    &usage_model,
                                                    &mapped,
                                                    None,
                                                ) {
                                                    terminal = terminal.with_metadata(
                                                        USAGE_UNRECONCILED_METADATA_KEY,
                                                        value,
                                                    );
                                                }
                                                terminal = terminal.with_usage(mapped);
                                            }
                                            for (key, value) in &stream_metadata {
                                                terminal = terminal
                                                    .with_metadata(key.as_str(), value.as_str());
                                            }
                                            items.push(Ok(terminal));
                                        }
                                        // G-50-5a: the error carries the merged
                                        // `message_start`/`message_delta` usage as its bill on
                                        // the request model, `None` when none arrived.
                                        Err(err) => items.push(Err(bill(
                                            err,
                                            &usage_model,
                                            held_usage.take().map(|usage| map_claude_usage(&usage)),
                                        ))),
                                    }
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

    /// Whether `model` is in the dated Anthropic table (`seeded::ANTHROPIC`). The table lists
    /// the models the live probe measured, so a current model that was not probed is reported
    /// `false` here yet still runs, on the adapter-wide fallback profile (D-07).
    async fn validate_model(&self, model: &str) -> Result<bool, LlmError> {
        Ok(seeded::ANTHROPIC.iter().any(|entry| entry.model == model))
    }

    async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
        Ok(seeded::ANTHROPIC
            .iter()
            .map(|entry| entry.model.to_string())
            .collect())
    }

    fn get_provider_name(&self) -> &'static str {
        ANTHROPIC_PROVIDER
    }

    fn get_capabilities(&self) -> ProviderCapabilities {
        declared_capabilities(self.config.context_window)
    }

    fn model_profile(&self, model: &str) -> Option<ModelProfile> {
        Some(self.model_registry.profile(model))
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
    /// The caller's stop sequences, in declared order with duplicates kept, when the model's
    /// profile admits their count (ADR-0061 (a), D-11a). Absent otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
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
    /// Present (possibly with `null` members) when the model refused; see [`AnthropicStopDetails`].
    #[serde(default)]
    stop_details: Option<AnthropicStopDetails>,
    usage: ClaudeUsage,
}

/// The `stop_details` object that accompanies `stop_reason: "refusal"` (RESEARCH, cited vendor
/// fact): `{ type, category, explanation }`. `category` is one of `cyber`, `bio`,
/// `frontier_llm`, `general_harms`, and either member may be `null`.
///
/// Buffered responses carry it at the top level, streamed ones on `message_delta.delta`. Both
/// members are vendor-supplied text, so they only ever reach an error through
/// `diagnostic_excerpt` (redact, then bound).
#[derive(Debug, Default, Deserialize)]
struct AnthropicStopDetails {
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    explanation: Option<String>,
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
/// [`with_block_census`], so it is genuinely load-bearing, not dead code kept only for
/// documentation. `thinking` is present only on a `thinking` block and feeds
/// [`LlmResponse::reasoning`] through [`concat_thinking_blocks`] (D-11).
#[derive(Debug, Deserialize)]
struct ClaudeContent {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    thinking: Option<String>,
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

/// The `type` of an event line that failed to parse as a [`ClaudeStreamEvent`], for the
/// `warn!` that replaces the old silent skip. `unknown` when the line is not JSON or has no
/// string `type`; otherwise only a bounded run of identifier characters, so the log line can
/// never carry the event's body.
fn unparsable_event_type(json_str: &str) -> String {
    serde_json::from_str::<serde_json::Value>(json_str)
        .ok()
        .and_then(|value| value.get("type")?.as_str().map(sanitize_token))
        .filter(|event_type| !event_type.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Concatenate the text of every `thinking` content block, in array order (D-11).
///
/// Only a block that actually carries a `thinking` string contributes; `redacted_thinking`
/// (opaque `data`) and every other block type are skipped, the same degrade-to-ignored rule
/// [`concat_text_blocks`] follows. No separator is inserted, mirroring the streaming path.
fn concat_thinking_blocks(content: &[ClaudeContent]) -> String {
    content
        .iter()
        .filter_map(|block| block.thinking.as_deref())
        .collect()
}

/// Map an Anthropic `stop_reason` to a [`FinishReason`], naming every documented value
/// (RESEARCH State of the Art):
///
/// | `stop_reason` | finish reason |
/// |---|---|
/// | `end_turn`, `stop_sequence` | `Stop` |
/// | `max_tokens`, `model_context_window_exceeded` | `Length` (A1-50-13: truncation) |
/// | `tool_use` | `FunctionCall` |
/// | `pause_turn` | `Error("pause_turn")` |
/// | `refusal` | `Error("refusal")`, never read: [`refusal_text`] makes the guard return
///   [`LlmError::Refusal`] first |
/// | any other value | `Error("unknown: <value>")` |
/// | absent | `Error("unknown")` |
///
/// The unknown value is wrapped as vendor text in a finish reason (not an error message), so it
/// is bounded to a short run of identifier characters; the full value is never trusted.
fn map_stop_reason(stop_reason: Option<&str>) -> FinishReason {
    match stop_reason {
        Some("end_turn" | "stop_sequence") => FinishReason::Stop,
        Some("max_tokens" | "model_context_window_exceeded") => FinishReason::Length,
        Some("tool_use") => FinishReason::FunctionCall,
        Some("pause_turn") => FinishReason::Error("pause_turn".to_string()),
        Some("refusal") => FinishReason::Error("refusal".to_string()),
        Some(other) => FinishReason::Error(format!("unknown: {}", sanitize_token(other))),
        None => FinishReason::Error("unknown".to_string()),
    }
}

/// Keep only the leading identifier characters of a vendor-supplied token (at most 64), so a
/// hostile or malformed value cannot smuggle control characters or a secret into a finish
/// reason or a log line.
fn sanitize_token(token: &str) -> String {
    token
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        .take(64)
        .collect()
}

/// The refusal text for [`classify_completion`], or `None` when the stop reason is not
/// `refusal`.
///
/// Built from `stop_details.category` and `stop_details.explanation`, joined with `"; "` (either
/// may be `null`); when both are absent the text is `stop_reason=refusal`, so a refusal is never
/// anonymous. The result is raw vendor text -- the guard redacts it before bounding it.
fn refusal_text(
    stop_reason: Option<&str>,
    details: Option<&AnthropicStopDetails>,
) -> Option<String> {
    if stop_reason != Some("refusal") {
        return None;
    }
    let category = details
        .and_then(|d| d.category.as_deref())
        .filter(|c| !c.trim().is_empty())
        .map(|c| format!("category={c}"));
    let explanation = details
        .and_then(|d| d.explanation.as_deref())
        .filter(|e| !e.trim().is_empty())
        .map(str::to_string);
    let parts: Vec<String> = category.into_iter().chain(explanation).collect();
    if parts.is_empty() {
        Some("stop_reason=refusal".to_string())
    } else {
        Some(parts.join("; "))
    }
}

/// Add the content-block census to an [`LlmError::EmptyCompletion`] so the reader learns "the
/// model spent its whole budget thinking" in one step (the guard's own payload names only the
/// finish reason). Any other error passes through untouched. The block-type names are vendor
/// text, so they go through `diagnostic_excerpt`.
fn with_block_census(mut err: LlmError, content: &[ClaudeContent], api_key: &str) -> LlmError {
    // The detail is extended in place, so any other field the variant carries is kept as is.
    if let LlmError::EmptyCompletion { detail, .. } = &mut err {
        let block_types = content
            .iter()
            .map(|block| block.content_type.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        *detail = format!(
            "{detail}; {} content block(s) (types: [{}])",
            content.len(),
            crate::redaction::diagnostic_excerpt(&block_types, api_key)
        );
    }
    err
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
    /// The response model, used to name the provider's model in a refusal or empty completion.
    #[serde(default)]
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ClaudeDelta {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    delta_type: Option<String>,
    /// A `text_delta`'s text.
    #[serde(default)]
    text: Option<String>,
    /// A `thinking_delta`'s text (D-11).
    #[serde(default)]
    thinking: Option<String>,
    /// On `message_delta`: the real stop reason, held until `message_stop` (Pitfall 2).
    #[serde(default)]
    stop_reason: Option<String>,
    /// On `message_delta`: the refusal details, when `stop_reason` is `refusal`.
    #[serde(default)]
    stop_details: Option<AnthropicStopDetails>,
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
            "claude-3-5-sonnet-20241022".to_string(),
        );
        assert!(config.validate().is_ok());

        let config_empty_key =
            AnthropicConfig::new("".to_string(), "claude-3-5-sonnet-20241022".to_string());
        assert!(config_empty_key.validate().is_err());

        let config_invalid_url = AnthropicConfig::new(
            "sk-ant-test123".to_string(),
            "claude-3-5-sonnet-20241022".to_string(),
        )
        .with_base_url("invalid-url".to_string());
        assert!(config_invalid_url.validate().is_err());

        let config_zero_tokens = AnthropicConfig::new(
            "sk-ant-test123".to_string(),
            "claude-3-5-sonnet-20241022".to_string(),
        )
        .with_max_tokens(0);
        assert!(config_zero_tokens.validate().is_err());
    }

    #[tokio::test]
    async fn test_anthropic_adapter_creation() {
        let config = AnthropicConfig::new(
            "sk-ant-test123".to_string(),
            "claude-3-5-sonnet-20241022".to_string(),
        );
        assert!(AnthropicAdapter::new(config).is_ok());
    }

    #[tokio::test]
    async fn test_anthropic_provider_capabilities() {
        let config = AnthropicConfig::new(
            "sk-ant-test123".to_string(),
            "claude-3-5-sonnet-20241022".to_string(),
        );
        let adapter = AnthropicAdapter::new(config).unwrap();
        let capabilities = adapter.get_capabilities();

        assert!(capabilities.supports_streaming);
        // `LlmRequest` has no field through which a tool definition could travel, and
        // this adapter neither sends `tools` nor parses `tool_calls` (WEB-03, D-14).
        assert!(!capabilities.supports_tool_calling);
        assert!(capabilities.supports_vision);
        assert!(capabilities.supports_system_messages);
        assert_eq!(capabilities.max_context_tokens, Some(1_000_000));
        assert_eq!(capabilities.temperature_range, Some((0.0, 1.0)));
        assert_eq!(adapter.get_provider_name(), "anthropic");
    }

    // ── Dated registry wiring (plan 50-06, D-05/D-08) ─────────────────────

    fn seeded_test_adapter() -> AnthropicAdapter {
        AnthropicAdapter::new(AnthropicConfig::new(
            "test-key".to_string(),
            ANTHROPIC_DEFAULT_MODEL.to_string(),
        ))
        .expect("a valid config builds")
    }

    #[test]
    fn anthropic_default_model_is_seeded() {
        let (profile, origin) = seeded_test_adapter()
            .model_registry
            .resolve(ANTHROPIC_DEFAULT_MODEL);
        assert_eq!(
            origin,
            crate::registry::ProfileOrigin::Seeded {
                verified_on: "2026-10-15"
            }
        );
        assert_eq!(profile.verified_on.as_deref(), Some("2026-10-15"));
        assert_eq!(profile.context_window, Some(1_000_000));
    }

    #[test]
    fn anthropic_default_model_is_the_operator_identified_current_model() {
        assert_eq!(ANTHROPIC_DEFAULT_MODEL, "claude-haiku-5-5");
    }

    #[test]
    fn anthropic_capabilities_report_the_default_models_seeded_window() {
        assert_eq!(
            seeded_test_adapter().get_capabilities().max_context_tokens,
            Some(1_000_000)
        );
    }

    #[test]
    fn anthropic_unlisted_model_still_resolves_the_adapter_default() {
        // Not `claude-opus-4-8`: Phase 50.1 (SURFACE-06) seeded it.
        let (_, origin) = seeded_test_adapter()
            .model_registry
            .resolve("claude-unlisted-test-model");
        assert_eq!(origin, crate::registry::ProfileOrigin::AdapterDefault);
    }

    /// SURFACE-06 (D-14): `claude-opus-4-8`, the model web3sec runs, is in the dated table, so
    /// the adapter resolves it as seeded (2026-10-20, `50.1-SEED-PROBE.md`) and never as the
    /// adapter-wide fallback.
    #[test]
    fn anthropic_opus_4_8_resolves_seeded_through_the_adapter() {
        let (profile, origin) = seeded_test_adapter()
            .model_registry
            .resolve("claude-opus-4-8");
        assert_eq!(
            origin,
            crate::registry::ProfileOrigin::Seeded {
                verified_on: "2026-10-20"
            }
        );
        assert_eq!(profile.context_window, Some(1_000_000));
        assert_eq!(profile.max_output_tokens, Some(128_000));
    }

    #[tokio::test]
    async fn anthropic_available_models_are_the_seeded_table_ids_in_order() {
        let models = seeded_test_adapter().get_available_models().await.unwrap();
        let table: Vec<String> = seeded::ANTHROPIC
            .iter()
            .map(|entry| entry.model.to_string())
            .collect();
        assert_eq!(models, table);
        assert!(models.contains(&ANTHROPIC_DEFAULT_MODEL.to_string()));
    }

    #[tokio::test]
    async fn anthropic_validate_model_accepts_a_seeded_id_and_rejects_a_claude_3_id() {
        let adapter = seeded_test_adapter();
        assert!(
            adapter
                .validate_model(ANTHROPIC_DEFAULT_MODEL)
                .await
                .unwrap()
        );
        for retired in [
            "claude-3-5-sonnet-20241022",
            "claude-2.1",
            "claude-instant-1.2",
        ] {
            assert!(
                !adapter.validate_model(retired).await.unwrap(),
                "{retired} is retired and must not validate"
            );
        }
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
            "claude-3-5-sonnet-20241022".to_string(),
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
            Err(LlmError::EmptyCompletion { detail: msg, .. }) => {
                assert!(
                    msg.contains("thinking"),
                    "message must name the block type it saw: {msg}"
                );
                assert!(
                    msg.contains("max_tokens"),
                    "message must name the stop reason it arrived with: {msg}"
                );
            }
            other => panic!("expected Err(LlmError::EmptyCompletion {{ .. }}), got {other:?}"),
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
                        Err(LlmError::empty_completion("no text".to_string()))
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
            LlmError::RateLimitExceeded { .. }
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

    // ── Phase 43 (PACE-01, D-02, Pitfall 6): first-429 surfacing and header hints ──

    #[tokio::test(start_paused = true)]
    async fn test_execute_with_retry_invokes_operation_exactly_once_on_rate_limit_exceeded() {
        let adapter = test_adapter();
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
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "a 429 is surfaced on the first attempt (D-02): the Cadence above paces the next call"
        );
    }

    /// Phase 50 (PARITY-08): a present context is appended to the query by the shared
    /// `user_turn_text` helper; an absent one leaves the query unchanged.
    #[test]
    fn anthropic_user_turn_appends_context() {
        use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};

        let adapter = test_adapter();
        let user_turn = |context: Option<&str>| -> serde_json::Value {
            let request = LlmRequest::new(
                "claude-opus-4-8",
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

    mod rate_limit_wiring {
        use super::*;
        use mockito::Server;
        use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
        use paladin_ports::output::rate_limit_hints::{RateLimitDimensionKind, RetryDelaySource};

        const SECS: fn(u64) -> Duration = Duration::from_secs;

        /// A fixed response `Date` so every RFC 3339 reset below has an exact expected delta
        /// (the resets are measured from the response `Date` when it parses).
        const RESPONSE_DATE: &str = "Tue, 14 Nov 2023 22:11:40 GMT";

        fn adapter_at(base_url: &str) -> AnthropicAdapter {
            AnthropicAdapter::new(
                AnthropicConfig::new("sk-ant-test123".to_string(), "claude-opus-4-8".to_string())
                    .with_base_url(base_url),
            )
            .expect("test config must build a valid adapter")
        }

        fn build_request() -> LlmRequest {
            LlmRequest::new(
                "claude-opus-4-8",
                PromptItem::new(PromptType::User(UserPrompt {
                    query: "Hello".to_string(),
                    context: None,
                }))
                .expect("a user prompt must build"),
            )
        }

        fn with_ratelimit_headers(mock: mockito::Mock, retry_after: Option<&str>) -> mockito::Mock {
            let mock = mock
                .with_header("date", RESPONSE_DATE)
                .with_header("anthropic-ratelimit-requests-limit", "50")
                .with_header("anthropic-ratelimit-requests-remaining", "0")
                .with_header("anthropic-ratelimit-requests-reset", "2023-11-14T22:13:30Z")
                .with_header("anthropic-ratelimit-tokens-limit", "40000")
                .with_header("anthropic-ratelimit-tokens-remaining", "39000")
                .with_header("anthropic-ratelimit-tokens-reset", "2023-11-14T22:13:25Z")
                .with_header("anthropic-ratelimit-input-tokens-limit", "30000")
                .with_header("anthropic-ratelimit-input-tokens-remaining", "29000")
                .with_header(
                    "anthropic-ratelimit-input-tokens-reset",
                    "2023-11-14T22:13:21Z",
                )
                .with_header("anthropic-ratelimit-output-tokens-limit", "8000")
                .with_header("anthropic-ratelimit-output-tokens-remaining", "0")
                .with_header(
                    "anthropic-ratelimit-output-tokens-reset",
                    "2023-11-14T22:14:20Z",
                );
            match retry_after {
                Some(v) => mock.with_header("retry-after", v),
                None => mock,
            }
        }

        fn assert_four_dimensions(err: &LlmError) {
            let hints = err.rate_limit_hints().expect("the 429 headers were parsed");
            for (kind, limit, remaining, reset) in [
                (RateLimitDimensionKind::Requests, 50, 0, 110),
                (RateLimitDimensionKind::Tokens, 40_000, 39_000, 105),
                (RateLimitDimensionKind::InputTokens, 30_000, 29_000, 101),
                (RateLimitDimensionKind::OutputTokens, 8_000, 0, 160),
            ] {
                let dim = hints.dimension(kind);
                assert_eq!(dim.limit(), Some(limit), "{kind:?}");
                assert_eq!(dim.remaining(), Some(remaining), "{kind:?}");
                assert_eq!(dim.reset_after(), Some(SECS(reset)), "{kind:?}");
            }
        }

        const RATE_LIMIT_BODY: &str = r#"{"type":"error","error":{"type":"rate_limit_error","message":"Number of request tokens has exceeded your per-minute rate limit"}}"#;

        #[tokio::test]
        async fn anthropic_429_is_surfaced_once_with_its_retry_delay_and_four_dimensions() {
            let mut server = Server::new_async().await;
            let mock = with_ratelimit_headers(
                server
                    .mock("POST", "/messages")
                    .with_status(429)
                    .with_body(RATE_LIMIT_BODY),
                Some("7"),
            )
            .expect(1)
            .create_async()
            .await;

            // `execute_with_retry(.., max_retries)` with the default of 3: three attempts are allowed; a
            // 429 must use only one.
            let started = std::time::Instant::now();
            let err = adapter_at(&server.url())
                .generate(build_request())
                .await
                .expect_err("a 429 is an error");

            assert!(matches!(err, LlmError::RateLimitExceeded { .. }), "{err:?}");
            assert_eq!(err.retry_after(), Some(SECS(7)));
            assert_eq!(
                err.rate_limit_hints()
                    .and_then(|h| h.explicit_retry_after()),
                Some((SECS(7), RetryDelaySource::RetryAfter))
            );
            assert_four_dimensions(&err);
            assert!(
                started.elapsed() < Duration::from_millis(900),
                "a surfaced 429 must not sleep through the retry back-off ({:?})",
                started.elapsed()
            );
            // The raw RFC 3339 strings never reach a rendered error (T-43-10).
            for rendered in [format!("{err}"), format!("{err:?}")] {
                assert!(!rendered.contains("2023-11-14T"), "{rendered}");
            }
            mock.assert_async().await;
        }

        #[tokio::test]
        async fn anthropic_429_without_retry_after_uses_the_exhausted_reset() {
            let mut server = Server::new_async().await;
            let mock = with_ratelimit_headers(
                server
                    .mock("POST", "/messages")
                    .with_status(429)
                    .with_body(RATE_LIMIT_BODY),
                None,
            )
            .expect(1)
            .create_async()
            .await;

            let err = adapter_at(&server.url())
                .generate(build_request())
                .await
                .expect_err("a 429 is an error");

            // Requests (110 s) and output tokens (160 s) are exhausted: the larger stands in.
            assert_eq!(err.retry_after(), Some(SECS(160)));
            assert_eq!(
                err.rate_limit_hints()
                    .and_then(|h| h.effective_retry_after()),
                Some((SECS(160), RetryDelaySource::ResetHeader))
            );
            mock.assert_async().await;
        }

        #[tokio::test]
        async fn anthropic_stream_429_carries_the_same_hints() {
            let mut server = Server::new_async().await;
            let mock = with_ratelimit_headers(
                server
                    .mock("POST", "/messages")
                    .with_status(429)
                    .with_body(RATE_LIMIT_BODY),
                Some("7"),
            )
            .expect(1)
            .create_async()
            .await;

            let result = adapter_at(&server.url())
                .generate_stream(build_request())
                .await;
            let err = match result {
                Err(err) => err,
                Ok(_) => panic!("expected Err(RateLimitExceeded), got Ok(<stream>)"),
            };

            assert!(matches!(err, LlmError::RateLimitExceeded { .. }), "{err:?}");
            assert_eq!(err.retry_after(), Some(SECS(7)));
            assert_four_dimensions(&err);
            mock.assert_async().await;
        }

        #[tokio::test]
        async fn anthropic_429_with_no_rate_limit_header_carries_no_hints() {
            let mut server = Server::new_async().await;
            let mock = server
                .mock("POST", "/messages")
                .with_status(429)
                .with_body(RATE_LIMIT_BODY)
                .expect(1)
                .create_async()
                .await;

            let err = adapter_at(&server.url())
                .generate(build_request())
                .await
                .expect_err("a 429 is an error");

            assert!(matches!(err, LlmError::RateLimitExceeded { .. }), "{err:?}");
            assert_eq!(err.retry_after(), None);
            assert!(err.rate_limit_hints().is_none());
            mock.assert_async().await;
        }

        const SPEND_CAP_BODY: &str = r#"{"type":"error","error":{"type":"rate_limit_error","message":"You have reached your workspace spend limit.","details":{"error_code":"enforced_spend_limit_reached"}}}"#;

        #[tokio::test]
        async fn anthropic_spend_cap_429_maps_to_usage_limit_exceeded_once() {
            let mut server = Server::new_async().await;
            // No retry-after, exactly as Anthropic documents for the spend-cap 429.
            let mock = server
                .mock("POST", "/messages")
                .with_status(429)
                .with_body(SPEND_CAP_BODY)
                .expect(1)
                .create_async()
                .await;

            let err = adapter_at(&server.url())
                .generate(build_request())
                .await
                .expect_err("a spend-cap 429 is an error");

            match &err {
                LlmError::UsageLimitExceeded {
                    provider,
                    regain_hint,
                } => {
                    assert_eq!(provider, "anthropic");
                    assert_eq!(*regain_hint, None);
                }
                other => panic!("expected UsageLimitExceeded, got {other:?}"),
            }
            mock.assert_async().await;
        }

        #[tokio::test]
        async fn anthropic_stream_spend_cap_429_maps_to_usage_limit_exceeded() {
            let mut server = Server::new_async().await;
            let mock = server
                .mock("POST", "/messages")
                .with_status(429)
                .with_body(SPEND_CAP_BODY)
                .expect(1)
                .create_async()
                .await;

            let result = adapter_at(&server.url())
                .generate_stream(build_request())
                .await;
            assert!(
                matches!(&result, Err(LlmError::UsageLimitExceeded { .. })),
                "expected UsageLimitExceeded, got {:?}",
                result.as_ref().err()
            );
            mock.assert_async().await;
        }

        #[test]
        fn map_error_distinguishes_the_spend_cap_from_ordinary_429_bodies() {
            let adapter = adapter_at("http://127.0.0.1:1");
            for body in [
                RATE_LIMIT_BODY,
                r#"{"error":{"details":{"error_code":"something_else"}}}"#,
                r#"{"error":{"message":"mentions enforced_spend_limit_reached in prose only"}}"#,
                r#"{"error":"enforced_spend_limit_reached"}"#,
                "not json",
                "",
            ] {
                assert!(
                    matches!(
                        adapter.map_error(429, body),
                        LlmError::RateLimitExceeded { .. }
                    ),
                    "body {body:?}"
                );
            }
            // The same body on another status is not a rate-limit error either way.
            assert!(!matches!(
                adapter.map_error(500, SPEND_CAP_BODY),
                LlmError::UsageLimitExceeded { .. }
            ));
        }
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

        let config =
            AnthropicConfig::new("sk-ant-test123".to_string(), "claude-opus-4-8".to_string())
                .with_base_url(server.url());
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

    // ── Phase 50 (PARITY-02, ADR-0061, D-01): the per-model parameter policy ──

    mod parameter_policy {
        use super::*;
        use crate::registry::UNPROFILED_MODEL_METADATA_KEY;
        use crate::sampling::{PARAMS_OMITTED_METADATA_KEY, RESPONSE_FORMAT_ADJUSTED_METADATA_KEY};
        use mockito::Server;
        use paladin_core::platform::container::prompt::{
            PromptItem, PromptParameters, PromptType, UserPrompt,
        };
        use paladin_ports::output::llm_port::ResponseFormat;
        use serde_json::{Value, json};
        use std::sync::Mutex as StdMutex;

        /// A model id that is in no seeded table, so these tests exercise the adapter-wide
        /// fallback profile (`claude-opus-4-8` was seeded by Phase 50.1, SURFACE-06; its own
        /// seeded behaviour is tested separately below).
        const MODEL: &str = "claude-unseeded-test-model";

        fn adapter_at(base_url: &str) -> AnthropicAdapter {
            AnthropicAdapter::new(
                AnthropicConfig::new("sk-ant-test123".to_string(), MODEL.to_string())
                    .with_base_url(base_url),
            )
            .expect("test config must build a valid adapter")
        }

        fn request_with(parameters: PromptParameters) -> LlmRequest {
            let mut request = LlmRequest::new(
                MODEL,
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

        /// Runs `generate()` against a mock `/messages` endpoint, returning the outgoing JSON
        /// body and the response metadata.
        async fn generate_capture(
            request: LlmRequest,
        ) -> (Value, std::collections::HashMap<String, String>) {
            let mut server = Server::new_async().await;
            let captured: Arc<StdMutex<Option<String>>> = Arc::new(StdMutex::new(None));
            let slot = Arc::clone(&captured);
            server
                .mock("POST", "/messages")
                .with_status(200)
                .with_body_from_request(move |req| {
                    *slot.lock().unwrap() =
                        Some(req.utf8_lossy_body().unwrap_or_default().into_owned());
                    TEXT_ONLY_OPUS_4_8_JSON.as_bytes().to_vec()
                })
                .create_async()
                .await;

            let response = adapter_at(&server.url())
                .generate(request)
                .await
                .expect("the mock answers a well-formed message");
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
            let captured: Arc<StdMutex<Option<String>>> = Arc::new(StdMutex::new(None));
            let slot = Arc::clone(&captured);
            server
                .mock("POST", "/messages")
                .with_status(200)
                .with_header("content-type", "text/event-stream")
                .with_body_from_request(move |req| {
                    *slot.lock().unwrap() =
                        Some(req.utf8_lossy_body().unwrap_or_default().into_owned());
                    b"event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n".to_vec()
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

        #[test]
        fn anthropic_model_profile_states_temperature_and_top_p_not_sent() {
            let profile = test_adapter()
                .model_profile("any-model")
                .expect("anthropic profiles every model");
            assert_eq!(profile.temperature, ParamSupport::NotSent);
            assert_eq!(profile.top_p, ParamSupport::NotSent);
            assert_eq!(profile.frequency_penalty, ParamSupport::NotSent);
            assert_eq!(profile.presence_penalty, ParamSupport::NotSent);
            assert_eq!(profile.stop_sequences, StopSupport::Supported { max: None });
            // The fallback window follows the declared capabilities, which describe the default
            // model's seeded window (plan 50-06).
            assert_eq!(profile.context_window, Some(1_000_000));
            assert!(!profile.json_object && !profile.json_schema);
        }

        #[tokio::test]
        async fn anthropic_temperature_is_omitted_and_recorded() {
            let mut parameters = unset();
            parameters.temperature = Some(0.7);
            parameters.top_p = Some(0.9);
            let (body, metadata) = generate_capture(request_with(parameters)).await;

            assert!(
                body.get("temperature").is_none(),
                "the Messages request carries no temperature: {body}"
            );
            assert!(body.get("top_p").is_none(), "nor top_p: {body}");
            assert_eq!(
                metadata
                    .get(PARAMS_OMITTED_METADATA_KEY)
                    .map(String::as_str),
                Some("temperature,top_p"),
                "the omission that used to be silent is now recorded"
            );
        }

        #[tokio::test]
        async fn anthropic_unset_values_are_neither_sent_nor_recorded() {
            let (body, metadata) = generate_capture(request_with(unset())).await;
            let obj = body.as_object().expect("body must be a JSON object");

            for key in ["temperature", "top_p", "stop_sequences"] {
                assert!(!obj.contains_key(key), "{key} was never set: {obj:?}");
            }
            assert_eq!(obj["max_tokens"], 4096, "the configured max_tokens applies");
            assert!(!metadata.contains_key(PARAMS_OMITTED_METADATA_KEY));
        }

        #[tokio::test]
        async fn anthropic_stop_sequences_keep_declared_order() {
            let mut parameters = unset();
            // A duplicate, deliberately: order and duplicates are the caller's.
            parameters.stop_sequences = Some(vec![
                "END".to_string(),
                "STOP".to_string(),
                "END".to_string(),
            ]);
            let (body, metadata) = generate_capture(request_with(parameters)).await;

            assert_eq!(body["stop_sequences"], json!(["END", "STOP", "END"]));
            assert!(!metadata.contains_key(PARAMS_OMITTED_METADATA_KEY));
        }

        #[tokio::test]
        async fn anthropic_stream_follows_the_profile() {
            let mut parameters = unset();
            parameters.temperature = Some(0.7);
            parameters.stop_sequences = Some(vec!["END".to_string(), "STOP".to_string()]);
            let body = stream_capture(request_with(parameters)).await;

            assert!(
                body.get("temperature").is_none(),
                "the streaming body omits the temperature too: {body}"
            );
            assert_eq!(body["stop_sequences"], json!(["END", "STOP"]));
            assert_eq!(body["stream"], true);
        }

        #[tokio::test]
        async fn anthropic_response_format_is_recorded_as_omitted() {
            let request = request_with(unset()).with_response_format(ResponseFormat::JsonObject);
            let (body, metadata) = generate_capture(request).await;

            assert!(
                body.get("response_format").is_none(),
                "Anthropic has no native JSON mode: {body}"
            );
            assert_eq!(
                metadata
                    .get(RESPONSE_FORMAT_ADJUSTED_METADATA_KEY)
                    .map(String::as_str),
                Some("omitted"),
                "the never-sent format is now recorded, not silently ignored"
            );

            let (_, metadata) = generate_capture(request_with(unset())).await;
            assert!(
                !metadata.contains_key(RESPONSE_FORMAT_ADJUSTED_METADATA_KEY),
                "no format requested, nothing adjusted"
            );
        }

        /// Phase 50, owner reading R-1 (option-b): a streamed call stamps the visibility keys
        /// the buffered path would, on the terminal chunk (`message_stop`) only.
        #[tokio::test]
        async fn anthropic_stream_terminal_chunk_carries_metadata() {
            let mut server = Server::new_async().await;
            server
                .mock("POST", "/messages")
                .with_status(200)
                .with_header("content-type", "text/event-stream")
                .with_body(
                    "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hi\"}}\n\n\
                     data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null},\"usage\":{\"output_tokens\":2}}\n\n\
                     data: {\"type\":\"message_stop\"}\n\n",
                )
                .create_async()
                .await;

            let mut parameters = unset();
            // The current Anthropic models are `NotSent` for temperature, so a bare request that
            // carries one has it omitted and recorded.
            parameters.temperature = Some(0.5);
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
                Some(MODEL)
            );
            for content in &chunks[..chunks.len() - 1] {
                assert!(content.metadata.is_empty(), "content chunk: {content:?}");
            }
        }

        #[tokio::test]
        async fn anthropic_unprofiled_model_is_stamped() {
            let (_, metadata) = generate_capture(request_with(unset())).await;
            assert_eq!(
                metadata
                    .get(UNPROFILED_MODEL_METADATA_KEY)
                    .map(String::as_str),
                Some(MODEL),
                "no seeded entry exists for this id, so it runs on the adapter-wide fallback"
            );
        }

        /// SURFACE-06 (D-14): a call to `claude-opus-4-8` is on a seeded profile, so the
        /// response carries no `paladin.unprofiled_model` stamp (the converse of
        /// `anthropic_unprofiled_model_is_stamped`).
        #[tokio::test]
        async fn anthropic_opus_4_8_is_not_stamped_unprofiled() {
            let mut request = request_with(unset());
            request.model = "claude-opus-4-8".to_string();
            let (_, metadata) = generate_capture(request).await;
            assert!(
                !metadata.contains_key(UNPROFILED_MODEL_METADATA_KEY),
                "claude-opus-4-8 is seeded, so no unprofiled stamp: {metadata:?}"
            );
        }

        #[test]
        fn anthropic_max_tokens_above_a_profiled_maximum_is_refused_by_name() {
            use crate::registry::{ModelOverride, ModelRegistry};
            use std::collections::BTreeMap;

            let mut adapter = test_adapter();
            let over = ModelOverride {
                max_output_tokens: Some(1000),
                ..ModelOverride::default()
            };
            adapter.model_registry =
                ModelRegistry::new(ANTHROPIC_PROVIDER, &[], fallback_profile(None))
                    .with_overrides(BTreeMap::from([(MODEL.to_string(), over)]))
                    .expect("a plain max_output_tokens override is valid");

            // No max_tokens on the request: the configured 4096 is what would be sent.
            match adapter.build_request(&request_with(unset())) {
                Err(LlmError::InvalidPrompt(message)) => {
                    assert!(message.contains("anthropic"), "{message}");
                    assert!(message.contains(MODEL), "{message}");
                    assert!(message.contains("max_tokens"), "{message}");
                }
                other => panic!("expected InvalidPrompt, got {other:?}"),
            }
        }

        #[test]
        fn anthropic_system_role_text_prompt_still_becomes_the_system_field() {
            use paladin_core::platform::container::prompt::{PromptRole, TextPrompt};

            let mut request = request_with(unset());
            request.prompt = PromptItem::new(PromptType::Text(TextPrompt {
                content: "Be terse.".to_string(),
                role: PromptRole::System,
            }))
            .expect("a text prompt must build");
            let (body, _) = test_adapter()
                .build_request(&request)
                .expect("a system-role text prompt builds");
            let body = serde_json::to_value(body).unwrap();

            assert_eq!(body["system"], "Be terse.");
            assert_eq!(body["messages"], json!([]));
        }
    }

    // ── Plan 31-04 (D-14/D-15/D-19): event-accumulation streaming usage ───

    mod streaming_usage_wiring {
        use super::*;
        use futures::StreamExt;
        use mockito::Server;
        use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};

        fn adapter_at(base_url: &str) -> AnthropicAdapter {
            let config =
                AnthropicConfig::new("sk-ant-test123".to_string(), "claude-sonnet-5".to_string())
                    .with_base_url(base_url);
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

        /// Predates Anthropic's place in the shared suite (plan 50-15 added the fixture, whose
        /// `streaming_usage_equals_non_streaming_usage` case now asserts the same three
        /// properties). Kept as the pin on the cached-prompt Sonnet fixture: exactly one chunk
        /// carries a finish reason, that SAME chunk is the only one carrying usage, and its usage
        /// equals the non-streaming path's `LlmResponse.usage` field-for-field.
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

    // ── Shared conformance suite (PARITY-07, plan 50-15) ──
    //
    // Anthropic joins the suite every other adapter already answers. Its Messages API framing
    // (`content[]` blocks; usage split across `message_start` and `message_delta` and no `[DONE]`
    // sentinel) is carried by `Wire::Anthropic`, whose default bodies this fixture inherits for
    // every case except the three below. Nested in its own module so every generated test's full
    // path contains "conformance" (`cargo test --lib conformance` selects it).
    /// Plan 50-23 (G-50-5a): Anthropic reports no vendor total, so both paths run the
    /// containment checks only -- a thinking count that escapes `output_tokens` is recorded.
    mod usage_reconciliation {
        use super::*;
        use crate::usage_reconcile::USAGE_UNRECONCILED_METADATA_KEY;
        use futures::StreamExt;
        use mockito::Server;
        use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};

        fn adapter_at(base_url: &str) -> AnthropicAdapter {
            let config =
                AnthropicConfig::new("sk-ant-test123".to_string(), "claude-sonnet-5".to_string())
                    .with_base_url(base_url);
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

        /// `thinking_tokens` 25 against `output_tokens` 10: a reading the inclusive contract
        /// cannot hold.
        const ESCAPING_THINKING_JSON: &str = r#"{"model":"claude-sonnet-5","id":"msg_1","type":"message","role":"assistant","content":[{"type":"text","text":"OK"}],"stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":16,"output_tokens":10,"output_tokens_details":{"thinking_tokens":25}}}"#;

        async fn generate_from(body: &'static str) -> LlmResponse {
            let mut server = Server::new_async().await;
            server
                .mock("POST", "/messages")
                .with_status(200)
                .with_body(body)
                .create_async()
                .await;
            adapter_at(&server.url())
                .generate(build_request())
                .await
                .expect("a well-formed body must parse")
        }

        #[tokio::test]
        async fn anthropic_thinking_beyond_output_is_a_reasoning_finding() {
            let response = generate_from(ESCAPING_THINKING_JSON).await;
            assert_eq!(response.usage.completion_tokens, 10, "the mapping is kept");
            assert_eq!(
                response
                    .metadata
                    .get(USAGE_UNRECONCILED_METADATA_KEY)
                    .map(String::as_str),
                Some("reasoning:reasoning=25,completion=10")
            );
        }

        #[tokio::test]
        async fn anthropic_normal_usage_records_no_finding() {
            let response = generate_from(CACHED_PROMPT_SONNET_5_JSON).await;
            assert!(
                !response
                    .metadata
                    .contains_key(USAGE_UNRECONCILED_METADATA_KEY),
                "{:?}",
                response.metadata
            );
        }

        #[tokio::test]
        async fn anthropic_stream_terminal_records_a_reasoning_finding() {
            let sse = "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_1\",\"type\":\"message\",\
                 \"role\":\"assistant\",\"model\":\"claude-sonnet-5\",\"content\":[],\"stop_reason\":null,\
                 \"stop_sequence\":null,\"usage\":{\"input_tokens\":16,\"output_tokens\":1}}}\n\n\
                 data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"OK\"}}\n\n\
                 data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null},\
                 \"usage\":{\"output_tokens\":10,\"output_tokens_details\":{\"thinking_tokens\":25}}}\n\n\
                 data: {\"type\":\"message_stop\"}\n\n";
            let mut server = Server::new_async().await;
            server
                .mock("POST", "/messages")
                .with_status(200)
                .with_header("content-type", "text/event-stream")
                .with_body(sse)
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

            let (terminal, earlier) = chunks.split_last().expect("a terminal chunk");
            assert!(terminal.finish_reason.is_some());
            assert_eq!(
                terminal
                    .metadata
                    .get(USAGE_UNRECONCILED_METADATA_KEY)
                    .map(String::as_str),
                Some("reasoning:reasoning=25,completion=10")
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

        struct AnthropicFixture;

        impl crate::conformance::ConformanceFixture for AnthropicFixture {
            const WIRE: crate::conformance::Wire = crate::conformance::Wire::Anthropic;

            fn default_model() -> &'static str {
                ANTHROPIC_DEFAULT_MODEL
            }

            fn provider_name() -> &'static str {
                "anthropic"
            }

            fn adapter(base_url: &str) -> Arc<dyn LlmPort> {
                let mut config = AnthropicConfig::new(
                    "test-key".to_string(),
                    ANTHROPIC_DEFAULT_MODEL.to_string(),
                )
                .with_base_url(base_url);
                // The suite measures one attempt per call (the status cases would otherwise sleep
                // through this adapter's retry back-off), like the OpenAI fixture's `max_retries: 0`.
                config.max_retries = 0;
                config.timeout_seconds = 5;
                Arc::new(AnthropicAdapter::new(config).expect("test config must build"))
            }

            fn success_body() -> String {
                // The usage object is the full real shape (cache read, cache write and thinking
                // counts) so the D-19 stream-versus-buffered case compares every sub-count.
                // `input_tokens` 85 + cache read 512 + cache write 128 = prompt 725.
                json!({
                    "id": "msg_1",
                    "type": "message",
                    "role": "assistant",
                    "model": ANTHROPIC_DEFAULT_MODEL,
                    "content": [{"type": "text", "text": "Hi there"}],
                    "stop_reason": "end_turn",
                    "stop_sequence": null,
                    "usage": {
                        "input_tokens": 85,
                        "cache_creation_input_tokens": 128,
                        "cache_read_input_tokens": 512,
                        "output_tokens": 40,
                        "output_tokens_details": {"thinking_tokens": 0}
                    }
                })
                .to_string()
            }

            fn stream_body() -> String {
                // D-19: `message_start` carries the input and cache figures and `message_delta`
                // the output and thinking figures; merged at `message_stop` they equal
                // `success_body()`'s usage field-for-field.
                concat!(
                    "event: message_start\n",
                    "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_1\",\"type\":\"message\",",
                    "\"role\":\"assistant\",\"model\":\"claude-haiku-5-5\",\"content\":[],\"stop_reason\":null,",
                    "\"stop_sequence\":null,\"usage\":{\"input_tokens\":85,\"cache_creation_input_tokens\":128,",
                    "\"cache_read_input_tokens\":512,\"output_tokens\":1}}}\n\n",
                    "event: content_block_delta\n",
                    "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hel\"}}\n\n",
                    "event: content_block_delta\n",
                    "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"lo \"}}\n\n",
                    "event: content_block_delta\n",
                    "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"world\"}}\n\n",
                    "event: message_delta\n",
                    "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null},",
                    "\"usage\":{\"output_tokens\":40,\"output_tokens_details\":{\"thinking_tokens\":0}}}\n\n",
                    "event: message_stop\n",
                    "data: {\"type\":\"message_stop\"}\n\n",
                )
                .to_string()
            }

            // Anthropic's stream parser skips an unparsable event with a `warn!` naming only its
            // bounded type (`anthropic_stream_skips_a_malformed_event_and_keeps_going`), so the
            // after-the-first-chunk case asserts that documented alternative.
            fn malformed_frame() -> crate::conformance::MalformedFrame {
                crate::conformance::MalformedFrame::SkippedWithWarning
            }

            fn error_body(status: u16) -> String {
                json!({
                    "type": "error",
                    "error": {"type": "api_error", "message": format!("mock error for status {status}")}
                })
                .to_string()
            }
        }

        crate::llm_conformance_suite!(AnthropicFixture);
    }

    /// Refusals, stop reasons, thinking text and the stream's real stop reason (plan 50-13,
    /// D-09/D-11/D-11a; RESEARCH Pitfalls 1 and 2).
    mod response_parity {
        use super::*;
        use futures::StreamExt;
        use mockito::Server;
        use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};

        fn parse(body: &str) -> Result<LlmResponse, LlmError> {
            let response: ClaudeResponse =
                serde_json::from_str(body).expect("test body must deserialize");
            test_adapter().parse_response(Uuid::new_v4(), response)
        }

        fn body(content: &str, stop_reason: &str, stop_details: &str) -> String {
            format!(
                r#"{{"id":"msg_1","model":"claude-opus-5","content":{content},"stop_reason":{stop_reason},"stop_details":{stop_details},"usage":{{"input_tokens":3,"output_tokens":4}}}}"#
            )
        }

        #[test]
        fn anthropic_refusal_is_typed_not_an_empty_completion() {
            let result = parse(&body(
                "[]",
                r#""refusal""#,
                r#"{"type":"refusal","category":"cyber","explanation":"This request seeks malware."}"#,
            ));
            match result {
                Err(LlmError::Refusal {
                    provider,
                    model,
                    detail,
                    ..
                }) => {
                    assert_eq!(provider, "anthropic");
                    assert_eq!(model, "claude-opus-5");
                    assert!(detail.contains("cyber"), "{detail}");
                    assert!(detail.contains("seeks malware"), "{detail}");
                }
                other => panic!("expected Err(Refusal), got {other:?}"),
            }
        }

        #[test]
        fn anthropic_refusal_with_null_details_names_the_stop_reason() {
            for details in [
                "null",
                r#"{"type":"refusal","category":null,"explanation":null}"#,
            ] {
                match parse(&body("[]", r#""refusal""#, details)) {
                    Err(LlmError::Refusal { detail, .. }) => {
                        assert!(detail.contains("stop_reason=refusal"), "{detail}");
                    }
                    other => panic!("expected Err(Refusal), got {other:?}"),
                }
            }
        }

        #[test]
        fn anthropic_refusal_wins_over_partial_text() {
            let result = parse(&body(
                r#"[{"type":"text","text":"Sure, here is"}]"#,
                r#""refusal""#,
                r#"{"type":"refusal","category":"bio","explanation":null}"#,
            ));
            assert!(
                matches!(result, Err(LlmError::Refusal { .. })),
                "{result:?}"
            );
        }

        #[test]
        fn anthropic_refusal_detail_redacts_the_configured_key_before_bounding() {
            // The key sits just before the character budget, so bounding first would keep a
            // prefix of it.
            let key = "sk-ant-test123";
            let mut explanation = "a".repeat(RESPONSE_EXCERPT_CHAR_BUDGET - 6);
            explanation.push_str(key);
            explanation.push_str(&"b".repeat(100));
            let details =
                format!(r#"{{"type":"refusal","category":null,"explanation":"{explanation}"}}"#);
            match parse(&body("[]", r#""refusal""#, &details)) {
                Err(LlmError::Refusal { detail, .. }) => {
                    assert!(!detail.contains(key), "{detail}");
                    assert!(!detail.contains("sk-ant"), "key prefix leaked: {detail}");
                    assert!(!detail.contains("test123"), "key tail leaked: {detail}");
                }
                other => panic!("expected Err(Refusal), got {other:?}"),
            }
        }

        #[test]
        fn anthropic_stop_reasons_map_by_name() {
            let text = r#"[{"type":"text","text":"ok"}]"#;
            for (wire, expected) in [
                ("end_turn", FinishReason::Stop),
                ("stop_sequence", FinishReason::Stop),
                ("max_tokens", FinishReason::Length),
                ("model_context_window_exceeded", FinishReason::Length),
                ("tool_use", FinishReason::FunctionCall),
                ("pause_turn", FinishReason::Error("pause_turn".to_string())),
                (
                    "something_new",
                    FinishReason::Error("unknown: something_new".to_string()),
                ),
            ] {
                let response = parse(&body(text, &format!("\"{wire}\""), "null"))
                    .unwrap_or_else(|e| panic!("{wire} must parse: {e:?}"));
                assert_eq!(
                    std::mem::discriminant(&response.finish_reason),
                    std::mem::discriminant(&expected),
                    "{wire}"
                );
                if let FinishReason::Error(label) = &expected {
                    assert!(
                        matches!(&response.finish_reason, FinishReason::Error(got) if got == label),
                        "{wire}: {:?}",
                        response.finish_reason
                    );
                }
            }
        }

        #[test]
        fn anthropic_unknown_stop_reason_is_bounded_to_identifier_characters() {
            let finish = map_stop_reason(Some("odd value\nwith sk-ant-secret"));
            assert!(
                matches!(&finish, FinishReason::Error(label) if label == "unknown: odd"),
                "{finish:?}"
            );
        }

        #[test]
        fn anthropic_tool_use_without_text_stays_ok() {
            let response = parse(&body(
                r#"[{"type":"tool_use","id":"toolu_1","name":"lookup","input":{}}]"#,
                r#""tool_use""#,
                "null",
            ))
            .expect("a tool_use stop with no text is a valid response");
            assert!(matches!(response.finish_reason, FinishReason::FunctionCall));
        }

        #[test]
        fn anthropic_empty_content_on_end_turn_is_an_empty_completion() {
            for content in ["[]", r#"[{"type":"text","text":"  "}]"#] {
                match parse(&body(content, r#""end_turn""#, "null")) {
                    Err(LlmError::EmptyCompletion { detail: msg, .. }) => {
                        assert!(msg.contains("finish_reason=stop"), "{msg}");
                        assert!(msg.contains("content block(s)"), "{msg}");
                    }
                    other => panic!("expected Err(EmptyCompletion), got {other:?}"),
                }
            }
        }

        #[test]
        fn anthropic_context_window_stop_with_no_text_names_the_cause() {
            match parse(&body("[]", r#""model_context_window_exceeded""#, "null")) {
                Err(LlmError::EmptyCompletion { detail: msg, .. }) => {
                    assert!(msg.contains("finish_reason=length"), "{msg}");
                }
                other => panic!("expected Err(EmptyCompletion), got {other:?}"),
            }
        }

        #[test]
        fn anthropic_thinking_text_reaches_reasoning_in_block_order() {
            let response = parse(&body(
                r#"[{"type":"thinking","thinking":"First, ","signature":"s1"},{"type":"redacted_thinking","data":"opaque"},{"type":"thinking","thinking":"then second.","signature":"s2"},{"type":"text","text":"The answer."}]"#,
                r#""end_turn""#,
                "null",
            ))
            .expect("thinking plus text must parse");
            assert_eq!(response.reasoning.as_deref(), Some("First, then second."));
            assert_eq!(response.content, "The answer.");
        }

        #[test]
        fn anthropic_blank_thinking_is_no_reasoning() {
            let response = parse(&body(
                r#"[{"type":"thinking","thinking":"","signature":"s"},{"type":"text","text":"ok"}]"#,
                r#""end_turn""#,
                "null",
            ))
            .expect("must parse");
            assert!(response.reasoning.is_none());
        }

        #[test]
        fn unparsable_event_type_names_only_a_bounded_type() {
            assert_eq!(
                unparsable_event_type(r#"{"type":"content_block_delta","index":"oops"}"#),
                "content_block_delta"
            );
            assert_eq!(unparsable_event_type("not json sk-ant-secret"), "unknown");
            assert_eq!(unparsable_event_type(r#"{"no_type":1}"#), "unknown");
            assert_eq!(unparsable_event_type(r#"{"type":7}"#), "unknown");
            let hostile = format!(r#"{{"type":"{}"}}"#, "x".repeat(500));
            assert_eq!(unparsable_event_type(&hostile).len(), 64);
        }

        // ── stream ────────────────────────────────────────────────────────

        fn adapter_at(base_url: &str) -> AnthropicAdapter {
            let config =
                AnthropicConfig::new("sk-ant-test123".to_string(), "claude-sonnet-5".to_string())
                    .with_base_url(base_url);
            AnthropicAdapter::new(config).expect("test config must build a valid adapter")
        }

        fn stream_request() -> LlmRequest {
            LlmRequest::new(
                "claude-sonnet-5",
                PromptItem::new(PromptType::User(UserPrompt {
                    query: "Hello".to_string(),
                    context: None,
                }))
                .expect("a user prompt must build"),
            )
        }

        const START: &str = "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_1\",\"model\":\"claude-sonnet-5\",\"usage\":{\"input_tokens\":5,\"output_tokens\":1}}}\n\n";
        const STOP: &str = "data: {\"type\":\"message_stop\"}\n\n";

        fn text_delta(text: &str) -> String {
            format!(
                "data: {{\"type\":\"content_block_delta\",\"index\":0,\"delta\":{{\"type\":\"text_delta\",\"text\":\"{text}\"}}}}\n\n"
            )
        }

        fn thinking_delta(text: &str) -> String {
            format!(
                "data: {{\"type\":\"content_block_delta\",\"index\":0,\"delta\":{{\"type\":\"thinking_delta\",\"thinking\":\"{text}\"}}}}\n\n"
            )
        }

        fn message_delta(stop_reason: &str, stop_details: &str) -> String {
            format!(
                "data: {{\"type\":\"message_delta\",\"delta\":{{\"stop_reason\":\"{stop_reason}\",\"stop_sequence\":null,\"stop_details\":{stop_details}}},\"usage\":{{\"output_tokens\":7}}}}\n\n"
            )
        }

        async fn run_stream(sse: String) -> Vec<Result<StreamingResponse, LlmError>> {
            let mut server = Server::new_async().await;
            server
                .mock("POST", "/messages")
                .with_status(200)
                .with_header("content-type", "text/event-stream")
                .with_body(sse)
                .create_async()
                .await;
            let stream = adapter_at(&server.url())
                .generate_stream(stream_request())
                .await
                .expect("streaming call must start");
            let mut stream = Box::into_pin(stream);
            let mut items = Vec::new();
            while let Some(item) = stream.next().await {
                items.push(item);
            }
            items
        }

        #[tokio::test]
        async fn anthropic_stream_maps_the_real_stop_reason_on_the_terminal_chunk() {
            for (wire, expect_length) in [("max_tokens", true), ("end_turn", false)] {
                let items = run_stream(format!(
                    "{START}{}{}{STOP}",
                    text_delta("Hi"),
                    message_delta(wire, "null")
                ))
                .await;
                let terminal = items
                    .last()
                    .expect("stream must end")
                    .as_ref()
                    .expect("terminal chunk must be Ok");
                assert_eq!(
                    matches!(terminal.finish_reason, Some(FinishReason::Length)),
                    expect_length,
                    "{wire}: {:?}",
                    terminal.finish_reason
                );
                assert!(terminal.usage.is_some(), "usage rides the terminal chunk");
            }
        }

        #[tokio::test]
        async fn anthropic_stream_refusal_after_partial_text_ends_in_refusal() {
            let items = run_stream(format!(
                "{START}{}{}{}{STOP}",
                text_delta("Sure, "),
                text_delta("here "),
                message_delta(
                    "refusal",
                    r#"{"type":"refusal","category":"cyber","explanation":"No."}"#
                )
            ))
            .await;
            assert_eq!(items.len(), 3, "two text items then the refusal: {items:?}");
            assert_eq!(items[0].as_ref().expect("text").delta, "Sure, ");
            assert_eq!(items[1].as_ref().expect("text").delta, "here ");
            match &items[2] {
                Err(LlmError::Refusal {
                    provider,
                    model,
                    detail,
                    ..
                }) => {
                    assert_eq!(provider, "anthropic");
                    assert_eq!(model, "claude-sonnet-5");
                    assert!(detail.contains("cyber"), "{detail}");
                }
                other => panic!("expected Err(Refusal) last, got {other:?}"),
            }
        }

        #[tokio::test]
        async fn anthropic_stream_thinking_deltas_reach_reasoning_in_wire_order() {
            let items = run_stream(format!(
                "{START}{}{}{}{}{STOP}",
                thinking_delta("one "),
                thinking_delta("two"),
                text_delta("Answer"),
                message_delta("end_turn", "null")
            ))
            .await;
            let reasoning: Vec<&str> = items
                .iter()
                .filter_map(|i| i.as_ref().ok()?.reasoning.as_deref())
                .collect();
            assert_eq!(reasoning, vec!["one ", "two"]);
            let text: String = items
                .iter()
                .filter_map(|i| i.as_ref().ok())
                .map(|c| c.delta.as_str())
                .collect();
            assert_eq!(text, "Answer");
            assert!(items.iter().all(Result::is_ok));
        }

        #[tokio::test]
        async fn anthropic_empty_stream_is_empty_completion() {
            let items = run_stream(format!(
                "{START}{}{}{}{STOP}",
                thinking_delta("pondering "),
                thinking_delta("more"),
                message_delta("end_turn", "null")
            ))
            .await;
            assert_eq!(items.len(), 3, "{items:?}");
            // The reasoning already yielded stays yielded.
            assert_eq!(
                items[0].as_ref().expect("reasoning").reasoning.as_deref(),
                Some("pondering ")
            );
            assert_eq!(
                items[1].as_ref().expect("reasoning").reasoning.as_deref(),
                Some("more")
            );
            match &items[2] {
                Err(LlmError::EmptyCompletion { detail: msg, .. }) => {
                    assert!(msg.contains("finish_reason=stop"), "{msg}");
                }
                other => panic!("expected Err(EmptyCompletion) last, got {other:?}"),
            }
        }

        #[tokio::test]
        async fn anthropic_stream_skips_a_malformed_event_and_keeps_going() {
            let items = run_stream(format!(
                "{START}data: {{\"type\":\"content_block_delta\",\"index\":\"bad\"\n\n{}{}{STOP}",
                text_delta("ok"),
                message_delta("end_turn", "null")
            ))
            .await;
            assert!(items.iter().all(Result::is_ok), "{items:?}");
            assert_eq!(items[0].as_ref().expect("text").delta, "ok");
        }
    }
}

/// The PARITY-03 knobs on `AnthropicConfig` (plan 50-08, D-12): each one is read by the adapter.
#[cfg(test)]
mod knob_tests {
    use super::*;
    use crate::test_support::{
        attempts_against_a_failing_endpoint_at, mock_capture, send_with_max_tokens, server,
    };
    use crate::window::effective_capabilities;
    use serde_json::json;

    const UNSEEDED: &str = "claude-unseeded-test-model";
    const PATH: &str = "/messages";

    fn config(base_url: &str, model: &str) -> AnthropicConfig {
        AnthropicConfig::new("test-key".to_string(), model.to_string()).with_base_url(base_url)
    }

    fn window_override(window: u32) -> ModelOverride {
        ModelOverride {
            context_window: Some(window),
            ..ModelOverride::default()
        }
    }

    fn response() -> serde_json::Value {
        json!({
            "id": "msg_1",
            "type": "message",
            "role": "assistant",
            "model": "test-model",
            "content": [{"type": "text", "text": "ok"}],
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {"input_tokens": 1, "output_tokens": 1}
        })
    }

    fn error_of(config: AnthropicConfig) -> String {
        match AnthropicAdapter::new(config) {
            Err(error) => error.to_string(),
            Ok(_) => panic!("construction must fail"),
        }
    }

    #[test]
    fn anthropic_new_takes_key_and_model_and_defaults_the_rest() {
        // The consumer muster pins this signature; the knobs arrive through builders.
        let config = AnthropicConfig::new("test-key".to_string(), "claude-haiku-5-5".to_string());
        assert_eq!(config.base_url, ANTHROPIC_DEFAULT_BASE_URL);
        assert_eq!(config.model, "claude-haiku-5-5");
        assert_eq!(config.max_tokens, 4096);
        assert_eq!(config.timeout_seconds, 60);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.context_window, None);
        assert!(config.model_overrides.is_empty());
    }

    /// D-07, `compiled-default` (owner decision, 2026-10-20): `new` sets the one compiled
    /// output budget for every model, seeded or not, so no default moves when a model is seeded
    /// and `new` agrees with `from_env` and the layered path. The seeded maximum output (128,000
    /// for the default model) is reachable only through `with_max_tokens`.
    #[test]
    fn anthropic_new_max_tokens_is_the_compiled_default_for_every_model() {
        assert_eq!(ANTHROPIC_DEFAULT_MAX_TOKENS, 4096);
        for model in ["claude-haiku-5-5", UNSEEDED, "claude-opus-4-8"] {
            let config = AnthropicConfig::new("test-key".to_string(), model.to_string());
            assert_eq!(config.max_tokens, ANTHROPIC_DEFAULT_MAX_TOKENS, "{model}");
        }
        // `claude-opus-4-8` is seeded (Phase 50.1) with a 128,000 maximum output; `new` still
        // sends 4,096 for it, and the seeded maximum is reachable only through `with_max_tokens`.
        let opus_maximum = seeded::ANTHROPIC
            .iter()
            .find(|entry| entry.model == "claude-opus-4-8")
            .and_then(|entry| (entry.build)().max_output_tokens);
        assert_eq!(opus_maximum, Some(128_000));
        let config = AnthropicConfig::new("k".to_string(), "claude-haiku-5-5".to_string())
            .with_max_tokens(16);
        assert_eq!(config.max_tokens, 16);
        let seeded_maximum = seeded::ANTHROPIC
            .iter()
            .find(|entry| entry.model == "claude-haiku-5-5")
            .and_then(|entry| (entry.build)().max_output_tokens);
        assert_eq!(seeded_maximum, Some(128_000));
        let config = AnthropicConfig::new("k".to_string(), "claude-haiku-5-5".to_string())
            .with_max_tokens(seeded_maximum.unwrap());
        assert_eq!(config.max_tokens, 128_000);
    }

    /// A seeded model whose row says `reasoning: false` keeps the provider's own 60 s default:
    /// only a seeded reasoning row moves `new` to 300 s (Phase 50 D-13). `claude-opus-4-8`'s
    /// default call carried no reasoning text (`50.1-SEED-PROBE.md`, probe 1), so it is 60 s.
    #[test]
    fn anthropic_new_timeout_for_seeded_opus_4_8_is_the_non_reasoning_default() {
        let config = AnthropicConfig::new("k".to_string(), "claude-opus-4-8".to_string());
        assert_eq!(config.timeout_seconds, 60);
    }

    /// The two Phase 50.1 Anthropic rows whose default call returned reasoning text
    /// (`claude-fable-5`, `claude-opus-5`; `50.1-SEED-PROBE.md`, probe 1) move `new` to the
    /// reasoning-model 300 s (D-13); every other seeded Anthropic row stays at 60 s.
    #[test]
    fn anthropic_new_timeout_is_300_only_for_the_seeded_reasoning_rows() {
        for entry in seeded::ANTHROPIC {
            let reasoning = (entry.build)().reasoning;
            let config = AnthropicConfig::new("k".to_string(), entry.model.to_string());
            let expected = if reasoning { 300 } else { 60 };
            assert_eq!(config.timeout_seconds, expected, "{}", entry.model);
        }
        for model in ["claude-fable-5", "claude-opus-5"] {
            let config = AnthropicConfig::new("k".to_string(), model.to_string());
            assert_eq!(config.timeout_seconds, 300, "{model}");
        }
    }

    #[test]
    fn anthropic_new_timeout_follows_the_model_and_builders_override_it() {
        // Unchanged D-13 default: 60 s for a non-reasoning model.
        let config = AnthropicConfig::new("k".to_string(), UNSEEDED.to_string());
        assert_eq!(config.timeout_seconds, 60);
        let config = config
            .with_timeout_seconds(7)
            .with_base_url("https://example.invalid/v1");
        assert_eq!(config.timeout_seconds, 7);
        assert_eq!(config.base_url, "https://example.invalid/v1");
    }

    #[tokio::test]
    async fn anthropic_max_tokens_is_the_default_output_budget_on_the_wire() {
        let mut server = server().await;
        let captured = mock_capture(&mut server, PATH, response()).await;
        let url = server.url();

        let adapter = AnthropicAdapter::new(config(&url, UNSEEDED).with_max_tokens(2048)).unwrap();
        let body = send_with_max_tokens(&adapter, &captured, UNSEEDED, None)
            .await
            .unwrap();
        assert_eq!(body["max_tokens"], 2048, "{body}");
        // A request's own value wins.
        let body = send_with_max_tokens(&adapter, &captured, UNSEEDED, Some(512))
            .await
            .unwrap();
        assert_eq!(body["max_tokens"], 512, "{body}");
    }

    #[tokio::test]
    async fn anthropic_max_tokens_default_is_checked_against_the_profile() {
        let mut server = server().await;
        let captured = mock_capture(&mut server, PATH, response()).await;
        let over = ModelOverride {
            max_output_tokens: Some(1000),
            ..ModelOverride::default()
        };
        let adapter = AnthropicAdapter::new(
            config(&server.url(), UNSEEDED)
                .with_max_tokens(2048)
                .with_model_overrides(BTreeMap::from([(UNSEEDED.to_string(), over)])),
        )
        .unwrap();
        let Err(LlmError::InvalidPrompt(message)) =
            send_with_max_tokens(&adapter, &captured, UNSEEDED, None).await
        else {
            panic!("a default above the model's maximum output must be InvalidPrompt");
        };
        for needle in ["max_tokens", "2048", "1000"] {
            assert!(
                message.contains(needle),
                "'{needle}' missing from: {message}"
            );
        }
        assert!(captured.lock().unwrap().is_none(), "nothing was sent");
    }

    #[tokio::test]
    async fn anthropic_max_retries_is_read_by_the_adapter() {
        // This adapter's loop counts total attempts (its long-standing convention): the budget
        // `n` sends up to `n` requests on a retryable 5xx.
        for (budget, attempts) in [(1_u32, 1_usize), (2, 2)] {
            let server = server().await;
            let adapter =
                AnthropicAdapter::new(config(&server.url(), UNSEEDED).with_max_retries(budget))
                    .unwrap();
            attempts_against_a_failing_endpoint_at(server, PATH, &adapter, UNSEEDED, attempts)
                .await;
        }
    }

    #[tokio::test]
    async fn anthropic_429_is_one_attempt_whatever_the_retry_budget() {
        let mut server = server().await;
        let mock = server
            .mock("POST", PATH)
            .with_status(429)
            .with_body(r#"{"type":"error","error":{"type":"rate_limit_error","message":"slow"}}"#)
            .expect(1)
            .create_async()
            .await;
        let adapter =
            AnthropicAdapter::new(config(&server.url(), UNSEEDED).with_max_retries(5)).unwrap();
        let captured = std::sync::Arc::new(std::sync::Mutex::new(None));
        let outcome = send_with_max_tokens(&adapter, &captured, UNSEEDED, None).await;
        assert!(matches!(outcome, Err(LlmError::RateLimitExceeded { .. })));
        mock.assert_async().await;
    }

    #[test]
    fn anthropic_context_window_override_is_declared() {
        let adapter = AnthropicAdapter::new(
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
    fn context_window_override_beats_a_seeded_window_on_anthropic() {
        let base = || config("https://example.invalid/v1", "claude-haiku-5-5");
        let seeded = AnthropicAdapter::new(base()).unwrap();
        assert_eq!(
            seeded
                .model_profile("claude-haiku-5-5")
                .unwrap()
                .context_window,
            Some(1_000_000)
        );

        let adapter = AnthropicAdapter::new(base().with_context_window(500_000)).unwrap();
        assert_eq!(
            adapter
                .model_profile("claude-haiku-5-5")
                .unwrap()
                .context_window,
            Some(500_000)
        );
        assert_eq!(
            effective_capabilities(&adapter, "claude-haiku-5-5").max_context_tokens,
            Some(500_000)
        );

        let adapter =
            AnthropicAdapter::new(base().with_context_window(500_000).with_model_overrides(
                BTreeMap::from([("claude-haiku-5-5".to_string(), window_override(900_000))]),
            ))
            .unwrap();
        assert_eq!(
            adapter
                .model_profile("claude-haiku-5-5")
                .unwrap()
                .context_window,
            Some(900_000)
        );

        // A window below the seeded maximum output is refused, never clamped.
        let message = error_of(base().with_context_window(100_000));
        for needle in ["anthropic", "claude-haiku-5-5", "100000", "128000"] {
            assert!(
                message.contains(needle),
                "'{needle}' missing from: {message}"
            );
        }
    }

    #[test]
    fn anthropic_invalid_override_fails_construction_naming_provider_and_model() {
        let with_override = |over: ModelOverride| {
            config("https://example.invalid/v1", UNSEEDED)
                .with_model_overrides(BTreeMap::from([(UNSEEDED.to_string(), over)]))
        };
        let adapter = AnthropicAdapter::new(with_override(window_override(900_000))).unwrap();
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
                message.contains("anthropic") && message.contains(UNSEEDED),
                "the error must name the provider and the model: {message}"
            );
        }
    }

    #[test]
    fn anthropic_config_defaults_a_reasoning_model_to_300s() {
        // No Anthropic entry is seeded as a reasoning model, so every model keeps today's 60 s
        // here; the 300 s path is exercised for the vendors that seed one (DeepSeek, Gemini,
        // Kimi, Qwen, Grok) and by `registry::knob_tests::is_reasoning_model_*`.
        for model in ["claude-haiku-5-5", UNSEEDED] {
            assert_eq!(
                config("https://example.invalid/v1", model).timeout_seconds,
                60,
                "new({model})"
            );
        }
        let mut explicit = config("https://example.invalid/v1", "claude-haiku-5-5");
        explicit.timeout_seconds = 45;
        assert_eq!(explicit.timeout_seconds, 45);
    }

    #[test]
    fn anthropic_config_debug_redacts_the_api_key() {
        let config = config("https://api.example.invalid/v1", "claude-haiku-5-5");
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
