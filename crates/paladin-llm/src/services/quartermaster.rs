//! The Quartermaster — the officer who allocates supplies under scarcity.
//!
//! `ProviderCapabilities::max_context_tokens` (`paladin_ports::output::llm_port`) is
//! declared by every LLM adapter shipped in this crate, yet before this module existed it
//! had zero production readers: `PaladinBuilder` already reads the neighbouring
//! `temperature_range` field pre-flight and errors rather than silently clamping
//! (ADR-0004) — this module applies the identical stance to the INPUT side of a call, the
//! assembled prompt, rather than the output-side temperature.
//!
//! Two responsibilities, kept deliberately separate:
//!
//! - [`Quartermaster::verify_fits`] — a pre-flight GUARD. It measures an already-assembled
//!   prompt against the provider's declared window (minus the caller's reserved
//!   completion budget) and returns an error naming the measured tokens, the allowance,
//!   and the provider when it would overflow. It never trims.
//! - [`Quartermaster::apportion`] — a bounded ALLOCATOR. Given FIXED (non-sheddable)
//!   material and a [`Convoy`] of caller-prioritised, shed-or-truncate-able material, it
//!   returns an [`Allotment`]: every retained item clamped to a per-item share (with a
//!   visible truncation marker when a cut was needed) and every shed item recorded with
//!   its label, priority, and original size. Nothing is dropped silently — the explicit
//!   anti-pattern this module rejects is
//!   `paladin_memory::services::rag_retrieval_service::RagRetrievalService::
//!   truncate_to_token_budget`, which drops lowest-scoring items with no marker and no
//!   record.
//!
//! **Framework owns measurement and enforcement; callers own policy.** The Quartermaster
//! never decides WHICH material matters more — that is the caller-supplied `priority` on
//! each [`ConvoyItem`]. No audit-specific (or any other application-specific) policy
//! crosses into this crate.
//!
//! **Honesty clause.** Not every model has an exact tokenizer available offline
//! (`claude-*`, `deepseek-*`). [`Allotment::exact_tally`] propagates
//! [`TokenCounter::is_exact`] so a caller reading a `Quartermaster`-produced allotment can
//! tell an exact tally from an estimate and budget its own margin accordingly.

use std::sync::Arc;

use paladin_ports::output::llm_port::{LlmPort, ProviderCapabilities};
use paladin_ports::output::token_counter_port::{
    PESSIMISTIC_TOKENS_PER_1000_BYTES, TokenCountError, TokenCounter,
};
use thiserror::Error;

/// Configuration governing how a [`Quartermaster`] resolves its allowance and apportions
/// material into a bounded [`Allotment`].
#[derive(Debug, Clone)]
pub struct AllotmentConfig {
    /// How many tokens of the provider's declared window the CALLER intends to spend on
    /// the completion.
    ///
    /// This must be the caller's LARGEST completion budget (its escalated retry, not its
    /// base), because the escalated attempt is the one that must still fit inside the
    /// declared window alongside the prompt.
    pub reserved_completion_tokens: u32,

    /// Used ONLY when the provider's [`ProviderCapabilities::max_context_tokens`] is
    /// `None`. This is caller policy: the framework will not invent a window on its own.
    pub fallback_context_tokens: Option<u32>,

    /// The minimum byte share a single retained [`ConvoyItem`] may be allotted.
    pub per_item_min_bytes: usize,

    /// The maximum byte share a single retained [`ConvoyItem`] may be allotted, even when
    /// the available budget divided across the retained set would otherwise hand it more.
    pub per_item_max_bytes: usize,

    /// The tokens-per-1000-bytes ratio used ONLY for byte-share PLANNING while apportioning
    /// (deciding how many bytes each item may keep). The final tally reported on the
    /// returned [`Allotment`] always goes through the configured [`TokenCounter`], never
    /// this ratio. Defaults to
    /// [`PESSIMISTIC_TOKENS_PER_1000_BYTES`]. Zero is rejected at
    /// [`Quartermaster::new`] construction time — the division-hazard guard.
    pub pessimistic_tokens_per_1000_bytes: u32,

    /// Appended to a retained item's body when it had to be shortened to fit its share, so
    /// a truncated excerpt is never mistaken for a complete one.
    pub truncation_marker: String,
}

impl Default for AllotmentConfig {
    fn default() -> Self {
        Self {
            reserved_completion_tokens: 0,
            fallback_context_tokens: None,
            per_item_min_bytes: 0,
            per_item_max_bytes: usize::MAX,
            pessimistic_tokens_per_1000_bytes: PESSIMISTIC_TOKENS_PER_1000_BYTES,
            truncation_marker: "\n... (truncated)".to_string(),
        }
    }
}

/// A single labelled piece of material a caller wants considered for a [`Convoy`].
///
/// # Priority contract
///
/// `priority` is unambiguous: a LOWER number is HIGHER priority and is shed LAST. Ties
/// keep insertion order (the sort [`Quartermaster::apportion`] performs is stable).
#[derive(Debug, Clone)]
pub struct ConvoyItem {
    /// A human-readable label identifying this item (surfaced on [`ShedItem`] and
    /// [`ApportionedItem`] so a caller can tell which material was affected).
    pub label: String,
    /// The material itself.
    pub body: String,
    /// Lower number == higher priority == shed LAST. Ties keep insertion order.
    pub priority: u8,
}

/// An ordered collection of [`ConvoyItem`]s awaiting apportionment. Carries no shedding
/// or truncation policy of its own — that lives entirely in [`Quartermaster::apportion`],
/// driven by each item's caller-supplied `priority`.
#[derive(Debug, Clone, Default)]
pub struct Convoy {
    items: Vec<ConvoyItem>,
}

impl Convoy {
    /// Creates an empty convoy.
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Appends an item to the convoy, in insertion order.
    pub fn push(&mut self, item: ConvoyItem) {
        self.items.push(item);
    }

    /// The number of items currently in the convoy.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the convoy currently holds no items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// A [`ConvoyItem`] that survived apportionment, possibly truncated to fit its share.
#[derive(Debug, Clone)]
pub struct ApportionedItem {
    /// Copied from the source [`ConvoyItem::label`].
    pub label: String,
    /// The (possibly truncated, possibly marker-suffixed) body actually retained.
    pub body: String,
    /// Whether this item's body had to be shortened to fit its allotted share.
    pub truncated: bool,
    /// The final byte length of `body` (including the truncation marker, if any).
    pub allotted_bytes: usize,
}

/// A [`ConvoyItem`] that did NOT survive apportionment — recorded so nothing is dropped
/// silently.
#[derive(Debug, Clone)]
pub struct ShedItem {
    /// Copied from the source [`ConvoyItem::label`].
    pub label: String,
    /// Copied from the source [`ConvoyItem::priority`].
    pub priority: u8,
    /// The byte length of the item's body BEFORE it was shed (never truncated — a shed
    /// item is dropped whole, not partially kept).
    pub original_bytes: usize,
}

/// The result of [`Quartermaster::apportion`]: every retained item (possibly truncated
/// and marked), every shed item (recorded, never silently dropped), and the final
/// measured tally.
#[derive(Debug, Clone)]
pub struct Allotment {
    /// Items that survived apportionment, in the order they were retained.
    pub apportioned: Vec<ApportionedItem>,
    /// Items that were shed to stay within budget, in the order they were shed.
    pub shed: Vec<ShedItem>,
    /// The final measured token tally of the fixed material plus every apportioned
    /// item's (possibly truncated) body.
    pub prompt_tokens: u32,
    /// The token allowance this allotment was apportioned against
    /// ([`Quartermaster::allotted_tokens`] at the time of apportioning).
    pub allotted_tokens: u32,
    /// Propagated from [`TokenCounter::is_exact`]: `true` if `prompt_tokens` is an exact
    /// tally, `false` if it is a deliberately over-counting estimate. A caller treating an
    /// estimate as exact would be over-trusting the guard — this field is how it avoids
    /// that mistake.
    pub exact_tally: bool,
}

impl Allotment {
    /// Renders the apportioned bodies, in retained order, concatenated into a single
    /// string ready for prompt assembly.
    pub fn render(&self) -> String {
        self.apportioned
            .iter()
            .map(|item| item.body.as_str())
            .collect()
    }
}

/// Errors a [`Quartermaster`] can return. Every variant names its own numbers — a caller
/// reading only the `Display` string can see measured-vs-allowed without inspecting the
/// struct.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum QuartermasterError {
    /// The provider declared no `max_context_tokens` and no
    /// [`AllotmentConfig::fallback_context_tokens`] was configured. The framework refuses
    /// to guess a window — supplying a fallback is explicit caller policy.
    #[error(
        "provider '{provider}' declared no max_context_tokens and no fallback_context_tokens \
         was configured"
    )]
    UndeclaredContextWindow {
        /// The provider whose declared capabilities were consulted.
        provider: String,
    },

    /// [`AllotmentConfig::reserved_completion_tokens`] is greater than or equal to the
    /// resolved context window, leaving zero or negative room for a prompt. Returned at
    /// [`Quartermaster::new`] construction time, before any apportioning work.
    #[error(
        "reserved_completion_tokens ({reserved}) must be strictly less than the resolved \
         context window ({window} tokens)"
    )]
    ReservationExceedsWindow {
        /// The configured `reserved_completion_tokens`.
        reserved: u32,
        /// The resolved context window (declared or fallback).
        window: u32,
    },

    /// The FIXED (non-sheddable) material passed to [`Quartermaster::apportion`] alone
    /// exceeds the byte allowance. This is an error, never a clamp (ADR-0004 stance
    /// applied to the input side): fixed material is, by definition, not something the
    /// Quartermaster is permitted to shorten.
    #[error(
        "fixed material alone measures {fixed_tokens} tokens, which meets or exceeds the \
         {allotted_tokens}-token allowance"
    )]
    FixedMaterialExceedsAllowance {
        /// The measured token count of the fixed material alone.
        fixed_tokens: u32,
        /// The token allowance the fixed material was measured against.
        allotted_tokens: u32,
    },

    /// [`Quartermaster::verify_fits`] measured a prompt that exceeds the allowance. Never
    /// trims — this is the pre-flight enforcement point (ADR-0004 stance).
    #[error(
        "prompt measured {measured_tokens} tokens, exceeding the {allotted_tokens}-token \
         allowance for provider '{provider}'"
    )]
    ContextOverflow {
        /// The measured token count of the assembled prompt.
        measured_tokens: u32,
        /// The token allowance the prompt was measured against.
        allotted_tokens: u32,
        /// The provider the allowance was resolved from.
        provider: String,
    },

    /// The [`AllotmentConfig`] passed to [`Quartermaster::new`] is internally invalid
    /// (e.g. a zero ratio, or `per_item_min_bytes > per_item_max_bytes`).
    #[error("invalid Quartermaster configuration: {0}")]
    InvalidConfig(String),

    /// A [`TokenCounter`] call failed while measuring material.
    #[error("token tally failed: {0}")]
    Tally(#[from] TokenCountError),
}

/// Measures an assembled prompt against a provider's OWN declared context window,
/// enforces it pre-flight, and apportions a bounded allotment under caller-supplied
/// priority.
///
/// See the [module-level documentation](self) for the framework/caller responsibility
/// split.
pub struct Quartermaster {
    counter: Arc<dyn TokenCounter>,
    capabilities: ProviderCapabilities,
    provider: String,
    config: AllotmentConfig,
}

impl std::fmt::Debug for Quartermaster {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Quartermaster")
            .field("provider", &self.provider)
            .field("capabilities", &self.capabilities)
            .field("config", &self.config)
            .finish()
    }
}

impl Quartermaster {
    /// Constructs a `Quartermaster` for `provider` from an already-obtained
    /// [`ProviderCapabilities`] and [`TokenCounter`].
    ///
    /// # Errors
    ///
    /// - [`QuartermasterError::InvalidConfig`] if `config.pessimistic_tokens_per_1000_bytes`
    ///   is zero, or if `config.per_item_min_bytes > config.per_item_max_bytes`.
    /// - [`QuartermasterError::UndeclaredContextWindow`] if `capabilities.max_context_tokens`
    ///   is `None` and `config.fallback_context_tokens` is also `None`.
    /// - [`QuartermasterError::ReservationExceedsWindow`] if
    ///   `config.reserved_completion_tokens` is greater than or equal to the resolved
    ///   window.
    pub fn new(
        provider: impl Into<String>,
        capabilities: ProviderCapabilities,
        counter: Arc<dyn TokenCounter>,
        config: AllotmentConfig,
    ) -> Result<Self, QuartermasterError> {
        let provider = provider.into();

        if config.pessimistic_tokens_per_1000_bytes == 0 {
            return Err(QuartermasterError::InvalidConfig(
                "pessimistic_tokens_per_1000_bytes must be non-zero".to_string(),
            ));
        }
        if config.per_item_min_bytes > config.per_item_max_bytes {
            return Err(QuartermasterError::InvalidConfig(format!(
                "per_item_min_bytes ({}) must be <= per_item_max_bytes ({})",
                config.per_item_min_bytes, config.per_item_max_bytes
            )));
        }

        let window = capabilities
            .max_context_tokens
            .or(config.fallback_context_tokens)
            .ok_or_else(|| QuartermasterError::UndeclaredContextWindow {
                provider: provider.clone(),
            })?;

        if config.reserved_completion_tokens >= window {
            return Err(QuartermasterError::ReservationExceedsWindow {
                reserved: config.reserved_completion_tokens,
                window,
            });
        }

        Ok(Self {
            counter,
            capabilities,
            provider,
            config,
        })
    }

    /// Constructs a `Quartermaster` by reading `llm`'s own declared provider name and
    /// capabilities — the method that gives `max_context_tokens` its first production
    /// reader.
    ///
    /// # Errors
    ///
    /// See [`Quartermaster::new`].
    pub fn from_port(
        llm: &dyn LlmPort,
        counter: Arc<dyn TokenCounter>,
        config: AllotmentConfig,
    ) -> Result<Self, QuartermasterError> {
        Self::new(
            llm.get_provider_name().to_string(),
            llm.get_capabilities(),
            counter,
            config,
        )
    }

    /// Resolves the context window: the provider's declared
    /// `max_context_tokens`, or [`AllotmentConfig::fallback_context_tokens`] when the
    /// provider declared none. Guaranteed to resolve to a value by the constructor's
    /// [`QuartermasterError::UndeclaredContextWindow`] guard.
    fn window(&self) -> u32 {
        self.capabilities
            .max_context_tokens
            .or(self.config.fallback_context_tokens)
            .unwrap_or(0)
    }

    /// The token allowance available for a prompt: the resolved window minus
    /// [`AllotmentConfig::reserved_completion_tokens`], saturating at zero.
    pub fn allotted_tokens(&self) -> u32 {
        self.window()
            .saturating_sub(self.config.reserved_completion_tokens)
    }

    /// Apportions `fixed` (non-sheddable material) plus `convoy` (caller-prioritised,
    /// shed-or-truncate-able material) into a bounded [`Allotment`].
    ///
    /// Algorithm, deterministic:
    ///
    /// 1. `allowance_bytes = allotted_tokens * 1000 / pessimistic_tokens_per_1000_bytes`
    ///    (the ratio is non-zero by construction — see [`Quartermaster::new`] — so this
    ///    division is provably safe without an `expect`).
    /// 2. If `fixed.len() >= allowance_bytes`, return
    ///    [`QuartermasterError::FixedMaterialExceedsAllowance`] — an error, never a clamp.
    /// 3. `budget = allowance_bytes - fixed.len()`.
    /// 4. Stable-sort convoy items by `(priority, insertion_index)`.
    /// 5. With `n` retained items, `share = (budget / n).clamp(min, max)`; each item's
    ///    provisional allotment is `min(item.body.len(), share)` plus the marker length
    ///    for any item that would be cut. If the provisional total exceeds `budget`, the
    ///    LAST item in sorted order (lowest priority) is shed and the step repeats. `n ==
    ///    0` is guarded before any division.
    /// 6. Each retained item is truncated to its final share with a char-boundary-safe
    ///    walk, with the truncation marker appended when a cut was needed.
    /// 7. The final `fixed + rendered` text is tallied through the configured
    ///    [`TokenCounter`] for `prompt_tokens`; [`TokenCounter::is_exact`] is carried into
    ///    `exact_tally`.
    /// 8. The allotment is returned with `shed` populated in shed order.
    ///
    /// # Errors
    ///
    /// - [`QuartermasterError::FixedMaterialExceedsAllowance`] if `fixed` alone meets or
    ///   exceeds the byte allowance.
    /// - [`QuartermasterError::Tally`] if the configured [`TokenCounter`] fails while
    ///   measuring the fixed material or the final assembled prompt.
    pub fn apportion(&self, fixed: &str, convoy: &Convoy) -> Result<Allotment, QuartermasterError> {
        let allotted_tokens = self.allotted_tokens();
        let ratio = self.config.pessimistic_tokens_per_1000_bytes;

        // Safe: `ratio` is non-zero by the `Quartermaster::new` constructor guard.
        let allowance_bytes: usize =
            usize::try_from(u64::from(allotted_tokens).saturating_mul(1000) / u64::from(ratio))
                .unwrap_or(usize::MAX);

        if fixed.len() >= allowance_bytes {
            let fixed_tokens = self.counter.count_tokens(fixed)?;
            return Err(QuartermasterError::FixedMaterialExceedsAllowance {
                fixed_tokens,
                allotted_tokens,
            });
        }

        let budget = allowance_bytes - fixed.len();

        // Stable-sort by (priority, insertion order): ascending priority number puts the
        // HIGHEST-priority items (lowest number) first. Popping from the end therefore
        // sheds the LOWEST-priority item first, per the documented priority contract.
        let mut retained: Vec<usize> = (0..convoy.items.len()).collect();
        retained.sort_by_key(|&idx| convoy.items[idx].priority);

        let mut shed: Vec<ShedItem> = Vec::new();
        let mut share = 0usize;

        loop {
            let n = retained.len();
            if n == 0 {
                break;
            }

            let raw_share = budget / n;
            share = raw_share.clamp(
                self.config.per_item_min_bytes,
                self.config.per_item_max_bytes,
            );

            let mut provisional_total = 0usize;
            for &idx in &retained {
                let item = &convoy.items[idx];
                let allotted = item.body.len().min(share);
                let will_cut = item.body.len() > share;
                let with_marker = if will_cut {
                    allotted.saturating_add(self.config.truncation_marker.len())
                } else {
                    allotted
                };
                provisional_total = provisional_total.saturating_add(with_marker);
            }

            if provisional_total <= budget || n == 1 {
                // n == 1: nothing lower-priority remains to shed — the single retained
                // item proceeds with whatever `share` it was clamped to.
                break;
            }

            if let Some(victim_idx) = retained.pop() {
                let victim = &convoy.items[victim_idx];
                shed.push(ShedItem {
                    label: victim.label.clone(),
                    priority: victim.priority,
                    original_bytes: victim.body.len(),
                });
            }
        }

        let mut apportioned = Vec::with_capacity(retained.len());
        for &idx in &retained {
            let item = &convoy.items[idx];
            let (body, truncated) =
                truncate_marked(&item.body, share, &self.config.truncation_marker);
            let allotted_bytes = body.len();
            apportioned.push(ApportionedItem {
                label: item.label.clone(),
                body,
                truncated,
                allotted_bytes,
            });
        }

        let rendered: String = apportioned.iter().map(|item| item.body.as_str()).collect();
        let assembled = format!("{fixed}{rendered}");
        let prompt_tokens = self.counter.count_tokens(&assembled)?;
        let exact_tally = self.counter.is_exact();

        Ok(Allotment {
            apportioned,
            shed,
            prompt_tokens,
            allotted_tokens,
            exact_tally,
        })
    }

    /// Pre-flight guard: tallies `assembled_prompt` through the configured
    /// [`TokenCounter`] and returns an error naming measured-vs-allowed when it would
    /// overflow the allowance. Never trims — this is the ADR-0004-shaped enforcement
    /// point; a caller that wants a bounded allotment instead should use
    /// [`Quartermaster::apportion`].
    ///
    /// # Errors
    ///
    /// - [`QuartermasterError::ContextOverflow`] if the measured tally exceeds
    ///   [`Quartermaster::allotted_tokens`].
    /// - [`QuartermasterError::Tally`] if the configured [`TokenCounter`] fails.
    pub fn verify_fits(&self, assembled_prompt: &str) -> Result<u32, QuartermasterError> {
        let allotted_tokens = self.allotted_tokens();
        let measured_tokens = self.counter.count_tokens(assembled_prompt)?;

        if measured_tokens > allotted_tokens {
            return Err(QuartermasterError::ContextOverflow {
                measured_tokens,
                allotted_tokens,
                provider: self.provider.clone(),
            });
        }

        Ok(measured_tokens)
    }
}

/// Char-boundary-safe truncation of `text` to `max_bytes`, appending `marker` when a cut
/// was needed. Mirrors `crates/audit-agents/src/deductive.rs::cap_bytes_marked`'s exact
/// boundary-walk shape — the trusted precedent already proven against multi-byte input.
///
/// Returns `(text, false)` unchanged when `text` already fits; returns `(truncated_text +
/// marker, true)` otherwise. Never panics: the walk always terminates at byte offset `0`,
/// which is always a valid char boundary, and the final slice is taken through the
/// checked `str::get` API rather than indexing syntax.
fn truncate_marked(text: &str, max_bytes: usize, marker: &str) -> (String, bool) {
    if text.len() <= max_bytes {
        return (text.to_string(), false);
    }

    let mut end = max_bytes;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }

    let head = text.get(..end).unwrap_or("");
    (format!("{head}{marker}"), true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_ports::output::token_counter_port::EstimatingTokenCounter;

    fn counter() -> Arc<dyn TokenCounter> {
        Arc::new(EstimatingTokenCounter::new("deepseek-chat"))
    }

    fn capabilities_with_window(window: Option<u32>) -> ProviderCapabilities {
        ProviderCapabilities {
            max_context_tokens: window,
            ..Default::default()
        }
    }

    fn qm(window: u32, config: AllotmentConfig) -> Quartermaster {
        Quartermaster::new(
            "deepseek",
            capabilities_with_window(Some(window)),
            counter(),
            config,
        )
        .unwrap()
    }

    #[test]
    fn a_fitting_muster_sheds_nothing_and_truncates_nothing() {
        let quartermaster = qm(10_000, AllotmentConfig::default());
        let mut convoy = Convoy::new();
        convoy.push(ConvoyItem {
            label: "a".to_string(),
            body: "short body a".to_string(),
            priority: 1,
        });
        convoy.push(ConvoyItem {
            label: "b".to_string(),
            body: "short body b".to_string(),
            priority: 2,
        });

        let allotment = quartermaster.apportion("", &convoy).unwrap();

        assert!(allotment.shed.is_empty());
        assert_eq!(allotment.apportioned.len(), 2);
        assert!(allotment.apportioned.iter().all(|item| !item.truncated));
    }

    #[test]
    fn an_over_budget_muster_sheds_the_lowest_priority_item_first() {
        // Small window -> small byte allowance, forcing a shed.
        let quartermaster = qm(100, AllotmentConfig::default());
        let mut convoy = Convoy::new();
        convoy.push(ConvoyItem {
            label: "high-priority".to_string(),
            body: "A".repeat(300),
            priority: 1, // lower number == higher priority
        });
        convoy.push(ConvoyItem {
            label: "low-priority".to_string(),
            body: "B".repeat(300),
            priority: 2, // higher number == lower priority == shed first
        });

        let allotment = quartermaster.apportion("", &convoy).unwrap();

        assert_eq!(allotment.shed.len(), 1);
        assert_eq!(allotment.shed[0].label, "low-priority");
        assert_eq!(allotment.apportioned.len(), 1);
        assert_eq!(allotment.apportioned[0].label, "high-priority");
    }

    #[test]
    fn every_shed_item_is_recorded_with_its_label_and_original_size() {
        let quartermaster = qm(100, AllotmentConfig::default());
        let mut convoy = Convoy::new();
        convoy.push(ConvoyItem {
            label: "high-priority".to_string(),
            body: "A".repeat(300),
            priority: 1,
        });
        convoy.push(ConvoyItem {
            label: "low-priority".to_string(),
            body: "B".repeat(300),
            priority: 2,
        });

        let allotment = quartermaster.apportion("", &convoy).unwrap();

        assert_eq!(allotment.shed.len(), 1);
        assert_eq!(allotment.shed[0].label, "low-priority");
        assert_eq!(allotment.shed[0].priority, 2);
        assert_eq!(allotment.shed[0].original_bytes, 300);
    }

    #[test]
    fn swapping_caller_priorities_changes_which_item_is_shed() {
        let quartermaster = qm(100, AllotmentConfig::default());
        let mut convoy = Convoy::new();
        // Same bodies as the prior tests, priorities swapped.
        convoy.push(ConvoyItem {
            label: "high-priority".to_string(),
            body: "A".repeat(300),
            priority: 2, // now lower priority
        });
        convoy.push(ConvoyItem {
            label: "low-priority".to_string(),
            body: "B".repeat(300),
            priority: 1, // now higher priority
        });

        let allotment = quartermaster.apportion("", &convoy).unwrap();

        assert_eq!(allotment.shed.len(), 1);
        assert_eq!(allotment.shed[0].label, "high-priority");
        assert_eq!(allotment.apportioned[0].label, "low-priority");
    }

    #[test]
    fn a_retained_item_over_its_share_is_truncated_and_marked() {
        let config = AllotmentConfig {
            per_item_max_bytes: 50,
            ..Default::default()
        };
        let quartermaster = qm(10_000, config);
        let mut convoy = Convoy::new();
        convoy.push(ConvoyItem {
            label: "only".to_string(),
            body: "x".repeat(500),
            priority: 1,
        });

        let allotment = quartermaster.apportion("", &convoy).unwrap();

        assert_eq!(allotment.apportioned.len(), 1);
        assert!(allotment.apportioned[0].truncated);
        assert!(allotment.apportioned[0].body.ends_with("\n... (truncated)"));
    }

    #[test]
    fn a_per_item_share_is_clamped_to_the_configured_maximum() {
        let config = AllotmentConfig {
            per_item_max_bytes: 10,
            ..Default::default()
        };
        // Huge window so budget/n would otherwise hand each item far more than 10 bytes.
        let quartermaster = qm(1_000_000, config);
        let mut convoy = Convoy::new();
        convoy.push(ConvoyItem {
            label: "a".to_string(),
            body: "x".repeat(5000),
            priority: 1,
        });
        convoy.push(ConvoyItem {
            label: "b".to_string(),
            body: "y".repeat(5000),
            priority: 2,
        });

        let allotment = quartermaster.apportion("", &convoy).unwrap();

        assert!(allotment.shed.is_empty());
        for item in &allotment.apportioned {
            assert!(item.allotted_bytes <= 10 + "\n... (truncated)".len());
        }
    }

    #[test]
    fn truncation_lands_on_a_char_boundary_for_multibyte_input() {
        let config = AllotmentConfig {
            per_item_max_bytes: 5, // small enough to force a cut mid multi-byte char
            ..Default::default()
        };
        let quartermaster = qm(10_000, config);
        let mut convoy = Convoy::new();
        convoy.push(ConvoyItem {
            label: "multibyte".to_string(),
            body: "你好世界👋🚀".to_string(),
            priority: 1,
        });

        // Must not panic, and the resulting body is a valid Rust String (UTF-8) by
        // construction — the truncation walk never lands off a char boundary.
        let allotment = quartermaster.apportion("", &convoy).unwrap();
        assert_eq!(allotment.apportioned.len(), 1);
        assert!(allotment.apportioned[0].truncated);
    }

    #[test]
    fn an_undeclared_window_with_no_fallback_is_an_error() {
        let result = Quartermaster::new(
            "mystery-provider",
            capabilities_with_window(None),
            counter(),
            AllotmentConfig::default(),
        );

        assert!(matches!(
            result,
            Err(QuartermasterError::UndeclaredContextWindow { .. })
        ));
    }

    #[test]
    fn an_undeclared_window_with_a_fallback_uses_the_fallback() {
        let config = AllotmentConfig {
            fallback_context_tokens: Some(2048),
            ..Default::default()
        };
        let quartermaster = Quartermaster::new(
            "mystery-provider",
            capabilities_with_window(None),
            counter(),
            config,
        )
        .unwrap();

        assert_eq!(quartermaster.allotted_tokens(), 2048);
    }

    #[test]
    fn a_reservation_larger_than_the_window_is_rejected_at_construction() {
        let config = AllotmentConfig {
            reserved_completion_tokens: 100,
            ..Default::default()
        };
        let result = Quartermaster::new(
            "deepseek",
            capabilities_with_window(Some(100)),
            counter(),
            config,
        );

        assert!(matches!(
            result,
            Err(QuartermasterError::ReservationExceedsWindow {
                reserved: 100,
                window: 100
            })
        ));
    }

    #[test]
    fn fixed_material_over_the_allowance_errors_instead_of_clamping() {
        // Tiny window -> tiny byte allowance.
        let quartermaster = qm(1, AllotmentConfig::default());
        let convoy = Convoy::new();
        let huge_fixed = "x".repeat(100_000);

        let result = quartermaster.apportion(&huge_fixed, &convoy);

        assert!(matches!(
            result,
            Err(QuartermasterError::FixedMaterialExceedsAllowance { .. })
        ));
    }

    #[test]
    fn verify_fits_reports_measured_and_allowed_on_overflow() {
        let quartermaster = qm(1, AllotmentConfig::default());
        let huge_prompt = "x".repeat(100_000);

        let result = quartermaster.verify_fits(&huge_prompt);

        match result {
            Err(QuartermasterError::ContextOverflow {
                measured_tokens,
                allotted_tokens,
                provider,
            }) => {
                assert!(measured_tokens > allotted_tokens);
                assert_eq!(provider, "deepseek");
            }
            other => panic!("expected ContextOverflow, got {other:?}"),
        }
    }

    #[test]
    fn verify_fits_returns_the_measured_tally_when_it_fits() {
        let quartermaster = qm(10_000, AllotmentConfig::default());
        let result = quartermaster.verify_fits("a short prompt");

        assert!(result.is_ok());
        assert!(result.unwrap() > 0);
    }

    #[cfg(feature = "mock")]
    #[test]
    fn the_window_comes_from_the_ports_declared_capabilities() {
        use crate::mock::MockLlmAdapter;

        let mock_adapter = MockLlmAdapter::new();
        let mock_quartermaster =
            Quartermaster::from_port(&mock_adapter, counter(), AllotmentConfig::default()).unwrap();

        let big_quartermaster = Quartermaster::new(
            "big-provider",
            capabilities_with_window(Some(64_000)),
            counter(),
            AllotmentConfig::default(),
        )
        .unwrap();

        // MockLlmAdapter declares max_context_tokens: Some(4096) — the window is read
        // from the port, not hardcoded, so the two allowances differ with no source
        // change other than which port was consulted.
        assert!(mock_quartermaster.allotted_tokens() < big_quartermaster.allotted_tokens());
    }

    #[test]
    fn an_empty_muster_provisions_zero_items() {
        let quartermaster = qm(10_000, AllotmentConfig::default());
        let convoy = Convoy::new();

        let allotment = quartermaster.apportion("fixed material", &convoy).unwrap();

        assert!(allotment.apportioned.is_empty());
        assert!(allotment.shed.is_empty());
    }
}
