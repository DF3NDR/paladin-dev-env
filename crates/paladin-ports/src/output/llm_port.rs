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
//!
//! async fn basic_completion(llm: &dyn LlmPort) -> Result<String, Box<dyn std::error::Error>> {
//!     let request = LlmRequest::new(
//!         "gpt-4",
//!         PromptItem::new(PromptType::User(UserPrompt {
//!             query: "Explain hexagonal architecture".to_string(),
//!             context: None,
//!         })).unwrap(),
//!     );
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
//! use futures::StreamExt;
//!
//! async fn stream_completion(llm: &dyn LlmPort) -> Result<(), Box<dyn std::error::Error>> {
//!     // Check if streaming is supported
//!     let caps = llm.get_capabilities();
//!     if !caps.supports_streaming {
//!         eprintln!("Provider does not support streaming");
//!         return Ok(());
//!     }
//!
//!     let request = LlmRequest::new(
//!         "gpt-4",
//!         PromptItem::new(PromptType::User(UserPrompt {
//!             query: "Write a story".to_string(),
//!             context: None,
//!         })).unwrap(),
//!     ).with_stream(true);
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
//!             usage: TokenUsage::new(10, 20),
//!             cost: None,
//!             reasoning: None,
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
//! - [`VisionCapableLlm`](crate::output::vision_llm_port::VisionCapableLlm) - Extended LLM port with vision/image understanding
//! - [`EmbeddingPort`](crate::output::embedding_port::EmbeddingPort) - Generate vector embeddings from text
//! - [`GarrisonPort`](crate::output::garrison_port::GarrisonPort) - Store conversation history for context
//! - [`ArsenalPort`](crate::output::arsenal_port::ArsenalPort) - Provide tools for function calling
//!
//! ## See Also
//!
//! - Application Ports (`crate::application::ports`)
//! - [Paladin Domain](paladin_core::platform::container::paladin)
//! - Infrastructure Adapters (`crate::infrastructure::adapters::llm`)

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;
use uuid::Uuid;

use paladin_core::platform::container::content::ContentItem;
use paladin_core::platform::container::cost::Cost;
use paladin_core::platform::container::prompt::PromptItem;
use paladin_core::platform::container::transience::Transience;

use crate::output::rate_limit_hints::RateLimitHints;

/// Re-exported so `paladin_ports::output::llm_port::ModelProfile` resolves (ADR-0061).
pub use super::model_profile::{
    BudgetBounds, EffortLevels, EffortRefusal, EffortSupport, ModelProfile, ParamSupport,
    ResolvedEffort, SamplingParam, StopSupport,
};

/// Re-exported so a port caller names the effort vocabulary beside the request that carries it.
/// The types live in the domain core so `PaladinData` can hold them (Phase 51, REASON-01).
pub use paladin_core::platform::container::reasoning::{ReasoningEffort, ReasoningFallback};

/// The usage of a call the vendor answered and billed even though the call
/// failed (G-50-5a): a reasoning model that spent its whole budget thinking and
/// returned no text ([`LlmError::EmptyCompletion`]), or a refusal the vendor
/// still charged for ([`LlmError::Refusal`]).
///
/// `model` is the vendor-reported served model when the response named one,
/// else the request model. `usage` is the adapter's mapped [`TokenUsage`],
/// under the same inclusive contract as a successful response's. `cost` is set
/// only by a pricing decorator (`paladin_llm::pricing::PricingLlmAdapter`);
/// an adapter leaves it `None`. Holds numbers, a model id and a [`Cost`] only:
/// no response text and no credential.
///
/// Marked `#[non_exhaustive]`: build it with [`BilledUsage::new`] and
/// [`BilledUsage::with_cost`].
///
/// # Examples
///
/// ```rust
/// use paladin_core::platform::container::cost::{Cost, CostError, CurrencyCode};
/// use paladin_ports::output::llm_port::{BilledUsage, TokenUsage};
///
/// let billed = BilledUsage::new("deepseek-flash", TokenUsage::new(1_000, 62_000));
/// assert_eq!(billed.usage.total_tokens, 63_000);
/// assert!(billed.cost.is_none());
///
/// let priced = billed.with_cost(Cost::new(4_200, CurrencyCode::new("USD")?));
/// assert_eq!(priced.cost.map(|cost| cost.nanos()), Some(4_200));
/// # Ok::<(), CostError>(())
/// ```
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct BilledUsage {
    /// The model the vendor billed: the served model when the response named
    /// one, else the request model.
    pub model: String,
    /// The billed usage, mapped under [`TokenUsage`]'s inclusive contract.
    pub usage: TokenUsage,
    /// The priced cost, set only by a pricing decorator; `None` when unpriced.
    pub cost: Option<Cost>,
}

impl BilledUsage {
    /// A bill for `usage` on `model`, not yet priced (`cost` is `None`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_ports::output::llm_port::{BilledUsage, TokenUsage};
    ///
    /// let billed = BilledUsage::new("gpt-x", TokenUsage::new(3, 2));
    /// assert_eq!(billed.model, "gpt-x");
    /// assert!(billed.cost.is_none());
    /// ```
    pub fn new(model: impl Into<String>, usage: TokenUsage) -> Self {
        Self {
            model: model.into(),
            usage,
            cost: None,
        }
    }

    /// The same bill, priced at `cost`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_core::platform::container::cost::{Cost, CostError, CurrencyCode};
    /// use paladin_ports::output::llm_port::{BilledUsage, TokenUsage};
    ///
    /// let usd = CurrencyCode::new("USD")?;
    /// let priced = BilledUsage::new("gpt-x", TokenUsage::new(3, 2))
    ///     .with_cost(Cost::new(10, usd.clone()));
    /// assert_eq!(priced.cost, Some(Cost::new(10, usd)));
    /// # Ok::<(), CostError>(())
    /// ```
    #[must_use]
    pub fn with_cost(mut self, cost: Cost) -> Self {
        self.cost = Some(cost);
        self
    }
}

/// Errors that can occur during LLM operations
///
/// Each variant represents a specific failure mode with context for recovery strategies.
/// All errors implement `std::error::Error` via `thiserror`.
///
/// ## Error Categories
///
/// ### Retryable Errors
/// - [`LlmError::NetworkError`] - Retry with exponential backoff
/// - [`LlmError::RateLimitExceeded`] - Retry after delay (read [`LlmError::retry_after`])
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
///         LlmError::RateLimitExceeded { .. } => {
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
///
/// # Non-exhaustive (X-10.2, D-04)
///
/// Marked `#[non_exhaustive]` so a future variant can be added without a
/// semver-major bump. Registered as deliberate-breaking in `MIGRATION.md`
/// §9.2 and `.cargo/semver-checks-allowlist.toml` (the `enum_marked_non_exhaustive`
/// lint) in the same commit that added this attribute. Every downstream
/// exhaustive match gained a wildcard arm at that commit.
#[derive(Debug, Clone, Error)]
#[non_exhaustive]
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
    /// **Recovery**: Wait and retry. The provider's own delay, when it gave one, is
    /// [`LlmError::retry_after`]; the full parsed header snapshot is
    /// [`LlmError::rate_limit_hints`].
    ///
    /// Marked `#[non_exhaustive]` so a field can be added later without a break: build it
    /// with [`LlmError::rate_limited`] or [`LlmError::rate_limited_with_hints`], and match it
    /// with `RateLimitExceeded { .. }`. Its `Display` text is unchanged (`Rate limit exceeded`)
    /// and it is still [`Transience::Transient`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_ports::output::llm_port::LlmError;
    /// use std::time::Duration;
    ///
    /// fn handle_rate_limit(error: &LlmError) -> Option<Duration> {
    ///     if matches!(error, LlmError::RateLimitExceeded { .. }) {
    ///         // Never wait less than the provider asked for.
    ///         return error.retry_after();
    ///     }
    ///     None
    /// }
    ///
    /// let error = LlmError::rate_limited(Some(Duration::from_secs(7)));
    /// assert_eq!(handle_rate_limit(&error), Some(Duration::from_secs(7)));
    /// assert_eq!(error.to_string(), "Rate limit exceeded");
    /// ```
    #[error("Rate limit exceeded")]
    #[non_exhaustive]
    RateLimitExceeded {
        /// The effective delay before the next attempt, as the provider stated it. `None`
        /// when the provider gave nothing parseable -- a delay is never guessed.
        retry_after: Option<Duration>,
        /// The full parsed rate-limit snapshot, when the adapter had response headers.
        hints: Option<Box<RateLimitHints>>,
    },

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

    /// The model declined to answer: a policy decision by the model itself,
    /// reported by the vendor through a dedicated refusal field (D-09).
    ///
    /// A refusal is not a [`FinishReason::ContentFilter`] finish (that is an
    /// `Ok` response the caller inspects), not an empty answer
    /// ([`LlmError::EmptyCompletion`]) and not a generic processing failure.
    /// It is its own type so a consumer deciding whether to try another
    /// provider can tell a policy refusal apart from a fault.
    ///
    /// `detail` is the vendor's refusal text, already redacted and then
    /// bounded by the adapter that built this error (credentials removed
    /// first, so a key can never be sliced across the truncation boundary).
    /// It is display text: never parse it into a value anything schedules
    /// against.
    ///
    /// **Recovery**: Do not retry the same request against the same provider;
    /// it reproduces the same refusal. A caller may deliberately try another
    /// provider or model. `FallbackLlmAdapter` stops on a refusal by default.
    ///
    /// A refused call the vendor still billed carries that bill in `billed`
    /// (G-50-5a); read it with [`LlmError::billed_usage`]. Marked
    /// `#[non_exhaustive]`: build it with [`LlmError::refusal`] (plus
    /// [`LlmError::with_billed_usage`]) and match it with `Refusal { .. }`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_core::platform::container::transience::Transience;
    /// use paladin_ports::output::llm_port::LlmError;
    ///
    /// let error = LlmError::refusal("anthropic", "claude-sonnet-4", "I can't help with that.");
    /// let rendered = error.to_string();
    /// assert!(rendered.contains("anthropic"));
    /// assert!(rendered.contains("claude-sonnet-4"));
    /// assert!(rendered.contains("I can't help with that."));
    /// assert_eq!(error.transience(), Transience::Permanent);
    /// assert!(matches!(&error, LlmError::Refusal { model, .. } if model == "claude-sonnet-4"));
    /// assert!(error.billed_usage().is_none());
    /// ```
    #[error("Refused by provider '{provider}' (model '{model}'): {detail}")]
    #[non_exhaustive]
    Refusal {
        /// The provider whose model refused (e.g. `"anthropic"`, `"openai"`).
        provider: String,
        /// The model that refused, as named on the request.
        model: String,
        /// The vendor's refusal text, redacted then bounded. Display-only.
        detail: String,
        /// The usage the vendor billed for the refused call, when it reported
        /// any. `None` when it reported none, never a fabricated zero.
        billed: Option<Box<BilledUsage>>,
    },

    /// A request parameter the target model cannot honour was refused **before any HTTP call**
    /// (REASON-02, D-00f). Today the parameter is a reasoning effort: the model is unmeasured, has
    /// no reasoning, lacks that level, or the vendor would silently run it as another level.
    ///
    /// The text names the parameter, the value, the provider, the model and the reason, and
    /// nothing else: no prompt text, no request body and no credential ever reaches it. It is
    /// [`Transience::Permanent`]: the identical request is refused identically, so loops and
    /// fallback chains do not retry it.
    ///
    /// **Recovery**: choose a level the model offers, pick a substituting
    /// [`ReasoningFallback`], or state the model's support in the
    /// `llm.<provider>.models.<model-id>.effort` override.
    ///
    /// Marked `#[non_exhaustive]`: build it with [`LlmError::unsupported_parameter`] and match
    /// it with `UnsupportedParameter { .. }` (naming fields followed by `..`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_core::platform::container::transience::Transience;
    /// use paladin_ports::output::llm_port::LlmError;
    ///
    /// let error = LlmError::unsupported_parameter(
    ///     "reasoning_effort",
    ///     "high",
    ///     "deepseek",
    ///     "deepseek-v4-pro",
    ///     "no measured effort data",
    /// );
    /// let rendered = error.to_string();
    /// assert!(rendered.contains("reasoning_effort"));
    /// assert!(rendered.contains("deepseek-v4-pro"));
    /// assert_eq!(error.transience(), Transience::Permanent);
    /// assert!(matches!(&error, LlmError::UnsupportedParameter { provider, .. } if provider == "deepseek"));
    /// ```
    #[error(
        "Unsupported parameter '{param}' = '{value}' for provider '{provider}' (model '{model}'): {reason}"
    )]
    #[non_exhaustive]
    UnsupportedParameter {
        /// The request parameter that was refused (for example `"reasoning_effort"`).
        param: String,
        /// The value that was refused, in the words a caller wrote it (for example `"high"`).
        value: String,
        /// The provider whose model cannot honour it.
        provider: String,
        /// The model that cannot honour it, as named on the request.
        model: String,
        /// Why it was refused, in plain English. Names how to fix it; carries no request content.
        reason: String,
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

    /// The provider returned no usable text: `content` was empty or
    /// whitespace-only on a finish that is neither a refusal, a content
    /// filter nor a function call (D-11a).
    ///
    /// The payload names the finish reason and carries the advice that fits
    /// it. For `finish_reason=length` this is the signature of a reasoning
    /// model (e.g. DeepSeek's `-flash`/`-pro` variants) whose hidden
    /// reasoning consumed the entire `max_tokens` budget, so the advice is a
    /// larger `max_tokens`. For `finish_reason=stop` the model simply
    /// returned no text and a larger budget would not help. It is distinct
    /// from [`LlmError::TokenLimitExceeded`], whose recovery (reduce prompt
    /// size) is the opposite of what a truncation needs.
    ///
    /// **Recovery**: Read the payload. For a `length` truncation, retry with a
    /// larger `max_tokens` budget; do not retry an identical request, it will
    /// reproduce the same truncation. For a `stop` with no text, rephrase the
    /// request or try another model.
    ///
    /// The vendor usually bills such a call (a `length` truncation bills the
    /// whole budget). That bill rides in `billed` (G-50-5a); read it with
    /// [`LlmError::billed_usage`]. Marked `#[non_exhaustive]`: build it with
    /// [`LlmError::empty_completion`] (plus [`LlmError::with_billed_usage`])
    /// and match it with `EmptyCompletion { .. }` or
    /// `EmptyCompletion { detail, .. }`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_ports::output::llm_port::LlmError;
    ///
    /// let error = LlmError::empty_completion(
    ///     "finish_reason=length, content empty (reasoning consumed max_tokens budget); \
    ///      retry with a larger max_tokens budget",
    /// );
    /// assert!(error.to_string().starts_with("Empty completion:"));
    /// assert!(error.to_string().contains("max_tokens"));
    /// if let LlmError::EmptyCompletion { detail, .. } = &error {
    ///     assert!(detail.starts_with("finish_reason=length"));
    /// }
    /// ```
    #[error("Empty completion: {detail}")]
    #[non_exhaustive]
    EmptyCompletion {
        /// The finish reason and the advice that fits it. Display-only.
        detail: String,
        /// The usage the vendor billed for the empty call, when it reported
        /// any. `None` when it reported none, never a fabricated zero.
        billed: Option<Box<BilledUsage>>,
    },

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
    /// Build a [`LlmError::RateLimitExceeded`] from an optional provider retry delay.
    ///
    /// Pass `None` when the provider gave no delay; nothing is guessed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_ports::output::llm_port::LlmError;
    /// use std::time::Duration;
    ///
    /// assert_eq!(LlmError::rate_limited(None).retry_after(), None);
    /// assert_eq!(
    ///     LlmError::rate_limited(Some(Duration::from_secs(3))).retry_after(),
    ///     Some(Duration::from_secs(3))
    /// );
    /// ```
    pub fn rate_limited(retry_after: Option<Duration>) -> Self {
        LlmError::RateLimitExceeded {
            retry_after,
            hints: None,
        }
    }

    /// Build a [`LlmError::EmptyCompletion`]: the provider answered but returned no usable
    /// text. `detail` names the finish reason and carries the advice that fits it.
    ///
    /// This is the supported way to build the variant (it is `#[non_exhaustive]`). The value
    /// carries no bill; attach the vendor's billed usage with [`LlmError::with_billed_usage`].
    /// Match it with `EmptyCompletion { .. }`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_core::platform::container::transience::Transience;
    /// use paladin_ports::output::llm_port::LlmError;
    ///
    /// let error = LlmError::empty_completion("finish_reason=stop, the model returned no text");
    /// assert!(matches!(error, LlmError::EmptyCompletion { .. }));
    /// assert_eq!(
    ///     error.to_string(),
    ///     "Empty completion: finish_reason=stop, the model returned no text"
    /// );
    /// assert_eq!(error.transience(), Transience::Permanent);
    /// ```
    pub fn empty_completion(detail: impl Into<String>) -> Self {
        LlmError::EmptyCompletion {
            detail: detail.into(),
            billed: None,
        }
    }

    /// Build a [`LlmError::Refusal`]: `model` at `provider` declined to answer. `detail` is
    /// the vendor's refusal text, already redacted and then bounded by the caller.
    ///
    /// This is the supported way to build the variant (it is `#[non_exhaustive]`). The value
    /// carries no bill; attach the vendor's billed usage with [`LlmError::with_billed_usage`].
    /// Match it with `Refusal { .. }` (naming fields followed by `..`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_core::platform::container::transience::Transience;
    /// use paladin_ports::output::llm_port::LlmError;
    ///
    /// let error = LlmError::refusal("anthropic", "claude-sonnet-4", "I can't help with that.");
    /// assert!(matches!(
    ///     &error,
    ///     LlmError::Refusal { provider, .. } if provider == "anthropic"
    /// ));
    /// assert_eq!(
    ///     error.to_string(),
    ///     "Refused by provider 'anthropic' (model 'claude-sonnet-4'): I can't help with that."
    /// );
    /// assert_eq!(error.transience(), Transience::Permanent);
    /// ```
    pub fn refusal(
        provider: impl Into<String>,
        model: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        LlmError::Refusal {
            provider: provider.into(),
            model: model.into(),
            detail: detail.into(),
            billed: None,
        }
    }

    /// Build a [`LlmError::UnsupportedParameter`]: `model` at `provider` cannot honour `param`
    /// set to `value`, for the stated `reason`.
    ///
    /// This is the supported way to build the variant (it is `#[non_exhaustive]`). It takes
    /// only these five strings, so no request content and no credential can reach the error.
    /// Match it with `UnsupportedParameter { .. }`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_ports::output::llm_port::LlmError;
    ///
    /// let error = LlmError::unsupported_parameter(
    ///     "reasoning_effort",
    ///     "xhigh",
    ///     "openai",
    ///     "gpt-x",
    ///     "the model does not offer that effort level",
    /// );
    /// assert_eq!(
    ///     error.to_string(),
    ///     "Unsupported parameter 'reasoning_effort' = 'xhigh' for provider 'openai' \
    ///      (model 'gpt-x'): the model does not offer that effort level"
    /// );
    /// assert!(error.billed_usage().is_none());
    /// ```
    pub fn unsupported_parameter(
        param: impl Into<String>,
        value: impl Into<String>,
        provider: impl Into<String>,
        model: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        LlmError::UnsupportedParameter {
            param: param.into(),
            value: value.into(),
            provider: provider.into(),
            model: model.into(),
            reason: reason.into(),
        }
    }

    /// Attach the usage the vendor billed for a call that still failed (G-50-5a).
    ///
    /// Only [`LlmError::EmptyCompletion`] and [`LlmError::Refusal`] carry a bill: they are the
    /// outcomes of a response the vendor returned and charged for. On
    /// [`LlmError::AllProvidersFailed`] the bill goes to its last error. Every other variant
    /// (transport, status, rate-limit errors, which never carry a billed body) is returned
    /// unchanged. An existing bill is replaced.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_ports::output::llm_port::{BilledUsage, LlmError, TokenUsage};
    ///
    /// let billed = BilledUsage::new("deepseek-flash", TokenUsage::new(1_000, 62_000));
    /// let error = LlmError::empty_completion("finish_reason=length").with_billed_usage(billed);
    /// assert_eq!(
    ///     error.billed_usage().map(|bill| bill.usage.total_tokens),
    ///     Some(63_000)
    /// );
    ///
    /// // A transport error never carries a bill.
    /// let network = LlmError::NetworkError("reset".to_string())
    ///     .with_billed_usage(BilledUsage::new("m", TokenUsage::new(1, 1)));
    /// assert!(network.billed_usage().is_none());
    /// ```
    #[must_use]
    pub fn with_billed_usage(self, billed: BilledUsage) -> Self {
        match self {
            LlmError::EmptyCompletion { detail, .. } => LlmError::EmptyCompletion {
                detail,
                billed: Some(Box::new(billed)),
            },
            LlmError::Refusal {
                provider,
                model,
                detail,
                ..
            } => LlmError::Refusal {
                provider,
                model,
                detail,
                billed: Some(Box::new(billed)),
            },
            LlmError::AllProvidersFailed { attempts, last } => LlmError::AllProvidersFailed {
                attempts,
                last: Box::new(last.with_billed_usage(billed)),
            },
            other => other,
        }
    }

    /// The usage the vendor billed for this failed call, if it reported any (G-50-5a).
    ///
    /// `Some` only on an [`LlmError::EmptyCompletion`] or [`LlmError::Refusal`] that an adapter
    /// billed; like [`retry_after`](LlmError::retry_after) this looks through
    /// [`LlmError::AllProvidersFailed`] to its last error. `None` means no usage was reported,
    /// never a zero bill.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_ports::output::llm_port::{BilledUsage, LlmError, TokenUsage};
    ///
    /// let refusal = LlmError::refusal("openai", "gpt-x", "I can't help with that.");
    /// assert!(refusal.billed_usage().is_none());
    ///
    /// let billed = refusal.with_billed_usage(BilledUsage::new("gpt-x", TokenUsage::new(12, 3)));
    /// let bill = billed.billed_usage();
    /// assert_eq!(bill.map(|bill| bill.model.as_str()), Some("gpt-x"));
    /// assert_eq!(bill.and_then(|bill| bill.cost.as_ref()), None);
    /// ```
    pub fn billed_usage(&self) -> Option<&BilledUsage> {
        match self {
            LlmError::EmptyCompletion { billed, .. } | LlmError::Refusal { billed, .. } => {
                billed.as_deref()
            }
            LlmError::AllProvidersFailed { last, .. } => last.billed_usage(),
            _ => None,
        }
    }

    /// Build a [`LlmError::RateLimitExceeded`] from a parsed header snapshot. The error's
    /// [`retry_after`](LlmError::retry_after) is the snapshot's
    /// [`effective_retry_after`](RateLimitHints::effective_retry_after); a snapshot that
    /// recorded nothing is not stored.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_ports::output::llm_port::LlmError;
    /// use paladin_ports::output::rate_limit_hints::{RateLimitHints, RetryDelaySource};
    /// use std::time::Duration;
    ///
    /// let hints = RateLimitHints::default()
    ///     .with_retry_after(Duration::from_secs(5), RetryDelaySource::RetryAfter);
    /// let error = LlmError::rate_limited_with_hints(hints);
    /// assert_eq!(error.retry_after(), Some(Duration::from_secs(5)));
    /// assert!(error.rate_limit_hints().is_some());
    /// ```
    pub fn rate_limited_with_hints(hints: RateLimitHints) -> Self {
        let retry_after = hints.effective_retry_after().map(|(delay, _)| delay);
        LlmError::RateLimitExceeded {
            retry_after,
            hints: (!hints.is_empty()).then(|| Box::new(hints)),
        }
    }

    /// The provider's retry delay carried by a rate-limit error, if any.
    ///
    /// For [`LlmError::AllProvidersFailed`] this is the delay of its last error, mirroring how
    /// [`transience`](LlmError::transience) delegates; every other variant reports `None`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_ports::output::llm_port::LlmError;
    /// use std::time::Duration;
    ///
    /// let chain = LlmError::AllProvidersFailed {
    ///     attempts: vec![("openai".to_string(), "rate limited".to_string())],
    ///     last: Box::new(LlmError::rate_limited(Some(Duration::from_secs(9)))),
    /// };
    /// assert_eq!(chain.retry_after(), Some(Duration::from_secs(9)));
    /// assert_eq!(LlmError::TokenLimitExceeded.retry_after(), None);
    /// ```
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            LlmError::RateLimitExceeded { retry_after, .. } => *retry_after,
            LlmError::AllProvidersFailed { last, .. } => last.retry_after(),
            _ => None,
        }
    }

    /// The parsed rate-limit snapshot carried by a rate-limit error, if the adapter had one.
    ///
    /// Like [`retry_after`](LlmError::retry_after) this looks through
    /// [`LlmError::AllProvidersFailed`] to its last error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_ports::output::llm_port::LlmError;
    ///
    /// assert!(LlmError::rate_limited(None).rate_limit_hints().is_none());
    /// ```
    pub fn rate_limit_hints(&self) -> Option<&RateLimitHints> {
        match self {
            LlmError::RateLimitExceeded { hints, .. } => hints.as_deref(),
            LlmError::AllProvidersFailed { last, .. } => last.rate_limit_hints(),
            _ => None,
        }
    }

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
            LlmError::RateLimitExceeded { .. } => Transience::Transient,

            // Obviously permanent: retrying the exact same request
            // reproduces the exact same failure. Each of these is matched on
            // its own dedicated arm -- never falling through to
            // `ProviderError`'s status-range classification below.
            LlmError::AuthenticationError(_) => Transience::Permanent,
            LlmError::InvalidPrompt(_) => Transience::Permanent,
            LlmError::UsageLimitExceeded { .. } => Transience::Permanent,
            LlmError::ModelNotAvailable(_) => Transience::Permanent,
            LlmError::TokenLimitExceeded => Transience::Permanent,
            LlmError::EmptyCompletion { .. } => Transience::Permanent,
            LlmError::Refusal { .. } => Transience::Permanent,
            // Refused before any HTTP call: the identical request is refused identically.
            LlmError::UnsupportedParameter { .. } => Transience::Permanent,

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

/// A structured-output request hint attached to an [`LlmRequest`] (RT-FR-17, D-28).
///
/// `ResponseFormat` tells an adapter that the caller wants its completion constrained
/// to JSON, optionally against a named JSON Schema. It is a **hint**, not a contract:
/// an adapter with no native structured-output mode -- Anthropic, as of this writing,
/// see the per-provider table in the `agent-runtime` user guide (plan 26-21) -- ignores
/// this field harmlessly. Correctness never depends on native support, because the
/// caller separately appends a schema-conformance instruction block to the prompt
/// (D-27's belt-and-braces rule): `response_format` is an optimization a supporting
/// provider can use, never the only mechanism enforcing shape.
///
/// # Non-exhaustive (X-10.2, D-28)
///
/// Marked `#[non_exhaustive]` so a future variant (e.g. a provider-specific mode) can
/// be added without a semver-major bump.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ResponseFormat {
    /// Constrain the completion to a syntactically valid JSON object, with no schema
    /// attached.
    JsonObject,
    /// Constrain the completion to a JSON object conforming to `schema`.
    JsonSchema {
        /// A human-readable name for the schema (required by some providers' native
        /// JSON-schema modes).
        name: String,
        /// The JSON Schema the completion must conform to.
        schema: serde_json::Value,
        /// Whether the provider should enforce the schema strictly (rejecting
        /// additional properties) where it supports doing so.
        strict: bool,
    },
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
/// - `response_format`: Optional structured-output hint (see [`ResponseFormat`])
///
/// # Construction (X-10.3, D-28)
///
/// As of v0.10.0 this struct is `#[non_exhaustive]` and gained the additive
/// `response_format` field. Construct it through [`LlmRequest::new`] plus the
/// chainable `with_*` builders (`with_attachments`, `with_stream`, `with_metadata`,
/// `with_response_format`) -- a full struct literal no longer compiles outside this
/// crate. This is a one-way decision: `LlmRequest` has no `Default`, so there was no
/// functional-update escape hatch, and the constructor keeps the *next* field free.
///
/// # Examples
///
/// ## Basic Text Request
///
/// ```rust
/// use paladin_ports::output::llm_port::LlmRequest;
/// use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
///
/// let request = LlmRequest::new(
///     "gpt-4",
///     PromptItem::new(PromptType::User(UserPrompt {
///         query: "Explain Rust ownership".to_string(),
///         context: None,
///     })).unwrap(),
/// );
/// ```
///
/// ## Request with Metadata
///
/// ```rust
/// use paladin_ports::output::llm_port::LlmRequest;
/// use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
/// use std::collections::HashMap;
///
/// let mut metadata = HashMap::new();
/// metadata.insert("user_id".to_string(), "user123".to_string());
/// metadata.insert("session_id".to_string(), "sess456".to_string());
/// metadata.insert("temperature".to_string(), "0.7".to_string());
///
/// let request = LlmRequest::new(
///     "gpt-4",
///     PromptItem::new(PromptType::User(UserPrompt {
///         query: "Analyze this data".to_string(),
///         context: None,
///     })).unwrap(),
/// ).with_metadata(metadata);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
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
    /// Optional structured-output hint for this request (see [`ResponseFormat`]).
    ///
    /// `None` by default; absent from a deserialized document also resolves to
    /// `None` (`#[serde(default)]`), so legacy JSON without this field still
    /// deserializes.
    #[serde(default)]
    pub response_format: Option<ResponseFormat>,
    /// The reasoning effort to ask the model for (see [`ReasoningEffort`]).
    ///
    /// `None` by default and then **nothing** about reasoning is sent: the vendor's own default
    /// stays in force and the request body is unchanged. [`ReasoningEffort::Auto`] is a set
    /// value, not the same as `None`. Precedence when several layers state one: this request,
    /// then the agent, then the provider config, then the vendor default. A model that cannot
    /// honour the effort refuses the request with [`LlmError::UnsupportedParameter`] before any
    /// HTTP call. Absent from a serialised request when `None`, so an unset request serialises
    /// exactly as before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ReasoningEffort>,
    /// What to do when the model lacks the exact level in [`reasoning`](Self::reasoning).
    ///
    /// `None` means the layer below decides, ending at [`ReasoningFallback::Refuse`]. Absent from
    /// a serialised request when `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_fallback: Option<ReasoningFallback>,
}

impl LlmRequest {
    /// Construct a new [`LlmRequest`] with sensible defaults for every field except
    /// `model` and `prompt`.
    ///
    /// Produces a fresh [`Uuid`] (different on every call), no attachments,
    /// `stream: false`, empty `metadata`, `response_format: None`, and no reasoning effort or
    /// fallback (`None`). Use the chainable `with_*` methods to override any default.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_ports::output::llm_port::LlmRequest;
    /// use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
    ///
    /// let prompt = PromptItem::new(PromptType::User(UserPrompt {
    ///     query: "Hello".to_string(),
    ///     context: None,
    /// })).unwrap();
    /// let request = LlmRequest::new("gpt-4", prompt);
    ///
    /// assert_eq!(request.model, "gpt-4");
    /// assert!(request.attachments.is_empty());
    /// assert!(!request.stream);
    /// assert!(request.metadata.is_empty());
    /// assert!(request.response_format.is_none());
    /// assert!(request.reasoning.is_none());
    /// assert!(request.reasoning_fallback.is_none());
    /// ```
    pub fn new(model: impl Into<String>, prompt: PromptItem) -> Self {
        Self {
            id: Uuid::new_v4(),
            model: model.into(),
            prompt,
            attachments: Vec::new(),
            stream: false,
            metadata: HashMap::new(),
            response_format: None,
            reasoning: None,
            reasoning_fallback: None,
        }
    }

    /// Attach additional content (images, documents) for multimodal models.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_ports::output::llm_port::LlmRequest;
    /// use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
    ///
    /// let prompt = PromptItem::new(PromptType::User(UserPrompt {
    ///     query: "Describe this image".to_string(),
    ///     context: None,
    /// })).unwrap();
    /// let request = LlmRequest::new("gpt-4-vision", prompt).with_attachments(vec![]);
    ///
    /// assert!(request.attachments.is_empty());
    /// ```
    pub fn with_attachments(mut self, attachments: Vec<ContentItem>) -> Self {
        self.attachments = attachments;
        self
    }

    /// Enable or disable streaming for this request.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_ports::output::llm_port::LlmRequest;
    /// use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
    ///
    /// let prompt = PromptItem::new(PromptType::User(UserPrompt {
    ///     query: "Write a story".to_string(),
    ///     context: None,
    /// })).unwrap();
    /// let request = LlmRequest::new("gpt-4", prompt).with_stream(true);
    ///
    /// assert!(request.stream);
    /// ```
    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }

    /// Attach custom metadata (e.g. `"temperature"`, `"user_id"`) to this request.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_ports::output::llm_port::LlmRequest;
    /// use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
    /// use std::collections::HashMap;
    ///
    /// let mut metadata = HashMap::new();
    /// metadata.insert("user_id".to_string(), "user123".to_string());
    ///
    /// let prompt = PromptItem::new(PromptType::User(UserPrompt {
    ///     query: "Hello".to_string(),
    ///     context: None,
    /// })).unwrap();
    /// let request = LlmRequest::new("gpt-4", prompt).with_metadata(metadata);
    ///
    /// assert_eq!(request.metadata.get("user_id"), Some(&"user123".to_string()));
    /// ```
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Request a structured-output mode from a supporting provider (see
    /// [`ResponseFormat`]).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_ports::output::llm_port::{LlmRequest, ResponseFormat};
    /// use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
    ///
    /// let prompt = PromptItem::new(PromptType::User(UserPrompt {
    ///     query: "Return JSON".to_string(),
    ///     context: None,
    /// })).unwrap();
    /// let request = LlmRequest::new("gpt-4", prompt)
    ///     .with_response_format(ResponseFormat::JsonObject);
    ///
    /// assert_eq!(request.response_format, Some(ResponseFormat::JsonObject));
    /// ```
    pub fn with_response_format(mut self, response_format: ResponseFormat) -> Self {
        self.response_format = Some(response_format);
        self
    }

    /// Ask the model for a reasoning effort. **This sets the effort the request asks for.**
    /// It is not [`LlmResponse::with_reasoning`], which records the reasoning *text* a vendor
    /// returned, nor [`TokenUsage::with_reasoning`], which records the reasoning-token *count*.
    ///
    /// Left unset, nothing is sent and the vendor default applies. A model that cannot honour
    /// the effort refuses the request with [`LlmError::UnsupportedParameter`] before any HTTP
    /// call.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_ports::output::llm_port::{LlmRequest, ReasoningEffort};
    /// use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
    ///
    /// let prompt = PromptItem::new(PromptType::User(UserPrompt {
    ///     query: "Prove it".to_string(),
    ///     context: None,
    /// })).unwrap();
    /// let request = LlmRequest::new("gpt-4", prompt).with_reasoning(ReasoningEffort::High);
    ///
    /// assert_eq!(request.reasoning, Some(ReasoningEffort::High));
    /// ```
    #[must_use]
    pub fn with_reasoning(mut self, effort: ReasoningEffort) -> Self {
        self.reasoning = Some(effort);
        self
    }

    /// Choose what to do when the model lacks the exact level asked for with
    /// [`with_reasoning`](Self::with_reasoning). Left unset, the request is refused
    /// ([`ReasoningFallback::Refuse`]).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_ports::output::llm_port::{LlmRequest, ReasoningFallback};
    /// use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
    ///
    /// let prompt = PromptItem::new(PromptType::User(UserPrompt {
    ///     query: "Prove it".to_string(),
    ///     context: None,
    /// })).unwrap();
    /// let request = LlmRequest::new("gpt-4", prompt)
    ///     .with_reasoning_fallback(ReasoningFallback::NearestLower);
    ///
    /// assert_eq!(request.reasoning_fallback, Some(ReasoningFallback::NearestLower));
    /// ```
    #[must_use]
    pub fn with_reasoning_fallback(mut self, fallback: ReasoningFallback) -> Self {
        self.reasoning_fallback = Some(fallback);
        self
    }
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
/// // Cost is computed by a pricing decorator (`paladin_llm::pricing::PricingLlmAdapter`,
/// // D-09) from `response.model` and `response.usage` -- never re-derived here from a bare
/// // total token count with a floating-point multiplier (D-00e, D-02).
/// match &response.cost {
///     Some(cost) => println!("Cost: {} nanos {}", cost.nanos(), cost.currency()),
///     None => println!("Cost: unpriced (no pricing configured for this model)"),
/// }
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
    /// Cost of this call, computed by a pricing decorator
    /// (`paladin_llm::pricing::PricingLlmAdapter`, D-09) from this response's own `model` and
    /// `usage`. `None` when no pricing decorator is installed, or when the model has no
    /// configured price row -- never a fabricated zero (D-00c). Absent from serialized JSON
    /// when `None`, so a `LlmResponse` document persisted before this field existed
    /// deserializes with `cost == None`, byte-identical to what it deserialized to before
    /// (the `served_by` precedent, D-25).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<Cost>,
    /// The vendor's reasoning (thinking) text for this response, when it returned one (D-11).
    ///
    /// `None` when the model returned no reasoning text, and never `Some("")`: empty or
    /// whitespace-only text is stored as `None` by [`LlmResponse::with_reasoning`]. It is a typed
    /// field on purpose -- reasoning text is never carried through `metadata` or any other
    /// untyped channel, so consumers and the Phase 51 reasoning surface can rely on this shape.
    /// Absent from serialized JSON when `None`, so a `LlmResponse` document persisted before this
    /// field existed deserializes with `reasoning == None`, and a response without reasoning
    /// serializes byte-identically to before (the `cost` precedent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
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

impl LlmResponse {
    /// Construct a response from the five fields every adapter knows: the originating request,
    /// the served model, the generated text, why generation stopped, and the token usage.
    ///
    /// The remaining fields take their neutral values: a fresh `id`, `cost: None` (a pricing
    /// decorator fills it), `created_at: Utc::now()`, empty `metadata` and `function_call: None`.
    /// Because `LlmResponse` is constructible, adding a field to it breaks every caller that
    /// writes a full struct literal; a caller that goes through this constructor is insulated
    /// from the next field (MIGRATION DR-6) and can chain a `with_*` method for the one it sets.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_ports::output::llm_port::{FinishReason, LlmResponse, TokenUsage};
    /// use uuid::Uuid;
    ///
    /// let response = LlmResponse::new(
    ///     Uuid::new_v4(),
    ///     "gpt-4o-mini",
    ///     "Hello!",
    ///     FinishReason::Stop,
    ///     TokenUsage::new(3, 2),
    /// );
    /// assert_eq!(response.content, "Hello!");
    /// assert!(response.cost.is_none());
    /// assert!(response.metadata.is_empty());
    /// assert!(response.function_call.is_none());
    /// ```
    pub fn new(
        request_id: Uuid,
        model: impl Into<String>,
        content: impl Into<String>,
        finish_reason: FinishReason,
        usage: TokenUsage,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            request_id,
            model: model.into(),
            content: content.into(),
            finish_reason,
            usage,
            cost: None,
            reasoning: None,
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: None,
        }
    }

    /// Chainable: attach the vendor's reasoning (thinking) text to this response (D-11).
    ///
    /// Text that is empty after trimming is stored as `None`, so a vendor's empty reasoning
    /// field never becomes `Some("")`.
    ///
    /// This records the reasoning *text* a vendor returned. It does not ask for any effort: to
    /// set the effort a request asks for, use [`LlmRequest::with_reasoning`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_ports::output::llm_port::{FinishReason, LlmResponse, TokenUsage};
    /// use uuid::Uuid;
    ///
    /// let response = LlmResponse::new(
    ///     Uuid::new_v4(),
    ///     "deepseek-reasoner",
    ///     "42",
    ///     FinishReason::Stop,
    ///     TokenUsage::new(5, 9),
    /// )
    /// .with_reasoning("Six times seven is forty-two.");
    /// assert_eq!(response.reasoning.as_deref(), Some("Six times seven is forty-two."));
    ///
    /// let blank = LlmResponse::new(
    ///     Uuid::new_v4(),
    ///     "deepseek-reasoner",
    ///     "42",
    ///     FinishReason::Stop,
    ///     TokenUsage::new(5, 9),
    /// )
    /// .with_reasoning("   ");
    /// assert!(blank.reasoning.is_none());
    /// ```
    #[must_use]
    pub fn with_reasoning(mut self, text: impl Into<String>) -> Self {
        let text = text.into();
        self.reasoning = if text.trim().is_empty() {
            None
        } else {
            Some(text)
        };
        self
    }
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
/// let usage = TokenUsage::new(150, 300);
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
/// use paladin_ports::output::llm_port::{FinishReason, StreamingResponse};
///
/// // Accumulate streaming chunks
/// let mut accumulated = String::new();
/// let chunks = vec![
///     StreamingResponse::delta("Hello"),
///     StreamingResponse::delta(" world"),
///     StreamingResponse::delta("!"),
///     StreamingResponse::terminal(FinishReason::Stop),
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
///
/// # Streaming usage contract (D-13, D-14)
///
/// As of v0.10.0 this struct is `#[non_exhaustive]` and gained the additive
/// `usage` field. Construct it through [`StreamingResponse::delta`] /
/// [`StreamingResponse::terminal`] plus the chainable
/// [`StreamingResponse::with_usage`] -- a full struct literal no longer
/// compiles outside this crate. This is a one-way decision, mirroring
/// [`LlmRequest`]: `StreamingResponse` has no `Default`, so there was no
/// functional-update escape hatch, and the constructors keep the *next*
/// field free.
///
/// `usage` is `Some` on exactly ONE chunk per stream -- the terminal chunk,
/// the same one that carries `finish_reason: Some(..)` -- and `None` on
/// every other chunk. A provider whose stream ends without ever reporting
/// usage yields a terminal chunk with `usage: None`; the execution service
/// then records [`TokenUsage::default`] for that call rather than
/// substituting a locally-computed estimate (D-17).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct StreamingResponse {
    /// Unique identifier for this chunk
    pub id: Uuid,
    /// Incremental text to append to the response
    pub delta: String,
    /// Why generation stopped (present only in final chunk)
    pub finish_reason: Option<FinishReason>,
    /// Token usage reported by the provider, present only on the terminal
    /// chunk that also carries `finish_reason: Some(..)` (D-13/D-14). `None`
    /// on every other chunk, and `None` on the terminal chunk itself when
    /// the provider's stream ended without reporting usage (D-16/D-17) --
    /// never a locally-computed estimate.
    #[serde(default)]
    pub usage: Option<TokenUsage>,
    /// Cost of this call, set by a pricing decorator (`PricingLlmAdapter` in
    /// `paladin-llm`, D-09) on the terminal chunk only, from that chunk's own
    /// `usage`. `None` on every other chunk, for a model with no configured
    /// price row, and when the terminal chunk reported no usage at all
    /// (D-00c -- never a fabricated zero).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<Cost>,
    /// Reasoning (thinking) text the vendor streamed on this chunk (D-11).
    ///
    /// `None` on every chunk that carried no reasoning text, and never `Some("")`: an empty or
    /// whitespace-only fragment is stored as `None` by [`StreamingResponse::with_reasoning`].
    /// Callers concatenate the `Some` values in wire order to rebuild the full reasoning text,
    /// exactly as they concatenate `delta` for the answer. It is a typed field on purpose --
    /// reasoning text is never carried through an untyped metadata channel (ADR-0061's sibling
    /// decision D-11). Absent from serialized JSON when `None`, so a chunk persisted before this
    /// field existed deserializes with `reasoning == None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    /// The `paladin.*` visibility keys this call stamped, on the **terminal chunk only**
    /// (Phase 50, ADR-0061 (h), owner reading R-1).
    ///
    /// The same keys the buffered path puts in [`LlmResponse::metadata`]
    /// (`paladin.params_omitted`, `paladin.unprofiled_model`,
    /// `paladin.response_format_adjusted`; `paladin.served_by` and `paladin.served_model` on a
    /// fallback chain's served stream), so a streamed call is as visible as a buffered one
    /// without log scraping. Empty on every content chunk, on a terminal chunk of a call that
    /// adjusted nothing, and never carried by an `Err` terminal item. Values are provider
    /// names, model ids and parameter names only -- never request content or credentials.
    /// Absent from serialized JSON when empty, so a chunk persisted before this field existed
    /// deserializes with an empty map. Set it through [`StreamingResponse::with_metadata`].
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

impl StreamingResponse {
    /// Construct a non-terminal delta chunk: fresh [`Uuid`], no finish
    /// reason, no usage.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_ports::output::llm_port::StreamingResponse;
    ///
    /// let chunk = StreamingResponse::delta("Hello");
    /// assert_eq!(chunk.delta, "Hello");
    /// assert!(chunk.finish_reason.is_none());
    /// assert!(chunk.usage.is_none());
    /// ```
    pub fn delta(text: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            delta: text.into(),
            finish_reason: None,
            usage: None,
            cost: None,
            reasoning: None,
            metadata: HashMap::new(),
        }
    }

    /// Construct a terminal chunk: fresh [`Uuid`], empty delta, the given
    /// finish reason, no usage yet -- chain [`Self::with_usage`] when the
    /// provider reported one.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_ports::output::llm_port::{FinishReason, StreamingResponse};
    ///
    /// let chunk = StreamingResponse::terminal(FinishReason::Stop);
    /// assert!(chunk.delta.is_empty());
    /// assert!(matches!(chunk.finish_reason, Some(FinishReason::Stop)));
    /// assert!(chunk.usage.is_none());
    /// ```
    pub fn terminal(finish_reason: FinishReason) -> Self {
        Self {
            id: Uuid::new_v4(),
            delta: String::new(),
            finish_reason: Some(finish_reason),
            usage: None,
            cost: None,
            reasoning: None,
            metadata: HashMap::new(),
        }
    }

    /// Chainable: attach the provider's reported usage to this chunk (D-14).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_ports::output::llm_port::{FinishReason, StreamingResponse, TokenUsage};
    ///
    /// let chunk = StreamingResponse::terminal(FinishReason::Stop)
    ///     .with_usage(TokenUsage::new(11, 7));
    /// assert_eq!(chunk.usage, Some(TokenUsage::new(11, 7)));
    /// ```
    pub fn with_usage(mut self, usage: TokenUsage) -> Self {
        self.usage = Some(usage);
        self
    }

    /// Chainable: attach a priced cost to this chunk (D-09/D-10), set by a pricing decorator on
    /// the terminal chunk only.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_core::platform::container::cost::{Cost, CurrencyCode};
    /// use paladin_ports::output::llm_port::{FinishReason, StreamingResponse, TokenUsage};
    ///
    /// let cost = Cost::new(22_500_000, CurrencyCode::new("USD").unwrap());
    /// let chunk = StreamingResponse::terminal(FinishReason::Stop)
    ///     .with_usage(TokenUsage::new(1_000, 2_000))
    ///     .with_cost(cost.clone());
    /// assert_eq!(chunk.cost, Some(cost));
    /// ```
    pub fn with_cost(mut self, cost: Cost) -> Self {
        self.cost = Some(cost);
        self
    }

    /// Chainable: attach the reasoning text the vendor streamed on this chunk (D-11).
    ///
    /// Text that is empty after trimming is stored as `None`, so a vendor's empty reasoning
    /// field never becomes `Some("")`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_ports::output::llm_port::StreamingResponse;
    ///
    /// let chunk = StreamingResponse::delta("").with_reasoning("thinking...");
    /// assert_eq!(chunk.reasoning.as_deref(), Some("thinking..."));
    ///
    /// let blank = StreamingResponse::delta("").with_reasoning("  ");
    /// assert!(blank.reasoning.is_none());
    /// ```
    #[must_use]
    pub fn with_reasoning(mut self, text: impl Into<String>) -> Self {
        let text = text.into();
        self.reasoning = if text.trim().is_empty() {
            None
        } else {
            Some(text)
        };
        self
    }

    /// Chainable: attach one `paladin.*` visibility entry to this chunk (Phase 50, ADR-0061
    /// (h)). Adapters call it on the terminal chunk only; a later entry for the same key
    /// replaces the earlier one.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_ports::output::llm_port::{FinishReason, StreamingResponse};
    ///
    /// let chunk = StreamingResponse::terminal(FinishReason::Stop)
    ///     .with_metadata("paladin.params_omitted", "temperature");
    /// assert_eq!(
    ///     chunk.metadata.get("paladin.params_omitted").map(String::as_str),
    ///     Some("temperature")
    /// );
    ///
    /// // A content chunk carries none.
    /// assert!(StreamingResponse::delta("Hello").metadata.is_empty());
    /// ```
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
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
/// let request = LlmRequest::new(
///     "gpt-4",
///     PromptItem::new(PromptType::User(UserPrompt {
///         query: "Hello, world!".to_string(),
///         context: None,
///     })).unwrap(),
/// );
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
    /// - [`LlmError::RateLimitExceeded`] - Too many requests (wait [`LlmError::retry_after`], then retry)
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
    ///
    /// async fn generate_text(llm: &dyn LlmPort) -> Result<String, Box<dyn std::error::Error>> {
    ///     let request = LlmRequest::new(
    ///         "gpt-4",
    ///         PromptItem::new(PromptType::User(UserPrompt {
    ///             query: "Write a haiku about Rust".to_string(),
    ///             context: None,
    ///         })).unwrap(),
    ///     );
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
    ///
    /// async fn generate_with_retry(llm: &dyn LlmPort) -> Result<String, LlmError> {
    ///     let request = LlmRequest::new(
    ///         "gpt-4",
    ///         PromptItem::new(PromptType::User(UserPrompt {
    ///             query: "Explain async Rust".to_string(),
    ///             context: None,
    ///         })).unwrap(),
    ///     );
    ///
    ///     match llm.generate(request).await {
    ///         Ok(response) => Ok(response.content),
    ///         Err(error @ LlmError::RateLimitExceeded { .. }) => {
    ///             // Wait at least as long as the provider asked, then retry.
    ///             let delay = error.retry_after().unwrap_or(tokio::time::Duration::from_secs(5));
    ///             tokio::time::sleep(delay).await;
    ///             // Retry logic here
    ///             Err(error)
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
    /// use futures::StreamExt;
    ///
    /// async fn stream_response(llm: &dyn LlmPort) -> Result<String, Box<dyn std::error::Error>> {
    ///     let request = LlmRequest::new(
    ///         "gpt-4",
    ///         PromptItem::new(PromptType::User(UserPrompt {
    ///             query: "Write a story".to_string(),
    ///             context: None,
    ///         })).unwrap(),
    ///     ).with_stream(true);
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

    /// Per-model parameter support and limits (ADR-0061, amending ADR-0004).
    ///
    /// Parameter support varies per model, not per adapter, so this reports what one `model`
    /// accepts. The adapter-wide [`get_capabilities`](Self::get_capabilities) is unchanged.
    ///
    /// # Returns
    ///
    /// - `Some(profile)` -- the model's parameter support and limits. An explicit sampling
    ///   parameter the profile does not admit is refused at build time, by name; an auto-selected
    ///   one is omitted from the wire with a `warn!` and a `paladin.params_omitted` metadata
    ///   entry.
    /// - `None` -- no profile (the default). The builder falls back to
    ///   [`ProviderCapabilities::temperature_range`], exactly as before this method existed.
    ///
    /// # Decorators
    ///
    /// A decorator that wraps another `LlmPort` **must forward** this method to its inner port.
    /// Keeping the default `None` silently disables the policy for everything behind it.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_ports::output::llm_port::{ModelProfile, ParamSupport};
    ///
    /// // What an implementor returns for a model that accepts only `temperature = 1.0`.
    /// let profile = Some(ModelProfile::new().with_temperature(ParamSupport::Fixed(1.0)));
    /// assert!(profile.is_some_and(|p| p.temperature.admits(1.0)));
    /// ```
    fn model_profile(&self, _model: &str) -> Option<ModelProfile> {
        None
    }

    /// Judge, before any call, whether a reasoning `effort` (with its `fallback`) would be served
    /// for `model` (Phase 51, D-09).
    ///
    /// This is the verdict a build-time or chain-level gate asks for. The `effort` and `fallback`
    /// are the values a request or an agent carries; the port resolves them against **its own**
    /// provider-level defaults (`llm.<provider>.reasoning_effort` / `reasoning_fallback`) exactly
    /// as [`generate`](Self::generate) would -- a request-level value first, field by field -- and
    /// admits the result against the model's profile. `Ok(())` means `generate` would not refuse
    /// the effort; no network call is made and nothing is logged or recorded.
    ///
    /// The one rule not judged is the per-request check that a reasoning budget stays below
    /// `max_tokens`: it needs the request's own cap, which a gate that has not built the request
    /// does not know. The adapter stays the authority for that rule at send time.
    ///
    /// # Default
    ///
    /// The default knows no provider-level defaults, so it judges `effort` and `fallback` alone:
    /// `effort: None` is `Ok(())` without reading the profile; otherwise the effort is admitted
    /// against [`model_profile`](Self::model_profile) with `fallback` (`Refuse` when `None`). A
    /// port with no profile refuses an explicit effort (nothing was measured, D-03). A third-party
    /// port therefore behaves as it did before this method existed.
    ///
    /// # Decorators
    ///
    /// A decorator that wraps another `LlmPort` **must forward** this method to its inner port.
    /// Keeping the default silently drops the wrapped adapter's provider-level defaults from every
    /// pre-judging gate: a configured `reasoning_fallback` would again be refused at build time
    /// with a hint to set what is already set.
    ///
    /// # Errors
    ///
    /// [`LlmError::UnsupportedParameter`] with `param == "reasoning_effort"`, carrying the same
    /// reason text `generate` would carry. It is `Permanent`.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_ports::output::llm_port::{LlmError, LlmPort, ReasoningEffort};
    ///
    /// /// Whether `port` would serve a `high` effort on `model`.
    /// fn serves_high(port: &dyn LlmPort, model: &str) -> Result<bool, LlmError> {
    ///     match port.check_reasoning(model, Some(ReasoningEffort::High), None) {
    ///         Ok(()) => Ok(true),
    ///         Err(LlmError::UnsupportedParameter { .. }) => Ok(false),
    ///         Err(other) => Err(other),
    ///     }
    /// }
    /// # let _ = serves_high;
    /// ```
    fn check_reasoning(
        &self,
        model: &str,
        effort: Option<ReasoningEffort>,
        fallback: Option<ReasoningFallback>,
    ) -> Result<(), LlmError> {
        let Some(effort) = effort else {
            return Ok(());
        };
        let fallback = fallback.unwrap_or_default();
        let refuse = |reason: String| {
            LlmError::unsupported_parameter(
                "reasoning_effort",
                effort.to_string(),
                self.get_provider_name(),
                model,
                reason,
            )
        };
        let Some(profile) = self.model_profile(model) else {
            return Err(refuse(
                "it reports no model profile, so an explicit reasoning effort cannot be checked"
                    .to_string(),
            ));
        };
        profile
            .effort
            .resolve(effort, fallback)
            .map(|_| ())
            .map_err(|refusal| {
                let mut reason = refusal.to_string();
                if fallback == ReasoningFallback::Refuse
                    && matches!(refusal, EffortRefusal::LevelUnsupported)
                {
                    reason.push_str(REASONING_FALLBACK_HINT);
                }
                refuse(reason)
            })
    }
}

/// The hint appended to a refusal when the fallback in force is `Refuse` and the model simply
/// lacks the level: the one way out. Kept in step with `paladin_llm::reasoning::FALLBACK_HINT`.
const REASONING_FALLBACK_HINT: &str = "; set a reasoning_fallback to substitute the nearest level";

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::prompt::{PromptType, UserPrompt};

    /// D-28 / RT-FR-17: `LlmRequest::new` sets documented defaults for every
    /// field except `model` and `prompt`.
    #[test]
    fn llm_request_new_sets_documented_defaults() {
        let prompt = PromptItem::new(PromptType::User(UserPrompt {
            query: "hi".to_string(),
            context: None,
        }))
        .unwrap();
        let request = LlmRequest::new("gpt-4", prompt);

        assert_eq!(request.model, "gpt-4");
        assert!(request.attachments.is_empty());
        assert!(!request.stream);
        assert!(request.metadata.is_empty());
        assert!(request.response_format.is_none());
        assert!(request.reasoning.is_none());
        assert!(request.reasoning_fallback.is_none());
    }

    fn user_prompt(query: &str) -> PromptItem {
        PromptItem::new(PromptType::User(UserPrompt {
            query: query.to_string(),
            context: None,
        }))
        .unwrap()
    }

    /// D-00e: an unset request serialises with neither effort key, so its JSON is unchanged.
    #[test]
    fn unset_reasoning_is_absent_from_the_serialised_request() {
        let request = LlmRequest::new("gpt-4", user_prompt("hi"));
        let json = serde_json::to_string(&request).unwrap();
        assert!(!json.contains("reasoning"), "{json}");
        let back: LlmRequest = serde_json::from_str(&json).unwrap();
        assert!(back.reasoning.is_none() && back.reasoning_fallback.is_none());
    }

    /// D-00e: a set effort and fallback survive a serde round trip and are set by the builders.
    #[test]
    fn with_reasoning_and_fallback_set_and_round_trip() {
        let request = LlmRequest::new("gpt-4", user_prompt("hi"))
            .with_reasoning(ReasoningEffort::High)
            .with_reasoning_fallback(ReasoningFallback::NearestHigher);
        assert_eq!(request.reasoning, Some(ReasoningEffort::High));
        assert_eq!(
            request.reasoning_fallback,
            Some(ReasoningFallback::NearestHigher)
        );
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"reasoning\":\"high\""), "{json}");
        assert!(
            json.contains("\"reasoning_fallback\":\"nearest_higher\""),
            "{json}"
        );
        let back: LlmRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.reasoning, Some(ReasoningEffort::High));
    }

    /// REASON-01: `Auto` is a set value, never treated as unset.
    #[test]
    fn auto_is_serialised_as_a_set_value() {
        let request =
            LlmRequest::new("gpt-4", user_prompt("hi")).with_reasoning(ReasoningEffort::Auto);
        assert!(
            serde_json::to_string(&request)
                .unwrap()
                .contains("\"reasoning\":\"auto\"")
        );
    }

    /// D-00f / REASON-02: the refusal names the parameter, value, provider, model and reason, is
    /// Permanent, carries no bill, and never renders request content (T-51-03).
    #[test]
    fn unsupported_parameter_names_its_five_fields_and_nothing_else() {
        let error = LlmError::unsupported_parameter(
            "reasoning_effort",
            "high",
            "deepseek",
            "deepseek-v4-pro",
            "no measured effort data",
        );
        let rendered = error.to_string();
        for needle in [
            "reasoning_effort",
            "high",
            "deepseek",
            "deepseek-v4-pro",
            "no measured effort data",
        ] {
            assert!(
                rendered.contains(needle),
                "{needle} missing from {rendered}"
            );
        }
        assert_eq!(error.transience(), Transience::Permanent);
        assert!(error.billed_usage().is_none());

        // Building a request carrying a planted prompt never leaks it into the error: the
        // constructor takes five strings and nothing else.
        let request = LlmRequest::new("deepseek-v4-pro", user_prompt("PLANTED-SECRET-PROMPT"))
            .with_reasoning(ReasoningEffort::High);
        let refused = LlmError::unsupported_parameter(
            "reasoning_effort",
            request.reasoning.map(|e| e.to_string()).unwrap_or_default(),
            "deepseek",
            request.model.clone(),
            "no measured effort data",
        );
        assert!(!refused.to_string().contains("PLANTED-SECRET-PROMPT"));
        assert!(!format!("{refused:?}").contains("PLANTED-SECRET-PROMPT"));
    }

    /// The shape-proof pattern survives future fields: name the fields, then `..`.
    #[test]
    fn unsupported_parameter_matches_with_a_rest_pattern() {
        let error = LlmError::unsupported_parameter("p", "v", "prov", "mod", "why");
        match &error {
            LlmError::UnsupportedParameter {
                param,
                value,
                provider,
                model,
                reason,
                ..
            } => {
                assert_eq!(
                    (param.as_str(), value.as_str(), provider.as_str()),
                    ("p", "v", "prov")
                );
                assert_eq!((model.as_str(), reason.as_str()), ("mod", "why"));
            }
            other => panic!("expected UnsupportedParameter, got {other:?}"),
        }
    }

    /// D-28: two `new` calls with identical arguments produce different ids.
    #[test]
    fn llm_request_new_generates_a_fresh_id_each_call() {
        let prompt_a = PromptItem::new(PromptType::User(UserPrompt {
            query: "a".to_string(),
            context: None,
        }))
        .unwrap();
        let prompt_b = PromptItem::new(PromptType::User(UserPrompt {
            query: "b".to_string(),
            context: None,
        }))
        .unwrap();
        let request_a = LlmRequest::new("gpt-4", prompt_a);
        let request_b = LlmRequest::new("gpt-4", prompt_b);

        assert_ne!(request_a.id, request_b.id);
    }

    /// D-28: chaining the same `with_*` setter twice replaces rather than
    /// accumulates -- last write wins.
    #[test]
    fn builder_methods_chain_and_last_write_wins() {
        let prompt = PromptItem::new(PromptType::User(UserPrompt {
            query: "hi".to_string(),
            context: None,
        }))
        .unwrap();
        let request = LlmRequest::new("gpt-4", prompt)
            .with_stream(true)
            .with_response_format(ResponseFormat::JsonObject)
            .with_stream(false);

        assert!(!request.stream);
        assert_eq!(request.response_format, Some(ResponseFormat::JsonObject));
    }

    /// D-28: a document with no `response_format` key deserializes with
    /// `response_format: None` (`#[serde(default)]`); a json-schema-shaped
    /// value round-trips unchanged.
    #[test]
    fn response_format_round_trips_through_serde_and_defaults_to_none() {
        let prompt = PromptItem::new(PromptType::User(UserPrompt {
            query: "hi".to_string(),
            context: None,
        }))
        .unwrap();
        let request = LlmRequest::new("gpt-4", prompt);
        let mut json = serde_json::to_value(&request).unwrap();
        json.as_object_mut().unwrap().remove("response_format");
        let deserialized: LlmRequest = serde_json::from_value(json).unwrap();
        assert!(deserialized.response_format.is_none());

        let format = ResponseFormat::JsonSchema {
            name: "answer".to_string(),
            schema: serde_json::json!({"type": "object"}),
            strict: true,
        };
        let json = serde_json::to_value(&format).unwrap();
        let round_tripped: ResponseFormat = serde_json::from_value(json).unwrap();
        assert_eq!(format, round_tripped);
    }

    /// D-28: both `ResponseFormat` variants survive a serde round trip with
    /// equality.
    #[test]
    fn response_format_carries_both_variants() {
        let json_object = ResponseFormat::JsonObject;
        let json = serde_json::to_value(&json_object).unwrap();
        let round_tripped: ResponseFormat = serde_json::from_value(json).unwrap();
        assert_eq!(json_object, round_tripped);

        let json_schema = ResponseFormat::JsonSchema {
            name: "answer".to_string(),
            schema: serde_json::json!({"type": "object", "properties": {}}),
            strict: false,
        };
        let json = serde_json::to_value(&json_schema).unwrap();
        let round_tripped: ResponseFormat = serde_json::from_value(json).unwrap();
        assert_eq!(json_schema, round_tripped);
    }

    /// A minimal implementor that does not override `model_profile`.
    struct NoProfilePort;

    #[async_trait]
    impl LlmPort for NoProfilePort {
        async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
            Err(LlmError::ProcessingError("unused".to_string()))
        }

        async fn generate_stream(
            &self,
            _request: LlmRequest,
        ) -> Result<
            Box<dyn futures::Stream<Item = Result<StreamingResponse, LlmError>> + Send>,
            LlmError,
        > {
            Err(LlmError::ProcessingError("unused".to_string()))
        }

        async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
            Ok(true)
        }

        async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
            Ok(Vec::new())
        }

        fn get_provider_name(&self) -> &'static str {
            "no-profile"
        }

        fn get_capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities::default()
        }
    }

    /// ADR-0061 / D-03: an adapter that does not override `model_profile` reports no profile, so
    /// the builder stays on `temperature_range` exactly as before; every existing implementor
    /// compiles unchanged.
    #[test]
    fn model_profile_defaults_to_none() {
        assert!(NoProfilePort.model_profile("any-model").is_none());
        assert!(NoProfilePort.model_profile("").is_none());
    }

    /// A port that reports one profile for every model and keeps the default `check_reasoning`.
    struct ProfiledPort(ModelProfile);

    #[async_trait]
    impl LlmPort for ProfiledPort {
        async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
            Err(LlmError::ProcessingError("unused".to_string()))
        }

        async fn generate_stream(
            &self,
            _request: LlmRequest,
        ) -> Result<
            Box<dyn futures::Stream<Item = Result<StreamingResponse, LlmError>> + Send>,
            LlmError,
        > {
            Err(LlmError::ProcessingError("unused".to_string()))
        }

        async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
            Ok(true)
        }

        async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
            Ok(Vec::new())
        }

        fn get_provider_name(&self) -> &'static str {
            "profiled"
        }

        fn get_capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities::default()
        }

        fn model_profile(&self, _model: &str) -> Option<ModelProfile> {
            Some(self.0.clone())
        }
    }

    /// D-09: the default `check_reasoning` is the builder's pre-gap rule -- the profile and the
    /// given fallback, no provider defaults.
    #[test]
    fn check_reasoning_default_judges_the_profile_alone() {
        let port = ProfiledPort(ModelProfile::new().with_effort(EffortSupport::Supported(
            EffortLevels::new([ReasoningEffort::Low, ReasoningEffort::High]),
        )));

        let error = port
            .check_reasoning("m", Some(ReasoningEffort::Medium), None)
            .unwrap_err();
        let LlmError::UnsupportedParameter {
            param,
            value,
            provider,
            model,
            reason,
            ..
        } = &error
        else {
            panic!("expected UnsupportedParameter, got {error:?}");
        };
        assert_eq!(param, "reasoning_effort");
        assert_eq!(value, "medium");
        assert_eq!(provider, "profiled");
        assert_eq!(model, "m");
        assert!(
            reason.contains("does not offer that effort level"),
            "{reason}"
        );
        assert!(
            reason.ends_with("; set a reasoning_fallback to substitute the nearest level"),
            "{reason}"
        );

        // An explicit Refuse still gets the hint; a substituting fallback is admitted.
        assert!(
            port.check_reasoning(
                "m",
                Some(ReasoningEffort::Medium),
                Some(ReasoningFallback::Refuse)
            )
            .is_err()
        );
        assert!(
            port.check_reasoning(
                "m",
                Some(ReasoningEffort::Medium),
                Some(ReasoningFallback::NearestLower)
            )
            .is_ok()
        );
        // A listed level is admitted; no effort is admitted without judging.
        assert!(
            port.check_reasoning("m", Some(ReasoningEffort::High), None)
                .is_ok()
        );
        assert!(port.check_reasoning("m", None, None).is_ok());

        // A refusal that is not "the level is missing" carries no fallback hint.
        let off = port
            .check_reasoning("m", Some(ReasoningEffort::Off), None)
            .unwrap_err();
        assert!(
            !off.to_string().contains("set a reasoning_fallback"),
            "{off}"
        );
    }

    /// D-03: with no profile an explicit effort cannot be checked and is refused; no effort is
    /// still fine.
    #[test]
    fn check_reasoning_default_refuses_an_explicit_effort_without_a_profile() {
        let error = NoProfilePort
            .check_reasoning("m", Some(ReasoningEffort::High), None)
            .unwrap_err();
        let LlmError::UnsupportedParameter { reason, .. } = &error else {
            panic!("expected UnsupportedParameter, got {error:?}");
        };
        assert_eq!(
            reason,
            "it reports no model profile, so an explicit reasoning effort cannot be checked"
        );
        assert!(NoProfilePort.check_reasoning("m", None, None).is_ok());
        assert!(
            NoProfilePort
                .check_reasoning("m", None, Some(ReasoningFallback::NearestLower))
                .is_ok()
        );
    }

    fn sample_llm_response() -> LlmResponse {
        LlmResponse {
            id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            model: "gpt-4".to_string(),
            content: "hello".to_string(),
            finish_reason: FinishReason::Stop,
            usage: TokenUsage::new(1, 2),
            cost: None,
            reasoning: None,
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: None,
        }
    }

    /// D-10/D-00c: a `LlmResponse` JSON document persisted before `cost` existed (no `cost`
    /// key at all) deserializes with `cost == None` (`#[serde(default)]`), and a `cost: None`
    /// response serializes with no `cost` key at all (`skip_serializing_if`) -- so legacy
    /// documents stay byte-identical.
    #[test]
    fn llm_response_without_cost_key_deserializes_to_none() {
        let response = sample_llm_response();
        assert!(response.cost.is_none());

        let json = serde_json::to_value(&response).unwrap();
        assert!(
            !json.as_object().unwrap().contains_key("cost"),
            "a None cost must be absent from serialized JSON, not present as null"
        );

        let deserialized: LlmResponse = serde_json::from_value(json).unwrap();
        assert!(deserialized.cost.is_none());
    }

    /// D-11: the streaming delta carries typed reasoning text, chainable like `with_usage`.
    #[test]
    fn streaming_with_reasoning_stores_the_text() {
        let chunk = StreamingResponse::delta("").with_reasoning("thinking...");
        assert_eq!(chunk.reasoning.as_deref(), Some("thinking..."));
        assert!(chunk.delta.is_empty());
    }

    /// Edge PARITY-05/empty (carrier half): an empty or whitespace-only reasoning fragment is
    /// `None`, never `Some("")`.
    #[test]
    fn streaming_with_reasoning_ignores_empty_text() {
        assert!(
            StreamingResponse::delta("x")
                .with_reasoning("")
                .reasoning
                .is_none()
        );
        assert!(
            StreamingResponse::delta("x")
                .with_reasoning("  \n\t ")
                .reasoning
                .is_none()
        );
    }

    /// The constructors leave `reasoning` unset.
    #[test]
    fn streaming_constructors_have_no_reasoning() {
        assert!(StreamingResponse::delta("a").reasoning.is_none());
        assert!(
            StreamingResponse::terminal(FinishReason::Stop)
                .reasoning
                .is_none()
        );
    }

    /// Runtime State Inventory: a `StreamingResponse` JSON document without a `reasoning` key
    /// deserializes with `None`, and a `None` is skipped on serialize.
    #[test]
    fn streaming_without_reasoning_key_deserializes_to_none() {
        let chunk = StreamingResponse::delta("hi");
        let json = serde_json::to_value(&chunk).unwrap();
        assert!(
            !json.as_object().unwrap().contains_key("reasoning"),
            "a None reasoning must be absent from serialized JSON, not present as null"
        );
        let back: StreamingResponse = serde_json::from_value(json).unwrap();
        assert!(back.reasoning.is_none());

        let with = StreamingResponse::delta("").with_reasoning("because");
        let json = serde_json::to_value(&with).unwrap();
        assert_eq!(json["reasoning"], "because");
        let back: StreamingResponse = serde_json::from_value(json).unwrap();
        assert_eq!(back.reasoning.as_deref(), Some("because"));
    }

    /// Phase 50, owner reading R-1 (option-b): the streaming terminal chunk carries the
    /// `paladin.*` visibility keys, attached with the chainable `with_metadata`.
    #[test]
    fn streaming_with_metadata_stores_the_entries() {
        let chunk = StreamingResponse::terminal(FinishReason::Stop)
            .with_metadata("paladin.params_omitted", "temperature")
            .with_metadata("paladin.unprofiled_model", "m1");
        assert_eq!(chunk.metadata.len(), 2);
        assert_eq!(
            chunk
                .metadata
                .get("paladin.params_omitted")
                .map(String::as_str),
            Some("temperature")
        );
        assert_eq!(
            chunk
                .metadata
                .get("paladin.unprofiled_model")
                .map(String::as_str),
            Some("m1")
        );
    }

    /// A later entry for the same key replaces the earlier one.
    #[test]
    fn streaming_with_metadata_replaces_a_repeated_key() {
        let chunk = StreamingResponse::terminal(FinishReason::Stop)
            .with_metadata("paladin.served_model", "a")
            .with_metadata("paladin.served_model", "b");
        assert_eq!(chunk.metadata.len(), 1);
        assert_eq!(
            chunk
                .metadata
                .get("paladin.served_model")
                .map(String::as_str),
            Some("b")
        );
    }

    /// The constructors leave `metadata` empty (content chunks carry none).
    #[test]
    fn streaming_constructors_have_empty_metadata() {
        assert!(StreamingResponse::delta("a").metadata.is_empty());
        assert!(
            StreamingResponse::terminal(FinishReason::Stop)
                .metadata
                .is_empty()
        );
    }

    /// An empty `metadata` map is absent from serialized JSON (byte-identical to before the
    /// field existed), a document without the key deserializes to an empty map, and a
    /// populated map round-trips.
    #[test]
    fn streaming_without_metadata_key_deserializes_to_empty() {
        let chunk = StreamingResponse::delta("hi");
        let json = serde_json::to_value(&chunk).unwrap();
        assert!(
            !json.as_object().unwrap().contains_key("metadata"),
            "an empty metadata map must be absent from serialized JSON"
        );
        let back: StreamingResponse = serde_json::from_value(json).unwrap();
        assert!(back.metadata.is_empty());

        let with = StreamingResponse::terminal(FinishReason::Stop)
            .with_metadata("paladin.params_omitted", "stop");
        let json = serde_json::to_value(&with).unwrap();
        assert_eq!(json["metadata"]["paladin.params_omitted"], "stop");
        let back: StreamingResponse = serde_json::from_value(json).unwrap();
        assert_eq!(
            back.metadata
                .get("paladin.params_omitted")
                .map(String::as_str),
            Some("stop")
        );
    }

    /// RESEARCH Open Question 1: `LlmResponse::new` sets the five given fields and defaults the
    /// rest (fresh id, no cost, empty metadata, no function call).
    #[test]
    fn llm_response_new_sets_the_given_fields_and_defaults_the_rest() {
        let request_id = Uuid::new_v4();
        let before = Utc::now();
        let a = LlmResponse::new(
            request_id,
            "m",
            "c",
            FinishReason::Stop,
            TokenUsage::new(1, 2),
        );
        let b = LlmResponse::new(
            request_id,
            "m",
            "c",
            FinishReason::Stop,
            TokenUsage::new(1, 2),
        );
        assert_eq!(a.request_id, request_id);
        assert_eq!(a.model, "m");
        assert_eq!(a.content, "c");
        assert!(matches!(a.finish_reason, FinishReason::Stop));
        assert_eq!(a.usage, TokenUsage::new(1, 2));
        assert!(a.cost.is_none());
        assert!(a.metadata.is_empty());
        assert!(a.function_call.is_none());
        assert!(a.created_at >= before);
        assert_ne!(a.id, b.id, "each response gets a fresh id");
    }

    /// Runtime State Inventory (D-11): a `LlmResponse` JSON document persisted before
    /// `reasoning` existed (no `reasoning` key at all) deserializes with `reasoning == None`
    /// (`#[serde(default)]`), and a `reasoning: None` response serializes with no `reasoning` key
    /// (`skip_serializing_if`) -- so legacy documents stay byte-identical (the `cost` precedent).
    #[test]
    fn llm_response_without_reasoning_key_deserializes_to_none() {
        let response = sample_llm_response();
        assert!(response.reasoning.is_none());

        let json = serde_json::to_value(&response).unwrap();
        assert!(
            !json.as_object().unwrap().contains_key("reasoning"),
            "a None reasoning must be absent from serialized JSON, not present as null"
        );

        let deserialized: LlmResponse = serde_json::from_value(json).unwrap();
        assert!(deserialized.reasoning.is_none());
    }

    /// A response that carries reasoning text round-trips through JSON.
    #[test]
    fn llm_response_reasoning_round_trips_through_json() {
        let response = sample_llm_response().with_reasoning("step one, step two");
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["reasoning"], "step one, step two");
        let back: LlmResponse = serde_json::from_value(json).unwrap();
        assert_eq!(back.reasoning.as_deref(), Some("step one, step two"));
    }

    /// Edge PARITY-05/empty (carrier half): `with_reasoning` stores `None` for text that is empty
    /// after trimming, so a vendor's empty reasoning field never becomes `Some("")`.
    #[test]
    fn with_reasoning_ignores_empty_text() {
        assert!(sample_llm_response().with_reasoning("").reasoning.is_none());
        assert!(
            sample_llm_response()
                .with_reasoning(" \n\t ")
                .reasoning
                .is_none()
        );
        assert_eq!(
            sample_llm_response()
                .with_reasoning("kept as given ")
                .reasoning
                .as_deref(),
            Some("kept as given ")
        );
    }

    /// `LlmResponse::new` leaves `reasoning` unset and puts nothing in `metadata` (D-11: no
    /// untyped channel).
    #[test]
    fn llm_response_new_has_no_reasoning() {
        let response = LlmResponse::new(
            Uuid::new_v4(),
            "m",
            "c",
            FinishReason::Stop,
            TokenUsage::new(1, 2),
        );
        assert!(response.reasoning.is_none());
        assert!(!response.metadata.contains_key("reasoning_content"));
    }

    /// D-28: compile-time guard for the prohibition -- `ProviderCapabilities`
    /// is constructed here by exhaustive struct literal, so this test stops
    /// compiling the moment a field is added to the struct.
    #[test]
    fn provider_capabilities_gained_no_field() {
        let _capabilities = ProviderCapabilities {
            supports_streaming: true,
            supports_tool_calling: false,
            supports_function_calling: false,
            supports_vision: false,
            supports_embeddings: false,
            max_context_tokens: None,
            supports_system_messages: true,
            temperature_range: None,
        };
    }

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
            (LlmError::rate_limited(None), Transient),
            (
                LlmError::rate_limited(Some(Duration::from_secs(7))),
                Transient,
            ),
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
            (LlmError::empty_completion("x"), Permanent),
            (LlmError::refusal("anthropic", "m", "d"), Permanent),
            (
                LlmError::unsupported_parameter("reasoning_effort", "high", "p", "m", "r"),
                Permanent,
            ),
            (
                LlmError::empty_completion("x")
                    .with_billed_usage(BilledUsage::new("m", TokenUsage::new(1, 1))),
                Permanent,
            ),
            (
                LlmError::refusal("anthropic", "m", "d")
                    .with_billed_usage(BilledUsage::new("m", TokenUsage::new(1, 1))),
                Permanent,
            ),
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

    /// D-09: a refusal's `Display` names the provider, the model and the
    /// (already redacted and bounded) detail.
    #[test]
    fn refusal_display_names_provider_model_and_detail() {
        let err = LlmError::refusal("openai", "gpt-x", "I can't help with that.");
        let rendered = err.to_string();
        assert!(rendered.contains("openai"), "{rendered}");
        assert!(rendered.contains("gpt-x"), "{rendered}");
        assert!(rendered.contains("I can't help with that."), "{rendered}");
        assert_eq!(err.transience(), Transience::Permanent);
    }

    /// G-50-5a (plan 50-25): the two constructors build exactly today's
    /// values -- same `Display`, same `Permanent` transience -- and the
    /// values match the shape-proof patterns (`EmptyCompletion { .. }`,
    /// `Refusal { .., .. }`) that survive plan 50-26's struct variants.
    #[test]
    fn llm_error_constructors_build_todays_values() {
        let empty = LlmError::empty_completion("x");
        assert_eq!(empty.to_string(), "Empty completion: x");
        assert!(matches!(empty, LlmError::EmptyCompletion { .. }));
        assert_eq!(empty.transience(), Transience::Permanent);

        let refusal = LlmError::refusal("p", "m", "d");
        assert_eq!(
            refusal.to_string(),
            "Refused by provider 'p' (model 'm'): d"
        );
        match &refusal {
            LlmError::Refusal {
                provider,
                model,
                detail,
                ..
            } => {
                assert_eq!(provider, "p");
                assert_eq!(model, "m");
                assert_eq!(detail, "d");
            }
            other => panic!("expected a Refusal, got {other:?}"),
        }
        assert_eq!(refusal.transience(), Transience::Permanent);
    }

    /// G-50-5a (plan 50-26): `BilledUsage::new(..).with_cost(..)` round-trips
    /// its model, usage and cost; `new` leaves the cost unset.
    #[test]
    fn billed_usage_new_and_with_cost_round_trip() {
        use paladin_core::platform::container::cost::CurrencyCode;

        let usage = TokenUsage::new(1_000, 62_000);
        let unpriced = BilledUsage::new("deepseek-flash", usage.clone());
        assert_eq!(unpriced.model, "deepseek-flash");
        assert_eq!(unpriced.usage, usage);
        assert_eq!(unpriced.cost, None);

        let usd = CurrencyCode::new("USD").expect("USD is a valid currency code");
        let cost = Cost::new(4_200, usd);
        let priced = unpriced.with_cost(cost.clone());
        assert_eq!(priced.cost, Some(cost));
        assert_eq!(priced.usage.total_tokens, 63_000);
    }

    /// G-50-5a: `with_billed_usage` attaches the bill to both billable
    /// variants, and `billed_usage()` reads it back; the `Display` text and
    /// `Permanent` transience are unchanged by the bill.
    #[test]
    fn with_billed_usage_sets_the_bill_on_empty_completion_and_refusal() {
        let bill = BilledUsage::new("m", TokenUsage::new(3, 2));

        let empty = LlmError::empty_completion("x").with_billed_usage(bill.clone());
        assert_eq!(empty.billed_usage(), Some(&bill));
        assert_eq!(empty.to_string(), "Empty completion: x");
        assert_eq!(empty.transience(), Transience::Permanent);
        assert!(matches!(empty, LlmError::EmptyCompletion { .. }));

        let refusal = LlmError::refusal("p", "m", "d").with_billed_usage(bill.clone());
        assert_eq!(refusal.billed_usage(), Some(&bill));
        assert_eq!(
            refusal.to_string(),
            "Refused by provider 'p' (model 'm'): d"
        );
        assert_eq!(refusal.transience(), Transience::Permanent);

        // Unbilled values carry no bill: never a fabricated zero (D-00c).
        assert_eq!(LlmError::empty_completion("x").billed_usage(), None);
        assert_eq!(LlmError::refusal("p", "m", "d").billed_usage(), None);
    }

    /// G-50-5a: a fallback chain's bill is its last error's bill, in both
    /// directions (`with_billed_usage` reaches `last`, `billed_usage` reads it).
    #[test]
    fn billed_usage_looks_through_all_providers_failed_to_its_last_error() {
        let bill = BilledUsage::new("m", TokenUsage::new(10, 0));
        for last in [
            LlmError::empty_completion("x"),
            LlmError::refusal("p", "m", "d"),
        ] {
            let chain = LlmError::AllProvidersFailed {
                attempts: vec![("p".into(), "boom".into())],
                last: Box::new(last),
            };
            assert_eq!(chain.billed_usage(), None);
            let chain = chain.with_billed_usage(bill.clone());
            assert_eq!(chain.billed_usage(), Some(&bill));
            match &chain {
                LlmError::AllProvidersFailed { last, .. } => {
                    assert_eq!(last.billed_usage(), Some(&bill));
                }
                other => panic!("expected AllProvidersFailed, got {other:?}"),
            }
        }
    }

    /// G-50-5a (A2-50-26): only the two billable variants carry a bill; any
    /// other variant has none and `with_billed_usage` returns it unchanged.
    #[test]
    fn with_billed_usage_leaves_other_variants_unchanged() {
        let bill = BilledUsage::new("m", TokenUsage::new(1, 1));
        let limited = LlmError::rate_limited(Some(Duration::from_secs(7)));
        assert_eq!(limited.billed_usage(), None);
        let limited = limited.with_billed_usage(bill.clone());
        assert_eq!(limited.billed_usage(), None);
        assert_eq!(limited.retry_after(), Some(Duration::from_secs(7)));
        assert!(matches!(limited, LlmError::RateLimitExceeded { .. }));

        let network = LlmError::NetworkError("reset".into()).with_billed_usage(bill);
        assert_eq!(network.billed_usage(), None);
        assert_eq!(network.to_string(), "Network error: reset");
    }

    /// D-11a: `EmptyCompletion`'s `Display` no longer appends max-tokens
    /// advice for every finish reason; the payload carries its own advice.
    #[test]
    fn empty_completion_display_has_no_fixed_max_tokens_suffix() {
        let stop = LlmError::empty_completion("finish_reason=stop, the model returned no text");
        assert_eq!(
            stop.to_string(),
            "Empty completion: finish_reason=stop, the model returned no text"
        );
        assert!(!stop.to_string().contains("max_tokens"));
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

    // --- PACE-01: typed retry delay on the 429 ---------------------------------

    #[test]
    fn rate_limited_none_has_no_delay_and_no_hints() {
        let err = LlmError::rate_limited(None);
        assert_eq!(err.retry_after(), None);
        assert!(err.rate_limit_hints().is_none());
    }

    #[test]
    fn rate_limited_some_carries_the_delay() {
        let err = LlmError::rate_limited(Some(Duration::from_secs(7)));
        assert_eq!(err.retry_after(), Some(Duration::from_secs(7)));
        assert!(err.rate_limit_hints().is_none());
    }

    #[test]
    fn display_is_byte_identical() {
        assert_eq!(
            LlmError::rate_limited(Some(Duration::from_secs(7))).to_string(),
            "Rate limit exceeded"
        );
        assert_eq!(
            LlmError::rate_limited(None).to_string(),
            "Rate limit exceeded"
        );
    }

    #[test]
    fn rate_limited_is_transient_with_or_without_a_delay() {
        assert_eq!(
            LlmError::rate_limited(None).transience(),
            Transience::Transient
        );
        assert_eq!(
            LlmError::rate_limited(Some(Duration::from_secs(1))).transience(),
            Transience::Transient
        );
    }

    #[test]
    fn all_providers_failed_delegates_retry_after_to_its_last_error() {
        let wrapped = LlmError::AllProvidersFailed {
            attempts: vec![("openai".into(), "429".into())],
            last: Box::new(LlmError::rate_limited(Some(Duration::from_secs(9)))),
        };
        assert_eq!(wrapped.retry_after(), Some(Duration::from_secs(9)));

        let other = LlmError::AllProvidersFailed {
            attempts: vec![("openai".into(), "boom".into())],
            last: Box::new(LlmError::NetworkError("reset".into())),
        };
        assert_eq!(other.retry_after(), None);
        assert_eq!(LlmError::NetworkError("x".into()).retry_after(), None);
    }

    #[test]
    fn rate_limited_with_hints_sets_retry_after_from_effective_retry_after() {
        use crate::output::rate_limit_hints::{
            RateLimitDimension, RateLimitDimensionKind, RateLimitHints, RetryDelaySource,
        };

        let explicit = RateLimitHints::default()
            .with_retry_after(Duration::from_secs(4), RetryDelaySource::RetryAfter);
        let err = LlmError::rate_limited_with_hints(explicit.clone());
        assert_eq!(err.retry_after(), Some(Duration::from_secs(4)));
        assert_eq!(err.rate_limit_hints(), Some(&explicit));

        let derived = RateLimitHints::default().with_dimension(
            RateLimitDimensionKind::Requests,
            RateLimitDimension::new(None, Some(0), Some(Duration::from_secs(12))),
        );
        let err = LlmError::rate_limited_with_hints(derived);
        assert_eq!(err.retry_after(), Some(Duration::from_secs(12)));

        // Hints that name nothing usable give no delay, and an empty parse is not stored.
        let empty = LlmError::rate_limited_with_hints(RateLimitHints::default());
        assert_eq!(empty.retry_after(), None);
        assert!(empty.rate_limit_hints().is_none());
    }
}
