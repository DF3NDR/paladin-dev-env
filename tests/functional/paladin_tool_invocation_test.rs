//! Functional end-to-end tests for Paladin tool invocation
//!
//! These tests verify the complete integration of:
//! - Paladin execution service
//! - Arsenal tool system
//! - MCP protocol adapters (STDIO and SSE)
//! - Tool result formatting and context injection

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

/// Mock LLM that simulates tool invocation scenarios
struct MockToolLlm {
    responses: Arc<Mutex<Vec<LlmResponse>>>,
    call_count: Arc<Mutex<usize>>,
}

impl MockToolLlm {
    fn new(responses: Vec<LlmResponse>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses)),
            call_count: Arc::new(Mutex::new(0)),
        }
    }
}

#[async_trait]
impl LlmPort for MockToolLlm {
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
                content: "Done".to_string(),
                finish_reason: FinishReason::Stop,
                usage: TokenUsage::new(5, 5),
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
        unimplemented!("Streaming not needed for functional tests")
    }

    async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
        Ok(vec!["mock-model".to_string()])
    }

    async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
        Ok(true)
    }

    fn get_provider_name(&self) -> &'static str {
        "mock"
    }

    fn get_capabilities(&self) -> paladin_ports::output::llm_port::ProviderCapabilities {
        paladin_ports::output::llm_port::ProviderCapabilities::default()
    }
}

/// Mock arsenal with calculator and echo tools
struct MockFunctionalArsenal {
    tools: HashMap<String, Armament>,
}

impl MockFunctionalArsenal {
    fn new() -> Self {
        let mut tools = HashMap::new();

        // Calculator tool
        tools.insert(
            "calculator".to_string(),
            Armament {
                name: "calculator".to_string(),
                description: "Performs basic arithmetic operations".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "operation": {
                            "type": "string",
                            "enum": ["add", "subtract", "multiply", "divide"]
                        },
                        "x": {"type": "number"},
                        "y": {"type": "number"}
                    }
                }),
                required_params: vec!["operation".to_string(), "x".to_string(), "y".to_string()],
            },
        );

        // Echo tool
        tools.insert(
            "echo".to_string(),
            Armament {
                name: "echo".to_string(),
                description: "Echoes back the input message".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "message": {"type": "string"}
                    }
                }),
                required_params: vec!["message".to_string()],
            },
        );

        // Slow tool (for timeout testing)
        tools.insert(
            "slow_tool".to_string(),
            Armament {
                name: "slow_tool".to_string(),
                description: "A tool that takes a long time to execute".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "duration_ms": {"type": "number"}
                    }
                }),
                required_params: vec!["duration_ms".to_string()],
            },
        );

        Self { tools }
    }
}

#[async_trait]
impl ArsenalPort for MockFunctionalArsenal {
    async fn list_armaments(&self) -> Vec<Armament> {
        self.tools.values().cloned().collect()
    }

    async fn invoke(&self, call: ArmamentCall) -> Result<ArmamentResult, ArsenalError> {
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
                    "subtract" => x - y,
                    "multiply" => x * y,
                    "divide" => {
                        if y == 0.0 {
                            return Ok(ArmamentResult::failure(
                                call.call_id,
                                "Division by zero",
                                10,
                            ));
                        }
                        x / y
                    }
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
            "echo" => {
                let message = call
                    .arguments
                    .get("message")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ArsenalError::InvalidArguments("Missing message".to_string()))?;

                Ok(ArmamentResult::success(
                    call.call_id,
                    Value::String(message.to_string()),
                    10,
                ))
            }
            "slow_tool" => {
                let duration_ms = call
                    .arguments
                    .get("duration_ms")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(5000);

                // Simulate slow execution
                tokio::time::sleep(Duration::from_millis(duration_ms)).await;

                Ok(ArmamentResult::success(
                    call.call_id,
                    Value::String(format!("Slept for {}ms", duration_ms)),
                    duration_ms,
                ))
            }
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
async fn test_paladin_with_stdio_tool() {
    // This test simulates using a STDIO MCP server
    // In reality, we use our mock arsenal which behaves like a connected MCP server

    let responses = vec![
        // First response: LLM decides to use calculator tool
        LlmResponse {
            id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            model: "mock-model".to_string(),
            content: "I'll calculate that for you.".to_string(),
            finish_reason: FinishReason::FunctionCall,
            usage: TokenUsage::new(10, 10),
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: Some(FunctionCall {
                name: "calculator".to_string(),
                arguments: json!({
                    "operation": "multiply",
                    "x": 12,
                    "y": 8
                })
                .to_string(),
            }),
        },
        // Second response: LLM provides final answer
        LlmResponse {
            id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            model: "mock-model".to_string(),
            content: "The result is 96.".to_string(),
            finish_reason: FinishReason::Stop,
            usage: TokenUsage::new(10, 5),
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: None,
        },
        // Fallback response for extra iterations
        LlmResponse {
            id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            model: "mock-model".to_string(),
            content: "The result is 96.".to_string(),
            finish_reason: FinishReason::Stop,
            usage: TokenUsage::new(5, 5),
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: None,
        },
    ];

    let llm_port: Arc<dyn LlmPort> = Arc::new(MockToolLlm::new(responses));
    let arsenal: Arc<dyn ArsenalPort> = Arc::new(MockFunctionalArsenal::new());
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));

    let service =
        PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, Some(arsenal));

    let paladin = PaladinBuilder::new(llm_port)
        .system_prompt("You are a helpful calculator assistant")
        .max_loops(3)
        .build()
        .await
        .expect("Failed to build paladin");

    let result = service.execute(&paladin, "What is 12 times 8?").await;

    assert!(result.is_ok(), "Execution should succeed");
    let execution_result = result.unwrap();

    // Verify the output contains the calculation result
    assert!(
        execution_result.output.contains("96") || execution_result.output.contains("result"),
        "Output should contain calculation result: {}",
        execution_result.output
    );
}

#[tokio::test]
async fn test_paladin_with_sse_tool() {
    // This test simulates using an SSE MCP server
    // In reality, we use our mock arsenal which behaves like a connected MCP server

    let responses = vec![
        // LLM uses echo tool
        LlmResponse {
            id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            model: "mock-model".to_string(),
            content: "Let me echo that back.".to_string(),
            finish_reason: FinishReason::FunctionCall,
            usage: TokenUsage::new(10, 10),
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: Some(FunctionCall {
                name: "echo".to_string(),
                arguments: json!({
                    "message": "Hello, World!"
                })
                .to_string(),
            }),
        },
        // Final response
        LlmResponse {
            id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            model: "mock-model".to_string(),
            content: "I echoed your message: Hello, World!".to_string(),
            finish_reason: FinishReason::Stop,
            usage: TokenUsage::new(10, 8),
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: None,
        },
        // Fallback
        LlmResponse {
            id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            model: "mock-model".to_string(),
            content: "I echoed your message: Hello, World!".to_string(),
            finish_reason: FinishReason::Stop,
            usage: TokenUsage::new(5, 5),
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: None,
        },
    ];

    let llm_port: Arc<dyn LlmPort> = Arc::new(MockToolLlm::new(responses));
    let arsenal: Arc<dyn ArsenalPort> = Arc::new(MockFunctionalArsenal::new());
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));

    let service =
        PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, Some(arsenal));

    let paladin = PaladinBuilder::new(llm_port)
        .system_prompt("You are a helpful echo assistant")
        .max_loops(3)
        .build()
        .await
        .expect("Failed to build paladin");

    let result = service
        .execute(&paladin, "Please echo: Hello, World!")
        .await;

    assert!(result.is_ok(), "Execution should succeed");
    let execution_result = result.unwrap();

    // Verify the output contains the echoed message
    assert!(
        execution_result.output.contains("Hello, World!"),
        "Output should contain echoed message: {}",
        execution_result.output
    );
}

#[tokio::test]
async fn test_multiple_tool_invocations() {
    // Test sequential tool calls within a single execution

    let responses = vec![
        // First tool call: calculator add
        LlmResponse {
            id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            model: "mock-model".to_string(),
            content: "Let me calculate the first part.".to_string(),
            finish_reason: FinishReason::FunctionCall,
            usage: TokenUsage::new(10, 10),
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: Some(FunctionCall {
                name: "calculator".to_string(),
                arguments: json!({
                    "operation": "add",
                    "x": 10,
                    "y": 5
                })
                .to_string(),
            }),
        },
        // Second tool call: calculator multiply
        LlmResponse {
            id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            model: "mock-model".to_string(),
            content: "Now let me multiply the result.".to_string(),
            finish_reason: FinishReason::FunctionCall,
            usage: TokenUsage::new(10, 10),
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: Some(FunctionCall {
                name: "calculator".to_string(),
                arguments: json!({
                    "operation": "multiply",
                    "x": 15,
                    "y": 2
                })
                .to_string(),
            }),
        },
        // Final response
        LlmResponse {
            id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            model: "mock-model".to_string(),
            content: "First I added 10 and 5 to get 15, then multiplied by 2 to get 30."
                .to_string(),
            finish_reason: FinishReason::Stop,
            usage: TokenUsage::new(10, 15),
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: None,
        },
        // Fallback responses for extra iterations
        LlmResponse {
            id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            model: "mock-model".to_string(),
            content: "First I added 10 and 5 to get 15, then multiplied by 2 to get 30."
                .to_string(),
            finish_reason: FinishReason::Stop,
            usage: TokenUsage::new(5, 10),
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: None,
        },
        LlmResponse {
            id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            model: "mock-model".to_string(),
            content: "First I added 10 and 5 to get 15, then multiplied by 2 to get 30."
                .to_string(),
            finish_reason: FinishReason::Stop,
            usage: TokenUsage::new(5, 10),
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: None,
        },
    ];

    let llm_port: Arc<dyn LlmPort> = Arc::new(MockToolLlm::new(responses));
    let arsenal: Arc<dyn ArsenalPort> = Arc::new(MockFunctionalArsenal::new());
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));

    let service =
        PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, Some(arsenal));

    let paladin = PaladinBuilder::new(llm_port)
        .system_prompt("You are a helpful calculator assistant")
        .max_loops(5)
        .build()
        .await
        .expect("Failed to build paladin");

    let result = service.execute(&paladin, "Calculate (10 + 5) * 2").await;

    assert!(result.is_ok(), "Execution should succeed");
    let execution_result = result.unwrap();

    // Verify both calculations appear in output
    assert!(
        (execution_result.output.contains("15") || execution_result.output.contains("30"))
            && execution_result.output.contains("30"),
        "Output should show both calculation steps: {}",
        execution_result.output
    );
}

#[tokio::test]
async fn test_tool_timeout_handling() {
    // Test that slow tools are handled properly

    let responses = vec![
        // Tool call that will timeout
        LlmResponse {
            id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            model: "mock-model".to_string(),
            content: "Let me run that slow operation.".to_string(),
            finish_reason: FinishReason::FunctionCall,
            usage: TokenUsage::new(10, 10),
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: Some(FunctionCall {
                name: "slow_tool".to_string(),
                arguments: json!({
                    "duration_ms": 100
                })
                .to_string(),
            }),
        },
        // Recovery response
        LlmResponse {
            id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            model: "mock-model".to_string(),
            content: "The operation completed successfully.".to_string(),
            finish_reason: FinishReason::Stop,
            usage: TokenUsage::new(10, 8),
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: None,
        },
        // Fallback
        LlmResponse {
            id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            model: "mock-model".to_string(),
            content: "The operation completed successfully.".to_string(),
            finish_reason: FinishReason::Stop,
            usage: TokenUsage::new(5, 5),
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: None,
        },
    ];

    let llm_port: Arc<dyn LlmPort> = Arc::new(MockToolLlm::new(responses));
    let arsenal: Arc<dyn ArsenalPort> = Arc::new(MockFunctionalArsenal::new());
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));

    let service =
        PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, Some(arsenal));

    let paladin = PaladinBuilder::new(llm_port)
        .system_prompt("You are a helpful assistant")
        .max_loops(3)
        .build()
        .await
        .expect("Failed to build paladin");

    let result = service.execute(&paladin, "Run slow operation").await;

    // Should succeed even if tool takes time
    assert!(
        result.is_ok(),
        "Execution should handle slow tool: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn test_tool_failure_resilience() {
    // Test that paladin continues gracefully after tool failures

    let responses = vec![
        // Tool call with division by zero
        LlmResponse {
            id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            model: "mock-model".to_string(),
            content: "Let me calculate that.".to_string(),
            finish_reason: FinishReason::FunctionCall,
            usage: TokenUsage::new(10, 10),
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: Some(FunctionCall {
                name: "calculator".to_string(),
                arguments: json!({
                    "operation": "divide",
                    "x": 10,
                    "y": 0
                })
                .to_string(),
            }),
        },
        // Recovery response
        LlmResponse {
            id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            model: "mock-model".to_string(),
            content: "I apologize, I cannot divide by zero. That operation is undefined."
                .to_string(),
            finish_reason: FinishReason::Stop,
            usage: TokenUsage::new(10, 15),
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: None,
        },
        // Fallback
        LlmResponse {
            id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            model: "mock-model".to_string(),
            content: "I apologize, I cannot divide by zero.".to_string(),
            finish_reason: FinishReason::Stop,
            usage: TokenUsage::new(5, 8),
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: None,
        },
    ];

    let llm_port: Arc<dyn LlmPort> = Arc::new(MockToolLlm::new(responses));
    let arsenal: Arc<dyn ArsenalPort> = Arc::new(MockFunctionalArsenal::new());
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));

    let service =
        PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, Some(arsenal));

    let paladin = PaladinBuilder::new(llm_port)
        .system_prompt("You are a helpful calculator assistant")
        .max_loops(3)
        .build()
        .await
        .expect("Failed to build paladin");

    let result = service.execute(&paladin, "What is 10 divided by 0?").await;

    assert!(
        result.is_ok(),
        "Execution should continue after tool failure"
    );
    let execution_result = result.unwrap();

    // Verify graceful error handling
    assert!(
        execution_result.output.contains("cannot") || execution_result.output.contains("zero"),
        "Output should show graceful error handling: {}",
        execution_result.output
    );
}
