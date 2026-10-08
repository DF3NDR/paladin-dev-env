//! Paladin execution integration tests
//!
//! Tests the full execution flow of Paladin agents via CLI configuration,
//! using MockLlmAdapter to avoid external API dependencies.

use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::core::base::entity::node::Node;
use paladin::core::platform::container::paladin::{MaxLoops, PaladinData, PaladinStatus};
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin_ports::output::llm_port::LlmPort;
use std::sync::Arc;
use std::time::Duration;

// Test helper imports
use crate::helpers::{MockLlmAdapter, create_mock_with_responses};

#[tokio::test]
async fn test_paladin_basic_execution() {
    // Arrange: Create mock LLM adapter with a simple response
    let mock_llm = Arc::new(MockLlmAdapter::new());
    mock_llm.add_success("Hello! I am a test Paladin. I can help you with your questions.");

    // Create Paladin configuration
    let paladin_data = PaladinData {
        system_prompt: "You are a helpful AI assistant.".to_string(),
        name: "TestPaladin".to_string(),
        user_name: "TestUser".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    let paladin = Node::new(paladin_data, None);

    // Create circuit breaker and execution service
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let execution_service = PaladinExecutionService::new(
        mock_llm.clone() as Arc<dyn LlmPort>,
        circuit_breaker,
        None, // No garrison
        None, // No arsenal
    );

    // Act: Execute Paladin with test input
    let result = execution_service
        .execute(&paladin, "What can you help me with?")
        .await;

    // Assert: Check result
    assert!(result.is_ok(), "Paladin execution should succeed");
    let response = result.unwrap();
    assert!(
        response.output.contains("test Paladin"),
        "Response should contain expected text"
    );

    // Assert: Verify mock was called
    assert_eq!(mock_llm.call_count(), 1, "Mock LLM should be called once");

    let last_prompt = mock_llm.last_prompt();
    assert!(last_prompt.is_some(), "Should have recorded a prompt");
    assert!(
        last_prompt.unwrap().contains("What can you help me with?"),
        "Prompt should contain user query"
    );
}

#[tokio::test]
async fn test_paladin_multiple_loops() {
    // Arrange: Create mock with multiple responses for loop execution
    let mock_llm = Arc::new(MockLlmAdapter::new());
    mock_llm.add_success("First response");
    mock_llm.add_success("Second response");
    mock_llm.add_success("Final response");

    let paladin_data = PaladinData {
        system_prompt: "You are a test assistant.".to_string(),
        name: "MultiLoopPaladin".to_string(),
        user_name: "TestUser".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(3),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    let paladin = Node::new(paladin_data, None);

    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let execution_service = PaladinExecutionService::new(
        mock_llm.clone() as Arc<dyn LlmPort>,
        circuit_breaker,
        None,
        None,
    );

    // Act: Execute with multiple loops possible
    let result = execution_service.execute(&paladin, "Test query").await;

    // Assert: Should execute successfully
    assert!(result.is_ok(), "Multi-loop execution should succeed");

    // Note: The actual number of calls depends on the execution logic
    // The mock may be called once or multiple times depending on stop conditions
    assert!(
        mock_llm.call_count() >= 1,
        "Mock should be called at least once"
    );
}

#[tokio::test]
async fn test_paladin_with_stop_word() {
    // Arrange: Create mock that triggers stop word
    let mock_llm = Arc::new(MockLlmAdapter::new());
    mock_llm.add_success("This is the final answer. STOP");

    let paladin_data = PaladinData {
        system_prompt: "You are a test assistant.".to_string(),
        name: "StopWordPaladin".to_string(),
        user_name: "TestUser".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(5),
        stop_words: vec!["STOP".to_string()],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    let paladin = Node::new(paladin_data, None);

    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let execution_service = PaladinExecutionService::new(
        mock_llm.clone() as Arc<dyn LlmPort>,
        circuit_breaker,
        None,
        None,
    );

    // Act: Execute - should stop when encountering stop word
    let result = execution_service
        .execute(&paladin, "Execute until stop word")
        .await;

    // Assert: Stop word detection should return an error
    assert!(
        result.is_err(),
        "Execution with stop word should return error"
    );

    // Verify it's a StopWordDetected error
    if let Err(e) = result {
        let error_msg = format!("{:?}", e);
        assert!(
            error_msg.contains("StopWordDetected") || error_msg.contains("STOP"),
            "Error should indicate stop word was detected: {}",
            error_msg
        );
    }
}

#[tokio::test]
async fn test_paladin_error_handling() {
    // Arrange: Create mock that simulates an error
    let mock_llm = Arc::new(MockLlmAdapter::new());
    mock_llm.add_failure(paladin_ports::output::llm_port::LlmError::rate_limited(
        None,
    ));

    let paladin_data = PaladinData {
        system_prompt: "You are a test assistant.".to_string(),
        name: "ErrorPaladin".to_string(),
        user_name: "TestUser".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    let paladin = Node::new(paladin_data, None);

    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let execution_service = PaladinExecutionService::new(
        mock_llm.clone() as Arc<dyn LlmPort>,
        circuit_breaker,
        None,
        None,
    );

    // Act: Execute - should fail with error
    let result = execution_service
        .execute(&paladin, "This should fail")
        .await;

    // Assert: Should return error
    assert!(result.is_err(), "Execution should fail when LLM errors");
}

#[tokio::test]
async fn test_paladin_with_helper_factory() {
    // Test using helper factory function
    let mock_llm = create_mock_with_responses(vec!["Response 1", "Response 2", "Response 3"]);

    let paladin_data = PaladinData {
        system_prompt: "You are a helpful assistant.".to_string(),
        name: "FactoryPaladin".to_string(),
        user_name: "TestUser".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(3),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    let paladin = Node::new(paladin_data, None);

    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let execution_service = PaladinExecutionService::new(
        mock_llm.clone() as Arc<dyn LlmPort>,
        circuit_breaker,
        None,
        None,
    );

    // Act
    let result = execution_service.execute(&paladin, "Test query").await;

    // Assert
    assert!(
        result.is_ok(),
        "Execution with factory-created mock should succeed"
    );
    assert!(mock_llm.call_count() >= 1, "Mock should be invoked");
}

#[tokio::test]
async fn test_paladin_invocation_tracking() {
    // Arrange: Test that we can track invocations for debugging/testing
    let mock_llm = Arc::new(MockLlmAdapter::new());
    mock_llm.add_success("Test response");

    let paladin_data = PaladinData {
        system_prompt: "You are a test assistant.".to_string(),
        name: "TrackingPaladin".to_string(),
        user_name: "TestUser".to_string(),
        model: "gpt-4-turbo".to_string(), // Specific model
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    let paladin = Node::new(paladin_data, None);

    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let execution_service = PaladinExecutionService::new(
        mock_llm.clone() as Arc<dyn LlmPort>,
        circuit_breaker,
        None,
        None,
    );

    // Act
    let _result = execution_service
        .execute(&paladin, "Track this invocation")
        .await;

    // Assert: Check invocation details
    let invocations = mock_llm.invocations();
    assert_eq!(invocations.len(), 1, "Should have one invocation");

    let invocation = &invocations[0];
    assert_eq!(
        invocation.model, "gpt-4-turbo",
        "Should track correct model"
    );
    assert!(
        invocation.prompt.contains("Track this invocation"),
        "Should track correct prompt"
    );
}
