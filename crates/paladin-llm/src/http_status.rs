//! One shared HTTP-status-to-[`LlmError`] mapping for every provider adapter
//! in this crate (Doc 04 FT-FR-01, Phase 25 D-03).
//!
//! Before this module, nine adapters each carried a private
//! `match status { .. }` block, and five of them erased a `5xx` or an
//! unknown `4xx` into a `ProcessingError` whose text read `HTTP <status>: <body>`
//! — exactly the string-encoded status the transience taxonomy
//! ([`LlmError::transience`]) cannot classify without parsing text.
//! [`map_http_status`] is now the single place a status becomes a typed
//! variant, so a future RT-06 (Phase 26) re-verifies three paths instead of
//! rebuilding nine.
//!
//! **Ordering is load-bearing: redact, then bound.** The response body is a
//! remote, attacker-influenceable string that may echo the request back
//! verbatim (gateways do). Bounding it *before* redaction can slice the
//! credential across the truncation boundary, so the exact-match pass never
//! sees the whole key and the surviving tail leaks. See
//! `.github/instructions/security.instructions.md` and [`crate::redaction`].

use paladin_ports::output::llm_port::LlmError;
use paladin_ports::output::rate_limit_hints::RateLimitHints;

use crate::redaction::{RESPONSE_EXCERPT_CHAR_BUDGET, bounded_excerpt, redact_credentials};

/// The phrase an OpenAI-family `400` body carries when the prompt overflowed
/// the model's context window. Reused verbatim from the original
/// `openai/adapter.rs` predicate so the `400 -> TokenLimitExceeded`
/// disambiguation is byte-identical to what it was before this helper.
const CONTEXT_LENGTH_OVERFLOW_SIGNATURE: &str = "maximum context length";

/// Whether a (redacted) `400` body signals a context-length overflow rather
/// than a malformed prompt.
fn signals_context_length_overflow(redacted_body: &str) -> bool {
    redacted_body.contains(CONTEXT_LENGTH_OVERFLOW_SIGNATURE)
}

/// Map a non-2xx provider response to the [`LlmError`] variant that
/// classifies it — the one shared mapping for every adapter (D-03).
///
/// `body` is the raw response text; `api_key` is the credential the adapter
/// sent, so an echoed request can be scrubbed exactly. The excerpt placed in
/// the returned error is **redacted first, then bounded** to
/// [`RESPONSE_EXCERPT_CHAR_BUDGET`] characters on character boundaries —
/// never byte-sliced, never bounded before redaction. Bounding first can
/// slice a credential across the truncation boundary and leak its tail
/// (T-25-20), which is why this ordering lives in exactly one place.
///
/// Dedicated mappings (unchanged from the per-adapter blocks they replace):
///
/// | status | variant |
/// |--------|---------|
/// | 401 | [`LlmError::AuthenticationError`] |
/// | 429 | [`LlmError::RateLimitExceeded`] (no retry delay; see [`map_http_status_with_hints`] to carry one) |
/// | 402 | [`LlmError::UsageLimitExceeded`] (no regain hint) |
/// | 404 | [`LlmError::ModelNotAvailable`] |
/// | 400 | [`LlmError::TokenLimitExceeded`] when the body signals a context-length overflow, else [`LlmError::InvalidPrompt`] |
/// | anything else | [`LlmError::ProviderError`] carrying `status` as a typed `u16` |
///
/// The 5xx range, 408 and every 4xx without a row above all reach
/// `ProviderError` through the final arm, so [`LlmError::transience`] can
/// classify them by value (408/429/5xx transient, other 4xx permanent).
/// A provider-specific pre-check (Anthropic's `403`, Gemini's RPC-status
/// envelope) may run *before* this helper, but never after it and never as a
/// second copy of the table.
///
/// # Examples
///
/// ```
/// use paladin_llm::http_status::map_http_status;
/// use paladin_ports::output::llm_port::LlmError;
///
/// let err = map_http_status("openai", 503, r#"{"error":"overloaded"}"#, "sk-secret");
/// match err {
///     LlmError::ProviderError { provider, status, message } => {
///         assert_eq!(provider, "openai");
///         assert_eq!(status, 503);
///         assert!(message.contains("overloaded"));
///     }
///     other => panic!("expected ProviderError, got {other:?}"),
/// }
///
/// assert!(matches!(
///     map_http_status("openai", 429, "", "sk-secret"),
///     LlmError::RateLimitExceeded { .. }
/// ));
/// ```
pub fn map_http_status(provider: &str, status: u16, body: &str, api_key: &str) -> LlmError {
    map_http_status_with_hints(provider, status, body, api_key, None)
}

/// [`map_http_status`] for an adapter that has already parsed the response's rate-limit headers
/// (PACE-01).
///
/// The table is identical; the only difference is the `429` row. With `hints` it returns
/// [`LlmError::rate_limited_with_hints`], so the error carries the provider's own retry delay and
/// quota dimensions; without them it returns [`LlmError::rate_limited`]`(None)`, exactly what
/// [`map_http_status`] produces. `hints` is ignored for every other status.
///
/// The adapter must snapshot the headers **before** it consumes the response body (the body read
/// takes the response by value), then hand the parsed
/// [`RateLimitHints`] here -- see [`crate::rate_limit_headers`]. The hints hold parsed numbers
/// only, so nothing from a provider header can reach the rendered error.
///
/// | status | variant |
/// |--------|---------|
/// | 429 | [`LlmError::RateLimitExceeded`] carrying `hints` when given |
/// | every other status | exactly as [`map_http_status`] |
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use paladin_llm::http_status::map_http_status_with_hints;
/// use paladin_ports::output::llm_port::LlmError;
/// use paladin_ports::output::rate_limit_hints::{RateLimitHints, RetryDelaySource};
///
/// let hints = RateLimitHints::default()
///     .with_retry_after(Duration::from_secs(7), RetryDelaySource::RetryAfter);
/// let err = map_http_status_with_hints("openai", 429, "", "sk-secret", Some(hints));
/// assert!(matches!(err, LlmError::RateLimitExceeded { .. }));
/// assert_eq!(err.retry_after(), Some(Duration::from_secs(7)));
///
/// // Without hints a 429 is the bare rate-limit error.
/// let bare = map_http_status_with_hints("openai", 429, "", "sk-secret", None);
/// assert_eq!(bare.retry_after(), None);
/// ```
pub fn map_http_status_with_hints(
    provider: &str,
    status: u16,
    body: &str,
    api_key: &str,
    hints: Option<RateLimitHints>,
) -> LlmError {
    // Redact BEFORE bounding — see the module docs for why the order is
    // load-bearing. The overflow predicate reads the full redacted body (not
    // the bounded excerpt) so a signature past the character budget is not
    // missed; only the bounded excerpt is ever emitted.
    let redacted = redact_credentials(body, api_key);
    let message = bounded_excerpt(&redacted, RESPONSE_EXCERPT_CHAR_BUDGET);

    match status {
        401 => LlmError::AuthenticationError(format!(
            "Invalid API key for provider '{provider}'. Error: {message}"
        )),
        429 => match hints {
            Some(hints) => LlmError::rate_limited_with_hints(hints),
            None => LlmError::rate_limited(None),
        },
        402 => LlmError::UsageLimitExceeded {
            provider: provider.to_string(),
            regain_hint: None,
        },
        404 => LlmError::ModelNotAvailable(message),
        400 => {
            if signals_context_length_overflow(&redacted) {
                LlmError::TokenLimitExceeded
            } else {
                LlmError::InvalidPrompt(message)
            }
        }
        _ => LlmError::ProviderError {
            provider: provider.to_string(),
            status,
            message,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::transience::Transience;
    use paladin_ports::output::rate_limit_hints::RetryDelaySource;
    use std::time::Duration;

    const PROVIDER: &str = "test-provider";
    const KEY: &str = "livekey-ABCDEF0123456789";

    #[test]
    fn unmapped_statuses_become_provider_error_with_the_status_field() {
        for status in [500u16, 502, 503, 504, 599, 408] {
            match map_http_status(PROVIDER, status, "boom", KEY) {
                LlmError::ProviderError {
                    provider,
                    status: got,
                    ..
                } => {
                    assert_eq!(provider, PROVIDER, "status {status}");
                    assert_eq!(got, status, "typed status field must equal the input");
                }
                other => panic!("status {status}: expected ProviderError, got {other:?}"),
            }
        }
    }

    #[test]
    fn dedicated_statuses_keep_their_existing_variants() {
        assert!(matches!(
            map_http_status(PROVIDER, 401, "bad key", KEY),
            LlmError::AuthenticationError(_)
        ));
        assert!(matches!(
            map_http_status(PROVIDER, 429, "slow down", KEY),
            LlmError::RateLimitExceeded { .. }
        ));
        match map_http_status(PROVIDER, 402, "insufficient balance", KEY) {
            LlmError::UsageLimitExceeded {
                provider,
                regain_hint,
            } => {
                assert_eq!(provider, PROVIDER);
                assert_eq!(regain_hint, None);
            }
            other => panic!("expected UsageLimitExceeded, got {other:?}"),
        }
        assert!(matches!(
            map_http_status(PROVIDER, 404, "no such model", KEY),
            LlmError::ModelNotAvailable(_)
        ));
        assert!(matches!(
            map_http_status(
                PROVIDER,
                400,
                "This model's maximum context length is 8192 tokens",
                KEY
            ),
            LlmError::TokenLimitExceeded
        ));
        assert!(matches!(
            map_http_status(PROVIDER, 400, "missing required field 'messages'", KEY),
            LlmError::InvalidPrompt(_)
        ));
    }

    #[test]
    fn unknown_4xx_becomes_provider_error_not_processing_error() {
        for status in [403u16, 409, 418, 451, 499] {
            let err = map_http_status(PROVIDER, status, "nope", KEY);
            assert!(
                !matches!(err, LlmError::ProcessingError(_)),
                "status {status}: must never be erased into ProcessingError"
            );
            assert!(
                matches!(err, LlmError::ProviderError { status: got, .. } if got == status),
                "status {status}: expected ProviderError carrying it, got {err:?}"
            );
        }
    }

    #[test]
    fn excerpt_is_redacted_before_it_is_bounded() {
        // The key starts 5 characters before the truncation boundary, so
        // bounding first leaves a 5-character head of the key in the
        // excerpt that the exact-match redaction pass can no longer see.
        let padding = "x".repeat(RESPONSE_EXCERPT_CHAR_BUDGET - 5);
        let body = format!("{padding}{KEY} trailing text");

        // Wrong-order control, computed here so the test proves the helper
        // does not match it rather than merely asserting a happy path.
        let wrong_order =
            redact_credentials(&bounded_excerpt(&body, RESPONSE_EXCERPT_CHAR_BUDGET), KEY);
        let leaked_head = &KEY[..5];
        assert!(
            wrong_order.contains(leaked_head),
            "control must leak the key head, otherwise this test proves nothing: {wrong_order}"
        );

        let message = match map_http_status(PROVIDER, 500, &body, KEY) {
            LlmError::ProviderError { message, .. } => message,
            other => panic!("expected ProviderError, got {other:?}"),
        };

        assert!(!message.contains(KEY), "full key leaked: {message}");
        assert!(
            !message.contains(leaked_head),
            "key head leaked across the truncation boundary: {message}"
        );
        assert_ne!(
            message, wrong_order,
            "helper must not reproduce the bound-then-redact control"
        );
    }

    #[test]
    fn multibyte_body_is_bounded_on_char_boundaries() {
        let body = "\u{1F5E1}\u{FE0F}\u{2694}\u{FE0F}".repeat(200);
        assert!(body.chars().count() > RESPONSE_EXCERPT_CHAR_BUDGET);

        let message = match map_http_status(PROVIDER, 502, &body, KEY) {
            LlmError::ProviderError { message, .. } => message,
            other => panic!("expected ProviderError, got {other:?}"),
        };

        let expected_prefix: String = body.chars().take(RESPONSE_EXCERPT_CHAR_BUDGET).collect();
        assert!(
            message.starts_with(&expected_prefix),
            "excerpt must be a character-prefix of the body, never a byte slice"
        );
        assert_eq!(
            message.chars().take(RESPONSE_EXCERPT_CHAR_BUDGET).count(),
            RESPONSE_EXCERPT_CHAR_BUDGET
        );
    }

    #[test]
    fn empty_body_produces_an_empty_message_and_still_classifies() {
        let err = map_http_status(PROVIDER, 503, "", KEY);
        match &err {
            LlmError::ProviderError {
                status, message, ..
            } => {
                assert_eq!(*status, 503);
                assert_eq!(message, "");
            }
            other => panic!("expected ProviderError, got {other:?}"),
        }
        assert_eq!(err.transience(), Transience::Transient);
    }

    fn sample_hints() -> RateLimitHints {
        RateLimitHints::default()
            .with_retry_after(Duration::from_secs(7), RetryDelaySource::RetryAfter)
    }

    #[test]
    fn map_http_status_with_hints_429_carries_the_hints() {
        let err = map_http_status_with_hints(PROVIDER, 429, "slow down", KEY, Some(sample_hints()));
        assert!(matches!(err, LlmError::RateLimitExceeded { .. }));
        assert_eq!(err.retry_after(), Some(Duration::from_secs(7)));
        assert_eq!(err.rate_limit_hints(), Some(&sample_hints()));
        assert_eq!(err.transience(), Transience::Transient);

        // No hints: the same bare error `map_http_status` has always produced.
        let bare = map_http_status_with_hints(PROVIDER, 429, "slow down", KEY, None);
        assert_eq!(bare.retry_after(), None);
        assert_eq!(bare.rate_limit_hints(), None);
    }

    #[test]
    fn map_http_status_with_hints_none_hints_matches_map_http_status_for_every_other_status() {
        for status in (300u16..=599).filter(|s| *s != 429) {
            for body in [
                "",
                "boom",
                "This model's maximum context length is 8192 tokens",
            ] {
                let plain = map_http_status(PROVIDER, status, body, KEY);
                let with_none = map_http_status_with_hints(PROVIDER, status, body, KEY, None);
                assert_eq!(
                    format!("{plain:?}"),
                    format!("{with_none:?}"),
                    "status {status}, body {body:?}"
                );
                // Hints only ever ride a 429: any other status ignores them entirely.
                let with_some =
                    map_http_status_with_hints(PROVIDER, status, body, KEY, Some(sample_hints()));
                assert_eq!(
                    format!("{plain:?}"),
                    format!("{with_some:?}"),
                    "status {status}, body {body:?}, hints supplied"
                );
            }
        }
        // And the 429 delegation is identical for the hint-less case.
        assert_eq!(
            format!("{:?}", map_http_status(PROVIDER, 429, "x", KEY)),
            format!(
                "{:?}",
                map_http_status_with_hints(PROVIDER, 429, "x", KEY, None)
            )
        );
    }

    #[test]
    fn a_429_error_never_renders_the_response_body_or_the_key() {
        let err = map_http_status_with_hints(
            PROVIDER,
            429,
            &format!("echoed {KEY} back"),
            KEY,
            Some(sample_hints()),
        );
        for rendered in [format!("{err}"), format!("{err:?}")] {
            assert!(!rendered.contains(KEY), "{rendered}");
            assert!(!rendered.contains("echoed"), "{rendered}");
        }
    }
}
