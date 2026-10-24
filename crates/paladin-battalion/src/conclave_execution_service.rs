//! Conclave Execution Service
//!
//! Provides orchestration logic for executing the MixtureOfAgents (MoA) pattern
//! where multiple expert Paladins process the same task in parallel, and an
//! aggregator Paladin synthesizes their outputs.

use chrono::Utc;
use log::{debug, info, warn};
use std::collections::HashMap;
use std::sync::Arc;

use paladin_core::platform::container::aegis::Aegis;
use paladin_core::platform::container::battalion::conclave::{
    Conclave, ConclaveError, ConclaveResult, ConclaveStatus, ObservabilityLevel,
};
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::transience::Transience;
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult};

use crate::aegis_attempt;
use crate::engine::retry::{backoff_delay, wait_backoff};

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
    /// * `ConclaveError::AggregatorFailed` - Aggregator failed to synthesize (including an
    ///   aggregator attempt that exceeded its bound)
    /// * `ConclaveError::ConfigurationError` - The Conclave's `battalion_config.aegis` is invalid
    ///   (checked before any expert runs)
    ///
    /// Each expert and aggregator attempt is bounded by `battalion_config.aegis.timeout`; there
    /// is no whole-Conclave clock. A timed-out expert attempt is a transient failure and is
    /// retried within `retry_attempts`.
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

        // Reject an invalid policy before any expert runs.
        conclave
            .config
            .battalion_config
            .validate_aegis()
            .map_err(|e| ConclaveError::ConfigurationError(e.to_string()))?;

        info!(
            "Starting Conclave execution: '{}' with {} experts",
            conclave.name(),
            conclave.expert_count()
        );

        match self.execute_internal(conclave, input).await {
            Ok(conclave_result) => {
                let duration = (Utc::now() - start_time).num_milliseconds() as u64;
                info!(
                    "Conclave '{}' completed in {}ms with status: {:?}",
                    conclave.name(),
                    duration,
                    conclave_result.status
                );
                Ok(conclave_result)
            }
            Err(e) => {
                warn!("Conclave '{}' failed: {}", conclave.name(), e);
                Err(e)
            }
        }
    }

    /// Internal execution logic (experts, then the aggregator)
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
            let expert_name = expert.data.name.clone();
            let expert_name_for_task = expert_name.clone();
            let expert_clone = expert.clone();
            let input_clone = input.to_string();
            let paladin_port = Arc::clone(&self.paladin_port);
            let retry_attempts = conclave.config.retry_attempts;
            let observability = conclave.config.observability_level;
            let aegis = conclave.config.battalion_config.aegis.clone();

            let task: tokio::task::JoinHandle<Result<(PaladinResult, u64, u32), PaladinError>> =
                tokio::spawn(async move {
                    Self::execute_expert_with_retry(
                        paladin_port,
                        &expert_clone,
                        &expert_name_for_task,
                        &input_clone,
                        retry_attempts,
                        observability,
                        aegis,
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
    /// Each attempt is one [`aegis_attempt::attempt_once`] call, bounded by
    /// `aegis.timeout` (per attempt). A failed attempt is retried while
    /// `retries < max_retries` and [`Self::is_retryable_error`] allows it, after
    /// waiting `engine::retry::backoff_delay` for the next attempt.
    ///
    /// `max_retries` (the Conclave's `retry_attempts`) is the retry *count*
    /// (D-04); `aegis.retry` contributes only the backoff shape, falling back
    /// to `RetryPolicy::default()` when it is `None`. The predicate stays
    /// Transient-only whatever `policy.retry_on` says (D-10). The input is
    /// re-sent unchanged on every attempt, and the returned retry count is the
    /// number of retries actually performed.
    async fn execute_expert_with_retry(
        paladin_port: Arc<dyn PaladinPort>,
        expert: &paladin_core::platform::container::paladin::Paladin,
        expert_name: &str,
        input: &str,
        max_retries: u32,
        observability: ObservabilityLevel,
        aegis: Aegis,
    ) -> Result<(PaladinResult, u64, u32), PaladinError> {
        let start_time = Utc::now();
        let bound = aegis_attempt::attempt_bound(aegis.timeout.as_ref());
        let policy = aegis.retry.clone().unwrap_or_default();
        let mut retries: u32 = 0;

        loop {
            let attempt_start = Utc::now();

            match aegis_attempt::attempt_once(&paladin_port, expert, input, bound, retries + 1)
                .await
            {
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
                Err(failure) => {
                    let e = failure.error;
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

                    // The one backoff implementation in the crate (D-12).
                    let delay = backoff_delay(&policy, retries + 1);

                    if matches!(
                        observability,
                        ObservabilityLevel::Standard | ObservabilityLevel::Verbose
                    ) {
                        debug!(
                            "Expert '{}' attempt {} failed: {}. Retrying in {:?}",
                            expert_name, retries, e, delay
                        );
                    }

                    // No cancellation token here: a Conclave run is not cancellable.
                    wait_backoff(delay, &None).await;
                }
            }
        }
    }

    /// Check if an error is retryable.
    ///
    /// An expert failure is retried if and only if
    /// [`PaladinError::transience`] classifies it as [`Transience::Transient`]
    /// (D-10, D-11). This is parity with the engine's default `TransientOnly`
    /// retry predicate: [`Transience::Unknown`] is **not** retried, and a
    /// [`Transience::Permanent`] failure never is. The `transience()` table is
    /// the only classification source -- the failure's message text is never
    /// read, so a provider-controlled string cannot steer the decision.
    fn is_retryable_error(error: &PaladinError) -> bool {
        error.transience() == Transience::Transient
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
            conclave.aggregator.data.name,
            formatted_context.len()
        );

        // One attempt, bounded like an expert attempt; a timeout surfaces as
        // `PaladinError::Timeout` and the caller maps it to `AggregatorFailed`.
        let bound =
            aegis_attempt::attempt_bound(conclave.config.battalion_config.aegis.timeout.as_ref());
        aegis_attempt::attempt_once(
            &self.paladin_port,
            &conclave.aggregator,
            formatted_context,
            bound,
            1,
        )
        .await
        .map_err(|failure| failure.error)
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
    use paladin_core::base::entity::Entity;
    use paladin_core::platform::container::aegis::{
        ErrorHandlerSpec, RetryPolicy as AegisRetryPolicy, TimeoutPolicy,
    };
    use paladin_core::platform::container::battalion::BattalionConfig;
    use paladin_core::platform::container::battalion::TokenUsage;
    use paladin_core::platform::container::battalion::conclave::ConclaveConfig;
    use paladin_core::platform::container::battlefield::FieldName;
    use paladin_core::platform::container::paladin::{MaxLoops, PaladinData, PaladinStatus};
    use paladin_core::platform::container::waypoint::NodeId;
    use paladin_ports::output::paladin_port::StopReason;
    use std::collections::VecDeque;
    use std::sync::Mutex;
    use std::time::Duration;

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

            let expert_name = &paladin.data.name;

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
                usage: TokenUsage::new(100, 0),
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

        Entity::new(data, Some(name.to_string()))
    }

    fn create_test_conclave() -> Conclave {
        let battalion_config = BattalionConfig::new("test_conclave");
        let config = ConclaveConfig::new("TestConclave", battalion_config).with_retry_attempts(2);

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

    #[tokio::test(start_paused = true)]
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

    #[tokio::test(start_paused = true)]
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

    #[tokio::test(start_paused = true)]
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

    /// D-10/D-11: the Conclave retries a failure if and only if
    /// `transience()` says `Transient`. One row per behaviour, including the
    /// variants whose answer changed against the v0.10 per-variant match.
    #[test]
    fn conclave_retry_predicate_is_transient_only() {
        use crate::llm_failure::to_paladin_error;
        use paladin_core::platform::container::arsenal::ArsenalError;
        use paladin_core::platform::container::garrison_error::GarrisonError;
        use paladin_ports::output::llm_port::LlmError;

        let cases: Vec<(PaladinError, bool)> = vec![
            (PaladinError::Timeout(10), true),
            (
                to_paladin_error(&LlmError::ProviderError {
                    provider: "openai".to_string(),
                    status: 503,
                    message: "upstream unavailable".to_string(),
                }),
                true,
            ),
            (
                to_paladin_error(&LlmError::AuthenticationError(
                    "invalid API key".to_string(),
                )),
                false,
            ),
            (
                to_paladin_error(&LlmError::ProcessingError("opaque".to_string())),
                false,
            ),
            (PaladinError::ExecutionError("boom".to_string()), false),
            (
                PaladinError::ConfigurationError("Invalid config".to_string()),
                false,
            ),
            (PaladinError::StopWordDetected("STOP".to_string()), false),
            // New retries versus v0.10 (recorded as MIGRATION.md 10.1 rows).
            (PaladinError::CircuitBreakerOpen, true),
            (
                PaladinError::GarrisonError(GarrisonError::StorageError("db".to_string())),
                true,
            ),
            (PaladinError::ArsenalError(ArsenalError::Timeout(5)), true),
        ];
        for (error, expected) in cases {
            assert_eq!(
                ConclaveExecutionService::is_retryable_error(&error),
                expected,
                "retry decision for {error:?}"
            );
        }
    }

    /// Edge LEGACY-03/encoding: the predicate never reads message text.
    #[test]
    fn conclave_retry_predicate_ignores_message_text() {
        let permanent_but_retryish = PaladinError::LlmFailure {
            transience: Transience::Permanent,
            status: None,
            provider: Some("openai".to_string()),
            message: "429 rate limit timeout 503 network connection".to_string(),
        };
        assert!(
            !ConclaveExecutionService::is_retryable_error(&permanent_but_retryish),
            "a Permanent failure is not retried whatever its message says: {permanent_but_retryish:?}"
        );

        let transient_empty = PaladinError::LlmFailure {
            transience: Transience::Transient,
            status: None,
            provider: None,
            message: String::new(),
        };
        assert!(
            ConclaveExecutionService::is_retryable_error(&transient_empty),
            "a Transient failure is retried even with an empty message: {transient_empty:?}"
        );
    }

    /// Plan 25-06 Test 3 (D-02): the Conclave retry predicate classifies a
    /// structured `LlmFailure` -- produced by `llm_failure::to_paladin_error`
    /// from a real `LlmError` -- by its carried transience, never by sniffing
    /// the message, and the rendered text is the legacy `LLM error: {e}`.
    #[test]
    fn conclave_execution_service_surfaces_structured_llm_failure() {
        use crate::llm_failure::to_paladin_error;
        use paladin_ports::output::llm_port::LlmError;

        // Transient by typed status: retried.
        let transient = to_paladin_error(&LlmError::ProviderError {
            provider: "openai".to_string(),
            status: 503,
            message: "upstream unavailable".to_string(),
        });
        assert!(matches!(
            transient,
            PaladinError::LlmFailure {
                transience: Transience::Transient,
                status: Some(503),
                ..
            }
        ));
        assert!(ConclaveExecutionService::is_retryable_error(&transient));

        // A 500 carries no message text a sniffer would recognise; the typed
        // status classifies it Transient by value (FT-FR-01).
        let server_fault = to_paladin_error(&LlmError::ProviderError {
            provider: "openai".to_string(),
            status: 500,
            message: "internal".to_string(),
        });
        assert!(ConclaveExecutionService::is_retryable_error(&server_fault));

        // Permanent: not retried.
        let permanent = to_paladin_error(&LlmError::AuthenticationError(
            "invalid API key".to_string(),
        ));
        assert!(matches!(
            permanent,
            PaladinError::LlmFailure {
                transience: Transience::Permanent,
                status: None,
                ..
            }
        ));
        assert!(!ConclaveExecutionService::is_retryable_error(&permanent));

        // Unknown: not retried by the Conclave predicate.
        let unknown = to_paladin_error(&LlmError::ProcessingError("opaque".to_string()));
        assert!(!ConclaveExecutionService::is_retryable_error(&unknown));

        // Rendered text is `LLM error: {e}` (the source error's own display).
        let fixed = LlmError::rate_limited(None);
        assert_eq!(
            to_paladin_error(&fixed).to_string(),
            format!("LLM error: {fixed}")
        );
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
                usage: TokenUsage::new(10, 0),
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
                usage: TokenUsage::new(10, 0),
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

    // ---- Aegis runner / engine backoff tests (44-07, D-12) -----------------------------

    /// One scripted call for a named Paladin: wait `delay`, then answer `result`.
    struct Step {
        delay: Duration,
        result: Result<(), PaladinError>,
    }

    fn fail(error: PaladinError) -> Step {
        Step {
            delay: Duration::ZERO,
            result: Err(error),
        }
    }

    fn slow_ok(delay: Duration) -> Step {
        Step {
            delay,
            result: Ok(()),
        }
    }

    fn transient() -> PaladinError {
        PaladinError::LlmFailure {
            transience: Transience::Transient,
            status: Some(503),
            provider: None,
            message: "unavailable".to_string(),
        }
    }

    /// A port that plays back a per-Paladin script and records every call as
    /// `(paladin name, input, instant)`. A Paladin whose script is exhausted
    /// succeeds immediately.
    struct ScriptedPort {
        scripts: Mutex<HashMap<String, VecDeque<Step>>>,
        calls: Mutex<Vec<(String, String, tokio::time::Instant)>>,
    }

    impl ScriptedPort {
        fn new(scripts: Vec<(&str, Vec<Step>)>) -> Arc<Self> {
            Arc::new(Self {
                scripts: Mutex::new(
                    scripts
                        .into_iter()
                        .map(|(name, steps)| (name.to_string(), steps.into()))
                        .collect(),
                ),
                calls: Mutex::new(Vec::new()),
            })
        }

        fn calls_for(&self, name: &str) -> Vec<(String, tokio::time::Instant)> {
            self.calls
                .lock()
                .unwrap()
                .iter()
                .filter(|(n, _, _)| n == name)
                .map(|(_, input, at)| (input.clone(), *at))
                .collect()
        }

        fn total_calls(&self) -> usize {
            self.calls.lock().unwrap().len()
        }
    }

    #[async_trait]
    impl PaladinPort for ScriptedPort {
        async fn execute(
            &self,
            paladin: &paladin_core::platform::container::paladin::Paladin,
            input: &str,
        ) -> Result<PaladinResult, PaladinError> {
            let name = paladin.data.name.clone();
            self.calls.lock().unwrap().push((
                name.clone(),
                input.to_string(),
                tokio::time::Instant::now(),
            ));
            let step = self
                .scripts
                .lock()
                .unwrap()
                .get_mut(&name)
                .and_then(|script| script.pop_front());
            let step = step.unwrap_or_else(|| slow_ok(Duration::ZERO));
            tokio::time::sleep(step.delay).await;
            step.result.map(|()| PaladinResult {
                output: format!("Analysis from {}", name),
                usage: TokenUsage::new(10, 0),
                execution_time_ms: 1,
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

    /// A two-expert Conclave (`Expert1`, `Expert2`, aggregator `Aggregator`).
    fn conclave_with(aegis: Aegis, retry_attempts: u32) -> Conclave {
        let battalion_config = BattalionConfig::new("aegis_conclave").with_aegis(aegis);
        let config = ConclaveConfig::new("AegisConclave", battalion_config)
            .with_retry_attempts(retry_attempts);
        Conclave::new(
            vec![
                create_test_paladin("Expert1"),
                create_test_paladin("Expert2"),
            ],
            create_test_paladin("Aggregator"),
            config,
        )
        .unwrap()
    }

    fn run_timeout_aegis(run_timeout: Duration) -> Aegis {
        Aegis {
            timeout: Some(TimeoutPolicy {
                run_timeout: Some(run_timeout),
                idle_timeout: None,
            }),
            ..Aegis::default()
        }
    }

    /// D-12: waits follow `engine::retry::backoff_delay` -- initial 1 s, factor 2.
    #[tokio::test(start_paused = true)]
    async fn conclave_backoff_follows_engine_sequence() {
        let port = ScriptedPort::new(vec![(
            "Expert1",
            vec![fail(transient()), fail(transient())],
        )]);
        let aegis = Aegis {
            retry: Some(AegisRetryPolicy {
                max_attempts: 3,
                initial_interval: Duration::from_secs(1),
                backoff_factor: 2.0,
                jitter: false,
                ..AegisRetryPolicy::default()
            }),
            ..Aegis::default()
        };
        let conclave = conclave_with(aegis, 2);
        let service = ConclaveExecutionService::new(port.clone());

        let started = tokio::time::Instant::now();
        let result = service.execute(&conclave, "task").await.unwrap();

        assert_eq!(result.retry_counts.get("Expert1"), Some(&2));
        assert_eq!(result.retry_counts.get("Expert2"), Some(&0));
        // 1 s before attempt 2 plus 2 s before attempt 3.
        assert_eq!(started.elapsed(), Duration::from_secs(3));
        let at: Vec<_> = port
            .calls_for("Expert1")
            .into_iter()
            .map(|(_, t)| t)
            .collect();
        assert_eq!(at[1] - at[0], Duration::from_secs(1));
        assert_eq!(at[2] - at[1], Duration::from_secs(2));
    }

    /// Open Question 2: a timed-out expert attempt is transient and is retried.
    #[tokio::test(start_paused = true)]
    async fn conclave_expert_attempt_times_out_and_retries() {
        let port = ScriptedPort::new(vec![(
            "Expert1",
            vec![slow_ok(Duration::from_secs(2)), slow_ok(Duration::ZERO)],
        )]);
        let conclave = conclave_with(run_timeout_aegis(Duration::from_secs(1)), 1);
        let service = ConclaveExecutionService::new(port.clone());

        let result = service.execute(&conclave, "task").await.unwrap();

        assert_eq!(result.status, ConclaveStatus::Success);
        assert_eq!(result.retry_counts.get("Expert1"), Some(&1));
        assert_eq!(port.calls_for("Expert1").len(), 2);
    }

    /// Open Question 2: a timed-out aggregator attempt ends the run as `AggregatorFailed`.
    #[tokio::test(start_paused = true)]
    async fn conclave_aggregator_timeout_is_aggregator_failed() {
        let port = ScriptedPort::new(vec![("Aggregator", vec![slow_ok(Duration::from_secs(2))])]);
        let conclave = conclave_with(run_timeout_aegis(Duration::from_secs(1)), 1);
        let service = ConclaveExecutionService::new(port.clone());

        match service.execute(&conclave, "task").await {
            Err(ConclaveError::AggregatorFailed(msg)) => {
                assert!(msg.contains("Timeout"), "{msg}");
            }
            other => panic!("Expected AggregatorFailed, got {:?}", other.map(|_| ())),
        }
        // The aggregator runs once: aggregator failures are not retried.
        assert_eq!(port.calls_for("Aggregator").len(), 1);
    }

    /// Edge LEGACY-03/empty: `retry_attempts = 0` runs a Transient-failing expert once.
    #[tokio::test(start_paused = true)]
    async fn conclave_zero_retry_attempts_runs_once() {
        let port = ScriptedPort::new(vec![(
            "Expert1",
            vec![fail(transient()), fail(transient()), fail(transient())],
        )]);
        let conclave = conclave_with(Aegis::default(), 0);
        let service = ConclaveExecutionService::new(port.clone());

        let result = service.execute(&conclave, "task").await.unwrap();

        assert_eq!(port.calls_for("Expert1").len(), 1);
        assert_eq!(result.status, ConclaveStatus::PartialSuccess);
        assert!(!result.expert_outputs.contains_key("Expert1"));
    }

    /// Edge LEGACY-03/empty: with `aegis.retry = None` the waits follow
    /// `RetryPolicy::default()` (500 ms, factor 2, jitter adding up to one base delay).
    #[tokio::test(start_paused = true)]
    async fn conclave_without_aegis_retry_uses_default_backoff_shape() {
        let port = ScriptedPort::new(vec![(
            "Expert1",
            vec![fail(transient()), fail(transient())],
        )]);
        let conclave = conclave_with(Aegis::default(), 2);
        let service = ConclaveExecutionService::new(port.clone());

        let result = service.execute(&conclave, "task").await.unwrap();
        assert_eq!(result.retry_counts.get("Expert1"), Some(&2));

        let at: Vec<_> = port
            .calls_for("Expert1")
            .into_iter()
            .map(|(_, t)| t)
            .collect();
        assert_eq!(at.len(), 3);
        let first = at[1] - at[0];
        let second = at[2] - at[1];
        assert!(
            first >= Duration::from_millis(500) && first < Duration::from_secs(1),
            "wait before attempt 2 was {first:?}"
        );
        assert!(
            second >= Duration::from_secs(1) && second < Duration::from_secs(2),
            "wait before attempt 3 was {second:?}"
        );
    }

    /// Edge LEGACY-03/idempotency: every retry re-sends the identical input and the
    /// recorded retry count is attempts - 1.
    #[tokio::test(start_paused = true)]
    async fn conclave_retry_resends_identical_input() {
        let port = ScriptedPort::new(vec![(
            "Expert1",
            vec![fail(transient()), fail(transient())],
        )]);
        let conclave = conclave_with(Aegis::default(), 2);
        let service = ConclaveExecutionService::new(port.clone());

        let result = service.execute(&conclave, "the exact task").await.unwrap();

        let calls = port.calls_for("Expert1");
        assert_eq!(calls.len(), 3);
        assert!(calls.iter().all(|(input, _)| input == "the exact task"));
        assert_eq!(
            result.retry_counts.get("Expert1"),
            Some(&((calls.len() - 1) as u32))
        );
    }

    /// T-44-20: an invalid policy is rejected before any expert runs.
    #[tokio::test(start_paused = true)]
    async fn conclave_rejects_invalid_aegis_before_any_expert_runs() {
        let port = ScriptedPort::new(vec![]);
        let mut conclave = conclave_with(Aegis::default(), 2);
        // `Conclave::new` already validates, so break the policy afterwards.
        conclave.config.battalion_config.aegis = Aegis {
            on_error: Some(ErrorHandlerSpec::Route {
                to: NodeId::new("cancel"),
                error_field: FieldName::new("failure").expect("valid field name"),
            }),
            ..Aegis::default()
        };
        let service = ConclaveExecutionService::new(port.clone());

        match service.execute(&conclave, "task").await {
            Err(ConclaveError::ConfigurationError(msg)) => assert!(msg.contains("Route"), "{msg}"),
            other => panic!("Expected ConfigurationError, got {:?}", other.map(|_| ())),
        }
        assert_eq!(port.total_calls(), 0);
    }
}
