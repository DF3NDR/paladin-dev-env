use std::sync::Arc;
use serde_json::Value;

use crate::core::platform::container::content::ContentItem;
use crate::core::platform::container::prompt::{PromptItem, PromptType, TextPrompt, PromptRole};
use crate::core::base::service::analysis_service::AnalysisService;
use crate::application::use_cases::analysis::llm_analysis_service::{
    LlmAnalysisService, LlmAnalysisInput, LlmAnalysisConfig
};
use crate::application::use_cases::content::content_analysis_service::ContentAnalysisService;

pub struct LlmContentAnalyzer {
    llm_service: Arc<LlmAnalysisService>,
    default_config: LlmAnalysisConfig,
}

impl LlmContentAnalyzer {
    pub fn new(llm_service: Arc<LlmAnalysisService>, default_config: LlmAnalysisConfig) -> Self {
        Self {
            llm_service,
            default_config,
        }
    }

    fn create_analysis_prompt(&self, content: &ContentItem) -> Result<PromptItem, String> {
        let content_text = match content.content() {
            crate::core::platform::container::content::ContentType::Text(text) => {
                text.content.as_deref().unwrap_or("No content")
            },
            _ => "Non-text content",
        };

        let prompt_text = format!(
            "Please analyze the following content and provide insights about:
            1. Main topics and themes
            2. Key information and facts
            3. Sentiment and tone
            4. Quality assessment
            5. Potential improvements or questions

            Content Title: {}
            Content: {}
            
            Please provide your analysis in a structured JSON format with the following structure:
            {{
                \"topics\": [\"topic1\", \"topic2\"],
                \"key_facts\": [\"fact1\", \"fact2\"],
                \"sentiment\": \"positive/negative/neutral\",
                \"tone\": \"formal/informal/technical/etc\",
                \"quality_score\": 0.8,
                \"suggestions\": [\"suggestion1\", \"suggestion2\"]
            }}",
            content.title().unwrap_or(&"Untitled".to_string()),
            content_text
        );

        let text_prompt = TextPrompt {
            content: prompt_text,
            role: PromptRole::User,
        };

        PromptItem::new_with_title(
            PromptType::Text(text_prompt),
            "Content Analysis Prompt".to_string()
        ).map_err(|e| format!("Failed to create prompt: {:?}", e))
    }
}

impl ContentAnalysisService for LlmContentAnalyzer {
    fn analyze_content(&self, content: &ContentItem) -> Result<Value, String> {
        // Create analysis prompt
        let prompt = self.create_analysis_prompt(content)?;
        
        // Prepare analysis input
        let input = LlmAnalysisInput {
            prompt,
            content_attachments: vec![content.clone()],
        };

        // Perform analysis - now the analyze method should be available
        let result = self.llm_service
            .analyze(&input, &self.default_config)
            .map_err(|e| format!("Analysis failed: {:?}", e))?;

        // Try to parse the result as JSON, fall back to simple structure if it fails
        let analysis_content = &result.result.content;
        
        // Try to parse as JSON first
        match serde_json::from_str::<Value>(analysis_content) {
            Ok(json_value) => Ok(json_value),
            Err(_) => {
                // If parsing fails, create a simple structure
                let mut analysis = serde_json::Map::new();
                analysis.insert("content".to_string(), Value::String(analysis_content.clone()));
                analysis.insert("model_used".to_string(), Value::String(result.result.model_used));
                analysis.insert("token_usage".to_string(), serde_json::to_value(result.result.token_usage)
                    .map_err(|e| format!("Failed to serialize token usage: {}", e))?);
                analysis.insert("processing_time_ms".to_string(), Value::Number(
                    serde_json::Number::from(result.result.processing_time_ms)
                ));
                
                Ok(Value::Object(analysis))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::platform::container::content::{ContentType, TextContent};
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
    fn test_llm_content_analyzer() {
        let mock_port = Arc::new(MockLlmPort);
        let llm_service = Arc::new(LlmAnalysisService::new(mock_port));
        
        let config = LlmAnalysisConfig {
            model: "test-model".to_string(),
            max_retries: 1,
            timeout_seconds: 30,
            enable_streaming: false,
        };

        let analyzer = LlmContentAnalyzer::new(llm_service, config);

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
}