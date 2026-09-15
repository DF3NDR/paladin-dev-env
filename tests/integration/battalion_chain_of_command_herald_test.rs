//! D-15 composite witness for WARN-01: proves the Chain of Command result -> Herald flow
//! end to end.
//!
//! Drives a real `ChainOfCommandExecutionService::execute` over mock Paladins and formats the
//! resulting value through a real `JsonHerald`, following the discipline
//! `battalion_herald_end_to_end_test.rs` established for GAP-03: this file contains no
//! hand-built `BattalionResult` (or `DelegationResult`) literal -- every formatted value comes
//! out of `ChainOfCommandExecutionService::execute`.
//!
//! Note on scope: `JsonHerald::format_battalion_result`'s JSON shape for a Chain of Command
//! result does not include a `final_output` key, and (per D-14, reproducing the Commander's
//! former inline conversion exactly) `to_battalion_result` sets `paladin_results` to an empty
//! vector for this pattern. Neither of those is a Herald change -- `Herald` itself is
//! unchanged by this plan (D-14) -- so this test proves the specialist's real output text by
//! asserting directly on the `DelegationResult` `execute()` returns, and separately proves the
//! Herald-formatted `Some(String)` parses as JSON with the correct battalion identity.

use async_trait::async_trait;
use paladin::application::services::battalion::chain_of_command_service::ChainOfCommandExecutionService;
use paladin::application::services::paladin::error::PaladinError;
use paladin::core::base::entity::node::Node;
use paladin::core::platform::container::battalion::BattalionConfig;
use paladin::core::platform::container::battalion::chain_of_command::{
    ChainOfCommand, DelegationStrategy,
};
use paladin::core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus};
use paladin::infrastructure::adapters::herald::JsonHerald;
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream, StopReason};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Mock Paladin port for driving the Chain of Command, following
/// `battalion_chain_of_command_integration_test.rs`'s `MockPaladinPort` shape.
#[derive(Clone)]
struct HeraldMockPaladinPort {
    outputs: Arc<Mutex<HashMap<String, String>>>,
}

impl HeraldMockPaladinPort {
    fn new() -> Self {
        Self {
            outputs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn set_output(&self, paladin_name: &str, output: &str) {
        self.outputs
            .lock()
            .unwrap()
            .insert(paladin_name.to_string(), output.to_string());
    }
}

#[async_trait]
impl PaladinPort for HeraldMockPaladinPort {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
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
    ) -> Result<PaladinStream, PaladinError> {
        unimplemented!("Streaming not needed for this test")
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

/// Helper to create a test Paladin, matching
/// `battalion_chain_of_command_integration_test.rs`'s `create_test_paladin`.
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
async fn chain_of_command_result_renders_through_json_herald() {
    let mock_port = HeraldMockPaladinPort::new();
    mock_port.set_output(
        "Commander",
        "SELECT: Specialist\nREASON: Specialist can handle this",
    );
    let specialist_output = "Specialist completed the herald test task";
    mock_port.set_output("Specialist", specialist_output);

    let paladin_port: Arc<dyn PaladinPort> = Arc::new(mock_port);

    let service = ChainOfCommandExecutionService::new(Arc::clone(&paladin_port))
        .with_herald(Arc::new(JsonHerald::new()));

    let config = BattalionConfig::new("herald_chain");
    let commander = create_test_paladin("Commander");
    let specialist = create_test_paladin("Specialist");

    let chain = ChainOfCommand::new(commander, vec![specialist], config)
        .expect("chain construction should succeed")
        .with_strategy(DelegationStrategy::Automatic);

    let started_at = chrono::Utc::now();

    // Real execution -- no hand-built DelegationResult or BattalionResult anywhere in this
    // file.
    let delegation_result = service
        .execute(&chain, "delegate this task")
        .await
        .expect("execute should succeed");

    // Proves the specialist really ran through the mock port and its own output text is what
    // execute() produced, not a stand-in value.
    assert_eq!(
        delegation_result.outputs,
        vec![specialist_output.to_string()]
    );
    assert!(
        delegation_result
            .selected_specialists
            .contains(&"Specialist".to_string())
    );

    let formatted = service
        .format_result(&chain, &delegation_result, started_at)
        .expect("format_result should succeed")
        .expect("Herald is configured, formatted output should be Some");

    let parsed: serde_json::Value =
        serde_json::from_str(&formatted).expect("Herald output should be valid JSON");

    assert_eq!(parsed["battalion_name"], "herald_chain");
    assert_eq!(parsed["strategy_used"], "ChainOfCommand");
    assert_eq!(parsed["status"], "Completed");
}

#[tokio::test]
async fn chain_of_command_format_result_is_none_without_herald() {
    let mock_port = HeraldMockPaladinPort::new();
    mock_port.set_output(
        "Commander",
        "SELECT: Specialist\nREASON: Specialist can handle this",
    );
    mock_port.set_output("Specialist", "Specialist completed the task");

    let paladin_port: Arc<dyn PaladinPort> = Arc::new(mock_port);

    // No `.with_herald(...)` -- format_result must return Ok(None).
    let service = ChainOfCommandExecutionService::new(Arc::clone(&paladin_port));

    let config = BattalionConfig::new("no_herald_chain");
    let commander = create_test_paladin("Commander");
    let specialist = create_test_paladin("Specialist");

    let chain = ChainOfCommand::new(commander, vec![specialist], config)
        .expect("chain construction should succeed")
        .with_strategy(DelegationStrategy::Automatic);

    let started_at = chrono::Utc::now();

    let delegation_result = service
        .execute(&chain, "delegate this task")
        .await
        .expect("execute should succeed");

    let formatted = service
        .format_result(&chain, &delegation_result, started_at)
        .expect("format_result should succeed without a Herald configured");

    assert_eq!(formatted, None);
}
