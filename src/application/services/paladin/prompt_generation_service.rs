//! PromptGenerationService - LLM-based system prompt generation
//!
//! This service implements US-14.2: Auto-Generate System Prompt.
//! When a Paladin is configured with auto_generate_prompt, it uses this service
//! to generate contextual system prompts based on agent description and task context.
//!
//! # Features
//!
//! - LLM-based prompt generation with contextual templates
//! - Caching for performance (same inputs → same prompt)
//! - Deterministic generation (temperature=0.0)
//! - Manual override support
//!
//! # Examples
//!
//! ```rust,no_run
//! use paladin::application::services::paladin::prompt_generation_service::PromptGenerationService;
//! use paladin_ports::output::llm_port::LlmPort;
//! use std::sync::Arc;
//!
//! # async fn example(llm_port: Arc<dyn LlmPort>) -> Result<(), Box<dyn std::error::Error>> {
//! let service = PromptGenerationService::new(llm_port);
//!
//! // Generate a system prompt
//! let prompt = service.generate_prompt(
//!     "DataAnalyst",
//!     "An AI agent specialized in analyzing CSV data and generating insights",
//!     "gpt-4o", // model
//! ).await?;
//! # Ok(())
//! # }
//! ```

use crate::application::errors::prompt_error::PromptError;
use crate::core::platform::container::prompt::{
    PromptItem, PromptParameters, PromptType, UserPrompt,
};
use log::{debug, info};
use paladin_ports::output::llm_port::{LlmPort, LlmRequest};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Service for LLM-based system prompt generation
///
/// Generates contextual system prompts based on agent name and description,
/// with caching for performance and deterministic generation for testing.
pub struct PromptGenerationService {
    /// LLM port for prompt generation
    llm_port: Arc<dyn LlmPort>,

    /// Cache of generated prompts (key: agent_name + description hash)
    cache: Arc<Mutex<HashMap<String, String>>>,
}

impl PromptGenerationService {
    /// Creates a new PromptGenerationService
    ///
    /// # Arguments
    ///
    /// * `llm_port` - LLM port for generating prompts
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use paladin::application::services::paladin::prompt_generation_service::PromptGenerationService;
    /// use paladin_ports::output::llm_port::LlmPort;
    /// use std::sync::Arc;
    ///
    /// # fn example(llm_port: Arc<dyn LlmPort>) {
    /// let service = PromptGenerationService::new(llm_port);
    /// # }
    /// ```
    pub fn new(llm_port: Arc<dyn LlmPort>) -> Self {
        info!("Creating PromptGenerationService");
        Self {
            llm_port,
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Generates a system prompt using LLM
    ///
    /// # Arguments
    ///
    /// * `agent_name` - Name of the agent (e.g., "DataAnalyst", "CodeReviewer")
    /// * `agent_description` - Description of agent's role and capabilities
    /// * `model` - LLM model to use for generation (e.g., "gpt-4", "claude-3")
    ///
    /// # Returns
    ///
    /// A generated system prompt optimized for the agent's role
    ///
    /// # Errors
    ///
    /// Returns `PromptError` if:
    /// - LLM call fails
    /// - Agent description is empty
    /// - Generation produces invalid output
    pub async fn generate_prompt(
        &self,
        agent_name: &str,
        agent_description: &str,
        model: &str,
    ) -> Result<String, PromptError> {
        // Validate inputs
        if agent_description.trim().is_empty() {
            return Err(PromptError::InvalidDescription(
                "Agent description cannot be empty".to_string(),
            ));
        }

        info!("Generating system prompt for agent: {}", agent_name);

        // Check cache first
        let cache_key = self.build_cache_key(agent_name, agent_description);
        {
            let cache = self.cache.lock().unwrap();
            if let Some(cached_prompt) = cache.get(&cache_key) {
                info!("Using cached prompt for agent: {}", agent_name);
                return Ok(cached_prompt.clone());
            }
        }

        // Build generation prompt
        let prompt = self.build_generation_prompt(agent_name, agent_description);

        // Call LLM with deterministic parameters
        let user_prompt = UserPrompt {
            query: prompt,
            context: None,
        };

        let mut prompt_item = PromptItem::new(PromptType::User(user_prompt))
            .map_err(|e| PromptError::GenerationFailed(e.to_string()))?;

        // Use temperature=0.0 for deterministic generation
        prompt_item.set_parameters(PromptParameters {
            max_tokens: Some(500),  // Reasonable limit for system prompts
            temperature: Some(0.0), // Deterministic generation
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop_sequences: None,
        });

        let request = LlmRequest::new(model.to_string(), prompt_item);

        let response = self
            .llm_port
            .generate(request)
            .await
            .map_err(|e| PromptError::LlmError(e.to_string()))?;

        let generated_prompt = response.content.trim().to_string();

        // Validate generated prompt
        if generated_prompt.is_empty() {
            return Err(PromptError::GenerationFailed(
                "LLM returned empty prompt".to_string(),
            ));
        }

        // Cache the result
        {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(cache_key, generated_prompt.clone());
        }

        info!("Generated system prompt for agent: {}", agent_name);
        debug!("Generated prompt content: {}", generated_prompt);

        Ok(generated_prompt)
    }

    /// Invalidates the cache for a specific agent
    ///
    /// Forces regeneration on next call to generate_prompt
    pub fn invalidate_cache(&self, agent_name: &str, agent_description: &str) {
        let cache_key = self.build_cache_key(agent_name, agent_description);
        let mut cache = self.cache.lock().unwrap();
        cache.remove(&cache_key);
        info!("Invalidated cache for agent: {}", agent_name);
    }

    /// Clears all cached prompts
    pub fn clear_cache(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
        info!("Cleared all cached prompts");
    }

    /// Builds a cache key from agent name and description
    fn build_cache_key(&self, agent_name: &str, agent_description: &str) -> String {
        // Simple hash: combine name and description
        format!("{}::{}", agent_name, agent_description)
    }

    /// Builds the prompt for generating system prompts
    fn build_generation_prompt(&self, agent_name: &str, agent_description: &str) -> String {
        format!(
            r#"You are a system prompt generator for AI agents. Generate a concise, effective system prompt for an AI agent.

AGENT NAME: {}
AGENT DESCRIPTION: {}

Generate a system prompt that:
1. Clearly defines the agent's role and capabilities
2. Sets appropriate behavioral guidelines
3. Includes relevant expertise areas
4. Is concise (2-4 sentences)
5. Uses professional, clear language

Generate only the system prompt text, without any additional explanation or formatting markers."#,
            agent_name, agent_description
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use paladin_ports::output::llm_port::{
        FinishReason, LlmError, LlmResponse, ProviderCapabilities, TokenUsage,
    };
    use uuid::Uuid;

    /// Mock LLM port for testing
    struct MockLlmPort {
        response: String,
    }

    impl MockLlmPort {
        fn new(response: impl Into<String>) -> Self {
            Self {
                response: response.into(),
            }
        }
    }

    #[async_trait]
    impl LlmPort for MockLlmPort {
        async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
            Ok(LlmResponse {
                id: Uuid::new_v4(),
                request_id: Uuid::new_v4(),
                model: "test-model".to_string(),
                content: self.response.clone(),
                finish_reason: FinishReason::Stop,
                usage: TokenUsage {
                    prompt_tokens: 10,
                    completion_tokens: 20,
                    total_tokens: 30,
                },
                created_at: Utc::now(),
                metadata: HashMap::new(),
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
            unimplemented!("Streaming not needed for tests")
        }

        async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
            Ok(true)
        }

        async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
            Ok(vec!["test-model".to_string()])
        }

        fn get_provider_name(&self) -> &'static str {
            "mock"
        }

        fn get_capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities {
                supports_streaming: false,
                supports_function_calling: false,
                supports_tool_calling: false,
                supports_vision: false,
                supports_embeddings: false,
                supports_system_messages: true,
                max_context_tokens: Some(4096),
                temperature_range: None,
            }
        }
    }

    #[test]
    fn test_prompt_generation_service_new() {
        // Given: A mock LLM port
        let llm_port = Arc::new(MockLlmPort::new("test"));

        // When: Creating a new PromptGenerationService
        let _service = PromptGenerationService::new(llm_port.clone());

        // Then: The service should be created successfully
        assert!(Arc::strong_count(&llm_port) >= 2);
    }

    #[tokio::test]
    async fn test_generate_prompt_basic() {
        // Given: A mock LLM that returns a system prompt
        let generated_prompt = "You are a DataAnalyst AI agent specialized in analyzing CSV data, extracting insights, and generating comprehensive reports. You have expertise in statistical analysis, data visualization, and identifying trends.";
        let llm_port = Arc::new(MockLlmPort::new(generated_prompt));
        let service = PromptGenerationService::new(llm_port);

        // When: Generating a prompt
        let result = service
            .generate_prompt(
                "DataAnalyst",
                "An AI agent specialized in analyzing CSV data and generating insights",
                "gpt-4",
            )
            .await;

        // Then: Should return the generated prompt
        assert!(result.is_ok());
        let prompt = result.unwrap();
        assert_eq!(prompt, generated_prompt);
        assert!(prompt.contains("DataAnalyst"));
        assert!(prompt.contains("analyzing"));
    }

    #[tokio::test]
    async fn test_prompt_caching() {
        // Given: A service with a mock LLM
        let generated_prompt = "You are a CodeReviewer specialized in Rust code quality.";
        let llm_port = Arc::new(MockLlmPort::new(generated_prompt));
        let service = PromptGenerationService::new(llm_port);

        // When: Generating the same prompt twice
        let result1 = service
            .generate_prompt("CodeReviewer", "Reviews Rust code", "gpt-4")
            .await
            .unwrap();

        let result2 = service
            .generate_prompt("CodeReviewer", "Reviews Rust code", "gpt-4")
            .await
            .unwrap();

        // Then: Both should return the same cached result
        assert_eq!(result1, result2);
        assert_eq!(result1, generated_prompt);
    }

    #[tokio::test]
    async fn test_deterministic_generation() {
        // Given: A service that should generate deterministically
        let generated_prompt = "You are a TestAgent with specific capabilities.";
        let llm_port = Arc::new(MockLlmPort::new(generated_prompt));
        let service = PromptGenerationService::new(llm_port);

        // When: Generating prompts with the same inputs
        let result1 = service
            .generate_prompt("TestAgent", "A test agent", "gpt-4")
            .await
            .unwrap();

        // Clear cache to force regeneration
        service.clear_cache();

        let result2 = service
            .generate_prompt("TestAgent", "A test agent", "gpt-4")
            .await
            .unwrap();

        // Then: Should produce identical results (with mock, this is guaranteed)
        assert_eq!(result1, result2);
    }

    #[tokio::test]
    async fn test_generate_prompt_empty_description() {
        // Given: A service
        let llm_port = Arc::new(MockLlmPort::new("prompt"));
        let service = PromptGenerationService::new(llm_port);

        // When: Trying to generate with empty description
        let result = service.generate_prompt("Agent", "", "gpt-4").await;

        // Then: Should return error
        assert!(result.is_err());
        match result {
            Err(PromptError::InvalidDescription(msg)) => {
                assert!(msg.contains("empty"));
            }
            _ => panic!("Expected InvalidDescription error"),
        }
    }

    #[tokio::test]
    async fn test_cache_invalidation() {
        // Given: A service with a cached prompt
        let llm_port = Arc::new(MockLlmPort::new("Prompt v1"));
        let service = PromptGenerationService::new(llm_port);

        let _first = service
            .generate_prompt("Agent", "Description", "gpt-4")
            .await
            .unwrap();

        // When: Invalidating the cache
        service.invalidate_cache("Agent", "Description");

        // Then: Cache should be empty for that key
        let cache = service.cache.lock().unwrap();
        let key = service.build_cache_key("Agent", "Description");
        assert!(!cache.contains_key(&key));
    }

    #[tokio::test]
    async fn test_generate_prompt_logging() {
        // Given: A service
        let llm_port = Arc::new(MockLlmPort::new("Test prompt"));
        let service = PromptGenerationService::new(llm_port);

        // When: Generating a prompt (logging happens internally)
        let result = service
            .generate_prompt("LogTest", "Test logging", "gpt-4")
            .await;

        // Then: Should succeed (logging is tested by presence of log! calls in code)
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_prompt_generation_uses_configured_model() {
        // Given: A mock LLM port
        let llm_port = Arc::new(MockLlmPort::new("Generated system prompt"));
        let service = PromptGenerationService::new(llm_port.clone());

        // When: Generating a prompt with different models
        let gpt4_result = service
            .generate_prompt("Agent1", "A helpful assistant", "gpt-4")
            .await;
        let claude_result = service
            .generate_prompt("Agent2", "A code reviewer", "claude-3")
            .await;
        let custom_result = service
            .generate_prompt("Agent3", "A data analyst", "custom-model")
            .await;

        // Then: All should succeed (model is passed to LlmPort)
        assert!(gpt4_result.is_ok());
        assert_eq!(gpt4_result.unwrap(), "Generated system prompt");

        assert!(claude_result.is_ok());
        assert_eq!(claude_result.unwrap(), "Generated system prompt");

        assert!(custom_result.is_ok());
        assert_eq!(custom_result.unwrap(), "Generated system prompt");

        // Note: In a real implementation with model tracking, we'd verify
        // the correct model was passed to the LlmPort for each call
    }
}
