//! Integration tests for Phalanx pattern
//!
//! Tests end-to-end Phalanx execution with concurrent scenarios

use async_trait::async_trait;
use paladin::application::services::battalion::phalanx_service::PhalanxExecutionService;
use paladin::application::services::paladin::error::PaladinError;
use paladin::core::base::entity::node::Node;
use paladin::core::platform::container::battalion::phalanx::{AggregationStrategy, Phalanx};
use paladin::core::platform::container::battalion::{BattalionConfig, ErrorStrategy};
use paladin::core::platform::container::paladin::MaxLoops;
use paladin::core::platform::container::paladin::{Paladin, PaladinData, PaladinStatus};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, StopReason};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

/// Mock PaladinPort that simulates realistic concurrent Paladin behavior
struct IntegrationMockPaladinPort {
    execution_log: Arc<Mutex<Vec<String>>>,
    failure_config: Arc<Mutex<FailureConfig>>,
    delay_ms: u64,
}

#[derive(Clone, Debug)]
struct FailureConfig {
    fail_paladin_names: Vec<String>,
    output_overrides: HashMap<String, String>,
}

impl IntegrationMockPaladinPort {
    fn new() -> Self {
        Self {
            execution_log: Arc::new(Mutex::new(Vec::new())),
            failure_config: Arc::new(Mutex::new(FailureConfig {
                fail_paladin_names: Vec::new(),
                output_overrides: HashMap::new(),
            })),
            delay_ms: 10,
        }
    }

    fn with_delay(mut self, delay_ms: u64) -> Self {
        self.delay_ms = delay_ms;
        self
    }

    fn with_failures(self, paladin_names: Vec<String>) -> Self {
        self.failure_config.lock().unwrap().fail_paladin_names = paladin_names;
        self
    }

    fn with_output_overrides(self, overrides: HashMap<String, String>) -> Self {
        self.failure_config.lock().unwrap().output_overrides = overrides;
        self
    }

    fn get_execution_log(&self) -> Vec<String> {
        self.execution_log.lock().unwrap().clone()
    }
}

#[async_trait]
impl PaladinPort for IntegrationMockPaladinPort {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        // Log execution
        let log_entry = format!("Executing {}: {}", paladin.node.name, input);
        self.execution_log.lock().unwrap().push(log_entry);

        // Simulate processing delay
        tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;

        // Check if this Paladin should fail
        let config = self.failure_config.lock().unwrap();
        let should_fail = config.fail_paladin_names.contains(&paladin.node.name);

        if should_fail {
            return Err(PaladinError::ExecutionError(format!(
                "Simulated failure for {}",
                paladin.node.name
            )));
        }

        // Check for output override
        let output = if let Some(override_output) = config.output_overrides.get(&paladin.node.name)
        {
            override_output.clone()
        } else {
            format!("[{}]: Processed: {}", paladin.node.name, input)
        };

        Ok(PaladinResult {
            output,
            usage: paladin_ports::output::llm_port::TokenUsage::new(50, 0),
            execution_time_ms: self.delay_ms,
            loop_count: 1,
            stop_reason: StopReason::Completed,
            ..Default::default()
        })
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<
        tokio::sync::mpsc::Receiver<
            Result<paladin_ports::output::paladin_port::PaladinStreamChunk, PaladinError>,
        >,
        PaladinError,
    > {
        let (_tx, rx) = tokio::sync::mpsc::channel(1);
        Ok(rx)
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

fn create_paladin(name: &str, system_prompt: &str) -> Paladin {
    let data = PaladinData {
        system_prompt: system_prompt.to_string(),
        name: name.to_string(),
        user_name: "TestUser".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(3),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    Node::new(data, Some(name.to_string()))
}

#[tokio::test]
async fn test_phalanx_concurrent_execution_with_10_paladins() {
    // Setup: Create 10 Paladins for concurrent execution
    let paladins: Vec<Paladin> = (1..=10)
        .map(|i| create_paladin(&format!("Agent{}", i), &format!("Agent {} task", i)))
        .collect();

    let config = BattalionConfig::new("concurrent_10");
    let phalanx = Phalanx::new(paladins, config).expect("Failed to create phalanx");

    // Execute
    let mock_port = Arc::new(IntegrationMockPaladinPort::new());
    let service = PhalanxExecutionService::new(mock_port.clone());

    let start = Instant::now();
    let result = service.execute(&phalanx, "Test input").await;
    let duration = start.elapsed();

    // Assert
    assert!(result.is_ok(), "Phalanx should execute successfully");

    let battalion_result = result.unwrap();
    assert_eq!(battalion_result.paladin_results.len(), 10);
    assert_eq!(battalion_result.battalion_name, "concurrent_10");

    // Verify all 10 Paladins executed
    let log = mock_port.get_execution_log();
    assert_eq!(log.len(), 10);

    // Verify concurrent execution (should be much faster than sequential)
    // With 10ms delay each, concurrent should be ~10-50ms, sequential would be 100ms+
    assert!(
        duration.as_millis() < 100,
        "Concurrent execution should be fast, took {}ms",
        duration.as_millis()
    );
}

#[tokio::test]
async fn test_phalanx_collect_all_aggregation() {
    let agent1 = create_paladin("Agent1", "First agent");
    let agent2 = create_paladin("Agent2", "Second agent");
    let agent3 = create_paladin("Agent3", "Third agent");

    let phalanx = Phalanx::new(
        vec![agent1, agent2, agent3],
        BattalionConfig::new("collect_all_test"),
    )
    .unwrap()
    .with_aggregation(AggregationStrategy::CollectAll);

    let mock_port = Arc::new(IntegrationMockPaladinPort::new());
    let service = PhalanxExecutionService::new(mock_port.clone());

    let result = service.execute(&phalanx, "Analyze data").await.unwrap();

    // CollectAll waits for all Paladins
    assert_eq!(result.paladin_results.len(), 3);

    let log = mock_port.get_execution_log();
    assert_eq!(log.len(), 3);

    // Verify all agents executed
    assert!(log.iter().any(|l| l.contains("Agent1")));
    assert!(log.iter().any(|l| l.contains("Agent2")));
    assert!(log.iter().any(|l| l.contains("Agent3")));
}

#[tokio::test]
async fn test_phalanx_first_success_aggregation() {
    let agent1 = create_paladin("Agent1", "First agent");
    let agent2 = create_paladin("Agent2", "Second agent");
    let agent3 = create_paladin("Agent3", "Third agent");

    let phalanx = Phalanx::new(
        vec![agent1, agent2, agent3],
        BattalionConfig::new("first_success_test"),
    )
    .unwrap()
    .with_aggregation(AggregationStrategy::FirstSuccess);

    let mock_port = Arc::new(IntegrationMockPaladinPort::new());
    let service = PhalanxExecutionService::new(mock_port);

    let result = service.execute(&phalanx, "Quick task").await.unwrap();

    // FirstSuccess returns only one result
    assert_eq!(result.paladin_results.len(), 1);
    assert!(result.final_output.contains("Processed"));
}

#[tokio::test]
async fn test_phalanx_majority_aggregation_with_consensus() {
    let agent1 = create_paladin("Agent1", "Voter 1");
    let agent2 = create_paladin("Agent2", "Voter 2");
    let agent3 = create_paladin("Agent3", "Voter 3");
    let agent4 = create_paladin("Agent4", "Voter 4");
    let agent5 = create_paladin("Agent5", "Voter 5");

    let phalanx = Phalanx::new(
        vec![agent1, agent2, agent3, agent4, agent5],
        BattalionConfig::new("majority_test"),
    )
    .unwrap()
    .with_aggregation(AggregationStrategy::Majority);

    // Set up so 3 agents return "Option A", 2 return "Option B"
    let mut overrides = HashMap::new();
    overrides.insert("Agent1".to_string(), "Option A".to_string());
    overrides.insert("Agent2".to_string(), "Option A".to_string());
    overrides.insert("Agent3".to_string(), "Option A".to_string());
    overrides.insert("Agent4".to_string(), "Option B".to_string());
    overrides.insert("Agent5".to_string(), "Option B".to_string());

    let mock_port = Arc::new(IntegrationMockPaladinPort::new().with_output_overrides(overrides));
    let service = PhalanxExecutionService::new(mock_port);

    let result = service.execute(&phalanx, "Vote on option").await.unwrap();

    // Majority should select "Option A" (3/5 = 60%)
    assert_eq!(result.paladin_results.len(), 1);
    assert_eq!(result.final_output, "Option A");
}

#[tokio::test]
async fn test_phalanx_majority_no_consensus_fails() {
    let agent1 = create_paladin("Agent1", "Voter 1");
    let agent2 = create_paladin("Agent2", "Voter 2");
    let agent3 = create_paladin("Agent3", "Voter 3");

    let phalanx = Phalanx::new(
        vec![agent1, agent2, agent3],
        BattalionConfig::new("no_consensus_test"),
    )
    .unwrap()
    .with_aggregation(AggregationStrategy::Majority);

    // All different outputs - no majority
    let mut overrides = HashMap::new();
    overrides.insert("Agent1".to_string(), "Option A".to_string());
    overrides.insert("Agent2".to_string(), "Option B".to_string());
    overrides.insert("Agent3".to_string(), "Option C".to_string());

    let mock_port = Arc::new(IntegrationMockPaladinPort::new().with_output_overrides(overrides));
    let service = PhalanxExecutionService::new(mock_port);

    let result = service.execute(&phalanx, "Vote on option").await;

    // Should fail - no majority consensus
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("No majority consensus")
    );
}

#[tokio::test]
async fn test_phalanx_concurrency_limiting() {
    // Create 10 Paladins but limit to 3 concurrent
    let paladins: Vec<Paladin> = (1..=10)
        .map(|i| create_paladin(&format!("Agent{}", i), "Task"))
        .collect();

    let phalanx = Phalanx::new(paladins, BattalionConfig::new("limited_concurrency"))
        .unwrap()
        .with_max_concurrency(3);

    let mock_port = Arc::new(IntegrationMockPaladinPort::new().with_delay(50));
    let service = PhalanxExecutionService::new(mock_port);

    let start = Instant::now();
    let result = service.execute(&phalanx, "Test").await.unwrap();
    let duration = start.elapsed();

    // All 10 should complete
    assert_eq!(result.paladin_results.len(), 10);

    // With max 3 concurrent and 50ms delay each:
    // Best case: 10 Paladins / 3 concurrent = 4 batches * 50ms = ~200ms
    // Should be faster than sequential (500ms) but slower than fully concurrent (~50ms)
    assert!(
        duration.as_millis() >= 150,
        "Should respect concurrency limit"
    );
    assert!(
        duration.as_millis() < 400,
        "Should still be faster than sequential"
    );
}

#[tokio::test]
async fn test_phalanx_partial_failures_continue_on_error() {
    let paladins: Vec<Paladin> = (1..=5)
        .map(|i| create_paladin(&format!("Agent{}", i), "Task"))
        .collect();

    let config = BattalionConfig::new("partial_fail_test")
        .with_error_strategy(ErrorStrategy::ContinueOnError);

    let phalanx = Phalanx::new(paladins, config).unwrap();

    let mock_port = Arc::new(
        IntegrationMockPaladinPort::new()
            .with_failures(vec!["Agent2".to_string(), "Agent4".to_string()]),
    );
    let service = PhalanxExecutionService::new(mock_port.clone());

    let result = service.execute(&phalanx, "Test").await;

    // Should succeed despite failures
    assert!(result.is_ok());

    let battalion_result = result.unwrap();

    // All 5 attempted, but only 3 successful (Agent1, Agent3, Agent5)
    let log = mock_port.get_execution_log();
    assert_eq!(log.len(), 5);
    assert_eq!(battalion_result.paladin_results.len(), 3);
}

#[tokio::test]
async fn test_phalanx_timeout_enforcement() {
    let agent1 = create_paladin("SlowAgent1", "Slow task");
    let agent2 = create_paladin("SlowAgent2", "Slow task");

    let config = BattalionConfig::new("timeout_test").with_timeout(1); // 1 second

    let phalanx = Phalanx::new(vec![agent1, agent2], config).unwrap();

    // Use mock with 2 second delay (exceeds timeout)
    let mock_port = Arc::new(IntegrationMockPaladinPort::new().with_delay(2000));
    let service = PhalanxExecutionService::new(mock_port);

    let result = service.execute(&phalanx, "Test").await;

    // Should timeout
    assert!(result.is_err());

    match result.unwrap_err() {
        paladin::core::platform::container::battalion::BattalionError::Timeout(seconds) => {
            assert_eq!(seconds, 1);
        }
        _ => panic!("Expected Timeout error"),
    }
}

#[tokio::test]
async fn test_phalanx_cancellation_support() {
    let paladins: Vec<Paladin> = (1..=5)
        .map(|i| create_paladin(&format!("Agent{}", i), "Long task"))
        .collect();

    let phalanx = Phalanx::new(paladins, BattalionConfig::new("cancellation_test")).unwrap();

    let mock_port = Arc::new(IntegrationMockPaladinPort::new().with_delay(1000));
    let service = PhalanxExecutionService::new(mock_port);

    let cancellation_token = CancellationToken::new();
    let token_clone = cancellation_token.clone();

    // Cancel after 100ms
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        token_clone.cancel();
    });

    let result = service
        .execute_with_cancellation(&phalanx, "Test", cancellation_token)
        .await;

    // Should be cancelled
    assert!(result.is_err());

    match result.unwrap_err() {
        paladin::core::platform::container::battalion::BattalionError::Cancelled => {}
        e => panic!("Expected Cancelled error, got: {:?}", e),
    }
}

#[tokio::test]
async fn test_phalanx_performance_overhead() {
    // Performance test: Verify orchestration overhead is < 1 second
    let paladins: Vec<Paladin> = (1..=10)
        .map(|i| create_paladin(&format!("Fast{}", i), "Quick"))
        .collect();

    let phalanx = Phalanx::new(paladins, BattalionConfig::new("perf_test")).unwrap();

    // Use very fast mock (1ms delay per Paladin)
    let mock_port = Arc::new(IntegrationMockPaladinPort::new().with_delay(1));
    let service = PhalanxExecutionService::new(mock_port);

    let start = Instant::now();
    let result = service.execute(&phalanx, "Performance test").await;
    let duration = start.elapsed();

    assert!(result.is_ok());

    // Total time should be < 1 second (requirement from PRD)
    // With 10 Paladins at 1ms each + orchestration overhead
    assert!(
        duration.as_millis() < 1000,
        "Orchestration overhead too high: {}ms",
        duration.as_millis()
    );

    // Should also be reasonably fast (< 100ms for 10x 1ms operations)
    assert!(
        duration.as_millis() < 100,
        "Performance degraded: {}ms for 10 fast operations",
        duration.as_millis()
    );
}

#[tokio::test]
async fn test_phalanx_large_scale_concurrent_execution() {
    // Test with 50 Paladins to verify scalability
    let paladins: Vec<Paladin> = (1..=50)
        .map(|i| create_paladin(&format!("Agent{}", i), "Scalability test"))
        .collect();

    let phalanx = Phalanx::new(paladins, BattalionConfig::new("large_scale"))
        .unwrap()
        .with_max_concurrency(10);

    let mock_port = Arc::new(IntegrationMockPaladinPort::new().with_delay(10));
    let service = PhalanxExecutionService::new(mock_port.clone());

    let result = service.execute(&phalanx, "Scale test").await;

    assert!(result.is_ok());

    let battalion_result = result.unwrap();
    assert_eq!(battalion_result.paladin_results.len(), 50);

    // Verify all 50 executed
    let log = mock_port.get_execution_log();
    assert_eq!(log.len(), 50);
}

#[tokio::test]
async fn test_phalanx_mixed_success_and_failure() {
    let paladins: Vec<Paladin> = (1..=6)
        .map(|i| create_paladin(&format!("Agent{}", i), "Mixed"))
        .collect();

    let config =
        BattalionConfig::new("mixed_test").with_error_strategy(ErrorStrategy::ContinueOnError);

    let phalanx = Phalanx::new(paladins, config).unwrap();

    // Fail agents 2, 4, 6 (50% failure rate)
    let mock_port = Arc::new(IntegrationMockPaladinPort::new().with_failures(vec![
        "Agent2".to_string(),
        "Agent4".to_string(),
        "Agent6".to_string(),
    ]));
    let service = PhalanxExecutionService::new(mock_port);

    let result = service.execute(&phalanx, "Mixed test").await;

    assert!(result.is_ok());

    let battalion_result = result.unwrap();

    // 3 successful (Agent1, Agent3, Agent5)
    assert_eq!(battalion_result.paladin_results.len(), 3);
}
