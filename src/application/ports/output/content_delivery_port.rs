/*

Content Delivery Port

A port that defines how the application delivers content to the user. This would
typically be an HTTP API that the user interacts with to access the application's
functionality.

This port is responsible for translating application use cases into responses that
can be sent back to the user. It provides an abstraction layer that allows the application
to deliver content without being tightly coupled to the details of how that content is
delivered.

Typical implementations of this port would be for the adapter to translate the results
of application use cases into HTTP responses, and to translate incoming HTTP requests
into application use cases. This allows the application to interact with the user through
an HTTP API without being tightly coupled to the details of how that API is implemented.

*/

use crate::core::platform::container::content::ContentItem;
use crate::core::platform::container::content_list::ContentList;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeliveryRequest {
    pub recipient_id: String,
    pub delivery_method: DeliveryMethod,
    pub content_payload: ContentPayload,
    pub priority: DeliveryPriority,
    pub scheduled_time: Option<DateTime<Utc>>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeliveryMethod {
    Http { endpoint: String, headers: Option<HashMap<String, String>> },
    Email { to: String, subject: String },
    Webhook { url: String, method: String },
    Push { device_token: String, title: String },
    Sms { phone_number: String },
    WebSocket { connection_id: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContentPayload {
    SingleItem(ContentItem),
    ContentList(ContentList),
    Notification(NotificationContent),
    Alert(AlertContent),
    Custom(HashMap<String, serde_json::Value>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotificationContent {
    pub title: String,
    pub body: String,
    pub category: String,
    pub action_url: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlertContent {
    pub level: AlertLevel,
    pub message: String,
    pub source: String,
    pub timestamp: DateTime<Utc>,
    pub action_required: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlertLevel {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeliveryPriority {
    Low,
    Normal,
    High,
    Urgent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeliveryResponse {
    pub delivery_id: Uuid,
    pub status: DeliveryStatus,
    pub delivered_at: Option<DateTime<Utc>>,
    pub attempt_count: u32,
    pub error_message: Option<String>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeliveryStatus {
    Pending,
    InProgress,
    Delivered,
    Failed,
    Cancelled,
    Scheduled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeliveryStats {
    pub total_deliveries: u64,
    pub successful_deliveries: u64,
    pub failed_deliveries: u64,
    pub pending_deliveries: u64,
    pub average_delivery_time_ms: Option<u64>,
}

#[derive(Debug, Clone, Error)]
pub enum ContentDeliveryError {
    #[error("Invalid delivery method: {0}")]
    InvalidDeliveryMethod(String),
    #[error("Recipient not found: {0}")]
    RecipientNotFound(String),
    #[error("Delivery failed: {0}")]
    DeliveryFailed(String),
    #[error("Content serialization error: {0}")]
    SerializationError(String),
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Authentication failed")]
    AuthenticationFailed,
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    #[error("Service unavailable")]
    ServiceUnavailable,
}

/// Content Delivery Service Port
/// Defines the contract for delivering content to various destinations
pub trait ContentDeliveryService {
    /// Deliver content using the specified delivery request
    fn deliver_content(&self, request: DeliveryRequest) -> Result<DeliveryResponse, ContentDeliveryError>;
    
    /// Schedule content delivery for a future time
    fn schedule_delivery(&self, request: DeliveryRequest) -> Result<DeliveryResponse, ContentDeliveryError>;
    
    /// Cancel a scheduled delivery
    fn cancel_delivery(&self, delivery_id: Uuid) -> Result<(), ContentDeliveryError>;
    
    /// Get delivery status
    fn get_delivery_status(&self, delivery_id: Uuid) -> Result<DeliveryResponse, ContentDeliveryError>;
    
    /// List deliveries for a recipient
    fn list_deliveries(&self, recipient_id: &str, limit: Option<u32>) -> Result<Vec<DeliveryResponse>, ContentDeliveryError>;
    
    /// Get delivery statistics
    fn get_delivery_stats(&self, recipient_id: Option<&str>) -> Result<DeliveryStats, ContentDeliveryError>;

    /// Validate delivery method configuration
    fn validate_delivery_method(&self, method: &DeliveryMethod) -> Result<(), ContentDeliveryError>;
}

/// Batch Content Delivery Service
/// For high-volume delivery scenarios
pub trait BatchContentDeliveryService {
    /// Deliver multiple content items in batch
    fn batch_deliver(&self, requests: Vec<DeliveryRequest>) -> Result<Vec<DeliveryResponse>, ContentDeliveryError>;
    
    /// Get batch delivery status
    fn get_batch_status(&self, batch_id: Uuid) -> Result<Vec<DeliveryResponse>, ContentDeliveryError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delivery_request_serialization() {
        let request = DeliveryRequest {
            recipient_id: "user123".to_string(),
            delivery_method: DeliveryMethod::Http {
                endpoint: "https://example.com/webhook".to_string(),
                headers: None,
            },
            content_payload: ContentPayload::Notification(NotificationContent {
                title: "Test Notification".to_string(),
                body: "This is a test".to_string(),
                category: "info".to_string(),
                action_url: None,
                expires_at: None,
            }),
            priority: DeliveryPriority::Normal,
            scheduled_time: None,
            metadata: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: DeliveryRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request, deserialized);
    }

    #[test]
    fn test_delivery_response_creation() {
        let response = DeliveryResponse {
            delivery_id: Uuid::new_v4(),
            status: DeliveryStatus::Delivered,
            delivered_at: Some(Utc::now()),
            attempt_count: 1,
            error_message: None,
            metadata: None,
        };

        assert_eq!(response.status, DeliveryStatus::Delivered);
        assert!(response.delivered_at.is_some());
        assert_eq!(response.attempt_count, 1);
    }
}