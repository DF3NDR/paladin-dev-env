/*
Event entity

This module contains the Event entity and its related traits and implementations.

An Event is a struct that represents an event that has occurred in the system. It contains
information about the event, such as the event type, the source of the event, and the timestamp
when the event occurred.

*/
use chrono::{DateTime, Utc};

pub struct Event {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source: String,
    pub event_type: String,
    pub timestamp: DateTime<Utc>,
}

impl Event {
    pub fn new(
        id: String,
        name: String,
        description: String,
        source: String,
        event_type: String,
    ) -> Self {
        Event {
            id,
            name,
            description,
            source,
            event_type,
            timestamp: Utc::now(),
        }
    }
}