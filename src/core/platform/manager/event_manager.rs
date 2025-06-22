use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::RwLock;
use thiserror::Error;

// Fixed imports - now importing Event from base component
use crate::core::base::component::event::Event;
use crate::core::base::entity::message::{Message, Location, MessagePriority};
use crate::core::base::service::message_service::{MessageService, MessageHandler, MessageResult, MessageError};

/// Defines the interface for handling events asynchronously.
#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: &Event) -> Result<(), EventError>;
}

/// Custom error type for event-related operations.
#[derive(Debug, Error)]
pub enum EventError {
    #[error("Failed to publish event: {0}")]
    PublishError(String),
    #[error("Failed to subscribe to event: {0}")]
    SubscribeError(String),
    #[error("Handler error: {0}")]
    HandlerError(String),
}

/// Handles messages from the MessageService and routes them to event subscribers.
struct EventMessageHandler {
    subscribers: Arc<RwLock<HashMap<String, Vec<Arc<dyn EventHandler>>>>>,
}

#[async_trait]
impl MessageHandler<Event> for EventMessageHandler {
    async fn handle_message(&self, message: Message<Event>) -> MessageResult<()> {
        if let Location::Service(name) = &message.destination {
            if name.starts_with("event:") {
                let event_type = name.strip_prefix("event:").unwrap_or("");
                let subscribers = self.subscribers.read().await;
                if let Some(handlers) = subscribers.get(event_type) {
                    for handler in handlers {
                        handler.handle(&message.message).await
                            .map_err(|e| MessageError::DeliveryFailed(e.to_string()))?;
                    }
                }
            }
        }
        Ok(())
    }

    fn supported_destinations(&self) -> Vec<Location> {
        vec![Location::Service("event:*".to_string())]
    }
}

/// Manages event publishing and subscription using the MessageService.
/// 
/// This service provides a high-level interface for event-driven communication
/// within the In4me system. It handles event routing, subscription management,
/// and integrates with the underlying MessageService for transport.
pub struct EventService {
    message_service: Arc<MessageService>,
    subscribers: Arc<RwLock<HashMap<String, Vec<Arc<dyn EventHandler>>>>>,
}

impl EventService {
    /// Creates a new EventService instance and registers its handler with the MessageService.
    pub async fn new(message_service: Arc<MessageService>) -> Result<Self, EventError> {
        let subscribers = Arc::new(RwLock::new(HashMap::new()));
        let handler = Arc::new(EventMessageHandler {
            subscribers: subscribers.clone(),
        });

        // Register the handler for "service" destination type to catch all "event:" messages
        message_service
            .register_handler("service".to_string(), handler)
            .await
            .map_err(|e| EventError::SubscribeError(e.to_string()))?;

        Ok(Self {
            message_service,
            subscribers,
        })
    }

    /// Publishes an event to the MessageService.
    pub async fn publish(&self, event: Event) -> Result<(), EventError> {
        let destination = Location::Service(format!("event:{}", event.event_type));
        let message = Message {
            id: event.id,
            source: Location::Service(event.source.clone()),
            destination,
            timestamp: event.timestamp,
            message: event,
            correlation_id: None,
            priority: MessagePriority::Normal,
        };

        self.message_service
            .send_message(message)
            .await
            .map_err(|e| EventError::PublishError(e.to_string()))?;
        Ok(())
    }

    /// Publishes multiple events in batch.
    pub async fn publish_batch(&self, events: Vec<Event>) -> Result<(), EventError> {
        for event in events {
            self.publish(event).await?;
        }
        Ok(())
    }

    /// Subscribes a handler to a specific event type.
    pub async fn subscribe(&self, event_type: &str, handler: Arc<dyn EventHandler>) -> Result<(), EventError> {
        let mut subscribers = self.subscribers.write().await;
        subscribers
            .entry(event_type.to_string())
            .or_insert_with(Vec::new)
            .push(handler);
        Ok(())
    }

    /// Subscribes a handler to multiple event types.
    pub async fn subscribe_multiple(&self, event_types: Vec<&str>, handler: Arc<dyn EventHandler>) -> Result<(), EventError> {
        for event_type in event_types {
            self.subscribe(event_type, handler.clone()).await?;
        }
        Ok(())
    }

    /// Unsubscribes all handlers from a specific event type.
    pub async fn unsubscribe(&self, event_type: &str) -> Result<(), EventError> {
        let mut subscribers = self.subscribers.write().await;
        subscribers.remove(event_type);
        Ok(())
    }

    /// Lists all subscribed event types.
    pub async fn list_subscriptions(&self) -> Vec<String> {
        let subscribers = self.subscribers.read().await;
        subscribers.keys().cloned().collect()
    }

    /// Gets the number of subscribers for a specific event type.
    pub async fn subscriber_count(&self, event_type: &str) -> usize {
        let subscribers = self.subscribers.read().await;
        subscribers.get(event_type).map(|v| v.len()).unwrap_or(0)
    }

    /// Checks if there are any subscribers for a specific event type.
    pub async fn has_subscribers(&self, event_type: &str) -> bool {
        self.subscriber_count(event_type).await > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};
    use serde_json::json;

    /// Mock event handler for testing.
    struct MockHandler {
        received: Arc<RwLock<Vec<Event>>>,
    }

    #[async_trait]
    impl EventHandler for MockHandler {
        async fn handle(&self, event: &Event) -> Result<(), EventError> {
            let mut received = self.received.write().await;
            received.push(event.clone());
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_event_publish_and_subscribe() {
        let message_service = Arc::new(MessageService::new(Default::default()));
        message_service.start().await.unwrap();
        
        let event_service = EventService::new(message_service.clone())
            .await
            .unwrap();

        let received_events = Arc::new(RwLock::new(Vec::new()));
        let handler = Arc::new(MockHandler {
            received: received_events.clone(),
        });

        // Subscribe to "content_ingested" events
        event_service.subscribe("content_ingested", handler).await.unwrap();

        // Publish an event
        let event = Event::new(
            "content_ingested".to_string(),
            json!({"url": "https://example.com"}),
            "test_source".to_string(),
        );
        event_service.publish(event.clone()).await.unwrap();

        // Wait for async processing
        sleep(Duration::from_millis(100)).await;

        // Verify the event was received
        let received = received_events.read().await;
        assert_eq!(received.len(), 1);
        assert_eq!(received[0].event_type, "content_ingested");
        assert_eq!(received[0].payload, json!({"url": "https://example.com"}));
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let message_service = Arc::new(MessageService::new(Default::default()));
        message_service.start().await.unwrap();
        
        let event_service = EventService::new(message_service.clone())
            .await
            .unwrap();

        let received_events_1 = Arc::new(RwLock::new(Vec::new()));
        let received_events_2 = Arc::new(RwLock::new(Vec::new()));
        
        let handler_1 = Arc::new(MockHandler { received: received_events_1.clone() });
        let handler_2 = Arc::new(MockHandler { received: received_events_2.clone() });

        // Subscribe both handlers to the same event type
        event_service.subscribe("test_event", handler_1).await.unwrap();
        event_service.subscribe("test_event", handler_2).await.unwrap();

        // Publish an event
        let event = Event::new(
            "test_event".to_string(),
            json!({"data": "test"}),
            "test_source".to_string(),
        );
        event_service.publish(event).await.unwrap();

        // Wait for async processing
        sleep(Duration::from_millis(100)).await;

        // Verify both handlers received the event
        let received_1 = received_events_1.read().await;
        let received_2 = received_events_2.read().await;
        assert_eq!(received_1.len(), 1);
        assert_eq!(received_2.len(), 1);
    }

    #[tokio::test]
    async fn test_batch_publish() {
        let message_service = Arc::new(MessageService::new(Default::default()));
        message_service.start().await.unwrap();
        
        let event_service = EventService::new(message_service.clone())
            .await
            .unwrap();

        let received_events = Arc::new(RwLock::new(Vec::new()));
        let handler = Arc::new(MockHandler {
            received: received_events.clone(),
        });

        event_service.subscribe("batch_event", handler).await.unwrap();

        // Publish multiple events in batch
        let events = vec![
            Event::new("batch_event".to_string(), json!({"id": 1}), "test".to_string()),
            Event::new("batch_event".to_string(), json!({"id": 2}), "test".to_string()),
            Event::new("batch_event".to_string(), json!({"id": 3}), "test".to_string()),
        ];
        
        event_service.publish_batch(events).await.unwrap();

        sleep(Duration::from_millis(100)).await;

        let received = received_events.read().await;
        assert_eq!(received.len(), 3);
    }

    #[tokio::test]
    async fn test_subscriber_count() {
        let message_service = Arc::new(MessageService::new(Default::default()));
        message_service.start().await.unwrap();
        
        let event_service = EventService::new(message_service.clone())
            .await
            .unwrap();

        let handler = Arc::new(MockHandler {
            received: Arc::new(RwLock::new(Vec::new())),
        });

        assert_eq!(event_service.subscriber_count("test_event").await, 0);
        assert!(!event_service.has_subscribers("test_event").await);

        event_service.subscribe("test_event", handler.clone()).await.unwrap();
        assert_eq!(event_service.subscriber_count("test_event").await, 1);
        assert!(event_service.has_subscribers("test_event").await);

        event_service.subscribe("test_event", handler).await.unwrap();
        assert_eq!(event_service.subscriber_count("test_event").await, 2);
    }
}