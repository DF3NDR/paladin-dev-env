//! Per-model parameter support and limits (ADR-0061, amending ADR-0004).
//!
//! Which sampling parameters a provider accepts, and with what values, varies **per model**, not
//! per adapter. [`ModelProfile`] states that for one (provider, model) pair. It is returned by
//! [`LlmPort::model_profile`](crate::output::llm_port::LlmPort::model_profile) and sits *beside*
//! the adapter-wide [`ProviderCapabilities`](crate::output::llm_port::ProviderCapabilities), which
//! is deliberately left untouched.
//!
//! # Policy in one paragraph
//!
//! An explicit sampling parameter the target model cannot honour is refused at build time, by
//! name. An auto-selected one is omitted from the wire with a `warn!` and a
//! `paladin.params_omitted` response-metadata entry. Nothing is ever clamped, snapped or rounded:
//! [`ParamSupport::admits`] is the single admission rule both the build-time check and the wire
//! filter use.
//!
//! # Fail closed
//!
//! [`ModelProfile::new`] states **nothing as sendable**. A profile that forgets a field refuses an
//! explicit value loudly instead of sending it. This is the lesson of the compat engine's
//! `CompatRequestParameters`, which has no `Default` because a struct-update spread of one once
//! made a vendor receive a parameter it rejects. `ModelProfile` has no `Default` either.
//!
//! # Future proofing
//!
//! [`ModelProfile`] and [`SamplingParam`] are `#[non_exhaustive]`, so a later phase can add effort
//! levels or budget bounds without a breaking change. Construct a profile only through
//! [`ModelProfile::new`] plus the `with_*` setters.

use serde::{Deserialize, Serialize};

/// What a model accepts for one numeric sampling parameter (temperature, `top_p`, a penalty).
///
/// The set of variants is closed by ADR-0061, so the enum is not `#[non_exhaustive]`.
///
/// - [`Range`](Self::Range) -- a span of accepted values, **both ends inclusive**.
/// - [`Fixed`](Self::Fixed) -- exactly one accepted value (for example Kimi's temperature `1.0`).
/// - [`NotSent`](Self::NotSent) -- the parameter must not appear on the wire at all, so the
///   vendor's own default applies.
///
/// # Examples
///
/// ```
/// use paladin_ports::output::model_profile::ParamSupport;
///
/// assert!(ParamSupport::Range(0.0, 1.0).admits(1.0));
/// assert!(ParamSupport::Fixed(1.0).admits(1.0));
/// assert!(!ParamSupport::Fixed(1.0).admits(0.5));
/// assert!(!ParamSupport::NotSent.admits(0.0));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParamSupport {
    /// An inclusive span `(lo, hi)`: a value `v` is accepted when `lo <= v <= hi`.
    Range(f32, f32),
    /// Exactly one accepted value: a value `v` is accepted when `v == x` (exact `f32` equality).
    Fixed(f32),
    /// The parameter is never sent; nothing is accepted.
    NotSent,
}

impl ParamSupport {
    /// The single admission rule (ADR-0061 (d)).
    ///
    /// - `Range(lo, hi)` admits `lo <= value <= hi`, both ends inclusive, with no epsilon.
    /// - `Fixed(x)` admits only `value == x`.
    /// - `NotSent` admits nothing.
    /// - `NaN` is admitted by none.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_ports::output::model_profile::ParamSupport;
    ///
    /// let range = ParamSupport::Range(0.0, 2.0);
    /// assert!(range.admits(0.0) && range.admits(2.0));
    /// assert!(!range.admits(2.0_f32.next_up()));
    /// assert!(!range.admits(f32::NAN));
    /// ```
    #[must_use]
    pub fn admits(&self, value: f32) -> bool {
        match *self {
            Self::Range(lo, hi) => value >= lo && value <= hi,
            Self::Fixed(fixed) => value == fixed,
            Self::NotSent => false,
        }
    }

    /// The support that is acceptable to **both** `self` and `other`.
    ///
    /// `NotSent` absorbs; two `Fixed` values intersect only when equal; a `Fixed` against a
    /// `Range` survives only when the range admits it; two ranges intersect to their overlap, or
    /// to `NotSent` when disjoint. A `NaN` bound or an inverted range states nothing sendable and
    /// is treated as `NotSent`. The operation is commutative and associative.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_ports::output::model_profile::ParamSupport;
    ///
    /// let wide = ParamSupport::Range(0.0, 2.0);
    /// let narrow = ParamSupport::Range(0.5, 1.0);
    /// assert_eq!(wide.intersect(&narrow), narrow);
    /// assert_eq!(wide.intersect(&ParamSupport::NotSent), ParamSupport::NotSent);
    /// ```
    #[must_use]
    pub fn intersect(&self, other: &Self) -> Self {
        match (self.normalised(), other.normalised()) {
            (Self::NotSent, _) | (_, Self::NotSent) => Self::NotSent,
            (Self::Fixed(a), Self::Fixed(b)) => {
                if a == b {
                    Self::Fixed(a)
                } else {
                    Self::NotSent
                }
            }
            (Self::Fixed(a), range @ Self::Range(..))
            | (range @ Self::Range(..), Self::Fixed(a)) => {
                if range.admits(a) {
                    Self::Fixed(a)
                } else {
                    Self::NotSent
                }
            }
            (Self::Range(lo_a, hi_a), Self::Range(lo_b, hi_b)) => {
                let lo = if lo_a >= lo_b { lo_a } else { lo_b };
                let hi = if hi_a <= hi_b { hi_a } else { hi_b };
                if lo <= hi {
                    Self::Range(lo, hi)
                } else {
                    Self::NotSent
                }
            }
        }
    }

    /// Collapse a value that states nothing sendable (a `NaN` bound, an inverted range) to
    /// `NotSent`, so `intersect` never has to reason about it.
    fn normalised(self) -> Self {
        match self {
            Self::Range(lo, hi) if lo.is_nan() || hi.is_nan() || lo > hi => Self::NotSent,
            Self::Fixed(v) if v.is_nan() => Self::NotSent,
            other => other,
        }
    }
}

/// What a model accepts for stop sequences.
///
/// # Examples
///
/// ```
/// use paladin_ports::output::model_profile::StopSupport;
///
/// let stop = StopSupport::Supported { max: Some(4) };
/// assert!(stop.admits(4));
/// assert!(!stop.admits(5));
/// assert!(!StopSupport::NotSent.admits(1));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopSupport {
    /// Stop sequences are never sent.
    NotSent,
    /// Stop sequences are accepted, up to `max` of them (`None` means no known limit).
    Supported {
        /// The most stop sequences one request may carry, when the vendor states a limit.
        max: Option<u32>,
    },
}

impl StopSupport {
    /// Whether a request carrying `count` stop sequences is acceptable.
    ///
    /// `NotSent` admits only an empty list. `Supported { max: None }` admits any count.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_ports::output::model_profile::StopSupport;
    ///
    /// assert!(StopSupport::NotSent.admits(0));
    /// assert!(StopSupport::Supported { max: None }.admits(100));
    /// ```
    #[must_use]
    pub fn admits(&self, count: usize) -> bool {
        match *self {
            Self::NotSent => count == 0,
            Self::Supported { max: None } => true,
            Self::Supported { max: Some(max) } => {
                usize::try_from(max).map_or(true, |limit| count <= limit)
            }
        }
    }

    /// The support acceptable to both `self` and `other`: `NotSent` absorbs, and the smaller known
    /// maximum wins. Commutative and associative.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_ports::output::model_profile::StopSupport;
    ///
    /// let a = StopSupport::Supported { max: Some(4) };
    /// let b = StopSupport::Supported { max: Some(16) };
    /// assert_eq!(a.intersect(&b), a);
    /// assert_eq!(a.intersect(&StopSupport::NotSent), StopSupport::NotSent);
    /// ```
    #[must_use]
    pub fn intersect(&self, other: &Self) -> Self {
        match (*self, *other) {
            (Self::NotSent, _) | (_, Self::NotSent) => Self::NotSent,
            (Self::Supported { max: a }, Self::Supported { max: b }) => Self::Supported {
                max: min_known(a, b),
            },
        }
    }
}

/// The sampling parameters a [`ModelProfile`] speaks for, named for refusal and omission
/// messages.
///
/// # Examples
///
/// ```
/// use paladin_ports::output::model_profile::SamplingParam;
///
/// assert_eq!(SamplingParam::TopP.as_str(), "top_p");
/// assert_eq!(SamplingParam::StopSequences.as_str(), "stop");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SamplingParam {
    /// `temperature`
    Temperature,
    /// `top_p`
    TopP,
    /// `frequency_penalty`
    FrequencyPenalty,
    /// `presence_penalty`
    PresencePenalty,
    /// `stop` (stop sequences)
    StopSequences,
}

impl SamplingParam {
    /// The parameter's plain wire-style name, as used in error messages and in the
    /// `paladin.params_omitted` metadata entry.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Temperature => "temperature",
            Self::TopP => "top_p",
            Self::FrequencyPenalty => "frequency_penalty",
            Self::PresencePenalty => "presence_penalty",
            Self::StopSequences => "stop",
        }
    }
}

/// What one model of one provider accepts, and its limits (ADR-0061).
///
/// Built only through [`ModelProfile::new`] plus the `with_*` setters; the struct is
/// `#[non_exhaustive]` so fields can be added later. [`ModelProfile::new`] is **fail closed**:
/// every sampling parameter is `NotSent`, so a profile that forgets a field refuses an explicit
/// value instead of sending it.
///
/// # Examples
///
/// ```
/// use paladin_ports::output::model_profile::{ModelProfile, ParamSupport, SamplingParam};
///
/// let profile = ModelProfile::new()
///     .with_temperature(ParamSupport::Fixed(1.0))
///     .with_context_window(262_144)
///     .with_verified_on("2026-10-15");
///
/// assert_eq!(
///     profile.support_for(SamplingParam::Temperature),
///     Some(ParamSupport::Fixed(1.0))
/// );
/// assert_eq!(
///     profile.support_for(SamplingParam::TopP),
///     Some(ParamSupport::NotSent)
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ModelProfile {
    /// What the model accepts for `temperature`.
    pub temperature: ParamSupport,
    /// What the model accepts for `top_p`.
    pub top_p: ParamSupport,
    /// What the model accepts for `frequency_penalty`.
    pub frequency_penalty: ParamSupport,
    /// What the model accepts for `presence_penalty`.
    pub presence_penalty: ParamSupport,
    /// What the model accepts for stop sequences.
    pub stop_sequences: StopSupport,
    /// The model's context window in tokens, when known.
    pub context_window: Option<u32>,
    /// The most tokens the model can generate in one response, when known.
    pub max_output_tokens: Option<u32>,
    /// Whether the model is a reasoning model.
    pub reasoning: bool,
    /// Whether the model supports a JSON-object `response_format`.
    pub json_object: bool,
    /// Whether the model supports a JSON-schema `response_format`.
    pub json_schema: bool,
    /// The date (`YYYY-MM-DD`) this entry was last checked against the vendor, when known.
    pub verified_on: Option<String>,
}

impl ModelProfile {
    /// A profile that states nothing as sendable (fail closed).
    ///
    /// Every sampling parameter and stop sequences are `NotSent`, both windows are `None`,
    /// `reasoning`, `json_object` and `json_schema` are `false`, and `verified_on` is `None`.
    /// Opt each capability in with a `with_*` setter.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_ports::output::model_profile::{ModelProfile, ParamSupport};
    ///
    /// let profile = ModelProfile::new();
    /// assert_eq!(profile.temperature, ParamSupport::NotSent);
    /// assert!(!profile.json_object);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            temperature: ParamSupport::NotSent,
            top_p: ParamSupport::NotSent,
            frequency_penalty: ParamSupport::NotSent,
            presence_penalty: ParamSupport::NotSent,
            stop_sequences: StopSupport::NotSent,
            context_window: None,
            max_output_tokens: None,
            reasoning: false,
            json_object: false,
            json_schema: false,
            verified_on: None,
        }
    }

    /// Set what the model accepts for `temperature`.
    #[must_use]
    pub fn with_temperature(mut self, support: ParamSupport) -> Self {
        self.temperature = support;
        self
    }

    /// Set what the model accepts for `top_p`.
    #[must_use]
    pub fn with_top_p(mut self, support: ParamSupport) -> Self {
        self.top_p = support;
        self
    }

    /// Set what the model accepts for `frequency_penalty`.
    #[must_use]
    pub fn with_frequency_penalty(mut self, support: ParamSupport) -> Self {
        self.frequency_penalty = support;
        self
    }

    /// Set what the model accepts for `presence_penalty`.
    #[must_use]
    pub fn with_presence_penalty(mut self, support: ParamSupport) -> Self {
        self.presence_penalty = support;
        self
    }

    /// Set what the model accepts for stop sequences.
    #[must_use]
    pub fn with_stop_sequences(mut self, support: StopSupport) -> Self {
        self.stop_sequences = support;
        self
    }

    /// Set the context window in tokens.
    #[must_use]
    pub fn with_context_window(mut self, tokens: u32) -> Self {
        self.context_window = Some(tokens);
        self
    }

    /// Set the maximum tokens the model can generate in one response.
    #[must_use]
    pub fn with_max_output_tokens(mut self, tokens: u32) -> Self {
        self.max_output_tokens = Some(tokens);
        self
    }

    /// Set whether the model is a reasoning model.
    #[must_use]
    pub fn with_reasoning(mut self, reasoning: bool) -> Self {
        self.reasoning = reasoning;
        self
    }

    /// Set whether the model supports a JSON-object `response_format`.
    #[must_use]
    pub fn with_json_object(mut self, supported: bool) -> Self {
        self.json_object = supported;
        self
    }

    /// Set whether the model supports a JSON-schema `response_format`.
    #[must_use]
    pub fn with_json_schema(mut self, supported: bool) -> Self {
        self.json_schema = supported;
        self
    }

    /// Record the date (`YYYY-MM-DD`) this entry was last checked against the vendor.
    #[must_use]
    pub fn with_verified_on(mut self, date: impl Into<String>) -> Self {
        self.verified_on = Some(date.into());
        self
    }

    /// The support the profile states for a numeric sampling parameter.
    ///
    /// Returns `None` for [`SamplingParam::StopSequences`], which has its own field
    /// ([`stop_sequences`](Self::stop_sequences)) because it is a list, not a number.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_ports::output::model_profile::{ModelProfile, SamplingParam};
    ///
    /// assert_eq!(ModelProfile::new().support_for(SamplingParam::StopSequences), None);
    /// ```
    #[must_use]
    pub fn support_for(&self, param: SamplingParam) -> Option<ParamSupport> {
        match param {
            SamplingParam::Temperature => Some(self.temperature),
            SamplingParam::TopP => Some(self.top_p),
            SamplingParam::FrequencyPenalty => Some(self.frequency_penalty),
            SamplingParam::PresencePenalty => Some(self.presence_penalty),
            SamplingParam::StopSequences => None,
        }
    }

    /// The strictest profile acceptable to both `self` and `other`.
    ///
    /// The five parameters are intersected; the smaller known context window and maximum output
    /// are kept; `reasoning` is OR-ed (a chain that may reach a reasoning model must treat it as
    /// one); `json_object` and `json_schema` are AND-ed (a format is usable only if every hop
    /// supports it); `verified_on` is cleared because a combination was never itself verified.
    /// Commutative and associative.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_ports::output::model_profile::{ModelProfile, ParamSupport};
    ///
    /// let a = ModelProfile::new().with_temperature(ParamSupport::Range(0.0, 2.0));
    /// let b = ModelProfile::new().with_temperature(ParamSupport::Range(0.0, 1.0));
    /// assert_eq!(a.intersect(&b).temperature, ParamSupport::Range(0.0, 1.0));
    /// ```
    #[must_use]
    pub fn intersect(&self, other: &Self) -> Self {
        Self {
            temperature: self.temperature.intersect(&other.temperature),
            top_p: self.top_p.intersect(&other.top_p),
            frequency_penalty: self.frequency_penalty.intersect(&other.frequency_penalty),
            presence_penalty: self.presence_penalty.intersect(&other.presence_penalty),
            stop_sequences: self.stop_sequences.intersect(&other.stop_sequences),
            context_window: min_known(self.context_window, other.context_window),
            max_output_tokens: min_known(self.max_output_tokens, other.max_output_tokens),
            reasoning: self.reasoning || other.reasoning,
            json_object: self.json_object && other.json_object,
            json_schema: self.json_schema && other.json_schema,
            verified_on: None,
        }
    }
}

impl Default for ModelProfile {
    /// Identical to [`ModelProfile::new`]: fail closed. Provided so containers that need
    /// `Default` can hold a profile; never use it as a struct-update base.
    fn default() -> Self {
        Self::new()
    }
}

/// The smaller of two optional limits, ignoring an unknown (`None`) side.
fn min_known(a: Option<u32>, b: Option<u32>) -> Option<u32> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.min(y)),
        (some @ Some(_), None) | (None, some @ Some(_)) => some,
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Representative values around the edges the table cases exercise.
    fn probe_values() -> Vec<f32> {
        vec![
            f32::NEG_INFINITY,
            -1.0,
            0.0_f32.next_down(),
            0.0,
            0.5,
            0.999_999_9,
            1.0,
            1.0_f32.next_up(),
            1.5,
            2.0,
            2.0_f32.next_up(),
            f32::INFINITY,
            f32::NAN,
        ]
    }

    fn support_table() -> Vec<ParamSupport> {
        vec![
            ParamSupport::NotSent,
            ParamSupport::Fixed(1.0),
            ParamSupport::Fixed(0.5),
            ParamSupport::Fixed(f32::NAN),
            ParamSupport::Range(0.0, 1.0),
            ParamSupport::Range(0.0, 2.0),
            ParamSupport::Range(1.0, 2.0),
            ParamSupport::Range(0.5, 0.5),
            ParamSupport::Range(1.0, 1.0),
            ParamSupport::Range(1.0, 0.0),
            ParamSupport::Range(f32::NAN, 1.0),
        ]
    }

    #[test]
    fn param_support_admits_table() {
        let range = ParamSupport::Range(0.0, 1.0);
        // Both ends inclusive.
        assert!(range.admits(0.0));
        assert!(range.admits(1.0));
        // The next representable f32 outside either end is not admitted.
        assert!(!range.admits(0.0_f32.next_down()));
        assert!(!range.admits(1.0_f32.next_up()));

        let fixed = ParamSupport::Fixed(1.0);
        assert!(fixed.admits(1.0));
        assert!(!fixed.admits(0.999_999_9));
        assert!(!fixed.admits(1.0_f32.next_up()));

        for v in probe_values() {
            assert!(!ParamSupport::NotSent.admits(v), "NotSent admitted {v}");
        }

        // NaN is admitted by no variant, including a degenerate or NaN-bounded one.
        for s in support_table() {
            assert!(!s.admits(f32::NAN), "{s:?} admitted NaN");
        }

        // Range(x, x) admits exactly what Fixed(x) admits.
        for x in [0.0_f32, 0.5, 1.0, 2.0] {
            for v in probe_values() {
                assert_eq!(
                    ParamSupport::Range(x, x).admits(v),
                    ParamSupport::Fixed(x).admits(v),
                    "Range({x}, {x}) and Fixed({x}) disagree on {v}"
                );
            }
        }
    }

    #[test]
    fn intersect_is_order_independent() {
        let table = support_table();
        for a in &table {
            for b in &table {
                assert_eq!(
                    a.intersect(b),
                    b.intersect(a),
                    "intersect not commutative for {a:?}, {b:?}"
                );
                for c in &table {
                    assert_eq!(
                        a.intersect(b).intersect(c),
                        a.intersect(&b.intersect(c)),
                        "intersect not associative for {a:?}, {b:?}, {c:?}"
                    );
                }
            }
        }

        let stops = [
            StopSupport::NotSent,
            StopSupport::Supported { max: None },
            StopSupport::Supported { max: Some(4) },
            StopSupport::Supported { max: Some(16) },
        ];
        for a in &stops {
            for b in &stops {
                assert_eq!(a.intersect(b), b.intersect(a));
                for c in &stops {
                    assert_eq!(a.intersect(b).intersect(c), a.intersect(&b.intersect(c)));
                }
            }
        }

        let profiles = [
            ModelProfile::new(),
            ModelProfile::new()
                .with_temperature(ParamSupport::Range(0.0, 2.0))
                .with_stop_sequences(StopSupport::Supported { max: Some(4) })
                .with_context_window(128_000)
                .with_json_object(true)
                .with_json_schema(true),
            ModelProfile::new()
                .with_temperature(ParamSupport::Fixed(1.0))
                .with_top_p(ParamSupport::Range(0.0, 1.0))
                .with_max_output_tokens(8_192)
                .with_reasoning(true)
                .with_json_object(true),
        ];
        for a in &profiles {
            for b in &profiles {
                assert_eq!(a.intersect(b), b.intersect(a));
                for c in &profiles {
                    assert_eq!(a.intersect(b).intersect(c), a.intersect(&b.intersect(c)));
                }
            }
        }
    }

    #[test]
    fn intersect_follows_the_documented_cases() {
        use ParamSupport::{Fixed, NotSent, Range};
        assert_eq!(NotSent.intersect(&Range(0.0, 1.0)), NotSent);
        assert_eq!(Fixed(1.0).intersect(&Fixed(1.0)), Fixed(1.0));
        assert_eq!(Fixed(1.0).intersect(&Fixed(0.5)), NotSent);
        assert_eq!(Fixed(1.0).intersect(&Range(0.0, 1.0)), Fixed(1.0));
        assert_eq!(Fixed(1.5).intersect(&Range(0.0, 1.0)), NotSent);
        assert_eq!(Range(0.0, 2.0).intersect(&Range(0.5, 3.0)), Range(0.5, 2.0));
        assert_eq!(Range(0.0, 1.0).intersect(&Range(2.0, 3.0)), NotSent);
        // Touching ranges overlap in exactly one point.
        assert_eq!(Range(0.0, 1.0).intersect(&Range(1.0, 2.0)), Range(1.0, 1.0));
    }

    #[test]
    fn stop_support_admits_and_intersects() {
        let four = StopSupport::Supported { max: Some(4) };
        assert!(four.admits(0));
        assert!(four.admits(4));
        assert!(!four.admits(5));
        assert!(StopSupport::Supported { max: None }.admits(10_000));
        assert!(StopSupport::NotSent.admits(0));
        assert!(!StopSupport::NotSent.admits(1));

        let sixteen = StopSupport::Supported { max: Some(16) };
        let unbounded = StopSupport::Supported { max: None };
        assert_eq!(four.intersect(&sixteen), four);
        assert_eq!(unbounded.intersect(&sixteen), sixteen);
        assert_eq!(unbounded.intersect(&unbounded), unbounded);
        assert_eq!(four.intersect(&StopSupport::NotSent), StopSupport::NotSent);
    }

    #[test]
    fn model_profile_new_is_fail_closed() {
        let p = ModelProfile::new();
        assert_eq!(p.temperature, ParamSupport::NotSent);
        assert_eq!(p.top_p, ParamSupport::NotSent);
        assert_eq!(p.frequency_penalty, ParamSupport::NotSent);
        assert_eq!(p.presence_penalty, ParamSupport::NotSent);
        assert_eq!(p.stop_sequences, StopSupport::NotSent);
        assert_eq!(p.context_window, None);
        assert_eq!(p.max_output_tokens, None);
        assert!(!p.reasoning && !p.json_object && !p.json_schema);
        assert_eq!(p.verified_on, None);
        assert_eq!(ModelProfile::default(), p);
        // A fail-closed profile refuses every explicit numeric value.
        for param in [
            SamplingParam::Temperature,
            SamplingParam::TopP,
            SamplingParam::FrequencyPenalty,
            SamplingParam::PresencePenalty,
        ] {
            let support = p.support_for(param);
            assert!(support.is_some_and(|s| !s.admits(0.0)));
        }
        assert_eq!(p.support_for(SamplingParam::StopSequences), None);
    }

    #[test]
    fn each_with_setter_sets_exactly_one_field() {
        let base = ModelProfile::new();
        let cases: Vec<(&str, ModelProfile)> = vec![
            (
                "temperature",
                base.clone().with_temperature(ParamSupport::Fixed(1.0)),
            ),
            ("top_p", base.clone().with_top_p(ParamSupport::Fixed(1.0))),
            (
                "frequency_penalty",
                base.clone()
                    .with_frequency_penalty(ParamSupport::Fixed(0.0)),
            ),
            (
                "presence_penalty",
                base.clone().with_presence_penalty(ParamSupport::Fixed(0.0)),
            ),
            (
                "stop_sequences",
                base.clone()
                    .with_stop_sequences(StopSupport::Supported { max: Some(2) }),
            ),
            ("context_window", base.clone().with_context_window(1)),
            ("max_output_tokens", base.clone().with_max_output_tokens(1)),
            ("reasoning", base.clone().with_reasoning(true)),
            ("json_object", base.clone().with_json_object(true)),
            ("json_schema", base.clone().with_json_schema(true)),
            ("verified_on", base.clone().with_verified_on("2026-10-15")),
        ];
        for (name, changed) in cases {
            assert_ne!(changed, base, "with_{name} changed nothing");
            // Reverting that one field restores the base, so nothing else moved.
            let mut restored = changed.clone();
            match name {
                "temperature" => restored.temperature = base.temperature,
                "top_p" => restored.top_p = base.top_p,
                "frequency_penalty" => restored.frequency_penalty = base.frequency_penalty,
                "presence_penalty" => restored.presence_penalty = base.presence_penalty,
                "stop_sequences" => restored.stop_sequences = base.stop_sequences,
                "context_window" => restored.context_window = base.context_window,
                "max_output_tokens" => restored.max_output_tokens = base.max_output_tokens,
                "reasoning" => restored.reasoning = base.reasoning,
                "json_object" => restored.json_object = base.json_object,
                "json_schema" => restored.json_schema = base.json_schema,
                _ => restored.verified_on = base.verified_on.clone(),
            }
            assert_eq!(restored, base, "with_{name} touched another field");
        }
    }

    #[test]
    fn model_profile_intersect_combines_fields() {
        let a = ModelProfile::new()
            .with_temperature(ParamSupport::Range(0.0, 2.0))
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            .with_stop_sequences(StopSupport::Supported { max: Some(16) })
            .with_context_window(200_000)
            .with_max_output_tokens(64_000)
            .with_reasoning(false)
            .with_json_object(true)
            .with_json_schema(true)
            .with_verified_on("2026-10-15");
        let b = ModelProfile::new()
            .with_temperature(ParamSupport::Fixed(1.0))
            .with_top_p(ParamSupport::NotSent)
            .with_stop_sequences(StopSupport::Supported { max: Some(4) })
            .with_context_window(128_000)
            .with_reasoning(true)
            .with_json_object(true)
            .with_json_schema(false)
            .with_verified_on("2026-10-01");

        let both = a.intersect(&b);
        assert_eq!(both.temperature, ParamSupport::Fixed(1.0));
        assert_eq!(both.top_p, ParamSupport::NotSent);
        assert_eq!(both.frequency_penalty, ParamSupport::NotSent);
        assert_eq!(both.stop_sequences, StopSupport::Supported { max: Some(4) });
        assert_eq!(both.context_window, Some(128_000));
        // One side unknown: the known limit stands.
        assert_eq!(both.max_output_tokens, Some(64_000));
        assert!(both.reasoning);
        assert!(both.json_object);
        assert!(!both.json_schema);
        assert_eq!(both.verified_on, None);
    }

    #[test]
    fn sampling_param_names_are_stable() {
        assert_eq!(SamplingParam::Temperature.as_str(), "temperature");
        assert_eq!(SamplingParam::TopP.as_str(), "top_p");
        assert_eq!(
            SamplingParam::FrequencyPenalty.as_str(),
            "frequency_penalty"
        );
        assert_eq!(SamplingParam::PresencePenalty.as_str(), "presence_penalty");
        assert_eq!(SamplingParam::StopSequences.as_str(), "stop");
    }

    #[test]
    fn param_support_serde_round_trips() {
        for s in [
            ParamSupport::Range(0.0, 1.0),
            ParamSupport::Fixed(1.0),
            ParamSupport::NotSent,
        ] {
            let json = serde_json::to_string(&s).expect("serialise");
            let back: ParamSupport = serde_json::from_str(&json).expect("deserialise");
            assert_eq!(back, s);
        }
        assert_eq!(
            serde_json::to_string(&ParamSupport::NotSent).expect("serialise"),
            "\"not_sent\""
        );
    }
}
