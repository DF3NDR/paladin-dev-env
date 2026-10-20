//! The per-model profile registry (ADR-0061 (c), D-05..D-07): which [`ModelProfile`] applies to
//! which model of one provider, and how that was decided.
//!
//! # Precedence
//!
//! For a model id, **by exact match only** (a seed for `kimi-k3` does not match
//! `kimi-k3-preview`):
//!
//! 1. a [`ModelOverride`] from configuration, applied **field by field** over
//! 2. the seeded entry for that model, if one exists, else over
//! 3. the adapter-wide fallback profile.
//!
//! A model with neither a seed nor an override runs on the adapter-wide fallback and is *visibly*
//! unprofiled: the registry logs one `warn!` (target `paladin::registry`) per instance per model
//! and the adapter stamps [`UNPROFILED_MODEL_METADATA_KEY`] on the response. An unknown model is
//! never a refusal (D-07) -- a deployment must be able to point at a model released yesterday.
//!
//! A [`ModelOverride`] is an explicit operator statement, so it is validated hard: finite,
//! ordered, non-zero, with a maximum output inside the context window, unknown keys rejected.
//! The merged result is re-validated too, so an override that is fine alone but contradicts the
//! seeded value it is laid over (a maximum output above the seeded window) is refused at
//! construction, naming provider and model.
// The registry is consumed by the compat engine and by the four bespoke adapters (OpenAI,
// DeepSeek, Anthropic, Gemini); without a consuming adapter feature its crate-private items are
// legitimately unused.
#![cfg_attr(
    not(any(
        feature = "kimi",
        feature = "qwen",
        feature = "grok",
        feature = "ollama",
        feature = "openai-compatible",
        feature = "openai",
        feature = "anthropic",
        feature = "deepseek",
        feature = "gemini"
    )),
    allow(dead_code)
)]

pub(crate) mod seeded;

use std::collections::{BTreeMap, HashSet};
use std::sync::Mutex;

use paladin_ports::output::model_profile::{ModelProfile, ParamSupport, StopSupport};
use serde::{Deserialize, Serialize};

/// The `LlmResponse.metadata` key under which an adapter records that the request's model has
/// neither a seeded entry nor an override, so the adapter-wide fallback profile applied. The
/// value is the model id.
///
/// Absent when the model was profiled.
pub const UNPROFILED_MODEL_METADATA_KEY: &str = "paladin.unprofiled_model";

/// How many times an adapter retries a retryable error after the first attempt when its config
/// does not say otherwise: the value every adapter hard-coded before PARITY-03 made it a knob.
pub(crate) const DEFAULT_MAX_RETRIES: u32 = 3;

/// The request timeout, in seconds, a direct `*Config` constructor gives a reasoning model when
/// no explicit timeout is set (D-13): a reasoning model spends its budget thinking before it
/// answers, so the 60 s hosted-preset default routinely cuts it off. Also the value the layered
/// config path (`config::layers`) gives a reasoning model, so it is not feature-gated.
pub(crate) const REASONING_DEFAULT_TIMEOUT_SECONDS: u64 = 300;

/// The seeded table for `provider`, empty for a provider that has none (or whose feature is
/// not compiled in).
#[cfg(any(
    feature = "kimi",
    feature = "qwen",
    feature = "grok",
    feature = "ollama",
    feature = "openai-compatible",
    feature = "anthropic",
    feature = "deepseek",
    feature = "gemini"
))]
fn seeded_table(provider: &str) -> &'static [SeededProfile] {
    match provider {
        #[cfg(feature = "kimi")]
        "kimi" => seeded::KIMI,
        #[cfg(feature = "qwen")]
        "qwen" => seeded::QWEN,
        #[cfg(feature = "grok")]
        "grok" => seeded::GROK,
        #[cfg(feature = "ollama")]
        "ollama" => seeded::OLLAMA,
        #[cfg(feature = "anthropic")]
        "anthropic" => seeded::ANTHROPIC,
        #[cfg(feature = "deepseek")]
        "deepseek" => seeded::DEEPSEEK,
        #[cfg(feature = "gemini")]
        "gemini" => seeded::GEMINI,
        _ => &[],
    }
}

/// Whether `model` is a reasoning model on `provider`: the `reasoning` field of its
/// [`ModelOverride`] when one sets it, else its seeded entry's, else `false`. Exact match on the
/// model id, like every other registry lookup.
///
/// The direct `*Config` constructors use this to default a reasoning model's timeout to
/// [`REASONING_DEFAULT_TIMEOUT_SECONDS`]; the layered config path reuses it with the overrides it
/// has resolved.
#[cfg(any(
    feature = "kimi",
    feature = "qwen",
    feature = "grok",
    feature = "ollama",
    feature = "openai-compatible",
    feature = "anthropic",
    feature = "deepseek",
    feature = "gemini"
))]
pub(crate) fn is_reasoning_model(
    provider: &str,
    model: &str,
    overrides: &BTreeMap<String, ModelOverride>,
) -> bool {
    if let Some(reasoning) = overrides.get(model).and_then(|over| over.reasoning) {
        return reasoning;
    }
    seeded_table(provider)
        .iter()
        .find(|entry| entry.model == model)
        .is_some_and(|entry| (entry.build)().reasoning)
}

/// The default request timeout a direct `*Config` constructor gives `model` when no explicit
/// timeout is set (D-13): [`REASONING_DEFAULT_TIMEOUT_SECONDS`] for a seeded reasoning model,
/// `compiled_default` (the provider's own long-standing default) for any other.
#[cfg(any(
    feature = "kimi",
    feature = "qwen",
    feature = "grok",
    feature = "ollama",
    feature = "openai-compatible",
    feature = "anthropic",
    feature = "deepseek",
    feature = "gemini"
))]
pub(crate) fn default_timeout_seconds(provider: &str, model: &str, compiled_default: u64) -> u64 {
    if is_reasoning_model(provider, model, &BTreeMap::new()) {
        REASONING_DEFAULT_TIMEOUT_SECONDS
    } else {
        compiled_default
    }
}

/// Generate the `with_*` builders of the shared knobs on one `*Config` type (D-12).
///
/// Every adapter config carries the same knobs -- `max_tokens`, `max_retries`,
/// `context_window`, `model_overrides`, and (Phase 50.1, SURFACE-02) the `base_url` and
/// `timeout_seconds` a constructor defaults -- so their builders (and rustdoc) come from one
/// place and cannot drift between the nine types. The one-argument form is for a config whose
/// `max_tokens` is an `Option<u32>`; `plain_max_tokens` is for Anthropic, whose `max_tokens` is a
/// required `u32`. The `organization` form is for OpenAI: the one-argument form plus
/// `with_organization`, the one knob only that config carries (50.1-05; one macro, no second
/// builder mechanism).
#[allow(unused_macros)]
macro_rules! config_knob_builders {
    ($config:ident) => {
        impl $config {
            /// Set the default output budget (`max_tokens`) sent when a request sets none.
            ///
            /// The value passes through the same maximum-output check as a request's own
            /// `max_tokens` (ADR-0061 (i)): above the model's profiled maximum, the request is
            /// refused before any HTTP call.
            #[must_use]
            pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
                self.max_tokens = Some(max_tokens);
                self
            }

            $crate::registry::config_knob_builders!(@common $config);
        }
    };
    ($config:ident, plain_max_tokens) => {
        impl $config {
            /// Set the output budget (`max_tokens`) sent when a request sets none.
            ///
            /// The value passes through the same maximum-output check as a request's own
            /// `max_tokens` (ADR-0061 (i)): above the model's profiled maximum, the request is
            /// refused before any HTTP call.
            #[must_use]
            pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
                self.max_tokens = max_tokens;
                self
            }

            $crate::registry::config_knob_builders!(@common $config);
        }
    };
    ($config:ident, organization) => {
        impl $config {
            /// Set the default output budget (`max_tokens`) sent when a request sets none.
            ///
            /// The value passes through the same maximum-output check as a request's own
            /// `max_tokens` (ADR-0061 (i)): above the model's profiled maximum, the request is
            /// refused before any HTTP call.
            #[must_use]
            pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
                self.max_tokens = Some(max_tokens);
                self
            }

            /// Set the organisation, sent as the `OpenAI-Organization` header value.
            ///
            /// [`from_env`](Self::from_env) reads the same value from `OPENAI_ORGANIZATION`.
            /// The only knob that is specific to OpenAI: no other provider carries an
            /// organisation.
            #[must_use]
            pub fn with_organization(mut self, organization: impl Into<String>) -> Self {
                self.organization = Some(organization.into());
                self
            }

            $crate::registry::config_knob_builders!(@common $config);
        }
    };
    (@common $config:ident) => {
        /// Set the endpoint, replacing the vendor default [`new`](Self::new) chose.
        ///
        /// The credential header goes wherever this points, so point it only at an endpoint you
        /// trust: a gateway or proxy you operate, or the vendor's own host. Redirects are never
        /// followed, so a `3xx` answer cannot carry the credential on to another host. The value
        /// is checked to be an `http` or `https` URL when the adapter is built, and an empty or
        /// other-scheme value fails construction.
        #[must_use]
        pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
            self.base_url = base_url.into();
            self
        }

        /// Set the request timeout, in seconds, replacing the default [`new`](Self::new) chose.
        ///
        /// That default is 300 s for a seeded reasoning model and the provider's own default
        /// otherwise (Phase 50 D-13); a value set here always wins over it, whichever order the
        /// builders are called in and however often this one is called (the last call wins).
        #[must_use]
        pub fn with_timeout_seconds(mut self, timeout_seconds: u64) -> Self {
            self.timeout_seconds = timeout_seconds;
            self
        }

        /// Set the retry budget for a retryable error (network failure, timeout, 5xx).
        ///
        /// The compat presets, OpenAI and DeepSeek retry up to this many times after the first
        /// attempt (`0` sends once). Anthropic and Gemini keep their long-standing loop, which
        /// counts total attempts (`0` and `1` both send once, `3` sends up to three); the
        /// counter conventions are not unified yet. A rate-limit answer (429) is never retried
        /// here, because the Cadence decorator owns waiting, and a permanent error is never
        /// retried.
        #[must_use]
        pub fn with_max_retries(mut self, max_retries: u32) -> Self {
            self.max_retries = max_retries;
            self
        }

        /// Set the adapter-wide context window, in tokens.
        ///
        /// The operator's statement for every model of this provider: it replaces the adapter's
        /// declared window, the fallback profile's window and every seeded model's window. A
        /// window in [`model_overrides`](Self::model_overrides) for a given model still wins.
        /// Construction fails when the window is below a model's maximum output.
        #[must_use]
        pub fn with_context_window(mut self, context_window: u32) -> Self {
            self.context_window = Some(context_window);
            self
        }

        /// Set the per-model overrides (D-06), keyed by exact model id.
        ///
        /// Each override is laid over that model's seeded entry (or the adapter-wide fallback)
        /// field by field. They are validated when the adapter is built; an invalid one fails
        /// construction with an error naming the provider and the model.
        ///
        /// A model marked `reasoning` only here does not get the 300 s reasoning timeout from
        /// the direct constructors; set `timeout_seconds` in the same place.
        #[must_use]
        pub fn with_model_overrides(
            mut self,
            model_overrides: std::collections::BTreeMap<String, $crate::registry::ModelOverride>,
        ) -> Self {
            self.model_overrides = model_overrides;
            self
        }
    };
}
#[allow(unused_imports)]
pub(crate) use config_knob_builders;

/// An operator's per-model statement, laid over the seeded entry (or the adapter-wide fallback)
/// field by field: a field left `None` keeps the value underneath (D-06).
///
/// Unknown keys are rejected (`deny_unknown_fields`) so a misspelt limit fails at load instead
/// of being silently inert. Integer fields are `u32`, so a value past `u32::MAX` fails to load
/// with an error naming the field rather than wrapping.
///
/// # Examples
///
/// ```
/// use paladin_llm::registry::ModelOverride;
/// use paladin_ports::output::model_profile::ParamSupport;
///
/// let mut over = ModelOverride::default();
/// over.temperature = Some(ParamSupport::Fixed(1.0));
/// over.max_output_tokens = Some(8192);
/// assert!(over.validate().is_ok());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct ModelOverride {
    /// The model's context window in tokens.
    pub context_window: Option<u32>,
    /// The most tokens the model can generate in one response.
    pub max_output_tokens: Option<u32>,
    /// What the model accepts for `temperature`.
    pub temperature: Option<ParamSupport>,
    /// What the model accepts for `top_p`.
    pub top_p: Option<ParamSupport>,
    /// What the model accepts for `frequency_penalty`.
    pub frequency_penalty: Option<ParamSupport>,
    /// What the model accepts for `presence_penalty`.
    pub presence_penalty: Option<ParamSupport>,
    /// What the model accepts for stop sequences.
    pub stop_sequences: Option<StopSupport>,
    /// Whether the model is a reasoning model.
    pub reasoning: Option<bool>,
    /// Whether the model supports a JSON-object `response_format`.
    pub json_object: Option<bool>,
    /// Whether the model supports a JSON-schema `response_format`.
    pub json_schema: Option<bool>,
}

impl ModelOverride {
    /// Check this override on its own.
    ///
    /// Rejects: a zero context window; a zero maximum output; a maximum output above the context
    /// window (when both are set here); a `Range` with `lo > hi`; any non-finite bound or fixed
    /// value; and `Supported { max: Some(0) }` stop sequences.
    ///
    /// # Errors
    ///
    /// A message naming the offending field.
    pub fn validate(&self) -> Result<(), String> {
        validate_limits(self.context_window, self.max_output_tokens)?;
        let numeric = [
            ("temperature", self.temperature),
            ("top_p", self.top_p),
            ("frequency_penalty", self.frequency_penalty),
            ("presence_penalty", self.presence_penalty),
        ];
        for (field, support) in numeric {
            if let Some(support) = support {
                validate_support(field, support)?;
            }
        }
        if let Some(stop) = self.stop_sequences {
            validate_stop(stop)?;
        }
        Ok(())
    }

    /// Lay this override over `base`: every field set here replaces the base value, every field
    /// left `None` keeps it.
    #[must_use]
    pub fn apply_to(&self, base: ModelProfile) -> ModelProfile {
        let mut profile = base;
        if let Some(window) = self.context_window {
            profile = profile.with_context_window(window);
        }
        if let Some(max) = self.max_output_tokens {
            profile = profile.with_max_output_tokens(max);
        }
        if let Some(support) = self.temperature {
            profile = profile.with_temperature(support);
        }
        if let Some(support) = self.top_p {
            profile = profile.with_top_p(support);
        }
        if let Some(support) = self.frequency_penalty {
            profile = profile.with_frequency_penalty(support);
        }
        if let Some(support) = self.presence_penalty {
            profile = profile.with_presence_penalty(support);
        }
        if let Some(stop) = self.stop_sequences {
            profile = profile.with_stop_sequences(stop);
        }
        if let Some(reasoning) = self.reasoning {
            profile = profile.with_reasoning(reasoning);
        }
        if let Some(supported) = self.json_object {
            profile = profile.with_json_object(supported);
        }
        if let Some(supported) = self.json_schema {
            profile = profile.with_json_schema(supported);
        }
        profile
    }
}

/// Window and maximum-output sanity shared by an override and a merged profile.
fn validate_limits(window: Option<u32>, max_output: Option<u32>) -> Result<(), String> {
    if window == Some(0) {
        return Err("context_window must be greater than zero".to_string());
    }
    if max_output == Some(0) {
        return Err("max_output_tokens must be greater than zero".to_string());
    }
    if let (Some(window), Some(max_output)) = (window, max_output)
        && max_output > window
    {
        return Err(format!(
            "max_output_tokens ({max_output}) must not exceed context_window ({window})"
        ));
    }
    Ok(())
}

/// One numeric support value: finite bounds, ordered range.
fn validate_support(field: &str, support: ParamSupport) -> Result<(), String> {
    match support {
        ParamSupport::Range(lo, hi) => {
            if !lo.is_finite() || !hi.is_finite() {
                return Err(format!(
                    "{field}: range bounds must be finite numbers, got {lo} to {hi}"
                ));
            }
            if lo > hi {
                return Err(format!(
                    "{field}: range lower bound {lo} is above its upper bound {hi}"
                ));
            }
        }
        ParamSupport::Fixed(value) => {
            if !value.is_finite() {
                return Err(format!("{field}: fixed value must be finite, got {value}"));
            }
        }
        ParamSupport::NotSent => {}
    }
    Ok(())
}

/// Stop-sequence support: a stated maximum of zero admits nothing, which is `not_sent`.
fn validate_stop(stop: StopSupport) -> Result<(), String> {
    if matches!(stop, StopSupport::Supported { max: Some(0) }) {
        return Err(
            "stop_sequences: a maximum of 0 admits no stop sequence; use not_sent instead"
                .to_string(),
        );
    }
    Ok(())
}

/// The sanity rules applied to a fully merged profile.
fn validate_profile(profile: &ModelProfile) -> Result<(), String> {
    validate_limits(profile.context_window, profile.max_output_tokens)?;
    validate_support("temperature", profile.temperature)?;
    validate_support("top_p", profile.top_p)?;
    validate_support("frequency_penalty", profile.frequency_penalty)?;
    validate_support("presence_penalty", profile.presence_penalty)?;
    validate_stop(profile.stop_sequences)
}

/// One dated, verified per-model entry in a vendor's seeded table (filled from plan 50-06).
#[derive(Debug, Clone, Copy)]
pub(crate) struct SeededProfile {
    /// The exact model id this entry describes.
    pub model: &'static str,
    /// The date (`YYYY-MM-DD`) the entry was last checked against the vendor.
    pub verified_on: &'static str,
    /// Builds the entry's profile. A function, not a value, so a static table can hold it.
    pub build: fn() -> ModelProfile,
}

/// The context window a seeded table states for `model`, or `None` when the table does not list
/// the model or states no window for it.
///
/// An adapter's declared `max_context_tokens` is read through this from its default model's
/// entry, so the capability declaration and the dated table cannot drift apart.
#[cfg(any(
    feature = "anthropic",
    feature = "deepseek",
    feature = "gemini",
    feature = "kimi",
    feature = "qwen",
    feature = "grok"
))]
pub(crate) fn seeded_window(table: &[SeededProfile], model: &str) -> Option<u32> {
    table
        .iter()
        .find(|entry| entry.model == model)
        .and_then(|entry| (entry.build)().context_window)
}

/// Where a resolved profile came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProfileOrigin {
    /// A seeded entry, untouched.
    Seeded {
        /// The entry's verification date.
        verified_on: &'static str,
    },
    /// A config override laid over the adapter-wide fallback (no seeded entry exists).
    Override,
    /// A config override laid over a seeded entry.
    SeededWithOverride {
        /// The underlying seeded entry's verification date.
        verified_on: &'static str,
    },
    /// Neither a seed nor an override: the adapter-wide fallback applies (D-07).
    AdapterDefault,
}

/// One provider's registry: seeded entries, config overrides and the adapter-wide fallback.
///
/// Lives on each adapter instance. Resolution is pure and exact-match; the only state is the
/// once-per-model "already warned" set behind [`ModelRegistry::note_unprofiled`].
#[derive(Debug)]
pub(crate) struct ModelRegistry {
    provider: &'static str,
    seeded: &'static [SeededProfile],
    overrides: BTreeMap<String, ModelOverride>,
    /// The operator's adapter-wide window ([`ModelRegistry::with_context_window`]).
    window: Option<u32>,
    fallback: ModelProfile,
    warned: Mutex<HashSet<String>>,
}

impl ModelRegistry {
    /// A registry for `provider` over a seeded table and an adapter-wide fallback, with no
    /// overrides.
    pub(crate) fn new(
        provider: &'static str,
        seeded: &'static [SeededProfile],
        fallback: ModelProfile,
    ) -> Self {
        Self {
            provider,
            seeded,
            overrides: BTreeMap::new(),
            window: None,
            fallback,
            warned: Mutex::new(HashSet::new()),
        }
    }

    /// Declare the operator's adapter-wide context window (D-13, the most specific statement
    /// wins): when `Some`, it replaces the window of the fallback and of every seeded entry at
    /// resolve time. A per-model override's window still wins over it.
    ///
    /// # Errors
    ///
    /// A message naming the provider, the model and both numbers when the window is zero or
    /// below a model's maximum output. Never clamps.
    pub(crate) fn with_context_window(mut self, window: Option<u32>) -> Result<Self, String> {
        self.window = window;
        let Some(window) = window else {
            return Ok(self);
        };
        if window == 0 {
            return Err(format!(
                "invalid context_window on provider '{}': it must be greater than zero",
                self.provider
            ));
        }
        // The fallback and every seeded entry now carry the operator's window; each must still
        // be coherent with the maximum output it declares.
        let fallback = self.window_applied(self.fallback.clone());
        validate_limits(fallback.context_window, fallback.max_output_tokens).map_err(|problem| {
            format!(
                "invalid context_window {window} on provider '{}' (adapter-wide fallback): {problem}",
                self.provider
            )
        })?;
        for entry in self.seeded {
            let merged = self.base_for(entry.model).0;
            validate_limits(merged.context_window, merged.max_output_tokens).map_err(
                |problem| {
                    format!(
                        "invalid context_window {window} for model '{}' on provider '{}': {problem}",
                        entry.model, self.provider
                    )
                },
            )?;
        }
        Ok(self)
    }

    /// `profile` with the operator's adapter-wide window laid over it, when one was declared.
    fn window_applied(&self, profile: ModelProfile) -> ModelProfile {
        match self.window {
            Some(window) => profile.with_context_window(window),
            None => profile,
        }
    }

    /// Attach per-model overrides, validating each one and the merged result it produces.
    ///
    /// # Errors
    ///
    /// A message naming the provider, the model and the problem: an empty model id, an invalid
    /// override, or an override that contradicts the profile it is laid over.
    pub(crate) fn with_overrides(
        mut self,
        overrides: BTreeMap<String, ModelOverride>,
    ) -> Result<Self, String> {
        for (model, over) in &overrides {
            let named = |problem: String| {
                format!(
                    "invalid override for model '{model}' on provider '{}': {problem}",
                    self.provider
                )
            };
            if model.is_empty() {
                return Err(named("the model id cannot be empty".to_string()));
            }
            over.validate().map_err(named)?;
            let merged = over.apply_to(self.base_for(model).0);
            validate_profile(&merged).map_err(named)?;
        }
        self.overrides = overrides;
        Ok(self)
    }

    /// The seeded entry (or the fallback) an override for `model` would be laid over, and
    /// whether a seed was found.
    fn base_for(&self, model: &str) -> (ModelProfile, Option<&'static SeededProfile>) {
        match self.seeded.iter().find(|entry| entry.model == model) {
            Some(entry) => (
                self.window_applied((entry.build)().with_verified_on(entry.verified_on)),
                Some(entry),
            ),
            None => (self.window_applied(self.fallback.clone()), None),
        }
    }

    /// Resolve `model` to its profile and record how: override over seed over fallback, exact
    /// match only. A seeded entry's `verified_on` is set on its profile.
    pub(crate) fn resolve(&self, model: &str) -> (ModelProfile, ProfileOrigin) {
        let (base, seed) = self.base_for(model);
        match (self.overrides.get(model), seed) {
            (Some(over), Some(entry)) => (
                over.apply_to(base),
                ProfileOrigin::SeededWithOverride {
                    verified_on: entry.verified_on,
                },
            ),
            (Some(over), None) => (over.apply_to(base), ProfileOrigin::Override),
            (None, Some(entry)) => (
                base,
                ProfileOrigin::Seeded {
                    verified_on: entry.verified_on,
                },
            ),
            (None, None) => (base, ProfileOrigin::AdapterDefault),
        }
    }

    /// The profile `model` resolves to.
    pub(crate) fn profile(&self, model: &str) -> ModelProfile {
        self.resolve(model).0
    }

    /// Whether `model` runs on the adapter-wide fallback (D-07). Logs one
    /// `warn!(target: "paladin::registry")` per instance per model the first time it does,
    /// naming provider and model and saying the adapter-wide values apply; returns `true` on
    /// every call for such a model so the caller can stamp
    /// [`UNPROFILED_MODEL_METADATA_KEY`] each time.
    pub(crate) fn note_unprofiled(&self, model: &str) -> bool {
        if !matches!(self.resolve(model).1, ProfileOrigin::AdapterDefault) {
            return false;
        }
        let mut warned = self
            .warned
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if warned.insert(model.to_string()) {
            log::warn!(
                target: "paladin::registry",
                "provider '{}' has no profile for model '{model}': its adapter-wide values apply. \
                 Add a models.{model} entry to declare what this model accepts; each response is \
                 marked with '{}'",
                self.provider,
                UNPROFILED_MODEL_METADATA_KEY,
            );
        }
        true
    }
}

/// Derive an adapter-wide fallback profile from what a compat preset sends today.
///
/// - `temperature`: `Range(temperature_range)` when the preset's request path carries it, else
///   `NotSent`. A preset that declares no range falls under ADR-0004's default `(0.0, 1.0)`.
/// - `top_p`: `Range(0.0, 1.0)` when carried, else `NotSent`.
/// - `frequency_penalty` / `presence_penalty`: `Range(-2.0, 2.0)` (the OpenAI chat-completions
///   contract) when carried, else `NotSent`.
/// - `stop_sequences`: the caller's `stop` (compat presets differ: Grok does not send it).
/// - `context_window`: the preset's advertised maximum.
/// - `json_object` is true and `json_schema` false: the engine downgrades a schema to an object.
///
/// Nothing here is a per-model fact; it is the preset's own declaration restated in profile form
/// so one admission rule serves the build-time check and the wire filter.
#[cfg(any(
    feature = "kimi",
    feature = "qwen",
    feature = "grok",
    feature = "ollama",
    feature = "openai-compatible"
))]
pub(crate) fn fallback_from_compat(
    caps: &paladin_ports::output::llm_port::ProviderCapabilities,
    params: &crate::compat::CompatRequestParameters,
    stop: StopSupport,
) -> ModelProfile {
    let (lo, hi) = caps.temperature_range.unwrap_or((0.0, 1.0));
    let support = |carried: bool, range: ParamSupport| {
        if carried {
            range
        } else {
            ParamSupport::NotSent
        }
    };

    let mut profile = ModelProfile::new()
        .with_temperature(support(params.temperature, ParamSupport::Range(lo, hi)))
        .with_top_p(support(params.top_p, ParamSupport::Range(0.0, 1.0)))
        .with_frequency_penalty(support(
            params.frequency_penalty,
            ParamSupport::Range(-2.0, 2.0),
        ))
        .with_presence_penalty(support(
            params.presence_penalty,
            ParamSupport::Range(-2.0, 2.0),
        ))
        .with_stop_sequences(stop)
        .with_json_object(true)
        .with_json_schema(false);
    if let Some(window) = caps.max_context_tokens {
        profile = profile.with_context_window(window);
    }
    profile
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile_a() -> ModelProfile {
        ModelProfile::new()
            .with_temperature(ParamSupport::Fixed(1.0))
            .with_context_window(1000)
            .with_max_output_tokens(500)
    }

    static SEEDS: [SeededProfile; 1] = [SeededProfile {
        model: "kimi-k3",
        verified_on: "2026-10-01",
        build: profile_a,
    }];

    fn fallback() -> ModelProfile {
        ModelProfile::new()
            .with_temperature(ParamSupport::Range(0.0, 1.0))
            .with_context_window(131_072)
    }

    fn registry() -> ModelRegistry {
        ModelRegistry::new("kimi", &SEEDS, fallback())
    }

    fn overrides(model: &str, over: ModelOverride) -> BTreeMap<String, ModelOverride> {
        BTreeMap::from([(model.to_string(), over)])
    }

    #[test]
    fn resolve_seeded_entry_wins_over_the_fallback() {
        let (profile, origin) = registry().resolve("kimi-k3");
        assert_eq!(profile.temperature, ParamSupport::Fixed(1.0));
        assert_eq!(profile.verified_on.as_deref(), Some("2026-10-01"));
        assert_eq!(
            origin,
            ProfileOrigin::Seeded {
                verified_on: "2026-10-01"
            }
        );
    }

    #[test]
    fn resolve_is_exact_match_only() {
        let (profile, origin) = registry().resolve("kimi-k3-preview");
        assert_eq!(origin, ProfileOrigin::AdapterDefault);
        assert_eq!(profile.temperature, ParamSupport::Range(0.0, 1.0));
    }

    #[test]
    fn resolve_unknown_and_empty_models_use_the_adapter_fallback() {
        for model in ["no-such-model", ""] {
            let (profile, origin) = registry().resolve(model);
            assert_eq!(origin, ProfileOrigin::AdapterDefault, "model {model:?}");
            assert_eq!(profile, fallback(), "model {model:?}");
        }
    }

    #[test]
    fn override_wins_field_by_field() {
        let over = ModelOverride {
            max_output_tokens: Some(200),
            ..ModelOverride::default()
        };
        let registry = registry()
            .with_overrides(overrides("kimi-k3", over))
            .expect("a valid override");

        let (profile, origin) = registry.resolve("kimi-k3");
        assert_eq!(profile.max_output_tokens, Some(200), "the override wins");
        assert_eq!(
            profile.temperature,
            ParamSupport::Fixed(1.0),
            "a field the override leaves None keeps the seeded value"
        );
        assert_eq!(profile.context_window, Some(1000));
        assert_eq!(
            origin,
            ProfileOrigin::SeededWithOverride {
                verified_on: "2026-10-01"
            }
        );
    }

    #[test]
    fn resolve_override_only_model_is_laid_over_the_fallback_and_is_not_unprofiled() {
        let over = ModelOverride {
            top_p: Some(ParamSupport::Fixed(0.9)),
            ..ModelOverride::default()
        };
        let registry = registry()
            .with_overrides(overrides("brand-new", over))
            .expect("a valid override");

        let (profile, origin) = registry.resolve("brand-new");
        assert_eq!(origin, ProfileOrigin::Override);
        assert_eq!(profile.top_p, ParamSupport::Fixed(0.9));
        assert_eq!(profile.temperature, ParamSupport::Range(0.0, 1.0));
        assert!(
            !registry.note_unprofiled("brand-new"),
            "an overridden model is profiled by the operator"
        );
    }

    #[test]
    fn note_unprofiled_is_true_for_the_fallback_every_time_and_false_for_a_seed() {
        let registry = registry();
        assert!(registry.note_unprofiled("mystery"));
        assert!(registry.note_unprofiled("mystery"), "true on every call");
        assert_eq!(
            registry.warned.lock().unwrap().len(),
            1,
            "the warning is deduplicated per model"
        );
        assert!(!registry.note_unprofiled("kimi-k3"));
    }

    #[test]
    fn with_overrides_refuses_a_merge_that_contradicts_the_seed() {
        // Alone this override is fine; over the seed's 1000-token window it is not.
        let over = ModelOverride {
            max_output_tokens: Some(5000),
            ..ModelOverride::default()
        };
        let error = registry()
            .with_overrides(overrides("kimi-k3", over))
            .expect_err("a maximum output above the seeded window");
        assert!(
            error.contains("kimi-k3") && error.contains("kimi"),
            "{error}"
        );
        assert!(error.contains("max_output_tokens"), "{error}");
    }

    #[test]
    fn with_overrides_refuses_an_empty_model_id() {
        let error = registry()
            .with_overrides(overrides("", ModelOverride::default()))
            .expect_err("an empty model id");
        assert!(error.contains("empty"), "{error}");
    }

    /// Parse through the `config` crate -- the loader application settings go through.
    fn parse(yaml: &str) -> Result<ModelOverride, config::ConfigError> {
        config::Config::builder()
            .add_source(config::File::from_str(yaml, config::FileFormat::Yaml))
            .build()?
            .try_deserialize()
    }

    #[test]
    fn model_override_rejects_an_unknown_key_naming_it() {
        let error = parse("context_windw: 100\n").expect_err("unknown key");
        assert!(error.to_string().contains("context_windw"), "{error}");
    }

    #[test]
    fn model_override_rejects_a_window_past_u32_naming_the_field() {
        let error = parse("context_window: 4294967296\n").expect_err("past u32::MAX");
        assert!(error.to_string().contains("context_window"), "{error}");
    }

    #[test]
    fn model_override_fixed_value_survives_the_yaml_f64_to_f32_conversion() {
        let over = parse("temperature:\n  fixed: 0.95\n").expect("a fixed temperature");
        let Some(support) = over.temperature else {
            panic!("temperature was set");
        };
        assert!(support.admits(0.95_f32), "exact f32 equality: {support:?}");
    }

    #[test]
    fn model_override_parses_every_shape() {
        let over = parse(
            "context_window: 262144\n\
             max_output_tokens: 8192\n\
             temperature: {range: [0.0, 1.0]}\n\
             top_p: not_sent\n\
             stop_sequences: {supported: {max: 4}}\n\
             reasoning: true\n\
             json_object: true\n\
             json_schema: false\n",
        )
        .expect("a full override");
        assert_eq!(over.context_window, Some(262_144));
        assert_eq!(over.temperature, Some(ParamSupport::Range(0.0, 1.0)));
        assert_eq!(over.top_p, Some(ParamSupport::NotSent));
        assert_eq!(
            over.stop_sequences,
            Some(StopSupport::Supported { max: Some(4) })
        );
        assert_eq!(over.reasoning, Some(true));
        assert_eq!(over.json_schema, Some(false));
        assert!(over.validate().is_ok());
    }

    fn rejected(over: ModelOverride, field: &str) {
        let error = over.validate().expect_err("must be rejected");
        assert!(error.contains(field), "'{field}' missing from: {error}");
    }

    #[test]
    fn model_override_validate_rejects_zero_limits_and_a_max_above_the_window() {
        let over = ModelOverride {
            context_window: Some(0),
            ..ModelOverride::default()
        };
        rejected(over, "context_window");

        let over = ModelOverride {
            max_output_tokens: Some(0),
            ..ModelOverride::default()
        };
        rejected(over, "max_output_tokens");

        let over = ModelOverride {
            context_window: Some(100),
            max_output_tokens: Some(101),
            ..ModelOverride::default()
        };
        rejected(over, "max_output_tokens");
    }

    #[test]
    fn model_override_validate_rejects_inverted_and_non_finite_ranges() {
        let over = ModelOverride {
            temperature: Some(ParamSupport::Range(1.0, 0.0)),
            ..ModelOverride::default()
        };
        rejected(over, "temperature");

        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let over = ModelOverride {
                top_p: Some(ParamSupport::Range(0.0, bad)),
                ..ModelOverride::default()
            };
            rejected(over, "top_p");

            let over = ModelOverride {
                presence_penalty: Some(ParamSupport::Fixed(bad)),
                ..ModelOverride::default()
            };
            rejected(over, "presence_penalty");
        }
    }

    #[test]
    fn model_override_validate_rejects_a_zero_stop_maximum() {
        let over = ModelOverride {
            stop_sequences: Some(StopSupport::Supported { max: Some(0) }),
            ..ModelOverride::default()
        };
        rejected(over, "stop_sequences");
    }

    #[test]
    fn model_override_validate_accepts_boundary_values() {
        let over = ModelOverride {
            context_window: Some(100),
            max_output_tokens: Some(100),
            temperature: Some(ParamSupport::Range(0.5, 0.5)),
            stop_sequences: Some(StopSupport::Supported { max: Some(1) }),
            ..ModelOverride::default()
        };
        assert!(over.validate().is_ok());
    }

    #[cfg(any(
        feature = "kimi",
        feature = "qwen",
        feature = "grok",
        feature = "ollama",
        feature = "openai-compatible"
    ))]
    #[test]
    fn fallback_from_compat_restates_the_declaration_as_a_profile() {
        use crate::compat::CompatRequestParameters;
        use paladin_ports::output::llm_port::ProviderCapabilities;

        let caps = ProviderCapabilities {
            max_context_tokens: Some(4096),
            temperature_range: Some((0.0, 1.99)),
            ..ProviderCapabilities::default()
        };
        let mut params = CompatRequestParameters::all();
        params.presence_penalty = false;

        let profile = fallback_from_compat(&caps, &params, StopSupport::Supported { max: None });
        assert_eq!(profile.temperature, ParamSupport::Range(0.0, 1.99));
        assert_eq!(profile.top_p, ParamSupport::Range(0.0, 1.0));
        assert_eq!(profile.frequency_penalty, ParamSupport::Range(-2.0, 2.0));
        assert_eq!(profile.presence_penalty, ParamSupport::NotSent);
        assert_eq!(profile.context_window, Some(4096));
        assert!(profile.json_object && !profile.json_schema);
    }
}

/// Tests of the dated per-vendor tables in [`seeded`]: structural invariants over every table, and
/// one test per PROBED vendor that pins an entry to the transcript row it cites
/// (`50-SEED-PROBE.md`, run on 2026-10-15).
#[cfg(test)]
mod seeded_tests {
    use super::*;

    /// Every compiled vendor table, by provider name.
    #[allow(unused_mut, clippy::vec_init_then_push)]
    fn tables() -> Vec<(&'static str, &'static [SeededProfile])> {
        let mut tables: Vec<(&'static str, &'static [SeededProfile])> = Vec::new();
        #[cfg(feature = "anthropic")]
        tables.push(("anthropic", seeded::ANTHROPIC));
        #[cfg(feature = "openai")]
        tables.push(("openai", seeded::OPENAI));
        #[cfg(feature = "deepseek")]
        tables.push(("deepseek", seeded::DEEPSEEK));
        #[cfg(feature = "kimi")]
        tables.push(("kimi", seeded::KIMI));
        #[cfg(feature = "qwen")]
        tables.push(("qwen", seeded::QWEN));
        #[cfg(feature = "grok")]
        tables.push(("grok", seeded::GROK));
        #[cfg(feature = "gemini")]
        tables.push(("gemini", seeded::GEMINI));
        #[cfg(feature = "ollama")]
        tables.push(("ollama", seeded::OLLAMA));
        tables
    }

    /// `YYYY-MM-DD` with a plausible month and day.
    fn is_iso_date(text: &str) -> bool {
        let bytes = text.as_bytes();
        if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
            return false;
        }
        let digits = |range: std::ops::Range<usize>| {
            text.get(range)
                .filter(|part| part.bytes().all(|b| b.is_ascii_digit()))
                .and_then(|part| part.parse::<u32>().ok())
        };
        match (digits(0..4), digits(5..7), digits(8..10)) {
            (Some(year), Some(month), Some(day)) => {
                year >= 2000 && (1..=12).contains(&month) && (1..=31).contains(&day)
            }
            _ => false,
        }
    }

    /// Build one seeded entry the way the registry does, panicking when it is absent.
    #[allow(dead_code)]
    fn entry(table: &'static [SeededProfile], model: &str) -> ModelProfile {
        let seeded = table
            .iter()
            .find(|entry| entry.model == model)
            .unwrap_or_else(|| panic!("no seeded entry for {model}"));
        (seeded.build)().with_verified_on(seeded.verified_on)
    }

    #[test]
    fn seeded_tables_are_dated_unique_and_consistent() {
        for (provider, table) in tables() {
            let mut seen = HashSet::new();
            for entry in table {
                assert!(
                    seen.insert(entry.model),
                    "{provider}: duplicate model id {}",
                    entry.model
                );
                assert!(
                    !entry.model.is_empty(),
                    "{provider}: an entry has an empty model id"
                );
                assert!(
                    is_iso_date(entry.verified_on),
                    "{provider}/{}: verified_on {:?} is not YYYY-MM-DD",
                    entry.model,
                    entry.verified_on
                );
                let profile = (entry.build)();
                assert_ne!(
                    profile.context_window,
                    Some(0),
                    "{provider}/{}: zero context window",
                    entry.model
                );
                assert_ne!(
                    profile.max_output_tokens,
                    Some(0),
                    "{provider}/{}: zero max output",
                    entry.model
                );
                if let (Some(window), Some(max_output)) =
                    (profile.context_window, profile.max_output_tokens)
                {
                    assert!(
                        max_output <= window,
                        "{provider}/{}: max output {max_output} exceeds window {window}",
                        entry.model
                    );
                }
                validate_profile(&profile)
                    .unwrap_or_else(|problem| panic!("{provider}/{}: {problem}", entry.model));
            }
        }
    }

    #[test]
    fn the_date_check_accepts_iso_dates_and_rejects_the_rest() {
        assert!(is_iso_date("2026-10-15"));
        for bad in [
            "",
            "2026-1-15",
            "2026/10/15",
            "20261015",
            "2026-13-01",
            "2026-10-00",
        ] {
            assert!(!is_iso_date(bad), "{bad:?} must not pass");
        }
    }

    #[test]
    fn every_entry_carries_the_probe_date() {
        // One probe run seeds every table; a different date here means a value came from a
        // source other than 50-SEED-PROBE.md.
        for (provider, table) in tables() {
            for entry in table {
                assert_eq!(
                    entry.verified_on, "2026-10-15",
                    "{provider}/{}: not from the 2026-10-15 probe",
                    entry.model
                );
            }
        }
    }

    #[test]
    fn unmeasured_and_retired_ids_are_not_seeded() {
        let unseeded = [
            // Anthropic: the eight curated legacy ids answered 404 (retired).
            "claude-3-5-sonnet-20241022",
            "claude-3-opus-20240229",
            "claude-2.1",
            "claude-instant-1.2",
            // DeepSeek: undocumented aliases, and listed-but-unprobed `deepseek-v4-pro`.
            "deepseek-chat",
            "deepseek-coder",
            "deepseek-v4-pro",
            // Kimi: every probe was rejected for the thinking mode.
            "kimi-k2.7-code",
            "kimi-k2.7-code-highspeed",
            // Qwen: rate-limited on every probe / 404 on the International endpoint.
            "qwen-turbo",
            "qwen3-max",
        ];
        for (provider, table) in tables() {
            for entry in table {
                assert!(
                    !unseeded.contains(&entry.model),
                    "{provider}/{}: has no usable measurement and must not be seeded",
                    entry.model
                );
            }
        }
    }

    #[cfg(feature = "ollama")]
    #[test]
    fn the_ollama_table_is_empty_because_the_probe_skipped_it() {
        assert!(seeded::OLLAMA.is_empty());
    }

    #[cfg(feature = "kimi")]
    #[test]
    fn seeded_lookup_is_exact_match_only() {
        let registry = ModelRegistry::new("kimi", seeded::KIMI, ModelProfile::new());
        assert!(matches!(
            registry.resolve("kimi-k3").1,
            ProfileOrigin::Seeded {
                verified_on: "2026-10-15"
            }
        ));
        for near_miss in ["kimi-k3-preview", "kimi", "KIMI-K3", "kimi-k3 "] {
            assert_eq!(
                registry.resolve(near_miss).1,
                ProfileOrigin::AdapterDefault,
                "{near_miss:?} must not prefix- or family-match"
            );
        }
    }

    #[cfg(feature = "openai")]
    #[test]
    fn openai_gpt_5_4_nano_seed_matches_the_transcript() {
        let p = entry(seeded::OPENAI, "gpt-5.4-nano");
        // Probes 2-5: 0.00, 1.00, 0.50 and 1.01 all ACCEPTED, so no upper bound was located.
        assert_eq!(p.temperature, ParamSupport::Range(0.0, 1.0));
        assert_eq!(p.top_p, ParamSupport::Range(0.0, 1.0));
        // Probes 9 and 10: `stop` is not supported with this model, for one and for five.
        assert_eq!(p.stop_sequences, StopSupport::NotSent);
        // Operator evidence (b), docs accessed 2026-10-15.
        assert_eq!(p.context_window, Some(400_000));
        assert_eq!(p.max_output_tokens, Some(128_000));
        // Probes 13 and 14: json_object and json_schema ACCEPTED.
        assert!(p.json_object && p.json_schema);
        assert_eq!(p.verified_on.as_deref(), Some("2026-10-15"));
    }

    #[cfg(feature = "openai")]
    #[test]
    fn openai_gpt_4o_mini_seed_matches_the_transcript() {
        let p = entry(seeded::OPENAI, "gpt-4o-mini");
        // Probe 10: five sequences rejected, "maximum length 4" (operator evidence (c)).
        assert_eq!(p.stop_sequences, StopSupport::Supported { max: Some(4) });
        assert_eq!(p.context_window, Some(128_000));
        assert_eq!(p.max_output_tokens, Some(16_384));
        assert!(p.json_object && p.json_schema);
        // The adapter's request type carries no penalty field, so none is admitted.
        assert_eq!(p.frequency_penalty, ParamSupport::NotSent);
        assert_eq!(p.presence_penalty, ParamSupport::NotSent);
    }

    #[cfg(feature = "anthropic")]
    #[test]
    fn anthropic_haiku_5_5_seed_matches_the_transcript() {
        let p = entry(seeded::ANTHROPIC, "claude-haiku-5-5");
        // Probes 2-4: only temperature 1.00 ACCEPTED; 0.00 and 0.50 "deprecated for this model".
        assert_eq!(p.temperature, ParamSupport::Fixed(1.0));
        // Probe 6: top_p "is deprecated for this model".
        assert_eq!(p.top_p, ParamSupport::NotSent);
        // Probes 9 and 10: one and five stop sequences ACCEPTED.
        assert_eq!(p.stop_sequences, StopSupport::Supported { max: None });
        // Listing: max_input_tokens=1000000, max_tokens=128000.
        assert_eq!(p.context_window, Some(1_000_000));
        assert_eq!(p.max_output_tokens, Some(128_000));
        // Probes 12 and 13: Anthropic has no JSON mode.
        assert!(!p.json_object && !p.json_schema);
    }

    #[cfg(feature = "deepseek")]
    #[test]
    fn deepseek_flash_seed_matches_the_transcript() {
        let p = entry(seeded::DEEPSEEK, "deepseek-flash");
        // Probes 2, 3 and 5: 0.00 and 2.00 ACCEPTED, 2.01 rejected "valid range is [0, 2]".
        assert_eq!(p.temperature, ParamSupport::Range(0.0, 2.0));
        assert_eq!(p.stop_sequences, StopSupport::Supported { max: None });
        // Listing: context_window=1048576, max_output_tokens=393216.
        assert_eq!(p.context_window, Some(1_048_576));
        assert_eq!(p.max_output_tokens, Some(393_216));
        // Probe 1 returned reasoning text.
        assert!(p.reasoning);
        // Probe 12 ACCEPTED, probe 13 rejected "response_format type is unavailable now".
        assert!(p.json_object);
        assert!(!p.json_schema);
    }

    #[cfg(feature = "kimi")]
    #[test]
    fn kimi_k3_seed_matches_the_transcript() {
        let p = entry(seeded::KIMI, "kimi-k3");
        // Probes 3-5: 1.00, 0.50 and 1.01 all rejected, "only 0.6 is allowed".
        assert_eq!(p.temperature, ParamSupport::NotSent);
        // Probes 6-8: top_p and both penalties rejected.
        assert_eq!(p.top_p, ParamSupport::NotSent);
        assert_eq!(p.frequency_penalty, ParamSupport::NotSent);
        assert_eq!(p.presence_penalty, ParamSupport::NotSent);
        assert_eq!(p.stop_sequences, StopSupport::Supported { max: None });
        // Listing context_length=1048576; no maximum output is documented.
        assert_eq!(p.context_window, Some(1_048_576));
        assert_eq!(p.max_output_tokens, None);
        assert!(p.reasoning && p.json_object && p.json_schema);
    }

    #[cfg(feature = "kimi")]
    #[test]
    fn kimi_k2_6_seed_matches_the_transcript() {
        let p = entry(seeded::KIMI, "kimi-k2.6");
        assert_eq!(p.temperature, ParamSupport::NotSent);
        assert_eq!(p.top_p, ParamSupport::NotSent);
        // Listing context_length=262144; no maximum output is documented.
        assert_eq!(p.context_window, Some(262_144));
        assert_eq!(p.max_output_tokens, None);
        assert_eq!(p.stop_sequences, StopSupport::Supported { max: None });
        assert!(p.reasoning && p.json_object && p.json_schema);
    }

    #[cfg(feature = "qwen")]
    #[test]
    fn qwen_3_8_flash_seed_matches_the_transcript() {
        let p = entry(seeded::QWEN, "qwen3.8-flash");
        // Probes 2, 3 and 5: 0.00 and 1.99 ACCEPTED, 2.00 rejected "[0.0, 2.0)".
        assert_eq!(p.temperature, ParamSupport::Range(0.0, 1.99));
        assert_eq!(p.frequency_penalty, ParamSupport::Range(-2.0, 2.0));
        assert_eq!(p.stop_sequences, StopSupport::Supported { max: None });
        // Operator evidence (b), docs accessed 2026-10-15.
        assert_eq!(p.context_window, Some(1_000_000));
        assert_eq!(p.max_output_tokens, Some(131_072));
        assert!(p.reasoning && p.json_object && p.json_schema);
    }

    #[cfg(feature = "qwen")]
    #[test]
    fn qwen_plus_seed_matches_the_transcript() {
        let p = entry(seeded::QWEN, "qwen-plus");
        assert_eq!(p.temperature, ParamSupport::Range(0.0, 1.99));
        // The listing exposes no limit and the vendor page states none: none is seeded.
        assert_eq!(p.context_window, None);
        assert_eq!(p.max_output_tokens, None);
        // Probe 1 carried no reasoning text.
        assert!(!p.reasoning);
    }

    #[cfg(feature = "grok")]
    #[test]
    fn grok_seeds_match_the_transcript() {
        for (model, window) in [
            ("grok-4.3", 1_000_000),
            ("grok-4.5", 500_000),
            ("grok-4.6", 500_000),
        ] {
            let p = entry(seeded::GROK, model);
            // Probes 9 and 10: `stop` rejected for one and for five.
            assert_eq!(p.stop_sequences, StopSupport::NotSent, "{model}");
            // Probes 7 and 8: both penalties rejected.
            assert_eq!(p.frequency_penalty, ParamSupport::NotSent, "{model}");
            assert_eq!(p.presence_penalty, ParamSupport::NotSent, "{model}");
            // Probes 2 and 4: 0.00 and 0.50 ACCEPTED. Probe 3 (2.00) was UNREACHABLE; probe 5
            // (2.01) was rejected "must be less than 2", so the bound is seeded at 1.99.
            assert_eq!(p.temperature, ParamSupport::Range(0.0, 1.99), "{model}");
            assert_eq!(p.top_p, ParamSupport::Range(0.0, 1.0), "{model}");
            // Listing context_length.
            assert_eq!(p.context_window, Some(window), "{model}");
            assert_eq!(p.max_output_tokens, None, "{model}");
            assert!(p.reasoning && p.json_object && p.json_schema, "{model}");
        }
    }

    #[cfg(feature = "gemini")]
    #[test]
    fn gemini_seeds_match_the_transcript() {
        for model in [
            "gemini-3.5-flash-lite",
            "gemini-3.5-flash",
            "gemini-3.6-flash",
        ] {
            let p = entry(seeded::GEMINI, model);
            // Probes 2, 3 and 5: 0.00 and 2.00 ACCEPTED, 2.01 rejected "[0.0, 2.0]".
            assert_eq!(p.temperature, ParamSupport::Range(0.0, 2.0), "{model}");
            // Probes 7 and 8: the wire has no penalty field.
            assert_eq!(p.frequency_penalty, ParamSupport::NotSent, "{model}");
            assert_eq!(p.presence_penalty, ParamSupport::NotSent, "{model}");
            assert_eq!(
                p.stop_sequences,
                StopSupport::Supported { max: None },
                "{model}"
            );
            // Listing: inputTokenLimit=1048576, outputTokenLimit=65536, thinking=true.
            assert_eq!(p.context_window, Some(1_048_576), "{model}");
            assert_eq!(p.max_output_tokens, Some(65_536), "{model}");
            assert!(p.reasoning && p.json_object && p.json_schema, "{model}");
        }
    }
}

/// The adapter-wide window knob and the reasoning-model lookup (plan 50-08, D-12/D-13).
#[cfg(test)]
mod knob_tests {
    use super::*;

    fn seeded_k3() -> ModelProfile {
        ModelProfile::new()
            .with_temperature(ParamSupport::Fixed(1.0))
            .with_context_window(1000)
            .with_max_output_tokens(500)
            .with_reasoning(true)
    }

    static SEEDS: [SeededProfile; 1] = [SeededProfile {
        model: "kimi-k3",
        verified_on: "2026-10-01",
        build: seeded_k3,
    }];

    fn fallback() -> ModelProfile {
        ModelProfile::new().with_context_window(131_072)
    }

    fn registry() -> ModelRegistry {
        ModelRegistry::new("kimi", &SEEDS, fallback())
    }

    #[test]
    fn with_context_window_replaces_the_fallback_and_every_seeded_window() {
        let registry = registry().with_context_window(Some(800)).unwrap();
        assert_eq!(registry.profile("kimi-k3").context_window, Some(800));
        assert_eq!(registry.profile("unseeded").context_window, Some(800));
        // Everything else about the seeded entry is untouched.
        assert_eq!(registry.profile("kimi-k3").max_output_tokens, Some(500));
    }

    #[test]
    fn with_context_window_none_changes_nothing() {
        let registry = registry().with_context_window(None).unwrap();
        assert_eq!(registry.profile("kimi-k3").context_window, Some(1000));
        assert_eq!(registry.profile("unseeded").context_window, Some(131_072));
    }

    #[test]
    fn a_per_model_override_window_still_beats_the_adapter_wide_window() {
        let over = ModelOverride {
            context_window: Some(900),
            ..ModelOverride::default()
        };
        let registry = registry()
            .with_context_window(Some(800))
            .unwrap()
            .with_overrides(BTreeMap::from([("kimi-k3".to_string(), over)]))
            .unwrap();
        assert_eq!(registry.profile("kimi-k3").context_window, Some(900));
        assert_eq!(registry.profile("unseeded").context_window, Some(800));
    }

    #[test]
    fn with_context_window_refuses_a_window_below_a_seeded_max_output() {
        let error = registry()
            .with_context_window(Some(400))
            .expect_err("400 is below the seeded maximum output of 500");
        for needle in ["kimi", "kimi-k3", "400", "500"] {
            assert!(error.contains(needle), "'{needle}' missing from: {error}");
        }
    }

    #[test]
    fn with_context_window_refuses_zero() {
        let error = registry()
            .with_context_window(Some(0))
            .expect_err("a zero window");
        assert!(error.contains("kimi") && error.contains("zero"), "{error}");
    }

    #[test]
    fn with_context_window_accepts_a_window_equal_to_a_seeded_max_output() {
        let registry = registry().with_context_window(Some(500)).unwrap();
        assert_eq!(registry.profile("kimi-k3").context_window, Some(500));
    }

    #[test]
    fn the_adapter_wide_window_is_validated_against_overrides_laid_over_it() {
        // 600 is a window the seed (maximum output 500) accepts; an override whose maximum
        // output is above it is then refused, naming the model.
        let over = ModelOverride {
            max_output_tokens: Some(700),
            ..ModelOverride::default()
        };
        let error = registry()
            .with_context_window(Some(600))
            .unwrap()
            .with_overrides(BTreeMap::from([("kimi-k3".to_string(), over)]))
            .expect_err("a maximum output above the adapter-wide window");
        assert!(error.contains("kimi-k3"), "{error}");
    }

    #[cfg(feature = "kimi")]
    #[test]
    fn is_reasoning_model_prefers_the_override() {
        let none = BTreeMap::new();
        // Seeded: kimi-k3 and kimi-k2.6 are reasoning models.
        assert!(is_reasoning_model("kimi", "kimi-k3", &none));
        // Unknown model, unknown provider, no override: not a reasoning model.
        assert!(!is_reasoning_model("kimi", "kimi-unknown", &none));
        assert!(!is_reasoning_model("no-such-provider", "kimi-k3", &none));
        // Exact match only.
        assert!(!is_reasoning_model("kimi", "kimi-k3-preview", &none));

        let say = |model: &str, reasoning: bool| {
            BTreeMap::from([(
                model.to_string(),
                ModelOverride {
                    reasoning: Some(reasoning),
                    ..ModelOverride::default()
                },
            )])
        };
        // The override beats the seed in both directions ...
        assert!(!is_reasoning_model(
            "kimi",
            "kimi-k3",
            &say("kimi-k3", false)
        ));
        assert!(is_reasoning_model(
            "kimi",
            "kimi-unknown",
            &say("kimi-unknown", true)
        ));
        // ... an override that leaves `reasoning` unset keeps the seed ...
        let unset = BTreeMap::from([("kimi-k3".to_string(), ModelOverride::default())]);
        assert!(is_reasoning_model("kimi", "kimi-k3", &unset));
        // ... and an override for another model is irrelevant.
        assert!(is_reasoning_model(
            "kimi",
            "kimi-k3",
            &say("kimi-k2.6", false)
        ));
    }
}

/// The Switching providers page against the registry (Phase 50 plan 50-16, PARITY-07, RESEARCH
/// Open Question 10): the mdBook page `docs/src/user-guides/switching-providers.md` carries one
/// table row per seeded model and one adapter-wide fallback row per provider, and this module
/// fails when the page and `seeded.rs` disagree, so a row cannot be edited on one side only.
///
/// Compiled only when all nine adapters are, because the page lists all nine providers and a
/// partial build has no table for the others to compare against. The workspace phase gate
/// (`--features llm-all`) and `cargo test -p paladin-llm --all-features` both compile it.
///
/// Skip rule (the Phase 48 precedent, `tests/entity_rename_guard.rs`): the check is skipped only
/// when the tree has no `docs/` directory at all, which is the Docker test image's build context
/// (`.dockerignore` excludes `docs/`). A tree that has `docs/` but not the page fails.
#[cfg(all(
    test,
    feature = "openai",
    feature = "anthropic",
    feature = "deepseek",
    feature = "kimi",
    feature = "qwen",
    feature = "grok",
    feature = "gemini",
    feature = "ollama",
    feature = "openai-compatible"
))]
mod switching_page_tests {
    use super::*;
    use std::path::{Path, PathBuf};

    /// The page, relative to the workspace root.
    const PAGE: &str = "docs/src/user-guides/switching-providers.md";
    /// The nine providers in the fixed order every table on the page uses.
    const PROVIDERS: [&str; 9] = [
        "openai",
        "anthropic",
        "deepseek",
        "kimi",
        "qwen",
        "grok",
        "gemini",
        "ollama",
        "openai-compatible",
    ];
    /// The model cell of an adapter-wide fallback row.
    const FALLBACK_LABEL: &str = "(adapter-wide fallback)";
    /// The `verified on` cell of an adapter-wide fallback row.
    const UNVERIFIED: &str = "unverified";
    const BEHAVIOUR_HEADING: &str = "## Per-provider and per-model behaviour";
    const RESPONSES_HEADING: &str = "## Responses";
    /// Columns of the behaviour table.
    const BEHAVIOUR_COLUMNS: usize = 11;

    /// What a check of the page concluded.
    #[derive(Debug, PartialEq, Eq)]
    enum Outcome {
        /// The tree has no `docs/` directory (the Docker test image): nothing to check.
        Skipped,
        /// The page was read and agrees with the registry.
        Checked,
    }

    fn workspace_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    /// The seeded table of `provider`; OpenAI-compatible has none by design.
    fn seeded_for(provider: &str) -> &'static [SeededProfile] {
        match provider {
            "openai" => seeded::OPENAI,
            "anthropic" => seeded::ANTHROPIC,
            "deepseek" => seeded::DEEPSEEK,
            "kimi" => seeded::KIMI,
            "qwen" => seeded::QWEN,
            "grok" => seeded::GROK,
            "gemini" => seeded::GEMINI,
            "ollama" => seeded::OLLAMA,
            _ => &[],
        }
    }

    fn with_thousands(value: u32) -> String {
        let digits = value.to_string();
        let mut out = String::new();
        for (index, digit) in digits.chars().enumerate() {
            if index > 0 && (digits.len() - index).is_multiple_of(3) {
                out.push(',');
            }
            out.push(digit);
        }
        out
    }

    fn render_limit(limit: Option<u32>) -> String {
        limit.map_or_else(|| "not set".to_string(), with_thousands)
    }

    fn render_param(support: ParamSupport) -> String {
        match support {
            ParamSupport::Range(lo, hi) => format!("{lo:?} to {hi:?}"),
            ParamSupport::Fixed(value) => format!("only {value:?}"),
            ParamSupport::NotSent => "not sent".to_string(),
        }
    }

    fn render_penalties(profile: &ModelProfile) -> String {
        if profile.frequency_penalty == profile.presence_penalty {
            render_param(profile.frequency_penalty)
        } else {
            format!(
                "frequency {}; presence {}",
                render_param(profile.frequency_penalty),
                render_param(profile.presence_penalty)
            )
        }
    }

    fn render_stop(stop: StopSupport) -> String {
        match stop {
            StopSupport::NotSent => "not sent".to_string(),
            StopSupport::Supported { max: None } => "yes".to_string(),
            StopSupport::Supported { max: Some(max) } => format!("up to {max}"),
        }
    }

    fn render_json(profile: &ModelProfile) -> String {
        match (profile.json_object, profile.json_schema) {
            (true, true) => "object, schema",
            (true, false) => "object",
            (false, true) => "schema",
            (false, false) => "none",
        }
        .to_string()
    }

    fn render_yes_no(value: bool) -> String {
        if value { "yes" } else { "no" }.to_string()
    }

    /// The cells one seeded entry must have on the page, footnote markers excluded.
    fn expected_cells(provider: &str, entry: &SeededProfile) -> Vec<String> {
        let profile = (entry.build)();
        vec![
            provider.to_string(),
            format!("`{}`", entry.model),
            entry.verified_on.to_string(),
            render_limit(profile.context_window),
            render_limit(profile.max_output_tokens),
            render_param(profile.temperature),
            render_param(profile.top_p),
            render_penalties(&profile),
            render_stop(profile.stop_sequences),
            render_yes_no(profile.reasoning),
            render_json(&profile),
        ]
    }

    /// Remove footnote markers (a space, `[`, one lowercase letter, `]`) from a cell.
    fn strip_markers(cell: &str) -> String {
        let chars: Vec<char> = cell.chars().collect();
        let mut out = String::new();
        let mut index = 0;
        while index < chars.len() {
            let is_marker = chars[index] == ' '
                && chars.get(index + 1) == Some(&'[')
                && chars.get(index + 2).is_some_and(char::is_ascii_lowercase)
                && chars.get(index + 3) == Some(&']');
            if is_marker {
                index += 4;
            } else {
                out.push(chars[index]);
                index += 1;
            }
        }
        out
    }

    /// The lines of the section that starts at `heading` and ends at the next `## ` heading.
    fn section<'a>(page: &'a str, heading: &str) -> Result<Vec<&'a str>, String> {
        let mut lines = page.lines();
        if !lines.by_ref().any(|line| line.trim_end() == heading) {
            return Err(format!("the page has no `{heading}` section"));
        }
        Ok(lines.take_while(|line| !line.starts_with("## ")).collect())
    }

    fn cells_of(line: &str) -> Vec<String> {
        line.trim()
            .trim_matches('|')
            .split('|')
            .map(|cell| cell.trim().to_string())
            .collect()
    }

    /// The data rows of the behaviour table, markers stripped.
    fn behaviour_rows(page: &str) -> Result<Vec<Vec<String>>, String> {
        let mut rows = Vec::new();
        for line in section(page, BEHAVIOUR_HEADING)? {
            if !line.trim_start().starts_with('|') {
                continue;
            }
            let cells = cells_of(line);
            let is_header = cells.first().is_some_and(|cell| cell == "Provider");
            let is_rule = cells
                .first()
                .is_some_and(|cell| !cell.is_empty() && cell.chars().all(|c| c == '-'));
            if is_header || is_rule {
                continue;
            }
            if cells.len() != BEHAVIOUR_COLUMNS {
                return Err(format!(
                    "behaviour table row has {} cells, expected {BEHAVIOUR_COLUMNS}: {line}",
                    cells.len()
                ));
            }
            rows.push(cells.iter().map(|cell| strip_markers(cell)).collect());
        }
        Ok(rows)
    }

    /// The providers named in the first cell of each response-mapping row, in page order.
    fn response_providers(page: &str) -> Result<Vec<String>, String> {
        Ok(section(page, RESPONSES_HEADING)?
            .into_iter()
            .filter(|line| line.trim_start().starts_with('|'))
            .filter_map(|line| cells_of(line).into_iter().next())
            .filter(|cell| PROVIDERS.contains(&cell.as_str()))
            .collect())
    }

    /// Every `(provider, model cell)` the page must list, in the order it must list them: per
    /// provider, the seeded models by id ascending, then the adapter-wide fallback row.
    fn expected_order() -> Vec<(String, String)> {
        let mut order = Vec::new();
        for provider in PROVIDERS {
            let mut models: Vec<&str> = seeded_for(provider).iter().map(|e| e.model).collect();
            models.sort_unstable();
            for model in models {
                order.push((provider.to_string(), format!("`{model}`")));
            }
            order.push((provider.to_string(), FALLBACK_LABEL.to_string()));
        }
        order
    }

    /// Compare the page text with the registry; the error names the first disagreement.
    fn check_page_text(page: &str) -> Result<(), String> {
        let rows = behaviour_rows(page)?;
        let actual: Vec<(String, String)> = rows
            .iter()
            .map(|cells| (cells[0].clone(), cells[1].clone()))
            .collect();
        let expected = expected_order();

        if let Some((provider, model)) = expected.iter().find(|row| !actual.contains(row)) {
            return Err(format!(
                "the page has no behaviour row for provider '{provider}' model {model}"
            ));
        }
        if let Some((provider, model)) = actual.iter().find(|row| !expected.contains(row)) {
            return Err(format!(
                "the page has a behaviour row for provider '{provider}' model {model} that the \
                 registry does not seed"
            ));
        }
        if actual != expected {
            return Err(
                "behaviour table rows are out of order: expected the fixed provider order, then \
                 model id ascending, then the adapter-wide fallback row"
                    .to_string(),
            );
        }

        for cells in &rows {
            let (provider, model) = (&cells[0], &cells[1]);
            if model == FALLBACK_LABEL {
                if cells[2] != UNVERIFIED {
                    return Err(format!(
                        "the adapter-wide fallback row of '{provider}' must read `{UNVERIFIED}` \
                         in the verified-on column, found `{}`",
                        cells[2]
                    ));
                }
                continue;
            }
            let entry = seeded_for(provider)
                .iter()
                .find(|entry| format!("`{}`", entry.model) == *model)
                .ok_or_else(|| format!("no seeded entry for {provider} {model}"))?;
            let want = expected_cells(provider, entry);
            if let Some(column) = (0..BEHAVIOUR_COLUMNS).find(|&i| cells[i] != want[i]) {
                return Err(format!(
                    "{provider} {model}: column {column} reads `{}` on the page but the registry \
                     says `{}`",
                    cells[column], want[column]
                ));
            }
        }

        let responses = response_providers(page)?;
        if responses != PROVIDERS {
            return Err(format!(
                "the response-mapping table must list the nine providers in the fixed order, \
                 found {responses:?}"
            ));
        }
        Ok(())
    }

    /// Read and check the page under `root`: skipped only when `root` has no `docs/` directory.
    fn check_page_in(root: &Path) -> Result<Outcome, String> {
        if !root.join("docs").is_dir() {
            return Ok(Outcome::Skipped);
        }
        let path = root.join(PAGE);
        let page = std::fs::read_to_string(&path).map_err(|error| {
            format!(
                "docs/ exists but {} cannot be read: {error}",
                path.display()
            )
        })?;
        check_page_text(&page).map(|()| Outcome::Checked)
    }

    /// The real page, or `None` in a tree without `docs/` (the mutation checks below then have
    /// nothing to mutate and pass vacuously, exactly like the main check skips).
    fn real_page() -> Option<String> {
        let root = workspace_root();
        if !root.join("docs").is_dir() {
            return None;
        }
        Some(
            std::fs::read_to_string(root.join(PAGE))
                .unwrap_or_else(|error| panic!("docs/ exists but the page is unreadable: {error}")),
        )
    }

    /// A fresh scratch directory under the system temp dir.
    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("paladin-switching-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap_or_else(|error| panic!("scratch dir: {error}"));
        dir
    }

    /// Apply `edit` to the first line containing `needle`.
    fn edit_line(page: &str, needle: &str, edit: impl Fn(&str) -> String) -> String {
        let mut done = false;
        page.lines()
            .map(|line| {
                if !done && line.contains(needle) {
                    done = true;
                    edit(line)
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn switching_providers_page_lists_every_seeded_model() {
        let outcome = check_page_in(&workspace_root());
        assert!(
            matches!(outcome, Ok(Outcome::Checked | Outcome::Skipped)),
            "{}",
            outcome.err().unwrap_or_default()
        );
    }

    #[test]
    fn a_missing_seeded_row_fails_the_check() {
        let Some(page) = real_page() else { return };
        let mutated: String = page
            .lines()
            .filter(|line| !line.starts_with("| openai | `gpt-4o-mini` |"))
            .collect::<Vec<_>>()
            .join("\n");
        let error = check_page_text(&mutated).expect_err("a deleted row must fail");
        assert!(error.contains("gpt-4o-mini"), "{error}");
    }

    #[test]
    fn an_extra_row_fails_the_check() {
        let Some(page) = real_page() else { return };
        let mutated = edit_line(&page, "| kimi | `kimi-k3` |", |line| {
            format!("{line}\n{}", line.replace("kimi-k3", "kimi-k9"))
        });
        let error = check_page_text(&mutated).expect_err("an extra row must fail");
        assert!(error.contains("kimi-k9"), "{error}");
    }

    #[test]
    fn an_edited_verified_on_date_fails_the_check() {
        let Some(page) = real_page() else { return };
        let mutated = edit_line(&page, "| kimi | `kimi-k3` |", |line| {
            line.replace("2026-10-15", "2026-10-14")
        });
        let error = check_page_text(&mutated).expect_err("a changed date must fail");
        assert!(
            error.contains("kimi-k3") && error.contains("2026-10-14"),
            "{error}"
        );
    }

    #[test]
    fn an_edited_value_fails_the_check() {
        let Some(page) = real_page() else { return };
        let mutated = edit_line(&page, "| anthropic | `claude-haiku-5-5` |", |line| {
            line.replace("only 1.0", "0.0 to 1.0")
        });
        let error = check_page_text(&mutated).expect_err("a changed value must fail");
        assert!(error.contains("claude-haiku-5-5"), "{error}");
    }

    #[test]
    fn rows_out_of_order_fail_the_check() {
        let Some(page) = real_page() else { return };
        let mut lines: Vec<String> = page.lines().map(str::to_string).collect();
        let first = lines
            .iter()
            .position(|l| l.starts_with("| grok | `grok-4.3` |"));
        let second = lines
            .iter()
            .position(|l| l.starts_with("| grok | `grok-4.5` |"));
        let (Some(first), Some(second)) = (first, second) else {
            panic!("the page must carry the two grok rows");
        };
        lines.swap(first, second);
        let error = check_page_text(&lines.join("\n")).expect_err("a swapped pair must fail");
        assert!(error.contains("out of order"), "{error}");
    }

    #[test]
    fn a_provider_with_no_fallback_row_fails_the_check() {
        let Some(page) = real_page() else { return };
        let mutated: String = page
            .lines()
            .filter(|line| !line.starts_with("| openai-compatible | (adapter-wide fallback) |"))
            .collect::<Vec<_>>()
            .join("\n");
        let error = check_page_text(&mutated).expect_err("a missing fallback row must fail");
        assert!(error.contains("openai-compatible"), "{error}");
    }

    #[test]
    fn a_fallback_row_must_read_unverified() {
        let Some(page) = real_page() else { return };
        let mutated = edit_line(&page, "| ollama | (adapter-wide fallback) |", |line| {
            line.replace("unverified", "2026-10-15")
        });
        let error = check_page_text(&mutated).expect_err("a dated fallback row must fail");
        assert!(error.contains("ollama"), "{error}");
    }

    #[test]
    fn a_missing_response_mapping_row_fails_the_check() {
        let Some(page) = real_page() else { return };
        let mutated: String = page
            .lines()
            .filter(|line| !line.starts_with("| gemini | `EmptyCompletion`"))
            .collect::<Vec<_>>()
            .join("\n");
        let error = check_page_text(&mutated).expect_err("a missing mapping row must fail");
        assert!(error.contains("response-mapping"), "{error}");
    }

    #[test]
    fn a_tree_with_docs_but_without_the_page_fails() {
        let root = scratch("no-page");
        std::fs::create_dir_all(root.join("docs/src"))
            .unwrap_or_else(|error| panic!("scratch docs dir: {error}"));
        let outcome = check_page_in(&root);
        let _ = std::fs::remove_dir_all(&root);
        let error = outcome.expect_err("docs/ without the page must fail, never skip");
        assert!(error.contains("cannot be read"), "{error}");
    }

    #[test]
    fn a_tree_without_a_docs_directory_is_skipped() {
        let root = scratch("no-docs");
        let outcome = check_page_in(&root);
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(outcome, Ok(Outcome::Skipped));
    }

    #[test]
    fn marker_stripping_removes_only_footnote_markers() {
        assert_eq!(strip_markers("0.0 to 1.0 [a]"), "0.0 to 1.0");
        assert_eq!(strip_markers("128,000 [d]"), "128,000");
        assert_eq!(strip_markers("object, schema"), "object, schema");
        assert_eq!(strip_markers("a [bc] d"), "a [bc] d");
    }

    #[test]
    fn thousands_separators_group_by_three() {
        assert_eq!(with_thousands(0), "0");
        assert_eq!(with_thousands(999), "999");
        assert_eq!(with_thousands(1_000), "1,000");
        assert_eq!(with_thousands(1_048_576), "1,048,576");
    }
}

/// The `with_base_url` / `with_timeout_seconds` builders `config_knob_builders!` generates on
/// every `*Config` type (Phase 50.1 plan 02, SURFACE-02, D-05, D-00d: one mechanism, nine types).
#[cfg(test)]
mod endpoint_builder_tests {
    /// An endpoint no vendor serves, so a builder that fell through to a default is visible.
    const URL: &str = "https://example.invalid/v1";

    /// Every compiled `*Config` takes the same two builders and stores their values in its
    /// plain `base_url` / `timeout_seconds` fields. One feature-gated block per type, each
    /// building its config with that type's own `new`.
    #[test]
    fn knob_builders_set_base_url_and_timeout_on_every_config() {
        #[cfg(feature = "openai")]
        {
            let config = crate::openai::OpenAIConfig::new("k".to_string())
                .with_base_url(URL)
                .with_timeout_seconds(7);
            assert_eq!((config.base_url.as_str(), config.timeout_seconds), (URL, 7));
        }
        #[cfg(feature = "anthropic")]
        {
            let config = crate::anthropic::AnthropicConfig::new(
                "k".to_string(),
                "https://api.anthropic.com".to_string(),
                "claude-haiku-5-5".to_string(),
                16,
            )
            .with_base_url(URL)
            .with_timeout_seconds(7);
            assert_eq!((config.base_url.as_str(), config.timeout_seconds), (URL, 7));
        }
        #[cfg(feature = "deepseek")]
        {
            let config =
                crate::deepseek::DeepSeekConfig::new("k".to_string(), "deepseek-chat".to_string())
                    .with_base_url(URL)
                    .with_timeout_seconds(7);
            assert_eq!((config.base_url.as_str(), config.timeout_seconds), (URL, 7));
        }
        #[cfg(feature = "gemini")]
        {
            let config =
                crate::gemini::GeminiConfig::new("k".to_string(), "gemini-3.5-flash".to_string())
                    .with_base_url(URL)
                    .with_timeout_seconds(7);
            assert_eq!((config.base_url.as_str(), config.timeout_seconds), (URL, 7));
        }
        #[cfg(feature = "kimi")]
        {
            let config = crate::kimi::KimiConfig::new("k".to_string(), "kimi-k3".to_string())
                .with_base_url(URL)
                .with_timeout_seconds(7);
            assert_eq!((config.base_url.as_str(), config.timeout_seconds), (URL, 7));
        }
        #[cfg(feature = "qwen")]
        {
            let config = crate::qwen::QwenConfig::new("k".to_string(), "qwen-plus".to_string())
                .with_base_url(URL)
                .with_timeout_seconds(7);
            assert_eq!((config.base_url.as_str(), config.timeout_seconds), (URL, 7));
        }
        #[cfg(feature = "grok")]
        {
            let config = crate::grok::GrokConfig::new("k".to_string(), "grok-4.3".to_string())
                .with_base_url(URL)
                .with_timeout_seconds(7);
            assert_eq!((config.base_url.as_str(), config.timeout_seconds), (URL, 7));
        }
        #[cfg(feature = "ollama")]
        {
            // `OllamaConfig::new` keeps the endpoint as its first argument (D-08); the builder
            // replaces it all the same.
            let config = crate::ollama::OllamaConfig::new(
                "http://localhost:11434".to_string(),
                "llama3".to_string(),
            )
            .with_base_url(URL)
            .with_timeout_seconds(7);
            assert_eq!((config.base_url.as_str(), config.timeout_seconds), (URL, 7));
        }
        #[cfg(feature = "openai-compatible")]
        {
            let capabilities =
                crate::openai_compatible::OpenAiCompatibleCapabilitiesConfig::from_lookup(&|_| {
                    None
                })
                .expect("no variables set is the conservative default");
            let config = crate::openai_compatible::OpenAiCompatibleConfig::new(
                "k".to_string(),
                "https://gateway.example/v1".to_string(),
                "some-model".to_string(),
                capabilities,
            )
            .with_base_url(URL)
            .with_timeout_seconds(7);
            assert_eq!((config.base_url.as_str(), config.timeout_seconds), (URL, 7));
        }
    }
}
