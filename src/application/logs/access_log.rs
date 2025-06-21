// /*
// Access Log Module

// This module defines the structure and functionality for an access log in the application.
// It includes the definition of the AccessLog struct, which contains a vector of LogEntry objects.
// It also provides methods to add entries to the access log and to retrieve the log entries. 
// It is built on top of the platform level LogService.
// */
// use std::sync::Arc;
// use std::collections::HashMap;
// use chrono::{DateTime, Utc};

// use crate::core::platform::manager::log_service::LogService;
// use crate::core::platform::container::log::{LogEntry, LogLevel, LogDestination, LogEntryBuilder};
// use crate::core::base::entity::message::Location;
// use crate::application::ports::output::log_port::{LogResult, LogQuery, LogStats};

// /// HTTP method enumeration
// #[derive(Debug, Clone, PartialEq)]
// pub enum HttpMethod {
//     GET,
//     POST,
//     PUT,
//     DELETE,
//     PATCH,
//     HEAD,
//     OPTIONS,
//     Other(String),
// }

// impl From<&str> for HttpMethod {
//     fn from(method: &str) -> Self {
//         match method.to_uppercase().as_str() {
//             "GET" => HttpMethod::GET,
//             "POST" => HttpMethod::POST,
//             "PUT" => HttpMethod::PUT,
//             "DELETE" => HttpMethod::DELETE,
//             "PATCH" => HttpMethod::PATCH,
//             "HEAD" => HttpMethod::HEAD,
//             "OPTIONS" => HttpMethod::OPTIONS,
//             other => HttpMethod::Other(other.to_string()),
//         }
//     }
// }

// impl ToString for HttpMethod {
//     fn to_string(&self) -> String {
//         match self {
//             HttpMethod::GET => "GET".to_string(),
//             HttpMethod::POST => "POST".to_string(),
//             HttpMethod::PUT => "PUT".to_string(),
//             HttpMethod::DELETE => "DELETE".to_string(),
//             HttpMethod::PATCH => "PATCH".to_string(),
//             HttpMethod::HEAD => "HEAD".to_string(),
//             HttpMethod::OPTIONS => "OPTIONS".to_string(),
//             HttpMethod::Other(method) => method.clone(),
//         }
//     }
// }

// /// Access log service for HTTP requests and API calls
// pub struct AccessLog {
//     /// Underlying log service
//     log_service: Arc<LogService>,
//     /// Log destination for this service
//     destination: LogDestination,
// }

// impl AccessLog {
//     /// Create a new access log service
//     pub fn new(log_service: Arc<LogService>) -> Self {
//         Self {
//             log_service,
//             destination: LogDestination::Access,
//         }
//     }
    
//     /// Log an HTTP request
//     pub async fn log_request(
//         &self,
//         method: HttpMethod,
//         path: &str,
//         status_code: u16,
//         response_time_ms: u64,
//         client_ip: Option<&str>,
//         user_agent: Option<&str>,
//         user_id: Option<&str>,
//     ) -> LogResult<()> {
//         let level = match status_code {
//             200..=299 => LogLevel::Info,
//             300..=399 => LogLevel::Info,
//             400..=499 => LogLevel::Warn,
//             500..=599 => LogLevel::Error,
//             _ => LogLevel::Info,
//         };
        
//         let message = format!(
//             "{} {} {} {}ms",
//             method.to_string(),
//             path,
//             status_code,
//             response_time_ms
//         );
        
//         let mut context = serde_json::json!({
//             "method": method.to_string(),
//             "path": path,
//             "status_code": status_code,
//             "response_time_ms": response_time_ms,
//             "event_type": "http_request"
//         });
        
//         if let Some(ip) = client_ip {
//             context["client_ip"] = serde_json::Value::String(ip.to_string());
//         }
        
//         if let Some(ua) = user_agent {
//             context["user_agent"] = serde_json::Value::String(ua.to_string());
//         }
        
//         if let Some(uid) = user_id {
//             context["user_id"] = serde_json::Value::String(uid.to_string());
//         }
        
//         let entry = LogEntryBuilder::new_entry_with_context(
//             Location::external("http_server"),
//             self.destination.clone(),
//             level,
//             message,
//             Some("access".to_string()),
//             Some("http_request".to_string()),
//             None,
//             Some(context),
//         );
        
//         self.log_service.write_entry(entry).await
//     }
    
//     /// Log API authentication event
//     pub async fn log_authentication(
//         &self,
//         user_id: &str,
//         success: bool,
//         method: &str,
//         client_ip: Option<&str>,
//         details: Option<&str>,
//     ) -> LogResult<()> {
//         let level = if success { LogLevel::Info } else { LogLevel::Warn };
//         let status = if success { "successful" } else { "failed" };
        
//         let message = format!(
//             "Authentication {} for user '{}' using {}",
//             status, user_id, method
//         );
        
//         let mut context = serde_json::json!({
//             "user_id": user_id,
//             "success": success,
//             "auth_method": method,
//             "event_type": "authentication"
//         });
        
//         if let Some(ip) = client_ip {
//             context["client_ip"] = serde_json::Value::String(ip.to_string());
//         }
        
//         if let Some(details) = details {
//             context["details"] = serde_json::Value::String(details.to_string());
//         }
        
//         let entry = LogEntryBuilder::new_entry_with_context(
//             Location::external("auth_service"),
//             self.destination.clone(),
//             level,
//             message,
//             Some("access".to_string()),
//             Some("authentication".to_string()),
//             None,
//             Some(context),
//         );
        
//         self.log_service.write_entry(entry).await
//     }
    
//     /// Log API rate limiting event
//     pub async fn log_rate_limit(
//         &self,
//         client_ip: &str,
//         endpoint: &str,
//         limit: u32,
//         window_seconds: u32,
//         user_id: Option<&str>,
//     ) -> LogResult<()> {
//         let message = format!(
//             "Rate limit exceeded for {} on endpoint '{}' (limit: {}/{} seconds)",
//             client_ip, endpoint, limit, window_seconds
//         );
        
//         let mut context = serde_json::json!({
//             "client_ip": client_ip,
//             "endpoint": endpoint,
//             "rate_limit": limit,
//             "window_seconds": window_seconds,
//             "event_type": "rate_limit"
//         });
        
//         if let Some(uid) = user_id {
//             context["user_id"] = serde_json::Value::String(uid.to_string());
//         }
        
//         let entry = LogEntryBuilder::new_entry_with_context(
//             Location::external("rate_limiter"),
//             self.destination.clone(),
//             LogLevel::Warn,
//             message,
//             Some("access".to_string()),
//             Some("rate_limit".to_string()),
//             None,
//             Some(context),
//         );
        
//         self.log_service.write_entry(entry).await
//     }
    
//     /// Log file access
//     pub async fn log_file_access(
//         &self,
//         file_path: &str,
//         action: &str,
//         user_id: Option<&str>,
//         success: bool,
//         file_size: Option<u64>,
//     ) -> LogResult<()> {
//         let level = if success { LogLevel::Info } else { LogLevel::Warn };
//         let status = if success { "successful" } else { "failed" };
        
//         let message = format!(
//             "File {} {} for '{}'",
//             action, status, file_path
//         );
        
//         let mut context = serde_json::json!({
//             "file_path": file_path,
//             "action": action,
//             "success": success,
//             "event_type": "file_access"
//         });
        
//         if let Some(uid) = user_id {
//             context["user_id"] = serde_json::Value::String(uid.to_string());
//         }
        
//         if let Some(size) = file_size {
//             context["file_size"] = serde_json::Value::Number(size.into());
//         }
        
//         let entry = LogEntryBuilder::new_entry_with_context(
//             Location::system("file_service"),
//             self.destination.clone(),
//             level,
//             message,
//             Some("access".to_string()),
//             Some("file_access".to_string()),
//             None,
//             Some(context),
//         );
        
//         self.log_service.write_entry(entry).await
//     }
    
//     /// Log database access
//     pub async fn log_database_access(
//         &self,
//         operation: &str,
//         table: &str,
//         user_id: Option<&str>,
//         duration_ms: u64,
//         affected_rows: Option<u64>,
//     ) -> LogResult<()> {
//         let level = if duration_ms > 1000 { LogLevel::Warn } else { LogLevel::Debug };
        
//         let message = format!(
//             "Database {} on '{}' completed in {}ms",
//             operation, table, duration_ms
//         );
        
//         let mut context = serde_json::json!({
//             "operation": operation,
//             "table": table,
//             "duration_ms": duration_ms,
//             "event_type": "database_access"
//         });
        
//         if let Some(uid) = user_id {
//             context["user_id"] = serde_json::Value::String(uid.to_string());
//         }
        
//         if let Some(rows) = affected_rows {
//             context["affected_rows"] = serde_json::Value::Number(rows.into());
//         }
        
//         let entry = LogEntryBuilder::new_entry_with_context(
//             Location::system("database"),
//             self.destination.clone(),
//             level,
//             message,
//             Some("access".to_string()),
//             Some("database_access".to_string()),
//             None,
//             Some(context),
//         );
        
//         self.log_service.write_entry(entry).await
//     }
    
//     /// Generic access log entry
//     pub async fn log(
//         &self,
//         level: LogLevel,
//         source: &str,
//         message: &str,
//         context: Option<serde_json::Value>,
//     ) -> LogResult<()> {
//         let entry = LogEntryBuilder::new_entry_with_context(
//             Location::external(source),
//             self.destination.clone(),
//             level,
//             message.to_string(),
//             Some("access".to_string()),
//             None,
//             None,
//             context,
//         );
        
//         self.log_service.write_entry(entry).await
//     }
    
//     /// Get access log entries
//     pub async fn get_entries(&self, query: LogQuery) -> LogResult<Vec<LogEntry>> {
//         self.log_service.read_entries(self.destination.clone(), query).await
//     }
    
//     /// Get access statistics by status code
//     pub async fn get_status_code_stats(&self, since: Option<DateTime<Utc>>) -> LogResult<HashMap<u16, u64>> {
//         let mut query = LogQuery::default();
//         if let Some(since) = since {
//             query.start_time = Some(since);
//         }
        
//         let entries = self.get_entries(query).await?;
//         let mut stats = HashMap::new();
        
//         for entry in entries {
//             if let Some(context) = &entry.message.context {
//                 if let Some(status_code) = context.get("status_code").and_then(|v| v.as_u64()) {
//                     *stats.entry(status_code as u16).or_insert(0) += 1;
//                 }
//             }
//         }
        
//         Ok(stats)
//     }
    
//     /// Get access log statistics
//     pub async fn get_stats(&self) -> LogResult<LogStats> {
//         self.log_service.get_destination_stats(self.destination.clone()).await
//     }
    
//     /// Clear access logs
//     pub async fn clear(&self) -> LogResult<()> {
//         self.log_service.clear_logs(self.destination.clone()).await
//     }
// }

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::core::platform::manager::log_service::LogServiceConfig;
//     use crate::core::platform::container::log::LogEntryExt;

//     async fn setup_test_log_service() -> Arc<LogService> {
//         let log_service = Arc::new(LogService::new(LogServiceConfig::default()));
//         log_service.initialize_default_logs().await.unwrap();
//         log_service
//     }

//     #[tokio::test]
//     async fn test_access_log_creation() {
//         let log_service = setup_test_log_service().await;
//         let access_log = AccessLog::new(log_service);

//         access_log.log_request(
//             HttpMethod::GET,
//             "/api/users",
//             200,
//             150,
//             Some("192.168.1.1"),
//             Some("Mozilla/5.0"),
//             Some("user123"),
//         ).await.unwrap();

//         tokio::time::sleep(std::time::Duration::from_millis(10)).await;

//         let entries = access_log.get_entries(LogQuery::default()).await.unwrap();
//         assert_eq!(entries.len(), 1);
//         assert!(entries[0].message.message.contains("GET /api/users 200"));
//         assert_eq!(entries[0].level(), LogLevel::Info);
//     }

//     #[tokio::test]
//     async fn test_authentication_log() {
//         let log_service = setup_test_log_service().await;
//         let access_log = AccessLog::new(log_service);

//         access_log.log_authentication(
//             "user123",
//             false,
//             "password",
//             Some("192.168.1.1"),
//             Some("Invalid password"),
//         ).await.unwrap();

//         tokio::time::sleep(std::time::Duration::from_millis(10)).await;

//         let entries = access_log.get_entries(LogQuery::default()).await.unwrap();
//         assert_eq!(entries.len(), 1);
//         assert!(entries[0].message.message.contains("Authentication failed"));
//         assert_eq!(entries[0].level(), LogLevel::Warn);
//     }

//     #[tokio::test]
//     async fn test_rate_limit_log() {
//         let log_service = setup_test_log_service().await;
//         let access_log = AccessLog::new(log_service);

//         access_log.log_rate_limit(
//             "192.168.1.1",
//             "/api/data",
//             100,
//             60,
//             Some("user123"),
//         ).await.unwrap();

//         tokio::time::sleep(std::time::Duration::from_millis(10)).await;

//         let entries = access_log.get_entries(LogQuery::default()).await.unwrap();
//         assert_eq!(entries.len(), 1);
//         assert!(entries[0].message.message.contains("Rate limit exceeded"));
//         assert_eq!(entries[0].level(), LogLevel::Warn);
//     }

//     #[tokio::test]
//     async fn test_file_access_log() {
//         let log_service = setup_test_log_service().await;
//         let access_log = AccessLog::new(log_service);

//         access_log.log_file_access(
//             "/tmp/file.txt",
//             "read",
//             Some("user123"),
//             true,
//             Some(1024),
//         ).await.unwrap();

//         tokio::time::sleep(std::time::Duration::from_millis(10)).await;

//         let entries = access_log.get_entries(LogQuery::default()).await.unwrap();
//         assert_eq!(entries.len(), 1);
//         assert!(entries[0].message.message.contains("File read successful"));
//         assert_eq!(entries[0].level(), LogLevel::Info);
//     }

//     #[tokio::test]
//     async fn test_database_access_log() {
//         let log_service = setup_test_log_service().await;
//         let access_log = AccessLog::new(log_service);

//         access_log.log_database_access(
//             "SELECT",
//             "users",
//             Some("user123"),
//             2000,
//             Some(5),
//         ).await.unwrap();

//         tokio::time::sleep(std::time::Duration::from_millis(10)).await;

//         let entries = access_log.get_entries(LogQuery::default()).await.unwrap();
//         assert_eq!(entries.len(), 1);
//         assert!(entries[0].message.message.contains("Database SELECT on 'users'"));
//         assert_eq!(entries[0].level(), LogLevel::Warn);
//     }

//     #[tokio::test]
//     async fn test_generic_access_log() {
//         let log_service = setup_test_log_service().await;
//         let access_log = AccessLog::new(log_service);

//         let context = serde_json::json!({
//             "custom_field": "custom_value",
//             "access_id": 12345
//         });

//         access_log.log(
//             LogLevel::Info,
//             "custom_source",
//             "Custom access log entry",
//             Some(context),
//         ).await.unwrap();

//         tokio::time::sleep(std::time::Duration::from_millis(10)).await;

//         let entries = access_log.get_entries(LogQuery::default()).await.unwrap();
//         assert_eq!(entries.len(), 1);
//         assert!(entries[0].message.message.contains("Custom access log entry"));
//         assert_eq!(entries[0].level(), LogLevel::Info);
//     }

//     #[tokio::test]
//     async fn test_status_code_stats() {
//         let log_service = setup_test_log_service().await;
//         let access_log = AccessLog::new(log_service);

//         access_log.log_request(
//             HttpMethod::GET,
//             "/api/users",
//             200,
//             100,
//             None,
//             None,
//             None,
//         ).await.unwrap();

//         access_log.log_request(
//             HttpMethod::POST,
//             "/api/users",
//             404,
//             120,
//             None,
//             None,
//             None,
//         ).await.unwrap();

//         tokio::time::sleep(std::time::Duration::from_millis(10)).await;

//         let stats = access_log.get_status_code_stats(None).await.unwrap();
//         assert_eq!(stats.get(&200), Some(&1));
//         assert_eq!(stats.get(&404), Some(&1));
//     }

//     #[tokio::test]
//     async fn test_http_method_conversion() {
//         assert_eq!(HttpMethod::from("GET"), HttpMethod::GET);
//         assert_eq!(HttpMethod::from("post"), HttpMethod::POST);
//         assert_eq!(HttpMethod::from("CUSTOM"), HttpMethod::Other("CUSTOM".to_string()));

//         assert_eq!(HttpMethod::GET.to_string(), "GET");
//         assert_eq!(HttpMethod::Other("CUSTOM".to_string()).to_string(), "CUSTOM");
//     }

//     #[tokio::test]
//     async fn test_access_log_stats_and_clear() {
//         let log_service = setup_test_log_service().await;
//         let access_log = AccessLog::new(log_service);

//         access_log.log_request(
//             HttpMethod::GET,
//             "/api/test",
//             200,
//             50,
//             None,
//             None,
//             None,
//         ).await.unwrap();

//         tokio::time::sleep(std::time::Duration::from_millis(10)).await;

//         let stats = access_log.get_stats().await.unwrap();
//         assert!(stats.entries_written > 0);

//         access_log.clear().await.unwrap();
//         let entries = access_log.get_entries(LogQuery::default()).await.unwrap();
//         assert_eq!(entries.len(), 0);
//     }
// }