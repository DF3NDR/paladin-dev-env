//! Token-tally contract — the measurement primitive the `Quartermaster` budgets against.
//!
//! This module defines the CONTRACT for turning a piece of text into a token count. It
//! does not itself embed any tokenizer: concrete, tokenizer-backed implementations
//! (e.g. `TiktokenCounter`) live in adapter crates such as `paladin-memory`, which depend
//! on this port rather than the other way around. That split exists so a caller in
//! `paladin-llm` (the `Quartermaster`) can measure a prompt against a provider's declared
//! context window without pulling in `paladin-memory` or an optional tokenizer dependency
//! (`tiktoken-rs`) it does not otherwise need.
//!
//! Not every model this framework talks to has an exact tokenizer available offline
//! (`claude-*`, `deepseek-*`). For those, [`EstimatingTokenCounter`] provides a
//! documented, deliberately OVER-counting estimate rather than failing or silently
//! reporting a number that looks exact. [`TokenCounter::is_exact`] is how a caller tells
//! the two kinds apart.

use thiserror::Error;

/// The reciprocal of the most pessimistic measured bytes-per-token ratio for models with
/// no exact tokenizer available offline (2.8 bytes/token), rounded so the resulting
/// estimate always errs toward OVER-counting tokens, never under.
///
/// Provenance: measured during debug session `deductive-32000-zero-output`. No
/// `claude-*`/`deepseek-*` tokenizer was available offline at measurement time
/// (`tiktoken`/`transformers` absent, no `pip`), so a RANGE (2.8-5.0 bytes/token) was
/// measured instead of a single point estimate. This constant deliberately pins the
/// pessimistic end of that range (`1000 / 2.8 ≈ 357.1`, rounded up to `358`) so every
/// downstream budget derived from it stays conservative.
///
/// Anything derived from this constant is an ESTIMATE, not an exact count. A guard built
/// on it must leave margin accordingly — see [`TokenCounter::is_exact`].
pub const PESSIMISTIC_TOKENS_PER_1000_BYTES: u32 = 358;

/// Errors a [`TokenCounter`] implementation can return.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TokenCountError {
    /// The requested model has no exact tokenizer available (e.g. an unrecognized
    /// OpenAI-family model name passed to a tiktoken-backed counter). Recovery: fall back
    /// to [`EstimatingTokenCounter`] for this model, or supply a model name the concrete
    /// counter recognizes.
    #[error("no exact tokenizer available for model '{0}'")]
    UnsupportedModel(String),

    /// The underlying tokenizer failed on otherwise well-formed input. Recovery: this is
    /// usually a bug in the input or the tokenizer library; retrying with the same input
    /// will not help.
    #[error("tokenization failed: {0}")]
    Tokenization(String),

    /// An [`EstimatingTokenCounter`] was constructed (or reconfigured) with a
    /// tokens-per-1000-bytes ratio of zero, which would make the estimate arithmetic
    /// divide by zero. Recovery: supply a non-zero ratio, or use
    /// [`EstimatingTokenCounter::new`] for the documented pessimistic default.
    #[error("invalid token-count ratio: {0}")]
    InvalidRatio(String),
}

/// Contract for turning a piece of text into a token count.
///
/// A [`TokenCounter`] implementation may be EXACT (backed by a real tokenizer for the
/// model it names) or an ESTIMATE (a documented, deliberately over-counting
/// approximation for a model with no offline tokenizer). [`is_exact`](Self::is_exact)
/// tells a caller which kind it has, so a caller budgeting against an estimate can leave
/// margin instead of mistaking the number for an exact measurement.
pub trait TokenCounter: Send + Sync {
    /// Counts the number of tokens in `text`.
    ///
    /// The returned number may be exact (a real tokenizer ran) or an estimate (a
    /// documented over-counting approximation) — see [`is_exact`](Self::is_exact) to tell
    /// which.
    ///
    /// # Errors
    ///
    /// Returns a [`TokenCountError`] if tokenization fails or the counter cannot measure
    /// the configured model. Never panics: implementations must use checked/saturating
    /// arithmetic so a pathologically large input cannot overflow.
    fn count_tokens(&self, text: &str) -> Result<u32, TokenCountError>;

    /// Returns the model name this counter is configured for.
    fn model_name(&self) -> &str;

    /// Whether [`count_tokens`](Self::count_tokens) returns an EXACT tally (`true`, the
    /// default) or a deliberately over-counting ESTIMATE (`false`).
    ///
    /// Defaulted `true` so an existing tokenizer-backed implementation (which is exact by
    /// construction) needs no change to adopt this trait. A caller that budgets against a
    /// non-exact tally must leave margin — an estimate is used to gate a real request, and
    /// presenting it as exact would be a false assurance.
    fn is_exact(&self) -> bool {
        true
    }
}

/// A [`TokenCounter`] for models with no exact tokenizer available offline (`claude-*`,
/// `deepseek-*`), backed by a documented, deliberately OVER-counting byte-ratio estimate.
///
/// Never errors on a well-formed string (empty string tallies to `0`), never returns an
/// exact-looking number for a model this counter cannot truly tokenize, and never panics
/// — a pathologically large input saturates at `u32::MAX` rather than overflowing.
///
/// # Examples
///
/// ```
/// use paladin_ports::output::token_counter_port::{EstimatingTokenCounter, TokenCounter};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let counter = EstimatingTokenCounter::new("deepseek-chat");
/// let count = counter.count_tokens("hello, world")?;
/// assert!(count > 0);
/// assert!(!counter.is_exact());
/// # Ok(())
/// # }
/// ```
pub struct EstimatingTokenCounter {
    model_name: String,
    tokens_per_1000_bytes: u32,
}

impl EstimatingTokenCounter {
    /// Creates an estimator for `model_name` using the documented pessimistic default
    /// ratio ([`PESSIMISTIC_TOKENS_PER_1000_BYTES`]).
    pub fn new(model_name: impl Into<String>) -> Self {
        Self {
            model_name: model_name.into(),
            tokens_per_1000_bytes: PESSIMISTIC_TOKENS_PER_1000_BYTES,
        }
    }

    /// Creates an estimator for `model_name` using a caller-supplied
    /// tokens-per-1000-bytes ratio.
    ///
    /// # Errors
    ///
    /// Returns [`TokenCountError::InvalidRatio`] if `tokens_per_1000_bytes` is zero —
    /// this is the division-hazard guard; the project forbids `panic!` in library code, so
    /// the zero case is an `Err`, never a runtime divide.
    pub fn with_ratio(
        model_name: impl Into<String>,
        tokens_per_1000_bytes: u32,
    ) -> Result<Self, TokenCountError> {
        if tokens_per_1000_bytes == 0 {
            return Err(TokenCountError::InvalidRatio(
                "tokens_per_1000_bytes must be non-zero".to_string(),
            ));
        }
        Ok(Self {
            model_name: model_name.into(),
            tokens_per_1000_bytes,
        })
    }
}

impl TokenCounter for EstimatingTokenCounter {
    fn count_tokens(&self, text: &str) -> Result<u32, TokenCountError> {
        let bytes = text.len() as u64;
        let scaled = bytes.saturating_mul(u64::from(self.tokens_per_1000_bytes));
        let tokens = scaled / 1000;
        Ok(u32::try_from(tokens).unwrap_or(u32::MAX))
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }

    fn is_exact(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimator_counts_a_claude_model_without_erroring() {
        let counter = EstimatingTokenCounter::new("claude-opus-4-8");
        let count = counter.count_tokens("some prompt text").unwrap();
        assert!(count > 0);
    }

    #[test]
    fn estimator_counts_a_deepseek_model_without_erroring() {
        let counter = EstimatingTokenCounter::new("deepseek-chat");
        let count = counter.count_tokens("some prompt text").unwrap();
        assert!(count > 0);
    }

    #[test]
    fn estimator_over_counts_against_the_optimistic_end_of_the_measured_ratio_range() {
        let counter = EstimatingTokenCounter::new("deepseek-chat");
        let text = "a".repeat(1000);
        let count = counter.count_tokens(&text).unwrap();
        // 1000 bytes at the pessimistic 358 tokens/1000-bytes ratio == 358.
        assert_eq!(count, 358);
        // The optimistic end of the measured 2.8-5.0 bytes/token range is 5.0
        // bytes/token == 200 tokens for 1000 bytes. The estimate must strictly
        // over-count relative to that optimistic figure.
        assert!(count > 200);
    }

    #[test]
    fn estimator_reports_itself_as_not_exact() {
        let counter = EstimatingTokenCounter::new("claude-opus-4-8");
        assert!(!counter.is_exact());
    }

    #[test]
    fn a_zero_ratio_is_rejected_rather_than_dividing() {
        let result = EstimatingTokenCounter::with_ratio("deepseek-chat", 0);
        assert!(matches!(result, Err(TokenCountError::InvalidRatio(_))));
    }

    #[test]
    fn an_empty_string_tallies_to_zero() {
        let counter = EstimatingTokenCounter::new("deepseek-chat");
        let count = counter.count_tokens("").unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn a_pathologically_long_input_saturates_without_panicking() {
        // Use the maximum ratio (u32::MAX) against a large-but-allocatable input to
        // exercise the saturating multiply without needing a usize::MAX allocation.
        let counter = EstimatingTokenCounter::with_ratio("deepseek-chat", u32::MAX).unwrap();
        let text = "a".repeat(10_000_000);
        let count = counter.count_tokens(&text).unwrap();
        assert_eq!(count, u32::MAX);
    }
}
