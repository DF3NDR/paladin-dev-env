pub mod email_notification_adapter;
pub mod push_notification_adapter;
pub mod system_notification_adapter;

// Re-export main adapters for convenience
pub use email_notification_adapter::{EmailNotificationAdapter, EmailAdapterConfig};
pub use system_notification_adapter::{SystemNotificationAdapter, SystemAdapterConfig};