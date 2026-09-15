//! Integration tests for Campaign execution service.
//!
//! Tests graph-based DAG orchestration including linear workflows, branching,
//! fan-out/fan-in, conditional routing, and error handling.

use async_trait::async_trait;
use paladin::application::services::battalion::campaign_service::CampaignExecutionService;
use paladin::core::base::entity::node::Node;
use paladin::core::platform::container::battalion::campaign::{
    Campaign, CampaignEdge, EdgeCondition,
};
use paladin::core::platform::container::battalion::{BattalionConfig, BattalionError};
use paladin::core::platform::container::paladin::MaxLoops;
use paladin::core::platform::container::paladin::{Paladin, PaladinData, PaladinStatus};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream, StopReason};
use std::sync::{Arc, Mutex};

/// Mock Paladin port for testing
#[derive(Clone)]
struct MockPaladinPort {
    /// Track execution order
    execution_log: Arc<Mutex<Vec<String>>>,
    /// Simulate paladin outputs
    outputs: Arc<Mutex<std::collections::HashMap<String, String>>>,
}

impl MockPaladinPort {
    fn new() -> Self {
        Self {
            execution_log: Arc::new(Mutex::new(Vec::new())),
            outputs: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    fn set_output(&self, paladin_name: &str, output: &str) {
        self.outputs
            .lock()
            .unwrap()
            .insert(paladin_name.to_string(), output.to_string());
    }

    fn get_execution_log(&self) -> Vec<String> {
        self.execution_log.lock().unwrap().clone()
    }
}

#[async_trait]
impl PaladinPort for MockPaladinPort {
    async fn execute(
        &self,
        paladin: &Paladin,
        input: &str,
    ) -> Result<PaladinResult, paladin::application::services::paladin::error::PaladinError> {
        // Log execution
        self.execution_log
            .lock()
            .unwrap()
            .push(paladin.node.name.clone());

        // Get simulated output
        let output = self
            .outputs
            .lock()
            .unwrap()
            .get(&paladin.node.name)
            .cloned()
            .unwrap_or_else(|| format!("{} processed: {}", paladin.node.name, input));

        Ok(PaladinResult {
            output,
            usage: paladin_ports::output::llm_port::TokenUsage::new(50, 0),
            execution_time_ms: 10,
            loop_count: 1,
            stop_reason: StopReason::Completed,
            ..Default::default()
        })
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinStream, paladin::application::services::paladin::error::PaladinError> {
        unimplemented!("Streaming not needed for tests")
    }

    fn validate(
        &self,
        _paladin: &Paladin,
    ) -> Result<(), paladin::application::services::paladin::error::PaladinError> {
        Ok(())
    }
}

/// Helper to create a test paladin
fn create_test_paladin(name: &str) -> Paladin {
    let data = PaladinData {
        system_prompt: format!("You are {}", name),
        name: name.to_string(),
        user_name: "test_user".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: Vec::new(),
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };

    Node::new(data, Some(name.to_string()))
}

#[tokio::test]
async fn test_campaign_service_creation() {
    let mock_port = Arc::new(MockPaladinPort::new());
    let _service = CampaignExecutionService::new(mock_port);
    // Should create without error
}

#[tokio::test]
async fn test_linear_campaign_execution() {
    let mock_port = Arc::new(MockPaladinPort::new());
    let service = CampaignExecutionService::new(mock_port.clone());

    // Create campaign: A → B → C
    let config = BattalionConfig::new("linear_campaign");
    let mut campaign = Campaign::new(config);

    let paladin_a = create_test_paladin("paladin_a");
    let paladin_b = create_test_paladin("paladin_b");
    let paladin_c = create_test_paladin("paladin_c");

    let id_a = campaign.add_paladin(paladin_a);
    let id_b = campaign.add_paladin(paladin_b);
    let id_c = campaign.add_paladin(paladin_c);

    campaign
        .add_edge(CampaignEdge::new(id_a, id_b, EdgeCondition::Always))
        .unwrap();
    campaign
        .add_edge(CampaignEdge::new(id_b, id_c, EdgeCondition::Always))
        .unwrap();

    campaign.set_entry_point(id_a).unwrap();

    // Execute
    let result = service.execute(&campaign, "initial input").await;

    assert!(result.is_ok());
    let execution_log = mock_port.get_execution_log();
    assert_eq!(execution_log, vec!["paladin_a", "paladin_b", "paladin_c"]);
}

#[tokio::test]
async fn test_branching_campaign_fan_out() {
    let mock_port = Arc::new(MockPaladinPort::new());
    let service = CampaignExecutionService::new(mock_port.clone());

    // Create campaign: A → [B, C]  (fan-out)
    let config = BattalionConfig::new("fan_out_campaign");
    let mut campaign = Campaign::new(config);

    let paladin_a = create_test_paladin("paladin_a");
    let paladin_b = create_test_paladin("paladin_b");
    let paladin_c = create_test_paladin("paladin_c");

    let id_a = campaign.add_paladin(paladin_a);
    let id_b = campaign.add_paladin(paladin_b);
    let id_c = campaign.add_paladin(paladin_c);

    campaign
        .add_edge(CampaignEdge::new(id_a, id_b, EdgeCondition::Always))
        .unwrap();
    campaign
        .add_edge(CampaignEdge::new(id_a, id_c, EdgeCondition::Always))
        .unwrap();

    campaign.set_entry_point(id_a).unwrap();

    // Execute
    let result = service.execute(&campaign, "initial input").await;

    assert!(result.is_ok());
    let execution_log = mock_port.get_execution_log();

    // A should execute first, then B and C in any order
    assert_eq!(execution_log[0], "paladin_a");
    assert!(execution_log.contains(&"paladin_b".to_string()));
    assert!(execution_log.contains(&"paladin_c".to_string()));
    assert_eq!(execution_log.len(), 3);
}

#[tokio::test]
async fn test_diamond_graph_campaign() {
    let mock_port = Arc::new(MockPaladinPort::new());
    let service = CampaignExecutionService::new(mock_port.clone());

    // Create campaign: A → [B, C] → D  (diamond shape)
    let config = BattalionConfig::new("diamond_campaign");
    let mut campaign = Campaign::new(config);

    let paladin_a = create_test_paladin("paladin_a");
    let paladin_b = create_test_paladin("paladin_b");
    let paladin_c = create_test_paladin("paladin_c");
    let paladin_d = create_test_paladin("paladin_d");

    let id_a = campaign.add_paladin(paladin_a);
    let id_b = campaign.add_paladin(paladin_b);
    let id_c = campaign.add_paladin(paladin_c);
    let id_d = campaign.add_paladin(paladin_d);

    campaign
        .add_edge(CampaignEdge::new(id_a, id_b, EdgeCondition::Always))
        .unwrap();
    campaign
        .add_edge(CampaignEdge::new(id_a, id_c, EdgeCondition::Always))
        .unwrap();
    campaign
        .add_edge(CampaignEdge::new(id_b, id_d, EdgeCondition::Always))
        .unwrap();
    campaign
        .add_edge(CampaignEdge::new(id_c, id_d, EdgeCondition::Always))
        .unwrap();

    campaign.set_entry_point(id_a).unwrap();

    // Execute
    let result = service.execute(&campaign, "initial input").await;

    assert!(result.is_ok());
    let execution_log = mock_port.get_execution_log();

    // Verify execution order: A first, then B and C, then D last
    assert_eq!(execution_log[0], "paladin_a");
    assert_eq!(execution_log[3], "paladin_d");
    assert!(execution_log.contains(&"paladin_b".to_string()));
    assert!(execution_log.contains(&"paladin_c".to_string()));
    assert_eq!(execution_log.len(), 4);
}

#[tokio::test]
async fn test_empty_campaign_validation() {
    let mock_port = Arc::new(MockPaladinPort::new());
    let service = CampaignExecutionService::new(mock_port);

    let config = BattalionConfig::new("empty_campaign");
    let campaign = Campaign::new(config);

    // Execute empty campaign should fail validation
    let result = service.execute(&campaign, "input").await;

    assert!(result.is_err());
    // Check for any graph validation error (actual message may vary)
    if let Err(err) = result {
        // Just verify it's a Battalion error - don't check exact message
        assert!(matches!(
            err,
            BattalionError::InvalidGraph(_) | BattalionError::ConfigurationError(_)
        ));
    }
}

#[tokio::test]
async fn test_single_paladin_campaign() {
    let mock_port = Arc::new(MockPaladinPort::new());
    let service = CampaignExecutionService::new(mock_port.clone());

    let config = BattalionConfig::new("single_campaign");
    let mut campaign = Campaign::new(config);

    let paladin = create_test_paladin("solo_paladin");
    let id = campaign.add_paladin(paladin);
    campaign.set_entry_point(id).unwrap();

    // Execute
    let result = service.execute(&campaign, "test input").await;

    assert!(result.is_ok());
    let execution_log = mock_port.get_execution_log();
    assert_eq!(execution_log, vec!["solo_paladin"]);
}

#[tokio::test]
async fn test_multiple_entry_points() {
    let mock_port = Arc::new(MockPaladinPort::new());
    let service = CampaignExecutionService::new(mock_port.clone());

    // Create campaign with two entry points: A and B (both → C)
    let config = BattalionConfig::new("multi_entry_campaign");
    let mut campaign = Campaign::new(config);

    let paladin_a = create_test_paladin("paladin_a");
    let paladin_b = create_test_paladin("paladin_b");
    let paladin_c = create_test_paladin("paladin_c");

    let id_a = campaign.add_paladin(paladin_a);
    let id_b = campaign.add_paladin(paladin_b);
    let id_c = campaign.add_paladin(paladin_c);

    campaign
        .add_edge(CampaignEdge::new(id_a, id_c, EdgeCondition::Always))
        .unwrap();
    campaign
        .add_edge(CampaignEdge::new(id_b, id_c, EdgeCondition::Always))
        .unwrap();

    campaign.set_entry_point(id_a).unwrap();
    campaign.set_entry_point(id_b).unwrap();

    // Execute
    let result = service.execute(&campaign, "initial input").await;

    assert!(result.is_ok());
    let execution_log = mock_port.get_execution_log();

    // Both entry points should execute, then C
    assert!(execution_log.contains(&"paladin_a".to_string()));
    assert!(execution_log.contains(&"paladin_b".to_string()));
    assert_eq!(execution_log[2], "paladin_c");
    assert_eq!(execution_log.len(), 3);
}

#[tokio::test]
async fn test_complex_workflow() {
    let mock_port = Arc::new(MockPaladinPort::new());
    let service = CampaignExecutionService::new(mock_port.clone());

    // Create complex workflow: A → B → [C, D] → E
    let config = BattalionConfig::new("complex_campaign");
    let mut campaign = Campaign::new(config);

    let paladins: Vec<_> = (b'A'..=b'E')
        .map(|c| create_test_paladin(&format!("paladin_{}", c as char)))
        .collect();

    let ids: Vec<_> = paladins
        .into_iter()
        .map(|p| campaign.add_paladin(p))
        .collect();

    // A → B
    campaign
        .add_edge(CampaignEdge::new(ids[0], ids[1], EdgeCondition::Always))
        .unwrap();
    // B → C
    campaign
        .add_edge(CampaignEdge::new(ids[1], ids[2], EdgeCondition::Always))
        .unwrap();
    // B → D
    campaign
        .add_edge(CampaignEdge::new(ids[1], ids[3], EdgeCondition::Always))
        .unwrap();
    // C → E
    campaign
        .add_edge(CampaignEdge::new(ids[2], ids[4], EdgeCondition::Always))
        .unwrap();
    // D → E
    campaign
        .add_edge(CampaignEdge::new(ids[3], ids[4], EdgeCondition::Always))
        .unwrap();

    campaign.set_entry_point(ids[0]).unwrap();

    // Execute
    let result = service.execute(&campaign, "initial input").await;

    assert!(result.is_ok());
    let execution_log = mock_port.get_execution_log();

    // Verify order: A, B, then C&D (parallel), then E
    assert_eq!(execution_log[0], "paladin_A");
    assert_eq!(execution_log[1], "paladin_B");
    assert_eq!(execution_log[4], "paladin_E");
    assert_eq!(execution_log.len(), 5);
}

#[tokio::test]
async fn test_campaign_with_edge_transform() {
    let mock_port = Arc::new(MockPaladinPort::new());
    mock_port.set_output("paladin_a", "result from A");

    let service = CampaignExecutionService::new(mock_port.clone());

    let config = BattalionConfig::new("transform_campaign");
    let mut campaign = Campaign::new(config);

    let paladin_a = create_test_paladin("paladin_a");
    let paladin_b = create_test_paladin("paladin_b");

    let id_a = campaign.add_paladin(paladin_a);
    let id_b = campaign.add_paladin(paladin_b);

    let edge = CampaignEdge::new(id_a, id_b, EdgeCondition::Always)
        .with_transform("Transform: {output}".to_string());

    campaign.add_edge(edge).unwrap();
    campaign.set_entry_point(id_a).unwrap();

    // Execute
    let result = service.execute(&campaign, "initial input").await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_campaign_execution_timeout() {
    let mock_port = Arc::new(MockPaladinPort::new());
    let service = CampaignExecutionService::new(mock_port);

    let mut config = BattalionConfig::new("timeout_campaign");
    config.timeout_seconds = 1; // 1 second timeout

    let mut campaign = Campaign::new(config);

    let paladin = create_test_paladin("slow_paladin");
    let id = campaign.add_paladin(paladin);
    campaign.set_entry_point(id).unwrap();

    // Note: This test passes because mock execution is fast
    // In real scenario with slow Paladin, timeout would trigger
    let result = service.execute(&campaign, "input").await;

    // Should complete successfully with fast mock
    assert!(result.is_ok());
}
