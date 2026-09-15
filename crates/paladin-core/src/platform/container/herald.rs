//! Herald trait for formatting Paladin execution results
//!
//! The Herald system provides a flexible, extensible mechanism for formatting
//! Paladin and Battalion execution results into multiple output formats (JSON,
//! Markdown, tables, etc.). Heralds enable consistent output formatting across
//! CLI tools, APIs, logging systems, and external integrations.
//!
//! # Examples
//!
//! ```rust,ignore
//! use paladin_core::platform::container::herald::Herald;
//! use paladin::infrastructure::adapters::herald::JsonHerald;
//!
//! let herald = JsonHerald::new();
//! let formatted = herald.format_paladin_result(&result)?;
//! println!("{}", formatted);
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// Re-export actual domain types for Herald consumers
pub use crate::platform::container::battalion::BattalionResult;
pub use crate::platform::container::execution_result::PaladinResult;
pub use crate::platform::container::paladin_error::PaladinError;
pub use crate::platform::container::token_usage::TokenUsage;

// Re-export HeraldError for convenience
pub use super::herald_error::HeraldError;

/// Herald trait for formatting Paladin execution results
///
/// This trait defines the contract for all output formatters in the Paladin system.
/// Implementors transform execution results into specific formats (JSON, Markdown, etc.)
/// while preserving all metadata and supporting both complete and streaming output modes.
///
/// # Thread Safety
///
/// All Herald implementations must be `Send + Sync` to support async execution
/// and concurrent access across multiple threads.
///
/// # Output Modes
///
/// Heralds support two output modes:
/// - **Complete Mode**: Format entire results after execution completes
/// - **Streaming Mode**: Format output progressively as it arrives
pub trait Herald: Send + Sync {
    /// Format a complete Paladin execution result
    ///
    /// This method takes a completed `PaladinResult` and formats it according
    /// to the Herald's output format. All metadata (tokens, timing, errors) must
    /// be included in the formatted output.
    ///
    /// # Arguments
    ///
    /// * `result` - The Paladin execution result to format
    ///
    /// # Returns
    ///
    /// Returns formatted output as a `String` or a `HeraldError` if formatting fails.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let formatted = herald.format_paladin_result(&result)?;
    /// println!("{}", formatted);
    /// ```
    fn format_paladin_result(&self, result: &PaladinResult) -> Result<String, HeraldError>;

    /// Format a complete Battalion execution result
    ///
    /// This method formats the aggregated results from a Battalion execution,
    /// including individual Paladin outputs, execution order/parallelism, and
    /// aggregated metadata.
    ///
    /// # Arguments
    ///
    /// * `result` - The Battalion execution result to format
    ///
    /// # Returns
    ///
    /// Returns formatted output as a `String` or a `HeraldError` if formatting fails.
    fn format_battalion_result(&self, result: &BattalionResult) -> Result<String, HeraldError>;

    /// Format a streaming chunk of output
    ///
    /// This method formats partial output as it arrives during streaming execution.
    /// The formatter may buffer chunks or return formatted output immediately,
    /// depending on the format requirements.
    ///
    /// # Arguments
    ///
    /// * `chunk` - A partial output chunk from streaming execution
    ///
    /// # Returns
    ///
    /// Returns `Some(String)` with formatted output if ready, `None` if buffering,
    /// or a `HeraldError` if formatting fails.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// while let Some(chunk) = stream.next().await {
    ///     if let Some(formatted) = herald.format_stream_chunk(&chunk)? {
    ///         print!("{}", formatted);
    ///     }
    /// }
    /// ```
    fn format_stream_chunk(&self, chunk: &StreamChunk) -> Result<Option<String>, HeraldError>;

    /// Finalize streaming output with metadata
    ///
    /// This method is called after streaming completes to append final metadata
    /// (execution time, token counts, etc.) to the formatted output.
    ///
    /// # Arguments
    ///
    /// * `metadata` - Execution metadata from the completed stream
    ///
    /// # Returns
    ///
    /// Returns formatted metadata as a `String` or a `HeraldError` if formatting fails.
    fn finalize_stream(&self, metadata: &ExecutionMetadata) -> Result<String, HeraldError>;

    /// Format an error for display
    ///
    /// This method formats Paladin errors in a way consistent with the Herald's
    /// output format. Unlike other methods, this never returns an error - it
    /// provides a best-effort formatted representation.
    ///
    /// # Arguments
    ///
    /// * `error` - The Paladin error to format
    ///
    /// # Returns
    ///
    /// Returns formatted error as a `String`.
    fn format_error(&self, error: &PaladinError) -> String;

    /// Get the formatter name/identifier
    ///
    /// Returns a unique identifier for this formatter (e.g., "json", "markdown", "table").
    /// Used for formatter registration and selection.
    fn name(&self) -> &str;

    /// Get the formatter's MIME type
    ///
    /// Returns the MIME type for the formatted output (e.g., "application/json",
    /// "text/markdown", "text/plain"). Useful for HTTP responses and content
    /// negotiation.
    fn mime_type(&self) -> &str;
}

/// Streaming chunk of output
///
/// Represents a single chunk of output during streaming execution.
/// Each chunk has a unique ID, sequence number, timestamp, and extensible metadata
/// for tracking, debugging, and custom provider-specific information.
///
/// # Fields
///
/// * `chunk_id` - Unique identifier for this specific chunk (UUID v4)
/// * `sequence_number` - Order in the stream (0-indexed), ensures proper ordering
/// * `timestamp` - When this chunk was generated (UTC timezone)
/// * `content` - The actual content/text of this chunk
/// * `token_count` - Approximate token count in this chunk (if available from provider)
/// * `is_final` - Whether this is the last chunk in the stream (enables cleanup logic)
/// * `metadata` - Extensible HashMap for provider-specific or custom metadata
///
/// # Extensible Metadata
///
/// The `metadata` field allows adding custom information without changing the struct:
/// - Provider-specific data (model confidence, temperature, etc.)
/// - Performance metrics (generation time, API latency)
/// - Debugging information (prompt hash, request ID)
/// - Custom application data
///
/// # Examples
///
/// ## Basic Usage
///
/// ```
/// use paladin_core::platform::container::herald::StreamChunk;
/// use uuid::Uuid;
/// use chrono::Utc;
///
/// let chunk = StreamChunk::builder()
///     .chunk_id(Uuid::new_v4())
///     .sequence_number(0)
///     .timestamp(Utc::now())
///     .content("Hello, world!".to_string())
///     .is_final(false)
///     .build()
///     .unwrap();
/// ```
///
/// ## With Token Count
///
/// ```
/// use paladin_core::platform::container::herald::StreamChunk;
/// use uuid::Uuid;
/// use chrono::Utc;
///
/// let chunk = StreamChunk::builder()
///     .chunk_id(Uuid::new_v4())
///     .sequence_number(5)
///     .timestamp(Utc::now())
///     .content("Processing data...".to_string())
///     .token_count(15)
///     .is_final(false)
///     .build()
///     .unwrap();
/// ```
///
/// ## With Custom Metadata
///
/// ```
/// use paladin_core::platform::container::herald::StreamChunk;
/// use uuid::Uuid;
/// use chrono::Utc;
/// use serde_json::json;
///
/// let chunk = StreamChunk::builder()
///     .chunk_id(Uuid::new_v4())
///     .sequence_number(0)
///     .timestamp(Utc::now())
///     .content("Result: 42".to_string())
///     .token_count(5)
///     .is_final(true)
///     .add_metadata("confidence".to_string(), json!(0.95))
///     .add_metadata("model_temperature".to_string(), json!(0.7))
///     .add_metadata("response_time_ms".to_string(), json!(350))
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    /// Unique identifier for this chunk
    pub chunk_id: Uuid,

    /// Sequence number in the stream (0-indexed)
    pub sequence_number: u64,

    /// Timestamp when chunk was generated
    pub timestamp: DateTime<Utc>,

    /// Content of this chunk
    pub content: String,

    /// Approximate token count in this chunk
    pub token_count: Option<u32>,

    /// Whether this is the final chunk in the stream
    pub is_final: bool,

    /// Extensible metadata for future fields
    #[serde(flatten)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl StreamChunk {
    /// Create a new StreamChunkBuilder
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_core::platform::container::herald::StreamChunk;
    /// use uuid::Uuid;
    /// use chrono::Utc;
    ///
    /// let chunk = StreamChunk::builder()
    ///     .chunk_id(Uuid::new_v4())
    ///     .sequence_number(0)
    ///     .timestamp(Utc::now())
    ///     .content("chunk content".to_string())
    ///     .is_final(false)
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn builder() -> StreamChunkBuilder {
        StreamChunkBuilder::default()
    }
}

/// Builder for StreamChunk
///
/// Provides a fluent interface for constructing StreamChunk instances
/// with validation.
#[derive(Default)]
pub struct StreamChunkBuilder {
    chunk_id: Option<Uuid>,
    sequence_number: Option<u64>,
    timestamp: Option<DateTime<Utc>>,
    content: Option<String>,
    token_count: Option<u32>,
    is_final: Option<bool>,
    metadata: HashMap<String, serde_json::Value>,
}

impl StreamChunkBuilder {
    /// Set the chunk ID
    pub fn chunk_id(mut self, chunk_id: Uuid) -> Self {
        self.chunk_id = Some(chunk_id);
        self
    }

    /// Set the sequence number
    pub fn sequence_number(mut self, sequence_number: u64) -> Self {
        self.sequence_number = Some(sequence_number);
        self
    }

    /// Set the timestamp
    pub fn timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Set the content
    pub fn content(mut self, content: String) -> Self {
        self.content = Some(content);
        self
    }

    /// Set the token count
    pub fn token_count(mut self, token_count: u32) -> Self {
        self.token_count = Some(token_count);
        self
    }

    /// Set whether this is the final chunk
    pub fn is_final(mut self, is_final: bool) -> Self {
        self.is_final = Some(is_final);
        self
    }

    /// Add a metadata entry
    pub fn add_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Build the StreamChunk
    ///
    /// # Errors
    ///
    /// Returns an error if required fields are missing
    pub fn build(self) -> Result<StreamChunk, HeraldError> {
        Ok(StreamChunk {
            chunk_id: self
                .chunk_id
                .ok_or_else(|| HeraldError::InvalidResult("chunk_id is required".to_string()))?,
            sequence_number: self.sequence_number.ok_or_else(|| {
                HeraldError::InvalidResult("sequence_number is required".to_string())
            })?,
            timestamp: self
                .timestamp
                .ok_or_else(|| HeraldError::InvalidResult("timestamp is required".to_string()))?,
            content: self
                .content
                .ok_or_else(|| HeraldError::InvalidResult("content is required".to_string()))?,
            token_count: self.token_count,
            is_final: self
                .is_final
                .ok_or_else(|| HeraldError::InvalidResult("is_final is required".to_string()))?,
            metadata: self.metadata,
        })
    }
}

/// Execution metadata for streaming with complete telemetry
///
/// Tracks comprehensive execution metrics including timing, token usage,
/// costs, and errors. Provides complete observability for Paladin executions
/// with support for extensible custom metadata.
///
/// # Fields
///
/// * `execution_id` - Unique identifier for this execution (UUID v4)
/// * `start_time` - When execution started (UTC timezone)
/// * `end_time` - When execution completed (None if still running)
/// * `duration_ms` - Calculated execution duration in milliseconds
/// * `model_used` - LLM model identifier (e.g., "gpt-4", "claude-3")
/// * `token_usage` - Token consumption statistics (prompt, completion, total)
/// * `cost_estimate` - Reserved for the Treasurer (Milestone 14 / FUT-08); no in-tree producer yet
/// * `error_count` - Number of errors encountered during execution
/// * `metadata` - Extensible HashMap for custom telemetry and provider-specific data
///
/// # Telemetry Use Cases
///
/// - **Performance Monitoring**: Track execution times, identify slow operations
/// - **Cost Tracking**: Monitor token usage and estimated costs per execution
/// - **Error Analysis**: Track error frequency and patterns
/// - **Model Comparison**: Compare performance across different models
/// - **Resource Planning**: Analyze token usage patterns for capacity planning
///
/// # Examples
///
/// ## Basic Usage
///
/// ```
/// use paladin_core::platform::container::herald::ExecutionMetadata;
/// use paladin_core::platform::container::token_usage::TokenUsage;
/// use chrono::Utc;
/// use uuid::Uuid;
///
/// let metadata = ExecutionMetadata::builder()
///     .execution_id(Uuid::new_v4())
///     .start_time(Utc::now())
///     .model_used("gpt-4".to_string())
///     .token_usage(TokenUsage::new(100, 50))
///     .build()
///     .expect("Valid metadata");
/// ```
///
/// ## Complete Execution with Duration Calculation
///
/// ```
/// use paladin_core::platform::container::herald::ExecutionMetadata;
/// use paladin_core::platform::container::token_usage::TokenUsage;
/// use chrono::Utc;
/// use uuid::Uuid;
///
/// let start = Utc::now();
///
/// // ... perform execution ...
///
/// let mut metadata = ExecutionMetadata::builder()
///     .execution_id(Uuid::new_v4())
///     .start_time(start)
///     .end_time(Utc::now())
///     .model_used("gpt-4".to_string())
///     .token_usage(TokenUsage::new(250, 500))
///     .build()
///     .unwrap();
///
/// // Calculate duration from timestamps
/// metadata.calculate_duration();
/// println!("Execution took {} ms", metadata.duration_ms.unwrap());
/// ```
///
/// ## With Cost Estimation and Error Tracking
///
/// ```
/// use paladin_core::platform::container::herald::ExecutionMetadata;
/// use paladin_core::platform::container::token_usage::TokenUsage;
/// use chrono::Utc;
/// use uuid::Uuid;
///
/// let metadata = ExecutionMetadata::builder()
///     .execution_id(Uuid::new_v4())
///     .start_time(Utc::now())
///     .end_time(Utc::now())
///     .duration_ms(2500)
///     .model_used("gpt-4".to_string())
///     .token_usage(TokenUsage::new(1000, 2000))
///     .cost_estimate(0.045)  // illustrative value; reserved for the Treasurer (Milestone 14 / FUT-08)
///     .error_count(2)        // Encountered 2 retryable errors
///     .build()
///     .unwrap();
///
/// if let Some(cost) = metadata.total_cost() {
///     println!("Execution cost: ${:.4}", cost);
/// }
/// ```
///
/// ## With Custom Metadata
///
/// ```
/// use paladin_core::platform::container::herald::ExecutionMetadata;
/// use paladin_core::platform::container::token_usage::TokenUsage;
/// use chrono::Utc;
/// use uuid::Uuid;
/// use serde_json::json;
///
/// let metadata = ExecutionMetadata::builder()
///     .execution_id(Uuid::new_v4())
///     .start_time(Utc::now())
///     .model_used("gpt-4".to_string())
///     .token_usage(TokenUsage::new(150, 300))
///     .add_metadata("user_id".to_string(), json!("user_123"))
///     .add_metadata("request_source".to_string(), json!("api"))
///     .add_metadata("cache_hit".to_string(), json!(false))
///     .add_metadata("model_temperature".to_string(), json!(0.7))
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    /// Unique execution identifier
    pub execution_id: Uuid,
    /// Execution start timestamp
    pub start_time: DateTime<Utc>,
    /// Execution end timestamp (None if still running)
    pub end_time: Option<DateTime<Utc>>,
    /// Calculated duration in milliseconds
    pub duration_ms: Option<u64>,
    /// Model identifier used for execution
    pub model_used: String,
    /// Token usage statistics
    pub token_usage: TokenUsage,
    /// Reserved for the Treasurer (Milestone 14 / FUT-08); no in-tree producer yet.
    pub cost_estimate: Option<f64>,
    /// Number of errors encountered during execution
    pub error_count: u32,
    /// Extensible metadata for custom fields
    #[serde(flatten)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ExecutionMetadata {
    /// Create a new builder for ExecutionMetadata
    pub fn builder() -> ExecutionMetadataBuilder {
        ExecutionMetadataBuilder::default()
    }

    /// Calculate duration from start_time and end_time
    ///
    /// If end_time is set, calculates the duration in milliseconds
    /// and stores it in duration_ms field.
    pub fn calculate_duration(&mut self) {
        if let Some(end) = self.end_time {
            let duration = end.signed_duration_since(self.start_time);
            self.duration_ms = Some(duration.num_milliseconds() as u64);
        }
    }

    /// Get the reserved cost estimate
    ///
    /// Returns the `cost_estimate` field as stored. The field is reserved for the
    /// Treasurer (Milestone 14 / FUT-08); it has no in-tree producer yet, so this
    /// returns `None` in this tree.
    pub fn total_cost(&self) -> Option<f64> {
        self.cost_estimate
    }
}

/// Builder for ExecutionMetadata
#[derive(Default)]
pub struct ExecutionMetadataBuilder {
    execution_id: Option<Uuid>,
    start_time: Option<DateTime<Utc>>,
    end_time: Option<DateTime<Utc>>,
    duration_ms: Option<u64>,
    model_used: Option<String>,
    token_usage: Option<TokenUsage>,
    cost_estimate: Option<f64>,
    error_count: u32,
    metadata: HashMap<String, serde_json::Value>,
}

impl ExecutionMetadataBuilder {
    /// Set the execution ID
    pub fn execution_id(mut self, execution_id: Uuid) -> Self {
        self.execution_id = Some(execution_id);
        self
    }

    /// Set the start time
    pub fn start_time(mut self, start_time: DateTime<Utc>) -> Self {
        self.start_time = Some(start_time);
        self
    }

    /// Set the end time
    pub fn end_time(mut self, end_time: DateTime<Utc>) -> Self {
        self.end_time = Some(end_time);
        self
    }

    /// Set the duration in milliseconds
    pub fn duration_ms(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    /// Set the model used
    pub fn model_used(mut self, model_used: String) -> Self {
        self.model_used = Some(model_used);
        self
    }

    /// Set the token usage
    pub fn token_usage(mut self, token_usage: TokenUsage) -> Self {
        self.token_usage = Some(token_usage);
        self
    }

    /// Set the cost estimate (reserved for the Treasurer, Milestone 14 / FUT-08 — no in-tree producer yet)
    pub fn cost_estimate(mut self, cost_estimate: f64) -> Self {
        self.cost_estimate = Some(cost_estimate);
        self
    }

    /// Set the error count
    pub fn error_count(mut self, error_count: u32) -> Self {
        self.error_count = error_count;
        self
    }

    /// Add a metadata entry
    pub fn add_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Build the ExecutionMetadata
    ///
    /// Returns an error if required fields are missing.
    pub fn build(self) -> Result<ExecutionMetadata, HeraldError> {
        Ok(ExecutionMetadata {
            execution_id: self.execution_id.ok_or_else(|| {
                HeraldError::InvalidResult("execution_id is required".to_string())
            })?,
            start_time: self
                .start_time
                .ok_or_else(|| HeraldError::InvalidResult("start_time is required".to_string()))?,
            end_time: self.end_time,
            duration_ms: self.duration_ms,
            model_used: self
                .model_used
                .ok_or_else(|| HeraldError::InvalidResult("model_used is required".to_string()))?,
            token_usage: self
                .token_usage
                .ok_or_else(|| HeraldError::InvalidResult("token_usage is required".to_string()))?,
            cost_estimate: self.cost_estimate,
            error_count: self.error_count,
            metadata: self.metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock Herald implementation for testing
    struct MockHerald;

    impl Herald for MockHerald {
        fn format_paladin_result(&self, result: &PaladinResult) -> Result<String, HeraldError> {
            Ok(format!("MOCK: {}", result.output))
        }

        fn format_battalion_result(&self, result: &BattalionResult) -> Result<String, HeraldError> {
            Ok(format!("MOCK BATTALION: {}", result.battalion_name))
        }

        fn format_stream_chunk(&self, chunk: &StreamChunk) -> Result<Option<String>, HeraldError> {
            if chunk.is_final {
                Ok(Some(chunk.content.clone()))
            } else {
                Ok(None)
            }
        }

        fn finalize_stream(&self, metadata: &ExecutionMetadata) -> Result<String, HeraldError> {
            Ok(format!(
                "Execution time: {}ms",
                metadata.duration_ms.unwrap_or(0)
            ))
        }

        fn format_error(&self, error: &PaladinError) -> String {
            format!("ERROR: {}", error)
        }

        fn name(&self) -> &str {
            "mock"
        }

        fn mime_type(&self) -> &str {
            "text/plain"
        }
    }

    #[test]
    fn test_herald_trait_object() {
        let herald: Box<dyn Herald> = Box::new(MockHerald);
        assert_eq!(herald.name(), "mock");
        assert_eq!(herald.mime_type(), "text/plain");
    }

    #[test]
    fn test_format_paladin_result() {
        use crate::platform::container::execution_result::StopReason;
        let herald = MockHerald;
        let result = PaladinResult {
            output: "Test output".to_string(),
            token_count: 100,
            execution_time_ms: 1500,
            loop_count: 1,
            stop_reason: StopReason::Completed,
            ..Default::default()
        };

        let formatted = herald.format_paladin_result(&result).unwrap();
        assert_eq!(formatted, "MOCK: Test output");
    }

    #[test]
    fn test_format_battalion_result() {
        use crate::platform::container::battalion::{BattalionStatus, BattalionStrategy};
        use chrono::Utc;
        use uuid::Uuid;
        let herald = MockHerald;
        let result = BattalionResult {
            battalion_id: Uuid::new_v4(),
            battalion_name: "TestBattalion".to_string(),
            started_at: Utc::now(),
            completed_at: Utc::now(),
            final_output: "Combined output".to_string(),
            paladin_results: vec![],
            status: BattalionStatus::Completed,
            strategy_used: BattalionStrategy::Formation,
            strategy_selection_reasoning: None,
            strategy_selection_time_ms: 0,
            per_paladin_times: std::collections::HashMap::new(),
            per_paladin_tokens: std::collections::HashMap::new(),
            total_tokens: 0,
            paladin_success_count: 0,
            paladin_failure_count: 0,
            node_errors: Vec::new(),
        };

        let formatted = herald.format_battalion_result(&result).unwrap();
        assert_eq!(formatted, "MOCK BATTALION: TestBattalion");
    }

    #[test]
    fn test_format_stream_chunk() {
        let herald = MockHerald;
        let chunk = StreamChunk::builder()
            .chunk_id(Uuid::new_v4())
            .sequence_number(0)
            .timestamp(Utc::now())
            .content("partial output".to_string())
            .is_final(false)
            .build()
            .unwrap();

        let result = herald.format_stream_chunk(&chunk).unwrap();
        assert!(result.is_none());

        let final_chunk = StreamChunk::builder()
            .chunk_id(Uuid::new_v4())
            .sequence_number(1)
            .timestamp(Utc::now())
            .content("final output".to_string())
            .is_final(true)
            .build()
            .unwrap();

        let result = herald.format_stream_chunk(&final_chunk).unwrap();
        assert_eq!(result, Some("final output".to_string()));
    }

    #[test]
    fn test_finalize_stream() {
        let herald = MockHerald;
        let metadata = ExecutionMetadata::builder()
            .execution_id(Uuid::new_v4())
            .start_time(Utc::now())
            .model_used("test-model".to_string())
            .token_usage(TokenUsage::new(300, 200))
            .duration_ms(1234)
            .build()
            .unwrap();

        let formatted = herald.finalize_stream(&metadata).unwrap();
        assert_eq!(formatted, "Execution time: 1234ms");
    }

    #[test]
    fn test_format_error() {
        let herald = MockHerald;
        let error = PaladinError::ExecutionError("Something went wrong".to_string());

        let formatted = herald.format_error(&error);
        assert!(formatted.contains("ERROR"));
        assert!(formatted.contains("Something went wrong"));
    }

    #[test]
    fn test_herald_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MockHerald>();
    }
}
