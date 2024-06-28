/*
Action Component

An Action Component is a struct that represents an action that can be taken in the system.
It contains the Service Call, the Action Type, the source of the Action,
and the Timestamp when the action was created and when it occurred.

A typical Action called by a Trigger in response to an Event. The Trigger will
initiate the Action and a call to the Action Service which will call the 
Service with the Arguments.

This gives a way for the system to respond to Events and to take Actions based on 
those Events in a decoupled way.
*/
use chrono::{DateTime, Utc};

pub struct Action {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source: String,
    pub service: String,
    pub arguments: String,
    pub timestamp: DateTime<Utc>,
}

impl Action {
    pub fn new(
        id: String,
        name: String,
        description: String,
        source: String,
        service: String,
        arguments: String,
    ) -> Self {
        Action {
            id,
            name,
            description,
            source,
            service,
            arguments,
            timestamp: Utc::now(),
        }
    }
}