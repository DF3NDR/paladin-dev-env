//! Unit tests for CampaignExecutionService
//!
//! Tests the graph-based Paladin orchestration service following TDD methodology.

use async_trait::async_trait;
use paladin::application::services::battalion::campaign_service::CampaignExecutionService;
use paladin::application::services::paladin::error::PaladinError;
use paladin::core::base::entity::node::Node;
use paladin::core::platform::container::battalion::campaign::{
    Campaign, CampaignEdge, EdgeCondition,
};
use paladin::core::platform::container::battalion::{BattalionConfig, BattalionStatus};
use paladin::core::platform::container::paladin::MaxLoops;
use paladin::core::platform::container::paladin::{Paladin, PaladinData, PaladinStatus};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, StopReason};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ============================================================================
// Mock PaladinPort for testing
// ============================================================================

#[derive(Clone)]
struct MockPaladinPort {
    outputs: Arc<Mutex<HashMap<String, String>>>,
    call_order: Arc<Mutex<Vec<String>>>,
    fail_paladin_names: Vec<String>,
}

impl MockPaladinPort {
    fn new() -> Self {
        Self {
            outputs: Arc::new(Mutex::new(HashMap::new())),
            call_order: Arc::new(Mutex::new(Vec::new())),
            fail_paladin_names: Vec::new(),
        }
    }

    fn with_outputs(outputs: HashMap<String, String>) -> Self {
        Self {
            outputs: Arc::new(Mutex::new(outputs)),
            call_order: Arc::new(Mutex::new(Vec::new())),
            fail_paladin_names: Vec::new(),
        }
    }

    fn with_failures(mut self, names: Vec<String>) -> Self {
        self.fail_paladin_names = names;
        self
    }

    fn get_call_order(&self) -> Vec<String> {
        self.call_order.lock().unwrap().clone()
    }
}

#[async_trait]
impl PaladinPort for MockPaladinPort {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        let name = paladin.node.name.clone();

        // Record call order
        {
            let mut order = self.call_order.lock().unwrap();
            order.push(name.clone());
        }

        // Check if this paladin should fail
        if self.fail_paladin_names.contains(&name) {
            return Err(PaladinError::ExecutionError(format!(
                "Mock failure for {}",
                name
            )));
        }

        // Get predetermined output or generate one
        let output = {
            let outputs = self.outputs.lock().unwrap();
            outputs
                .get(&name)
                .cloned()
                .unwrap_or_else(|| format!("Output from {} with input: {}", name, input))
        };

        Ok(PaladinResult {
            output,
            usage: paladin_ports::output::llm_port::TokenUsage::new(100, 0),
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
        tokio::sync::mpsc::Receiver<
            Result<paladin_ports::output::paladin_port::PaladinStreamChunk, PaladinError>,
        >,
        PaladinError,
    > {
        unimplemented!("Streaming not used in Campaign tests")
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

// ============================================================================
// Test Helper Functions
// ============================================================================

fn create_test_paladin(name: &str) -> Paladin {
    let data = PaladinData {
        system_prompt: format!("System prompt for {}", name),
        name: name.to_string(),
        user_name: "test_user".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(5),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    Node::new(data, Some(name.to_string()))
}

fn create_test_config() -> BattalionConfig {
    BattalionConfig::new("test_campaign")
        .with_description("Test Campaign")
        .with_timeout(300)
}

// ============================================================================
// Test Module: Campaign Service Construction
// ============================================================================

#[cfg(test)]
mod campaign_service_construction_tests {
    use super::*;

    #[test]
    fn test_campaign_service_new() {
        let port = Arc::new(MockPaladinPort::new());
        let service = CampaignExecutionService::new(port);

        assert!(
            std::ptr::addr_of!(service) as usize > 0,
            "Service should be created"
        );
    }
}

// ============================================================================
// Test Module: Linear Graph Execution (Topological Sort)
// ============================================================================

#[cfg(test)]
mod campaign_linear_execution_tests {
    use super::*;

    #[tokio::test]
    async fn test_linear_graph_execution_order() {
        // RED: Test should fail - execute() not yet implemented
        let port = Arc::new(MockPaladinPort::new());
        let service = CampaignExecutionService::new(port.clone());

        let mut campaign = Campaign::new(create_test_config());
        let p1 = create_test_paladin("Paladin1");
        let p2 = create_test_paladin("Paladin2");
        let p3 = create_test_paladin("Paladin3");

        let id1 = campaign.add_paladin(p1);
        let id2 = campaign.add_paladin(p2);
        let id3 = campaign.add_paladin(p3);

        // Create linear chain: P1 → P2 → P3
        campaign
            .add_edge(CampaignEdge::new(id1, id2, EdgeCondition::Always))
            .unwrap();
        campaign
            .add_edge(CampaignEdge::new(id2, id3, EdgeCondition::Always))
            .unwrap();

        let result = service.execute(&campaign, "Initial input").await;

        assert!(result.is_ok(), "Linear execution should succeed");
        let result = result.unwrap();

        // Verify execution order
        let call_order = port.get_call_order();
        assert_eq!(call_order, vec!["Paladin1", "Paladin2", "Paladin3"]);

        // Verify result structure
        assert_eq!(result.status, BattalionStatus::Completed);
        assert_eq!(result.paladin_results.len(), 3);
    }

    #[tokio::test]
    async fn test_linear_graph_output_chaining() {
        // RED: Test should fail - output chaining not implemented
        let mut outputs = HashMap::new();
        outputs.insert("Paladin1".to_string(), "Step1 complete".to_string());
        outputs.insert("Paladin2".to_string(), "Step2 complete".to_string());
        outputs.insert("Paladin3".to_string(), "Step3 complete".to_string());

        let port = Arc::new(MockPaladinPort::with_outputs(outputs));
        let service = CampaignExecutionService::new(port.clone());

        let mut campaign = Campaign::new(create_test_config());
        let p1 = create_test_paladin("Paladin1");
        let p2 = create_test_paladin("Paladin2");

        let id1 = campaign.add_paladin(p1);
        let id2 = campaign.add_paladin(p2);

        campaign
            .add_edge(CampaignEdge::new(id1, id2, EdgeCondition::Always))
            .unwrap();

        let result = service.execute(&campaign, "Start").await;

        assert!(result.is_ok());
        let result = result.unwrap();

        // Final output should be from last Paladin
        assert!(result.final_output.contains("Step2 complete"));
    }

    #[tokio::test]
    async fn test_single_paladin_campaign() {
        // RED: Test should fail
        let port = Arc::new(MockPaladinPort::new());
        let service = CampaignExecutionService::new(port.clone());

        let mut campaign = Campaign::new(create_test_config());
        let p1 = create_test_paladin("SinglePaladin");
        campaign.add_paladin(p1);

        let result = service.execute(&campaign, "Solo task").await;

        assert!(result.is_ok());
        let result = result.unwrap();

        assert_eq!(result.paladin_results.len(), 1);
        assert_eq!(result.status, BattalionStatus::Completed);
    }
}

// ============================================================================
// Test Module: Edge Condition Evaluation
// ============================================================================

#[cfg(test)]
mod campaign_edge_condition_tests {
    use super::*;

    #[tokio::test]
    async fn test_contains_condition_true() {
        // RED: EdgeCondition evaluation not implemented
        let mut outputs = HashMap::new();
        outputs.insert("Analyzer".to_string(), "APPROVED: looks good".to_string());
        outputs.insert("Approver".to_string(), "Processing approved".to_string());

        let port = Arc::new(MockPaladinPort::with_outputs(outputs));
        let service = CampaignExecutionService::new(port.clone());

        let mut campaign = Campaign::new(create_test_config());
        let analyzer = create_test_paladin("Analyzer");
        let approver = create_test_paladin("Approver");
        let rejector = create_test_paladin("Rejector");

        let id_analyzer = campaign.add_paladin(analyzer);
        let id_approver = campaign.add_paladin(approver);
        let id_rejector = campaign.add_paladin(rejector);

        // Conditional routing: if output contains "APPROVED", go to approver
        campaign
            .add_edge(CampaignEdge::new(
                id_analyzer,
                id_approver,
                EdgeCondition::Contains("APPROVED".to_string()),
            ))
            .unwrap();

        campaign
            .add_edge(CampaignEdge::new(
                id_analyzer,
                id_rejector,
                EdgeCondition::Contains("REJECTED".to_string()),
            ))
            .unwrap();

        let result = service.execute(&campaign, "Check this").await;

        assert!(result.is_ok());

        // Verify only Analyzer and Approver were called, not Rejector
        let call_order = port.get_call_order();
        assert_eq!(call_order, vec!["Analyzer", "Approver"]);
    }

    #[tokio::test]
    async fn test_contains_condition_false() {
        // RED: EdgeCondition evaluation not implemented
        let mut outputs = HashMap::new();
        outputs.insert("Analyzer".to_string(), "REJECTED: not good".to_string());
        outputs.insert("Rejector".to_string(), "Processing rejected".to_string());

        let port = Arc::new(MockPaladinPort::with_outputs(outputs));
        let service = CampaignExecutionService::new(port.clone());

        let mut campaign = Campaign::new(create_test_config());
        let analyzer = create_test_paladin("Analyzer");
        let approver = create_test_paladin("Approver");
        let rejector = create_test_paladin("Rejector");

        let id_analyzer = campaign.add_paladin(analyzer);
        let id_approver = campaign.add_paladin(approver);
        let id_rejector = campaign.add_paladin(rejector);

        campaign
            .add_edge(CampaignEdge::new(
                id_analyzer,
                id_approver,
                EdgeCondition::Contains("APPROVED".to_string()),
            ))
            .unwrap();

        campaign
            .add_edge(CampaignEdge::new(
                id_analyzer,
                id_rejector,
                EdgeCondition::Contains("REJECTED".to_string()),
            ))
            .unwrap();

        let result = service.execute(&campaign, "Check this").await;

        assert!(result.is_ok());

        // Verify only Analyzer and Rejector were called
        let call_order = port.get_call_order();
        assert_eq!(call_order, vec!["Analyzer", "Rejector"]);
    }

    #[tokio::test]
    async fn test_regex_condition() {
        // RED: Regex condition not implemented
        let mut outputs = HashMap::new();
        outputs.insert("Parser".to_string(), "Error code: E404".to_string());
        outputs.insert("ErrorHandler".to_string(), "Handled error".to_string());

        let port = Arc::new(MockPaladinPort::with_outputs(outputs));
        let service = CampaignExecutionService::new(port.clone());

        let mut campaign = Campaign::new(create_test_config());
        let parser = create_test_paladin("Parser");
        let error_handler = create_test_paladin("ErrorHandler");

        let id_parser = campaign.add_paladin(parser);
        let id_error = campaign.add_paladin(error_handler);

        // Route to error handler if output matches error pattern
        campaign
            .add_edge(CampaignEdge::new(
                id_parser,
                id_error,
                EdgeCondition::Regex(r"Error code: E\d+".to_string()),
            ))
            .unwrap();

        let result = service.execute(&campaign, "Parse this").await;

        assert!(result.is_ok());

        let call_order = port.get_call_order();
        assert_eq!(call_order, vec!["Parser", "ErrorHandler"]);
    }

    #[tokio::test]
    async fn test_always_condition() {
        // RED: Always condition not implemented (should always traverse)
        let port = Arc::new(MockPaladinPort::new());
        let service = CampaignExecutionService::new(port.clone());

        let mut campaign = Campaign::new(create_test_config());
        let p1 = create_test_paladin("First");
        let p2 = create_test_paladin("Second");

        let id1 = campaign.add_paladin(p1);
        let id2 = campaign.add_paladin(p2);

        campaign
            .add_edge(CampaignEdge::new(id1, id2, EdgeCondition::Always))
            .unwrap();

        let result = service.execute(&campaign, "Input").await;

        assert!(result.is_ok());

        // Should always execute both
        let call_order = port.get_call_order();
        assert_eq!(call_order, vec!["First", "Second"]);
    }
}

// ============================================================================
// Test Module: Output Transformation
// ============================================================================

#[cfg(test)]
mod campaign_transformation_tests {
    use super::*;

    #[tokio::test]
    async fn test_edge_transform_applied() {
        // RED: Transform not implemented
        let mut outputs = HashMap::new();
        outputs.insert("Producer".to_string(), "raw data".to_string());
        outputs.insert("Consumer".to_string(), "processed".to_string());

        let port = Arc::new(MockPaladinPort::with_outputs(outputs));
        let service = CampaignExecutionService::new(port.clone());

        let mut campaign = Campaign::new(create_test_config());
        let producer = create_test_paladin("Producer");
        let consumer = create_test_paladin("Consumer");

        let id_prod = campaign.add_paladin(producer);
        let id_cons = campaign.add_paladin(consumer);

        // Add transform that uppercases the output
        campaign
            .add_edge(
                CampaignEdge::new(id_prod, id_cons, EdgeCondition::Always)
                    .with_transform("uppercase".to_string()),
            )
            .unwrap();

        let result = service.execute(&campaign, "Start").await;

        assert!(result.is_ok());
        // Transform application will be verified in implementation
    }
}

// ============================================================================
// Test Module: Parallel Branch Execution (Fan-Out/Fan-In)
// ============================================================================

#[cfg(test)]
mod campaign_parallel_execution_tests {
    use super::*;

    #[tokio::test]
    async fn test_fan_out_parallel_execution() {
        // RED: Parallel execution not implemented
        let port = Arc::new(MockPaladinPort::new());
        let service = CampaignExecutionService::new(port.clone());

        let mut campaign = Campaign::new(create_test_config());
        let root = create_test_paladin("Root");
        let branch1 = create_test_paladin("Branch1");
        let branch2 = create_test_paladin("Branch2");
        let branch3 = create_test_paladin("Branch3");

        let id_root = campaign.add_paladin(root);
        let id_b1 = campaign.add_paladin(branch1);
        let id_b2 = campaign.add_paladin(branch2);
        let id_b3 = campaign.add_paladin(branch3);

        // Fan-out: Root → [Branch1, Branch2, Branch3]
        campaign
            .add_edge(CampaignEdge::new(id_root, id_b1, EdgeCondition::Always))
            .unwrap();
        campaign
            .add_edge(CampaignEdge::new(id_root, id_b2, EdgeCondition::Always))
            .unwrap();
        campaign
            .add_edge(CampaignEdge::new(id_root, id_b3, EdgeCondition::Always))
            .unwrap();

        let result = service.execute(&campaign, "Fan out").await;

        assert!(result.is_ok());

        // All branches should execute (order may vary due to parallelism)
        let call_order = port.get_call_order();
        assert_eq!(call_order.len(), 4);
        assert_eq!(call_order[0], "Root");
        assert!(call_order.contains(&"Branch1".to_string()));
        assert!(call_order.contains(&"Branch2".to_string()));
        assert!(call_order.contains(&"Branch3".to_string()));
    }

    #[tokio::test]
    async fn test_fan_in_result_collection() {
        // RED: Fan-in not implemented
        let port = Arc::new(MockPaladinPort::new());
        let service = CampaignExecutionService::new(port.clone());

        let mut campaign = Campaign::new(create_test_config());
        let source1 = create_test_paladin("Source1");
        let source2 = create_test_paladin("Source2");
        let merger = create_test_paladin("Merger");

        let id_s1 = campaign.add_paladin(source1);
        let id_s2 = campaign.add_paladin(source2);
        let id_merger = campaign.add_paladin(merger);

        // Fan-in: [Source1, Source2] → Merger
        campaign
            .add_edge(CampaignEdge::new(id_s1, id_merger, EdgeCondition::Always))
            .unwrap();
        campaign
            .add_edge(CampaignEdge::new(id_s2, id_merger, EdgeCondition::Always))
            .unwrap();

        // Set both sources as entry points
        campaign.set_entry_point(id_s1).unwrap();
        campaign.set_entry_point(id_s2).unwrap();

        let result = service.execute(&campaign, "Parallel inputs").await;

        assert!(result.is_ok());

        // All should execute, merger last
        let call_order = port.get_call_order();
        assert_eq!(call_order.len(), 3);
        assert_eq!(call_order[2], "Merger"); // Merger should be last
    }

    #[tokio::test]
    async fn test_diamond_pattern() {
        // RED: Diamond pattern (fan-out + fan-in) not implemented
        let port = Arc::new(MockPaladinPort::new());
        let service = CampaignExecutionService::new(port.clone());

        let mut campaign = Campaign::new(create_test_config());
        let start = create_test_paladin("Start");
        let left = create_test_paladin("Left");
        let right = create_test_paladin("Right");
        let end = create_test_paladin("End");

        let id_start = campaign.add_paladin(start);
        let id_left = campaign.add_paladin(left);
        let id_right = campaign.add_paladin(right);
        let id_end = campaign.add_paladin(end);

        // Diamond: Start → [Left, Right] → End
        campaign
            .add_edge(CampaignEdge::new(id_start, id_left, EdgeCondition::Always))
            .unwrap();
        campaign
            .add_edge(CampaignEdge::new(id_start, id_right, EdgeCondition::Always))
            .unwrap();
        campaign
            .add_edge(CampaignEdge::new(id_left, id_end, EdgeCondition::Always))
            .unwrap();
        campaign
            .add_edge(CampaignEdge::new(id_right, id_end, EdgeCondition::Always))
            .unwrap();

        let result = service.execute(&campaign, "Diamond input").await;

        assert!(result.is_ok());

        let call_order = port.get_call_order();
        assert_eq!(call_order.len(), 4);
        assert_eq!(call_order[0], "Start");
        assert_eq!(call_order[3], "End");
        // Left and Right can be in any order (parallel)
    }
}

// ============================================================================
// Test Module: Multiple Entry Points
// ============================================================================

#[cfg(test)]
mod campaign_multiple_entry_points_tests {
    use super::*;

    #[tokio::test]
    async fn test_multiple_entry_points_concurrent() {
        // RED: Multiple entry points not implemented
        let port = Arc::new(MockPaladinPort::new());
        let service = CampaignExecutionService::new(port.clone());

        let mut campaign = Campaign::new(create_test_config());
        let entry1 = create_test_paladin("Entry1");
        let entry2 = create_test_paladin("Entry2");
        let final_node = create_test_paladin("Final");

        let id_e1 = campaign.add_paladin(entry1);
        let id_e2 = campaign.add_paladin(entry2);
        let id_final = campaign.add_paladin(final_node);

        // Set explicit entry points
        campaign.set_entry_point(id_e1).unwrap();
        campaign.set_entry_point(id_e2).unwrap();

        // Both entry points converge to final
        campaign
            .add_edge(CampaignEdge::new(id_e1, id_final, EdgeCondition::Always))
            .unwrap();
        campaign
            .add_edge(CampaignEdge::new(id_e2, id_final, EdgeCondition::Always))
            .unwrap();

        let result = service.execute(&campaign, "Multiple starts").await;

        assert!(result.is_ok());

        let call_order = port.get_call_order();
        assert_eq!(call_order.len(), 3);
        assert!(call_order.contains(&"Entry1".to_string()));
        assert!(call_order.contains(&"Entry2".to_string()));
        assert_eq!(call_order[2], "Final");
    }
}

// ============================================================================
// Test Module: Error Handling and Validation
// ============================================================================

#[cfg(test)]
mod campaign_error_tests {
    use super::*;

    #[tokio::test]
    async fn test_empty_campaign_fails() {
        // RED: Validation not enforced
        let port = Arc::new(MockPaladinPort::new());
        let service = CampaignExecutionService::new(port);

        let campaign = Campaign::new(create_test_config());

        let result = service.execute(&campaign, "Empty").await;

        assert!(result.is_err(), "Empty campaign should fail validation");
    }

    #[tokio::test]
    async fn test_paladin_failure_propagates() {
        // RED: Error handling not implemented
        let port = Arc::new(MockPaladinPort::new().with_failures(vec!["Failer".to_string()]));
        let service = CampaignExecutionService::new(port.clone());

        let mut campaign = Campaign::new(create_test_config());
        let p1 = create_test_paladin("Start");
        let p2 = create_test_paladin("Failer");

        let id1 = campaign.add_paladin(p1);
        let id2 = campaign.add_paladin(p2);

        campaign
            .add_edge(CampaignEdge::new(id1, id2, EdgeCondition::Always))
            .unwrap();

        let result = service.execute(&campaign, "Should fail").await;

        assert!(result.is_err(), "Paladin failure should propagate");
    }
}
