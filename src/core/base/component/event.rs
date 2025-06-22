/*
Event Component 

This module contains the Event component and its related traits and implementations.

An Event represents a significant occurrence within the system. These are used for communication 
between different parts of the system, triggering actions, messages, etc.  It contains
information about the event, such as the event type, the source of the event, and the timestamp
when the event occurred.

This component is used to handle events in a structured way, allowing for easy management and
processing of events within the system. It can be used to implement event-driven architectures,
where components can listen for specific events and react accordingly.
It is designed to be flexible and extensible, allowing for the addition of new event types, 
containers, and properties as needed.
*/
/*
Event Component 

This module contains the Event component and its related traits and implementations.

An Event represents a significant occurrence within the system. These are used for communication 
between different parts of the system, triggering actions, messages, etc.  It contains
information about the event, such as the event type, the source of the event, and the timestamp
when the event occurred.

This component is used to handle events in a structured way, allowing for easy management and
processing of events within the system. It can be used to implement event-driven architectures,
where components can listen for specific events and react accordingly.
It is designed to be flexible and extensible, allowing for the addition of new event types, 
containers, and properties as needed.
*/

use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use serde_json::Value;
use uuid::Uuid;

/// Represents an event in the In4me system.
/// 
/// Events are domain entities that represent significant occurrences within the system.
/// They are used for communication between different parts of the system, triggering
/// actions, and implementing event-driven architectures.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Event {
    /// Unique identifier for the event
    pub id: Uuid,
    /// Type of the event (e.g., "content_ingested", "user_created")
    pub event_type: String,
    /// JSON payload containing event-specific data
    pub payload: Value,
    /// Timestamp when the event occurred
    pub timestamp: DateTime<Utc>,
    /// Source component or service that generated the event
    pub source: String,
    /// Optional correlation ID for tracking related events
    pub correlation_id: Option<Uuid>,
    /// Event version for schema evolution
    pub version: String,
}

impl Event {
    /// Creates a new event with the given type, payload, and source.
    pub fn new(event_type: String, payload: Value, source: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type,
            payload,
            timestamp: Utc::now(),
            source,
            correlation_id: None,
            version: "1.0".to_string(),
        }
    }

    /// Creates a new event with correlation ID for tracking related events.
    pub fn new_with_correlation(
        event_type: String, 
        payload: Value, 
        source: String, 
        correlation_id: Uuid
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type,
            payload,
            timestamp: Utc::now(),
            source,
            correlation_id: Some(correlation_id),
            version: "1.0".to_string(),
        }
    }

    /// Creates a new event with a specific version for schema evolution.
    pub fn new_versioned(
        event_type: String, 
        payload: Value, 
        source: String, 
        version: String
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type,
            payload,
            timestamp: Utc::now(),
            source,
            correlation_id: None,
            version,
        }
    }

    /// Checks if this event is of a specific type.
    pub fn is_type(&self, event_type: &str) -> bool {
        self.event_type == event_type
    }

    /// Extracts a specific field from the payload.
    pub fn get_payload_field(&self, field: &str) -> Option<&Value> {
        self.payload.get(field)
    }

    /// Creates a new event derived from this one (useful for event chains).
    pub fn derive_event(&self, new_type: String, new_payload: Value, new_source: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type: new_type,
            payload: new_payload,
            timestamp: Utc::now(),
            source: new_source,
            correlation_id: Some(self.correlation_id.unwrap_or(self.id)),
            version: self.version.clone(),
        }
    }
}

impl Default for Event {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type: "unknown".to_string(),
            payload: Value::Null,
            timestamp: Utc::now(),
            source: "system".to_string(),
            correlation_id: None,
            version: "1.0".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_event_creation() {
        let event = Event::new(
            "test_event".to_string(),
            json!({"key": "value"}),
            "test_source".to_string(),
        );

        assert_eq!(event.event_type, "test_event");
        assert_eq!(event.payload, json!({"key": "value"}));
        assert_eq!(event.source, "test_source");
        assert_eq!(event.version, "1.0");
        assert!(event.correlation_id.is_none());
    }

    #[test]
    fn test_event_with_correlation() {
        let correlation_id = Uuid::new_v4();
        let event = Event::new_with_correlation(
            "correlated_event".to_string(),
            json!({"data": "test"}),
            "test_source".to_string(),
            correlation_id,
        );

        assert_eq!(event.correlation_id, Some(correlation_id));
    }

    #[test]
    fn test_event_versioned() {
        let event = Event::new_versioned(
            "versioned_event".to_string(),
            json!({"data": "test"}),
            "test_source".to_string(),
            "2.0".to_string(),
        );

        assert_eq!(event.version, "2.0");
    }

    #[test]
    fn test_is_type() {
        let event = Event::new(
            "test_event".to_string(),
            json!({}),
            "test_source".to_string(),
        );

        assert!(event.is_type("test_event"));
        assert!(!event.is_type("other_event"));
    }

    #[test]
    fn test_get_payload_field() {
        let event = Event::new(
            "test_event".to_string(),
            json!({"key": "value", "number": 42}),
            "test_source".to_string(),
        );

        assert_eq!(event.get_payload_field("key"), Some(&json!("value")));
        assert_eq!(event.get_payload_field("number"), Some(&json!(42)));
        assert_eq!(event.get_payload_field("missing"), None);
    }

    #[test]
    fn test_derive_event() {
        let original = Event::new(
            "original_event".to_string(),
            json!({"data": "original"}),
            "original_source".to_string(),
        );

        let derived = original.derive_event(
            "derived_event".to_string(),
            json!({"data": "derived"}),
            "derived_source".to_string(),
        );

        assert_eq!(derived.event_type, "derived_event");
        assert_eq!(derived.payload, json!({"data": "derived"}));
        assert_eq!(derived.source, "derived_source");
        assert_eq!(derived.correlation_id, Some(original.id));
        assert_eq!(derived.version, original.version);
    }

    #[test]
    fn test_default_event() {
        let event = Event::default();
        assert_eq!(event.event_type, "unknown");
        assert_eq!(event.payload, Value::Null);
        assert_eq!(event.source, "system");
        assert_eq!(event.version, "1.0");
    }
}