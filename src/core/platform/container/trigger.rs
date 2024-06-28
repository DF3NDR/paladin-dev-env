/*
Trigger Component

A Trigger is a message element sent to a manager service to take action. Triggers are 
used to respond to Events. This module contains the Trigger Component and its related traits
and implementations.
*/
use super::action::Action;
use chrono::{DateTime, Utc};

pub struct Trigger {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source: String,
    pub target: String,
    pub action: Action,
    pub timestamp: DateTime<Utc>,
}

impl Trigger {
    pub fn new(
        id: String,
        name: String,
        description: String,
        source: String,
        target: String,
        action: Action,
    ) -> Self {
        Trigger {
            id,
            name,
            description,
            source,
            target,
            action,
            timestamp: Utc::now(),
        }
    }
}