//! Campaign Integration Tests
//!
//! End-to-end integration tests for Campaign pattern with real graph execution scenarios.

use async_trait::async_trait;
use paladin::application::services::battalion::campaign_service::CampaignExecutionService;
use paladin::application::services::paladin::error::PaladinError;
use paladin::core::base::entity::node::Node;
use paladin::core::platform::container::battalion::BattalionConfig;
use paladin::core::platform::container::battalion::campaign::{
    Campaign, CampaignEdge, EdgeCondition,
};
use paladin::core::platform::container::paladin::MaxLoops;
use paladin::core::platform::container::paladin::{Paladin, PaladinData, PaladinStatus};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, StopReason};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ============================================================================
// Mock PaladinPort for Integration Testing
// ============================================================================

#[derive(Clone)]
struct IntegrationMockPort {
    outputs: Arc<Mutex<HashMap<String, String>>>,
    call_order: Arc<Mutex<Vec<String>>>,
}

impl IntegrationMockPort {
    fn new() -> Self {
        Self {
            outputs: Arc::new(Mutex::new(HashMap::new())),
            call_order: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn with_outputs(outputs: HashMap<String, String>) -> Self {
        Self {
            outputs: Arc::new(Mutex::new(outputs)),
            call_order: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn get_call_order(&self) -> Vec<String> {
        self.call_order.lock().unwrap().clone()
    }
}

#[async_trait]
impl PaladinPort for IntegrationMockPort {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        let name = paladin.node.name.clone();

        // Record execution order
        {
            let mut order = self.call_order.lock().unwrap();
            order.push(name.clone());
        }

        // Get predetermined output or generate one
        let output = {
            let outputs = self.outputs.lock().unwrap();
            outputs
                .get(&name)
                .cloned()
                .unwrap_or_else(|| format!("Output from {} processing: {}", name, input))
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
        unimplemented!("Streaming not used in Campaign integration tests")
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn create_paladin(name: &str) -> Paladin {
    let data = PaladinData {
        system_prompt: format!("You are {}", name),
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

fn create_config() -> BattalionConfig {
    BattalionConfig::new("integration_campaign")
        .with_description("Integration Test Campaign")
        .with_timeout(60)
}

// ============================================================================
// Integration Test: Linear Graph (Simple Chain)
// ============================================================================

#[tokio::test]
async fn test_linear_graph_chain_execution() {
    // Create a simple linear chain: A → B → C → D
    let port = Arc::new(IntegrationMockPort::new());
    let service = CampaignExecutionService::new(port.clone());

    let mut campaign = Campaign::new(create_config());

    let analyst = create_paladin("DataAnalyst");
    let processor = create_paladin("DataProcessor");
    let validator = create_paladin("DataValidator");
    let reporter = create_paladin("ReportGenerator");

    let id_a = campaign.add_paladin(analyst);
    let id_b = campaign.add_paladin(processor);
    let id_c = campaign.add_paladin(validator);
    let id_d = campaign.add_paladin(reporter);

    // Create linear chain
    campaign
        .add_edge(CampaignEdge::new(id_a, id_b, EdgeCondition::Always))
        .unwrap();
    campaign
        .add_edge(CampaignEdge::new(id_b, id_c, EdgeCondition::Always))
        .unwrap();
    campaign
        .add_edge(CampaignEdge::new(id_c, id_d, EdgeCondition::Always))
        .unwrap();

    let result = service.execute(&campaign, "Analyze dataset XYZ").await;

    assert!(result.is_ok(), "Linear chain should execute successfully");
    let result = result.unwrap();

    // Verify execution order
    let call_order = port.get_call_order();
    assert_eq!(
        call_order,
        vec![
            "DataAnalyst",
            "DataProcessor",
            "DataValidator",
            "ReportGenerator"
        ],
        "Paladins should execute in topological order"
    );

    // Verify result metadata
    assert_eq!(result.paladin_results.len(), 4);
    assert!(result.final_output.contains("ReportGenerator"));
}

// ============================================================================
// Integration Test: Branching Graph with Conditional Routing
// ============================================================================

#[tokio::test]
async fn test_branching_graph_conditional_routing() {
    // Create branching workflow with conditional routing
    let mut outputs = HashMap::new();
    outputs.insert(
        "QualityChecker".to_string(),
        "QUALITY_CHECK: PASSED".to_string(),
    );
    outputs.insert(
        "SuccessPath".to_string(),
        "Processing success path".to_string(),
    );
    outputs.insert(
        "FailurePath".to_string(),
        "Processing failure path".to_string(),
    );

    let port = Arc::new(IntegrationMockPort::with_outputs(outputs));
    let service = CampaignExecutionService::new(port.clone());

    let mut campaign = Campaign::new(create_config());

    let checker = create_paladin("QualityChecker");
    let success_handler = create_paladin("SuccessPath");
    let failure_handler = create_paladin("FailurePath");

    let id_checker = campaign.add_paladin(checker);
    let id_success = campaign.add_paladin(success_handler);
    let id_failure = campaign.add_paladin(failure_handler);

    // Conditional branching based on output
    campaign
        .add_edge(CampaignEdge::new(
            id_checker,
            id_success,
            EdgeCondition::Contains("PASSED".to_string()),
        ))
        .unwrap();

    campaign
        .add_edge(CampaignEdge::new(
            id_checker,
            id_failure,
            EdgeCondition::Contains("FAILED".to_string()),
        ))
        .unwrap();

    let result = service.execute(&campaign, "Check quality").await;

    assert!(result.is_ok(), "Conditional routing should work");

    let call_order = port.get_call_order();
    assert_eq!(call_order.len(), 2, "Should execute checker and one branch");
    assert_eq!(call_order[0], "QualityChecker");
    assert_eq!(
        call_order[1], "SuccessPath",
        "Should route to success path when PASSED"
    );
}

#[tokio::test]
async fn test_branching_graph_failure_path() {
    // Test routing to failure path
    let mut outputs = HashMap::new();
    outputs.insert(
        "QualityChecker".to_string(),
        "QUALITY_CHECK: FAILED".to_string(),
    );
    outputs.insert(
        "FailurePath".to_string(),
        "Processing failure path".to_string(),
    );

    let port = Arc::new(IntegrationMockPort::with_outputs(outputs));
    let service = CampaignExecutionService::new(port.clone());

    let mut campaign = Campaign::new(create_config());

    let checker = create_paladin("QualityChecker");
    let success_handler = create_paladin("SuccessPath");
    let failure_handler = create_paladin("FailurePath");

    let id_checker = campaign.add_paladin(checker);
    let id_success = campaign.add_paladin(success_handler);
    let id_failure = campaign.add_paladin(failure_handler);

    campaign
        .add_edge(CampaignEdge::new(
            id_checker,
            id_success,
            EdgeCondition::Contains("PASSED".to_string()),
        ))
        .unwrap();

    campaign
        .add_edge(CampaignEdge::new(
            id_checker,
            id_failure,
            EdgeCondition::Contains("FAILED".to_string()),
        ))
        .unwrap();

    let result = service.execute(&campaign, "Check quality").await;

    assert!(result.is_ok(), "Failure path routing should work");

    let call_order = port.get_call_order();
    assert_eq!(
        call_order[1], "FailurePath",
        "Should route to failure path when FAILED"
    );
}

// ============================================================================
// Integration Test: Complex DAG with Fan-Out/Fan-In
// ============================================================================

#[tokio::test]
async fn test_complex_dag_with_fan_out_fan_in() {
    // Create a diamond pattern with fan-out and fan-in
    // Start → [ParallelA, ParallelB, ParallelC] → Aggregator → FinalStep
    let port = Arc::new(IntegrationMockPort::new());
    let service = CampaignExecutionService::new(port.clone());

    let mut campaign = Campaign::new(create_config());

    let start = create_paladin("Coordinator");
    let parallel_a = create_paladin("ProcessorA");
    let parallel_b = create_paladin("ProcessorB");
    let parallel_c = create_paladin("ProcessorC");
    let aggregator = create_paladin("ResultAggregator");
    let final_step = create_paladin("FinalReporter");

    let id_start = campaign.add_paladin(start);
    let id_a = campaign.add_paladin(parallel_a);
    let id_b = campaign.add_paladin(parallel_b);
    let id_c = campaign.add_paladin(parallel_c);
    let id_agg = campaign.add_paladin(aggregator);
    let id_final = campaign.add_paladin(final_step);

    // Fan-out: Start → [A, B, C]
    campaign
        .add_edge(CampaignEdge::new(id_start, id_a, EdgeCondition::Always))
        .unwrap();
    campaign
        .add_edge(CampaignEdge::new(id_start, id_b, EdgeCondition::Always))
        .unwrap();
    campaign
        .add_edge(CampaignEdge::new(id_start, id_c, EdgeCondition::Always))
        .unwrap();

    // Fan-in: [A, B, C] → Aggregator
    campaign
        .add_edge(CampaignEdge::new(id_a, id_agg, EdgeCondition::Always))
        .unwrap();
    campaign
        .add_edge(CampaignEdge::new(id_b, id_agg, EdgeCondition::Always))
        .unwrap();
    campaign
        .add_edge(CampaignEdge::new(id_c, id_agg, EdgeCondition::Always))
        .unwrap();

    // Final step
    campaign
        .add_edge(CampaignEdge::new(id_agg, id_final, EdgeCondition::Always))
        .unwrap();

    let result = service.execute(&campaign, "Process complex workflow").await;

    assert!(result.is_ok(), "Complex DAG should execute successfully");
    let _result = result.unwrap();

    let call_order = port.get_call_order();

    // Verify all Paladins executed
    assert_eq!(call_order.len(), 6, "All 6 Paladins should execute");

    // Verify ordering constraints
    assert_eq!(call_order[0], "Coordinator", "Coordinator should be first");

    // Parallel processors (A, B, C) should come after Coordinator but before Aggregator
    let coord_index = 0;
    let agg_index = call_order
        .iter()
        .position(|n| n == "ResultAggregator")
        .unwrap();

    assert!(call_order[coord_index + 1..agg_index].contains(&"ProcessorA".to_string()));
    assert!(call_order[coord_index + 1..agg_index].contains(&"ProcessorB".to_string()));
    assert!(call_order[coord_index + 1..agg_index].contains(&"ProcessorC".to_string()));

    // Aggregator should come before FinalReporter
    let final_index = call_order
        .iter()
        .position(|n| n == "FinalReporter")
        .unwrap();
    assert!(
        agg_index < final_index,
        "Aggregator should complete before FinalReporter"
    );

    assert_eq!(
        call_order.last().unwrap(),
        "FinalReporter",
        "FinalReporter should be last"
    );
}

#[tokio::test]
async fn test_multiple_independent_branches() {
    // Create a graph with multiple independent branches from a single source
    let port = Arc::new(IntegrationMockPort::new());
    let service = CampaignExecutionService::new(port.clone());

    let mut campaign = Campaign::new(create_config());

    let source = create_paladin("DataSource");
    let branch1_a = create_paladin("Branch1A");
    let branch1_b = create_paladin("Branch1B");
    let branch2_a = create_paladin("Branch2A");
    let branch2_b = create_paladin("Branch2B");

    let id_src = campaign.add_paladin(source);
    let id_1a = campaign.add_paladin(branch1_a);
    let id_1b = campaign.add_paladin(branch1_b);
    let id_2a = campaign.add_paladin(branch2_a);
    let id_2b = campaign.add_paladin(branch2_b);

    // Create two independent branches
    // Branch 1: Source → 1A → 1B
    campaign
        .add_edge(CampaignEdge::new(id_src, id_1a, EdgeCondition::Always))
        .unwrap();
    campaign
        .add_edge(CampaignEdge::new(id_1a, id_1b, EdgeCondition::Always))
        .unwrap();

    // Branch 2: Source → 2A → 2B
    campaign
        .add_edge(CampaignEdge::new(id_src, id_2a, EdgeCondition::Always))
        .unwrap();
    campaign
        .add_edge(CampaignEdge::new(id_2a, id_2b, EdgeCondition::Always))
        .unwrap();

    let result = service.execute(&campaign, "Multi-branch processing").await;

    assert!(result.is_ok(), "Multiple branches should execute");

    let call_order = port.get_call_order();
    assert_eq!(call_order.len(), 5, "All 5 Paladins should execute");
    assert_eq!(call_order[0], "DataSource", "Source should be first");

    // Verify branch ordering
    let idx_1a = call_order.iter().position(|n| n == "Branch1A").unwrap();
    let idx_1b = call_order.iter().position(|n| n == "Branch1B").unwrap();
    let idx_2a = call_order.iter().position(|n| n == "Branch2A").unwrap();
    let idx_2b = call_order.iter().position(|n| n == "Branch2B").unwrap();

    assert!(idx_1a < idx_1b, "Branch1A should execute before Branch1B");
    assert!(idx_2a < idx_2b, "Branch2A should execute before Branch2B");
}

// ============================================================================
// Integration Test: Cycle Detection Validation
// ============================================================================

#[tokio::test]
async fn test_cycle_detection_prevents_execution() {
    // Create a graph with a cycle: A → B → C → A
    let port = Arc::new(IntegrationMockPort::new());
    let service = CampaignExecutionService::new(port.clone());

    let mut campaign = Campaign::new(create_config());

    let node_a = create_paladin("NodeA");
    let node_b = create_paladin("NodeB");
    let node_c = create_paladin("NodeC");

    let id_a = campaign.add_paladin(node_a);
    let id_b = campaign.add_paladin(node_b);
    let id_c = campaign.add_paladin(node_c);

    // Create cycle: A → B → C → A
    campaign
        .add_edge(CampaignEdge::new(id_a, id_b, EdgeCondition::Always))
        .unwrap();
    campaign
        .add_edge(CampaignEdge::new(id_b, id_c, EdgeCondition::Always))
        .unwrap();
    campaign
        .add_edge(CampaignEdge::new(id_c, id_a, EdgeCondition::Always))
        .unwrap();

    // Execution should fail due to cycle detection
    let result = service.execute(&campaign, "Should detect cycle").await;

    assert!(
        result.is_err(),
        "Campaign with cycle should fail validation"
    );

    let error_msg = format!("{:?}", result.unwrap_err());
    assert!(
        error_msg.contains("Cycle") || error_msg.contains("cycle"),
        "Error should mention cycle detection"
    );

    // Verify no Paladins were executed
    let call_order = port.get_call_order();
    assert_eq!(
        call_order.len(),
        0,
        "No Paladins should execute when cycle detected"
    );
}

#[tokio::test]
async fn test_self_loop_detection() {
    // Create a self-loop: A → A
    let port = Arc::new(IntegrationMockPort::new());
    let service = CampaignExecutionService::new(port);

    let mut campaign = Campaign::new(create_config());

    let node_a = create_paladin("NodeA");
    let id_a = campaign.add_paladin(node_a);

    // Create self-loop
    campaign
        .add_edge(CampaignEdge::new(id_a, id_a, EdgeCondition::Always))
        .unwrap();

    let result = service.execute(&campaign, "Should detect self-loop").await;

    assert!(result.is_err(), "Self-loop should be detected as invalid");
}

// ============================================================================
// Integration Test: Regex Condition Routing
// ============================================================================

#[tokio::test]
async fn test_regex_pattern_routing() {
    // Test regex-based conditional routing
    let mut outputs = HashMap::new();
    outputs.insert(
        "LogParser".to_string(),
        "Error: Code E404 - Resource not found".to_string(),
    );
    outputs.insert("ErrorHandler".to_string(), "Handled E404 error".to_string());

    let port = Arc::new(IntegrationMockPort::with_outputs(outputs));
    let service = CampaignExecutionService::new(port.clone());

    let mut campaign = Campaign::new(create_config());

    let parser = create_paladin("LogParser");
    let error_handler = create_paladin("ErrorHandler");
    let normal_handler = create_paladin("NormalHandler");

    let id_parser = campaign.add_paladin(parser);
    let id_error = campaign.add_paladin(error_handler);
    let id_normal = campaign.add_paladin(normal_handler);

    // Route to error handler if output matches error pattern
    campaign
        .add_edge(CampaignEdge::new(
            id_parser,
            id_error,
            EdgeCondition::Regex(r"Error.*E\d+".to_string()),
        ))
        .unwrap();

    // Route to normal handler if no error
    campaign
        .add_edge(CampaignEdge::new(
            id_parser,
            id_normal,
            EdgeCondition::Contains("Success".to_string()),
        ))
        .unwrap();

    let result = service.execute(&campaign, "Parse logs").await;

    assert!(result.is_ok(), "Regex routing should work");

    let call_order = port.get_call_order();
    assert_eq!(call_order.len(), 2);
    assert_eq!(call_order[0], "LogParser");
    assert_eq!(
        call_order[1], "ErrorHandler",
        "Should route to error handler via regex match"
    );
}

// ============================================================================
// Integration Test: Empty Campaign Validation
// ============================================================================

#[tokio::test]
async fn test_empty_campaign_validation() {
    let port = Arc::new(IntegrationMockPort::new());
    let service = CampaignExecutionService::new(port);

    let campaign = Campaign::new(create_config());

    let result = service.execute(&campaign, "Empty campaign").await;

    assert!(result.is_err(), "Empty campaign should fail validation");
}
