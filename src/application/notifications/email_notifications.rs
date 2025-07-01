/*
Email Notification Application Service

This application service coordinates email notification operations using the notification
ports and domain logic. It implements the application layer use cases for email notifications
while delegating to the appropriate ports for delivery, storage, and other concerns.

This service follows the hexagonal architecture pattern by depending on abstractions (ports)
rather than concrete implementations, enabling testability and flexibility.
*/

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::application::ports::output::notification_port::{
    NotificationDeliveryPort, NotificationTemplatePort, NotificationPortResult, 
    NotificationDeliveryResult, BulkDeliveryResult, DeliveryCapabilities,
    NotificationPortError,
};
use crate::core::platform::container::notification::{
    Notification, NotificationContent, NotificationChannel, NotificationPriority,
    NotificationRecipient, NotificationTemplate, NotificationStatus,
};

/// Email notification application service
/// 
/// This service provides high-level email notification operations for the application layer.
/// It coordinates between domain logic and infrastructure concerns through ports.
pub struct EmailNotificationService {
    /// Email delivery port
    delivery_port: Arc<dyn NotificationDeliveryPort>,
    /// Template processing port (optional)
    template_port: Option<Arc<dyn NotificationTemplatePort>>,
}

impl EmailNotificationService {
    /// Create new email notification service
    pub fn new(
        delivery_port: Arc<dyn NotificationDeliveryPort>,
        template_port: Option<Arc<dyn NotificationTemplatePort>>,
    ) -> Self {
        Self {
            delivery_port,
            template_port,
        }
    }
    
    /// Send a simple email notification
    pub async fn send_simple_email(
        &self,
        to: String,
        subject: String,
        body: String,
        priority: Option<NotificationPriority>,
    ) -> NotificationPortResult<NotificationDeliveryResult> {
        // Create notification content
        let content = NotificationContent::new(subject, body, "email".to_string());
        
        // Create recipient
        let recipient = NotificationRecipient::Email(to);
        
        // Create notification
        let notification = Notification::new(
            recipient,
            content,
            NotificationChannel::Email,
            priority.unwrap_or(NotificationPriority::Normal),
        ).map_err(|e| NotificationPortError::ValidationError(e.to_string()))?;
        
        // Send through delivery port
        self.delivery_port.deliver_notification(notification).await
    }
    
    /// Send email with template
    pub async fn send_templated_email(
        &self,
        to: String,
        template_id: String,
        variables: HashMap<String, serde_json::Value>,
        priority: Option<NotificationPriority>,
    ) -> NotificationPortResult<NotificationDeliveryResult> {
        // Get template port
        let template_port = self.template_port.as_ref()
            .ok_or_else(|| NotificationPortError::ConfigurationError(
                "No template port configured".to_string()
            ))?;
        
        // Render template
        let content = template_port.render_template(&template_id, variables).await?;
        
        // Create recipient
        let recipient = NotificationRecipient::Email(to);
        
        // Create notification
        let notification = Notification::new(
            recipient,
            content,
            NotificationChannel::Email,
            priority.unwrap_or(NotificationPriority::Normal),
        ).map_err(|e| NotificationPortError::ValidationError(e.to_string()))?;
        
        // Send through delivery port
        self.delivery_port.deliver_notification(notification).await
    }
    
    /// Send bulk emails
    pub async fn send_bulk_emails(
        &self,
        emails: Vec<EmailRequest>,
    ) -> NotificationPortResult<BulkDeliveryResult> {
        let mut notifications = Vec::new();
        
        for email_req in emails {
            let content = if let Some(template_id) = email_req.template_id {
                // Use template
                let template_port = self.template_port.as_ref()
                    .ok_or_else(|| NotificationPortError::ConfigurationError(
                        "No template port configured for templated email".to_string()
                    ))?;
                
                template_port.render_template(&template_id, email_req.template_variables.unwrap_or_default()).await?
            } else {
                // Simple content
                NotificationContent::new(
                    email_req.subject.unwrap_or_default(),
                    email_req.body.unwrap_or_default(),
                    "email".to_string(),
                )
            };
            
            let recipient = NotificationRecipient::Email(email_req.to);
            
            let notification = Notification::new(
                recipient,
                content,
                NotificationChannel::Email,
                email_req.priority.unwrap_or(NotificationPriority::Normal),
            ).map_err(|e| NotificationPortError::ValidationError(e.to_string()))?;
            
            notifications.push(notification);
        }
        
        // Send through delivery port
        self.delivery_port.deliver_bulk(notifications).await
    }
    
    /// Send email with attachments
    pub async fn send_email_with_attachments(
        &self,
        to: String,
        subject: String,
        body: String,
        attachments: Vec<EmailAttachment>,
        priority: Option<NotificationPriority>,
    ) -> NotificationPortResult<NotificationDeliveryResult> {
        // Check if delivery port supports attachments
        let capabilities = self.delivery_port.capabilities();
        if !capabilities.supports_attachments {
            return Err(NotificationPortError::ConfigurationError(
                "Email delivery port does not support attachments".to_string()
            ));
        }
        
        // Create notification content with attachments
        let mut content = NotificationContent::new(subject, body, "email".to_string());
        
        for attachment in attachments {
            let notification_attachment = crate::core::platform::container::notification::NotificationAttachment::new(
                attachment.filename,
                attachment.content_type,
                attachment.data,
            );
            content.add_attachment(notification_attachment);
        }
        
        // Create recipient
        let recipient = NotificationRecipient::Email(to);
        
        // Create notification
        let notification = Notification::new(
            recipient,
            content,
            NotificationChannel::Email,
            priority.unwrap_or(NotificationPriority::Normal),
        ).map_err(|e| NotificationPortError::ValidationError(e.to_string()))?;
        
        // Send through delivery port
        self.delivery_port.deliver_notification(notification).await
    }
    
    /// Create email template
    pub async fn create_email_template(
        &self,
        template: EmailTemplate,
    ) -> NotificationPortResult<String> {
        let template_port = self.template_port.as_ref()
            .ok_or_else(|| NotificationPortError::ConfigurationError(
                "No template port configured".to_string()
            ))?;
        
        let notification_template = NotificationTemplate::new(
            template.id,
            template.name,
            NotificationChannel::Email,
            template.body_template,
            template.variables,
        );
        
        template_port.create_template(notification_template).await
    }
    
    /// Validate email address
    pub fn validate_email(&self, email: &str) -> bool {
        // Basic email validation
        email.contains('@') && email.len() > 3 && email.contains('.')
    }
    
    /// Check service health
    pub async fn health_check(&self) -> bool {
        self.delivery_port.health_check().await
    }
    
    /// Get service capabilities
    pub fn get_capabilities(&self) -> EmailServiceCapabilities {
        let delivery_caps = self.delivery_port.capabilities();
        
        EmailServiceCapabilities {
            supports_templates: self.template_port.is_some(),
            supports_attachments: delivery_caps.supports_attachments,
            supports_bulk: delivery_caps.supports_bulk,
            supports_rich_content: delivery_caps.supports_rich_content,
            max_attachment_size: delivery_caps.max_attachment_size,
            rate_limit: delivery_caps.rate_limit,
        }
    }
}

/// Email request for bulk operations
#[derive(Debug, Clone)]
pub struct EmailRequest {
    pub to: String,
    pub subject: Option<String>,
    pub body: Option<String>,
    pub template_id: Option<String>,
    pub template_variables: Option<HashMap<String, serde_json::Value>>,
    pub priority: Option<NotificationPriority>,
}

/// Email attachment
#[derive(Debug, Clone)]
pub struct EmailAttachment {
    pub filename: String,
    pub content_type: String,
    pub data: Vec<u8>,
}

/// Email template definition
#[derive(Debug, Clone)]
pub struct EmailTemplate {
    pub id: String,
    pub name: String,
    pub subject_template: Option<String>,
    pub body_template: String,
    pub variables: Vec<String>,
}

/// Email service capabilities
#[derive(Debug, Clone)]
pub struct EmailServiceCapabilities {
    pub supports_templates: bool,
    pub supports_attachments: bool,
    pub supports_bulk: bool,
    pub supports_rich_content: bool,
    pub max_attachment_size: Option<usize>,
    pub rate_limit: Option<u32>,
}

/// Factory for creating email notification services
pub struct EmailNotificationServiceFactory;

impl EmailNotificationServiceFactory {
    /// Create email notification service with delivery port
    pub fn create_with_delivery_port(
        delivery_port: Arc<dyn NotificationDeliveryPort>,
    ) -> EmailNotificationService {
        EmailNotificationService::new(delivery_port, None)
    }
    
    /// Create email notification service with delivery and template ports
    pub fn create_with_ports(
        delivery_port: Arc<dyn NotificationDeliveryPort>,
        template_port: Arc<dyn NotificationTemplatePort>,
    ) -> EmailNotificationService {
        EmailNotificationService::new(delivery_port, Some(template_port))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock delivery port for testing
    struct MockEmailDeliveryPort;
    
    #[async_trait]
    impl NotificationDeliveryPort for MockEmailDeliveryPort {
        fn channel(&self) -> NotificationChannel {
            NotificationChannel::Email
        }
        
        fn can_handle(&self, notification: &Notification) -> bool {
            notification.channel == NotificationChannel::Email
        }
        
        async fn deliver_notification(&self, notification: Notification) -> NotificationPortResult<NotificationDeliveryResult> {
            Ok(NotificationDeliveryResult {
                notification_id: notification.id,
                status: NotificationStatus::Sent,
                external_id: Some("mock-id".to_string()),
                processing_time_ms: 100,
                error_message: None,
                delivered_at: Utc::now(),
                channel: NotificationChannel::Email,
                metadata: HashMap::new(),
            })
        }
        
        async fn health_check(&self) -> bool {
            true
        }
        
        fn capabilities(&self) -> DeliveryCapabilities {
            DeliveryCapabilities {
                supports_bulk: true,
                supports_receipts: true,
                supports_attachments: true,
                supports_rich_content: true,
                supports_templates: false,
                max_attachment_size: Some(25 * 1024 * 1024),
                rate_limit: Some(100),
            }
        }
    }

    #[tokio::test]
    async fn test_send_simple_email() {
        let delivery_port = Arc::new(MockEmailDeliveryPort);
        let service = EmailNotificationService::new(delivery_port, None);
        
        let result = service.send_simple_email(
            "test@example.com".to_string(),
            "Test Subject".to_string(),
            "Test Body".to_string(),
            Some(NotificationPriority::High),
        ).await;
        
        assert!(result.is_ok());
        let delivery_result = result.unwrap();
        assert_eq!(delivery_result.status, NotificationStatus::Sent);
    }

    #[test]
    fn test_email_validation() {
        let delivery_port = Arc::new(MockEmailDeliveryPort);
        let service = EmailNotificationService::new(delivery_port, None);
        
        assert!(service.validate_email("test@example.com"));
        assert!(!service.validate_email("invalid-email"));
        assert!(!service.validate_email("@example.com"));
    }

    #[tokio::test]
    async fn test_health_check() {
        let delivery_port = Arc::new(MockEmailDeliveryPort);
        let service = EmailNotificationService::new(delivery_port, None);
        
        assert!(service.health_check().await);
    }

    #[test]
    fn test_capabilities() {
        let delivery_port = Arc::new(MockEmailDeliveryPort);
        let service = EmailNotificationService::new(delivery_port, None);
        
        let capabilities = service.get_capabilities();
        assert!(!capabilities.supports_templates); // No template port
        assert!(capabilities.supports_attachments);
        assert!(capabilities.supports_bulk);
    }
}