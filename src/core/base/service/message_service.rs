/*
Message Service

The core base-level message service that provides the most generalized messaging functionality.
This service handles message routing, delivery, and basic message operations that other 
messaging services can build upon.

This is the foundation service for:
- Log Service (LogEntry messages)
- Notification Service (Notification messages)
- Event Service (Event messages)
- Any other messaging-based services

All platform and application level messaging services should extend this base service.
*/
use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::core::base::entity::message::{Message, Location, MessagePriority};

/// Result type for message operations
pub type MessageResult<T> = Result<T, MessageError>;

/// Errors that can occur in message operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum MessageError {
    #[error("Invalid destination: {0}")]
    InvalidDestination(String),
    
    #[error("Message delivery failed: {0}")]
    DeliveryFailed(String),
    
    #[error("Handler not found for destination: {0}")]
    HandlerNotFound(String),
    
    #[error("Message routing failed: {0}")]
    RoutingFailed(String),
    
    #[error("Message serialization failed: {0}")]
    SerializationFailed(String),
    
    #[error("Message queue full")]
    QueueFull,
    
    #[error("Service unavailable")]
    ServiceUnavailable,
    
    #[error("Permission denied for destination: {0}")]
    PermissionDenied(String),
    
    #[error("Message expired")]
    MessageExpired,
    
    #[error("Invalid message format: {0}")]
    InvalidFormat(String),
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Statistics for message operations
#[derive(Debug, Clone, Default)]
pub struct MessageStats {
    /// Total messages sent
    pub messages_sent: u64,
    /// Total messages delivered
    pub messages_delivered: u64,
    /// Total messages failed
    pub messages_failed: u64,
    /// Messages by priority
    pub messages_by_priority: HashMap<MessagePriority, u64>,
    /// Messages by destination type
    pub destination_stats: HashMap<String, u64>,
    /// Average delivery time in milliseconds
    pub avg_delivery_time_ms: Option<u64>,
    /// Last activity timestamp
    pub last_activity: Option<DateTime<Utc>>,
}

/// Configuration for the message service
#[derive(Debug, Clone)]
pub struct MessageServiceConfig {
    /// Maximum number of pending messages per queue
    pub max_queue_size: usize,
    /// Default message TTL in seconds
    pub default_ttl_seconds: i64,
    /// Whether to enable message persistence
    pub enable_persistence: bool,
    /// Number of worker threads for message processing
    pub worker_threads: usize,
    /// Retry attempts for failed messages
    pub retry_attempts: u32,
    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,
}

impl Default for MessageServiceConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 10000,
            default_ttl_seconds: 3600, // 1 hour
            enable_persistence: false,
            worker_threads: 4,
            retry_attempts: 3,
            retry_delay_ms: 1000,
        }
    }
}

/// Trait for message handlers
#[async_trait]
pub trait MessageHandler<T>: Send + Sync {
    /// Handle a message
    async fn handle_message(&self, message: Message<T>) -> MessageResult<()>;
    
    /// Get the destinations this handler can process
    fn supported_destinations(&self) -> Vec<Location>;
    
    /// Check if this handler can process messages to the given destination
    fn can_handle(&self, destination: &Location) -> bool {
        self.supported_destinations().contains(destination)
    }
}

/// Message delivery receipt
#[derive(Debug, Clone)]
pub struct DeliveryReceipt {
    /// Original message ID
    pub message_id: Uuid,
    /// Delivery status
    pub status: DeliveryStatus,
    /// Delivery timestamp
    pub delivered_at: DateTime<Utc>,
    /// Delivery details or error message
    pub details: Option<String>,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
}

/// Status of message delivery
#[derive(Debug, Clone, PartialEq)]
pub enum DeliveryStatus {
    /// Message delivered successfully
    Delivered,
    /// Message delivery failed
    Failed,
    /// Message delivery pending
    Pending,
    /// Message expired before delivery
    Expired,
    /// Message delivery was retried
    Retried,
}

/// Core message service implementation
pub struct MessageService {
    /// Service configuration
    config: MessageServiceConfig,
    /// Message handlers by destination type
    handlers: Arc<RwLock<HashMap<String, Arc<dyn MessageHandler<serde_json::Value>>>>>,
    /// Message queues by destination
    queues: Arc<RwLock<HashMap<Location, mpsc::UnboundedSender<Message<serde_json::Value>>>>>,
    /// Service statistics
    stats: Arc<RwLock<MessageStats>>,
    /// Active workers
    workers: Arc<RwLock<Vec<tokio::task::JoinHandle<()>>>>,
}

impl MessageService {
    /// Create a new message service
    pub fn new(config: MessageServiceConfig) -> Self {
        Self {
            config,
            handlers: Arc::new(RwLock::new(HashMap::new())),
            queues: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(MessageStats::default())),
            workers: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Start the message service
    pub async fn start(&self) -> MessageResult<()> {
        // Start worker threads
        let mut workers = self.workers.write().await;
        
        for i in 0..self.config.worker_threads {
            let worker = self.start_worker(i).await;
            workers.push(worker);
        }
        
        Ok(())
    }
    
    /// Stop the message service
    pub async fn stop(&self) -> MessageResult<()> {
        let mut workers = self.workers.write().await;
        
        // Cancel all workers
        for worker in workers.drain(..) {
            worker.abort();
        }
        
        Ok(())
    }
    
    /// Register a message handler for specific destinations
    pub async fn register_handler<T>(&self, 
        destination_type: String, 
        handler: Arc<dyn MessageHandler<T>>
    ) -> MessageResult<()> 
    where
        T: serde::Serialize + serde::de::DeserializeOwned + Send + Sync + 'static,
    {
        // Create a wrapper that handles the generic type conversion
        let wrapper = GenericHandlerWrapper::new(handler);
        
        let mut handlers = self.handlers.write().await;
        handlers.insert(destination_type, Arc::new(wrapper));
        
        Ok(())
    }
    
    /// Send a message
    pub async fn send_message<T>(&self, message: Message<T>) -> MessageResult<DeliveryReceipt>
    where
        T: serde::Serialize + Send + Sync,
    {
        let start_time = std::time::Instant::now();
        
        // Check if message is expired
        if message.is_expired(self.config.default_ttl_seconds) {
            return Ok(DeliveryReceipt {
                message_id: message.id,
                status: DeliveryStatus::Expired,
                delivered_at: Utc::now(),
                details: Some("Message expired before delivery".to_string()),
                processing_time_ms: start_time.elapsed().as_millis() as u64,
            });
        }
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.messages_sent += 1;
            *stats.messages_by_priority.entry(message.priority).or_insert(0) += 1;
            *stats.destination_stats.entry(message.destination.to_string()).or_insert(0) += 1;
            stats.last_activity = Some(Utc::now());
        }
        
        // Convert message to generic format
        let json_value = serde_json::to_value(&message.message)
            .map_err(|e| MessageError::SerializationFailed(e.to_string()))?;
        
        let generic_message = Message {
            id: message.id,
            source: message.source,
            destination: message.destination.clone(),
            timestamp: message.timestamp,
            message: json_value,
            correlation_id: message.correlation_id,
            priority: message.priority,
        };
        
        // Try to deliver immediately if handler is available
        if let Some(handler) = self.find_handler(&message.destination).await {
            match handler.handle_message(generic_message).await {
                Ok(()) => {
                    let mut stats = self.stats.write().await;
                    stats.messages_delivered += 1;
                    
                    return Ok(DeliveryReceipt {
                        message_id: message.id,
                        status: DeliveryStatus::Delivered,
                        delivered_at: Utc::now(),
                        details: None,
                        processing_time_ms: start_time.elapsed().as_millis() as u64,
                    });
                }
                Err(e) => {
                    let mut stats = self.stats.write().await;
                    stats.messages_failed += 1;
                    
                    return Ok(DeliveryReceipt {
                        message_id: message.id,
                        status: DeliveryStatus::Failed,
                        delivered_at: Utc::now(),
                        details: Some(e.to_string()),
                        processing_time_ms: start_time.elapsed().as_millis() as u64,
                    });
                }
            }
        }
        
        // Queue the message for later processing
        self.queue_message(generic_message).await?;
        
        Ok(DeliveryReceipt {
            message_id: message.id,
            status: DeliveryStatus::Pending,
            delivered_at: Utc::now(),
            details: Some("Message queued for processing".to_string()),
            processing_time_ms: start_time.elapsed().as_millis() as u64,
        })
    }
    
    /// Send multiple messages
    pub async fn send_messages<T>(&self, messages: Vec<Message<T>>) -> MessageResult<Vec<DeliveryReceipt>>
    where
        T: serde::Serialize + Send + Sync,
    {
        let mut receipts = Vec::new();
        
        for message in messages {
            let receipt = self.send_message(message).await?;
            receipts.push(receipt);
        }
        
        Ok(receipts)
    }
    
    /// Get service statistics
    pub async fn get_stats(&self) -> MessageStats {
        let stats = self.stats.read().await;
        stats.clone()
    }
    
    /// Get list of registered destinations
    pub async fn list_destinations(&self) -> Vec<String> {
        let handlers = self.handlers.read().await;
        handlers.keys().cloned().collect()
    }
    
    /// Check service health
    pub async fn health_check(&self) -> MessageResult<bool> {
        let handlers = self.handlers.read().await;
        let queues = self.queues.read().await;
        
        // Service is healthy if we have handlers and queues are not overflowing
        Ok(!handlers.is_empty() && queues.len() < self.config.max_queue_size)
    }
    
    // Private helper methods
    
    /// Find a handler for the given destination
    async fn find_handler(&self, destination: &Location) -> Option<Arc<dyn MessageHandler<serde_json::Value>>> {
        let handlers = self.handlers.read().await;
        
        // Try exact match first
        if let Some(handler) = handlers.get(&destination.to_string()) {
            return Some(handler.clone());
        }
        
        // Try by destination type
        let dest_type = match destination {
            Location::System(_) => "system",
            Location::Service(_) => "service",
            Location::External(_) => "external",
            Location::User(_) => "user",
        };
        
        handlers.get(dest_type).cloned()
    }
    
    /// Queue a message for later processing
    async fn queue_message(&self, message: Message<serde_json::Value>) -> MessageResult<()> {
        let mut queues = self.queues.write().await;
        
        let sender = queues.entry(message.destination.clone())
            .or_insert_with(|| {
                let (tx, mut rx) = mpsc::unbounded_channel::<Message<serde_json::Value>>();
                
                // Spawn processor for this queue
                let handlers = self.handlers.clone();
                let stats = self.stats.clone();
                
                tokio::spawn(async move {
                    while let Some(msg) = rx.recv().await {
                        // Process message
                        if let Some(handler) = Self::find_handler_static(&handlers, &msg.destination).await {
                            match handler.handle_message(msg).await {
                                Ok(()) => {
                                    let mut stats = stats.write().await;
                                    stats.messages_delivered += 1;
                                }
                                Err(_) => {
                                    let mut stats = stats.write().await;
                                    stats.messages_failed += 1;
                                }
                            }
                        }
                    }
                });
                
                tx
            });
        
        sender.send(message)
            .map_err(|_| MessageError::QueueFull)?;
        
        Ok(())
    }
    
    /// Static version of find_handler for use in spawned tasks
    async fn find_handler_static(
        handlers: &Arc<RwLock<HashMap<String, Arc<dyn MessageHandler<serde_json::Value>>>>>,
        destination: &Location
    ) -> Option<Arc<dyn MessageHandler<serde_json::Value>>> {
        let handlers_guard = handlers.read().await;
        
        if let Some(handler) = handlers_guard.get(&destination.to_string()) {
            return Some(handler.clone());
        }
        
        let dest_type = match destination {
            Location::System(_) => "system",
            Location::Service(_) => "service", 
            Location::External(_) => "external",
            Location::User(_) => "user",
        };
        
        handlers_guard.get(dest_type).cloned()
    }
    
    /// Start a worker thread
    async fn start_worker(&self, _worker_id: usize) -> tokio::task::JoinHandle<()> {
        // For now, workers are handled by the queue processors
        // This can be expanded for more complex worker patterns
        tokio::spawn(async move {
            // Worker logic would go here
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        })
    }
}

/// Wrapper to handle generic type conversion for handlers
struct GenericHandlerWrapper<T> {
    inner: Arc<dyn MessageHandler<T>>,
}

impl<T> GenericHandlerWrapper<T>
where
    T: serde::de::DeserializeOwned + Send + Sync + 'static,
{
    fn new(handler: Arc<dyn MessageHandler<T>>) -> Self {
        Self { inner: handler }
    }
}

#[async_trait]
impl<T> MessageHandler<serde_json::Value> for GenericHandlerWrapper<T>
where
    T: serde::de::DeserializeOwned + Send + Sync + 'static,
{
    async fn handle_message(&self, message: Message<serde_json::Value>) -> MessageResult<()> {
        // Convert JSON value back to the specific type
        let typed_message = serde_json::from_value::<T>(message.message)
            .map_err(|e| MessageError::SerializationFailed(e.to_string()))?;
        
        let converted_message = Message {
            id: message.id,
            source: message.source,
            destination: message.destination,
            timestamp: message.timestamp,
            message: typed_message,
            correlation_id: message.correlation_id,
            priority: message.priority,
        };
        
        self.inner.handle_message(converted_message).await
    }
    
    fn supported_destinations(&self) -> Vec<Location> {
        self.inner.supported_destinations()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHandler;
    
    #[async_trait]
    impl MessageHandler<String> for TestHandler {
        async fn handle_message(&self, _message: Message<String>) -> MessageResult<()> {
            Ok(())
        }
        
        fn supported_destinations(&self) -> Vec<Location> {
            vec![Location::service("test")]
        }
    }

    #[tokio::test]
    async fn test_message_service_creation() {
        let config = MessageServiceConfig::default();
        let service = MessageService::new(config);
        
        assert!(service.health_check().await.is_ok());
    }

    #[tokio::test]
    async fn test_handler_registration() {
        let service = MessageService::new(MessageServiceConfig::default());
        let handler = Arc::new(TestHandler);
        
        let result = service.register_handler("test".to_string(), handler).await;
        assert!(result.is_ok());
        
        let destinations = service.list_destinations().await;
        assert!(destinations.contains(&"test".to_string()));
    }

    #[tokio::test]
    async fn test_message_sending() {
        let service = MessageService::new(MessageServiceConfig::default());
        let handler = Arc::new(TestHandler);
        service.register_handler("service".to_string(), handler).await.unwrap();
        
        let message = Message::new(
            Location::system("test-system"),
            Location::service("test"),
            "Hello, World!".to_string(),
        );
        
        let receipt = service.send_message(message).await.unwrap();
        assert_eq!(receipt.status, DeliveryStatus::Delivered);
    }
}