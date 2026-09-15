//! Token Counting Utilities
//!
//! Provides token counting capabilities for different LLM providers.
//! [`TiktokenCounter`] is the crate's exact, BPE-based `TokenCounterPort` adapter.

use paladin_ports::output::garrison_port::GarrisonError;
use paladin_ports::output::token_counter_port::TokenCounterPort;
use std::collections::HashMap;
use std::sync::RwLock;
use tiktoken_rs::{CoreBPE, get_bpe_from_model};

/// Token counter using tiktoken for OpenAI models.
///
/// Supports various OpenAI models including GPT-3.5, GPT-4, and their variants.
/// Results are cached per unique input string to amortise repeated tokenisation
/// of the same text (e.g., system prompts included in every request).
///
/// # Supported models
///
/// Any model name accepted by `tiktoken_rs::get_bpe_from_model`, including
/// `"gpt-4"`, `"gpt-4o"`, `"gpt-3.5-turbo"`, and embedding models.
///
/// # Examples
///
/// ```no_run
/// use paladin_memory::garrison::TiktokenCounter;
/// use paladin_ports::output::token_counter_port::TokenCounterPort;
///
/// let counter = TiktokenCounter::new("gpt-4").unwrap();
/// let count = counter.count("Hello, world!", "gpt-4");
/// assert!(count > 0);
/// ```
pub struct TiktokenCounter {
    bpe: CoreBPE,
    model_name: String,
    cache: RwLock<HashMap<String, u32>>,
}

impl TiktokenCounter {
    /// Creates a new tiktoken counter for the specified model.
    ///
    /// # Arguments
    ///
    /// * `model_name` - The name of the OpenAI model (e.g., `"gpt-4"`, `"gpt-3.5-turbo"`)
    ///
    /// # Errors
    ///
    /// Returns [`GarrisonError::TokenizationError`] if the model name is not
    /// supported by `tiktoken_rs`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use paladin_memory::garrison::TiktokenCounter;
    ///
    /// let counter = TiktokenCounter::new("gpt-4").unwrap();
    /// ```
    pub fn new(model_name: &str) -> Result<Self, GarrisonError> {
        let bpe = get_bpe_from_model(model_name).map_err(|e| {
            GarrisonError::TokenizationError(format!(
                "Failed to initialize tokenizer for model '{}': {}",
                model_name, e
            ))
        })?;

        Ok(Self {
            bpe,
            model_name: model_name.to_string(),
            cache: RwLock::new(HashMap::new()),
        })
    }

    /// Returns the model name this counter was constructed for -- the name
    /// the encoding was resolved for at [`TiktokenCounter::new`].
    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    /// Clears the token count cache.
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
    }

    /// Returns the current number of cached token-count entries.
    pub fn cache_size(&self) -> usize {
        self.cache.read().map(|c| c.len()).unwrap_or(0)
    }
}

/// `impl TokenCounterPort for TiktokenCounter` (Doc 05 RT-FR-10, D-13).
///
/// `count` deliberately does **not** re-resolve `model` through
/// `get_bpe_from_model` -- the encoding this instance uses was already
/// resolved, fallibly, at [`TiktokenCounter::new`]. `count` delegates to that
/// already-loaded encoding regardless of what `model` is passed here, which
/// is what makes the port method infallible: an unrecognised `model` string
/// at count-time is never looked up, so it can never produce the
/// `TokenizationError` [`TiktokenCounter::new`] can.
impl TokenCounterPort for TiktokenCounter {
    fn count(&self, text: &str, _model: &str) -> u32 {
        // Check cache first.
        if let Ok(cache) = self.cache.read()
            && let Some(&count) = cache.get(text)
        {
            return count;
        }

        // Count tokens via the already-loaded encoding.
        let tokens = self.bpe.encode_with_special_tokens(text);
        let count = tokens.len() as u32;

        // Cache the result.
        if let Ok(mut cache) = self.cache.write() {
            // Limit cache size to prevent unbounded growth.
            if cache.len() >= 1000 {
                cache.clear();
            }
            cache.insert(text.to_string(), count);
        }

        count
    }

    fn name(&self) -> &str {
        "tiktoken"
    }

    /// Exact for the encoding resolved at [`TiktokenCounter::new`] -- `count`
    /// delegates to that already-loaded encoding and never re-resolves its
    /// own `model` argument, so this is never a claim about arbitrary model
    /// strings passed to `count`.
    fn is_exact(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tiktoken_counter_creation() {
        let counter = TiktokenCounter::new("gpt-4").unwrap();
        assert_eq!(counter.model_name(), "gpt-4");
    }

    #[test]
    fn test_tiktoken_counter_unsupported_model() {
        let result = TiktokenCounter::new("unsupported-model-xyz");
        assert!(result.is_err());
    }

    #[test]
    fn test_count_simple() {
        let counter = TiktokenCounter::new("gpt-4").unwrap();
        let via_port: &dyn TokenCounterPort = &counter;
        let count = via_port.count("Hello, world!", "gpt-4");
        assert!(count > 0);
        assert!(count < 10); // "Hello, world!" should be just a few tokens
    }

    #[test]
    fn test_count_empty_string() {
        let counter = TiktokenCounter::new("gpt-4").unwrap();
        let via_port: &dyn TokenCounterPort = &counter;
        let count = via_port.count("", "gpt-4");
        assert_eq!(count, 0);
    }

    #[test]
    fn test_count_caching() {
        let counter = TiktokenCounter::new("gpt-4").unwrap();
        let via_port: &dyn TokenCounterPort = &counter;

        // First call - not cached
        let text = "This is a test message for caching.";
        let count1 = via_port.count(text, "gpt-4");

        // Second call - should be cached
        let count2 = via_port.count(text, "gpt-4");

        assert_eq!(count1, count2);
        assert_eq!(counter.cache_size(), 1);
    }

    #[test]
    fn test_cache_clearing() {
        let counter = TiktokenCounter::new("gpt-4").unwrap();
        let via_port: &dyn TokenCounterPort = &counter;

        via_port.count("Test 1", "gpt-4");
        via_port.count("Test 2", "gpt-4");

        assert_eq!(counter.cache_size(), 2);

        counter.clear_cache();
        assert_eq!(counter.cache_size(), 0);
    }

    #[test]
    fn test_longer_text_counts_more() {
        let counter = TiktokenCounter::new("gpt-4").unwrap();
        let via_port: &dyn TokenCounterPort = &counter;
        let long_text = "This is a longer piece of text that should result in more tokens. \
                         It contains multiple sentences and should demonstrate that the token \
                         counter is working correctly for larger inputs.";
        let count = via_port.count(long_text, "gpt-4");
        assert!(count > 20); // Should be significantly more tokens
    }

    #[test]
    fn test_special_characters() {
        let counter = TiktokenCounter::new("gpt-4").unwrap();
        let via_port: &dyn TokenCounterPort = &counter;
        let text = "Hello! 你好! مرحبا! 👋";
        let count = via_port.count(text, "gpt-4");
        assert!(count > 0);
    }

    #[test]
    fn test_multiple_models() {
        let gpt4 = TiktokenCounter::new("gpt-4").unwrap();
        let gpt35 = TiktokenCounter::new("gpt-3.5-turbo").unwrap();

        let text = "Test message";
        let count_gpt4 = TokenCounterPort::count(&gpt4, text, "gpt-4");
        let count_gpt35 = TokenCounterPort::count(&gpt35, text, "gpt-3.5-turbo");

        // Both should return valid counts (they might differ slightly)
        assert!(count_gpt4 > 0);
        assert!(count_gpt35 > 0);
    }

    // ── `impl TokenCounterPort for TiktokenCounter` (RT-03, D-13) ───────────

    use crate::token_counter::HeuristicTokenCounter;

    /// Test 5: under the `content-processing` feature, `TiktokenCounter`
    /// satisfies `TokenCounterPort`, and its exact BPE count for a known
    /// model differs from the `chars / 4` heuristic for text where the two
    /// disagree.
    #[test]
    fn tiktoken_counter_implements_the_port() {
        let counter = TiktokenCounter::new("gpt-4").unwrap();
        let via_port: &dyn TokenCounterPort = &counter;
        let text = "The quick brown fox jumps over the lazy dog, repeatedly and verbosely.";

        let tiktoken_count = via_port.count(text, "gpt-4");
        let heuristic_count = HeuristicTokenCounter.count(text, "gpt-4");

        assert!(tiktoken_count > 0);
        assert_ne!(
            tiktoken_count, heuristic_count,
            "BPE and chars/4 must disagree on this text"
        );
    }

    /// Test 6: `count` never consults `get_bpe_from_model` again at
    /// count-time -- it delegates to the encoding this instance already
    /// loaded at construction, so an unrecognised model string passed to
    /// `count` (as opposed to `TiktokenCounter::new`) never errors or
    /// panics; it just returns a number.
    #[test]
    fn tiktoken_counter_falls_back_inside_the_adapter_for_an_unknown_model() {
        let counter = TiktokenCounter::new("gpt-4").unwrap();
        let via_port: &dyn TokenCounterPort = &counter;

        let count = via_port.count("hello there", "a-model-nobody-has-heard-of");
        assert!(count > 0);
    }

    /// Test 7 (shared): `name()` returns a stable string distinct from the
    /// heuristic's, usable in the trimmer's debug log.
    #[test]
    fn tiktoken_name_identifies_the_counter() {
        let counter = TiktokenCounter::new("gpt-4").unwrap();
        let via_port: &dyn TokenCounterPort = &counter;
        assert_eq!(via_port.name(), "tiktoken");
        assert_ne!(via_port.name(), HeuristicTokenCounter.name());
    }

    /// Test 8: `TiktokenCounter` reports exact tokenisation unconditionally,
    /// scoped to the encoding resolved at `new("gpt-4")` -- and that answer
    /// is unchanged after counting the empty string, proving it does not
    /// depend on any argument.
    #[test]
    fn tiktoken_counter_is_exact() {
        let counter = TiktokenCounter::new("gpt-4").unwrap();
        let via_port: &dyn TokenCounterPort = &counter;
        assert!(via_port.is_exact());
        let _ = via_port.count("", "gpt-4");
        assert!(via_port.is_exact());
    }
}
