//! The Commissary — the officer who issues rations under scarcity.
//!
//! `ProviderCapabilities::max_context_tokens` (`paladin_ports::output::llm_port`) is
//! declared by every LLM adapter shipped in this crate, yet before this module existed
//! [`TokenCounterPort`] (`paladin_ports::output::token_counter_port`) — the infallible,
//! v0.10.0-native counting seam — had zero production readers. This module applies the
//! same fail-loud stance `PaladinBuilder` already takes on the neighbouring
//! `temperature_range` field (ADR-0004: error rather than silently clamp) to the INPUT
//! side of a call, the assembled prompt, rather than the output-side temperature.
//!
//! Two responsibilities, kept deliberately separate:
//!
//! - [`Commissary::verify_fits`] — a pre-flight GUARD. It measures an already-assembled
//!   prompt against the provider's declared window (minus the caller's reserved
//!   completion budget) and returns an error naming the measured tokens, the allowance,
//!   and the provider when it would overflow. It never trims.
//! - [`Commissary::dispense`] — a bounded ALLOCATOR. Given FIXED (non-sheddable)
//!   material and a [`Consignment`] of caller-prioritised, shed-or-truncate-able
//!   material, it returns a [`Stockpile`]: every retained item clamped to a per-item
//!   share (with a visible truncation marker when a cut was needed) and every shed item
//!   recorded with its label, priority, and original size. Nothing is dropped silently
//!   — the explicit anti-pattern this module rejects is
//!   `paladin_memory::services::rag_retrieval_service::RagRetrievalService::
//!   truncate_to_token_budget`, which drops lowest-scoring items with no marker and no
//!   record.
//!
//! **Framework owns measurement and enforcement; callers own policy.** The Commissary
//! never decides WHICH material matters more — that is the caller-supplied `priority`
//! on each [`ConsignmentItem`]. No audit-specific (or any other application-specific)
//! policy crosses into this crate.
//!
//! **Honesty clause.** Not every model has an exact tokenizer available offline
//! (`claude-*`, `deepseek-*`). [`TokenCounterPort::count`] is infallible and carries no
//! built-in exactness signal (unlike the removed, fallible `TokenCounter::is_exact`), so
//! the caller supplies `is_exact_counter` explicitly at construction — it already knows
//! which concrete counter it injected (e.g. `HeuristicTokenCounter` → `false`,
//! `TiktokenCounter` → `true`). That flag is threaded straight into
//! [`Stockpile::exact_tally`] so a caller reading a `Commissary`-produced stockpile can
//! tell an exact tally from an estimate and budget its own margin accordingly.

use std::sync::Arc;

use paladin_ports::output::llm_port::{LlmPort, ProviderCapabilities};
use paladin_ports::output::token_counter_port::TokenCounterPort;
use thiserror::Error;

/// Planning-only tokens-per-1000-bytes ratio used ONLY internally by
/// [`Commissary::dispense`] to decide how many bytes each item may keep while
/// dispensing (deciding byte shares, not the final tally). The final tally reported on
/// the returned [`Stockpile`] always goes through the injected [`TokenCounterPort`],
/// never this ratio.
///
/// Not part of [`TokenCounterPort`]'s public contract — v0.10.0 does not ship a public
/// equivalent of the old, removed `PESSIMISTIC_TOKENS_PER_1000_BYTES` port constant, so
/// it is recreated here as a private module constant, carrying over its documented
/// pessimistic default value unchanged.
const PESSIMISTIC_TOKENS_PER_1000_BYTES: u32 = 358;

/// Configuration governing how a [`Commissary`] resolves its allowance and dispenses
/// material into a bounded [`Stockpile`].
#[derive(Debug, Clone)]
pub struct CommissaryPlan {
    /// How many tokens of the provider's declared window the CALLER intends to spend on
    /// the completion.
    ///
    /// This must be the caller's LARGEST completion budget (its escalated retry, not
    /// its base), because the escalated attempt is the one that must still fit inside
    /// the declared window alongside the prompt.
    pub reserved_completion_tokens: u32,

    /// Used ONLY when the provider's [`ProviderCapabilities::max_context_tokens`] is
    /// `None`. This is caller policy: the framework will not invent a window on its
    /// own.
    pub fallback_context_tokens: Option<u32>,

    /// The minimum byte share a single retained [`ConsignmentItem`] may be allotted.
    pub per_item_min_bytes: usize,

    /// The maximum byte share a single retained [`ConsignmentItem`] may be allotted,
    /// even when the available budget divided across the retained set would otherwise
    /// hand it more.
    pub per_item_max_bytes: usize,

    /// The tokens-per-1000-bytes ratio used ONLY for byte-share PLANNING while
    /// dispensing (deciding how many bytes each item may keep). The final tally
    /// reported on the returned [`Stockpile`] always goes through the configured
    /// [`TokenCounterPort`], never this ratio. Defaults to
    /// [`PESSIMISTIC_TOKENS_PER_1000_BYTES`]. Zero is rejected at
    /// [`Commissary::new`] construction time — the division-hazard guard.
    pub pessimistic_tokens_per_1000_bytes: u32,

    /// Appended to a retained item's body when it had to be shortened to fit its share,
    /// so a truncated excerpt is never mistaken for a complete one.
    pub truncation_marker: String,

    /// The model identifier passed to [`TokenCounterPort::count`] alongside every
    /// measured text. Neither shipped adapter (`HeuristicTokenCounter`,
    /// `TiktokenCounter`) actually consults this value at count-time — both document
    /// that they ignore or already-resolved it — but it exists so a FUTURE counter
    /// adapter that does branch on `model` gets a real one, and so a caller reading a
    /// `Commissary`'s own `Debug` output has an honest answer to "which model was this
    /// budget measured for."
    pub model_hint: String,
}

impl Default for CommissaryPlan {
    fn default() -> Self {
        Self {
            reserved_completion_tokens: 0,
            fallback_context_tokens: None,
            per_item_min_bytes: 0,
            per_item_max_bytes: usize::MAX,
            pessimistic_tokens_per_1000_bytes: PESSIMISTIC_TOKENS_PER_1000_BYTES,
            truncation_marker: "\n... (truncated)".to_string(),
            model_hint: String::new(),
        }
    }
}

/// A single labelled piece of material a caller wants considered for a [`Consignment`].
///
/// # Priority contract
///
/// `priority` is unambiguous: a LOWER number is HIGHER priority and is shed LAST. Ties
/// keep insertion order (the sort [`Commissary::dispense`] performs is stable).
#[derive(Debug, Clone)]
pub struct ConsignmentItem {
    /// A human-readable label identifying this item (surfaced on [`ShedItem`] and
    /// [`DispensedItem`] so a caller can tell which material was affected).
    pub label: String,
    /// The material itself.
    pub body: String,
    /// Lower number == higher priority == shed LAST. Ties keep insertion order.
    pub priority: u8,
}

/// An ordered collection of [`ConsignmentItem`]s awaiting dispensing. Carries no
/// shedding or truncation policy of its own — that lives entirely in
/// [`Commissary::dispense`], driven by each item's caller-supplied `priority`.
#[derive(Debug, Clone, Default)]
pub struct Consignment {
    items: Vec<ConsignmentItem>,
}

impl Consignment {
    /// Creates an empty consignment.
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Appends an item to the consignment, in insertion order.
    pub fn push(&mut self, item: ConsignmentItem) {
        self.items.push(item);
    }

    /// The number of items currently in the consignment.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the consignment currently holds no items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// A [`ConsignmentItem`] that survived dispensing, possibly truncated to fit its share.
#[derive(Debug, Clone)]
pub struct DispensedItem {
    /// Copied from the source [`ConsignmentItem::label`].
    pub label: String,
    /// The (possibly truncated, possibly marker-suffixed) body actually retained.
    pub body: String,
    /// Whether this item's body had to be shortened to fit its allotted share.
    pub truncated: bool,
    /// The final byte length of `body` (including the truncation marker, if any).
    pub allotted_bytes: usize,
}

/// A [`ConsignmentItem`] that did NOT survive dispensing — recorded so nothing is
/// dropped silently.
#[derive(Debug, Clone)]
pub struct ShedItem {
    /// Copied from the source [`ConsignmentItem::label`].
    pub label: String,
    /// Copied from the source [`ConsignmentItem::priority`].
    pub priority: u8,
    /// The byte length of the item's body BEFORE it was shed (never truncated — a shed
    /// item is dropped whole, not partially kept).
    pub original_bytes: usize,
}

/// The result of [`Commissary::dispense`]: every retained item (possibly truncated and
/// marked), every shed item (recorded, never silently dropped), and the final measured
/// tally.
#[derive(Debug, Clone)]
pub struct Stockpile {
    /// Items that survived dispensing, in the order they were retained.
    pub dispensed: Vec<DispensedItem>,
    /// Items that were shed to stay within budget, in the order they were shed.
    pub shed: Vec<ShedItem>,
    /// The final measured token tally of the fixed material plus every dispensed
    /// item's (possibly truncated) body.
    pub prompt_tokens: u32,
    /// The token allowance this stockpile was dispensed against
    /// ([`Commissary::allotted_tokens`] at the time of dispensing).
    pub allotted_tokens: u32,
    /// Threaded from the `is_exact_counter` argument supplied at
    /// [`Commissary::new`]/[`Commissary::from_port`] construction time: `true` if
    /// `prompt_tokens` is an exact tally, `false` if it is a deliberately over-counting
    /// estimate. A caller treating an estimate as exact would be over-trusting the
    /// guard — this field is how it avoids that mistake.
    pub exact_tally: bool,
}

impl Stockpile {
    /// Renders the dispensed bodies, in retained order, concatenated into a single
    /// string ready for prompt assembly.
    pub fn render(&self) -> String {
        self.dispensed
            .iter()
            .map(|item| item.body.as_str())
            .collect()
    }
}

/// Errors a [`Commissary`] can return. Every variant names its own numbers — a caller
/// reading only the `Display` string can see measured-vs-allowed without inspecting the
/// struct.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum CommissaryError {
    /// The provider declared no `max_context_tokens` and no
    /// [`CommissaryPlan::fallback_context_tokens`] was configured. The framework
    /// refuses to guess a window — supplying a fallback is explicit caller policy.
    #[error(
        "provider '{provider}' declared no max_context_tokens and no fallback_context_tokens \
         was configured"
    )]
    UndeclaredContextWindow {
        /// The provider whose declared capabilities were consulted.
        provider: String,
    },

    /// [`CommissaryPlan::reserved_completion_tokens`] is greater than or equal to the
    /// resolved context window, leaving zero or negative room for a prompt. Returned at
    /// [`Commissary::new`] construction time, before any dispensing work.
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

    /// The FIXED (non-sheddable) material passed to [`Commissary::dispense`] alone
    /// exceeds the byte allowance. This is an error, never a clamp (ADR-0004 stance
    /// applied to the input side): fixed material is, by definition, not something the
    /// Commissary is permitted to shorten.
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

    /// [`Commissary::verify_fits`] measured a prompt that exceeds the allowance. Never
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

    /// The [`CommissaryPlan`] passed to [`Commissary::new`] is internally invalid (e.g.
    /// a zero ratio, or `per_item_min_bytes > per_item_max_bytes`).
    #[error("invalid Commissary configuration: {0}")]
    InvalidConfig(String),
}

/// Measures an assembled prompt against a provider's OWN declared context window,
/// enforces it pre-flight, and dispenses a bounded stockpile under caller-supplied
/// priority.
///
/// See the [module-level documentation](self) for the framework/caller responsibility
/// split.
pub struct Commissary {
    counter: Arc<dyn TokenCounterPort>,
    capabilities: ProviderCapabilities,
    provider: String,
    config: CommissaryPlan,
    is_exact_counter: bool,
}

impl std::fmt::Debug for Commissary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Commissary")
            .field("provider", &self.provider)
            .field("capabilities", &self.capabilities)
            .field("config", &self.config)
            .field("is_exact_counter", &self.is_exact_counter)
            .finish()
    }
}

impl Commissary {
    /// Constructs a `Commissary` for `provider` from an already-obtained
    /// [`ProviderCapabilities`] and [`TokenCounterPort`].
    ///
    /// `is_exact_counter` is caller-supplied because the new, infallible
    /// [`TokenCounterPort`] contract carries no built-in exactness signal (unlike the
    /// removed, fallible `TokenCounter::is_exact`) — the caller already knows which
    /// concrete counter it is injecting (e.g. `HeuristicTokenCounter` → `false`,
    /// `TiktokenCounter` → `true`).
    ///
    /// # Errors
    ///
    /// - [`CommissaryError::InvalidConfig`] if `config.pessimistic_tokens_per_1000_bytes`
    ///   is zero, or if `config.per_item_min_bytes > config.per_item_max_bytes`.
    /// - [`CommissaryError::UndeclaredContextWindow`] if `capabilities.max_context_tokens`
    ///   is `None` and `config.fallback_context_tokens` is also `None`.
    /// - [`CommissaryError::ReservationExceedsWindow`] if
    ///   `config.reserved_completion_tokens` is greater than or equal to the resolved
    ///   window.
    pub fn new(
        provider: impl Into<String>,
        capabilities: ProviderCapabilities,
        counter: Arc<dyn TokenCounterPort>,
        is_exact_counter: bool,
        config: CommissaryPlan,
    ) -> Result<Self, CommissaryError> {
        let provider = provider.into();

        if config.pessimistic_tokens_per_1000_bytes == 0 {
            return Err(CommissaryError::InvalidConfig(
                "pessimistic_tokens_per_1000_bytes must be non-zero".to_string(),
            ));
        }
        if config.per_item_min_bytes > config.per_item_max_bytes {
            return Err(CommissaryError::InvalidConfig(format!(
                "per_item_min_bytes ({}) must be <= per_item_max_bytes ({})",
                config.per_item_min_bytes, config.per_item_max_bytes
            )));
        }

        let window = capabilities
            .max_context_tokens
            .or(config.fallback_context_tokens)
            .ok_or_else(|| CommissaryError::UndeclaredContextWindow {
                provider: provider.clone(),
            })?;

        if config.reserved_completion_tokens >= window {
            return Err(CommissaryError::ReservationExceedsWindow {
                reserved: config.reserved_completion_tokens,
                window,
            });
        }

        Ok(Self {
            counter,
            capabilities,
            provider,
            config,
            is_exact_counter,
        })
    }

    /// Constructs a `Commissary` by reading `llm`'s own declared provider name and
    /// capabilities — the method that gives `max_context_tokens` a real production
    /// reader.
    ///
    /// # Errors
    ///
    /// See [`Commissary::new`].
    pub fn from_port(
        llm: &dyn LlmPort,
        counter: Arc<dyn TokenCounterPort>,
        is_exact_counter: bool,
        config: CommissaryPlan,
    ) -> Result<Self, CommissaryError> {
        Self::new(
            llm.get_provider_name().to_string(),
            llm.get_capabilities(),
            counter,
            is_exact_counter,
            config,
        )
    }

    /// Resolves the context window: the provider's declared `max_context_tokens`, or
    /// [`CommissaryPlan::fallback_context_tokens`] when the provider declared none.
    /// Guaranteed to resolve to a value by the constructor's
    /// [`CommissaryError::UndeclaredContextWindow`] guard.
    fn window(&self) -> u32 {
        self.capabilities
            .max_context_tokens
            .or(self.config.fallback_context_tokens)
            .unwrap_or(0)
    }

    /// The token allowance available for a prompt: the resolved window minus
    /// [`CommissaryPlan::reserved_completion_tokens`], saturating at zero.
    pub fn allotted_tokens(&self) -> u32 {
        self.window()
            .saturating_sub(self.config.reserved_completion_tokens)
    }

    /// Dispenses `fixed` (non-sheddable material) plus `consignment` (caller-prioritised,
    /// shed-or-truncate-able material) into a bounded [`Stockpile`].
    ///
    /// Algorithm, deterministic:
    ///
    /// 1. `allowance_bytes = allotted_tokens * 1000 / pessimistic_tokens_per_1000_bytes`
    ///    (the ratio is non-zero by construction — see [`Commissary::new`] — so this
    ///    division is provably safe without an `expect`).
    /// 2. If `fixed.len() >= allowance_bytes`, return
    ///    [`CommissaryError::FixedMaterialExceedsAllowance`] — an error, never a clamp.
    /// 3. `budget = allowance_bytes - fixed.len()`.
    /// 4. Stable-sort consignment items by `(priority, insertion_index)`.
    /// 5. With `n` retained items, `share = (budget / n).clamp(min, max)`; each item's
    ///    provisional allotment is `min(item.body.len(), share)` plus the marker length
    ///    for any item that would be cut. If the provisional total exceeds `budget`, the
    ///    LAST item in sorted order (lowest priority) is shed and the step repeats. `n ==
    ///    0` is guarded before any division.
    /// 6. Each retained item is truncated to its final share with a char-boundary-safe
    ///    walk, with the truncation marker appended when a cut was needed.
    /// 7. The final `fixed + rendered` text is tallied through the configured
    ///    [`TokenCounterPort`] for `prompt_tokens`; `is_exact_counter` (supplied at
    ///    construction) is carried into `exact_tally`.
    /// 8. The stockpile is returned with `shed` populated in shed order.
    ///
    /// # Errors
    ///
    /// - [`CommissaryError::FixedMaterialExceedsAllowance`] if `fixed` alone meets or
    ///   exceeds the byte allowance.
    pub fn dispense(
        &self,
        fixed: &str,
        consignment: &Consignment,
    ) -> Result<Stockpile, CommissaryError> {
        let allotted_tokens = self.allotted_tokens();
        let ratio = self.config.pessimistic_tokens_per_1000_bytes;

        // Safe: `ratio` is non-zero by the `Commissary::new` constructor guard.
        let allowance_bytes: usize =
            usize::try_from(u64::from(allotted_tokens).saturating_mul(1000) / u64::from(ratio))
                .unwrap_or(usize::MAX);

        if fixed.len() >= allowance_bytes {
            let fixed_tokens = self.counter.count(fixed, &self.config.model_hint);
            return Err(CommissaryError::FixedMaterialExceedsAllowance {
                fixed_tokens,
                allotted_tokens,
            });
        }

        let budget = allowance_bytes - fixed.len();

        // Stable-sort by (priority, insertion order): ascending priority number puts
        // the HIGHEST-priority items (lowest number) first. Popping from the end
        // therefore sheds the LOWEST-priority item first, per the documented priority
        // contract.
        let mut retained: Vec<usize> = (0..consignment.items.len()).collect();
        retained.sort_by_key(|&idx| consignment.items[idx].priority);

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
                let item = &consignment.items[idx];
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
                let victim = &consignment.items[victim_idx];
                shed.push(ShedItem {
                    label: victim.label.clone(),
                    priority: victim.priority,
                    original_bytes: victim.body.len(),
                });
            }
        }

        let mut dispensed = Vec::with_capacity(retained.len());
        for &idx in &retained {
            let item = &consignment.items[idx];
            let (body, truncated) =
                truncate_marked(&item.body, share, &self.config.truncation_marker);
            let allotted_bytes = body.len();
            dispensed.push(DispensedItem {
                label: item.label.clone(),
                body,
                truncated,
                allotted_bytes,
            });
        }

        let rendered: String = dispensed.iter().map(|item| item.body.as_str()).collect();
        let assembled = format!("{fixed}{rendered}");
        let prompt_tokens = self.counter.count(&assembled, &self.config.model_hint);

        Ok(Stockpile {
            dispensed,
            shed,
            prompt_tokens,
            allotted_tokens,
            exact_tally: self.is_exact_counter,
        })
    }

    /// Pre-flight guard: tallies `assembled_prompt` through the configured
    /// [`TokenCounterPort`] and returns an error naming measured-vs-allowed when it
    /// would overflow the allowance. Never trims — this is the ADR-0004-shaped
    /// enforcement point; a caller that wants a bounded stockpile instead should use
    /// [`Commissary::dispense`].
    ///
    /// # Errors
    ///
    /// - [`CommissaryError::ContextOverflow`] if the measured tally exceeds
    ///   [`Commissary::allotted_tokens`].
    pub fn verify_fits(&self, assembled_prompt: &str) -> Result<u32, CommissaryError> {
        let allotted_tokens = self.allotted_tokens();
        let measured_tokens = self.counter.count(assembled_prompt, &self.config.model_hint);

        if measured_tokens > allotted_tokens {
            return Err(CommissaryError::ContextOverflow {
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
