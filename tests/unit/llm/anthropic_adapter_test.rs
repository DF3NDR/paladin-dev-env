// tests/unit/llm/anthropic_adapter_test.rs
//
// Unit tests for Anthropic adapter with mocked HTTP responses

use mockito::{Server, ServerGuard};
use paladin::core::platform::container::prompt::{
    PromptItem, PromptType, SystemPrompt, UserPrompt,
};
use paladin_llm::anthropic::{AnthropicAdapter, AnthropicConfig};
use paladin_ports::output::llm_port::{LlmPort, LlmRequest};
use std::collections::HashMap;
use uuid::Uuid;

/// Helper to create a mock server and adapter configured to use it
///
/// Uses the async server constructor: the blocking, synchronous
/// `Server::new()` panics with a runtime-nesting error when called from
/// inside an already-running Tokio runtime, which every call site here is
/// (`#[tokio::test]`).
async fn setup_mock_server() -> (ServerGuard, AnthropicAdapter) {
    let server = Server::new_async().await;
    let config = AnthropicConfig {
        api_key: "test-api-key".to_string(),
        base_url: server.url(),
        model: "claude-3-5-sonnet-20241022".to_string(),
        max_tokens: 4096,
        timeout_seconds: 30,
    };
    let adapter = AnthropicAdapter::new(config).unwrap();
    (server, adapter)
}

/// Helper to create a basic LLM request with system prompt
fn create_test_request(content: &str) -> LlmRequest {
    // PromptItem::new returns a Result; construction here uses fixed, valid
    // inputs, so unwrap cannot fail.
    let system_prompt = PromptItem::new(PromptType::System(SystemPrompt {
        instructions: content.to_string(),
        constraints: None,
    }))
    .unwrap();

    LlmRequest {
        id: Uuid::new_v4(),
        model: "claude-3-5-sonnet-20241022".to_string(),
        prompt: system_prompt,
        attachments: vec![],
        stream: false,
        metadata: HashMap::new(),
    }
}

/// Helper to create a request with user prompt
fn create_user_request(content: &str) -> LlmRequest {
    let user_prompt = PromptItem::new(PromptType::User(UserPrompt {
        query: content.to_string(),
        context: None,
    }))
    .unwrap();

    LlmRequest {
        id: Uuid::new_v4(),
        model: "claude-3-5-sonnet-20241022".to_string(),
        prompt: user_prompt,
        attachments: vec![],
        stream: false,
        metadata: HashMap::new(),
    }
}

#[tokio::test]
async fn test_anthropic_successful_completion() {
    let (mut server, adapter) = setup_mock_server().await;

    let mock_response = r#"{
        "id": "msg_123",
        "type": "message",
        "role": "assistant",
        "content": [{
            "type": "text",
            "text": "Hello! How can I assist you today?"
        }],
        "model": "claude-3-5-sonnet-20241022",
        "stop_reason": "end_turn",
        "usage": {
            "input_tokens": 10,
            "output_tokens": 20
        }
    }"#;

    let _mock = server
        .mock("POST", "/messages")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(mock_response)
        .create_async()
        .await;

    let request = create_user_request("Hello");
    let response = adapter.generate(request).await;

    assert!(response.is_ok());
    let response = response.unwrap();
    assert_eq!(response.content, "Hello! How can I assist you today?");
    assert_eq!(response.usage.prompt_tokens, 10);
    assert_eq!(response.usage.completion_tokens, 20);
    assert_eq!(response.usage.total_tokens, 30);
}

#[tokio::test]
async fn test_anthropic_streaming_response() {
    let (mut server, adapter) = setup_mock_server().await;

    // Mock Claude SSE streaming response with different event types
    let mock_stream = "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_123\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[],\"model\":\"claude-3-5-sonnet-20241022\"}}\n\nevent: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\nevent: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}\n\nevent: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\" world\"}}\n\nevent: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\nevent: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"}}\n\nevent: message_stop\ndata: {\"type\":\"message_stop\"}\n\n";

    let _mock = server
        .mock("POST", "/messages")
        .with_status(200)
        .with_header("content-type", "text/event-stream")
        .with_body(mock_stream)
        .create_async()
        .await;

    let request = create_user_request("Hello");
    let stream_result = adapter.generate_stream(request).await;

    assert!(stream_result.is_ok());
    // Streaming validation would require consuming the stream
    // For now, we just verify it doesn't error
}

#[tokio::test]
async fn test_anthropic_system_message_formatting() {
    let (mut server, adapter) = setup_mock_server().await;

    let mock_response = r#"{
        "id": "msg_123",
        "type": "message",
        "role": "assistant",
        "content": [{
            "type": "text",
            "text": "I understand the instructions."
        }],
        "model": "claude-3-5-sonnet-20241022",
        "stop_reason": "end_turn",
        "usage": {
            "input_tokens": 15,
            "output_tokens": 10
        }
    }"#;

    let _mock = server
        .mock("POST", "/messages")
        .match_header("anthropic-version", "2023-06-01")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(mock_response)
        .create_async()
        .await;

    // Use system prompt to test system message formatting
    let request = create_test_request("You are a helpful assistant");
    let response = adapter.generate(request).await;

    assert!(response.is_ok());
    let response = response.unwrap();
    assert_eq!(response.content, "I understand the instructions.");
}

#[tokio::test]
async fn test_anthropic_max_tokens_enforcement() {
    let (mut server, adapter) = setup_mock_server().await;

    let mock_response = r#"{
        "id": "msg_123",
        "type": "message",
        "role": "assistant",
        "content": [{
            "type": "text",
            "text": "Response"
        }],
        "model": "claude-3-5-sonnet-20241022",
        "stop_reason": "max_tokens",
        "usage": {
            "input_tokens": 10,
            "output_tokens": 100
        }
    }"#;

    let _mock = server
        .mock("POST", "/messages")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(mock_response)
        .create_async()
        .await;

    let request = create_user_request("Generate a long response");
    let response = adapter.generate(request).await;

    assert!(response.is_ok());
    let response = response.unwrap();
    // Verify that max_tokens stop reason is properly handled
    assert!(matches!(
        response.finish_reason,
        paladin_ports::output::llm_port::FinishReason::Length
    ));
}

#[tokio::test]
async fn test_anthropic_auth_failure_401() {
    let (mut server, adapter) = setup_mock_server().await;

    let error_response = r#"{
        "type": "error",
        "error": {
            "type": "authentication_error",
            "message": "invalid x-api-key"
        }
    }"#;

    let _mock = server
        .mock("POST", "/messages")
        .with_status(401)
        .with_header("content-type", "application/json")
        .with_body(error_response)
        .create_async()
        .await;

    let request = create_user_request("Hello");
    let response = adapter.generate(request).await;

    assert!(response.is_err());
    let error = response.unwrap_err();
    assert!(matches!(
        error,
        paladin_ports::output::llm_port::LlmError::AuthenticationError(_)
    ));
}

#[tokio::test]
async fn test_anthropic_rate_limit_429() {
    let (mut server, adapter) = setup_mock_server().await;

    let error_response = r#"{
        "type": "error",
        "error": {
            "type": "rate_limit_error",
            "message": "Number of request tokens has exceeded your per-minute rate limit"
        }
    }"#;

    let _mock = server
        .mock("POST", "/messages")
        .with_status(429)
        .with_header("content-type", "application/json")
        .with_body(error_response)
        .create_async()
        .await;

    let request = create_user_request("Hello");
    let response = adapter.generate(request).await;

    assert!(response.is_err());
    let error = response.unwrap_err();
    assert!(matches!(
        error,
        paladin_ports::output::llm_port::LlmError::RateLimitExceeded
    ));
}

#[tokio::test]
async fn test_anthropic_invalid_request_400() {
    let (mut server, adapter) = setup_mock_server().await;

    let error_response = r#"{
        "type": "error",
        "error": {
            "type": "invalid_request_error",
            "message": "messages: text content blocks must be non-empty"
        }
    }"#;

    let _mock = server
        .mock("POST", "/messages")
        .with_status(400)
        .with_header("content-type", "application/json")
        .with_body(error_response)
        .create_async()
        .await;

    let request = create_user_request("");
    let response = adapter.generate(request).await;

    assert!(response.is_err());
    let error = response.unwrap_err();
    assert!(matches!(
        error,
        paladin_ports::output::llm_port::LlmError::InvalidPrompt(_)
    ));
}

#[tokio::test]
async fn test_anthropic_server_error_500() {
    let (mut server, adapter) = setup_mock_server().await;

    let error_response = r#"{
        "type": "error",
        "error": {
            "type": "api_error",
            "message": "Internal server error"
        }
    }"#;

    let _mock = server
        .mock("POST", "/messages")
        .with_status(500)
        .with_header("content-type", "application/json")
        .with_body(error_response)
        .create_async()
        .await;

    let request = create_user_request("Hello");
    let response = adapter.generate(request).await;

    assert!(response.is_err());
    // Phase 25 (FT-FR-01, D-03): a 5xx is a typed `ProviderError` whose
    // status is read from the field, never from rendered text.
    match response.unwrap_err() {
        paladin_ports::output::llm_port::LlmError::ProviderError {
            provider, status, ..
        } => {
            assert_eq!(provider, "anthropic");
            assert_eq!(status, 500);
        }
        other => panic!("expected ProviderError {{ status: 500 }}, got {other:?}"),
    }
}

#[tokio::test]
async fn test_anthropic_client_refuses_to_follow_a_redirect() {
    // CR-02 (`25-REVIEW.md`): the adapter sends its credential as a
    // custom `x-api-key` header, which reqwest's built-in cross-host
    // redirect header-stripping does NOT cover. This must surface as a
    // refused-redirect `ProviderError`, and the redirect target must never
    // receive a request — proving the header was never replayed rather
    // than merely asserting on the returned error shape.
    let (mut server, adapter) = setup_mock_server().await;

    let redirect_target = server
        .mock("POST", "/redirected")
        .expect(0)
        .create_async()
        .await;
    let _mock = server
        .mock("POST", "/messages")
        .with_status(302)
        .with_header("Location", "/redirected")
        .with_body("moved")
        .create_async()
        .await;

    let request = create_user_request("Hello");
    let response = adapter.generate(request).await;

    assert!(response.is_err());
    match response.unwrap_err() {
        paladin_ports::output::llm_port::LlmError::ProviderError {
            provider,
            status,
            message,
        } => {
            assert_eq!(provider, "anthropic");
            assert_eq!(status, 302);
            assert!(message.contains("redirect"), "got: {message}");
        }
        other => panic!("expected ProviderError {{ status: 302 }}, got {other:?}"),
    }
    redirect_target.assert_async().await;
}

#[tokio::test]
async fn test_anthropic_malformed_response() {
    let (mut server, adapter) = setup_mock_server().await;

    let malformed_response = r#"{"invalid": "json", "missing": "content"}"#;

    let _mock = server
        .mock("POST", "/messages")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(malformed_response)
        .create_async()
        .await;

    let request = create_user_request("Hello");
    let response = adapter.generate(request).await;

    assert!(response.is_err());
    let error = response.unwrap_err();
    assert!(matches!(
        error,
        paladin_ports::output::llm_port::LlmError::ProcessingError(_)
    ));
}
