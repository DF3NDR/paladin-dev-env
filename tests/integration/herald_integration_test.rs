//! Integration tests for Herald output formatting with Paladin and Battalion execution
//!
//! These tests verify that Herald formatters correctly format real Paladin and Battalion
//! execution results with proper metadata inclusion.

use async_trait::async_trait;
use chrono::Utc;
use paladin::application::services::battalion::formation_service::FormationExecutionService;
use paladin::application::services::battalion::phalanx_service::PhalanxExecutionService;
use paladin::application::services::paladin::error::PaladinError;
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::core::platform::container::battalion::BattalionConfig;
use paladin::core::platform::container::battalion::formation::Formation;
use paladin::core::platform::container::battalion::phalanx::Phalanx;
use paladin::core::platform::container::herald::Herald;
use paladin::core::platform::container::paladin::Paladin;
#[cfg(feature = "cli")]
use paladin::infrastructure::adapters::herald::TableHerald;
use paladin::infrastructure::adapters::herald::{JsonHerald, MarkdownHerald};
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin_ports::output::llm_port::{
    FinishReason, LlmError, LlmPort, LlmRequest, LlmResponse, TokenUsage,
};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Mock LLM Port for testing
struct MockLlmPort {
    response_text: String,
}

impl MockLlmPort {
    fn new(response_text: impl Into<String>) -> Self {
        Self {
            response_text: response_text.into(),
        }
    }
}

#[async_trait]
impl LlmPort for MockLlmPort {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        Ok(LlmResponse {
            id: Uuid::new_v4(),
            request_id: request.id,
            model: request.model,
            content: self.response_text.clone(),
            finish_reason: FinishReason::Stop,
            usage: TokenUsage::new(50, 100),
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

#[tokio::test]
async fn test_paladin_with_json_herald() {
    // Create mock LLM port
    let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmPort::new("Test response from Paladin"));

    // Create circuit breaker
    let circuit_breaker = Arc::new(CircuitBreaker::new(
        5,
        3,
        std::time::Duration::from_secs(60),
    ));

    // Create Herald
    let herald: Arc<dyn Herald> = Arc::new(JsonHerald::new());

    // Create Paladin with Herald
    let paladin = PaladinBuilder::new(Arc::clone(&llm_port))
        .system_prompt("You are a helpful assistant")
        .name("Test Paladin")
        .build()
        .await
        .expect("Failed to build Paladin");

    // Create execution service with Herald
    let service = PaladinExecutionService::new(
        Arc::clone(&llm_port),
        circuit_breaker,
        None, // No garrison
        None, // No arsenal
    )
    .with_herald(Arc::clone(&herald));

    // Execute (this will use the mock LLM)
    let result = service.execute(&paladin, "Hello").await;
    assert!(result.is_ok(), "Paladin execution should succeed");

    let result = result.unwrap();

    // Format the result
    let formatted = service.format_result(&result, &paladin);
    assert!(formatted.is_ok(), "Herald formatting should succeed");

    let formatted = formatted.unwrap();
    assert!(formatted.is_some(), "Herald should format the result");

    let json_output = formatted.unwrap();

    // Verify it's valid JSON
    let parsed: serde_json::Value =
        serde_json::from_str(&json_output).expect("Output should be valid JSON");

    // Verify JSON contains expected fields from PaladinResult
    assert!(
        parsed.get("output").is_some(),
        "JSON should contain 'output' field"
    );
    assert!(
        parsed.get("token_count").is_some(),
        "JSON should contain 'token_count' field"
    );
    assert!(
        parsed.get("execution_time_ms").is_some(),
        "JSON should contain 'execution_time_ms' field"
    );
    assert!(
        parsed.get("loop_count").is_some(),
        "JSON should contain 'loop_count' field"
    );
    assert!(
        parsed.get("stop_reason").is_some(),
        "JSON should contain 'stop_reason' field"
    );

    println!("JSON Herald output:\n{}", json_output);
}

#[tokio::test]
async fn test_paladin_with_markdown_herald() {
    // Create mock LLM port
    let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmPort::new("Markdown test response"));

    // Create circuit breaker
    let circuit_breaker = Arc::new(CircuitBreaker::new(
        5,
        3,
        std::time::Duration::from_secs(60),
    ));

    // Create Herald
    let herald: Arc<dyn Herald> = Arc::new(MarkdownHerald::new());

    // Create Paladin
    let paladin = PaladinBuilder::new(Arc::clone(&llm_port))
        .system_prompt("You are a helpful assistant")
        .name("Markdown Paladin")
        .build()
        .await
        .expect("Failed to build Paladin");

    // Create execution service with Herald
    let service = PaladinExecutionService::new(Arc::clone(&llm_port), circuit_breaker, None, None)
        .with_herald(Arc::clone(&herald));

    // Execute
    let result = service.execute(&paladin, "Test").await;
    assert!(result.is_ok());

    let result = result.unwrap();

    // Format the result
    let formatted = service.format_result(&result, &paladin);
    assert!(formatted.is_ok());

    let formatted = formatted.unwrap();
    assert!(formatted.is_some());

    let markdown_output = formatted.unwrap();

    // Verify Markdown content structure
    assert!(
        markdown_output.contains("##") || markdown_output.contains("**"),
        "Markdown should contain heading markers (##) or bold markers (**)"
    );
    assert!(
        markdown_output.contains("Paladin Result"),
        "Markdown should contain 'Paladin Result' heading"
    );
    assert!(
        markdown_output.contains("Output"),
        "Markdown should contain 'Output' section"
    );
    assert!(
        markdown_output.contains("Metadata"),
        "Markdown should contain 'Metadata' section"
    );
    assert!(
        markdown_output.contains("Token Count"),
        "Markdown should contain 'Token Count' field"
    );

    println!("Markdown Herald output:\n{}", markdown_output);
}

#[tokio::test]
#[cfg(feature = "cli")]
async fn test_paladin_with_table_herald() {
    // Create mock LLM port
    let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmPort::new("Table test response"));

    // Create circuit breaker
    let circuit_breaker = Arc::new(CircuitBreaker::new(
        5,
        3,
        std::time::Duration::from_secs(60),
    ));

    // Create Herald
    let herald: Arc<dyn Herald> = Arc::new(TableHerald::default());

    // Create Paladin
    let paladin = PaladinBuilder::new(Arc::clone(&llm_port))
        .system_prompt("You are a helpful assistant")
        .name("Table Paladin")
        .build()
        .await
        .expect("Failed to build Paladin");

    // Create execution service with Herald
    let service = PaladinExecutionService::new(Arc::clone(&llm_port), circuit_breaker, None, None)
        .with_herald(Arc::clone(&herald));

    // Execute
    let result = service.execute(&paladin, "Test").await;
    assert!(result.is_ok());

    let result = result.unwrap();

    // Format the result
    let formatted = service.format_result(&result, &paladin);
    assert!(formatted.is_ok());

    let formatted = formatted.unwrap();
    assert!(formatted.is_some());

    let table_output = formatted.unwrap();

    // Verify table structure (has table borders)
    assert!(
        table_output.contains("│") || table_output.contains("|"),
        "Output should be a table"
    );
    // Table formatter may not include paladin name in simple format
    assert!(!table_output.is_empty(), "Table output should not be empty");

    println!("Table Herald output:\n{}", table_output);
}

#[tokio::test]
async fn test_paladin_without_herald() {
    // Create mock LLM port
    let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmPort::new("No herald response"));

    // Create circuit breaker
    let circuit_breaker = Arc::new(CircuitBreaker::new(
        5,
        3,
        std::time::Duration::from_secs(60),
    ));

    // Create Paladin
    let paladin = PaladinBuilder::new(Arc::clone(&llm_port))
        .system_prompt("You are a helpful assistant")
        .name("No Herald Paladin")
        .build()
        .await
        .expect("Failed to build Paladin");

    // Create execution service WITHOUT Herald
    let service = PaladinExecutionService::new(Arc::clone(&llm_port), circuit_breaker, None, None);
    // Note: not calling .with_herald()

    // Execute
    let result = service.execute(&paladin, "Test").await;
    assert!(result.is_ok());

    let result = result.unwrap();

    // Format the result - should return None
    let formatted = service.format_result(&result, &paladin);
    assert!(formatted.is_ok());

    let formatted = formatted.unwrap();
    assert!(
        formatted.is_none(),
        "Should return None when no Herald is configured"
    );
}

#[tokio::test]
async fn test_herald_with_metadata() {
    // Create mock LLM port
    let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmPort::new("Metadata test response"));

    // Create circuit breaker
    let circuit_breaker = Arc::new(CircuitBreaker::new(
        5,
        3,
        std::time::Duration::from_secs(60),
    ));

    // Create JSON Herald with metadata enabled
    let herald: Arc<dyn Herald> = Arc::new(JsonHerald::new());

    // Create Paladin
    let paladin = PaladinBuilder::new(Arc::clone(&llm_port))
        .system_prompt("You are a helpful assistant")
        .name("Metadata Paladin")
        .build()
        .await
        .expect("Failed to build Paladin");

    // Create execution service
    let service = PaladinExecutionService::new(Arc::clone(&llm_port), circuit_breaker, None, None)
        .with_herald(Arc::clone(&herald));

    // Execute
    let result = service.execute(&paladin, "Test").await;
    assert!(result.is_ok());

    let result = result.unwrap();

    // Verify metadata is present in result
    assert!(
        result.usage.total_tokens > 0,
        "Token count should be populated"
    );
    let _ = result.execution_time_ms;
    assert!(result.loop_count >= 1, "Loop count should be at least 1");

    // Format
    let formatted = service.format_result(&result, &paladin).unwrap().unwrap();

    // Parse JSON
    let parsed: serde_json::Value =
        serde_json::from_str(&formatted).expect("Output should be valid JSON");

    // The formatted output should contain the result data
    // (The exact metadata format depends on the Herald implementation)
    assert!(parsed.is_object());

    println!("Formatted output with metadata:\n{}", formatted);
}

/// Mock Paladin Port for Battalion testing
struct MockPaladinPort {
    response_suffix: String,
}

impl MockPaladinPort {
    fn new(response_suffix: impl Into<String>) -> Self {
        Self {
            response_suffix: response_suffix.into(),
        }
    }
}

#[async_trait]
impl PaladinPort for MockPaladinPort {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        use paladin_ports::output::paladin_port::StopReason;

        Ok(PaladinResult {
            output: format!(
                "{}: {} - {}",
                paladin.node.name, input, self.response_suffix
            ),
            usage: paladin_ports::output::llm_port::TokenUsage::new(50, 0),
            execution_time_ms: 25,
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
        let (_tx, rx) = tokio::sync::mpsc::channel(10);
        Ok(rx)
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

#[tokio::test]
async fn test_battalion_formation_with_herald() {
    // Create mock LLM port
    let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmPort::new("Formation step completed"));

    // Create mock Paladin port
    let paladin_port: Arc<dyn PaladinPort> = Arc::new(MockPaladinPort::new("completed"));

    // Create Herald
    let herald: Arc<dyn Herald> = Arc::new(JsonHerald::new());

    // Create two Paladins for the Formation
    let paladin1 = PaladinBuilder::new(Arc::clone(&llm_port))
        .system_prompt("You are step 1")
        .name("Paladin 1")
        .build()
        .await
        .expect("Failed to build Paladin 1");

    let paladin2 = PaladinBuilder::new(Arc::clone(&llm_port))
        .system_prompt("You are step 2")
        .name("Paladin 2")
        .build()
        .await
        .expect("Failed to build Paladin 2");

    // Create Formation
    let formation = Formation::new(vec![paladin1, paladin2], BattalionConfig::default())
        .expect("Failed to create Formation");

    // Create Formation service with Herald
    let service =
        FormationExecutionService::new(Arc::clone(&paladin_port)).with_herald(Arc::clone(&herald));

    // Execute Formation
    let result = service.execute(&formation, "Start").await;
    assert!(result.is_ok(), "Formation execution should succeed");

    let result = result.unwrap();

    // Format the result with Herald
    let formatted = service.format_result(&result);
    assert!(formatted.is_ok(), "Herald formatting should succeed");

    let formatted = formatted.unwrap();
    assert!(formatted.is_some(), "Herald should format Battalion result");

    let json_output = formatted.unwrap();

    // Verify it's valid JSON
    let parsed: serde_json::Value =
        serde_json::from_str(&json_output).expect("Output should be valid JSON");

    println!("Formation Herald output:\n{}", json_output);
    println!(
        "Parsed JSON keys: {:?}",
        parsed.as_object().map(|o| o.keys().collect::<Vec<_>>())
    );

    // Verify JSON contains Battalion fields
    assert!(parsed.as_object().is_some(), "Should be a JSON object");
    assert!(parsed.get("status").is_some(), "Should have status field");

    println!("All assertions passed for Formation Herald output");
}

#[tokio::test]
async fn test_battalion_phalanx_with_herald() {
    // Create mock LLM port
    let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmPort::new("Phalanx step completed"));

    // Create mock Paladin port
    let paladin_port: Arc<dyn PaladinPort> = Arc::new(MockPaladinPort::new("completed"));

    // Create Herald
    let herald: Arc<dyn Herald> = Arc::new(MarkdownHerald::new());

    // Create three Paladins for concurrent execution
    let paladin1 = PaladinBuilder::new(Arc::clone(&llm_port))
        .system_prompt("You are worker 1")
        .name("Worker 1")
        .build()
        .await
        .expect("Failed to build Worker 1");

    let paladin2 = PaladinBuilder::new(Arc::clone(&llm_port))
        .system_prompt("You are worker 2")
        .name("Worker 2")
        .build()
        .await
        .expect("Failed to build Worker 2");

    let paladin3 = PaladinBuilder::new(Arc::clone(&llm_port))
        .system_prompt("You are worker 3")
        .name("Worker 3")
        .build()
        .await
        .expect("Failed to build Worker 3");

    // Create Phalanx with CollectAll strategy
    let phalanx = Phalanx::new(
        vec![paladin1, paladin2, paladin3],
        BattalionConfig::default(),
    )
    .expect("Failed to create Phalanx");

    // Create Phalanx service with Herald
    let service =
        PhalanxExecutionService::new(Arc::clone(&paladin_port)).with_herald(Arc::clone(&herald));

    // Execute Phalanx
    let result = service.execute(&phalanx, "Parallel task").await;
    assert!(result.is_ok(), "Phalanx execution should succeed");

    let result = result.unwrap();

    // Format the result with Herald
    let formatted = service.format_result(&result);
    assert!(formatted.is_ok(), "Herald formatting should succeed");

    let formatted = formatted.unwrap();
    assert!(formatted.is_some(), "Herald should format Phalanx result");

    let markdown_output = formatted.unwrap();

    // Verify Markdown content
    assert!(markdown_output.contains("##") || markdown_output.contains("**"));

    println!("Phalanx Herald output:\n{}", markdown_output);
}

#[tokio::test]
async fn test_runtime_herald_override() {
    // Create mock LLM port
    let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmPort::new("Runtime override test"));

    // Create circuit breaker
    let circuit_breaker = Arc::new(CircuitBreaker::new(
        5,
        3,
        std::time::Duration::from_secs(60),
    ));

    // Create Paladin
    let paladin = PaladinBuilder::new(Arc::clone(&llm_port))
        .system_prompt("You are a helpful assistant")
        .name("Override Test Paladin")
        .build()
        .await
        .expect("Failed to build Paladin");

    // Create execution service with JSON Herald
    let json_herald: Arc<dyn Herald> = Arc::new(JsonHerald::new());
    let service = PaladinExecutionService::new(Arc::clone(&llm_port), circuit_breaker, None, None)
        .with_herald(Arc::clone(&json_herald));

    // Execute with JSON Herald
    let result = service.execute(&paladin, "Test 1").await;
    assert!(result.is_ok());

    let result = result.unwrap();
    let formatted_json = service.format_result(&result, &paladin).unwrap().unwrap();

    // Verify JSON output
    let parsed: serde_json::Value = serde_json::from_str(&formatted_json).unwrap();
    assert!(parsed.is_object());

    // Now override with Markdown Herald at runtime
    let markdown_herald: Arc<dyn Herald> = Arc::new(MarkdownHerald::new());
    let service = service.with_herald(Arc::clone(&markdown_herald));

    // Execute with overridden Markdown Herald
    let result = service.execute(&paladin, "Test 2").await;
    assert!(result.is_ok());

    let result = result.unwrap();
    let formatted_markdown = service.format_result(&result, &paladin).unwrap().unwrap();

    // Verify Markdown output
    assert!(formatted_markdown.contains("##") || formatted_markdown.contains("**"));

    // The two outputs should be different formats
    assert_ne!(
        formatted_json, formatted_markdown,
        "Overridden Herald should produce different format"
    );

    println!("JSON format:\n{}\n", formatted_json);
    println!("Markdown format after override:\n{}", formatted_markdown);
}
