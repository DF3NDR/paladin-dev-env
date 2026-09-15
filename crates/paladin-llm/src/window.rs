//! The shared context-window precedence resolver every consumer of
//! [`ProviderCapabilities`] calls through, instead of writing its own
//! config-table/capability/fallback walk (PRIM-04, D-01/D-02).
//!
//! [`resolve_context_window`] applies a fixed, four-step precedence order:
//!
//! 1. `config_table.get(model)` -- a per-model override, when one exists.
//! 2. `capabilities.max_context_tokens` -- the provider's own declared window.
//! 3. `policy`'s framework default (the lenient [`WindowFallbackPolicy::Default`] branch).
//! 4. `policy`'s caller-supplied fallback (the strict [`WindowFallbackPolicy::Strict`]
//!    branch), or refusal if the caller supplied none.
//!
//! **Strictness is a TYPE, not a flag.** [`WindowFallbackPolicy`] carries the
//! lenient/strict distinction as two enum variants rather than a boolean flag plus a
//! separately-held `Option<u32>` -- a flag and an option can disagree with each other; a
//! two-variant enum cannot. This is the same no-invented-window stance ADR-0010 applies
//! elsewhere in this crate: the framework never manufactures a context window on its
//! own. [`WindowFallbackPolicy::Default`] always resolves because its `u32` IS the
//! framework's own documented default, supplied by the caller at construction time, not
//! invented here. [`WindowFallbackPolicy::Strict`] carries only the CALLER's own
//! `Option<u32>` fallback -- when that is `None`, the resolver refuses with
//! [`UnknownContextWindow`] rather than guessing. Supplying a fallback is explicit
//! caller policy either way; this module never invents one.

use std::collections::HashMap;

use paladin_ports::output::llm_port::ProviderCapabilities;
use thiserror::Error;

/// How [`resolve_context_window`] behaves when neither the config table nor the
/// provider's declared capabilities produce a window.
///
/// Two variants, not a boolean flag plus a separate `Option<u32>` -- see the
/// [module-level documentation](self) for why the distinction is a type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowFallbackPolicy {
    /// Always resolves: `tokens` is the framework's own documented default, supplied by
    /// the caller at construction time (e.g. `HistoryTrimmerConfig::default_context_tokens`).
    /// This is not the resolver inventing a window -- the caller already decided this
    /// number ahead of time as its policy for "no window is known".
    Default(u32),
    /// May refuse. `caller_fallback` is the CALLER's own fallback (e.g.
    /// `CommissaryPlan::fallback_context_tokens`) -- used only when both the config table
    /// and the provider's capabilities are silent. `None` means the caller supplied no
    /// fallback, and the resolver returns [`UnknownContextWindow`] rather than guessing.
    Strict {
        /// The caller-supplied fallback window, or `None` to refuse when no earlier step
        /// resolved one.
        caller_fallback: Option<u32>,
    },
}

/// Which of [`resolve_context_window`]'s four precedence steps produced a resolved
/// window -- carried alongside the token count on [`ResolvedWindow`] so an operator
/// reading a debug log can answer "why did my history get trimmed at this number"
/// (or "why was this prompt measured against this allowance") without guessing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowSource {
    /// The model had an entry in the caller-supplied config table.
    ConfigTable,
    /// The provider's own [`ProviderCapabilities::max_context_tokens`] had a value.
    ProviderCapabilities,
    /// Neither of the above; [`WindowFallbackPolicy::Default`]'s framework default was
    /// used.
    Default,
    /// Neither the config table nor the provider's capabilities resolved a window, and
    /// the policy was [`WindowFallbackPolicy::Strict`] with a caller-supplied fallback
    /// present.
    CallerFallback,
}

impl WindowSource {
    /// Every [`WindowSource`] variant, exactly once, in precedence order -- the array the
    /// source-label invariant test walks so a future variant added without a label fails
    /// loudly rather than silently regressing an operator-facing log line.
    pub const ALL: [WindowSource; 4] = [
        WindowSource::ConfigTable,
        WindowSource::ProviderCapabilities,
        WindowSource::Default,
        WindowSource::CallerFallback,
    ];

    /// A human-readable label naming the exact field this source was read from.
    ///
    /// # Contract
    ///
    /// These strings are load-bearing: `HistoryTrimmer`'s own precedence tests assert on
    /// them by substring, so a rename here that drops a required substring is a silent
    /// regression for that consumer. The required substrings are:
    ///
    /// - [`WindowSource::ConfigTable`] contains `"config"`
    /// - [`WindowSource::ProviderCapabilities`] contains both `"provider"` and
    ///   `"capabilities"`
    /// - [`WindowSource::Default`] contains `"default"`
    ///
    /// No wildcard arm is used in the match below -- a new variant must be given a label
    /// here before it will compile.
    pub fn as_str(self) -> &'static str {
        match self {
            WindowSource::ConfigTable => "model_context_limits config table",
            WindowSource::ProviderCapabilities => {
                "provider capabilities (get_capabilities().max_context_tokens)"
            }
            WindowSource::Default => "default_context_tokens",
            WindowSource::CallerFallback => "caller-supplied fallback_context_tokens",
        }
    }
}

/// The result of a successful [`resolve_context_window`] call: the resolved token count
/// and which precedence step produced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedWindow {
    /// The resolved context-window size, in tokens.
    pub tokens: u32,
    /// Which precedence step produced `tokens`.
    pub source: WindowSource,
}

/// No context window could be resolved for `model`: the config table had no entry, the
/// provider declared no `max_context_tokens`, and the [`WindowFallbackPolicy`] was
/// [`WindowFallbackPolicy::Strict`] with no caller-supplied fallback. The framework never
/// invents a window -- this error is the refusal, not a bug.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("no context window could be resolved for model '{model}'")]
pub struct UnknownContextWindow {
    /// The model the caller asked about.
    pub model: String,
}

/// Resolves a context window for `model` through the fixed four-step precedence order
/// documented at the [module level](self): `config_table` entry, then
/// `capabilities.max_context_tokens`, then `policy`'s terminal step.
///
/// Synchronous, pure, no I/O: identical arguments always produce an identical
/// [`ResolvedWindow`] (or [`UnknownContextWindow`]), regardless of call order or how many
/// other calls happen in between.
///
/// `config_table` accepts `None` (no table at all) and `Some(&empty_map)`
/// interchangeably -- both are a no-op for step 1, falling through to step 2. A consumer
/// with no per-model override concept (e.g. `Commissary`, which has no config-table field
/// of its own) simply always passes `None`.
///
/// # Errors
///
/// Returns [`UnknownContextWindow`] when `policy` is [`WindowFallbackPolicy::Strict`]
/// with `caller_fallback: None` and neither the config table nor the provider's
/// capabilities resolved a window.
pub fn resolve_context_window(
    model: &str,
    config_table: Option<&HashMap<String, u32>>,
    capabilities: &ProviderCapabilities,
    policy: WindowFallbackPolicy,
) -> Result<ResolvedWindow, UnknownContextWindow> {
    if let Some(&tokens) = config_table.and_then(|table| table.get(model)) {
        return Ok(ResolvedWindow {
            tokens,
            source: WindowSource::ConfigTable,
        });
    }

    if let Some(tokens) = capabilities.max_context_tokens {
        return Ok(ResolvedWindow {
            tokens,
            source: WindowSource::ProviderCapabilities,
        });
    }

    match policy {
        WindowFallbackPolicy::Default(tokens) => Ok(ResolvedWindow {
            tokens,
            source: WindowSource::Default,
        }),
        WindowFallbackPolicy::Strict {
            caller_fallback: Some(tokens),
        } => Ok(ResolvedWindow {
            tokens,
            source: WindowSource::CallerFallback,
        }),
        WindowFallbackPolicy::Strict {
            caller_fallback: None,
        } => Err(UnknownContextWindow {
            model: model.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capabilities(max_context_tokens: Option<u32>) -> ProviderCapabilities {
        ProviderCapabilities {
            max_context_tokens,
            ..ProviderCapabilities::default()
        }
    }

    /// Precedence test 1 of 4: a config-table entry beats a present provider capability.
    #[test]
    fn precedence_config_table_beats_a_present_capability() {
        let mut table = HashMap::new();
        table.insert("gpt-4".to_string(), 1_234);

        let resolved = resolve_context_window(
            "gpt-4",
            Some(&table),
            &capabilities(Some(8_765)),
            WindowFallbackPolicy::Strict {
                caller_fallback: None,
            },
        )
        .unwrap();

        assert_eq!(resolved.tokens, 1_234);
        assert_eq!(resolved.source, WindowSource::ConfigTable);
    }

    /// Equal-values edge (PRIM-04/adjacency): a table entry and a capability carrying the
    /// identical value still report the config-table source -- equal values neither merge
    /// nor leave the reported source ambiguous.
    #[test]
    fn equal_values_still_report_the_config_table_source() {
        let mut table = HashMap::new();
        table.insert("gpt-4".to_string(), 8_765);

        let resolved = resolve_context_window(
            "gpt-4",
            Some(&table),
            &capabilities(Some(8_765)),
            WindowFallbackPolicy::Strict {
                caller_fallback: None,
            },
        )
        .unwrap();

        assert_eq!(resolved.tokens, 8_765);
        assert_eq!(resolved.source, WindowSource::ConfigTable);
    }

    /// Precedence test 2 of 4: a present provider capability beats a strict policy's
    /// caller-supplied fallback.
    #[test]
    fn precedence_provider_capability_beats_the_caller_fallback() {
        let resolved = resolve_context_window(
            "gpt-4",
            None,
            &capabilities(Some(8_765)),
            WindowFallbackPolicy::Strict {
                caller_fallback: Some(2_222),
            },
        )
        .unwrap();

        assert_eq!(resolved.tokens, 8_765);
        assert_eq!(resolved.source, WindowSource::ProviderCapabilities);
    }

    /// Precedence test 3 of 4: with the table and the capability both absent, a lenient
    /// policy resolves to its framework default rather than erroring.
    #[test]
    fn precedence_lenient_policy_resolves_to_the_framework_default() {
        let resolved = resolve_context_window(
            "gpt-4",
            None,
            &capabilities(None),
            WindowFallbackPolicy::Default(4_321),
        )
        .unwrap();

        assert_eq!(resolved.tokens, 4_321);
        assert_eq!(resolved.source, WindowSource::Default);
    }

    /// With the table and the capability both absent, a strict policy carrying a
    /// caller-supplied fallback resolves to it, reporting the caller-fallback source --
    /// the only case that reaches [`WindowSource::CallerFallback`].
    #[test]
    fn strict_policy_resolves_to_the_callers_fallback_when_present() {
        let resolved = resolve_context_window(
            "gpt-4",
            None,
            &capabilities(None),
            WindowFallbackPolicy::Strict {
                caller_fallback: Some(2_222),
            },
        )
        .unwrap();

        assert_eq!(resolved.tokens, 2_222);
        assert_eq!(resolved.source, WindowSource::CallerFallback);
    }

    /// Precedence test 4 of 4: with the table and the capability both absent, a strict
    /// policy with no caller-supplied fallback refuses, and the rendered error message
    /// names the model it was asked about.
    #[test]
    fn precedence_strict_policy_with_no_fallback_returns_the_error() {
        let err = resolve_context_window(
            "gpt-4",
            None,
            &capabilities(None),
            WindowFallbackPolicy::Strict {
                caller_fallback: None,
            },
        )
        .unwrap_err();

        assert_eq!(err.model, "gpt-4");
        assert!(
            err.to_string().contains("gpt-4"),
            "the rendered error message must name the model: {err}"
        );
    }

    /// EDGE(PRIM-04/empty): an absent config table (`None`) and a present-but-empty one
    /// behave identically -- both are a no-op, falling through to the next step.
    #[test]
    fn absent_and_empty_config_table_behave_identically() {
        let empty: HashMap<String, u32> = HashMap::new();

        let with_none = resolve_context_window(
            "gpt-4",
            None,
            &capabilities(Some(8_765)),
            WindowFallbackPolicy::Default(1),
        )
        .unwrap();
        let with_empty = resolve_context_window(
            "gpt-4",
            Some(&empty),
            &capabilities(Some(8_765)),
            WindowFallbackPolicy::Default(1),
        )
        .unwrap();

        assert_eq!(with_none, with_empty);
        assert_eq!(with_none.source, WindowSource::ProviderCapabilities);
    }

    /// EDGE(PRIM-04/ordering): the resolver is a pure function of its four arguments --
    /// repeated calls with identical arguments, in any order and interleaved with other
    /// calls, return the identical count-and-source pair.
    #[test]
    fn resolution_is_pure_across_repeated_and_interleaved_calls() {
        let mut table = HashMap::new();
        table.insert("gpt-4".to_string(), 1_234);
        let caps = capabilities(Some(8_765));
        let policy = WindowFallbackPolicy::Strict {
            caller_fallback: None,
        };

        let first = resolve_context_window("gpt-4", Some(&table), &caps, policy).unwrap();
        // An unrelated call interleaved in between must not perturb the result.
        let _unrelated = resolve_context_window(
            "claude-3",
            None,
            &capabilities(None),
            WindowFallbackPolicy::Default(999),
        )
        .unwrap();
        let second = resolve_context_window("gpt-4", Some(&table), &caps, policy).unwrap();
        let third = resolve_context_window("gpt-4", Some(&table), &caps, policy).unwrap();

        assert_eq!(first, second);
        assert_eq!(second, third);
    }

    /// The source-label invariant (D-05): every [`WindowSource`] variant's label is
    /// non-empty and unique, and the three labels `HistoryTrimmer`'s existing precedence
    /// tests assert on by substring carry their required substring. A later variant added
    /// without a label, or a rename that drops a required substring, fails this test
    /// rather than silently regressing the logged source an operator reads.
    #[test]
    fn source_label_invariant_walks_every_variant() {
        let labels: Vec<&str> = WindowSource::ALL.iter().map(|s| s.as_str()).collect();

        for label in &labels {
            assert!(!label.is_empty(), "every source label must be non-empty");
        }

        let mut unique = labels.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(
            unique.len(),
            labels.len(),
            "every source label must be unique: {labels:?}"
        );

        assert!(
            WindowSource::ConfigTable.as_str().contains("config"),
            "missing required substring 'config' in {:?}",
            WindowSource::ConfigTable.as_str()
        );
        assert!(
            WindowSource::ProviderCapabilities
                .as_str()
                .contains("provider")
                && WindowSource::ProviderCapabilities
                    .as_str()
                    .contains("capabilities"),
            "missing required substring 'provider' and/or 'capabilities' in {:?}",
            WindowSource::ProviderCapabilities.as_str()
        );
        assert!(
            WindowSource::Default.as_str().contains("default"),
            "missing required substring 'default' in {:?}",
            WindowSource::Default.as_str()
        );
    }
}
