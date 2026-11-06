//! The D-13 layering: how one provider's construction inputs are decided.
//!
//! Every input an adapter needs (key, base URL, model, timeout, retries, output budget, context
//! window, per-model overrides) is taken from the most specific operator statement that sets it,
//! in this fixed order, field by field:
//!
//! 1. the per-agent model (only the model, only when a caller names one),
//! 2. the `llm.<provider>` block,
//! 3. the provider's legacy environment variables (exactly the ones its `from_env` reads),
//! 4. the adapter's compiled default.
//!
//! The reasoning defaults (`reasoning_effort`, `reasoning_fallback`, Phase 51) are read from the
//! block alone: there is no environment variable for them and no compiled default (D-00i).
//!
//! An environment variable never overrides a value written in the block: the operator who typed
//! it into the file made the more specific statement (PARITY-03 prohibition, T-50-09-05).
//!
//! The timeout has one extra step between 3 and 4: when neither the block nor the environment
//! sets it and the resolved model is a reasoning model, it is
//! [`REASONING_DEFAULT_TIMEOUT_SECONDS`](crate::registry::REASONING_DEFAULT_TIMEOUT_SECONDS), the
//! same rule the direct `*Config` constructors apply (D-13). Whether the model is a reasoning
//! model counts the block's own `models` overrides, which the direct constructors cannot see.
//!
//! # Environment access
//!
//! Nothing here reads the process environment. Every function takes the lookup as a closure, so
//! the whole layering is a pure function of its inputs and an environment snapshot: tests pass a
//! map-backed closure (writing the process environment is `unsafe` under edition 2024 and this
//! crate denies `unsafe_code`), and concurrent callers share no state (RESEARCH Pattern 3).
//!
//! # Credentials
//!
//! Error strings name the provider, the model and the *variable* that was tried, never a value.
//! [`SecretSource`] and [`Layered`] print their key as `<redacted>` from `Debug`.

// The layering is consumed by the per-provider bridges, each behind its own adapter feature, so a
// build with no adapter compiled in has nothing that calls most of it.
#![cfg_attr(
    not(any(
        feature = "openai",
        feature = "anthropic",
        feature = "deepseek",
        feature = "kimi",
        feature = "qwen",
        feature = "grok",
        feature = "ollama",
        feature = "gemini",
        feature = "openai-compatible"
    )),
    allow(dead_code)
)]

use std::collections::BTreeMap;

use paladin_ports::output::llm_port::{ReasoningEffort, ReasoningFallback};

use crate::config::llm::LlmProviderConfig;
use crate::reasoning::{ReasoningWire, SELF_HOSTED_PROVIDERS};
use crate::registry::{ModelOverride, REASONING_DEFAULT_TIMEOUT_SECONDS};

/// The longest model id construction accepts, in characters.
pub(crate) const MAX_MODEL_ID_CHARS: usize = 256;

/// The longest model id echoed back inside an error message, in characters. A refused id may be
/// arbitrarily long and may carry control characters; the message shows a bounded, escaped prefix.
const MAX_ECHOED_MODEL_CHARS: usize = 80;

/// The largest `max_retries` an operator may configure. Each retry re-sends (and re-bills) the
/// whole request and waits an exponentially growing, capped interval first, so a value beyond a
/// handful buys minutes of waiting, not reliability. Adapters also clamp the wait themselves.
pub(crate) const MAX_RETRIES_CEILING: u32 = 10;

/// What an `api_key` string in a provider block turned out to be.
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum SecretSource {
    /// Used as written: any text that is not an exact `${NAME}` placeholder.
    Literal(String),
    /// An exact `${NAME}` placeholder whose variable is set and non-blank.
    FromEnv {
        /// The variable the placeholder named.
        var: String,
        /// The variable's value.
        value: String,
    },
    /// An exact `${NAME}` placeholder whose variable is unset or blank. Never sent to a vendor:
    /// the literal placeholder text is not a credential.
    MissingEnv {
        /// The variable the placeholder named.
        var: String,
    },
    /// Empty or whitespace only: the block makes no statement about the key.
    Empty,
}

impl std::fmt::Debug for SecretSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Literal(_) => f.write_str("Literal(<redacted>)"),
            Self::FromEnv { var, .. } => f
                .debug_struct("FromEnv")
                .field("var", var)
                .field("value", &"<redacted>")
                .finish(),
            Self::MissingEnv { var } => f.debug_struct("MissingEnv").field("var", var).finish(),
            Self::Empty => f.write_str("Empty"),
        }
    }
}

/// Classify a block's `api_key`.
///
/// Only an exact `${NAME}` with `NAME` made of `[A-Z0-9_]` (at least one character) is a
/// placeholder. `abc${X}`, `${X}abc`, `$X`, `${x}` (lower case) and `${}` are literals: the
/// narrow grammar means a key that merely contains `${` is never rewritten behind the operator's
/// back. A placeholder resolves through `env`; a variable that is unset or blank is
/// [`SecretSource::MissingEnv`], never silently the placeholder text (Pitfall 8).
pub(crate) fn resolve_secret_placeholder(
    raw: &str,
    env: &dyn Fn(&str) -> Option<String>,
) -> SecretSource {
    if raw.trim().is_empty() {
        return SecretSource::Empty;
    }
    match placeholder_name(raw) {
        Some(var) => match env(var).filter(|value| !value.trim().is_empty()) {
            Some(value) => SecretSource::FromEnv {
                var: var.to_string(),
                value,
            },
            None => SecretSource::MissingEnv {
                var: var.to_string(),
            },
        },
        None => SecretSource::Literal(raw.to_string()),
    }
}

/// The variable name inside an exact `${NAME}` placeholder, or `None` when `raw` is not one.
fn placeholder_name(raw: &str) -> Option<&str> {
    let name = raw.strip_prefix("${")?.strip_suffix('}')?;
    let valid = !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_');
    valid.then_some(name)
}

/// One provider's legacy environment variables: the names its `from_env` already reads, and no
/// others. `None` where the provider reads none.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ProviderEnv {
    /// The canonical provider name, used in error strings.
    pub(crate) provider: &'static str,
    /// The API key variable. `None` for a keyless provider (Ollama).
    pub(crate) key: Option<&'static str>,
    /// The base URL variable.
    pub(crate) base_url: Option<&'static str>,
    /// The model variable. `None` for a provider whose model has no environment variable (OpenAI).
    pub(crate) model: Option<&'static str>,
    /// The timeout variable, in seconds.
    pub(crate) timeout: Option<&'static str>,
    /// The retry-count variable.
    pub(crate) retries: Option<&'static str>,
    /// The output-budget variable.
    pub(crate) max_tokens: Option<&'static str>,
}

/// The adapter's compiled defaults: the bottom layer.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CompiledDefaults {
    /// The default base URL. `None` when there is no defensible default (the generic
    /// OpenAI-compatible provider), which makes the base URL required.
    pub(crate) base_url: Option<&'static str>,
    /// The default model. `None` with `model_required` makes the model required; `None` without
    /// it means the provider has no compiled default model.
    pub(crate) model: Option<&'static str>,
    /// Whether a model must be resolved from the layers above (the generic provider).
    pub(crate) model_required: bool,
    /// The default request timeout in seconds, for a model that is not a reasoning model.
    pub(crate) timeout_seconds: u64,
    /// The default retry count.
    pub(crate) max_retries: u32,
    /// The default output budget, when the provider has one.
    pub(crate) max_tokens: Option<u32>,
}

/// One provider's resolved construction inputs.
#[derive(Clone)]
pub(crate) struct Layered {
    /// The API key; empty for a keyless provider (and, in lenient mode, for an unresolved one, so
    /// adapter construction refuses it rather than sending a placeholder).
    pub(crate) api_key: String,
    /// The base URL.
    pub(crate) base_url: String,
    /// The resolved model; empty for a provider whose config carries none.
    pub(crate) model: String,
    /// The request timeout in seconds.
    pub(crate) timeout_seconds: u64,
    /// The retry count.
    pub(crate) max_retries: u32,
    /// The default output budget.
    pub(crate) max_tokens: Option<u32>,
    /// The adapter-wide context window.
    pub(crate) context_window: Option<u32>,
    /// The block's per-model overrides.
    pub(crate) model_overrides: BTreeMap<String, ModelOverride>,
    /// The block's provider-level default reasoning effort (Phase 51, D-00e). Block only: there
    /// is no environment variable and no compiled default (D-00i).
    pub(crate) reasoning_effort: Option<ReasoningEffort>,
    /// The block's provider-level default reasoning fallback (Phase 51, D-05). Block only.
    pub(crate) reasoning_fallback: Option<ReasoningFallback>,
    /// The block's declared reasoning wire (Phase 51, D-10), carried only for `ollama` and
    /// `openai-compatible`; any other provider's block that sets it is a resolve problem.
    pub(crate) reasoning_wire: Option<ReasoningWire>,
}

impl std::fmt::Debug for Layered {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Layered")
            .field("api_key", &crate::redaction::RedactedKey(&self.api_key))
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("timeout_seconds", &self.timeout_seconds)
            .field("max_retries", &self.max_retries)
            .field("max_tokens", &self.max_tokens)
            .field("context_window", &self.context_window)
            .field("model_overrides", &self.model_overrides)
            .field("reasoning_effort", &self.reasoning_effort)
            .field("reasoning_fallback", &self.reasoning_fallback)
            .field("reasoning_wire", &self.reasoning_wire)
            .finish()
    }
}

/// Why `model` is not an acceptable model id, or `Ok(())`.
///
/// A model id reaches a request body, and for Gemini its URL path, so an id that could alter a
/// path or smuggle a control character is refused before any adapter is built (T-50-09-04):
/// empty or whitespace-only, longer than [`MAX_MODEL_ID_CHARS`] characters, containing any
/// whitespace or control character, containing `..`, `?`, `#` or `%`, or starting with `/`.
///
/// # Errors
///
/// The reason, without echoing the id.
pub(crate) fn validate_model_id(model: &str) -> Result<(), &'static str> {
    if model.trim().is_empty() {
        return Err("is empty");
    }
    if model.chars().count() > MAX_MODEL_ID_CHARS {
        return Err("is longer than 256 characters");
    }
    if model.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err("contains whitespace or a control character");
    }
    if model.contains("..") {
        return Err("contains '..'");
    }
    if model.contains(['?', '#', '%']) {
        return Err("contains '?', '#' or '%'");
    }
    if model.starts_with('/') {
        return Err("starts with '/'");
    }
    Ok(())
}

/// `provider 'p' (model 'm')`, or `provider 'p'` when no model is known: the prefix every error
/// of this module carries. The model is escaped and bounded, since it may be a refused id.
pub(crate) fn subject(provider: &str, model: Option<&str>) -> String {
    match model {
        Some(model) if !model.is_empty() => {
            let shown: String = model
                .chars()
                .take(MAX_ECHOED_MODEL_CHARS)
                .flat_map(char::escape_debug)
                .collect();
            let ellipsis = if model.chars().count() > MAX_ECHOED_MODEL_CHARS {
                "..."
            } else {
                ""
            };
            format!("provider '{provider}' (model '{shown}{ellipsis}')")
        }
        Some(_) => format!("provider '{provider}' (model '')"),
        None => format!("provider '{provider}'"),
    }
}

/// A non-blank string, or `None`.
fn non_blank(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

/// Read the variable `name` (when the provider has one) as a non-blank string.
fn env_string(name: Option<&'static str>, env: &dyn Fn(&str) -> Option<String>) -> Option<String> {
    name.and_then(env).and_then(|v| non_blank(Some(&v)))
}

/// Read the numeric variable `name`. `Ok(None)` when it is absent or blank, `Err` naming the
/// variable (never its value) when it does not parse.
fn env_number<T: std::str::FromStr>(
    name: Option<&'static str>,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<Option<T>, &'static str> {
    let Some(var) = name else { return Ok(None) };
    match env(var).as_deref().map(str::trim).filter(|v| !v.is_empty()) {
        None => Ok(None),
        Some(raw) => raw.parse::<T>().map(Some).map_err(|_| var),
    }
}

/// A numeric field: the block's value when it sets one, else the provider's variable. A variable
/// that is set but does not parse records a problem naming the variable (never its value) and
/// counts as unset.
fn layered_number<T: std::str::FromStr>(
    block_value: Option<T>,
    var: Option<&'static str>,
    env: &dyn Fn(&str) -> Option<String>,
    problems: &mut Vec<String>,
    subject_text: &str,
    what: &str,
) -> Option<T> {
    if block_value.is_some() {
        return block_value;
    }
    match env_number::<T>(var, env) {
        Ok(value) => value,
        Err(var) => {
            problems.push(format!(
                "{subject_text}: environment variable {var} is not a valid {what}"
            ));
            None
        }
    }
}

/// The core of the layering: resolve every input, collecting each problem found while filling the
/// field from the next layer down. The first problem is the one a strict caller reports.
fn resolve(
    block: Option<&LlmProviderConfig>,
    agent_model: Option<&str>,
    provider_env: &ProviderEnv,
    defaults: &CompiledDefaults,
    is_reasoning: &dyn Fn(&str) -> bool,
    env: &dyn Fn(&str) -> Option<String>,
) -> (Layered, Vec<String>) {
    let provider = provider_env.provider;
    let mut problems: Vec<String> = Vec::new();

    // Model: agent > block > env > compiled. The two operator-written statements (the agent's
    // model and the block's `default_model`) are shape-checked; the environment value keeps the
    // pre-layering behaviour of reaching the adapter unchecked.
    if let Some(requested) = agent_model
        && let Err(reason) = validate_model_id(requested)
    {
        problems.push(format!(
            "{}: model id {reason}",
            subject(provider, Some(requested))
        ));
    }
    let block_model = block.and_then(|b| non_blank(b.default_model.as_deref()));
    if let Some(configured) = &block_model
        && let Err(reason) = validate_model_id(configured)
    {
        problems.push(format!(
            "{}: llm.{provider}.default_model {reason}",
            subject(provider, Some(configured))
        ));
    }
    let model: Option<String> = agent_model
        .filter(|m| !m.trim().is_empty())
        .map(str::to_string)
        .or(block_model)
        .or_else(|| env_string(provider_env.model, env))
        .or_else(|| defaults.model.map(str::to_string));
    if model.is_none() && defaults.model_required {
        let var = provider_env
            .model
            .unwrap_or("the model environment variable");
        problems.push(format!(
            "{}: no model: set `llm.{provider}.default_model` or {var}, or name a model",
            subject(provider, None)
        ));
    }
    let subject_text = subject(provider, model.as_deref());

    // Key: the block's key unless it is empty or an exact placeholder; then the provider's
    // legacy variable. An unset placeholder is an error, never a fallback to a different
    // credential than the one the operator named.
    let api_key = match provider_env.key {
        None => String::new(),
        Some(var) => {
            let from_env = || env_string(Some(var), env);
            let missing = || {
                format!(
                    "{subject_text}: no API key: set `llm.{provider}.api_key` or the {var} \
                     environment variable"
                )
            };
            match block.map(|b| resolve_secret_placeholder(&b.api_key, env)) {
                Some(SecretSource::Literal(value)) | Some(SecretSource::FromEnv { value, .. }) => {
                    value
                }
                Some(SecretSource::MissingEnv { var: placeholder }) => {
                    problems.push(format!(
                        "{subject_text}: llm.{provider}.api_key is the placeholder \
                         '${{{placeholder}}}', but environment variable {placeholder} is not set"
                    ));
                    String::new()
                }
                Some(SecretSource::Empty) | None => from_env().unwrap_or_else(|| {
                    problems.push(missing());
                    String::new()
                }),
            }
        }
    };

    // Base URL.
    let base_url = non_blank(block.and_then(|b| b.base_url.as_deref()))
        .or_else(|| env_string(provider_env.base_url, env))
        .or_else(|| defaults.base_url.map(str::to_string))
        .unwrap_or_else(|| {
            let var = provider_env
                .base_url
                .unwrap_or("the base URL environment variable");
            problems.push(format!(
                "{subject_text}: no base URL: set `llm.{provider}.base_url` or {var}; there is \
                 no default endpoint for this provider"
            ));
            String::new()
        });

    // Timeout: block > env > 300 s for a reasoning model > compiled. The environment is read
    // only for a field the block leaves unset, so a variable the block overrides can never
    // matter, even a malformed one.
    let timeout_seconds = layered_number::<u64>(
        block.and_then(|b| b.timeout_seconds),
        provider_env.timeout,
        env,
        &mut problems,
        &subject_text,
        "seconds",
    )
    .unwrap_or_else(|| {
        if model.as_deref().is_some_and(is_reasoning) {
            REASONING_DEFAULT_TIMEOUT_SECONDS
        } else {
            defaults.timeout_seconds
        }
    });

    let max_retries = layered_number::<u32>(
        block.and_then(|b| b.max_retries),
        provider_env.retries,
        env,
        &mut problems,
        &subject_text,
        "count",
    )
    .unwrap_or(defaults.max_retries);

    // A zero timeout makes the HTTP client fail every request at once, and an unbounded retry
    // count turns one outage into hours of back-off. Both are refused here, naming the provider
    // and the key (never an environment variable's raw text).
    if timeout_seconds == 0 {
        problems.push(format!(
            "{subject_text}: `llm.{provider}.timeout_seconds` must be at least 1"
        ));
    }
    if max_retries > MAX_RETRIES_CEILING {
        problems.push(format!(
            "{subject_text}: `llm.{provider}.max_retries` must be at most {MAX_RETRIES_CEILING}, \
             got {max_retries}"
        ));
    }

    let max_tokens = layered_number::<u32>(
        block.and_then(|b| b.max_tokens),
        provider_env.max_tokens,
        env,
        &mut problems,
        &subject_text,
        "token count",
    )
    .or(defaults.max_tokens);

    // Reasoning wire (Phase 51, D-10): only a self-hosted endpoint can declare it. A named vendor
    // changes its wire through a Paladin release after the dated probe, never through
    // configuration, so a block that sets it is refused rather than silently ignored.
    let reasoning_wire = block.and_then(|b| b.reasoning_wire);
    if reasoning_wire.is_some() && !SELF_HOSTED_PROVIDERS.contains(&provider) {
        problems.push(format!(
            "{subject_text}: `llm.{provider}.reasoning_wire` is only accepted for `ollama` and \
             `openai-compatible`; named vendors change their wire only through a Paladin release"
        ));
    }

    let layered = Layered {
        api_key,
        base_url,
        model: model.unwrap_or_default(),
        timeout_seconds,
        max_retries,
        max_tokens,
        context_window: block.and_then(|b| b.context_window),
        model_overrides: block.map(|b| b.models.clone()).unwrap_or_default(),
        reasoning_effort: block.and_then(|b| b.reasoning_effort),
        reasoning_fallback: block.and_then(|b| b.reasoning_fallback),
        reasoning_wire,
    };
    (layered, problems)
}

/// Resolve one provider's construction inputs in D-13's order.
///
/// `agent_model` is the per-agent model, the most specific statement of all; `is_reasoning` says
/// whether a model id is a reasoning model (the 300 s default applies when neither the block nor
/// the environment sets a timeout); `env` looks a variable up.
///
/// # Errors
///
/// A message naming the provider, the model and the variable tried (never a value) when: the
/// agent's or the block's model id is malformed, no key resolves (empty block key and the
/// provider's variable unset, or an exact `${NAME}` placeholder whose variable is unset), a
/// required base URL or model is absent, or a numeric environment variable does not parse.
pub(crate) fn resolve_layers(
    block: Option<&LlmProviderConfig>,
    agent_model: Option<&str>,
    provider_env: &ProviderEnv,
    defaults: &CompiledDefaults,
    is_reasoning: &dyn Fn(&str) -> bool,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<Layered, String> {
    let (layered, problems) = resolve(
        block,
        agent_model,
        provider_env,
        defaults,
        is_reasoning,
        env,
    );
    match problems.into_iter().next() {
        Some(first) => Err(first),
        None => Ok(layered),
    }
}

/// [`resolve_layers`] for the infallible `From<&LlmProviderConfig>` conversions: the same
/// layering with no environment, where a problem becomes an empty field instead of an error.
///
/// An unresolved key or base URL is therefore empty, and the adapter's own construction check
/// refuses it ("API key cannot be empty") rather than a `${NAME}` placeholder being sent to a
/// vendor as the credential. A malformed model id is passed through for the adapter to refuse.
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
pub(crate) fn resolve_layers_lenient(
    block: &LlmProviderConfig,
    provider_env: &ProviderEnv,
    defaults: &CompiledDefaults,
    is_reasoning: &dyn Fn(&str) -> bool,
) -> Layered {
    resolve(
        Some(block),
        None,
        provider_env,
        defaults,
        is_reasoning,
        &|_| None,
    )
    .0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A map-backed environment: no `std::env` access anywhere in this module's tests.
    fn env_of<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| {
            pairs
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| (*v).to_string())
        }
    }

    const ENV: ProviderEnv = ProviderEnv {
        provider: "acme",
        key: Some("ACME_API_KEY"),
        base_url: Some("ACME_BASE_URL"),
        model: Some("ACME_MODEL"),
        timeout: Some("ACME_TIMEOUT_SECONDS"),
        retries: Some("ACME_MAX_RETRIES"),
        max_tokens: Some("ACME_MAX_TOKENS"),
    };

    const DEFAULTS: CompiledDefaults = CompiledDefaults {
        base_url: Some("https://compiled.example/v1"),
        model: Some("compiled-model"),
        model_required: false,
        timeout_seconds: 60,
        max_retries: 3,
        max_tokens: Some(4096),
    };

    fn never_reasoning(_: &str) -> bool {
        false
    }

    fn resolve_with(
        block: Option<&LlmProviderConfig>,
        agent_model: Option<&str>,
        env: &dyn Fn(&str) -> Option<String>,
    ) -> Result<Layered, String> {
        resolve_layers(block, agent_model, &ENV, &DEFAULTS, &never_reasoning, env)
    }

    // ── Phase 51: reasoning defaults and the wire declaration ───────────────────

    /// The block-only reasoning keys reach `Layered`; with none set, nothing is invented (D-00i)
    /// and no environment variable can supply them.
    #[test]
    fn reasoning_defaults_come_from_the_block_alone() {
        let env = env_of(&[("ACME_API_KEY", "k"), ("ACME_REASONING_EFFORT", "high")]);
        let bare = resolve_with(None, None, &env).expect("resolves");
        assert_eq!(
            (
                bare.reasoning_effort,
                bare.reasoning_fallback,
                bare.reasoning_wire
            ),
            (None, None, None)
        );
        let block = LlmProviderConfig::new("")
            .with_reasoning_effort(ReasoningEffort::Medium)
            .with_reasoning_fallback(ReasoningFallback::NearestHigher);
        let layered = resolve_with(Some(&block), None, &env).expect("resolves");
        assert_eq!(layered.reasoning_effort, Some(ReasoningEffort::Medium));
        assert_eq!(
            layered.reasoning_fallback,
            Some(ReasoningFallback::NearestHigher)
        );
    }

    /// D-10, T-51-20: only the self-hosted endpoints may declare a wire; any other provider's
    /// block that sets one is refused with a message naming the key.
    #[test]
    fn a_reasoning_wire_is_refused_outside_the_self_hosted_providers() {
        let env = env_of(&[("ACME_API_KEY", "k")]);
        let block = LlmProviderConfig::new("").with_reasoning_wire(ReasoningWire::TopLevelEffort);
        let error = resolve_with(Some(&block), None, &env).expect_err("acme is a named vendor");
        assert!(error.contains("llm.acme.reasoning_wire"), "{error}");
        assert!(error.contains("Paladin release"), "{error}");

        for provider in SELF_HOSTED_PROVIDERS {
            let self_hosted = ProviderEnv {
                provider,
                key: None,
                ..ENV
            };
            let layered = resolve_layers(
                Some(&block),
                None,
                &self_hosted,
                &DEFAULTS,
                &never_reasoning,
                &env,
            )
            .expect("a self-hosted endpoint may declare its wire");
            assert_eq!(layered.reasoning_wire, Some(ReasoningWire::TopLevelEffort));
        }
    }

    /// D-10, TESTS-05 (51 IN-01): the layering's `reasoning_wire` refusal and the effort
    /// refusal's "set `llm.<provider>.reasoning_wire`" hint read ONE constant, so the two sites
    /// cannot drift. The sibling test is
    /// `reasoning::tests::wire_refusal_points_self_hosted_providers_at_reasoning_wire`.
    #[test]
    fn reasoning_wire_providers_are_the_self_hosted_pair() {
        let shared = crate::reasoning::SELF_HOSTED_PROVIDERS;
        assert_eq!(shared.len(), 2, "the self-hosted pair");
        assert!(shared.contains(&"ollama"));
        assert!(shared.contains(&"openai-compatible"));

        let env = env_of(&[("ACME_API_KEY", "k")]);
        let block = LlmProviderConfig::new("").with_reasoning_wire(ReasoningWire::TopLevelEffort);
        for provider in shared {
            let self_hosted = ProviderEnv {
                provider,
                key: None,
                ..ENV
            };
            resolve_layers(
                Some(&block),
                None,
                &self_hosted,
                &DEFAULTS,
                &never_reasoning,
                &env,
            )
            .unwrap_or_else(|error| panic!("{provider} may declare its wire: {error}"));
        }
        let error = resolve_with(Some(&block), None, &env).expect_err("acme is a named vendor");
        assert!(error.contains("llm.acme.reasoning_wire"), "{error}");
    }

    // ── placeholders ────────────────────────────────────────────────────────

    #[test]
    fn placeholder_resolves_from_its_variable() {
        let env = env_of(&[("MOONSHOT_API_KEY", "sk-from-env")]);
        match resolve_secret_placeholder("${MOONSHOT_API_KEY}", &env) {
            SecretSource::FromEnv { var, value } => {
                assert_eq!(var, "MOONSHOT_API_KEY");
                assert_eq!(value, "sk-from-env");
            }
            other => panic!("expected FromEnv, got {other:?}"),
        }
    }

    #[test]
    fn an_unset_or_blank_placeholder_variable_is_missing_not_literal() {
        let none = env_of(&[]);
        assert!(matches!(
            resolve_secret_placeholder("${MOONSHOT_API_KEY}", &none),
            SecretSource::MissingEnv { ref var } if var == "MOONSHOT_API_KEY"
        ));
        let blank = env_of(&[("MOONSHOT_API_KEY", "   ")]);
        assert!(matches!(
            resolve_secret_placeholder("${MOONSHOT_API_KEY}", &blank),
            SecretSource::MissingEnv { .. }
        ));
    }

    #[test]
    fn only_an_exact_placeholder_is_a_placeholder() {
        let env = env_of(&[("X", "leaked"), ("x", "leaked")]);
        for literal in [
            "abc${X}", "${X}abc", "$X", "${x}", "${}", "${X", "$${X}", "${X Y}", "${X-Y}", " ${X}",
        ] {
            assert_eq!(
                resolve_secret_placeholder(literal, &env),
                SecretSource::Literal(literal.to_string()),
                "{literal:?} must be used literally"
            );
        }
    }

    #[test]
    fn an_empty_or_blank_key_is_empty() {
        let env = env_of(&[]);
        assert_eq!(resolve_secret_placeholder("", &env), SecretSource::Empty);
        assert_eq!(
            resolve_secret_placeholder("  \t", &env),
            SecretSource::Empty
        );
    }

    #[test]
    fn secret_source_and_layered_debug_never_print_a_key() {
        let env = env_of(&[("K", "env-secret-value")]);
        for source in [
            SecretSource::Literal("literal-secret-value".to_string()),
            resolve_secret_placeholder("${K}", &env),
        ] {
            let rendered = format!("{source:?}");
            assert!(!rendered.contains("secret-value"), "{rendered}");
        }
        let block = LlmProviderConfig::new("block-secret-value");
        let layered = resolve_with(Some(&block), None, &env_of(&[])).unwrap();
        assert!(!format!("{layered:?}").contains("secret-value"));
    }

    // ── model-id shape ──────────────────────────────────────────────────────

    #[test]
    fn model_ids_are_shape_checked() {
        for good in [
            "kimi-k3",
            "gemini-3.5-flash-lite",
            "llama3:8b-instruct-q4_K_M",
            "hf.co/user/model:tag",
            "models/x-1.5",
            &"a".repeat(256),
        ] {
            assert!(validate_model_id(good).is_ok(), "{good:?} must be accepted");
        }
        for bad in [
            "",
            "   ",
            "a b",
            "a\tb",
            "a\nb",
            "a\u{0}b",
            "../etc/passwd",
            "a..b",
            "a?b",
            "a#b",
            "a%2Fb",
            "/leading",
            &"a".repeat(257),
        ] {
            assert!(validate_model_id(bad).is_err(), "{bad:?} must be refused");
        }
    }

    #[test]
    fn a_refused_model_id_is_named_escaped_and_bounded() {
        let long = format!("a?{}", "z".repeat(500));
        let error = resolve_with(
            Some(&LlmProviderConfig::new("k")),
            Some(&long),
            &env_of(&[]),
        )
        .unwrap_err();
        assert!(
            error.starts_with("provider 'acme' (model 'a?zzz"),
            "{error}"
        );
        assert!(error.contains("..."), "the echo is bounded: {error}");
        assert!(error.len() < 400, "the echo is bounded: {}", error.len());
        let control = resolve_with(
            Some(&LlmProviderConfig::new("k")),
            Some("a\u{1b}[31mb"),
            &env_of(&[]),
        )
        .unwrap_err();
        assert!(
            !control.contains('\u{1b}'),
            "control characters are escaped"
        );
    }

    // ── precedence ──────────────────────────────────────────────────────────

    #[test]
    fn compiled_defaults_apply_when_nothing_else_does() {
        let block = LlmProviderConfig::new("k");
        let layered = resolve_with(Some(&block), None, &env_of(&[])).unwrap();
        assert_eq!(layered.api_key, "k");
        assert_eq!(layered.base_url, "https://compiled.example/v1");
        assert_eq!(layered.model, "compiled-model");
        assert_eq!(layered.timeout_seconds, 60);
        assert_eq!(layered.max_retries, 3);
        assert_eq!(layered.max_tokens, Some(4096));
        assert_eq!(layered.context_window, None);
        assert!(layered.model_overrides.is_empty());
    }

    #[test]
    fn env_fills_what_the_block_leaves_unset() {
        let env = env_of(&[
            ("ACME_BASE_URL", "https://env.example/v1"),
            ("ACME_MODEL", "env-model"),
            ("ACME_TIMEOUT_SECONDS", "45"),
            ("ACME_MAX_RETRIES", "7"),
            ("ACME_MAX_TOKENS", "999"),
        ]);
        let layered = resolve_with(Some(&LlmProviderConfig::new("k")), None, &env).unwrap();
        assert_eq!(layered.base_url, "https://env.example/v1");
        assert_eq!(layered.model, "env-model");
        assert_eq!(layered.timeout_seconds, 45);
        assert_eq!(layered.max_retries, 7);
        assert_eq!(layered.max_tokens, Some(999));
    }

    #[test]
    fn block_beats_env_for_every_field() {
        let env = env_of(&[
            ("ACME_API_KEY", "env-key"),
            ("ACME_BASE_URL", "https://env.example/v1"),
            ("ACME_MODEL", "env-model"),
            ("ACME_TIMEOUT_SECONDS", "45"),
            ("ACME_MAX_RETRIES", "7"),
            ("ACME_MAX_TOKENS", "999"),
        ]);
        let block = LlmProviderConfig::new("block-key")
            .with_base_url("https://block.example/v1")
            .with_default_model("block-model")
            .with_timeout_seconds(11)
            .with_max_retries(1)
            .with_max_tokens(222)
            .with_context_window(333);
        let layered = resolve_with(Some(&block), None, &env).unwrap();
        assert_eq!(layered.api_key, "block-key");
        assert_eq!(layered.base_url, "https://block.example/v1");
        assert_eq!(layered.model, "block-model");
        assert_eq!(layered.timeout_seconds, 11);
        assert_eq!(layered.max_retries, 1);
        assert_eq!(layered.max_tokens, Some(222));
        assert_eq!(layered.context_window, Some(333));
    }

    #[test]
    fn a_block_value_equal_to_its_env_var_yields_the_same_config_as_either_alone() {
        let env = env_of(&[
            ("ACME_API_KEY", "same-key"),
            ("ACME_BASE_URL", "https://same.example/v1"),
            ("ACME_MODEL", "same-model"),
            ("ACME_TIMEOUT_SECONDS", "45"),
            ("ACME_MAX_RETRIES", "2"),
            ("ACME_MAX_TOKENS", "100"),
        ]);
        let block = LlmProviderConfig::new("same-key")
            .with_base_url("https://same.example/v1")
            .with_default_model("same-model")
            .with_timeout_seconds(45)
            .with_max_retries(2)
            .with_max_tokens(100);
        let both = resolve_with(Some(&block), None, &env).unwrap();
        let block_only = resolve_with(Some(&block), None, &env_of(&[])).unwrap();
        let env_only = resolve_with(None, None, &env).unwrap();
        for other in [&block_only, &env_only] {
            assert_eq!(both.api_key, other.api_key);
            assert_eq!(both.base_url, other.base_url);
            assert_eq!(both.model, other.model);
            assert_eq!(both.timeout_seconds, other.timeout_seconds);
            assert_eq!(both.max_retries, other.max_retries);
            assert_eq!(both.max_tokens, other.max_tokens);
        }
    }

    #[test]
    fn per_agent_model_beats_block_default_model() {
        let env = env_of(&[("ACME_MODEL", "env-model")]);
        let block = LlmProviderConfig::new("k").with_default_model("block-model");
        let layered = resolve_with(Some(&block), Some("agent-model"), &env).unwrap();
        assert_eq!(layered.model, "agent-model");
        // Nothing but the model changes.
        let plain = resolve_with(Some(&block), None, &env).unwrap();
        assert_eq!(plain.model, "block-model");
        assert_eq!(layered.base_url, plain.base_url);
        assert_eq!(layered.timeout_seconds, plain.timeout_seconds);
    }

    #[test]
    fn a_per_agent_model_equal_to_the_block_default_yields_an_identical_config() {
        let block = LlmProviderConfig::new("k").with_default_model("m-1");
        let by_agent = resolve_with(Some(&block), Some("m-1"), &env_of(&[])).unwrap();
        let by_block = resolve_with(Some(&block), None, &env_of(&[])).unwrap();
        assert_eq!(format!("{by_agent:?}"), format!("{by_block:?}"));
    }

    #[test]
    fn the_precedence_order_does_not_depend_on_which_layers_set_which_fields() {
        // The agent sets only the model, the block only the timeout, the environment the rest:
        // each field comes from the highest layer that sets it.
        let env = env_of(&[
            ("ACME_MODEL", "env-model"),
            ("ACME_TIMEOUT_SECONDS", "45"),
            ("ACME_MAX_RETRIES", "7"),
        ]);
        let block = LlmProviderConfig::new("k").with_timeout_seconds(11);
        let layered = resolve_with(Some(&block), Some("agent-model"), &env).unwrap();
        assert_eq!(layered.model, "agent-model");
        assert_eq!(layered.timeout_seconds, 11);
        assert_eq!(layered.max_retries, 7);
        assert_eq!(layered.base_url, "https://compiled.example/v1");
    }

    #[test]
    fn blank_block_strings_make_no_statement() {
        let mut block = LlmProviderConfig::new("k");
        block.base_url = Some("  ".to_string());
        block.default_model = Some(String::new());
        let env = env_of(&[("ACME_BASE_URL", "https://env.example/v1")]);
        let layered = resolve_with(Some(&block), None, &env).unwrap();
        assert_eq!(layered.base_url, "https://env.example/v1");
        assert_eq!(layered.model, "compiled-model");
    }

    // ── operator-settable knobs are bounded (WR-02) ─────────────────────────

    #[test]
    fn a_zero_timeout_is_refused_by_name() {
        let block = LlmProviderConfig::new("k").with_timeout_seconds(0);
        let message = resolve_with(Some(&block), None, &env_of(&[])).unwrap_err();
        assert!(
            message.contains("acme") && message.contains("timeout_seconds"),
            "the refusal must name the provider and the key: {message}"
        );
        let env = env_of(&[("ACME_TIMEOUT_SECONDS", "0")]);
        let from_env = resolve_with(Some(&LlmProviderConfig::new("k")), None, &env).unwrap_err();
        assert!(from_env.contains("timeout_seconds"), "{from_env}");
    }

    #[test]
    fn max_retries_above_the_ceiling_is_refused_and_the_ceiling_is_allowed() {
        let at_ceiling = LlmProviderConfig::new("k").with_max_retries(MAX_RETRIES_CEILING);
        let layered = resolve_with(Some(&at_ceiling), None, &env_of(&[])).unwrap();
        assert_eq!(layered.max_retries, MAX_RETRIES_CEILING);

        for too_many in [MAX_RETRIES_CEILING + 1, u32::MAX] {
            let block = LlmProviderConfig::new("k").with_max_retries(too_many);
            let message = resolve_with(Some(&block), None, &env_of(&[])).unwrap_err();
            assert!(
                message.contains("acme") && message.contains("max_retries"),
                "the refusal must name the provider and the key: {message}"
            );
        }
        let env = env_of(&[("ACME_MAX_RETRIES", "4000000000")]);
        let from_env = resolve_with(Some(&LlmProviderConfig::new("k")), None, &env).unwrap_err();
        assert!(from_env.contains("max_retries"), "{from_env}");
    }

    // ── the 300 s reasoning default ─────────────────────────────────────────

    #[test]
    fn reasoning_model_defaults_to_300s() {
        let reasoning = |m: &str| m == "thinker";
        let resolve_model =
            |block: Option<&LlmProviderConfig>, model: &str, env: &[(&str, &str)]| {
                resolve_layers(
                    block,
                    Some(model),
                    &ENV,
                    &DEFAULTS,
                    &reasoning,
                    &env_of(env),
                )
                .unwrap()
                .timeout_seconds
            };
        let block = LlmProviderConfig::new("k");
        // A reasoning model with no timeout anywhere: 300.
        assert_eq!(resolve_model(Some(&block), "thinker", &[]), 300);
        // Any other model keeps the adapter's compiled default.
        assert_eq!(resolve_model(Some(&block), "plain", &[]), 60);
        // An explicit block or env timeout always wins, even for a reasoning model.
        let with_timeout = LlmProviderConfig::new("k").with_timeout_seconds(20);
        assert_eq!(resolve_model(Some(&with_timeout), "thinker", &[]), 20);
        assert_eq!(
            resolve_model(Some(&block), "thinker", &[("ACME_TIMEOUT_SECONDS", "30")]),
            30
        );
        // The block's own timeout beats the env timeout.
        assert_eq!(
            resolve_model(
                Some(&with_timeout),
                "thinker",
                &[("ACME_TIMEOUT_SECONDS", "30")]
            ),
            20
        );
        // No block at all behaves the same.
        assert_eq!(
            resolve_model(None, "thinker", &[("ACME_API_KEY", "k")]),
            300
        );
    }

    #[test]
    fn the_reasoning_check_sees_the_resolved_model() {
        let seen = std::cell::RefCell::new(Vec::new());
        let probe = |m: &str| {
            seen.borrow_mut().push(m.to_string());
            false
        };
        let block = LlmProviderConfig::new("k").with_default_model("block-model");
        resolve_layers(Some(&block), None, &ENV, &DEFAULTS, &probe, &env_of(&[])).unwrap();
        assert_eq!(seen.borrow().as_slice(), ["block-model"]);
        // An explicit timeout never asks.
        seen.borrow_mut().clear();
        let block = block.with_timeout_seconds(5);
        resolve_layers(Some(&block), None, &ENV, &DEFAULTS, &probe, &env_of(&[])).unwrap();
        assert!(seen.borrow().is_empty());
    }

    // ── the key ─────────────────────────────────────────────────────────────

    #[test]
    fn no_block_and_env_set_behaves_as_env_only() {
        let env = env_of(&[("ACME_API_KEY", "env-key")]);
        let layered = resolve_with(None, None, &env).unwrap();
        assert_eq!(layered.api_key, "env-key");
        assert_eq!(layered.model, "compiled-model");
    }

    #[test]
    fn a_block_with_only_a_key_is_filled_from_env_and_compiled_defaults() {
        let env = env_of(&[("ACME_BASE_URL", "https://env.example/v1")]);
        let layered = resolve_with(Some(&LlmProviderConfig::new("k")), None, &env).unwrap();
        assert_eq!(layered.base_url, "https://env.example/v1");
        assert_eq!(layered.max_retries, 3);
    }

    #[test]
    fn an_empty_block_key_falls_back_to_the_provider_env_var() {
        let env = env_of(&[("ACME_API_KEY", "env-key")]);
        let layered = resolve_with(Some(&LlmProviderConfig::new("")), None, &env).unwrap();
        assert_eq!(layered.api_key, "env-key");
    }

    #[test]
    fn a_placeholder_key_resolves_from_its_variable() {
        let env = env_of(&[("MOONSHOT_API_KEY", "sk-moon")]);
        let block = LlmProviderConfig::new("${MOONSHOT_API_KEY}");
        assert_eq!(
            resolve_with(Some(&block), None, &env).unwrap().api_key,
            "sk-moon"
        );
    }

    #[test]
    fn an_unset_placeholder_variable_is_an_error_naming_provider_and_variable() {
        // The provider's own variable is set, but the operator named a different one: the
        // operator's statement wins and its absence is an error, not a fallback.
        let env = env_of(&[("ACME_API_KEY", "other-credential")]);
        let block = LlmProviderConfig::new("${MY_ACME_KEY}").with_default_model("acme-1");
        let error = resolve_with(Some(&block), None, &env).unwrap_err();
        assert!(
            error.starts_with("provider 'acme' (model 'acme-1'):"),
            "{error}"
        );
        assert!(error.contains("MY_ACME_KEY"), "{error}");
        assert!(!error.contains("other-credential"), "{error}");
    }

    #[test]
    fn literal_lookalikes_are_used_verbatim_as_the_key() {
        let env = env_of(&[("X", "leaked")]);
        for text in ["abc${X}", "$X", "${x}"] {
            let layered = resolve_with(Some(&LlmProviderConfig::new(text)), None, &env).unwrap();
            assert_eq!(layered.api_key, text);
        }
    }

    #[test]
    fn no_block_and_no_env_var_is_an_error_naming_provider_model_and_variable() {
        let error = resolve_with(None, Some("acme-1"), &env_of(&[])).unwrap_err();
        assert!(
            error.starts_with("provider 'acme' (model 'acme-1'):"),
            "{error}"
        );
        assert!(error.contains("ACME_API_KEY"), "{error}");
        assert!(error.contains("llm.acme.api_key"), "{error}");
    }

    #[test]
    fn a_keyless_provider_needs_no_key() {
        let keyless = ProviderEnv { key: None, ..ENV };
        let layered = resolve_layers(
            None,
            None,
            &keyless,
            &DEFAULTS,
            &never_reasoning,
            &env_of(&[]),
        )
        .unwrap();
        assert_eq!(layered.api_key, "");
    }

    #[test]
    fn no_error_ever_contains_a_key_value() {
        let env = env_of(&[
            ("ACME_API_KEY", "sk-env-credential"),
            ("ACME_TIMEOUT_SECONDS", "sk-env-credential"),
        ]);
        let block = LlmProviderConfig::new("sk-block-credential");
        for error in [
            resolve_with(Some(&block), None, &env).unwrap_err(),
            resolve_with(Some(&block), Some("a b"), &env).unwrap_err(),
            resolve_with(None, None, &env).unwrap_err(),
        ] {
            assert!(!error.contains("sk-env-credential"), "{error}");
            assert!(!error.contains("sk-block-credential"), "{error}");
        }
    }

    // ── required values and malformed numbers ───────────────────────────────

    #[test]
    fn a_required_base_url_and_model_must_come_from_a_layer() {
        let required = CompiledDefaults {
            base_url: None,
            model: None,
            model_required: true,
            ..DEFAULTS
        };
        let block = LlmProviderConfig::new("k");
        let error = resolve_layers(
            Some(&block),
            None,
            &ENV,
            &required,
            &never_reasoning,
            &env_of(&[]),
        )
        .unwrap_err();
        assert!(error.contains("no model"), "{error}");
        assert!(error.contains("ACME_MODEL"), "{error}");
        let with_model = LlmProviderConfig::new("k").with_default_model("m");
        let error = resolve_layers(
            Some(&with_model),
            None,
            &ENV,
            &required,
            &never_reasoning,
            &env_of(&[]),
        )
        .unwrap_err();
        assert!(error.contains("no base URL"), "{error}");
        assert!(error.contains("ACME_BASE_URL"), "{error}");
        let env = env_of(&[("ACME_BASE_URL", "https://e.example/v1")]);
        let layered = resolve_layers(
            Some(&with_model),
            None,
            &ENV,
            &required,
            &never_reasoning,
            &env,
        )
        .unwrap();
        assert_eq!(layered.base_url, "https://e.example/v1");
    }

    #[test]
    fn a_malformed_numeric_env_var_is_an_error_naming_the_variable_not_its_value() {
        for (var, value) in [
            ("ACME_TIMEOUT_SECONDS", "soon"),
            ("ACME_MAX_RETRIES", "-1"),
            ("ACME_MAX_TOKENS", "1.5"),
        ] {
            let pairs = [(var, value)];
            let env = env_of(&pairs);
            let error = resolve_with(Some(&LlmProviderConfig::new("k")), None, &env).unwrap_err();
            assert!(error.contains(var), "{error}");
            assert!(!error.contains(value), "{error}");
        }
    }

    #[test]
    fn a_malformed_env_value_matters_only_for_a_field_the_block_leaves_unset() {
        let env = env_of(&[("ACME_TIMEOUT_SECONDS", "soon")]);
        // The block sets the timeout, so the garbage variable is never consulted ...
        let block = LlmProviderConfig::new("k").with_timeout_seconds(5);
        assert_eq!(
            resolve_with(Some(&block), None, &env)
                .unwrap()
                .timeout_seconds,
            5
        );
        // ... and with the block silent, it is reported.
        assert!(resolve_with(Some(&LlmProviderConfig::new("k")), None, &env).is_err());
    }

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
    #[test]
    fn lenient_resolution_never_errors_and_never_sends_a_placeholder() {
        let block = LlmProviderConfig::new("${NOT_SET_ANYWHERE}");
        let layered = resolve_layers_lenient(&block, &ENV, &DEFAULTS, &never_reasoning);
        assert_eq!(
            layered.api_key, "",
            "the placeholder text must never become the key"
        );
        assert_eq!(layered.model, "compiled-model");
        let empty = resolve_layers_lenient(
            &LlmProviderConfig::new(""),
            &ENV,
            &DEFAULTS,
            &never_reasoning,
        );
        assert_eq!(empty.api_key, "");
    }

    #[test]
    fn the_overrides_travel_with_the_block() {
        let over = ModelOverride {
            context_window: Some(900),
            ..ModelOverride::default()
        };
        let block = LlmProviderConfig::new("k")
            .with_models(BTreeMap::from([("m".to_string(), over.clone())]));
        let layered = resolve_with(Some(&block), None, &env_of(&[])).unwrap();
        assert_eq!(layered.model_overrides.get("m"), Some(&over));
    }
}
