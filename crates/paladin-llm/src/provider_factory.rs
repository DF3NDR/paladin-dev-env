//! # LLM Provider Factory
//!
//! Creates [`paladin_ports::output::llm_port::LlmPort`] adapter instances by
//! provider name: from the provider's legacy environment variables
//! ([`create`](crate::provider_factory::LlmProviderFactory::create)) or from an `llm.<provider>` configuration block
//! ([`create_from_config`](crate::provider_factory::LlmProviderFactory::create_from_config), D-12), both through the same
//! layering (D-13; `config::layers`).
//!
//! Backed by a single `cfg`-gated, table-driven registry (D-10):
//! [`create`](crate::provider_factory::LlmProviderFactory::create),
//! [`get_default_provider`](crate::provider_factory::LlmProviderFactory::get_default_provider),
//! [`list_available_providers`](crate::provider_factory::LlmProviderFactory::list_available_providers)
//! and [`ProviderFactoryError::UnknownProvider`](crate::provider_factory::ProviderFactoryError::UnknownProvider)'s message all derive from
//! the same table. Adding a provider is one row in `provider_registry`;
//! a provider whose feature is compiled out is structurally absent from
//! every one of those surfaces — there is no second lookup path to forget
//! to `cfg`-gate (the defect this replaces:
//! `provider_factory.rs:123-149` in the pre-D-10 shape reported a provider
//! as available whenever its env var was set, even when its feature had
//! been compiled out).

use std::sync::{Arc, OnceLock};

use paladin_ports::output::llm_port::LlmPort;
use thiserror::Error;

use crate::config::layers::{self, SecretSource, subject};
use crate::config::llm::{LlmConfig, LlmProviderConfig};

/// Errors that can be returned by [`LlmProviderFactory`].
#[derive(Debug, Error)]
pub enum ProviderFactoryError {
    /// The requested provider name is not recognised.
    #[error(
        "Unknown provider: {0}. Supported providers: {supported}",
        supported = provider_names().join(", ")
    )]
    UnknownProvider(String),

    /// Required environment variables or configuration are missing.
    #[error("Provider configuration missing: {0}")]
    ConfigurationMissing(String),

    /// The provider adapter could not be constructed.
    #[error("Failed to create provider adapter: {0}")]
    AdapterCreationFailed(String),
}

/// How a construction function looks an environment variable up: the process environment in
/// production, a map in tests (`std::env::set_var` is `unsafe` under edition 2024).
type EnvLookup<'a> = &'a dyn Fn(&str) -> Option<String>;

/// The process environment, as [`EnvLookup`].
fn process_env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/// One row of the provider registry: a name, the env var whose presence
/// signals a credential is configured (`None` for a provider that needs no
/// credential, e.g. a future local-endpoint preset), and the constructor
/// function.
struct ProviderRegistration {
    name: &'static str,
    env_var: Option<&'static str>,
    /// Build the adapter from, in D-13 order: the per-agent model, the provider's
    /// `llm.<provider>` block, the provider's legacy environment variables and the adapter's
    /// compiled defaults. Calls that provider's `<provider>_from_layers` in `config/bridge.rs`.
    construct_from_config: ConstructFromConfig,
}

/// The shape of a registry row's constructor: the provider's block, the per-agent model, and the
/// environment lookup in, an adapter (or a named error) out.
type ConstructFromConfig = fn(
    Option<&LlmProviderConfig>,
    Option<&str>,
    EnvLookup<'_>,
) -> Result<Arc<dyn LlmPort>, ProviderFactoryError>;

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
/// The error for a failure of `*Adapter::new`: the prefix names the provider and the model, and
/// `cause` is the adapter's own message (which never carries the key).
fn creation_failed(
    subject: &str,
    label: &str,
    cause: impl std::fmt::Display,
) -> ProviderFactoryError {
    ProviderFactoryError::AdapterCreationFailed(format!(
        "{subject}: Failed to create {label} adapter: {cause}"
    ))
}

#[cfg(feature = "openai")]
fn construct_openai(
    block: Option<&LlmProviderConfig>,
    model: Option<&str>,
    env: EnvLookup<'_>,
) -> Result<Arc<dyn LlmPort>, ProviderFactoryError> {
    use crate::config::bridge::openai_from_layers;
    use crate::openai::OpenAIAdapter;
    let config = openai_from_layers(block, model, env)
        .map_err(ProviderFactoryError::ConfigurationMissing)?;
    // The model that labels errors is the one the caller or block named, not the compiled
    // default `OpenAIConfig.model` falls back to, so a message names only what was stated.
    let named = model
        .map(str::to_string)
        .or_else(|| block.and_then(|b| b.default_model.clone()));
    let subject = subject("openai", named.as_deref());
    let adapter = OpenAIAdapter::new(config).map_err(|e| creation_failed(&subject, "OpenAI", e))?;
    Ok(Arc::new(adapter))
}

#[cfg(feature = "deepseek")]
fn construct_deepseek(
    block: Option<&LlmProviderConfig>,
    model: Option<&str>,
    env: EnvLookup<'_>,
) -> Result<Arc<dyn LlmPort>, ProviderFactoryError> {
    use crate::config::bridge::deepseek_from_layers;
    use crate::deepseek::DeepSeekAdapter;
    let config = deepseek_from_layers(block, model, env)
        .map_err(ProviderFactoryError::ConfigurationMissing)?;
    let subject = subject("deepseek", Some(&config.model));
    let adapter =
        DeepSeekAdapter::new(config).map_err(|e| creation_failed(&subject, "DeepSeek", e))?;
    Ok(Arc::new(adapter))
}

#[cfg(feature = "anthropic")]
fn construct_anthropic(
    block: Option<&LlmProviderConfig>,
    model: Option<&str>,
    env: EnvLookup<'_>,
) -> Result<Arc<dyn LlmPort>, ProviderFactoryError> {
    use crate::anthropic::AnthropicAdapter;
    use crate::config::bridge::anthropic_from_layers;
    let config = anthropic_from_layers(block, model, env)
        .map_err(ProviderFactoryError::ConfigurationMissing)?;
    let subject = subject("anthropic", Some(&config.model));
    let adapter =
        AnthropicAdapter::new(config).map_err(|e| creation_failed(&subject, "Anthropic", e))?;
    Ok(Arc::new(adapter))
}

#[cfg(feature = "kimi")]
fn construct_kimi(
    block: Option<&LlmProviderConfig>,
    model: Option<&str>,
    env: EnvLookup<'_>,
) -> Result<Arc<dyn LlmPort>, ProviderFactoryError> {
    use crate::config::bridge::kimi_from_layers;
    use crate::kimi::KimiAdapter;
    let config =
        kimi_from_layers(block, model, env).map_err(ProviderFactoryError::ConfigurationMissing)?;
    let subject = subject("kimi", Some(&config.model));
    let adapter = KimiAdapter::new(config).map_err(|e| creation_failed(&subject, "Kimi", e))?;
    Ok(Arc::new(adapter))
}

#[cfg(feature = "qwen")]
fn construct_qwen(
    block: Option<&LlmProviderConfig>,
    model: Option<&str>,
    env: EnvLookup<'_>,
) -> Result<Arc<dyn LlmPort>, ProviderFactoryError> {
    use crate::config::bridge::qwen_from_layers;
    use crate::qwen::QwenAdapter;
    let config =
        qwen_from_layers(block, model, env).map_err(ProviderFactoryError::ConfigurationMissing)?;
    let subject = subject("qwen", Some(&config.model));
    let adapter = QwenAdapter::new(config).map_err(|e| creation_failed(&subject, "Qwen", e))?;
    Ok(Arc::new(adapter))
}

#[cfg(feature = "grok")]
fn construct_grok(
    block: Option<&LlmProviderConfig>,
    model: Option<&str>,
    env: EnvLookup<'_>,
) -> Result<Arc<dyn LlmPort>, ProviderFactoryError> {
    use crate::config::bridge::grok_from_layers;
    use crate::grok::GrokAdapter;
    let config =
        grok_from_layers(block, model, env).map_err(ProviderFactoryError::ConfigurationMissing)?;
    let subject = subject("grok", Some(&config.model));
    let adapter = GrokAdapter::new(config).map_err(|e| creation_failed(&subject, "Grok", e))?;
    Ok(Arc::new(adapter))
}

#[cfg(feature = "gemini")]
fn construct_gemini(
    block: Option<&LlmProviderConfig>,
    model: Option<&str>,
    env: EnvLookup<'_>,
) -> Result<Arc<dyn LlmPort>, ProviderFactoryError> {
    use crate::config::bridge::gemini_from_layers;
    use crate::gemini::GeminiAdapter;
    let config = gemini_from_layers(block, model, env)
        .map_err(ProviderFactoryError::ConfigurationMissing)?;
    let subject = subject("gemini", Some(&config.model));
    let adapter = GeminiAdapter::new(config).map_err(|e| creation_failed(&subject, "Gemini", e))?;
    Ok(Arc::new(adapter))
}

#[cfg(feature = "openai-compatible")]
fn construct_openai_compatible(
    block: Option<&LlmProviderConfig>,
    model: Option<&str>,
    env: EnvLookup<'_>,
) -> Result<Arc<dyn LlmPort>, ProviderFactoryError> {
    use crate::config::bridge::openai_compatible_from_layers;
    use crate::openai_compatible::OpenAiCompatibleAdapter;
    let config = openai_compatible_from_layers(block, model, env)
        .map_err(ProviderFactoryError::ConfigurationMissing)?;
    let subject = subject("openai-compatible", Some(&config.model));
    let adapter = OpenAiCompatibleAdapter::new(config)
        .map_err(|e| creation_failed(&subject, "openai-compatible", e))?;
    Ok(Arc::new(adapter))
}

#[cfg(feature = "ollama")]
fn construct_ollama(
    block: Option<&LlmProviderConfig>,
    model: Option<&str>,
    env: EnvLookup<'_>,
) -> Result<Arc<dyn LlmPort>, ProviderFactoryError> {
    use crate::config::bridge::ollama_from_layers;
    use crate::ollama::OllamaAdapter;
    // No credential to resolve: Ollama requires none (D-12).
    let config = ollama_from_layers(block, model, env)
        .map_err(ProviderFactoryError::ConfigurationMissing)?;
    let subject = subject("ollama", Some(&config.model));
    let adapter = OllamaAdapter::new(config).map_err(|e| creation_failed(&subject, "Ollama", e))?;
    Ok(Arc::new(adapter))
}

/// Build the `cfg`-gated provider registry table. Exactly one row per
/// enabled provider feature; a provider whose feature is compiled out has
/// no row at all, so it cannot be reported as available regardless of
/// whether its env var happens to be set.
///
/// `vec![]` cannot express this construction: each row is individually
/// `#[cfg]`-gated on its own provider feature, so the number of pushes
/// varies per build — clippy's `vec_init_then_push` lint cannot see that
/// from a single call site, hence the explicit `allow`.
#[allow(unused_mut, clippy::vec_init_then_push)]
fn build_provider_registry() -> Vec<ProviderRegistration> {
    let mut rows: Vec<ProviderRegistration> = Vec::new();

    #[cfg(feature = "openai")]
    rows.push(ProviderRegistration {
        name: "openai",
        env_var: Some("OPENAI_API_KEY"),
        construct_from_config: construct_openai,
    });

    #[cfg(feature = "deepseek")]
    rows.push(ProviderRegistration {
        name: "deepseek",
        env_var: Some("DEEPSEEK_API_KEY"),
        construct_from_config: construct_deepseek,
    });

    #[cfg(feature = "anthropic")]
    rows.push(ProviderRegistration {
        name: "anthropic",
        env_var: Some("ANTHROPIC_API_KEY"),
        construct_from_config: construct_anthropic,
    });

    #[cfg(feature = "kimi")]
    rows.push(ProviderRegistration {
        name: "kimi",
        env_var: Some("MOONSHOT_API_KEY"),
        construct_from_config: construct_kimi,
    });

    #[cfg(feature = "qwen")]
    rows.push(ProviderRegistration {
        name: "qwen",
        env_var: Some("DASHSCOPE_API_KEY"),
        construct_from_config: construct_qwen,
    });

    #[cfg(feature = "grok")]
    rows.push(ProviderRegistration {
        name: "grok",
        env_var: Some("XAI_API_KEY"),
        construct_from_config: construct_grok,
    });

    // Gemini is a curated (named-vendor) preset like Kimi/Qwen/Grok — it
    // just happens to be bespoke-protocol rather than compat-engine-backed
    // (D-08). Declared alongside the other curated presets, before the
    // generic openai-compatible row and Ollama's credential-free row, for
    // the same reason those two are ordered last: neither should ever
    // pre-empt an explicitly-configured named provider in
    // `get_default_provider()`'s declared-table-order scan.
    #[cfg(feature = "gemini")]
    rows.push(ProviderRegistration {
        name: "gemini",
        env_var: Some("GEMINI_API_KEY"),
        construct_from_config: construct_gemini,
    });

    // Placed after every curated (named-vendor) preset row so it never
    // pre-empts an explicitly-configured named provider in
    // `get_default_provider()`'s declared-table-order scan, but BEFORE
    // Ollama: Ollama's `env_var: None` row unconditionally "matches" in
    // that scan, so if it were declared first it would always win and the
    // generic provider's own credential would never be reachable through
    // `get_default_provider()` at all.
    #[cfg(feature = "openai-compatible")]
    rows.push(ProviderRegistration {
        name: "openai-compatible",
        env_var: Some("OPENAI_COMPATIBLE_API_KEY"),
        construct_from_config: construct_openai_compatible,
    });

    // Placed after every credentialed row: a compiled-in, credential-free
    // Ollama must never pre-empt an explicitly-configured hosted provider
    // in `get_default_provider()`'s declared-table-order scan.
    #[cfg(feature = "ollama")]
    rows.push(ProviderRegistration {
        name: "ollama",
        env_var: None,
        construct_from_config: construct_ollama,
    });

    rows
}

/// The `cfg`-gated provider registry table, memoized for the process
/// lifetime.
fn provider_registry() -> &'static [ProviderRegistration] {
    static REGISTRY: OnceLock<Vec<ProviderRegistration>> = OnceLock::new();
    REGISTRY.get_or_init(build_provider_registry)
}

/// The names of every provider compiled into this build, in registry
/// declaration order. Consumed by the config layer and by documentation
/// tooling so both read one list rather than each hand-maintaining a copy.
pub fn provider_names() -> Vec<&'static str> {
    provider_registry().iter().map(|row| row.name).collect()
}

/// The registry row for `provider_name`, matched case-insensitively with `_` and `-` alike.
///
/// Mirrors `config/llm.rs`'s `get_provider_config`, which accepts both the hyphenated and
/// underscore spellings of "openai-compatible" — the two must stay aligned so a name the config
/// layer blesses (WR-01) is always a name this factory resolves. No registry row contains an
/// underscore, so this transform can only merge separator spellings of the same row; it cannot
/// collide two distinct providers.
fn find_row(provider_name: &str) -> Option<&'static ProviderRegistration> {
    let lower = provider_name.to_lowercase().replace('_', "-");
    provider_registry().iter().find(|row| row.name == lower)
}

/// `provider (model 'model')` for an error about a provider this build does not register: the
/// provider as the caller wrote it, and the model escaped and bounded like every other echo.
fn subject_suffix(provider: &str, model: &str) -> String {
    let named = subject("x", Some(model));
    // `subject` renders `provider 'x' (model '<escaped>')`; reuse its escaping for the model part.
    let model_part = named.strip_prefix("provider 'x' ").unwrap_or("(model '')");
    format!("{provider} {model_part}")
}

/// Factory for creating LLM provider adapters by name.
///
/// Each provider is only available when the corresponding feature flag is
/// enabled. See [`provider_names`] for the list compiled into this build.
///
/// # Example
///
/// ```rust,no_run
/// # #[cfg(feature = "openai")]
/// # {
/// use paladin_llm::provider_factory::LlmProviderFactory;
///
/// let factory = LlmProviderFactory::new();
/// let provider = factory.create("openai").expect("OPENAI_API_KEY must be set");
/// # }
/// ```
pub struct LlmProviderFactory;

impl LlmProviderFactory {
    /// Create a new provider factory.
    pub fn new() -> Self {
        Self
    }

    /// Create an [`LlmPort`] adapter by provider name.
    ///
    /// Configuration is loaded from environment variables (see each
    /// adapter's `*Config::from_env()` for the expected variable names).
    /// The name is matched case-insensitively **and with underscores
    /// normalised to hyphens** against [`provider_names`], so
    /// `"openai-compatible"`, `"openai_compatible"` and
    /// `"OpenAI_Compatible"` all resolve to the one generic-provider row.
    /// `"openai-compatible"` remains the canonical spelling reported by
    /// [`provider_names`] and by `get_provider_name()` (D-07, D-09).
    ///
    /// This goes through the same layering as
    /// [`create_from_config`](Self::create_from_config) with no `llm.<provider>` block, so the
    /// environment variables are the only operator input, apart from one change: a model that is
    /// seeded as a reasoning model (or the provider's default when it is one) gets a 300 s
    /// timeout instead of 60 s unless the provider's timeout variable sets one (D-13).
    ///
    /// # Errors
    ///
    /// Returns [`ProviderFactoryError`] if the provider name is unknown, a
    /// required environment variable is absent, or the adapter fails to
    /// initialise.
    pub fn create(&self, provider_name: &str) -> Result<Arc<dyn LlmPort>, ProviderFactoryError> {
        match find_row(provider_name) {
            Some(row) => (row.construct_from_config)(None, None, &process_env),
            None => Err(ProviderFactoryError::UnknownProvider(
                provider_name.to_string(),
            )),
        }
    }

    /// Create an [`LlmPort`] adapter for `provider` from the `llm.<provider>` block of `config`
    /// (D-12), optionally for a specific per-agent `model`.
    ///
    /// # Precedence (D-13)
    ///
    /// Every input is taken from the most specific operator statement that sets it, field by
    /// field: the per-agent `model`, then the provider's block in `config`, then the provider's
    /// legacy environment variables (exactly the ones its `from_env` reads), then the adapter's
    /// compiled default. An environment variable never overrides a value written in the block.
    /// A `model` equal to the block's `default_model` builds an identical adapter.
    ///
    /// * **Key.** The block's `api_key` is used as written unless it is empty (then the
    ///   provider's key variable supplies it) or an exact `${NAME}` placeholder (then the
    ///   variable `NAME` supplies it). Only an exact match is a placeholder: `abc${X}` and `$X`
    ///   are used literally. A placeholder whose variable is unset is an error, never sent to the
    ///   vendor as the credential. Ollama has no key.
    /// * **Timeout.** When neither the block nor the provider's timeout variable sets one, a
    ///   model whose profile (seeded, or the block's `models` override) is `reasoning: true`
    ///   gets 300 s; any other model keeps the adapter's compiled default.
    /// * **Overrides.** The block's `max_tokens`, `context_window` and `models.<model-id>`
    ///   reach the adapter, so its `model_profile` reports them.
    /// * **Provider name.** Matched like [`create`](Self::create): case-insensitively, with `_`
    ///   and `-` alike, so the block of `openai_compatible` is found for any spelling.
    ///
    /// For OpenAI, `model` only selects the profile the timeout default is decided from: the
    /// adapter takes the model with each request.
    ///
    /// A `model` is refused before any adapter is built when it is empty or whitespace, longer
    /// than 256 characters, contains whitespace or a control character, contains `..`, `?`, `#`
    /// or `%`, or starts with `/`.
    ///
    /// # Errors
    ///
    /// Every error names the provider and, where one is known, the model
    /// (`provider 'kimi' (model 'kimi-k3'): ...`), and for a missing credential the environment
    /// variable or placeholder variable it tried -- never a key value.
    /// [`ProviderFactoryError::UnknownProvider`] carries `"<provider> (model '<model>')"`;
    /// [`ProviderFactoryError::ConfigurationMissing`] a missing or malformed input;
    /// [`ProviderFactoryError::AdapterCreationFailed`] a refusal by the adapter itself (an
    /// invalid `models` override, a window below a model's maximum output) or a malformed
    /// `model`.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "kimi")]
    /// # {
    /// use paladin_llm::config::llm::{LlmConfig, LlmProviderConfig};
    /// use paladin_llm::provider_factory::LlmProviderFactory;
    ///
    /// let config = LlmConfig {
    ///     kimi: Some(
    ///         LlmProviderConfig::new("${MOONSHOT_API_KEY}")
    ///             .with_default_model("kimi-k3")
    ///             .with_timeout_seconds(120),
    ///     ),
    ///     ..LlmConfig::default()
    /// };
    /// let port = LlmProviderFactory::new()
    ///     .create_from_config(&config, "kimi", None)
    ///     .expect("MOONSHOT_API_KEY must be set");
    /// # }
    /// ```
    pub fn create_from_config(
        &self,
        config: &LlmConfig,
        provider: &str,
        model: Option<&str>,
    ) -> Result<Arc<dyn LlmPort>, ProviderFactoryError> {
        self.create_from_config_with_env(config, provider, model, &process_env)
    }

    /// [`create_from_config`](Self::create_from_config) over an injected environment lookup.
    ///
    /// A pure function of its inputs and the environment snapshot `env` returns: it mutates no
    /// global state, holds no lock and writes no environment, so concurrent callers building
    /// different providers and models each get an independent adapter.
    pub(crate) fn create_from_config_with_env(
        &self,
        config: &LlmConfig,
        provider: &str,
        model: Option<&str>,
        env: EnvLookup<'_>,
    ) -> Result<Arc<dyn LlmPort>, ProviderFactoryError> {
        let Some(row) = find_row(provider) else {
            let named = match model {
                Some(model) => subject_suffix(provider, model),
                None => provider.to_string(),
            };
            return Err(ProviderFactoryError::UnknownProvider(named));
        };
        if let Some(requested) = model
            && let Err(reason) = layers::validate_model_id(requested)
        {
            return Err(ProviderFactoryError::AdapterCreationFailed(format!(
                "{}: model id {reason}",
                subject(row.name, Some(requested))
            )));
        }
        (row.construct_from_config)(config.get_provider_config(row.name), model, env)
    }

    /// Whether `provider` can be constructed from `config` and the process environment, judged
    /// by its credential: the provider is keyless (Ollama), or its block carries a usable key
    /// (non-empty, or an exact `${NAME}` placeholder whose variable is set), or its legacy key
    /// variable is set and non-blank. A provider configured purely in YAML is therefore
    /// available (RESEARCH Pitfall 8), unlike [`list_available_providers`](Self::list_available_providers), which is
    /// environment-only.
    ///
    /// A block whose key is a placeholder with an unset variable is **not** configured, even when
    /// the provider's own variable is set: construction refuses it rather than substitute a
    /// credential the operator did not name, so this reports what
    /// [`create_from_config`](Self::create_from_config) would do. An unknown or compiled-out
    /// provider is never configured. Only the credential is judged: a required base URL or model
    /// that is still missing surfaces from `create_from_config`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_llm::provider_factory::LlmProviderFactory;
    ///
    /// // No configuration and a name no build registers.
    /// assert!(!LlmProviderFactory::is_configured(None, "no-such-provider"));
    /// ```
    pub fn is_configured(config: Option<&LlmConfig>, provider: &str) -> bool {
        Self::is_configured_with_env(config, provider, &process_env)
    }

    /// [`is_configured`](Self::is_configured) over an injected environment lookup.
    pub(crate) fn is_configured_with_env(
        config: Option<&LlmConfig>,
        provider: &str,
        env: EnvLookup<'_>,
    ) -> bool {
        let Some(row) = find_row(provider) else {
            return false;
        };
        let Some(var) = row.env_var else {
            return true;
        };
        let provider_env_set = env(var).is_some_and(|v| !v.trim().is_empty());
        match config.and_then(|c| c.get_provider_config(row.name)) {
            None => provider_env_set,
            Some(block) => match layers::resolve_secret_placeholder(&block.api_key, env) {
                SecretSource::Literal(_) | SecretSource::FromEnv { .. } => true,
                SecretSource::MissingEnv { .. } => false,
                SecretSource::Empty => provider_env_set,
            },
        }
    }

    /// Return the name of the first available provider — the first table
    /// row (in registry declaration order) whose credential env var holds a
    /// non-blank value, or whose row requires no credential at all.
    ///
    /// A variable set to an empty or whitespace-only string is treated as
    /// unset: every preset's own `validate()` rejects such a value, so
    /// reporting it as available here would name a provider `create()`
    /// refuses to construct.
    ///
    /// Returns `None` when the table is empty (no provider feature
    /// compiled in) or no row's credential is present and non-blank.
    pub fn get_default_provider() -> Option<String> {
        provider_registry()
            .iter()
            .find(|row| match row.env_var {
                Some(var) => std::env::var(var).is_ok_and(|v| !v.trim().is_empty()),
                None => true,
            })
            .map(|row| row.name.to_string())
    }

    /// Return the names of all providers that are both compiled in and
    /// have their credential configured (or need none).
    ///
    /// A provider whose feature was not compiled in is structurally absent
    /// from this list regardless of whether its env var happens to be set.
    /// A variable set to an empty or whitespace-only string is likewise
    /// treated as not configured, so this list stays in agreement with
    /// [`create`](Self::create): a name reported here is always one
    /// `create()` can actually construct.
    pub fn list_available_providers() -> Vec<String> {
        provider_registry()
            .iter()
            .filter(|row| match row.env_var {
                Some(var) => std::env::var(var).is_ok_and(|v| !v.trim().is_empty()),
                None => true,
            })
            .map(|row| row.name.to_string())
            .collect()
    }
}

impl Default for LlmProviderFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factory_creation() {
        let factory = LlmProviderFactory::new();
        assert_eq!(std::mem::size_of_val(&factory), 0);
    }

    #[test]
    fn test_unknown_provider_returns_error() {
        let factory = LlmProviderFactory::new();
        let result = factory.create("bogus_provider");
        assert!(result.is_err());
        if let Err(ProviderFactoryError::UnknownProvider(name)) = result {
            assert_eq!(name, "bogus_provider");
        } else {
            panic!("Expected UnknownProvider error");
        }
    }

    #[test]
    fn test_list_available_providers_returns_vec() {
        // Smoke test — just ensure it returns without panicking.
        let _ = LlmProviderFactory::list_available_providers();
    }

    #[test]
    fn provider_names_has_no_duplicate_entries() {
        let names = provider_names();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            names.len(),
            sorted.len(),
            "duplicate provider name in registry: {:?}",
            names
        );
    }

    /// The second of Task 3's two hygiene assertions (17-04): every
    /// registered provider name is an operator-facing configuration value
    /// (typed into an env var or a config file), so it must be lowercase
    /// and free of whitespace regardless of which features are compiled in.
    #[test]
    fn provider_names_are_lowercase_and_whitespace_free() {
        for name in provider_names() {
            assert_eq!(
                name,
                name.to_lowercase(),
                "provider name {name:?} must be lowercase"
            );
            assert!(
                !name.chars().any(|c| c.is_whitespace()),
                "provider name {name:?} must contain no whitespace"
            );
        }
    }

    #[test]
    fn list_available_providers_only_contains_names_from_the_registry() {
        let registry_names: std::collections::HashSet<&str> =
            provider_names().into_iter().collect();
        for name in LlmProviderFactory::list_available_providers() {
            assert!(
                registry_names.contains(name.as_str()),
                "list_available_providers returned {name}, which is not in the compiled-in \
                 registry {registry_names:?} — a compiled-out provider must never be reported \
                 as available"
            );
        }
    }

    // ── WR-01: config-layer and factory-layer name acceptance agree ──
    //
    // `LlmConfig::get_provider_config` (config/llm.rs:153-166) matches
    // `"openai-compatible" | "openai_compatible"` case-insensitively, and
    // `is_recognised_provider_field_name` (config/llm.rs:174-193) accepts
    // both spellings for `default_provider` too — so a config file naming
    // the underscore spelling validates. These four tests pin that
    // `create()` agrees: a name the config layer blesses must be a name
    // this factory resolves (WR-01).

    /// `create("openai_compatible")` must not return `UnknownProvider` — it
    /// may return `ConfigurationMissing` if the three `OPENAI_COMPATIBLE_*`
    /// env vars are unset (an acceptable outcome this assertion allows),
    /// but never `UnknownProvider`, because `config/llm.rs:153-166`'s
    /// `get_provider_config` already accepts this exact spelling.
    #[cfg(feature = "openai-compatible")]
    #[test]
    fn create_accepts_the_underscore_spelling_of_openai_compatible() {
        let factory = LlmProviderFactory::new();
        let result = factory.create("openai_compatible");
        assert!(
            !matches!(result, Err(ProviderFactoryError::UnknownProvider(_))),
            "openai_compatible must resolve through create() because config/llm.rs's \
             get_provider_config already accepts this spelling; got {:?}",
            result.as_ref().err()
        );
    }

    /// Case-insensitivity composes with separator normalisation:
    /// `"OpenAI_Compatible"` must resolve exactly like the lowercase
    /// underscore spelling.
    #[cfg(feature = "openai-compatible")]
    #[test]
    fn create_accepts_the_underscore_spelling_case_insensitively() {
        let factory = LlmProviderFactory::new();
        let result = factory.create("OpenAI_Compatible");
        assert!(
            !matches!(result, Err(ProviderFactoryError::UnknownProvider(_))),
            "OpenAI_Compatible must resolve through create() case-insensitively; got {:?}",
            result.as_ref().err()
        );
    }

    /// Non-merge control: separator normalisation must merge spellings of
    /// the *same* row, never widen `"openai"` into the `"openai-compatible"`
    /// row. Guards against a normalisation broader than one separator
    /// substitution (T-17-63).
    #[cfg(feature = "openai")]
    #[test]
    fn create_does_not_absorb_openai_into_openai_compatible() {
        let factory = LlmProviderFactory::new();
        let result = factory.create("openai");
        assert!(
            !matches!(result, Err(ProviderFactoryError::UnknownProvider(_))),
            "openai must still resolve to its own row; got {:?}",
            result.as_ref().err()
        );
        if let Err(ProviderFactoryError::ConfigurationMissing(msg)) = &result {
            assert!(
                msg.contains("OPENAI_API_KEY"),
                "openai's ConfigurationMissing message must name OPENAI_API_KEY: {msg}"
            );
            assert!(
                !msg.contains("OPENAI_COMPATIBLE_API_KEY"),
                "openai's ConfigurationMissing message must not name the generic provider's \
                 OPENAI_COMPATIBLE_API_KEY — that would mean the normalisation resolved \
                 \"openai\" to the openai-compatible row: {msg}"
            );
        }
    }

    /// Every provider-field-name spelling `LlmConfig`'s recogniser blesses
    /// (`is_recognised_provider_field_name`, config/llm.rs:179-193) that is
    /// also compiled into this build must resolve through `create()`. This
    /// is the property WR-01 closes, checked directly rather than pinned to
    /// one literal: a future divergence between the two hand-maintained
    /// lists is exactly this defect class.
    ///
    /// `CONFIG_RECOGNISED_SPELLINGS` must track `config/llm.rs`'s
    /// recogniser arm list; a divergence between the two is the defect
    /// class WR-01 belongs to.
    #[test]
    fn every_config_recognised_and_compiled_provider_name_resolves_through_create() {
        const CONFIG_RECOGNISED_SPELLINGS: [&str; 10] = [
            "openai",
            "deepseek",
            "anthropic",
            "kimi",
            "qwen",
            "grok",
            "ollama",
            "gemini",
            "openai-compatible",
            "openai_compatible",
        ];

        let factory = LlmProviderFactory::new();
        let compiled: Vec<String> = provider_names()
            .into_iter()
            .map(|n| n.to_lowercase().replace('_', "-"))
            .collect();

        let mut exercised = 0usize;
        for &name in &CONFIG_RECOGNISED_SPELLINGS {
            let normalized = name.to_lowercase().replace('_', "-");
            if !compiled.contains(&normalized) {
                eprintln!(
                    "skipping {name:?} — its provider feature is not compiled into this build"
                );
                continue;
            }

            let result = factory.create(name);
            exercised += 1;
            assert!(
                !matches!(result, Err(ProviderFactoryError::UnknownProvider(_))),
                "{name:?} is recognised by LlmConfig's config-layer recogniser and is compiled \
                 into this build, so create() must not return UnknownProvider for it; got {:?}",
                result.as_ref().err()
            );
        }

        assert!(
            exercised > 0,
            "no compiled-in spelling was exercised — this test would pass vacuously"
        );
    }

    // ── D-10 regression coverage: kimi-only build ──
    //
    // Exercised under `cargo test -p paladin-llm --no-default-features
    // --features kimi provider_factory`. Gated so it does not run (and
    // therefore does not assert an exact single-entry list) under the
    // default `openai` + `mock` feature set.
    #[cfg(all(
        feature = "kimi",
        not(feature = "openai"),
        not(feature = "anthropic"),
        not(feature = "deepseek"),
        not(feature = "qwen"),
        not(feature = "grok"),
        not(feature = "ollama")
    ))]
    mod kimi_only_build {
        use super::*;

        #[test]
        fn provider_names_returns_exactly_kimi() {
            assert_eq!(provider_names(), vec!["kimi"]);
        }

        #[test]
        fn create_bogus_provider_error_message_lists_kimi() {
            let factory = LlmProviderFactory::new();
            let result = factory.create("bogus_provider");
            match result {
                Err(ProviderFactoryError::UnknownProvider(name)) => {
                    assert_eq!(name, "bogus_provider");
                }
                Err(other) => panic!("expected UnknownProvider, got Err({other})"),
                Ok(_) => panic!("expected UnknownProvider, got Ok(_)"),
            }

            let message =
                ProviderFactoryError::UnknownProvider("bogus_provider".to_string()).to_string();
            assert!(
                message.contains("kimi"),
                "UnknownProvider message must list the compiled-in providers: {message}"
            );
        }

        #[test]
        fn create_normalizes_case_for_a_registered_provider_name() {
            let factory = LlmProviderFactory::new();
            let upper = factory.create("KIMI");
            let lower = factory.create("kimi");

            // Neither call should fail with UnknownProvider — the row is
            // found regardless of case. (It may still fail with
            // ConfigurationMissing if MOONSHOT_API_KEY is unset in this
            // test process, which is expected and fine here.)
            assert!(!matches!(
                upper,
                Err(ProviderFactoryError::UnknownProvider(_))
            ));
            assert!(!matches!(
                lower,
                Err(ProviderFactoryError::UnknownProvider(_))
            ));
        }

        #[test]
        fn build_with_zero_providers_is_not_exercised_here() {
            // Documents intent: the zero-provider empty-table case
            // (`cargo build -p paladin-llm --no-default-features`, no
            // provider feature at all) is covered by the acceptance
            // criterion's build command, not by an in-crate test — there
            // is no way to compile this test module without at least one
            // provider feature enabled.
        }
    }

    // ── D-12 regression coverage: Ollama resolves with no credential ──
    //
    // The first (and only) registry row whose `env_var` is `None`.
    // `get_default_provider()`/`list_available_providers()` must treat a
    // `None` credential as "available" without any environment variable
    // being set.
    #[cfg(feature = "ollama")]
    mod ollama_requires_no_credential {
        use super::*;

        #[test]
        fn create_resolves_with_no_credential_in_environment() {
            let factory = LlmProviderFactory::new();
            let result = factory.create("ollama");
            assert!(
                result.is_ok(),
                "ollama must resolve with no credential env var set: {:?}",
                result.err()
            );
        }

        #[test]
        fn list_available_providers_includes_ollama_whenever_compiled_in() {
            assert!(
                LlmProviderFactory::list_available_providers()
                    .iter()
                    .any(|name| name == "ollama"),
                "a compiled-in, credential-free ollama row must always report as available"
            );
        }
    }

    // ── D-10 regression coverage: the four-new-preset build (17-03) ──
    //
    // Exercised under `cargo test -p paladin-llm --no-default-features
    // --features kimi,qwen,grok,ollama`. Proves table declaration order is
    // preserved end to end and that the credential-free `ollama` row lands
    // last, never pre-empting a credentialed row.
    //
    // Gate widened in plan 17-04 (Rule 1 auto-fix, mirroring plan 17-03's
    // own precedent for this exact gate): `not(feature = "openai-compatible")`
    // added so this module's exact five-row-free assertion does not silently
    // break under the plan's own combined verification command
    // (`--features kimi,qwen,grok,ollama,openai-compatible`), which now adds
    // a fifth row. See `five_new_preset_build` below for that combined case.
    #[cfg(all(
        feature = "kimi",
        feature = "qwen",
        feature = "grok",
        feature = "ollama",
        not(feature = "openai"),
        not(feature = "anthropic"),
        not(feature = "deepseek"),
        not(feature = "openai-compatible")
    ))]
    mod four_new_preset_build {
        use super::*;

        #[test]
        fn provider_names_returns_exactly_kimi_qwen_grok_ollama_in_table_order() {
            assert_eq!(provider_names(), vec!["kimi", "qwen", "grok", "ollama"]);
        }
    }

    // ── D-10 regression coverage: the five-new-preset build (17-04) ──
    //
    // Exercised under `cargo test -p paladin-llm --no-default-features
    // --features kimi,qwen,grok,ollama,openai-compatible`. Proves the
    // generic `openai-compatible` row lands after every curated preset but
    // BEFORE the credential-free `ollama` row — see the placement comment on
    // `build_provider_registry` for why that specific position is required
    // for `get_default_provider()` to ever be able to select it.
    //
    // Gate widened in plan 17-05 (Rule 1 auto-fix, mirroring plan 17-04's own
    // precedent for this exact gate): `not(feature = "gemini")` added so this
    // module's exact five-row assertion does not silently break under plan
    // 17-05's own combined verification command (`--features
    // kimi,qwen,grok,ollama,openai-compatible,gemini`), which now adds a
    // sixth row. See `six_new_preset_build` below for that combined case.
    #[cfg(all(
        feature = "kimi",
        feature = "qwen",
        feature = "grok",
        feature = "ollama",
        feature = "openai-compatible",
        not(feature = "openai"),
        not(feature = "anthropic"),
        not(feature = "deepseek"),
        not(feature = "gemini")
    ))]
    mod five_new_preset_build {
        use super::*;

        #[test]
        fn provider_names_returns_exactly_kimi_qwen_grok_openai_compatible_ollama_in_table_order() {
            assert_eq!(
                provider_names(),
                vec!["kimi", "qwen", "grok", "openai-compatible", "ollama"]
            );
        }
    }

    // ── D-10 regression coverage: the six-preset build (17-05) ──
    //
    // Exercised under `cargo test -p paladin-llm --no-default-features
    // --features kimi,qwen,grok,ollama,openai-compatible,gemini` — plan
    // 17-05's own combined verification command. Proves Gemini's row lands
    // alongside the other curated (named-vendor) presets, before the
    // generic `openai-compatible` row and Ollama's credential-free row —
    // see the placement comment on `build_provider_registry` for why.
    #[cfg(all(
        feature = "kimi",
        feature = "qwen",
        feature = "grok",
        feature = "ollama",
        feature = "openai-compatible",
        feature = "gemini",
        not(feature = "openai"),
        not(feature = "anthropic"),
        not(feature = "deepseek")
    ))]
    mod six_preset_build {
        use super::*;

        #[test]
        fn provider_names_returns_exactly_kimi_qwen_grok_gemini_openai_compatible_ollama_in_table_order()
         {
            assert_eq!(
                provider_names(),
                vec![
                    "kimi",
                    "qwen",
                    "grok",
                    "gemini",
                    "openai-compatible",
                    "ollama"
                ]
            );
        }
    }

    // ── The registry-wide provider-name round-trip invariant (Task 3, 17-04) ──
    //
    // Companion test to this plan's `<assumption_delta_decision>`: every
    // adapter reachable through the registry must resolve to its own
    // registered name, for every compiled-in provider. It goes red the
    // moment a future phase reintroduces the singular
    // one-adapter-type-one-name assumption — most likely by adding a second
    // generic row that reuses the `"openai-compatible"` literal under a
    // different table name.
    //
    // Gated on `feature = "ollama"` specifically: Ollama's registry row
    // requires no credential (`env_var: None`) and is therefore guaranteed
    // to construct regardless of which secrets happen to be present in the
    // test process. That guarantee is what makes the "at least one row was
    // exercised" assertion below safe to run in an environment with zero
    // configured credentials — without it, a CI run with no API keys set
    // could exercise zero rows and pass vacuously, which is the exact
    // failure mode this test exists to prevent. Exercised in this plan's own
    // verification command: `cargo test -p paladin-llm --no-default-features
    // --features ollama,openai-compatible provider_name_round_trips`.
    #[cfg(feature = "ollama")]
    mod provider_name_round_trip {
        use super::*;

        #[test]
        fn provider_name_round_trips_for_every_registry_row() {
            let mut exercised = 0usize;

            for row in provider_registry() {
                // Uses the same non-blank check `get_default_provider()` and
                // `list_available_providers()` use (CR-01): an env var set
                // to an empty (or whitespace-only) string is *set* but is
                // not a usable credential — every preset's own `validate()`
                // rejects an empty key. Treating it as "present" here would
                // panic on `construct()`'s resulting `AdapterCreationFailed`,
                // which is a false alarm about this test's own invariant
                // (every adapter that *does* construct round-trips its
                // name), not a real violation of it.
                let credential_present = match row.env_var {
                    Some(var) => std::env::var(var).is_ok_and(|v| !v.trim().is_empty()),
                    None => true,
                };

                if !credential_present {
                    eprintln!(
                        "skipping provider_name round-trip for {:?} — its credential env var \
                         is not set (or is empty) in this test process",
                        row.name
                    );
                    continue;
                }

                match (row.construct_from_config)(None, None, &process_env) {
                    Ok(adapter) => {
                        exercised += 1;
                        assert_eq!(
                            adapter.get_provider_name(),
                            row.name,
                            "row {:?} constructed an adapter whose get_provider_name() did not \
                             round-trip to its own registered name",
                            row.name
                        );
                    }
                    Err(e) => {
                        panic!(
                            "row {:?} reported its credential as present but construct() failed: {e}",
                            row.name
                        );
                    }
                }
            }

            assert!(
                exercised > 0,
                "no registry row was exercised — this module is gated on `feature = \"ollama\"` \
                 specifically so its credential-free row guarantees at least one construction \
                 regardless of environment; zero exercised rows means that guarantee broke"
            );
        }
    }

    /// `create()` is safe to call concurrently: N concurrent calls each
    /// resolve the registry independently and return distinct results with
    /// no shared mutable state. Exercised against an unknown provider name
    /// rather than a credentialed one — mutating `MOONSHOT_API_KEY` via
    /// `std::env::set_var` is `unsafe` under Rust 2024 and this crate
    /// denies `unsafe_code`; the credentialed 20-concurrent-`create("kimi")`
    /// variant is deferred to the workspace-level test target (plan 17-07
    /// owns env-var-dependent tests). This test still proves the concurrency
    /// property D-10 depends on: the lazily-initialized `OnceLock` registry
    /// resolves safely under concurrent readers.
    #[tokio::test]
    async fn create_is_safe_to_call_concurrently() {
        let factory = Arc::new(LlmProviderFactory::new());
        let mut handles = Vec::new();

        for _ in 0..20 {
            let factory = Arc::clone(&factory);
            handles.push(tokio::spawn(async move {
                factory.create("bogus-concurrent-provider")
            }));
        }

        for handle in handles {
            let result = handle.await.expect("task must not panic");
            assert!(matches!(
                result,
                Err(ProviderFactoryError::UnknownProvider(_))
            ));
        }
    }

    // ── Plan 50-09: create_from_config, is_configured and the layered create ──

    /// Config-driven construction against mock vendors. Every environment lookup is a map: no
    /// test here reads or writes the process environment.
    mod from_config {
        use super::*;
        use crate::config::llm::{LlmConfig, LlmProviderConfig};
        use crate::registry::ModelOverride;
        use crate::test_support::server;
        use mockito::Matcher;
        use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
        use paladin_ports::output::llm_port::{LlmError, LlmRequest, LlmResponse};
        use serde_json::Value;
        // Every user of `json!` here builds a provider's wire body, so it needs a provider.
        #[cfg(any(
            feature = "openai",
            feature = "anthropic",
            feature = "deepseek",
            feature = "kimi",
            feature = "qwen",
            feature = "grok",
            feature = "ollama",
            feature = "openai-compatible",
            feature = "gemini"
        ))]
        use serde_json::json;
        use std::collections::BTreeMap;

        // Only the tests behind the `kimi` feature call it, so a build without that feature
        // compiles it unused.
        #[allow(dead_code)]
        fn env_of<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
            move |name| {
                pairs
                    .iter()
                    .find(|(k, _)| *k == name)
                    .map(|(_, v)| (*v).to_string())
            }
        }

        fn no_env(_: &str) -> Option<String> {
            None
        }

        /// A `LlmConfig` carrying `block` as the block for `provider`.
        fn config_with(provider: &str, block: LlmProviderConfig) -> LlmConfig {
            let mut config = LlmConfig {
                default_provider: None,
                ..LlmConfig::default()
            };
            match provider {
                "openai" => config.openai = Some(block),
                "deepseek" => config.deepseek = Some(block),
                "anthropic" => config.anthropic = Some(block),
                "kimi" => config.kimi = Some(block),
                "qwen" => config.qwen = Some(block),
                "grok" => config.grok = Some(block),
                "ollama" => config.ollama = Some(block),
                "gemini" => config.gemini = Some(block),
                "openai-compatible" => config.openai_compatible = Some(block),
                other => panic!("no block field for provider {other}"),
            }
            config
        }

        /// One compiled provider's wire: where it answers and what a success looks like.
        struct WireCase {
            provider: &'static str,
            model: &'static str,
            path: String,
            response: Value,
            keyed: bool,
        }

        // Shared by the chat-completions providers' cases and Kimi's end-to-end tests.
        #[cfg(any(
            feature = "openai",
            feature = "deepseek",
            feature = "kimi",
            feature = "qwen",
            feature = "grok",
            feature = "ollama",
            feature = "openai-compatible"
        ))]
        fn chat_response() -> Value {
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
        }

        /// A case per provider compiled into this build. Unseeded model ids, so the fallback
        /// profile applies and the request carries exactly what the test sets.
        #[allow(unused_mut, clippy::vec_init_then_push)]
        fn wire_cases() -> Vec<WireCase> {
            let mut cases: Vec<WireCase> = Vec::new();
            // Used by the chat-completions providers' cases only.
            #[cfg(any(
                feature = "openai",
                feature = "deepseek",
                feature = "kimi",
                feature = "qwen",
                feature = "grok",
                feature = "ollama",
                feature = "openai-compatible"
            ))]
            let chat = |provider: &'static str, model: &'static str, keyed: bool| WireCase {
                provider,
                model,
                path: "/chat/completions".to_string(),
                response: chat_response(),
                keyed,
            };
            #[cfg(feature = "openai")]
            cases.push(chat("openai", "gpt-block-model", true));
            #[cfg(feature = "deepseek")]
            cases.push(chat("deepseek", "deepseek-block-model", true));
            #[cfg(feature = "kimi")]
            cases.push(chat("kimi", "kimi-block-model", true));
            #[cfg(feature = "qwen")]
            cases.push(chat("qwen", "qwen-block-model", true));
            #[cfg(feature = "grok")]
            cases.push(chat("grok", "grok-block-model", true));
            #[cfg(feature = "ollama")]
            cases.push(chat("ollama", "ollama-block-model", false));
            #[cfg(feature = "openai-compatible")]
            cases.push(chat("openai-compatible", "compat-block-model", true));
            #[cfg(feature = "anthropic")]
            cases.push(WireCase {
                provider: "anthropic",
                model: "claude-block-model",
                path: "/messages".to_string(),
                response: json!({
                    "id": "msg_1",
                    "type": "message",
                    "role": "assistant",
                    "model": "test-model",
                    "content": [{"type": "text", "text": "ok"}],
                    "stop_reason": "end_turn",
                    "stop_sequence": null,
                    "usage": {"input_tokens": 1, "output_tokens": 1}
                }),
                keyed: true,
            });
            #[cfg(feature = "gemini")]
            cases.push(WireCase {
                provider: "gemini",
                model: "gemini-block-model",
                path: "/models/gemini-block-model:generateContent".to_string(),
                response: json!({
                    "candidates": [{
                        "content": {"role": "model", "parts": [{"text": "ok"}]},
                        "finishReason": "STOP"
                    }],
                    "usageMetadata": {
                        "promptTokenCount": 1,
                        "candidatesTokenCount": 1,
                        "totalTokenCount": 2
                    }
                }),
                keyed: true,
            });
            cases
        }

        /// The block a deployment would write for `case`, pointed at `base_url`.
        fn block_for(case: &WireCase, base_url: &str) -> LlmProviderConfig {
            let key = if case.keyed { "block-test-key" } else { "" };
            LlmProviderConfig::new(key)
                .with_base_url(base_url)
                .with_default_model(case.model)
                .with_timeout_seconds(5)
                .with_max_retries(0)
        }

        async fn generate_once(port: &dyn LlmPort, model: &str) -> Result<LlmResponse, LlmError> {
            let request = LlmRequest::new(
                model,
                PromptItem::new(PromptType::User(UserPrompt {
                    query: "Hello".to_string(),
                    context: None,
                }))
                .unwrap(),
            );
            port.generate(request).await
        }

        // ── each provider, from its block ───────────────────────────────────

        #[tokio::test]
        async fn create_from_config_builds_each_provider_from_its_block() {
            let cases = wire_cases();
            assert!(
                !cases.is_empty(),
                "this build compiles at least one provider"
            );
            for case in cases {
                let mut vendor = server().await;
                let captured = crate::test_support::mock_capture(
                    &mut vendor,
                    &case.path,
                    case.response.clone(),
                )
                .await;
                let block = block_for(&case, &vendor.url());
                let config = config_with(case.provider, block);
                let port = LlmProviderFactory::new()
                    .create_from_config_with_env(&config, case.provider, None, &no_env)
                    .unwrap_or_else(|e| panic!("{}: {e}", case.provider));
                assert_eq!(port.get_provider_name(), case.provider);
                let response = generate_once(port.as_ref(), case.model)
                    .await
                    .unwrap_or_else(|e| panic!("{}: {e}", case.provider));
                assert_eq!(response.content, "ok", "{}", case.provider);
                let body =
                    captured.lock().unwrap().take().unwrap_or_else(|| {
                        panic!("{}: the mock received no request", case.provider)
                    });
                if case.provider == "gemini" {
                    // Gemini carries the model in the URL path the mock matched.
                    assert!(body.get("contents").is_some(), "{body}");
                } else {
                    assert_eq!(body["model"], case.model, "{}: {body}", case.provider);
                }
            }
        }

        #[tokio::test]
        async fn the_public_create_from_config_reads_the_same_blocks() {
            // Ollama needs no key and no environment, so the real-environment entry point can be
            // exercised without touching any variable.
            #[cfg(feature = "ollama")]
            {
                let block = LlmProviderConfig::new("")
                    .with_base_url("http://127.0.0.1:1/v1")
                    .with_default_model("ollama-block-model");
                let config = config_with("ollama", block);
                let port = LlmProviderFactory::new()
                    .create_from_config(&config, "ollama", None)
                    .expect("a keyless block builds");
                assert_eq!(port.get_provider_name(), "ollama");
            }
        }

        // ── D-13: the block beats the environment, end to end ───────────────

        #[cfg(feature = "kimi")]
        #[tokio::test]
        async fn env_never_overrides_the_block_end_to_end() {
            let mut vendor = server().await;
            let mock = vendor
                .mock("POST", "/chat/completions")
                .match_header("authorization", "Bearer block-key")
                .with_status(200)
                .with_body(chat_response().to_string())
                .expect(1)
                .create_async()
                .await;
            let block = LlmProviderConfig::new("block-key")
                .with_base_url(vendor.url())
                .with_max_retries(0);
            let config = config_with("kimi", block);
            // The environment names a different key and a dead endpoint: neither may win.
            let env = env_of(&[
                ("MOONSHOT_API_KEY", "env-key"),
                ("MOONSHOT_BASE_URL", "http://127.0.0.1:1/v1"),
            ]);
            let port = LlmProviderFactory::new()
                .create_from_config_with_env(&config, "kimi", None, &env)
                .unwrap();
            generate_once(port.as_ref(), "kimi-block-model")
                .await
                .unwrap();
            mock.assert_async().await;
        }

        #[cfg(feature = "kimi")]
        #[tokio::test]
        async fn a_placeholder_key_is_resolved_and_never_sent_literally() {
            let mut vendor = server().await;
            let mock = vendor
                .mock("POST", "/chat/completions")
                .match_header("authorization", "Bearer sk-from-the-variable")
                .with_status(200)
                .with_body(chat_response().to_string())
                .expect(1)
                .create_async()
                .await;
            let block = LlmProviderConfig::new("${MOONSHOT_API_KEY}")
                .with_base_url(vendor.url())
                .with_max_retries(0);
            let config = config_with("kimi", block);
            let env = env_of(&[("MOONSHOT_API_KEY", "sk-from-the-variable")]);
            let port = LlmProviderFactory::new()
                .create_from_config_with_env(&config, "kimi", None, &env)
                .unwrap();
            generate_once(port.as_ref(), "kimi-block-model")
                .await
                .unwrap();
            mock.assert_async().await;

            // With the variable unset nothing is built, so nothing can reach a vendor.
            let Err(error) = LlmProviderFactory::new().create_from_config_with_env(
                &config,
                "kimi",
                Some("kimi-k3"),
                &no_env,
            ) else {
                panic!("an unset placeholder variable must be a construction error");
            };
            let message = error.to_string();
            assert!(
                message.contains("provider 'kimi' (model 'kimi-k3')"),
                "{message}"
            );
            assert!(message.contains("MOONSHOT_API_KEY"), "{message}");
        }

        // ── model overrides ─────────────────────────────────────────────────

        fn window_override(window: u32) -> ModelOverride {
            ModelOverride {
                context_window: Some(window),
                ..ModelOverride::default()
            }
        }

        #[cfg(feature = "kimi")]
        #[test]
        fn create_from_config_honours_model_overrides() {
            let factory = LlmProviderFactory::new();
            let build = |block: LlmProviderConfig| {
                factory
                    .create_from_config_with_env(&config_with("kimi", block), "kimi", None, &no_env)
                    .expect("the block builds")
            };
            let window = |port: &Arc<dyn LlmPort>, model: &str| {
                port.model_profile(model)
                    .and_then(|profile| profile.context_window)
            };

            // A per-model override is what that model's profile reports.
            let port = build(LlmProviderConfig::new("k").with_models(BTreeMap::from([(
                "kimi-never-seen".to_string(),
                window_override(700_000),
            )])));
            assert_eq!(window(&port, "kimi-never-seen"), Some(700_000));

            // A block `context_window` with no per-model override beats the seeded window.
            let port = build(LlmProviderConfig::new("k").with_context_window(500_000));
            assert_eq!(
                window(&port, "kimi-k3"),
                Some(500_000),
                "beats the seeded window"
            );
            assert_eq!(window(&port, "kimi-never-seen"), Some(500_000));

            // `models.<id>.context_window` beats the block value, for that model only.
            let port = build(
                LlmProviderConfig::new("k")
                    .with_context_window(500_000)
                    .with_models(BTreeMap::from([(
                        "kimi-k3".to_string(),
                        window_override(700_000),
                    )])),
            );
            assert_eq!(window(&port, "kimi-k3"), Some(700_000));
            assert_eq!(window(&port, "kimi-k2.6"), Some(500_000));
        }

        #[cfg(feature = "anthropic")]
        #[test]
        fn a_bespoke_adapter_honours_model_overrides_too() {
            let block = LlmProviderConfig::new("k")
                .with_context_window(400_000)
                .with_models(BTreeMap::from([(
                    "claude-never-seen".to_string(),
                    window_override(650_000),
                )]));
            let port = LlmProviderFactory::new()
                .create_from_config_with_env(
                    &config_with("anthropic", block),
                    "anthropic",
                    None,
                    &no_env,
                )
                .unwrap();
            let window = |model: &str| port.model_profile(model).and_then(|p| p.context_window);
            assert_eq!(window("claude-never-seen"), Some(650_000));
            assert_eq!(window("claude-haiku-5-5"), Some(400_000));
        }

        #[cfg(feature = "kimi")]
        #[test]
        fn an_invalid_override_fails_construction_naming_provider_and_model() {
            let block = LlmProviderConfig::new("k").with_models(BTreeMap::from([(
                "kimi-bad".to_string(),
                ModelOverride {
                    context_window: Some(0),
                    ..ModelOverride::default()
                },
            )]));
            let Err(error) = LlmProviderFactory::new().create_from_config_with_env(
                &config_with("kimi", block),
                "kimi",
                None,
                &no_env,
            ) else {
                panic!("a zero window must fail construction");
            };
            assert!(
                matches!(error, ProviderFactoryError::AdapterCreationFailed(_)),
                "{error}"
            );
            assert!(error.to_string().contains("provider 'kimi'"), "{error}");
        }

        // ── errors name the provider and the model, never a key ─────────────

        #[test]
        fn create_from_config_names_provider_and_model_on_error() {
            let factory = LlmProviderFactory::new();
            // Unknown provider, with and without a model.
            let message = factory
                .create_from_config_with_env(&LlmConfig::default(), "bogus", Some("m-1"), &no_env)
                .err()
                .expect("unknown provider")
                .to_string();
            assert!(message.contains("bogus (model 'm-1')"), "{message}");
            assert!(message.contains("Unknown provider"), "{message}");

            // Every compiled provider with a key to find: no block and no variable, an unset
            // placeholder and a malformed model each name provider and model.
            let secret_block = LlmProviderConfig::new("sk-super-secret-key-value");
            for name in provider_names() {
                let model = "named-model";
                let expected = format!("provider '{name}' (model '{model}')");

                let bare = factory.create_from_config_with_env(
                    &LlmConfig::default(),
                    name,
                    Some(model),
                    &no_env,
                );
                match bare {
                    // Ollama needs no credential, so it builds from nothing (D-12).
                    Ok(_) => assert_eq!(name, "ollama"),
                    Err(error) => {
                        let message = error.to_string();
                        assert!(message.contains(&expected), "{message}");
                    }
                }

                let placeholder = config_with(name, LlmProviderConfig::new("${UNSET_THING}"));
                if name != "ollama" {
                    let message = factory
                        .create_from_config_with_env(&placeholder, name, Some(model), &no_env)
                        .err()
                        .expect("an unset placeholder variable is an error")
                        .to_string();
                    assert!(message.contains(&expected), "{message}");
                    assert!(message.contains("UNSET_THING"), "{message}");
                }

                let config = config_with(name, secret_block.clone());
                for bad_model in ["has space", "../x", "a?b", "a#b", "a%b", "/lead", ""] {
                    let message = factory
                        .create_from_config_with_env(&config, name, Some(bad_model), &no_env)
                        .err()
                        .unwrap_or_else(|| panic!("{name}: {bad_model:?} must be refused"))
                        .to_string();
                    assert!(message.contains(&format!("provider '{name}'")), "{message}");
                    assert!(
                        !message.contains("sk-super-secret-key-value"),
                        "no error may carry the key: {message}"
                    );
                }
                let too_long = "m".repeat(257);
                assert!(
                    factory
                        .create_from_config_with_env(&config, name, Some(&too_long), &no_env)
                        .is_err(),
                    "{name}: a 257-character model id must be refused"
                );
            }
        }

        #[cfg(feature = "kimi")]
        #[test]
        fn a_malformed_model_is_refused_before_any_adapter_is_built() {
            // The block would build; only the model is wrong, and nothing was constructed.
            let config = config_with("kimi", LlmProviderConfig::new("sk-k"));
            for bad in [
                "kimi k3", "kimi\tk3", "kimi\nk3", "..", "kimi?x", "/kimi", "",
            ] {
                let result = LlmProviderFactory::new().create_from_config_with_env(
                    &config,
                    "kimi",
                    Some(bad),
                    &no_env,
                );
                assert!(
                    matches!(result, Err(ProviderFactoryError::AdapterCreationFailed(_))),
                    "{bad:?} must be refused"
                );
            }
            assert!(
                LlmProviderFactory::new()
                    .create_from_config_with_env(&config, "kimi", Some("kimi-k3"), &no_env)
                    .is_ok()
            );
        }

        #[cfg(feature = "kimi")]
        #[test]
        fn a_per_agent_model_equal_to_the_block_default_builds_an_identical_adapter() {
            let block = LlmProviderConfig::new("k")
                .with_default_model("kimi-k3")
                .with_context_window(400_000);
            let config = config_with("kimi", block);
            let factory = LlmProviderFactory::new();
            let by_agent = factory
                .create_from_config_with_env(&config, "kimi", Some("kimi-k3"), &no_env)
                .unwrap();
            let by_block = factory
                .create_from_config_with_env(&config, "kimi", None, &no_env)
                .unwrap();
            assert_eq!(
                format!("{:?}", by_agent.model_profile("kimi-k3")),
                format!("{:?}", by_block.model_profile("kimi-k3"))
            );
            assert_eq!(by_agent.get_capabilities(), by_block.get_capabilities());
        }

        // ── redirects ───────────────────────────────────────────────────────

        /// T-50-09-01: a `3xx` from a configured `base_url` is not followed, so the credential
        /// header never reaches the redirect target. Proven on an adapter the factory built, for
        /// every provider compiled into this build.
        #[tokio::test]
        async fn factory_built_adapter_refuses_redirects() {
            for case in wire_cases() {
                // The target must see no request of either method: a 302 turns a followed POST
                // into a GET, a 307 keeps the POST and its headers.
                let mut target = server().await;
                let target_post = target
                    .mock("POST", Matcher::Any)
                    .expect(0)
                    .create_async()
                    .await;
                let target_get = target
                    .mock("GET", Matcher::Any)
                    .expect(0)
                    .create_async()
                    .await;
                let mut vendor = server().await;
                vendor
                    .mock("POST", Matcher::Any)
                    .with_status(307)
                    .with_header("location", &format!("{}/chat/completions", target.url()))
                    .with_body("redirecting")
                    .expect_at_least(1)
                    .create_async()
                    .await;
                let block = block_for(&case, &vendor.url());
                let port = LlmProviderFactory::new()
                    .create_from_config_with_env(
                        &config_with(case.provider, block),
                        case.provider,
                        None,
                        &no_env,
                    )
                    .unwrap_or_else(|e| panic!("{}: {e}", case.provider));
                let result = generate_once(port.as_ref(), case.model).await;
                assert!(
                    result.is_err(),
                    "{}: a refused redirect must surface as an error",
                    case.provider
                );
                target_post.assert_async().await;
                target_get.assert_async().await;
            }
        }

        // ── concurrency ─────────────────────────────────────────────────────

        /// `create_from_config` is a pure function of its inputs and an environment snapshot, so
        /// concurrent callers get independent adapters. This evidences, and cannot prove, the
        /// absence of shared mutable state.
        #[tokio::test]
        async fn concurrent_create_from_config_builds_independent_adapters() {
            let names = provider_names();
            assert!(!names.is_empty());
            let mut handles = Vec::new();
            for i in 0..16u32 {
                let provider = names[(i as usize) % names.len()];
                handles.push(tokio::task::spawn_blocking(move || {
                    let model = format!("concurrent-model-{i}");
                    let window = 100_000 + i;
                    let block = LlmProviderConfig::new("k")
                        .with_base_url("http://127.0.0.1:1")
                        .with_default_model(model.clone())
                        .with_models(BTreeMap::from([(model.clone(), window_override(window))]));
                    let config = config_with(provider, block);
                    let port = LlmProviderFactory::new()
                        .create_from_config_with_env(&config, provider, Some(&model), &no_env)
                        .unwrap_or_else(|e| panic!("task {i} ({provider}): {e}"));
                    (i, provider, model, window, port)
                }));
            }
            let mut built = Vec::new();
            for handle in handles {
                built.push(handle.await.expect("a task must not panic"));
            }
            for (i, provider, model, window, port) in &built {
                assert_eq!(port.get_provider_name(), *provider, "task {i}");
                let own = port.model_profile(model).and_then(|p| p.context_window);
                assert_eq!(own, Some(*window), "task {i} must report its own override");
                // Another task's model is unknown to this adapter, so its override is absent.
                for (j, _, other_model, other_window, _) in &built {
                    if j == i {
                        continue;
                    }
                    let seen = port
                        .model_profile(other_model)
                        .and_then(|p| p.context_window);
                    assert_ne!(
                        seen,
                        Some(*other_window),
                        "task {i} must not see task {j}'s override"
                    );
                }
            }
        }

        // ── availability ────────────────────────────────────────────────────

        #[test]
        fn is_configured_accepts_yaml_only_and_env_only_providers() {
            // Unknown and compiled-out names are never configured.
            assert!(!LlmProviderFactory::is_configured_with_env(
                None, "bogus", &no_env
            ));
            #[cfg(feature = "kimi")]
            {
                let is = |config: Option<&LlmConfig>, env: &[(&str, &str)]| {
                    LlmProviderFactory::is_configured_with_env(config, "kimi", &env_of(env))
                };
                // Nothing anywhere.
                assert!(!is(None, &[]));
                // Environment only.
                assert!(is(None, &[("MOONSHOT_API_KEY", "sk-env")]));
                assert!(
                    !is(None, &[("MOONSHOT_API_KEY", "  ")]),
                    "blank counts as unset"
                );
                // YAML only: a block with a literal key and no environment.
                let literal = config_with("kimi", LlmProviderConfig::new("sk-yaml"));
                assert!(is(Some(&literal), &[]));
                // An empty block key falls back to the provider variable.
                let empty = config_with("kimi", LlmProviderConfig::new(""));
                assert!(!is(Some(&empty), &[]));
                assert!(is(Some(&empty), &[("MOONSHOT_API_KEY", "sk-env")]));
                // A placeholder is usable only when its variable is set.
                let placeholder =
                    config_with("kimi", LlmProviderConfig::new("${MOONSHOT_API_KEY}"));
                assert!(is(Some(&placeholder), &[("MOONSHOT_API_KEY", "sk-env")]));
                assert!(!is(Some(&placeholder), &[]));
                // A placeholder naming a different, unset variable is not rescued by the
                // provider's own variable: construction would refuse it too.
                let other = config_with("kimi", LlmProviderConfig::new("${MY_KIMI_KEY}"));
                assert!(!is(Some(&other), &[("MOONSHOT_API_KEY", "sk-env")]));
                // A block for another provider says nothing about kimi.
                let ollama_only = config_with("ollama", LlmProviderConfig::new("x"));
                assert!(!is(Some(&ollama_only), &[]));
            }
            #[cfg(feature = "ollama")]
            {
                // Keyless: always available, with or without a block.
                assert!(LlmProviderFactory::is_configured_with_env(
                    None, "ollama", &no_env
                ));
                assert!(LlmProviderFactory::is_configured_with_env(
                    Some(&LlmConfig::default()),
                    "OLLAMA",
                    &no_env
                ));
            }
        }

        #[test]
        fn is_configured_agrees_with_create_from_config() {
            // A name `is_configured` reports is one the credential check lets through: for every
            // compiled provider, a literal-key block is configured and builds (given the base URL
            // and model the generic provider requires).
            for name in provider_names() {
                let block = LlmProviderConfig::new("sk-agree")
                    .with_base_url("http://127.0.0.1:1")
                    .with_default_model("agree-model");
                let config = config_with(name, block);
                assert!(
                    LlmProviderFactory::is_configured_with_env(Some(&config), name, &no_env),
                    "{name}"
                );
                assert!(
                    LlmProviderFactory::new()
                        .create_from_config_with_env(&config, name, None, &no_env)
                        .is_ok(),
                    "{name}"
                );
            }
        }

        // ── create(name) is the same layering with no block ─────────────────

        #[cfg(feature = "kimi")]
        #[test]
        fn create_uses_the_layering_without_a_block() {
            let row = find_row("kimi").expect("kimi is compiled in");
            // Env only: builds from the legacy variables, no block anywhere.
            let env = env_of(&[
                ("MOONSHOT_API_KEY", "sk-env"),
                ("MOONSHOT_MODEL", "kimi-env"),
            ]);
            let port = (row.construct_from_config)(None, None, &env).expect("env-only builds");
            assert_eq!(port.get_provider_name(), "kimi");
            // No env: the error names the provider and the variable, as `create` always did.
            let message = (row.construct_from_config)(None, None, &no_env)
                .err()
                .expect("no key anywhere")
                .to_string();
            assert!(message.contains("provider 'kimi'"), "{message}");
            assert!(message.contains("MOONSHOT_API_KEY"), "{message}");
            assert!(matches!(
                (row.construct_from_config)(None, None, &no_env),
                Err(ProviderFactoryError::ConfigurationMissing(_))
            ));
        }

        #[cfg(feature = "openai")]
        #[test]
        fn create_keeps_naming_openai_api_key_for_an_openai_with_no_credential() {
            let row = find_row("openai").expect("openai is compiled in");
            let message = (row.construct_from_config)(None, None, &no_env)
                .err()
                .expect("no key anywhere")
                .to_string();
            assert!(message.contains("OPENAI_API_KEY"), "{message}");
            assert!(!message.contains("OPENAI_COMPATIBLE_API_KEY"), "{message}");
        }

        // ── name spellings ──────────────────────────────────────────────────

        #[cfg(feature = "openai-compatible")]
        #[test]
        fn provider_name_spellings_resolve_alike() {
            let block = LlmProviderConfig::new("k")
                .with_base_url("http://127.0.0.1:1")
                .with_default_model("compat-m")
                .with_models(BTreeMap::from([(
                    "compat-m".to_string(),
                    window_override(123_456),
                )]));
            let config = config_with("openai-compatible", block);
            for spelling in [
                "openai-compatible",
                "openai_compatible",
                "OpenAI_Compatible",
            ] {
                let port = LlmProviderFactory::new()
                    .create_from_config_with_env(&config, spelling, None, &no_env)
                    .unwrap_or_else(|e| panic!("{spelling}: {e}"));
                assert_eq!(port.get_provider_name(), "openai-compatible", "{spelling}");
                // The same block was found: its override reaches the profile.
                let window = port
                    .model_profile("compat-m")
                    .and_then(|p| p.context_window);
                assert_eq!(window, Some(123_456), "{spelling}");
                assert!(
                    LlmProviderFactory::is_configured_with_env(Some(&config), spelling, &no_env),
                    "{spelling}"
                );
            }
            assert!(find_row("openai-compatible").is_some());
            assert_eq!(
                find_row("OpenAI_Compatible").map(|r| r.name),
                find_row("openai-compatible").map(|r| r.name)
            );
        }

        // ── Phase 51 (D-00e, D-10): the provider-level reasoning default, end to end ────────────
        //
        // Every case builds the adapter through `create_from_config_with_env` from a block, the
        // production path from `llm.<provider>` YAML to a constructed adapter, and reads the
        // request body a mock vendor captured.

        //
        // Compiled only when at least one provider feature is on: every case here builds a
        // provider's adapter, so a `mock`-only build has nothing to drive and would carry an
        // all-dead module.
        #[cfg(any(
            feature = "openai",
            feature = "anthropic",
            feature = "deepseek",
            feature = "kimi",
            feature = "qwen",
            feature = "grok",
            feature = "ollama",
            feature = "openai-compatible",
            feature = "gemini"
        ))]
        mod reasoning_defaults {
            use super::*;
            use crate::reasoning::{
                REASONING_EFFECTIVE_METADATA_KEY, REASONING_REQUESTED_METADATA_KEY, ReasoningWire,
            };
            use paladin_ports::output::llm_port::ReasoningEffort;
            // Named only by the DeepSeek fallback case.
            #[cfg(feature = "deepseek")]
            use paladin_ports::output::llm_port::ReasoningFallback;
            use paladin_ports::output::model_profile::{EffortLevels, EffortSupport};

            /// What one provider needs to prove its default reaches the wire.
            struct Case {
                provider: &'static str,
                /// The operator's `effort` override for the block's default model.
                support: EffortSupport,
                /// The wire a self-hosted provider declares in its block.
                wire: Option<ReasoningWire>,
                /// The provider-level default.
                default: ReasoningEffort,
                /// A different effort a request sets itself.
                other: ReasoningEffort,
                /// Where the effort lands in the request body.
                read: fn(&Value) -> Option<Value>,
                /// How an effort is spelled there.
                expect: fn(ReasoningEffort) -> Value,
            }

            /// A model no override or seed describes, so its effort is `Unmeasured`.
            const UNPROFILED: &str = "an-unprofiled-model";

            // `level_spelling` and `levels` serve every provider's case except Qwen's (its wire has
            // no level field), so they compile with any provider whose case calls them.
            #[cfg(any(
                feature = "openai",
                feature = "anthropic",
                feature = "deepseek",
                feature = "kimi",
                feature = "grok",
                feature = "ollama",
                feature = "openai-compatible",
                feature = "gemini"
            ))]
            fn level_spelling(effort: ReasoningEffort) -> Value {
                json!(effort.to_string())
            }

            #[cfg(any(
                feature = "openai",
                feature = "anthropic",
                feature = "deepseek",
                feature = "kimi",
                feature = "grok",
                feature = "ollama",
                feature = "openai-compatible",
                feature = "gemini"
            ))]
            fn levels(levels: &[ReasoningEffort]) -> EffortSupport {
                EffortSupport::Supported(EffortLevels::new(levels.iter().copied()))
            }

            /// Send one request through the adapter the block builds. `with_default` puts the
            /// case's default in the block; `effort` is the request's own.
            async fn send(
                case: &Case,
                with_default: bool,
                effort: Option<ReasoningEffort>,
                model: Option<&str>,
            ) -> (Option<Value>, Result<LlmResponse, LlmError>) {
                let wire = wire_cases()
                    .into_iter()
                    .find(|c| c.provider == case.provider)
                    .unwrap_or_else(|| panic!("{} is not compiled in", case.provider));
                let mut vendor = server().await;
                let captured = crate::test_support::mock_capture(
                    &mut vendor,
                    &wire.path,
                    wire.response.clone(),
                )
                .await;
                let mut block = block_for(&wire, &vendor.url()).with_models(BTreeMap::from([(
                    wire.model.to_string(),
                    ModelOverride {
                        effort: Some(case.support.clone()),
                        ..ModelOverride::default()
                    },
                )]));
                if with_default {
                    block = block.with_reasoning_effort(case.default);
                }
                if let Some(declared) = case.wire {
                    block = block.with_reasoning_wire(declared);
                }
                let config = config_with(case.provider, block);
                let port = LlmProviderFactory::new()
                    .create_from_config_with_env(&config, case.provider, None, &no_env)
                    .unwrap_or_else(|e| panic!("{}: {e}", case.provider));
                let mut request = LlmRequest::new(
                    model.unwrap_or(wire.model),
                    PromptItem::new(PromptType::User(UserPrompt {
                        query: "Hello".to_string(),
                        context: None,
                    }))
                    .unwrap(),
                );
                if let Some(effort) = effort {
                    request = request.with_reasoning(effort);
                }
                let result = port.generate(request).await;
                let body = captured.lock().unwrap().take();
                (body, result)
            }

            fn stamped(response: &LlmResponse) -> (Option<&str>, Option<&str>) {
                (
                    response
                        .metadata
                        .get(REASONING_REQUESTED_METADATA_KEY)
                        .map(String::as_str),
                    response
                        .metadata
                        .get(REASONING_EFFECTIVE_METADATA_KEY)
                        .map(String::as_str),
                )
            }

            /// The precedence contract of D-00e for one provider: request, then provider
            /// default, then nothing; and a refused default names the key that set it.
            async fn assert_default_applies(case: Case) {
                let name = case.provider;
                let default = case.default.to_string();
                let other = case.other.to_string();

                // 1. No effort on the request: the provider default is sent and stamped.
                let (body, result) = send(&case, true, None, None).await;
                let response = result.unwrap_or_else(|e| panic!("{name}: {e}"));
                let body = body.unwrap_or_else(|| panic!("{name}: no request reached the wire"));
                assert_eq!(
                    (case.read)(&body),
                    Some((case.expect)(case.default)),
                    "{name}: {body}"
                );
                assert_eq!(
                    stamped(&response),
                    (Some(default.as_str()), Some(default.as_str())),
                    "{name}"
                );

                // 2. A different request effort wins, and only it reaches the wire.
                let (body, result) = send(&case, true, Some(case.other), None).await;
                let response = result.unwrap_or_else(|e| panic!("{name}: {e}"));
                let body = body.unwrap();
                assert_eq!(
                    (case.read)(&body),
                    Some((case.expect)(case.other)),
                    "{name}: {body}"
                );
                assert_eq!(
                    stamped(&response),
                    (Some(other.as_str()), Some(other.as_str())),
                    "{name}"
                );

                // 3. The same level as the default: one field, requested and effective equal it.
                let (body, result) = send(&case, true, Some(case.default), None).await;
                let response = result.unwrap_or_else(|e| panic!("{name}: {e}"));
                let body = body.unwrap();
                assert_eq!(
                    (case.read)(&body),
                    Some((case.expect)(case.default)),
                    "{name}: {body}"
                );
                assert_eq!(
                    stamped(&response),
                    (Some(default.as_str()), Some(default.as_str())),
                    "{name}"
                );

                // 4. Neither set: nothing on the wire and nothing stamped (D-00i, the golden).
                let (body, result) = send(&case, false, None, None).await;
                let response = result.unwrap_or_else(|e| panic!("{name}: {e}"));
                let body = body.unwrap();
                assert_eq!((case.read)(&body), None, "{name}: {body}");
                assert_eq!(stamped(&response), (None, None), "{name}");

                // 5. The default on a model that cannot honour it refuses before HTTP, naming the
                //    key (Pitfall 6); the same model with an explicit request effort does not.
                let (body, result) = send(&case, true, None, Some(UNPROFILED)).await;
                assert!(
                    body.is_none(),
                    "{name}: a refused default must not reach the wire"
                );
                match result {
                    Err(LlmError::UnsupportedParameter {
                        reason, provider, ..
                    }) => {
                        assert_eq!(provider, name);
                        assert!(
                            reason.contains(&format!("llm.{name}.reasoning_effort")),
                            "{name}: {reason}"
                        );
                    }
                    other => panic!("{name}: expected UnsupportedParameter, got {other:?}"),
                }
                let (body, result) = send(&case, true, Some(case.other), Some(UNPROFILED)).await;
                assert!(body.is_none(), "{name}");
                match result {
                    Err(LlmError::UnsupportedParameter { reason, .. }) => assert!(
                        !reason.contains("(set by"),
                        "{name}: a request-level effort names no config key: {reason}"
                    ),
                    other => panic!("{name}: expected UnsupportedParameter, got {other:?}"),
                }
            }

            #[cfg(feature = "deepseek")]
            #[tokio::test]
            async fn deepseek_applies_the_provider_default_below_the_request() {
                assert_default_applies(Case {
                    provider: "deepseek",
                    support: levels(&[
                        ReasoningEffort::Low,
                        ReasoningEffort::High,
                        ReasoningEffort::Max,
                    ]),
                    wire: None,
                    default: ReasoningEffort::High,
                    other: ReasoningEffort::Low,
                    read: |body| body.get("reasoning_effort").cloned(),
                    expect: level_spelling,
                })
                .await;
            }

            #[cfg(feature = "openai")]
            #[tokio::test]
            async fn openai_applies_the_provider_default_below_the_request() {
                assert_default_applies(Case {
                    provider: "openai",
                    support: levels(&[ReasoningEffort::Low, ReasoningEffort::High]),
                    wire: None,
                    default: ReasoningEffort::High,
                    other: ReasoningEffort::Low,
                    read: |body| body.get("reasoning_effort").cloned(),
                    expect: level_spelling,
                })
                .await;
            }

            #[cfg(feature = "anthropic")]
            #[tokio::test]
            async fn anthropic_applies_the_provider_default_below_the_request() {
                assert_default_applies(Case {
                    provider: "anthropic",
                    support: levels(&[ReasoningEffort::Low, ReasoningEffort::High]),
                    wire: None,
                    default: ReasoningEffort::High,
                    other: ReasoningEffort::Low,
                    read: |body| body.pointer("/output_config/effort").cloned(),
                    expect: level_spelling,
                })
                .await;
            }

            #[cfg(feature = "gemini")]
            #[tokio::test]
            async fn gemini_applies_the_provider_default_below_the_request() {
                assert_default_applies(Case {
                    provider: "gemini",
                    support: levels(&[ReasoningEffort::Low, ReasoningEffort::High]),
                    wire: None,
                    default: ReasoningEffort::High,
                    other: ReasoningEffort::Low,
                    read: |body| {
                        body.pointer("/generationConfig/thinkingConfig/thinkingLevel")
                            .cloned()
                    },
                    expect: level_spelling,
                })
                .await;
            }

            // Plan 51-11 restated this test from `51-EFFORT-PROBE.md`'s verdict table: Kimi's
            // `thinking_object` form is confirmed (invalid value drew 400 on both models), so a
            // provider default lands in `thinking.effort` and nowhere else.
            #[cfg(feature = "kimi")]
            #[tokio::test]
            async fn kimi_applies_the_provider_default_as_thinking_effort() {
                assert_default_applies(Case {
                    provider: "kimi",
                    support: levels(&[ReasoningEffort::Low, ReasoningEffort::High]),
                    wire: None,
                    default: ReasoningEffort::High,
                    other: ReasoningEffort::Low,
                    read: |body| body.pointer("/thinking/effort").cloned(),
                    expect: level_spelling,
                })
                .await;
            }

            #[cfg(feature = "qwen")]
            #[tokio::test]
            async fn qwen_applies_the_provider_default_budget_below_the_request() {
                use paladin_ports::output::model_profile::BudgetBounds;
                // Qwen's `EnableThinking` wire carries a bounded budget and no levels.
                assert_default_applies(Case {
                    provider: "qwen",
                    support: EffortSupport::Supported(
                        EffortLevels::new([]).with_budget(BudgetBounds::new(1, 32_768)),
                    ),
                    wire: None,
                    default: ReasoningEffort::Budget(512),
                    other: ReasoningEffort::Budget(256),
                    read: |body| body.get("thinking_budget").cloned(),
                    expect: |effort| match effort {
                        ReasoningEffort::Budget(n) => json!(n),
                        other => panic!("not a budget: {other}"),
                    },
                })
                .await;
            }

            #[cfg(feature = "ollama")]
            #[tokio::test]
            async fn ollama_applies_the_provider_default_once_its_wire_is_declared() {
                assert_default_applies(Case {
                    provider: "ollama",
                    support: levels(&[ReasoningEffort::Low, ReasoningEffort::High]),
                    wire: Some(ReasoningWire::TopLevelEffort),
                    default: ReasoningEffort::High,
                    other: ReasoningEffort::Low,
                    read: |body| body.get("reasoning_effort").cloned(),
                    expect: level_spelling,
                })
                .await;
            }

            #[cfg(feature = "openai-compatible")]
            #[tokio::test]
            async fn openai_compatible_applies_the_provider_default_once_its_wire_is_declared() {
                assert_default_applies(Case {
                    provider: "openai-compatible",
                    support: levels(&[ReasoningEffort::Low, ReasoningEffort::High]),
                    wire: Some(ReasoningWire::TopLevelEffort),
                    default: ReasoningEffort::High,
                    other: ReasoningEffort::Low,
                    read: |body| body.get("reasoning_effort").cloned(),
                    expect: level_spelling,
                })
                .await;
            }

            /// Plan 51-11 restated this case from `51-EFFORT-PROBE.md`'s verdict table: Grok's
            /// top-level `reasoning_effort` is confirmed (invalid value drew 400 on 5 of 5
            /// models), so a provider default lands in that field, and a default on a model with
            /// no effort data still refuses before HTTP naming the key that set it.
            #[cfg(feature = "grok")]
            #[tokio::test]
            async fn grok_applies_the_provider_default_as_reasoning_effort() {
                assert_default_applies(Case {
                    provider: "grok",
                    support: levels(&[ReasoningEffort::Low, ReasoningEffort::High]),
                    wire: None,
                    default: ReasoningEffort::High,
                    other: ReasoningEffort::Low,
                    read: |body| body.get("reasoning_effort").cloned(),
                    expect: level_spelling,
                })
                .await;
            }

            // ── D-10: the self-hosted wire declared from YAML ──────────────────────────────

            #[cfg(feature = "openai-compatible")]
            #[test]
            fn an_openai_compatible_block_declares_its_wire() {
                let wire = wire_cases()
                    .into_iter()
                    .find(|c| c.provider == "openai-compatible")
                    .unwrap();
                let block = block_for(&wire, "http://127.0.0.1:1/v1")
                    .with_reasoning_wire(ReasoningWire::TopLevelEffort)
                    .with_models(BTreeMap::from([(
                        wire.model.to_string(),
                        ModelOverride {
                            effort: Some(levels(&[ReasoningEffort::High])),
                            ..ModelOverride::default()
                        },
                    )]));
                let config = config_with("openai-compatible", block);
                let port = LlmProviderFactory::new()
                    .create_from_config_with_env(&config, "openai-compatible", None, &no_env)
                    .expect("a self-hosted block may declare its wire");
                // The declared wire unmasks the override's levels in the reported profile.
                let profile = port.model_profile(wire.model).expect("a profile");
                assert!(
                    matches!(profile.effort, EffortSupport::Supported(_)),
                    "{:?}",
                    profile.effort
                );
                // Without the declaration the same override is masked to `WireUnsupported`.
                let undeclared = config_with(
                    "openai-compatible",
                    block_for(&wire, "http://127.0.0.1:1/v1").with_models(BTreeMap::from([(
                        wire.model.to_string(),
                        ModelOverride {
                            effort: Some(levels(&[ReasoningEffort::High])),
                            ..ModelOverride::default()
                        },
                    )])),
                );
                let port = LlmProviderFactory::new()
                    .create_from_config_with_env(&undeclared, "openai-compatible", None, &no_env)
                    .unwrap();
                let profile = port.model_profile(wire.model).expect("a profile");
                assert_eq!(profile.effort, EffortSupport::WireUnsupported);
            }

            /// Named vendors change their wire only through a Paladin release (D-10, T-51-20).
            #[test]
            fn a_named_vendor_block_with_a_reasoning_wire_is_refused() {
                for case in wire_cases() {
                    if matches!(case.provider, "ollama" | "openai-compatible") {
                        continue;
                    }
                    let block = block_for(&case, "http://127.0.0.1:1/v1")
                        .with_reasoning_wire(ReasoningWire::TopLevelEffort);
                    let config = config_with(case.provider, block);
                    let error = match LlmProviderFactory::new().create_from_config_with_env(
                        &config,
                        case.provider,
                        None,
                        &no_env,
                    ) {
                        Err(error) => error.to_string(),
                        Ok(_) => panic!(
                            "{}: a named vendor must refuse reasoning_wire",
                            case.provider
                        ),
                    };
                    assert!(
                        error.contains(&format!("llm.{}.reasoning_wire", case.provider)),
                        "{}: {error}",
                        case.provider
                    );
                    assert!(
                        error.contains("Paladin release"),
                        "{}: {error}",
                        case.provider
                    );
                }
            }

            #[cfg(feature = "ollama")]
            #[test]
            fn an_ollama_block_declares_its_wire() {
                let wire = wire_cases()
                    .into_iter()
                    .find(|c| c.provider == "ollama")
                    .unwrap();
                let block = block_for(&wire, "http://127.0.0.1:1/v1")
                    .with_reasoning_wire(ReasoningWire::ThinkingObject);
                let config = config_with("ollama", block);
                LlmProviderFactory::new()
                    .create_from_config_with_env(&config, "ollama", None, &no_env)
                    .expect("a keyless self-hosted block may declare its wire");
            }

            #[cfg(feature = "deepseek")]
            #[tokio::test]
            async fn a_provider_fallback_default_substitutes_when_the_model_lacks_the_level() {
                // The block's fallback is the provider-level default; the model admits only
                // `low` and `high`, so a default of `medium` runs as `low`.
                let mut vendor = server().await;
                let wire = wire_cases()
                    .into_iter()
                    .find(|c| c.provider == "deepseek")
                    .unwrap();
                let captured = crate::test_support::mock_capture(
                    &mut vendor,
                    &wire.path,
                    wire.response.clone(),
                )
                .await;
                let block = block_for(&wire, &vendor.url())
                    .with_models(BTreeMap::from([(
                        wire.model.to_string(),
                        ModelOverride {
                            effort: Some(levels(&[ReasoningEffort::Low, ReasoningEffort::High])),
                            ..ModelOverride::default()
                        },
                    )]))
                    .with_reasoning_effort(ReasoningEffort::Medium)
                    .with_reasoning_fallback(ReasoningFallback::NearestLower);
                let config = config_with("deepseek", block);
                let port = LlmProviderFactory::new()
                    .create_from_config_with_env(&config, "deepseek", None, &no_env)
                    .unwrap();
                let response = generate_once(port.as_ref(), wire.model).await.unwrap();
                let body = captured.lock().unwrap().take().unwrap();
                assert_eq!(body["reasoning_effort"], "low", "{body}");
                assert_eq!(stamped(&response), (Some("medium"), Some("low")));
            }
        }
    }
}
