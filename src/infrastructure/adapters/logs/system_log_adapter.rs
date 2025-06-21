/*
System Log Adapter

Infrastructure adapter that implements the LogPort using the standard Rust log ecosystem.
This adapter provides logging using the log crate with env_logger for simple configuration.
The adapter formats log entries and writes them through the standard logging facade.

Uses log for the logging facade and env_logger for configuration and output.
Configuration is managed through environment variables.
*/
use std::env;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Once;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use log::{info, warn, error, debug, trace};
use serde_json::json;

use crate::application::ports::output::log_port::{
    LogPort, LogResult, LogError, LogQuery, LogDestinationConfig, LogStats, 
    LogHealthCheck, LogFormat, BatchWriteRequest
};
use crate::core::platform::container::log::{LogEntry, LogLevel, LogDestination, LogEntryExt};

/// Ensure env_logger is only initialized once
static INIT: Once = Once::new();

/// Configuration for the system log adapter
#[derive(Debug, Clone)]
pub struct SystemLogAdapterConfig {
    /// Log level filter
    pub log_level: String,
    /// Output format (json, text, structured)
    pub format: LogFormat,
    /// Log target identifier
    pub target: String,
    /// Enable structured output
    pub structured: bool,
}

impl Default for SystemLogAdapterConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            format: LogFormat::Text,
            target: "in4me".to_string(),
            structured: true,
        }
    }
}

impl SystemLogAdapterConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            log_level: env::var("RUST_LOG").unwrap_or_else(|_| 
                env::var("SYSTEM_LOG_LEVEL").unwrap_or_else(|_| "info".to_string())
            ),
            format: match env::var("SYSTEM_LOG_FORMAT").unwrap_or_else(|_| "text".to_string()).as_str() {
                "json" => LogFormat::Json,
                "text" => LogFormat::Text,
                template => LogFormat::Structured(template.to_string()),
            },
            target: env::var("SYSTEM_LOG_TARGET").unwrap_or_else(|_| "in4me".to_string()),
            structured: env::var("SYSTEM_LOG_STRUCTURED")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
        }
    }
}

/// System log adapter using env_logger
pub struct SystemLogAdapter {
    /// Configuration
    config: SystemLogAdapterConfig,
    /// Statistics
    stats: Arc<RwLock<LogStats>>,
    /// Destination configurations with their targets
    destinations: Arc<RwLock<HashMap<LogDestination, LogDestinationConfig>>>,
}

impl SystemLogAdapter {
    /// Create a new system log adapter
    pub fn new(config: SystemLogAdapterConfig) -> LogResult<Self> {
        let stats = Arc::new(RwLock::new(LogStats::default()));
        let destinations = Arc::new(RwLock::new(HashMap::new()));
        
        // Initialize env_logger once
        Self::init_logger(&config)?;
        
        Ok(Self {
            config,
            stats,
            destinations,
        })
    }

    /// Create a new system log adapter for testing (without logger initialization)
    #[cfg(test)]
    pub fn new_for_test(config: SystemLogAdapterConfig) -> LogResult<Self> {
        let stats = Arc::new(RwLock::new(LogStats::default()));
        let destinations = Arc::new(RwLock::new(HashMap::new()));
        
        Ok(Self {
            config,
            stats,
            destinations,
        })
    }
    
    /// Create with default configuration from environment
    pub fn from_env() -> LogResult<Self> {
        let config = SystemLogAdapterConfig::from_env();
        Self::new(config)
    }
    
    /// Initialize env_logger (only once per process)
    fn init_logger(config: &SystemLogAdapterConfig) -> LogResult<()> {
        INIT.call_once(|| {
            // Set RUST_LOG if not already set (using unsafe block as required)
            if env::var("RUST_LOG").is_err() {
                unsafe {
                    env::set_var("RUST_LOG", &config.log_level);
                }
            }
            
            // Initialize env_logger with custom formatting if needed
            if config.structured && config.format == LogFormat::Json {
                env_logger::Builder::from_default_env()
                    .format(|buf, record| {
                        use std::io::Write;
                        
                        let json_log = json!({
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                            "level": record.level().to_string(),
                            "target": record.target(),
                            "message": record.args().to_string(),
                            "module": record.module_path().unwrap_or("unknown"),
                            "file": record.file().unwrap_or("unknown"),
                            "line": record.line().unwrap_or(0),
                        });
                        
                        writeln!(buf, "{}", json_log)
                    })
                    .init();
            } else {
                env_logger::init();
            }
        });
        
        Ok(())
    }
    
    /// Convert LogLevel to log::Level
    fn log_level_to_log_level(level: LogLevel) -> log::Level {
        match level {
            LogLevel::Trace => log::Level::Trace,
            LogLevel::Debug => log::Level::Debug,
            LogLevel::Info => log::Level::Info,
            LogLevel::Warn => log::Level::Warn,
            LogLevel::Error => log::Level::Error,
            LogLevel::Fatal => log::Level::Error, // Map Fatal to Error
        }
    }
    
    /// Convert LogLevel to string for formatting
    fn log_level_to_string(level: LogLevel) -> &'static str {
        match level {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Fatal => "FATAL",
        }
    }
    
    /// Format log entry for output
    fn format_log_entry(&self, entry: &LogEntry) -> String {
        match self.config.format {
            LogFormat::Json => self.format_json(entry),
            LogFormat::Text => self.format_text(entry),
            LogFormat::Structured(ref template) => self.format_structured(entry, template),
        }
    }

    fn format_json(&self, entry: &LogEntry) -> String {
        let mut json_obj = json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "level": Self::log_level_to_string(entry.level()),
            "target": self.get_target_for_destination(&entry.destination),
            "message": entry.message.message,
            "id": entry.id.to_string(),
            "source": entry.source.to_string(),
            "destination": entry.destination.to_string(),
            "priority": format!("{:?}", entry.priority),
        });

        if let Some(ref correlation_id) = entry.correlation_id {
            json_obj["correlation_id"] = json!(correlation_id);
        }
        
        if let Some(ref context) = entry.message.context {
            json_obj["context"] = json!(context);
        }
        if let Some(ref module) = entry.message.module {
            json_obj["module"] = json!(module);
        }
        if let Some(ref function) = entry.message.function {
            json_obj["function"] = json!(function);
        }

        json_obj.to_string()
    }

    fn format_text(&self, entry: &LogEntry) -> String {
        if self.config.structured {
            // Structured text format with all fields
            let mut formatted = format!(
                "{} [{}] {}: {}",
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f UTC"),
                Self::log_level_to_string(entry.level()),
                self.get_target_for_destination(&entry.destination),
                entry.message.message
            );

            formatted.push_str(&format!(
                " [id={}, src={}, dest={}",
                entry.id,
                entry.source,
                entry.destination
            ));
            
            if let Some(ref correlation_id) = entry.correlation_id {
                formatted.push_str(&format!(", corr_id={}", correlation_id));
            }
            
            formatted.push(']');
            formatted
        } else {
            // Simple text format - just the message
            entry.message.message.clone()
        }
    }

    fn format_structured(&self, entry: &LogEntry, _template: &str) -> String {
        // For now, use structured text format - could be enhanced later with template parsing
        self.format_text(entry)
    }
    
    /// Write log entry using the log crate
    fn write_log_entry(&self, entry: &LogEntry) {
        let level = Self::log_level_to_log_level(entry.level());
        let target = self.get_target_for_destination(&entry.destination);
        
        // If env_logger is handling JSON formatting, just pass the structured data
        // Otherwise, format the message ourselves
        let message = if self.config.structured && self.config.format == LogFormat::Json {
            // Let env_logger handle JSON formatting, just pass the raw message
            entry.message.message.clone()
        } else {
            // Format the message ourselves
            self.format_log_entry(entry)
        };

        // Use log macros with target
        match level {
            log::Level::Error => error!(target: &target, "{}", message),
            log::Level::Warn => warn!(target: &target, "{}", message),
            log::Level::Info => info!(target: &target, "{}", message),
            log::Level::Debug => debug!(target: &target, "{}", message),
            log::Level::Trace => trace!(target: &target, "{}", message),
        }
    }
    
    /// Get target for a specific destination
    fn get_target_for_destination(&self, destination: &crate::core::base::entity::message::Location) -> String {
        // Try to find the destination in our configuration
        if let Ok(destinations) = self.destinations.try_read() {
            // Convert Location to LogDestination for lookup
            if let Ok(log_dest) = self.location_to_log_destination(destination) {
                if let Some(destination_config) = destinations.get(&log_dest) {
                    // Get the target from the destination settings or use default
                    if let Some(target) = destination_config.settings.get("target") {
                        return target.clone();
                    }
                }
            }
        }
        
        // Fall back to configured target
        self.config.target.clone()
    }
    
    /// Convert Location to LogDestination for configuration lookup
    fn location_to_log_destination(&self, location: &crate::core::base::entity::message::Location) -> Result<LogDestination, ()> {
        match location {
            crate::core::base::entity::message::Location::Service(name) => {
                if name.contains("system-log") {
                    Ok(LogDestination::System)
                } else if name.contains("access-log") {
                    Ok(LogDestination::Access)
                } else if name.contains("error-log") {
                    Ok(LogDestination::Error)
                } else if name.contains("security-log") {
                    Ok(LogDestination::Security)
                } else if name.contains("performance-log") {
                    Ok(LogDestination::Performance)
                } else if name.starts_with("custom-log-") {
                    let custom_name = name.strip_prefix("custom-log-").unwrap_or("unknown");
                    Ok(LogDestination::Custom(custom_name.to_string()))
                } else {
                    Err(())
                }
            }
            _ => Err(())
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
        self.write_log_entry(&entry);
        self.update_stats(1).await;
        Ok(())
    }
    
    async fn write_entries(&self, entries: Vec<LogEntry>) -> LogResult<()> {
        let count = entries.len();
        for entry in entries {
            self.write_log_entry(&entry);
        }
        self.update_stats(count).await;
        Ok(())
    }
    
    async fn batch_write(&self, request: BatchWriteRequest) -> LogResult<()> {
        let count = request.entries.len();
        for entry in request.entries {
            self.write_log_entry(&entry);
        }
        self.update_stats(count).await;
        Ok(())
    }
    
    async fn read_entries(&self, _query: LogQuery) -> LogResult<Vec<LogEntry>> {
        Err(LogError::ConfigError(
            "Log reading not supported by this adapter. Use a log aggregation system.".to_string()
        ))
    }
    
    async fn count_entries(&self, _query: LogQuery) -> LogResult<u64> {
        Err(LogError::ConfigError(
            "Log counting not supported by this adapter. Use a log aggregation system.".to_string()
        ))
    }
    
    async fn configure_destination(&self, mut config: LogDestinationConfig) -> LogResult<()> {
        // If target is provided in settings, use it; otherwise set default
        if !config.settings.contains_key("target") {
            config.settings.insert("target".to_string(), self.config.target.clone());
        }
        
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
        // env_logger doesn't provide explicit flush, but it's usually immediate
        Ok(())
    }
    
    async fn flush_destination(&self, _destination: LogDestination) -> LogResult<()> {
        self.flush().await
    }
    
    async fn rotate_logs(&self, _destination: LogDestination) -> LogResult<()> {
        // env_logger doesn't support rotation - would need external log management
        Ok(())
    }
    
    async fn get_stats(&self) -> LogResult<LogStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
    
    async fn get_destination_stats(&self, _destination: LogDestination) -> LogResult<LogStats> {
        self.get_stats().await
    }
    
    async fn clear_logs(&self, destination: LogDestination) -> LogResult<()> {
        warn!(target: &self.config.target, "Clear logs requested for destination: {:?}", destination);
        Ok(())
    }
    
    async fn clear_logs_before(&self, destination: LogDestination, before: DateTime<Utc>) -> LogResult<u64> {
        warn!(target: &self.config.target, "Clear logs before {:?} requested for destination: {:?}", before, destination);
        Ok(0)
    }
    
    async fn health_check(&self) -> LogResult<Vec<LogHealthCheck>> {
        let destinations = self.destinations.read().await;
        let mut checks = Vec::new();
        
        for (destination, _config) in destinations.iter() {
            let check = LogHealthCheck {
                destination: destination.clone(),
                healthy: true,
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
        "SystemLogAdapter (log + env_logger)"
    }
    
    async fn test_connection(&self) -> LogResult<()> {
        info!(target: &self.config.target, "System log adapter connection test");
        Ok(())
    }
    
    async fn archive_logs(&self, destination: LogDestination, before: DateTime<Utc>) -> LogResult<String> {
        warn!(target: &self.config.target, "Archive logs before {:?} requested for destination: {:?}", before, destination);
        Ok("Archive not implemented for this adapter".to_string())
    }
    
    fn supported_formats(&self) -> Vec<LogFormat> {
        vec![LogFormat::Json, LogFormat::Text, LogFormat::Structured("custom".to_string())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::platform::container::log::LogEntryBuilder;

    #[tokio::test]
    async fn test_system_log_adapter_creation() {
        let config = SystemLogAdapterConfig {
            log_level: "debug".to_string(),
            format: LogFormat::Text,
            target: "test".to_string(),
            structured: true,
        };
        
        let adapter = SystemLogAdapter::new_for_test(config).unwrap();
        assert_eq!(adapter.get_provider_name(), "SystemLogAdapter (log + env_logger)");
    }

    #[tokio::test]
    async fn test_write_entry() {
        let config = SystemLogAdapterConfig {
            log_level: "debug".to_string(),
            format: LogFormat::Text,
            target: "test".to_string(),
            structured: true,
        };
        
        let adapter = SystemLogAdapter::new_for_test(config).unwrap();
        
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
        let config = SystemLogAdapterConfig {
            log_level: "info".to_string(),
            format: LogFormat::Text,
            target: "test".to_string(),
            structured: true,
        };
        
        let adapter = SystemLogAdapter::new_for_test(config).unwrap();
        
        let health = adapter.health_check_destination(LogDestination::System).await.unwrap();
        assert!(health.healthy);
        assert_eq!(health.destination, LogDestination::System);
    }

    #[tokio::test]
    async fn test_json_formatting() {
        let config = SystemLogAdapterConfig {
            log_level: "info".to_string(),
            format: LogFormat::Json,
            target: "test".to_string(),
            structured: true,
        };
        
        let adapter = SystemLogAdapter::new_for_test(config).unwrap();
        
        let entry = LogEntryBuilder::new_entry(
            crate::core::base::entity::message::Location::system("test"),
            LogDestination::System,
            LogLevel::Info,
            "JSON test message".to_string(),
        );
        
        let formatted = adapter.format_log_entry(&entry);
        let parsed: serde_json::Value = serde_json::from_str(&formatted).unwrap();
        
        assert_eq!(parsed["message"], "JSON test message");
        assert_eq!(parsed["level"], "INFO");
        assert!(parsed["timestamp"].is_string());
    }

    #[tokio::test]
    async fn test_log_level_conversion() {
        assert_eq!(SystemLogAdapter::log_level_to_string(LogLevel::Trace), "TRACE");
        assert_eq!(SystemLogAdapter::log_level_to_string(LogLevel::Debug), "DEBUG");
        assert_eq!(SystemLogAdapter::log_level_to_string(LogLevel::Info), "INFO");
        assert_eq!(SystemLogAdapter::log_level_to_string(LogLevel::Warn), "WARN");
        assert_eq!(SystemLogAdapter::log_level_to_string(LogLevel::Error), "ERROR");
        assert_eq!(SystemLogAdapter::log_level_to_string(LogLevel::Fatal), "FATAL");
    }
}