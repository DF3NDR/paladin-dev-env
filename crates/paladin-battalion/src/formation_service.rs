//! Formation Execution Service
//!
//! Provides orchestration logic for executing Paladins in sequential Formation pattern.

use chrono::Utc;
use log::{debug, info, warn};
use std::sync::Arc;
use tokio::time::{Duration, timeout};
use uuid::Uuid;

use crate::error_aggregation::AggregatedError;
use crate::retry::{calculate_retry_delay, should_retry};
use paladin_core::platform::container::battalion::formation::Formation;
use paladin_core::platform::container::battalion::{
    BattalionError, BattalionResult, BattalionStatus, BattalionStrategy, ErrorStrategy, NodeError,
    TokenUsage,
};
use paladin_core::platform::container::herald::Herald;
use paladin_core::platform::container::paladin::Paladin;
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult};
use std::collections::HashMap;

/// Service for executing Formation patterns
///
/// Orchestrates sequential Paladin execution where output from one Paladin
/// flows to the next, with configurable error handling and retry logic.
///
/// # Examples
///
/// ```ignore
/// use paladin_battalion::formation_service::FormationExecutionService;
/// use std::sync::Arc;
///
/// let service = FormationExecutionService::new(paladin_port);
/// let result = service.execute(&formation, "Initial input").await?;
/// ```
pub struct FormationExecutionService {
    /// Paladin execution port
    paladin_port: Arc<dyn PaladinPort>,
    /// Optional Herald for formatting Battalion results
    herald: Option<Arc<dyn Herald>>,
}

impl FormationExecutionService {
    /// Create a new FormationExecutionService
    ///
    /// # Arguments
    ///
    /// * `paladin_port` - Port for executing individual Paladins
    ///
    /// # Example
    ///
    /// ```ignore
    /// let service = FormationExecutionService::new(paladin_port);
    /// ```
    pub fn new(paladin_port: Arc<dyn PaladinPort>) -> Self {
        info!("Creating FormationExecutionService");
        Self {
            paladin_port,
            herald: None,
        }
    }

    /// Set the Herald for formatting results
    ///
    /// This allows runtime override of the default Herald. If set, this Herald
    /// will be used to format Battalion results.
    ///
    /// # Arguments
    ///
    /// * `herald` - The Herald to use for formatting
    ///
    /// # Example
    ///
    /// ```ignore
    /// let service = FormationExecutionService::new(paladin_port)
    ///     .with_herald(Arc::new(JsonHerald::new()));
    /// ```
    pub fn with_herald(mut self, herald: Arc<dyn Herald>) -> Self {
        self.herald = Some(herald);
        self
    }

    /// Format a Battalion result using the configured Herald
    ///
    /// Converts the Battalion result into the Herald's output format. If no Herald
    /// is configured, returns None.
    ///
    /// # Arguments
    ///
    /// * `result` - The Battalion result to format
    ///
    /// # Returns
    ///
    /// * `Ok(Some(String))` - Formatted output if Herald is configured
    /// * `Ok(None)` - If no Herald is configured
    /// * `Err(BattalionError)` - If formatting fails
    ///
    /// # Example
    ///
    /// ```ignore
    /// let formatted = service.format_result(&result)?;
    /// if let Some(output) = formatted {
    ///     println!("{}", output);
    /// }
    /// ```
    pub fn format_result(
        &self,
        result: &BattalionResult,
    ) -> Result<Option<String>, BattalionError> {
        match &self.herald {
            Some(herald) => {
                // Herald now uses actual BattalionResult directly - no conversion needed!
                herald
                    .format_battalion_result(result)
                    .map(Some)
                    .map_err(|e| {
                        BattalionError::FormationError(format!("Herald formatting error: {}", e))
                    })
            }
            None => Ok(None),
        }
    }

    /// Execute a Formation with the given input
    ///
    /// Executes Paladins sequentially, passing output from one to the next.
    /// Supports shared context injection, timeout enforcement, and configurable
    /// error strategies.
    ///
    /// # Arguments
    ///
    /// * `formation` - The Formation to execute
    /// * `initial_input` - Initial input for the first Paladin
    ///
    /// # Returns
    ///
    /// * `Ok(BattalionResult)` - Final result with all Paladin outputs
    /// * `Err(BattalionError)` - If execution fails according to error strategy
    ///
    /// # Example
    ///
    /// ```ignore
    /// let result = service.execute(&formation, "Analyze this data").await?;
    /// println!("Final output: {}", result.final_output);
    /// ```
    pub async fn execute(
        &self,
        formation: &Formation,
        initial_input: &str,
    ) -> Result<BattalionResult, BattalionError> {
        let battalion_id = Uuid::new_v4();
        let _started_at = Utc::now();

        info!(
            "Starting Formation execution: {} (ID: {}) with {} Paladins",
            formation.config.name,
            battalion_id,
            formation.paladins.len()
        );

        // Execute with timeout
        let timeout_duration = Duration::from_secs(formation.config.timeout_seconds);

        match timeout(
            timeout_duration,
            self.execute_internal(formation, initial_input, battalion_id),
        )
        .await
        {
            Ok(result) => {
                info!("Formation {} completed successfully", battalion_id);
                result
            }
            Err(_) => {
                warn!(
                    "Formation {} timed out after {} seconds",
                    battalion_id, formation.config.timeout_seconds
                );
                Err(BattalionError::Timeout(formation.config.timeout_seconds))
            }
        }
    }

    /// Internal execution logic without timeout wrapper
    async fn execute_internal(
        &self,
        formation: &Formation,
        initial_input: &str,
        battalion_id: Uuid,
    ) -> Result<BattalionResult, BattalionError> {
        let started_at = Utc::now();
        let mut current_input = initial_input.to_string();
        let mut paladin_results: Vec<PaladinResult> = Vec::new();
        let mut aggregated_error = AggregatedError::new(formation.paladins.len());
        let mut per_paladin_times: HashMap<String, u64> = HashMap::new();
        let mut per_paladin_tokens: HashMap<String, TokenUsage> = HashMap::new();
        let mut total_tokens: u64 = 0;
        let mut node_errors: Vec<NodeError> = Vec::new();

        // Prepend shared context if present
        if let Some(context) = &formation.shared_context {
            current_input = format!("{}\n\n{}", context, current_input);
        }

        // Execute Paladins sequentially
        for (index, paladin) in formation.paladins.iter().enumerate() {
            debug!(
                "Executing Paladin {}/{}: {}",
                index + 1,
                formation.paladins.len(),
                paladin.node.name
            );

            match self
                .execute_paladin_with_strategy(
                    paladin,
                    &current_input,
                    &formation.config.error_strategy,
                    &formation.config.retry_policy,
                )
                .await
            {
                Ok(result) => {
                    // Aggregate per-Paladin time/token metrics before the
                    // result is moved into paladin_results, mirroring
                    // PhalanxExecutionService::execute_internal.
                    per_paladin_times.insert(paladin.node.name.clone(), result.execution_time_ms);
                    per_paladin_tokens.insert(paladin.node.name.clone(), result.usage.clone());
                    total_tokens += u64::from(result.usage.total_tokens);

                    // Success: Update input for next Paladin
                    current_input = result.output.clone();
                    paladin_results.push(result);
                    aggregated_error.record_success();
                }
                Err(error) => {
                    // Error: Handle based on strategy
                    match formation.config.error_strategy {
                        ErrorStrategy::FailFast => {
                            warn!(
                                "FailFast: Formation failed at Paladin {} due to error",
                                index + 1
                            );
                            return Err(error);
                        }
                        ErrorStrategy::ContinueOnError | ErrorStrategy::RetryThenContinue => {
                            warn!(
                                "ContinueOnError: Paladin {} failed, continuing with empty output",
                                index + 1
                            );
                            node_errors.push(NodeError {
                                node_name: paladin.node.name.clone(),
                                error: error.to_string(),
                            });
                            aggregated_error.add_error(error);
                            // Continue with empty output
                            current_input = String::new();
                        }
                    }
                }
            }
        }

        // Check if we should fail based on aggregated errors
        if aggregated_error.has_errors() {
            match formation.config.error_strategy {
                ErrorStrategy::FailFast => {
                    // Should not reach here
                    unreachable!("FailFast should have returned earlier");
                }
                ErrorStrategy::ContinueOnError | ErrorStrategy::RetryThenContinue => {
                    warn!(
                        "Formation completed with errors: {}",
                        aggregated_error.summary()
                    );
                    // Continue to return partial results
                }
            }
        }

        // Success/failure counts mirror PhalanxExecutionService::execute_internal:
        // every entry that made it into paladin_results succeeded, and every
        // recorded node_errors entry is one Paladin that failed outright under
        // a continue-past-failure strategy.
        let paladin_success_count = paladin_results.len();
        let paladin_failure_count = node_errors.len();

        // Create result. Built as a struct literal (mirroring
        // PhalanxExecutionService) rather than through BattalionResult::new,
        // since the plain constructor defaults the aggregation fields this
        // Formation now populates.
        let result = BattalionResult {
            battalion_id,
            battalion_name: formation.config.name.clone(),
            started_at,
            completed_at: Utc::now(),
            final_output: current_input, // Final output from last Paladin
            paladin_results,
            status: BattalionStatus::Completed,
            strategy_used: BattalionStrategy::Formation,
            strategy_selection_reasoning: None,
            strategy_selection_time_ms: 0,
            per_paladin_times,
            per_paladin_tokens,
            total_tokens,
            paladin_success_count,
            paladin_failure_count,
            node_errors,
        };

        Ok(result)
    }

    /// Execute a single Paladin with error strategy and retry logic
    async fn execute_paladin_with_strategy(
        &self,
        paladin: &Paladin,
        input: &str,
        error_strategy: &ErrorStrategy,
        retry_policy: &paladin_core::platform::container::battalion::RetryPolicy,
    ) -> Result<PaladinResult, BattalionError> {
        match error_strategy {
            ErrorStrategy::FailFast | ErrorStrategy::ContinueOnError => {
                // No retry, just execute
                self.paladin_port
                    .execute(paladin, input)
                    .await
                    .map_err(|e| BattalionError::PaladinError(e.to_string()))
            }
            ErrorStrategy::RetryThenContinue => {
                // Retry with exponential backoff
                let mut attempt = 0;
                loop {
                    match self.paladin_port.execute(paladin, input).await {
                        Ok(result) => return Ok(result),
                        Err(e) => {
                            if should_retry(retry_policy, attempt) {
                                let delay = calculate_retry_delay(retry_policy, attempt);
                                warn!(
                                    "Paladin {} failed (attempt {}), retrying after {:?}",
                                    paladin.node.name,
                                    attempt + 1,
                                    delay
                                );
                                tokio::time::sleep(delay).await;
                                attempt += 1;
                            } else {
                                warn!(
                                    "Paladin {} failed after {} attempts, giving up",
                                    paladin.node.name,
                                    attempt + 1
                                );
                                return Err(BattalionError::PaladinError(e.to_string()));
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use paladin_core::base::entity::node::Node;
    use paladin_core::platform::container::battalion::{
        BattalionConfig, ErrorStrategy, RetryPolicy,
    };
    use paladin_core::platform::container::paladin::{MaxLoops, PaladinData, PaladinStatus};
    use paladin_ports::output::paladin_port::{PaladinResult, StopReason};
    use std::sync::Mutex;

    // Mock PaladinPort for testing
    struct MockPaladinPort {
        call_count: Arc<Mutex<usize>>,
        should_fail: bool,
        fail_until_attempt: Option<usize>,
    }

    impl MockPaladinPort {
        fn new() -> Self {
            Self {
                call_count: Arc::new(Mutex::new(0)),
                should_fail: false,
                fail_until_attempt: None,
            }
        }

        fn new_with_failure() -> Self {
            Self {
                call_count: Arc::new(Mutex::new(0)),
                should_fail: true,
                fail_until_attempt: None,
            }
        }

        fn new_with_retry_success(fail_until: usize) -> Self {
            Self {
                call_count: Arc::new(Mutex::new(0)),
                should_fail: false,
                fail_until_attempt: Some(fail_until),
            }
        }

        fn get_call_count(&self) -> usize {
            *self.call_count.lock().unwrap()
        }
    }

    #[async_trait]
    impl PaladinPort for MockPaladinPort {
        async fn execute(
            &self,
            paladin: &Paladin,
            input: &str,
        ) -> Result<PaladinResult, paladin_core::platform::container::paladin_error::PaladinError>
        {
            let mut count = self.call_count.lock().unwrap();
            *count += 1;
            let current_count = *count;
            drop(count);

            if let Some(fail_until) = self.fail_until_attempt
                && current_count <= fail_until
            {
                return Err(
                    paladin_core::platform::container::paladin_error::PaladinError::ExecutionError(
                        format!(
                            "Intentional failure for testing (attempt {})",
                            current_count
                        ),
                    ),
                );
            }

            if self.should_fail {
                return Err(
                    paladin_core::platform::container::paladin_error::PaladinError::ExecutionError(
                        "Mock Paladin execution failed".to_string(),
                    ),
                );
            }

            Ok(PaladinResult {
                output: format!("Processed: {} by {}", input, paladin.node.name),
                usage: TokenUsage::new(100, 0),
                execution_time_ms: 100,
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
                Result<
                    paladin_ports::output::paladin_port::PaladinStreamChunk,
                    paladin_core::platform::container::paladin_error::PaladinError,
                >,
            >,
            paladin_core::platform::container::paladin_error::PaladinError,
        > {
            let (_tx, rx) = tokio::sync::mpsc::channel(1);
            Ok(rx)
        }

        fn validate(
            &self,
            _paladin: &Paladin,
        ) -> Result<(), paladin_core::platform::container::paladin_error::PaladinError> {
            Ok(())
        }
    }

    fn create_test_paladin(name: &str) -> Paladin {
        let data = PaladinData {
            system_prompt: format!("You are {}", name),
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
    async fn test_formation_service_creation() {
        let mock_port = Arc::new(MockPaladinPort::new());
        let _service = FormationExecutionService::new(mock_port);
        // Service created successfully
        // Test passes if we reach here without panicking
    }

    #[tokio::test]
    async fn test_sequential_execution_success() {
        let mock_port = Arc::new(MockPaladinPort::new());
        let service = FormationExecutionService::new(mock_port.clone());

        let p1 = create_test_paladin("P1");
        let p2 = create_test_paladin("P2");
        let p3 = create_test_paladin("P3");

        let formation =
            Formation::new(vec![p1, p2, p3], BattalionConfig::new("test_formation")).unwrap();

        let result = service.execute(&formation, "Initial input").await;
        assert!(result.is_ok());

        let battalion_result = result.unwrap();
        assert_eq!(battalion_result.paladin_results.len(), 3);
        assert_eq!(battalion_result.status, BattalionStatus::Completed);
        assert_eq!(mock_port.get_call_count(), 3);
    }

    #[tokio::test]
    async fn test_output_passing_between_paladins() {
        let mock_port = Arc::new(MockPaladinPort::new());
        let service = FormationExecutionService::new(mock_port);

        let p1 = create_test_paladin("P1");
        let p2 = create_test_paladin("P2");

        let formation =
            Formation::new(vec![p1, p2], BattalionConfig::new("test_formation")).unwrap();

        let result = service.execute(&formation, "Start").await.unwrap();

        // First Paladin processes "Start"
        assert!(result.paladin_results[0].output.contains("Start"));

        // Second Paladin processes output from first
        assert!(
            result.paladin_results[1]
                .output
                .contains("Processed: Processed: Start by P1")
        );
    }

    #[tokio::test]
    async fn test_failfast_error_strategy() {
        let mock_port = Arc::new(MockPaladinPort::new_with_failure());
        let service = FormationExecutionService::new(mock_port);

        let p1 = create_test_paladin("P1");
        let p2 = create_test_paladin("P2");

        let config =
            BattalionConfig::new("test_formation").with_error_strategy(ErrorStrategy::FailFast);

        let formation = Formation::new(vec![p1, p2], config).unwrap();

        let result = service.execute(&formation, "Input").await;
        assert!(result.is_err());

        match result.unwrap_err() {
            BattalionError::PaladinError(_) => { /* Expected */ }
            _ => panic!("Expected PaladinError"),
        }
    }

    #[tokio::test]
    async fn test_continue_on_error_strategy() {
        // Create mock that fails only first Paladin
        let mock_port = Arc::new(MockPaladinPort::new_with_retry_success(1));
        let service = FormationExecutionService::new(mock_port);

        let p1 = create_test_paladin("P1");
        let p2 = create_test_paladin("P2");

        let config = BattalionConfig::new("test_formation")
            .with_error_strategy(ErrorStrategy::ContinueOnError);

        let formation = Formation::new(vec![p1, p2], config).unwrap();

        let result = service.execute(&formation, "Input").await;
        // Should succeed despite first failure
        assert!(result.is_ok());

        let battalion_result = result.unwrap();
        // Only second Paladin result included
        assert_eq!(battalion_result.paladin_results.len(), 1);
    }

    #[tokio::test]
    async fn test_formation_aggregates_per_paladin_times_and_tokens() {
        let mock_port = Arc::new(MockPaladinPort::new());
        let service = FormationExecutionService::new(mock_port);

        let p1 = create_test_paladin("Alpha");
        let p2 = create_test_paladin("Beta");
        let p3 = create_test_paladin("Gamma");

        let formation =
            Formation::new(vec![p1, p2, p3], BattalionConfig::new("test_formation")).unwrap();

        let result = service.execute(&formation, "Initial input").await.unwrap();

        // First Paladin's output is the first entry — execution order preserved.
        assert_eq!(result.paladin_results.len(), 3);

        assert_eq!(result.per_paladin_times.len(), 3);
        assert!(result.per_paladin_times.contains_key("Alpha"));
        assert!(result.per_paladin_times.contains_key("Beta"));
        assert!(result.per_paladin_times.contains_key("Gamma"));

        assert_eq!(result.per_paladin_tokens.len(), 3);
        let expected_total: u64 = result
            .per_paladin_tokens
            .values()
            .map(|t| u64::from(t.total_tokens))
            .sum();
        assert_eq!(result.total_tokens, expected_total);
        // Mock returns token_count=100 per Paladin, three Paladins ran.
        assert_eq!(result.total_tokens, 300);
    }

    #[tokio::test]
    async fn test_formation_records_node_errors_on_continue_on_error() {
        // Fails only the first call (P1); P2 and P3 succeed.
        let mock_port = Arc::new(MockPaladinPort::new_with_retry_success(1));
        let service = FormationExecutionService::new(mock_port);

        let p1 = create_test_paladin("P1");
        let p2 = create_test_paladin("P2");
        let p3 = create_test_paladin("P3");

        let config = BattalionConfig::new("test_formation")
            .with_error_strategy(ErrorStrategy::ContinueOnError);

        let formation = Formation::new(vec![p1, p2, p3], config).unwrap();

        let result = service.execute(&formation, "Input").await.unwrap();

        assert_eq!(result.node_errors.len(), 1);
        assert_eq!(result.node_errors[0].node_name, "P1");
        assert!(!result.node_errors[0].error.is_empty());
        assert_eq!(result.paladin_failure_count, 1);

        // The successful Paladins still appear in the aggregation maps.
        assert_eq!(result.per_paladin_times.len(), 2);
        assert!(result.per_paladin_times.contains_key("P2"));
        assert!(result.per_paladin_times.contains_key("P3"));
        assert_eq!(result.per_paladin_tokens.len(), 2);
    }

    #[tokio::test]
    async fn test_shared_context_injection() {
        let mock_port = Arc::new(MockPaladinPort::new());
        let service = FormationExecutionService::new(mock_port);

        let p1 = create_test_paladin("P1");
        let p2 = create_test_paladin("P2");

        let formation = Formation::new(vec![p1, p2], BattalionConfig::new("test_formation"))
            .unwrap()
            .with_shared_context("Shared: Context info".to_string());

        let result = service.execute(&formation, "Input").await.unwrap();

        // First Paladin should see shared context + input
        assert!(
            result.paladin_results[0]
                .output
                .contains("Shared: Context info")
        );
    }

    #[tokio::test]
    async fn test_retry_then_continue_strategy() {
        let mock_port = Arc::new(MockPaladinPort::new_with_retry_success(2));
        let service = FormationExecutionService::new(mock_port.clone());

        let p1 = create_test_paladin("P1");

        let retry_policy = RetryPolicy {
            max_attempts: 3,
            base_delay: Duration::from_millis(10),
            ..Default::default()
        };

        let config = BattalionConfig::new("test_formation")
            .with_error_strategy(ErrorStrategy::RetryThenContinue)
            .with_retry_policy(retry_policy);

        let formation = Formation::new(vec![p1.clone(), p1], config).unwrap();

        let result = service.execute(&formation, "Input").await;
        assert!(result.is_ok());

        // Should have retried and succeeded
        assert!(mock_port.get_call_count() >= 3); // Initial + 2 retries
    }

    #[tokio::test]
    async fn test_timeout_enforcement() {
        // This test would need a mock that delays execution
        // For now, just verify the timeout configuration is respected
        let mock_port = Arc::new(MockPaladinPort::new());
        let service = FormationExecutionService::new(mock_port);

        let p1 = create_test_paladin("P1");
        let p2 = create_test_paladin("P2");

        let config = BattalionConfig::new("test_formation").with_timeout(1); // 1 second timeout

        let formation = Formation::new(vec![p1, p2], config).unwrap();

        let result = service.execute(&formation, "Input").await;
        // Should complete quickly, not timeout
        assert!(result.is_ok());
    }
}
