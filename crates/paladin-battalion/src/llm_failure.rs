//! The one `LlmError` -> [`PaladinError::LlmFailure`] conversion (D-02, X-06).
//!
//! Before Phase 25 every site that held a real
//! [`LlmError`](paladin_ports::output::llm_port::LlmError) erased it into
//! `PaladinError::LlmError(e.to_string())` one line before the engine needed
//! to know whether the failure was worth retrying. This module replaces those
//! eight erasures with a single conversion into the structured
//! [`PaladinError::LlmFailure`] variant so `transience`, the HTTP `status` and
//! the `provider` survive the crossing into the core error taxonomy.
//!
//! # Why this lives in `paladin-battalion`
//!
//! `paladin-core` cannot see `LlmError` at all: dependencies flow inward only
//! (X-01), and `LlmError` is a `paladin-ports` type. `paladin-battalion`
//! already depends on both `paladin-core` and `paladin-ports`, and the
//! application facade crate already depends on `paladin-battalion`, so one
//! definition here serves both the engine-side call sites
//! (`conclave_execution_service`) and the application-side ones
//! (`paladin_execution_service`, `temperature_service`). There is
//! deliberately no second copy under `src/`.
//!
//! # Invariants
//!
//! - **Rendered text is byte-identical to the legacy erasure (X-03).**
//!   `PaladinError::LlmFailure` renders as `LLM error: {message}` and
//!   `message` is the source `LlmError`'s own `Display`, so the result renders
//!   exactly what `PaladinError::LlmError(err.to_string())` rendered at the
//!   same site. No log line, message assertion or downstream string consumer
//!   changes.
//! - **Every field is read from a typed source (FT-FR-01, T-25-24).**
//!   `transience` comes from
//!   [`LlmError::transience`](paladin_ports::output::llm_port::LlmError::transience);
//!   `status` and `provider` come from `ProviderError`'s and
//!   `UsageLimitExceeded`'s own fields. Nothing is parsed out of a rendered
//!   message, and a variant without a status converts with `None`, never a
//!   sentinel such as `0`.
//! - **Retryability is unchanged (T-25-25).**
//!   [`PaladinError::is_retryable`] answers `true` for `LlmFailure` exactly
//!   as it did for the legacy variant, so
//!   `src/infrastructure/resilience/circuit_breaker.rs` is not edited and its
//!   accounting does not drift.

use paladin_core::platform::container::node_error::NodeErrorSource;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_ports::output::llm_port::LlmError;

/// Convert a real [`LlmError`] into the structured [`PaladinError::LlmFailure`].
///
/// This is the single conversion every first-party site uses in place of the
/// legacy `PaladinError::LlmError(e.to_string())` erasure. See the module
/// documentation for the invariants it upholds.
///
/// # Examples
///
/// ```rust
/// use paladin_battalion::llm_failure::to_paladin_error;
/// use paladin_core::platform::container::paladin_error::PaladinError;
/// use paladin_core::platform::container::transience::Transience;
/// use paladin_ports::output::llm_port::LlmError;
///
/// let err = LlmError::ProviderError {
///     provider: "openai".to_string(),
///     status: 503,
///     message: "upstream unavailable".to_string(),
/// };
///
/// let converted = to_paladin_error(&err);
///
/// // The rendered text is exactly what the legacy erasure rendered.
/// assert_eq!(converted.to_string(), format!("LLM error: {err}"));
///
/// // ...but transience, status and provider now survive the crossing.
/// match converted {
///     PaladinError::LlmFailure { transience, status, provider, .. } => {
///         assert_eq!(transience, Transience::Transient);
///         assert_eq!(status, Some(503));
///         assert_eq!(provider.as_deref(), Some("openai"));
///     }
///     other => panic!("expected LlmFailure, got {other:?}"),
/// }
/// ```
pub fn to_paladin_error(err: &LlmError) -> PaladinError {
    let (status, provider) = typed_origin(err);
    PaladinError::LlmFailure {
        transience: err.transience(),
        status,
        provider,
        // The source's own rendering: `LlmFailure` displays as
        // `LLM error: {message}`, byte-identical to what the legacy
        // `PaladinError::LlmError(err.to_string())` rendered (X-03).
        message: err.to_string(),
    }
}

/// Read the HTTP status and provider name from the variants that carry them
/// as typed fields; `(None, None)` for every other variant.
///
/// Only `ProviderError` carries a status, so every other variant converts
/// with `status: None` -- never a sentinel such as `0`, and never a value
/// parsed out of the rendered message (T-25-24). `UsageLimitExceeded` names
/// its provider by value without a status. A fallback chain reports whatever
/// its last attempt carried, matching how its `transience()` delegates to
/// `last` (FT-FR-16).
///
/// `LlmError` is `#[non_exhaustive]` from this crate's point of view (D-04),
/// so the wildcard arm is compiler-required; a future variant that gains a
/// typed status or provider must add its own arm here to have it cross.
fn typed_origin(err: &LlmError) -> (Option<u16>, Option<String>) {
    match err {
        LlmError::ProviderError {
            provider, status, ..
        } => (Some(*status), Some(provider.clone())),
        LlmError::UsageLimitExceeded { provider, .. } => (None, Some(provider.clone())),
        LlmError::AllProvidersFailed { last, .. } => typed_origin(last),
        _ => (None, None),
    }
}

/// Convert a `PaladinPort::execute` failure into the structured
/// [`NodeErrorSource`] the superstep engine records (Doc 04 D-07).
///
/// The one place a `PaladinError` crosses into the `NodeError` family, kept
/// beside [`to_paladin_error`] so the two conversions read the SAME typed
/// fields: a [`PaladinError::LlmFailure`] becomes
/// [`NodeErrorSource::Llm`] carrying the `status` and `provider`
/// `to_paladin_error` put there (so an LLM failure is distinguishable from
/// any other Paladin failure), and every other variant becomes
/// [`NodeErrorSource::Paladin`] whose `kind` is the variant name and whose
/// `status`/`provider` are `None` -- never a sentinel, never parsed out of
/// a message (T-25-24). `message` is the error's own `Display` in both
/// cases, so the `WaypointStatus::Failed.error` display line built from it
/// is byte-identical to the legacy `StateNodeError(e.to_string())` path
/// (X-03). Redaction happened upstream, before the text entered
/// `LlmFailure` (D-34); this function adds no new unredacted path.
///
/// Replaces plan 25-01's temporary "every failure is
/// `NodeErrorSource::Function`" mapping for Paladin nodes.
///
/// # Examples
///
/// ```rust
/// use paladin_battalion::llm_failure::to_node_error_source;
/// use paladin_core::platform::container::node_error::NodeErrorSource;
/// use paladin_core::platform::container::paladin_error::PaladinError;
/// use paladin_core::platform::container::transience::Transience;
///
/// let err = PaladinError::LlmFailure {
///     transience: Transience::Transient,
///     status: Some(503),
///     provider: Some("openai".to_string()),
///     message: "upstream unavailable".to_string(),
/// };
/// match to_node_error_source(&err) {
///     NodeErrorSource::Llm { kind, status, provider, message } => {
///         assert_eq!(kind, "LlmFailure");
///         assert_eq!(status, Some(503));
///         assert_eq!(provider.as_deref(), Some("openai"));
///         assert_eq!(message, err.to_string());
///     }
///     other => panic!("expected Llm, got {other:?}"),
/// }
/// ```
pub fn to_node_error_source(err: &PaladinError) -> NodeErrorSource {
    match err {
        PaladinError::LlmFailure {
            status, provider, ..
        } => NodeErrorSource::Llm {
            kind: "LlmFailure".to_string(),
            status: *status,
            provider: provider.clone(),
            message: err.to_string(),
        },
        other => NodeErrorSource::Paladin {
            kind: paladin_error_kind(other).to_string(),
            status: None,
            provider: None,
            message: other.to_string(),
        },
    }
}

/// The variant name of a [`PaladinError`], the machine-stable `kind` a
/// [`NodeErrorSource::Paladin`] carries.
///
/// `PaladinError` is `#[non_exhaustive]` from this crate's point of view
/// (D-04), so the wildcard arm is compiler-required; a future variant
/// surfaces as `"Other"` until it is given its own arm here.
fn paladin_error_kind(err: &PaladinError) -> &'static str {
    match err {
        PaladinError::ConfigurationError(_) => "ConfigurationError",
        PaladinError::ExecutionError(_) => "ExecutionError",
        PaladinError::LlmError(_) => "LlmError",
        PaladinError::LlmFailure { .. } => "LlmFailure",
        PaladinError::Timeout(_) => "Timeout",
        PaladinError::StopWordDetected(_) => "StopWordDetected",
        PaladinError::CircuitBreakerOpen => "CircuitBreakerOpen",
        PaladinError::MaxRetriesExceeded(_) => "MaxRetriesExceeded",
        PaladinError::GarrisonError(_) => "GarrisonError",
        PaladinError::GarrisonRequired => "GarrisonRequired",
        PaladinError::ArsenalError(_) => "ArsenalError",
        _ => "Other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::transience::Transience;

    #[test]
    fn llm_failure_becomes_node_error_source_llm_with_its_typed_fields() {
        let err = to_paladin_error(&LlmError::ProviderError {
            provider: "openai".to_string(),
            status: 503,
            message: "upstream unavailable".to_string(),
        });
        match to_node_error_source(&err) {
            NodeErrorSource::Llm {
                kind,
                status,
                provider,
                message,
            } => {
                assert_eq!(kind, "LlmFailure");
                assert_eq!(status, Some(503));
                assert_eq!(provider.as_deref(), Some("openai"));
                assert_eq!(message, err.to_string());
            }
            other => panic!("expected Llm, got {other:?}"),
        }
    }

    #[test]
    fn non_llm_paladin_failures_become_node_error_source_paladin_named_by_variant() {
        let cases = [
            (
                PaladinError::ExecutionError("boom".to_string()),
                "ExecutionError",
            ),
            (PaladinError::Timeout(30), "Timeout"),
            (PaladinError::CircuitBreakerOpen, "CircuitBreakerOpen"),
            (PaladinError::LlmError("legacy".to_string()), "LlmError"),
        ];
        for (err, expected_kind) in cases {
            match to_node_error_source(&err) {
                NodeErrorSource::Paladin {
                    kind,
                    status,
                    provider,
                    message,
                } => {
                    assert_eq!(kind, expected_kind);
                    assert_eq!(status, None, "never a sentinel");
                    assert_eq!(provider, None);
                    assert_eq!(message, err.to_string());
                }
                other => panic!("expected Paladin for {err:?}, got {other:?}"),
            }
        }
    }

    /// One instance of every `LlmError` variant.
    ///
    /// Kept in sync with the enum by hand: `LlmError` is `#[non_exhaustive]`
    /// from this crate's point of view, so the compiler cannot enforce the
    /// list here. `paladin-ports`' own `llm_error_transience_table` test (in
    /// the crate that owns the enum) is the exhaustive guard; this list
    /// mirrors it one-for-one.
    fn every_variant() -> Vec<LlmError> {
        vec![
            LlmError::NetworkError("connection refused".to_string()),
            LlmError::AuthenticationError("invalid API key".to_string()),
            LlmError::InvalidPrompt("empty prompt".to_string()),
            LlmError::RateLimitExceeded,
            LlmError::UsageLimitExceeded {
                provider: "anthropic".to_string(),
                regain_hint: Some("2026-10-01 00:00 UTC".to_string()),
            },
            LlmError::ModelNotAvailable("gpt-99".to_string()),
            LlmError::TokenLimitExceeded,
            LlmError::EmptyCompletion("finish_reason=length, content empty".to_string()),
            LlmError::ProcessingError("unexpected body".to_string()),
            LlmError::Timeout("30s elapsed".to_string()),
            LlmError::ProviderError {
                provider: "openai".to_string(),
                status: 503,
                message: "upstream unavailable".to_string(),
            },
            LlmError::AllProvidersFailed {
                attempts: vec![
                    ("openai".to_string(), "HTTP 503".to_string()),
                    ("deepseek".to_string(), "invalid API key".to_string()),
                ],
                last: Box::new(LlmError::AuthenticationError("invalid API key".to_string())),
            },
        ]
    }

    /// Destructure the structured variant or fail the test naming what came
    /// back instead.
    fn expect_failure(err: PaladinError) -> (Transience, Option<u16>, Option<String>, String) {
        match err {
            PaladinError::LlmFailure {
                transience,
                status,
                provider,
                message,
            } => (transience, status, provider, message),
            other => panic!("expected PaladinError::LlmFailure, got {other:?}"),
        }
    }

    /// Test 1 (X-03): the rendered text is byte-identical to the legacy
    /// erasure `PaladinError::LlmError(e.to_string())`, which renders as
    /// `LLM error: {e}`. Covers every variant, not a sample.
    #[test]
    fn conversion_preserves_the_rendered_message_exactly() {
        for err in every_variant() {
            let legacy_rendering = format!("LLM error: {err}");
            assert_eq!(
                to_paladin_error(&err).to_string(),
                legacy_rendering,
                "rendering drifted for {err:?}"
            );
        }
    }

    /// Test 2 (FT-FR-01): transience is the source's own `transience()`.
    #[test]
    fn conversion_carries_transience_from_the_source() {
        for err in every_variant() {
            let (transience, ..) = expect_failure(to_paladin_error(&err));
            assert_eq!(
                transience,
                err.transience(),
                "transience drifted for {err:?}"
            );
        }
    }

    /// Test 3: `ProviderError`'s typed `status` and `provider` fields cross
    /// as-is.
    #[test]
    fn provider_error_conversion_carries_status_and_provider() {
        let err = LlmError::ProviderError {
            provider: "openai".to_string(),
            status: 503,
            message: "upstream unavailable".to_string(),
        };

        let (transience, status, provider, message) = expect_failure(to_paladin_error(&err));

        assert_eq!(transience, Transience::Transient);
        assert_eq!(status, Some(503));
        assert_eq!(provider.as_deref(), Some("openai"));
        assert_eq!(message, err.to_string());
    }

    /// Test 4: variants that carry no status convert with `None`, never a
    /// sentinel such as `0`.
    #[test]
    fn variants_without_a_status_convert_with_none() {
        let without_status = [
            LlmError::NetworkError("connection refused".to_string()),
            LlmError::EmptyCompletion("finish_reason=length".to_string()),
            LlmError::ProcessingError("unexpected body".to_string()),
        ];

        for err in without_status {
            let (_, status, provider, _) = expect_failure(to_paladin_error(&err));
            assert_eq!(status, None, "status must be None for {err:?}");
            assert_eq!(provider, None, "provider must be None for {err:?}");
        }
    }

    /// Test 5: `UsageLimitExceeded` names its provider by value and that
    /// name crosses; it carries no HTTP status so `status` stays `None`.
    #[test]
    fn usage_limit_exceeded_carries_its_provider() {
        let err = LlmError::UsageLimitExceeded {
            provider: "anthropic".to_string(),
            regain_hint: None,
        };

        let (transience, status, provider, _) = expect_failure(to_paladin_error(&err));

        assert_eq!(transience, Transience::Permanent);
        assert_eq!(status, None);
        assert_eq!(provider.as_deref(), Some("anthropic"));
    }

    /// Test 6 (FT-FR-16): a fallback chain's transience is exactly its last
    /// attempt's, and the last attempt's typed status/provider cross too.
    #[test]
    fn all_providers_failed_converts_with_its_last_error_transience() {
        let transient_tail = LlmError::AllProvidersFailed {
            attempts: vec![("openai".to_string(), "HTTP 503".to_string())],
            last: Box::new(LlmError::ProviderError {
                provider: "deepseek".to_string(),
                status: 502,
                message: "bad gateway".to_string(),
            }),
        };
        let (transience, status, provider, _) = expect_failure(to_paladin_error(&transient_tail));
        assert_eq!(transience, transient_tail.transience());
        assert_eq!(transience, Transience::Transient);
        assert_eq!(status, Some(502));
        assert_eq!(provider.as_deref(), Some("deepseek"));

        let permanent_tail = LlmError::AllProvidersFailed {
            attempts: vec![("openai".to_string(), "HTTP 503".to_string())],
            last: Box::new(LlmError::AuthenticationError("invalid API key".to_string())),
        };
        let (transience, status, provider, _) = expect_failure(to_paladin_error(&permanent_tail));
        assert_eq!(transience, permanent_tail.transience());
        assert_eq!(transience, Transience::Permanent);
        assert_eq!(status, None);
        assert_eq!(provider, None);
    }

    /// Test 7 (T-25-25): the legacy `is_retryable()` answer for an LLM
    /// failure was a blanket `true`; the structured variant answers the same,
    /// so the circuit breaker's accounting does not drift.
    #[test]
    fn converted_failure_is_retryable_like_the_legacy_variant() {
        for err in every_variant() {
            assert!(
                to_paladin_error(&err).is_retryable(),
                "is_retryable() must stay true for {err:?}"
            );
        }
    }
}
