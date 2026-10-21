//! Conversion bridges between [`LlmProviderConfig`]/[`VisionConfig`]
//! (application config types with serde) and the runtime adapter config types
//! within the same crate.
//!
//! Placing the impls here keeps all `paladin-llm` types together and avoids
//! the Rust orphan rule restriction (both source and target types are local to
//! this crate).
//!
//! # Per-provider `from_layers`
//!
//! Each adapter has a `pub(crate) fn <provider>_from_layers(block, model, env)` that builds its
//! `*Config` through `config::layers::resolve_layers`: the per-agent
//! model, then the `llm.<provider>` block, then the provider's legacy environment variables
//! (exactly the ones its `from_env` reads), then the adapter's compiled default, field by field
//! (D-13; see `config::layers`). `LlmProviderFactory::create_from_config` calls these, so they
//! are the production path from a configuration block to a constructed adapter.
//!
//! The `From<&LlmProviderConfig>` / `TryFrom` conversions stay for callers that build a config
//! from one block with no environment. They run the same layering with an empty environment, so
//! their defaults can no longer drift from the adapter's own: an unresolved key or base URL
//! becomes empty and the adapter refuses it at construction, and a `${NAME}` placeholder is never
//! carried through as the credential.

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
use crate::config::layers::{CompiledDefaults, ProviderEnv, resolve_layers};
// Every provider but the generic one builds its config through `Layered` and also has an
// infallible `From`, which resolves leniently.
#[cfg(any(
    feature = "openai",
    feature = "anthropic",
    feature = "deepseek",
    feature = "kimi",
    feature = "qwen",
    feature = "grok",
    feature = "ollama",
    feature = "gemini"
))]
use crate::config::layers::{Layered, resolve_layers_lenient};
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
use crate::registry::DEFAULT_MAX_RETRIES;

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

#[cfg(feature = "vision")]
use crate::config::vision::VisionConfig as ConfigVisionConfig;
#[cfg(feature = "vision")]
use crate::openai::vision::{
    VisionConfig as AdapterVisionConfig, VisionProviderConfig as AdapterVisionProviderConfig,
    VisionRetryConfig as AdapterVisionRetryConfig,
};

/// A map-free environment lookup, as `from_layers` takes it.
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
type Env<'a> = &'a dyn Fn(&str) -> Option<String>;

// ── OpenAI ───────────────────────────────────────────────────────────────────

#[cfg(feature = "openai")]
const OPENAI_ENV: ProviderEnv = ProviderEnv {
    provider: "openai",
    key: Some("OPENAI_API_KEY"),
    base_url: Some("OPENAI_BASE_URL"),
    // `OpenAIConfig.model` is the adapter's default model, but no variable names it: the
    // layered value is the agent's or block's model, else `OPENAI_DEFAULT_MODEL` (D-06). The
    // layering also shape-checks it, names it in errors and picks the timeout default (A3).
    model: None,
    timeout: Some("OPENAI_TIMEOUT_SECONDS"),
    retries: Some("OPENAI_MAX_RETRIES"),
    max_tokens: None,
};

#[cfg(feature = "openai")]
const OPENAI_DEFAULTS: CompiledDefaults = CompiledDefaults {
    base_url: Some(crate::openai::adapter::OPENAI_DEFAULT_BASE_URL),
    model: Some(crate::openai::adapter::OPENAI_DEFAULT_MODEL),
    model_required: false,
    timeout_seconds: 300,
    max_retries: DEFAULT_MAX_RETRIES,
    max_tokens: Some(crate::openai::adapter::OPENAI_DEFAULT_MAX_TOKENS),
};

#[cfg(feature = "openai")]
fn openai_config(layered: Layered, organization: Option<String>) -> OpenAIConfig {
    OpenAIConfig {
        api_key: layered.api_key,
        base_url: layered.base_url,
        model: layered.model,
        organization,
        timeout_seconds: layered.timeout_seconds,
        max_retries: layered.max_retries,
        max_tokens: layered.max_tokens,
        context_window: layered.context_window,
        model_overrides: layered.model_overrides,
    }
}

/// Build an [`OpenAIConfig`] from the layers (D-13).
///
/// `model` (the agent's model) becomes [`OpenAIConfig::model`], the adapter's default model;
/// without one the block's `default_model` is used, else
/// [`OPENAI_DEFAULT_MODEL`](crate::openai::adapter::OPENAI_DEFAULT_MODEL). It also selects the
/// profile the timeout default is decided from (A3-50-09); the 300 s default is OpenAI's
/// compiled timeout too, so a reasoning model changes nothing there, and the field changes no
/// adapter-wide limit. `OPENAI_ORGANIZATION` fills `organization`; the block has no such key.
///
/// # Errors
///
/// A message naming the provider, the model and the variable tried; see
/// [`resolve_layers`](crate::config::layers::resolve_layers).
#[cfg(feature = "openai")]
pub(crate) fn openai_from_layers(
    block: Option<&LlmProviderConfig>,
    model: Option<&str>,
    env: Env<'_>,
) -> Result<OpenAIConfig, String> {
    let layered = resolve_layers(block, model, &OPENAI_ENV, &OPENAI_DEFAULTS, &|_| false, env)?;
    let organization = env("OPENAI_ORGANIZATION")
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    Ok(openai_config(layered, organization))
}

#[cfg(feature = "openai")]
impl From<&LlmProviderConfig> for OpenAIConfig {
    /// Convert an [`LlmProviderConfig`] into an [`OpenAIConfig`].
    ///
    /// `organization` is always `None`; set it via env var
    /// (`OPENAI_ORGANIZATION`, read by [`OpenAIConfig::from_env`] and by
    /// `LlmProviderFactory::create_from_config`) or construct `OpenAIConfig` directly.
    fn from(cfg: &LlmProviderConfig) -> Self {
        let layered = resolve_layers_lenient(cfg, &OPENAI_ENV, &OPENAI_DEFAULTS, &|_| false);
        openai_config(layered, None)
    }
}

// ── Anthropic ────────────────────────────────────────────────────────────────

#[cfg(feature = "anthropic")]
const ANTHROPIC_ENV: ProviderEnv = ProviderEnv {
    provider: "anthropic",
    key: Some("ANTHROPIC_API_KEY"),
    base_url: Some("ANTHROPIC_BASE_URL"),
    model: Some("ANTHROPIC_MODEL"),
    timeout: Some("ANTHROPIC_TIMEOUT_SECONDS"),
    retries: None,
    max_tokens: Some("ANTHROPIC_MAX_TOKENS"),
};

#[cfg(feature = "anthropic")]
const ANTHROPIC_DEFAULTS: CompiledDefaults = CompiledDefaults {
    base_url: Some(crate::anthropic::adapter::ANTHROPIC_DEFAULT_BASE_URL),
    model: Some(crate::anthropic::adapter::ANTHROPIC_DEFAULT_MODEL),
    model_required: false,
    timeout_seconds: 60,
    max_retries: DEFAULT_MAX_RETRIES,
    max_tokens: Some(crate::anthropic::adapter::ANTHROPIC_DEFAULT_MAX_TOKENS),
};

#[cfg(feature = "anthropic")]
fn anthropic_config(layered: Layered) -> AnthropicConfig {
    AnthropicConfig {
        api_key: layered.api_key,
        base_url: layered.base_url,
        model: layered.model,
        // The Claude API requires the field; the compiled default always supplies one.
        max_tokens: layered
            .max_tokens
            .unwrap_or(crate::anthropic::adapter::ANTHROPIC_DEFAULT_MAX_TOKENS),
        timeout_seconds: layered.timeout_seconds,
        max_retries: layered.max_retries,
        context_window: layered.context_window,
        model_overrides: layered.model_overrides,
    }
}

/// Whether `model` is a reasoning model on `provider`, counting the block's own overrides.
#[cfg(any(
    feature = "anthropic",
    feature = "deepseek",
    feature = "kimi",
    feature = "qwen",
    feature = "grok",
    feature = "ollama",
    feature = "gemini",
    feature = "openai-compatible"
))]
fn reasoning_with_overrides(
    provider: &'static str,
    block: Option<&LlmProviderConfig>,
) -> impl Fn(&str) -> bool {
    let overrides = block.map(|b| b.models.clone()).unwrap_or_default();
    move |model| crate::registry::is_reasoning_model(provider, model, &overrides)
}

/// Build an [`AnthropicConfig`] from the layers (D-13).
///
/// # Errors
///
/// A message naming the provider, the model and the variable tried; see
/// [`resolve_layers`](crate::config::layers::resolve_layers).
#[cfg(feature = "anthropic")]
pub(crate) fn anthropic_from_layers(
    block: Option<&LlmProviderConfig>,
    model: Option<&str>,
    env: Env<'_>,
) -> Result<AnthropicConfig, String> {
    let is_reasoning = reasoning_with_overrides("anthropic", block);
    resolve_layers(
        block,
        model,
        &ANTHROPIC_ENV,
        &ANTHROPIC_DEFAULTS,
        &is_reasoning,
        env,
    )
    .map(anthropic_config)
}

#[cfg(feature = "anthropic")]
impl From<&LlmProviderConfig> for AnthropicConfig {
    /// Convert an [`LlmProviderConfig`] into an [`AnthropicConfig`].
    ///
    /// Absent fields take the adapter's own compiled defaults (the current
    /// [`ANTHROPIC_DEFAULT_MODEL`](crate::anthropic::adapter::ANTHROPIC_DEFAULT_MODEL), a 60 s
    /// timeout), which replace the stale October-2024 model id and 300 s this conversion used to
    /// carry.
    fn from(cfg: &LlmProviderConfig) -> Self {
        let is_reasoning = reasoning_with_overrides("anthropic", Some(cfg));
        anthropic_config(resolve_layers_lenient(
            cfg,
            &ANTHROPIC_ENV,
            &ANTHROPIC_DEFAULTS,
            &is_reasoning,
        ))
    }
}

// ── DeepSeek ─────────────────────────────────────────────────────────────────

#[cfg(feature = "deepseek")]
const DEEPSEEK_ENV: ProviderEnv = ProviderEnv {
    provider: "deepseek",
    key: Some("DEEPSEEK_API_KEY"),
    base_url: Some("DEEPSEEK_BASE_URL"),
    model: Some("DEEPSEEK_MODEL"),
    timeout: Some("DEEPSEEK_TIMEOUT_SECONDS"),
    retries: None,
    max_tokens: None,
};

#[cfg(feature = "deepseek")]
const DEEPSEEK_DEFAULTS: CompiledDefaults = CompiledDefaults {
    base_url: Some(crate::deepseek::adapter::DEEPSEEK_DEFAULT_BASE_URL),
    model: Some(crate::deepseek::adapter::DEEPSEEK_DEFAULT_MODEL),
    model_required: false,
    timeout_seconds: 60,
    max_retries: DEFAULT_MAX_RETRIES,
    max_tokens: None,
};

#[cfg(feature = "deepseek")]
fn deepseek_config(layered: Layered) -> DeepSeekConfig {
    DeepSeekConfig {
        api_key: layered.api_key,
        base_url: layered.base_url,
        model: layered.model,
        timeout_seconds: layered.timeout_seconds,
        max_tokens: layered.max_tokens,
        max_retries: layered.max_retries,
        context_window: layered.context_window,
        model_overrides: layered.model_overrides,
    }
}

/// Build a [`DeepSeekConfig`] from the layers (D-13).
///
/// # Errors
///
/// A message naming the provider, the model and the variable tried; see
/// [`resolve_layers`](crate::config::layers::resolve_layers).
#[cfg(feature = "deepseek")]
pub(crate) fn deepseek_from_layers(
    block: Option<&LlmProviderConfig>,
    model: Option<&str>,
    env: Env<'_>,
) -> Result<DeepSeekConfig, String> {
    let is_reasoning = reasoning_with_overrides("deepseek", block);
    resolve_layers(
        block,
        model,
        &DEEPSEEK_ENV,
        &DEEPSEEK_DEFAULTS,
        &is_reasoning,
        env,
    )
    .map(deepseek_config)
}

#[cfg(feature = "deepseek")]
impl From<&LlmProviderConfig> for DeepSeekConfig {
    /// Convert an [`LlmProviderConfig`] into a [`DeepSeekConfig`].
    ///
    /// Absent fields take the adapter's own compiled defaults (the current
    /// [`DEEPSEEK_DEFAULT_MODEL`](crate::deepseek::adapter::DEEPSEEK_DEFAULT_MODEL), 300 s for a
    /// reasoning model), which replace the retired `deepseek-chat` id this conversion used to
    /// carry.
    fn from(cfg: &LlmProviderConfig) -> Self {
        let is_reasoning = reasoning_with_overrides("deepseek", Some(cfg));
        deepseek_config(resolve_layers_lenient(
            cfg,
            &DEEPSEEK_ENV,
            &DEEPSEEK_DEFAULTS,
            &is_reasoning,
        ))
    }
}

// ── Kimi (Moonshot AI) ───────────────────────────────────────────────────────

#[cfg(feature = "kimi")]
const KIMI_ENV: ProviderEnv = ProviderEnv {
    provider: "kimi",
    key: Some("MOONSHOT_API_KEY"),
    base_url: Some("MOONSHOT_BASE_URL"),
    model: Some("MOONSHOT_MODEL"),
    timeout: Some("MOONSHOT_TIMEOUT_SECONDS"),
    retries: None,
    max_tokens: None,
};

#[cfg(feature = "kimi")]
const KIMI_DEFAULTS: CompiledDefaults = CompiledDefaults {
    base_url: Some(crate::kimi::adapter::KIMI_DEFAULT_BASE_URL),
    model: Some(crate::kimi::adapter::KIMI_DEFAULT_MODEL),
    model_required: false,
    timeout_seconds: 60,
    max_retries: DEFAULT_MAX_RETRIES,
    max_tokens: None,
};

#[cfg(feature = "kimi")]
fn kimi_config(layered: Layered) -> KimiConfig {
    KimiConfig {
        api_key: layered.api_key,
        base_url: layered.base_url,
        model: layered.model,
        timeout_seconds: layered.timeout_seconds,
        max_tokens: layered.max_tokens,
        max_retries: layered.max_retries,
        context_window: layered.context_window,
        model_overrides: layered.model_overrides,
    }
}

/// Build a [`KimiConfig`] from the layers (D-13).
///
/// # Errors
///
/// A message naming the provider, the model and the variable tried; see
/// [`resolve_layers`](crate::config::layers::resolve_layers).
#[cfg(feature = "kimi")]
pub(crate) fn kimi_from_layers(
    block: Option<&LlmProviderConfig>,
    model: Option<&str>,
    env: Env<'_>,
) -> Result<KimiConfig, String> {
    let is_reasoning = reasoning_with_overrides("kimi", block);
    resolve_layers(block, model, &KIMI_ENV, &KIMI_DEFAULTS, &is_reasoning, env).map(kimi_config)
}

#[cfg(feature = "kimi")]
impl From<&LlmProviderConfig> for KimiConfig {
    /// Convert an [`LlmProviderConfig`] into a [`KimiConfig`], falling back
    /// to Kimi's own defaults for any absent optional field.
    fn from(cfg: &LlmProviderConfig) -> Self {
        let is_reasoning = reasoning_with_overrides("kimi", Some(cfg));
        kimi_config(resolve_layers_lenient(
            cfg,
            &KIMI_ENV,
            &KIMI_DEFAULTS,
            &is_reasoning,
        ))
    }
}

// ── Qwen (DashScope) ─────────────────────────────────────────────────────────

#[cfg(feature = "qwen")]
const QWEN_ENV: ProviderEnv = ProviderEnv {
    provider: "qwen",
    key: Some("DASHSCOPE_API_KEY"),
    base_url: Some("DASHSCOPE_BASE_URL"),
    model: Some("DASHSCOPE_MODEL"),
    timeout: Some("DASHSCOPE_TIMEOUT_SECONDS"),
    retries: None,
    max_tokens: None,
};

#[cfg(feature = "qwen")]
const QWEN_DEFAULTS: CompiledDefaults = CompiledDefaults {
    base_url: Some(crate::qwen::adapter::QWEN_DEFAULT_BASE_URL),
    model: Some(crate::qwen::adapter::QWEN_DEFAULT_MODEL),
    model_required: false,
    timeout_seconds: 60,
    max_retries: DEFAULT_MAX_RETRIES,
    max_tokens: None,
};

#[cfg(feature = "qwen")]
fn qwen_config(layered: Layered) -> QwenConfig {
    QwenConfig {
        api_key: layered.api_key,
        base_url: layered.base_url,
        model: layered.model,
        timeout_seconds: layered.timeout_seconds,
        max_tokens: layered.max_tokens,
        max_retries: layered.max_retries,
        context_window: layered.context_window,
        model_overrides: layered.model_overrides,
    }
}

/// Build a [`QwenConfig`] from the layers (D-13).
///
/// # Errors
///
/// A message naming the provider, the model and the variable tried; see
/// [`resolve_layers`](crate::config::layers::resolve_layers).
#[cfg(feature = "qwen")]
pub(crate) fn qwen_from_layers(
    block: Option<&LlmProviderConfig>,
    model: Option<&str>,
    env: Env<'_>,
) -> Result<QwenConfig, String> {
    let is_reasoning = reasoning_with_overrides("qwen", block);
    resolve_layers(block, model, &QWEN_ENV, &QWEN_DEFAULTS, &is_reasoning, env).map(qwen_config)
}

#[cfg(feature = "qwen")]
impl From<&LlmProviderConfig> for QwenConfig {
    /// Convert an [`LlmProviderConfig`] into a [`QwenConfig`], falling back
    /// to Qwen's own defaults for any absent optional field.
    fn from(cfg: &LlmProviderConfig) -> Self {
        let is_reasoning = reasoning_with_overrides("qwen", Some(cfg));
        qwen_config(resolve_layers_lenient(
            cfg,
            &QWEN_ENV,
            &QWEN_DEFAULTS,
            &is_reasoning,
        ))
    }
}

// ── Grok (xAI) ───────────────────────────────────────────────────────────────

#[cfg(feature = "grok")]
const GROK_ENV: ProviderEnv = ProviderEnv {
    provider: "grok",
    key: Some("XAI_API_KEY"),
    base_url: Some("XAI_BASE_URL"),
    model: Some("XAI_MODEL"),
    timeout: Some("XAI_TIMEOUT_SECONDS"),
    retries: None,
    max_tokens: None,
};

#[cfg(feature = "grok")]
const GROK_DEFAULTS: CompiledDefaults = CompiledDefaults {
    base_url: Some(crate::grok::adapter::GROK_DEFAULT_BASE_URL),
    model: Some(crate::grok::adapter::GROK_DEFAULT_MODEL),
    model_required: false,
    timeout_seconds: 60,
    max_retries: DEFAULT_MAX_RETRIES,
    max_tokens: None,
};

#[cfg(feature = "grok")]
fn grok_config(layered: Layered) -> GrokConfig {
    GrokConfig {
        api_key: layered.api_key,
        base_url: layered.base_url,
        model: layered.model,
        timeout_seconds: layered.timeout_seconds,
        max_tokens: layered.max_tokens,
        max_retries: layered.max_retries,
        context_window: layered.context_window,
        model_overrides: layered.model_overrides,
    }
}

/// Build a [`GrokConfig`] from the layers (D-13).
///
/// # Errors
///
/// A message naming the provider, the model and the variable tried; see
/// [`resolve_layers`](crate::config::layers::resolve_layers).
#[cfg(feature = "grok")]
pub(crate) fn grok_from_layers(
    block: Option<&LlmProviderConfig>,
    model: Option<&str>,
    env: Env<'_>,
) -> Result<GrokConfig, String> {
    let is_reasoning = reasoning_with_overrides("grok", block);
    resolve_layers(block, model, &GROK_ENV, &GROK_DEFAULTS, &is_reasoning, env).map(grok_config)
}

#[cfg(feature = "grok")]
impl From<&LlmProviderConfig> for GrokConfig {
    /// Convert an [`LlmProviderConfig`] into a [`GrokConfig`], falling back
    /// to Grok's own defaults for any absent optional field.
    fn from(cfg: &LlmProviderConfig) -> Self {
        let is_reasoning = reasoning_with_overrides("grok", Some(cfg));
        grok_config(resolve_layers_lenient(
            cfg,
            &GROK_ENV,
            &GROK_DEFAULTS,
            &is_reasoning,
        ))
    }
}

// ── Gemini ───────────────────────────────────────────────────────────────────

#[cfg(feature = "gemini")]
const GEMINI_ENV: ProviderEnv = ProviderEnv {
    provider: "gemini",
    key: Some("GEMINI_API_KEY"),
    base_url: Some("GEMINI_BASE_URL"),
    model: Some("GEMINI_MODEL"),
    timeout: Some("GEMINI_TIMEOUT_SECONDS"),
    retries: None,
    max_tokens: None,
};

#[cfg(feature = "gemini")]
const GEMINI_DEFAULTS: CompiledDefaults = CompiledDefaults {
    base_url: Some(crate::gemini::adapter::GEMINI_DEFAULT_BASE_URL),
    model: Some(crate::gemini::adapter::GEMINI_DEFAULT_MODEL),
    model_required: false,
    timeout_seconds: 60,
    max_retries: DEFAULT_MAX_RETRIES,
    max_tokens: None,
};

#[cfg(feature = "gemini")]
fn gemini_config(layered: Layered) -> GeminiConfig {
    GeminiConfig {
        api_key: layered.api_key,
        base_url: layered.base_url,
        model: layered.model,
        timeout_seconds: layered.timeout_seconds,
        max_tokens: layered.max_tokens,
        max_retries: layered.max_retries,
        context_window: layered.context_window,
        model_overrides: layered.model_overrides,
    }
}

/// Build a [`GeminiConfig`] from the layers (D-13).
///
/// # Errors
///
/// A message naming the provider, the model and the variable tried; see
/// [`resolve_layers`](crate::config::layers::resolve_layers).
#[cfg(feature = "gemini")]
pub(crate) fn gemini_from_layers(
    block: Option<&LlmProviderConfig>,
    model: Option<&str>,
    env: Env<'_>,
) -> Result<GeminiConfig, String> {
    let is_reasoning = reasoning_with_overrides("gemini", block);
    resolve_layers(
        block,
        model,
        &GEMINI_ENV,
        &GEMINI_DEFAULTS,
        &is_reasoning,
        env,
    )
    .map(gemini_config)
}

#[cfg(feature = "gemini")]
impl From<&LlmProviderConfig> for GeminiConfig {
    /// Convert an [`LlmProviderConfig`] into a [`GeminiConfig`], falling
    /// back to Gemini's own defaults for any absent optional field.
    fn from(cfg: &LlmProviderConfig) -> Self {
        let is_reasoning = reasoning_with_overrides("gemini", Some(cfg));
        gemini_config(resolve_layers_lenient(
            cfg,
            &GEMINI_ENV,
            &GEMINI_DEFAULTS,
            &is_reasoning,
        ))
    }
}

// ── Ollama ───────────────────────────────────────────────────────────────────

#[cfg(feature = "ollama")]
const OLLAMA_ENV: ProviderEnv = ProviderEnv {
    provider: "ollama",
    // No credential: Ollama requires none (D-12), so no variable is read and no key resolved.
    key: None,
    base_url: Some("OLLAMA_BASE_URL"),
    model: Some("OLLAMA_MODEL"),
    timeout: Some("OLLAMA_TIMEOUT_SECONDS"),
    retries: None,
    max_tokens: None,
};

#[cfg(feature = "ollama")]
const OLLAMA_DEFAULTS: CompiledDefaults = CompiledDefaults {
    base_url: Some(crate::ollama::adapter::OLLAMA_DEFAULT_BASE_URL),
    model: Some(crate::ollama::adapter::OLLAMA_DEFAULT_MODEL),
    model_required: false,
    timeout_seconds: 120,
    max_retries: DEFAULT_MAX_RETRIES,
    max_tokens: None,
};

#[cfg(feature = "ollama")]
fn ollama_config(layered: Layered) -> OllamaConfig {
    OllamaConfig {
        base_url: layered.base_url,
        model: layered.model,
        timeout_seconds: layered.timeout_seconds,
        max_tokens: layered.max_tokens,
        max_retries: layered.max_retries,
        context_window: layered.context_window,
        model_overrides: layered.model_overrides,
        // Plan 51-06 wires the `llm.ollama.reasoning_wire` key; until then the default stands.
        reasoning_wire: crate::reasoning::ReasoningWire::Unsupported,
    }
}

/// Build an [`OllamaConfig`] from the layers (D-13). The block's `api_key` is never read:
/// [`OllamaConfig`] has no credential field at all (D-12).
///
/// # Errors
///
/// A message naming the provider, the model and the variable tried; see
/// [`resolve_layers`](crate::config::layers::resolve_layers).
#[cfg(feature = "ollama")]
pub(crate) fn ollama_from_layers(
    block: Option<&LlmProviderConfig>,
    model: Option<&str>,
    env: Env<'_>,
) -> Result<OllamaConfig, String> {
    let is_reasoning = reasoning_with_overrides("ollama", block);
    resolve_layers(
        block,
        model,
        &OLLAMA_ENV,
        &OLLAMA_DEFAULTS,
        &is_reasoning,
        env,
    )
    .map(ollama_config)
}

#[cfg(feature = "ollama")]
impl From<&LlmProviderConfig> for OllamaConfig {
    /// Convert an [`LlmProviderConfig`] into an [`OllamaConfig`], falling
    /// back to Ollama's own defaults for any absent optional field.
    ///
    /// `cfg.api_key` is deliberately **ignored** — [`OllamaConfig`] has no
    /// credential field at all (D-12: Ollama requires none). Threading an
    /// empty string through would imply a credential concept this provider
    /// does not have; the value is simply never read here.
    fn from(cfg: &LlmProviderConfig) -> Self {
        let is_reasoning = reasoning_with_overrides("ollama", Some(cfg));
        ollama_config(resolve_layers_lenient(
            cfg,
            &OLLAMA_ENV,
            &OLLAMA_DEFAULTS,
            &is_reasoning,
        ))
    }
}

// ── Generic operator-configured OpenAI-compatible provider (D-03) ──────────────

#[cfg(feature = "openai-compatible")]
const OPENAI_COMPATIBLE_ENV: ProviderEnv = ProviderEnv {
    provider: "openai-compatible",
    key: Some("OPENAI_COMPATIBLE_API_KEY"),
    base_url: Some("OPENAI_COMPATIBLE_BASE_URL"),
    model: Some("OPENAI_COMPATIBLE_MODEL"),
    timeout: Some("OPENAI_COMPATIBLE_TIMEOUT_SECONDS"),
    retries: None,
    max_tokens: None,
};

/// No base URL and no model default: there is no vendor to inherit them from (D-03), so both
/// must come from a layer above.
#[cfg(feature = "openai-compatible")]
const OPENAI_COMPATIBLE_DEFAULTS: CompiledDefaults = CompiledDefaults {
    base_url: None,
    model: None,
    model_required: true,
    timeout_seconds: 60,
    max_retries: DEFAULT_MAX_RETRIES,
    max_tokens: None,
};

/// Build an [`OpenAiCompatibleConfig`] from the layers (D-13).
///
/// Unlike every other provider, `base_url` and the model are **required** (D-03): there is no
/// vendor to default them from, so a missing one is an error naming the variable. The capability
/// declarations (`supports_*`, `max_context_tokens`, temperature range) have no block key and
/// come from the `OPENAI_COMPATIBLE_*` variables read through `env`, with the conservative
/// defaults (D-04) where unset.
///
/// # Errors
///
/// A message naming the provider, the model and the variable tried; see
/// [`resolve_layers`](crate::config::layers::resolve_layers).
#[cfg(feature = "openai-compatible")]
pub(crate) fn openai_compatible_from_layers(
    block: Option<&LlmProviderConfig>,
    model: Option<&str>,
    env: Env<'_>,
) -> Result<OpenAiCompatibleConfig, String> {
    let is_reasoning = reasoning_with_overrides("openai-compatible", block);
    let layered = resolve_layers(
        block,
        model,
        &OPENAI_COMPATIBLE_ENV,
        &OPENAI_COMPATIBLE_DEFAULTS,
        &is_reasoning,
        env,
    )?;
    let capabilities = OpenAiCompatibleCapabilitiesConfig::from_lookup(env).map_err(|e| {
        format!(
            "{}: {e}",
            crate::config::layers::subject("openai-compatible", Some(&layered.model))
        )
    })?;
    Ok(OpenAiCompatibleConfig {
        api_key: layered.api_key,
        base_url: layered.base_url,
        model: layered.model,
        timeout_seconds: layered.timeout_seconds,
        capabilities,
        max_tokens: layered.max_tokens,
        max_retries: layered.max_retries,
        context_window: layered.context_window,
        model_overrides: layered.model_overrides,
        // Plan 51-06 wires the `llm.openai-compatible.reasoning_wire` key; until then the
        // default stands.
        reasoning_wire: crate::reasoning::ReasoningWire::Unsupported,
    })
}

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
    /// this conversion could paper over with an invented placeholder. An
    /// empty key, or a `${NAME}` placeholder (there is no environment here to resolve it
    /// from), is an error too, so a placeholder is never sent as the credential.
    /// `capabilities` has no `LlmProviderConfig` counterpart at all — this
    /// bridge cannot express an operator's capability declaration, so it
    /// falls back to [`OpenAiCompatibleCapabilitiesConfig`]'s own
    /// conservative defaults (D-04: unset means the conservative answer).
    /// A caller needing capability control must construct
    /// [`OpenAiCompatibleConfig`] directly, go through
    /// [`OpenAiCompatibleConfig::from_env`], or use
    /// `LlmProviderFactory::create_from_config`, which reads the capability variables.
    fn try_from(cfg: &LlmProviderConfig) -> Result<Self, Self::Error> {
        openai_compatible_from_layers(Some(cfg), None, &|_| None)
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

#[cfg(test)]
mod tests {
    #![allow(unused_imports, dead_code)]
    use super::*;

    /// A map-backed environment: no `std::env` access in any test here.
    fn env_of<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| {
            pairs
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| (*v).to_string())
        }
    }

    // ── Anthropic: the provider with every env-backed field except retries ──

    #[cfg(feature = "anthropic")]
    mod anthropic {
        use super::*;
        use crate::anthropic::adapter::{ANTHROPIC_DEFAULT_MAX_TOKENS, ANTHROPIC_DEFAULT_MODEL};

        #[test]
        fn anthropic_env_only_reads_exactly_the_legacy_variables() {
            let env = env_of(&[
                ("ANTHROPIC_API_KEY", "sk-env"),
                ("ANTHROPIC_BASE_URL", "https://env.example/v1"),
                ("ANTHROPIC_MODEL", "claude-env"),
                ("ANTHROPIC_MAX_TOKENS", "777"),
                ("ANTHROPIC_TIMEOUT_SECONDS", "42"),
            ]);
            let config = anthropic_from_layers(None, None, &env).unwrap();
            assert_eq!(config.api_key, "sk-env");
            assert_eq!(config.base_url, "https://env.example/v1");
            assert_eq!(config.model, "claude-env");
            assert_eq!(config.max_tokens, 777);
            assert_eq!(config.timeout_seconds, 42);
            assert_eq!(config.max_retries, DEFAULT_MAX_RETRIES);
            assert_eq!(config.context_window, None);
            assert!(config.model_overrides.is_empty());
        }

        #[test]
        fn block_beats_env_for_every_field() {
            let env = env_of(&[
                ("ANTHROPIC_API_KEY", "sk-env"),
                ("ANTHROPIC_BASE_URL", "https://env.example/v1"),
                ("ANTHROPIC_MODEL", "claude-env"),
                ("ANTHROPIC_MAX_TOKENS", "777"),
                ("ANTHROPIC_TIMEOUT_SECONDS", "42"),
            ]);
            let block = LlmProviderConfig::new("sk-block")
                .with_base_url("https://block.example/v1")
                .with_default_model("claude-block")
                .with_max_tokens(555)
                .with_timeout_seconds(9)
                .with_max_retries(1)
                .with_context_window(123_456);
            let config = anthropic_from_layers(Some(&block), None, &env).unwrap();
            assert_eq!(config.api_key, "sk-block");
            assert_eq!(config.base_url, "https://block.example/v1");
            assert_eq!(config.model, "claude-block");
            assert_eq!(config.max_tokens, 555);
            assert_eq!(config.timeout_seconds, 9);
            assert_eq!(config.max_retries, 1);
            assert_eq!(config.context_window, Some(123_456));
        }

        #[test]
        fn the_stale_bridge_defaults_are_the_adapters_current_ones() {
            let block = LlmProviderConfig::new("k");
            let config = AnthropicConfig::from(&block);
            assert_eq!(config.model, ANTHROPIC_DEFAULT_MODEL);
            assert_eq!(
                config.base_url,
                crate::anthropic::adapter::ANTHROPIC_DEFAULT_BASE_URL
            );
            assert_eq!(config.max_tokens, ANTHROPIC_DEFAULT_MAX_TOKENS);
            // The adapter's own compiled timeout, not the 300 s the bridge used to carry.
            assert_eq!(config.timeout_seconds, 60);
        }

        #[test]
        fn a_per_agent_model_replaces_the_block_default() {
            let block = LlmProviderConfig::new("k").with_default_model("claude-block");
            let config =
                anthropic_from_layers(Some(&block), Some("claude-agent"), &env_of(&[])).unwrap();
            assert_eq!(config.model, "claude-agent");
        }
    }

    // ── OpenAI: retries, organization, the default-model field ──────────────

    #[cfg(feature = "openai")]
    mod openai {
        use super::*;

        #[test]
        fn openai_env_only_reads_exactly_the_legacy_variables() {
            let env = env_of(&[
                ("OPENAI_API_KEY", "sk-env"),
                ("OPENAI_BASE_URL", "https://env.example/v1"),
                ("OPENAI_ORGANIZATION", "org-env"),
                ("OPENAI_TIMEOUT_SECONDS", "42"),
                ("OPENAI_MAX_RETRIES", "6"),
            ]);
            let config = openai_from_layers(None, None, &env).unwrap();
            assert_eq!(config.api_key, "sk-env");
            assert_eq!(config.base_url, "https://env.example/v1");
            assert_eq!(config.organization.as_deref(), Some("org-env"));
            assert_eq!(config.timeout_seconds, 42);
            assert_eq!(config.max_retries, 6);
            assert_eq!(
                config.max_tokens,
                Some(crate::openai::adapter::OPENAI_DEFAULT_MAX_TOKENS)
            );
        }

        #[test]
        fn block_beats_env_for_every_field() {
            let env = env_of(&[
                ("OPENAI_API_KEY", "sk-env"),
                ("OPENAI_BASE_URL", "https://env.example/v1"),
                ("OPENAI_TIMEOUT_SECONDS", "42"),
                ("OPENAI_MAX_RETRIES", "6"),
            ]);
            let block = LlmProviderConfig::new("sk-block")
                .with_base_url("https://block.example/v1")
                .with_timeout_seconds(9)
                .with_max_retries(0)
                .with_max_tokens(321)
                .with_context_window(9_999);
            let config = openai_from_layers(Some(&block), None, &env).unwrap();
            assert_eq!(config.api_key, "sk-block");
            assert_eq!(config.base_url, "https://block.example/v1");
            assert_eq!(config.timeout_seconds, 9);
            assert_eq!(config.max_retries, 0);
            assert_eq!(config.max_tokens, Some(321));
            assert_eq!(config.context_window, Some(9_999));
        }

        #[test]
        fn openai_model_argument_becomes_the_default_model_and_is_shape_checked() {
            let env = env_of(&[("OPENAI_API_KEY", "sk-env")]);
            // A well-formed model lands on the config's model field, and the compiled default
            // is used when none is named ...
            let a = openai_from_layers(None, Some("gpt-4o-mini"), &env).unwrap();
            let b = openai_from_layers(None, None, &env).unwrap();
            assert_eq!(a.model, "gpt-4o-mini");
            assert_eq!(b.model, crate::openai::adapter::OPENAI_DEFAULT_MODEL);
            assert_eq!(a.timeout_seconds, b.timeout_seconds);
            assert_eq!(a.base_url, b.base_url);
            // ... the block's default model is the fallback below the agent's ...
            let block = LlmProviderConfig::new("sk-block").with_default_model("gpt-4o");
            let c = openai_from_layers(Some(&block), None, &env).unwrap();
            assert_eq!(c.model, "gpt-4o");
            let d = openai_from_layers(Some(&block), Some("gpt-4o-mini"), &env).unwrap();
            assert_eq!(d.model, "gpt-4o-mini");
            // ... and a malformed one is still refused, naming it.
            let error = openai_from_layers(None, Some("gpt/../x"), &env).unwrap_err();
            assert!(
                error.contains("provider 'openai' (model 'gpt/../x')"),
                "{error}"
            );
        }
    }

    // ── Kimi: the compat-preset shape, placeholders and the 300 s default ───

    #[cfg(feature = "kimi")]
    mod kimi {
        use super::*;
        use crate::kimi::adapter::{KIMI_DEFAULT_BASE_URL, KIMI_DEFAULT_MODEL};
        use crate::registry::ModelOverride;
        use std::collections::BTreeMap;

        fn reasoning_override() -> BTreeMap<String, ModelOverride> {
            BTreeMap::from([(
                "kimi-private".to_string(),
                ModelOverride {
                    reasoning: Some(true),
                    ..ModelOverride::default()
                },
            )])
        }

        #[test]
        fn kimi_env_only_reads_exactly_the_legacy_variables() {
            let env = env_of(&[
                ("MOONSHOT_API_KEY", "sk-env"),
                ("MOONSHOT_BASE_URL", "https://env.example/v1"),
                ("MOONSHOT_MODEL", "kimi-env"),
                ("MOONSHOT_TIMEOUT_SECONDS", "42"),
            ]);
            let config = kimi_from_layers(None, None, &env).unwrap();
            assert_eq!(config.api_key, "sk-env");
            assert_eq!(config.base_url, "https://env.example/v1");
            assert_eq!(config.model, "kimi-env");
            assert_eq!(config.timeout_seconds, 42);
            assert_eq!(config.max_tokens, None);
            assert_eq!(config.max_retries, DEFAULT_MAX_RETRIES);
        }

        #[test]
        fn block_beats_env_for_every_field() {
            let env = env_of(&[
                ("MOONSHOT_API_KEY", "sk-env"),
                ("MOONSHOT_BASE_URL", "https://env.example/v1"),
                ("MOONSHOT_MODEL", "kimi-env"),
                ("MOONSHOT_TIMEOUT_SECONDS", "42"),
            ]);
            let block = LlmProviderConfig::new("sk-block")
                .with_base_url("https://block.example/v1")
                .with_default_model("kimi-block")
                .with_timeout_seconds(7)
                .with_max_tokens(64)
                .with_max_retries(2)
                .with_context_window(300_000);
            let config = kimi_from_layers(Some(&block), None, &env).unwrap();
            assert_eq!(config.api_key, "sk-block");
            assert_eq!(config.base_url, "https://block.example/v1");
            assert_eq!(config.model, "kimi-block");
            assert_eq!(config.timeout_seconds, 7);
            assert_eq!(config.max_tokens, Some(64));
            assert_eq!(config.max_retries, 2);
            assert_eq!(config.context_window, Some(300_000));
        }

        #[test]
        fn per_agent_model_beats_block_default_model() {
            let block = LlmProviderConfig::new("k").with_default_model("kimi-block");
            let config = kimi_from_layers(Some(&block), Some("kimi-agent"), &env_of(&[])).unwrap();
            assert_eq!(config.model, "kimi-agent");
            // A per-agent model equal to the block's default yields an identical config.
            let same = kimi_from_layers(Some(&block), Some("kimi-block"), &env_of(&[])).unwrap();
            let plain = kimi_from_layers(Some(&block), None, &env_of(&[])).unwrap();
            assert_eq!(format!("{same:?}"), format!("{plain:?}"));
        }

        #[test]
        fn reasoning_model_defaults_to_300s_through_the_bridge() {
            let env = env_of(&[]);
            // The seeded default is a reasoning model.
            let block = LlmProviderConfig::new("k");
            let config = kimi_from_layers(Some(&block), None, &env).unwrap();
            assert_eq!(config.model, KIMI_DEFAULT_MODEL);
            assert_eq!(config.timeout_seconds, 300);
            // A model Paladin has never seen keeps the adapter's compiled default ...
            let config = kimi_from_layers(Some(&block), Some("kimi-unseen"), &env).unwrap();
            assert_eq!(config.timeout_seconds, 60);
            // ... until the block's own `models` override marks it as reasoning, which the
            // direct constructors cannot see.
            let block = LlmProviderConfig::new("k").with_models(reasoning_override());
            let config = kimi_from_layers(Some(&block), Some("kimi-private"), &env).unwrap();
            assert_eq!(config.timeout_seconds, 300);
            // An override saying `reasoning: false` beats the seed.
            let off = BTreeMap::from([(
                KIMI_DEFAULT_MODEL.to_string(),
                ModelOverride {
                    reasoning: Some(false),
                    ..ModelOverride::default()
                },
            )]);
            let block = LlmProviderConfig::new("k").with_models(off);
            let config = kimi_from_layers(Some(&block), None, &env).unwrap();
            assert_eq!(config.timeout_seconds, 60);
            // An explicit block or env timeout always wins.
            let block = LlmProviderConfig::new("k").with_timeout_seconds(20);
            assert_eq!(
                kimi_from_layers(Some(&block), None, &env)
                    .unwrap()
                    .timeout_seconds,
                20
            );
            let env = env_of(&[
                ("MOONSHOT_API_KEY", "k"),
                ("MOONSHOT_TIMEOUT_SECONDS", "33"),
            ]);
            assert_eq!(
                kimi_from_layers(None, None, &env).unwrap().timeout_seconds,
                33
            );
        }

        #[test]
        fn the_block_overrides_reach_the_config() {
            let block = LlmProviderConfig::new("k").with_models(reasoning_override());
            let config = kimi_from_layers(Some(&block), None, &env_of(&[])).unwrap();
            assert!(config.model_overrides.contains_key("kimi-private"));
        }

        // ── placeholders, through one provider's from_layers ────────────────

        #[test]
        fn a_placeholder_key_resolves_from_its_variable() {
            let env = env_of(&[("MOONSHOT_API_KEY", "sk-from-env")]);
            let block = LlmProviderConfig::new("${MOONSHOT_API_KEY}");
            let config = kimi_from_layers(Some(&block), None, &env).unwrap();
            assert_eq!(config.api_key, "sk-from-env");
        }

        #[test]
        fn an_unset_placeholder_variable_names_provider_model_and_variable() {
            let block = LlmProviderConfig::new("${MOONSHOT_API_KEY}");
            let error = kimi_from_layers(Some(&block), Some("kimi-k3"), &env_of(&[])).unwrap_err();
            assert!(
                error.starts_with("provider 'kimi' (model 'kimi-k3'):"),
                "{error}"
            );
            assert!(error.contains("MOONSHOT_API_KEY"), "{error}");
        }

        #[test]
        fn literal_lookalikes_are_used_as_written() {
            for text in ["abc${X}", "$X"] {
                let env = env_of(&[("X", "leaked")]);
                let config =
                    kimi_from_layers(Some(&LlmProviderConfig::new(text)), None, &env).unwrap();
                assert_eq!(config.api_key, text);
            }
        }

        #[test]
        fn an_empty_key_falls_back_to_the_provider_env_var_and_a_key_only_block_is_filled() {
            let env = env_of(&[
                ("MOONSHOT_API_KEY", "sk-env"),
                ("MOONSHOT_BASE_URL", "https://env.example/v1"),
            ]);
            let config = kimi_from_layers(Some(&LlmProviderConfig::new("")), None, &env).unwrap();
            assert_eq!(config.api_key, "sk-env");
            assert_eq!(config.base_url, "https://env.example/v1");
            let bare =
                kimi_from_layers(Some(&LlmProviderConfig::new("k")), None, &env_of(&[])).unwrap();
            assert_eq!(bare.base_url, KIMI_DEFAULT_BASE_URL);
            assert_eq!(bare.model, KIMI_DEFAULT_MODEL);
        }

        #[test]
        fn no_block_and_no_env_var_is_an_error_naming_provider_model_and_variable() {
            let error = kimi_from_layers(None, Some("kimi-k3"), &env_of(&[])).unwrap_err();
            assert!(
                error.starts_with("provider 'kimi' (model 'kimi-k3'):"),
                "{error}"
            );
            assert!(error.contains("MOONSHOT_API_KEY"), "{error}");
        }

        #[test]
        fn the_infallible_from_never_carries_a_placeholder_as_the_key() {
            let block = LlmProviderConfig::new("${MOONSHOT_API_KEY}");
            let config = KimiConfig::from(&block);
            assert_eq!(
                config.api_key, "",
                "an unresolved placeholder must not be sent"
            );
            let keyed = KimiConfig::from(&LlmProviderConfig::new("sk-test"));
            assert_eq!(keyed.api_key, "sk-test");
        }

        #[test]
        fn no_error_contains_a_key_value() {
            let env = env_of(&[("MOONSHOT_TIMEOUT_SECONDS", "sk-leaky-credential")]);
            let block = LlmProviderConfig::new("sk-block-credential");
            let errors = [
                kimi_from_layers(Some(&block), None, &env).unwrap_err(),
                kimi_from_layers(Some(&block), Some("a b"), &env_of(&[])).unwrap_err(),
            ];
            for error in errors {
                assert!(!error.contains("sk-leaky-credential"), "{error}");
                assert!(!error.contains("sk-block-credential"), "{error}");
            }
        }
    }

    // ── The other compat presets and the bespoke adapters: env names ────────

    #[cfg(feature = "qwen")]
    #[test]
    fn qwen_env_only_reads_exactly_the_legacy_variables() {
        let env = env_of(&[
            ("DASHSCOPE_API_KEY", "sk-env"),
            ("DASHSCOPE_BASE_URL", "https://env.example/v1"),
            ("DASHSCOPE_MODEL", "qwen-env"),
            ("DASHSCOPE_TIMEOUT_SECONDS", "42"),
        ]);
        let config = qwen_from_layers(None, None, &env).unwrap();
        assert_eq!(
            (
                config.api_key.as_str(),
                config.base_url.as_str(),
                config.model.as_str()
            ),
            ("sk-env", "https://env.example/v1", "qwen-env")
        );
        assert_eq!(config.timeout_seconds, 42);
        // qwen-plus is seeded as not reasoning; qwen3.8-flash (the default) is.
        let plain = qwen_from_layers(
            None,
            Some("qwen-plus"),
            &env_of(&[("DASHSCOPE_API_KEY", "k")]),
        )
        .unwrap();
        assert_eq!(plain.timeout_seconds, 60);
        let reasoning =
            qwen_from_layers(None, None, &env_of(&[("DASHSCOPE_API_KEY", "k")])).unwrap();
        assert_eq!(reasoning.timeout_seconds, 300);
    }

    #[cfg(feature = "grok")]
    #[test]
    fn grok_env_only_reads_exactly_the_legacy_variables() {
        let env = env_of(&[
            ("XAI_API_KEY", "sk-env"),
            ("XAI_BASE_URL", "https://env.example/v1"),
            ("XAI_MODEL", "grok-env"),
            ("XAI_TIMEOUT_SECONDS", "42"),
        ]);
        let config = grok_from_layers(None, None, &env).unwrap();
        assert_eq!(
            (
                config.api_key.as_str(),
                config.base_url.as_str(),
                config.model.as_str()
            ),
            ("sk-env", "https://env.example/v1", "grok-env")
        );
        assert_eq!(config.timeout_seconds, 42);
    }

    #[cfg(feature = "gemini")]
    #[test]
    fn gemini_env_only_reads_exactly_the_legacy_variables() {
        let env = env_of(&[
            ("GEMINI_API_KEY", "sk-env"),
            ("GEMINI_BASE_URL", "https://env.example/v1"),
            ("GEMINI_MODEL", "gemini-env"),
            ("GEMINI_TIMEOUT_SECONDS", "42"),
        ]);
        let config = gemini_from_layers(None, None, &env).unwrap();
        assert_eq!(
            (
                config.api_key.as_str(),
                config.base_url.as_str(),
                config.model.as_str()
            ),
            ("sk-env", "https://env.example/v1", "gemini-env")
        );
        assert_eq!(config.timeout_seconds, 42);
    }

    #[cfg(feature = "deepseek")]
    #[test]
    fn deepseek_env_only_reads_exactly_the_legacy_variables_and_defaults_are_current() {
        let env = env_of(&[
            ("DEEPSEEK_API_KEY", "sk-env"),
            ("DEEPSEEK_BASE_URL", "https://env.example/v1"),
            ("DEEPSEEK_MODEL", "deepseek-env"),
            ("DEEPSEEK_TIMEOUT_SECONDS", "42"),
        ]);
        let config = deepseek_from_layers(None, None, &env).unwrap();
        assert_eq!(
            (
                config.api_key.as_str(),
                config.base_url.as_str(),
                config.model.as_str()
            ),
            ("sk-env", "https://env.example/v1", "deepseek-env")
        );
        assert_eq!(config.timeout_seconds, 42);
        let defaulted = DeepSeekConfig::from(&LlmProviderConfig::new("k"));
        assert_eq!(
            defaulted.model,
            crate::deepseek::adapter::DEEPSEEK_DEFAULT_MODEL
        );
        assert_eq!(
            defaulted.base_url,
            crate::deepseek::adapter::DEEPSEEK_DEFAULT_BASE_URL
        );
        // `deepseek-flash` is a seeded reasoning model: 300 s, not the old bridge's 60 s.
        assert_eq!(defaulted.timeout_seconds, 300);
    }

    #[cfg(feature = "ollama")]
    #[test]
    fn ollama_needs_no_key_and_reads_exactly_the_legacy_variables() {
        let env = env_of(&[
            ("OLLAMA_BASE_URL", "http://env.example:11434/v1"),
            ("OLLAMA_MODEL", "llama-env"),
            ("OLLAMA_TIMEOUT_SECONDS", "42"),
        ]);
        let config = ollama_from_layers(None, None, &env).unwrap();
        assert_eq!(config.base_url, "http://env.example:11434/v1");
        assert_eq!(config.model, "llama-env");
        assert_eq!(config.timeout_seconds, 42);
        // An empty environment still builds: no credential exists (D-12).
        let bare = ollama_from_layers(None, None, &env_of(&[])).unwrap();
        assert_eq!(bare.timeout_seconds, 120);
        // A block's api_key is never read, even a placeholder that cannot resolve.
        let block = LlmProviderConfig::new("${NOT_SET}");
        assert!(ollama_from_layers(Some(&block), None, &env_of(&[])).is_ok());
    }

    #[cfg(feature = "openai-compatible")]
    mod openai_compatible {
        use super::*;

        fn full_env() -> Vec<(&'static str, &'static str)> {
            vec![
                ("OPENAI_COMPATIBLE_API_KEY", "sk-env"),
                ("OPENAI_COMPATIBLE_BASE_URL", "https://env.example/v1"),
                ("OPENAI_COMPATIBLE_MODEL", "compat-env"),
                ("OPENAI_COMPATIBLE_TIMEOUT_SECONDS", "42"),
                ("OPENAI_COMPATIBLE_SUPPORTS_VISION", "true"),
                ("OPENAI_COMPATIBLE_MAX_CONTEXT_TOKENS", "32768"),
            ]
        }

        #[test]
        fn env_only_reads_the_legacy_variables_including_the_capability_declarations() {
            let pairs = full_env();
            let env = env_of(&pairs);
            let config = openai_compatible_from_layers(None, None, &env).unwrap();
            assert_eq!(config.api_key, "sk-env");
            assert_eq!(config.base_url, "https://env.example/v1");
            assert_eq!(config.model, "compat-env");
            assert_eq!(config.timeout_seconds, 42);
            assert!(config.capabilities.supports_vision);
            assert_eq!(config.capabilities.max_context_tokens, Some(32_768));
        }

        #[test]
        fn base_url_and_model_are_required_and_named() {
            let block = LlmProviderConfig::new("k");
            let error =
                openai_compatible_from_layers(Some(&block), None, &env_of(&[])).unwrap_err();
            assert!(error.contains("OPENAI_COMPATIBLE_MODEL"), "{error}");
            let block = LlmProviderConfig::new("k").with_default_model("m");
            let error =
                openai_compatible_from_layers(Some(&block), None, &env_of(&[])).unwrap_err();
            assert!(
                error.starts_with("provider 'openai-compatible' (model 'm'):"),
                "{error}"
            );
            assert!(error.contains("OPENAI_COMPATIBLE_BASE_URL"), "{error}");
        }

        #[test]
        fn a_block_with_a_base_url_and_model_builds_with_conservative_capabilities() {
            let block = LlmProviderConfig::new("k")
                .with_base_url("https://block.example/v1")
                .with_default_model("m");
            let config = OpenAiCompatibleConfig::try_from(&block).unwrap();
            assert!(config.capabilities.supports_streaming);
            assert!(!config.capabilities.supports_vision);
            assert!(!config.capabilities.supports_system_messages);
            assert_eq!(config.capabilities.max_context_tokens, None);
        }

        #[test]
        fn a_malformed_capability_variable_is_an_error_naming_it() {
            let block = LlmProviderConfig::new("k")
                .with_base_url("https://b.example/v1")
                .with_default_model("m");
            let env = env_of(&[("OPENAI_COMPATIBLE_SUPPORTS_VISION", "maybe")]);
            let error = openai_compatible_from_layers(Some(&block), None, &env).unwrap_err();
            assert!(
                error.contains("OPENAI_COMPATIBLE_SUPPORTS_VISION"),
                "{error}"
            );
            assert!(
                error.starts_with("provider 'openai-compatible' (model 'm'):"),
                "{error}"
            );
        }

        #[test]
        fn try_from_refuses_an_unresolvable_placeholder_instead_of_sending_it() {
            let block = LlmProviderConfig::new("${OPENAI_COMPATIBLE_API_KEY}")
                .with_base_url("https://b.example/v1")
                .with_default_model("m");
            let error = OpenAiCompatibleConfig::try_from(&block).unwrap_err();
            assert!(error.contains("OPENAI_COMPATIBLE_API_KEY"), "{error}");
        }
    }
}
