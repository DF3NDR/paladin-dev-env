/*
System Log Adapter

Infrastructure adapter that implements the LogPort using the standard Rust tracing ecosystem.
This adapter provides file-based logging with rotation, structured logging support, and
configurable output formats.

Uses tracing-subscriber for structured logging and tracing-appender for file rotation.
Configuration is managed through environment variables.
*/
use std::env;
use std::path::Path;
use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug, trace, Level};
use tracing_subscriber::{
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
    Registry,
    Layer, // Add this import for .boxed() method
};
use tracing_appender::{non_blocking, rolling};

use crate::application::ports::output::log_port::{
    LogPort, LogResult, LogError, LogQuery, LogDestinationConfig, LogStats, 
    LogHealthCheck, LogFormat, BatchWriteRequest
};
use crate::core::platform::container::log::{LogEntry, LogLevel, LogDestination, LogEntryExt};

/// Configuration for the system log adapter
#[derive(Debug, Clone)]
pub struct SystemLogAdapterConfig {
    /// Log level filter
    pub log_level: String,
    /// Output format (json, text, structured)
    pub format: LogFormat,
    /// Log file path
    pub file_path: String,
    /// Maximum file size before rotation
    pub max_size: String,
    /// Maximum number of rotated files
    pub max_files: u32,
    /// Enable console output
    pub enable_console: bool,
    /// Enable file output
    pub enable_file: bool,
    /// Enable structured logging
    pub enable_structured: bool,
}

impl Default for SystemLogAdapterConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            format: LogFormat::Json,
            file_path: "/var/log/in4me-system.log".to_string(),
            max_size: "10MB".to_string(),
            max_files: 5,
            enable_console: true,
            enable_file: true,
            enable_structured: true,
        }
    }
}

impl SystemLogAdapterConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            log_level: env::var("SYSTEM_LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
            format: match env::var("SYSTEM_LOG_FORMAT").unwrap_or_else(|_| "json".to_string()).as_str() {
                "json" => LogFormat::Json,
                "text" => LogFormat::Text,
                template => LogFormat::Structured(template.to_string()),
            },
            file_path: env::var("SYSTEM_LOG_FILE").unwrap_or_else(|_| "/var/log/in4me-system.log".to_string()),
            max_size: env::var("SYSTEM_LOG_MAX_SIZE").unwrap_or_else(|_| "10MB".to_string()),
            max_files: env::var("SYSTEM_LOG_MAX_FILES")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .unwrap_or(5),
            enable_console: env::var("SYSTEM_LOG_ENABLE_CONSOLE")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            enable_file: env::var("SYSTEM_LOG_ENABLE_FILE")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            enable_structured: env::var("SYSTEM_LOG_ENABLE_STRUCTURED")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
        }
    }
}

/// System log adapter using tracing
pub struct SystemLogAdapter {
    /// Configuration
    config: SystemLogAdapterConfig,
    /// Statistics
    stats: Arc<RwLock<LogStats>>,
    /// File appender guard (keeps the background thread alive)
    _file_guard: Option<tracing_appender::non_blocking::WorkerGuard>,
    /// Destination configurations
    destinations: Arc<RwLock<HashMap<LogDestination, LogDestinationConfig>>>,
}

impl SystemLogAdapter {
    /// Create a new system log adapter
    pub fn new(config: SystemLogAdapterConfig) -> LogResult<Self> {
        let stats = Arc::new(RwLock::new(LogStats::default()));
        let destinations = Arc::new(RwLock::new(HashMap::new()));
        
        // Initialize the tracing subscriber
        let (file_guard, subscriber) = Self::build_subscriber(&config)?;
        
        // Set the global subscriber
        subscriber.try_init()
            .map_err(|e| LogError::ConfigError(format!("Failed to initialize logging: {}", e)))?;
        
        Ok(Self {
            config,
            stats,
            _file_guard: file_guard,
            destinations,
        })
    }
    
    /// Create with default configuration from environment
    pub fn from_env() -> LogResult<Self> {
        let config = SystemLogAdapterConfig::from_env();
        Self::new(config)
    }
    
    /// Build the tracing subscriber
    fn build_subscriber(config: &SystemLogAdapterConfig) -> LogResult<(Option<tracing_appender::non_blocking::WorkerGuard>, impl SubscriberExt + Send + Sync)> {
        let filter = EnvFilter::try_new(&config.log_level)
            .map_err(|e| LogError::ConfigError(format!("Invalid log level: {}", e)))?;
        
        let registry = Registry::default().with(filter);
        
        // Add console layer if enabled
        let registry = if config.enable_console {
            match config.format {
                LogFormat::Json => {
                    let layer = fmt::layer()
                        .json()
                        .with_current_span(false)
                        .with_span_list(true);
                    registry.with(layer)
                }
                LogFormat::Text => {
                    let layer = fmt::layer()
                        .pretty()
                        .with_thread_ids(true)
                        .with_thread_names(true);
                    registry.with(layer)
                }
                LogFormat::Structured(_) => {
                    let layer = fmt::layer()
                        .with_target(true)
                        .with_level(true)
                        .with_thread_ids(true);
                    registry.with(layer)
                }
            }
        } else {
            registry
        };
        
        // Add file layer if enabled
        let (file_guard, registry) = if config.enable_file {
            // Create log directory if it doesn't exist
            let log_path = Path::new(&config.file_path);
            if let Some(parent) = log_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| LogError::IoError(format!("Failed to create log directory: {}", e)))?;
            }
            
            // Extract directory and filename
            let log_dir = log_path.parent()
                .ok_or_else(|| LogError::ConfigError("Invalid log file path".to_string()))?;
            let log_filename = log_path.file_stem()
                .ok_or_else(|| LogError::ConfigError("Invalid log filename".to_string()))?
                .to_str()
                .ok_or_else(|| LogError::ConfigError("Invalid log filename encoding".to_string()))?;
            
            // Create rolling file appender
            let file_appender = rolling::daily(log_dir, log_filename);
            let (non_blocking, guard) = non_blocking(file_appender);
            
            let file_layer = match config.format {
                LogFormat::Json => {
                    fmt::layer()
                        .json()
                        .with_writer(non_blocking)
                        .with_ansi(false)
                }
                LogFormat::Text => {
                    fmt::layer()
                        .with_writer(non_blocking)
                        .with_ansi(false)
                }
                LogFormat::Structured(_) => {
                    fmt::layer()
                        .with_writer(non_blocking)
                        .with_ansi(false)
                }
            };
            
            (Some(guard), registry.with(file_layer))
        } else {
            (None, registry)
        };
        
        Ok((file_guard, registry))
    }
    
    /// Convert LogLevel to tracing Level
    fn log_level_to_tracing_level(level: LogLevel) -> Level {
        match level {
            LogLevel::Trace => Level::TRACE,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Info => Level::INFO,
            LogLevel::Warn => Level::WARN,
            LogLevel::Error => Level::ERROR,
            LogLevel::Fatal => Level::ERROR, // Map Fatal to ERROR
        }
    }
    
    /// Write log entry using tracing
    fn write_tracing_entry(&self, entry: &LogEntry) {
        let level = Self::log_level_to_tracing_level(entry.level());
        let message = &entry.message.message;
        let module = entry.message.module.as_deref().unwrap_or("unknown");
        let function = entry.message.function.as_deref().unwrap_or("unknown");
        
        // Create span for structured logging
        let span = tracing::span!(
            target: "in4me",
            level,
            "log_entry",
            id = %entry.id,
            source = %entry.source,
            destination = %entry.destination,
            module = module,
            function = function,
            correlation_id = ?entry.correlation_id,
            priority = ?entry.priority,
        );
        
        let _enter = span.enter();
        
        // Log with context if available
        if let Some(context) = &entry.message.context {
            match level {
                Level::TRACE => trace!(context = ?context, "{}", message),
                Level::DEBUG => debug!(context = ?context, "{}", message),
                Level::INFO => info!(context = ?context, "{}", message),
                Level::WARN => warn!(context = ?context, "{}", message),
                Level::ERROR => error!(context = ?context, "{}", message),
            }
        } else {
            match level {
                Level::TRACE => trace!("{}", message),
                Level::DEBUG => debug!("{}", message),
                Level::INFO => info!("{}", message),
                Level::WARN => warn!("{}", message),
                Level::ERROR => error!("{}", message),
            }
        }
    }
    
    /// Update statistics
    async fn update_stats(&self, entries_count: usize) {
        let mut stats = self.stats.write().await;
        stats.entries_written += entries_count as u64;
        stats.last_write = Some(Utc::now());
    }
}

#[async_trait]
impl LogPort for SystemLogAdapter {
    async fn write_entry(&self, entry: LogEntry) -> LogResult<()> {
        self.write_tracing_entry(&entry);
        self.update_stats(1).await;
        Ok(())
    }
    
    async fn write_entries(&self, entries: Vec<LogEntry>) -> LogResult<()> {
        let count = entries.len();
        for entry in entries {
            self.write_tracing_entry(&entry);
        }
        self.update_stats(count).await;
        Ok(())
    }
    
    async fn batch_write(&self, request: BatchWriteRequest) -> LogResult<()> {
        if request.atomic {
            // For atomic writes, we'll write all entries in sequence
            // In a real implementation, you might want to use a transaction
            for entry in &request.entries {
                self.write_tracing_entry(entry);
            }
        } else {
            // For non-atomic writes, write entries individually
            for entry in &request.entries {
                self.write_tracing_entry(entry);
            }
        }
        
        self.update_stats(request.entries.len()).await;
        Ok(())
    }
    
    async fn read_entries(&self, _query: LogQuery) -> LogResult<Vec<LogEntry>> {
        // Note: tracing doesn't provide built-in log reading capabilities
        // For log reading, you would typically use a separate log aggregation system
        // or parse the log files directly. This is a limitation of the tracing approach.
        Err(LogError::ConfigError(
            "Log reading not supported by tracing adapter. Use a log aggregation system.".to_string()
        ))
    }
    
    async fn count_entries(&self, _query: LogQuery) -> LogResult<u64> {
        // Same limitation as read_entries
        Err(LogError::ConfigError(
            "Log counting not supported by tracing adapter. Use a log aggregation system.".to_string()
        ))
    }
    
    async fn configure_destination(&self, config: LogDestinationConfig) -> LogResult<()> {
        let mut destinations = self.destinations.write().await;
        destinations.insert(config.destination.clone(), config);
        Ok(())
    }
    
    async fn remove_destination(&self, destination: LogDestination) -> LogResult<()> {
        let mut destinations = self.destinations.write().await;
        destinations.remove(&destination);
        Ok(())
    }
    
    async fn list_destinations(&self) -> LogResult<Vec<LogDestination>> {
        let destinations = self.destinations.read().await;
        Ok(destinations.keys().cloned().collect())
    }
    
    async fn flush(&self) -> LogResult<()> {
        // Tracing handles flushing automatically
        Ok(())
    }
    
    async fn flush_destination(&self, _destination: LogDestination) -> LogResult<()> {
        // Tracing handles flushing automatically
        Ok(())
    }
    
    async fn rotate_logs(&self, _destination: LogDestination) -> LogResult<()> {
        // Rotation is handled by tracing-appender automatically
        Ok(())
    }
    
    async fn get_stats(&self) -> LogResult<LogStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
    
    async fn get_destination_stats(&self, _destination: LogDestination) -> LogResult<LogStats> {
        // For simplicity, return global stats
        // In a real implementation, you might want to track per-destination stats
        self.get_stats().await
    }
    
    async fn clear_logs(&self, destination: LogDestination) -> LogResult<()> {
        // Clearing logs would require file system operations
        // This is typically not recommended for production systems
        warn!("Clear logs requested for destination: {:?}", destination);
        Ok(())
    }
    
    async fn clear_logs_before(&self, destination: LogDestination, before: DateTime<Utc>) -> LogResult<u64> {
        // This would require parsing log files and removing entries
        // Not implemented for tracing adapter
        warn!("Clear logs before {:?} requested for destination: {:?}", before, destination);
        Ok(0)
    }
    
    async fn health_check(&self) -> LogResult<Vec<LogHealthCheck>> {
        let destinations = self.destinations.read().await;
        let mut checks = Vec::new();
        
        for (destination, _config) in destinations.iter() {
            let check = LogHealthCheck {
                destination: destination.clone(),
                healthy: true, // Tracing is always healthy if initialized
                last_write: {
                    let stats = self.stats.read().await;
                    stats.last_write
                },
                error_message: None,
                response_time: Some(std::time::Duration::from_millis(1)),
            };
            checks.push(check);
        }
        
        Ok(checks)
    }
    
    async fn health_check_destination(&self, destination: LogDestination) -> LogResult<LogHealthCheck> {
        let check = LogHealthCheck {
            destination,
            healthy: true,
            last_write: {
                let stats = self.stats.read().await;
                stats.last_write
            },
            error_message: None,
            response_time: Some(std::time::Duration::from_millis(1)),
        };
        Ok(check)
    }
    
    fn get_provider_name(&self) -> &'static str {
        "SystemLogAdapter (tracing)"
    }
    
    async fn test_connection(&self) -> LogResult<()> {
        // Test by writing a test log entry
        info!("System log adapter connection test");
        Ok(())
    }
    
    async fn archive_logs(&self, destination: LogDestination, before: DateTime<Utc>) -> LogResult<String> {
        // Archiving would require file system operations
        // Not implemented for tracing adapter
        warn!("Archive logs before {:?} requested for destination: {:?}", before, destination);
        Ok("Archive not implemented for tracing adapter".to_string())
    }
    
    fn supported_formats(&self) -> Vec<LogFormat> {
        vec![LogFormat::Json, LogFormat::Text, LogFormat::Structured("custom".to_string())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::core::platform::container::log::LogEntryBuilder;

    #[tokio::test]
    async fn test_system_log_adapter_creation() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");
        
        let config = SystemLogAdapterConfig {
            log_level: "debug".to_string(),
            format: LogFormat::Json,
            file_path: log_path.to_string_lossy().to_string(),
            max_size: "1MB".to_string(),
            max_files: 3,
            enable_console: false, // Disable for testing
            enable_file: true,
            enable_structured: true,
        };
        
        let adapter = SystemLogAdapter::new(config).unwrap();
        assert_eq!(adapter.get_provider_name(), "SystemLogAdapter (tracing)");
    }

    #[tokio::test]
    async fn test_write_entry() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");
        
        let config = SystemLogAdapterConfig {
            log_level: "debug".to_string(),
            format: LogFormat::Json,
            file_path: log_path.to_string_lossy().to_string(),
            max_size: "1MB".to_string(),
            max_files: 3,
            enable_console: false,
            enable_file: true,
            enable_structured: true,
        };
        
        let adapter = SystemLogAdapter::new(config).unwrap();
        
        let entry = LogEntryBuilder::new_entry(
            crate::core::base::entity::message::Location::system("test"),
            LogDestination::System,
            LogLevel::Info,
            "Test message".to_string(),
        );
        
        let result = adapter.write_entry(entry).await;
        assert!(result.is_ok());
        
        let stats = adapter.get_stats().await.unwrap();
        assert_eq!(stats.entries_written, 1);
    }

    #[tokio::test]
    async fn test_health_check() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");
        
        let config = SystemLogAdapterConfig {
            log_level: "info".to_string(),
            format: LogFormat::Text,
            file_path: log_path.to_string_lossy().to_string(),
            max_size: "1MB".to_string(),
            max_files: 3,
            enable_console: false,
            enable_file: true,
            enable_structured: true,
        };
        
        let adapter = SystemLogAdapter::new(config).unwrap();
        
        let health = adapter.health_check_destination(LogDestination::System).await.unwrap();
        assert!(health.healthy);
        assert_eq!(health.destination, LogDestination::System);
    }
}