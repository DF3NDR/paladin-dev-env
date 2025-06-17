/*
Email Notification Service

This is the adapter that provides an interface for sending email notifications via an external email service.
It implements the NotificationPublisherService trait, which defines the methods for delivering notifications.
It includes methods for sending emails, which can be used to notify users or other systems about events
or updates in the system.
It can be used to send notifications, alerts, or any other type of content that needs to be delivered via email.

This implementation uses lettre for SMTP email sending and includes template rendering capabilities.
*/

use crate::application::ports::output::notification_publisher_port::{
    NotificationPublisherService, NotificationTemplateService, BulkNotificationService,
    NotificationRequest, NotificationResponse, NotificationRecipient, NotificationChannel,
    NotificationContent, NotificationTemplate, NotificationStats, NotificationStatus,
    NotificationError, NotificationPriority
};
use lettre::{
    Message, SmtpTransport, Transport, 
    message::{header::ContentType, Attachment, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
};
use handlebars::Handlebars;
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use chrono::Utc;

#[derive(Debug, Clone)]
pub struct EmailNotificationService {
    smtp_transport: SmtpTransport,
    from_address: String,
    from_name: Option<String>,
    template_engine: Arc<Handlebars<'static>>,
    notification_history: Arc<Mutex<HashMap<Uuid, NotificationResponse>>>,
    templates: Arc<Mutex<HashMap<String, NotificationTemplate>>>,
}

#[derive(Debug, Clone)]
pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub username: String,
    pub password: String,
    pub from_address: String,
    pub from_name: Option<String>,
    pub use_tls: bool,
}

impl EmailNotificationService {
    pub fn new(config: EmailConfig) -> Result<Self, NotificationError> {
        let credentials = Credentials::new(config.username, config.password);
        
        let smtp_transport = if config.use_tls {
            SmtpTransport::relay(&config.smtp_host)
                .map_err(|e| NotificationError::ConfigurationError(format!("SMTP relay error: {}", e)))?
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
            smtp_transport,
            from_address: config.from_address,
            from_name: config.from_name,
            template_engine: Arc::new(handlebars),
            notification_history: Arc::new(Mutex::new(HashMap::new())),
            templates: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    fn extract_email_address(&self, recipient: &NotificationRecipient) -> Result<String, NotificationError> {
        match recipient {
            NotificationRecipient::Email(email) => Ok(email.clone()),
            _ => Err(NotificationError::InvalidRecipient(
                "Email service only supports Email recipients".to_string()
            )),
        }
    }

    fn build_email_message(&self, to: &str, content: &NotificationContent) -> Result<Message, NotificationError> {
        let from_address = if let Some(ref name) = self.from_name {
            format!("{} <{}>", name, self.from_address)
        } else {
            self.from_address.clone()
        };

        let mut message_builder = Message::builder()
            .from(from_address.parse().map_err(|e| {
                NotificationError::ConfigurationError(format!("Invalid from address: {}", e))
            })?)
            .to(to.parse().map_err(|e| {
                NotificationError::InvalidRecipient(format!("Invalid email address: {}", e))
            })?)
            .subject(&content.title);

        let message = if let Some(ref attachments) = content.attachments {
            // Build multipart message with attachments
            let mut multipart = MultiPart::mixed().singlepart(
                SinglePart::builder()
                    .header(ContentType::TEXT_HTML)
                    .body(content.body.clone())
            );

            for attachment in attachments {
                let attachment_part = Attachment::new(attachment.filename.clone())
                    .body(attachment.data.clone(), attachment.content_type.parse().unwrap_or(ContentType::TEXT_PLAIN));
                multipart = multipart.singlepart(attachment_part);
            }

            message_builder.multipart(multipart)
        } else {
            // Simple HTML message
            message_builder.body(content.body.clone())
        };

        message.map_err(|e| NotificationError::SendingFailed(format!("Failed to build email message: {}", e)))
    }

    fn send_email(&self, to: &str, content: &NotificationContent) -> Result<(), NotificationError> {
        let message = self.build_email_message(to, content)?;
        
        self.smtp_transport
            .send(&message)
            .map_err(|e| NotificationError::SendingFailed(format!("SMTP send failed: {}", e)))?;

        Ok(())
    }

    fn render_content_with_template(&self, template_id: &str, variables: &HashMap<String, serde_json::Value>) -> Result<NotificationContent, NotificationError> {
        let templates = self.templates.lock().map_err(|_| NotificationError::ServiceUnavailable)?;
        let template = templates.get(template_id)
            .ok_or_else(|| NotificationError::TemplateNotFound(template_id.to_string()))?;

        let subject = if let Some(ref subject_template) = template.subject_template {
            self.template_engine.render_template(subject_template, variables)
                .map_err(|e| NotificationError::TemplateRenderingFailed(e.to_string()))?
        } else {
            template.name.clone()
        };

        let body = self.template_engine.render_template(&template.body_template, variables)
            .map_err(|e| NotificationError::TemplateRenderingFailed(e.to_string()))?;

        Ok(NotificationContent {
            title: subject,
            body,
            category: "email".to_string(),
            action_url: None,
            attachments: None,
            template_id: Some(template_id.to_string()),
            template_variables: Some(variables.clone()),
        })
    }

    fn create_notification_response(&self, notification_id: Uuid, status: NotificationStatus, error: Option<&NotificationError>) -> NotificationResponse {
        NotificationResponse {
            id: notification_id,
            status,
            sent_at: if matches!(status, NotificationStatus::Sent | NotificationStatus::Delivered) { Some(Utc::now()) } else { None },
            delivered_at: if matches!(status, NotificationStatus::Delivered) { Some(Utc::now()) } else { None },
            read_at: None,
            error_message: error.map(|e| e.to_string()),
            retry_count: 0,
            external_id: None,
            metadata: None,
        }
    }

    fn store_notification_history(&self, response: NotificationResponse) {
        if let Ok(mut history) = self.notification_history.lock() {
            history.insert(response.id, response);
        }
    }
}

impl NotificationPublisherService for EmailNotificationService {
    fn send_notification(&self, request: NotificationRequest) -> Result<NotificationResponse, NotificationError> {
        // Validate channel
        if request.channel != NotificationChannel::Email {
            return Err(NotificationError::InvalidChannel(
                "Email service only supports Email channel".to_string()
            ));
        }

        let notification_id = request.id.unwrap_or_else(Uuid::new_v4);
        let email_address = self.extract_email_address(&request.recipient)?;

        // Render content if template is specified
        let content = if let Some(ref template_id) = request.content.template_id {
            let variables = request.content.template_variables.unwrap_or_default();
            self.render_content_with_template(template_id, &variables)?
        } else {
            request.content
        };

        // Send email
        let result = self.send_email(&email_address, &content);
        
        let (status, error) = match result {
            Ok(()) => (NotificationStatus::Sent, None),
            Err(ref e) => (NotificationStatus::Failed, Some(e)),
        };

        let response = self.create_notification_response(notification_id, status, error);
        self.store_notification_history(response.clone());

        match result {
            Ok(()) => Ok(response),
            Err(e) => Err(e),
        }
    }

    fn schedule_notification(&self, request: NotificationRequest) -> Result<NotificationResponse, NotificationError> {
        let notification_id = request.id.unwrap_or_else(Uuid::new_v4);
        
        // For demo purposes, we'll just mark it as queued
        // In a real implementation, you'd integrate with a job scheduler
        let response = self.create_notification_response(notification_id, NotificationStatus::Queued, None);
        self.store_notification_history(response.clone());
        
        // TODO: Integrate with actual scheduler (e.g., tokio-cron-scheduler)
        Ok(response)
    }

    fn cancel_notification(&self, notification_id: Uuid) -> Result<(), NotificationError> {
        if let Ok(mut history) = self.notification_history.lock() {
            if let Some(mut notification) = history.get(&notification_id).cloned() {
                notification.status = NotificationStatus::Cancelled;
                history.insert(notification_id, notification);
                Ok(())
            } else {
                Err(NotificationError::NotificationNotFound(notification_id.to_string()))
            }
        } else {
            Err(NotificationError::ServiceUnavailable)
        }
    }

    fn get_notification_status(&self, notification_id: Uuid) -> Result<NotificationResponse, NotificationError> {
        if let Ok(history) = self.notification_history.lock() {
            history.get(&notification_id)
                .cloned()
                .ok_or_else(|| NotificationError::NotificationNotFound(notification_id.to_string()))
        } else {
            Err(NotificationError::ServiceUnavailable)
        }
    }

    fn list_notifications(&self, _recipient: &NotificationRecipient, limit: Option<u32>) -> Result<Vec<NotificationResponse>, NotificationError> {
        if let Ok(history) = self.notification_history.lock() {
            let mut notifications: Vec<NotificationResponse> = history.values().cloned().collect();
            
            // Sort by most recent first
            notifications.sort_by(|a, b| {
                b.sent_at.unwrap_or(Utc::now()).cmp(&a.sent_at.unwrap_or(Utc::now()))
            });

            if let Some(limit) = limit {
                notifications.truncate(limit as usize);
            }

            Ok(notifications)
        } else {
            Err(NotificationError::ServiceUnavailable)
        }
    }

    fn get_notification_stats(&self, _channel: Option<NotificationChannel>) -> Result<NotificationStats, NotificationError> {
        if let Ok(history) = self.notification_history.lock() {
            let total_sent = history.values()
                .filter(|n| matches!(n.status, NotificationStatus::Sent | NotificationStatus::Delivered))
                .count() as u64;
            
            let total_delivered = history.values()
                .filter(|n| matches!(n.status, NotificationStatus::Delivered))
                .count() as u64;
            
            let total_failed = history.values()
                .filter(|n| matches!(n.status, NotificationStatus::Failed))
                .count() as u64;
            
            let total_pending = history.values()
                .filter(|n| matches!(n.status, NotificationStatus::Pending | NotificationStatus::Queued))
                .count() as u64;

            let delivery_rate = if total_sent > 0 {
                (total_delivered as f64 / total_sent as f64) * 100.0
            } else {
                0.0
            };

            let mut channel_breakdown = HashMap::new();
            channel_breakdown.insert(NotificationChannel::Email, total_sent);

            Ok(NotificationStats {
                total_sent,
                total_delivered,
                total_failed,
                total_pending,
                delivery_rate,
                average_delivery_time_ms: None,
                channel_breakdown,
            })
        } else {
            Err(NotificationError::ServiceUnavailable)
        }
    }
}

impl NotificationTemplateService for EmailNotificationService {
    fn create_template(&self, template: NotificationTemplate) -> Result<String, NotificationError> {
        if template.channel != NotificationChannel::Email {
            return Err(NotificationError::InvalidChannel(
                "Email service only supports Email templates".to_string()
            ));
        }

        // Register template with handlebars
        self.template_engine.register_template_string(&template.id, &template.body_template)
            .map_err(|e| NotificationError::TemplateRenderingFailed(e.to_string()))?;

        if let Some(ref subject_template) = template.subject_template {
            let subject_template_id = format!("{}_subject", template.id);
            self.template_engine.register_template_string(&subject_template_id, subject_template)
                .map_err(|e| NotificationError::TemplateRenderingFailed(e.to_string()))?;
        }

        // Store template
        if let Ok(mut templates) = self.templates.lock() {
            templates.insert(template.id.clone(), template.clone());
        }

        Ok(template.id)
    }

    fn update_template(&self, template: NotificationTemplate) -> Result<(), NotificationError> {
        self.create_template(template)?;
        Ok(())
    }

    fn delete_template(&self, template_id: &str) -> Result<(), NotificationError> {
        self.template_engine.unregister_template(template_id);
        
        if let Ok(mut templates) = self.templates.lock() {
            templates.remove(template_id);
        }

        Ok(())
    }

    fn get_template(&self, template_id: &str) -> Result<NotificationTemplate, NotificationError> {
        if let Ok(templates) = self.templates.lock() {
            templates.get(template_id)
                .cloned()
                .ok_or_else(|| NotificationError::TemplateNotFound(template_id.to_string()))
        } else {
            Err(NotificationError::ServiceUnavailable)
        }
    }

    fn list_templates(&self, channel: Option<NotificationChannel>) -> Result<Vec<NotificationTemplate>, NotificationError> {
        if let Ok(templates) = self.templates.lock() {
            let filtered_templates: Vec<NotificationTemplate> = templates.values()
                .filter(|t| channel.is_none() || Some(t.channel.clone()) == channel)
                .cloned()
                .collect();
            
            Ok(filtered_templates)
        } else {
            Err(NotificationError::ServiceUnavailable)
        }
    }

    fn render_template(&self, template_id: &str, variables: HashMap<String, serde_json::Value>) -> Result<NotificationContent, NotificationError> {
        self.render_content_with_template(template_id, &variables)
    }
}

impl BulkNotificationService for EmailNotificationService {
    fn send_bulk_notifications(&self, requests: Vec<NotificationRequest>) -> Result<Vec<NotificationResponse>, NotificationError> {
        let mut responses = Vec::new();
        
        for request in requests {
            let response = self.send_notification(request)?;
            responses.push(response);
        }
        
        Ok(responses)
    }

    fn get_bulk_status(&self, _batch_id: Uuid) -> Result<Vec<NotificationResponse>, NotificationError> {
        // In a real implementation, you'd track batch IDs
        Err(NotificationError::SendingFailed("Bulk status tracking not implemented".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> EmailConfig {
        EmailConfig {
            smtp_host: "smtp.gmail.com".to_string(),
            smtp_port: 587,
            username: "test@example.com".to_string(),
            password: "password".to_string(),
            from_address: "noreply@example.com".to_string(),
            from_name: Some("Test Service".to_string()),
            use_tls: true,
        }
    }

    #[test]
    fn test_email_notification_service_creation() {
        let config = create_test_config();
        let service = EmailNotificationService::new(config);
        assert!(service.is_ok());
    }

    #[test]
    fn test_extract_email_address() {
        let config = create_test_config();
        let service = EmailNotificationService::new(config).unwrap();
        
        let email_recipient = NotificationRecipient::Email("test@example.com".to_string());
        let result = service.extract_email_address(&email_recipient);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test@example.com");

        let invalid_recipient = NotificationRecipient::Phone("123456789".to_string());
        let result = service.extract_email_address(&invalid_recipient);
        assert!(result.is_err());
    }

    #[test]
    fn test_template_management() {
        let config = create_test_config();
        let service = EmailNotificationService::new(config).unwrap();
        
        let template = NotificationTemplate {
            id: "welcome".to_string(),
            name: "Welcome Email".to_string(),
            channel: NotificationChannel::Email,
            subject_template: Some("Welcome {{name}}!".to_string()),
            body_template: "Hello {{name}}, welcome to our service!".to_string(),
            variables: vec!["name".to_string()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let result = service.create_template(template.clone());
        assert!(result.is_ok());

        let retrieved = service.get_template("welcome");
        assert!(retrieved.is_ok());
        assert_eq!(retrieved.unwrap().name, "Welcome Email");
    }

    #[test]
    fn test_notification_stats() {
        let config = create_test_config();
        let service = EmailNotificationService::new(config).unwrap();
        
        let stats = service.get_notification_stats(None).unwrap();
        assert_eq!(stats.total_sent, 0);
        assert_eq!(stats.total_delivered, 0);
        assert_eq!(stats.total_failed, 0);
        assert_eq!(stats.total_pending, 0);
    }
}