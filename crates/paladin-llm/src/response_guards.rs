//! The one rule that decides what a completion's text and finish reason mean (PARITY-05).
//!
//! Every adapter that parses an OpenAI-shaped response (the five `CompatEngine` presets, OpenAI
//! and DeepSeek) hands the same five facts to [`classify_completion`] and returns whatever it
//! decides, so the three copies of this rule cannot drift apart:
//!
//! 1. **A vendor refusal wins** -- [`LlmError::Refusal`], even when the vendor also sent partial
//!    text. A refusal is checked before emptiness because a refused response usually has no text,
//!    and reporting it as empty would hide the one fact a caller deciding whether to try another
//!    provider needs (D-09; RESEARCH Pitfall 1).
//! 2. **A `content_filter` finish stays `Ok`** -- [`FinishReason::ContentFilter`] is a finish
//!    reason, not an error, so the caller inspects it on the response. It is exempt from the empty
//!    check even with no text (D-09; RESEARCH premise correction 4).
//! 3. **A function-call finish stays `Ok`** -- a model that wants a tool legitimately returns no
//!    text.
//! 4. **Text that is empty after trimming is [`LlmError::EmptyCompletion`] on any other finish
//!    reason** -- `stop` included (D-11a). It used to be `Ok("")` unless the finish reason was
//!    `length`. The payload names the finish reason and carries the advice that fits it: a larger
//!    `max_tokens` for `length`, "the model returned no text" otherwise.
//! 5. **Otherwise `Ok`.**
//!
//! # Redaction
//!
//! `Refusal.detail` is built only through [`crate::redaction::diagnostic_excerpt`]: credentials
//! are removed first, then the text is bounded by Unicode scalar values (never bytes). Bounding
//! first could slice a key across the truncation boundary and leak its prefix (D-00a).
//!
//! # Streams
//!
//! A stream cannot be classified until it ends, so [`StreamCompletion`] watches the deltas as
//! they pass and [`StreamCompletion::finish`] runs [`classify_completion`] on the accumulated
//! facts at the terminal chunk. Deltas already yielded stay yielded; only the terminal item
//! changes.
//!
//! # Billing (G-50-5a)
//!
//! The vendor bills an empty or refused response like any other. An adapter maps the response's
//! usage before it classifies, then routes the guard's error through [`bill`], which attaches the
//! usage to the error ([`LlmError::billed_usage`]) so a caller can still settle the call. No
//! usage reported means no bill, never a fabricated zero (D-00c).

use paladin_ports::output::llm_port::{BilledUsage, FinishReason, LlmError, TokenUsage};

use crate::redaction::diagnostic_excerpt;

/// Decide what a completed (or fully accumulated streamed) response means.
///
/// `refusal` is the vendor's dedicated refusal text, if the response carried one; blank text
/// counts as absent. `api_key` is the adapter's configured key, redacted out of any refusal text
/// before it is bounded. See the module documentation for the precedence and the reasons.
///
/// # Errors
///
/// - [`LlmError::Refusal`] when `refusal` carries text.
/// - [`LlmError::EmptyCompletion`] when `content` is empty or whitespace-only and `finish` is
///   neither [`FinishReason::ContentFilter`] nor [`FinishReason::FunctionCall`].
pub(crate) fn classify_completion(
    provider: &str,
    model: &str,
    content: &str,
    finish: &FinishReason,
    refusal: Option<&str>,
    api_key: &str,
) -> Result<(), LlmError> {
    if let Some(text) = refusal.filter(|text| !text.trim().is_empty()) {
        return Err(LlmError::refusal(
            provider,
            model,
            diagnostic_excerpt(text, api_key),
        ));
    }

    if matches!(
        finish,
        FinishReason::ContentFilter | FinishReason::FunctionCall
    ) {
        return Ok(());
    }

    if content.trim().is_empty() {
        return Err(LlmError::empty_completion(empty_payload(
            content.len(),
            finish,
        )));
    }

    Ok(())
}

/// Attach a failed call's billed `usage` on `model` to the guard's `err` (G-50-5a).
///
/// `model` is the vendor-reported served model when the response named one, else the request
/// model (A3-50-26). `None` usage returns `err` unchanged: nothing was reported, so nothing is
/// billed. Only [`LlmError::EmptyCompletion`] and [`LlmError::Refusal`] take the bill; any other
/// error passes through as it is ([`LlmError::with_billed_usage`]).
pub(crate) fn bill(err: LlmError, model: &str, usage: Option<TokenUsage>) -> LlmError {
    match usage {
        Some(usage) => err.with_billed_usage(BilledUsage::new(model, usage)),
        None => err,
    }
}

/// The `EmptyCompletion` payload: the finish reason, and the advice that fits it.
fn empty_payload(raw_len: usize, finish: &FinishReason) -> String {
    match finish {
        FinishReason::Length => format!(
            "finish_reason=length with empty content ({raw_len} raw chars) — reasoning likely \
             consumed the entire max_tokens budget; retry with a larger max_tokens"
        ),
        other => format!(
            "finish_reason={} with empty content ({raw_len} raw chars) — the model returned no \
             text",
            finish_label(other)
        ),
    }
}

/// A short, fixed label for a finish reason, safe to embed in an error message.
fn finish_label(finish: &FinishReason) -> &'static str {
    match finish {
        FinishReason::Stop => "stop",
        FinishReason::Length => "length",
        FinishReason::ContentFilter => "content_filter",
        FinishReason::FunctionCall => "function_call",
        // The wrapped text is vendor-supplied; it is deliberately left out of the label.
        FinishReason::Error(_) => "error",
    }
}

/// Accumulates, across a stream's deltas, exactly what [`classify_completion`] needs at the end.
///
/// Holds only what the decision depends on: the content up to and including the first delta that
/// is non-blank (after which nothing more is kept), and the joined refusal text.
#[derive(Debug, Default)]
pub(crate) struct StreamCompletion {
    content: String,
    has_text: bool,
    refusal: String,
}

impl StreamCompletion {
    /// Record one content delta.
    pub(crate) fn observe_content(&mut self, delta: &str) {
        if !self.has_text {
            self.content.push_str(delta);
            self.has_text = !self.content.trim().is_empty();
        }
    }

    /// Record one refusal delta. Kept whole: redaction runs over the full text before it is
    /// bounded, so capping here could slice a credential across the cap.
    // The Gemini and DeepSeek streams have no refusal field, so a build with only those two
    // adapters compiles this method unused (the other seven adapters call it).
    #[allow(dead_code)]
    pub(crate) fn observe_refusal(&mut self, delta: &str) {
        self.refusal.push_str(delta);
    }

    /// Classify the stream at its terminal chunk.
    ///
    /// # Errors
    ///
    /// The same errors [`classify_completion`] returns for the accumulated facts.
    pub(crate) fn finish(
        &self,
        provider: &str,
        model: &str,
        finish: &FinishReason,
        api_key: &str,
    ) -> Result<(), LlmError> {
        classify_completion(
            provider,
            model,
            &self.content,
            finish,
            Some(self.refusal.as_str()),
            api_key,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::redaction::RESPONSE_EXCERPT_CHAR_BUDGET;

    fn classify(
        content: &str,
        finish: &FinishReason,
        refusal: Option<&str>,
    ) -> Result<(), LlmError> {
        classify_completion("acme", "acme-1", content, finish, refusal, "")
    }

    #[test]
    fn classify_completion_precedence() {
        // A refusal wins over every finish reason and every text, partial text included.
        for finish in [
            FinishReason::Stop,
            FinishReason::Length,
            FinishReason::ContentFilter,
            FinishReason::FunctionCall,
            FinishReason::Error("x".into()),
        ] {
            for text in ["", "   ", "partial answer"] {
                let result = classify(text, &finish, Some("I can't help with that."));
                assert!(
                    matches!(result, Err(LlmError::Refusal { .. })),
                    "refusal must win for {finish:?} / {text:?}, got {result:?}"
                );
            }
        }

        // A content filter with no text is `Ok`, not an empty completion.
        assert!(classify("", &FinishReason::ContentFilter, None).is_ok());
        // A function call with no text is `Ok`.
        assert!(classify("", &FinishReason::FunctionCall, None).is_ok());
        // Empty on a `stop`, whitespace-only on a `stop`: both empty.
        assert!(matches!(
            classify("", &FinishReason::Stop, None),
            Err(LlmError::EmptyCompletion { .. })
        ));
        assert!(matches!(
            classify("   \n\t", &FinishReason::Stop, None),
            Err(LlmError::EmptyCompletion { .. })
        ));
        // Empty on `length` names the larger-max_tokens remedy.
        match classify("", &FinishReason::Length, None) {
            Err(LlmError::EmptyCompletion { detail: msg, .. }) => {
                assert!(msg.contains("finish_reason=length"), "{msg}");
                assert!(msg.contains("larger max_tokens"), "{msg}");
            }
            other => panic!("expected EmptyCompletion, got {other:?}"),
        }
        // Empty on `stop` names the finish reason and does NOT advise a larger budget.
        match classify("", &FinishReason::Stop, None) {
            Err(LlmError::EmptyCompletion { detail: msg, .. }) => {
                assert!(msg.contains("finish_reason=stop"), "{msg}");
                assert!(!msg.contains("max_tokens"), "{msg}");
            }
            other => panic!("expected EmptyCompletion, got {other:?}"),
        }
        // Text is fine.
        assert!(classify("ok", &FinishReason::Stop, None).is_ok());
        assert!(classify("ok", &FinishReason::Length, None).is_ok());
        // A blank refusal field is absent, not a refusal.
        assert!(classify("ok", &FinishReason::Stop, Some("  ")).is_ok());
    }

    #[test]
    fn refusal_names_provider_and_model() {
        match classify("", &FinishReason::Stop, Some("no")) {
            Err(LlmError::Refusal {
                provider,
                model,
                detail,
                ..
            }) => {
                assert_eq!(provider, "acme");
                assert_eq!(model, "acme-1");
                assert_eq!(detail, "no");
            }
            other => panic!("expected Refusal, got {other:?}"),
        }
    }

    fn detail_of(refusal: &str, api_key: &str) -> String {
        match classify_completion(
            "acme",
            "acme-1",
            "",
            &FinishReason::Stop,
            Some(refusal),
            api_key,
        ) {
            Err(LlmError::Refusal { detail, .. }) => detail,
            other => panic!("expected Refusal, got {other:?}"),
        }
    }

    #[test]
    fn refusal_detail_keeps_exactly_the_budget_whole_and_bounds_one_more() {
        let exactly = "r".repeat(RESPONSE_EXCERPT_CHAR_BUDGET);
        assert_eq!(detail_of(&exactly, ""), exactly);

        let over = "r".repeat(RESPONSE_EXCERPT_CHAR_BUDGET + 1);
        let bounded = detail_of(&over, "");
        assert_ne!(bounded, over);
        assert!(bounded.starts_with(&exactly), "{bounded}");
        assert!(bounded.contains("truncated"), "{bounded}");
    }

    #[test]
    fn refusal_detail_redacts_before_bounding() {
        // The key starts 8 characters before the 512-character boundary, so truncating first
        // would keep its first 8 characters.
        let key = "Zq7Xp9Lm2Vb4Nc8T";
        let mut text = "a".repeat(RESPONSE_EXCERPT_CHAR_BUDGET - 8);
        text.push_str(key);
        text.push_str(&"b".repeat(100));

        let detail = detail_of(&text, key);

        let chars: Vec<char> = key.chars().collect();
        for width in 3..=chars.len() {
            for window in chars.windows(width) {
                let fragment: String = window.iter().collect();
                assert!(
                    !detail.contains(&fragment),
                    "key fragment {fragment:?} leaked into {detail:?}"
                );
            }
        }
        // The placeholder itself straddles the boundary here, so only its opening survives.
        assert!(detail.contains("[REDACT"), "{detail}");
    }

    #[test]
    fn refusal_detail_bounds_multibyte_text_by_chars() {
        // Three bytes per character: a byte cut at 512 would land mid-character and panic.
        let text = "拒".repeat(RESPONSE_EXCERPT_CHAR_BUDGET + 50);
        let detail = detail_of(&text, "");
        let kept = detail.chars().take_while(|c| *c == '拒').count();
        assert_eq!(kept, RESPONSE_EXCERPT_CHAR_BUDGET);
    }

    #[test]
    fn stream_completion_counts_whitespace_only_deltas_as_empty() {
        let mut stream = StreamCompletion::default();
        stream.observe_content("  ");
        stream.observe_content("\n");
        assert!(matches!(
            stream.finish("acme", "m", &FinishReason::Stop, ""),
            Err(LlmError::EmptyCompletion { .. })
        ));

        stream.observe_content("hi");
        assert!(stream.finish("acme", "m", &FinishReason::Stop, "").is_ok());
        // Once text has been seen nothing more is kept.
        stream.observe_content(&"x".repeat(10_000));
        assert!(stream.content.len() < 100);
    }

    #[test]
    fn stream_completion_refusal_wins_over_text_and_joins_deltas() {
        let mut stream = StreamCompletion::default();
        stream.observe_content("partial");
        stream.observe_refusal("I can't ");
        stream.observe_refusal("help.");
        match stream.finish("acme", "m", &FinishReason::Stop, "") {
            Err(LlmError::Refusal { detail, .. }) => assert_eq!(detail, "I can't help."),
            other => panic!("expected Refusal, got {other:?}"),
        }
    }

    /// G-50-5a: `bill` attaches reported usage to the guard's two errors, and leaves an
    /// unreported bill (`None`) and any other error alone.
    #[test]
    fn bill_attaches_reported_usage_only() {
        let usage = TokenUsage::new(1_000, 62_000);
        let billed = bill(
            LlmError::empty_completion("x"),
            "served-1",
            Some(usage.clone()),
        );
        assert_eq!(
            billed.billed_usage(),
            Some(&BilledUsage::new("served-1", usage.clone()))
        );
        let refused = bill(
            LlmError::refusal("acme", "acme-1", "no"),
            "acme-1",
            Some(usage.clone()),
        );
        assert_eq!(
            refused.billed_usage().map(|bill| bill.usage.total_tokens),
            Some(63_000)
        );

        assert_eq!(
            bill(LlmError::empty_completion("x"), "m", None).billed_usage(),
            None
        );
        let network = bill(LlmError::NetworkError("reset".into()), "m", Some(usage));
        assert_eq!(network.billed_usage(), None);
    }

    #[test]
    fn stream_completion_content_filter_and_function_call_stay_ok() {
        let stream = StreamCompletion::default();
        assert!(
            stream
                .finish("acme", "m", &FinishReason::ContentFilter, "")
                .is_ok()
        );
        assert!(
            stream
                .finish("acme", "m", &FinishReason::FunctionCall, "")
                .is_ok()
        );
    }
}
