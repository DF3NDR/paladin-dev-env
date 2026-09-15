//! Integration tests for Formation pattern
//!
//! Tests end-to-end Formation execution with real-world scenarios

use async_trait::async_trait;
use paladin::application::services::battalion::formation_service::FormationExecutionService;
use paladin::application::services::paladin::error::PaladinError;
use paladin::core::base::entity::node::Node;
use paladin::core::platform::container::battalion::formation::Formation;
use paladin::core::platform::container::battalion::{BattalionConfig, ErrorStrategy, RetryPolicy};
use paladin::core::platform::container::paladin::MaxLoops;
use paladin::core::platform::container::paladin::{Paladin, PaladinData, PaladinStatus};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, StopReason};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Mock PaladinPort that simulates realistic Paladin behavior
struct IntegrationMockPaladinPort {
    execution_log: Arc<Mutex<Vec<String>>>,
    failure_config: Arc<Mutex<FailureConfig>>,
}

#[derive(Clone, Debug)]
struct FailureConfig {
    fail_paladin_names: Vec<String>,
    delay_ms: u64,
}

impl IntegrationMockPaladinPort {
    fn new() -> Self {
        Self {
            execution_log: Arc::new(Mutex::new(Vec::new())),
            failure_config: Arc::new(Mutex::new(FailureConfig {
                fail_paladin_names: Vec::new(),
                delay_ms: 10,
            })),
        }
    }

    fn with_failures(self, paladin_names: Vec<String>) -> Self {
        self.failure_config.lock().unwrap().fail_paladin_names = paladin_names;
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
        let delay = self.failure_config.lock().unwrap().delay_ms;
        tokio::time::sleep(Duration::from_millis(delay)).await;

        // Check if this Paladin should fail
        let should_fail = self
            .failure_config
            .lock()
            .unwrap()
            .fail_paladin_names
            .contains(&paladin.node.name);

        if should_fail {
            return Err(PaladinError::ExecutionError(format!(
                "Simulated failure for {}",
                paladin.node.name
            )));
        }

        // Successful execution
        Ok(PaladinResult {
            output: format!("[{}]: Processed: {}", paladin.node.name, input),
            usage: paladin_ports::output::llm_port::TokenUsage::new(50, 0),
            execution_time_ms: delay,
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
async fn test_formation_end_to_end_success() {
    // Setup: Create a 3-stage processing pipeline
    let researcher = create_paladin("Researcher", "You research topics");
    let analyst = create_paladin("Analyst", "You analyze data");
    let summarizer = create_paladin("Summarizer", "You create summaries");

    let config = BattalionConfig::new("research_pipeline")
        .with_timeout(10)
        .with_description("Research → Analysis → Summary pipeline");

    let formation = Formation::new(vec![researcher, analyst, summarizer], config)
        .expect("Failed to create formation");

    // Execute
    let mock_port = Arc::new(IntegrationMockPaladinPort::new());
    let service = FormationExecutionService::new(mock_port.clone());

    let result = service.execute(&formation, "Quantum Computing").await;

    // Assert
    assert!(result.is_ok(), "Formation should execute successfully");

    let battalion_result = result.unwrap();
    assert_eq!(battalion_result.paladin_results.len(), 3);
    assert_eq!(battalion_result.battalion_name, "research_pipeline");

    // Verify sequential execution order
    let log = mock_port.get_execution_log();
    assert_eq!(log.len(), 3);
    assert!(log[0].contains("Researcher"));
    assert!(log[1].contains("Analyst"));
    assert!(log[2].contains("Summarizer"));

    // Verify output chaining
    assert!(
        battalion_result.paladin_results[1]
            .output
            .contains("Researcher")
    );
    assert!(
        battalion_result.paladin_results[2]
            .output
            .contains("Analyst")
    );
}

#[tokio::test]
async fn test_formation_with_shared_context() {
    let p1 = create_paladin("Step1", "First step");
    let p2 = create_paladin("Step2", "Second step");

    let formation = Formation::new(vec![p1, p2], BattalionConfig::new("contextual_pipeline"))
        .unwrap()
        .with_shared_context("Project: AI Safety Research\nDeadline: Q1 2026".to_string());

    let mock_port = Arc::new(IntegrationMockPaladinPort::new());
    let service = FormationExecutionService::new(mock_port.clone());

    let result = service.execute(&formation, "Initial task").await.unwrap();

    // First Paladin should receive shared context
    let log = mock_port.get_execution_log();
    assert!(log[0].contains("Project: AI Safety Research"));
    assert_eq!(result.paladin_results.len(), 2);
}

#[tokio::test]
async fn test_formation_failfast_error_handling() {
    let p1 = create_paladin("Step1", "First");
    let p2 = create_paladin("Step2", "Second"); // This will fail
    let p3 = create_paladin("Step3", "Third");

    let config = BattalionConfig::new("failfast_test").with_error_strategy(ErrorStrategy::FailFast);

    let formation = Formation::new(vec![p1, p2, p3], config).unwrap();

    let mock_port =
        Arc::new(IntegrationMockPaladinPort::new().with_failures(vec!["Step2".to_string()]));
    let service = FormationExecutionService::new(mock_port.clone());

    let result = service.execute(&formation, "Test input").await;

    // Should fail at Step2
    assert!(result.is_err());

    // Only Step1 and Step2 should have executed
    let log = mock_port.get_execution_log();
    assert_eq!(log.len(), 2);
    assert!(log[0].contains("Step1"));
    assert!(log[1].contains("Step2"));
    // Step3 should NOT have executed
}

#[tokio::test]
async fn test_formation_continue_on_error() {
    let p1 = create_paladin("Step1", "First");
    let p2 = create_paladin("Step2", "Second"); // This will fail
    let p3 = create_paladin("Step3", "Third");

    let config =
        BattalionConfig::new("continue_test").with_error_strategy(ErrorStrategy::ContinueOnError);

    let formation = Formation::new(vec![p1, p2, p3], config).unwrap();

    let mock_port =
        Arc::new(IntegrationMockPaladinPort::new().with_failures(vec!["Step2".to_string()]));
    let service = FormationExecutionService::new(mock_port.clone());

    let result = service.execute(&formation, "Test input").await;

    // Should complete despite Step2 failure
    assert!(result.is_ok());

    let battalion_result = result.unwrap();

    // All steps should have executed
    let log = mock_port.get_execution_log();
    assert_eq!(log.len(), 3);

    // But only 2 successful results (Step2 failed)
    assert_eq!(battalion_result.paladin_results.len(), 2);
}

#[tokio::test]
async fn test_formation_retry_then_continue() {
    let p1 = create_paladin("Step1", "First");
    let p2 = create_paladin("Step2", "Second");

    let mut retry_policy = RetryPolicy {
        max_attempts: 3,
        ..Default::default()
    };
    retry_policy.base_delay = Duration::from_millis(5);

    let config = BattalionConfig::new("retry_test")
        .with_error_strategy(ErrorStrategy::RetryThenContinue)
        .with_retry_policy(retry_policy);

    let formation = Formation::new(vec![p1, p2], config).unwrap();

    let mock_port =
        Arc::new(IntegrationMockPaladinPort::new().with_failures(vec!["Step2".to_string()]));
    let service = FormationExecutionService::new(mock_port.clone());

    let result = service.execute(&formation, "Test input").await;

    // Should continue after exhausting retries
    assert!(result.is_ok());

    let log = mock_port.get_execution_log();
    // Step1 executed once, Step2 attempted 3 times (initial + 2 retries)
    assert!(
        log.len() >= 4,
        "Expected at least 4 executions (1 Step1 + 3 Step2 attempts), got {}",
        log.len()
    );
}

#[tokio::test]
async fn test_formation_timeout_enforcement() {
    let p1 = create_paladin("Slow1", "First");
    let p2 = create_paladin("Slow2", "Second");

    let config = BattalionConfig::new("timeout_test").with_timeout(1); // 1 second timeout

    let formation = Formation::new(vec![p1, p2], config).unwrap();

    // Use a mock with longer delays
    let mock_port = IntegrationMockPaladinPort::new();
    mock_port.failure_config.lock().unwrap().delay_ms = 600; // 600ms per Paladin = 1.2s total

    let service = FormationExecutionService::new(Arc::new(mock_port));

    let result = service.execute(&formation, "Test input").await;

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
async fn test_formation_output_chaining() {
    let extractor = create_paladin("Extractor", "Extract key information");
    let transformer = create_paladin("Transformer", "Transform data format");
    let validator = create_paladin("Validator", "Validate output");

    let formation = Formation::new(
        vec![extractor, transformer, validator],
        BattalionConfig::new("data_pipeline"),
    )
    .unwrap();

    let mock_port = Arc::new(IntegrationMockPaladinPort::new());
    let service = FormationExecutionService::new(mock_port);

    let result = service
        .execute(&formation, "Raw data: [1,2,3]")
        .await
        .unwrap();

    // Each stage should build on the previous
    assert!(result.paladin_results[0].output.contains("Raw data"));
    assert!(result.paladin_results[1].output.contains("Extractor"));
    assert!(result.paladin_results[2].output.contains("Transformer"));

    // Final output should be from last Paladin
    assert!(result.final_output.contains("Validator"));
}

#[tokio::test]
async fn test_formation_large_pipeline() {
    // Test with 10 Paladins in sequence
    let paladins: Vec<Paladin> = (1..=10)
        .map(|i| create_paladin(&format!("Stage{}", i), &format!("Process stage {}", i)))
        .collect();

    let formation = Formation::new(
        paladins,
        BattalionConfig::new("large_pipeline").with_timeout(30),
    )
    .unwrap();

    let mock_port = Arc::new(IntegrationMockPaladinPort::new());
    let service = FormationExecutionService::new(mock_port.clone());

    let result = service.execute(&formation, "Initial input").await;

    assert!(result.is_ok());

    let battalion_result = result.unwrap();
    assert_eq!(battalion_result.paladin_results.len(), 10);

    // Verify all stages executed in order
    let log = mock_port.get_execution_log();
    assert_eq!(log.len(), 10);
    for i in 1..=10 {
        assert!(log[i - 1].contains(&format!("Stage{}", i)));
    }
}

#[tokio::test]
async fn test_formation_multiple_failures_continue_on_error() {
    let paladins: Vec<Paladin> = (1..=5)
        .map(|i| create_paladin(&format!("Stage{}", i), &format!("Stage {}", i)))
        .collect();

    let config = BattalionConfig::new("multi_failure_test")
        .with_error_strategy(ErrorStrategy::ContinueOnError);

    let formation = Formation::new(paladins, config).unwrap();

    let mock_port = Arc::new(
        IntegrationMockPaladinPort::new()
            .with_failures(vec!["Stage2".to_string(), "Stage4".to_string()]),
    );
    let service = FormationExecutionService::new(mock_port.clone());

    let result = service.execute(&formation, "Test").await;

    assert!(result.is_ok());

    let battalion_result = result.unwrap();

    // All 5 stages executed
    let log = mock_port.get_execution_log();
    assert_eq!(log.len(), 5);

    // But only 3 successful results (Stage2 and Stage4 failed)
    assert_eq!(battalion_result.paladin_results.len(), 3);
}
