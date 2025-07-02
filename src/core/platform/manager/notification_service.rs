/*
Notification Service

Platform-level notification service that orchestrates notification operations and integrates
with the core Message Service foundation. This service provides specialized notification
functionality while building upon the generalized messaging capabilities.

This service follows the hexagonal architecture pattern and provides:
- Notification lifecycle management
- Channel routing and delivery coordination
- Template processing and content generation
- Event-driven notification handling
- Integration with external notification adapters

The service acts as a platform orchestrator that application-layer use cases can build upon
for specific notification scenarios and business requirements.
*/

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::core::base::entity::message::{Message, Location};
use crate::core::base::service::message_service::{MessageService, MessageHandler, MessageResult, MessageError};
use crate::core::platform::container::notification::{
    Notification, NotificationContent, NotificationChannel, NotificationPriority,
    NotificationRecipient, NotificationStatus, NotificationTemplate, NotificationEvent,
    NotificationDomainError,
};

/// Result type for notification service operations
pub type NotificationServiceResult<T> = Result<T, NotificationServiceError>;

/// Errors that can occur in notification service operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum NotificationServiceError {
    #[error("Domain error: {0}")]
    DomainError(#[from] NotificationDomainError),
    
    #[error("Message service error: {0}")]
    MessageError(#[from] MessageError),
    
    #[error("Template not found: {0}")]
    TemplateNotFound(String),
    
    #[error("Channel not available: {0}")]
    ChannelNotAvailable(String),
    
    #[error("Delivery failed: {0}")]
    DeliveryFailed(String),
    
    #[error("Service not initialized")]
    ServiceNotInitialized,
    
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Storage error: {0}")]
    StorageError(String),
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Statistics for notification service operations
#[derive(Debug, Clone, Default)]
pub struct NotificationServiceStats {
    /// Total notifications created
    pub notifications_created: u64,
    /// Total notifications sent
    pub notifications_sent: u64,
    /// Total notifications delivered
    pub notifications_delivered: u64,
    /// Total notifications failed
    pub notifications_failed: u64,
    /// Notifications by channel
    pub channel_stats: HashMap<NotificationChannel, u64>,
    /// Notifications by priority
    pub priority_stats: HashMap<NotificationPriority, u64>,
    /// Average delivery time in milliseconds
    pub avg_delivery_time_ms: Option<u64>,
    /// Last activity timestamp
    pub last_activity: Option<DateTime<Utc>>,
    /// Active notifications count
    pub active_notifications: u64,
}

/// Configuration for the notification service
#[derive(Debug, Clone)]
pub struct NotificationServiceConfig {
    /// Default maximum retries for failed notifications
    pub default_max_retries: u32,
    /// Default expiry time for notifications in seconds
    pub default_expiry_seconds: i64,
    /// Enable notification persistence
    pub enable_persistence: bool,
    /// Maximum notifications to process per batch
    pub batch_size: usize,
    /// Processing interval in milliseconds
    pub processing_interval_ms: u64,
    /// Template cache size
    pub template_cache_size: usize,
    /// Maximum attachment size in bytes
    pub max_attachment_size: usize,
}

impl Default for NotificationServiceConfig {
    fn default() -> Self {
        Self {
            default_max_retries: 3,
            default_expiry_seconds: 86400, // 24 hours
            enable_persistence: true,
            batch_size: 100,
            processing_interval_ms: 1000,
            template_cache_size: 1000,
            max_attachment_size: 25 * 1024 * 1024, // 25MB
        }
    }
}

/// Trait for notification channel handlers
#[async_trait]
pub trait NotificationChannelHandler: Send + Sync {
    /// Get the channel this handler supports
    fn channel(&self) -> NotificationChannel;
    
    /// Check if this handler can process the notification
    fn can_handle(&self, notification: &Notification) -> bool;
    
    /// Process a notification for delivery
    async fn handle_notification(&self, notification: Notification) -> NotificationServiceResult<NotificationDeliveryResult>;
    
    /// Check handler health
    async fn health_check(&self) -> bool;
}

/// Result of notification delivery attempt
#[derive(Debug, Clone)]
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
    pub timestamp: DateTime<Utc>,
}

/// Trait for template processing
#[async_trait]
pub trait NotificationTemplateProcessor: Send + Sync {
    /// Render template with variables
    async fn render_template(
        &self,
        template: &NotificationTemplate,
        variables: &HashMap<String, serde_json::Value>,
    ) -> NotificationServiceResult<NotificationContent>;
    
    /// Validate template syntax
    async fn validate_template(&self, template: &NotificationTemplate) -> NotificationServiceResult<()>;
}

/// Platform notification service implementation
pub struct NotificationService {
    /// Service configuration
    config: NotificationServiceConfig,
    /// Underlying message service
    message_service: Arc<MessageService>,
    /// Channel handlers by channel type
    channel_handlers: Arc<RwLock<HashMap<NotificationChannel, Arc<dyn NotificationChannelHandler>>>>,
    /// Template processor
    template_processor: Arc<RwLock<Option<Arc<dyn NotificationTemplateProcessor>>>>,
    /// Template cache
    template_cache: Arc<RwLock<HashMap<String, NotificationTemplate>>>,
    /// Active notifications
    active_notifications: Arc<RwLock<HashMap<Uuid, Notification>>>,
    /// Service statistics
    stats: Arc<RwLock<NotificationServiceStats>>,
    /// Event publishers
    event_senders: Arc<RwLock<Vec<mpsc::UnboundedSender<NotificationEvent>>>>,
    /// Processing workers
    workers: Arc<RwLock<Vec<tokio::task::JoinHandle<()>>>>,
    /// Service state
    is_running: Arc<RwLock<bool>>,
}

impl NotificationService {
    /// Create new notification service
    pub fn new(config: NotificationServiceConfig, message_service: Arc<MessageService>) -> Self {
        Self {
            config,
            message_service,
            channel_handlers: Arc::new(RwLock::new(HashMap::new())),
            template_processor: Arc::new(RwLock::new(None)),
            template_cache: Arc::new(RwLock::new(HashMap::new())),
            active_notifications: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(NotificationServiceStats::default())),
            event_senders: Arc::new(RwLock::new(Vec::new())),
            workers: Arc::new(RwLock::new(Vec::new())),
            is_running: Arc::new(RwLock::new(false)),
        }
    }
    
    /// Start the notification service
    pub async fn start(&self) -> NotificationServiceResult<()> {
        println!("Starting notification service");
        
        {
            let mut is_running = self.is_running.write().await;
            if *is_running {
                return Ok(());
            }
            *is_running = true;
        }
        
        // Start the message service
        self.message_service.start().await?;
        
        // Register notification message handler
        let notification_handler = Arc::new(NotificationMessageHandler::new(
            self.channel_handlers.clone(),
            self.stats.clone(),
        ));
        
        self.message_service
            .register_handler("notification".to_string(), notification_handler)
            .await?;
        
        // Start processing workers
        self.start_workers().await?;
        
        println!("Notification service started successfully");
        Ok(())
    }
    
    /// Stop the notification service
    pub async fn stop(&self) -> NotificationServiceResult<()> {
        println!("Stopping notification service");
        
        {
            let mut is_running = self.is_running.write().await;
            if !*is_running {
                return Ok(());
            }
            *is_running = false;
        }
        
        // Stop workers
        let mut workers = self.workers.write().await;
        for worker in workers.drain(..) {
            worker.abort();
        }
        
        // Stop message service
        self.message_service.stop().await?;
        
        println!("Notification service stopped");
        Ok(())
    }
    
    /// Create a new notification
    pub async fn create_notification(
        &self,
        recipient: NotificationRecipient,
        content: NotificationContent,
        channel: NotificationChannel,
        priority: NotificationPriority,
    ) -> NotificationServiceResult<Notification> {
        println!(
            "Creating notification for recipient: {:?}, channel: {:?}",
            recipient, channel
        );
        
        // Create the notification
        let mut notification = Notification::new(recipient, content, channel, priority)?;
        
        // Set default expiry if not specified
        if notification.expiry_time.is_none() {
            let expiry = Utc::now() + chrono::Duration::seconds(self.config.default_expiry_seconds);
            notification.set_expiry(expiry)?;
        }
        
        // Set max retries
        notification.max_retries = self.config.default_max_retries;
        
        // Store notification
        {
            let mut active = self.active_notifications.write().await;
            active.insert(notification.id, notification.clone());
        }
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.notifications_created += 1;
            *stats.channel_stats.entry(notification.channel.clone()).or_insert(0) += 1;
            *stats.priority_stats.entry(notification.priority).or_insert(0) += 1;
            stats.last_activity = Some(Utc::now());
            stats.active_notifications += 1;
        }
        
        // Publish event
        self.publish_event(NotificationEvent::NotificationCreated {
            notification_id: notification.id,
            recipient: notification.recipient.clone(),
            channel: notification.channel.clone(),
            priority: notification.priority,
            created_at: notification.created_at,
        }).await;
        
        println!("Notification created: {}", notification.id);
        Ok(notification)
    }
    
    /// Send a notification immediately
    pub async fn send_notification(&self, notification_id: Uuid) -> NotificationServiceResult<NotificationDeliveryResult> {
        println!("Sending notification: {}", notification_id);
        
        let mut notification = {
            let mut active = self.active_notifications.write().await;
            active.remove(&notification_id)
                .ok_or_else(|| NotificationServiceError::ValidationError(
                    format!("Notification not found: {}", notification_id)
                ))?
        };
        
        // Check if notification is expired
        if notification.is_expired() {
            notification.update_status(NotificationStatus::Expired)?;
            self.publish_event(NotificationEvent::NotificationExpired {
                notification_id: notification.id,
                expired_at: Utc::now(),
            }).await;
            
            return Ok(NotificationDeliveryResult {
                notification_id: notification.id,
                status: NotificationStatus::Expired,
                external_id: None,
                processing_time_ms: 0,
                error_message: Some("Notification expired".to_string()),
                timestamp: Utc::now(),
            });
        }
        
        // Process template if needed
        if notification.content.uses_template() {
            notification.content = self.process_template(&notification.content).await?;
        }
        
        // Update status to queued
        notification.update_status(NotificationStatus::Queued)?;
        self.publish_event(NotificationEvent::NotificationQueued {
            notification_id: notification.id,
            channel: notification.channel.clone(),
            queued_at: Utc::now(),
        }).await;
        
        // Find appropriate handler
        let handler = {
            let handlers = self.channel_handlers.read().await;
            handlers.get(&notification.channel)
                .cloned()
                .ok_or_else(|| NotificationServiceError::ChannelNotAvailable(
                    format!("No handler available for channel: {:?}", notification.channel)
                ))?
        };
        
        // Send notification
        let result = handler.handle_notification(notification.clone()).await;
        
        match result {
            Ok(delivery_result) => {
                // Update notification status
                notification.update_status(delivery_result.status.clone())?;
                notification.external_id = delivery_result.external_id.clone();
                
                // Publish events based on status
                match delivery_result.status {
                    NotificationStatus::Sent => {
                        self.publish_event(NotificationEvent::NotificationSent {
                            notification_id: notification.id,
                            channel: notification.channel.clone(),
                            external_id: delivery_result.external_id.clone(),
                            sent_at: delivery_result.timestamp,
                        }).await;
                    }
                    NotificationStatus::Failed => {
                        self.publish_event(NotificationEvent::NotificationFailed {
                            notification_id: notification.id,
                            error: delivery_result.error_message.clone().unwrap_or_default(),
                            retry_count: notification.retry_count,
                            failed_at: delivery_result.timestamp,
                        }).await;
                    }
                    _ => {}
                }
                
                // Update statistics
                {
                    let mut stats = self.stats.write().await;
                    match delivery_result.status {
                        NotificationStatus::Sent | NotificationStatus::Delivered => {
                            stats.notifications_sent += 1;
                            if delivery_result.status == NotificationStatus::Delivered {
                                stats.notifications_delivered += 1;
                            }
                        }
                        NotificationStatus::Failed => {
                            stats.notifications_failed += 1;
                        }
                        _ => {}
                    }
                    stats.last_activity = Some(Utc::now());
                    stats.active_notifications -= 1;
                }
                
                println!("Notification {} processed with status: {:?}", notification_id, delivery_result.status);
                Ok(delivery_result)
            }
            Err(error) => {
                eprintln!("Failed to send notification {}: {}", notification_id, error);
                
                // Check if we can retry
                if notification.can_retry() {
                    if let Some(retry_status) = notification.status.next_retry(notification.max_retries) {
                        notification.update_status(retry_status)?;
                        
                        // Re-queue for retry
                        let retry_count = notification.retry_count;
                        {
                            let mut active = self.active_notifications.write().await;
                            active.insert(notification.id, notification);
                        }
                        
                        return Ok(NotificationDeliveryResult {
                            notification_id,
                            status: NotificationStatus::Retry(retry_count),
                            external_id: None,
                            processing_time_ms: 0,
                            error_message: Some(error.to_string()),
                            timestamp: Utc::now(),
                        });
                    }
                }
                
                // Final failure
                notification.update_status(NotificationStatus::Failed)?;
                self.publish_event(NotificationEvent::NotificationFailed {
                    notification_id: notification.id,
                    error: error.to_string(),
                    retry_count: notification.retry_count,
                    failed_at: Utc::now(),
                }).await;
                
                let mut stats = self.stats.write().await;
                stats.notifications_failed += 1;
                stats.active_notifications -= 1;
                
                Err(error)
            }
        }
    }
    
    /// Schedule a notification for future delivery
    pub async fn schedule_notification(
        &self,
        notification_id: Uuid,
        scheduled_time: DateTime<Utc>,
    ) -> NotificationServiceResult<()> {
        println!("Scheduling notification {} for {}", notification_id, scheduled_time);
        
        let mut active = self.active_notifications.write().await;
        let notification = active.get_mut(&notification_id)
            .ok_or_else(|| NotificationServiceError::ValidationError(
                format!("Notification not found: {}", notification_id)
            ))?;
        
        notification.schedule(scheduled_time)?;
        
        self.publish_event(NotificationEvent::NotificationScheduled {
            notification_id: notification.id,
            scheduled_time,
            scheduled_at: Utc::now(),
        }).await;
        
        println!("Notification {} scheduled for {}", notification_id, scheduled_time);
        Ok(())
    }
    
    /// Cancel a notification
    pub async fn cancel_notification(&self, notification_id: Uuid, reason: Option<String>) -> NotificationServiceResult<()> {
        println!("Cancelling notification: {}", notification_id);
        
        let mut active = self.active_notifications.write().await;
        if let Some(mut notification) = active.remove(&notification_id) {
            notification.update_status(NotificationStatus::Cancelled)?;
            
            self.publish_event(NotificationEvent::NotificationCancelled {
                notification_id: notification.id,
                reason,
                cancelled_at: Utc::now(),
            }).await;
            
            let mut stats = self.stats.write().await;
            stats.active_notifications -= 1;
            
            println!("Notification {} cancelled", notification_id);
        }
        
        Ok(())
    }
    
    /// Register a channel handler
    pub async fn register_channel_handler(&self, handler: Arc<dyn NotificationChannelHandler>) {
        let channel = handler.channel();
        println!("Registering channel handler for: {:?}", channel);
        
        let mut handlers = self.channel_handlers.write().await;
        handlers.insert(channel, handler);
    }
    
    /// Set template processor
    pub async fn set_template_processor(&self, processor: Arc<dyn NotificationTemplateProcessor>) {
        println!("Setting template processor");
        let mut template_processor = self.template_processor.write().await;
        *template_processor = Some(processor);
    }
    
    /// Add template to cache
    pub async fn cache_template(&self, template: NotificationTemplate) -> NotificationServiceResult<()> {
        template.validate()?;
        
        let mut cache = self.template_cache.write().await;
        cache.insert(template.id.clone(), template);
        
        Ok(())
    }
    
    /// Get service statistics
    pub async fn get_stats(&self) -> NotificationServiceStats {
        let stats = self.stats.read().await;
        stats.clone()
    }
    
    /// Get active notifications count
    pub async fn get_active_count(&self) -> usize {
        let active = self.active_notifications.read().await;
        active.len()
    }
    
    /// Subscribe to notification events
    pub async fn subscribe_to_events(&self) -> mpsc::UnboundedReceiver<NotificationEvent> {
        let (sender, receiver) = mpsc::unbounded_channel();
        let mut senders = self.event_senders.write().await;
        senders.push(sender);
        receiver
    }
    
    /// Check service health
    pub async fn health_check(&self) -> bool {
        let is_running = *self.is_running.read().await;
        let message_service_healthy = self.message_service.health_check().await.unwrap_or(false);
        
        // Check channel handlers health
        let handlers = self.channel_handlers.read().await;
        let mut healthy_handlers = 0;
        for handler in handlers.values() {
            if handler.health_check().await {
                healthy_handlers += 1;
            }
        }
        
        is_running && message_service_healthy && (handlers.is_empty() || healthy_handlers > 0)
    }
    
    // Private helper methods
    
    /// Process template for notification content
    async fn process_template(&self, content: &NotificationContent) -> NotificationServiceResult<NotificationContent> {
        let template_id = content.template_id.as_ref()
            .ok_or_else(|| NotificationServiceError::ValidationError("No template ID specified".to_string()))?;
        
        // Get template from cache
        let template = {
            let cache = self.template_cache.read().await;
            cache.get(template_id).cloned()
                .ok_or_else(|| NotificationServiceError::TemplateNotFound(template_id.clone()))?
        };
        
        // Get template processor
        let processor = {
            let processor_guard = self.template_processor.read().await;
            processor_guard.as_ref()
                .ok_or_else(|| NotificationServiceError::ConfigurationError("No template processor configured".to_string()))?
                .clone()
        };
        
        // Render template
        processor.render_template(&template, &content.template_variables).await
    }
    
    /// Start processing workers
    async fn start_workers(&self) -> NotificationServiceResult<()> {
        println!("Starting notification processing workers");
        
        let mut workers = self.workers.write().await;
        
        // Start scheduled notification processor
        let scheduled_processor = self.start_scheduled_processor().await;
        workers.push(scheduled_processor);
        
        // Start retry processor
        let retry_processor = self.start_retry_processor().await;
        workers.push(retry_processor);
        
        Ok(())
    }
    
    /// Start scheduled notification processor
    async fn start_scheduled_processor(&self) -> tokio::task::JoinHandle<()> {
        let active_notifications = self.active_notifications.clone();
        let _channel_handlers = self.channel_handlers.clone();
        let _stats = self.stats.clone();
        let interval = self.config.processing_interval_ms;
        
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(tokio::time::Duration::from_millis(interval));
            
            loop {
                interval_timer.tick().await;
                
                // Get notifications ready to send
                let ready_notifications: Vec<Uuid> = {
                    let active = active_notifications.read().await;
                    active.values()
                        .filter(|n| n.should_send_now())
                        .map(|n| n.id)
                        .collect()
                };
                
                // Process ready notifications
                for notification_id in ready_notifications {
                    // In a real implementation, this would call the notification service
                    // For now, we'll just log it
                    println!("Would send scheduled notification: {}", notification_id);
                }
            }
        })
    }
    
    /// Start retry processor
    async fn start_retry_processor(&self) -> tokio::task::JoinHandle<()> {
        let active_notifications = self.active_notifications.clone();
        let _channel_handlers = self.channel_handlers.clone();
        let _stats = self.stats.clone();
        let interval = self.config.processing_interval_ms * 2; // Less frequent than scheduled processor
        
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(tokio::time::Duration::from_millis(interval));
            
            loop {
                interval_timer.tick().await;
                
                // Get notifications ready for retry
                let retry_notifications: Vec<Uuid> = {
                    let active = active_notifications.read().await;
                    active.values()
                        .filter(|n| n.can_retry() && matches!(n.status, NotificationStatus::Retry(_)))
                        .map(|n| n.id)
                        .collect()
                };
                
                // Process retry notifications
                for notification_id in retry_notifications {
                    // In a real implementation, this would call the notification service
                    // For now, we'll just log it
                    println!("Would retry notification: {}", notification_id);
                    
                    // Add delay between retries
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
        })
    }
    
    /// Publish notification event
    async fn publish_event(&self, event: NotificationEvent) {
        let senders = self.event_senders.read().await;
        let mut failed_senders = Vec::new();
        
        for (index, sender) in senders.iter().enumerate() {
            if sender.send(event.clone()).is_err() {
                failed_senders.push(index);
            }
        }
        
        // Remove failed senders
        if !failed_senders.is_empty() {
            drop(senders);
            let mut senders = self.event_senders.write().await;
            for &index in failed_senders.iter().rev() {
                senders.remove(index);
            }
        }
    }
}

/// Message handler for notifications
struct NotificationMessageHandler {
    #[allow(dead_code)]
    channel_handlers: Arc<RwLock<HashMap<NotificationChannel, Arc<dyn NotificationChannelHandler>>>>,
    #[allow(dead_code)]
    stats: Arc<RwLock<NotificationServiceStats>>,
}

impl NotificationMessageHandler {
    fn new(
        channel_handlers: Arc<RwLock<HashMap<NotificationChannel, Arc<dyn NotificationChannelHandler>>>>,
        stats: Arc<RwLock<NotificationServiceStats>>,
    ) -> Self {
        Self {
            channel_handlers,
            stats,
        }
    }
}

#[async_trait]
impl MessageHandler<NotificationContent> for NotificationMessageHandler {
    async fn handle_message(&self, message: Message<NotificationContent>) -> MessageResult<()> {
        println!("Handling notification message: {}", message.id);
        
        // This handler processes notifications sent through the message system
        // In practice, this would coordinate with the notification service
        // to deliver the notification through the appropriate channel
        
        Ok(())
    }
    
    fn supported_destinations(&self) -> Vec<Location> {
        vec![
            Location::service("notification"),
            Location::external("notification"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::base::service::message_service::MessageServiceConfig;

    #[tokio::test]
    async fn test_notification_service_creation() {
        let config = NotificationServiceConfig::default();
        let message_service = Arc::new(MessageService::new(MessageServiceConfig::default()));
        let service = NotificationService::new(config, message_service);
        
        assert!(!service.health_check().await); // Not started yet
    }

    #[tokio::test]
    async fn test_notification_creation() {
        let config = NotificationServiceConfig::default();
        let message_service = Arc::new(MessageService::new(MessageServiceConfig::default()));
        let service = NotificationService::new(config, message_service);
        
        let recipient = NotificationRecipient::Email("test@example.com".to_string());
        let content = NotificationContent::new(
            "Test".to_string(),
            "Test body".to_string(),
            "test".to_string(),
        );
        
        let notification = service.create_notification(
            recipient,
            content,
            NotificationChannel::Email,
            NotificationPriority::Normal,
        ).await;
        
        assert!(notification.is_ok());
        let stats = service.get_stats().await;
        assert_eq!(stats.notifications_created, 1);
    }
}


