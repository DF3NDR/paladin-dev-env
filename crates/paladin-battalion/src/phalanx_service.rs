//! Phalanx Execution Service
//!
//! Provides orchestration logic for executing Paladins in concurrent Phalanx pattern.

use chrono::Utc;
use futures::future::{BoxFuture, FutureExt, select_ok};
use log::{debug, info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{Duration, timeout};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::error_aggregation::AggregatedError;
use paladin_core::platform::container::battalion::phalanx::{AggregationStrategy, Phalanx};
use paladin_core::platform::container::battalion::{
    BattalionError, BattalionResult, ErrorStrategy, NodeError,
};
use paladin_core::platform::container::herald::Herald;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult};

#[cfg(test)]
use paladin_core::platform::container::battalion::{BattalionStatus, TokenUsage};

#[cfg(test)]
use tokio::sync::mpsc;

/// Service for executing Phalanx patterns
///
/// Orchestrates concurrent Paladin execution with configurable aggregation strategies,
/// concurrency limiting via semaphore, and cancellation support.
///
/// # Examples
///
/// ```ignore
/// let service = PhalanxExecutionService::new(paladin_port);
/// let result = service.execute(&phalanx, "Analyze this data").await?;
/// ```
pub struct PhalanxExecutionService {
    paladin_port: Arc<dyn PaladinPort>,
    /// Optional Herald for formatting Battalion results
    herald: Option<Arc<dyn Herald>>,
}

impl PhalanxExecutionService {
    /// Create a new Phalanx execution service
    pub fn new(paladin_port: Arc<dyn PaladinPort>) -> Self {
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
    /// let service = PhalanxExecutionService::new(paladin_port)
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
                        BattalionError::PhalanxError(format!("Herald formatting error: {}", e))
                    })
            }
            None => Ok(None),
        }
    }

    /// Execute a Phalanx with the given input
    ///
    /// Paladins are executed concurrently according to the aggregation strategy.
    /// Respects timeout, concurrency limits, and error strategies.
    pub async fn execute(
        &self,
        phalanx: &Phalanx,
        input: &str,
    ) -> Result<BattalionResult, BattalionError> {
        let config = phalanx.config();
        let timeout_duration = Duration::from_secs(config.timeout_seconds);

        info!(
            "Starting Phalanx execution: {} with {} Paladins",
            config.name,
            phalanx.paladin_count()
        );

        // Wrap execution with timeout
        match timeout(timeout_duration, self.execute_internal(phalanx, input)).await {
            Ok(result) => result,
            Err(_) => {
                warn!(
                    "Phalanx '{}' timed out after {} seconds",
                    config.name, config.timeout_seconds
                );
                Err(BattalionError::Timeout(config.timeout_seconds))
            }
        }
    }

    /// Execute Phalanx with cancellation support
    ///
    /// Allows external cancellation of ongoing execution
    pub async fn execute_with_cancellation(
        &self,
        phalanx: &Phalanx,
        input: &str,
        cancellation_token: CancellationToken,
    ) -> Result<BattalionResult, BattalionError> {
        let config = phalanx.config();
        let timeout_duration = Duration::from_secs(config.timeout_seconds);

        tokio::select! {
            result = timeout(timeout_duration, self.execute_internal(phalanx, input)) => {
                match result {
                    Ok(r) => r,
                    Err(_) => Err(BattalionError::Timeout(config.timeout_seconds)),
                }
            }
            _ = cancellation_token.cancelled() => {
                info!("Phalanx '{}' cancelled", config.name);
                Err(BattalionError::Cancelled)
            }
        }
    }

    /// Internal execution logic
    async fn execute_internal(
        &self,
        phalanx: &Phalanx,
        input: &str,
    ) -> Result<BattalionResult, BattalionError> {
        let config = phalanx.config();
        let started_at = Utc::now();
        let battalion_id = Uuid::new_v4();

        // Validate aggregation strategy
        self.validate_aggregation_strategy(phalanx)?;

        // Get paladin names for metrics tracking
        let paladin_names: Vec<String> = phalanx
            .paladins()
            .iter()
            .map(|p| p.node.name.clone())
            .collect();

        // Execute based on aggregation strategy
        let (paladin_results, errors) = match phalanx.aggregation_strategy() {
            AggregationStrategy::CollectAll => self.execute_collect_all(phalanx, input).await?,
            AggregationStrategy::FirstSuccess => self.execute_first_success(phalanx, input).await?,
            AggregationStrategy::Majority => self.execute_majority(phalanx, input).await?,
            AggregationStrategy::Custom(fn_name) => {
                return Err(BattalionError::ConfigurationError(format!(
                    "Custom aggregation '{}' not yet implemented",
                    fn_name
                )));
            }
        };

        // Handle errors according to error strategy
        if !errors.is_empty() {
            match config.error_strategy {
                ErrorStrategy::FailFast => {
                    let mut agg_error = AggregatedError::new(phalanx.paladin_count());
                    for error in errors {
                        agg_error.add_error(BattalionError::ExecutionError(error));
                    }
                    return Err(BattalionError::AggregationError(format!(
                        "Phalanx execution failed with {} errors",
                        agg_error.errors.len()
                    )));
                }
                ErrorStrategy::ContinueOnError => {
                    warn!(
                        "Phalanx '{}' completed with {} errors (ContinueOnError)",
                        config.name,
                        errors.len()
                    );
                }
                ErrorStrategy::RetryThenContinue => {
                    // Retries handled at Paladin level in concurrent execution
                    warn!(
                        "Phalanx '{}' completed with {} errors after retries",
                        config.name,
                        errors.len()
                    );
                }
            }
        }

        // Determine final output based on aggregation
        let final_output = if paladin_results.is_empty() {
            String::new()
        } else {
            paladin_results.last().unwrap().output.clone()
        };

        // Build per-paladin metrics from execution results
        let failed_names: Vec<String> = errors
            .iter()
            .filter_map(|e| e.split(':').next().map(|s| s.trim().to_string()))
            .collect();

        // Surface the already-built per-agent "{name}: {error}" strings as
        // structured NodeError entries instead of dropping them (D-03 /
        // MERGE-04) — one entry per failed node, name/error text preserved.
        let node_errors: Vec<NodeError> = errors
            .iter()
            .filter_map(|e| {
                e.split_once(": ").map(|(name, err)| NodeError {
                    node_name: name.trim().to_string(),
                    error: err.trim().to_string(),
                })
            })
            .collect();

        let mut per_paladin_times = HashMap::new();
        let mut per_paladin_tokens = HashMap::new();
        let mut total_tokens: u64 = 0;

        // Track which successful results map to which paladin names
        // Results are returned in order matching successful paladins
        let successful_names: Vec<&String> = paladin_names
            .iter()
            .filter(|name| !failed_names.contains(name))
            .collect();

        for (i, result) in paladin_results.iter().enumerate() {
            if let Some(name) = successful_names.get(i) {
                per_paladin_times.insert((*name).clone(), result.execution_time_ms);
                per_paladin_tokens.insert((*name).clone(), result.usage.clone());
                total_tokens += u64::from(result.usage.total_tokens);
            }
        }

        let paladin_success_count = paladin_results.len();
        let paladin_failure_count = errors.len();

        let completed_at = Utc::now();
        Ok(BattalionResult {
            battalion_id,
            battalion_name: config.name.clone(),
            paladin_results,
            started_at,
            completed_at,
            final_output,
            status: paladin_core::platform::container::battalion::BattalionStatus::Completed,
            strategy_used: paladin_core::platform::container::battalion::BattalionStrategy::Phalanx,
            strategy_selection_reasoning: None,
            strategy_selection_time_ms: 0,
            per_paladin_times,
            per_paladin_tokens,
            total_tokens,
            paladin_success_count,
            paladin_failure_count,
            node_errors,
        })
    }

    /// Validate aggregation strategy requirements
    fn validate_aggregation_strategy(&self, phalanx: &Phalanx) -> Result<(), BattalionError> {
        if matches!(
            phalanx.aggregation_strategy(),
            AggregationStrategy::Majority
        ) && phalanx.paladin_count() < 3
        {
            return Err(BattalionError::ValidationError(
                "Majority aggregation requires at least 3 Paladins".to_string(),
            ));
        }
        Ok(())
    }

    /// CollectAll: Wait for all Paladins to complete
    async fn execute_collect_all(
        &self,
        phalanx: &Phalanx,
        input: &str,
    ) -> Result<(Vec<PaladinResult>, Vec<String>), BattalionError> {
        let semaphore = phalanx
            .max_concurrency()
            .map(|max| Arc::new(Semaphore::new(max)));

        let mut tasks = Vec::new();

        for paladin in phalanx.paladins() {
            let paladin_clone: paladin_core::platform::container::paladin::Paladin =
                paladin.clone();
            let input_clone = input.to_string();
            let port = self.paladin_port.clone();
            let semaphore_clone = semaphore.clone();

            let task: tokio::task::JoinHandle<Result<PaladinResult, PaladinError>> =
                tokio::spawn(async move {
                    // Acquire semaphore permit if concurrency limiting is enabled
                    let _permit = if let Some(sem) = &semaphore_clone {
                        Some(sem.acquire().await.unwrap())
                    } else {
                        None
                    };

                    debug!("Executing Paladin: {}", paladin_clone.node.name);
                    port.execute(&paladin_clone, &input_clone).await
                });

            tasks.push(task);
        }

        // Wait for all tasks to complete
        let mut results = Vec::new();
        let mut errors = Vec::new();

        for (i, task) in tasks.into_iter().enumerate() {
            match task.await {
                Ok(Ok(result)) => results.push(result),
                Ok(Err(e)) => {
                    let paladin_name = &phalanx.paladins()[i].node.name;
                    errors.push(format!("{}: {}", paladin_name, e));
                }
                Err(e) => {
                    let paladin_name = &phalanx.paladins()[i].node.name;
                    errors.push(format!("{}: Task join error: {}", paladin_name, e));
                }
            }
        }

        Ok((results, errors))
    }

    /// FirstSuccess: Return first successful result (early termination)
    async fn execute_first_success(
        &self,
        phalanx: &Phalanx,
        input: &str,
    ) -> Result<(Vec<PaladinResult>, Vec<String>), BattalionError> {
        let mut futures: Vec<BoxFuture<Result<PaladinResult, BattalionError>>> = Vec::new();

        for paladin in phalanx.paladins() {
            let paladin_clone: paladin_core::platform::container::paladin::Paladin =
                paladin.clone();
            let input_clone = input.to_string();
            let port = self.paladin_port.clone();

            let fut: BoxFuture<Result<PaladinResult, BattalionError>> = async move {
                port.execute(&paladin_clone, &input_clone)
                    .await
                    .map_err(|e| BattalionError::PaladinError(e.to_string()))
            }
            .boxed();

            futures.push(fut);
        }

        // Use select_ok to get first successful result
        match select_ok(futures).await {
            Ok((result, _remaining)) => {
                info!("FirstSuccess: Got first successful result");
                Ok((vec![result], vec![]))
            }
            Err(e) => {
                // All failed
                Err(BattalionError::ExecutionError(format!(
                    "All Paladins failed: {}",
                    e
                )))
            }
        }
    }

    /// Majority: Require consensus (≥50% agreement)
    async fn execute_majority(
        &self,
        phalanx: &Phalanx,
        input: &str,
    ) -> Result<(Vec<PaladinResult>, Vec<String>), BattalionError> {
        // First collect all results
        let (results, errors) = self.execute_collect_all(phalanx, input).await?;

        if results.is_empty() {
            return Err(BattalionError::ExecutionError(
                "No Paladin results to determine majority".to_string(),
            ));
        }

        // Count output occurrences
        let mut output_counts: HashMap<String, usize> = HashMap::new();
        for result in &results {
            *output_counts.entry(result.output.clone()).or_insert(0) += 1;
        }

        // Find majority (>50% threshold)
        let total_count = results.len();
        let majority_threshold = (total_count / 2) + 1;

        let majority_output = output_counts
            .iter()
            .find(|(_, count)| **count >= majority_threshold)
            .map(|(output, _)| output.clone());

        match majority_output {
            Some(output) => {
                info!(
                    "Majority consensus reached: {} out of {} Paladins agreed",
                    output_counts.get(&output).unwrap(),
                    total_count
                );
                // Return only the majority result
                let majority_result = results.into_iter().find(|r| r.output == output).unwrap();
                Ok((vec![majority_result], errors))
            }
            None => Err(BattalionError::ExecutionError(
                "No majority consensus reached".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use paladin_core::base::entity::node::Node;
    use paladin_core::platform::container::battalion::BattalionConfig;
    use paladin_core::platform::container::paladin::MaxLoops;
    use paladin_core::platform::container::paladin::{Paladin, PaladinData, PaladinStatus};
    use paladin_core::platform::container::paladin_error::PaladinError;
    use paladin_ports::output::paladin_port::StopReason;
    use std::sync::Mutex;

    /// Mock PaladinPort for testing
    struct MockPaladinPort {
        call_count: Arc<Mutex<usize>>,
        fail_paladin_names: Arc<Mutex<Vec<String>>>,
        delay_ms: u64,
        output_override: Arc<Mutex<HashMap<String, String>>>,
    }

    impl MockPaladinPort {
        fn new() -> Self {
            Self {
                call_count: Arc::new(Mutex::new(0)),
                fail_paladin_names: Arc::new(Mutex::new(Vec::new())),
                delay_ms: 10,
                output_override: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        fn with_failures(self, names: Vec<String>) -> Self {
            *self.fail_paladin_names.lock().unwrap() = names;
            self
        }

        fn with_output_override(self, overrides: HashMap<String, String>) -> Self {
            *self.output_override.lock().unwrap() = overrides;
            self
        }
    }

    #[async_trait]
    impl PaladinPort for MockPaladinPort {
        async fn execute(
            &self,
            paladin: &Paladin,
            input: &str,
        ) -> Result<PaladinResult, PaladinError> {
            *self.call_count.lock().unwrap() += 1;

            tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;

            // Check if this Paladin should fail
            let should_fail = self
                .fail_paladin_names
                .lock()
                .unwrap()
                .contains(&paladin.node.name);

            if should_fail {
                return Err(PaladinError::ExecutionError(format!(
                    "Mock failure for {}",
                    paladin.node.name
                )));
            }

            // Check for output override
            let output = if let Some(override_output) =
                self.output_override.lock().unwrap().get(&paladin.node.name)
            {
                override_output.clone()
            } else {
                format!("{}: {}", paladin.node.name, input)
            };

            Ok(PaladinResult {
                output,
                usage: TokenUsage::new(50, 0),
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
            let (_tx, rx) = mpsc::channel(1);
            Ok(rx)
        }

        fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
            Ok(())
        }
    }

    fn create_paladin(name: &str) -> Paladin {
        let data = PaladinData {
            system_prompt: format!("{} prompt", name),
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
    async fn test_phalanx_service_creation() {
        let mock_port = Arc::new(MockPaladinPort::new());
        let _service = PhalanxExecutionService::new(mock_port);
    }

    #[tokio::test]
    async fn test_collect_all_strategy_success() {
        let p1 = create_paladin("Agent1");
        let p2 = create_paladin("Agent2");
        let p3 = create_paladin("Agent3");

        let phalanx =
            Phalanx::new(vec![p1, p2, p3], BattalionConfig::new("test_collect_all")).unwrap();

        let mock_port = Arc::new(MockPaladinPort::new());
        let service = PhalanxExecutionService::new(mock_port);

        let result = service.execute(&phalanx, "Test input").await;

        assert!(result.is_ok());
        let battalion_result = result.unwrap();
        assert_eq!(battalion_result.paladin_results.len(), 3);
        assert_eq!(battalion_result.status, BattalionStatus::Completed);
    }

    #[tokio::test]
    async fn test_collect_all_with_concurrency_limit() {
        let paladins: Vec<Paladin> = (1..=10)
            .map(|i| create_paladin(&format!("Agent{}", i)))
            .collect();

        let phalanx = Phalanx::new(paladins, BattalionConfig::new("test_concurrency"))
            .unwrap()
            .with_max_concurrency(3);

        let mock_port = Arc::new(MockPaladinPort::new());
        let service = PhalanxExecutionService::new(mock_port);

        let result = service.execute(&phalanx, "Test input").await;

        assert!(result.is_ok());
        let battalion_result = result.unwrap();
        assert_eq!(battalion_result.paladin_results.len(), 10);
    }

    #[tokio::test]
    async fn test_first_success_strategy() {
        let p1 = create_paladin("Agent1");
        let p2 = create_paladin("Agent2");
        let p3 = create_paladin("Agent3");

        let phalanx = Phalanx::new(vec![p1, p2, p3], BattalionConfig::new("test_first"))
            .unwrap()
            .with_aggregation(AggregationStrategy::FirstSuccess);

        let mock_port = Arc::new(MockPaladinPort::new());
        let service = PhalanxExecutionService::new(mock_port);

        let result = service.execute(&phalanx, "Test input").await;

        assert!(result.is_ok());
        let battalion_result = result.unwrap();
        // FirstSuccess returns only one result
        assert_eq!(battalion_result.paladin_results.len(), 1);
    }

    #[tokio::test]
    async fn test_majority_strategy_with_consensus() {
        let p1 = create_paladin("Agent1");
        let p2 = create_paladin("Agent2");
        let p3 = create_paladin("Agent3");

        let phalanx = Phalanx::new(vec![p1, p2, p3], BattalionConfig::new("test_majority"))
            .unwrap()
            .with_aggregation(AggregationStrategy::Majority);

        // Set up so Agent1 and Agent2 return "Result A", Agent3 returns different
        let mut overrides = HashMap::new();
        overrides.insert("Agent1".to_string(), "Result A".to_string());
        overrides.insert("Agent2".to_string(), "Result A".to_string());
        overrides.insert("Agent3".to_string(), "Result B".to_string());

        let mock_port = Arc::new(MockPaladinPort::new().with_output_override(overrides));
        let service = PhalanxExecutionService::new(mock_port);

        let result = service.execute(&phalanx, "Test input").await;

        assert!(result.is_ok());
        let battalion_result = result.unwrap();
        assert_eq!(battalion_result.paladin_results.len(), 1);
        assert_eq!(battalion_result.paladin_results[0].output, "Result A");
    }

    #[tokio::test]
    async fn test_majority_strategy_validation() {
        let p1 = create_paladin("Agent1");
        let p2 = create_paladin("Agent2");

        let phalanx = Phalanx::new(vec![p1, p2], BattalionConfig::new("test_majority_invalid"))
            .unwrap()
            .with_aggregation(AggregationStrategy::Majority);

        let mock_port = Arc::new(MockPaladinPort::new());
        let service = PhalanxExecutionService::new(mock_port);

        let result = service.execute(&phalanx, "Test input").await;

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("at least 3 Paladins")
        );
    }

    #[tokio::test]
    async fn test_partial_failures_with_continue_on_error() {
        let p1 = create_paladin("Agent1");
        let p2 = create_paladin("Agent2");
        let p3 = create_paladin("Agent3");

        let config = BattalionConfig::new("test_partial_fail")
            .with_error_strategy(ErrorStrategy::ContinueOnError);

        let phalanx = Phalanx::new(vec![p1, p2, p3], config).unwrap();

        let mock_port = Arc::new(MockPaladinPort::new().with_failures(vec!["Agent2".to_string()]));
        let service = PhalanxExecutionService::new(mock_port);

        let result = service.execute(&phalanx, "Test input").await;

        assert!(result.is_ok());
        let battalion_result = result.unwrap();
        // Only 2 successful results (Agent1 and Agent3)
        assert_eq!(battalion_result.paladin_results.len(), 2);

        // node_errors carries the real failed node's name + error text (D-03 / MERGE-04)
        assert_eq!(battalion_result.node_errors.len(), 1);
        assert_eq!(battalion_result.node_errors[0].node_name, "Agent2");
        assert!(
            battalion_result.node_errors[0]
                .error
                .contains("Mock failure for Agent2")
        );
    }

    #[tokio::test]
    async fn test_timeout_enforcement() {
        let p1 = create_paladin("Agent1");
        let p2 = create_paladin("Agent2");

        let config = BattalionConfig::new("test_timeout").with_timeout(1);

        let phalanx = Phalanx::new(vec![p1, p2], config).unwrap();

        let mut mock_port = MockPaladinPort::new();
        mock_port.delay_ms = 2000; // 2 seconds > 1 second timeout

        let service = PhalanxExecutionService::new(Arc::new(mock_port));

        let result = service.execute(&phalanx, "Test input").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            BattalionError::Timeout(seconds) => assert_eq!(seconds, 1),
            _ => panic!("Expected Timeout error"),
        }
    }

    #[tokio::test]
    async fn test_cancellation_support() {
        let p1 = create_paladin("Agent1");
        let p2 = create_paladin("Agent2");

        let phalanx = Phalanx::new(vec![p1, p2], BattalionConfig::new("test_cancel")).unwrap();

        let mut mock_port = MockPaladinPort::new();
        mock_port.delay_ms = 1000; // 1 second delay

        let service = PhalanxExecutionService::new(Arc::new(mock_port));
        let cancellation_token = CancellationToken::new();
        let token_clone = cancellation_token.clone();

        // Cancel after 100ms
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            token_clone.cancel();
        });

        let result = service
            .execute_with_cancellation(&phalanx, "Test input", cancellation_token)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            BattalionError::Cancelled => {}
            _ => panic!("Expected Cancelled error"),
        }
    }

    #[tokio::test]
    async fn test_phalanx_per_paladin_timing() {
        let p1 = create_paladin("Analyst");
        let p2 = create_paladin("Reviewer");
        let p3 = create_paladin("Editor");

        let phalanx = Phalanx::new(vec![p1, p2, p3], BattalionConfig::new("timing_test")).unwrap();

        let mock_port = Arc::new(MockPaladinPort::new());
        let service = PhalanxExecutionService::new(mock_port);

        let result = service.execute(&phalanx, "Test input").await.unwrap();

        // per_paladin_times should be populated with entries for each Paladin
        assert_eq!(result.per_paladin_times.len(), 3);
        assert!(result.per_paladin_times.contains_key("Analyst"));
        assert!(result.per_paladin_times.contains_key("Reviewer"));
        assert!(result.per_paladin_times.contains_key("Editor"));

        // All times should be > 0 (mock has 10ms delay)
        for time_ms in result.per_paladin_times.values() {
            assert!(*time_ms > 0, "Paladin execution time should be > 0");
        }
    }

    #[tokio::test]
    async fn test_phalanx_per_paladin_tokens() {
        let p1 = create_paladin("Analyst");
        let p2 = create_paladin("Reviewer");

        let phalanx = Phalanx::new(vec![p1, p2], BattalionConfig::new("tokens_test")).unwrap();

        let mock_port = Arc::new(MockPaladinPort::new());
        let service = PhalanxExecutionService::new(mock_port);

        let result = service.execute(&phalanx, "Test input").await.unwrap();

        // per_paladin_tokens should be populated from PaladinResult.token_count
        assert_eq!(result.per_paladin_tokens.len(), 2);
        assert!(result.per_paladin_tokens.contains_key("Analyst"));
        assert!(result.per_paladin_tokens.contains_key("Reviewer"));

        // Mock returns token_count=50, so total_tokens for each should be 50
        let analyst_tokens = result.per_paladin_tokens.get("Analyst").unwrap();
        assert_eq!(analyst_tokens.total_tokens, 50);

        // total_tokens should be the sum across all paladins
        assert_eq!(result.total_tokens, 100); // 50 + 50
    }

    #[tokio::test]
    async fn test_phalanx_metrics_with_partial_failures() {
        let p1 = create_paladin("Success1");
        let p2 = create_paladin("Failure1");
        let p3 = create_paladin("Success2");

        let config = BattalionConfig::new("partial_metrics")
            .with_error_strategy(ErrorStrategy::ContinueOnError);

        let phalanx = Phalanx::new(vec![p1, p2, p3], config).unwrap();

        let mock_port =
            Arc::new(MockPaladinPort::new().with_failures(vec!["Failure1".to_string()]));
        let service = PhalanxExecutionService::new(mock_port);

        let result = service.execute(&phalanx, "Test input").await.unwrap();

        // Only successful paladins should have timing and token entries
        assert_eq!(result.per_paladin_times.len(), 2);
        assert!(result.per_paladin_times.contains_key("Success1"));
        assert!(result.per_paladin_times.contains_key("Success2"));
        assert!(!result.per_paladin_times.contains_key("Failure1"));

        assert_eq!(result.per_paladin_tokens.len(), 2);
        assert!(!result.per_paladin_tokens.contains_key("Failure1"));

        // total_tokens should only count successful paladins
        assert_eq!(result.total_tokens, 100); // 50 + 50

        // Success/failure counts should be accurate
        assert_eq!(result.paladin_success_count, 2);
        assert_eq!(result.paladin_failure_count, 1);

        // node_errors carries the real failed node's name + error text (D-03 / MERGE-04)
        assert_eq!(result.node_errors.len(), 1);
        assert_eq!(result.node_errors[0].node_name, "Failure1");
        assert!(
            result.node_errors[0]
                .error
                .contains("Mock failure for Failure1")
        );
    }

    #[tokio::test]
    async fn test_phalanx_node_errors_empty_on_full_success() {
        let p1 = create_paladin("Agent1");
        let p2 = create_paladin("Agent2");

        let phalanx = Phalanx::new(vec![p1, p2], BattalionConfig::new("no_errors_test")).unwrap();

        let mock_port = Arc::new(MockPaladinPort::new());
        let service = PhalanxExecutionService::new(mock_port);

        let result = service.execute(&phalanx, "Test input").await.unwrap();

        // A fully-successful Phalanx run has zero node_errors entries.
        assert!(result.node_errors.is_empty());
    }

    #[tokio::test]
    async fn test_phalanx_metrics_success_failure_counts() {
        let p1 = create_paladin("Agent1");
        let p2 = create_paladin("Agent2");
        let p3 = create_paladin("Agent3");

        let phalanx = Phalanx::new(vec![p1, p2, p3], BattalionConfig::new("count_test")).unwrap();

        let mock_port = Arc::new(MockPaladinPort::new());
        let service = PhalanxExecutionService::new(mock_port);

        let result = service.execute(&phalanx, "Test input").await.unwrap();

        // All succeed
        assert_eq!(result.paladin_success_count, 3);
        assert_eq!(result.paladin_failure_count, 0);
    }
}
