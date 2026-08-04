//! Token Counting Utilities
//!
//! Provides token counting capabilities for different LLM providers.
//!
//! The [`TokenCounter`] CONTRACT is lifted to `paladin-ports`
//! (`paladin_ports::output::token_counter_port`) so callers in other crates (notably the
//! `Quartermaster` in `paladin-llm`) can measure a prompt against a provider's declared
//! context window without depending on `paladin-memory` or its optional `tiktoken-rs`
//! dependency. This module keeps the concrete tiktoken-backed ADAPTER
//! ([`TiktokenCounter`]) and re-exports the lifted trait so existing call sites resolve
//! unchanged.

use std::collections::HashMap;
use std::sync::RwLock;
use tiktoken_rs::{CoreBPE, get_bpe_from_model};

pub use paladin_ports::output::token_counter_port::{TokenCountError, TokenCounter};

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
/// use paladin_memory::garrison::{TokenCounter, TiktokenCounter};
///
/// let counter = TiktokenCounter::new("gpt-4").unwrap();
/// let count = counter.count_tokens("Hello, world!").unwrap();
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
    /// Returns [`TokenCountError::UnsupportedModel`] if the model name is not
    /// supported by `tiktoken_rs`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use paladin_memory::garrison::TiktokenCounter;
    ///
    /// let counter = TiktokenCounter::new("gpt-4").unwrap();
    /// ```
    pub fn new(model_name: &str) -> Result<Self, TokenCountError> {
        let bpe = get_bpe_from_model(model_name).map_err(|e| {
            TokenCountError::UnsupportedModel(format!(
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

impl TokenCounter for TiktokenCounter {
    fn count_tokens(&self, text: &str) -> Result<u32, TokenCountError> {
        // Check cache first
        if let Ok(cache) = self.cache.read()
            && let Some(&count) = cache.get(text)
        {
            return Ok(count);
        }

        // Count tokens
        let tokens = self.bpe.encode_with_special_tokens(text);
        let count = tokens.len() as u32;

        // Cache the result
        if let Ok(mut cache) = self.cache.write() {
            // Limit cache size to prevent unbounded growth
            if cache.len() >= 1000 {
                cache.clear();
            }
            cache.insert(text.to_string(), count);
        }

        Ok(count)
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

/// Factory for creating token counters by model name.
///
/// Currently creates [`TiktokenCounter`] instances for all OpenAI-compatible
/// models.  Call [`TokenCounterFactory::for_model`] to obtain a
/// `Box<dyn TokenCounter>` without committing to a concrete type.
pub struct TokenCounterFactory;

impl TokenCounterFactory {
    /// Creates a token counter for the specified model.
    ///
    /// Currently supports OpenAI models via tiktoken.  Future implementations
    /// can add support for other providers.
    ///
    /// # Arguments
    ///
    /// * `model_name` - The model name (e.g., `"gpt-4"`, `"gpt-3.5-turbo"`)
    ///
    /// # Errors
    ///
    /// Returns [`TokenCountError::UnsupportedModel`] if the model is not supported.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use paladin_memory::garrison::{TokenCounterFactory, TokenCounter};
    ///
    /// let counter = TokenCounterFactory::for_model("gpt-4").unwrap();
    /// let count = counter.count_tokens("Test message").unwrap();
    /// ```
    pub fn for_model(model_name: &str) -> Result<Box<dyn TokenCounter>, TokenCountError> {
        let counter = TiktokenCounter::new(model_name)?;
        Ok(Box::new(counter))
    }

    /// Returns a list of well-known supported model names.
    pub fn supported_models() -> Vec<&'static str> {
        vec![
            "gpt-4",
            "gpt-4-32k",
            "gpt-4-turbo",
            "gpt-4o",
            "gpt-3.5-turbo",
            "gpt-3.5-turbo-16k",
            "text-embedding-ada-002",
            "text-embedding-3-small",
            "text-embedding-3-large",
        ]
    }

    /// Returns `true` if the model name is in the well-known supported list.
    pub fn is_supported(model_name: &str) -> bool {
        Self::supported_models().contains(&model_name)
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
    fn tiktoken_counter_rejects_an_unknown_model_with_unsupported_model() {
        let result = TiktokenCounter::new("a-model-tiktoken-has-never-heard-of");
        assert!(matches!(result, Err(TokenCountError::UnsupportedModel(_))));
    }

    #[test]
    fn test_count_tokens_simple() {
        let counter = TiktokenCounter::new("gpt-4").unwrap();
        let count = counter.count_tokens("Hello, world!").unwrap();
        assert!(count > 0);
        assert!(count < 10); // "Hello, world!" should be just a few tokens
    }

    #[test]
    fn test_count_tokens_empty_string() {
        let counter = TiktokenCounter::new("gpt-4").unwrap();
        let count = counter.count_tokens("").unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_count_tokens_caching() {
        let counter = TiktokenCounter::new("gpt-4").unwrap();

        // First call - not cached
        let text = "This is a test message for caching.";
        let count1 = counter.count_tokens(text).unwrap();

        // Second call - should be cached
        let count2 = counter.count_tokens(text).unwrap();

        assert_eq!(count1, count2);
        assert_eq!(counter.cache_size(), 1);
    }

    #[test]
    fn test_cache_clearing() {
        let counter = TiktokenCounter::new("gpt-4").unwrap();

        counter.count_tokens("Test 1").unwrap();
        counter.count_tokens("Test 2").unwrap();

        assert_eq!(counter.cache_size(), 2);

        counter.clear_cache();
        assert_eq!(counter.cache_size(), 0);
    }

    #[test]
    fn test_factory_for_model() {
        let counter = TokenCounterFactory::for_model("gpt-4").unwrap();
        let count = counter.count_tokens("Test").unwrap();
        assert!(count > 0);
    }

    #[test]
    fn test_factory_supported_models() {
        let models = TokenCounterFactory::supported_models();
        assert!(models.contains(&"gpt-4"));
        assert!(models.contains(&"gpt-3.5-turbo"));
    }

    #[test]
    fn test_factory_is_supported() {
        assert!(TokenCounterFactory::is_supported("gpt-4"));
        assert!(TokenCounterFactory::is_supported("gpt-3.5-turbo"));
        assert!(!TokenCounterFactory::is_supported("unknown-model"));
    }

    #[test]
    fn test_factory_error_on_unknown_model() {
        let result = TokenCounterFactory::for_model("not-a-real-model-xyz");
        assert!(result.is_err(), "Expected error for unsupported model");
    }

    #[test]
    fn test_longer_text_token_count() {
        let counter = TiktokenCounter::new("gpt-4").unwrap();
        let long_text = "This is a longer piece of text that should result in more tokens. \
                         It contains multiple sentences and should demonstrate that the token \
                         counter is working correctly for larger inputs.";
        let count = counter.count_tokens(long_text).unwrap();
        assert!(count > 20); // Should be significantly more tokens
    }

    #[test]
    fn test_special_characters() {
        let counter = TiktokenCounter::new("gpt-4").unwrap();
        let text = "Hello! 你好! مرحبا! 👋";
        let count = counter.count_tokens(text).unwrap();
        assert!(count > 0);
    }

    #[test]
    fn test_multiple_models() {
        let gpt4 = TiktokenCounter::new("gpt-4").unwrap();
        let gpt35 = TiktokenCounter::new("gpt-3.5-turbo").unwrap();

        let text = "Test message";
        let count_gpt4 = gpt4.count_tokens(text).unwrap();
        let count_gpt35 = gpt35.count_tokens(text).unwrap();

        // Both should return valid counts (they might differ slightly)
        assert!(count_gpt4 > 0);
        assert!(count_gpt35 > 0);
    }
}
