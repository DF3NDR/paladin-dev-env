// tests/unit/llm/deepseek_adapter_test.rs
//
// Unit tests for DeepSeek adapter with mocked HTTP responses

use mockito::{Server, ServerGuard};
use paladin::core::platform::container::prompt::{PromptItem, PromptType, SystemPrompt};
use paladin_llm::deepseek::{DeepSeekAdapter, DeepSeekConfig};
use paladin_ports::output::llm_port::{LlmPort, LlmRequest};

/// Helper to create a mock server and adapter configured to use it
///
/// Uses the async server constructor: the blocking, synchronous
/// `Server::new()` panics with a runtime-nesting error when called from
/// inside an already-running Tokio runtime, which every call site here is
/// (`#[tokio::test]`).
async fn setup_mock_server() -> (ServerGuard, DeepSeekAdapter) {
    let server = Server::new_async().await;
    let config = DeepSeekConfig {
        api_key: "test-api-key".to_string(),
        base_url: server.url(),
        model: "deepseek-chat".to_string(),
        timeout_seconds: 30,
    };
    let adapter = DeepSeekAdapter::new(config).unwrap();
    (server, adapter)
}

/// Helper to create a basic LLM request
fn create_test_request(content: &str) -> LlmRequest {
    // PromptItem::new returns a Result; construction here uses fixed, valid
    // inputs, so unwrap cannot fail.
    let system_prompt = PromptItem::new(PromptType::System(SystemPrompt {
        instructions: content.to_string(),
        constraints: None,
    }))
    .unwrap();

    LlmRequest::new("deepseek-chat", system_prompt)
}

#[tokio::test]
async fn test_deepseek_successful_completion() {
    let (mut server, adapter) = setup_mock_server().await;

    let mock_response = r#"{
        "id": "chatcmpl-123",
        "object": "chat.completion",
        "created": 1677652288,
        "model": "deepseek-chat",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "Hello! How can I help you today?"
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 20,
            "total_tokens": 30
        }
    }"#;

    let _mock = server
        .mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(mock_response)
        .create_async()
        .await;

    let request = create_test_request("Hello");
    let response = adapter.generate(request).await;

    assert!(response.is_ok());
    let response = response.unwrap();
    assert_eq!(response.content, "Hello! How can I help you today?");
    assert_eq!(response.usage.prompt_tokens, 10);
    assert_eq!(response.usage.completion_tokens, 20);
    assert_eq!(response.usage.total_tokens, 30);
}

#[tokio::test]
async fn test_deepseek_streaming_response() {
    let (mut server, adapter) = setup_mock_server().await;

    // Mock SSE streaming response
    let mock_stream = "data: {\"id\":\"chatcmpl-123\",\"object\":\"chat.completion.chunk\",\"created\":1677652288,\"model\":\"deepseek-chat\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hello\"},\"finish_reason\":null}]}\n\ndata: {\"id\":\"chatcmpl-123\",\"object\":\"chat.completion.chunk\",\"created\":1677652288,\"model\":\"deepseek-chat\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" world\"},\"finish_reason\":null}]}\n\ndata: {\"id\":\"chatcmpl-123\",\"object\":\"chat.completion.chunk\",\"created\":1677652288,\"model\":\"deepseek-chat\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n";

    let _mock = server
        .mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "text/event-stream")
        .with_body(mock_stream)
        .create_async()
        .await;

    let request = create_test_request("Hello");
    let stream_result = adapter.generate_stream(request).await;

    assert!(stream_result.is_ok());
    // Streaming validation would require consuming the stream
    // For now, we just verify it doesn't error
}

#[tokio::test]
async fn test_deepseek_auth_failure_401() {
    let (mut server, adapter) = setup_mock_server().await;

    let error_response = r#"{
        "error": {
            "message": "Invalid authentication",
            "type": "invalid_request_error",
            "code": "invalid_api_key"
        }
    }"#;

    let _mock = server
        .mock("POST", "/chat/completions")
        .with_status(401)
        .with_header("content-type", "application/json")
        .with_body(error_response)
        .create_async()
        .await;

    let request = create_test_request("Hello");
    let response = adapter.generate(request).await;

    assert!(response.is_err());
    let error = response.unwrap_err();
    assert!(matches!(
        error,
        paladin_ports::output::llm_port::LlmError::AuthenticationError(_)
    ));
}

#[tokio::test]
async fn test_deepseek_rate_limit_429() {
    let (mut server, adapter) = setup_mock_server().await;

    let error_response = r#"{
        "error": {
            "message": "Rate limit exceeded",
            "type": "rate_limit_error"
        }
    }"#;

    let _mock = server
        .mock("POST", "/chat/completions")
        .with_status(429)
        .with_header("content-type", "application/json")
        .with_body(error_response)
        .create_async()
        .await;

    let request = create_test_request("Hello");
    let response = adapter.generate(request).await;

    assert!(response.is_err());
    let error = response.unwrap_err();
    assert!(matches!(
        error,
        paladin_ports::output::llm_port::LlmError::RateLimitExceeded
    ));
}

#[tokio::test]
async fn test_deepseek_timeout() {
    // A mockito server responds immediately (with 501) to any unmocked
    // request, so it cannot exercise a real client-side timeout. Instead,
    // bind a raw TCP listener and never accept()/respond on it: the TCP
    // handshake completes (the kernel backlog accepts SYNs once `listen()`
    // is active), the client's HTTP request bytes are buffered, and no
    // response is ever produced -- so the request hangs until the 1-second
    // timeout below fires for real.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    let config = DeepSeekConfig {
        api_key: "test-api-key".to_string(),
        base_url: format!("http://{addr}"),
        model: "deepseek-chat".to_string(),
        timeout_seconds: 1,
    };
    let adapter = DeepSeekAdapter::new(config).unwrap();

    let request = create_test_request("Hello");
    let response = adapter.generate(request).await;
    // Keep the listener alive until after the request has had its chance to
    // time out, so the connection is not torn down early.
    drop(listener);

    assert!(response.is_err());
    // The adapter now maps reqwest's timeout errors to a dedicated
    // LlmError::Timeout variant (previously indistinguishable from
    // LlmError::NetworkError) so callers can retry with a longer timeout
    // rather than treating every network failure identically.
    let error = response.unwrap_err();
    assert!(matches!(
        error,
        paladin_ports::output::llm_port::LlmError::Timeout(_)
    ));
}

#[tokio::test]
async fn test_deepseek_invalid_model_error() {
    let (mut server, adapter) = setup_mock_server().await;

    let error_response = r#"{
        "error": {
            "message": "The model 'invalid-model' does not exist",
            "type": "invalid_request_error",
            "code": "model_not_found"
        }
    }"#;

    let _mock = server
        .mock("POST", "/chat/completions")
        .with_status(400)
        .with_header("content-type", "application/json")
        .with_body(error_response)
        .create_async()
        .await;

    let request = create_test_request("Hello");
    let response = adapter.generate(request).await;

    assert!(response.is_err());
    let error = response.unwrap_err();
    assert!(matches!(
        error,
        paladin_ports::output::llm_port::LlmError::InvalidPrompt(_)
    ));
}

#[tokio::test]
async fn test_deepseek_server_error_500() {
    let (mut server, adapter) = setup_mock_server().await;

    let error_response = r#"{
        "error": {
            "message": "Internal server error",
            "type": "server_error"
        }
    }"#;

    let _mock = server
        .mock("POST", "/chat/completions")
        .with_status(500)
        .with_header("content-type", "application/json")
        .with_body(error_response)
        .create_async()
        .await;

    let request = create_test_request("Hello");
    let response = adapter.generate(request).await;

    assert!(response.is_err());
    // Phase 25 (FT-FR-01, D-03): a 5xx is a typed `ProviderError` whose
    // status is read from the field, never from rendered text.
    match response.unwrap_err() {
        paladin_ports::output::llm_port::LlmError::ProviderError {
            provider, status, ..
        } => {
            assert_eq!(provider, "deepseek");
            assert_eq!(status, 500);
        }
        other => panic!("expected ProviderError {{ status: 500 }}, got {other:?}"),
    }
}

#[tokio::test]
async fn test_deepseek_client_refuses_to_follow_a_redirect() {
    // CR-02 (`25-REVIEW.md`): a `302` from the configured base URL must
    // surface as a refused-redirect `ProviderError`, and the redirect
    // target must never receive a request — proving the `Authorization`
    // header was never replayed rather than merely asserting on the
    // returned error shape. `generate` retries a `ProviderError` up to its
    // hardcoded `max_retries` (3), so the redirect mock is hit more than
    // once, but the redirect TARGET must still be hit zero times across
    // every attempt.
    let (mut server, adapter) = setup_mock_server().await;

    let redirect_target = server
        .mock("POST", "/redirected")
        .expect(0)
        .create_async()
        .await;
    let _mock = server
        .mock("POST", "/chat/completions")
        .with_status(302)
        .with_header("Location", "/redirected")
        .with_body("moved")
        .create_async()
        .await;

    let request = create_test_request("Hello");
    let response = adapter.generate(request).await;

    assert!(response.is_err());
    match response.unwrap_err() {
        paladin_ports::output::llm_port::LlmError::ProviderError {
            provider,
            status,
            message,
        } => {
            assert_eq!(provider, "deepseek");
            assert_eq!(status, 302);
            assert!(message.contains("redirect"), "got: {message}");
        }
        other => panic!("expected ProviderError {{ status: 302 }}, got {other:?}"),
    }
    redirect_target.assert_async().await;
}

#[tokio::test]
async fn test_deepseek_malformed_response() {
    let (mut server, adapter) = setup_mock_server().await;

    let malformed_response = r#"{"invalid": "json", "missing": "required_fields"}"#;

    let _mock = server
        .mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(malformed_response)
        .create_async()
        .await;

    let request = create_test_request("Hello");
    let response = adapter.generate(request).await;

    assert!(response.is_err());
    let error = response.unwrap_err();
    assert!(matches!(
        error,
        paladin_ports::output::llm_port::LlmError::ProcessingError(_)
    ));
}
