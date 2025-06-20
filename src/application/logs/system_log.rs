/*
System Log Module

This module defines the structure and functionality for a system log in the application.
It includes the definition of the SystemLog struct, which contains a vector of LogEntry objects.
It also provides methods to add entries to the system log and to retrieve the log entries.
It is built on top of the platform level LogService.
*/
use std::sync::Arc;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::core::platform::manager::log_service::{LogService, LogServiceConfig};
use crate::core::platform::container::log::{LogEntry, LogLevel, LogDestination};
use crate::core::base::entity::message::Location;
use crate::application::ports::output::log_port::{LogResult, LogQuery, LogStats};

/// System log service for general system operations
pub struct SystemLog {
    /// Underlying log service
    log_service: Arc<LogService>,
    /// Log destination for this service
    destination: LogDestination,
}

impl SystemLog {
    /// Create a new system log service
    pub fn new(log_service: Arc<LogService>) -> Self {
        Self {
            log_service,
            destination: LogDestination::System,
        }
    }
    
    /// Log system startup event
    pub async fn log_startup(&self, component: &str, version: &str) -> LogResult<()> {
        let entry = LogEntry::with_context(
            Location::system(component),
            self.destination.clone(),
            LogLevel::Info,
            format!("System component '{}' started successfully", component),
            Some("system".to_string()),
            Some("startup".to_string()),
            None,
            Some(serde_json::json!({
                "component": component,
                "version": version,
                "event_type": "startup"
            })),
        );
        
        self.log_service.write_entry(entry).await
    }
    
    /// Log system shutdown event
    pub async fn log_shutdown(&self, component: &str, reason: Option<&str>) -> LogResult<()> {
        let message = match reason {
            Some(reason) => format!("System component '{}' shutting down: {}", component, reason),
            None => format!("System component '{}' shutting down", component),
        };
        
        let mut context = serde_json::json!({
            "component": component,
            "event_type": "shutdown"
        });
        
        if let Some(reason) = reason {
            context["reason"] = serde_json::Value::String(reason.to_string());
        }
        
        let entry = LogEntry::with_context(
            Location::system(component),
            self.destination.clone(),
            LogLevel::Info,
            message,
            Some("system".to_string()),
            Some("shutdown".to_string()),
            None,
            Some(context),
        );
        
        self.log_service.write_entry(entry).await
    }
    
    /// Log configuration changes
    pub async fn log_config_change(
        &self,
        component: &str,
        setting: &str,
        old_value: Option<&str>,
        new_value: &str,
    ) -> LogResult<()> {
        let message = format!(
            "Configuration changed for '{}': {} = {}",
            component, setting, new_value
        );
        
        let mut context = serde_json::json!({
            "component": component,
            "setting": setting,
            "new_value": new_value,
            "event_type": "config_change"
        });
        
        if let Some(old_value) = old_value {
            context["old_value"] = serde_json::Value::String(old_value.to_string());
        }
        
        let entry = LogEntry::with_context(
            Location::system(component),
            self.destination.clone(),
            LogLevel::Info,
            message,
            Some("system".to_string()),
            Some("config_change".to_string()),
            None,
            Some(context),
        );
        
        self.log_service.write_entry(entry).await
    }
    
    /// Log service health status
    pub async fn log_health_status(
        &self,
        service: &str,
        healthy: bool,
        details: Option<&str>,
    ) -> LogResult<()> {
        let level = if healthy { LogLevel::Info } else { LogLevel::Warn };
        let status = if healthy { "healthy" } else { "unhealthy" };
        
        let message = format!("Service '{}' health check: {}", service, status);
        
        let mut context = serde_json::json!({
            "service": service,
            "healthy": healthy,
            "event_type": "health_check"
        });
        
        if let Some(details) = details {
            context["details"] = serde_json::Value::String(details.to_string());
        }
        
        let entry = LogEntry::with_context(
            Location::system("health_monitor"),
            self.destination.clone(),
            level,
            message,
            Some("system".to_string()),
            Some("health_check".to_string()),
            None,
            Some(context),
        );
        
        self.log_service.write_entry(entry).await
    }
    
    /// Log resource usage
    pub async fn log_resource_usage(
        &self,
        component: &str,
        cpu_percent: f64,
        memory_mb: u64,
        disk_mb: Option<u64>,
    ) -> LogResult<()> {
        let message = format!(
            "Resource usage for '{}': CPU {:.1}%, Memory {}MB",
            component, cpu_percent, memory_mb
        );
        
        let mut context = serde_json::json!({
            "component": component,
            "cpu_percent": cpu_percent,
            "memory_mb": memory_mb,
            "event_type": "resource_usage"
        });
        
        if let Some(disk_mb) = disk_mb {
            context["disk_mb"] = serde_json::Value::Number(disk_mb.into());
        }
        
        let entry = LogEntry::with_context(
            Location::system("resource_monitor"),
            self.destination.clone(),
            LogLevel::Debug,
            message,
            Some("system".to_string()),
            Some("resource_monitor".to_string()),
            None,
            Some(context),
        );
        
        self.log_service.write_entry(entry).await
    }
    
    /// Generic system log entry
    pub async fn log(
        &self,
        level: LogLevel,
        component: &str,
        message: &str,
        context: Option<serde_json::Value>,
    ) -> LogResult<()> {
        let entry = LogEntry::with_context(
            Location::system(component),
            self.destination.clone(),
            level,
            message.to_string(),
            Some("system".to_string()),
            None,
            None,
            context,
        );
        
        self.log_service.write_entry(entry).await
    }
    
    /// Get system log entries
    pub async fn get_entries(&self, query: LogQuery) -> LogResult<Vec<LogEntry>> {
        self.log_service.read_entries(self.destination.clone(), query).await
    }
    
    /// Get system log statistics
    pub async fn get_stats(&self) -> LogResult<LogStats> {
        self.log_service.get_destination_stats(self.destination.clone()).await
    }
    
    /// Clear system logs
    pub async fn clear(&self) -> LogResult<()> {
        self.log_service.clear_logs(self.destination.clone()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_system_log_creation() {
        let log_service = Arc::new(LogService::new(LogServiceConfig::default()));
        log_service.initialize_default_logs().await.unwrap();
        
        let system_log = SystemLog::new(log_service);
        
        // Test startup log
        system_log.log_startup("test_component", "1.0.0").await.unwrap();
        
        let entries = system_log.get_entries(LogQuery::default()).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].message.message.contains("started successfully"));
    }

    #[tokio::test]
    async fn test_config_change_log() {
        let log_service = Arc::new(LogService::new(LogServiceConfig::default()));
        log_service.initialize_default_logs().await.unwrap();
        
        let system_log = SystemLog::new(log_service);
        
        system_log.log_config_change(
            "database",
            "max_connections",
            Some("10"),
            "20"
        ).await.unwrap();
        
        let entries = system_log.get_entries(LogQuery::default()).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].message.message.contains("Configuration changed"));
    }

    #[tokio::test]
    async fn test_health_status_log() {
        let log_service = Arc::new(LogService::new(LogServiceConfig::default()));
        log_service.initialize_default_logs().await.unwrap();
        
        let system_log = SystemLog::new(log_service);
        
        system_log.log_health_status("database", false, Some("Connection timeout")).await.unwrap();
        
        let entries = system_log.get_entries(LogQuery::default()).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].message.message.contains("unhealthy"));
        assert_eq!(entries[0].level(), LogLevel::Warn);
    }
}