//! One shared HTTP-status-to-[`LlmError`] mapping for every provider adapter
//! in this crate (Doc 04 FT-FR-01, Phase 25 D-03).
//!
//! Before this module, nine adapters each carried a private
//! `match status { .. }` block, and five of them erased a `5xx` or an
//! unknown `4xx` into `ProcessingError(format!("HTTP {}: ..."))` — exactly
//! the string-encoded status the transience taxonomy
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

use crate::redaction::{RESPONSE_EXCERPT_CHAR_BUDGET, bounded_excerpt, redact_credentials};

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::transience::Transience;

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
            LlmError::RateLimitExceeded
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
        let wrong_order = redact_credentials(
            &bounded_excerpt(&body, RESPONSE_EXCERPT_CHAR_BUDGET),
            KEY,
        );
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
}
