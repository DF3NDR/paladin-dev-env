//! LLM provider configuration types for the paladin-llm crate.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use paladin_ports::output::llm_port::{ReasoningEffort, ReasoningFallback};

use crate::provider_factory::provider_names;
use crate::reasoning::ReasoningWire;
use crate::redaction::RedactedKey;
use crate::registry::ModelOverride;

/// Configuration for an individual LLM provider: one `llm.<provider>` block.
///
/// Every field but `api_key` is optional. Plan 50-09 (PARITY-03; D-06, D-12, D-13) gave the block
/// the same knobs the nine adapter configs carry, and
/// [`LlmProviderFactory::create_from_config`](crate::provider_factory::LlmProviderFactory::create_from_config)
/// layers it under a per-agent model and over the provider's legacy environment variables.
///
/// Marked `#[non_exhaustive]` (owner decision, Phase 50 plan 50-08, option-a, applied here by
/// plan 50-09): build one with [`LlmProviderConfig::new`] and the `with_*` builders, or
/// deserialize it, so a later field is not another break for callers.
///
/// `Debug` prints the API key as `<redacted>`; the key is a public `String`, so a derived `Debug`
/// would put it into any log line or panic message that formats the block (D-00a).
#[derive(Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LlmProviderConfig {
    /// API key for the provider. May be empty (the provider's own environment variable then
    /// supplies it, D-13) or an exact `${ENV_VAR}` placeholder naming the variable that holds
    /// the key; any other text, including `abc${X}` and `$X`, is used literally.
    ///
    /// Never serialised (WR-04): the block is `Serialize` through `LlmConfig`, `Settings` and
    /// `AgentRuntimeDeps`, so any config dump, snapshot or `--print-config` would otherwise emit
    /// every provider key in plaintext. Omitting the field, rather than writing a placeholder,
    /// means a serialise-then-deserialise round trip yields an empty key (which the provider's
    /// environment variable then supplies) instead of a placeholder string mistaken for a
    /// credential.
    #[serde(default, skip_serializing)]
    pub api_key: String,
    /// Base URL for the API endpoint.
    pub base_url: Option<String>,
    /// Default model to use.
    pub default_model: Option<String>,
    /// Default temperature (0.0–2.0).
    pub default_temperature: Option<f32>,
    /// Default timeout in seconds; at least 1 (a zero timeout is refused when the provider is built).
    pub timeout_seconds: Option<u64>,
    /// Maximum retries for failed requests; at most 10 (a larger value is refused when the
    /// provider is built).
    pub max_retries: Option<u32>,
    /// Default output budget (`max_tokens`) sent when a request sets none. YAML key
    /// `max_tokens`. Beats the provider's legacy environment variable (where it has one) and
    /// the adapter's compiled default; the value is checked against the model's profiled
    /// maximum output exactly as a request's own `max_tokens` is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Adapter-wide context window in tokens. YAML key `context_window`. Replaces every
    /// compiled window of this provider; a `context_window` under [`models`](Self::models) for
    /// one model still wins for that model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u32>,
    /// Per-model overrides (D-06), keyed by exact model id. YAML key `models`, i.e.
    /// `llm.<provider>.models.<model-id>`. Each override is laid over the model's seeded entry
    /// field by field, so a model Paladin has never seen can be described without a release.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub models: BTreeMap<String, ModelOverride>,
    /// The provider-level default reasoning effort (Phase 51, D-00e). YAML key
    /// `reasoning_effort`: a level (`low`, `medium`, `high`, `xhigh`, ...), `off`, `auto` or
    /// `{budget: 2048}`.
    ///
    /// Sent for every request of this provider that sets no effort of its own, resolved per
    /// request against that request's own model; a request's (or an agent's) own effort always
    /// wins. Set it only where every model this provider serves supports it: a request on a
    /// model that cannot honour it is refused before any HTTP call, with a reason naming this
    /// key. No provider has a compiled-in default (D-00i), and there is no environment variable
    /// for it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<ReasoningEffort>,
    /// The provider-level default fallback (Phase 51, D-05). YAML key `reasoning_fallback`:
    /// `refuse`, `nearest_lower` or `nearest_higher`. Applies when the effort in force is not
    /// one the model accepts; a request's own fallback wins. Unset means refuse.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_fallback: Option<ReasoningFallback>,
    /// Where a self-hosted endpoint parses a reasoning effort in the request body (Phase 51,
    /// D-10). YAML key `reasoning_wire`: `top_level_effort`, `thinking_object`,
    /// `enable_thinking` or `unsupported`.
    ///
    /// Accepted only for `ollama` and `openai-compatible`, the endpoints whose wire only the
    /// operator can know. On any named vendor (OpenAI, Anthropic, DeepSeek, Gemini, Kimi, Qwen,
    /// Grok) the block is refused when the provider is built: those wires change only through a
    /// Paladin release, after the dated probe.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_wire: Option<ReasoningWire>,
}

impl std::fmt::Debug for LlmProviderConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmProviderConfig")
            .field("api_key", &RedactedKey(&self.api_key))
            .field("base_url", &self.base_url)
            .field("default_model", &self.default_model)
            .field("default_temperature", &self.default_temperature)
            .field("timeout_seconds", &self.timeout_seconds)
            .field("max_retries", &self.max_retries)
            .field("max_tokens", &self.max_tokens)
            .field("context_window", &self.context_window)
            .field("models", &self.models)
            .field("reasoning_effort", &self.reasoning_effort)
            .field("reasoning_fallback", &self.reasoning_fallback)
            .field("reasoning_wire", &self.reasoning_wire)
            .finish()
    }
}

impl LlmProviderConfig {
    /// A block carrying only `api_key`; every other field is unset and filled from the layers
    /// below it (D-13).
    #[must_use]
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: None,
            default_model: None,
            default_temperature: None,
            timeout_seconds: None,
            max_retries: None,
            max_tokens: None,
            context_window: None,
            models: BTreeMap::new(),
            reasoning_effort: None,
            reasoning_fallback: None,
            reasoning_wire: None,
        }
    }

    /// Set the base URL (`base_url`).
    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// Set the default model (`default_model`).
    #[must_use]
    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = Some(model.into());
        self
    }

    /// Set the default temperature (`default_temperature`).
    #[must_use]
    pub fn with_default_temperature(mut self, temperature: f32) -> Self {
        self.default_temperature = Some(temperature);
        self
    }

    /// Set the request timeout in seconds (`timeout_seconds`).
    #[must_use]
    pub fn with_timeout_seconds(mut self, seconds: u64) -> Self {
        self.timeout_seconds = Some(seconds);
        self
    }

    /// Set the retry count (`max_retries`).
    #[must_use]
    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = Some(retries);
        self
    }

    /// Set the default output budget (`max_tokens`).
    #[must_use]
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set the adapter-wide context window (`context_window`).
    #[must_use]
    pub fn with_context_window(mut self, context_window: u32) -> Self {
        self.context_window = Some(context_window);
        self
    }

    /// Set the per-model overrides (`models`), replacing any already set.
    #[must_use]
    pub fn with_models(mut self, models: BTreeMap<String, ModelOverride>) -> Self {
        self.models = models;
        self
    }

    /// Set the provider-level default reasoning effort (`reasoning_effort`, Phase 51 D-00e).
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_llm::config::LlmProviderConfig;
    /// use paladin_ports::output::llm_port::{ReasoningEffort, ReasoningFallback};
    ///
    /// let block = LlmProviderConfig::new("")
    ///     .with_reasoning_effort(ReasoningEffort::High)
    ///     .with_reasoning_fallback(ReasoningFallback::NearestLower);
    /// assert_eq!(block.reasoning_effort, Some(ReasoningEffort::High));
    /// assert_eq!(block.reasoning_fallback, Some(ReasoningFallback::NearestLower));
    /// ```
    #[must_use]
    pub fn with_reasoning_effort(mut self, effort: ReasoningEffort) -> Self {
        self.reasoning_effort = Some(effort);
        self
    }

    /// Set the provider-level default reasoning fallback (`reasoning_fallback`, Phase 51 D-05).
    #[must_use]
    pub fn with_reasoning_fallback(mut self, fallback: ReasoningFallback) -> Self {
        self.reasoning_fallback = Some(fallback);
        self
    }

    /// Declare the wire a self-hosted endpoint reads a reasoning effort from
    /// (`reasoning_wire`, Phase 51 D-10).
    ///
    /// Honoured for `ollama` and `openai-compatible` only; any other provider refuses a block
    /// that sets it when the provider is built.
    #[must_use]
    pub fn with_reasoning_wire(mut self, wire: ReasoningWire) -> Self {
        self.reasoning_wire = Some(wire);
        self
    }
}

/// Configuration for all LLM providers.
///
/// One `Option<LlmProviderConfig>` field per registered provider (Claude's
/// Discretion, `17-CONTEXT.md`: six additional named fields rather than a
/// `HashMap<String, LlmProviderConfig>`, so an existing `config.yml` naming
/// only `openai`/`deepseek`/`anthropic` keeps deserializing byte-for-byte —
/// every field defaults to `None` when its key is absent, per serde's own
/// `Option<T>`-is-absent-tolerant behaviour, with no `#[serde(default)]`
/// needed except where `openai_compatible`'s hyphenated rename requires one).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Default provider to use if not specified.
    ///
    /// Accepts any name this build's provider registry knows
    /// ([`provider_names`]) — `"openai"`, `"deepseek"`, `"anthropic"`, and
    /// (when compiled in) `"kimi"`, `"qwen"`, `"grok"`, `"ollama"`,
    /// `"gemini"`, `"openai-compatible"`.
    pub default_provider: Option<String>,
    /// OpenAI configuration.
    pub openai: Option<LlmProviderConfig>,
    /// DeepSeek configuration.
    pub deepseek: Option<LlmProviderConfig>,
    /// Anthropic configuration.
    pub anthropic: Option<LlmProviderConfig>,
    /// Kimi (Moonshot AI) configuration.
    pub kimi: Option<LlmProviderConfig>,
    /// Qwen (DashScope) configuration.
    pub qwen: Option<LlmProviderConfig>,
    /// Grok (xAI) configuration.
    pub grok: Option<LlmProviderConfig>,
    /// Ollama (self-hosted, credential-free — D-12) configuration.
    pub ollama: Option<LlmProviderConfig>,
    /// Gemini configuration.
    pub gemini: Option<LlmProviderConfig>,
    /// Generic operator-configured OpenAI-compatible provider configuration
    /// (D-03). Accepts the hyphenated key `openai-compatible` (matching the
    /// provider's own registered name) and the snake_case alias
    /// `openai_compatible`, so both an operator typing the provider literal
    /// verbatim and a `#[derive(Deserialize)]` consumer keying off the Rust
    /// field name land on the same field.
    #[serde(default, rename = "openai-compatible", alias = "openai_compatible")]
    pub openai_compatible: Option<LlmProviderConfig>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            default_provider: Some("openai".to_string()),
            openai: None,
            deepseek: None,
            anthropic: None,
            kimi: None,
            qwen: None,
            grok: None,
            ollama: None,
            gemini: None,
            openai_compatible: None,
        }
    }
}

impl LlmConfig {
    /// Validate LLM configuration.
    ///
    /// Checks that `default_provider` names a provider this crate recognises. Its block need
    /// **not** be present: a provider configured only through its environment variables is built
    /// by `LlmProviderFactory::create_from_config` with no block (D-13). An empty
    /// `api_key` is likewise **not** an error: the key may come from the provider's environment
    /// variable (D-13), and construction reports one that is still missing.
    ///
    /// Returns `Err(String)` describing the first validation failure found.
    pub fn validate(&self) -> Result<(), String> {
        if let Some(default) = &self.default_provider {
            match self.get_provider_config(default) {
                Some(_) => {}
                None => {
                    // `get_provider_config` recognises all nine field names
                    // unconditionally (it is not gated on which paladin-llm
                    // adapter features happen to be compiled), so reaching
                    // this branch means either the name is one of the nine
                    // but its block is absent, or the name is not one of
                    // the nine at all. `provider_names()` (the D-10
                    // registry, compiled-features-aware) is consulted only
                    // to phrase the second case's message with the names
                    // this build actually registers — never to decide which
                    // branch we are in, since that decision must stay
                    // correct under every feature combination this crate
                    // builds with (including combinations that compile out
                    // some of the nine config fields' matching adapter).
                    if is_recognised_provider_field_name(default) {
                        // A recognised provider with no block is valid: every field then
                        // resolves from the provider's environment variables (D-13) through
                        // `LlmProviderFactory::create_from_config`, and `is_configured`
                        // reports a credential that is missing from both places.
                        return Ok(());
                    }
                    return Err(format!(
                        "Invalid default provider: {default}. Must be one of: {}",
                        provider_names().join(", ")
                    ));
                }
            }
        }

        // No block is rejected for an empty `api_key` (A1-50-09, D-13): the key may come from
        // the provider's own environment variable, so an empty one is a gap the layers fill
        // rather than a mistake. A key that is still missing at construction is reported then,
        // by `LlmProviderFactory::create_from_config`, naming the provider, the model and the
        // variable it tried -- and up front by `LlmProviderFactory::is_configured`. Ollama has
        // no key at all (D-12).

        Ok(())
    }

    /// Get the provider config for a specific provider name (case-insensitive).
    ///
    /// Recognises all nine field names this struct carries, regardless of
    /// which paladin-llm adapter features are compiled into this build —
    /// the config surface is decoupled from the (feature-gated) registry
    /// [`provider_names`] enumerates, matching PROV-03's requirement that
    /// existing config files keep loading unchanged.
    pub fn get_provider_config(&self, provider_name: &str) -> Option<&LlmProviderConfig> {
        match provider_name.to_lowercase().as_str() {
            "openai" => self.openai.as_ref(),
            "deepseek" => self.deepseek.as_ref(),
            "anthropic" => self.anthropic.as_ref(),
            "kimi" => self.kimi.as_ref(),
            "qwen" => self.qwen.as_ref(),
            "grok" => self.grok.as_ref(),
            "ollama" => self.ollama.as_ref(),
            "gemini" => self.gemini.as_ref(),
            "openai-compatible" | "openai_compatible" => self.openai_compatible.as_ref(),
            _ => None,
        }
    }

    /// Get the default provider name.
    pub fn get_default_provider_name(&self) -> Option<String> {
        self.default_provider.clone()
    }
}

/// Whether `name` (case-insensitive) is one of the nine provider field names
/// [`LlmConfig`] structurally recognises — independent of which paladin-llm
/// adapter features this build compiled in. Used only to phrase
/// [`LlmConfig::validate`]'s error message; never to decide whether a
/// present config block is honoured (that is `get_provider_config`'s job).
fn is_recognised_provider_field_name(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "openai"
            | "deepseek"
            | "anthropic"
            | "kimi"
            | "qwen"
            | "grok"
            | "ollama"
            | "gemini"
            | "openai-compatible"
            | "openai_compatible"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_config_default() {
        let config = LlmConfig::default();
        assert_eq!(config.default_provider, Some("openai".to_string()));
        assert!(config.openai.is_none());
        assert!(config.deepseek.is_none());
        assert!(config.anthropic.is_none());
    }

    /// D-13 (plan 50-10): a recognised default provider with no block is valid -- its fields
    /// resolve from the provider's environment variables, and a credential missing from both is
    /// reported by `is_configured` and `create_from_config`, which name the provider, the model
    /// and the variable. (Until Phase 50 this asserted the opposite.)
    #[test]
    fn test_llm_config_validate_default_provider_block_is_optional() {
        for name in ["openai", "deepseek", "anthropic", "openai_compatible"] {
            let config = LlmConfig {
                default_provider: Some(name.to_string()),
                openai: None,
                deepseek: None,
                anthropic: None,
                ..Default::default()
            };
            assert!(
                config.validate().is_ok(),
                "a default provider '{name}' with no block resolves from the environment"
            );
        }
    }

    #[test]
    fn test_llm_config_validate_invalid_provider_name() {
        let config = LlmConfig {
            default_provider: Some("invalid_provider".to_string()),
            openai: Some(LlmProviderConfig::new("key")),
            deepseek: None,
            anthropic: None,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    /// A1-50-09: a block may leave its key to the provider's environment variable (D-13), so
    /// an empty key is no longer a validation error; construction reports a missing one,
    /// naming the provider and the variable.
    #[test]
    fn validate_accepts_an_empty_key_filled_from_env() {
        let config = LlmConfig {
            default_provider: Some("openai".to_string()),
            openai: Some(LlmProviderConfig::new("")),
            deepseek: None,
            anthropic: None,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_llm_config_validate_success() {
        let config = LlmConfig {
            default_provider: Some("deepseek".to_string()),
            openai: None,
            deepseek: Some(
                LlmProviderConfig::new("test-key")
                    .with_base_url("https://api.deepseek.com/v1")
                    .with_default_model("deepseek-chat")
                    .with_default_temperature(0.7)
                    .with_timeout_seconds(300)
                    .with_max_retries(3),
            ),
            anthropic: None,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_llm_config_get_provider_config() {
        let openai_config = LlmProviderConfig::new("openai-key");

        let config = LlmConfig {
            default_provider: Some("openai".to_string()),
            openai: Some(openai_config),
            deepseek: None,
            anthropic: None,
            ..Default::default()
        };

        let retrieved = config.get_provider_config("openai");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().api_key, "openai-key");

        let not_found = config.get_provider_config("deepseek");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_llm_config_get_provider_config_case_insensitive() {
        let deepseek_config = LlmProviderConfig::new("deepseek-key");

        let config = LlmConfig {
            default_provider: Some("deepseek".to_string()),
            openai: None,
            deepseek: Some(deepseek_config),
            anthropic: None,
            ..Default::default()
        };

        assert!(config.get_provider_config("DeepSeek").is_some());
        assert!(config.get_provider_config("DEEPSEEK").is_some());
        assert!(config.get_provider_config("deepseek").is_some());
    }

    #[test]
    fn test_llm_config_get_default_provider_name() {
        let config = LlmConfig {
            default_provider: Some("anthropic".to_string()),
            openai: None,
            deepseek: None,
            anthropic: Some(LlmProviderConfig::new("key")),
            ..Default::default()
        };

        assert_eq!(
            config.get_default_provider_name(),
            Some("anthropic".to_string())
        );
    }

    // ── New behaviour, Task 3 (17-06): the nine-provider config surface ──

    /// PROV-03's own "done when": a config naming only the three providers
    /// that existed before this phase — no `kimi`/`qwen`/`grok`/`ollama`/
    /// `gemini`/`openai-compatible` key at all — must keep deserializing
    /// and validating exactly as before. Exercised via `serde_json` rather
    /// than YAML: this crate carries no YAML deserializer dependency, and
    /// the behaviour under test (serde's derived `Deserialize` impl
    /// treating a missing `Option<T>` field as `None`, per serde's own
    /// `missing_field` helper) is format-independent — a JSON object and a
    /// YAML mapping exercise the identical generated code path.
    #[test]
    fn test_llm_config_pre_phase_17_fixture_still_deserializes_and_validates() {
        let fixture = r#"{
            "default_provider": "openai",
            "openai": {
                "api_key": "test-key",
                "base_url": null,
                "default_model": null,
                "default_temperature": null,
                "timeout_seconds": null,
                "max_retries": null
            },
            "deepseek": {
                "api_key": "test-key",
                "base_url": null,
                "default_model": null,
                "default_temperature": null,
                "timeout_seconds": null,
                "max_retries": null
            },
            "anthropic": {
                "api_key": "test-key",
                "base_url": null,
                "default_model": null,
                "default_temperature": null,
                "timeout_seconds": null,
                "max_retries": null
            }
        }"#;

        let config: LlmConfig = serde_json::from_str(fixture).expect(
            "a pre-phase-17 config naming only openai/deepseek/anthropic must still deserialize",
        );

        assert!(config.kimi.is_none());
        assert!(config.qwen.is_none());
        assert!(config.grok.is_none());
        assert!(config.ollama.is_none());
        assert!(config.gemini.is_none());
        assert!(config.openai_compatible.is_none());
        assert!(config.validate().is_ok());
    }

    /// New behaviour: a config naming each of the six providers Phase 17
    /// added deserializes into its matching field, including the
    /// hyphenated `openai-compatible` key.
    #[test]
    fn test_llm_config_new_provider_names_deserialize_into_matching_fields() {
        let fixture = r#"{
            "default_provider": "kimi",
            "kimi": {"api_key": "kimi-key", "base_url": null, "default_model": null, "default_temperature": null, "timeout_seconds": null, "max_retries": null},
            "qwen": {"api_key": "qwen-key", "base_url": null, "default_model": null, "default_temperature": null, "timeout_seconds": null, "max_retries": null},
            "grok": {"api_key": "grok-key", "base_url": null, "default_model": null, "default_temperature": null, "timeout_seconds": null, "max_retries": null},
            "ollama": {"api_key": "", "base_url": null, "default_model": null, "default_temperature": null, "timeout_seconds": null, "max_retries": null},
            "gemini": {"api_key": "gemini-key", "base_url": null, "default_model": null, "default_temperature": null, "timeout_seconds": null, "max_retries": null},
            "openai-compatible": {"api_key": "compat-key", "base_url": "https://compat.example.com/v1", "default_model": "compat-model", "default_temperature": null, "timeout_seconds": null, "max_retries": null}
        }"#;

        let config: LlmConfig =
            serde_json::from_str(fixture).expect("the six-new-provider fixture must deserialize");

        assert_eq!(config.kimi.as_ref().unwrap().api_key, "kimi-key");
        assert_eq!(config.qwen.as_ref().unwrap().api_key, "qwen-key");
        assert_eq!(config.grok.as_ref().unwrap().api_key, "grok-key");
        assert!(config.ollama.is_some());
        assert_eq!(config.gemini.as_ref().unwrap().api_key, "gemini-key");
        assert_eq!(
            config.openai_compatible.as_ref().unwrap().api_key,
            "compat-key"
        );
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_llm_config_get_provider_config_openai_compatible_hyphen_and_underscore_case_insensitive()
     {
        let config = LlmConfig {
            openai_compatible: Some(
                LlmProviderConfig::new("compat-key")
                    .with_base_url("https://compat.example.com/v1")
                    .with_default_model("compat-model"),
            ),
            ..Default::default()
        };

        assert!(config.get_provider_config("OPENAI-COMPATIBLE").is_some());
        assert!(config.get_provider_config("openai_compatible").is_some());
        assert_eq!(
            config
                .get_provider_config("openai-compatible")
                .unwrap()
                .api_key,
            "compat-key"
        );
    }

    /// D-12: Ollama requires no credential, so an empty `api_key` on its
    /// block is not a validation error — the one deliberate exception to
    /// every other provider's non-empty-api_key rule.
    #[test]
    fn test_llm_config_validate_ollama_empty_api_key_is_allowed() {
        let config = LlmConfig {
            default_provider: Some("ollama".to_string()),
            ollama: Some(
                LlmProviderConfig::new("")
                    .with_base_url("http://localhost:11434/v1")
                    .with_default_model("llama3"),
            ),
            ..Default::default()
        };

        assert!(config.validate().is_ok());
    }

    /// The error message for a totally unrecognised `default_provider`
    /// lists names drawn from [`provider_names`] rather than a hardcoded
    /// three — gated on the six-new-preset feature combination (this
    /// plan's own `<verify>` command) so the "at least three" assertion is
    /// only exercised where this build's registry actually has that many
    /// rows; a default-features build (`openai` + `mock` only) has a
    /// one-row registry and would fail an unconditional version of this
    /// assertion for reasons that have nothing to do with the behaviour
    /// under test.
    #[cfg(all(
        feature = "kimi",
        feature = "qwen",
        feature = "grok",
        feature = "ollama",
        feature = "openai-compatible",
        feature = "gemini"
    ))]
    #[test]
    fn test_llm_config_validate_unknown_provider_name_lists_registered_names() {
        let config = LlmConfig {
            default_provider: Some("nosuchprovider".to_string()),
            ..Default::default()
        };

        let err = config
            .validate()
            .expect_err("an unrecognised default_provider must be rejected");
        let matched = [
            "kimi",
            "qwen",
            "grok",
            "ollama",
            "gemini",
            "openai-compatible",
        ]
        .iter()
        .filter(|name| err.contains(**name))
        .count();
        assert!(
            matched >= 3,
            "error message must contain at least three registered provider names: {err}"
        );
    }

    // ── Plan 50-09, Task 1: the PARITY-03 knobs, redaction and YAML compatibility ──

    /// Load the `llm` section of `yaml` the way the application does: through the `config`
    /// crate's YAML source, so key handling matches `Settings::new`.
    fn llm_from_yaml(yaml: &str) -> LlmConfig {
        config::Config::builder()
            .add_source(config::File::from_str(yaml, config::FileFormat::Yaml))
            .build()
            .expect("the YAML must parse")
            .get::<LlmConfig>("llm")
            .expect("the llm section must deserialize")
    }

    /// PROV-03: a v0.10.1-shaped file -- the six pre-Phase-50 keys per block and nothing else --
    /// loads to the same values as before, and the new fields are unset.
    #[test]
    fn existing_yaml_loads_unchanged() {
        let fixture = r#"
llm:
  default_provider: "openai"
  openai:
    api_key: "${OPENAI_API_KEY}"
    base_url: "https://api.openai.com/v1"
    default_model: "gpt-4"
    default_temperature: 0.7
    timeout_seconds: 300
    max_retries: 3
  kimi:
    api_key: "${MOONSHOT_API_KEY}"
    timeout_seconds: 60
  openai-compatible:
    api_key: "${OPENAI_COMPATIBLE_API_KEY}"
    base_url: "https://your-endpoint.example.com/v1"
    default_model: "your-model-name"
"#;
        let config = llm_from_yaml(fixture);
        let openai = config.openai.as_ref().expect("openai block");
        assert_eq!(openai.api_key, "${OPENAI_API_KEY}");
        assert_eq!(
            openai.base_url.as_deref(),
            Some("https://api.openai.com/v1")
        );
        assert_eq!(openai.default_model.as_deref(), Some("gpt-4"));
        assert_eq!(openai.default_temperature, Some(0.7));
        assert_eq!(openai.timeout_seconds, Some(300));
        assert_eq!(openai.max_retries, Some(3));
        let kimi = config.kimi.as_ref().expect("kimi block");
        assert_eq!(kimi.api_key, "${MOONSHOT_API_KEY}");
        assert_eq!(kimi.timeout_seconds, Some(60));
        assert_eq!(kimi.base_url, None);
        let compat = config.openai_compatible.as_ref().expect("compat block");
        assert_eq!(compat.default_model.as_deref(), Some("your-model-name"));
        for block in [openai, kimi, compat] {
            assert_eq!(block.max_tokens, None);
            assert_eq!(block.context_window, None);
            assert!(block.models.is_empty());
        }
        assert!(config.validate().is_ok());
    }

    /// The repository's own `config.example.yml` loads, including its keyless `ollama` block
    /// (which carried no `api_key` and used to fail the whole file).
    #[test]
    fn example_config_llm_section_loads_unchanged() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../config.example.yml");
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("config.example.yml must be readable at {path}: {e}"));
        let config = llm_from_yaml(&text);
        assert_eq!(config.default_provider.as_deref(), Some("openai"));
        let openai = config.openai.as_ref().expect("openai block");
        assert_eq!(openai.api_key, "${OPENAI_API_KEY}");
        assert_eq!(openai.default_model.as_deref(), Some("gpt-4o-mini"));
        assert_eq!(openai.timeout_seconds, Some(300));
        assert_eq!(openai.max_retries, Some(3));
        let anthropic = config.anthropic.as_ref().expect("anthropic block");
        assert_eq!(anthropic.default_model.as_deref(), Some("claude-haiku-5-5"));
        // Pitfall 7: claude-haiku-5-5 takes only temperature 1.0, so the example's block must
        // not carry a 0.7 that would refuse every agent using it.
        assert_eq!(anthropic.default_temperature, None);
        let kimi = config.kimi.as_ref().expect("kimi block");
        assert_eq!(kimi.default_model.as_deref(), Some("kimi-k2.6"));
        assert_eq!(kimi.timeout_seconds, Some(60));
        let ollama = config.ollama.as_ref().expect("ollama block");
        assert_eq!(ollama.api_key, "");
        assert_eq!(
            ollama.base_url.as_deref(),
            Some("http://localhost:11434/v1")
        );
        assert!(config.openai_compatible.is_some());
        assert!(config.validate().is_ok());
        for name in [
            "openai",
            "anthropic",
            "deepseek",
            "kimi",
            "qwen",
            "grok",
            "ollama",
            "gemini",
            "openai-compatible",
        ] {
            let block = config.get_provider_config(name).expect(name);
            assert_eq!(block.max_tokens, None, "{name}");
            assert_eq!(block.context_window, None, "{name}");
            assert!(block.models.is_empty(), "{name}");
        }
    }

    #[test]
    fn a_models_entry_deserializes_into_one_override() {
        let config = llm_from_yaml(
            r#"
llm:
  kimi:
    api_key: "k"
    max_tokens: 2048
    context_window: 500000
    models:
      kimi-k3:
        context_window: 900000
"#,
        );
        let kimi = config.kimi.expect("kimi block");
        assert_eq!(kimi.max_tokens, Some(2048));
        assert_eq!(kimi.context_window, Some(500_000));
        assert_eq!(kimi.models.len(), 1);
        let over = kimi.models.get("kimi-k3").expect("the kimi-k3 override");
        assert_eq!(over.context_window, Some(900_000));
        assert_eq!(over.max_output_tokens, None);
    }

    #[test]
    fn an_unknown_key_under_a_model_is_rejected_naming_it() {
        let error = config::Config::builder()
            .add_source(config::File::from_str(
                r#"
llm:
  kimi:
    api_key: "k"
    models:
      kimi-k3:
        context_windo: 900000
"#,
                config::FileFormat::Yaml,
            ))
            .build()
            .expect("the YAML must parse")
            .get::<LlmConfig>("llm")
            .expect_err("a misspelt override key must be rejected");
        assert!(
            error.to_string().contains("context_windo"),
            "the message must name the unknown key: {error}"
        );
    }

    #[test]
    fn a_block_with_no_api_key_deserializes_with_an_empty_key() {
        let block: LlmProviderConfig = serde_json::from_str(r#"{"base_url": "http://x"}"#)
            .expect("a block with no api_key must deserialize");
        assert_eq!(block.api_key, "");
        assert_eq!(block.base_url.as_deref(), Some("http://x"));
    }

    #[test]
    fn debug_never_prints_the_key() {
        let block = LlmProviderConfig::new("sk-super-secret-value").with_default_model("m");
        let rendered = format!("{block:?}");
        assert!(rendered.contains("<redacted>"), "{rendered}");
        assert!(!rendered.contains("sk-super-secret-value"), "{rendered}");
        assert!(rendered.contains("default_model"), "{rendered}");
        // The key never reaches Debug through the enclosing config either.
        let config = LlmConfig {
            openai: Some(block),
            ..Default::default()
        };
        assert!(!format!("{config:?}").contains("sk-super-secret-value"));
        // An empty key prints empty, not as a redaction of something that is not there.
        let keyless = format!("{:?}", LlmProviderConfig::new(""));
        assert!(!keyless.contains("<redacted>"), "{keyless}");
    }

    #[test]
    fn the_api_key_is_never_serialized() {
        // WR-04: the type must not leak a key by construction, however it is serialised.
        const SECRET: &str = "sk-never-serialize-me";
        let block = LlmProviderConfig::new(SECRET);
        let json = serde_json::to_string(&block).expect("serialize");
        assert!(!json.contains(SECRET), "{json}");
        assert!(!json.contains("api_key"), "{json}");

        let config = LlmConfig {
            openai: Some(LlmProviderConfig::new(SECRET)),
            anthropic: Some(LlmProviderConfig::new(SECRET)),
            ..LlmConfig::default()
        };
        let json = serde_json::to_string(&config).expect("serialize");
        assert!(!json.contains(SECRET), "{json}");

        // A round trip loses the key rather than inventing one: the provider's environment
        // variable supplies it, exactly as for an absent `api_key`.
        let back: LlmConfig = serde_json::from_str(&json).expect("round trip");
        assert_eq!(back.openai.expect("openai block").api_key, "");
    }

    #[test]
    fn empty_new_fields_are_not_serialized() {
        let value = serde_json::to_value(LlmProviderConfig::new("k")).expect("serialize");
        let object = value.as_object().expect("an object");
        for key in ["max_tokens", "context_window", "models"] {
            assert!(
                !object.contains_key(key),
                "{key} must be skipped when empty"
            );
        }
        let set = LlmProviderConfig::new("k")
            .with_max_tokens(1)
            .with_context_window(2)
            .with_models(BTreeMap::from([(
                "m".to_string(),
                ModelOverride::default(),
            )]));
        let value = serde_json::to_value(set).expect("serialize");
        for key in ["max_tokens", "context_window", "models"] {
            assert!(value.get(key).is_some(), "{key} must be written when set");
        }
    }

    #[test]
    fn builders_set_exactly_their_field() {
        let block = LlmProviderConfig::new("k")
            .with_base_url("http://x")
            .with_default_model("m")
            .with_default_temperature(0.5)
            .with_timeout_seconds(7)
            .with_max_retries(2)
            .with_max_tokens(100)
            .with_context_window(200);
        assert_eq!(block.api_key, "k");
        assert_eq!(block.base_url.as_deref(), Some("http://x"));
        assert_eq!(block.default_model.as_deref(), Some("m"));
        assert_eq!(block.default_temperature, Some(0.5));
        assert_eq!(block.timeout_seconds, Some(7));
        assert_eq!(block.max_retries, Some(2));
        assert_eq!(block.max_tokens, Some(100));
        assert_eq!(block.context_window, Some(200));
        assert!(block.models.is_empty());
    }

    // ── Phase 51: the provider-level reasoning keys (D-00e, D-05, D-10) ──────────────────────

    #[test]
    fn reasoning_keys_deserialise_in_every_spelling() {
        let level: LlmProviderConfig =
            serde_json::from_str(r#"{"reasoning_effort": "high"}"#).expect("a level");
        assert_eq!(level.reasoning_effort, Some(ReasoningEffort::High));
        let budget: LlmProviderConfig =
            serde_json::from_str(r#"{"reasoning_effort": {"budget": 2048}}"#).expect("a budget");
        assert_eq!(budget.reasoning_effort, Some(ReasoningEffort::Budget(2048)));
        let full: LlmProviderConfig = serde_json::from_str(
            r#"{"reasoning_effort": "xhigh", "reasoning_fallback": "nearest_lower",
                "reasoning_wire": "top_level_effort"}"#,
        )
        .expect("all three keys");
        assert_eq!(full.reasoning_effort, Some(ReasoningEffort::XHigh));
        assert_eq!(
            full.reasoning_fallback,
            Some(ReasoningFallback::NearestLower)
        );
        assert_eq!(full.reasoning_wire, Some(ReasoningWire::TopLevelEffort));
    }

    #[test]
    fn reasoning_keys_reach_a_block_through_the_yaml_loader() {
        let yaml = "api_key: ''\nreasoning_effort: high\nreasoning_fallback: nearest_higher\n";
        let block: LlmProviderConfig = config::Config::builder()
            .add_source(config::File::from_str(yaml, config::FileFormat::Yaml))
            .build()
            .expect("yaml loads")
            .try_deserialize()
            .expect("the block deserialises");
        assert_eq!(block.reasoning_effort, Some(ReasoningEffort::High));
        assert_eq!(
            block.reasoning_fallback,
            Some(ReasoningFallback::NearestHigher)
        );
        assert_eq!(block.reasoning_wire, None);
    }

    #[test]
    fn a_block_setting_no_reasoning_key_serialises_none_of_them() {
        let value = serde_json::to_value(LlmProviderConfig::new("k")).expect("serialize");
        for key in ["reasoning_effort", "reasoning_fallback", "reasoning_wire"] {
            assert!(value.get(key).is_none(), "{key} must be omitted when unset");
        }
        let set = LlmProviderConfig::new("k")
            .with_reasoning_effort(ReasoningEffort::Medium)
            .with_reasoning_fallback(ReasoningFallback::Refuse)
            .with_reasoning_wire(ReasoningWire::EnableThinking);
        let value = serde_json::to_value(set).expect("serialize");
        assert_eq!(value["reasoning_effort"], "medium");
        assert_eq!(value["reasoning_fallback"], "refuse");
        assert_eq!(value["reasoning_wire"], "enable_thinking");
    }

    #[test]
    fn an_unknown_reasoning_value_is_refused_at_load() {
        for bad in [
            r#"{"reasoning_effort": "extreme"}"#,
            r#"{"reasoning_fallback": "nearest"}"#,
            r#"{"reasoning_wire": "custom"}"#,
        ] {
            assert!(
                serde_json::from_str::<LlmProviderConfig>(bad).is_err(),
                "{bad} must not load"
            );
        }
    }

    #[test]
    fn debug_names_the_reasoning_keys_and_still_redacts_the_key() {
        let block = LlmProviderConfig::new("secret-key-value")
            .with_reasoning_effort(ReasoningEffort::Low)
            .with_reasoning_wire(ReasoningWire::ThinkingObject);
        let debug = format!("{block:?}");
        assert!(debug.contains("reasoning_effort: Some(Low)"), "{debug}");
        assert!(debug.contains("reasoning_fallback"), "{debug}");
        assert!(debug.contains("reasoning_wire"), "{debug}");
        assert!(!debug.contains("secret-key-value"), "{debug}");
    }
}
