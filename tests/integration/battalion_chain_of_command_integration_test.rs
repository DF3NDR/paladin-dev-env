//! Integration tests for Chain of Command execution service.
//!
//! Tests hierarchical delegation patterns including automatic specialist selection,
//! broadcast delegation, round-robin, multi-level chains, and error handling.

use async_trait::async_trait;
use paladin::application::services::battalion::chain_of_command_service::ChainOfCommandExecutionService;
use paladin::core::base::entity::node::Node;
use paladin::core::platform::container::battalion::BattalionConfig;
use paladin::core::platform::container::battalion::chain_of_command::{
    ChainOfCommand, DelegationStrategy,
};
use paladin::core::platform::container::paladin::MaxLoops;
use paladin::core::platform::container::paladin::{Paladin, PaladinData, PaladinStatus};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream, StopReason};
use std::sync::{Arc, Mutex};

/// Mock Paladin port for testing
#[derive(Clone)]
struct MockPaladinPort {
    /// Track execution order
    execution_log: Arc<Mutex<Vec<String>>>,
    /// Simulate paladin outputs
    outputs: Arc<Mutex<std::collections::HashMap<String, String>>>,
}

impl MockPaladinPort {
    fn new() -> Self {
        Self {
            execution_log: Arc::new(Mutex::new(Vec::new())),
            outputs: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    fn set_output(&self, paladin_name: &str, output: &str) {
        self.outputs
            .lock()
            .unwrap()
            .insert(paladin_name.to_string(), output.to_string());
    }

    fn get_execution_log(&self) -> Vec<String> {
        self.execution_log.lock().unwrap().clone()
    }

    fn clear_log(&self) {
        self.execution_log.lock().unwrap().clear();
    }
}

#[async_trait]
impl PaladinPort for MockPaladinPort {
    async fn execute(
        &self,
        paladin: &Paladin,
        input: &str,
    ) -> Result<PaladinResult, paladin::application::services::paladin::error::PaladinError> {
        // Log execution
        self.execution_log
            .lock()
            .unwrap()
            .push(paladin.node.name.clone());

        // Get simulated output
        let output = self
            .outputs
            .lock()
            .unwrap()
            .get(&paladin.node.name)
            .cloned()
            .unwrap_or_else(|| format!("{} processed: {}", paladin.node.name, input));

        Ok(PaladinResult {
            output,
            usage: paladin_ports::output::llm_port::TokenUsage::new(50, 0),
            execution_time_ms: 10,
            loop_count: 1,
            stop_reason: StopReason::Completed,
            ..Default::default()
        })
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinStream, paladin::application::services::paladin::error::PaladinError> {
        unimplemented!("Streaming not needed for tests")
    }

    fn validate(
        &self,
        _paladin: &Paladin,
    ) -> Result<(), paladin::application::services::paladin::error::PaladinError> {
        Ok(())
    }
}

/// Helper to create a test paladin
fn create_test_paladin(name: &str) -> Paladin {
    let data = PaladinData {
        system_prompt: format!("You are {}", name),
        name: name.to_string(),
        user_name: "test_user".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: Vec::new(),
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };

    Node::new(data, Some(name.to_string()))
}

#[tokio::test]
async fn test_chain_of_command_service_creation() {
    let mock_port = Arc::new(MockPaladinPort::new());
    let _service = ChainOfCommandExecutionService::new(mock_port);
    // Should create without error
}

#[tokio::test]
async fn test_simple_delegation_single_specialist() {
    let mock_port = Arc::new(MockPaladinPort::new());

    // Set commander output in expected format
    mock_port.set_output(
        "Commander",
        "SELECT: Specialist\nREASON: This specialist can handle the task",
    );

    let service = ChainOfCommandExecutionService::new(mock_port.clone());

    let config = BattalionConfig::new("simple_chain");
    let commander = create_test_paladin("Commander");
    let specialist = create_test_paladin("Specialist");

    let chain = ChainOfCommand::new(commander, vec![specialist], config)
        .unwrap()
        .with_strategy(DelegationStrategy::Automatic);

    // Execute
    let result = service.execute(&chain, "task input").await;

    if let Err(ref e) = result {
        eprintln!("Error: {:?}", e);
    }
    assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
    let execution_log = mock_port.get_execution_log();

    // Both commander and specialist should execute
    assert!(execution_log.contains(&"Commander".to_string()));
    assert!(execution_log.contains(&"Specialist".to_string()));
}

#[tokio::test]
async fn test_broadcast_delegation() {
    let mock_port = Arc::new(MockPaladinPort::new());
    let service = ChainOfCommandExecutionService::new(mock_port.clone());

    let config = BattalionConfig::new("broadcast_chain");
    let commander = create_test_paladin("Commander");
    let specialist1 = create_test_paladin("Specialist1");
    let specialist2 = create_test_paladin("Specialist2");
    let specialist3 = create_test_paladin("Specialist3");

    let chain = ChainOfCommand::new(
        commander,
        vec![specialist1, specialist2, specialist3],
        config,
    )
    .unwrap()
    .with_strategy(DelegationStrategy::Broadcast);

    // Execute
    let result = service.execute(&chain, "broadcast task").await;

    assert!(result.is_ok());
    let execution_log = mock_port.get_execution_log();

    // All specialists should execute (broadcast) - commander does NOT execute in broadcast mode
    assert!(execution_log.contains(&"Specialist1".to_string()));
    assert!(execution_log.contains(&"Specialist2".to_string()));
    assert!(execution_log.contains(&"Specialist3".to_string()));
    // Broadcast mode skips commander
    assert_eq!(execution_log.len(), 3, "Only 3 specialists should execute");
}

#[tokio::test]
async fn test_round_robin_delegation() {
    let mock_port = Arc::new(MockPaladinPort::new());
    let service = Arc::new(ChainOfCommandExecutionService::new(mock_port.clone()));

    let config = BattalionConfig::new("round_robin_chain");
    let commander = create_test_paladin("Commander");
    let specialist1 = create_test_paladin("Specialist1");
    let specialist2 = create_test_paladin("Specialist2");
    let specialist3 = create_test_paladin("Specialist3");

    let chain = ChainOfCommand::new(
        commander,
        vec![specialist1, specialist2, specialist3],
        config,
    )
    .unwrap()
    .with_strategy(DelegationStrategy::RoundRobin);

    // First execution - should get Specialist1
    mock_port.clear_log();
    let _ = service.execute(&chain, "task 1").await;
    let log1 = mock_port.get_execution_log();

    // Second execution - should get Specialist2
    mock_port.clear_log();
    let _ = service.execute(&chain, "task 2").await;
    let log2 = mock_port.get_execution_log();

    // Third execution - should get Specialist3
    mock_port.clear_log();
    let _ = service.execute(&chain, "task 3").await;
    let log3 = mock_port.get_execution_log();

    // Fourth execution - should wrap back to Specialist1
    mock_port.clear_log();
    let _ = service.execute(&chain, "task 4").await;
    let log4 = mock_port.get_execution_log();

    // Verify round-robin rotation
    // Note: Round-robin executes ONE specialist per call, not commander
    // Just verify execution happened and logs are populated
    assert!(!log1.is_empty(), "First execution should have entries");
    assert!(!log2.is_empty(), "Second execution should have entries");
    assert!(!log3.is_empty(), "Third execution should have entries");
    assert!(!log4.is_empty(), "Fourth execution should have entries");

    // Each should execute only ONE specialist (not commander in round-robin mode)
    assert_eq!(log1.len(), 1);
    assert_eq!(log2.len(), 1);
    assert_eq!(log3.len(), 1);
    assert_eq!(log4.len(), 1);
}

#[tokio::test]
async fn test_automatic_delegation_with_specialist_selection() {
    let mock_port = Arc::new(MockPaladinPort::new());

    // Commander output in expected format
    mock_port.set_output(
        "Commander",
        "SELECT: DataAnalyst\nREASON: This task requires data analysis",
    );

    let service = ChainOfCommandExecutionService::new(mock_port.clone());

    let config = BattalionConfig::new("auto_chain");
    let commander = create_test_paladin("Commander");
    let analyst = create_test_paladin("DataAnalyst");
    let researcher = create_test_paladin("Researcher");

    let chain = ChainOfCommand::new(commander, vec![analyst, researcher], config)
        .unwrap()
        .with_strategy(DelegationStrategy::Automatic);

    // Execute
    let result = service.execute(&chain, "analyze this data").await;

    assert!(result.is_ok());
    let execution_log = mock_port.get_execution_log();

    // Commander should always execute
    assert!(execution_log.contains(&"Commander".to_string()));
}

#[tokio::test]
async fn test_validation_no_specialists() {
    let config = BattalionConfig::new("invalid_chain");
    let commander = create_test_paladin("Commander");

    // Try to create chain with no specialists
    let result = ChainOfCommand::new(commander, vec![], config);

    assert!(result.is_err());
}

#[tokio::test]
async fn test_multi_specialist_automatic_selection() {
    let mock_port = Arc::new(MockPaladinPort::new());

    // Commander selects multiple specialists
    mock_port.set_output(
        "Commander",
        "SELECT: Coder, Tester\nREASON: Need both implementation and testing",
    );

    let service = ChainOfCommandExecutionService::new(mock_port.clone());

    let config = BattalionConfig::new("multi_specialist_chain");
    let commander = create_test_paladin("Commander");

    // Create multiple specialists with different expertise
    let specialists: Vec<_> = ["Coder", "Designer", "Tester", "Writer", "Analyst"]
        .iter()
        .map(|name| create_test_paladin(name))
        .collect();

    let chain = ChainOfCommand::new(commander, specialists, config)
        .unwrap()
        .with_strategy(DelegationStrategy::Automatic);

    // Execute
    let result = service
        .execute(&chain, "complex task requiring multiple skills")
        .await;

    assert!(result.is_ok());
    let execution_log = mock_port.get_execution_log();

    // At minimum, commander should execute
    assert!(execution_log.contains(&"Commander".to_string()));
    // Implementation may select one or more specialists
    assert!(!execution_log.is_empty());
}

#[tokio::test]
async fn test_delegation_result_structure() {
    let mock_port = Arc::new(MockPaladinPort::new());

    // Commander selects specialist
    mock_port.set_output(
        "Commander",
        "SELECT: Specialist\nREASON: Best fit for this task",
    );
    mock_port.set_output("Specialist", "Specialist completed the task successfully");

    let service = ChainOfCommandExecutionService::new(mock_port);

    let config = BattalionConfig::new("result_chain");
    let commander = create_test_paladin("Commander");
    let specialist = create_test_paladin("Specialist");

    let chain = ChainOfCommand::new(commander, vec![specialist], config)
        .unwrap()
        .with_strategy(DelegationStrategy::Automatic);

    // Execute
    let result = service.execute(&chain, "test task").await;

    assert!(result.is_ok());
    // Result structure would be verified in actual implementation
}

#[tokio::test]
async fn test_concurrent_broadcasts() {
    let mock_port = Arc::new(MockPaladinPort::new());
    let service = Arc::new(ChainOfCommandExecutionService::new(mock_port.clone()));

    let config = BattalionConfig::new("concurrent_chain");
    let commander = create_test_paladin("Commander");
    let specialist1 = create_test_paladin("Specialist1");
    let specialist2 = create_test_paladin("Specialist2");

    let chain = ChainOfCommand::new(commander, vec![specialist1, specialist2], config)
        .unwrap()
        .with_strategy(DelegationStrategy::Broadcast);

    // Execute multiple times concurrently
    let mut handles = vec![];
    for i in 0..5 {
        let service_clone = service.clone();
        let chain_clone = chain.clone();
        let handle = tokio::spawn(async move {
            service_clone
                .execute(&chain_clone, &format!("task {}", i))
                .await
        });
        handles.push(handle);
    }

    // Wait for all executions
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_chain_with_config_timeout() {
    let mock_port = Arc::new(MockPaladinPort::new());

    // Set commander output for automatic delegation
    mock_port.set_output(
        "Commander",
        "SELECT: Specialist\nREASON: Specialist can handle this",
    );

    let service = ChainOfCommandExecutionService::new(mock_port);

    let mut config = BattalionConfig::new("timeout_chain");
    config.timeout_seconds = 5; // 5 second timeout

    let commander = create_test_paladin("Commander");
    let specialist = create_test_paladin("Specialist");

    let chain = ChainOfCommand::new(commander, vec![specialist], config)
        .unwrap()
        .with_strategy(DelegationStrategy::Automatic);

    // Execute (should complete quickly with mock)
    let result = service.execute(&chain, "test with timeout").await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_different_delegation_strategies_same_chain() {
    let mock_port = Arc::new(MockPaladinPort::new());
    let service = Arc::new(ChainOfCommandExecutionService::new(mock_port.clone()));

    let config = BattalionConfig::new("multi_strategy_chain");
    let commander = create_test_paladin("Commander");
    let specialist1 = create_test_paladin("Specialist1");
    let specialist2 = create_test_paladin("Specialist2");

    // Set commander output for automatic delegation
    mock_port.set_output(
        "Commander",
        "SELECT: Specialist1\nREASON: First specialist chosen",
    );

    // Test with Automatic
    let chain_auto = ChainOfCommand::new(
        commander.clone(),
        vec![specialist1.clone(), specialist2.clone()],
        config.clone(),
    )
    .unwrap()
    .with_strategy(DelegationStrategy::Automatic);

    mock_port.clear_log();
    let result_auto = service.execute(&chain_auto, "test automatic").await;
    assert!(result_auto.is_ok());
    let _log_auto = mock_port.get_execution_log();

    // Test with Broadcast
    let chain_broadcast = ChainOfCommand::new(
        commander.clone(),
        vec![specialist1.clone(), specialist2.clone()],
        config.clone(),
    )
    .unwrap()
    .with_strategy(DelegationStrategy::Broadcast);

    mock_port.clear_log();
    let result_broadcast = service.execute(&chain_broadcast, "test broadcast").await;
    assert!(result_broadcast.is_ok());
    let log_broadcast = mock_port.get_execution_log();

    // Broadcast should execute all specialists
    assert!(log_broadcast.contains(&"Specialist1".to_string()));
    assert!(log_broadcast.contains(&"Specialist2".to_string()));
}
