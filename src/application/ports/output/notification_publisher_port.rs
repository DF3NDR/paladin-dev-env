/*
Notification Publisher Port

This port defines the contract for publishing notifications to various channels.
It includes interface for publishing notifications, scheduling them, and managing their delivery status.
It abstracts the underlying notification delivery mechanisms, allowing the application to interact with the external notification services without being tightly coupled to their implementation details.
Typical implementations of this port would be for the adapter to translate the application's notification requirements into calls to the notification service, and to translate the results of those calls back into a format that the application can use.
This allows the application to interact with the notification service through a clean interface without being  tightly coupled to the details of how that service is implemented.

Some examples of adapters for notifications   typically includes email, SMS, push notifications, and webhooks.
*/

use serde::{Deserialize, Serialize};
use thiserror::Error;
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotificationRequest {
    pub id: Option<Uuid>,
    pub recipient: NotificationRecipient,
    pub content: NotificationContent,
    pub channel: NotificationChannel,
    pub priority: NotificationPriority,
    pub scheduled_time: Option<DateTime<Utc>>,
    pub expiry_time: Option<DateTime<Utc>>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NotificationRecipient {
    Email(String),
    Phone(String),
    UserId(String),
    DeviceToken(String),
    WebhookUrl(String),
    Multiple(Vec<NotificationRecipient>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotificationContent {
    pub title: String,
    pub body: String,
    pub category: String,
    pub action_url: Option<String>,
    pub attachments: Option<Vec<NotificationAttachment>>,
    pub template_id: Option<String>,
    pub template_variables: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotificationAttachment {
    pub filename: String,
    pub content_type: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NotificationChannel {
    Email,
    Sms,
    Push,
    WebPush,
    Webhook,
    InApp,
    Slack,
    Discord,
    Teams,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NotificationPriority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotificationResponse {
    pub id: Uuid,
    pub status: NotificationStatus,
    pub sent_at: Option<DateTime<Utc>>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub read_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub retry_count: u32,
    pub external_id: Option<String>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NotificationStatus {
    Pending,
    Queued,
    Sending,
    Sent,
    Delivered,
    Read,
    Failed,
    Cancelled,
    Expired,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotificationTemplate {
    pub id: String,
    pub name: String,
    pub channel: NotificationChannel,
    pub subject_template: Option<String>,
    pub body_template: String,
    pub variables: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotificationStats {
    pub total_sent: u64,
    pub total_delivered: u64,
    pub total_failed: u64,
    pub total_pending: u64,
    pub delivery_rate: f64,
    pub average_delivery_time_ms: Option<u64>,
    pub channel_breakdown: HashMap<NotificationChannel, u64>,
}

#[derive(Debug, Clone, Error)]
pub enum NotificationError {
    #[error("Invalid recipient: {0}")]
    InvalidRecipient(String),
    #[error("Invalid channel: {0}")]
    InvalidChannel(String),
    #[error("Template not found: {0}")]
    TemplateNotFound(String),
    #[error("Template rendering failed: {0}")]
    TemplateRenderingFailed(String),
    #[error("Notification sending failed: {0}")]
    SendingFailed(String),
    #[error("Notification not found: {0}")]
    NotificationNotFound(String),
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    #[error("Service unavailable")]
    ServiceUnavailable,
    #[error("Authentication failed")]
    AuthenticationFailed,
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
}

/// Notification Publisher Service
/// Main interface for publishing notifications to various channels
pub trait NotificationPublisherService {
    /// Send a notification immediately
    fn send_notification(&self, request: NotificationRequest) -> Result<NotificationResponse, NotificationError>;
    
    /// Schedule a notification for future delivery
    fn schedule_notification(&self, request: NotificationRequest) -> Result<NotificationResponse, NotificationError>;
    
    /// Cancel a scheduled notification
    fn cancel_notification(&self, notification_id: Uuid) -> Result<(), NotificationError>;
    
    /// Get notification status
    fn get_notification_status(&self, notification_id: Uuid) -> Result<NotificationResponse, NotificationError>;
    
    /// List notifications for a recipient
    fn list_notifications(&self, recipient: &NotificationRecipient, limit: Option<u32>) -> Result<Vec<NotificationResponse>, NotificationError>;
    
    /// Get notification statistics
    fn get_notification_stats(&self, channel: Option<NotificationChannel>) -> Result<NotificationStats, NotificationError>;
}

/// Notification Template Service
/// Interface for managing notification templates
pub trait NotificationTemplateService {
    /// Create a new notification template
    fn create_template(&self, template: NotificationTemplate) -> Result<String, NotificationError>;
    
    /// Update an existing template
    fn update_template(&self, template: NotificationTemplate) -> Result<(), NotificationError>;
    
    /// Delete a template
    fn delete_template(&self, template_id: &str) -> Result<(), NotificationError>;
    
    /// Get a template by ID
    fn get_template(&self, template_id: &str) -> Result<NotificationTemplate, NotificationError>;
    
    /// List all templates for a channel
    fn list_templates(&self, channel: Option<NotificationChannel>) -> Result<Vec<NotificationTemplate>, NotificationError>;
    
    /// Render a template with variables
    fn render_template(&self, template_id: &str, variables: HashMap<String, serde_json::Value>) -> Result<NotificationContent, NotificationError>;
}

/// Bulk Notification Service
/// For high-volume notification scenarios
pub trait BulkNotificationService {
    /// Send notifications in bulk
    fn send_bulk_notifications(&self, requests: Vec<NotificationRequest>) -> Result<Vec<NotificationResponse>, NotificationError>;
    
    /// Get bulk operation status
    fn get_bulk_status(&self, batch_id: Uuid) -> Result<Vec<NotificationResponse>, NotificationError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_request_serialization() {
        let request = NotificationRequest {
            id: Some(Uuid::new_v4()),
            recipient: NotificationRecipient::Email("test@example.com".to_string()),
            content: NotificationContent {
                title: "Test Notification".to_string(),
                body: "This is a test notification".to_string(),
                category: "info".to_string(),
                action_url: None,
                attachments: None,
                template_id: None,
                template_variables: None,
            },
            channel: NotificationChannel::Email,
            priority: NotificationPriority::Normal,
            scheduled_time: None,
            expiry_time: None,
            metadata: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: NotificationRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request, deserialized);
    }

    #[test]
    fn test_notification_recipient_email() {
        let recipient = NotificationRecipient::Email("user@example.com".to_string());
        
        match recipient {
            NotificationRecipient::Email(email) => assert_eq!(email, "user@example.com"),
            _ => panic!("Expected email recipient"),
        }
    }

    #[test]
    fn test_notification_status_transitions() {
        let status = NotificationStatus::Pending;
        assert_eq!(status, NotificationStatus::Pending);
        
        let status = NotificationStatus::Sent;
        assert_eq!(status, NotificationStatus::Sent);
    }
}