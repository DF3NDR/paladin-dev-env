//! Formation execution integration tests
//!
//! Tests the sequential execution of Formation patterns using MockLlmAdapter
//! to verify proper output chaining and orchestration.

use paladin::application::services::battalion::formation_service::FormationExecutionService;
use paladin::core::base::entity::node::Node;
use paladin::core::platform::container::battalion::BattalionConfig;
use paladin::core::platform::container::battalion::formation::Formation;
use paladin::core::platform::container::paladin::{MaxLoops, PaladinData, PaladinStatus};
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use std::sync::Arc;
use std::time::Duration;

use crate::helpers::{MockLlmAdapter, MockPaladinPort};

#[tokio::test]
async fn test_formation_basic_sequential_execution() {
    // Arrange: Create 3 Paladins with mock LLM
    let mock_llm = Arc::new(MockLlmAdapter::new());

    // Add sequential responses for each Paladin
    mock_llm.add_success("Step 1: Data analyzed");
    mock_llm.add_success("Step 2: Analysis refined from previous step");
    mock_llm.add_success("Step 3: Final synthesis of all previous steps");

    // Create first Paladin (analyzer)
    let paladin1_data = PaladinData {
        system_prompt: "You are a data analyzer.".to_string(),
        name: "Analyzer".to_string(),
        user_name: "TestUser".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    let paladin1 = Node::new(paladin1_data, None);

    // Create second Paladin (refiner)
    let paladin2_data = PaladinData {
        system_prompt: "You refine analysis.".to_string(),
        name: "Refiner".to_string(),
        user_name: "TestUser".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    let paladin2 = Node::new(paladin2_data, None);

    // Create third Paladin (synthesizer)
    let paladin3_data = PaladinData {
        system_prompt: "You synthesize information.".to_string(),
        name: "Synthesizer".to_string(),
        user_name: "TestUser".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    let paladin3 = Node::new(paladin3_data, None);

    // Create Formation
    let paladins = vec![paladin1, paladin2, paladin3];
    let config = BattalionConfig::new("test_formation");
    let formation = Formation::new(paladins, config).expect("Failed to create Formation");

    // Create execution services
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let paladin_port = Arc::new(MockPaladinPort::new(mock_llm.clone(), circuit_breaker));
    let formation_service = FormationExecutionService::new(paladin_port);

    // Act: Execute Formation
    let result = formation_service
        .execute(&formation, "Analyze this data set")
        .await;

    // Assert: Verify successful execution
    assert!(result.is_ok(), "Formation execution should succeed");
    let battalion_result = result.unwrap();

    // Verify all 3 Paladins were invoked
    assert_eq!(mock_llm.call_count(), 3, "All 3 Paladins should be invoked");

    // Verify we have results for each Paladin
    assert_eq!(
        battalion_result.paladin_results.len(),
        3,
        "Should have 3 Paladin results"
    );

    // Verify final output contains the last step
    assert!(
        battalion_result.final_output.contains("Step 3")
            || battalion_result.final_output.contains("synthesis"),
        "Final output should contain last Paladin's output"
    );
}

#[tokio::test]
async fn test_formation_output_chaining() {
    // Arrange: Test that output from one Paladin feeds into next
    let mock_llm = Arc::new(MockLlmAdapter::new());

    // Add responses that show clear chaining
    mock_llm.add_success("OUTPUT_1: First processing done");
    mock_llm.add_success("OUTPUT_2: Received OUTPUT_1 and processed it");

    // Create 2 Paladins (minimum required)
    let paladin1_data = PaladinData {
        system_prompt: "Process the input.".to_string(),
        name: "FirstProcessor".to_string(),
        user_name: "TestUser".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    let paladin1 = Node::new(paladin1_data, None);

    let paladin2_data = PaladinData {
        system_prompt: "Process the previous output.".to_string(),
        name: "SecondProcessor".to_string(),
        user_name: "TestUser".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    let paladin2 = Node::new(paladin2_data, None);

    // Create Formation
    let paladins = vec![paladin1, paladin2];
    let config = BattalionConfig::new("chaining_test");
    let formation = Formation::new(paladins, config).expect("Failed to create Formation");

    // Create execution services
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let paladin_port = Arc::new(MockPaladinPort::new(mock_llm.clone(), circuit_breaker));
    let formation_service = FormationExecutionService::new(paladin_port);

    // Act
    let result = formation_service.execute(&formation, "Initial input").await;

    // Assert
    assert!(result.is_ok(), "Formation execution should succeed");
    assert_eq!(mock_llm.call_count(), 2, "Both Paladins should be invoked");

    // Verify invocation order by checking prompts
    let invocations = mock_llm.invocations();
    assert_eq!(invocations.len(), 2, "Should have 2 invocations");

    // First Paladin should receive initial input
    assert!(
        invocations[0].prompt.contains("Initial input"),
        "First Paladin should receive initial input"
    );

    // Second Paladin should receive output from first
    assert!(
        invocations[1].prompt.contains("OUTPUT_1")
            || invocations[1].prompt.contains("First processing"),
        "Second Paladin should receive output from first Paladin"
    );
}

#[tokio::test]
async fn test_formation_with_shared_context() {
    // Arrange: Test shared context injection
    let mock_llm = Arc::new(MockLlmAdapter::new());
    mock_llm.add_success("Response 1 with context");
    mock_llm.add_success("Response 2 with context");

    // Create 2 Paladins
    let paladin1_data = PaladinData {
        system_prompt: "Process input.".to_string(),
        name: "Paladin1".to_string(),
        user_name: "TestUser".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    let paladin1 = Node::new(paladin1_data, None);

    let paladin2_data = PaladinData {
        system_prompt: "Process input.".to_string(),
        name: "Paladin2".to_string(),
        user_name: "TestUser".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    let paladin2 = Node::new(paladin2_data, None);

    // Create Formation with shared context
    let paladins = vec![paladin1, paladin2];
    let config = BattalionConfig::new("context_test");
    let formation = Formation::new(paladins, config)
        .expect("Failed to create Formation")
        .with_shared_context("SHARED_CONTEXT: Use this in your analysis".to_string());

    // Create execution services
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let paladin_port = Arc::new(MockPaladinPort::new(mock_llm.clone(), circuit_breaker));
    let formation_service = FormationExecutionService::new(paladin_port);

    // Act
    let result = formation_service.execute(&formation, "Test input").await;

    // Assert
    assert!(result.is_ok(), "Formation execution should succeed");

    // Verify first Paladin received shared context
    let invocations = mock_llm.invocations();
    assert!(
        invocations[0].prompt.contains("SHARED_CONTEXT"),
        "First Paladin should receive shared context"
    );
}

#[tokio::test]
async fn test_formation_error_propagation() {
    // Arrange: Test error handling in Formation
    let mock_llm = Arc::new(MockLlmAdapter::new());

    // First Paladin succeeds, second fails
    mock_llm.add_success("First step succeeded");
    mock_llm.add_failure(paladin_ports::output::llm_port::LlmError::rate_limited(
        None,
    ));

    // Create 2 Paladins
    let paladin1_data = PaladinData {
        system_prompt: "First step.".to_string(),
        name: "FirstStep".to_string(),
        user_name: "TestUser".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    let paladin1 = Node::new(paladin1_data, None);

    let paladin2_data = PaladinData {
        system_prompt: "Second step.".to_string(),
        name: "SecondStep".to_string(),
        user_name: "TestUser".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    let paladin2 = Node::new(paladin2_data, None);

    // Create Formation
    let paladins = vec![paladin1, paladin2];
    let config = BattalionConfig::new("error_test");
    let formation = Formation::new(paladins, config).expect("Failed to create Formation");

    // Create execution services
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let paladin_port = Arc::new(MockPaladinPort::new(mock_llm.clone(), circuit_breaker));
    let formation_service = FormationExecutionService::new(paladin_port);

    // Act
    let result = formation_service.execute(&formation, "Test input").await;

    // Assert: Should propagate error based on error strategy
    // Default strategy is StopOnError, so execution should fail
    assert!(
        result.is_err(),
        "Formation should fail when a Paladin errors"
    );
}
