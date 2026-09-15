//! Token Usage Tracking
//!
//! This module defines [`TokenUsage`], a pure domain value type for tracking
//! LLM token consumption. The `application` layer re-exports it from here.

use serde::{Deserialize, Serialize};

/// Token usage statistics for an LLM request
///
/// Tracks the number of tokens consumed by prompt and completion so that
/// callers can estimate cost and enforce budget limits.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Number of tokens in the input prompt
    pub prompt_tokens: u32,
    /// Number of tokens in the generated completion
    pub completion_tokens: u32,
    /// Total tokens (prompt + completion)
    pub total_tokens: u32,
}

impl TokenUsage {
    /// Create a new `TokenUsage` with specified counts
    pub fn new(prompt_tokens: u32, completion_tokens: u32) -> Self {
        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        }
    }

    /// Create a `TokenUsage` from a total count only (no prompt/completion breakdown)
    pub fn from_total(total_tokens: u32) -> Self {
        Self {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_computes_total_from_prompt_and_completion() {
        let usage = TokenUsage::new(10, 5);
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 5);
        assert_eq!(usage.total_tokens, 15);
    }

    #[test]
    fn from_total_leaves_prompt_and_completion_at_zero() {
        let usage = TokenUsage::from_total(263);
        assert_eq!(usage.prompt_tokens, 0);
        assert_eq!(usage.completion_tokens, 0);
        assert_eq!(usage.total_tokens, 263);
    }

    #[test]
    fn default_is_all_zero() {
        let usage = TokenUsage::default();
        assert_eq!(usage.prompt_tokens, 0);
        assert_eq!(usage.completion_tokens, 0);
        assert_eq!(usage.total_tokens, 0);
    }

    #[test]
    fn partial_eq_compares_all_three_fields_not_only_total() {
        assert_eq!(TokenUsage::new(1, 2), TokenUsage::new(1, 2));
        assert_ne!(TokenUsage::new(1, 2), TokenUsage::from_total(3));
    }

    // --- Failing tests for Phase 31 plan 01 (RED) ---
    // These assert the three new optional sub-counts, the chainable builders,
    // the saturating Add/AddAssign/Sum arithmetic and the widened serde shape
    // that do not exist yet on this struct. They must fail to compile/pass
    // until the GREEN commit adds the fields, builders and trait impls.

    #[test]
    fn builders_set_optionals_and_leave_prompt_completion_total_untouched() {
        let usage = TokenUsage::new(10, 5)
            .with_cache_read(4)
            .with_cache_write(2)
            .with_reasoning(3);
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 5);
        assert_eq!(usage.total_tokens, 15);
        assert_eq!(usage.cache_read_tokens, Some(4));
        assert_eq!(usage.cache_write_tokens, Some(2));
        assert_eq!(usage.reasoning_tokens, Some(3));
    }

    #[test]
    fn add_saturates_at_u32_max_without_panicking() {
        let a = TokenUsage::new(u32::MAX, 1);
        let b = TokenUsage::new(1, 0);
        let sum = a + b;
        assert_eq!(sum.prompt_tokens, u32::MAX);
        assert_eq!(sum.completion_tokens, 1);
        assert_eq!(sum.total_tokens, u32::MAX);
    }

    #[test]
    fn default_is_the_additive_identity() {
        let x = TokenUsage::new(1_234, 567)
            .with_cache_read(100)
            .with_cache_write(50)
            .with_reasoning(200);
        assert_eq!(x.clone() + TokenUsage::default(), x);
        assert_eq!(TokenUsage::default() + x.clone(), x);
    }

    #[test]
    fn option_merge_none_plus_none_is_none() {
        let a = TokenUsage::new(1_234, 567);
        let b = TokenUsage::new(100, 50);
        let sum = a + b;
        assert_eq!(sum.cache_read_tokens, None);
        assert_eq!(sum.cache_write_tokens, None);
        assert_eq!(sum.reasoning_tokens, None);
    }

    #[test]
    fn option_merge_none_plus_some_is_some() {
        let a = TokenUsage::new(1_234, 567);
        let b = TokenUsage::new(100, 50).with_cache_read(7);
        let sum = a + b;
        assert_eq!(sum.cache_read_tokens, Some(7));
    }

    #[test]
    fn option_merge_some_plus_some_saturating_adds() {
        let a = TokenUsage::new(1_234, 567).with_cache_read(7);
        let b = TokenUsage::new(100, 50).with_cache_read(5);
        let sum = a + b;
        assert_eq!(sum.cache_read_tokens, Some(12));
    }

    #[test]
    fn option_merge_some_max_plus_some_saturates() {
        let a = TokenUsage::new(0, 0).with_reasoning(u32::MAX);
        let b = TokenUsage::new(0, 0).with_reasoning(1);
        let sum = a + b;
        assert_eq!(sum.reasoning_tokens, Some(u32::MAX));
    }

    #[test]
    fn add_recomputes_total_as_prompt_plus_completion() {
        let a = TokenUsage::new(1_234, 567);
        let b = TokenUsage::new(100, 50);
        let sum = a + b;
        assert_eq!(sum.total_tokens, sum.prompt_tokens + sum.completion_tokens);
    }

    #[test]
    fn add_assign_matches_add() {
        let a = TokenUsage::new(1_234, 567).with_cache_read(100);
        let b = TokenUsage::new(100, 50).with_cache_read(50);
        let mut acc = a.clone();
        acc += b.clone();
        assert_eq!(acc, a + b);
    }

    #[test]
    fn sum_over_iterator_equals_sequential_add() {
        let a = TokenUsage::new(1_234, 567).with_cache_read(100);
        let b = TokenUsage::new(100, 50).with_cache_write(50);
        let c = TokenUsage::new(200, 20).with_reasoning(15);
        let summed: TokenUsage = vec![a.clone(), b.clone(), c.clone()].into_iter().sum();
        assert_eq!(summed, a + b + c);
    }

    #[test]
    fn legacy_json_without_optionals_deserialises_to_none() {
        let legacy = r#"{"prompt_tokens":10,"completion_tokens":5,"total_tokens":15}"#;
        let usage: TokenUsage = serde_json::from_str(legacy).unwrap();
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 5);
        assert_eq!(usage.total_tokens, 15);
        assert_eq!(usage.cache_read_tokens, None);
        assert_eq!(usage.cache_write_tokens, None);
        assert_eq!(usage.reasoning_tokens, None);
    }

    #[test]
    fn six_key_json_round_trips_and_serialises_all_six_keys() {
        let usage = TokenUsage::new(1_234, 567)
            .with_cache_read(100)
            .with_cache_write(50)
            .with_reasoning(200);
        let json = serde_json::to_string(&usage).unwrap();
        assert!(json.contains("\"prompt_tokens\""));
        assert!(json.contains("\"completion_tokens\""));
        assert!(json.contains("\"total_tokens\""));
        assert!(json.contains("\"cache_read_tokens\""));
        assert!(json.contains("\"cache_write_tokens\""));
        assert!(json.contains("\"reasoning_tokens\""));
        let round_tripped: TokenUsage = serde_json::from_str(&json).unwrap();
        assert_eq!(round_tripped, usage);
    }

    #[test]
    fn none_optionals_serialise_as_null_not_omitted() {
        let usage = TokenUsage::new(10, 5);
        let json = serde_json::to_string(&usage).unwrap();
        assert!(json.contains("\"cache_read_tokens\":null"));
        assert!(json.contains("\"cache_write_tokens\":null"));
        assert!(json.contains("\"reasoning_tokens\":null"));
    }
}
