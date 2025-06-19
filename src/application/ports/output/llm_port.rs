/*

LLM Port

A port that defines how the application interacts with the LLM (Low Level Model).

This port is responsible for translating high-level application use cases into interactions
with the LLM. It provides an abstraction layer that allows the application to interact with
the LLM without being tightly coupled to its implementation details.

Typical implementations of this port would be for the adapter to translate the requirements
of the high-level use cases into calls to the LLM, and to translate the results of those calls
back into a format that the application can use.

An LLM Api usually requires a few standard fields to be present in the request and response
like the prompt, max_tokens, different weights for "temperature". The LLM Port handles 
these fields and provide a clean interface for the application to interact with the LLM and
for the adapter to translate the application's requirements into calls to the LLM.

*/
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::core::platform::container::prompt::PromptItem;
use crate::core::platform::container::content::ContentItem;

#[derive(Debug, Clone, Error)]
pub enum LlmError {
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Authentication failed: {0}")]
    AuthenticationError(String),
    #[error("Invalid prompt: {0}")]
    InvalidPrompt(String),
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    #[error("Model not available: {0}")]
    ModelNotAvailable(String),
    #[error("Token limit exceeded")]
    TokenLimitExceeded,
    #[error("Processing error: {0}")]
    ProcessingError(String),
    #[error("Timeout: {0}")]
    Timeout(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    pub id: Uuid,
    pub model: String,
    pub prompt: PromptItem,
    pub attachments: Vec<ContentItem>,
    pub stream: bool,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub id: Uuid,
    pub request_id: Uuid,
    pub model: String,
    pub content: String,
    pub finish_reason: FinishReason,
    pub usage: TokenUsage,
    pub created_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FinishReason {
    Stop,
    Length,
    ContentFilter,
    FunctionCall,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingResponse {
    pub id: Uuid,
    pub delta: String,
    pub finish_reason: Option<FinishReason>,
}

/// Main LLM Port trait
#[async_trait]
pub trait LlmPort: Send + Sync {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError>;
    
    async fn generate_stream(&self, request: LlmRequest) -> Result<Box<dyn futures::Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError>;
    
    async fn validate_model(&self, model: &str) -> Result<bool, LlmError>;
    
    async fn get_available_models(&self) -> Result<Vec<String>, LlmError>;
    
    fn get_provider_name(&self) -> &'static str;
}