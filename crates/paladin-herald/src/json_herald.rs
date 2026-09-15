//! JSON Herald formatter implementation
//!
//! The JsonHerald provides JSON serialization for Paladin and Battalion execution results.
//! It supports both pretty-printed and compact JSON output, with configurable metadata inclusion.
//!
//! # Examples
//!
//! ```rust,ignore
//! use paladin_herald::JsonHerald;
//! use paladin::core::platform::container::herald::Herald;
//!
//! let herald = JsonHerald::new();
//! let formatted = herald.format_paladin_result(&result)?;
//! println!("{}", formatted);
//! ```

use paladin_core::platform::container::herald::{
    BattalionResult, ExecutionMetadata, Herald, HeraldError, PaladinError, PaladinResult,
    StreamChunk,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Configuration for JSON Herald formatter
#[doc(hidden)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonHeraldConfig {
    /// Enable pretty-printing (formatted with indentation)
    pub pretty: bool,
    /// Include metadata fields in output
    pub include_metadata: bool,
}

impl Default for JsonHeraldConfig {
    fn default() -> Self {
        Self {
            pretty: true,
            include_metadata: true,
        }
    }
}

/// JSON formatter for Paladin execution results
///
/// The JsonHerald converts Paladin and Battalion results into JSON format,
/// making them suitable for API responses, logging systems, and programmatic
/// consumption.
///
/// # Thread Safety
///
/// JsonHerald is thread-safe and can be shared across threads using `Arc`.
///
/// # Configuration
///
/// - **pretty**: When true, outputs formatted JSON with indentation
/// - **include_metadata**: When true, includes execution metadata in output
///
/// # Examples
///
/// ```rust,ignore
/// // Create with default configuration (pretty: true, include_metadata: true)
/// let herald = JsonHerald::new();
///
/// // Create with custom configuration
/// let config = JsonHeraldConfig {
///     pretty: false,
///     include_metadata: true,
/// };
/// let herald = JsonHerald::with_config(config);
/// ```
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct JsonHerald {
    config: JsonHeraldConfig,
}

impl JsonHerald {
    /// Create a new JsonHerald with default configuration
    ///
    /// Default configuration:
    /// - pretty: true (formatted output)
    /// - include_metadata: true (all metadata included)
    pub fn new() -> Self {
        Self {
            config: JsonHeraldConfig::default(),
        }
    }

    /// Create a new JsonHerald with custom configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration for the JSON formatter
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let config = JsonHeraldConfig {
    ///     pretty: false,
    ///     include_metadata: false,
    /// };
    /// let herald = JsonHerald::with_config(config);
    /// ```
    pub fn with_config(config: JsonHeraldConfig) -> Self {
        Self { config }
    }

    /// Serialize value to JSON string based on configuration
    fn serialize<T: Serialize>(&self, value: &T) -> Result<String, HeraldError> {
        if self.config.pretty {
            serde_json::to_string_pretty(value)
        } else {
            serde_json::to_string(value)
        }
        .map_err(|e| HeraldError::SerializationError(format!("JSON serialization failed: {}", e)))
    }

    /// Convert PaladinResult to JSON Value
    fn paladin_result_to_json(&self, result: &PaladinResult) -> Value {
        let mut json = json!({
            "output": result.output,
            "token_count": result.usage.total_tokens,
            "execution_time_ms": result.execution_time_ms,
            "loop_count": result.loop_count,
            "stop_reason": format!("{:?}", result.stop_reason),
        });

        if self.config.include_metadata {
            json["metadata"] = json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });
        }

        json
    }

    /// Convert BattalionResult to JSON Value
    fn battalion_result_to_json(&self, result: &BattalionResult) -> Value {
        let paladin_results: Vec<Value> = result
            .paladin_results
            .iter()
            .map(|r| self.paladin_result_to_json(r))
            .collect();

        let mut json = json!({
            "battalion_id": result.battalion_id,
            "battalion_name": result.battalion_name,
            "status": format!("{:?}", result.status),
            "strategy_used": format!("{:?}", result.strategy_used),
            "paladin_results": paladin_results,
            "total_tokens": result.total_tokens,
            "per_paladin_tokens": result.per_paladin_tokens,
            "node_errors": result.node_errors,
        });

        if self.config.include_metadata {
            json["metadata"] = json!({
                "paladin_count": result.paladin_results.len(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });
        }

        json
    }
}

impl Default for JsonHerald {
    fn default() -> Self {
        Self::new()
    }
}

impl Herald for JsonHerald {
    fn format_paladin_result(&self, result: &PaladinResult) -> Result<String, HeraldError> {
        let json = self.paladin_result_to_json(result);
        self.serialize(&json)
    }

    fn format_battalion_result(&self, result: &BattalionResult) -> Result<String, HeraldError> {
        let json = self.battalion_result_to_json(result);
        self.serialize(&json)
    }

    fn format_stream_chunk(&self, chunk: &StreamChunk) -> Result<Option<String>, HeraldError> {
        // For JSON streaming, we use NDJSON (newline-delimited JSON) format
        // Each chunk is emitted as a separate JSON object on its own line
        let json = json!({
            "content": chunk.content,
            "is_final": chunk.is_final,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        // Use compact serialization for streaming (pretty printing not suitable for NDJSON)
        let serialized = serde_json::to_string(&json).map_err(|e| {
            HeraldError::SerializationError(format!("Stream chunk serialization failed: {}", e))
        })?;

        Ok(Some(format!("{}\n", serialized)))
    }

    fn finalize_stream(&self, metadata: &ExecutionMetadata) -> Result<String, HeraldError> {
        let json = json!({
            "type": "metadata",
            "execution_id": metadata.execution_id,
            "duration_ms": metadata.duration_ms,
            "model_used": metadata.model_used,
            "total_tokens": metadata.token_usage.total_tokens,
            "cost_estimate": metadata.cost_estimate,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        let serialized = serde_json::to_string(&json).map_err(|e| {
            HeraldError::SerializationError(format!("Metadata serialization failed: {}", e))
        })?;

        Ok(format!("{}\n", serialized))
    }

    fn format_error(&self, error: &PaladinError) -> String {
        let json = json!({
            "error": true,
            "message": error.to_string(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        // Best-effort serialization, never fail
        serde_json::to_string_pretty(&json)
            .unwrap_or_else(|_| format!(r#"{{"error": true, "message": "{}"}}"#, error))
    }

    fn name(&self) -> &str {
        "json"
    }

    fn mime_type(&self) -> &str {
        "application/json"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use paladin_core::platform::container::battalion::{
        BattalionStatus, BattalionStrategy, NodeError, TokenUsage,
    };
    use paladin_ports::output::paladin_port::StopReason;
    use uuid::Uuid;

    fn create_test_paladin_result() -> PaladinResult {
        PaladinResult {
            output: "Test output content".to_string(),
            usage: TokenUsage::new(100, 0),
            execution_time_ms: 1500,
            loop_count: 1,
            stop_reason: StopReason::Completed,
            ..Default::default()
        }
    }

    fn create_test_battalion_result() -> BattalionResult {
        BattalionResult {
            battalion_id: Uuid::new_v4(),
            battalion_name: "TestBattalion".to_string(),
            started_at: Utc::now(),
            completed_at: Utc::now(),
            final_output: "Combined output".to_string(),
            paladin_results: vec![
                create_test_paladin_result(),
                PaladinResult {
                    output: "Second output".to_string(),
                    usage: TokenUsage::new(150, 0),
                    execution_time_ms: 2000,
                    loop_count: 2,
                    stop_reason: StopReason::Completed,
                    ..Default::default()
                },
            ],
            status: BattalionStatus::Completed,
            strategy_used:
                paladin_core::platform::container::battalion::BattalionStrategy::Formation,
            strategy_selection_reasoning: None,
            strategy_selection_time_ms: 0,
            per_paladin_times: [
                ("paladin_1".to_string(), 1500u64),
                ("paladin_2".to_string(), 2000u64),
            ]
            .into_iter()
            .collect(),
            per_paladin_tokens: std::collections::HashMap::new(),
            total_tokens: 0,
            paladin_success_count: 2,
            paladin_failure_count: 0,
            node_errors: Vec::new(),
        }
    }

    #[test]
    fn test_new_creates_default_config() {
        let herald = JsonHerald::new();
        assert!(herald.config.pretty);
        assert!(herald.config.include_metadata);
    }

    #[test]
    fn test_with_config_uses_custom_config() {
        let config = JsonHeraldConfig {
            pretty: false,
            include_metadata: false,
        };
        let herald = JsonHerald::with_config(config);
        assert!(!herald.config.pretty);
        assert!(!herald.config.include_metadata);
    }

    #[test]
    fn test_format_paladin_result_success() {
        let herald = JsonHerald::new();
        let result = create_test_paladin_result();

        let formatted = herald.format_paladin_result(&result).unwrap();

        // Verify it's valid JSON
        let parsed: Value = serde_json::from_str(&formatted).unwrap();
        assert_eq!(parsed["output"], "Test output content");
        assert_eq!(parsed["token_count"], 100);
        assert_eq!(parsed["execution_time_ms"], 1500);
        assert_eq!(parsed["loop_count"], 1);
        assert_eq!(parsed["stop_reason"], "Completed");
    }

    #[test]
    fn test_format_paladin_result_includes_metadata() {
        let herald = JsonHerald::new();
        let result = create_test_paladin_result();

        let formatted = herald.format_paladin_result(&result).unwrap();
        let parsed: Value = serde_json::from_str(&formatted).unwrap();

        assert!(parsed["metadata"].is_object());
        assert!(parsed["metadata"]["timestamp"].is_string());
    }

    #[test]
    fn test_format_paladin_result_without_metadata() {
        let config = JsonHeraldConfig {
            pretty: false,
            include_metadata: false,
        };
        let herald = JsonHerald::with_config(config);
        let result = create_test_paladin_result();

        let formatted = herald.format_paladin_result(&result).unwrap();
        let parsed: Value = serde_json::from_str(&formatted).unwrap();

        assert!(parsed["metadata"].is_null());
    }

    #[test]
    fn test_format_battalion_result_success() {
        let herald = JsonHerald::new();
        let result = create_test_battalion_result();

        let formatted = herald.format_battalion_result(&result).unwrap();

        let parsed: Value = serde_json::from_str(&formatted).unwrap();
        assert!(parsed["battalion_id"].is_string());
        assert_eq!(parsed["battalion_name"], "TestBattalion");
        assert_eq!(parsed["status"], "Completed");
        assert!(parsed["paladin_results"].is_array());
        assert_eq!(parsed["paladin_results"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_format_battalion_result_includes_metadata() {
        let herald = JsonHerald::new();
        let result = create_test_battalion_result();

        let formatted = herald.format_battalion_result(&result).unwrap();
        let parsed: Value = serde_json::from_str(&formatted).unwrap();

        assert!(parsed["metadata"].is_object());
        assert_eq!(parsed["metadata"]["paladin_count"], 2);
    }

    #[test]
    fn test_json_herald_battalion_includes_strategy_and_total_tokens() {
        let herald = JsonHerald::new();

        let mut per_paladin_tokens = std::collections::HashMap::new();
        per_paladin_tokens.insert("Scout".to_string(), TokenUsage::from_total(137));
        per_paladin_tokens.insert("Sentinel".to_string(), TokenUsage::from_total(263));

        let result = BattalionResult {
            battalion_id: Uuid::new_v4(),
            battalion_name: "TokenBattalion".to_string(),
            started_at: Utc::now(),
            completed_at: Utc::now(),
            final_output: "Combined output".to_string(),
            paladin_results: vec![
                PaladinResult {
                    output: "Scout output".to_string(),
                    usage: TokenUsage::new(137, 0),
                    execution_time_ms: 1500,
                    loop_count: 1,
                    stop_reason: StopReason::Completed,
                    ..Default::default()
                },
                PaladinResult {
                    output: "Sentinel output".to_string(),
                    usage: TokenUsage::new(263, 0),
                    execution_time_ms: 2000,
                    loop_count: 2,
                    stop_reason: StopReason::Completed,
                    ..Default::default()
                },
            ],
            status: BattalionStatus::Completed,
            strategy_used: BattalionStrategy::Phalanx,
            strategy_selection_reasoning: None,
            strategy_selection_time_ms: 0,
            per_paladin_times: std::collections::HashMap::new(),
            per_paladin_tokens,
            total_tokens: 400,
            paladin_success_count: 2,
            paladin_failure_count: 1,
            node_errors: vec![NodeError {
                node_name: "Herald".to_string(),
                error: "connection refused".to_string(),
            }],
        };

        let formatted = herald.format_battalion_result(&result).unwrap();
        let parsed: Value = serde_json::from_str(&formatted).unwrap();

        assert_eq!(parsed["strategy_used"], "Phalanx");
        // The aggregate equals the sum of the two distinct, non-round token counts.
        assert_eq!(parsed["total_tokens"], 400);
        assert_eq!(parsed["per_paladin_tokens"]["Scout"]["total_tokens"], 137);
        assert_eq!(
            parsed["per_paladin_tokens"]["Sentinel"]["total_tokens"],
            263
        );
        assert_eq!(parsed["node_errors"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["node_errors"][0]["node_name"], "Herald");
        assert_eq!(parsed["node_errors"][0]["error"], "connection refused");
    }

    #[test]
    fn test_json_herald_battalion_empty_paladin_results_still_valid() {
        let herald = JsonHerald::new();

        let result = BattalionResult {
            battalion_id: Uuid::new_v4(),
            battalion_name: "EmptyBattalion".to_string(),
            started_at: Utc::now(),
            completed_at: Utc::now(),
            final_output: String::new(),
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
        let parsed: Value = serde_json::from_str(&formatted).unwrap();

        assert!(parsed["paladin_results"].as_array().unwrap().is_empty());
        assert_eq!(parsed["total_tokens"], 0);
        assert!(parsed["node_errors"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_format_stream_chunk_ndjson() {
        let herald = JsonHerald::new();
        let chunk = StreamChunk::builder()
            .chunk_id(uuid::Uuid::new_v4())
            .sequence_number(0)
            .timestamp(chrono::Utc::now())
            .content("partial content".to_string())
            .is_final(false)
            .build()
            .unwrap();

        let formatted = herald.format_stream_chunk(&chunk).unwrap();
        assert!(formatted.is_some());

        let output = formatted.unwrap();
        assert!(output.ends_with('\n'));

        // Remove trailing newline and parse
        let json_str = output.trim_end();
        let parsed: Value = serde_json::from_str(json_str).unwrap();
        assert_eq!(parsed["content"], "partial content");
        assert_eq!(parsed["is_final"], false);
    }

    #[test]
    fn test_finalize_stream() {
        use paladin_ports::output::llm_port::TokenUsage;
        let herald = JsonHerald::new();
        let metadata = ExecutionMetadata::builder()
            .execution_id(uuid::Uuid::new_v4())
            .start_time(chrono::Utc::now())
            .model_used("gpt-4".to_string())
            .token_usage(TokenUsage::new(300, 200))
            .duration_ms(1234)
            .build()
            .unwrap();

        let formatted = herald.finalize_stream(&metadata).unwrap();
        assert!(formatted.ends_with('\n'));

        let json_str = formatted.trim_end();
        let parsed: Value = serde_json::from_str(json_str).unwrap();
        assert_eq!(parsed["type"], "metadata");
        assert_eq!(parsed["duration_ms"], 1234);
        assert_eq!(parsed["total_tokens"], 500);
    }

    #[test]
    fn test_format_error() {
        let herald = JsonHerald::new();
        let error = PaladinError::ExecutionError("Something went wrong".to_string());

        let formatted = herald.format_error(&error);

        let parsed: Value = serde_json::from_str(&formatted).unwrap();
        assert_eq!(parsed["error"], true);
        assert!(
            parsed["message"]
                .as_str()
                .unwrap()
                .contains("Something went wrong")
        );
    }

    #[test]
    fn test_name() {
        let herald = JsonHerald::new();
        assert_eq!(herald.name(), "json");
    }

    #[test]
    fn test_mime_type() {
        let herald = JsonHerald::new();
        assert_eq!(herald.mime_type(), "application/json");
    }

    #[test]
    fn test_pretty_vs_compact_output() {
        let result = create_test_paladin_result();

        let pretty_herald = JsonHerald::with_config(JsonHeraldConfig {
            pretty: true,
            include_metadata: false,
        });
        let pretty_output = pretty_herald.format_paladin_result(&result).unwrap();

        let compact_herald = JsonHerald::with_config(JsonHeraldConfig {
            pretty: false,
            include_metadata: false,
        });
        let compact_output = compact_herald.format_paladin_result(&result).unwrap();

        // Pretty output should have more characters due to whitespace
        assert!(pretty_output.len() > compact_output.len());

        // Both should parse to the same data
        let pretty_parsed: Value = serde_json::from_str(&pretty_output).unwrap();
        let compact_parsed: Value = serde_json::from_str(&compact_output).unwrap();
        assert_eq!(pretty_parsed["output"], compact_parsed["output"]);
    }

    #[test]
    fn test_roundtrip_paladin_result() {
        let herald = JsonHerald::new();
        let original = create_test_paladin_result();

        // Format to JSON
        let formatted = herald.format_paladin_result(&original).unwrap();

        // Parse back
        let parsed: Value = serde_json::from_str(&formatted).unwrap();

        // Verify key fields match
        assert_eq!(parsed["output"].as_str().unwrap(), original.output);
        assert_eq!(
            parsed["token_count"].as_u64().unwrap(),
            u64::from(original.usage.total_tokens)
        );
        assert_eq!(
            parsed["execution_time_ms"].as_u64().unwrap(),
            original.execution_time_ms
        );
        assert_eq!(
            parsed["loop_count"].as_u64().unwrap(),
            original.loop_count as u64
        );
    }

    #[test]
    fn test_roundtrip_battalion_result() {
        let herald = JsonHerald::new();
        let original = create_test_battalion_result();

        // Format to JSON
        let formatted = herald.format_battalion_result(&original).unwrap();

        // Parse back
        let parsed: Value = serde_json::from_str(&formatted).unwrap();

        // Verify key fields match
        assert_eq!(
            parsed["battalion_id"].as_str().unwrap(),
            original.battalion_id.to_string()
        );
        assert_eq!(
            parsed["battalion_name"].as_str().unwrap(),
            original.battalion_name
        );
        assert_eq!(parsed["paladin_results"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_herald_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<JsonHerald>();
    }

    #[test]
    fn test_default_trait() {
        let herald = JsonHerald::default();
        assert!(herald.config.pretty);
        assert!(herald.config.include_metadata);
    }

    #[test]
    fn test_streaming_output_consistency() {
        let herald = JsonHerald::new();

        // Simulate streaming chunks
        let chunks = vec![
            StreamChunk::builder()
                .chunk_id(uuid::Uuid::new_v4())
                .sequence_number(0)
                .timestamp(chrono::Utc::now())
                .content("First chunk ".to_string())
                .is_final(false)
                .build()
                .unwrap(),
            StreamChunk::builder()
                .chunk_id(uuid::Uuid::new_v4())
                .sequence_number(1)
                .timestamp(chrono::Utc::now())
                .content("Second chunk ".to_string())
                .is_final(false)
                .build()
                .unwrap(),
            StreamChunk::builder()
                .chunk_id(uuid::Uuid::new_v4())
                .sequence_number(2)
                .timestamp(chrono::Utc::now())
                .content("Final chunk".to_string())
                .is_final(true)
                .build()
                .unwrap(),
        ];

        // Collect streamed output
        let mut streamed_output = String::new();
        for chunk in &chunks {
            if let Some(formatted) = herald.format_stream_chunk(chunk).unwrap() {
                streamed_output.push_str(&formatted);
            }
        }

        // Add metadata
        use paladin_ports::output::llm_port::TokenUsage;
        let metadata = ExecutionMetadata::builder()
            .execution_id(uuid::Uuid::new_v4())
            .start_time(chrono::Utc::now())
            .model_used("gpt-4".to_string())
            .token_usage(TokenUsage::new(90, 60))
            .duration_ms(500)
            .build()
            .unwrap();
        let metadata_output = herald.finalize_stream(&metadata).unwrap();
        streamed_output.push_str(&metadata_output);

        // Verify NDJSON format (each line should be valid JSON)
        let lines: Vec<&str> = streamed_output.lines().collect();
        assert_eq!(
            lines.len(),
            4,
            "Should have 3 chunk lines + 1 metadata line"
        );

        // Verify all lines are valid JSON
        for line in &lines {
            let parsed: Value = serde_json::from_str(line)
                .unwrap_or_else(|_| panic!("Each line should be valid JSON: {}", line));
            assert!(parsed.is_object());
        }

        // Verify content chunks
        let chunk0: Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(chunk0["content"], "First chunk ");
        assert_eq!(chunk0["is_final"], false);

        let chunk1: Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(chunk1["content"], "Second chunk ");

        let chunk2: Value = serde_json::from_str(lines[2]).unwrap();
        assert_eq!(chunk2["content"], "Final chunk");
        assert_eq!(chunk2["is_final"], true);

        // Verify metadata
        let meta: Value = serde_json::from_str(lines[3]).unwrap();
        assert_eq!(meta["type"], "metadata");
        assert_eq!(meta["duration_ms"], 500);
        assert_eq!(meta["total_tokens"], 150);
    }
}
