use std::sync::Arc;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

use crate::core::base::service::analysis_service::{
    AnalysisService, AnalysisResult, AnalysisError, AnalysisConfig
};
use crate::application::ports::output::llm_port::{LlmPort, LlmRequest};
use crate::core::platform::container::prompt::PromptItem;
use crate::core::platform::container::content::ContentItem;

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
            return Err(AnalysisError::InvalidInput("Model name cannot be empty".to_string()));
        }
        if self.timeout_seconds == 0 {
            return Err(AnalysisError::InvalidInput("Timeout must be greater than 0".to_string()));
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

pub struct LlmAnalysisService {
    llm_port: Arc<dyn LlmPort>,
}

impl LlmAnalysisService {
    pub fn new(llm_port: Arc<dyn LlmPort>) -> Self {
        Self { llm_port }
    }
}

#[async_trait::async_trait]
impl AnalysisService<LlmAnalysisInput, LlmAnalysisOutput, LlmAnalysisConfig> for LlmAnalysisService {
    fn analyze(&self, input: &LlmAnalysisInput, config: &LlmAnalysisConfig) -> Result<AnalysisResult<LlmAnalysisOutput>, AnalysisError> {
        // This will need to be async in practice, but keeping sync for the trait
        tokio::runtime::Runtime::new()
            .map_err(|e| AnalysisError::ProcessingError(format!("Runtime error: {}", e)))?
            .block_on(self.analyze_async(input, config))
    }

    fn get_analysis_type(&self) -> &'static str {
        "llm_analysis"
    }

    fn validate_input(&self, input: &LlmAnalysisInput) -> Result<(), AnalysisError> {
        // Validate prompt
        match input.prompt.prompt_type() {
            crate::core::platform::container::prompt::PromptType::Text(text_prompt) => {
                if text_prompt.content.is_empty() {
                    return Err(AnalysisError::InvalidInput("Prompt content cannot be empty".to_string()));
                }
            },
            _ => {} // Other prompt types can be added as needed
        }
        Ok(())
    }
}

impl LlmAnalysisService {
    async fn analyze_async(&self, input: &LlmAnalysisInput, config: &LlmAnalysisConfig) -> Result<AnalysisResult<LlmAnalysisOutput>, AnalysisError> {
        let start_time = std::time::Instant::now();
        
        // Validate configuration
        config.validate()?;
        
        // Validate input
        self.validate_input(input)?;

        // Create LLM request
        let request = LlmRequest {
            id: Uuid::new_v4(),
            model: config.model.clone(),
            prompt: input.prompt.clone(),
            attachments: input.content_attachments.clone(),
            stream: config.enable_streaming,
            metadata: std::collections::HashMap::new(),
        };

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
                        input_hash: None, // Could compute hash of input if needed
                        result: output,
                        confidence: None,
                        metadata: std::collections::HashMap::new(),
                        processing_time_ms: processing_time,
                    });
                },
                Err(e) => {
                    last_error = Some(e);
                    if attempt < config.max_retries {
                        tokio::time::sleep(tokio::time::Duration::from_secs(2_u64.pow(attempt))).await;
                    }
                }
            }
        }

        Err(AnalysisError::ProcessingError(
            format!("LLM analysis failed after {} retries: {:?}", config.max_retries, last_error)
        ))
    }
}