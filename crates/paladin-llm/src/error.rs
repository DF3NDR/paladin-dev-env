//! # LLM Provider Errors
//!
//! Defines the error type for all LLM provider adapters in this crate.
//! [`LlmProviderError`] is the crate-local error type; a `From` conversion
//! to [`paladin_ports::output::llm_port::LlmError`] is provided so errors
//! propagate cleanly across the port boundary.

use paladin_ports::output::llm_port::LlmError;
use thiserror::Error;

/// Errors that can occur in LLM provider adapters.
///
/// This type is used internally in `paladin-llm`. At the port boundary it is
/// converted into [`LlmError`] via the [`From`] implementation below.
#[derive(Debug, Error, Clone)]
pub enum LlmProviderError {
    /// The API key is missing or invalid.
    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    /// A network-level failure (connection, timeout, DNS, etc.).
    #[error("Network error: {0}")]
    NetworkError(String),

    /// The request was rate-limited by the provider.
    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    /// The prompt or request was rejected as invalid.
    #[error("Invalid prompt: {0}")]
    InvalidPrompt(String),

    /// The response could not be parsed or the provider returned unexpected data.
    #[error("Processing error: {0}")]
    ProcessingError(String),

    /// The token limit was exceeded for this request.
    #[error("Token limit exceeded")]
    TokenLimitExceeded,

    /// The requested model is not available from this provider.
    #[error("Model not available: {0}")]
    ModelNotAvailable(String),

    /// A request timed out.
    #[error("Timeout: {0}")]
    Timeout(String),

    /// Adapter configuration is missing or invalid.
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
}

impl From<LlmProviderError> for LlmError {
    fn from(err: LlmProviderError) -> Self {
        match err {
            LlmProviderError::AuthenticationError(msg) => LlmError::AuthenticationError(msg),
            LlmProviderError::NetworkError(msg) => LlmError::NetworkError(msg),
            LlmProviderError::RateLimitExceeded => LlmError::rate_limited(None),
            LlmProviderError::InvalidPrompt(msg) => LlmError::InvalidPrompt(msg),
            LlmProviderError::ProcessingError(msg) => LlmError::ProcessingError(msg),
            LlmProviderError::TokenLimitExceeded => LlmError::TokenLimitExceeded,
            LlmProviderError::ModelNotAvailable(msg) => LlmError::ModelNotAvailable(msg),
            LlmProviderError::Timeout(msg) => LlmError::Timeout(msg),
            LlmProviderError::ConfigurationError(msg) => {
                LlmError::ProcessingError(format!("Configuration error: {}", msg))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
//
// `impl From<LlmProviderError> for LlmError` had no caller anywhere in the
// tree before these tests -- the reason this file measured 0.00% line
// coverage in the Phase 3 entry measurement
// (`.planning/phases/03-verification-depth/03-coverage-measurement.md`:
// `error.rs` 13/13 lines missed). Recorded disposition (this plan's
// "Recorded decisions" section): exercised, not deleted -- the impl is a
// documented, exhaustive conversion this crate's own module doc names as
// the port-boundary error mapping, so a test is the cheaper, non-destructive
// way to close the 0% entry.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authentication_error_converts_to_authentication_error() {
        let err = LlmProviderError::AuthenticationError("bad api key".to_string());
        match LlmError::from(err) {
            LlmError::AuthenticationError(msg) => assert_eq!(msg, "bad api key"),
            other => panic!("expected LlmError::AuthenticationError, got {other:?}"),
        }
    }

    #[test]
    fn network_error_converts_to_network_error() {
        let err = LlmProviderError::NetworkError("connection refused".to_string());
        match LlmError::from(err) {
            LlmError::NetworkError(msg) => assert_eq!(msg, "connection refused"),
            other => panic!("expected LlmError::NetworkError, got {other:?}"),
        }
    }

    #[test]
    fn rate_limit_exceeded_converts_to_rate_limit_exceeded() {
        let err = LlmProviderError::RateLimitExceeded;
        match LlmError::from(err) {
            LlmError::RateLimitExceeded { .. } => {}
            other => panic!("expected a rate-limit error, got {other:?}"),
        }
    }

    #[test]
    fn invalid_prompt_converts_to_invalid_prompt() {
        let err = LlmProviderError::InvalidPrompt("prompt was empty".to_string());
        match LlmError::from(err) {
            LlmError::InvalidPrompt(msg) => assert_eq!(msg, "prompt was empty"),
            other => panic!("expected LlmError::InvalidPrompt, got {other:?}"),
        }
    }

    #[test]
    fn processing_error_converts_to_processing_error() {
        let err = LlmProviderError::ProcessingError("could not parse response".to_string());
        match LlmError::from(err) {
            LlmError::ProcessingError(msg) => assert_eq!(msg, "could not parse response"),
            other => panic!("expected LlmError::ProcessingError, got {other:?}"),
        }
    }

    #[test]
    fn token_limit_exceeded_converts_to_token_limit_exceeded() {
        let err = LlmProviderError::TokenLimitExceeded;
        match LlmError::from(err) {
            LlmError::TokenLimitExceeded => {}
            other => panic!("expected LlmError::TokenLimitExceeded, got {other:?}"),
        }
    }

    #[test]
    fn model_not_available_converts_to_model_not_available() {
        let err = LlmProviderError::ModelNotAvailable("gpt-9".to_string());
        match LlmError::from(err) {
            LlmError::ModelNotAvailable(msg) => assert_eq!(msg, "gpt-9"),
            other => panic!("expected LlmError::ModelNotAvailable, got {other:?}"),
        }
    }

    #[test]
    fn timeout_converts_to_timeout() {
        let err = LlmProviderError::Timeout("30s exceeded".to_string());
        match LlmError::from(err) {
            LlmError::Timeout(msg) => assert_eq!(msg, "30s exceeded"),
            other => panic!("expected LlmError::Timeout, got {other:?}"),
        }
    }

    #[test]
    fn configuration_error_converts_to_processing_error_with_prefixed_message() {
        // The only non-identity arm: ConfigurationError has no matching
        // LlmError variant, so it is folded into ProcessingError with a
        // "Configuration error: " prefix on the payload.
        let err = LlmProviderError::ConfigurationError("missing base_url".to_string());
        match LlmError::from(err) {
            LlmError::ProcessingError(msg) => {
                assert_eq!(msg, "Configuration error: missing base_url");
            }
            other => panic!("expected LlmError::ProcessingError, got {other:?}"),
        }
    }

    // -- Display strings -------------------------------------------------

    #[test]
    fn display_strings_render_as_documented_for_every_variant() {
        assert_eq!(
            LlmProviderError::AuthenticationError("x".to_string()).to_string(),
            "Authentication error: x"
        );
        assert_eq!(
            LlmProviderError::NetworkError("x".to_string()).to_string(),
            "Network error: x"
        );
        assert_eq!(
            LlmProviderError::RateLimitExceeded.to_string(),
            "Rate limit exceeded"
        );
        assert_eq!(
            LlmProviderError::InvalidPrompt("x".to_string()).to_string(),
            "Invalid prompt: x"
        );
        assert_eq!(
            LlmProviderError::ProcessingError("x".to_string()).to_string(),
            "Processing error: x"
        );
        assert_eq!(
            LlmProviderError::TokenLimitExceeded.to_string(),
            "Token limit exceeded"
        );
        assert_eq!(
            LlmProviderError::ModelNotAvailable("x".to_string()).to_string(),
            "Model not available: x"
        );
        assert_eq!(
            LlmProviderError::Timeout("x".to_string()).to_string(),
            "Timeout: x"
        );
        assert_eq!(
            LlmProviderError::ConfigurationError("x".to_string()).to_string(),
            "Configuration error: x"
        );
    }

    // -- Exhaustiveness witness -------------------------------------------
    //
    // No wildcard arm, deliberately: if a new `LlmProviderError` variant is
    // ever added without updating this match, the build breaks here rather
    // than the new variant silently escaping both the `From` impl and this
    // test module's coverage.
    fn assert_every_variant_is_named(err: LlmProviderError) {
        match err {
            LlmProviderError::AuthenticationError(_) => {}
            LlmProviderError::NetworkError(_) => {}
            LlmProviderError::RateLimitExceeded => {}
            LlmProviderError::InvalidPrompt(_) => {}
            LlmProviderError::ProcessingError(_) => {}
            LlmProviderError::TokenLimitExceeded => {}
            LlmProviderError::ModelNotAvailable(_) => {}
            LlmProviderError::Timeout(_) => {}
            LlmProviderError::ConfigurationError(_) => {}
        }
    }

    #[test]
    fn exhaustiveness_witness_covers_a_representative_variant() {
        // Calling it once proves it compiles and runs; the real guarantee
        // is the absence of a wildcard arm above, checked at compile time.
        assert_every_variant_is_named(LlmProviderError::RateLimitExceeded);
    }
}
