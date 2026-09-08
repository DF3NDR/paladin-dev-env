//! Shared pagination primitives for every `/v1` list endpoint (D-47):
//! `PageQuery`, `resolve_limit`, `encode_cursor`/`decode_cursor`.
//!
//! One `limit` + opaque `cursor` shape, applied verbatim by every list
//! handler in this crate (`run_controller`, `thread_controller`,
//! `assistant_controller`, `schedule_controller`) so PLAT-FR-16's pagination
//! claim is a single sentence, not six independent ones.
//!
//! `limit` is clamped by validation, never silently: an omitted `limit`
//! defaults to `DEFAULT_PAGE_LIMIT` (20); `Some(0)` and anything above
//! `MAX_PAGE_LIMIT` (100) are rejected with `400 bad_request`; `Some(100)`
//! succeeds. `cursor` is an opaque, base64url (no padding) encoding of a
//! compact JSON value whose shape is entirely this crate's own choice --
//! never documented as a stable format a client should parse, and a
//! malformed cursor answers `400 bad_request` (code `invalid_cursor`) rather
//! than a silent full-table scan or a `500`.
//!
//! ## Cursor-walk semantics (must_haves backstop)
//!
//! A keyset cursor gives a STABLE ordering for rows that already existed
//! when the first page was fetched, but it is NOT a point-in-time snapshot:
//! a row inserted after the first page was read, whose sort key would place
//! it before the cursor's own position, is never seen by a walk that is
//! already past that point. Every list handler's own `#[utoipa::path]`
//! description states this explicitly (D-47's own must_haves backstop item)
//! rather than implying snapshot isolation the underlying `WHERE key > ?`
//! keyset query does not provide.

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;

/// Default `limit` when a caller omits it entirely (D-47).
pub const DEFAULT_PAGE_LIMIT: u32 = 20;
/// Maximum accepted `limit` (D-47); `0` and anything above this value is a
/// typed `400`, never silently clamped.
pub const MAX_PAGE_LIMIT: u32 = 100;

/// Shared `limit`/`cursor` query parameters, mirroring
/// `thread_controller::HistoryQuery`'s own field names exactly (D-47) --
/// every list handler in this crate deserializes its own query struct with
/// these same two fields (some also add filters), so the wire shape is
/// identical everywhere even though each handler still owns its own
/// `#[derive(Deserialize)]` struct for its filter fields.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PageQuery {
    /// Maximum number of items to return (`1..=100`; omitted defaults to
    /// [`DEFAULT_PAGE_LIMIT`]).
    #[serde(default)]
    pub limit: Option<u32>,
    /// Opaque pagination cursor from a previous page's `next_cursor`.
    #[serde(default)]
    pub cursor: Option<String>,
}

/// Resolve a caller-supplied `limit` into a validated page size (D-47).
///
/// # Errors
///
/// Returns `400 bad_request` for `Some(0)` or a value above
/// [`MAX_PAGE_LIMIT`], with the message `"limit must be between 1 and
/// 100"`.
pub fn resolve_limit(limit: Option<u32>) -> Result<u32, ApiError> {
    match limit {
        None => Ok(DEFAULT_PAGE_LIMIT),
        Some(l) if l == 0 || l > MAX_PAGE_LIMIT => {
            Err(ApiError::bad_request("limit must be between 1 and 100"))
        }
        Some(l) => Ok(l),
    }
}

/// Encode `value` as an opaque, base64url (no padding) `next_cursor` --
/// internal structure is never a published contract (D-47).
///
/// `serde_json::to_vec` failing on a plain struct of primitives/strings
/// (every cursor type in this crate) would be a programmer error, not a
/// caller-input condition -- there is no `NaN`/`Infinity` float or
/// non-string map key anywhere in a cursor shape this crate defines, so an
/// empty-array fallback (rather than a panic) is the correct, total
/// response to an invariant this function's own callers are responsible for
/// upholding.
pub fn encode_cursor(value: &impl Serialize) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Decode an opaque `cursor` query parameter into `T`.
///
/// # Errors
///
/// Returns `400 bad_request` (code `invalid_cursor`, message `"invalid
/// cursor"` -- never the caller's raw input and never this format's
/// internal encoding) on malformed base64, invalid JSON, or a JSON value
/// that does not match `T`'s shape.
pub fn decode_cursor<T: serde::de::DeserializeOwned>(raw: &str) -> Result<T, ApiError> {
    fn invalid_cursor() -> ApiError {
        ApiError::new(
            axum::http::StatusCode::BAD_REQUEST,
            "invalid_cursor",
            "invalid cursor",
        )
    }
    let bytes = URL_SAFE_NO_PAD.decode(raw).map_err(|_| invalid_cursor())?;
    serde_json::from_slice(&bytes).map_err(|_| invalid_cursor())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct SampleCursor {
        id: String,
        n: u32,
    }

    // --- resolve_limit -------------------------------------------------

    #[test]
    fn resolve_limit_none_defaults_to_twenty() {
        assert_eq!(resolve_limit(None).unwrap(), DEFAULT_PAGE_LIMIT);
    }

    #[test]
    fn resolve_limit_zero_is_400() {
        let err = resolve_limit(Some(0)).unwrap_err();
        assert_eq!(err.status(), axum::http::StatusCode::BAD_REQUEST);
        assert_eq!(err.to_body()["error"]["code"], "bad_request");
    }

    #[test]
    fn resolve_limit_101_is_400() {
        let err = resolve_limit(Some(101)).unwrap_err();
        assert_eq!(err.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[test]
    fn resolve_limit_100_succeeds() {
        assert_eq!(resolve_limit(Some(100)).unwrap(), 100);
    }

    #[test]
    fn resolve_limit_one_succeeds() {
        assert_eq!(resolve_limit(Some(1)).unwrap(), 1);
    }

    // --- encode_cursor / decode_cursor ----------------------------------

    #[test]
    fn cursor_round_trips() {
        let cursor = SampleCursor {
            id: "abc".to_string(),
            n: 7,
        };
        let encoded = encode_cursor(&cursor);
        let decoded: SampleCursor = decode_cursor(&encoded).unwrap();
        assert_eq!(decoded, cursor);
    }

    #[test]
    fn cursor_is_base64url_no_padding() {
        let cursor = SampleCursor {
            id: "x".to_string(),
            n: 1,
        };
        let encoded = encode_cursor(&cursor);
        assert!(!encoded.contains('+'));
        assert!(!encoded.contains('/'));
        assert!(!encoded.contains('='));
    }

    #[test]
    fn decode_cursor_bad_base64_is_invalid_cursor_400() {
        let err = decode_cursor::<SampleCursor>("not-valid-base64!!!").unwrap_err();
        assert_eq!(err.status(), axum::http::StatusCode::BAD_REQUEST);
        assert_eq!(err.to_body()["error"]["code"], "invalid_cursor");
        assert_eq!(err.to_body()["error"]["message"], "invalid cursor");
    }

    #[test]
    fn decode_cursor_bad_shape_is_invalid_cursor_400() {
        // Valid base64url, valid JSON, wrong shape for `SampleCursor`.
        let encoded = URL_SAFE_NO_PAD.encode(b"[1,2,3]");
        let err = decode_cursor::<SampleCursor>(&encoded).unwrap_err();
        assert_eq!(err.status(), axum::http::StatusCode::BAD_REQUEST);
        assert_eq!(err.to_body()["error"]["code"], "invalid_cursor");
    }

    #[test]
    fn decode_cursor_never_leaks_the_raw_input_or_encoding() {
        let err = decode_cursor::<SampleCursor>("garbage-cursor-value").unwrap_err();
        let body = err.to_body().to_string();
        assert!(!body.contains("garbage-cursor-value"));
    }
}
