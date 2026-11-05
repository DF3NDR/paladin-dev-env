use async_trait::async_trait;
use chrono::{DateTime, Utc};
use s3::Bucket;
use s3::BucketConfiguration;
use s3::creds::Credentials;
use s3::error::S3Error;
use s3::region::Region;
use s3::serde_types::{Object, Part};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use uuid::Uuid;

use paladin_core::base::entity::message::{Location, MessagePriority};
use paladin_core::platform::container::log::{LogEntry, LogLevel, LogMessage};
use paladin_ports::output::file_storage_port::{
    AdvancedFileStoragePort, BatchFileStoragePort, DownloadOptions, FileItem, FileListResult,
    FileStorageError, FileStoragePort, FileStorageResult, FileStorageUtils, FileVersioningPort,
    FullFileStoragePort, ListOptions, StorageHealth, StorageStats, UploadOptions,
};
use paladin_ports::output::log_port::LogPort;

/// Configuration for an S3-compatible store connection using rust-s3 (RustFS in
/// dev/test/CI; MinIO, AWS S3, DigitalOcean Spaces or any SigV4 S3 endpoint in production).
#[doc(hidden)]
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

/// Build the self-describing multipart upload token handed to callers.
///
/// The token is `<object key byte length>:<object key><S3 upload id>`. The
/// `AdvancedFileStoragePort` multipart methods receive only the upload id, but the S3
/// part, complete and abort calls also need the object key, so the key travels inside
/// the id. That keeps the adapter stateless: nothing leaks and the token survives a
/// restart or another replica. Callers must treat the token as opaque.
fn encode_multipart_token(object_name: &str, s3_upload_id: &str) -> String {
    format!("{}:{}{}", object_name.len(), object_name, s3_upload_id)
}

/// Split a multipart upload token into `(object key, S3 upload id)`.
///
/// Every malformed shape (no length prefix, a non-numeric length, a length past the
/// end of the token, or a length that cuts a UTF-8 character) is
/// [`FileStorageError::InvalidPath`]; this never panics and never touches the network.
fn split_token(token: &str) -> FileStorageResult<(&str, &str)> {
    let bad = || FileStorageError::InvalidPath("malformed multipart upload id".to_string());
    let (len, rest) = token.split_once(':').ok_or_else(bad)?;
    let n: usize = len.parse().map_err(|_| bad())?;
    let key = rest.get(..n).ok_or_else(bad)?;
    let upload_id = rest.get(n..).ok_or_else(bad)?;
    Ok((key, upload_id))
}

/// Decode a multipart upload token and re-validate it before any request is made.
///
/// A caller can forge a token, so the decoded key goes through the same
/// `validate_path` check `path_to_object_name` applies (no `..`, no leading `/`, not
/// empty) and an empty S3 upload id is rejected. Returns `(object key, S3 upload id)`.
fn decode_multipart_token(token: &str) -> FileStorageResult<(String, &str)> {
    let (key, upload_id) = split_token(token)?;
    if upload_id.is_empty() {
        return Err(FileStorageError::InvalidPath(
            "malformed multipart upload id".to_string(),
        ));
    }
    <() as FileStorageUtils>::validate_path(Path::new(key))?;
    Ok((key.to_string(), upload_id))
}

/// Extract the S3 `<Code>` value from an error body, bounded to 64 characters.
///
/// Only this short code is ever embedded in an error: a response body can carry
/// arbitrary text, so the body itself is never propagated.
fn s3_error_code(body: &str) -> String {
    body.split_once("<Code>")
        .and_then(|(_, rest)| rest.split_once("</Code>"))
        .map(|(code, _)| code.chars().take(64).collect())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Render an [`S3Error`] as bounded text that is safe to embed in a [`FileStorageError`], a log
/// line or a health report (45-REVIEW WR-02).
///
/// A non-2xx S3 response reaches callers as `HttpFailWithBody(status, body)`, and the body can
/// echo the access-key id, the string-to-sign and other request detail; `Display` would embed
/// all of it. Only the numeric status and the short `<Code>` value (via [`s3_error_code`], which
/// is bounded to 64 characters) are reported. Every other variant maps to a fixed label that
/// carries none of the variant's payload text, so it can never leak a secret or a signed URL.
fn s3_err_summary(e: &S3Error) -> String {
    match e {
        S3Error::HttpFailWithBody(status, body) => {
            format!("HTTP {status} {}", s3_error_code(body))
        }
        S3Error::HttpFail => "S3 request failed (non-2xx response)".to_string(),
        S3Error::MaxExpiry(_) => "presigned URL expiry exceeds the S3 maximum".to_string(),
        S3Error::Credentials(_)
        | S3Error::WLCredentials
        | S3Error::RLCredentials
        | S3Error::CredentialsReadLock
        | S3Error::CredentialsWriteLock => "S3 credentials unavailable".to_string(),
        S3Error::Io(_) => "S3 request failed (io)".to_string(),
        _ => "S3 request failed".to_string(),
    }
}

/// The arguments `list_files` sends to the S3 list-objects call, derived from the caller's
/// [`ListOptions`] by one pure function so the mapping is testable without a store.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ListPageRequest {
    prefix: String,
    delimiter: Option<String>,
    continuation_token: Option<String>,
    max_keys: Option<usize>,
}

/// Map [`ListOptions`] to the S3 list request.
///
/// `ListOptions::limit` is the page size (`max-keys`). It must never be sent as the S3
/// `delimiter`: the store would treat the limit's digits as a path separator and roll every
/// key containing them into `CommonPrefixes`, silently dropping those objects (45-REVIEW CR-01).
/// No delimiter is ever sent, so the listing is flat and complete.
fn list_page_request(options: &ListOptions) -> ListPageRequest {
    ListPageRequest {
        prefix: options.prefix.clone().unwrap_or_default(),
        delimiter: None,
        continuation_token: options.continuation_token.clone(),
        max_keys: options.limit,
    }
}

/// Adapter for an S3-compatible store using the rust-s3 crate (RustFS in dev/test/CI;
/// MinIO, AWS S3, DigitalOcean Spaces or any SigV4 S3 endpoint in production).
#[doc(hidden)]
pub struct MinioAdapter {
    bucket: Box<Bucket>,
    config: MinioConfig,
    log_port: Option<Arc<dyn LogPort>>,
}

impl MinioAdapter {
    /// Create a new S3-compatible store adapter using rust-s3
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
        )
        .map_err(|e| {
            FileStorageError::AuthenticationError(format!("Invalid credentials: {}", e))
        })?;

        // Create custom region for MinIO
        let region = if config.secure {
            Region::Custom {
                region: config
                    .region
                    .clone()
                    .unwrap_or_else(|| "us-east-1".to_string()),
                endpoint: format!("https://{}", config.endpoint),
            }
        } else {
            Region::Custom {
                region: config
                    .region
                    .clone()
                    .unwrap_or_else(|| "us-east-1".to_string()),
                endpoint: format!("http://{}", config.endpoint),
            }
        };

        // Create bucket instance
        let bucket = Bucket::new(&config.bucket, region, credentials)
            .map_err(|e| {
                FileStorageError::ConfigurationError(format!(
                    "Failed to create bucket: {}",
                    s3_err_summary(&e)
                ))
            })?
            .with_path_style();

        let adapter = Self {
            bucket,
            config,
            log_port,
        };

        // Ensure bucket exists
        adapter.ensure_bucket_exists().await?;

        adapter
            .log_operation(
                LogLevel::Info,
                "MinIO adapter initialized successfully".to_string(),
            )
            .await;

        Ok(adapter)
    }

    /// Ensure the bucket exists, create if it doesn't
    async fn ensure_bucket_exists(&self) -> FileStorageResult<()> {
        // Check if bucket exists by attempting to list objects
        match timeout(
            self.config.connection_timeout,
            self.bucket.list("".to_string(), Some("/".to_string())),
        )
        .await
        {
            Ok(Ok(_)) => {
                // Bucket exists and is accessible
                Ok(())
            }
            Ok(Err(S3Error::HttpFailWithBody(404, _))) | Ok(Err(S3Error::HttpFail)) => {
                // Bucket doesn't exist, try to create it
                self.create_bucket().await
            }
            Ok(Err(e)) => Err(FileStorageError::ConnectionError(format!(
                "Failed to check bucket: {}",
                s3_err_summary(&e)
            ))),
            Err(_) => Err(FileStorageError::Timeout),
        }
    }

    async fn create_bucket(&self) -> FileStorageResult<()> {
        let config = BucketConfiguration::default();

        // Path-style is required: RustFS (and MinIO without domain config) rejects
        // virtual-hosted bucket creation.
        match timeout(
            self.config.connection_timeout,
            Bucket::create_with_path_style(
                &self.config.bucket,
                self.bucket.region(),
                self.bucket.credentials().await.map_err(|e| {
                    FileStorageError::AuthenticationError(format!(
                        "Failed to get credentials: {}",
                        e
                    ))
                })?,
                config,
            ),
        )
        .await
        {
            Ok(Ok(_)) => {
                self.log_operation(
                    LogLevel::Info,
                    format!("Created bucket: {}", self.config.bucket),
                )
                .await;
                Ok(())
            }
            Ok(Err(e)) => {
                // Check if error is because bucket already exists
                if let S3Error::HttpFailWithBody(409, _) = e {
                    // Bucket already exists, which is fine
                    Ok(())
                } else {
                    Err(FileStorageError::ConnectionError(format!(
                        "Failed to create bucket: {}",
                        s3_err_summary(&e)
                    )))
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
                message: LogMessage::new(level, message.clone()),
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
    fn path_to_object_name(&self, path: &Path) -> FileStorageResult<String> {
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

        // The store's quote-stripped ETag, used as an opaque change token. It is composite
        // (`<md5>-<N>`) after a multipart upload, so it is not a verified digest of the content.
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
            headers.insert(
                "Content-Disposition".to_string(),
                content_disposition.clone(),
            );
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

    /// Apply the caller's filters to listed objects and append the survivors as `FileItem`s.
    fn collect_listed(&self, objects: Vec<Object>, options: &ListOptions, out: &mut Vec<FileItem>) {
        for object in objects {
            let path = PathBuf::from(&object.key);
            if self.should_include_file(&object, &path, options) {
                out.push(self.s3_object_to_file_item(&object, path));
            }
        }
    }

    /// Execute operation with timeout and retries
    async fn execute_with_retry<F, T, Fut>(&self, operation: F) -> FileStorageResult<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, S3Error>>,
    {
        let mut last_error: Option<S3Error> = None;

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

        Err(FileStorageError::IoError(format!(
            "Operation failed after {} retries: {}",
            self.config.max_retries,
            last_error
                .map(|e| s3_err_summary(&e))
                .unwrap_or_else(|| "Unknown error".to_string())
        )))
    }
}

#[async_trait]
impl FileStoragePort for MinioAdapter {
    async fn upload_file(
        &self,
        path: &Path,
        content: &[u8],
        options: Option<UploadOptions>,
    ) -> FileStorageResult<FileItem> {
        let object_name = self.path_to_object_name(path)?;
        let options = options.unwrap_or_default();

        // Check if file exists and overwrite is disabled
        if !options.overwrite && self.file_exists(path).await? {
            return Err(FileStorageError::InvalidPath(format!(
                "File already exists: {}",
                path.display()
            )));
        }

        // Create headers
        let headers = self.create_upload_headers(&options);

        // Auto-detect content type if not provided
        let content_type = headers
            .get("Content-Type")
            .cloned()
            .or_else(|| <() as FileStorageUtils>::detect_content_type(path))
            .unwrap_or_else(|| "application/octet-stream".to_string());

        // Upload the file
        self.execute_with_retry(|| {
            self.bucket
                .put_object_with_content_type(&object_name, content, &content_type)
        })
        .await
        .map_err(|e| FileStorageError::IoError(format!("Failed to upload file: {}", e)))?;

        // Get file info after upload
        let file_item = self.get_file_info(path).await?;

        self.log_operation(
            LogLevel::Info,
            format!(
                "Uploaded file: {} ({} bytes)",
                path.display(),
                content.len()
            ),
        )
        .await;

        Ok(file_item)
    }

    async fn download_file(
        &self,
        path: &Path,
        _options: Option<DownloadOptions>,
    ) -> FileStorageResult<Vec<u8>> {
        let object_name = self.path_to_object_name(path)?;

        let response = self
            .execute_with_retry(|| self.bucket.get_object(&object_name))
            .await
            .map_err(|e| {
                FileStorageError::FileNotFound(format!("Failed to download file: {}", e))
            })?;

        let content = response.bytes().to_vec();

        self.log_operation(
            LogLevel::Info,
            format!(
                "Downloaded file: {} ({} bytes)",
                path.display(),
                content.len()
            ),
        )
        .await;

        Ok(content)
    }

    async fn delete_file(&self, path: &Path) -> FileStorageResult<()> {
        let object_name = self.path_to_object_name(path)?;

        self.execute_with_retry(|| self.bucket.delete_object(&object_name))
            .await
            .map_err(|e| FileStorageError::IoError(format!("Failed to delete file: {}", e)))?;

        self.log_operation(LogLevel::Info, format!("Deleted file: {}", path.display()))
            .await;

        Ok(())
    }

    async fn file_exists(&self, path: &Path) -> FileStorageResult<bool> {
        let object_name = self.path_to_object_name(path)?;

        match timeout(
            self.config.connection_timeout,
            self.bucket.head_object(&object_name),
        )
        .await
        {
            Ok(Ok(_)) => Ok(true),
            Ok(Err(S3Error::HttpFail)) | Ok(Err(S3Error::HttpFailWithBody(404, _))) => Ok(false),
            Ok(Err(e)) => Err(FileStorageError::IoError(format!(
                "Failed to check file existence: {}",
                s3_err_summary(&e)
            ))),
            Err(_) => Err(FileStorageError::Timeout),
        }
    }

    async fn get_file_info(&self, path: &Path) -> FileStorageResult<FileItem> {
        let object_name = self.path_to_object_name(path)?;

        let (head_result, _) = self
            .execute_with_retry(|| self.bucket.head_object(&object_name))
            .await
            .map_err(|e| FileStorageError::FileNotFound(format!("File not found: {}", e)))?;

        // Parse the response to create FileItem
        let mut metadata = HashMap::new();

        // Extract user-defined metadata if available
        if let Some(user_metadata) = &head_result.metadata {
            for (key, value) in user_metadata {
                metadata.insert(key.clone(), value.clone());
            }
        }

        let size = head_result.content_length.unwrap_or(0) as u64;

        let mut file_item = FileItem::new(path.to_path_buf(), size);
        file_item.metadata = metadata;
        file_item.content_type = head_result.content_type.clone();

        // The store's quote-stripped ETag, used as an opaque change token. It is composite
        // (`<md5>-<N>`) after a multipart upload, so it is not a verified digest of the content.
        if let Some(etag) = &head_result.e_tag {
            file_item.md5_hash = Some(etag.trim_matches('"').to_string());
        }

        if let Some(last_modified) = &head_result.last_modified
            && let Ok(dt) = DateTime::parse_from_rfc2822(last_modified)
        {
            file_item.modified_at = dt.with_timezone(&Utc);
        }

        Ok(file_item)
    }

    /// List files, honouring `limit` as the page size and `continuation_token` as the resume
    /// point.
    ///
    /// With a `limit` or a `continuation_token` this issues exactly one S3 list-objects page and
    /// reports `has_more` / `continuation_token` from that page's truncation state, so a caller
    /// can follow the token to the rest. Without either it lists every page. The prefix, size and
    /// extension filters run after the fetch, so a filtered page may hold fewer than `limit`
    /// items while `has_more` is still `true`; callers keep following the token until it is
    /// `None`.
    async fn list_files(&self, options: Option<ListOptions>) -> FileStorageResult<FileListResult> {
        let options = options.unwrap_or_default();
        let req = list_page_request(&options);
        let paged = req.max_keys.is_some() || req.continuation_token.is_some();

        let mut files = Vec::new();
        let mut has_more = false;
        let mut continuation_token = None;

        if paged {
            let (page, _status) = self
                .execute_with_retry(|| {
                    self.bucket.list_page(
                        req.prefix.clone(),
                        req.delimiter.clone(),
                        req.continuation_token.clone(),
                        None,
                        req.max_keys,
                    )
                })
                .await
                .map_err(|e| FileStorageError::IoError(format!("Failed to list files: {}", e)))?;

            has_more = page.is_truncated;
            continuation_token = page.next_continuation_token;
            self.collect_listed(page.contents, &options, &mut files);
        } else {
            let results = self
                .execute_with_retry(|| self.bucket.list(req.prefix.clone(), req.delimiter.clone()))
                .await
                .map_err(|e| FileStorageError::IoError(format!("Failed to list files: {}", e)))?;

            for list_result in results {
                self.collect_listed(list_result.contents, &options, &mut files);
            }
        }

        // Sort files by modification date (newest first)
        files.sort_by_key(|b| std::cmp::Reverse(b.modified_at));

        Ok(FileListResult {
            files,
            continuation_token,
            has_more,
            total_count: None,
        })
    }

    async fn copy_file(
        &self,
        source_path: &Path,
        destination_path: &Path,
    ) -> FileStorageResult<FileItem> {
        let source_object = self.path_to_object_name(source_path)?;
        let dest_object = self.path_to_object_name(destination_path)?;

        // rust-s3's `copy_object_internal` prepends `<bucket>/` itself, so pass the bare key.
        self.execute_with_retry(|| {
            self.bucket
                .copy_object_internal(&source_object, &dest_object)
        })
        .await
        .map_err(|e| FileStorageError::IoError(format!("Failed to copy file: {}", e)))?;

        let file_item = self.get_file_info(destination_path).await?;

        self.log_operation(
            LogLevel::Info,
            format!(
                "Copied file: {} -> {}",
                source_path.display(),
                destination_path.display()
            ),
        )
        .await;

        Ok(file_item)
    }

    async fn move_file(
        &self,
        source_path: &Path,
        destination_path: &Path,
    ) -> FileStorageResult<FileItem> {
        // Copy file to new location
        let file_item = self.copy_file(source_path, destination_path).await?;

        // Delete original file
        self.delete_file(source_path).await?;

        self.log_operation(
            LogLevel::Info,
            format!(
                "Moved file: {} -> {}",
                source_path.display(),
                destination_path.display()
            ),
        )
        .await;

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

            let file_type = file.extension().unwrap_or("unknown").to_lowercase();

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
            self.bucket
                .list_page(String::new(), None, None, None, Some(1)),
        )
        .await
        {
            Ok(Ok(_)) => {
                let response_time = start_time.elapsed().as_millis() as u64;

                self.log_operation(LogLevel::Info, "MinIO health check passed".to_string())
                    .await;

                Ok(StorageHealth {
                    is_available: true,
                    response_time_ms: Some(response_time),
                    error: None,
                    checked_at: Utc::now(),
                })
            }
            Ok(Err(e)) => {
                let error_msg = format!("MinIO health check failed: {}", s3_err_summary(&e));
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
    fn should_include_file(&self, object: &Object, path: &Path, options: &ListOptions) -> bool {
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
        let size = object.size; // Remove try_into() since object.size is already u64
        if let Some(min_size) = options.min_size
            && size < min_size
        {
            return false;
        }
        if let Some(max_size) = options.max_size
            && size > max_size
        {
            return false;
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

            if let Some(modified_after) = options.modified_after
                && parsed_date < modified_after
            {
                return false;
            }
            if let Some(modified_before) = options.modified_before
                && parsed_date > modified_before
            {
                return false;
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
        let upload_tasks: Vec<_> = files
            .into_iter()
            .map(|(path, content, options)| async move {
                self.upload_file(&path, &content, options).await
            })
            .collect();

        // Execute uploads concurrently
        for task in upload_tasks {
            match task.await {
                Ok(file_item) => results.push(file_item),
                Err(e) => {
                    self.log_operation(
                        LogLevel::Error,
                        format!("Failed to upload file in batch: {}", e),
                    )
                    .await;
                    return Err(e);
                }
            }
        }

        self.log_operation(
            LogLevel::Info,
            format!("Batch uploaded {} files", results.len()),
        )
        .await;

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
                        format!(
                            "Failed to download file in batch: {} - {}",
                            path.display(),
                            e
                        ),
                    )
                    .await;
                    return Err(e);
                }
            }
        }

        self.log_operation(
            LogLevel::Info,
            format!("Batch downloaded {} files", results.len()),
        )
        .await;

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
                    )
                    .await;
                    return Err(e);
                }
            }
        }

        self.log_operation(
            LogLevel::Info,
            format!("Batch deleted {} files", deleted.len()),
        )
        .await;

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
                        format!(
                            "Failed to get file info in batch: {} - {}",
                            path.display(),
                            e
                        ),
                    )
                    .await;
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
        path: &Path,
        expires_in: Duration,
        _options: Option<UploadOptions>,
    ) -> FileStorageResult<String> {
        let object_name = self.path_to_object_name(path)?;
        let expires_in_secs = expires_in.as_secs() as u32;

        // Fix: Add the missing 4th parameter (custom query parameters)
        let url = self
            .bucket
            .presign_put(&object_name, expires_in_secs, None, None)
            .await
            .map_err(|e| {
                FileStorageError::IoError(format!(
                    "Failed to generate upload URL: {}",
                    s3_err_summary(&e)
                ))
            })?;

        Ok(url)
    }

    async fn generate_download_url(
        &self,
        path: &Path,
        expires_in: Duration,
        _options: Option<DownloadOptions>,
    ) -> FileStorageResult<String> {
        let object_name = self.path_to_object_name(path)?;
        let expires_in_secs = expires_in.as_secs() as u32;

        // Fix: Add the missing 4th parameter (custom query parameters)
        let url = self
            .bucket
            .presign_get(&object_name, expires_in_secs, None)
            .await
            .map_err(|e| {
                FileStorageError::IoError(format!(
                    "Failed to generate download URL: {}",
                    s3_err_summary(&e)
                ))
            })?;

        Ok(url)
    }

    /// Start a multipart upload and return an opaque upload token.
    ///
    /// The returned id is `<key length>:<key><S3 upload id>`; callers must pass it back
    /// unchanged to [`upload_part`](Self::upload_part),
    /// [`complete_multipart_upload`](Self::complete_multipart_upload) or
    /// [`abort_multipart_upload`](Self::abort_multipart_upload).
    async fn create_multipart_upload(
        &self,
        path: &Path,
        _options: Option<UploadOptions>,
    ) -> FileStorageResult<String> {
        let object_name = self.path_to_object_name(path)?;

        let response = self
            .bucket
            .initiate_multipart_upload(&object_name, "application/octet-stream")
            .await
            .map_err(|e| {
                FileStorageError::IoError(format!(
                    "Failed to initiate multipart upload: {}",
                    s3_err_summary(&e)
                ))
            })?;

        Ok(encode_multipart_token(&object_name, &response.upload_id))
    }

    /// Upload one part and return the part's ETag (raw, opaque to the port).
    ///
    /// Every non-final part must be at least 5 MiB on S3 and RustFS.
    ///
    /// A rejected part returns an error and **leaves the multipart upload open**: this crate
    /// builds `rust-s3` with its default `fail-on-err` feature, which turns a non-2xx response
    /// into an error before `rust-s3`'s own abort-on-rejected-part branch can run. The caller
    /// must call [`FileStoragePort::abort_multipart_upload`] to discard the parts already
    /// stored, or the store keeps (and bills for) them. Only the bounded S3 status and code
    /// are reported in the error, never the response body (45-REVIEW WR-01, WR-02).
    async fn upload_part(
        &self,
        upload_id: &str,
        part_number: u32,
        content: &[u8],
    ) -> FileStorageResult<String> {
        let (key, id) = decode_multipart_token(upload_id)?;

        let part = self
            .bucket
            .put_multipart_chunk(
                content.to_vec(),
                &key,
                part_number,
                id,
                "application/octet-stream",
            )
            .await
            .map_err(|e| {
                FileStorageError::IoError(format!("Failed to upload part: {}", s3_err_summary(&e)))
            })?;

        Ok(part.etag)
    }

    /// Complete a multipart upload from its `(part number, ETag)` pairs.
    ///
    /// S3 can answer `200` with an `<Error>` body on completion, so both the status and
    /// the body are checked; only the bounded S3 status and error code are reported, never the
    /// body. A failed completion, like a rejected part, leaves the multipart upload open: the
    /// caller must call [`FileStoragePort::abort_multipart_upload`] to discard it (45-REVIEW WR-01).
    async fn complete_multipart_upload(
        &self,
        upload_id: &str,
        parts: Vec<(u32, String)>,
    ) -> FileStorageResult<FileItem> {
        let (key, id) = decode_multipart_token(upload_id)?;

        let parts = parts
            .into_iter()
            .map(|(part_number, etag)| Part { part_number, etag })
            .collect();

        let response = self
            .bucket
            .complete_multipart_upload(&key, id, parts)
            .await
            .map_err(|e| {
                FileStorageError::IoError(format!(
                    "Failed to complete multipart upload: {}",
                    s3_err_summary(&e)
                ))
            })?;

        let body = response.as_str().unwrap_or_default();
        // `fail-on-err` (a `rust-s3` default feature this crate leaves on) already turns a
        // non-2xx response into an `Err` above, so the status test is defensive. The `<Error>`
        // body test is live: S3 can answer `200` with an error document on completion.
        if response.status_code() >= 300 || body.contains("<Error>") {
            return Err(FileStorageError::IoError(format!(
                "Failed to complete multipart upload: {}",
                s3_error_code(body)
            )));
        }

        self.get_file_info(Path::new(&key)).await
    }

    /// Abort a multipart upload, discarding every uploaded part.
    ///
    /// A `404 NoSuchUpload` (already aborted or completed) is
    /// [`FileStorageError::FileNotFound`]. Nothing aborts an upload on the caller's behalf, not
    /// even after a rejected part, so this is the call that releases a failed upload's parts;
    /// it reports `FileNotFound` only when the upload no longer exists (45-REVIEW WR-01).
    async fn abort_multipart_upload(&self, upload_id: &str) -> FileStorageResult<()> {
        let (key, id) = decode_multipart_token(upload_id)?;

        match self.bucket.abort_upload(&key, id).await {
            Ok(()) => Ok(()),
            Err(S3Error::HttpFailWithBody(404, body)) if body.contains("NoSuchUpload") => {
                Err(FileStorageError::FileNotFound(
                    "multipart upload not found or already finished".to_string(),
                ))
            }
            Err(e) => Err(FileStorageError::IoError(format!(
                "Failed to abort multipart upload: {}",
                s3_err_summary(&e)
            ))),
        }
    }
}

#[async_trait]
impl FileVersioningPort for MinioAdapter {
    async fn upload_file_version(
        &self,
        path: &Path,
        content: &[u8],
        options: Option<UploadOptions>,
    ) -> FileStorageResult<FileItem> {
        // For basic versioning, we can append a timestamp to the filename
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let mut versioned_path = path.to_path_buf();

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

    async fn list_file_versions(&self, path: &Path) -> FileStorageResult<Vec<FileItem>> {
        let filename_stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| FileStorageError::InvalidPath("Invalid filename".to_string()))?;

        let prefix = path
            .parent()
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
        path: &Path,
        _version_id: &str,
        options: Option<DownloadOptions>,
    ) -> FileStorageResult<Vec<u8>> {
        // For simple versioning, treat version_id as the versioned filename
        self.download_file(path, options).await
    }

    async fn delete_file_version(&self, path: &Path, _version_id: &str) -> FileStorageResult<()> {
        // For simple versioning, treat version_id as the versioned filename
        self.delete_file(path).await
    }

    async fn get_file_version_info(
        &self,
        path: &Path,
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
        self.log_operation(LogLevel::Info, "Shutting down MinIO adapter".to_string())
            .await;
        // rust-s3 handles connection cleanup automatically
        Ok(())
    }

    /// Get MinIO connection info for debugging
    pub fn get_connection_info(&self) -> String {
        format!(
            "{}://{}/{}",
            if self.config.secure { "https" } else { "http" },
            self.config.endpoint,
            self.config.bucket
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn retry_exhaustion_text_never_echoes_the_response_body() {
        let credentials = Credentials::new(Some("ak"), Some("sk"), None, None, None).unwrap();
        let region = Region::Custom {
            region: "us-east-1".to_string(),
            endpoint: "http://127.0.0.1:1".to_string(),
        };
        let adapter = MinioAdapter {
            bucket: Bucket::new("b", region, credentials)
                .unwrap()
                .with_path_style(),
            config: MinioConfig {
                max_retries: 0,
                ..MinioConfig::default()
            },
            log_port: None,
        };

        let err = adapter
            .execute_with_retry(|| async {
                Err::<(), _>(S3Error::HttpFailWithBody(
                    403,
                    "<Error><Code>AccessDenied</Code><AWSAccessKeyId>AKIAEXAMPLE0000000000\
                     </AWSAccessKeyId></Error>"
                        .to_string(),
                ))
            })
            .await
            .unwrap_err();
        let text = err.to_string();
        assert!(text.contains("HTTP 403 AccessDenied"), "got {text}");
        assert!(!text.contains("AKIA"), "got {text}");
    }

    #[test]
    fn s3_err_summary_never_echoes_the_response_body() {
        let body = "<Error><Code>SignatureDoesNotMatch</Code>\
            <AWSAccessKeyId>AKIAEXAMPLE0000000000</AWSAccessKeyId>\
            <StringToSign>secret-request-detail</StringToSign></Error>";
        let summary = s3_err_summary(&S3Error::HttpFailWithBody(403, body.to_string()));
        assert_eq!(summary, "HTTP 403 SignatureDoesNotMatch");
        assert!(!summary.contains("AKIA"));
        assert!(!summary.contains("secret-request-detail"));

        // A body with no <Code> reports the status and the fixed fallback code.
        let summary = s3_err_summary(&S3Error::HttpFailWithBody(
            500,
            "internal detail AKIAEXAMPLE0000000000".to_string(),
        ));
        assert_eq!(summary, format!("HTTP 500 {}", s3_error_code("")));
        assert!(!summary.contains("AKIA"));
        assert!(!summary.contains("internal detail"));

        // A non-HTTP variant yields a fixed string carrying none of its payload text.
        let summary = s3_err_summary(&S3Error::Credentials(
            s3::creds::error::CredentialsError::ConfigMissingAccessKeyId,
        ));
        assert!(!summary.is_empty());
        assert!(!summary.to_lowercase().contains("accesskeyid"));
        let summary = s3_err_summary(&S3Error::Io(std::io::Error::other(
            "secret-io-detail AKIAEXAMPLE0000000000",
        )));
        assert!(!summary.contains("secret-io-detail"));
        assert!(!summary.contains("AKIA"));
        assert!(!s3_err_summary(&S3Error::HttpFail).is_empty());
        assert!(!s3_err_summary(&S3Error::MaxExpiry(999_999)).contains("999999"));
    }

    #[test]
    fn list_request_sends_the_limit_as_max_keys_never_as_a_delimiter() {
        let options = ListOptions {
            prefix: Some("p/".to_string()),
            limit: Some(5),
            continuation_token: Some("tok".to_string()),
            ..Default::default()
        };
        let req = list_page_request(&options);
        assert_eq!(req.delimiter, None, "the limit must never be a delimiter");
        assert_eq!(req.max_keys, Some(5));
        assert_eq!(req.continuation_token, Some("tok".to_string()));
        assert_eq!(req.prefix, "p/");

        let req = list_page_request(&ListOptions::default());
        assert_eq!(req.delimiter, None);
        assert_eq!(req.max_keys, None);
        assert_eq!(req.continuation_token, None);
    }

    #[test]
    fn test_minio_config_default() {
        let config = MinioConfig::default();

        assert_eq!(config.endpoint, "localhost:9000");
        assert_eq!(config.access_key, "minioadmin");
        assert_eq!(config.secret_key, "minioadmin");
        assert_eq!(config.bucket, "paladin-files");
        assert_eq!(config.region, Some("us-east-1".to_string()));
        assert!(!config.secure);
        assert!(config.path_style);
        assert_eq!(config.connection_timeout, Duration::from_secs(30));
        assert_eq!(config.request_timeout, Duration::from_secs(300));
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.max_idle_conns, 10);
    }

    #[test]
    fn test_minio_config_custom() {
        let config = MinioConfig {
            endpoint: "s3.amazonaws.com".to_string(),
            access_key: "AKIAIOSFODNN7EXAMPLE".to_string(),
            secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
            bucket: "my-bucket".to_string(),
            region: Some("us-west-2".to_string()),
            secure: true,
            path_style: false,
            connection_timeout: Duration::from_secs(60),
            request_timeout: Duration::from_secs(600),
            max_retries: 5,
            max_idle_conns: 20,
        };

        assert_eq!(config.endpoint, "s3.amazonaws.com");
        assert_eq!(config.access_key, "AKIAIOSFODNN7EXAMPLE");
        assert_eq!(config.bucket, "my-bucket");
        assert_eq!(config.region, Some("us-west-2".to_string()));
        assert!(config.secure);
        assert!(!config.path_style);
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.max_idle_conns, 20);
    }

    #[test]
    fn test_minio_config_clone() {
        let config1 = MinioConfig::default();
        let config2 = config1.clone();

        assert_eq!(config1.endpoint, config2.endpoint);
        assert_eq!(config1.bucket, config2.bucket);
        assert_eq!(config1.max_retries, config2.max_retries);
    }

    #[test]
    fn test_minio_config_debug_format() {
        let config = MinioConfig::default();
        let debug_str = format!("{:?}", config);

        assert!(debug_str.contains("MinioConfig"));
        assert!(debug_str.contains("endpoint"));
        assert!(debug_str.contains("localhost:9000"));
    }

    #[test]
    fn test_minio_config_serialization() {
        let config = MinioConfig::default();

        // Test that serialization works.
        let serialized = serde_json::to_string(&config).expect("Should serialize");
        assert!(serialized.contains("localhost:9000"));
        assert!(serialized.contains("paladin-files"));

        // R-95: `Serialize` omits both keys, so the round trip supplies them itself (this
        // asserts nothing about whether the keys are present in the serialised text, which is
        // what keeps it passing before and after the fix).
        let mut value: serde_json::Value =
            serde_json::from_str(&serialized).expect("Should parse as JSON");
        let object = value.as_object_mut().expect("serialises as an object");
        object.insert(
            "access_key".to_string(),
            serde_json::json!("test-key-not-a-secret-r95-minio-round-trip-access"),
        );
        object.insert(
            "secret_key".to_string(),
            serde_json::json!("test-key-not-a-secret-r95-minio-round-trip-secret"),
        );

        // Test deserialization
        let deserialized: MinioConfig = serde_json::from_value(value).expect("Should deserialize");
        assert_eq!(deserialized.endpoint, config.endpoint);
        assert_eq!(deserialized.bucket, config.bucket);
        assert_eq!(
            deserialized.access_key,
            "test-key-not-a-secret-r95-minio-round-trip-access"
        );
        assert_eq!(
            deserialized.secret_key,
            "test-key-not-a-secret-r95-minio-round-trip-secret"
        );
    }

    /// R-95 (58-06 register, 56-REVIEW WR-09 class): serialising a `MinioConfig` must
    /// write neither key.
    #[test]
    fn minio_config_serialisation_omits_both_keys() {
        let access = "test-key-not-a-secret-r95-minio-access-key";
        let secret = "test-key-not-a-secret-r95-minio-secret-key";
        let config = MinioConfig {
            access_key: access.to_string(),
            secret_key: secret.to_string(),
            ..MinioConfig::default()
        };
        assert_eq!(config.access_key, access);
        assert_eq!(config.secret_key, secret);
        let value = serde_json::to_value(&config).expect("MinioConfig serialises");
        let object = value.as_object().expect("serialises as an object");
        for key in ["access_key", "secret_key"] {
            assert!(!object.contains_key(key), "MinioConfig serialised `{key}`");
        }
        let text = value.to_string();
        assert!(
            !text.contains(access),
            "serialisation carries the access key"
        );
        assert!(
            !text.contains(secret),
            "serialisation carries the secret key"
        );
        // Non-vacuous: the neighbouring fields are still written.
        assert_eq!(object["endpoint"], "localhost:9000");
        assert_eq!(object["bucket"], "paladin-files");
    }

    /// R-95 (DF-07 class): a `Debug` rendering must carry neither key.
    #[test]
    fn minio_config_debug_redacts_both_keys() {
        let access = "test-key-not-a-secret-r95-minio-access-key";
        let secret = "test-key-not-a-secret-r95-minio-secret-key";
        let config = MinioConfig {
            endpoint: "files.example.internal:9000".to_string(),
            access_key: access.to_string(),
            secret_key: secret.to_string(),
            ..MinioConfig::default()
        };
        assert_eq!(config.access_key, access);
        assert_eq!(config.secret_key, secret);
        let rendered = format!("{config:?}");
        assert!(!rendered.contains(access), "access key leaked: {rendered}");
        assert!(!rendered.contains(secret), "secret key leaked: {rendered}");
        assert!(
            rendered.contains("<redacted>"),
            "marker missing: {rendered}"
        );
        assert!(rendered.contains("files.example.internal:9000"));
        assert!(rendered.contains("paladin-files"));

        // An empty key shows the empty-string form, not the marker.
        let empty = MinioConfig {
            access_key: String::new(),
            secret_key: String::new(),
            ..MinioConfig::default()
        };
        let rendered = format!("{empty:?}");
        assert!(rendered.contains("access_key: \"\""), "{rendered}");
        assert!(rendered.contains("secret_key: \"\""), "{rendered}");
        assert!(!rendered.contains("<redacted>"), "{rendered}");
    }

    /// R-95 drift guard: the pretty `Debug` of a default value names every serialised
    /// field plus both key names, so a later field cannot be dropped from the
    /// hand-written `Debug`.
    #[test]
    fn minio_config_debug_names_every_field() {
        let rendered = format!("{:#?}", MinioConfig::default());
        let value = serde_json::to_value(MinioConfig::default()).expect("serialises");
        let object = value.as_object().expect("serialises as an object");
        for key in object
            .keys()
            .map(String::as_str)
            .chain(["access_key", "secret_key"])
        {
            assert!(
                rendered.contains(&format!("{key}:")),
                "Debug output does not name `{key}`: {rendered}"
            );
        }
    }

    /// R-95 load pin: both keys stay required on load (no serde default), so a value
    /// missing either one fails naming it, and a value with both loads.
    #[test]
    fn minio_config_requires_both_keys_on_load() {
        let mut base = serde_json::to_value(MinioConfig::default()).expect("serialises");
        let object = base.as_object_mut().expect("serialises as an object");
        object.remove("access_key");
        object.remove("secret_key");

        let mut only_secret = base.clone();
        only_secret.as_object_mut().expect("object").insert(
            "secret_key".to_string(),
            serde_json::json!("test-key-not-a-secret-r95-minio-load-secret"),
        );
        let err = serde_json::from_value::<MinioConfig>(only_secret)
            .expect_err("a value without access_key must not load")
            .to_string();
        assert!(
            err.contains("missing field") && err.contains("access_key"),
            "{err}"
        );

        let mut only_access = base.clone();
        only_access.as_object_mut().expect("object").insert(
            "access_key".to_string(),
            serde_json::json!("test-key-not-a-secret-r95-minio-load-access"),
        );
        let err = serde_json::from_value::<MinioConfig>(only_access)
            .expect_err("a value without secret_key must not load")
            .to_string();
        assert!(
            err.contains("missing field") && err.contains("secret_key"),
            "{err}"
        );

        let mut both = base;
        let object = both.as_object_mut().expect("object");
        object.insert(
            "access_key".to_string(),
            serde_json::json!("test-key-not-a-secret-r95-minio-load-access"),
        );
        object.insert(
            "secret_key".to_string(),
            serde_json::json!("test-key-not-a-secret-r95-minio-load-secret"),
        );
        let loaded: MinioConfig = serde_json::from_value(both).expect("both keys load");
        assert_eq!(
            loaded.access_key,
            "test-key-not-a-secret-r95-minio-load-access"
        );
        assert_eq!(
            loaded.secret_key,
            "test-key-not-a-secret-r95-minio-load-secret"
        );
    }

    #[test]
    fn test_minio_config_with_optional_region() {
        let config_with_region = MinioConfig {
            region: Some("eu-west-1".to_string()),
            ..Default::default()
        };
        assert_eq!(config_with_region.region, Some("eu-west-1".to_string()));

        let config_without_region = MinioConfig {
            region: None,
            ..Default::default()
        };
        assert_eq!(config_without_region.region, None);
    }

    #[test]
    fn test_minio_config_timeout_values() {
        let config = MinioConfig {
            connection_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(120),
            ..Default::default()
        };

        assert_eq!(config.connection_timeout.as_secs(), 10);
        assert_eq!(config.request_timeout.as_secs(), 120);
    }

    #[test]
    fn test_minio_config_retry_settings() {
        let config = MinioConfig {
            max_retries: 0,
            ..Default::default()
        };
        assert_eq!(config.max_retries, 0);

        let config_with_retries = MinioConfig {
            max_retries: 10,
            ..Default::default()
        };
        assert_eq!(config_with_retries.max_retries, 10);
    }

    #[test]
    fn test_minio_config_secure_https_endpoint() {
        let secure_config = MinioConfig {
            endpoint: "s3.amazonaws.com".to_string(),
            secure: true,
            ..Default::default()
        };
        assert!(secure_config.secure);

        let insecure_config = MinioConfig {
            endpoint: "localhost:9000".to_string(),
            secure: false,
            ..Default::default()
        };
        assert!(!insecure_config.secure);
    }

    #[test]
    fn test_minio_config_path_style_setting() {
        let path_style_config = MinioConfig {
            path_style: true,
            ..Default::default()
        };
        assert!(path_style_config.path_style);

        let virtual_hosted_config = MinioConfig {
            path_style: false,
            ..Default::default()
        };
        assert!(!virtual_hosted_config.path_style);
    }

    #[test]
    fn multipart_token_round_trips() {
        let token = encode_multipart_token("uploads/a.bin", "ID-1");
        assert_eq!(token, "13:uploads/a.binID-1");

        let (key, id) = split_token(&token).expect("well-formed token splits");
        assert_eq!(key, "uploads/a.bin");
        assert_eq!(id, "ID-1");

        let (key, id) = decode_multipart_token(&token).expect("well-formed token decodes");
        assert_eq!(key, "uploads/a.bin");
        assert_eq!(id, "ID-1");
    }

    #[test]
    fn split_token_rejects_malformed_tokens() {
        // no length prefix, non-numeric length, length past the end, and a slice that
        // would cut a multi-byte character in half.
        for bad in ["no-colon", "x:abc", "99:short", "1:\u{e9}rest"] {
            assert!(
                matches!(split_token(bad), Err(FileStorageError::InvalidPath(_))),
                "expected InvalidPath for {bad:?}"
            );
        }
    }

    #[test]
    fn decode_multipart_token_rejects_a_traversal_key() {
        let token = encode_multipart_token("../escape.bin", "ID-1");
        assert!(matches!(
            decode_multipart_token(&token),
            Err(FileStorageError::InvalidPath(_))
        ));
    }

    #[test]
    fn decode_multipart_token_rejects_an_empty_upload_id() {
        let token = encode_multipart_token("uploads/a.bin", "");
        assert!(matches!(
            decode_multipart_token(&token),
            Err(FileStorageError::InvalidPath(_))
        ));
    }

    // Note: Tests requiring actual MinIO connection are in integration tests
    // The following would require mocking or actual MinIO instance:
    // - new()
    // - upload_file()
    // - download_file()
    // - delete_file()
    // - list_files()
    // - health_check()
}
