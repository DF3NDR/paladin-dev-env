//! Integration tests for Chain of Command Battalion pattern
//!
//! These tests verify end-to-end functionality of the Chain of Command
//! delegation pattern with realistic scenarios.

use async_trait::async_trait;
use paladin::application::services::battalion::chain_of_command_service::ChainOfCommandExecutionService;
use paladin::application::services::paladin::error::PaladinError;
use paladin::core::platform::container::battalion::BattalionConfig;
use paladin::core::platform::container::battalion::chain_of_command::{
    ChainOfCommand, DelegationStrategy,
};
use paladin::core::platform::container::paladin::{Paladin, PaladinData};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, StopReason};
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

/// Mock Paladin Port for integration testing
struct IntegrationMockPort {
    responses: Arc<TokioMutex<Vec<String>>>,
    call_log: Arc<TokioMutex<Vec<(String, String)>>>, // (paladin_name, input)
}

impl IntegrationMockPort {
    fn new(responses: Vec<String>) -> Self {
        Self {
            responses: Arc::new(TokioMutex::new(responses)),
            call_log: Arc::new(TokioMutex::new(Vec::new())),
        }
    }

    async fn get_call_log(&self) -> Vec<(String, String)> {
        self.call_log.lock().await.clone()
    }
}

#[async_trait]
impl PaladinPort for IntegrationMockPort {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        // Log the call
        let mut log = self.call_log.lock().await;
        log.push((paladin.node.name.clone(), input.to_string()));
        drop(log);

        // Get next response
        let mut responses = self.responses.lock().await;
        let response = if !responses.is_empty() {
            responses.remove(0)
        } else {
            format!("{} processed: {}", paladin.node.name, input)
        };

        Ok(PaladinResult {
            output: response,
            usage: paladin_ports::output::llm_port::TokenUsage::new(100, 0),
            execution_time_ms: 50,
            loop_count: 1,
            stop_reason: StopReason::Completed,
            ..Default::default()
        })
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<paladin_ports::output::paladin_port::PaladinStream, PaladinError> {
        unimplemented!("Streaming not used in tests")
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

fn create_test_paladin(name: &str, prompt: &str) -> Paladin {
    let data = PaladinData {
        system_prompt: prompt.to_string(),
        name: name.to_string(),
        user_name: "test_user".to_string(),
        ..Default::default()
    };
    Paladin::new(data, Some(name.to_string()))
}

#[tokio::test]
async fn test_automatic_delegation_end_to_end() {
    // Setup: Commander selects database specialist
    let mock_port = Arc::new(IntegrationMockPort::new(vec![
        "SELECT: database_specialist\nREASON: This task requires database expertise".to_string(),
        "Database query executed successfully: Found 42 records".to_string(),
    ]));
    let service = ChainOfCommandExecutionService::new(mock_port.clone());

    let commander = create_test_paladin("commander", "You coordinate database and API specialists");
    let db_specialist = create_test_paladin(
        "database_specialist",
        "You are an expert in database queries",
    );
    let api_specialist = create_test_paladin("api_specialist", "You are an expert in API calls");
    let config = BattalionConfig::default();

    let chain = ChainOfCommand::new(commander, vec![db_specialist, api_specialist], config)
        .expect("Should create chain")
        .with_strategy(DelegationStrategy::Automatic);

    // Execute
    let result = service
        .execute(&chain, "Query the users table for active users")
        .await;

    // Verify
    assert!(result.is_ok());
    let delegation_result = result.unwrap();
    assert_eq!(delegation_result.selected_specialists.len(), 1);
    assert_eq!(
        delegation_result.selected_specialists[0],
        "database_specialist"
    );
    assert!(delegation_result.reasoning.contains("database expertise"));
    assert!(delegation_result.outputs[0].contains("42 records"));

    // Verify call log
    let log = mock_port.get_call_log().await;
    assert_eq!(log.len(), 2); // Commander + database specialist
    assert_eq!(log[0].0, "commander");
    assert_eq!(log[1].0, "database_specialist");
}

#[tokio::test]
async fn test_broadcast_delegation_end_to_end() {
    // Setup: All specialists receive the same task
    let mock_port = Arc::new(IntegrationMockPort::new(vec![
        "Specialist 1 analysis complete".to_string(),
        "Specialist 2 analysis complete".to_string(),
        "Specialist 3 analysis complete".to_string(),
    ]));
    let service = ChainOfCommandExecutionService::new(mock_port.clone());

    let commander = create_test_paladin("commander", "You coordinate analysis specialists");
    let specialists: Vec<_> = (1..=3)
        .map(|i| create_test_paladin(&format!("specialist_{}", i), "Analysis expert"))
        .collect();
    let config = BattalionConfig::default();

    let chain = ChainOfCommand::new(commander, specialists, config)
        .expect("Should create chain")
        .with_strategy(DelegationStrategy::Broadcast);

    // Execute
    let result = service
        .execute(&chain, "Analyze this data from multiple perspectives")
        .await;

    // Verify
    assert!(result.is_ok());
    let delegation_result = result.unwrap();
    assert_eq!(delegation_result.selected_specialists.len(), 3);
    assert_eq!(delegation_result.outputs.len(), 3);

    // Verify all specialists were called
    let log = mock_port.get_call_log().await;
    assert_eq!(log.len(), 3);
    assert!(log.iter().any(|(name, _)| name == "specialist_1"));
    assert!(log.iter().any(|(name, _)| name == "specialist_2"));
    assert!(log.iter().any(|(name, _)| name == "specialist_3"));
}

#[tokio::test]
async fn test_round_robin_cycling_end_to_end() {
    // Setup: Three specialists should be called in rotation
    let mock_port = Arc::new(IntegrationMockPort::new(vec![
        "Task 1 result".to_string(),
        "Task 2 result".to_string(),
        "Task 3 result".to_string(),
        "Task 4 result".to_string(),
    ]));
    let service = ChainOfCommandExecutionService::new(mock_port.clone());

    let commander = create_test_paladin("commander", "You coordinate load-balanced specialists");
    let specialists: Vec<_> = (1..=3)
        .map(|i| create_test_paladin(&format!("specialist_{}", i), "Load balanced worker"))
        .collect();
    let config = BattalionConfig::default();

    let chain = ChainOfCommand::new(commander, specialists, config)
        .expect("Should create chain")
        .with_strategy(DelegationStrategy::RoundRobin);

    // Execute 4 times to verify cycling
    let result1 = service.execute(&chain, "Task 1").await.unwrap();
    let result2 = service.execute(&chain, "Task 2").await.unwrap();
    let result3 = service.execute(&chain, "Task 3").await.unwrap();
    let result4 = service.execute(&chain, "Task 4").await.unwrap();

    // Verify rotation: 1, 2, 3, 1
    assert_eq!(result1.selected_specialists[0], "specialist_1");
    assert_eq!(result2.selected_specialists[0], "specialist_2");
    assert_eq!(result3.selected_specialists[0], "specialist_3");
    assert_eq!(result4.selected_specialists[0], "specialist_1");

    // Verify call log shows correct order
    let log = mock_port.get_call_log().await;
    assert_eq!(log.len(), 4);
    assert_eq!(log[0].0, "specialist_1");
    assert_eq!(log[1].0, "specialist_2");
    assert_eq!(log[2].0, "specialist_3");
    assert_eq!(log[3].0, "specialist_1");
}

#[tokio::test]
async fn test_custom_delegation_end_to_end() {
    let mock_port = Arc::new(IntegrationMockPort::new(vec![
        "Custom logic result".to_string(),
    ]));
    let service = ChainOfCommandExecutionService::new(mock_port.clone());

    let commander = create_test_paladin("commander", "You coordinate custom delegation");
    let specialist = create_test_paladin("specialist_1", "Custom specialist");
    let config = BattalionConfig::default();

    let chain = ChainOfCommand::new(commander, vec![specialist], config)
        .expect("Should create chain")
        .with_strategy(DelegationStrategy::Custom(
            "Select based on task complexity".to_string(),
        ));

    // Execute
    let result = service.execute(&chain, "Complex task").await;

    // Verify
    assert!(result.is_ok());
    let delegation_result = result.unwrap();
    assert!(delegation_result.reasoning.contains("custom logic"));
    assert!(
        delegation_result
            .reasoning
            .contains("Select based on task complexity")
    );
}

#[tokio::test]
async fn test_automatic_delegation_with_multiple_specialists_selected() {
    // Commander selects multiple specialists
    let mock_port = Arc::new(IntegrationMockPort::new(vec![
        "SELECT: specialist_1, specialist_2\nREASON: Both database and cache need updating"
            .to_string(),
        "Database updated successfully".to_string(),
        "Cache cleared successfully".to_string(),
    ]));
    let service = ChainOfCommandExecutionService::new(mock_port.clone());

    let commander = create_test_paladin("commander", "You coordinate data operations");
    let specialist_1 = create_test_paladin("specialist_1", "Database specialist");
    let specialist_2 = create_test_paladin("specialist_2", "Cache specialist");
    let config = BattalionConfig::default();

    let chain = ChainOfCommand::new(commander, vec![specialist_1, specialist_2], config)
        .expect("Should create chain")
        .with_strategy(DelegationStrategy::Automatic);

    // Execute
    let result = service.execute(&chain, "Update user data").await;

    // Verify
    assert!(result.is_ok());
    let delegation_result = result.unwrap();
    assert_eq!(delegation_result.selected_specialists.len(), 2);
    assert!(
        delegation_result
            .selected_specialists
            .contains(&"specialist_1".to_string())
    );
    assert!(
        delegation_result
            .selected_specialists
            .contains(&"specialist_2".to_string())
    );
    assert_eq!(delegation_result.outputs.len(), 2);
}
