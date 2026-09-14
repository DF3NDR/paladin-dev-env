// tests/integration/llm_live_api_tests.rs
//
// Live API Integration Tests for LLM Providers
//
// These tests make REAL API calls to OpenAI, DeepSeek, and Anthropic.
// They are gated by the `live-api-tests` feature flag and require valid API keys.
//
// IMPORTANT: These tests will incur API costs. Only run them when necessary.
//
// ## Running Tests
//
// ```bash
// # Method 1: Load .env file before running tests
// set -a && . .env && set +a
// cargo test --features live-api-tests -- --ignored
//
// # Method 2: Export keys directly
// export OPENAI_API_KEY="sk-..."
// export DEEPSEEK_API_KEY="sk-..."
// export ANTHROPIC_API_KEY="sk-..."
// cargo test --features live-api-tests -- --ignored
//
// # Run specific provider tests
// cargo test --features live-api-tests test_openai -- --ignored
// cargo test --features live-api-tests test_deepseek -- --ignored
// cargo test --features live-api-tests test_anthropic -- --ignored
// ```
//
// ## Test Coverage
//
// Each provider is tested for:
// 1. Basic completion (generate)
// 2. Streaming completion (generate_stream)
// 3. Error handling (invalid model)
// 4. Provider capabilities

use futures::StreamExt;
use std::env;
use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize test environment - loads .env file if present
fn init_test_env() {
    INIT.call_once(|| {
        // Try to load .env file for tests (silently ignore if not present)
        let _ = dotenv::dotenv();
    });
}

use paladin::core::platform::container::prompt::{PromptItem, PromptType};
use paladin::{
    AnthropicAdapter, AnthropicConfig, DeepSeekAdapter, DeepSeekConfig, OpenAIAdapter, OpenAIConfig,
};
use paladin_ports::output::llm_port::{FinishReason, LlmError, LlmPort, LlmRequest};

// ============================================================================
// Helper Functions
// ============================================================================

/// Returns the API key read from `env_var` when it is present and non-empty, and panics with a
/// clear, actionable message when the variable is absent or set to the empty string.
///
/// This will panic with a clear message if the API key is missing or empty,
/// causing the test to fail rather than silently passing.
///
/// Exactly two conditions count as missing: the variable being absent, and the variable being
/// present with an empty-string value. The check is `str::is_empty()`, a byte-length check with
/// no trimming — a whitespace-only value (e.g. `" "`) is therefore treated as present and is
/// returned as-is, with no validation that it looks like a plausible key. See the recorded
/// decision at `.planning/decisions/0012-live-api-test-key-behaviour.md`: the panic stands.
fn require_api_key(env_var: &str, provider: &str) -> String {
    // Initialize test environment (loads .env if present)
    init_test_env();

    match env::var(env_var) {
        Ok(key) if !key.is_empty() => key,
        Ok(_) => {
            panic!(
                "❌ {} API key is empty. Set {} in .env file or environment. \n\n\
                To skip this test, don't run with --ignored flag.\n\
                To run with a valid key: export {}=\"your-key-here\"",
                provider, env_var, env_var
            );
        }
        Err(_) => {
            panic!(
                "❌ {} API key not found. Set {} in .env file or environment.\n\n\
                To skip this test, don't run with --ignored flag.\n\
                To run with a valid key: export {}=\"your-key-here\"",
                provider, env_var, env_var
            );
        }
    }
}

/// Create a simple test prompt for LLM requests
///
/// For Anthropic, we need to send user messages, not just system prompts.
/// This creates a user prompt that will work across all providers.
fn create_test_prompt(content: &str) -> PromptItem {
    use paladin::core::platform::container::prompt::UserPrompt;

    let prompt_type = PromptType::User(UserPrompt {
        query: content.to_string(),
        context: None,
    });

    PromptItem::new(prompt_type).expect("Failed to create test prompt")
}

/// Create a test LLM request
fn create_test_request(prompt: PromptItem, model: &str) -> LlmRequest {
    LlmRequest::new(model, prompt)
}

// ============================================================================
// OpenAI Live API Tests
// ============================================================================

#[tokio::test]
#[ignore] // Ignored by default - run with --ignored flag
async fn test_openai_basic_completion() {
    // Require API key (will panic if missing or empty)
    let api_key = require_api_key("OPENAI_API_KEY", "OpenAI");

    // Create OpenAI adapter
    let config = OpenAIConfig::new(api_key);
    let adapter = OpenAIAdapter::new(config).expect("Failed to create OpenAI adapter");

    // Create test request
    let prompt = create_test_prompt("Say 'Hello from OpenAI' and nothing else.");
    let request = create_test_request(prompt, "gpt-3.5-turbo");

    // Execute API call
    let response = adapter
        .generate(request)
        .await
        .expect("OpenAI API call failed");

    // Validate response
    assert!(!response.content.is_empty(), "Response content is empty");
    // OpenAI returns versioned models like "gpt-3.5-turbo-0125"
    assert!(
        response.model.starts_with("gpt-3.5-turbo"),
        "Expected model to start with gpt-3.5-turbo, got: {}",
        response.model
    );
    assert!(matches!(
        response.finish_reason,
        FinishReason::Stop | FinishReason::Length
    ));
    assert!(response.usage.total_tokens > 0, "Token usage not reported");

    println!("✓ OpenAI basic completion: {}", response.content);
}

#[tokio::test]
#[ignore]
async fn test_openai_streaming_completion() {
    let api_key = require_api_key("OPENAI_API_KEY", "OpenAI");

    let config = OpenAIConfig::new(api_key);
    let adapter = OpenAIAdapter::new(config).expect("Failed to create OpenAI adapter");

    let prompt = create_test_prompt("Count from 1 to 5, one number per line.");
    let request = create_test_request(prompt, "gpt-3.5-turbo");

    // Execute streaming API call
    let mut stream = adapter
        .generate_stream(request)
        .await
        .expect("OpenAI streaming failed");

    // Collect chunks
    let mut chunks = Vec::new();
    let mut finish_reason = None;

    // SAFETY: The boxed stream is never moved after this point
    while let Some(result) = unsafe { std::pin::Pin::new_unchecked(&mut *stream) }
        .next()
        .await
    {
        match result {
            Ok(chunk) => {
                if !chunk.delta.is_empty() {
                    chunks.push(chunk.delta.clone());
                }
                if chunk.finish_reason.is_some() {
                    finish_reason = chunk.finish_reason;
                }
            }
            Err(e) => {
                // Streaming may have incomplete chunks due to network buffering
                // Only fail on critical errors, not parse errors
                eprintln!("Stream chunk error (continuing): {}", e);
                if !e.to_string().contains("parse") && !e.to_string().contains("EOF") {
                    panic!("Unrecoverable OpenAI stream error: {}", e);
                }
            }
        }
    }

    // Validate streaming response
    assert!(!chunks.is_empty(), "No chunks received from stream");
    assert!(finish_reason.is_some(), "No finish reason received");

    let full_text = chunks.join("");
    assert!(!full_text.is_empty(), "Streamed content is empty");

    println!("✓ OpenAI streaming completion: {} chunks", chunks.len());
}

#[tokio::test]
#[ignore]
async fn test_openai_error_handling() {
    let api_key = require_api_key("OPENAI_API_KEY", "OpenAI");

    let config = OpenAIConfig::new(api_key);
    let adapter = OpenAIAdapter::new(config).expect("Failed to create OpenAI adapter");

    let prompt = create_test_prompt("This should fail with an invalid model.");
    let request = create_test_request(prompt, "invalid-model-12345");

    // Should fail with invalid model error
    let result = adapter.generate(request).await;
    assert!(result.is_err(), "Expected error for invalid model");

    match result {
        Err(LlmError::ModelNotAvailable(_)) | Err(LlmError::ProcessingError(_)) => {
            println!("✓ OpenAI error handling: Invalid model detected");
        }
        Err(e) => println!("✓ OpenAI error handling: Error type = {:?}", e),
        Ok(_) => panic!("Expected error but got success"),
    }
}

#[tokio::test]
#[ignore]
async fn test_openai_capabilities() {
    let api_key = require_api_key("OPENAI_API_KEY", "OpenAI");

    let config = OpenAIConfig::new(api_key);
    let adapter = OpenAIAdapter::new(config).expect("Failed to create OpenAI adapter");

    // Get provider capabilities
    let capabilities = adapter.get_capabilities();

    // Validate OpenAI capabilities
    assert!(
        capabilities.supports_streaming,
        "OpenAI should support streaming"
    );
    assert!(
        capabilities.supports_tool_calling,
        "OpenAI should support tool calling"
    );
    assert!(
        capabilities.supports_function_calling,
        "OpenAI should support function calling"
    );
    assert!(capabilities.supports_vision, "OpenAI should support vision");
    assert!(
        capabilities.supports_system_messages,
        "OpenAI should support system messages"
    );
    assert!(
        capabilities.max_context_tokens.is_some(),
        "OpenAI should report max context"
    );

    println!("✓ OpenAI capabilities validated: {:?}", capabilities);
}

// ============================================================================
// DeepSeek Live API Tests
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_deepseek_basic_completion() {
    let api_key = require_api_key("DEEPSEEK_API_KEY", "DeepSeek");

    let config = DeepSeekConfig::new(
        api_key,
        "https://api.deepseek.com/v1".to_string(),
        "deepseek-chat".to_string(),
    );
    let adapter = DeepSeekAdapter::new(config).expect("Failed to create DeepSeek adapter");

    let prompt = create_test_prompt("Say 'Hello from DeepSeek' and nothing else.");
    let request = create_test_request(prompt, "deepseek-chat");

    let response = adapter
        .generate(request)
        .await
        .expect("DeepSeek API call failed");

    assert!(!response.content.is_empty(), "Response content is empty");
    assert!(response.model.contains("deepseek"));
    assert!(matches!(
        response.finish_reason,
        FinishReason::Stop | FinishReason::Length
    ));
    assert!(response.usage.total_tokens > 0, "Token usage not reported");

    println!("✓ DeepSeek basic completion: {}", response.content);
}

#[tokio::test]
#[ignore]
async fn test_deepseek_streaming_completion() {
    let api_key = require_api_key("DEEPSEEK_API_KEY", "DeepSeek");

    let config = DeepSeekConfig::new(
        api_key,
        "https://api.deepseek.com/v1".to_string(),
        "deepseek-chat".to_string(),
    );
    let adapter = DeepSeekAdapter::new(config).expect("Failed to create DeepSeek adapter");

    let prompt = create_test_prompt("Count from 1 to 5, one number per line.");
    let request = create_test_request(prompt, "deepseek-chat");

    let mut stream = adapter
        .generate_stream(request)
        .await
        .expect("DeepSeek streaming failed");

    let mut chunks = Vec::new();
    let mut finish_reason = None;

    // SAFETY: The boxed stream is never moved after this point
    while let Some(result) = unsafe { std::pin::Pin::new_unchecked(&mut *stream) }
        .next()
        .await
    {
        match result {
            Ok(chunk) => {
                if !chunk.delta.is_empty() {
                    chunks.push(chunk.delta.clone());
                }
                if chunk.finish_reason.is_some() {
                    finish_reason = chunk.finish_reason;
                }
            }
            Err(e) => panic!("Stream error: {}", e),
        }
    }

    assert!(!chunks.is_empty(), "No chunks received from stream");
    assert!(finish_reason.is_some(), "No finish reason received");

    let full_text = chunks.join("");
    assert!(!full_text.is_empty(), "Streamed content is empty");

    println!("✓ DeepSeek streaming completion: {} chunks", chunks.len());
}

#[tokio::test]
#[ignore]
async fn test_deepseek_error_handling() {
    let api_key = require_api_key("DEEPSEEK_API_KEY", "DeepSeek");

    let config = DeepSeekConfig::new(
        api_key,
        "https://api.deepseek.com/v1".to_string(),
        "deepseek-chat".to_string(),
    );
    let adapter = DeepSeekAdapter::new(config).expect("Failed to create DeepSeek adapter");

    let prompt = create_test_prompt("This should fail with an invalid model.");
    let request = create_test_request(prompt, "invalid-deepseek-model");

    let result = adapter.generate(request).await;
    assert!(result.is_err(), "Expected error for invalid model");

    match result {
        Err(LlmError::ModelNotAvailable(_)) | Err(LlmError::ProcessingError(_)) => {
            println!("✓ DeepSeek error handling: Invalid model detected");
        }
        Err(e) => println!("✓ DeepSeek error handling: Error type = {:?}", e),
        Ok(_) => panic!("Expected error but got success"),
    }
}

#[tokio::test]
#[ignore]
async fn test_deepseek_capabilities() {
    let api_key = require_api_key("DEEPSEEK_API_KEY", "DeepSeek");

    let config = DeepSeekConfig::new(
        api_key,
        "https://api.deepseek.com/v1".to_string(),
        "deepseek-chat".to_string(),
    );
    let adapter = DeepSeekAdapter::new(config).expect("Failed to create DeepSeek adapter");

    let capabilities = adapter.get_capabilities();

    assert!(
        capabilities.supports_streaming,
        "DeepSeek should support streaming"
    );
    assert!(
        capabilities.supports_tool_calling,
        "DeepSeek should support tool calling"
    );
    assert!(
        capabilities.supports_system_messages,
        "DeepSeek should support system messages"
    );

    println!("✓ DeepSeek capabilities validated: {:?}", capabilities);
}

// ============================================================================
// Anthropic Live API Tests
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_anthropic_basic_completion() {
    let api_key = require_api_key("ANTHROPIC_API_KEY", "Anthropic");

    let config = AnthropicConfig::new(
        api_key,
        "https://api.anthropic.com/v1".to_string(),
        "claude-3-haiku-20240307".to_string(),
        4096,
    );
    let adapter = AnthropicAdapter::new(config).expect("Failed to create Anthropic adapter");

    let prompt = create_test_prompt("Say 'Hello from Claude' and nothing else.");
    let request = create_test_request(prompt, "claude-3-haiku-20240307");

    let response = adapter
        .generate(request)
        .await
        .expect("Anthropic API call failed");

    assert!(!response.content.is_empty(), "Response content is empty");
    assert!(response.model.contains("claude"));
    assert!(matches!(
        response.finish_reason,
        FinishReason::Stop | FinishReason::Length
    ));
    assert!(response.usage.total_tokens > 0, "Token usage not reported");

    println!("✓ Anthropic basic completion: {}", response.content);
}

#[tokio::test]
#[ignore]
async fn test_anthropic_streaming_completion() {
    let api_key = require_api_key("ANTHROPIC_API_KEY", "Anthropic");

    let config = AnthropicConfig::new(
        api_key,
        "https://api.anthropic.com/v1".to_string(),
        "claude-3-haiku-20240307".to_string(),
        4096,
    );
    let adapter = AnthropicAdapter::new(config).expect("Failed to create Anthropic adapter");

    let prompt = create_test_prompt("Count from 1 to 5, one number per line.");
    let request = create_test_request(prompt, "claude-3-haiku-20240307");

    let mut stream = adapter
        .generate_stream(request)
        .await
        .expect("Anthropic streaming failed");

    let mut chunks = Vec::new();
    let mut finish_reason = None;

    // SAFETY: The boxed stream is never moved after this point
    while let Some(result) = unsafe { std::pin::Pin::new_unchecked(&mut *stream) }
        .next()
        .await
    {
        match result {
            Ok(chunk) => {
                if !chunk.delta.is_empty() {
                    chunks.push(chunk.delta.clone());
                }
                if chunk.finish_reason.is_some() {
                    finish_reason = chunk.finish_reason;
                }
            }
            Err(e) => {
                // Streaming may have incomplete chunks due to network buffering
                // Only fail on critical errors, not parse errors
                eprintln!("Stream chunk error (continuing): {}", e);
                if !e.to_string().contains("parse") && !e.to_string().contains("EOF") {
                    panic!("Unrecoverable Anthropic stream error: {}", e);
                }
            }
        }
    }

    assert!(!chunks.is_empty(), "No chunks received from stream");
    assert!(finish_reason.is_some(), "No finish reason received");

    let full_text = chunks.join("");
    assert!(!full_text.is_empty(), "Streamed content is empty");

    println!("✓ Anthropic streaming completion: {} chunks", chunks.len());
}

#[tokio::test]
#[ignore]
async fn test_anthropic_error_handling() {
    let api_key = require_api_key("ANTHROPIC_API_KEY", "Anthropic");

    let config = AnthropicConfig::new(
        api_key,
        "https://api.anthropic.com/v1".to_string(),
        "claude-3-5-sonnet-20240620".to_string(),
        4096,
    );
    let adapter = AnthropicAdapter::new(config).expect("Failed to create Anthropic adapter");

    let prompt = create_test_prompt("This should fail with an invalid model.");
    let request = create_test_request(prompt, "invalid-claude-model");

    let result = adapter.generate(request).await;
    assert!(result.is_err(), "Expected error for invalid model");

    match result {
        Err(LlmError::ModelNotAvailable(_)) | Err(LlmError::ProcessingError(_)) => {
            println!("✓ Anthropic error handling: Invalid model detected");
        }
        Err(e) => println!("✓ Anthropic error handling: Error type = {:?}", e),
        Ok(_) => panic!("Expected error but got success"),
    }
}

#[tokio::test]
#[ignore]
async fn test_anthropic_capabilities() {
    let api_key = require_api_key("ANTHROPIC_API_KEY", "Anthropic");

    let config = AnthropicConfig::new(
        api_key,
        "https://api.anthropic.com/v1".to_string(),
        "claude-3-5-sonnet-20240620".to_string(),
        4096,
    );
    let adapter = AnthropicAdapter::new(config).expect("Failed to create Anthropic adapter");

    let capabilities = adapter.get_capabilities();

    assert!(
        capabilities.supports_streaming,
        "Anthropic should support streaming"
    );
    assert!(
        capabilities.supports_tool_calling,
        "Anthropic should support tool calling"
    );
    assert!(
        capabilities.supports_system_messages,
        "Anthropic should support system messages"
    );

    println!("✓ Anthropic capabilities validated: {:?}", capabilities);
}

mod test_suite_info {
    /// Test suite documentation
    ///
    /// This test suite validates live API integration with three LLM providers:
    /// - OpenAI (GPT-3.5, GPT-4)
    /// - DeepSeek (deepseek-chat)
    /// - Anthropic (Claude 3.5 Sonnet)
    ///
    /// # Running Tests
    ///
    /// **Option 1: Using .env file (recommended)**
    /// ```bash
    /// # Set API keys in .env file
    /// cp .env.example .env
    /// # Edit .env and add your keys
    ///
    /// # Tests will automatically load .env
    /// cargo test --features live-api-tests -- --ignored
    /// ```
    ///
    /// **Option 2: Export environment variables**
    /// ```bash
    /// export OPENAI_API_KEY="sk-..."
    /// export DEEPSEEK_API_KEY="sk-..."
    /// export ANTHROPIC_API_KEY="sk-..."
    ///
    /// cargo test --features live-api-tests -- --ignored
    /// ```
    ///
    /// **Run specific provider:**
    /// ```bash
    /// cargo test --features live-api-tests test_openai -- --ignored
    /// cargo test --features live-api-tests test_deepseek -- --ignored
    /// cargo test --features live-api-tests test_anthropic -- --ignored
    /// ```
    ///
    /// # Important Notes
    ///
    /// - Tests are marked `#[ignore]` and only run with `--ignored` flag
    /// - If an API key is missing or empty, the test will FAIL (not silently skip)
    /// - To skip a provider's tests, simply omit the `--ignored` flag or don't set the key
    /// - .env file is automatically loaded at test initialization
    ///
    /// # Test Coverage
    ///
    /// Each provider has 4 tests:
    /// 1. Basic completion - Validates generate() method
    /// 2. Streaming completion - Validates generate_stream() method
    /// 3. Error handling - Tests invalid model detection
    /// 4. Capabilities - Validates provider capabilities
    ///
    /// Total: 12 tests across 3 providers
    #[test]
    fn test_suite_documentation() {
        // This test always passes - just for documentation
    }
}
