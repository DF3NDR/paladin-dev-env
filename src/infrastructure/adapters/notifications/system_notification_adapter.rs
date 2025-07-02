/*
System Notification Adapter

This adapter implements system/in-app notifications for internal application use.
It provides immediate feedback to users within the application interface.

Features:
- In-memory notification storage
- Real-time notification delivery
- Simple status tracking
- Basic health monitoring

This is useful for alerts, confirmations, and status updates within the application.
*/

use crate::application::ports::output::notification_port::{
    NotificationDeliveryPort, BasicNotificationPort,
    NotificationPortResult, NotificationPortError, NotificationDeliveryResult,
    DeliveryCapabilities,
    NotificationChannel, NotificationStatus,
    Notification, NotificationRecipient,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use chrono::{DateTime, Utc};

/// System notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemAdapterConfig {
    /// Maximum number of notifications to store in memory
    pub max_stored_notifications: usize,
    /// Auto-cleanup old notifications after this duration (seconds)
    pub cleanup_interval_seconds: u64,
}

impl Default for SystemAdapterConfig {
    fn default() -> Self {
        Self {
            max_stored_notifications: 1000,
            cleanup_interval_seconds: 3600, // 1 hour
        }
    }
}

/// System notification adapter
#[derive(Debug)]
pub struct SystemNotificationAdapter {
    config: SystemAdapterConfig,
    notifications: Arc<RwLock<Vec<Notification>>>,
    delivery_stats: Arc<RwLock<SystemDeliveryStats>>,
}

/// Internal delivery statistics
#[derive(Debug, Default)]
struct SystemDeliveryStats {
    total_delivered: u64,
    last_delivery: Option<DateTime<Utc>>,
}

impl SystemNotificationAdapter {
    /// Create a new system notification adapter
    pub fn new(config: SystemAdapterConfig) -> Self {
        Self {
            config,
            notifications: Arc::new(RwLock::new(Vec::new())),
            delivery_stats: Arc::new(RwLock::new(SystemDeliveryStats::default())),
        }
    }

    /// Get all stored notifications for a recipient
    pub fn get_notifications_for_recipient(&self, recipient: &NotificationRecipient) -> NotificationPortResult<Vec<Notification>> {
        let notifications = self.notifications.read().map_err(|_| {
            NotificationPortError::ServiceUnavailable("Notification storage unavailable".to_string())
        })?;
        
        let filtered: Vec<Notification> = notifications
            .iter()
            .filter(|n| &n.recipient == recipient)
            .cloned()
            .collect();
            
        Ok(filtered)
    }

    /// Clear old notifications based on cleanup policy
    pub fn cleanup_old_notifications(&self) -> NotificationPortResult<usize> {
        let mut notifications = self.notifications.write().map_err(|_| {
            NotificationPortError::ServiceUnavailable("Notification storage unavailable".to_string())
        })?;

        let before_count = notifications.len();
        
        // Simple cleanup: keep only the most recent notifications
        if notifications.len() > self.config.max_stored_notifications {
            notifications.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            notifications.truncate(self.config.max_stored_notifications);
        }
        
        let after_count = notifications.len();
        Ok(before_count - after_count)
    }
}

// ============================================================================
// PORT IMPLEMENTATIONS
// ============================================================================

#[async_trait]
impl NotificationDeliveryPort for SystemNotificationAdapter {
    fn channel(&self) -> NotificationChannel {
        NotificationChannel::System
    }

    fn can_handle(&self, notification: &Notification) -> bool {
        notification.channel == NotificationChannel::System
    }

    async fn deliver_notification(&self, mut notification: Notification) -> NotificationPortResult<NotificationDeliveryResult> {
        let start_time = Instant::now();
        
        // Validate notification
        if !self.can_handle(&notification) {
            return Err(NotificationPortError::ValidationError(
                "System adapter cannot handle this notification".to_string()
            ));
        }

        // For system notifications, delivery is immediate (just store it)
        notification.status = NotificationStatus::Delivered;
        
        // Store notification
        {
            let mut notifications = self.notifications.write().map_err(|_| {
                NotificationPortError::ServiceUnavailable("Notification storage unavailable".to_string())
            })?;
            notifications.push(notification.clone());
        }

        // Update stats
        {
            let mut stats = self.delivery_stats.write().map_err(|_| {
                NotificationPortError::ServiceUnavailable("Stats unavailable".to_string())
            })?;
            stats.total_delivered += 1;
            stats.last_delivery = Some(Utc::now());
        }

        let processing_time = start_time.elapsed().as_millis() as u64;
        
        let mut metadata = HashMap::new();
        metadata.insert("delivery_method".to_string(), serde_json::Value::String("in_memory".to_string()));
        metadata.insert("stored_at".to_string(), serde_json::Value::String(Utc::now().to_rfc3339()));
        
        Ok(NotificationDeliveryResult {
            notification_id: notification.id,
            status: NotificationStatus::Delivered,
            external_id: Some(notification.id.to_string()),
            processing_time_ms: processing_time,
            error_message: None,
            delivered_at: Utc::now(),
            channel: NotificationChannel::System,
            metadata,
        })
    }

    async fn health_check(&self) -> bool {
        // Simple health check - verify we can access storage
        self.notifications.read().is_ok() && self.delivery_stats.read().is_ok()
    }

    fn capabilities(&self) -> DeliveryCapabilities {
        DeliveryCapabilities {
            supports_bulk: true,
            supports_receipts: false,
            supports_attachments: false, // System notifications are typically simple
            supports_rich_content: true,
            supports_templates: false, // Can be added later
            max_attachment_size: None,
            rate_limit: None, // No rate limiting for system notifications
        }
    }
}

// Implement BasicNotificationPort trait for convenience
#[async_trait]
impl BasicNotificationPort for SystemNotificationAdapter {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::platform::container::notification::{NotificationContent, NotificationPriority};

    fn create_test_system_notification() -> Notification {
        let content = NotificationContent::new(
            "System Alert".to_string(),
            "This is a test system notification".to_string(),
            "system".to_string(),
        );
        
        Notification::new(
            NotificationRecipient::SystemComponent("user123".to_string()),
            content,
            NotificationChannel::System,
            NotificationPriority::High,
        ).unwrap()
    }

    #[test]
    fn test_system_adapter_creation() {
        let config = SystemAdapterConfig::default();
        let adapter = SystemNotificationAdapter::new(config);
        assert_eq!(adapter.channel(), NotificationChannel::System);
    }

    #[test]
    fn test_can_handle_notification() {
        let config = SystemAdapterConfig::default();
        let adapter = SystemNotificationAdapter::new(config);
        let notification = create_test_system_notification();
        
        assert!(adapter.can_handle(&notification));
    }

    #[tokio::test]
    async fn test_deliver_system_notification() {
        let config = SystemAdapterConfig::default();
        let adapter = SystemNotificationAdapter::new(config);
        let notification = create_test_system_notification();
        let recipient = notification.recipient.clone();
        
        // Deliver notification
        let result = adapter.deliver_notification(notification).await;
        assert!(result.is_ok());
        
        let delivery_result = result.unwrap();
        assert_eq!(delivery_result.status, NotificationStatus::Delivered);
        assert_eq!(delivery_result.channel, NotificationChannel::System);
        
        // Verify notification was stored
        let stored_notifications = adapter.get_notifications_for_recipient(&recipient).unwrap();
        assert_eq!(stored_notifications.len(), 1);
        assert_eq!(stored_notifications[0].status, NotificationStatus::Delivered);
    }

    #[tokio::test]
    async fn test_health_check() {
        let config = SystemAdapterConfig::default();
        let adapter = SystemNotificationAdapter::new(config);
        
        let is_healthy = adapter.health_check().await;
        assert!(is_healthy);
    }

    #[test]
    fn test_capabilities() {
        let config = SystemAdapterConfig::default();
        let adapter = SystemNotificationAdapter::new(config);
        let capabilities = adapter.capabilities();
        
        assert!(capabilities.supports_bulk);
        assert!(capabilities.supports_rich_content);
        assert!(!capabilities.supports_attachments);
        assert!(!capabilities.supports_receipts);
        assert!(capabilities.rate_limit.is_none());
    }

    #[test]
    fn test_cleanup_old_notifications() {
        let config = SystemAdapterConfig {
            max_stored_notifications: 2,
            cleanup_interval_seconds: 3600,
        };
        let adapter = SystemNotificationAdapter::new(config);
        
        // Add more notifications than the limit
        {
            let mut notifications = adapter.notifications.write().unwrap();
            for _i in 0..5 {
                let mut notification = create_test_system_notification();
                notification.id = uuid::Uuid::new_v4();
                notifications.push(notification);
            }
        }
        
        // Cleanup
        let cleaned_count = adapter.cleanup_old_notifications().unwrap();
        assert_eq!(cleaned_count, 3); // 5 - 2 = 3 cleaned
        
        // Verify only 2 notifications remain
        let remaining = adapter.notifications.read().unwrap();
        assert_eq!(remaining.len(), 2);
    }
}
