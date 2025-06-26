use async_trait::async_trait;
use redis::{Client, aio::ConnectionManager, AsyncCommands};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::Utc;

use crate::application::ports::output::queue_port::{
    QueuePort, BatchQueuePort, PriorityQueuePort, QueueManagementPort, FullQueuePort
};
use crate::application::ports::output::log_port::LogPort;
use crate::core::platform::container::queue_item::{QueueItem, QueueItemSummary, QueueItemStatus};
use crate::core::platform::manager::queue_service::{QueueStats, QueueConfig, QueueError};
use crate::core::base::entity::message::{MessagePriority, Location};
use crate::core::platform::container::log::{LogMessage, LogEntry, LogLevel};

/// Configuration for Redis connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisQueueConfig {
    pub redis_host: String,
    pub redis_port: u16,
    pub redis_password: Option<String>,
    pub redis_db: u8,
    pub connection_timeout: u64,
    pub key_prefix: String,
    pub max_retries: u32,
}

impl Default for RedisQueueConfig {
    fn default() -> Self {
        Self {
            redis_host: "localhost".to_string(),
            redis_port: 6379,
            redis_password: None,
            redis_db: 0,
            connection_timeout: 30,
            key_prefix: "in4me:queue".to_string(),
            max_retries: 3,
        }
    }
}

/// Redis queue adapter that implements all queue port traits
pub struct RedisQueueAdapter {
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
        log_port: Option<Arc<dyn LogPort>>
    ) -> Result<Self, QueueError> {
        let connection_url = if let Some(password) = &config.redis_password {
            format!("redis://:{}@{}:{}/{}", password, config.redis_host, config.redis_port, config.redis_db)
        } else {
            format!("redis://{}:{}/{}", config.redis_host, config.redis_port, config.redis_db)
        };

        let client = Client::open(connection_url)
            .map_err(|e| QueueError::OperationFailed(format!("Failed to create Redis client: {}", e)))?;

        let conn = ConnectionManager::new(client.clone())
            .await
            .map_err(|e| QueueError::OperationFailed(format!("Failed to connect to Redis: {}", e)))?;

        Ok(Self {
            client,
            conn: Arc::new(RwLock::new(conn)),
            config,
            log_port,
            queue_configs: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    // In RedisQueueAdapter, check this method:
    fn queue_key(&self, queue_name: &str) -> String {
        format!("{}:queue:{}", self.config.key_prefix, queue_name)
    }

    /// Generate Redis key for priority queue
    fn priority_queue_key(&self, queue_name: &str, priority: MessagePriority) -> String {
        let priority_str = match priority {
            MessagePriority::Critical => "critical",
            MessagePriority::High => "high", 
            MessagePriority::Normal => "normal",
            MessagePriority::Low => "low",
        };
        format!("{}:{}:{}", self.config.key_prefix, queue_name, priority_str)
    }

    /// Generate Redis key for queue metadata
    fn queue_meta_key(&self, queue_name: &str) -> String {
        format!("{}:meta:{}", self.config.key_prefix, queue_name)
    }

    /// Generate Redis key for processing items
    fn processing_key(&self, queue_name: &str) -> String {
        format!("{}:processing:{}", self.config.key_prefix, queue_name)
    }

    /// Generate Redis key for completed items
    fn completed_key(&self, queue_name: &str) -> String {
        format!("{}:completed:{}", self.config.key_prefix, queue_name)
    }

    /// Generate Redis key for failed items
    fn failed_key(&self, queue_name: &str) -> String {
        format!("{}:failed:{}", self.config.key_prefix, queue_name)
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

    // Add debugging to serialize_item
    fn serialize_item<T>(&self, item: &QueueItem<T>) -> Result<String, QueueError>
    where
        T: Serialize,
    {
        let serialized = serde_json::to_string(item)
            .map_err(|e| QueueError::SerializationError(format!("Failed to serialize item: {}", e)))?;
        
        println!("DEBUG: Serialized item: {}", serialized);
        Ok(serialized)
    }

    /// Deserialize queue item from Redis storage
    fn deserialize_item(&self, data: &str) -> Result<QueueItem<serde_json::Value>, QueueError> {
        serde_json::from_str(data)
            .map_err(|e| QueueError::SerializationError(format!("Failed to deserialize item: {}", e)))
    }

    /// Check if queue exists
    async fn queue_exists(&self, queue_name: &str) -> Result<bool, QueueError> {
        let mut conn = self.conn.write().await;
        let meta_key = self.queue_meta_key(queue_name);
        
        let exists: bool = conn.exists(&meta_key)
            .await
            .map_err(|e| QueueError::OperationFailed(format!("Failed to check queue existence: {}", e)))?;
        
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
    async fn create_queue(&self, queue_name: String, config: Option<QueueConfig>) -> Result<(), QueueError> {
        // Check if queue already exists
        if self.queue_exists(&queue_name).await? {
            return Err(QueueError::OperationFailed("Queue already exists".to_string()));
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
        let meta_key = self.queue_meta_key(&queue_name);
        let config_json = serde_json::to_string(&queue_config)
            .map_err(|e| QueueError::SerializationError(format!("Failed to serialize config: {}", e)))?;

        // Store queue metadata (this is what queue_exists() looks for)
        let _: () = conn.hset(&meta_key, "config", &config_json)
            .await
            .map_err(|e| QueueError::OperationFailed(format!("Failed to create queue metadata: {}", e)))?;

        let _: () = conn.hset(&meta_key, "created_at", chrono::Utc::now().to_rfc3339())
            .await
            .map_err(|e| QueueError::OperationFailed(format!("Failed to set creation time: {}", e)))?;

        // Store the queue configuration in memory
        {
            let mut configs = self.queue_configs.write().await;
            configs.insert(queue_name.clone(), queue_config);
        }

        // Add to queue list (for list_queues())
        let queue_list_key = format!("{}:queues", self.config.key_prefix);
        let _: () = conn.sadd(&queue_list_key, &queue_name)
            .await
            .map_err(|e| QueueError::OperationFailed(format!("Failed to add queue to list: {}", e)))?;

        self.log_operation(LogLevel::Info, format!("Created queue: {}", queue_name)).await;

        Ok(())
    }

    async fn delete_queue(&self, name: &str) -> Result<(), QueueError> {
        if !self.queue_exists(name).await? {
            return Err(QueueError::QueueNotFound(name.to_string()));
        }

        let mut conn = self.conn.write().await;
        
        // Delete all queue-related keys using pipeline for efficiency
        let mut pipe = redis::pipe();
        pipe.del(&self.queue_key(name))
            .del(&self.queue_meta_key(name))
            .del(&self.processing_key(name))
            .del(&self.completed_key(name))
            .del(&self.failed_key(name));
        
        // Delete priority queues
        for priority in Self::get_priority_levels() {
            pipe.del(&self.priority_queue_key(name, priority));
        }

        let _: () = pipe.query_async(&mut *conn)
            .await
            .map_err(|e| QueueError::OperationFailed(format!("Failed to delete queue: {}", e)))?;

        // Remove from memory cache
        {
            let mut configs = self.queue_configs.write().await;
            configs.remove(name);
        }

        self.log_operation(LogLevel::Info, format!("Deleted queue: {}", name)).await;
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
        let serialized = self.serialize_item(&item)?;
        
        let mut conn = self.conn.write().await;
        
        // Determine which queue to use based on priority
        let queue_key = if let Some(config) = self.queue_configs.read().await.get(queue_name) {
            if config.priority_based {
                self.priority_queue_key(queue_name, item.message.priority)
            } else {
                self.queue_key(queue_name)
            }
        } else {
            self.queue_key(queue_name)
        };
        
        println!("DEBUG: Queue key: {}", queue_key);
        println!("DEBUG: Serialized item: {}", serialized);

        // Add to queue (LPUSH for FIFO with RPOP)
        let _: () = conn.lpush(&queue_key, &serialized)
            .await
            .map_err(|e| QueueError::OperationFailed(format!("Failed to enqueue item: {}", e)))?;

        println!("DEBUG: lpush completed successfully");
        
        self.log_operation(
            LogLevel::Info, 
            format!("Enqueued item {} to queue {}", item_id, queue_name)
        ).await;

        
        Ok(item_id)
    }

    async fn dequeue(&self, queue_name: &str) -> Result<Option<QueueItem<serde_json::Value>>, QueueError> {
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
                    let queue_key = self.priority_queue_key(queue_name, priority);
                    let data: Option<String> = conn.rpop(&queue_key, None).await
                        .map_err(|e| QueueError::OperationFailed(format!("Failed to dequeue from priority queue: {}", e)))?;
                    if data.is_some() {
                        item_data = data;
                        break;
                    }
                }
                item_data
            } else {
                // Regular FIFO queue
                let queue_key = self.queue_key(queue_name);
                conn.rpop(&queue_key, None)
                    .await
                    .map_err(|e| QueueError::OperationFailed(format!("Failed to dequeue item: {}", e)))?
            }
        } else {
            // Fallback to regular queue
            let queue_key = self.queue_key(queue_name);
            conn.rpop(&queue_key, None)
                .await
                .map_err(|e| QueueError::OperationFailed(format!("Failed to dequeue item: {}", e)))?
        };

        if let Some(data) = result {
            let item = self.deserialize_item(&data)?;
            let item_id = item.id();
            
            // Move item to processing
            let processing_key = self.processing_key(queue_name);
            let _: () = conn.hset(&processing_key, item_id.to_string(), &data)
                .await
                .map_err(|e| QueueError::OperationFailed(format!("Failed to mark item as processing: {}", e)))?;

            self.log_operation(
                LogLevel::Info, 
                format!("Dequeued item {} from queue {}", item_id, queue_name)
            ).await;

            Ok(Some(item))
        } else {
            Ok(None)
        }
    }

    async fn start_processing(&self, queue_name: &str, item_id: Uuid, worker_id: String) -> Result<(), QueueError> {
        let mut conn = self.conn.write().await;
        let processing_key = self.processing_key(queue_name);
        
        // Update processing metadata
        let metadata_key = format!("{}:worker", processing_key);
        let _: () = conn.hset(&metadata_key, item_id.to_string(), &worker_id)
            .await
            .map_err(|e| QueueError::OperationFailed(format!("Failed to start processing: {}", e)))?;

        self.log_operation(
            LogLevel::Info, 
            format!("Started processing item {} in queue {} by worker {}", item_id, queue_name, worker_id)
        ).await;

        Ok(())
    }

    async fn complete_processing(&self, queue_name: &str, item_id: Uuid, result_data: Option<serde_json::Value>) -> Result<(), QueueError> {
        let mut conn = self.conn.write().await;
        let processing_key = self.processing_key(queue_name);
        
        // Get item data from processing
        let item_data: Option<String> = conn.hget(&processing_key, item_id.to_string())
            .await
            .map_err(|e| QueueError::OperationFailed(format!("Failed to get processing item: {}", e)))?;

        if let Some(data) = item_data {
            // Move to completed
            let completed_key = self.completed_key(queue_name);
            if let Some(result) = result_data {
                let result_json = serde_json::to_string(&result)
                    .map_err(|e| QueueError::SerializationError(format!("Failed to serialize result: {}", e)))?;
                let _: () = conn.hset(&format!("{}:result", completed_key), item_id.to_string(), result_json)
                    .await
                    .map_err(|e| QueueError::OperationFailed(format!("Failed to store result: {}", e)))?;
            }
            
            let _: () = conn.hset(&completed_key, item_id.to_string(), &data)
                .await
                .map_err(|e| QueueError::OperationFailed(format!("Failed to mark as completed: {}", e)))?;

            // Remove from processing
            let _: () = conn.hdel(&processing_key, item_id.to_string())
                .await
                .map_err(|e| QueueError::OperationFailed(format!("Failed to remove from processing: {}", e)))?;

            self.log_operation(
                LogLevel::Info, 
                format!("Completed processing item {} in queue {}", item_id, queue_name)
            ).await;

            Ok(())
        } else {
            Err(QueueError::ItemNotFound(item_id))
        }
    }

    async fn fail_processing(&self, queue_name: &str, item_id: Uuid, error: String) -> Result<bool, QueueError> {
        let mut conn = self.conn.write().await;
        let processing_key = self.processing_key(queue_name);
        
        // Get item data from processing
        let item_data: Option<String> = conn.hget(&processing_key, item_id.to_string())
            .await
            .map_err(|e| QueueError::OperationFailed(format!("Failed to get processing item: {}", e)))?;

        if let Some(data) = item_data {
            let mut item = self.deserialize_item(&data)?;
            
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
                        self.priority_queue_key(queue_name, item.message.priority)
                    } else {
                        self.queue_key(queue_name)
                    }
                } else {
                    self.queue_key(queue_name)
                };

                let serialized = self.serialize_item(&item)?;
                let _: () = conn.lpush(&queue_key, &serialized)
                    .await
                    .map_err(|e| QueueError::OperationFailed(format!("Failed to re-queue item: {}", e)))?;
                
                // Remove from processing
                let _: () = conn.hdel(&processing_key, item_id.to_string())
                    .await
                    .map_err(|e| QueueError::OperationFailed(format!("Failed to remove from processing: {}", e)))?;

                self.log_operation(
                    LogLevel::Warn, 
                    format!("Re-queued item {} for retry ({}/{}) in queue {}: {}", 
                        item_id, current_attempts + 1, max_retries, queue_name, error)
                ).await;

                Ok(true) // Will retry
            } else {
                // Move to failed - max retries exceeded
                let failed_key = self.failed_key(queue_name);
                let _: () = conn.hset(&failed_key, item_id.to_string(), &data)
                    .await
                    .map_err(|e| QueueError::OperationFailed(format!("Failed to mark as failed: {}", e)))?;

                // Store error details
                let _: () = conn.hset(&format!("{}:error", failed_key), item_id.to_string(), &error)
                    .await
                    .map_err(|e| QueueError::OperationFailed(format!("Failed to store error: {}", e)))?;

                // Remove from processing
                let _: () = conn.hdel(&processing_key, item_id.to_string())
                    .await
                    .map_err(|e| QueueError::OperationFailed(format!("Failed to remove from processing: {}", e)))?;

                self.log_operation(
                    LogLevel::Error, 
                    format!("Failed processing item {} in queue {} after {} retries: {}", 
                        item_id, queue_name, max_retries, error)
                ).await;

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
        let processing_key = self.processing_key(queue_name);
        let completed_key = self.completed_key(queue_name);
        let failed_key = self.failed_key(queue_name);

        // Check if this is a priority-based queue
        let config = self.queue_configs.read().await.get(queue_name).cloned();
        
        let pending: usize = if let Some(config) = config {
            if config.priority_based {
                // For priority-based queues, count items in all priority queues
                let mut total_pending = 0;
                for priority in Self::get_priority_levels() {
                    let priority_key = self.priority_queue_key(queue_name, priority);
                    let count: usize = conn.llen(&priority_key).await.unwrap_or(0);
                    total_pending += count;
                }
                total_pending
            } else {
                // Regular FIFO queue
                let queue_key = self.queue_key(queue_name);
                conn.llen(&queue_key).await.unwrap_or(0)
            }
        } else {
            // Fallback to regular queue
            let queue_key = self.queue_key(queue_name);
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
        
        match conn.smembers::<_, Vec<String>>(&queue_list_key).await {
            Ok(queues) => queues,
            Err(_) => Vec::new(),
        }
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
                    let priority_key = self.priority_queue_key(queue_name, priority);
                    let count: usize = conn.llen(&priority_key).await.unwrap_or(0);
                    total_size += count;
                }
                total_size
            } else {
                // Regular FIFO queue
                let queue_key = self.queue_key(queue_name);
                conn.llen(&queue_key).await.unwrap_or(0)
            }
        } else {
            // Fallback to regular queue
            let queue_key = self.queue_key(queue_name);
            conn.llen(&queue_key).await.unwrap_or(0)
        };
        
        Ok(size)
    }

    async fn cleanup_expired(&self) {
        self.log_operation(LogLevel::Info, "Starting cleanup of expired items".to_string()).await;
        
        // Get all queues and check for expired items
        let queues = self.list_queues().await;
        for _queue_name in queues { // Fixed: prefix with underscore to indicate intentional non-use
            // Logic to clean up expired items in each queue
            // TODO: This is a placeholder - actual implementation would depend on how expiration is tracked
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
                self.log_operation(LogLevel::Info, "Redis queue adapter health check passed".to_string()).await;
                Ok(true)
            }
            Err(e) => {
                self.log_operation(
                    LogLevel::Error, 
                    format!("Redis queue adapter health check failed: {}", e)
                ).await;
                Err(QueueError::OperationFailed(format!("Health check failed: {}", e)))
            }
        }
    }
}

#[async_trait]
impl BatchQueuePort for RedisQueueAdapter {
    async fn enqueue_batch<T>(&self, queue_name: &str, items: Vec<QueueItem<T>>) -> Result<Vec<Uuid>, QueueError>
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
            
            let serialized = self.serialize_item(item)?;
            let queue_key = self.queue_key(queue_name);
            pipe.lpush(&queue_key, &serialized);
        }

        let _: () = pipe.query_async(&mut *conn)
            .await
            .map_err(|e| QueueError::OperationFailed(format!("Failed to enqueue batch: {}", e)))?;

        self.log_operation(
            LogLevel::Info, 
            format!("Enqueued batch of {} items to queue {}", items.len(), queue_name)
        ).await;

        Ok(item_ids)
    }

    async fn enqueue_with_priority<T>(&self, queue_name: &str, item: QueueItem<T>, priority: MessagePriority) -> Result<Uuid, QueueError>
    where
        T: Serialize + Clone + for<'de> Deserialize<'de> + Send + Sync, // Fixed: match trait requirements
    {
        if !self.queue_exists(queue_name).await? {
            return Err(QueueError::QueueNotFound(queue_name.to_string()));
        }

        let item_id = item.id();
        let serialized = self.serialize_item(&item)?;
        
        let mut conn = self.conn.write().await;
        let queue_key = self.priority_queue_key(queue_name, priority);

        let _: () = conn.lpush(&queue_key, &serialized)
            .await
            .map_err(|e| QueueError::OperationFailed(format!("Failed to enqueue with priority: {}", e)))?;

        self.log_operation(
            LogLevel::Info, 
            format!("Enqueued item {} to queue {} with priority {:?}", item_id, queue_name, priority)
        ).await;

        Ok(item_id)
    }

    async fn dequeue_batch(&self, queue_name: &str, count: usize) -> Result<Vec<QueueItem<serde_json::Value>>, QueueError> {
        if !self.queue_exists(queue_name).await? {
            return Err(QueueError::QueueNotFound(queue_name.to_string()));
        }

        if count == 0 {
            return Ok(Vec::new());
        }

        let mut conn = self.conn.write().await;
        let mut items = Vec::new();
        let queue_key = self.queue_key(queue_name);
        let processing_key = self.processing_key(queue_name);

        // Simple approach: dequeue items one by one
        // In production, you might want to use a Lua script for atomicity
        for _ in 0..count {
            let data: Option<String> = conn.rpop(&queue_key, None).await
                .map_err(|e| QueueError::OperationFailed(format!("Failed to dequeue batch item: {}", e)))?;
            
            if let Some(item_data) = data {
                let item = self.deserialize_item(&item_data)?;
                let item_id = item.id();
                
                // Mark as processing
                let _: () = conn.hset(&processing_key, item_id.to_string(), &item_data)
                    .await
                    .map_err(|e| QueueError::OperationFailed(format!("Failed to mark batch item as processing: {}", e)))?;
                
                items.push(item);
            } else {
                break; // No more items
            }
        }

        self.log_operation(
            LogLevel::Info, 
            format!("Dequeued batch of {} items from queue {}", items.len(), queue_name)
        ).await;

        Ok(items)
    }

    async fn get_item_summaries(&self, queue_name: &str, item_ids: Vec<Uuid>) -> Result<Vec<QueueItemSummary>, QueueError> {
        if !self.queue_exists(queue_name).await? {
            return Err(QueueError::QueueNotFound(queue_name.to_string()));
        }

        let mut conn = self.conn.write().await;
        let mut summaries = Vec::new();

        // Check in processing, completed, and failed
        let processing_key = self.processing_key(queue_name);
        let completed_key = self.completed_key(queue_name);
        let failed_key = self.failed_key(queue_name);

        for item_id in item_ids {
            let id_str = item_id.to_string();
            
            // Check processing
            if let Ok(Some(_)) = conn.hget::<_, _, Option<String>>(&processing_key, &id_str).await {
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
            if let Ok(Some(_)) = conn.hget::<_, _, Option<String>>(&completed_key, &id_str).await {
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
            if let Ok(Some(_)) = conn.hget::<_, _, Option<String>>(&failed_key, &id_str).await {
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
    async fn enqueue_with_priority<T>(&self, queue_name: &str, item: QueueItem<T>, priority: MessagePriority) -> Result<Uuid, QueueError>
    where
        T: Serialize + Send + Sync, 
    {
        if !self.queue_exists(queue_name).await? {
            return Err(QueueError::QueueNotFound(queue_name.to_string()));
        }

        let item_id = item.action.id; // Use the action's ID directly
        let serialized = self.serialize_item(&item)?;
        
        let mut conn = self.conn.write().await;
        let queue_key = self.priority_queue_key(queue_name, priority);

        let _: () = conn.lpush(&queue_key, &serialized)
            .await
            .map_err(|e| QueueError::OperationFailed(format!("Failed to enqueue with priority: {}", e)))?;

        self.log_operation(
            LogLevel::Info, 
            format!("Enqueued item {} to queue {} with priority {:?}", item_id, queue_name, priority)
        ).await;

        Ok(item_id)
    }

    async fn dequeue_highest_priority(&self, queue_name: &str) -> Result<Option<QueueItem<serde_json::Value>>, QueueError> {
        if !self.queue_exists(queue_name).await? {
            return Err(QueueError::QueueNotFound(queue_name.to_string()));
        }

        let mut conn = self.conn.write().await;
        let processing_key = self.processing_key(queue_name);

        // Try each priority level in order
        for priority in Self::get_priority_levels() {
            let queue_key = self.priority_queue_key(queue_name, priority);
            
            let data: Option<String> = conn.rpop(&queue_key, None).await
                .map_err(|e| QueueError::OperationFailed(format!("Failed to dequeue priority item: {}", e)))?;
            
            if let Some(item_data) = data {
                let item = self.deserialize_item(&item_data)?;
                let item_id = item.id();
                
                // Move to processing
                let _: () = conn.hset(&processing_key, item_id.to_string(), &item_data)
                    .await
                    .map_err(|e| QueueError::OperationFailed(format!("Failed to mark as processing: {}", e)))?;

                self.log_operation(
                    LogLevel::Info, 
                    format!("Dequeued highest priority item {} from queue {} (priority: {:?})", 
                           item_id, queue_name, priority)
                ).await;

                return Ok(Some(item));
            }
        }

        Ok(None)
    }

    async fn get_items_by_priority(&self, queue_name: &str, priority: MessagePriority) -> Result<Vec<QueueItemSummary>, QueueError> {
        if !self.queue_exists(queue_name).await? {
            return Err(QueueError::QueueNotFound(queue_name.to_string()));
        }

        let mut conn = self.conn.write().await;
        let queue_key = self.priority_queue_key(queue_name, priority);

        // Get all items from priority queue without removing them
        let items: Vec<String> = conn.lrange(&queue_key, 0, -1)
            .await
            .map_err(|e| QueueError::OperationFailed(format!("Failed to get priority items: {}", e)))?;

        let mut summaries = Vec::new();
        for data in items {
            if let Ok(item) = self.deserialize_item(&data) {
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
        let meta_key = self.queue_meta_key(queue_name);
        
        let _: () = conn.hset(&meta_key, "paused", "true")
            .await
            .map_err(|e| QueueError::OperationFailed(format!("Failed to pause queue: {}", e)))?;

        self.log_operation(LogLevel::Info, format!("Paused queue: {}", queue_name)).await;
        Ok(())
    }

    async fn resume_queue(&self, queue_name: &str) -> Result<(), QueueError> {
        if !self.queue_exists(queue_name).await? {
            return Err(QueueError::QueueNotFound(queue_name.to_string()));
        }

        let mut conn = self.conn.write().await;
        let meta_key = self.queue_meta_key(queue_name);
        
        let _: () = conn.hset(&meta_key, "paused", "false")
            .await
            .map_err(|e| QueueError::OperationFailed(format!("Failed to resume queue: {}", e)))?;

        self.log_operation(LogLevel::Info, format!("Resumed queue: {}", queue_name)).await;
        Ok(())
    }

    async fn cancel_item(&self, queue_name: &str, item_id: Uuid) -> Result<(), QueueError> {
        let mut conn = self.conn.write().await;
        let processing_key = self.processing_key(queue_name);
        let id_str = item_id.to_string();

        // Check if item is in processing
        let exists: bool = conn.hexists(&processing_key, &id_str)
            .await
            .map_err(|e| QueueError::OperationFailed(format!("Failed to check item existence: {}", e)))?;

        if exists {
            // Remove from processing
            let _: () = conn.hdel(&processing_key, &id_str)
                .await
                .map_err(|e| QueueError::OperationFailed(format!("Failed to cancel item: {}", e)))?;

            self.log_operation(
                LogLevel::Info, 
                format!("Cancelled item {} in queue {}", item_id, queue_name)
            ).await;

            Ok(())
        } else {
            Err(QueueError::ItemNotFound(item_id))
        }
    }

    async fn retry_item(&self, queue_name: &str, item_id: Uuid) -> Result<(), QueueError> {
        let mut conn = self.conn.write().await;
        let failed_key = self.failed_key(queue_name);
        let id_str = item_id.to_string();

        // Get item from failed queue
        let item_data: Option<String> = conn.hget(&failed_key, &id_str)
            .await
            .map_err(|e| QueueError::OperationFailed(format!("Failed to get failed item: {}", e)))?;

        if let Some(data) = item_data {
            let mut item = self.deserialize_item(&data)?;
            
            // Reset attempt count and re-queue
            item.attempt_count = 0;
            let serialized = serde_json::to_string(&item)
                .map_err(|e| QueueError::SerializationError(format!("Failed to serialize for retry: {}", e)))?;

            let queue_key = self.queue_key(queue_name);
            let _: () = conn.lpush(&queue_key, &serialized)
                .await
                .map_err(|e| QueueError::OperationFailed(format!("Failed to retry item: {}", e)))?;

            // Remove from failed
            let _: () = conn.hdel(&failed_key, &id_str)
                .await
                .map_err(|e| QueueError::OperationFailed(format!("Failed to remove from failed: {}", e)))?;

            self.log_operation(
                LogLevel::Info, 
                format!("Retried item {} in queue {}", item_id, queue_name)
            ).await;

            Ok(())
        } else {
            Err(QueueError::ItemNotFound(item_id))
        }
    }

    async fn get_item_details(&self, queue_name: &str, item_id: Uuid) -> Result<QueueItem<serde_json::Value>, QueueError> {
        let mut conn = self.conn.write().await;
        let id_str = item_id.to_string();

        // Check all possible locations
        let keys = vec![
            self.processing_key(queue_name),
            self.completed_key(queue_name),
            self.failed_key(queue_name),
        ];

        for key in keys {
            let item_data: Option<String> = conn.hget(&key, &id_str)
                .await
                .map_err(|e| QueueError::OperationFailed(format!("Failed to get item details: {}", e)))?;
            
            if let Some(data) = item_data {
                return Ok(self.deserialize_item(&data)?);
            }
        }

        // Check in pending queues
        let queue_key = self.queue_key(queue_name);
        let items: Vec<String> = conn.lrange(&queue_key, 0, -1)
            .await
            .map_err(|e| QueueError::OperationFailed(format!("Failed to search queue: {}", e)))?;

        for data in items {
            if let Ok(item) = self.deserialize_item(&data) {
                if item.id() == item_id {
                    return Ok(item);
                }
            }
        }

        Err(QueueError::ItemNotFound(item_id))
    }

    async fn purge_completed(&self, queue_name: &str) -> Result<usize, QueueError> {
        let mut conn = self.conn.write().await;
        let completed_key = self.completed_key(queue_name);
        
        let count: usize = conn.hlen(&completed_key)
            .await
            .map_err(|e| QueueError::OperationFailed(format!("Failed to get completed count: {}", e)))?;

        let _: () = conn.del(&completed_key)
            .await
            .map_err(|e| QueueError::OperationFailed(format!("Failed to purge completed: {}", e)))?;

        self.log_operation(
            LogLevel::Info, 
            format!("Purged {} completed items from queue {}", count, queue_name)
        ).await;

        Ok(count)
    }

    async fn purge_failed(&self, queue_name: &str) -> Result<usize, QueueError> {
        let mut conn = self.conn.write().await;
        let failed_key = self.failed_key(queue_name);
        
        let count: usize = conn.hlen(&failed_key)
            .await
            .map_err(|e| QueueError::OperationFailed(format!("Failed to get failed count: {}", e)))?;

        let _: () = conn.del(&failed_key)
            .await
            .map_err(|e| QueueError::OperationFailed(format!("Failed to purge failed: {}", e)))?;

        self.log_operation(
            LogLevel::Info, 
            format!("Purged {} failed items from queue {}", count, queue_name)
        ).await;

        Ok(count)
    }

    async fn get_queue_config(&self, queue_name: &str) -> Result<QueueConfig, QueueError> {
        if let Some(config) = self.queue_configs.read().await.get(queue_name) {
            Ok(config.clone())
        } else {
            // Try to load from Redis
            let mut conn = self.conn.write().await;
            let meta_key = self.queue_meta_key(queue_name);
            
            let config_json: Option<String> = conn.hget(&meta_key, "config")
                .await
                .map_err(|e| QueueError::OperationFailed(format!("Failed to get queue config: {}", e)))?;
            
            if let Some(json) = config_json {
                let config: QueueConfig = serde_json::from_str(&json)
                    .map_err(|e| QueueError::SerializationError(format!("Failed to deserialize config: {}", e)))?;
                
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

    async fn update_queue_config(&self, queue_name: &str, config: QueueConfig) -> Result<(), QueueError> {
        if !self.queue_exists(queue_name).await? {
            return Err(QueueError::QueueNotFound(queue_name.to_string()));
        }

        let mut conn = self.conn.write().await;
        let meta_key = self.queue_meta_key(queue_name);
        let config_json = serde_json::to_string(&config)
            .map_err(|e| QueueError::SerializationError(format!("Failed to serialize config: {}", e)))?;

        let _: () = conn.hset(&meta_key, "config", config_json)
            .await
            .map_err(|e| QueueError::OperationFailed(format!("Failed to update config: {}", e)))?;

        // Update cache
        {
            let mut configs = self.queue_configs.write().await;
            configs.insert(queue_name.to_string(), config);
        }

        self.log_operation(
            LogLevel::Info, 
            format!("Updated configuration for queue {}", queue_name)
        ).await;

        Ok(())
    }
}

// Implement FullQueuePort
impl FullQueuePort for RedisQueueAdapter {}

impl RedisQueueAdapter {
    /// Shutdown the adapter and close connections
    pub async fn shutdown(&self) -> Result<(), QueueError> {
        self.log_operation(LogLevel::Info, "Shutting down Redis queue adapter".to_string()).await;
        // Connection will be automatically closed when dropped
        Ok(())
    }

    /// Get Redis connection info for debugging
    pub fn get_connection_info(&self) -> String {
        format!("{}:{}/{}", self.config.redis_host, self.config.redis_port, self.config.redis_db)
    }
}
