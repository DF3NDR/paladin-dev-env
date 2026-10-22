//! Reasoning effort at the wire: exact or refused, never guessed (Phase 51, D-00f).
//!
//! # The policy in one paragraph
//!
//! A request that sets [`LlmRequest::reasoning`](paladin_ports::output::llm_port::LlmRequest)
//! asks for a specific depth of thinking. An adapter either puts **exactly** that effort on the
//! wire or refuses the request with
//! [`LlmError::UnsupportedParameter`](paladin_ports::output::llm_port::LlmError) before any HTTP
//! call. It never drops the effort (the sampling module's omit-and-log path is deliberately not
//! used here) and it never swaps it for another effort unless the caller chose a substituting
//! [`ReasoningFallback`]. A request that sets
//! no effort is untouched: nothing is sent, the profile is not consulted and no metadata is
//! stamped, so its wire body stays byte-identical to what it was before this feature existed
//! (D-00i).
//!
//! # The admission rule
//!
//! The decision is one pure function on the model's profile,
//! [`EffortSupport::resolve`](paladin_ports::output::model_profile::EffortSupport::resolve). The
//! crate-private `admit` in this module wraps it for the adapters: it turns a refusal into an
//! error that names the provider, the model, the effort and the configuration key that unlocks
//! the model, and it records a substitution. Because the wire is folded into the profile an
//! adapter returns from `model_profile` (see `mask_effort`), `PaladinBuilder` and the fallback
//! chain judge exactly what the wire can send.
//!
//! # Visibility rule
//!
//! When an effort was set, every buffered response and the terminal chunk of a stream carries
//! [`REASONING_REQUESTED_METADATA_KEY`] (the effort asked for) and
//! [`REASONING_EFFECTIVE_METADATA_KEY`] (the effort the adapter acted on). They are equal unless a
//! substituting fallback was chosen, in which case the first substitution per adapter instance,
//! model and pair of values also emits one `warn!` (target `paladin::reasoning`). Both values are
//! the `Display` spellings of
//! [`ReasoningEffort`], plain ASCII.
//!
//! # The wire declaration
//!
//! [`ReasoningWire`] names where an adapter puts an effort in the request body. A native adapter
//! fixes its form in code; the self-hosted compatible adapters let the operator choose it.
//!
//! Text in errors and logs names the provider, the model and the effort values only -- never a
//! prompt, a key or a response body.
// The helpers below are consumed by the nine adapters; without a consuming adapter feature they
// are legitimately unused.
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

use std::collections::HashSet;
use std::fmt;
use std::sync::Mutex;

use paladin_ports::output::llm_port::{LlmError, ReasoningEffort, ReasoningFallback};
use paladin_ports::output::model_profile::{
    EffortRefusal, EffortSupport, ModelProfile, ResolvedEffort,
};
use serde::{Deserialize, Serialize};

pub use paladin_core::platform::container::reasoning::{
    REASONING_EFFECTIVE_METADATA_KEY, REASONING_REQUESTED_METADATA_KEY,
};

/// Where an adapter puts a reasoning effort in the request body.
///
/// The YAML and JSON spellings are `unsupported`, `top_level_effort`, `thinking_object` and
/// `enable_thinking`. A native adapter fixes its form in code; the self-hosted compatible
/// adapters take the operator's choice from `llm.<provider>.reasoning_wire`.
///
/// Marked `#[non_exhaustive]`: match it with a wildcard arm.
///
/// # Examples
///
/// ```
/// use paladin_llm::reasoning::ReasoningWire;
///
/// assert_eq!(ReasoningWire::TopLevelEffort.to_string(), "top_level_effort");
/// let parsed: ReasoningWire = serde_json::from_str("\"thinking_object\"")?;
/// assert_eq!(parsed, ReasoningWire::ThinkingObject);
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ReasoningWire {
    /// No wire form: every explicit effort is refused, `off` included.
    Unsupported,
    /// A top-level `reasoning_effort` field in the request body.
    TopLevelEffort,
    /// A `thinking` object carrying the type and the effort: `thinking: {type, effort}`.
    ThinkingObject,
    /// The `enable_thinking` switch with a `thinking_budget` token budget.
    EnableThinking,
}

impl fmt::Display for ReasoningWire {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Unsupported => "unsupported",
            Self::TopLevelEffort => "top_level_effort",
            Self::ThinkingObject => "thinking_object",
            Self::EnableThinking => "enable_thinking",
        })
    }
}

/// What one adapter's request body can carry of the profile's effort data.
///
/// Each flag says whether the adapter has a wire form for that kind of effort. `NONE` carries
/// nothing, so the profile an adapter returns for a wire with no effort form refuses everything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WireCaps {
    /// The wire carries one of the six levels.
    pub levels: bool,
    /// The wire can switch reasoning off.
    pub off: bool,
    /// The wire can ask for the vendor's own default depth.
    pub auto: bool,
    /// The wire carries a reasoning-token budget.
    pub budget: bool,
}

impl WireCaps {
    /// A wire with no effort form at all (D-04): every explicit effort is refused.
    pub(crate) const NONE: Self = Self {
        levels: false,
        off: false,
        auto: false,
        budget: false,
    };
}

/// Remove from `support` whatever `caps` says the adapter's wire cannot carry, so the profile an
/// adapter reports promises exactly what it can send.
///
/// `WireCaps::NONE` gives [`EffortSupport::WireUnsupported`] whatever the data was. Otherwise a
/// `Supported` entry loses its levels (and its coerced list, which only matters beside levels)
/// when the wire has no level form, `off` when it cannot switch reasoning off, `auto` when it
/// cannot ask for the default depth, and its budget span when it carries no budget.
/// `Unmeasured` and `NoReasoning` pass through unchanged.
pub(crate) fn mask_effort(support: EffortSupport, caps: WireCaps) -> EffortSupport {
    if caps == WireCaps::NONE {
        return EffortSupport::WireUnsupported;
    }
    match support {
        EffortSupport::Supported(mut levels) => {
            if !caps.levels {
                levels.levels.clear();
                levels.coerced.clear();
            }
            if !caps.off {
                levels.off = false;
            }
            if !caps.auto {
                levels.auto = false;
            }
            if !caps.budget {
                levels.budget = None;
            }
            EffortSupport::Supported(levels)
        }
        other => other,
    }
}

/// The most distinct (model, requested, effective) substitutions one adapter instance remembers.
/// Past it a substitution warns every time instead of being remembered: memory stays bounded and
/// the signal is never lost.
const SUBSTITUTION_LOG_CAPACITY: usize = 256;

/// Once-per-instance record of which (model, requested, effective) substitutions have already
/// been logged. The reasoning twin of the sampling module's `OmissionLog`.
///
/// One lives on each adapter instance, so a deployment with two instances warns once per
/// instance and a hot loop of identical requests warns once in total.
#[derive(Debug, Default)]
pub(crate) struct SubstitutionLog {
    warned: Mutex<HashSet<(String, ReasoningEffort, ReasoningEffort)>>,
}

impl SubstitutionLog {
    /// An empty log.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Record that `effective` was sent where `requested` was asked for, for `provider` and
    /// `model`.
    ///
    /// Emits one `log::warn!` (target `paladin::reasoning`) the first time this instance sees the
    /// triple and returns whether it warned. The text names provider, model and the two effort
    /// values only.
    pub(crate) fn record(
        &self,
        provider: &str,
        model: &str,
        requested: ReasoningEffort,
        effective: ReasoningEffort,
    ) -> bool {
        // A poisoned lock only means another thread panicked while holding it; the set it guards
        // is still a valid set of "already warned" keys, so recover it rather than panic.
        let mut warned = self
            .warned
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        let key = (model.to_string(), requested, effective);
        if warned.contains(&key) {
            return false;
        }
        if warned.len() < SUBSTITUTION_LOG_CAPACITY {
            warned.insert(key);
        }
        log::warn!(
            target: "paladin::reasoning",
            "provider '{provider}' model '{model}': reasoning effort '{requested}' is not offered \
             by this model; '{effective}' was sent instead because the request chose a \
             substituting fallback (D-06). Both values are recorded in the response metadata \
             under '{}' and '{}'",
            REASONING_REQUESTED_METADATA_KEY,
            REASONING_EFFECTIVE_METADATA_KEY,
        );
        true
    }
}

/// The wire spelling of one of the six levels, or `None` for `Off`, `Auto` and a budget.
///
/// Every vendor that takes a level spells it this way (`minimal` ... `max`); the lower-case
/// `Display` text of [`ReasoningEffort`] is the same string, kept as a `&'static str` so a
/// request body can hold it without allocating.
pub(crate) fn level_spelling(effort: ReasoningEffort) -> Option<&'static str> {
    match effort {
        ReasoningEffort::Minimal => Some("minimal"),
        ReasoningEffort::Low => Some("low"),
        ReasoningEffort::Medium => Some("medium"),
        ReasoningEffort::High => Some("high"),
        ReasoningEffort::XHigh => Some("xhigh"),
        ReasoningEffort::Max => Some("max"),
        _ => None,
    }
}

/// The providers whose wire form the operator can choose with `llm.<provider>.reasoning_wire`.
const SELF_HOSTED_PROVIDERS: [&str; 2] = ["ollama", "openai-compatible"];

/// One line naming what a profile's effort data admits, for a refusal message.
fn describe_admitted(support: &EffortSupport) -> String {
    match support {
        EffortSupport::NoReasoning => "only off (the model has no reasoning to adjust)".to_string(),
        EffortSupport::Supported(levels) => {
            let mut parts = Vec::new();
            if levels.levels.is_empty() {
                parts.push("no levels".to_string());
            } else {
                let listed: Vec<String> = levels.levels.iter().map(ToString::to_string).collect();
                parts.push(format!("levels [{}]", listed.join(", ")));
            }
            if levels.off {
                parts.push("off".to_string());
            }
            if levels.auto {
                parts.push("auto".to_string());
            }
            if let Some(bounds) = levels.budget {
                parts.push(format!("budgets {}..={}", bounds.min, bounds.max));
            }
            if !levels.coerced.is_empty() {
                let coerced: Vec<String> = levels.coerced.iter().map(ToString::to_string).collect();
                parts.push(format!(
                    "(the vendor silently runs [{}] as another level, so they are refused)",
                    coerced.join(", ")
                ));
            }
            parts.join(", ")
        }
        // `Unmeasured` and `WireUnsupported` are explained by their own reason; a future support
        // kind this message does not know says nothing rather than something wrong.
        _ => "nothing".to_string(),
    }
}

/// The refusal text: the rule's own sentence plus the way to unlock the model (D-03, D-04).
fn refusal_reason(
    provider: &str,
    model: &str,
    refusal: EffortRefusal,
    support: &EffortSupport,
) -> String {
    match refusal {
        EffortRefusal::Unmeasured => format!(
            "{refusal}; declare the model's efforts with the `llm.{provider}.models.{model}.effort` \
             override (see the Reasoning effort guide)"
        ),
        EffortRefusal::WireUnsupported if SELF_HOSTED_PROVIDERS.contains(&provider) => format!(
            "{refusal}; set `llm.{provider}.reasoning_wire` to the wire form this endpoint accepts \
             (see the Reasoning effort guide)"
        ),
        EffortRefusal::WireUnsupported => format!(
            "{refusal}; reasoning effort is not verified for this provider in this release; no \
             configuration unlocks it"
        ),
        other => format!(
            "{other}; this model accepts: {}",
            describe_admitted(support)
        ),
    }
}

/// The effort and fallback in force for one request: the request's own value, else the provider
/// level default from `llm.<provider>.reasoning_effort` / `reasoning_fallback` (Phase 51, D-00e).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InForce {
    /// The effort to admit; `None` sends nothing.
    pub(crate) effort: Option<ReasoningEffort>,
    /// The fallback to admit it under; `None` means refuse.
    pub(crate) fallback: Option<ReasoningFallback>,
    /// `llm.<provider>.reasoning_effort` when `effort` came from the provider default (the
    /// request set none), so a refusal can name the line to change; `None` when the request
    /// set the effort itself.
    pub(crate) source_key: Option<String>,
}

/// Resolve the effort and fallback in force: a request-level value always wins, field by field,
/// and with neither set nothing changes (D-00e, D-00i).
///
/// Called per request, never at construction, so the provider default is judged against the
/// request's own model (research Pitfall 6).
pub(crate) fn in_force(
    provider: &str,
    request_effort: Option<ReasoningEffort>,
    request_fallback: Option<ReasoningFallback>,
    default_effort: Option<ReasoningEffort>,
    default_fallback: Option<ReasoningFallback>,
) -> InForce {
    let from_default = request_effort.is_none() && default_effort.is_some();
    InForce {
        effort: request_effort.or(default_effort),
        fallback: request_fallback.or(default_fallback),
        source_key: from_default.then(|| format!("llm.{provider}.reasoning_effort")),
    }
}

/// Decide whether `requested` may reach the wire for `model`, before any HTTP call.
///
/// - `source_key`: when the effort in force came from a provider-level default rather than the
///   request, the configuration key that set it (`llm.<provider>.reasoning_effort`); a refusal
///   then ends with "(set by `<key>`)" so an operator knows which line to change.
/// - `None` requested: `Ok(None)`. Nothing is sent, the profile is not read and no metadata is
///   stamped (D-00i).
/// - Otherwise the profile's effort data decides through
///   [`EffortSupport::resolve`](paladin_ports::output::model_profile::EffortSupport::resolve)
///   under `fallback` (`Refuse` when `None`). A refusal becomes
///   [`LlmError::UnsupportedParameter`] naming `reasoning_effort`, the requested value, the
///   provider, the model and the unlock hint. A substitution is recorded in `log`.
///
/// `max_tokens` is the effective output cap of the request, for the budget-versus-output rule.
///
/// # Errors
///
/// [`LlmError::UnsupportedParameter`], which is `Permanent`: it is raised before the retry loop.
// Eight inputs, each independent and each read once: bundling them into a struct would only move the
// list to every call site.
#[allow(clippy::too_many_arguments)]
pub(crate) fn admit(
    provider: &str,
    model: &str,
    profile: &ModelProfile,
    requested: Option<ReasoningEffort>,
    fallback: Option<ReasoningFallback>,
    max_tokens: Option<u32>,
    source_key: Option<&str>,
    log: &SubstitutionLog,
) -> Result<Option<ResolvedEffort>, LlmError> {
    let Some(requested) = requested else {
        return Ok(None);
    };
    let refuse = |reason: String| {
        // The effort came from a provider-level default, not from the request: say which line
        // of configuration to change (research Pitfall 6).
        let reason = match source_key {
            Some(key) => format!("{reason} (set by `{key}`)"),
            None => reason,
        };
        LlmError::unsupported_parameter(
            "reasoning_effort",
            requested.to_string(),
            provider,
            model,
            reason,
        )
    };
    let resolved = profile
        .effort
        .resolve(requested, fallback.unwrap_or_default())
        .map_err(|refusal| refuse(refusal_reason(provider, model, refusal, &profile.effort)))?;
    // A budget is part of the output cap: a request whose cap is not above the budget leaves the
    // model no room to answer. Checked on the effective cap, after the profile accepted the
    // budget, so a profile refusal is reported first.
    if let (ReasoningEffort::Budget(budget), Some(cap)) = (resolved.effective, max_tokens)
        && budget >= cap
    {
        return Err(refuse(format!(
            "a reasoning budget must be below max_tokens ({cap}); the budget is {budget}. Raise \
             max_tokens above the budget or lower the budget"
        )));
    }
    if resolved.substituted() {
        log.record(provider, model, resolved.requested, resolved.effective);
    }
    Ok(Some(resolved))
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_ports::output::llm_port::{LlmError, ReasoningEffort, ReasoningFallback};
    use paladin_ports::output::model_profile::{
        BudgetBounds, EffortLevels, EffortSupport, ModelProfile,
    };

    fn profile(levels: EffortLevels) -> ModelProfile {
        ModelProfile::new().with_effort(EffortSupport::Supported(levels))
    }

    fn reason_of(error: LlmError) -> String {
        match error {
            LlmError::UnsupportedParameter { reason, .. } => reason,
            other => panic!("expected UnsupportedParameter, got {other:?}"),
        }
    }

    #[test]
    fn reasoning_wire_spellings_are_the_configuration_contract() {
        for (wire, spelling) in [
            (ReasoningWire::Unsupported, "unsupported"),
            (ReasoningWire::TopLevelEffort, "top_level_effort"),
            (ReasoningWire::ThinkingObject, "thinking_object"),
            (ReasoningWire::EnableThinking, "enable_thinking"),
        ] {
            assert_eq!(
                serde_json::to_string(&wire).unwrap(),
                format!("\"{spelling}\"")
            );
            assert_eq!(
                serde_json::from_str::<ReasoningWire>(&format!("\"{spelling}\"")).unwrap(),
                wire
            );
            assert_eq!(wire.to_string(), spelling);
        }
    }

    #[test]
    fn the_metadata_keys_are_re_exported_with_their_pinned_spelling() {
        assert_eq!(
            REASONING_REQUESTED_METADATA_KEY,
            "paladin.reasoning_requested"
        );
        assert_eq!(
            REASONING_EFFECTIVE_METADATA_KEY,
            "paladin.reasoning_effective"
        );
    }

    #[test]
    fn a_request_value_beats_the_provider_default_field_by_field() {
        let high = Some(ReasoningEffort::High);
        let low = Some(ReasoningEffort::Low);
        let lower = Some(ReasoningFallback::NearestLower);
        let higher = Some(ReasoningFallback::NearestHigher);
        // Request effort wins; no source key because the request named it.
        let won = in_force("deepseek", low, None, high, lower);
        assert_eq!(won.effort, low);
        assert_eq!(won.fallback, lower);
        assert_eq!(won.source_key, None);
        // Request fallback wins over the default fallback independently of the effort.
        assert_eq!(
            in_force("deepseek", None, higher, high, lower).fallback,
            higher
        );
        // Nothing set anywhere: nothing in force.
        let none = in_force("deepseek", None, None, None, None);
        assert_eq!(
            (none.effort, none.fallback, none.source_key),
            (None, None, None)
        );
        // The provider default fills an unset request and names its key.
        let defaulted = in_force("kimi", None, None, high, None);
        assert_eq!(defaulted.effort, high);
        assert_eq!(
            defaulted.source_key.as_deref(),
            Some("llm.kimi.reasoning_effort")
        );
        // A default fallback alone does not make the effort come from configuration.
        assert_eq!(in_force("kimi", None, None, None, lower).source_key, None);
    }

    #[test]
    fn a_refusal_of_a_provider_default_names_the_config_key() {
        let log = SubstitutionLog::new();
        let error = admit(
            "deepseek",
            "deepseek-flash",
            &ModelProfile::new(),
            Some(ReasoningEffort::High),
            None,
            None,
            Some("llm.deepseek.reasoning_effort"),
            &log,
        )
        .unwrap_err();
        let LlmError::UnsupportedParameter { reason, .. } = &error else {
            panic!("expected UnsupportedParameter, got {error:?}");
        };
        assert!(
            reason.ends_with("(set by `llm.deepseek.reasoning_effort`)"),
            "{reason}"
        );
        // A request-level effort names no config key.
        let error = admit(
            "deepseek",
            "deepseek-flash",
            &ModelProfile::new(),
            Some(ReasoningEffort::High),
            None,
            None,
            None,
            &log,
        )
        .unwrap_err();
        let LlmError::UnsupportedParameter { reason, .. } = &error else {
            panic!("expected UnsupportedParameter, got {error:?}");
        };
        assert!(!reason.contains("set by"), "{reason}");
    }

    #[test]
    fn an_unset_effort_is_admitted_without_reading_the_profile() {
        // A default profile is `Unmeasured`, which would refuse anything: unset must not care.
        let log = SubstitutionLog::new();
        let admitted = admit(
            "deepseek",
            "m",
            &ModelProfile::new(),
            None,
            Some(ReasoningFallback::NearestLower),
            Some(10),
            None,
            &log,
        );
        assert!(matches!(admitted, Ok(None)));
    }

    #[test]
    fn an_unmeasured_model_refuses_naming_the_override_key() {
        let log = SubstitutionLog::new();
        let error = admit(
            "deepseek",
            "deepseek-flash",
            &ModelProfile::new(),
            Some(ReasoningEffort::High),
            None,
            None,
            None,
            &log,
        )
        .unwrap_err();
        match &error {
            LlmError::UnsupportedParameter {
                param,
                value,
                provider,
                model,
                reason,
                ..
            } => {
                assert_eq!(param, "reasoning_effort");
                assert_eq!(value, "high");
                assert_eq!(provider, "deepseek");
                assert_eq!(model, "deepseek-flash");
                assert!(
                    reason.contains("llm.deepseek.models.deepseek-flash.effort"),
                    "{reason}"
                );
            }
            other => panic!("expected UnsupportedParameter, got {other:?}"),
        }
    }

    #[test]
    fn wire_unsupported_names_reasoning_wire_only_on_self_hosted_providers() {
        let log = SubstitutionLog::new();
        let wire_unsupported = ModelProfile::new().with_effort(EffortSupport::WireUnsupported);
        for provider in ["ollama", "openai-compatible"] {
            let reason = reason_of(
                admit(
                    provider,
                    "m",
                    &wire_unsupported,
                    Some(ReasoningEffort::Low),
                    None,
                    None,
                    None,
                    &log,
                )
                .unwrap_err(),
            );
            assert!(
                reason.contains(&format!("llm.{provider}.reasoning_wire")),
                "{reason}"
            );
        }
        let reason = reason_of(
            admit(
                "deepseek",
                "m",
                &wire_unsupported,
                Some(ReasoningEffort::Low),
                None,
                None,
                None,
                &log,
            )
            .unwrap_err(),
        );
        assert!(!reason.contains("reasoning_wire"), "{reason}");
        assert!(reason.contains("no configuration unlocks it"), "{reason}");
    }

    #[test]
    fn a_refused_level_names_the_levels_the_model_does_take() {
        let log = SubstitutionLog::new();
        let model = profile(
            EffortLevels::new([ReasoningEffort::Low, ReasoningEffort::High]).with_off(true),
        );
        let reason = reason_of(
            admit(
                "openai",
                "m",
                &model,
                Some(ReasoningEffort::Medium),
                None,
                None,
                None,
                &log,
            )
            .unwrap_err(),
        );
        assert!(reason.contains("low"), "{reason}");
        assert!(reason.contains("high"), "{reason}");
        assert!(reason.contains("off"), "{reason}");
    }

    #[test]
    fn an_exact_level_resolves_with_no_substitution_and_no_warning() {
        let log = SubstitutionLog::new();
        let model = profile(EffortLevels::new([ReasoningEffort::High]));
        let resolved = admit(
            "deepseek",
            "m",
            &model,
            Some(ReasoningEffort::High),
            None,
            None,
            None,
            &log,
        )
        .unwrap()
        .expect("an effort was set");
        assert_eq!(resolved.requested, ReasoningEffort::High);
        assert_eq!(resolved.effective, ReasoningEffort::High);
        assert!(!resolved.substituted());
    }

    #[test]
    fn a_substitution_warns_once_per_model_requested_and_effective() {
        let log = SubstitutionLog::new();
        assert!(log.record(
            "deepseek",
            "m",
            ReasoningEffort::XHigh,
            ReasoningEffort::High
        ));
        assert!(!log.record(
            "deepseek",
            "m",
            ReasoningEffort::XHigh,
            ReasoningEffort::High
        ));
        // A different model, requested or effective value is a new key.
        assert!(log.record(
            "deepseek",
            "other",
            ReasoningEffort::XHigh,
            ReasoningEffort::High
        ));
        assert!(log.record("deepseek", "m", ReasoningEffort::Max, ReasoningEffort::High));
    }

    #[test]
    fn admit_substitutes_only_on_request_and_records_it() {
        let log = SubstitutionLog::new();
        let model = profile(EffortLevels::new([
            ReasoningEffort::Low,
            ReasoningEffort::High,
        ]));
        let refused = admit(
            "deepseek",
            "m",
            &model,
            Some(ReasoningEffort::XHigh),
            None,
            None,
            None,
            &log,
        );
        assert!(refused.is_err(), "Refuse is the default");

        let resolved = admit(
            "deepseek",
            "m",
            &model,
            Some(ReasoningEffort::XHigh),
            Some(ReasoningFallback::NearestLower),
            None,
            None,
            &log,
        )
        .unwrap()
        .expect("an effort was set");
        assert_eq!(resolved.effective, ReasoningEffort::High);
        assert!(resolved.substituted());
        // The warning was spent by the first admit, so a second identical one stays quiet.
        assert!(!log.record(
            "deepseek",
            "m",
            ReasoningEffort::XHigh,
            ReasoningEffort::High
        ));
    }

    fn budget_profile() -> ModelProfile {
        profile(
            EffortLevels::new([ReasoningEffort::High]).with_budget(BudgetBounds::new(1024, 32_000)),
        )
    }

    #[test]
    fn a_budget_must_stay_below_max_tokens_by_at_least_one() {
        let log = SubstitutionLog::new();
        let run = |max_tokens: Option<u32>| {
            admit(
                "kimi",
                "m",
                &budget_profile(),
                Some(ReasoningEffort::Budget(2048)),
                None,
                max_tokens,
                None,
                &log,
            )
        };
        let reason = reason_of(run(Some(2048)).unwrap_err());
        assert!(reason.contains("budget must be below max"), "{reason}");
        assert!(reason.contains("2048"), "{reason}");
        assert!(
            run(Some(2049)).unwrap().is_some(),
            "n == max_tokens - 1 passes"
        );
        assert!(run(Some(100)).is_err(), "a budget above the cap is refused");
        assert!(run(None).unwrap().is_some(), "no cap, no output check");
    }

    #[test]
    fn the_profile_refusal_wins_over_the_output_check() {
        // A budget outside the span is refused as out of bounds, whatever max_tokens says.
        let log = SubstitutionLog::new();
        let reason = reason_of(
            admit(
                "kimi",
                "m",
                &budget_profile(),
                Some(ReasoningEffort::Budget(10)),
                None,
                Some(5),
                None,
                &log,
            )
            .unwrap_err(),
        );
        assert!(reason.contains("1024"), "{reason}");
        assert!(!reason.contains("max_tokens"), "{reason}");
    }

    #[test]
    fn the_output_check_applies_to_budgets_only() {
        let log = SubstitutionLog::new();
        let model = profile(EffortLevels::new([ReasoningEffort::High]));
        let admitted = admit(
            "kimi",
            "m",
            &model,
            Some(ReasoningEffort::High),
            None,
            Some(1),
            None,
            &log,
        );
        assert!(admitted.unwrap().is_some());
    }

    #[test]
    fn wire_caps_none_masks_any_support_to_wire_unsupported() {
        for support in [
            EffortSupport::Unmeasured,
            EffortSupport::NoReasoning,
            EffortSupport::Supported(EffortLevels::new([ReasoningEffort::High])),
        ] {
            assert_eq!(
                mask_effort(support, WireCaps::NONE),
                EffortSupport::WireUnsupported
            );
        }
    }

    #[test]
    fn mask_effort_removes_what_the_wire_cannot_carry() {
        let full = EffortSupport::Supported(
            EffortLevels::new([ReasoningEffort::Low, ReasoningEffort::High])
                .with_off(true)
                .with_auto(true)
                .with_budget(BudgetBounds::new(1, 4096))
                .with_coerced([ReasoningEffort::Medium]),
        );
        let caps = WireCaps {
            levels: true,
            off: true,
            auto: true,
            budget: false,
        };
        let EffortSupport::Supported(masked) = mask_effort(full.clone(), caps) else {
            panic!("a wire that carries levels stays Supported");
        };
        assert_eq!(masked.levels, [ReasoningEffort::Low, ReasoningEffort::High]);
        assert!(masked.off && masked.auto);
        assert_eq!(masked.budget, None);
        assert_eq!(masked.coerced, [ReasoningEffort::Medium]);

        let no_levels = WireCaps {
            levels: false,
            off: true,
            auto: false,
            budget: true,
        };
        let EffortSupport::Supported(masked) = mask_effort(full, no_levels) else {
            panic!("still Supported");
        };
        assert!(masked.levels.is_empty() && masked.coerced.is_empty());
        assert!(masked.off && !masked.auto);
        assert_eq!(masked.budget, Some(BudgetBounds::new(1, 4096)));
    }

    #[test]
    fn mask_effort_passes_unmeasured_and_no_reasoning_through_on_a_real_wire() {
        let caps = WireCaps {
            levels: true,
            off: true,
            auto: true,
            budget: false,
        };
        assert_eq!(
            mask_effort(EffortSupport::Unmeasured, caps),
            EffortSupport::Unmeasured
        );
        assert_eq!(
            mask_effort(EffortSupport::NoReasoning, caps),
            EffortSupport::NoReasoning
        );
    }
}
