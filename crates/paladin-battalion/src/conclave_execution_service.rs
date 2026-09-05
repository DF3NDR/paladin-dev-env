//! Conclave Execution Service
//!
//! Provides orchestration logic for executing the MixtureOfAgents (MoA) pattern
//! where multiple expert Paladins process the same task in parallel, and an
//! aggregator Paladin synthesizes their outputs.

use chrono::Utc;
use log::{debug, info, warn};
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{sleep, timeout};

use paladin_core::platform::container::battalion::conclave::{
    Conclave, ConclaveError, ConclaveResult, ConclaveStatus, ObservabilityLevel,
};
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult};

/// Service for executing Conclave patterns
///
/// Orchestrates parallel expert execution with retry logic, formats their outputs
/// for the aggregator, and synthesizes the final result.
///
/// # Examples
///
/// ```ignore
/// use std::sync::Arc;
/// use paladin_battalion::conclave_execution_service::ConclaveExecutionService;
///
/// let service = ConclaveExecutionService::new(paladin_port);
/// let result = service.execute(&conclave, "Analyze this system architecture").await?;
///
/// println!("Aggregated output: {}", result.aggregated_output.output);
/// println!("Expert count: {}", result.successful_expert_count());
/// ```
pub struct ConclaveExecutionService {
    paladin_port: Arc<dyn PaladinPort>,
}

impl ConclaveExecutionService {
    /// Create a new Conclave execution service
    ///
    /// # Arguments
    ///
    /// * `paladin_port` - Port for executing Paladins
    ///
    /// # Example
    ///
    /// ```ignore
    /// let service = ConclaveExecutionService::new(paladin_port);
    /// ```
    pub fn new(paladin_port: Arc<dyn PaladinPort>) -> Self {
        Self { paladin_port }
    }

    /// Execute a Conclave with the given input
    ///
    /// Executes all expert Paladins in parallel with retry logic, then synthesizes
    /// their outputs through the aggregator Paladin.
    ///
    /// # Arguments
    ///
    /// * `conclave` - The Conclave configuration and Paladins
    /// * `input` - The task input to analyze
    ///
    /// # Returns
    ///
    /// * `Ok(ConclaveResult)` - Successful execution with expert and aggregated outputs
    /// * `Err(ConclaveError)` - Execution failed
    ///
    /// # Errors
    ///
    /// * `ConclaveError::AllExpertsFailed` - All experts failed after retries
    /// * `ConclaveError::AggregatorFailed` - Aggregator failed to synthesize
    /// * `ConclaveError::Timeout` - Execution exceeded timeout
    ///
    /// # Example
    ///
    /// ```ignore
    /// let result = service.execute(&conclave, "Review this code").await?;
    ///
    /// match result.status {
    ///     ConclaveStatus::Success => println!("All experts succeeded!"),
    ///     ConclaveStatus::PartialSuccess => println!("Some experts failed but synthesis succeeded"),
    ///     ConclaveStatus::Failed => println!("Critical failure"),
    /// }
    /// ```
    pub async fn execute(
        &self,
        conclave: &Conclave,
        input: &str,
    ) -> Result<ConclaveResult, ConclaveError> {
        let start_time = Utc::now();
        let timeout_duration = Duration::from_secs(conclave.config.timeout_seconds);

        info!(
            "Starting Conclave execution: '{}' with {} experts",
            conclave.name(),
            conclave.expert_count()
        );

        // Execute with timeout
        let result = timeout(timeout_duration, self.execute_internal(conclave, input)).await;

        match result {
            Ok(Ok(conclave_result)) => {
                let duration = (Utc::now() - start_time).num_milliseconds() as u64;
                info!(
                    "Conclave '{}' completed in {}ms with status: {:?}",
                    conclave.name(),
                    duration,
                    conclave_result.status
                );
                Ok(conclave_result)
            }
            Ok(Err(e)) => {
                warn!("Conclave '{}' failed: {}", conclave.name(), e);
                Err(e)
            }
            Err(_) => {
                warn!(
                    "Conclave '{}' timed out after {}s",
                    conclave.name(),
                    conclave.config.timeout_seconds
                );
                Err(ConclaveError::Timeout(conclave.config.timeout_seconds))
            }
        }
    }

    /// Internal execution logic without timeout wrapper
    async fn execute_internal(
        &self,
        conclave: &Conclave,
        input: &str,
    ) -> Result<ConclaveResult, ConclaveError> {
        let start_time = Utc::now();

        // Execute all experts in parallel with retry logic
        let (expert_outputs, expert_times, retry_counts) =
            self.execute_experts_parallel(conclave, input).await?;

        // Check if we have any successful expert outputs
        if expert_outputs.is_empty() {
            return Err(ConclaveError::AllExpertsFailed);
        }

        // Format expert outputs for aggregator
        let formatted_context =
            self.format_expert_outputs_for_aggregator(&expert_outputs, input, &conclave.config);

        // Execute aggregator with formatted expert outputs
        let aggregator_start = Utc::now();
        let aggregated_output = self
            .execute_aggregator(conclave, &formatted_context)
            .await
            .map_err(|e| ConclaveError::AggregatorFailed(e.to_string()))?;
        let _aggregator_time = (Utc::now() - aggregator_start).num_milliseconds() as u64;

        // Determine status based on expert success rate
        let total_experts = conclave.expert_count();
        let successful_experts = expert_outputs.len();
        let status = if successful_experts == total_experts {
            ConclaveStatus::Success
        } else {
            ConclaveStatus::PartialSuccess
        };

        let total_execution_time = (Utc::now() - start_time).num_milliseconds() as u64;

        self.log_execution_summary(
            conclave,
            &status,
            successful_experts,
            total_experts,
            total_execution_time,
        );

        Ok(ConclaveResult {
            expert_outputs,
            aggregated_output,
            execution_time_ms: total_execution_time,
            expert_execution_times: expert_times,
            retry_counts,
            status,
        })
    }

    /// Execute all expert Paladins in parallel with retry logic
    ///
    /// Returns successful expert outputs, execution times, and retry counts.
    async fn execute_experts_parallel(
        &self,
        conclave: &Conclave,
        input: &str,
    ) -> Result<
        (
            HashMap<String, PaladinResult>,
            HashMap<String, u64>,
            HashMap<String, u32>,
        ),
        ConclaveError,
    > {
        let mut expert_outputs = HashMap::new();
        let mut expert_times = HashMap::new();
        let mut retry_counts = HashMap::new();

        // Spawn tasks for all experts
        let mut expert_tasks = Vec::new();

        for expert in &conclave.experts {
            let expert_name = expert.node.name.clone();
            let expert_name_for_task = expert_name.clone();
            let expert_clone = expert.clone();
            let input_clone = input.to_string();
            let paladin_port = Arc::clone(&self.paladin_port);
            let retry_attempts = conclave.config.retry_attempts;
            let observability = conclave.config.observability_level;

            let task: tokio::task::JoinHandle<Result<(PaladinResult, u64, u32), PaladinError>> =
                tokio::spawn(async move {
                    Self::execute_expert_with_retry(
                        paladin_port,
                        &expert_clone,
                        &expert_name_for_task,
                        &input_clone,
                        retry_attempts,
                        observability,
                    )
                    .await
                });

            expert_tasks.push((expert_name, task));
        }

        // Collect results from all expert tasks
        for (expert_name, task) in expert_tasks {
            match task.await {
                Ok(Ok((result, execution_time, retries))) => {
                    debug!(
                        "Expert '{}' succeeded after {} retries in {}ms",
                        expert_name, retries, execution_time
                    );
                    let expert_name_string: String = expert_name.clone();
                    expert_outputs.insert(expert_name_string.clone(), result);
                    expert_times.insert(expert_name_string.clone(), execution_time);
                    retry_counts.insert(expert_name, retries);
                }
                Ok(Err(e)) => {
                    warn!("Expert '{}' failed after all retries: {}", expert_name, e);
                    // Don't insert into expert_outputs - this expert failed
                    retry_counts.insert(expert_name, conclave.config.retry_attempts);
                }
                Err(e) => {
                    warn!("Expert '{}' task panicked: {}", expert_name, e);
                    retry_counts.insert(expert_name, conclave.config.retry_attempts);
                }
            }
        }

        Ok((expert_outputs, expert_times, retry_counts))
    }

    /// Execute a single expert with retry logic
    ///
    /// Implements exponential backoff with jitter for transient errors.
    async fn execute_expert_with_retry(
        paladin_port: Arc<dyn PaladinPort>,
        expert: &paladin_core::platform::container::paladin::Paladin,
        expert_name: &str,
        input: &str,
        max_retries: u32,
        observability: ObservabilityLevel,
    ) -> Result<(PaladinResult, u64, u32), PaladinError> {
        let start_time = Utc::now();
        let mut retries = 0;

        loop {
            let attempt_start = Utc::now();

            match paladin_port.execute(expert, input).await {
                Ok(result) => {
                    let execution_time = (Utc::now() - start_time).num_milliseconds() as u64;

                    if matches!(observability, ObservabilityLevel::Verbose) {
                        info!(
                            "Expert '{}' succeeded on attempt {} in {}ms",
                            expert_name,
                            retries + 1,
                            (Utc::now() - attempt_start).num_milliseconds()
                        );
                    }

                    return Ok((result, execution_time, retries));
                }
                Err(e) => {
                    if retries >= max_retries {
                        warn!(
                            "Expert '{}' failed after {} attempts: {}",
                            expert_name,
                            retries + 1,
                            e
                        );
                        return Err(e);
                    }

                    // Check if error is retryable
                    if !Self::is_retryable_error(&e) {
                        warn!(
                            "Expert '{}' encountered non-retryable error: {}",
                            expert_name, e
                        );
                        return Err(e);
                    }

                    retries += 1;

                    // Calculate backoff delay
                    let delay = Self::calculate_retry_delay(retries - 1);

                    if matches!(
                        observability,
                        ObservabilityLevel::Standard | ObservabilityLevel::Verbose
                    ) {
                        debug!(
                            "Expert '{}' attempt {} failed: {}. Retrying in {:?}",
                            expert_name, retries, e, delay
                        );
                    }

                    sleep(delay).await;
                }
            }
        }
    }

    /// Check if an error is retryable
    ///
    /// Transient errors (network, timeout, rate limit) should be retried.
    /// Permanent errors (invalid API key, invalid config) should not.
    fn is_retryable_error(error: &PaladinError) -> bool {
        match error {
            // Retryable errors
            PaladinError::Timeout(_) => true,
            PaladinError::LlmError(msg) => {
                // Check for rate limit indicators
                let msg_lower = msg.to_lowercase();
                msg_lower.contains("rate limit")
                    || msg_lower.contains("timeout")
                    || msg_lower.contains("network")
                    || msg_lower.contains("connection")
                    || msg_lower.contains("503")
                    || msg_lower.contains("429")
            }
            // Non-retryable errors
            PaladinError::ConfigurationError(_) => false,
            PaladinError::ExecutionError(_) => false,
            PaladinError::StopWordDetected(_) => false,
            PaladinError::CircuitBreakerOpen => false,
            PaladinError::MaxRetriesExceeded(_) => false,
            PaladinError::GarrisonError(_) => false,
            PaladinError::GarrisonRequired => false,
            PaladinError::ArsenalError(_) => false,
            // FT-01 (25-02 Task 1 landed the variant; D-02): classify by the carried
            // transience instead of message sniffing.
            PaladinError::LlmFailure { .. } => matches!(
                error.transience(),
                paladin_core::platform::container::transience::Transience::Transient
            ),
            // X-10.2 (D-04): `PaladinError` is `#[non_exhaustive]`; a future
            // variant is classified by its own `transience()` rather than a
            // silently-wrong `true`/`false` guess, mirroring the `LlmFailure`
            // arm immediately above.
            _ => matches!(
                error.transience(),
                paladin_core::platform::container::transience::Transience::Transient
            ),
        }
    }

    /// Calculate retry delay with exponential backoff and jitter
    ///
    /// Implements: delay = (2^attempt) seconds with ±20% jitter
    fn calculate_retry_delay(attempt: u32) -> Duration {
        let base_delay_secs = 2u64.pow(attempt); // 1s, 2s, 4s, 8s, 16s
        let base_delay_secs = base_delay_secs.min(16); // Cap at 16 seconds

        // Add jitter: ±20% random variance
        let mut rng = rand::thread_rng();
        let jitter_factor = rng.gen_range(0.8..=1.2);
        let delay_with_jitter = (base_delay_secs as f64 * jitter_factor) as u64;

        Duration::from_secs(delay_with_jitter)
    }

    /// Format expert outputs for aggregator consumption
    ///
    /// Creates a structured prompt with all expert outputs, optionally including
    /// expert names based on configuration.
    fn format_expert_outputs_for_aggregator(
        &self,
        expert_outputs: &HashMap<String, PaladinResult>,
        original_task: &str,
        config: &paladin_core::platform::container::battalion::conclave::ConclaveConfig,
    ) -> String {
        let mut formatted = String::new();

        // Use custom synthesis prompt if provided, otherwise use default template
        if let Some(custom_prompt) = &config.synthesis_prompt {
            formatted.push_str(custom_prompt);
            formatted.push_str("\n\n");
        } else {
            formatted.push_str("You are synthesizing outputs from multiple expert agents.\n\n");
        }

        formatted.push_str(&format!("Task: {}\n\n", original_task));
        formatted.push_str("Expert Outputs:\n");
        formatted.push_str("---\n");

        for (expert_name, result) in expert_outputs {
            if config.include_expert_names {
                formatted.push_str(&format!("Expert '{}':\n", expert_name));
            }

            // Truncate if max tokens specified
            let output = if let Some(max_tokens) = config.max_expert_output_tokens {
                Self::truncate_output(&result.output, max_tokens)
            } else {
                result.output.clone()
            };

            formatted.push_str(&output);
            formatted.push_str("\n\n---\n");
        }

        if config.synthesis_prompt.is_none() {
            formatted.push_str("\nYour goal: Synthesize these expert perspectives into a comprehensive, balanced response.\n");
            formatted.push_str(
                "Highlight areas of consensus and note any divergent opinions respectfully.\n",
            );
            formatted
                .push_str("Provide a cohesive analysis that integrates all relevant insights.");
        }

        formatted
    }

    /// Truncate output to approximate token limit
    ///
    /// Uses rough estimation: 1 token ≈ 4 characters
    fn truncate_output(output: &str, max_tokens: usize) -> String {
        let max_chars = max_tokens * 4;
        if output.len() <= max_chars {
            output.to_string()
        } else {
            let mut truncated = output.chars().take(max_chars).collect::<String>();
            truncated.push_str("... [truncated]");
            truncated
        }
    }

    /// Execute the aggregator Paladin with formatted expert outputs
    async fn execute_aggregator(
        &self,
        conclave: &Conclave,
        formatted_context: &str,
    ) -> Result<PaladinResult, PaladinError> {
        debug!(
            "Executing aggregator '{}' with {} chars of context",
            conclave.aggregator.node.name,
            formatted_context.len()
        );

        self.paladin_port
            .execute(&conclave.aggregator, formatted_context)
            .await
    }

    /// Log execution summary based on observability level
    fn log_execution_summary(
        &self,
        conclave: &Conclave,
        status: &ConclaveStatus,
        successful: usize,
        total: usize,
        execution_time_ms: u64,
    ) {
        match conclave.config.observability_level {
            ObservabilityLevel::Minimal => {
                // Only log if failed
                if !matches!(status, ConclaveStatus::Success) {
                    info!(
                        "Conclave '{}' completed with status: {:?}",
                        conclave.name(),
                        status
                    );
                }
            }
            ObservabilityLevel::Standard => {
                info!(
                    "Conclave '{}' completed: {}/{} experts succeeded in {}ms ({})",
                    conclave.name(),
                    successful,
                    total,
                    execution_time_ms,
                    match status {
                        ConclaveStatus::Success => "Success",
                        ConclaveStatus::PartialSuccess => "Partial Success",
                        ConclaveStatus::Failed => "Failed",
                    }
                );
            }
            ObservabilityLevel::Verbose => {
                info!("Conclave '{}' execution summary:", conclave.name());
                info!("  Status: {:?}", status);
                info!("  Experts: {}/{} succeeded", successful, total);
                info!("  Total time: {}ms", execution_time_ms);
                info!("  Retry attempts: {}", conclave.config.retry_attempts);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use paladin_core::base::entity::node::Node;
    use paladin_core::platform::container::battalion::BattalionConfig;
    use paladin_core::platform::container::battalion::conclave::ConclaveConfig;
    use paladin_core::platform::container::paladin::{MaxLoops, PaladinData, PaladinStatus};
    use paladin_ports::output::paladin_port::StopReason;
    use std::sync::Mutex;

    // Mock PaladinPort for testing
    struct MockPaladinPort {
        // Track calls for verification
        call_count: Arc<Mutex<usize>>,
        // Simulate failures for first N attempts
        fail_attempts: Arc<Mutex<HashMap<String, usize>>>,
    }

    impl MockPaladinPort {
        fn new() -> Self {
            Self {
                call_count: Arc::new(Mutex::new(0)),
                fail_attempts: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        fn with_failures(self, expert_name: &str, fail_count: usize) -> Self {
            self.fail_attempts
                .lock()
                .unwrap()
                .insert(expert_name.to_string(), fail_count);
            self
        }
    }

    #[async_trait]
    impl PaladinPort for MockPaladinPort {
        async fn execute(
            &self,
            paladin: &paladin_core::platform::container::paladin::Paladin,
            input: &str,
        ) -> Result<PaladinResult, PaladinError> {
            let mut count = self.call_count.lock().unwrap();
            *count += 1;

            let expert_name = &paladin.node.name;

            // Check if this expert should fail
            if let Some(fail_count) = self.fail_attempts.lock().unwrap().get_mut(expert_name)
                && *fail_count > 0
            {
                *fail_count -= 1;
                return Err(PaladinError::Timeout(10));
            }

            // Success case
            Ok(PaladinResult {
                output: format!("Analysis from {}: {}", expert_name, input),
                token_count: 100,
                execution_time_ms: 50,
                loop_count: 1,
                stop_reason: StopReason::Completed,
                ..Default::default()
            })
        }

        async fn execute_stream(
            &self,
            _paladin: &paladin_core::platform::container::paladin::Paladin,
            _input: &str,
        ) -> Result<paladin_ports::output::paladin_port::PaladinStream, PaladinError> {
            unimplemented!("Stream not needed for tests")
        }

        fn validate(
            &self,
            _paladin: &paladin_core::platform::container::paladin::Paladin,
        ) -> Result<(), PaladinError> {
            Ok(())
        }
    }

    fn create_test_paladin(name: &str) -> paladin_core::platform::container::paladin::Paladin {
        let data = PaladinData {
            system_prompt: format!("You are {}", name),
            name: name.to_string(),
            user_name: "TestUser".to_string(),
            model: "gpt-4o".to_string(),
            temperature: 0.7,
            max_loops: MaxLoops::Fixed(1),
            stop_words: vec![],
            status: PaladinStatus::Idle,
            vision_enabled: false,
            ..Default::default()
        };

        Node::new(data, Some(name.to_string()))
    }

    fn create_test_conclave() -> Conclave {
        let battalion_config = BattalionConfig::new("test_conclave");
        let config = ConclaveConfig::new("TestConclave", battalion_config)
            .with_timeout(60)
            .with_retry_attempts(2);

        let experts = vec![
            create_test_paladin("Expert1"),
            create_test_paladin("Expert2"),
            create_test_paladin("Expert3"),
        ];

        let aggregator = create_test_paladin("Aggregator");

        Conclave::new(experts, aggregator, config).unwrap()
    }

    #[tokio::test]
    async fn test_successful_conclave_execution() {
        let mock_port = Arc::new(MockPaladinPort::new());
        let service = ConclaveExecutionService::new(mock_port.clone());
        let conclave = create_test_conclave();

        let result = service.execute(&conclave, "Test task").await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.status, ConclaveStatus::Success);
        assert_eq!(result.successful_expert_count(), 3);
        assert!(result.all_experts_succeeded());

        // Verify all experts + aggregator were called
        let call_count = *mock_port.call_count.lock().unwrap();
        assert_eq!(call_count, 4); // 3 experts + 1 aggregator
    }

    #[tokio::test]
    async fn test_partial_success_with_one_expert_failure() {
        let mock_port = Arc::new(MockPaladinPort::new().with_failures("Expert2", 10));
        let service = ConclaveExecutionService::new(mock_port);
        let conclave = create_test_conclave();

        let result = service.execute(&conclave, "Test task").await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.status, ConclaveStatus::PartialSuccess);
        assert_eq!(result.successful_expert_count(), 2); // Expert1 and Expert3
        assert!(!result.all_experts_succeeded());
        assert!(result.is_completed());
    }

    #[tokio::test]
    async fn test_retry_logic_with_transient_failures() {
        let mock_port = Arc::new(MockPaladinPort::new().with_failures("Expert1", 1));
        let service = ConclaveExecutionService::new(mock_port.clone());
        let conclave = create_test_conclave();

        let result = service.execute(&conclave, "Test task").await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.status, ConclaveStatus::Success);
        assert_eq!(result.successful_expert_count(), 3);

        // Expert1 should have retried once
        assert_eq!(result.retry_counts.get("Expert1"), Some(&1));
        assert_eq!(result.retry_counts.get("Expert2"), Some(&0));
        assert_eq!(result.retry_counts.get("Expert3"), Some(&0));
    }

    #[tokio::test]
    async fn test_all_experts_fail() {
        let mock_port = Arc::new(
            MockPaladinPort::new()
                .with_failures("Expert1", 10)
                .with_failures("Expert2", 10)
                .with_failures("Expert3", 10),
        );
        let service = ConclaveExecutionService::new(mock_port);
        let conclave = create_test_conclave();

        let result = service.execute(&conclave, "Test task").await;
        assert!(result.is_err());

        match result {
            Err(ConclaveError::AllExpertsFailed) => {} // Expected
            _ => panic!("Expected AllExpertsFailed error"),
        }
    }

    #[test]
    fn test_retry_delay_calculation() {
        let delay0 = ConclaveExecutionService::calculate_retry_delay(0);
        assert!(delay0.as_secs() <= 3); // 1s with +20% jitter = max ~1.2s (allow buffer)

        let delay1 = ConclaveExecutionService::calculate_retry_delay(1);
        assert!(delay1.as_secs() >= 1 && delay1.as_secs() <= 4); // 2s ± 20% with jitter

        let delay2 = ConclaveExecutionService::calculate_retry_delay(2);
        assert!(delay2.as_secs() >= 3 && delay2.as_secs() <= 6); // 4s ± 20% with jitter
    }

    #[test]
    fn test_is_retryable_error() {
        assert!(ConclaveExecutionService::is_retryable_error(
            &PaladinError::Timeout(10)
        ));
        assert!(ConclaveExecutionService::is_retryable_error(
            &PaladinError::LlmError("Rate limit exceeded".to_string())
        ));
        assert!(ConclaveExecutionService::is_retryable_error(
            &PaladinError::LlmError("503 Service Unavailable".to_string())
        ));

        assert!(!ConclaveExecutionService::is_retryable_error(
            &PaladinError::ConfigurationError("Invalid config".to_string())
        ));
        assert!(!ConclaveExecutionService::is_retryable_error(
            &PaladinError::StopWordDetected("STOP".to_string())
        ));
    }

    #[test]
    fn test_format_expert_outputs() {
        let service = ConclaveExecutionService::new(Arc::new(MockPaladinPort::new()));
        let battalion_config = BattalionConfig::new("test");
        let config = ConclaveConfig::new("Test", battalion_config).with_expert_names(true);

        let mut expert_outputs = HashMap::new();
        expert_outputs.insert(
            "Expert1".to_string(),
            PaladinResult {
                output: "Analysis 1".to_string(),
                token_count: 10,
                execution_time_ms: 100,
                loop_count: 1,
                stop_reason: StopReason::Completed,
                ..Default::default()
            },
        );
        expert_outputs.insert(
            "Expert2".to_string(),
            PaladinResult {
                output: "Analysis 2".to_string(),
                token_count: 10,
                execution_time_ms: 100,
                loop_count: 1,
                stop_reason: StopReason::Completed,
                ..Default::default()
            },
        );

        let formatted =
            service.format_expert_outputs_for_aggregator(&expert_outputs, "Test task", &config);

        assert!(formatted.contains("Task: Test task"));
        assert!(formatted.contains("Expert 'Expert1'"));
        assert!(formatted.contains("Expert 'Expert2'"));
        assert!(formatted.contains("Analysis 1"));
        assert!(formatted.contains("Analysis 2"));
    }

    #[test]
    fn test_truncate_output() {
        let output = "A".repeat(1000);
        let truncated = ConclaveExecutionService::truncate_output(&output, 100); // 100 tokens = ~400 chars

        assert!(truncated.len() < output.len());
        assert!(truncated.ends_with("[truncated]"));
    }
}
