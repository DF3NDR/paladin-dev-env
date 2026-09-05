//! Generic operator-configured OpenAI-compatible LLM Adapter (D-03).
//!
//! Unlike every other adapter in this crate, [`OpenAiCompatibleAdapter`] has
//! no vendor of its own: `base_url`, credential, model and every capability
//! flag come entirely from operator configuration, with pessimistic
//! defaults for anything left unset (D-04). It exists so a consumer can
//! point Paladin at any OpenAI-compatible endpoint — a self-hosted vLLM or
//! LiteLLM gateway, Groq, Together, Mistral, Fireworks, Bedrock's
//! OpenAI-compat mode, or any future one — without writing a new preset.
//! This is what lets D-03 dispose of the rest of the candidate field as
//! *already covered* rather than deferred.
//!
//! ## Naming — resolved at the Task 1 checkpoint
//!
//! This surface's exact operator-facing names (provider literal, env-var
//! prefix, capability-declaration mechanism) were confirmed by the human at
//! an interactive `AskUserQuestion` checkpoint the `/gsd-execute-phase 17`
//! orchestrator raised for this plan's Task 1 on 2026-08-17, selecting
//! **option-a**:
//!   - Provider literal: `"openai-compatible"` (fixed by D-09)
//!   - Type name: [`OpenAiCompatibleAdapter`] (fixed by D-09)
//!   - Env-var prefix: `OPENAI_COMPATIBLE_`
//!   - Capability declaration: BOTH individual environment variables (this
//!     module's `from_env()` path) AND a structured config-file block
//!     ([`OpenAiCompatibleCapabilitiesConfig`]'s `Deserialize` impl, for a
//!     future config-file loader) — the deciding rationale being that the
//!     provider must be fully usable from environment variables alone, with
//!     no config file present, matching every other adapter in this crate
//!     and PROJECT.md's env-var-only credential posture.
//!
//! This surface is public API operators write configuration against (D-03's
//! one-way reversibility rating) — once these names appear in a deployed
//! `config.yml` or exported shell environment, renaming any of them is a
//! second breaking change, not a revert.
//!
//! ## `OPENAI_COMPATIBLE_API_KEY` is not `OPENAI_API_KEY`
//!
//! The prefix was deliberately chosen to match the provider literal
//! (`openai-compatible`), which puts `OPENAI_COMPATIBLE_API_KEY` one word
//! away from [`crate::openai`]'s `OPENAI_API_KEY`. **These are two
//! different credentials for two different providers.** Setting the wrong
//! one sends a real OpenAI key to whatever `OPENAI_COMPATIBLE_BASE_URL`
//! names (or leaves the OpenAI preset unable to resolve because its own key
//! was never set). There is no cross-checking between the two variables —
//! read both names character-by-character before exporting either.
//!
//! ## Environment Variables
//!
//! | Variable | Required | Default |
//! |---|---|---|
//! | `OPENAI_COMPATIBLE_API_KEY` | yes | none — no defensible default |
//! | `OPENAI_COMPATIBLE_BASE_URL` | yes | none — no defensible default (D-03: no vendor to inherit an endpoint from) |
//! | `OPENAI_COMPATIBLE_MODEL` | yes | none — no defensible default |
//! | `OPENAI_COMPATIBLE_TIMEOUT_SECONDS` | no | `60` |
//! | `OPENAI_COMPATIBLE_SUPPORTS_STREAMING` | no | `true` (baseline compat-spec behaviour, not an add-on) |
//! | `OPENAI_COMPATIBLE_SUPPORTS_TOOL_CALLING` | no | `false` |
//! | `OPENAI_COMPATIBLE_SUPPORTS_FUNCTION_CALLING` | no | `false` |
//! | `OPENAI_COMPATIBLE_SUPPORTS_VISION` | no | `false` |
//! | `OPENAI_COMPATIBLE_SUPPORTS_EMBEDDINGS` | no | `false` |
//! | `OPENAI_COMPATIBLE_SUPPORTS_SYSTEM_MESSAGES` | no | `false` |
//! | `OPENAI_COMPATIBLE_MAX_CONTEXT_TOKENS` | no | `None` (unknown/unbounded) |
//! | `OPENAI_COMPATIBLE_TEMPERATURE_MIN` / `OPENAI_COMPATIBLE_TEMPERATURE_MAX` | no | `None` (both or neither; falls back to ADR-0004's global `[0.0, 1.0]`) |
//!
//! **Unset means the conservative answer for every capability field. This
//! adapter must never claim a capability nobody asserted** (D-04,
//! 17-RESEARCH.md Pitfall 5) — the next person adding a capability field
//! here must default it `false`/`None` unless it describes baseline
//! compat-spec behaviour the way streaming does.
//!
//! ## Provider identity (T-17-20, accepted debt)
//!
//! [`get_provider_name`](OpenAiCompatibleAdapter::get_provider_name) returns
//! the fixed literal `"openai-compatible"` regardless of the endpoint
//! configured. See that method's own doc comment for the accepted cost.
//!
//! ## Trust boundary: the operator-supplied `base_url` (T-17-18)
//!
//! `base_url` determines where every request — carrying the configured API
//! key — is sent. This adapter builds its engine with `reqwest`'s redirect
//! policy set to `none`, so a `3xx` response from the configured host can
//! never cause the `Authorization` header to be replayed to a different,
//! attacker-influenced host. The residual case is the operator's own trust
//! decision: nothing here stops an operator from deliberately pointing
//! `base_url` at an internal/metadata-service address. Paladin's deployment
//! model is single-tenant with operator-controlled configuration
//! (PROJECT.md); no allowlist is introduced.
//!
//! A plain-HTTP `base_url` is permitted — a local gateway is a legitimate
//! case — but transmits the configured API key in clear text. Construction
//! logs a `warn!` once when the scheme is `http` and the host is not
//! loopback (T-17-22).

use async_trait::async_trait;
use futures::Stream;
use serde::Deserialize;
use std::env;
use std::net::IpAddr;

use paladin_ports::output::llm_port::{
    LlmError, LlmPort, LlmRequest, LlmResponse, ProviderCapabilities, StreamingResponse,
};

use crate::compat::{
    CompatCapabilities, CompatEngine, CompatEngineConfig, CompatRequestParameters,
};

/// Default request timeout, in seconds, when `OPENAI_COMPATIBLE_TIMEOUT_SECONDS`
/// is unset.
pub const OPENAI_COMPATIBLE_DEFAULT_TIMEOUT_SECONDS: u64 = 60;

/// Capabilities the operator declares for the endpoint `OpenAiCompatibleAdapter`
/// is pointed at.
///
/// **The rule this struct exists to enforce: unset means the conservative
/// answer.** This adapter must never claim a capability nobody asserted —
/// Phase 14 already paid once to fix a capability flag that over-reported,
/// and an adapter pointed at an unknown endpoint is the easiest place to
/// reintroduce that defect (D-04, 17-RESEARCH.md Pitfall 5). The next person
/// adding a field to this struct must default it `false`/`None` unless it
/// describes baseline OpenAI-compatible spec behaviour the way streaming
/// does.
///
/// Implements [`Deserialize`] so a structured config-file block can supply
/// it (Task 1's resolved naming, "config-file block" half); [`Self::from_env`]
/// reads the individual environment variables (the "individual environment
/// variables" half) so the provider is fully usable with no config file at
/// all.
#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiCompatibleCapabilitiesConfig {
    /// Whether the configured endpoint's request path supports streaming.
    /// The **only** field in this struct defaulting `true` — streaming is
    /// baseline OpenAI-compatible spec behaviour, not an add-on.
    #[serde(default = "default_true")]
    pub supports_streaming: bool,
    /// Whether the configured endpoint's request path can carry a tool
    /// definition. `false` unless declared — `LlmRequest` has no tools
    /// field today regardless (RESEARCH.md Pitfall 4).
    #[serde(default)]
    pub supports_tool_calling: bool,
    /// Whether the configured endpoint's response path returns a populated
    /// function call. `false` unless declared.
    #[serde(default)]
    pub supports_function_calling: bool,
    /// Whether the configured endpoint's request path can carry image
    /// content. `false` unless declared.
    #[serde(default)]
    pub supports_vision: bool,
    /// Whether the configured endpoint exposes embeddings generation.
    /// `false` unless declared.
    #[serde(default)]
    pub supports_embeddings: bool,
    /// Whether the configured endpoint's request path supports a system
    /// message. **Not** defaulted `true` — system-message support is not
    /// universal across compatible gateways, so an unset value must mean
    /// the conservative answer (unlike the presets built on this same
    /// engine, which each assert this from direct vendor knowledge).
    #[serde(default)]
    pub supports_system_messages: bool,
    /// The configured endpoint's advertised maximum context window, in
    /// tokens. `None` when unset — there is no defensible default for an
    /// arbitrary endpoint.
    #[serde(default)]
    pub max_context_tokens: Option<u32>,
    /// The configured endpoint's valid temperature range. `None` when
    /// unset, so ADR-0004's global `[0.0, 1.0]` applies (D-00q).
    #[serde(default)]
    pub temperature_range: Option<(f32, f32)>,
}

/// The **only** default-value helper in this file returning a true boolean.
/// See [`OpenAiCompatibleCapabilitiesConfig::supports_streaming`] for why.
fn default_true() -> bool {
    true
}

/// Parse an optional environment-variable string as a boolean capability
/// declaration.
///
/// `None` (variable unset) yields `default`. A variable that **is** set but
/// does not parse as `true`/`false` (case-insensitively) is a configuration
/// error — never silently coerced to `true` or `false` (Task 2's own
/// behavior spec).
fn parse_bool_env_value(
    var_name: &'static str,
    value: Option<String>,
    default: bool,
) -> Result<bool, String> {
    match value {
        None => Ok(default),
        Some(raw) => match raw.trim().to_ascii_lowercase().as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            other => Err(format!(
                "Invalid {var_name} value {other:?} — expected \"true\" or \"false\""
            )),
        },
    }
}

/// Parse an optional environment-variable string as `Option<u32>`.
///
/// `None` (variable unset) yields `None`. A variable that is set but does
/// not parse as a `u32` is a configuration error.
fn parse_u32_env_value(
    var_name: &'static str,
    value: Option<String>,
) -> Result<Option<u32>, String> {
    match value {
        None => Ok(None),
        Some(raw) => raw
            .trim()
            .parse::<u32>()
            .map(Some)
            .map_err(|_| format!("Invalid {var_name} value {raw:?} — expected an integer")),
    }
}

/// Parse the `OPENAI_COMPATIBLE_TEMPERATURE_MIN` / `_MAX` pair as
/// `Option<(f32, f32)>`.
///
/// Both unset yields `None` (ADR-0004's global range applies). Both set and
/// both finite with `min <= max` yields `Some((min, max))` — equal bounds
/// are accepted; pinning a provider to one temperature is a legitimate
/// declaration. Exactly one set is a configuration error — a half-declared
/// range is not a defensible partial state.
///
/// Both set, either non-finite (`NaN`, `inf`, `-inf`), or `min > max`, is
/// also a configuration error naming the offending variable(s) and value(s)
/// (WR-02). This function never repairs a bad declaration — it never swaps
/// an inverted pair, clamps a non-finite bound, or falls back to a default.
/// An operator whose configuration were silently corrected would never
/// learn it was wrong, and the tuple this function emits must stay ordered
/// `(min, max)` to match every curated preset's
/// `ProviderCapabilities::temperature_range` in this crate.
fn parse_temperature_range_env(
    min: Option<String>,
    max: Option<String>,
) -> Result<Option<(f32, f32)>, String> {
    match (min, max) {
        (None, None) => Ok(None),
        (Some(min_raw), Some(max_raw)) => {
            let min: f32 = min_raw.trim().parse().map_err(|_| {
                format!("Invalid OPENAI_COMPATIBLE_TEMPERATURE_MIN value {min_raw:?} — expected a number")
            })?;
            let max: f32 = max_raw.trim().parse().map_err(|_| {
                format!("Invalid OPENAI_COMPATIBLE_TEMPERATURE_MAX value {max_raw:?} — expected a number")
            })?;
            // Finiteness must be checked before ordering: "NaN".parse::<f32>()
            // succeeds, and every comparison against NaN is `false` in both
            // directions, so an ordering guard placed first would pass
            // (NaN, NaN) — and (NaN, x), (x, NaN) — straight through as a
            // range no downstream comparison could ever resolve.
            if !min.is_finite() {
                return Err(format!(
                    "Invalid OPENAI_COMPATIBLE_TEMPERATURE_MIN value {min} — must be a finite number"
                ));
            }
            if !max.is_finite() {
                return Err(format!(
                    "Invalid OPENAI_COMPATIBLE_TEMPERATURE_MAX value {max} — must be a finite number"
                ));
            }
            // Strictly-greater, never >=: equal bounds are a legitimate
            // single-point declaration and must stay accepted.
            if min > max {
                return Err(format!(
                    "OPENAI_COMPATIBLE_TEMPERATURE_MIN value {min} must not exceed \
                     OPENAI_COMPATIBLE_TEMPERATURE_MAX value {max}"
                ));
            }
            Ok(Some((min, max)))
        }
        (Some(_), None) => Err(
            "OPENAI_COMPATIBLE_TEMPERATURE_MIN is set but OPENAI_COMPATIBLE_TEMPERATURE_MAX is not \
             — both or neither must be set"
                .to_string(),
        ),
        (None, Some(_)) => Err(
            "OPENAI_COMPATIBLE_TEMPERATURE_MAX is set but OPENAI_COMPATIBLE_TEMPERATURE_MIN is not \
             — both or neither must be set"
                .to_string(),
        ),
    }
}

impl OpenAiCompatibleCapabilitiesConfig {
    /// Read capability declarations from the individual
    /// `OPENAI_COMPATIBLE_SUPPORTS_*` / `_MAX_CONTEXT_TOKENS` /
    /// `_TEMPERATURE_MIN` / `_TEMPERATURE_MAX` environment variables.
    ///
    /// # Errors
    /// Returns an error naming the specific variable when it is set to a
    /// value that fails to parse. Never silently defaults an unparseable
    /// value to `true` or `false`.
    pub fn from_env() -> Result<Self, String> {
        Self::from_parts(
            env::var("OPENAI_COMPATIBLE_SUPPORTS_STREAMING").ok(),
            env::var("OPENAI_COMPATIBLE_SUPPORTS_TOOL_CALLING").ok(),
            env::var("OPENAI_COMPATIBLE_SUPPORTS_FUNCTION_CALLING").ok(),
            env::var("OPENAI_COMPATIBLE_SUPPORTS_VISION").ok(),
            env::var("OPENAI_COMPATIBLE_SUPPORTS_EMBEDDINGS").ok(),
            env::var("OPENAI_COMPATIBLE_SUPPORTS_SYSTEM_MESSAGES").ok(),
            env::var("OPENAI_COMPATIBLE_MAX_CONTEXT_TOKENS").ok(),
            env::var("OPENAI_COMPATIBLE_TEMPERATURE_MIN").ok(),
            env::var("OPENAI_COMPATIBLE_TEMPERATURE_MAX").ok(),
        )
    }

    /// The pure defaulting/validation logic behind [`Self::from_env`],
    /// separated out so it is testable without mutating process environment
    /// variables — `std::env::set_var` is `unsafe` under Rust 2024 and this
    /// crate denies `unsafe_code` (`#![deny(unsafe_code)]`).
    #[allow(clippy::too_many_arguments)]
    fn from_parts(
        supports_streaming: Option<String>,
        supports_tool_calling: Option<String>,
        supports_function_calling: Option<String>,
        supports_vision: Option<String>,
        supports_embeddings: Option<String>,
        supports_system_messages: Option<String>,
        max_context_tokens: Option<String>,
        temperature_min: Option<String>,
        temperature_max: Option<String>,
    ) -> Result<Self, String> {
        Ok(Self {
            supports_streaming: parse_bool_env_value(
                "OPENAI_COMPATIBLE_SUPPORTS_STREAMING",
                supports_streaming,
                true,
            )?,
            supports_tool_calling: parse_bool_env_value(
                "OPENAI_COMPATIBLE_SUPPORTS_TOOL_CALLING",
                supports_tool_calling,
                false,
            )?,
            supports_function_calling: parse_bool_env_value(
                "OPENAI_COMPATIBLE_SUPPORTS_FUNCTION_CALLING",
                supports_function_calling,
                false,
            )?,
            supports_vision: parse_bool_env_value(
                "OPENAI_COMPATIBLE_SUPPORTS_VISION",
                supports_vision,
                false,
            )?,
            supports_embeddings: parse_bool_env_value(
                "OPENAI_COMPATIBLE_SUPPORTS_EMBEDDINGS",
                supports_embeddings,
                false,
            )?,
            supports_system_messages: parse_bool_env_value(
                "OPENAI_COMPATIBLE_SUPPORTS_SYSTEM_MESSAGES",
                supports_system_messages,
                false,
            )?,
            max_context_tokens: parse_u32_env_value(
                "OPENAI_COMPATIBLE_MAX_CONTEXT_TOKENS",
                max_context_tokens,
            )?,
            temperature_range: parse_temperature_range_env(temperature_min, temperature_max)?,
        })
    }
}

/// Configuration for the generic operator-configured OpenAI-compatible
/// adapter (D-03).
///
/// Unlike every named preset in this crate, every field here — including
/// `base_url` and `model` — is **required**, with no defensible default:
/// there is no vendor to inherit an endpoint or a model identifier from.
#[derive(Debug, Clone)]
pub struct OpenAiCompatibleConfig {
    /// API key for the configured endpoint.
    pub api_key: String,
    /// Base URL for the configured endpoint. See the module-level "Trust
    /// boundary" section — this value determines where the API key is
    /// sent.
    pub base_url: String,
    /// Default model to request.
    pub model: String,
    /// Request timeout in seconds.
    pub timeout_seconds: u64,
    /// Operator-declared capabilities (D-04).
    pub capabilities: OpenAiCompatibleCapabilitiesConfig,
}

impl OpenAiCompatibleConfig {
    /// Load configuration from environment variables.
    ///
    /// # Environment Variables
    /// See the module-level table. `OPENAI_COMPATIBLE_API_KEY`,
    /// `OPENAI_COMPATIBLE_BASE_URL` and `OPENAI_COMPATIBLE_MODEL` are all
    /// required.
    ///
    /// # Errors
    /// Returns an error naming the specific missing or unparseable
    /// variable.
    pub fn from_env() -> Result<Self, String> {
        Self::from_parts(
            env::var("OPENAI_COMPATIBLE_API_KEY").ok(),
            env::var("OPENAI_COMPATIBLE_BASE_URL").ok(),
            env::var("OPENAI_COMPATIBLE_MODEL").ok(),
            env::var("OPENAI_COMPATIBLE_TIMEOUT_SECONDS").ok(),
            OpenAiCompatibleCapabilitiesConfig::from_env()?,
        )
    }

    /// The pure defaulting/validation logic behind [`Self::from_env`],
    /// separated out so it is testable without mutating process environment
    /// variables.
    fn from_parts(
        api_key: Option<String>,
        base_url: Option<String>,
        model: Option<String>,
        timeout_seconds: Option<String>,
        capabilities: OpenAiCompatibleCapabilitiesConfig,
    ) -> Result<Self, String> {
        let api_key = api_key
            .ok_or_else(|| "OPENAI_COMPATIBLE_API_KEY environment variable not set".to_string())?;
        let base_url = base_url.ok_or_else(|| {
            "OPENAI_COMPATIBLE_BASE_URL environment variable not set — there is no defensible \
             default endpoint for an operator-configured provider"
                .to_string()
        })?;
        let model = model.ok_or_else(|| {
            "OPENAI_COMPATIBLE_MODEL environment variable not set — there is no defensible \
             default model for an operator-configured provider"
                .to_string()
        })?;
        let timeout_seconds = timeout_seconds
            .unwrap_or_else(|| OPENAI_COMPATIBLE_DEFAULT_TIMEOUT_SECONDS.to_string())
            .parse()
            .map_err(|_| "Invalid OPENAI_COMPATIBLE_TIMEOUT_SECONDS value".to_string())?;

        let config = Self {
            api_key,
            base_url,
            model,
            timeout_seconds,
            capabilities,
        };

        config.validate()?;
        Ok(config)
    }

    /// Create configuration with custom values.
    pub fn new(
        api_key: String,
        base_url: String,
        model: String,
        capabilities: OpenAiCompatibleCapabilitiesConfig,
    ) -> Self {
        Self {
            api_key,
            base_url,
            model,
            timeout_seconds: OPENAI_COMPATIBLE_DEFAULT_TIMEOUT_SECONDS,
            capabilities,
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

/// `true` when `base_url` uses the `http` scheme and its host is not a
/// loopback address (T-17-22). Parse failures are treated as "cannot prove
/// it's safe" and therefore do not warn — this is a diagnostic aid, not a
/// security control, so it fails closed on the side of silence rather than
/// panicking or blocking construction on a URL it cannot parse.
fn is_plaintext_to_non_loopback_host(base_url: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(base_url) else {
        return false;
    };
    if url.scheme() != "http" {
        return false;
    }
    match url.host_str() {
        Some("localhost") => false,
        Some(host) => !host
            .parse::<IpAddr>()
            .map(|ip| ip.is_loopback())
            .unwrap_or(false),
        None => false,
    }
}

/// The provider name this adapter reports through [`LlmPort::get_provider_name`]
/// and that its engine stamps on every `LlmError::ProviderError` (Phase 25
/// D-03). Deliberately host-independent (D-07/D-09/T-17-20).
const OPENAI_COMPATIBLE_PROVIDER: &str = "openai-compatible";

/// Generic operator-configured OpenAI-compatible LLM Adapter implementing
/// [`LlmPort`].
///
/// Every method delegates to an owned [`CompatEngine`] (D-05) — this struct
/// carries no protocol logic of its own.
pub struct OpenAiCompatibleAdapter {
    engine: CompatEngine,
}

impl OpenAiCompatibleAdapter {
    /// Create a new generic OpenAI-compatible adapter.
    ///
    /// # Errors
    /// Returns an error if configuration is invalid or the underlying HTTP
    /// client cannot be created.
    pub fn new(config: OpenAiCompatibleConfig) -> Result<Self, LlmError> {
        config.validate().map_err(|e| {
            LlmError::AuthenticationError(format!("Invalid openai-compatible configuration: {}", e))
        })?;

        if is_plaintext_to_non_loopback_host(&config.base_url) {
            log::warn!(
                "openai-compatible base_url uses plain HTTP to a non-loopback host — the \
                 configured API key will be transmitted in clear text; set OPENAI_COMPATIBLE_BASE_URL \
                 to an https:// endpoint unless this is a deliberately trusted local network"
            );
        }

        let engine_config = CompatEngineConfig {
            base_url: config.base_url,
            api_key: config.api_key,
            model: config.model,
            timeout_seconds: config.timeout_seconds,
            max_retries: 3,
            capabilities: CompatCapabilities {
                supports_streaming: config.capabilities.supports_streaming,
                supports_tool_calling: config.capabilities.supports_tool_calling,
                supports_function_calling: config.capabilities.supports_function_calling,
                supports_vision: config.capabilities.supports_vision,
                supports_embeddings: config.capabilities.supports_embeddings,
                max_context_tokens: config.capabilities.max_context_tokens,
                supports_system_messages: config.capabilities.supports_system_messages,
                temperature_range: config.capabilities.temperature_range,
            },
            // Unchanged pre-existing behaviour (17-18): no vendor-specific
            // sampling-parameter restriction has been measured for this
            // generic, operator-configured endpoint. Exposing this as an
            // operator knob is deliberately deferred (17-18's Deferred
            // section) — it widens D-04's operator-declared surface and its
            // env-var contract, and is not taken here without a concrete
            // report of a self-hosted endpoint rejecting a parameter.
            request_parameters: CompatRequestParameters::all(),
            // No vendor-curated fallback list exists for an arbitrary
            // operator-configured endpoint (unlike the named presets, which
            // each ship a curated list per D-13). An empty live `/models`
            // response or a failed fetch resolves to an empty list here,
            // which is the honest answer for an endpoint this adapter knows
            // nothing about in advance.
            fallback_models: Vec::new(),
            error_override: None,
            // T-17-18: this adapter's base_url is entirely operator-supplied
            // (unlike every other preset's fixed vendor host), so a redirect
            // response must never be followed — see the module-level "Trust
            // boundary" doc section.
            redirect_policy: Some(reqwest::redirect::Policy::none()),
        };

        Ok(Self {
            // Phase 25 D-03: name the engine explicitly (it matches the
            // engine's own default, but the preset owns its identity).
            engine: CompatEngine::new(engine_config)?
                .with_provider_name(OPENAI_COMPATIBLE_PROVIDER),
        })
    }
}

#[async_trait]
impl LlmPort for OpenAiCompatibleAdapter {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        self.engine.generate(request).await
    }

    async fn generate_stream(
        &self,
        request: LlmRequest,
    ) -> Result<Box<dyn Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError> {
        self.engine.generate_stream(request).await
    }

    async fn validate_model(&self, model: &str) -> Result<bool, LlmError> {
        Ok(self.engine.validate_model(model).await)
    }

    async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
        Ok(self.engine.available_models().await)
    }

    /// Returns the fixed literal `"openai-compatible"`, regardless of the
    /// endpoint configured (D-07/D-09).
    ///
    /// **Accepted cost (T-17-20, the plan's assumption-delta decision):**
    /// two `OpenAiCompatibleAdapter` instances pointed at different
    /// endpoints are indistinguishable in logs, metrics and error messages
    /// — both report `"openai-compatible"`. This is deliberate: widening
    /// `LlmPort::get_provider_name() -> &'static str` to `-> &str` was
    /// rejected for this phase as a breaking change to a public port trait
    /// that moves every adapter signature, the mock, and
    /// `.project/current-exports.txt`. The trigger for revisiting this: any
    /// future phase already making a breaking `LlmPort` change (near-zero
    /// marginal cost to widen then), or the first operator-visible incident
    /// where two generic instances cannot be told apart in diagnostics.
    ///
    /// This value never contains the configured `base_url` or any other
    /// operator-supplied value (D-07) — it is a compile-time constant.
    fn get_provider_name(&self) -> &'static str {
        OPENAI_COMPATIBLE_PROVIDER
    }

    fn get_capabilities(&self) -> ProviderCapabilities {
        self.engine.capabilities()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http_status::map_http_status;
    use mockito::Server;
    use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
    use paladin_ports::output::llm_port::FinishReason;
    use serde_json::json;
    use std::collections::HashMap;
    use uuid::Uuid;

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

    fn default_capabilities() -> OpenAiCompatibleCapabilitiesConfig {
        OpenAiCompatibleCapabilitiesConfig::from_parts(
            None, None, None, None, None, None, None, None, None,
        )
        .unwrap()
    }

    // ── D-04 pessimistic-default posture (RESEARCH.md Pitfall 5's Red step) ──

    #[test]
    fn deserializing_empty_json_object_yields_pessimistic_defaults() {
        let caps: OpenAiCompatibleCapabilitiesConfig = serde_json::from_str("{}").unwrap();
        assert!(
            caps.supports_streaming,
            "streaming is the sole true-by-default field"
        );
        assert!(!caps.supports_tool_calling);
        assert!(!caps.supports_function_calling);
        assert!(!caps.supports_vision);
        assert!(!caps.supports_embeddings);
        assert!(!caps.supports_system_messages);
        assert_eq!(caps.max_context_tokens, None);
        assert_eq!(caps.temperature_range, None);
    }

    #[test]
    fn from_env_with_no_capability_variables_set_yields_the_identical_pessimistic_defaults() {
        let caps = default_capabilities();
        assert!(caps.supports_streaming);
        assert!(!caps.supports_tool_calling);
        assert!(!caps.supports_function_calling);
        assert!(!caps.supports_vision);
        assert!(!caps.supports_embeddings);
        assert!(!caps.supports_system_messages);
        assert_eq!(caps.max_context_tokens, None);
        assert_eq!(caps.temperature_range, None);
    }

    #[test]
    fn setting_a_capability_to_true_is_reflected() {
        let caps = OpenAiCompatibleCapabilitiesConfig::from_parts(
            None,
            Some("true".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        assert!(caps.supports_tool_calling);
    }

    #[test]
    fn setting_a_capability_to_an_unparseable_value_is_a_configuration_error() {
        let result = OpenAiCompatibleCapabilitiesConfig::from_parts(
            None,
            Some("maybe".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        assert!(
            result.is_err(),
            "an unparseable capability value must error, never silently default"
        );
        let message = result.unwrap_err();
        assert!(message.contains("OPENAI_COMPATIBLE_SUPPORTS_TOOL_CALLING"));
    }

    #[test]
    fn max_context_tokens_set_to_an_unparseable_value_is_a_configuration_error() {
        let result = OpenAiCompatibleCapabilitiesConfig::from_parts(
            None,
            None,
            None,
            None,
            None,
            None,
            Some("not-a-number".to_string()),
            None,
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn temperature_range_requires_both_min_and_max_or_neither() {
        let only_min = OpenAiCompatibleCapabilitiesConfig::from_parts(
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("0.0".to_string()),
            None,
        );
        assert!(only_min.is_err());

        let both = OpenAiCompatibleCapabilitiesConfig::from_parts(
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("0.0".to_string()),
            Some("2.0".to_string()),
        )
        .unwrap();
        assert_eq!(both.temperature_range, Some((0.0, 2.0)));
    }

    // ── WR-02: a declared temperature range must be a range ──

    #[test]
    fn parse_temperature_range_env_rejects_an_inverted_range() {
        // The realistic operator mistake: a copy-paste transposition of MIN
        // and MAX. Every curated preset in this crate declares its range
        // ordered (min, max) — GrokConfig's Some((0.0, 2.0)), KimiConfig's
        // Some((0.0, 1.0)) — and an inverted tuple silently accepted here
        // would violate that crate-wide convention for every downstream
        // consumer that clamps into range.0..=range.1.
        let result = parse_temperature_range_env(Some("2.0".to_string()), Some("0.0".to_string()));
        assert!(result.is_err(), "an inverted range must be rejected");
        let message = result.unwrap_err();
        assert!(message.contains("OPENAI_COMPATIBLE_TEMPERATURE_MIN"));
        assert!(message.contains("OPENAI_COMPATIBLE_TEMPERATURE_MAX"));
        assert!(message.contains('2'));
        assert!(message.contains('0'));
    }

    #[test]
    fn parse_temperature_range_env_accepts_equal_bounds() {
        // The check is strictly-greater, not greater-or-equal: pinning a
        // provider to one temperature is a legitimate operator declaration.
        // Rejecting it would be a regression this test exists to prevent.
        // This test passes today.
        let result = parse_temperature_range_env(Some("1.0".to_string()), Some("1.0".to_string()));
        assert_eq!(result, Ok(Some((1.0, 1.0))));
    }

    #[test]
    fn parse_temperature_range_env_accepts_an_ordered_range() {
        // Positive control: the ordinary case. Proves the new guards did not
        // turn into a rejection of everything. This test passes today.
        let result = parse_temperature_range_env(Some("0.0".to_string()), Some("2.0".to_string()));
        assert_eq!(result, Ok(Some((0.0, 2.0))));
    }

    #[test]
    fn parse_temperature_range_env_rejects_a_nan_bound() {
        // `f32` parsing accepts "NaN", and every comparison against NaN is
        // `false` in both directions — an ordering guard (`min > max`) alone
        // would let (NaN, 1.0) pass straight through. This is why
        // finiteness is checked separately from the ordering comparison.
        let result = parse_temperature_range_env(Some("NaN".to_string()), Some("1.0".to_string()));
        assert!(result.is_err(), "a NaN bound must be rejected");
        let message = result.unwrap_err();
        assert!(message.contains("OPENAI_COMPATIBLE_TEMPERATURE_MIN"));
    }

    #[test]
    fn parse_temperature_range_env_rejects_an_infinite_bound() {
        let result = parse_temperature_range_env(Some("0.0".to_string()), Some("inf".to_string()));
        assert!(result.is_err(), "an infinite bound must be rejected");
        let message = result.unwrap_err();
        assert!(message.contains("OPENAI_COMPATIBLE_TEMPERATURE_MAX"));
    }

    #[test]
    fn parse_temperature_range_env_half_set_diagnostics_are_unchanged() {
        // Regression guard: the new ordering/finiteness arms must not
        // reorder or reword the existing both-or-neither diagnostics.
        let only_min = parse_temperature_range_env(Some("0.0".to_string()), None);
        assert!(only_min.is_err());
        let only_min_message = only_min.unwrap_err();
        assert!(only_min_message.contains("both or neither"));

        let only_max = parse_temperature_range_env(None, Some("0.0".to_string()));
        assert!(only_max.is_err());
        let only_max_message = only_max.unwrap_err();
        assert!(only_max_message.contains("both or neither"));
    }

    // ── OpenAiCompatibleConfig::from_env() required fields ──

    #[test]
    fn from_env_errors_naming_base_url_when_base_url_absent() {
        let result = OpenAiCompatibleConfig::from_parts(
            Some("test-key".to_string()),
            None,
            Some("some-model".to_string()),
            None,
            default_capabilities(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("OPENAI_COMPATIBLE_BASE_URL"));
    }

    #[test]
    fn from_env_errors_naming_model_when_model_absent() {
        let result = OpenAiCompatibleConfig::from_parts(
            Some("test-key".to_string()),
            Some("https://example.invalid/v1".to_string()),
            None,
            None,
            default_capabilities(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("OPENAI_COMPATIBLE_MODEL"));
    }

    #[test]
    fn from_env_errors_naming_api_key_when_api_key_absent() {
        let result = OpenAiCompatibleConfig::from_parts(
            None,
            Some("https://example.invalid/v1".to_string()),
            Some("some-model".to_string()),
            None,
            default_capabilities(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("OPENAI_COMPATIBLE_API_KEY"));
    }

    #[test]
    fn from_env_succeeds_and_defaults_timeout_when_every_required_field_is_present() {
        let config = OpenAiCompatibleConfig::from_parts(
            Some("test-key".to_string()),
            Some("https://example.invalid/v1".to_string()),
            Some("some-model".to_string()),
            None,
            default_capabilities(),
        )
        .unwrap();
        assert_eq!(
            config.timeout_seconds,
            OPENAI_COMPATIBLE_DEFAULT_TIMEOUT_SECONDS
        );
    }

    // ── Request shaping / response parsing ──

    #[tokio::test]
    async fn generate_posts_to_configured_base_url_chat_completions() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/chat/completions")
            .match_header("authorization", "Bearer test-key")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "id": "cmpl-1",
                    "model": "some-model",
                    "choices": [{
                        "index": 0,
                        "message": {"role": "assistant", "content": "Hi there"},
                        "finish_reason": "stop"
                    }],
                    "usage": {"prompt_tokens": 5, "completion_tokens": 3, "total_tokens": 8}
                })
                .to_string(),
            )
            .create_async()
            .await;

        let config = OpenAiCompatibleConfig::new(
            "test-key".to_string(),
            server.url(),
            "some-model".to_string(),
            default_capabilities(),
        );
        let adapter = OpenAiCompatibleAdapter::new(config).unwrap();

        let response = adapter
            .generate(build_request("some-model"))
            .await
            .expect("mock server returned a well-formed response");

        assert_eq!(response.content, "Hi there");
        assert!(matches!(response.finish_reason, FinishReason::Stop));
        mock.assert_async().await;
    }

    // ── Streaming ──

    #[tokio::test]
    async fn generate_stream_assembles_deltas_in_wire_order_with_terminal_stop() {
        use futures::StreamExt;

        let sse_body = concat!(
            "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"Hel\"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"lo \"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"world\"},\"finish_reason\":\"stop\"}]}\n\n",
            "data: [DONE]\n\n",
        );

        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(sse_body)
            .create_async()
            .await;

        let config = OpenAiCompatibleConfig::new(
            "test-key".to_string(),
            server.url(),
            "some-model".to_string(),
            default_capabilities(),
        );
        let adapter = OpenAiCompatibleAdapter::new(config).unwrap();

        let stream = adapter
            .generate_stream(build_request("some-model"))
            .await
            .unwrap();
        let mut stream = Box::into_pin(stream);

        let mut assembled = String::new();
        let mut last_finish_reason = None;
        while let Some(item) = stream.next().await {
            let chunk = item.unwrap();
            assembled.push_str(&chunk.delta);
            if chunk.finish_reason.is_some() {
                last_finish_reason = chunk.finish_reason;
            }
        }

        assert_eq!(assembled, "Hello world");
        assert!(matches!(last_finish_reason, Some(FinishReason::Stop)));
    }

    // ── Error mapping ──

    #[tokio::test]
    async fn generate_maps_401_to_authentication_error() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(401)
            .with_body("invalid api key")
            .create_async()
            .await;

        let config = OpenAiCompatibleConfig::new(
            "test-key".to_string(),
            server.url(),
            "some-model".to_string(),
            default_capabilities(),
        );
        let adapter = OpenAiCompatibleAdapter::new(config).unwrap();

        let result = adapter.generate(build_request("some-model")).await;
        assert!(matches!(result, Err(LlmError::AuthenticationError(_))));
    }

    #[tokio::test]
    async fn generate_maps_429_to_rate_limit_exceeded() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(429)
            .with_body("slow down")
            .create_async()
            .await;

        let config = OpenAiCompatibleConfig::new(
            "test-key".to_string(),
            server.url(),
            "some-model".to_string(),
            default_capabilities(),
        );
        let adapter = OpenAiCompatibleAdapter::new(config).unwrap();

        let result = adapter.generate(build_request("some-model")).await;
        assert!(matches!(result, Err(LlmError::RateLimitExceeded)));
    }

    #[tokio::test]
    async fn generate_maps_404_to_model_not_available() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(404)
            .with_body("no such model")
            .create_async()
            .await;

        let config = OpenAiCompatibleConfig::new(
            "test-key".to_string(),
            server.url(),
            "some-model".to_string(),
            default_capabilities(),
        );
        let adapter = OpenAiCompatibleAdapter::new(config).unwrap();

        let result = adapter.generate(build_request("some-model")).await;
        assert!(matches!(result, Err(LlmError::ModelNotAvailable(_))));
    }

    #[tokio::test]
    async fn generate_maps_400_to_invalid_prompt() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(400)
            .with_body("bad request")
            .create_async()
            .await;

        let config = OpenAiCompatibleConfig::new(
            "test-key".to_string(),
            server.url(),
            "some-model".to_string(),
            default_capabilities(),
        );
        let adapter = OpenAiCompatibleAdapter::new(config).unwrap();

        let result = adapter.generate(build_request("some-model")).await;
        assert!(matches!(result, Err(LlmError::InvalidPrompt(_))));
    }

    /// Phase 25 (FT-FR-01, D-03): the preset's non-2xx path is the shared
    /// `map_http_status` — proven by comparing the adapter's error against
    /// the helper's own output for the same status, body and key.
    #[tokio::test]
    async fn openai_compatible_non_2xx_routes_through_the_shared_mapper() {
        let body = r#"{"error":"overloaded"}"#;
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(503)
            .with_body(body)
            .expect_at_least(1)
            .create_async()
            .await;

        let config = OpenAiCompatibleConfig::new(
            "test-key".to_string(),
            server.url(),
            "some-model".to_string(),
            default_capabilities(),
        );
        let adapter = OpenAiCompatibleAdapter::new(config).unwrap();

        let result = adapter.generate(build_request("some-model")).await;
        let expected = map_http_status("openai-compatible", 503, body, "test-key");
        match (result, expected) {
            (
                Err(LlmError::ProviderError {
                    provider,
                    status,
                    message,
                }),
                LlmError::ProviderError {
                    provider: want_provider,
                    status: want_status,
                    message: want_message,
                },
            ) => {
                assert_eq!(provider, want_provider);
                assert_eq!(status, want_status);
                assert_eq!(message, want_message);
            }
            (other, _) => panic!("expected ProviderError {{ status: 503 }}, got {other:?}"),
        }
    }

    // ── Provider identity (D-07/D-09/T-17-20) ──

    #[test]
    fn get_provider_name_returns_the_same_literal_for_two_different_base_urls() {
        let config_a = OpenAiCompatibleConfig::new(
            "test-key".to_string(),
            "https://gateway-a.example.invalid/v1".to_string(),
            "some-model".to_string(),
            default_capabilities(),
        );
        let config_b = OpenAiCompatibleConfig::new(
            "test-key".to_string(),
            "https://gateway-b.example.invalid/v1".to_string(),
            "some-model".to_string(),
            default_capabilities(),
        );
        let adapter_a = OpenAiCompatibleAdapter::new(config_a).unwrap();
        let adapter_b = OpenAiCompatibleAdapter::new(config_b).unwrap();

        assert_eq!(adapter_a.get_provider_name(), "openai-compatible");
        assert_eq!(adapter_b.get_provider_name(), "openai-compatible");
    }

    #[test]
    fn get_provider_name_never_contains_the_configured_host() {
        let config = OpenAiCompatibleConfig::new(
            "test-key".to_string(),
            "https://a-very-distinctive-hostname.example.invalid/v1".to_string(),
            "some-model".to_string(),
            default_capabilities(),
        );
        let adapter = OpenAiCompatibleAdapter::new(config).unwrap();
        let name = adapter.get_provider_name();
        assert!(!name.contains("a-very-distinctive-hostname"));
    }

    // ── Capabilities are configuration-only, never runtime-inferred (T-17-19) ──

    #[tokio::test]
    async fn a_models_response_with_a_fabricated_capability_field_does_not_change_capabilities() {
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/models")
            .with_status(200)
            .with_body(
                json!({
                    "data": [
                        {"id": "some-model", "supports_tool_calling": true, "context_length": 999_999}
                    ]
                })
                .to_string(),
            )
            .create_async()
            .await;

        let config = OpenAiCompatibleConfig::new(
            "test-key".to_string(),
            server.url(),
            "some-model".to_string(),
            default_capabilities(),
        );
        let adapter = OpenAiCompatibleAdapter::new(config).unwrap();

        let before = adapter.get_capabilities();
        let _ = adapter.get_available_models().await.unwrap();
        let after = adapter.get_capabilities();

        assert_eq!(before, after);
        assert!(!after.supports_tool_calling);
        assert_eq!(after.max_context_tokens, None);
    }

    #[test]
    fn get_capabilities_reports_exactly_what_was_declared() {
        let capabilities = OpenAiCompatibleCapabilitiesConfig::from_parts(
            None,
            Some("true".to_string()),
            None,
            None,
            None,
            Some("true".to_string()),
            Some("32768".to_string()),
            Some("0.0".to_string()),
            Some("1.5".to_string()),
        )
        .unwrap();
        let config = OpenAiCompatibleConfig::new(
            "test-key".to_string(),
            "https://example.invalid/v1".to_string(),
            "some-model".to_string(),
            capabilities,
        );
        let adapter = OpenAiCompatibleAdapter::new(config).unwrap();
        let caps = adapter.get_capabilities();

        assert!(caps.supports_streaming);
        assert!(caps.supports_tool_calling);
        assert!(!caps.supports_function_calling);
        assert!(!caps.supports_vision);
        assert!(!caps.supports_embeddings);
        assert!(caps.supports_system_messages);
        assert_eq!(caps.max_context_tokens, Some(32768));
        assert_eq!(caps.temperature_range, Some((0.0, 1.5)));
    }

    // ── Plaintext-to-non-loopback detection (T-17-22) ──

    #[test]
    fn plaintext_http_to_a_remote_host_is_flagged() {
        assert!(is_plaintext_to_non_loopback_host(
            "http://remote-gateway.example.invalid/v1"
        ));
    }

    #[test]
    fn plaintext_http_to_localhost_is_not_flagged() {
        assert!(!is_plaintext_to_non_loopback_host(
            "http://localhost:8000/v1"
        ));
    }

    #[test]
    fn plaintext_http_to_a_loopback_ip_is_not_flagged() {
        assert!(!is_plaintext_to_non_loopback_host(
            "http://127.0.0.1:8000/v1"
        ));
    }

    #[test]
    fn https_to_a_remote_host_is_not_flagged() {
        assert!(!is_plaintext_to_non_loopback_host(
            "https://remote-gateway.example.invalid/v1"
        ));
    }
}
