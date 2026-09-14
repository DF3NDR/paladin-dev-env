//! OpenAI GPT adapter.
//!
//! Implements [`LlmPort`] for the OpenAI Chat Completions API.
//! Supports GPT-3.5-Turbo, GPT-4, GPT-4o, and other compatible models.

use async_trait::async_trait;
use chrono::Utc;
use futures::{Stream, StreamExt};
use paladin_core::platform::container::content::{ContentItem, ContentType};
use paladin_core::platform::container::prompt::{PromptItem, PromptRole, PromptType};
use paladin_ports::output::llm_port::{
    FinishReason, LlmError, LlmPort, LlmRequest, LlmResponse, ProviderCapabilities, ResponseFormat,
    StreamingResponse, TokenUsage,
};
use rand::Rng;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::pin::Pin;
use std::time::Duration;
use uuid::Uuid;

use crate::http_status::map_http_status;

/// The provider name this adapter reports through [`LlmPort::get_provider_name`]
/// and stamps on every [`LlmError::ProviderError`] it emits.
const OPENAI_PROVIDER: &str = "openai";

/// Configuration for the OpenAI adapter.
#[derive(Debug, Clone)]
pub struct OpenAIConfig {
    /// OpenAI API key.
    pub api_key: String,
    /// Base URL for the API (default: `https://api.openai.com/v1`).
    pub base_url: String,
    /// Optional organisation ID.
    pub organization: Option<String>,
    /// Request timeout in seconds (default: 300).
    pub timeout_seconds: u64,
    /// Maximum retry attempts (default: 3).
    pub max_retries: u32,
}

impl OpenAIConfig {
    /// Load configuration from environment variables.
    ///
    /// Required:
    /// - `OPENAI_API_KEY`
    ///
    /// Optional:
    /// - `OPENAI_BASE_URL` (default: `https://api.openai.com/v1`)
    /// - `OPENAI_ORGANIZATION`
    /// - `OPENAI_TIMEOUT_SECONDS` (default: 300)
    /// - `OPENAI_MAX_RETRIES` (default: 3)
    pub fn from_env() -> Result<Self, String> {
        let api_key = env::var("OPENAI_API_KEY")
            .map_err(|_| "OPENAI_API_KEY environment variable not set")?;

        let base_url =
            env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1".to_string());

        let organization = env::var("OPENAI_ORGANIZATION").ok();

        let timeout_seconds = env::var("OPENAI_TIMEOUT_SECONDS")
            .unwrap_or_else(|_| "300".to_string())
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

    /// Create a configuration with the given API key and sensible defaults.
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.openai.com/v1".to_string(),
            organization: None,
            timeout_seconds: 300,
            max_retries: 3,
        }
    }

    /// Validate the configuration fields.
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

// ---------------------------------------------------------------------------
// Internal API structures
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    stream: bool,
    /// `LlmRequest.response_format` on the wire (RT-FR-17, D-28). Omitted
    /// entirely when the caller sets no hint, keeping the body byte
    /// -identical to a pre-0.10 request (X-03).
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<OpenAIResponseFormat>,
}

/// OpenAI's own two native JSON-mode wire shapes (RT-FR-17, D-28).
///
/// `{"type":"json_object"}` for [`ResponseFormat::JsonObject`], or
/// `{"type":"json_schema","json_schema":{"name":..,"schema":..,"strict":..}}`
/// for [`ResponseFormat::JsonSchema`] — OpenAI's documented `json_schema`
/// response-format shape.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum OpenAIResponseFormat {
    JsonObject,
    JsonSchema { json_schema: OpenAIJsonSchemaSpec },
}

#[derive(Debug, Serialize)]
struct OpenAIJsonSchemaSpec {
    name: String,
    schema: serde_json::Value,
    strict: bool,
}

/// Convert the provider-agnostic [`ResponseFormat`] hint into OpenAI's wire
/// shape.
///
/// `ResponseFormat` is `#[non_exhaustive]` (D-28), so a future variant this
/// match has not been taught degrades to the plain JSON-object form rather
/// than silently dropping the field — EDGE(RT-05/wire shape) requires
/// `response_format` is never omitted once the caller asked for JSON.
fn to_openai_response_format(format: &ResponseFormat) -> OpenAIResponseFormat {
    match format {
        ResponseFormat::JsonObject => OpenAIResponseFormat::JsonObject,
        ResponseFormat::JsonSchema {
            name,
            schema,
            strict,
        } => OpenAIResponseFormat::JsonSchema {
            json_schema: OpenAIJsonSchemaSpec {
                name: name.clone(),
                schema: schema.clone(),
                strict: *strict,
            },
        },
        _ => OpenAIResponseFormat::JsonObject,
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    #[allow(dead_code)]
    id: String,
    model: String,
    choices: Vec<OpenAIChoice>,
    usage: OpenAIUsage,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    #[allow(dead_code)]
    index: u32,
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
struct OpenAIStreamChunk {
    #[allow(dead_code)]
    id: String,
    choices: Vec<OpenAIStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamChoice {
    #[allow(dead_code)]
    index: u32,
    delta: OpenAIStreamDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamDelta {
    #[allow(dead_code)]
    role: Option<String>,
    content: Option<String>,
}

// ---------------------------------------------------------------------------
// Adapter
// ---------------------------------------------------------------------------

/// OpenAI LLM adapter implementing [`LlmPort`].
pub struct OpenAIAdapter {
    pub(crate) config: OpenAIConfig,
    pub(crate) client: Client,
}

impl OpenAIAdapter {
    /// Create a new adapter from explicit configuration.
    pub fn new(config: OpenAIConfig) -> Result<Self, String> {
        config.validate()?;
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            // CR-02 (`25-REVIEW.md`): `OPENAI_BASE_URL` is
            // operator-configurable and every request carries the
            // `Authorization: Bearer` credential header. Refusing redirects
            // means a `3xx` from whatever host it resolves to can never
            // replay that header to a different, attacker-influenced host —
            // matches every `CompatEngine`-based preset and the bespoke
            // Gemini adapter (T-17-18/T-17-52). A refused redirect surfaces
            // via `Self::map_error`'s `300..=399` arm.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
        Ok(Self { config, client })
    }

    /// Map a non-2xx OpenAI response to [`LlmError`].
    ///
    /// `300..=399` is named explicitly (mirroring
    /// `CompatEngine::map_error`/`GeminiAdapter::map_error`) because this
    /// client's redirect policy is `none` (see [`Self::new`]), so a `3xx`
    /// response is never followed — it arrives here as an ordinary
    /// non-success status instead. Everything else is the crate-wide
    /// [`map_http_status`] (Phase 25 D-03, FT-FR-01).
    fn map_error(&self, status: u16, body: &str) -> LlmError {
        match status {
            300..=399 => LlmError::ProviderError {
                provider: OPENAI_PROVIDER.to_string(),
                status,
                message: format!(
                    "the configured base URL responded with a redirect (HTTP {status}), which \
                     this client refuses to follow because doing so would forward the \
                     credential header to a different, potentially attacker-influenced host. \
                     Correct the configured base-URL setting to point directly at the intended \
                     endpoint. Response excerpt: {}",
                    crate::redaction::diagnostic_excerpt(body, &self.config.api_key)
                ),
            },
            _ => map_http_status(OPENAI_PROVIDER, status, body, &self.config.api_key),
        }
    }

    /// Create an adapter by loading configuration from environment variables.
    pub fn from_env() -> Result<Self, String> {
        Self::new(OpenAIConfig::from_env()?)
    }

    /// Convert a [`PromptItem`] and optional attachments into OpenAI messages.
    fn convert_to_messages(
        &self,
        prompt: &PromptItem,
        attachments: &[ContentItem],
    ) -> Result<Vec<OpenAIMessage>, LlmError> {
        let mut messages = Vec::new();

        match prompt.prompt_type() {
            PromptType::System(system_prompt) => {
                let mut content = system_prompt.instructions.clone();
                if let Some(constraints) = &system_prompt.constraints
                    && !constraints.is_empty()
                {
                    content.push_str("\n\nConstraints:\n");
                    for constraint in constraints {
                        content.push_str(&format!("- {}\n", constraint));
                    }
                }
                messages.push(OpenAIMessage {
                    role: "system".to_string(),
                    content,
                });
            }
            PromptType::User(user_prompt) => {
                messages.push(OpenAIMessage {
                    role: "user".to_string(),
                    content: user_prompt.context.clone().unwrap_or_default(),
                });
            }
            PromptType::Assistant(assistant_prompt) => {
                let mut content = assistant_prompt.response.clone();
                if let Some(reasoning) = &assistant_prompt.reasoning {
                    content.push_str(&format!("\n\nReasoning: {}", reasoning));
                }
                messages.push(OpenAIMessage {
                    role: "assistant".to_string(),
                    content,
                });
            }
            PromptType::Text(text_prompt) => {
                let role = match text_prompt.role {
                    PromptRole::System => "system",
                    PromptRole::User => "user",
                    PromptRole::Assistant => "assistant",
                    PromptRole::Function => "function",
                };
                messages.push(OpenAIMessage {
                    role: role.to_string(),
                    content: text_prompt.content.clone(),
                });
            }
            PromptType::Function(function_prompt) => {
                messages.push(OpenAIMessage {
                    role: "function".to_string(),
                    content: function_prompt.function_name.clone(),
                });
            }
        }

        for content in attachments {
            if let Ok(content_text) = self.convert_content_to_text(content)
                && !content_text.is_empty()
            {
                messages.push(OpenAIMessage {
                    role: "user".to_string(),
                    content: format!("Content to analyze:\n{}", content_text),
                });
            }
        }

        Ok(messages)
    }

    fn convert_content_to_text(&self, content: &ContentItem) -> Result<String, LlmError> {
        match content.content() {
            ContentType::Text(text_content) => {
                Ok(text_content.content.as_deref().unwrap_or("").to_string())
            }
            ContentType::Video(video_content) => Ok(format!(
                "Video: {} (Duration: {}s)",
                content.title().unwrap_or(&"Untitled".to_string()),
                video_content.duration
            )),
            ContentType::Audio(audio_content) => Ok(format!(
                "Audio: {} (Duration: {}s)",
                content.title().unwrap_or(&"Untitled".to_string()),
                audio_content.duration
            )),
            ContentType::Image(image_content) => Ok(format!(
                "Image: {} ({}x{})",
                content.title().unwrap_or(&"Untitled".to_string()),
                image_content.resolution.0,
                image_content.resolution.1
            )),
        }
    }

    fn convert_finish_reason(&self, reason: Option<String>) -> FinishReason {
        match reason.as_deref() {
            Some("stop") => FinishReason::Stop,
            Some("length") => FinishReason::Length,
            Some("content_filter") => FinishReason::ContentFilter,
            Some("function_call") => FinishReason::FunctionCall,
            Some(other) => FinishReason::Error(format!("Unknown: {}", other)),
            None => FinishReason::Stop,
        }
    }

    async fn make_request_with_retries(
        &self,
        request: &OpenAIRequest,
    ) -> Result<OpenAIResponse, LlmError> {
        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            match self.make_single_request(request).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    last_error = Some(e.clone());

                    if matches!(e, LlmError::AuthenticationError(_)) {
                        return Err(e);
                    }

                    if attempt < self.config.max_retries {
                        let base_delay = Duration::from_secs(1);
                        let exponential_delay = base_delay * 2_u32.pow(attempt);
                        let max_delay = Duration::from_secs(10);
                        let delay = exponential_delay.min(max_delay);

                        let jitter_ms = {
                            let mut rng = rand::thread_rng();
                            rng.gen_range(0..=(delay.as_millis() / 5)) as u64
                        };
                        let total_delay = delay + Duration::from_millis(jitter_ms);

                        tokio::time::sleep(total_delay).await;
                    }
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| LlmError::ProcessingError("Maximum retries exceeded".to_string())))
    }

    async fn make_single_request(
        &self,
        request: &OpenAIRequest,
    ) -> Result<OpenAIResponse, LlmError> {
        let url = format!("{}/chat/completions", self.config.base_url);

        let mut req = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json");

        if let Some(org) = &self.config.organization {
            req = req.header("OpenAI-Organization", org);
        }

        let response = req
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
            // Shared status-to-variant mapping for every adapter (D-03,
            // FT-FR-01), with a `300..=399` pre-check (CR-02) since this
            // client refuses to follow redirects.
            return Err(self.map_error(status.as_u16(), &response_text));
        }

        serde_json::from_str::<OpenAIResponse>(&response_text)
            .map_err(|e| LlmError::ProcessingError(format!("Failed to parse response: {}", e)))
    }

    async fn make_streaming_request(
        &self,
        request: &OpenAIRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamingResponse, LlmError>> + Send>>, LlmError>
    {
        let url = format!("{}/chat/completions", self.config.base_url);

        let mut req = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json");

        if let Some(org) = &self.config.organization {
            req = req.header("OpenAI-Organization", org);
        }

        let response = req
            .json(request)
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            // Same shared mapping as the generate path, so a status yields
            // the same typed variant whether or not the call streams.
            return Err(self.map_error(status.as_u16(), &error_text));
        }

        let stream = response.bytes_stream().map(|chunk_result| {
            chunk_result
                .map_err(|e| LlmError::NetworkError(format!("Stream error: {}", e)))
                .and_then(|chunk| {
                    let chunk_str = String::from_utf8_lossy(&chunk);

                    for line in chunk_str.lines() {
                        if let Some(data) = line.strip_prefix("data: ") {
                            if data == "[DONE]" {
                                return Ok(StreamingResponse {
                                    id: Uuid::new_v4(),
                                    delta: String::new(),
                                    finish_reason: Some(FinishReason::Stop),
                                });
                            }

                            match serde_json::from_str::<OpenAIStreamChunk>(data) {
                                Ok(chunk) => {
                                    if let Some(choice) = chunk.choices.first() {
                                        let delta =
                                            choice.delta.content.clone().unwrap_or_default();
                                        let finish_reason =
                                            choice.finish_reason.as_ref().map(|r| {
                                                match r.as_str() {
                                                    "stop" => FinishReason::Stop,
                                                    "length" => FinishReason::Length,
                                                    "content_filter" => FinishReason::ContentFilter,
                                                    "function_call" => FinishReason::FunctionCall,
                                                    other => FinishReason::Error(format!(
                                                        "Unknown: {}",
                                                        other
                                                    )),
                                                }
                                            });

                                        return Ok(StreamingResponse {
                                            id: Uuid::new_v4(),
                                            delta,
                                            finish_reason,
                                        });
                                    }
                                }
                                Err(e) => {
                                    return Err(LlmError::ProcessingError(format!(
                                        "Failed to parse stream chunk: {}",
                                        e
                                    )));
                                }
                            }
                        }
                    }

                    Ok(StreamingResponse {
                        id: Uuid::new_v4(),
                        delta: String::new(),
                        finish_reason: None,
                    })
                })
        });

        Ok(Box::pin(stream))
    }
}

#[async_trait]
impl LlmPort for OpenAIAdapter {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let messages = self.convert_to_messages(&request.prompt, &request.attachments)?;

        let temperature = request
            .prompt
            .node
            .node
            .parameters
            .temperature
            .unwrap_or(0.7);
        let max_tokens = request
            .prompt
            .node
            .node
            .parameters
            .max_tokens
            .unwrap_or(4096);

        let openai_request = OpenAIRequest {
            model: request.model.clone(),
            messages,
            temperature: Some(temperature),
            max_tokens: Some(max_tokens),
            top_p: Some(1.0),
            stream: false,
            response_format: request
                .response_format
                .as_ref()
                .map(to_openai_response_format),
        };

        let response = self.make_request_with_retries(&openai_request).await?;

        if response.choices.is_empty() {
            return Err(LlmError::ProcessingError(
                "No choices in response".to_string(),
            ));
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
            function_call: None,
        })
    }

    async fn generate_stream(
        &self,
        request: LlmRequest,
    ) -> Result<Box<dyn Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError> {
        let messages = self.convert_to_messages(&request.prompt, &request.attachments)?;

        let temperature = request
            .prompt
            .node
            .node
            .parameters
            .temperature
            .unwrap_or(0.7);
        let max_tokens = request
            .prompt
            .node
            .node
            .parameters
            .max_tokens
            .unwrap_or(4096);

        let openai_request = OpenAIRequest {
            model: request.model.clone(),
            messages,
            temperature: Some(temperature),
            max_tokens: Some(max_tokens),
            top_p: Some(1.0),
            stream: true,
            response_format: request
                .response_format
                .as_ref()
                .map(to_openai_response_format),
        };

        let stream = self.make_streaming_request(&openai_request).await?;
        Ok(Box::new(stream))
    }

    async fn validate_model(&self, model: &str) -> Result<bool, LlmError> {
        let available_models = self.get_available_models().await?;
        Ok(available_models.contains(&model.to_string()))
    }

    async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
        let url = format!("{}/models", self.config.base_url);

        let mut req = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key));

        if let Some(org) = &self.config.organization {
            req = req.header("OpenAI-Organization", org);
        }

        let response = req
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(format!("Failed to fetch models: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(self.map_error(status.as_u16(), &error_text));
        }

        let response_text = response
            .text()
            .await
            .map_err(|e| LlmError::ProcessingError(format!("Failed to read response: {}", e)))?;

        let models_response: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| LlmError::ProcessingError(format!("Failed to parse response: {}", e)))?;

        let models = models_response["data"]
            .as_array()
            .ok_or_else(|| LlmError::ProcessingError("Invalid models response format".to_string()))?
            .iter()
            .filter_map(|model| model["id"].as_str().map(String::from))
            .collect();

        Ok(models)
    }

    fn get_provider_name(&self) -> &'static str {
        OPENAI_PROVIDER
    }

    fn get_capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_streaming: true,
            // `LlmRequest` carries no field through which a tool definition could
            // travel, and this adapter neither sends `tools` nor parses `tool_calls`
            // out of a response. The flag describes what this adapter does, not what
            // the vendor's API offers (WEB-03, D-14). This adapter's `generate()` also
            // hard-codes the absent function call in the response it builds, so the
            // function-calling flag describes what this adapter does rather than what
            // the vendor's API offers (WEB-03, D-12).
            supports_tool_calling: false,
            supports_function_calling: false,
            supports_vision: true,
            max_context_tokens: Some(128000),
            supports_embeddings: true,
            supports_system_messages: true,
            temperature_range: Some((0.0, 1.0)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = OpenAIConfig::new("test-key".to_string());
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.base_url, "https://api.openai.com/v1");
        assert_eq!(config.timeout_seconds, 300);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_config_validation() {
        let valid_config = OpenAIConfig::new("test-key".to_string());
        assert!(valid_config.validate().is_ok());

        let invalid_config = OpenAIConfig {
            api_key: String::new(),
            base_url: "https://api.openai.com/v1".to_string(),
            organization: None,
            timeout_seconds: 300,
            max_retries: 3,
        };
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_adapter_creation() {
        let config = OpenAIConfig::new("test-key".to_string());
        let adapter = OpenAIAdapter::new(config);
        assert!(adapter.is_ok());
    }

    #[test]
    fn test_get_provider_name() {
        let config = OpenAIConfig::new("test-key".to_string());
        let adapter = OpenAIAdapter::new(config).unwrap();
        assert_eq!(adapter.get_provider_name(), "openai");
    }

    #[test]
    fn test_get_capabilities() {
        let config = OpenAIConfig::new("test-key".to_string());
        let adapter = OpenAIAdapter::new(config).unwrap();
        let caps = adapter.get_capabilities();
        assert!(caps.supports_streaming);
        // `LlmRequest` has no field through which a tool definition could travel, and
        // this adapter neither sends `tools` nor parses `tool_calls` (WEB-03, D-14).
        assert!(!caps.supports_tool_calling);
        // This adapter hard-codes the absent function call in the response it
        // builds, so the flag describes what this adapter does rather than what
        // the vendor's API offers (WEB-03, D-12).
        assert!(!caps.supports_function_calling);
        assert!(caps.supports_vision);
        assert_eq!(caps.max_context_tokens, Some(128000));
        assert_eq!(caps.temperature_range, Some((0.0, 1.0)));
    }

    #[test]
    fn test_config_with_organization() {
        let mut config = OpenAIConfig::new("test-key".to_string());
        config.organization = Some("org-123".to_string());
        assert_eq!(config.organization, Some("org-123".to_string()));
    }

    #[test]
    fn test_config_validation_empty_base_url() {
        let config = OpenAIConfig {
            api_key: "test-key".to_string(),
            base_url: String::new(),
            organization: None,
            timeout_seconds: 300,
            max_retries: 3,
        };
        assert!(config.validate().is_err());
    }

    // ── CR-02 (`25-REVIEW.md`): refused-redirect mapping ───────────────────

    #[test]
    fn map_error_maps_a_redirect_status_to_an_actionable_provider_error() {
        let config = OpenAIConfig::new("test-key".to_string());
        let adapter = OpenAIAdapter::new(config).unwrap();

        for expected in [301u16, 302, 307] {
            match adapter.map_error(expected, "moved") {
                LlmError::ProviderError {
                    provider,
                    status,
                    message,
                } => {
                    assert_eq!(provider, "openai");
                    assert_eq!(status, expected, "typed status field must carry the code");
                    assert!(
                        message.contains("redirect"),
                        "status {expected}: message must name the refused redirect, got: {message}"
                    );
                }
                other => panic!("status {expected}: expected ProviderError, got {other:?}"),
            }
        }
    }

    // ── Phase 25 (FT-FR-01, D-03): non-2xx routes through map_http_status ──

    mod status_mapping {
        use super::*;
        use mockito::Server;
        use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};

        /// `max_retries: 0` so a retryable status surfaces after one attempt
        /// instead of sleeping through the adapter's 1s-base backoff.
        fn adapter_at(base_url: &str) -> OpenAIAdapter {
            OpenAIAdapter::new(OpenAIConfig {
                api_key: "test-key".to_string(),
                base_url: base_url.to_string(),
                organization: None,
                timeout_seconds: 5,
                max_retries: 0,
            })
            .expect("test config must build a valid adapter")
        }

        fn build_request(stream: bool) -> LlmRequest {
            LlmRequest::new(
                "gpt-4o",
                PromptItem::new(PromptType::User(UserPrompt {
                    query: "Hello".to_string(),
                    context: None,
                }))
                .expect("a user prompt must build"),
            )
            .with_stream(stream)
        }

        #[tokio::test]
        async fn openai_non_2xx_routes_through_the_shared_mapper() {
            let mut server = Server::new_async().await;
            server
                .mock("POST", "/chat/completions")
                .with_status(503)
                .with_body(r#"{"error":{"message":"overloaded"}}"#)
                .create_async()
                .await;

            let result = adapter_at(&server.url())
                .generate(build_request(false))
                .await;
            match result {
                Err(LlmError::ProviderError {
                    provider, status, ..
                }) => {
                    assert_eq!(provider, "openai");
                    assert_eq!(status, 503);
                }
                other => panic!("expected ProviderError {{ status: 503 }}, got {other:?}"),
            }
        }

        #[tokio::test]
        async fn openai_dedicated_status_mappings_are_unchanged() {
            for (status, body, expect) in [
                (401u16, r#"{"error":"invalid key"}"#, "AuthenticationError"),
                (429, r#"{"error":"slow down"}"#, "RateLimitExceeded"),
                (400, r#"{"error":"bad request"}"#, "InvalidPrompt"),
                (
                    400,
                    r#"{"error":"This model's maximum context length is 8192 tokens"}"#,
                    "TokenLimitExceeded",
                ),
            ] {
                let mut server = Server::new_async().await;
                server
                    .mock("POST", "/chat/completions")
                    .with_status(status.into())
                    .with_body(body)
                    .create_async()
                    .await;

                let err = adapter_at(&server.url())
                    .generate(build_request(false))
                    .await
                    .expect_err("non-2xx must be an error");
                let ok = matches!(
                    (expect, &err),
                    ("AuthenticationError", LlmError::AuthenticationError(_))
                        | ("RateLimitExceeded", LlmError::RateLimitExceeded)
                        | ("InvalidPrompt", LlmError::InvalidPrompt(_))
                        | ("TokenLimitExceeded", LlmError::TokenLimitExceeded)
                );
                assert!(ok, "status {status}: expected {expect}, got {err:?}");
            }
        }

        #[tokio::test]
        async fn openai_client_refuses_to_follow_a_redirect() {
            // CR-02 (`25-REVIEW.md`) end-to-end regression: a `302` from the
            // configured base URL must surface as a refused-redirect
            // `ProviderError`, and the redirect target must never receive a
            // request — proving the `Authorization` header was never
            // replayed rather than merely asserting on the returned error
            // shape.
            let mut server = Server::new_async().await;
            let redirect_target = server
                .mock("POST", "/redirected")
                .expect(0)
                .create_async()
                .await;
            server
                .mock("POST", "/chat/completions")
                .with_status(302)
                .with_header("Location", "/redirected")
                .with_body("moved")
                .create_async()
                .await;

            let result = adapter_at(&server.url())
                .generate(build_request(false))
                .await;

            match result {
                Err(LlmError::ProviderError {
                    provider,
                    status,
                    message,
                }) => {
                    assert_eq!(provider, "openai");
                    assert_eq!(status, 302);
                    assert!(message.contains("redirect"), "got: {message}");
                }
                other => panic!("expected ProviderError {{ status: 302 }}, got {other:?}"),
            }
            redirect_target.assert_async().await;
        }

        #[tokio::test]
        async fn streaming_non_2xx_routes_through_the_shared_mapper() {
            let mut server = Server::new_async().await;
            server
                .mock("POST", "/chat/completions")
                .with_status(503)
                .with_body(r#"{"error":{"message":"overloaded"}}"#)
                .create_async()
                .await;

            let result = adapter_at(&server.url())
                .generate_stream(build_request(true))
                .await;
            // `Ok` carries a boxed `dyn Stream` with no `Debug`, so match by hand.
            match &result {
                Err(LlmError::ProviderError {
                    provider, status, ..
                }) => {
                    assert_eq!(provider, "openai");
                    assert_eq!(*status, 503);
                }
                Ok(_) => panic!("expected Err(ProviderError), got Ok(<stream>)"),
                Err(other) => panic!("expected ProviderError {{ status: 503 }}, got {other:?}"),
            }
        }
    }

    // ── Phase 26 (RT-05, D-28): response_format reaches the wire ──────────

    mod response_format_wiring {
        use super::*;
        use mockito::Server;
        use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
        use serde_json::Value;
        use std::collections::BTreeSet;
        use std::sync::{Arc, Mutex};

        fn adapter_at(base_url: &str) -> OpenAIAdapter {
            OpenAIAdapter::new(OpenAIConfig {
                api_key: "test-key".to_string(),
                base_url: base_url.to_string(),
                organization: None,
                timeout_seconds: 5,
                max_retries: 0,
            })
            .expect("test config must build a valid adapter")
        }

        fn build_request() -> LlmRequest {
            LlmRequest::new(
                "gpt-4o",
                PromptItem::new(PromptType::User(UserPrompt {
                    query: "Hello".to_string(),
                    context: None,
                }))
                .expect("a user prompt must build"),
            )
        }

        /// Runs `generate()` against a mock `/chat/completions` endpoint,
        /// capturing the raw outgoing request body as parsed JSON — mirrors
        /// `CompatEngine`'s `generate_and_capture_body` test helper
        /// (`compat/engine.rs`). Asserting on the parsed JSON rather than a
        /// raw-string substring keeps this immune to key-ordering changes.
        async fn generate_and_capture_body(request: LlmRequest) -> Value {
            let mut server = Server::new_async().await;
            let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
            let captured_clone = Arc::clone(&captured);

            server
                .mock("POST", "/chat/completions")
                .with_status(200)
                .with_body_from_request(move |req| {
                    let body_text = req.utf8_lossy_body().unwrap_or_default().into_owned();
                    *captured_clone.lock().unwrap() = Some(body_text);
                    serde_json::json!({
                        "id": "cmpl-1",
                        "model": "gpt-4o",
                        "choices": [{
                            "index": 0,
                            "message": {"role": "assistant", "content": "ok"},
                            "finish_reason": "stop"
                        }],
                        "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
                    })
                    .to_string()
                    .into_bytes()
                })
                .create_async()
                .await;

            let result = adapter_at(&server.url()).generate(request).await;
            assert!(
                result.is_ok(),
                "mock server returned a well-formed response: {result:?}"
            );

            let body_text = captured
                .lock()
                .unwrap()
                .take()
                .expect("mock must have been called exactly once");
            serde_json::from_str(&body_text).expect("captured body must be valid JSON")
        }

        #[tokio::test]
        async fn openai_request_carries_response_format_json_object() {
            let request = build_request().with_response_format(ResponseFormat::JsonObject);
            let body = generate_and_capture_body(request).await;

            assert_eq!(
                body.get("response_format"),
                Some(&serde_json::json!({"type": "json_object"}))
            );
        }

        #[tokio::test]
        async fn openai_request_carries_response_format_json_schema() {
            let schema = serde_json::json!({
                "type": "object",
                "properties": {"answer": {"type": "string"}}
            });
            let request = build_request().with_response_format(ResponseFormat::JsonSchema {
                name: "answer_schema".to_string(),
                schema: schema.clone(),
                strict: true,
            });

            let body = generate_and_capture_body(request).await;

            assert_eq!(body["response_format"]["type"], "json_schema");
            assert_eq!(
                body["response_format"]["json_schema"]["name"],
                "answer_schema"
            );
            assert_eq!(body["response_format"]["json_schema"]["schema"], schema);
            assert_eq!(body["response_format"]["json_schema"]["strict"], true);
        }

        #[tokio::test]
        async fn openai_request_without_response_format_is_byte_identical_to_today() {
            let body = generate_and_capture_body(build_request()).await;
            let obj = body.as_object().expect("body must be a JSON object");

            assert!(
                !obj.contains_key("response_format"),
                "absent response_format must not appear on the wire, got: {obj:?}"
            );
            // The full key set must be exactly what a pre-0.10 request
            // produced -- proving this is an additive, X-03-compliant change
            // rather than an accidental reshape of the existing fields.
            let keys: BTreeSet<&str> = obj.keys().map(String::as_str).collect();
            assert_eq!(
                keys,
                BTreeSet::from([
                    "model",
                    "messages",
                    "temperature",
                    "max_tokens",
                    "top_p",
                    "stream"
                ])
            );
        }
    }
}
