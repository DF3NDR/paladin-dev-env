/*
Logging Container

The Log Container is a type of Node that has a UUID with a vector of LogEntry items. 
The Log can be abstracted in the application layer so that multiple Logs may exist. 

By defining only the most fundamental requirements within the Core we allow the application
to handle all the details that it requires while still enabling the other containers and
services at this layer to meet their requirements. The event_manager, messaging_service,
etc. require at least one log to be defined.

The Log Entry Container is a type of Message that an Event will Trigger. It contains the
log message, the log level, and the timestamp when the log entry was created.

The Config Service Settings requires at least one log location to be defined.
*/
use crate::core::base::entity::message::{Message, Location, MessagePriority};
use crate::core::base::entity::node::Node;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

/// Log severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
    Fatal = 5,
}

impl LogLevel {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Fatal => "FATAL",
        }
    }
    
    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "TRACE" => Some(LogLevel::Trace),
            "DEBUG" => Some(LogLevel::Debug),
            "INFO" => Some(LogLevel::Info),
            "WARN" | "WARNING" => Some(LogLevel::Warn),
            "ERROR" => Some(LogLevel::Error),
            "FATAL" => Some(LogLevel::Fatal),
            _ => None,
        }
    }
    
    /// Convert to message priority
    pub fn to_priority(&self) -> MessagePriority {
        match self {
            LogLevel::Trace | LogLevel::Debug => MessagePriority::Low,
            LogLevel::Info => MessagePriority::Normal,
            LogLevel::Warn => MessagePriority::High,
            LogLevel::Error | LogLevel::Fatal => MessagePriority::Critical,
        }
    }
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Info
    }
}

/// Log destinations for routing log entries
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LogDestination {
    /// General system operations log
    System,
    /// HTTP access and API requests
    Access,
    /// Error conditions and exceptions
    Error,
    /// Security-related events
    Security,
    /// Performance and metrics data
    Performance,
    /// Custom log destination
    Custom(String),
}

impl LogDestination {
    /// Convert to Location for message routing
    pub fn to_location(&self) -> Location {
        match self {
            LogDestination::System => Location::service("system-log"),
            LogDestination::Access => Location::service("access-log"),
            LogDestination::Error => Location::service("error-log"),
            LogDestination::Security => Location::service("security-log"),
            LogDestination::Performance => Location::service("performance-log"),
            LogDestination::Custom(name) => Location::service(&format!("custom-log-{}", name)),
        }
    }
    
    /// Get the log destination name
    pub fn name(&self) -> String {
        match self {
            LogDestination::System => "system".to_string(),
            LogDestination::Access => "access".to_string(),
            LogDestination::Error => "error".to_string(),
            LogDestination::Security => "security".to_string(),
            LogDestination::Performance => "performance".to_string(),
            LogDestination::Custom(name) => name.clone(),
        }
    }
}

/// Log message content
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LogMessage {
    /// Log severity level
    pub level: LogLevel,
    /// The actual log message
    pub message: String,
    /// Optional module or component that generated the log
    pub module: Option<String>,
    /// Optional function or method name
    pub function: Option<String>,
    /// Optional file name and line number
    pub location: Option<String>,
    /// Optional additional context data
    pub context: Option<serde_json::Value>,
}

impl LogMessage {
    /// Create a new log message
    pub fn new(level: LogLevel, message: String) -> Self {
        Self {
            level,
            message,
            module: None,
            function: None,
            location: None,
            context: None,
        }
    }
    
    /// Create with module information
    pub fn with_module(mut self, module: String) -> Self {
        self.module = Some(module);
        self
    }
    
    /// Create with function information
    pub fn with_function(mut self, function: String) -> Self {
        self.function = Some(function);
        self
    }
    
    /// Create with location information
    pub fn with_location(mut self, location: String) -> Self {
        self.location = Some(location);
        self
    }
    
    /// Create with context data
    pub fn with_context(mut self, context: serde_json::Value) -> Self {
        self.context = Some(context);
        self
    }
    
    /// Get formatted message for display
    pub fn formatted(&self) -> String {
        let mut parts = vec![
            format!("[{}]", self.level.as_str()),
            self.message.clone(),
        ];
        
        if let Some(module) = &self.module {
            parts.push(format!("module:{}", module));
        }
        
        if let Some(function) = &self.function {
            parts.push(format!("fn:{}", function));
        }
        
        if let Some(location) = &self.location {
            parts.push(format!("at:{}", location));
        }
        
        parts.join(" ")
    }
}

/// Type alias for log entries - Messages containing LogMessage
pub type LogEntry = Message<LogMessage>;

/// Helper functions for creating LogEntry instances
pub struct LogEntryBuilder;

impl LogEntryBuilder {
    /// Create a new log entry
    pub fn new_entry(
        source: Location,
        destination: LogDestination,
        level: LogLevel,
        message: String,
    ) -> LogEntry {
        let log_message = LogMessage::new(level, message);
        let priority = level.to_priority();
        
        Message::with_priority(
            source,
            destination.to_location(),
            log_message,
            priority,
        )
    }
    
    /// Create a log entry with additional context
    pub fn new_entry_with_context(
        source: Location,
        destination: LogDestination,
        level: LogLevel,
        message: String,
        module: Option<String>,
        function: Option<String>,
        location: Option<String>,
        context: Option<serde_json::Value>,
    ) -> LogEntry {
        let mut log_message = LogMessage::new(level, message);
        
        if let Some(module) = module {
            log_message = log_message.with_module(module);
        }
        
        if let Some(function) = function {
            log_message = log_message.with_function(function);
        }
        
        if let Some(location) = location {
            log_message = log_message.with_location(location);
        }
        
        if let Some(context) = context {
            log_message = log_message.with_context(context);
        }
        
        let priority = level.to_priority();
        
        Message::with_priority(
            source,
            destination.to_location(),
            log_message,
            priority,
        )
    }
}

/// Extension trait for LogEntry to add log-specific functionality
pub trait LogEntryExt {
    /// Get the log level of this entry
    fn level(&self) -> LogLevel;
    
    /// Get the formatted message
    fn formatted_message(&self) -> String;
    
    /// Check if this entry matches a minimum log level
    fn matches_level(&self, min_level: LogLevel) -> bool;
}

impl LogEntryExt for LogEntry {
    fn level(&self) -> LogLevel {
        self.message.level
    }
    
    fn formatted_message(&self) -> String {
        self.message.formatted()
    }
    
    fn matches_level(&self, min_level: LogLevel) -> bool {
        self.message.level >= min_level
    }
}

/// Log container data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogContainer {
    /// Name of this log
    pub name: String,
    /// Log destination type
    pub destination: LogDestination,
    /// Minimum log level to accept
    pub min_level: LogLevel,
    /// Maximum number of entries to keep (0 = unlimited)
    pub max_entries: usize,
    /// Current log entries
    pub entries: Vec<LogEntry>,
    /// Whether this log is currently active
    pub active: bool,
}

impl Hash for LogContainer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.destination.hash(state);
    }
}

impl LogContainer {
    /// Create a new log container
    pub fn new(name: String, destination: LogDestination, min_level: LogLevel) -> Self {
        Self {
            name,
            destination,
            min_level,
            max_entries: 0, // Unlimited by default
            entries: Vec::new(),
            active: true,
        }
    }
    
    /// Create with maximum entries limit
    pub fn with_max_entries(mut self, max_entries: usize) -> Self {
        self.max_entries = max_entries;
        self
    }
    
    /// Add a log entry if it meets the minimum level requirement
    pub fn add_entry(&mut self, entry: LogEntry) -> bool {
        if !self.active || !entry.matches_level(self.min_level) {
            return false;
        }
        
        self.entries.push(entry);
        
        // Enforce max entries limit
        if self.max_entries > 0 && self.entries.len() > self.max_entries {
            self.entries.remove(0); // Remove oldest entry
        }
        
        true
    }
    
    /// Get entries matching a minimum level
    pub fn get_entries(&self, min_level: Option<LogLevel>) -> Vec<&LogEntry> {
        match min_level {
            Some(level) => self.entries.iter().filter(|e| e.matches_level(level)).collect(),
            None => self.entries.iter().collect(),
        }
    }
    
    /// Get entries within a time range
    pub fn get_entries_since(&self, since: DateTime<Utc>) -> Vec<&LogEntry> {
        self.entries.iter().filter(|e| e.timestamp >= since).collect()
    }
    
    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
    }
    
    /// Get entry count
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
    
    /// Enable/disable this log
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }
    
    /// Check if log is active
    pub fn is_active(&self) -> bool {
        self.active
    }
    
    /// Set minimum log level
    pub fn set_min_level(&mut self, level: LogLevel) {
        self.min_level = level;
    }
}

/// Type alias for Log as a Node containing LogContainer
pub type Log = Node<LogContainer>;

/// Extension trait for Log to add log-specific functionality
pub trait LogExt {
    /// Create a new log node
    fn new_log(name: String, destination: LogDestination, min_level: LogLevel) -> Self;
    
    /// Add an entry to this log
    fn add_entry(&mut self, entry: LogEntry) -> bool;
    
    /// Get the log destination
    fn destination(&self) -> &LogDestination;
    
    /// Get the minimum log level
    fn min_level(&self) -> LogLevel;
}

impl LogExt for Log {
    fn new_log(name: String, destination: LogDestination, min_level: LogLevel) -> Self {
        let container = LogContainer::new(name.clone(), destination, min_level);
        Node::new(container, Some(name))
    }
    
    fn add_entry(&mut self, entry: LogEntry) -> bool {
        let result = self.node.add_entry(entry);
        if result {
            self.modified = Utc::now();
        }
        result
    }
    
    fn destination(&self) -> &LogDestination {
        &self.node.destination
    }
    
    fn min_level(&self) -> LogLevel {
        self.node.min_level
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_conversion() {
        assert_eq!(LogLevel::from_str("INFO"), Some(LogLevel::Info));
        assert_eq!(LogLevel::from_str("error"), Some(LogLevel::Error));
        assert_eq!(LogLevel::from_str("INVALID"), None);
        
        assert_eq!(LogLevel::Info.as_str(), "INFO");
        assert_eq!(LogLevel::Error.to_priority(), MessagePriority::Critical);
    }

    #[test]
    fn test_log_message_creation() {
        let msg = LogMessage::new(LogLevel::Info, "Test message".to_string())
            .with_module("test_module".to_string())
            .with_function("test_function".to_string());
        
        assert_eq!(msg.level, LogLevel::Info);
        assert_eq!(msg.message, "Test message");
        assert_eq!(msg.module, Some("test_module".to_string()));
        assert_eq!(msg.function, Some("test_function".to_string()));
    }

    #[test]
    fn test_log_entry_creation() {
        let source = Location::service("test-service");
        let entry = LogEntryBuilder::new_entry(
            source,
            LogDestination::System,
            LogLevel::Info,
            "Test log entry".to_string(),
        );
        
        assert_eq!(entry.level(), LogLevel::Info);
        assert_eq!(entry.message.message, "Test log entry");
        assert_eq!(entry.destination, LogDestination::System.to_location());
    }

    #[test]
    fn test_log_container() {
        let mut container = LogContainer::new(
            "test-log".to_string(),
            LogDestination::System,
            LogLevel::Info,
        );
        
        let entry = LogEntryBuilder::new_entry(
            Location::service("test"),
            LogDestination::System,
            LogLevel::Info,
            "Test message".to_string(),
        );
        
        assert!(container.add_entry(entry));
        assert_eq!(container.entry_count(), 1);
        
        // Test level filtering
        let debug_entry = LogEntryBuilder::new_entry(
            Location::service("test"),
            LogDestination::System,
            LogLevel::Debug,
            "Debug message".to_string(),
        );
        
        assert!(!container.add_entry(debug_entry)); // Should be filtered out
        assert_eq!(container.entry_count(), 1);
    }

    #[test]
    fn test_log_node() {
        let mut log = Log::new_log(
            "system-log".to_string(),
            LogDestination::System,
            LogLevel::Info,
        );
        
        let entry = LogEntryBuilder::new_entry(
            Location::service("test"),
            LogDestination::System,
            LogLevel::Warn,
            "Warning message".to_string(),
        );
        
        let initial_modified = log.modified;
        assert!(log.add_entry(entry));
        assert!(log.modified > initial_modified);
        assert_eq!(log.node.entry_count(), 1);
    }
}