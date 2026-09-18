//! Unit tests for ChainOfCommandExecutionService
//!
//! Tests follow TDD Red-Green-Refactor methodology:
//! 1. Write failing test (Red)
//! 2. Implement minimal code to pass (Green)
//! 3. Refactor while keeping tests green

use paladin::application::services::battalion::chain_of_command_service::{
    ChainOfCommandExecutionService, DelegationResult,
};
use paladin::application::services::paladin::error::PaladinError;
use paladin::core::platform::container::battalion::BattalionConfig;
use paladin::core::platform::container::battalion::chain_of_command::ChainOfCommand;
use paladin::core::platform::container::paladin::{Paladin, PaladinData};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult};
use std::sync::Arc;

// Test helpers
mod helpers {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Mutex;

    /// Mock PaladinPort for testing
    pub struct MockPaladinPort {
        pub call_count: Arc<Mutex<usize>>,
        pub responses: Vec<String>,
    }

    impl MockPaladinPort {
        pub fn new(responses: Vec<String>) -> Self {
            Self {
                call_count: Arc::new(Mutex::new(0)),
                responses,
            }
        }

        pub fn simple(response: &str) -> Self {
            Self::new(vec![response.to_string()])
        }
    }

    #[async_trait]
    impl PaladinPort for MockPaladinPort {
        async fn execute(
            &self,
            _paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinResult, PaladinError> {
            let mut count = self.call_count.lock().unwrap();
            let response = if *count < self.responses.len() {
                self.responses[*count].clone()
            } else {
                "default response".to_string()
            };
            *count += 1;

            Ok(PaladinResult {
                output: response,
                usage: paladin_ports::output::llm_port::TokenUsage::new(0, 0),
                execution_time_ms: 100,
                loop_count: 1,
                stop_reason: paladin_ports::output::paladin_port::StopReason::Completed,
                ..Default::default()
            })
        }

        async fn execute_stream(
            &self,
            _paladin: &Paladin,
            _input: &str,
        ) -> Result<paladin_ports::output::paladin_port::PaladinStream, PaladinError> {
            unimplemented!("streaming not needed for tests")
        }

        fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
            Ok(())
        }
    }

    pub fn create_test_paladin(name: &str) -> Paladin {
        let data = PaladinData {
            system_prompt: format!("{} system prompt", name),
            name: name.to_string(),
            user_name: "test_user".to_string(),
            ..Default::default()
        };
        Paladin::new(data, Some(name.to_string()))
    }
}

use helpers::*;

/// Tests for ChainOfCommandExecutionService construction
mod service_construction_tests {
    use super::*;

    #[test]
    fn test_service_new_with_valid_port() {
        let mock_port = Arc::new(MockPaladinPort::simple("response"));
        let service = ChainOfCommandExecutionService::new(mock_port);

        // Service should be created successfully
        assert!(std::mem::size_of_val(&service) > 0);
    }

    #[test]
    fn test_service_new_stores_port() {
        let mock_port = Arc::new(MockPaladinPort::simple("response"));
        let service = ChainOfCommandExecutionService::new(mock_port.clone());

        // Service should store the port (validated implicitly by successful creation)
        assert!(std::mem::size_of_val(&service) > 0);
    }
}

/// Tests for service validation
mod service_validation_tests {
    use super::*;

    #[tokio::test]
    async fn test_validate_chain_of_command() {
        let mock_port = Arc::new(MockPaladinPort::simple("response"));
        let service = ChainOfCommandExecutionService::new(mock_port);

        let commander = create_test_paladin("commander");
        let specialist = create_test_paladin("specialist");
        let config = BattalionConfig::default();

        let chain = ChainOfCommand::new(commander, vec![specialist], config)
            .expect("Should create valid chain");

        // Validation should succeed
        let result = service.validate(&chain).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_empty_chain_fails() {
        let commander = create_test_paladin("commander");
        let config = BattalionConfig::default();

        // Creating chain with no specialists should fail
        let result = ChainOfCommand::new(commander, vec![], config);
        assert!(result.is_err());
    }
}

/// Tests for DelegationResult structure
mod delegation_result_tests {
    use super::*;

    #[test]
    fn test_delegation_result_creation() {
        let result = DelegationResult {
            selected_specialists: vec!["specialist_1".to_string()],
            reasoning: "Test reasoning".to_string(),
            outputs: vec!["output_1".to_string()],
        };

        assert_eq!(result.selected_specialists.len(), 1);
        assert_eq!(result.reasoning, "Test reasoning");
        assert_eq!(result.outputs.len(), 1);
    }

    #[test]
    fn test_delegation_result_with_multiple_specialists() {
        let result = DelegationResult {
            selected_specialists: vec![
                "specialist_1".to_string(),
                "specialist_2".to_string(),
                "specialist_3".to_string(),
            ],
            reasoning: "Broadcast to all specialists".to_string(),
            outputs: vec![
                "output_1".to_string(),
                "output_2".to_string(),
                "output_3".to_string(),
            ],
        };

        assert_eq!(result.selected_specialists.len(), 3);
        assert_eq!(result.outputs.len(), 3);
    }
}

/// Tests for Automatic delegation strategy
mod automatic_delegation_tests {
    use super::*;
    use paladin::core::platform::container::battalion::chain_of_command::DelegationStrategy;

    #[tokio::test]
    async fn test_automatic_delegation_selects_single_specialist() {
        // Commander response indicates selection of specialist_1
        let mock_port = Arc::new(MockPaladinPort::new(vec![
            "SELECT: specialist_1\nREASON: This task requires database expertise".to_string(),
            "Query result: 42 records found".to_string(),
        ]));
        let service = ChainOfCommandExecutionService::new(mock_port.clone());

        let commander = create_test_paladin("commander");
        let specialist_1 = create_test_paladin("specialist_1");
        let specialist_2 = create_test_paladin("specialist_2");
        let config = BattalionConfig::default();

        let chain = ChainOfCommand::new(commander, vec![specialist_1, specialist_2], config)
            .expect("Should create valid chain")
            .with_strategy(DelegationStrategy::Automatic);

        let result = service.execute(&chain, "Query the database").await;

        assert!(result.is_ok());
        let delegation_result = result.unwrap();
        assert_eq!(delegation_result.selected_specialists.len(), 1);
        assert_eq!(delegation_result.selected_specialists[0], "specialist_1");
        assert!(delegation_result.reasoning.contains("database expertise"));
        assert_eq!(delegation_result.outputs.len(), 1);
    }

    #[tokio::test]
    async fn test_automatic_delegation_selects_multiple_specialists() {
        // Commander response indicates selection of multiple specialists
        let mock_port = Arc::new(MockPaladinPort::new(vec![
            "SELECT: specialist_1, specialist_2\nREASON: This task requires both database and API expertise".to_string(),
            "Database result: success".to_string(),
            "API result: success".to_string(),
        ]));
        let service = ChainOfCommandExecutionService::new(mock_port.clone());

        let commander = create_test_paladin("commander");
        let specialist_1 = create_test_paladin("specialist_1");
        let specialist_2 = create_test_paladin("specialist_2");
        let config = BattalionConfig::default();

        let chain = ChainOfCommand::new(commander, vec![specialist_1, specialist_2], config)
            .expect("Should create valid chain")
            .with_strategy(DelegationStrategy::Automatic);

        let result = service.execute(&chain, "Fetch and store data").await;

        assert!(result.is_ok());
        let delegation_result = result.unwrap();
        assert_eq!(delegation_result.selected_specialists.len(), 2);
        assert!(delegation_result.reasoning.contains("database and API"));
        assert_eq!(delegation_result.outputs.len(), 2);
    }

    #[tokio::test]
    async fn test_automatic_delegation_with_invalid_selection() {
        // Commander response indicates selection of non-existent specialist
        let mock_port = Arc::new(MockPaladinPort::new(vec![
            "SELECT: specialist_unknown\nREASON: This specialist doesn't exist".to_string(),
        ]));
        let service = ChainOfCommandExecutionService::new(mock_port);

        let commander = create_test_paladin("commander");
        let specialist_1 = create_test_paladin("specialist_1");
        let config = BattalionConfig::default();

        let chain = ChainOfCommand::new(commander, vec![specialist_1], config)
            .expect("Should create valid chain")
            .with_strategy(DelegationStrategy::Automatic);

        let result = service.execute(&chain, "Do something").await;

        // Should return error for invalid specialist selection
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_automatic_delegation_commander_formats_context() {
        // Commander should receive specialist descriptions in context
        let mock_port = Arc::new(MockPaladinPort::new(vec![
            "SELECT: specialist_1\nREASON: Best match for the task".to_string(),
            "Task completed".to_string(),
        ]));
        let service = ChainOfCommandExecutionService::new(mock_port.clone());

        let commander = create_test_paladin("commander");
        let specialist_1 = create_test_paladin("specialist_1");
        let config = BattalionConfig::default();

        let chain = ChainOfCommand::new(commander, vec![specialist_1], config)
            .expect("Should create valid chain")
            .with_strategy(DelegationStrategy::Automatic);

        let result = service.execute(&chain, "Analyze data").await;

        assert!(result.is_ok());
        // Verify commander was called (check call count)
        let call_count = mock_port.call_count.lock().unwrap();
        assert!(*call_count >= 2); // Commander + at least one specialist
    }
}

/// Tests for Broadcast delegation strategy
mod broadcast_delegation_tests {
    use super::*;
    use paladin::core::platform::container::battalion::chain_of_command::DelegationStrategy;

    #[tokio::test]
    async fn test_broadcast_executes_all_specialists() {
        // All specialists should receive the input
        let mock_port = Arc::new(MockPaladinPort::new(vec![
            "specialist_1 result".to_string(),
            "specialist_2 result".to_string(),
            "specialist_3 result".to_string(),
        ]));
        let service = ChainOfCommandExecutionService::new(mock_port.clone());

        let commander = create_test_paladin("commander");
        let specialist_1 = create_test_paladin("specialist_1");
        let specialist_2 = create_test_paladin("specialist_2");
        let specialist_3 = create_test_paladin("specialist_3");
        let config = BattalionConfig::default();

        let chain = ChainOfCommand::new(
            commander,
            vec![specialist_1, specialist_2, specialist_3],
            config,
        )
        .expect("Should create valid chain")
        .with_strategy(DelegationStrategy::Broadcast);

        let result = service.execute(&chain, "Process this data").await;

        assert!(result.is_ok());
        let delegation_result = result.unwrap();
        assert_eq!(delegation_result.selected_specialists.len(), 3);
        assert_eq!(delegation_result.outputs.len(), 3);
        assert!(delegation_result.reasoning.contains("all specialists"));
    }

    #[tokio::test]
    async fn test_broadcast_with_single_specialist() {
        let mock_port = Arc::new(MockPaladinPort::simple("result"));
        let service = ChainOfCommandExecutionService::new(mock_port);

        let commander = create_test_paladin("commander");
        let specialist = create_test_paladin("specialist_1");
        let config = BattalionConfig::default();

        let chain = ChainOfCommand::new(commander, vec![specialist], config)
            .expect("Should create valid chain")
            .with_strategy(DelegationStrategy::Broadcast);

        let result = service.execute(&chain, "Do task").await;

        assert!(result.is_ok());
        let delegation_result = result.unwrap();
        assert_eq!(delegation_result.selected_specialists.len(), 1);
        assert_eq!(delegation_result.outputs.len(), 1);
    }

    #[tokio::test]
    async fn test_broadcast_executes_concurrently() {
        // Verify all specialists are executed (via call count)
        let mock_port = Arc::new(MockPaladinPort::new(vec![
            "result_1".to_string(),
            "result_2".to_string(),
            "result_3".to_string(),
            "result_4".to_string(),
            "result_5".to_string(),
        ]));
        let service = ChainOfCommandExecutionService::new(mock_port.clone());

        let commander = create_test_paladin("commander");
        let specialists: Vec<_> = (1..=5)
            .map(|i| create_test_paladin(&format!("specialist_{}", i)))
            .collect();
        let config = BattalionConfig::default();

        let chain = ChainOfCommand::new(commander, specialists, config)
            .expect("Should create valid chain")
            .with_strategy(DelegationStrategy::Broadcast);

        let result = service.execute(&chain, "Parallel task").await;

        assert!(result.is_ok());
        let delegation_result = result.unwrap();
        assert_eq!(delegation_result.selected_specialists.len(), 5);
        assert_eq!(delegation_result.outputs.len(), 5);

        // All specialists should have been called
        let call_count = mock_port.call_count.lock().unwrap();
        assert_eq!(*call_count, 5);
    }
}

/// Tests for RoundRobin delegation strategy
mod round_robin_delegation_tests {
    use super::*;
    use paladin::core::platform::container::battalion::chain_of_command::DelegationStrategy;

    #[tokio::test]
    async fn test_round_robin_cycles_through_specialists() {
        // Should cycle: specialist_1 -> specialist_2 -> specialist_3 -> specialist_1
        let mock_port = Arc::new(MockPaladinPort::new(vec![
            "result_1".to_string(),
            "result_2".to_string(),
            "result_3".to_string(),
            "result_4".to_string(),
        ]));
        let service = ChainOfCommandExecutionService::new(mock_port.clone());

        let commander = create_test_paladin("commander");
        let specialist_1 = create_test_paladin("specialist_1");
        let specialist_2 = create_test_paladin("specialist_2");
        let specialist_3 = create_test_paladin("specialist_3");
        let config = BattalionConfig::default();

        let chain = ChainOfCommand::new(
            commander,
            vec![specialist_1, specialist_2, specialist_3],
            config,
        )
        .expect("Should create valid chain")
        .with_strategy(DelegationStrategy::RoundRobin);

        // First call should use specialist_1
        let result1 = service.execute(&chain, "Task 1").await;
        assert!(result1.is_ok());
        let dr1 = result1.unwrap();
        assert_eq!(dr1.selected_specialists.len(), 1);
        assert_eq!(dr1.selected_specialists[0], "specialist_1");

        // Second call should use specialist_2
        let result2 = service.execute(&chain, "Task 2").await;
        assert!(result2.is_ok());
        let dr2 = result2.unwrap();
        assert_eq!(dr2.selected_specialists[0], "specialist_2");

        // Third call should use specialist_3
        let result3 = service.execute(&chain, "Task 3").await;
        assert!(result3.is_ok());
        let dr3 = result3.unwrap();
        assert_eq!(dr3.selected_specialists[0], "specialist_3");

        // Fourth call should wrap back to specialist_1
        let result4 = service.execute(&chain, "Task 4").await;
        assert!(result4.is_ok());
        let dr4 = result4.unwrap();
        assert_eq!(dr4.selected_specialists[0], "specialist_1");
    }

    #[tokio::test]
    async fn test_round_robin_with_single_specialist() {
        let mock_port = Arc::new(MockPaladinPort::new(vec![
            "result_1".to_string(),
            "result_2".to_string(),
        ]));
        let service = ChainOfCommandExecutionService::new(mock_port);

        let commander = create_test_paladin("commander");
        let specialist = create_test_paladin("specialist_1");
        let config = BattalionConfig::default();

        let chain = ChainOfCommand::new(commander, vec![specialist], config)
            .expect("Should create valid chain")
            .with_strategy(DelegationStrategy::RoundRobin);

        // Both calls should use the same specialist
        let result1 = service.execute(&chain, "Task 1").await;
        assert!(result1.is_ok());
        assert_eq!(result1.unwrap().selected_specialists[0], "specialist_1");

        let result2 = service.execute(&chain, "Task 2").await;
        assert!(result2.is_ok());
        assert_eq!(result2.unwrap().selected_specialists[0], "specialist_1");
    }

    #[tokio::test]
    async fn test_round_robin_reasoning_includes_rotation() {
        let mock_port = Arc::new(MockPaladinPort::simple("result"));
        let service = ChainOfCommandExecutionService::new(mock_port);

        let commander = create_test_paladin("commander");
        let specialist_1 = create_test_paladin("specialist_1");
        let specialist_2 = create_test_paladin("specialist_2");
        let config = BattalionConfig::default();

        let chain = ChainOfCommand::new(commander, vec![specialist_1, specialist_2], config)
            .expect("Should create valid chain")
            .with_strategy(DelegationStrategy::RoundRobin);

        let result = service.execute(&chain, "Task").await;
        assert!(result.is_ok());
        let delegation_result = result.unwrap();
        assert!(delegation_result.reasoning.to_lowercase().contains("round"));
    }
}

/// Tests for Custom delegation strategy
mod custom_delegation_tests {
    use super::*;
    use paladin::core::platform::container::battalion::chain_of_command::DelegationStrategy;

    #[tokio::test]
    async fn test_custom_delegation_with_logic_string() {
        let mock_port = Arc::new(MockPaladinPort::simple("result"));
        let service = ChainOfCommandExecutionService::new(mock_port);

        let commander = create_test_paladin("commander");
        let specialist_1 = create_test_paladin("specialist_1");
        let specialist_2 = create_test_paladin("specialist_2");
        let config = BattalionConfig::default();

        let chain = ChainOfCommand::new(commander, vec![specialist_1, specialist_2], config)
            .expect("Should create valid chain")
            .with_strategy(DelegationStrategy::Custom(
                "select first specialist".to_string(),
            ));

        let result = service.execute(&chain, "Task").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_custom_delegation_includes_logic_in_reasoning() {
        let mock_port = Arc::new(MockPaladinPort::simple("result"));
        let service = ChainOfCommandExecutionService::new(mock_port);

        let commander = create_test_paladin("commander");
        let specialist = create_test_paladin("specialist_1");
        let config = BattalionConfig::default();

        let custom_logic = "Use specialist based on task priority";
        let chain = ChainOfCommand::new(commander, vec![specialist], config)
            .expect("Should create valid chain")
            .with_strategy(DelegationStrategy::Custom(custom_logic.to_string()));

        let result = service.execute(&chain, "High priority task").await;
        assert!(result.is_ok());
        let delegation_result = result.unwrap();
        assert!(delegation_result.reasoning.contains("custom logic"));
    }
}
