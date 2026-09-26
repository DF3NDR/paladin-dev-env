use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::core::platform::container::arsenal::handoff_tool::HandoffTool;
use paladin_ports::output::llm_port::{
    FinishReason, LlmError, LlmPort, LlmRequest, LlmResponse, ProviderCapabilities, TokenUsage,
};
use serde_json::json;
use std::sync::Arc;

// Mock LLM Port for testing
struct MockLlmPort;

#[async_trait::async_trait]
impl LlmPort for MockLlmPort {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        Ok(LlmResponse {
            id: uuid::Uuid::new_v4(),
            request_id: request.id,
            model: request.model,
            content: "Mock response".to_string(),
            finish_reason: FinishReason::Stop,
            usage: TokenUsage::new(10, 20),
            cost: None,
            created_at: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
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
        Err(LlmError::ProcessingError(
            "Mock does not support streaming".to_string(),
        ))
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

    fn get_capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::default()
    }
}

#[tokio::test]
async fn test_handoff_tool_schema_has_agent_enum() {
    // Create mock LLM
    let llm = Arc::new(MockLlmPort);

    // Create specialist agents
    let specialist1 = PaladinBuilder::new(llm.clone())
        .name("code_expert")
        .system_prompt("Expert in code analysis")
        .build()
        .await
        .unwrap();

    let specialist2 = PaladinBuilder::new(llm.clone())
        .name("doc_expert")
        .system_prompt("Expert in documentation")
        .build()
        .await
        .unwrap();

    let specialists = vec![Arc::new(specialist1), Arc::new(specialist2)];

    // Create handoff tool with specialists
    let tool = HandoffTool::new(specialists);

    // Get the schema
    let schema = tool.get_schema();

    // Verify schema structure
    assert_eq!(schema["type"], "function");
    assert_eq!(schema["function"]["name"], "handoff_to_agent");
    assert_eq!(
        schema["function"]["description"],
        "Delegate the current task to a specialist agent"
    );

    // Verify parameters
    let params = &schema["function"]["parameters"];
    assert_eq!(params["type"], "object");

    // Check required fields
    let required = params["required"].as_array().unwrap();
    assert!(required.contains(&json!("agent_name")));
    assert!(required.contains(&json!("message")));

    // Verify agent_name has enum with specialist names
    let agent_name_prop = &params["properties"]["agent_name"];
    assert_eq!(agent_name_prop["type"], "string");
    assert_eq!(
        agent_name_prop["description"],
        "Name of the specialist agent to delegate to"
    );

    let enum_values = agent_name_prop["enum"].as_array().unwrap();
    assert_eq!(enum_values.len(), 2);
    assert!(enum_values.contains(&json!("code_expert")));
    assert!(enum_values.contains(&json!("doc_expert")));

    // Verify message field
    let message_prop = &params["properties"]["message"];
    assert_eq!(message_prop["type"], "string");
    assert_eq!(
        message_prop["description"],
        "Task description and context to pass to the specialist agent"
    );
}

#[test]
fn test_handoff_tool_schema_empty_specialists() {
    // Create handoff tool with no specialists
    let tool = HandoffTool::new(vec![]);

    let schema = tool.get_schema();

    // Should still have valid schema but empty enum
    let agent_name_prop = &schema["function"]["parameters"]["properties"]["agent_name"];
    let enum_values = agent_name_prop["enum"].as_array().unwrap();
    assert_eq!(enum_values.len(), 0);
}

#[tokio::test]
async fn test_validate_parameters_valid_agent() {
    let llm = Arc::new(MockLlmPort);

    let specialist = PaladinBuilder::new(llm)
        .name("test_agent")
        .system_prompt("Test specialist")
        .build()
        .await
        .unwrap();

    let tool = HandoffTool::new(vec![Arc::new(specialist)]);

    let mut params = std::collections::HashMap::new();
    params.insert("agent_name".to_string(), json!("test_agent"));
    params.insert("message".to_string(), json!("Test task"));

    // Should not panic or return error
    let result = tool.validate_parameters(&params);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_validate_parameters_invalid_agent() {
    let llm = Arc::new(MockLlmPort);

    let specialist = PaladinBuilder::new(llm)
        .name("test_agent")
        .system_prompt("Test specialist")
        .build()
        .await
        .unwrap();

    let tool = HandoffTool::new(vec![Arc::new(specialist)]);

    let mut params = std::collections::HashMap::new();
    params.insert("agent_name".to_string(), json!("invalid_agent"));
    params.insert("message".to_string(), json!("Test task"));

    let result = tool.validate_parameters(&params);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_validate_parameters_missing_agent_name() {
    let llm = Arc::new(MockLlmPort);

    let specialist = PaladinBuilder::new(llm)
        .name("test_agent")
        .system_prompt("Test specialist")
        .build()
        .await
        .unwrap();

    let tool = HandoffTool::new(vec![Arc::new(specialist)]);

    let mut params = std::collections::HashMap::new();
    params.insert("message".to_string(), json!("Test task"));

    let result = tool.validate_parameters(&params);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_validate_parameters_missing_message() {
    let llm = Arc::new(MockLlmPort);

    let specialist = PaladinBuilder::new(llm)
        .name("test_agent")
        .system_prompt("Test specialist")
        .build()
        .await
        .unwrap();

    let tool = HandoffTool::new(vec![Arc::new(specialist)]);

    let mut params = std::collections::HashMap::new();
    params.insert("agent_name".to_string(), json!("test_agent"));

    let result = tool.validate_parameters(&params);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_validate_parameters_empty_message() {
    let llm = Arc::new(MockLlmPort);

    let specialist = PaladinBuilder::new(llm)
        .name("test_agent")
        .system_prompt("Test specialist")
        .build()
        .await
        .unwrap();

    let tool = HandoffTool::new(vec![Arc::new(specialist)]);

    let mut params = std::collections::HashMap::new();
    params.insert("agent_name".to_string(), json!("test_agent"));
    params.insert("message".to_string(), json!(""));

    let result = tool.validate_parameters(&params);
    assert!(result.is_err());
}
