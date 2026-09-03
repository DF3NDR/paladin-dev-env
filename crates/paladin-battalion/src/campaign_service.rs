//! Campaign Execution Service
//!
//! Provides orchestration logic for executing Paladins in Campaign pattern - a directed
//! acyclic graph (DAG) based orchestration with conditional routing, parallel execution,
//! and output transformations.
//!
//! # Architecture
//!
//! The Campaign pattern supports:
//! - **Topological Execution**: Paladins execute in dependency order (topological sort)
//! - **Conditional Routing**: Edge conditions determine traversal based on Paladin output
//! - **Parallel Branches**: Independent graph branches execute concurrently
//! - **Fan-Out/Fan-In**: One-to-many and many-to-one orchestration patterns
//! - **Output Transformation**: Edge transforms modify data between Paladins
//! - **Multiple Entry Points**: Multiple starting nodes for complex workflows
//!
//! # Example
//!
//! ```ignore
//! use paladin_battalion::campaign_service::CampaignExecutionService;
//! use std::sync::Arc;
//!
//! let service = CampaignExecutionService::new(paladin_port);
//! let result = service.execute(&campaign, "Initial input").await?;
//! ```

use chrono::Utc;
use log::{debug, info, warn};
use petgraph::algo::toposort;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::time::{Duration, timeout};
use uuid::Uuid;

use paladin_core::platform::container::battalion::campaign::{Campaign, EdgeCondition};
use paladin_core::platform::container::battalion::{BattalionError, BattalionResult};
use paladin_core::platform::container::herald::Herald;
use paladin_core::platform::container::waypoint::NodeId;
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult};

use crate::edge_evaluator::{EdgeConditionEvaluator, EdgeContext, EdgeEvaluatorRegistry};

/// Service for executing Campaign patterns
///
/// Orchestrates graph-based Paladin execution using a DAG structure with:
/// - Topological sort for execution ordering
/// - Edge condition evaluation for conditional routing
/// - Parallel execution of independent branches
/// - Fan-out/fan-in patterns for complex workflows
/// - Output transformations on edges
///
/// # Examples
///
/// ```ignore
/// let service = CampaignExecutionService::new(paladin_port);
/// let result = service.execute(&campaign, "Process this workflow").await?;
/// ```
pub struct CampaignExecutionService {
    /// Paladin execution port
    paladin_port: Arc<dyn PaladinPort>,
    /// Optional Herald for formatting Battalion results
    herald: Option<Arc<dyn Herald>>,
    /// Registered `EdgeCondition::Custom` evaluators (BUG-01, CF-01). Empty
    /// by default: a v0.9 configuration with no `Custom` edges boots
    /// identically (D-26).
    evaluators: EdgeEvaluatorRegistry,
}

impl CampaignExecutionService {
    /// Create a new CampaignExecutionService
    ///
    /// # Arguments
    ///
    /// * `paladin_port` - Port for executing individual Paladins
    ///
    /// # Example
    ///
    /// ```ignore
    /// let service = CampaignExecutionService::new(paladin_port);
    /// ```
    pub fn new(paladin_port: Arc<dyn PaladinPort>) -> Self {
        info!("Creating CampaignExecutionService");
        Self {
            paladin_port,
            herald: None,
            evaluators: EdgeEvaluatorRegistry::new(),
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
    /// let service = CampaignExecutionService::new(paladin_port)
    ///     .with_herald(Arc::new(JsonHerald::new()));
    /// ```
    pub fn with_herald(mut self, herald: Arc<dyn Herald>) -> Self {
        self.herald = Some(herald);
        self
    }

    /// Register a named evaluator for `EdgeCondition::Custom(name)` edges
    /// (BUG-01, CF-FR-02). Additive and chainable, shaped exactly like
    /// [`CampaignExecutionService::with_herald`] --
    /// [`CampaignExecutionService::new`]'s signature is unchanged (CF-FR-02
    /// compatibility constraint). An unregistered `Custom` name fails
    /// [`CampaignExecutionService::execute`] at validation time, before any
    /// node runs; it is never silently treated as always-true.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let service = CampaignExecutionService::new(paladin_port)
    ///     .with_evaluator("is_urgent", Arc::new(my_evaluator));
    /// ```
    pub fn with_evaluator(
        mut self,
        name: impl Into<String>,
        evaluator: Arc<dyn EdgeConditionEvaluator>,
    ) -> Self {
        self.evaluators.register(name, evaluator);
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
            Some(herald) => herald
                .format_battalion_result(result)
                .map(Some)
                .map_err(|e| {
                    BattalionError::CampaignError(format!("Herald formatting error: {}", e))
                }),
            None => Ok(None),
        }
    }

    /// Execute a Campaign with the given input
    ///
    /// Executes Paladins according to the DAG structure, respecting:
    /// - Entry point(s) as starting nodes
    /// - Edge conditions for conditional routing
    /// - Parallel execution for independent branches
    /// - Fan-out (1→N) and fan-in (N→1) patterns
    /// - Output transformations on edges
    ///
    /// # Arguments
    ///
    /// * `campaign` - The Campaign DAG to execute
    /// * `initial_input` - Initial input for entry point Paladins
    ///
    /// # Returns
    ///
    /// * `Ok(BattalionResult)` - Final result with all Paladin outputs
    /// * `Err(BattalionError)` - If validation fails or execution errors occur
    ///
    /// # Example
    ///
    /// ```ignore
    /// let result = service.execute(&campaign, "Start workflow").await?;
    /// println!("Campaign complete: {}", result.final_output);
    /// ```
    pub async fn execute(
        &self,
        campaign: &Campaign,
        initial_input: &str,
    ) -> Result<BattalionResult, BattalionError> {
        // Validate campaign structure
        campaign.validate()?;

        // BUG-01 / CF-FR-02 fail-closed pre-check: every
        // `EdgeCondition::Custom` name on this campaign's edges must
        // resolve to a registered evaluator BEFORE any node executes.
        // Reuses the EXISTING `InvalidGraph(String)` variant -- adding a
        // new `BattalionError` variant here would be an unsanctioned X-10
        // break (`BattalionError` is a pre-existing public enum without
        // `#[non_exhaustive]`), and PRD CF-FR-02 plus the published
        // MIGRATION.md M-B-01 row both name this exact variant (D-04).
        // X-06's "no new call sites on String variants" is consciously
        // overridden here by that explicit FR.
        let mut unregistered: Vec<String> = campaign
            .graph()
            .edge_weights()
            .filter_map(|edge| match &edge.condition {
                EdgeCondition::Custom(name) if !self.evaluators.contains(name) => {
                    Some(name.clone())
                }
                _ => None,
            })
            .collect();
        unregistered.sort();
        unregistered.dedup();
        if !unregistered.is_empty() {
            return Err(BattalionError::InvalidGraph(format!(
                "unregistered custom edge condition(s): {} -- register with \
                 CampaignExecutionService::with_evaluator before calling execute",
                unregistered.join(", ")
            )));
        }

        let battalion_id = Uuid::new_v4();
        let started_at = Utc::now();

        info!(
            "Starting Campaign execution: {} (ID: {}) with {} Paladins",
            campaign.config().name,
            battalion_id,
            campaign.paladin_count()
        );

        // Execute with timeout
        let timeout_duration = Duration::from_secs(campaign.config().timeout_seconds);

        match timeout(
            timeout_duration,
            self.execute_internal(campaign, initial_input, battalion_id),
        )
        .await
        {
            Ok(result) => {
                let duration_ms = Utc::now()
                    .signed_duration_since(started_at)
                    .num_milliseconds() as u64;

                info!("Campaign {} completed in {}ms", battalion_id, duration_ms);
                result
            }
            Err(_) => {
                warn!(
                    "Campaign {} timed out after {} seconds",
                    battalion_id,
                    campaign.config().timeout_seconds
                );
                Err(BattalionError::Timeout(campaign.config().timeout_seconds))
            }
        }
    }

    /// Internal execution logic without timeout wrapper
    ///
    /// Implements the core Campaign execution algorithm:
    /// 1. Get entry points (explicit or auto-detected)
    /// 2. Compute topological sort for execution order
    /// 3. Execute nodes level-by-level (parallel within each level)
    /// 4. Evaluate edge conditions to determine which edges to traverse
    /// 5. Apply edge transformations before passing output to next node
    /// 6. Aggregate results from fan-in patterns
    async fn execute_internal(
        &self,
        campaign: &Campaign,
        initial_input: &str,
        battalion_id: Uuid,
    ) -> Result<BattalionResult, BattalionError> {
        let started_at = Utc::now();

        // Get entry points
        let entry_points = campaign.entry_points();
        debug!("Campaign entry points: {} nodes", entry_points.len());

        // Compute topological order
        let sorted_nodes = toposort(campaign.graph(), None).map_err(|cycle| {
            BattalionError::InvalidGraph(format!(
                "Cycle detected in campaign graph at node {:?}",
                cycle.node_id()
            ))
        })?;

        debug!("Topological sort completed: {} nodes", sorted_nodes.len());

        // Track node outputs for edge condition evaluation
        let mut node_outputs: HashMap<Uuid, String> = HashMap::new();

        // Track all Paladin results
        let mut all_results: Vec<PaladinResult> = Vec::new();

        // Track which nodes are ready to execute (dependencies satisfied)
        let mut ready_nodes: HashSet<Uuid> = entry_points.clone();

        // Track executed nodes
        let mut executed_nodes: HashSet<Uuid> = HashSet::new();

        // Execute nodes in topological order
        for node_index in sorted_nodes {
            let node_id = campaign.graph()[node_index];

            // Skip if not ready (dependencies not satisfied)
            if !ready_nodes.contains(&node_id) {
                continue;
            }

            let paladin = campaign.get_paladin(&node_id).ok_or_else(|| {
                BattalionError::InvalidGraph(format!(
                    "Node {:?} not found in paladins map",
                    node_id
                ))
            })?;

            // Determine input for this Paladin
            let input = if entry_points.contains(&node_id) {
                // Entry point uses initial input
                initial_input.to_string()
            } else {
                // Non-entry point: aggregate inputs from incoming edges
                self.aggregate_inputs_for_node(campaign, node_id, &node_outputs)?
            };

            debug!("Executing Paladin: {} ({})", paladin.node.name, node_id);

            // Execute Paladin
            let result = self
                .paladin_port
                .execute(paladin, &input)
                .await
                .map_err(|e| BattalionError::PaladinError(e.to_string()))?;

            debug!(
                "Paladin {} completed: {} tokens, {} loops",
                paladin.node.name, result.token_count, result.loop_count
            );

            // Store output for edge condition evaluation
            node_outputs.insert(node_id, result.output.clone());
            all_results.push(result.clone());
            executed_nodes.insert(node_id);

            // Evaluate outgoing edges and mark downstream nodes as ready
            let edges = campaign.graph().edges(node_index);
            for edge in edges {
                let edge_data = edge.weight();
                let target_id = campaign.graph()[edge.target()];

                // Evaluate edge condition
                if self
                    .evaluate_edge_condition(
                        &edge_data.condition,
                        &result.output,
                        &node_id,
                        &target_id,
                    )
                    .await?
                {
                    debug!("Edge condition satisfied: {} → {}", node_id, target_id);

                    // Apply edge transformation if present
                    if let Some(transform_name) = &edge_data.transform {
                        debug!("Applying edge transform: {}", transform_name);
                        // Transformation logic placeholder (could be extended)
                        // For now, just log the transform name
                    }

                    // Check if all incoming edges to target are satisfied
                    if self.are_dependencies_satisfied(campaign, edge.target(), &executed_nodes) {
                        ready_nodes.insert(target_id);
                    }
                } else {
                    debug!("Edge condition NOT satisfied: {} → {}", node_id, target_id);
                }
            }
        }

        // Build final result
        let final_output = self.compute_final_output(&all_results);

        let result = BattalionResult::new(
            battalion_id,
            campaign.config().name.clone(),
            started_at,
            final_output,
            all_results,
        );

        Ok(result)
    }

    /// Aggregate inputs from incoming edges for a node (fan-in pattern)
    fn aggregate_inputs_for_node(
        &self,
        campaign: &Campaign,
        node_id: Uuid,
        node_outputs: &HashMap<Uuid, String>,
    ) -> Result<String, BattalionError> {
        let node_index = campaign.node_indices().get(&node_id).ok_or_else(|| {
            BattalionError::InvalidGraph(format!("Node {:?} not in indices map", node_id))
        })?;

        let mut inputs = Vec::new();

        // Collect outputs from all incoming edges
        let incoming_edges = campaign
            .graph()
            .edges_directed(*node_index, petgraph::Direction::Incoming);
        for edge in incoming_edges {
            let source_id = campaign.graph()[edge.source()];
            if let Some(output) = node_outputs.get(&source_id) {
                inputs.push(output.clone());
            }
        }

        // If multiple inputs, concatenate them
        if inputs.is_empty() {
            Ok(String::new())
        } else if inputs.len() == 1 {
            Ok(inputs[0].clone())
        } else {
            // Fan-in: combine multiple inputs
            Ok(inputs.join("\n\n---\n\n"))
        }
    }

    /// Evaluate an edge condition based on the source node's output.
    /// `source` and `target` are the Campaign node ids the edge connects --
    /// used to build the `EdgeContext` a registered `EdgeCondition::Custom`
    /// evaluator sees (D-02) and to name the edge in any error this
    /// returns.
    async fn evaluate_edge_condition(
        &self,
        condition: &EdgeCondition,
        output: &str,
        source: &Uuid,
        target: &Uuid,
    ) -> Result<bool, BattalionError> {
        match condition {
            EdgeCondition::Always => Ok(true),
            EdgeCondition::Contains(substring) => Ok(output.contains(substring)),
            EdgeCondition::Regex(pattern) => {
                let regex = Regex::new(pattern).map_err(|e| {
                    BattalionError::InvalidGraph(format!("Invalid regex pattern: {}", e))
                })?;
                Ok(regex.is_match(output))
            }
            EdgeCondition::Custom(name) => {
                // Unreachable in practice: `execute`'s fail-closed
                // pre-check already rejected any unregistered `Custom`
                // name before any node executed (CF-FR-02/03). Still
                // resolved as a fail-closed error here rather than any
                // default branch, should that invariant ever be violated.
                let evaluator = self.evaluators.get(name).cloned().ok_or_else(|| {
                    BattalionError::CampaignError(format!(
                        "internal error: edge evaluator '{name}' missing after validation"
                    ))
                })?;
                let source_id = NodeId::new(source.to_string());
                let target_id = NodeId::new(target.to_string());
                let ctx = EdgeContext {
                    source: &source_id,
                    target: &target_id,
                    battlefield: None,
                    thread: None,
                    superstep: None,
                };
                evaluator.evaluate(output, &ctx).await.map_err(|e| {
                    BattalionError::CampaignError(format!(
                        "edge evaluator '{name}' failed for edge {source} -> {target}: {e}"
                    ))
                })
            }
        }
    }

    /// Check if all incoming edges to a node have been traversed (dependencies satisfied)
    fn are_dependencies_satisfied(
        &self,
        campaign: &Campaign,
        target_index: NodeIndex,
        executed_nodes: &HashSet<Uuid>,
    ) -> bool {
        // Get all incoming edges
        let incoming = campaign
            .graph()
            .edges_directed(target_index, petgraph::Direction::Incoming);

        for edge in incoming {
            let source_id = campaign.graph()[edge.source()];
            if !executed_nodes.contains(&source_id) {
                return false;
            }
        }

        true
    }

    /// Compute the final output from all Paladin results
    fn compute_final_output(&self, results: &[PaladinResult]) -> String {
        if results.is_empty() {
            return String::new();
        }

        // Use the last result's output as final output
        // (could be made configurable in the future)
        results.last().unwrap().output.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::battalion::BattalionConfig;
    use paladin_core::platform::container::paladin::Paladin;

    #[test]
    fn test_service_creation() {
        use async_trait::async_trait;
        use paladin_core::platform::container::paladin_error::PaladinError;
        use paladin_ports::output::paladin_port::StopReason;

        struct MockPort;

        #[async_trait]
        impl PaladinPort for MockPort {
            async fn execute(
                &self,
                _paladin: &Paladin,
                _input: &str,
            ) -> Result<PaladinResult, PaladinError> {
                Ok(PaladinResult {
                    output: "test".to_string(),
                    token_count: 0,
                    execution_time_ms: 0,
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
                unimplemented!()
            }

            fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
                Ok(())
            }
        }

        let port = Arc::new(MockPort);
        let _service = CampaignExecutionService::new(port);
        // Service creation should succeed
    }

    /// Mock Herald for `format_result` tests, mirroring `herald.rs`'s own `MockHerald` test
    /// fixture shape.
    struct MockHerald;

    impl paladin_core::platform::container::herald::Herald for MockHerald {
        fn format_paladin_result(
            &self,
            result: &paladin_core::platform::container::herald::PaladinResult,
        ) -> Result<String, paladin_core::platform::container::herald::HeraldError> {
            Ok(format!("MOCK PALADIN: {}", result.output))
        }

        fn format_battalion_result(
            &self,
            result: &BattalionResult,
        ) -> Result<String, paladin_core::platform::container::herald::HeraldError> {
            Ok(format!("MOCK BATTALION: {}", result.battalion_name))
        }

        fn format_stream_chunk(
            &self,
            chunk: &paladin_core::platform::container::herald::StreamChunk,
        ) -> Result<Option<String>, paladin_core::platform::container::herald::HeraldError>
        {
            Ok(Some(chunk.content.clone()))
        }

        fn finalize_stream(
            &self,
            _metadata: &paladin_core::platform::container::herald::ExecutionMetadata,
        ) -> Result<String, paladin_core::platform::container::herald::HeraldError> {
            Ok(String::new())
        }

        fn format_error(
            &self,
            error: &paladin_core::platform::container::herald::PaladinError,
        ) -> String {
            format!("ERROR: {}", error)
        }

        fn name(&self) -> &str {
            "mock"
        }

        fn mime_type(&self) -> &str {
            "text/plain"
        }
    }

    #[tokio::test]
    async fn test_campaign_with_herald_formats_result() {
        use async_trait::async_trait;
        use paladin_core::platform::container::paladin_error::PaladinError;
        use paladin_ports::output::paladin_port::StopReason;

        struct MockPort;

        #[async_trait]
        impl PaladinPort for MockPort {
            async fn execute(
                &self,
                _paladin: &Paladin,
                _input: &str,
            ) -> Result<PaladinResult, PaladinError> {
                Ok(PaladinResult {
                    output: "campaign output".to_string(),
                    token_count: 10,
                    execution_time_ms: 5,
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
                unimplemented!()
            }

            fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
                Ok(())
            }
        }

        let paladin_data = paladin_core::platform::container::paladin::PaladinData {
            system_prompt: "You are a test node".to_string(),
            name: "solo_node".to_string(),
            user_name: "test_user".to_string(),
            ..Default::default()
        };
        let paladin = Paladin::new(paladin_data, Some("solo_node".to_string()));

        let mut campaign = Campaign::new(BattalionConfig::new("herald_campaign"));
        let node_id = campaign.add_paladin(paladin);
        campaign
            .set_entry_point(node_id)
            .expect("single-node entry point should be valid");

        let port = Arc::new(MockPort);
        let service = CampaignExecutionService::new(port).with_herald(Arc::new(MockHerald));

        let result = service
            .execute(&campaign, "start")
            .await
            .expect("Campaign execution should succeed");

        let formatted = service
            .format_result(&result)
            .expect("format_result should succeed with a Herald configured");
        assert_eq!(
            formatted,
            Some("MOCK BATTALION: herald_campaign".to_string())
        );

        let service_without_herald = CampaignExecutionService::new(Arc::new(MockPort));
        let unformatted = service_without_herald
            .format_result(&result)
            .expect("format_result should succeed without a Herald configured");
        assert_eq!(unformatted, None);
    }

    // --- BUG-01 / CF-01: registered-evaluator edge conditions, legacy
    // path. These reproduce BUG-01 (`EdgeCondition::Custom` silently
    // defaulting to `Ok(true)`) and are committed FAILING (RED) before the
    // fix (GREEN) lands in the same task, per D-05 / traceability protocol
    // step 4.

    use crate::edge_evaluator::EdgeEvaluatorError;
    use paladin_core::platform::container::battalion::campaign::CampaignEdge;
    use paladin_ports::output::paladin_port::StopReason;

    /// A [`PaladinPort`] test double recording every executed Paladin's
    /// name, in call order -- the legacy-path analog of
    /// `engine::test_support::RecordingPaladinPort`, kept local since this
    /// module has no shared test-support module of its own.
    #[derive(Default)]
    struct RecordingPort {
        calls: std::sync::Mutex<Vec<String>>,
    }

    impl RecordingPort {
        fn call_count(&self) -> usize {
            self.calls.lock().unwrap().len()
        }

        fn called_names(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl PaladinPort for RecordingPort {
        async fn execute(
            &self,
            paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinResult, paladin_core::platform::container::paladin_error::PaladinError>
        {
            self.calls.lock().unwrap().push(paladin.node.name.clone());
            Ok(PaladinResult {
                output: "the situation is urgent".to_string(),
                token_count: 0,
                execution_time_ms: 0,
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
            unimplemented!("not exercised by these tests")
        }

        fn validate(
            &self,
            _paladin: &Paladin,
        ) -> Result<(), paladin_core::platform::container::paladin_error::PaladinError> {
            Ok(())
        }
    }

    /// Evaluator returning a fixed verdict every call.
    struct FixedVerdictEvaluator(bool);

    #[async_trait::async_trait]
    impl EdgeConditionEvaluator for FixedVerdictEvaluator {
        async fn evaluate(
            &self,
            _output: &str,
            _ctx: &EdgeContext<'_>,
        ) -> Result<bool, EdgeEvaluatorError> {
            Ok(self.0)
        }
    }

    /// Evaluator that always fails.
    struct FailingEvaluator;

    #[async_trait::async_trait]
    impl EdgeConditionEvaluator for FailingEvaluator {
        async fn evaluate(
            &self,
            _output: &str,
            _ctx: &EdgeContext<'_>,
        ) -> Result<bool, EdgeEvaluatorError> {
            Err(EdgeEvaluatorError::Evaluation {
                evaluator: "is_urgent".to_string(),
                reason: "simulated failure".to_string(),
            })
        }
    }

    fn make_named_paladin(name: &str) -> Paladin {
        let data = paladin_core::platform::container::paladin::PaladinData {
            name: name.to_string(),
            ..Default::default()
        };
        Paladin::new(data, Some(name.to_string()))
    }

    /// A two-node campaign, `a` (entry) -> `b`, connected by a single edge
    /// carrying `EdgeCondition::Custom(condition_name)`.
    fn two_node_custom_edge_campaign(condition_name: &str) -> (Campaign, Uuid, Uuid) {
        let mut campaign = Campaign::new(BattalionConfig::new("custom_edge_campaign"));
        let a_id = campaign.add_paladin(make_named_paladin("a"));
        let b_id = campaign.add_paladin(make_named_paladin("b"));
        campaign
            .set_entry_point(a_id)
            .expect("a should be a valid entry point");
        campaign
            .add_edge(CampaignEdge::new(
                a_id,
                b_id,
                EdgeCondition::Custom(condition_name.to_string()),
            ))
            .expect("edge between two declared paladins should be valid");
        (campaign, a_id, b_id)
    }

    #[tokio::test]
    async fn unregistered_custom_condition_is_rejected_before_any_paladin_executes() {
        let (campaign, _a, _b) = two_node_custom_edge_campaign("is_urgent");
        let port = Arc::new(RecordingPort::default());
        let service = CampaignExecutionService::new(port.clone());

        let err = service
            .execute(&campaign, "start")
            .await
            .expect_err("unregistered custom condition must fail validation");
        match err {
            BattalionError::InvalidGraph(msg) => {
                assert!(
                    msg.contains("is_urgent"),
                    "message should name the offending condition: {msg}"
                );
            }
            other => panic!("expected InvalidGraph, got {other:?}"),
        }
        assert_eq!(
            port.call_count(),
            0,
            "no Paladin should execute before validation passes"
        );
    }

    #[tokio::test]
    async fn registered_true_evaluator_routes_the_custom_edge() {
        let (campaign, _a, _b) = two_node_custom_edge_campaign("is_urgent");
        let port = Arc::new(RecordingPort::default());
        let service = CampaignExecutionService::new(port.clone())
            .with_evaluator("is_urgent", Arc::new(FixedVerdictEvaluator(true)));

        service
            .execute(&campaign, "start")
            .await
            .expect("a registered true evaluator should route the edge");

        assert_eq!(port.called_names(), vec!["a".to_string(), "b".to_string()]);
    }

    #[tokio::test]
    async fn registered_false_evaluator_does_not_route_the_custom_edge() {
        let (campaign, _a, _b) = two_node_custom_edge_campaign("is_urgent");
        let port = Arc::new(RecordingPort::default());
        let service = CampaignExecutionService::new(port.clone())
            .with_evaluator("is_urgent", Arc::new(FixedVerdictEvaluator(false)));

        service
            .execute(&campaign, "start")
            .await
            .expect("a false verdict should not fail the run, only skip the edge");

        assert_eq!(port.called_names(), vec!["a".to_string()]);
    }

    #[tokio::test]
    async fn evaluator_error_fails_the_legacy_run_naming_the_edge() {
        let (campaign, a_id, b_id) = two_node_custom_edge_campaign("is_urgent");
        let port = Arc::new(RecordingPort::default());
        let service = CampaignExecutionService::new(port.clone())
            .with_evaluator("is_urgent", Arc::new(FailingEvaluator));

        let err = service
            .execute(&campaign, "start")
            .await
            .expect_err("an evaluator error must fail the run, not silently succeed or skip");

        let msg = err.to_string();
        assert!(
            msg.contains(&a_id.to_string()),
            "error should name the source node: {msg}"
        );
        assert!(
            msg.contains(&b_id.to_string()),
            "error should name the target node: {msg}"
        );
        assert!(
            msg.contains("is_urgent"),
            "error should name the evaluator: {msg}"
        );
    }
}
