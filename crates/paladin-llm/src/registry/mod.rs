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
// The registry is consumed by the compat engine today and by the bespoke adapters from plan
// 50-05; without a consuming adapter feature its crate-private items are legitimately unused.
// TEMPORARY (plan 50-03 task 1): removed in the next commit, when the compat engine consumes
// every item below.
#![allow(dead_code)]
#![cfg_attr(
    not(any(
        feature = "kimi",
        feature = "qwen",
        feature = "grok",
        feature = "ollama",
        feature = "openai-compatible"
    )),
    allow(dead_code)
)]

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
            fallback,
            warned: Mutex::new(HashSet::new()),
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
                (entry.build)().with_verified_on(entry.verified_on),
                Some(entry),
            ),
            None => (self.fallback.clone(), None),
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
/// - `stop_sequences`: the caller's `stop` (compat presets differ: Grok refuses it).
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
