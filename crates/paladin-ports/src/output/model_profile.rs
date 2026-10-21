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
//! [`ModelProfile`] and [`SamplingParam`] are `#[non_exhaustive]`, so a later phase can add
//! further capabilities without a breaking change. Construct a profile only through
//! [`ModelProfile::new`] plus the `with_*` setters.
//!
//! # Reasoning effort (Phase 51)
//!
//! [`EffortSupport`] states which reasoning efforts a model takes, and
//! [`EffortSupport::resolve`] is the **one** admission rule for a requested
//! [`ReasoningEffort`]: the adapters, the builder, the fallback pre-flight and the docs drift test
//! all call it, so they cannot drift apart (the [`ParamSupport::admits`] precedent). It is fail
//! closed: [`ModelProfile::new`] states [`EffortSupport::Unmeasured`], which refuses every effort.
//! A level is substituted only when the caller chose a substituting [`ReasoningFallback`].

use std::fmt;

use paladin_core::platform::container::reasoning::{ReasoningEffort, ReasoningFallback};
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

/// An inclusive span of reasoning-token budgets a model accepts (REASON-06).
///
/// Built with the infallible [`BudgetBounds::new`] so seed tables can state a span without a
/// panicking call; [`is_valid`](Self::is_valid) reports a span that states nothing usable
/// (`min == 0` or `min > max`). [`EffortSupport::resolve`] accepts no budget at all against an
/// invalid span (fail closed), and [`EffortLevels::validate`] reports it.
///
/// Marked `#[non_exhaustive]`: build it with [`BudgetBounds::new`].
///
/// # Examples
///
/// ```
/// use paladin_ports::output::model_profile::BudgetBounds;
///
/// let bounds = BudgetBounds::new(1024, 32_000);
/// assert!(bounds.is_valid());
/// assert!(!BudgetBounds::new(0, 10).is_valid());
/// assert!(!BudgetBounds::new(10, 5).is_valid());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct BudgetBounds {
    /// The smallest accepted budget in reasoning tokens (inclusive).
    pub min: u32,
    /// The largest accepted budget in reasoning tokens (inclusive).
    pub max: u32,
}

impl BudgetBounds {
    /// State a span of accepted budgets, both ends inclusive. Never fails; check
    /// [`is_valid`](Self::is_valid) on data that did not come from a trusted table.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_ports::output::model_profile::BudgetBounds;
    ///
    /// let bounds = BudgetBounds::new(1, u32::MAX);
    /// assert_eq!((bounds.min, bounds.max), (1, u32::MAX));
    /// ```
    #[must_use]
    pub const fn new(min: u32, max: u32) -> Self {
        Self { min, max }
    }

    /// Whether the span states something usable: `min >= 1` and `min <= max`.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_ports::output::model_profile::BudgetBounds;
    ///
    /// assert!(BudgetBounds::new(5, 5).is_valid());
    /// assert!(!BudgetBounds::new(0, 5).is_valid());
    /// ```
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        self.min != 0 && self.min <= self.max
    }
}

/// The reasoning efforts one model takes when it is `Supported`: its levels, whether `off` and
/// `auto` are accepted, and an optional budget span (REASON-02, REASON-06).
///
/// The `levels` are always the six ranked levels, strictly ascending; build them with
/// [`EffortLevels::new`], which normalises its input, and check a deserialised value with
/// [`EffortLevels::validate`]. An empty `levels` list is valid: it is a budget-only model such as
/// Qwen, which refuses every level whatever the fallback.
///
/// Marked `#[non_exhaustive]`: build it with [`EffortLevels::new`] plus the `with_*` setters.
///
/// # Examples
///
/// ```
/// use paladin_ports::output::model_profile::{BudgetBounds, EffortLevels};
/// use paladin_ports::output::llm_port::ReasoningEffort;
///
/// let levels = EffortLevels::new([ReasoningEffort::High, ReasoningEffort::Low])
///     .with_off(true)
///     .with_budget(BudgetBounds::new(1024, 32_000));
/// assert_eq!(levels.levels, [ReasoningEffort::Low, ReasoningEffort::High]);
/// assert!(levels.validate().is_ok());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct EffortLevels {
    /// The levels the model lists, strictly ascending by rank, with no `Off`, `Auto` or
    /// `Budget`. May be empty.
    pub levels: Vec<ReasoningEffort>,
    /// Whether reasoning can be switched off ([`ReasoningEffort::Off`] is accepted).
    #[serde(default)]
    pub off: bool,
    /// Whether the vendor's own default depth can be requested ([`ReasoningEffort::Auto`]).
    #[serde(default)]
    pub auto: bool,
    /// The reasoning-token budgets accepted, when the model takes a budget at all.
    #[serde(default)]
    pub budget: Option<BudgetBounds>,
    /// Levels the vendor accepts but silently runs as another. They are refused whatever the
    /// fallback and are never a substitution target (D-00f, D-07).
    #[serde(default)]
    pub coerced: Vec<ReasoningEffort>,
    /// A level is sent together with the vendor's explicit thinking-enable flag. Admission
    /// never reads it; it only shapes the wire form.
    #[serde(default)]
    pub enable_thinking_with_level: bool,
}

/// Keep only the six ranked levels, sorted by rank and de-duplicated.
fn normalise_levels(levels: impl IntoIterator<Item = ReasoningEffort>) -> Vec<ReasoningEffort> {
    let mut kept: Vec<ReasoningEffort> = levels.into_iter().filter(|e| e.is_level()).collect();
    kept.sort_by_key(|e| e.level_rank());
    kept.dedup();
    kept
}

/// Check that a deserialised level list holds only levels, strictly ascending.
fn check_levels(name: &str, list: &[ReasoningEffort]) -> Result<(), String> {
    let mut previous: Option<u8> = None;
    for effort in list {
        let Some(rank) = effort.level_rank() else {
            return Err(format!("{name} holds '{effort}', which is not a level"));
        };
        if let Some(before) = previous {
            if rank == before {
                return Err(format!("{name} repeats the level '{effort}'"));
            }
            if rank < before {
                return Err(format!("{name} is not sorted ascending at '{effort}'"));
            }
        }
        previous = Some(rank);
    }
    Ok(())
}

impl EffortLevels {
    /// List the levels a model takes. Infallible: anything that is not one of the six levels
    /// (`Off`, `Auto`, a `Budget`) is dropped, and the rest is sorted by rank and de-duplicated,
    /// so a seed table needs no panicking call. `off`, `auto`, `budget` and `coerced` start
    /// empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_ports::output::model_profile::EffortLevels;
    /// use paladin_ports::output::llm_port::ReasoningEffort::{High, Low, Off};
    ///
    /// assert_eq!(EffortLevels::new([High, Low, Low, Off]).levels, [Low, High]);
    /// assert!(EffortLevels::new([]).validate().is_ok());
    /// ```
    #[must_use]
    pub fn new(levels: impl IntoIterator<Item = ReasoningEffort>) -> Self {
        Self {
            levels: normalise_levels(levels),
            off: false,
            auto: false,
            budget: None,
            coerced: Vec::new(),
            enable_thinking_with_level: false,
        }
    }

    /// Set whether reasoning can be switched off.
    #[must_use]
    pub fn with_off(mut self, off: bool) -> Self {
        self.off = off;
        self
    }

    /// Set whether the vendor's own default depth can be requested.
    #[must_use]
    pub fn with_auto(mut self, auto: bool) -> Self {
        self.auto = auto;
        self
    }

    /// Set the budget span the model accepts.
    #[must_use]
    pub fn with_budget(mut self, bounds: BudgetBounds) -> Self {
        self.budget = Some(bounds);
        self
    }

    /// Set the levels the vendor accepts but silently runs as another. They are normalised like
    /// [`EffortLevels::new`] and removed from `levels` (fail closed: a coerced level is never
    /// both listed and refused).
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_ports::output::model_profile::EffortLevels;
    /// use paladin_ports::output::llm_port::ReasoningEffort::{High, Low};
    ///
    /// let levels = EffortLevels::new([Low, High]).with_coerced([Low]);
    /// assert_eq!(levels.levels, [High]);
    /// assert_eq!(levels.coerced, [Low]);
    /// ```
    #[must_use]
    pub fn with_coerced(mut self, coerced: impl IntoIterator<Item = ReasoningEffort>) -> Self {
        let coerced = normalise_levels(coerced);
        self.levels.retain(|level| !coerced.contains(level));
        self.coerced = coerced;
        self
    }

    /// Set whether a level is sent together with the vendor's explicit thinking-enable flag.
    #[must_use]
    pub fn with_enable_thinking_with_level(mut self, together: bool) -> Self {
        self.enable_thinking_with_level = together;
        self
    }

    /// Check data that did not come from [`EffortLevels::new`] (for example a deserialised
    /// override): `levels` and `coerced` hold only levels, strictly ascending and without
    /// repeats; the two lists are disjoint; a budget span is valid.
    ///
    /// # Errors
    ///
    /// Returns a plain-English description of the first problem found.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_ports::output::model_profile::EffortLevels;
    ///
    /// let bad: EffortLevels = serde_json::from_str(r#"{"levels":["high","low"]}"#)?;
    /// assert!(bad.validate().is_err());
    /// # Ok::<(), serde_json::Error>(())
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        check_levels("levels", &self.levels)?;
        check_levels("coerced", &self.coerced)?;
        if let Some(level) = self.levels.iter().find(|l| self.coerced.contains(l)) {
            return Err(format!("the level '{level}' is both listed and coerced"));
        }
        if let Some(bounds) = self.budget.filter(|b| !b.is_valid()) {
            return Err(format!(
                "the budget span {}..={} is invalid (min must be at least 1 and at most max)",
                bounds.min, bounds.max
            ));
        }
        Ok(())
    }
}

/// Intersect two budget spans: the overlap, or `None` when either side takes no (valid) budget
/// or the spans are disjoint. An invalid span states nothing usable and counts as no budget.
fn and_budget(a: Option<BudgetBounds>, b: Option<BudgetBounds>) -> Option<BudgetBounds> {
    let (a, b) = (
        a.filter(BudgetBounds::is_valid)?,
        b.filter(BudgetBounds::is_valid)?,
    );
    let (min, max) = (a.min.max(b.min), a.max.min(b.max));
    (min <= max).then(|| BudgetBounds::new(min, max))
}

impl EffortLevels {
    /// The admission arm for a `Supported` model; see [`EffortSupport::resolve`].
    fn admit(
        &self,
        requested: ReasoningEffort,
        fallback: ReasoningFallback,
    ) -> Result<ResolvedEffort, EffortRefusal> {
        // A level the vendor silently runs as another is refused whatever the fallback.
        if requested.is_level() && self.coerced.contains(&requested) {
            return Err(EffortRefusal::LevelCoerced);
        }
        match requested {
            ReasoningEffort::Off if self.off => Ok(ResolvedEffort::exact(requested, true)),
            ReasoningEffort::Off => Err(EffortRefusal::OffUnsupported),
            ReasoningEffort::Auto if self.auto => Ok(ResolvedEffort::exact(requested, true)),
            ReasoningEffort::Auto => Err(EffortRefusal::AutoUnsupported),
            ReasoningEffort::Budget(n) => match self.budget {
                None => Err(EffortRefusal::BudgetUnsupported),
                Some(bounds) if bounds.is_valid() && (bounds.min..=bounds.max).contains(&n) => {
                    Ok(ResolvedEffort::exact(requested, true))
                }
                Some(bounds) => Err(EffortRefusal::BudgetOutOfBounds(bounds)),
            },
            level => self.admit_level(level, fallback),
        }
    }

    /// Admit one of the six levels, substituting only along the caller's chosen direction.
    fn admit_level(
        &self,
        level: ReasoningEffort,
        fallback: ReasoningFallback,
    ) -> Result<ResolvedEffort, EffortRefusal> {
        let Some(rank) = level.level_rank() else {
            // A future non-level variant this rule does not know: fail closed.
            return Err(EffortRefusal::LevelUnsupported);
        };
        if self.levels.contains(&level) {
            return Ok(ResolvedEffort::exact(level, true));
        }
        // Candidates are listed levels only: a coerced level is never a substitution target.
        let candidates = self
            .levels
            .iter()
            .copied()
            .filter(|l| !self.coerced.contains(l))
            .filter_map(|l| l.level_rank().map(|r| (r, l)));
        let chosen = match fallback {
            ReasoningFallback::NearestLower => candidates
                .filter(|(r, _)| *r < rank)
                .max_by_key(|(r, _)| *r),
            ReasoningFallback::NearestHigher => candidates
                .filter(|(r, _)| *r > rank)
                .min_by_key(|(r, _)| *r),
            // `Refuse`, and any future fallback this rule does not know: fail closed.
            _ => return Err(EffortRefusal::LevelUnsupported),
        };
        chosen
            .map(|(_, effective)| ResolvedEffort::substitute(level, effective))
            .ok_or(EffortRefusal::NoLevelInDirection)
    }

    /// AND of two supports: a level, `off`, `auto` or budget span is usable only when both
    /// operands allow it; a level coerced on either side stays coerced (and unlisted).
    fn and(&self, other: &Self) -> Self {
        let coerced = normalise_levels(self.coerced.iter().chain(&other.coerced).copied());
        let listed = self
            .levels
            .iter()
            .filter(|l| other.levels.contains(l) && !coerced.contains(l))
            .copied();
        Self {
            levels: normalise_levels(listed),
            off: self.off && other.off,
            auto: self.auto && other.auto,
            budget: and_budget(self.budget, other.budget),
            coerced,
            enable_thinking_with_level: self.enable_thinking_with_level
                || other.enable_thinking_with_level,
        }
    }
}

/// Which reasoning efforts one model takes (REASON-02, D-03).
///
/// Fail closed: the [`Default`] is [`Unmeasured`](Self::Unmeasured), which refuses every effort,
/// so a model nobody has measured never receives one by guess.
///
/// - [`Unmeasured`](Self::Unmeasured) -- no measured data: refuses everything.
/// - [`WireUnsupported`](Self::WireUnsupported) -- the adapter has no wire form for an effort on
///   this model: refuses everything, `Off` included.
/// - [`NoReasoning`](Self::NoReasoning) -- the model has no reasoning to adjust: accepts only
///   `Off`, as an exact no-op that sends nothing.
/// - [`Supported`](Self::Supported) -- the model takes the efforts its [`EffortLevels`] list.
///
/// Marked `#[non_exhaustive]`: match it with a wildcard arm.
///
/// # Examples
///
/// ```
/// use paladin_ports::output::model_profile::{EffortLevels, EffortSupport};
/// use paladin_ports::output::llm_port::{ReasoningEffort, ReasoningFallback};
///
/// let support = EffortSupport::Supported(EffortLevels::new([ReasoningEffort::High]));
/// assert!(support.resolve(ReasoningEffort::High, ReasoningFallback::Refuse).is_ok());
/// assert!(EffortSupport::default()
///     .resolve(ReasoningEffort::High, ReasoningFallback::Refuse)
///     .is_err());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum EffortSupport {
    /// No measured effort data for this model. Refuses every effort.
    #[default]
    Unmeasured,
    /// The adapter has no wire form for an effort on this model. Refuses every effort.
    WireUnsupported,
    /// The model does not reason. Accepts only `Off`, as an exact no-op.
    NoReasoning,
    /// The model takes the efforts these levels list.
    Supported(EffortLevels),
}

/// The outcome of an accepted effort: what was asked, what reaches the wire, and whether
/// anything reaches the wire at all.
///
/// Marked `#[non_exhaustive]`: read it, do not construct it.
///
/// # Examples
///
/// ```
/// use paladin_ports::output::model_profile::{EffortLevels, EffortSupport};
/// use paladin_ports::output::llm_port::{ReasoningEffort, ReasoningFallback};
///
/// let support = EffortSupport::Supported(EffortLevels::new([ReasoningEffort::Low]));
/// let resolved = support.resolve(ReasoningEffort::Low, ReasoningFallback::Refuse)?;
/// assert_eq!(resolved.effective, ReasoningEffort::Low);
/// assert!(resolved.on_wire);
/// assert!(!resolved.substituted());
/// # Ok::<(), paladin_ports::output::model_profile::EffortRefusal>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ResolvedEffort {
    /// The effort the caller asked for.
    pub requested: ReasoningEffort,
    /// The effort that reaches the wire. Equal to `requested` unless the caller chose a
    /// substituting [`ReasoningFallback`] and a level was substituted.
    pub effective: ReasoningEffort,
    /// Whether anything is sent. `false` only for `Off` on a model that does not reason, an
    /// exact no-op.
    pub on_wire: bool,
}

impl ResolvedEffort {
    /// An effort accepted exactly as asked.
    fn exact(requested: ReasoningEffort, on_wire: bool) -> Self {
        Self {
            requested,
            effective: requested,
            on_wire,
        }
    }

    /// A level substituted for the one asked for (the caller chose a substituting fallback).
    fn substitute(requested: ReasoningEffort, effective: ReasoningEffort) -> Self {
        Self {
            requested,
            effective,
            on_wire: true,
        }
    }

    /// Whether a different effort than the one asked for reaches the wire.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_ports::output::model_profile::{EffortLevels, EffortSupport};
    /// use paladin_ports::output::llm_port::{ReasoningEffort, ReasoningFallback};
    ///
    /// let support = EffortSupport::Supported(EffortLevels::new([ReasoningEffort::High]));
    /// let resolved = support.resolve(ReasoningEffort::Max, ReasoningFallback::NearestLower)?;
    /// assert!(resolved.substituted());
    /// assert_eq!(resolved.effective, ReasoningEffort::High);
    /// # Ok::<(), paladin_ports::output::model_profile::EffortRefusal>(())
    /// ```
    #[must_use]
    pub fn substituted(&self) -> bool {
        self.requested != self.effective
    }
}

/// Why [`EffortSupport::resolve`] refused an effort.
///
/// The [`Display`](fmt::Display) text is plain English with no provider-specific wording; the
/// caller adds the provider, model and parameter (see `LlmError::UnsupportedParameter`).
///
/// Marked `#[non_exhaustive]`: match it with a wildcard arm.
///
/// # Examples
///
/// ```
/// use paladin_ports::output::model_profile::EffortRefusal;
///
/// assert!(EffortRefusal::Unmeasured.to_string().contains("measured"));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EffortRefusal {
    /// The model has no measured effort data.
    Unmeasured,
    /// The adapter has no wire form for an effort on this model.
    WireUnsupported,
    /// The model does not reason, so only `Off` could be honoured.
    NoReasoning,
    /// The model does not list the level asked for, and the fallback is `Refuse`.
    LevelUnsupported,
    /// The vendor accepts the level but silently runs it as another.
    LevelCoerced,
    /// The chosen fallback direction holds no listed level.
    NoLevelInDirection,
    /// The model cannot run with reasoning off.
    OffUnsupported,
    /// The model has no automatic reasoning depth.
    AutoUnsupported,
    /// The model takes no reasoning-token budget.
    BudgetUnsupported,
    /// The budget lies outside the span the model accepts (carried here).
    BudgetOutOfBounds(BudgetBounds),
}

impl fmt::Display for EffortRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unmeasured => f.write_str("no measured reasoning-effort data for this model"),
            Self::WireUnsupported => {
                f.write_str("no wire form for a reasoning effort on this model")
            }
            Self::NoReasoning => f.write_str("this model has no reasoning to adjust"),
            Self::LevelUnsupported => f.write_str("this model does not offer that effort level"),
            Self::LevelCoerced => {
                f.write_str("the vendor accepts that effort level but silently runs it as another")
            }
            Self::NoLevelInDirection => {
                f.write_str("no offered effort level lies in the chosen fallback direction")
            }
            Self::OffUnsupported => f.write_str("this model cannot run with reasoning off"),
            Self::AutoUnsupported => {
                f.write_str("this model has no automatic reasoning depth to request")
            }
            Self::BudgetUnsupported => f.write_str("this model takes no reasoning-token budget"),
            Self::BudgetOutOfBounds(bounds) if bounds.is_valid() => write!(
                f,
                "the reasoning-token budget is outside the accepted span {}..={}",
                bounds.min, bounds.max
            ),
            Self::BudgetOutOfBounds(_) => {
                f.write_str("the accepted reasoning-token budget span is invalid")
            }
        }
    }
}

impl std::error::Error for EffortRefusal {}

impl EffortSupport {
    /// The single admission rule for a requested effort (D-03, D-06, D-07).
    ///
    /// Pure and free of logging. Against a model whose support is:
    ///
    /// - `Unmeasured` or `WireUnsupported`: refuses everything.
    /// - `NoReasoning`: accepts only `Off`, as an exact no-op (`on_wire == false`).
    /// - `Supported`: refuses a `coerced` level whatever the fallback; accepts a listed level
    ///   exactly; accepts `Off` only when `off`, `Auto` only when `auto`; accepts `Budget(n)` only
    ///   inside the `budget` bounds, inclusive. A level the model lacks is refused under
    ///   `Refuse`, and under `NearestLower` / `NearestHigher` replaced by the closest listed
    ///   level strictly below / above by rank, or refused with
    ///   [`EffortRefusal::NoLevelInDirection`] when that direction holds none. The direction is
    ///   never flipped, and `Off`, `Auto`, a `Budget` and a coerced level are never substituted.
    ///
    /// # Errors
    ///
    /// Returns the [`EffortRefusal`] naming why the effort was not admitted.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_ports::output::model_profile::{EffortLevels, EffortRefusal, EffortSupport};
    /// use paladin_ports::output::llm_port::{ReasoningEffort, ReasoningFallback};
    ///
    /// let support = EffortSupport::Supported(EffortLevels::new([
    ///     ReasoningEffort::Low,
    ///     ReasoningEffort::High,
    /// ]));
    /// assert_eq!(
    ///     support.resolve(ReasoningEffort::Medium, ReasoningFallback::Refuse),
    ///     Err(EffortRefusal::LevelUnsupported)
    /// );
    /// ```
    pub fn resolve(
        &self,
        requested: ReasoningEffort,
        fallback: ReasoningFallback,
    ) -> Result<ResolvedEffort, EffortRefusal> {
        match self {
            Self::Unmeasured => Err(EffortRefusal::Unmeasured),
            Self::WireUnsupported => Err(EffortRefusal::WireUnsupported),
            Self::NoReasoning => {
                if requested == ReasoningEffort::Off {
                    Ok(ResolvedEffort::exact(requested, false))
                } else {
                    Err(EffortRefusal::NoReasoning)
                }
            }
            Self::Supported(levels) => levels.admit(requested, fallback),
        }
    }

    /// The support acceptable to **both** `self` and `other` (D-09).
    ///
    /// AND semantics, consistent with `json_object` / `json_schema`: a level, `off`, `auto` or
    /// budget span is usable only when every operand supports it (budget spans intersect to
    /// their overlap, or to none), and a level coerced on either operand stays coerced and
    /// unlisted. Precedence between kinds is `WireUnsupported` over `Unmeasured` over
    /// `NoReasoning` over `Supported`. `NoReasoning` acts as "off only": intersected with a
    /// `Supported` row it stays `NoReasoning` when that row accepts `off`, otherwise nothing is
    /// usable (an empty `Supported`). Whenever an intersection leaves no level, no `auto` and no
    /// budget, the `coerced` and `enable_thinking_with_level` detail is dropped, since nothing
    /// is left to refuse by level; that normal form is what keeps the operation associative.
    /// The result is commutative and associative.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_ports::output::model_profile::{EffortLevels, EffortSupport};
    /// use paladin_ports::output::llm_port::ReasoningEffort::{High, Low, Max};
    ///
    /// let a = EffortSupport::Supported(EffortLevels::new([Low, High]));
    /// let b = EffortSupport::Supported(EffortLevels::new([High, Max]));
    /// assert_eq!(
    ///     a.intersect(&b),
    ///     EffortSupport::Supported(EffortLevels::new([High]))
    /// );
    /// assert_eq!(
    ///     EffortSupport::NoReasoning.intersect(&EffortSupport::WireUnsupported),
    ///     EffortSupport::WireUnsupported
    /// );
    /// ```
    #[must_use]
    pub fn intersect(&self, other: &Self) -> Self {
        match (self, other) {
            (Self::WireUnsupported, _) | (_, Self::WireUnsupported) => Self::WireUnsupported,
            (Self::Unmeasured, _) | (_, Self::Unmeasured) => Self::Unmeasured,
            (a, b) => match (a.as_levels(), b.as_levels()) {
                (Some(a), Some(b)) => Self::from_and(a.and(&b)),
                // A future support kind this rule does not know: fail closed.
                _ => Self::Unmeasured,
            },
        }
    }

    /// The `EffortLevels` view used by `intersect`: `NoReasoning` is "off only".
    fn as_levels(&self) -> Option<std::borrow::Cow<'_, EffortLevels>> {
        match self {
            Self::NoReasoning => Some(std::borrow::Cow::Owned(
                EffortLevels::new([]).with_off(true),
            )),
            Self::Supported(levels) => Some(std::borrow::Cow::Borrowed(levels)),
            _ => None,
        }
    }

    /// Normal form of an intersection result: nothing but possibly `off` left collapses to
    /// `NoReasoning` (when `off`) or an empty `Supported` (when not).
    fn from_and(result: EffortLevels) -> Self {
        if result.levels.is_empty() && !result.auto && result.budget.is_none() {
            if result.off {
                Self::NoReasoning
            } else {
                Self::Supported(EffortLevels::new([]))
            }
        } else {
            Self::Supported(result)
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
    /// Whether the default call to this model **returned reasoning text**. It is a measurement
    /// about the model's output, not a statement of what effort the model accepts, and effort
    /// admission never consults it: [`effort`](Self::effort) decides that. It still drives the
    /// reasoning-model timeout default and is OR-ed when profiles are intersected.
    pub reasoning: bool,
    /// Whether the model supports a JSON-object `response_format`.
    pub json_object: bool,
    /// Whether the model supports a JSON-schema `response_format`.
    pub json_schema: bool,
    /// The date (`YYYY-MM-DD`) this entry was last checked against the vendor, when known.
    pub verified_on: Option<String>,
    /// Which reasoning efforts the model takes (Phase 51). Fail closed:
    /// [`EffortSupport::Unmeasured`] until a probe-backed table says otherwise.
    pub effort: EffortSupport,
    /// The date (`YYYY-MM-DD`) of the effort probe the [`effort`](Self::effort) data came from.
    pub effort_verified_on: Option<String>,
}

impl ModelProfile {
    /// A profile that states nothing as sendable (fail closed).
    ///
    /// Every sampling parameter and stop sequences are `NotSent`, both windows are `None`,
    /// `reasoning`, `json_object` and `json_schema` are `false`, `verified_on` is `None`, and
    /// `effort` is [`EffortSupport::Unmeasured`] with `effort_verified_on` `None`. Opt each
    /// capability in with a `with_*` setter.
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
            effort: EffortSupport::Unmeasured,
            effort_verified_on: None,
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

    /// Record that the default call to this model returned reasoning text. This is a measurement
    /// about the output; it does not state which efforts the model accepts (use
    /// [`with_effort`](Self::with_effort)).
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

    /// Set which reasoning efforts the model takes.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_ports::output::model_profile::{EffortSupport, ModelProfile};
    ///
    /// let profile = ModelProfile::new().with_effort(EffortSupport::NoReasoning);
    /// assert_eq!(profile.effort, EffortSupport::NoReasoning);
    /// ```
    #[must_use]
    pub fn with_effort(mut self, effort: EffortSupport) -> Self {
        self.effort = effort;
        self
    }

    /// Record the date (`YYYY-MM-DD`) of the effort probe the effort data came from.
    #[must_use]
    pub fn with_effort_verified_on(mut self, date: impl Into<String>) -> Self {
        self.effort_verified_on = Some(date.into());
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
    /// supports it); `effort` is intersected with [`EffortSupport::intersect`]; `verified_on` and
    /// `effort_verified_on` are cleared because a combination was never itself verified.
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
            effort: self.effort.intersect(&other.effort),
            effort_verified_on: None,
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
        assert_eq!(p.effort, EffortSupport::Unmeasured);
        assert_eq!(p.effort_verified_on, None);
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
            (
                "effort",
                base.clone().with_effort(EffortSupport::NoReasoning),
            ),
            (
                "effort_verified_on",
                base.clone().with_effort_verified_on("2026-10-21"),
            ),
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
                "effort" => restored.effort = base.effort.clone(),
                "effort_verified_on" => {
                    restored.effort_verified_on = base.effort_verified_on.clone();
                }
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

    fn levels(list: &[ReasoningEffort]) -> EffortSupport {
        EffortSupport::Supported(EffortLevels::new(list.iter().copied()))
    }

    #[test]
    fn unmeasured_profile_refuses_every_effort() {
        let profile = ModelProfile::new();
        assert_eq!(profile.effort, EffortSupport::Unmeasured);
        let all = [
            ReasoningEffort::Off,
            ReasoningEffort::Auto,
            ReasoningEffort::Minimal,
            ReasoningEffort::Low,
            ReasoningEffort::Medium,
            ReasoningEffort::High,
            ReasoningEffort::XHigh,
            ReasoningEffort::Max,
            ReasoningEffort::Budget(2048),
        ];
        for effort in all {
            assert_eq!(
                profile.effort.resolve(effort, ReasoningFallback::Refuse),
                Err(EffortRefusal::Unmeasured),
                "Unmeasured admitted {effort}"
            );
        }
    }

    #[test]
    fn listed_level_resolves_exactly() {
        use ReasoningEffort::{High, Low};
        let support = levels(&[Low, High]);
        let resolved = support
            .resolve(High, ReasoningFallback::Refuse)
            .expect("High is listed");
        assert_eq!(resolved.requested, High);
        assert_eq!(resolved.effective, High);
        assert!(resolved.on_wire);
        assert!(!resolved.substituted());
    }

    #[test]
    fn no_reasoning_accepts_only_off_as_a_no_op() {
        let resolved = EffortSupport::NoReasoning
            .resolve(ReasoningEffort::Off, ReasoningFallback::Refuse)
            .expect("Off is an exact no-op");
        assert!(!resolved.on_wire);
        assert!(!resolved.substituted());
        assert_eq!(
            EffortSupport::NoReasoning.resolve(ReasoningEffort::Low, ReasoningFallback::Refuse),
            Err(EffortRefusal::NoReasoning)
        );
        assert_eq!(
            EffortSupport::WireUnsupported.resolve(ReasoningEffort::Off, ReasoningFallback::Refuse),
            Err(EffortRefusal::WireUnsupported)
        );
    }

    #[test]
    fn effort_refusal_display_is_plain_english() {
        assert!(
            EffortRefusal::BudgetOutOfBounds(BudgetBounds::new(1024, 32_000))
                .to_string()
                .contains("1024..=32000")
        );
        for refusal in [
            EffortRefusal::Unmeasured,
            EffortRefusal::WireUnsupported,
            EffortRefusal::NoReasoning,
            EffortRefusal::LevelUnsupported,
            EffortRefusal::LevelCoerced,
            EffortRefusal::NoLevelInDirection,
            EffortRefusal::OffUnsupported,
            EffortRefusal::AutoUnsupported,
            EffortRefusal::BudgetUnsupported,
            EffortRefusal::BudgetOutOfBounds(BudgetBounds::new(0, 0)),
        ] {
            assert!(!refusal.to_string().is_empty());
        }
    }

    const THREE_FALLBACKS: [ReasoningFallback; 3] = [
        ReasoningFallback::Refuse,
        ReasoningFallback::NearestLower,
        ReasoningFallback::NearestHigher,
    ];

    /// Levels {Low, High}, off accepted, auto not, no budget, Medium coerced.
    fn low_high_with_coerced_medium() -> EffortSupport {
        EffortSupport::Supported(
            EffortLevels::new([ReasoningEffort::Low, ReasoningEffort::High])
                .with_off(true)
                .with_coerced([ReasoningEffort::Medium]),
        )
    }

    /// D-03 / D-06 / D-07 admission table over one `Supported` row.
    #[test]
    fn admission_table_for_a_supported_row() {
        use ReasoningEffort::{Auto, Budget, High, Low, Max, Medium, Minimal, Off, XHigh};
        use ReasoningFallback::{NearestHigher, NearestLower, Refuse};
        let support = low_high_with_coerced_medium();

        // Off accepted (on the wire) under every fallback.
        for fb in THREE_FALLBACKS {
            let off = support.resolve(Off, fb).expect("off is accepted");
            assert!(off.on_wire && !off.substituted());
            assert_eq!(
                support.resolve(Auto, fb),
                Err(EffortRefusal::AutoUnsupported)
            );
            // A coerced level is refused whatever the fallback.
            assert_eq!(
                support.resolve(Medium, fb),
                Err(EffortRefusal::LevelCoerced)
            );
            // A budget on a row without one is never converted to a level.
            assert_eq!(
                support.resolve(Budget(2048), fb),
                Err(EffortRefusal::BudgetUnsupported)
            );
        }

        // Missing level below the list.
        assert_eq!(
            support.resolve(Minimal, NearestLower),
            Err(EffortRefusal::NoLevelInDirection)
        );
        assert_eq!(
            support.resolve(Minimal, NearestHigher).map(|r| r.effective),
            Ok(Low)
        );
        assert_eq!(
            support.resolve(Minimal, Refuse),
            Err(EffortRefusal::LevelUnsupported)
        );
        // Missing level above the list.
        assert_eq!(
            support.resolve(XHigh, NearestLower).map(|r| r.effective),
            Ok(High)
        );
        assert_eq!(
            support.resolve(Max, NearestHigher),
            Err(EffortRefusal::NoLevelInDirection)
        );
        assert_eq!(
            support.resolve(Max, Refuse),
            Err(EffortRefusal::LevelUnsupported)
        );
    }

    /// Edge REASON-02/adjacency: with {Low, High}, Medium picks by the caller's direction.
    #[test]
    fn substitution_follows_the_chosen_direction_and_keeps_both_values() {
        use ReasoningEffort::{High, Low, Medium};
        let support = levels(&[Low, High]);
        let lower = support
            .resolve(Medium, ReasoningFallback::NearestLower)
            .expect("Low is below Medium");
        assert_eq!((lower.requested, lower.effective), (Medium, Low));
        assert!(lower.substituted() && lower.on_wire);
        let higher = support
            .resolve(Medium, ReasoningFallback::NearestHigher)
            .expect("High is above Medium");
        assert_eq!((higher.requested, higher.effective), (Medium, High));
        assert!(higher.substituted() && higher.on_wire);
        assert_eq!(
            support.resolve(Medium, ReasoningFallback::Refuse),
            Err(EffortRefusal::LevelUnsupported)
        );
    }

    /// D-07: `Off` is never substituted by a level, nor a level by `Off`.
    #[test]
    fn off_is_never_substituted() {
        let no_off = levels(&[ReasoningEffort::Low, ReasoningEffort::High]);
        for fb in [
            ReasoningFallback::NearestLower,
            ReasoningFallback::NearestHigher,
        ] {
            assert_eq!(
                no_off.resolve(ReasoningEffort::Off, fb),
                Err(EffortRefusal::OffUnsupported)
            );
            assert_eq!(
                no_off.resolve(ReasoningEffort::Auto, fb),
                Err(EffortRefusal::AutoUnsupported)
            );
        }
    }

    /// A coerced level is never a substitution target even when it is the nearest.
    #[test]
    fn a_coerced_level_is_never_a_substitution_target() {
        use ReasoningEffort::{High, Low, Max, Medium};
        let support =
            EffortSupport::Supported(EffortLevels::new([Low, Medium, High]).with_coerced([Medium]));
        // Medium was removed from `levels` by with_coerced, so Low is the nearest lower to High+1.
        let r = support
            .resolve(Max, ReasoningFallback::NearestLower)
            .expect("High is listed");
        assert_eq!(r.effective, High);
        let r = support
            .resolve(ReasoningEffort::Minimal, ReasoningFallback::NearestHigher)
            .expect("Low is listed");
        assert_eq!(r.effective, Low);
    }

    /// Edge REASON-06: inclusive bounds, off-by-one refusals, never converted to a level.
    #[test]
    fn budget_table_is_inclusive_and_never_substituted() {
        use ReasoningEffort::Budget;
        let support = EffortSupport::Supported(
            EffortLevels::new([ReasoningEffort::High]).with_budget(BudgetBounds::new(1024, 32_000)),
        );
        let bounds = BudgetBounds::new(1024, 32_000);
        for fb in THREE_FALLBACKS {
            for n in [1024_u32, 2048, 32_000] {
                let r = support.resolve(Budget(n), fb).expect("inside the span");
                assert_eq!(r.effective, Budget(n));
                assert!(r.on_wire && !r.substituted());
            }
            for n in [0_u32, 1023, 32_001, u32::MAX] {
                assert_eq!(
                    support.resolve(Budget(n), fb),
                    Err(EffortRefusal::BudgetOutOfBounds(bounds)),
                    "Budget({n}) under {fb}"
                );
            }
        }
    }

    /// Edge REASON-06/empty: an invalid span accepts no budget at all.
    #[test]
    fn an_invalid_budget_span_fails_closed() {
        for bounds in [BudgetBounds::new(0, 10), BudgetBounds::new(10, 5)] {
            assert!(!bounds.is_valid());
            let support = EffortSupport::Supported(EffortLevels::new([]).with_budget(bounds));
            for n in [1_u32, 5, 10, u32::MAX] {
                assert_eq!(
                    support.resolve(ReasoningEffort::Budget(n), ReasoningFallback::Refuse),
                    Err(EffortRefusal::BudgetOutOfBounds(bounds))
                );
            }
            assert!(support_levels(&support).validate().is_err());
        }
        assert!(BudgetBounds::new(1, 1).is_valid());
    }

    fn support_levels(support: &EffortSupport) -> &EffortLevels {
        match support {
            EffortSupport::Supported(levels) => levels,
            other => panic!("expected Supported, got {other:?}"),
        }
    }

    /// Edge REASON-01/empty: a budget-only model refuses every level whatever the fallback.
    #[test]
    fn empty_levels_validate_and_refuse_every_level() {
        let empty = EffortLevels::new([]);
        assert!(empty.validate().is_ok());
        let budget_only = EffortSupport::Supported(
            EffortLevels::new([])
                .with_off(true)
                .with_auto(true)
                .with_budget(BudgetBounds::new(1, 100)),
        );
        for fb in THREE_FALLBACKS {
            for level in ReasoningEffort::LEVELS {
                let refusal = budget_only.resolve(level, fb);
                if fb == ReasoningFallback::Refuse {
                    assert_eq!(refusal, Err(EffortRefusal::LevelUnsupported));
                } else {
                    assert_eq!(refusal, Err(EffortRefusal::NoLevelInDirection));
                }
            }
            assert!(budget_only.resolve(ReasoningEffort::Off, fb).is_ok());
            assert!(budget_only.resolve(ReasoningEffort::Auto, fb).is_ok());
            assert!(
                budget_only
                    .resolve(ReasoningEffort::Budget(100), fb)
                    .is_ok()
            );
        }
    }

    /// Edge REASON-01/ordering: construction normalises, validation rejects bad data.
    #[test]
    fn effort_levels_normalise_and_validate() {
        use ReasoningEffort::{Auto, Budget, High, Low, Off};
        assert_eq!(
            EffortLevels::new([High, Low, Low, Off, Auto, Budget(1)]).levels,
            [Low, High]
        );
        let coerced = EffortLevels::new([Low, High]).with_coerced([Low]);
        assert_eq!(coerced.levels, [High]);
        assert_eq!(coerced.coerced, [Low]);
        assert!(coerced.validate().is_ok());

        for bad in [
            r#"{"levels":["high","low"]}"#,
            r#"{"levels":["low","low"]}"#,
            r#"{"levels":["off"]}"#,
            r#"{"levels":["auto"]}"#,
            r#"{"levels":[{"budget":5}]}"#,
            r#"{"levels":["low"],"budget":{"min":0,"max":10}}"#,
            r#"{"levels":["low"],"budget":{"min":9,"max":3}}"#,
            r#"{"levels":["low"],"coerced":["low"]}"#,
            r#"{"levels":[],"coerced":["high","low"]}"#,
        ] {
            let parsed: EffortLevels = serde_json::from_str(bad).expect("shape parses");
            assert!(parsed.validate().is_err(), "{bad} validated");
        }
        let good: EffortLevels = serde_json::from_str(r#"{"levels":["low","high"]}"#).unwrap();
        assert!(good.validate().is_ok());
        assert!(serde_json::from_str::<EffortLevels>(r#"{"levels":[],"surprise":1}"#).is_err());
    }

    /// Edge REASON-01/precision: a budget survives resolve with no rounding.
    #[test]
    fn budget_precision_survives_resolve() {
        for n in [1_u32, 1024, 32_768, u32::MAX] {
            let support = EffortSupport::Supported(
                EffortLevels::new([]).with_budget(BudgetBounds::new(1, u32::MAX)),
            );
            let r = support
                .resolve(ReasoningEffort::Budget(n), ReasoningFallback::Refuse)
                .expect("inside the span");
            assert_eq!(r.effective, ReasoningEffort::Budget(n));
        }
    }

    fn intersect_operands() -> Vec<EffortSupport> {
        use ReasoningEffort::{High, Low, Max};
        vec![
            EffortSupport::Unmeasured,
            EffortSupport::WireUnsupported,
            EffortSupport::NoReasoning,
            EffortSupport::Supported(
                EffortLevels::new([Low, High])
                    .with_off(true)
                    .with_budget(BudgetBounds::new(1024, 32_000))
                    .with_coerced([ReasoningEffort::Medium]),
            ),
            EffortSupport::Supported(
                EffortLevels::new([High, Max])
                    .with_auto(true)
                    .with_budget(BudgetBounds::new(2048, 64_000))
                    .with_enable_thinking_with_level(true),
            ),
            EffortSupport::Supported(EffortLevels::new([])),
            EffortSupport::Supported(EffortLevels::new([]).with_off(true)),
            EffortSupport::Supported(EffortLevels::new([]).with_budget(BudgetBounds::new(1, 10))),
            EffortSupport::Supported(
                EffortLevels::new([Low, High, Max])
                    .with_coerced([High])
                    .with_off(true),
            ),
            EffortSupport::Supported(EffortLevels::new([Low]).with_budget(BudgetBounds::new(0, 5))),
        ]
    }

    /// D-09: the effort intersection is commutative and associative.
    #[test]
    fn effort_intersect_is_commutative_and_associative() {
        let table = intersect_operands();
        for a in &table {
            for b in &table {
                assert_eq!(
                    a.intersect(b),
                    b.intersect(a),
                    "not commutative for {a:?}, {b:?}"
                );
                for c in &table {
                    assert_eq!(
                        a.intersect(b).intersect(c),
                        a.intersect(&b.intersect(c)),
                        "not associative for {a:?}, {b:?}, {c:?}"
                    );
                }
            }
        }
    }

    /// D-09: AND semantics for levels, off, auto, budget; union for coerced.
    #[test]
    fn effort_intersect_follows_the_documented_cases() {
        use ReasoningEffort::{High, Low, Max};
        let ops = intersect_operands();
        let (a, b) = (&ops[3], &ops[4]);
        let both = a.intersect(b);
        let l = support_levels(&both);
        assert_eq!(l.levels, [High]);
        assert!(!l.off && !l.auto);
        assert_eq!(l.budget, Some(BudgetBounds::new(2048, 32_000)));
        assert_eq!(l.coerced, [ReasoningEffort::Medium]);
        assert!(l.enable_thinking_with_level);

        // {Low, High} and {High, Max} (no budgets) list exactly High.
        let two = levels(&[Low, High]).intersect(&levels(&[High, Max]));
        assert_eq!(support_levels(&two).levels, [High]);

        // Disjoint budgets give none; a coerced level on either side stays coerced.
        let x =
            EffortSupport::Supported(EffortLevels::new([]).with_budget(BudgetBounds::new(1, 10)));
        let y =
            EffortSupport::Supported(EffortLevels::new([]).with_budget(BudgetBounds::new(20, 30)));
        assert_eq!(support_levels(&x.intersect(&y)).budget, None);
        let listed = levels(&[Low, High]);
        let coerced = EffortSupport::Supported(EffortLevels::new([Low, High]).with_coerced([High]));
        let narrowed = listed.intersect(&coerced);
        assert_eq!(support_levels(&narrowed).levels, [Low]);
        assert_eq!(support_levels(&narrowed).coerced, [High]);
        assert_eq!(
            narrowed.resolve(High, ReasoningFallback::NearestLower),
            Err(EffortRefusal::LevelCoerced)
        );

        // Absorbing elements: WireUnsupported over Unmeasured over NoReasoning over Supported.
        for other in &ops {
            assert_eq!(
                EffortSupport::WireUnsupported.intersect(other),
                EffortSupport::WireUnsupported
            );
            if *other != EffortSupport::WireUnsupported {
                assert_eq!(
                    EffortSupport::Unmeasured.intersect(other),
                    EffortSupport::Unmeasured
                );
            }
        }
        assert_eq!(
            EffortSupport::NoReasoning.intersect(&EffortSupport::NoReasoning),
            EffortSupport::NoReasoning
        );
        // NoReasoning with a Supported row: NoReasoning when it accepts Off, else nothing usable.
        let accepts_off = EffortSupport::Supported(EffortLevels::new([Low]).with_off(true));
        assert_eq!(
            EffortSupport::NoReasoning.intersect(&accepts_off),
            EffortSupport::NoReasoning
        );
        let no_off = levels(&[Low]);
        let nothing = EffortSupport::NoReasoning.intersect(&no_off);
        assert_eq!(nothing, EffortSupport::Supported(EffortLevels::new([])));
        for effort in [ReasoningEffort::Off, ReasoningEffort::Auto, Low] {
            assert!(nothing.resolve(effort, ReasoningFallback::Refuse).is_err());
        }
    }

    /// The whole `ModelProfile` intersection stays commutative with effort data in play.
    #[test]
    fn model_profile_intersect_covers_effort() {
        let a = ModelProfile::new()
            .with_effort(low_high_with_coerced_medium())
            .with_effort_verified_on("2026-10-21");
        let b =
            ModelProfile::new().with_effort(levels(&[ReasoningEffort::High, ReasoningEffort::Max]));
        let both = a.intersect(&b);
        assert_eq!(support_levels(&both.effort).levels, [ReasoningEffort::High]);
        assert_eq!(both.effort_verified_on, None);
        assert_eq!(both, b.intersect(&a));
    }

    /// A3-51-01 neighbour: the support enum serialises with snake_case tags.
    #[test]
    fn effort_support_serde_round_trips() {
        for support in intersect_operands() {
            let json = serde_json::to_string(&support).expect("serialise");
            let back: EffortSupport = serde_json::from_str(&json).expect("deserialise");
            assert_eq!(back, support);
        }
        assert_eq!(
            serde_json::to_string(&EffortSupport::Unmeasured).unwrap(),
            "\"unmeasured\""
        );
    }
}
