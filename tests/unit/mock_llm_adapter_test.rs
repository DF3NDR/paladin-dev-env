// tests/unit/mock_llm_adapter_test.rs
//
// Comprehensive unit tests for MockLlmAdapter following TDD methodology

use paladin::MockLlmAdapter;
use paladin::core::base::entity::node::Node;
use paladin::core::platform::container::prompt::{
    PromptData, PromptItem, PromptParameters, PromptType, UserPrompt,
};
use paladin_ports::output::llm_port::{
    FinishReason, LlmError, LlmPort, LlmRequest, ResponseFormat,
};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn test_mock_llm_adapter_simple_success() {
    // Test basic successful generation
    let adapter = MockLlmAdapter::new().with_response("Hello, world!".to_string());

    let request = create_test_request();
    let result = adapter.generate(request).await;

    assert!(result.is_ok(), "Should generate successfully");
    let response = result.unwrap();
    assert_eq!(response.content, "Hello, world!");
    assert!(matches!(response.finish_reason, FinishReason::Stop));
    assert!(response.usage.total_tokens > 0);
}

#[tokio::test]
async fn test_mock_llm_adapter_multiple_responses() {
    // Test sequential responses from queue
    let adapter = MockLlmAdapter::new().with_responses(vec![
        "Response 1".to_string(),
        "Response 2".to_string(),
        "Response 3".to_string(),
    ]);

    let request1 = create_test_request();
    let response1 = adapter.generate(request1).await.unwrap();
    assert_eq!(response1.content, "Response 1");

    let request2 = create_test_request();
    let response2 = adapter.generate(request2).await.unwrap();
    assert_eq!(response2.content, "Response 2");

    let request3 = create_test_request();
    let response3 = adapter.generate(request3).await.unwrap();
    assert_eq!(response3.content, "Response 3");
}

#[tokio::test]
async fn test_mock_llm_adapter_simulates_error() {
    // Test error simulation
    let adapter = MockLlmAdapter::new().with_error(LlmError::RateLimitExceeded);

    let request = create_test_request();
    let result = adapter.generate(request).await;

    assert!(result.is_err(), "Should fail with configured error");
    assert!(matches!(result.unwrap_err(), LlmError::RateLimitExceeded));
}

#[tokio::test]
async fn test_mock_llm_adapter_respects_delay() {
    // Test that delays are applied
    let adapter = MockLlmAdapter::new()
        .with_response("Delayed response".to_string())
        .with_delay(Duration::from_millis(100));

    let request = create_test_request();
    let start = std::time::Instant::now();
    let result = adapter.generate(request).await;
    let elapsed = start.elapsed();

    assert!(result.is_ok(), "Should succeed after delay");
    assert!(
        elapsed >= Duration::from_millis(100),
        "Should take at least 100ms, took {:?}",
        elapsed
    );
}

#[tokio::test]
async fn test_mock_llm_adapter_tracks_calls() {
    // Test call tracking
    let adapter = Arc::new(MockLlmAdapter::new().with_response("Test".to_string()));

    // Make multiple calls
    for _ in 0..5 {
        let request = create_test_request();
        let _ = adapter.generate(request).await;
    }

    assert_eq!(adapter.get_call_count(), 5, "Should track all calls");
}

#[tokio::test]
async fn test_mock_llm_adapter_streaming() {
    // Test streaming generation - just verify it returns without error
    // Actually consuming the stream requires complex Pin/Unpin handling
    let adapter = MockLlmAdapter::new().with_response("Hello world!".to_string());

    let request = create_test_request();
    let stream_result = adapter.generate_stream(request).await;

    assert!(stream_result.is_ok(), "Should create stream");

    // Stream creation tested - actual consumption would require Pin handling
    // which is tested in integration tests
}

#[tokio::test]
async fn test_mock_llm_adapter_validates_model() {
    // Test model validation
    let adapter = MockLlmAdapter::new()
        .with_available_models(vec!["model-1".to_string(), "model-2".to_string()]);

    assert!(adapter.validate_model("model-1").await.unwrap());
    assert!(adapter.validate_model("model-2").await.unwrap());
    assert!(!adapter.validate_model("model-3").await.unwrap());
}

#[tokio::test]
async fn test_mock_llm_adapter_get_available_models() {
    // Test getting available models
    let models = vec!["gpt-4".to_string(), "gpt-3.5-turbo".to_string()];

    let adapter = MockLlmAdapter::new().with_available_models(models.clone());

    let result = adapter.get_available_models().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), models);
}

#[tokio::test]
async fn test_mock_llm_adapter_provider_name() {
    // Test provider name
    let adapter = MockLlmAdapter::new();
    assert_eq!(adapter.get_provider_name(), "MockLLM");
}

#[tokio::test]
async fn test_mock_llm_adapter_custom_token_usage() {
    // Test custom token usage configuration
    let adapter = MockLlmAdapter::new()
        .with_response("Test response".to_string())
        .with_token_usage(100, 50, 150);

    let request = create_test_request();
    let response = adapter.generate(request).await.unwrap();

    assert_eq!(response.usage.prompt_tokens, 100);
    assert_eq!(response.usage.completion_tokens, 50);
    assert_eq!(response.usage.total_tokens, 150);
}

#[tokio::test]
async fn test_mock_llm_adapter_custom_finish_reason() {
    // Test custom finish reason
    let adapter = MockLlmAdapter::new()
        .with_response("Test response".to_string())
        .with_finish_reason(FinishReason::Length);

    let request = create_test_request();
    let response = adapter.generate(request).await.unwrap();

    assert!(matches!(response.finish_reason, FinishReason::Length));
}

#[tokio::test]
async fn test_mock_llm_adapter_reset() {
    // Test resetting call count and state
    let adapter = Arc::new(MockLlmAdapter::new().with_response("Test".to_string()));

    // Make some calls
    for _ in 0..3 {
        let request = create_test_request();
        let _ = adapter.generate(request).await;
    }

    assert_eq!(adapter.get_call_count(), 3);

    adapter.reset();
    assert_eq!(adapter.get_call_count(), 0, "Should reset call count");
}

#[tokio::test]
async fn test_mock_llm_adapter_concurrent_access() {
    // Test thread-safe concurrent access
    let adapter = Arc::new(MockLlmAdapter::new().with_response("Concurrent response".to_string()));

    let mut handles = vec![];
    for _ in 0..10 {
        let adapter_clone = Arc::clone(&adapter);
        let handle = tokio::spawn(async move {
            let request = create_test_request();
            adapter_clone.generate(request).await
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "Concurrent call should succeed");
    }

    assert_eq!(
        adapter.get_call_count(),
        10,
        "Should track all concurrent calls"
    );
}

#[tokio::test]
async fn test_mock_llm_adapter_error_then_success() {
    // Test error followed by success using the convenience method
    let adapter = MockLlmAdapter::new().with_error_then_response(
        LlmError::NetworkError("Connection failed".to_string()),
        "Success after error".to_string(),
    );

    // First call should fail
    let request1 = create_test_request();
    let result1 = adapter.generate(request1).await;
    assert!(result1.is_err());

    // Second call should succeed
    let request2 = create_test_request();
    let result2 = adapter.generate(request2).await;
    assert!(result2.is_ok());
    assert_eq!(result2.unwrap().content, "Success after error");
}

#[tokio::test]
async fn test_mock_llm_adapter_default_values() {
    // Test default values when no configuration provided
    let adapter = MockLlmAdapter::new();

    let request = create_test_request();
    let response = adapter.generate(request).await.unwrap();

    // Should have some default response
    assert!(!response.content.is_empty());
    assert!(response.usage.total_tokens > 0);
    assert_eq!(response.model, "mock-model");
}

/// D-28 / X-03 (plan 26-03): `response_format` is a new, additive field that
/// no adapter reads yet -- it stays inert on the wire until plan 26-06 wires
/// the four native JSON-mode paths. `MockLlmAdapter` stands in for "any
/// shipped adapter" here: attaching `response_format` must not change
/// dispatch or the resulting response in any observable way.
#[tokio::test]
async fn all_adapters_ignore_response_format_until_wired() {
    let adapter = MockLlmAdapter::new().with_response("Hello, world!".to_string());

    let plain_response = adapter
        .generate(create_test_request())
        .await
        .expect("plain request must succeed");

    let structured_request = create_test_request().with_response_format(ResponseFormat::JsonObject);
    let structured_response = adapter
        .generate(structured_request)
        .await
        .expect("a request carrying response_format must succeed identically");

    assert_eq!(plain_response.content, structured_response.content);
    assert_eq!(
        plain_response.usage.total_tokens,
        structured_response.usage.total_tokens
    );
    assert_eq!(
        format!("{:?}", plain_response.finish_reason),
        format!("{:?}", structured_response.finish_reason),
        "attaching response_format must not change finish_reason on an adapter \
         that does not read it (plan 26-06 wires this later)"
    );
}

// Helper function to create a test request
fn create_test_request() -> LlmRequest {
    let prompt_data = PromptData {
        prompt_type: PromptType::User(UserPrompt {
            query: "Test prompt".to_string(),
            context: None,
        }),
        content_attachments: vec![],
        parameters: PromptParameters {
            max_tokens: None,
            temperature: Some(0.7),
            stop_sequences: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
        },
        context: None,
        expected_output: None,
        tags: None,
        category: None,
        author: None,
        metadata: BTreeMap::new(),
    };

    let prompt_item = PromptItem {
        node: Node::new(prompt_data, Some("test-prompt".to_string())),
    };

    LlmRequest::new("mock-model", prompt_item)
}
