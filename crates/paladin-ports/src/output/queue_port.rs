//! # Queue Port - Async Task Queue and Job Processing Interface
//!
//! This module defines the port trait for asynchronous task queue operations, enabling
//! distributed job processing, work distribution, and background task execution across
//! Paladin and Battalion operations.
//!
//! ## Purpose
//!
//! The Queue port provides a standardized interface for:
//! - **Task Distribution**: Enqueue work items for async processing
//! - **Job Processing**: Dequeue and process items with worker coordination
//! - **Priority Management**: Handle items based on urgency (Critical → High → Normal → Low)
//! - **Retry Handling**: Automatic retry with exponential backoff for failed items
//! - **Queue Lifecycle**: Create, pause, resume, and monitor multiple queues
//!
//! Following hexagonal architecture, this trait abstracts queue operations from their
//! implementations (in-memory, Redis, RabbitMQ, AWS SQS).
//!
//! ## Hexagonal Architecture Context
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │               Application Layer                      │
//! │  ┌──────────────────────────────────────────────┐  │
//! │  │  PaladinExecutionService                      │  │
//! │  │    - Queues Paladin execution tasks           │  │
//! │  │  BattalionServices                            │  │
//! │  │    - Distributes sub-tasks to workers         │  │
//! │  │  ContentIngestionService                      │  │
//! │  │    - Queues documents for processing          │  │
//! │  └──────────────────────────────────────────────┘  │
//! │                         │                            │
//! │                         ▼                            │
//! │  ┌──────────────────────────────────────────────┐  │
//! │  │  QueuePort (this module)                      │  │
//! │  │    - Task queue interface                     │  │
//! │  └──────────────────────────────────────────────┘  │
//! └─────────────────────────────────────────────────────┘
//!                          │
//!                          ▼
//! ┌─────────────────────────────────────────────────────┐
//! │            Infrastructure Layer                      │
//! │  ┌──────────────────────────────────────────────┐  │
//! │  │  InMemoryQueue (Local processing)            │  │
//! │  │  RedisQueueAdapter (Distributed)             │  │
//! │  │  RabbitMQAdapter (Message broker)            │  │
//! │  │  SqsAdapter (AWS SQS)                        │  │
//! │  └──────────────────────────────────────────────┘  │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! ## Thread Safety
//!
//! All implementations must be `Send + Sync`:
//! - **Send**: Queue operations may happen across threads
//! - **Sync**: Multiple workers access the queue concurrently
//! - Implementations must handle concurrent enqueue/dequeue safely
//!
//! ## Error Handling
//!
//! Queue operations can fail for several reasons:
//! - **Queue Not Found**: Requested queue doesn't exist
//! - **Queue Full**: Maximum capacity reached (configurable)
//! - **Item Not Found**: Item ID doesn't exist or already processed
//! - **Serialization Error**: Item payload cannot be serialized/deserialized
//! - **Operation Failed**: Backend-specific errors (network, Redis down, etc.)
//!
//! All errors are represented via [`QueueError`]
//! with detailed context for debugging and recovery.
//!
//! ## Common Use Cases
//!
//! ### 1. Async Paladin Execution
//!
//! ```rust,ignore
//! use paladin_ports::output::queue_port::QueuePort;
//! use paladin_core::platform::container::queue_item::{QueueItem, QueueItemConfig};
//! use paladin_core::base::entity::message::Message;
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! struct PaladinTask {
//!     paladin_id: String,
//!     input: String,
//! }
//!
//! async fn queue_paladin_execution(
//!     queue: &dyn QueuePort,
//!     task: PaladinTask,
//! ) -> Result<(), Box<dyn std::error::Error>> {
//!     // Create message with task payload
//!     let message = Message::new(
//!         task,
//!         "paladin-service".to_string(),
//!         paladin_core::base::entity::message::Location::Local,
//!     );
//!
//!     // Create queue item with retry config
//!     let config = QueueItemConfig {
//!         max_retries: 3,
//!         timeout_seconds: 300,
//!         ..Default::default()
//!     };
//!
//!     let item = QueueItem::new(
//!         "paladin-executions".to_string(),
//!         message,
//!         Some(config),
//!     );
//!
//!     // Enqueue for async processing
//!     let item_id = queue.enqueue("paladin-executions", item).await?;
//!     println!("Queued Paladin task: {}", item_id);
//!
//!     Ok(())
//! }
//! ```
//!
//! ### 2. Worker Processing Loop
//!
//! ```rust,ignore
//! use paladin_ports::output::queue_port::QueuePort;
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! struct DocumentTask {
//!     document_id: String,
//!     operation: String,
//! }
//!
//! async fn worker_process_loop(
//!     queue: &dyn QueuePort,
//!     worker_id: String,
//! ) -> Result<(), Box<dyn std::error::Error>> {
//!     loop {
//!         // Dequeue next item
//!         if let Some(item) = queue.dequeue("document-processing").await? {
//!             let item_id = item.action.id;
//!
//!             // Mark as processing
//!             queue.start_processing("document-processing", item_id, worker_id.clone()).await?;
//!
//!             // Process the task
//!             match process_document(&item).await {
//!                 Ok(result) => {
//!                     // Mark as complete
//!                     queue.complete_processing(
//!                         "document-processing",
//!                         item_id,
//!                         Some(result),
//!                     ).await?;
//!                     println!("Completed task: {}", item_id);
//!                 }
//!                 Err(e) => {
//!                     // Mark as failed (will retry if configured)
//!                     let should_retry = queue.fail_processing(
//!                         "document-processing",
//!                         item_id,
//!                         e.to_string(),
//!                     ).await?;
//!
//!                     if should_retry {
//!                         println!("Task {} will retry", item_id);
//!                     } else {
//!                         println!("Task {} failed permanently", item_id);
//!                     }
//!                 }
//!             }
//!         } else {
//!             // No items available, wait before polling again
//!             tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
//!         }
//!     }
//! }
//!
//! async fn process_document(
//!     item: &paladin_core::platform::container::queue_item::QueueItem<serde_json::Value>,
//! ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
//!     // Simulate document processing
//!     Ok(serde_json::json!({"status": "processed"}))
//! }
//! ```
//!
//! ### 3. Priority-Based Processing
//!
//! ```rust,ignore
//! use paladin_ports::output::queue_port::QueuePort;
//! use paladin_core::platform::container::queue_item::{QueueItem, QueueItemConfig};
//! use paladin_core::base::entity::message::{Message, MessagePriority};
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! struct AlertTask {
//!     alert_type: String,
//!     severity: String,
//! }
//!
//! async fn queue_critical_alert(
//!     queue: &dyn QueuePort,
//!     alert: AlertTask,
//! ) -> Result<(), Box<dyn std::error::Error>> {
//!     // Create message with Critical priority
//!     let mut message = Message::new(
//!         alert,
//!         "alert-service".to_string(),
//!         paladin_core::base::entity::message::Location::Local,
//!     );
//!     message.priority = MessagePriority::Critical;
//!
//!     let item = QueueItem::new(
//!         "alerts".to_string(),
//!         message,
//!         Some(QueueItemConfig::default()),
//!     );
//!
//!     // Will be processed before Normal/Low priority items
//!     queue.enqueue("alerts", item).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ### 4. Queue Monitoring and Stats
//!
//! ```rust,ignore
//! use paladin_ports::output::queue_port::QueuePort;
//!
//! async fn monitor_queue_health(
//!     queue: &dyn QueuePort,
//! ) -> Result<(), Box<dyn std::error::Error>> {
//!     // Get all queue names
//!     let queues = queue.list_queues().await;
//!
//!     for queue_name in queues {
//!         let stats = queue.get_queue_stats(&queue_name).await?;
//!
//!         println!("Queue: {}", stats.name);
//!         println!("  Total items: {}", stats.total_items);
//!         println!("  Pending: {}", stats.pending_items);
//!         println!("  Processing: {}", stats.processing_items);
//!         println!("  Completed: {}", stats.completed_items);
//!         println!("  Failed: {}", stats.failed_items);
//!         println!("  Throughput: {:.2}/min", stats.throughput_per_minute);
//!
//!         if let Some(avg_time) = stats.average_processing_time_ms {
//!             println!("  Avg processing time: {}ms", avg_time);
//!         }
//!
//!         // Alert if queue is growing too large
//!         if stats.pending_items > 1000 {
//!             println!("⚠️  Queue {} has {} pending items!", queue_name, stats.pending_items);
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Implementation Notes
//!
//! ### Queue Backend Selection
//!
//! Choose based on deployment requirements:
//!
//! - **InMemoryQueue**: Best for development, single-instance deployments
//!   - Fastest performance
//!   - No external dependencies
//!   - Lost on process restart
//!
//! - **RedisQueue**: Distributed, persistent, high performance
//!   - Sub-millisecond latency
//!   - Supports clustering
//!   - Survives process restarts
//!
//! - **RabbitMQ**: Message broker with advanced routing
//!   - Guaranteed delivery
//!   - Complex routing patterns
//!   - Higher latency
//!
//! - **AWS SQS**: Fully managed, infinite scale
//!   - No infrastructure management
//!   - Higher cost
//!   - ~1 second minimum latency
//!
//! ### Performance Optimization
//!
//! 1. **Batch Operations**: Use `enqueue_batch()` / `dequeue_batch()` for bulk work
//! 2. **Priority Queues**: Enable `priority_based: true` only when needed
//! 3. **Worker Scaling**: Run multiple workers per queue for parallelism
//! 4. **Polling Strategy**: Use exponential backoff when queue is empty
//! 5. **Cleanup**: Run `cleanup_expired()` periodically to reclaim memory
//!
//! ### Retry Strategy
//!
//! Configure retry behavior via `QueueItemConfig`:
//!
//! ```rust,ignore
//! let config = QueueItemConfig {
//!     max_retries: 3,              // Number of retry attempts
//!     retry_delay_seconds: 60,     // Initial delay between retries
//!     retry_backoff_multiplier: 2.0, // Exponential backoff (60s → 120s → 240s)
//!     timeout_seconds: 300,        // Max processing time per attempt
//!     ..Default::default()
//! };
//! ```
//!
//! ### Best Practices
//!
//! 1. **Idempotent Operations**: Design tasks to be safely retried
//! 2. **Timeouts**: Set reasonable `timeout_seconds` for long-running tasks
//! 3. **Dead Letter Queues**: Move permanently failed items to DLQ for investigation
//! 4. **Monitoring**: Track `throughput_per_minute` and `pending_items` metrics
//! 5. **Graceful Shutdown**: Finish processing current items before stopping workers
//! 6. **Queue Naming**: Use descriptive names (e.g., `paladin-executions`, `document-processing`)
//!
//! ## Common Pitfalls
//!
//! - Not handling `QueueEmpty` gracefully (use polling loop with backoff)
//! - Forgetting to call `start_processing()` before processing (breaks visibility timeout)
//! - Not calling `complete_processing()` or `fail_processing()` (items stay "stuck")
//! - Setting `max_retries` too high (wastes resources on truly broken tasks)
//! - Using priority queues when not needed (adds overhead)
//! - Not monitoring queue depth (can lead to memory exhaustion)
//!
//! ## Related Modules
//!
//! - [`QueueItem`] - Queue item structure
//! - [`QueueItemConfig`] - Item configuration
//! - [`QueueConfig`] - Queue configuration
//! - [`QueueStats`] - Queue statistics
//! - [`QueueError`] - Error types
//! - [`Message`](paladin_core::base::entity::message::Message) - Message wrapper
//! - [`MessagePriority`] - Priority levels
//!
//! ## See Also
//!
//! - `examples/async_paladin_queue.rs` - Async execution example
//! - `examples/worker_pool.rs` - Worker processing example
//! - `examples/priority_queue.rs` - Priority-based processing example

use async_trait::async_trait;
use paladin_core::base::entity::message::{Location, MessagePriority};
use paladin_core::platform::container::queue_config::QueueConfig;
use paladin_core::platform::container::queue_item::{QueueItem, QueueItemConfig, QueueItemSummary};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;

/// Queue service errors
#[derive(Debug, Error)]
pub enum QueueError {
    /// Queue not found
    #[error("Queue not found: {0}")]
    QueueNotFound(String),
    /// Queue item not found
    #[error("Queue item not found: {0}")]
    ItemNotFound(Uuid),
    /// Queue is full
    #[error("Queue is full: {queue_name} (capacity: {capacity})")]
    QueueFull {
        /// Name of the queue that is full.
        queue_name: String,
        /// Maximum capacity of the queue.
        capacity: usize,
    },
    /// Queue is empty
    #[error("Queue is empty: {0}")]
    QueueEmpty(String),
    /// Invalid queue configuration
    #[error("Invalid queue configuration: {0}")]
    InvalidConfiguration(String),
    /// Queue operation failed
    #[error("Queue operation failed: {0}")]
    OperationFailed(String),
    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Queue statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    /// Queue name
    pub name: String,
    /// Total items ever enqueued
    pub total_items: usize,
    /// Items waiting to be processed
    pub pending_items: usize,
    /// Items currently being processed
    pub processing_items: usize,
    /// Items successfully completed
    pub completed_items: usize,
    /// Items that failed processing
    pub failed_items: usize,
    /// Items abandoned due to timeout
    pub abandoned_items: usize,
    /// Age of oldest pending item in seconds
    pub oldest_item_age_seconds: Option<i64>,
    /// Average processing time in milliseconds
    pub average_processing_time_ms: Option<u64>,
    /// Queue throughput per minute
    pub throughput_per_minute: f64,
}
use std::collections::HashMap;
use uuid::Uuid;

/// Port trait for core queue operations.
///
/// Provides the primary interface for async task queue management with support for:
/// - Queue lifecycle (create, delete)
/// - Item operations (enqueue, dequeue)
/// - Processing state management (start, complete, fail)
/// - Monitoring (stats, health)
///
/// # Capabilities
///
/// - **Queue Management**: Create/delete queues with custom configurations
/// - **Item Enqueue**: Add tasks to queues with serializable payloads
/// - **Item Dequeue**: Retrieve next item for processing (FIFO or priority-based)
/// - **Processing Lifecycle**: Track item states (pending → processing → complete/failed)
/// - **Automatic Retry**: Failed items retry based on `QueueItemConfig`
/// - **Statistics**: Monitor queue depth, throughput, and processing times
/// - **Health Check**: Verify queue backend connectivity
///
/// # Thread Safety
///
/// All implementations must be `Send + Sync` to support:
/// - Concurrent enqueue operations from multiple threads
/// - Safe dequeue by multiple workers
/// - Thread-safe state transitions
///
/// # Implementation Requirements
///
/// Implementations should:
/// 1. Use atomic operations for state transitions (pending → processing)
/// 2. Implement visibility timeout for processing items
/// 3. Support automatic retry with exponential backoff
/// 4. Handle serialization/deserialization gracefully
/// 5. Return `QueueEmpty` (not an error) when queue has no items
/// 6. Preserve completed/failed items based on `QueueConfig`
///
/// # Examples
///
/// ## Basic Queue Operations
///
/// ```rust,ignore
/// use paladin_ports::output::queue_port::QueuePort;
/// use paladin_core::platform::container::queue_config::QueueConfig;
/// use paladin_core::platform::container::queue_item::{QueueItem, QueueItemConfig};
/// use paladin_core::base::entity::message::Message;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct MyTask {
///     task_id: String,
///     data: String,
/// }
///
/// async fn basic_queue_usage(
///     queue: &dyn QueuePort,
/// ) -> Result<(), Box<dyn std::error::Error>> {
///     // Create a new queue
///     let config = QueueConfig {
///         max_capacity: 10000,
///         priority_based: true,
///         ..Default::default()
///     };
///     queue.create_queue("my-tasks".to_string(), Some(config)).await?;
///
///     // Enqueue a task
///     let task = MyTask {
///         task_id: "task-001".to_string(),
///         data: "process this".to_string(),
///     };
///
///     let message = Message::new(
///         task,
///         "my-service".to_string(),
///         paladin_core::base::entity::message::Location::Local,
///     );
///
///     let item = QueueItem::new(
///         "my-tasks".to_string(),
///         message,
///         Some(QueueItemConfig::default()),
///     );
///
///     let item_id = queue.enqueue("my-tasks", item).await?;
///     println!("Enqueued task: {}", item_id);
///
///     // Check queue stats
///     let stats = queue.get_queue_stats("my-tasks").await?;
///     println!("Pending items: {}", stats.pending_items);
///
///     Ok(())
///     }
/// ```
///
/// ## Worker Processing with Retry
///
/// ```rust,ignore
/// use paladin_ports::output::queue_port::QueuePort;
///
/// async fn worker_with_retry(
///     queue: &dyn QueuePort,
///     worker_id: String,
/// ) -> Result<(), Box<dyn std::error::Error>> {
///     loop {
///         // Dequeue next item
///         let item = match queue.dequeue("tasks").await? {
///             Some(i) => i,
///             None => {
///                 // Queue empty, wait before polling again
///                 tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
///                 continue;
///             }
///         };
///
///         let item_id = item.action.id;
///
///         // Mark as processing
///         queue.start_processing("tasks", item_id, worker_id.clone()).await?;
///
///         // Process (simulate work)
///         match process_task(&item).await {
///             Ok(result) => {
///                 queue.complete_processing("tasks", item_id, Some(result)).await?;
///                 println!("✅ Task {} completed", item_id);
///             }
///             Err(e) => {
///                 let will_retry = queue.fail_processing("tasks", item_id, e.to_string()).await?;
///                 if will_retry {
///                     println!("🔄 Task {} will retry", item_id);
///                 } else {
///                     println!("❌ Task {} failed permanently", item_id);
///                 }
///             }
///         }
///     }
/// }
///
/// async fn process_task(
///     item: &paladin_core::platform::container::queue_item::QueueItem<serde_json::Value>,
/// ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
///     // Simulate processing
///     Ok(serde_json::json!({"status": "done"}))
/// }
/// ```
#[async_trait]
pub trait QueuePort: Send + Sync {
    /// Create a new queue with optional configuration
    async fn create_queue(
        &self,
        name: String,
        config: Option<QueueConfig>,
    ) -> Result<(), QueueError>;

    /// Delete an existing queue
    async fn delete_queue(&self, name: &str) -> Result<(), QueueError>;

    /// Enqueue an item into a specific queue
    async fn enqueue<T>(&self, queue_name: &str, item: QueueItem<T>) -> Result<Uuid, QueueError>
    where
        T: Serialize + Clone + for<'de> serde::Deserialize<'de> + Send + Sync;

    /// Dequeue an item from a specific queue
    async fn dequeue(
        &self,
        queue_name: &str,
    ) -> Result<Option<QueueItem<serde_json::Value>>, QueueError>;

    /// Start processing an item (mark as in-progress)
    async fn start_processing(
        &self,
        queue_name: &str,
        item_id: Uuid,
        worker_id: String,
    ) -> Result<(), QueueError>;

    /// Complete processing an item successfully
    async fn complete_processing(
        &self,
        queue_name: &str,
        item_id: Uuid,
        result_data: Option<serde_json::Value>,
    ) -> Result<(), QueueError>;

    /// Fail processing an item (with potential retry)
    async fn fail_processing(
        &self,
        queue_name: &str,
        item_id: Uuid,
        error: String,
    ) -> Result<bool, QueueError>;

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
///
/// # Examples
///
/// Generic methods (`enqueue_typed`, `process_with_handler`) make this trait not
/// object-safe, so implementors are reached through a generic bound rather than `dyn`.
///
/// ```rust
/// use paladin_ports::output::queue_port::{TypedQueuePort, QueueError};
/// use paladin_core::platform::container::queue_item::QueueItem;
/// use paladin_core::base::entity::message::{Location, Message};
///
/// async fn enqueue_typed_task<S>(queue: &S, task: String) -> Result<(), QueueError>
/// where
///     S: TypedQueuePort<String>,
/// {
///     let message = Message::new(Location::service("caller"), Location::system("queue"), task);
///     let item = QueueItem::new("typed-tasks".to_string(), message, None);
///     queue.enqueue_typed("typed-tasks", item).await?;
///     Ok(())
/// }
/// ```
#[async_trait]
pub trait TypedQueuePort<T>: Send + Sync
where
    T: Serialize + DeserializeOwned + Send + Sync + Clone,
{
    /// Enqueue a strongly-typed item
    async fn enqueue_typed(&self, queue_name: &str, item: QueueItem<T>)
    -> Result<Uuid, QueueError>;

    /// Dequeue a strongly-typed item
    async fn dequeue_typed(&self, queue_name: &str) -> Result<Option<QueueItem<T>>, QueueError>;

    /// Process items with a typed handler
    async fn process_with_handler<F, Fut>(
        &self,
        queue_name: &str,
        handler: F,
    ) -> Result<(), QueueError>
    where
        F: Fn(QueueItem<T>) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<Option<serde_json::Value>, String>> + Send;
}

/// Batch queue operations port
///
/// # Examples
///
/// Generic methods make this trait not object-safe, so implementors are reached through a
/// generic bound rather than `dyn`.
///
/// ```rust
/// use paladin_ports::output::queue_port::{BatchQueuePort, QueueError};
/// use paladin_core::platform::container::queue_item::QueueItem;
/// use paladin_core::base::entity::message::{Location, Message};
/// use uuid::Uuid;
///
/// async fn enqueue_report_batch<S>(
///     queue: &S,
///     reports: Vec<String>,
/// ) -> Result<Vec<Uuid>, QueueError>
/// where
///     S: BatchQueuePort,
/// {
///     let items: Vec<_> = reports
///         .into_iter()
///         .map(|report| {
///             let message =
///                 Message::new(Location::service("caller"), Location::system("queue"), report);
///             QueueItem::new("reports".to_string(), message, None)
///         })
///         .collect();
///
///     queue.enqueue_batch("reports", items).await
/// }
/// ```
#[async_trait]
pub trait BatchQueuePort: Send + Sync {
    /// Enqueue multiple items at once
    async fn enqueue_batch<T>(
        &self,
        queue_name: &str,
        items: Vec<QueueItem<T>>,
    ) -> Result<Vec<Uuid>, QueueError>
    where
        T: Serialize + Clone + for<'de> serde::Deserialize<'de> + Send + Sync;

    /// Enqueue with explicit priority override
    async fn enqueue_with_priority<T>(
        &self,
        queue_name: &str,
        item: QueueItem<T>,
        priority: MessagePriority,
    ) -> Result<Uuid, QueueError>
    where
        T: Serialize + Clone + for<'de> serde::Deserialize<'de> + Send + Sync;

    /// Dequeue multiple items at once
    async fn dequeue_batch(
        &self,
        queue_name: &str,
        count: usize,
    ) -> Result<Vec<QueueItem<serde_json::Value>>, QueueError>;

    /// Get summaries for multiple items
    async fn get_item_summaries(
        &self,
        queue_name: &str,
        item_ids: Vec<Uuid>,
    ) -> Result<Vec<QueueItemSummary>, QueueError>;
}

/// Priority queue operations port
///
/// # Examples
///
/// The generic `enqueue_with_priority` method makes this trait not object-safe, so
/// implementors are reached through a generic bound rather than `dyn`.
///
/// ```rust
/// use paladin_ports::output::queue_port::{PriorityQueuePort, QueueError};
/// use paladin_core::platform::container::queue_item::QueueItem;
/// use paladin_core::base::entity::message::{Location, Message, MessagePriority};
///
/// async fn enqueue_urgent_task<S>(queue: &S, task: String) -> Result<(), QueueError>
/// where
///     S: PriorityQueuePort,
/// {
///     let message = Message::new(Location::service("caller"), Location::system("queue"), task);
///     let item = QueueItem::new("tasks".to_string(), message, None);
///
///     queue
///         .enqueue_with_priority("tasks", item, MessagePriority::Critical)
///         .await?;
///     Ok(())
/// }
/// ```
#[async_trait]
pub trait PriorityQueuePort: Send + Sync {
    /// Enqueue with explicit priority override
    async fn enqueue_with_priority<T>(
        &self,
        queue_name: &str,
        item: QueueItem<T>,
        priority: MessagePriority,
    ) -> Result<Uuid, QueueError>
    where
        T: Serialize + Clone + for<'de> serde::Deserialize<'de> + Send + Sync;

    /// Dequeue highest priority item
    async fn dequeue_highest_priority(
        &self,
        queue_name: &str,
    ) -> Result<Option<QueueItem<serde_json::Value>>, QueueError>;

    /// Get items by priority level
    async fn get_items_by_priority(
        &self,
        queue_name: &str,
        priority: MessagePriority,
    ) -> Result<Vec<QueueItemSummary>, QueueError>;
}

/// Monitoring and management port for queue operations
///
/// # Examples
///
/// ```rust
/// use paladin_ports::output::queue_port::{QueueManagementPort, QueueError};
///
/// async fn pause_and_purge(queue: &dyn QueueManagementPort) -> Result<usize, QueueError> {
///     queue.pause_queue("nightly-jobs").await?;
///     let purged = queue.purge_failed("nightly-jobs").await?;
///     queue.resume_queue("nightly-jobs").await?;
///     Ok(purged)
/// }
/// ```
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
    async fn get_item_details(
        &self,
        queue_name: &str,
        item_id: Uuid,
    ) -> Result<QueueItem<serde_json::Value>, QueueError>;

    /// Purge completed items from a queue
    async fn purge_completed(&self, queue_name: &str) -> Result<usize, QueueError>;

    /// Purge failed items from a queue
    async fn purge_failed(&self, queue_name: &str) -> Result<usize, QueueError>;

    /// Get queue configuration
    async fn get_queue_config(&self, queue_name: &str) -> Result<QueueConfig, QueueError>;

    /// Update queue configuration
    async fn update_queue_config(
        &self,
        queue_name: &str,
        config: QueueConfig,
    ) -> Result<(), QueueError>;
}

/// Combined queue port that includes all queue operations
/// This is the main port that application services should depend on
///
/// # Examples
///
/// This combinator trait declares no methods of its own — it is a bound that grants access to
/// every constituent port's methods at once. [`QueuePort`], [`BatchQueuePort`],
/// [`PriorityQueuePort`] and [`QueueManagementPort`] each document their own call pattern above.
/// Because two of those supertraits have generic methods, `FullQueuePort` is also not
/// object-safe, so implementors are reached through a generic bound rather than `dyn`.
///
/// ```rust
/// use paladin_ports::output::queue_port::{FullQueuePort, QueueError};
///
/// async fn queue_health_summary<S: FullQueuePort>(queue: &S) -> Result<bool, QueueError> {
///     // From `QueuePort` (base capability).
///     let healthy = queue.health_check().await?;
///     // From `QueueManagementPort` (combined capability).
///     queue.pause_queue("maintenance").await?;
///     Ok(healthy)
/// }
/// ```
pub trait FullQueuePort:
    QueuePort + BatchQueuePort + PriorityQueuePort + QueueManagementPort + Send + Sync
{
}

/// Helper trait for creating queue items
pub trait QueueItemFactory {
    /// Create a queue item from a message
    fn create_item<T>(
        &self,
        queue_name: String,
        payload: T,
        source: Location,
        destination: Location,
        config: Option<QueueItemConfig>,
    ) -> QueueItem<T>
    where
        T: Clone + Serialize + DeserializeOwned;

    /// Create a priority queue item
    fn create_priority_item<T>(
        &self,
        queue_name: String,
        payload: T,
        source: Location,
        destination: Location,
        priority: paladin_core::base::entity::message::MessagePriority,
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
        source: Location,
        destination: Location,
        config: Option<QueueItemConfig>,
    ) -> QueueItem<T>
    where
        T: Clone + Serialize + DeserializeOwned,
    {
        let message =
            paladin_core::base::entity::message::Message::new(source, destination, payload);
        QueueItem::new(queue_name, message, config)
    }

    fn create_priority_item<T>(
        &self,
        queue_name: String,
        payload: T,
        source: Location,
        destination: Location,
        priority: paladin_core::base::entity::message::MessagePriority,
        config: Option<QueueItemConfig>,
    ) -> QueueItem<T>
    where
        T: Clone + Serialize + DeserializeOwned,
    {
        let message = paladin_core::base::entity::message::Message::with_priority(
            source,
            destination,
            payload,
            priority,
        );
        QueueItem::new(queue_name, message, config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Location;

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
            paladin_core::base::entity::message::MessagePriority::Critical,
            None,
        );

        assert_eq!(
            item.message.priority,
            paladin_core::base::entity::message::MessagePriority::Critical
        );
    }
}
