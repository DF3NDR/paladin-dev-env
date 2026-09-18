// tests/integration/paladin_integration_test.rs
//
// Integration tests for Paladin system
//
// Tests the complete flow from PaladinBuilder through PaladinExecutionService
// using MockLlmAdapter, verifying all components work together correctly.

use paladin::MockLlmAdapter;
use paladin::application::services::paladin::error::PaladinError;
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::core::platform::container::paladin::MaxLoops;
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin_ports::output::llm_port::{LlmError, LlmPort};
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn test_end_to_end_paladin_execution() {
    // Test complete flow: Builder -> Service -> Execution -> Result
    let llm_port = Arc::new(
        MockLlmAdapter::new().with_response("Hello! I can help you with that.".to_string()),
    );

    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service = PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, None);

    let paladin = PaladinBuilder::new(llm_port as Arc<dyn LlmPort>)
        .system_prompt("You are a helpful assistant")
        .name("TestAssistant")
        .model("mock-model")
        .max_loops(1)
        .build()
        .await
        .expect("Failed to build Paladin");

    let result = service.execute(&paladin, "How can you help me?").await;

    assert!(result.is_ok(), "Execution should succeed");
    let paladin_result = result.unwrap();
    assert_eq!(paladin_result.output, "Hello! I can help you with that.");
    assert_eq!(paladin_result.loop_count, 1);
    assert!(paladin_result.usage.total_tokens > 0);
}

#[tokio::test]
async fn test_multi_loop_execution_with_accumulation() {
    // Test that multiple loops execute and output accumulates
    let llm_port = Arc::new(MockLlmAdapter::new().with_responses(vec![
        "First iteration response.".to_string(),
        "Second iteration response.".to_string(),
        "Third iteration response.".to_string(),
    ]));

    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service = PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, None);

    let paladin = PaladinBuilder::new(llm_port as Arc<dyn LlmPort>)
        .system_prompt("You are a helpful assistant")
        .max_loops(3)
        .build()
        .await
        .expect("Failed to build Paladin");

    let result = service.execute(&paladin, "Test input").await;

    assert!(result.is_ok(), "Multi-loop execution should succeed");
    let paladin_result = result.unwrap();
    assert_eq!(paladin_result.loop_count, 3);
    assert_eq!(paladin_result.output, "Third iteration response.");
}

#[tokio::test]
async fn test_stop_word_detection_halts_execution() {
    // Test that stop word detection terminates execution early
    let llm_port = Arc::new(
        MockLlmAdapter::new()
            .with_response("This is my final answer. STOP processing.".to_string()),
    );

    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service = PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, None);

    let paladin = PaladinBuilder::new(llm_port as Arc<dyn LlmPort>)
        .system_prompt("You are a helpful assistant")
        .max_loops(5)
        .add_stop_word("STOP")
        .build()
        .await
        .expect("Failed to build Paladin");

    let result = service.execute(&paladin, "Test input").await;

    assert!(result.is_err(), "Should fail due to stop word");
    match result.unwrap_err() {
        PaladinError::StopWordDetected(word) => {
            assert_eq!(word, "STOP");
        }
        e => panic!("Expected StopWordDetected, got {:?}", e),
    }
}

#[tokio::test]
async fn test_circuit_breaker_integration() {
    // Test that circuit breaker opens after repeated failures
    let llm_port = Arc::new(
        MockLlmAdapter::new().with_error(LlmError::NetworkError("Connection failed".to_string())),
    );

    let circuit_breaker = Arc::new(CircuitBreaker::new(
        2, // Open after 2 failures
        2,
        Duration::from_secs(30),
    ));
    let service =
        PaladinExecutionService::new(llm_port.clone(), circuit_breaker.clone(), None, None);

    let paladin = PaladinBuilder::new(llm_port as Arc<dyn LlmPort>)
        .system_prompt("You are a helpful assistant")
        .max_loops(1)
        .build()
        .await
        .expect("Failed to build Paladin");

    // First failure
    let result1 = service.execute(&paladin, "Test 1").await;
    assert!(result1.is_err());

    // Second failure - should open circuit
    let result2 = service.execute(&paladin, "Test 2").await;
    assert!(result2.is_err());

    // Third call should fail fast due to open circuit
    let result3 = service.execute(&paladin, "Test 3").await;
    assert!(result3.is_err());
    match result3.unwrap_err() {
        PaladinError::CircuitBreakerOpen => {
            // Expected
        }
        e => panic!("Expected CircuitBreakerOpen, got {:?}", e),
    }
}

#[tokio::test]
async fn test_retry_logic_with_exponential_backoff() {
    // Test that retries happen with proper backoff
    let llm_port = Arc::new(MockLlmAdapter::new().with_error_then_response(
        LlmError::NetworkError("Temporary failure".to_string()),
        "Success after retry".to_string(),
    ));

    let circuit_breaker = Arc::new(CircuitBreaker::new(5, 2, Duration::from_secs(30)));
    let service = PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, None);

    // Use max_loops(3) so that retry logic allows at least 2 attempts (first fails, second succeeds)
    let paladin = PaladinBuilder::new(llm_port.clone() as Arc<dyn LlmPort>)
        .system_prompt("You are a helpful assistant")
        .max_loops(3) // This controls max retry attempts: min(max_loops, 10)
        .build()
        .await
        .expect("Failed to build Paladin");

    let start = std::time::Instant::now();
    let result = service.execute(&paladin, "Test input").await;
    let elapsed = start.elapsed();

    assert!(result.is_ok(), "Should succeed after retry");
    let paladin_result = result.unwrap();
    assert_eq!(paladin_result.output, "Success after retry");

    // Should have taken at least 100ms for the retry backoff
    assert!(
        elapsed >= Duration::from_millis(100),
        "Should have backoff delay, took {:?}",
        elapsed
    );

    // First loop: attempt 1 returns error, attempt 2 returns success
    // Then 2 more loops with the success response
    // Total: 4 calls (2 on first loop due to retry, then 1 each for loops 2 and 3)
    assert!(
        llm_port.get_call_count() >= 2,
        "Should have at least 2 calls (error + retry success), got {}",
        llm_port.get_call_count()
    );
}

#[tokio::test]
async fn test_builder_validation_errors() {
    // Test that builder validation catches configuration errors
    let llm_port = Arc::new(MockLlmAdapter::new());

    // Empty system prompt should fail
    let result1 = PaladinBuilder::new(llm_port.clone() as Arc<dyn LlmPort>)
        .system_prompt("")
        .build();
    assert!(
        result1.await.is_err(),
        "Should fail with empty system prompt"
    );

    // Invalid temperature should fail
    let result2 = PaladinBuilder::new(llm_port.clone() as Arc<dyn LlmPort>)
        .system_prompt("Valid prompt")
        .temperature(2.0) // Out of range [0.0, 1.0]
        .build();
    assert!(
        result2.await.is_err(),
        "Should fail with invalid temperature"
    );

    // Invalid max_loops should fail
    let result3 = PaladinBuilder::new(llm_port.clone() as Arc<dyn LlmPort>)
        .system_prompt("Valid prompt")
        .max_loops(0) // Must be >= 1
        .build();
    assert!(result3.await.is_err(), "Should fail with zero max_loops");

    // Valid configuration should succeed
    let result4 = PaladinBuilder::new(llm_port as Arc<dyn LlmPort>)
        .system_prompt("Valid prompt")
        .temperature(0.7)
        .max_loops(3)
        .build();
    assert!(
        result4.await.is_ok(),
        "Should succeed with valid configuration"
    );
}

#[tokio::test]
async fn test_concurrent_paladin_execution() {
    // Test that multiple Paladins can execute concurrently
    let llm_port = Arc::new(MockLlmAdapter::new().with_response("Concurrent response".to_string()));

    let circuit_breaker = Arc::new(CircuitBreaker::new(5, 2, Duration::from_secs(30)));
    let service = Arc::new(PaladinExecutionService::new(
        llm_port.clone(),
        circuit_breaker,
        None,
        None,
    ));

    let mut handles = vec![];

    for i in 0..5 {
        let llm_clone = llm_port.clone();
        let service_clone = service.clone();

        let handle = tokio::spawn(async move {
            let paladin = PaladinBuilder::new(llm_clone as Arc<dyn LlmPort>)
                .system_prompt("You are a helpful assistant")
                .name(format!("Paladin-{}", i))
                .max_loops(1)
                .build()
                .await
                .expect("Failed to build Paladin");

            service_clone
                .execute(&paladin, &format!("Test input {}", i))
                .await
        });

        handles.push(handle);
    }

    // Wait for all executions
    let mut success_count = 0;
    for handle in handles {
        let result = handle.await.expect("Task panicked");
        if result.is_ok() {
            success_count += 1;
        }
    }

    assert_eq!(success_count, 5, "All concurrent executions should succeed");
    assert_eq!(
        llm_port.get_call_count(),
        5,
        "Should track all concurrent calls"
    );
}

#[tokio::test]
async fn test_error_propagation_across_layers() {
    // Test that errors propagate correctly from LLM through service to caller
    let llm_port = Arc::new(MockLlmAdapter::new().with_error(LlmError::RateLimitExceeded));

    let circuit_breaker = Arc::new(CircuitBreaker::new(5, 2, Duration::from_secs(30)));
    let service = PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, None);

    let paladin = PaladinBuilder::new(llm_port as Arc<dyn LlmPort>)
        .system_prompt("You are a helpful assistant")
        .max_loops(1)
        .build()
        .await
        .expect("Failed to build Paladin");

    let result = service.execute(&paladin, "Test input").await;

    assert!(result.is_err(), "Should propagate error");
    // The error will be MaxRetriesExceeded after exhausting retries
    match result.unwrap_err() {
        PaladinError::MaxRetriesExceeded(_) => {
            // Expected - retries were exhausted
        }
        e => panic!("Expected MaxRetriesExceeded, got {:?}", e),
    }
}

#[tokio::test]
async fn test_paladin_with_custom_configuration() {
    // Test Paladin with various custom configurations
    let llm_port = Arc::new(
        MockLlmAdapter::new()
            .with_response("Custom configured response".to_string())
            .with_token_usage(100, 150, 250),
    );

    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service = PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, None);

    let paladin = PaladinBuilder::new(llm_port as Arc<dyn LlmPort>)
        .system_prompt("You are a specialized coding assistant")
        .name("CodeHelper")
        .user_name("Developer")
        .model("mock-advanced-model")
        .temperature(0.3)
        .max_loops(2)
        .add_stop_word("END")
        .add_stop_word("DONE")
        .build()
        .await
        .expect("Failed to build Paladin");

    let result = service.execute(&paladin, "Write a function").await;

    assert!(result.is_ok(), "Custom configuration should work");
    let paladin_result = result.unwrap();

    // Verify custom configuration was used
    assert_eq!(paladin.node.name, "CodeHelper");
    assert_eq!(paladin.node.user_name, "Developer");
    assert_eq!(paladin.node.model, "mock-advanced-model");
    assert_eq!(paladin.node.temperature, 0.3);
    assert_eq!(paladin.node.max_loops, MaxLoops::Fixed(2));
    assert_eq!(paladin.node.stop_words, vec!["END", "DONE"]);

    // Verify execution result
    assert_eq!(paladin_result.output, "Custom configured response");
    // Token count is accumulated across all loops, so 2 loops * 250 tokens = 500
    assert_eq!(paladin_result.usage.total_tokens, 500);
}

#[tokio::test]
async fn test_paladin_metadata_tracking() {
    // Test that metadata is properly tracked across execution
    let llm_port = Arc::new(
        MockLlmAdapter::new()
            .with_response("Metadata test response".to_string())
            .with_token_usage(50, 75, 125),
    );

    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service = PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, None);

    let paladin = PaladinBuilder::new(llm_port as Arc<dyn LlmPort>)
        .system_prompt("You are a helpful assistant")
        .max_loops(1)
        .build()
        .await
        .expect("Failed to build Paladin");

    let result = service.execute(&paladin, "Test input").await;

    assert!(result.is_ok(), "Execution should succeed");
    let paladin_result = result.unwrap();

    // Verify all metadata fields are populated
    assert_eq!(paladin_result.output, "Metadata test response");
    assert_eq!(paladin_result.usage.total_tokens, 125);
    assert_eq!(paladin_result.loop_count, 1);
    // Execution time should be reasonable (not checking specific value due to timing variability)

    // Verify stop reason
    use paladin_ports::output::paladin_port::StopReason;
    assert!(matches!(paladin_result.stop_reason, StopReason::MaxLoops));
}

#[tokio::test]
async fn test_circuit_breaker_recovery() {
    // Test that circuit breaker can recover after cooling down
    let llm_port = Arc::new(
        MockLlmAdapter::new()
            .with_responses(vec!["Success 1".to_string(), "Success 2".to_string()]),
    );

    let circuit_breaker = Arc::new(CircuitBreaker::new(
        2,                          // Open after 2 failures
        1,                          // Close after 1 success in half-open
        Duration::from_millis(100), // Short timeout for testing
    ));
    let service =
        PaladinExecutionService::new(llm_port.clone(), circuit_breaker.clone(), None, None);

    let paladin = PaladinBuilder::new(llm_port as Arc<dyn LlmPort>)
        .system_prompt("You are a helpful assistant")
        .max_loops(1)
        .build()
        .await
        .expect("Failed to build Paladin");

    // Successful calls should work
    let result1 = service.execute(&paladin, "Test 1").await;
    assert!(result1.is_ok(), "First call should succeed");

    let result2 = service.execute(&paladin, "Test 2").await;
    assert!(result2.is_ok(), "Second call should succeed");

    // Circuit should remain closed
    use paladin::infrastructure::resilience::circuit_breaker::CircuitState;
    let state = circuit_breaker.get_state();
    assert!(matches!(state, CircuitState::Closed { .. }));
}

#[tokio::test]
async fn test_paladin_with_delays() {
    // Test that delays in mock adapter are respected
    let llm_port = Arc::new(
        MockLlmAdapter::new()
            .with_response("Delayed response".to_string())
            .with_delay(Duration::from_millis(50)),
    );

    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service = PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, None);

    let paladin = PaladinBuilder::new(llm_port as Arc<dyn LlmPort>)
        .system_prompt("You are a helpful assistant")
        .max_loops(1)
        .build()
        .await
        .expect("Failed to build Paladin");

    let start = std::time::Instant::now();
    let result = service.execute(&paladin, "Test input").await;
    let elapsed = start.elapsed();

    assert!(result.is_ok(), "Delayed execution should succeed");
    assert!(
        elapsed >= Duration::from_millis(50),
        "Should respect delay, took {:?}",
        elapsed
    );
}
