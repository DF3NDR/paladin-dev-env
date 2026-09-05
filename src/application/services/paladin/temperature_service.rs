use crate::application::services::paladin::error::PaladinError;
use crate::core::base::entity::node::Node;
use crate::core::platform::container::prompt::{
    PromptData, PromptItem, PromptParameters, PromptType, UserPrompt,
};
use log::{debug, info};
use paladin_battalion::llm_failure::to_paladin_error;
use paladin_ports::output::llm_port::{LlmPort, LlmRequest};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use uuid::Uuid;

/// Task types for adaptive temperature selection
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskType {
    /// Creative tasks: writing, brainstorming, ideation
    /// High temperature (0.8-0.9) for diverse, imaginative outputs
    Creative,
    /// Analytical tasks: math, logic, code analysis, fact extraction
    /// Low temperature (0.1-0.3) for precise, deterministic outputs
    Analytical,
    /// Standard tasks: general conversation, Q&A, summarization
    /// Medium temperature (0.5-0.7) for balanced outputs
    Standard,
}

/// Configuration for temperature calculation
#[derive(Debug, Clone)]
pub struct TemperatureConfig {
    /// Temperature for creative tasks (default: 0.85)
    pub creative_temp: f32,
    /// Temperature for analytical tasks (default: 0.2)
    pub analytical_temp: f32,
    /// Temperature for standard tasks (default: 0.6)
    pub standard_temp: f32,
    /// Enable LLM-based task type detection (default: true)
    pub enable_llm_detection: bool,
}

impl Default for TemperatureConfig {
    fn default() -> Self {
        Self {
            creative_temp: 0.85,
            analytical_temp: 0.2,
            standard_temp: 0.6,
            enable_llm_detection: true,
        }
    }
}

/// Service for adaptive temperature selection based on task type
///
/// # Examples
///
/// ```
/// use paladin::application::services::paladin::temperature_service::TemperatureService;
/// use paladin_ports::output::llm_port::LlmPort;
/// use std::sync::Arc;
///
/// fn build(llm_port: Arc<dyn LlmPort>) -> TemperatureService {
///     TemperatureService::new(llm_port)
/// }
/// ```
pub struct TemperatureService {
    llm_port: Arc<dyn LlmPort>,
    config: TemperatureConfig,
}

impl TemperatureService {
    /// Create a new TemperatureService with default configuration
    pub fn new(llm_port: Arc<dyn LlmPort>) -> Self {
        Self {
            llm_port,
            config: TemperatureConfig::default(),
        }
    }

    /// Create a new TemperatureService with custom configuration
    pub fn with_config(llm_port: Arc<dyn LlmPort>, config: TemperatureConfig) -> Self {
        Self { llm_port, config }
    }

    /// Calculate optimal temperature based on agent description and task context
    ///
    /// # Arguments
    /// * `agent_description` - Description of the agent's role/purpose
    /// * `task_context` - Optional context about the specific task
    ///
    /// # Returns
    /// Optimal temperature value (0.0-1.0)
    pub async fn calculate_optimal_temperature(
        &self,
        agent_description: &str,
        task_context: Option<&str>,
    ) -> Result<f32, PaladinError> {
        // Validate inputs
        if agent_description.trim().is_empty() {
            return Err(PaladinError::ConfigurationError(
                "Agent description cannot be empty".into(),
            ));
        }

        info!(
            "Calculating optimal temperature: agent_description={}, has_task_context={}",
            agent_description,
            task_context.is_some()
        );

        // Detect task type
        let task_type = if self.config.enable_llm_detection {
            self.detect_task_type_with_llm(agent_description, task_context)
                .await?
        } else {
            self.detect_task_type_heuristic(agent_description, task_context)
        };

        // Map task type to temperature
        let temperature = match task_type {
            TaskType::Creative => self.config.creative_temp,
            TaskType::Analytical => self.config.analytical_temp,
            TaskType::Standard => self.config.standard_temp,
        };

        debug!(
            "Calculated optimal temperature: task_type={:?}, temperature={}",
            task_type, temperature
        );

        Ok(temperature)
    }

    /// Detect task type using LLM analysis
    async fn detect_task_type_with_llm(
        &self,
        agent_description: &str,
        task_context: Option<&str>,
    ) -> Result<TaskType, PaladinError> {
        let prompt = self.build_detection_prompt(agent_description, task_context);

        // Build LLM request
        let request = LlmRequest {
            id: Uuid::new_v4(),
            model: "gpt-4".to_string(), // Use consistent model for classification
            prompt: PromptItem {
                node: Node::new(
                    PromptData {
                        prompt_type: PromptType::User(UserPrompt {
                            query: prompt.clone(),
                            context: None,
                        }),
                        content_attachments: vec![],
                        parameters: PromptParameters {
                            max_tokens: Some(500),
                            temperature: Some(0.0),
                            top_p: None,
                            frequency_penalty: None,
                            presence_penalty: None,
                            stop_sequences: None,
                        },
                        context: None,
                        expected_output: None,
                        tags: None,
                        category: None,
                        author: None,
                        metadata: BTreeMap::new(),
                    },
                    Some("temperature_detection".to_string()),
                ),
            },
            attachments: vec![],
            stream: false,
            metadata: HashMap::new(),
        };

        let response = self
            .llm_port
            .generate(request)
            .await
            .map_err(|e| to_paladin_error(&e))?;

        // Parse response
        let task_type = self.parse_task_type(&response.content)?;

        debug!(
            "LLM detected task type: response={}, detected_type={:?}",
            response.content, task_type
        );

        Ok(task_type)
    }

    /// Detect task type using keyword heuristics (fallback)
    fn detect_task_type_heuristic(
        &self,
        agent_description: &str,
        task_context: Option<&str>,
    ) -> TaskType {
        let combined =
            format!("{} {}", agent_description, task_context.unwrap_or_default()).to_lowercase();

        // Creative keywords
        let creative_keywords = [
            "creative",
            "writing",
            "story",
            "brainstorm",
            "idea",
            "imaginative",
            "novel",
            "poetry",
            "artistic",
            "design",
        ];

        // Analytical keywords
        let analytical_keywords = [
            "analytical",
            "analyze",
            "math",
            "calculation",
            "logic",
            "code",
            "debug",
            "precise",
            "fact",
            "data",
            "research",
            "technical",
        ];

        // Count keyword matches
        let creative_score = creative_keywords
            .iter()
            .filter(|kw| combined.contains(*kw))
            .count();

        let analytical_score = analytical_keywords
            .iter()
            .filter(|kw| combined.contains(*kw))
            .count();

        // Determine task type
        if analytical_score > creative_score {
            TaskType::Analytical
        } else if creative_score > 0 {
            TaskType::Creative
        } else {
            TaskType::Standard
        }
    }

    /// Build prompt for LLM-based task type detection
    fn build_detection_prompt(
        &self,
        agent_description: &str,
        task_context: Option<&str>,
    ) -> String {
        let context_part = task_context
            .map(|c| format!("\n\nTask Context:\n{}", c))
            .unwrap_or_default();

        format!(
            r#"You are a task classifier. Analyze the following agent description and determine the task type.

Agent Description:
{}{}

Task Types:
- CREATIVE: Writing, brainstorming, ideation, storytelling, artistic work
- ANALYTICAL: Math, logic, code analysis, fact extraction, technical work, debugging
- STANDARD: General conversation, Q&A, summarization, information retrieval

Respond with ONLY one word: CREATIVE, ANALYTICAL, or STANDARD"#,
            agent_description, context_part
        )
    }

    /// Parse LLM response to extract task type
    fn parse_task_type(&self, response: &str) -> Result<TaskType, PaladinError> {
        let normalized = response.trim().to_uppercase();

        if normalized.contains("CREATIVE") {
            Ok(TaskType::Creative)
        } else if normalized.contains("ANALYTICAL") {
            Ok(TaskType::Analytical)
        } else if normalized.contains("STANDARD") {
            Ok(TaskType::Standard)
        } else {
            // Default to standard if unclear
            debug!(
                "Could not parse task type from '{}', defaulting to Standard",
                response
            );
            Ok(TaskType::Standard)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use futures::stream;
    use paladin_ports::output::llm_port::{
        FinishReason, LlmError, LlmPort, LlmResponse, ProviderCapabilities, StreamingResponse,
        TokenUsage,
    };
    use std::sync::Mutex;

    /// Mock LLM port for testing
    struct MockLlmPort {
        response: Mutex<String>,
    }

    impl MockLlmPort {
        fn new(response: &str) -> Arc<Self> {
            Arc::new(Self {
                response: Mutex::new(response.to_string()),
            })
        }
    }

    #[async_trait]
    impl LlmPort for MockLlmPort {
        async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
            Ok(LlmResponse {
                id: Uuid::new_v4(),
                request_id: Uuid::new_v4(),
                model: "gpt-4".to_string(),
                content: self.response.lock().unwrap().clone(),
                finish_reason: FinishReason::Stop,
                usage: TokenUsage {
                    prompt_tokens: 10,
                    completion_tokens: 5,
                    total_tokens: 15,
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
            Box<dyn futures::Stream<Item = Result<StreamingResponse, LlmError>> + Send>,
            LlmError,
        > {
            Ok(Box::new(stream::empty()))
        }

        async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
            Ok(true)
        }

        async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
            Ok(vec!["gpt-4".to_string()])
        }

        fn get_provider_name(&self) -> &'static str {
            "mock"
        }

        fn get_capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities {
                supports_streaming: false,
                supports_tool_calling: false,
                supports_function_calling: false,
                supports_vision: false,
                supports_embeddings: false,
                max_context_tokens: Some(8192),
                supports_system_messages: true,
                temperature_range: None,
            }
        }
    }

    /// An `LlmPort` whose `generate` fails with a caller-chosen real
    /// `LlmError`, so the site migrated by plan 25-06 (D-02) can be observed
    /// converting it through `llm_failure::to_paladin_error`.
    struct FailingLlmPort(fn() -> LlmError);

    #[async_trait]
    impl LlmPort for FailingLlmPort {
        async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
            Err((self.0)())
        }

        async fn generate_stream(
            &self,
            _request: LlmRequest,
        ) -> Result<
            Box<dyn futures::Stream<Item = Result<StreamingResponse, LlmError>> + Send>,
            LlmError,
        > {
            Err((self.0)())
        }

        async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
            Ok(true)
        }

        async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
            Ok(vec![])
        }

        fn get_provider_name(&self) -> &'static str {
            "failing"
        }

        fn get_capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities::default()
        }
    }

    fn provider_503() -> LlmError {
        LlmError::ProviderError {
            provider: "openai".to_string(),
            status: 503,
            message: "upstream unavailable".to_string(),
        }
    }

    fn auth_failure() -> LlmError {
        LlmError::AuthenticationError("invalid API key".to_string())
    }

    /// Plan 25-06 Test 4 (D-02, X-03): the temperature-detection site
    /// surfaces a real `LlmError` as the structured `LlmFailure` -- typed
    /// transience/status/provider intact -- while rendering exactly what the
    /// legacy `PaladinError::LlmError(e.to_string())` rendered.
    #[tokio::test]
    async fn temperature_service_surfaces_structured_llm_failure() {
        use paladin_core::platform::container::transience::Transience;

        // Transient, status-carrying.
        let service = TemperatureService::new(Arc::new(FailingLlmPort(provider_503)));
        let err = service
            .calculate_optimal_temperature("A creative storyteller", None)
            .await
            .expect_err("a provider 503 must fail detection");
        match &err {
            PaladinError::LlmFailure {
                transience,
                status,
                provider,
                ..
            } => {
                assert_eq!(*transience, Transience::Transient);
                assert_eq!(*status, Some(503));
                assert_eq!(provider.as_deref(), Some("openai"));
            }
            other => panic!("expected PaladinError::LlmFailure, got {other:?}"),
        }
        assert_eq!(err.to_string(), format!("LLM error: {}", provider_503()));

        // Permanent, no status.
        let service = TemperatureService::new(Arc::new(FailingLlmPort(auth_failure)));
        let err = service
            .calculate_optimal_temperature("A creative storyteller", None)
            .await
            .expect_err("an authentication failure must fail detection");
        match err {
            PaladinError::LlmFailure {
                transience,
                status,
                provider,
                ..
            } => {
                assert_eq!(transience, Transience::Permanent);
                assert_eq!(status, None);
                assert_eq!(provider, None);
            }
            other => panic!("expected PaladinError::LlmFailure, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_temperature_service_creation() {
        // Given: A mock LLM port
        let llm_port = MockLlmPort::new("STANDARD");

        // When: Creating a new service
        let service = TemperatureService::new(llm_port);

        // Then: Service is created with default config
        assert_eq!(service.config.creative_temp, 0.85);
        assert_eq!(service.config.analytical_temp, 0.2);
        assert_eq!(service.config.standard_temp, 0.6);
        assert!(service.config.enable_llm_detection);
    }

    #[tokio::test]
    async fn test_temperature_service_with_custom_config() {
        // Given: A mock LLM port and custom config
        let llm_port = MockLlmPort::new("STANDARD");
        let config = TemperatureConfig {
            creative_temp: 0.9,
            analytical_temp: 0.1,
            standard_temp: 0.5,
            enable_llm_detection: false,
        };

        // When: Creating service with custom config
        let service = TemperatureService::with_config(llm_port, config.clone());

        // Then: Service uses custom config
        assert_eq!(service.config.creative_temp, 0.9);
        assert_eq!(service.config.analytical_temp, 0.1);
        assert_eq!(service.config.standard_temp, 0.5);
        assert!(!service.config.enable_llm_detection);
    }

    #[tokio::test]
    async fn test_calculate_temperature_creative_task() {
        // Given: Service with mock LLM returning CREATIVE
        let llm_port = MockLlmPort::new("CREATIVE");
        let service = TemperatureService::new(llm_port);

        // When: Calculating temperature for creative agent
        let result: Result<f32, PaladinError> = service
            .calculate_optimal_temperature("A creative writing assistant", None)
            .await;

        // Then: Returns high temperature
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0.85);
    }

    #[tokio::test]
    async fn test_calculate_temperature_analytical_task() {
        // Given: Service with mock LLM returning ANALYTICAL
        let llm_port = MockLlmPort::new("ANALYTICAL");
        let service = TemperatureService::new(llm_port);

        // When: Calculating temperature for analytical agent
        let result: Result<f32, PaladinError> = service
            .calculate_optimal_temperature("A code analysis and debugging assistant", None)
            .await;

        // Then: Returns low temperature
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0.2);
    }

    #[tokio::test]
    async fn test_calculate_temperature_standard_task() {
        // Given: Service with mock LLM returning STANDARD
        let llm_port = MockLlmPort::new("STANDARD");
        let service = TemperatureService::new(llm_port);

        // When: Calculating temperature for standard agent
        let result: Result<f32, PaladinError> = service
            .calculate_optimal_temperature("A general Q&A assistant", None)
            .await;

        // Then: Returns medium temperature
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0.6);
    }

    #[tokio::test]
    async fn test_calculate_temperature_with_task_context() {
        // Given: Service with mock LLM
        let llm_port = MockLlmPort::new("CREATIVE");
        let service = TemperatureService::new(llm_port);

        // When: Calculating with task context
        let result: Result<f32, PaladinError> = service
            .calculate_optimal_temperature(
                "A writing assistant",
                Some("Write a short story about a robot"),
            )
            .await;

        // Then: Returns creative temperature
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0.85);
    }

    #[tokio::test]
    async fn test_calculate_temperature_empty_description() {
        // Given: Service with mock LLM
        let llm_port = MockLlmPort::new("STANDARD");
        let service = TemperatureService::new(llm_port);

        // When: Calculating with empty description
        let result: Result<f32, PaladinError> =
            service.calculate_optimal_temperature("", None).await;

        // Then: Returns ConfigurationError
        assert!(result.is_err());
        match result.unwrap_err() {
            PaladinError::ConfigurationError(msg) => {
                assert!(msg.contains("cannot be empty"));
            }
            _ => panic!("Expected ConfigurationError"),
        }
    }

    #[tokio::test]
    async fn test_heuristic_detection_creative() {
        // Given: Service with heuristic detection enabled
        let llm_port = MockLlmPort::new("ignored");
        let config = TemperatureConfig {
            enable_llm_detection: false,
            ..Default::default()
        };
        let service = TemperatureService::with_config(llm_port, config);

        // When: Calculating for creative keywords
        let result: Result<f32, PaladinError> = service
            .calculate_optimal_temperature("A creative writing and brainstorming assistant", None)
            .await;

        // Then: Returns creative temperature
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0.85);
    }

    #[tokio::test]
    async fn test_heuristic_detection_analytical() {
        // Given: Service with heuristic detection enabled
        let llm_port = MockLlmPort::new("ignored");
        let config = TemperatureConfig {
            enable_llm_detection: false,
            ..Default::default()
        };
        let service = TemperatureService::with_config(llm_port, config);

        // When: Calculating for analytical keywords
        let result: Result<f32, PaladinError> = service
            .calculate_optimal_temperature("A code analysis and math problem solver", None)
            .await;

        // Then: Returns analytical temperature
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0.2);
    }

    #[tokio::test]
    async fn test_heuristic_detection_standard() {
        // Given: Service with heuristic detection enabled
        let llm_port = MockLlmPort::new("ignored");
        let config = TemperatureConfig {
            enable_llm_detection: false,
            ..Default::default()
        };
        let service = TemperatureService::with_config(llm_port, config);

        // When: Calculating for generic description
        let result: Result<f32, PaladinError> = service
            .calculate_optimal_temperature("A helpful assistant", None)
            .await;

        // Then: Returns standard temperature
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0.6);
    }

    #[tokio::test]
    async fn test_parse_task_type_creative() {
        // Given: Service
        let llm_port = MockLlmPort::new("CREATIVE");
        let service = TemperatureService::new(llm_port);

        // When: Parsing "CREATIVE" response
        let result = service.parse_task_type("CREATIVE");

        // Then: Returns Creative
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TaskType::Creative);
    }

    #[tokio::test]
    async fn test_parse_task_type_analytical() {
        // Given: Service
        let llm_port = MockLlmPort::new("ANALYTICAL");
        let service = TemperatureService::new(llm_port);

        // When: Parsing "ANALYTICAL" response
        let result = service.parse_task_type("ANALYTICAL");

        // Then: Returns Analytical
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TaskType::Analytical);
    }

    #[tokio::test]
    async fn test_parse_task_type_standard() {
        // Given: Service
        let llm_port = MockLlmPort::new("STANDARD");
        let service = TemperatureService::new(llm_port);

        // When: Parsing "STANDARD" response
        let result = service.parse_task_type("STANDARD");

        // Then: Returns Standard
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TaskType::Standard);
    }

    #[tokio::test]
    async fn test_parse_task_type_with_explanation() {
        // Given: Service
        let llm_port = MockLlmPort::new("dummy");
        let service = TemperatureService::new(llm_port);

        // When: Parsing response with explanation
        let result = service.parse_task_type(
            "Based on the description, this is clearly a CREATIVE task involving storytelling.",
        );

        // Then: Extracts Creative correctly
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TaskType::Creative);
    }

    #[tokio::test]
    async fn test_parse_task_type_ambiguous_defaults_to_standard() {
        // Given: Service
        let llm_port = MockLlmPort::new("dummy");
        let service = TemperatureService::new(llm_port);

        // When: Parsing ambiguous response
        let result = service.parse_task_type("I'm not sure about this one");

        // Then: Defaults to Standard
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TaskType::Standard);
    }

    #[tokio::test]
    async fn test_build_detection_prompt_without_context() {
        // Given: Service
        let llm_port = MockLlmPort::new("STANDARD");
        let service = TemperatureService::new(llm_port);

        // When: Building prompt without context
        let prompt = service.build_detection_prompt("A helpful assistant", None);

        // Then: Prompt contains description but no context section
        assert!(prompt.contains("A helpful assistant"));
        assert!(!prompt.contains("Task Context:"));
        assert!(prompt.contains("CREATIVE"));
        assert!(prompt.contains("ANALYTICAL"));
        assert!(prompt.contains("STANDARD"));
    }

    #[tokio::test]
    async fn test_build_detection_prompt_with_context() {
        // Given: Service
        let llm_port = MockLlmPort::new("STANDARD");
        let service = TemperatureService::new(llm_port);

        // When: Building prompt with context
        let prompt =
            service.build_detection_prompt("A helpful assistant", Some("Solve math problems"));

        // Then: Prompt contains both description and context
        assert!(prompt.contains("A helpful assistant"));
        assert!(prompt.contains("Task Context:"));
        assert!(prompt.contains("Solve math problems"));
    }
}
