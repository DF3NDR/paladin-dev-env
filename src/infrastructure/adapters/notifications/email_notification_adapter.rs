/*
Email Notification Adapter

This adapter implements the notification delivery and template ports for email notifications.
It follows Hexagonal Architecture principles by implementing clean port interfaces
without being coupled to specific application logic.

Features:
- SMTP email delivery with TLS support
- HTML and plain text content
- File attachments
- Template rendering with Handlebars
- Delivery tracking and health checks
- Bulk delivery support

This implementation uses lettre for SMTP email sending and Handlebars for template rendering.
*/

use crate::application::ports::output::notification_port::{
    NotificationDeliveryPort, NotificationTemplatePort, BasicNotificationPort,
    NotificationPortResult, NotificationPortError, NotificationDeliveryResult,
    DeliveryCapabilities,
    NotificationChannel, NotificationTemplate, NotificationContent,
    Notification, NotificationStatus, NotificationRecipient,
};
use async_trait::async_trait;
use lettre::{
    Message, SmtpTransport, Transport,
    message::{header::ContentType, Attachment, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
};
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use chrono::{DateTime, Utc};

/// Email delivery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAdapterConfig {
    /// SMTP server hostname
    pub smtp_host: String,
    /// SMTP server port
    pub smtp_port: u16,
    /// SMTP username
    pub username: String,
    /// SMTP password
    pub password: String,
    /// From email address
    pub from_address: String,
    /// From name (optional)
    pub from_name: Option<String>,
    /// Use TLS encryption
    pub use_tls: bool,
    /// Connection timeout in seconds
    pub timeout_seconds: Option<u64>,
    /// Maximum attachment size in bytes
    pub max_attachment_size: usize,
    /// Rate limit (emails per minute)
    pub rate_limit: Option<u32>,
}

impl Default for EmailAdapterConfig {
    fn default() -> Self {
        Self {
            smtp_host: "localhost".to_string(),
            smtp_port: 587,
            username: String::new(),
            password: String::new(),
            from_address: "noreply@example.com".to_string(),
            from_name: Some("System".to_string()),
            use_tls: true,
            timeout_seconds: Some(30),
            max_attachment_size: 25 * 1024 * 1024, // 25MB
            rate_limit: Some(100), // 100 emails per minute
        }
    }
}

/// Email notification adapter
#[derive(Debug)]
pub struct EmailNotificationAdapter {
    config: EmailAdapterConfig,
    smtp_transport: SmtpTransport,
    template_engine: Arc<RwLock<Handlebars<'static>>>,
    templates: Arc<RwLock<HashMap<String, NotificationTemplate>>>,
    delivery_stats: Arc<RwLock<DeliveryStats>>,
}

/// Internal delivery statistics
#[derive(Debug, Default)]
struct DeliveryStats {
    total_sent: u64,
    total_failed: u64,
    last_success: Option<DateTime<Utc>>,
    last_failure: Option<DateTime<Utc>>,
}

impl EmailNotificationAdapter {
    /// Create a new email notification adapter
    pub fn new(config: EmailAdapterConfig) -> NotificationPortResult<Self> {
        let credentials = Credentials::new(config.username.clone(), config.password.clone());
        
        let smtp_transport = if config.use_tls {
            SmtpTransport::relay(&config.smtp_host)
                .map_err(|e| NotificationPortError::ConfigurationError(
                    format!("SMTP relay configuration error: {}", e)
                ))?
                .port(config.smtp_port)
                .credentials(credentials)
                .build()
        } else {
            SmtpTransport::builder_dangerous(&config.smtp_host)
                .port(config.smtp_port)
                .credentials(credentials)
                .build()
        };

        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(true);

        Ok(Self {
            config,
            smtp_transport,
            template_engine: Arc::new(RwLock::new(handlebars)),
            templates: Arc::new(RwLock::new(HashMap::new())),
            delivery_stats: Arc::new(RwLock::new(DeliveryStats::default())),
        })
    }

    /// Extract email address from notification recipient
    fn extract_email_address(&self, recipient: &NotificationRecipient) -> NotificationPortResult<String> {
        match recipient {
            NotificationRecipient::Email(email) => Ok(email.clone()),
            _ => Err(NotificationPortError::ValidationError(
                "Email adapter only supports email recipients".to_string()
            )),
        }
    }

    /// Build email message from notification
    fn build_email_message(
        &self,
        notification: &Notification,
    ) -> NotificationPortResult<Message> {
        let to_email = self.extract_email_address(&notification.recipient)?;
        
        let from_address = if let Some(ref name) = self.config.from_name {
            format!("{} <{}>", name, self.config.from_address)
        } else {
            self.config.from_address.clone()
        };

        let message_builder = Message::builder()
            .from(from_address.parse().map_err(|e| {
                NotificationPortError::ConfigurationError(format!("Invalid from address: {}", e))
            })?)
            .to(to_email.parse().map_err(|e| {
                NotificationPortError::ValidationError(format!("Invalid email address: {}", e))
            })?)
            .subject(&notification.content.title);

        // Build message with or without attachments
        let message = if !notification.content.attachments.is_empty() {
            self.build_multipart_message(message_builder, &notification.content, &notification.content.attachments)?
        } else {
            // Simple HTML message
            message_builder
                .header(ContentType::TEXT_HTML)
                .body(notification.content.body.clone())
                .map_err(|e| NotificationPortError::DeliveryFailed(
                    format!("Failed to build email body: {}", e)
                ))?
        };

        Ok(message)
    }

    /// Build multipart message with attachments
    fn build_multipart_message(
        &self,
        message_builder: lettre::message::MessageBuilder,
        content: &NotificationContent,
        attachments: &[crate::core::platform::container::notification::NotificationAttachment],
    ) -> NotificationPortResult<Message> {
        let mut multipart = MultiPart::mixed().singlepart(
            SinglePart::builder()
                .header(ContentType::TEXT_HTML)
                .body(content.body.clone())
        );

        for attachment in attachments {
            // Check attachment size
            if attachment.data.len() > self.config.max_attachment_size {
                return Err(NotificationPortError::ValidationError(
                    format!("Attachment '{}' exceeds maximum size", attachment.filename)
                ));
            }

            let content_type = attachment.content_type.parse()
                .unwrap_or(ContentType::TEXT_PLAIN);
                
            let attachment_part = Attachment::new(attachment.filename.clone())
                .body(attachment.data.clone(), content_type);
            multipart = multipart.singlepart(attachment_part);
        }

        message_builder.multipart(multipart)
            .map_err(|e| NotificationPortError::DeliveryFailed(
                format!("Failed to build multipart message: {}", e)
            ))
    }

    /// Send email via SMTP
    async fn send_email(&self, message: Message) -> NotificationPortResult<String> {
        let result = tokio::task::spawn_blocking({
            let transport = self.smtp_transport.clone();
            move || transport.send(&message)
        }).await;

        match result {
            Ok(Ok(response)) => {
                // Update success stats
                if let Ok(mut stats) = self.delivery_stats.write() {
                    stats.total_sent += 1;
                    stats.last_success = Some(Utc::now());
                }
                
                // Extract message ID if available  
                let message_id = response.first_word().unwrap_or("unknown").to_string();
                Ok(message_id)
            }
            Ok(Err(e)) => {
                // Update failure stats
                if let Ok(mut stats) = self.delivery_stats.write() {
                    stats.total_failed += 1;
                    stats.last_failure = Some(Utc::now());
                }
                
                Err(NotificationPortError::DeliveryFailed(
                    format!("SMTP delivery failed: {}", e)
                ))
            }
            Err(e) => Err(NotificationPortError::DeliveryFailed(
                format!("Task execution failed: {}", e)
            )),
        }
    }

    /// Render template content
    async fn render_template_content(
        &self,
        template_id: &str,
        variables: &HashMap<String, serde_json::Value>,
    ) -> NotificationPortResult<NotificationContent> {
        let template = {
            let templates = self.templates.read().map_err(|_| {
                NotificationPortError::ServiceUnavailable("Template storage unavailable".to_string())
            })?;
            templates.get(template_id).cloned().ok_or_else(|| {
                NotificationPortError::TemplateError(format!("Template '{}' not found", template_id))
            })?
        };

        let engine = self.template_engine.read().map_err(|_| {
            NotificationPortError::ServiceUnavailable("Template engine unavailable".to_string())
        })?;

        // Render subject
        let subject = if let Some(ref subject_template) = template.subject_template {
            engine.render_template(subject_template, variables)
                .map_err(|e| NotificationPortError::TemplateError(
                    format!("Subject template rendering failed: {}", e)
                ))?
        } else {
            template.name.clone()
        };

        // Render body
        let body = engine.render_template(&template.body_template, variables)
            .map_err(|e| NotificationPortError::TemplateError(
                format!("Body template rendering failed: {}", e)
            ))?;

        let mut metadata = HashMap::new();
        metadata.insert("template_id".to_string(), serde_json::Value::String(template_id.to_string()));
        metadata.insert("rendered_at".to_string(), serde_json::Value::String(Utc::now().to_rfc3339()));

        Ok(NotificationContent {
            title: subject,
            body,
            category: "email".to_string(),
            action_url: None,
            attachments: Vec::new(),
            template_id: Some(template_id.to_string()),
            template_variables: variables.clone(),
            metadata,
        })
    }
}

// ============================================================================
// PORT IMPLEMENTATIONS
// ============================================================================

#[async_trait]
impl NotificationDeliveryPort for EmailNotificationAdapter {
    fn channel(&self) -> NotificationChannel {
        NotificationChannel::Email
    }

    fn can_handle(&self, notification: &Notification) -> bool {
        notification.channel == NotificationChannel::Email &&
        matches!(notification.recipient, NotificationRecipient::Email(_))
    }

    async fn deliver_notification(&self, notification: Notification) -> NotificationPortResult<NotificationDeliveryResult> {
        let start_time = Instant::now();
        
        // Validate notification
        if !self.can_handle(&notification) {
            return Err(NotificationPortError::ValidationError(
                "Email adapter cannot handle this notification".to_string()
            ));
        }

        // Build email message
        let message = self.build_email_message(&notification)?;
        
        // Send email
        let external_id = self.send_email(message).await?;
        
        let processing_time = start_time.elapsed().as_millis() as u64;
        
        let mut metadata = HashMap::new();
        metadata.insert("smtp_message_id".to_string(), serde_json::Value::String(external_id.clone()));
        metadata.insert("from_address".to_string(), serde_json::Value::String(self.config.from_address.clone()));
        
        Ok(NotificationDeliveryResult {
            notification_id: notification.id,
            status: NotificationStatus::Sent, // Email is considered sent when SMTP accepts it
            external_id: Some(external_id),
            processing_time_ms: processing_time,
            error_message: None,
            delivered_at: Utc::now(),
            channel: NotificationChannel::Email,
            metadata,
        })
    }

    async fn health_check(&self) -> bool {
        // Simple health check by trying to connect to SMTP server
        tokio::task::spawn_blocking({
            let transport = self.smtp_transport.clone();
            move || {
                // This is a simple test - in production you might want a more sophisticated health check
                transport.test_connection().unwrap_or(false)
            }
        }).await.unwrap_or(false)
    }

    fn capabilities(&self) -> DeliveryCapabilities {
        DeliveryCapabilities {
            supports_bulk: true,
            supports_receipts: false, // Email doesn't provide delivery receipts by default
            supports_attachments: true,
            supports_rich_content: true,
            supports_templates: true,
            max_attachment_size: Some(self.config.max_attachment_size),
            rate_limit: self.config.rate_limit,
        }
    }
}

#[async_trait]
impl NotificationTemplatePort for EmailNotificationAdapter {
    async fn create_template(&self, template: NotificationTemplate) -> NotificationPortResult<String> {
        if template.channel != NotificationChannel::Email {
            return Err(NotificationPortError::ValidationError(
                "Email adapter only supports email templates".to_string()
            ));
        }

        // Register body template with handlebars
        {
            let mut engine = self.template_engine.write().map_err(|_| {
                NotificationPortError::ServiceUnavailable("Template engine unavailable".to_string())
            })?;
            
            engine.register_template_string(&template.id, &template.body_template)
                .map_err(|e| NotificationPortError::TemplateError(
                    format!("Failed to register body template: {}", e)
                ))?;

            // Register subject template if provided
            if let Some(ref subject_template) = template.subject_template {
                let subject_template_id = format!("{}_subject", template.id);
                engine.register_template_string(&subject_template_id, subject_template)
                    .map_err(|e| NotificationPortError::TemplateError(
                        format!("Failed to register subject template: {}", e)
                    ))?;
            }
        }

        // Store template
        {
            let mut templates = self.templates.write().map_err(|_| {
                NotificationPortError::ServiceUnavailable("Template storage unavailable".to_string())
            })?;
            templates.insert(template.id.clone(), template.clone());
        }

        Ok(template.id)
    }

    async fn update_template(&self, template: NotificationTemplate) -> NotificationPortResult<()> {
        // Just call create_template as it will overwrite existing templates
        self.create_template(template).await?;
        Ok(())
    }

    async fn delete_template(&self, template_id: &str) -> NotificationPortResult<()> {
        // Remove from handlebars
        {
            let mut engine = self.template_engine.write().map_err(|_| {
                NotificationPortError::ServiceUnavailable("Template engine unavailable".to_string())
            })?;
            
            engine.unregister_template(template_id);
            engine.unregister_template(&format!("{}_subject", template_id));
        }
        
        // Remove from storage
        {
            let mut templates = self.templates.write().map_err(|_| {
                NotificationPortError::ServiceUnavailable("Template storage unavailable".to_string())
            })?;
            templates.remove(template_id);
        }

        Ok(())
    }

    async fn get_template(&self, template_id: &str) -> NotificationPortResult<NotificationTemplate> {
        let templates = self.templates.read().map_err(|_| {
            NotificationPortError::ServiceUnavailable("Template storage unavailable".to_string())
        })?;
        
        templates.get(template_id).cloned().ok_or_else(|| {
            NotificationPortError::TemplateError(format!("Template '{}' not found", template_id))
        })
    }

    async fn list_templates(&self, channel: Option<NotificationChannel>) -> NotificationPortResult<Vec<NotificationTemplate>> {
        let templates = self.templates.read().map_err(|_| {
            NotificationPortError::ServiceUnavailable("Template storage unavailable".to_string())
        })?;
        
        let filtered_templates: Vec<NotificationTemplate> = templates.values()
            .filter(|t| channel.is_none() || Some(t.channel.clone()) == channel)
            .cloned()
            .collect();
            
        Ok(filtered_templates)
    }

    async fn render_template(
        &self,
        template_id: &str,
        variables: HashMap<String, serde_json::Value>,
    ) -> NotificationPortResult<NotificationContent> {
        self.render_template_content(template_id, &variables).await
    }

    async fn validate_template(&self, template: &NotificationTemplate) -> NotificationPortResult<()> {
        let _engine = self.template_engine.read().map_err(|_| {
            NotificationPortError::ServiceUnavailable("Template engine unavailable".to_string())
        })?;

        // Test compile body template by trying to register it temporarily
        {
            let mut temp_engine = Handlebars::new();
            temp_engine.register_template_string("test_body", &template.body_template)
                .map_err(|e| NotificationPortError::TemplateError(
                    format!("Body template syntax error: {}", e)
                ))?;

            // Test compile subject template if provided
            if let Some(ref subject_template) = template.subject_template {
                temp_engine.register_template_string("test_subject", subject_template)
                    .map_err(|e| NotificationPortError::TemplateError(
                        format!("Subject template syntax error: {}", e)
                    ))?;
            }
        }

        Ok(())
    }
}

// Implement BasicNotificationPort trait for convenience
#[async_trait]
impl BasicNotificationPort for EmailNotificationAdapter {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::platform::container::notification::{NotificationPriority};

    fn create_test_config() -> EmailAdapterConfig {
        EmailAdapterConfig {
            smtp_host: "smtp.example.com".to_string(),
            smtp_port: 587,
            username: "test@example.com".to_string(),
            password: "password123".to_string(),
            from_address: "noreply@example.com".to_string(),
            from_name: Some("Test Service".to_string()),
            use_tls: true,
            timeout_seconds: Some(30),
            max_attachment_size: 10 * 1024 * 1024, // 10MB
            rate_limit: Some(60),
        }
    }

    fn create_test_notification() -> Notification {
        let content = NotificationContent::new(
            "Test Subject".to_string(),
            "Test body".to_string(),
            "email".to_string(),
        );
        
        Notification::new(
            NotificationRecipient::Email("test@example.com".to_string()),
            content,
            NotificationChannel::Email,
            NotificationPriority::Normal,
        ).unwrap()
    }

    #[test]
    fn test_email_adapter_creation() {
        let config = create_test_config();
        let adapter = EmailNotificationAdapter::new(config);
        assert!(adapter.is_ok());
    }

    #[test]
    fn test_email_adapter_channel() {
        let config = create_test_config();
        let adapter = EmailNotificationAdapter::new(config).unwrap();
        assert_eq!(adapter.channel(), NotificationChannel::Email);
    }

    #[test]
    fn test_can_handle_notification() {
        let config = create_test_config();
        let adapter = EmailNotificationAdapter::new(config).unwrap();
        let notification = create_test_notification();
        
        assert!(adapter.can_handle(&notification));
    }

    #[test]
    fn test_extract_email_address() {
        let config = create_test_config();
        let adapter = EmailNotificationAdapter::new(config).unwrap();
        
        let email_recipient = NotificationRecipient::Email("test@example.com".to_string());
        let result = adapter.extract_email_address(&email_recipient);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test@example.com");

        let phone_recipient = NotificationRecipient::Phone("123456789".to_string());
        let result = adapter.extract_email_address(&phone_recipient);
        assert!(result.is_err());
    }

    #[test]
    fn test_capabilities() {
        let config = create_test_config();
        let adapter = EmailNotificationAdapter::new(config).unwrap();
        let capabilities = adapter.capabilities();
        
        assert!(capabilities.supports_bulk);
        assert!(capabilities.supports_attachments);
        assert!(capabilities.supports_rich_content);
        assert!(capabilities.supports_templates);
        assert!(!capabilities.supports_receipts);
        assert_eq!(capabilities.max_attachment_size, Some(10 * 1024 * 1024));
        assert_eq!(capabilities.rate_limit, Some(60));
    }

    #[tokio::test]
    async fn test_template_lifecycle() {
        let config = create_test_config();
        let adapter = EmailNotificationAdapter::new(config).unwrap();
        
        let template = NotificationTemplate {
            id: "welcome".to_string(),
            name: "Welcome Email".to_string(),
            channel: NotificationChannel::Email,
            subject_template: Some("Welcome {{name}}!".to_string()),
            body_template: "Hello {{name}}, welcome to our service!".to_string(),
            variables: vec!["name".to_string()],
            version: 1,
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Create template
        let result = adapter.create_template(template.clone()).await;
        assert!(result.is_ok());

        // Get template
        let retrieved = adapter.get_template("welcome").await;
        assert!(retrieved.is_ok());
        assert_eq!(retrieved.unwrap().name, "Welcome Email");

        // Validate template
        let validation = adapter.validate_template(&template).await;
        assert!(validation.is_ok());

        // Delete template
        let deletion = adapter.delete_template("welcome").await;
        assert!(deletion.is_ok());

        // Verify deletion
        let retrieved_after_delete = adapter.get_template("welcome").await;
        assert!(retrieved_after_delete.is_err());
    }

    #[tokio::test]
    async fn test_template_rendering() {
        let config = create_test_config();
        let adapter = EmailNotificationAdapter::new(config).unwrap();
        
        let template = NotificationTemplate {
            id: "greeting".to_string(),
            name: "Greeting".to_string(),
            channel: NotificationChannel::Email,
            subject_template: Some("Hello {{name}}".to_string()),
            body_template: "Dear {{name}}, this is a test message.".to_string(),
            variables: vec!["name".to_string()],
            version: 1,
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Create template
        adapter.create_template(template).await.unwrap();

        // Render template
        let mut variables = HashMap::new();
        variables.insert("name".to_string(), serde_json::Value::String("John".to_string()));
        
        let rendered = adapter.render_template("greeting", variables).await;
        assert!(rendered.is_ok());
        
        let content = rendered.unwrap();
        assert_eq!(content.title, "Hello John");
        assert_eq!(content.body, "Dear John, this is a test message.");
    }
}
