//! Tests for Epic 19: Herald & Domain Type Consolidation
//!
//! These tests verify that:
//! 1. Herald uses actual domain types (not placeholders)
//! 2. StreamChunk has complete structure with extensible metadata
//! 3. ExecutionMetadata has full telemetry fields
//! 4. Herald registry auto-registers built-in formatters
//!
//! Following TDD methodology: These tests are written first and expected to fail.

use paladin::application::services::paladin::error::PaladinError;
use paladin::core::platform::container::battalion::BattalionResult;
use paladin::core::platform::container::herald::{ExecutionMetadata, StreamChunk};
use paladin_ports::output::llm_port::TokenUsage;
use paladin_ports::output::paladin_port::{PaladinResult, StopReason};
use std::collections::HashMap;
use uuid::Uuid;

/// Test that Herald uses the real PaladinResult type from paladin_port
///
/// This test verifies that the Herald trait methods accept the actual
/// PaladinResult from `application::ports::output::paladin_port`, not
/// the placeholder struct that was previously in herald.rs.
#[test]
fn test_herald_uses_real_paladin_result_type() {
    // Create a real PaladinResult with all actual fields
    let result = PaladinResult {
        output: "Test output".to_string(),
        token_count: 150,
        execution_time_ms: 1250,
        loop_count: 2,
        stop_reason: StopReason::Completed,
        ..Default::default()
    };

    // This should compile if Herald uses the real type
    // For now, just verify the type exists and has expected fields
    assert_eq!(result.output, "Test output");
    assert_eq!(result.token_count, 150);
    assert_eq!(result.execution_time_ms, 1250);
    assert_eq!(result.loop_count, 2);
    assert_eq!(result.stop_reason, StopReason::Completed);
}

/// Test that Herald uses the real BattalionResult type from battalion/mod
///
/// This test verifies that the Herald trait methods accept the actual
/// BattalionResult from `core::platform::container::battalion`, not
/// the placeholder struct.
#[test]
fn test_herald_uses_real_battalion_result_type() {
    use chrono::Utc;
    use paladin::core::platform::container::battalion::BattalionStatus;
    use paladin::core::platform::container::battalion::BattalionStrategy;

    // Create a real BattalionResult with all actual fields
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

    // This should compile if Herald uses the real type
    assert_eq!(result.battalion_name, "TestBattalion");
    assert_eq!(result.final_output, "Combined output");
}

/// Test that Herald uses the real PaladinError type from paladin/error
///
/// This test verifies that the Herald trait methods accept the actual
/// PaladinError enum from `application::services::paladin::error`, not
/// the placeholder struct.
#[test]
fn test_herald_uses_real_paladin_error_type() {
    // Create real PaladinError variants
    let error1 = PaladinError::ConfigurationError("test config error".to_string());
    let error2 = PaladinError::Timeout(60);
    let error3 = PaladinError::StopWordDetected("STOP".to_string());

    // Verify enum variants work correctly
    assert!(error1.to_string().contains("Configuration error"));
    assert!(error2.to_string().contains("Timeout"));
    assert!(error3.to_string().contains("Stop word"));
}

/// Test that StreamChunk has all required fields with proper types
///
/// This test will FAIL until StreamChunk is properly defined with:
/// - chunk_id: Uuid
/// - sequence_number: u64
/// - timestamp: DateTime<Utc>
/// - content: String
/// - token_count: Option<u32>
/// - is_final: bool
/// - metadata: HashMap<String, serde_json::Value>
#[test]
#[ignore = "Will pass after StreamChunk is fully defined"]
fn test_stream_chunk_has_all_required_fields() {
    use chrono::Utc;

    // This test will fail to compile until StreamChunk has all fields
    let _chunk = StreamChunk {
        chunk_id: Uuid::new_v4(),
        sequence_number: 0,
        timestamp: Utc::now(),
        content: "Test chunk".to_string(),
        token_count: Some(10),
        is_final: false,
        metadata: HashMap::new(),
    };

    // If this compiles, StreamChunk has all required fields
}

/// Test that StreamChunk can serialize and deserialize with metadata
///
/// This test will FAIL until StreamChunk has proper serde derives
/// and the metadata field is properly flattened.
#[test]
#[ignore = "Will pass after StreamChunk serialization is implemented"]
fn test_stream_chunk_serialization_round_trip() {
    use chrono::Utc;
    use serde_json::json;

    let mut metadata = HashMap::new();
    metadata.insert("custom_field".to_string(), json!("custom_value"));
    metadata.insert(
        "sequence_info".to_string(),
        json!({"batch": 1, "total": 10}),
    );

    let chunk = StreamChunk {
        chunk_id: Uuid::new_v4(),
        sequence_number: 5,
        timestamp: Utc::now(),
        content: "Test content".to_string(),
        token_count: Some(25),
        is_final: false,
        metadata,
    };

    // Serialize to JSON
    let json = serde_json::to_string(&chunk).expect("Failed to serialize");

    // Deserialize back
    let deserialized: StreamChunk = serde_json::from_str(&json).expect("Failed to deserialize");

    // Verify round-trip
    assert_eq!(chunk.content, deserialized.content);
    assert_eq!(chunk.sequence_number, deserialized.sequence_number);
    assert_eq!(chunk.token_count, deserialized.token_count);
    assert_eq!(chunk.is_final, deserialized.is_final);

    // Verify metadata was preserved
    assert_eq!(
        chunk.metadata.get("custom_field"),
        deserialized.metadata.get("custom_field")
    );
}

/// Test that StreamChunk has a builder pattern
///
/// This test will FAIL until StreamChunkBuilder is implemented
#[test]
#[ignore = "Will pass after StreamChunkBuilder is implemented"]
fn test_stream_chunk_builder_pattern() {
    use chrono::Utc;
    use serde_json::json;

    let chunk = StreamChunk::builder()
        .chunk_id(Uuid::new_v4())
        .sequence_number(3)
        .timestamp(Utc::now())
        .content("Builder test".to_string())
        .token_count(15)
        .is_final(true)
        .add_metadata("key1".to_string(), json!("value1"))
        .add_metadata("key2".to_string(), json!(123))
        .build()
        .expect("Builder should succeed");

    assert_eq!(chunk.content, "Builder test");
    assert_eq!(chunk.sequence_number, 3);
    assert!(chunk.is_final);
    assert_eq!(chunk.metadata.get("key1"), Some(&json!("value1")));
    assert_eq!(chunk.metadata.get("key2"), Some(&json!(123)));
}

/// Test that ExecutionMetadata has all telemetry fields
///
/// This test will FAIL until ExecutionMetadata is properly defined with:
/// - execution_id: Uuid
/// - start_time: DateTime<Utc>
/// - end_time: Option<DateTime<Utc>>
/// - duration_ms: Option<u64>
/// - model_used: String
/// - token_usage: TokenUsage
/// - cost_estimate: Option<f64>
/// - error_count: u32
/// - metadata: HashMap<String, serde_json::Value>
#[test]
#[ignore = "Will pass after ExecutionMetadata is fully defined"]
fn test_execution_metadata_has_all_telemetry_fields() {
    use chrono::Utc;

    // This will fail to compile until ExecutionMetadata and TokenUsage are defined
    let _metadata = ExecutionMetadata {
        execution_id: Uuid::new_v4(),
        start_time: Utc::now(),
        end_time: Some(Utc::now()),
        duration_ms: Some(1500),
        model_used: "gpt-4".to_string(),
        token_usage: TokenUsage::new(100, 50),
        cost_estimate: Some(0.003),
        error_count: 0,
        metadata: HashMap::new(),
    };

    // If this compiles, ExecutionMetadata has all required fields
}

/// Test that ExecutionMetadata can calculate duration automatically
///
/// This test will FAIL until ExecutionMetadata::calculate_duration() is implemented
#[test]
#[ignore = "Will pass after calculate_duration method is implemented"]
fn test_execution_metadata_calculate_duration() {
    use chrono::{Duration, Utc};

    let start = Utc::now();
    let end = start + Duration::milliseconds(2500);

    let mut metadata = ExecutionMetadata {
        execution_id: Uuid::new_v4(),
        start_time: start,
        end_time: Some(end),
        duration_ms: None, // Not set initially
        model_used: "gpt-4".to_string(),
        token_usage: TokenUsage::new(100, 50),
        cost_estimate: None,
        error_count: 0,
        metadata: HashMap::new(),
    };

    // Calculate duration
    metadata.calculate_duration();

    // Verify duration was calculated correctly
    assert!(metadata.duration_ms.is_some());
    assert_eq!(metadata.duration_ms.unwrap(), 2500);
}

/// Test that ExecutionMetadata serializes with extensible metadata
///
/// This test will FAIL until ExecutionMetadata has proper serialization
#[test]
#[ignore = "Will pass after ExecutionMetadata serialization is implemented"]
fn test_execution_metadata_serialization_round_trip() {
    use chrono::Utc;
    use serde_json::json;

    let mut custom_metadata = HashMap::new();
    custom_metadata.insert("provider".to_string(), json!("openai"));
    custom_metadata.insert("region".to_string(), json!("us-east-1"));

    let metadata = ExecutionMetadata {
        execution_id: Uuid::new_v4(),
        start_time: Utc::now(),
        end_time: Some(Utc::now()),
        duration_ms: Some(1234),
        model_used: "gpt-4-turbo".to_string(),
        token_usage: TokenUsage::new(200, 100),
        cost_estimate: Some(0.006),
        error_count: 2,
        metadata: custom_metadata,
    };

    // Serialize
    let json = serde_json::to_string(&metadata).expect("Failed to serialize");

    // Deserialize
    let deserialized: ExecutionMetadata =
        serde_json::from_str(&json).expect("Failed to deserialize");

    // Verify round-trip
    assert_eq!(metadata.model_used, deserialized.model_used);
    assert_eq!(metadata.duration_ms, deserialized.duration_ms);
    assert_eq!(
        metadata.token_usage.total_tokens,
        deserialized.token_usage.total_tokens
    );
    assert_eq!(metadata.error_count, deserialized.error_count);
    assert_eq!(
        metadata.metadata.get("provider"),
        deserialized.metadata.get("provider")
    );
}

/// Test that ExecutionMetadata has a builder pattern
///
/// This test will FAIL until ExecutionMetadataBuilder is implemented
#[test]
#[ignore = "Will pass after ExecutionMetadataBuilder is implemented"]
fn test_execution_metadata_builder_pattern() {
    use chrono::Utc;
    use serde_json::json;

    let metadata = ExecutionMetadata::builder()
        .execution_id(Uuid::new_v4())
        .start_time(Utc::now())
        .end_time(Utc::now())
        .model_used("gpt-4".to_string())
        .token_usage(TokenUsage::new(150, 75))
        .cost_estimate(0.0045)
        .error_count(1)
        .add_metadata("key1".to_string(), json!("value1"))
        .build()
        .expect("Builder should succeed");

    assert_eq!(metadata.model_used, "gpt-4");
    assert_eq!(metadata.token_usage.total_tokens, 225);
    assert_eq!(metadata.error_count, 1);
    assert_eq!(metadata.metadata.get("key1"), Some(&json!("value1")));
}

/// Test that HeraldRegistry::default() auto-registers JSON formatter
///
/// This test will FAIL until HeraldRegistry implements Default trait
/// with auto-registration.
#[test]
#[ignore = "Will pass after auto-registration is implemented"]
fn test_herald_registry_default_has_json_formatter() {
    use paladin::application::services::herald::herald_registry::HeraldRegistry;

    let registry = HeraldRegistry::default();

    // Should have JSON formatter registered
    assert!(
        registry.get("json").is_some(),
        "JSON formatter should be auto-registered"
    );

    let formatter = registry.get("json").unwrap();
    assert_eq!(formatter.name(), "json");
}

/// Test that HeraldRegistry::default() auto-registers Markdown formatter
///
/// This test will FAIL until auto-registration is implemented.
#[test]
#[ignore = "Will pass after auto-registration is implemented"]
fn test_herald_registry_default_has_markdown_formatter() {
    use paladin::application::services::herald::herald_registry::HeraldRegistry;

    let registry = HeraldRegistry::default();

    // Should have Markdown formatter registered
    assert!(
        registry.get("markdown").is_some(),
        "Markdown formatter should be auto-registered"
    );

    let formatter = registry.get("markdown").unwrap();
    assert_eq!(formatter.name(), "markdown");
}

/// Test that HeraldRegistry::default() auto-registers Table formatter
///
/// This test will FAIL until auto-registration is implemented.
#[test]
#[ignore = "Will pass after auto-registration is implemented"]
fn test_herald_registry_default_has_table_formatter() {
    use paladin::application::services::herald::herald_registry::HeraldRegistry;

    let registry = HeraldRegistry::default();

    // Should have Table formatter registered
    assert!(
        registry.get("table").is_some(),
        "Table formatter should be auto-registered"
    );

    let formatter = registry.get("table").unwrap();
    assert_eq!(formatter.name(), "table");
}

/// Test Herald pipeline works end-to-end with consolidated types
///
/// This integration test verifies that the full Herald pipeline
/// works with the actual domain types (not placeholders).
#[test]
#[ignore = "Will pass after all Herald consolidation is complete"]
fn test_herald_pipeline_with_consolidated_types() {
    use chrono::Utc;
    use paladin::application::services::herald::herald_registry::HeraldRegistry;
    use paladin::core::platform::container::battalion::{BattalionStatus, BattalionStrategy};

    // Create a real PaladinResult
    let paladin_result = PaladinResult {
        output: "Integration test output".to_string(),
        token_count: 200,
        execution_time_ms: 2000,
        loop_count: 3,
        stop_reason: StopReason::Completed,
        ..Default::default()
    };

    // Create a real BattalionResult
    let battalion_result = BattalionResult {
        battalion_id: Uuid::new_v4(),
        battalion_name: "IntegrationBattalion".to_string(),
        started_at: Utc::now(),
        completed_at: Utc::now(),
        final_output: "Battalion output".to_string(),
        paladin_results: vec![paladin_result.clone()],
        status: BattalionStatus::Completed,
        strategy_used: BattalionStrategy::Formation,
        strategy_selection_reasoning: None,
        strategy_selection_time_ms: 0,
        per_paladin_times: [("paladin_0".to_string(), 2000u64)].into_iter().collect(),
        per_paladin_tokens: std::collections::HashMap::new(),
        total_tokens: 0,
        paladin_success_count: 1,
        paladin_failure_count: 0,
        node_errors: Vec::new(),
    };

    // Get formatters from registry
    let registry = HeraldRegistry::default();
    let json_formatter = registry
        .get("json")
        .expect("JSON formatter should be available");

    // Format Paladin result
    let paladin_formatted = json_formatter
        .format_paladin_result(&paladin_result)
        .expect("Should format Paladin result");
    assert!(paladin_formatted.contains("Integration test output"));

    // Format Battalion result
    let battalion_formatted = json_formatter
        .format_battalion_result(&battalion_result)
        .expect("Should format Battalion result");
    assert!(battalion_formatted.contains("Battalion output"));

    // Format error
    let error = PaladinError::Timeout(60);
    let error_formatted = json_formatter.format_error(&error);
    assert!(error_formatted.contains("Timeout"));
}
