use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use paladin_core::base::service::analysis_service::{
    AnalysisConfig, AnalysisError, AnalysisResult, AnalysisService,
};
use paladin_core::platform::container::content::ContentItem;
use paladin_core::platform::container::prompt::PromptItem;
use paladin_ports::output::llm_port::{LlmPort, LlmRequest};

#[derive(Debug, Clone)]
pub struct LlmAnalysisConfig {
    pub model: String,
    pub max_retries: u32,
    pub timeout_seconds: u64,
    pub enable_streaming: bool,
}

impl AnalysisConfig for LlmAnalysisConfig {
    fn validate(&self) -> Result<(), AnalysisError> {
        if self.model.is_empty() {
            return Err(AnalysisError::InvalidInput(
                "Model name cannot be empty".to_string(),
            ));
        }
        if self.timeout_seconds == 0 {
            return Err(AnalysisError::InvalidInput(
                "Timeout must be greater than 0".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct LlmAnalysisInput {
    pub prompt: PromptItem,
    pub content_attachments: Vec<ContentItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmAnalysisOutput {
    pub content: String,
    pub model_used: String,
    pub token_usage: TokenUsage,
    pub processing_time_ms: u64,
}

// Re-export the canonical domain type from paladin-core (ADR-0016 / DEBT-05).
pub use paladin_core::platform::container::token_usage::TokenUsage;

/// LLM-backed content analysis service.
///
/// Wraps an [`LlmPort`] adapter with retry, timing and [`AnalysisResult`]
/// packaging so callers depend only on the port trait, not a concrete
/// provider.
///
/// # Examples
///
/// ```
/// use paladin_llm::llm_analysis_service::LlmAnalysisService;
/// use paladin_ports::output::llm_port::LlmPort;
/// use std::sync::Arc;
///
/// fn build(llm_port: Arc<dyn LlmPort>) -> LlmAnalysisService {
///     LlmAnalysisService::new(llm_port)
/// }
/// ```
#[derive(Clone)] // Remove Debug derive here
pub struct LlmAnalysisService {
    llm_port: Arc<dyn LlmPort>,
}

// Implement Debug manually
impl std::fmt::Debug for LlmAnalysisService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmAnalysisService")
            .field(
                "llm_port",
                &format!("Arc<dyn LlmPort: {}>", self.llm_port.get_provider_name()),
            )
            .finish()
    }
}

impl LlmAnalysisService {
    pub fn new(llm_port: Arc<dyn LlmPort>) -> Self {
        Self { llm_port }
    }

    /// Async version of analyze that should be used when possible
    pub async fn analyze_async(
        &self,
        input: &LlmAnalysisInput,
        config: &LlmAnalysisConfig,
    ) -> Result<AnalysisResult<LlmAnalysisOutput>, AnalysisError> {
        let start_time = std::time::Instant::now();

        // Validate configuration
        config.validate()?;

        // Validate input
        self.validate_input(input)?;

        // Create LLM request
        let request = LlmRequest::new(config.model.clone(), input.prompt.clone())
            .with_attachments(input.content_attachments.clone())
            .with_stream(config.enable_streaming);

        // Call LLM with retries
        let mut last_error = None;
        for attempt in 0..=config.max_retries {
            match self.llm_port.generate(request.clone()).await {
                Ok(response) => {
                    let processing_time = start_time.elapsed().as_millis() as u64;

                    let output = LlmAnalysisOutput {
                        content: response.content,
                        model_used: response.model,
                        token_usage: TokenUsage {
                            prompt_tokens: response.usage.prompt_tokens,
                            completion_tokens: response.usage.completion_tokens,
                            total_tokens: response.usage.total_tokens,
                        },
                        processing_time_ms: processing_time,
                    };

                    return Ok(AnalysisResult {
                        id: Uuid::new_v4(),
                        created_at: chrono::Utc::now(),
                        analysis_type: self.get_analysis_type().to_string(),
                        input_hash: None,
                        result: output,
                        confidence: None,
                        metadata: std::collections::HashMap::new(),
                        processing_time_ms: processing_time,
                    });
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < config.max_retries {
                        tokio::time::sleep(tokio::time::Duration::from_secs(2_u64.pow(attempt)))
                            .await;
                    }
                }
            }
        }

        Err(AnalysisError::ProcessingError(format!(
            "LLM analysis failed after {} retries: {:?}",
            config.max_retries, last_error
        )))
    }
}

#[async_trait::async_trait]
impl AnalysisService<LlmAnalysisInput, LlmAnalysisOutput, LlmAnalysisConfig>
    for LlmAnalysisService
{
    fn analyze(
        &self,
        _input: &LlmAnalysisInput,
        _config: &LlmAnalysisConfig,
    ) -> Result<AnalysisResult<LlmAnalysisOutput>, AnalysisError> {
        // For sync version, return an error or use a different approach
        // This avoids the nested runtime issue
        Err(AnalysisError::ProcessingError(
            "Sync analysis not supported. Use analyze_async for proper async handling.".to_string(),
        ))
    }

    fn get_analysis_type(&self) -> &'static str {
        "llm_analysis"
    }

    fn validate_input(&self, input: &LlmAnalysisInput) -> Result<(), AnalysisError> {
        // Validate prompt
        // Other prompt types can be added as needed
        if let paladin_core::platform::container::prompt::PromptType::Text(text_prompt) =
            input.prompt.prompt_type()
            && text_prompt.content.is_empty()
        {
            return Err(AnalysisError::InvalidInput(
                "Prompt content cannot be empty".to_string(),
            ));
        }
        Ok(())
    }
}
