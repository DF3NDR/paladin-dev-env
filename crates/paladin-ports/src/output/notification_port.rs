#![allow(missing_docs)]
//! Notification Ports - Multi-Channel Notification Delivery Abstraction
//!
//! This module defines the output ports (interfaces) for the notification system following
//! Hexagonal Architecture principles. These ports provide clean abstractions that allow
//! the application layer to interact with external notification delivery mechanisms
//! (email, SMS, push notifications, webhooks, Slack, etc.) without being coupled to
//! their implementation details.
//!
//! # Purpose
//!
//! Notification ports enable Paladin agents to communicate with users and external systems
//! through multiple channels while maintaining a clean separation between the core business
//! logic and the specific notification delivery mechanisms. This allows you to:
//!
//! - Send notifications through multiple channels (email, SMS, push, webhooks, Slack)
//! - Switch between notification providers without changing application code
//! - Test notification logic without sending real messages
//! - Implement retry logic and failure handling consistently
//! - Track delivery status and statistics across all channels
//! - Use templates for consistent message formatting
//!
//! # Hexagonal Architecture (Ports & Adapters)
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │                    Application Layer                          │
//! │  ┌────────────────────────────────────────────────────────┐  │
//! │  │  Paladin Agent Execution                                │  │
//! │  │  - Send alerts on task completion                       │  │
//! │  │  - Notify users of errors                               │  │
//! │  │  - Deliver reports via email                            │  │
//! │  └─────────────────────┬────────────────────────────────────┘  │
//! │                        │                                       │
//! │                        ↓                                       │
//! │  ┌────────────────────────────────────────────────────────┐  │
//! │  │  NotificationDeliveryPort (trait)                       │  │
//! │  │  NotificationTemplatePort (trait)                       │  │
//! │  │  - deliver_notification()                               │  │
//! │  │  - render_template()                                    │  │
//! │  └────────────────────┬───────────────────────────────────┘  │
//! └─────────────────────────┼────────────────────────────────────┘
//!                          │
//!          ┌───────────────┼───────────────┐
//!          │               │               │
//!          ↓               ↓               ↓
//!   ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
//!   │ Email       │ │ SMS         │ │ Slack       │
//!   │ Adapter     │ │ Adapter     │ │ Adapter     │
//!   │ (SMTP)      │ │ (Twilio)    │ │ (Webhook)   │
//!   └─────────────┘ └─────────────┘ └─────────────┘
//! ```
//!
//! # Port Segregation (Interface Segregation Principle)
//!
//! The notification system is split into focused interfaces:
//!
//! - **NotificationDeliveryPort**: Core delivery functionality for sending notifications
//! - **NotificationTemplatePort**: Template management and content rendering
//! - **BasicNotificationPort**: Simplified interface combining delivery with basic operations
//!
//! Each port defines a focused interface that can be implemented independently,
//! allowing for flexible adapter implementations and better testability.
//!
//! # Common Use Cases
//!
//! ## 1. Send Email Notification
//!
//! ```ignore
//! use paladin_ports::output::notification_port::{
//!     NotificationDeliveryPort, NotificationPortError
//! };
//! use paladin_core::platform::container::notification::{
//!     Notification, NotificationChannel, NotificationContent,
//!     NotificationRecipient, NotificationPriority
//! };
//! use std::sync::Arc;
//!
//! async fn send_task_completion_email(
//!     notification_port: Arc<dyn NotificationDeliveryPort>,
//! ) -> Result<(), NotificationPortError> {
//!     let notification = Notification::builder()
//!         .recipient(NotificationRecipient::email("user@example.com"))
//!         .channel(NotificationChannel::Email)
//!         .priority(NotificationPriority::Normal)
//!         .content(NotificationContent::text(
//!             "Task Complete",
//!             "Your Paladin task has finished successfully."
//!         ))
//!         .build()?;
//!
//!     let result = notification_port.deliver_notification(notification).await?;
//!     println!("Email sent: {:?}", result.status);
//!     Ok(())
//! }
//! ```
//!
//! ## 2. Send Bulk Notifications
//!
//! ```ignore
//! use paladin_ports::output::notification_port::{
//!     NotificationDeliveryPort, BulkDeliveryResult
//! };
//! use paladin_core::platform::container::notification::{
//!     Notification, NotificationChannel
//! };
//! use std::sync::Arc;
//!
//! async fn send_bulk_alerts(
//!     notification_port: Arc<dyn NotificationDeliveryPort>,
//!     users: Vec<String>,
//!     message: String,
//! ) -> Result<BulkDeliveryResult, Box<dyn std::error::Error>> {
//!     let notifications: Vec<Notification> = users
//!         .iter()
//!         .map(|email| {
//!             Notification::builder()
//!                 .recipient(NotificationRecipient::email(email))
//!                 .channel(NotificationChannel::Email)
//!                 .content(NotificationContent::text("Alert", &message))
//!                 .build()
//!         })
//!         .collect::<Result<Vec<_>, _>>()?;
//!
//!     let result = notification_port.deliver_bulk(notifications).await?;
//!     println!("Sent {}/{} notifications", result.success_count, result.total_count);
//!     Ok(result)
//! }
//! ```
//!
//! ## 3. Use Template for Consistent Formatting
//!
//! ```ignore
//! use paladin_ports::output::notification_port::{
//!     NotificationTemplatePort, NotificationPortError
//! };
//! use paladin_core::platform::container::notification::{
//!     NotificationTemplate, NotificationChannel
//! };
//! use std::collections::HashMap;
//! use std::sync::Arc;
//!
//! async fn send_templated_notification(
//!     template_port: Arc<dyn NotificationTemplatePort>,
//! ) -> Result<(), NotificationPortError> {
//!     // Render template with variables
//!     let mut variables = HashMap::new();
//!     variables.insert("user_name".to_string(), serde_json::json!("Alice"));
//!     variables.insert("task_name".to_string(), serde_json::json!("Data Analysis"));
//!     variables.insert("duration".to_string(), serde_json::json!("2 hours"));
//!
//!     let content = template_port
//!         .render_template("task_completion_email", variables)
//!         .await?;
//!
//!     println!("Rendered: {} - {}", content.subject, content.body);
//!     Ok(())
//! }
//! ```
//!
//! ## 4. Multi-Channel Notification with Fallback
//!
//! ```ignore
//! use paladin_ports::output::notification_port::{
//!     NotificationDeliveryPort, NotificationPortError
//! };
//! use paladin_core::platform::container::notification::{
//!     Notification, NotificationChannel, NotificationStatus
//! };
//! use std::sync::Arc;
//!
//! async fn send_with_fallback(
//!     primary: Arc<dyn NotificationDeliveryPort>,
//!     fallback: Arc<dyn NotificationDeliveryPort>,
//!     notification: Notification,
//! ) -> Result<(), NotificationPortError> {
//!     match primary.deliver_notification(notification.clone()).await {
//!         Ok(result) if result.status == NotificationStatus::Delivered => {
//!             println!("Delivered via primary channel");
//!             Ok(())
//!         }
//!         _ => {
//!             println!("Primary failed, trying fallback...");
//!             fallback.deliver_notification(notification).await?;
//!             Ok(())
//!         }
//!     }
//! }
//! ```
//!
//! # Channel Support
//!
//! Different adapters support different notification channels:
//!
//! | Channel | Use Case | Typical Adapter |
//! |---------|----------|----------------|
//! | Email | Reports, alerts, receipts | SMTP, SendGrid, AWS SES |
//! | SMS | Urgent alerts, 2FA codes | Twilio, AWS SNS, Nexmo |
//! | Push | Mobile app notifications | FCM, APNs, OneSignal |
//! | Webhook | System-to-system | HTTP client |
//! | Slack | Team collaboration | Slack API, webhooks |
//! | InApp | Application notifications | Database storage |
//!
//! # Error Handling & Retryability
//!
//! NotificationPortError variants indicate whether operations should be retried:
//!
//! | Error | Retryable? | Recovery Strategy |
//! |-------|------------|-------------------|
//! | DeliveryFailed | Maybe | Check error message, implement exponential backoff |
//! | TemplateError | No | Fix template syntax |
//! | StorageError | Yes | Retry with exponential backoff |
//! | ConnectionError | Yes | Retry, check network/DNS |
//! | AuthenticationError | No | Fix credentials in configuration |
//! | RateLimitExceeded | Yes | Wait and retry, implement rate limiting |
//! | ServiceUnavailable | Yes | Exponential backoff, circuit breaker |
//! | ConfigurationError | No | Fix configuration |
//! | ValidationError | No | Fix notification data |
//! | Timeout | Yes | Retry with longer timeout |
//!
//! ## Retry Pattern Example
//!
//! ```ignore
//! use paladin_ports::output::notification_port::{
//!     NotificationDeliveryPort, NotificationPortError
//! };
//! use paladin_core::platform::container::notification::Notification;
//! use std::sync::Arc;
//! use std::time::Duration;
//! use tokio::time::sleep;
//!
//! async fn deliver_with_retry(
//!     port: Arc<dyn NotificationDeliveryPort>,
//!     notification: Notification,
//!     max_retries: u32,
//! ) -> Result<(), NotificationPortError> {
//!     let mut attempts = 0;
//!     let mut backoff = Duration::from_millis(100);
//!
//!     loop {
//!         match port.deliver_notification(notification.clone()).await {
//!             Ok(result) => return Ok(()),
//!             Err(e) if attempts >= max_retries => return Err(e),
//!             Err(NotificationPortError::RateLimitExceeded) => {
//!                 sleep(backoff).await;
//!                 backoff *= 2;
//!             }
//!             Err(NotificationPortError::ServiceUnavailable(_)) => {
//!                 sleep(backoff).await;
//!                 backoff *= 2;
//!             }
//!             Err(e) => return Err(e), // Non-retryable error
//!         }
//!         attempts += 1;
//!     }
//! }
//! ```
//!
//! # Thread Safety
//!
//! All notification ports are `Send + Sync`, allowing safe use across async task boundaries.
//! This is critical for Paladin's concurrent agent execution model where multiple agents
//! may send notifications simultaneously.
//!
//! # Implementation Notes
//!
//! ## Adapter Implementation Checklist
//!
//! When implementing a notification adapter:
//!
//! 1. **Channel Handling**: Implement `can_handle()` to filter notifications by channel
//! 2. **Capabilities**: Return accurate `DeliveryCapabilities` to inform callers of limits
//! 3. **Error Mapping**: Map provider errors to appropriate `NotificationPortError` variants
//! 4. **Idempotency**: Handle duplicate deliveries gracefully (check external_id)
//! 5. **Timeouts**: Set reasonable timeouts for external API calls
//! 6. **Health Checks**: Implement `health_check()` for monitoring/circuit breakers
//! 7. **Metrics**: Track delivery success/failure rates, latency
//! 8. **Logging**: Log delivery attempts, failures, external IDs for tracing
//!
//! ## Performance Considerations
//!
//! - **Connection Pooling**: Reuse HTTP/SMTP connections across deliveries
//! - **Bulk Delivery**: Implement native bulk APIs when supported (SendGrid, Twilio)
//! - **Async I/O**: Use non-blocking I/O for all external calls
//! - **Circuit Breaker**: Stop sending to failing providers temporarily
//! - **Queue Integration**: Use QueuePort for async delivery to prevent blocking agents
//!
//! ## Testing Strategy
//!
//! ```ignore
//! use paladin_ports::output::notification_port::{
//!     NotificationDeliveryPort, NotificationDeliveryResult,
//!     NotificationPortError, DeliveryCapabilities
//! };
//! use paladin_core::platform::container::notification::{
//!     Notification, NotificationChannel, NotificationStatus
//! };
//! use async_trait::async_trait;
//!
//! /// Mock notification port for testing
//! struct MockNotificationPort {
//!     should_fail: bool,
//! }
//!
//! #[async_trait]
//! impl NotificationDeliveryPort for MockNotificationPort {
//!     fn channel(&self) -> NotificationChannel {
//!         NotificationChannel::Email
//!     }
//!
//!     fn can_handle(&self, notification: &Notification) -> bool {
//!         notification.channel == NotificationChannel::Email
//!     }
//!
//!     async fn deliver_notification(
//!         &self,
//!         notification: Notification,
//!     ) -> Result<NotificationDeliveryResult, NotificationPortError> {
//!         if self.should_fail {
//!             Err(NotificationPortError::DeliveryFailed("Mock failure".into()))
//!         } else {
//!             Ok(NotificationDeliveryResult {
//!                 notification_id: notification.id,
//!                 status: NotificationStatus::Delivered,
//!                 external_id: Some("mock-123".into()),
//!                 processing_time_ms: 10,
//!                 error_message: None,
//!                 delivered_at: chrono::Utc::now(),
//!                 channel: NotificationChannel::Email,
//!                 metadata: Default::default(),
//!             })
//!         }
//!     }
//!
//!     async fn health_check(&self) -> bool {
//!         !self.should_fail
//!     }
//!
//!     fn capabilities(&self) -> DeliveryCapabilities {
//!         DeliveryCapabilities {
//!             supports_bulk: false,
//!             supports_receipts: false,
//!             supports_attachments: true,
//!             supports_rich_content: true,
//!             supports_templates: false,
//!             max_attachment_size: Some(10 * 1024 * 1024),
//!             rate_limit: Some(100),
//!         }
//!     }
//! }
//! ```
//!
//! # Common Pitfalls
//!
//! 1. **Not Checking Capabilities**: Always check `capabilities()` before using advanced features
//! 2. **Ignoring Rate Limits**: Respect provider rate limits to avoid account suspension
//! 3. **Blocking on Delivery**: Use QueuePort for async delivery to avoid blocking agents
//! 4. **Missing Error Context**: Include external_id and provider error messages for debugging
//! 5. **No Health Checks**: Implement health_check() to detect provider outages early
//! 6. **Template Injection**: Sanitize user input in template variables to prevent injection
//!
//! # Related Modules
//!
//! - [`paladin_core::platform::container::notification`] - Domain types for notifications
//! - [`crate::output::queue_port`] - Async notification delivery queue
//! - [`crate::output::llm_port`] - LLM integration for generating notification content
//! - `crate::infrastructure::adapters::notification` - Concrete notification adapters (SMTP, Twilio, etc.)

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// Re-export domain types for convenience
pub use paladin_core::platform::container::notification::{
    Notification, NotificationAttachment, NotificationChannel, NotificationContent,
    NotificationDomainError, NotificationEvent, NotificationPriority, NotificationRecipient,
    NotificationStatus, NotificationTemplate,
};

/// Result type for notification port operations
pub type NotificationPortResult<T> = Result<T, NotificationPortError>;

/// Errors that can occur in notification port operations
///
/// This enum represents all possible error conditions when interacting with notification
/// delivery systems. Each variant indicates a specific failure mode and provides guidance
/// on whether the operation should be retried.
///
/// # Error Categories
///
/// - **Transient Errors**: Can be retried (RateLimitExceeded, ServiceUnavailable, ConnectionError)
/// - **Permanent Errors**: Should not be retried (AuthenticationError, TemplateError, ValidationError)
/// - **Contextual Errors**: Retry depends on context (DeliveryFailed, StorageError)
///
/// # Examples
///
/// ```
/// use paladin_ports::output::notification_port::NotificationPortError;
///
/// // Check if an error should be retried
/// fn should_retry(error: &NotificationPortError) -> bool {
///     matches!(
///         error,
///         NotificationPortError::RateLimitExceeded
///             | NotificationPortError::ServiceUnavailable(_)
///             | NotificationPortError::ConnectionError(_)
///             | NotificationPortError::Timeout
///     )
/// }
/// ```
#[derive(Debug, Clone, thiserror::Error)]
pub enum NotificationPortError {
    /// Domain-level error from notification entity validation
    ///
    /// **Retryable**: No - Fix the notification data
    ///
    /// **Recovery**: Validate notification fields before sending
    #[error("Domain error: {0}")]
    DomainError(#[from] NotificationDomainError),

    /// Failed to deliver notification to recipient
    ///
    /// **Retryable**: Maybe - Check the error message for details
    ///
    /// **Recovery**: Retry with exponential backoff if transient, otherwise check recipient address
    ///
    /// # Examples
    /// - "SMTP server rejected recipient" - Check email address validity
    /// - "Temporary network error" - Retry with backoff
    #[error("Delivery failed: {0}")]
    DeliveryFailed(String),

    /// Template rendering or validation error
    ///
    /// **Retryable**: No - Fix the template syntax or variables
    ///
    /// **Recovery**: Validate template syntax, ensure all variables are provided
    #[error("Template error: {0}")]
    TemplateError(String),

    /// Storage operation failed (reading/writing notification data)
    ///
    /// **Retryable**: Yes - Use exponential backoff
    ///
    /// **Recovery**: Check database connection, retry after delay
    #[error("Storage error: {0}")]
    StorageError(String),

    /// Network connection error
    ///
    /// **Retryable**: Yes - Retry with exponential backoff
    ///
    /// **Recovery**: Check network connectivity, DNS resolution, firewall rules
    #[error("Connection error: {0}")]
    ConnectionError(String),

    /// Authentication or authorization failed
    ///
    /// **Retryable**: No - Fix credentials in configuration
    ///
    /// **Recovery**: Verify API keys, credentials, OAuth tokens in configuration
    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    /// Rate limit exceeded for notification provider
    ///
    /// **Retryable**: Yes - Wait and retry
    ///
    /// **Recovery**: Implement rate limiting, exponential backoff, or use bulk delivery
    ///
    /// # Example
    /// ```
    /// use paladin_ports::output::notification_port::NotificationPortError;
    /// use std::time::Duration;
    /// use tokio::time::sleep;
    ///
    /// async fn handle_rate_limit(error: &NotificationPortError) -> bool {
    ///     if matches!(error, NotificationPortError::RateLimitExceeded) {
    ///         sleep(Duration::from_secs(60)).await; // Wait 1 minute
    ///         true // Retry
    ///     } else {
    ///         false
    ///     }
    /// }
    /// ```
    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    /// Notification service temporarily unavailable
    ///
    /// **Retryable**: Yes - Use circuit breaker pattern
    ///
    /// **Recovery**: Retry with exponential backoff, implement circuit breaker to prevent cascading failures
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    /// Configuration error (invalid settings)
    ///
    /// **Retryable**: No - Fix configuration
    ///
    /// **Recovery**: Validate notification port configuration at startup
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// Validation error (invalid notification data)
    ///
    /// **Retryable**: No - Fix notification data
    ///
    /// **Recovery**: Validate notification before sending (check recipient, content, attachments)
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Operation timed out
    ///
    /// **Retryable**: Yes - Retry with longer timeout
    ///
    /// **Recovery**: Increase timeout configuration, check network latency
    #[error("Timeout error")]
    Timeout,

    /// Unknown or unexpected error
    ///
    /// **Retryable**: Maybe - Depends on context
    ///
    /// **Recovery**: Log error details, investigate root cause
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
/// adapters must implement. It focuses purely on the delivery mechanism and provides
/// a channel-specific abstraction for sending notifications through various providers.
///
/// # Capabilities
///
/// - **Channel-Specific Delivery**: Each port handles one notification channel (Email, SMS, Push, etc.)
/// - **Single & Bulk Delivery**: Send individual notifications or batches for efficiency
/// - **Health Monitoring**: Check provider availability before sending
/// - **Capability Discovery**: Query supported features (attachments, templates, rate limits)
/// - **Async Execution**: All delivery operations are asynchronous for non-blocking performance
///
/// # Requirements
///
/// Implementations must:
/// - Be `Send + Sync` for safe concurrent use across async tasks
/// - Return accurate channel information via `channel()`
/// - Implement `can_handle()` to filter notifications by channel
/// - Provide comprehensive error context in `NotificationPortError`
/// - Track delivery status and external IDs for tracing
/// - Respect rate limits indicated in `capabilities()`
/// - Implement `health_check()` for circuit breaker patterns
///
/// # Examples
///
/// ## Basic Notification Delivery
///
/// ```ignore
/// use paladin_ports::output::notification_port::{
///     NotificationDeliveryPort, NotificationPortError
/// };
/// use paladin_core::platform::container::notification::{
///     Notification, NotificationChannel, NotificationContent,
///     NotificationRecipient, NotificationPriority
/// };
/// use std::sync::Arc;
///
/// async fn send_alert(
///     port: Arc<dyn NotificationDeliveryPort>,
/// ) -> Result<(), NotificationPortError> {
///     // Build notification
///     let notification = Notification::builder()
///         .recipient(NotificationRecipient::email("admin@example.com"))
///         .channel(NotificationChannel::Email)
///         .priority(NotificationPriority::High)
///         .content(NotificationContent::text(
///             "System Alert",
///             "High CPU usage detected on production server"
///         ))
///         .build()?;
///
///     // Check if port can handle this channel
///     if !port.can_handle(&notification) {
///         return Err(NotificationPortError::ValidationError(
///             "Port cannot handle this notification channel".into()
///         ));
///     }
///
///     // Deliver notification
///     let result = port.deliver_notification(notification).await?;
///     println!("Delivered via {}: {:?}", result.channel, result.status);
///     Ok(())
/// }
/// ```
///
/// ## Health Check Before Delivery
///
/// ```ignore
/// use paladin_ports::output::notification_port::{
///     NotificationDeliveryPort, NotificationPortError
/// };
/// use std::sync::Arc;
/// use std::time::Duration;
/// use tokio::time::sleep;
///
/// async fn send_with_health_check(
///     port: Arc<dyn NotificationDeliveryPort>,
///     notification: paladin_core::platform::container::notification::Notification,
/// ) -> Result<(), NotificationPortError> {
///     // Check health before sending
///     if !port.health_check().await {
///         // Wait and retry health check
///         sleep(Duration::from_secs(5)).await;
///         if !port.health_check().await {
///             return Err(NotificationPortError::ServiceUnavailable(
///                 "Notification service is down".into()
///             ));
///         }
///     }
///
///     port.deliver_notification(notification).await?;
///     Ok(())
/// }
/// ```
///
/// ## Query Capabilities
///
/// ```ignore
/// use paladin_ports::output::notification_port::NotificationDeliveryPort;
/// use std::sync::Arc;
///
/// fn check_capabilities(port: Arc<dyn NotificationDeliveryPort>) {
///     let caps = port.capabilities();
///
///     if caps.supports_bulk {
///         println!("Port supports bulk delivery");
///     }
///
///     if let Some(limit) = caps.rate_limit {
///         println!("Rate limit: {} messages/minute", limit);
///     }
///
///     if let Some(max_size) = caps.max_attachment_size {
///         println!("Max attachment size: {} bytes", max_size);
///     }
/// }
/// ```
///
/// # Implementation Notes
///
/// ## Error Handling
///
/// Map provider-specific errors to appropriate `NotificationPortError` variants:
/// - Network errors → `ConnectionError`
/// - API auth failures → `AuthenticationError`
/// - Rate limiting → `RateLimitExceeded`
/// - Service outages → `ServiceUnavailable`
/// - Invalid data → `ValidationError`
///
/// ## Performance Tips
///
/// 1. **Connection Pooling**: Reuse HTTP/SMTP connections across deliveries
/// 2. **Bulk Delivery**: Override `deliver_bulk()` with native provider bulk APIs when available
/// 3. **Async I/O**: Use tokio for non-blocking external calls
/// 4. **Timeout Configuration**: Set reasonable timeouts (5-30 seconds typical)
/// 5. **Circuit Breaker**: Use `health_check()` to detect provider outages early
///
/// ## Testing
///
/// See module-level documentation for mock implementation example.
#[async_trait]
pub trait NotificationDeliveryPort: Send + Sync {
    /// Get the notification channel this port handles
    ///
    /// Returns the specific channel (Email, SMS, Push, Webhook, Slack, InApp)
    /// that this port implementation supports.
    fn channel(&self) -> NotificationChannel;

    /// Check if this port can handle the given notification
    ///
    /// Implementations should check if the notification's channel matches
    /// the channel supported by this port, and optionally validate other
    /// requirements (e.g., recipient format, content type).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use paladin_core::platform::container::notification::{Notification, NotificationChannel};
    ///
    /// fn can_handle(&self, notification: &Notification) -> bool {
    ///     notification.channel == NotificationChannel::Email
    ///         && notification.recipient.email().is_some()
    /// }
    /// ```
    fn can_handle(&self, notification: &Notification) -> bool;

    /// Deliver a single notification
    ///
    /// Sends the notification through the provider's API and returns detailed
    /// delivery status including external tracking IDs, timestamps, and any errors.
    ///
    /// # Errors
    ///
    /// - `DeliveryFailed`: Notification rejected by provider or delivery failed
    /// - `ConnectionError`: Network or connection issue
    /// - `AuthenticationError`: Invalid credentials
    /// - `RateLimitExceeded`: Provider rate limit hit
    /// - `ServiceUnavailable`: Provider is down
    /// - `ValidationError`: Invalid notification data
    /// - `Timeout`: Operation exceeded timeout
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use paladin_ports::output::notification_port::{
    ///     NotificationDeliveryPort, NotificationDeliveryResult
    /// };
    /// use paladin_core::platform::container::notification::Notification;
    ///
    /// async fn deliver_notification(
    ///     &self,
    ///     notification: Notification,
    /// ) -> Result<NotificationDeliveryResult, NotificationPortError> {
    ///     // Call external provider API
    ///     let external_id = self.provider_api.send(&notification).await?;
    ///
    ///     Ok(NotificationDeliveryResult {
    ///         notification_id: notification.id,
    ///         status: NotificationStatus::Sent,
    ///         external_id: Some(external_id),
    ///         processing_time_ms: 250,
    ///         error_message: None,
    ///         delivered_at: chrono::Utc::now(),
    ///         channel: self.channel(),
    ///         metadata: Default::default(),
    ///     })
    /// }
    /// ```
    async fn deliver_notification(
        &self,
        notification: Notification,
    ) -> NotificationPortResult<NotificationDeliveryResult>;

    /// Deliver multiple notifications (if supported)
    ///
    /// Sends multiple notifications in a single operation when the provider
    /// supports bulk delivery. Default implementation sends notifications
    /// sequentially using `deliver_notification()`.
    ///
    /// **Override this method** if your provider has native bulk APIs (e.g., SendGrid, Twilio)
    /// for better performance and reduced API calls.
    ///
    /// # Performance
    ///
    /// - Default: O(n) API calls (one per notification)
    /// - Bulk API: O(1) or O(n/batch_size) API calls
    ///
    /// # Examples
    ///
    /// ```ignore
    /// async fn deliver_bulk(
    ///     &self,
    ///     notifications: Vec<Notification>,
    /// ) -> Result<BulkDeliveryResult, NotificationPortError> {
    ///     // Use provider's bulk API
    ///     let results = self.provider_api.send_bulk(&notifications).await?;
    ///
    ///     Ok(BulkDeliveryResult {
    ///         total_count: notifications.len(),
    ///         success_count: results.iter().filter(|r| r.success).count(),
    ///         failure_count: results.iter().filter(|r| !r.success).count(),
    ///         results: results.into_iter().map(|r| r.into()).collect(),
    ///         total_processing_time_ms: 500,
    ///     })
    /// }
    /// ```
    async fn deliver_bulk(
        &self,
        notifications: Vec<Notification>,
    ) -> NotificationPortResult<BulkDeliveryResult> {
        // Default implementation delivers one by one
        let mut results = Vec::new();
        let start_time = std::time::Instant::now();
        let mut success_count = 0;
        let mut failure_count = 0;

        for notification in notifications {
            match self.deliver_notification(notification).await {
                Ok(result) => {
                    if matches!(
                        result.status,
                        NotificationStatus::Sent | NotificationStatus::Delivered
                    ) {
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
    ///
    /// Returns `true` if the notification provider is reachable and healthy,
    /// `false` otherwise. Used for circuit breaker patterns and monitoring.
    ///
    /// Implementations should perform a lightweight check (e.g., ping endpoint,
    /// check authentication) without sending actual notifications.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// async fn health_check(&self) -> bool {
    ///     // Simple connectivity check
    ///     self.provider_api.ping().await.is_ok()
    /// }
    /// ```
    async fn health_check(&self) -> bool;

    /// Get delivery port capabilities
    ///
    /// Returns metadata about what features this port supports (bulk delivery,
    /// attachments, templates, rate limits, etc.). Callers should check
    /// capabilities before using advanced features.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use paladin_ports::output::notification_port::DeliveryCapabilities;
    ///
    /// fn capabilities(&self) -> DeliveryCapabilities {
    ///     DeliveryCapabilities {
    ///         supports_bulk: true,
    ///         supports_receipts: true,
    ///         supports_attachments: true,
    ///         supports_rich_content: true, // HTML email
    ///         supports_templates: true,
    ///         max_attachment_size: Some(25 * 1024 * 1024), // 25MB
    ///         rate_limit: Some(1000), // 1000 per minute
    ///     }
    /// }
    /// ```
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
/// This port handles template management and content rendering for notifications.
/// Templates allow you to define reusable notification formats with variable
/// substitution, supporting multiple channels and localization.
///
/// # Capabilities
///
/// - **Template CRUD**: Create, read, update, delete notification templates
/// - **Variable Rendering**: Render templates with dynamic variables
/// - **Channel Support**: Filter templates by notification channel
/// - **Syntax Validation**: Validate template syntax before saving
/// - **Localization**: Support for multi-language templates (implementation-specific)
///
/// # Requirements
///
/// Implementations must:
/// - Support variable substitution with safe escaping
/// - Validate template syntax to prevent injection attacks
/// - Handle missing variables gracefully (error or default values)
/// - Be thread-safe (`Send + Sync`)
///
/// # Examples
///
/// ## Create and Render Template
///
/// ```ignore
/// use paladin_ports::output::notification_port::{
///     NotificationTemplatePort, NotificationPortError
/// };
/// use paladin_core::platform::container::notification::{
///     NotificationTemplate, NotificationChannel
/// };
/// use std::collections::HashMap;
/// use std::sync::Arc;
///
/// async fn use_template(
///     port: Arc<dyn NotificationTemplatePort>,
/// ) -> Result<(), NotificationPortError> {
///     // Create template
///     let template = NotificationTemplate::new(
///         "welcome_email",
///         NotificationChannel::Email,
///         "Welcome {{user_name}}!",
///         "Hello {{user_name}}, welcome to our platform!"
///     );
///
///     let template_id = port.create_template(template).await?;
///
///     // Render with variables
///     let mut vars = HashMap::new();
///     vars.insert("user_name".to_string(), serde_json::json!("Alice"));
///
///     let content = port.render_template(&template_id, vars).await?;
///     println!("Subject: {}", content.subject);
///     println!("Body: {}", content.body);
///     Ok(())
/// }
/// ```
///
/// ## List and Validate Templates
///
/// ```ignore
/// use paladin_ports::output::notification_port::NotificationTemplatePort;
/// use paladin_core::platform::container::notification::NotificationChannel;
/// use std::sync::Arc;
///
/// async fn audit_templates(port: Arc<dyn NotificationTemplatePort>) {
///     // List all email templates
///     let templates = port
///         .list_templates(Some(NotificationChannel::Email))
///         .await
///         .unwrap();
///
///     for template in templates {
///         // Validate each template
///         match port.validate_template(&template).await {
///             Ok(_) => println!("✓ Template '{}' is valid", template.name),
///             Err(e) => println!("✗ Template '{}' invalid: {}", template.name, e),
///         }
///     }
/// }
/// ```
///
/// # Implementation Notes
///
/// ## Template Engines
///
/// Common template engines for Rust:
/// - **Handlebars**: `{{variable}}` syntax, helpers, partials
/// - **Tera**: Django/Jinja2-like syntax, filters, macros
/// - **Liquid**: Shopify's template language
/// - **Askama**: Compile-time type-safe templates
///
/// ## Security
///
/// - **Sanitize Variables**: Escape HTML in email templates to prevent XSS
/// - **Validate Syntax**: Reject templates with invalid syntax at creation time
/// - **Limit Complexity**: Prevent templates from executing arbitrary code
/// - **Audit Access**: Log template modifications for security auditing
///
/// ## Performance
///
/// - **Template Caching**: Cache compiled templates to avoid repeated parsing
/// - **Async Rendering**: Render templates asynchronously for large batches
/// - **Precompilation**: Compile templates at startup for production use
#[async_trait]
pub trait NotificationTemplatePort: Send + Sync {
    /// Create a new template
    ///
    /// Stores a notification template for future use. Template syntax is validated
    /// before storage.
    ///
    /// # Returns
    ///
    /// The template ID (string) for future reference.
    ///
    /// # Errors
    ///
    /// - `TemplateError`: Invalid template syntax
    /// - `ValidationError`: Invalid template data
    /// - `StorageError`: Failed to store template
    async fn create_template(
        &self,
        template: NotificationTemplate,
    ) -> NotificationPortResult<String>;

    /// Update an existing template
    ///
    /// Replaces an existing template with new content. Template ID must match.
    ///
    /// # Errors
    ///
    /// - `TemplateError`: Invalid template syntax
    /// - `ValidationError`: Template not found or invalid data
    /// - `StorageError`: Failed to update template
    async fn update_template(&self, template: NotificationTemplate) -> NotificationPortResult<()>;

    /// Delete a template
    ///
    /// Removes a template from storage. This operation cannot be undone.
    ///
    /// # Errors
    ///
    /// - `ValidationError`: Template not found
    /// - `StorageError`: Failed to delete template
    async fn delete_template(&self, template_id: &str) -> NotificationPortResult<()>;

    /// Get a template by ID
    ///
    /// Retrieves a template for inspection or rendering.
    ///
    /// # Errors
    ///
    /// - `ValidationError`: Template not found
    /// - `StorageError`: Failed to retrieve template
    async fn get_template(&self, template_id: &str)
    -> NotificationPortResult<NotificationTemplate>;

    /// List templates with optional filtering
    ///
    /// Returns all templates, optionally filtered by notification channel.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // List all templates
    /// let all = port.list_templates(None).await?;
    ///
    /// // List only email templates
    /// let emails = port.list_templates(Some(NotificationChannel::Email)).await?;
    /// ```
    async fn list_templates(
        &self,
        channel: Option<NotificationChannel>,
    ) -> NotificationPortResult<Vec<NotificationTemplate>>;

    /// Render template with variables
    ///
    /// Substitutes variables into the template and returns rendered content.
    /// Variable values should be JSON-serializable.
    ///
    /// # Errors
    ///
    /// - `TemplateError`: Missing required variables or rendering failed
    /// - `ValidationError`: Template not found
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut vars = HashMap::new();
    /// vars.insert("name".to_string(), serde_json::json!("Alice"));
    /// vars.insert("order_id".to_string(), serde_json::json!(12345));
    /// vars.insert("total".to_string(), serde_json::json!(99.99));
    ///
    /// let content = port.render_template("order_confirmation", vars).await?;
    /// ```
    async fn render_template(
        &self,
        template_id: &str,
        variables: HashMap<String, serde_json::Value>,
    ) -> NotificationPortResult<NotificationContent>;

    /// Validate template syntax
    ///
    /// Checks if a template has valid syntax without rendering it.
    /// Use this before `create_template()` or `update_template()` to validate user input.
    ///
    /// # Errors
    ///
    /// - `TemplateError`: Invalid syntax or structure
    async fn validate_template(
        &self,
        template: &NotificationTemplate,
    ) -> NotificationPortResult<()>;
}

/// Basic notification port for simple use cases
///
/// This trait combines delivery functionality with a simplified API for straightforward
/// notification scenarios. It's a convenience wrapper around `NotificationDeliveryPort`
/// for applications that don't need template management or advanced features.
///
/// Use this trait when you:
/// - Only need basic send functionality
/// - Don't use templates or complex workflows
/// - Want a simpler API surface
///
/// # Examples
///
/// ```ignore
/// use paladin_ports::output::notification_port::{
///     BasicNotificationPort, NotificationPortError
/// };
/// use paladin_core::platform::container::notification::{
///     Notification, NotificationChannel, NotificationContent, NotificationRecipient
/// };
/// use std::sync::Arc;
///
/// async fn send_simple(
///     port: Arc<dyn BasicNotificationPort>,
/// ) -> Result<(), NotificationPortError> {
///     let notification = Notification::builder()
///         .recipient(NotificationRecipient::email("user@example.com"))
///         .channel(NotificationChannel::Email)
///         .content(NotificationContent::text("Hello", "This is a test"))
///         .build()?;
///
///     port.send_notification(notification).await?;
///     Ok(())
/// }
/// ```
#[async_trait]
pub trait BasicNotificationPort: NotificationDeliveryPort + Send + Sync {
    /// Send a notification and return delivery result
    async fn send_notification(
        &self,
        notification: Notification,
    ) -> NotificationPortResult<NotificationDeliveryResult> {
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
            rate_limit: Some(100),                       // 100 per minute
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
