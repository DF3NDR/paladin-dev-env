//! Unit tests for PaladinBuilder Arsenal integration

use async_trait::async_trait;
use chrono::Utc;
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::config::MCPServerConfig;
use paladin::core::platform::container::arsenal::Armament;
use paladin_ports::output::arsenal_port::ArsenalRegistry;
use paladin_ports::output::llm_port::{
    FinishReason, LlmError, LlmPort, LlmRequest, LlmResponse, StreamingResponse, TokenUsage,
};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

// Mock LLM Port for testing
struct MockLlmPort;

#[async_trait]
impl LlmPort for MockLlmPort {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        Ok(LlmResponse {
            id: Uuid::new_v4(),
            request_id: request.id,
            content: "Test response".to_string(),
            model: "test".to_string(),
            usage: TokenUsage::new(10, 20),
            cost: None,
            finish_reason: FinishReason::Stop,
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: None,
        })
    }

    async fn generate_stream(
        &self,
        _request: LlmRequest,
    ) -> Result<Box<dyn futures::Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError>
    {
        Err(LlmError::ProcessingError(
            "Streaming not supported in mock".to_string(),
        ))
    }

    async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
        Ok(true)
    }

    async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
        Ok(vec!["test".to_string()])
    }

    fn get_provider_name(&self) -> &'static str {
        "Mock"
    }

    fn get_capabilities(&self) -> paladin_ports::output::llm_port::ProviderCapabilities {
        paladin_ports::output::llm_port::ProviderCapabilities::default()
    }
}

// Mock Arsenal Registry for testing
struct MockArsenalRegistry;

#[async_trait]
impl ArsenalRegistry for MockArsenalRegistry {
    async fn register(&self, _armament: Armament) {
        // Mock implementation
    }

    async fn unregister(&self, _name: &str) -> Option<Armament> {
        None
    }

    async fn get(&self, _name: &str) -> Option<Armament> {
        None
    }
}

#[tokio::test]
async fn test_builder_add_mcp_stdio() {
    let llm_port = Arc::new(MockLlmPort);
    let builder = PaladinBuilder::new(llm_port)
        .system_prompt("Test prompt")
        .add_mcp_stdio("web_search", "uvx", &["mcp-web-search"]);

    // Build to verify no errors
    let result = builder.build();
    assert!(
        result.await.is_ok(),
        "Builder should succeed with MCP STDIO config"
    );
}

#[tokio::test]
async fn test_builder_add_multiple_mcp_servers() {
    let llm_port = Arc::new(MockLlmPort);
    let builder = PaladinBuilder::new(llm_port)
        .system_prompt("Test prompt")
        .add_mcp_stdio("web_search", "uvx", &["mcp-web-search"])
        .add_mcp_stdio("code_analyzer", "python", &["-m", "mcp_code_analyzer"])
        .add_mcp_stdio("calculator", "python", &["-m", "mcp_calculator"]);

    // Build to verify no errors
    let result = builder.build();
    assert!(
        result.await.is_ok(),
        "Builder should succeed with multiple MCP servers"
    );
}

#[tokio::test]
async fn test_builder_with_arsenal_registry() {
    let llm_port = Arc::new(MockLlmPort);
    let registry = Arc::new(MockArsenalRegistry);

    let builder = PaladinBuilder::new(llm_port)
        .system_prompt("Test prompt")
        .with_arsenal_registry(registry);

    // Build to verify no errors
    let result = builder.build();
    assert!(
        result.await.is_ok(),
        "Builder should succeed with arsenal registry"
    );
}

#[tokio::test]
async fn test_builder_full_arsenal_configuration() {
    let llm_port = Arc::new(MockLlmPort);
    let registry = Arc::new(MockArsenalRegistry);

    let builder = PaladinBuilder::new(llm_port)
        .system_prompt("You are a powerful assistant with many tools")
        .name("ToolMaster")
        .model("gpt-4")
        .temperature(0.7)
        .max_loops(10)
        .with_arsenal_registry(registry)
        .add_mcp_stdio("web_search", "uvx", &["mcp-web-search"])
        .add_mcp_stdio("code_analyzer", "python", &["-m", "mcp_code_analyzer"]);

    // Build to verify no errors
    let result = builder.build();
    assert!(
        result.await.is_ok(),
        "Builder should succeed with full arsenal configuration"
    );
}

#[test]
fn test_mcp_stdio_config_structure() {
    let config = MCPServerConfig {
        name: "test_tool".to_string(),
        server_type: "stdio".to_string(),
        command: Some("test_cmd".to_string()),
        args: Some(vec!["arg1".to_string(), "arg2".to_string()]),
        endpoint: None,
        auth_token_env: None,
    };

    assert_eq!(config.name, "test_tool");
    assert_eq!(config.server_type, "stdio");
    assert!(config.command.is_some());
    assert!(config.args.is_some());
    assert!(config.endpoint.is_none());
}

#[test]
fn test_mcp_sse_config_structure() {
    let config = MCPServerConfig {
        name: "test_service".to_string(),
        server_type: "sse".to_string(),
        command: None,
        args: None,
        endpoint: Some("http://localhost:8080/mcp".to_string()),
        auth_token_env: None,
    };

    assert_eq!(config.name, "test_service");
    assert_eq!(config.server_type, "sse");
    assert!(config.command.is_none());
    assert!(config.args.is_none());
    assert!(config.endpoint.is_some());
}
