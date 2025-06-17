/*
File Storage Client

This output adapter implements a content delivery service that writes content to a file system.
It implements the ContentDeliveryService trait and can be used to store content items,
content lists, notifications, and alerts to various file formats and locations.

The adapter supports multiple storage formats (JSON, XML, plain text) and provides
comprehensive file management capabilities including directory organization,
file naming strategies, and metadata preservation.

This is designed to be used as a backup storage solution, content archival system,
or for debugging and auditing purposes in the information gathering and delivery pipeline.
*/

use crate::application::ports::output::content_delivery_port::{
    ContentDeliveryService, BatchContentDeliveryService, DeliveryRequest, DeliveryResponse,
    DeliveryMethod, ContentPayload, DeliveryStatus, DeliveryStats, ContentDeliveryError,
    DeliveryPriority
};
use crate::core::platform::container::content::{ContentItem, ContentType};
use crate::core::platform::container::content_list::ContentList;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct FileStorageClient {
    base_path: PathBuf,
    delivery_history: Arc<Mutex<HashMap<Uuid, DeliveryResponse>>>,
    file_format: FileFormat,
    naming_strategy: NamingStrategy,
    max_file_size_bytes: u64,
    create_directories: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FileFormat {
    Json,
    Xml,
    PlainText,
    Csv,
    Markdown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NamingStrategy {
    Timestamp,
    Uuid,
    Sequential,
    ContentBased,
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct FileStorageConfig {
    pub base_path: PathBuf,
    pub file_format: FileFormat,
    pub naming_strategy: NamingStrategy,
    pub max_file_size_bytes: Option<u64>,
    pub create_directories: bool,
}

impl Default for FileStorageConfig {
    fn default() -> Self {
        Self {
            base_path: PathBuf::from("./storage"),
            file_format: FileFormat::Json,
            naming_strategy: NamingStrategy::Timestamp,
            max_file_size_bytes: Some(10 * 1024 * 1024), // 10MB default
            create_directories: true,
        }
    }
}

impl FileStorageClient {
    pub fn new(config: FileStorageConfig) -> Result<Self, ContentDeliveryError> {
        let base_path = config.base_path;
        
        // Create base directory if it doesn't exist and create_directories is enabled
        if config.create_directories && !base_path.exists() {
            fs::create_dir_all(&base_path)
                .map_err(|e| ContentDeliveryError::DeliveryFailed(
                    format!("Failed to create storage directory: {}", e)
                ))?;
        }

        // Verify base path is writable
        if !base_path.exists() || !base_path.is_dir() {
            return Err(ContentDeliveryError::DeliveryFailed(
                format!("Storage path does not exist or is not a directory: {:?}", base_path)
            ));
        }

        Ok(Self {
            base_path,
            delivery_history: Arc::new(Mutex::new(HashMap::new())),
            file_format: config.file_format,
            naming_strategy: config.naming_strategy,
            max_file_size_bytes: config.max_file_size_bytes.unwrap_or(u64::MAX),
            create_directories: config.create_directories,
        })
    }

    fn generate_filename(&self, content_type: &str, delivery_id: &Uuid) -> String {
        let extension = match self.file_format {
            FileFormat::Json => "json",
            FileFormat::Xml => "xml",
            FileFormat::PlainText => "txt",
            FileFormat::Csv => "csv",
            FileFormat::Markdown => "md",
        };

        match &self.naming_strategy {
            NamingStrategy::Timestamp => {
                format!("{}_{}.{}", content_type, Utc::now().format("%Y%m%d_%H%M%S_%3f"), extension)
            },
            NamingStrategy::Uuid => {
                format!("{}_{}.{}", content_type, delivery_id, extension)
            },
            NamingStrategy::Sequential => {
                // Find next sequential number
                let mut counter = 1;
                loop {
                    let filename = format!("{}_{:06}.{}", content_type, counter, extension);
                    let path = self.base_path.join(&filename);
                    if !path.exists() {
                        break filename;
                    }
                    counter += 1;
                }
            },
            NamingStrategy::ContentBased => {
                // Use a hash of content for naming (simplified)
                format!("{}_{:x}.{}", content_type, delivery_id.as_u128(), extension)
            },
            NamingStrategy::Custom(template) => {
                template.replace("{type}", content_type)
                       .replace("{id}", &delivery_id.to_string())
                       .replace("{timestamp}", &Utc::now().format("%Y%m%d_%H%M%S").to_string())
                       + "." + extension
            },
        }
    }

    fn get_content_directory(&self, payload: &ContentPayload) -> PathBuf {
        let subdir = match payload {
            ContentPayload::SingleItem(_) => "content_items",
            ContentPayload::ContentList(_) => "content_lists",
            ContentPayload::Notification(_) => "notifications",
            ContentPayload::Alert(_) => "alerts",
            ContentPayload::Custom(_) => "custom",
        };
        self.base_path.join(subdir)
    }

    fn serialize_content(&self, payload: &ContentPayload) -> Result<String, ContentDeliveryError> {
        match self.file_format {
            FileFormat::Json => {
                serde_json::to_string_pretty(payload)
                    .map_err(|e| ContentDeliveryError::SerializationError(e.to_string()))
            },
            FileFormat::Xml => {
                // For XML, we'll use a simple JSON-to-XML conversion
                let json_value = serde_json::to_value(payload)
                    .map_err(|e| ContentDeliveryError::SerializationError(e.to_string()))?;
                self.json_to_xml(&json_value)
            },
            FileFormat::PlainText => {
                self.payload_to_text(payload)
            },
            FileFormat::Csv => {
                self.payload_to_csv(payload)
            },
            FileFormat::Markdown => {
                self.payload_to_markdown(payload)
            },
        }
    }

    fn json_to_xml(&self, value: &serde_json::Value) -> Result<String, ContentDeliveryError> {
        // Simplified XML conversion - in production, use a proper XML library
        match value {
            serde_json::Value::Object(map) => {
                let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<root>\n");
                for (key, val) in map {
                    xml.push_str(&format!("  <{}>{}</{}>\n", key, self.json_value_to_string(val), key));
                }
                xml.push_str("</root>");
                Ok(xml)
            },
            _ => Ok(format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<root>{}</root>", self.json_value_to_string(value))),
        }
    }

    fn json_value_to_string(&self, value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Null => "null".to_string(),
            _ => value.to_string(),
        }
    }

    fn payload_to_text(&self, payload: &ContentPayload) -> Result<String, ContentDeliveryError> {
        match payload {
            ContentPayload::SingleItem(item) => {
                let mut text = format!("Content Item: {}\n", item.uuid);
                text.push_str(&format!("Created: {}\n", item.created));
                if let Some(ref title) = item.title {
                    text.push_str(&format!("Title: {}\n", title));
                }
                if let Some(ref description) = item.description {
                    text.push_str(&format!("Description: {}\n", description));
                }
                
                match &item.content {
                    ContentType::Text(text_content) => {
                        if let Some(ref content) = text_content.content {
                            text.push_str(&format!("Content: {}\n", content));
                        }
                    },
                    _ => {
                        text.push_str("Content: [Binary content not displayed in text format]\n");
                    }
                }
                
                Ok(text)
            },
            ContentPayload::Notification(notif) => {
                Ok(format!("Notification: {}\n{}\nCategory: {}\n", 
                          notif.title, notif.body, notif.category))
            },
            ContentPayload::Alert(alert) => {
                Ok(format!("Alert [{:?}]: {}\nSource: {}\nTimestamp: {}\n", 
                          alert.level, alert.message, alert.source, alert.timestamp))
            },
            _ => {
                serde_json::to_string_pretty(payload)
                    .map_err(|e| ContentDeliveryError::SerializationError(e.to_string()))
            }
        }
    }

    fn payload_to_csv(&self, payload: &ContentPayload) -> Result<String, ContentDeliveryError> {
        match payload {
            ContentPayload::ContentList(list) => {
                let mut csv = String::from("UUID,Title,Description,Created,Modified,Source\n");
                for item in &list.list_items {
                    match item {
                        crate::core::platform::container::content_list::ContentListItem::Item(content_item) => {
                            csv.push_str(&format!("\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"\n",
                                content_item.uuid,
                                content_item.title.as_deref().unwrap_or(""),
                                content_item.description.as_deref().unwrap_or(""),
                                content_item.created,
                                content_item.modified,
                                content_item.source.as_deref().unwrap_or("")
                            ));
                        },
                        _ => {} // Skip non-item entries for CSV
                    }
                }
                Ok(csv)
            },
            _ => {
                Err(ContentDeliveryError::SerializationError(
                    "CSV format only supported for ContentList".to_string()
                ))
            }
        }
    }

    fn payload_to_markdown(&self, payload: &ContentPayload) -> Result<String, ContentDeliveryError> {
        match payload {
            ContentPayload::SingleItem(item) => {
                let mut md = format!("# {}\n\n", item.title.as_deref().unwrap_or("Content Item"));
                md.push_str(&format!("**UUID:** {}\n", item.uuid));
                md.push_str(&format!("**Created:** {}\n", item.created));
                md.push_str(&format!("**Modified:** {}\n\n", item.modified));
                
                if let Some(ref description) = item.description {
                    md.push_str(&format!("## Description\n\n{}\n\n", description));
                }
                
                if let ContentType::Text(text_content) = &item.content {
                    if let Some(ref content) = text_content.content {
                        md.push_str(&format!("## Content\n\n{}\n", content));
                    }
                }
                
                Ok(md)
            },
            ContentPayload::Notification(notif) => {
                Ok(format!("# {}\n\n{}\n\n**Category:** {}\n", 
                          notif.title, notif.body, notif.category))
            },
            _ => {
                serde_json::to_string_pretty(payload)
                    .map_err(|e| ContentDeliveryError::SerializationError(e.to_string()))
            }
        }
    }

    fn write_content_to_file(&self, file_path: &Path, content: &str) -> Result<(), ContentDeliveryError> {
        // Check file size limit
        if content.len() as u64 > self.max_file_size_bytes {
            return Err(ContentDeliveryError::DeliveryFailed(
                format!("Content size {} exceeds maximum file size limit {}", 
                       content.len(), self.max_file_size_bytes)
            ));
        }

        // Create directory if it doesn't exist
        if let Some(parent) = file_path.parent() {
            if self.create_directories && !parent.exists() {
                fs::create_dir_all(parent)
                    .map_err(|e| ContentDeliveryError::DeliveryFailed(
                        format!("Failed to create directory: {}", e)
                    ))?;
            }
        }

        // Write content to file
        let mut file = File::create(file_path)
            .map_err(|e| ContentDeliveryError::DeliveryFailed(
                format!("Failed to create file: {}", e)
            ))?;

        file.write_all(content.as_bytes())
            .map_err(|e| ContentDeliveryError::DeliveryFailed(
                format!("Failed to write content: {}", e)
            ))?;

        file.sync_all()
            .map_err(|e| ContentDeliveryError::DeliveryFailed(
                format!("Failed to sync file: {}", e)
            ))?;

        Ok(())
    }

    fn create_delivery_response(&self, delivery_id: Uuid, status: DeliveryStatus, file_path: Option<&Path>, error: Option<&ContentDeliveryError>) -> DeliveryResponse {
        let mut metadata = HashMap::new();
        if let Some(path) = file_path {
            metadata.insert("file_path".to_string(), serde_json::Value::String(path.to_string_lossy().to_string()));
            metadata.insert("file_format".to_string(), serde_json::Value::String(format!("{:?}", self.file_format)));
        }

        DeliveryResponse {
            delivery_id,
            status,
            delivered_at: if matches!(status, DeliveryStatus::Delivered) { Some(Utc::now()) } else { None },
            attempt_count: 1,
            error_message: error.map(|e| e.to_string()),
            metadata: if metadata.is_empty() { None } else { Some(metadata) },
        }
    }

    fn store_delivery_history(&self, response: DeliveryResponse) {
        if let Ok(mut history) = self.delivery_history.lock() {
            history.insert(response.delivery_id, response);
        }
    }
}

impl ContentDeliveryService for FileStorageClient {
    fn deliver_content(&self, request: DeliveryRequest) -> Result<DeliveryResponse, ContentDeliveryError> {
        let delivery_id = Uuid::new_v4();
        
        // Determine content type for file naming
        let content_type = match &request.content_payload {
            ContentPayload::SingleItem(_) => "content_item",
            ContentPayload::ContentList(_) => "content_list",
            ContentPayload::Notification(_) => "notification",
            ContentPayload::Alert(_) => "alert",
            ContentPayload::Custom(_) => "custom",
        };

        // Generate filename and full path
        let filename = self.generate_filename(content_type, &delivery_id);
        let content_dir = self.get_content_directory(&request.content_payload);
        let file_path = content_dir.join(&filename);

        // Serialize content
        let serialized_content = self.serialize_content(&request.content_payload)?;

        // Write to file
        let result = self.write_content_to_file(&file_path, &serialized_content);

        let (status, error) = match result {
            Ok(()) => (DeliveryStatus::Delivered, None),
            Err(ref e) => (DeliveryStatus::Failed, Some(e)),
        };

        let response = self.create_delivery_response(
            delivery_id, 
            status, 
            if result.is_ok() { Some(&file_path) } else { None }, 
            error
        );
        
        self.store_delivery_history(response.clone());

        match result {
            Ok(()) => Ok(response),
            Err(e) => Err(e),
        }
    }

    fn schedule_delivery(&self, request: DeliveryRequest) -> Result<DeliveryResponse, ContentDeliveryError> {
        // For file storage, we'll just mark as scheduled and store the request
        let delivery_id = Uuid::new_v4();
        let response = self.create_delivery_response(delivery_id, DeliveryStatus::Scheduled, None, None);
        self.store_delivery_history(response.clone());
        
        // TODO: Integrate with actual scheduler
        Ok(response)
    }

    fn cancel_delivery(&self, delivery_id: Uuid) -> Result<(), ContentDeliveryError> {
        if let Ok(mut history) = self.delivery_history.lock() {
            if let Some(mut delivery) = history.get(&delivery_id).cloned() {
                delivery.status = DeliveryStatus::Cancelled;
                history.insert(delivery_id, delivery);
                Ok(())
            } else {
                Err(ContentDeliveryError::RecipientNotFound(delivery_id.to_string()))
            }
        } else {
            Err(ContentDeliveryError::ServiceUnavailable)
        }
    }

    fn get_delivery_status(&self, delivery_id: Uuid) -> Result<DeliveryResponse, ContentDeliveryError> {
        if let Ok(history) = self.delivery_history.lock() {
            history.get(&delivery_id)
                .cloned()
                .ok_or_else(|| ContentDeliveryError::RecipientNotFound(delivery_id.to_string()))
        } else {
            Err(ContentDeliveryError::ServiceUnavailable)
        }
    }

    fn list_deliveries(&self, _recipient_id: &str, limit: Option<u32>) -> Result<Vec<DeliveryResponse>, ContentDeliveryError> {
        if let Ok(history) = self.delivery_history.lock() {
            let mut deliveries: Vec<DeliveryResponse> = history.values().cloned().collect();
            
            // Sort by most recent first
            deliveries.sort_by(|a, b| {
                b.delivered_at.unwrap_or(Utc::now()).cmp(&a.delivered_at.unwrap_or(Utc::now()))
            });

            if let Some(limit) = limit {
                deliveries.truncate(limit as usize);
            }

            Ok(deliveries)
        } else {
            Err(ContentDeliveryError::ServiceUnavailable)
        }
    }

    fn get_delivery_stats(&self, _recipient_id: Option<&str>) -> Result<DeliveryStats, ContentDeliveryError> {
        if let Ok(history) = self.delivery_history.lock() {
            let total = history.len() as u64;
            let successful = history.values()
                .filter(|d| matches!(d.status, DeliveryStatus::Delivered))
                .count() as u64;
            let failed = history.values()
                .filter(|d| matches!(d.status, DeliveryStatus::Failed))
                .count() as u64;
            let pending = history.values()
                .filter(|d| matches!(d.status, DeliveryStatus::Pending | DeliveryStatus::InProgress | DeliveryStatus::Scheduled))
                .count() as u64;

            Ok(DeliveryStats {
                total_deliveries: total,
                successful_deliveries: successful,
                failed_deliveries: failed,
                pending_deliveries: pending,
                average_delivery_time_ms: None, // File operations are typically fast
            })
        } else {
            Err(ContentDeliveryError::ServiceUnavailable)
        }
    }

    fn validate_delivery_method(&self, _method: &DeliveryMethod) -> Result<(), ContentDeliveryError> {
        // File storage doesn't use delivery methods in the traditional sense
        Ok(())
    }
}

impl BatchContentDeliveryService for FileStorageClient {
    fn batch_deliver(&self, requests: Vec<DeliveryRequest>) -> Result<Vec<DeliveryResponse>, ContentDeliveryError> {
        let mut responses = Vec::new();
        
        for request in requests {
            let response = self.deliver_content(request)?;
            responses.push(response);
        }
        
        Ok(responses)
    }

    fn get_batch_status(&self, _batch_id: Uuid) -> Result<Vec<DeliveryResponse>, ContentDeliveryError> {
        // In a real implementation, you'd track batch IDs
        Err(ContentDeliveryError::DeliveryFailed("Batch status tracking not implemented".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::platform::container::content::{TextContent, ContentType};
    use tempfile::tempdir;

    fn create_test_config() -> FileStorageConfig {
        let temp_dir = tempdir().unwrap();
        FileStorageConfig {
            base_path: temp_dir.path().to_path_buf(),
            file_format: FileFormat::Json,
            naming_strategy: NamingStrategy::Uuid,
            max_file_size_bytes: Some(1024 * 1024), // 1MB
            create_directories: true,
        }
    }

    #[test]
    fn test_file_storage_client_creation() {
        let config = create_test_config();
        let client = FileStorageClient::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_deliver_single_content_item() {
        let config = create_test_config();
        let client = FileStorageClient::new(config).unwrap();
        
        let text_content = TextContent::new(None, Some("Test content".to_string())).unwrap();
        let content_item = ContentItem::new(ContentType::Text(text_content)).unwrap();

        let request = DeliveryRequest {
            recipient_id: "test_user".to_string(),
            delivery_method: DeliveryMethod::Http {
                endpoint: "file://storage".to_string(),
                headers: None,
            },
            content_payload: ContentPayload::SingleItem(content_item),
            priority: DeliveryPriority::Normal,
            scheduled_time: None,
            metadata: None,
        };

        let result = client.deliver_content(request);
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert_eq!(response.status, DeliveryStatus::Delivered);
        assert!(response.delivered_at.is_some());
        assert!(response.metadata.is_some());
    }

    #[test]
    fn test_different_file_formats() {
        let temp_dir = tempdir().unwrap();
        
        let formats = vec![
            FileFormat::Json,
            FileFormat::PlainText,
            FileFormat::Markdown,
        ];

        for format in formats {
            let config = FileStorageConfig {
                base_path: temp_dir.path().to_path_buf(),
                file_format: format,
                naming_strategy: NamingStrategy::Uuid,
                max_file_size_bytes: Some(1024 * 1024),
                create_directories: true,
            };

            let client = FileStorageClient::new(config).unwrap();
            let text_content = TextContent::new(None, Some("Test content".to_string())).unwrap();
            let content_item = ContentItem::new(ContentType::Text(text_content)).unwrap();

            let request = DeliveryRequest {
                recipient_id: "test_user".to_string(),
                delivery_method: DeliveryMethod::Http {
                    endpoint: "file://storage".to_string(),
                    headers: None,
                },
                content_payload: ContentPayload::SingleItem(content_item),
                priority: DeliveryPriority::Normal,
                scheduled_time: None,
                metadata: None,
            };

            let result = client.deliver_content(request);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_delivery_stats() {
        let config = create_test_config();
        let client = FileStorageClient::new(config).unwrap();
        
        let stats = client.get_delivery_stats(None).unwrap();
        assert_eq!(stats.total_deliveries, 0);
        assert_eq!(stats.successful_deliveries, 0);
        assert_eq!(stats.failed_deliveries, 0);
        assert_eq!(stats.pending_deliveries, 0);
    }

    #[test]
    fn test_filename_generation() {
        let config = create_test_config();
        let client = FileStorageClient::new(config).unwrap();
        
        let delivery_id = Uuid::new_v4();
        let filename = client.generate_filename("test", &delivery_id);
        
        assert!(filename.contains("test"));
        assert!(filename.ends_with(".json"));
    }
}