//! OpenAI Vision Extension
//!
//! Extends [`OpenAIAdapter`] with vision capabilities for multimodal requests.
//! Supports GPT-4o, GPT-4 Vision Preview, and GPT-4o-mini models.
//!
//! # Configuration
//!
//! Vision behaviour is controlled by [`VisionConfig`], which falls back to
//! [`VisionConfig::default`] when not explicitly provided.

use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose};
use chrono::Utc;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;

use paladin_core::platform::container::vision::{
    ImageDetail, VisionContent, VisionError, VisionRequest,
};
use paladin_ports::output::llm_port::{
    FinishReason, LlmError, LlmRequest, LlmResponse, TokenUsage,
};
use paladin_ports::output::vision_llm_port::VisionCapableLlm;
use paladin_ports::output::vision_port::{VisionPort, VisionResult, VisionTokenUsage};

use super::adapter::OpenAIAdapter;

// ── VisionConfig types ───────────────────────────────────────────────────────

/// Retry configuration for vision API calls.
#[derive(Debug, Clone)]
pub struct VisionRetryConfig {
    /// Maximum number of retry attempts for transient errors.
    pub max_retries: u32,
    /// Initial backoff delay in milliseconds.
    pub initial_backoff_ms: u64,
    /// Multiplier for exponential backoff (e.g., 2.0 for doubling).
    pub backoff_multiplier: f64,
}

impl Default for VisionRetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 1000,
            backoff_multiplier: 2.0,
        }
    }
}

/// Configuration for a single vision provider.
#[derive(Debug, Clone)]
pub struct VisionProviderConfig {
    /// Maximum tokens to request in vision responses.
    pub max_tokens: usize,
}

impl Default for VisionProviderConfig {
    fn default() -> Self {
        Self { max_tokens: 4096 }
    }
}

/// Configuration for vision capabilities (multi-modal image analysis).
#[derive(Debug, Clone, Default)]
pub struct VisionConfig {
    /// Retry configuration for vision API calls.
    pub retry: VisionRetryConfig,
    /// OpenAI vision provider configuration.
    pub openai: VisionProviderConfig,
    /// Anthropic vision provider configuration.
    pub anthropic: VisionProviderConfig,
}

// ── OpenAI vision message types ──────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
enum OpenAIContentPart {
    Text { text: String },
    ImageUrl { image_url: OpenAIImageUrl },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct OpenAIImageUrl {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct OpenAIVisionMessage {
    role: String,
    content: Vec<OpenAIContentPart>,
}

/// Vision-capable models supported by OpenAI.
const VISION_MODELS: &[&str] = &[
    "gpt-4o",
    "gpt-4o-mini",
    "gpt-4-vision-preview",
    "gpt-4-turbo",
    "gpt-4-turbo-2024-04-09",
];

#[derive(Debug, Serialize)]
struct OpenAIVisionApiRequest {
    model: String,
    messages: Vec<OpenAIVisionMessage>,
    max_tokens: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct OpenAIVisionApiResponse {
    #[allow(dead_code)]
    id: String,
    model: String,
    choices: Vec<OpenAIVisionChoice>,
    usage: OpenAIVisionUsage,
}

#[derive(Debug, Deserialize)]
struct OpenAIVisionChoice {
    message: OpenAIVisionResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIVisionResponseMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIVisionUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct OpenAIErrorResponse {
    error: OpenAIErrorDetails,
}

#[derive(Debug, Deserialize)]
struct OpenAIErrorDetails {
    message: String,
    #[allow(dead_code)]
    #[serde(rename = "type")]
    error_type: Option<String>,
}

// ── OpenAIAdapter vision helper methods ──────────────────────────────────────

impl OpenAIAdapter {
    /// Check if a model supports vision.
    pub fn is_vision_model(model: &str) -> bool {
        VISION_MODELS.contains(&model)
    }

    fn calculate_backoff_delay(
        retry_attempt: u32,
        initial_backoff_ms: u64,
        backoff_multiplier: f64,
    ) -> u64 {
        let delay = initial_backoff_ms as f64 * backoff_multiplier.powi(retry_attempt as i32);
        delay as u64
    }

    fn is_transient_error(status: StatusCode) -> bool {
        matches!(
            status,
            StatusCode::TOO_MANY_REQUESTS
                | StatusCode::INTERNAL_SERVER_ERROR
                | StatusCode::BAD_GATEWAY
                | StatusCode::SERVICE_UNAVAILABLE
                | StatusCode::GATEWAY_TIMEOUT
        )
    }

    fn map_status_to_error(status: StatusCode, message: String) -> VisionError {
        match status {
            StatusCode::BAD_REQUEST => VisionError::InvalidImage(message),
            StatusCode::UNAUTHORIZED => VisionError::AuthenticationError(message),
            StatusCode::TOO_MANY_REQUESTS => VisionError::RateLimitExceeded(message),
            StatusCode::INTERNAL_SERVER_ERROR
            | StatusCode::BAD_GATEWAY
            | StatusCode::SERVICE_UNAVAILABLE
            | StatusCode::GATEWAY_TIMEOUT => VisionError::ProviderError(message),
            _ => VisionError::ProviderError(format!("HTTP {}: {}", status.as_u16(), message)),
        }
    }

    /// Execute vision API call with retry logic.
    async fn execute_vision_request(
        &self,
        request_body: OpenAIVisionApiRequest,
        vision_config: &VisionConfig,
    ) -> Result<OpenAIVisionApiResponse, VisionError> {
        let max_retries = vision_config.retry.max_retries;
        let mut last_error: Option<VisionError> = None;

        for attempt in 0..=max_retries {
            let response = self
                .client
                .post(format!("{}/chat/completions", self.config.base_url))
                .header("Authorization", format!("Bearer {}", self.config.api_key))
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .await
                .map_err(|e| VisionError::NetworkError(format!("Request failed: {}", e)))?;

            let status = response.status();

            if status.is_success() {
                let api_response: OpenAIVisionApiResponse = response.json().await.map_err(|e| {
                    VisionError::ProviderError(format!("Failed to parse response: {}", e))
                })?;
                return Ok(api_response);
            }

            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error response".to_string());

            let error_message =
                if let Ok(err) = serde_json::from_str::<OpenAIErrorResponse>(&error_text) {
                    err.error.message
                } else {
                    error_text
                };

            let error = Self::map_status_to_error(status, error_message);

            if attempt < max_retries && Self::is_transient_error(status) {
                last_error = Some(error);
                let delay_ms = Self::calculate_backoff_delay(
                    attempt,
                    vision_config.retry.initial_backoff_ms,
                    vision_config.retry.backoff_multiplier,
                );
                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                continue;
            }

            return Err(error);
        }

        Err(last_error.unwrap_or(VisionError::MaxRetriesExceeded(max_retries)))
    }

    /// Convert [`VisionContent`] to OpenAI format.
    async fn convert_vision_content(
        &self,
        content: &VisionContent,
    ) -> Result<OpenAIContentPart, LlmError> {
        match content {
            VisionContent::ImageUrl { url, detail } => Ok(OpenAIContentPart::ImageUrl {
                image_url: OpenAIImageUrl {
                    url: url.clone(),
                    detail: Self::convert_detail(*detail),
                },
            }),
            VisionContent::ImageBase64 {
                data,
                media_type,
                detail,
            } => {
                let data_url = format!("data:{};base64,{}", media_type, data);
                Ok(OpenAIContentPart::ImageUrl {
                    image_url: OpenAIImageUrl {
                        url: data_url,
                        detail: Self::convert_detail(*detail),
                    },
                })
            }
            VisionContent::ImageFile { path, detail } => {
                let image_data = fs::read(path).await.map_err(|e| {
                    LlmError::ProcessingError(format!("Failed to read image file: {}", e))
                })?;
                let mime_type = Self::detect_mime_type(path)?;
                let base64_data = general_purpose::STANDARD.encode(&image_data);
                let data_url = format!("data:{};base64,{}", mime_type, base64_data);
                Ok(OpenAIContentPart::ImageUrl {
                    image_url: OpenAIImageUrl {
                        url: data_url,
                        detail: Self::convert_detail(*detail),
                    },
                })
            }
        }
    }

    fn convert_detail(detail: ImageDetail) -> Option<String> {
        match detail {
            ImageDetail::Auto => None,
            ImageDetail::Low => Some("low".to_string()),
            ImageDetail::High => Some("high".to_string()),
        }
    }

    fn detect_mime_type(path: &Path) -> Result<String, LlmError> {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| LlmError::InvalidPrompt("Image file has no extension".to_string()))?
            .to_lowercase();

        match extension.as_str() {
            "jpg" | "jpeg" => Ok("image/jpeg".to_string()),
            "png" => Ok("image/png".to_string()),
            "gif" => Ok("image/gif".to_string()),
            "webp" => Ok("image/webp".to_string()),
            _ => Err(LlmError::InvalidPrompt(format!(
                "Unsupported image format: {}",
                extension
            ))),
        }
    }

    async fn build_vision_messages(
        &self,
        request: &LlmRequest,
        vision: &VisionRequest,
    ) -> Result<Vec<OpenAIVisionMessage>, LlmError> {
        if !Self::is_vision_model(&request.model) {
            return Err(LlmError::ModelNotAvailable(format!(
                "Model {} does not support vision. Supported: {}",
                request.model,
                VISION_MODELS.join(", ")
            )));
        }

        let mut content_parts = vec![];

        if !vision.text.is_empty() {
            content_parts.push(OpenAIContentPart::Text {
                text: vision.text.clone(),
            });
        }

        for image in &vision.images {
            let image_part = self.convert_vision_content(image).await?;
            content_parts.push(image_part);
        }

        Ok(vec![OpenAIVisionMessage {
            role: "user".to_string(),
            content: content_parts,
        }])
    }
}

// ── VisionCapableLlm impl ─────────────────────────────────────────────────────

#[async_trait]
impl VisionCapableLlm for OpenAIAdapter {
    async fn generate_with_vision(
        &self,
        request: LlmRequest,
        vision: VisionRequest,
    ) -> Result<LlmResponse, LlmError> {
        let vision_config = VisionConfig::default();

        let messages = self.build_vision_messages(&request, &vision).await?;

        let request_body = OpenAIVisionApiRequest {
            model: request.model.clone(),
            messages,
            max_tokens: vision_config.openai.max_tokens,
            temperature: request
                .metadata
                .get("temperature")
                .and_then(|v| v.parse::<f32>().ok()),
        };

        let api_response = self
            .execute_vision_request(request_body, &vision_config)
            .await
            .map_err(|e| match e {
                VisionError::InvalidImage(msg) => LlmError::InvalidPrompt(msg),
                VisionError::AuthenticationError(msg) => LlmError::AuthenticationError(msg),
                VisionError::RateLimitExceeded(msg) => {
                    LlmError::ProcessingError(format!("Rate limit exceeded: {}", msg))
                }
                VisionError::NetworkError(msg) => LlmError::NetworkError(msg),
                VisionError::ProviderError(msg) | VisionError::UnsupportedProvider(msg) => {
                    LlmError::ProcessingError(msg)
                }
                VisionError::Timeout(seconds) => LlmError::Timeout(format!("{} seconds", seconds)),
                VisionError::MaxRetriesExceeded(attempts) => LlmError::ProcessingError(format!(
                    "Max retries exceeded: {} attempts",
                    attempts
                )),
                _ => LlmError::ProcessingError(format!("Vision error: {}", e)),
            })?;

        let content = api_response
            .choices
            .first()
            .map(|choice| choice.message.content.clone())
            .ok_or_else(|| LlmError::ProcessingError("No response content".to_string()))?;

        let finish_reason = api_response
            .choices
            .first()
            .and_then(|choice| choice.finish_reason.as_ref())
            .map(|reason| match reason.as_str() {
                "stop" => FinishReason::Stop,
                "length" => FinishReason::Length,
                "content_filter" => FinishReason::ContentFilter,
                _ => FinishReason::Error(reason.clone()),
            })
            .unwrap_or(FinishReason::Stop);

        Ok(LlmResponse {
            id: uuid::Uuid::new_v4(),
            request_id: request.id,
            model: api_response.model,
            content,
            finish_reason,
            usage: TokenUsage::new(
                api_response.usage.prompt_tokens,
                api_response.usage.completion_tokens,
            ),
            created_at: Utc::now(),
            metadata: Default::default(),
            function_call: None,
        })
    }

    fn supports_vision(&self) -> bool {
        true
    }
}

// ── VisionPort impl ───────────────────────────────────────────────────────────

#[async_trait]
impl VisionPort for OpenAIAdapter {
    async fn analyze_image(
        &self,
        prompt: &str,
        images: Vec<VisionContent>,
        model: &str,
        max_tokens: Option<u32>,
    ) -> Result<VisionResult, VisionError> {
        if !Self::is_vision_model(model) {
            return Err(VisionError::ModelNotSupported(format!(
                "Model {} does not support vision",
                model
            )));
        }

        if images.is_empty() {
            return Err(VisionError::InvalidRequest(
                "At least one image must be provided".to_string(),
            ));
        }

        let mut content_parts = vec![OpenAIContentPart::Text {
            text: prompt.to_string(),
        }];

        for image in images {
            let image_part = self.convert_vision_content(&image).await.map_err(|e| {
                VisionError::InvalidRequest(format!("Failed to convert image: {}", e))
            })?;
            content_parts.push(image_part);
        }

        let vision_message = OpenAIVisionMessage {
            role: "user".to_string(),
            content: content_parts,
        };

        let request = OpenAIVisionApiRequest {
            model: model.to_string(),
            messages: vec![vision_message],
            max_tokens: max_tokens.unwrap_or(1000) as usize,
            temperature: None,
        };

        let vision_config = VisionConfig::default();
        let response = self.execute_vision_request(request, &vision_config).await?;

        let content = response
            .choices
            .first()
            .ok_or_else(|| VisionError::InvalidRequest("No choices in response".to_string()))?
            .message
            .content
            .clone();

        Ok(VisionResult {
            content,
            model: response.model,
            token_usage: VisionTokenUsage {
                prompt_tokens: response.usage.prompt_tokens,
                completion_tokens: response.usage.completion_tokens,
                total_tokens: response.usage.total_tokens,
            },
            metadata: std::collections::HashMap::new(),
            timestamp: chrono::Utc::now(),
        })
    }

    fn is_vision_model(&self, model: &str) -> bool {
        Self::is_vision_model(model)
    }

    fn provider_name(&self) -> &str {
        "openai"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openai::OpenAIConfig;

    fn create_test_adapter() -> OpenAIAdapter {
        let config = OpenAIConfig::new("test-key".to_string());
        OpenAIAdapter::new(config).unwrap()
    }

    #[test]
    fn test_is_vision_model() {
        assert!(OpenAIAdapter::is_vision_model("gpt-4o"));
        assert!(OpenAIAdapter::is_vision_model("gpt-4o-mini"));
        assert!(OpenAIAdapter::is_vision_model("gpt-4-vision-preview"));
        assert!(OpenAIAdapter::is_vision_model("gpt-4-turbo"));
        assert!(!OpenAIAdapter::is_vision_model("gpt-3.5-turbo"));
        assert!(!OpenAIAdapter::is_vision_model("gpt-4"));
    }

    #[test]
    fn test_convert_detail() {
        assert_eq!(OpenAIAdapter::convert_detail(ImageDetail::Auto), None);
        assert_eq!(
            OpenAIAdapter::convert_detail(ImageDetail::Low),
            Some("low".to_string())
        );
        assert_eq!(
            OpenAIAdapter::convert_detail(ImageDetail::High),
            Some("high".to_string())
        );
    }

    #[test]
    fn test_vision_config_default() {
        let adapter = create_test_adapter();
        assert!(adapter.supports_vision());
        assert_eq!(adapter.provider_name(), "openai");
    }

    #[test]
    fn test_vision_retry_config_default() {
        let config = VisionRetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_backoff_ms, 1000);
        assert_eq!(config.backoff_multiplier, 2.0);
    }
}
