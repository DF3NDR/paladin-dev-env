//! Generic, credential-shape redaction for Vault adapter-boundary error text (D-34).
//!
//! Neither `SqliteVault` nor `SemanticVault` holds a credential of its own to
//! redact exactly -- unlike `paladin-llm`'s API-key-aware redaction (which
//! knows the adapter's own configured key) or `paladin-storage`'s
//! `waypoint::redact` (which knows the exact database URL). But a wrapped
//! backend error (a `sqlx` error, a `SanctumError`, an `EmbeddingError`)
//! could still echo a credential-shaped token if the underlying store or
//! embedding provider is itself HTTP-backed -- so this module redacts by
//! SHAPE (`Bearer <token>` headers and `sk-`-prefixed tokens) as
//! defense-in-depth, the same shapes `paladin_llm::redaction` matches
//! without an adapter-specific key.
//!
//! **Ordering is load-bearing, exactly as it is in `paladin_llm::redaction`:
//! redact, then bound.** Bounding first can slice a credential-shaped token
//! in half at the truncation boundary and leak the surviving prefix. Every
//! `VaultError::Storage` / `VaultError::Serialization` message in this
//! crate's Vault adapters is built by [`redact_and_bound`], never by
//! bounding raw backend text directly.

/// Character budget for a Vault adapter's error message, applied after
/// redaction.
const ERROR_TEXT_CHAR_BUDGET: usize = 512;

/// What a redacted credential-shaped token is replaced with.
const CREDENTIAL_PLACEHOLDER: &str = "[REDACTED]";

/// Replace the token that follows every occurrence of `marker` with
/// [`CREDENTIAL_PLACEHOLDER`].
///
/// The token is taken to run until the first whitespace or JSON delimiter.
/// `marker` must be ASCII so the byte offsets `find` returns are always
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
            // library must never panic: stop scanning and emit the
            // remainder verbatim via the trailing `push_str` below.
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

/// Redact any credential-shaped token out of `text`, then bound it to
/// [`ERROR_TEXT_CHAR_BUDGET`] characters -- the order every adapter-boundary
/// `VaultError::Storage` / `VaultError::Serialization` message in this
/// crate's Vault adapters must follow (D-34): redact first, so a token
/// straddling the truncation boundary is still fully removed.
pub(crate) fn redact_and_bound(text: &str) -> String {
    let redacted = redact_token_after(&redact_token_after(text, "Bearer "), "bearer ");
    let redacted = redact_token_after(&redacted, "sk-");

    if redacted.chars().count() <= ERROR_TEXT_CHAR_BUDGET {
        redacted
    } else {
        let truncated: String = redacted.chars().take(ERROR_TEXT_CHAR_BUDGET).collect();
        format!("{truncated}... [truncated]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_and_bound_masks_bearer_tokens() {
        let text = "backend rejected: Bearer sk-should-never-appear-0123456789";
        let redacted = redact_and_bound(text);
        assert!(!redacted.contains("sk-should-never-appear-0123456789"));
        assert!(redacted.contains(CREDENTIAL_PLACEHOLDER));
        assert!(redacted.contains("backend rejected"));
    }

    #[test]
    fn redact_and_bound_redacts_before_truncating_a_token_straddling_the_budget_boundary() {
        let secret = "sk-boundary-straddling-secret-0123456789";
        let padding = "x".repeat(ERROR_TEXT_CHAR_BUDGET - 10);
        let text = format!("{padding} {secret}");

        let redacted = redact_and_bound(&text);
        assert!(
            !redacted.contains(secret),
            "a credential-shaped token straddling the truncation boundary must still be fully \
             removed: {redacted}"
        );
    }

    #[test]
    fn redact_and_bound_leaves_credential_free_text_unchanged_when_short() {
        let text = "no credentials here";
        assert_eq!(redact_and_bound(text), text);
    }
}
