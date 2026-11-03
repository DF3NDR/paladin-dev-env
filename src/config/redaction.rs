//! Credential redaction for `Debug` output of configuration types.
//!
//! Mirrors `paladin_llm::redaction::RedactedKey`, which is `pub(crate)` in
//! paladin-llm and so cannot be imported here; widening it would add public
//! surface to a second crate for a patch fix (PD-11-2, D-00b). The rule is the
//! same: a non-empty secret prints as `<redacted>`, an empty one as `""`, so an
//! unconfigured credential stays distinguishable from a configured one.
//!
//! Crate-private on purpose: no public path is added (DF-07).

/// A secret as `Debug` prints it: `<redacted>`, or `""` when the value is empty.
///
/// Config types carry their secrets in public `String` fields, so a derived `Debug` would print
/// them into any log line, panic message or test failure that formats the config (DF-07). Their
/// hand-written `Debug` impls wrap the secret in this instead.
pub(crate) struct RedactedSecret<'a>(pub(crate) &'a str);

impl std::fmt::Debug for RedactedSecret<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_empty() {
            f.write_str("\"\"")
        } else {
            f.write_str("<redacted>")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacted_secret_debug_redacts_a_value_and_quotes_empty() {
        let secret = "test-key-not-a-secret-df07-helper";
        let text = format!("{:?}", RedactedSecret(secret));
        assert_eq!(text, "<redacted>");
        assert!(!text.contains(secret));
        assert_eq!(format!("{:?}", RedactedSecret("")), "\"\"");
    }
}
