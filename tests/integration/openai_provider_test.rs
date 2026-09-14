//! OpenAI Provider Integration Tests
//!
//! Tests for OpenAI adapter integration with real API calls (requires API key).
//! These tests are gated behind the integration-tests feature flag.

#[cfg(all(test, feature = "integration-tests"))]
mod openai_integration_tests {
    use paladin::core::platform::container::prompt::{PromptItem, PromptType, SystemPrompt};
    use paladin::{OpenAIAdapter, OpenAIConfig};
    use paladin_ports::output::llm_port::{FinishReason, LlmPort, LlmRequest};
    use std::collections::HashMap;
    use std::env;

    /// Helper to create OpenAI adapter from environment
    fn create_openai_adapter() -> OpenAIAdapter {
        let api_key = env::var("OPENAI_API_KEY")
            .expect("OPENAI_API_KEY must be set for OpenAI integration tests");

        let config = OpenAIConfig::new(api_key);
        OpenAIAdapter::new(config).expect("Failed to create OpenAI adapter")
    }

    #[tokio::test]
    #[ignore] // Requires API key and makes real API calls
    async fn test_openai_simple_completion() {
        let adapter = create_openai_adapter();

        let prompt_type = PromptType::System(SystemPrompt {
            instructions: "Say 'Hello, Paladin!' and nothing else.".to_string(),
            constraints: None,
        });
        let prompt = PromptItem::new(prompt_type).expect("Failed to create prompt");

        let request = LlmRequest::new("gpt-3.5-turbo", prompt);

        let request_id = request.id;
        let result = adapter.generate(request).await;
        assert!(result.is_ok(), "OpenAI API call failed: {:?}", result.err());

        let response = result.unwrap();
        assert!(
            !response.content.is_empty(),
            "Response content should not be empty"
        );
        assert!(
            response.content.to_lowercase().contains("hello"),
            "Response should contain 'hello'"
        );
        assert_eq!(response.request_id, request_id);
        assert!(matches!(response.finish_reason, FinishReason::Stop));
    }

    #[tokio::test]
    #[ignore] // Requires API key and makes real API calls
    async fn test_openai_function_calling() {
        let adapter = create_openai_adapter();

        let prompt_type = PromptType::System(SystemPrompt {
            instructions: "What's the weather in San Francisco?".to_string(),
            constraints: None,
        });
        let prompt = PromptItem::new(prompt_type).expect("Failed to create prompt");

        let mut meta = HashMap::new();
        meta.insert(
            "functions".to_string(),
            r#"[{"name": "get_weather", "description": "Get weather for a location", "parameters": {"type": "object", "properties": {"location": {"type": "string"}}}}]"#.to_string(),
        );
        let request = LlmRequest::new("gpt-3.5-turbo", prompt).with_metadata(meta);

        let result = adapter.generate(request).await;
        assert!(
            result.is_ok(),
            "OpenAI function call failed: {:?}",
            result.err()
        );

        let response = result.unwrap();
        // Function calling should return either a function call or content
        let has_function = response.function_call.is_some();
        let has_content = !response.content.is_empty();
        assert!(
            has_function || has_content,
            "Response should have either function call or content"
        );

        if let Some(func) = response.function_call {
            assert_eq!(func.name, "get_weather");
            assert!(!func.arguments.is_empty());
        }
    }

    #[tokio::test]
    #[ignore] // Requires API key and makes real API calls
    async fn test_openai_token_usage() {
        let adapter = create_openai_adapter();

        let prompt_type = PromptType::System(SystemPrompt {
            instructions: "Count to 5.".to_string(),
            constraints: None,
        });
        let prompt = PromptItem::new(prompt_type).expect("Failed to create prompt");

        let request = LlmRequest::new("gpt-3.5-turbo", prompt);

        let response = adapter.generate(request).await.unwrap();

        // Verify token usage tracking
        assert!(
            response.usage.prompt_tokens > 0,
            "Prompt tokens should be tracked"
        );
        assert!(
            response.usage.completion_tokens > 0,
            "Completion tokens should be tracked"
        );
        assert_eq!(
            response.usage.total_tokens,
            response.usage.prompt_tokens + response.usage.completion_tokens,
            "Total tokens should equal sum of prompt and completion"
        );
    }

    #[tokio::test]
    #[ignore] // Requires API key and makes real API calls
    async fn test_openai_model_validation() {
        let adapter = create_openai_adapter();

        // Test with an invalid/non-existent model
        let prompt_type = PromptType::System(SystemPrompt {
            instructions: "Test".to_string(),
            constraints: None,
        });
        let prompt = PromptItem::new(prompt_type).expect("Failed to create prompt");

        let request = LlmRequest::new("gpt-nonexistent-model", prompt);

        let result = adapter.generate(request).await;
        assert!(result.is_err(), "Should fail with non-existent model");
    }

    #[tokio::test]
    #[ignore] // Requires API key and makes real API calls
    async fn test_openai_with_system_message() {
        let adapter = create_openai_adapter();

        let prompt_type = PromptType::System(SystemPrompt {
            instructions: "What is your purpose?".to_string(),
            constraints: Some(vec![
                "You are a helpful AI assistant specializing in Rust programming.".to_string(),
            ]),
        });
        let prompt = PromptItem::new(prompt_type).expect("Failed to create prompt");

        let request = LlmRequest::new("gpt-3.5-turbo", prompt);

        let response = adapter.generate(request).await.unwrap();
        assert!(!response.content.is_empty());
        // The response might reference being an assistant or helping with Rust
        // but we can't guarantee exact content, so just verify we got a response
    }
}
