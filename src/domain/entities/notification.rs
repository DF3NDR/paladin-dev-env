// src/domain/entities/notification.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Notification {
    pub title: String,
    pub message: String,
    pub recipient: String,
    pub timestamp: u64,
}
