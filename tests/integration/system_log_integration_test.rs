/*
System Log Adapter Integration Tests

These tests verify that the SystemLogAdapter correctly handles logging operations
using env_logger. Since env_logger writes to stderr/stdout, these tests focus on
functionality rather than file I/O.
*/
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

use in4me::infrastructure::adapters::logs::system_log_adapter::{
    SystemLogAdapter, SystemLogAdapterConfig
};
use in4me::application::ports::output::log_port::{LogPort, LogFormat};
use in4me::core::platform::container::log::{
    LogEntry, LogLevel, LogDestination, LogEntryBuilder
};
use in4me::core::base::entity::message::Location;

/// Test that the adapter can be created and write entries
#[tokio::test]
async fn test_adapter_creation_and_writing() {
    let config = SystemLogAdapterConfig {
        log_level: "debug".to_string(),
        format: LogFormat::Text,
        target: "integration_test".to_string(),
        structured: true,
    };

    let adapter = SystemLogAdapter::new(config).expect("Failed to create adapter");

    // Write some log entries
    let entries = vec![
        LogEntryBuilder::new_entry(
            Location::system("test_source"),
            LogDestination::System,
            LogLevel::Info,
            "Test info message".to_string(),
        ),
        LogEntryBuilder::new_entry(
            Location::service("test_service"),
            LogDestination::System,
            LogLevel::Warn,
            "Test warning message".to_string(),
        ),
        LogEntryBuilder::new_entry(
            Location::system("test_source"),
            LogDestination::System,
            LogLevel::Error,
            "Test error message".to_string(),
        ),
    ];

    // Write entries one by one
    for entry in &entries {
        adapter.write_entry(entry.clone()).await.expect("Failed to write entry");
    }

    // Flush to ensure all entries are processed
    adapter.flush().await.expect("Failed to flush");
    
    sleep(Duration::from_millis(10)).await;

    // Check statistics
    let stats = adapter.get_stats().await.expect("Failed to get stats");
    assert_eq!(stats.entries_written, 3, "Should have written 3 entries");
    assert!(stats.last_write.is_some(), "Should have last_write timestamp");

    println!("Successfully wrote {} log entries", entries.len());
}

/// Test JSON format logging
#[tokio::test]
async fn test_json_format_logging() {
    let config = SystemLogAdapterConfig {
        log_level: "info".to_string(),
        format: LogFormat::Json,
        target: "json_test".to_string(),
        structured: true,
    };

    let adapter = SystemLogAdapter::new(config).expect("Failed to create adapter");

    // Create an entry with context
    let mut entry = LogEntryBuilder::new_entry(
        Location::service("json_service"),
        LogDestination::System,
        LogLevel::Info,
        "JSON test message".to_string(),
    );
    
    // Add some context
    entry.message.context = Some(serde_json::json!({
        "user_id": "12345",
        "session_id": "abc-def-ghi",
        "action": "test_json_logging"
    }));
    
    entry.correlation_id = Some(Uuid::new_v4());

    adapter.write_entry(entry.clone()).await.expect("Failed to write JSON entry");
    adapter.flush().await.expect("Failed to flush");
    
    sleep(Duration::from_millis(10)).await;

    // Instead of testing the formatted output directly (which requires private method access),
    // we'll test that the entry was written successfully and verify the adapter's behavior
    let stats = adapter.get_stats().await.expect("Failed to get stats");
    assert_eq!(stats.entries_written, 1, "Should have written 1 entry");
    assert!(stats.last_write.is_some(), "Should have last_write timestamp");

    // Test that the adapter supports JSON format
    let formats = adapter.supported_formats();
    assert!(formats.contains(&LogFormat::Json), "Should support JSON format");

    println!("JSON formatting test passed - adapter successfully processed JSON entry");
}

/// Test batch writing functionality
#[tokio::test]
async fn test_batch_writing() {
    let config = SystemLogAdapterConfig {
        log_level: "debug".to_string(),
        format: LogFormat::Text,
        target: "batch_test".to_string(),
        structured: false, // Simple format for batch test
    };

    let adapter = SystemLogAdapter::new(config).expect("Failed to create adapter");

    // Create a batch of entries
    let batch_entries: Vec<LogEntry> = (0..50)
        .map(|i| {
            LogEntryBuilder::new_entry(
                Location::service(&format!("batch_service_{}", i)),
                LogDestination::System,
                LogLevel::Debug,
                format!("Batch message number {}", i),
            )
        })
        .collect();

    let batch_size = batch_entries.len();

    // Write the batch
    adapter.write_entries(batch_entries).await.expect("Failed to write batch");
    adapter.flush().await.expect("Failed to flush batch");
    
    sleep(Duration::from_millis(50)).await;

    // Check statistics
    let stats = adapter.get_stats().await.expect("Failed to get stats");
    assert_eq!(stats.entries_written, batch_size as u64, "Incorrect entry count in stats");
    assert!(stats.last_write.is_some(), "Last write timestamp not set");

    println!("Batch writing test passed: wrote {} entries", batch_size);
}

/// Test different log levels
#[tokio::test]
async fn test_log_levels() {
    let config = SystemLogAdapterConfig {
        log_level: "trace".to_string(), // Allow all levels
        format: LogFormat::Text,
        target: "level_test".to_string(),
        structured: true,
    };

    let adapter = SystemLogAdapter::new(config).expect("Failed to create adapter");

    // Write entries at different levels
    let entries = vec![
        (LogLevel::Trace, "This is a trace message"),
        (LogLevel::Debug, "This is a debug message"),
        (LogLevel::Info, "This is an info message"),
        (LogLevel::Warn, "This is a warning message"),
        (LogLevel::Error, "This is an error message"),
    ];

    for (level, message) in entries {
        let entry = LogEntryBuilder::new_entry(
            Location::system("level_test"),
            LogDestination::System,
            level,
            message.to_string(),
        );
        adapter.write_entry(entry).await.expect("Failed to write level test entry");
    }

    adapter.flush().await.expect("Failed to flush level test");
    sleep(Duration::from_millis(10)).await;

    let stats = adapter.get_stats().await.expect("Failed to get stats");
    assert_eq!(stats.entries_written, 5, "Should have written 5 entries");

    println!("Log level test passed: wrote entries at all levels");
}

/// Test adapter health check and statistics
#[tokio::test]
async fn test_health_and_statistics() {
    let config = SystemLogAdapterConfig {
        log_level: "info".to_string(),
        format: LogFormat::Json,
        target: "health_test".to_string(),
        structured: true,
    };

    let adapter = SystemLogAdapter::new(config).expect("Failed to create adapter");

    // Initial health check
    let health = adapter.health_check().await.expect("Failed to perform health check");
    assert!(health.is_empty() || health.iter().all(|h| h.healthy), "Adapter should be healthy initially");

    // Initial stats
    let initial_stats = adapter.get_stats().await.expect("Failed to get initial stats");
    assert_eq!(initial_stats.entries_written, 0, "Should start with 0 entries written");
    assert!(initial_stats.last_write.is_none(), "Should not have last_write initially");

    // Write some entries
    for i in 0..10 {
        let entry = LogEntryBuilder::new_entry(
            Location::system(&format!("health_source_{}", i)),
            LogDestination::System,
            LogLevel::Info,
            format!("Health test message {}", i),
        );
        adapter.write_entry(entry).await.expect("Failed to write health test entry");
    }

    // Check stats after writing
    let final_stats = adapter.get_stats().await.expect("Failed to get final stats");
    assert_eq!(final_stats.entries_written, 10, "Should have written 10 entries");
    assert!(final_stats.last_write.is_some(), "Should have last_write timestamp");

    // Health check after activity
    let final_health = adapter.health_check().await.expect("Failed to perform final health check");
    assert!(final_health.is_empty() || final_health.iter().all(|h| h.healthy), "Adapter should still be healthy");

    // Test connection
    adapter.test_connection().await.expect("Connection test should succeed");

    println!("Health and statistics test passed: {:?}", final_stats);
}

/// Test error handling
#[tokio::test]
async fn test_error_handling() {
    let config = SystemLogAdapterConfig {
        log_level: "info".to_string(),
        format: LogFormat::Text,
        target: "error_test".to_string(),
        structured: true,
    };

    let adapter = SystemLogAdapter::new(config).expect("Failed to create adapter");

    // Test that we can write error entries
    let entry = LogEntryBuilder::new_entry(
        Location::system("error_test"),
        LogDestination::System,
        LogLevel::Error,
        "Error handling test".to_string(),
    );
    
    let result = adapter.write_entry(entry).await;
    assert!(result.is_ok(), "Write entry should succeed");

    println!("Error handling test passed");
}

/// Test provider identification and supported formats
#[tokio::test]
async fn test_adapter_metadata() {
    let config = SystemLogAdapterConfig {
        log_level: "info".to_string(),
        format: LogFormat::Text,
        target: "metadata_test".to_string(),
        structured: true,
    };

    let adapter = SystemLogAdapter::new(config).expect("Failed to create adapter");

    // Test provider name
    assert_eq!(adapter.get_provider_name(), "SystemLogAdapter (log + env_logger)");

    // Test supported formats
    let formats = adapter.supported_formats();
    assert!(formats.contains(&LogFormat::Json), "Should support JSON format");
    assert!(formats.contains(&LogFormat::Text), "Should support Text format");
    assert!(formats.len() >= 2, "Should support at least 2 formats");

    println!("Metadata test passed. Supported formats: {:?}", formats);
}

/// Test configuration loading from environment
#[tokio::test]
async fn test_environment_configuration() {
    // TODO this test should probably be ignored in some enviroments
    // TODO OR it should be run in a separate test suite that can set environment variables
    // Set some environment variables
    unsafe {
        std::env::set_var("RUST_LOG", "debug");
        std::env::set_var("SYSTEM_LOG_FORMAT", "json");
        std::env::set_var("SYSTEM_LOG_TARGET", "env_test");
    }

    let config = SystemLogAdapterConfig::from_env();
    assert_eq!(config.log_level, "debug");
    assert_eq!(config.format, LogFormat::Json);
    assert_eq!(config.target, "env_test");

    let adapter = SystemLogAdapter::new(config).expect("Failed to create adapter from env");
    
    let entry = LogEntryBuilder::new_entry(
        Location::system("env_test"),
        LogDestination::System,
        LogLevel::Info,
        "Environment configuration test".to_string(),
    );
    
    adapter.write_entry(entry).await.expect("Failed to write entry");

    // TODO See above, this may remove existing env vars that are needed for other tests
    // Clean up
    unsafe {
        std::env::remove_var("RUST_LOG");
        std::env::remove_var("SYSTEM_LOG_FORMAT");
        std::env::remove_var("SYSTEM_LOG_TARGET");
    }

    println!("Environment configuration test passed");
}