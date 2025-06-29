use async_trait::async_trait;
use chrono::Utc;
use minio::s3::args::{
    BucketExistsArgs, GetObjectArgs, ListObjectsArgs, MakeBucketArgs, PutObjectArgs,
    RemoveObjectArgs, StatObjectArgs, CopyObjectArgs,
};
use minio::s3::client::{Client, ClientBuilder};
use minio::s3::creds::StaticProvider;
use minio::s3::http::BaseUrl;
use minio::s3::response::{ListObjectsResponse, StatObjectResponse};
use minio::s3::types::{S3Object};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use uuid::Uuid;

use crate::application::ports::output::file_storage_port::{
    AdvancedFileStoragePort, BatchFileStoragePort, FileItem, FileListResult, FileStorageError,
    FileStoragePort, FileStorageResult, FileStorageUtils, FileVersioningPort, FullFileStoragePort,
    ListOptions, StorageHealth, StorageStats, UploadOptions, DownloadOptions,
};
use crate::application::ports::output::log_port::LogPort;
use crate::core::platform::container::log::{LogEntry, LogLevel, LogDestination};
use crate::core::base::entity::message::Location;

/// Configuration for MinIO connection
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
    pub max_idle_conns: u32,
}

impl Default for MinioConfig {
    fn default() -> Self {
        Self {
            endpoint: "localhost:9000".to_string(),
            access_key: "minioadmin".to_string(),
            secret_key: "minioadmin".to_string(),
            bucket: "in4me-files".to_string(),
            region: None,
            secure: false,
            path_style: true,
            connection_timeout: Duration::from_secs(30),
            request_timeout: Duration::from_secs(300),
            max_idle_conns: 10,
        }
    }
}

/// MinIO adapter for file storage
pub struct MinioAdapter {
    client: Client,
    bucket: String,
    config: MinioConfig,
    log_port: Option<Arc<dyn LogPort>>,
}

impl MinioAdapter {
    /// Create a new MinIO adapter
    pub async fn new(
        config: MinioConfig,
        log_port: Option<Arc<dyn LogPort>>,
    ) -> FileStorageResult<Self> {
        // Parse the endpoint URL
        let base_url = if config.secure {
            format!("https://{}", config.endpoint)
        } else {
            format!("http://{}", config.endpoint)
        };

        let base_url = BaseUrl::from_str(&base_url)
            .map_err(|e| FileStorageError::ConfigurationError(format!("Invalid endpoint URL: {}", e)))?;

        // Create credentials provider
        let creds_provider = StaticProvider::new(&config.access_key, &config.secret_key, None);

        // Build the client
        let client_builder = ClientBuilder::new(base_url)
            .provider(Some(Box::new(creds_provider)));

        // The minio::s3::ClientBuilder does not support setting region directly.
        // If region support is needed, it should be handled via configuration or credentials, not the builder.
        // if let Some(region) = &config.region {
        //     client_builder = client_builder.region(region);
        // }

        let client = client_builder
            .build()
            .map_err(|e| FileStorageError::ConfigurationError(format!("Failed to create MinIO client: {}", e)))?;

        let adapter = Self {
            client,
            bucket: config.bucket.clone(),
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
        let exists_args = BucketExistsArgs::new(&self.bucket)
            .map_err(|e| FileStorageError::ConfigurationError(format!("Invalid bucket name: {}", e)))?;

        let bucket_exists = self
            .client
            .bucket_exists(&exists_args)
            .map_err(|e| FileStorageError::ConnectionError(format!("Failed to check bucket existence: {}", e)))?;

        if !bucket_exists {
            let make_bucket_args = MakeBucketArgs::new(&self.bucket)
                .map_err(|e| FileStorageError::ConfigurationError(format!("Invalid bucket name: {}", e)))?;

            self.client
                .make_bucket(&make_bucket_args)
                .await
                .map_err(|e| FileStorageError::ConnectionError(format!("Failed to create bucket: {}", e)))?;

            self.log_operation(LogLevel::Info, format!("Created bucket: {}", self.bucket)).await;
        }

        Ok(())
    }

    /// Log operation to LogPort if available
    async fn log_operation(&self, level: LogLevel, message: String) {
        if let Some(log_port) = &self.log_port {
            let entry = LogEntry {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                level,
                message: message.clone(),
                source: Some(Location::service("minio-adapter")),
                destination: LogDestination::System,
                metadata: HashMap::new(),
                correlation_id: None,
                session_id: None,
                trace_id: None,
            };

            if let Err(e) = log_port.write_entry(entry).await {
                eprintln!("Failed to log operation: {} - Error: {}", message, e);
            }
        }
    }

    /// Convert PathBuf to string, ensuring proper format
    fn path_to_object_name(&self, path: &PathBuf) -> FileStorageResult<String> {
        FileStorageUtils::validate_path(path)?;
        
        let path_str = path.to_string_lossy();
        // Remove leading slash if present
        let cleaned_path = path_str.strip_prefix('/').unwrap_or(&path_str);
        Ok(cleaned_path.to_string())
    }

    /// Convert MinIO object info to FileItem
    fn object_to_file_item(&self, object: &Object, path: PathBuf) -> FileItem {
        let mut metadata = HashMap::new();
        
        // Add MinIO-specific metadata
        if let Some(etag) = &object.etag {
            metadata.insert("etag".to_string(), etag.clone());
        }
        
        if let Some(storage_class) = &object.storage_class {
            metadata.insert("storage_class".to_string(), storage_class.clone());
        }

        // Extract user metadata (prefixed with "x-amz-meta-")
        if let Some(user_metadata) = &object.user_metadata {
            for (key, value) in user_metadata {
                if let Some(user_key) = key.strip_prefix("x-amz-meta-") {
                    metadata.insert(user_key.to_string(), value.clone());
                }
            }
        }

        let mut file_item = FileItem::new(path, object.size.unwrap_or(0));
        file_item.modified_at = object.last_modified.unwrap_or_else(Utc::now);
        file_item.metadata = metadata;
        
        if let Some(etag) = &object.etag {
            file_item.md5_hash = Some(etag.trim_matches('"').to_string());
        }

        // Detect content type
        if let Some(content_type) = FileStorageUtils::detect_content_type(&file_item.path) {
            file_item.content_type = Some(content_type);
        }

        file_item
    }

    /// Convert StatObjectResponse to FileItem
    fn stat_response_to_file_item(&self, response: &StatObjectResponse, path: PathBuf) -> FileItem {
        let mut metadata = HashMap::new();
        
        if let Some(etag) = &response.etag {
            metadata.insert("etag".to_string(), etag.clone());
        }

        if let Some(version_id) = &response.version_id {
            metadata.insert("version_id".to_string(), version_id.clone());
        }

        // Add user metadata
        for (key, value) in &response.metadata {
            if let Some(user_key) = key.strip_prefix("x-amz-meta-") {
                metadata.insert(user_key.to_string(), value.clone());
            }
        }

        let mut file_item = FileItem::new(path, response.size);
        file_item.modified_at = response.last_modified;
        file_item.metadata = metadata;
        file_item.content_type = response.content_type.clone();
        
        if let Some(etag) = &response.etag {
            file_item.md5_hash = Some(etag.trim_matches('"').to_string());
        }

        file_item
    }

    /// Apply upload options to put object args
    fn apply_upload_options(&self, mut args: PutObjectArgs<'_>, options: &UploadOptions) -> PutObjectArgs<'_> {
        if let Some(content_type) = &options.content_type {
            args = args.content_type(content_type);
        }

        if let Some(cache_control) = &options.cache_control {
            args = args.cache_control(cache_control);
        }

        if let Some(content_disposition) = &options.content_disposition {
            args = args.content_disposition(content_disposition);
        }

        // Add user metadata
        for (key, value) in &options.metadata {
            args = args.user_metadata(&format!("x-amz-meta-{}", key), value);
        }

        // Add tags as metadata (MinIO doesn't have separate tagging in this version)
        if !options.tags.is_empty() {
            let tags_value = options.tags.join(",");
            args = args.user_metadata("x-amz-meta-tags", &tags_value);
        }

        args
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

        // Auto-detect content type if not provided
        let content_type = options.content_type.clone()
            .or_else(|| FileStorageUtils::detect_content_type(path));

        let mut put_args = PutObjectArgs::new(&self.bucket, &object_name, content, Some(content.len()))
            .map_err(|e| FileStorageError::InvalidPath(format!("Invalid object args: {}", e)))?;

        if let Some(ct) = content_type {
            put_args = put_args.content_type(&ct);
        }

        // Apply upload options
        put_args = self.apply_upload_options(put_args, &options);

        // Upload the file
        self.client
            .put_object(&put_args)
            .await
            .map_err(|e| FileStorageError::IoError(format!("Failed to upload file: {}", e)))?;

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
        options: Option<DownloadOptions>,
    ) -> FileStorageResult<Vec<u8>> {
        let object_name = self.path_to_object_name(path)?;
        let _options = options.unwrap_or_default();

        let get_args = GetObjectArgs::new(&self.bucket, &object_name)
            .map_err(|e| FileStorageError::InvalidPath(format!("Invalid object args: {}", e)))?;

        let mut response = self
            .client
            .get_object(&get_args)
            .await
            .map_err(|e| FileStorageError::FileNotFound(format!("Failed to download file: {}", e)))?;

        let mut content = Vec::new();
        response
            .read_to_end(&mut content)
            .await
            .map_err(|e| FileStorageError::IoError(format!("Failed to read file content: {}", e)))?;

        self.log_operation(
            LogLevel::Info,
            format!("Downloaded file: {} ({} bytes)", path.display(), content.len()),
        ).await;

        Ok(content)
    }

    async fn delete_file(&self, path: &PathBuf) -> FileStorageResult<()> {
        let object_name = self.path_to_object_name(path)?;

        let remove_args = RemoveObjectArgs::new(&self.bucket, &object_name)
            .map_err(|e| FileStorageError::InvalidPath(format!("Invalid object args: {}", e)))?;

        self.client
            .remove_object(&remove_args)
            .await
            .map_err(|e| FileStorageError::IoError(format!("Failed to delete file: {}", e)))?;

        self.log_operation(LogLevel::Info, format!("Deleted file: {}", path.display())).await;

        Ok(())
    }

    async fn file_exists(&self, path: &PathBuf) -> FileStorageResult<bool> {
        let object_name = self.path_to_object_name(path)?;

        let stat_args = StatObjectArgs::new(&self.bucket, &object_name)
            .map_err(|e| FileStorageError::InvalidPath(format!("Invalid object args: {}", e)))?;

        match self.client.stat_object(&stat_args).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    async fn get_file_info(&self, path: &PathBuf) -> FileStorageResult<FileItem> {
        let object_name = self.path_to_object_name(path)?;

        let stat_args = StatObjectArgs::new(&self.bucket, &object_name)
            .map_err(|e| FileStorageError::InvalidPath(format!("Invalid object args: {}", e)))?;

        let response = self
            .client
            .stat_object(&stat_args)
            .await
            .map_err(|e| FileStorageError::FileNotFound(format!("File not found: {}", e)))?;

        Ok(self.stat_response_to_file_item(&response, path.clone()))
    }

    async fn list_files(&self, options: Option<ListOptions>) -> FileStorageResult<FileListResult> {
        let options = options.unwrap_or_default();

        let mut list_args = ListObjectsArgs::new(&self.bucket)
            .map_err(|e| FileStorageError::ConfigurationError(format!("Invalid list args: {}", e)))?;

        if let Some(prefix) = &options.prefix {
            list_args = list_args.prefix(prefix);
        }

        if let Some(limit) = options.limit {
            list_args = list_args.max_keys(limit as u16);
        }

        let response = self
            .client
            .list_objects_v1(&list_args)
            .await
            .map_err(|e| FileStorageError::IoError(format!("Failed to list files: {}", e)))?;

        let mut files = Vec::new();
        
        for object in response.contents {
            if let Some(key) = &object.key {
                let path = PathBuf::from(key);
                
                // Apply filters
                if self.should_include_file(&object, &path, &options) {
                    let file_item = self.object_to_file_item(&object, path);
                    files.push(file_item);
                }
            }
        }

        // Sort files by modification date (newest first)
        files.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));

        let has_more = response.is_truncated.unwrap_or(false);
        let continuation_token = response.next_marker;

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

        let copy_args = CopyObjectArgs::new(&self.bucket, &dest_object, &format!("{}/{}", self.bucket, source_object))
            .map_err(|e| FileStorageError::InvalidPath(format!("Invalid copy args: {}", e)))?;

        self.client
            .copy_object(&copy_args)
            .await
            .map_err(|e| FileStorageError::IoError(format!("Failed to copy file: {}", e)))?;

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

        // Try to check if bucket exists as a simple health check
        let exists_args = BucketExistsArgs::new(&self.bucket)
            .map_err(|e| FileStorageError::ConfigurationError(format!("Invalid bucket name: {}", e)))?;

        match self.client.bucket_exists(&exists_args).await {
            Ok(_) => {
                let response_time = start_time.elapsed().as_millis() as u64;
                
                self.log_operation(LogLevel::Info, "MinIO health check passed".to_string()).await;

                Ok(StorageHealth {
                    is_available: true,
                    response_time_ms: Some(response_time),
                    error: None,
                    checked_at: Utc::now(),
                })
            }
            Err(e) => {
                let error_msg = format!("MinIO health check failed: {}", e);
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
    /// Helper method to determine if a file should be included based on filters
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
        if let Some(size) = object.size {
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
        }

        // Filter by modification date
        if let Some(last_modified) = object.last_modified {
            if let Some(modified_after) = options.modified_after {
                if last_modified < modified_after {
                    return false;
                }
            }
            if let Some(modified_before) = options.modified_before {
                if last_modified > modified_before {
                    return false;
                }
            }
        }

        // Filter by tags (stored in metadata)
        if !options.tags.is_empty() {
            if let Some(user_metadata) = &object.user_metadata {
                if let Some(tags_value) = user_metadata.get("x-amz-meta-tags") {
                    let file_tags: Vec<&str> = tags_value.split(',').collect();
                    if !options.tags.iter().all(|tag| file_tags.contains(&tag.as_str())) {
                        return false;
                    }
                } else {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }
}

#[async_trait]
impl BatchFileStoragePort for MinioAdapter {
    async fn upload_files(
        &self,
        files: Vec<(PathBuf, Vec<u8>, Option<UploadOptions>)>,
    ) -> FileStorageResult<Vec<FileItem>> {
        let mut results = Vec::new();

        for (path, content, options) in files {
            match self.upload_file(&path, &content, options).await {
                Ok(file_item) => results.push(file_item),
                Err(e) => {
                    self.log_operation(
                        LogLevel::Error,
                        format!("Failed to upload file in batch: {} - {}", path.display(), e),
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
        
        // MinIO presigned URL generation would require additional methods
        // For now, we'll return an error indicating this feature needs implementation
        Err(FileStorageError::Unknown(
            "Presigned URL generation not yet implemented".to_string()
        ))
    }

    async fn generate_download_url(
        &self,
        path: &PathBuf,
        expires_in: Duration,
        _options: Option<DownloadOptions>,
    ) -> FileStorageResult<String> {
        let object_name = self.path_to_object_name(path)?;
        
        // MinIO presigned URL generation would require additional methods
        // For now, we'll return an error indicating this feature needs implementation
        Err(FileStorageError::Unknown(
            "Presigned URL generation not yet implemented".to_string()
        ))
    }

    async fn create_multipart_upload(
        &self,
        _path: &PathBuf,
        _options: Option<UploadOptions>,
    ) -> FileStorageResult<String> {
        Err(FileStorageError::Unknown(
            "Multipart upload not yet implemented".to_string()
        ))
    }

    async fn upload_part(
        &self,
        _upload_id: &str,
        _part_number: u32,
        _content: &[u8],
    ) -> FileStorageResult<String> {
        Err(FileStorageError::Unknown(
            "Multipart upload not yet implemented".to_string()
        ))
    }

    async fn complete_multipart_upload(
        &self,
        _upload_id: &str,
        _parts: Vec<(u32, String)>,
    ) -> FileStorageResult<FileItem> {
        Err(FileStorageError::Unknown(
            "Multipart upload not yet implemented".to_string()
        ))
    }

    async fn abort_multipart_upload(&self, _upload_id: &str) -> FileStorageResult<()> {
        Err(FileStorageError::Unknown(
            "Multipart upload not yet implemented".to_string()
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
        // MinIO client doesn't require explicit shutdown
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