/*
Queue Item Container

A Queue Item is a type of Message that is placed in a Queue via the Queue Manager Service.

A Queue Item will be abstracted in the Application Layer along with an abstraction of the
Queue Manager Service. 

This container is the base for the Queue Item Message Container on which more complex 
Queue Item Messages can be built.

Example:
A typical use case would be a Job Queue where the Queue Item is a Job that needs to be 
processed. The Job Queue Item and Job Queue Manager Service would be defined in the
Application Layer.
*/

use crate::core::base::entity::message::{Message, MessagePriority};
use crate::core::base::component::action::{Action, ActionResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Queue Item processing status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueueItemStatus {
    /// Item is waiting to be processed
    Pending,
    /// Item is currently being processed
    Processing,
    /// Item processing completed successfully
    Completed,
    /// Item processing failed
    Failed,
    /// Item processing was cancelled
    Cancelled,
    /// Item was deferred for later processing
    Deferred,
    /// Item exceeded retry limits
    Abandoned,
}

impl Default for QueueItemStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// Queue Item configuration for processing behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItemConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Delay between retries in milliseconds
    pub retry_delay_ms: u64,
    /// Time-to-live in seconds (0 = no expiration)
    pub ttl_seconds: u64,
    /// Processing timeout in seconds
    pub timeout_seconds: u64,
    /// Whether to preserve the item after completion
    pub preserve_after_completion: bool,
}

impl Default for QueueItemConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay_ms: 1000,
            ttl_seconds: 3600, // 1 hour
            timeout_seconds: 300, // 5 minutes
            preserve_after_completion: false,
        }
    }
}

/// Base Queue Item container that extends the Action component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem<T> {
    /// Base action functionality
    pub action: Action,
    /// The underlying message containing the payload
    pub message: Message<T>,
    /// Current processing status
    pub status: QueueItemStatus,
    /// Queue Item configuration
    pub config: QueueItemConfig,
    /// Number of processing attempts
    pub attempt_count: u32,
    /// Queue name this item belongs to
    pub queue_name: String,
    /// Processing worker identifier
    pub worker_id: Option<String>,
    /// Time when processing started
    pub processing_started_at: Option<DateTime<Utc>>,
    /// Time when item was last deferred
    pub deferred_until: Option<DateTime<Utc>>,
    /// Additional queue-specific metadata
    pub queue_metadata: HashMap<String, serde_json::Value>,
}

impl<T> QueueItem<T> 
where 
    T: Clone + serde::Serialize + for<'de> serde::Deserialize<'de>,
{
    /// Create a new queue item
    pub fn new(
        queue_name: String,
        message: Message<T>,
        config: Option<QueueItemConfig>,
    ) -> Self {
        let item_name = format!("Queue Item: {}", message.id);
        let description = format!("Queue item for queue '{}' from {}", queue_name, message.source);
        
        let action = Action::new(
            item_name,
            description,
            message.source.to_string(),
            queue_name.clone(),
        ).with_priority(match message.priority {
            MessagePriority::Low => crate::core::base::component::action::ActionPriority::Low,
            MessagePriority::Normal => crate::core::base::component::action::ActionPriority::Normal,
            MessagePriority::High => crate::core::base::component::action::ActionPriority::High,
            MessagePriority::Critical => crate::core::base::component::action::ActionPriority::Critical,
        });

        Self {
            action,
            message,
            status: QueueItemStatus::Pending,
            config: config.unwrap_or_default(),
            attempt_count: 0,
            queue_name,
            worker_id: None,
            processing_started_at: None,
            deferred_until: None,
            queue_metadata: HashMap::new(),
        }
    }

    /// Create a queue item with custom configuration
    pub fn with_config(
        queue_name: String,
        message: Message<T>,
        config: QueueItemConfig,
    ) -> Self {
        Self::new(queue_name, message, Some(config))
    }

    /// Get the queue item ID
    pub fn id(&self) -> Uuid {
        self.action.id
    }

    /// Get the message ID
    pub fn message_id(&self) -> Uuid {
        self.message.id
    }

    /// Get the message payload
    pub fn payload(&self) -> &T {
        &self.message.message
    }

    /// Get a mutable reference to the message payload
    pub fn payload_mut(&mut self) -> &mut T {
        &mut self.message.message
    }

    /// Check if the item can be processed
    pub fn can_process(&self) -> bool {
        match self.status {
            QueueItemStatus::Pending => true,
            QueueItemStatus::Deferred => {
                if let Some(deferred_until) = self.deferred_until {
                    Utc::now() >= deferred_until
                } else {
                    true
                }
            }
            _ => false,
        }
    }

    /// Check if the item is expired
    pub fn is_expired(&self) -> bool {
        if self.config.ttl_seconds == 0 {
            return false; // No expiration
        }
        self.message.is_expired(self.config.ttl_seconds as i64)
    }

    /// Check if the item has exceeded retry limits
    pub fn is_retry_exhausted(&self) -> bool {
        self.attempt_count > self.config.max_retries
    }

    /// Check if the item has timed out during processing
    pub fn is_processing_timeout(&self) -> bool {
        if let Some(started_at) = self.processing_started_at {
            let elapsed = Utc::now().timestamp() - started_at.timestamp();
            elapsed > self.config.timeout_seconds as i64
        } else {
            false
        }
    }

    /// Start processing the queue item
    pub fn start_processing(&mut self, worker_id: String) -> Result<(), String> {
        if !self.can_process() {
            return Err(format!("Queue item cannot be processed, current status: {:?}", self.status));
        }

        if self.is_expired() {
            self.status = QueueItemStatus::Abandoned;
            return Err("Queue item has expired".to_string());
        }

        self.status = QueueItemStatus::Processing;
        self.worker_id = Some(worker_id);
        self.processing_started_at = Some(Utc::now());
        self.attempt_count += 1;
        
        // Start the underlying action
        self.action.start_execution();
        
        Ok(())
    }

    /// Complete processing successfully
    pub fn complete_processing(&mut self, result_data: Option<serde_json::Value>) {
        self.status = QueueItemStatus::Completed;
        self.processing_started_at = None;
        
        let action_result = ActionResult {
            success: true,
            duration_ms: self.processing_duration_ms(),
            data: result_data,
            error: None,
            metadata: self.queue_metadata.clone(),
        };
        
        self.action.complete_execution(action_result);
    }

    /// Fail processing with an error
    pub fn fail_processing(&mut self, error: String) -> bool {
        self.status = QueueItemStatus::Failed;
        self.processing_started_at = None;
        
        let duration_ms = self.processing_duration_ms();
        let can_retry = self.action.fail_execution(error.clone(), duration_ms) && !self.is_retry_exhausted();
        
        if can_retry {
            self.status = QueueItemStatus::Pending; // Reset to pending for retry
        } else {
            self.status = QueueItemStatus::Abandoned;
        }
        
        can_retry
    }

    /// Cancel processing
    pub fn cancel_processing(&mut self) {
        self.status = QueueItemStatus::Cancelled;
        self.processing_started_at = None;
        self.worker_id = None;
        self.action.cancel();
    }

    /// Defer processing until a specific time
    pub fn defer_processing(&mut self, defer_until: DateTime<Utc>) {
        self.status = QueueItemStatus::Deferred;
        self.deferred_until = Some(defer_until);
        self.processing_started_at = None;
        self.worker_id = None;
    }

    /// Get processing duration in milliseconds
    fn processing_duration_ms(&self) -> u64 {
        if let Some(started_at) = self.processing_started_at {
            let elapsed = Utc::now().timestamp_millis() - started_at.timestamp_millis();
            elapsed.max(0) as u64
        } else {
            0
        }
    }

    /// Add queue-specific metadata
    pub fn add_metadata(&mut self, key: String, value: serde_json::Value) {
        self.queue_metadata.insert(key, value);
    }

    /// Get queue-specific metadata
    pub fn get_metadata(&self, key: &str) -> Option<&serde_json::Value> {
        self.queue_metadata.get(key)
    }

    /// Transform the payload to a different type
    pub fn map_payload<U, F>(self, f: F) -> QueueItem<U>
    where
        F: FnOnce(T) -> U,
        U: Clone + serde::Serialize + for<'de> serde::Deserialize<'de>,
    {
        let new_message = self.message.map(f);
        
        QueueItem {
            action: self.action,
            message: new_message,
            status: self.status,
            config: self.config,
            attempt_count: self.attempt_count,
            queue_name: self.queue_name,
            worker_id: self.worker_id,
            processing_started_at: self.processing_started_at,
            deferred_until: self.deferred_until,
            queue_metadata: self.queue_metadata,
        }
    }

    /// Create a summary for monitoring/logging
    pub fn summary(&self) -> QueueItemSummary {
        QueueItemSummary {
            id: self.id(),
            message_id: self.message_id(),
            queue_name: self.queue_name.clone(),
            status: self.status.clone(),
            attempt_count: self.attempt_count,
            worker_id: self.worker_id.clone(),
            age_seconds: self.message.age_seconds(),
            is_expired: self.is_expired(),
            processing_duration_ms: if self.status == QueueItemStatus::Processing {
                Some(self.processing_duration_ms())
            } else {
                None
            },
        }
    }
}

/// Summary information for queue item monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItemSummary {
    pub id: Uuid,
    pub message_id: Uuid,
    pub queue_name: String,
    pub status: QueueItemStatus,
    pub attempt_count: u32,
    pub worker_id: Option<String>,
    pub age_seconds: i64,
    pub is_expired: bool,
    pub processing_duration_ms: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::base::entity::message::Location;

    #[test]
    fn test_queue_item_creation() {
        let message = Message::new(
            Location::service("test-service"),
            Location::system("queue-system"),
            "test payload".to_string(),
        );
        
        let queue_item = QueueItem::new("test-queue".to_string(), message, None);
        
        assert_eq!(queue_item.queue_name, "test-queue");
        assert_eq!(queue_item.status, QueueItemStatus::Pending);
        assert_eq!(queue_item.attempt_count, 0);
        assert!(queue_item.can_process());
    }

    #[test]
    fn test_queue_item_processing_lifecycle() {
        let message = Message::new(
            Location::service("test"),
            Location::system("queue"),
            "payload".to_string(),
        );
        
        let mut queue_item = QueueItem::new("test".to_string(), message, None);
        
        // Start processing
        assert!(queue_item.start_processing("worker-1".to_string()).is_ok());
        assert_eq!(queue_item.status, QueueItemStatus::Processing);
        assert_eq!(queue_item.worker_id, Some("worker-1".to_string()));
        assert_eq!(queue_item.attempt_count, 1);
        
        // Complete processing
        queue_item.complete_processing(Some(serde_json::json!({"result": "success"})));
        assert_eq!(queue_item.status, QueueItemStatus::Completed);
    }

    #[test]
    fn test_queue_item_retry_logic() {
        let message = Message::new(
            Location::service("test"),
            Location::system("queue"),
            "payload".to_string(),
        );
        
        let config = QueueItemConfig {
            max_retries: 2,
            ..Default::default()
        };
        
        let mut queue_item = QueueItem::with_config("test".to_string(), message, config);
        
        // First failure - should allow retry
        queue_item.start_processing("worker-1".to_string()).unwrap();
        let can_retry = queue_item.fail_processing("Test error".to_string());
        assert!(can_retry);
        assert_eq!(queue_item.status, QueueItemStatus::Pending);
        
        // Second failure - should allow retry
        queue_item.start_processing("worker-1".to_string()).unwrap();
        let can_retry = queue_item.fail_processing("Test error 2".to_string());
        assert!(can_retry);
        
        // Third failure - should not allow retry (exceeded max_retries)
        queue_item.start_processing("worker-1".to_string()).unwrap();
        let can_retry = queue_item.fail_processing("Test error 3".to_string());
        assert!(!can_retry);
        assert_eq!(queue_item.status, QueueItemStatus::Abandoned);
    }

    #[test]
    fn test_queue_item_deferral() {
        let message = Message::new(
            Location::service("test"),
            Location::system("queue"),
            "payload".to_string(),
        );
        
        let mut queue_item = QueueItem::new("test".to_string(), message, None);
        
        let defer_until = Utc::now() + chrono::Duration::minutes(5);
        queue_item.defer_processing(defer_until);
        
        assert_eq!(queue_item.status, QueueItemStatus::Deferred);
        assert!(!queue_item.can_process()); // Should not be processable yet
    }
}