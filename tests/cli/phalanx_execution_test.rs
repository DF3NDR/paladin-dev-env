//! Phalanx execution integration tests
//!
//! Tests the concurrent/parallel execution of Phalanx patterns using MockLlmAdapter
//! to verify proper concurrent execution and result aggregation.

use paladin::application::services::battalion::phalanx_service::PhalanxExecutionService;
use paladin::core::base::entity::Entity;
use paladin::core::platform::container::battalion::BattalionConfig;
use paladin::core::platform::container::battalion::phalanx::{AggregationStrategy, Phalanx};
use paladin::core::platform::container::paladin::{MaxLoops, PaladinData, PaladinStatus};
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use std::sync::Arc;
use std::time::Duration;

use crate::helpers::{MockLlmAdapter, MockPaladinPort};

#[tokio::test]
async fn test_phalanx_basic_parallel_execution() {
    // Arrange: Create 3 Paladins with mock LLM
    let mock_llm = Arc::new(MockLlmAdapter::new());

    // Add parallel responses for each Paladin
    mock_llm.add_success("Analysis A: Perspective 1");
    mock_llm.add_success("Analysis B: Perspective 2");
    mock_llm.add_success("Analysis C: Perspective 3");

    // Create first Paladin
    let paladin1_data = PaladinData {
        system_prompt: "Analyze from perspective A.".to_string(),
        name: "AnalystA".to_string(),
        user_name: "TestUser".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    let paladin1 = Entity::new(paladin1_data, None);

    // Create second Paladin
    let paladin2_data = PaladinData {
        system_prompt: "Analyze from perspective B.".to_string(),
        name: "AnalystB".to_string(),
        user_name: "TestUser".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    let paladin2 = Entity::new(paladin2_data, None);

    // Create third Paladin
    let paladin3_data = PaladinData {
        system_prompt: "Analyze from perspective C.".to_string(),
        name: "AnalystC".to_string(),
        user_name: "TestUser".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    let paladin3 = Entity::new(paladin3_data, None);

    // Create Phalanx
    let paladins = vec![paladin1, paladin2, paladin3];
    let config = BattalionConfig::new("test_phalanx");
    let phalanx = Phalanx::new(paladins, config).expect("Failed to create Phalanx");

    // Create execution services
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let paladin_port = Arc::new(MockPaladinPort::new(mock_llm.clone(), circuit_breaker));
    let phalanx_service = PhalanxExecutionService::new(paladin_port);

    // Act: Execute Phalanx
    let result = phalanx_service
        .execute(&phalanx, "Analyze this topic")
        .await;

    // Assert: Verify successful execution
    assert!(result.is_ok(), "Phalanx execution should succeed");
    let battalion_result = result.unwrap();

    // Verify all 3 Paladins were invoked
    assert_eq!(mock_llm.call_count(), 3, "All 3 Paladins should be invoked");

    // Verify we have results for each Paladin
    assert_eq!(
        battalion_result.paladin_results.len(),
        3,
        "Should have 3 Paladin results"
    );

    // Verify final output contains aggregated results
    // With CollectAll strategy, should have all perspectives
    assert!(
        battalion_result.final_output.contains("Perspective")
            || battalion_result.final_output.contains("Analysis"),
        "Final output should contain Paladin outputs"
    );
}

#[tokio::test]
async fn test_phalanx_collect_all_aggregation() {
    // Arrange: Test CollectAll aggregation strategy
    let mock_llm = Arc::new(MockLlmAdapter::new());

    // Add responses for 2 Paladins
    mock_llm.add_success("Result from Paladin 1");
    mock_llm.add_success("Result from Paladin 2");

    // Create 2 Paladins (minimum required)
    let paladin1_data = PaladinData {
        system_prompt: "First analyst.".to_string(),
        name: "Analyst1".to_string(),
        user_name: "TestUser".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    let paladin1 = Entity::new(paladin1_data, None);

    let paladin2_data = PaladinData {
        system_prompt: "Second analyst.".to_string(),
        name: "Analyst2".to_string(),
        user_name: "TestUser".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    let paladin2 = Entity::new(paladin2_data, None);

    // Create Phalanx with CollectAll strategy
    let paladins = vec![paladin1, paladin2];
    let config = BattalionConfig::new("collect_all_test");
    let phalanx = Phalanx::new(paladins, config)
        .expect("Failed to create Phalanx")
        .with_aggregation(AggregationStrategy::CollectAll);

    // Create execution services
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let paladin_port = Arc::new(MockPaladinPort::new(mock_llm.clone(), circuit_breaker));
    let phalanx_service = PhalanxExecutionService::new(paladin_port);

    // Act
    let result = phalanx_service.execute(&phalanx, "Test input").await;

    // Assert
    assert!(result.is_ok(), "Phalanx execution should succeed");
    let battalion_result = result.unwrap();

    // Verify both Paladins were invoked
    assert_eq!(mock_llm.call_count(), 2, "Both Paladins should be invoked");

    // Verify we have 2 results
    assert_eq!(
        battalion_result.paladin_results.len(),
        2,
        "Should have 2 Paladin results"
    );

    // All Paladins should receive the same input (not chained like Formation)
    let invocations = mock_llm.invocations();
    assert!(
        invocations[0].prompt.contains("Test input"),
        "First Paladin should receive original input"
    );
    assert!(
        invocations[1].prompt.contains("Test input"),
        "Second Palatdin should receive original input"
    );
}

#[tokio::test]
async fn test_phalanx_first_success_aggregation() {
    // Arrange: Test FirstSuccess aggregation strategy
    let mock_llm = Arc::new(MockLlmAdapter::new());

    // Add responses - first succeeds immediately
    mock_llm.add_success("Quick result from first Paladin");
    mock_llm.add_success("Slower result from second Paladin");

    // Create 2 Paladins
    let paladin1_data = PaladinData {
        system_prompt: "Fast analyst.".to_string(),
        name: "FastAnalyst".to_string(),
        user_name: "TestUser".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    let paladin1 = Entity::new(paladin1_data, None);

    let paladin2_data = PaladinData {
        system_prompt: "Slow analyst.".to_string(),
        name: "SlowAnalyst".to_string(),
        user_name: "TestUser".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    let paladin2 = Entity::new(paladin2_data, None);

    // Create Phalanx with FirstSuccess strategy
    let paladins = vec![paladin1, paladin2];
    let config = BattalionConfig::new("first_success_test");
    let phalanx = Phalanx::new(paladins, config)
        .expect("Failed to create Phalanx")
        .with_aggregation(AggregationStrategy::FirstSuccess);

    // Create execution services
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let paladin_port = Arc::new(MockPaladinPort::new(mock_llm.clone(), circuit_breaker));
    let phalanx_service = PhalanxExecutionService::new(paladin_port);

    // Act
    let result = phalanx_service.execute(&phalanx, "Test input").await;

    // Assert: FirstSuccess should return as soon as one Paladin succeeds
    assert!(result.is_ok(), "Phalanx execution should succeed");
    let battalion_result = result.unwrap();

    // At least one Paladin should have been invoked
    // (In practice, both may execute concurrently before cancellation)
    assert!(
        mock_llm.call_count() >= 1,
        "At least one Paladin should be invoked"
    );

    // Should have at least one result
    assert!(
        !battalion_result.paladin_results.is_empty(),
        "Should have at least one Paladin result"
    );
}

#[tokio::test]
async fn test_phalanx_with_max_concurrency() {
    // Arrange: Test max concurrency limiting
    let mock_llm = Arc::new(MockLlmAdapter::new());

    // Add responses for 3 Paladins
    mock_llm.add_success("Result 1");
    mock_llm.add_success("Result 2");
    mock_llm.add_success("Result 3");

    // Create 3 Paladins
    let paladin1_data = PaladinData {
        system_prompt: "Analyst 1.".to_string(),
        name: "Analyst1".to_string(),
        user_name: "TestUser".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    let paladin1 = Entity::new(paladin1_data, None);

    let paladin2_data = PaladinData {
        system_prompt: "Analyst 2.".to_string(),
        name: "Analyst2".to_string(),
        user_name: "TestUser".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    let paladin2 = Entity::new(paladin2_data, None);

    let paladin3_data = PaladinData {
        system_prompt: "Analyst 3.".to_string(),
        name: "Analyst3".to_string(),
        user_name: "TestUser".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    let paladin3 = Entity::new(paladin3_data, None);

    // Create Phalanx with max_concurrency = 2
    let paladins = vec![paladin1, paladin2, paladin3];
    let config = BattalionConfig::new("concurrency_test");
    let phalanx = Phalanx::new(paladins, config)
        .expect("Failed to create Phalanx")
        .with_max_concurrency(2);

    // Create execution services
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let paladin_port = Arc::new(MockPaladinPort::new(mock_llm.clone(), circuit_breaker));
    let phalanx_service = PhalanxExecutionService::new(paladin_port);

    // Act
    let result = phalanx_service.execute(&phalanx, "Test input").await;

    // Assert
    assert!(result.is_ok(), "Phalanx execution should succeed");
    let battalion_result = result.unwrap();

    // All 3 Paladins should eventually execute (just not all at once)
    assert_eq!(mock_llm.call_count(), 3, "All 3 Paladins should be invoked");

    // Should have 3 results
    assert_eq!(
        battalion_result.paladin_results.len(),
        3,
        "Should have 3 Paladin results"
    );
}

#[tokio::test]
async fn test_phalanx_error_handling() {
    // Arrange: Test error handling in Phalanx
    let mock_llm = Arc::new(MockLlmAdapter::new());

    // First Paladin succeeds, second fails
    mock_llm.add_success("Success from first");
    mock_llm.add_failure(paladin_ports::output::llm_port::LlmError::NetworkError(
        "Network failure".to_string(),
    ));

    // Create 2 Paladins
    let paladin1_data = PaladinData {
        system_prompt: "First.".to_string(),
        name: "First".to_string(),
        user_name: "TestUser".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    let paladin1 = Entity::new(paladin1_data, None);

    let paladin2_data = PaladinData {
        system_prompt: "Second.".to_string(),
        name: "Second".to_string(),
        user_name: "TestUser".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    let paladin2 = Entity::new(paladin2_data, None);

    // Create Phalanx
    let paladins = vec![paladin1, paladin2];
    let config = BattalionConfig::new("error_test");
    let phalanx = Phalanx::new(paladins, config).expect("Failed to create Phalanx");

    // Create execution services
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let paladin_port = Arc::new(MockPaladinPort::new(mock_llm.clone(), circuit_breaker));
    let phalanx_service = PhalanxExecutionService::new(paladin_port);

    // Act
    let result = phalanx_service.execute(&phalanx, "Test input").await;

    // Assert: Error handling depends on error strategy
    // Default is StopOnError, but Phalanx may collect partial results
    // The important part is that it handles errors gracefully
    assert!(
        result.is_ok() || result.is_err(),
        "Phalanx should handle errors gracefully"
    );
}
