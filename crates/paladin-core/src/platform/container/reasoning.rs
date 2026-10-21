//! Reasoning effort: how hard a model is asked to think (Phase 51, REASON-01).
//!
//! A [`ReasoningEffort`] is a unit in plain words (`low`, `high`, `xhigh`), not a vendor wire
//! value. Each adapter maps it onto the spelling its own vendor takes, and a model that cannot
//! honour it refuses the request before any HTTP call instead of silently dropping it.
//!
//! - [`ReasoningEffort::Auto`] means "on, at the vendor's default depth". It is a **value**, and is
//!   distinct from an unset effort (`None` on the request), which sends nothing at all and leaves
//!   the vendor default in force.
//! - [`ReasoningEffort::Budget`] is a reasoning-token budget. Only some vendors take one, and it
//!   is never converted to a level or the other way round.
//! - [`ReasoningFallback`] is the caller's choice of what to do when the model lacks the exact
//!   level asked for: refuse (the default), or step to the nearest lower or higher level.
//!
//! The types live here, in the domain core, so that `PaladinData` can hold them and dependencies
//! keep flowing inward; `paladin-ports` re-exports them on its `llm_port` module.
//!
//! # Examples
//!
//! ```
//! use paladin_core::platform::container::reasoning::{ReasoningEffort, ReasoningFallback};
//!
//! assert_eq!(ReasoningEffort::XHigh.to_string(), "xhigh");
//! assert_eq!(ReasoningEffort::Budget(2048).to_string(), "budget:2048");
//! assert_eq!(ReasoningFallback::default(), ReasoningFallback::Refuse);
//! ```

use std::fmt;

use serde::{Deserialize, Serialize};

/// Response-metadata key recording the effort the caller asked for, stamped beside
/// `paladin.served_by` and only when an effort was set (an unset request leaves the response
/// metadata untouched).
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::reasoning::REASONING_REQUESTED_METADATA_KEY;
///
/// assert_eq!(REASONING_REQUESTED_METADATA_KEY, "paladin.reasoning_requested");
/// ```
pub const REASONING_REQUESTED_METADATA_KEY: &str = "paladin.reasoning_requested";

/// Response-metadata key recording the effort that actually reached the wire, stamped beside
/// `paladin.served_by` and only when an effort was set. It differs from
/// [`REASONING_REQUESTED_METADATA_KEY`] only when the caller chose a substituting
/// [`ReasoningFallback`].
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::reasoning::REASONING_EFFECTIVE_METADATA_KEY;
///
/// assert_eq!(REASONING_EFFECTIVE_METADATA_KEY, "paladin.reasoning_effective");
/// ```
pub const REASONING_EFFECTIVE_METADATA_KEY: &str = "paladin.reasoning_effective";

/// How hard a model is asked to think, in provider-neutral words.
///
/// The six levels run `Minimal < Low < Medium < High < XHigh < Max`; see
/// [`level_rank`](Self::level_rank). `Off`, `Auto` and `Budget` are not levels and are never
/// ranked or substituted for one another.
///
/// Serde spells the unit variants `off auto minimal low medium high xhigh max` and a budget as
/// `{"budget":2048}`. [`Display`](fmt::Display) gives the same words and `budget:2048`.
///
/// Marked `#[non_exhaustive]`: match it with a wildcard arm.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::reasoning::ReasoningEffort;
///
/// assert_eq!(serde_json::to_string(&ReasoningEffort::XHigh)?, "\"xhigh\"");
/// assert_eq!(serde_json::to_string(&ReasoningEffort::Budget(2048))?, "{\"budget\":2048}");
/// assert_eq!(ReasoningEffort::High.level_rank(), Some(3));
/// assert_eq!(ReasoningEffort::Auto.level_rank(), None);
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ReasoningEffort {
    /// Reasoning explicitly off, where the model can be run without it.
    Off,
    /// Reasoning on, at the vendor's own default depth. Not the same as an unset effort.
    Auto,
    /// The lightest level.
    Minimal,
    /// A light level.
    Low,
    /// A middle level.
    Medium,
    /// A deep level.
    High,
    /// A deeper level than `High`.
    #[serde(rename = "xhigh")]
    XHigh,
    /// The deepest level.
    Max,
    /// A reasoning-token budget, kept exactly as given (never rounded).
    Budget(u32),
}

impl ReasoningEffort {
    /// The six levels in ascending order of depth.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_core::platform::container::reasoning::ReasoningEffort;
    ///
    /// assert_eq!(ReasoningEffort::LEVELS[0], ReasoningEffort::Minimal);
    /// assert_eq!(ReasoningEffort::LEVELS[5], ReasoningEffort::Max);
    /// ```
    pub const LEVELS: [ReasoningEffort; 6] = [
        ReasoningEffort::Minimal,
        ReasoningEffort::Low,
        ReasoningEffort::Medium,
        ReasoningEffort::High,
        ReasoningEffort::XHigh,
        ReasoningEffort::Max,
    ];

    /// The position of a level in the total order `Minimal(0) < Low(1) < Medium(2) < High(3) <
    /// XHigh(4) < Max(5)`; `None` for `Off`, `Auto` and `Budget`, which are not levels.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_core::platform::container::reasoning::ReasoningEffort;
    ///
    /// assert_eq!(ReasoningEffort::Minimal.level_rank(), Some(0));
    /// assert_eq!(ReasoningEffort::Max.level_rank(), Some(5));
    /// assert_eq!(ReasoningEffort::Budget(1024).level_rank(), None);
    /// ```
    #[must_use]
    pub const fn level_rank(self) -> Option<u8> {
        match self {
            Self::Minimal => Some(0),
            Self::Low => Some(1),
            Self::Medium => Some(2),
            Self::High => Some(3),
            Self::XHigh => Some(4),
            Self::Max => Some(5),
            Self::Off | Self::Auto | Self::Budget(_) => None,
        }
    }

    /// Whether this is one of the six ranked levels.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_core::platform::container::reasoning::ReasoningEffort;
    ///
    /// assert!(ReasoningEffort::Medium.is_level());
    /// assert!(!ReasoningEffort::Off.is_level());
    /// ```
    #[must_use]
    pub const fn is_level(self) -> bool {
        self.level_rank().is_some()
    }
}

impl fmt::Display for ReasoningEffort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Off => f.write_str("off"),
            Self::Auto => f.write_str("auto"),
            Self::Minimal => f.write_str("minimal"),
            Self::Low => f.write_str("low"),
            Self::Medium => f.write_str("medium"),
            Self::High => f.write_str("high"),
            Self::XHigh => f.write_str("xhigh"),
            Self::Max => f.write_str("max"),
            Self::Budget(n) => write!(f, "budget:{n}"),
        }
    }
}

/// What to do when a model lacks the exact effort level the caller asked for (D-05, D-06).
///
/// Only a **level** is ever substituted, and only along the direction the caller chose: the rule
/// never flips direction, and never substitutes `Off`, `Auto`, a `Budget` or a level the vendor
/// silently runs as another.
///
/// Marked `#[non_exhaustive]`: match it with a wildcard arm.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::reasoning::ReasoningFallback;
///
/// assert_eq!(ReasoningFallback::default(), ReasoningFallback::Refuse);
/// assert_eq!(ReasoningFallback::NearestLower.to_string(), "nearest_lower");
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ReasoningFallback {
    /// Refuse the request when the exact level is not available. The default.
    #[default]
    Refuse,
    /// Step to the closest listed level strictly below the one asked for.
    NearestLower,
    /// Step to the closest listed level strictly above the one asked for.
    NearestHigher,
}

impl fmt::Display for ReasoningFallback {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Refuse => f.write_str("refuse"),
            Self::NearestLower => f.write_str("nearest_lower"),
            Self::NearestHigher => f.write_str("nearest_higher"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [ReasoningEffort; 9] = [
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

    /// A3-51-01: serde snake_case would spell `XHigh` as `x_high`; the explicit rename is pinned.
    #[test]
    fn effort_spellings_round_trip_through_serde() {
        let spellings = [
            "\"off\"",
            "\"auto\"",
            "\"minimal\"",
            "\"low\"",
            "\"medium\"",
            "\"high\"",
            "\"xhigh\"",
            "\"max\"",
            "{\"budget\":2048}",
        ];
        for (effort, spelling) in ALL.iter().zip(spellings) {
            assert_eq!(serde_json::to_string(effort).unwrap(), spelling);
            let back: ReasoningEffort = serde_json::from_str(spelling).unwrap();
            assert_eq!(back, *effort);
        }
        assert_eq!(
            serde_json::to_value(ReasoningEffort::XHigh).unwrap(),
            serde_json::json!("xhigh")
        );
    }

    #[test]
    fn unknown_effort_spellings_are_rejected() {
        for bad in [
            "\"x_high\"",
            "\"ultra\"",
            "\"HIGH\"",
            "\"\"",
            "{\"budget\":-1}",
        ] {
            assert!(
                serde_json::from_str::<ReasoningEffort>(bad).is_err(),
                "{bad} deserialised"
            );
        }
    }

    #[test]
    fn effort_display_uses_the_same_lowercase_words() {
        let shown: Vec<String> = ALL.iter().map(ToString::to_string).collect();
        assert_eq!(
            shown,
            [
                "off",
                "auto",
                "minimal",
                "low",
                "medium",
                "high",
                "xhigh",
                "max",
                "budget:2048"
            ]
        );
    }

    #[test]
    fn level_rank_is_a_total_stable_order() {
        let ranks: Vec<Option<u8>> = ReasoningEffort::LEVELS
            .iter()
            .map(|e| e.level_rank())
            .collect();
        assert_eq!(
            ranks,
            [Some(0), Some(1), Some(2), Some(3), Some(4), Some(5)]
        );
        assert_eq!(ReasoningEffort::Off.level_rank(), None);
        assert_eq!(ReasoningEffort::Auto.level_rank(), None);
        assert_eq!(ReasoningEffort::Budget(1).level_rank(), None);
        for level in ReasoningEffort::LEVELS {
            assert!(level.is_level());
        }
        assert!(!ReasoningEffort::Off.is_level());
        assert!(!ReasoningEffort::Auto.is_level());
        assert!(!ReasoningEffort::Budget(7).is_level());
    }

    /// Edge REASON-01/precision: a budget is never rounded.
    #[test]
    fn budget_keeps_its_exact_value() {
        for n in [1_u32, 1024, 32_768, u32::MAX] {
            let effort = ReasoningEffort::Budget(n);
            let json = serde_json::to_string(&effort).unwrap();
            assert_eq!(json, format!("{{\"budget\":{n}}}"));
            assert_eq!(
                serde_json::from_str::<ReasoningEffort>(&json).unwrap(),
                effort
            );
            assert_eq!(effort.to_string(), format!("budget:{n}"));
        }
    }

    #[test]
    fn auto_is_a_value_distinct_from_unset() {
        let set: Option<ReasoningEffort> = Some(ReasoningEffort::Auto);
        assert_ne!(set, None);
    }

    #[test]
    fn fallback_defaults_to_refuse_and_round_trips() {
        assert_eq!(ReasoningFallback::default(), ReasoningFallback::Refuse);
        for (fallback, word) in [
            (ReasoningFallback::Refuse, "refuse"),
            (ReasoningFallback::NearestLower, "nearest_lower"),
            (ReasoningFallback::NearestHigher, "nearest_higher"),
        ] {
            assert_eq!(
                serde_json::to_string(&fallback).unwrap(),
                format!("\"{word}\"")
            );
            assert_eq!(fallback.to_string(), word);
            let back: ReasoningFallback = serde_json::from_str(&format!("\"{word}\"")).unwrap();
            assert_eq!(back, fallback);
        }
        assert!(serde_json::from_str::<ReasoningFallback>("\"ultra\"").is_err());
    }

    #[test]
    fn metadata_keys_sit_beside_served_by() {
        assert_eq!(
            REASONING_REQUESTED_METADATA_KEY,
            "paladin.reasoning_requested"
        );
        assert_eq!(
            REASONING_EFFECTIVE_METADATA_KEY,
            "paladin.reasoning_effective"
        );
    }
}
