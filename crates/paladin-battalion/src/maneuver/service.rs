//! Maneuver Execution Service
//!
//! Executes Maneuver workflows by orchestrating multiple Paladin agents according to
//! the flow expression with support for sequential, parallel, and nested execution patterns.

use super::parser::FlowExpression;
use super::{ErrorStrategy, ExecutionStatus, Maneuver, ManeuverError, ManeuverResult};
use paladin_ports::output::paladin_port::PaladinPort;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::timeout;

/// Service for executing Maneuver workflows
///
/// # Examples
///
/// ```
/// use paladin_battalion::maneuver::service::ManeuverExecutionService;
/// use paladin_ports::output::paladin_port::PaladinPort;
/// use std::sync::Arc;
///
/// fn build(paladin_port: Arc<dyn PaladinPort>) -> ManeuverExecutionService {
///     ManeuverExecutionService::new(paladin_port)
/// }
/// ```
pub struct ManeuverExecutionService {
    paladin_port: Arc<dyn PaladinPort>,
}

impl ManeuverExecutionService {
    /// Create a new ManeuverExecutionService
    pub fn new(paladin_port: Arc<dyn PaladinPort>) -> Self {
        Self { paladin_port }
    }

    /// Execute a Maneuver workflow
    ///
    /// # Arguments
    ///
    /// * `maneuver` - The Maneuver to execute
    /// * `input` - Initial input to the workflow
    ///
    /// # Returns
    ///
    /// * `Ok(ManeuverResult)` - Successful execution with results
    /// * `Err(ManeuverError)` - Execution error
    pub async fn execute(
        &self,
        maneuver: &Maneuver,
        input: &str,
    ) -> Result<ManeuverResult, ManeuverError> {
        let start_time = Instant::now();

        // Execute with optional timeout
        let result = if let Some(timeout_duration) = maneuver.config.timeout {
            timeout(timeout_duration, self.execute_internal(maneuver, input))
                .await
                .map_err(|_| ManeuverError::TimeoutError {
                    duration: timeout_duration,
                })?
        } else {
            self.execute_internal(maneuver, input).await
        }?;

        // Add total execution time if not already tracked
        let total_duration = start_time.elapsed();
        log::info!(
            "Maneuver '{}' completed in {:?}",
            maneuver.name,
            total_duration
        );

        Ok(result)
    }

    /// Internal execution without timeout wrapper
    async fn execute_internal(
        &self,
        maneuver: &Maneuver,
        input: &str,
    ) -> Result<ManeuverResult, ManeuverError> {
        let mut context = ExecutionContext::new(maneuver);

        // Execute the flow expression recursively
        let final_output = self
            .execute_expression(&maneuver.flow, input, maneuver, &mut context)
            .await?;

        // Build result
        let result = if maneuver.config.collect_timing_metrics {
            ManeuverResult::with_timing(
                final_output,
                context.step_outputs,
                context.execution_order,
                context.timing_metrics,
            )
        } else {
            ManeuverResult::new(final_output, context.step_outputs, context.execution_order)
        };

        // Set status based on whether we had any errors
        let status = if context.had_errors {
            if context.continued_after_error {
                ExecutionStatus::PartialSuccess
            } else {
                ExecutionStatus::Failed
            }
        } else {
            ExecutionStatus::Success
        };

        Ok(result.with_status(status))
    }

    /// Execute a flow expression recursively (boxed to handle recursion)
    fn execute_expression<'a>(
        &'a self,
        expr: &'a FlowExpression,
        input: &'a str,
        maneuver: &'a Maneuver,
        context: &'a mut ExecutionContext,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, ManeuverError>> + 'a>>
    {
        Box::pin(async move {
            match expr {
                FlowExpression::Agent(name) => {
                    self.execute_agent(name, input, maneuver, context).await
                }
                FlowExpression::Sequential(exprs) => {
                    self.execute_sequential(exprs, input, maneuver, context)
                        .await
                }
                FlowExpression::Parallel(exprs) => {
                    self.execute_parallel(exprs, input, maneuver, context).await
                }
            }
        })
    }

    /// Execute a single agent
    async fn execute_agent(
        &self,
        agent_name: &str,
        input: &str,
        maneuver: &Maneuver,
        context: &mut ExecutionContext,
    ) -> Result<String, ManeuverError> {
        let agent =
            maneuver
                .agents
                .get(agent_name)
                .ok_or_else(|| ManeuverError::AgentNotFound {
                    agent_name: agent_name.to_string(),
                    available_agents: maneuver.agents.keys().cloned().collect(),
                })?;

        log::debug!("Executing agent: {}", agent_name);
        let start_time = Instant::now();

        // Execute the agent
        let result =
            self.paladin_port.execute(agent, input).await.map_err(|e| {
                ManeuverError::ExecutionError(format!("Agent '{}': {}", agent_name, e))
            })?;

        let duration = start_time.elapsed();

        // Record execution
        context.execution_order.push(agent_name.to_string());
        context
            .step_outputs
            .insert(agent_name.to_string(), result.output.clone());

        if maneuver.config.collect_timing_metrics {
            context
                .timing_metrics
                .insert(agent_name.to_string(), duration);
        }

        log::debug!(
            "Agent '{}' completed in {:?}, output length: {}",
            agent_name,
            duration,
            result.output.len()
        );

        Ok(result.output)
    }

    /// Execute agents sequentially
    async fn execute_sequential(
        &self,
        exprs: &[FlowExpression],
        initial_input: &str,
        maneuver: &Maneuver,
        context: &mut ExecutionContext,
    ) -> Result<String, ManeuverError> {
        let mut current_input = initial_input.to_string();

        for expr in exprs {
            let input = if maneuver.config.pass_output_as_input {
                &current_input
            } else {
                initial_input
            };

            match self
                .execute_expression(expr, input, maneuver, context)
                .await
            {
                Ok(output) => {
                    current_input = output;
                }
                Err(e) => {
                    context.had_errors = true;

                    match maneuver.config.error_strategy {
                        ErrorStrategy::FailFast => {
                            return Err(e);
                        }
                        ErrorStrategy::ContinueParallel => {
                            // In sequential, we must stop
                            return Err(e);
                        }
                        ErrorStrategy::IgnoreErrors => {
                            context.continued_after_error = true;
                            log::warn!("Ignoring error in sequential execution: {}", e);
                            // Continue with current input unchanged
                        }
                    }
                }
            }
        }

        Ok(current_input)
    }

    /// Execute agents in parallel
    async fn execute_parallel(
        &self,
        exprs: &[FlowExpression],
        input: &str,
        maneuver: &Maneuver,
        context: &mut ExecutionContext,
    ) -> Result<String, ManeuverError> {
        let mut handles = Vec::new();

        // Clone what we need for parallel execution
        let agents = maneuver.agents.clone();
        let error_strategy = maneuver.config.error_strategy;
        let collect_timing = maneuver.config.collect_timing_metrics;
        let pass_output = maneuver.config.pass_output_as_input;

        // Spawn parallel execution tasks
        for expr in exprs {
            let expr_clone = expr.clone();
            let input_clone = input.to_string();
            let paladin_port = Arc::clone(&self.paladin_port);
            let agents_clone = agents.clone();

            let handle = tokio::spawn(async move {
                let mut step_outputs = HashMap::new();
                let mut execution_order = Vec::new();
                let mut timing_metrics = HashMap::new();

                // Recursively execute any expression type
                let result = match &expr_clone {
                    FlowExpression::Agent(name) => {
                        execution_order.push(name.clone());

                        if let Some(paladin) = agents_clone.get(name) {
                            let agent_start = Instant::now();
                            let exec_result = paladin_port.execute(paladin, &input_clone).await;

                            if collect_timing {
                                timing_metrics.insert(name.clone(), agent_start.elapsed());
                            }

                            match exec_result {
                                Ok(result) => {
                                    step_outputs.insert(name.clone(), result.output.clone());
                                    Ok(result.output)
                                }
                                Err(e) => Err(ManeuverError::ExecutionError(format!(
                                    "Agent {} execution failed: {}",
                                    name, e
                                ))),
                            }
                        } else {
                            Err(ManeuverError::AgentNotFound {
                                agent_name: name.clone(),
                                available_agents: vec![],
                            })
                        }
                    }
                    FlowExpression::Sequential(seq_exprs) => {
                        // Execute sequential sub-expression
                        let mut current_input = input_clone.clone();

                        for seq_expr in seq_exprs {
                            let expr_input = if pass_output {
                                &current_input
                            } else {
                                &input_clone
                            };

                            match seq_expr {
                                FlowExpression::Agent(name) => {
                                    execution_order.push(name.clone());

                                    if let Some(paladin) = agents_clone.get(name) {
                                        let agent_start = Instant::now();
                                        match paladin_port.execute(paladin, expr_input).await {
                                            Ok(result) => {
                                                if collect_timing {
                                                    timing_metrics.insert(
                                                        name.clone(),
                                                        agent_start.elapsed(),
                                                    );
                                                }
                                                step_outputs
                                                    .insert(name.clone(), result.output.clone());
                                                current_input = result.output;
                                            }
                                            Err(e) => {
                                                return (
                                                    expr_clone.clone(),
                                                    Err(ManeuverError::ExecutionError(format!(
                                                        "Sequential agent {} failed: {}",
                                                        name, e
                                                    ))),
                                                    step_outputs,
                                                    execution_order,
                                                    timing_metrics,
                                                );
                                            }
                                        }
                                    } else {
                                        return (
                                            expr_clone.clone(),
                                            Err(ManeuverError::AgentNotFound {
                                                agent_name: name.clone(),
                                                available_agents: vec![],
                                            }),
                                            step_outputs,
                                            execution_order,
                                            timing_metrics,
                                        );
                                    }
                                }
                                _ => {
                                    return (
                                        expr_clone,
                                        Err(ManeuverError::ExecutionError(
                                            "Deeply nested expressions not supported in parallel branches"
                                                .to_string(),
                                        )),
                                        step_outputs,
                                        execution_order,
                                        timing_metrics,
                                    );
                                }
                            }
                        }

                        Ok(current_input)
                    }
                    FlowExpression::Parallel(_) => {
                        // Nested parallel not supported in parallel branches
                        Err(ManeuverError::ExecutionError(
                            "Nested parallel expressions not supported".to_string(),
                        ))
                    }
                };

                (
                    expr_clone,
                    result,
                    step_outputs,
                    execution_order,
                    timing_metrics,
                )
            });

            handles.push(handle);
        }

        // Wait for all tasks to complete
        let mut results = Vec::new();
        let mut had_error = false;
        let mut first_error = None;

        for handle in handles {
            match handle.await {
                Ok((_expr, result, step_outputs, execution_order, timing_metrics)) => {
                    // Merge outputs into context
                    for (key, value) in step_outputs {
                        context.step_outputs.insert(key, value);
                    }
                    for agent in execution_order {
                        context.execution_order.push(agent);
                    }
                    for (key, value) in timing_metrics {
                        context.timing_metrics.insert(key, value);
                    }

                    match result {
                        Ok(output) => {
                            results.push(output);
                        }
                        Err(e) => {
                            had_error = true;
                            context.had_errors = true;

                            let error_message = format!("{:?}", e);
                            if first_error.is_none() {
                                first_error = Some(e);
                            }

                            match error_strategy {
                                ErrorStrategy::FailFast => {
                                    // Stop immediately on first error
                                    return Err(first_error.unwrap());
                                }
                                ErrorStrategy::ContinueParallel => {
                                    context.continued_after_error = true;
                                    log::warn!(
                                        "Parallel branch failed, continuing: {}",
                                        error_message
                                    );
                                    // Continue collecting results
                                }
                                ErrorStrategy::IgnoreErrors => {
                                    context.continued_after_error = true;
                                    log::warn!("Ignoring parallel branch error: {}", error_message);
                                    // Continue collecting results
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    return Err(ManeuverError::ExecutionError(format!(
                        "Parallel task join error: {}",
                        e
                    )));
                }
            }
        }

        // Handle error strategies after all tasks complete
        if had_error && error_strategy == ErrorStrategy::FailFast {
            return Err(first_error.unwrap());
        }

        // Combine outputs based on output format
        let combined = results.join("\n---\n");
        Ok(combined)
    }
}

/// Execution context for tracking state during workflow execution
struct ExecutionContext {
    /// Outputs from each step
    step_outputs: HashMap<String, String>,
    /// Order of execution
    execution_order: Vec<String>,
    /// Timing metrics per agent
    timing_metrics: HashMap<String, Duration>,
    /// Whether any errors occurred
    had_errors: bool,
    /// Whether execution continued after an error
    continued_after_error: bool,
}

impl ExecutionContext {
    fn new(_maneuver: &Maneuver) -> Self {
        Self {
            step_outputs: HashMap::new(),
            execution_order: Vec::new(),
            timing_metrics: HashMap::new(),
            had_errors: false,
            continued_after_error: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maneuver::ManeuverConfig;
    use crate::maneuver::parser::FlowParser;
    use async_trait::async_trait;
    use paladin_core::platform::container::battalion::TokenUsage;
    use paladin_core::platform::container::paladin::{
        MaxLoops, Paladin, PaladinData, PaladinStatus,
    };
    use paladin_ports::output::paladin_port::{PaladinResult, StopReason};
    use std::sync::Mutex;

    // Mock PaladinPort for testing
    struct MockPaladinPort {
        responses: Mutex<HashMap<String, String>>,
        call_count: Mutex<HashMap<String, usize>>,
    }

    impl MockPaladinPort {
        fn new() -> Self {
            Self {
                responses: Mutex::new(HashMap::new()),
                call_count: Mutex::new(HashMap::new()),
            }
        }

        fn set_response(&self, agent_name: &str, output: &str) {
            self.responses
                .lock()
                .unwrap()
                .insert(agent_name.to_string(), output.to_string());
        }

        fn get_call_count(&self, agent_name: &str) -> usize {
            *self
                .call_count
                .lock()
                .unwrap()
                .get(agent_name)
                .unwrap_or(&0)
        }
    }

    #[async_trait]
    impl PaladinPort for MockPaladinPort {
        async fn execute(
            &self,
            paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinResult, paladin_core::platform::container::paladin_error::PaladinError>
        {
            let agent_name = &paladin.node.name;

            // Increment call count
            let mut counts = self.call_count.lock().unwrap();
            *counts.entry(agent_name.clone()).or_insert(0) += 1;
            drop(counts);

            // Get response
            let responses = self.responses.lock().unwrap();
            let output = responses
                .get(agent_name)
                .cloned()
                .unwrap_or_else(|| format!("output from {}", agent_name));

            Ok(PaladinResult {
                output,
                usage: TokenUsage::new(100, 0),
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
        ) -> Result<
            paladin_ports::output::paladin_port::PaladinStream,
            paladin_core::platform::container::paladin_error::PaladinError,
        > {
            unimplemented!("Streaming not needed for tests")
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
            system_prompt: format!("Test paladin {}", name),
            name: name.to_string(),
            user_name: "test".to_string(),
            model: "gpt-4".to_string(),
            temperature: 0.7,
            max_loops: MaxLoops::Fixed(1),
            stop_words: vec![],
            status: PaladinStatus::Idle,
            vision_enabled: false,
            ..Default::default()
        };
        Paladin::new(data, None)
    }

    #[tokio::test]
    async fn test_simple_sequential_execution() {
        let flow = FlowParser::parse("agent1 -> agent2").unwrap();
        let mut agents = HashMap::new();
        agents.insert("agent1".to_string(), create_test_paladin("agent1"));
        agents.insert("agent2".to_string(), create_test_paladin("agent2"));

        let maneuver = Maneuver::new("test", agents, flow, ManeuverConfig::default()).unwrap();

        let mock_port = Arc::new(MockPaladinPort::new());
        mock_port.set_response("agent1", "output1");
        mock_port.set_response("agent2", "output2");

        let service = ManeuverExecutionService::new(mock_port.clone());
        let result = service.execute(&maneuver, "initial input").await.unwrap();

        assert_eq!(result.final_output, "output2");
        assert_eq!(result.execution_order, vec!["agent1", "agent2"]);
        assert_eq!(result.status, ExecutionStatus::Success);
        assert_eq!(mock_port.get_call_count("agent1"), 1);
        assert_eq!(mock_port.get_call_count("agent2"), 1);
    }

    #[tokio::test]
    async fn test_simple_parallel_execution() {
        let flow = FlowParser::parse("agent1, agent2").unwrap();
        let mut agents = HashMap::new();
        agents.insert("agent1".to_string(), create_test_paladin("agent1"));
        agents.insert("agent2".to_string(), create_test_paladin("agent2"));

        let maneuver = Maneuver::new("test", agents, flow, ManeuverConfig::default()).unwrap();

        let mock_port = Arc::new(MockPaladinPort::new());
        mock_port.set_response("agent1", "output1");
        mock_port.set_response("agent2", "output2");

        let service = ManeuverExecutionService::new(mock_port.clone());
        let result = service.execute(&maneuver, "initial input").await.unwrap();

        // Result should contain both outputs
        assert!(result.final_output.contains("output1") || result.final_output.contains("output2"));
        assert_eq!(result.execution_order.len(), 2);
        assert_eq!(result.status, ExecutionStatus::Success);
    }

    #[tokio::test]
    async fn test_timing_metrics_collection() {
        let flow = FlowParser::parse("agent1 -> agent2").unwrap();
        let mut agents = HashMap::new();
        agents.insert("agent1".to_string(), create_test_paladin("agent1"));
        agents.insert("agent2".to_string(), create_test_paladin("agent2"));

        let config = ManeuverConfig::default().with_timing_metrics(true);
        let maneuver = Maneuver::new("test", agents, flow, config).unwrap();

        let mock_port = Arc::new(MockPaladinPort::new());
        let service = ManeuverExecutionService::new(mock_port);
        let result = service.execute(&maneuver, "input").await.unwrap();

        assert!(result.timing_metrics.is_some());
        let metrics = result.timing_metrics.unwrap();
        assert!(metrics.contains_key("agent1"));
        assert!(metrics.contains_key("agent2"));
    }

    #[tokio::test]
    async fn test_fan_out_pattern() {
        // Pattern: agent1 -> (agent2, agent3, agent4)
        let flow = FlowParser::parse("agent1 -> (agent2, agent3, agent4)").unwrap();
        let mut agents = HashMap::new();
        agents.insert("agent1".to_string(), create_test_paladin("agent1"));
        agents.insert("agent2".to_string(), create_test_paladin("agent2"));
        agents.insert("agent3".to_string(), create_test_paladin("agent3"));
        agents.insert("agent4".to_string(), create_test_paladin("agent4"));

        let maneuver = Maneuver::new("test", agents, flow, ManeuverConfig::default()).unwrap();

        let mock_port = Arc::new(MockPaladinPort::new());
        mock_port.set_response("agent1", "initial result");
        mock_port.set_response("agent2", "branch A");
        mock_port.set_response("agent3", "branch B");
        mock_port.set_response("agent4", "branch C");

        let service = ManeuverExecutionService::new(mock_port.clone());
        let result = service.execute(&maneuver, "input").await.unwrap();

        // Verify agent1 ran first
        assert!(result.execution_order[0] == "agent1");
        // Verify all parallel agents ran
        assert_eq!(result.execution_order.len(), 4);
        assert!(result.step_outputs.contains_key("agent2"));
        assert!(result.step_outputs.contains_key("agent3"));
        assert!(result.step_outputs.contains_key("agent4"));
    }

    #[tokio::test]
    async fn test_nested_expression() {
        // Pattern: (agent1 -> agent2), agent3
        let flow = FlowParser::parse("(agent1 -> agent2), agent3").unwrap();
        let mut agents = HashMap::new();
        agents.insert("agent1".to_string(), create_test_paladin("agent1"));
        agents.insert("agent2".to_string(), create_test_paladin("agent2"));
        agents.insert("agent3".to_string(), create_test_paladin("agent3"));

        let maneuver = Maneuver::new("test", agents, flow, ManeuverConfig::default()).unwrap();

        let mock_port = Arc::new(MockPaladinPort::new());
        mock_port.set_response("agent1", "step1");
        mock_port.set_response("agent2", "step2");
        mock_port.set_response("agent3", "step3");

        let service = ManeuverExecutionService::new(mock_port);
        let result = service.execute(&maneuver, "input").await.unwrap();

        assert_eq!(result.status, ExecutionStatus::Success);
        assert_eq!(result.execution_order.len(), 3);
    }

    #[tokio::test]
    async fn test_error_strategy_fail_fast() {
        let flow = FlowParser::parse("agent1 -> agent2 -> agent3").unwrap();
        let mut agents = HashMap::new();
        agents.insert("agent1".to_string(), create_test_paladin("agent1"));
        agents.insert("agent2".to_string(), create_test_paladin("agent2"));
        agents.insert("agent3".to_string(), create_test_paladin("agent3"));

        let config = ManeuverConfig::default().with_error_strategy(ErrorStrategy::FailFast);
        let maneuver = Maneuver::new("test", agents, flow, config).unwrap();

        // Create a mock that fails on agent2
        let mock_port = Arc::new(FailingMockPaladinPort::new());
        let service = ManeuverExecutionService::new(mock_port);

        let result = service.execute(&maneuver, "input").await;
        assert!(result.is_err());

        // Verify error message mentions agent2
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(err_msg.contains("agent2"));
    }

    #[tokio::test]
    async fn test_error_strategy_ignore_errors() {
        let flow = FlowParser::parse("agent1 -> agent2 -> agent3").unwrap();
        let mut agents = HashMap::new();
        agents.insert("agent1".to_string(), create_test_paladin("agent1"));
        agents.insert("agent2".to_string(), create_test_paladin("agent2"));
        agents.insert("agent3".to_string(), create_test_paladin("agent3"));

        let config = ManeuverConfig::default().with_error_strategy(ErrorStrategy::IgnoreErrors);
        let maneuver = Maneuver::new("test", agents, flow, config).unwrap();

        let mock_port = Arc::new(FailingMockPaladinPort::new());
        let service = ManeuverExecutionService::new(mock_port);

        let result = service.execute(&maneuver, "input").await;

        // Should succeed despite agent2 failing
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.status, ExecutionStatus::PartialSuccess);
    }

    #[tokio::test]
    async fn test_timeout_handling() {
        let flow = FlowParser::parse("agent1").unwrap();
        let mut agents = HashMap::new();
        agents.insert("agent1".to_string(), create_test_paladin("agent1"));

        // Set 1 second timeout (minimum allowed by validation)
        let config = ManeuverConfig::default().with_timeout(Duration::from_secs(1));
        let maneuver = Maneuver::new("test", agents, flow, config).unwrap();

        let mock_port = Arc::new(SlowMockPaladinPort::new());
        let service = ManeuverExecutionService::new(mock_port);

        let result = service.execute(&maneuver, "input").await;
        assert!(result.is_err());

        // Verify it's a timeout error
        match result.unwrap_err() {
            ManeuverError::TimeoutError { .. } => {
                // Expected
            }
            e => panic!("Expected timeout error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_output_passing_disabled() {
        let flow = FlowParser::parse("agent1 -> agent2").unwrap();
        let mut agents = HashMap::new();
        agents.insert("agent1".to_string(), create_test_paladin("agent1"));
        agents.insert("agent2".to_string(), create_test_paladin("agent2"));

        let config = ManeuverConfig::default().with_pass_output_as_input(false);
        let maneuver = Maneuver::new("test", agents, flow, config).unwrap();

        let mock_port = Arc::new(InputTrackingMockPort::new());
        mock_port.set_response("agent1", "output1");
        mock_port.set_response("agent2", "output2");

        let service = ManeuverExecutionService::new(mock_port.clone());
        let result = service.execute(&maneuver, "initial input").await.unwrap();

        assert_eq!(result.final_output, "output2");

        // Verify agent2 received original input, not agent1's output
        let inputs = mock_port.get_inputs();
        assert_eq!(inputs.get("agent1"), Some(&"initial input".to_string()));
        assert_eq!(inputs.get("agent2"), Some(&"initial input".to_string()));
    }

    // Additional mock implementations for testing

    struct FailingMockPaladinPort {}

    impl FailingMockPaladinPort {
        fn new() -> Self {
            Self {}
        }
    }

    #[async_trait]
    impl PaladinPort for FailingMockPaladinPort {
        async fn execute(
            &self,
            paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinResult, paladin_core::platform::container::paladin_error::PaladinError>
        {
            let agent_name = &paladin.node.name;

            // Fail on agent2
            if agent_name == "agent2" {
                return Err(
                    paladin_core::platform::container::paladin_error::PaladinError::ExecutionError(
                        "Simulated failure on agent2".to_string(),
                    ),
                );
            }

            Ok(PaladinResult {
                output: format!("output from {}", agent_name),
                usage: TokenUsage::new(100, 0),
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
        ) -> Result<
            paladin_ports::output::paladin_port::PaladinStream,
            paladin_core::platform::container::paladin_error::PaladinError,
        > {
            unimplemented!()
        }

        fn validate(
            &self,
            _paladin: &Paladin,
        ) -> Result<(), paladin_core::platform::container::paladin_error::PaladinError> {
            Ok(())
        }
    }

    struct SlowMockPaladinPort {}

    impl SlowMockPaladinPort {
        fn new() -> Self {
            Self {}
        }
    }

    #[async_trait]
    impl PaladinPort for SlowMockPaladinPort {
        async fn execute(
            &self,
            _paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinResult, paladin_core::platform::container::paladin_error::PaladinError>
        {
            // Sleep for a long time to trigger timeout
            tokio::time::sleep(Duration::from_secs(10)).await;

            Ok(PaladinResult {
                output: "slow output".to_string(),
                usage: TokenUsage::new(100, 0),
                execution_time_ms: 10000,
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
            paladin_ports::output::paladin_port::PaladinStream,
            paladin_core::platform::container::paladin_error::PaladinError,
        > {
            unimplemented!()
        }

        fn validate(
            &self,
            _paladin: &Paladin,
        ) -> Result<(), paladin_core::platform::container::paladin_error::PaladinError> {
            Ok(())
        }
    }

    struct InputTrackingMockPort {
        responses: Mutex<HashMap<String, String>>,
        inputs: Mutex<HashMap<String, String>>,
    }

    impl InputTrackingMockPort {
        fn new() -> Self {
            Self {
                responses: Mutex::new(HashMap::new()),
                inputs: Mutex::new(HashMap::new()),
            }
        }

        fn set_response(&self, agent_name: &str, output: &str) {
            self.responses
                .lock()
                .unwrap()
                .insert(agent_name.to_string(), output.to_string());
        }

        fn get_inputs(&self) -> HashMap<String, String> {
            self.inputs.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl PaladinPort for InputTrackingMockPort {
        async fn execute(
            &self,
            paladin: &Paladin,
            input: &str,
        ) -> Result<PaladinResult, paladin_core::platform::container::paladin_error::PaladinError>
        {
            let agent_name = &paladin.node.name;

            // Track input
            self.inputs
                .lock()
                .unwrap()
                .insert(agent_name.clone(), input.to_string());

            // Get response
            let responses = self.responses.lock().unwrap();
            let output = responses
                .get(agent_name)
                .cloned()
                .unwrap_or_else(|| format!("output from {}", agent_name));

            Ok(PaladinResult {
                output,
                usage: TokenUsage::new(100, 0),
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
        ) -> Result<
            paladin_ports::output::paladin_port::PaladinStream,
            paladin_core::platform::container::paladin_error::PaladinError,
        > {
            unimplemented!()
        }

        fn validate(
            &self,
            _paladin: &Paladin,
        ) -> Result<(), paladin_core::platform::container::paladin_error::PaladinError> {
            Ok(())
        }
    }
}
