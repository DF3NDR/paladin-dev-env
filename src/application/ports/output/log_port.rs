/*
Logger Port

A port that defines how the application logs information. This could be a file, a database,
a logging service, or any other logging mechanism.

This port is responsible for providing an abstraction layer that allows the application to log
information without being tightly coupled to the details of how that information is logged.

Typical implementations of this port would be for the adapter to translate log messages
from the application into calls to the logging mechanism, and to translate log messages
from the logging mechanism back into a format that the application can use.

An example of a logging mechanism would be a file logger that writes log messages to a file.
The Logger Port would provide an interface for the application to log messages, and the adapter
would translate those messages into calls to the file logger.

Errors and Events emitted by the application would typically be made available to this port
by way of the application's use cases through the logging_service.
*/
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

use crate::core::platform::container::log::{LogEntry, LogLevel, LogDestination};

/// Result type for logging operations
pub type LogResult<T> = Result<T, LogError>;

/// Errors that can occur during logging operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum LogError {
    #[error("IO error: {0}")]
    IoError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Log destination not found: {0}")]
    DestinationNotFound(String),
    
    #[error("Log buffer full")]
    BufferFull,
    
    #[error("Log service unavailable")]
    ServiceUnavailable,
    
    #[error("Invalid log format: {0}")]
    InvalidFormat(String),
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Configuration for log rotation
#[derive(Debug, Clone)]
pub struct LogRotationConfig {
    /// Maximum file size in bytes before rotation
    pub max_size: u64,
    /// Maximum number of rotated files to keep
    pub max_files: u32,
    /// Whether to compress rotated files
    pub compress: bool,
}

impl Default for LogRotationConfig {
    fn default() -> Self {
        Self {
            max_size: 10 * 1024 * 1024, // 10MB
            max_files: 5,
            compress: true,
        }
    }
}

/// Configuration for a log destination
#[derive(Debug, Clone)]
pub struct LogDestinationConfig {
    /// The log destination identifier
    pub destination: LogDestination,
    /// Minimum log level for this destination
    pub min_level: LogLevel,
    /// Output format (json, text, etc.)
    pub format: LogFormat,
    /// Whether this destination is enabled
    pub enabled: bool,
    /// Rotation configuration (if applicable)
    pub rotation: Option<LogRotationConfig>,
    /// Additional destination-specific settings
    pub settings: HashMap<String, String>,
}

/// Log output formats
#[derive(Debug, Clone, PartialEq)]
pub enum LogFormat {
    /// Plain text format
    Text,
    /// JSON format
    Json,
    /// Structured format with custom template
    Structured(String),
}

impl Default for LogFormat {
    fn default() -> Self {
        Self::Text
    }
}

/// Statistics about logging operations
#[derive(Debug, Clone, Default)]
pub struct LogStats {
    /// Total number of log entries written
    pub entries_written: u64,
    /// Number of entries by level
    pub entries_by_level: HashMap<LogLevel, u64>,
    /// Number of errors encountered
    pub errors: u64,
    /// Bytes written
    pub bytes_written: u64,
    /// Last write timestamp
    pub last_write: Option<DateTime<Utc>>,
}

/// Query parameters for retrieving log entries
#[derive(Debug, Clone)]
pub struct LogQuery {
    /// Filter by minimum log level
    pub min_level: Option<LogLevel>,
    /// Filter by maximum log level
    pub max_level: Option<LogLevel>,
    /// Filter by time range (start)
    pub start_time: Option<DateTime<Utc>>,
    /// Filter by time range (end)
    pub end_time: Option<DateTime<Utc>>,
    /// Filter by source location
    pub source: Option<String>,
    /// Filter by module
    pub module: Option<String>,
    /// Search in message content
    pub message_contains: Option<String>,
    /// Maximum number of entries to return
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

impl Default for LogQuery {
    fn default() -> Self {
        Self {
            min_level: None,
            max_level: None,
            start_time: None,
            end_time: None,
            source: None,
            module: None,
            message_contains: None,
            limit: Some(1000), // Default limit
            offset: None,
        }
    }
}

/// Batch write request for multiple entries
#[derive(Debug, Clone)]
pub struct BatchWriteRequest {
    /// Entries to write
    pub entries: Vec<LogEntry>,
    /// Whether to write atomically (all or none)
    pub atomic: bool,
    /// Maximum time to wait for write completion
    pub timeout: Option<std::time::Duration>,
}

impl BatchWriteRequest {
    /// Create a new batch write request
    pub fn new(entries: Vec<LogEntry>) -> Self {
        Self {
            entries,
            atomic: true,
            timeout: None,
        }
    }
    
    /// Set atomic write behavior
    pub fn with_atomic(mut self, atomic: bool) -> Self {
        self.atomic = atomic;
        self
    }
    
    /// Set write timeout
    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

/// Health check result for log destinations
#[derive(Debug, Clone)]
pub struct LogHealthCheck {
    /// The destination being checked
    pub destination: LogDestination,
    /// Whether the destination is healthy
    pub healthy: bool,
    /// Last successful write timestamp
    pub last_write: Option<DateTime<Utc>>,
    /// Error message if unhealthy
    pub error_message: Option<String>,
    /// Response time for last operation
    pub response_time: Option<std::time::Duration>,
}

/// Port defining the logging interface
#[async_trait]
pub trait LogPort: Send + Sync {
    /// Write a single log entry
    async fn write_entry(&self, entry: LogEntry) -> LogResult<()>;
    
    /// Write multiple log entries atomically
    async fn write_entries(&self, entries: Vec<LogEntry>) -> LogResult<()>;
    
    /// Write entries in batch with options
    async fn batch_write(&self, request: BatchWriteRequest) -> LogResult<()>;
    
    /// Read log entries based on query parameters
    async fn read_entries(&self, query: LogQuery) -> LogResult<Vec<LogEntry>>;
    
    /// Get the count of entries matching query
    async fn count_entries(&self, query: LogQuery) -> LogResult<u64>;
    
    /// Configure a log destination
    async fn configure_destination(&self, config: LogDestinationConfig) -> LogResult<()>;
    
    /// Remove a log destination configuration
    async fn remove_destination(&self, destination: LogDestination) -> LogResult<()>;
    
    /// Get list of configured destinations
    async fn list_destinations(&self) -> LogResult<Vec<LogDestination>>;
    
    /// Flush any buffered log entries
    async fn flush(&self) -> LogResult<()>;
    
    /// Flush specific destination
    async fn flush_destination(&self, destination: LogDestination) -> LogResult<()>;
    
    /// Rotate logs for a specific destination
    async fn rotate_logs(&self, destination: LogDestination) -> LogResult<()>;
    
    /// Get logging statistics
    async fn get_stats(&self) -> LogResult<LogStats>;
    
    /// Get statistics for specific destination
    async fn get_destination_stats(&self, destination: LogDestination) -> LogResult<LogStats>;
    
    /// Clear logs for a destination
    async fn clear_logs(&self, destination: LogDestination) -> LogResult<()>;
    
    /// Clear logs older than specified date
    async fn clear_logs_before(&self, destination: LogDestination, before: DateTime<Utc>) -> LogResult<u64>;
    
    /// Health check for log destinations
    async fn health_check(&self) -> LogResult<Vec<LogHealthCheck>>;
    
    /// Health check for specific destination
    async fn health_check_destination(&self, destination: LogDestination) -> LogResult<LogHealthCheck>;
    
    /// Get the name of this log port implementation
    fn get_provider_name(&self) -> &'static str;
    
    /// Test connectivity and permissions
    async fn test_connection(&self) -> LogResult<()>;
    
    /// Archive old logs
    async fn archive_logs(&self, destination: LogDestination, before: DateTime<Utc>) -> LogResult<String>;
    
    /// Get available log formats supported by this implementation
    fn supported_formats(&self) -> Vec<LogFormat>;
}

/// Trait for log formatters
pub trait LogFormatter: Send + Sync {
    /// Format a log entry for output
    fn format_entry(&self, entry: &LogEntry) -> LogResult<String>;
    
    /// Format multiple entries
    fn format_entries(&self, entries: &[LogEntry]) -> LogResult<Vec<String>> {
        entries.iter().map(|e| self.format_entry(e)).collect()
    }
    
    /// Get the format identifier
    fn format_type(&self) -> LogFormat;
}

/// Default text formatter
pub struct TextLogFormatter;

impl LogFormatter for TextLogFormatter {
    fn format_entry(&self, entry: &LogEntry) -> LogResult<String> {
        Ok(format!(
            "{} [{}] {} - {}",
            entry.timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
            entry.message.level.as_str(),
            format!("{:?}", entry.source),
            entry.message.formatted()
        ))
    }
    
    fn format_type(&self) -> LogFormat {
        LogFormat::Text
    }
}

/// JSON formatter
pub struct JsonLogFormatter;

impl LogFormatter for JsonLogFormatter {
    fn format_entry(&self, entry: &LogEntry) -> LogResult<String> {
        serde_json::to_string(entry)
            .map_err(|e| LogError::SerializationError(e.to_string()))
    }
    
    fn format_type(&self) -> LogFormat {
        LogFormat::Json
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::base::entity::message::Location;
    use crate::core::platform::container::log::{LogEntryBuilder, LogLevel, LogDestination};

    #[test]
    fn test_log_query_default() {
        let query = LogQuery::default();
        assert_eq!(query.limit, Some(1000));
        assert!(query.min_level.is_none());
    }

    #[test]
    fn test_batch_write_request() {
        let entries = vec![
            LogEntryBuilder::new_entry(
                Location::service("test"),
                LogDestination::System,
                LogLevel::Info,
                "Test message".to_string(),
            )
        ];
        
        let request = BatchWriteRequest::new(entries.clone())
            .with_atomic(false)
            .with_timeout(std::time::Duration::from_secs(5));
        
        assert_eq!(request.entries.len(), 1);
        assert!(!request.atomic);
        assert_eq!(request.timeout, Some(std::time::Duration::from_secs(5)));
    }

    #[test]
    fn test_text_formatter() {
        let formatter = TextLogFormatter;
        let entry = LogEntryBuilder::new_entry(
            Location::service("test-service"),
            LogDestination::System,
            LogLevel::Info,
            "Test message".to_string(),
        );
        let formatted = formatter.format_entry(&entry).unwrap();
        assert!(formatted.contains("[INFO]"));
        assert!(formatted.contains("Test message"));
    }

    #[test]
    fn test_json_formatter() {
        let formatter = JsonLogFormatter;
        let entry = LogEntryBuilder::new_entry(
            Location::service("test-service"),
            LogDestination::System,
            LogLevel::Info,
            "Test message".to_string(),
        );
        
        let formatted = formatter.format_entry(&entry).unwrap();
        assert!(formatted.contains("\"Info\"") || formatted.contains("Info"));
        assert!(formatted.contains("Test message"));
    }
}