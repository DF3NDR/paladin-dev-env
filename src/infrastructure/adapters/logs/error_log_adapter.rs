// /*
// Error Log Adapter

// Infrastructure adapter that implements the LogPort specifically for error logging.
// This adapter provides enhanced error logging capabilities including error categorization,
// severity levels, stack traces, and error correlation using the log crate with env_logger.

// Uses log for the logging facade and env_logger for configuration and output.
// Configuration is managed through environment variables with error-specific settings.
// */
// use std::env;
// use std::collections::HashMap;
// use std::sync::Arc;
// use std::sync::Once;
// use async_trait::async_trait;
// use chrono::{DateTime, Utc};
// use tokio::sync::RwLock;
// use log::{info, warn, error, debug, trace};
// use serde_json::json;

// use crate::application::ports::output::log_port::{
//     LogPort, LogResult, LogQuery, LogDestinationConfig, LogStats, 
//     LogHealthCheck, LogFormat, BatchWriteRequest
// };
// use crate::core::platform::container::log::{LogEntry, LogEntryBuilder, LogLevel, LogDestination, LogEntryExt};
// use crate::application::logs::error_log::{ErrorSeverity, ErrorCategory};

// /// Ensure env_logger is only initialized once for error logging
// static ERROR_INIT: Once = Once::new();

// /// Configuration for the error log adapter
// #[derive(Debug, Clone)]
// pub struct ErrorLogAdapterConfig {
//     /// Log level filter
//     pub log_level: String,
//     /// Output format (json, text, structured)
//     pub format: LogFormat,
//     /// Log target identifier
//     pub target: String,
//     /// Enable structured output
//     pub structured: bool,
//     /// Include stack traces in error logs
//     pub include_stack_traces: bool,
//     /// Maximum number of recent errors to keep in memory for analysis
//     pub error_cache_size: usize,
//     /// Enable error correlation tracking
//     pub enable_correlation: bool,
//     /// Minimum severity level to log
//     pub min_severity: ErrorSeverity,
// }

// impl Default for ErrorLogAdapterConfig {
//     fn default() -> Self {
//         Self {
//             log_level: "error".to_string(),
//             format: LogFormat::Json, // JSON is better for error analysis
//             target: "in4me-errors".to_string(),
//             structured: true,
//             include_stack_traces: true,
//             error_cache_size: 1000,
//             enable_correlation: true,
//             min_severity: ErrorSeverity::Low,
//         }
//     }
// }

// impl ErrorLogAdapterConfig {
//     /// Load configuration from environment variables
//     pub fn from_env() -> Self {
//         Self {
//             log_level: env::var("ERROR_LOG_LEVEL").unwrap_or_else(|_| "error".to_string()),
//             format: match env::var("ERROR_LOG_FORMAT").unwrap_or_else(|_| "json".to_string()).as_str() {
//                 "json" => LogFormat::Json,
//                 "text" => LogFormat::Text,
//                 template => LogFormat::Structured(template.to_string()),
//             },
//             target: env::var("ERROR_LOG_TARGET").unwrap_or_else(|_| "in4me-errors".to_string()),
//             structured: env::var("ERROR_LOG_STRUCTURED")
//                 .unwrap_or_else(|_| "true".to_string())
//                 .parse()
//                 .unwrap_or(true),
//             include_stack_traces: env::var("ERROR_LOG_STACK_TRACES")
//                 .unwrap_or_else(|_| "true".to_string())
//                 .parse()
//                 .unwrap_or(true),
//             error_cache_size: env::var("ERROR_LOG_CACHE_SIZE")
//                 .unwrap_or_else(|_| "1000".to_string())
//                 .parse()
//                 .unwrap_or(1000),
//             enable_correlation: env::var("ERROR_LOG_CORRELATION")
//                 .unwrap_or_else(|_| "true".to_string())
//                 .parse()
//                 .unwrap_or(true),
//             min_severity: match env::var("ERROR_LOG_MIN_SEVERITY")
//                 .unwrap_or_else(|_| "low".to_string())
//                 .to_lowercase()
//                 .as_str() {
//                 "critical" => ErrorSeverity::Critical,
//                 "high" => ErrorSeverity::High,
//                 "medium" => ErrorSeverity::Medium,
//                 _ => ErrorSeverity::Low,
//             },
//         }
//     }
// }

// /// Error information for enhanced tracking
// #[derive(Debug, Clone)]
// pub struct ErrorInfo {
//     pub severity: ErrorSeverity,
//     pub category: ErrorCategory,
//     pub error_code: Option<String>,
//     pub stack_trace: Option<String>,
//     pub user_id: Option<String>,
//     pub request_id: Option<String>,
//     pub session_id: Option<String>,
//     pub additional_context: Option<serde_json::Value>,
// }

// /// Error log adapter using env_logger with enhanced error features
// pub struct ErrorLogAdapter {
//     /// Configuration
//     config: ErrorLogAdapterConfig,
//     /// Statistics
//     stats: Arc<RwLock<LogStats>>,
//     /// Destination configurations with their targets
//     destinations: Arc<RwLock<HashMap<LogDestination, LogDestinationConfig>>>,
//     /// Recent errors cache for analysis
//     error_cache: Arc<RwLock<Vec<LogEntry>>>,
//     /// Error correlation tracking
//     error_correlations: Arc<RwLock<HashMap<String, Vec<String>>>>,
// }

// impl ErrorLogAdapter {
//     /// Create a new error log adapter
//     pub fn new(config: ErrorLogAdapterConfig) -> LogResult<Self> {
//         let stats = Arc::new(RwLock::new(LogStats::default()));
//         let destinations = Arc::new(RwLock::new(HashMap::new()));
//         let error_cache = Arc::new(RwLock::new(Vec::new()));
//         let error_correlations = Arc::new(RwLock::new(HashMap::new()));
        
//         // Initialize env_logger once for error logging
//         Self::init_logger(&config)?;
        
//         Ok(Self {
//             config,
//             stats,
//             destinations,
//             error_cache,
//             error_correlations,
//         })
//     }

//     /// Create a new error log adapter for testing (without logger initialization)
//     #[cfg(test)]
//     pub fn new_for_test(config: ErrorLogAdapterConfig) -> LogResult<Self> {
//         let stats = Arc::new(RwLock::new(LogStats::default()));
//         let destinations = Arc::new(RwLock::new(HashMap::new()));
//         let error_cache = Arc::new(RwLock::new(Vec::new()));
//         let error_correlations = Arc::new(RwLock::new(HashMap::new()));
        
//         Ok(Self {
//             config,
//             stats,
//             destinations,
//             error_cache,
//             error_correlations,
//         })
//     }
    
//     /// Create with default configuration from environment
//     pub fn from_env() -> LogResult<Self> {
//         let config = ErrorLogAdapterConfig::from_env();
//         Self::new(config)
//     }
    
//     /// Initialize env_logger for error logging (only once per process)
//     fn init_logger(config: &ErrorLogAdapterConfig) -> LogResult<()> {
//         ERROR_INIT.call_once(|| {
//             // Set RUST_LOG if not already set (using unsafe block as required)
//             if env::var("RUST_LOG").is_err() {
//                 unsafe {
//                     env::set_var("RUST_LOG", &format!("{}={}", config.target, config.log_level));
//                 }
//             }
            
//             // Initialize env_logger with error-specific formatting
//             if config.structured && config.format == LogFormat::Json {
//                 env_logger::Builder::from_default_env()
//                     .format(|buf, record| {
//                         use std::io::Write;
                        
//                         let json_log = json!({
//                             "timestamp": chrono::Utc::now().to_rfc3339(),
//                             "level": record.level().to_string(),
//                             "target": record.target(),
//                             "message": record.args().to_string(),
//                             "module": record.module_path().unwrap_or("unknown"),
//                             "file": record.file().unwrap_or("unknown"),
//                             "line": record.line().unwrap_or(0),
//                             "log_type": "error",
//                         });
                        
//                         writeln!(buf, "{}", json_log)
//                     })
//                     .init();
//             } else {
//                 env_logger::Builder::from_default_env()
//                     .format(|buf, record| {
//                         use std::io::Write;
//                         writeln!(buf, "[ERROR] {} [{}] {}: {}", 
//                             chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f UTC"),
//                             record.level(),
//                             record.target(), 
//                             record.args())
//                     })
//                     .init();
//             }
//         });
        
//         Ok(())
//     }
    
//     /// Convert LogLevel to log::Level
//     fn log_level_to_log_level(level: LogLevel) -> log::Level {
//         match level {
//             LogLevel::Trace => log::Level::Trace,
//             LogLevel::Debug => log::Level::Debug,
//             LogLevel::Info => log::Level::Info,
//             LogLevel::Warn => log::Level::Warn,
//             LogLevel::Error => log::Level::Error,
//             LogLevel::Fatal => log::Level::Error, // Map Fatal to Error
//         }
//     }
    
//     /// Convert LogLevel to string for formatting
//     fn log_level_to_string(level: LogLevel) -> &'static str {
//         match level {
//             LogLevel::Trace => "TRACE",
//             LogLevel::Debug => "DEBUG",
//             LogLevel::Info => "INFO",
//             LogLevel::Warn => "WARN",
//             LogLevel::Error => "ERROR",
//             LogLevel::Fatal => "FATAL",
//         }
//     }

//     /// Convert ErrorSeverity to LogLevel
//     fn severity_to_log_level(severity: ErrorSeverity) -> LogLevel {
//         match severity {
//             ErrorSeverity::Low => LogLevel::Warn,
//             ErrorSeverity::Medium => LogLevel::Error,
//             ErrorSeverity::High => LogLevel::Error,
//             ErrorSeverity::Critical => LogLevel::Fatal,
//         }
//     }
    
//     /// Format log entry for output with error-specific enhancements
//     fn format_log_entry(&self, entry: &LogEntry) -> String {
//         match self.config.format {
//             LogFormat::Json => self.format_json(entry),
//             LogFormat::Text => self.format_text(entry),
//             LogFormat::Structured(ref template) => self.format_structured(entry, template),
//         }
//     }

//     fn format_json(&self, entry: &LogEntry) -> String {
//         let mut json_obj = json!({
//             "timestamp": chrono::Utc::now().to_rfc3339(),
//             "level": Self::log_level_to_string(entry.level()),
//             "target": self.get_target_for_destination(&entry.destination),
//             "message": entry.message.message,
//             "id": entry.id.to_string(),
//             "source": entry.source.to_string(),
//             "destination": entry.destination.to_string(),
//             "priority": format!("{:?}", entry.priority),
//             "log_type": "error",
//         });

//         if let Some(ref correlation_id) = entry.correlation_id {
//             json_obj["correlation_id"] = json!(correlation_id);
//         }
        
//         if let Some(ref context) = entry.message.context {
//             json_obj["context"] = json!(context);
            
//             // Extract error-specific fields from context
//             if let Some(severity) = context.get("severity") {
//                 json_obj["error_severity"] = json!(severity);
//             }
//             if let Some(category) = context.get("category") {
//                 json_obj["error_category"] = json!(category);
//             }
//             if let Some(error_code) = context.get("error_code") {
//                 json_obj["error_code"] = json!(error_code);
//             }
//             if let Some(stack_trace) = context.get("stack_trace") {
//                 json_obj["stack_trace"] = json!(stack_trace);
//             }
//             if let Some(user_id) = context.get("user_id") {
//                 json_obj["user_id"] = json!(user_id);
//             }
//             if let Some(request_id) = context.get("request_id") {
//                 json_obj["request_id"] = json!(request_id);
//             }
//         }
        
//         if let Some(ref module) = entry.message.module {
//             json_obj["module"] = json!(module);
//         }
//         if let Some(ref function) = entry.message.function {
//             json_obj["function"] = json!(function);
//         }

//         json_obj.to_string()
//     }

//     fn format_text(&self, entry: &LogEntry) -> String {
//         if self.config.structured {
//             let mut formatted = format!(
//                 "{} [{}] [ERROR] {}: {}",
//                 chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f UTC"),
//                 Self::log_level_to_string(entry.level()),
//                 self.get_target_for_destination(&entry.destination),
//                 entry.message.message
//             );

//             formatted.push_str(&format!(
//                 " [id={}, src={}, dest={}",
//                 entry.id,
//                 entry.source,
//                 entry.destination
//             ));
            
//             if let Some(ref correlation_id) = entry.correlation_id {
//                 formatted.push_str(&format!(", corr_id={}", correlation_id));
//             }
            
//             // Add error-specific information
//             if let Some(ref context) = entry.message.context {
//                 if let Some(severity) = context.get("severity").and_then(|s| s.as_str()) {
//                     formatted.push_str(&format!(", severity={}", severity));
//                 }
//                 if let Some(category) = context.get("category").and_then(|c| c.as_str()) {
//                     formatted.push_str(&format!(", category={}", category));
//                 }
//                 if let Some(error_code) = context.get("error_code").and_then(|c| c.as_str()) {
//                     formatted.push_str(&format!(", code={}", error_code));
//                 }
//             }
            
//             formatted.push(']');
//             formatted
//         } else {
//             // Simple format for errors
//             format!("[ERROR] {}", entry.message.message)
//         }
//     }

//     fn format_structured(&self, entry: &LogEntry, _template: &str) -> String {
//         // For now, use structured text format - could be enhanced later with template parsing
//         self.format_text(entry)
//     }
    
//     /// Write log entry using the log crate with error-specific handling
//     fn write_log_entry(&self, entry: &LogEntry) {
//         let level = Self::log_level_to_log_level(entry.level());
//         let target = self.get_target_for_destination(&entry.destination);
        
//         // Format the message for error logging
//         let message = if self.config.structured && self.config.format == LogFormat::Json {
//             // Let env_logger handle JSON formatting, just pass the raw message
//             entry.message.message.clone()
//         } else {
//             // Format the message ourselves
//             self.format_log_entry(entry)
//         };

//         // Use log macros with target - errors typically use error! or warn!
//         match level {
//             log::Level::Error => error!(target: &target, "{}", message),
//             log::Level::Warn => warn!(target: &target, "{}", message),
//             log::Level::Info => info!(target: &target, "{}", message),
//             log::Level::Debug => debug!(target: &target, "{}", message),
//             log::Level::Trace => trace!(target: &target, "{}", message),
//         }
//     }
    
//     /// Get target for a specific destination
//     fn get_target_for_destination(&self, destination: &crate::core::base::entity::message::Location) -> String {
//         // Try to find the destination in our configuration
//         if let Ok(destinations) = self.destinations.try_read() {
//             // Convert Location to LogDestination for lookup
//             if let Ok(log_dest) = self.location_to_log_destination(destination) {
//                 if let Some(destination_config) = destinations.get(&log_dest) {
//                     // Get the target from the destination settings or use default
//                     if let Some(target) = destination_config.settings.get("target") {
//                         return target.clone();
//                     }
//                 }
//             }
//         }
        
//         // Fall back to configured target
//         self.config.target.clone()
//     }
    
//     /// Convert Location to LogDestination for configuration lookup
//     fn location_to_log_destination(&self, location: &crate::core::base::entity::message::Location) -> Result<LogDestination, ()> {
//         match location {
//             crate::core::base::entity::message::Location::Service(name) => {
//                 if name.contains("error-log") || name.contains("error") {
//                     Ok(LogDestination::Error)
//                 } else if name.contains("system-log") {
//                     Ok(LogDestination::System)
//                 } else if name.contains("security-log") {
//                     Ok(LogDestination::Security)
//                 } else if name.starts_with("custom-error-") {
//                     let custom_name = name.strip_prefix("custom-error-").unwrap_or("unknown");
//                     Ok(LogDestination::Custom(format!("error-{}", custom_name)))
//                 } else {
//                     Ok(LogDestination::Error) // Default to error for this adapter
//                 }
//             }
//             _ => Ok(LogDestination::Error) // Default fallback
//         }
//     }
    
//     /// Update statistics with error-specific tracking
//     async fn update_stats(&self, entries_count: usize) {
//         let mut stats = self.stats.write().await;
//         stats.entries_written += entries_count as u64;
//         stats.last_write = Some(Utc::now());
//         stats.errors += entries_count as u64; // Track as errors
//     }

//     /// Cache error for analysis
//     async fn cache_error(&self, entry: &LogEntry) {
//         if self.config.error_cache_size > 0 {
//             let mut cache = self.error_cache.write().await;
//             cache.push(entry.clone());
            
//             // Keep cache size under limit
//             if cache.len() > self.config.error_cache_size {
//                 cache.remove(0);
//             }
//         }
//     }

//     /// Track error correlation
//     async fn track_correlation(&self, entry: &LogEntry) {
//         if self.config.enable_correlation {
//             if let Some(ref correlation_id) = entry.correlation_id {
//                 let mut correlations = self.error_correlations.write().await;
//                 let correlation_key = correlation_id.to_string();
                
//                 correlations
//                     .entry(correlation_key)
//                     .or_insert_with(Vec::new)
//                     .push(entry.id.to_string());
//             }
//         }
//     }

//     /// Get recent errors from cache
//     pub async fn get_recent_errors(&self, limit: Option<usize>) -> Vec<LogEntry> {
//         let cache = self.error_cache.read().await;
//         let take_count = limit.unwrap_or(cache.len()).min(cache.len());
//         cache.iter().rev().take(take_count).cloned().collect()
//     }

//     /// Get correlated errors
//     pub async fn get_correlated_errors(&self, correlation_id: &str) -> Vec<String> {
//         let correlations = self.error_correlations.read().await;
//         correlations.get(correlation_id).cloned().unwrap_or_default()
//     }
    
//     /// Log enhanced error with additional context
//     pub async fn log_enhanced_error(&self, error_info: ErrorInfo, message: String, source: &str) -> LogResult<()> {
//         // Check if we should log this severity level
//         if !self.should_log_severity(&error_info.severity) {
//             return Ok(());
//         }

//         let level = Self::severity_to_log_level(error_info.severity.clone());
        
//         // Build enhanced context
//         let mut context = json!({
//             "severity": format!("{:?}", error_info.severity),
//             "category": error_info.category.to_string(),
//             "event_type": "enhanced_error",
//             "source": source,
//         });

//         if let Some(ref error_code) = error_info.error_code {
//             context["error_code"] = json!(error_code);
//         }
        
//         if self.config.include_stack_traces {
//             if let Some(ref stack_trace) = error_info.stack_trace {
//                 context["stack_trace"] = json!(stack_trace);
//             }
//         }
        
//         if let Some(ref user_id) = error_info.user_id {
//             context["user_id"] = json!(user_id);
//         }
        
//         if let Some(ref request_id) = error_info.request_id {
//             context["request_id"] = json!(request_id);
//         }
        
//         if let Some(ref session_id) = error_info.session_id {
//             context["session_id"] = json!(session_id);
//         }
        
//         if let Some(ref additional) = error_info.additional_context {
//             context["additional_context"] = additional.clone();
//         }

//         // Create LogEntry using LogEntryBuilder or the proper constructor
//         let mut entry = LogEntryBuilder::new_entry(
//             crate::core::base::entity::message::Location::system(source),
//             LogDestination::Error,
//             level,
//             message,
//         );

//         // Add the enhanced context to the entry
//         entry.message.context = Some(context);
//         entry.message.module = Some("error".to_string());
//         entry.message.function = Some("enhanced_error".to_string());

//         // Cache and track correlation
//         self.cache_error(&entry).await;
//         self.track_correlation(&entry).await;

//         // Write the entry
//         self.write_entry(entry).await
//     }

//     /// Check if we should log this severity level
//     fn should_log_severity(&self, severity: &ErrorSeverity) -> bool {

        
//         let severity_order = |s: &ErrorSeverity| match s {
//             ErrorSeverity::Low => 0,
//             ErrorSeverity::Medium => 1,
//             ErrorSeverity::High => 2,
//             ErrorSeverity::Critical => 3,
//         };
        
//         severity_order(severity) >= severity_order(&self.config.min_severity)
//     }
// }

// #[async_trait]
// impl LogPort for ErrorLogAdapter {
//     async fn write_entry(&self, entry: LogEntry) -> LogResult<()> {
//         // Cache and track correlation for all entries
//         self.cache_error(&entry).await;
//         self.track_correlation(&entry).await;
        
//         self.write_log_entry(&entry);
//         self.update_stats(1).await;
//         Ok(())
//     }
    
//     async fn write_entries(&self, entries: Vec<LogEntry>) -> LogResult<()> {
//         let count = entries.len();
//         for entry in entries {
//             self.cache_error(&entry).await;
//             self.track_correlation(&entry).await;
//             self.write_log_entry(&entry);
//         }
//         self.update_stats(count).await;
//         Ok(())
//     }
    
//     async fn batch_write(&self, request: BatchWriteRequest) -> LogResult<()> {
//         let count = request.entries.len();
//         for entry in request.entries {
//             self.cache_error(&entry).await;
//             self.track_correlation(&entry).await;
//             self.write_log_entry(&entry);
//         }
//         self.update_stats(count).await;
//         Ok(())
//     }
    
//     async fn read_entries(&self, _query: LogQuery) -> LogResult<Vec<LogEntry>> {
//         // Return recent cached errors instead of unsupported operation
//         Ok(self.get_recent_errors(Some(100)).await)
//     }
    
//     async fn count_entries(&self, _query: LogQuery) -> LogResult<u64> {
//         let cache = self.error_cache.read().await;
//         Ok(cache.len() as u64)
//     }
    
//     async fn configure_destination(&self, mut config: LogDestinationConfig) -> LogResult<()> {
//         // If target is provided in settings, use it; otherwise set default
//         if !config.settings.contains_key("target") {
//             config.settings.insert("target".to_string(), self.config.target.clone());
//         }
        
//         let mut destinations = self.destinations.write().await;
//         destinations.insert(config.destination.clone(), config);
//         Ok(())
//     }
    
//     async fn remove_destination(&self, destination: LogDestination) -> LogResult<()> {
//         let mut destinations = self.destinations.write().await;
//         destinations.remove(&destination);
//         Ok(())
//     }
    
//     async fn list_destinations(&self) -> LogResult<Vec<LogDestination>> {
//         let destinations = self.destinations.read().await;
//         Ok(destinations.keys().cloned().collect())
//     }
    
//     async fn flush(&self) -> LogResult<()> {
//         // env_logger doesn't provide explicit flush, but it's usually immediate
//         Ok(())
//     }
    
//     async fn flush_destination(&self, _destination: LogDestination) -> LogResult<()> {
//         self.flush().await
//     }
    
//     async fn rotate_logs(&self, _destination: LogDestination) -> LogResult<()> {
//         // env_logger doesn't support rotation - would need external log management
//         Ok(())
//     }
    
//     async fn get_stats(&self) -> LogResult<LogStats> {
//         let stats = self.stats.read().await;
//         Ok(stats.clone())
//     }
    
//     async fn get_destination_stats(&self, _destination: LogDestination) -> LogResult<LogStats> {
//         self.get_stats().await
//     }
    
//     async fn clear_logs(&self, destination: LogDestination) -> LogResult<()> {
//         error!(target: &self.config.target, "Clear error logs requested for destination: {:?}", destination);
        
//         // Clear the error cache
//         let mut cache = self.error_cache.write().await;
//         cache.clear();
        
//         // Clear correlations
//         let mut correlations = self.error_correlations.write().await;
//         correlations.clear();
        
//         Ok(())
//     }
    
//     async fn clear_logs_before(&self, destination: LogDestination, before: DateTime<Utc>) -> LogResult<u64> {
//         error!(target: &self.config.target, "Clear error logs before {:?} requested for destination: {:?}", before, destination);
        
//         // Remove entries from cache that are older than 'before'
//         let mut cache = self.error_cache.write().await;
//         let original_count = cache.len() as u64;
//         cache.retain(|entry| entry.timestamp > before);
//         let removed_count = original_count - (cache.len() as u64);
        
//         Ok(removed_count)
//     }
    
//     async fn health_check(&self) -> LogResult<Vec<LogHealthCheck>> {
//         let destinations = self.destinations.read().await;
//         let mut checks = Vec::new();
        
//         for (destination, _config) in destinations.iter() {
//             let check = LogHealthCheck {
//                 destination: destination.clone(),
//                 healthy: true,
//                 last_write: {
//                     let stats = self.stats.read().await;
//                     stats.last_write
//                 },
//                 error_message: None,
//                 response_time: Some(std::time::Duration::from_millis(1)),
//             };
//             checks.push(check);
//         }
        
//         Ok(checks)
//     }
    
//     async fn health_check_destination(&self, destination: LogDestination) -> LogResult<LogHealthCheck> {
//         let check = LogHealthCheck {
//             destination,
//             healthy: true,
//             last_write: {
//                 let stats = self.stats.read().await;
//                 stats.last_write
//             },
//             error_message: None,
//             response_time: Some(std::time::Duration::from_millis(1)),
//         };
//         Ok(check)
//     }
    
//     fn get_provider_name(&self) -> &'static str {
//         "ErrorLogAdapter (log + env_logger)"
//     }
    
//     async fn test_connection(&self) -> LogResult<()> {
//         error!(target: &self.config.target, "Error log adapter connection test");
//         Ok(())
//     }
    
//     async fn archive_logs(&self, destination: LogDestination, before: DateTime<Utc>) -> LogResult<String> {
//         error!(target: &self.config.target, "Archive error logs before {:?} requested for destination: {:?}", before, destination);
//         Ok("Error log archive not implemented for this adapter".to_string())
//     }
    
//     fn supported_formats(&self) -> Vec<LogFormat> {
//         vec![LogFormat::Json, LogFormat::Text, LogFormat::Structured("error-template".to_string())]
//     }
// }

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::core::platform::container::log::LogEntryBuilder;

//     #[tokio::test]
//     async fn test_error_log_adapter_creation() {
//         let config = ErrorLogAdapterConfig {
//             log_level: "error".to_string(),
//             format: LogFormat::Json,
//             target: "test-errors".to_string(),
//             structured: true,
//             include_stack_traces: true,
//             error_cache_size: 100,
//             enable_correlation: true,
//             min_severity: ErrorSeverity::Low,
//         };
        
//         let adapter = ErrorLogAdapter::new_for_test(config).unwrap();
//         assert_eq!(adapter.get_provider_name(), "ErrorLogAdapter (log + env_logger)");
//     }

//     #[tokio::test]
//     async fn test_enhanced_error_logging() {
//         let config = ErrorLogAdapterConfig::default();
//         let adapter = ErrorLogAdapter::new_for_test(config).unwrap();
        
//         let error_info = ErrorInfo {
//             severity: ErrorSeverity::High,
//             category: ErrorCategory::Application,
//             error_code: Some("APP_001".to_string()),
//             stack_trace: Some("line 42: error occurred".to_string()),
//             user_id: Some("user123".to_string()),
//             request_id: Some("req456".to_string()),
//             session_id: Some("sess789".to_string()),
//             additional_context: Some(json!({"module": "user_service"})),
//         };
        
//         let result = adapter.log_enhanced_error(
//             error_info,
//             "Test enhanced error".to_string(),
//             "test_component"
//         ).await;
        
//         assert!(result.is_ok());
        
//         let stats = adapter.get_stats().await.unwrap();
//         assert_eq!(stats.entries_written, 1);
        
//         let recent_errors = adapter.get_recent_errors(Some(1)).await;
//         assert_eq!(recent_errors.len(), 1);
//         assert!(recent_errors[0].message.message.contains("Test enhanced error"));
//     }

//     #[tokio::test]
//     async fn test_error_severity_filtering() {
//         let mut config = ErrorLogAdapterConfig::default();
//         config.min_severity = ErrorSeverity::High;
        
//         let adapter = ErrorLogAdapter::new_for_test(config).unwrap();
        
//         // This should be logged (High >= High)
//         let high_error = ErrorInfo {
//             severity: ErrorSeverity::High,
//             category: ErrorCategory::System,
//             error_code: None,
//             stack_trace: None,
//             user_id: None,
//             request_id: None,
//             session_id: None,
//             additional_context: None,
//         };
        
//         // This should not be logged (Low < High)
//         let low_error = ErrorInfo {
//             severity: ErrorSeverity::Low,
//             category: ErrorCategory::System,
//             error_code: None,
//             stack_trace: None,
//             user_id: None,
//             request_id: None,
//             session_id: None,
//             additional_context: None,
//         };
        
//         adapter.log_enhanced_error(high_error, "High severity error".to_string(), "test").await.unwrap();
//         adapter.log_enhanced_error(low_error, "Low severity error".to_string(), "test").await.unwrap(); // Should be filtered out
        
//         let stats = adapter.get_stats().await.unwrap();
//         assert_eq!(stats.entries_written, 1); // Only high severity should be logged
//     }

//     #[tokio::test]
//     async fn test_error_cache() {
//         let mut config = ErrorLogAdapterConfig::default();
//         config.error_cache_size = 2; // Small cache for testing
        
//         let adapter = ErrorLogAdapter::new_for_test(config).unwrap();
        
//         // Add 3 errors to test cache size limit
//         for i in 0..3 {
//             let entry = LogEntryBuilder::new_entry(
//                 crate::core::base::entity::message::Location::system("test"),
//                 LogDestination::Error,
//                 LogLevel::Error,
//                 format!("Error {}", i),
//             );
//             adapter.write_entry(entry).await.unwrap();
//         }
        
//         let recent_errors = adapter.get_recent_errors(None).await;
//         assert_eq!(recent_errors.len(), 2); // Should be limited by cache size
//     }

//     #[tokio::test]
//     async fn test_json_error_formatting() {
//         let config = ErrorLogAdapterConfig {
//             format: LogFormat::Json,
//             structured: true,
//             ..Default::default()
//         };
        
//         let adapter = ErrorLogAdapter::new_for_test(config).unwrap();
        
//         let mut entry = LogEntryBuilder::new_entry(
//             crate::core::base::entity::message::Location::system("test"),
//             LogDestination::Error,
//             LogLevel::Error,
//             "JSON error test".to_string(),
//         );
        
//         // Add error context
//         entry.message.context = Some(json!({
//             "severity": "High",
//             "category": "application",
//             "error_code": "TEST_001",
//             "stack_trace": "line 1: error"
//         }));
        
//         let formatted = adapter.format_log_entry(&entry);
//         let parsed: serde_json::Value = serde_json::from_str(&formatted).unwrap();
        
//         assert_eq!(parsed["message"], "JSON error test");
//         assert_eq!(parsed["level"], "ERROR");
//         assert_eq!(parsed["log_type"], "error");
//         assert_eq!(parsed["error_severity"], "High");
//         assert_eq!(parsed["error_category"], "application");
//         assert_eq!(parsed["error_code"], "TEST_001");
//         assert!(parsed["timestamp"].is_string());
//     }
// }