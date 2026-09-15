//! Regression guard for the pre-existing "full agent bridge" (Phase 12.1
//! Plan 03, T-12.1-08 / Pitfall 3).
//!
//! Direct source verification during Phase 12.1 research showed that
//! `PaladinExecutionService`'s Layer-3 tool-call dispatch
//! (`function_call` -> `handle_tool_call` -> `arsenal.invoke`) was ALREADY
//! wired correctly and did NOT need to be rebuilt. Plan 03 only un-stubs
//! `ArsenalExecutionService::invoke` itself (a different file) and
//! explicitly does NOT modify `paladin_execution_service.rs`.
//!
//! This test proves that pre-existing dispatch is UNCHANGED: a fake
//! `LlmPort` returns a response carrying a `function_call`, wired to a spy
//! `ArsenalPort` test double, and we assert the spy's `invoke` was called
//! EXACTLY ONCE with an `ArmamentCall` whose `tool_name`/`arguments` match
//! the function call the LLM requested.

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

/// Minimal fake `LlmPort`: first call returns a response carrying a
/// `function_call`; every subsequent call returns a plain `Stop` response so
/// the Paladin's loop terminates.
struct FakeToolCallingLlm {
    responses: Mutex<Vec<LlmResponse>>,
}

impl FakeToolCallingLlm {
    fn new(first: LlmResponse) -> Self {
        Self {
            responses: Mutex::new(vec![first]),
        }
    }

    fn stop_response() -> LlmResponse {
        LlmResponse {
            id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            model: "fake-model".to_string(),
            content: "Done".to_string(),
            finish_reason: FinishReason::Stop,
            usage: TokenUsage::new(1, 1),
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: None,
        }
    }
}

#[async_trait]
impl LlmPort for FakeToolCallingLlm {
    async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let mut responses = self.responses.lock().await;
        if !responses.is_empty() {
            Ok(responses.remove(0))
        } else {
            Ok(Self::stop_response())
        }
    }

    async fn generate_stream(
        &self,
        _request: LlmRequest,
    ) -> Result<Box<dyn futures::Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError>
    {
        Err(LlmError::ProcessingError(
            "streaming not needed for this regression test".to_string(),
        ))
    }

    async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
        Ok(true)
    }

    async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
        Ok(vec!["fake-model".to_string()])
    }

    fn get_provider_name(&self) -> &'static str {
        "fake"
    }

    fn get_capabilities(&self) -> paladin_ports::output::llm_port::ProviderCapabilities {
        paladin_ports::output::llm_port::ProviderCapabilities::default()
    }
}

/// Spy `ArsenalPort` double: records every `ArmamentCall` it receives and
/// always returns a canned success, without touching a real MCP client.
struct SpyArsenal {
    calls: Mutex<Vec<ArmamentCall>>,
}

impl SpyArsenal {
    fn new() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
        }
    }

    async fn recorded_calls(&self) -> Vec<ArmamentCall> {
        self.calls.lock().await.clone()
    }
}

#[async_trait]
impl ArsenalPort for SpyArsenal {
    async fn list_armaments(&self) -> Vec<Armament> {
        vec![Armament {
            name: "echo".to_string(),
            description: "Echoes the message argument".to_string(),
            parameters: json!({
                "type": "object",
                "properties": { "message": { "type": "string" } }
            }),
            required_params: vec!["message".to_string()],
        }]
    }

    async fn invoke(&self, call: ArmamentCall) -> Result<ArmamentResult, ArsenalError> {
        self.calls.lock().await.push(call.clone());
        Ok(ArmamentResult::success(
            call.call_id,
            Value::String("spied output".to_string()),
            1,
        ))
    }

    fn validate_call(&self, call: &ArmamentCall) -> Result<(), ArsenalError> {
        if call.tool_name.is_empty() {
            return Err(ArsenalError::InvalidArguments(
                "Tool name cannot be empty".to_string(),
            ));
        }
        Ok(())
    }
}

/// T-12.1-08 / Pitfall 3 guard: proves the pre-existing
/// `function_call` -> `handle_tool_call` -> `arsenal.invoke` dispatch in
/// `PaladinExecutionService` is unchanged by Plan 03 -- it still calls
/// `ArsenalPort::invoke` exactly once with an `ArmamentCall` whose
/// `tool_name`/`arguments` match the LLM's `function_call`.
#[tokio::test]
async fn function_call_dispatch_still_invokes_arsenal_exactly_once_with_matching_call() {
    let function_call_response = LlmResponse {
        id: Uuid::new_v4(),
        request_id: Uuid::new_v4(),
        model: "fake-model".to_string(),
        content: "I'll echo that.".to_string(),
        finish_reason: FinishReason::FunctionCall,
        usage: TokenUsage::new(5, 5),
        created_at: Utc::now(),
        metadata: HashMap::new(),
        function_call: Some(FunctionCall {
            name: "echo".to_string(),
            arguments: json!({ "message": "hello regression" }).to_string(),
        }),
    };

    let llm_port: Arc<dyn LlmPort> = Arc::new(FakeToolCallingLlm::new(function_call_response));
    let arsenal_spy = Arc::new(SpyArsenal::new());
    let arsenal: Arc<dyn ArsenalPort> = arsenal_spy.clone();
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));

    let service =
        PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, Some(arsenal));

    let paladin = PaladinBuilder::new(llm_port)
        .system_prompt("You are a helpful echo assistant")
        .name("RegressionGuardPaladin")
        .max_loops(2)
        .build()
        .await
        .expect("Failed to build paladin");

    let result = service
        .execute(&paladin, "Please echo hello regression")
        .await;
    assert!(
        result.is_ok(),
        "execution should succeed: {:?}",
        result.err()
    );

    let recorded = arsenal_spy.recorded_calls().await;
    assert_eq!(
        recorded.len(),
        1,
        "arsenal.invoke should be called exactly once, got: {recorded:?}"
    );

    let call = &recorded[0];
    assert_eq!(call.tool_name, "echo");
    assert_eq!(
        call.arguments.get("message").and_then(Value::as_str),
        Some("hello regression")
    );
}
