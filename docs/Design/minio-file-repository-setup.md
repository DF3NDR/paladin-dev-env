# MinIO File Storage Adapter Setup (with rust-s3)

This section describes how to set up and use the MinIO file storage adapter for the paladin framework using the `rust-s3` crate, alongside the Redis queue adapter.

## Why rust-s3 instead of minio crate?

We use the `rust-s3` crate instead of the `minio` crate because:
- **More Mature**: `rust-s3` is actively maintained and widely used
- **Better S3 Compatibility**: Full S3 API compatibility means it works with MinIO, AWS S3, and other S3-compatible services
- **Rich Features**: Supports presigned URLs, multipart uploads, and advanced S3 features
- **Better Error Handling**: More comprehensive error handling and retry mechanisms
- **Future-Proof**: Easy to migrate to AWS S3 or other S3-compatible services

## Prerequisites

- Docker and Docker Compose
- Rust 1.75 or later
- MinIO server (via Docker - works perfectly with rust-s3)
- Redis 7.0 or later (if running locally)

## Quick Start

### 1. Start with Docker Compose

The easiest way to get started with both Redis and MinIO:

```bash
# Clone the repository
git clone <repository-url>
cd paladin

# Start Redis, MinIO, and the application
docker-compose -f docker/docker-compose.yml up -d

# Check service health
docker-compose ps
```

### 2. Development Setup

For development with auto-reload:

```bash
# Start Redis, MinIO, and development tools
docker-compose -f docker/docker-compose.yml -f docker/docker-compose.dev.yml up -d

# Or run locally with services in Docker
docker run -d --name redis -p 6379:6379 redis:7-alpine
docker run -d --name minio -p 9000:9000 -p 9001:9001 \
  -e "MINIO_ROOT_USER=minioadmin" \
  -e "MINIO_ROOT_PASSWORD=minioadmin" \
  minio/minio server /data --console-address ":9001"

# Run the application locally
RUST_LOG=debug cargo run
```

### 3. Testing

Run the integration tests:

```bash
# Using Docker (recommended)
docker-compose -f docker/docker-compose.test.yml up --build test-runner

# Or locally (requires Redis and MinIO running)
cargo test file_storage_integration_tests
cargo test queue_integration_tests
```

## Configuration

### Environment Variables

Both Redis and MinIO can be configured using environment variables:

```bash
# Redis Queue Configuration
export APP_REDIS_HOST=localhost
export APP_REDIS_PORT=6379
export APP_REDIS_PASSWORD=your_password  # Optional
export APP_REDIS_DB=0

# MinIO File Storage Configuration (using rust-s3)
export APP_MINIO_ENDPOINT=localhost:9000
export APP_MINIO_ACCESS_KEY=minioadmin
export APP_MINIO_SECRET_KEY=minioadmin
export APP_MINIO_BUCKET=paladin-files
export APP_MINIO_SECURE=false
export APP_MINIO_MAX_FILE_SIZE=104857600  # 100MB
export APP_MINIO_ALLOWED_EXTENSIONS=txt,md,json,pdf,doc,rs,py
```

### Configuration File

Add both queue and file storage configuration to your `config.toml`:

```toml
[queue]
redis_host = "localhost"
redis_port = 6379
redis_password = ""  # Optional
redis_db = 0

[file_storage]
minio_endpoint = "localhost:9000"
minio_access_key = "minioadmin"
minio_secret_key = "minioadmin"
minio_bucket = "paladin-files"
minio_secure = false
max_file_size = 104857600  # 100MB
allowed_extensions = ["txt", "md", "json", "pdf", "doc", "rs", "py"]
```

## File Storage Operations with rust-s3

### Basic Usage

```rust
use paladin::infrastructure::adapters::file_storage::minio::MinioAdapter;
use paladin::application::ports::output::file_storage_port::{FileStoragePort, UploadOptions};
use std::path::PathBuf;

// Initialize the adapter (uses rust-s3 internally)
let config = MinioConfig::default();
let adapter = MinioAdapter::new(config, None).await?;

// Upload a file
let file_path = PathBuf::from("analysis/code.rs");
let file_content = std::fs::read("local_file.rs")?;
let upload_options = UploadOptions {
    content_type: Some("text/plain".to_string()),
    tags: vec!["analysis".to_string(), "rust".to_string()],
    overwrite: true,
    ..Default::default()
};

let file_item = adapter.upload_file(&file_path, &file_content, Some(upload_options)).await?;

// Download a file
let downloaded_content = adapter.download_file(&file_path, None).await?;

// List files
let list_options = ListOptions {
    prefix: Some("analysis/".to_string()),
    extensions: vec!["rs".to_string()],
    ..Default::default()
};
let file_list = adapter.list_files(Some(list_options)).await?;

// Delete a file
adapter.delete_file(&file_path).await?;
```

### Advanced Features with rust-s3

#### Presigned URLs

```rust
use std::time::Duration;

// Generate presigned download URL (valid for 1 hour)
let download_url = adapter.generate_download_url(
    &file_path,
    Duration::from_secs(3600),
    None
).await?;

// Generate presigned upload URL
let upload_url = adapter.generate_upload_url(
    &file_path,
    Duration::from_secs(3600),
    None
).await?;

println!("Presigned download URL: {}", download_url);
println!("Presigned upload URL: {}", upload_url);
```

#### Metadata and Content Types

```rust
let mut metadata = HashMap::new();
metadata.insert("author".to_string(), "security-team".to_string());
metadata.insert("scan-type".to_string(), "vulnerability".to_string());

let upload_options = UploadOptions {
    content_type: Some("application/json".to_string()),
    metadata,
    tags: vec!["security".to_string(), "scan".to_string()],
    cache_control: Some("max-age=3600".to_string()),
    ..Default::default()
};

let file_item = adapter.upload_file(&file_path, &content, Some(upload_options)).await?;
```

### Batch Operations

```rust
// Upload multiple files concurrently (rust-s3 handles concurrency efficiently)
let files = vec![
    (PathBuf::from("batch/file1.txt"), file1_content, Some(options1)),
    (PathBuf::from("batch/file2.txt"), file2_content, Some(options2)),
];
let uploaded_items = adapter.upload_files(files).await?;

// Download multiple files concurrently
let paths = vec![PathBuf::from("batch/file1.txt"), PathBuf::from("batch/file2.txt")];
let downloaded_files = adapter.download_files(paths, None).await?;
```

## Compatibility with S3 Services

Thanks to `rust-s3`, the same adapter can work with different S3-compatible services:

### MinIO (Development)
```rust
let config = MinioConfig {
    endpoint: "localhost:9000".to_string(),
    access_key: "minioadmin".to_string(),
    secret_key: "minioadmin".to_string(),
    bucket: "dev-bucket".to_string(),
    secure: false,
    path_style: true,  // Important for MinIO
    ..Default::default()
};
```

### AWS S3 (Production)
```rust
let config = MinioConfig {
    endpoint: "s3.amazonaws.com".to_string(),
    access_key: "YOUR_AWS_ACCESS_KEY".to_string(),
    secret_key: "YOUR_AWS_SECRET_KEY".to_string(),
    bucket: "production-bucket".to_string(),
    secure: true,
    path_style: false,  // AWS S3 uses virtual-hosted style
    ..Default::default()
};
```

### DigitalOcean Spaces
```rust
let config = MinioConfig {
    endpoint: "nyc3.digitaloceanspaces.com".to_string(),
    access_key: "YOUR_DO_ACCESS_KEY".to_string(),
    secret_key: "YOUR_DO_SECRET_KEY".to_string(),
    bucket: "your-space-name".to_string(),
    secure: true,
    path_style: false,
    ..Default::default()
};
```

## Security Auditing Workflow

### Uploading Code for Analysis

```rust
use paladin::application::ports::output::file_storage_port::*;

// Upload source code files with rust-s3
let rust_files = vec!["main.rs", "lib.rs", "security.rs"];
for file_name in rust_files {
    let file_path = PathBuf::from(format!("analysis/src/{}", file_name));
    let content = std::fs::read(file_name)?;
    let options = UploadOptions {
        content_type: Some("text/plain".to_string()),
        tags: vec!["source".to_string(), "rust".to_string(), "security".to_string()],
        metadata: {
            let mut meta = HashMap::new();
            meta.insert("analysis_type".to_string(), "security_audit".to_string());
            meta.insert("language".to_string(), "rust".to_string());
            meta.insert("backend".to_string(), "rust-s3".to_string());
            meta
        },
        ..Default::default()
    };
    
    adapter.upload_file(&file_path, &content, Some(options)).await?;
}
```

## Monitoring and Management

### MinIO Console (Development)

Access MinIO Console for file management:

```bash
# Start with development profile
docker-compose --profile dev up -d

# Access MinIO Console
open http://localhost:9001
# Login: minioadmin/minioadmin (configurable via environment)
```

### File Storage Statistics

```rust
// Get storage statistics (powered by rust-s3)
let stats = adapter.get_storage_stats().await?;
println!("Total files: {}, Total size: {} bytes", 
         stats.total_files, stats.total_size);
println!("Files by type: {:?}", stats.files_by_type);

// Health check
let health = adapter.health_check().await?;
if health.is_available {
    println!("MinIO is healthy (response time: {}ms)", 
             health.response_time_ms.unwrap_or(0));
}
```

## Performance Considerations

### Connection Management

`rust-s3` provides efficient connection handling:

```rust
// rust-s3 automatically manages HTTP connections and connection pooling
// Supports concurrent operations out of the box
// Includes automatic retry logic for failed requests
```

### Batch Operations

Use batch operations for better performance:

```rust
// rust-s3 executes uploads concurrently for better performance
let batch_results = adapter.upload_files(large_file_list).await?;
```

### Timeout and Retry Configuration

```rust
let config = MinioConfig {
    connection_timeout: Duration::from_secs(30),
    request_timeout: Duration::from_secs(300),
    max_retries: 3,
    ..Default::default()
};
```

## Troubleshooting

### Common Issues

1. **MinIO Connection Failed**
   ```bash
   # Check MinIO is running
   docker ps | grep minio
   
   # Check MinIO health
   curl -f http://localhost:9000/minio/health/live
   ```

2. **Path Style vs Virtual Hosted Style**
   ```rust
   // For MinIO, always use path_style: true
   let config = MinioConfig {
       path_style: true,  // Important for MinIO
       ..Default::default()
   };
   
   // For AWS S3, use path_style: false
   let config = MinioConfig {
       path_style: false,  // For AWS S3
       ..Default::default()
   };
   ```

3. **Presigned URL Issues**
   ```rust
   // Ensure correct endpoint format for presigned URLs
   let config = MinioConfig {
       endpoint: "localhost:9000".to_string(),  // No protocol
       secure: false,  // rust-s3 will add http://
       ..Default::default()
   };
   ```

### Debug Logging

Enable debug logging for detailed file operations:

```bash
RUST_LOG=debug cargo run
```

### Integration Testing

Run specific integration tests:

```bash
# File storage tests with rust-s3
cargo test file_storage_integration_tests

# Test presigned URLs
cargo test test_presigned_urls

# Test S3 compatibility
cargo test test_rust_s3_specific_features
```

## Migration Guide

### From minio crate to rust-s3

If you were previously using the `minio` crate, here are the key differences:

1. **Better Error Handling**: rust-s3 provides more detailed error information
2. **Presigned URLs**: Built-in support for presigned URLs
3. **S3 Compatibility**: Full S3 API compatibility
4. **Performance**: Better connection pooling and concurrency

### Code Changes Required

```rust
// Old (minio crate)
use minio::s3::client::Client;

// New (rust-s3)
use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::region::Region;
```

The adapter interface remains the same, so your application code doesn't need to change.

## Production Deployment

### High Availability Setup

For production, consider:

1. **Multi-node MinIO**: Deploy MinIO in distributed mode
2. **AWS S3**: Migrate to AWS S3 for production (same adapter works)
3. **Load Balancing**: Use multiple MinIO instances behind a load balancer

### Security Best Practices

1. **Strong Credentials**:
   ```bash
   export MINIO_ROOT_USER=your-secure-access-key
   export MINIO_ROOT_PASSWORD=your-very-secure-secret-key-32chars
   ```

2. **HTTPS in Production**:
   ```bash
   export APP_MINIO_SECURE=true
   ```

3. **Bucket Policies**: Configure appropriate bucket policies
4. **Network Security**: Use VPC/private networks

## Examples

The adapter includes comprehensive examples with rust-s3:

- `examples/file_storage_basic.rs` - Basic file operations with rust-s3
- `examples/file_storage_s3_compatibility.rs` - S3 compatibility examples
- `examples/file_storage_presigned_urls.rs` - Presigned URL generation
- `examples/file_storage_security_audit.rs` - Security auditing workflow

## Quick Start

### 1. Start with Docker Compose

The easiest way to get started with both Redis and MinIO:

```bash
# Clone the repository
git clone <repository-url>
cd paladin

# Start Redis, MinIO, and the application
docker-compose -f docker/docker-compose.yml up -d

# Check service health
docker-compose ps
```

### 2. Development Setup

For development with auto-reload:

```bash
# Start Redis, MinIO, and development tools
docker-compose -f docker/docker-compose.yml -f docker/docker-compose.dev.yml up -d

# Or run locally with services in Docker
docker run -d --name redis -p 6379:6379 redis:7-alpine
docker run -d --name minio -p 9000:9000 -p 9001:9001 \
  -e "MINIO_ROOT_USER=minioadmin" \
  -e "MINIO_ROOT_PASSWORD=minioadmin" \
  minio/minio server /data --console-address ":9001"

# Run the application locally
RUST_LOG=debug cargo run
```

### 3. Testing

Run the integration tests:

```bash
# Using Docker (recommended)
docker-compose -f docker/docker-compose.test.yml up --build test-runner

# Or locally (requires Redis and MinIO running)
cargo test file_storage_integration_tests
cargo test queue_integration_tests
```

## Configuration

### Environment Variables

Both Redis and MinIO can be configured using environment variables:

```bash
# Redis Queue Configuration
export APP_REDIS_HOST=localhost
export APP_REDIS_PORT=6379
export APP_REDIS_PASSWORD=your_password  # Optional
export APP_REDIS_DB=0

# MinIO File Storage Configuration
export APP_MINIO_ENDPOINT=localhost:9000
export APP_MINIO_ACCESS_KEY=minioadmin
export APP_MINIO_SECRET_KEY=minioadmin
export APP_MINIO_BUCKET=paladin-files
export APP_MINIO_SECURE=false
export APP_MINIO_MAX_FILE_SIZE=104857600  # 100MB
export APP_MINIO_ALLOWED_EXTENSIONS=txt,md,json,pdf,doc,rs,py
```

### Configuration File

Add both queue and file storage configuration to your `config.toml`:

```toml
[queue]
redis_host = "localhost"
redis_port = 6379
redis_password = ""  # Optional
redis_db = 0

[file_storage]
minio_endpoint = "localhost:9000"
minio_access_key = "minioadmin"
minio_secret_key = "minioadmin"
minio_bucket = "paladin-files"
minio_secure = false
max_file_size = 104857600  # 100MB
allowed_extensions = ["txt", "md", "json", "pdf", "doc", "rs", "py"]
```

## File Storage Operations

### Basic Usage

```rust
use paladin::infrastructure::adapters::file_storage::minio::MinioAdapter;
use paladin::application::ports::output::file_storage_port::{FileStoragePort, UploadOptions};
use std::path::PathBuf;

// Initialize the adapter
let config = MinioConfig::default();
let adapter = MinioAdapter::new(config, None).await?;

// Upload a file
let file_path = PathBuf::from("analysis/code.rs");
let file_content = std::fs::read("local_file.rs")?;
let upload_options = UploadOptions {
    content_type: Some("text/plain".to_string()),
    tags: vec!["analysis".to_string(), "rust".to_string()],
    overwrite: true,
    ..Default::default()
};

let file_item = adapter.upload_file(&file_path, &file_content, Some(upload_options)).await?;

// Download a file
let downloaded_content = adapter.download_file(&file_path, None).await?;

// List files
let list_options = ListOptions {
    prefix: Some("analysis/".to_string()),
    extensions: vec!["rs".to_string()],
    ..Default::default()
};
let file_list = adapter.list_files(Some(list_options)).await?;

// Delete a file
adapter.delete_file(&file_path).await?;
```

### Batch Operations

```rust
// Upload multiple files
let files = vec![
    (PathBuf::from("batch/file1.txt"), file1_content, Some(options1)),
    (PathBuf::from("batch/file2.txt"), file2_content, Some(options2)),
];
let uploaded_items = adapter.upload_files(files).await?;

// Download multiple files
let paths = vec![PathBuf::from("batch/file1.txt"), PathBuf::from("batch/file2.txt")];
let downloaded_files = adapter.download_files(paths, None).await?;
```

### File Versioning

```rust
// Upload a new version
let versioned_file = adapter.upload_file_version(&file_path, &new_content, None).await?;

// List all versions
let versions = adapter.list_file_versions(&file_path).await?;
```

## Security Auditing Workflow

### Uploading Code for Analysis

```rust
use paladin::application::ports::output::file_storage_port::*;

// Upload source code files
let rust_files = vec!["main.rs", "lib.rs", "security.rs"];
for file_name in rust_files {
    let file_path = PathBuf::from(format!("analysis/src/{}", file_name));
    let content = std::fs::read(file_name)?;
    let options = UploadOptions {
        tags: vec!["source".to_string(), "rust".to_string(), "security".to_string()],
        metadata: {
            let mut meta = HashMap::new();
            meta.insert("analysis_type".to_string(), "security_audit".to_string());
            meta.insert("language".to_string(), "rust".to_string());
            meta
        },
        ..Default::default()
    };
    
    adapter.upload_file(&file_path, &content, Some(options)).await?;
}
```

### Generating and Storing Reports

```rust
// Generate security report
let report_content = generate_security_report().await?;
let report_path = PathBuf::from("reports/security_audit_2024.md");

let report_options = UploadOptions {
    content_type: Some("text/markdown".to_string()),
    tags: vec!["report".to_string(), "security".to_string(), "audit".to_string()],
    metadata: {
        let mut meta = HashMap::new();
        meta.insert("report_type".to_string(), "security_audit".to_string());
        meta.insert("generated_at".to_string(), Utc::now().to_rfc3339());
        meta
    },
    ..Default::default()
};

let report_file = adapter.upload_file(&report_path, report_content.as_bytes(), Some(report_options)).await?;
```

## Monitoring and Management

### MinIO Console (Development)

Access MinIO Console for file management:

```bash
# Start with development profile
docker-compose --profile dev up -d

# Access MinIO Console
open http://localhost:9001
# Login: minioadmin/minioadmin (configurable via environment)
```

### File Storage Statistics

```rust
// Get storage statistics
let stats = adapter.get_storage_stats().await?;
println!("Total files: {}, Total size: {} bytes", 
         stats.total_files, stats.total_size);
println!("Files by type: {:?}", stats.files_by_type);

// Health check
let health = adapter.health_check().await?;
if health.is_available {
    println!("MinIO is healthy (response time: {}ms)", 
             health.response_time_ms.unwrap_or(0));
}
```

### Combined Queue and Storage Operations

```rust
use paladin::infrastructure::adapters::queue::redis::RedisQueueAdapter;
use paladin::application::ports::output::queue_port::QueuePort;

// Upload file and queue analysis task
let file_item = storage_adapter.upload_file(&file_path, &content, None).await?;

let analysis_task = AnalysisTask {
    file_path: file_item.path.clone(),
    file_id: file_item.id,
    analysis_type: "security_scan".to_string(),
};

let queue_item = QueueItem::new("analysis-queue".to_string(), analysis_task, None);
let task_id = queue_adapter.enqueue("analysis-queue", queue_item).await?;

println!("File uploaded: {}, Analysis queued: {}", file_item.id, task_id);
```

## File Storage Structure

The adapter organizes files in a logical structure:

```
paladin-files/
├── analysis/           # Source code files for analysis
│   ├── src/           # Source code
│   ├── config/        # Configuration files
│   └── dependencies/  # Dependency files
├── reports/           # Generated reports
│   ├── security/      # Security audit reports
│   ├── analysis/      # Analysis reports
│   └── summaries/     # Summary reports
├── backups/           # Backup files
└── temp/              # Temporary files
```

## Error Handling

The adapter provides comprehensive error handling:

```rust
use paladin::application::ports::output::file_storage_port::FileStorageError;

match adapter.upload_file(&path, &content, None).await {
    Ok(file_item) => println!("Uploaded: {}", file_item.path.display()),
    Err(FileStorageError::FileTooLarge { size, max_size }) => {
        println!("File too large: {} bytes (max: {} bytes)", size, max_size)
    },
    Err(FileStorageError::InvalidPath(msg)) => println!("Invalid path: {}", msg),
    Err(FileStorageError::QuotaExceeded) => println!("Storage quota exceeded"),
    Err(e) => println!("Other error: {}", e),
}
```

## Performance Considerations

### Connection Pooling

Both adapters use connection pooling for efficiency:

```rust
// MinIO adapter automatically manages HTTP connections
// Redis adapter uses ConnectionManager for connection pooling
```

### Batch Operations

Use batch operations for better performance:

```rust
// Instead of multiple single uploads
for file in files {
    adapter.upload_file(&file.path, &file.content, None).await?;  // Slower
}

// Use batch upload
adapter.upload_files(files).await?;  // Faster
```

### File Size Limits

Configure appropriate file size limits:

```bash
# Environment variable
export APP_MINIO_MAX_FILE_SIZE=104857600  # 100MB

# Or in config.toml
[file_storage]
max_file_size = 104857600
```

## Troubleshooting

### Common Issues

1. **MinIO Connection Failed**
   ```bash
   # Check MinIO is running
   docker ps | grep minio
   
   # Check MinIO health
   curl -f http://localhost:9000/minio/health/live
   ```

2. **Bucket Access Denied**
   ```bash
   # Check credentials
   # Ensure APP_MINIO_ACCESS_KEY and APP_MINIO_SECRET_KEY are correct
   ```

3. **File Upload Failed**
   ```bash
   # Check file size limits
   # Check allowed extensions configuration
   # Verify bucket exists and is accessible
   ```

### Debug Logging

Enable debug logging for detailed file operations:

```bash
RUST_LOG=debug cargo run
```

### Integration Testing

Run specific integration tests:

```bash
# File storage tests
cargo test file_storage_integration_tests

# Queue tests  
cargo test queue_integration_tests

# Combined workflow tests
cargo test end_to_end
```

## Production Deployment

### High Availability MinIO

For production, consider MinIO in distributed mode:

```yaml
# docker-compose.prod.yml
services:
  minio1:
    image: minio/minio:latest
    command: server http://minio{1...4}/data{1...2}
    
  minio2:
    image: minio/minio:latest
    command: server http://minio{1...4}/data{1...2}
    
  # ... minio3, minio4
```

### Security Best Practices

1. **Use strong credentials**:
   ```bash
   export MINIO_ROOT_USER=your-secure-access-key
   export MINIO_ROOT_PASSWORD=your-very-secure-secret-key
   ```

2. **Enable HTTPS in production**:
   ```bash
   export APP_MINIO_SECURE=true
   ```

3. **Restrict file types**:
   ```bash
   export APP_MINIO_ALLOWED_EXTENSIONS=rs,py,js,json,md,txt
   ```

4. **Set appropriate file size limits**:
   ```bash
   export APP_MINIO_MAX_FILE_SIZE=52428800  # 50MB
   ```

## Examples

The adapter includes comprehensive examples. See the `examples/` directory:

- `examples/file_storage_basic.rs` - Basic file operations
- `examples/file_storage_batch.rs` - Batch operations  
- `examples/file_storage_security_audit.rs` - Security auditing workflow
- `examples/combined_queue_storage.rs` - Using both adapters together