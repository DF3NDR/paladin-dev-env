//! Formation Execution Service
//!
//! Provides orchestration logic for executing Paladins in sequential Formation pattern.

use chrono::Utc;
use log::{debug, info, warn};
use std::sync::Arc;
use uuid::Uuid;

use crate::aegis_attempt;
use crate::error_aggregation::AggregatedError;
use paladin_core::platform::container::aegis::ErrorHandlerSpec;
use paladin_core::platform::container::battalion::formation::Formation;
use paladin_core::platform::container::battalion::{
    BattalionError, BattalionResult, BattalionStatus, BattalionStrategy, NodeError, TokenUsage,
};
use paladin_core::platform::container::herald::Herald;
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult};
use std::collections::HashMap;

/// Service for executing Formation patterns
///
/// Orchestrates sequential Paladin execution where output from one Paladin
/// flows to the next. The timeout, retry and failure-handling policy comes from
/// the Formation's `BattalionConfig.aegis`; every Paladin attempt runs through
/// the shared per-attempt runner (`aegis_attempt::run_with_aegis`).
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
    /// Supports shared context injection. The Formation has no whole-run
    /// timeout: each Paladin attempt is bounded by `aegis.timeout` (D-02),
    /// retried under `aegis.retry`, and a failure that survives its retries
    /// follows `aegis.on_error`:
    ///
    /// * `None` -- fail fast: the first failed Paladin ends the run. A timed-out
    ///   attempt returns the structured [`BattalionError::Node`]; any other
    ///   failure returns [`BattalionError::PaladinError`] with the error text.
    /// * `Some(ErrorHandlerSpec::Absorb { .. })` -- the failure is recorded in
    ///   `node_errors`, counted in `paladin_failure_count`, the next Paladin
    ///   receives an empty input and the run still reports `Completed`. The
    ///   `fallback_delta` is a documented no-op for a string pipeline.
    ///
    /// # Arguments
    ///
    /// * `formation` - The Formation to execute
    /// * `initial_input` - Initial input for the first Paladin
    ///
    /// # Returns
    ///
    /// * `Ok(BattalionResult)` - Final result with all Paladin outputs
    /// * `Err(BattalionError)` - If the Aegis policy is unsupported
    ///   ([`BattalionError::ValidationError`], before any Paladin runs) or a
    ///   Paladin fails under the fail-fast policy
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
        // Reject Aegis policies Formation cannot honour before any Paladin runs.
        formation.config.validate_aegis()?;

        let battalion_id = Uuid::new_v4();
        let _started_at = Utc::now();

        info!(
            "Starting Formation execution: {} (ID: {}) with {} Paladins",
            formation.config.name,
            battalion_id,
            formation.paladins.len()
        );

        let result = self
            .execute_internal(formation, initial_input, battalion_id)
            .await?;
        info!("Formation {} completed successfully", battalion_id);
        Ok(result)
    }

    /// Run every Paladin in order; see [`Self::execute`] for the policy.
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

            match aegis_attempt::run_with_aegis(
                &self.paladin_port,
                paladin,
                &current_input,
                &formation.config.aegis,
                &None,
            )
            .await
            {
                Ok(outcome) => {
                    let result = outcome.result;
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
                Err(failure) => match &formation.config.aegis.on_error {
                    None => {
                        warn!(
                            "Fail fast: Formation failed at Paladin {} ({}) on attempt {}",
                            index + 1,
                            paladin.node.name,
                            failure.node_error.attempt
                        );
                        return Err(failure.into_fail_fast_error());
                    }
                    Some(ErrorHandlerSpec::Absorb { .. }) => {
                        warn!(
                            "Absorb: Paladin {} ({}) failed on attempt {}, continuing with empty \
                             input",
                            index + 1,
                            paladin.node.name,
                            failure.node_error.attempt
                        );
                        // 44-06 replaces this v0.10 summary shape with the
                        // structured `failure.node_error`.
                        node_errors.push(NodeError {
                            node_name: paladin.node.name.clone(),
                            error: failure.error.to_string(),
                        });
                        aggregated_error.add_error(BattalionError::Node(failure.node_error));
                        // The Absorb fallback_delta is a no-op for a string
                        // pipeline: the next Paladin gets an empty input.
                        current_input = String::new();
                    }
                    // `ErrorHandlerSpec` is `#[non_exhaustive]`; `validate_aegis`
                    // already rejects `Route` and `Custom`, so this is reached
                    // only by a future variant. Fail closed, never continue.
                    Some(other) => {
                        return Err(BattalionError::ValidationError(format!(
                            "Formation does not support the aegis.on_error handler {other:?}"
                        )));
                    }
                },
            }
        }

        if aggregated_error.has_errors() {
            warn!(
                "Formation completed with errors: {}",
                aggregated_error.summary()
            );
        }

        // Success/failure counts mirror PhalanxExecutionService::execute_internal:
        // every entry that made it into paladin_results succeeded, and every
        // recorded node_errors entry is one Paladin that failed outright under
        // the Absorb handler.
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
    use paladin_core::platform::container::paladin::{
        MaxLoops, Paladin, PaladinData, PaladinStatus,
    };
    use paladin_core::platform::container::paladin_error::PaladinError;
    use paladin_core::platform::container::transience::Transience;
    use paladin_ports::output::paladin_port::{PaladinResult, StopReason};
    use std::collections::VecDeque;
    use std::sync::Mutex;
    use std::time::Duration;

    /// One scripted call for a named Paladin: wait `delay`, then answer.
    struct Step {
        delay: Duration,
        fail: Option<PaladinError>,
    }

    fn slow(delay: Duration) -> Step {
        Step { delay, fail: None }
    }

    fn failing(error: PaladinError) -> Step {
        Step {
            delay: Duration::ZERO,
            fail: Some(error),
        }
    }

    fn execution_error(message: &str) -> PaladinError {
        PaladinError::ExecutionError(message.to_string())
    }

    /// A port that plays back a per-Paladin script (an exhausted or absent
    /// script answers with an immediate success) and records every
    /// `(paladin name, input)` it receives. Sleeps with `tokio::time::sleep`
    /// so a paused clock drives the timing.
    struct RecordingPort {
        scripts: Mutex<HashMap<String, VecDeque<Step>>>,
        calls: Mutex<Vec<(String, String)>>,
    }

    impl RecordingPort {
        fn new() -> Arc<Self> {
            Self::scripted(Vec::new())
        }

        fn scripted(scripts: Vec<(&str, Vec<Step>)>) -> Arc<Self> {
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

        fn get_call_count(&self) -> usize {
            self.calls.lock().map(|c| c.len()).unwrap_or_default()
        }

        fn calls_for(&self, name: &str) -> usize {
            self.calls
                .lock()
                .map(|c| c.iter().filter(|(n, _)| n == name).count())
                .unwrap_or_default()
        }

        fn inputs_for(&self, name: &str) -> Vec<String> {
            self.calls
                .lock()
                .map(|c| {
                    c.iter()
                        .filter(|(n, _)| n == name)
                        .map(|(_, input)| input.clone())
                        .collect()
                })
                .unwrap_or_default()
        }
    }

    #[async_trait]
    impl PaladinPort for RecordingPort {
        async fn execute(
            &self,
            paladin: &Paladin,
            input: &str,
        ) -> Result<PaladinResult, PaladinError> {
            let name = paladin.node.name.clone();
            if let Ok(mut calls) = self.calls.lock() {
                calls.push((name.clone(), input.to_string()));
            }
            let step = self
                .scripts
                .lock()
                .ok()
                .and_then(|mut s| s.get_mut(&name).and_then(|q| q.pop_front()));
            if let Some(step) = step {
                tokio::time::sleep(step.delay).await;
                if let Some(error) = step.fail {
                    return Err(error);
                }
            }

            Ok(PaladinResult {
                output: format!("Processed: {} by {}", input, name),
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

    fn formation_with(names: &[&str], aegis: Aegis) -> Formation {
        let paladins = names.iter().map(|n| create_test_paladin(n)).collect();
        Formation::new(
            paladins,
            BattalionConfig::new("test_formation").with_aegis(aegis),
        )
        .expect("a non-empty Formation is valid")
    }

    fn run_timeout(duration: Duration) -> Option<TimeoutPolicy> {
        Some(TimeoutPolicy {
            run_timeout: Some(duration),
            idle_timeout: None,
        })
    }

    fn absorb() -> Option<ErrorHandlerSpec> {
        Some(ErrorHandlerSpec::Absorb {
            fallback_delta: StateDelta::new(),
        })
    }

    #[tokio::test]
    async fn test_formation_service_creation() {
        let mock_port = RecordingPort::new();
        let _service = FormationExecutionService::new(mock_port);
        // Service created successfully
        // Test passes if we reach here without panicking
    }

    #[tokio::test]
    async fn test_sequential_execution_success() {
        let mock_port = RecordingPort::new();
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
        let mock_port = RecordingPort::new();
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
    async fn test_formation_aggregates_per_paladin_times_and_tokens() {
        let mock_port = RecordingPort::new();
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
    async fn test_shared_context_injection() {
        let mock_port = RecordingPort::new();
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

    #[tokio::test(start_paused = true)]
    async fn formation_bounds_each_attempt_not_the_run() {
        // D-02: three 600 ms Paladins under a 1 s per-attempt bound complete
        // (1.8 s in total); there is no whole-run wall clock.
        let aegis = Aegis {
            timeout: run_timeout(Duration::from_secs(1)),
            ..Aegis::default()
        };
        let formation = formation_with(&["P1", "P2", "P3"], aegis.clone());
        let delay = Duration::from_millis(600);
        let port = RecordingPort::scripted(vec![
            ("P1", vec![slow(delay)]),
            ("P2", vec![slow(delay)]),
            ("P3", vec![slow(delay)]),
        ]);
        let service = FormationExecutionService::new(port.clone());

        let started = tokio::time::Instant::now();
        let result = service
            .execute(&formation, "go")
            .await
            .expect("each 600 ms Paladin is inside its 1 s bound");
        assert_eq!(result.status, BattalionStatus::Completed);
        assert_eq!(result.paladin_results.len(), 3);
        assert!(started.elapsed() >= Duration::from_millis(1800));
        assert_eq!(port.get_call_count(), 3);

        // A 1.5 s Paladin under the same bound fails its attempt at 1 s.
        let formation = formation_with(&["P1"], aegis);
        let port = RecordingPort::scripted(vec![("P1", vec![slow(Duration::from_millis(1500))])]);
        let service = FormationExecutionService::new(port);
        let started = tokio::time::Instant::now();
        service
            .execute(&formation, "go")
            .await
            .expect_err("1.5 s exceeds the 1 s per-attempt bound");
        assert_eq!(started.elapsed(), Duration::from_secs(1));
    }

    #[tokio::test(start_paused = true)]
    async fn formation_timeout_surfaces_structured_node_error() {
        // D-03: with on_error None a timed-out attempt ends the Formation with
        // the structured BattalionError::Node; the third Paladin never runs.
        let aegis = Aegis {
            timeout: run_timeout(Duration::from_secs(1)),
            ..Aegis::default()
        };
        let formation = formation_with(&["P1", "P2", "P3"], aegis);
        let port = RecordingPort::scripted(vec![("P2", vec![slow(Duration::from_millis(1500))])]);
        let service = FormationExecutionService::new(port.clone());

        match service.execute(&formation, "go").await {
            Err(BattalionError::Node(error)) => {
                assert_eq!(error.node_id.as_str(), "P2");
                assert_eq!(error.attempt, 1);
                assert_eq!(error.transience, Transience::Transient);
                assert_eq!(error.source, NodeErrorSource::Timeout(TimeoutKind::Run));
            }
            other => panic!("expected BattalionError::Node, got {other:?}"),
        }
        assert_eq!(port.calls_for("P3"), 0, "the third Paladin never runs");
    }

    #[tokio::test(start_paused = true)]
    async fn formation_fail_fast_keeps_the_paladin_error_contract() {
        // Research Open Question 3: a non-timeout failure keeps the v0.10
        // BattalionError::PaladinError(<message>) contract.
        let formation = formation_with(&["P1", "P2", "P3"], Aegis::default());
        let port = RecordingPort::scripted(vec![(
            "P2",
            vec![failing(execution_error("Mock Paladin execution failed"))],
        )]);
        let service = FormationExecutionService::new(port.clone());

        match service.execute(&formation, "go").await {
            Err(BattalionError::PaladinError(message)) => {
                assert!(
                    message.contains("Mock Paladin execution failed"),
                    "the error text is preserved, got {message:?}"
                );
            }
            other => panic!("expected BattalionError::PaladinError, got {other:?}"),
        }
        assert_eq!(port.calls_for("P3"), 0, "the third Paladin never runs");
    }

    #[tokio::test(start_paused = true)]
    async fn formation_absorb_continues_with_empty_input() {
        // D-05 / D-07: Absorb records the failure, hands the next Paladin an
        // empty input and still reports Completed.
        let aegis = Aegis {
            on_error: absorb(),
            ..Aegis::default()
        };
        let formation = formation_with(&["P1", "P2", "P3"], aegis);
        let port =
            RecordingPort::scripted(vec![("P2", vec![failing(execution_error("P2 exploded"))])]);
        let service = FormationExecutionService::new(port.clone());

        let result = service
            .execute(&formation, "Input")
            .await
            .expect("Absorb continues past the failure");

        assert_eq!(result.status, BattalionStatus::Completed);
        assert_eq!(result.paladin_results.len(), 2);
        assert_eq!(result.node_errors.len(), 1);
        assert_eq!(result.node_errors[0].node_name, "P2");
        assert!(!result.node_errors[0].error.is_empty());
        assert_eq!(result.paladin_failure_count, 1);
        assert_eq!(result.paladin_success_count, 2);
        assert_eq!(
            port.inputs_for("P3"),
            vec![String::new()],
            "the Paladin after the absorbed failure receives an empty input"
        );

        // The successful Paladins still appear in the aggregation maps.
        assert_eq!(result.per_paladin_times.len(), 2);
        assert!(result.per_paladin_times.contains_key("P1"));
        assert!(result.per_paladin_times.contains_key("P3"));
        assert_eq!(result.per_paladin_tokens.len(), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn formation_retry_honours_retry_on() {
        // An ExecutionError is Unknown: TransientAndUnknown retries it,
        // TransientOnly does not (research Finding 4 / Pitfall 1).
        let retrying = |retry_on: RetryPredicate| Aegis {
            retry: Some(AegisRetryPolicy {
                max_attempts: 3,
                initial_interval: Duration::from_millis(10),
                jitter: false,
                retry_on,
                ..AegisRetryPolicy::default()
            }),
            ..Aegis::default()
        };
        let always_failing = || {
            vec![(
                "P1",
                vec![
                    failing(execution_error("boom 1")),
                    failing(execution_error("boom 2")),
                    failing(execution_error("boom 3")),
                ],
            )]
        };

        let formation = formation_with(&["P1"], retrying(RetryPredicate::TransientAndUnknown));
        let port = RecordingPort::scripted(always_failing());
        let service = FormationExecutionService::new(port.clone());
        service
            .execute(&formation, "go")
            .await
            .expect_err("all three attempts fail");
        assert_eq!(port.calls_for("P1"), 3);

        let formation = formation_with(&["P1"], retrying(RetryPredicate::TransientOnly));
        let port = RecordingPort::scripted(always_failing());
        let service = FormationExecutionService::new(port.clone());
        service
            .execute(&formation, "go")
            .await
            .expect_err("an Unknown failure is not retried");
        assert_eq!(port.calls_for("P1"), 1);

        // A v0.10 RetryPolicy { max_attempts: 2 } maps to max_attempts 3; the
        // Paladin that fails twice then succeeds keeps its old call count.
        let formation = formation_with(&["P1"], retrying(RetryPredicate::TransientAndUnknown));
        let port = RecordingPort::scripted(vec![(
            "P1",
            vec![
                failing(execution_error("boom 1")),
                failing(execution_error("boom 2")),
            ],
        )]);
        let service = FormationExecutionService::new(port.clone());
        let result = service
            .execute(&formation, "go")
            .await
            .expect("the third attempt succeeds");
        assert_eq!(result.paladin_results.len(), 1);
        assert_eq!(port.calls_for("P1"), 3);
    }

    #[tokio::test]
    async fn formation_rejects_invalid_aegis_before_any_paladin_runs() {
        // T-44-11: a handler Formation cannot honour is a ValidationError at
        // entry, never silently treated as Absorb.
        let aegis = Aegis {
            on_error: Some(ErrorHandlerSpec::Custom("not-supported".to_string())),
            ..Aegis::default()
        };
        let formation = formation_with(&["P1", "P2"], aegis);
        let port = RecordingPort::new();
        let service = FormationExecutionService::new(port.clone());

        let error = service
            .execute(&formation, "go")
            .await
            .expect_err("an unsupported handler is rejected");
        assert!(
            matches!(error, BattalionError::ValidationError(_)),
            "expected ValidationError, got {error:?}"
        );
        assert_eq!(port.get_call_count(), 0, "no Paladin ran");
    }
}
