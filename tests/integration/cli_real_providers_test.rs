//! CLI Integration Tests - Tier 3: API-Key-Gated Provider Tests
//!
//! These tests require real LLM API keys:
//! - OPENAI_API_KEY for OpenAI tests
//! - ANTHROPIC_API_KEY for Anthropic tests
//! - DEEPSEEK_API_KEY for DeepSeek tests
//!
//! Run with: `cargo test --test lib --features integration-tests integration::cli_real_providers_test -- --ignored`
//!
//! Tests are gated with both `#[cfg(feature = "integration-tests")]` and `#[ignore]`.

#[cfg(all(test, feature = "integration-tests"))]
mod provider_tests {
    use futures::StreamExt;
    use paladin::LlmProviderFactory;
    use paladin::application::cli::config::loader::load_paladin_config;
    use paladin::core::platform::container::prompt::{PromptItem, PromptType, SystemPrompt};
    use paladin_ports::output::llm_port::LlmRequest;
    use std::env;
    use std::fs;
    use tempfile::TempDir;

    /// Helper to create temp config file
    fn write_config(dir: &TempDir, name: &str, content: &str) -> std::path::PathBuf {
        let path = dir.path().join(format!("{}.yaml", name));
        fs::write(&path, content).expect("Failed to write config");
        path
    }

    /// Helper to create a test prompt
    fn create_test_prompt(content: &str) -> PromptItem {
        let prompt_type = PromptType::System(SystemPrompt {
            instructions: content.to_string(),
            constraints: None,
        });
        PromptItem::new(prompt_type).expect("Failed to create prompt")
    }

    /// Helper to create a test request
    fn create_test_request(prompt: PromptItem, model: &str) -> LlmRequest {
        LlmRequest::new(model, prompt)
    }

    // =========================================================================
    // 5.3.1 & 5.3.2: OpenAI provider tests
    // =========================================================================

    #[tokio::test]
    #[ignore = "Requires OPENAI_API_KEY environment variable"]
    async fn test_openai_provider_connection() {
        let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set for this test");

        assert!(!api_key.is_empty(), "OPENAI_API_KEY must not be empty");

        // Create provider via factory
        let factory = LlmProviderFactory::new();
        let provider = factory.create("openai");
        assert!(
            provider.is_ok(),
            "OpenAI provider should be created successfully"
        );

        // Validate connection with a simple request
        let provider = provider.unwrap();
        let prompt = create_test_prompt("Say 'hello' and nothing else.");
        let request = create_test_request(prompt, "gpt-3.5-turbo");

        let result = provider.generate(request).await;

        assert!(
            result.is_ok(),
            "OpenAI should respond successfully. Error: {:?}",
            result.err()
        );

        let response = result.unwrap();
        assert!(
            !response.content.is_empty(),
            "OpenAI response should not be empty"
        );
        println!("OpenAI response: {}", response.content);
    }

    #[tokio::test]
    #[ignore = "Requires OPENAI_API_KEY environment variable"]
    async fn test_openai_agent_config_end_to_end() {
        let _api_key =
            env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set for this test");

        let temp_dir = TempDir::new().unwrap();
        let config_path = write_config(
            &temp_dir,
            "openai_agent",
            r#"
name: "test-openai-agent"
system_prompt: "You are a concise test assistant. Respond in one word."
model: "gpt-3.5-turbo"
temperature: 0.1
max_loops: 1

provider:
  type: "openai"
"#,
        );

        // Load and validate config
        let config = load_paladin_config(&config_path).expect("Config should load successfully");

        assert_eq!(config.name, "test-openai-agent");
        assert_eq!(config.provider.provider_type, "openai");

        // Create provider and verify it can generate
        let factory = LlmProviderFactory::new();
        let provider = factory
            .create("openai")
            .expect("Provider should be created");

        let prompt = create_test_prompt(&format!("{}\n\nUser: What is 2+2?", config.system_prompt));
        let request = create_test_request(prompt, &config.model);

        let result = provider.generate(request).await;

        assert!(result.is_ok(), "End-to-end agent execution should succeed");
    }

    // =========================================================================
    // 5.3.3 & 5.3.4: Anthropic provider tests
    // =========================================================================

    #[tokio::test]
    #[ignore = "Requires ANTHROPIC_API_KEY environment variable"]
    async fn test_anthropic_provider_connection() {
        let api_key =
            env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY must be set for this test");

        assert!(!api_key.is_empty(), "ANTHROPIC_API_KEY must not be empty");

        let factory = LlmProviderFactory::new();
        let provider = factory.create("anthropic");
        assert!(
            provider.is_ok(),
            "Anthropic provider should be created successfully"
        );

        let provider = provider.unwrap();
        let prompt = create_test_prompt("Say 'hello' and nothing else.");
        let request = create_test_request(prompt, "claude-3-5-sonnet-20241022");

        let result = provider.generate(request).await;

        assert!(
            result.is_ok(),
            "Anthropic should respond successfully. Error: {:?}",
            result.err()
        );

        let response = result.unwrap();
        assert!(
            !response.content.is_empty(),
            "Anthropic response should not be empty"
        );
        println!("Anthropic response: {}", response.content);
    }

    // =========================================================================
    // 5.3.5 & 5.3.6: DeepSeek provider tests
    // =========================================================================

    #[tokio::test]
    #[ignore = "Requires DEEPSEEK_API_KEY environment variable"]
    async fn test_deepseek_provider_connection() {
        let api_key =
            env::var("DEEPSEEK_API_KEY").expect("DEEPSEEK_API_KEY must be set for this test");

        assert!(!api_key.is_empty(), "DEEPSEEK_API_KEY must not be empty");

        let factory = LlmProviderFactory::new();
        let provider = factory.create("deepseek");
        assert!(
            provider.is_ok(),
            "DeepSeek provider should be created successfully"
        );

        let provider = provider.unwrap();
        let prompt = create_test_prompt("Say 'hello' and nothing else.");
        let request = create_test_request(prompt, "deepseek-chat");

        let result = provider.generate(request).await;

        assert!(
            result.is_ok(),
            "DeepSeek should respond successfully. Error: {:?}",
            result.err()
        );

        let response = result.unwrap();
        assert!(
            !response.content.is_empty(),
            "DeepSeek response should not be empty"
        );
        println!("DeepSeek response: {}", response.content);
    }

    // =========================================================================
    // 5.3.8: Test streaming response handling
    // =========================================================================

    #[tokio::test]
    #[ignore = "Requires OPENAI_API_KEY environment variable"]
    async fn test_openai_streaming_response() {
        let _api_key =
            env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set for this test");

        let factory = LlmProviderFactory::new();
        let provider = factory
            .create("openai")
            .expect("Provider should be created");

        let prompt = create_test_prompt("Count from 1 to 5, separated by commas.");
        let mut request = create_test_request(prompt, "gpt-3.5-turbo");
        request.stream = true;

        let result = provider.generate_stream(request).await;

        assert!(
            result.is_ok(),
            "Streaming should start successfully. Error: {:?}",
            result.err()
        );

        let mut stream = result.unwrap();
        let mut collected = String::new();
        let mut chunk_count = 0;

        // SAFETY: The boxed stream is never moved after this point
        while let Some(chunk_result) = unsafe { std::pin::Pin::new_unchecked(&mut *stream) }
            .next()
            .await
        {
            match chunk_result {
                Ok(chunk) => {
                    if !chunk.delta.is_empty() {
                        collected.push_str(&chunk.delta);
                        chunk_count += 1;
                    }
                }
                Err(e) => {
                    panic!("Stream error: {:?}", e);
                }
            }
        }

        assert!(!collected.is_empty(), "Streaming should produce output");
        assert!(chunk_count > 0, "Should receive at least one chunk");
        println!("Streaming result ({} chunks): {}", chunk_count, collected);
    }
}
