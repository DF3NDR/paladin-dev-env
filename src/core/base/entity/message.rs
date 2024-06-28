/* !
Message Entity

Message represents a generic communication package. It contains message content, the 
sender, the receiver and a timestamp. The sender and recievers are generic types that a 
both limited to being a Message Service type or a Node type.

The Message struct is used to to build Message types on top of at higher layers.
 */
use chrono::{DateTime, Utc};
use uuid::Uuid;

// ToDO this could use a better name (Participant, Endpoint)
#[derive(Debug, Clone)]
pub enum Location {
    Node(Uuid),
    // This value should automatically be whatever function or service creates the message.
    // don't know how to make this happen. `String` is a placeholder.
    Service(String),
}


#[derive(Debug, Clone)]
pub struct Message<T> {
    pub source: Location,
    pub destination: Location,
    pub timestamp: DateTime<Utc>,
    pub message: T,
}

impl<T> Message<T> {
    pub fn new(
        source: Source,
        destination: Destination,
        message: T,
    ) -> Self {
        Self {
            source,
            destination,
            timestamp: Utc::now(),
            message,
        }
    }
}
