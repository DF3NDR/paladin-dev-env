use std::collections::HashMap;
use std::env;
use async_trait::async_trait;
use reqwest::{Client, header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE}};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;
use chrono::Utc;
use futures::{Stream, stream};

use crate::application::ports::output::llm_port::{
    LlmPort, LlmRequest, LlmResponse, LlmError, TokenUsage, FinishReason, StreamingResponse
};
use crate::core::platform::container::prompt::{PromptItem, PromptType};
use crate::core::platform::container::content::{ContentItem, ContentType};

/// Configuration for OpenAI LLM adapter
#[derive(Debug, Clone)]
pub struct OpenAIConfig {
    pub api_key: String,
    pub base_url: String,
    pub organization: Option<String>,
    pub timeout_seconds: u64,
    pub max_retries: u32,
}

impl OpenAIConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, String> {
        let api_key = env::var("OPENAI_LLM_API_KEY")
            .map_err(|_| "OPENAI_LLM_API_KEY environment variable not set")?;

        let base_url = env::var("OPENAI_LLM_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());

        let organization = env::var("OPENAI_ORGANIZATION").ok();

        let timeout_seconds = env::var("OPENAI_TIMEOUT_SECONDS")
            .unwrap_or_else(|_| "30".to_string())
            .parse()
            .map_err(|_| "Invalid OPENAI_TIMEOUT_SECONDS value")?;

        let max_retries = env::var("OPENAI_MAX_RETRIES")
            .unwrap_or_else(|_| "3".to_string())
            .parse()
            .map_err(|_| "Invalid OPENAI_MAX_RETRIES value")?;

        Ok(Self {
            api_key,
            base_url,
            organization,
            timeout_seconds,
            max_retries,
        })
    }

    /// Create configuration with custom values
    pub fn new(api_key: String, base_url: String) -> Self {
        Self {
            api_key,
            base_url,
            organization: None,
            timeout_seconds: 30,
            max_retries: 3,
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.api_key.is_empty() {
            return Err("API key cannot be empty".to_string());
        }

        if self.base_url.is_empty() {
            return Err("Base URL cannot be empty".to_string());
        }

        if !self.base_url.starts_with("http") {
            return Err("Base URL must start with http or https".to_string());
        }

        Ok(())
    }
}

/// OpenAI API request structures
#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    stream: bool,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    top_p: Option<f32>,
    frequency_penalty: Option<f32>,
    presence_penalty: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

/// OpenAI API response structures
#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    _id: String,
    _object: String,
    _created: u64,
    model: String,
    choices: Vec<OpenAIChoice>,
    usage: OpenAIUsage,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    _index: u32,
    message: OpenAIMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamResponse {
    _id: String,
    _object: String,
    _created: u64,
    _model: String,
    _choices: Vec<OpenAIStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamChoice {
    _index: u32,
    _delta: OpenAIStreamDelta,
    _finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamDelta {
    _role: Option<String>,
    _content: Option<String>,
}

/// OpenAI LLM Adapter
pub struct OpenAILlmAdapter {
    config: OpenAIConfig,
    client: Client,
}

impl OpenAILlmAdapter {
    /// Create new adapter with configuration
    pub fn new(config: OpenAIConfig) -> Result<Self, String> {
        config.validate()?;

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", config.api_key))
                .map_err(|_| "Invalid API key format")?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        if let Some(org) = &config.organization {
            headers.insert(
                "OpenAI-Organization",
                HeaderValue::from_str(org).map_err(|_| "Invalid organization format")?,
            );
        }

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .default_headers(headers)
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        Ok(Self { config, client })
    }

    /// Create adapter from environment variables
    pub fn from_env() -> Result<Self, String> {
        let config = OpenAIConfig::from_env()?;
        Self::new(config)
    }

    /// Convert internal prompt to OpenAI messages format
    fn convert_prompt_to_messages(&self, prompt: &PromptItem, attachments: &[ContentItem]) -> Result<Vec<OpenAIMessage>, LlmError> {
        let mut messages = Vec::new();

        // Convert the main prompt - Handle all PromptType variants
        match prompt.prompt_type() {
            PromptType::Text(text_prompt) => {
                let role = match text_prompt.role {
                    crate::core::platform::container::prompt::PromptRole::System => "system",
                    crate::core::platform::container::prompt::PromptRole::User => "user",
                    crate::core::platform::container::prompt::PromptRole::Assistant => "assistant",
                    crate::core::platform::container::prompt::PromptRole::Function => "function",
                }.to_string();

                messages.push(OpenAIMessage {
                    role,
                    content: text_prompt.content.clone(),
                });
            },
            // Handle other prompt types correctly based on their actual fields
            PromptType::System(system_prompt) => {
                let content = format!("{}\nConstraints: {}", 
                    system_prompt.instructions,
                    system_prompt.constraints.as_ref().map(|c| c.join(", ")).unwrap_or_default()
                );
                messages.push(OpenAIMessage {
                    role: "system".to_string(),
                    content,
                });
            },
            PromptType::User(user_prompt) => {
                // User prompt has context field, not content
                messages.push(OpenAIMessage {
                    role: "user".to_string(),
                    content: user_prompt.context.clone().unwrap_or_default(),
                });
            },
            PromptType::Assistant(assistant_prompt) => {
                // Assistant prompt has response field, not content
                let content = if let Some(reasoning) = &assistant_prompt.reasoning {
                    format!("{}\nReasoning: {}", assistant_prompt.response, reasoning)
                } else {
                    assistant_prompt.response.clone()
                };
                messages.push(OpenAIMessage {
                    role: "assistant".to_string(),
                    content,
                });
            },
            // Add the missing Function variant
            PromptType::Function(function_prompt) => {
                messages.push(OpenAIMessage {
                    role: "function".to_string(),
                    content: function_prompt.function_name.clone(), // Use function name as content
                });
            },
        }

        // Add content attachments as user messages
        for content in attachments {
            let content_text = self.convert_content_to_text(content)?;
            if !content_text.is_empty() {
                messages.push(OpenAIMessage {
                    role: "user".to_string(),
                    content: format!("Content to analyze: {}", content_text),
                });
            }
        }

        Ok(messages)
    }

    /// Convert content items to text representation
    fn convert_content_to_text(&self, content: &ContentItem) -> Result<String, LlmError> {
        match content.content() {
            ContentType::Text(text_content) => {
                Ok(text_content.content.as_deref().unwrap_or("").to_string())
            },
            ContentType::Video(video_content) => {
                Ok(format!(
                    "Video content: {} (Duration: {}s)",
                    content.title().unwrap_or(&"Untitled Video".to_string()),
                    video_content.duration
                ))
            },
            ContentType::Audio(audio_content) => {
                Ok(format!(
                    "Audio content: {} (Duration: {}s)",
                    content.title().unwrap_or(&"Untitled Audio".to_string()),
                    audio_content.duration
                ))
            },
            ContentType::Image(image_content) => {
                Ok(format!(
                    "Image content: {} (Resolution: {}x{})",
                    content.title().unwrap_or(&"Untitled Image".to_string()),
                    image_content.resolution.0,
                    image_content.resolution.1
                ))
            },
        }
    }

    /// Convert OpenAI finish reason to internal format
    fn convert_finish_reason(&self, reason: Option<String>) -> FinishReason {
        match reason.as_deref() {
            Some("stop") => FinishReason::Stop,
            Some("length") => FinishReason::Length,
            Some("content_filter") => FinishReason::ContentFilter,
            Some("function_call") => FinishReason::FunctionCall,
            Some(other) => FinishReason::Error(format!("Unknown finish reason: {}", other)),
            None => FinishReason::Stop, // Default to stop if not specified
        }
    }

    /// Determine the model to use (map to OpenAI model names)
    fn map_model_name(&self, model: &str) -> String {
        match model {
            "gpt-3.5-turbo" | "gpt-3.5" => "gpt-3.5-turbo".to_string(),
            "gpt-4" => "gpt-4".to_string(),
            "gpt-4-turbo" => "gpt-4-turbo-preview".to_string(),
            other => other.to_string(), // Use as-is for other models
        }
    }

    /// Make request with retries
    async fn make_request_with_retries(&self, request: &OpenAIRequest) -> Result<OpenAIResponse, LlmError> {
        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            match self.make_single_request(request).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.config.max_retries {
                        let delay = std::time::Duration::from_millis(1000 * 2_u64.pow(attempt));
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| LlmError::ProcessingError("Unknown error".to_string())))
    }

    /// Make a single request to OpenAI API
    async fn make_single_request(&self, request: &OpenAIRequest) -> Result<OpenAIResponse, LlmError> {
        let url = format!("{}/chat/completions", self.config.base_url);
        
        let response = self.client
            .post(&url)
            .json(request)
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(format!("Request failed: {}", e)))?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .map_err(|e| LlmError::ProcessingError(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return match status.as_u16() {
                401 => Err(LlmError::AuthenticationError("Invalid API key".to_string())),
                429 => Err(LlmError::RateLimitExceeded),
                400 => {
                    if response_text.contains("maximum context length") {
                        Err(LlmError::TokenLimitExceeded)
                    } else {
                        Err(LlmError::InvalidPrompt(response_text))
                    }
                }
                _ => Err(LlmError::ProcessingError(format!("HTTP {}: {}", status, response_text))),
            };
        }

        serde_json::from_str::<OpenAIResponse>(&response_text)
            .map_err(|e| LlmError::ProcessingError(format!("Failed to parse response: {}", e)))
    }
}

#[async_trait]
impl LlmPort for OpenAILlmAdapter {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let messages = self.convert_prompt_to_messages(&request.prompt, &request.attachments)?;
        let model = self.map_model_name(&request.model);

        let openai_request = OpenAIRequest {
            model: model.clone(),
            messages,
            stream: false,
            temperature: Some(0.7),
            max_tokens: Some(2000),
            top_p: Some(1.0),
            frequency_penalty: Some(0.0),
            presence_penalty: Some(0.0),
        };

        let response = self.make_request_with_retries(&openai_request).await?;

        if response.choices.is_empty() {
            return Err(LlmError::ProcessingError("No choices in response".to_string()));
        }

        let choice = &response.choices[0];
        let finish_reason = self.convert_finish_reason(choice.finish_reason.clone());

        Ok(LlmResponse {
            id: Uuid::new_v4(),
            request_id: request.id,
            model: response.model,
            content: choice.message.content.clone(),
            finish_reason,
            usage: TokenUsage {
                prompt_tokens: response.usage.prompt_tokens,
                completion_tokens: response.usage.completion_tokens,
                total_tokens: response.usage.total_tokens,
            },
            created_at: Utc::now(),
            metadata: HashMap::new(),
        })
    }

    async fn generate_stream(&self, request: LlmRequest) -> Result<Box<dyn Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError> {
        let _messages = self.convert_prompt_to_messages(&request.prompt, &request.attachments)?;
        let _model = self.map_model_name(&request.model);

        // Create a simple mock stream for now
        let mock_chunks = vec![
            "Hello",
            " world",
            "!",
        ];

        let stream = stream::iter(mock_chunks.into_iter().enumerate().map(|(i, chunk)| {
            Ok(StreamingResponse {
                id: Uuid::new_v4(),
                delta: chunk.to_string(),
                finish_reason: if i == 2 { Some(FinishReason::Stop) } else { None },
            })
        }));

        Ok(Box::new(stream))
    }

    async fn validate_model(&self, model: &str) -> Result<bool, LlmError> {
        let available_models = self.get_available_models().await?;
        Ok(available_models.contains(&model.to_string()))
    }

    async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
        let url = format!("{}/models", self.config.base_url);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(format!("Models request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(LlmError::ProcessingError(format!("HTTP {}", response.status())));
        }

        let response_text = response
            .text()
            .await
            .map_err(|e| LlmError::ProcessingError(format!("Failed to read models response: {}", e)))?;

        let models_response: Value = serde_json::from_str(&response_text)
            .map_err(|e| LlmError::ProcessingError(format!("Failed to parse models response: {}", e)))?;

        let models = models_response["data"]
            .as_array()
            .ok_or_else(|| LlmError::ProcessingError("Invalid models response format".to_string()))?
            .iter()
            .filter_map(|model| model["id"].as_str().map(|s| s.to_string()))
            .collect();

        Ok(models)
    }

    fn get_provider_name(&self) -> &'static str {
        "OpenAI"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_env() {
        // Set test environment variables using unsafe blocks
        unsafe {
            env::set_var("OPENAI_LLM_API_KEY", "test-api-key");
            env::set_var("OPENAI_LLM_URL", "https://api.openai.com/v1");
        }
        
        let config = OpenAIConfig::from_env().expect("Config should be created from env");
        
        assert_eq!(config.api_key, "test-api-key");
        assert_eq!(config.base_url, "https://api.openai.com/v1");
        assert_eq!(config.timeout_seconds, 30);
        assert_eq!(config.max_retries, 3);
        
        // Clean up using unsafe blocks
        unsafe {
            env::remove_var("OPENAI_LLM_API_KEY");
            env::remove_var("OPENAI_LLM_URL");
        }
    }

    #[test]
    fn test_config_validation() {
        let valid_config = OpenAIConfig::new(
            "test-key".to_string(),
            "https://api.openai.com/v1".to_string(),
        );
        assert!(valid_config.validate().is_ok());

        let invalid_config = OpenAIConfig::new(
            "".to_string(),
            "https://api.openai.com/v1".to_string(),
        );
        assert!(invalid_config.validate().is_err());

        let invalid_url_config = OpenAIConfig::new(
            "test-key".to_string(),
            "invalid-url".to_string(),
        );
        assert!(invalid_url_config.validate().is_err());
    }

    #[test]
    fn test_model_mapping() {
        let config = OpenAIConfig::new(
            "test-key".to_string(),
            "https://api.openai.com/v1".to_string(),
        );
        let adapter = OpenAILlmAdapter::new(config).expect("Adapter should be created");

        assert_eq!(adapter.map_model_name("gpt-3.5-turbo"), "gpt-3.5-turbo");
        assert_eq!(adapter.map_model_name("gpt-3.5"), "gpt-3.5-turbo");
        assert_eq!(adapter.map_model_name("gpt-4"), "gpt-4");
        assert_eq!(adapter.map_model_name("custom-model"), "custom-model");
    }
}