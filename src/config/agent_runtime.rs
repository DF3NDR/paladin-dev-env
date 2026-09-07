//! `AgentRuntimeConfig` -- the single X-09 config home for every built-in
//! execution middleware Phase 26 ships (Doc 05, D-10).
//!
//! Mirrors [`crate::config::node_cache::NodeCacheConfig`]'s shape
//! field-for-field (`Default` + `validate()` + `EnvOverridable`). Every one
//! of the twelve sub-structs below defaults to `enabled: false` (or, where a
//! section names a *policy* rather than a feature, to today's-behavior
//! value -- see [`ToolErrorConfig`]), so a v0.9 `config.yml` with no
//! `agent_runtime:` section boots v0.10 with **identical behavior**: X-09's
//! "new subsystems are disabled by default" requirement, asserted by
//! [`tests::default_agent_runtime_config_is_inert`] and backed at the
//! `Settings` boundary by MIGRATION §9.5's obligation.
//!
//! Scalar fields on each sub-struct are env-overridable under
//! `APP_AGENT_RUNTIME_<SECTION>_<FIELD>` (upper snake-case), mirroring
//! [`crate::config::env_utils`]. Regex rule lists ([`GuardrailConfig::rules`]),
//! per-tool maps ([`ToolCallLimitConfig::per_tool`], [`ToolErrorConfig::per_tool`])
//! and model context tables ([`HistoryTrimmerConfig::model_context_limits`])
//! are **config-file only** -- there is deliberately no environment-variable
//! form for a collection-shaped field, and [`tests::env_overrides_apply_to_scalar_fields_only`]
//! pins that no such variable has any effect.
//!
//! No field in this tree is secret-shaped: [`ModelFallbackConfig`] names
//! providers only (`providers: Vec<String>`, e.g. `"openai"`), never a
//! credential -- those keep coming from
//! `paladin_llm::provider_factory::LlmProviderFactory`'s existing
//! env/config path (D-12, D-41). Because nothing here can hold a secret,
//! this module deliberately does **not** apply the hand-written `Debug`
//! convention [`crate::config::node_cache::NodeCacheConfig`] uses for its
//! `redis_password` field -- every type below derives `Debug` plainly, and
//! [`tests::config_carries_no_secret_shaped_field`] pins the absence of any
//! key/secret/token/password-shaped field.
//!
//! `PaladinConfig` (`paladin_config.rs:44`, pre-existing, pub fields,
//! `Default`, builder) is **not** touched by this module or by any plan in
//! this phase (D-10, D-34, X-03).
//!
//! # A note on `build_chain`
//!
//! A facade helper `AgentRuntimeConfig::build_chain(&self, deps) ->
//! Result<Vec<Arc<dyn ExecutionMiddleware>>, ..>` will assemble every
//! enabled section into one ordered middleware chain once every built-in
//! middleware exists (landing in plan 26-20, after the other ten plans that
//! consume the sub-structs below). That function's **documented, fixed
//! assembly order** is recorded here now, in this module's rustdoc, so it
//! has exactly one written home rather than being re-derived when the
//! function is finally written:
//!
//! ```text
//! limits -> guardrail -> trimmer/summarizer -> recall -> protocol -> resilience
//! ```
//!
//! That is: [`ModelCallLimitConfig`] and [`TokenBudgetConfig`] and
//! [`ToolCallLimitConfig`] first, then [`GuardrailConfig`], then
//! [`HistoryTrimmerConfig`] and [`SummarizationConfig`], then
//! [`VaultRecallConfig`], then the tool-call protocol middleware (built in
//! plan 26-19, no config sub-struct of its own), and finally
//! [`ModelRetryConfig`] / [`ModelFallbackConfig`] last.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use paladin_core::platform::container::aegis::{RetryPolicy, RetryPredicate};
use paladin_llm::provider_factory::LlmProviderFactory;
use paladin_ports::output::llm_port::LlmPort;

use crate::config::env_utils::{EnvOverridable, read_env};

/// The single X-09 config home for every built-in execution middleware
/// (D-10). See the module-level documentation for the disabled-by-default
/// contract, the env-override scope and the deliberately-omitted `Debug`
/// redaction convention.
///
/// # Examples
///
/// ```
/// use paladin::config::agent_runtime::AgentRuntimeConfig;
///
/// let config = AgentRuntimeConfig::default();
/// assert!(!config.model_call_limit.enabled);
/// assert!(!config.token_budget.enabled);
/// assert!(config.validate().is_ok());
/// ```
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AgentRuntimeConfig {
    /// Caps the number of model calls the reasoning loop makes (D-08).
    #[serde(default)]
    pub model_call_limit: ModelCallLimitConfig,
    /// Caps the accumulated `total_tokens` a run may spend (D-08).
    #[serde(default)]
    pub token_budget: TokenBudgetConfig,
    /// Caps the number of tool (Armament/handoff) calls a run may make,
    /// overall and per named tool (D-08).
    #[serde(default)]
    pub tool_call_limit: ToolCallLimitConfig,
    /// Screens prompt and/or response text against a rule set (D-09).
    #[serde(default)]
    pub guardrail: GuardrailConfig,
    /// Bounds how much conversation history is kept in the assembled
    /// prompt (D-14, D-15).
    #[serde(default)]
    pub history_trimmer: HistoryTrimmerConfig,
    /// Compacts old history into a Garrison summary entry once a threshold
    /// is crossed (D-16).
    #[serde(default)]
    pub summarization: SummarizationConfig,
    /// Recalls long-term Vault memories into the prompt on the first loop
    /// iteration, best-effort (D-25).
    #[serde(default)]
    pub vault_recall: VaultRecallConfig,
    /// Retries a failed model call according to a policy mirroring
    /// [`paladin_core::platform::container::aegis::RetryPolicy`] (D-12).
    #[serde(default)]
    pub model_retry: ModelRetryConfig,
    /// Falls back across a named provider chain on a transient model
    /// failure (D-12).
    #[serde(default)]
    pub model_fallback: ModelFallbackConfig,
    /// Governs whether a failed tool call is fed back to the model or
    /// fails the run (D-33, D-34).
    #[serde(default)]
    pub tool_errors: ToolErrorConfig,
    /// Bounds the structured-output repair loop's retry count (D-26).
    #[serde(default)]
    pub structured: StructuredOutputConfig,
    /// Whether the built-in `vault_get`/`vault_put` Armaments are wired
    /// into a run's arsenal (D-22).
    #[serde(default)]
    pub vault_tools: VaultToolsConfig,
}

// `#[derive(Default)]` here (unlike `NodeCacheConfig`'s and
// `WaypointStoreConfig`'s hand-written `Default`, which pick non-default
// literal values such as a hostname or a key prefix): every field's
// disabled-by-default value already *is* that field's own type's
// `Default::default()`, so a derived impl and a hand-written one produce an
// identical, compiler-verified result -- clippy's `derivable_impls` lint
// enforces this. The disabled-by-default contract still lives in each
// sub-struct's own hand-written `Default` (see e.g. `ModelCallLimitConfig`
// below), colocated with its `validate()`.
impl AgentRuntimeConfig {
    /// Validates every sub-struct's own invariants. Never clamps, never
    /// panics -- each sub-struct's `validate()` returns a typed error
    /// naming the offending field (X-06, X-09).
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin::config::agent_runtime::{AgentRuntimeConfig, ModelCallLimitConfig};
    ///
    /// let mut config = AgentRuntimeConfig::default();
    /// assert!(config.validate().is_ok());
    ///
    /// config.model_call_limit = ModelCallLimitConfig {
    ///     enabled: true,
    ///     max_calls: 0,
    /// };
    /// assert!(config.validate().is_err());
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        self.model_call_limit.validate()?;
        self.token_budget.validate()?;
        self.tool_call_limit.validate()?;
        self.guardrail.validate()?;
        self.history_trimmer.validate()?;
        self.summarization.validate()?;
        self.vault_recall.validate()?;
        self.model_retry.validate()?;
        self.model_fallback.validate()?;
        self.tool_errors.validate()?;
        self.structured.validate()?;
        self.vault_tools.validate()?;
        Ok(())
    }
}

impl EnvOverridable for AgentRuntimeConfig {
    fn apply_env_overrides(&mut self) {
        self.model_call_limit.apply_env_overrides();
        self.token_budget.apply_env_overrides();
        self.tool_call_limit.apply_env_overrides();
        self.guardrail.apply_env_overrides();
        self.history_trimmer.apply_env_overrides();
        self.summarization.apply_env_overrides();
        self.vault_recall.apply_env_overrides();
        self.model_retry.apply_env_overrides();
        self.model_fallback.apply_env_overrides();
        self.tool_errors.apply_env_overrides();
        self.structured.apply_env_overrides();
        self.vault_tools.apply_env_overrides();
    }
}

// ── model_call_limit ────────────────────────────────────────────────────────

/// Caps the number of model calls the reasoning loop makes (D-08). Counts
/// once per `before_model`, post-retry -- the service's buffered retries
/// and the summarizer's own calls are not counted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelCallLimitConfig {
    /// Whether this limit is enforced at all. `false` out of the box.
    pub enabled: bool,
    /// The maximum number of model calls a single run may make, once
    /// `enabled`. Must be greater than `0` when `enabled`.
    pub max_calls: u32,
}

impl Default for ModelCallLimitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_calls: 10,
        }
    }
}

impl ModelCallLimitConfig {
    /// Rejects `max_calls == 0` when `enabled`; never clamps.
    pub fn validate(&self) -> Result<(), String> {
        if self.enabled && self.max_calls == 0 {
            return Err(
                "agent_runtime.model_call_limit.max_calls must be greater than 0 when enabled"
                    .to_string(),
            );
        }
        Ok(())
    }
}

impl EnvOverridable for ModelCallLimitConfig {
    fn apply_env_overrides(&mut self) {
        if let Some(v) = read_env::<bool>("APP_AGENT_RUNTIME_MODEL_CALL_LIMIT_ENABLED") {
            self.enabled = v;
        }
        if let Some(v) = read_env::<u32>("APP_AGENT_RUNTIME_MODEL_CALL_LIMIT_MAX_CALLS") {
            self.max_calls = v;
        }
    }
}

// ── token_budget ─────────────────────────────────────────────────────────────

/// Caps the accumulated `total_tokens` a run may spend (D-08). Accumulates
/// `response.usage.total_tokens` in `after_model`; the overshoot when the
/// budget is crossed is at most one response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenBudgetConfig {
    /// Whether this budget is enforced at all. `false` out of the box.
    pub enabled: bool,
    /// The maximum accumulated token count a single run may spend, once
    /// `enabled`. Must be greater than `0` when `enabled`.
    pub max_tokens: u32,
}

impl Default for TokenBudgetConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_tokens: 100_000,
        }
    }
}

impl TokenBudgetConfig {
    /// Rejects `max_tokens == 0` when `enabled`; never clamps.
    pub fn validate(&self) -> Result<(), String> {
        if self.enabled && self.max_tokens == 0 {
            return Err(
                "agent_runtime.token_budget.max_tokens must be greater than 0 when enabled"
                    .to_string(),
            );
        }
        Ok(())
    }
}

impl EnvOverridable for TokenBudgetConfig {
    fn apply_env_overrides(&mut self) {
        if let Some(v) = read_env::<bool>("APP_AGENT_RUNTIME_TOKEN_BUDGET_ENABLED") {
            self.enabled = v;
        }
        if let Some(v) = read_env::<u32>("APP_AGENT_RUNTIME_TOKEN_BUDGET_MAX_TOKENS") {
            self.max_tokens = v;
        }
    }
}

// ── tool_call_limit ──────────────────────────────────────────────────────────

/// Caps the number of tool (Armament/handoff) calls a run may make,
/// overall and per named tool (D-08). Denies with `ToolFlow::Deny` rather
/// than failing the run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCallLimitConfig {
    /// Whether this limit is enforced at all. `false` out of the box.
    pub enabled: bool,
    /// The overall maximum number of tool calls a single run may make,
    /// once `enabled`. Must be greater than `0` when `enabled`.
    pub max_calls: u32,
    /// Per-tool-name overrides of `max_calls`. **Config-file only** -- no
    /// environment variable can populate this map (see the module-level
    /// documentation).
    #[serde(default)]
    pub per_tool: HashMap<String, u32>,
}

impl Default for ToolCallLimitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_calls: 20,
            per_tool: HashMap::new(),
        }
    }
}

impl ToolCallLimitConfig {
    /// Rejects `max_calls == 0` when `enabled`; never clamps.
    pub fn validate(&self) -> Result<(), String> {
        if self.enabled && self.max_calls == 0 {
            return Err(
                "agent_runtime.tool_call_limit.max_calls must be greater than 0 when enabled"
                    .to_string(),
            );
        }
        Ok(())
    }
}

impl EnvOverridable for ToolCallLimitConfig {
    fn apply_env_overrides(&mut self) {
        if let Some(v) = read_env::<bool>("APP_AGENT_RUNTIME_TOOL_CALL_LIMIT_ENABLED") {
            self.enabled = v;
        }
        if let Some(v) = read_env::<u32>("APP_AGENT_RUNTIME_TOOL_CALL_LIMIT_MAX_CALLS") {
            self.max_calls = v;
        }
        // `per_tool` is config-file only (see module docs) -- no env var
        // reads into this map.
    }
}

// ── guardrail ────────────────────────────────────────────────────────────────

/// Which part of a model interaction a [`GuardrailRuleConfig`] screens
/// (D-09).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardrailTarget {
    /// Screen the rendered prompt sections in `before_model`.
    Prompt,
    /// Screen `response.content` in `after_model`.
    Response,
    /// Screen both.
    Both,
}

/// What a [`GuardrailRuleConfig`] does when its pattern matches (D-09).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum GuardrailOnMatch {
    /// Fail the run with `PaladinError::GuardrailTripped`.
    Fail,
    /// Replace every match with `replacement`.
    Redact {
        /// The replacement text.
        replacement: String,
    },
    /// Finish the run immediately with `message` as the output.
    Finish {
        /// The message to finish with.
        message: String,
    },
}

/// One config-supplied guardrail rule. The regex form only -- the
/// code-only `Predicate` matcher (D-09) has no config representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuardrailRuleConfig {
    /// A human-readable name for this rule, used in error messages and
    /// traces.
    pub name: String,
    /// Which part of the interaction this rule screens.
    pub target: GuardrailTarget,
    /// The regular expression pattern (compiled at construction, plan
    /// 26-08 -- never here).
    pub pattern: String,
    /// What to do when `pattern` matches.
    pub on_match: GuardrailOnMatch,
}

/// Screens prompt and/or response text against a rule set (D-09). Default
/// rule set is empty, per the PRD.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuardrailConfig {
    /// Whether the guardrail middleware is wired at all. `false` out of
    /// the box.
    pub enabled: bool,
    /// The configured rule set. **Config-file only** -- no environment
    /// variable can populate this list.
    #[serde(default)]
    pub rules: Vec<GuardrailRuleConfig>,
    /// The maximum byte length of any rule's `pattern` this config
    /// accepts. Not a ReDoS mitigation (the `regex` crate is
    /// linear-time and has no such surface) but a documented bound
    /// rather than an implicit reliance on `regex`'s internal size
    /// ceiling (RESEARCH.md Pitfall 5). Defaults to `65536` (`1 << 16`)
    /// bytes; enforced by the compile path in plan 26-08.
    pub pattern_size_limit_bytes: usize,
}

impl Default for GuardrailConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            rules: Vec::new(),
            pattern_size_limit_bytes: 1 << 16,
        }
    }
}

impl GuardrailConfig {
    /// Rejects a zero `pattern_size_limit_bytes`; never clamps.
    pub fn validate(&self) -> Result<(), String> {
        if self.pattern_size_limit_bytes == 0 {
            return Err(
                "agent_runtime.guardrail.pattern_size_limit_bytes must be greater than 0"
                    .to_string(),
            );
        }
        Ok(())
    }
}

impl EnvOverridable for GuardrailConfig {
    fn apply_env_overrides(&mut self) {
        if let Some(v) = read_env::<bool>("APP_AGENT_RUNTIME_GUARDRAIL_ENABLED") {
            self.enabled = v;
        }
        if let Some(v) = read_env::<usize>("APP_AGENT_RUNTIME_GUARDRAIL_PATTERN_SIZE_LIMIT_BYTES") {
            self.pattern_size_limit_bytes = v;
        }
        // `rules` is config-file only (see module docs) -- no env var
        // reads into this list.
    }
}

// ── history_trimmer ──────────────────────────────────────────────────────────

/// Bounds how much conversation history is kept in the assembled prompt
/// (D-14, D-15).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryTrimmerConfig {
    /// Whether the trimmer is wired at all. `false` out of the box.
    pub enabled: bool,
    /// Tokens reserved for the model's response, subtracted from the
    /// resolved context limit before history is admitted. Must be less
    /// than `default_context_tokens`.
    pub reserve_for_response: u32,
    /// The context-token limit used when a model's limit cannot be
    /// resolved from `model_context_limits` or the port's own
    /// `get_capabilities()` (D-14).
    pub default_context_tokens: u32,
    /// Per-model context-token overrides, consulted before the port's own
    /// `get_capabilities()`. **Config-file only** -- no environment
    /// variable can populate this map.
    #[serde(default)]
    pub model_context_limits: HashMap<String, u32>,
    /// Replaces the hard-coded `recall_recent(20)` call once the trimmer
    /// is installed.
    pub recall_limit: u32,
}

impl Default for HistoryTrimmerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            reserve_for_response: 1024,
            default_context_tokens: 8192,
            model_context_limits: HashMap::new(),
            recall_limit: 20,
        }
    }
}

impl HistoryTrimmerConfig {
    /// Rejects `reserve_for_response >= default_context_tokens` -- a
    /// trimmer that reserves at least the whole context budget could
    /// never admit any history. Never clamps.
    pub fn validate(&self) -> Result<(), String> {
        if self.reserve_for_response >= self.default_context_tokens {
            return Err(format!(
                "agent_runtime.history_trimmer.reserve_for_response ({}) must be less than \
                 default_context_tokens ({})",
                self.reserve_for_response, self.default_context_tokens
            ));
        }
        Ok(())
    }
}

impl EnvOverridable for HistoryTrimmerConfig {
    fn apply_env_overrides(&mut self) {
        if let Some(v) = read_env::<bool>("APP_AGENT_RUNTIME_HISTORY_TRIMMER_ENABLED") {
            self.enabled = v;
        }
        if let Some(v) = read_env::<u32>("APP_AGENT_RUNTIME_HISTORY_TRIMMER_RESERVE_FOR_RESPONSE") {
            self.reserve_for_response = v;
        }
        if let Some(v) = read_env::<u32>("APP_AGENT_RUNTIME_HISTORY_TRIMMER_DEFAULT_CONTEXT_TOKENS")
        {
            self.default_context_tokens = v;
        }
        if let Some(v) = read_env::<u32>("APP_AGENT_RUNTIME_HISTORY_TRIMMER_RECALL_LIMIT") {
            self.recall_limit = v;
        }
        // `model_context_limits` is config-file only (see module docs) --
        // no env var reads into this map.
    }
}

// ── summarization ────────────────────────────────────────────────────────────

/// Compacts old history into a Garrison summary entry once a threshold is
/// crossed (D-16).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SummarizationConfig {
    /// Whether the summarization middleware is wired at all. `false` out
    /// of the box.
    pub enabled: bool,
    /// Summarize once the effective history exceeds this many tokens (via
    /// the installed `TokenCounterPort`), if set.
    #[serde(default)]
    pub threshold_tokens: Option<u32>,
    /// Summarize once the effective history exceeds this many messages
    /// (the PRD acceptance figure).
    pub threshold_messages: u32,
    /// How many of the oldest raw entries beyond this count are folded
    /// into the summary.
    pub keep_recent: u32,
    /// The model used to produce the summary, if different from the
    /// service's own port and the Paladin's model.
    #[serde(default)]
    pub summarizer_model: Option<String>,
}

impl Default for SummarizationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold_tokens: None,
            threshold_messages: 30,
            keep_recent: 10,
            summarizer_model: None,
        }
    }
}

impl SummarizationConfig {
    /// Rejects `threshold_messages == 0` when `enabled`; never clamps.
    pub fn validate(&self) -> Result<(), String> {
        if self.enabled && self.threshold_messages == 0 {
            return Err(
                "agent_runtime.summarization.threshold_messages must be greater than 0 when \
                 enabled"
                    .to_string(),
            );
        }
        Ok(())
    }
}

impl EnvOverridable for SummarizationConfig {
    fn apply_env_overrides(&mut self) {
        if let Some(v) = read_env::<bool>("APP_AGENT_RUNTIME_SUMMARIZATION_ENABLED") {
            self.enabled = v;
        }
        if let Some(v) = read_env::<u32>("APP_AGENT_RUNTIME_SUMMARIZATION_THRESHOLD_TOKENS") {
            self.threshold_tokens = Some(v);
        }
        if let Some(v) = read_env::<u32>("APP_AGENT_RUNTIME_SUMMARIZATION_THRESHOLD_MESSAGES") {
            self.threshold_messages = v;
        }
        if let Some(v) = read_env::<u32>("APP_AGENT_RUNTIME_SUMMARIZATION_KEEP_RECENT") {
            self.keep_recent = v;
        }
        if let Some(v) = read_env::<String>("APP_AGENT_RUNTIME_SUMMARIZATION_SUMMARIZER_MODEL") {
            self.summarizer_model = Some(v);
        }
    }
}

// ── vault_recall ─────────────────────────────────────────────────────────────

/// Recalls long-term Vault memories into the prompt on the first loop
/// iteration, best-effort (D-25).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VaultRecallConfig {
    /// Whether the recall middleware is wired at all. `false` out of the
    /// box.
    pub enabled: bool,
    /// How many top-scoring records to request from the Vault's `search`.
    pub top_k: u32,
    /// Drop results scoring below this floor.
    pub score_floor: f32,
}

impl Default for VaultRecallConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            top_k: 5,
            score_floor: 0.0,
        }
    }
}

impl VaultRecallConfig {
    /// Rejects `top_k == 0` when `enabled`, and a `score_floor` outside
    /// `[0.0, 1.0]`; never clamps.
    pub fn validate(&self) -> Result<(), String> {
        if self.enabled && self.top_k == 0 {
            return Err(
                "agent_runtime.vault_recall.top_k must be greater than 0 when enabled".to_string(),
            );
        }
        if !(0.0..=1.0).contains(&self.score_floor) {
            return Err(
                "agent_runtime.vault_recall.score_floor must be between 0.0 and 1.0".to_string(),
            );
        }
        Ok(())
    }
}

impl EnvOverridable for VaultRecallConfig {
    fn apply_env_overrides(&mut self) {
        if let Some(v) = read_env::<bool>("APP_AGENT_RUNTIME_VAULT_RECALL_ENABLED") {
            self.enabled = v;
        }
        if let Some(v) = read_env::<u32>("APP_AGENT_RUNTIME_VAULT_RECALL_TOP_K") {
            self.top_k = v;
        }
        if let Some(v) = read_env::<f32>("APP_AGENT_RUNTIME_VAULT_RECALL_SCORE_FLOOR") {
            self.score_floor = v;
        }
    }
}

// ── model_retry ──────────────────────────────────────────────────────────────

/// Which classified errors a [`ModelRetryConfig`] retries -- mirrors
/// [`paladin_core::platform::container::aegis::RetryPredicate`]'s wire
/// form (D-12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryOnConfig {
    /// Retry only transient errors (the default).
    TransientOnly,
    /// Retry transient and unknown-classified errors.
    TransientAndUnknown,
}

/// Retries a failed model call according to a policy mirroring
/// [`paladin_core::platform::container::aegis::RetryPolicy`]'s fields and
/// `Default` values (D-12).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelRetryConfig {
    /// Whether retry is wired at all. `false` out of the box -- with no
    /// retry middleware the loop keeps today's shape byte-for-byte (X-03).
    pub enabled: bool,
    /// Maximum number of attempts (including the first). Must be at least
    /// `1`.
    pub max_attempts: u32,
    /// The delay before attempt 2, before any `backoff_factor` scaling, in
    /// milliseconds.
    pub initial_interval_ms: u64,
    /// The multiplier applied per additional attempt.
    pub backoff_factor: f64,
    /// The ceiling every computed delay is capped at, in milliseconds.
    pub max_interval_ms: u64,
    /// Whether to add random jitter to avoid a thundering-herd retry
    /// storm.
    pub jitter: bool,
    /// Which classified errors this policy retries.
    pub retry_on: RetryOnConfig,
}

impl Default for ModelRetryConfig {
    /// Mirrors `RetryPolicy::default()` (`aegis.rs:98`): `max_attempts: 3`,
    /// `initial_interval_ms: 500`, `backoff_factor: 2.0`,
    /// `max_interval_ms: 60_000`, `jitter: true`,
    /// `retry_on: TransientOnly`.
    fn default() -> Self {
        Self {
            enabled: false,
            max_attempts: 3,
            initial_interval_ms: 500,
            backoff_factor: 2.0,
            max_interval_ms: 60_000,
            jitter: true,
            retry_on: RetryOnConfig::TransientOnly,
        }
    }
}

impl ModelRetryConfig {
    /// Rejects `max_attempts == 0` -- never interpreted as unlimited
    /// retries, and never silently treated as a single attempt (mirrors
    /// `Aegis::validate`'s `RetryMaxAttemptsZero` rule).
    pub fn validate(&self) -> Result<(), String> {
        if self.max_attempts == 0 {
            return Err(
                "agent_runtime.model_retry.max_attempts must be at least 1, got 0".to_string(),
            );
        }
        Ok(())
    }
}

impl EnvOverridable for ModelRetryConfig {
    fn apply_env_overrides(&mut self) {
        if let Some(v) = read_env::<bool>("APP_AGENT_RUNTIME_MODEL_RETRY_ENABLED") {
            self.enabled = v;
        }
        if let Some(v) = read_env::<u32>("APP_AGENT_RUNTIME_MODEL_RETRY_MAX_ATTEMPTS") {
            self.max_attempts = v;
        }
        if let Some(v) = read_env::<u64>("APP_AGENT_RUNTIME_MODEL_RETRY_INITIAL_INTERVAL_MS") {
            self.initial_interval_ms = v;
        }
        if let Some(v) = read_env::<f64>("APP_AGENT_RUNTIME_MODEL_RETRY_BACKOFF_FACTOR") {
            self.backoff_factor = v;
        }
        if let Some(v) = read_env::<u64>("APP_AGENT_RUNTIME_MODEL_RETRY_MAX_INTERVAL_MS") {
            self.max_interval_ms = v;
        }
        if let Some(v) = read_env::<bool>("APP_AGENT_RUNTIME_MODEL_RETRY_JITTER") {
            self.jitter = v;
        }
        if let Some(v) = read_env::<String>("APP_AGENT_RUNTIME_MODEL_RETRY_RETRY_ON") {
            match v.to_ascii_lowercase().as_str() {
                "transient_only" => self.retry_on = RetryOnConfig::TransientOnly,
                "transient_and_unknown" => self.retry_on = RetryOnConfig::TransientAndUnknown,
                // Unparseable: leave the field at its prior value,
                // matching read_env's own silently-swallowed-parse-error
                // contract.
                _ => {}
            }
        }
    }
}

impl From<&ModelRetryConfig> for RetryPolicy {
    /// Maps all six mirrored fields 1:1 (D-12). `ModelRetryConfig::default()`
    /// converts to a `RetryPolicy` equal to `RetryPolicy::default()` field
    /// for field -- both mirror the exact same six values, so the two
    /// cannot drift (`tests::model_retry_config_maps_to_retry_policy_defaults`).
    fn from(config: &ModelRetryConfig) -> Self {
        RetryPolicy {
            max_attempts: config.max_attempts,
            initial_interval: Duration::from_millis(config.initial_interval_ms),
            backoff_factor: config.backoff_factor,
            max_interval: Duration::from_millis(config.max_interval_ms),
            jitter: config.jitter,
            retry_on: match config.retry_on {
                RetryOnConfig::TransientOnly => RetryPredicate::TransientOnly,
                RetryOnConfig::TransientAndUnknown => RetryPredicate::TransientAndUnknown,
            },
        }
    }
}

// ── model_fallback ───────────────────────────────────────────────────────────

/// Falls back across a named provider chain on a transient model failure
/// (D-12). Names providers only; the factory resolves them at
/// `build_chain` time. Credentials keep coming from
/// `LlmProviderFactory`'s existing env/config path, never from this
/// struct (D-12, D-41) -- see [`tests::config_carries_no_secret_shaped_field`].
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ModelFallbackConfig {
    /// Whether fallback is wired at all. `false` out of the box.
    pub enabled: bool,
    /// Provider **names** (e.g. `"openai"`, `"anthropic"`) resolved
    /// through `LlmProviderFactory::create` in provider order, forming
    /// the fallback chain. Never a credential.
    #[serde(default)]
    pub providers: Vec<String>,
}

impl ModelFallbackConfig {
    /// Rejects an empty `providers` list when `enabled`; never clamps.
    pub fn validate(&self) -> Result<(), String> {
        if self.enabled && self.providers.is_empty() {
            return Err(
                "agent_runtime.model_fallback.providers must not be empty when enabled".to_string(),
            );
        }
        Ok(())
    }
}

impl EnvOverridable for ModelFallbackConfig {
    fn apply_env_overrides(&mut self) {
        if let Some(v) = read_env::<bool>("APP_AGENT_RUNTIME_MODEL_FALLBACK_ENABLED") {
            self.enabled = v;
        }
        // `providers` is config-file only (see module docs) -- no env var
        // reads into this list.
    }
}

/// Every provider name `paladin_llm::provider_factory` can ever construct,
/// independent of which cargo features happen to be compiled into the
/// CURRENT build (D-12). Mirrors that crate's own registry declaration
/// order (`provider_factory.rs`'s `build_provider_registry`) and its own
/// `CONFIG_RECOGNISED_SPELLINGS` test list; a divergence between this list
/// and either of those is the defect class both exist to avoid.
///
/// [`ModelFallbackConfig::resolve_chain`] uses this list to distinguish an
/// outright-unknown provider name from a real provider whose feature is
/// simply not compiled into the CURRENT build -- a distinction
/// `LlmProviderFactory::create` itself cannot make, because a
/// feature-gated-off provider has no row in its registry at all and so is
/// indistinguishable, from inside that crate, from a name that was never a
/// provider (D-10's own "structurally absent" design in that crate).
const KNOWN_PROVIDER_NAMES: [&str; 9] = [
    "openai",
    "deepseek",
    "anthropic",
    "kimi",
    "qwen",
    "grok",
    "gemini",
    "openai-compatible",
    "ollama",
];

/// One provider name [`ModelFallbackConfig::resolve_chain`] could not
/// resolve, distinguishing three failure kinds an operator should not
/// confuse (D-12).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnresolvedProvider {
    /// `name` is not a provider name `paladin_llm` recognises at all --
    /// likely a typo.
    Unknown {
        /// The offending name, exactly as configured.
        name: String,
    },
    /// `name` is a real provider, but its cargo feature was not compiled
    /// into this build.
    NotCompiled {
        /// The offending name, exactly as configured.
        name: String,
    },
    /// `name` is a real, compiled-in provider, but
    /// `LlmProviderFactory::create` failed to construct it for another
    /// reason (typically a missing credential) -- NEVER misreported as
    /// `Unknown`/`NotCompiled`, because the name itself was recognised.
    ConstructionFailed {
        /// The offending name, exactly as configured.
        name: String,
        /// The underlying factory error's message.
        reason: String,
    },
}

impl fmt::Display for UnresolvedProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnresolvedProvider::Unknown { name } => write!(f, "unknown provider '{name}'"),
            UnresolvedProvider::NotCompiled { name } => {
                write!(f, "provider '{name}' is not compiled into this build")
            }
            UnresolvedProvider::ConstructionFailed { name, reason } => {
                write!(f, "provider '{name}' could not be constructed: {reason}")
            }
        }
    }
}

/// Errors from [`ModelFallbackConfig::resolve_chain`] (D-12).
#[derive(Debug, thiserror::Error)]
pub enum AgentRuntimeConfigError {
    /// One or more configured provider names could not be resolved.
    /// Collects EVERY offending name rather than stopping at the first, so
    /// an operator fixing a config learns about every typo in one pass.
    #[error(
        "agent_runtime.model_fallback.providers: {}",
        .0.iter().map(ToString::to_string).collect::<Vec<_>>().join(", ")
    )]
    UnresolvedProviders(Vec<UnresolvedProvider>),
}

impl ModelFallbackConfig {
    /// Resolve `providers` (in configured order) through `factory`, one
    /// [`paladin_ports::output::llm_port::LlmPort`] per name, forming the
    /// fallback chain [`crate::application::services::paladin::middleware::ModelFallbackMiddleware::new`]
    /// consumes (D-12). Returns an empty chain without touching `factory`
    /// at all when `!self.enabled` -- a disabled section installs nothing
    /// (mirrors every other built-in's `enabled`-first check, D-10).
    ///
    /// Collects EVERY unresolvable name into one
    /// [`AgentRuntimeConfigError::UnresolvedProviders`] rather than
    /// stopping at the first, distinguishing an outright-unknown name from
    /// a real provider whose cargo feature is not compiled into this
    /// build (via [`KNOWN_PROVIDER_NAMES`], a list independent of the
    /// CURRENT build's compiled features).
    ///
    /// Credentials are NEVER read from or stored in this struct (D-12,
    /// D-41): every resolved port's credential continues to come from
    /// `LlmProviderFactory`'s existing env/config path -- this method only
    /// ever passes a provider NAME to `factory.create`.
    ///
    /// Called from `AgentRuntimeConfig::build_chain` (plan 26-20); code
    /// composition (`ModelFallbackMiddleware::new(chain)` directly) remains
    /// the primary API, and this is the configuration path (D-12).
    pub fn resolve_chain(
        &self,
        factory: &LlmProviderFactory,
    ) -> Result<Vec<Arc<dyn LlmPort>>, AgentRuntimeConfigError> {
        if !self.enabled {
            return Ok(Vec::new());
        }

        let mut chain = Vec::with_capacity(self.providers.len());
        let mut problems = Vec::new();

        for name in &self.providers {
            match factory.create(name) {
                Ok(port) => chain.push(port),
                // Only `UnknownProvider` means "this name has no registry
                // row" -- which is ambiguous between "never a provider" and
                // "a provider, but its feature is off" until resolved
                // against `KNOWN_PROVIDER_NAMES`. Any OTHER factory error
                // (`ConfigurationMissing`, `AdapterCreationFailed`) means
                // the name WAS recognised and compiled in, but construction
                // failed for a different reason -- never misreported as
                // Unknown/NotCompiled.
                Err(paladin_llm::provider_factory::ProviderFactoryError::UnknownProvider(_)) => {
                    let normalized = name.to_lowercase().replace('_', "-");
                    if KNOWN_PROVIDER_NAMES.contains(&normalized.as_str()) {
                        problems.push(UnresolvedProvider::NotCompiled { name: name.clone() });
                    } else {
                        problems.push(UnresolvedProvider::Unknown { name: name.clone() });
                    }
                }
                Err(other) => problems.push(UnresolvedProvider::ConstructionFailed {
                    name: name.clone(),
                    reason: other.to_string(),
                }),
            }
        }

        if !problems.is_empty() {
            return Err(AgentRuntimeConfigError::UnresolvedProviders(problems));
        }
        Ok(chain)
    }
}

// ── tool_errors ──────────────────────────────────────────────────────────────

/// How a failed tool (Armament/handoff) call is surfaced to the model
/// (D-33, D-34).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolErrorMode {
    /// Feed the failure back into the model's context and continue --
    /// today's v0.9 behavior, unchanged (D-33).
    FeedToModel,
    /// Fail the run with a structured `PaladinError::ArmamentFailed`-style
    /// error. The new opt-in.
    FailRun,
}

/// Governs whether a failed tool call is fed back to the model or fails
/// the run (D-33, D-34). Default is [`ToolErrorMode::FeedToModel`] --
/// today's behavior, not a new default. This is the one section whose
/// default is a today's-behavior *policy* rather than `enabled: false`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolErrorConfig {
    /// The default mode applied to every tool call not named in
    /// `per_tool`.
    pub mode: ToolErrorMode,
    /// Per-tool-name overrides of `mode`. **Config-file only** -- no
    /// environment variable can populate this map.
    #[serde(default)]
    pub per_tool: HashMap<String, ToolErrorMode>,
}

impl Default for ToolErrorConfig {
    fn default() -> Self {
        Self {
            mode: ToolErrorMode::FeedToModel,
            per_tool: HashMap::new(),
        }
    }
}

impl ToolErrorConfig {
    /// Always valid -- an enum-typed mode and a per-tool override map have
    /// no invalid state to reject.
    pub fn validate(&self) -> Result<(), String> {
        Ok(())
    }
}

impl EnvOverridable for ToolErrorConfig {
    fn apply_env_overrides(&mut self) {
        if let Some(v) = read_env::<String>("APP_AGENT_RUNTIME_TOOL_ERRORS_MODE") {
            match v.to_ascii_lowercase().as_str() {
                "feed_to_model" => self.mode = ToolErrorMode::FeedToModel,
                "fail_run" => self.mode = ToolErrorMode::FailRun,
                // Unparseable: leave the field at its prior value,
                // matching read_env's own silently-swallowed-parse-error
                // contract.
                _ => {}
            }
        }
        // `per_tool` is config-file only (see module docs) -- no env var
        // reads into this map.
    }
}

// ── structured ───────────────────────────────────────────────────────────────

/// Bounds the structured-output repair loop's retry count (D-26).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructuredOutputConfig {
    /// How many re-prompt attempts `run_structured` makes after an
    /// initial parse/shape failure, before returning
    /// `PaladinError::StructuredOutputInvalid`.
    pub max_repair_attempts: u32,
}

impl Default for StructuredOutputConfig {
    fn default() -> Self {
        Self {
            max_repair_attempts: 1,
        }
    }
}

impl StructuredOutputConfig {
    /// Always valid -- any `u32` (including `0`, meaning "no repair
    /// attempts") is an acceptable value.
    pub fn validate(&self) -> Result<(), String> {
        Ok(())
    }
}

impl EnvOverridable for StructuredOutputConfig {
    fn apply_env_overrides(&mut self) {
        if let Some(v) = read_env::<u32>("APP_AGENT_RUNTIME_STRUCTURED_MAX_REPAIR_ATTEMPTS") {
            self.max_repair_attempts = v;
        }
    }
}

// ── vault_tools ──────────────────────────────────────────────────────────────

/// Whether the built-in `vault_get`/`vault_put` Armaments are wired into a
/// run's arsenal (D-22). Opt-in.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct VaultToolsConfig {
    /// Whether the vault tools are wired at all. `false` out of the box.
    pub enabled: bool,
}

impl VaultToolsConfig {
    /// Always valid -- a single bool has no invalid state to reject.
    pub fn validate(&self) -> Result<(), String> {
        Ok(())
    }
}

impl EnvOverridable for VaultToolsConfig {
    fn apply_env_overrides(&mut self) {
        if let Some(v) = read_env::<bool>("APP_AGENT_RUNTIME_VAULT_TOOLS_ENABLED") {
            self.enabled = v;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::{Config, File, FileFormat};
    use serial_test::serial;
    use std::env;

    /// A minimal `Settings`-shaped wrapper used to exercise
    /// `agent_runtime`'s deserialization behavior in isolation, without
    /// needing every other required `Settings` field populated.
    #[derive(Debug, Deserialize)]
    struct Wrapper {
        #[serde(default)]
        agent_runtime: AgentRuntimeConfig,
    }

    fn deserialize_wrapper(yaml: &str) -> Wrapper {
        Config::builder()
            .add_source(File::from_str(yaml, FileFormat::Yaml))
            .build()
            .expect("config should build from the inline yaml fixture")
            .try_deserialize()
            .expect("wrapper should deserialize")
    }

    /// Deserializes only the `agent_runtime:` key out of a tracked config
    /// file, ignoring every other field entirely (unlike
    /// `Settings::load_from_file`, which requires every provider block --
    /// including pre-existing, out-of-scope gaps such as
    /// `config.example.yml`'s deliberately-credential-less `llm.ollama`
    /// block -- to be present and well-typed). This isolates the
    /// `agent_runtime` pinning assertion from unrelated config schema
    /// drift.
    fn deserialize_wrapper_from_file(path: &str) -> Wrapper {
        Config::builder()
            .add_source(File::new(path, FileFormat::Yaml))
            .build()
            .unwrap_or_else(|e| panic!("{path} should build: {e}"))
            .try_deserialize()
            .unwrap_or_else(|e| panic!("{path}'s agent_runtime section should deserialize: {e}"))
    }

    /// Test 1: the default `AgentRuntimeConfig` is fully inert -- every
    /// sub-struct that has an `enabled` flag is `false`, every collection
    /// is empty, and `tool_errors.mode` is today's `FeedToModel` behavior
    /// (D-33).
    #[test]
    fn default_agent_runtime_config_is_inert() {
        let config = AgentRuntimeConfig::default();

        assert!(!config.model_call_limit.enabled);
        assert!(!config.token_budget.enabled);
        assert!(!config.tool_call_limit.enabled);
        assert!(config.tool_call_limit.per_tool.is_empty());
        assert!(!config.guardrail.enabled);
        assert!(config.guardrail.rules.is_empty());
        assert!(!config.history_trimmer.enabled);
        assert!(config.history_trimmer.model_context_limits.is_empty());
        assert!(!config.summarization.enabled);
        assert!(!config.vault_recall.enabled);
        assert!(!config.model_retry.enabled);
        assert!(!config.model_fallback.enabled);
        // This is the deliberate "today's-behavior policy" default, not an
        // `enabled: false` flag (D-33).
        assert_eq!(config.tool_errors.mode, ToolErrorMode::FeedToModel);
        assert!(!config.vault_tools.enabled);

        assert!(config.validate().is_ok());
    }

    /// Test 2: `agent_runtime: {}` deserializes to a value equal to
    /// `AgentRuntimeConfig::default()`.
    #[test]
    fn empty_agent_runtime_table_deserializes_to_default() {
        let wrapper = deserialize_wrapper("agent_runtime: {}\n");
        assert_eq!(wrapper.agent_runtime, AgentRuntimeConfig::default());
    }

    /// Test 3: a document with no `agent_runtime` key at all resolves the
    /// field to `AgentRuntimeConfig::default()`.
    #[test]
    fn absent_agent_runtime_section_deserializes_to_default() {
        let wrapper = deserialize_wrapper("{}\n");
        assert_eq!(wrapper.agent_runtime, AgentRuntimeConfig::default());
    }

    /// Test 4: an out-of-range or zero scalar on four representative
    /// sub-structs is rejected by `validate()`, each naming the offending
    /// field.
    #[test]
    fn validate_rejects_zero_and_out_of_range_scalars() {
        let config = ModelCallLimitConfig {
            enabled: true,
            max_calls: 0,
        };
        assert!(config.validate().unwrap_err().contains("max_calls"));

        let config = TokenBudgetConfig {
            enabled: true,
            max_tokens: 0,
        };
        assert!(config.validate().unwrap_err().contains("max_tokens"));

        let config = HistoryTrimmerConfig {
            reserve_for_response: u32::MAX,
            default_context_tokens: 8192,
            ..HistoryTrimmerConfig::default()
        };
        assert!(
            config
                .validate()
                .unwrap_err()
                .contains("reserve_for_response")
        );

        let config = ModelRetryConfig {
            max_attempts: 0,
            ..ModelRetryConfig::default()
        };
        assert!(config.validate().unwrap_err().contains("max_attempts"));
    }

    /// Test 5: `APP_AGENT_RUNTIME_TOKEN_BUDGET_*` env vars override their
    /// scalar fields; env vars named after a map/list-shaped field have no
    /// effect at all.
    #[test]
    #[serial]
    fn env_overrides_apply_to_scalar_fields_only() {
        unsafe {
            env::set_var("APP_AGENT_RUNTIME_TOKEN_BUDGET_ENABLED", "true");
            env::set_var("APP_AGENT_RUNTIME_TOKEN_BUDGET_MAX_TOKENS", "250");
            // These names follow the documented scheme but target
            // map/list-shaped fields -- no code reads them.
            env::set_var("APP_AGENT_RUNTIME_GUARDRAIL_RULES", "[]");
            env::set_var("APP_AGENT_RUNTIME_TOOL_CALL_LIMIT_PER_TOOL", "{}");
            env::set_var(
                "APP_AGENT_RUNTIME_HISTORY_TRIMMER_MODEL_CONTEXT_LIMITS",
                "{}",
            );
        }

        let mut config = AgentRuntimeConfig::default();
        config.apply_env_overrides();

        assert!(config.token_budget.enabled);
        assert_eq!(config.token_budget.max_tokens, 250);
        assert!(config.guardrail.rules.is_empty());
        assert!(config.tool_call_limit.per_tool.is_empty());
        assert!(config.history_trimmer.model_context_limits.is_empty());

        unsafe {
            env::remove_var("APP_AGENT_RUNTIME_TOKEN_BUDGET_ENABLED");
            env::remove_var("APP_AGENT_RUNTIME_TOKEN_BUDGET_MAX_TOKENS");
            env::remove_var("APP_AGENT_RUNTIME_GUARDRAIL_RULES");
            env::remove_var("APP_AGENT_RUNTIME_TOOL_CALL_LIMIT_PER_TOOL");
            env::remove_var("APP_AGENT_RUNTIME_HISTORY_TRIMMER_MODEL_CONTEXT_LIMITS");
        }
    }

    /// Test 6: `ModelFallbackConfig` -- the one sub-struct that names an
    /// external system -- exposes provider **names** only. Its field list
    /// is `enabled: bool` and `providers: Vec<String>`; neither name
    /// contains `key`, `secret`, `token` or `password`, and a real
    /// `Debug` rendering never surfaces one either.
    #[test]
    fn config_carries_no_secret_shaped_field() {
        let config = ModelFallbackConfig {
            enabled: true,
            providers: vec!["openai".to_string(), "anthropic".to_string()],
        };
        let rendered = format!("{config:?}").to_ascii_lowercase();
        for banned in ["key", "secret", "token", "password"] {
            assert!(
                !rendered.contains(banned),
                "ModelFallbackConfig's Debug output unexpectedly mentions '{banned}': \
                 {rendered}"
            );
        }
    }

    /// Test 7 (Task 2): loading the tracked `config.test.yml` (no
    /// `agent_runtime:` key) resolves every section to
    /// `AgentRuntimeConfig::default()` and `Settings::validate()`
    /// succeeds. This backs the narrowest honest form of MIGRATION §9.5's
    /// obligation for this phase: this plan adds no server-boot behavior,
    /// so the claim is exactly "a config file with no `agent_runtime:` key
    /// resolves every section to inert". The **full** v0.9-config boot
    /// test across every subsystem is SHIP-02 / Phase 29 scope -- not
    /// discharged here. Reads the tracked `config.test.yml` only, never a
    /// developer's untracked local `config.yml` (Phase 24's carried
    /// concern: that file is known not to deserialize).
    #[test]
    #[serial]
    fn v0_9_config_resolves_every_agent_runtime_section_inert() {
        let settings = crate::config::settings::Settings::load_from_file("config.test.yml")
            .expect("config.test.yml should load");
        assert_eq!(settings.agent_runtime, AgentRuntimeConfig::default());
        assert!(settings.validate().is_ok());
    }

    /// Test 8 (Task 2): the `agent_runtime:` block in the tracked
    /// `config.example.yml`, with every field written out explicitly,
    /// deserializes to a value equal to `AgentRuntimeConfig::default()` --
    /// so the documented example and the code default cannot silently
    /// drift apart.
    #[test]
    #[serial]
    fn example_config_agent_runtime_block_round_trips_to_default() {
        let wrapper = deserialize_wrapper_from_file("config.example.yml");
        assert_eq!(wrapper.agent_runtime, AgentRuntimeConfig::default());
    }

    /// Test 9 (Task 2): `AgentRuntimeConfig::default().validate()`
    /// succeeds, and so does every sub-struct's own
    /// `Default::default().validate()`.
    #[test]
    fn every_sub_struct_validates_under_its_own_default() {
        assert!(AgentRuntimeConfig::default().validate().is_ok());
        assert!(ModelCallLimitConfig::default().validate().is_ok());
        assert!(TokenBudgetConfig::default().validate().is_ok());
        assert!(ToolCallLimitConfig::default().validate().is_ok());
        assert!(GuardrailConfig::default().validate().is_ok());
        assert!(HistoryTrimmerConfig::default().validate().is_ok());
        assert!(SummarizationConfig::default().validate().is_ok());
        assert!(VaultRecallConfig::default().validate().is_ok());
        assert!(ModelRetryConfig::default().validate().is_ok());
        assert!(ModelFallbackConfig::default().validate().is_ok());
        assert!(ToolErrorConfig::default().validate().is_ok());
        assert!(StructuredOutputConfig::default().validate().is_ok());
        assert!(VaultToolsConfig::default().validate().is_ok());
    }

    // ── ModelFallbackConfig::resolve_chain (plan 26-10, D-12) ───────────

    /// Test 1: `{ enabled: true, providers: ["openai", "deepseek"] }`
    /// resolves to two ports, in that order. Both are default-compiled
    /// features, so this runs under both the default and `--all-features`
    /// builds; credentials come only from the factory's own env path
    /// (D-12) -- set here, under `#[serial]`, exactly like this module's
    /// own `env_overrides_apply_to_scalar_fields_only`.
    #[test]
    #[serial]
    fn resolve_chain_builds_ports_in_configured_order() {
        unsafe {
            env::set_var("OPENAI_API_KEY", "sk-test-key-for-resolve-chain");
            env::set_var("DEEPSEEK_API_KEY", "sk-test-key-for-resolve-chain");
        }

        let config = ModelFallbackConfig {
            enabled: true,
            providers: vec!["openai".to_string(), "deepseek".to_string()],
        };
        let factory = LlmProviderFactory::new();
        let result = config.resolve_chain(&factory);

        unsafe {
            env::remove_var("OPENAI_API_KEY");
            env::remove_var("DEEPSEEK_API_KEY");
        }

        let chain = result.expect("both providers are compiled and credentialed");
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].get_provider_name(), "openai");
        assert_eq!(chain[1].get_provider_name(), "deepseek");
    }

    /// Test 2: two entirely bogus names both surface in the SAME typed
    /// error, not just the first (D-12: an operator learns about every
    /// typo at once).
    #[test]
    fn unknown_provider_is_a_typed_error_listing_every_offender() {
        let config = ModelFallbackConfig {
            enabled: true,
            providers: vec![
                "totally-bogus-one".to_string(),
                "totally-bogus-two".to_string(),
            ],
        };
        let factory = LlmProviderFactory::new();
        // `Vec<Arc<dyn LlmPort>>` is not `Debug` (a trait object), so
        // `.unwrap_err()` cannot be used here -- match instead.
        let err = match config.resolve_chain(&factory) {
            Err(e) => e,
            Ok(_) => panic!("two bogus provider names must not resolve"),
        };

        let AgentRuntimeConfigError::UnresolvedProviders(problems) = &err;
        assert_eq!(problems.len(), 2);
        assert!(matches!(
            &problems[0],
            UnresolvedProvider::Unknown { name } if name == "totally-bogus-one"
        ));
        assert!(matches!(
            &problems[1],
            UnresolvedProvider::Unknown { name } if name == "totally-bogus-two"
        ));
        let message = err.to_string();
        assert!(message.contains("totally-bogus-one"), "{message}");
        assert!(message.contains("totally-bogus-two"), "{message}");
    }

    /// Test 3: `"ollama"` is a REAL provider name, but this crate's default
    /// feature set (`llm-openai`, `llm-anthropic`, `llm-deepseek`) does not
    /// compile it in -- reported as `NotCompiled`, never `Unknown`.
    /// Deliberately run under the DEFAULT feature set, not
    /// `--all-features`: under `--all-features` every `KNOWN_PROVIDER_NAMES`
    /// entry is compiled, so no name can ever exercise this branch (there
    /// would be nothing left "not compiled" to name) -- corrected from the
    /// plan's stated `--all-features` invocation for this one test
    /// (deviation, Rule 3).
    #[cfg(not(feature = "llm-ollama"))]
    #[test]
    fn uncompiled_provider_is_reported_distinctly_from_unknown() {
        let config = ModelFallbackConfig {
            enabled: true,
            providers: vec!["ollama".to_string()],
        };
        let factory = LlmProviderFactory::new();
        let err = match config.resolve_chain(&factory) {
            Err(e) => e,
            Ok(_) => panic!("ollama must not resolve when its feature is not compiled in"),
        };

        let AgentRuntimeConfigError::UnresolvedProviders(problems) = &err;
        assert_eq!(
            problems,
            &vec![UnresolvedProvider::NotCompiled {
                name: "ollama".to_string()
            }]
        );
        assert!(err.to_string().contains("not compiled into this build"));
    }

    /// Test 4: `enabled: false` (the default) resolves to no chain and
    /// never touches the factory -- an invalid/unresolvable `providers`
    /// list has zero effect while disabled.
    #[test]
    fn disabled_config_resolves_to_no_chain() {
        let config = ModelFallbackConfig {
            enabled: false,
            providers: vec!["this-would-be-unresolvable".to_string()],
        };
        let factory = LlmProviderFactory::new();
        // `Vec<Arc<dyn LlmPort>>` is not `Debug`/`PartialEq` (a trait
        // object), so `assert_eq!` against `Vec::new()` cannot be used --
        // `.is_empty()` instead.
        match config.resolve_chain(&factory) {
            Ok(chain) => assert!(chain.is_empty()),
            Err(e) => panic!("a disabled config must never fail to resolve: {e}"),
        }
    }

    /// Test 5: resolution reads credentials from the factory's existing
    /// env/config path only -- `ModelFallbackConfig` itself contributes no
    /// credential, and a resolved port still works when the ONLY source of
    /// its credential is that env var.
    #[test]
    #[serial]
    fn config_holds_no_credential() {
        unsafe {
            env::set_var("OPENAI_API_KEY", "sk-test-key-for-resolve-chain");
        }

        let config = ModelFallbackConfig {
            enabled: true,
            providers: vec!["openai".to_string()],
        };
        let factory = LlmProviderFactory::new();
        let result = config.resolve_chain(&factory);

        unsafe {
            env::remove_var("OPENAI_API_KEY");
        }

        let chain = result.expect("openai resolves once its credential is in the environment");
        assert_eq!(chain.len(), 1);
        assert_eq!(chain[0].get_provider_name(), "openai");
    }

    /// Test 6: `ModelRetryConfig::default()` converts to a `RetryPolicy`
    /// equal to `RetryPolicy::default()`, field for field.
    #[test]
    fn model_retry_config_maps_to_retry_policy_defaults() {
        let converted: RetryPolicy = (&ModelRetryConfig::default()).into();
        assert_eq!(converted, RetryPolicy::default());
    }
}
