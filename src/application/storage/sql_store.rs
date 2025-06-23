/*
SQL Database Application Repository

An application repository is a port that defines the operations that the application layer can perform on the data layer (infrastructure repositories) and the interface requirements that the data layers adapter must implement. The application repository is implemented by the data layer, which contains the actual implementation of the operations defined in the application repository. The application repository acts as a bridge between the application layer and the data layer, allowing the application layer to interact with the data layer without having to know the details of the data layer implementation.

TODO Should this be renamed as the ContentSqlRepository in the content_sql_store.rs?
*/
use crate::core::platform::container::content::{ContentItem, ContentItemError};
use crate::core::platform::container::content_list::{ContentList, ContentListError};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use thiserror::Error;
use async_trait::async_trait;

#[derive(Debug, Clone, Error)]
pub enum RepositoryError {
    #[error("Database connection error: {0}")]
    ConnectionError(String),
    #[error("Query execution error: {0}")]
    QueryError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Entity not found: {0}")]
    NotFound(String),
    #[error("Duplicate entry: {0}")]
    DuplicateEntry(String),
    #[error("Transaction error: {0}")]
    TransactionError(String),
    #[error("Migration error: {0}")]
    MigrationError(String),
    #[error("Content item error: {0}")]
    ContentItemError(#[from] ContentItemError),
    #[error("Content list error: {0}")]
    ContentListError(#[from] ContentListError),
}

#[async_trait]
pub trait ContentRepository: Send + Sync {
    /// Get content item by hash
    async fn get_by_hash(&self, hash: &str) -> Result<Option<ContentItem>, RepositoryError>;
    
    /// Get content item by UUID  
    async fn get_by_id(&self, id: Uuid) -> Result<Option<ContentItem>, RepositoryError>;
    
    /// Create/save a content item
    async fn create(&self, content: ContentItem) -> Result<Uuid, RepositoryError>;
    
    /// Update an existing content item
    async fn update(&self, content: &ContentItem) -> Result<(), RepositoryError>;
    
    /// Delete a content item by UUID
    async fn delete(&self, id: Uuid) -> Result<(), RepositoryError>;
    
    /// Get all content items with pagination
    async fn list(&self) -> Result<Vec<ContentItem>, RepositoryError>;
    
    /// Find content items by tags
    async fn find_by_tags(&self, tags: &[String]) -> Result<Vec<ContentItem>, RepositoryError>;
    
    /// Find content items by source
    async fn find_by_source(&self, source: &str) -> Result<Vec<ContentItem>, RepositoryError>;
    
    /// Count total content items
    async fn count(&self) -> Result<u64, RepositoryError>;
    
    /// Check if content with hash exists
    async fn exists_by_hash(&self, hash: &str) -> Result<bool, RepositoryError>;
}

// Todo make async
/// Content List Repository  
/// Defines operations for managing ContentList entities
pub trait ContentListRepository {
    /// Get content list by UUID
    fn get_by_id(&self, id: Uuid) -> Result<Option<ContentList>, RepositoryError>;
    
    /// Get content list by name
    fn get_by_name(&self, name: &str) -> Result<Option<ContentList>, RepositoryError>;
    
    /// Save a content list
    fn save(&self, content_list: &ContentList) -> Result<(), RepositoryError>;
    
    /// Update an existing content list
    fn update(&self, content_list: &ContentList) -> Result<(), RepositoryError>;
    
    /// Delete a content list by UUID
    fn delete(&self, id: Uuid) -> Result<(), RepositoryError>;
    
    /// Get all content lists with pagination
    fn find_all(&self, limit: Option<u32>, offset: Option<u32>) -> Result<Vec<ContentList>, RepositoryError>;
    
    /// Find content lists created within a date range
    fn find_by_date_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<ContentList>, RepositoryError>;
    
    /// Count total content lists
    fn count(&self) -> Result<u64, RepositoryError>;
    
    /// Add item to content list
    fn add_item_to_list(&self, list_id: Uuid, item: &ContentItem) -> Result<(), RepositoryError>;
    
    /// Remove item from content list
    fn remove_item_from_list(&self, list_id: Uuid, item_id: Uuid) -> Result<(), RepositoryError>;
}

/// Database Transaction Manager
/// Handles database transactions across multiple operations
pub trait TransactionManager {
    /// Execute operations within a transaction
    fn with_transaction<F, R>(&self, operation: F) -> Result<R, RepositoryError>
    where
        F: FnOnce() -> Result<R, RepositoryError>;
}

/// Database Migration Manager
/// Handles database schema migrations
#[async_trait]
pub trait MigrationManager: Send + Sync {
    /// Run database migrations
    async fn migrate(&self) -> Result<(), RepositoryError>;
    
    /// Check if database is up to date
    async fn is_up_to_date(&self) -> Result<bool, RepositoryError>;
    
    /// Get current migration version
    async fn current_version(&self) -> Result<Option<String>, RepositoryError>;
}

#[async_trait]
pub trait SqlStore: Send + Sync {
    /// Get repository statistics
    async fn get_stats(&self) -> Result<RepositoryStats, RepositoryError>;
    
    /// Perform health check
    async fn health_check(&self) -> Result<bool, RepositoryError>;
    
    /// Clean up old content
    async fn cleanup(&self, older_than: DateTime<Utc>) -> Result<u64, RepositoryError>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepositoryStats {
    pub total_content_items: u64,
    pub total_content_lists: u64,
    pub total_items_to_fetch: u64,
    pub database_size_bytes: Option<u64>,
    pub last_updated: DateTime<Utc>,
}