#[cfg(test)]
mod queue_integration_tests {
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::sleep;
    use uuid::Uuid;
    use serde_json::json;

    use testcontainers::{
        core::{IntoContainerPort, WaitFor}, 
        runners::AsyncRunner, 
        GenericImage
    };

    use in4me::infrastructure::adapters::queue::redis::{RedisQueueAdapter, RedisQueueConfig};
    use in4me::infrastructure::adapters::logs::system_log_adapter::SystemLogAdapter;
    use in4me::application::ports::output::queue_port::{QueuePort, BatchQueuePort, PriorityQueuePort, QueueManagementPort};
    use in4me::application::ports::output::log_port::LogPort;
    use in4me::core::platform::container::queue_item::{QueueItem, QueueItemConfig};
    use in4me::core::platform::manager::queue_service::{QueueConfig, QueueError};
    use in4me::core::base::entity::message::{Message, Location, MessagePriority};

    // ToDo consider setting up with testcontainers_modules instead of Generic
    struct TestContext {
        adapter: Arc<RedisQueueAdapter>,
        container: testcontainers::ContainerAsync<GenericImage>,
        port: u16,
    }

    impl TestContext {
        async fn new() -> Result<Self, Box<dyn std::error::Error>> {
            let container = GenericImage::new("redis", "7.2.4")
                .with_exposed_port(6379.tcp())
                .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
                .start()
                .await
                .expect("Failed to start Redis");
            
            let port = container.get_host_port_ipv4(6379).await?;

            // Wait a bit for Redis to start
            sleep(Duration::from_millis(500)).await;

            let log_adapter = Arc::new(SystemLogAdapter::new(Default::default()).unwrap()) as Arc<dyn LogPort>; // Fixed unwrap
            
            let redis_config = RedisQueueConfig {
                redis_host: "localhost".to_string(),
                redis_port: port,
                redis_password: None,
                redis_db: 0,
                connection_timeout: 10,
                key_prefix: "test:queue".to_string(),
                max_retries: 5,
            };

            let adapter = Arc::new(
                RedisQueueAdapter::new(redis_config, Some(log_adapter))
                    .await?
            );

            Ok(TestContext {
                adapter,
                container: container,
                port,
            })
        }

        fn create_test_queue_item(&self, payload: serde_json::Value) -> QueueItem<serde_json::Value> {
            let message = Message::new(
                Location::service("test-producer"),
                Location::service("test-consumer"),
                payload,
            );

            QueueItem::new(
                "default-queue".to_string(), // ← Generic name, will be overridden when enqueuing
                message,
                Some(QueueItemConfig {
                    max_retries: 5,
                    retry_delay_ms: 0,
                    ttl_seconds: 0,
                    timeout_seconds: 30,
                    preserve_after_completion: false,
                }),
            )
        }
        
        fn create_queue_item_for(&self, queue_name: &str, payload: serde_json::Value) -> QueueItem<serde_json::Value> {
            let message = Message::new(
                Location::service("test-producer"),
                Location::service("test-consumer"),
                payload,
            );

            QueueItem::new(
                queue_name.to_string(),
                message,
                Some(QueueItemConfig {
                    max_retries: 5,
                    retry_delay_ms: 0,
                    ttl_seconds: 0,
                    timeout_seconds: 30,
                    preserve_after_completion: false,
                }),
            )
        }

        fn create_priority_queue_item(&self, payload: serde_json::Value, priority: MessagePriority) -> QueueItem<serde_json::Value> {
            let mut message = Message::new(
                Location::service("test-producer"),
                Location::service("test-consumer"),
                payload,
            );
            message.priority = priority;

            QueueItem::new(
                "workflow-queue".to_string(), // ← Make sure this matches the queue name used in the test
                message,
                Some(QueueItemConfig {
                    max_retries: 5, // ← This should be > 1 to allow retries
                    retry_delay_ms: 100,
                    ttl_seconds: 0,
                    timeout_seconds: 30,
                    preserve_after_completion: false,
                }),
            )
        }
    }

    #[tokio::test]
    async fn test_redis_connection_and_health_check() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = TestContext::new().await?;
        
        // Test health check
        let health_result = ctx.adapter.health_check().await;
        assert!(health_result.is_ok());
        assert!(health_result.unwrap());

        Ok(())
    }

    #[tokio::test]
    async fn test_direct_redis_operations() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = TestContext::new().await?;

        println!("=== DIRECT REDIS DEBUG ===");
        
        // Test direct Redis connection
        let health = ctx.adapter.health_check().await?;
        println!("Health check: {}", health);

        // Create queue and check Redis keys directly
        ctx.adapter.create_queue("direct-test".to_string(), None).await?;
        
        // Try manual Redis operations to see what's happening
        // This requires access to the Redis connection - you might need to add a debug method
        
        let test_payload = json!({"test": "direct"});
        let queue_item = ctx.create_test_queue_item(test_payload.clone());
        let item_id = queue_item.id();
        
        println!("=== Before enqueue ===");
        println!("Item ID: {}", item_id);
        
        // Enqueue
        let result = ctx.adapter.enqueue("direct-test", queue_item).await;
        println!("Enqueue result: {:?}", result);
        
        if let Ok(enqueued_id) = result {
            println!("Enqueued ID: {}", enqueued_id);
            
            // Check immediately
            let length = ctx.adapter.queue_length("direct-test").await?;
            println!("Queue length immediately after enqueue: {}", length);
            
            let stats = ctx.adapter.get_queue_stats("direct-test").await?;
            println!("Stats: pending={}, processing={}, completed={}, failed={}", 
                     stats.pending_items, stats.processing_items, stats.completed_items, stats.failed_items);
        }

        Ok(())
    }

    #[tokio::test] 
    async fn test_queue_creation_and_deletion() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = TestContext::new().await?;

        // Test queue creation
        let queue_config = QueueConfig {
            max_capacity: 1000,
            priority_based: true,
            ..Default::default()
        };

        ctx.adapter.create_queue("test-queue".to_string(), Some(queue_config.clone())).await?;

        // Verify queue exists in list
        let queues = ctx.adapter.list_queues().await;
        assert!(queues.contains(&"test-queue".to_string()));

        // Test getting queue config
        let retrieved_config = ctx.adapter.get_queue_config("test-queue").await?;
        assert_eq!(retrieved_config.max_capacity, 1000);
        assert!(retrieved_config.priority_based);

        // Test queue deletion
        ctx.adapter.delete_queue("test-queue").await?;

        // Verify queue no longer exists
        let queues_after_delete = ctx.adapter.list_queues().await;
        assert!(!queues_after_delete.contains(&"test-queue".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_enqueue_and_dequeue_operations() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = TestContext::new().await?;

        // Create test queue
        ctx.adapter.create_queue("test-queue".to_string(), None).await?;

        // Test enqueue
        let test_payload = json!({"message": "Hello, Redis!", "timestamp": 12345});
        let queue_item = ctx.create_test_queue_item(test_payload.clone());
        let item_id = queue_item.id();

        println!("Enqueueing item with ID: {}", item_id);
        let enqueued_id = ctx.adapter.enqueue("test-queue", queue_item).await?;
        assert_eq!(enqueued_id, item_id);

        // Add a small delay to ensure Redis operations complete
        sleep(Duration::from_millis(100)).await;

        // Test queue length
        let length = ctx.adapter.queue_length("test-queue").await?;
        println!("Queue length after enqueue: {}", length);
        
        if length == 0 {
            // Debug information
            let stats = ctx.adapter.get_queue_stats("test-queue").await?;
            println!("Queue stats: pending={}, processing={}, completed={}, failed={}", 
                     stats.pending_items, stats.processing_items, stats.completed_items, stats.failed_items);
            
            // Check if item might be in a different state
            let queues = ctx.adapter.list_queues().await;
            println!("Available queues: {:?}", queues);
        }
        
        // Make assertion more descriptive
        assert_eq!(length, 1, "Expected queue length to be 1 after enqueue, but got {}. Item ID: {}", length, item_id);

        // Test dequeue
        let dequeued_item = ctx.adapter.dequeue("test-queue").await?;
        
        if dequeued_item.is_none() {
            println!("Dequeue returned None, checking queue state...");
            let stats_after_dequeue_attempt = ctx.adapter.get_queue_stats("test-queue").await?;
            println!("Stats after dequeue attempt: pending={}, processing={}, completed={}, failed={}", 
                     stats_after_dequeue_attempt.pending_items, 
                     stats_after_dequeue_attempt.processing_items, 
                     stats_after_dequeue_attempt.completed_items, 
                     stats_after_dequeue_attempt.failed_items);
        }
        
        assert!(dequeued_item.is_some(), "Expected to dequeue an item, but got None");

        let item = dequeued_item.unwrap();
        assert_eq!(item.id(), item_id);
        assert_eq!(item.message.message, test_payload);

        // Queue should now be empty
        let length_after = ctx.adapter.queue_length("test-queue").await?;
        assert_eq!(length_after, 0);

        Ok(())
    }
    
    #[tokio::test]
    async fn test_processing_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = TestContext::new().await?;

        // Create test queue
        ctx.adapter.create_queue("test-queue".to_string(), None).await?;

        // Enqueue an item
        let test_payload = json!({"task": "process_data", "data": [1, 2, 3]});
        let queue_item = ctx.create_test_queue_item(test_payload.clone());
        let item_id = queue_item.id();

        ctx.adapter.enqueue("test-queue", queue_item).await?;

        // Dequeue the item
        let dequeued_item_opt = ctx.adapter.dequeue("test-queue").await?;
        assert!(dequeued_item_opt.is_some(), "Expected to dequeue an item, but got None");
        let dequeued_item = dequeued_item_opt.unwrap();
        let dequeued_id = dequeued_item.id();
        assert_eq!(dequeued_id, item_id);

        // Start processing
        let worker_id = "worker-001".to_string();
        ctx.adapter.start_processing("test-queue", item_id, worker_id.clone()).await?;

        // Complete processing with result
        let result_data = json!({"status": "completed", "processed_count": 3});
        ctx.adapter.complete_processing("test-queue", item_id, Some(result_data)).await?;

        // Get queue stats to verify completion
        let stats = ctx.adapter.get_queue_stats("test-queue").await?;
        assert_eq!(stats.completed_items, 1);
        assert_eq!(stats.processing_items, 0);
        assert_eq!(stats.pending_items, 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_processing_failure_and_retry() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = TestContext::new().await?;

        // Create test queue
        ctx.adapter.create_queue("test-queue".to_string(), None).await?;

        // Enqueue an item with retry configuration
        let test_payload = json!({"task": "failing_task", "attempts": 0});
        let mut queue_item = ctx.create_test_queue_item(test_payload.clone());
        queue_item.config.max_retries = 2; // Allow 2 retries
        let item_id = queue_item.id();

        ctx.adapter.enqueue("test-queue", queue_item).await?;

        // Dequeue and fail processing (first attempt)
        ctx.adapter.dequeue("test-queue").await?.unwrap();
        let should_retry = ctx.adapter.fail_processing("test-queue", item_id, "Simulated failure".to_string()).await?;
        assert!(should_retry); // Should retry

        // Check that item is back in queue
        let stats_after_first_fail = ctx.adapter.get_queue_stats("test-queue").await?;
        assert_eq!(stats_after_first_fail.pending_items, 1);

        // Dequeue and fail again (second attempt)
        let dequeued_item_retry = ctx.adapter.dequeue("test-queue").await?.unwrap();
        // Test: dequeued_item_retry should have the same ID and payload as the original item
        assert_eq!(dequeued_item_retry.id(), item_id, "Dequeued retry item ID should match the original item ID");
        assert_eq!(dequeued_item_retry.message.message, test_payload, "Dequeued retry item payload should match the original payload");

        let should_retry_again = ctx.adapter.fail_processing("test-queue", item_id, "Simulated failure again".to_string()).await?;
        assert!(should_retry_again); // Should still retry

        // Dequeue and fail final time (third attempt - max retries reached)
        let dequeued_item_final = ctx.adapter.dequeue("test-queue").await?.unwrap();
        let should_retry_final = ctx.adapter.fail_processing("test-queue", item_id, "Final failure".to_string()).await?;
        assert!(!should_retry_final); // Should not retry anymore

        // Check that item is now in failed state
        let final_stats = ctx.adapter.get_queue_stats("test-queue").await?;
        assert_eq!(final_stats.failed_items, 1);
        assert_eq!(final_stats.pending_items, 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_batch_operations() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = TestContext::new().await?;

        // Create test queue with explicit non-priority config
        let config = QueueConfig {
            priority_based: false,  // ← Explicit non-priority
            ..Default::default()
        };
        ctx.adapter.create_queue("batch-test-queue".to_string(), Some(config)).await?;

        // Create multiple test items with correct queue name
        let items = (0..5).map(|i| {
            let payload = json!({"batch_item": i, "message": format!("Item {}", i)});
            ctx.create_queue_item_for("batch-test-queue", payload)
        }).collect::<Vec<_>>();

        // Test batch enqueue
        let enqueued_ids = ctx.adapter.enqueue_batch("batch-test-queue", items).await?;
        assert_eq!(enqueued_ids.len(), 5);

        // Check queue length
        let length = ctx.adapter.queue_length("batch-test-queue").await?;
        assert_eq!(length, 5);

        // Test batch dequeue
        let dequeued_items = ctx.adapter.dequeue_batch("batch-test-queue", 3).await?;
        assert_eq!(dequeued_items.len(), 3);

        // Check remaining queue length
        let remaining_length = ctx.adapter.queue_length("batch-test-queue").await?;
        assert_eq!(remaining_length, 2);

        Ok(())
    }

     #[tokio::test]
    async fn test_priority_queue_operations() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = TestContext::new().await?;

        // Create priority queue
        let priority_config = QueueConfig {
            priority_based: true,
            ..Default::default()
        };
        ctx.adapter.create_queue("priority-test-queue".to_string(), Some(priority_config)).await?;

        // Enqueue items with different priorities
        let low_item = ctx.create_priority_queue_item(
            json!({"priority": "low", "message": "Low priority task"}), 
            MessagePriority::Low
        );
        let high_item = ctx.create_priority_queue_item(
            json!({"priority": "high", "message": "High priority task"}), 
            MessagePriority::High
        );
        let critical_item = ctx.create_priority_queue_item(
            json!({"priority": "critical", "message": "Critical task"}), 
            MessagePriority::Critical
        );

        // Enqueue in non-priority order - Fix ambiguity by using PriorityQueuePort explicitly
        PriorityQueuePort::enqueue_with_priority(&*ctx.adapter, "priority-test-queue", low_item, MessagePriority::Low).await?;
        PriorityQueuePort::enqueue_with_priority(&*ctx.adapter, "priority-test-queue", high_item, MessagePriority::High).await?;
        PriorityQueuePort::enqueue_with_priority(&*ctx.adapter, "priority-test-queue", critical_item, MessagePriority::Critical).await?;

        // Dequeue should return critical first
        let first_item = ctx.adapter.dequeue_highest_priority("priority-test-queue").await?.unwrap();
        assert_eq!(first_item.message.message["priority"], "critical");

        // Then high
        let second_item = ctx.adapter.dequeue_highest_priority("priority-test-queue").await?.unwrap();
        assert_eq!(second_item.message.message["priority"], "high");

        // Then low
        let third_item = ctx.adapter.dequeue_highest_priority("priority-test-queue").await?.unwrap();
        assert_eq!(third_item.message.message["priority"], "low");

        // Queue should be empty
        let empty_result = ctx.adapter.dequeue_highest_priority("priority-test-queue").await?;
        assert!(empty_result.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_queue_management_operations() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = TestContext::new().await?;

        // Create test queue
        ctx.adapter.create_queue("management-test-queue".to_string(), None).await?;

        // Test pause/resume
        ctx.adapter.pause_queue("management-test-queue").await?;
        ctx.adapter.resume_queue("management-test-queue").await?;

        // Enqueue some items for management testing
        let test_item = ctx.create_test_queue_item(json!({"test": "management"}));
        let item_id = test_item.id();
        ctx.adapter.enqueue("management-test-queue", test_item).await?;

        // Dequeue and start processing
        let dequeued = ctx.adapter.dequeue("management-test-queue").await;
        assert!(dequeued.is_ok(), "Dequeue failed unexpectedly: {:?}", dequeued);
        let dequeued = dequeued.unwrap();
        assert!(dequeued.is_some(), "Expected to dequeue an item, but got None");
        // let item_id = dequeued.id();
        ctx.adapter.start_processing("management-test-queue", item_id, "test-worker".to_string()).await?;

        // Test cancel item
        ctx.adapter.cancel_item("management-test-queue", item_id).await?;

        // Add item to failed state for retry testing
        let failed_item = ctx.create_test_queue_item(json!({"test": "retry"}));
        let failed_id = failed_item.id();
        ctx.adapter.enqueue("management-test-queue", failed_item).await?;
        
        // This should have a test that fails if the dequeue fails
        let dequeued_failed = ctx.adapter.dequeue("management-test-queue").await;
        assert!(dequeued_failed.is_ok(), "Dequeue failed unexpectedly: {:?}", dequeued_failed);
        let dequeued_failed = dequeued_failed.unwrap();
        assert!(dequeued_failed.is_some(), "Expected to dequeue a failed item, but got None");
        
        // Fail it multiple times to get it to failed state
        for _ in 0..4 {
            ctx.adapter.fail_processing("management-test-queue", failed_id, "Test failure".to_string()).await?;
        }

        // Test retry failed item
        ctx.adapter.retry_item("management-test-queue", failed_id).await?;

        // Should be back in pending state
        let stats = ctx.adapter.get_queue_stats("management-test-queue").await?;
        assert_eq!(stats.pending_items, 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_queue_statistics_and_monitoring() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = TestContext::new().await?;

        // Create test queue with explicit non-priority config
        let config = QueueConfig {
            priority_based: false,
            ..Default::default()
        };
        ctx.adapter.create_queue("stats-test-queue".to_string(), Some(config)).await?;

        // Create items with correct queue name
        let items = (0..10).map(|i| {
            ctx.create_queue_item_for("stats-test-queue", json!({"item": i}))
        }).collect::<Vec<_>>();
        
        let item_ids: Vec<Uuid> = items.iter().map(|item| item.id()).collect();
        ctx.adapter.enqueue_batch("stats-test-queue", items).await?;

        // Process some items to completion
        for i in 0..3 {
            let item = ctx.adapter.dequeue("stats-test-queue").await?.unwrap();
            ctx.adapter.start_processing("stats-test-queue", item.id(), format!("worker-{}", i)).await?;
            ctx.adapter.complete_processing("stats-test-queue", item.id(), Some(json!({"result": i}))).await?;
        }

        // Fail some items
        for i in 3..6 {
            let item = ctx.adapter.dequeue("stats-test-queue").await?.unwrap();
            // Fail multiple times to exhaust retries
            for _ in 0..4 {
                ctx.adapter.fail_processing("stats-test-queue", item.id(), "Test failure".to_string()).await?;
            }
        }

        // Leave some in processing
        for i in 6..8 {
            let item = ctx.adapter.dequeue("stats-test-queue").await?.unwrap();
            ctx.adapter.start_processing("stats-test-queue", item.id(), format!("worker-{}", i)).await?;
            // Don't complete or fail these
        }

        // Check final stats
        let final_stats = ctx.adapter.get_queue_stats("stats-test-queue").await?;
        assert_eq!(final_stats.pending_items, 2); // 10 - 8 processed = 2 remaining
        assert_eq!(final_stats.processing_items, 2); // 2 items left in processing
        assert_eq!(final_stats.completed_items, 3); // 3 completed
        assert_eq!(final_stats.failed_items, 3); // 3 failed

        // Test get all stats
        let all_stats = ctx.adapter.get_all_stats().await;
        assert!(all_stats.contains_key("stats-test-queue"));

        Ok(())
    }

    #[tokio::test]
    async fn test_item_summaries_and_details() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = TestContext::new().await?;

        // Create test queue
        ctx.adapter.create_queue("details-test-queue".to_string(), None).await?;

        // Create and enqueue test items
        let test_item = ctx.create_test_queue_item(json!({"detail_test": true, "value": 42}));
        let item_id = test_item.id();
        ctx.adapter.enqueue("details-test-queue", test_item).await?;

        // Dequeue and process
        let dequeued = ctx.adapter.dequeue("details-test-queue").await?.unwrap();
        ctx.adapter.start_processing("details-test-queue", item_id, "detail-worker".to_string()).await?;

        // Test get item details
        let item_details = ctx.adapter.get_item_details("details-test-queue", item_id).await?;
        assert_eq!(item_details.id(), item_id);
        assert_eq!(item_details.message.message["detail_test"], true);
        assert_eq!(item_details.message.message["value"], 42);

        // Test get item summaries
        let summaries = ctx.adapter.get_item_summaries("details-test-queue", vec![item_id]).await?;
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].id, item_id);
        assert_eq!(summaries[0].queue_name, "details-test-queue");

        Ok(())
    }

    #[tokio::test]
    async fn test_purge_operations() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = TestContext::new().await?;

        // Create test queue
        ctx.adapter.create_queue("purge-test-queue".to_string(), None).await?;

        // Create items for different states
        let completed_items = (0..5).map(|i| {
            ctx.create_test_queue_item(json!({"type": "completed", "id": i}))
        }).collect::<Vec<_>>();

        let failed_items = (0..3).map(|i| {
            ctx.create_test_queue_item(json!({"type": "failed", "id": i}))
        }).collect::<Vec<_>>();

        // Process completed items
        for item in completed_items {
            let item_id = item.id();
            ctx.adapter.enqueue("purge-test-queue", item).await?;
            let dequeued = ctx.adapter.dequeue("purge-test-queue").await?.unwrap();
            ctx.adapter.start_processing("purge-test-queue", item_id, "purge-worker".to_string()).await?;
            ctx.adapter.complete_processing("purge-test-queue", item_id, Some(json!({"completed": true}))).await?;
        }

        // Process failed items
        for item in failed_items {
            let item_id = item.id();
            ctx.adapter.enqueue("purge-test-queue", item).await?;
            let dequeued = ctx.adapter.dequeue("purge-test-queue").await?.unwrap();
            // Fail multiple times to exhaust retries
            for _ in 0..4 {
                ctx.adapter.fail_processing("purge-test-queue", item_id, "Purge test failure".to_string()).await?;
            }
        }

        // Verify initial state
        let stats_before = ctx.adapter.get_queue_stats("purge-test-queue").await?;
        assert_eq!(stats_before.completed_items, 5);
        assert_eq!(stats_before.failed_items, 3);

        // Test purge completed
        let purged_completed = ctx.adapter.purge_completed("purge-test-queue").await?;
        assert_eq!(purged_completed, 5);

        // Test purge failed
        let purged_failed = ctx.adapter.purge_failed("purge-test-queue").await?;
        assert_eq!(purged_failed, 3);

        // Verify final state
        let stats_after = ctx.adapter.get_queue_stats("purge-test-queue").await?;
        assert_eq!(stats_after.completed_items, 0);
        assert_eq!(stats_after.failed_items, 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_error_handling() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = TestContext::new().await?;

        // Test operations on non-existent queue
        let result = ctx.adapter.enqueue("non-existent-queue", ctx.create_test_queue_item(json!({"test": true}))).await;
        assert!(matches!(result, Err(QueueError::QueueNotFound(_))));

        let result = ctx.adapter.dequeue("non-existent-queue").await;
        assert!(matches!(result, Err(QueueError::QueueNotFound(_))));

        let result = ctx.adapter.get_queue_stats("non-existent-queue").await;
        assert!(matches!(result, Err(QueueError::QueueNotFound(_))));

        // Test duplicate queue creation
        ctx.adapter.create_queue("duplicate-test".to_string(), None).await?;
        let result = ctx.adapter.create_queue("duplicate-test".to_string(), None).await;
        assert!(matches!(result, Err(QueueError::OperationFailed(_))));

        // Test operations on non-existent items
        let fake_id = Uuid::new_v4();
        let result = ctx.adapter.get_item_details("duplicate-test", fake_id).await;
        assert!(matches!(result, Err(QueueError::ItemNotFound(_))));

        let result = ctx.adapter.cancel_item("duplicate-test", fake_id).await;
        assert!(matches!(result, Err(QueueError::ItemNotFound(_))));

        let result = ctx.adapter.retry_item("duplicate-test", fake_id).await;
        assert!(matches!(result, Err(QueueError::ItemNotFound(_))));

        Ok(())
    }

    #[tokio::test]
    async fn test_queue_config_updates() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = TestContext::new().await?;

        // Create test queue with initial config
        let initial_config = QueueConfig {
            max_capacity: 100,
            priority_based: false,
            preserve_completed: true,
            preserve_failed: true,
            cleanup_interval_seconds: 300,
            ..Default::default()
        };

        ctx.adapter.create_queue("config-test-queue".to_string(), Some(initial_config.clone())).await?;

        // Verify initial config
        let retrieved_config = ctx.adapter.get_queue_config("config-test-queue").await?;
        assert_eq!(retrieved_config.max_capacity, 100);
        assert!(!retrieved_config.priority_based);

        // Update config
        let updated_config = QueueConfig {
            max_capacity: 500,
            priority_based: true,
            preserve_completed: false,
            preserve_failed: true,
            cleanup_interval_seconds: 600,
            ..Default::default()
        };

        ctx.adapter.update_queue_config("config-test-queue", updated_config.clone()).await?;

        // Verify updated config
        let final_config = ctx.adapter.get_queue_config("config-test-queue").await?;
        assert_eq!(final_config.max_capacity, 500);
        assert!(final_config.priority_based);
        assert!(!final_config.preserve_completed);
        assert_eq!(final_config.cleanup_interval_seconds, 600);

        Ok(())
    }

    #[tokio::test]
    async fn test_concurrent_operations() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = TestContext::new().await?;

        // Create test queue
        ctx.adapter.create_queue("concurrent-test-queue".to_string(), None).await?;

        // Spawn multiple tasks to enqueue items concurrently
        let adapter = ctx.adapter.clone();
        let enqueue_handles: Vec<_> = (0..10).map(|i| {
            let adapter = adapter.clone();
            tokio::spawn(async move {
                let payload = json!({"concurrent_item": i, "worker": format!("worker-{}", i)});
                let item = QueueItem::new(
                    "concurrent-test-queue".to_string(),
                    Message::new(
                        Location::service(&format!("producer-{}", i)),
                        Location::service("consumer"),
                        payload,
                    ),
                    None,
                );
                adapter.enqueue("concurrent-test-queue", item).await
            })
        }).collect();

        // Wait for all enqueue operations to complete
        let mut enqueue_results = Vec::new();
        for handle in enqueue_handles {
            let result = handle.await??;
            enqueue_results.push(result);
        }

        // All should succeed
        assert_eq!(enqueue_results.len(), 10);

        // Verify queue length
        let length = ctx.adapter.queue_length("concurrent-test-queue").await?;
        assert_eq!(length, 10);

        // Spawn multiple tasks to dequeue items concurrently
        let dequeue_handles: Vec<_> = (0..10).map(|_| {
            let adapter = adapter.clone();
            tokio::spawn(async move {
                adapter.dequeue("concurrent-test-queue").await
            })
        }).collect();

        // Wait for all dequeue operations to complete
        let mut dequeue_results = Vec::new();
        for handle in dequeue_handles {
            let result = handle.await??;
            dequeue_results.push(result);
        }

        // All should return items (no None results since we have exactly 10 items)
        let successful_dequeues = dequeue_results.iter().filter(|r| r.is_some()).count();
        assert_eq!(successful_dequeues, 10);

        // Queue should be empty
        let final_length = ctx.adapter.queue_length("concurrent-test-queue").await?;
        assert_eq!(final_length, 0);

        Ok(())
    }

     #[tokio::test]
    async fn test_end_to_end_workflow() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = TestContext::new().await?;

        // Create a workflow queue
        let workflow_config = QueueConfig {
            max_capacity: 1000,
            priority_based: true,
            preserve_completed: true,
            preserve_failed: true,
            ..Default::default()
        };

        ctx.adapter.create_queue("workflow-queue".to_string(), Some(workflow_config)).await?;

        // Simulate a complete workflow
        
        // 1. Producer adds high-priority urgent task
        let urgent_task = ctx.create_priority_queue_item(
            json!({
                "task_type": "urgent_processing",
                "data": {"file_id": "urgent-file-123", "priority": "high"},
                "metadata": {"created_by": "system", "deadline": "2024-01-01T12:00:00Z"}
            }),
            MessagePriority::High
        );
        let urgent_id = urgent_task.id();
        PriorityQueuePort::enqueue_with_priority(&*ctx.adapter, "workflow-queue", urgent_task, MessagePriority::High).await?;

        // 2. Producer adds several normal tasks
        let normal_tasks: Vec<_> = (0..5).map(|i| {
            ctx.create_priority_queue_item(
                json!({
                    "task_type": "batch_processing",
                    "data": {"batch_id": format!("batch-{}", i), "items": i * 10},
                    "metadata": {"created_by": "scheduler"}
                }),
                MessagePriority::Normal
            )
        }).collect();
        
        let normal_ids: Vec<_> = normal_tasks.iter().map(|t| t.id()).collect();
        for task in normal_tasks {
            PriorityQueuePort::enqueue_with_priority(&*ctx.adapter, "workflow-queue", task, MessagePriority::Normal).await?;
        }

        // 3. Consumer processes urgent task first
        let first_task = ctx.adapter.dequeue_highest_priority("workflow-queue").await?.unwrap();
        assert_eq!(first_task.id(), urgent_id);
        assert_eq!(first_task.message.message["task_type"], "urgent_processing");

        // Start processing urgent task
        ctx.adapter.start_processing("workflow-queue", urgent_id, "urgent-worker".to_string()).await?;

        // Complete urgent task successfully
        let urgent_result = json!({"status": "completed", "processed_at": "2024-01-01T11:30:00Z", "result": "success"});
        ctx.adapter.complete_processing("workflow-queue", urgent_id, Some(urgent_result)).await?;

        // 4. Process normal tasks with some failures
        for (i, &task_id) in normal_ids.iter().enumerate() {
            let task = ctx.adapter.dequeue_highest_priority("workflow-queue").await?.unwrap();
            assert_eq!(task.id(), task_id);
            
            ctx.adapter.start_processing("workflow-queue", task_id, format!("batch-worker-{}", i)).await?;
            
            if i == 2 {
                // Simulate a failure on the third task
                ctx.adapter.fail_processing("workflow-queue", task_id, "Network timeout during batch processing".to_string()).await?;
            } else {
                // Complete successfully
                let result = json!({"status": "completed", "items_processed": i * 10, "worker": format!("batch-worker-{}", i)});
                ctx.adapter.complete_processing("workflow-queue", task_id, Some(result)).await?;
            }
        }

        // 5. Check workflow results
        let final_stats = ctx.adapter.get_queue_stats("workflow-queue").await?;
        assert_eq!(final_stats.completed_items, 5); // 1 urgent + 4 successful normal
        println!("Pending items: {}", final_stats.pending_items);
        assert_eq!(final_stats.pending_items, 1); // 1 failed task should be retried
        assert_eq!(final_stats.processing_items, 0);

        // 6. Retry the failed task
        println!("DEBUG: Checking queue stats before retry...");
        let pre_retry_stats = ctx.adapter.get_queue_stats("workflow-queue").await?;
        println!("DEBUG: Pre-retry stats - pending: {}, processing: {}, completed: {}, failed: {}", 
                 pre_retry_stats.pending_items, pre_retry_stats.processing_items, 
                 pre_retry_stats.completed_items, pre_retry_stats.failed_items);

        println!("DEBUG: Checking if we can dequeue with highest priority...");
        let debug_item = ctx.adapter.dequeue_highest_priority("workflow-queue").await?;
        println!("DEBUG: dequeue_highest_priority returned: {:?}", debug_item.is_some());

        if debug_item.is_none() {
            println!("DEBUG: No item found with dequeue_highest_priority, trying regular dequeue...");
            let regular_debug = ctx.adapter.dequeue("workflow-queue").await?;
            println!("DEBUG: regular dequeue returned: {:?}", regular_debug.is_some());
        } else {
            // Put it back for the actual test
            let item = debug_item.unwrap();
            println!("DEBUG: Found item with priority: {:?}", item.message.priority);
            PriorityQueuePort::enqueue_with_priority(&*ctx.adapter, "workflow-queue", item, MessagePriority::Normal).await?;
        }
        
        // Try both dequeue methods to see which one works
        let retry_task = match ctx.adapter.dequeue_highest_priority("workflow-queue").await? {
            Some(item) => Some(item),
            None => ctx.adapter.dequeue("workflow-queue").await.ok().flatten(),
        }.unwrap();
        let retry_id = retry_task.id();
        assert_eq!(retry_id, normal_ids[2]); 

        ctx.adapter.start_processing("workflow-queue", retry_id, "retry-worker".to_string()).await?;
        let retry_result = json!({"status": "completed", "items_processed": 20, "retry": true});
        ctx.adapter.complete_processing("workflow-queue", retry_id, Some(retry_result)).await?;

        // 7. Final verification
        let workflow_complete_stats = ctx.adapter.get_queue_stats("workflow-queue").await?;
        assert_eq!(workflow_complete_stats.completed_items, 6); // All tasks completed
        assert_eq!(workflow_complete_stats.pending_items, 0);
        assert_eq!(workflow_complete_stats.failed_items, 0);
        assert_eq!(workflow_complete_stats.total_items, 6);

        println!("End-to-end workflow test completed successfully!");

        Ok(())
    }
}