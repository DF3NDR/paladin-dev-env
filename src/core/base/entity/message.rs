/*
Message Entity

Base message entity for the system. Messages are used for communication between
components and as the foundation for logging and notification systems.

Messages contain source and destination locations, timestamps, priorities,
and can carry any type of payload data.
*/
use std::hash::{Hash, Hasher};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Message priority levels for routing and processing
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MessagePriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl Default for MessagePriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Location identifier for message routing
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Location {
    /// An external system or endpoint
    External(String),
    /// System-level component
    System(String),
    /// Service-level component
    Service(String),
    /// User or client location
    User(String),
}

impl Location {
    /// Create a new service location
    pub fn service(name: &str) -> Self {
        Location::Service(name.to_string())
    }
    
    /// Create a new system location
    pub fn system(name: &str) -> Self {
        Location::System(name.to_string())
    }
    
    /// Create a new external location
    pub fn external(name: &str) -> Self {
        Location::External(name.to_string())
    }
    
    /// Create a new user location
    pub fn user(id: &str) -> Self {
        Location::User(id.to_string())
    }
    
    /// Get the location name
    pub fn name(&self) -> &str {
        match self {
            Location::External(name) => name,
            Location::System(name) => name,
            Location::Service(name) => name,
            Location::User(id) => id,
        }
    }
    
    /// Check if this is a system location
    pub fn is_system(&self) -> bool {
        matches!(self, Location::System(_))
    }
    
    /// Check if this is a service location
    pub fn is_service(&self) -> bool {
        matches!(self, Location::Service(_))
    }
    
    /// Check if this is an external location
    pub fn is_external(&self) -> bool {
        matches!(self, Location::External(_))
    }
}

/// Base message structure
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message<T> {
    /// Unique message identifier
    pub id: Uuid,
    /// Message source location
    pub source: Location,
    /// Message destination location
    pub destination: Location,
    /// Message timestamp
    pub timestamp: DateTime<Utc>,
    /// Message payload
    pub message: T,
    /// Optional correlation ID for message tracking
    pub correlation_id: Option<Uuid>,
    /// Message priority
    pub priority: MessagePriority,
}

impl<T> Hash for Message<T>
where
    T: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.source.hash(state);
        self.destination.hash(state);
        self.message.hash(state);
    }
}

impl<T> Message<T> {
    /// Create a new message with default priority
    pub fn new(
        source: Location,
        destination: Location,
        message: T,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            source,
            destination,
            timestamp: Utc::now(),
            message,
            correlation_id: None,
            priority: MessagePriority::default(),
        }
    }
    
    /// Create a new message with specified priority
    pub fn with_priority(
        source: Location,
        destination: Location,
        message: T,
        priority: MessagePriority,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            source,
            destination,
            timestamp: Utc::now(),
            message,
            correlation_id: None,
            priority,
        }
    }
    
    /// Create a new message with correlation ID
    pub fn with_correlation(
        source: Location,
        destination: Location,
        message: T,
        correlation_id: Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            source,
            destination,
            timestamp: Utc::now(),
            message,
            correlation_id: Some(correlation_id),
            priority: MessagePriority::default(),
        }
    }
    
    /// Create a complete message with all options
    pub fn complete(
        source: Location,
        destination: Location,
        message: T,
        priority: MessagePriority,
        correlation_id: Option<Uuid>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            source,
            destination,
            timestamp: Utc::now(),
            message,
            correlation_id,
            priority,
        }
    }
    
    /// Set correlation ID
    pub fn set_correlation_id(&mut self, correlation_id: Uuid) {
        self.correlation_id = Some(correlation_id);
    }
    
    /// Set priority
    pub fn set_priority(&mut self, priority: MessagePriority) {
        self.priority = priority;
    }
    
    /// Get message age in seconds
    pub fn age_seconds(&self) -> i64 {
        Utc::now().timestamp() - self.timestamp.timestamp()
    }
    
    /// Check if message is expired based on TTL
    pub fn is_expired(&self, ttl_seconds: i64) -> bool {
        self.age_seconds() > ttl_seconds
    }
    
    /// Check if message has correlation ID
    pub fn has_correlation(&self) -> bool {
        self.correlation_id.is_some()
    }
    
    /// Get a reference to the message payload
    pub fn payload(&self) -> &T {
        &self.message
    }
    
    /// Get a mutable reference to the message payload
    pub fn payload_mut(&mut self) -> &mut T {
        &mut self.message
    }
    
    /// Transform the message payload to a different type
    pub fn map<U, F>(self, f: F) -> Message<U>
    where
        F: FnOnce(T) -> U,
    {
        Message {
            id: self.id,
            source: self.source,
            destination: self.destination,
            timestamp: self.timestamp,
            message: f(self.message),
            correlation_id: self.correlation_id,
            priority: self.priority,
        }
    }
    
    /// Create a reply message to this message
    pub fn reply<U>(&self, reply_message: U) -> Message<U> {
        Message {
            id: Uuid::new_v4(),
            source: self.destination.clone(),
            destination: self.source.clone(),
            timestamp: Utc::now(),
            message: reply_message,
            correlation_id: Some(self.id),
            priority: self.priority,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let source = Location::service("test-service");
        let destination = Location::system("test-system");
        let message = Message::new(source.clone(), destination.clone(), "test message".to_string());
        
        assert_eq!(message.source, source);
        assert_eq!(message.destination, destination);
        assert_eq!(message.message, "test message");
        assert_eq!(message.priority, MessagePriority::Normal);
        assert!(message.correlation_id.is_none());
    }

    #[test]
    fn test_message_with_priority() {
        let source = Location::service("test");
        let destination = Location::system("test");
        let message = Message::with_priority(
            source, 
            destination, 
            "urgent".to_string(), 
            MessagePriority::Critical
        );
        
        assert_eq!(message.priority, MessagePriority::Critical);
    }

    #[test]
    fn test_location_creation() {
        let service_loc = Location::service("my-service");
        assert_eq!(service_loc.name(), "my-service");
        assert!(service_loc.is_service());
        assert!(!service_loc.is_system());
        
        let system_loc = Location::system("core");
        assert!(system_loc.is_system());
        assert!(!system_loc.is_external());
    }

    #[test]
    fn test_message_reply() {
        let original = Message::new(
            Location::service("client"),
            Location::service("server"),
            "request".to_string(),
        );
        
        let reply = original.reply("response".to_string());
        
        assert_eq!(reply.source, original.destination);
        assert_eq!(reply.destination, original.source);
        assert_eq!(reply.correlation_id, Some(original.id));
        assert_eq!(reply.message, "response");
    }

    #[test]
    fn test_message_age() {
        let message = Message::new(
            Location::service("test"),
            Location::system("test"),
            "test".to_string(),
        );
        
        // Message should be very new (less than 1 second old)
        assert!(message.age_seconds() < 1);
        assert!(!message.is_expired(60)); // Not expired with 60 second TTL
    }

    #[test]
    fn test_message_priority_ordering() {
        assert!(MessagePriority::Critical > MessagePriority::High);
        assert!(MessagePriority::High > MessagePriority::Normal);
        assert!(MessagePriority::Normal > MessagePriority::Low);
    }
}