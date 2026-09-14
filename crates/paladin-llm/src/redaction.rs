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
//!
//! [`redact_secret_patterns`] (D-34) is the key-less pattern half of
//! [`redact_credentials`], factored out for a caller with no specific
//! configured key to redact against — e.g. `ToolResultFormatter::format_error`
//! sanitizing a failed tool's error text before it reaches the model. The same
//! ordering rule applies: call it before [`bounded_excerpt`], never after.

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
/// [`CREDENTIAL_PLACEHOLDER`], but only when `marker` starts at a word
/// boundary.
///
/// The token is taken to run until the first whitespace or JSON delimiter.
/// `marker` must be ASCII so the byte offsets returned by `find` are always
/// character boundaries; every slice is nonetheless taken through the
/// checked `get` API so this function has no panicking path.
///
/// **Word-boundary guard (WR-01, `26-REVIEW.md`):** `marker` only counts as
/// a real occurrence when it is at the start of `body`/`rest` or the
/// character immediately before it is not alphanumeric. Without this, the
/// `key=`/`token=` markers [`redact_secret_patterns`] passes through this
/// function match inside any ordinary word ending in those letters and
/// directly followed by `=` -- `"monkey=5"`, `"donkey=3"`, `"turkey=roast"`,
/// `"jockey=true"` would otherwise all be misredacted. An underscore is
/// intentionally NOT a boundary-breaking character, so `api_key=...` /
/// `access_token=...` (the common real-world spellings) are still
/// redacted.
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

        let is_word_boundary = rest[..idx]
            .chars()
            .next_back()
            .map(|c| !c.is_alphanumeric())
            .unwrap_or(true);

        if is_word_boundary {
            let end = tail
                .find(|c: char| c.is_whitespace() || matches!(c, '"' | ',' | '}' | ']' | '\\'))
                .unwrap_or(tail.len());

            if end > 0 {
                out.push_str(CREDENTIAL_PLACEHOLDER);
            }

            rest = tail.get(end..).unwrap_or("");
        } else {
            // `marker` is the tail of a longer word (e.g. `monkey=`) --
            // not a real marker occurrence. Leave the following text
            // untouched and keep scanning past it.
            rest = tail;
        }
    }

    out.push_str(rest);
    out
}

/// Strip anything credential-shaped out of text destined for a log line.
///
/// Two passes, in order of precision:
/// 1. the adapter's OWN configured `api_key`, matched exactly — this cannot
///    miss, and covers a gateway that echoes the request back verbatim;
/// 2. [`redact_secret_patterns`] — the shared, key-less pattern pass (D-34)
///    covering `Bearer`/`sk-`/`AKIA`-style keys, `key=`/`token=` query
///    values, and JWT-shaped triples.
///
/// Redaction MUST run before truncation, otherwise a bounded excerpt could
/// slice a secret in half and leak the surviving prefix.
pub fn redact_credentials(body: &str, api_key: &str) -> String {
    let exact = if api_key.is_empty() {
        body.to_string()
    } else {
        body.replace(api_key, CREDENTIAL_PLACEHOLDER)
    };

    redact_secret_patterns(&exact)
}

/// Strip anything credential-shaped out of text by PATTERN alone, with no
/// caller-supplied key to match exactly (D-34).
///
/// This is the pattern half of [`redact_credentials`], factored out for a
/// caller with no specific configured key to redact against — e.g. a
/// failed tool's error text, which could carry a credential the caller
/// never configured (a leaked upstream secret, another provider's key,
/// a stray access key in a URL). Covers, applied in order:
///
/// 1. `Bearer <token>` / `bearer <token>` — the header form;
/// 2. any `sk-`-prefixed token (this also covers `sk-ant-`-style keys,
///    since they are `sk-`-prefixed);
/// 3. any `AKIA`-prefixed AWS access key ID;
/// 4. `key=` / `token=` query-string values;
/// 5. JWT-shaped `header.payload.signature` triples — three dot-separated
///    base64url segments, each at least [`JWT_MIN_SEGMENT_LEN`] characters
///    (chosen well above a dotted version string like `1.2.3` or a
///    hostname label, so those are never misredacted).
///
/// **Ordering is load-bearing: call this BEFORE [`bounded_excerpt`], never
/// after** (see the module docs). Bounding first can slice a secret across
/// the truncation boundary, breaking the shape this function pattern-matches
/// on (e.g. a truncated JWT with no visible closing segment) and leaking the
/// surviving fragment.
pub fn redact_secret_patterns(text: &str) -> String {
    let no_bearer = redact_token_after(&redact_token_after(text, "Bearer "), "bearer ");
    let no_sk = redact_token_after(&no_bearer, "sk-");
    let no_akia = redact_token_after(&no_sk, "AKIA");
    let no_key_query = redact_token_after(&no_akia, "key=");
    let no_token_query = redact_token_after(&no_key_query, "token=");
    redact_jwt_triples(&no_token_query)
}

/// A single base64url character (RFC 4648 §5): alphanumeric, `-`, or `_`.
fn is_b64url_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

/// The exclusive end index of the maximal base64url run starting at
/// `start`. Scans by CHARACTER, never byte offset — see the module docs'
/// char-boundary-safety rule.
fn b64url_run_end(chars: &[char], start: usize) -> usize {
    let mut end = start;
    while end < chars.len() && is_b64url_char(chars[end]) {
        end += 1;
    }
    end
}

/// Minimum length, in characters, for a base64url run to be treated as a
/// JWT segment by [`redact_jwt_triples`].
const JWT_MIN_SEGMENT_LEN: usize = 10;

/// Redact JWT-shaped `header.payload.signature` triples: three
/// dot-separated base64url runs, each at least [`JWT_MIN_SEGMENT_LEN`]
/// characters. A matched triple is replaced by a single
/// [`CREDENTIAL_PLACEHOLDER`], never a per-segment replacement (a JWT's
/// payload can carry sensitive claims even without a signature secret).
fn redact_jwt_triples(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;

    while i < chars.len() {
        let seg1_end = b64url_run_end(&chars, i);
        let matched = (seg1_end - i >= JWT_MIN_SEGMENT_LEN && chars.get(seg1_end) == Some(&'.'))
            .then(|| seg1_end + 1)
            .and_then(|seg2_start| {
                let seg2_end = b64url_run_end(&chars, seg2_start);
                (seg2_end - seg2_start >= JWT_MIN_SEGMENT_LEN && chars.get(seg2_end) == Some(&'.'))
                    .then_some(seg2_end + 1)
            })
            .and_then(|seg3_start| {
                let seg3_end = b64url_run_end(&chars, seg3_start);
                (seg3_end - seg3_start >= JWT_MIN_SEGMENT_LEN).then_some(seg3_end)
            });

        match matched {
            Some(triple_end) => {
                out.push_str(CREDENTIAL_PLACEHOLDER);
                i = triple_end;
            }
            None => {
                out.push(chars[i]);
                i += 1;
            }
        }
    }

    out
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

    // ── D-34: redact_secret_patterns (Task 2, Plan 26-19) ────────────────

    #[test]
    fn redact_secret_patterns_covers_the_documented_set() {
        let bearer = "Authorization: Bearer sk-livekey-abcdef0123456789";
        let redacted = redact_secret_patterns(bearer);
        assert!(!redacted.contains("abcdef0123456789"), "got {redacted}");
        assert!(redacted.contains(CREDENTIAL_PLACEHOLDER), "got {redacted}");

        let sk = r#"{"key_field":"sk-plainkey-abcdefghij0123456789"}"#;
        let redacted = redact_secret_patterns(sk);
        assert!(!redacted.contains("abcdefghij0123456789"), "got {redacted}");

        let sk_ant = r#"{"key_field":"sk-ant-api03-abcdefghij0123456789"}"#;
        let redacted = redact_secret_patterns(sk_ant);
        assert!(!redacted.contains("abcdefghij0123456789"), "got {redacted}");

        let akia = "aws_access_key_id=AKIAIOSFODNN7EXAMPLE";
        let redacted = redact_secret_patterns(akia);
        assert!(!redacted.contains("IOSFODNN7EXAMPLE"), "got {redacted}");

        let key_query = "https://example.com/v1?key=abcdefghijklmnop0123456789";
        let redacted = redact_secret_patterns(key_query);
        assert!(
            !redacted.contains("abcdefghijklmnop0123456789"),
            "got {redacted}"
        );

        let token_query = "https://example.com/v1?token=abcdefghijklmnop0123456789";
        let redacted = redact_secret_patterns(token_query);
        assert!(
            !redacted.contains("abcdefghijklmnop0123456789"),
            "got {redacted}"
        );

        let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dGhpc19pc19hX3NpZ25hdHVyZQ";
        let redacted = redact_secret_patterns(jwt);
        assert!(
            !redacted.contains("eyJzdWIiOiIxMjM0NTY3ODkwIn0"),
            "got {redacted}"
        );
        assert!(redacted.contains(CREDENTIAL_PLACEHOLDER), "got {redacted}");

        let benign = "the quick brown fox jumps over the lazy dog";
        assert_eq!(redact_secret_patterns(benign), benign);
    }

    #[test]
    fn redact_secret_patterns_does_not_misfire_on_ordinary_words_ending_in_key_or_token() {
        // WR-01 (26-REVIEW.md): `key=`/`token=` must only match as a whole
        // query-parameter-shaped marker (preceded by a non-alphanumeric,
        // non-underscore boundary or start-of-string), not as the tail of
        // an ordinary word like `monkey=`/`donkey=`/`turkey=`/`jockey=`
        // immediately followed by `=`.
        for benign in [
            r#"{"monkey=5}"#.to_string(),
            "donkey=3".to_string(),
            "turkey=roast".to_string(),
            "jockey=true".to_string(),
        ] {
            assert_eq!(
                redact_secret_patterns(&benign),
                benign,
                "benign word wrongly redacted: {benign}"
            );
        }

        // A real query parameter directly after one of these words must
        // still be redacted -- the fix must not blanket-disable the marker.
        let real_after_benign = "monkey says hi; api_key=abcdefghijklmnop0123456789";
        let redacted = redact_secret_patterns(real_after_benign);
        assert!(
            !redacted.contains("abcdefghijklmnop0123456789"),
            "got {redacted}"
        );
        assert!(redacted.contains(CREDENTIAL_PLACEHOLDER), "got {redacted}");
    }

    #[test]
    fn redaction_precedes_bounding() {
        // Position a JWT so the excerpt budget cuts through the middle of
        // its payload segment. Redacting the WHOLE string first removes it
        // regardless of where the result later gets truncated; truncating
        // FIRST breaks the triple's shape (no closing dot, no third
        // segment visible) so the pattern redactor no longer recognizes it
        // -- leaking the visible fragment of the payload. This is exactly
        // the failure mode `redact_secret_patterns` must run BEFORE
        // `bounded_excerpt`, never after.
        let jwt_header = "eyJhbGciOiJIUzI1NiJ9"; // 21 chars, base64url
        let jwt_payload = "SUPER_SECRET_PAYLOAD_CONTENT_0123456789"; // 40 chars
        let jwt_signature = "dGhpc19pc19hX3NpZ25hdHVyZQ"; // 26 chars
        let jwt = format!("{jwt_header}.{jwt_payload}.{jwt_signature}");

        let visible_payload_prefix_len = 10;
        let padding_len = RESPONSE_EXCERPT_CHAR_BUDGET
            - jwt_header.chars().count()
            - 1
            - visible_payload_prefix_len;
        let padding = "x".repeat(padding_len);
        let body = format!("{padding}{jwt}");
        let leaked_fragment = &jwt_payload[..visible_payload_prefix_len];

        // CORRECT order: redact the whole JWT, THEN bound.
        let correct = bounded_excerpt(&redact_secret_patterns(&body), RESPONSE_EXCERPT_CHAR_BUDGET);
        assert!(
            !correct.contains(leaked_fragment),
            "correct ordering should not leak any payload fragment: {correct}"
        );
        assert!(!correct.contains(jwt_payload), "got {correct}");

        // WRONG order (demonstrating the failure mode this rule guards
        // against): bounding first slices the JWT's payload segment in
        // half, so the pattern redactor no longer recognizes a complete
        // triple and the visible fragment survives untouched.
        let wrong = redact_secret_patterns(&bounded_excerpt(&body, RESPONSE_EXCERPT_CHAR_BUDGET));
        assert!(
            wrong.contains(leaked_fragment),
            "expected the wrong-order composition to leak a fragment of the \
             payload, demonstrating why redaction must precede bounding: {wrong}"
        );
    }

    #[test]
    fn redact_credentials_still_behaves_identically() {
        // Guards the D-34 factoring-out: redact_credentials must still
        // behave exactly as before -- its own exact-key pass plus the
        // (now-shared) pattern pass -- with the pre-existing tests above
        // passing unmodified.
        let secret = "sk-configured-key-0123456789";
        let body = format!(r#"{{"auth":"{secret}","note":"unrelated"}}"#);
        let redacted = redact_credentials(&body, secret);

        assert!(!redacted.contains(secret), "got {redacted}");
        assert!(redacted.contains(CREDENTIAL_PLACEHOLDER), "got {redacted}");
        assert!(redacted.contains("unrelated"), "got {redacted}");
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
