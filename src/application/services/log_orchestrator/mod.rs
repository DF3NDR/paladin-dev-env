/*
Log Orchestrator

Application-layer orchestrator for log management.  Relocated from
`core::platform::manager::log_service` as part of Milestone 6 Epic 2 to enforce
hexagonal architecture boundaries.

This module wraps the base Message Service to provide specialised logging
functionality: destination routing, level filtering, in-memory storage, and
optional forwarding to a persistent `LogPort`.
*/

pub mod types;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use chrono::Utc;

use crate::core::base::service::message_service::MessageService;
use crate::core::platform::container::log::{
    Log, LogContainer, LogDestination, LogEntry, LogEntryExt,
};
use paladin_ports::output::log_port::{LogError, LogHealthCheck, LogQuery, LogResult, LogStats};

use types::{LogMessageHandler, LogServiceConfig};

pub use types::{LogPort, LogServiceConfig as LogOrchestratorConfig};

// ---------------------------------------------------------------------------
// Backwards-compatibility alias
// ---------------------------------------------------------------------------

/// Alias kept so call-sites that still use the old `LogService` name compile
/// without changes.
pub type LogService = LogOrchestrator;

// ---------------------------------------------------------------------------
// LogOrchestrator
// ---------------------------------------------------------------------------

/// Application-layer log orchestrator.
///
/// Manages log destinations, entry routing, level filtering, and optional
/// persistent storage via a `LogPort`.  Previously named `LogService`.
pub struct LogOrchestrator {
    /// Underlying message service
    message_service: Arc<MessageService>,
    /// Configuration
    config: LogServiceConfig,
    /// In-memory logs by destination
    logs: Arc<RwLock<HashMap<LogDestination, Log>>>,
    /// Output port for persistent logging
    log_port: Option<Arc<dyn LogPort>>,
    /// Service statistics
    stats: Arc<RwLock<LogStats>>,
}

impl LogOrchestrator {
    /// Create a new log orchestrator.
    pub fn new(config: LogServiceConfig) -> Self {
        let message_service = Arc::new(MessageService::new(config.message_config.clone()));

        Self {
            message_service,
            config,
            logs: Arc::new(RwLock::new(HashMap::new())),
            log_port: None,
            stats: Arc::new(RwLock::new(LogStats::default())),
        }
    }

    /// Attach a persistent log port.
    pub fn with_port(mut self, port: Arc<dyn LogPort>) -> Self {
        self.log_port = Some(port);
        self
    }

    /// Initialize the orchestrator.
    pub async fn initialize(&self) -> LogResult<()> {
        self.message_service
            .start()
            .await
            .map_err(|e| LogError::ConfigError(e.to_string()))?;

        let handler = Arc::new(LogMessageHandler {
            logs: self.logs.clone(),
            log_port: self.log_port.clone(),
            config: self.config.clone(),
        });

        self.message_service
            .register_handler("service".to_string(), handler.clone())
            .await
            .map_err(|e| LogError::ConfigError(e.to_string()))?;

        self.message_service
            .register_handler("log".to_string(), handler)
            .await
            .map_err(|e| LogError::ConfigError(e.to_string()))?;

        self.initialize_default_logs().await?;

        Ok(())
    }

    /// Initialize default log destinations.
    pub async fn initialize_default_logs(&self) -> LogResult<()> {
        let destinations = vec![
            (LogDestination::System, "system-log"),
            (LogDestination::Access, "access-log"),
            (LogDestination::Error, "error-log"),
            (LogDestination::Security, "security-log"),
            (LogDestination::Performance, "performance-log"),
        ];

        let mut logs = self.logs.write().await;

        for (destination, name) in destinations {
            let container = LogContainer::new(
                name.to_string(),
                destination.clone(),
                self.config.default_min_level,
            )
            .with_max_entries(self.config.max_memory_entries);

            let log = crate::core::base::entity::Entity::new(container, Some(name.to_string()));
            logs.insert(destination, log);
        }

        Ok(())
    }

    /// Write a log entry using the message service.
    pub async fn write_entry(&self, entry: LogEntry) -> LogResult<()> {
        {
            let mut stats = self.stats.write().await;
            stats.entries_written += 1;
            *stats.entries_by_level.entry(entry.level()).or_insert(0) += 1;
            stats.last_write = Some(Utc::now());
        }

        let message = crate::core::base::entity::message::Message {
            id: entry.id,
            source: entry.source,
            destination: entry.destination,
            timestamp: entry.timestamp,
            message: entry.message,
            correlation_id: entry.correlation_id,
            priority: entry.priority,
        };

        let receipt = self
            .message_service
            .send_message(message)
            .await
            .map_err(|e| LogError::IoError(e.to_string()))?;

        if receipt.status != crate::core::base::service::message_service::DeliveryStatus::Delivered
            && receipt.status
                != crate::core::base::service::message_service::DeliveryStatus::Pending
        {
            return Err(LogError::IoError(format!(
                "Message delivery failed: {:?}",
                receipt.status
            )));
        }

        Ok(())
    }

    /// Write multiple entries.
    pub async fn write_entries(&self, entries: Vec<LogEntry>) -> LogResult<()> {
        let messages: Vec<_> = entries
            .into_iter()
            .map(|entry| crate::core::base::entity::message::Message {
                id: entry.id,
                source: entry.source,
                destination: entry.destination,
                timestamp: entry.timestamp,
                message: entry.message,
                correlation_id: entry.correlation_id,
                priority: entry.priority,
            })
            .collect();

        let receipts = self
            .message_service
            .send_messages(messages)
            .await
            .map_err(|e| LogError::IoError(e.to_string()))?;

        {
            let mut stats = self.stats.write().await;
            for receipt in &receipts {
                if receipt.status
                    == crate::core::base::service::message_service::DeliveryStatus::Delivered
                {
                    stats.entries_written += 1;
                }
            }
            stats.last_write = Some(Utc::now());
        }

        Ok(())
    }

    /// Read entries from in-memory logs.
    pub async fn read_entries(
        &self,
        destination: LogDestination,
        query: LogQuery,
    ) -> LogResult<Vec<LogEntry>> {
        let logs = self.logs.read().await;

        if let Some(log) = logs.get(&destination) {
            let entries = log.data.get_entries(query.min_level);
            let mut filtered: Vec<LogEntry> = entries.into_iter().cloned().collect();

            if let Some(start) = query.start_time {
                filtered.retain(|e| e.timestamp >= start);
            }
            if let Some(end) = query.end_time {
                filtered.retain(|e| e.timestamp <= end);
            }
            if let Some(source) = &query.source {
                filtered.retain(|e| e.source.to_string().contains(source));
            }
            if let Some(module) = &query.module {
                filtered.retain(|e| {
                    e.message
                        .module
                        .as_ref()
                        .is_some_and(|m| m.contains(module))
                });
            }
            if let Some(content) = &query.message_contains {
                filtered.retain(|e| e.message.message.contains(content));
            }
            if let Some(offset) = query.offset {
                if offset < filtered.len() {
                    filtered.drain(0..offset);
                } else {
                    filtered.clear();
                }
            }
            if let Some(limit) = query.limit {
                filtered.truncate(limit);
            }

            Ok(filtered)
        } else {
            Err(LogError::DestinationNotFound(destination.name()))
        }
    }

    /// Get log statistics.
    pub async fn get_stats(&self) -> LogResult<LogStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    /// Get statistics for a specific destination.
    pub async fn get_destination_stats(&self, destination: LogDestination) -> LogResult<LogStats> {
        let logs = self.logs.read().await;

        if let Some(log) = logs.get(&destination) {
            let entries = log.data.get_entries(None);
            let mut stats = LogStats {
                entries_written: entries.len() as u64,
                ..Default::default()
            };

            for entry in &entries {
                *stats.entries_by_level.entry(entry.level()).or_insert(0) += 1;
            }

            if let Some(last_entry) = entries.last() {
                stats.last_write = Some(last_entry.timestamp);
            }

            Ok(stats)
        } else {
            Err(LogError::DestinationNotFound(destination.name()))
        }
    }

    /// Flush buffered entries.
    pub async fn flush(&self) -> LogResult<()> {
        if let Some(port) = &self.log_port {
            port.flush().await?;
        }
        Ok(())
    }

    /// Clear logs for a destination.
    pub async fn clear_logs(&self, destination: LogDestination) -> LogResult<()> {
        let mut logs = self.logs.write().await;

        if let Some(log) = logs.get_mut(&destination) {
            log.data.clear();
        }

        if let Some(port) = &self.log_port {
            port.clear_logs(destination).await?;
        }

        Ok(())
    }

    /// Get list of configured destinations.
    pub async fn list_destinations(&self) -> Vec<LogDestination> {
        let logs = self.logs.read().await;
        logs.keys().cloned().collect()
    }

    /// Health check.
    pub async fn health_check(&self) -> LogResult<Vec<LogHealthCheck>> {
        if let Some(port) = &self.log_port {
            port.health_check().await
        } else {
            let logs = self.logs.read().await;
            let mut checks = Vec::new();

            for (destination, log) in logs.iter() {
                let check = LogHealthCheck {
                    destination: destination.clone(),
                    healthy: true,
                    last_write: log.data.entries.last().map(|e| e.timestamp),
                    error_message: None,
                    response_time: Some(std::time::Duration::from_millis(1)),
                };
                checks.push(check);
            }

            Ok(checks)
        }
    }

    /// Stop the log orchestrator.
    pub async fn stop(&self) -> LogResult<()> {
        self.message_service
            .stop()
            .await
            .map_err(|e| LogError::IoError(e.to_string()))?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::base::entity::message::Location;
    use crate::core::platform::container::log::{LogEntryBuilder, LogLevel, LogMessage};
    use async_trait::async_trait;
    use paladin_ports::output::log_port::{
        BatchWriteRequest, LogDestinationConfig, LogFormat, LogQuery,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::time::{Duration, sleep};

    // -----------------------------------------------------------------------
    // Mock LogPort
    // -----------------------------------------------------------------------

    #[derive(Debug)]
    struct MockLogPort {
        entries: Arc<RwLock<Vec<LogEntry>>>,
        should_fail: bool,
        write_count: Arc<AtomicUsize>,
    }

    impl MockLogPort {
        fn new() -> Self {
            Self {
                entries: Arc::new(RwLock::new(Vec::new())),
                should_fail: false,
                write_count: Arc::new(AtomicUsize::new(0)),
            }
        }

        fn with_failure(mut self) -> Self {
            self.should_fail = true;
            self
        }

        async fn get_entries(&self) -> Vec<LogEntry> {
            self.entries.read().await.clone()
        }

        async fn entry_count(&self) -> usize {
            self.entries.read().await.len()
        }

        fn write_count(&self) -> usize {
            self.write_count.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl LogPort for MockLogPort {
        async fn write_entry(&self, entry: LogEntry) -> LogResult<()> {
            if self.should_fail {
                return Err(LogError::IoError("Mock failure".to_string()));
            }
            let mut entries = self.entries.write().await;
            entries.push(entry);
            self.write_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn write_entries(&self, entries: Vec<LogEntry>) -> LogResult<()> {
            if self.should_fail {
                return Err(LogError::IoError("Mock failure".to_string()));
            }
            let mut stored = self.entries.write().await;
            self.write_count.fetch_add(entries.len(), Ordering::SeqCst);
            stored.extend(entries);
            Ok(())
        }

        async fn read_entries(&self, _query: LogQuery) -> LogResult<Vec<LogEntry>> {
            Ok(self.entries.read().await.clone())
        }

        async fn clear_logs(&self, _destination: LogDestination) -> LogResult<()> {
            self.entries.write().await.clear();
            Ok(())
        }

        async fn get_stats(&self) -> LogResult<LogStats> {
            Ok(LogStats::default())
        }

        async fn health_check(&self) -> LogResult<Vec<LogHealthCheck>> {
            Ok(vec![LogHealthCheck {
                destination: LogDestination::System,
                healthy: !self.should_fail,
                last_write: None,
                error_message: if self.should_fail {
                    Some("Mock failure".to_string())
                } else {
                    None
                },
                response_time: Some(Duration::from_millis(1)),
            }])
        }

        async fn flush(&self) -> LogResult<()> {
            Ok(())
        }

        async fn batch_write(&self, _request: BatchWriteRequest) -> LogResult<()> {
            Ok(())
        }

        async fn count_entries(&self, _query: LogQuery) -> LogResult<u64> {
            Ok(self.entries.read().await.len() as u64)
        }

        async fn configure_destination(&self, _config: LogDestinationConfig) -> LogResult<()> {
            Ok(())
        }

        async fn remove_destination(&self, _destination: LogDestination) -> LogResult<()> {
            Ok(())
        }

        async fn list_destinations(&self) -> LogResult<Vec<LogDestination>> {
            Ok(vec![LogDestination::System])
        }

        async fn flush_destination(&self, _destination: LogDestination) -> LogResult<()> {
            Ok(())
        }

        async fn rotate_logs(&self, _destination: LogDestination) -> LogResult<()> {
            Ok(())
        }

        async fn get_destination_stats(&self, _destination: LogDestination) -> LogResult<LogStats> {
            Ok(LogStats::default())
        }

        async fn clear_logs_before(
            &self,
            _destination: LogDestination,
            _before: chrono::DateTime<chrono::Utc>,
        ) -> LogResult<u64> {
            Ok(0)
        }

        async fn health_check_destination(
            &self,
            destination: LogDestination,
        ) -> LogResult<LogHealthCheck> {
            Ok(LogHealthCheck {
                destination,
                healthy: !self.should_fail,
                last_write: None,
                error_message: if self.should_fail {
                    Some("Mock failure".to_string())
                } else {
                    None
                },
                response_time: Some(Duration::from_millis(1)),
            })
        }

        fn get_provider_name(&self) -> &'static str {
            "MockLogPort"
        }

        async fn test_connection(&self) -> LogResult<()> {
            if self.should_fail {
                Err(LogError::IoError("Mock failure".to_string()))
            } else {
                Ok(())
            }
        }

        async fn archive_logs(
            &self,
            _destination: LogDestination,
            _before: chrono::DateTime<chrono::Utc>,
        ) -> LogResult<String> {
            Ok("mock-archive-path".to_string())
        }

        fn supported_formats(&self) -> Vec<LogFormat> {
            vec![]
        }
    }

    // -----------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_log_service_creation() {
        let config = LogServiceConfig::default();
        let service = LogOrchestrator::new(config);
        service.initialize().await.unwrap();

        let destinations = service.list_destinations().await;
        assert_eq!(destinations.len(), 5);
        assert!(destinations.contains(&LogDestination::System));
        assert!(destinations.contains(&LogDestination::Access));
        assert!(destinations.contains(&LogDestination::Error));
        assert!(destinations.contains(&LogDestination::Security));
        assert!(destinations.contains(&LogDestination::Performance));
    }

    #[tokio::test]
    async fn test_write_and_read_entry_debug() {
        let service = LogOrchestrator::new(LogServiceConfig::default());
        service.initialize().await.unwrap();

        let entry = LogEntryBuilder::new_entry(
            Location::service("test-service"),
            LogDestination::System,
            LogLevel::Info,
            "Debug test message".to_string(),
        );

        let write_result = service.write_entry(entry.clone()).await;
        assert!(
            write_result.is_ok(),
            "Failed to write entry: {:?}",
            write_result
        );

        sleep(Duration::from_millis(500)).await;

        let query = LogQuery::default();
        let entries = service
            .read_entries(LogDestination::System, query)
            .await
            .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].message.message, "Debug test message");
    }

    #[tokio::test]
    async fn test_destination_routing() {
        let service = LogOrchestrator::new(LogServiceConfig::default());
        service.initialize().await.unwrap();

        let test_cases = vec![
            (LogDestination::System, "System message"),
            (LogDestination::Access, "Access message"),
            (LogDestination::Error, "Error message"),
            (LogDestination::Security, "Security message"),
            (LogDestination::Performance, "Performance message"),
        ];

        for (destination, message) in test_cases {
            let entry = LogEntryBuilder::new_entry(
                Location::service("test"),
                destination.clone(),
                LogLevel::Info,
                message.to_string(),
            );
            service.write_entry(entry).await.unwrap();
        }

        sleep(Duration::from_millis(200)).await;

        for (destination, expected_message) in [
            (LogDestination::System, "System message"),
            (LogDestination::Access, "Access message"),
            (LogDestination::Error, "Error message"),
            (LogDestination::Security, "Security message"),
            (LogDestination::Performance, "Performance message"),
        ] {
            let query = LogQuery::default();
            let entries = service
                .read_entries(destination.clone(), query)
                .await
                .unwrap();
            assert_eq!(entries.len(), 1, "Expected 1 entry for {:?}", destination);
            assert_eq!(entries[0].message.message, expected_message);
        }
    }

    #[tokio::test]
    async fn test_message_handler_destination_extraction() {
        let config = LogServiceConfig::default();
        let handler = LogMessageHandler {
            logs: Arc::new(RwLock::new(HashMap::new())),
            log_port: None,
            config,
        };

        let test_cases = vec![
            ("system-log", LogDestination::System),
            ("access-log", LogDestination::Access),
            ("error-log", LogDestination::Error),
            ("security-log", LogDestination::Security),
            ("performance-log", LogDestination::Performance),
            (
                "custom-log-mylog",
                LogDestination::Custom("mylog".to_string()),
            ),
            ("unknown", LogDestination::System),
            ("some-other-service", LogDestination::System),
        ];

        for (service_name, expected_dest) in test_cases {
            let message = crate::core::base::entity::message::Message::new(
                Location::service("test"),
                Location::service(service_name),
                LogMessage::new(LogLevel::Info, "test".to_string()),
            );
            let result = handler.extract_destination(&message);
            assert!(result.is_ok(), "Failed for service: {}", service_name);
            assert_eq!(
                result.unwrap(),
                expected_dest,
                "Wrong destination for: {}",
                service_name
            );
        }

        for location in [Location::system("test"), Location::user("test")] {
            let message = crate::core::base::entity::message::Message::new(
                Location::service("test"),
                location.clone(),
                LogMessage::new(LogLevel::Info, "test".to_string()),
            );
            assert!(handler.extract_destination(&message).is_err());
        }
    }

    #[tokio::test]
    async fn test_log_level_filtering() {
        let config = LogServiceConfig {
            default_min_level: LogLevel::Warn,
            ..Default::default()
        };

        let service = LogOrchestrator::new(config);
        service.initialize().await.unwrap();

        for (level, message) in [
            (LogLevel::Debug, "Debug message"),
            (LogLevel::Info, "Info message"),
            (LogLevel::Warn, "Warning message"),
            (LogLevel::Error, "Error message"),
            (LogLevel::Fatal, "Fatal message"),
        ] {
            let entry = LogEntryBuilder::new_entry(
                Location::service("test"),
                LogDestination::System,
                level,
                message.to_string(),
            );
            service.write_entry(entry).await.unwrap();
        }

        sleep(Duration::from_millis(200)).await;

        let entries = service
            .read_entries(LogDestination::System, LogQuery::default())
            .await
            .unwrap();
        assert_eq!(entries.len(), 3);
        let levels: Vec<LogLevel> = entries.iter().map(|e| e.level()).collect();
        assert!(levels.contains(&LogLevel::Warn));
        assert!(levels.contains(&LogLevel::Error));
        assert!(levels.contains(&LogLevel::Fatal));
        assert!(!levels.contains(&LogLevel::Debug));
        assert!(!levels.contains(&LogLevel::Info));
    }

    #[tokio::test]
    async fn test_batch_write_entries() {
        let service = LogOrchestrator::new(LogServiceConfig::default());
        service.initialize().await.unwrap();

        let entries: Vec<LogEntry> = (0..5)
            .map(|i| {
                LogEntryBuilder::new_entry(
                    Location::service("batch-test"),
                    LogDestination::System,
                    LogLevel::Info,
                    format!("Batch message {}", i),
                )
            })
            .collect();

        service.write_entries(entries).await.unwrap();
        sleep(Duration::from_millis(200)).await;

        let read_entries = service
            .read_entries(LogDestination::System, LogQuery::default())
            .await
            .unwrap();
        assert_eq!(read_entries.len(), 5);
        for i in 0..5 {
            assert!(
                read_entries
                    .iter()
                    .any(|e| e.message.message == format!("Batch message {}", i))
            );
        }
    }

    #[tokio::test]
    async fn test_query_filtering() {
        let service = LogOrchestrator::new(LogServiceConfig::default());
        service.initialize().await.unwrap();

        for (module, message, level) in [
            ("module1", "error occurred", LogLevel::Error),
            ("module2", "info message", LogLevel::Info),
            ("module1", "warning issued", LogLevel::Warn),
            ("module3", "debug info", LogLevel::Debug),
        ] {
            let log_msg =
                LogMessage::new(level, message.to_string()).with_module(module.to_string());
            let entry = crate::core::base::entity::message::Message::new(
                Location::service("test"),
                LogDestination::System.to_location(),
                log_msg,
            );
            service.write_entry(entry).await.unwrap();
        }

        sleep(Duration::from_millis(200)).await;

        let filtered = service
            .read_entries(
                LogDestination::System,
                LogQuery {
                    module: Some("module1".to_string()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(filtered.len(), 2);

        let filtered = service
            .read_entries(
                LogDestination::System,
                LogQuery {
                    message_contains: Some("error".to_string()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(filtered.len(), 1);

        let filtered = service
            .read_entries(
                LogDestination::System,
                LogQuery {
                    min_level: Some(LogLevel::Warn),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(filtered.len(), 2);
    }

    #[tokio::test]
    async fn test_pagination() {
        let service = LogOrchestrator::new(LogServiceConfig::default());
        service.initialize().await.unwrap();

        for i in 0..10 {
            let entry = LogEntryBuilder::new_entry(
                Location::service("paginate-test"),
                LogDestination::System,
                LogLevel::Info,
                format!("Message {}", i),
            );
            service.write_entry(entry).await.unwrap();
        }

        sleep(Duration::from_millis(300)).await;

        let limited = service
            .read_entries(
                LogDestination::System,
                LogQuery {
                    limit: Some(5),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(limited.len(), 5);

        let paginated = service
            .read_entries(
                LogDestination::System,
                LogQuery {
                    offset: Some(3),
                    limit: Some(4),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(paginated.len(), 4);
    }

    #[tokio::test]
    async fn test_with_log_port() {
        let mock_port = Arc::new(MockLogPort::new());
        let service =
            LogOrchestrator::new(LogServiceConfig::default()).with_port(mock_port.clone());
        service.initialize().await.unwrap();

        let entry = LogEntryBuilder::new_entry(
            Location::service("test"),
            LogDestination::System,
            LogLevel::Info,
            "Test with port".to_string(),
        );
        service.write_entry(entry).await.unwrap();
        sleep(Duration::from_millis(200)).await;

        assert_eq!(mock_port.write_count(), 1);
        assert_eq!(mock_port.entry_count().await, 1);
        assert_eq!(
            mock_port.get_entries().await[0].message.message,
            "Test with port"
        );
    }

    #[tokio::test]
    async fn test_error_handling_with_failing_port() {
        let failing_port = Arc::new(MockLogPort::new().with_failure());
        let service = LogOrchestrator::new(LogServiceConfig::default()).with_port(failing_port);
        service.initialize().await.unwrap();

        let entry = LogEntryBuilder::new_entry(
            Location::service("error-test"),
            LogDestination::Error,
            LogLevel::Error,
            "This should fail".to_string(),
        );
        let result = service.write_entry(entry).await;
        assert!(result.is_err());
        if let Err(LogError::IoError(msg)) = result {
            assert!(msg.contains("Message delivery failed"));
        } else {
            panic!("Expected IoError with delivery failure message");
        }
    }

    #[tokio::test]
    async fn test_statistics() {
        let service = LogOrchestrator::new(LogServiceConfig::default());
        service.initialize().await.unwrap();

        let stats = service.get_stats().await.unwrap();
        assert_eq!(stats.entries_written, 0);

        for level in [LogLevel::Info, LogLevel::Warn, LogLevel::Error] {
            let entry = LogEntryBuilder::new_entry(
                Location::service("stats-test"),
                LogDestination::System,
                level,
                "Statistics test".to_string(),
            );
            service.write_entry(entry).await.unwrap();
        }

        sleep(Duration::from_millis(100)).await;

        let stats = service.get_stats().await.unwrap();
        assert_eq!(stats.entries_written, 3);
        assert!(stats.last_write.is_some());

        let dest_stats = service
            .get_destination_stats(LogDestination::System)
            .await
            .unwrap();
        assert_eq!(dest_stats.entries_written, 3);
    }

    #[tokio::test]
    async fn test_clear_logs() {
        let service = LogOrchestrator::new(LogServiceConfig::default());
        service.initialize().await.unwrap();

        for i in 0..3 {
            let entry = LogEntryBuilder::new_entry(
                Location::service("clear-test"),
                LogDestination::System,
                LogLevel::Info,
                format!("Message to clear {}", i),
            );
            service.write_entry(entry).await.unwrap();
        }

        sleep(Duration::from_millis(100)).await;

        let entries = service
            .read_entries(LogDestination::System, LogQuery::default())
            .await
            .unwrap();
        assert_eq!(entries.len(), 3);

        service.clear_logs(LogDestination::System).await.unwrap();

        let entries = service
            .read_entries(LogDestination::System, LogQuery::default())
            .await
            .unwrap();
        assert_eq!(entries.len(), 0);
    }

    #[tokio::test]
    async fn test_health_check() {
        let service = LogOrchestrator::new(LogServiceConfig::default());
        service.initialize().await.unwrap();

        let health = service.health_check().await.unwrap();
        assert_eq!(health.len(), 5);
        for check in health {
            assert!(check.healthy);
            assert!(check.error_message.is_none());
        }
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let service = Arc::new(LogOrchestrator::new(LogServiceConfig::default()));
        service.initialize().await.unwrap();

        let mut handles = vec![];
        for i in 0..10 {
            let svc = service.clone();
            handles.push(tokio::spawn(async move {
                let entry = LogEntryBuilder::new_entry(
                    Location::service(&format!("concurrent-{}", i)),
                    LogDestination::System,
                    LogLevel::Info,
                    format!("Concurrent message {}", i),
                );
                svc.write_entry(entry).await
            }));
        }

        for handle in handles {
            handle.await.unwrap().unwrap();
        }

        sleep(Duration::from_millis(300)).await;

        let entries = service
            .read_entries(LogDestination::System, LogQuery::default())
            .await
            .unwrap();
        assert_eq!(entries.len(), 10);
    }

    #[tokio::test]
    async fn test_service_lifecycle() {
        let service = LogOrchestrator::new(LogServiceConfig::default());
        assert!(service.initialize().await.is_ok());

        let entry = LogEntryBuilder::new_entry(
            Location::service("lifecycle-test"),
            LogDestination::System,
            LogLevel::Info,
            "Lifecycle test".to_string(),
        );
        assert!(service.write_entry(entry).await.is_ok());
        assert!(service.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_custom_destination() {
        let service = LogOrchestrator::new(LogServiceConfig::default());
        service.initialize().await.unwrap();

        let custom_dest = LogDestination::Custom("test-custom".to_string());
        let entry = LogEntryBuilder::new_entry(
            Location::service("test"),
            custom_dest.clone(),
            LogLevel::Info,
            "Custom destination message".to_string(),
        );

        // Custom destinations are not auto-initialised — result may be ok or err; just ensure no panic.
        let _result = service.write_entry(entry).await;
    }

    #[tokio::test]
    async fn test_message_service_integration() {
        let service = LogOrchestrator::new(LogServiceConfig::default());
        service.initialize().await.unwrap();

        sleep(Duration::from_millis(50)).await;

        let destinations = service.message_service.list_destinations().await;
        assert!(
            destinations.contains(&"service".to_string())
                || destinations.contains(&"log".to_string()),
            "Expected 'service' or 'log' destination, got: {:?}",
            destinations
        );

        let log_message = LogMessage::new(LogLevel::Info, "Direct message test".to_string());
        let message = crate::core::base::entity::message::Message::new(
            Location::service("test"),
            Location::service("system-log"),
            log_message,
        );

        let receipt = service.message_service.send_message(message).await.unwrap();
        assert!(
            receipt.status
                == crate::core::base::service::message_service::DeliveryStatus::Delivered
                || receipt.status
                    == crate::core::base::service::message_service::DeliveryStatus::Pending
        );
    }
}
