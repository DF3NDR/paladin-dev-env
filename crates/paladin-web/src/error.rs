//! Unified API error model (Milestone 12, Epic 4).
//!
//! Every `paladin-web` handler renders failures through [`ApiError`], which serializes a
//! single, stable envelope:
//!
//! ```json
//! { "error": { "code": "not_found", "message": "unknown agent 'x'", "details": null } }
//! ```
//!
//! `code` is a stable `snake_case` machine identifier; `message` is human-facing; `details`
//! is optional structured context (rendered as `null` when absent). Constructors map to
//! the HTTP statuses the API uses.

use axum::Json;
use axum::http::{HeaderValue, StatusCode, header::RETRY_AFTER};
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, SecondsFormat, Utc};
use paladin_core::platform::container::allowance::AllowanceRefusal;
use paladin_core::platform::container::treasury_ledger::format_cost;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use utoipa::ToSchema;

/// OpenAPI schema for the error envelope returned by every failing handler.
///
/// Mirrors [`ApiError::to_body`] (`{ "error": { "code", "message", "details" } }`); it exists
/// purely so the generated spec can describe error responses. The runtime body is built by
/// `ApiError`, not this type.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiErrorBody {
    /// The error detail object.
    pub error: ApiErrorDetail,
}

/// The `error` object inside [`ApiErrorBody`].
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiErrorDetail {
    /// Stable, machine-readable error code (e.g. `"not_found"`, `"unauthorized"`).
    pub code: String,
    /// Human-readable message.
    pub message: String,
    /// Optional structured context; `null` when absent.
    #[schema(value_type = Object, nullable)]
    pub details: Option<Value>,
}

/// A structured API error: HTTP status + machine `code` + human `message` + optional details.
#[derive(Debug, Clone)]
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
    details: Option<Value>,
    /// Whole seconds for the `Retry-After` response header, when set (Phase 41, D-12).
    retry_after: Option<u64>,
}

impl ApiError {
    /// Construct an error from an explicit status, stable code, and message.
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            details: None,
            retry_after: None,
        }
    }

    /// Attach structured details (rendered under `error.details`).
    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }

    /// Attach a `Retry-After` header value, in whole seconds (Phase 41, D-12).
    pub fn with_retry_after(mut self, secs: u64) -> Self {
        self.retry_after = Some(secs);
        self
    }

    /// The `Retry-After` seconds this error renders with, when set.
    pub fn retry_after(&self) -> Option<u64> {
        self.retry_after
    }

    /// The HTTP status this error renders with.
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// `400 Bad Request` — malformed or invalid input (`code = "bad_request"`).
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "bad_request", message)
    }

    /// `401 Unauthorized` — missing or invalid credentials (`code = "unauthorized"`).
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "unauthorized", message)
    }

    /// `403 Forbidden` — authenticated but not permitted (`code = "forbidden"`).
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, "forbidden", message)
    }

    /// `404 Not Found` (`code = "not_found"`).
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "not_found", message)
    }

    /// `409 Conflict` (`code = "conflict"`).
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, "conflict", message)
    }

    /// `422 Unprocessable Entity` (`code = "unprocessable_entity"`).
    pub fn unprocessable(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "unprocessable_entity",
            message,
        )
    }

    /// `413 Payload Too Large` (`code = "payload_too_large"`).
    pub fn payload_too_large(message: impl Into<String>) -> Self {
        Self::new(StatusCode::PAYLOAD_TOO_LARGE, "payload_too_large", message)
    }

    /// `429 Too Many Requests` (`code = "too_many_requests"`).
    pub fn too_many_requests(message: impl Into<String>) -> Self {
        Self::new(StatusCode::TOO_MANY_REQUESTS, "too_many_requests", message)
    }

    /// `429 Too Many Requests` for an exhausted allowance (`code = "allowance_exhausted"`,
    /// Phase 41 D-12, D-13).
    ///
    /// A different code from the per-IP rate limiter's, so a client can tell quota from pacing by
    /// the body code alone. `error.details` carries exactly the refused ceiling's own figures --
    /// `scope`, `kind`, `balance`, `ceiling`, `window_start`, `window_end` (the last two `null`
    /// for a lifetime ceiling) -- and never the caller's tenant id or API key name. A
    /// `Retry-After` header carries the whole seconds from the store clock to the window's end,
    /// and is omitted for a lifetime refusal.
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::{TimeZone, Utc};
    /// use paladin_core::platform::container::allowance::{
    ///     AllowanceLimitKind, AllowanceRefusal, AllowanceScopeKind,
    /// };
    /// use paladin_core::platform::container::cost::{Cost, CurrencyCode};
    /// use paladin_web::error::ApiError;
    ///
    /// let usd = CurrencyCode::new("USD")?;
    /// let at = |day, hour| Utc.with_ymd_and_hms(2026, 10, day, hour, 0, 0).single();
    /// let (start, end, now) = (at(3, 0).ok_or("t")?, at(4, 0).ok_or("t")?, at(3, 23).ok_or("t")?);
    /// let refusal = AllowanceRefusal {
    ///     scope_kind: AllowanceScopeKind::ApiKey,
    ///     limit_kind: AllowanceLimitKind::Window,
    ///     balance: Cost::new(2_500_000_000, usd.clone()),
    ///     ceiling: Cost::new(2_500_000_000, usd),
    ///     window: Some((start, end)),
    ///     evaluated_at: now,
    /// };
    /// let err = ApiError::allowance_exhausted(&refusal);
    /// assert_eq!(err.status().as_u16(), 429);
    /// assert_eq!(err.retry_after(), Some(3600));
    /// assert_eq!(err.to_body()["error"]["code"], "allowance_exhausted");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn allowance_exhausted(refusal: &AllowanceRefusal) -> Self {
        let rfc3339 = |instant: DateTime<Utc>| instant.to_rfc3339_opts(SecondsFormat::Secs, true);
        let (window_start, window_end) = match refusal.window {
            Some((start, end)) => (Some(rfc3339(start)), Some(rfc3339(end))),
            None => (None, None),
        };
        let error = Self::new(
            StatusCode::TOO_MANY_REQUESTS,
            "allowance_exhausted",
            refusal.to_string(),
        )
        .with_details(json!({
            "scope": refusal.scope_kind.as_str(),
            "kind": refusal.limit_kind.as_str(),
            "balance": format_cost(&refusal.balance),
            "ceiling": format_cost(&refusal.ceiling),
            "window_start": window_start,
            "window_end": window_end,
        }));
        match refusal.retry_after_secs() {
            Some(secs) => error.with_retry_after(secs),
            None => error,
        }
    }

    /// `501 Not Implemented` (`code = "not_implemented"`).
    pub fn not_implemented(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_IMPLEMENTED, "not_implemented", message)
    }

    /// `502 Bad Gateway` — an upstream (LLM/tool) execution failure (`code = "bad_gateway"`).
    pub fn bad_gateway(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_GATEWAY, "bad_gateway", message)
    }

    /// `504 Gateway Timeout` — execution exceeded its deadline (`code = "gateway_timeout"`).
    pub fn gateway_timeout(message: impl Into<String>) -> Self {
        Self::new(StatusCode::GATEWAY_TIMEOUT, "gateway_timeout", message)
    }

    /// `500 Internal Server Error` (`code = "internal"`).
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal", message)
    }

    /// Render the JSON body (without the status), e.g. for an SSE `error` event.
    pub fn to_body(&self) -> Value {
        json!({
            "error": {
                "code": self.code,
                "message": self.message,
                "details": self.details,
            }
        })
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let retry_after = self.retry_after;
        let mut response = (self.status, Json(self.to_body())).into_response();
        if let Some(secs) = retry_after {
            response
                .headers_mut()
                .insert(RETRY_AFTER, HeaderValue::from(secs));
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_has_nested_envelope_with_null_details() {
        let err = ApiError::not_found("unknown agent 'x'");
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
        let body = err.to_body();
        assert_eq!(body["error"]["code"], "not_found");
        assert_eq!(body["error"]["message"], "unknown agent 'x'");
        assert!(body["error"]["details"].is_null());
    }

    #[test]
    fn error_body_schema_mirrors_runtime_body() {
        // The documented schema type must round-trip a real ApiError body 1:1, so the spec
        // and the wire format can't drift.
        let body = ApiError::not_found("missing").to_body();
        let parsed: ApiErrorBody =
            serde_json::from_value(body.clone()).expect("schema type parses the runtime body");
        assert_eq!(parsed.error.code, "not_found");
        assert_eq!(parsed.error.message, "missing");
        assert!(parsed.error.details.is_none());
        // Re-serializing the schema type yields the same JSON shape.
        assert_eq!(serde_json::to_value(&parsed).unwrap(), body);
    }

    #[test]
    fn with_details_is_rendered() {
        let err = ApiError::bad_request("invalid").with_details(json!({ "field": "input" }));
        let body = err.to_body();
        assert_eq!(body["error"]["details"]["field"], "input");
    }

    #[test]
    fn constructors_map_to_expected_status_and_code() {
        for (err, status, code) in [
            (
                ApiError::bad_request("m"),
                StatusCode::BAD_REQUEST,
                "bad_request",
            ),
            (ApiError::not_found("m"), StatusCode::NOT_FOUND, "not_found"),
            (ApiError::conflict("m"), StatusCode::CONFLICT, "conflict"),
            (
                ApiError::unprocessable("m"),
                StatusCode::UNPROCESSABLE_ENTITY,
                "unprocessable_entity",
            ),
            (
                ApiError::payload_too_large("m"),
                StatusCode::PAYLOAD_TOO_LARGE,
                "payload_too_large",
            ),
            (
                ApiError::too_many_requests("m"),
                StatusCode::TOO_MANY_REQUESTS,
                "too_many_requests",
            ),
            (
                ApiError::not_implemented("m"),
                StatusCode::NOT_IMPLEMENTED,
                "not_implemented",
            ),
            (
                ApiError::bad_gateway("m"),
                StatusCode::BAD_GATEWAY,
                "bad_gateway",
            ),
            (
                ApiError::gateway_timeout("m"),
                StatusCode::GATEWAY_TIMEOUT,
                "gateway_timeout",
            ),
            (
                ApiError::internal("m"),
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal",
            ),
        ] {
            assert_eq!(err.status(), status);
            assert_eq!(err.to_body()["error"]["code"], code);
        }
    }

    fn refusal(
        scope: paladin_core::platform::container::allowance::AllowanceScopeKind,
        limit: paladin_core::platform::container::allowance::AllowanceLimitKind,
        window: bool,
    ) -> AllowanceRefusal {
        use chrono::TimeZone;
        use paladin_core::platform::container::cost::{Cost, CurrencyCode};
        let usd = CurrencyCode::new("USD").expect("USD");
        let at = |day, hour| {
            Utc.with_ymd_and_hms(2026, 10, day, hour, 0, 0)
                .single()
                .expect("instant")
        };
        AllowanceRefusal {
            scope_kind: scope,
            limit_kind: limit,
            balance: Cost::new(2_500_000_000, usd.clone()),
            ceiling: Cost::new(2_500_000_000, usd),
            window: window.then(|| (at(3, 0), at(4, 0))),
            evaluated_at: at(3, 23),
        }
    }

    #[tokio::test]
    async fn allowance_exhausted_window_refusal_sets_retry_after_and_figures() {
        use paladin_core::platform::container::allowance::{
            AllowanceLimitKind, AllowanceScopeKind,
        };
        let err = ApiError::allowance_exhausted(&refusal(
            AllowanceScopeKind::ApiKey,
            AllowanceLimitKind::Window,
            true,
        ));
        assert_eq!(err.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(err.retry_after(), Some(3_600));

        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            response
                .headers()
                .get("retry-after")
                .map(HeaderValue::as_bytes),
            Some(&b"3600"[..])
        );
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let body: Value = serde_json::from_slice(&bytes).expect("json");
        assert_eq!(body["error"]["code"], "allowance_exhausted");
        assert_ne!(body["error"]["code"], "too_many_requests");
        let details = &body["error"]["details"];
        assert_eq!(details["scope"], "api_key");
        assert_eq!(details["kind"], "window");
        assert_eq!(details["balance"], "2.5000 USD");
        assert_eq!(details["ceiling"], "2.5000 USD");
        assert_eq!(details["window_start"], "2026-10-03T00:00:00Z");
        assert_eq!(details["window_end"], "2026-10-04T00:00:00Z");
    }

    #[tokio::test]
    async fn allowance_exhausted_lifetime_refusal_omits_retry_after() {
        use paladin_core::platform::container::allowance::{
            AllowanceLimitKind, AllowanceScopeKind,
        };
        let err = ApiError::allowance_exhausted(&refusal(
            AllowanceScopeKind::Tenant,
            AllowanceLimitKind::Lifetime,
            false,
        ));
        assert_eq!(err.retry_after(), None);
        let response = err.into_response();
        assert!(response.headers().get("retry-after").is_none());
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let body: Value = serde_json::from_slice(&bytes).expect("json");
        assert_eq!(body["error"]["details"]["scope"], "tenant");
        assert_eq!(body["error"]["details"]["kind"], "lifetime");
        assert!(body["error"]["details"]["window_start"].is_null());
        assert!(body["error"]["details"]["window_end"].is_null());
    }

    #[test]
    fn allowance_exhausted_details_have_exactly_six_keys() {
        use paladin_core::platform::container::allowance::{
            AllowanceLimitKind, AllowanceScopeKind,
        };
        let body = ApiError::allowance_exhausted(&refusal(
            AllowanceScopeKind::ApiKey,
            AllowanceLimitKind::Window,
            true,
        ))
        .to_body();
        let mut keys: Vec<&str> = body["error"]["details"]
            .as_object()
            .expect("details object")
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "balance",
                "ceiling",
                "kind",
                "scope",
                "window_end",
                "window_start"
            ]
        );
    }
}
