//! Redis connection-URL redaction shared by every Redis-backed adapter in this crate.
//!
//! A Redis URL can embed a password (`redis://:secret@host:6379/0`). It must never reach a
//! log line, an error message or a `Debug` rendering unredacted (security.instructions.md,
//! "Manual review still matters"). This helper was lifted verbatim from the run queue's
//! private copy (PACE-03, plan 43-07) so the run queue and the cadence adapter route through
//! one function rather than two that could drift apart.

/// Render `url` with any embedded password replaced by a fixed placeholder, or a fixed
/// placeholder for the whole value if it does not even parse as a URL (never echo an
/// unparsable value verbatim -- it could still be, or contain, a credential).
pub(crate) fn redact_connection_url(url: &str) -> String {
    match url::Url::parse(url) {
        Ok(mut parsed) => {
            if parsed.password().is_some() {
                let _ = parsed.set_password(Some("REDACTED"));
            }
            parsed.to_string()
        }
        Err(_) => "[REDACTED: unparsable connection url]".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_password_is_replaced_and_the_rest_of_the_url_survives() {
        let rendered = redact_connection_url("redis://:hunter2@cache.internal:6379/2");
        assert!(!rendered.contains("hunter2"), "{rendered}");
        assert!(rendered.contains("REDACTED"), "{rendered}");
        assert!(rendered.contains("cache.internal"), "{rendered}");
        assert!(rendered.contains("/2"), "{rendered}");
    }

    #[test]
    fn a_url_without_a_password_is_unchanged() {
        let rendered = redact_connection_url("redis://127.0.0.1:6380/2");
        assert_eq!(rendered, "redis://127.0.0.1:6380/2");
        assert!(!rendered.contains("REDACTED"));
    }

    #[test]
    fn an_unparsable_value_is_never_echoed() {
        let rendered = redact_connection_url("not a url :: password=hunter2");
        assert!(!rendered.contains("hunter2"), "{rendered}");
        assert!(rendered.contains("REDACTED"), "{rendered}");
    }
}
