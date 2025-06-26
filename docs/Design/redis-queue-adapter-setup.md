# Redis Queue Adapter Setup

This section describes how to set up and use the Redis queue adapter for the in4me framework.

## Prerequisites

- Docker and Docker Compose
- Rust 1.75 or later
- Redis 7.0 or later (if running locally)

## Quick Start

### 1. Start with Docker Compose

The easiest way to get started is using Docker Compose:

```bash
# Clone the repository
git clone <repository-url>
cd in4me

# Start Redis and the application
docker-compose -f docker/docker-compose.yml up -d

# Check service health
docker-compose ps
```

### 2. Development Setup

For development with auto-reload:

```bash
# Start Redis and development tools
docker-compose -f docker/docker-compose.yml -f docker/docker-compose.dev.yml up -d

# Or run locally with Redis in Docker
docker run -d --name redis -p 6379:6379 redis:7-alpine

# Run the application locally
RUST_LOG=debug cargo run
```

### 3. Testing

Run the integration tests:

```bash
# Using Docker (recommended)
docker-compose -f docker/docker-compose.test.yml up --build test-runner

# Or locally (requires Redis running)
cargo test queue_integration_tests
```

## Configuration

### Environment Variables

The Redis queue adapter can be configured using environment variables:

```bash
# Redis connection
export APP_REDIS_HOST=localhost
export APP_REDIS_PORT=6379
export APP_REDIS_PASSWORD=your_password  # Optional
export APP_REDIS_DB=0
export APP_REDIS_CONNECTION_TIMEOUT=30

# Queue settings
export APP_REDIS_KEY_PREFIX=in4me:queue
export APP_REDIS_MAX_RETRIES=3
export APP_REDIS_ENABLE_PRIORITY_QUEUES=true
```

### Configuration File

Add queue configuration to your `config.toml`:

```toml
[queue]
redis_host = "localhost"
redis_port = 6379
redis_password = ""  # Optional
redis_db = 0
connection_timeout = 30
key_prefix = "in4me:queue"
max_retries = 3
enable_priority_queues = true
```

## Queue Operations

### Basic Usage

```rust
use in4me::infrastructure::adapters::queue::redis::RedisQueueAdapter;
use in4me::application::ports::output::queue_port::QueuePort;

// Initialize the adapter
let config = RedisQueueConfig::default();
let adapter = RedisQueueAdapter::new(config, None).await?;

// Create a queue
adapter.create_queue("my-queue".to_string(), None).await?;

// Enqueue an item
let message = Message::new(
    Location::service("producer"),
    Location::service("consumer"),
    serde_json::json!({"task": "process_data", "id": 123})
);
let queue_item = QueueItem::new("my-queue".to_string(), message, None);
let item_id = adapter.enqueue("my-queue", queue_item).await?;

// Dequeue an item
if let Some(item) = adapter.dequeue("my-queue").await? {
    // Process the item
    adapter.start_processing("my-queue", item.id(), "worker-1".to_string()).await?;
    
    // Complete processing
    let result = serde_json::json!({"status": "completed"});
    adapter.complete_processing("my-queue", item.id(), Some(result)).await?;
}
```

### Priority Queues

```rust
use in4me::core::base::entity::message::MessagePriority;

// Enqueue with priority
adapter.enqueue_with_priority("priority-queue", high_priority_item, MessagePriority::High).await?;

// Dequeue highest priority first
let item = adapter.dequeue_highest_priority("priority-queue").await?;
```

### Batch Operations

```rust
// Enqueue multiple items at once
let items = vec![item1, item2, item3];
let item_ids = adapter.enqueue_batch("batch-queue", items).await?;

// Dequeue multiple items
let items = adapter.dequeue_batch("batch-queue", 5).await?;
```

## Monitoring and Management

### Redis Commander (Development)

Access Redis Commander for queue inspection:

```bash
# Start with development profile
docker-compose --profile dev up -d

# Access Redis Commander
open http://localhost:8081
# Login: admin/admin (configurable via environment)
```

### Queue Statistics

```rust
// Get queue statistics
let stats = adapter.get_queue_stats("my-queue").await?;
println!("Pending: {}, Processing: {}, Completed: {}, Failed: {}", 
         stats.pending_items, stats.processing_items, 
         stats.completed_items, stats.failed_items);

// Get all queue statistics
let all_stats = adapter.get_all_stats().await;
for (queue_name, stats) in all_stats {
    println!("Queue {}: {} total items", queue_name, stats.total_items);
}
```

### Health Checks

```rust
// Check adapter health
let is_healthy = adapter.health_check().await?;
```

## Queue Management

### Retry Failed Items

```rust
// Retry a specific failed item
adapter.retry_item("my-queue", failed_item_id).await?;
```

### Purge Completed/Failed Items

```rust
// Clean up completed items
let purged_completed = adapter.purge_completed("my-queue").await?;

// Clean up failed items
let purged_failed = adapter.purge_failed("my-queue").await?;
```

### Pause/Resume Queues

```rust
// Pause queue processing
adapter.pause_queue("my-queue").await?;

// Resume queue processing
adapter.resume_queue("my-queue").await?;
```

## Redis Key Structure

The adapter uses the following Redis key patterns:

```
in4me:queue:{queue_name}                    # Main queue (FIFO list)
in4me:queue:{queue_name}:high              # High priority queue
in4me:queue:{queue_name}:normal            # Normal priority queue
in4me:queue:{queue_name}:low               # Low priority queue
in4me:queue:{queue_name}:critical          # Critical priority queue

in4me:queue:meta:{queue_name}              # Queue metadata (hash)
in4me:queue:processing:{queue_name}        # Items being processed (hash)
in4me:queue:completed:{queue_name}         # Completed items (hash)
in4me:queue:failed:{queue_name}            # Failed items (hash)
```

## Error Handling

The adapter provides comprehensive error handling:

```rust
use in4me::core::platform::manager::queue_service::QueueError;

match adapter.enqueue("my-queue", item).await {
    Ok(item_id) => println!("Enqueued item: {}", item_id),
    Err(QueueError::QueueNotFound(name)) => println!("Queue {} not found", name),
    Err(QueueError::QueueFull { queue_name, capacity }) => {
        println!("Queue {} is full (capacity: {})", queue_name, capacity)
    },
    Err(QueueError::OperationFailed(msg)) => println!("Operation failed: {}", msg),
    Err(e) => println!("Other error: {}", e),
}
```

## Performance Considerations

### Connection Pooling

The adapter uses Redis connection manager for efficient connection pooling:

```rust
// Connections are automatically managed
// No need for manual connection handling
```

### Batch Operations

Use batch operations for better performance:

```rust
// Instead of multiple single enqueues
for item in items {
    adapter.enqueue("queue", item).await?;  // Slower
}

// Use batch enqueue
adapter.enqueue_batch("queue", items).await?;  // Faster
```

### Pipeline Operations

The adapter internally uses Redis pipelines for efficient batch operations.

## Troubleshooting

### Common Issues

1. **Connection Failed**
   ```bash
   # Check Redis is running
   docker ps | grep redis
   
   # Check Redis connectivity
   redis-cli ping
   ```

2. **Permission Denied**
   ```bash
   # Check Redis password configuration
   # Ensure APP_REDIS_PASSWORD matches Redis requirepass
   ```

3. **Memory Issues**
   ```bash
   # Check Redis memory usage
   redis-cli info memory
   
   # Configure maxmemory policy in redis.conf
   maxmemory-policy allkeys-lru
   ```

### Debug Logging

Enable debug logging for detailed queue operations:

```bash
RUST_LOG=debug cargo run
```

### Redis Logs

Check Redis logs for connection and operation issues:

```bash
# Docker logs
docker logs in4me-redis

# Or check Redis info
redis-cli info
```

## Production Deployment

### Redis Configuration

For production, ensure proper Redis configuration:

1. **Persistence**: Enable AOF for durability
2. **Memory**: Set appropriate maxmemory and policy
3. **Security**: Use password authentication
4. **Monitoring**: Enable slow log and latency monitoring

### High Availability

Consider Redis Sentinel or Cluster for high availability:

```yaml
# docker-compose.prod.yml
services:
  redis-master:
    image: redis:7-alpine
    command: redis-server --appendonly yes --requirepass ${REDIS_PASSWORD}
    
  redis-replica:
    image: redis:7-alpine
    command: redis-server --appendonly yes --slaveof redis-master 6379
```

### Monitoring

Use Redis monitoring tools:

- Redis Insight for GUI-based monitoring
- Prometheus Redis exporter for metrics
- Custom health checks in your application

## Testing

The adapter includes comprehensive integration tests. Run them with:

```bash
# Full test suite
cargo test

# Queue-specific tests
cargo test queue_integration_tests

# With logging
RUST_LOG=debug cargo test queue_integration_tests -- --nocapture
```

## Examples

See the `examples/` directory for complete usage examples:

- `examples/basic_queue.rs` - Basic queue operations
- `examples/priority_queue.rs` - Priority queue usage
- `examples/batch_processing.rs` - Batch operations
- `examples/error_handling.rs` - Error handling patterns