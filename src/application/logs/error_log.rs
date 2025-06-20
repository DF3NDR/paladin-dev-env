/*
Error Log Module

This module defines the structure and functionality for an error log in the application.
It includes the definition of the ErrorLog struct, which contains a vector of LogEntry objects.
It also provides methods to add entries to the error log and to retrieve the log entries.
It is built on top of the platform level LogService.
*/
use std::sync::Arc;
use std::collections::HashMap;
use chrono::{DateTime, Utc};

use crate::core::platform::manager::log_service::{LogService, LogServiceConfig};
use crate::core::platform::container::log::{LogEntry, LogLevel, LogDestination};
use crate::core::base::entity::message::Location;
use crate::application::ports::output::log_port::{LogResult, LogQuery, LogStats};

/// Error severity levels
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl ErrorSeverity {
    /// Convert to log level
    pub fn to_log_level(&self) -> LogLevel {
        match self {
            ErrorSeverity::Low => LogLevel::Warn,
            ErrorSeverity::Medium => LogLevel::Error,
            ErrorSeverity::High => LogLevel::Error,
            ErrorSeverity::Critical => LogLevel::Fatal,
        }
    }
}

/// Error categories
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorCategory {
    /// Application logic errors
    Application,
    /// Database-related errors
    Database,
    /// Network and communication errors
    Network,
    /// Authentication and authorization errors
    Security,
    /// System resource errors
    System,
    /// External service errors
    ExternalService,
    /// Configuration errors
    Configuration,
    /// Validation errors
    Validation,
    /// Unknown or uncategorized errors
    Unknown,
}

impl ToString for ErrorCategory {
    fn to_string(&self) -> String {
        match self {
            ErrorCategory::Application => "application".to_string(),
            ErrorCategory::Database => "database".to_string(),
            ErrorCategory::Network => "network".to_string(),
            ErrorCategory::Security => "security".to_string(),
            ErrorCategory::System => "system".to_string(),
            ErrorCategory::ExternalService => "external_service".to_string(),
            ErrorCategory::Configuration => "configuration".to_string(),
            ErrorCategory::Validation => "validation".to_string(),
            ErrorCategory::Unknown => "unknown".to_string(),
        }
    }
}

/// Error log service for application errors and exceptions
pub struct ErrorLog {
    /// Underlying log service
    log_service: Arc<LogService>,
    /// Log destination for this service
    destination: LogDestination,
}

impl ErrorLog {
    /// Create a new error log service
    pub fn new(log_service: Arc<LogService>) -> Self {
        Self {
            log_service,
            destination: LogDestination::Error,
        }
    }
    
    /// Log a generic error
    pub async fn log_error(
        &self,
        severity: ErrorSeverity,
        category: ErrorCategory,
        message: &str,
        source: &str,
        error_code: Option<&str>,
        stack_trace: Option<&str>,
        context: Option<serde_json::Value>,
    ) -> LogResult<()> {
        let level = severity.to_log_level();
        
        let mut error_context = serde_json::json!({
            "severity": format!("{:?}", severity),
            "category": category.to_string(),
            "event_type": "error",
            "source": source
        });
        
        if let Some(code) = error_code {
            error_context["error_code"] = serde_json::Value::String(code.to_string());
        }
        
        if let Some(stack) = stack_trace {
            error_context["stack_trace"] = serde_json::Value::String(stack.to_string());
        }
        
        if let Some(ctx) = context {
            error_context["additional_context"] = ctx;
        }
        
        let entry = LogEntry::with_context(
            Location::system(source),
            self.destination.clone(),
            level,
            message.to_string(),
            Some("error".to_string()),
            Some("error_handler".to_string()),
            None,
            Some(error_context),
        );
        
        self.log_service.write_entry(entry).await
    }
    
    /// Log an exception with full details
    pub async fn log_exception(
        &self,
        exception_type: &str,
        message: &str,
        source: &str,
        stack_trace: &str,
        user_id: Option<&str>,
        request_id: Option<&str>,
    ) -> LogResult<()> {
        let mut context = serde_json::json!({
            "exception_type": exception_type,
            "stack_trace": stack_trace,
            "event_type": "exception",
            "source": source
        });
        
        if let Some(uid) = user_id {
            context["user_id"] = serde_json::Value::String(uid.to_string());
        }
        
        if let Some(req_id) = request_id {
            context["request_id"] = serde_json::Value::String(req_id.to_string());
        }
        
        let entry = LogEntry::with_context(
            Location::system(source),
            self.destination.clone(),
            LogLevel::Error,
            format!("{}: {}", exception_type, message),
            Some("error".to_string()),
            Some("exception_handler".to_string()),
            None,
            Some(context),
        );
        
        self.log_service.write_entry(entry).await
    }
    
    /// Log database errors
    pub async fn log_database_error(
        &self,
        operation: &str,
        table: Option<&str>,
        error_message: &str,
        sql_state: Option<&str>,
        query: Option<&str>,
    ) -> LogResult<()> {
        let message = format!("Database error during {}: {}", operation, error_message);
        
        let mut context = serde_json::json!({
            "operation": operation,
            "error_message": error_message,
            "category": "database",
            "event_type": "database_error"
        });
        
        if let Some(table) = table {
            context["table"] = serde_json::Value::String(table.to_string());
        }
        
        if let Some(state) = sql_state {
            context["sql_state"] = serde_json::Value::String(state.to_string());
        }
        
        if let Some(query) = query {
            context["query"] = serde_json::Value::String(query.to_string());
        }
        
        let entry = LogEntry::with_context(
            Location::system("database"),
            self.destination.clone(),
            LogLevel::Error,
            message,
            Some("error".to_string()),
            Some("database_error".to_string()),
            None,
            Some(context),
        );
        
        self.log_service.write_entry(entry).await
    }
    
    /// Log network errors
    pub async fn log_network_error(
        &self,
        operation: &str,
        endpoint: &str,
        error_message: &str,
        status_code: Option<u16>,
        timeout: Option<bool>,
    ) -> LogResult<()> {
        let message = format!("Network error during {} to {}: {}", operation, endpoint, error_message);
        
        let mut context = serde_json::json!({
            "operation": operation,
            "endpoint": endpoint,
            "error_message": error_message,
            "category": "network",
            "event_type": "network_error"
        });
        
        if let Some(code) = status_code {
            context["status_code"] = serde_json::Value::Number(code.into());
        }
        
        if let Some(timeout) = timeout {
            context["timeout"] = serde_json::Value::Bool(timeout);
        }
        
        let entry = LogEntry::with_context(
            Location::external(endpoint),
            self.destination.clone(),
            LogLevel::Error,
            message,
            Some("error".to_string()),
            Some("network_error".to_string()),
            None,
            Some(context),
        );
        
        self.log_service.write_entry(entry).await
    }
    
    /// Log security errors
    pub async fn log_security_error(
        &self,
        security_event: &str,
        user_id: Option<&str>,
        client_ip: Option<&str>,
        details: &str,
        blocked: bool,
    ) -> LogResult<()> {
        let level = if blocked { LogLevel::Warn } else { LogLevel::Error };
        let action = if blocked { "blocked" } else { "detected" };
        
        let message = format!("Security event {} {}: {}", security_event, action, details);
        
        let mut context = serde_json::json!({
            "security_event": security_event,
            "details": details,
            "blocked": blocked,
            "category": "security",
            "event_type": "security_error"
        });
        
        if let Some(uid) = user_id {
            context["user_id"] = serde_json::Value::String(uid.to_string());
        }
        
        if let Some(ip) = client_ip {
            context["client_ip"] = serde_json::Value::String(ip.to_string());
        }
        
        let entry = LogEntry::with_context(
            Location::system("security"),
            self.destination.clone(),
            level,
            message,
            Some("error".to_string()),
            Some("security_error".to_string()),
            None,
            Some(context),
        );
        
        self.log_service.write_entry(entry).await
    }
    
    /// Log validation errors
    pub async fn log_validation_error(
        &self,
        field: &str,
        value: Option<&str>,
        rule: &str,
        message: &str,
        context_data: Option<serde_json::Value>,
    ) -> LogResult<()> {
        let log_message = format!("Validation error for field '{}': {}", field, message);
        
        let mut context = serde_json::json!({
            "field": field,
            "validation_rule": rule,
            "validation_message": message,
            "category": "validation",
            "event_type": "validation_error"
        });
        
        if let Some(val) = value {
            context["field_value"] = serde_json::Value::String(val.to_string());
        }
        
        if let Some(ctx) = context_data {
            context["form_context"] = ctx;
        }
        
        let entry = LogEntry::with_context(
            Location::system("validator"),
            self.destination.clone(),
            LogLevel::Warn,
            log_message,
            Some("error".to_string()),
            Some("validation_error".to_string()),
            None,
            Some(context),
        );
        
        self.log_service.write_entry(entry).await
    }
    
    /// Log configuration errors
    pub async fn log_configuration_error(
        &self,
        component: &str,
        config_key: &str,
        error_message: &str,
        config_value: Option<&str>,
    ) -> LogResult<()> {
        let message = format!("Configuration error in '{}' for key '{}': {}", component, config_key, error_message);
        
        let mut context = serde_json::json!({
            "component": component,
            "config_key": config_key,
            "error_message": error_message,
            "category": "configuration",
            "event_type": "config_error"
        });
        
        if let Some(value) = config_value {
            context["config_value"] = serde_json::Value::String(value.to_string());
        }
        
        let entry = LogEntry::with_context(
            Location::system(component),
            self.destination.clone(),
            LogLevel::Error,
            message,
            Some("error".to_string()),
            Some("config_error".to_string()),
            None,
            Some(context),
        );
        
        self.log_service.write_entry(entry).await
    }
    
    /// Generic error log entry
    pub async fn log(
        &self,
        level: LogLevel,
        source: &str,
        message: &str,
        context: Option<serde_json::Value>,
    ) -> LogResult<()> {
        let entry = LogEntry::with_context(
            Location::system(source),
            self.destination.clone(),
            level,
            message.to_string(),
            Some("error".to_string()),
            None,
            None,
            context,
        );
        
        self.log_service.write_entry(entry).await
    }
    
    /// Get error log entries
    pub async fn get_entries(&self, query: LogQuery) -> LogResult<Vec<LogEntry>> {
        self.log_service.read_entries(self.destination.clone(), query).await
    }
    
    /// Get error statistics by category
    pub async fn get_category_stats(&self, since: Option<DateTime<Utc>>) -> LogResult<HashMap<String, u64>> {
        let mut query = LogQuery::default();
        if let Some(since) = since {
            query.start_time = Some(since);
        }
        
        let entries = self.get_entries(query).await?;
        let mut stats = HashMap::new();
        
        for entry in entries {
            if let Some(context) = &entry.message.context {
                if let Some(category) = context.get("category").and_then(|v| v.as_str()) {
                    *stats.entry(category.to_string()).or_insert(0) += 1;
                }
            }
        }
        
        Ok(stats)
    }
    
    /// Get error statistics by severity
    pub async fn get_severity_stats(&self, since: Option<DateTime<Utc>>) -> LogResult<HashMap<String, u64>> {
        let mut query = LogQuery::default();
        if let Some(since) = since {
            query.start_time = Some(since);
        }
        
        let entries = self.get_entries(query).await?;
        let mut stats = HashMap::new();
        
        for entry in entries {
            if let Some(context) = &entry.message.context {
                if let Some(severity) = context.get("severity").and_then(|v| v.as_str()) {
                    *stats.entry(severity.to_string()).or_insert(0) += 1;
                }
            }
        }
        
        Ok(stats)
    }
    
    /// Get error log statistics
    pub async fn get_stats(&self) -> LogResult<LogStats> {
        self.log_service.get_destination_stats(self.destination.clone()).await
    }
    
    /// Clear error logs
    pub async fn clear(&self) -> LogResult<()> {
        self.log_service.clear_logs(self.destination.clone()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_error_log_creation() {
        let log_service = Arc::new(LogService::new(LogServiceConfig::default()));
        log_service.initialize_default_logs().await.unwrap();
        
        let error_log = ErrorLog::new(log_service);
        
        error_log.log_error(
            ErrorSeverity::High,
            ErrorCategory::Application,
            "Test application error",
            "test_component",
            Some("APP_001"),
            None,
            None,
        ).await.unwrap();
        
        let entries = error_log.get_entries(LogQuery::default()).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].message.message.contains("Test application error"));
        assert_eq!(entries[0].level(), LogLevel::Error);
    }

    #[tokio::test]
    async fn test_exception_log() {
        let log_service = Arc::new(LogService::new(LogServiceConfig::default()));
        log_service.initialize_default_logs().await.unwrap();
        
        let error_log = ErrorLog::new(log_service);
        
        error_log.log_exception(
            "NullPointerException",
            "Attempted to access null object",
            "user_service",
            "line 42: user.getName()",
            Some("user123"),
            Some("req456"),
        ).await.unwrap();
        
        let entries = error_log.get_entries(LogQuery::default()).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].message.message.contains("NullPointerException"));
    }

    #[tokio::test]
    async fn test_database_error_log() {
        let log_service = Arc::new(LogService::new(LogServiceConfig::default()));
        log_service.initialize_default_logs().await.unwrap();
        
        let error_log = ErrorLog::new(log_service);
        
        error_log.log_database_error(
            "SELECT",
            Some("users"),
            "Connection timeout",
            Some("08001"),
            Some("SELECT * FROM users WHERE id = ?"),
        ).await.unwrap();
        
        let entries = error_log.get_entries(LogQuery::default()).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].message.message.contains("Database error during SELECT"));
    }

    #[tokio::test]
    async fn test_validation_error_log() {
        let log_service = Arc::new(LogService::new(LogServiceConfig::default()));
        log_service.initialize_default_logs().await.unwrap();
        
        let error_log = ErrorLog::new(log_service);
        
        error_log.log_validation_error(
            "email",
            Some("invalid-email"),
            "email_format",
            "Invalid email format",
            None,
        ).await.unwrap();
        
        let entries = error_log.get_entries(LogQuery::default()).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].message.message.contains("Validation error for field 'email'"));
        assert_eq!(entries[0].level(), LogLevel::Warn);
    }

    #[tokio::test]
    async fn test_error_severity_levels() {
        assert_eq!(ErrorSeverity::Low.to_log_level(), LogLevel::Warn);
        assert_eq!(ErrorSeverity::Critical.to_log_level(), LogLevel::Fatal);
    }

    #[tokio::test]
    async fn test_error_category_string_conversion() {
        assert_eq!(ErrorCategory::Database.to_string(), "database");
        assert_eq!(ErrorCategory::Security.to_string(), "security");
        assert_eq!(ErrorCategory::Unknown.to_string(), "unknown");
    }
}