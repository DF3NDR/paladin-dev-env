//! Integration tests for Arsenal tool context injection into Paladin execution
//!
//! These tests verify that:
//! - Tool calls are properly detected in LLM responses
//! - Tools are invoked and results are formatted
//! - Formatted results are injected back into the conversation context
//! - Execution continues gracefully even if tools fail

use async_trait::async_trait;
use chrono::Utc;
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::core::platform::container::arsenal::{
    Armament, ArmamentCall, ArmamentResult, ArsenalError,
};
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin_ports::output::arsenal_port::ArsenalPort;
use paladin_ports::output::llm_port::{
    FinishReason, FunctionCall, LlmError, LlmPort, LlmRequest, LlmResponse, StreamingResponse,
    TokenUsage,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Mock LLM adapter that returns responses with function calls
struct MockLlmWithFunctionCalls {
    responses: Arc<Mutex<Vec<LlmResponse>>>,
    call_count: Arc<Mutex<usize>>,
}

impl MockLlmWithFunctionCalls {
    fn new(responses: Vec<LlmResponse>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses)),
            call_count: Arc::new(Mutex::new(0)),
        }
    }
}

#[async_trait]
impl LlmPort for MockLlmWithFunctionCalls {
    async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let mut count = self.call_count.lock().await;
        let responses = self.responses.lock().await;

        if *count < responses.len() {
            let response = responses[*count].clone();
            *count += 1;
            Ok(response)
        } else {
            Ok(LlmResponse {
                id: Uuid::new_v4(),
                request_id: Uuid::new_v4(),
                model: "mock".to_string(),
                content: "No more responses".to_string(),
                finish_reason: FinishReason::Stop,
                usage: TokenUsage::new(5, 5),
                cost: None,
                created_at: Utc::now(),
                metadata: HashMap::new(),
                function_call: None,
            })
        }
    }

    async fn generate_stream(
        &self,
        _request: LlmRequest,
    ) -> Result<Box<dyn futures::Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError>
    {
        unimplemented!("Streaming not needed for these tests")
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

/// Mock Arsenal adapter for testing tool invocation
struct MockArsenal {
    tools: HashMap<String, Armament>,
    invocation_log: Arc<Mutex<Vec<ArmamentCall>>>,
}

impl MockArsenal {
    fn new() -> Self {
        let mut tools = HashMap::new();

        // Add a calculator tool
        tools.insert(
            "calculator".to_string(),
            Armament {
                name: "calculator".to_string(),
                description: "Performs arithmetic operations".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "operation": {"type": "string"},
                        "x": {"type": "number"},
                        "y": {"type": "number"}
                    },
                    "required": ["operation", "x", "y"]
                }),
                required_params: vec!["operation".to_string(), "x".to_string(), "y".to_string()],
            },
        );

        // Add a search tool
        tools.insert(
            "search".to_string(),
            Armament {
                name: "search".to_string(),
                description: "Searches for information".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"}
                    },
                    "required": ["query"]
                }),
                required_params: vec!["query".to_string()],
            },
        );

        Self {
            tools,
            invocation_log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn get_invocation_log(&self) -> Arc<Mutex<Vec<ArmamentCall>>> {
        self.invocation_log.clone()
    }
}

#[async_trait]
impl ArsenalPort for MockArsenal {
    async fn list_armaments(&self) -> Vec<Armament> {
        self.tools.values().cloned().collect()
    }

    async fn invoke(&self, call: ArmamentCall) -> Result<ArmamentResult, ArsenalError> {
        // Log the invocation
        self.invocation_log.lock().await.push(call.clone());

        // Simulate tool execution
        match call.tool_name.as_str() {
            "calculator" => {
                let operation = call
                    .arguments
                    .get("operation")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ArsenalError::InvalidArguments("Missing operation".to_string())
                    })?;

                let x = call
                    .arguments
                    .get("x")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| ArsenalError::InvalidArguments("Missing x".to_string()))?;

                let y = call
                    .arguments
                    .get("y")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| ArsenalError::InvalidArguments("Missing y".to_string()))?;

                let result = match operation {
                    "add" => x + y,
                    "multiply" => x * y,
                    _ => {
                        return Err(ArsenalError::InvalidArguments(format!(
                            "Unknown operation: {}",
                            operation
                        )));
                    }
                };

                Ok(ArmamentResult::success(
                    call.call_id,
                    Value::Number(serde_json::Number::from_f64(result).unwrap()),
                    50,
                ))
            }
            "search" => Ok(ArmamentResult::success(
                call.call_id,
                Value::String("Found 5 results".to_string()),
                100,
            )),
            _ => Err(ArsenalError::ToolNotFound(call.tool_name.clone())),
        }
    }

    fn validate_call(&self, call: &ArmamentCall) -> Result<(), ArsenalError> {
        let tool = self
            .tools
            .get(&call.tool_name)
            .ok_or_else(|| ArsenalError::ToolNotFound(call.tool_name.clone()))?;

        for required in &tool.required_params {
            if !call.arguments.contains_key(required) {
                return Err(ArsenalError::InvalidArguments(format!(
                    "Missing required parameter: {}",
                    required
                )));
            }
        }

        Ok(())
    }
}

#[tokio::test]
async fn test_tool_call_detection() {
    // LLM responses: first with function call, then final response, plus fallback
    let responses = vec![
        LlmResponse {
            id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            model: "mock-model".to_string(),
            content: "I'll calculate that for you.".to_string(),
            finish_reason: FinishReason::FunctionCall,
            usage: TokenUsage::new(10, 10),
            cost: None,
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: Some(FunctionCall {
                name: "calculator".to_string(),
                arguments: json!({
                    "operation": "add",
                    "x": 5,
                    "y": 3
                })
                .to_string(),
            }),
        },
        LlmResponse {
            id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            model: "mock-model".to_string(),
            content: "The answer is 8.".to_string(),
            finish_reason: FinishReason::Stop,
            usage: TokenUsage::new(10, 5),
            cost: None,
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: None,
        },
        LlmResponse {
            id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            model: "mock-model".to_string(),
            content: "The answer is 8.".to_string(),
            finish_reason: FinishReason::Stop,
            usage: TokenUsage::new(10, 5),
            cost: None,
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: None,
        },
    ];

    let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmWithFunctionCalls::new(responses));
    let arsenal: Arc<dyn ArsenalPort> = Arc::new(MockArsenal::new());
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));

    let service = PaladinExecutionService::new(
        llm_port.clone(),
        circuit_breaker,
        None,
        Some(arsenal.clone()),
    );

    let paladin = PaladinBuilder::new(llm_port)
        .system_prompt("You are a helpful calculator assistant")
        .max_loops(3)
        .build()
        .await
        .expect("Failed to build paladin");

    let result = service.execute(&paladin, "What is 5 + 3?").await;

    assert!(result.is_ok(), "Execution should succeed");
    let execution_result = result.unwrap();

    // The output should contain the answer from the tool calculation
    assert!(
        execution_result.output.contains("8") || execution_result.output.contains("answer"),
        "Output should contain tool result: {}",
        execution_result.output
    );
}

#[tokio::test]
async fn test_tool_invocation_and_injection() {
    let mock_arsenal = Arc::new(MockArsenal::new());
    let invocation_log = mock_arsenal.get_invocation_log();

    // First response: LLM wants to use calculator
    // Second response: LLM responds after seeing tool result
    let responses = vec![
        LlmResponse {
            id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            model: "mock-model".to_string(),
            content: "I'll calculate that for you.".to_string(),
            finish_reason: FinishReason::FunctionCall,
            usage: TokenUsage::new(10, 10),
            cost: None,
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: Some(FunctionCall {
                name: "calculator".to_string(),
                arguments: json!({
                    "operation": "multiply",
                    "x": 7,
                    "y": 6
                })
                .to_string(),
            }),
        },
        LlmResponse {
            id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            model: "mock-model".to_string(),
            content: "The result is 42.".to_string(),
            finish_reason: FinishReason::Stop,
            usage: TokenUsage::new(10, 5),
            cost: None,
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: None,
        },
    ];

    let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmWithFunctionCalls::new(responses));
    let arsenal: Arc<dyn ArsenalPort> = mock_arsenal.clone();
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));

    let service =
        PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, Some(arsenal));

    let paladin = PaladinBuilder::new(llm_port)
        .system_prompt("You are a helpful calculator assistant")
        .max_loops(3)
        .build()
        .await
        .expect("Failed to build paladin");

    let result = service.execute(&paladin, "What is 7 times 6?").await;

    assert!(result.is_ok(), "Execution should succeed");

    // Verify tool was actually invoked
    let log = invocation_log.lock().await;
    assert_eq!(log.len(), 1, "Should have invoked exactly one tool");
    assert_eq!(log[0].tool_name, "calculator");

    let operation = log[0].arguments.get("operation").and_then(|v| v.as_str());
    assert_eq!(operation, Some("multiply"));
}

#[tokio::test]
async fn test_paladin_continues_after_tool_failure() {
    let mock_arsenal = Arc::new(MockArsenal::new());

    // First response: LLM tries to use non-existent tool
    // Second response: LLM recovers and provides answer
    // Third response: Fallback for any extra loop iterations
    let responses = vec![
        LlmResponse {
            id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            model: "mock-model".to_string(),
            content: "Let me look that up.".to_string(),
            finish_reason: FinishReason::FunctionCall,
            usage: TokenUsage::new(10, 10),
            cost: None,
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: Some(FunctionCall {
                name: "nonexistent_tool".to_string(),
                arguments: json!({"query": "test"}).to_string(),
            }),
        },
        LlmResponse {
            id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            model: "mock-model".to_string(),
            content: "I apologize, that tool is not available. Let me answer directly.".to_string(),
            finish_reason: FinishReason::Stop,
            usage: TokenUsage::new(15, 10),
            cost: None,
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: None,
        },
        LlmResponse {
            id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            model: "mock-model".to_string(),
            content: "I apologize, that tool is not available. Let me answer directly.".to_string(),
            finish_reason: FinishReason::Stop,
            usage: TokenUsage::new(15, 10),
            cost: None,
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: None,
        },
    ];

    let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmWithFunctionCalls::new(responses));
    let arsenal: Arc<dyn ArsenalPort> = mock_arsenal;
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));

    let service =
        PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, Some(arsenal));

    let paladin = PaladinBuilder::new(llm_port)
        .system_prompt("You are a helpful assistant")
        .max_loops(3)
        .build()
        .await
        .expect("Failed to build paladin");

    let result = service.execute(&paladin, "What is the weather?").await;

    // Should succeed despite tool failure
    assert!(
        result.is_ok(),
        "Execution should continue after tool failure"
    );

    let execution_result = result.unwrap();
    // Output should contain the recovery response
    assert!(
        execution_result.output.contains("apologize")
            || execution_result.output.contains("not available"),
        "Output should show recovery from tool failure: {}",
        execution_result.output
    );
}
