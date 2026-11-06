use async_trait::async_trait;
use chrono::Utc;
use redis::{AsyncCommands, Client, aio::ConnectionManager};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use paladin_core::base::entity::message::{Location, MessagePriority};
use paladin_core::platform::container::log::{LogEntry, LogLevel, LogMessage};
use paladin_core::platform::container::queue_config::QueueConfig;
use paladin_core::platform::container::queue_item::{QueueItem, QueueItemStatus, QueueItemSummary};
use paladin_ports::output::log_port::LogPort;
use paladin_ports::output::queue_port::{
    BatchQueuePort, FullQueuePort, PriorityQueuePort, QueueManagementPort, QueuePort,
};
use paladin_ports::output::queue_port::{QueueError, QueueStats};

/// A secret as `Debug` prints it: `<redacted>`, or `""` when empty.
///
/// A module-private copy of the three-line rule in `paladin_llm::redaction::RedactedKey` (and
/// paladin-ai's `config::redaction::RedactedSecret`), both `pub(crate)` there. Neither is
/// reachable from this crate, and a shared helper here would need a `lib.rs` change, so this
/// file carries its own (the paladin-notifications DF-07 precedent; R-95).
struct RedactedSecret<'a>(&'a str);

impl std::fmt::Debug for RedactedSecret<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_empty() {
            f.write_str("\"\"")
        } else {
            f.write_str("<redacted>")
        }
    }
}

/// Configuration for Redis connection
///
/// `Debug` is implemented by hand: it prints `redis_password` as `<redacted>` when one is set
/// (the derived `Debug` printed it), and `Serialize` omits the password altogether. Loading is
/// unchanged, so the password is still optional when a value is deserialised (R-95).
#[doc(hidden)]
#[derive(Clone, Serialize, Deserialize)]
pub struct RedisQueueConfig {
    pub redis_host: String,
    pub redis_port: u16,
    /// Never written by `Serialize` (`skip_serializing`, no serde default) and printed as
    /// `<redacted>` by `Debug`.
    #[serde(skip_serializing)]
    pub redis_password: Option<String>,
    pub redis_db: u8,
    pub connection_timeout: u64,
    pub key_prefix: String,
    pub max_retries: u32,
}

impl std::fmt::Debug for RedisQueueConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisQueueConfig")
            .field("redis_host", &self.redis_host)
            .field("redis_port", &self.redis_port)
            .field(
                "redis_password",
                &self.redis_password.as_deref().map(RedactedSecret),
            )
            .field("redis_db", &self.redis_db)
            .field("connection_timeout", &self.connection_timeout)
            .field("key_prefix", &self.key_prefix)
            .field("max_retries", &self.max_retries)
            .finish()
    }
}

impl Default for RedisQueueConfig {
    fn default() -> Self {
        Self {
            redis_host: "localhost".to_string(),
            redis_port: 6379,
            redis_password: None,
            redis_db: 0,
            connection_timeout: 30,
            key_prefix: "paladin:queue".to_string(),
            max_retries: 3,
        }
    }
}

/// Generate the Redis key for a queue's main (FIFO) list.
fn queue_key(config: &RedisQueueConfig, queue_name: &str) -> String {
    format!("{}:queue:{}", config.key_prefix, queue_name)
}

/// Generate the Redis key for a queue's priority-specific list.
fn priority_queue_key(config: &RedisQueueConfig, queue: &str, priority: MessagePriority) -> String {
    let priority_str = match priority {
        MessagePriority::Critical => "critical",
        MessagePriority::High => "high",
        MessagePriority::Normal => "normal",
        MessagePriority::Low => "low",
    };
    format!("{}:{}:{}", config.key_prefix, queue, priority_str)
}

/// Generate the Redis key for a queue's metadata hash.
fn queue_meta_key(config: &RedisQueueConfig, queue_name: &str) -> String {
    format!("{}:meta:{}", config.key_prefix, queue_name)
}

/// Generate the Redis key for a queue's in-progress items hash.
fn processing_key(config: &RedisQueueConfig, queue_name: &str) -> String {
    format!("{}:processing:{}", config.key_prefix, queue_name)
}

/// Generate the Redis key for a queue's completed items hash.
fn completed_key(config: &RedisQueueConfig, queue_name: &str) -> String {
    format!("{}:completed:{}", config.key_prefix, queue_name)
}

/// Generate the Redis key for a queue's failed items hash.
fn failed_key(config: &RedisQueueConfig, queue_name: &str) -> String {
    format!("{}:failed:{}", config.key_prefix, queue_name)
}

/// Serialize a queue item to JSON for Redis storage, stamping the target queue name
/// into the serialized payload.
fn serialize_item<T>(item: &QueueItem<T>, queue_name: &str) -> Result<String, QueueError>
where
    T: Serialize,
{
    // Create a JSON value from the item first
    let mut item_json = serde_json::to_value(item).map_err(|e| {
        QueueError::SerializationError(format!("Failed to convert item to JSON: {}", e))
    })?;

    // Update the queue_name in the JSON
    if let Some(obj) = item_json.as_object_mut() {
        obj.insert(
            "queue_name".to_string(),
            serde_json::Value::String(queue_name.to_string()),
        );
    }

    let serialized = serde_json::to_string(&item_json)
        .map_err(|e| QueueError::SerializationError(format!("Failed to serialize item: {}", e)))?;

    Ok(serialized)
}

/// Deserialize a queue item from Redis storage.
fn deserialize_item(data: &str) -> Result<QueueItem<serde_json::Value>, QueueError> {
    serde_json::from_str(data)
        .map_err(|e| QueueError::SerializationError(format!("Failed to deserialize item: {}", e)))
}

/// Redis queue adapter that implements all queue port traits
#[doc(hidden)]
pub struct RedisQueueAdapter {
    #[allow(dead_code)]
    client: Client,
    conn: Arc<RwLock<ConnectionManager>>,
    config: RedisQueueConfig,
    log_port: Option<Arc<dyn LogPort>>,
    queue_configs: Arc<RwLock<HashMap<String, QueueConfig>>>,
}

impl RedisQueueAdapter {
    /// Create a new Redis queue adapter
    pub async fn new(
        config: RedisQueueConfig,
        log_port: Option<Arc<dyn LogPort>>,
    ) -> Result<Self, QueueError> {
        let connection_url = if let Some(password) = &config.redis_password {
            format!(
                "redis://:{}@{}:{}/{}",
                password, config.redis_host, config.redis_port, config.redis_db
            )
        } else {
            format!(
                "redis://{}:{}/{}",
                config.redis_host, config.redis_port, config.redis_db
            )
        };

        let client = Client::open(connection_url).map_err(|e| {
            QueueError::OperationFailed(format!("Failed to create Redis client: {}", e))
        })?;

        let conn = ConnectionManager::new(client.clone()).await.map_err(|e| {
            QueueError::OperationFailed(format!("Failed to connect to Redis: {}", e))
        })?;

        Ok(Self {
            client,
            conn: Arc::new(RwLock::new(conn)),
            config,
            log_port,
            queue_configs: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Log operation to LogPort if available
    async fn log_operation(&self, level: LogLevel, message: String) {
        if let Some(log_port) = &self.log_port {
            let entry = LogEntry {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                message: LogMessage::new(level, message.clone()),
                source: Location::service("redis-queue-adapter"),
                destination: Location::system("log"),
                correlation_id: None,
                priority: MessagePriority::Normal,
            };

            if let Err(e) = log_port.write_entry(entry).await {
                eprintln!("Failed to log operation: {} - Error: {}", message, e);
            }
        }
    }

    /// Check if queue exists
    async fn queue_exists(&self, queue_name: &str) -> Result<bool, QueueError> {
        let mut conn = self.conn.write().await;
        let meta_key = queue_meta_key(&self.config, queue_name);

        let exists: bool = conn.exists(&meta_key).await.map_err(|e| {
            QueueError::OperationFailed(format!("Failed to check queue existence: {}", e))
        })?;

        Ok(exists)
    }

    /// Get all priority levels for priority-based dequeue
    fn get_priority_levels() -> Vec<MessagePriority> {
        vec![
            MessagePriority::Critical,
            MessagePriority::High,
            MessagePriority::Normal,
            MessagePriority::Low,
        ]
    }
}

#[async_trait]
impl QueuePort for RedisQueueAdapter {
    async fn create_queue(
        &self,
        queue_name: String,
        config: Option<QueueConfig>,
    ) -> Result<(), QueueError> {
        // Check if queue already exists
        if self.queue_exists(&queue_name).await? {
            return Err(QueueError::OperationFailed(
                "Queue already exists".to_string(),
            ));
        }

        // Use provided config or create a simple default (NOT priority-based)
        let queue_config = config.unwrap_or_else(|| QueueConfig {
            max_capacity: 10000,
            priority_based: false,
            preserve_completed: true,
            preserve_failed: true,
            cleanup_interval_seconds: 3600,
            default_item_config: Default::default(),
        });

        let mut conn = self.conn.write().await;

        // Create the metadata key that queue_exists() checks for
        let meta_key = queue_meta_key(&self.config, &queue_name);
        let config_json = serde_json::to_string(&queue_config).map_err(|e| {
            QueueError::SerializationError(format!("Failed to serialize config: {}", e))
        })?;

        // Store queue metadata (this is what queue_exists() looks for)
        let _: () = conn
            .hset(&meta_key, "config", &config_json)
            .await
            .map_err(|e| {
                QueueError::OperationFailed(format!("Failed to create queue metadata: {}", e))
            })?;

        let _: () = conn
            .hset(&meta_key, "created_at", chrono::Utc::now().to_rfc3339())
            .await
            .map_err(|e| {
                QueueError::OperationFailed(format!("Failed to set creation time: {}", e))
            })?;

        // Store the queue configuration in memory
        {
            let mut configs = self.queue_configs.write().await;
            configs.insert(queue_name.clone(), queue_config);
        }

        // Add to queue list (for list_queues())
        let queue_list_key = format!("{}:queues", self.config.key_prefix);
        let _: () = conn.sadd(&queue_list_key, &queue_name).await.map_err(|e| {
            QueueError::OperationFailed(format!("Failed to add queue to list: {}", e))
        })?;

        self.log_operation(LogLevel::Info, format!("Created queue: {}", queue_name))
            .await;

        Ok(())
    }

    async fn delete_queue(&self, name: &str) -> Result<(), QueueError> {
        if !self.queue_exists(name).await? {
            return Err(QueueError::QueueNotFound(name.to_string()));
        }

        let mut conn = self.conn.write().await;

        // Delete all queue-related keys using pipeline for efficiency
        let mut pipe = redis::pipe();
        pipe.del(queue_key(&self.config, name))
            .del(queue_meta_key(&self.config, name))
            .del(processing_key(&self.config, name))
            .del(completed_key(&self.config, name))
            .del(failed_key(&self.config, name));

        // Delete priority queues
        for priority in Self::get_priority_levels() {
            pipe.del(priority_queue_key(&self.config, name, priority));
        }

        let _: () = pipe
            .query_async(&mut *conn)
            .await
            .map_err(|e| QueueError::OperationFailed(format!("Failed to delete queue: {}", e)))?;

        // Remove from queue list - THIS WAS MISSING!
        let queue_list_key = format!("{}:queues", self.config.key_prefix);
        let _: () = conn.srem(&queue_list_key, name).await.map_err(|e| {
            QueueError::OperationFailed(format!("Failed to remove queue from list: {}", e))
        })?;

        // Remove from memory cache
        {
            let mut configs = self.queue_configs.write().await;
            configs.remove(name);
        }

        self.log_operation(LogLevel::Info, format!("Deleted queue: {}", name))
            .await;
        Ok(())
    }

    async fn enqueue<T>(&self, queue_name: &str, item: QueueItem<T>) -> Result<Uuid, QueueError>
    where
        T: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
    {
        if !self.queue_exists(queue_name).await? {
            return Err(QueueError::QueueNotFound(queue_name.to_string()));
        }

        let item_id = item.id();
        let serialized = serialize_item(&item, queue_name)?;
        let mut conn = self.conn.write().await;

        // Determine which queue to use based on priority
        let queue_key = if let Some(config) = self.queue_configs.read().await.get(queue_name) {
            if config.priority_based {
                priority_queue_key(&self.config, queue_name, item.message.priority)
            } else {
                queue_key(&self.config, queue_name)
            }
        } else {
            queue_key(&self.config, queue_name)
        };

        // Add to queue (LPUSH for FIFO with RPOP)
        let _: () = conn
            .lpush(&queue_key, &serialized)
            .await
            .map_err(|e| QueueError::OperationFailed(format!("Failed to enqueue item: {}", e)))?;

        self.log_operation(
            LogLevel::Info,
            format!("Enqueued item {} to queue {}", item_id, queue_name),
        )
        .await;

        Ok(item_id)
    }

    async fn dequeue(
        &self,
        queue_name: &str,
    ) -> Result<Option<QueueItem<serde_json::Value>>, QueueError> {
        if !self.queue_exists(queue_name).await? {
            return Err(QueueError::QueueNotFound(queue_name.to_string()));
        }

        let mut conn = self.conn.write().await;

        // Check if this is a priority-based queue
        let config = self.queue_configs.read().await.get(queue_name).cloned();

        let result = if let Some(config) = config {
            if config.priority_based {
                // For priority-based queues, try dequeuing from priority levels in order
                let mut item_data = None;
                for priority in Self::get_priority_levels() {
                    let queue_key = priority_queue_key(&self.config, queue_name, priority);
                    let data: Option<String> = conn.rpop(&queue_key, None).await.map_err(|e| {
                        QueueError::OperationFailed(format!(
                            "Failed to dequeue from priority queue: {}",
                            e
                        ))
                    })?;
                    if data.is_some() {
                        item_data = data;
                        break;
                    }
                }
                item_data
            } else {
                // Regular FIFO queue
                let queue_key = queue_key(&self.config, queue_name);
                conn.rpop(&queue_key, None).await.map_err(|e| {
                    QueueError::OperationFailed(format!("Failed to dequeue item: {}", e))
                })?
            }
        } else {
            // Fallback to regular queue
            let queue_key = queue_key(&self.config, queue_name);
            conn.rpop(&queue_key, None).await.map_err(|e| {
                QueueError::OperationFailed(format!("Failed to dequeue item: {}", e))
            })?
        };

        if let Some(data) = result {
            let item = deserialize_item(&data)?;
            let item_id = item.id();

            // Move item to processing
            let processing_key = processing_key(&self.config, queue_name);
            let _: () = conn
                .hset(&processing_key, item_id.to_string(), &data)
                .await
                .map_err(|e| {
                    QueueError::OperationFailed(format!("Failed to mark item as processing: {}", e))
                })?;

            self.log_operation(
                LogLevel::Info,
                format!("Dequeued item {} from queue {}", item_id, queue_name),
            )
            .await;

            Ok(Some(item))
        } else {
            Ok(None)
        }
    }

    async fn start_processing(
        &self,
        queue_name: &str,
        item_id: Uuid,
        worker_id: String,
    ) -> Result<(), QueueError> {
        let mut conn = self.conn.write().await;
        let processing_key = processing_key(&self.config, queue_name);

        // Check if item exists in processing
        let exists: bool = conn
            .hexists(&processing_key, item_id.to_string())
            .await
            .map_err(|e| {
                QueueError::OperationFailed(format!("Failed to check processing item: {}", e))
            })?;

        if !exists {
            return Err(QueueError::ItemNotFound(item_id));
        }

        // Update processing metadata
        let metadata_key = format!("{}:worker", processing_key);
        let _: () = conn
            .hset(&metadata_key, item_id.to_string(), &worker_id)
            .await
            .map_err(|e| {
                QueueError::OperationFailed(format!("Failed to start processing: {}", e))
            })?;

        self.log_operation(
            LogLevel::Info,
            format!(
                "Started processing item {} in queue {} by worker {}",
                item_id, queue_name, worker_id
            ),
        )
        .await;

        Ok(())
    }

    async fn complete_processing(
        &self,
        queue_name: &str,
        item_id: Uuid,
        result_data: Option<serde_json::Value>,
    ) -> Result<(), QueueError> {
        let mut conn = self.conn.write().await;
        let processing_key = processing_key(&self.config, queue_name);

        // Get item data from processing
        let item_data: Option<String> = conn
            .hget(&processing_key, item_id.to_string())
            .await
            .map_err(|e| {
                QueueError::OperationFailed(format!("Failed to get processing item: {}", e))
            })?;

        if let Some(data) = item_data {
            // Move to completed
            let completed_key = completed_key(&self.config, queue_name);
            if let Some(result) = result_data {
                let result_json = serde_json::to_string(&result).map_err(|e| {
                    QueueError::SerializationError(format!("Failed to serialize result: {}", e))
                })?;
                let _: () = conn
                    .hset(
                        format!("{}:result", completed_key),
                        item_id.to_string(),
                        result_json,
                    )
                    .await
                    .map_err(|e| {
                        QueueError::OperationFailed(format!("Failed to store result: {}", e))
                    })?;
            }

            let _: () = conn
                .hset(&completed_key, item_id.to_string(), &data)
                .await
                .map_err(|e| {
                    QueueError::OperationFailed(format!("Failed to mark as completed: {}", e))
                })?;

            // Remove from processing
            let _: () = conn
                .hdel(&processing_key, item_id.to_string())
                .await
                .map_err(|e| {
                    QueueError::OperationFailed(format!("Failed to remove from processing: {}", e))
                })?;

            self.log_operation(
                LogLevel::Info,
                format!(
                    "Completed processing item {} in queue {}",
                    item_id, queue_name
                ),
            )
            .await;

            Ok(())
        } else {
            Err(QueueError::ItemNotFound(item_id))
        }
    }

    async fn fail_processing(
        &self,
        queue_name: &str,
        item_id: Uuid,
        error: String,
    ) -> Result<bool, QueueError> {
        let mut conn = self.conn.write().await;
        let processing_key = processing_key(&self.config, queue_name);

        // Get item data from processing
        let item_data: Option<String> = conn
            .hget(&processing_key, item_id.to_string())
            .await
            .map_err(|e| {
                QueueError::OperationFailed(format!("Failed to get processing item: {}", e))
            })?;

        if let Some(data) = item_data {
            let mut item = deserialize_item(&data)?;

            // Use the item's max_retries setting, not the global config
            let max_retries = item.config.max_retries;
            let current_attempts = item.attempt_count;

            if current_attempts < max_retries {
                // Re-queue for retry
                item.attempt_count += 1;

                // Check if this is a priority-based queue and re-queue to the correct location
                let config = self.queue_configs.read().await.get(queue_name).cloned();
                let queue_key = if let Some(config) = config {
                    if config.priority_based {
                        // Re-queue to the correct priority queue based on the message priority
                        priority_queue_key(&self.config, queue_name, item.message.priority)
                    } else {
                        queue_key(&self.config, queue_name)
                    }
                } else {
                    queue_key(&self.config, queue_name)
                };

                // Instead of re-serializing, modify the original JSON data and re-queue that
                let mut item_json: serde_json::Value =
                    serde_json::from_str(&data).map_err(|e| {
                        QueueError::SerializationError(format!("Failed to parse item JSON: {}", e))
                    })?;

                // Update attempt count in the JSON
                if let Some(obj) = item_json.as_object_mut() {
                    obj.insert(
                        "attempt_count".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(item.attempt_count)),
                    );
                    obj.insert(
                        "queue_name".to_string(),
                        serde_json::Value::String(queue_name.to_string()),
                    );
                }

                let serialized = serde_json::to_string(&item_json).map_err(|e| {
                    QueueError::SerializationError(format!(
                        "Failed to serialize updated item: {}",
                        e
                    ))
                })?;

                let _: () = conn.lpush(&queue_key, &serialized).await.map_err(|e| {
                    QueueError::OperationFailed(format!("Failed to re-queue item: {}", e))
                })?;

                // Remove from processing
                let _: () = conn
                    .hdel(&processing_key, item_id.to_string())
                    .await
                    .map_err(|e| {
                        QueueError::OperationFailed(format!(
                            "Failed to remove from processing: {}",
                            e
                        ))
                    })?;

                self.log_operation(
                    LogLevel::Warn,
                    format!(
                        "Re-queued item {} for retry ({}/{}) in queue {}: {}",
                        item_id,
                        current_attempts + 1,
                        max_retries,
                        queue_name,
                        error
                    ),
                )
                .await;

                Ok(true) // Will retry
            } else {
                // Move to failed - max retries exceeded
                let failed_key = failed_key(&self.config, queue_name);
                let _: () = conn
                    .hset(&failed_key, item_id.to_string(), &data)
                    .await
                    .map_err(|e| {
                        QueueError::OperationFailed(format!("Failed to mark as failed: {}", e))
                    })?;

                // Store error details
                let _: () = conn
                    .hset(format!("{}:error", failed_key), item_id.to_string(), &error)
                    .await
                    .map_err(|e| {
                        QueueError::OperationFailed(format!("Failed to store error: {}", e))
                    })?;

                // Remove from processing
                let _: () = conn
                    .hdel(&processing_key, item_id.to_string())
                    .await
                    .map_err(|e| {
                        QueueError::OperationFailed(format!(
                            "Failed to remove from processing: {}",
                            e
                        ))
                    })?;

                self.log_operation(
                    LogLevel::Error,
                    format!(
                        "Failed processing item {} in queue {} after {} retries: {}",
                        item_id, queue_name, max_retries, error
                    ),
                )
                .await;

                Ok(false) // No more retries
            }
        } else {
            Err(QueueError::ItemNotFound(item_id))
        }
    }

    async fn get_queue_stats(&self, queue_name: &str) -> Result<QueueStats, QueueError> {
        if !self.queue_exists(queue_name).await? {
            return Err(QueueError::QueueNotFound(queue_name.to_string()));
        }

        let mut conn = self.conn.write().await;

        // Get counts from different Redis keys
        let processing_key = processing_key(&self.config, queue_name);
        let completed_key = completed_key(&self.config, queue_name);
        let failed_key = failed_key(&self.config, queue_name);

        // Check if this is a priority-based queue
        let config = self.queue_configs.read().await.get(queue_name).cloned();

        let pending: usize = if let Some(config) = config {
            if config.priority_based {
                // For priority-based queues, count items in all priority queues
                let mut total_pending = 0;
                for priority in Self::get_priority_levels() {
                    let priority_key = priority_queue_key(&self.config, queue_name, priority);
                    let count: usize = conn.llen(&priority_key).await.unwrap_or(0);
                    total_pending += count;
                }
                total_pending
            } else {
                // Regular FIFO queue
                let queue_key = queue_key(&self.config, queue_name);
                conn.llen(&queue_key).await.unwrap_or(0)
            }
        } else {
            // Fallback to regular queue
            let queue_key = queue_key(&self.config, queue_name);
            conn.llen(&queue_key).await.unwrap_or(0)
        };

        let processing: usize = conn.hlen(&processing_key).await.unwrap_or(0);
        let completed: usize = conn.hlen(&completed_key).await.unwrap_or(0);
        let failed: usize = conn.hlen(&failed_key).await.unwrap_or(0);

        Ok(QueueStats {
            name: queue_name.to_string(),
            total_items: pending + processing + completed + failed,
            pending_items: pending,
            processing_items: processing,
            completed_items: completed,
            failed_items: failed,
            abandoned_items: 0,
            oldest_item_age_seconds: Some(0),
            average_processing_time_ms: Some(0),
            throughput_per_minute: 0.0,
        })
    }

    async fn list_queues(&self) -> Vec<String> {
        let mut conn = self.conn.write().await;
        let queue_list_key = format!("{}:queues", self.config.key_prefix);

        conn.smembers::<_, Vec<String>>(&queue_list_key)
            .await
            .unwrap_or_default()
    }

    async fn queue_length(&self, queue_name: &str) -> Result<usize, QueueError> {
        let mut conn = self.conn.write().await;

        // Check if this is a priority-based queue
        let config = self.queue_configs.read().await.get(queue_name).cloned();

        let size: usize = if let Some(config) = config {
            if config.priority_based {
                // For priority-based queues, count items in all priority queues
                let mut total_size = 0;
                for priority in Self::get_priority_levels() {
                    let priority_key = priority_queue_key(&self.config, queue_name, priority);
                    let count: usize = conn.llen(&priority_key).await.unwrap_or(0);
                    total_size += count;
                }
                total_size
            } else {
                // Regular FIFO queue
                let queue_key = queue_key(&self.config, queue_name);
                conn.llen(&queue_key).await.unwrap_or(0)
            }
        } else {
            // Fallback to regular queue
            let queue_key = queue_key(&self.config, queue_name);
            conn.llen(&queue_key).await.unwrap_or(0)
        };

        Ok(size)
    }

    async fn cleanup_expired(&self) {
        self.log_operation(
            LogLevel::Info,
            "Starting cleanup of expired items".to_string(),
        )
        .await;

        // Get all queues and check for expired items
        let queues = self.list_queues().await;
        for _queue_name in queues {
            // Logic to clean up expired items in each queue
            // Cleanup expired items in processing, completed, and failed queues
            // This assumes QueueItem<T> has an `expires_at: Option<DateTime<Utc>>` field

            // let processing_key = processing_key(&self.config, &_queue_name);
            // let completed_key = completed_key(&self.config, &_queue_name);
            // let failed_key = failed_key(&self.config, &_queue_name);

            // let now = chrono::Utc::now();

            // // Helper closure to cleanup expired items in a hash
            // async fn cleanup_expired_in_hash(
            //     conn: &mut ConnectionManager,
            //     key: &str,
            //     queue_name: &str,
            //     log: &Option<Arc<dyn LogPort>>,
            //     now: chrono::DateTime<chrono::Utc>,
            // ) -> Result<usize, QueueError> {
            //     let items: HashMap<String, String> = conn.hgetall(key)
            //         .await
            //         .map_err(|e| QueueError::OperationFailed(format!("Failed to get items for cleanup: {}", e)))?;
            //     let mut expired_ids = Vec::new();
            //     for (id, data) in &items {
            //         if let Ok(item) = serde_json::from_str::<QueueItem<serde_json::Value>>(data) {
            //             if let Some(expires_at) = item.expires_at {
            //                 if expires_at < now {
            //                     expired_ids.push(id.clone());
            //                 }
            //             }
            //         }
            //     }
            //     if !expired_ids.is_empty() {
            //         let _: () = conn.hdel(key, expired_ids.clone())
            //             .await
            //             .map_err(|e| QueueError::OperationFailed(format!("Failed to delete expired items: {}", e)))?;
            //         if let Some(log_port) = log {
            //             let msg = format!("Cleaned up {} expired items from {}", expired_ids.len(), key);
            //             let entry = LogEntry {
            //                 id: Uuid::new_v4(),
            //                 timestamp: now,
            //                 message: LogMessage::new(LogLevel::Info, msg.clone()),
            //                 source: Location::service("redis-queue-adapter"),
            //                 destination: Location::system("log"),
            //                 correlation_id: None,
            //                 priority: MessagePriority::Normal,
            //             };
            //             let _ = log_port.write_entry(entry).await;
            //         }
            //     }
            //     Ok(expired_ids.len())
            // }

            // let mut conn = self.conn.write().await;
            // let _ = cleanup_expired_in_hash(&mut *conn, &processing_key, &_queue_name, &self.log_port, now).await;
            // let _ = cleanup_expired_in_hash(&mut *conn, &completed_key, &_queue_name, &self.log_port, now).await;
            // let _ = cleanup_expired_in_hash(&mut *conn, &failed_key, &_queue_name, &self.log_port, now).await;

            // // For pending queues (list), remove expired items by scanning and rebuilding the list
            // // Handle both regular and priority-based queues
            // let config = self.queue_configs.read().await.get(&_queue_name).cloned();
            // let queue_keys = if let Some(config) = config {
            //     if config.priority_based {
            //         Self::get_priority_levels()
            //             .into_iter()
            //             .map(|priority| priority_queue_key(&self.config, &_queue_name, priority))
            //             .collect::<Vec<_>>()
            //     } else {
            //         vec![queue_key(&self.config, &_queue_name)]
            //     }
            // } else {
            //     vec![queue_key(&self.config, &_queue_name)]
            // };

            // for queue_key in queue_keys {
            //     let items: Vec<String> = conn.lrange(&queue_key, 0, -1)
            //         .await
            //         .unwrap_or_default();
            //     let mut keep_items = Vec::new();
            //     let mut expired_count = 0;
            //     for data in items {
            //         if let Ok(item) = serde_json::from_str::<QueueItem<serde_json::Value>>(&data) {
            //             if let Some(expires_at) = item.expires_at {
            //                 if expires_at < now {
            //                     expired_count += 1;
            //                     continue;
            //                 }
            //             }
            //         }
            //         keep_items.push(data);
            //     }
            //     if expired_count > 0 {
            //         // Replace the list with only non-expired items
            //         let _: () = conn.del(&queue_key).await.unwrap_or(());
            //         if !keep_items.is_empty() {
            //             let _: () = conn.rpush(&queue_key, keep_items).await.unwrap_or(());
            //         }
            //         if let Some(log_port) = &self.log_port {
            //             let msg = format!("Cleaned up {} expired items from {}", expired_count, queue_key);
            //             let entry = LogEntry {
            //                 id: Uuid::new_v4(),
            //                 timestamp: now,
            //                 message: LogMessage::new(LogLevel::Info, msg.clone()),
            //                 source: Location::service("redis-queue-adapter"),
            //                 destination: Location::system("log"),
            //                 correlation_id: None,
            //                 priority: MessagePriority::Normal,
            //             };
            //             let _ = log_port.write_entry(entry).await;
            //         }
            //     }
            // }
        }
    }

    async fn get_all_stats(&self) -> HashMap<String, QueueStats> {
        let mut all_stats = HashMap::new();
        let queues = self.list_queues().await;

        for queue_name in queues {
            if let Ok(stats) = self.get_queue_stats(&queue_name).await {
                all_stats.insert(queue_name, stats);
            }
        }

        all_stats
    }

    async fn health_check(&self) -> Result<bool, QueueError> {
        let mut conn = self.conn.write().await;

        // Simple ping to check Redis connectivity
        match conn.ping::<String>().await {
            Ok(_) => {
                self.log_operation(
                    LogLevel::Info,
                    "Redis queue adapter health check passed".to_string(),
                )
                .await;
                Ok(true)
            }
            Err(e) => {
                self.log_operation(
                    LogLevel::Error,
                    format!("Redis queue adapter health check failed: {}", e),
                )
                .await;
                Err(QueueError::OperationFailed(format!(
                    "Health check failed: {}",
                    e
                )))
            }
        }
    }
}

#[async_trait]
impl BatchQueuePort for RedisQueueAdapter {
    async fn enqueue_batch<T>(
        &self,
        queue_name: &str,
        items: Vec<QueueItem<T>>,
    ) -> Result<Vec<Uuid>, QueueError>
    where
        T: Serialize + Clone + for<'de> Deserialize<'de> + Send + Sync,
    {
        if !self.queue_exists(queue_name).await? {
            return Err(QueueError::QueueNotFound(queue_name.to_string()));
        }

        if items.is_empty() {
            return Ok(Vec::new());
        }

        let mut conn = self.conn.write().await;
        let mut item_ids = Vec::new();

        // Use pipeline for efficiency
        let mut pipe = redis::pipe();

        for item in &items {
            let item_id = item.id();
            item_ids.push(item_id);

            let serialized = serialize_item(item, queue_name)?;
            let queue_key = queue_key(&self.config, queue_name);
            pipe.lpush(&queue_key, &serialized);
        }

        let _: () = pipe
            .query_async(&mut *conn)
            .await
            .map_err(|e| QueueError::OperationFailed(format!("Failed to enqueue batch: {}", e)))?;

        self.log_operation(
            LogLevel::Info,
            format!(
                "Enqueued batch of {} items to queue {}",
                items.len(),
                queue_name
            ),
        )
        .await;

        Ok(item_ids)
    }

    async fn enqueue_with_priority<T>(
        &self,
        queue_name: &str,
        item: QueueItem<T>,
        priority: MessagePriority,
    ) -> Result<Uuid, QueueError>
    where
        T: Serialize + Clone + for<'de> Deserialize<'de> + Send + Sync, // Fixed: match trait requirements
    {
        if !self.queue_exists(queue_name).await? {
            return Err(QueueError::QueueNotFound(queue_name.to_string()));
        }

        let item_id = item.id();
        let serialized = serialize_item(&item, queue_name)?;

        let mut conn = self.conn.write().await;
        let queue_key = priority_queue_key(&self.config, queue_name, priority);

        let _: () = conn.lpush(&queue_key, &serialized).await.map_err(|e| {
            QueueError::OperationFailed(format!("Failed to enqueue with priority: {}", e))
        })?;

        self.log_operation(
            LogLevel::Info,
            format!(
                "Enqueued item {} to queue {} with priority {:?}",
                item_id, queue_name, priority
            ),
        )
        .await;

        Ok(item_id)
    }

    async fn dequeue_batch(
        &self,
        queue_name: &str,
        count: usize,
    ) -> Result<Vec<QueueItem<serde_json::Value>>, QueueError> {
        if !self.queue_exists(queue_name).await? {
            return Err(QueueError::QueueNotFound(queue_name.to_string()));
        }

        if count == 0 {
            return Ok(Vec::new());
        }

        let mut conn = self.conn.write().await;
        let mut items = Vec::new();
        let queue_key = queue_key(&self.config, queue_name);
        let processing_key = processing_key(&self.config, queue_name);

        // Simple approach: dequeue items one by one
        // In production, you might want to use a Lua script for atomicity
        for _ in 0..count {
            let data: Option<String> = conn.rpop(&queue_key, None).await.map_err(|e| {
                QueueError::OperationFailed(format!("Failed to dequeue batch item: {}", e))
            })?;

            if let Some(item_data) = data {
                let item = deserialize_item(&item_data)?;
                let item_id = item.id();

                // Mark as processing
                let _: () = conn
                    .hset(&processing_key, item_id.to_string(), &item_data)
                    .await
                    .map_err(|e| {
                        QueueError::OperationFailed(format!(
                            "Failed to mark batch item as processing: {}",
                            e
                        ))
                    })?;

                items.push(item);
            } else {
                break; // No more items
            }
        }

        self.log_operation(
            LogLevel::Info,
            format!(
                "Dequeued batch of {} items from queue {}",
                items.len(),
                queue_name
            ),
        )
        .await;

        Ok(items)
    }

    async fn get_item_summaries(
        &self,
        queue_name: &str,
        item_ids: Vec<Uuid>,
    ) -> Result<Vec<QueueItemSummary>, QueueError> {
        if !self.queue_exists(queue_name).await? {
            return Err(QueueError::QueueNotFound(queue_name.to_string()));
        }

        let mut conn = self.conn.write().await;
        let mut summaries = Vec::new();

        // Check in processing, completed, and failed
        let processing_key = processing_key(&self.config, queue_name);
        let completed_key = completed_key(&self.config, queue_name);
        let failed_key = failed_key(&self.config, queue_name);

        for item_id in item_ids {
            let id_str = item_id.to_string();

            // Check processing
            if let Ok(Some(_)) = conn
                .hget::<_, _, Option<String>>(&processing_key, &id_str)
                .await
            {
                summaries.push(QueueItemSummary {
                    id: item_id,
                    queue_name: queue_name.to_string(),
                    status: QueueItemStatus::Processing,
                    message_id: item_id,
                    attempt_count: 0,
                    age_seconds: 0,
                    is_expired: false,
                    processing_duration_ms: Some(0),
                    worker_id: None,
                });
                continue;
            }

            // Check completed
            if let Ok(Some(_)) = conn
                .hget::<_, _, Option<String>>(&completed_key, &id_str)
                .await
            {
                summaries.push(QueueItemSummary {
                    id: item_id,
                    queue_name: queue_name.to_string(),
                    status: QueueItemStatus::Completed,
                    message_id: item_id,
                    attempt_count: 0,
                    age_seconds: 0,
                    is_expired: false,
                    processing_duration_ms: Some(0),
                    worker_id: None,
                });
                continue;
            }

            // Check failed
            if let Ok(Some(_)) = conn
                .hget::<_, _, Option<String>>(&failed_key, &id_str)
                .await
            {
                summaries.push(QueueItemSummary {
                    id: item_id,
                    queue_name: queue_name.to_string(),
                    status: QueueItemStatus::Failed,
                    message_id: item_id,
                    attempt_count: 0,
                    age_seconds: 0,
                    is_expired: false,
                    processing_duration_ms: Some(0),
                    worker_id: None,
                });
                continue;
            }
        }

        Ok(summaries)
    }
}

#[async_trait]
impl PriorityQueuePort for RedisQueueAdapter {
    async fn enqueue_with_priority<T>(
        &self,
        queue_name: &str,
        item: QueueItem<T>,
        priority: MessagePriority,
    ) -> Result<Uuid, QueueError>
    where
        T: Serialize + Send + Sync,
    {
        if !self.queue_exists(queue_name).await? {
            return Err(QueueError::QueueNotFound(queue_name.to_string()));
        }

        let item_id = item.action.id; // Use the action's ID directly
        let serialized = serialize_item(&item, queue_name)?;

        let mut conn = self.conn.write().await;
        let queue_key = priority_queue_key(&self.config, queue_name, priority);

        let _: () = conn.lpush(&queue_key, &serialized).await.map_err(|e| {
            QueueError::OperationFailed(format!("Failed to enqueue with priority: {}", e))
        })?;

        self.log_operation(
            LogLevel::Info,
            format!(
                "Enqueued item {} to queue {} with priority {:?}",
                item_id, queue_name, priority
            ),
        )
        .await;

        Ok(item_id)
    }

    async fn dequeue_highest_priority(
        &self,
        queue_name: &str,
    ) -> Result<Option<QueueItem<serde_json::Value>>, QueueError> {
        if !self.queue_exists(queue_name).await? {
            return Err(QueueError::QueueNotFound(queue_name.to_string()));
        }

        let mut conn = self.conn.write().await;
        let processing_key = processing_key(&self.config, queue_name);

        // Try each priority level in order
        for priority in Self::get_priority_levels() {
            let queue_key = priority_queue_key(&self.config, queue_name, priority);

            let data: Option<String> = conn.rpop(&queue_key, None).await.map_err(|e| {
                QueueError::OperationFailed(format!("Failed to dequeue priority item: {}", e))
            })?;

            if let Some(item_data) = data {
                let item = deserialize_item(&item_data)?;
                let item_id = item.id();

                // Move to processing
                let _: () = conn
                    .hset(&processing_key, item_id.to_string(), &item_data)
                    .await
                    .map_err(|e| {
                        QueueError::OperationFailed(format!("Failed to mark as processing: {}", e))
                    })?;

                self.log_operation(
                    LogLevel::Info,
                    format!(
                        "Dequeued highest priority item {} from queue {} (priority: {:?})",
                        item_id, queue_name, priority
                    ),
                )
                .await;

                return Ok(Some(item));
            }
        }

        Ok(None)
    }

    async fn get_items_by_priority(
        &self,
        queue_name: &str,
        priority: MessagePriority,
    ) -> Result<Vec<QueueItemSummary>, QueueError> {
        if !self.queue_exists(queue_name).await? {
            return Err(QueueError::QueueNotFound(queue_name.to_string()));
        }

        let mut conn = self.conn.write().await;
        let queue_key = priority_queue_key(&self.config, queue_name, priority);

        // Get all items from priority queue without removing them
        let items: Vec<String> = conn.lrange(&queue_key, 0, -1).await.map_err(|e| {
            QueueError::OperationFailed(format!("Failed to get priority items: {}", e))
        })?;

        let mut summaries = Vec::new();
        for data in items {
            if let Ok(item) = deserialize_item(&data) {
                summaries.push(QueueItemSummary {
                    id: item.id(),
                    queue_name: queue_name.to_string(),
                    status: QueueItemStatus::Pending,
                    message_id: item.id(),
                    attempt_count: item.attempt_count,
                    age_seconds: 0,
                    is_expired: false,
                    processing_duration_ms: Some(0),
                    worker_id: None,
                });
            }
        }

        Ok(summaries)
    }
}

#[async_trait]
impl QueueManagementPort for RedisQueueAdapter {
    async fn pause_queue(&self, queue_name: &str) -> Result<(), QueueError> {
        if !self.queue_exists(queue_name).await? {
            return Err(QueueError::QueueNotFound(queue_name.to_string()));
        }

        let mut conn = self.conn.write().await;
        let meta_key = queue_meta_key(&self.config, queue_name);

        let _: () = conn
            .hset(&meta_key, "paused", "true")
            .await
            .map_err(|e| QueueError::OperationFailed(format!("Failed to pause queue: {}", e)))?;

        self.log_operation(LogLevel::Info, format!("Paused queue: {}", queue_name))
            .await;
        Ok(())
    }

    async fn resume_queue(&self, queue_name: &str) -> Result<(), QueueError> {
        if !self.queue_exists(queue_name).await? {
            return Err(QueueError::QueueNotFound(queue_name.to_string()));
        }

        let mut conn = self.conn.write().await;
        let meta_key = queue_meta_key(&self.config, queue_name);

        let _: () = conn
            .hset(&meta_key, "paused", "false")
            .await
            .map_err(|e| QueueError::OperationFailed(format!("Failed to resume queue: {}", e)))?;

        self.log_operation(LogLevel::Info, format!("Resumed queue: {}", queue_name))
            .await;
        Ok(())
    }

    async fn cancel_item(&self, queue_name: &str, item_id: Uuid) -> Result<(), QueueError> {
        let mut conn = self.conn.write().await;
        let processing_key = processing_key(&self.config, queue_name);
        let id_str = item_id.to_string();

        // Check if item is in processing
        let exists: bool = conn.hexists(&processing_key, &id_str).await.map_err(|e| {
            QueueError::OperationFailed(format!("Failed to check item existence: {}", e))
        })?;

        if exists {
            // Remove from processing
            let _: () = conn.hdel(&processing_key, &id_str).await.map_err(|e| {
                QueueError::OperationFailed(format!("Failed to cancel item: {}", e))
            })?;

            self.log_operation(
                LogLevel::Info,
                format!("Cancelled item {} in queue {}", item_id, queue_name),
            )
            .await;

            Ok(())
        } else {
            Err(QueueError::ItemNotFound(item_id))
        }
    }

    async fn retry_item(&self, queue_name: &str, item_id: Uuid) -> Result<(), QueueError> {
        let mut conn = self.conn.write().await;
        let failed_key = failed_key(&self.config, queue_name);
        let id_str = item_id.to_string();

        // Get item from failed queue
        let item_data: Option<String> = conn.hget(&failed_key, &id_str).await.map_err(|e| {
            QueueError::OperationFailed(format!("Failed to get failed item: {}", e))
        })?;

        if let Some(data) = item_data {
            // Reset attempt count in the JSON data directly
            let mut item_json: serde_json::Value = serde_json::from_str(&data).map_err(|e| {
                QueueError::SerializationError(format!("Failed to parse item JSON: {}", e))
            })?;

            // Reset attempt count and update queue name
            if let Some(obj) = item_json.as_object_mut() {
                obj.insert(
                    "attempt_count".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(0)),
                );
                obj.insert(
                    "queue_name".to_string(),
                    serde_json::Value::String(queue_name.to_string()),
                );
            }

            let serialized = serde_json::to_string(&item_json).map_err(|e| {
                QueueError::SerializationError(format!("Failed to serialize retry item: {}", e))
            })?;

            let queue_key = queue_key(&self.config, queue_name);
            let _: () = conn
                .lpush(&queue_key, &serialized)
                .await
                .map_err(|e| QueueError::OperationFailed(format!("Failed to retry item: {}", e)))?;

            // Remove from failed
            let _: () = conn.hdel(&failed_key, &id_str).await.map_err(|e| {
                QueueError::OperationFailed(format!("Failed to remove from failed: {}", e))
            })?;

            self.log_operation(
                LogLevel::Info,
                format!("Retried item {} in queue {}", item_id, queue_name),
            )
            .await;

            Ok(())
        } else {
            Err(QueueError::ItemNotFound(item_id))
        }
    }

    async fn get_item_details(
        &self,
        queue_name: &str,
        item_id: Uuid,
    ) -> Result<QueueItem<serde_json::Value>, QueueError> {
        let mut conn = self.conn.write().await;
        let id_str = item_id.to_string();

        // Check all possible locations
        let keys = vec![
            processing_key(&self.config, queue_name),
            completed_key(&self.config, queue_name),
            failed_key(&self.config, queue_name),
        ];

        for key in keys {
            let item_data: Option<String> = conn.hget(&key, &id_str).await.map_err(|e| {
                QueueError::OperationFailed(format!("Failed to get item details: {}", e))
            })?;

            if let Some(data) = item_data {
                return deserialize_item(&data);
            }
        }

        // Check in pending queues
        let queue_key = queue_key(&self.config, queue_name);
        let items: Vec<String> = conn
            .lrange(&queue_key, 0, -1)
            .await
            .map_err(|e| QueueError::OperationFailed(format!("Failed to search queue: {}", e)))?;

        for data in items {
            if let Ok(item) = deserialize_item(&data)
                && item.id() == item_id
            {
                return Ok(item);
            }
        }

        Err(QueueError::ItemNotFound(item_id))
    }

    async fn purge_completed(&self, queue_name: &str) -> Result<usize, QueueError> {
        let mut conn = self.conn.write().await;
        let completed_key = completed_key(&self.config, queue_name);

        let count: usize = conn.hlen(&completed_key).await.map_err(|e| {
            QueueError::OperationFailed(format!("Failed to get completed count: {}", e))
        })?;

        let _: () = conn.del(&completed_key).await.map_err(|e| {
            QueueError::OperationFailed(format!("Failed to purge completed: {}", e))
        })?;

        self.log_operation(
            LogLevel::Info,
            format!("Purged {} completed items from queue {}", count, queue_name),
        )
        .await;

        Ok(count)
    }

    async fn purge_failed(&self, queue_name: &str) -> Result<usize, QueueError> {
        let mut conn = self.conn.write().await;
        let failed_key = failed_key(&self.config, queue_name);

        let count: usize = conn.hlen(&failed_key).await.map_err(|e| {
            QueueError::OperationFailed(format!("Failed to get failed count: {}", e))
        })?;

        let _: () = conn
            .del(&failed_key)
            .await
            .map_err(|e| QueueError::OperationFailed(format!("Failed to purge failed: {}", e)))?;

        self.log_operation(
            LogLevel::Info,
            format!("Purged {} failed items from queue {}", count, queue_name),
        )
        .await;

        Ok(count)
    }

    async fn get_queue_config(&self, queue_name: &str) -> Result<QueueConfig, QueueError> {
        if let Some(config) = self.queue_configs.read().await.get(queue_name) {
            Ok(config.clone())
        } else {
            // Try to load from Redis
            let mut conn = self.conn.write().await;
            let meta_key = queue_meta_key(&self.config, queue_name);

            let config_json: Option<String> =
                conn.hget(&meta_key, "config").await.map_err(|e| {
                    QueueError::OperationFailed(format!("Failed to get queue config: {}", e))
                })?;

            if let Some(json) = config_json {
                let config: QueueConfig = serde_json::from_str(&json).map_err(|e| {
                    QueueError::SerializationError(format!("Failed to deserialize config: {}", e))
                })?;

                // Cache it
                {
                    let mut configs = self.queue_configs.write().await;
                    configs.insert(queue_name.to_string(), config.clone());
                }

                Ok(config)
            } else {
                Err(QueueError::QueueNotFound(queue_name.to_string()))
            }
        }
    }

    async fn update_queue_config(
        &self,
        queue_name: &str,
        config: QueueConfig,
    ) -> Result<(), QueueError> {
        if !self.queue_exists(queue_name).await? {
            return Err(QueueError::QueueNotFound(queue_name.to_string()));
        }

        let mut conn = self.conn.write().await;
        let meta_key = queue_meta_key(&self.config, queue_name);
        let config_json = serde_json::to_string(&config).map_err(|e| {
            QueueError::SerializationError(format!("Failed to serialize config: {}", e))
        })?;

        let _: () = conn
            .hset(&meta_key, "config", config_json)
            .await
            .map_err(|e| QueueError::OperationFailed(format!("Failed to update config: {}", e)))?;

        // Update cache
        {
            let mut configs = self.queue_configs.write().await;
            configs.insert(queue_name.to_string(), config);
        }

        self.log_operation(
            LogLevel::Info,
            format!("Updated configuration for queue {}", queue_name),
        )
        .await;

        Ok(())
    }
}

// Implement FullQueuePort
impl FullQueuePort for RedisQueueAdapter {}

impl RedisQueueAdapter {
    /// Shutdown the adapter and close connections
    pub async fn shutdown(&self) -> Result<(), QueueError> {
        self.log_operation(
            LogLevel::Info,
            "Shutting down Redis queue adapter".to_string(),
        )
        .await;
        // Connection will be automatically closed when dropped
        Ok(())
    }

    /// Get Redis connection info for debugging
    pub fn get_connection_info(&self) -> String {
        format!(
            "{}:{}/{}",
            self.config.redis_host, self.config.redis_port, self.config.redis_db
        )
    }
}

// ---------------------------------------------------------------------------
// Pure, connection-free tests -- these construct no RedisQueueAdapter and
// require no Redis server or Docker. They exercise only the key-construction
// and (de)serialization seams that Task 1 of this plan moved off `&self` so
// they could be reached without a live `ConnectionManager`.
//
// The live-server code paths of this file (every trait method that reaches
// through `self.conn`) remain uncovered here. They are exercised instead by
// the testcontainers-based suite at
// `tests/integration/redis_queue_integration_test.rs`, which requires
// Docker. Docker is absent from this execution environment, so closing that
// gap is deferred with reason: owner is Phase 15 (PIPE), which is where this
// project's coverage-tooling and CI-hardening work is scoped.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::base::entity::message::Message;
    use std::collections::HashSet;

    /// Build a `RedisQueueConfig` for tests, overriding only `key_prefix` with a
    /// synthetic literal. No field here is a real hostname, credential, or
    /// connection string -- the module never opens a connection.
    fn test_config() -> RedisQueueConfig {
        RedisQueueConfig {
            key_prefix: "test-prefix".to_string(),
            ..RedisQueueConfig::default()
        }
    }

    /// Build a synthetic `QueueItem<serde_json::Value>` for round-trip tests.
    fn test_item(queue_name: &str, priority: MessagePriority) -> QueueItem<serde_json::Value> {
        let message = Message::with_priority(
            Location::service("test-source"),
            Location::service("test-destination"),
            serde_json::json!({"payload": "synthetic-fixture-value"}),
            priority,
        );
        QueueItem::new(queue_name.to_string(), message, None)
    }

    #[test]
    fn redis_queue_config_default_matches_documented_values() {
        let config = RedisQueueConfig::default();

        assert_eq!(config.redis_host, "localhost");
        assert_eq!(config.redis_port, 6379);
        assert_eq!(config.redis_password, None);
        assert_eq!(config.redis_db, 0);
        assert_eq!(config.connection_timeout, 30);
        assert_eq!(config.key_prefix, "paladin:queue");
        assert_eq!(config.max_retries, 3);
    }

    /// R-95 (58-06 register, 56-REVIEW WR-09 class): serialising a
    /// `RedisQueueConfig` must not write the password.
    #[test]
    fn redis_queue_config_serialisation_omits_the_redis_password() {
        let sentinel = "test-key-not-a-secret-r95-redis-queue-password";
        let config = RedisQueueConfig {
            redis_password: Some(sentinel.to_string()),
            ..test_config()
        };
        assert_eq!(config.redis_password.as_deref(), Some(sentinel));
        let value = serde_json::to_value(&config).expect("RedisQueueConfig serialises");
        let object = value.as_object().expect("serialises as an object");
        assert!(
            !object.contains_key("redis_password"),
            "RedisQueueConfig serialised `redis_password`"
        );
        assert!(
            !value.to_string().contains(sentinel),
            "RedisQueueConfig serialisation carries the password value"
        );
        // Non-vacuous: the neighbouring fields are still written.
        assert_eq!(object["redis_host"], "localhost");
        assert_eq!(object["key_prefix"], "test-prefix");
    }

    /// R-95 (DF-07 class): a `Debug` rendering must not carry the password.
    #[test]
    fn redis_queue_config_debug_redacts_the_redis_password() {
        let sentinel = "test-key-not-a-secret-r95-redis-queue-password";
        let config = RedisQueueConfig {
            redis_host: "queue.example.internal".to_string(),
            redis_password: Some(sentinel.to_string()),
            ..test_config()
        };
        assert_eq!(config.redis_password.as_deref(), Some(sentinel));
        let rendered = format!("{config:?}");
        assert!(
            !rendered.contains(sentinel),
            "password leaked into Debug output: {rendered}"
        );
        assert!(
            rendered.contains("<redacted>"),
            "marker missing: {rendered}"
        );
        assert!(rendered.contains("queue.example.internal"));
        assert!(rendered.contains("test-prefix"));

        let unset = format!("{:?}", test_config());
        assert!(unset.contains("redis_password: None"), "{unset}");
    }

    /// R-95 drift guard: the pretty `Debug` of a default config names every
    /// serialised field plus `redis_password`, so a later field cannot be
    /// dropped from (or silently left out of) the hand-written `Debug`.
    #[test]
    fn redis_queue_config_debug_names_every_field() {
        let rendered = format!("{:#?}", RedisQueueConfig::default());
        let value = serde_json::to_value(RedisQueueConfig::default()).expect("serialises");
        let object = value.as_object().expect("serialises as an object");
        for key in object.keys().map(String::as_str).chain(["redis_password"]) {
            assert!(
                rendered.contains(&format!("{key}:")),
                "Debug output does not name `{key}`: {rendered}"
            );
        }
    }

    /// R-95 load pin: a JSON value carrying `redis_password` still loads it.
    #[test]
    fn redis_queue_config_still_loads_a_redis_password() {
        let sentinel = "test-key-not-a-secret-r95-redis-queue-loaded";
        let value = serde_json::json!({
            "redis_host": "queue.example.internal",
            "redis_port": 6379,
            "redis_password": sentinel,
            "redis_db": 1,
            "connection_timeout": 30,
            "key_prefix": "test-prefix",
            "max_retries": 3,
        });
        let config: RedisQueueConfig =
            serde_json::from_value(value).expect("a value with a password loads");
        assert_eq!(config.redis_password.as_deref(), Some(sentinel));
        assert_eq!(config.redis_host, "queue.example.internal");
    }

    #[test]
    fn queue_key_builds_expected_literal() {
        let config = test_config();
        assert_eq!(queue_key(&config, "orders"), "test-prefix:queue:orders");
    }

    #[test]
    fn priority_queue_key_builds_expected_literal_for_each_variant() {
        let config = test_config();
        assert_eq!(
            priority_queue_key(&config, "orders", MessagePriority::Critical),
            "test-prefix:orders:critical"
        );
        assert_eq!(
            priority_queue_key(&config, "orders", MessagePriority::High),
            "test-prefix:orders:high"
        );
        assert_eq!(
            priority_queue_key(&config, "orders", MessagePriority::Normal),
            "test-prefix:orders:normal"
        );
        assert_eq!(
            priority_queue_key(&config, "orders", MessagePriority::Low),
            "test-prefix:orders:low"
        );
    }

    #[test]
    fn queue_meta_key_builds_expected_literal() {
        let config = test_config();
        assert_eq!(queue_meta_key(&config, "orders"), "test-prefix:meta:orders");
    }

    #[test]
    fn processing_key_builds_expected_literal() {
        let config = test_config();
        assert_eq!(
            processing_key(&config, "orders"),
            "test-prefix:processing:orders"
        );
    }

    #[test]
    fn completed_key_builds_expected_literal() {
        let config = test_config();
        assert_eq!(
            completed_key(&config, "orders"),
            "test-prefix:completed:orders"
        );
    }

    #[test]
    fn failed_key_builds_expected_literal() {
        let config = test_config();
        assert_eq!(failed_key(&config, "orders"), "test-prefix:failed:orders");
    }

    #[test]
    fn priority_queue_key_no_two_variants_collide() {
        let config = test_config();
        let keys: HashSet<String> = RedisQueueAdapter::get_priority_levels()
            .into_iter()
            .map(|priority| priority_queue_key(&config, "orders", priority))
            .collect();

        assert_eq!(
            keys.len(),
            RedisQueueAdapter::get_priority_levels().len(),
            "expected one distinct key per MessagePriority variant, got: {keys:?}"
        );
    }

    #[test]
    fn serialize_then_deserialize_round_trips_identifying_fields() {
        let item = test_item("original-queue", MessagePriority::High);
        let item_id = item.id();
        let message_id = item.message_id();

        let serialized = serialize_item(&item, "retargeted-queue").expect("serialization failed");
        let round_tripped = deserialize_item(&serialized).expect("deserialization failed");

        assert_eq!(round_tripped.id(), item_id);
        assert_eq!(round_tripped.message_id(), message_id);
        // serialize_item stamps the passed-in queue name into the payload,
        // overriding whatever queue_name the item carried when constructed.
        assert_eq!(round_tripped.queue_name, "retargeted-queue");
        assert_eq!(round_tripped.message.priority, MessagePriority::High);
    }

    #[test]
    fn deserialize_item_on_invalid_json_returns_serialization_error_not_panic() {
        let result = deserialize_item("not-valid-json{{{");

        match result {
            Err(QueueError::SerializationError(_)) => {}
            other => panic!("expected Err(QueueError::SerializationError(_)), got {other:?}"),
        }
    }

    #[test]
    fn get_priority_levels_returns_documented_order() {
        assert_eq!(
            RedisQueueAdapter::get_priority_levels(),
            vec![
                MessagePriority::Critical,
                MessagePriority::High,
                MessagePriority::Normal,
                MessagePriority::Low,
            ]
        );
    }
}
