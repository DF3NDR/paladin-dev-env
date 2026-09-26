//! End-to-end proof for GAP-03 / Epic 8 task 7.13.
//!
//! Drives a real `FormationExecutionService` over mock Paladins and formats the resulting
//! `BattalionResult` through all three Heralds (`JsonHerald`, `MarkdownHerald`, `TableHerald`),
//! asserting the five content requirements ROADMAP success criterion 3 names: Battalion identity
//! (name, id, strategy), per-Paladin results in execution order, aggregated token usage derived
//! from the mocks' own counts, partial results on failure, and all three Heralds exercised.
//!
//! `TableHerald` requires the `cli` feature (ADR-0023); this whole file's tests exercise all
//! three Heralds together, so the entire module is `#![cfg(feature = "cli")]`-gated and does not
//! run in a default-features build (two `#[test]` functions skipped, per ADR-0023).
//!
//! Task 7.13's own inline note names the missing piece as "needs Battalion execution setup" — a
//! hand-built `BattalionResult` literal would close the criterion on paper without proving the
//! producer side. This file contains no such literal; every result comes out of
//! `FormationExecutionService::execute`.

#![cfg(feature = "cli")]

use async_trait::async_trait;
use chrono::Utc;
use paladin::application::services::battalion::formation_service::FormationExecutionService;
use paladin::application::services::paladin::error::PaladinError;
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::core::platform::container::battalion::formation::Formation;
use paladin::core::platform::container::battalion::{BattalionConfig, ErrorStrategy};
use paladin::core::platform::container::herald::Herald;
use paladin::core::platform::container::paladin::Paladin;
use paladin::infrastructure::adapters::herald::{JsonHerald, MarkdownHerald, TableHerald};
use paladin_ports::output::llm_port::{
    FinishReason, LlmError, LlmPort, LlmRequest, LlmResponse, TokenUsage as LlmTokenUsage,
};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream, StopReason};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Mock LLM Port used only to satisfy `PaladinBuilder::new`'s constructor requirement.
///
/// `FormationExecutionService` drives Paladins through a `PaladinPort` mock
/// (`FormationMockPaladinPort` below), not through the LLM, so this port's `generate` output is
/// never read by the assertions in this file.
struct MockLlmPort;

#[async_trait]
impl LlmPort for MockLlmPort {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        Ok(LlmResponse {
            id: Uuid::new_v4(),
            request_id: request.id,
            model: request.model,
            content: "unused".to_string(),
            finish_reason: FinishReason::Stop,
            usage: LlmTokenUsage::new(0, 0),
            cost: None,
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: None,
        })
    }

    async fn generate_stream(
        &self,
        _request: LlmRequest,
    ) -> Result<
        Box<
            dyn futures::Stream<
                    Item = Result<paladin_ports::output::llm_port::StreamingResponse, LlmError>,
                > + Send,
        >,
        LlmError,
    > {
        unimplemented!("Streaming not needed for this test")
    }

    async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
        Ok(true)
    }

    async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
        Ok(vec!["mock-model".to_string()])
    }

    fn get_provider_name(&self) -> &'static str {
        "mock"
    }

    fn get_capabilities(&self) -> paladin_ports::output::llm_port::ProviderCapabilities {
        paladin_ports::output::llm_port::ProviderCapabilities::default()
    }
}

/// Mock `PaladinPort` for driving `FormationExecutionService`.
///
/// Configured per-name with a distinct, non-round `(output, token_count, execution_time_ms)`
/// triple so every downstream assertion can only pass if the rendering read the real values.
/// The failure-injection mechanism follows `IntegrationMockPaladinPort`'s
/// (`commander_integration_tests.rs`) configurable-failure pattern rather than inventing a new
/// mock shape.
#[derive(Clone)]
struct FormationMockPaladinPort {
    responses: HashMap<String, (String, u32, u64)>,
    fail_names: Arc<Mutex<HashSet<String>>>,
    execution_log: Arc<Mutex<Vec<String>>>,
}

impl FormationMockPaladinPort {
    fn new(responses: HashMap<String, (String, u32, u64)>) -> Self {
        Self {
            responses,
            fail_names: Arc::new(Mutex::new(HashSet::new())),
            execution_log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn with_failures(self, names: Vec<String>) -> Self {
        *self.fail_names.lock().unwrap() = names.into_iter().collect();
        self
    }

    #[allow(dead_code)]
    fn get_execution_log(&self) -> Vec<String> {
        self.execution_log.lock().unwrap().clone()
    }
}

#[async_trait]
impl PaladinPort for FormationMockPaladinPort {
    async fn execute(
        &self,
        paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinResult, PaladinError> {
        self.execution_log
            .lock()
            .unwrap()
            .push(paladin.node.name.clone());

        if self.fail_names.lock().unwrap().contains(&paladin.node.name) {
            return Err(PaladinError::ExecutionError(format!(
                "Simulated failure for {}",
                paladin.node.name
            )));
        }

        let (output, token_count, execution_time_ms) = self
            .responses
            .get(&paladin.node.name)
            .cloned()
            .unwrap_or_else(|| panic!("No mock response configured for {}", paladin.node.name));

        Ok(PaladinResult {
            output,
            usage: paladin_ports::output::llm_port::TokenUsage::new(token_count, 0),
            execution_time_ms,
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
        let (_tx, rx) = tokio::sync::mpsc::channel(1);
        Ok(rx)
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

/// Builds a Paladin via `PaladinBuilder`, matching the setup shape of
/// `herald_integration_test.rs`'s `test_battalion_formation_with_herald`.
async fn build_paladin(llm_port: &Arc<dyn LlmPort>, name: &str) -> Paladin {
    PaladinBuilder::new(Arc::clone(llm_port))
        .system_prompt(format!("You are {}", name))
        .name(name)
        .build()
        .await
        .unwrap_or_else(|_| panic!("Failed to build Paladin {}", name))
}

#[tokio::test]
async fn test_formation_result_through_json_markdown_table_heralds() {
    let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmPort);

    let paladin1 = build_paladin(&llm_port, "Scoutmaster").await;
    let paladin2 = build_paladin(&llm_port, "Sentinel").await;
    let paladin3 = build_paladin(&llm_port, "Vanguard").await;

    let mut responses: HashMap<String, (String, u32, u64)> = HashMap::new();
    responses.insert(
        "Scoutmaster".to_string(),
        ("Scoutmaster completed the recon.".to_string(), 137, 1011),
    );
    responses.insert(
        "Sentinel".to_string(),
        ("Sentinel confirmed the perimeter.".to_string(), 263, 1148),
    );
    responses.insert(
        "Vanguard".to_string(),
        ("Vanguard led the advance.".to_string(), 401, 1285),
    );

    let paladin_port: Arc<dyn PaladinPort> =
        Arc::new(FormationMockPaladinPort::new(responses.clone()));

    let formation = Formation::new(
        vec![paladin1, paladin2, paladin3],
        BattalionConfig::new("recon_formation"),
    )
    .expect("Failed to create Formation");

    let service = FormationExecutionService::new(Arc::clone(&paladin_port));

    let result = service.execute(&formation, "Begin the mission").await;
    assert!(result.is_ok(), "Formation execution should succeed");
    let result = result.unwrap();

    // Derived from the mocks' own token counts — changing one mock's count changes this value.
    let expected_total: u64 = responses
        .values()
        .map(|(_, tokens, _)| u64::from(*tokens))
        .sum();
    assert_eq!(result.total_tokens, expected_total);
    assert_eq!(result.paladin_results.len(), 3);

    // --- JSON Herald ---
    let json_herald = JsonHerald::new();
    let json_output = json_herald
        .format_battalion_result(&result)
        .expect("JSON formatting should succeed");
    let parsed: serde_json::Value =
        serde_json::from_str(&json_output).expect("JSON Herald output should be valid JSON");

    // 1. Battalion name, id and strategy.
    assert_eq!(parsed["battalion_name"], "recon_formation");
    assert_eq!(parsed["battalion_id"], result.battalion_id.to_string());
    assert_eq!(parsed["strategy_used"], "Formation");

    // 2. Per-Paladin results in execution order — positional, not containment.
    let per_paladin = parsed["paladin_results"]
        .as_array()
        .expect("paladin_results should be an array");
    assert_eq!(per_paladin.len(), 3);
    assert_eq!(per_paladin[0]["output"], responses["Scoutmaster"].0.clone());
    assert_eq!(per_paladin[1]["output"], responses["Sentinel"].0.clone());
    assert_eq!(per_paladin[2]["output"], responses["Vanguard"].0.clone());

    // 3. Aggregated token usage, derived from the mocks' own counts.
    assert_eq!(parsed["total_tokens"].as_u64().unwrap(), expected_total);

    // --- Markdown Herald ---
    let markdown_herald = MarkdownHerald::new();
    let markdown_output = markdown_herald
        .format_battalion_result(&result)
        .expect("Markdown formatting should succeed");

    assert!(markdown_output.contains("recon_formation"));
    assert!(markdown_output.contains("Formation"));
    assert!(markdown_output.contains(&expected_total.to_string()));
    assert!(markdown_output.contains("Scoutmaster"));
    assert!(markdown_output.contains("Sentinel"));
    assert!(markdown_output.contains("Vanguard"));

    // --- Table Herald ---
    let table_herald = TableHerald::default();
    let table_output = table_herald
        .format_battalion_result(&result)
        .expect("Table formatting should succeed");

    assert!(table_output.contains("Scoutmaster"));
    assert!(table_output.contains("Sentinel"));
    assert!(table_output.contains("Vanguard"));
    assert!(table_output.contains(&expected_total.to_string()));
}

#[tokio::test]
async fn test_formation_partial_results_through_all_three_heralds() {
    let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmPort);

    let paladin1 = build_paladin(&llm_port, "Outrider").await;
    let paladin2 = build_paladin(&llm_port, "Skirmisher").await;
    let paladin3 = build_paladin(&llm_port, "Bulwark").await;

    let mut responses: HashMap<String, (String, u32, u64)> = HashMap::new();
    responses.insert(
        "Outrider".to_string(),
        ("Outrider secured the flank.".to_string(), 173, 921),
    );
    // Skirmisher fails deterministically — no response entry is consulted for it.
    responses.insert(
        "Bulwark".to_string(),
        ("Bulwark held the line.".to_string(), 349, 1467),
    );

    let paladin_port: Arc<dyn PaladinPort> = Arc::new(
        FormationMockPaladinPort::new(responses.clone())
            .with_failures(vec!["Skirmisher".to_string()]),
    );

    // Continue-on-error is required: fail-fast returns Err and produces no partial result to
    // render, which is not what this criterion is about.
    let config = BattalionConfig::new("partial_formation")
        .with_error_strategy(ErrorStrategy::ContinueOnError);

    let formation = Formation::new(vec![paladin1, paladin2, paladin3], config)
        .expect("Failed to create Formation");

    let service = FormationExecutionService::new(Arc::clone(&paladin_port));

    let result = service
        .execute(&formation, "Hold the position")
        .await
        .expect("ContinueOnError should return a partial result, not an Err");

    assert_eq!(result.paladin_success_count, 2);
    assert_eq!(result.paladin_failure_count, 1);
    assert_eq!(result.node_errors.len(), 1);
    assert_eq!(result.node_errors[0].node_name, "Skirmisher");
    assert!(result.node_errors[0].error.contains("Skirmisher"));

    // --- JSON Herald ---
    let json_herald = JsonHerald::new();
    let json_output = json_herald
        .format_battalion_result(&result)
        .expect("JSON formatting should succeed");
    let parsed: serde_json::Value =
        serde_json::from_str(&json_output).expect("JSON Herald output should be valid JSON");

    let per_paladin = parsed["paladin_results"]
        .as_array()
        .expect("paladin_results should be an array");
    assert_eq!(per_paladin.len(), 2, "Only the two successes are recorded");
    let outputs: Vec<&str> = per_paladin
        .iter()
        .map(|p| p["output"].as_str().unwrap())
        .collect();
    assert!(outputs.contains(&responses["Outrider"].0.as_str()));
    assert!(outputs.contains(&responses["Bulwark"].0.as_str()));

    // Structured node-error array and its length, not a substring search.
    let node_errors_json = parsed["node_errors"]
        .as_array()
        .expect("node_errors should be an array");
    assert_eq!(node_errors_json.len(), 1);
    assert_eq!(node_errors_json[0]["node_name"], "Skirmisher");

    // --- Markdown Herald ---
    let markdown_herald = MarkdownHerald::new();
    let markdown_output = markdown_herald
        .format_battalion_result(&result)
        .expect("Markdown formatting should succeed");

    assert!(markdown_output.contains("Outrider"));
    assert!(markdown_output.contains("Bulwark"));
    assert!(markdown_output.contains("Skirmisher"));
    assert!(markdown_output.contains("**Success Count:** 2"));
    assert!(markdown_output.contains("**Failure Count:** 1"));

    // --- Table Herald ---
    let table_herald = TableHerald::default();
    let table_output = table_herald
        .format_battalion_result(&result)
        .expect("Table formatting should succeed");

    assert!(table_output.contains("Outrider"));
    assert!(table_output.contains("Bulwark"));
    assert!(table_output.contains("Skirmisher"));
    assert!(table_output.contains("Failures"));
}
