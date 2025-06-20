use std::sync::Arc;
use serde_json::Value;

use crate::core::platform::container::content::ContentItem;
use crate::core::platform::container::prompt::PromptItem;
use crate::core::base::service::analysis_service::AnalysisService;
use crate::application::use_cases::analysis::llm_analysis_service::{
    LlmAnalysisService, LlmAnalysisInput, LlmAnalysisConfig
};

/// Input for LLM-based content analysis
#[derive(Debug, Clone)]
pub struct LlmContentAnalysisInput {
    pub prompt: PromptItem,
    pub content: ContentItem,
}

/// Configuration for LLM content analysis
#[derive(Debug, Clone)]
pub struct LlmContentAnalysisConfig {
    pub llm_config: LlmAnalysisConfig,
    pub include_content_metadata: bool,
    pub max_content_length: Option<usize>,
}

impl Default for LlmContentAnalysisConfig {
    fn default() -> Self {
        Self {
            llm_config: LlmAnalysisConfig {
                model: "gpt-3.5-turbo".to_string(),
                max_retries: 3,
                timeout_seconds: 30,
                enable_streaming: false,
            },
            include_content_metadata: true,
            max_content_length: Some(10000),
        }
    }
}

/// Use case for analyzing content using LLM with pre-existing prompts
pub struct LlmContentAnalyzer {
    llm_service: Arc<LlmAnalysisService>,
}

impl LlmContentAnalyzer {
    pub fn new(llm_service: Arc<LlmAnalysisService>) -> Self {
        Self { llm_service }
    }

    /// Analyze content using a pre-existing prompt
    pub fn analyze_with_prompt(
        &self, 
        input: &LlmContentAnalysisInput, 
        config: &LlmContentAnalysisConfig
    ) -> Result<Value, String> {
        // Validate inputs
        self.validate_input(input, config)?;

        // Prepare the LLM analysis input
        let llm_input = LlmAnalysisInput {
            prompt: input.prompt.clone(),
            content_attachments: vec![input.content.clone()],
        };

        // Perform analysis using the LLM service
        let result = self.llm_service
            .analyze(&llm_input, &config.llm_config)
            .map_err(|e| format!("LLM analysis failed: {:?}", e))?;

        // Process and return the result
        self.process_analysis_result(&result.result.content, &input.content, config)
    }

    /// Validate the input for analysis
    fn validate_input(
        &self, 
        input: &LlmContentAnalysisInput, 
        config: &LlmContentAnalysisConfig
    ) -> Result<(), String> {
        // Validate prompt
        match input.prompt.prompt_type() {
            crate::core::platform::container::prompt::PromptType::Text(text_prompt) => {
                if text_prompt.content.trim().is_empty() {
                    return Err("Prompt content cannot be empty".to_string());
                }
            },
            _ => {
                // For now, we only support text prompts for content analysis
                return Err("Only text prompts are supported for content analysis".to_string());
            }
        }

        // Validate content length if specified
        if let Some(max_length) = config.max_content_length {
            let content_text = self.extract_content_text(&input.content);
            if content_text.len() > max_length {
                return Err(format!(
                    "Content length ({}) exceeds maximum allowed length ({})", 
                    content_text.len(), 
                    max_length
                ));
            }
        }

        Ok(())
    }

    /// Extract text representation of content for validation
    fn extract_content_text(&self, content: &ContentItem) -> String {
        match content.content() {
            crate::core::platform::container::content::ContentType::Text(text) => {
                text.content.as_deref().unwrap_or("").to_string()
            },
            crate::core::platform::container::content::ContentType::Video(_) => {
                format!("Video content: {}", content.title().unwrap_or(&"Untitled".to_string()))
            },
            crate::core::platform::container::content::ContentType::Audio(_) => {
                format!("Audio content: {}", content.title().unwrap_or(&"Untitled".to_string()))
            },
            crate::core::platform::container::content::ContentType::Image(_) => {
                format!("Image content: {}", content.title().unwrap_or(&"Untitled".to_string()))
            },
        }
    }

    /// Process the raw LLM result and enhance it with metadata if configured
    fn process_analysis_result(
        &self,
        raw_result: &str,
        content: &ContentItem,
        config: &LlmContentAnalysisConfig,
    ) -> Result<Value, String> {
        // Try to parse the result as JSON first
        let mut result = match serde_json::from_str::<Value>(raw_result) {
            Ok(json_value) => json_value,
            Err(_) => {
                // If parsing fails, wrap the raw result
                serde_json::json!({
                    "analysis": raw_result,
                    "format": "raw_text"
                })
            }
        };

        // Add metadata if configured
        if config.include_content_metadata {
            if let Value::Object(ref mut map) = result {
                let metadata = serde_json::json!({
                    "content_id": content.uuid(),
                    "content_title": content.title().unwrap_or(&"Untitled".to_string()),
                    "content_type": match content.content() {
                        crate::core::platform::container::content::ContentType::Text(_) => "text",
                        crate::core::platform::container::content::ContentType::Video(_) => "video",
                        crate::core::platform::container::content::ContentType::Audio(_) => "audio",
                        crate::core::platform::container::content::ContentType::Image(_) => "image",
                    },
                    "analysis_timestamp": chrono::Utc::now().to_rfc3339(),
                });
                map.insert("content_metadata".to_string(), metadata);
            }
        }

        Ok(result)
    }
}

/// Traditional ContentAnalysisService trait implementation for backward compatibility
/// This allows the LlmContentAnalyzer to work with existing code that expects this interface
pub trait ContentAnalysisService {
    fn analyze_content(&self, content: &ContentItem) -> Result<Value, String>;
}

/// Adapter that implements ContentAnalysisService using a default prompt
pub struct DefaultPromptContentAnalyzer {
    analyzer: LlmContentAnalyzer,
    default_prompt: PromptItem,
    config: LlmContentAnalysisConfig,
}

impl DefaultPromptContentAnalyzer {
    pub fn new(
        llm_service: Arc<LlmAnalysisService>,
        default_prompt: PromptItem,
        config: LlmContentAnalysisConfig,
    ) -> Self {
        Self {
            analyzer: LlmContentAnalyzer::new(llm_service),
            default_prompt,
            config,
        }
    }

    /// Create a default content analysis prompt
    pub fn create_default_analysis_prompt() -> Result<PromptItem, String> {
        use crate::core::platform::container::prompt::{PromptType, TextPrompt, PromptRole};

        let prompt_text = r#"Please analyze the following content and provide insights in JSON format:

{
  "main_topics": ["topic1", "topic2"],
  "key_information": ["fact1", "fact2"],
  "sentiment": "positive/negative/neutral",
  "tone": "formal/informal/technical/etc",
  "quality_score": 0.8,
  "summary": "Brief summary of the content",
  "suggestions": ["improvement1", "improvement2"]
}

Analyze the attached content according to these categories."#;

        let text_prompt = TextPrompt {
            content: prompt_text.to_string(),
            role: PromptRole::User,
        };

        PromptItem::new_with_title(
            PromptType::Text(text_prompt),
            "Default Content Analysis Prompt".to_string()
        ).map_err(|e| format!("Failed to create default prompt: {:?}", e))
    }
}

impl ContentAnalysisService for DefaultPromptContentAnalyzer {
    fn analyze_content(&self, content: &ContentItem) -> Result<Value, String> {
        let input = LlmContentAnalysisInput {
            prompt: self.default_prompt.clone(),
            content: content.clone(),
        };

        self.analyzer.analyze_with_prompt(&input, &self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::platform::container::content::{ContentType, TextContent};
    use crate::core::platform::container::prompt::{PromptType, TextPrompt, PromptRole};
    use crate::application::ports::output::llm_port::{LlmPort, LlmRequest, LlmResponse, LlmError, TokenUsage, FinishReason};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use uuid::Uuid;
    use chrono::Utc;

    struct MockLlmPort;

    #[async_trait]
    impl LlmPort for MockLlmPort {
        async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
            Ok(LlmResponse {
                id: Uuid::new_v4(),
                request_id: Uuid::new_v4(),
                model: "test-model".to_string(),
                content: r#"{"topics": ["test"], "sentiment": "positive", "quality_score": 0.8}"#.to_string(),
                finish_reason: FinishReason::Stop,
                usage: TokenUsage {
                    prompt_tokens: 100,
                    completion_tokens: 50,
                    total_tokens: 150,
                },
                created_at: Utc::now(),
                metadata: HashMap::new(),
            })
        }

        async fn generate_stream(&self, _request: LlmRequest) -> Result<Box<dyn futures::Stream<Item = Result<crate::application::ports::output::llm_port::StreamingResponse, LlmError>> + Send>, LlmError> {
            todo!()
        }

        async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
            Ok(true)
        }

        async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
            Ok(vec!["test-model".to_string()])
        }

        fn get_provider_name(&self) -> &'static str {
            "test"
        }
    }

    #[test]
    fn test_llm_content_analyzer_with_prompt() {
        let mock_port = Arc::new(MockLlmPort);
        let llm_service = Arc::new(LlmAnalysisService::new(mock_port));
        
        let analyzer = LlmContentAnalyzer::new(llm_service);

        // Create test content
        let text_content = TextContent::new(
            None,
            Some("This is test content for analysis".to_string())
        ).expect("Failed to create text content");

        let content = ContentItem::new_with_title(
            ContentType::Text(text_content),
            "Test Content".to_string(),
        ).expect("Failed to create content item");

        // Create test prompt
        let text_prompt = TextPrompt {
            content: "Analyze this content and return insights in JSON format.".to_string(),
            role: PromptRole::User,
        };

        let prompt = PromptItem::new_with_title(
            PromptType::Text(text_prompt),
            "Test Analysis Prompt".to_string(),
        ).expect("Failed to create prompt");

        // Create analysis input
        let input = LlmContentAnalysisInput {
            prompt,
            content,
        };

        let config = LlmContentAnalysisConfig::default();

        let result = analyzer.analyze_with_prompt(&input, &config);
        assert!(result.is_ok());
        
        let analysis = result.unwrap();
        assert!(analysis.is_object());

        // Check that metadata was added
        assert!(analysis.get("content_metadata").is_some());
    }

    #[test]
    fn test_default_prompt_content_analyzer() {
        let mock_port = Arc::new(MockLlmPort);
        let llm_service = Arc::new(LlmAnalysisService::new(mock_port));
        
        let default_prompt = DefaultPromptContentAnalyzer::create_default_analysis_prompt()
            .expect("Failed to create default prompt");
        
        let config = LlmContentAnalysisConfig::default();
        
        let analyzer = DefaultPromptContentAnalyzer::new(llm_service, default_prompt, config);

        // Create test content
        let text_content = TextContent::new(
            None,
            Some("This is test content for analysis".to_string())
        ).expect("Failed to create text content");

        let content = ContentItem::new_with_title(
            ContentType::Text(text_content),
            "Test Content".to_string(),
        ).expect("Failed to create content item");

        let result = analyzer.analyze_content(&content);
        assert!(result.is_ok());
        
        let analysis = result.unwrap();
        assert!(analysis.is_object());
    }

    #[test]
    fn test_content_length_validation() {
        let mock_port = Arc::new(MockLlmPort);
        let llm_service = Arc::new(LlmAnalysisService::new(mock_port));
        
        let analyzer = LlmContentAnalyzer::new(llm_service);

        // Create content that exceeds length limit
        let long_content = "a".repeat(1000);
        let text_content = TextContent::new(
            None,
            Some(long_content)
        ).expect("Failed to create text content");

        let content = ContentItem::new_with_title(
            ContentType::Text(text_content),
            "Long Content".to_string(),
        ).expect("Failed to create content item");

        let text_prompt = TextPrompt {
            content: "Analyze this content.".to_string(),
            role: PromptRole::User,
        };

        let prompt = PromptItem::new_with_title(
            PromptType::Text(text_prompt),
            "Test Prompt".to_string(),
        ).expect("Failed to create prompt");

        let input = LlmContentAnalysisInput { prompt, content };
        
        // Set a low max content length
        let config = LlmContentAnalysisConfig {
            max_content_length: Some(100),
            ..Default::default()
        };

        let result = analyzer.analyze_with_prompt(&input, &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("exceeds maximum allowed length"));
    }
}