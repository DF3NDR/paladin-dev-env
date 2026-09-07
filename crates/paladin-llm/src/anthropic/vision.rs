//! Anthropic Claude Vision Extension
//!
//! Extends [`AnthropicAdapter`] with vision capabilities for Claude 3+ models.
//! Supports Claude 3 Opus, Sonnet, Haiku and Claude 3.5 variants with
//! multimodal content blocks.
//!
//! **Note**: Anthropic requires all images to be base64-encoded.
//! URL and file-path images are automatically downloaded/read and converted.

use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose};
use chrono::Utc;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;

use paladin_core::platform::container::vision::{VisionContent, VisionError, VisionRequest};
use paladin_ports::output::llm_port::{
    FinishReason, LlmError, LlmRequest, LlmResponse, TokenUsage,
};
use paladin_ports::output::vision_llm_port::VisionCapableLlm;
use paladin_ports::output::vision_port::{VisionPort, VisionResult, VisionTokenUsage};

use super::adapter::AnthropicAdapter;

// ── Vision config (defaults) ──────────────────────────────────────────────────

/// Maximum tokens for Anthropic vision responses when not configured elsewhere.
const DEFAULT_MAX_TOKENS: usize = 4096;
/// Maximum retry attempts for transient errors.
const DEFAULT_MAX_RETRIES: u32 = 3;
/// Initial backoff in milliseconds.
const DEFAULT_INITIAL_BACKOFF_MS: u64 = 1000;
/// Exponential backoff multiplier.
const DEFAULT_BACKOFF_MULTIPLIER: f64 = 2.0;

// ── Anthropic multimodal message types ───────────────────────────────────────

/// Anthropic content block for multimodal messages.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClaudeContentBlock {
    Text { text: String },
    Image { source: ClaudeImageSource },
}

/// Anthropic image source (base64 encoding only).
#[derive(Debug, Serialize, Deserialize, Clone)]
struct ClaudeImageSource {
    #[serde(rename = "type")]
    source_type: String, // always "base64"
    media_type: String,
    data: String,
}

/// Anthropic vision message.
#[derive(Debug, Serialize)]
struct ClaudeVisionMessage {
    role: String,
    content: Vec<ClaudeContentBlock>,
}

/// Anthropic vision API request.
#[derive(Debug, Serialize)]
struct ClaudeVisionApiRequest {
    model: String,
    messages: Vec<ClaudeVisionMessage>,
    max_tokens: usize,
}

/// Anthropic vision API response.
#[derive(Debug, Deserialize)]
struct ClaudeVisionApiResponse {
    #[allow(dead_code)]
    id: String,
    model: String,
    content: Vec<ClaudeResponseContent>,
    usage: ClaudeVisionUsage,
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClaudeResponseContent {
    Text { text: String },
}

#[derive(Debug, Deserialize)]
struct ClaudeVisionUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct ClaudeErrorResponse {
    error: ClaudeErrorDetails,
}

#[derive(Debug, Deserialize)]
struct ClaudeErrorDetails {
    message: String,
    #[allow(dead_code)]
    #[serde(rename = "type")]
    error_type: String,
}

/// Vision-capable Claude 3+ models.
const VISION_MODELS: &[&str] = &[
    "claude-3-opus-20240229",
    "claude-3-sonnet-20240229",
    "claude-3-haiku-20240307",
    "claude-3-5-sonnet-20240620",
    "claude-3-5-sonnet-20241022",
    "claude-3-5-haiku-20241022",
];

// ── AnthropicAdapter vision helper methods ────────────────────────────────────

impl AnthropicAdapter {
    /// Returns `true` if the given model supports vision.
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

    fn map_status_to_vision_error(status: StatusCode, message: String) -> VisionError {
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

    async fn execute_vision_request(
        &self,
        request_body: ClaudeVisionApiRequest,
    ) -> Result<ClaudeVisionApiResponse, VisionError> {
        let mut last_error: Option<VisionError> = None;

        for attempt in 0..=DEFAULT_MAX_RETRIES {
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                "x-api-key",
                reqwest::header::HeaderValue::from_str(&self.config.api_key).map_err(|e| {
                    VisionError::AuthenticationError(format!("Invalid API key: {}", e))
                })?,
            );
            headers.insert(
                "anthropic-version",
                reqwest::header::HeaderValue::from_static("2023-06-01"),
            );
            headers.insert(
                "content-type",
                reqwest::header::HeaderValue::from_static("application/json"),
            );

            let response = self
                .client
                .post(format!("{}/messages", self.config.base_url))
                .headers(headers)
                .json(&request_body)
                .send()
                .await
                .map_err(|e| VisionError::NetworkError(format!("Request failed: {}", e)))?;

            let status = response.status();

            if status.is_success() {
                let api_response: ClaudeVisionApiResponse = response.json().await.map_err(|e| {
                    VisionError::ProviderError(format!("Failed to parse response: {}", e))
                })?;
                return Ok(api_response);
            }

            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error response".to_string());

            let error_message =
                if let Ok(err) = serde_json::from_str::<ClaudeErrorResponse>(&error_text) {
                    err.error.message
                } else {
                    error_text
                };

            let error = Self::map_status_to_vision_error(status, error_message);

            if attempt < DEFAULT_MAX_RETRIES && Self::is_transient_error(status) {
                last_error = Some(error);
                let delay_ms = Self::calculate_backoff_delay(
                    attempt,
                    DEFAULT_INITIAL_BACKOFF_MS,
                    DEFAULT_BACKOFF_MULTIPLIER,
                );
                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                continue;
            }

            return Err(error);
        }

        Err(last_error.unwrap_or(VisionError::MaxRetriesExceeded(DEFAULT_MAX_RETRIES)))
    }

    async fn convert_vision_content(
        &self,
        content: &VisionContent,
    ) -> Result<ClaudeContentBlock, LlmError> {
        match content {
            VisionContent::ImageUrl { url, .. } => {
                let (data, media_type) = Self::download_and_encode_image(url).await?;
                Ok(ClaudeContentBlock::Image {
                    source: ClaudeImageSource {
                        source_type: "base64".to_string(),
                        media_type,
                        data,
                    },
                })
            }
            VisionContent::ImageBase64 {
                data, media_type, ..
            } => Ok(ClaudeContentBlock::Image {
                source: ClaudeImageSource {
                    source_type: "base64".to_string(),
                    media_type: media_type.clone(),
                    data: data.clone(),
                },
            }),
            VisionContent::ImageFile { path, .. } => {
                let image_data = fs::read(path).await.map_err(|e| {
                    LlmError::ProcessingError(format!("Failed to read image file: {}", e))
                })?;
                let media_type = Self::detect_mime_type(path)?;
                let base64_data = general_purpose::STANDARD.encode(&image_data);
                Ok(ClaudeContentBlock::Image {
                    source: ClaudeImageSource {
                        source_type: "base64".to_string(),
                        media_type,
                        data: base64_data,
                    },
                })
            }
        }
    }

    async fn download_and_encode_image(url: &str) -> Result<(String, String), LlmError> {
        let response = reqwest::get(url)
            .await
            .map_err(|e| LlmError::NetworkError(format!("Failed to download image: {}", e)))?;

        if !response.status().is_success() {
            return Err(LlmError::NetworkError(format!(
                "Image download failed with status: {}",
                response.status()
            )));
        }

        let media_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("image/jpeg")
            .to_string();

        if !media_type.starts_with("image/") {
            return Err(LlmError::InvalidPrompt(format!(
                "URL does not point to an image. Content-Type: {}",
                media_type
            )));
        }

        let image_bytes = response
            .bytes()
            .await
            .map_err(|e| LlmError::NetworkError(format!("Failed to read image data: {}", e)))?;

        Ok((general_purpose::STANDARD.encode(&image_bytes), media_type))
    }

    /// Detect MIME type from a file extension.
    pub fn detect_mime_type(path: &Path) -> Result<String, LlmError> {
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
            other => Err(LlmError::InvalidPrompt(format!(
                "Unsupported image format: {}",
                other
            ))),
        }
    }

    async fn build_vision_content_blocks(
        &self,
        request: &LlmRequest,
        vision: &VisionRequest,
    ) -> Result<Vec<ClaudeContentBlock>, LlmError> {
        if !Self::is_vision_model(&request.model) {
            return Err(LlmError::ModelNotAvailable(format!(
                "Model {} does not support vision. Supported models: {}",
                request.model,
                VISION_MODELS.join(", ")
            )));
        }

        let mut content_blocks = vec![];

        if !vision.text.is_empty() {
            content_blocks.push(ClaudeContentBlock::Text {
                text: vision.text.clone(),
            });
        }

        for image in &vision.images {
            content_blocks.push(self.convert_vision_content(image).await?);
        }

        Ok(content_blocks)
    }
}

// ── VisionCapableLlm impl ─────────────────────────────────────────────────────

#[async_trait]
impl VisionCapableLlm for AnthropicAdapter {
    async fn generate_with_vision(
        &self,
        request: LlmRequest,
        vision: VisionRequest,
    ) -> Result<LlmResponse, LlmError> {
        let content_blocks = self.build_vision_content_blocks(&request, &vision).await?;

        let request_body = ClaudeVisionApiRequest {
            model: request.model.clone(),
            messages: vec![ClaudeVisionMessage {
                role: "user".to_string(),
                content: content_blocks,
            }],
            max_tokens: DEFAULT_MAX_TOKENS,
        };

        let api_response =
            self.execute_vision_request(request_body)
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
                    VisionError::Timeout(seconds) => {
                        LlmError::Timeout(format!("{} seconds", seconds))
                    }
                    VisionError::MaxRetriesExceeded(attempts) => LlmError::ProcessingError(
                        format!("Max retries exceeded: {} attempts", attempts),
                    ),
                    _ => LlmError::ProcessingError(format!("Vision error: {}", e)),
                })?;

        let content = api_response
            .content
            .iter()
            .map(|block| match block {
                ClaudeResponseContent::Text { text } => text.clone(),
            })
            .next()
            .ok_or_else(|| LlmError::ProcessingError("No text content in response".to_string()))?;

        let finish_reason = api_response
            .stop_reason
            .as_deref()
            .map(|reason| match reason {
                "end_turn" | "stop_sequence" => FinishReason::Stop,
                "max_tokens" => FinishReason::Length,
                other => FinishReason::Error(other.to_string()),
            })
            .unwrap_or(FinishReason::Stop);

        Ok(LlmResponse {
            id: uuid::Uuid::new_v4(),
            request_id: request.id,
            model: api_response.model,
            content,
            finish_reason,
            usage: TokenUsage {
                prompt_tokens: api_response.usage.input_tokens,
                completion_tokens: api_response.usage.output_tokens,
                total_tokens: api_response.usage.input_tokens + api_response.usage.output_tokens,
            },
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
impl VisionPort for AnthropicAdapter {
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

        let mut content_blocks = vec![ClaudeContentBlock::Text {
            text: prompt.to_string(),
        }];

        for image in images {
            let block = self.convert_vision_content(&image).await.map_err(|e| {
                VisionError::InvalidRequest(format!("Failed to convert image: {}", e))
            })?;
            content_blocks.push(block);
        }

        let request_body = ClaudeVisionApiRequest {
            model: model.to_string(),
            messages: vec![ClaudeVisionMessage {
                role: "user".to_string(),
                content: content_blocks,
            }],
            max_tokens: max_tokens.unwrap_or(1000) as usize,
        };

        let response = self.execute_vision_request(request_body).await?;

        let content = response
            .content
            .iter()
            .map(|block| match block {
                ClaudeResponseContent::Text { text } => text.clone(),
            })
            .next()
            .ok_or_else(|| {
                VisionError::InvalidRequest("No text content in response".to_string())
            })?;

        Ok(VisionResult {
            content,
            model: response.model,
            token_usage: VisionTokenUsage {
                prompt_tokens: response.usage.input_tokens,
                completion_tokens: response.usage.output_tokens,
                total_tokens: response.usage.input_tokens + response.usage.output_tokens,
            },
            metadata: std::collections::HashMap::new(),
            timestamp: chrono::Utc::now(),
        })
    }

    fn is_vision_model(&self, model: &str) -> bool {
        Self::is_vision_model(model)
    }

    fn provider_name(&self) -> &str {
        "anthropic"
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::vision::ImageDetail;
    use std::path::Path;

    fn create_test_adapter() -> AnthropicAdapter {
        let config = super::super::adapter::AnthropicConfig::new(
            "sk-ant-test-key".to_string(),
            "https://api.anthropic.com/v1".to_string(),
            "claude-3-opus-20240229".to_string(),
            4096,
        );
        AnthropicAdapter::new(config).unwrap()
    }

    #[test]
    fn test_is_vision_model() {
        assert!(AnthropicAdapter::is_vision_model("claude-3-opus-20240229"));
        assert!(AnthropicAdapter::is_vision_model(
            "claude-3-sonnet-20240229"
        ));
        assert!(AnthropicAdapter::is_vision_model("claude-3-haiku-20240307"));
        assert!(AnthropicAdapter::is_vision_model(
            "claude-3-5-sonnet-20240620"
        ));
        assert!(AnthropicAdapter::is_vision_model(
            "claude-3-5-sonnet-20241022"
        ));
        assert!(AnthropicAdapter::is_vision_model(
            "claude-3-5-haiku-20241022"
        ));
        assert!(!AnthropicAdapter::is_vision_model("claude-2.1"));
        assert!(!AnthropicAdapter::is_vision_model("claude-instant-1.2"));
    }

    #[test]
    fn test_detect_mime_type() {
        assert_eq!(
            AnthropicAdapter::detect_mime_type(Path::new("test.jpg")).unwrap(),
            "image/jpeg"
        );
        assert_eq!(
            AnthropicAdapter::detect_mime_type(Path::new("test.jpeg")).unwrap(),
            "image/jpeg"
        );
        assert_eq!(
            AnthropicAdapter::detect_mime_type(Path::new("test.png")).unwrap(),
            "image/png"
        );
        assert_eq!(
            AnthropicAdapter::detect_mime_type(Path::new("test.gif")).unwrap(),
            "image/gif"
        );
        assert_eq!(
            AnthropicAdapter::detect_mime_type(Path::new("test.webp")).unwrap(),
            "image/webp"
        );
        assert!(AnthropicAdapter::detect_mime_type(Path::new("test.txt")).is_err());
        assert!(AnthropicAdapter::detect_mime_type(Path::new("test.bmp")).is_err());
    }

    #[tokio::test]
    async fn test_convert_vision_content_base64() {
        let adapter = create_test_adapter();
        let content = VisionContent::ImageBase64 {
            data: "abc123".to_string(),
            media_type: "image/png".to_string(),
            detail: ImageDetail::High,
        };
        let result = adapter.convert_vision_content(&content).await.unwrap();
        match result {
            ClaudeContentBlock::Image { source } => {
                assert_eq!(source.source_type, "base64");
                assert_eq!(source.media_type, "image/png");
                assert_eq!(source.data, "abc123");
            }
            _ => panic!("Expected Image content block"),
        }
    }

    #[tokio::test]
    async fn test_build_vision_content_blocks_non_vision_model() {
        let adapter = create_test_adapter();

        use paladin_core::platform::container::prompt::{PromptItem, PromptType, TextPrompt};
        use paladin_ports::output::llm_port::LlmRequest;

        let llm_request = LlmRequest::new(
            "claude-2.1",
            PromptItem::new(PromptType::Text(TextPrompt {
                content: "test".to_string(),
                role: paladin_core::platform::container::prompt::PromptRole::User,
            }))
            .unwrap(),
        );

        let vision = VisionRequest::new(
            "Describe this".to_string(),
            vec![VisionContent::ImageBase64 {
                data: "abc123".to_string(),
                media_type: "image/png".to_string(),
                detail: ImageDetail::Auto,
            }],
        )
        .unwrap();

        let result = adapter
            .build_vision_content_blocks(&llm_request, &vision)
            .await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            LlmError::ModelNotAvailable(_)
        ));
    }

    #[test]
    fn test_supports_vision() {
        let adapter = create_test_adapter();
        assert!(adapter.supports_vision());
    }

    #[test]
    fn test_calculate_backoff_delay() {
        assert_eq!(
            AnthropicAdapter::calculate_backoff_delay(0, 1000, 2.0),
            1000
        );
        assert_eq!(
            AnthropicAdapter::calculate_backoff_delay(1, 1000, 2.0),
            2000
        );
        assert_eq!(
            AnthropicAdapter::calculate_backoff_delay(2, 1000, 2.0),
            4000
        );
    }

    #[test]
    fn test_is_transient_error() {
        assert!(AnthropicAdapter::is_transient_error(
            StatusCode::TOO_MANY_REQUESTS
        ));
        assert!(AnthropicAdapter::is_transient_error(
            StatusCode::INTERNAL_SERVER_ERROR
        ));
        assert!(AnthropicAdapter::is_transient_error(
            StatusCode::SERVICE_UNAVAILABLE
        ));
        assert!(!AnthropicAdapter::is_transient_error(
            StatusCode::BAD_REQUEST
        ));
        assert!(!AnthropicAdapter::is_transient_error(
            StatusCode::UNAUTHORIZED
        ));
        assert!(!AnthropicAdapter::is_transient_error(StatusCode::OK));
    }
}
