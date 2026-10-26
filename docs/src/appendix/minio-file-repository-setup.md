# S3-Compatible File Storage Setup (with rust-s3)

This section describes how to set up and use the S3-compatible file storage adapter for the paladin framework using the `rust-s3` crate, alongside the Redis queue adapter. The adapter is a generic SigV4 S3 client: it talks to RustFS in development, testing and CI, and to MinIO, AWS S3, DigitalOcean Spaces or any other S3-compatible endpoint in production.

The development, test, CI and reference Kubernetes object store is RustFS, recorded as ADR-0055 (`.planning/decisions/0055-dev-test-reference-object-store-rustfs.md`).

> **Historical names.** `MinioAdapter`, `MinioConfig`, the `minio:` configuration section and the
> `APP_MINIO_*` environment variables keep their historical names, whatever store they point at. A
> rename to an S3-neutral noun is a deferred public break, so no operator configuration changes when
> the store behind the adapter changes.

> This is appendix reference material, not a tutorial: the code blocks below are illustrative
> fragments fenced `rust,ignore` and are not compiled by mdBook's build. The API forms are
> verified against `crates/paladin-ports/src/output/file_storage_port.rs` and
> `src/infrastructure/adapters/file_storage/mod.rs`.

## Why rust-s3 instead of minio crate?

We use the `rust-s3` crate instead of the `minio` crate because:
- **More Mature**: `rust-s3` is actively maintained and widely used
- **Better S3 Compatibility**: Full S3 API compatibility means it works with RustFS, MinIO, AWS S3, and other S3-compatible services
- **Rich Features**: Supports presigned URLs, multipart uploads, and advanced S3 features
- **Better Error Handling**: More comprehensive error handling and retry mechanisms
- **Future-Proof**: Easy to migrate to AWS S3 or other S3-compatible services

## Prerequisites

- Docker and Docker Compose
- Rust 1.88 or later
- An S3-compatible object store (RustFS via Docker for development; works with rust-s3)
- Redis 7.0 or later (if running locally)

## Quick Start

The development stack runs Redis and RustFS (`rustfs/rustfs:1.0.0`). There is no bucket-init container: the adapter creates its bucket (`paladin-files`) itself on first connect.

### 1. Start with Docker Compose

The easiest way to get started with both Redis and RustFS:

```bash
# Clone the repository
git clone <repository-url>
cd paladin

# Start Redis, RustFS, and the application (make services-up does the same)
docker compose -f docker/docker-compose.yml up -d

# Or start only the backing services and run the application locally
docker compose -f docker/docker-compose.yml up -d redis rustfs

# Check service health
docker compose -f docker/docker-compose.yml ps
make health
```

Readiness and console:

```bash
# Readiness (liveness is /health)
curl -f http://localhost:9000/health/ready

# RustFS console (development compose only); prints the URL and the dev credentials
make minio-console
# http://localhost:9001/rustfs/console/index.html
# Sign in with RUSTFS_ACCESS_KEY / RUSTFS_SECRET_KEY from your .env
```

### 2. Development Setup

For development with auto-reload:

```bash
# Start Redis, RustFS, and development tools
docker compose -f docker/docker-compose.yml -f docker/docker-compose.dev.yml up -d

# Or run locally with services in Docker
docker run -d --name redis -p 6379:6379 redis:7-alpine
docker run -d --name rustfs -p 9000:9000 -p 9001:9001 \
  -e "RUSTFS_ACCESS_KEY=paladin-dev" \
  -e "RUSTFS_SECRET_KEY=paladin-dev-secret" \
  rustfs/rustfs:1.0.0

# Run the application locally
RUST_LOG=debug cargo run
```

The image entrypoint supplies the data directory, so no command arguments are needed. The store-side variables are `RUSTFS_ACCESS_KEY` and `RUSTFS_SECRET_KEY`; the application-side variables (`APP_MINIO_ACCESS_KEY` and `APP_MINIO_SECRET_KEY`) must carry the same pair.

### 3. Testing

Run the integration tests:

```bash
# Using Docker (recommended)
docker compose -f docker/docker-compose.test.yml up --build test-runner

# Or locally (requires Redis and an S3-compatible store running)
cargo test file_storage_integration_tests
cargo test queue_integration_tests
```

## Configuration

### Environment Variables

Both Redis and the object store can be configured using environment variables:

```bash
# Redis Queue Configuration
export APP_REDIS_HOST=localhost
export APP_REDIS_PORT=6379
export APP_REDIS_PASSWORD=your_password  # Optional
export APP_REDIS_DB=0

# S3-compatible File Storage Configuration (using rust-s3; historical MINIO names)
export APP_MINIO_ENDPOINT=localhost:9000
export APP_MINIO_ACCESS_KEY=your-access-key
export APP_MINIO_SECRET_KEY=your-secret-key
export APP_MINIO_BUCKET=paladin-files
export APP_MINIO_SECURE=false
export APP_MINIO_MAX_FILE_SIZE=104857600  # 100MB
export APP_MINIO_ALLOWED_EXTENSIONS=txt,md,json,pdf,doc,rs,py
```

For the dev compose, the store's own root credentials are `RUSTFS_ACCESS_KEY` and `RUSTFS_SECRET_KEY` in `.env` (see `.env.example`); use a real secret store outside development.

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
minio_access_key = "your-access-key"
minio_secret_key = "your-secret-key"
minio_bucket = "paladin-files"
minio_secure = false
max_file_size = 104857600  # 100MB
allowed_extensions = ["txt", "md", "json", "pdf", "doc", "rs", "py"]
```

## File Storage Operations with rust-s3

### Basic Usage

```rust,ignore
use paladin::infrastructure::adapters::file_storage::minio::MinioAdapter;
use paladin_ports::output::file_storage_port::{FileStoragePort, UploadOptions};
use std::path::PathBuf;

// Initialize the adapter (uses rust-s3 internally; creates the bucket if it is missing)
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

```rust,ignore
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

// A presigned URL carries a credential-derived signature: do not log the query string.
println!("Presigned download URL host: {}", download_url.split('?').next().unwrap_or(""));
```

#### Metadata and Content Types

```rust,ignore
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

```rust,ignore
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

### File Versioning

```rust,ignore
// Upload a new version
let versioned_file = adapter.upload_file_version(&file_path, &new_content, None).await?;

// List all versions
let versions = adapter.list_file_versions(&file_path).await?;
```

### Generating and Storing Reports

```rust,ignore
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

### Combined Queue and Storage Operations

```rust,ignore
use paladin::infrastructure::adapters::queue::redis::RedisQueueAdapter;
use paladin_ports::output::queue_port::QueuePort;

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

### File Storage Structure

The adapter organizes files in a logical structure inside the one configured bucket (`paladin-files` by default). Prefixes are conventions of the caller, not separate buckets:

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

### Error Handling

The adapter provides comprehensive error handling:

```rust,ignore
use paladin_ports::output::file_storage_port::FileStorageError;

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

## Compatibility with S3 Services

Thanks to `rust-s3`, the same adapter can work with different S3-compatible services. Only the endpoint, credentials, `secure` and `path_style` differ:

### RustFS (development, testing, CI, reference Kubernetes)

The development compose files, the CI service containers, the devcontainer, the contract suite's local mode and `k8s/rustfs.yaml` all run the pinned image `rustfs/rustfs:1.0.0`. RustFS is a path-style store: keep `path_style: true`. Treat an object's ETag as an opaque token, never as a content hash: after a multipart upload it is a composite value.

```rust,ignore
let config = MinioConfig {
    endpoint: "localhost:9000".to_string(),
    access_key: "your-access-key".to_string(),
    secret_key: "your-secret-key".to_string(),
    bucket: "dev-bucket".to_string(),
    secure: false,
    path_style: true,  // Required for RustFS
    ..Default::default()
};
```

### MinIO

MinIO remains a supported production target through the same adapter (it is no longer the development stack's store). Use path-style addressing, and point the endpoint at your own MinIO deployment.

```rust,ignore
let config = MinioConfig {
    endpoint: "minio.internal.example:9000".to_string(),
    access_key: "YOUR_MINIO_ACCESS_KEY".to_string(),
    secret_key: "YOUR_MINIO_SECRET_KEY".to_string(),
    bucket: "production-bucket".to_string(),
    secure: true,
    path_style: true,  // Important for MinIO
    ..Default::default()
};
```

### AWS S3 (Production)
```rust,ignore
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
```rust,ignore
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

```rust,ignore
use paladin_ports::output::file_storage_port::*;

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

### RustFS Console (Development)

The RustFS console is enabled in the development compose and the devcontainer only. It is disabled in CI, the test compose and the Kubernetes manifest, which keeps the exposed surface small.

```bash
# Start the development stack
docker compose -f docker/docker-compose.yml up -d redis rustfs

# Print the console URL and the dev credentials
make minio-console
# Console: http://localhost:9001/rustfs/console/index.html
# Sign in with RUSTFS_ACCESS_KEY / RUSTFS_SECRET_KEY from your .env
```

### File Storage Statistics

```rust,ignore
// Get storage statistics (powered by rust-s3)
let stats = adapter.get_storage_stats().await?;
println!("Total files: {}, Total size: {} bytes",
         stats.total_files, stats.total_size);
println!("Files by type: {:?}", stats.files_by_type);

// Health check
let health = adapter.health_check().await?;
if health.is_available {
    println!("Object store is healthy (response time: {}ms)",
             health.response_time_ms.unwrap_or(0));
}
```

## Performance Considerations

### Connection Management

`rust-s3` provides efficient connection handling:

```rust,ignore
// rust-s3 automatically manages HTTP connections and connection pooling
// Supports concurrent operations out of the box
// Includes automatic retry logic for failed requests
```

### Batch Operations

Use batch operations for better performance:

```rust,ignore
// rust-s3 executes uploads concurrently for better performance
let batch_results = adapter.upload_files(large_file_list).await?;
```

### Timeout and Retry Configuration

```rust,ignore
let config = MinioConfig {
    connection_timeout: Duration::from_secs(30),
    request_timeout: Duration::from_secs(300),
    max_retries: 3,
    ..Default::default()
};
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

1. **Object Store Connection Failed**
   ```bash
   # Check the store is running
   docker ps | grep rustfs

   # Check liveness and readiness
   curl -f http://localhost:9000/health
   curl -f http://localhost:9000/health/ready
   ```

2. **Path Style vs Virtual Hosted Style**
   ```rust
   // For RustFS and MinIO, always use path_style: true
   let config = MinioConfig {
       path_style: true,  // Important for RustFS and MinIO
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

4. **Bucket Access Denied**
   ```bash
   # Ensure APP_MINIO_ACCESS_KEY and APP_MINIO_SECRET_KEY match the store's
   # RUSTFS_ACCESS_KEY / RUSTFS_SECRET_KEY (or a provisioned IAM user)
   ```

5. **File Upload Failed**
   ```bash
   # Check file size limits
   # Check allowed extensions configuration
   # The adapter creates its bucket on first connect; a permission error there
   # means the credentials cannot create buckets
   ```

6. **Store Will Not Start After an Upgrade**
   ```bash
   # Volumes from before the RustFS swap hold an incompatible on-disk format and
   # are not migrated. Remove the old volume and let the stack start clean.
   docker compose -f docker/docker-compose.yml down
   docker volume ls   # then `docker volume rm` the object-store volume left over from before the swap
   ```

### Debug Logging

Enable debug logging for detailed file operations:

```bash
RUST_LOG=debug cargo run
```

### Integration Testing

The `FileStoragePort` contract suite runs against a real S3-compatible store and covers presigned URLs, multipart uploads and ETags. In external mode it reads the `TEST_MINIO_*` variables (the test compose publishes the store on `localhost:9010`):

```bash
export USE_EXTERNAL_TEST_SERVICES=true
export TEST_REDIS_HOST=localhost TEST_REDIS_PORT=6380
export TEST_MINIO_ENDPOINT=localhost:9010
export TEST_MINIO_ACCESS_KEY=your-test-access-key
export TEST_MINIO_SECRET_KEY=your-test-secret-key

cargo test --test lib --features integration-tests,s3-storage file_storage_integration_tests -- --ignored --test-threads=1

# A single case
cargo test --test lib --features integration-tests,s3-storage test_presigned_urls -- --ignored
```

CI runs this suite in the Integration Tests and Docker Integration Tests jobs and fails if fewer than 11 cases pass.

## Migration Guide

### From minio crate to rust-s3

If you were previously using the `minio` crate, here are the key differences:

1. **Better Error Handling**: rust-s3 provides more detailed error information
2. **Presigned URLs**: Built-in support for presigned URLs
3. **S3 Compatibility**: Full S3 API compatibility
4. **Performance**: Better connection pooling and concurrency

### Code Changes Required

```rust,ignore
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

1. **AWS S3 or a managed S3 endpoint**: the same adapter works unchanged
2. **Multi-node MinIO**: Deploy MinIO in distributed mode from your own supported source, if you run your own store
3. **Load Balancing**: Use multiple store instances behind a load balancer

### Production Kubernetes manifest (ADR-0055)

`k8s/rustfs.yaml` (renamed from the earlier MinIO manifest) is the one object-store manifest: it serves both the Kubernetes Smoke Test and the reference deployment. It is a reference, not a production store: a single node (`replicas: 1`), `emptyDir` storage that does not outlive the pod, the console disabled, a non-root pod (uid 10001), and no high availability.

For production, point the same adapter at AWS S3, a managed S3 endpoint, MinIO or any other SigV4 S3 store: set the `minio:` section (or `APP_MINIO_*`) to that endpoint, with `path_style` matching the store. The application-side credentials must match the store's root pair unless you provision a separate IAM user, which is the least-privilege choice. Keep the credentials in a Kubernetes Secret or your secret store, never in the manifest.

### Security Best Practices

1. **Use strong credentials**:
   ```bash
   export RUSTFS_ACCESS_KEY=your-secure-access-key
   export RUSTFS_SECRET_KEY=your-very-secure-secret-key-32chars
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

5. **Bucket Policies and Network Security**: configure appropriate bucket policies and use VPC or private networks. The development compose does not grant anonymous access to the bucket, and production should not either.

## Examples

The contract suite at `tests/integration/file_storage_integration_tests.rs` is the executable reference for the adapter's behaviour, including presigned URLs, multipart uploads and ETags. See also the `examples/` directory and `examples/README.md` for runnable examples of the other framework components.
