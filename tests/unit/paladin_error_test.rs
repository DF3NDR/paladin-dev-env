use paladin::application::services::paladin::error::PaladinError;
use paladin_core::platform::container::transience::Transience;

/// An `LlmFailure` carrying the given classification.
fn llm_failure(transience: Transience, message: &str) -> PaladinError {
    PaladinError::LlmFailure {
        transience,
        status: None,
        provider: Some("openai".to_string()),
        message: message.to_string(),
    }
}

#[test]
fn test_paladin_error_messages() {
    // Test each error variant has correct message formatting
    let config_err = PaladinError::ConfigurationError("bad config".to_string());
    assert_eq!(config_err.to_string(), "Configuration error: bad config");

    let exec_err = PaladinError::ExecutionError("execution failed".to_string());
    assert_eq!(exec_err.to_string(), "Execution error: execution failed");

    let llm_err = llm_failure(Transience::Transient, "rate limited");
    assert_eq!(llm_err.to_string(), "LLM error: rate limited");

    let timeout_err = PaladinError::Timeout(120);
    assert_eq!(timeout_err.to_string(), "Timeout after 120 seconds");

    let stop_word_err = PaladinError::StopWordDetected("STOP".to_string());
    assert_eq!(stop_word_err.to_string(), "Stop word detected: STOP");

    let circuit_err = PaladinError::CircuitBreakerOpen;
    assert_eq!(
        circuit_err.to_string(),
        "Circuit breaker open: too many failures"
    );

    let retry_err = PaladinError::MaxRetriesExceeded(5);
    assert_eq!(retry_err.to_string(), "Maximum retry attempts (5) exceeded");
}

#[test]
fn test_error_transience_split() {
    // Worth retrying
    assert_eq!(
        PaladinError::Timeout(100).transience(),
        Transience::Transient
    );
    assert_eq!(
        PaladinError::CircuitBreakerOpen.transience(),
        Transience::Transient
    );
    assert_eq!(
        llm_failure(Transience::Transient, "temp").transience(),
        Transience::Transient
    );

    // Not worth retrying: the same request fails the same way
    assert_eq!(
        PaladinError::ConfigurationError("temp".to_string()).transience(),
        Transience::Permanent
    );
    assert_eq!(
        PaladinError::StopWordDetected("STOP".to_string()).transience(),
        Transience::Permanent
    );
    assert_eq!(
        PaladinError::MaxRetriesExceeded(3).transience(),
        Transience::Permanent
    );
    assert_eq!(
        llm_failure(Transience::Permanent, "bad key").transience(),
        Transience::Permanent
    );

    // Cannot be classified from a bare string
    assert_eq!(
        PaladinError::ExecutionError("temp".to_string()).transience(),
        Transience::Unknown
    );
}

#[test]
fn test_error_classification_ignores_message_text() {
    // The classification is the carried field, never a substring of the text.
    let permanent = llm_failure(Transience::Permanent, "429 rate limit timeout 503");
    assert_eq!(permanent.transience(), Transience::Permanent);

    let transient = llm_failure(Transience::Transient, "");
    assert_eq!(transient.transience(), Transience::Transient);
}

#[test]
fn test_error_debug_formatting() {
    let error = PaladinError::ExecutionError("test error".to_string());
    let debug_str = format!("{:?}", error);
    assert!(debug_str.contains("ExecutionError"));
    assert!(debug_str.contains("test error"));
}

#[test]
fn test_all_error_variants_covered() {
    // Ensure we can construct all variants
    let _config = PaladinError::ConfigurationError("test".to_string());
    let _exec = PaladinError::ExecutionError("test".to_string());
    let _llm = llm_failure(Transience::Unknown, "test");
    let _timeout = PaladinError::Timeout(100);
    let _stop = PaladinError::StopWordDetected("STOP".to_string());
    let _circuit = PaladinError::CircuitBreakerOpen;
    let _retry = PaladinError::MaxRetriesExceeded(3);
}
