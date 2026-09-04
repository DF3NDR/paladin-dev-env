//! Credential redaction for connection-string-derived error text (T-22-18).
//!
//! `SqliteWaypointStore`/`PostgresWaypointStore` are constructed from a
//! caller-supplied `database_url` that may embed a password
//! (`scheme://user:password@host/db`). Neither backend's driver is trusted
//! to omit that password from a connection or query error's `Display`
//! output, so every error this crate wraps into a `WaypointError::Backend`
//! is passed through [`redact_database_url_password`] first — **redact
//! before any truncation**, since bounding a diagnostic string first can
//! slice a password in half and leak the surviving prefix (this project's
//! security instructions).

/// Extract the password component of a `scheme://user:password@host/...`
/// connection string.
///
/// Tries the `url` crate first, but does not depend on it recognising the
/// scheme: `sqlite://` and other non-"special" schemes are not on the list
/// `url::Url` treats as authority-bearing, so `Url::parse` alone would miss
/// a password on exactly the backend (SQLite) this module also has to
/// protect. Falls back to a manual `://...:PASSWORD@` scan that only cares
/// about the generic authority shape, not the scheme.
fn extract_url_password(database_url: &str) -> Option<String> {
    if let Ok(parsed) = url::Url::parse(database_url)
        && let Some(password) = parsed.password()
        && !password.is_empty()
    {
        return Some(password.to_string());
    }

    let after_scheme = database_url.split_once("://")?.1;
    let before_at = after_scheme.split_once('@')?.0;
    let (_, password) = before_at.rsplit_once(':')?;
    if password.is_empty() {
        None
    } else {
        Some(password.to_string())
    }
}

/// Redact `database_url`'s password (if any) from `text`.
///
/// `text` is typically the `Display` output of a driver error. Returns
/// `text` unchanged when `database_url` carries no password to redact (the
/// common case: no credential, or a bare file path).
pub(crate) fn redact_database_url_password(text: &str, database_url: &str) -> String {
    match extract_url_password(database_url) {
        Some(password) => text.replace(&password, "[REDACTED]"),
        None => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_password_from_postgres_style_url() {
        let url = "postgres://appuser:super-secret-pw@db.internal:5432/paladin";
        let text = format!("connection failed: {url}");
        let redacted = redact_database_url_password(&text, url);
        assert!(!redacted.contains("super-secret-pw"));
        assert!(redacted.contains("[REDACTED]"));
    }

    #[test]
    fn redacts_password_from_sqlite_style_url_even_though_url_crate_may_not_recognise_it() {
        let url = "sqlite://user:hunter2@/nonexistent/path/db.sqlite";
        let text = format!("unable to open database file for {url}");
        let redacted = redact_database_url_password(&text, url);
        assert!(!redacted.contains("hunter2"));
        assert!(redacted.contains("[REDACTED]"));
    }

    #[test]
    fn leaves_text_unchanged_when_url_has_no_password() {
        let url = "sqlite::memory:";
        let text = "unable to open database file".to_string();
        assert_eq!(redact_database_url_password(&text, url), text);
    }

    #[test]
    fn leaves_text_unchanged_when_url_has_no_credentials_at_all() {
        let url = "postgres://db.internal:5432/paladin";
        let text = format!("connection failed: {url}");
        assert_eq!(redact_database_url_password(&text, url), text);
    }
}
