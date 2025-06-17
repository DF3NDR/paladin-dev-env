/*
Notificaiton

Notification Type is a Message Type.


*/
use crate::core::base::entity::message::{Message, Location};

use chrono::Utc;

#[derive(Debug, Clone)]
pub struct NotificationMessage {
    pub title: String,
    pub body: String,
}

pub type Notification = Message<NotificationMessage>;

impl Notification {
    pub fn new(
        source: Location,
        destination: Location,
        title: String,
        body: String,
    ) -> Self {
        Self {
            source,
            destination,
            timestamp: Utc::now(),
            message: NotificationMessage { title, body },
        }
    }
    
    // ToDo Create NotificationManger 
    // pub fn send(&self) {
    //     println!("Notification Sent: {:?}", self);
    //     NotificationManager::send(self);
    // }
}

