//! # LLM Port - Large Language Model Integration Interface
//!
//! Port trait defining how the application interacts with Language Model providers.
//!
//! ## Purpose
//!
//! The LLM Port provides a unified abstraction layer for interacting with various Large
//! Language Model providers (OpenAI, DeepSeek, Anthropic, etc.). It decouples the core
//! domain logic from specific LLM API implementations, enabling:
//!
//! - **Provider Independence**: Switch between LLM providers without changing application code
//! - **Testing**: Mock LLM interactions for unit/integration testing
//! - **Multi-Provider**: Use multiple providers simultaneously based on capabilities
//! - **Graceful Degradation**: Feature detection enables fallback strategies
//!
//! The port handles standardized request/response structures, streaming, function calling,
//! error recovery, and provider capability detection.
//!
//! ## Hexagonal Architecture
//!
//! This is an **output port** in the application layer. It defines the interface for LLM
//! operations, allowing the core domain logic (Paladin agents, Battalion orchestration)
//! to remain independent of infrastructure concerns like HTTP clients, API keys, and
//! provider-specific quirks.
//!
//! **Adapter Implementations:**
//! - `OpenAIAdapter` - OpenAI GPT models (GPT-4, GPT-3.5, etc.)
//! - `DeepSeekAdapter` - DeepSeek models with competitive pricing
//! - `AnthropicAdapter` - Claude models with extended context windows
//! - Custom adapters for on-premise or specialized models
//!
//! ## Thread Safety
//!
//! All implementations must be `Send + Sync` to support concurrent async operations.
//! Multiple Paladin agents may call the same LLM adapter concurrently. Implementations
//! should use connection pooling and handle rate limiting internally.
//!
//! ## Error Handling
//!
//! Operations return `Result<T, LlmError>` with detailed error variants:
//! - Network errors are retryable with exponential backoff
//! - Authentication errors indicate configuration issues
//! - Rate limit errors should trigger automatic backoff
//! - Token limit errors require prompt compression or model change
//!
//! See [`LlmError`] for all error categories and handling strategies.
//!
//! ## Examples
//!
//! ### Basic Usage
//!
//! ```rust,no_run
//! use paladin_ports::output::llm_port::{LlmPort, LlmRequest};
//! use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
//! use uuid::Uuid;
//! use std::collections::HashMap;
//!
//! async fn basic_completion(llm: &dyn LlmPort) -> Result<String, Box<dyn std::error::Error>> {
//!     let request = LlmRequest {
//!         id: Uuid::new_v4(),
//!         model: "gpt-4".to_string(),
//!         prompt: PromptItem::new(PromptType::User(UserPrompt {
//!             query: "Explain hexagonal architecture".to_string(),
//!             context: None,
//!         })).unwrap(),
//!         attachments: vec![],
//!         stream: false,
//!         metadata: HashMap::new(),
//!     };
//!
//!     let response = llm.generate(request).await?;
//!     Ok(response.content)
//! }
//! ```
//!
//! ### Streaming Responses
//!
//! ```rust,no_run
//! use paladin_ports::output::llm_port::{LlmPort, LlmRequest};
//! use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
//! use uuid::Uuid;
//! use futures::StreamExt;
//! use std::collections::HashMap;
//!
//! async fn stream_completion(llm: &dyn LlmPort) -> Result<(), Box<dyn std::error::Error>> {
//!     // Check if streaming is supported
//!     let caps = llm.get_capabilities();
//!     if !caps.supports_streaming {
//!         eprintln!("Provider does not support streaming");
//!         return Ok(());
//!     }
//!
//!     let request = LlmRequest {
//!         id: Uuid::new_v4(),
//!         model: "gpt-4".to_string(),
//!         prompt: PromptItem::new(PromptType::User(UserPrompt {
//!             query: "Write a story".to_string(),
//!             context: None,
//!         })).unwrap(),
//!         attachments: vec![],
//!         stream: true,
//!         metadata: HashMap::new(),
//!     };
//!
//!     let mut stream = llm.generate_stream(request).await?;
//!     // Note: Use pin_mut! or tokio::pin! to pin the stream before iterating
//!     // while let Some(result) = stream.next().await { ... }
//!     Ok(())
//! }
//! ```
//!
//! ### Provider Capability Detection
//!
//! ```rust,no_run
//! use paladin_ports::output::llm_port::LlmPort;
//!
//! async fn select_strategy(llm: &dyn LlmPort) {
//!     let caps = llm.get_capabilities();
//!
//!     println!("Provider: {}", llm.get_provider_name());
//!     println!("Streaming: {}", caps.supports_streaming);
//!     println!("Tool Calling: {}", caps.supports_tool_calling);
//!     println!("Vision: {}", caps.supports_vision);
//!
//!     if let Some(max_tokens) = caps.max_context_tokens {
//!         println!("Max context: {} tokens", max_tokens);
//!     }
//! }
//! ```
//!
//! ### Custom Implementation
//!
//! ```rust,no_run
//! use paladin_ports::output::llm_port::{
//!     LlmPort, LlmRequest, LlmResponse, LlmError, ProviderCapabilities,
//!     FinishReason, TokenUsage
//! };
//! use async_trait::async_trait;
//! use uuid::Uuid;
//! use chrono::Utc;
//! use std::collections::HashMap;
//!
//! struct LocalLlmAdapter {
//!     endpoint: String,
//! }
//!
//! #[async_trait]
//! impl LlmPort for LocalLlmAdapter {
//!     async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
//!         // Custom implementation for local LLM server
//!         Ok(LlmResponse {
//!             id: Uuid::new_v4(),
//!             request_id: request.id,
//!             model: request.model,
//!             content: "Response from local LLM".to_string(),
//!             finish_reason: FinishReason::Stop,
//!             usage: TokenUsage {
//!                 prompt_tokens: 10,
//!                 completion_tokens: 20,
//!                 total_tokens: 30,
//!             },
//!             created_at: Utc::now(),
//!             metadata: HashMap::new(),
//!             function_call: None,
//!         })
//!     }
//!
//!     async fn generate_stream(
//!         &self,
//!         _request: LlmRequest,
//!     ) -> Result<Box<dyn futures::Stream<Item = Result<paladin_ports::output::llm_port::StreamingResponse, LlmError>> + Send>, LlmError> {
//!         Err(LlmError::ProcessingError("Streaming not supported".to_string()))
//!     }
//!
//!     async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
//!         Ok(true)
//!     }
//!
//!     async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
//!         Ok(vec!["local-llm-7b".to_string()])
//!     }
//!
//!     fn get_provider_name(&self) -> &'static str {
//!         "local"
//!     }
//!
//!     fn get_capabilities(&self) -> ProviderCapabilities {
//!         ProviderCapabilities {
//!             supports_streaming: false,
//!             supports_tool_calling: false,
//!             supports_function_calling: false,
//!             supports_vision: false,
//!             supports_embeddings: false,
//!             max_context_tokens: Some(4096),
//!             supports_system_messages: true,
//!             temperature_range: None,
//!         }
//!     }
//! }
//! ```
//!
//! ## Implementation Notes
//!
//! ### Performance Considerations
//! - **Connection Pooling**: Reuse HTTP clients/connections across requests
//! - **Batching**: Group multiple requests when provider supports it
//! - **Caching**: Cache model lists and capabilities (TTL: 1 hour recommended)
//! - **Timeouts**: Set reasonable timeouts (30-60s for generation, 120s for streaming)
//! - **Rate Limiting**: Implement token bucket or leaky bucket algorithm
//!
//! ### Best Practices
//! 1. **Error Recovery**: Implement exponential backoff for retryable errors
//! 2. **Logging**: Log all LLM interactions for debugging and audit trails
//! 3. **Cost Tracking**: Track token usage for billing/budgeting
//! 4. **Failover**: Support fallback to alternative models/providers
//! 5. **Testing**: Use mock adapters in tests to avoid API calls
//!
//! ### Common Pitfalls
//! - Don't hold connections open during streaming if client disconnects
//! - Don't retry authentication errors (they won't succeed without config change)
//! - Don't ignore token limits (leads to truncated responses)
//! - Don't block async runtime with synchronous HTTP clients
//!
//! ## Related Ports
//!
//! - [`VisionLlmPort`](crate::output::vision_llm_port::VisionLlmPort) - Extended LLM port with vision/image understanding
//! - [`EmbeddingPort`](crate::output::embedding_port::EmbeddingPort) - Generate vector embeddings from text
//! - [`GarrisonPort`](crate::output::garrison_port::GarrisonPort) - Store conversation history for context
//! - [`ArsenalPort`](crate::output::arsenal_port::ArsenalPort) - Provide tools for function calling
//!
//! ## See Also
//!
//! - [Application Ports](crate::application::ports)
//! - [Paladin Domain](paladin_core::platform::container::paladin)
//! - [Infrastructure Adapters](crate::infrastructure::adapters::llm)

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

use paladin_core::platform::container::content::ContentItem;
use paladin_core::platform::container::prompt::PromptItem;
use paladin_core::platform::container::transience::Transience;

/// Errors that can occur during LLM operations
///
/// Each variant represents a specific failure mode with context for recovery strategies.
/// All errors implement `std::error::Error` via `thiserror`.
///
/// ## Error Categories
///
/// ### Retryable Errors
/// - [`LlmError::NetworkError`] - Retry with exponential backoff
/// - [`LlmError::RateLimitExceeded`] - Retry after delay (check `Retry-After` header)
/// - [`LlmError::Timeout`] - Retry with potentially longer timeout
///
/// ### Configuration Errors (Non-Retryable)
/// - [`LlmError::AuthenticationError`] - Fix API key/credentials
/// - [`LlmError::InvalidPrompt`] - Fix prompt format/content
/// - [`LlmError::ModelNotAvailable`] - Change model or provider
///
/// ### Capacity Errors
/// - [`LlmError::TokenLimitExceeded`] - Reduce prompt size or upgrade model
///
/// ## Examples
///
/// ```rust
/// use paladin_ports::output::llm_port::LlmError;
///
/// fn handle_error(error: LlmError) -> bool {
///     match error {
///         LlmError::NetworkError(_) | LlmError::Timeout(_) => {
///             // Retryable - implement backoff
///             true
///         }
///         LlmError::RateLimitExceeded => {
///             // Retryable after delay
///             true
///         }
///         LlmError::AuthenticationError(_) => {
///             // Non-retryable - needs configuration fix
///             false
///         }
///         _ => false,
///     }
/// }
/// ```
#[derive(Debug, Clone, Error)]
pub enum LlmError {
    /// Network communication failure (DNS, connection, socket errors)
    ///
    /// **Recovery**: Retry with exponential backoff (max 3-5 attempts)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_ports::output::llm_port::LlmError;
    ///
    /// let error = LlmError::NetworkError("Connection refused".to_string());
    /// assert!(error.to_string().contains("Network error"));
    /// ```
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Authentication or authorization failure (invalid API key, expired token)
    ///
    /// **Recovery**: Check configuration, rotate credentials. Do not retry.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_ports::output::llm_port::LlmError;
    ///
    /// let error = LlmError::AuthenticationError("Invalid API key".to_string());
    /// ```
    #[error("Authentication failed: {0}")]
    AuthenticationError(String),

    /// Prompt validation failure (empty, malformed, or containing prohibited content)
    ///
    /// **Recovery**: Fix prompt content/structure. Do not retry.
    #[error("Invalid prompt: {0}")]
    InvalidPrompt(String),

    /// Rate limit exceeded (too many requests in time window)
    ///
    /// **Recovery**: Wait and retry. Check `Retry-After` header if available.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_ports::output::llm_port::LlmError;
    /// use std::thread;
    /// use std::time::Duration;
    ///
    /// fn handle_rate_limit(error: LlmError) {
    ///     if matches!(error, LlmError::RateLimitExceeded) {
    ///         // Wait before retry (implement exponential backoff in production)
    ///         println!("Rate limited, waiting before retry");
    ///     }
    /// }
    /// ```
    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    /// The provider reports the account has reached its configured API usage
    /// limit for the billing period — a hard quota/balance ceiling, not a
    /// transient per-request throttle.
    ///
    /// Anthropic surfaces this as an HTTP 400 `invalid_request_error` whose
    /// body carries a specific usage-cap phrase (which is why it was
    /// previously mis-classified as [`LlmError::InvalidPrompt`] — the 400
    /// status code alone does not distinguish the two). DeepSeek surfaces an
    /// insufficient-balance condition as HTTP 402.
    ///
    /// This is distinct from its neighbours: unlike [`LlmError::RateLimitExceeded`]
    /// it does NOT clear on backoff-and-retry — the quota resets on a
    /// provider-side billing schedule, not a short window. Unlike
    /// [`LlmError::InvalidPrompt`] the prompt itself is fine; re-authoring it
    /// is pointless. Unlike [`LlmError::AuthenticationError`] the credentials
    /// are valid — the account is simply out of budget.
    ///
    /// **Recovery**: Do not retry. The caller must stop issuing calls to this
    /// provider for the remainder of the run and tell the operator to re-run
    /// after the quota resets. `regain_hint` is best-effort prose lifted
    /// verbatim from the provider's response body and must never be parsed
    /// into a value the pipeline schedules against — it is for operator
    /// display only.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_ports::output::llm_port::LlmError;
    ///
    /// let error = LlmError::UsageLimitExceeded {
    ///     provider: "anthropic".to_string(),
    ///     regain_hint: Some("2026-08-01 00:00 UTC".to_string()),
    /// };
    /// let rendered = error.to_string();
    /// assert!(rendered.contains("anthropic"));
    /// assert!(rendered.contains("2026-08-01 00:00 UTC"));
    ///
    /// let error_no_hint = LlmError::UsageLimitExceeded {
    ///     provider: "deepseek".to_string(),
    ///     regain_hint: None,
    /// };
    /// let rendered_no_hint = error_no_hint.to_string();
    /// assert!(rendered_no_hint.contains("deepseek"));
    /// assert!(!rendered_no_hint.contains("None"));
    /// ```
    #[error(
        "Usage limit exceeded for provider '{provider}': the account's configured API usage limit has been reached{}",
        regain_hint.as_deref().map(|h| format!(" — access regains at {h}")).unwrap_or_default()
    )]
    UsageLimitExceeded {
        /// The provider that reported the usage-cap condition (e.g.
        /// `"anthropic"`, `"deepseek"`).
        provider: String,
        /// Best-effort prose extracted from the provider's response body
        /// describing when access regains. Display-only — never parsed into
        /// a scheduled `DateTime`.
        regain_hint: Option<String>,
    },

    /// Requested model not available or not supported by provider
    ///
    /// **Recovery**: Use `get_available_models()` to find alternatives
    #[error("Model not available: {0}")]
    ModelNotAvailable(String),

    /// Prompt exceeds model's maximum context window
    ///
    /// **Recovery**: Reduce prompt size, summarize context, or use a model with larger context
    #[error("Token limit exceeded")]
    TokenLimitExceeded,

    /// Completion was truncated before any content was produced: the provider
    /// reported `finish_reason=length` while `content` was empty or whitespace-only.
    ///
    /// This is the signature of a reasoning model (e.g. DeepSeek's `-flash`/`-pro`
    /// variants) whose hidden `reasoning_content` consumed the entire `max_tokens`
    /// budget, leaving no tokens for the visible answer. It is distinct from
    /// [`LlmError::TokenLimitExceeded`], whose recovery (reduce prompt size) is the
    /// opposite of what this condition needs.
    ///
    /// **Recovery**: Retry with a larger `max_tokens` budget to give the model
    /// headroom for both reasoning and the visible completion. Do not retry with
    /// an identical request — it will reproduce the same truncation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_ports::output::llm_port::LlmError;
    ///
    /// let error = LlmError::EmptyCompletion(
    ///     "finish_reason=length, content empty (reasoning consumed max_tokens budget)".to_string()
    /// );
    /// assert!(error.to_string().contains("max_tokens"));
    /// ```
    #[error(
        "Empty completion: {0} — retry with a larger max_tokens budget (reasoning likely consumed the whole budget)"
    )]
    EmptyCompletion(String),

    /// General processing error during LLM interaction
    ///
    /// **Recovery**: Depends on specific error message. May be retryable.
    #[error("Processing error: {0}")]
    ProcessingError(String),

    /// Request timed out waiting for response
    ///
    /// **Recovery**: Retry with potentially longer timeout
    #[error("Timeout: {0}")]
    Timeout(String),

    /// A non-2xx HTTP response from a provider with no dedicated variant
    /// above (D-03): every adapter's non-2xx match routes through one shared
    /// `map_http_status` helper (plan 25-05), which builds this variant for
    /// any status that is not one of the dedicated 401/429/400/402/404
    /// mappings already covered by [`LlmError::AuthenticationError`],
    /// [`LlmError::RateLimitExceeded`], [`LlmError::InvalidPrompt`],
    /// [`LlmError::UsageLimitExceeded`] and [`LlmError::ModelNotAvailable`].
    ///
    /// `message` crosses a trust boundary (T-25-06): it is populated only by
    /// `map_http_status`, which redacts through `paladin-llm`'s
    /// `redaction.rs` **before** bounding -- never parsed by
    /// [`LlmError::transience`], which classifies `status` only.
    #[error("Provider '{provider}' returned HTTP {status}: {message}")]
    ProviderError {
        /// The provider that returned the non-2xx response (e.g.
        /// `"openai"`, `"deepseek"`).
        provider: String,
        /// The HTTP status code.
        status: u16,
        /// A redacted, human-readable summary of the response body.
        message: String,
    },

    /// Every provider in a fallback chain (Doc 04 FT-FR-16/17) failed; the
    /// chain gives up and surfaces the LAST provider's own error alongside
    /// the full attempt history.
    #[error(
        "All {} provider(s) in the fallback chain failed; last error: {last}",
        attempts.len()
    )]
    AllProvidersFailed {
        /// One `(provider, error message)` pair per hop attempted, in
        /// chronological order.
        attempts: Vec<(String, String)>,
        /// The last provider's own error -- this variant's `transience()` is
        /// exactly this error's own `transience()`.
        last: Box<LlmError>,
    },
}

impl LlmError {
    /// Classify whether this error is worth retrying (Doc 04 FT-FR-01, D-05).
    ///
    /// Every arm reads a typed field or a variant identity only -- never a
    /// rendered `Display` string, a substring or a parsed status code out of
    /// message text. Dedicated variants ([`LlmError::AuthenticationError`],
    /// [`LlmError::RateLimitExceeded`], [`LlmError::InvalidPrompt`],
    /// [`LlmError::UsageLimitExceeded`], [`LlmError::ModelNotAvailable`],
    /// [`LlmError::TokenLimitExceeded`]) are matched on their own arms and
    /// never fall through to [`LlmError::ProviderError`]'s status-range arms.
    pub fn transience(&self) -> Transience {
        match self {
            // Obviously transient: a network blip, a request timeout or a
            // rate limit all describe conditions that clear with time.
            LlmError::NetworkError(_) => Transience::Transient,
            LlmError::Timeout(_) => Transience::Transient,
            LlmError::RateLimitExceeded => Transience::Transient,

            // Obviously permanent: retrying the exact same request
            // reproduces the exact same failure. Each of these is matched on
            // its own dedicated arm -- never falling through to
            // `ProviderError`'s status-range classification below.
            LlmError::AuthenticationError(_) => Transience::Permanent,
            LlmError::InvalidPrompt(_) => Transience::Permanent,
            LlmError::UsageLimitExceeded { .. } => Transience::Permanent,
            LlmError::ModelNotAvailable(_) => Transience::Permanent,
            LlmError::TokenLimitExceeded => Transience::Permanent,
            LlmError::EmptyCompletion(_) => Transience::Permanent,

            // Unresolvable from a bare string: no typed field distinguishes
            // a transient cause from a permanent one.
            LlmError::ProcessingError(_) => Transience::Unknown,

            // Status-carrying provider error, classified by value only
            // (T-25-07): never by inspecting `message`. 408 (request
            // timeout), 429 (rate limited) and every 5xx (server-side fault)
            // are transient; every other 4xx (client-side fault: bad
            // request, unauthorized, payment required, not found, and any
            // other 4xx not enumerated above) is permanent. This arm is only
            // reached for a status with no dedicated variant above (D-03) --
            // a dedicated variant's own arm always wins.
            LlmError::ProviderError { status, .. } => match status {
                408 | 429 => Transience::Transient,
                500..=599 => Transience::Transient,
                _ => Transience::Permanent,
            },

            // A fallback chain's own transience is exactly its last
            // attempt's transience (D-05, FT-FR-16).
            LlmError::AllProvidersFailed { last, .. } => last.transience(),
        }
    }
}

/// Request structure for LLM generation operations
///
/// Contains all parameters needed to generate a completion from a language model.
/// The structure is provider-agnostic and gets translated to provider-specific
/// formats by adapters.
///
/// # Fields
///
/// - `id`: Unique identifier for this request (for tracking and correlation)
/// - `model`: Model identifier (e.g., "gpt-4", "claude-3-opus", "deepseek-chat")
/// - `prompt`: The prompt to send to the model (may include system/user messages)
/// - `attachments`: Additional content items (images, documents) for context
/// - `stream`: Whether to stream the response incrementally
/// - `metadata`: Custom key-value pairs for tracking, logging, or provider-specific options
///
/// # Examples
///
/// ## Basic Text Request
///
/// ```rust
/// use paladin_ports::output::llm_port::LlmRequest;
/// use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
/// use uuid::Uuid;
/// use std::collections::HashMap;
///
/// let request = LlmRequest {
///     id: Uuid::new_v4(),
///     model: "gpt-4".to_string(),
///     prompt: PromptItem::new(PromptType::User(UserPrompt {
///         query: "Explain Rust ownership".to_string(),
///         context: None,
///     })).unwrap(),
///     attachments: vec![],
///     stream: false,
///     metadata: HashMap::new(),
/// };
/// ```
///
/// ## Request with Metadata
///
/// ```rust
/// use paladin_ports::output::llm_port::LlmRequest;
/// use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
/// use uuid::Uuid;
/// use std::collections::HashMap;
///
/// let mut metadata = HashMap::new();
/// metadata.insert("user_id".to_string(), "user123".to_string());
/// metadata.insert("session_id".to_string(), "sess456".to_string());
/// metadata.insert("temperature".to_string(), "0.7".to_string());
///
/// let request = LlmRequest {
///     id: Uuid::new_v4(),
///     model: "gpt-4".to_string(),
///     prompt: PromptItem::new(PromptType::User(UserPrompt {
///         query: "Analyze this data".to_string(),
///         context: None,
///     })).unwrap(),
///     attachments: vec![],
///     stream: false,
///     metadata,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    /// Unique identifier for request tracking and correlation
    pub id: Uuid,
    /// Model identifier (provider-specific: "gpt-4", "claude-3-opus", etc.)
    pub model: String,
    /// Prompt containing messages to send to the model
    pub prompt: PromptItem,
    /// Additional content (images, documents) for multimodal models
    pub attachments: Vec<ContentItem>,
    /// Enable streaming for incremental response delivery
    pub stream: bool,
    /// Custom metadata for tracking, logging, or provider-specific options
    /// Common keys: "temperature", "max_tokens", "top_p", "user_id"
    pub metadata: HashMap<String, String>,
}

/// Response structure for LLM generation operations
///
/// Contains the generated completion along with metadata about the generation
/// process (tokens used, finish reason, timing).
///
/// # Fields
///
/// - `id`: Unique identifier for this response
/// - `request_id`: ID of the originating request (for correlation)
/// - `model`: Actual model used (may differ from requested if fallback occurred)
/// - `content`: Generated text content
/// - `finish_reason`: Why the generation stopped
/// - `usage`: Token usage statistics for billing/monitoring
/// - `created_at`: When this response was created
/// - `metadata`: Additional provider-specific information
/// - `function_call`: Tool/function call request from the model (if applicable)
///
/// # Examples
///
/// ## Checking Finish Reason
///
/// ```rust
/// use paladin_ports::output::llm_port::{LlmResponse, FinishReason};
///
/// # fn example(response: LlmResponse) {
/// match response.finish_reason {
///     FinishReason::Stop => {
///         // Normal completion - model finished naturally
///         println!("Complete response: {}", response.content);
///     }
///     FinishReason::Length => {
///         // Hit token limit - response may be truncated
///         println!("Warning: Response truncated due to length");
///     }
///     FinishReason::FunctionCall => {
///         // Model wants to call a function
///         if let Some(call) = response.function_call {
///             println!("Function call: {}", call.name);
///         }
///     }
///     _ => {}
/// }
/// # }
/// ```
///
/// ## Tracking Token Usage
///
/// ```rust
/// use paladin_ports::output::llm_port::LlmResponse;
///
/// # fn track_usage(response: LlmResponse) {
/// println!("Tokens used:");
/// println!("  Prompt: {}", response.usage.prompt_tokens);
/// println!("  Completion: {}", response.usage.completion_tokens);
/// println!("  Total: {}", response.usage.total_tokens);
///
/// // Calculate approximate cost (example: $0.03 per 1K tokens)
/// let cost = (response.usage.total_tokens as f64 / 1000.0) * 0.03;
/// println!("Estimated cost: ${:.4}", cost);
/// # }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    /// Unique identifier for this response
    pub id: Uuid,
    /// ID of the originating request (for correlation)
    pub request_id: Uuid,
    /// Actual model used for generation
    pub model: String,
    /// Generated text content
    pub content: String,
    /// Why the generation stopped (completed, length limit, etc.)
    pub finish_reason: FinishReason,
    /// Token usage statistics
    pub usage: TokenUsage,
    /// When this response was created (UTC)
    pub created_at: DateTime<Utc>,
    /// Additional provider-specific information
    pub metadata: HashMap<String, String>,
    /// Function call details if the model requested a tool invocation.
    ///
    /// As of this writing, no shipped adapter — OpenAI, Anthropic, DeepSeek, or the bundled
    /// mock in `paladin_llm::mock` — ever populates this field; `generate()` always yields
    /// `None` here. The reasoning loop's tool-invocation branch in
    /// `paladin_execution_service.rs` (both the Arsenal invocation and the
    /// `handoff_to_specialist` path) is therefore reachable only through a consumer-supplied
    /// [`LlmPort`] implementation that parses tool calls itself. See ADR-0042 for the tracked
    /// status of LLM-native tool calling.
    ///
    /// This does not limit Arsenal/MCP tool execution itself, which ships and works through a
    /// different seam — what is unreachable through a shipped provider is the LLM-initiated
    /// entry into it.
    pub function_call: Option<FunctionCall>,
}

/// Function call request from the LLM
///
/// When the LLM wants to invoke a tool, it returns this structure
/// containing the function name and arguments as JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// Name of the function/tool to call
    pub name: String,
    /// Arguments as a JSON-formatted string
    pub arguments: String,
}

/// Why an LLM generation stopped
///
/// Indicates the reason the model stopped generating tokens. Understanding the
/// finish reason is important for determining if the response is complete and
/// whether any follow-up actions are needed.
///
/// # Variants
///
/// - `Stop`: Natural completion (model reached logical end)
/// - `Length`: Hit token limit (response may be truncated)
/// - `ContentFilter`: Content policy violation detected
/// - `FunctionCall`: Model wants to invoke a tool/function
/// - `Error`: Generation failed with an error
///
/// # Examples
///
/// ```rust
/// use paladin_ports::output::llm_port::FinishReason;
///
/// fn handle_finish_reason(reason: FinishReason) -> bool {
///     match reason {
///         FinishReason::Stop => {
///             println!("Generation completed normally");
///             true
///         }
///         FinishReason::Length => {
///             println!("Warning: Response truncated, consider requesting more tokens");
///             false
///         }
///         FinishReason::FunctionCall => {
///             println!("Model requesting function call");
///             true
///         }
///         FinishReason::ContentFilter => {
///             println!("Content filtered by safety policies");
///             false
///         }
///         FinishReason::Error(msg) => {
///             eprintln!("Generation error: {}", msg);
///             false
///         }
///     }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FinishReason {
    /// Generation completed naturally (model reached logical end)
    Stop,
    /// Hit maximum token limit (response may be incomplete)
    Length,
    /// Content filtered by safety/policy systems
    ContentFilter,
    /// Model requesting a function/tool call
    FunctionCall,
    /// Generation failed with an error
    Error(String),
}

/// Token usage statistics for a generation operation
///
/// Tracks the number of tokens consumed during LLM interaction for
/// billing, monitoring, and optimization purposes.
///
/// # Fields
///
/// - `prompt_tokens`: Tokens in the input prompt
/// - `completion_tokens`: Tokens in the generated response
/// - `total_tokens`: Sum of prompt and completion tokens
///
/// # Cost Calculation
///
/// Most LLM providers charge based on token usage. Different models
/// have different per-token costs, often with separate pricing for
/// prompt and completion tokens.
///
/// # Examples
///
/// ```rust
/// use paladin_ports::output::llm_port::TokenUsage;
///
/// let usage = TokenUsage {
///     prompt_tokens: 150,
///     completion_tokens: 300,
///     total_tokens: 450,
/// };
///
/// // Calculate cost for GPT-4 (example: $0.03/1K prompt, $0.06/1K completion)
/// let prompt_cost = (usage.prompt_tokens as f64 / 1000.0) * 0.03;
/// let completion_cost = (usage.completion_tokens as f64 / 1000.0) * 0.06;
/// let total_cost = prompt_cost + completion_cost;
///
/// println!("Total cost: ${:.4}", total_cost);
/// assert_eq!(usage.total_tokens, usage.prompt_tokens + usage.completion_tokens);
/// ```
// Re-export pure domain type from core
pub use paladin_core::platform::container::token_usage::TokenUsage;

/// Incremental response chunk for streaming operations
///
/// When using streaming mode, the LLM response is delivered incrementally
/// as a series of chunks. Each chunk contains a text delta and optionally
/// a finish reason when the stream ends.
///
/// # Fields
///
/// - `id`: Unique identifier for this chunk
/// - `delta`: Incremental text to append to previous chunks
/// - `finish_reason`: Why generation stopped (only in final chunk)
///
/// # Examples
///
/// ```rust
/// use paladin_ports::output::llm_port::StreamingResponse;
/// use uuid::Uuid;
///
/// // Accumulate streaming chunks
/// let mut accumulated = String::new();
/// let chunks = vec![
///     StreamingResponse {
///         id: Uuid::new_v4(),
///         delta: "Hello".to_string(),
///         finish_reason: None,
///     },
///     StreamingResponse {
///         id: Uuid::new_v4(),
///         delta: " world".to_string(),
///         finish_reason: None,
///     },
///     StreamingResponse {
///         id: Uuid::new_v4(),
///         delta: "!".to_string(),
///         finish_reason: Some(paladin_ports::output::llm_port::FinishReason::Stop),
///     },
/// ];
///
/// for chunk in chunks {
///     accumulated.push_str(&chunk.delta);
///     if chunk.finish_reason.is_some() {
///         break;
///     }
/// }
///
/// assert_eq!(accumulated, "Hello world!");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingResponse {
    /// Unique identifier for this chunk
    pub id: Uuid,
    /// Incremental text to append to the response
    pub delta: String,
    /// Why generation stopped (present only in final chunk)
    pub finish_reason: Option<FinishReason>,
}

/// Provider capabilities for feature detection
///
/// This structure allows clients to query what features a specific
/// LLM provider supports, enabling graceful degradation when features
/// are not available.
///
/// ## Tool-call reachability
///
/// A declared `supports_tool_calling` or `supports_function_calling` describes what *this
/// adapter's* `generate()` does, not what the vendor's underlying API offers. As of this
/// writing, no shipped adapter — OpenAI, Anthropic, DeepSeek, or the bundled mock in
/// `paladin_llm::mock` — declares either flag `true`, because none of them ever populates
/// [`LlmResponse::function_call`]. The reasoning loop's tool-invocation branch is therefore
/// reachable only through a consumer-supplied [`LlmPort`] implementation that parses tool
/// calls itself; see ADR-0042 for the tracked status of LLM-native tool calling. Arsenal/MCP
/// tool execution itself ships and works through a different seam — this limitation is about
/// the LLM-initiated entry into it, not about Arsenal's availability.
///
/// # Example
///
/// ```rust
/// # use paladin_ports::output::llm_port::ProviderCapabilities;
/// let capabilities = ProviderCapabilities {
///     supports_streaming: true,
///     supports_tool_calling: false,
///     supports_function_calling: false,
///     supports_vision: false,
///     supports_embeddings: false,
///     max_context_tokens: Some(128000),
///     supports_system_messages: true,
///     temperature_range: Some((0.0, 2.0)),
/// };
///
/// if capabilities.supports_streaming {
///     println!("Provider supports streaming responses");
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderCapabilities {
    /// Whether the provider supports streaming responses
    pub supports_streaming: bool,
    /// Whether the provider supports tool calling (external function execution).
    ///
    /// See the reachability note on this struct's documentation — no shipped adapter
    /// declares this `true` today.
    pub supports_tool_calling: bool,
    /// Whether the provider supports function calling (structured output).
    ///
    /// See the reachability note on this struct's documentation — no shipped adapter
    /// declares this `true` today.
    pub supports_function_calling: bool,
    /// Whether the provider supports vision/image inputs
    pub supports_vision: bool,
    /// Whether the provider supports embeddings generation
    pub supports_embeddings: bool,
    /// Maximum context window size in tokens (None if unlimited or unknown)
    pub max_context_tokens: Option<u32>,
    /// Whether the provider supports system messages
    pub supports_system_messages: bool,
    /// The provider's supported temperature range, inclusive at both endpoints.
    ///
    /// `Some((min, max))` means a requested temperature `t` is valid exactly when
    /// `t >= min && t <= max` — no epsilon tolerance, no rounding. `None` means the
    /// provider has not declared a range; validation falls back to the framework's
    /// default `[0.0, 1.0]` inclusive range (ADR-0004).
    pub temperature_range: Option<(f32, f32)>,
}

impl Default for ProviderCapabilities {
    fn default() -> Self {
        Self {
            supports_streaming: false,
            supports_tool_calling: false,
            supports_function_calling: false,
            supports_vision: false,
            supports_embeddings: false,
            max_context_tokens: None,
            supports_system_messages: true,
            temperature_range: None,
        }
    }
}

/// Main LLM Port trait for language model interactions
///
/// This trait defines the complete interface for interacting with Large Language Model
/// providers. All LLM adapters must implement this trait to be used within the Paladin
/// framework.
///
/// # Async Model
///
/// All methods are `async` to support non-blocking I/O. Implementations should use
/// `tokio` or a compatible async runtime. Methods must not block the runtime.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` to support concurrent operations. Multiple
/// Paladin agents may call methods simultaneously. Implementations should use:
/// - Connection pooling for efficiency
/// - Internal synchronization for shared state
/// - Thread-safe HTTP clients (e.g., `reqwest::Client`)
///
/// # Lifecycle
///
/// - **Initialization**: Create adapter with configuration (API keys, base URLs)
/// - **Usage**: Call methods as needed (adapters should manage connections internally)
/// - **Cleanup**: Automatic via Drop trait (no explicit cleanup needed)
///
/// # Provider Implementation Guidelines
///
/// When implementing this trait for a new provider:
///
/// 1. **Error Handling**: Map provider errors to `LlmError` variants appropriately
/// 2. **Rate Limiting**: Implement exponential backoff internally
/// 3. **Retries**: Retry transient failures automatically (configurable max attempts)
/// 4. **Timeouts**: Set configurable timeouts for all operations
/// 5. **Streaming**: Implement efficient streaming with backpressure
/// 6. **Testing**: Provide mock adapter for testing
///
/// # Examples
///
/// See [module-level documentation](self) for comprehensive examples.
///
/// ## Using an Adapter
///
/// ```rust,no_run
/// # use paladin_ports::output::llm_port::{LlmPort, LlmRequest, LlmError};
/// # use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
/// # use uuid::Uuid;
/// # use std::collections::HashMap;
/// # async fn example(llm: &dyn LlmPort) -> Result<(), LlmError> {
/// // Check provider capabilities
/// let caps = llm.get_capabilities();
/// println!("Using provider: {}", llm.get_provider_name());
///
/// // Validate model availability
/// if !llm.validate_model("gpt-4").await? {
///     println!("Model not available, checking alternatives...");
///     let models = llm.get_available_models().await?;
///     println!("Available: {:?}", models);
/// }
///
/// // Generate completion
/// let request = LlmRequest {
///     id: Uuid::new_v4(),
///     model: "gpt-4".to_string(),
///     prompt: PromptItem::new(PromptType::User(UserPrompt {
///         query: "Hello, world!".to_string(),
///         context: None,
///     })).unwrap(),
///     attachments: vec![],
///     stream: false,
///     metadata: HashMap::new(),
/// };
///
/// let response = llm.generate(request).await?;
/// println!("Response: {}", response.content);
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait LlmPort: Send + Sync {
    /// Generate a completion from the LLM
    ///
    /// Sends a prompt to the language model and returns the complete generated response.
    /// This method blocks until the entire response is generated (for streaming, use
    /// [`generate_stream`](Self::generate_stream)).
    ///
    /// # Parameters
    ///
    /// - `request`: The LLM request containing:
    ///   - `model`: Model identifier (e.g., "gpt-4", "claude-3-opus")
    ///   - `prompt`: Messages to send to the model
    ///   - `attachments`: Additional content for multimodal models
    ///   - `metadata`: Provider-specific options (temperature, max_tokens, etc.)
    ///
    /// # Returns
    ///
    /// Returns `Result<LlmResponse, LlmError>` where:
    /// - `Ok(response)` contains the generated completion with:
    ///   - `content`: Generated text
    ///   - `usage`: Token counts for billing
    ///   - `finish_reason`: Why generation stopped
    ///   - `function_call`: Tool call request (if applicable)
    /// - `Err(error)` indicates failure (see Error section)
    ///
    /// # Errors
    ///
    /// - [`LlmError::NetworkError`] - Connection failure (retryable)
    /// - [`LlmError::AuthenticationError`] - Invalid credentials (fix configuration)
    /// - [`LlmError::InvalidPrompt`] - Malformed prompt (fix request)
    /// - [`LlmError::RateLimitExceeded`] - Too many requests (retry with backoff)
    /// - [`LlmError::ModelNotAvailable`] - Model not supported (use different model)
    /// - [`LlmError::TokenLimitExceeded`] - Prompt too long (reduce size)
    /// - [`LlmError::ProcessingError`] - Provider-side error (may be retryable)
    /// - [`LlmError::Timeout`] - Request timeout (retry or increase timeout)
    ///
    /// # Thread Safety
    ///
    /// This method is safe to call concurrently from multiple tasks. Implementations
    /// must handle concurrent requests efficiently using connection pooling.
    ///
    /// # Examples
    ///
    /// ## Basic Generation
    ///
    /// ```rust,no_run
    /// use paladin_ports::output::llm_port::{LlmPort, LlmRequest};
    /// use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
    /// use uuid::Uuid;
    /// use std::collections::HashMap;
    ///
    /// async fn generate_text(llm: &dyn LlmPort) -> Result<String, Box<dyn std::error::Error>> {
    ///     let request = LlmRequest {
    ///         id: Uuid::new_v4(),
    ///         model: "gpt-4".to_string(),
    ///         prompt: PromptItem::new(PromptType::User(UserPrompt {
    ///             query: "Write a haiku about Rust".to_string(),
    ///             context: None,
    ///         })).unwrap(),
    ///         attachments: vec![],
    ///         stream: false,
    ///         metadata: HashMap::new(),
    ///     };
    ///
    ///     let response = llm.generate(request).await?;
    ///     Ok(response.content)
    /// }
    /// ```
    ///
    /// ## With Error Handling
    ///
    /// ```rust,no_run
    /// use paladin_ports::output::llm_port::{LlmPort, LlmRequest, LlmError};
    /// use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
    /// use uuid::Uuid;
    /// use std::collections::HashMap;
    ///
    /// async fn generate_with_retry(llm: &dyn LlmPort) -> Result<String, LlmError> {
    ///     let request = LlmRequest {
    ///         id: Uuid::new_v4(),
    ///         model: "gpt-4".to_string(),
    ///         prompt: PromptItem::new(PromptType::User(UserPrompt {
    ///             query: "Explain async Rust".to_string(),
    ///             context: None,
    ///         })).unwrap(),
    ///         attachments: vec![],
    ///         stream: false,
    ///         metadata: HashMap::new(),
    ///     };
    ///
    ///     match llm.generate(request).await {
    ///         Ok(response) => Ok(response.content),
    ///         Err(LlmError::RateLimitExceeded) => {
    ///             // Implement retry with backoff
    ///             tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    ///             // Retry logic here
    ///             Err(LlmError::RateLimitExceeded)
    ///         }
    ///         Err(e) => Err(e),
    ///     }
    /// }
    /// ```
    ///
    /// # Implementation Notes
    ///
    /// Implementations should:
    /// - Set appropriate request timeouts (default: 30-60 seconds)
    /// - Implement automatic retry for transient failures
    /// - Log requests for debugging and audit trails
    /// - Track token usage for billing/monitoring
    /// - Handle provider-specific error codes correctly
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError>;

    /// Generate a streaming completion from the LLM
    ///
    /// Sends a prompt to the language model and returns a stream of incremental responses.
    /// This is more efficient for long responses as it allows processing to begin before
    /// the entire response is complete.
    ///
    /// # Parameters
    ///
    /// - `request`: The LLM request (same as [`generate`](Self::generate))
    ///   - Set `stream: true` for clarity (though some adapters ignore this field)
    ///
    /// # Returns
    ///
    /// Returns `Result<Stream, LlmError>` where:
    /// - `Ok(stream)` is a stream of [`StreamingResponse`] chunks:
    ///   - Each chunk contains a `delta` (text to append)
    ///   - Final chunk contains `finish_reason`
    /// - `Err(error)` indicates failure (see Error section)
    ///
    /// # Errors
    ///
    /// Same errors as [`generate`](Self::generate), plus:
    /// - [`LlmError::ProcessingError`] if provider doesn't support streaming
    ///
    /// # Thread Safety
    ///
    /// The returned stream is `Send` and can be moved across tasks. However, streams
    /// should not be shared between tasks without synchronization.
    ///
    /// # Examples
    ///
    /// ## Basic Streaming
    ///
    /// ```rust,no_run
    /// use paladin_ports::output::llm_port::{LlmPort, LlmRequest};
    /// use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
    /// use uuid::Uuid;
    /// use futures::StreamExt;
    /// use std::collections::HashMap;
    ///
    /// async fn stream_response(llm: &dyn LlmPort) -> Result<String, Box<dyn std::error::Error>> {
    ///     let request = LlmRequest {
    ///         id: Uuid::new_v4(),
    ///         model: "gpt-4".to_string(),
    ///         prompt: PromptItem::new(PromptType::User(UserPrompt {
    ///             query: "Write a story".to_string(),
    ///             context: None,
    ///         })).unwrap(),
    ///         attachments: vec![],
    ///         stream: true,
    ///         metadata: HashMap::new(),
    ///     };
    ///
    ///     let mut stream = llm.generate_stream(request).await?;
    ///     let mut complete_response = String::new();
    ///
    ///     // Note: Use tokio::pin! or futures::pin_mut! to pin the stream
    ///     // tokio::pin!(stream);
    ///     // while let Some(result) = stream.next().await {
    ///     //     match result {
    ///     //         Ok(chunk) => {
    ///     //             print!("{}", chunk.delta);
    ///     //             complete_response.push_str(&chunk.delta);
    ///     //         }
    ///     //         Err(e) => eprintln!("Stream error: {}", e),
    ///     //     }
    ///     // }
    ///
    ///     Ok(complete_response)
    /// }
    /// ```
    ///
    /// ## Capability Check First
    ///
    /// ```rust,no_run
    /// use paladin_ports::output::llm_port::LlmPort;
    ///
    /// async fn use_streaming_if_supported(llm: &dyn LlmPort) {
    ///     let caps = llm.get_capabilities();
    ///
    ///     if caps.supports_streaming {
    ///         println!("Using streaming mode");
    ///         // Use generate_stream()
    ///     } else {
    ///         println!("Fallback to non-streaming");
    ///         // Use generate()
    ///     }
    /// }
    /// ```
    ///
    /// # Implementation Notes
    ///
    /// Implementations should:
    /// - Use Server-Sent Events (SSE) or similar streaming protocol
    /// - Handle backpressure to avoid memory issues
    /// - Close stream gracefully on client drop
    /// - Emit errors in stream rather than panicking
    /// - Set reasonable chunk sizes (not too small, not too large)
    async fn generate_stream(
        &self,
        request: LlmRequest,
    ) -> Result<Box<dyn futures::Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError>;

    /// Validate if a model is available from this provider
    ///
    /// Checks whether the specified model identifier is supported by this provider.
    /// Use this before calling [`generate`](Self::generate) to avoid errors.
    ///
    /// # Parameters
    ///
    /// - `model`: Model identifier to validate (e.g., "gpt-4", "claude-3-opus")
    ///
    /// # Returns
    ///
    /// Returns `Result<bool, LlmError>` where:
    /// - `Ok(true)` - Model is available and can be used
    /// - `Ok(false)` - Model is not available (wrong provider or deprecated)
    /// - `Err(error)` - Validation failed (network error, etc.)
    ///
    /// # Errors
    ///
    /// - [`LlmError::NetworkError`] - Could not reach provider to validate
    /// - [`LlmError::AuthenticationError`] - Invalid credentials
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use paladin_ports::output::llm_port::LlmPort;
    ///
    /// async fn check_model(llm: &dyn LlmPort, model: &str) -> Result<(), Box<dyn std::error::Error>> {
    ///     if llm.validate_model(model).await? {
    ///         println!("{} is available", model);
    ///     } else {
    ///         println!("{} not available, checking alternatives...", model);
    ///         let models = llm.get_available_models().await?;
    ///         println!("Available models: {:?}", models);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Implementation Notes
    ///
    /// Implementations may:
    /// - Cache validation results (TTL: 1 hour recommended)
    /// - Use static lists for providers with fixed model sets
    /// - Query provider APIs for dynamic model lists
    async fn validate_model(&self, model: &str) -> Result<bool, LlmError>;

    /// Get a list of available models from this provider
    ///
    /// Retrieves all models currently available from the provider. Use this to
    /// discover models, build UI selection lists, or implement fallback logic.
    ///
    /// # Returns
    ///
    /// Returns `Result<Vec<String>, LlmError>` where:
    /// - `Ok(models)` - List of model identifiers (e.g., ["gpt-4", "gpt-3.5-turbo"])
    /// - `Err(error)` - Could not retrieve model list
    ///
    /// # Errors
    ///
    /// - [`LlmError::NetworkError`] - Could not reach provider
    /// - [`LlmError::AuthenticationError`] - Invalid credentials
    /// - [`LlmError::ProcessingError`] - Provider API error
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use paladin_ports::output::llm_port::LlmPort;
    ///
    /// async fn list_models(llm: &dyn LlmPort) -> Result<(), Box<dyn std::error::Error>> {
    ///     let models = llm.get_available_models().await?;
    ///
    ///     println!("Available models from {}:", llm.get_provider_name());
    ///     for model in models {
    ///         println!("  - {}", model);
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Implementation Notes
    ///
    /// Implementations should:
    /// - Cache results (TTL: 1 hour) to reduce API calls
    /// - Return consistent identifiers (e.g., always lowercase)
    /// - Filter deprecated or beta models unless requested
    /// - Handle pagination if provider has many models
    async fn get_available_models(&self) -> Result<Vec<String>, LlmError>;

    /// Get the provider name
    ///
    /// Returns a static string identifying this provider. Used for logging,
    /// monitoring, and provider-specific logic.
    ///
    /// # Returns
    ///
    /// A static string identifier (e.g., "openai", "deepseek", "anthropic", "local")
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use paladin_ports::output::llm_port::LlmPort;
    ///
    /// fn log_provider(llm: &dyn LlmPort) {
    ///     println!("Using LLM provider: {}", llm.get_provider_name());
    /// }
    /// ```
    ///
    /// # Implementation Notes
    ///
    /// - Use lowercase, hyphenated names (e.g., "azure-openai")
    /// - Keep names short and recognizable
    /// - Use consistent names across the codebase
    fn get_provider_name(&self) -> &'static str;

    /// Get the capabilities of this provider
    ///
    /// Returns a structure describing what features this provider supports.
    /// Use this for feature detection and graceful degradation.
    ///
    /// # Returns
    ///
    /// A [`ProviderCapabilities`] structure with flags for:
    /// - Streaming support
    /// - Tool/function calling
    /// - Vision/multimodal
    /// - Embeddings
    /// - Maximum context window
    /// - System message support
    ///
    /// # Examples
    ///
    /// ## Feature Detection
    ///
    /// ```rust,no_run
    /// use paladin_ports::output::llm_port::LlmPort;
    ///
    /// async fn adaptive_strategy(llm: &dyn LlmPort) {
    ///     let caps = llm.get_capabilities();
    ///
    ///     // Choose strategy based on capabilities
    ///     if caps.supports_tool_calling {
    ///         println!("Using tool-augmented generation");
    ///         // Provide tool definitions
    ///     } else {
    ///         println!("Using pure text generation");
    ///         // No tools, rely on prompting
    ///     }
    ///
    ///     // Check context window
    ///     if let Some(max_tokens) = caps.max_context_tokens {
    ///         if max_tokens >= 100_000 {
    ///             println!("Long context mode: loading full documents");
    ///         } else {
    ///             println!("Standard context: using summaries");
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// ## Fallback Strategy
    ///
    /// ```rust,no_run
    /// use paladin_ports::output::llm_port::LlmPort;
    ///
    /// fn select_provider<'a>(
    ///     primary: &'a dyn LlmPort,
    ///     fallback: &'a dyn LlmPort,
    ///     need_vision: bool,
    /// ) -> &'a dyn LlmPort {
    ///     if need_vision && primary.get_capabilities().supports_vision {
    ///         primary
    ///     } else if fallback.get_capabilities().supports_vision {
    ///         fallback
    ///     } else {
    ///         primary // Use primary even if vision not supported
    ///     }
    /// }
    /// ```
    ///
    /// # Implementation Notes
    ///
    /// - Return accurate capabilities (don't report unsupported features)
    /// - Update capabilities as provider APIs evolve
    /// - Consider making this async if capabilities require API check
    fn get_capabilities(&self) -> ProviderCapabilities;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_capabilities_default() {
        let capabilities = ProviderCapabilities::default();
        assert!(!capabilities.supports_streaming);
        assert!(!capabilities.supports_tool_calling);
        assert!(!capabilities.supports_function_calling);
        assert!(!capabilities.supports_vision);
        assert!(!capabilities.supports_embeddings);
        assert_eq!(capabilities.max_context_tokens, None);
        assert!(capabilities.supports_system_messages);
        assert_eq!(capabilities.temperature_range, None);
    }

    #[test]
    fn test_provider_capabilities_default_temperature_range_is_none() {
        assert_eq!(ProviderCapabilities::default().temperature_range, None);
    }

    #[test]
    fn test_provider_capabilities_creation() {
        let capabilities = ProviderCapabilities {
            supports_streaming: true,
            supports_tool_calling: true,
            supports_function_calling: true,
            supports_vision: false,
            supports_embeddings: false,
            max_context_tokens: Some(128000),
            supports_system_messages: true,
            temperature_range: Some((0.0, 2.0)),
        };

        assert!(capabilities.supports_streaming);
        assert!(capabilities.supports_tool_calling);
        assert_eq!(capabilities.max_context_tokens, Some(128000));
        assert_eq!(capabilities.temperature_range, Some((0.0, 2.0)));
    }

    #[test]
    fn test_provider_capabilities_serialization() {
        let capabilities = ProviderCapabilities {
            supports_streaming: true,
            supports_tool_calling: false,
            supports_function_calling: true,
            supports_vision: false,
            supports_embeddings: false,
            max_context_tokens: Some(64000),
            supports_system_messages: true,
            temperature_range: Some((0.0, 1.0)),
        };

        // Test serialization
        let json = serde_json::to_string(&capabilities).unwrap();
        assert!(json.contains("supports_streaming"));
        assert!(json.contains("max_context_tokens"));
        assert!(json.contains("temperature_range"));

        // Test deserialization
        let deserialized: ProviderCapabilities = serde_json::from_str(&json).unwrap();
        assert_eq!(capabilities, deserialized);
    }

    #[test]
    fn test_provider_capabilities_equality() {
        let caps1 = ProviderCapabilities {
            supports_streaming: true,
            supports_tool_calling: true,
            supports_function_calling: true,
            supports_vision: false,
            supports_embeddings: false,
            max_context_tokens: Some(100000),
            supports_system_messages: true,
            temperature_range: Some((0.0, 1.0)),
        };

        let caps2 = ProviderCapabilities {
            supports_streaming: true,
            supports_tool_calling: true,
            supports_function_calling: true,
            supports_vision: false,
            supports_embeddings: false,
            max_context_tokens: Some(100000),
            supports_system_messages: true,
            temperature_range: Some((0.0, 1.0)),
        };

        assert_eq!(caps1, caps2);
    }

    /// One row per `LlmError` variant (D-05), so a variant added later
    /// cannot silently inherit a neighbour's classification.
    #[test]
    fn llm_error_transience_table() {
        use Transience::*;
        let cases: Vec<(LlmError, Transience)> = vec![
            (LlmError::NetworkError("x".into()), Transient),
            (LlmError::Timeout("x".into()), Transient),
            (LlmError::RateLimitExceeded, Transient),
            (LlmError::AuthenticationError("x".into()), Permanent),
            (LlmError::InvalidPrompt("x".into()), Permanent),
            (
                LlmError::UsageLimitExceeded {
                    provider: "openai".into(),
                    regain_hint: None,
                },
                Permanent,
            ),
            (LlmError::ModelNotAvailable("x".into()), Permanent),
            (LlmError::TokenLimitExceeded, Permanent),
            (LlmError::EmptyCompletion("x".into()), Permanent),
            (LlmError::ProcessingError("x".into()), Unknown),
        ];
        for (err, expected) in cases {
            assert_eq!(
                err.transience(),
                expected,
                "variant {err:?} classified as {:?}, expected {expected:?}",
                err.transience()
            );
        }
    }

    /// `ProviderError`'s boundary statuses classify by value only: 408, 429
    /// and every 5xx are Transient; every other 4xx (400, 401, 402, 404,
    /// 407, 418, 499) is Permanent.
    #[test]
    fn provider_error_status_boundaries_classify_by_value() {
        for status in [408u16, 429, 500, 503, 599] {
            let err = LlmError::ProviderError {
                provider: "openai".into(),
                status,
                message: "boom".into(),
            };
            assert_eq!(
                err.transience(),
                Transience::Transient,
                "status {status} should classify Transient"
            );
        }
        for status in [400u16, 401, 402, 404, 407, 418, 499] {
            let err = LlmError::ProviderError {
                provider: "openai".into(),
                status,
                message: "boom".into(),
            };
            assert_eq!(
                err.transience(),
                Transience::Permanent,
                "status {status} should classify Permanent"
            );
        }
    }

    /// Classification never reads the message: an empty `ProviderError`
    /// message still classifies by status.
    #[test]
    fn empty_provider_message_does_not_change_classification() {
        let err = LlmError::ProviderError {
            provider: "openai".into(),
            status: 503,
            message: String::new(),
        };
        assert_eq!(err.transience(), Transience::Transient);
    }

    /// `AllProvidersFailed` takes the transience of its LAST error.
    #[test]
    fn all_providers_failed_takes_the_transience_of_its_last_error() {
        let transient = LlmError::AllProvidersFailed {
            attempts: vec![("openai".into(), "boom".into())],
            last: Box::new(LlmError::NetworkError("connection reset".into())),
        };
        assert_eq!(transient.transience(), Transience::Transient);

        let permanent = LlmError::AllProvidersFailed {
            attempts: vec![("openai".into(), "boom".into())],
            last: Box::new(LlmError::AuthenticationError("bad key".into())),
        };
        assert_eq!(permanent.transience(), Transience::Permanent);
    }
}
