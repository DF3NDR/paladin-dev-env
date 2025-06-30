/*
File Storage Port

A port that defines how the application interacts with file storage systems.
This port provides an abstraction layer that allows the application to store and retrieve files
without being tightly coupled to the specific file storage implementation.

Typical implementations of this port would include local file systems, cloud storage services
like AWS S3, MinIO, Google Cloud Storage, or Azure Blob Storage.

The port handles common file operations like upload, download, delete, and listing files,
along with metadata management and error handling.
*/
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;
use md5::compute;
use mime_guess::from_path;

/// Result type for file storage operations
pub type FileStorageResult<T> = Result<T, FileStorageError>;

/// Errors that can occur during file storage operations
#[derive(Debug, Clone, Error)]
pub enum FileStorageError {
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Storage quota exceeded")]
    QuotaExceeded,
    
    #[error("File too large: {size} bytes (max: {max_size} bytes)")]
    FileTooLarge { size: u64, max_size: u64 },
    
    #[error("Invalid file path: {0}")]
    InvalidPath(String),
    
    #[error("Bucket not found: {0}")]
    BucketNotFound(String),
    
    #[error("Connection error: {0}")]
    ConnectionError(String),
    
    #[error("Authentication failed: {0}")]
    AuthenticationError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("IO error: {0}")]
    IoError(String),
    
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    
    #[error("Operation timeout")]
    Timeout,
    
    #[error("Service unavailable")]
    ServiceUnavailable,
    
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
    ServerSideEncryptionCustomerKey { key: String },
    /// Server-side encryption with KMS
    ServerSideEncryptionKms { key_id: String },
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
#[async_trait]
pub trait FileStoragePort: Send + Sync {
    /// Upload a file to storage
    async fn upload_file(
        &self,
        path: &PathBuf,
        content: &[u8],
        options: Option<UploadOptions>,
    ) -> FileStorageResult<FileItem>;

    /// Download a file from storage
    async fn download_file(
        &self,
        path: &PathBuf,
        options: Option<DownloadOptions>,
    ) -> FileStorageResult<Vec<u8>>;

    /// Delete a file from storage
    async fn delete_file(&self, path: &PathBuf) -> FileStorageResult<()>;

    /// Check if a file exists
    async fn file_exists(&self, path: &PathBuf) -> FileStorageResult<bool>;

    /// Get file metadata without downloading content
    async fn get_file_info(&self, path: &PathBuf) -> FileStorageResult<FileItem>;

    /// List files in storage
    async fn list_files(&self, options: Option<ListOptions>) -> FileStorageResult<FileListResult>;

    /// Copy a file within storage
    async fn copy_file(
        &self,
        source_path: &PathBuf,
        destination_path: &PathBuf,
    ) -> FileStorageResult<FileItem>;

    /// Move/rename a file within storage
    async fn move_file(
        &self,
        source_path: &PathBuf,
        destination_path: &PathBuf,
    ) -> FileStorageResult<FileItem>;

    /// Get storage statistics
    async fn get_storage_stats(&self) -> FileStorageResult<StorageStats>;

    /// Health check for storage service
    async fn health_check(&self) -> FileStorageResult<StorageHealth>;
}

/// Batch file operations port
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
#[async_trait]
pub trait AdvancedFileStoragePort: Send + Sync {
    /// Generate a pre-signed URL for file upload
    async fn generate_upload_url(
        &self,
        path: &PathBuf,
        expires_in: std::time::Duration,
        options: Option<UploadOptions>,
    ) -> FileStorageResult<String>;

    /// Generate a pre-signed URL for file download
    async fn generate_download_url(
        &self,
        path: &PathBuf,
        expires_in: std::time::Duration,
        options: Option<DownloadOptions>,
    ) -> FileStorageResult<String>;

    /// Create a multipart upload session
    async fn create_multipart_upload(
        &self,
        path: &PathBuf,
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
#[async_trait]
pub trait FileVersioningPort: Send + Sync {
    /// Upload a new version of a file
    async fn upload_file_version(
        &self,
        path: &PathBuf,
        content: &[u8],
        options: Option<UploadOptions>,
    ) -> FileStorageResult<FileItem>;

    /// List all versions of a file
    async fn list_file_versions(&self, path: &PathBuf) -> FileStorageResult<Vec<FileItem>>;

    /// Download a specific version of a file
    async fn download_file_version(
        &self,
        path: &PathBuf,
        version_id: &str,
        options: Option<DownloadOptions>,
    ) -> FileStorageResult<Vec<u8>>;

    /// Delete a specific version of a file
    async fn delete_file_version(&self, path: &PathBuf, version_id: &str) -> FileStorageResult<()>;

    /// Get info for a specific version
    async fn get_file_version_info(
        &self,
        path: &PathBuf,
        version_id: &str,
    ) -> FileStorageResult<FileItem>;
}

/// Combined file storage port with all capabilities
pub trait FullFileStoragePort:
    FileStoragePort + BatchFileStoragePort + AdvancedFileStoragePort + FileVersioningPort + Send + Sync
{
}

/// Helper trait for common file operations
pub trait FileStorageUtils {
    /// Detect content type from file extension
    fn detect_content_type(path: &PathBuf) -> Option<String>;
    
    fn detect_content_type_with_fallback(path: &PathBuf, fallback: &str) -> String;
    
    fn validate_content_type_for_domain(path: &PathBuf, expected_types: &[&str]) -> FileStorageResult<String>;

    /// Generate MD5 hash of content
    fn calculate_md5(content: &[u8]) -> String;

    /// Validate file path
    fn validate_path(path: &PathBuf) -> FileStorageResult<()>;

    /// Sanitize filename
    fn sanitize_filename(filename: &str) -> String;
}

impl FileStorageUtils for () {
    fn detect_content_type(path: &PathBuf) -> Option<String> {
        Some(from_path(path).first_or_text_plain().to_string())
    }
    
    fn detect_content_type_with_fallback(path: &PathBuf, fallback: &str) -> String {
        from_path(path)
            .first()
            .map(|mime| mime.to_string())
            .unwrap_or_else(|| fallback.to_string())
    }
    
    fn validate_content_type_for_domain(path: &PathBuf, expected_types: &[&str]) -> FileStorageResult<String> {
        let detected = Self::detect_content_type(path)
            .unwrap_or_else(|| "application/octet-stream".to_string());
        
        if expected_types.is_empty() || expected_types.contains(&detected.as_str()) {
            Ok(detected)
        } else {
            Err(FileStorageError::InvalidPath(
                format!("File type '{}' not allowed. Expected one of: {:?}", detected, expected_types)
            ))
        }
    }

    fn calculate_md5(content: &[u8]) -> String {
        let hasher = compute(content);
        format!("{:x}", hasher)
    }

    fn validate_path(path: &PathBuf) -> FileStorageResult<()> {
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
            return Err(FileStorageError::InvalidPath("Path cannot be empty".to_string()));
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