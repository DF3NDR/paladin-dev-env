// /*
// System Log Module

// This module defines the structure and functionality for a system log in the application.
// It includes the definition of the SystemLog struct, which contains a vector of LogEntry objects.
// It also provides methods to add entries to the system log and to retrieve the log entries.
// It is built on top of the platform level LogService.
// */
// use std::sync::Arc;

// use crate::core::platform::manager::log_service::LogService;
// use crate::core::platform::container::log::{LogEntry, LogLevel, LogDestination, LogEntryBuilder};
// use crate::core::base::entity::message::Location;
// use crate::application::ports::output::log_port::{LogResult, LogQuery, LogStats};

// /// System log service for general system operations
// pub struct SystemLog {
//     /// Underlying log service
//     log_service: Arc<LogService>,
//     /// Log destination for this service
//     destination: LogDestination,
// }

// impl SystemLog {
//     /// Create a new system log service
//     pub fn new(log_service: Arc<LogService>) -> Self {
//         Self {
//             log_service,
//             destination: LogDestination::System,
//         }
//     }
    
//     /// Log system startup event
//     pub async fn log_startup(&self, component: &str, version: &str) -> LogResult<()> {
//         let entry = LogEntryBuilder::new_entry_with_context(
//             Location::system(component),
//             self.destination.clone(),
//             LogLevel::Info,
//             format!("System component '{}' started successfully", component),
//             Some("system".to_string()),
//             Some("startup".to_string()),
//             None,
//             Some(serde_json::json!({
//                 "component": component,
//                 "version": version,
//                 "event_type": "startup"
//             })),
//         );
        
//         self.log_service.write_entry(entry).await
//     }
    
//     /// Log system shutdown event
//     pub async fn log_shutdown(&self, component: &str, reason: Option<&str>) -> LogResult<()> {
//         let message = match reason {
//             Some(reason) => format!("System component '{}' shutting down: {}", component, reason),
//             None => format!("System component '{}' shutting down", component),
//         };
        
//         let mut context = serde_json::json!({
//             "component": component,
//             "event_type": "shutdown"
//         });
        
//         if let Some(reason) = reason {
//             context["reason"] = serde_json::Value::String(reason.to_string());
//         }
        
//         let entry = LogEntryBuilder::new_entry_with_context(
//             Location::system(component),
//             self.destination.clone(),
//             LogLevel::Info,
//             message,
//             Some("system".to_string()),
//             Some("shutdown".to_string()),
//             None,
//             Some(context),
//         );
        
//         self.log_service.write_entry(entry).await
//     }
    
//     /// Log configuration changes
//     pub async fn log_config_change(
//         &self,
//         component: &str,
//         setting: &str,
//         old_value: Option<&str>,
//         new_value: &str,
//     ) -> LogResult<()> {
//         let message = format!(
//             "Configuration changed for '{}': {} = {}",
//             component, setting, new_value
//         );
        
//         let mut context = serde_json::json!({
//             "component": component,
//             "setting": setting,
//             "new_value": new_value,
//             "event_type": "config_change"
//         });
        
//         if let Some(old_value) = old_value {
//             context["old_value"] = serde_json::Value::String(old_value.to_string());
//         }
        
//         let entry = LogEntryBuilder::new_entry_with_context(
//             Location::system(component),
//             self.destination.clone(),
//             LogLevel::Info,
//             message,
//             Some("system".to_string()),
//             Some("config_change".to_string()),
//             None,
//             Some(context),
//         );
        
//         self.log_service.write_entry(entry).await
//     }
    
//     /// Log service health status
//     pub async fn log_health_status(
//         &self,
//         service: &str,
//         healthy: bool,
//         details: Option<&str>,
//     ) -> LogResult<()> {
//         let level = if healthy { LogLevel::Info } else { LogLevel::Warn };
//         let status = if healthy { "healthy" } else { "unhealthy" };
        
//         let message = format!("Service '{}' health check: {}", service, status);
        
//         let mut context = serde_json::json!({
//             "service": service,
//             "healthy": healthy,
//             "event_type": "health_check"
//         });
        
//         if let Some(details) = details {
//             context["details"] = serde_json::Value::String(details.to_string());
//         }
        
//         let entry = LogEntryBuilder::new_entry_with_context(
//             Location::system("health_monitor"),
//             self.destination.clone(),
//             level,
//             message,
//             Some("system".to_string()),
//             Some("health_check".to_string()),
//             None,
//             Some(context),
//         );
        
//         self.log_service.write_entry(entry).await
//     }
    
//     /// Log resource usage
//     pub async fn log_resource_usage(
//         &self,
//         component: &str,
//         cpu_percent: f64,
//         memory_mb: u64,
//         disk_mb: Option<u64>,
//     ) -> LogResult<()> {
//         let message = format!(
//             "Resource usage for '{}': CPU {:.1}%, Memory {}MB",
//             component, cpu_percent, memory_mb
//         );
        
//         let mut context = serde_json::json!({
//             "component": component,
//             "cpu_percent": cpu_percent,
//             "memory_mb": memory_mb,
//             "event_type": "resource_usage"
//         });
        
//         if let Some(disk_mb) = disk_mb {
//             context["disk_mb"] = serde_json::Value::Number(disk_mb.into());
//         }
        
//         let entry = LogEntryBuilder::new_entry_with_context(
//             Location::system("resource_monitor"),
//             self.destination.clone(),
//             LogLevel::Debug,
//             message,
//             Some("system".to_string()),
//             Some("resource_monitor".to_string()),
//             None,
//             Some(context),
//         );
        
//         self.log_service.write_entry(entry).await
//     }
    
//     /// Generic system log entry
//     pub async fn log(
//         &self,
//         level: LogLevel,
//         component: &str,
//         message: &str,
//         context: Option<serde_json::Value>,
//     ) -> LogResult<()> {
//         let entry = LogEntryBuilder::new_entry_with_context(
//             Location::system(component),
//             self.destination.clone(),
//             level,
//             message.to_string(),
//             Some("system".to_string()),
//             None,
//             None,
//             context,
//         );
        
//         self.log_service.write_entry(entry).await
//     }
    
//     /// Get system log entries
//     pub async fn get_entries(&self, query: LogQuery) -> LogResult<Vec<LogEntry>> {
//         self.log_service.read_entries(self.destination.clone(), query).await
//     }
    
//     /// Get system log statistics
//     pub async fn get_stats(&self) -> LogResult<LogStats> {
//         self.log_service.get_destination_stats(self.destination.clone()).await
//     }
    
//     /// Clear system logs
//     pub async fn clear(&self) -> LogResult<()> {
//         self.log_service.clear_logs(self.destination.clone()).await
//     }
// }

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::core::platform::manager::log_service::LogServiceConfig;
//     use crate::core::platform::container::log::LogEntryExt;
//     use std::sync::atomic::{AtomicUsize, Ordering};

//     static TEST_ID: AtomicUsize = AtomicUsize::new(0);

//     async fn setup_test_log_service() -> Arc<LogService> {
//         let _id = TEST_ID.fetch_add(1, Ordering::SeqCst);
//         let mut _config = LogServiceConfig::default();
//         // config.target = format!("system_log_test_{}", id);
//         let log_service = Arc::new(LogService::new(config));
//         log_service.initialize_default_logs().await.unwrap();
//         log_service
//     }

//     #[tokio::test]
//     async fn test_system_log_creation() {
//         let log_service = setup_test_log_service().await;
//         let system_log = SystemLog::new(log_service);

//         system_log.log_startup("test_component", "1.0.0").await.unwrap();

//         tokio::time::sleep(std::time::Duration::from_millis(10)).await;

//         let entries = system_log.get_entries(LogQuery::default()).await.unwrap();
//         assert_eq!(entries.len(), 1);
//         assert!(entries[0].message.message.contains("started successfully"));
//         assert_eq!(entries[0].level(), LogLevel::Info);
//     }

//     #[tokio::test]
//     async fn test_shutdown_log() {
//         let log_service = setup_test_log_service().await;
//         let system_log = SystemLog::new(log_service);

//         system_log.log_shutdown("test_component", Some("maintenance")).await.unwrap();

//         tokio::time::sleep(std::time::Duration::from_millis(10)).await;

//         let entries = system_log.get_entries(LogQuery::default()).await.unwrap();
//         assert_eq!(entries.len(), 1);
//         assert!(entries[0].message.message.contains("shutting down: maintenance"));
//         assert_eq!(entries[0].level(), LogLevel::Info);
//     }

//     #[tokio::test]
//     async fn test_config_change_log() {
//         let log_service = setup_test_log_service().await;
//         let system_log = SystemLog::new(log_service);

//         system_log.log_config_change(
//             "database",
//             "max_connections",
//             Some("10"),
//             "20"
//         ).await.unwrap();

//         tokio::time::sleep(std::time::Duration::from_millis(10)).await;

//         let entries = system_log.get_entries(LogQuery::default()).await.unwrap();
//         assert_eq!(entries.len(), 1);
//         assert!(entries[0].message.message.contains("Configuration changed"));
//         assert_eq!(entries[0].level(), LogLevel::Info);
//     }

//     #[tokio::test]
//     async fn test_health_status_log() {
//         let log_service = setup_test_log_service().await;
//         let system_log = SystemLog::new(log_service);

//         system_log.log_health_status("database", false, Some("Connection timeout")).await.unwrap();

//         tokio::time::sleep(std::time::Duration::from_millis(10)).await;

//         let entries = system_log.get_entries(LogQuery::default()).await.unwrap();
//         assert_eq!(entries.len(), 1);
//         assert!(entries[0].message.message.contains("unhealthy"));
//         assert_eq!(entries[0].level(), LogLevel::Warn);
//     }

//     #[tokio::test]
//     async fn test_resource_usage_log() {
//         let log_service = setup_test_log_service().await;
//         let system_log = SystemLog::new(log_service);

//         system_log.log_resource_usage("worker", 75.5, 2048, Some(10000)).await.unwrap();

//         tokio::time::sleep(std::time::Duration::from_millis(10)).await;

//         let entries = system_log.get_entries(LogQuery::default()).await.unwrap();
//         assert_eq!(entries.len(), 1);
//         assert!(entries[0].message.message.contains("Resource usage for 'worker'"));
//         assert_eq!(entries[0].level(), LogLevel::Debug);
//     }

//     #[tokio::test]
//     async fn test_generic_system_log() {
//         let log_service = setup_test_log_service().await;
//         let system_log = SystemLog::new(log_service);

//         let context = serde_json::json!({
//             "custom_field": "custom_value",
//             "system_id": 12345
//         });

//         system_log.log(
//             LogLevel::Info,
//             "custom_component",
//             "Custom system log entry",
//             Some(context),
//         ).await.unwrap();

//         tokio::time::sleep(std::time::Duration::from_millis(10)).await;

//         let entries = system_log.get_entries(LogQuery::default()).await.unwrap();
//         assert_eq!(entries.len(), 1);
//         assert!(entries[0].message.message.contains("Custom system log entry"));
//         assert_eq!(entries[0].level(), LogLevel::Info);
//     }

//     #[tokio::test]
//     async fn test_system_log_stats_and_clear() {
//         let log_service = setup_test_log_service().await;
//         let system_log = SystemLog::new(log_service);

//         system_log.log_startup("test_component", "1.0.0").await.unwrap();

//         tokio::time::sleep(std::time::Duration::from_millis(10)).await;

//         let stats = system_log.get_stats().await.unwrap();
//         assert!(stats.entries_written > 0);

//         system_log.clear().await.unwrap();
//         let entries = system_log.get_entries(LogQuery::default()).await.unwrap();
//         assert_eq!(entries.len(), 0);
//     }
// }