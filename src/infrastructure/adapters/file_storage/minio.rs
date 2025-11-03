use async_trait::async_trait;
use chrono::{DateTime, Utc};
use s3::Bucket;
use s3::creds::Credentials;
use s3::region::Region;
use s3::BucketConfiguration;
use s3::error::S3Error;
use s3::serde_types::Object;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use uuid::Uuid;

use crate::application::ports::output::file_storage_port::{
    AdvancedFileStoragePort, BatchFileStoragePort, FileItem, FileListResult, FileStorageError,
    FileStoragePort, FileStorageResult, FileStorageUtils, FileVersioningPort, FullFileStoragePort,
    ListOptions, StorageHealth, StorageStats, UploadOptions, DownloadOptions,
};
use crate::application::ports::output::log_port::LogPort;
use crate::core::platform::container::log::{LogEntry, LogLevel, LogMessage};
use crate::core::base::entity::message::{Location, MessagePriority};

/// Configuration for MinIO connection using rust-s3
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinioConfig {
    pub endpoint: String,
    pub access_key: String,
    pub secret_key: String,
    pub bucket: String,
    pub region: Option<String>,
    pub secure: bool,
    pub path_style: bool,
    pub connection_timeout: Duration,
    pub request_timeout: Duration,
    pub max_retries: u32,
    pub max_idle_conns: u32,
}

impl Default for MinioConfig {
    fn default() -> Self {
        Self {
            endpoint: "localhost:9000".to_string(),
            access_key: "minioadmin".to_string(),
            secret_key: "minioadmin".to_string(),
            bucket: "paladin-files".to_string(),
            region: Some("us-east-1".to_string()),
            secure: false,
            path_style: true,
            connection_timeout: Duration::from_secs(30),
            request_timeout: Duration::from_secs(300),
            max_retries: 3,
            max_idle_conns: 10,
        }
    }
}

/// MinIO adapter using rust-s3 crate
pub struct MinioAdapter {
    bucket: Box<Bucket>,
    config: MinioConfig,
    log_port: Option<Arc<dyn LogPort>>,
}

impl MinioAdapter {
    /// Create a new MinIO adapter using rust-s3
    pub async fn new(
        config: MinioConfig,
        log_port: Option<Arc<dyn LogPort>>,
    ) -> FileStorageResult<Self> {
        // Create credentials
        let credentials = Credentials::new(
            Some(&config.access_key),
            Some(&config.secret_key),
            None,
            None,
            None,
        ).map_err(|e| FileStorageError::AuthenticationError(format!("Invalid credentials: {}", e)))?;

        // Create custom region for MinIO
        let region = if config.secure {
            Region::Custom {
                region: config.region.clone().unwrap_or_else(|| "us-east-1".to_string()),
                endpoint: format!("https://{}", config.endpoint),
            }
        } else {
            Region::Custom {
                region: config.region.clone().unwrap_or_else(|| "us-east-1".to_string()),
                endpoint: format!("http://{}", config.endpoint),
            }
        };

        // Create bucket instance
        let bucket = Bucket::new(&config.bucket, region, credentials)
            .map_err(|e| FileStorageError::ConfigurationError(format!("Failed to create bucket: {}", e)))?
            .with_path_style();

        let adapter = Self {
            bucket,
            config,
            log_port,
        };

        // Ensure bucket exists
        adapter.ensure_bucket_exists().await?;

        adapter.log_operation(LogLevel::Info, "MinIO adapter initialized successfully".to_string()).await;

        Ok(adapter)
    }

    /// Ensure the bucket exists, create if it doesn't
    async fn ensure_bucket_exists(&self) -> FileStorageResult<()> {
        // Check if bucket exists by attempting to list objects
        match timeout(
            self.config.connection_timeout,
            self.bucket.list("".to_string(), Some("/".to_string()))
        ).await {
            Ok(Ok(_)) => {
                // Bucket exists and is accessible
                Ok(())
            }
            Ok(Err(S3Error::HttpFailWithBody(404, _))) | 
            Ok(Err(S3Error::HttpFail)) => {
                // Bucket doesn't exist, try to create it
                self.create_bucket().await
            }
            Ok(Err(e)) => {
                Err(FileStorageError::ConnectionError(format!("Failed to check bucket: {}", e)))
            }
            Err(_) => {
                Err(FileStorageError::Timeout)
            }
        }
    }

    async fn create_bucket(&self) -> FileStorageResult<()> {
        let config = BucketConfiguration::default();
        
        // Fix: Use the static method Bucket::create instead of instance method
        match timeout(
            self.config.connection_timeout,
            Bucket::create(
                &self.config.bucket,
                self.bucket.region(),
                self.bucket.credentials().await.map_err(|e| {
                    FileStorageError::AuthenticationError(format!("Failed to get credentials: {}", e))
                })?,
                config
            )
        ).await {
            Ok(Ok(_)) => {
                self.log_operation(LogLevel::Info, format!("Created bucket: {}", self.config.bucket)).await;
                Ok(())
            }
            Ok(Err(e)) => {
                // Check if error is because bucket already exists
                if let S3Error::HttpFailWithBody(409, _) = e {
                    // Bucket already exists, which is fine
                    Ok(())
                } else {
                    Err(FileStorageError::ConnectionError(format!("Failed to create bucket: {}", e)))
                }
            }
            Err(_) => Err(FileStorageError::Timeout),
        }
    }
    
    /// Log operation to LogPort if available
    async fn log_operation(&self, level: LogLevel, message: String) {
        if let Some(log_port) = &self.log_port {
            let entry = LogEntry {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                message: LogMessage::new(level.clone(), message.clone()),
                source: Location::service("minio-adapter"),
                destination: Location::system("minio-adapter"),
                correlation_id: None,
                priority: MessagePriority::Normal,
            };

            if let Err(e) = log_port.write_entry(entry).await {
                eprintln!("Failed to log operation: {} - Error: {}", message, e);
            }
        }
    }

    /// Convert PathBuf to string, ensuring proper format
    fn path_to_object_name(&self, path: &PathBuf) -> FileStorageResult<String> {
        <() as FileStorageUtils>::validate_path(path)?;
        
        let path_str = path.to_string_lossy();
        // Remove leading slash if present
        let cleaned_path = path_str.strip_prefix('/').unwrap_or(&path_str);
        Ok(cleaned_path.to_string())
    }

    /// Convert S3 object info to FileItem
fn s3_object_to_file_item(&self, object: &Object, path: PathBuf) -> FileItem {
        let mut metadata = HashMap::new();
        
        // Add S3-specific metadata
        if let Some(etag) = &object.e_tag {
            metadata.insert("etag".to_string(), etag.clone());
        }
        
        if let Some(storage_class) = &object.storage_class {
            metadata.insert("storage_class".to_string(), storage_class.clone());
        }

        let size = object.size;
        let mut file_item = FileItem::new(path, size);
        
        // Fix: Parse the last_modified string to DateTime<Utc>
        if let Ok(parsed_date) = DateTime::parse_from_rfc3339(&object.last_modified) {
            file_item.modified_at = parsed_date.with_timezone(&Utc);
        } else if let Ok(parsed_date) = DateTime::parse_from_rfc2822(&object.last_modified) {
            file_item.modified_at = parsed_date.with_timezone(&Utc);
        } else {
            // Fall back to current time if parsing fails
            file_item.modified_at = Utc::now();
        }
        
        file_item.metadata = metadata;
        
        if let Some(etag) = &object.e_tag {
            file_item.md5_hash = Some(etag.trim_matches('"').to_string());
        }

        // Detect content type
        if let Some(content_type) = <() as FileStorageUtils>::detect_content_type(&file_item.path) {
            file_item.content_type = Some(content_type);
        }

        file_item
    }
    
    /// Apply upload options by creating headers map
    fn create_upload_headers(&self, options: &UploadOptions) -> HashMap<String, String> {
        let mut headers = HashMap::new();

        if let Some(content_type) = &options.content_type {
            headers.insert("Content-Type".to_string(), content_type.clone());
        }

        if let Some(cache_control) = &options.cache_control {
            headers.insert("Cache-Control".to_string(), cache_control.clone());
        }

        if let Some(content_disposition) = &options.content_disposition {
            headers.insert("Content-Disposition".to_string(), content_disposition.clone());
        }

        // Add user metadata with x-amz-meta- prefix
        for (key, value) in &options.metadata {
            headers.insert(format!("x-amz-meta-{}", key), value.clone());
        }

        // Add tags as metadata (S3 doesn't have separate tagging in basic operations)
        if !options.tags.is_empty() {
            let tags_value = options.tags.join(",");
            headers.insert("x-amz-meta-tags".to_string(), tags_value);
        }

        headers
    }

    /// Execute operation with timeout and retries
    async fn execute_with_retry<F, T, Fut>(&self, operation: F) -> FileStorageResult<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, S3Error>>,
    {
        let mut last_error = None;
        
        for attempt in 0..=self.config.max_retries {
            match timeout(self.config.request_timeout, operation()).await {
                Ok(Ok(result)) => return Ok(result),
                Ok(Err(e)) => {
                    last_error = Some(e);
                    if attempt < self.config.max_retries {
                        let delay = Duration::from_millis(100 * (attempt + 1) as u64);
                        tokio::time::sleep(delay).await;
                    }
                }
                Err(_) => {
                    return Err(FileStorageError::Timeout);
                }
            }
        }

        Err(FileStorageError::IoError(
            format!("Operation failed after {} retries: {}", 
                   self.config.max_retries, 
                   last_error.map(|e| e.to_string()).unwrap_or_else(|| "Unknown error".to_string()))
        ))
    }
}

#[async_trait]
impl FileStoragePort for MinioAdapter {
    async fn upload_file(
        &self,
        path: &PathBuf,
        content: &[u8],
        options: Option<UploadOptions>,
    ) -> FileStorageResult<FileItem> {
        let object_name = self.path_to_object_name(path)?;
        let options = options.unwrap_or_default();

        // Check if file exists and overwrite is disabled
        if !options.overwrite && self.file_exists(path).await? {
            return Err(FileStorageError::InvalidPath(
                format!("File already exists: {}", path.display())
            ));
        }

        // Create headers
        let headers = self.create_upload_headers(&options);
        
        // Auto-detect content type if not provided
        let content_type = headers.get("Content-Type")
            .cloned()
            .or_else(|| <() as FileStorageUtils>::detect_content_type(path))
            .unwrap_or_else(|| "application/octet-stream".to_string());

        // Upload the file
        self.execute_with_retry(|| {
            self.bucket.put_object_with_content_type(&object_name, content, &content_type)
        }).await.map_err(|e| FileStorageError::IoError(format!("Failed to upload file: {}", e)))?;

        // Get file info after upload
        let file_item = self.get_file_info(path).await?;

        self.log_operation(
            LogLevel::Info,
            format!("Uploaded file: {} ({} bytes)", path.display(), content.len()),
        ).await;

        Ok(file_item)
    }

    async fn download_file(
        &self,
        path: &PathBuf,
        _options: Option<DownloadOptions>,
    ) -> FileStorageResult<Vec<u8>> {
        let object_name = self.path_to_object_name(path)?;

        let response = self.execute_with_retry(|| {
            self.bucket.get_object(&object_name)
        }).await.map_err(|e| FileStorageError::FileNotFound(format!("Failed to download file: {}", e)))?;

        let content = response.bytes().to_vec();

        self.log_operation(
            LogLevel::Info,
            format!("Downloaded file: {} ({} bytes)", path.display(), content.len()),
        ).await;

        Ok(content)
    }

    async fn delete_file(&self, path: &PathBuf) -> FileStorageResult<()> {
        let object_name = self.path_to_object_name(path)?;

        self.execute_with_retry(|| {
            self.bucket.delete_object(&object_name)
        }).await.map_err(|e| FileStorageError::IoError(format!("Failed to delete file: {}", e)))?;

        self.log_operation(LogLevel::Info, format!("Deleted file: {}", path.display())).await;

        Ok(())
    }

    async fn file_exists(&self, path: &PathBuf) -> FileStorageResult<bool> {
        let object_name = self.path_to_object_name(path)?;

        match timeout(self.config.connection_timeout, self.bucket.head_object(&object_name)).await {
            Ok(Ok(_)) => Ok(true),
            Ok(Err(S3Error::HttpFail)) | Ok(Err(S3Error::HttpFailWithBody(404, _))) => Ok(false),
            Ok(Err(e)) => Err(FileStorageError::IoError(format!("Failed to check file existence: {}", e))),
            Err(_) => Err(FileStorageError::Timeout),
        }
    }

    async fn get_file_info(&self, path: &PathBuf) -> FileStorageResult<FileItem> {
        let object_name = self.path_to_object_name(path)?;

        let (head_result, _) = self.execute_with_retry(|| {
            self.bucket.head_object(&object_name)
        }).await.map_err(|e| FileStorageError::FileNotFound(format!("File not found: {}", e)))?;

        // Parse the response to create FileItem
        let mut metadata = HashMap::new();

        // Extract user-defined metadata if available
        if let Some(user_metadata) = &head_result.metadata {
            for (key, value) in user_metadata {
                metadata.insert(key.clone(), value.clone());
            }
        }

        let size = head_result.content_length.unwrap_or(0) as u64;

        let mut file_item = FileItem::new(path.clone(), size);
        file_item.metadata = metadata;
        file_item.content_type = head_result.content_type.clone();

        if let Some(etag) = &head_result.e_tag {
            file_item.md5_hash = Some(etag.trim_matches('"').to_string());
        }

        if let Some(last_modified) = &head_result.last_modified {
            if let Ok(dt) = DateTime::parse_from_rfc2822(last_modified) {
                file_item.modified_at = dt.with_timezone(&Utc);
            }
        }

        Ok(file_item)
    }

    async fn list_files(&self, options: Option<ListOptions>) -> FileStorageResult<FileListResult> {
        let options = options.unwrap_or_default();

        let prefix = options.prefix.clone().unwrap_or_default();
        let max_keys = options.limit.map(|l| l.to_string());

        let results = self.execute_with_retry(|| {
            self.bucket.list(prefix.clone(), max_keys.clone())
        }).await.map_err(|e| FileStorageError::IoError(format!("Failed to list files: {}", e)))?;

        let mut files = Vec::new();
        
        // Process all list results
        for list_result in results {
            for object in list_result.contents {
                let path = PathBuf::from(&object.key);
                
                // Apply filters
                if self.should_include_file(&object, &path, &options) {
                    let file_item = self.s3_object_to_file_item(&object, path);
                    files.push(file_item);
                }
            }
        }

        // Sort files by modification date (newest first)
        files.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));

        // For simplicity, we'll assume no more results for now
        let has_more = false;
        let continuation_token = None;

        Ok(FileListResult {
            files,
            continuation_token,
            has_more,
            total_count: None,
        })
    }

    async fn copy_file(
        &self,
        source_path: &PathBuf,
        destination_path: &PathBuf,
    ) -> FileStorageResult<FileItem> {
        let source_object = self.path_to_object_name(source_path)?;
        let dest_object = self.path_to_object_name(destination_path)?;

        // S3 copy operation
        let copy_source = format!("{}/{}", self.config.bucket, source_object);
        
        self.execute_with_retry(|| {
            self.bucket.copy_object_internal(&copy_source, &dest_object)
        }).await.map_err(|e| FileStorageError::IoError(format!("Failed to copy file: {}", e)))?;

        let file_item = self.get_file_info(destination_path).await?;

        self.log_operation(
            LogLevel::Info,
            format!("Copied file: {} -> {}", source_path.display(), destination_path.display()),
        ).await;

        Ok(file_item)
    }

    async fn move_file(
        &self,
        source_path: &PathBuf,
        destination_path: &PathBuf,
    ) -> FileStorageResult<FileItem> {
        // Copy file to new location
        let file_item = self.copy_file(source_path, destination_path).await?;

        // Delete original file
        self.delete_file(source_path).await?;

        self.log_operation(
            LogLevel::Info,
            format!("Moved file: {} -> {}", source_path.display(), destination_path.display()),
        ).await;

        Ok(file_item)
    }

    async fn get_storage_stats(&self) -> FileStorageResult<StorageStats> {
        let list_options = ListOptions::default();
        let file_list = self.list_files(Some(list_options)).await?;

        let mut total_files = 0u64;
        let mut total_size = 0u64;
        let mut files_by_type = HashMap::new();
        let mut size_by_type = HashMap::new();

        for file in file_list.files {
            total_files += 1;
            total_size += file.size;

            let file_type = file.extension()
                .unwrap_or("unknown")
                .to_lowercase();

            *files_by_type.entry(file_type.clone()).or_insert(0) += 1;
            *size_by_type.entry(file_type).or_insert(0) += file.size;
        }

        Ok(StorageStats {
            total_files,
            total_size,
            files_by_type,
            size_by_type,
            last_updated: Utc::now(),
        })
    }

    async fn health_check(&self) -> FileStorageResult<StorageHealth> {
        let start_time = std::time::Instant::now();

        // Try to list objects as a simple health check
        match timeout(
            self.config.connection_timeout,
            self.bucket.list("".to_string(), Some("1".to_string()))
        ).await {
            Ok(Ok(_)) => {
                let response_time = start_time.elapsed().as_millis() as u64;
                
                self.log_operation(LogLevel::Info, "MinIO health check passed".to_string()).await;

                Ok(StorageHealth {
                    is_available: true,
                    response_time_ms: Some(response_time),
                    error: None,
                    checked_at: Utc::now(),
                })
            }
            Ok(Err(e)) => {
                let error_msg = format!("MinIO health check failed: {}", e);
                self.log_operation(LogLevel::Error, error_msg.clone()).await;

                Ok(StorageHealth {
                    is_available: false,
                    response_time_ms: None,
                    error: Some(error_msg),
                    checked_at: Utc::now(),
                })
            }
            Err(_) => {
                let error_msg = "MinIO health check timed out".to_string();
                self.log_operation(LogLevel::Error, error_msg.clone()).await;

                Ok(StorageHealth {
                    is_available: false,
                    response_time_ms: None,
                    error: Some(error_msg),
                    checked_at: Utc::now(),
                })
            }
        }
    }
}

impl MinioAdapter {
    fn should_include_file(&self, object: &Object, path: &PathBuf, options: &ListOptions) -> bool {
        // Filter by extensions
        if !options.extensions.is_empty() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if !options.extensions.contains(&ext.to_lowercase()) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Filter by size
        let size = object.size;  // Remove try_into() since object.size is already u64
        if let Some(min_size) = options.min_size {
            if size < min_size {
                return false;
            }
        }
        if let Some(max_size) = options.max_size {
            if size > max_size {
                return false;
            }
        }

        // Filter by modification date
        // Fix: object.last_modified is a String, not Option<String>
        if !object.last_modified.is_empty() {
            // Parse the last_modified string to DateTime<Utc>
            let parsed_date = if let Ok(dt) = DateTime::parse_from_rfc3339(&object.last_modified) {
                dt.with_timezone(&Utc)
            } else if let Ok(dt) = DateTime::parse_from_rfc2822(&object.last_modified) {
                dt.with_timezone(&Utc)
            } else {
                // If we can't parse the date, skip date filtering for this object
                return true;
            };

            if let Some(modified_after) = options.modified_after {
                if parsed_date < modified_after {
                    return false;
                }
            }
            if let Some(modified_before) = options.modified_before {
                if parsed_date > modified_before {
                    return false;
                }
            }
        }

        // Note: Tag filtering would require additional S3 API calls for each object
        // For performance, we skip tag filtering in the basic list operation
        if !options.tags.is_empty() {
            // This would require a separate API call to get object tags
            // For now, we'll return true and handle tag filtering in a more sophisticated way if needed
        }

        true
    }
}

// Implement the remaining traits with similar patterns
#[async_trait]
impl BatchFileStoragePort for MinioAdapter {
    async fn upload_files(
        &self,
        files: Vec<(PathBuf, Vec<u8>, Option<UploadOptions>)>,
    ) -> FileStorageResult<Vec<FileItem>> {
        let mut results = Vec::new();

        // Note: rust-s3 doesn't have native batch upload, so we do them concurrently
        let upload_tasks: Vec<_> = files.into_iter().map(|(path, content, options)| {
            async move {
                self.upload_file(&path, &content, options).await
            }
        }).collect();

        // Execute uploads concurrently
        for task in upload_tasks {
            match task.await {
                Ok(file_item) => results.push(file_item),
                Err(e) => {
                    self.log_operation(
                        LogLevel::Error,
                        format!("Failed to upload file in batch: {}", e),
                    ).await;
                    return Err(e);
                }
            }
        }

        self.log_operation(
            LogLevel::Info,
            format!("Batch uploaded {} files", results.len()),
        ).await;

        Ok(results)
    }

    async fn download_files(
        &self,
        paths: Vec<PathBuf>,
        options: Option<DownloadOptions>,
    ) -> FileStorageResult<Vec<(PathBuf, Vec<u8>)>> {
        let mut results = Vec::new();

        for path in paths {
            match self.download_file(&path, options.clone()).await {
                Ok(content) => results.push((path, content)),
                Err(e) => {
                    self.log_operation(
                        LogLevel::Error,
                        format!("Failed to download file in batch: {} - {}", path.display(), e),
                    ).await;
                    return Err(e);
                }
            }
        }

        self.log_operation(
            LogLevel::Info,
            format!("Batch downloaded {} files", results.len()),
        ).await;

        Ok(results)
    }

    async fn delete_files(&self, paths: Vec<PathBuf>) -> FileStorageResult<Vec<PathBuf>> {
        let mut deleted = Vec::new();

        for path in paths {
            match self.delete_file(&path).await {
                Ok(()) => deleted.push(path),
                Err(e) => {
                    self.log_operation(
                        LogLevel::Error,
                        format!("Failed to delete file in batch: {} - {}", path.display(), e),
                    ).await;
                    return Err(e);
                }
            }
        }

        self.log_operation(
            LogLevel::Info,
            format!("Batch deleted {} files", deleted.len()),
        ).await;

        Ok(deleted)
    }

    async fn get_files_info(&self, paths: Vec<PathBuf>) -> FileStorageResult<Vec<FileItem>> {
        let mut results = Vec::new();

        for path in paths {
            match self.get_file_info(&path).await {
                Ok(file_item) => results.push(file_item),
                Err(e) => {
                    self.log_operation(
                        LogLevel::Error,
                        format!("Failed to get file info in batch: {} - {}", path.display(), e),
                    ).await;
                    return Err(e);
                }
            }
        }

        Ok(results)
    }
}

#[async_trait]
impl AdvancedFileStoragePort for MinioAdapter {
    async fn generate_upload_url(
        &self,
        path: &PathBuf,
        expires_in: Duration,
        _options: Option<UploadOptions>,
    ) -> FileStorageResult<String> {
        let object_name = self.path_to_object_name(path)?;
        let expires_in_secs = expires_in.as_secs() as u32;
        
        // Fix: Add the missing 4th parameter (custom query parameters)
        let url = self.bucket.presign_put(&object_name, expires_in_secs, None, None)
            .await
            .map_err(|e| FileStorageError::IoError(format!("Failed to generate upload URL: {}", e)))?;
        
        Ok(url)
    }

    async fn generate_download_url(
        &self,
        path: &PathBuf,
        expires_in: Duration,
        _options: Option<DownloadOptions>,
    ) -> FileStorageResult<String> {
        let object_name = self.path_to_object_name(path)?;
        let expires_in_secs = expires_in.as_secs() as u32;
        
        // Fix: Add the missing 4th parameter (custom query parameters)
        let url = self.bucket.presign_get(&object_name, expires_in_secs, None)
            .await
            .map_err(|e| FileStorageError::IoError(format!("Failed to generate download URL: {}", e)))?;
        
        Ok(url)
    }
    
    async fn create_multipart_upload(
        &self,
        path: &PathBuf,
        _options: Option<UploadOptions>,
    ) -> FileStorageResult<String> {
        let object_name = self.path_to_object_name(path)?;
        
        let response = self.bucket.initiate_multipart_upload(&object_name, "application/octet-stream")
            .await
            .map_err(|e| FileStorageError::IoError(format!("Failed to initiate multipart upload: {}", e)))?;
        
        Ok(response.upload_id)
    }

    async fn upload_part(
        &self,
        _upload_id: &str,
        _part_number: u32,
        _content: &[u8],
    ) -> FileStorageResult<String> {
        // Note: This would require implementing multipart upload with rust-s3
        // The current version might not have direct support, so this is a placeholder
        Err(FileStorageError::Unknown(
            "Multipart upload not fully implemented with rust-s3".to_string()
        ))
    }

    async fn complete_multipart_upload(
        &self,
        _upload_id: &str,
        _parts: Vec<(u32, String)>,
    ) -> FileStorageResult<FileItem> {
        Err(FileStorageError::Unknown(
            "Multipart upload not fully implemented with rust-s3".to_string()
        ))
    }

    async fn abort_multipart_upload(&self, _upload_id: &str) -> FileStorageResult<()> {
        Err(FileStorageError::Unknown(
            "Multipart upload not fully implemented with rust-s3".to_string()
        ))
    }
}

#[async_trait]
impl FileVersioningPort for MinioAdapter {
    async fn upload_file_version(
        &self,
        path: &PathBuf,
        content: &[u8],
        options: Option<UploadOptions>,
    ) -> FileStorageResult<FileItem> {
        // For basic versioning, we can append a timestamp to the filename
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let mut versioned_path = path.clone();
        
        if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
            if let Some(extension) = path.extension().and_then(|s| s.to_str()) {
                let new_filename = format!("{}_{}.{}", filename, timestamp, extension);
                versioned_path.set_file_name(new_filename);
            } else {
                let new_filename = format!("{}_{}", filename, timestamp);
                versioned_path.set_file_name(new_filename);
            }
        }

        self.upload_file(&versioned_path, content, options).await
    }

    async fn list_file_versions(&self, path: &PathBuf) -> FileStorageResult<Vec<FileItem>> {
        let filename_stem = path.file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| FileStorageError::InvalidPath("Invalid filename".to_string()))?;

        let prefix = path.parent()
            .map(|p| format!("{}/{}", p.display(), filename_stem))
            .unwrap_or_else(|| filename_stem.to_string());

        let list_options = ListOptions {
            prefix: Some(prefix),
            ..Default::default()
        };

        let file_list = self.list_files(Some(list_options)).await?;
        Ok(file_list.files)
    }

    async fn download_file_version(
        &self,
        path: &PathBuf,
        _version_id: &str,
        options: Option<DownloadOptions>,
    ) -> FileStorageResult<Vec<u8>> {
        // For simple versioning, treat version_id as the versioned filename
        self.download_file(path, options).await
    }

    async fn delete_file_version(&self, path: &PathBuf, _version_id: &str) -> FileStorageResult<()> {
        // For simple versioning, treat version_id as the versioned filename
        self.delete_file(path).await
    }

    async fn get_file_version_info(
        &self,
        path: &PathBuf,
        _version_id: &str,
    ) -> FileStorageResult<FileItem> {
        // For simple versioning, treat version_id as the versioned filename
        self.get_file_info(path).await
    }
}

// Implement FullFileStoragePort
impl FullFileStoragePort for MinioAdapter {}

impl MinioAdapter {
    /// Shutdown the adapter and close connections
    pub async fn shutdown(&self) -> FileStorageResult<()> {
        self.log_operation(LogLevel::Info, "Shutting down MinIO adapter".to_string()).await;
        // rust-s3 handles connection cleanup automatically
        Ok(())
    }

    /// Get MinIO connection info for debugging
    pub fn get_connection_info(&self) -> String {
        format!("{}://{}/{}", 
                if self.config.secure { "https" } else { "http" },
                self.config.endpoint, 
                self.config.bucket)
    }
}