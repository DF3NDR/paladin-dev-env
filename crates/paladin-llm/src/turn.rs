//! The one user-turn text helper every provider adapter's request builder calls (PARITY-08).
//!
//! # The defect this module exists to prevent
//!
//! Phase 50 handoff finding B-4: the OpenAI adapter built the user message from
//! `UserPrompt::context` while `PaladinExecutionService` fills `UserPrompt::query` and leaves
//! `context` as `None`. On the main execution path every OpenAI request therefore went out with an
//! empty user message. The other adapters read `query` only, so the split between "which field is
//! the prompt" was a per-adapter convention, and one adapter held the wrong one.
//!
//! # The rule
//!
//! Every adapter's request builder (OpenAI, Anthropic, DeepSeek, Gemini and the shared
//! `CompatEngine` behind Kimi, Qwen, Grok, Ollama and the OpenAI-compatible provider) builds the
//! user turn through [`user_turn_text`] and nothing else, so the same [`UserPrompt`] puts the same
//! text on the wire for every provider.
//!
//! # The joining rule
//!
//! A part (the query, then the context) is *present* when it is non-empty after trimming. Present
//! parts are joined with one blank line (`"\n\n"`). Trimming decides presence only: each part's own
//! text is sent unchanged, with no normalisation, trimming or truncation, so multibyte text reaches
//! the wire byte for byte.

use paladin_core::platform::container::prompt::UserPrompt;

/// The separator between the query and the context: one blank line.
const PART_SEPARATOR: &str = "\n\n";

/// The text of the user turn for `user`: the query, then the context, each only when it is
/// non-empty after trimming, joined by one blank line.
///
/// Both parts absent yields the empty string; the adapters' own empty-prompt handling decides what
/// to do with that.
pub(crate) fn user_turn_text(user: &UserPrompt) -> String {
    let context = user.context.as_deref().unwrap_or_default();
    let mut text = String::with_capacity(user.query.len() + PART_SEPARATOR.len() + context.len());
    for part in [user.query.as_str(), context] {
        if part.trim().is_empty() {
            continue;
        }
        if !text.is_empty() {
            text.push_str(PART_SEPARATOR);
        }
        text.push_str(part);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prompt(query: &str, context: Option<&str>) -> UserPrompt {
        UserPrompt {
            query: query.to_string(),
            context: context.map(str::to_string),
        }
    }

    #[test]
    fn query_only_is_the_query() {
        assert_eq!(
            user_turn_text(&prompt("What is 2+2?", None)),
            "What is 2+2?"
        );
    }

    #[test]
    fn no_context_yields_exactly_the_query() {
        assert_eq!(user_turn_text(&prompt("q", None)), "q");
    }

    #[test]
    fn empty_context_yields_exactly_the_query() {
        assert_eq!(user_turn_text(&prompt("q", Some(""))), "q");
    }

    #[test]
    fn whitespace_only_context_yields_exactly_the_query() {
        assert_eq!(user_turn_text(&prompt("q", Some(" \t\n  "))), "q");
    }

    #[test]
    fn empty_query_with_a_context_yields_the_context_alone() {
        assert_eq!(user_turn_text(&prompt("", Some("ctx"))), "ctx");
        assert_eq!(user_turn_text(&prompt("  \n", Some("ctx"))), "ctx");
    }

    #[test]
    fn both_present_are_joined_by_one_blank_line() {
        assert_eq!(
            user_turn_text(&prompt("What is 2+2?", Some("Extra context."))),
            "What is 2+2?\n\nExtra context."
        );
    }

    #[test]
    fn each_part_is_sent_unchanged() {
        // Trimming decides presence only; the text of a present part keeps its own whitespace.
        assert_eq!(
            user_turn_text(&prompt("  q  \n", Some("\tctx "))),
            "  q  \n\n\n\tctx "
        );
    }

    #[test]
    fn multibyte_text_is_unchanged() {
        assert_eq!(
            user_turn_text(&prompt("Grüße — 你好 🚀", Some("Kontext — 上下文 🌍"))),
            "Grüße — 你好 🚀\n\nKontext — 上下文 🌍"
        );
    }

    #[test]
    fn both_absent_yield_the_empty_string() {
        assert_eq!(user_turn_text(&prompt("", None)), "");
    }
}
