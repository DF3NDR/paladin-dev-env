//! Paladin error types
//!
//! This module defines error types for Paladin execution operations.
use crate::platform::container::arsenal::ArsenalError;
use crate::platform::container::garrison_error::GarrisonError;
use crate::platform::container::transience::Transience;
use thiserror::Error;

/// Errors that can occur during Paladin operations.
///
/// # Example
///
/// ```
/// use paladin_core::platform::container::paladin_error::PaladinError;
///
/// let error = PaladinError::ConfigurationError("Invalid temperature".to_string());
/// assert_eq!(error.to_string(), "Configuration error: Invalid temperature");
/// ```
#[derive(Debug, Error)]
pub enum PaladinError {
    /// Configuration validation failed
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// Error during Paladin execution
    #[error("Execution error: {0}")]
    ExecutionError(String),

    /// Error from the LLM provider.
    ///
    /// **Legacy / retained for compatibility (D-02, X-06):** this stringly
    /// variant classifies [`Transience::Unknown`] via [`PaladinError::transience`]
    /// and, as of this phase, is constructed by no first-party code — every
    /// call site that used to build this variant now builds
    /// [`PaladinError::LlmFailure`] instead, which carries the same message
    /// (byte-identical `Display`) plus a typed `transience`/`status`/`provider`.
    /// The variant is not removed (X-03: no pre-existing public variant is
    /// removed or reshaped) so any external caller still matching on it keeps
    /// compiling.
    #[error("LLM error: {0}")]
    LlmError(String),

    /// A structured LLM-provider failure crossing the `paladin-core` boundary
    /// by value (D-02). Rendered identically to the legacy
    /// [`PaladinError::LlmError`] arm (`"LLM error: {message}"`) so
    /// `src/infrastructure/resilience/circuit_breaker.rs`'s message-based
    /// callers observe no change; [`PaladinError::is_retryable`] returns
    /// `true` for this variant, the same legacy answer `LlmError(_)` gave.
    #[error("LLM error: {message}")]
    LlmFailure {
        /// Whether this failure is worth retrying, classified by the
        /// originating [`crate::platform::container::transience::Transience`]-aware
        /// adapter (typically `paladin-ports`' `LlmError::transience()`).
        transience: Transience,
        /// The HTTP status code, if the failure came from an HTTP response.
        status: Option<u16>,
        /// The LLM provider name, if known.
        provider: Option<String>,
        /// A redacted, human-readable summary of the failure.
        message: String,
    },

    /// Execution exceeded the configured timeout
    #[error("Timeout after {0} seconds")]
    Timeout(u64),

    /// A stop word was detected in the output
    #[error("Stop word detected: {0}")]
    StopWordDetected(String),

    /// Circuit breaker is open, rejecting requests
    #[error("Circuit breaker open: too many failures")]
    CircuitBreakerOpen,

    /// Maximum retry attempts exceeded
    #[error("Maximum retry attempts ({0}) exceeded")]
    MaxRetriesExceeded(u32),

    /// Error from the Garrison memory system
    #[error("Garrison error: {0}")]
    GarrisonError(#[from] GarrisonError),

    /// Garrison is required for multi-turn conversations but not provided
    #[error("Garrison is required for multi-turn conversations")]
    GarrisonRequired,

    /// Error from the Arsenal tool system
    #[error("Arsenal error: {0}")]
    ArsenalError(#[from] ArsenalError),
}

impl PaladinError {
    /// Check if this error is retryable.
    ///
    /// **Legacy predicate, superseded by [`PaladinError::transience`] (D-05).**
    /// Every answer this gives today is unchanged by this phase and is never
    /// re-tuned: [`PaladinError::LlmFailure`] returns `true`, the same
    /// blanket answer the legacy [`PaladinError::LlmError`] arm always gave,
    /// so `src/infrastructure/resilience/circuit_breaker.rs`'s behaviour does
    /// not shift under the new variant.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            PaladinError::LlmError(_)
                | PaladinError::LlmFailure { .. }
                | PaladinError::ExecutionError(_)
        )
    }

    /// Check if this error represents a terminal state.
    ///
    /// **Legacy predicate, superseded by [`PaladinError::transience`] (D-05).**
    /// Every answer this gives today is unchanged by this phase.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            PaladinError::Timeout(_)
                | PaladinError::StopWordDetected(_)
                | PaladinError::CircuitBreakerOpen
                | PaladinError::MaxRetriesExceeded(_)
                | PaladinError::GarrisonRequired
        )
    }

    /// Classify whether this error is worth retrying (Doc 04 FT-FR-01, D-05).
    ///
    /// Every arm reads a typed field or a variant identity only -- never a
    /// rendered `Display` string, a substring or a parsed status code out of
    /// message text. `GarrisonError`/`ArsenalError` delegate to a
    /// per-inner-variant match rather than a blanket [`Transience::Unknown`],
    /// each row documented inline with its reasoning.
    pub fn transience(&self) -> Transience {
        match self {
            // Obviously transient: a timeout or an open circuit breaker both
            // describe conditions that clear with time, not a fault in the
            // request itself.
            PaladinError::Timeout(_) => Transience::Transient,
            PaladinError::CircuitBreakerOpen => Transience::Transient,

            // Obviously permanent: retrying the exact same request reproduces
            // the exact same failure.
            PaladinError::ConfigurationError(_) => Transience::Permanent,
            PaladinError::StopWordDetected(_) => Transience::Permanent,
            PaladinError::GarrisonRequired => Transience::Permanent,
            PaladinError::MaxRetriesExceeded(_) => Transience::Permanent,

            // Unresolvable from a bare string: no typed field distinguishes
            // a transient cause from a permanent one.
            PaladinError::ExecutionError(_) => Transience::Unknown,
            PaladinError::LlmError(_) => Transience::Unknown,

            // The structured variant carries its own classification, set by
            // the adapter that produced it (typically `LlmError::transience()`).
            PaladinError::LlmFailure { transience, .. } => *transience,

            // Delegate to the inner Garrison error's own per-variant reasoning.
            PaladinError::GarrisonError(inner) => match inner {
                // Storage/tokenization failures may be a transient
                // network/database/service blip (the type's own rustdoc
                // documents both as "Retryable: Yes").
                GarrisonError::StorageError(_) => Transience::Transient,
                GarrisonError::TokenizationError(_) => Transience::Transient,
                // Serialization/not-found/configuration failures reproduce
                // identically on retry (the type's own rustdoc documents all
                // three as "Retryable: No").
                GarrisonError::SerializationError(_) => Transience::Permanent,
                GarrisonError::NotFound(_) => Transience::Permanent,
                GarrisonError::ConfigurationError(_) => Transience::Permanent,
                // Generic message with implementation-specific retryability
                // (the type's own rustdoc says exactly that) -- unresolvable
                // without inspecting the message, which classification must
                // never do.
                GarrisonError::Custom(_) => Transience::Unknown,
            },

            // Delegate to the inner Arsenal error's own per-variant reasoning.
            PaladinError::ArsenalError(inner) => match inner {
                // A registry lookup failure or a validation failure
                // reproduces identically on retry.
                ArsenalError::ToolNotFound(_) => Transience::Permanent,
                ArsenalError::InvalidArguments(_) => Transience::Permanent,
                // A tool call that timed out, or a transport-layer fault
                // (connection reset, DNS, socket errors), both describe
                // conditions that may clear on retry.
                ArsenalError::Timeout(_) => Transience::Transient,
                ArsenalError::TransportError(_) => Transience::Transient,
                // Credentials rejected by the remote MCP server reproduce
                // identically on retry without an operator fixing the
                // credential.
                ArsenalError::AuthFailed(_) => Transience::Permanent,
                // A protocol-level mismatch could be either a transient
                // framing hiccup or a genuine incompatibility -- not
                // obviously one or the other, so Unknown per D-05's default.
                ArsenalError::ProtocolError(_) => Transience::Unknown,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_configuration_error_message() {
        let error = PaladinError::ConfigurationError("Invalid parameter".to_string());
        assert_eq!(error.to_string(), "Configuration error: Invalid parameter");
    }

    #[test]
    fn test_timeout_error_message() {
        let error = PaladinError::Timeout(300);
        assert_eq!(error.to_string(), "Timeout after 300 seconds");
    }

    #[test]
    fn test_is_retryable() {
        assert!(PaladinError::LlmError("temp".to_string()).is_retryable());
        assert!(PaladinError::ExecutionError("temp".to_string()).is_retryable());
        assert!(!PaladinError::ConfigurationError("temp".to_string()).is_retryable());
        assert!(!PaladinError::Timeout(100).is_retryable());
    }

    #[test]
    fn test_is_terminal() {
        assert!(PaladinError::Timeout(100).is_terminal());
        assert!(PaladinError::CircuitBreakerOpen.is_terminal());
        assert!(PaladinError::GarrisonRequired.is_terminal());
        assert!(!PaladinError::LlmError("temp".to_string()).is_terminal());
    }

    #[test]
    fn test_garrison_error_conversion() {
        let garrison_error = GarrisonError::StorageError("test".to_string());
        let paladin_error: PaladinError = garrison_error.into();
        assert!(matches!(paladin_error, PaladinError::GarrisonError(_)));
    }

    /// One row per `PaladinError` variant (D-05), including every
    /// `GarrisonError`/`ArsenalError` inner variant as its own row, so a
    /// variant added later cannot silently inherit a neighbour's
    /// classification.
    #[test]
    fn paladin_error_transience_table() {
        use Transience::*;
        let cases: Vec<(PaladinError, Transience)> = vec![
            (PaladinError::Timeout(30), Transient),
            (PaladinError::CircuitBreakerOpen, Transient),
            (PaladinError::ConfigurationError("x".into()), Permanent),
            (PaladinError::StopWordDetected("x".into()), Permanent),
            (PaladinError::GarrisonRequired, Permanent),
            (PaladinError::MaxRetriesExceeded(3), Permanent),
            (PaladinError::ExecutionError("x".into()), Unknown),
            (PaladinError::LlmError("x".into()), Unknown),
            (
                PaladinError::LlmFailure {
                    transience: Transient,
                    status: Some(503),
                    provider: Some("openai".into()),
                    message: "x".into(),
                },
                Transient,
            ),
            (
                PaladinError::LlmFailure {
                    transience: Permanent,
                    status: Some(401),
                    provider: Some("openai".into()),
                    message: "x".into(),
                },
                Permanent,
            ),
            (
                PaladinError::LlmFailure {
                    transience: Unknown,
                    status: None,
                    provider: None,
                    message: "x".into(),
                },
                Unknown,
            ),
            (
                PaladinError::GarrisonError(GarrisonError::StorageError("x".into())),
                Transient,
            ),
            (
                PaladinError::GarrisonError(GarrisonError::TokenizationError("x".into())),
                Transient,
            ),
            (
                PaladinError::GarrisonError(GarrisonError::SerializationError("x".into())),
                Permanent,
            ),
            (
                PaladinError::GarrisonError(GarrisonError::NotFound("x".into())),
                Permanent,
            ),
            (
                PaladinError::GarrisonError(GarrisonError::ConfigurationError("x".into())),
                Permanent,
            ),
            (
                PaladinError::GarrisonError(GarrisonError::Custom("x".into())),
                Unknown,
            ),
            (
                PaladinError::ArsenalError(ArsenalError::ToolNotFound("x".into())),
                Permanent,
            ),
            (
                PaladinError::ArsenalError(ArsenalError::InvalidArguments("x".into())),
                Permanent,
            ),
            (
                PaladinError::ArsenalError(ArsenalError::Timeout(5)),
                Transient,
            ),
            (
                PaladinError::ArsenalError(ArsenalError::ProtocolError("x".into())),
                Unknown,
            ),
            (
                PaladinError::ArsenalError(ArsenalError::TransportError("x".into())),
                Transient,
            ),
            (
                PaladinError::ArsenalError(ArsenalError::AuthFailed("x".into())),
                Permanent,
            ),
        ];
        for (err, expected) in cases {
            assert_eq!(
                err.transience(),
                expected,
                "variant {err:?} classified as {:?}, expected {expected:?}",
                err.transience()
            );
        }
    }

    /// Every `is_retryable()`/`is_terminal()` answer is unchanged by this
    /// phase (X-03) -- including `true` for the new `LlmFailure` variant,
    /// the same blanket answer the legacy `LlmError(_)` arm always gave.
    #[test]
    fn legacy_retryability_predicates_are_unchanged() {
        assert!(PaladinError::LlmError("temp".to_string()).is_retryable());
        assert!(PaladinError::ExecutionError("temp".to_string()).is_retryable());
        assert!(!PaladinError::ConfigurationError("temp".to_string()).is_retryable());
        assert!(!PaladinError::Timeout(100).is_retryable());
        assert!(
            PaladinError::LlmFailure {
                transience: Transience::Unknown,
                status: None,
                provider: None,
                message: "temp".to_string(),
            }
            .is_retryable()
        );

        assert!(PaladinError::Timeout(100).is_terminal());
        assert!(PaladinError::CircuitBreakerOpen.is_terminal());
        assert!(PaladinError::GarrisonRequired.is_terminal());
        assert!(!PaladinError::LlmError("temp".to_string()).is_terminal());
        assert!(
            !PaladinError::LlmFailure {
                transience: Transience::Unknown,
                status: None,
                provider: None,
                message: "temp".to_string(),
            }
            .is_terminal()
        );
    }

    /// `LlmFailure`'s rendered `Display` is byte-identical to the legacy
    /// stringly `LlmError(String)` arm for the same message (D-02, X-03).
    #[test]
    fn llm_failure_display_matches_the_legacy_stringly_variant() {
        let message = "connection reset".to_string();
        let legacy = PaladinError::LlmError(message.clone());
        let structured = PaladinError::LlmFailure {
            transience: Transience::Transient,
            status: Some(503),
            provider: Some("openai".to_string()),
            message,
        };
        assert_eq!(legacy.to_string(), structured.to_string());
    }
}
