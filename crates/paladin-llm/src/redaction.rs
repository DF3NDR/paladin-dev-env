//! Credential redaction for diagnostic text.
//!
//! Crate-level, **not** feature-gated. Extracted from the DeepSeek adapter
//! (`deepseek/adapter.rs:250-356`) so both the shared OpenAI-compatible core
//! (`compat/engine.rs`, gated behind provider features) and the bespoke
//! Gemini adapter (plan 17-05, which does not use the compatible core) share
//! one implementation of this security-critical behaviour.
//!
//! **Ordering is load-bearing: redact, then bound.** Bounding a response body
//! before redaction can slice a secret in half at the truncation boundary and
//! leak the surviving prefix. Every call site in this crate MUST call
//! [`redact_credentials`] before [`bounded_excerpt`], never the reverse.

/// Character budget for a diagnostic excerpt of a response body.
pub const RESPONSE_EXCERPT_CHAR_BUDGET: usize = 512;

/// What a redacted credential is replaced with in a diagnostic excerpt.
///
/// `pub(crate)` (not private) so callers elsewhere in this crate that
/// migrated off a local duplicate of this module (WR-01, `25-REVIEW.md`) —
/// e.g. `deepseek::adapter`'s own tests — can assert against the same
/// constant this module redacts with, rather than a copy-pasted literal
/// that could silently drift.
pub(crate) const CREDENTIAL_PLACEHOLDER: &str = "[REDACTED]";

/// Deserialize a possibly-`null` (or absent) string field as an empty string.
///
/// Shared across every provider under `compat/` because a reasoning-model
/// preset (not just DeepSeek) can report an empty answer as JSON `null`
/// rather than `""` when its hidden reasoning consumes the whole
/// `max_tokens` budget. Kept here rather than per-preset so future presets
/// inherit the tolerance for free.
pub fn deserialize_null_as_empty_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

/// Build a diagnostic excerpt of a response body, bounded by CHARACTER count
/// rather than byte count.
///
/// Slicing a UTF-8 `&str` by byte offset panics when the offset lands
/// mid-character, and panics are forbidden in this library — a captured
/// production response body is full of multi-byte characters. When `body`
/// exceeds `budget` characters, an ASCII elision marker reports the total
/// byte length of the untruncated body so the reader knows how much was
/// withheld.
pub fn bounded_excerpt(body: &str, budget: usize) -> String {
    if body.chars().count() <= budget {
        return body.to_string();
    }

    let truncated: String = body.chars().take(budget).collect();
    format!("{truncated}... [truncated, {} total bytes]", body.len())
}

/// Replace the token that follows every occurrence of `marker` with
/// [`CREDENTIAL_PLACEHOLDER`].
///
/// The token is taken to run until the first whitespace or JSON delimiter.
/// `marker` must be ASCII so the byte offsets returned by `find` are always
/// character boundaries; every slice is nonetheless taken through the
/// checked `get` API so this function has no panicking path.
fn redact_token_after(body: &str, marker: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut rest = body;

    while let Some(idx) = rest.find(marker) {
        let cut = idx + marker.len();
        let (head, tail) = match (rest.get(..cut), rest.get(cut..)) {
            (Some(head), Some(tail)) => (head, tail),
            // Unreachable for an ASCII `marker` located by `find`, but this
            // library must never panic: stop scanning and emit the remainder
            // verbatim via the trailing `push_str` below.
            _ => break,
        };

        out.push_str(head);

        let end = tail
            .find(|c: char| c.is_whitespace() || matches!(c, '"' | ',' | '}' | ']' | '\\'))
            .unwrap_or(tail.len());

        if end > 0 {
            out.push_str(CREDENTIAL_PLACEHOLDER);
        }

        rest = tail.get(end..).unwrap_or("");
    }

    out.push_str(rest);
    out
}

/// Strip anything credential-shaped out of text destined for a log line.
///
/// Three passes, in order of precision:
/// 1. the adapter's OWN configured `api_key`, matched exactly — this cannot
///    miss, and covers a gateway that echoes the request back verbatim;
/// 2. `Bearer <token>` / `bearer <token>`, the header form;
/// 3. any surviving `sk-`-prefixed token.
///
/// Redaction MUST run before truncation, otherwise a bounded excerpt could
/// slice a secret in half and leak the surviving prefix.
pub fn redact_credentials(body: &str, api_key: &str) -> String {
    let exact = if api_key.is_empty() {
        body.to_string()
    } else {
        body.replace(api_key, CREDENTIAL_PLACEHOLDER)
    };

    let no_bearer = redact_token_after(&redact_token_after(&exact, "Bearer "), "bearer ");
    redact_token_after(&no_bearer, "sk-")
}

/// Render untrusted provider text as a log-safe diagnostic excerpt:
/// credentials stripped first, then bounded to
/// [`RESPONSE_EXCERPT_CHAR_BUDGET`] characters.
///
/// The ordering is load-bearing — truncating first could slice a secret in
/// half and leak the surviving prefix.
pub fn diagnostic_excerpt(body: &str, api_key: &str) -> String {
    let redacted = redact_credentials(body, api_key);
    bounded_excerpt(&redacted, RESPONSE_EXCERPT_CHAR_BUDGET)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_excerpt_returns_input_unchanged_when_shorter_than_budget() {
        let body = r#"{"error":"short"}"#;
        assert_eq!(bounded_excerpt(body, RESPONSE_EXCERPT_CHAR_BUDGET), body);
    }

    #[test]
    fn bounded_excerpt_is_char_boundary_safe_on_multibyte_input() {
        // Byte-slicing this would panic mid-character; char-count truncation
        // must not. A production body is full of multi-byte text.
        let body = "\u{1F5E1}\u{FE0F}\u{2694}\u{FE0F}".repeat(64);
        let budget = 5;
        let excerpt = bounded_excerpt(&body, budget);

        assert!(excerpt.starts_with("\u{1F5E1}"));
        assert!(excerpt.contains("[truncated,"));
        assert_eq!(
            excerpt.chars().take(budget).count(),
            budget,
            "must keep exactly `budget` characters before the elision marker"
        );
    }

    #[test]
    fn diagnostic_excerpt_never_echoes_the_configured_api_key() {
        // The constraint that motivated this test: a captured body excerpt is
        // written straight to an operator-facing log line, so it must never
        // carry a credential — asserted, not assumed.
        let secret = "sk-livekey-abcdef0123456789";

        // A gateway echoing the whole request back, headers included.
        let echoed = format!(
            r#"{{"error":"bad gateway","request":{{"headers":{{"authorization":"Bearer {secret}"}}}}}}"#
        );
        let excerpt = diagnostic_excerpt(&echoed, secret);

        assert!(
            !excerpt.contains(secret),
            "excerpt leaked the API key: {excerpt}"
        );
        assert!(
            !excerpt.contains("livekey"),
            "excerpt leaked part of the API key: {excerpt}"
        );
        assert!(
            excerpt.contains(CREDENTIAL_PLACEHOLDER),
            "excerpt should show the redaction happened: {excerpt}"
        );
        // The surrounding diagnostic context must survive redaction.
        assert!(excerpt.contains("bad gateway"), "got {excerpt}");
    }

    #[test]
    fn diagnostic_excerpt_redacts_before_truncating_a_key_straddling_the_budget_boundary() {
        // A key positioned so that byte-first truncation would slice it in
        // half must still be fully removed: redact-then-bound, never the
        // reverse.
        let secret = "sk-boundary-straddling-secret-value-0123456789";
        let padding = "x".repeat(RESPONSE_EXCERPT_CHAR_BUDGET - 10);
        let body = format!(r#"{{"pad":"{padding}","key":"{secret}"}}"#);

        let excerpt = diagnostic_excerpt(&body, secret);

        assert!(
            !excerpt.contains(secret),
            "excerpt leaked the API key across the truncation boundary: {excerpt}"
        );
    }

    #[test]
    fn redact_credentials_masks_bearer_and_sk_tokens_it_was_not_configured_with() {
        // Defense in depth: a key OTHER than this adapter's own (e.g. an
        // upstream proxy's) must still be masked by shape.
        let body = r#"{"msg":"denied","auth":"Bearer sk-someoneelses-9876543210"}"#;
        let redacted = redact_credentials(body, "");

        assert!(!redacted.contains("9876543210"), "got {redacted}");
        assert!(redacted.contains(CREDENTIAL_PLACEHOLDER), "got {redacted}");
        assert!(redacted.contains("denied"), "got {redacted}");
    }

    #[test]
    fn redact_credentials_leaves_credential_free_bodies_untouched() {
        let body = r#"{"id":"chatcmpl-1","choices":[{"index":0}]}"#;
        assert_eq!(redact_credentials(body, "sk-not-present"), body);
    }

    #[test]
    fn deserialize_null_as_empty_string_normalizes_null_to_empty() {
        #[derive(serde::Deserialize)]
        struct Wrapper {
            #[serde(default, deserialize_with = "deserialize_null_as_empty_string")]
            content: String,
        }

        let from_null: Wrapper = serde_json::from_str(r#"{"content":null}"#).unwrap();
        assert_eq!(from_null.content, "");

        let from_value: Wrapper = serde_json::from_str(r#"{"content":"hello"}"#).unwrap();
        assert_eq!(from_value.content, "hello");
    }
}
