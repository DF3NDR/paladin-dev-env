//! The default, ungated [`TokenCounterPort`] adapter: a character-based
//! approximation (Doc 05 RT-FR-10, D-13).

use paladin_ports::output::token_counter_port::TokenCounterPort;

/// A synchronous, infallible token-count **approximation**: `±30 %` of an
/// exact BPE count, computed as `text.chars().count()` divided by 4,
/// **rounded up**.
///
/// Counts Unicode *scalar values* (`char`s), not bytes and not grapheme
/// clusters -- an 8-character string of a 2-byte-per-character script (e.g.
/// `"é".repeat(8)`) counts as 2, not 4, even though it occupies 16 bytes.
/// This is the phase-wide default (D-13): every budget feature added by this
/// phase (`HistoryTrimmer`, `SummarizationMiddleware`) works against this
/// counter with no cargo feature and no external tokenizer dependency.
#[derive(Debug, Default, Clone, Copy)]
pub struct HeuristicTokenCounter;

#[cfg(test)]
mod tests {
    use super::*;

    /// Test 1: the heuristic counts Unicode scalar values, not bytes -- an
    /// 8-character, 16-byte string counts as 2 (8 / 4), not 4 (16 / 4).
    #[test]
    fn heuristic_counts_chars_not_bytes() {
        let counter = HeuristicTokenCounter;
        let text = "é".repeat(8);
        assert_eq!(text.chars().count(), 8);
        assert_eq!(text.len(), 16, "each é is 2 bytes in UTF-8");
        assert_eq!(counter.count(&text, "any-model"), 2);
    }

    /// Test 2: the division rounds up, and an empty string counts as 0.
    #[test]
    fn heuristic_rounds_up() {
        let counter = HeuristicTokenCounter;
        assert_eq!(counter.count("abcde", "any-model"), 2, "5 chars / 4 rounds up to 2");
        assert_eq!(counter.count("", "any-model"), 0);
    }

    /// Test 3: no model string -- empty, unknown, or well-known -- ever
    /// produces an error path; `count` always just returns a number.
    #[test]
    fn heuristic_is_infallible_for_any_model_string() {
        let counter = HeuristicTokenCounter;
        let text = "hello world";
        let _: u32 = counter.count(text, "");
        let _: u32 = counter.count(text, "a-model-nobody-has-heard-of");
        let _: u32 = counter.count(text, "gpt-4");
    }

    /// Test 4: 20 calls with identical arguments return the identical
    /// number, and a fresh instance returns the same number as a used one.
    #[test]
    fn heuristic_is_deterministic() {
        let counter = HeuristicTokenCounter;
        let text = "the quick brown fox jumps over the lazy dog";
        let first = counter.count(text, "gpt-4");
        for _ in 0..20 {
            assert_eq!(counter.count(text, "gpt-4"), first);
        }
        let fresh = HeuristicTokenCounter;
        assert_eq!(fresh.count(text, "gpt-4"), first);
    }

    /// Test 7 (shared): `name()` returns a stable, adapter-identifying
    /// string.
    #[test]
    fn name_identifies_the_counter() {
        assert_eq!(HeuristicTokenCounter.name(), "heuristic");
    }
}
