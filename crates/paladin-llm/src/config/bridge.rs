//! Conversion bridges between [`LlmProviderConfig`]/[`VisionConfig`]
//! (application config types with serde) and the runtime adapter config types
//! within the same crate.
//!
//! Placing the impls here keeps all `paladin-llm` types together and avoids
//! the Rust orphan rule restriction (both source and target types are local to
//! this crate).

#[cfg(any(
    feature = "openai",
    feature = "anthropic",
    feature = "deepseek",
    feature = "kimi",
    feature = "qwen",
    feature = "grok",
    feature = "ollama",
    feature = "gemini",
    feature = "openai-compatible"
))]
use crate::config::llm::LlmProviderConfig;

#[cfg(feature = "openai")]
use crate::openai::adapter::OpenAIConfig;

#[cfg(feature = "anthropic")]
use crate::anthropic::adapter::AnthropicConfig;

#[cfg(feature = "deepseek")]
use crate::deepseek::adapter::DeepSeekConfig;

#[cfg(feature = "kimi")]
use crate::kimi::adapter::KimiConfig;

#[cfg(feature = "qwen")]
use crate::qwen::adapter::QwenConfig;

#[cfg(feature = "grok")]
use crate::grok::adapter::GrokConfig;

#[cfg(feature = "ollama")]
use crate::ollama::adapter::OllamaConfig;

#[cfg(feature = "gemini")]
use crate::gemini::adapter::GeminiConfig;

#[cfg(feature = "openai-compatible")]
use crate::openai_compatible::adapter::{
    OpenAiCompatibleCapabilitiesConfig, OpenAiCompatibleConfig,
};

#[cfg(any(
    feature = "openai",
    feature = "anthropic",
    feature = "deepseek",
    feature = "kimi",
    feature = "qwen",
    feature = "grok",
    feature = "ollama",
    feature = "gemini",
    feature = "openai-compatible"
))]
use std::collections::BTreeMap;

#[cfg(feature = "vision")]
use crate::config::vision::VisionConfig as ConfigVisionConfig;
#[cfg(feature = "vision")]
use crate::openai::vision::{
    VisionConfig as AdapterVisionConfig, VisionProviderConfig as AdapterVisionProviderConfig,
    VisionRetryConfig as AdapterVisionRetryConfig,
};

// ── OpenAI ───────────────────────────────────────────────────────────────────

#[cfg(feature = "openai")]
impl From<&LlmProviderConfig> for OpenAIConfig {
    /// Convert an [`LlmProviderConfig`] into an [`OpenAIConfig`].
    ///
    /// `organization` is always `None`; set it via env var
    /// (`OPENAI_ORGANIZATION`) or construct `OpenAIConfig` directly.
    fn from(cfg: &LlmProviderConfig) -> Self {
        Self {
            api_key: cfg.api_key.clone(),
            base_url: cfg
                .base_url
                .clone()
                .unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
            organization: None,
            timeout_seconds: cfg.timeout_seconds.unwrap_or(300),
            max_retries: cfg.max_retries.unwrap_or(3),
            max_tokens: Some(4096),
            context_window: None,
            model_overrides: BTreeMap::new(),
        }
    }
}

// ── Anthropic ────────────────────────────────────────────────────────────────

#[cfg(feature = "anthropic")]
impl From<&LlmProviderConfig> for AnthropicConfig {
    /// Convert an [`LlmProviderConfig`] into an [`AnthropicConfig`].
    fn from(cfg: &LlmProviderConfig) -> Self {
        Self {
            api_key: cfg.api_key.clone(),
            base_url: cfg
                .base_url
                .clone()
                .unwrap_or_else(|| "https://api.anthropic.com/v1".to_string()),
            model: cfg
                .default_model
                .clone()
                .unwrap_or_else(|| "claude-3-5-sonnet-20241022".to_string()),
            max_tokens: 4096,
            timeout_seconds: cfg.timeout_seconds.unwrap_or(300),
            max_retries: cfg
                .max_retries
                .unwrap_or(crate::registry::DEFAULT_MAX_RETRIES),
            context_window: None,
            model_overrides: BTreeMap::new(),
        }
    }
}

// ── DeepSeek ─────────────────────────────────────────────────────────────────

#[cfg(feature = "deepseek")]
impl From<&LlmProviderConfig> for DeepSeekConfig {
    /// Convert an [`LlmProviderConfig`] into a [`DeepSeekConfig`].
    fn from(cfg: &LlmProviderConfig) -> Self {
        Self {
            api_key: cfg.api_key.clone(),
            base_url: cfg
                .base_url
                .clone()
                .unwrap_or_else(|| "https://api.deepseek.com/v1".to_string()),
            model: cfg
                .default_model
                .clone()
                .unwrap_or_else(|| "deepseek-chat".to_string()),
            timeout_seconds: cfg.timeout_seconds.unwrap_or(60),
            max_tokens: None,
            max_retries: cfg
                .max_retries
                .unwrap_or(crate::registry::DEFAULT_MAX_RETRIES),
            context_window: None,
            model_overrides: BTreeMap::new(),
        }
    }
}

// ── Kimi (Moonshot AI) ───────────────────────────────────────────────────────

#[cfg(feature = "kimi")]
impl From<&LlmProviderConfig> for KimiConfig {
    /// Convert an [`LlmProviderConfig`] into a [`KimiConfig`], falling back
    /// to Kimi's own defaults for any absent optional field.
    fn from(cfg: &LlmProviderConfig) -> Self {
        Self {
            api_key: cfg.api_key.clone(),
            base_url: cfg
                .base_url
                .clone()
                .unwrap_or_else(|| crate::kimi::adapter::KIMI_DEFAULT_BASE_URL.to_string()),
            model: cfg
                .default_model
                .clone()
                .unwrap_or_else(|| crate::kimi::adapter::KIMI_DEFAULT_MODEL.to_string()),
            timeout_seconds: cfg.timeout_seconds.unwrap_or(60),
            max_tokens: None,
            max_retries: cfg
                .max_retries
                .unwrap_or(crate::registry::DEFAULT_MAX_RETRIES),
            context_window: None,
            model_overrides: BTreeMap::new(),
        }
    }
}

// ── Qwen (DashScope) ─────────────────────────────────────────────────────────

#[cfg(feature = "qwen")]
impl From<&LlmProviderConfig> for QwenConfig {
    /// Convert an [`LlmProviderConfig`] into a [`QwenConfig`], falling back
    /// to Qwen's own defaults for any absent optional field.
    fn from(cfg: &LlmProviderConfig) -> Self {
        Self {
            api_key: cfg.api_key.clone(),
            base_url: cfg
                .base_url
                .clone()
                .unwrap_or_else(|| crate::qwen::adapter::QWEN_DEFAULT_BASE_URL.to_string()),
            model: cfg
                .default_model
                .clone()
                .unwrap_or_else(|| crate::qwen::adapter::QWEN_DEFAULT_MODEL.to_string()),
            timeout_seconds: cfg.timeout_seconds.unwrap_or(60),
            max_tokens: None,
            max_retries: cfg
                .max_retries
                .unwrap_or(crate::registry::DEFAULT_MAX_RETRIES),
            context_window: None,
            model_overrides: BTreeMap::new(),
        }
    }
}

// ── Grok (xAI) ───────────────────────────────────────────────────────────────

#[cfg(feature = "grok")]
impl From<&LlmProviderConfig> for GrokConfig {
    /// Convert an [`LlmProviderConfig`] into a [`GrokConfig`], falling back
    /// to Grok's own defaults for any absent optional field.
    fn from(cfg: &LlmProviderConfig) -> Self {
        Self {
            api_key: cfg.api_key.clone(),
            base_url: cfg
                .base_url
                .clone()
                .unwrap_or_else(|| crate::grok::adapter::GROK_DEFAULT_BASE_URL.to_string()),
            model: cfg
                .default_model
                .clone()
                .unwrap_or_else(|| crate::grok::adapter::GROK_DEFAULT_MODEL.to_string()),
            timeout_seconds: cfg.timeout_seconds.unwrap_or(60),
            max_tokens: None,
            max_retries: cfg
                .max_retries
                .unwrap_or(crate::registry::DEFAULT_MAX_RETRIES),
            context_window: None,
            model_overrides: BTreeMap::new(),
        }
    }
}

// ── Gemini ───────────────────────────────────────────────────────────────────

#[cfg(feature = "gemini")]
impl From<&LlmProviderConfig> for GeminiConfig {
    /// Convert an [`LlmProviderConfig`] into a [`GeminiConfig`], falling
    /// back to Gemini's own defaults for any absent optional field.
    fn from(cfg: &LlmProviderConfig) -> Self {
        Self {
            api_key: cfg.api_key.clone(),
            base_url: cfg
                .base_url
                .clone()
                .unwrap_or_else(|| crate::gemini::adapter::GEMINI_DEFAULT_BASE_URL.to_string()),
            model: cfg
                .default_model
                .clone()
                .unwrap_or_else(|| crate::gemini::adapter::GEMINI_DEFAULT_MODEL.to_string()),
            timeout_seconds: cfg.timeout_seconds.unwrap_or(60),
            max_tokens: None,
            max_retries: cfg
                .max_retries
                .unwrap_or(crate::registry::DEFAULT_MAX_RETRIES),
            context_window: None,
            model_overrides: BTreeMap::new(),
        }
    }
}

// ── Ollama ───────────────────────────────────────────────────────────────────

#[cfg(feature = "ollama")]
impl From<&LlmProviderConfig> for OllamaConfig {
    /// Convert an [`LlmProviderConfig`] into an [`OllamaConfig`], falling
    /// back to Ollama's own defaults for any absent optional field.
    ///
    /// `cfg.api_key` is deliberately **ignored** — [`OllamaConfig`] has no
    /// credential field at all (D-12: Ollama requires none). Threading an
    /// empty string through would imply a credential concept this provider
    /// does not have; the operator-facing `LlmProviderConfig.api_key` field
    /// stays required by its own type, but its value is simply never read
    /// here.
    fn from(cfg: &LlmProviderConfig) -> Self {
        Self {
            base_url: cfg
                .base_url
                .clone()
                .unwrap_or_else(|| crate::ollama::adapter::OLLAMA_DEFAULT_BASE_URL.to_string()),
            model: cfg
                .default_model
                .clone()
                .unwrap_or_else(|| crate::ollama::adapter::OLLAMA_DEFAULT_MODEL.to_string()),
            timeout_seconds: cfg.timeout_seconds.unwrap_or(120),
            max_tokens: None,
            max_retries: cfg
                .max_retries
                .unwrap_or(crate::registry::DEFAULT_MAX_RETRIES),
            context_window: None,
            model_overrides: BTreeMap::new(),
        }
    }
}

// ── Generic operator-configured OpenAI-compatible provider (D-03) ──────────────

#[cfg(feature = "openai-compatible")]
impl TryFrom<&LlmProviderConfig> for OpenAiCompatibleConfig {
    type Error = String;

    /// Convert an [`LlmProviderConfig`] into an [`OpenAiCompatibleConfig`].
    ///
    /// Unlike every other provider's `From` impl in this file, this is a
    /// **`TryFrom`** — deliberately, not for symmetry with the rest. Every
    /// other provider has a vendor to inherit `base_url`/`model` defaults
    /// from; the generic provider does not (D-03), so a config block
    /// missing either is a genuine configuration error rather than a value
    /// this conversion could paper over with an invented placeholder.
    /// `capabilities` has no `LlmProviderConfig` counterpart at all — this
    /// bridge cannot express an operator's capability declaration, so it
    /// falls back to [`OpenAiCompatibleCapabilitiesConfig`]'s own
    /// conservative defaults (D-04: unset means the conservative answer).
    /// A caller needing capability control must construct
    /// [`OpenAiCompatibleConfig`] directly or go through
    /// [`OpenAiCompatibleConfig::from_env`].
    fn try_from(cfg: &LlmProviderConfig) -> Result<Self, Self::Error> {
        let base_url = cfg
            .base_url
            .clone()
            .ok_or_else(|| "openai-compatible config requires base_url to be set".to_string())?;
        let model = cfg.default_model.clone().ok_or_else(|| {
            "openai-compatible config requires default_model to be set".to_string()
        })?;

        Ok(Self {
            api_key: cfg.api_key.clone(),
            base_url,
            model,
            timeout_seconds: cfg.timeout_seconds.unwrap_or(60),
            capabilities: OpenAiCompatibleCapabilitiesConfig {
                supports_streaming: true,
                supports_tool_calling: false,
                supports_function_calling: false,
                supports_vision: false,
                supports_embeddings: false,
                supports_system_messages: false,
                max_context_tokens: None,
                temperature_range: None,
            },
            max_tokens: None,
            max_retries: cfg
                .max_retries
                .unwrap_or(crate::registry::DEFAULT_MAX_RETRIES),
            context_window: None,
            model_overrides: BTreeMap::new(),
        })
    }
}

// ── Vision config ────────────────────────────────────────────────────────────

#[cfg(feature = "vision")]
impl From<&ConfigVisionConfig> for AdapterVisionConfig {
    /// Convert the application [`VisionConfig`](ConfigVisionConfig) (with serde)
    /// into the runtime [`VisionConfig`](AdapterVisionConfig) used by adapters.
    fn from(cfg: &ConfigVisionConfig) -> Self {
        Self {
            retry: AdapterVisionRetryConfig {
                max_retries: cfg.retry.max_retries,
                initial_backoff_ms: cfg.retry.initial_backoff_ms,
                backoff_multiplier: cfg.retry.backoff_multiplier,
            },
            openai: AdapterVisionProviderConfig {
                max_tokens: cfg.openai.max_tokens,
            },
            anthropic: AdapterVisionProviderConfig {
                max_tokens: cfg.anthropic.max_tokens,
            },
        }
    }
}
