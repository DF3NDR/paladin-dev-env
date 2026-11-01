//! File Storage Ports - Cloud & Local File Storage Abstraction
//!
//! This module defines the output ports (interfaces) for file storage systems following
//! Hexagonal Architecture principles. These ports provide clean abstractions that allow
//! the application layer to store, retrieve, and manage files across different storage
//! backends (local filesystem, AWS S3, MinIO, Google Cloud Storage, Azure Blob Storage)
//! without being coupled to their implementation details.
//!
//! # Purpose
//!
//! File storage ports enable Paladin agents to persist and retrieve files (documents, artifacts,
//! state snapshots, logs, embeddings, etc.) while maintaining a clean separation between the
//! core business logic and specific storage mechanisms. This allows you to:
//!
//! - Store and retrieve files across multiple storage backends
//! - Switch between providers without changing application code
//! - Test file operations without real storage systems
//! - Implement versioning and multipart uploads
//! - Generate pre-signed URLs for direct client uploads/downloads
//! - Track storage usage and health
//!
//! # Hexagonal Architecture (Ports & Adapters)
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │                    Application Layer                          │
//! │  ┌────────────────────────────────────────────────────────┐  │
//! │  │  Paladin Agent Execution                                │  │
//! │  │  - Store execution artifacts                            │  │
//! │  │  - Save/load agent state (Citadel)                      │  │
//! │  │  - Store document embeddings (Sanctum)                  │  │
//! │  │  - Archive conversation history (Garrison)              │  │
//! │  └─────────────────────┬────────────────────────────────────┘  │
//! │                        │                                       │
//! │                        ↓                                       │
//! │  ┌────────────────────────────────────────────────────────┐  │
//! │  │  FileStoragePort (trait)                                │  │
//! │  │  BatchFileStoragePort (trait)                           │  │
//! │  │  AdvancedFileStoragePort (trait)                        │  │
//! │  │  - upload_file(), download_file()                       │  │
//! │  │  - generate_upload_url(), multipart uploads             │  │
//! │  └────────────────────┬───────────────────────────────────┘  │
//! └─────────────────────────┼────────────────────────────────────┘
//!                          │
//!          ┌───────────────┼───────────────┐
//!          │               │               │
//!          ↓               ↓               ↓
//!   ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
//!   │ MinIO       │ │ AWS S3      │ │ Local FS    │
//!   │ Adapter     │ │ Adapter     │ │ Adapter     │
//!   └─────────────┘ └─────────────┘ └─────────────┘
//! ```
//!
//! # Port Segregation (Interface Segregation Principle)
//!
//! The file storage system is split into focused interfaces:
//!
//! - **FileStoragePort**: Core CRUD operations (upload, download, delete, list)
//! - **BatchFileStoragePort**: Bulk operations for efficiency
//! - **AdvancedFileStoragePort**: Pre-signed URLs, multipart uploads
//! - **FileVersioningPort**: File versioning and history
//! - **FullFileStoragePort**: Combined interface with all capabilities
//! - **FileStorageUtils**: Helper functions for content type detection, validation
//!
//! Implement only the ports your adapter supports. Most adapters implement `FileStoragePort`
//! at minimum, with `AdvancedFileStoragePort` for cloud providers that support pre-signed URLs.
//!
//! # Common Use Cases
//!
//! ## 1. Upload and Download Files
//!
//! ```ignore
//! use paladin_ports::output::file_storage_port::{
//!     FileStoragePort, UploadOptions, FileStorageError
//! };
//! use std::path::Path;
//! use std::sync::Arc;
//!
//! async fn store_artifact(
//!     storage: Arc<dyn FileStoragePort>,
//!     content: Vec<u8>,
//! ) -> Result<(), FileStorageError> {
//!     // Upload file
//!     let path = Path::new("artifacts/result.json");
//!     let options = UploadOptions {
//!         content_type: Some("application/json".into()),
//!         overwrite: false,
//!         ..Default::default()
//!     };
//!
//!     let file_item = storage.upload_file(path, &content, Some(options)).await?;
//!     println!("Uploaded: {} ({} bytes)", file_item.path.display(), file_item.size);
//!
//!     // Download file
//!     let downloaded = storage.download_file(path, None).await?;
//!     println!("Downloaded {} bytes", downloaded.len());
//!     Ok(())
//! }
//! ```
//!
//! ## 2. Generate Pre-Signed Upload URL (Direct Client Upload)
//!
//! ```ignore
//! use paladin_ports::output::file_storage_port::{
//!     AdvancedFileStoragePort, UploadOptions, FileStorageError
//! };
//! use std::path::Path;
//! use std::time::Duration;
//! use std::sync::Arc;
//!
//! async fn direct_upload_url(
//!     storage: Arc<dyn AdvancedFileStoragePort>,
//! ) -> Result<String, FileStorageError> {
//!     let path = Path::new("uploads/document.pdf");
//!     let expires_in = Duration::from_secs(3600); // 1 hour
//!
//!     let url = storage
//!         .generate_upload_url(path, expires_in, None)
//!         .await?;
//!
//!     println!("Share this URL with client: {}", url);
//!     Ok(url)
//! }
//! ```
//!
//! ## 3. List and Filter Files
//!
//! ```ignore
//! use paladin_ports::output::file_storage_port::{
//!     FileStoragePort, ListOptions, FileStorageError
//! };
//! use std::sync::Arc;
//!
//! async fn list_recent_artifacts(
//!     storage: Arc<dyn FileStoragePort>,
//! ) -> Result<(), FileStorageError> {
//!     let options = ListOptions {
//!         prefix: Some("artifacts/".into()),
//!         limit: Some(100),
//!         extensions: vec!["json".into(), "txt".into()],
//!         min_size: Some(1024), // At least 1KB
//!         ..Default::default()
//!     };
//!
//!     let result = storage.list_files(Some(options)).await?;
//!     for file in result.files {
//!         println!("{}: {} bytes", file.path.display(), file.size);
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## 4. Multipart Upload for Large Files
//!
//! ```ignore
//! use paladin_ports::output::file_storage_port::{
//!     AdvancedFileStoragePort, UploadOptions, FileStorageError
//! };
//! use std::path::Path;
//! use std::sync::Arc;
//!
//! async fn upload_large_file(
//!     storage: Arc<dyn AdvancedFileStoragePort>,
//!     file_data: Vec<u8>,
//! ) -> Result<(), FileStorageError> {
//!     let path = Path::new("large-files/model.bin");
//!     let chunk_size = 5 * 1024 * 1024; // 5MB chunks
//!
//!     // Create multipart upload
//!     let upload_id = storage
//!         .create_multipart_upload(path, None)
//!         .await?;
//!
//!     // Upload parts
//!     let mut parts = Vec::new();
//!     for (i, chunk) in file_data.chunks(chunk_size).enumerate() {
//!         let part_number = (i + 1) as u32;
//!         let etag = storage
//!             .upload_part(&upload_id, part_number, chunk)
//!             .await?;
//!         parts.push((part_number, etag));
//!     }
//!
//!     // Complete upload
//!     let file_item = storage
//!         .complete_multipart_upload(&upload_id, parts)
//!         .await?;
//!
//!     println!("Uploaded {} bytes in {} parts", file_item.size, file_data.chunks(chunk_size).count());
//!     Ok(())
//! }
//! ```
//!
//! ## 5. Batch Operations
//!
//! ```ignore
//! use paladin_ports::output::file_storage_port::{
//!     BatchFileStoragePort, FileStorageError
//! };
//! use std::path::PathBuf;
//! use std::sync::Arc;
//!
//! async fn batch_upload(
//!     storage: Arc<dyn BatchFileStoragePort>,
//!     files: Vec<(String, Vec<u8>)>,
//! ) -> Result<(), FileStorageError> {
//!     let uploads: Vec<_> = files
//!         .into_iter()
//!         .map(|(name, content)| (PathBuf::from(name), content, None))
//!         .collect();
//!
//!     let results = storage.upload_files(uploads).await?;
//!     println!("Uploaded {} files", results.len());
//!     Ok(())
//! }
//! ```
//!
//! # Storage Backend Comparison
//!
//! | Backend | Best For | Pros | Cons | Adapter |
//! |---------|----------|------|------|---------|
//! | **Local FS** | Development, testing | Simple, fast, no setup | No redundancy, single server | `LocalFileStorageAdapter` |
//! | **MinIO** | Self-hosted S3 | S3-compatible, self-hosted, cost-effective | Requires infrastructure | `MinioAdapter` |
//! | **AWS S3** | Production cloud | Highly available, durable (11 9's), global | Cost scales with usage | `S3Adapter` |
//! | **Google Cloud Storage** | Google Cloud ecosystem | Global CDN, strong consistency | Google Cloud lock-in | `GcsAdapter` |
//! | **Azure Blob Storage** | Azure ecosystem | Azure integration, hot/cool tiers | Azure lock-in | `AzureBlobAdapter` |
//!
//! # Error Handling & Retryability
//!
//! FileStorageError variants indicate whether operations should be retried:
//!
//! | Error | Retryable? | Recovery Strategy |
//! |-------|------------|-------------------|
//! | FileNotFound | No | Check path, create file |
//! | PermissionDenied | No | Fix credentials/IAM permissions |
//! | QuotaExceeded | No | Increase quota or delete old files |
//! | FileTooLarge | No | Split file or increase size limit |
//! | InvalidPath | No | Validate and sanitize path |
//! | BucketNotFound | No | Create bucket or fix configuration |
//! | ConnectionError | Yes | Retry with exponential backoff |
//! | AuthenticationError | No | Fix credentials |
//! | SerializationError | No | Fix data format |
//! | IoError | Maybe | Retry, check disk/network |
//! | ConfigurationError | No | Fix configuration |
//! | Timeout | Yes | Retry with longer timeout |
//! | ServiceUnavailable | Yes | Exponential backoff, circuit breaker |
//!
//! ## Retry Pattern Example
//!
//! ```ignore
//! use paladin_ports::output::file_storage_port::{
//!     FileStoragePort, FileStorageError
//! };
//! use std::path::Path;
//! use std::sync::Arc;
//! use std::time::Duration;
//! use tokio::time::sleep;
//!
//! async fn download_with_retry(
//!     storage: Arc<dyn FileStoragePort>,
//!     path: &Path,
//!     max_retries: u32,
//! ) -> Result<Vec<u8>, FileStorageError> {
//!     let mut attempts = 0;
//!     let mut backoff = Duration::from_millis(100);
//!
//!     loop {
//!         match storage.download_file(path, None).await {
//!             Ok(data) => return Ok(data),
//!             Err(e) if attempts >= max_retries => return Err(e),
//!             Err(FileStorageError::ConnectionError(_)) => {
//!                 sleep(backoff).await;
//!                 backoff *= 2;
//!             }
//!             Err(FileStorageError::ServiceUnavailable) => {
//!                 sleep(backoff).await;
//!                 backoff *= 2;
//!             }
//!             Err(e) => return Err(e), // Non-retryable
//!         }
//!         attempts += 1;
//!     }
//! }
//! ```
//!
//! # Thread Safety
//!
//! All file storage ports are `Send + Sync`, allowing safe use across async task boundaries.
//! This is critical for Paladin's concurrent agent execution where multiple agents may
//! read/write files simultaneously.
//!
//! # Implementation Notes
//!
//! ## Adapter Implementation Checklist
//!
//! When implementing a file storage adapter:
//!
//! 1. **Path Validation**: Use `FileStorageUtils::validate_path()` to prevent directory traversal
//! 2. **Content Type Detection**: Auto-detect MIME types from file extensions
//! 3. **MD5 Hashing**: Calculate and verify MD5 hashes for data integrity
//! 4. **Error Mapping**: Map provider errors to appropriate `FileStorageError` variants
//! 5. **Metadata**: Preserve user metadata across operations
//! 6. **Health Checks**: Implement `health_check()` for monitoring
//! 7. **Timeouts**: Set reasonable operation timeouts (30s upload, 60s download typical)
//! 8. **Connection Pooling**: Reuse connections to cloud providers
//! 9. **Logging**: Log operations with file paths, sizes, errors for debugging
//! 10. **Security**: Never log file content, sanitize paths, validate content types
//!
//! ## Performance Considerations
//!
//! - **Streaming**: Use streaming APIs for large files instead of loading into memory
//! - **Batch Operations**: Implement `BatchFileStoragePort` for bulk operations
//! - **Multipart Upload**: Use multipart for files > 5MB to improve reliability
//! - **Async I/O**: Use non-blocking I/O for all storage operations
//! - **Connection Pooling**: Reuse HTTP clients and connections
//! - **Compression**: Compress before upload when appropriate
//! - **CDN**: Use CloudFront/CDN for frequently accessed files
//! - **Lifecycle Policies**: Auto-delete or archive old files
//!
//! ## Testing Strategy
//!
//! ```ignore
//! use paladin_ports::output::file_storage_port::{
//!     FileStoragePort, FileItem, FileStorageResult, FileStorageError,
//!     ListOptions, FileListResult, UploadOptions, DownloadOptions,
//!     StorageStats, StorageHealth
//! };
//! use async_trait::async_trait;
//! use std::path::Path;
//! use std::collections::HashMap;
//!
//! /// Mock file storage for testing
//! struct MockFileStorage {
//!     files: std::sync::RwLock<HashMap<String, Vec<u8>>>,
//!     should_fail: bool,
//! }
//!
//! #[async_trait]
//! impl FileStoragePort for MockFileStorage {
//!     async fn upload_file(
//!         &self,
//!         path: &Path,
//!         content: &[u8],
//!         options: Option<UploadOptions>,
//!     ) -> FileStorageResult<FileItem> {
//!         if self.should_fail {
//!             return Err(FileStorageError::ServiceUnavailable);
//!         }
//!
//!         let mut files = self.files.write().unwrap();
//!         files.insert(path.to_string_lossy().to_string(), content.to_vec());
//!
//!         Ok(FileItem::new(path.to_path_buf(), content.len() as u64))
//!     }
//!
//!     async fn download_file(
//!         &self,
//!         path: &Path,
//!         options: Option<DownloadOptions>,
//!     ) -> FileStorageResult<Vec<u8>> {
//!         let files = self.files.read().unwrap();
//!         files
//!             .get(&path.to_string_lossy().to_string())
//!             .cloned()
//!             .ok_or_else(|| FileStorageError::FileNotFound(path.to_string_lossy().to_string()))
//!     }
//!
//!     async fn delete_file(&self, path: &Path) -> FileStorageResult<()> {
//!         let mut files = self.files.write().unwrap();
//!         files.remove(&path.to_string_lossy().to_string());
//!         Ok(())
//!     }
//!
//!     async fn file_exists(&self, path: &Path) -> FileStorageResult<bool> {
//!         let files = self.files.read().unwrap();
//!         Ok(files.contains_key(&path.to_string_lossy().to_string()))
//!     }
//!
//!     async fn get_file_info(&self, path: &Path) -> FileStorageResult<FileItem> {
//!         let files = self.files.read().unwrap();
//!         let content = files
//!             .get(&path.to_string_lossy().to_string())
//!             .ok_or_else(|| FileStorageError::FileNotFound(path.to_string_lossy().to_string()))?;
//!
//!         Ok(FileItem::new(path.to_path_buf(), content.len() as u64))
//!     }
//!
//!     async fn list_files(&self, options: Option<ListOptions>) -> FileStorageResult<FileListResult> {
//!         let files = self.files.read().unwrap();
//!         let file_items: Vec<FileItem> = files
//!             .iter()
//!             .map(|(path, content)| {
//!                 FileItem::new(std::path::PathBuf::from(path), content.len() as u64)
//!             })
//!             .collect();
//!
//!         Ok(FileListResult {
//!             files: file_items,
//!             continuation_token: None,
//!             has_more: false,
//!             total_count: Some(files.len() as u64),
//!         })
//!     }
//!
//!     async fn copy_file(
//!         &self,
//!         source_path: &Path,
//!         destination_path: &Path,
//!     ) -> FileStorageResult<FileItem> {
//!         let content = self.download_file(source_path, None).await?;
//!         self.upload_file(destination_path, &content, None).await
//!     }
//!
//!     async fn move_file(
//!         &self,
//!         source_path: &Path,
//!         destination_path: &Path,
//!     ) -> FileStorageResult<FileItem> {
//!         let file_item = self.copy_file(source_path, destination_path).await?;
//!         self.delete_file(source_path).await?;
//!         Ok(file_item)
//!     }
//!
//!     async fn get_storage_stats(&self) -> FileStorageResult<StorageStats> {
//!         let files = self.files.read().unwrap();
//!         Ok(StorageStats {
//!             total_files: files.len() as u64,
//!             total_size: files.values().map(|v| v.len() as u64).sum(),
//!             files_by_type: HashMap::new(),
//!             size_by_type: HashMap::new(),
//!             last_updated: chrono::Utc::now(),
//!         })
//!     }
//!
//!     async fn health_check(&self) -> FileStorageResult<StorageHealth> {
//!         Ok(StorageHealth {
//!             is_available: !self.should_fail,
//!             response_time_ms: Some(5),
//!             error: if self.should_fail {
//!                 Some("Mock failure".into())
//!             } else {
//!                 None
//!             },
//!             checked_at: chrono::Utc::now(),
//!         })
//!     }
//! }
//! ```
//!
//! # Common Pitfalls
//!
//! 1. **Not Validating Paths**: Always validate paths to prevent directory traversal attacks
//! 2. **Loading Large Files**: Stream files instead of loading entire content into memory
//! 3. **Ignoring Errors**: Handle `FileNotFound`, `PermissionDenied` errors appropriately
//! 4. **Missing Content Types**: Always set content type for proper browser handling
//! 5. **No Rate Limiting**: Implement rate limiting for uploads to prevent abuse
//! 6. **Insecure Pre-Signed URLs**: Set short expiration times on pre-signed URLs
//! 7. **Not Using Multipart**: Use multipart uploads for files > 5MB for reliability
//! 8. **Blocking Operations**: All storage operations must be async to avoid blocking agents
//!
//! # Related Modules
//!
//! - [`crate::output::citadel_port`] - State persistence (uses FileStoragePort)
//! - [`crate::output::sanctum_port`] - Vector storage (stores embeddings)
//! - [`crate::output::garrison_port`] - Conversation memory (may use file storage)
//! - `crate::infrastructure::adapters::file_storage` - Concrete file storage adapters (MinIO, S3, Local)

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use md5::compute;
use mime_guess::from_path;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

/// Result type for file storage operations
pub type FileStorageResult<T> = Result<T, FileStorageError>;

/// Errors that can occur during file storage operations
///
/// This enum represents all possible error conditions when interacting with file storage
/// systems. Each variant indicates a specific failure mode and provides guidance on
/// whether the operation should be retried.
///
/// # Error Categories
///
/// - **Transient Errors**: Can be retried (ConnectionError, ServiceUnavailable, Timeout)
/// - **Permanent Errors**: Should not be retried (FileNotFound, PermissionDenied, QuotaExceeded)
/// - **Contextual Errors**: Retry depends on context (IoError)
///
/// # Examples
///
/// ```
/// use paladin_ports::output::file_storage_port::FileStorageError;
///
/// // Check if an error should be retried
/// fn should_retry(error: &FileStorageError) -> bool {
///     matches!(
///         error,
///         FileStorageError::ConnectionError(_)
///             | FileStorageError::ServiceUnavailable
///             | FileStorageError::Timeout
///     )
/// }
/// ```
#[derive(Debug, Clone, Error)]
pub enum FileStorageError {
    /// File not found at specified path
    ///
    /// **Retryable**: No - Check path or create file
    ///
    /// **Recovery**: Verify path exists, create file, or handle missing file gracefully
    #[error("File not found: {0}")]
    FileNotFound(String),

    /// Permission denied for storage operation
    ///
    /// **Retryable**: No - Fix credentials or IAM permissions
    ///
    /// **Recovery**: Check bucket policies, IAM roles, access keys
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Storage quota exceeded
    ///
    /// **Retryable**: No - Increase quota or delete old files
    ///
    /// **Recovery**: Delete unused files, increase storage quota, implement lifecycle policies
    #[error("Storage quota exceeded")]
    QuotaExceeded,

    /// File size exceeds maximum allowed
    ///
    /// **Retryable**: No - Split file or increase size limit
    ///
    /// **Recovery**: Use multipart upload, compress file, or increase size limit
    ///
    /// # Example
    /// ```
    /// use paladin_ports::output::file_storage_port::FileStorageError;
    ///
    /// fn handle_file_size(error: &FileStorageError) {
    ///     if let FileStorageError::FileTooLarge { size, max_size } = error {
    ///         println!("File {} bytes exceeds max {} bytes", size, max_size);
    ///         // Consider using multipart upload
    ///     }
    /// }
    /// ```
    #[error("File too large: {size} bytes (max: {max_size} bytes)")]
    FileTooLarge {
        /// Actual file size
        size: u64,
        /// Maximum allowed size
        max_size: u64,
    },

    /// Invalid file path (contains .., starts with /, too long, etc.)
    ///
    /// **Retryable**: No - Validate and sanitize path
    ///
    /// **Recovery**: Use `FileStorageUtils::validate_path()` before operations
    #[error("Invalid file path: {0}")]
    InvalidPath(String),

    /// Storage bucket/container not found
    ///
    /// **Retryable**: No - Create bucket or fix configuration
    ///
    /// **Recovery**: Verify bucket name in configuration, create bucket if needed
    #[error("Bucket not found: {0}")]
    BucketNotFound(String),

    /// Network connection error
    ///
    /// **Retryable**: Yes - Retry with exponential backoff
    ///
    /// **Recovery**: Check network connectivity, DNS resolution, firewall rules
    #[error("Connection error: {0}")]
    ConnectionError(String),

    /// Authentication or authorization failed
    ///
    /// **Retryable**: No - Fix credentials
    ///
    /// **Recovery**: Verify access keys, OAuth tokens, or credential configuration
    #[error("Authentication failed: {0}")]
    AuthenticationError(String),

    /// Failed to serialize/deserialize data
    ///
    /// **Retryable**: No - Fix data format
    ///
    /// **Recovery**: Validate data structure before storage operations
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// I/O operation failed
    ///
    /// **Retryable**: Maybe - Depends on error details
    ///
    /// **Recovery**: Check disk space, permissions, network for network-backed filesystems
    #[error("IO error: {0}")]
    IoError(String),

    /// Configuration error (invalid settings)
    ///
    /// **Retryable**: No - Fix configuration
    ///
    /// **Recovery**: Validate storage configuration at startup
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// Operation timed out
    ///
    /// **Retryable**: Yes - Retry with longer timeout
    ///
    /// **Recovery**: Increase timeout, check network latency, use multipart for large files
    #[error("Operation timeout")]
    Timeout,

    /// Storage service temporarily unavailable
    ///
    /// **Retryable**: Yes - Use circuit breaker pattern
    ///
    /// **Recovery**: Exponential backoff, circuit breaker, health checks
    #[error("Service unavailable")]
    ServiceUnavailable,

    /// Unknown or unexpected error
    ///
    /// **Retryable**: Maybe - Depends on context
    ///
    /// **Recovery**: Log error details, investigate root cause
    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Represents a file item in storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileItem {
    /// Unique identifier for the file
    pub id: Uuid,
    /// File path relative to bucket/container
    pub path: PathBuf,
    /// File size in bytes
    pub size: u64,
    /// MIME type of the file
    pub content_type: Option<String>,
    /// MD5 hash of file content
    pub md5_hash: Option<String>,
    /// When the file was uploaded
    pub uploaded_at: DateTime<Utc>,
    /// When the file was last modified
    pub modified_at: DateTime<Utc>,
    /// Custom metadata associated with the file
    pub metadata: HashMap<String, String>,
    /// File tags for categorization
    pub tags: Vec<String>,
}

impl FileItem {
    /// Create a new FileItem
    pub fn new(path: PathBuf, size: u64) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            path,
            size,
            content_type: None,
            md5_hash: None,
            uploaded_at: now,
            modified_at: now,
            metadata: HashMap::new(),
            tags: Vec::new(),
        }
    }

    /// Set content type
    pub fn with_content_type(mut self, content_type: String) -> Self {
        self.content_type = Some(content_type);
        self
    }

    /// Set MD5 hash
    pub fn with_md5_hash(mut self, hash: String) -> Self {
        self.md5_hash = Some(hash);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Add tag
    pub fn with_tag(mut self, tag: String) -> Self {
        self.tags.push(tag);
        self
    }

    /// Get file extension
    pub fn extension(&self) -> Option<&str> {
        self.path.extension().and_then(|ext| ext.to_str())
    }

    /// Get filename
    pub fn filename(&self) -> Option<&str> {
        self.path.file_name().and_then(|name| name.to_str())
    }
}

/// File listing options
#[derive(Debug, Clone, Default)]
pub struct ListOptions {
    /// Path prefix to filter by
    pub prefix: Option<String>,
    /// Maximum number of files to return
    pub limit: Option<usize>,
    /// Continuation token for pagination
    pub continuation_token: Option<String>,
    /// Whether to include metadata in results
    pub include_metadata: bool,
    /// Filter by tags
    pub tags: Vec<String>,
    /// Filter by file extension
    pub extensions: Vec<String>,
    /// Minimum file size filter
    pub min_size: Option<u64>,
    /// Maximum file size filter
    pub max_size: Option<u64>,
    /// Modified after date filter
    pub modified_after: Option<DateTime<Utc>>,
    /// Modified before date filter
    pub modified_before: Option<DateTime<Utc>>,
}

/// File listing result with pagination support
#[derive(Debug, Clone)]
pub struct FileListResult {
    /// List of files
    pub files: Vec<FileItem>,
    /// Continuation token for next page
    pub continuation_token: Option<String>,
    /// Whether there are more results
    pub has_more: bool,
    /// Total count (if available)
    pub total_count: Option<u64>,
}

/// Upload options for file operations
#[derive(Debug, Clone, Default)]
pub struct UploadOptions {
    /// Content type override
    pub content_type: Option<String>,
    /// Custom metadata
    pub metadata: HashMap<String, String>,
    /// Tags to assign to the file
    pub tags: Vec<String>,
    /// Whether to overwrite existing files
    pub overwrite: bool,
    /// Server-side encryption settings
    pub encryption: Option<EncryptionOptions>,
    /// Cache control settings
    pub cache_control: Option<String>,
    /// Content disposition
    pub content_disposition: Option<String>,
}

/// Encryption options for file storage
#[derive(Debug, Clone)]
pub enum EncryptionOptions {
    /// Server-side encryption with service-managed keys
    ServerSideEncryption,
    /// Server-side encryption with customer-provided keys
    ServerSideEncryptionCustomerKey {
        /// Customer-provided encryption key.
        key: String,
    },
    /// Server-side encryption with KMS
    ServerSideEncryptionKms {
        /// KMS key identifier.
        key_id: String,
    },
}

/// Download options
#[derive(Debug, Clone, Default)]
pub struct DownloadOptions {
    /// Range to download (start, end) in bytes
    pub range: Option<(u64, Option<u64>)>,
    /// Only download if modified since date
    pub if_modified_since: Option<DateTime<Utc>>,
    /// Only download if not modified since date
    pub if_unmodified_since: Option<DateTime<Utc>>,
    /// Expected ETag value
    pub if_match: Option<String>,
    /// Expected ETag value (must not match)
    pub if_none_match: Option<String>,
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    /// Total number of files
    pub total_files: u64,
    /// Total storage used in bytes
    pub total_size: u64,
    /// Number of files by type
    pub files_by_type: HashMap<String, u64>,
    /// Storage used by type
    pub size_by_type: HashMap<String, u64>,
    /// Last update time
    pub last_updated: DateTime<Utc>,
}

/// File storage health information
#[derive(Debug, Clone)]
pub struct StorageHealth {
    /// Whether the storage is available
    pub is_available: bool,
    /// Response time in milliseconds
    pub response_time_ms: Option<u64>,
    /// Error message if unavailable
    pub error: Option<String>,
    /// Last health check time
    pub checked_at: DateTime<Utc>,
}

/// Main file storage port trait
///
/// This port defines the core file storage functionality that all storage adapters must
/// implement. It provides CRUD operations for files along with metadata management,
/// health checks, and storage statistics.
///
/// # Capabilities
///
/// - **Upload/Download**: Store and retrieve files with metadata
/// - **Delete**: Remove files from storage
/// - **Existence Check**: Verify if file exists before operations
/// - **Metadata Queries**: Get file info without downloading content
/// - **Listing**: Browse files with filtering and pagination
/// - **Copy/Move**: Duplicate or relocate files within storage
/// - **Statistics**: Track storage usage and file counts
/// - **Health Monitoring**: Check storage availability
///
/// # Requirements
///
/// Implementations must:
/// - Be `Send + Sync` for safe concurrent use across async tasks
/// - Validate paths using `FileStorageUtils::validate_path()` to prevent traversal attacks
/// - Calculate MD5 hashes for uploaded files
/// - Auto-detect content types from file extensions
/// - Handle metadata preservation across operations
/// - Implement timeouts for all operations (30-60s typical)
/// - Return detailed error context in `FileStorageError`
///
/// # Examples
///
/// ## Basic File Upload and Download
///
/// ```ignore
/// use paladin_ports::output::file_storage_port::{
///     FileStoragePort, UploadOptions, FileStorageError
/// };
/// use std::path::Path;
/// use std::sync::Arc;
///
/// async fn store_and_retrieve(
///     storage: Arc<dyn FileStoragePort>,
/// ) -> Result<(), FileStorageError> {
///     // Upload file
///     let path = Path::new("documents/report.pdf");
///     let content = b"PDF content here...";
///
///     let options = UploadOptions {
///         content_type: Some("application/pdf".into()),
///         overwrite: false,
///         ..Default::default()
///     };
///
///     let file_item = storage.upload_file(path, content, Some(options)).await?;
///     println!("Uploaded: {} ({} bytes)", file_item.path.display(), file_item.size);
///
///     // Download file
///     let downloaded = storage.download_file(path, None).await?;
///     assert_eq!(downloaded, content);
///     Ok(())
/// }
/// ```
///
/// ## List Files with Filtering
///
/// ```ignore
/// use paladin_ports::output::file_storage_port::{
///     FileStoragePort, ListOptions
/// };
/// use std::sync::Arc;
///
/// async fn list_json_files(storage: Arc<dyn FileStoragePort>) {
///     let options = ListOptions {
///         prefix: Some("data/".into()),
///         extensions: vec!["json".into()],
///         limit: Some(100),
///         ..Default::default()
///     };
///
///     let result = storage.list_files(Some(options)).await.unwrap();
///     for file in result.files {
///         println!("{}: {} bytes", file.path.display(), file.size);
///     }
/// }
/// ```
///
/// ## Copy and Move Files
///
/// ```ignore
/// use paladin_ports::output::file_storage_port::FileStoragePort;
/// use std::path::Path;
/// use std::sync::Arc;
///
/// async fn organize_files(storage: Arc<dyn FileStoragePort>) {
///     let source = Path::new("uploads/temp.txt");
///     let backup = Path::new("backups/temp.txt");
///     let final_dest = Path::new("archived/document.txt");
///
///     // Create backup copy
///     storage.copy_file(source, backup).await.unwrap();
///
///     // Move to final location
///     storage.move_file(source, final_dest).await.unwrap();
/// }
/// ```
///
/// # Implementation Notes
///
/// ## Path Security
///
/// Always validate paths to prevent directory traversal:
/// ```ignore
/// use paladin_ports::output::file_storage_port::FileStorageUtils;
/// use std::path::Path;
///
/// fn validate_upload_path(path: &Path) -> Result<(), String> {
///     FileStorageUtils::validate_path(path)
///         .map_err(|e| format!("Invalid path: {}", e))
/// }
/// ```
///
/// ## Content Type Detection
///
/// Auto-detect MIME types from extensions:
/// ```ignore
/// use paladin_ports::output::file_storage_port::FileStorageUtils;
/// use std::path::Path;
///
/// let path = Path::new("document.pdf");
/// let content_type = FileStorageUtils::detect_content_type(path);
/// assert_eq!(content_type, Some("application/pdf".to_string()));
/// ```
///
/// ## Error Handling
///
/// Map provider errors appropriately:
/// - S3 NoSuchKey → `FileNotFound`
/// - S3 AccessDenied → `PermissionDenied`
/// - Network errors → `ConnectionError`
/// - Timeout → `Timeout`
///
/// # Performance Tips
///
/// 1. **Streaming**: Use streaming APIs for large files (don't load into memory)
/// 2. **Batch Operations**: Implement `BatchFileStoragePort` for bulk uploads
/// 3. **Connection Pooling**: Reuse HTTP clients for cloud storage
/// 4. **Async I/O**: All operations must be non-blocking
/// 5. **Health Checks**: Cache health check results for 30-60 seconds
#[async_trait]
pub trait FileStoragePort: Send + Sync {
    /// Upload a file to storage
    ///
    /// Stores file content at the specified path with optional metadata and upload options.
    /// Calculates MD5 hash and detects content type automatically if not provided.
    ///
    /// # Arguments
    ///
    /// * `path` - Relative path where file should be stored (e.g., "documents/report.pdf")
    /// * `content` - File content as bytes
    /// * `options` - Optional upload configuration (content type, metadata, tags, overwrite)
    ///
    /// # Returns
    ///
    /// `FileItem` with metadata including ID, size, MD5 hash, upload timestamp
    ///
    /// # Errors
    ///
    /// - `InvalidPath`: Path contains .., starts with /, or is too long
    /// - `FileTooLarge`: Content exceeds storage limits
    /// - `PermissionDenied`: No write permission for path
    /// - `QuotaExceeded`: Storage quota exceeded
    /// - `ConnectionError`: Network issue
    /// - `ServiceUnavailable`: Storage backend down
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let content = b"Hello, world!";
    /// let options = UploadOptions {
    ///     content_type: Some("text/plain".into()),
    ///     metadata: [("author".into(), "Alice".into())].into(),
    ///     overwrite: true,
    ///     ..Default::default()
    /// };
    ///
    /// let file = storage.upload_file(Path::new("greeting.txt"), content, Some(options)).await?;
    /// println!("Uploaded: {}", file.id);
    /// ```
    async fn upload_file(
        &self,
        path: &Path,
        content: &[u8],
        options: Option<UploadOptions>,
    ) -> FileStorageResult<FileItem>;

    /// Download a file from storage
    ///
    /// Retrieves file content from storage. Supports range downloads for large files.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to file (e.g., "documents/report.pdf")
    /// * `options` - Optional download configuration (range, conditional headers)
    ///
    /// # Returns
    ///
    /// File content as bytes
    ///
    /// # Errors
    ///
    /// - `FileNotFound`: File doesn't exist at path
    /// - `PermissionDenied`: No read permission
    /// - `ConnectionError`: Network issue
    /// - `Timeout`: Download took too long
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Full download
    /// let content = storage.download_file(Path::new("data.json"), None).await?;
    ///
    /// // Range download (first 1KB)
    /// let options = DownloadOptions {
    ///     range: Some((0, Some(1024))),
    ///     ..Default::default()
    /// };
    /// let chunk = storage.download_file(Path::new("large.bin"), Some(options)).await?;
    /// ```
    async fn download_file(
        &self,
        path: &Path,
        options: Option<DownloadOptions>,
    ) -> FileStorageResult<Vec<u8>>;

    /// Delete a file from storage
    ///
    /// Removes the file at the specified path. This operation cannot be undone.
    ///
    /// # Errors
    ///
    /// - `FileNotFound`: File doesn't exist (may be OK depending on use case)
    /// - `PermissionDenied`: No delete permission
    ///
    /// # Examples
    ///
    /// ```ignore
    /// storage.delete_file(Path::new("temp/old.txt")).await?;
    /// ```
    async fn delete_file(&self, path: &Path) -> FileStorageResult<()>;

    /// Check if a file exists
    ///
    /// Verifies file existence without downloading content. Useful before operations.
    ///
    /// # Returns
    ///
    /// `true` if file exists, `false` otherwise
    ///
    /// # Examples
    ///
    /// ```ignore
    /// if storage.file_exists(Path::new("config.json")).await? {
    ///     println!("Config file found");
    /// } else {
    ///     println!("Creating default config");
    /// }
    /// ```
    async fn file_exists(&self, path: &Path) -> FileStorageResult<bool>;

    /// Get file metadata without downloading content
    ///
    /// Retrieves file information (size, content type, upload time) without transferring
    /// the file content. Much faster than downloading for large files.
    ///
    /// # Returns
    ///
    /// `FileItem` with metadata
    ///
    /// # Errors
    ///
    /// - `FileNotFound`: File doesn't exist
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let info = storage.get_file_info(Path::new("video.mp4")).await?;
    /// println!("Size: {} MB", info.size / 1024 / 1024);
    /// println!("Type: {}", info.content_type.unwrap_or_default());
    /// ```
    async fn get_file_info(&self, path: &Path) -> FileStorageResult<FileItem>;

    /// List files in storage
    ///
    /// Returns a list of files matching the specified filters with pagination support.
    ///
    /// # Arguments
    ///
    /// * `options` - Optional filters (prefix, limit, extensions, size range, date range)
    ///
    /// # Returns
    ///
    /// `FileListResult` with files and pagination token
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // List all files
    /// let all_files = storage.list_files(None).await?;
    ///
    /// // List PDFs in documents/ folder
    /// let options = ListOptions {
    ///     prefix: Some("documents/".into()),
    ///     extensions: vec!["pdf".into()],
    ///     limit: Some(50),
    ///     ..Default::default()
    /// };
    /// let pdfs = storage.list_files(Some(options)).await?;
    /// ```
    async fn list_files(&self, options: Option<ListOptions>) -> FileStorageResult<FileListResult>;

    /// Copy a file within storage
    ///
    /// Creates a copy of the file at a new path. More efficient than download+upload
    /// for cloud storage (server-side copy).
    ///
    /// # Returns
    ///
    /// `FileItem` for the copied file
    ///
    /// # Errors
    ///
    /// - `FileNotFound`: Source file doesn't exist
    /// - `PermissionDenied`: No copy permission
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let copy = storage
    ///     .copy_file(Path::new("data.json"), Path::new("data-backup.json"))
    ///     .await?;
    /// ```
    async fn copy_file(
        &self,
        source_path: &Path,
        destination_path: &Path,
    ) -> FileStorageResult<FileItem>;

    /// Move/rename a file within storage
    ///
    /// Moves the file to a new path, removing it from the original location.
    /// More efficient than copy+delete for cloud storage.
    ///
    /// # Returns
    ///
    /// `FileItem` for the moved file
    ///
    /// # Errors
    ///
    /// - `FileNotFound`: Source file doesn't exist
    /// - `PermissionDenied`: No move/delete permission
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let moved = storage
    ///     .move_file(Path::new("temp/upload.pdf"), Path::new("documents/report.pdf"))
    ///     .await?;
    /// ```
    async fn move_file(
        &self,
        source_path: &Path,
        destination_path: &Path,
    ) -> FileStorageResult<FileItem>;

    /// Get storage statistics
    ///
    /// Returns aggregated statistics about storage usage including total files,
    /// total size, and breakdowns by file type.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let stats = storage.get_storage_stats().await?;
    /// println!("Total files: {}", stats.total_files);
    /// println!("Total size: {} GB", stats.total_size / 1024 / 1024 / 1024);
    /// ```
    async fn get_storage_stats(&self) -> FileStorageResult<StorageStats>;

    /// Health check for storage service
    ///
    /// Performs a lightweight check to verify storage is accessible and responsive.
    /// Used for monitoring, circuit breakers, and readiness probes.
    ///
    /// # Returns
    ///
    /// `StorageHealth` with availability status and response time
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let health = storage.health_check().await?;
    /// if !health.is_available {
    ///     eprintln!("Storage unavailable: {}", health.error.unwrap_or_default());
    /// }
    /// ```
    async fn health_check(&self) -> FileStorageResult<StorageHealth>;
}

/// Batch file operations port
///
/// # Examples
///
/// ```rust
/// use paladin_ports::output::file_storage_port::{BatchFileStoragePort, FileStorageResult};
/// use std::path::PathBuf;
///
/// async fn upload_report_bundle(
///     storage: &dyn BatchFileStoragePort,
///     files: Vec<(PathBuf, Vec<u8>)>,
/// ) -> FileStorageResult<usize> {
///     let uploads = files
///         .into_iter()
///         .map(|(path, content)| (path, content, None))
///         .collect();
///
///     let uploaded = storage.upload_files(uploads).await?;
///     Ok(uploaded.len())
/// }
/// ```
#[async_trait]
pub trait BatchFileStoragePort: Send + Sync {
    /// Upload multiple files
    async fn upload_files(
        &self,
        files: Vec<(PathBuf, Vec<u8>, Option<UploadOptions>)>,
    ) -> FileStorageResult<Vec<FileItem>>;

    /// Download multiple files
    async fn download_files(
        &self,
        paths: Vec<PathBuf>,
        options: Option<DownloadOptions>,
    ) -> FileStorageResult<Vec<(PathBuf, Vec<u8>)>>;

    /// Delete multiple files
    async fn delete_files(&self, paths: Vec<PathBuf>) -> FileStorageResult<Vec<PathBuf>>;

    /// Get info for multiple files
    async fn get_files_info(&self, paths: Vec<PathBuf>) -> FileStorageResult<Vec<FileItem>>;
}

/// Advanced file operations port
///
/// # Examples
///
/// ```rust
/// use paladin_ports::output::file_storage_port::{AdvancedFileStoragePort, FileStorageResult};
/// use std::path::Path;
/// use std::time::Duration;
///
/// async fn share_download_link(
///     storage: &dyn AdvancedFileStoragePort,
///     path: &Path,
/// ) -> FileStorageResult<String> {
///     storage
///         .generate_download_url(path, Duration::from_secs(3600), None)
///         .await
/// }
/// ```
#[async_trait]
pub trait AdvancedFileStoragePort: Send + Sync {
    /// Generate a pre-signed URL for file upload
    async fn generate_upload_url(
        &self,
        path: &Path,
        expires_in: std::time::Duration,
        options: Option<UploadOptions>,
    ) -> FileStorageResult<String>;

    /// Generate a pre-signed URL for file download
    async fn generate_download_url(
        &self,
        path: &Path,
        expires_in: std::time::Duration,
        options: Option<DownloadOptions>,
    ) -> FileStorageResult<String>;

    /// Create a multipart upload session
    async fn create_multipart_upload(
        &self,
        path: &Path,
        options: Option<UploadOptions>,
    ) -> FileStorageResult<String>;

    /// Upload a part in multipart upload
    async fn upload_part(
        &self,
        upload_id: &str,
        part_number: u32,
        content: &[u8],
    ) -> FileStorageResult<String>;

    /// Complete multipart upload
    async fn complete_multipart_upload(
        &self,
        upload_id: &str,
        parts: Vec<(u32, String)>,
    ) -> FileStorageResult<FileItem>;

    /// Abort multipart upload
    async fn abort_multipart_upload(&self, upload_id: &str) -> FileStorageResult<()>;
}

/// File versioning port
///
/// # Examples
///
/// ```rust
/// use paladin_ports::output::file_storage_port::{FileVersioningPort, FileStorageResult};
/// use std::path::Path;
///
/// async fn upload_new_revision(
///     storage: &dyn FileVersioningPort,
///     path: &Path,
///     content: &[u8],
/// ) -> FileStorageResult<usize> {
///     storage.upload_file_version(path, content, None).await?;
///     let versions = storage.list_file_versions(path).await?;
///     Ok(versions.len())
/// }
/// ```
#[async_trait]
pub trait FileVersioningPort: Send + Sync {
    /// Upload a new version of a file
    async fn upload_file_version(
        &self,
        path: &Path,
        content: &[u8],
        options: Option<UploadOptions>,
    ) -> FileStorageResult<FileItem>;

    /// List all versions of a file
    async fn list_file_versions(&self, path: &Path) -> FileStorageResult<Vec<FileItem>>;

    /// Download a specific version of a file
    async fn download_file_version(
        &self,
        path: &Path,
        version_id: &str,
        options: Option<DownloadOptions>,
    ) -> FileStorageResult<Vec<u8>>;

    /// Delete a specific version of a file
    async fn delete_file_version(&self, path: &Path, version_id: &str) -> FileStorageResult<()>;

    /// Get info for a specific version
    async fn get_file_version_info(
        &self,
        path: &Path,
        version_id: &str,
    ) -> FileStorageResult<FileItem>;
}

/// Combined file storage port with all capabilities
///
/// # Examples
///
/// This combinator trait declares no methods of its own — it is a bound that grants access to
/// every constituent port's methods at once. [`FileStoragePort`], [`BatchFileStoragePort`],
/// [`AdvancedFileStoragePort`] and [`FileVersioningPort`] each document their own call pattern
/// above; a `FullFileStoragePort` implementor exposes all of them through one reference.
///
/// ```rust
/// use paladin_ports::output::file_storage_port::{FullFileStoragePort, FileStorageResult};
/// use std::path::Path;
///
/// async fn upload_then_list_versions(
///     storage: &dyn FullFileStoragePort,
///     path: &Path,
///     content: &[u8],
/// ) -> FileStorageResult<usize> {
///     // From `FileStoragePort` (base capability).
///     storage.upload_file(path, content, None).await?;
///     // From `FileVersioningPort` (combined capability).
///     let versions = storage.list_file_versions(path).await?;
///     Ok(versions.len())
/// }
/// ```
pub trait FullFileStoragePort:
    FileStoragePort + BatchFileStoragePort + AdvancedFileStoragePort + FileVersioningPort + Send + Sync
{
}

/// Helper trait for common file operations
pub trait FileStorageUtils {
    /// Detect content type from file extension
    fn detect_content_type(path: &Path) -> Option<String>;

    /// Detect content type from file extension with a fallback value.
    fn detect_content_type_with_fallback(path: &Path, fallback: &str) -> String;

    /// Validate that the content type at `path` matches one of the expected MIME types.
    fn validate_content_type_for_domain(
        path: &Path,
        expected_types: &[&str],
    ) -> FileStorageResult<String>;

    /// Generate MD5 hash of content
    fn calculate_md5(content: &[u8]) -> String;

    /// Validate file path
    fn validate_path(path: &Path) -> FileStorageResult<()>;

    /// Sanitize filename
    fn sanitize_filename(filename: &str) -> String;
}

impl FileStorageUtils for () {
    fn detect_content_type(path: &Path) -> Option<String> {
        Some(from_path(path).first_or_text_plain().to_string())
    }

    fn detect_content_type_with_fallback(path: &Path, fallback: &str) -> String {
        from_path(path)
            .first()
            .map(|mime| mime.to_string())
            .unwrap_or_else(|| fallback.to_string())
    }

    fn validate_content_type_for_domain(
        path: &Path,
        expected_types: &[&str],
    ) -> FileStorageResult<String> {
        let detected = Self::detect_content_type(path)
            .unwrap_or_else(|| "application/octet-stream".to_string());

        if expected_types.is_empty() || expected_types.contains(&detected.as_str()) {
            Ok(detected)
        } else {
            Err(FileStorageError::InvalidPath(format!(
                "File type '{}' not allowed. Expected one of: {:?}",
                detected, expected_types
            )))
        }
    }

    fn calculate_md5(content: &[u8]) -> String {
        let hasher = compute(content);
        format!("{:x}", hasher)
    }

    fn validate_path(path: &Path) -> FileStorageResult<()> {
        let path_str = path.to_string_lossy();

        // Check for invalid characters
        if path_str.contains("..") {
            return Err(FileStorageError::InvalidPath(
                "Path cannot contain '..'".to_string(),
            ));
        }

        if path_str.starts_with('/') {
            return Err(FileStorageError::InvalidPath(
                "Path cannot start with '/'".to_string(),
            ));
        }

        if path_str.is_empty() {
            return Err(FileStorageError::InvalidPath(
                "Path cannot be empty".to_string(),
            ));
        }

        // Check path length
        if path_str.len() > 1024 {
            return Err(FileStorageError::InvalidPath(
                "Path too long (max 1024 characters)".to_string(),
            ));
        }

        Ok(())
    }

    fn sanitize_filename(filename: &str) -> String {
        filename
            .chars()
            .map(|c| match c {
                '<' | '>' | ':' | '"' | '|' | '?' | '*' | '\\' | '/' => '_',
                c if c.is_control() => '_',
                c => c,
            })
            .collect::<String>()
            .trim()
            .to_string()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
//
// `FileStorageUtils` itself declares only trait signatures (no default
// bodies) -- the executable code the Phase 3 entry measurement records at
// 0.00% (`file_storage_port.rs` 104/104 lines missed,
// `.planning/phases/03-verification-depth/03-coverage-measurement.md`) is the
// `impl FileStorageUtils for ()` block above (lines ~1370-1447). `()` is
// already the zero-sized implementor the plan asked a test-local struct to
// provide; calling through it directly reaches the same bodies without a
// redundant delegating wrapper, so no extra type is declared here.
#[cfg(test)]
mod tests {
    use super::*;

    // -- detect_content_type -------------------------------------------

    #[test]
    fn detect_content_type_known_extension_returns_documented_mime() {
        let path = Path::new("document.pdf");
        assert_eq!(
            <() as FileStorageUtils>::detect_content_type(path),
            Some("application/pdf".to_string())
        );
    }

    #[test]
    fn detect_content_type_unknown_extension_returns_text_plain_fallback() {
        let path = Path::new("data.totally-unknown-extension-xyz");
        assert_eq!(
            <() as FileStorageUtils>::detect_content_type(path),
            Some("text/plain".to_string())
        );
    }

    #[test]
    fn detect_content_type_absent_extension_returns_text_plain_fallback() {
        let path = Path::new("no_extension_at_all");
        assert_eq!(
            <() as FileStorageUtils>::detect_content_type(path),
            Some("text/plain".to_string())
        );
    }

    // -- detect_content_type_with_fallback ------------------------------

    #[test]
    fn detect_content_type_with_fallback_known_extension_ignores_fallback() {
        let path = Path::new("payload.json");
        assert_eq!(
            <() as FileStorageUtils>::detect_content_type_with_fallback(
                path,
                "application/x-should-not-be-used"
            ),
            "application/json".to_string()
        );
    }

    #[test]
    fn detect_content_type_with_fallback_unknown_extension_returns_fallback() {
        let path = Path::new("data.totally-unknown-extension-xyz");
        assert_eq!(
            <() as FileStorageUtils>::detect_content_type_with_fallback(
                path,
                "application/x-custom-fallback"
            ),
            "application/x-custom-fallback".to_string()
        );
    }

    // -- validate_content_type_for_domain -------------------------------

    #[test]
    fn validate_content_type_for_domain_accepts_expected_type() {
        let path = Path::new("document.pdf");
        let result =
            <() as FileStorageUtils>::validate_content_type_for_domain(path, &["application/pdf"]);
        match result {
            Ok(mime) => assert_eq!(mime, "application/pdf"),
            Err(e) => panic!("expected Ok(\"application/pdf\"), got Err({e})"),
        }
    }

    #[test]
    fn validate_content_type_for_domain_rejects_unexpected_type() {
        let path = Path::new("document.pdf");
        let result =
            <() as FileStorageUtils>::validate_content_type_for_domain(path, &["text/plain"]);
        match result {
            Err(FileStorageError::InvalidPath(msg)) => {
                assert!(
                    msg.contains("application/pdf") && msg.contains("text/plain"),
                    "error message should name both the detected and expected types, got: {msg}"
                );
            }
            other => panic!("expected Err(InvalidPath(_)), got {other:?}"),
        }
    }

    #[test]
    fn validate_content_type_for_domain_empty_expected_list_accepts_anything() {
        let path = Path::new("document.pdf");
        let result = <() as FileStorageUtils>::validate_content_type_for_domain(path, &[]);
        assert_eq!(result.unwrap(), "application/pdf".to_string());
    }

    // -- calculate_md5 ----------------------------------------------------

    #[test]
    fn calculate_md5_of_known_bytes_matches_pinned_digest() {
        // Pinned against the well-known MD5("hello") test vector.
        assert_eq!(
            <() as FileStorageUtils>::calculate_md5(b"hello"),
            "5d41402abc4b2a76b9719d911017c592".to_string()
        );
    }

    #[test]
    fn calculate_md5_of_empty_slice_matches_pinned_digest() {
        // Pinned against the well-known MD5("") test vector.
        assert_eq!(
            <() as FileStorageUtils>::calculate_md5(b""),
            "d41d8cd98f00b204e9800998ecf8427e".to_string()
        );
    }

    // -- validate_path ------------------------------------------------

    #[test]
    fn validate_path_accepts_plain_relative_path() {
        let path = Path::new("documents/report.pdf");
        assert!(<() as FileStorageUtils>::validate_path(path).is_ok());
    }

    #[test]
    fn validate_path_rejects_traversal() {
        let path = Path::new("../etc/passwd");
        match <() as FileStorageUtils>::validate_path(path) {
            Err(FileStorageError::InvalidPath(msg)) => {
                assert_eq!(msg, "Path cannot contain '..'");
            }
            other => {
                panic!("expected Err(InvalidPath(\"Path cannot contain '..'\")), got {other:?}")
            }
        }
    }

    #[test]
    fn validate_path_rejects_absolute_path() {
        let path = Path::new("/etc/passwd");
        match <() as FileStorageUtils>::validate_path(path) {
            Err(FileStorageError::InvalidPath(msg)) => {
                assert_eq!(msg, "Path cannot start with '/'");
            }
            other => {
                panic!("expected Err(InvalidPath(\"Path cannot start with '/'\")), got {other:?}")
            }
        }
    }

    #[test]
    fn validate_path_rejects_empty_path() {
        let path = Path::new("");
        match <() as FileStorageUtils>::validate_path(path) {
            Err(FileStorageError::InvalidPath(msg)) => {
                assert_eq!(msg, "Path cannot be empty");
            }
            other => panic!("expected Err(InvalidPath(\"Path cannot be empty\")), got {other:?}"),
        }
    }

    #[test]
    fn validate_path_rejects_path_over_1024_characters() {
        let long_relative_path = "a".repeat(1025);
        let path = Path::new(&long_relative_path);
        match <() as FileStorageUtils>::validate_path(path) {
            Err(FileStorageError::InvalidPath(msg)) => {
                assert_eq!(msg, "Path too long (max 1024 characters)");
            }
            other => panic!(
                "expected Err(InvalidPath(\"Path too long (max 1024 characters)\")), got {other:?}"
            ),
        }
    }

    // -- sanitize_filename ----------------------------------------------

    #[test]
    fn sanitize_filename_leaves_already_safe_name_unchanged() {
        let name = "already-safe_name.v2.txt";
        assert_eq!(<() as FileStorageUtils>::sanitize_filename(name), name);
    }

    #[test]
    fn sanitize_filename_rewrites_reserved_characters_to_underscore() {
        let name = "a<b>c:d\"e|f?g*h\\i/j";
        let expected = "a_b_c_d_e_f_g_h_i_j";
        assert_eq!(<() as FileStorageUtils>::sanitize_filename(name), expected);
    }

    #[test]
    fn sanitize_filename_rewrites_control_characters_to_underscore() {
        let name = "abc\u{1}def";
        let expected = "abc_def";
        assert_eq!(<() as FileStorageUtils>::sanitize_filename(name), expected);
    }

    #[test]
    fn sanitize_filename_trims_leading_and_trailing_whitespace() {
        let name = "  safe_name.txt  ";
        let expected = "safe_name.txt";
        assert_eq!(<() as FileStorageUtils>::sanitize_filename(name), expected);
    }
}
