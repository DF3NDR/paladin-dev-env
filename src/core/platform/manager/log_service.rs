/*
Log Service

The core platform log service that manages log operations across the system.
This service coordinates between different log destinations and handles routing
of log entries based on their destinations and levels.
*/
use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

use crate::application::ports::output::log_port::{
    LogPort, LogResult, LogError, LogQuery, LogDestinationConfig, LogStats, LogHealthCheck
};
use crate::core::platform::container::log::{
    Log, LogEntry, LogLevel, LogDestination, LogContainer
};

/// Configuration for the log service
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
}

impl Default for LogServiceConfig {
    fn default() -> Self {
        Self {
            default_min_level: LogLevel::Info,
            max_memory_entries: 1000,
            async_logging: true,
            buffer_size: 100,
            flush_interval: std::time::Duration::from_secs(5),
        }
    }
}

/// The core log service implementation
pub struct LogService {
    /// Configuration
    config: LogServiceConfig,
    /// In-memory logs by destination
    logs: Arc<RwLock<HashMap<LogDestination, Log>>>,
    /// Output port for persistent logging
    log_port: Option<Arc<dyn LogPort>>,
    /// Service statistics
    stats: Arc<RwLock<LogStats>>,
    /// Buffer for async logging
    buffer: Arc<RwLock<Vec<LogEntry>>>,
}

impl LogService {
    /// Create a new log service
    pub fn new(config: LogServiceConfig) -> Self {
        Self {
            config,
            logs: Arc::new(RwLock::new(HashMap::new())),
            log_port: None,
            stats: Arc::new(RwLock::new(LogStats::default())),
            buffer: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Create with log port
    pub fn with_port(mut self, port: Arc<dyn LogPort>) -> Self {
        self.log_port = Some(port);
        self
    }
    
    /// Initialize default log destinations
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
            let log = Log::new_log(
                name.to_string(),
                destination.clone(),
                self.config.default_min_level,
            );
            logs.insert(destination, log);
        }
        
        Ok(())
    }
    
    /// Add or update a log destination
    pub async fn add_log_destination(
        &self,
        destination: LogDestination,
        min_level: LogLevel,
    ) -> LogResult<()> {
        let mut logs = self.logs.write().await;
        
        let log = Log::new_log(
            destination.name(),
            destination.clone(),
            min_level,
        );
        
        logs.insert(destination, log);
        Ok(())
    }
    
    /// Remove a log destination
    pub async fn remove_log_destination(&self, destination: &LogDestination) -> LogResult<()> {
        let mut logs = self.logs.write().await;
        logs.remove(destination);
        Ok(())
    }
    
    /// Write a log entry
    pub async fn write_entry(&self, entry: LogEntry) -> LogResult<()> {
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.entries_written += 1;
            *stats.entries_by_level.entry(entry.level()).or_insert(0) += 1;
            stats.last_write = Some(Utc::now());
        }
        
        // Get destination from entry
        let destination = self.extract_destination(&entry)?;
        
        // Write to in-memory log
        {
            let mut logs = self.logs.write().await;
            if let Some(log) = logs.get_mut(&destination) {
                log.add_entry(entry.clone());
            }
        }
        
        // Handle persistent logging
        if let Some(port) = &self.log_port {
            if self.config.async_logging {
                self.buffer_entry(entry).await?;
            } else {
                port.write_entry(entry).await?;
            }
        }
        
        Ok(())
    }
    
    /// Write multiple entries
    pub async fn write_entries(&self, entries: Vec<LogEntry>) -> LogResult<()> {
        for entry in entries {
            self.write_entry(entry).await?;
        }
        Ok(())
    }
    
    /// Read entries from in-memory logs
    pub async fn read_entries(&self, destination: LogDestination, query: LogQuery) -> LogResult<Vec<LogEntry>> {
        let logs = self.logs.read().await;
        
        if let Some(log) = logs.get(&destination) {
            let entries = log.node.get_entries(query.min_level);
            
            // Apply additional filters
            let mut filtered: Vec<LogEntry> = entries.into_iter().cloned().collect();
            
            // Filter by time range
            if let Some(start) = query.start_time {
                filtered.retain(|e| e.timestamp >= start);
            }
            if let Some(end) = query.end_time {
                filtered.retain(|e| e.timestamp <= end);
            }
            
            // Filter by source
            if let Some(source) = &query.source {
                filtered.retain(|e| e.source.to_string().contains(source));
            }
            
            // Filter by module
            if let Some(module) = &query.module {
                filtered.retain(|e| {
                    e.message.module.as_ref().map_or(false, |m| m.contains(module))
                });
            }
            
            // Filter by message content
            if let Some(content) = &query.message_contains {
                filtered.retain(|e| e.message.message.contains(content));
            }
            
            // Apply pagination
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
    
    /// Get log statistics
    pub async fn get_stats(&self) -> LogResult<LogStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
    
    /// Get statistics for a specific destination
    pub async fn get_destination_stats(&self, destination: LogDestination) -> LogResult<LogStats> {
        let logs = self.logs.read().await;
        
        if let Some(log) = logs.get(&destination) {
            let entries = log.node.get_entries(None);
            let mut stats = LogStats::default();
            
            stats.entries_written = entries.len() as u64;
            
            for entry in entries {
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
    
    /// Flush buffered entries
    pub async fn flush(&self) -> LogResult<()> {
        if let Some(port) = &self.log_port {
            let entries = {
                let mut buffer = self.buffer.write().await;
                let entries = buffer.clone();
                buffer.clear();
                entries
            };
            
            if !entries.is_empty() {
                port.write_entries(entries).await?;
            }
            
            port.flush().await?;
        }
        
        Ok(())
    }
    
    /// Clear logs for a destination
    pub async fn clear_logs(&self, destination: LogDestination) -> LogResult<()> {
        let mut logs = self.logs.write().await;
        
        if let Some(log) = logs.get_mut(&destination) {
            log.node.clear();
        }
        
        if let Some(port) = &self.log_port {
            port.clear_logs(destination).await?;
        }
        
        Ok(())
    }
    
    /// Get list of configured destinations
    pub async fn list_destinations(&self) -> Vec<LogDestination> {
        let logs = self.logs.read().await;
        logs.keys().cloned().collect()
    }
    
    /// Health check
    pub async fn health_check(&self) -> LogResult<Vec<LogHealthCheck>> {
        if let Some(port) = &self.log_port {
            port.health_check().await
        } else {
            // Return health check for in-memory logs only
            let logs = self.logs.read().await;
            let mut checks = Vec::new();
            
            for (destination, log) in logs.iter() {
                let check = LogHealthCheck {
                    destination: destination.clone(),
                    healthy: true,
                    last_write: log.node.entries.last().map(|e| e.timestamp),
                    error_message: None,
                    response_time: Some(std::time::Duration::from_millis(1)),
                };
                checks.push(check);
            }
            
            Ok(checks)
        }
    }
    
    // Private helper methods
    
    /// Extract destination from log entry
    fn extract_destination(&self, entry: &LogEntry) -> LogResult<LogDestination> {
        // Parse destination from the entry's destination location
        let dest_str = entry.destination.to_string();
        
        if dest_str.contains("system-log") {
            Ok(LogDestination::System)
        } else if dest_str.contains("access-log") {
            Ok(LogDestination::Access)
        } else if dest_str.contains("error-log") {
            Ok(LogDestination::Error)
        } else if dest_str.contains("security-log") {
            Ok(LogDestination::Security)
        } else if dest_str.contains("performance-log") {
            Ok(LogDestination::Performance)
        } else if dest_str.starts_with("custom-log-") {
            let name = dest_str.strip_prefix("custom-log-").unwrap_or("unknown");
            Ok(LogDestination::Custom(name.to_string()))
        } else {
            // Default to system log
            Ok(LogDestination::System)
        }
    }
    
    /// Buffer an entry for async logging
    async fn buffer_entry(&self, entry: LogEntry) -> LogResult<()> {
        let mut buffer = self.buffer.write().await;
        buffer.push(entry);
        
        // Flush if buffer is full
        if buffer.len() >= self.config.buffer_size {
            if let Some(port) = &self.log_port {
                let entries = buffer.clone();
                buffer.clear();
                drop(buffer); // Release lock before async call
                
                port.write_entries(entries).await?;
            }
        }
        
        Ok(())
    }
    
    /// Start the flush timer for async logging
    pub async fn start_flush_timer(&self) -> LogResult<()> {
        if !self.config.async_logging {
            return Ok(());
        }
        
        let buffer = self.buffer.clone();
        let port = self.log_port.clone();
        let interval = self.config.flush_interval;
        
        tokio::spawn(async move {
            let mut flush_interval = tokio::time::interval(interval);
            
            loop {
                flush_interval.tick().await;
                
                if let Some(port) = &port {
                    let entries = {
                        let mut buffer = buffer.write().await;
                        if buffer.is_empty() {
                            continue;
                        }
                        let entries = buffer.clone();
                        buffer.clear();
                        entries
                    };
                    
                    if let Err(e) = port.write_entries(entries).await {
                        eprintln!("Failed to flush log entries: {}", e);
                    }
                }
            }
        });
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::base::entity::message::Location;

    #[tokio::test]
    async fn test_log_service_creation() {
        let config = LogServiceConfig::default();
        let service = LogService::new(config);
        
        let destinations = service.list_destinations().await;
        assert!(destinations.is_empty());
    }

    #[tokio::test]
    async fn test_initialize_default_logs() {
        let service = LogService::new(LogServiceConfig::default());
        service.initialize_default_logs().await.unwrap();
        
        let destinations = service.list_destinations().await;
        assert_eq!(destinations.len(), 5);
        assert!(destinations.contains(&LogDestination::System));
        assert!(destinations.contains(&LogDestination::Access));
    }

    #[tokio::test]
    async fn test_write_and_read_entry() {
        let service = LogService::new(LogServiceConfig::default());
        service.initialize_default_logs().await.unwrap();
        
        let entry = LogEntry::new(
            Location::service("test"),
            LogDestination::System,
            LogLevel::Info,
            "Test message".to_string(),
        );
        
        service.write_entry(entry.clone()).await.unwrap();
        
        let query = LogQuery::default();
        let entries = service.read_entries(LogDestination::System, query).await.unwrap();
        
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].message.message, "Test message");
    }

    #[tokio::test]
    async fn test_destination_stats() {
        let service = LogService::new(LogServiceConfig::default());
        service.initialize_default_logs().await.unwrap();
        
        let entry = LogEntry::new(
            Location::service("test"),
            LogDestination::System,
            LogLevel::Error,
            "Error message".to_string(),
        );
        
        service.write_entry(entry).await.unwrap();
        
        let stats = service.get_destination_stats(LogDestination::System).await.unwrap();
        assert_eq!(stats.entries_written, 1);
        assert_eq!(stats.entries_by_level.get(&LogLevel::Error), Some(&1));
    }
}