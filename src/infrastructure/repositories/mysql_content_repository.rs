use crate::application::storage::sql_store::{
    ContentRepository, ContentListRepository, 
    TransactionManager, MigrationManager, SqlStore, RepositoryError, RepositoryStats
};
use crate::core::platform::container::content::{ContentItem, ContentType, TextContent, VideoContent, AudioContent, ImageContent, ContentData};
use crate::core::platform::container::content_list::ContentList;
use crate::core::base::entity::node::Node;
use async_trait::async_trait;

use sqlx::{MySqlPool, Row, mysql::MySqlPoolOptions};
use serde_json;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use url::Url;

#[derive(Debug, Clone)]
pub struct MySqlContentRepository {
    pool: MySqlPool,
}

#[derive(Debug, Clone)]
pub struct MySqlConfig {
    pub database_url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout_seconds: u64,
    pub idle_timeout_seconds: u64,
}

impl Default for MySqlConfig {
    fn default() -> Self {
        Self {
            database_url: "mysql://user:password@localhost:3306/paladin".to_string(),
            max_connections: 10,
            min_connections: 1,
            connect_timeout_seconds: 30,
            idle_timeout_seconds: 600,
        }
    }
}

impl MySqlContentRepository {
    pub async fn new(config: MySqlConfig) -> Result<Self, RepositoryError> {
        let pool = MySqlPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .connect(&config.database_url)
            .await
            .map_err(|e| RepositoryError::ConnectionError(e.to_string()))?;

        let repository = Self { pool };
        
        // Run migrations on startup
        repository.migrate().await?;
        
        Ok(repository)
    }

    pub async fn from_pool(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub async fn with_config(config: MySqlConfig) -> Result<Self, RepositoryError> {
        let pool = MySqlPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .connect(&config.database_url)
            .await
            .map_err(|e| RepositoryError::ConnectionError(e.to_string()))?;

        let repository = Self { pool };
        
        // Run migrations on startup
        repository.migrate().await?;
        
        Ok(repository)
    }

    async fn serialize_content_type(&self, content: &ContentType) -> Result<(String, serde_json::Value), RepositoryError> {
        let (content_type, content_data) = match content {
            ContentType::Text(text) => {
                let data = serde_json::json!({
                    "path": text.path,
                    "content": text.content,
                    "filesize": text.filesize
                });
                ("text", data)
            },
            ContentType::Video(video) => {
                let data = serde_json::json!({
                    "path": video.path,
                    "duration": video.duration,
                    "filesize": video.filesize
                });
                ("video", data)
            },
            ContentType::Audio(audio) => {
                let data = serde_json::json!({
                    "path": audio.path,
                    "duration": audio.duration,
                    "filesize": audio.filesize
                });
                ("audio", data)
            },
            ContentType::Image(image) => {
                let data = serde_json::json!({
                    "path": image.path,
                    "resolution": [image.resolution.0, image.resolution.1],
                    "filesize": image.filesize
                });
                ("image", data)
            },
        };

        Ok((content_type.to_string(), content_data))
    }

    async fn deserialize_content_type(&self, content_type: &str, content_data: &str) -> Result<ContentType, RepositoryError> {
        let data: serde_json::Value = serde_json::from_str(content_data)
            .map_err(|e| RepositoryError::SerializationError(e.to_string()))?;

        match content_type {
            "text" => {
                let path = data["path"].as_str().map(|s| s.to_string());
                let content = data["content"].as_str().map(|s| s.to_string());
                let filesize = data["filesize"].as_u64().unwrap_or(0);
                
                let text_content = TextContent {
                    path,
                    content,
                    filesize,
                };
                
                Ok(ContentType::Text(text_content))
            },
            "video" => {
                let path = data["path"].as_str().map(|s| s.to_string());
                let duration = data["duration"].as_u64().unwrap_or(0);
                let filesize = data["filesize"].as_u64().unwrap_or(0);
                
                let video_content = VideoContent {
                    path,
                    duration,
                    filesize,
                };
                
                Ok(ContentType::Video(video_content))
            },
            "audio" => {
                let path = data["path"].as_str().map(|s| s.to_string());
                let duration = data["duration"].as_u64().unwrap_or(0);
                let filesize = data["filesize"].as_u64().unwrap_or(0);
                
                let audio_content = AudioContent {
                    path,
                    duration,
                    filesize,
                };
                
                Ok(ContentType::Audio(audio_content))
            },
            "image" => {
                let path = data["path"].as_str().map(|s| s.to_string());
                let resolution = if let Some(res_array) = data["resolution"].as_array() {
                    let width = res_array.get(0).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    let height = res_array.get(1).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    (width, height)
                } else {
                    (0, 0)
                };
                let filesize = data["filesize"].as_u64().unwrap_or(0);
                
                let image_content = ImageContent {
                    path,
                    resolution,
                    filesize,
                };
                
                Ok(ContentType::Image(image_content))
            },
            _ => Err(RepositoryError::SerializationError(format!("Unknown content type: {}", content_type))),
        }
    }

    async fn row_to_content_item(&self, row: &sqlx::mysql::MySqlRow) -> Result<ContentItem, RepositoryError> {
        // Extract database values
        let uuid_str: String = row.try_get("uuid")
            .map_err(|e| RepositoryError::QueryError(e.to_string()))?;
        let uuid = Uuid::parse_str(&uuid_str)
            .map_err(|e| RepositoryError::SerializationError(e.to_string()))?;

        let created: DateTime<Utc> = row.try_get("created")
            .map_err(|e| RepositoryError::QueryError(e.to_string()))?;
        let modified: DateTime<Utc> = row.try_get("modified")
            .map_err(|e| RepositoryError::QueryError(e.to_string()))?;

        let content_type_str: String = row.try_get("content_type")
            .map_err(|e| RepositoryError::QueryError(e.to_string()))?;
        let content_data_str: String = row.try_get("content_data")
            .map_err(|e| RepositoryError::QueryError(e.to_string()))?;
        
        let content = self.deserialize_content_type(&content_type_str, &content_data_str).await?;

        // Extract optional fields
        let title: Option<String> = row.try_get("title")
            .map_err(|e| RepositoryError::QueryError(e.to_string()))?;

        let url = row.try_get::<Option<String>, _>("url")
            .map_err(|e| RepositoryError::QueryError(e.to_string()))?
            .and_then(|url_str| Url::parse(&url_str).ok());

        let hash: Option<String> = row.try_get("hash")
            .map_err(|e| RepositoryError::QueryError(e.to_string()))?;

        let source_url = row.try_get::<Option<String>, _>("source_url")
            .map_err(|e| RepositoryError::QueryError(e.to_string()))?
            .and_then(|url_str| Url::parse(&url_str).ok());

        let description: Option<String> = row.try_get("description")
            .map_err(|e| RepositoryError::QueryError(e.to_string()))?;

        let tags_str: Option<String> = row.try_get("tags")
            .map_err(|e| RepositoryError::QueryError(e.to_string()))?;
        let tags = tags_str.and_then(|s| serde_json::from_str(&s).ok());

        let source: Option<String> = row.try_get("source")
            .map_err(|e| RepositoryError::QueryError(e.to_string()))?;

        let author: Option<String> = row.try_get("author")
            .map_err(|e| RepositoryError::QueryError(e.to_string()))?;

        let source_id: Option<String> = row.try_get("source_id")
            .map_err(|e| RepositoryError::QueryError(e.to_string()))?;

        let pub_date: Option<DateTime<Utc>> = row.try_get("pub_date")
            .map_err(|e| RepositoryError::QueryError(e.to_string()))?;

        let mod_date: Option<DateTime<Utc>> = row.try_get("mod_date")
            .map_err(|e| RepositoryError::QueryError(e.to_string()))?;

        let version: bool = row.try_get("version")
            .map_err(|e| RepositoryError::QueryError(e.to_string()))?;

        // Reconstruct ContentData
        let content_data = ContentData {
            content,
            url,
            hash,
            source_id,
            source_url,
            description,
            tags,
            source,
            author,
            pub_date,
            mod_date,
        };

        // Reconstruct Node with database values
        let node = Node {
            uuid,
            created,
            modified,
            node: content_data,
            name: title,
            version,
        };

        // Create ContentItem with reconstructed node
        Ok(ContentItem { node })
    }
}

#[async_trait]
impl ContentRepository for MySqlContentRepository {
    async fn get_by_hash(&self, hash: &str) -> Result<Option<ContentItem>, RepositoryError> {
        let row = sqlx::query("SELECT * FROM content_items WHERE hash = ?")
            .bind(hash)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| RepositoryError::QueryError(e.to_string()))?;

        match row {
            Some(row) => Ok(Some(self.row_to_content_item(&row).await?)),
            None => Ok(None),
        }
    }

    async fn get_by_id(&self, id: Uuid) -> Result<Option<ContentItem>, RepositoryError> {
        let row = sqlx::query("SELECT * FROM content_items WHERE uuid = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| RepositoryError::QueryError(e.to_string()))?;

        match row {
            Some(row) => Ok(Some(self.row_to_content_item(&row).await?)),
            None => Ok(None),
        }
    }

    async fn create(&self, content: ContentItem) -> Result<Uuid, RepositoryError> {
        let id = content.uuid();
        let (content_type, content_data_json) = self.serialize_content_type(&content.node.node.content).await?;
        let content_data_str = serde_json::to_string(&content_data_json)
            .map_err(|e| RepositoryError::SerializationError(e.to_string()))?;
        let tags_str = content.node.node.tags.as_ref()
            .map(|tags| serde_json::to_string(tags))
            .transpose()
            .map_err(|e| RepositoryError::SerializationError(e.to_string()))?;

        sqlx::query(r#"
            INSERT INTO content_items (
                uuid, created, modified, content_type, content_data, url, hash,
                source_url, title, description, tags, source, author, source_id,
                pub_date, mod_date, version
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(content.node.uuid.to_string())
        .bind(content.node.created)
        .bind(content.node.modified)
        .bind(&content_type)
        .bind(&content_data_str)
        .bind(content.node.node.url.as_ref().map(|u| u.as_str()))
        .bind(&content.node.node.hash)
        .bind(content.node.node.source_url.as_ref().map(|u| u.as_str()))
        .bind(&content.node.name)
        .bind(&content.node.node.description)
        .bind(tags_str.as_deref())
        .bind(&content.node.node.source)
        .bind(&content.node.node.author)
        .bind(&content.node.node.source_id)
        .bind(content.node.node.pub_date)
        .bind(content.node.node.mod_date)
        .bind(content.node.version)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::QueryError(e.to_string()))?;

        Ok(id)
    }

    async fn update(&self, content: &ContentItem) -> Result<(), RepositoryError> {
        let (content_type, content_data_json) = self.serialize_content_type(&content.node.node.content).await?;
        let content_data_str = serde_json::to_string(&content_data_json)
            .map_err(|e| RepositoryError::SerializationError(e.to_string()))?;
        let tags_str = content.node.node.tags.as_ref()
            .map(|tags| serde_json::to_string(tags))
            .transpose()
            .map_err(|e| RepositoryError::SerializationError(e.to_string()))?;

        let result = sqlx::query(r#"
            UPDATE content_items SET 
                modified = ?, content_type = ?, content_data = ?, url = ?, hash = ?,
                source_url = ?, title = ?, description = ?, tags = ?, 
                source = ?, author = ?, source_id = ?, pub_date = ?, mod_date = ?, version = ?
            WHERE uuid = ?
        "#)
        .bind(content.node.modified)
        .bind(&content_type)
        .bind(&content_data_str)
        .bind(content.node.node.url.as_ref().map(|u| u.as_str()))
        .bind(&content.node.node.hash)
        .bind(content.node.node.source_url.as_ref().map(|u| u.as_str()))
        .bind(&content.node.name)
        .bind(&content.node.node.description)
        .bind(tags_str.as_deref())
        .bind(&content.node.node.source)
        .bind(&content.node.node.author)
        .bind(&content.node.node.source_id)
        .bind(content.node.node.pub_date)
        .bind(content.node.node.mod_date)
        .bind(content.node.version)
        .bind(content.node.uuid.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::QueryError(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound(content.node.uuid.to_string()));
        }

        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), RepositoryError> {
        let result = sqlx::query("DELETE FROM content_items WHERE uuid = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| RepositoryError::QueryError(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound(id.to_string()));
        }

        Ok(())
    }

    async fn list(&self) -> Result<Vec<ContentItem>, RepositoryError> {
        let rows = sqlx::query("SELECT * FROM content_items ORDER BY created DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| RepositoryError::QueryError(e.to_string()))?;

        let mut items = Vec::new();
        for row in rows {
            items.push(self.row_to_content_item(&row).await?);
        }

        Ok(items)
    }

    async fn find_by_tags(&self, tags: &[String]) -> Result<Vec<ContentItem>, RepositoryError> {
        let mut items = Vec::new();
        for tag in tags {
            let rows = sqlx::query("SELECT * FROM content_items WHERE JSON_CONTAINS(tags, ?)")
                .bind(format!("\"{}\"", tag))
                .fetch_all(&self.pool)
                .await
                .map_err(|e| RepositoryError::QueryError(e.to_string()))?;

            for row in rows {
                items.push(self.row_to_content_item(&row).await?);
            }
        }
        Ok(items)
    }

    async fn find_by_source(&self, source: &str) -> Result<Vec<ContentItem>, RepositoryError> {
        let rows = sqlx::query("SELECT * FROM content_items WHERE source = ?")
            .bind(source)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| RepositoryError::QueryError(e.to_string()))?;

        let mut items = Vec::new();
        for row in rows {
            items.push(self.row_to_content_item(&row).await?);
        }
        Ok(items)
    }

    async fn count(&self) -> Result<u64, RepositoryError> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM content_items")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| RepositoryError::QueryError(e.to_string()))?;

        let count: i64 = row.try_get("count")
            .map_err(|e| RepositoryError::QueryError(e.to_string()))?;

        Ok(count as u64)
    }

    async fn exists_by_hash(&self, hash: &str) -> Result<bool, RepositoryError> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM content_items WHERE hash = ?")
            .bind(hash)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| RepositoryError::QueryError(e.to_string()))?;

        let count: i64 = row.try_get("count")
            .map_err(|e| RepositoryError::QueryError(e.to_string()))?;

        Ok(count > 0)
    }
}

#[async_trait]
impl MigrationManager for MySqlContentRepository {
    async fn migrate(&self) -> Result<(), RepositoryError> {
        // Create content_items table
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS content_items (
                uuid VARCHAR(36) PRIMARY KEY,
                created TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                modified TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
                content_type VARCHAR(50) NOT NULL,
                content_data JSON NOT NULL,
                url TEXT,
                hash VARCHAR(255),
                source_url TEXT,
                title TEXT,
                description TEXT,
                tags JSON,
                source VARCHAR(255),
                author VARCHAR(255),
                source_id VARCHAR(255),
                pub_date TIMESTAMP NULL,
                mod_date TIMESTAMP NULL,
                version BOOLEAN DEFAULT TRUE,
                INDEX idx_hash (hash),
                INDEX idx_source (source),
                INDEX idx_created (created),
                INDEX idx_content_type (content_type)
            ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
        "#)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::MigrationError(e.to_string()))?;

        // Create content_lists table
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS content_lists (
                uuid VARCHAR(36) PRIMARY KEY,
                name VARCHAR(255) NOT NULL,
                url TEXT NOT NULL,
                created TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                modified TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
                version BOOLEAN DEFAULT TRUE,
                INDEX idx_name (name),
                INDEX idx_created (created)
            ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
        "#)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::MigrationError(e.to_string()))?;

        // Create content_list_items junction table
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS content_list_items (
                list_uuid VARCHAR(36),
                item_uuid VARCHAR(36),
                created TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (list_uuid, item_uuid),
                FOREIGN KEY (list_uuid) REFERENCES content_lists(uuid) ON DELETE CASCADE,
                INDEX idx_list_uuid (list_uuid),
                INDEX idx_item_uuid (item_uuid)
            ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
        "#)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::MigrationError(e.to_string()))?;

        Ok(())
    }

    async fn is_up_to_date(&self) -> Result<bool, RepositoryError> {
        let tables = vec!["content_items", "content_lists", "content_list_items"];
        
        for table in tables {
            let result = sqlx::query("SHOW TABLES LIKE ?")
                .bind(table)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| RepositoryError::QueryError(e.to_string()))?;
            
            if result.is_none() {
                return Ok(false);
            }
        }
        
        Ok(true)
    }

    async fn current_version(&self) -> Result<Option<String>, RepositoryError> {
        Ok(Some("1.0.0".to_string()))
    }
}

impl ContentListRepository for MySqlContentRepository {
    fn get_by_id(&self, _id: Uuid) -> Result<Option<ContentList>, RepositoryError> {
        todo!("Implement content list retrieval")
    }

    fn get_by_name(&self, _name: &str) -> Result<Option<ContentList>, RepositoryError> {
        todo!("Implement content list retrieval by name")
    }

    fn save(&self, _content_list: &ContentList) -> Result<(), RepositoryError> {
        todo!("Implement content list saving")
    }

    fn update(&self, _content_list: &ContentList) -> Result<(), RepositoryError> {
        todo!("Implement content list updating")
    }

    fn delete(&self, _id: Uuid) -> Result<(), RepositoryError> {
        todo!("Implement content list deletion")
    }

    fn find_all(&self, _limit: Option<u32>, _offset: Option<u32>) -> Result<Vec<ContentList>, RepositoryError> {
        todo!("Implement content list finding")
    }

    fn find_by_date_range(&self, _start: DateTime<Utc>, _end: DateTime<Utc>) -> Result<Vec<ContentList>, RepositoryError> {
        todo!("Implement content list date range finding")
    }

    fn count(&self) -> Result<u64, RepositoryError> {
        todo!("Implement content list counting")
    }

    fn add_item_to_list(&self, _list_id: Uuid, _item: &ContentItem) -> Result<(), RepositoryError> {
        todo!("Implement adding item to list")
    }

    fn remove_item_from_list(&self, _list_id: Uuid, _item_id: Uuid) -> Result<(), RepositoryError> {
        todo!("Implement removing item from list")
    }
}

impl TransactionManager for MySqlContentRepository {
    fn with_transaction<F, R>(&self, operation: F) -> Result<R, RepositoryError>
    where
        F: FnOnce() -> Result<R, RepositoryError>,
    {
        operation()
    }
}

#[async_trait]
impl SqlStore for MySqlContentRepository {
    async fn get_stats(&self) -> Result<RepositoryStats, RepositoryError> {
        let content_items_count = ContentRepository::count(self).await?;
        
        Ok(RepositoryStats {
            total_content_items: content_items_count,
            total_content_lists: 0,
            total_items_to_fetch: 0,
            database_size_bytes: None,
            last_updated: Utc::now(),
        })
    }

    async fn health_check(&self) -> Result<bool, RepositoryError> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| RepositoryError::QueryError(e.to_string()))?;
        
        Ok(true)
    }

    async fn cleanup(&self, older_than: DateTime<Utc>) -> Result<u64, RepositoryError> {
        let result = sqlx::query("DELETE FROM content_items WHERE created < ?")
            .bind(older_than)
            .execute(&self.pool)
            .await
            .map_err(|e| RepositoryError::QueryError(e.to_string()))?;
        
        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::platform::container::content::TextContent;

    #[tokio::test]
    async fn test_mysql_repository_creation() {
        println!("MySQL repository tests require database setup");
    }

    #[test]
    fn test_content_type_serialization() {
        let text_content = TextContent {
            path: Some("test.txt".to_string()),
            content: Some("Test content".to_string()),
            filesize: 12,
        };
        
        let content_type = ContentType::Text(text_content);
        println!("Content type created: {:?}", content_type);
    }

    #[test]
    fn test_content_item_reconstruction() {
        let text_content = TextContent {
            path: None,
            content: Some("Test content".to_string()),
            filesize: 12,
        };
        
        let content_data = ContentData {
            content: ContentType::Text(text_content),
            url: None,
            hash: None,
            source_id: None,
            source_url: None,
            description: None,
            tags: None,
            source: None,
            author: None,
            pub_date: None,
            mod_date: None,
        };

        let uuid = Uuid::new_v4();
        let now = Utc::now();
        
        let node = Node {
            uuid,
            created: now,
            modified: now,
            node: content_data,
            name: Some("Test Item".to_string()),
            version: true,
        };

        let content_item = ContentItem { node };
        
        assert_eq!(content_item.uuid(), uuid);
        assert_eq!(content_item.created(), now);
        assert_eq!(content_item.title(), Some(&"Test Item".to_string()));
    }
}