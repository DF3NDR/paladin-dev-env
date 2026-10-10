//! Phalanx Execution Service
//!
//! Provides orchestration logic for executing Paladins in concurrent Phalanx pattern.

use chrono::Utc;
use futures::future::{BoxFuture, FutureExt, select_ok};
use log::{debug, info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::aegis_attempt::{self, AttemptFailure};
use paladin_core::platform::container::aegis::{Aegis, ErrorHandlerSpec};
use paladin_core::platform::container::battalion::phalanx::{AggregationStrategy, Phalanx};
use paladin_core::platform::container::battalion::{
    BattalionError, BattalionResult, BattalionStatus, BattalionStrategy, NodeError,
};
use paladin_core::platform::container::herald::Herald;
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult};

#[cfg(test)]
use paladin_core::platform::container::battalion::TokenUsage;

#[cfg(test)]
use tokio::sync::mpsc;

/// A Paladin's name paired with its successful result.
type Success = (String, PaladinResult);

/// Successes and typed failures, both in Paladin declaration order.
type Collected = (Vec<Success>, Vec<AttemptFailure>);

/// Service for executing Phalanx patterns
///
/// Orchestrates concurrent Paladin execution with configurable aggregation strategies,
/// concurrency limiting via semaphore, and cancellation support. Every Paladin attempt
/// runs under the Phalanx's Aegis policy: a per-attempt timeout, independent retry per
/// Paladin, and continue-past-failure through `aegis.on_error`.
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
    /// Every Paladin runs under the Phalanx's [`Aegis`]
    /// (`BattalionConfig.aegis`):
    ///
    /// - **Timeout:** `aegis.timeout` bounds each attempt of each Paladin, not
    ///   the whole run -- there is no whole-run wall clock, so two Paladins
    ///   that take 600 ms each both complete under a one second bound.
    /// - **Retry:** `aegis.retry` retries each Paladin independently; one
    ///   Paladin sleeping in backoff never delays another Paladin's result. A
    ///   concurrency-limited Phalanx (`with_max_concurrency`) holds one permit
    ///   for a Paladin's whole attempt sequence.
    /// - **Failures:** failures are collected in Paladin declaration order. With
    ///   `aegis.on_error = None` the run then fails with
    ///   [`BattalionError::AggregationError`] listing every failure; with
    ///   `Some(Absorb { .. })` the run completes with one `node_errors` entry per
    ///   failed Paladin.
    ///
    /// # Errors
    ///
    /// * [`BattalionError::ValidationError`] -- the Aegis policy cannot be
    ///   honoured by a Phalanx, or the aggregation strategy's requirements are
    ///   not met; no Paladin runs
    /// * [`BattalionError::AggregationError`] -- one or more Paladins failed and
    ///   `aegis.on_error` is `None`
    pub async fn execute(
        &self,
        phalanx: &Phalanx,
        input: &str,
    ) -> Result<BattalionResult, BattalionError> {
        // Reject Aegis policies a Phalanx cannot honour before any Paladin runs.
        phalanx.config().validate_aegis()?;

        info!(
            "Starting Phalanx execution: {} with {} Paladins",
            phalanx.config().name,
            phalanx.paladin_count()
        );

        self.execute_internal(phalanx, input, &None).await
    }

    /// Execute Phalanx with cancellation support
    ///
    /// Allows external cancellation of ongoing execution. Like
    /// [`Self::execute`] it has no whole-run timeout; each attempt is bounded by
    /// `aegis.timeout`. When `cancellation_token` fires the call returns
    /// [`BattalionError::Cancelled`], and a Paladin waiting in retry backoff
    /// stops retrying.
    pub async fn execute_with_cancellation(
        &self,
        phalanx: &Phalanx,
        input: &str,
        cancellation_token: CancellationToken,
    ) -> Result<BattalionResult, BattalionError> {
        // Reject Aegis policies a Phalanx cannot honour before any Paladin runs.
        phalanx.config().validate_aegis()?;

        let token = Some(cancellation_token.clone());
        // `biased` with the token first: once it fires, the run reports
        // `Cancelled` deterministically, even when a Paladin that stopped
        // retrying because of the same token has already produced its failure.
        tokio::select! {
            biased;
            _ = cancellation_token.cancelled() => {
                info!("Phalanx '{}' cancelled", phalanx.config().name);
                Err(BattalionError::Cancelled)
            }
            result = self.execute_internal(phalanx, input, &token) => result,
        }
    }

    /// Internal execution logic
    async fn execute_internal(
        &self,
        phalanx: &Phalanx,
        input: &str,
        cancel: &Option<CancellationToken>,
    ) -> Result<BattalionResult, BattalionError> {
        let config = phalanx.config();
        let started_at = Utc::now();
        let battalion_id = Uuid::new_v4();

        // Validate aggregation strategy
        self.validate_aggregation_strategy(phalanx)?;

        // Execute based on aggregation strategy
        let (successes, failures) = match phalanx.aggregation_strategy() {
            AggregationStrategy::CollectAll => {
                self.execute_collect_all(phalanx, input, cancel).await?
            }
            AggregationStrategy::FirstSuccess => {
                self.execute_first_success(phalanx, input, cancel).await?
            }
            AggregationStrategy::Majority => self.execute_majority(phalanx, input, cancel).await?,
            AggregationStrategy::Custom(fn_name) => {
                return Err(BattalionError::ConfigurationError(format!(
                    "Custom aggregation '{}' not yet implemented",
                    fn_name
                )));
            }
        };

        // Handle failures according to the Aegis error handler (D-03, D-05).
        if !failures.is_empty() {
            match &config.aegis.on_error {
                None => {
                    // Collect-then-fail: every failure is reported, each as its
                    // structured NodeError display (a per-attempt timeout reads
                    // `run timeout`), never a re-parsed string.
                    let joined = failures
                        .iter()
                        .map(|failure| failure.node_error.to_string())
                        .collect::<Vec<_>>()
                        .join("; ");
                    return Err(BattalionError::AggregationError(format!(
                        "Phalanx execution failed with {} errors: {}",
                        failures.len(),
                        joined
                    )));
                }
                Some(ErrorHandlerSpec::Absorb { .. }) => {
                    warn!(
                        "Phalanx '{}' completed with {} errors (Absorb)",
                        config.name,
                        failures.len()
                    );
                }
                // `ErrorHandlerSpec` is `#[non_exhaustive]`; `validate_aegis`
                // already rejects `Route` and `Custom`, so this is reached only
                // by a future variant. Fail closed, never continue.
                Some(other) => {
                    return Err(BattalionError::ValidationError(format!(
                        "Phalanx does not support the aegis.on_error handler {other:?}"
                    )));
                }
            }
        }

        // 44-06 replaces this v0.10 summary shape with the structured
        // `failure.node_error`. Declaration order is preserved by the collector.
        let node_errors: Vec<NodeError> = failures
            .iter()
            .map(|failure| NodeError {
                node_name: failure.node_error.node_id.as_str().to_string(),
                error: failure.error.to_string(),
            })
            .collect();

        // Per-Paladin metrics are keyed by the Paladins that actually
        // succeeded; no failure string is parsed back into a name.
        let mut per_paladin_times = HashMap::new();
        let mut per_paladin_tokens = HashMap::new();
        let mut total_tokens: u64 = 0;
        let mut paladin_results = Vec::with_capacity(successes.len());

        for (name, result) in successes {
            per_paladin_times.insert(name.clone(), result.execution_time_ms);
            per_paladin_tokens.insert(name, result.usage.clone());
            total_tokens += u64::from(result.usage.total_tokens);
            paladin_results.push(result);
        }

        // Determine final output based on aggregation
        let final_output = paladin_results
            .last()
            .map(|result| result.output.clone())
            .unwrap_or_default();

        let paladin_success_count = paladin_results.len();
        let paladin_failure_count = failures.len();

        let completed_at = Utc::now();
        Ok(BattalionResult {
            battalion_id,
            battalion_name: config.name.clone(),
            paladin_results,
            started_at,
            completed_at,
            final_output,
            status: BattalionStatus::Completed,
            strategy_used: BattalionStrategy::Phalanx,
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
    ///
    /// Every Paladin runs in its own task through the Aegis runner. Successes
    /// and failures both follow declaration order because the join handles are
    /// awaited in the order the Paladins were declared, not completion order.
    async fn execute_collect_all(
        &self,
        phalanx: &Phalanx,
        input: &str,
        cancel: &Option<CancellationToken>,
    ) -> Result<Collected, BattalionError> {
        let semaphore = phalanx
            .max_concurrency()
            .map(|max| Arc::new(Semaphore::new(max)));
        let mut tasks = Vec::with_capacity(phalanx.paladin_count());

        for paladin in phalanx.paladins() {
            let name = paladin.node.name.clone();
            let paladin_clone = paladin.clone();
            let input_clone = input.to_string();
            let port = self.paladin_port.clone();
            let semaphore_clone = semaphore.clone();
            let aegis = phalanx.config().aegis.clone();
            let cancel_clone = cancel.clone();

            let task: tokio::task::JoinHandle<Result<Success, AttemptFailure>> =
                tokio::spawn(run_paladin(
                    port,
                    paladin_clone,
                    input_clone,
                    aegis,
                    cancel_clone,
                    semaphore_clone,
                ));

            tasks.push((name, task));
        }

        // Wait for all tasks to complete, in declaration order.
        let mut successes = Vec::new();
        let mut failures = Vec::new();

        for (name, task) in tasks {
            match task.await {
                Ok(Ok(success)) => successes.push(success),
                Ok(Err(failure)) => failures.push(failure),
                Err(e) => failures.push(AttemptFailure::from_error(
                    &name,
                    1,
                    PaladinError::ExecutionError(format!("Task join error: {e}")),
                )),
            }
        }

        Ok((successes, failures))
    }

    /// FirstSuccess: Return first successful result (early termination)
    async fn execute_first_success(
        &self,
        phalanx: &Phalanx,
        input: &str,
        cancel: &Option<CancellationToken>,
    ) -> Result<Collected, BattalionError> {
        let mut futures: Vec<BoxFuture<Result<Success, AttemptFailure>>> = Vec::new();

        for paladin in phalanx.paladins() {
            let paladin_clone = paladin.clone();
            let input_clone = input.to_string();
            let port = self.paladin_port.clone();
            let aegis = phalanx.config().aegis.clone();
            let cancel_clone = cancel.clone();

            let fut: BoxFuture<Result<Success, AttemptFailure>> = async move {
                aegis_attempt::run_with_aegis(
                    &port,
                    &paladin_clone,
                    &input_clone,
                    &aegis,
                    &cancel_clone,
                )
                .await
                .map(|outcome| (paladin_clone.node.name.clone(), outcome.result))
            }
            .boxed();

            futures.push(fut);
        }

        if futures.is_empty() {
            return Err(BattalionError::ExecutionError(
                "All Paladins failed: no Paladins to run".to_string(),
            ));
        }

        // Use select_ok to get first successful result
        match select_ok(futures).await {
            Ok((success, _remaining)) => {
                info!("FirstSuccess: Got first successful result");
                Ok((vec![success], vec![]))
            }
            Err(last_failure) => {
                // All failed; report the last failure, as the v0.10 contract did.
                Err(BattalionError::ExecutionError(format!(
                    "All Paladins failed: {}",
                    BattalionError::PaladinError(last_failure.error.to_string())
                )))
            }
        }
    }

    /// Majority: Require consensus (≥50% agreement)
    async fn execute_majority(
        &self,
        phalanx: &Phalanx,
        input: &str,
        cancel: &Option<CancellationToken>,
    ) -> Result<Collected, BattalionError> {
        // First collect all results
        let (successes, failures) = self.execute_collect_all(phalanx, input, cancel).await?;

        if successes.is_empty() {
            return Err(BattalionError::ExecutionError(
                "No Paladin results to determine majority".to_string(),
            ));
        }

        // Count output occurrences
        let mut output_counts: HashMap<&str, usize> = HashMap::new();
        for (_, result) in &successes {
            *output_counts.entry(result.output.as_str()).or_insert(0) += 1;
        }

        // Find majority (>50% threshold)
        let total_count = successes.len();
        let majority_threshold = (total_count / 2) + 1;

        let majority = output_counts
            .iter()
            .find(|(_, count)| **count >= majority_threshold)
            .map(|(output, count)| ((*output).to_string(), *count));

        let Some((output, agreed)) = majority else {
            return Err(BattalionError::ExecutionError(
                "No majority consensus reached".to_string(),
            ));
        };

        info!(
            "Majority consensus reached: {} out of {} Paladins agreed",
            agreed, total_count
        );

        // Return only the majority result (the first Paladin, in declaration
        // order, that produced it).
        let majority_success = successes
            .into_iter()
            .find(|(_, result)| result.output == output)
            .ok_or_else(|| {
                BattalionError::ExecutionError(
                    "Majority output vanished while selecting its result".to_string(),
                )
            })?;
        Ok((vec![majority_success], failures))
    }
}

/// Run one Paladin to its final outcome inside its own task.
///
/// Holds one concurrency permit (when the Phalanx is limited) across the
/// Paladin's whole attempt sequence, so the limit also bounds retry
/// amplification (T-44-12). A closed limiter is a recorded failure, never a
/// panic.
async fn run_paladin(
    port: Arc<dyn PaladinPort>,
    paladin: Paladin,
    input: String,
    aegis: Aegis,
    cancel: Option<CancellationToken>,
    semaphore: Option<Arc<Semaphore>>,
) -> Result<Success, AttemptFailure> {
    let name = paladin.node.name.clone();
    let _permit = match &semaphore {
        Some(sem) => match sem.acquire().await {
            Ok(permit) => Some(permit),
            Err(e) => {
                return Err(AttemptFailure::from_error(
                    &name,
                    1,
                    PaladinError::ExecutionError(format!("Concurrency limiter closed: {e}")),
                ));
            }
        },
        None => None,
    };

    debug!("Executing Paladin: {}", name);
    aegis_attempt::run_with_aegis(&port, &paladin, &input, &aegis, &cancel)
        .await
        .map(|outcome| (name, outcome.result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use paladin_core::base::entity::node::Node;
    use paladin_core::platform::container::aegis::{
        Aegis, RetryPolicy as AegisRetryPolicy, RetryPredicate, TimeoutPolicy,
    };
    use paladin_core::platform::container::battalion::BattalionConfig;
    use paladin_core::platform::container::battlefield::StateDelta;
    use paladin_core::platform::container::node_error::{NodeErrorSource, TimeoutKind};
    use paladin_core::platform::container::paladin::MaxLoops;
    use paladin_core::platform::container::paladin::{Paladin, PaladinData, PaladinStatus};
    use paladin_core::platform::container::paladin_error::PaladinError;
    use paladin_core::platform::container::transience::Transience;
    use paladin_ports::output::paladin_port::StopReason;
    use std::collections::VecDeque;
    use std::sync::Mutex;
    use std::time::Duration;

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

    fn absorb_aegis() -> Aegis {
        Aegis {
            on_error: Some(ErrorHandlerSpec::Absorb {
                fallback_delta: StateDelta::new(),
            }),
            ..Aegis::default()
        }
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
    async fn test_partial_failures_with_absorb() {
        let p1 = create_paladin("Agent1");
        let p2 = create_paladin("Agent2");
        let p3 = create_paladin("Agent3");

        let config = BattalionConfig::new("test_partial_fail").with_aegis(absorb_aegis());

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

        let config = BattalionConfig::new("partial_metrics").with_aegis(absorb_aegis());

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

    // ---- Aegis-configured tests (Phase 44) -------------------------------------

    /// One scripted call for a named Paladin: wait `delay`, then answer.
    struct Step {
        delay: Duration,
        fail: Option<PaladinError>,
    }

    fn slow(delay: Duration) -> Step {
        Step { delay, fail: None }
    }

    fn failing_after(delay: Duration, error: PaladinError) -> Step {
        Step {
            delay,
            fail: Some(error),
        }
    }

    fn failing(error: PaladinError) -> Step {
        failing_after(Duration::ZERO, error)
    }

    fn execution_error(message: &str) -> PaladinError {
        PaladinError::ExecutionError(message.to_string())
    }

    fn transient() -> PaladinError {
        PaladinError::LlmFailure {
            transience: Transience::Transient,
            status: Some(503),
            provider: None,
            message: "unavailable".to_string(),
        }
    }

    /// A port that plays back a per-Paladin script (an exhausted or absent
    /// script answers with an immediate success) and records every call as a
    /// `start <name>` / `end <name>` event plus the clock reading at each
    /// successful return. Sleeps with `tokio::time::sleep`, so a paused clock
    /// drives the timing.
    struct RecordingPort {
        scripts: Mutex<HashMap<String, VecDeque<Step>>>,
        events: Mutex<Vec<String>>,
        finished_at: Mutex<HashMap<String, tokio::time::Instant>>,
    }

    impl RecordingPort {
        fn scripted(scripts: Vec<(&str, Vec<Step>)>) -> Arc<Self> {
            Arc::new(Self {
                scripts: Mutex::new(
                    scripts
                        .into_iter()
                        .map(|(name, steps)| (name.to_string(), steps.into()))
                        .collect(),
                ),
                events: Mutex::new(Vec::new()),
                finished_at: Mutex::new(HashMap::new()),
            })
        }

        fn events(&self) -> Vec<String> {
            self.events.lock().map(|e| e.clone()).unwrap_or_default()
        }

        fn calls_for(&self, name: &str) -> usize {
            let start = format!("start {name}");
            self.events().iter().filter(|e| **e == start).count()
        }

        fn total_calls(&self) -> usize {
            self.events()
                .iter()
                .filter(|e| e.starts_with("start "))
                .count()
        }

        fn finished_at(&self, name: &str) -> Option<tokio::time::Instant> {
            self.finished_at
                .lock()
                .ok()
                .and_then(|m| m.get(name).copied())
        }

        fn record(&self, event: String) {
            if let Ok(mut events) = self.events.lock() {
                events.push(event);
            }
        }
    }

    #[async_trait]
    impl PaladinPort for RecordingPort {
        async fn execute(
            &self,
            paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinResult, PaladinError> {
            let name = paladin.node.name.clone();
            self.record(format!("start {name}"));
            let step = self
                .scripts
                .lock()
                .ok()
                .and_then(|mut s| s.get_mut(&name).and_then(|q| q.pop_front()));
            if let Some(step) = step {
                tokio::time::sleep(step.delay).await;
                if let Some(error) = step.fail {
                    self.record(format!("end {name}"));
                    return Err(error);
                }
            }
            self.record(format!("end {name}"));
            if let Ok(mut finished) = self.finished_at.lock() {
                finished.insert(name.clone(), tokio::time::Instant::now());
            }

            Ok(PaladinResult {
                output: format!("{name} done"),
                usage: TokenUsage::new(50, 0),
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

    fn phalanx_with(names: &[&str], aegis: Aegis) -> Phalanx {
        let paladins = names.iter().map(|n| create_paladin(n)).collect();
        Phalanx::new(
            paladins,
            BattalionConfig::new("test_phalanx").with_aegis(aegis),
        )
        .expect("a non-empty Phalanx is valid")
    }

    fn run_timeout(duration: Duration) -> Option<TimeoutPolicy> {
        Some(TimeoutPolicy {
            run_timeout: Some(duration),
            idle_timeout: None,
        })
    }

    fn timeout_aegis(duration: Duration) -> Aegis {
        Aegis {
            timeout: run_timeout(duration),
            ..Aegis::default()
        }
    }

    fn retry_aegis(max_attempts: u32, initial_interval: Duration) -> Aegis {
        Aegis {
            retry: Some(AegisRetryPolicy {
                max_attempts,
                initial_interval,
                jitter: false,
                retry_on: RetryPredicate::TransientOnly,
                ..AegisRetryPolicy::default()
            }),
            ..Aegis::default()
        }
    }

    #[tokio::test(start_paused = true)]
    async fn phalanx_bounds_each_attempt_not_the_run() {
        // D-02: two 600 ms Paladins under a 1 s per-attempt bound both
        // complete; there is no whole-run wall clock.
        let aegis = timeout_aegis(Duration::from_secs(1));
        let phalanx = phalanx_with(&["P1", "P2"], aegis.clone());
        let delay = Duration::from_millis(600);
        let port =
            RecordingPort::scripted(vec![("P1", vec![slow(delay)]), ("P2", vec![slow(delay)])]);
        let service = PhalanxExecutionService::new(port.clone());

        let started = tokio::time::Instant::now();
        let result = service
            .execute(&phalanx, "go")
            .await
            .expect("each 600 ms Paladin is inside its 1 s bound");
        assert_eq!(result.status, BattalionStatus::Completed);
        assert_eq!(result.paladin_results.len(), 2);
        assert_eq!(started.elapsed(), delay, "the Paladins run concurrently");

        // A 2 s Paladin under the same bound fails its own attempt at 1 s.
        let phalanx = phalanx_with(&["P1", "P2"], aegis);
        let port = RecordingPort::scripted(vec![("P1", vec![slow(Duration::from_secs(2))])]);
        let service = PhalanxExecutionService::new(port);
        let started = tokio::time::Instant::now();
        service
            .execute(&phalanx, "go")
            .await
            .expect_err("2 s exceeds the 1 s per-attempt bound");
        assert_eq!(started.elapsed(), Duration::from_secs(1));
    }

    #[tokio::test(start_paused = true)]
    async fn phalanx_fail_fast_aggregates_structured_failures() {
        // D-03: with on_error None every failure is collected and listed by its
        // structured NodeError display; a per-attempt timeout reads `run timeout`.
        let phalanx = phalanx_with(&["Slow", "Fast"], timeout_aegis(Duration::from_secs(1)));
        let port = RecordingPort::scripted(vec![("Slow", vec![slow(Duration::from_secs(2))])]);
        let service = PhalanxExecutionService::new(port);

        match service.execute(&phalanx, "go").await {
            Err(BattalionError::AggregationError(message)) => {
                assert!(
                    message.starts_with("Phalanx execution failed with 1 errors"),
                    "got {message:?}"
                );
                assert!(message.contains("run timeout"), "got {message:?}");
                assert!(message.contains("node Slow attempt 1"), "got {message:?}");
                assert!(!message.contains("Fast"), "the success is not listed");
            }
            other => panic!("expected AggregationError, got {other:?}"),
        }

        // Two failures are both listed, in declaration order.
        let phalanx = phalanx_with(&["A", "B", "C"], Aegis::default());
        let port = RecordingPort::scripted(vec![
            ("A", vec![failing(execution_error("boom a"))]),
            (
                "B",
                vec![failing_after(
                    Duration::from_millis(5),
                    execution_error("boom b"),
                )],
            ),
        ]);
        let service = PhalanxExecutionService::new(port);
        match service.execute(&phalanx, "go").await {
            Err(BattalionError::AggregationError(message)) => {
                assert!(
                    message.starts_with("Phalanx execution failed with 2 errors"),
                    "got {message:?}"
                );
                let a = message.find("node A").expect("A is listed");
                let b = message.find("node B").expect("B is listed");
                assert!(a < b, "failures follow declaration order: {message:?}");
                assert!(message.contains("boom a") && message.contains("boom b"));
            }
            other => panic!("expected AggregationError, got {other:?}"),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn phalanx_absorb_collects_every_failure() {
        // D-05 / D-07: Absorb reports Completed with one node_errors entry per
        // failed Paladin; the metrics are keyed by the Paladins that succeeded.
        let aegis = Aegis {
            on_error: absorb_aegis().on_error,
            ..timeout_aegis(Duration::from_secs(1))
        };
        let phalanx = phalanx_with(&["Slow", "Fast", "Odd"], aegis);
        let port = RecordingPort::scripted(vec![
            ("Slow", vec![slow(Duration::from_secs(2))]),
            // A message with colons must never be split back into a name.
            ("Odd", vec![failing(execution_error("x: y: z"))]),
        ]);
        let service = PhalanxExecutionService::new(port);

        let result = service
            .execute(&phalanx, "go")
            .await
            .expect("Absorb continues past failures");
        assert_eq!(result.status, BattalionStatus::Completed);
        assert_eq!(result.paladin_success_count, 1);
        assert_eq!(result.paladin_failure_count, 2);
        assert_eq!(result.node_errors.len(), 2);
        assert_eq!(result.node_errors[0].node_name, "Slow");
        assert_eq!(result.node_errors[1].node_name, "Odd");
        assert!(result.node_errors[1].error.contains("x: y: z"));

        let mut timed: Vec<&String> = result.per_paladin_times.keys().collect();
        timed.sort();
        assert_eq!(timed, vec!["Fast"]);
        let mut tokens: Vec<&String> = result.per_paladin_tokens.keys().collect();
        tokens.sort();
        assert_eq!(tokens, vec!["Fast"]);
        assert_eq!(result.total_tokens, 50);
        assert_eq!(result.final_output, "Fast done");
    }

    #[tokio::test(start_paused = true)]
    async fn phalanx_node_errors_follow_declaration_order() {
        // Determinism: a Paladin declared first that fails last is still the
        // first node_errors entry; successes follow declaration order too.
        let phalanx = phalanx_with(&["A", "B", "C", "D"], absorb_aegis());
        let port = RecordingPort::scripted(vec![
            (
                "A",
                vec![failing_after(
                    Duration::from_millis(300),
                    execution_error("a"),
                )],
            ),
            ("B", vec![slow(Duration::from_millis(100))]),
            (
                "C",
                vec![failing_after(
                    Duration::from_millis(50),
                    execution_error("c"),
                )],
            ),
            ("D", vec![slow(Duration::from_millis(10))]),
        ]);
        let service = PhalanxExecutionService::new(port);

        let result = service.execute(&phalanx, "go").await.expect("Absorb");
        let failed: Vec<&str> = result
            .node_errors
            .iter()
            .map(|e| e.node_name.as_str())
            .collect();
        assert_eq!(failed, vec!["A", "C"]);
        let outputs: Vec<&str> = result
            .paladin_results
            .iter()
            .map(|r| r.output.as_str())
            .collect();
        assert_eq!(outputs, vec!["B done", "D done"]);
        assert_eq!(result.final_output, "D done");
    }

    #[tokio::test(start_paused = true)]
    async fn phalanx_paladins_retry_independently() {
        // A fails Transient once then succeeds after a 1 s backoff; B's result
        // is not delayed by A's backoff, and the run ends at 1 s, not 1.1 s.
        let phalanx = phalanx_with(&["A", "B"], retry_aegis(2, Duration::from_secs(1)));
        let port = RecordingPort::scripted(vec![
            ("A", vec![failing(transient()), slow(Duration::ZERO)]),
            ("B", vec![slow(Duration::from_millis(100))]),
        ]);
        let service = PhalanxExecutionService::new(port.clone());

        let started = tokio::time::Instant::now();
        let result = service.execute(&phalanx, "go").await.expect("both succeed");
        assert_eq!(result.paladin_results.len(), 2);
        assert_eq!(result.paladin_failure_count, 0);
        assert_eq!(started.elapsed(), Duration::from_secs(1));
        assert_eq!(port.calls_for("A"), 2);
        assert_eq!(port.calls_for("B"), 1);
        let b_done = port.finished_at("B").expect("B finished");
        assert_eq!(
            b_done - started,
            Duration::from_millis(100),
            "B never waits for A's backoff"
        );

        // A closed limiter is a recorded failure, never a panic.
        let semaphore = Arc::new(Semaphore::new(1));
        semaphore.close();
        let port: Arc<dyn PaladinPort> = RecordingPort::scripted(vec![]);
        let failure = run_paladin(
            port,
            create_paladin("Limited"),
            "go".to_string(),
            Aegis::default(),
            None,
            Some(semaphore),
        )
        .await
        .expect_err("a closed limiter fails the Paladin");
        assert_eq!(failure.node_error.node_id.as_str(), "Limited");
        assert!(failure.error.to_string().contains("limiter closed"));
    }

    #[tokio::test(start_paused = true)]
    async fn phalanx_concurrency_limit_holds_a_permit_across_retries() {
        // With one permit, A's whole attempt sequence (including its backoff)
        // finishes before B starts.
        let phalanx = phalanx_with(&["A", "B"], retry_aegis(2, Duration::from_secs(1)))
            .with_max_concurrency(1);
        let port = RecordingPort::scripted(vec![
            (
                "A",
                vec![failing(transient()), slow(Duration::from_millis(100))],
            ),
            ("B", vec![slow(Duration::from_millis(100))]),
        ]);
        let service = PhalanxExecutionService::new(port.clone());

        let result = service.execute(&phalanx, "go").await.expect("both succeed");
        assert_eq!(result.paladin_results.len(), 2);
        assert_eq!(
            port.events(),
            vec!["start A", "end A", "start A", "end A", "start B", "end B"]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn phalanx_first_success_reports_all_failures() {
        let phalanx = phalanx_with(&["A", "B", "C"], Aegis::default())
            .with_aggregation(AggregationStrategy::FirstSuccess);
        let port = RecordingPort::scripted(vec![
            ("A", vec![failing(execution_error("a down"))]),
            ("B", vec![failing(execution_error("b down"))]),
            ("C", vec![failing(execution_error("c down"))]),
        ]);
        let service = PhalanxExecutionService::new(port.clone());

        match service.execute(&phalanx, "go").await {
            Err(BattalionError::ExecutionError(message)) => {
                assert!(
                    message.starts_with("All Paladins failed"),
                    "got {message:?}"
                );
            }
            other => panic!("expected ExecutionError, got {other:?}"),
        }
        assert_eq!(port.total_calls(), 3, "every Paladin was tried");

        // One success among failures wins.
        let phalanx = phalanx_with(&["A", "B"], Aegis::default())
            .with_aggregation(AggregationStrategy::FirstSuccess);
        let port = RecordingPort::scripted(vec![("A", vec![failing(execution_error("a down"))])]);
        let service = PhalanxExecutionService::new(port);
        let result = service.execute(&phalanx, "go").await.expect("B succeeds");
        assert_eq!(result.paladin_results.len(), 1);
        assert_eq!(result.paladin_results[0].output, "B done");
        assert!(result.per_paladin_times.contains_key("B"));
    }

    #[tokio::test(start_paused = true)]
    async fn phalanx_cancellation_during_backoff_stops_retrying() {
        let phalanx = phalanx_with(&["A", "B"], retry_aegis(5, Duration::from_secs(10)));
        let port = RecordingPort::scripted(vec![(
            "A",
            vec![
                failing(transient()),
                failing(transient()),
                failing(transient()),
            ],
        )]);
        let service = PhalanxExecutionService::new(port.clone());
        let token = CancellationToken::new();
        let canceller = token.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            canceller.cancel();
        });

        let result = service
            .execute_with_cancellation(&phalanx, "go", token)
            .await;
        assert!(
            matches!(result, Err(BattalionError::Cancelled)),
            "got {result:?}"
        );

        // Well past every remaining backoff: the Paladin must not retry.
        tokio::time::sleep(Duration::from_secs(120)).await;
        assert_eq!(port.calls_for("A"), 1, "no retry after cancellation");
    }

    #[tokio::test]
    async fn phalanx_rejects_invalid_aegis_before_any_paladin_runs() {
        // T-44-11: a handler a Phalanx cannot honour is a ValidationError at
        // entry, on both entry points, before any Paladin runs.
        let aegis = Aegis {
            on_error: Some(ErrorHandlerSpec::Custom("not-supported".to_string())),
            ..Aegis::default()
        };
        let phalanx = phalanx_with(&["A", "B"], aegis);
        let port = RecordingPort::scripted(vec![]);
        let service = PhalanxExecutionService::new(port.clone());

        let error = service
            .execute(&phalanx, "go")
            .await
            .expect_err("an unsupported handler is rejected");
        assert!(
            matches!(error, BattalionError::ValidationError(_)),
            "{error:?}"
        );
        let error = service
            .execute_with_cancellation(&phalanx, "go", CancellationToken::new())
            .await
            .expect_err("an unsupported handler is rejected");
        assert!(
            matches!(error, BattalionError::ValidationError(_)),
            "{error:?}"
        );
        assert_eq!(port.total_calls(), 0, "no Paladin ran");
    }

    #[tokio::test(start_paused = true)]
    async fn phalanx_timeout_is_typed_as_run_timeout() {
        // The per-attempt timeout is recorded as NodeErrorSource::Timeout(Run),
        // carried end to end through the runner into the aggregated message.
        let phalanx = phalanx_with(&["A", "B"], timeout_aegis(Duration::from_millis(250)));
        let port = RecordingPort::scripted(vec![("A", vec![slow(Duration::from_secs(1))])]);
        let service = PhalanxExecutionService::new(port.clone());
        let (successes, failures) = service
            .execute_collect_all(&phalanx, "go", &None)
            .await
            .expect("collection never fails outright");
        assert_eq!(successes.len(), 1, "B is inside the bound");
        assert_eq!(failures.len(), 1);
        assert_eq!(
            failures[0].node_error.source,
            NodeErrorSource::Timeout(TimeoutKind::Run)
        );
        assert_eq!(failures[0].node_error.transience, Transience::Transient);
    }
}
