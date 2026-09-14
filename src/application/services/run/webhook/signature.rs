//! HMAC-SHA256 webhook body signing (D-41).
//!
//! [`sign_webhook_body`] signs the EXACT byte buffer that will be (or was)
//! sent -- signed once, from one buffer, never re-serialized. A receiver
//! verifies by recomputing this same HMAC over the raw bytes it captured
//! and comparing against the [`WEBHOOK_SIGNATURE_HEADER`] header value.

use hmac::{Hmac, Mac};
use sha2::Sha256;

/// The HTTP header a signed webhook delivery carries its signature in.
pub const WEBHOOK_SIGNATURE_HEADER: &str = "X-Paladin-Signature";

/// Sign `body` with `key` (the run's `WebhookSpec` signing value), returning
/// `sha256=<lowercase hex>` -- the exact [`WEBHOOK_SIGNATURE_HEADER`] value.
///
/// `body` must be the EXACT bytes handed to the HTTP client (D-41): signing
/// a re-serialized copy of the same logical payload can produce different
/// bytes (key order, whitespace) and break receiver-side verification.
pub fn sign_webhook_body(key: &[u8], body: &[u8]) -> String {
    match <Hmac<Sha256> as Mac>::new_from_slice(key) {
        Ok(mut mac) => {
            mac.update(body);
            format!("sha256={}", hex_encode(&mac.finalize().into_bytes()))
        }
        // HMAC-SHA256 accepts a key of any length (RFC 2104: longer keys
        // are hashed down, shorter keys are zero-padded) -- this branch is
        // unreachable in practice with this crate's own `Hmac<Sha256>`, but
        // this function stays panic-free rather than `.expect()`ing
        // (CLAUDE.md, library code must return `Result` or degrade safely
        // rather than panic). An all-zero digest is an obviously-wrong
        // signature a receiver will simply fail to verify, never a crash.
        Err(_) => format!("sha256={}", hex_encode(&[0u8; 32])),
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 4231 test case 2 style vector: key `"key"`, body `"The quick
    /// brown fox jumps over the lazy dog"`.
    #[test]
    fn webhook_signature() {
        let signature = sign_webhook_body(b"key", b"The quick brown fox jumps over the lazy dog");
        assert_eq!(
            signature,
            "sha256=f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8"
        );
    }

    #[test]
    fn webhook_signature_header_name_is_x_paladin_signature() {
        assert_eq!(WEBHOOK_SIGNATURE_HEADER, "X-Paladin-Signature");
    }

    #[test]
    fn webhook_signature_is_deterministic() {
        let a = sign_webhook_body(b"k", b"body");
        let b = sign_webhook_body(b"k", b"body");
        assert_eq!(a, b);
    }

    #[test]
    fn webhook_signature_differs_for_different_bodies() {
        let a = sign_webhook_body(b"k", b"body-a");
        let b = sign_webhook_body(b"k", b"body-b");
        assert_ne!(a, b);
    }

    #[test]
    fn webhook_signature_always_starts_with_sha256_prefix() {
        let signature = sign_webhook_body(b"k", b"body");
        assert!(signature.starts_with("sha256="));
        assert_eq!(signature.len(), "sha256=".len() + 64);
    }
}
