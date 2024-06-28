/*
Logging Container

This is actually two containers, the Log Container and the Log Entry Container.

The Log Container is a type of Node that has a UUID with a vector of type  Log Entry Field. 
The Log can be abstracted in the application layer so that multiple Logs may exist. 

By defining only the most fundamental requirements within the Core we allow the application
to handle all the details that it requires while still enabling the other containers and
services at this layer to meet their requirements.  The event_manager, messaging_service,
etc. require at least one log to be defined.

The Log Entry Container is a type of Message that an Event will Trigger. It contains the
log message, the log level, and the timestamp when the log entry was created.

The Config Service Settings requires at least one log location to be defined.
*/
use crate::core::base::entity::message::{Message, Source, Destination};
use chrono::Utc;

#[derive(Debug, Clone)]
pub struct LogMessage {
    pub level: String,
    pub message: String,
}

enum LogLevel {
    Info,
    Warning,
    Error,
    Debug,
}

// This needs to add to the message::Destination
enum LogDestination {
    ErrorLog,
    AccessLog,
    SystemLog,
}

pub type LogEntry = Message<LogMessage>;

impl LogEntry {
    pub fn new(
        source: Source,
        destination: LogDestination,
        level: String,
        message: LogMessage,
    ) -> Self {
        Self {
            source,
            destination,
            timestamp: Utc::now(),
            message: LogMessage { level, message },
        }
    }
}
