/*
Log Orchestrator — Application-Layer Types

Coordination types for the log orchestrator.  These live in the application
layer because `LogMessageHandler` holds an `Arc<dyn LogPort>` (a port
reference), making it an application-level concern rather than a pure domain
type.
*/

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use async_trait::async_trait;

use crate::core::base::entity::message::Location;
use crate::core::base::service::message_service::{
    MessageError, MessageHandler, MessageResult, MessageServiceConfig,
};
use crate::core::platform::container::log::{Log, LogDestination, LogEntry, LogLevel, LogMessage};
use paladin_ports::output::log_port::{LogError, LogResult};

// Re-export for convenience inside the module tree.
pub use paladin_ports::output::log_port::LogPort;

// ---------------------------------------------------------------------------
// LogServiceConfig
// ---------------------------------------------------------------------------

/// Configuration for the log orchestrator.
#[derive(Debug, Clone)]
pub struct LogServiceConfig {
    /// Default minimum log level
    pub default_min_level: LogLevel,
    /// Maximum number of in-memory log entries per destination
    pub max_memory_entries: usize,
    /// Whether to enable async logging
    pub async_logging: bool,
    /// Buffer size for async logging
    pub buffer_size: usize,
    /// Flush interval for buffered entries
    pub flush_interval: std::time::Duration,
    /// Message service configuration
    pub message_config: MessageServiceConfig,
}

impl Default for LogServiceConfig {
    fn default() -> Self {
        Self {
            default_min_level: LogLevel::Info,
            max_memory_entries: 1000,
            async_logging: true,
            buffer_size: 100,
            flush_interval: std::time::Duration::from_secs(5),
            message_config: MessageServiceConfig::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// LogMessageHandler  (pub(super) — visible only to mod.rs)
// ---------------------------------------------------------------------------

/// Internal message handler that routes log messages to in-memory logs and
/// forwards them to the configured `LogPort`.
pub(super) struct LogMessageHandler {
    pub(super) logs: Arc<RwLock<HashMap<LogDestination, Log>>>,
    pub(super) log_port: Option<Arc<dyn LogPort>>,
    // Handler configuration; retained for future log-routing rules.
    #[allow(dead_code)]
    pub(super) config: LogServiceConfig,
}

impl LogMessageHandler {
    pub(super) fn extract_destination(
        &self,
        entry: &crate::core::base::entity::message::Message<LogMessage>,
    ) -> LogResult<LogDestination> {
        match &entry.destination {
            Location::Service(name) => {
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
                    // For unrecognised service names, fall back to System.
                    Ok(LogDestination::System)
                }
            }
            _ => Err(LogError::DestinationNotFound(format!(
                "{:?}",
                entry.destination
            ))),
        }
    }
}

#[async_trait]
impl MessageHandler<LogMessage> for LogMessageHandler {
    async fn handle_message(
        &self,
        message: crate::core::base::entity::message::Message<LogMessage>,
    ) -> MessageResult<()> {
        let destination = match self.extract_destination(&message) {
            Ok(dest) => dest,
            Err(_) => return Ok(()),
        };

        let normalized_destination = destination.to_location();

        let log_entry = LogEntry {
            id: message.id,
            source: message.source,
            destination: normalized_destination.clone(),
            timestamp: message.timestamp,
            message: message.message,
            correlation_id: message.correlation_id,
            priority: message.priority,
        };

        {
            let mut logs = self.logs.write().await;
            if let Some(log) = logs.get_mut(&destination) {
                log.data.add_entry(log_entry.clone());
            }
        }

        if let Some(port) = &self.log_port {
            port.write_entry(log_entry)
                .await
                .map_err(|e| MessageError::DeliveryFailed(e.to_string()))?;
        }

        Ok(())
    }

    fn supported_destinations(&self) -> Vec<Location> {
        vec![
            LogDestination::System.to_location(),
            LogDestination::Access.to_location(),
            LogDestination::Error.to_location(),
            LogDestination::Security.to_location(),
            LogDestination::Performance.to_location(),
        ]
    }
}
