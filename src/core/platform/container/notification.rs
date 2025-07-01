/*
Notification Container Module

Rich domain model for the notification system following Domain-Driven Design principles.
This module contains the core domain entities, value objects, and domain events for notifications.

The notification system is built as an extension of the Message system but provides specialized
domain concepts for notification management, delivery tracking, and content templating.

Domain Concepts:
- Notification: Core aggregate root representing a notification entity
- NotificationContent: Value object for rich content with templates
- NotificationRecipient: Value object for recipient targeting
- NotificationChannel: Enum for delivery channels
- NotificationPriority: Priority levels for delivery
- NotificationStatus: Delivery status tracking
- NotificationTemplate: Template management for content generation
- NotificationEvent: Domain events for notification lifecycle

The notification domain follows hexagonal architecture principles with clear boundaries
between domain logic and external concerns.
*/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::fmt;
use thiserror::Error;

use crate::core::base::entity::message::{Message, Location, MessagePriority};

/// Result type for notification domain operations
pub type NotificationResult<T> = Result<T, NotificationDomainError>;

/// Domain errors for notification operations
#[derive(Debug, Clone, Error, PartialEq)]
pub enum NotificationDomainError {
    #[error("Invalid recipient: {0}")]
    InvalidRecipient(String),
    
    #[error("Invalid content: {0}")]
    InvalidContent(String),
    
    #[error("Template not found: {0}")]
    TemplateNotFound(String),
    
    #[error("Template rendering failed: {0}")]
    TemplateRenderingFailed(String),
    
    #[error("Notification expired")]
    NotificationExpired,
    
    #[error("Invalid notification state transition from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },
    
    #[error("Notification already processed")]
    NotificationAlreadyProcessed,
    
    #[error("Validation error: {0}")]
    ValidationError(String),
}

/// Notification priority levels for delivery ordering
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum NotificationPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
    Emergency = 4,
}

impl Default for NotificationPriority {
    fn default() -> Self {
        Self::Normal
    }
}

impl From<NotificationPriority> for MessagePriority {
    fn from(priority: NotificationPriority) -> Self {
        match priority {
            NotificationPriority::Low => MessagePriority::Low,
            NotificationPriority::Normal => MessagePriority::Normal,
            NotificationPriority::High => MessagePriority::High,
            NotificationPriority::Critical | NotificationPriority::Emergency => MessagePriority::Critical,
        }
    }
}

/// Delivery channels for notifications
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NotificationChannel {
    Email,
    Sms,
    Push,
    WebPush,
    Webhook,
    InApp,
    System,
    Slack,
    Discord,
    Teams,
    Custom(String),
}

impl fmt::Display for NotificationChannel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotificationChannel::Email => write!(f, "email"),
            NotificationChannel::Sms => write!(f, "sms"),
            NotificationChannel::Push => write!(f, "push"),
            NotificationChannel::WebPush => write!(f, "webpush"),
            NotificationChannel::Webhook => write!(f, "webhook"),
            NotificationChannel::InApp => write!(f, "inapp"),
            NotificationChannel::System => write!(f, "system"),
            NotificationChannel::Slack => write!(f, "slack"),
            NotificationChannel::Discord => write!(f, "discord"),
            NotificationChannel::Teams => write!(f, "teams"),
            NotificationChannel::Custom(name) => write!(f, "custom:{}", name),
        }
    }
}

/// Notification delivery status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationStatus {
    Draft,
    Pending,
    Queued,
    Sending,
    Sent,
    Delivered,
    Read,
    Failed,
    Cancelled,
    Expired,
    Retry(u32),
}

impl NotificationStatus {
    /// Check if the notification is in a final state
    pub fn is_final(&self) -> bool {
        matches!(self, 
            NotificationStatus::Delivered |
            NotificationStatus::Read |
            NotificationStatus::Failed |
            NotificationStatus::Cancelled |
            NotificationStatus::Expired
        )
    }
    
    /// Check if the notification can be retried
    pub fn can_retry(&self) -> bool {
        matches!(self, NotificationStatus::Failed | NotificationStatus::Retry(_))
    }
    
    /// Get the next retry status
    pub fn next_retry(&self, max_retries: u32) -> Option<NotificationStatus> {
        match self {
            NotificationStatus::Failed => Some(NotificationStatus::Retry(1)),
            NotificationStatus::Retry(count) if *count < max_retries => {
                Some(NotificationStatus::Retry(count + 1))
            }
            _ => None,
        }
    }
}

/// Recipient information for notifications
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NotificationRecipient {
    Email(String),
    Phone(String),
    UserId(String),
    DeviceToken(String),
    WebhookUrl(String),
    SystemComponent(String),
    Multiple(Vec<NotificationRecipient>),
}

impl NotificationRecipient {
    /// Validate the recipient format
    pub fn validate(&self) -> NotificationResult<()> {
        match self {
            NotificationRecipient::Email(email) => {
                if email.contains('@') && email.len() > 3 {
                    Ok(())
                } else {
                    Err(NotificationDomainError::InvalidRecipient(
                        format!("Invalid email format: {}", email)
                    ))
                }
            }
            NotificationRecipient::Phone(phone) => {
                if phone.len() >= 10 {
                    Ok(())
                } else {
                    Err(NotificationDomainError::InvalidRecipient(
                        format!("Invalid phone format: {}", phone)
                    ))
                }
            }
            NotificationRecipient::UserId(id) => {
                if !id.is_empty() {
                    Ok(())
                } else {
                    Err(NotificationDomainError::InvalidRecipient(
                        "User ID cannot be empty".to_string()
                    ))
                }
            }
            NotificationRecipient::Multiple(recipients) => {
                for recipient in recipients {
                    recipient.validate()?;
                }
                Ok(())
            }
            _ => Ok(()), // Other types are assumed valid for now
        }
    }
    
    /// Get the primary identifier for the recipient
    pub fn identifier(&self) -> String {
        match self {
            NotificationRecipient::Email(email) => email.clone(),
            NotificationRecipient::Phone(phone) => phone.clone(),
            NotificationRecipient::UserId(id) => id.clone(),
            NotificationRecipient::DeviceToken(token) => token.clone(),
            NotificationRecipient::WebhookUrl(url) => url.clone(),
            NotificationRecipient::SystemComponent(component) => component.clone(),
            NotificationRecipient::Multiple(recipients) => {
                format!("multiple:{}", recipients.len())
            }
        }
    }
    
    /// Get the compatible channels for this recipient type
    pub fn compatible_channels(&self) -> Vec<NotificationChannel> {
        match self {
            NotificationRecipient::Email(_) => vec![NotificationChannel::Email],
            NotificationRecipient::Phone(_) => vec![NotificationChannel::Sms],
            NotificationRecipient::UserId(_) => vec![
                NotificationChannel::Email,
                NotificationChannel::Push,
                NotificationChannel::InApp,
            ],
            NotificationRecipient::DeviceToken(_) => vec![
                NotificationChannel::Push,
                NotificationChannel::WebPush,
            ],
            NotificationRecipient::WebhookUrl(_) => vec![NotificationChannel::Webhook],
            NotificationRecipient::SystemComponent(_) => vec![NotificationChannel::System],
            NotificationRecipient::Multiple(_) => vec![], // Determined by individual recipients
        }
    }
}

/// Rich content for notifications with template support
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotificationContent {
    pub title: String,
    pub body: String,
    pub category: String,
    pub action_url: Option<String>,
    pub attachments: Vec<NotificationAttachment>,
    pub template_id: Option<String>,
    pub template_variables: HashMap<String, serde_json::Value>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl NotificationContent {
    /// Create new notification content
    pub fn new(title: String, body: String, category: String) -> Self {
        Self {
            title,
            body,
            category,
            action_url: None,
            attachments: Vec::new(),
            template_id: None,
            template_variables: HashMap::new(),
            metadata: HashMap::new(),
        }
    }
    
    /// Create content with template
    pub fn with_template(
        template_id: String,
        variables: HashMap<String, serde_json::Value>,
        category: String,
    ) -> Self {
        Self {
            title: String::new(), // Will be filled by template
            body: String::new(),  // Will be filled by template
            category,
            action_url: None,
            attachments: Vec::new(),
            template_id: Some(template_id),
            template_variables: variables,
            metadata: HashMap::new(),
        }
    }
    
    /// Validate the content
    pub fn validate(&self) -> NotificationResult<()> {
        if self.title.is_empty() && self.template_id.is_none() {
            return Err(NotificationDomainError::InvalidContent(
                "Title cannot be empty when no template is specified".to_string()
            ));
        }
        
        if self.body.is_empty() && self.template_id.is_none() {
            return Err(NotificationDomainError::InvalidContent(
                "Body cannot be empty when no template is specified".to_string()
            ));
        }
        
        if self.category.is_empty() {
            return Err(NotificationDomainError::InvalidContent(
                "Category cannot be empty".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Check if this content uses a template
    pub fn uses_template(&self) -> bool {
        self.template_id.is_some()
    }
    
    /// Add attachment
    pub fn add_attachment(&mut self, attachment: NotificationAttachment) {
        self.attachments.push(attachment);
    }
    
    /// Add template variable
    pub fn add_variable<T: serde::Serialize>(&mut self, key: String, value: T) -> NotificationResult<()> {
        let json_value = serde_json::to_value(value)
            .map_err(|e| NotificationDomainError::ValidationError(e.to_string()))?;
        self.template_variables.insert(key, json_value);
        Ok(())
    }
}

/// Attachment for notifications
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotificationAttachment {
    pub filename: String,
    pub content_type: String,
    pub data: Vec<u8>,
    pub size: usize,
}

impl NotificationAttachment {
    /// Create new attachment
    pub fn new(filename: String, content_type: String, data: Vec<u8>) -> Self {
        let size = data.len();
        Self {
            filename,
            content_type,
            data,
            size,
        }
    }
    
    /// Validate attachment
    pub fn validate(&self, max_size: usize) -> NotificationResult<()> {
        if self.filename.is_empty() {
            return Err(NotificationDomainError::InvalidContent(
                "Attachment filename cannot be empty".to_string()
            ));
        }
        
        if self.size > max_size {
            return Err(NotificationDomainError::InvalidContent(
                format!("Attachment size {} exceeds maximum {}", self.size, max_size)
            ));
        }
        
        Ok(())
    }
}

/// Template for notification content generation
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
    pub version: u32,
    pub is_active: bool,
}

impl NotificationTemplate {
    /// Create new template
    pub fn new(
        id: String,
        name: String,
        channel: NotificationChannel,
        body_template: String,
        variables: Vec<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            name,
            channel,
            subject_template: None,
            body_template,
            variables,
            created_at: now,
            updated_at: now,
            version: 1,
            is_active: true,
        }
    }
    
    /// Validate template
    pub fn validate(&self) -> NotificationResult<()> {
        if self.id.is_empty() {
            return Err(NotificationDomainError::ValidationError(
                "Template ID cannot be empty".to_string()
            ));
        }
        
        if self.name.is_empty() {
            return Err(NotificationDomainError::ValidationError(
                "Template name cannot be empty".to_string()
            ));
        }
        
        if self.body_template.is_empty() {
            return Err(NotificationDomainError::ValidationError(
                "Template body cannot be empty".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Update template
    pub fn update(&mut self, body_template: String, variables: Vec<String>) {
        self.body_template = body_template;
        self.variables = variables;
        self.updated_at = Utc::now();
        self.version += 1;
    }
    
    /// Deactivate template
    pub fn deactivate(&mut self) {
        self.is_active = false;
        self.updated_at = Utc::now();
    }
}

/// Core notification aggregate root
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Notification {
    pub id: Uuid,
    pub recipient: NotificationRecipient,
    pub content: NotificationContent,
    pub channel: NotificationChannel,
    pub priority: NotificationPriority,
    pub status: NotificationStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub scheduled_time: Option<DateTime<Utc>>,
    pub expiry_time: Option<DateTime<Utc>>,
    pub sent_at: Option<DateTime<Utc>>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub read_at: Option<DateTime<Utc>>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub external_id: Option<String>,
    pub correlation_id: Option<Uuid>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Notification {
    /// Create new notification
    pub fn new(
        recipient: NotificationRecipient,
        content: NotificationContent,
        channel: NotificationChannel,
        priority: NotificationPriority,
    ) -> NotificationResult<Self> {
        // Validate inputs
        recipient.validate()?;
        content.validate()?;
        
        // Validate channel compatibility
        if !recipient.compatible_channels().is_empty() && 
           !recipient.compatible_channels().contains(&channel) {
            return Err(NotificationDomainError::InvalidRecipient(
                format!("Channel {:?} not compatible with recipient type", channel)
            ));
        }
        
        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4(),
            recipient,
            content,
            channel,
            priority,
            status: NotificationStatus::Draft,
            created_at: now,
            updated_at: now,
            scheduled_time: None,
            expiry_time: None,
            sent_at: None,
            delivered_at: None,
            read_at: None,
            retry_count: 0,
            max_retries: 3,
            external_id: None,
            correlation_id: None,
            metadata: HashMap::new(),
        })
    }
    
    /// Schedule notification for future delivery
    pub fn schedule(&mut self, scheduled_time: DateTime<Utc>) -> NotificationResult<()> {
        if scheduled_time <= Utc::now() {
            return Err(NotificationDomainError::ValidationError(
                "Scheduled time must be in the future".to_string()
            ));
        }
        
        self.scheduled_time = Some(scheduled_time);
        self.update_status(NotificationStatus::Pending)?;
        Ok(())
    }
    
    /// Set expiry time
    pub fn set_expiry(&mut self, expiry_time: DateTime<Utc>) -> NotificationResult<()> {
        if expiry_time <= Utc::now() {
            return Err(NotificationDomainError::ValidationError(
                "Expiry time must be in the future".to_string()
            ));
        }
        
        self.expiry_time = Some(expiry_time);
        self.updated_at = Utc::now();
        Ok(())
    }
    
    /// Update notification status
    pub fn update_status(&mut self, new_status: NotificationStatus) -> NotificationResult<()> {
        // Validate state transition
        if !self.can_transition_to(&new_status) {
            return Err(NotificationDomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: format!("{:?}", new_status),
            });
        }
        
        let _old_status = self.status.clone();
        self.status = new_status;
        self.updated_at = Utc::now();
        
        // Update timestamps based on status
        match &self.status {
            NotificationStatus::Sent => {
                self.sent_at = Some(Utc::now());
            }
            NotificationStatus::Delivered => {
                if self.sent_at.is_none() {
                    self.sent_at = Some(Utc::now());
                }
                self.delivered_at = Some(Utc::now());
            }
            NotificationStatus::Read => {
                if self.delivered_at.is_none() {
                    self.delivered_at = Some(Utc::now());
                }
                self.read_at = Some(Utc::now());
            }
            NotificationStatus::Retry(count) => {
                self.retry_count = *count;
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// Check if notification can transition to new status
    fn can_transition_to(&self, new_status: &NotificationStatus) -> bool {
        use NotificationStatus::*;
        
        match (&self.status, new_status) {
            // From Draft
            (Draft, Pending | Queued | Cancelled) => true,
            
            // From Pending
            (Pending, Queued | Cancelled | Expired) => true,
            
            // From Queued
            (Queued, Sending | Cancelled | Expired) => true,
            
            // From Sending
            (Sending, Sent | Failed | Cancelled) => true,
            
            // From Sent
            (Sent, Delivered | Failed) => true,
            
            // From Delivered
            (Delivered, Read) => true,
            
            // From Failed
            (Failed, Retry(_) | Cancelled) => true,
            
            // From Retry
            (Retry(_), Sending | Failed | Cancelled) => true,
            
            // Final states cannot transition
            (Cancelled | Expired | Read, _) => false,
            
            // Same status is allowed
            (status1, status2) if status1 == status2 => true,
            
            _ => false,
        }
    }
    
    /// Check if notification is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expiry) = self.expiry_time {
            Utc::now() > expiry
        } else {
            false
        }
    }
    
    /// Check if notification should be sent now
    pub fn should_send_now(&self) -> bool {
        if self.is_expired() {
            return false;
        }
        
        match self.scheduled_time {
            Some(scheduled) => Utc::now() >= scheduled,
            None => matches!(self.status, NotificationStatus::Queued),
        }
    }
    
    /// Check if notification can be retried
    pub fn can_retry(&self) -> bool {
        self.status.can_retry() && self.retry_count < self.max_retries && !self.is_expired()
    }
    
    /// Convert to message for delivery
    pub fn to_message(&self, source: Location, destination: Location) -> Message<NotificationContent> {
        Message::complete(
            source,
            destination,
            self.content.clone(),
            self.priority.into(),
            self.correlation_id,
        )
    }
    
    /// Add metadata
    pub fn add_metadata<T: serde::Serialize>(&mut self, key: String, value: T) -> NotificationResult<()> {
        let json_value = serde_json::to_value(value)
            .map_err(|e| NotificationDomainError::ValidationError(e.to_string()))?;
        self.metadata.insert(key, json_value);
        self.updated_at = Utc::now();
        Ok(())
    }
}

/// Domain events for notification lifecycle
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NotificationEvent {
    NotificationCreated {
        notification_id: Uuid,
        recipient: NotificationRecipient,
        channel: NotificationChannel,
        priority: NotificationPriority,
        created_at: DateTime<Utc>,
    },
    NotificationScheduled {
        notification_id: Uuid,
        scheduled_time: DateTime<Utc>,
        scheduled_at: DateTime<Utc>,
    },
    NotificationQueued {
        notification_id: Uuid,
        channel: NotificationChannel,
        queued_at: DateTime<Utc>,
    },
    NotificationSent {
        notification_id: Uuid,
        channel: NotificationChannel,
        external_id: Option<String>,
        sent_at: DateTime<Utc>,
    },
    NotificationDelivered {
        notification_id: Uuid,
        delivered_at: DateTime<Utc>,
    },
    NotificationRead {
        notification_id: Uuid,
        read_at: DateTime<Utc>,
    },
    NotificationFailed {
        notification_id: Uuid,
        error: String,
        retry_count: u32,
        failed_at: DateTime<Utc>,
    },
    NotificationCancelled {
        notification_id: Uuid,
        reason: Option<String>,
        cancelled_at: DateTime<Utc>,
    },
    NotificationExpired {
        notification_id: Uuid,
        expired_at: DateTime<Utc>,
    },
}

impl NotificationEvent {
    /// Get the notification ID for this event
    pub fn notification_id(&self) -> Uuid {
        match self {
            NotificationEvent::NotificationCreated { notification_id, .. } |
            NotificationEvent::NotificationScheduled { notification_id, .. } |
            NotificationEvent::NotificationQueued { notification_id, .. } |
            NotificationEvent::NotificationSent { notification_id, .. } |
            NotificationEvent::NotificationDelivered { notification_id, .. } |
            NotificationEvent::NotificationRead { notification_id, .. } |
            NotificationEvent::NotificationFailed { notification_id, .. } |
            NotificationEvent::NotificationCancelled { notification_id, .. } |
            NotificationEvent::NotificationExpired { notification_id, .. } => *notification_id,
        }
    }
    
    /// Get the event timestamp
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            NotificationEvent::NotificationCreated { created_at, .. } => *created_at,
            NotificationEvent::NotificationScheduled { scheduled_at, .. } => *scheduled_at,
            NotificationEvent::NotificationQueued { queued_at, .. } => *queued_at,
            NotificationEvent::NotificationSent { sent_at, .. } => *sent_at,
            NotificationEvent::NotificationDelivered { delivered_at, .. } => *delivered_at,
            NotificationEvent::NotificationRead { read_at, .. } => *read_at,
            NotificationEvent::NotificationFailed { failed_at, .. } => *failed_at,
            NotificationEvent::NotificationCancelled { cancelled_at, .. } => *cancelled_at,
            NotificationEvent::NotificationExpired { expired_at, .. } => *expired_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_creation() {
        let recipient = NotificationRecipient::Email("test@example.com".to_string());
        let content = NotificationContent::new(
            "Test Title".to_string(),
            "Test Body".to_string(),
            "test".to_string(),
        );
        
        let notification = Notification::new(
            recipient,
            content,
            NotificationChannel::Email,
            NotificationPriority::Normal,
        );
        
        assert!(notification.is_ok());
        let notification = notification.unwrap();
        assert_eq!(notification.status, NotificationStatus::Draft);
        assert_eq!(notification.retry_count, 0);
    }

    #[test]
    fn test_notification_status_transitions() {
        let recipient = NotificationRecipient::Email("test@example.com".to_string());
        let content = NotificationContent::new(
            "Test".to_string(),
            "Test".to_string(),
            "test".to_string(),
        );
        
        let mut notification = Notification::new(
            recipient,
            content,
            NotificationChannel::Email,
            NotificationPriority::Normal,
        ).unwrap();
        
        // Valid transition
        assert!(notification.update_status(NotificationStatus::Pending).is_ok());
        assert_eq!(notification.status, NotificationStatus::Pending);
        
        // Invalid transition
        assert!(notification.update_status(NotificationStatus::Read).is_err());
    }

    #[test]
    fn test_recipient_validation() {
        let valid_email = NotificationRecipient::Email("test@example.com".to_string());
        assert!(valid_email.validate().is_ok());
        
        let invalid_email = NotificationRecipient::Email("invalid".to_string());
        assert!(invalid_email.validate().is_err());
    }

    #[test]
    fn test_notification_template() {
        let template = NotificationTemplate::new(
            "test-template".to_string(),
            "Test Template".to_string(),
            NotificationChannel::Email,
            "Hello {{name}}!".to_string(),
            vec!["name".to_string()],
        );
        
        assert!(template.validate().is_ok());
        assert_eq!(template.version, 1);
        assert!(template.is_active);
    }

    #[test]
    fn test_notification_expiry() {
        let recipient = NotificationRecipient::Email("test@example.com".to_string());
        let content = NotificationContent::new(
            "Test".to_string(),
            "Test".to_string(),
            "test".to_string(),
        );
        
        let mut notification = Notification::new(
            recipient,
            content,
            NotificationChannel::Email,
            NotificationPriority::Normal,
        ).unwrap();
        
        // Set expiry in the past
        let past_time = Utc::now() - chrono::Duration::hours(1);
        notification.expiry_time = Some(past_time);
        
        assert!(notification.is_expired());
        assert!(!notification.should_send_now());
    }
}
