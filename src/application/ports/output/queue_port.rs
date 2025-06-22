/*
Queue Port Module

This module defines the Queue Port, which is an interface for the Queue Manager Service.
It is used to abstract the Queue Manager Service from the Application Layer, allowing for
different implementations of the Queue Manager Service to be used without changing the
Application Layer code.
The Queue Port defines the methods that the Application Layer can use to interact with the
Queue Manager Service, such as adding items to the queue, retrieving items from the queue,
and checking the status of the queue.
This allows for a clean separation of concerns and makes it easier to test the Application Layer
without relying on the actual Queue Manager Service implementation.

The Queue Port is part of the Application Layer in our Hexagonal Architecture, sitting above 
the Platform Layer it allows for Queue Adapters to be implemented in the Infrastructure Layer.
It is designed to be flexible and extensible, allowing for different queue implementations
to be with specific Queues developed within the Application Layer code. The external queue 
implementation is specified by the adapter. 
*/

use crate::core::platform::container::queue_item::{QueueItem, QueueItemConfig, QueueItemSummary};
use crate::core::platform::manager::queue_service::{QueueStats, QueueConfig, QueueError};
use async_trait::async_trait;
use uuid::Uuid;
use std::collections::HashMap;
use serde::{Serialize, de::DeserializeOwned};

/// Queue Port trait - defines the interface for queue operations
/// This trait abstracts the queue service implementation from the application layer
#[async_trait]
pub trait QueuePort: Send + Sync {
    /// Create a new queue with optional configuration
    async fn create_queue(&self, name: String, config: Option<QueueConfig>) -> Result<(), QueueError>;
    
    /// Delete an existing queue
    async fn delete_queue(&self, name: &str) -> Result<(), QueueError>;
    
    /// Enqueue an item into a specific queue
    async fn enqueue<T>(&self, queue_name: &str, item: QueueItem<T>) -> Result<Uuid, QueueError>
    where
        T: Serialize + Clone + for<'de> serde::Deserialize<'de> + Send + Sync;
    
    /// Dequeue an item from a specific queue
    async fn dequeue(&self, queue_name: &str) -> Result<Option<QueueItem<serde_json::Value>>, QueueError>;
    
    /// Start processing an item (mark as in-progress)
    async fn start_processing(&self, queue_name: &str, item_id: Uuid, worker_id: String) -> Result<(), QueueError>;
    
    /// Complete processing an item successfully
    async fn complete_processing(&self, queue_name: &str, item_id: Uuid, result_data: Option<serde_json::Value>) -> Result<(), QueueError>;
    
    /// Fail processing an item (with potential retry)
    async fn fail_processing(&self, queue_name: &str, item_id: Uuid, error: String) -> Result<bool, QueueError>;
    
    /// Get statistics for a specific queue
    async fn get_queue_stats(&self, queue_name: &str) -> Result<QueueStats, QueueError>;
    
    /// List all available queues
    async fn list_queues(&self) -> Vec<String>;
    
    /// Get the length of a specific queue
    async fn queue_length(&self, queue_name: &str) -> Result<usize, QueueError>;
    
    /// Cleanup expired items across all queues
    async fn cleanup_expired(&self);
    
    /// Get statistics for all queues
    async fn get_all_stats(&self) -> HashMap<String, QueueStats>;
    
    /// Health check for the queue service
    async fn health_check(&self) -> Result<bool, QueueError>;
}

/// Specialized queue port for typed queue operations
/// This provides type-safe operations for specific queue types
#[async_trait]
pub trait TypedQueuePort<T>: Send + Sync 
where 
    T: Serialize + DeserializeOwned + Send + Sync + Clone,
{
    /// Enqueue a strongly-typed item
    async fn enqueue_typed(&self, queue_name: &str, item: QueueItem<T>) -> Result<Uuid, QueueError>;
    
    /// Dequeue a strongly-typed item
    async fn dequeue_typed(&self, queue_name: &str) -> Result<Option<QueueItem<T>>, QueueError>;
    
    /// Process items with a typed handler
    async fn process_with_handler<F, Fut>(&self, queue_name: &str, handler: F) -> Result<(), QueueError>
    where
        F: Fn(QueueItem<T>) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<Option<serde_json::Value>, String>> + Send;
}

/// Batch queue operations port
#[async_trait]
pub trait BatchQueuePort: Send + Sync {
    /// Enqueue multiple items at once
    async fn enqueue_batch<T>(&self, queue_name: &str, items: Vec<QueueItem<T>>) -> Result<Vec<Uuid>, QueueError>
    where
        T: Serialize + Clone + for<'de> serde::Deserialize<'de> + Send + Sync;

    /// Enqueue with explicit priority override
    async fn enqueue_with_priority<T>(&self, queue_name: &str, item: QueueItem<T>, priority: crate::core::base::entity::message::MessagePriority) -> Result<Uuid, QueueError>
    where
        T: Serialize + Clone + for<'de> serde::Deserialize<'de> + Send + Sync;
    
    /// Dequeue multiple items at once
    async fn dequeue_batch(&self, queue_name: &str, count: usize) -> Result<Vec<QueueItem<serde_json::Value>>, QueueError>;
    
    /// Get summaries for multiple items
    async fn get_item_summaries(&self, queue_name: &str, item_ids: Vec<Uuid>) -> Result<Vec<QueueItemSummary>, QueueError>;
}

/// Priority queue operations port
#[async_trait]
pub trait PriorityQueuePort: Send + Sync {
    /// Enqueue with explicit priority override
    async fn enqueue_with_priority<T>(&self, queue_name: &str, item: QueueItem<T>, priority: crate::core::base::entity::message::MessagePriority) -> Result<Uuid, QueueError>
    where
        T: Serialize + Send + Sync;
    
    /// Dequeue highest priority item
    async fn dequeue_highest_priority(&self, queue_name: &str) -> Result<Option<QueueItem<serde_json::Value>>, QueueError>;
    
    /// Get items by priority level
    async fn get_items_by_priority(&self, queue_name: &str, priority: crate::core::base::entity::message::MessagePriority) -> Result<Vec<QueueItemSummary>, QueueError>;
}

/// Monitoring and management port for queue operations
#[async_trait]
pub trait QueueManagementPort: Send + Sync {
    /// Pause processing for a queue
    async fn pause_queue(&self, queue_name: &str) -> Result<(), QueueError>;
    
    /// Resume processing for a queue
    async fn resume_queue(&self, queue_name: &str) -> Result<(), QueueError>;
    
    /// Cancel a specific item
    async fn cancel_item(&self, queue_name: &str, item_id: Uuid) -> Result<(), QueueError>;
    
    /// Retry a failed item
    async fn retry_item(&self, queue_name: &str, item_id: Uuid) -> Result<(), QueueError>;
    
    /// Get detailed item information
    async fn get_item_details(&self, queue_name: &str, item_id: Uuid) -> Result<QueueItem<serde_json::Value>, QueueError>;
    
    /// Purge completed items from a queue
    async fn purge_completed(&self, queue_name: &str) -> Result<usize, QueueError>;
    
    /// Purge failed items from a queue
    async fn purge_failed(&self, queue_name: &str) -> Result<usize, QueueError>;
    
    /// Get queue configuration
    async fn get_queue_config(&self, queue_name: &str) -> Result<QueueConfig, QueueError>;
    
    /// Update queue configuration
    async fn update_queue_config(&self, queue_name: &str, config: QueueConfig) -> Result<(), QueueError>;
}

/// Combined queue port that includes all queue operations
/// This is the main port that application services should depend on
pub trait FullQueuePort: QueuePort + BatchQueuePort + PriorityQueuePort + QueueManagementPort + Send + Sync {}

/// Helper trait for creating queue items
pub trait QueueItemFactory {
    /// Create a queue item from a message
    fn create_item<T>(
        &self,
        queue_name: String,
        payload: T,
        source: crate::core::base::entity::message::Location,
        destination: crate::core::base::entity::message::Location,
        config: Option<QueueItemConfig>,
    ) -> QueueItem<T>
    where
        T: Clone + Serialize + DeserializeOwned;
    
    /// Create a priority queue item
    fn create_priority_item<T>(
        &self,
        queue_name: String,
        payload: T,
        source: crate::core::base::entity::message::Location,
        destination: crate::core::base::entity::message::Location,
        priority: crate::core::base::entity::message::MessagePriority,
        config: Option<QueueItemConfig>,
    ) -> QueueItem<T>
    where
        T: Clone + Serialize + DeserializeOwned;
}

/// Default implementation of QueueItemFactory
pub struct DefaultQueueItemFactory;

impl QueueItemFactory for DefaultQueueItemFactory {
    fn create_item<T>(
        &self,
        queue_name: String,
        payload: T,
        source: crate::core::base::entity::message::Location,
        destination: crate::core::base::entity::message::Location,
        config: Option<QueueItemConfig>,
    ) -> QueueItem<T>
    where
        T: Clone + Serialize + DeserializeOwned,
    {
        let message = crate::core::base::entity::message::Message::new(source, destination, payload);
        QueueItem::new(queue_name, message, config)
    }
    
    fn create_priority_item<T>(
        &self,
        queue_name: String,
        payload: T,
        source: crate::core::base::entity::message::Location,
        destination: crate::core::base::entity::message::Location,
        priority: crate::core::base::entity::message::MessagePriority,
        config: Option<QueueItemConfig>,
    ) -> QueueItem<T>
    where
        T: Clone + Serialize + DeserializeOwned,
    {
        let message = crate::core::base::entity::message::Message::with_priority(
            source, destination, payload, priority
        );
        QueueItem::new(queue_name, message, config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::base::entity::message::Location;

    #[test]
    fn test_queue_item_factory() {
        let factory = DefaultQueueItemFactory;
        
        let item = factory.create_item(
            "test-queue".to_string(),
            "test payload".to_string(),
            Location::service("test-service"),
            Location::system("queue-system"),
            None,
        );
        
        assert_eq!(item.queue_name, "test-queue");
        assert_eq!(item.payload(), &"test payload".to_string());
    }

    #[test]
    fn test_priority_queue_item_factory() {
        let factory = DefaultQueueItemFactory;
        
        let item = factory.create_priority_item(
            "priority-queue".to_string(),
            "urgent task".to_string(),
            Location::service("test-service"),
            Location::system("queue-system"),
            crate::core::base::entity::message::MessagePriority::Critical,
            None,
        );
        
        assert_eq!(item.message.priority, crate::core::base::entity::message::MessagePriority::Critical);
    }
}