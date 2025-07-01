/*
Notification Ports

This module defines the output ports (interfaces) for the notification system following
Hexagonal Architecture principles. These ports provide clean abstractions that allow
the application layer to interact with external notification delivery mechanisms
without being coupled to their implementation details.

The ports are segregated based on Interface Segregation Principle (ISP) to ensure
that implementations only need to implement the functionality they actually provide.

Port Segregation:
- NotificationDeliveryPort: Core delivery functionality
- NotificationSchedulingPort: Scheduling and timing operations  
- NotificationTemplatePort: Template management and rendering
- NotificationQueryPort: Read operations and status queries
- NotificationStoragePort: Persistence operations
- NotificationEventPort: Event publishing for notification lifecycle

Each port defines a focused interface that can be implemented independently,
allowing for flexible adapter implementations and better testability.
*/

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

// Re-export domain types for convenience
pub use crate::core::platform::container::notification::{
    Notification, NotificationContent, NotificationChannel, NotificationPriority,
    NotificationRecipient, NotificationStatus, NotificationTemplate, NotificationEvent,
    NotificationAttachment, NotificationDomainError,
};

/// Result type for notification port operations
pub type NotificationPortResult<T> = Result<T, NotificationPortError>;

/// Errors that can occur in notification port operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum NotificationPortError {
    #[error("Domain error: {0}")]
    DomainError(#[from] NotificationDomainError),
    
    #[error("Delivery failed: {0}")]
    DeliveryFailed(String),
    
    #[error("Template error: {0}")]
    TemplateError(String),
    
    #[error("Storage error: {0}")]
    StorageError(String),
    
    #[error("Connection error: {0}")]
    ConnectionError(String),
    
    #[error("Authentication error: {0}")]
    AuthenticationError(String),
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
    
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Timeout error")]
    Timeout,
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Delivery result for notification operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationDeliveryResult {
    /// Notification ID
    pub notification_id: Uuid,
    /// Delivery status
    pub status: NotificationStatus,
    /// External service ID if available
    pub external_id: Option<String>,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
    /// Error message if delivery failed
    pub error_message: Option<String>,
    /// Delivery timestamp
    pub delivered_at: DateTime<Utc>,
    /// Channel used for delivery
    pub channel: NotificationChannel,
    /// Metadata from the delivery adapter
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Bulk delivery result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkDeliveryResult {
    /// Total notifications processed
    pub total_count: usize,
    /// Successful deliveries
    pub success_count: usize,
    /// Failed deliveries
    pub failure_count: usize,
    /// Individual delivery results
    pub results: Vec<NotificationDeliveryResult>,
    /// Overall processing time
    pub total_processing_time_ms: u64,
}

/// Notification statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationStats {
    /// Total notifications sent
    pub total_sent: u64,
    /// Total notifications delivered
    pub total_delivered: u64,
    /// Total notifications failed
    pub total_failed: u64,
    /// Total notifications pending
    pub total_pending: u64,
    /// Delivery rate (delivered/sent)
    pub delivery_rate: f64,
    /// Average delivery time in milliseconds
    pub average_delivery_time_ms: Option<u64>,
    /// Statistics by channel
    pub channel_breakdown: HashMap<NotificationChannel, ChannelStats>,
    /// Statistics by priority
    pub priority_breakdown: HashMap<NotificationPriority, u64>,
    /// Time period for these statistics
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

/// Channel-specific statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelStats {
    /// Notifications sent through this channel
    pub sent: u64,
    /// Notifications delivered through this channel
    pub delivered: u64,
    /// Notifications failed through this channel
    pub failed: u64,
    /// Average delivery time for this channel
    pub avg_delivery_time_ms: Option<u64>,
    /// Last successful delivery
    pub last_success: Option<DateTime<Utc>>,
    /// Last failure
    pub last_failure: Option<DateTime<Utc>>,
}

/// Query filters for notifications
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NotificationQuery {
    /// Filter by recipient
    pub recipient: Option<NotificationRecipient>,
    /// Filter by channel
    pub channel: Option<NotificationChannel>,
    /// Filter by status
    pub status: Option<NotificationStatus>,
    /// Filter by priority
    pub priority: Option<NotificationPriority>,
    /// Filter by date range
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    /// Limit number of results
    pub limit: Option<u32>,
    /// Offset for pagination
    pub offset: Option<u32>,
    /// Sort order
    pub sort_by: Option<NotificationSortField>,
    pub sort_order: Option<SortOrder>,
}

/// Fields available for sorting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationSortField {
    CreatedAt,
    UpdatedAt,
    Priority,
    Status,
    Channel,
}

/// Sort order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortOrder {
    Ascending,
    Descending,
}

// ============================================================================
// OUTPUT PORTS (INTERFACES)
// ============================================================================

/// Core notification delivery port
/// 
/// This port defines the essential delivery functionality that all notification
/// adapters must implement. It focuses purely on the delivery mechanism.
#[async_trait]
pub trait NotificationDeliveryPort: Send + Sync {
    /// Get the channel this port handles
    fn channel(&self) -> NotificationChannel;
    
    /// Check if this port can handle the given notification
    fn can_handle(&self, notification: &Notification) -> bool;
    
    /// Deliver a single notification
    async fn deliver_notification(&self, notification: Notification) -> NotificationPortResult<NotificationDeliveryResult>;
    
    /// Deliver multiple notifications (if supported)
    async fn deliver_bulk(&self, notifications: Vec<Notification>) -> NotificationPortResult<BulkDeliveryResult> {
        // Default implementation delivers one by one
        let mut results = Vec::new();
        let start_time = std::time::Instant::now();
        let mut success_count = 0;
        let mut failure_count = 0;
        
        for notification in notifications {
            match self.deliver_notification(notification).await {
                Ok(result) => {
                    if matches!(result.status, NotificationStatus::Sent | NotificationStatus::Delivered) {
                        success_count += 1;
                    } else {
                        failure_count += 1;
                    }
                    results.push(result);
                }
                Err(error) => {
                    failure_count += 1;
                    // Create a failure result
                    results.push(NotificationDeliveryResult {
                        notification_id: Uuid::new_v4(), // We don't have access to the original ID
                        status: NotificationStatus::Failed,
                        external_id: None,
                        processing_time_ms: 0,
                        error_message: Some(error.to_string()),
                        delivered_at: Utc::now(),
                        channel: self.channel(),
                        metadata: HashMap::new(),
                    });
                }
            }
        }
        
        Ok(BulkDeliveryResult {
            total_count: results.len(),
            success_count,
            failure_count,
            results,
            total_processing_time_ms: start_time.elapsed().as_millis() as u64,
        })
    }
    
    /// Check delivery port health
    async fn health_check(&self) -> bool;
    
    /// Get delivery port capabilities
    fn capabilities(&self) -> DeliveryCapabilities;
}

/// Delivery capabilities supported by a port
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryCapabilities {
    /// Supports bulk delivery
    pub supports_bulk: bool,
    /// Supports delivery receipts
    pub supports_receipts: bool,
    /// Supports attachments
    pub supports_attachments: bool,
    /// Supports rich content (HTML, etc.)
    pub supports_rich_content: bool,
    /// Supports templates
    pub supports_templates: bool,
    /// Maximum attachment size in bytes
    pub max_attachment_size: Option<usize>,
    /// Rate limits (messages per minute)
    pub rate_limit: Option<u32>,
}

/// Notification template port
///
/// This port handles template management and content rendering.
#[async_trait]
pub trait NotificationTemplatePort: Send + Sync {
    /// Create a new template
    async fn create_template(&self, template: NotificationTemplate) -> NotificationPortResult<String>;
    
    /// Update an existing template
    async fn update_template(&self, template: NotificationTemplate) -> NotificationPortResult<()>;
    
    /// Delete a template
    async fn delete_template(&self, template_id: &str) -> NotificationPortResult<()>;
    
    /// Get a template by ID
    async fn get_template(&self, template_id: &str) -> NotificationPortResult<NotificationTemplate>;
    
    /// List templates with optional filtering
    async fn list_templates(&self, channel: Option<NotificationChannel>) -> NotificationPortResult<Vec<NotificationTemplate>>;
    
    /// Render template with variables
    async fn render_template(
        &self,
        template_id: &str,
        variables: HashMap<String, serde_json::Value>,
    ) -> NotificationPortResult<NotificationContent>;
    
    /// Validate template syntax
    async fn validate_template(&self, template: &NotificationTemplate) -> NotificationPortResult<()>;
}

/// Basic notification port for simple use cases
///
/// This trait combines just delivery functionality for simple notification adapters.
#[async_trait]
pub trait BasicNotificationPort: NotificationDeliveryPort + Send + Sync {
    /// Send a notification and return delivery result
    async fn send_notification(&self, notification: Notification) -> NotificationPortResult<NotificationDeliveryResult> {
        self.deliver_notification(notification).await
    }
}

/// Configuration for notification ports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPortConfig {
    /// Channel-specific configurations
    pub channels: HashMap<NotificationChannel, serde_json::Value>,
    /// Global settings
    pub global: HashMap<String, serde_json::Value>,
}

impl NotificationPortConfig {
    /// Get configuration for a specific channel
    pub fn get_channel_config(&self, channel: &NotificationChannel) -> Option<&serde_json::Value> {
        self.channels.get(channel)
    }
    
    /// Get global configuration value
    pub fn get_global_config(&self, key: &str) -> Option<&serde_json::Value> {
        self.global.get(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_query_default() {
        let query = NotificationQuery::default();
        assert!(query.recipient.is_none());
        assert!(query.channel.is_none());
        assert!(query.limit.is_none());
    }

    #[test]
    fn test_delivery_capabilities() {
        let capabilities = DeliveryCapabilities {
            supports_bulk: true,
            supports_receipts: true,
            supports_attachments: false,
            supports_rich_content: true,
            supports_templates: true,
            max_attachment_size: Some(10 * 1024 * 1024), // 10MB
            rate_limit: Some(100), // 100 per minute
        };
        
        assert!(capabilities.supports_bulk);
        assert!(!capabilities.supports_attachments);
        assert_eq!(capabilities.max_attachment_size, Some(10 * 1024 * 1024));
    }

    #[test]
    fn test_notification_port_error() {
        let error = NotificationPortError::DeliveryFailed("Test error".to_string());
        assert!(error.to_string().contains("Test error"));
    }
}
